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
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState").field("web", &self.web).finish()
    }
}

impl AppState {
    pub fn new(web: WebSource, terminals: Arc<orbynode_terminal::TerminalManager>) -> Self {
        AppState {
            started_at: Instant::now(),
            web,
            terminals,
        }
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
        }
    }
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
        .merge(terminal_routes::routes())
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
