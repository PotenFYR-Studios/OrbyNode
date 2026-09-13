//! Workflow REST API (Milestone 16, Plan §139).

use std::collections::BTreeMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use orbynode_workflows::{WorkflowDefinition, WorkflowRun};
use serde::Deserialize;

use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/workflows", get(list_definitions).post(create_definition))
        .route("/workflows/{name}", get(get_definition))
        .route("/workflows/{name}/start", post(start_workflow))
        .route("/workflow-runs/latest", get(latest_run))
        .route("/workflow-runs/{id}", get(get_run))
        .route("/workflow-runs/{id}/advance", post(advance_run))
        .route("/workflow-runs/{id}/approve", post(approve_run))
        .route("/workflow-runs/{id}/cancel", post(cancel_run))
}

#[derive(Deserialize)]
struct DefinitionBody {
    name: String,
    steps: Vec<orbynode_workflows::WorkflowStep>,
}

#[derive(Deserialize)]
struct StartBody {
    variables: BTreeMap<String, String>,
}

async fn list_definitions(State(state): State<AppState>) -> Json<Vec<WorkflowDefinition>> {
    Json(state.workflows.list_definitions().await)
}

async fn get_definition(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<WorkflowDefinition>, ApiError> {
    state
        .workflows
        .get_definition(&name)
        .await?
        .map(Json)
        .ok_or(ApiError(StatusCode::NOT_FOUND))
}

async fn create_definition(
    State(state): State<AppState>,
    Json(body): Json<DefinitionBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let definition = WorkflowDefinition {
        name: body.name,
        steps: body.steps,
    };
    let id = state.workflows.put_definition(definition).await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

async fn start_workflow(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<StartBody>,
) -> Result<Json<WorkflowRun>, ApiError> {
    let run = state.workflows.start(&name, body.variables).await?;
    publish_run(&state, &run).await;
    Ok(Json(run))
}

async fn latest_run(State(state): State<AppState>) -> Json<Option<WorkflowRun>> {
    Json(state.workflows.latest_run().await)
}

async fn get_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<WorkflowRun>, ApiError> {
    state
        .workflows
        .get_run(&id)
        .await?
        .map(Json)
        .ok_or(ApiError(StatusCode::NOT_FOUND))
}

async fn advance_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<WorkflowRun>, ApiError> {
    let run = state.workflows.advance(&id).await?;
    publish_run(&state, &run).await;
    Ok(Json(run))
}

async fn approve_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<WorkflowRun>, ApiError> {
    let run = state.workflows.approve(&id).await?;
    publish_run(&state, &run).await;
    Ok(Json(run))
}

async fn cancel_run(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<WorkflowRun>, ApiError> {
    let run = state.workflows.cancel(&id).await?;
    publish_run(&state, &run).await;
    Ok(Json(run))
}

async fn publish_run(state: &AppState, run: &WorkflowRun) {
    state.realtime.publish(orbynode_realtime::Event {
        stream: orbynode_realtime::Stream::new("workflows".to_string()),
        etype: "workflow.updated".into(),
        data: serde_json::to_value(run).unwrap_or_default(),
        priority: orbynode_realtime::Priority::Critical,
        bytes: Vec::new(),
    });
}

impl From<orbynode_workflows::WorkflowError> for ApiError {
    fn from(error: orbynode_workflows::WorkflowError) -> Self {
        use orbynode_workflows::WorkflowError;
        match error {
            WorkflowError::NotFound => ApiError(StatusCode::NOT_FOUND),
            WorkflowError::Invalid(_) => ApiError(StatusCode::BAD_REQUEST),
            WorkflowError::Io(_) | WorkflowError::Database(_) | WorkflowError::Serialization(_) => {
                tracing::error!("workflow error");
                ApiError(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_router;
    use axum::body::Body;
    use axum::http::{Request, header};
    use orbynode_auth::SessionInfo;
    use orbynode_workflows::RunStatus;
    use tower::ServiceExt;

    async fn authenticated(app: axum::Router) -> (axum::Router, SessionInfo) {
        let response = app
            .clone()
            .oneshot(
                Request::post("/setup")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"username":"workflow-owner","display_name":"Workflow Owner","password":"good-pass-workflow"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        if status != StatusCode::CREATED {
            let response = app
                .clone()
                .oneshot(
                    Request::post("/login")
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(
                            r#"{"username":"workflow-owner","password":"good-pass-workflow"}"#,
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            let status = response.status();
            assert_eq!(status, StatusCode::OK, "unexpected login status");
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
            let session = SessionInfo {
                token: value["token"].as_str().unwrap().to_owned(),
                user_id: value["user_id"].as_i64().unwrap(),
                csrf: value["csrf"].as_str().unwrap().to_owned(),
                expires_at: value["expires_at"].as_i64().unwrap(),
            };
            return (app, session);
        }
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let session = SessionInfo {
            token: value["token"].as_str().unwrap().to_owned(),
            user_id: value["user_id"].as_i64().unwrap(),
            csrf: value["csrf"].as_str().unwrap().to_owned(),
            expires_at: value["expires_at"].as_i64().unwrap(),
        };
        (app, session)
    }

    fn authed_request(
        method: axum::http::Method,
        uri: &str,
        session: &SessionInfo,
    ) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header(
                header::COOKIE,
                format!("{}={}", crate::auth_routes::SESSION_COOKIE, session.token),
            )
            .header(crate::auth_routes::CSRF_HEADER, session.csrf.clone())
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn workflow_lifecycle_is_exposed() {
        let state = AppState {
            db: orbynode_database::Db::open("sqlite::memory:")
                .await
                .unwrap(),
            ..AppState::default()
        };
        state
            .auth
            .setup_owner("workflow-owner", "Workflow Owner", "good-pass-workflow")
            .await
            .unwrap();
        let (app, session) = authenticated(build_router(state)).await;
        let definition = serde_json::json!({
            "name": "delivery",
            "steps": [
                {"name": "test", "kind": "agent", "agents": ["ci"], "command": "true"},
                {"name": "approve", "kind": "approval", "agents": [], "command": null}
            ]
        });
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/workflows")
                    .header(
                        header::COOKIE,
                        format!("{}={}", crate::auth_routes::SESSION_COOKIE, session.token),
                    )
                    .header(crate::auth_routes::CSRF_HEADER, session.csrf.clone())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(definition.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(axum::http::Method::POST)
                    .uri("/workflows/delivery/start")
                    .header(
                        header::COOKIE,
                        format!("{}={}", crate::auth_routes::SESSION_COOKIE, session.token),
                    )
                    .header(crate::auth_routes::CSRF_HEADER, session.csrf.clone())
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"variables":{}}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let run: WorkflowRun = serde_json::from_slice(&body).unwrap();
        assert_eq!(run.status, RunStatus::WaitingApproval);

        let response = app
            .oneshot(authed_request(
                axum::http::Method::POST,
                &format!("/workflow-runs/{}/approve", run.id),
                &session,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
