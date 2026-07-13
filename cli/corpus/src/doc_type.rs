//! The document-type fact: the enum, its per-variant predicates, and the pure
//! path-inference matcher.

use std::path::Path;
use std::path::PathBuf;

/// A meta document type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocTypeKey {
    Decisions,
    WorkItems,
    Plans,
    Research,
    PlanReviews,
    PrReviews,
    WorkItemReviews,
    Validations,
    Notes,
    PrDescriptions,
    DesignGaps,
    DesignInventories,
    RootCauseAnalyses,
    Templates,
}

impl DocTypeKey {
    #[must_use]
    pub const fn all() -> [Self; 14] {
        [
            Self::Decisions,
            Self::WorkItems,
            Self::Plans,
            Self::Research,
            Self::PlanReviews,
            Self::PrReviews,
            Self::WorkItemReviews,
            Self::Validations,
            Self::Notes,
            Self::PrDescriptions,
            Self::DesignGaps,
            Self::DesignInventories,
            Self::RootCauseAnalyses,
            Self::Templates,
        ]
    }

    #[must_use]
    pub const fn config_path_key(self) -> Option<&'static str> {
        match self {
            Self::Decisions => Some("decisions"),
            Self::WorkItems => Some("work"),
            Self::Plans => Some("plans"),
            Self::Research => Some("research_codebase"),
            Self::PlanReviews => Some("review_plans"),
            Self::PrReviews => Some("review_prs"),
            Self::WorkItemReviews => Some("review_work"),
            Self::Validations => Some("validations"),
            Self::Notes => Some("notes"),
            Self::PrDescriptions => Some("prs"),
            Self::DesignGaps => Some("research_design_gaps"),
            Self::DesignInventories => Some("research_design_inventories"),
            Self::RootCauseAnalyses => Some("research_issues"),
            Self::Templates => None,
        }
    }

    /// The typed-linkage vocabulary name, as used in `<type>:<id>` references.
    /// Distinct from [`Self::wire_str`] (the API wire token) and from
    /// [`Self::config_path_key`] (the config key). `None` for a virtual type.
    #[must_use]
    pub const fn linkage_type_name(self) -> Option<&'static str> {
        match self {
            Self::WorkItems => Some("work-item"),
            Self::Plans => Some("plan"),
            Self::Validations => Some("plan-validation"),
            Self::PrDescriptions => Some("pr-description"),
            Self::Decisions => Some("adr"),
            Self::Research => Some("codebase-research"),
            Self::RootCauseAnalyses => Some("issue-research"),
            Self::DesignInventories => Some("design-inventory"),
            Self::DesignGaps => Some("design-gap"),
            Self::PlanReviews => Some("plan-review"),
            Self::WorkItemReviews => Some("work-item-review"),
            Self::PrReviews => Some("pr-review"),
            Self::Notes => Some("note"),
            Self::Templates => None,
        }
    }

    /// The type whose linkage vocabulary name is `name`.
    #[must_use]
    pub fn from_linkage_type_name(name: &str) -> Option<Self> {
        Self::all()
            .into_iter()
            .find(|kind| kind.linkage_type_name() == Some(name))
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Decisions => "Decisions",
            Self::WorkItems => "Work items",
            Self::Plans => "Plans",
            Self::Research => "Research",
            Self::PlanReviews => "Plan reviews",
            Self::PrReviews => "PR reviews",
            Self::WorkItemReviews => "Work item reviews",
            Self::Validations => "Validations",
            Self::Notes => "Notes",
            Self::PrDescriptions => "PR descriptions",
            Self::DesignGaps => "Design gaps",
            Self::DesignInventories => "Design inventories",
            Self::RootCauseAnalyses => "Root cause analyses",
            Self::Templates => "Templates",
        }
    }

    #[must_use]
    pub const fn in_lifecycle(self) -> bool {
        !matches!(self, Self::Templates | Self::RootCauseAnalyses)
    }

    #[must_use]
    pub const fn carries_target_frontmatter(self) -> bool {
        matches!(
            self,
            Self::PlanReviews
                | Self::WorkItemReviews
                | Self::PrReviews
                | Self::Validations
        )
    }

    #[must_use]
    pub const fn participates_in_lifecycle(self) -> bool {
        matches!(
            self,
            Self::Plans
                | Self::Research
                | Self::WorkItems
                | Self::PlanReviews
                | Self::WorkItemReviews
                | Self::PrReviews
                | Self::PrDescriptions
                | Self::Validations
        )
    }

    #[must_use]
    pub const fn in_kanban(self) -> bool {
        matches!(self, Self::WorkItems)
    }

    #[must_use]
    pub const fn is_virtual(self) -> bool {
        matches!(self, Self::Templates)
    }

    #[must_use]
    pub const fn nested_manifest_filename(self) -> Option<&'static str> {
        match self {
            Self::DesignInventories => Some("inventory.md"),
            _ => None,
        }
    }

    #[must_use]
    pub const fn wire_str(self) -> &'static str {
        match self {
            Self::Decisions => "decisions",
            Self::WorkItems => "work-items",
            Self::Plans => "plans",
            Self::Research => "research",
            Self::PlanReviews => "plan-reviews",
            Self::PrReviews => "pr-reviews",
            Self::WorkItemReviews => "work-item-reviews",
            Self::Validations => "validations",
            Self::Notes => "notes",
            Self::PrDescriptions => "pr-descriptions",
            Self::DesignGaps => "design-gaps",
            Self::DesignInventories => "design-inventories",
            Self::RootCauseAnalyses => "root-cause-analyses",
            Self::Templates => "templates",
        }
    }

    #[must_use]
    pub fn from_wire_str(token: &str) -> Option<Self> {
        Self::all()
            .into_iter()
            .find(|kind| kind.wire_str() == token)
    }
}

