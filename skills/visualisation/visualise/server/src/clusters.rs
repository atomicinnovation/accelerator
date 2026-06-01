use std::collections::HashMap;
use std::path::PathBuf;

use serde::Serialize;

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
}

pub fn compute_clusters(entries: &[IndexEntry]) -> Vec<LifecycleCluster> {
    compute_clusters_with_backfill(entries).0
}

/// Like `compute_clusters`, but also returns a `HashMap` keyed by every
/// clustered entry's canonical path with the cluster's `Completeness`.
/// Callers apply the map to `Indexer::entries` so per-entry
/// `IndexEntry.completeness` mirrors the cluster's view of the same slug.
/// The cluster-clone entries already carry the backfilled completeness;
/// this map exists so the canonical entries map stays in lockstep.
pub fn compute_clusters_with_backfill(
    entries: &[IndexEntry],
) -> (Vec<LifecycleCluster>, HashMap<PathBuf, Completeness>) {
    let mut buckets: HashMap<String, Vec<IndexEntry>> = HashMap::new();
    for e in entries {
        if matches!(e.r#type, DocTypeKey::Templates) {
            continue;
        }
        let Some(slug) = e.slug.clone() else { continue };
        buckets.entry(slug).or_default().push(e.clone());
    }

    let mut backfill: HashMap<PathBuf, Completeness> = HashMap::new();
    let mut clusters: Vec<LifecycleCluster> = buckets
        .into_iter()
        .map(|(slug, mut entries)| {
            entries.sort_by(|a, b| {
                canonical_rank(a.r#type)
                    .cmp(&canonical_rank(b.r#type))
                    .then(a.mtime_ms.cmp(&b.mtime_ms))
            });
            let last_changed_ms = entries.iter().map(|e| e.mtime_ms).max().unwrap_or(0);
            let title = derive_title(&slug, &entries);
            let completeness = derive_completeness(&entries);
            for e in entries.iter_mut() {
                e.completeness = Some(completeness.clone());
                backfill.insert(e.path.clone(), completeness.clone());
            }
            LifecycleCluster {
                slug,
                title,
                entries,
                completeness,
                last_changed_ms,
            }
        })
        .collect();

    clusters.sort_by(|a, b| a.slug.cmp(&b.slug));
    (clusters, backfill)
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
    use crate::test_support::entry_for_test;

    fn entry(kind: DocTypeKey, slug: &str, mtime_ms: i64, title: &str) -> IndexEntry {
        entry_for_test(kind, slug, mtime_ms, title)
    }

    #[test]
    fn same_slug_clusters_into_one_entry() {
        let entries = vec![
            entry(DocTypeKey::Plans, "foo", 10, "Plan for Foo"),
            entry(DocTypeKey::PlanReviews, "foo", 20, "Review"),
            entry(DocTypeKey::WorkItems, "foo", 5, "Work Item"),
        ];
        let clusters = compute_clusters(&entries);
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
        let clusters = compute_clusters(&entries);
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
        let clusters = compute_clusters(&entries);
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
        let clusters = compute_clusters(&entries);
        let c = &clusters[0].completeness;
        assert!(c.has_work_item);
        assert!(c.has_plan);
        assert!(c.has_decision);
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
        let clusters = compute_clusters(&entries);
        assert_eq!(
            clusters[0].completeness.present,
            vec!["work-items".to_string(), "plans".to_string()]
        );
    }

    #[test]
    fn present_for_solitary_work_item_is_single_entry() {
        let entries = vec![entry(DocTypeKey::WorkItems, "foo", 5, "T")];
        let clusters = compute_clusters(&entries);
        assert_eq!(
            clusters[0].completeness.present,
            vec!["work-items".to_string()]
        );
    }

    #[test]
    fn present_includes_long_tail_keys_after_workflow_keys() {
        let entries = vec![
            entry(DocTypeKey::Notes, "foo", 10, "N"),
            entry(DocTypeKey::DesignGaps, "foo", 20, "G"),
        ];
        let clusters = compute_clusters(&entries);
        assert_eq!(
            clusters[0].completeness.present,
            vec!["notes".to_string(), "design-gaps".to_string()]
        );
    }

    #[test]
    fn backfill_map_carries_cluster_completeness_for_every_clustered_entry() {
        let entries = vec![
            entry(DocTypeKey::WorkItems, "foo", 10, "T"),
            entry(DocTypeKey::Plans, "foo", 20, "P"),
        ];
        let (clusters, backfill) = compute_clusters_with_backfill(&entries);
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
        let (clusters, backfill) = compute_clusters_with_backfill(&[orphan]);
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
        let (clusters, backfill) = compute_clusters_with_backfill(&entries);
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
            entry(DocTypeKey::Decisions, "foo", 30, "D"),
        ];
        let (clusters, backfill) = compute_clusters_with_backfill(&entries);
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
        let entries = vec![
            entry(DocTypeKey::WorkItems, "foo", 1, "T"),
            entry(DocTypeKey::Research, "foo", 2, "R"),
            entry(DocTypeKey::Plans, "foo", 3, "P"),
            entry(DocTypeKey::PlanReviews, "foo", 4, "PR"),
            entry(DocTypeKey::Validations, "foo", 5, "V"),
            entry(DocTypeKey::PrDescriptions, "foo", 6, "PD"),
            entry(DocTypeKey::PrReviews, "foo", 7, "PrR"),
            entry(DocTypeKey::Decisions, "foo", 8, "D"),
            entry(DocTypeKey::Notes, "foo", 9, "N"),
            entry(DocTypeKey::DesignInventories, "foo", 10, "DI"),
            entry(DocTypeKey::DesignGaps, "foo", 11, "DG"),
        ];
        let clusters = compute_clusters(&entries);
        assert_eq!(
            clusters[0].completeness.present,
            vec![
                "work-items".to_string(),
                "research".to_string(),
                "plans".to_string(),
                "plan-reviews".to_string(),
                "validations".to_string(),
                "pr-descriptions".to_string(),
                "pr-reviews".to_string(),
                "decisions".to_string(),
                "notes".to_string(),
                "design-inventories".to_string(),
                "design-gaps".to_string(),
            ]
        );
    }

    #[test]
    fn completeness_camelcase_field_names_match_typescript_interface() {
        let entries = vec![
            entry(DocTypeKey::DesignGaps, "foo", 10, "Gap"),
            entry(DocTypeKey::DesignInventories, "foo", 20, "Inventory"),
        ];
        let clusters = compute_clusters(&entries);
        let json = serde_json::to_value(&clusters[0].completeness).unwrap();
        assert_eq!(json["hasDesignGap"], true);
        assert_eq!(json["hasDesignInventory"], true);
    }

    #[test]
    fn design_gap_and_inventory_completeness_flags_are_set() {
        let entries = vec![
            entry(DocTypeKey::DesignGaps, "foo", 10, "Gap"),
            entry(DocTypeKey::DesignInventories, "foo", 20, "Inventory"),
        ];
        let clusters = compute_clusters(&entries);
        assert_eq!(clusters.len(), 1);
        let c = &clusters[0].completeness;
        assert!(c.has_design_gap);
        assert!(c.has_design_inventory);
    }

    #[test]
    fn design_inventory_sorts_before_design_gap_in_cluster() {
        let entries = vec![
            entry(DocTypeKey::DesignGaps, "foo", 10, "Gap"),
            entry(DocTypeKey::DesignInventories, "foo", 20, "Inventory"),
            entry(DocTypeKey::Notes, "foo", 30, "Note"),
        ];
        let clusters = compute_clusters(&entries);
        assert_eq!(clusters.len(), 1);
        let types: Vec<DocTypeKey> = clusters[0].entries.iter().map(|e| e.r#type).collect();
        let notes_pos = types.iter().position(|t| *t == DocTypeKey::Notes).unwrap();
        let inv_pos = types.iter().position(|t| *t == DocTypeKey::DesignInventories).unwrap();
        let gap_pos = types.iter().position(|t| *t == DocTypeKey::DesignGaps).unwrap();
        assert!(notes_pos < inv_pos, "Notes should sort before DesignInventories");
        assert!(inv_pos < gap_pos, "DesignInventories should sort before DesignGaps");
    }

    #[test]
    fn templates_are_excluded_from_clusters() {
        let mut t = entry(DocTypeKey::Plans, "shared", 10, "Plan");
        let mut tmpl = entry(DocTypeKey::Templates, "shared", 20, "Template");
        tmpl.slug = Some("shared".to_string());
        t.slug = Some("shared".to_string());
        let clusters = compute_clusters(&[t, tmpl]);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].entries.len(), 1);
        assert_eq!(clusters[0].entries[0].r#type, DocTypeKey::Plans);
    }

    #[test]
    fn entries_without_slug_are_excluded() {
        let mut e = entry(DocTypeKey::Plans, "x", 10, "P");
        e.slug = None;
        let clusters = compute_clusters(&[e]);
        assert!(clusters.is_empty());
    }

    #[test]
    fn last_changed_ms_is_max_mtime_across_entries() {
        let entries = vec![
            entry(DocTypeKey::WorkItems, "foo", 100, "T"),
            entry(DocTypeKey::Plans, "foo", 500, "P"),
            entry(DocTypeKey::PlanReviews, "foo", 300, "R"),
        ];
        let clusters = compute_clusters(&entries);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].last_changed_ms, 500);
    }

    #[test]
    fn last_changed_ms_for_single_entry_is_that_entry_mtime() {
        let entries = vec![entry(DocTypeKey::Plans, "solo", 42, "P")];
        let clusters = compute_clusters(&entries);
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
        let clusters = compute_clusters(&entries);
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
        let clusters = compute_clusters(&entries);
        let slugs: Vec<String> = clusters.iter().map(|c| c.slug.clone()).collect();
        assert_eq!(slugs, vec!["alpha", "bravo", "charlie"]);
    }
}
