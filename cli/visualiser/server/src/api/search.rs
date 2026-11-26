use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use super::ApiError;
use crate::indexer::{link_keys_of, IndexEntry, Indexer};
use crate::server::AppState;
use corpus::DocTypeKey;

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
    #[serde(with = "crate::doc_type_serde")]
    doc_type: DocTypeKey,
    id: Option<String>,
    title: String,
    slug: String,
    rel_path: String,
    mtime_ms: i64,
    set_id: Option<String>,
    set_relative_path: Option<String>,
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
    let slug_lc = entry.search_key().map(str::to_ascii_lowercase);

    if !entry.is_member() && slug_lc.as_deref() == Some(q_lc) {
        return Some(Bucket::ExactSlug);
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
    let link_keys = link_keys_of(entry);
    let placement = entry.set_placement();
    Some(SearchResultRow {
        doc_type: entry.r#type,
        id: link_keys.id,
        title: entry.title.clone(),
        slug: link_keys.slug?,
        rel_path: entry.served_rel_path(),
        mtime_ms: entry.mtime_ms,
        set_id: placement.map(|p| p.context.set_id.clone()),
        set_relative_path: placement.map(|p| p.set_relative_path.clone()),
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

    let results = search_index(&state.indexer, &q_lc).await;
    Ok(Json(SearchResponse { results }).into_response())
}

async fn search_index(indexer: &Indexer, q_lc: &str) -> Vec<SearchResultRow> {
    ranked(indexer.documents_including_members().await, q_lc)
}

fn ranked(candidates: Vec<IndexEntry>, q_lc: &str) -> Vec<SearchResultRow> {
    let mut buckets: [Vec<IndexEntry>; 4] = [vec![], vec![], vec![], vec![]];
    for entry in candidates {
        if entry.r#type.is_virtual() {
            continue;
        }
        if let Some(bucket) = classify(&entry, q_lc) {
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

    buckets
        .into_iter()
        .flatten()
        .filter_map(|e| project(&e))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{dossier, entry_for_test, set_member_for_test};

    fn row_json(rows: &[SearchResultRow]) -> Vec<serde_json::Value> {
        rows.iter()
            .map(|row| serde_json::to_value(row).unwrap())
            .collect()
    }

    async fn searched(
        records: Vec<IndexEntry>,
        q_lc: &str,
    ) -> Vec<serde_json::Value> {
        let indexer = Indexer::from_records("/x".into(), records).await;
        row_json(&search_index(&indexer, q_lc).await)
    }

    #[tokio::test]
    async fn a_phrase_in_a_members_first_paragraph_finds_the_member() {
        let root = dossier("dossier-a");
        let mut member = set_member_for_test(&root, "findings/01-cost.md");
        member.slug = None;
        member.title = "Cost model".into();
        member.body_preview = "The cache halves the bill.".into();

        let rows = searched(vec![root, member], "halves the bill").await;

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["title"], "Cost model");
        assert_eq!(rows[0]["slug"], "dossier-a");
        assert_eq!(rows[0]["setId"], "dossier-a");
        assert_eq!(rows[0]["setRelativePath"], "findings/01-cost.md");
    }

    #[test]
    fn searching_a_sets_slug_ranks_its_root_above_a_member_named_alike() {
        let mut root = dossier("dossier-a");
        root.mtime_ms = 1;
        let mut member = set_member_for_test(&root, "notes/dossier-a.md");
        member.slug = None;
        member.title = "dossier-a".into();
        member.mtime_ms = 2;

        let rows = row_json(&ranked(vec![member, root], "dossier-a"));

        let relative_paths: Vec<&serde_json::Value> =
            rows.iter().map(|row| &row["setRelativePath"]).collect();
        assert_eq!(relative_paths, ["cover.md", "notes/dossier-a.md"]);
    }

    #[test]
    fn a_member_is_found_by_its_filename_stem() {
        let root = dossier("dossier-a");
        let mut member =
            set_member_for_test(&root, "findings/01-cost-model.md");
        member.slug = None;
        member.title = "Economics".into();

        let rows = row_json(&ranked(vec![root, member], "cost-model"));

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["title"], "Economics");
    }

    #[test]
    fn a_document_outside_a_set_has_null_set_fields_in_its_row() {
        let plan = entry_for_test(DocTypeKey::PLANS, "foo", 1, "Foo");

        let rows = row_json(&ranked(vec![plan], "foo"));

        assert_eq!(rows[0]["setId"], serde_json::Value::Null);
        assert_eq!(rows[0]["setRelativePath"], serde_json::Value::Null);
    }
}
