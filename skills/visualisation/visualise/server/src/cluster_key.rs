//! Composite cluster-key resolver. Walks ADR-0034 typed-linkage
//! frontmatter back to a canonical work-item id. Falls through to
//! `None` when no chain reaches a work item; callers fall back to
//! `IndexEntry.slug`.
//!
//! Target resolution for review/validation entries delegates to
//! `indexer::target_path_from_entry` rather than re-parsing the
//! `target:` frontmatter directly. This keeps the typed-ref
//! vocabulary parser as the single source of truth for path/Plan
//! shapes; this module owns only the `WorkItem` short-circuit and
//! the recursive walk.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tracing::warn;

use crate::config::WorkItemConfig;
use crate::docs::DocTypeKey;
use crate::indexer::{canonicalise_one_id, target_path_from_entry, IndexEntry};
use crate::typed_ref::{parse_typed_ref, TypedRef};

/// The longest legitimate chain in today's vocabulary is
/// work-item-review → plan → parent → work-item (3 hops). 8 gives
/// generous headroom for transitional shapes that bounce through
/// path-target intermediaries during the epic-0057 migration window
/// without measurably impacting cost (each extra hop is one `HashMap`
/// lookup). When the limit is hit, we emit a warn-log so the
/// silent-fallback case is observable.
const MAX_DEPTH: u8 = 8;

pub fn resolve_cluster_key(
    entry: &IndexEntry,
    entries_by_path: &HashMap<PathBuf, &IndexEntry>,
    work_item_by_id: &HashMap<String, PathBuf>,
    plans_by_id: &HashMap<String, PathBuf>,
    project_root: &Path,
    work_item_cfg: &WorkItemConfig,
) -> Option<String> {
    walk(
        entry,
        entries_by_path,
        work_item_by_id,
        plans_by_id,
        project_root,
        work_item_cfg,
        0,
    )
}

fn walk(
    entry: &IndexEntry,
    entries_by_path: &HashMap<PathBuf, &IndexEntry>,
    work_item_by_id: &HashMap<String, PathBuf>,
    plans_by_id: &HashMap<String, PathBuf>,
    project_root: &Path,
    work_item_cfg: &WorkItemConfig,
    depth: u8,
) -> Option<String> {
    if depth >= MAX_DEPTH {
        warn!(
            entry_path = %entry.path.display(),
            entry_type = ?entry.r#type,
            entry_slug = ?entry.slug,
            depth,
            "cluster_key walk truncated at MAX_DEPTH; entry will fall back \
             to slug bucket if a slug is present, otherwise be excluded \
             from clustering",
        );
        return None;
    }
    match entry.r#type {
        DocTypeKey::WorkItems => entry.work_item_id.clone(),
        DocTypeKey::Plans
        | DocTypeKey::Research
        | DocTypeKey::PrDescriptions => {
            parent_or_legacy_id(entry, work_item_cfg)
        }
        DocTypeKey::PlanReviews
        | DocTypeKey::WorkItemReviews
        | DocTypeKey::PrReviews
        | DocTypeKey::Validations => {
            // First: short-circuit a typed `work-item:` target without a
            // filesystem lookup. This is the canonical ADR-0034 shape for
            // work-item reviews.
            let raw = entry.frontmatter.get("target").and_then(|v| v.as_str());
            if let Some(s) = raw {
                if let Some(TypedRef::WorkItem(id)) = parse_typed_ref(s) {
                    return canonicalise_one_id(&id, work_item_cfg);
                }
            }
            // Otherwise: delegate to target_path_from_entry, which owns
            // Plan(id) / Path(p) dispatch + normalize_target_key path
            // safety. Recurse on the resulting entry.
            let target_path = target_path_from_entry(
                entry,
                plans_by_id,
                work_item_by_id,
                work_item_cfg,
                project_root,
            )?;
            let target_entry: &IndexEntry =
                *entries_by_path.get(&target_path)?;
            walk(
                target_entry,
                entries_by_path,
                work_item_by_id,
                plans_by_id,
                project_root,
                work_item_cfg,
                depth + 1,
            )
        }
        DocTypeKey::Decisions
        | DocTypeKey::Notes
        | DocTypeKey::DesignGaps
        | DocTypeKey::DesignInventories
        | DocTypeKey::Templates => None,
    }
}

