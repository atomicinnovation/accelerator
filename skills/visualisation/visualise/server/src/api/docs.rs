use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::{
    body::Body,
    extract::{Path as AxumPath, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use super::{api_from_fd, parse_kind, ApiError};
use crate::docs::DocTypeKey;
use crate::file_driver::FileDriver;
use crate::indexer::IndexEntry;
use crate::server::AppState;

#[derive(Debug, Deserialize)]
pub(crate) struct DocsListQuery {
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Serialize)]
struct DocsListResponse {
    docs: Vec<IndexEntry>,
}

pub(crate) async fn docs_list(
    State(state): State<Arc<AppState>>,
    Query(q): Query<DocsListQuery>,
) -> Result<Response, ApiError> {
    let kind = parse_kind(&q.type_).ok_or(ApiError::InvalidDocType(q.type_.clone()))?;
    if kind == DocTypeKey::Templates {
        return Err(ApiError::InvalidDocType(q.type_.clone()));
    }
    let mut entries: Vec<IndexEntry> = state.indexer.all_by_type(kind).await;
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Json(DocsListResponse { docs: entries }).into_response())
}

pub(crate) async fn doc_fetch(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    if path.contains("..") || path.starts_with('/') {
        return Err(ApiError::PathEscape);
    }
    let abs = state.cfg.project_root.join(&path);

    let entry = state.indexer.get(&abs).await;
    if let Some(ref e) = entry {
        if let Some(inm) = headers.get("if-none-match") {
            let quoted = format!("\"{}\"", e.etag);
            if inm.to_str().ok() == Some(&quoted) || inm.to_str().ok() == Some(&e.etag) {
                return Ok((StatusCode::NOT_MODIFIED, [(header::ETAG, quoted)]).into_response());
            }
        }
    }

    let (etag, bytes) = match entry {
        Some(e) => {
            let content = state.file_driver.read(&e.path).await.map_err(api_from_fd)?;
            (e.etag, content.bytes)
        }
        None => {
            let content = state.file_driver.read(&abs).await.map_err(api_from_fd)?;
            if let Some(inm) = headers.get("if-none-match") {
                let quoted = format!("\"{}\"", content.etag);
                if inm.to_str().ok() == Some(&quoted) || inm.to_str().ok() == Some(&content.etag) {
                    return Ok((StatusCode::NOT_MODIFIED, [(header::ETAG, quoted)]).into_response());
                }
            }
            (content.etag, content.bytes)
        }
    };

    Ok((
        StatusCode::OK,
        [
            (
                header::ETAG,
                HeaderValue::from_str(&format!("\"{etag}\"")).unwrap(),
            ),
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/markdown; charset=utf-8"),
            ),
        ],
        Body::from(bytes),
    )
        .into_response())
}

// ── Write structs ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PatchFrontmatterBody {
    patch: PatchFields,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PatchFields {
    status: Option<crate::patcher::TicketStatus>,
}

