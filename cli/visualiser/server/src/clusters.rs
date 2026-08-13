use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::config::WorkItemConfig;
use crate::indexer::IndexEntry;
use corpus::cluster::ClusterEntry;
use corpus::DocTypeKey;

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

impl From<corpus::cluster::Completeness> for Completeness {
    fn from(c: corpus::cluster::Completeness) -> Self {
        Self {
            has_work_item: c.has_work_item,
            has_research: c.has_research,
            has_plan: c.has_plan,
            has_plan_review: c.has_plan_review,
            has_validation: c.has_validation,
            has_pr_description: c.has_pr_description,
            has_pr_review: c.has_pr_review,
            has_decision: c.has_decision,
            has_notes: c.has_notes,
            has_design_inventory: c.has_design_inventory,
            has_design_gap: c.has_design_gap,
            present: c.present,
        }
    }
}

impl ClusterEntry for IndexEntry {
    fn path(&self) -> &Path {
        &self.path
    }
    fn doc_type(&self) -> DocTypeKey {
        self.r#type
    }
    fn slug(&self) -> Option<&str> {
        self.slug.as_deref()
    }
    fn title(&self) -> &str {
        &self.title
    }
    fn mtime_ms(&self) -> i64 {
        self.mtime_ms
    }
    fn frontmatter_parsed(&self) -> bool {
        self.frontmatter_state == "parsed"
    }
    fn work_item_id(&self) -> Option<&str> {
        self.work_item_id.as_deref()
    }
    fn parent(&self) -> Option<&str> {
        self.frontmatter.get("parent").and_then(|v| v.as_str())
    }
    fn target(&self) -> Option<&str> {
        self.frontmatter.get("target").and_then(|v| v.as_str())
    }
}

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

/// Snapshots and conventions the cluster-key resolver needs, threaded from the
/// indexer caller into `corpus::cluster`.
pub struct ClusterContext<'a> {
    pub work_item_by_id: &'a HashMap<String, PathBuf>,
    pub plans_by_id: &'a HashMap<String, PathBuf>,
    pub project_root: &'a Path,
    pub work_item_cfg: &'a WorkItemConfig,
}

