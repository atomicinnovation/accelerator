use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use super::ApiError;
use crate::docs::DocTypeKey;
use crate::indexer::IndexEntry;
use crate::server::AppState;

/// Server-side hard cap on `q` length. Over-cap input short-circuits to empty
/// results, defending against amplification attacks that bypass the client
/// debounce. Applied to the raw input *before* trim so leading/trailing
/// whitespace cannot be used to pad past the cap.
const MAX_Q_LEN: usize = 128;

#[derive(Debug, Deserialize)]
pub(crate) struct SearchQuery {
    #[serde(default)]
    q: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchResultRow {
    doc_type: DocTypeKey,
    title: String,
    slug: String,
    mtime_ms: i64,
}

#[derive(Serialize)]
pub(crate) struct SearchResponse {
    results: Vec<SearchResultRow>,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bucket {
    ExactSlug = 0,
    Prefix = 1,
    Interior = 2,
    Body = 3,
}

/// Classify a single entry into a ranking bucket against the lowercased query.
/// Returns `None` when no field matches.
///
/// Performance: title and slug are short and lowercased eagerly; `body_preview`
/// is lowercased only after the title/slug paths fail, avoiding the largest
/// allocation in the common case where matches come from title/slug.
fn classify(entry: &IndexEntry, q_lc: &str) -> Option<Bucket> {
    let title_lc = entry.title.to_ascii_lowercase();
    let slug_lc = entry.slug.as_deref().map(str::to_ascii_lowercase);

    if let Some(s) = slug_lc.as_deref() {
        if s == q_lc {
            return Some(Bucket::ExactSlug);
        }
    }
    let title_prefix = title_lc.starts_with(q_lc);
    let slug_prefix = slug_lc.as_deref().is_some_and(|s| s.starts_with(q_lc));
    if title_prefix || slug_prefix {
        return Some(Bucket::Prefix);
    }
    let title_interior = title_lc.contains(q_lc);
    let slug_interior = slug_lc.as_deref().is_some_and(|s| s.contains(q_lc));
    if title_interior || slug_interior {
        return Some(Bucket::Interior);
    }
    let body_lc = entry.body_preview.to_ascii_lowercase();
    if body_lc.contains(q_lc) {
        return Some(Bucket::Body);
    }
    None
}

/// Project an `IndexEntry` to a wire-shape `SearchResultRow`.
/// Returns `None` for slug-less entries — every bucketing path funnels through
/// this projector so the slug invariant is enforced at the type level.
fn project(entry: &IndexEntry) -> Option<SearchResultRow> {
    let slug = entry.slug.clone()?;
    Some(SearchResultRow {
        doc_type: entry.r#type,
        title: entry.title.clone(),
        slug,
        mtime_ms: entry.mtime_ms,
    })
}

pub(crate) async fn search(
    State(state): State<Arc<AppState>>,
    Query(q): Query<SearchQuery>,
) -> Result<Response, ApiError> {
    // Length cap applies to the raw input — leading/trailing whitespace cannot
    // be used to pad past the cap.
    if q.q.len() > MAX_Q_LEN {
        return Ok(Json(SearchResponse { results: vec![] }).into_response());
    }
    let trimmed = q.q.trim();
    if trimmed.is_empty() {
        return Ok(Json(SearchResponse { results: vec![] }).into_response());
    }
    let q_lc = trimmed.to_ascii_lowercase();

    let snapshot = state.indexer.all().await;

    let mut buckets: [Vec<IndexEntry>; 4] = [vec![], vec![], vec![], vec![]];
    for entry in snapshot {
        if entry.r#type == DocTypeKey::Templates {
            continue;
        }
        if let Some(bucket) = classify(&entry, &q_lc) {
            buckets[bucket as usize].push(entry);
        }
    }

    for bucket in &mut buckets {
        bucket.sort_by_cached_key(|e| {
            (
                std::cmp::Reverse(e.mtime_ms),
                e.rel_path.to_string_lossy().into_owned(),
            )
        });
    }

    let results: Vec<SearchResultRow> = buckets
        .into_iter()
        .flatten()
        .filter_map(|e| project(&e))
        .collect();

    Ok(Json(SearchResponse { results }).into_response())
}
