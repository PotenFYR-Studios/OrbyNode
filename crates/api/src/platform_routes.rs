//! Versioned platform APIs (Milestone 17, Plan §140): REST, webhooks,
//! MCP-style tool manifest, plugin manifest, and event stream helpers.

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{ApiError, AppState};

pub const API_VERSION: &str = "v1";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/tokens", post(create_token).get(list_tokens))
        .route("/api/v1/tokens/{id}/revoke", post(revoke_token))
        .route("/api/v1/webhooks", get(list_webhooks).post(create_webhook))
        .route(
            "/api/v1/webhooks/{name}",
            put(update_webhook).delete(delete_webhook),
        )
        .route("/api/v1/events", post(publish_event))
        .route("/api/v1/mcp", get(mcp_manifest))
        .route("/api/v1/mcp/tools/{tool}/invoke", post(invoke_mcp_tool))
        .route("/api/v1/plugins", get(list_plugins).post(register_plugin))
        .route("/api/v1/plugins/{id}", get(get_plugin).put(update_plugin))
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ApiToken {
    pub id: String,
    pub name: String,
    pub scopes: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub revoked_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Webhook {
    pub name: String,
    pub url: String,
    pub events: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub manifest: String,
    pub enabled: bool,
}

#[derive(Deserialize)]
struct CreateToken {
    name: String,
    #[serde(default)]
    scopes: Vec<String>,
    #[serde(default)]
    expires_at: Option<i64>,
}

#[derive(Deserialize)]
struct WebhookBody {
    name: String,
    url: String,
    secret: String,
    #[serde(default)]
    events: Vec<String>,
    #[serde(default = "bool::default")]
    enabled: bool,
}

#[derive(Deserialize)]
struct EventBody {
    stream: String,
    #[serde(rename = "type")]
    etype: String,
    #[serde(default)]
    data: serde_json::Value,
    #[serde(default)]
    critical: bool,
}

#[derive(Deserialize)]
struct PluginBody {
    id: String,
    name: String,
    version: String,
    #[serde(default)]
    permissions: Vec<String>,
    #[serde(default)]
    enabled: bool,
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn random_id(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{prefix}-{nanos:x}")
}

fn token_hash(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

async fn insert_token(
    state: &AppState,
    id: &str,
    name: &str,
    token: &str,
    scopes: &[String],
    expires_at: Option<i64>,
) -> Result<ApiToken, ApiError> {
    let created_at = now();
    let scopes = serde_json::to_string(scopes).unwrap_or_else(|_| "[]".into());
    sqlx::query("INSERT INTO api_tokens (id, name, token_hash, scopes, created_at, expires_at) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(id)
        .bind(name)
        .bind(token_hash(token))
        .bind(&scopes)
        .bind(created_at)
        .bind(expires_at)
        .execute(&state.db.pool)
        .await
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR))?;
    Ok(ApiToken {
        id: id.to_owned(),
        name: name.to_owned(),
        scopes,
        created_at,
        expires_at,
        revoked_at: None,
    })
}

async fn create_token(
    State(state): State<AppState>,
    Json(body): Json<CreateToken>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    let id = random_id("tok");
    let secret = format!(
        "obn_{}_{}",
        id,
        token_hash(&format!("{}:{}", id, now()))
            .get(..32)
            .unwrap_or("secret")
    );
    let token = insert_token(
        &state,
        &id,
        &body.name,
        &secret,
        &body.scopes,
        body.expires_at,
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"token": token, "secret": secret})),
    ))
}

async fn list_tokens(State(state): State<AppState>) -> Result<Json<Vec<ApiToken>>, ApiError> {
    let tokens = sqlx::query_as::<_, ApiToken>(
        "SELECT id, name, scopes, created_at, expires_at, revoked_at FROM api_tokens ORDER BY created_at DESC",
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR))?;
    Ok(Json(tokens))
}

async fn revoke_token(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let result =
        sqlx::query("UPDATE api_tokens SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL")
            .bind(now())
            .bind(&id)
            .execute(&state.db.pool)
            .await
            .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR))?;
    if result.rows_affected() == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn upsert_webhook(state: &AppState, body: WebhookBody) -> Result<Webhook, ApiError> {
    let timestamp = now();
    let events = serde_json::to_string(&body.events).unwrap_or_else(|_| "[]".into());
    sqlx::query("INSERT INTO webhooks (name, url, secret, events, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?) ON CONFLICT(name) DO UPDATE SET url = excluded.url, secret = excluded.secret, events = excluded.events, enabled = excluded.enabled, updated_at = excluded.updated_at")
        .bind(&body.name)
        .bind(&body.url)
        .bind(body.secret)
        .bind(&events)
        .bind(body.enabled)
        .bind(timestamp)
        .bind(timestamp)
        .execute(&state.db.pool)
        .await
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR))?;
    Ok(Webhook {
        name: body.name,
        url: body.url,
        events,
        enabled: body.enabled,
    })
}