impl<'a> ClusterContext<'a> {
    pub fn from_entries(
        _entries: &'a [IndexEntry],
        work_item_by_id: &'a HashMap<String, PathBuf>,
        plans_by_id: &'a HashMap<String, PathBuf>,
        project_root: &'a Path,
        work_item_cfg: &'a WorkItemConfig,
    ) -> Self {
        Self {
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
impl Default for EmptyClusterFixture {
    fn default() -> Self {
        Self::new()
    }
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
            work_item_by_id: &self.wi,
            plans_by_id: &self.pl,
            project_root: &self.root,
            work_item_cfg: &self.cfg,
        }
    }
}

pub fn compute_clusters(
    entries: &[IndexEntry],
    ctx: &ClusterContext<'_>,
) -> Vec<LifecycleCluster> {
    compute_clusters_with_backfill(entries, ctx).0
}

/// Like `compute_clusters`, but also returns:
///
/// - A `HashMap` keyed by every clustered entry's canonical path with the
///   cluster's `Completeness`. Callers apply the map to `Indexer::entries`
///   so per-entry `IndexEntry.completeness` mirrors the cluster's view.
/// - A `HashMap` keyed by every non-template entry's path with the
///   resolved `cluster_key` (or `None` for slug-fallback / orphan-bucket
///   entries). Callers apply this map onto `IndexEntry.cluster_key` so
///   the canonical entries map stays in lockstep with the cluster view.
///
/// The pure grouping algorithm lives in `corpus::cluster`; this adapter
/// injects the id convention and re-projects the path-keyed result back onto
/// the server's `IndexEntry`-shaped wire types.
pub fn compute_clusters_with_backfill(
    entries: &[IndexEntry],
    ctx: &ClusterContext<'_>,
) -> (
    Vec<LifecycleCluster>,
    HashMap<PathBuf, Completeness>,
    HashMap<PathBuf, Option<String>>,
) {
    let corpus_ctx = corpus::cluster::ClusterContext::from_entries(
        entries,
        ctx.work_item_by_id,
        ctx.plans_by_id,
        ctx.project_root,
        ctx.work_item_cfg.scheme(),
        ctx.work_item_cfg.scanner(),
    );
    let clustering = corpus::cluster::compute(entries, &corpus_ctx);

    let clusters = clustering
        .clusters
        .into_iter()
        .map(|c| {
            let completeness = Completeness::from(c.completeness);
            let cluster_entries = c
                .members
                .iter()
                .map(|&i| {
                    let mut e = entries[i].clone();
                    e.completeness = Some(completeness.clone());
                    e.cluster_key = clustering
                        .cluster_key_by_path
                        .get(&e.path)
                        .cloned()
                        .unwrap_or(None);
                    e
                })
                .collect();
            LifecycleCluster {
                slug: c.slug,
                title: c.title,
                entries: cluster_entries,
                completeness,
                last_changed_ms: c.last_changed_ms,
                cluster_key: c.cluster_key,
            }
        })
        .collect();

    let backfill = clustering
        .completeness_by_path
        .into_iter()
        .map(|(k, v)| (k, Completeness::from(v)))
        .collect();
    (clusters, backfill, clustering.cluster_key_by_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{entry_for_test, entry_for_test_with_filename};
    use serde_json::json;

    fn entry(
        kind: DocTypeKey,
        slug: &str,
        mtime_ms: i64,
        title: &str,
    ) -> IndexEntry {
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
    /// themselves — `work_item_by_id` from `WorkItems` entries, `plans_by_id`
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
            .filter_map(|e| {
                e.work_item_id.clone().map(|id| (id, e.path.clone()))
            })
            .collect();
        let plans_by_id: HashMap<String, PathBuf> = entries
            .iter()
            .filter(|e| e.r#type == DocTypeKey::Plans)
            .filter_map(|e| {
                e.path.file_stem().and_then(|s| {
                    s.to_str().map(|s| (s.to_string(), e.path.clone()))
                })
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
        let kinds: Vec<DocTypeKey> =
            clusters[0].entries.iter().map(|e| e.r#type).collect();
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
            .find(|c| {
                c.entries.iter().any(|e| e.r#type == DocTypeKey::WorkItems)
            })
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
            let bf = backfill.get(&e.path).expect(
                "backfill map should contain every clustered entry path",
            );
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
                assert_eq!(
                    entry_completeness.present,
                    backfill_completeness.present
                );
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
        let any = clusters
            .iter()
            .find(|c| c.completeness.has_design_gap)
            .unwrap();
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
        let slugs: Vec<String> =
            clusters.iter().map(|c| c.slug.clone()).collect();
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

    // ── Cluster-key integration tests ─────────────────────────────────────

    #[test]
    fn plan_with_parent_work_item_id_clusters_with_the_work_item() {
        let cfg = WorkItemConfig::default();
        let mut wi = entry_for_test(DocTypeKey::WorkItems, "pipeline", 1, "WI");
        wi.work_item_id = Some("0040".into());
        wi.path = PathBuf::from("/repo/meta/work/0040-pipeline.md");
        let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 2, "Plan");
        plan.path =
            PathBuf::from("/repo/meta/plans/2026-05-31-0040-pipeline.md");
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
        let plan_path =
            PathBuf::from("/repo/meta/plans/2026-05-31-0040-pipeline.md");
        let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 2, "Plan");
        plan.path = plan_path.clone();
        plan.frontmatter = json!({ "parent": "work-item:0040" });
        let mut val =
            entry_for_test(DocTypeKey::Validations, "pipeline", 3, "Val");
        val.path = PathBuf::from(
            "/repo/meta/validations/2026-05-31-pipeline-validation.md",
        );
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
        let mut wi = entry_for_test(
            DocTypeKey::WorkItems,
            "design-token-system",
            1,
            "WI",
        );
        wi.work_item_id = Some("0033".into());
        wi.path = PathBuf::from("/repo/meta/work/0033-design-token-system.md");
        let mut review = entry_for_test(
            DocTypeKey::WorkItemReviews,
            "design-token-system",
            2,
            "R",
        );
        review.path = PathBuf::from(
            "/repo/meta/reviews/work/0033-design-token-system-review-1.md",
        );
        review.frontmatter =
            json!({ "target": "meta/work/0033-design-token-system.md" });
        let (clusters, _, _) =
            run_clusters(&[wi.clone(), review.clone()], &cfg);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].cluster_key.as_deref(), Some("0033"));
        assert!(clusters[0].entries.iter().any(|e| e.path == review.path));
    }

    #[test]
    fn plan_without_typed_linkage_falls_back_to_slug_bucket() {
        let cfg = WorkItemConfig::default();
        let plan = entry_for_test(DocTypeKey::Plans, "orphan-plan", 1, "Plan");
        let (clusters, _, cluster_key_by_path) =
            run_clusters(std::slice::from_ref(&plan), &cfg);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].slug, "orphan-plan");
        assert_eq!(clusters[0].cluster_key, None);
        assert_eq!(cluster_key_by_path[&plan.path], None);
    }

    #[test]
    fn path_shape_parent_resolves_to_work_item_cluster() {
        // Integration-level coverage of the path-shape `parent:` resolution
        // (id_from_value's TypedRef::Path → extract_id branch): a plan whose
        // `parent:` is a work-item path clusters with that work item.
        let cfg = WorkItemConfig::default();
        let mut wi = entry_for_test(
            DocTypeKey::WorkItems,
            "design-token-system",
            1,
            "WI",
        );
        wi.work_item_id = Some("0033".into());
        wi.path = PathBuf::from("/repo/meta/work/0033-design-token-system.md");
        let mut plan = entry_for_test(DocTypeKey::Plans, "tokens", 2, "Plan");
        plan.frontmatter =
            json!({ "parent": "meta/work/0033-design-token-system.md" });
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
        let mut wi = entry_for_test(
            DocTypeKey::WorkItems,
            "templates-view-redesign",
            1,
            "WI",
        );
        wi.work_item_id = Some("0042".into());
        wi.path =
            PathBuf::from("/repo/meta/work/0042-templates-view-redesign.md");
        let mut plan = entry_for_test(
            DocTypeKey::Plans,
            "templates-view-redesign",
            2,
            "Plan",
        );
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
        let mut wi =
            entry_for_test(DocTypeKey::WorkItems, "shared-slug", 1, "WI");
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
        let mut wi =
            entry_for_test(DocTypeKey::WorkItems, "ac2-coverage", 1, "WI");
        wi.work_item_id = Some("0099".into());
        wi.path = PathBuf::from("/repo/meta/work/0099-ac2-coverage.md");
        for orphan_kind in [
            DocTypeKey::Decisions,
            DocTypeKey::Notes,
            DocTypeKey::DesignGaps,
            DocTypeKey::DesignInventories,
        ] {
            let mut orphan =
                entry_for_test(orphan_kind, "ac2-coverage", 2, "O");
            orphan.path = PathBuf::from(format!(
                "/repo/meta/{orphan_kind:?}/ac2-coverage.md"
            ));
            let (clusters, _, cluster_key_by_path) =
                run_clusters(&[wi.clone(), orphan.clone()], &cfg);
            assert_eq!(
                clusters.len(),
                1,
                "{orphan_kind:?} must merge via bridge"
            );
            assert_eq!(
                clusters[0].cluster_key.as_deref(),
                Some("0099"),
                "{orphan_kind:?}"
            );
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
