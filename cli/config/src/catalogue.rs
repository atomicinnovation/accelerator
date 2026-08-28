//! The recognised-key catalogue and its defaults, modelled as domain data.

use crate::node::Scalar;
use crate::service::Value;

pub const AGENT_PREFIX: &str = "accelerator:";

/// A catalogue default: a scalar or a sequence of scalars. Each maps directly
/// to the [`Value`] shape the parser yields for the corresponding present value.
pub enum Default {
    Scalar(&'static str),
    Seq(&'static [&'static str]),
}

impl Default {
    fn to_value(&self) -> Value {
        match self {
            Self::Scalar(text) => {
                Value::Scalar(Scalar::String((*text).to_owned()))
            }
            Self::Seq(items) => Value::Sequence(
                items
                    .iter()
                    .map(|item| Scalar::String((*item).to_owned()))
                    .collect(),
            ),
        }
    }
}

pub const PATH_KEYS: &[(&str, Default)] = &[
    ("paths.plans", Default::Scalar("meta/plans")),
    (
        "paths.research_codebase",
        Default::Scalar("meta/research/codebase"),
    ),
    ("paths.decisions", Default::Scalar("meta/decisions")),
    ("paths.prs", Default::Scalar("meta/prs")),
    ("paths.validations", Default::Scalar("meta/validations")),
    ("paths.review_plans", Default::Scalar("meta/reviews/plans")),
    ("paths.review_prs", Default::Scalar("meta/reviews/prs")),
    ("paths.review_work", Default::Scalar("meta/reviews/work")),
    ("paths.templates", Default::Scalar(".accelerator/templates")),
    ("paths.work", Default::Scalar("meta/work")),
    ("paths.notes", Default::Scalar("meta/notes")),
    ("paths.tmp", Default::Scalar(".accelerator/tmp")),
    (
        "paths.integrations",
        Default::Scalar(".accelerator/state/integrations"),
    ),
    (
        "paths.research_design_inventories",
        Default::Scalar("meta/research/design-inventories"),
    ),
    (
        "paths.research_design_gaps",
        Default::Scalar("meta/research/design-gaps"),
    ),
    ("paths.global", Default::Scalar("meta/global")),
    (
        "paths.research_issues",
        Default::Scalar("meta/research/issues"),
    ),
];

pub const DOC_TYPES: &[(&str, &str)] = &[
    ("work-item", "work"),
    ("plan", "plans"),
    ("plan-validation", "validations"),
    ("pr-description", "prs"),
    ("adr", "decisions"),
    ("codebase-research", "research_codebase"),
    ("issue-research", "research_issues"),
    ("design-inventory", "research_design_inventories"),
    ("design-gap", "research_design_gaps"),
    ("plan-review", "review_plans"),
    ("work-item-review", "review_work"),
    ("pr-review", "review_prs"),
    ("note", "notes"),
];

pub const TEMPLATE_KEYS: &[&str] = &[
    "templates.plan",
    "templates.codebase-research",
    "templates.adr",
    "templates.validation",
    "templates.pr-description",
    "templates.work-item",
    "templates.rca",
    "templates.design-inventory",
    "templates.design-gap",
    "templates.plan-review",
    "templates.work-item-review",
    "templates.pr-review",
    "templates.note",
];

pub const WORK_KEYS: &[(&str, Default)] = &[
    ("work.integration", Default::Scalar("")),
    ("work.id_pattern", Default::Scalar("{number:04d}")),
    ("work.default_project_code", Default::Scalar("")),
];

/// The non-empty values `work.integration` accepts; empty (unset) is always
/// permitted. A `work` read of any other value is a fail-closed refusal.
pub const WORK_INTEGRATION_VALUES: &[&str] =
    &["jira", "linear", "trello", "github-issues"];

/// Whether `value` is an accepted `work.integration`: empty (unset) is always
/// permitted, else membership in [`WORK_INTEGRATION_VALUES`].
#[must_use]
pub fn is_valid_work_integration(value: &str) -> bool {
    value.is_empty() || WORK_INTEGRATION_VALUES.contains(&value)
}

/// Integration and tool keys read ad-hoc by their own consumers.
///
/// They carry no catalogue default — an unset key means the consumer's own
/// default applies — so `dump` surfaces them by presence only.
pub const EXTRA_KEYS: &[&str] = &[
    "jira.allowed_sites",
    "jira.site",
    "jira.email",
    "jira.token",
    "jira.token_cmd",
    "linear.team_id",
    "linear.token",
    "linear.token_cmd",
    "github.token",
    "github.token_cmd",
    "visualiser.editor",
    "visualiser.editor_project",
    "visualiser.binary",
    "design.browser_path",
];

pub const REVIEW_KEYS: &[(&str, Default)] = &[
    ("review.max_inline_comments", Default::Scalar("10")),
    ("review.min_lenses", Default::Scalar("4")),
    ("review.max_lenses", Default::Scalar("8")),
    ("review.dedup_proximity", Default::Scalar("3")),
    (
        "review.core_lenses",
        Default::Seq(&[
            "architecture",
            "code-quality",
            "test-coverage",
            "correctness",
        ]),
    ),
    ("review.disabled_lenses", Default::Seq(&[])),
    (
        "review.pr_request_changes_severity",
        Default::Scalar("critical"),
    ),
    ("review.plan_revise_severity", Default::Scalar("critical")),
    ("review.plan_revise_major_count", Default::Scalar("3")),
    (
        "review.work_item_revise_severity",
        Default::Scalar("critical"),
    ),
    ("review.work_item_revise_major_count", Default::Scalar("2")),
];

