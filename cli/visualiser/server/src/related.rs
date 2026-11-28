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
    // An entry's own slug can differ from its cluster's representative
    // slug, and several clusters can share a slug, so a keyed entry finds
    // its cluster by key and any other by the cluster that holds it.
    let inferred_cluster: Vec<IndexEntry> = {
        let cluster = match entry.cluster_key.as_deref() {
            Some(ck) => clusters
                .iter()
                .find(|c| c.cluster_key.as_deref() == Some(ck)),
            None => entry.slug.as_ref().and_then(|slug| {
                clusters
                    .iter()
                    .find(|c| &c.slug == slug && c.holds(&entry.path))
            }),
        };
        cluster
            .map(|c| {
                c.entries
                    .iter()
                    .filter(|e| e.path != entry.path)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    };

    let declared_outbound = indexer.declared_outbound(entry).await;

    // Declared inbound: reviews-by-target ∪ work-item-refs-by-id,
    // deduped by path. Preserves reviews_by_target ordering and
    // appends only ref entries whose path is not already present.
    let mut declared_inbound = indexer.reviews_by_target(&entry.path).await;
    if let Some(ref id) = entry.identity_key {
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

/// Total relation count for an entry — equals
/// `inferredCluster.len() + declaredOutbound.len() + declaredInbound.len()`
/// from the `/api/related/{path}` response, by construction.
pub fn count_from_resolution(r: &RelatedResolution) -> usize {
    r.inferred_cluster.len()
        + r.declared_outbound.len()
        + r.declared_inbound.len()
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
    let mut counts: HashMap<PathBuf, usize> =
        HashMap::with_capacity(entries_snapshot.len());
    for entry in entries_snapshot {
        let resolution = resolve_related(indexer, clusters, entry).await;
        counts.insert(entry.path.clone(), count_from_resolution(&resolution));
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clusters::recompute_clusters;
    use crate::test_support::{dossier, entry_for_test, set_member_for_test};
    use corpus::DocTypeKey;
    use tokio::sync::RwLock;

    struct Related {
        indexer: Indexer,
        clusters: Vec<LifecycleCluster>,
    }

    impl Related {
        async fn of(records: Vec<IndexEntry>) -> Self {
            let indexer =
                Indexer::from_records(PathBuf::from("/x"), records).await;
            let clusters = RwLock::new(Vec::new());
            recompute_clusters(&indexer, &clusters).await;
            Self {
                indexer,
                clusters: clusters.into_inner(),
            }
        }

        async fn to(&self, entry: &IndexEntry) -> RelatedResolution {
            resolve_related(&self.indexer, &self.clusters, entry).await
        }
    }

    fn paths(entries: &[IndexEntry]) -> Vec<PathBuf> {
        entries.iter().map(|e| e.path.clone()).collect()
    }

    fn work_item_0042() -> IndexEntry {
        let mut item = entry_for_test(DocTypeKey::WORK_ITEMS, "x", 0, "X");
        item.path = PathBuf::from("/x/work/0042-x.md");
        item.identity_key = Some("0042".into());
        item
    }

    fn set_with_a_member_parented_by_0042(
    ) -> (IndexEntry, IndexEntry, IndexEntry) {
        let root = dossier("dossier-a");
        let mut member = set_member_for_test(&root, "notes/a.md");
        member.frontmatter = serde_json::json!({ "parent": "0042" });
        member.work_item_refs = vec!["0042".into()];
        let sibling = set_member_for_test(&root, "notes/b.md");
        (root, member, sibling)
    }

    #[tokio::test]
    async fn a_members_declared_parent_is_attributed_to_that_member() {
        let item = work_item_0042();
        let (root, member, sibling) = set_with_a_member_parented_by_0042();
        let related = Related::of(vec![
            item.clone(),
            root.clone(),
            member.clone(),
            sibling.clone(),
        ])
        .await;

        assert_eq!(
            paths(&related.to(&member).await.declared_outbound),
            vec![item.path.clone()]
        );
        assert!(related.to(&root).await.declared_outbound.is_empty());
        assert!(related.to(&sibling).await.declared_outbound.is_empty());
    }

    #[tokio::test]
    async fn a_work_item_names_a_member_parented_by_it_as_inbound() {
        let item = work_item_0042();
        let (root, member, sibling) = set_with_a_member_parented_by_0042();
        let related =
            Related::of(vec![item.clone(), root, member.clone(), sibling])
                .await;

        let inbound = related.to(&item).await.declared_inbound;

        assert_eq!(paths(&inbound), vec![member.path]);
        let wire = serde_json::to_value(&inbound[0]).unwrap();
        assert_eq!(wire["setId"], "dossier-a");
        assert_eq!(wire["setRelativePath"], "notes/a.md");
    }

    #[tokio::test]
    async fn no_member_is_ever_an_inferred_item() {
        let mut root = dossier("shared");
        root.slug = Some("shared".into());
        let member = set_member_for_test(&root, "shared.md");
        let mut plan = entry_for_test(DocTypeKey::PLANS, "shared", 0, "P");
        plan.path = PathBuf::from("/x/plans/2026-01-01-shared.md");
        let related =
            Related::of(vec![root.clone(), member.clone(), plan.clone()]).await;

        for entry in [&root, &plan] {
            let inferred = related.to(entry).await.inferred_cluster;
            assert!(!paths(&inferred).contains(&member.path), "{inferred:?}");
        }
    }
}