/// For plans/research/pr-descriptions, accept (in priority order):
/// 1. `parent: "work-item:NNNN"`  (ADR-0034 canonical)
/// 2. `parent: "NNNN"` or bare `parent: "0042"`  (transitional)
/// 3. `work_item_id: "0042"`  (legacy frontmatter)
/// 4. `work_item_id: "meta/work/0033-foo.md"`  (legacy path shape)
fn parent_or_legacy_id(
    entry: &IndexEntry,
    cfg: &WorkItemConfig,
) -> Option<String> {
    if let Some(raw) = entry.frontmatter.get("parent").and_then(|v| v.as_str())
    {
        if let Some(id) = id_from_value(raw, cfg) {
            return Some(id);
        }
    }
    if let Some(raw) = entry
        .frontmatter
        .get("work_item_id")
        .and_then(|v| v.as_str())
    {
        if let Some(id) = id_from_value(raw, cfg) {
            // Deprecated legacy branch: the canonical clustering key is now
            // `parent:` (the migration derives it from the foreign
            // `work_item_id:`). Retained this release for un-migrated repos;
            // its removal is the story-0070 follow-on contract story.
            warn!(
                entry_path = %entry.path.display(),
                "cluster key resolved via the legacy `work_item_id:` branch; \
                 migrate to `parent:` (deprecated fallback — story 0070 follow-on)",
            );
            return Some(id);
        }
    }
    None
}

