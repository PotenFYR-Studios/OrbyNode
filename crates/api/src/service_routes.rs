//! Services REST + preview proxy (Milestone 10, Plan §32/§33/§34/§35).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use orbynode_services::{
    HostSnapshot, ListeningPort, ServiceRegistry, ServiceSpec, TerminalSnapshot, discover_ports,
    host_snapshot, terminal_snapshots,
};

use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/services/ports", get(ports))
        .route("/services/host", get(host))
        .route("/observability/host", get(host))
        .route("/observability/sessions", get(sessions))
        .route("/services", get(list_services).post(register_service))
        .route("/services/{name}", axum::routing::delete(remove_service))
        .route("/preview/{port}/{*path}", get(preview))
        .route("/preview/{port}", get(preview_root))
}

async fn ports(State(state): State<AppState>) -> Result<Json<Vec<ListeningPort>>, ApiError> {
    let found = discover_ports().await.map_err(|e| {
        tracing::error!(error = %e, "port discovery failed");
        ApiError(StatusCode::INTERNAL_SERVER_ERROR)
    })?;
    // Fan out through the bus so dashboards update without polling (§59).
    state.realtime.publish(orbynode_realtime::Event {
        stream: orbynode_realtime::Stream::new("host"),
        etype: "services.ports".into(),
        data: serde_json::to_value(&found).unwrap_or_default(),
        priority: orbynode_realtime::Priority::Droppable,
        bytes: Vec::new(),
    });
    Ok(Json(found))
}

async fn host(State(state): State<AppState>) -> Result<Json<HostSnapshot>, ApiError> {
    let snap = host_snapshot().await;
    state.realtime.publish(orbynode_realtime::Event {
        stream: orbynode_realtime::Stream::new("host"),
        etype: "host.metrics".into(),
        data: serde_json::to_value(&snap).unwrap_or_default(),
        priority: orbynode_realtime::Priority::Droppable,
        bytes: Vec::new(),
    });
    Ok(Json(snap))
}

async fn sessions(State(state): State<AppState>) -> Json<Vec<TerminalSnapshot>> {
    let mut ids: Vec<u64> = state.terminals.list().iter().map(|t| t.id).collect();
    ids.sort_unstable();
    let snapshots = terminal_snapshots(&ids).await;
    state.realtime.publish(orbynode_realtime::Event {
        stream: orbynode_realtime::Stream::new("host"),
        etype: "sessions.metrics".into(),
        data: serde_json::to_value(&snapshots).unwrap_or_default(),
        priority: orbynode_realtime::Priority::Droppable,
        bytes: Vec::new(),
    });
    Json(snapshots)
}

async fn list_services(State(state): State<AppState>) -> Json<Vec<ServiceSpec>> {
    Json(state.service_registry.list())
}

#[derive(serde::Deserialize)]
struct RegisterBody {
    name: String,
    project: String,
    command: String,
    port: Option<u16>,
}

async fn register_service(
    State(state): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> StatusCode {
    state.service_registry.upsert(ServiceSpec {
        name: body.name,
        project: body.project,
        command: body.command,
        port: body.port,
    });
    StatusCode::NO_CONTENT
}

async fn remove_service(State(state): State<AppState>, Path(name): Path<String>) -> StatusCode {
    state.service_registry.remove(&name);
    StatusCode::NO_CONTENT
}

/// Authenticated local preview (Plan §33): proxies /preview/{port}/... to
/// 127.0.0.1:{port}. Never exposes arbitrary hosts; loopback only.
async fn preview(
    State(_state): State<AppState>,
    Path((port, path)): Path<(u16, String)>,
) -> Response {
    proxy_to(port, &path).await
}

async fn preview_root(State(_state): State<AppState>, Path(port): Path<u16>) -> Response {
    proxy_to(port, "").await
}

async fn proxy_to(port: u16, path: &str) -> Response {
    let url = format!("http://127.0.0.1:{port}/{path}");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| {
            tracing::error!(error = %e, "preview client build failed");
            ApiError(StatusCode::INTERNAL_SERVER_ERROR)
        });
    let client = match client {
        Ok(c) => c,
        Err(e) => return e.into_response(),
    };
    match client.get(&url).send().await {
        Ok(resp) => {
            let status = axum::http::StatusCode::from_u16(resp.status().as_u16())
                .unwrap_or(StatusCode::BAD_GATEWAY);
            let body = resp.bytes().await.unwrap_or_default();
            (status, body).into_response()
        }
        Err(e) => {
            tracing::info!(url, error = %e, "preview upstream unreachable");
            ApiError(StatusCode::BAD_GATEWAY).into_response()
        }
    }
}

pub type SharedRegistry = Arc<ServiceRegistry>;