/// Infers a document's type from its path, choosing the type whose configured
/// directory is the longest segment-anchored match. An exact-length tie keeps
/// the first entry in `table`.
#[must_use]
pub fn infer(
    path: &Path,
    table: &[(DocTypeKey, PathBuf)],
) -> Option<DocTypeKey> {
    let path = path.to_str()?;
    let mut best: Option<(usize, DocTypeKey)> = None;
    for (kind, dir) in table {
        let Some(dir) = dir.to_str() else {
            continue;
        };
        if dir.is_empty() || !segment_match(path, dir) {
            continue;
        }
        if best.is_none_or(|(best_len, _)| dir.len() > best_len) {
            best = Some((dir.len(), *kind));
        }
    }
    best.map(|(_, kind)| kind)
}

fn segment_match(path: &str, dir: &str) -> bool {
    let mut prefix = String::with_capacity(dir.len() + 1);
    prefix.push_str(dir);
    prefix.push('/');
    if path.starts_with(&prefix) {
        return true;
    }
    let mut embedded = String::with_capacity(dir.len() + 2);
    embedded.push('/');
    embedded.push_str(dir);
    embedded.push('/');
    path.contains(&embedded)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{infer, DocTypeKey};

    #[test]
    fn all_returns_fourteen_distinct_variants() {
        let mut variants = DocTypeKey::all().to_vec();
        variants.sort_by_key(|kind| kind.wire_str());
        variants.dedup();
        assert_eq!(variants.len(), 14);
    }

    #[test]
    fn wire_str_round_trips_for_every_variant() {
        for kind in DocTypeKey::all() {
            assert_eq!(DocTypeKey::from_wire_str(kind.wire_str()), Some(kind));
        }
        assert_eq!(DocTypeKey::from_wire_str("bogus"), None);
    }

    #[test]
    fn templates_is_virtual_and_out_of_lifecycle() {
        assert!(DocTypeKey::Templates.is_virtual());
        assert_eq!(DocTypeKey::Templates.config_path_key(), None);
        assert!(!DocTypeKey::Templates.in_lifecycle());
    }

    #[test]
    fn root_cause_analyses_is_an_out_of_lifecycle_peer() {
        let rca = DocTypeKey::RootCauseAnalyses;
        assert_eq!(rca.config_path_key(), Some("research_issues"));
        assert!(!rca.in_lifecycle());
        assert!(!rca.participates_in_lifecycle());
        assert!(!rca.is_virtual());
    }

    #[test]
    fn work_items_is_the_only_kanban_type() {
        for kind in DocTypeKey::all() {
            assert_eq!(kind.in_kanban(), kind == DocTypeKey::WorkItems);
        }
    }

    #[test]
    fn carries_target_frontmatter_covers_review_and_validation_types() {
        for kind in DocTypeKey::all() {
            let expected = matches!(
                kind,
                DocTypeKey::PlanReviews
                    | DocTypeKey::WorkItemReviews
                    | DocTypeKey::PrReviews
                    | DocTypeKey::Validations
            );
            assert_eq!(kind.carries_target_frontmatter(), expected);
        }
    }

    fn table() -> Vec<(DocTypeKey, PathBuf)> {
        vec![
            (DocTypeKey::Plans, PathBuf::from("meta/plans")),
            (DocTypeKey::PlanReviews, PathBuf::from("meta/reviews/plans")),
            (DocTypeKey::WorkItems, PathBuf::from("meta/work")),
        ]
    }

    #[test]
    fn infer_matches_a_segment_anchored_directory() {
        assert_eq!(
            infer(Path::new("meta/work/0042-foo.md"), &table()),
            Some(DocTypeKey::WorkItems)
        );
    }

    #[test]
    fn infer_prefers_the_longest_configured_directory() {
        let path = Path::new("meta/reviews/plans/2026-01-01-foo-review-1.md");
        assert_eq!(infer(path, &table()), Some(DocTypeKey::PlanReviews));
    }

    #[test]
    fn infer_returns_none_when_no_directory_matches() {
        assert_eq!(infer(Path::new("meta/notes/foo.md"), &table()), None);
    }

    #[test]
    fn infer_requires_a_whole_segment_not_a_prefix() {
        assert_eq!(infer(Path::new("meta/plans-archive/x.md"), &table()), None);
    }
}