async fn create_webhook(
    State(state): State<AppState>,
    Json(body): Json<WebhookBody>,
) -> Result<(StatusCode, Json<Webhook>), ApiError> {
    let webhook = upsert_webhook(&state, body).await?;
    Ok((StatusCode::CREATED, Json(webhook)))
}

async fn update_webhook(
    State(state): State<AppState>,
    Json(body): Json<WebhookBody>,
) -> Result<Json<Webhook>, ApiError> {
    Ok(Json(upsert_webhook(&state, body).await?))
}

async fn list_webhooks(State(state): State<AppState>) -> Result<Json<Vec<Webhook>>, ApiError> {
    let rows = sqlx::query_as::<_, Webhook>(
        "SELECT name, url, events, enabled FROM webhooks ORDER BY name",
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR))?;
    Ok(Json(rows))
}

async fn delete_webhook(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<StatusCode, ApiError> {
    let result = sqlx::query("DELETE FROM webhooks WHERE name = ?")
        .bind(&name)
        .execute(&state.db.pool)
        .await
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR))?;
    if result.rows_affected() == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND));
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn publish_event(
    State(state): State<AppState>,
    Json(body): Json<EventBody>,
) -> Result<StatusCode, ApiError> {
    if body.stream.trim().is_empty() || body.etype.trim().is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST));
    }
    state.realtime.publish(orbynode_realtime::Event {
        stream: orbynode_realtime::Stream::new(body.stream),
        etype: body.etype,
        data: body.data,
        priority: if body.critical {
            orbynode_realtime::Priority::Critical
        } else {
            orbynode_realtime::Priority::Droppable
        },
        bytes: Vec::new(),
    });
    Ok(StatusCode::ACCEPTED)
}

fn mcp_tools() -> serde_json::Value {
    json!([
        {
            "name": "dashboard.snapshot",
            "description": "Read current attention, nodes, metrics, and workflow state.",
            "input_schema": {"type": "object", "properties": {}}
        },
        {
            "name": "terminal.create",
            "description": "Create a durable terminal session.",
            "input_schema": {
                "type": "object",
                "properties": {"project_id": {"type": "integer"}, "title": {"type": "string"}},
                "required": ["project_id"]
            }
        },
        {
            "name": "task.move",
            "description": "Move a task with optimistic concurrency.",
            "input_schema": {
                "type": "object",
                "properties": {"id": {"type": "integer"}, "state": {"type": "string"}, "version": {"type": "integer"}},
                "required": ["id", "state", "version"]
            }
        }
    ])
}

async fn mcp_manifest(State(state): State<AppState>) -> Json<serde_json::Value> {
    let tools =
        sqlx::query_as::<_, (String, bool)>("SELECT name, enabled FROM mcp_tools ORDER BY name")
            .fetch_all(&state.db.pool)
            .await
            .unwrap_or_default();
    let custom = tools
        .into_iter()
        .filter(|(_, enabled)| *enabled)
        .map(|(name, _)| json!({"name": name, "description": "Custom MCP tool", "input_schema": {"type": "object"}}));
    let mut all = mcp_tools();
    if let Some(array) = all.as_array_mut() {
        array.extend(custom);
    }
    Json(json!({
        "protocol": "orbynode.mcp",
        "version": 1,
        "tools": all
    }))
}

#[derive(Deserialize)]
struct ToolInvocation {
    #[serde(default)]
    args: serde_json::Value,
}

async fn invoke_mcp_tool(
    State(state): State<AppState>,
    Path(tool): Path<String>,
    Json(body): Json<ToolInvocation>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match tool.as_str() {
        "dashboard.snapshot" => {
            let attention = state.attention.list();
            let nodes = state
                .nodes
                .nodes()
                .await
                .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR))?;
            let host = orbynode_services::host_snapshot().await;
            Ok(Json(json!({
                "attention": attention,
                "nodes": nodes,
                "host": host
            })))
        }
        "terminal.create" => {
            let project_id = body.args["project_id"]
                .as_i64()
                .ok_or(ApiError(StatusCode::BAD_REQUEST))?;
            let Some(project) = state.db.get_project(project_id).await? else {
                return Err(ApiError(StatusCode::NOT_FOUND));
            };
            let cfg = orbynode_terminal::TerminalConfig {
                cwd: Some(std::path::PathBuf::from(project.path)),
                ..orbynode_terminal::TerminalConfig::default()
            };
            let term = state.terminals.create(cfg).map_err(crate::ApiError::from)?;
            state.bridge_terminal_to_bus(term.id());
            Ok(Json(json!({"id": term.id()})))
        }
        "task.move" => {
            let id = body.args["id"]
                .as_i64()
                .ok_or(ApiError(StatusCode::BAD_REQUEST))?;
            let task_state = body.args["state"]
                .as_str()
                .ok_or(ApiError(StatusCode::BAD_REQUEST))?;
            let version = body.args["version"]
                .as_i64()
                .ok_or(ApiError(StatusCode::BAD_REQUEST))?;
            let state_enum = orbynode_database::TaskState::parse(task_state);
            match state.db.move_task(id, state_enum, version).await {
                Ok(Ok(task)) => Ok(Json(json!(task))),
                Ok(Err(_)) => Err(ApiError(StatusCode::CONFLICT)),
                Err(_) => Err(ApiError(StatusCode::NOT_FOUND)),
            }
        }
        _ => {
            let enabled: Option<bool> =
                sqlx::query_scalar("SELECT enabled FROM mcp_tools WHERE name = ?")
                    .bind(&tool)
                    .fetch_optional(&state.db.pool)
                    .await
                    .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR))?;
            match enabled {
                Some(true) => Ok(Json(json!({"ok": true}))),
                Some(false) => Err(ApiError(StatusCode::FORBIDDEN)),
                None => Err(ApiError(StatusCode::NOT_FOUND)),
            }
        }
    }
}

