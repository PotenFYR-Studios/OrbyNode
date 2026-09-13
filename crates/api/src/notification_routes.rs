//! Notification rules and delivery status (Milestone 15, Plan §43).

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use orbynode_auth::{Perm, User};
use orbynode_notifications::NotificationRule;

use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/notifications/rules", get(list_rules).post(upsert_rule))
        .route("/notifications/test", post(send_test))
}

#[derive(Debug, Clone, serde::Deserialize)]
struct UpsertRuleBody {
    event: String,
    project_id: Option<i64>,
    channel: String,
    target: String,
    #[serde(default = "bool::default")]
    enabled: bool,
}

async fn list_rules(State(state): State<AppState>) -> Json<Vec<NotificationRule>> {
    Json(state.notifications.list().await)
}

async fn upsert_rule(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
    Json(body): Json<UpsertRuleBody>,
) -> Result<Json<NotificationRule>, ApiError> {
    require_manage(&user)?;
    let rule = NotificationRule {
        id: orbynode_notifications::new_id(),
        event: body.event,
        project_id: body.project_id,
        channel: body.channel,
        target: body.target,
        enabled: body.enabled,
    };
    state.notifications.upsert(rule.clone()).await;
    state
        .record(Some(&user), "notification.rule", &rule.id, &rule.event)
        .await;
    Ok(Json(rule))
}

async fn send_test(
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> Result<StatusCode, ApiError> {
    require_manage(&user)?;
    let notifications = state.notifications.clone();
    notifications
        .send(orbynode_agents::attention::AttentionItem {
            id: 0,
            priority: orbynode_agents::attention::AttentionPriority::P4Metrics,
            resource: "notification:test".into(),
            kind: "notification.test".into(),
            summary: "OrbyNode notification test".into(),
            source_stream: "notifications".into(),
            created_at: 0,
        })
        .await;
    state
        .record(Some(&user), "notification.test", "notifications", "")
        .await;
    Ok(StatusCode::NO_CONTENT)
}

fn require_manage(user: &User) -> Result<(), ApiError> {
    if orbynode_auth::role_has(user.role, Perm::SettingsManage) {
        Ok(())
    } else {
        Err(ApiError(StatusCode::FORBIDDEN))
    }
}
