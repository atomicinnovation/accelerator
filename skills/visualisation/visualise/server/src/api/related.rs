use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{Path as AxumPath, State},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use super::ApiError;
use crate::indexer::IndexEntry;
use crate::server::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RelatedArtifactsResponse {
    pub inferred_cluster: Vec<IndexEntry>,
    pub declared_outbound: Vec<IndexEntry>,
    pub declared_inbound: Vec<IndexEntry>,
}

/// GET /api/related/*path
///
/// Returns same-slug cluster siblings (`inferredCluster`, self excluded)
/// plus declared cross-references in both directions
/// (`declaredOutbound`, `declaredInbound`). Entries that appear in
/// both inferred and declared groups are dropped from inferred — the
/// declared relation is the more specific signal.
pub(crate) async fn related_get(
    State(state): State<Arc<AppState>>,
    AxumPath(path): AxumPath<String>,
) -> Result<Response, ApiError> {
    // Per-segment path validation runs on the *decoded* capture so
    // percent-encoded path-escape sequences (`%2F`, `%00`, …) cannot
    // smuggle in segments after the matchit catch-all.
    let decoded = decode_path(&path).ok_or(ApiError::PathEscape)?;
    if decoded.starts_with('/') {
        return Err(ApiError::PathEscape);
    }
    for seg in decoded.split('/') {
        if seg == ".." || seg == "." || seg.is_empty() || seg.contains('\\') || seg.contains('\0') {
            return Err(ApiError::PathEscape);
        }
    }

    let abs = state.cfg.project_root.join(&decoded);
    let entry = state
        .indexer
        .get(&abs)
        .await
        .ok_or_else(|| ApiError::NotFound(decoded.clone()))?;

    // Inferred cluster: same-slug siblings, self excluded.
    let inferred_cluster: Vec<IndexEntry> = if let Some(slug) = &entry.slug {
        let clusters = state.clusters.read().await;
        clusters
            .iter()
            .find(|c| &c.slug == slug)
            .map(|c| {
                c.entries
                    .iter()
                    .filter(|e| e.path != entry.path)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    // Declared outbound: resolved targets of self.frontmatter.target
    // (Phase 9: plan-reviews only).
    let declared_outbound = state.indexer.declared_outbound(&entry).await;

    // Declared inbound: plan-reviews targeting self + entries that cross-ref
    // self as a work-item (via work-item:, parent:, related:).
    let mut declared_inbound = state.indexer.reviews_by_target(&entry.path).await;
    if let Some(ref id) = entry.work_item_id {
        let ref_entries = state.indexer.work_item_refs_by_id(id).await;
        let existing_paths: HashSet<PathBuf> =
            declared_inbound.iter().map(|e| e.path.clone()).collect();
        for ref_entry in ref_entries {
            if !existing_paths.contains(&ref_entry.path) {
                declared_inbound.push(ref_entry);
            }
        }
    }

    // Dedup overlap: an entry that appears in both inferred and any
    // declared list is dropped from inferred. The declared relation
    // is the more specific signal and the UI groups them separately.
    let declared_paths: HashSet<PathBuf> = declared_outbound
        .iter()
        .chain(declared_inbound.iter())
        .map(|e| e.path.clone())
        .collect();
    let inferred_cluster: Vec<IndexEntry> = inferred_cluster
        .into_iter()
        .filter(|e| !declared_paths.contains(&e.path))
        .collect();

    Ok(Json(RelatedArtifactsResponse {
        inferred_cluster,
        declared_outbound,
        declared_inbound,
    })
    .into_response())
}

/// Single-pass percent-decoding for a captured `*path`. Returns `None`
/// if the input contains a malformed `%XX` triplet (truncated, non-hex)
/// — those map to 403 PathEscape rather than a silent passthrough.
/// Reserved characters that are not percent-encoded pass through
/// unchanged; the per-segment validator handles the rest.
fn decode_path(raw: &str) -> Option<String> {
    let bytes = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let hi = (bytes[i + 1] as char).to_digit(16)?;
            let lo = (bytes[i + 2] as char).to_digit(16)?;
            out.push(((hi << 4) | lo) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_path_passes_plain_through() {
        assert_eq!(
            decode_path("meta/plans/foo.md").as_deref(),
            Some("meta/plans/foo.md")
        );
    }

    #[test]
    fn decode_path_decodes_percent_slash_and_nul() {
        assert_eq!(decode_path("foo%2F..%2Fbar").as_deref(), Some("foo/../bar"),);
        assert_eq!(decode_path("foo%00bar").as_deref(), Some("foo\0bar"),);
    }

    #[test]
    fn decode_path_rejects_truncated_or_non_hex() {
        assert!(decode_path("foo%").is_none());
        assert!(decode_path("foo%2").is_none());
        assert!(decode_path("foo%XY").is_none());
    }
}
