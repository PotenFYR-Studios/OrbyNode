//! Remote-node pairing, heartbeat, and aggregation routes (Milestone 13).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use orbynode_auth::{Perm, User};
use orbynode_nodes::NodeStatus;

use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/nodes", get(list_nodes))
        .route("/nodes", post(register_node))
        .route("/nodes/{id}/pairing", post(create_pairing_authorized))
        .route("/nodes/{id}/revoke", post(revoke_node))
        .route("/nodes/{id}/agents", get(node_agents))
}

#[derive(serde::Deserialize)]
struct RegisterNodeBody {
    name: String,
    fingerprint: String,
    public_key: String,
}

async fn list_nodes(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<Json<Vec<orbynode_nodes::Node>>, ApiError> {
    require_node_view(&user)?;
    Ok(Json(state.nodes.nodes().await.map_err(node_error)?))
}

async fn register_node(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(body): Json<RegisterNodeBody>,
) -> Result<(StatusCode, Json<orbynode_nodes::Node>), ApiError> {
    require_node_manage(&user)?;
    let node = state
        .nodes
        .register(&body.name, &body.fingerprint, &body.public_key)
        .await
        .map_err(node_error)?;
    state
        .record(
            Some(&user),
            "machine.pairing",
            &format!("node:{}", node.id),
            "registered",
        )
        .await;
    Ok((StatusCode::CREATED, Json(node)))
}

async fn create_pairing_authorized(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<orbynode_nodes::Pairing>, ApiError> {
    require_node_manage(&user)?;
    Ok(Json(
        state.nodes.create_pairing(&id).await.map_err(node_error)?,
    ))
}

async fn revoke_node(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    require_node_manage(&user)?;
    state.nodes.revoke(&id).await.map_err(node_error)?;
    state
        .record(Some(&user), "node.revoke", &format!("node:{id}"), "")
        .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn node_agents(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    require_node_view(&user)?;
    let node = state
        .nodes
        .node(&id)
        .await
        .map_err(node_error)?
        .ok_or(ApiError(StatusCode::NOT_FOUND))?;
    if node.status != NodeStatus::Online {
        return Ok(Json(Vec::new()));
    }
    Ok(Json(Vec::new()))
}

fn require_node_view(user: &User) -> Result<(), ApiError> {
    if orbynode_auth::role_has(user.role, Perm::AgentView) {
        Ok(())
    } else {
        Err(ApiError(StatusCode::FORBIDDEN))
    }
}

fn require_node_manage(user: &User) -> Result<(), ApiError> {
    if orbynode_auth::role_has(user.role, Perm::SettingsManage) {
        Ok(())
    } else {
        Err(ApiError(StatusCode::FORBIDDEN))
    }
}

fn node_error(error: anyhow::Error) -> ApiError {
    tracing::error!(error = %error, "node registry error");
    ApiError(StatusCode::BAD_REQUEST)
}
