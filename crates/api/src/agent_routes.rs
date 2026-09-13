//! Detected-agent REST routes (Milestone 6, Plan §129).

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use orbynode_agents::DetectedAgent;

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/agents", get(list_agents))
}

async fn list_agents(State(state): State<AppState>) -> Json<Vec<DetectedAgent>> {
    Json(state.detector.list().await)
}
