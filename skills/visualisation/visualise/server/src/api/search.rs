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

/// Hard cap on returned rows. Bounds the response for broad-match queries (a
/// short fragment can match many entries); applied after bucket ordering so
/// only the lowest-relevance tail is dropped, never an exact/prefix hit. The
/// value is a generous backstop, not a wire contract beyond "bounded".
const MAX_RESULTS: usize = 50;

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
    /// Exact-identity tier: an exact slug **or** an exact `work_item_id`
    /// (numeric-reduced for numeric queries — see `classify`).
    ExactSlug = 0,
    Prefix = 1,
    Interior = 2,
    Body = 3,
}

/// Bare numeric form of an id, used for padding-tolerant numeric matching:
/// leading non-digit prefix and leading zeros stripped.
/// `"0042"` -> `"42"`, `"eng-0042"` -> `"42"`, `"0000"`/`""` -> `""`.
/// Assumes the normalised-id shape from `WorkItemConfig::normalise_id` — bare
/// digits or `CODE-digits` with an alphabetic prefix — so the digits are the
/// trailing run; a future id shape with digits elsewhere would mis-key.
fn numeric_key(id_lc: &str) -> &str {
    id_lc
        .rsplit(|c: char| !c.is_ascii_digit())
        .next()
        .unwrap_or("")
        .trim_start_matches('0')
}

/// Classify a single entry into a ranking bucket against the lowercased query.
/// Returns `None` when no field matches.
///
/// Matched fields: `title`, `slug`, `work_item_id`, then (lazily)
/// `body_preview`. `title`, `slug` and `work_item_id` are short and lowercased
/// eagerly; `body_preview` is lowercased only on the fall-through path, avoiding
/// the largest allocation when matches come from the cheaper fields.
///
/// `work_item_id` matching: a purely-numeric query is compared against the id's
/// bare numeric form (see `numeric_key`), so `42`/`0042` exactly match
/// `0042`/`ENG-0042` and `00` (which reduces to empty) matches nothing; any
/// other query substring-matches the full lowercased id (so `eng-0042` matches
/// `ENG-0042`).
///
/// If a third matchable id field is added (e.g. `external_id`), refactor to
/// iterate candidate id fields per tier with a per-field numeric-reduction
/// policy, rather than hand-copying this block a third time.
fn classify(entry: &IndexEntry, q_lc: &str) -> Option<Bucket> {
    let title_lc = entry.title.to_ascii_lowercase();
    let slug_lc = entry.slug.as_deref().map(str::to_ascii_lowercase);
    let id_lc = entry.work_item_id.as_deref().map(str::to_ascii_lowercase);

    // For a numeric query, compare query and id in bare numeric form; otherwise
    // compare against the full lowercased id. A numeric query of all zeros
    // reduces to empty and must match no id (`id_active` guards that). `q_lc` is
    // non-empty here (the handler rejects empty/whitespace queries).
    let q_is_num = q_lc.bytes().all(|b| b.is_ascii_digit());
    let id_q: &str = if q_is_num {
        q_lc.trim_start_matches('0')
    } else {
        q_lc
    };
    let id_cmp: Option<&str> =
        id_lc
            .as_deref()
            .map(|id| if q_is_num { numeric_key(id) } else { id });
    let id_active = !id_q.is_empty();
    let id_exact = id_active && id_cmp == Some(id_q);
    let id_prefix = id_active
        && id_cmp.is_some_and(|id| !id.is_empty() && id.starts_with(id_q));
    let id_interior = id_active
        && id_cmp.is_some_and(|id| !id.is_empty() && id.contains(id_q));

    if slug_lc.as_deref() == Some(q_lc) || id_exact {
        return Some(Bucket::ExactSlug);
    }
    let title_prefix = title_lc.starts_with(q_lc);
    let slug_prefix = slug_lc.as_deref().is_some_and(|s| s.starts_with(q_lc));
    if title_prefix || slug_prefix || id_prefix {
        return Some(Bucket::Prefix);
    }
    let title_interior = title_lc.contains(q_lc);
    let slug_interior = slug_lc.as_deref().is_some_and(|s| s.contains(q_lc));
    if title_interior || slug_interior || id_interior {
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
        .take(MAX_RESULTS)
        .collect();

    Ok(Json(SearchResponse { results }).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_key_reduces_to_bare_trailing_digit_run() {
        assert_eq!(numeric_key("0042"), "42");
        assert_eq!(numeric_key("eng-0042"), "42");
        // normalise_id passes foreign prefixes through unpadded.
        assert_eq!(numeric_key("ops-7"), "7");
        assert_eq!(numeric_key("0000"), "");
        assert_eq!(numeric_key(""), "");
        // No digits at all reduces to empty.
        assert_eq!(numeric_key("abc"), "");
    }
}
