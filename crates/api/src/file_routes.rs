//! Files + Git REST (Milestone 8, Plan §23/§24). All routes are auth-gated;
//! file paths are root-enforced by orbynode-files (§23 security).

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use orbynode_files::git::GitRepo;
use orbynode_files::{FilesError, Workspace};

use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/projects/{id}/files",
            get(list_files).post(write_file).delete(delete_file),
        )
        .route("/projects/{id}/files/read", get(read_file))
        .route("/projects/{id}/git/status", get(git_status))
        .route("/projects/{id}/git/diff", get(git_diff))
        .route("/projects/{id}/git/log", get(git_log))
        .route("/projects/{id}/git/branches", get(git_branches))
        .route("/projects/{id}/git/stage", post(git_stage))
        .route("/projects/{id}/git/commit", post(git_commit))
        .route("/projects/{id}/git/branch", post(git_branch))
        .route("/projects/{id}/git/switch", post(git_switch))
}

async fn workspace_of(state: &AppState, project_id: i64) -> Result<Workspace, ApiError> {
    let project = state
        .db
        .get_project(project_id)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError(StatusCode::NOT_FOUND))?;
    Ok(Workspace::new(project.path))
}

async fn git_of(state: &AppState, project_id: i64) -> Result<GitRepo, ApiError> {
    let project = state
        .db
        .get_project(project_id)
        .await
        .map_err(ApiError::from)?
        .ok_or(ApiError(StatusCode::NOT_FOUND))?;
    Ok(GitRepo::open(project.path))
}

fn files_err(e: FilesError) -> ApiError {
    match e {
        FilesError::OutsideRoot => ApiError(StatusCode::FORBIDDEN),
        FilesError::NotFound => ApiError(StatusCode::NOT_FOUND),
        FilesError::Io(_) => ApiError(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn git_err(e: orbynode_files::git::GitError) -> ApiError {
    match e {
        orbynode_files::git::GitError::NotARepository => ApiError(StatusCode::CONFLICT),
        orbynode_files::git::GitError::Git(_) | orbynode_files::git::GitError::Io(_) => {
            tracing::error!(error = %e, "git error");
            ApiError(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(serde::Deserialize)]
struct ListQuery {
    #[serde(default)]
    path: String,
}

async fn list_files(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<orbynode_files::FileEntry>>, ApiError> {
    let ws = workspace_of(&state, id).await?;
    let path = if q.path.is_empty() { "." } else { &q.path };
    Ok(Json(ws.list(path).map_err(files_err)?))
}

#[derive(serde::Deserialize)]
struct ReadQuery {
    path: String,
}

async fn read_file(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(q): Query<ReadQuery>,
) -> Result<String, ApiError> {
    let ws = workspace_of(&state, id).await?;
    let bytes = ws.read(&q.path).map_err(files_err)?;
    String::from_utf8(bytes).map_err(|_| ApiError(StatusCode::UNSUPPORTED_MEDIA_TYPE))
}

#[derive(serde::Deserialize)]
struct WriteBody {
    path: String,
    contents: String,
}

async fn write_file(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<WriteBody>,
) -> Result<StatusCode, ApiError> {
    let ws = workspace_of(&state, id).await?;
    ws.write(&body.path, &body.contents).map_err(files_err)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
struct DeleteQuery {
    path: String,
}

async fn delete_file(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(q): Query<DeleteQuery>,
) -> Result<StatusCode, ApiError> {
    let ws = workspace_of(&state, id).await?;
    ws.delete(&q.path).map_err(files_err)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn git_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<orbynode_files::git::GitStatus>, ApiError> {
    let repo = git_of(&state, id).await?;
    Ok(Json(repo.status().map_err(git_err)?))
}

async fn git_diff(State(state): State<AppState>, Path(id): Path<i64>) -> Result<String, ApiError> {
    let repo = git_of(&state, id).await?;
    repo.diff().map_err(git_err)
}

async fn git_log(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<String>>, ApiError> {
    let repo = git_of(&state, id).await?;
    Ok(Json(repo.log(50).map_err(git_err)?))
}

async fn git_branches(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<String>>, ApiError> {
    let repo = git_of(&state, id).await?;
    Ok(Json(repo.branches().map_err(git_err)?))
}

#[derive(serde::Deserialize)]
struct StageBody {
    paths: Vec<String>,
}

async fn git_stage(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<StageBody>,
) -> Result<StatusCode, ApiError> {
    let repo = git_of(&state, id).await?;
    let refs: Vec<&str> = body.paths.iter().map(String::as_str).collect();
    repo.stage(&refs).map_err(git_err)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
struct CommitBody {
    message: String,
}

async fn git_commit(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<CommitBody>,
) -> Result<StatusCode, ApiError> {
    let repo = git_of(&state, id).await?;
    repo.commit(&body.message).map_err(git_err)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
struct BranchBody {
    name: String,
}

async fn git_branch(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<BranchBody>,
) -> Result<StatusCode, ApiError> {
    let repo = git_of(&state, id).await?;
    repo.create_branch(&body.name).map_err(git_err)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn git_switch(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<BranchBody>,
) -> Result<StatusCode, ApiError> {
    let repo = git_of(&state, id).await?;
    repo.switch(&body.name).map_err(git_err)?;
    Ok(StatusCode::NO_CONTENT)
}
