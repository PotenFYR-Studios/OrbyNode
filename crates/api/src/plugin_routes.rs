//! Plugin registry REST (ADR 018): manifest records, enable/disable.
//! No plugin execution - isolation lands before any runtime (Plan §145).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, patch};
use axum::{Extension, Json, Router};

use crate::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/plugins", get(list))
        .route("/plugins/{id}", patch(update).get(get_one))
}

fn require(user: &orbynode_auth::User, perm: orbynode_auth::Perm) -> Result<(), ApiError> {
    if orbynode_auth::role_has(user.role, perm) {
        Ok(())
    } else {
        Err(ApiError(StatusCode::FORBIDDEN))
    }
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct PluginRow {
    id: String,
    name: String,
    manifest: String,
    enabled: bool,
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<PluginRow>>, ApiError> {
    let rows: Vec<PluginRow> =
        sqlx::query_as("SELECT id, name, manifest, enabled FROM plugins ORDER BY name")
            .fetch_all(&state.db.pool)
            .await
            .map_err(|e| ApiError::from(orbynode_database::DbError::Sqlx(e)))?;
    Ok(Json(rows))
}

async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PluginRow>, ApiError> {
    let row: PluginRow =
        sqlx::query_as("SELECT id, name, manifest, enabled FROM plugins WHERE id = ?")
            .bind(&id)
            .fetch_optional(&state.db.pool)
            .await
            .map_err(|e| ApiError::from(orbynode_database::DbError::Sqlx(e)))?
            .ok_or(ApiError(StatusCode::NOT_FOUND))?;
    Ok(Json(row))
}

#[derive(serde::Deserialize)]
struct UpdateBody {
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    manifest: Option<String>,
}

async fn update(
    State(state): State<AppState>,
    Extension(user): Extension<orbynode_auth::User>,
    Path(id): Path<String>,
    Json(body): Json<UpdateBody>,
) -> Result<StatusCode, ApiError> {
    require(&user, orbynode_auth::Perm::UsersManage)?;
    if let Some(enabled) = body.enabled {
        sqlx::query(
            "UPDATE plugins SET enabled = ?, updated_at = strftime('%s','now') WHERE id = ?",
        )
        .bind(enabled)
        .bind(&id)
        .execute(&state.db.pool)
        .await
        .map_err(|e| ApiError::from(orbynode_database::DbError::Sqlx(e)))?;
        state
            .record(
                Some(&user),
                "plugin.update",
                &format!("plugin:{id}"),
                if enabled { "enabled" } else { "disabled" },
            )
            .await;
    }
    if let Some(manifest) = &body.manifest {
        // Validate the manifest is well-formed JSON before storing (ADR 018:
        // records are declarative; a broken manifest must not persist).
        if serde_json::from_str::<serde_json::Value>(manifest).is_err() {
            return Err(ApiError(StatusCode::BAD_REQUEST));
        }
        sqlx::query(
            "UPDATE plugins SET manifest = ?, updated_at = strftime('%s','now') WHERE id = ?",
        )
        .bind(manifest)
        .bind(&id)
        .execute(&state.db.pool)
        .await
        .map_err(|e| ApiError::from(orbynode_database::DbError::Sqlx(e)))?;
    }
    Ok(StatusCode::NO_CONTENT)
}
