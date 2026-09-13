use crate::{ApiError, AppState};
use axum::extract::State;
use axum::{Json, Router};
use orbynode_agents::attention::AttentionItem;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/attention", axum::routing::get(list_attention))
        .route(
            "/attention/{id}/resolve",
            axum::routing::post(resolve_attention),
        )
}

async fn list_attention(State(state): State<AppState>) -> Json<Vec<AttentionItem>> {
    Json(state.attention.list())
}

async fn resolve_attention(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<u64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !state.attention.resolve(id) {
        return Err(ApiError(axum::http::StatusCode::NOT_FOUND));
    }
    Ok(Json(serde_json::json!({"resolved": true})))
}