/// PATCH /api/docs/{path}/frontmatter
///
/// Exposed URL: /api/docs/{path}/frontmatter.
/// Registered route: /api/docs/*path (shared with GET doc_fetch) because matchit 0.7
/// forbids catch-all + literal suffix on the same route. This handler strips the
/// trailing /frontmatter suffix and returns 400 if it is absent.
pub(crate) async fn doc_patch_frontmatter(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
    headers: HeaderMap,
    body: Result<Json<PatchFrontmatterBody>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, ApiError> {
    use super::api_from_fd;

    // Step 1 — strip /frontmatter suffix (matchit workaround, see route registration).
    let doc_rel = path
        .strip_suffix("/frontmatter")
        .ok_or(ApiError::PatchEndpointMismatch)?;

    // Step 2 — per-segment path validation.
    // Precise rejection: only segments that *equal* ".." or "." are rejected, so
    // legitimate filenames like "0001..todo.md" are accepted. The real security
    // boundary is the canonicalize + writable_roots check inside the driver.
    if doc_rel.starts_with('/') {
        return Err(ApiError::PathEscape);
    }
    for seg in doc_rel.split('/') {
        if seg == ".." || seg == "." || seg.is_empty() || seg.contains('\\') || seg.contains('\0') {
            return Err(ApiError::PathEscape);
        }
    }

    // Step 3 — build absolute path.
    let abs = state.cfg.project_root.join(doc_rel);

    // Step 4 — index lookup (404 if unknown).
    let entry = state
        .indexer
        .get(&abs)
        .await
        .ok_or_else(|| ApiError::NotFound(doc_rel.to_string()))?;

    // Step 5 — ticket-type guard.
    if entry.r#type != crate::docs::DocTypeKey::Tickets {
        return Err(ApiError::OnlyTicketsAreWritable);
    }

    // Step 6 — parse JSON body. Result<Json<T>, JsonRejection> intercepts axum's extractor
    // (syntax/data/unknown-field errors) and remaps them all to 400 via InvalidPatch.
    let Json(body) = body.map_err(|e| ApiError::InvalidPatch(e.to_string()))?;
    let status = body
        .patch
        .status
        .ok_or_else(|| ApiError::InvalidPatch("patch object is empty".into()))?;

    // Step 7 — read and validate If-Match header.
    let if_match = {
        let v = headers
            .get(header::IF_MATCH)
            .ok_or(ApiError::IfMatchRequired)?;
        let s = v
            .to_str()
            .map_err(|_| ApiError::UnsupportedIfMatch("non-UTF-8 If-Match header".into()))?;
        if s == "*" {
            return Err(ApiError::UnsupportedIfMatch(
                "wildcard If-Match is not supported; supply a specific etag".into(),
            ));
        }
        if s.starts_with("W/") {
            return Err(ApiError::UnsupportedIfMatch(
                "weak etags are not supported; supply a strong etag without the W/ prefix".into(),
            ));
        }
        if s.contains(',') {
            return Err(ApiError::UnsupportedIfMatch(
                "etag lists are not supported; supply exactly one etag".into(),
            ));
        }
        s.to_string()
    };

    // Step 8 — write_frontmatter.
    // on_committed fires inside the per-path mutex, after persist + fsync, before the
    // lock is released. Registering here (while locked) closes the race where an
    // inotify/FSEvents notification could arrive at the watcher's debounce handler
    // before mark_self_write has run.
    let committed_canonical: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));
    let canonical_capture = committed_canonical.clone();
    let coordinator = state.write_coordinator.clone();
    let on_committed: Box<dyn FnOnce(&Path) + Send> = Box::new(move |canonical: &Path| {
        *canonical_capture.lock().unwrap() = Some(canonical.to_path_buf());
        coordinator.mark_self_write(canonical);
    });

    let content = state
        .file_driver
        .write_frontmatter(
            &abs,
            crate::patcher::FrontmatterPatch::Status(status),
            &if_match,
            on_committed,
        )
        .await
        .map_err(api_from_fd)?;

    // Steps 9-10 — refresh index then broadcast (only for real writes, not idempotent).
    // on_committed was not called for idempotent patches (driver short-circuited), so
    // committed_canonical is None. Extract before awaiting: std::sync::MutexGuard is
    // !Send and must not be held across an await point.
    let committed = committed_canonical.lock().unwrap().take();
    if let Some(canonical) = committed {
        // Refresh index first so subscribers that refetch on the broadcast see fresh data.
        let _ = state.indexer.refresh_one(&canonical).await;

        // Use the etag from FileContent, not from the index after refresh. A concurrent
        // external edit between persist and refresh would change the on-disk etag, which
        // would break the self-cause filter for this tab.
        state
            .sse_hub
            .broadcast(crate::sse_hub::SsePayload::DocChanged {
                doc_type: entry.r#type,
                path: doc_rel.to_string(),
                etag: Some(content.etag.clone()),
            });
    }

    // Step 11 — 204 No Content with new ETag.
    let quoted = format!("\"{}\"", content.etag);
    Ok((
        StatusCode::NO_CONTENT,
        [(
            header::ETAG,
            HeaderValue::from_str(&quoted).unwrap_or_else(|_| HeaderValue::from_static("")),
        )],
    )
        .into_response())
}
