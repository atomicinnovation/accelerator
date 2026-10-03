//! The recognised-key catalogue and its defaults, modelled as domain data.

use crate::error::ConfigError;
use crate::key::Key;
use crate::node::Scalar;
use crate::service::ConfigAccess;
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
    (
        "paths.research_topics",
        Default::Scalar("meta/research/topics"),
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
    ("topic-research", "research_topics"),
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
    "templates.topic-research-manifest",
    "templates.topic-research-brief",
    "templates.topic-research-outline",
    "templates.topic-research-finding",
    "templates.topic-research-synthesis",
];

pub const WORK_KEYS: &[(&str, Default)] = &[
    ("work.integration", Default::Scalar("")),
    ("work.id_pattern", Default::Scalar("{number:04d}")),
    ("work.default_project_code", Default::Scalar("")),
    ("work.key", Default::Scalar("")),
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

/// Who may supply a key's value.
///
/// A consent key holds a value only the user may supply: a team-level value,
/// or one read from a VCS-tracked `config.local.md`, is refused. The path and
/// command kinds add value checks on top of that provenance rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trust {
    Open,
    Consent,
    PathConsent,
    CommandConsent {
        admitted_environment: &'static [&'static str],
    },
}

/// An integration or tool key read ad-hoc by its own consumer.
///
/// `overrides` are the environment variables that override the value, in
/// precedence order. They are distinct from a command key's
/// `admitted_environment`, which is what reaches the command's process.
#[derive(Debug, PartialEq, Eq)]
pub struct ExtraKey {
    pub name: &'static str,
    pub trust: Trust,
    pub overrides: &'static [&'static str],
    pub recovery: Option<&'static str>,
}

impl ExtraKey {
    const fn open(name: &'static str) -> Self {
        Self::overridden(name, Trust::Open, &[])
    }

    const fn overridden(
        name: &'static str,
        trust: Trust,
        overrides: &'static [&'static str],
    ) -> Self {
        Self {
            name,
            trust,
            overrides,
            recovery: None,
        }
    }

    /// The route out of a refusal: `recovery` when the key names one, else
    /// its own first override.
    #[must_use]
    pub fn recovery_hint(&self) -> Option<&'static str> {
        self.recovery.or_else(|| self.overrides.first().copied())
    }
}

/// The variables every command-valued key's process receives when the parent
/// sets them.
pub const BASE_COMMAND_ENVIRONMENT: &[&str] = &[
    "PATH",
    "HOME",
    "TERM",
    "XDG_CONFIG_HOME",
    "XDG_RUNTIME_DIR",
    "DBUS_SESSION_BUS_ADDRESS",
];

const BASE_COMMAND: Trust = Trust::CommandConsent {
    admitted_environment: &[],
};

/// Integration and tool keys read ad-hoc by their own consumers.
///
/// They carry no catalogue default — an unset key means the consumer's own
/// default applies — so `dump` surfaces them by presence only.
pub const EXTRA_KEYS: &[ExtraKey] = &[
    ExtraKey::overridden(
        "jira.allowed_sites",
        Trust::Consent,
        &["ACCELERATOR_JIRA_ALLOWED_SITES"],
    ),
    ExtraKey::open("jira.site"),
    ExtraKey::open("jira.email"),
    ExtraKey::overridden(
        "jira.token",
        Trust::Open,
        &["ACCELERATOR_JIRA_TOKEN"],
    ),
    ExtraKey::overridden(
        "jira.token_cmd",
        BASE_COMMAND,
        &["ACCELERATOR_JIRA_TOKEN_CMD"],
    ),
    ExtraKey::open("jira.project_key"),
    ExtraKey::open("linear.team_id"),
    ExtraKey::open("linear.team_key"),
    ExtraKey::overridden(
        "linear.token",
        Trust::Open,
        &["ACCELERATOR_LINEAR_TOKEN"],
    ),
    ExtraKey::overridden(
        "linear.token_cmd",
        BASE_COMMAND,
        &["ACCELERATOR_LINEAR_TOKEN_CMD"],
    ),
    ExtraKey::overridden(
        "github.token",
        Trust::Open,
        &["GH_TOKEN", "GITHUB_TOKEN"],
    ),
    ExtraKey {
        name: "github.token_cmd",
        trust: Trust::CommandConsent {
            admitted_environment: &["GH_HOST", "GH_CONFIG_DIR"],
        },
        overrides: &[],
        recovery: Some("GH_TOKEN"),
    },
    ExtraKey::overridden(
        "openalex.api_key",
        Trust::Open,
        &["ACCELERATOR_OPENALEX_API_KEY"],
    ),
    ExtraKey::overridden(
        "openalex.api_key_cmd",
        BASE_COMMAND,
        &["ACCELERATOR_OPENALEX_API_KEY_CMD"],
    ),
    ExtraKey::open("visualiser.editor"),
    ExtraKey::open("visualiser.editor_project"),
    ExtraKey::open("visualiser.binary"),
    ExtraKey::overridden(
        "design.browser_path",
        Trust::PathConsent,
        &["ACCELERATOR_DESIGN_BROWSER_PATH"],
    ),
];

