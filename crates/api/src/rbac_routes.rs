//! RBAC REST (Milestone 11): user management, project membership, audit tail.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Extension, Json, Router};
use orbynode_auth::{Perm, Role, User};

use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users).post(create_user))
        .route("/projects/{id}/members", get(list_members).post(set_member))
        .route(
            "/projects/{id}/members/{user_id}",
            axum::routing::delete(remove_member),
        )
        .route("/audit", get(audit_tail))
}

fn require(user: &User, perm: Perm) -> Result<(), ApiError> {
    if orbynode_auth::role_has(user.role, perm) {
        Ok(())
    } else {
        Err(ApiError(StatusCode::FORBIDDEN))
    }
}

async fn list_users(State(state): State<AppState>, Extension(user): Extension<User>) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    require(&user, Perm::UsersManage)?;
    Ok(Json(state.db.list_users().await.map_err(ApiError::from)?))
}

#[derive(serde::Deserialize)]
struct CreateUserBody {
    username: String,
    display_name: String,
    password: String,
    role: Role,
}

async fn create_user(
    State(state): State<AppState>,
    Extension(actor): Extension<User>,
    Json(body): Json<CreateUserBody>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    require(&actor, Perm::UsersManage)?;
    let u = state
        .auth
        .create_user(&body.username, &body.display_name, &body.password, body.role)
        .await
        .map_err(ApiError::from)?;
    state
        .record(Some(&actor), "user.create", &format!("user:{}", u.username), body.role.as_str())
        .await;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": u.id,
            "username": u.username,
            "role": u.role.as_str(),
        })),
    ))
}

async fn list_members(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Path(project_id): Path<i64>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    require(&user, Perm::ProjectView)?;
    Ok(Json(state.db.list_members(project_id).await.map_err(ApiError::from)?))
}

#[derive(serde::Deserialize)]
struct SetMemberBody {
    user_id: i64,
    role: String,
}

async fn set_member(
    State(state): State<AppState>,
    Extension(actor): Extension<User>,
    Path(project_id): Path<i64>,
    Json(body): Json<SetMemberBody>,
) -> Result<StatusCode, ApiError> {
    require(&actor, Perm::ProjectManage)?;
    state
        .db
        .set_member(project_id, body.user_id, &body.role)
        .await
        .map_err(ApiError::from)?;
    state
        .record(Some(&actor), "member.set", &format!("project:{project_id}/user:{}", body.user_id), &body.role)
        .await;
    // Live revocation (Plan §105): permission change republishes the ACL so
    // the gateway drops now-forbidden subscriptions on next check.
    state.realtime.publish(orbynode_realtime::Event {
        stream: orbynode_realtime::Stream::new(format!("project:{project_id}")),
        etype: "acl.changed".into(),
        data: serde_json::json!({"user_id": body.user_id, "role": body.role}),
        priority: orbynode_realtime::Priority::Critical,
        bytes: Vec::new(),
    });
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_member(
    State(state): State<AppState>,
    Extension(actor): Extension<User>,
    Path((project_id, member_user_id)): Path<(i64, i64)>,
) -> Result<StatusCode, ApiError> {
    require(&actor, Perm::ProjectManage)?;
    state
        .db
        .remove_member(project_id, member_user_id)
        .await
        .map_err(ApiError::from)?;
    state
        .record(Some(&actor), "member.remove", &format!("project:{project_id}/user:{member_user_id}"), "")
        .await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
struct AuditQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    100
}

async fn audit_tail(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<orbynode_database::AuditEntry>>, ApiError> {
    require(&user, Perm::AuditView)?;
    Ok(Json(state.db.audit_tail(q.limit).await.map_err(ApiError::from)?))
}
