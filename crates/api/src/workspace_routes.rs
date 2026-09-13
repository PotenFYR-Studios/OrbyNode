//! Workspace / tab / pane REST (ADR 021). All routes auth-gated; pane input
//! and journal output are bounded and audited. The web GUI is the only pane
//! surface (no multiplexer client).

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Extension, Json, Router};
use orbynode_auth::Perm;
use orbynode_database::NewPane;

use crate::workspace_engine::WorkspaceEngine;
use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/workspaces", get(list).post(create_workspace))
        .route(
            "/workspaces/{id}",
            axum::routing::patch(rename_workspace).delete(delete_workspace),
        )
        .route("/workspaces/{id}/tabs", get(list_tabs).post(create_tab))
        .route(
            "/tabs/{id}",
            axum::routing::patch(rename_tab).delete(delete_tab),
        )
        .route("/tabs/{id}/panes", get(list_panes).post(create_pane))
        .route("/panes/{id}", delete(close_pane).patch(patch_pane))
        .route("/panes/{id}/input", post(pane_input))
        .route("/panes/{id}/output", get(pane_output))
}

fn require(user: &orbynode_auth::User, perm: Perm) -> Result<(), ApiError> {
    if orbynode_auth::role_has(user.role, perm) {
        Ok(())
    } else {
        Err(ApiError(StatusCode::FORBIDDEN))
    }
}

fn engine(state: &AppState) -> WorkspaceEngine {
    WorkspaceEngine::new(state.db.clone(), Default::default())
}

async fn list(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let workspaces = state.db.list_workspaces().await?;
    let mut out = Vec::new();
    for ws in workspaces {
        let tabs = state.db.list_tabs(ws.id).await?;
        let mut tab_list = Vec::new();
        for tab in tabs {
            let panes = state.db.list_panes(tab.id).await?;
            tab_list.push(serde_json::json!({ "tab": tab, "panes": panes }));
        }
        out.push(serde_json::json!({ "workspace": ws, "tabs": tab_list }));
    }
    Ok(Json(serde_json::json!({ "workspaces": out })))
}

#[derive(serde::Deserialize)]
struct NameBody {
    name: String,
}

async fn create_workspace(
    State(state): State<AppState>,
    Extension(user): Extension<orbynode_auth::User>,
    Json(body): Json<NameBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    require(&user, Perm::TerminalCreate)?;
    let ws = engine(&state)
        .create_workspace(&body.name)
        .await
        .map_err(ApiError::engine)?;
    state
        .record(
            Some(&user),
            "workspace.create",
            &format!("workspace:{}", ws.id),
            "",
        )
        .await;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(ws).unwrap_or_default()),
    ))
}

async fn rename_workspace(
    State(state): State<AppState>,
    Extension(user): Extension<orbynode_auth::User>,
    Path(id): Path<i64>,
    Json(body): Json<NameBody>,
) -> Result<StatusCode, ApiError> {
    require(&user, Perm::TerminalCreate)?;
    state.db.rename_workspace(id, &body.name).await?;
    engine(&state).snapshot().await.ok();
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_workspace(
    State(state): State<AppState>,
    Extension(user): Extension<orbynode_auth::User>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    require(&user, Perm::TerminalCreate)?;
    state.db.delete_workspace(id).await?;
    state
        .record(
            Some(&user),
            "workspace.delete",
            &format!("workspace:{id}"),
            "",
        )
        .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_tabs(
    State(state): State<AppState>,
    Extension(user): Extension<orbynode_auth::User>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require(&user, Perm::TerminalCreate)?;
    let tabs = state.db.list_tabs(id).await?;
    Ok(Json(serde_json::to_value(tabs).unwrap_or_default()))
}

async fn create_tab(
    State(state): State<AppState>,
    Extension(user): Extension<orbynode_auth::User>,
    Path(id): Path<i64>,
    Json(body): Json<NameBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    require(&user, Perm::TerminalCreate)?;
    let tab = engine(&state)
        .create_tab(id, &body.name)
        .await
        .map_err(ApiError::engine)?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(tab).unwrap_or_default()),
    ))
}

