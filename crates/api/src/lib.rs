//! OrbyNode HTTP API — REST endpoints and embedded web UI delivery (ADR 003, ADR 006).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;
use tower_http::trace::TraceLayer;

pub mod gateway;

/// Small error wrapper shared by route modules (avoids `result_large_err`).
#[derive(Debug)]
pub struct ApiError(pub StatusCode);

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> Response {
        self.0.into_response()
    }
}

impl From<orbynode_database::DbError> for ApiError {
    fn from(e: orbynode_database::DbError) -> Self {
        tracing::error!(error = %e, "db error");
        ApiError(StatusCode::INTERNAL_SERVER_ERROR)
    }
}
pub mod project_routes;
pub mod terminal_routes;

include!(concat!(env!("OUT_DIR"), "/embedded_assets.rs"));

/// Where the web UI is served from.
#[derive(Debug, Clone)]
pub enum WebSource {
    /// Compiled into the binary by `build.rs` (production).
    Embedded,
    /// Read from disk on each request (development override).
    Disk(PathBuf),
}

#[derive(Clone)]
pub struct AppState {
    pub started_at: Instant,
    pub web: WebSource,
    pub terminals: Arc<orbynode_terminal::TerminalManager>,
    pub realtime: Arc<orbynode_realtime::EventBus>,
    pub db: orbynode_database::Db,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState").field("web", &self.web).finish()
    }
}

impl AppState {
    pub fn new(
        web: WebSource,
        terminals: Arc<orbynode_terminal::TerminalManager>,
        realtime: Arc<orbynode_realtime::EventBus>,
        db: orbynode_database::Db,
    ) -> Self {
        AppState {
            started_at: Instant::now(),
            web,
            terminals,
            realtime,
            db,
        }
    }

    /// M2 gateway session hook: authorization is allow-all pre-auth (M4
    /// replaces it with the real ACL; ADR 009 keeps call sites stable).
    pub fn gateway_session(
        &self,
        _stream: &orbynode_realtime::Stream,
    ) -> Result<(), orbynode_realtime::ClientError> {
        Ok(())
    }

    pub fn gateway_unsub(&self, _stream: &orbynode_realtime::Stream) {
        // M2: per-connection registries live in the gateway task; no-op here.
    }

    /// Bridge a terminal's PTY broadcast into the realtime bus as a stream
    /// (`terminal:<id>`). Called once per terminal; capture stays single (§59).
    pub fn bridge_terminal_to_bus(&self, id: u64) {
        let Some(term) = self.terminals.get(id) else {
            return;
        };
        let bus = self.realtime.clone();
        let stream = orbynode_realtime::Stream::new(format!("terminal:{id}"));
        let mut rx = term.subscribe();
        tokio::spawn(async move {
            while let Ok(bytes) = rx.recv().await {
                bus.publish(orbynode_realtime::Event {
                    stream: stream.clone(),
                    etype: "terminal.output".into(),
                    data: serde_json::json!({}),
                    priority: orbynode_realtime::Priority::Droppable,
                    bytes,
                });
                // Bytes ride the envelope's sibling field, not JSON-escaped
                // (§65). The gateway encodes them as base64 in `data.bytes`.
                let _ = bytes;
            }
        });
    }

    /// Track a subscription (M2: always true; M11 revocation replaces this).
    pub fn gateway_track(&self, _stream: &orbynode_realtime::Stream) -> bool {
        true
    }
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            started_at: Instant::now(),
            web: WebSource::Embedded,
            terminals: orbynode_terminal::TerminalManager::new(
                orbynode_terminal::TerminalConfig::default(),
            ),
            realtime: Arc::new(orbynode_realtime::EventBus::new(
                orbynode_realtime::ReplayConfig::default(),
            )),
            db: test_db(),

        }
    }
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
        .merge(terminal_routes::routes())
        .merge(gateway::routes())
        .merge(project_routes::routes())
        .fallback(get(serve_web))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "uptime_secs": state.started_at.elapsed().as_secs(),
    }))
}

async fn version() -> Json<serde_json::Value> {
    Json(json!({
        "name": orbynode_core::NAME,
        "version": orbynode_core::version(),
    }))
}

/// Serve the SPA: exact asset if it exists, otherwise `index.html`.
async fn serve_web(State(state): State<AppState>, req: Request) -> Response {
    let path = req.uri().path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    // Path traversal guard (Plan §16): static assets are plain hashed names,
    // so fail closed on any encoding or dot-segment trickery.
    if path.contains('%') || path.split('/').any(|seg| seg == "..") {
        return StatusCode::NOT_FOUND.into_response();
    }

    match lookup(&state.web, path) {
        Some(bytes) => ([(header::CONTENT_TYPE, mime(path))], bytes).into_response(),
        None => {
            // SPA fallback for extension-less routes; missing real assets 404.
            if path.rsplit('.').next().is_some_and(|ext| ext != path) {
                return StatusCode::NOT_FOUND.into_response();
            }
            match lookup(&state.web, "index.html") {
                Some(bytes) => {
                    ([(header::CONTENT_TYPE, mime("index.html"))], bytes).into_response()
                }
                None => StatusCode::NOT_FOUND.into_response(),
            }
        }
    }
}

fn lookup(source: &WebSource, rel: &str) -> Option<Vec<u8>> {
    match source {
        WebSource::Embedded => ASSETS
            .iter()
            .find(|(name, _)| *name == rel)
            .map(|(_, bytes)| bytes.to_vec()),
        WebSource::Disk(root) => std::fs::read(root.join(rel)).ok(),
    }
}

fn mime(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") | Some("map") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

/// Process-wide default in-memory DB for default/test state. A static runtime
/// avoids nested-runtime panics when `default()` runs inside tokio tests.
fn test_db() -> orbynode_database::Db {
    static DB: std::sync::OnceLock<orbynode_database::Db> = std::sync::OnceLock::new();
    DB.get_or_init(|| {
        std::thread::spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("test-db runtime")
                .block_on(async {
                    orbynode_database::Db::open("sqlite::memory:")
                        .await
                        .expect("test db")
                })
        })
        .join()
        .expect("test-db thread")
    })
    .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use tower::ServiceExt;

    async fn get(uri: &str) -> (StatusCode, String) {
        let app = build_router(AppState::default());
        let resp = app
            .oneshot(HttpRequest::get(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8_lossy(&bytes).into_owned())
    }

    #[tokio::test]
    async fn health_reports_ok() {
        let (status, body) = get("/health").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("\"status\":\"ok\""), "{body}");
    }

    #[tokio::test]
    async fn version_reports_name_and_version() {
        let (status, body) = get("/version").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains(orbynode_core::NAME), "{body}");
        assert!(body.contains(orbynode_core::version()), "{body}");
    }

    #[tokio::test]
    async fn index_served_with_spa_fallback() {
        let (status, body) = get("/").await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("OrbyNode"), "{body}");
        let (status, _) = get("/some/spa/route").await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn traversal_is_rejected() {
        let (status, _) = get("/%2e%2e/%2e%2e/etc/passwd").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = get("/../etc/passwd").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn missing_asset_404s() {
        let (status, _) = get("/missing-9ab.js").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
