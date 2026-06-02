use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::config::WorkItemConfig;
use crate::docs::DocTypeKey;
use crate::indexer::IndexEntry;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Completeness {
    pub has_work_item: bool,
    pub has_research: bool,
    pub has_plan: bool,
    pub has_plan_review: bool,
    pub has_validation: bool,
    pub has_pr_description: bool,
    pub has_pr_review: bool,
    pub has_decision: bool,
    pub has_notes: bool,
    pub has_design_inventory: bool,
    pub has_design_gap: bool,
    pub present: Vec<String>,
}

// Single source of truth for stage push order. MUST match the frontend's
// LIFECYCLE_PIPELINE_STEPS (in `frontend/src/api/types.ts`) followed by
// LONG_TAIL_PIPELINE_STEPS. Cross-reference any reordering on both sides.
const STAGE_PUSH_ORDER: &[(fn(&Completeness) -> bool, &str)] = &[
    (|c| c.has_work_item, "work-items"),
    (|c| c.has_research, "research"),
    (|c| c.has_plan, "plans"),
    (|c| c.has_plan_review, "plan-reviews"),
    (|c| c.has_validation, "validations"),
    (|c| c.has_pr_description, "pr-descriptions"),
    (|c| c.has_pr_review, "pr-reviews"),
    (|c| c.has_decision, "decisions"),
    (|c| c.has_notes, "notes"),
    (|c| c.has_design_inventory, "design-inventories"),
    (|c| c.has_design_gap, "design-gaps"),
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleCluster {
    pub slug: String,
    pub title: String,
    pub entries: Vec<IndexEntry>,
    pub completeness: Completeness,
    pub last_changed_ms: i64,
    /// Canonical cluster identity — the resolved work-item id when the
    /// cluster has one, else `None`. Serialises on the wire as
    /// `clusterKey` (camelCase) and is `null` for slug-fallback clusters.
    pub cluster_key: Option<String>,
}

/// Snapshots required by the cluster-key resolver. Built once at
/// clustering time by the indexer caller from a coherent view of
/// `entries`.
pub struct ClusterContext<'a> {
    pub entries_by_path: HashMap<PathBuf, &'a IndexEntry>,
    pub work_item_by_id: &'a HashMap<String, PathBuf>,
    pub plans_by_id: &'a HashMap<String, PathBuf>,
    pub project_root: &'a Path,
    pub work_item_cfg: &'a WorkItemConfig,
}

impl<'a> ClusterContext<'a> {
    /// Build `entries_by_path` from the borrowed entries slice so the
    /// snapshot is trivially coherent with the slice being clustered.
    /// Zero deep clones — entries are referenced, not owned.
    pub fn from_entries(
        entries: &'a [IndexEntry],
        work_item_by_id: &'a HashMap<String, PathBuf>,
        plans_by_id: &'a HashMap<String, PathBuf>,
        project_root: &'a Path,
        work_item_cfg: &'a WorkItemConfig,
    ) -> Self {
        let entries_by_path = entries.iter().map(|e| (e.path.clone(), e)).collect();
        Self {
            entries_by_path,
            work_item_by_id,
            plans_by_id,
            project_root,
            work_item_cfg,
        }
    }
}

/// Stack-allocated empty maps + config + root, used by tests that
/// only exercise the slug-fallback path. The caller owns the
/// storage and constructs a borrowing `ClusterContext` against it.
#[cfg(test)]
pub struct EmptyClusterFixture {
    pub wi: HashMap<String, PathBuf>,
    pub pl: HashMap<String, PathBuf>,
    pub root: PathBuf,
    pub cfg: WorkItemConfig,
}

#[cfg(test)]
impl EmptyClusterFixture {
    pub fn new() -> Self {
        Self {
            wi: HashMap::new(),
            pl: HashMap::new(),
            root: PathBuf::from("/repo"),
            cfg: WorkItemConfig::default(),
        }
    }
    pub fn ctx(&self) -> ClusterContext<'_> {
        ClusterContext {
            entries_by_path: HashMap::new(),
            work_item_by_id: &self.wi,
            plans_by_id: &self.pl,
            project_root: &self.root,
            work_item_cfg: &self.cfg,
        }
    }
}

pub fn compute_clusters(entries: &[IndexEntry], ctx: &ClusterContext<'_>) -> Vec<LifecycleCluster> {
    compute_clusters_with_backfill(entries, ctx).0
}

