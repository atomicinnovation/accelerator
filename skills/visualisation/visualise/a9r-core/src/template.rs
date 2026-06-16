//! Template resolution (`config-read-template`).
//!
//! Ports `config_resolve_template`, `config_enumerate_templates`,
//! `config_format_available_templates` from
//! [`scripts/config-common.sh`](../../../../scripts/config-common.sh) and the
//! `_output_template` fence-wrapping from
//! [`scripts/config-read-template.sh`](../../../../scripts/config-read-template.sh).
//!
//! Three-tier resolution (first hit wins):
//!   1. config `templates.<key>` path (relative → project-root-anchored);
//!   2. `<paths.templates>/<key>.md` (the configured templates directory,
//!      default `.accelerator/templates`);
//!   3. `<plugin_root>/templates/<key>.md`.
//!
//! The resolved file is emitted wrapped in a ```markdown fence unless it is
//! already fenced (first line starts with ```), in which case it is emitted
//! verbatim. A miss is a fatal error listing the available plugin templates.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::{read_value, repo, CommandOutput, ConfigError};

/// Usage string emitted by `config-read-template` with no template name.
pub const TEMPLATE_USAGE: &str = "Usage: config-read-template.sh <template_name>";

/// Anchor a possibly-relative config path against the project root.
fn anchor(project_root: &Path, value: &str) -> PathBuf {
    if value.starts_with('/') {
        PathBuf::from(value)
    } else {
        project_root.join(value)
    }
}

/// Wrap (or pass through) the resolved template file's contents, matching
/// `_output_template`: verbatim if the first line opens a code fence, else
/// surrounded by a ```markdown … ``` pair.
fn output_template(path: &Path) -> std::io::Result<String> {
    let contents = std::fs::read_to_string(path)?;
    let first_line = contents.split('\n').next().unwrap_or("");
    if first_line.starts_with("```") {
        Ok(contents)
    } else {
        Ok(format!("```markdown\n{contents}```\n"))
    }
}

/// Enumerate available template keys: the basenames (without `.md`) of
/// `<plugin_root>/templates/*.md`, sorted (bash glob order). Empty if the
/// directory is absent.
fn enumerate(plugin_root: &Path) -> Vec<String> {
    let dir = plugin_root.join("templates");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    // Bash globs `*.md` (sorted by the FULL filename, so `plan-review.md`
    // precedes `plan.md` because `-` < `.`), then strips `.md` via basename.
    // Sort the full names first, then strip — stripping first would reorder
    // any `<key>` / `<key>-suffix` pair.
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        // Case-sensitive `.md` suffix, matching the bash `*.md` glob. Use
        // strip_suffix (not ends_with) so clippy's case-insensitive-extension
        // lint does not push us off the intended byte-exact behaviour.
        .filter(|n| n.strip_suffix(".md").is_some())
        .collect();
    names.sort();
    names
        .into_iter()
        .filter_map(|n| n.strip_suffix(".md").map(ToString::to_string))
        .collect()
}

/// Format the available-template list (`a, b, c`) or `(none found)`.
fn format_available(plugin_root: &Path) -> String {
    let keys = enumerate(plugin_root);
    if keys.is_empty() {
        "(none found)".to_string()
    } else {
        keys.join(", ")
    }
}

/// Resolve and emit a template (`config-read-template`). The caller (binary)
/// runs the legacy-layout guard and the empty-name usage check first.
///
/// `plugin_root` is the plugin install root (the parent of `scripts/`); the
/// binary supplies it from `ACCELERATOR_PLUGIN_ROOT`.
pub fn read_template(
    start: &Path,
    key: &str,
    plugin_root: &Path,
    migration_mode: bool,
) -> Result<CommandOutput, ConfigError> {
    let project_root = repo::project_root(start);
    let mut stderr = String::new();

    // Tier 1: config-specified path (templates.<key>).
    let config_path = read_value(start, &format!("templates.{key}"), "", migration_mode).value;
    if !config_path.is_empty() {
        let resolved = anchor(&project_root, &config_path);
        if resolved.is_file() {
            return finish(output_template(&resolved), stderr);
        }
        let _ = writeln!(
            stderr,
            "Warning: configured template path '{}' not found, falling back to defaults",
            resolved.display()
        );
    }

    // Tier 2: configured templates directory (<paths.templates>/<key>.md).
    let templates_dir = read_value(
        start,
        "paths.templates",
        crate::defaults::path_default("templates").unwrap_or(""),
        migration_mode,
    )
    .value;
    let tier2 = anchor(&project_root, &templates_dir).join(format!("{key}.md"));
    if tier2.is_file() {
        return finish(output_template(&tier2), stderr);
    }

    // Tier 3: plugin default (<plugin_root>/templates/<key>.md).
    let tier3 = plugin_root.join("templates").join(format!("{key}.md"));
    if tier3.is_file() {
        return finish(output_template(&tier3), stderr);
    }

    // Miss: fatal error listing the available plugin templates. Any tier-1
    // warning precedes the error on stderr, matching bash.
    let available = format_available(plugin_root);
    let error = format!("Error: Template '{key}' not found. Available templates: {available}");
    Err(ConfigError::Message(format!("{stderr}{error}")))
}