/// The catalogue's descriptor for an extra key, or `None` when undeclared.
#[must_use]
pub fn declared(name: &str) -> Option<&'static ExtraKey> {
    EXTRA_KEYS.iter().find(|key| key.name == name)
}

/// Every key only the user may supply.
pub fn consent_keys() -> impl Iterator<Item = &'static ExtraKey> {
    EXTRA_KEYS.iter().filter(|key| key.trust != Trust::Open)
}

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

pub const RESEARCH_KEYS: &[(&str, Default)] = &[
    ("research.topic.breadth", Default::Scalar("8")),
    ("research.topic.depth", Default::Scalar("1")),
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
    "researcher",
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
    for group in [
        PATH_KEYS,
        WORK_KEYS,
        REVIEW_KEYS,
        RESEARCH_KEYS,
        VISUALISER_KEYS,
    ] {
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

/// Resolves `agents.<name>` to its configured override, else to `<name>`
/// under [`AGENT_PREFIX`].
///
/// An explicit-empty override coalesces to the prefixed default, as does a
/// name outside [`AGENT_KEYS`], which carries no catalogue default.
///
/// # Errors
///
/// A [`ConfigError`] when the name is malformed or a config level cannot be
/// read.
pub fn agent_name(
    config: &dyn ConfigAccess,
    name: &str,
) -> Result<String, ConfigError> {
    let key = Key::parse(&format!("agents.{name}"))?;
    Ok(config
        .effective_nonempty(&key, None)?
        .configured_value()
        .unwrap_or_else(|| format!("{AGENT_PREFIX}{name}")))
}

#[cfg(test)]
mod tests {
    use super::{
        consent_keys, declared, default_for, Trust, AGENT_KEYS, DOC_TYPES,
        EXTRA_KEYS, PATH_KEYS, RESEARCH_KEYS, REVIEW_KEYS, TEMPLATE_KEYS,
        VISUALISER_KEYS, WORK_KEYS,
    };
    use crate::node::Scalar;
    use crate::service::Value;

    #[test]
    fn the_catalogue_holds_sixty_five_keys_across_seven_groups() {
        let count = PATH_KEYS.len()
            + TEMPLATE_KEYS.len()
            + WORK_KEYS.len()
            + REVIEW_KEYS.len()
            + RESEARCH_KEYS.len()
            + AGENT_KEYS.len()
            + VISUALISER_KEYS.len();
        assert_eq!(count, 65);
        assert_eq!(DOC_TYPES.len(), 14);
    }

    #[test]
    fn default_for_the_research_knobs_are_typed_scalars() {
        assert_eq!(
            default_for("research.topic.breadth"),
            Some(Value::Scalar(Scalar::String("8".to_owned())))
        );
        assert_eq!(
            default_for("research.topic.depth"),
            Some(Value::Scalar(Scalar::String("1".to_owned())))
        );
    }

    #[test]
    fn work_key_defaults_to_an_empty_scalar() {
        assert_eq!(
            default_for("work.key"),
            Some(Value::Scalar(Scalar::String(String::new())))
        );
    }

    #[test]
    fn extra_keys_declares_the_tracker_scope_keys() {
        assert!(is_extra("jira.project_key"));
        assert!(is_extra("linear.team_key"));
    }

    fn is_extra(name: &str) -> bool {
        EXTRA_KEYS.iter().any(|key| key.name == name)
    }

    #[test]
    fn consent_keys_are_exactly_the_six_user_only_keys() {
        let names: Vec<&str> = consent_keys().map(|key| key.name).collect();
        assert_eq!(
            names,
            [
                "jira.allowed_sites",
                "jira.token_cmd",
                "linear.token_cmd",
                "github.token_cmd",
                "openalex.api_key_cmd",
                "design.browser_path",
            ]
        );
    }

    #[test]
    fn each_consent_key_declares_its_kind() {
        let trust = |name| declared(name).map(|key| key.trust);
        assert_eq!(trust("jira.allowed_sites"), Some(Trust::Consent));
        assert_eq!(trust("design.browser_path"), Some(Trust::PathConsent));
        for name in
            ["jira.token_cmd", "linear.token_cmd", "openalex.api_key_cmd"]
        {
            assert_eq!(
                trust(name),
                Some(Trust::CommandConsent {
                    admitted_environment: &[]
                }),
                "{name} admits only the base environment"
            );
        }
        assert_eq!(
            trust("github.token_cmd"),
            Some(Trust::CommandConsent {
                admitted_environment: &["GH_HOST", "GH_CONFIG_DIR"]
            })
        );
    }

    #[test]
    fn visualiser_editor_is_open() {
        assert_eq!(
            declared("visualiser.editor").map(|key| key.trust),
            Some(Trust::Open)
        );
    }

    #[test]
    fn every_key_pins_its_overrides_and_recovery() {
        let expected: &[(&str, &[&str], Option<&str>)] = &[
            (
                "jira.allowed_sites",
                &["ACCELERATOR_JIRA_ALLOWED_SITES"],
                None,
            ),
            ("jira.site", &[], None),
            ("jira.email", &[], None),
            ("jira.token", &["ACCELERATOR_JIRA_TOKEN"], None),
            ("jira.token_cmd", &["ACCELERATOR_JIRA_TOKEN_CMD"], None),
            ("jira.project_key", &[], None),
            ("linear.team_id", &[], None),
            ("linear.team_key", &[], None),
            ("linear.token", &["ACCELERATOR_LINEAR_TOKEN"], None),
            ("linear.token_cmd", &["ACCELERATOR_LINEAR_TOKEN_CMD"], None),
            ("github.token", &["GH_TOKEN", "GITHUB_TOKEN"], None),
            ("github.token_cmd", &[], Some("GH_TOKEN")),
            ("openalex.api_key", &["ACCELERATOR_OPENALEX_API_KEY"], None),
            (
                "openalex.api_key_cmd",
                &["ACCELERATOR_OPENALEX_API_KEY_CMD"],
                None,
            ),
            ("visualiser.editor", &[], None),
            ("visualiser.editor_project", &[], None),
            ("visualiser.binary", &[], None),
            (
                "design.browser_path",
                &["ACCELERATOR_DESIGN_BROWSER_PATH"],
                None,
            ),
        ];
        let actual: Vec<(&str, &[&str], Option<&str>)> = EXTRA_KEYS
            .iter()
            .map(|key| (key.name, key.overrides, key.recovery))
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn a_recovery_hint_falls_back_to_the_first_override() {
        let hint =
            |name| declared(name).and_then(super::ExtraKey::recovery_hint);
        assert_eq!(hint("jira.token_cmd"), Some("ACCELERATOR_JIRA_TOKEN_CMD"));
        assert_eq!(hint("github.token_cmd"), Some("GH_TOKEN"));
        assert_eq!(hint("jira.site"), None);
    }

    #[test]
    fn an_undeclared_name_has_no_descriptor() {
        assert!(declared("no.such.key").is_none());
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
        assert!(is_extra("github.token"));
        assert!(is_extra("github.token_cmd"));
    }

    #[test]
    fn extra_keys_declares_the_openalex_credential_keys() {
        assert!(is_extra("openalex.api_key"));
        assert!(is_extra("openalex.api_key_cmd"));
    }

    #[test]
    fn extra_keys_declares_the_design_browser_path_hatch() {
        // Presence-only, no catalogue default — the executor reads it ad-hoc
        // from the personal level.
        assert!(is_extra("design.browser_path"));
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
