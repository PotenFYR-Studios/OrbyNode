//! Terminal REST + WebSocket endpoints (Milestone 1, ADR 002).
//!
//! M1 transport: REST for create/list/resize/terminate, one WebSocket per
//! terminal for the stream (binary out, text JSON in). M2 replaces this with
//! the multiplexed gateway (ADR 009). Auth arrives with M4 — localhost-only
//! binding is the containment until then (ADR 001).

use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use orbynode_terminal::{TerminalConfig, TerminalError, TerminalId, TerminalInfo};

use crate::AppState;
use futures_util::{SinkExt, StreamExt};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/terminals", post(create_terminal).get(list_terminals))
        .route(
            "/terminals/{id}",
            get(get_terminal).delete(terminate_terminal),
        )
        .route("/terminals/{id}/input", post(write_input))
        .route("/terminals/{id}/resize", post(resize_terminal))
        .route("/terminals/{id}/ws", get(ws_upgrade))
}

/// Small error wrapper so handler `Result`s don't trip clippy's
/// `result_large_err` (a full `Response` is >128 bytes on the stack).
#[derive(Debug)]
struct ApiError(StatusCode);

impl From<TerminalError> for ApiError {
    fn from(e: TerminalError) -> Self {
        match e {
            TerminalError::NotFound => ApiError(StatusCode::NOT_FOUND),
            TerminalError::Io(_) => ApiError(StatusCode::CONFLICT),
        }
    }
}

impl From<ApiError> for Response {
    fn from(e: ApiError) -> Self {
        e.0.into_response()
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> Response {
        self.0.into_response()
    }
}

async fn create_terminal(
    State(state): State<AppState>,
    body: String,
) -> Result<Json<TerminalInfo>, ApiError> {
    // Body is optional; an empty body means default config.
    let body: CreateTerminalBody = if body.trim().is_empty() {
        CreateTerminalBody::default()
    } else {
        serde_json::from_str(&body).map_err(|_| ApiError(StatusCode::BAD_REQUEST))?
    };
    let cfg = TerminalConfig {
        cols: body.cols.unwrap_or(80),
        rows: body.rows.unwrap_or(24),
        scrollback_bytes: body.scrollback_bytes.unwrap_or(1024 * 1024),
        cwd: body.cwd.map(std::path::PathBuf::from),
    };
    let term = state.terminals.create(cfg).map_err(ApiError::from)?;
    Ok(Json(term.info()))
}

#[derive(serde::Deserialize, Default)]
struct CreateTerminalBody {
    cols: Option<u16>,
    rows: Option<u16>,
    scrollback_bytes: Option<usize>,
    cwd: Option<String>,
}

async fn list_terminals(State(state): State<AppState>) -> Json<Vec<TerminalInfo>> {
    Json(state.terminals.list())
}

async fn get_terminal(
    State(state): State<AppState>,
    Path(id): Path<TerminalId>,
) -> Result<Json<TerminalInfo>, ApiError> {
    state
        .terminals
        .get(id)
        .map(|t| Json(t.info()))
        .ok_or(ApiError(StatusCode::NOT_FOUND))
}

async fn terminate_terminal(
    State(state): State<AppState>,
    Path(id): Path<TerminalId>,
) -> Result<StatusCode, ApiError> {
    state
        .terminals
        .terminate(id)
        .await
        .map_err(ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
struct InputBody {
    data: String,
}

async fn write_input(
    State(state): State<AppState>,
    Path(id): Path<TerminalId>,
    Json(body): Json<InputBody>,
) -> Result<StatusCode, ApiError> {
    let term = state
        .terminals
        .get(id)
        .ok_or(ApiError(StatusCode::NOT_FOUND))?;
    term.write(body.data.as_bytes()).map_err(ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
struct ResizeBody {
    cols: u16,
    rows: u16,
}

async fn resize_terminal(
    State(state): State<AppState>,
    Path(id): Path<TerminalId>,
    Json(body): Json<ResizeBody>,
) -> Result<StatusCode, ApiError> {
    let term = state
        .terminals
        .get(id)
        .ok_or(ApiError(StatusCode::NOT_FOUND))?;
    term.resize(body.cols, body.rows).map_err(ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Terminal WebSocket: server→client raw PTY bytes as binary frames,
/// client→server JSON control messages (`{"input": "..."}`, `{"resize":
/// {"cols": n, "rows": n}}`). Scrollback replays first, then live output.
async fn ws_upgrade(
    State(state): State<AppState>,
    Path(id): Path<TerminalId>,
    ws: WebSocketUpgrade,
) -> Response {
    let Some(term) = state.terminals.get(id) else {
        return ApiError(StatusCode::NOT_FOUND).into_response();
    };
    ws.on_upgrade(move |socket| run_terminal_socket(term, socket))
}

async fn run_terminal_socket(term: Arc<orbynode_terminal::Terminal>, mut socket: WebSocket) {
    // 1. Replay scrollback, then attach live.
    let (live_tx, live_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(1024);
    let mut sub = term.subscribe();

    // The gap between fetching scrollback and the first recv() of `sub` is
    // bridged by a small staging buffer: anything received during replay is
    // flushed before live forwarding starts.
    let mut staged: Vec<Vec<u8>> = Vec::new();
    loop {
        match sub.try_recv() {
            Ok(chunk) => staged.push(chunk),
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => break,
            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => continue,
            Err(tokio::sync::broadcast::error::TryRecvError::Closed) => return,
        }
    }

    if socket
        .send(Message::Binary(term.scrollback().into()))
        .await
        .is_err()
    {
        return;
    }
    for chunk in staged {
        if socket.send(Message::Binary(chunk.into())).await.is_err() {
            return;
        }
    }

    // 2. Reader task: broadcast → channel → writer half.
    let (mut sender, mut receiver) = socket.split();
    let outbound = tokio::spawn(async move {
        let mut rx = live_rx;
        while let Some(chunk) = rx.recv().await {
            if sender.send(Message::Binary(chunk.into())).await.is_err() {
                break;
            }
            // Drain any immediately-available chunks into one frame batch.
            while let Ok(more) = rx.try_recv() {
                if sender.send(Message::Binary(more.into())).await.is_err() {
                    return;
                }
            }
        }
    });

    // 3. Forward PTY output into the channel; drive inbound control messages.
    loop {
        tokio::select! {
            chunk = sub.recv() => {
                match chunk {
                    Ok(bytes) => {
                        if live_tx.send(bytes).await.is_err() {
                            break; // outbound task died
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // Slow client missed chunks (Plan §66): request resync
                        // by sending the whole scrollback again.
                        if live_tx.send(term.scrollback()).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_control(&term, text.as_bytes());
                    }
                    Some(Ok(Message::Binary(bytes))) => {
                        handle_control(&term, &bytes);
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }
    drop(live_tx); // ends the channel: outbound task drains and finishes
    let _ = outbound.await;
}

fn handle_control(term: &Arc<orbynode_terminal::Terminal>, bytes: &[u8]) {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return;
    };
    if let Some(input) = value.get("input").and_then(|v| v.as_str()) {
        let _ = term.write(input.as_bytes());
    }
    if let Some(resize) = value.get("resize")
        && let (Some(cols), Some(rows)) = (
            resize.get("cols").and_then(|v| v.as_u64()),
            resize.get("rows").and_then(|v| v.as_u64()),
        )
    {
        let _ = term.resize(
            cols.min(u64::from(u16::MAX)) as u16,
            rows.min(u64::from(u16::MAX)) as u16,
        );
    }
}
