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

/// Integration and tool keys read ad-hoc by their own consumers.
///
/// They carry no catalogue default — an unset key means the consumer's own
/// default applies — so `dump` surfaces them by presence only. The bash mirror
/// is `EXTRA_KEYS` in `config-defaults.sh`.
pub const EXTRA_KEYS: &[&str] = &[
    "jira.site",
    "jira.email",
    "jira.token",
    "jira.token_cmd",
    "linear.token",
    "linear.token_cmd",
    "visualiser.editor",
    "visualiser.editor_project",
    "visualiser.binary",
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
/// absent-means-disabled and stay in the bash `EXTRA_KEYS` registry with no
/// default. The visualiser server keeps a matching runtime fallback in its own
/// crate (`server/src/config.rs`) because it cannot depend on this one; this
/// catalogue is the authoritative declaration and the bash mirror is
/// drift-tested against it.
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
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::process::Command;

    use super::{
        default_for, Default, AGENT_KEYS, DOC_TYPES, PATH_KEYS, REVIEW_KEYS,
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
    fn default_for_an_unrecognised_key_is_none() {
        assert_eq!(default_for("no.such.key"), None);
    }

    type TestError = Box<dyn std::error::Error>;

    fn scripts_dir() -> Result<PathBuf, TestError> {
        Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../scripts")
            .canonicalize()?)
    }

    fn render_default(default: &Default) -> String {
        match default {
            Default::Scalar(text) => (*text).to_owned(),
            Default::Seq(items) => format!("[{}]", items.join(", ")),
        }
    }

    fn rust_defaults() -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        for group in [PATH_KEYS, WORK_KEYS, REVIEW_KEYS, VISUALISER_KEYS] {
            for (key, default) in group {
                map.insert((*key).to_owned(), render_default(default));
            }
        }
        for name in AGENT_KEYS {
            map.insert(format!("agents.{name}"), format!("accelerator:{name}"));
        }
        map
    }

    fn bash_available() -> bool {
        Command::new("bash")
            .arg("-c")
            .arg("exit 0")
            .status()
            .is_ok_and(|status| status.success())
    }

    const EXTRACT: &str = r#"
set -euo pipefail
scripts="$1"
root="$2"
cd "$root"
source "$scripts/config-common.sh"
for i in "${!PATH_KEYS[@]}"; do
  printf 'K\t%s\t%s\n' "${PATH_KEYS[$i]}" "${PATH_DEFAULTS[$i]}"
done
for i in "${!WORK_KEYS[@]}"; do
  printf 'K\t%s\t%s\n' "${WORK_KEYS[$i]}" "${WORK_DEFAULTS[$i]}"
done
for i in "${!REVIEW_KEYS[@]}"; do
  printf 'K\t%s\t%s\n' "${REVIEW_KEYS[$i]}" "${REVIEW_DEFAULTS[$i]}"
done
for i in "${!AGENT_KEYS[@]}"; do
  printf 'K\t%s\t%s\n' "${AGENT_KEYS[$i]}" "${AGENT_DEFAULTS[$i]}"
done
for i in "${!VISUALISER_KEYS[@]}"; do
  printf 'K\t%s\t%s\n' "${VISUALISER_KEYS[$i]}" "${VISUALISER_DEFAULTS[$i]}"
done
for i in "${!DOC_TYPE_NAMES[@]}"; do
  printf 'D\t%s\t%s\n' "${DOC_TYPE_NAMES[$i]}" "${DOC_TYPE_PATH_KEYS[$i]}"
done
for k in "${TEMPLATE_KEYS[@]}"; do printf 'T\t%s\n' "$k"; done
"#;

    fn seed_config(root: &std::path::Path) -> Result<(), TestError> {
        let dir = root.join(".accelerator");
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("config.md"), "---\nseed: x\n---\n")?;
        std::fs::create_dir_all(root.join(".git"))?;
        Ok(())
    }

    #[test]
    fn the_rust_catalogue_matches_the_bash_catalogue() -> Result<(), TestError>
    {
        if !bash_available() {
            assert!(
                std::env::var_os("CI").is_none()
                    && std::env::var_os("GITHUB_ACTIONS").is_none(),
                "bash is required for the catalogue drift test under CI"
            );
            eprintln!("skipping drift test: bash unavailable (silent pass)");
            return Ok(());
        }

        let scripts = scripts_dir()?;
        let root = std::env::temp_dir().join(format!(
            "config-drift-{}-{}",
            std::process::id(),
            line!()
        ));
        seed_config(&root)?;

        let output = Command::new("bash")
            .arg("-c")
            .arg(EXTRACT)
            .arg("extract")
            .arg(&scripts)
            .arg(&root)
            .output()?;
        assert!(
            output.status.success(),
            "bash extraction failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout)?;

        let mut bash_keys: BTreeMap<String, String> = BTreeMap::new();
        let mut bash_templates: Vec<String> = Vec::new();
        let mut bash_doc_types: Vec<(String, String)> = Vec::new();
        for line in stdout.lines() {
            let fields: Vec<&str> = line.split('\t').collect();
            match fields.as_slice() {
                ["K", key, value] => {
                    bash_keys.insert((*key).to_owned(), (*value).to_owned());
                }
                ["T", key] => bash_templates.push((*key).to_owned()),
                ["D", name, path_key] => {
                    bash_doc_types
                        .push(((*name).to_owned(), (*path_key).to_owned()));
                }
                _ => {}
            }
        }

        assert_eq!(bash_keys, rust_defaults(), "key/default drift");

        let rust_templates: Vec<String> =
            TEMPLATE_KEYS.iter().map(|k| (*k).to_owned()).collect();
        assert_eq!(bash_templates, rust_templates, "template-key drift");

        let rust_doc_types: Vec<(String, String)> = DOC_TYPES
            .iter()
            .map(|(name, path_key)| {
                ((*name).to_owned(), (*path_key).to_owned())
            })
            .collect();
        assert_eq!(bash_doc_types, rust_doc_types, "doc-type pairing drift");

        std::fs::remove_dir_all(&root).ok();
        Ok(())
    }
}
