use std::collections::HashMap;

use serde::Serialize;

use crate::docs::DocTypeKey;
use crate::indexer::IndexEntry;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Completeness {
    pub has_ticket: bool,
    pub has_research: bool,
    pub has_plan: bool,
    pub has_plan_review: bool,
    pub has_validation: bool,
    pub has_pr: bool,
    pub has_pr_review: bool,
    pub has_decision: bool,
    pub has_notes: bool,
}

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
    let mut buckets: HashMap<String, Vec<IndexEntry>> = HashMap::new();
    for e in entries {
        if matches!(e.r#type, DocTypeKey::Templates) {
            continue;
        }
        let Some(slug) = e.slug.clone() else { continue };
        buckets.entry(slug).or_default().push(e.clone());
    }

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
    clusters
}

fn canonical_rank(kind: DocTypeKey) -> u8 {
    match kind {
        DocTypeKey::Tickets => 0,
        DocTypeKey::Research => 1,
        DocTypeKey::Plans => 2,
        DocTypeKey::PlanReviews => 3,
        DocTypeKey::Validations => 4,
        DocTypeKey::Prs => 5,
        DocTypeKey::PrReviews => 6,
        DocTypeKey::Decisions => 7,
        DocTypeKey::Notes => 8,
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
        has_ticket: false,
        has_research: false,
        has_plan: false,
        has_plan_review: false,
        has_validation: false,
        has_pr: false,
        has_pr_review: false,
        has_decision: false,
        has_notes: false,
    };
    for e in entries {
        match e.r#type {
            DocTypeKey::Tickets => c.has_ticket = true,
            DocTypeKey::Research => c.has_research = true,
            DocTypeKey::Plans => c.has_plan = true,
            DocTypeKey::PlanReviews => c.has_plan_review = true,
            DocTypeKey::Validations => c.has_validation = true,
            DocTypeKey::Prs => c.has_pr = true,
            DocTypeKey::PrReviews => c.has_pr_review = true,
            DocTypeKey::Decisions => c.has_decision = true,
            DocTypeKey::Notes => c.has_notes = true,
            DocTypeKey::Templates => {}
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
            entry(DocTypeKey::Tickets, "foo", 5, "Ticket"),
        ];
        let clusters = compute_clusters(&entries);
        assert_eq!(clusters.len(), 1);
        let c = &clusters[0];
        assert_eq!(c.slug, "foo");
        assert_eq!(c.entries.len(), 3);
    }

    #[test]
    fn canonical_ordering_is_ticket_then_plan_then_review() {
        let entries = vec![
            entry(DocTypeKey::PlanReviews, "foo", 30, "Review"),
            entry(DocTypeKey::Plans, "foo", 20, "Plan"),
            entry(DocTypeKey::Tickets, "foo", 10, "Ticket"),
        ];
        let clusters = compute_clusters(&entries);
        let kinds: Vec<DocTypeKey> = clusters[0].entries.iter().map(|e| e.r#type).collect();
        assert_eq!(
            kinds,
            vec![
                DocTypeKey::Tickets,
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
        let titles: Vec<String> = clusters[0].entries.iter().map(|e| e.title.clone()).collect();
        assert_eq!(titles, vec!["Review 1", "Review 2", "Review 3"]);
    }

    #[test]
    fn completeness_flags_track_present_types() {
        let entries = vec![
            entry(DocTypeKey::Tickets, "foo", 10, "T"),
            entry(DocTypeKey::Plans, "foo", 20, "P"),
            entry(DocTypeKey::Decisions, "foo", 30, "D"),
        ];
        let clusters = compute_clusters(&entries);
        let c = &clusters[0].completeness;
        assert!(c.has_ticket);
        assert!(c.has_plan);
        assert!(c.has_decision);
        assert!(!c.has_research);
        assert!(!c.has_plan_review);
        assert!(!c.has_validation);
        assert!(!c.has_pr);
        assert!(!c.has_pr_review);
        assert!(!c.has_notes);
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
            entry(DocTypeKey::Tickets, "foo", 100, "T"),
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
            entry(DocTypeKey::Plans,   "foo", 100, "P-foo"),
            entry(DocTypeKey::Tickets, "foo", 500, "T-foo"),
            entry(DocTypeKey::Plans,   "bar", 900, "P-bar"),
            entry(DocTypeKey::Tickets, "bar", 200, "T-bar"),
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
