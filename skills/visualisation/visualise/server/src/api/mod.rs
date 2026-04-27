mod docs;
mod events;
mod lifecycle;
mod templates;
mod types;

use std::sync::Arc;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};

use crate::server::AppState;

pub fn mount(_state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/events", get(events::events))
        .route("/api/types", get(types::types))
        .route("/api/docs", get(docs::docs_list))
        .route("/api/docs/*path", get(docs::doc_fetch))
        .route("/api/templates", get(templates::templates_list))
        .route("/api/templates/:name", get(templates::template_detail))
        .route("/api/lifecycle", get(lifecycle::lifecycle_list))
        .route("/api/lifecycle/:slug", get(lifecycle::lifecycle_one))
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ApiError {
    #[error("invalid doc type: {0}")]
    InvalidDocType(String),
    #[error("path escape")]
    PathEscape,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            ApiError::InvalidDocType(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            ApiError::PathEscape => (StatusCode::FORBIDDEN, "path escape".into()),
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}

pub(crate) fn parse_kind(s: &str) -> Option<crate::docs::DocTypeKey> {
    serde_json::from_str::<crate::docs::DocTypeKey>(&format!("\"{s}\"")).ok()
}

pub(crate) fn api_from_fd(e: crate::file_driver::FileDriverError) -> ApiError {
    use crate::file_driver::FileDriverError as F;
    match e {
        F::PathEscape { .. } | F::TypeNotConfigured { .. } => ApiError::PathEscape,
        F::NotFound { path } => ApiError::NotFound(path.display().to_string()),
        F::TooLarge { path, size, limit } => ApiError::Internal(format!(
            "{} is {} bytes (limit {})",
            path.display(),
            size,
            limit
        )),
        F::Io { source, .. } => ApiError::Internal(source.to_string()),
        // Write-path errors — mapped properly in Phase 3; stub to Internal for now.
        F::EtagMismatch { .. } => ApiError::Internal("etag mismatch".into()),
        F::Patch(p) => ApiError::Internal(p.to_string()),
        F::PathNotWritable { .. } => ApiError::Internal("path not writable".into()),
        F::CrossFilesystem { .. } => ApiError::Internal("cross-filesystem rename".into()),
    }
}