/// Like `compute_clusters`, but also returns:
///
/// - A `HashMap` keyed by every clustered entry's canonical path with the
///   cluster's `Completeness`. Callers apply the map to `Indexer::entries`
///   so per-entry `IndexEntry.completeness` mirrors the cluster's view.
/// - A `HashMap` keyed by every non-template entry's path with the
///   resolved cluster_key (or `None` for slug-fallback / orphan-bucket
///   entries). Callers apply this map onto `IndexEntry.cluster_key` so
///   the canonical entries map stays in lockstep with the cluster view.
pub fn compute_clusters_with_backfill(
    entries: &[IndexEntry],
    ctx: &ClusterContext<'_>,
) -> (
    Vec<LifecycleCluster>,
    HashMap<PathBuf, Completeness>,
    HashMap<PathBuf, Option<String>>,
) {
    // 1. Resolve cluster_key for every non-template entry.
    let mut cluster_key_by_path: HashMap<PathBuf, Option<String>> =
        HashMap::with_capacity(entries.len());
    for e in entries {
        if matches!(e.r#type, DocTypeKey::Templates) {
            continue;
        }
        let key = crate::cluster_key::resolve_cluster_key(
            e,
            &ctx.entries_by_path,
            ctx.work_item_by_id,
            ctx.plans_by_id,
            ctx.project_root,
            ctx.work_item_cfg,
        );
        cluster_key_by_path.insert(e.path.clone(), key);
    }

    // 2. Build a slug → cluster_key bridge so lifecycle-participating
    //    entries that lack typed-linkage frontmatter still merge with
    //    their cluster_key-carrying siblings via slug equivalence.
    //    Without this, a research-with-no-parent whose filename slug
    //    matches the work-item's slug lands in its own slug-only bucket
    //    and produces a second cluster sharing the same representative
    //    slug — breaking the /lifecycle/<slug> URL invariant. WorkItems
    //    win the slug → cluster_key mapping (they are the canonical
    //    source of the slug); other typed entries are inserted only
    //    when WorkItems hasn't already claimed the slug.
    let mut slug_to_cluster_key: HashMap<String, String> = HashMap::new();
    for e in entries {
        if e.r#type != DocTypeKey::WorkItems {
            continue;
        }
        if let (Some(slug), Some(ck)) = (
            e.slug.as_deref(),
            cluster_key_by_path
                .get(&e.path)
                .and_then(|o| o.as_deref()),
        ) {
            slug_to_cluster_key.insert(slug.to_string(), ck.to_string());
        }
    }
    for e in entries {
        if e.r#type == DocTypeKey::WorkItems {
            continue;
        }
        if let (Some(slug), Some(ck)) = (
            e.slug.as_deref(),
            cluster_key_by_path
                .get(&e.path)
                .and_then(|o| o.as_deref()),
        ) {
            slug_to_cluster_key
                .entry(slug.to_string())
                .or_insert_with(|| ck.to_string());
        }
    }

    // 3. Bucketing. Bucket key is cluster_key when present; otherwise
    //    slug — but only for types that participate in the lifecycle
    //    pipeline. Lifecycle-participating slug-fallback entries first
    //    check `slug_to_cluster_key` so they merge with any typed
    //    cluster sharing their slug. Orphan-by-design types (Notes,
    //    Decisions, DesignGaps, DesignInventories) get their own
    //    per-path bucket so they cannot accidentally collision-merge
    //    with unrelated entries that share a slug derivation.
    //    Templates are filtered out earlier.
    let mut buckets: HashMap<String, Vec<IndexEntry>> = HashMap::new();
    for e in entries {
        if matches!(e.r#type, DocTypeKey::Templates) {
            continue;
        }
        let key = cluster_key_by_path
            .get(&e.path)
            .and_then(|o| o.as_deref());
        let bucket_key = match key {
            Some(k) => Some(k.to_string()),
            None if e.r#type.participates_in_lifecycle() => {
                match e
                    .slug
                    .as_deref()
                    .and_then(|s| slug_to_cluster_key.get(s))
                {
                    Some(ck) => {
                        // Adopt the cluster's key for this entry so the
                        // backfill / wire shape reflects the merge.
                        cluster_key_by_path.insert(e.path.clone(), Some(ck.clone()));
                        Some(ck.clone())
                    }
                    None => e.slug.clone(),
                }
            }
            None => {
                // Orphan-by-design (Notes, Decisions, DesignGaps,
                // DesignInventories): consult the slug → cluster_key
                // bridge so a note/decision/etc. whose slug matches a
                // typed cluster joins that cluster. Falls back to a
                // per-path bucket only when no typed cluster shares
                // the slug — which preserves the "two orphan-type
                // entries with colliding slugs must not merge"
                // invariant pinned by
                // `orphan_types_with_colliding_slugs_do_not_merge`.
                match e
                    .slug
                    .as_deref()
                    .and_then(|s| slug_to_cluster_key.get(s))
                {
                    Some(ck) => {
                        cluster_key_by_path.insert(e.path.clone(), Some(ck.clone()));
                        Some(ck.clone())
                    }
                    None => Some(format!("__orphan__::{}", e.path.display())),
                }
            }
        };
        let Some(k) = bucket_key else { continue };
        buckets.entry(k).or_default().push(e.clone());
    }

    // 3. Build clusters.
    let mut backfill: HashMap<PathBuf, Completeness> = HashMap::new();
    let mut clusters: Vec<LifecycleCluster> = buckets
        .into_iter()
        .map(|(_bucket_key, mut bucket_entries)| {
            bucket_entries.sort_by(|a, b| {
                canonical_rank(a.r#type)
                    .cmp(&canonical_rank(b.r#type))
                    .then(a.mtime_ms.cmp(&b.mtime_ms))
            });
            let last_changed_ms = bucket_entries.iter().map(|e| e.mtime_ms).max().unwrap_or(0);
            // Representative cluster_key: read from the backfill map for the
            // first WorkItems entry, else the first entry by path order.
            let cluster_key = pick_representative_cluster_key(&bucket_entries, &cluster_key_by_path);
            let representative_slug = pick_representative_slug(&bucket_entries, cluster_key.as_deref());
            let title = derive_title(&representative_slug, &bucket_entries);
            let completeness = derive_completeness(&bucket_entries);
            for e in bucket_entries.iter_mut() {
                e.completeness = Some(completeness.clone());
                e.cluster_key = cluster_key_by_path
                    .get(&e.path)
                    .cloned()
                    .unwrap_or(None);
                backfill.insert(e.path.clone(), completeness.clone());
            }
            LifecycleCluster {
                slug: representative_slug,
                title,
                entries: bucket_entries,
                completeness,
                last_changed_ms,
                cluster_key,
            }
        })
        .collect();

    clusters.sort_by(|a, b| a.slug.cmp(&b.slug));
    (clusters, backfill, cluster_key_by_path)
}

fn pick_representative_cluster_key(
    bucket: &[IndexEntry],
    cluster_key_by_path: &HashMap<PathBuf, Option<String>>,
) -> Option<String> {
    // Prefer the WorkItems entry's cluster_key when present.
    if let Some(wi) = bucket.iter().find(|e| e.r#type == DocTypeKey::WorkItems) {
        if let Some(key) = cluster_key_by_path.get(&wi.path).cloned().flatten() {
            return Some(key);
        }
    }
    // Otherwise, any entry's cluster_key (deterministic order by path).
    let mut sorted: Vec<&IndexEntry> = bucket.iter().collect();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));
    for e in &sorted {
        if let Some(key) = cluster_key_by_path.get(&e.path).cloned().flatten() {
            return Some(key);
        }
    }
    None
}

