use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::clusters::LifecycleCluster;
use crate::indexer::{IndexEntry, Indexer};

/// Triple of related-artifact lists produced by `resolve_related`.
/// Mirrors the wire shape of `/api/related/{path}` exactly so the
/// handler in `api/related.rs` is a thin serialisation wrapper.
pub struct RelatedResolution {
    pub inferred_cluster: Vec<IndexEntry>,
    pub declared_outbound: Vec<IndexEntry>,
    pub declared_inbound: Vec<IndexEntry>,
}

/// Pure resolution of an entry's related artifacts. Source of truth for
/// both `/api/related/{path}` and the indexer's per-entry `linked_count`
/// back-fill, so AC-6's equality is a tautology rather than a parallel
/// invariant. Acquires its own brief read locks on the indexer's
/// secondary indexes; callers must not hold `entries.write()` across
/// this call (the secondary-index helpers re-enter `entries.read()`).
pub async fn resolve_related(
    indexer: &Indexer,
    clusters: &[LifecycleCluster],
    entry: &IndexEntry,
) -> RelatedResolution {
    // Inferred cluster: same-slug siblings, self excluded.
    let inferred_cluster: Vec<IndexEntry> = if let Some(slug) = &entry.slug {
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

    let declared_outbound = indexer.declared_outbound(entry).await;

    // Declared inbound: reviews-by-target ∪ work-item-refs-by-id,
    // deduped by path. Preserves reviews_by_target ordering and
    // appends only ref entries whose path is not already present.
    let mut declared_inbound = indexer.reviews_by_target(&entry.path).await;
    if let Some(ref id) = entry.work_item_id {
        let ref_entries = indexer.work_item_refs_by_id(id).await;
        let existing_paths: HashSet<PathBuf> =
            declared_inbound.iter().map(|e| e.path.clone()).collect();
        for ref_entry in ref_entries {
            if !existing_paths.contains(&ref_entry.path) {
                declared_inbound.push(ref_entry);
            }
        }
    }

    // Inferred-vs-declared dedup: an entry that appears in both the
    // inferred and any declared list is dropped from inferred — the
    // declared relation is the more specific signal.
    let declared_paths: HashSet<PathBuf> = declared_outbound
        .iter()
        .chain(declared_inbound.iter())
        .map(|e| e.path.clone())
        .collect();
    let inferred_cluster: Vec<IndexEntry> = inferred_cluster
        .into_iter()
        .filter(|e| !declared_paths.contains(&e.path))
        .collect();

    RelatedResolution {
        inferred_cluster,
        declared_outbound,
        declared_inbound,
    }
}

/// Total relation count for an entry — equals `inferredCluster.len()
/// + declaredOutbound.len() + declaredInbound.len()` from the
/// `/api/related/{path}` response, by construction.
pub fn count_from_resolution(r: &RelatedResolution) -> usize {
    r.inferred_cluster.len() + r.declared_outbound.len() + r.declared_inbound.len()
}

/// Pass 1 of the linked-count back-fill pipeline: iterate the entries
/// snapshot, call `resolve_related` for each, and collect the resulting
/// counts into a path → count map. The map is then handed to
/// `Indexer::apply_linked_count_backfill` under `entries.write()`.
///
/// The iteration acquires no entries lock itself — each `resolve_related`
/// call takes its own brief read locks internally — so callers must not
/// hold `entries.write()` across this function.
pub async fn collect_linked_counts(
    indexer: &Indexer,
    clusters: &[LifecycleCluster],
    entries_snapshot: &[IndexEntry],
) -> HashMap<PathBuf, usize> {
    let mut counts: HashMap<PathBuf, usize> = HashMap::with_capacity(entries_snapshot.len());
    for entry in entries_snapshot {
        let resolution = resolve_related(indexer, clusters, entry).await;
        counts.insert(entry.path.clone(), count_from_resolution(&resolution));
    }
    counts
}
