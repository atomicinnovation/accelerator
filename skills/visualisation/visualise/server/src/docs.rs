use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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
    Templates,
}

impl DocTypeKey {
    pub fn all() -> [DocTypeKey; 13] {
        [
            DocTypeKey::Decisions,
            DocTypeKey::WorkItems,
            DocTypeKey::Plans,
            DocTypeKey::Research,
            DocTypeKey::PlanReviews,
            DocTypeKey::PrReviews,
            DocTypeKey::WorkItemReviews,
            DocTypeKey::Validations,
            DocTypeKey::Notes,
            DocTypeKey::PrDescriptions,
            DocTypeKey::DesignGaps,
            DocTypeKey::DesignInventories,
            DocTypeKey::Templates,
        ]
    }

    pub fn config_path_key(self) -> Option<&'static str> {
        match self {
            DocTypeKey::Decisions => Some("decisions"),
            DocTypeKey::WorkItems => Some("work"),
            DocTypeKey::Plans => Some("plans"),
            DocTypeKey::Research => Some("research_codebase"),
            DocTypeKey::PlanReviews => Some("review_plans"),
            DocTypeKey::PrReviews => Some("review_prs"),
            DocTypeKey::WorkItemReviews => Some("review_work"),
            DocTypeKey::Validations => Some("validations"),
            DocTypeKey::Notes => Some("notes"),
            // Wire token renamed to "pr-descriptions" in work item 0041; on-disk
            // path and config key intentionally retained as "prs" for back-compat
            // with user config files. See plan 2026-05-16-0041.
            DocTypeKey::PrDescriptions => Some("prs"),
            DocTypeKey::DesignGaps => Some("research_design_gaps"),
            DocTypeKey::DesignInventories => Some("research_design_inventories"),
            DocTypeKey::Templates => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DocTypeKey::Decisions => "Decisions",
            DocTypeKey::WorkItems => "Work items",
            DocTypeKey::Plans => "Plans",
            DocTypeKey::Research => "Research",
            DocTypeKey::PlanReviews => "Plan reviews",
            DocTypeKey::PrReviews => "PR reviews",
            DocTypeKey::WorkItemReviews => "Work item reviews",
            DocTypeKey::Validations => "Validations",
            DocTypeKey::Notes => "Notes",
            DocTypeKey::PrDescriptions => "PR descriptions",
            DocTypeKey::DesignGaps => "Design gaps",
            DocTypeKey::DesignInventories => "Design inventories",
            DocTypeKey::Templates => "Templates",
        }
    }

    pub fn in_lifecycle(self) -> bool {
        !matches!(self, DocTypeKey::Templates)
    }

    /// True iff this doc type carries a `target:` frontmatter key per
    /// ADR-0034's type-pair table. Review/validation artifacts declare
    /// their target via `target:`; everything else uses `parent:` /
    /// `work_item_id:` / no linkage.
    pub fn carries_target_frontmatter(self) -> bool {
        matches!(
            self,
            Self::PlanReviews
                | Self::WorkItemReviews
                | Self::PrReviews
                | Self::Validations,
        )
    }

    /// True iff this doc type is part of the work-item lifecycle
    /// pipeline. These types fall back to slug-bucketing when their
    /// typed-linkage walk returns no cluster_key (legacy filename
    /// shapes). Orphan-by-design types return `false` and are kept
    /// in per-path buckets to prevent slug-collision merges.
    pub fn participates_in_lifecycle(self) -> bool {
        matches!(
            self,
            Self::Plans
                | Self::Research
                | Self::WorkItems
                | Self::PlanReviews
                | Self::WorkItemReviews
                | Self::PrReviews
                | Self::PrDescriptions
                | Self::Validations,
        )
    }

    pub fn in_kanban(self) -> bool {
        matches!(self, DocTypeKey::WorkItems)
    }

    pub fn is_virtual(self) -> bool {
        matches!(self, DocTypeKey::Templates)
    }

    /// Some doc types are stored as `<root>/<slug-dir>/<manifest>.md` rather
    /// than flat `<root>/<slug>.md`. This returns the manifest filename to
    /// look for in each direct subdirectory of the doc-type root.
    ///
    /// Design inventories follow this pattern — each inventory is a dated
    /// `<root>/YYYY-MM-DD-HHMMSS-{source-id}/inventory.md` artifact alongside
    /// its `screenshots/` and `assets/`.
    pub fn nested_manifest_filename(self) -> Option<&'static str> {
        match self {
            DocTypeKey::DesignInventories => Some("inventory.md"),
            _ => None,
        }
    }

    /// Returns the kebab-case wire token for this variant. Pinned by the
    /// per-variant `wire_str_round_trips_for_every_variant` test below.
    pub fn wire_str(self) -> &'static str {
        match self {
            DocTypeKey::Decisions => "decisions",
            DocTypeKey::WorkItems => "work-items",
            DocTypeKey::Plans => "plans",
            DocTypeKey::Research => "research",
            DocTypeKey::PlanReviews => "plan-reviews",
            DocTypeKey::PrReviews => "pr-reviews",
            DocTypeKey::WorkItemReviews => "work-item-reviews",
            DocTypeKey::Validations => "validations",
            DocTypeKey::Notes => "notes",
            DocTypeKey::PrDescriptions => "pr-descriptions",
            DocTypeKey::DesignGaps => "design-gaps",
            DocTypeKey::DesignInventories => "design-inventories",
            DocTypeKey::Templates => "templates",
        }
    }

    /// Parses a kebab-case wire token back to its variant. Returns `None`
    /// for unknown strings (no panic, no Err — silent drop is the documented
    /// contract for `parse_selection_query` consumers).
    pub fn from_wire_str(s: &str) -> Option<Self> {
        Self::all().iter().copied().find(|dt| dt.wire_str() == s)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocType {
    pub key: DocTypeKey,
    pub label: String,
    pub dir_path: Option<PathBuf>,
    pub in_lifecycle: bool,
    pub in_kanban: bool,
    pub r#virtual: bool,
    /// Number of indexed entries of this doc type as of the API call.
    ///
    /// On the JSON wire, this field is always populated by the
    /// `api::types::types` handler from the live indexer state. Templates
    /// is excluded from the index and so observes `count = 0` via
    /// `unwrap_or(0)` in the handler.
    ///
    /// In-process, `describe_types` constructs `DocType` values with
    /// `count: 0` as a placeholder — the API handler MUST overwrite this
    /// before serialisation. A non-handler consumer of `describe_types`
    /// (e.g., a future CLI introspector) would observe the placeholder
    /// directly and SHOULD NOT trust this field; consider splitting the
    /// type if a second consumer appears.
    pub count: usize,
}

pub fn describe_types(cfg: &crate::config::Config) -> Vec<DocType> {
    let mut out = Vec::with_capacity(DocTypeKey::all().len());
    for key in DocTypeKey::all() {
        let dir_path = key
            .config_path_key()
            .and_then(|k| cfg.doc_paths.get(k).cloned());
        out.push(DocType {
            key,
            label: key.label().to_string(),
            dir_path,
            in_lifecycle: key.in_lifecycle(),
            in_kanban: key.in_kanban(),
            r#virtual: key.is_virtual(),
            count: 0,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kebab_case_round_trip_covers_every_variant() {
        let pairs = [
            (DocTypeKey::Decisions, "decisions"),
            (DocTypeKey::WorkItems, "work-items"),
            (DocTypeKey::Plans, "plans"),
            (DocTypeKey::Research, "research"),
            (DocTypeKey::PlanReviews, "plan-reviews"),
            (DocTypeKey::PrReviews, "pr-reviews"),
            (DocTypeKey::WorkItemReviews, "work-item-reviews"),
            (DocTypeKey::Validations, "validations"),
            (DocTypeKey::Notes, "notes"),
            (DocTypeKey::PrDescriptions, "pr-descriptions"),
            (DocTypeKey::DesignGaps, "design-gaps"),
            (DocTypeKey::DesignInventories, "design-inventories"),
            (DocTypeKey::Templates, "templates"),
        ];
        for (variant, wire) in pairs {
            let ser = serde_json::to_string(&variant).unwrap();
            assert_eq!(ser, format!("\"{wire}\""));
            let de: DocTypeKey = serde_json::from_str(&ser).unwrap();
            assert_eq!(de, variant);
        }
    }

    #[test]
    fn all_returns_every_variant_exactly_once() {
        let mut v = DocTypeKey::all().to_vec();
        v.sort_by_key(|k| k.label().to_string());
        v.dedup();
        assert_eq!(
            v.len(),
            13,
            "DocTypeKey::all must return 13 distinct variants"
        );
    }

    #[test]
    fn wire_str_round_trips_for_every_variant() {
        for variant in DocTypeKey::all() {
            assert_eq!(
                DocTypeKey::from_wire_str(variant.wire_str()),
                Some(variant),
            );
        }
        assert_eq!(DocTypeKey::from_wire_str("bogus"), None);
    }

    #[test]
    fn wire_str_matches_serde_serialisation_for_every_variant() {
        for variant in DocTypeKey::all() {
            let ser = serde_json::to_string(&variant).unwrap();
            assert_eq!(ser, format!("\"{}\"", variant.wire_str()));
        }
    }

    #[test]
    fn work_item_reviews_serialises_to_kebab_case_wire_form() {
        let v = DocTypeKey::WorkItemReviews;
        assert_eq!(serde_json::to_string(&v).unwrap(), "\"work-item-reviews\"");
    }

    #[test]
    fn work_item_reviews_uses_review_work_config_path_key() {
        assert_eq!(DocTypeKey::WorkItemReviews.config_path_key(), Some("review_work"));
    }

    #[test]
    fn work_item_reviews_appears_in_all_and_in_lifecycle_only() {
        assert!(DocTypeKey::all().contains(&DocTypeKey::WorkItemReviews));
        assert!(DocTypeKey::WorkItemReviews.in_lifecycle());
        assert!(!DocTypeKey::WorkItemReviews.in_kanban());
        assert!(!DocTypeKey::WorkItemReviews.is_virtual());
    }

    #[test]
    fn doc_type_key_all_returns_thirteen_variants() {
        assert_eq!(DocTypeKey::all().len(), 13);
    }

    #[test]
    fn carries_target_frontmatter_covers_only_review_and_validation_types() {
        for k in DocTypeKey::all() {
            let expected = matches!(
                k,
                DocTypeKey::PlanReviews
                    | DocTypeKey::WorkItemReviews
                    | DocTypeKey::PrReviews
                    | DocTypeKey::Validations,
            );
            assert_eq!(k.carries_target_frontmatter(), expected, "{k:?}");
        }
    }

    #[test]
    fn participates_in_lifecycle_excludes_orphan_and_template_types() {
        for k in DocTypeKey::all() {
            let expected = matches!(
                k,
                DocTypeKey::Plans
                    | DocTypeKey::Research
                    | DocTypeKey::WorkItems
                    | DocTypeKey::PlanReviews
                    | DocTypeKey::WorkItemReviews
                    | DocTypeKey::PrReviews
                    | DocTypeKey::PrDescriptions
                    | DocTypeKey::Validations,
            );
            assert_eq!(k.participates_in_lifecycle(), expected, "{k:?}");
        }
    }

    #[test]
    fn templates_is_virtual_and_out_of_lifecycle() {
        assert!(DocTypeKey::Templates.is_virtual());
        assert!(!DocTypeKey::Templates.in_lifecycle());
        assert!(!DocTypeKey::Templates.in_kanban());
    }

    #[test]
    fn work_items_is_the_only_kanban_type() {
        for k in DocTypeKey::all() {
            assert_eq!(
                k.in_kanban(),
                matches!(k, DocTypeKey::WorkItems),
                "in_kanban mismatch for {k:?}",
            );
        }
    }

    #[test]
    fn describe_types_populates_dir_paths_from_config() {
        let mut doc_paths = std::collections::HashMap::new();
        doc_paths.insert("decisions".into(), PathBuf::from("/abs/decisions"));
        doc_paths.insert("review_plans".into(), PathBuf::from("/abs/reviews/plans"));

        let cfg = crate::config::Config {
            plugin_root: "/p".into(),
            plugin_version: "test".into(),
            project_root: "/p".into(),
            tmp_path: "/t".into(),
            host: "127.0.0.1".into(),
            owner_pid: 0,
            owner_start_time: None,
            log_path: "/l".into(),
            doc_paths,
            templates: Default::default(),
            work_item: None,
            kanban_columns: None,
            idle_timeout: None,
            editor: None,
            editor_project: None,
        };

        let types = describe_types(&cfg);
        assert_eq!(types.len(), 13);
        let decisions = types
            .iter()
            .find(|t| t.key == DocTypeKey::Decisions)
            .unwrap();
        assert_eq!(
            decisions.dir_path.as_deref(),
            Some(std::path::Path::new("/abs/decisions"))
        );
        let plan_reviews = types
            .iter()
            .find(|t| t.key == DocTypeKey::PlanReviews)
            .unwrap();
        assert_eq!(
            plan_reviews.dir_path.as_deref(),
            Some(std::path::Path::new("/abs/reviews/plans"))
        );
        let templates = types
            .iter()
            .find(|t| t.key == DocTypeKey::Templates)
            .unwrap();
        assert!(templates.dir_path.is_none());
        assert!(templates.r#virtual);
    }

    #[test]
    fn virtual_flag_always_serialised_in_json() {
        let cfg = crate::config::Config {
            plugin_root: "/p".into(),
            plugin_version: "test".into(),
            project_root: "/p".into(),
            tmp_path: "/t".into(),
            host: "127.0.0.1".into(),
            owner_pid: 0,
            owner_start_time: None,
            log_path: "/l".into(),
            doc_paths: Default::default(),
            templates: Default::default(),
            work_item: None,
            kanban_columns: None,
            idle_timeout: None,
            editor: None,
            editor_project: None,
        };
        let types = describe_types(&cfg);
        let decisions = types
            .iter()
            .find(|t| t.key == DocTypeKey::Decisions)
            .unwrap();
        let json = serde_json::to_value(decisions).unwrap();
        assert_eq!(
            json.get("virtual"),
            Some(&serde_json::Value::Bool(false)),
            "virtual must always be emitted, even when false"
        );
        let templates = types
            .iter()
            .find(|t| t.key == DocTypeKey::Templates)
            .unwrap();
        let json = serde_json::to_value(templates).unwrap();
        assert_eq!(json.get("virtual"), Some(&serde_json::Value::Bool(true)));
    }
}
