mod docs;
mod events;
pub(crate) mod info;
mod lifecycle;
mod related;
mod templates;
mod types;
mod work_item_config;

use std::sync::Arc;

use axum::{
    http::{header, HeaderValue, StatusCode},
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
        // PATCH URL exposed to clients is /api/docs/{path}/frontmatter, but matchit 0.7
        // forbids catch-all + literal suffix (https://github.com/ibraheemdev/matchit/issues/39).
        // Both method handlers share this route; the PATCH handler strips the trailing
        // /frontmatter suffix from *path and returns 400 if it is absent.
        .route(
            "/api/docs/*path",
            get(docs::doc_fetch).patch(docs::doc_patch_frontmatter),
        )
        .route("/api/templates", get(templates::templates_list))
        .route("/api/templates/:name", get(templates::template_detail))
        .route("/api/lifecycle", get(lifecycle::lifecycle_list))
        .route("/api/lifecycle/:slug", get(lifecycle::lifecycle_one))
        .route("/api/related/*path", get(related::related_get))
        .route("/api/work-item/config", get(work_item_config::get_work_item_config))
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
    // Write-path errors
    #[error("patch URL must end with /frontmatter")]
    PatchEndpointMismatch,
    #[error("only work-items are writable")]
    OnlyWorkItemsAreWritable,
    #[error("unknown kanban status")]
    UnknownKanbanStatus { accepted_keys: Vec<String> },
    #[error("invalid patch: {0}")]
    InvalidPatch(String),
    #[error("unsupported If-Match value: {0}")]
    UnsupportedIfMatch(String),
    #[error("If-Match header is required")]
    IfMatchRequired,
    #[error("etag mismatch")]
    EtagMismatch { current: String },
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::InvalidDocType(s) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": s })),
            )
                .into_response(),
            ApiError::PathEscape => (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({ "error": "path escape" })),
            )
                .into_response(),
            ApiError::NotFound(s) => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": s })),
            )
                .into_response(),
            ApiError::Internal(s) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": s })),
            )
                .into_response(),
            ApiError::PatchEndpointMismatch => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "patch URL must end with /frontmatter" })),
            )
                .into_response(),
            ApiError::OnlyWorkItemsAreWritable => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "only work-items are writable" })),
            )
                .into_response(),
            ApiError::UnknownKanbanStatus { accepted_keys } => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "unknown_kanban_status",
                    "acceptedKeys": accepted_keys
                })),
            )
                .into_response(),
            ApiError::InvalidPatch(s) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": s })),
            )
                .into_response(),
            ApiError::UnsupportedIfMatch(s) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": s })),
            )
                .into_response(),
            // 428 Precondition Required — no currentEtag, no ETag header.
            // A client that omits If-Match entirely demonstrated no prior knowledge,
            // so we do not leak the current state. This prevents the optimistic-
            // concurrency rollback UI from triggering on a client programming bug.
            ApiError::IfMatchRequired => (
                StatusCode::from_u16(428).unwrap(),
                Json(serde_json::json!({ "error": "if-match-required" })),
            )
                .into_response(),
            // 412 Precondition Failed — include currentEtag + ETag header.
            // The client demonstrated prior knowledge by sending an If-Match value;
            // echoing the current etag lets it retry with the up-to-date precondition.
            ApiError::EtagMismatch { current } => {
                let quoted = format!("\"{}\"", current);
                (
                    StatusCode::PRECONDITION_FAILED,
                    [(
                        header::ETAG,
                        HeaderValue::from_str(&quoted)
                            .unwrap_or_else(|_| HeaderValue::from_static("")),
                    )],
                    Json(serde_json::json!({ "currentEtag": current })),
                )
                    .into_response()
            }
        }
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
        F::EtagMismatch { current } => ApiError::EtagMismatch { current },
        F::Patch(p) => ApiError::InvalidPatch(p.to_string()),
        F::PathNotWritable { .. } => ApiError::OnlyWorkItemsAreWritable,
        F::CrossFilesystem { .. } => ApiError::Internal("cross-filesystem rename".into()),
    }
}