fn pick_representative_slug(bucket: &[IndexEntry], cluster_key: Option<&str>) -> String {
    // 1. WorkItems entry's slug, if Some.
    if let Some(wi_slug) = bucket
        .iter()
        .find(|e| e.r#type == DocTypeKey::WorkItems)
        .and_then(|e| e.slug.clone())
    {
        return wi_slug;
    }
    // 2. Any entry's slug (deterministic order by path).
    let mut sorted: Vec<&IndexEntry> = bucket.iter().collect();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));
    if let Some(s) = sorted.iter().find_map(|e| e.slug.clone()) {
        return s;
    }
    // 3. Last resort: the cluster_key string itself. Guarantees the
    //    cluster URL is always derivable.
    cluster_key.unwrap_or("").to_string()
}

fn canonical_rank(kind: DocTypeKey) -> u8 {
    match kind {
        DocTypeKey::WorkItems => 0,
        DocTypeKey::Research => 1,
        DocTypeKey::Plans => 2,
        DocTypeKey::PlanReviews => 3,
        DocTypeKey::Validations => 4,
        DocTypeKey::PrDescriptions => 5,
        DocTypeKey::PrReviews => 6,
        DocTypeKey::WorkItemReviews => 6,
        DocTypeKey::Decisions => 7,
        DocTypeKey::Notes => 8,
        DocTypeKey::DesignInventories => 9,
        DocTypeKey::DesignGaps => 10,
        DocTypeKey::Templates => u8::MAX,
    }
}

fn derive_title(slug: &str, entries: &[IndexEntry]) -> String {
    for e in entries {
        if e.frontmatter_state == "parsed" && !e.title.is_empty() {
            return e.title.clone();
        }
    }
    if let Some(e) = entries.first() {
        if !e.title.is_empty() {
            return e.title.clone();
        }
    }
    slug.to_string()
}