/// Bundle the resolved stdout with the accumulated stderr, mapping an IO
/// failure (the file vanished between the `is_file` check and the read) to a
/// fatal message.
fn finish(stdout: std::io::Result<String>, stderr: String) -> Result<CommandOutput, ConfigError> {
    match stdout {
        Ok(stdout) => Ok(CommandOutput { stdout, stderr }),
        Err(e) => Err(ConfigError::Message(format!(
            "{stderr}Error: failed to read template: {e}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn repo(team: Option<&str>) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join(".git")).unwrap();
        if let Some(t) = team {
            fs::create_dir_all(tmp.path().join(".accelerator")).unwrap();
            fs::write(tmp.path().join(".accelerator/config.md"), t).unwrap();
        }
        tmp
    }

    fn plugin_with(templates: &[(&str, &str)]) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("templates")).unwrap();
        for (name, body) in templates {
            fs::write(
                tmp.path().join("templates").join(format!("{name}.md")),
                body,
            )
            .unwrap();
        }
        tmp
    }

    #[test]
    fn tier3_plugin_default_is_fenced() {
        let r = repo(None);
        let p = plugin_with(&[("plan", "# Plan\n\n## Overview\n")]);
        let out = read_template(r.path(), "plan", p.path(), false).unwrap();
        assert_eq!(out.stdout, "```markdown\n# Plan\n\n## Overview\n```\n");
        assert!(out.stderr.is_empty());
    }

    #[test]
    fn tier2_templates_dir_override() {
        let r = repo(None);
        fs::create_dir_all(r.path().join(".accelerator/templates")).unwrap();
        fs::write(
            r.path().join(".accelerator/templates/plan.md"),
            "# Custom\n\n## My Custom Section\n",
        )
        .unwrap();
        let p = plugin_with(&[("plan", "# Plugin default\n")]);
        let out = read_template(r.path(), "plan", p.path(), false).unwrap();
        assert!(out.stdout.contains("My Custom Section"));
        assert!(out.stdout.starts_with("```markdown\n"));
    }

    #[test]
    fn tier2_paths_templates_overridden_dir() {
        let r = repo(Some("---\npaths:\n  templates: docs/templates\n---\n"));
        fs::create_dir_all(r.path().join("docs/templates")).unwrap();
        fs::write(
            r.path().join("docs/templates/plan.md"),
            "# Overridden Directory Plan\n",
        )
        .unwrap();
        let p = plugin_with(&[("plan", "# Plugin default\n")]);
        let out = read_template(r.path(), "plan", p.path(), false).unwrap();
        assert!(out.stdout.contains("Overridden Directory Plan"));
    }

    #[test]
    fn tier1_config_path_takes_precedence() {
        let r = repo(Some("---\ntemplates:\n  plan: custom/my-plan.md\n---\n"));
        fs::create_dir_all(r.path().join("custom")).unwrap();
        fs::write(
            r.path().join("custom/my-plan.md"),
            "# Config-Specified Plan\n",
        )
        .unwrap();
        fs::create_dir_all(r.path().join(".accelerator/templates")).unwrap();
        fs::write(
            r.path().join(".accelerator/templates/plan.md"),
            "# Templates-Dir Plan\n",
        )
        .unwrap();
        let p = plugin_with(&[("plan", "# Plugin default\n")]);
        let out = read_template(r.path(), "plan", p.path(), false).unwrap();
        assert!(out.stdout.contains("Config-Specified Plan"));
    }

    #[test]
    fn already_fenced_is_not_double_wrapped() {
        let r = repo(None);
        fs::create_dir_all(r.path().join(".accelerator/templates")).unwrap();
        fs::write(
            r.path().join(".accelerator/templates/plan.md"),
            "```markdown\n# Already Fenced\n```\n",
        )
        .unwrap();
        let p = plugin_with(&[("plan", "# Plugin\n")]);
        let out = read_template(r.path(), "plan", p.path(), false).unwrap();
        assert_eq!(out.stdout.matches("```markdown").count(), 1);
    }

    #[test]
    fn tier1_missing_warns_and_falls_back() {
        let r = repo(Some("---\ntemplates:\n  plan: nonexistent/plan.md\n---\n"));
        let p = plugin_with(&[("plan", "# Plan\n\n## Overview\n")]);
        let out = read_template(r.path(), "plan", p.path(), false).unwrap();
        assert!(out.stderr.contains("Warning"));
        assert!(out.stdout.contains("## Overview"));
    }

    #[test]
    fn tier1_relative_path_anchored_to_project_root() {
        let r = repo(Some(
            "---\ntemplates:\n  plan: relative/path/plan.md\n---\n",
        ));
        fs::create_dir_all(r.path().join("relative/path")).unwrap();
        fs::write(
            r.path().join("relative/path/plan.md"),
            "# Relative Path Plan\n",
        )
        .unwrap();
        let p = plugin_with(&[("plan", "# Plugin\n")]);
        let out = read_template(r.path(), "plan", p.path(), false).unwrap();
        assert!(out.stdout.contains("Relative Path Plan"));
    }

    #[test]
    fn unknown_template_errors_with_available_list() {
        let r = repo(None);
        let p = plugin_with(&[("plan", "x"), ("research", "y"), ("adr", "z")]);
        let err = read_template(r.path(), "nonexistent", p.path(), false).unwrap_err();
        let msg = err.stderr();
        assert!(msg.contains("not found"));
        assert!(msg.contains("plan"));
        assert!(msg.contains("research"));
        assert!(msg.contains("adr"));
    }

    #[test]
    fn no_templates_dir_reports_none_found() {
        let r = repo(None);
        let p = tempfile::tempdir().unwrap();
        let err = read_template(r.path(), "x", p.path(), false).unwrap_err();
        assert!(err.stderr().contains("(none found)"));
    }
}