/// Normalise a `parent/work_item_id` frontmatter value to a canonical
/// work-item id. Handles three shapes routed through `parse_typed_ref`:
/// - `TypedRef::WorkItem(id)` — typed canonical form
/// - `TypedRef::Path(p)` — legacy path shape, e.g. `meta/work/0033-foo.md`
/// - bare numeric/`PROJ-NNNN` token — routed via `canonicalise_one_id`
fn id_from_value(raw: &str, cfg: &WorkItemConfig) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    match parse_typed_ref(s) {
        Some(TypedRef::WorkItem(id)) => canonicalise_one_id(&id, cfg),
        Some(TypedRef::Path(p)) => {
            let stem = p.file_stem()?.to_str()?;
            cfg.extract_id(&format!("{stem}.md"))
        }
        Some(_) => None,
        None => canonicalise_one_id(s, cfg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::entry_for_test;
    use serde_json::json;

    fn empty_entries() -> HashMap<PathBuf, &'static IndexEntry> {
        HashMap::new()
    }

    fn project_root() -> PathBuf {
        PathBuf::from("/repo")
    }

    #[test]
    fn work_item_returns_its_own_work_item_id() {
        let cfg = WorkItemConfig::default();
        let mut wi = entry_for_test(DocTypeKey::WorkItems, "pipeline", 1, "WI");
        wi.work_item_id = Some("0040".into());
        let resolved = resolve_cluster_key(
            &wi,
            &empty_entries(),
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );
        assert_eq!(resolved.as_deref(), Some("0040"));
    }

    #[test]
    fn plan_with_typed_work_item_parent_resolves() {
        let cfg = WorkItemConfig::default();
        let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 1, "P");
        plan.frontmatter = json!({ "parent": "work-item:0042" });
        let resolved = resolve_cluster_key(
            &plan,
            &empty_entries(),
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );
        assert_eq!(resolved.as_deref(), Some("0042"));
    }

    #[test]
    fn plan_with_bare_parent_id_resolves() {
        let cfg = WorkItemConfig::default();
        let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 1, "P");
        plan.frontmatter = json!({ "parent": "0042" });
        let resolved = resolve_cluster_key(
            &plan,
            &empty_entries(),
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );
        assert_eq!(resolved.as_deref(), Some("0042"));
    }

    #[test]
    fn parent_typed_form_resolves_same_as_bare_id() {
        // Phase 6's producer change (refine-work-item and create-plan now emit
        // the typed "work-item:NNNN" form) relies on the typed and bare-id
        // shapes resolving to the SAME canonical cluster id. The two shapes are
        // covered separately above; this asserts their equivalence directly so
        // a future divergence is caught.
        let cfg = WorkItemConfig::default();

        let mut typed = entry_for_test(DocTypeKey::Plans, "pipeline", 1, "P");
        typed.frontmatter = json!({ "parent": "work-item:0042" });
        let typed_key = resolve_cluster_key(
            &typed,
            &empty_entries(),
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );

        let mut bare = entry_for_test(DocTypeKey::Plans, "pipeline", 1, "P");
        bare.frontmatter = json!({ "parent": "0042" });
        let bare_key = resolve_cluster_key(
            &bare,
            &empty_entries(),
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );

        assert_eq!(typed_key, bare_key);
        assert_eq!(typed_key.as_deref(), Some("0042"));
    }

    #[test]
    fn plan_with_work_item_id_frontmatter_resolves() {
        let cfg = WorkItemConfig::default();
        let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 1, "P");
        plan.frontmatter = json!({ "work_item_id": "0042" });
        let resolved = resolve_cluster_key(
            &plan,
            &empty_entries(),
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );
        assert_eq!(resolved.as_deref(), Some("0042"));
    }

    #[test]
    fn legacy_work_item_id_branch_emits_deprecation_warning() {
        // Story 0070: the retained legacy `work_item_id:` clustering branch
        // emits a deprecation warning when it resolves (the canonical key is
        // now `parent:`). Capture synchronously on the test thread.
        let body = crate::log::test_support::capture_logs(|| {
            let cfg = WorkItemConfig::default();
            let mut plan =
                entry_for_test(DocTypeKey::Plans, "pipeline", 1, "P");
            plan.frontmatter = json!({ "work_item_id": "0042" });
            let resolved = resolve_cluster_key(
                &plan,
                &empty_entries(),
                &HashMap::new(),
                &HashMap::new(),
                &project_root(),
                &cfg,
            );
            assert_eq!(resolved.as_deref(), Some("0042"));
        });
        assert!(
            body.contains("legacy `work_item_id:` branch"),
            "expected cluster-key legacy-branch deprecation warning, got: {body}"
        );
    }

    #[test]
    fn plan_with_path_shape_work_item_id_resolves() {
        let cfg = WorkItemConfig::default();
        let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 1, "P");
        plan.frontmatter = json!({ "work_item_id": "meta/work/0033-foo.md" });
        let resolved = resolve_cluster_key(
            &plan,
            &empty_entries(),
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );
        assert_eq!(resolved.as_deref(), Some("0033"));
    }

    #[test]
    fn plan_with_empty_work_item_id_and_no_parent_resolves_none() {
        let cfg = WorkItemConfig::default();
        let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 1, "P");
        plan.frontmatter = json!({ "work_item_id": "" });
        let resolved = resolve_cluster_key(
            &plan,
            &empty_entries(),
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );
        assert_eq!(resolved, None);
    }

    #[test]
    fn plan_review_target_path_resolves_transitively_via_plan() {
        let cfg = WorkItemConfig::default();
        let plan_path =
            PathBuf::from("/repo/meta/plans/2026-05-31-0040-pipeline.md");
        let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 1, "P");
        plan.path = plan_path.clone();
        plan.frontmatter = json!({ "parent": "work-item:0040" });
        let mut entries: HashMap<PathBuf, &IndexEntry> = HashMap::new();
        entries.insert(plan_path.clone(), &plan);
        let mut review = entry_for_test(DocTypeKey::PlanReviews, "rev", 2, "R");
        review.frontmatter =
            json!({ "target": "meta/plans/2026-05-31-0040-pipeline.md" });
        let resolved = resolve_cluster_key(
            &review,
            &entries,
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );
        assert_eq!(resolved.as_deref(), Some("0040"));
    }

    #[test]
    fn plan_review_target_plan_id_resolves_transitively_via_plans_by_id() {
        let cfg = WorkItemConfig::default();
        let plan_path =
            PathBuf::from("/repo/meta/plans/2026-05-31-0040-pipeline.md");
        let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 1, "P");
        plan.path = plan_path.clone();
        plan.frontmatter = json!({ "parent": "work-item:0040" });
        let mut entries: HashMap<PathBuf, &IndexEntry> = HashMap::new();
        entries.insert(plan_path.clone(), &plan);
        let mut plans_by_id = HashMap::new();
        plans_by_id
            .insert("2026-05-31-0040-pipeline".to_string(), plan_path.clone());
        let mut review = entry_for_test(DocTypeKey::PlanReviews, "rev", 2, "R");
        review.frontmatter =
            json!({ "target": "plan:2026-05-31-0040-pipeline" });
        let resolved = resolve_cluster_key(
            &review,
            &entries,
            &HashMap::new(),
            &plans_by_id,
            &project_root(),
            &cfg,
        );
        assert_eq!(resolved.as_deref(), Some("0040"));
    }

    #[test]
    fn work_item_review_target_path_resolves_to_work_item_id() {
        let cfg = WorkItemConfig::default();
        let wi_path =
            PathBuf::from("/repo/meta/work/0033-design-token-system.md");
        let mut wi = entry_for_test(
            DocTypeKey::WorkItems,
            "design-token-system",
            1,
            "WI",
        );
        wi.work_item_id = Some("0033".into());
        wi.path = wi_path.clone();
        let mut entries: HashMap<PathBuf, &IndexEntry> = HashMap::new();
        entries.insert(wi_path.clone(), &wi);
        let mut review =
            entry_for_test(DocTypeKey::WorkItemReviews, "rev", 2, "R");
        review.frontmatter =
            json!({ "target": "meta/work/0033-design-token-system.md" });
        let resolved = resolve_cluster_key(
            &review,
            &entries,
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );
        assert_eq!(resolved.as_deref(), Some("0033"));
    }

    #[test]
    fn work_item_review_typed_work_item_target_short_circuits() {
        let cfg = WorkItemConfig::default();
        let mut review =
            entry_for_test(DocTypeKey::WorkItemReviews, "rev", 2, "R");
        review.frontmatter = json!({ "target": "work-item:0033" });
        let resolved = resolve_cluster_key(
            &review,
            &empty_entries(),
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );
        assert_eq!(resolved.as_deref(), Some("0033"));
    }

    #[test]
    fn validation_target_plan_resolves_transitively_two_hop() {
        let cfg = WorkItemConfig::default();
        let plan_path =
            PathBuf::from("/repo/meta/plans/2026-05-31-0040-pipeline.md");
        let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 1, "P");
        plan.path = plan_path.clone();
        plan.frontmatter = json!({ "parent": "work-item:0040" });
        let mut entries: HashMap<PathBuf, &IndexEntry> = HashMap::new();
        entries.insert(plan_path.clone(), &plan);
        let mut val = entry_for_test(DocTypeKey::Validations, "val", 2, "V");
        val.frontmatter =
            json!({ "target": "meta/plans/2026-05-31-0040-pipeline.md" });
        let resolved = resolve_cluster_key(
            &val,
            &entries,
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );
        assert_eq!(resolved.as_deref(), Some("0040"));
    }

    #[test]
    fn research_without_parent_resolves_none() {
        let cfg = WorkItemConfig::default();
        let r = entry_for_test(DocTypeKey::Research, "orphan", 1, "R");
        let resolved = resolve_cluster_key(
            &r,
            &empty_entries(),
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );
        assert_eq!(resolved, None);
    }

    #[test]
    fn orphan_by_design_types_resolve_none() {
        let cfg = WorkItemConfig::default();
        for kind in [
            DocTypeKey::Notes,
            DocTypeKey::DesignGaps,
            DocTypeKey::DesignInventories,
            DocTypeKey::Decisions,
            DocTypeKey::Templates,
        ] {
            let e = entry_for_test(kind, "x", 1, "T");
            let resolved = resolve_cluster_key(
                &e,
                &empty_entries(),
                &HashMap::new(),
                &HashMap::new(),
                &project_root(),
                &cfg,
            );
            assert_eq!(resolved, None, "{kind:?}");
        }
    }

    #[test]
    fn cycle_between_two_plan_reviews_does_not_recurse_forever() {
        // Two reviews each pointing at the other via path targets. The
        // depth limit must cut the walk; we should observe None without
        // stack overflow.
        let cfg = WorkItemConfig::default();
        let path_a = PathBuf::from("/repo/meta/reviews/plans/a.md");
        let path_b = PathBuf::from("/repo/meta/reviews/plans/b.md");
        let mut a = entry_for_test(DocTypeKey::PlanReviews, "a", 1, "A");
        a.path = path_a.clone();
        a.frontmatter = json!({ "target": "meta/reviews/plans/b.md" });
        let mut b = entry_for_test(DocTypeKey::PlanReviews, "b", 1, "B");
        b.path = path_b.clone();
        b.frontmatter = json!({ "target": "meta/reviews/plans/a.md" });
        let mut entries: HashMap<PathBuf, &IndexEntry> = HashMap::new();
        entries.insert(path_a.clone(), &a);
        entries.insert(path_b.clone(), &b);
        let resolved = resolve_cluster_key(
            &a,
            &entries,
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );
        assert_eq!(resolved, None);
    }

    #[test]
    fn plan_parent_resolves_to_canonical_id_even_when_work_item_missing() {
        let cfg = WorkItemConfig::default();
        let mut plan = entry_for_test(DocTypeKey::Plans, "pipeline", 1, "P");
        plan.frontmatter = json!({ "parent": "work-item:0099" });
        // Empty work_item_by_id — cluster_key is a logical id, not a path
        // lookup result.
        let resolved = resolve_cluster_key(
            &plan,
            &empty_entries(),
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );
        assert_eq!(resolved.as_deref(), Some("0099"));
    }

    #[test]
    fn project_prefix_canonicalisation_under_numeric_pattern_pads() {
        let cfg = WorkItemConfig::default();
        let mut plan = entry_for_test(DocTypeKey::Plans, "p", 1, "P");
        plan.frontmatter = json!({ "parent": "42" });
        let resolved = resolve_cluster_key(
            &plan,
            &empty_entries(),
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );
        assert_eq!(resolved.as_deref(), Some("0042"));
    }

    #[test]
    fn project_prefix_canonicalisation_under_project_pattern_prefixes() {
        let cfg = WorkItemConfig::with_pattern_for_test("PROJ", 4);
        let mut plan_a = entry_for_test(DocTypeKey::Plans, "p", 1, "P");
        plan_a.frontmatter = json!({ "parent": "42" });
        let resolved_a = resolve_cluster_key(
            &plan_a,
            &empty_entries(),
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );
        assert_eq!(resolved_a.as_deref(), Some("PROJ-0042"));

        let mut plan_b = entry_for_test(DocTypeKey::Plans, "p", 1, "P");
        plan_b.frontmatter = json!({ "parent": "PROJ-0042" });
        let resolved_b = resolve_cluster_key(
            &plan_b,
            &empty_entries(),
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );
        assert_eq!(resolved_b.as_deref(), Some("PROJ-0042"));
    }

    #[test]
    fn malformed_empty_typed_parent_resolves_none() {
        let cfg = WorkItemConfig::default();
        let mut plan = entry_for_test(DocTypeKey::Plans, "p", 1, "P");
        plan.frontmatter = json!({ "parent": "work-item:" });
        let resolved = resolve_cluster_key(
            &plan,
            &empty_entries(),
            &HashMap::new(),
            &HashMap::new(),
            &project_root(),
            &cfg,
        );
        assert_eq!(resolved, None);
    }
}