async fn rename_tab(
    State(state): State<AppState>,
    Extension(user): Extension<orbynode_auth::User>,
    Path(id): Path<i64>,
    Json(body): Json<NameBody>,
) -> Result<StatusCode, ApiError> {
    require(&user, Perm::TerminalCreate)?;
    state.db.rename_tab(id, &body.name).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_tab(
    State(state): State<AppState>,
    Extension(user): Extension<orbynode_auth::User>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    require(&user, Perm::TerminalCreate)?;
    state.db.delete_tab(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
struct CreatePaneBody {
    tab_id: i64,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    cwd: String,
    #[serde(default)]
    title: String,
}

async fn list_panes(
    State(state): State<AppState>,
    Extension(user): Extension<orbynode_auth::User>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require(&user, Perm::TerminalCreate)?;
    let panes = state.db.list_panes(id).await?;
    Ok(Json(serde_json::to_value(panes).unwrap_or_default()))
}

/// Creates the pane row, spawns a real PTY, bridges it to the realtime bus,
/// and links the row to the live terminal.
async fn create_pane(
    State(state): State<AppState>,
    Extension(user): Extension<orbynode_auth::User>,
    Path(_tab_path): Path<i64>,
    Json(body): Json<CreatePaneBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    require(&user, Perm::TerminalCreate)?;
    let kind = if body.kind.is_empty() {
        "shell".to_owned()
    } else {
        body.kind
    };
    let pane = engine(&state)
        .create_pane(&NewPane {
            tab_id: body.tab_id,
            kind: kind.clone(),
            cwd: body.cwd.clone(),
            title: if body.title.is_empty() {
                kind
            } else {
                body.title
            },
            ..NewPane::default()
        })
        .await
        .map_err(ApiError::engine)?;

    let cfg = orbynode_terminal::TerminalConfig {
        cwd: if body.cwd.is_empty() {
            None
        } else {
            Some(std::path::PathBuf::from(&body.cwd))
        },
        ..orbynode_terminal::TerminalConfig::default()
    };
    let term = state.terminals.create(cfg)?;
    state.bridge_terminal_to_bus(term.id());
    state
        .db
        .set_pane_terminal(pane.id, Some(term.id() as i64))
        .await?;

    // ADR 021 tier 2: journal this pane's output as a second fan-out
    // subscriber. Slow journals never block the PTY reader (bounded staging).
    let journal_db = state.db.clone();
    let pane_id = pane.id;
    let mut journal_rx = term.subscribe();
    tokio::spawn(async move {
        loop {
            match journal_rx.recv().await {
                Ok(bytes) => {
                    let eng = WorkspaceEngine::new(journal_db.clone(), Default::default());
                    eng.journal_output(pane_id, &bytes).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    });

    engine(&state).snapshot().await.ok();

    state
        .record(
            Some(&user),
            "pane.create",
            &format!("pane:{}", pane.id),
            &format!("terminal:{}", term.id()),
        )
        .await;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(pane).unwrap_or_default()),
    ))
}

async fn close_pane(
    State(state): State<AppState>,
    Extension(user): Extension<orbynode_auth::User>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    require(&user, Perm::TerminalCreate)?;
    if let Some(pane) = state.db.get_pane(id).await?
        && let Some(tid) = pane.terminal_id
    {
        let _ = state.terminals.terminate(tid as u64).await;
    }
    state.db.close_pane(id).await?;
    state
        .record(Some(&user), "pane.close", &format!("pane:{id}"), "")
        .await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
struct PatchPaneBody {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    split_dir: Option<String>,
    #[serde(default)]
    split_ratio: Option<f64>,
    #[serde(default)]
    position: Option<i64>,
}

async fn patch_pane(
    State(state): State<AppState>,
    Extension(user): Extension<orbynode_auth::User>,
    Path(id): Path<i64>,
    Json(body): Json<PatchPaneBody>,
) -> Result<StatusCode, ApiError> {
    require(&user, Perm::TerminalCreate)?;
    if let Some(title) = &body.title {
        state.db.rename_pane(id, title).await?;
    }
    if let (Some(dir), Some(pos)) = (&body.split_dir, body.position) {
        state
            .db
            .update_pane_layout(id, dir, body.split_ratio, pos)
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
struct InputBody {
    data: String,
}

/// Write to the pane's PTY. Also records agent session IDs when the input
/// matches a resume command (tier-3 bookkeeping).
async fn pane_input(
    State(state): State<AppState>,
    Extension(user): Extension<orbynode_auth::User>,
    Path(id): Path<i64>,
    Json(body): Json<InputBody>,
) -> Result<StatusCode, ApiError> {
    require(&user, Perm::TerminalWrite)?;
    let pane = state
        .db
        .get_pane(id)
        .await?
        .ok_or(ApiError(StatusCode::NOT_FOUND))?;
    let tid = pane.terminal_id.ok_or(ApiError(StatusCode::CONFLICT))?;
    let term = state
        .terminals
        .get(tid as u64)
        .ok_or(ApiError(StatusCode::NOT_FOUND))?;
    term.write(body.data.as_bytes())?;
    state
        .record(
            Some(&user),
            "pane.input",
            &format!("pane:{id}"),
            &format!("{} bytes", body.data.len()),
        )
        .await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
struct OutputQuery {
    #[serde(default)]
    since_seq: i64,
}

/// Bounded journal read (ADR 021 tier-2 replay).
async fn pane_output(
    State(state): State<AppState>,
    Extension(user): Extension<orbynode_auth::User>,
    Path(id): Path<i64>,
    Query(q): Query<OutputQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require(&user, Perm::TerminalCreate)?;
    state
        .db
        .get_pane(id)
        .await?
        .ok_or(ApiError(StatusCode::NOT_FOUND))?;
    let chunks = engine(&state)
        .journal_read(id, q.since_seq)
        .await
        .map_err(ApiError::engine)?;
    let items: Vec<serde_json::Value> = chunks
        .into_iter()
        .map(|(seq, chunk)| {
            serde_json::json!({
                "seq": seq,
                "text": String::from_utf8_lossy(&chunk),
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "chunks": items })))
}

impl ApiError {
    pub fn engine(e: String) -> Self {
        tracing::error!(error = %e, "workspace engine error");
        ApiError(StatusCode::INTERNAL_SERVER_ERROR)
    }
}