/// Built-in review lens names for code reviews (pr and plan modes).
pub const BUILTIN_CODE_LENSES: &[&str] = &[
    "architecture",
    "code-quality",
    "compatibility",
    "correctness",
    "database",
    "documentation",
    "performance",
    "portability",
    "safety",
    "security",
    "standards",
    "test-coverage",
    "usability",
];

/// Built-in review lens names for work-item reviews.
pub const BUILTIN_WORK_ITEM_LENSES: &[&str] = &[
    "clarity",
    "completeness",
    "dependency",
    "scope",
    "testability",
];

pub const AGENT_KEYS: &[&str] = &[
    "reviewer",
    "browser-analyser",
    "browser-locator",
    "codebase-locator",
    "codebase-analyser",
    "codebase-pattern-finder",
    "documents-locator",
    "documents-analyser",
    "web-search-researcher",
];

/// Visualiser keys that carry a catalogue default.
///
/// The remaining visualiser keys (`editor`, `editor_project`, `binary`) are
/// absent-means-disabled and carry no default. The visualiser server keeps a
/// matching runtime fallback in its own crate (`server/src/config.rs`) because
/// it cannot depend on this one; this catalogue is the authoritative
/// declaration.
pub const VISUALISER_KEYS: &[(&str, Default)] = &[
    (
        "visualiser.kanban_columns",
        Default::Seq(&[
            "draft",
            "ready",
            "in-progress",
            "review",
            "done",
            "blocked",
            "abandoned",
        ]),
    ),
    ("visualiser.idle_timeout", Default::Scalar("8h")),
];

/// Resolves a recognised key to its catalogue default, applying [`AGENT_PREFIX`]
/// for agent keys. Returns `None` for an unrecognised key or a template key
/// (which carries no default).
#[must_use]
pub fn default_for(key: &str) -> Option<Value> {
    for group in [PATH_KEYS, WORK_KEYS, REVIEW_KEYS, VISUALISER_KEYS] {
        if let Some((_, default)) = group.iter().find(|(name, _)| *name == key)
        {
            return Some(default.to_value());
        }
    }
    if let Some(name) = key.strip_prefix("agents.") {
        if AGENT_KEYS.contains(&name) {
            return Some(Value::Scalar(Scalar::String(format!(
                "{AGENT_PREFIX}{name}"
            ))));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        default_for, AGENT_KEYS, DOC_TYPES, EXTRA_KEYS, PATH_KEYS, REVIEW_KEYS,
        TEMPLATE_KEYS, VISUALISER_KEYS, WORK_KEYS,
    };
    use crate::node::Scalar;
    use crate::service::Value;

    #[test]
    fn the_catalogue_holds_fifty_five_keys_across_six_groups() {
        let count = PATH_KEYS.len()
            + TEMPLATE_KEYS.len()
            + WORK_KEYS.len()
            + REVIEW_KEYS.len()
            + AGENT_KEYS.len()
            + VISUALISER_KEYS.len();
        assert_eq!(count, 55);
        assert_eq!(DOC_TYPES.len(), 13);
    }

    #[test]
    fn default_for_a_scalar_key_is_a_typed_scalar() {
        assert_eq!(
            default_for("paths.work"),
            Some(Value::Scalar(Scalar::String("meta/work".to_owned())))
        );
    }

    #[test]
    fn default_for_an_agent_key_is_prefixed() {
        assert_eq!(
            default_for("agents.reviewer"),
            Some(Value::Scalar(Scalar::String(
                "accelerator:reviewer".to_owned()
            )))
        );
    }

    #[test]
    fn default_for_an_array_key_is_a_typed_sequence() {
        assert_eq!(
            default_for("review.core_lenses"),
            Some(Value::Sequence(vec![
                Scalar::String("architecture".to_owned()),
                Scalar::String("code-quality".to_owned()),
                Scalar::String("test-coverage".to_owned()),
                Scalar::String("correctness".to_owned()),
            ]))
        );
        assert_eq!(
            default_for("review.disabled_lenses"),
            Some(Value::Sequence(Vec::new()))
        );
    }

    #[test]
    fn default_for_a_template_key_is_none() {
        assert_eq!(default_for("templates.plan"), None);
    }

    #[test]
    fn work_integration_accepts_empty_and_members_and_rejects_others() {
        assert!(super::is_valid_work_integration(""));
        for value in super::WORK_INTEGRATION_VALUES {
            assert!(super::is_valid_work_integration(value), "{value}");
        }
        assert!(!super::is_valid_work_integration("bitbucket"));
        assert!(!super::is_valid_work_integration("Jira"));
    }

    #[test]
    fn default_for_an_unrecognised_key_is_none() {
        assert_eq!(default_for("no.such.key"), None);
    }

    #[test]
    fn extra_keys_declares_the_github_credential_keys() {
        assert!(EXTRA_KEYS.contains(&"github.token"));
        assert!(EXTRA_KEYS.contains(&"github.token_cmd"));
    }

    #[test]
    fn extra_keys_declares_the_design_browser_path_hatch() {
        // Presence-only, no catalogue default — the executor reads it ad-hoc
        // from the personal level.
        assert!(EXTRA_KEYS.contains(&"design.browser_path"));
    }

    #[test]
    fn the_path_defaults_relied_on_as_fallbacks_are_present_and_non_empty() {
        for key in ["paths.tmp", "paths.templates", "paths.integrations"] {
            let default = default_for(key);
            assert!(
                matches!(
                    &default,
                    Some(Value::Scalar(Scalar::String(text)))
                        if !text.is_empty()
                ),
                "{key} must have a non-empty scalar default, got {default:?}"
            );
        }
    }
}
