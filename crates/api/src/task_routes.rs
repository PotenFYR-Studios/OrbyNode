//! Task board + worktree REST (Milestone 9, Plan §25/§26).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use orbynode_database::{Conflict, Task, TaskState};
use orbynode_files::git::GitRepo;
use orbynode_files::worktree::{WorktreeManager, default_base};
use orbynode_realtime::{Event, Stream};

use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/projects/{id}/tasks", get(list_tasks).post(create_task))
        .route("/tasks/{id}", get(get_task).delete(delete_task))
        .route("/tasks/{id}/move", post(move_task))
        .route("/tasks/{id}/worktree", post(ensure_worktree))
        .route(
            "/tasks/{id}/worktree",
            axum::routing::delete(remove_worktree),
        )
}

async fn project_path(state: &AppState, project_id: i64) -> Result<String, ApiError> {
    let p = state
        .db
        .get_project(project_id)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError(StatusCode::NOT_FOUND))?;
    Ok(p.path)
}

async fn project_name(state: &AppState, project_id: i64) -> Result<String, ApiError> {
    let p = state
        .db
        .get_project(project_id)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError(StatusCode::NOT_FOUND))?;
    Ok(p.name)
}

async fn list_tasks(
    State(state): State<AppState>,
    Path(project_id): Path<i64>,
) -> Result<Json<Vec<Task>>, ApiError> {
    Ok(Json(
        state
            .db
            .list_tasks(project_id)
            .await
            .map_err(ApiError::from)?,
    ))
}

#[derive(serde::Deserialize)]
struct CreateTaskBody {
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default = "default_priority")]
    priority: i32,
}

fn default_priority() -> i32 {
    3
}

async fn create_task(
    State(state): State<AppState>,
    Path(project_id): Path<i64>,
    Json(body): Json<CreateTaskBody>,
) -> Result<(StatusCode, Json<Task>), ApiError> {
    let t = state
        .db
        .create_task(project_id, &body.title, &body.description, body.priority)
        .await
        .map_err(ApiError::from)?;
    Ok((StatusCode::CREATED, Json(t)))
}

async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Task>, ApiError> {
    state
        .db
        .get_task(id)
        .await
        .map_err(ApiError::from)?
        .map(Json)
        .ok_or(ApiError(StatusCode::NOT_FOUND))
}

async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    state.db.delete_task(id).await.map_err(ApiError::from)?;
    state
        .attention
        .observe(&Event {
            stream: Stream::new("tasks"),
            etype: "task.deleted".into(),
            data: serde_json::json!({ "id": id }),
            priority: orbynode_realtime::Priority::Critical,
            bytes: Vec::new(),
        })
        .await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
struct MoveBody {
    state: TaskState,
    /// Optimistic concurrency (§68): the version the client last saw.
    version: i64,
}

async fn move_task(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<MoveBody>,
) -> Result<Json<Task>, ApiError> {
    match state.db.move_task(id, body.state, body.version).await {
        Ok(Ok(task)) => {
            state
                .attention
                .observe(&Event {
                    stream: Stream::new(format!("tasks:project{}", task.project_id)),
                    etype: "task.updated".into(),
                    data: serde_json::to_value(&task).unwrap_or_default(),
                    priority: orbynode_realtime::Priority::Critical,
                    bytes: Vec::new(),
                })
                .await;
            Ok(Json(task))
        }
        Ok(Err(Conflict)) => Err(ApiError(StatusCode::CONFLICT)),
        Err(e) => Err(ApiError::from(e)),
    }
}

async fn ensure_worktree(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Task>, ApiError> {
    let task = state
        .db
        .get_task(id)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError(StatusCode::NOT_FOUND))?;
    let path = project_path(&state, task.project_id).await?;
    let name = project_name(&state, task.project_id).await?;
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/"));
    let mgr = WorktreeManager::new(GitRepo::open(path), default_base(&home, &name));
    let wt = mgr.ensure(&task.title).map_err(|e| {
        tracing::error!(error = %e, "worktree ensure failed");
        ApiError(StatusCode::INTERNAL_SERVER_ERROR)
    })?;
    state
        .db
        .set_task_worktree(id, &wt.branch, &wt.path)
        .await
        .map_err(ApiError::from)?;
    Ok(Json(
        state
            .db
            .get_task(id)
            .await
            .map_err(ApiError::from)?
            .expect("exists"),
    ))
}

async fn remove_worktree(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let task = state
        .db
        .get_task(id)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError(StatusCode::NOT_FOUND))?;
    let path = project_path(&state, task.project_id).await?;
    let name = project_name(&state, task.project_id).await?;
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/"));
    let mgr = WorktreeManager::new(GitRepo::open(path), default_base(&home, &name));
    mgr.remove(&task.title).map_err(|e| {
        tracing::error!(error = %e, "worktree remove failed");
        ApiError(StatusCode::INTERNAL_SERVER_ERROR)
    })?;
    Ok(StatusCode::NO_CONTENT)
}