fn derive_completeness(entries: &[IndexEntry]) -> Completeness {
    let mut c = Completeness {
        has_work_item: false,
        has_research: false,
        has_plan: false,
        has_plan_review: false,
        has_validation: false,
        has_pr_description: false,
        has_pr_review: false,
        has_decision: false,
        has_notes: false,
        has_design_inventory: false,
        has_design_gap: false,
        present: Vec::new(),
    };
    for e in entries {
        match e.r#type {
            DocTypeKey::WorkItems => c.has_work_item = true,
            DocTypeKey::Research => c.has_research = true,
            DocTypeKey::Plans => c.has_plan = true,
            DocTypeKey::PlanReviews => c.has_plan_review = true,
            DocTypeKey::WorkItemReviews => {}
            DocTypeKey::Validations => c.has_validation = true,
            DocTypeKey::PrDescriptions => c.has_pr_description = true,
            DocTypeKey::PrReviews => c.has_pr_review = true,
            DocTypeKey::Decisions => c.has_decision = true,
            DocTypeKey::Notes => c.has_notes = true,
            DocTypeKey::DesignGaps => c.has_design_gap = true,
            DocTypeKey::DesignInventories => c.has_design_inventory = true,
            DocTypeKey::Templates => {}
        }
    }
    for (test, key) in STAGE_PUSH_ORDER {
        if test(&c) {
            c.present.push((*key).into());
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{entry_for_test, entry_for_test_with_filename};
    use serde_json::json;

    fn entry(kind: DocTypeKey, slug: &str, mtime_ms: i64, title: &str) -> IndexEntry {
        entry_for_test(kind, slug, mtime_ms, title)
    }

    fn compute(entries: &[IndexEntry]) -> Vec<LifecycleCluster> {
        let fx = EmptyClusterFixture::new();
        compute_clusters(entries, &fx.ctx())
    }

    fn compute_with_backfill(
        entries: &[IndexEntry],
    ) -> (
        Vec<LifecycleCluster>,
        HashMap<PathBuf, Completeness>,
        HashMap<PathBuf, Option<String>>,
    ) {
        let fx = EmptyClusterFixture::new();
        compute_clusters_with_backfill(entries, &fx.ctx())
    }

    /// Run clustering with snapshot maps derived from the entries
    /// themselves — work_item_by_id from WorkItems entries, plans_by_id
    /// from Plans entries (by file stem).
    fn run_clusters(
        entries: &[IndexEntry],
        cfg: &WorkItemConfig,
    ) -> (
        Vec<LifecycleCluster>,
        HashMap<PathBuf, Completeness>,
        HashMap<PathBuf, Option<String>>,
    ) {
        let work_item_by_id: HashMap<String, PathBuf> = entries
            .iter()
            .filter(|e| e.r#type == DocTypeKey::WorkItems)
            .filter_map(|e| e.work_item_id.clone().map(|id| (id, e.path.clone())))
            .collect();
        let plans_by_id: HashMap<String, PathBuf> = entries
            .iter()
            .filter(|e| e.r#type == DocTypeKey::Plans)
            .filter_map(|e| {
                e.path
                    .file_stem()
                    .and_then(|s| s.to_str().map(|s| (s.to_string(), e.path.clone())))
            })
            .collect();
        let project_root = PathBuf::from("/repo");
        let ctx = ClusterContext::from_entries(
            entries,
            &work_item_by_id,
            &plans_by_id,
            &project_root,
            cfg,
        );
        compute_clusters_with_backfill(entries, &ctx)
    }

    #[test]
    fn same_slug_clusters_into_one_entry() {
        let entries = vec![
            entry(DocTypeKey::Plans, "foo", 10, "Plan for Foo"),
            entry(DocTypeKey::PlanReviews, "foo", 20, "Review"),
            entry(DocTypeKey::WorkItems, "foo", 5, "Work Item"),
        ];
        let clusters = compute(&entries);
        assert_eq!(clusters.len(), 1);
        let c = &clusters[0];
        assert_eq!(c.slug, "foo");
        assert_eq!(c.entries.len(), 3);
    }

    #[test]
    fn canonical_ordering_is_work_item_then_plan_then_review() {
        let entries = vec![
            entry(DocTypeKey::PlanReviews, "foo", 30, "Review"),
            entry(DocTypeKey::Plans, "foo", 20, "Plan"),
            entry(DocTypeKey::WorkItems, "foo", 10, "Work Item"),
        ];
        let clusters = compute(&entries);
        let kinds: Vec<DocTypeKey> = clusters[0].entries.iter().map(|e| e.r#type).collect();
        assert_eq!(
            kinds,
            vec![
                DocTypeKey::WorkItems,
                DocTypeKey::Plans,
                DocTypeKey::PlanReviews,
            ]
        );
    }

    #[test]
    fn mtime_breaks_ties_within_a_type() {
        let entries = vec![
            entry(DocTypeKey::PlanReviews, "foo", 300, "Review 3"),
            entry(DocTypeKey::PlanReviews, "foo", 100, "Review 1"),
            entry(DocTypeKey::PlanReviews, "foo", 200, "Review 2"),
        ];
        let clusters = compute(&entries);
        let titles: Vec<String> = clusters[0]
            .entries
            .iter()
            .map(|e| e.title.clone())
            .collect();
        assert_eq!(titles, vec!["Review 1", "Review 2", "Review 3"]);
    }

    #[test]
    fn completeness_flags_track_present_types() {
        let entries = vec![
            entry(DocTypeKey::WorkItems, "foo", 10, "T"),
            entry(DocTypeKey::Plans, "foo", 20, "P"),
            entry(DocTypeKey::Decisions, "foo", 30, "D"),
        ];
        let clusters = compute(&entries);
        // Decisions is orphan-by-design, so it forms its own per-path
        // bucket and never merges with the lifecycle (WorkItems + Plans)
        // slug-bucket.
        let lifecycle = clusters
            .iter()
            .find(|c| c.entries.iter().any(|e| e.r#type == DocTypeKey::WorkItems))
            .expect("lifecycle cluster present");
        let c = &lifecycle.completeness;
        assert!(c.has_work_item);
        assert!(c.has_plan);
        assert!(!c.has_decision);
        assert!(!c.has_research);
        assert!(!c.has_plan_review);
        assert!(!c.has_validation);
        assert!(!c.has_pr_description);
        assert!(!c.has_pr_review);
        assert!(!c.has_notes);
        assert!(!c.has_design_gap);
        assert!(!c.has_design_inventory);
    }

    #[test]
    fn present_contains_workflow_keys_in_canonical_order() {
        let entries = vec![
            entry(DocTypeKey::Plans, "foo", 10, "P"),
            entry(DocTypeKey::WorkItems, "foo", 5, "T"),
        ];
        let clusters = compute(&entries);
        assert_eq!(
            clusters[0].completeness.present,
            vec!["work-items".to_string(), "plans".to_string()]
        );
    }

    #[test]
    fn present_for_solitary_work_item_is_single_entry() {
        let entries = vec![entry(DocTypeKey::WorkItems, "foo", 5, "T")];
        let clusters = compute(&entries);
        assert_eq!(
            clusters[0].completeness.present,
            vec!["work-items".to_string()]
        );
    }

    #[test]
    fn backfill_map_carries_cluster_completeness_for_every_clustered_entry() {
        let entries = vec![
            entry(DocTypeKey::WorkItems, "foo", 10, "T"),
            entry(DocTypeKey::Plans, "foo", 20, "P"),
        ];
        let (clusters, backfill, _) = compute_with_backfill(&entries);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].entries.len(), 2);
        for e in &clusters[0].entries {
            let c = e
                .completeness
                .as_ref()
                .expect("clustered entry should have completeness");
            assert!(c.has_work_item);
            assert!(c.has_plan);
            assert_eq!(c.present, clusters[0].completeness.present);
            let bf = backfill
                .get(&e.path)
                .expect("backfill map should contain every clustered entry path");
            assert_eq!(bf.present, clusters[0].completeness.present);
        }
    }

    #[test]
    fn orphan_entries_are_absent_from_backfill_map() {
        let mut orphan = entry(DocTypeKey::Plans, "x", 10, "P");
        orphan.slug = None;
        let (clusters, backfill, _) = compute_with_backfill(&[orphan]);
        assert!(clusters.is_empty());
        assert!(backfill.is_empty());
    }

    #[test]
    fn entries_in_distinct_clusters_get_distinct_completeness() {
        let entries = vec![
            entry(DocTypeKey::WorkItems, "foo", 10, "WI-foo"),
            entry(DocTypeKey::Plans, "foo", 20, "P-foo"),
            entry(DocTypeKey::WorkItems, "bar", 30, "WI-bar"),
        ];
        let (clusters, backfill, _) = compute_with_backfill(&entries);
        assert_eq!(clusters.len(), 2);
        let foo = clusters.iter().find(|c| c.slug == "foo").unwrap();
        let bar = clusters.iter().find(|c| c.slug == "bar").unwrap();
        assert!(foo.completeness.has_plan);
        assert!(!bar.completeness.has_plan);
        for e in &foo.entries {
            assert!(backfill[&e.path].has_plan);
        }
        for e in &bar.entries {
            assert!(!backfill[&e.path].has_plan);
        }
    }

    #[test]
    fn cluster_entries_completeness_matches_backfill_for_same_path() {
        let entries = vec![
            entry(DocTypeKey::WorkItems, "foo", 10, "T"),
            entry(DocTypeKey::Plans, "foo", 20, "P"),
        ];
        let (clusters, backfill, _) = compute_with_backfill(&entries);
        for cluster in &clusters {
            for e in &cluster.entries {
                let entry_completeness = e
                    .completeness
                    .as_ref()
                    .expect("clustered entry should have completeness");
                let backfill_completeness = backfill
                    .get(&e.path)
                    .expect("backfill must contain every clustered entry");
                assert_eq!(entry_completeness.present, backfill_completeness.present);
            }
        }
    }

    #[test]
    fn present_canonical_ordering_for_all_flags_true() {
        // All entries share slug "foo". Decisions/Notes/Design types are
        // orphan-by-design, so they form their own per-path buckets and
        // don't merge with the lifecycle cluster.
        let entries = vec![
            entry(DocTypeKey::WorkItems, "foo", 1, "T"),
            entry(DocTypeKey::Research, "foo", 2, "R"),
            entry(DocTypeKey::Plans, "foo", 3, "P"),
            entry(DocTypeKey::PlanReviews, "foo", 4, "PR"),
            entry(DocTypeKey::Validations, "foo", 5, "V"),
            entry(DocTypeKey::PrDescriptions, "foo", 6, "PD"),
            entry(DocTypeKey::PrReviews, "foo", 7, "PrR"),
        ];
        let clusters = compute(&entries);
        let foo = clusters.iter().find(|c| c.slug == "foo").unwrap();
        assert_eq!(
            foo.completeness.present,
            vec![
                "work-items".to_string(),
                "research".to_string(),
                "plans".to_string(),
                "plan-reviews".to_string(),
                "validations".to_string(),
                "pr-descriptions".to_string(),
                "pr-reviews".to_string(),
            ]
        );
    }

    #[test]
    fn completeness_camelcase_field_names_match_typescript_interface() {
        let entries = vec![
            entry(DocTypeKey::DesignGaps, "foo", 10, "Gap"),
            entry(DocTypeKey::DesignInventories, "foo", 20, "Inventory"),
        ];
        let clusters = compute(&entries);
        // Orphan-by-design types each get their own per-path bucket.
        let any = clusters.iter().find(|c| c.completeness.has_design_gap).unwrap();
        let json = serde_json::to_value(&any.completeness).unwrap();
        assert_eq!(json["hasDesignGap"], true);
    }

    #[test]
    fn templates_are_excluded_from_clusters() {
        let mut t = entry(DocTypeKey::Plans, "shared", 10, "Plan");
        let mut tmpl = entry(DocTypeKey::Templates, "shared", 20, "Template");
        tmpl.slug = Some("shared".to_string());
        t.slug = Some("shared".to_string());
        let clusters = compute(&[t, tmpl]);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].entries.len(), 1);
        assert_eq!(clusters[0].entries[0].r#type, DocTypeKey::Plans);
    }

    #[test]
    fn entries_without_slug_are_excluded() {
        let mut e = entry(DocTypeKey::Plans, "x", 10, "P");
        e.slug = None;
        let clusters = compute(&[e]);
        assert!(clusters.is_empty());
    }

    #[test]
    fn last_changed_ms_is_max_mtime_across_entries() {
        let entries = vec![
            entry(DocTypeKey::WorkItems, "foo", 100, "T"),
            entry(DocTypeKey::Plans, "foo", 500, "P"),
            entry(DocTypeKey::PlanReviews, "foo", 300, "R"),
        ];
        let clusters = compute(&entries);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].last_changed_ms, 500);
    }

    #[test]
    fn last_changed_ms_for_single_entry_is_that_entry_mtime() {
        let entries = vec![entry(DocTypeKey::Plans, "solo", 42, "P")];
        let clusters = compute(&entries);
        assert_eq!(clusters[0].last_changed_ms, 42);
    }

    #[test]
    fn last_changed_ms_is_per_cluster_and_survives_slug_sort() {
        let entries = vec![
            entry(DocTypeKey::Plans, "foo", 100, "P-foo"),
            entry(DocTypeKey::WorkItems, "foo", 500, "T-foo"),
            entry(DocTypeKey::Plans, "bar", 900, "P-bar"),
            entry(DocTypeKey::WorkItems, "bar", 200, "T-bar"),
        ];
        let clusters = compute(&entries);
        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].slug, "bar");
        assert_eq!(clusters[0].last_changed_ms, 900);
        assert_eq!(clusters[1].slug, "foo");
        assert_eq!(clusters[1].last_changed_ms, 500);
    }

    #[test]
    fn clusters_are_sorted_by_slug_alphabetically() {
        let entries = vec![
            entry(DocTypeKey::Plans, "bravo", 10, "B"),
            entry(DocTypeKey::Plans, "alpha", 20, "A"),
            entry(DocTypeKey::Plans, "charlie", 30, "C"),
        ];
        let clusters = compute(&entries);
        let slugs: Vec<String> = clusters.iter().map(|c| c.slug.clone()).collect();
        assert_eq!(slugs, vec!["alpha", "bravo", "charlie"]);
    }

    #[test]
    fn phase_1_id_prefixed_and_bare_slugs_now_cluster_into_one_bucket() {
        let cfg = WorkItemConfig::default();
        let plan = entry_for_test_with_filename(
            DocTypeKey::Plans,
            "2026-05-31-0040-pipeline-visualisation-overhaul.md",
            &cfg,
        );
        let wi = entry_for_test_with_filename(
            DocTypeKey::WorkItems,
            "0040-pipeline-visualisation-overhaul.md",
            &cfg,
        );
        let (clusters, _, _) = compute_with_backfill(&[plan, wi]);
        assert_eq!(clusters.len(), 1);
    }

    // ── Phase 4 cluster-key integration tests ─────────────────────────────

    #[test]
    fn plan_with_parent_work_item_id_clusters_with_the_work_item() {
        let cfg = WorkItemConfig::default();
        let mut wi = entry_for_test(DocTypeKey::WorkItems, "pipeline", 1, "WI");
        wi.work_item_id = Some("0040".into());
        wi.path = PathBuf::from("/repo/meta/work/0040-pipeline.md");
        let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 2, "Plan");
        plan.path = PathBuf::from("/repo/meta/plans/2026-05-31-0040-pipeline.md");
        plan.frontmatter = json!({ "parent": "work-item:0040" });
        let (clusters, _, cluster_key_by_path) =
            run_clusters(&[wi.clone(), plan.clone()], &cfg);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].cluster_key.as_deref(), Some("0040"));
        assert_eq!(clusters[0].slug, "pipeline");
        assert!(clusters[0]
            .entries
            .iter()
            .any(|e| e.r#type == DocTypeKey::Plans));
        assert!(clusters[0]
            .entries
            .iter()
            .any(|e| e.r#type == DocTypeKey::WorkItems));
        assert_eq!(cluster_key_by_path[&wi.path], Some("0040".into()));
        assert_eq!(cluster_key_by_path[&plan.path], Some("0040".into()));
    }

    #[test]
    fn validation_with_target_path_clusters_via_plan_parent() {
        let cfg = WorkItemConfig::default();
        let mut wi = entry_for_test(DocTypeKey::WorkItems, "pipeline", 1, "WI");
        wi.work_item_id = Some("0040".into());
        wi.path = PathBuf::from("/repo/meta/work/0040-pipeline.md");
        let plan_path = PathBuf::from("/repo/meta/plans/2026-05-31-0040-pipeline.md");
        let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 2, "Plan");
        plan.path = plan_path.clone();
        plan.frontmatter = json!({ "parent": "work-item:0040" });
        let mut val = entry_for_test(DocTypeKey::Validations, "pipeline", 3, "Val");
        val.path = PathBuf::from("/repo/meta/validations/2026-05-31-pipeline-validation.md");
        val.frontmatter =
            json!({ "target": "meta/plans/2026-05-31-0040-pipeline.md" });
        let (clusters, _, _) = run_clusters(&[wi, plan, val.clone()], &cfg);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].cluster_key.as_deref(), Some("0040"));
        assert!(clusters[0].entries.iter().any(|e| e.path == val.path));
    }

    #[test]
    fn work_item_review_no_date_filename_clusters_via_target() {
        let cfg = WorkItemConfig::default();
        let mut wi = entry_for_test(DocTypeKey::WorkItems, "design-token-system", 1, "WI");
        wi.work_item_id = Some("0033".into());
        wi.path = PathBuf::from("/repo/meta/work/0033-design-token-system.md");
        let mut review = entry_for_test(DocTypeKey::WorkItemReviews, "design-token-system", 2, "R");
        review.path = PathBuf::from(
            "/repo/meta/reviews/work/0033-design-token-system-review-1.md",
        );
        review.frontmatter =
            json!({ "target": "meta/work/0033-design-token-system.md" });
        let (clusters, _, _) = run_clusters(&[wi.clone(), review.clone()], &cfg);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].cluster_key.as_deref(), Some("0033"));
        assert!(clusters[0].entries.iter().any(|e| e.path == review.path));
    }

    #[test]
    fn plan_without_typed_linkage_falls_back_to_slug_bucket() {
        let cfg = WorkItemConfig::default();
        let plan = entry_for_test(DocTypeKey::Plans, "orphan-plan", 1, "Plan");
        let (clusters, _, cluster_key_by_path) = run_clusters(&[plan.clone()], &cfg);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].slug, "orphan-plan");
        assert_eq!(clusters[0].cluster_key, None);
        assert_eq!(cluster_key_by_path[&plan.path], None);
    }

    #[test]
    fn legacy_work_item_id_path_shape_resolves_to_work_item_cluster() {
        let cfg = WorkItemConfig::default();
        let mut wi = entry_for_test(DocTypeKey::WorkItems, "design-token-system", 1, "WI");
        wi.work_item_id = Some("0033".into());
        wi.path = PathBuf::from("/repo/meta/work/0033-design-token-system.md");
        let mut plan = entry_for_test(DocTypeKey::Plans, "tokens", 2, "Plan");
        plan.frontmatter =
            json!({ "work_item_id": "meta/work/0033-design-token-system.md" });
        let (clusters, _, _) = run_clusters(&[wi, plan], &cfg);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].cluster_key.as_deref(), Some("0033"));
    }

    #[test]
    fn project_prefixed_workspace_clusters_correctly() {
        let cfg = WorkItemConfig::with_pattern_for_test("PROJ", 4);
        let mut wi = entry_for_test(DocTypeKey::WorkItems, "pipeline", 1, "WI");
        wi.work_item_id = Some("PROJ-0040".into());
        let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 2, "Plan");
        plan.frontmatter = json!({ "parent": "work-item:PROJ-0040" });
        let (clusters, _, _) = run_clusters(&[wi, plan], &cfg);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].cluster_key.as_deref(), Some("PROJ-0040"));
    }

    #[test]
    fn notes_remain_orphaned_when_they_carry_no_linkage() {
        let cfg = WorkItemConfig::default();
        let note = entry_for_test(DocTypeKey::Notes, "random-thought", 1, "N");
        let (clusters, _, _) = run_clusters(&[note], &cfg);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].cluster_key, None);
        assert_eq!(clusters[0].slug, "random-thought");
    }

    #[test]
    fn orphan_types_with_colliding_slugs_do_not_merge() {
        let cfg = WorkItemConfig::default();
        let mut note_a = entry_for_test(DocTypeKey::Notes, "shared", 1, "A");
        note_a.path = PathBuf::from("/repo/meta/notes/a.md");
        let mut note_b = entry_for_test(DocTypeKey::Notes, "shared", 2, "B");
        note_b.path = PathBuf::from("/repo/meta/notes/b.md");
        let (clusters, _, _) = run_clusters(&[note_a, note_b], &cfg);
        assert_eq!(clusters.len(), 2, "orphan-type notes must not slug-merge");
    }

    #[test]
    fn research_with_no_parent_merges_with_work_item_via_slug_match() {
        // Regression test for the "templates-view-redesign" split-cluster
        // bug: a research file whose filename slug matches the work-item's
        // slug, but which carries no parent/work_item_id frontmatter, used
        // to land in a separate slug-only bucket. Both clusters then took
        // the same representative slug, so `/lifecycle/<slug>` would
        // return whichever ended up first in the sort — typically the
        // smaller, research-only one.
        let cfg = WorkItemConfig::default();
        let mut wi = entry_for_test(DocTypeKey::WorkItems, "templates-view-redesign", 1, "WI");
        wi.work_item_id = Some("0042".into());
        wi.path = PathBuf::from("/repo/meta/work/0042-templates-view-redesign.md");
        let mut plan = entry_for_test(DocTypeKey::Plans, "templates-view-redesign", 2, "Plan");
        plan.path = PathBuf::from(
            "/repo/meta/plans/2026-05-18-0042-templates-view-redesign.md",
        );
        plan.frontmatter = json!({ "parent": "work-item:0042" });
        let mut research = entry_for_test(
            DocTypeKey::Research,
            "templates-view-redesign",
            3,
            "Research",
        );
        research.path = PathBuf::from(
            "/repo/meta/research/codebase/2026-05-18-0042-templates-view-redesign.md",
        );
        // Deliberately no parent / work_item_id — the failure mode.
        let (clusters, _, cluster_key_by_path) =
            run_clusters(&[wi.clone(), plan.clone(), research.clone()], &cfg);
        assert_eq!(clusters.len(), 1, "research must merge with WI bucket");
        assert_eq!(clusters[0].cluster_key.as_deref(), Some("0042"));
        assert!(clusters[0]
            .entries
            .iter()
            .any(|e| e.r#type == DocTypeKey::Research));
        // The research's cluster_key is back-filled with the merged key
        // so /api/related and the wire shape agree.
        assert_eq!(
            cluster_key_by_path[&research.path].as_deref(),
            Some("0042"),
            "slug-merged research must adopt the cluster's key",
        );
    }

    #[test]
    fn lifecycle_type_with_no_linkage_still_slug_merges_with_work_item() {
        let cfg = WorkItemConfig::default();
        let mut wi = entry_for_test(DocTypeKey::WorkItems, "shared-slug", 1, "WI");
        wi.work_item_id = Some("0040".into());
        let plan = entry_for_test(DocTypeKey::Plans, "shared-slug", 2, "Plan");
        let (clusters, _, _) = run_clusters(&[wi, plan], &cfg);
        assert_eq!(clusters.len(), 1);
    }

    #[test]
    fn cluster_key_is_backfilled_onto_every_clustered_entry() {
        let cfg = WorkItemConfig::default();
        let mut wi = entry_for_test(DocTypeKey::WorkItems, "pipeline", 1, "WI");
        wi.work_item_id = Some("0040".into());
        let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 2, "Plan");
        plan.frontmatter = json!({ "parent": "work-item:0040" });
        let (_, _, cluster_key_by_path) =
            run_clusters(&[wi.clone(), plan.clone()], &cfg);
        assert_eq!(cluster_key_by_path[&wi.path].as_deref(), Some("0040"));
        assert_eq!(cluster_key_by_path[&plan.path].as_deref(), Some("0040"));
    }

    #[test]
    fn orphan_by_design_type_joins_typed_cluster_when_slug_matches() {
        // A Decision whose slug matches a work-item's slug joins the
        // work-item's cluster via the slug → cluster_key bridge, even
        // though Decisions are orphan-by-design. This is the path the
        // ac2-coverage e2e fixture relies on. The orphan-vs-orphan
        // slug-collision guard is unchanged (see
        // `orphan_types_with_colliding_slugs_do_not_merge`).
        let cfg = WorkItemConfig::default();
        let mut wi = entry_for_test(DocTypeKey::WorkItems, "ac2-coverage", 1, "WI");
        wi.work_item_id = Some("0099".into());
        wi.path = PathBuf::from("/repo/meta/work/0099-ac2-coverage.md");
        for orphan_kind in [
            DocTypeKey::Decisions,
            DocTypeKey::Notes,
            DocTypeKey::DesignGaps,
            DocTypeKey::DesignInventories,
        ] {
            let mut orphan = entry_for_test(orphan_kind, "ac2-coverage", 2, "O");
            orphan.path = PathBuf::from(format!("/repo/meta/{:?}/ac2-coverage.md", orphan_kind));
            let (clusters, _, cluster_key_by_path) =
                run_clusters(&[wi.clone(), orphan.clone()], &cfg);
            assert_eq!(clusters.len(), 1, "{orphan_kind:?} must merge via bridge");
            assert_eq!(clusters[0].cluster_key.as_deref(), Some("0099"), "{orphan_kind:?}");
            assert_eq!(
                cluster_key_by_path[&orphan.path].as_deref(),
                Some("0099"),
                "{orphan_kind:?}: bridged entry must adopt cluster's key",
            );
        }
    }

    #[test]
    fn cluster_key_field_serialises_as_camelcase_on_wire() {
        let cfg = WorkItemConfig::default();
        let mut wi = entry_for_test(DocTypeKey::WorkItems, "pipeline", 1, "WI");
        wi.work_item_id = Some("0042".into());
        let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 2, "Plan");
        plan.frontmatter = json!({ "parent": "work-item:0042" });
        let (clusters, _, _) = run_clusters(&[wi, plan], &cfg);
        let cluster = clusters.into_iter().next().expect("one cluster");
        let json = serde_json::to_value(&cluster).unwrap();
        assert_eq!(json["clusterKey"], "0042");
        for entry_json in json["entries"].as_array().expect("entries array") {
            assert_eq!(entry_json["clusterKey"], "0042");
        }
    }

    #[test]
    fn cluster_key_serialises_as_null_when_absent() {
        let cfg = WorkItemConfig::default();
        let plan = entry_for_test(DocTypeKey::Plans, "orphan-plan", 1, "Plan");
        let (clusters, _, _) = run_clusters(&[plan], &cfg);
        let cluster = clusters.into_iter().next().expect("one cluster");
        let json = serde_json::to_value(&cluster).unwrap();
        assert_eq!(json["clusterKey"], serde_json::Value::Null);
        assert!(json.as_object().unwrap().contains_key("clusterKey"));
    }

    #[test]
    fn cluster_without_work_item_uses_alphabetically_first_slug() {
        let cfg = WorkItemConfig::default();
        let mut a = entry_for_test(DocTypeKey::Plans, "beta-slug", 1, "A");
        a.path = PathBuf::from("/repo/meta/plans/a.md");
        let mut b = entry_for_test(DocTypeKey::Research, "alpha-slug", 2, "B");
        b.path = PathBuf::from("/repo/meta/research/b.md");
        a.frontmatter = json!({ "parent": "work-item:0040" });
        b.frontmatter = json!({ "parent": "work-item:0040" });
        let (clusters, _, _) = run_clusters(&[a, b], &cfg);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].slug, "beta-slug");
    }
}