fn manifest(body: &PluginBody) -> String {
    json!({
        "id": body.id,
        "name": body.name,
        "version": body.version,
        "permissions": body.permissions,
        "api_version": API_VERSION
    })
    .to_string()
}

async fn upsert_plugin(state: &AppState, body: PluginBody) -> Result<Plugin, ApiError> {
    if body.id.trim().is_empty() || body.name.trim().is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST));
    }
    let timestamp = now();
    let manifest = manifest(&body);
    sqlx::query("INSERT INTO plugins (id, name, manifest, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET name = excluded.name, manifest = excluded.manifest, enabled = excluded.enabled, updated_at = excluded.updated_at")
        .bind(&body.id)
        .bind(&body.name)
        .bind(&manifest)
        .bind(body.enabled)
        .bind(timestamp)
        .bind(timestamp)
        .execute(&state.db.pool)
        .await
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR))?;
    Ok(Plugin {
        id: body.id,
        name: body.name,
        manifest,
        enabled: body.enabled,
    })
}

async fn register_plugin(
    State(state): State<AppState>,
    Json(body): Json<PluginBody>,
) -> Result<(StatusCode, Json<Plugin>), ApiError> {
    let plugin = upsert_plugin(&state, body).await?;
    Ok((StatusCode::CREATED, Json(plugin)))
}

async fn update_plugin(
    State(state): State<AppState>,
    Json(body): Json<PluginBody>,
) -> Result<Json<Plugin>, ApiError> {
    Ok(Json(upsert_plugin(&state, body).await?))
}

async fn list_plugins(State(state): State<AppState>) -> Result<Json<Vec<Plugin>>, ApiError> {
    let plugins = sqlx::query_as::<_, Plugin>(
        "SELECT id, name, manifest, enabled FROM plugins ORDER BY name",
    )
    .fetch_all(&state.db.pool)
    .await
    .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR))?;
    Ok(Json(plugins))
}

async fn get_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Plugin>, ApiError> {
    sqlx::query_as::<_, Plugin>("SELECT id, name, manifest, enabled FROM plugins WHERE id = ?")
        .bind(&id)
        .fetch_optional(&state.db.pool)
        .await
        .map_err(|_| ApiError(StatusCode::INTERNAL_SERVER_ERROR))?
        .map(Json)
        .ok_or(ApiError(StatusCode::NOT_FOUND))
}

pub fn authorized(headers: &HeaderMap) -> bool {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use orbynode_auth::SessionInfo;
    use tower::ServiceExt;

    async fn authenticated(app: axum::Router) -> (axum::Router, SessionInfo) {
        let response = app
            .clone()
            .oneshot(
                Request::post("/setup")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"username":"owner","display_name":"Owner","password":"good-pass-1"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let value: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        (
            app,
            SessionInfo {
                token: value["token"].as_str().unwrap().to_owned(),
                user_id: value["user_id"].as_i64().unwrap(),
                csrf: value["csrf"].as_str().unwrap().to_owned(),
                expires_at: value["expires_at"].as_i64().unwrap(),
            },
        )
    }

    #[tokio::test]
    async fn platform_manifests_and_interfaces_are_versioned() {
        let (app, session) = authenticated(crate::build_router(AppState::default())).await;
        let auth = [
            (
                "cookie",
                format!("{}={}", crate::auth_routes::SESSION_COOKIE, session.token),
            ),
            ("csrf", session.csrf.clone()),
        ];
        let response = app
            .clone()
            .oneshot(
                Request::get("/api/v1/mcp")
                    .header("cookie", auth[0].1.clone())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::post("/api/v1/events")
                    .header("cookie", auth[0].1.clone())
                    .header(crate::auth_routes::CSRF_HEADER, auth[1].1.clone())
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"stream":"api.test","type":"hello"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        let response = app
            .oneshot(
                Request::post("/api/v1/plugins")
                    .header("cookie", auth[0].1.clone())
                    .header(crate::auth_routes::CSRF_HEADER, auth[1].1.clone())
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"id":"demo","name":"Demo","version":"1.0.0"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }
}
