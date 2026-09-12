//! Project/session REST routes (Milestone 3, ADR 004).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use orbynode_database::{Project, Session};

use crate::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/projects", get(list_projects).post(create_project))
        .route("/projects/{id}", get(get_project).delete(delete_project))
        .route(
            "/projects/{id}/sessions",
            get(list_sessions).post(create_session),
        )
}

#[derive(serde::Deserialize)]
struct CreateProjectBody {
    name: String,
    path: String,
}

async fn list_projects(State(state): State<AppState>) -> Result<Json<Vec<Project>>, crate::ApiError> {
    Ok(Json(state.db.list_projects().await.map_err(crate::ApiError::from)?))
}

async fn create_project(
    State(state): State<AppState>,
    Json(body): Json<CreateProjectBody>,
) -> Result<(StatusCode, Json<Project>), crate::ApiError> {
    let p = state
        .db
        .create_project(&body.name, &body.path)
        .await
        .map_err(crate::ApiError::from)?;
    Ok((StatusCode::CREATED, Json(p)))
}

async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Project>, crate::ApiError> {
    state
        .db
        .get_project(id)
        .await
        .map_err(crate::ApiError::from)?
        .map(Json)
        .ok_or(crate::ApiError(StatusCode::NOT_FOUND))
}

async fn delete_project(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, crate::ApiError> {
    state.db.delete_project(id).await.map_err(crate::ApiError::from)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
struct CreateSessionBody {
    name: String,
}

async fn list_sessions(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<Session>>, crate::ApiError> {
    Ok(Json(state.db.list_sessions(id).await.map_err(crate::ApiError::from)?))
}

async fn create_session(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<CreateSessionBody>,
) -> Result<(StatusCode, Json<Session>), crate::ApiError> {
    let s = state
        .db
        .create_session(id, &body.name)
        .await
        .map_err(crate::ApiError::from)?;
    Ok((StatusCode::CREATED, Json(s)))
}
