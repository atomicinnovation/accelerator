use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path as AxumPath, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::docs::DocTypeKey;
use crate::file_driver::FileDriver;
use crate::indexer::IndexEntry;
use crate::server::AppState;
use super::{ApiError, api_from_fd, parse_kind};

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
            let content = state
                .file_driver
                .read(&e.path)
                .await
                .map_err(api_from_fd)?;
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
            (header::ETAG, HeaderValue::from_str(&format!("\"{etag}\"")).unwrap()),
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/markdown; charset=utf-8"),
            ),
        ],
        Body::from(bytes),
    )
        .into_response())
}
