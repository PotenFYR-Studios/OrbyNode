//! Integration Manager REST (Milestone 7, Plan §12).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use orbynode_integrations::{FsStore, IntegrationManager, IntegrationStatus, Target};

use crate::ApiError;
use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/integrations", get(list))
        .route("/integrations/{target}/install", post(install))
        .route("/integrations/{target}/uninstall", post(uninstall))
        .route("/integrations/{target}/rollback", post(rollback))
}

fn mgr(_state: &AppState) -> IntegrationManager<FsStore> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/"));
    IntegrationManager::new(Arc::new(FsStore::new(home)))
}

fn parse_target(t: &str) -> Result<Target, ApiError> {
    match t {
        "claude" => Ok(Target::Claude),
        "codex" => Ok(Target::Codex),
        "gemini" => Ok(Target::Gemini),
        "opencode" => Ok(Target::OpenCode),
        "hermes" => Ok(Target::Hermes),
        _ => Err(ApiError(StatusCode::NOT_FOUND)),
    }
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<IntegrationStatus>>, ApiError> {
    let m = mgr(&state);
    let mut out = Vec::new();
    for t in [
        Target::Claude,
        Target::Codex,
        Target::Gemini,
        Target::OpenCode,
        Target::Hermes,
    ] {
        out.push(m.status(t).map_err(integ_err)?);
    }
    Ok(Json(out))
}

fn integ_err(e: orbynode_integrations::IntegrationError) -> ApiError {
    tracing::error!(error = %e, "integration error");
    ApiError(StatusCode::INTERNAL_SERVER_ERROR)
}

async fn install(
    State(state): State<AppState>,
    Path(target): Path<String>,
) -> Result<Json<IntegrationStatus>, ApiError> {
    let t = parse_target(&target)?;
    let m = mgr(&state);
    m.install(t).map_err(integ_err)?;
    Ok(Json(m.status(t).map_err(integ_err)?))
}

async fn uninstall(
    State(state): State<AppState>,
    Path(target): Path<String>,
) -> Result<Json<IntegrationStatus>, ApiError> {
    let t = parse_target(&target)?;
    let m = mgr(&state);
    m.uninstall(t).map_err(integ_err)?;
    Ok(Json(m.status(t).map_err(integ_err)?))
}

async fn rollback(
    State(state): State<AppState>,
    Path(target): Path<String>,
) -> Result<Json<IntegrationStatus>, ApiError> {
    let t = parse_target(&target)?;
    let m = mgr(&state);
    m.rollback(t).map_err(integ_err)?;
    Ok(Json(m.status(t).map_err(integ_err)?))
}
