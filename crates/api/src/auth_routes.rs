//! Authentication HTTP layer (Milestone 4, ADR 005): setup, login, logout,
//! me, session middleware, and CSRF enforcement.

use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use orbynode_auth::{AuthError, SessionInfo};
use serde_json::json;

use crate::{ApiError, AppState};

pub const SESSION_COOKIE: &str = "orbynode_session";
pub const CSRF_HEADER: &str = "X-Orbynode-CSRF";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/setup", get(setup_status).post(setup_owner))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
}

// ---------- Handlers ----------

async fn setup_status(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiError> {
    let pending = state.auth.setup_pending().await.map_err(auth_err)?;
    Ok(Json(json!({"setup_pending": pending})))
}

#[derive(serde::Deserialize)]
struct SetupBody {
    username: String,
    display_name: String,
    password: String,
}

async fn setup_owner(
    State(state): State<AppState>,
    Json(body): Json<SetupBody>,
) -> Result<Response, ApiError> {
    // Bootstrap owner creation; 403 once any user exists (ADR 005).
    state
        .auth
        .setup_owner(&body.username, &body.display_name, &body.password)
        .await
        .map_err(|e| match e {
            AuthError::SetupCompleted => ApiError(StatusCode::FORBIDDEN),
            other => ApiError::from(other),
        })?;
    let session = state
        .auth
        .login(&body.username, &body.password)
        .await
        .map_err(ApiError::from)?;
    let mut resp = (StatusCode::CREATED, Json(session.clone())).into_response();
    set_session_cookie(&mut resp, &session, false);
    Ok(resp)
}

#[derive(serde::Deserialize)]
struct LoginBody {
    username: String,
    password: String,
}

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginBody>,
) -> Result<Response, ApiError> {
    let session = state
        .auth
        .login(&body.username, &body.password)
        .await
        .map_err(|e| match e {
            AuthError::Locked => ApiError(StatusCode::TOO_MANY_REQUESTS),
            _ => ApiError(StatusCode::UNAUTHORIZED),
        })?;
    // Session (incl. csrf) in the body for non-cookie clients, cookie in the
    // header for browser sessions.
    let mut resp = Json(session.clone()).into_response();
    set_session_cookie(&mut resp, &session, false);
    Ok(resp)
}

async fn logout(State(state): State<AppState>, req: Request) -> Result<StatusCode, ApiError> {
    if let Some(token) = cookie_token(req.headers()) {
        state.auth.logout(&token).await.map_err(ApiError::from)?;
    }
    let mut resp = StatusCode::NO_CONTENT.into_response();
    clear_session_cookie(&mut resp);
    Ok(StatusCode::NO_CONTENT)
}

async fn me(
    State(state): State<AppState>,
    req: Request,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Some(token) = cookie_token(req.headers()) else {
        return Err(ApiError(StatusCode::UNAUTHORIZED));
    };
    let Some(user) = state
        .auth
        .user_for_session(&token)
        .await
        .map_err(ApiError::from)?
    else {
        return Err(ApiError(StatusCode::UNAUTHORIZED));
    };
    Ok(Json(json!({
        "id": user.id,
        "username": user.username,
        "display_name": user.display_name,
        "role": user.role.as_str(),
    })))
}

// ---------- Middleware ----------

/// Require a valid session; CSRF header on mutations. Attaches the resolved
/// user to request extensions for handlers.
pub async fn require_auth(State(state): State<AppState>, mut req: Request, next: Next) -> Response {
    let Some(token) = cookie_token(req.headers()) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let user = match state.auth.user_for_session(&token).await {
        Ok(Some(u)) => u,
        Ok(None) => return StatusCode::UNAUTHORIZED.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "session lookup failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    // CSRF: mutations need the header to match the session's csrf.
    if is_mutating(req.method()) {
        let csrf_ok = state
            .auth
            .csrf_for_session(&token)
            .await
            .ok()
            .flatten()
            .zip(req.headers().get(CSRF_HEADER).and_then(|v| v.to_str().ok()))
            .map(|(expected, got)| expected == got)
            .unwrap_or(false);
        if !csrf_ok {
            return (StatusCode::FORBIDDEN, "missing or invalid CSRF header").into_response();
        }
    }
    req.extensions_mut().insert(user);
    next.run(req).await
}

// ---------- helpers ----------

fn auth_err(e: AuthError) -> ApiError {
    match e {
        AuthError::SetupCompleted => ApiError(StatusCode::FORBIDDEN),
        _ => ApiError(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn is_mutating(method: &axum::http::Method) -> bool {
    !matches!(
        *method,
        axum::http::Method::GET | axum::http::Method::HEAD | axum::http::Method::OPTIONS
    )
}

pub fn cookie_token(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(header::COOKIE)?.to_str().ok()?;
    for pair in raw.split(';') {
        let pair = pair.trim();
        if let Some(value) = pair.strip_prefix(SESSION_COOKIE) {
            let value = value.strip_prefix('=')?;
            if value.len() == 64 && value.chars().all(|c| c.is_ascii_hexdigit()) {
                return Some(value.to_owned());
            }
        }
    }
    None
}

fn set_session_cookie(resp: &mut Response, session: &SessionInfo, secure: bool) {
    let _ = session.expires_at;
    let mut cookie = format!(
        "{SESSION_COOKIE}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}",
        session.token,
        7 * 24 * 3600
    );
    if secure {
        cookie.push_str("; Secure");
    }
    resp.headers_mut()
        .append(header::SET_COOKIE, cookie.parse().expect("cookie header"));
}

fn clear_session_cookie(resp: &mut Response) {
    let cookie = format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0");
    resp.headers_mut()
        .append(header::SET_COOKIE, cookie.parse().expect("cookie header"));
}

pub use orbynode_auth::AuthService;
