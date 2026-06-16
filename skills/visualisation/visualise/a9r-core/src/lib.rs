//! `a9r-core` — pure config-resolution logic shared by the `a9r` binary.
//!
//! This crate is the single owner of the `config-read-*` parsing/resolution
//! semantics, ported byte-for-byte from the bash scripts under `scripts/`
//! (`config-read-value.sh`, `config-read-path.sh`, `config-common.sh`,
//! `config-defaults.sh`, `vcs-common.sh`). It has **zero** dependency on the
//! visualiser lib — the dependency arrows point `a9r → {a9r-core, visualiser}`,
//! never the reverse — so the logic boundary stays clean and a later
//! two-binary split remains possible.
//!
//! The visualiser lib keeps its own `frontmatter`/`config` modules: those
//! parse documents for the SPA (gray_matter-based YAML) and serve a different
//! purpose from this crate's bash-faithful config reader. The two are
//! intentionally separate; `a9r-core` owns the *config-read CLI contract*.
//!
//! ## Functional core / imperative shell
//!
//! Nothing here touches stdout/stderr or exits. Entry points return
//! [`ReadOutcome`] (the stdout value line + ordered stderr warnings) or
//! [`ConfigError`]; the `a9r` binary owns stream/exit policy. This keeps the
//! byte-for-byte CLI contract testable without spawning a process.

use std::path::Path;

pub mod body;
pub mod defaults;
pub mod files;
pub mod frontmatter;
pub mod lookup;
pub mod repo;

use frontmatter::Frontmatter;

/// Usage string emitted by `config-read-value` on an empty/missing key.
pub const VALUE_USAGE: &str = "Usage: config-read-value.sh <key> [default]";
/// Usage string emitted by `config-read-path` on an empty/missing key.
pub const PATH_USAGE: &str = "Usage: config-read-path.sh <path_key> [default]";
/// Usage string emitted by `config-read-skill-context` with no skill name.
pub const SKILL_CONTEXT_USAGE: &str = "Usage: config-read-skill-context.sh <skill-name>";
/// Usage string emitted by `config-read-skill-instructions` with no skill name.
pub const SKILL_INSTRUCTIONS_USAGE: &str = "Usage: config-read-skill-instructions.sh <skill-name>";

/// The outcome of a config read: the single stdout line the binary prints
/// (the binary appends exactly one trailing newline) and any stderr warning
/// lines, in emission order.
#[derive(Debug, PartialEq, Eq)]
pub struct ReadOutcome {
    pub value: String,
    pub warnings: Vec<String>,
}

/// The exact stdout/stderr bytes a command produces. Unlike [`ReadOutcome`]
/// (which the binary decorates with a trailing newline), [`Self::stdout`] is
/// written **verbatim** — the section-emitting commands
/// (`config-read-context`, `config-read-agents`, `config-read-template`, …)
/// control their own trailing newline, and an empty `stdout` must print
/// nothing at all. [`Self::stderr`] is likewise verbatim (already
/// newline-terminated where bash emits a line).
#[derive(Debug, PartialEq, Eq, Default)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
}

/// A fatal config error. Exit-code mapping is centralised in
/// [`ConfigError::exit_code`] so the surprising-but-required behaviour
/// (not-found deliberately exits 0, handled by the binary printing a
/// [`ReadOutcome`]) lives at one site and cannot be silently "fixed".
#[derive(Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// Legacy `.claude/accelerator.md` layout detected — the `exit 1` guard.
    LegacyLayout,
    /// Missing/empty key — usage error.
    Usage(&'static str),
    /// A dynamic error message (exit 1), e.g. the `config-read-template`
    /// "not found, available templates: …" message whose text is computed at
    /// runtime.
    Message(String),
}

impl ConfigError {
    /// The process exit code. Both fatal variants are exit 1, matching bash.
    /// Note: not-found is **not** an error here — it exits 0 by echoing the
    /// default (the binary prints a [`ReadOutcome`]); there is no distinct
    /// not-found code.
    pub fn exit_code(&self) -> u8 {
        1
    }

    /// The exact stderr text (without a trailing newline; the binary adds
    /// one). The legacy message is two lines, matching the bash `printf
    /// '%s\n' line1 line2`.
    pub fn stderr(&self) -> String {
        match self {
            Self::LegacyLayout => concat!(
                "Accelerator: legacy config detected at .claude/accelerator.md.\n",
                "Run /accelerator:migrate to update the layout, then retry."
            )
            .to_string(),
            Self::Usage(msg) => (*msg).to_string(),
            Self::Message(msg) => msg.clone(),
        }
    }
}

/// Result of resolving a `config-read-path` invocation up to (but not
/// including) the value lookup. The binary emits [`Self::warnings`] to
/// stderr, runs [`assert_no_legacy_layout`], then calls [`read_value`] with
/// [`Self::value_key`] and [`Self::default`] — preserving the exact bash
/// ordering (path warnings → legacy assert → value warnings → stdout).
#[derive(Debug, PartialEq, Eq)]
pub struct PathResolution {
    pub value_key: String,
    pub default: String,
    pub warnings: Vec<String>,
}

/// The legacy-layout guard, resolving the project root from `start` exactly
/// as `config_assert_no_legacy_layout` does (`config_project_root`, with the
/// `$PWD` fallback).
pub fn assert_no_legacy_layout(start: &Path, migration_mode: bool) -> Result<(), ConfigError> {
    let root = repo::project_root(start);
    files::assert_no_legacy_layout(&root, migration_mode)
}

fn resolve_value(start: &Path, key: &str, migration_mode: bool) -> (Option<String>, Vec<String>) {
    let root = repo::project_root(start);
    let (section, subkey) = lookup::split_key(key);
    let mut result: Option<String> = None;
    let mut warnings = Vec::new();
    for file in files::config_files(&root, migration_mode) {
        let Ok(contents) = std::fs::read_to_string(&file) else {
            continue;
        };
        match frontmatter::extract(&contents) {
            Frontmatter::Absent => {}
            Frontmatter::Unclosed => {
                if frontmatter::opens_loosely(&contents) {
                    warnings.push(format!(
                        "Warning: {} has unclosed YAML frontmatter — ignoring",
                        file.display()
                    ));
                }
            }
            // Found-empty (Some("")) overrides the result and suppresses the
            // default; last file wins (the loop does not break).
            Frontmatter::Closed(text) => {
                if let Some(v) = lookup::lookup(&text, section, subkey) {
                    result = Some(v);
                }
            }
        }
    }
    (result, warnings)
}

/// Read a single config value (`config-read-value`). The default is applied
/// only when **no** file produced a match; a present-but-empty value
/// (`Some("")`) suppresses the default.
pub fn read_value(start: &Path, key: &str, default: &str, migration_mode: bool) -> ReadOutcome {
    let (found, warnings) = resolve_value(start, key, migration_mode);
    ReadOutcome {
        value: found.unwrap_or_else(|| default.to_string()),
        warnings,
    }
}

fn file_contains(path: &Path, needle: &str) -> bool {
    std::fs::read_to_string(path).is_ok_and(|c| c.contains(needle))
}

/// The header `config-read-context` prints before the joined bodies.
const CONTEXT_HEADER: &str = "## Project Context\n\nThe following project-specific context has been provided. Take this into\naccount when making decisions, selecting approaches, and generating output.\n";

/// Read the project context (`config-read-context`): the trimmed markdown
/// body of each config file (team first, local second), joined by a blank
/// line, under a fixed header. If every body is empty, the output is empty
/// (the binary prints nothing). This command does **not** run the
/// legacy-layout guard, matching the bash script.
pub fn read_context(start: &Path, migration_mode: bool) -> CommandOutput {
    let root = repo::project_root(start);
    let mut sections: Vec<String> = Vec::new();
    for file in files::config_files(&root, migration_mode) {
        let Ok(contents) = std::fs::read_to_string(&file) else {
            continue;
        };
        let trimmed = body::trim_body(&body::extract_body(&contents));
        if !trimmed.is_empty() {
            sections.push(trimmed);
        }
    }
    if sections.is_empty() {
        return CommandOutput::default();
    }
    let joined = sections.join("\n\n");
    CommandOutput {
        stdout: format!("{CONTEXT_HEADER}\n{joined}\n"),
        stderr: String::new(),
    }
}

/// Which per-skill customisation file a [`read_skill_section`] call reads and
/// the header wrapper it prints.
pub enum SkillSection {
    /// `context.md` → "## Skill-Specific Context".
    Context,
    /// `instructions.md` → "## Additional Instructions".
    Instructions,
}

impl SkillSection {
    fn file_name(&self) -> &'static str {
        match self {
            Self::Context => "context.md",
            Self::Instructions => "instructions.md",
        }
    }

    /// The full section text given the resolved skill name and trimmed body,
    /// reproducing the script's `echo`/`printf` block byte-for-byte (trailing
    /// newline included).
    fn render(&self, skill: &str, content: &str) -> String {
        match self {
            Self::Context => format!(
                "## Skill-Specific Context\n\nThe following context is specific to the {skill} skill. Apply this\ncontext in addition to any project-wide context above.\n\n{content}\n",
            ),
            Self::Instructions => format!(
                "## Additional Instructions\n\nThe following additional instructions have been provided for the\n{skill} skill. Follow these instructions in addition to all\ninstructions above.\n\n{content}\n",
            ),
        }
    }
}

/// Read a per-skill `context.md` / `instructions.md`
/// (`config-read-skill-context` / `config-read-skill-instructions`). The file
/// lives at `<root>/.accelerator/skills/<skill>/<file>`. A missing file, or a
/// file that trims to empty, produces empty output (exit 0).
///
/// The caller (binary) runs the legacy-layout guard and the empty-skill usage
/// check **before** this, matching the bash ordering (assert → arg check →
/// read).
pub fn read_skill_section(start: &Path, skill: &str, section: &SkillSection) -> CommandOutput {
    let root = repo::project_root(start);
    let file = root
        .join(".accelerator/skills")
        .join(skill)
        .join(section.file_name());
    let Ok(contents) = std::fs::read_to_string(&file) else {
        return CommandOutput::default();
    };
    let trimmed = body::trim_body(&contents);
    if trimmed.is_empty() {
        return CommandOutput::default();
    }
    CommandOutput {
        stdout: section.render(skill, &trimmed),
        stderr: String::new(),
    }
}

/// Resolve the default and migration warnings for a `config-read-path`
/// invocation, returning the `paths.<key>` value key to look up.
///
/// Mirrors `config-read-path.sh`: an explicit **non-empty** default wins;
/// an explicit empty default (`''`) is treated as omitted (the bash `[ -n
/// "${2:-}" ]` guard), falling through to the defaults table; a table miss
/// yields an empty default plus a stderr warning. The two-stage
/// legacy-override probe (substring presence in the config files AND a
/// non-empty *resolved* legacy value) is reproduced via the in-process value
/// lookup.
pub fn resolve_path(
    start: &Path,
    key: &str,
    explicit_default: Option<&str>,
    migration_mode: bool,
) -> PathResolution {
    let mut warnings = Vec::new();

    let explicit = explicit_default.filter(|d| !d.is_empty());
    let default = if let Some(d) = explicit {
        d.to_string()
    } else if let Some(d) = defaults::path_default(key) {
        d.to_string()
    } else {
        match key {
            "design_inventories" | "design_gaps" => warnings.push(format!(
                "config-read-path.sh: warning: key '{key}' was renamed by \
                 migration 0004 to 'research_{key}'; run /accelerator:migrate"
            )),
            _ => warnings.push(format!(
                "config-read-path.sh: warning: unknown key '{key}' — no \
                 centralized default"
            )),
        }
        String::new()
    };

    if let Some(legacy) = key
        .strip_prefix("research_")
        .filter(|_| matches!(key, "research_design_inventories" | "research_design_gaps"))
    {
        if let Some(repo_root) = repo::find_repo_root(start) {
            let team = repo_root.join(".accelerator/config.md");
            let local = repo_root.join(".accelerator/config.local.md");
            if file_contains(&team, legacy) || file_contains(&local, legacy) {
                let probe = read_value(start, &format!("paths.{legacy}"), "", migration_mode);
                if !probe.value.is_empty() {
                    warnings.push(format!(
                        "config-read-path.sh: warning: your config sets \
                         'paths.{legacy}' (renamed by migration 0004 to \
                         'paths.{key}'); the legacy override is being \
                         ignored. Run /accelerator:migrate"
                    ));
                }
            }
        }
    }

    PathResolution {
        value_key: format!("paths.{key}"),
        default,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(path: &Path, body: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    /// A repo root with a `.git` marker so root discovery resolves to it.
    fn repo_with(team: Option<&str>, local: Option<&str>) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join(".git")).unwrap();
        if let Some(t) = team {
            write(&tmp.path().join(".accelerator/config.md"), t);
        }
        if let Some(l) = local {
            write(&tmp.path().join(".accelerator/config.local.md"), l);
        }
        tmp
    }

    #[test]
    fn value_found_in_team_config() {
        let tmp = repo_with(Some("---\nagents:\n  reviewer: my-rev\n---\n"), None);
        let out = read_value(tmp.path(), "agents.reviewer", "fallback", false);
        assert_eq!(out.value, "my-rev");
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn not_found_returns_default_with_no_warning() {
        let tmp = repo_with(Some("---\nagents:\n  reviewer: x\n---\n"), None);
        let out = read_value(tmp.path(), "agents.missing", "the-default", false);
        assert_eq!(out.value, "the-default");
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn last_file_wins_local_overrides_team() {
        let tmp = repo_with(
            Some("---\nenabled: team\n---\n"),
            Some("---\nenabled: local\n---\n"),
        );
        let out = read_value(tmp.path(), "enabled", "d", false);
        assert_eq!(out.value, "local");
    }

    #[test]
    fn last_file_wins_only_from_the_second_file() {
        // Team sets it, local does not → team value survives (the loop does
        // not reset on a non-matching later file).
        let tmp = repo_with(
            Some("---\nenabled: team\n---\n"),
            Some("---\nother: x\n---\n"),
        );
        let out = read_value(tmp.path(), "enabled", "d", false);
        assert_eq!(out.value, "team");
    }

    #[test]
    fn found_empty_suppresses_default() {
        let tmp = repo_with(Some("---\nenabled:\n---\n"), None);
        let out = read_value(tmp.path(), "enabled", "the-default", false);
        assert_eq!(out.value, "", "present-but-empty must beat the default");
    }

    #[test]
    fn local_empty_overrides_team_non_empty() {
        // The empty match in local still sets found=true, overriding team.
        let tmp = repo_with(
            Some("---\nenabled: team\n---\n"),
            Some("---\nenabled:\n---\n"),
        );
        let out = read_value(tmp.path(), "enabled", "d", false);
        assert_eq!(out.value, "");
    }

    #[test]
    fn unclosed_frontmatter_warns_and_falls_through_to_default() {
        let tmp = repo_with(Some("---\nenabled: x\n"), None);
        let out = read_value(tmp.path(), "enabled", "the-default", false);
        assert_eq!(out.value, "the-default");
        assert_eq!(out.warnings.len(), 1);
        assert!(out.warnings[0].contains("Warning"));
        assert!(out.warnings[0].contains("unclosed YAML frontmatter"));
    }

    #[test]
    fn empty_but_closed_frontmatter_is_silent_not_found() {
        let tmp = repo_with(Some("---\n---\n"), None);
        let out = read_value(tmp.path(), "enabled", "the-default", false);
        assert_eq!(out.value, "the-default");
        assert!(out.warnings.is_empty(), "empty-closed must NOT warn");
    }

    // ── legacy guard ────────────────────────────────────────────────────

    #[test]
    fn legacy_only_layout_errors() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join(".git")).unwrap();
        write(&tmp.path().join(".claude/accelerator.md"), "x");
        let err = assert_no_legacy_layout(tmp.path(), false).unwrap_err();
        assert_eq!(err, ConfigError::LegacyLayout);
        assert_eq!(err.exit_code(), 1);
        assert!(err.stderr().contains(".claude/accelerator.md"));
        assert!(err.stderr().contains("/accelerator:migrate"));
    }

    // ── read_path ───────────────────────────────────────────────────────

    #[test]
    fn path_known_key_uses_table_default() {
        let tmp = repo_with(None, None);
        let pr = resolve_path(tmp.path(), "plans", None, false);
        assert_eq!(pr.value_key, "paths.plans");
        assert_eq!(pr.default, "meta/plans");
        assert!(pr.warnings.is_empty());
        // ...and with no config, read_value echoes the table default.
        let out = read_value(tmp.path(), &pr.value_key, &pr.default, false);
        assert_eq!(out.value, "meta/plans");
    }

    #[test]
    fn path_explicit_nonempty_default_wins_over_table() {
        let tmp = repo_with(None, None);
        let pr = resolve_path(tmp.path(), "plans", Some("custom/dir"), false);
        assert_eq!(pr.default, "custom/dir");
        assert!(pr.warnings.is_empty());
    }

    #[test]
    fn path_explicit_empty_default_is_treated_as_omitted() {
        // `config-read-path plans ''` ≡ `config-read-path plans` — both fall
        // through to the table.
        let tmp = repo_with(None, None);
        let omitted = resolve_path(tmp.path(), "plans", None, false);
        let empty = resolve_path(tmp.path(), "plans", Some(""), false);
        assert_eq!(omitted.default, "meta/plans");
        assert_eq!(empty.default, "meta/plans");
    }

    #[test]
    fn path_unknown_key_warns_but_still_reads_config() {
        // Unknown key with no default → warning + empty default, BUT the
        // value lookup still honours a user-set paths.<unknown>.
        let tmp = repo_with(Some("---\npaths:\n  unknownkey: /custom/path\n---\n"), None);
        let pr = resolve_path(tmp.path(), "unknownkey", None, false);
        assert_eq!(pr.value_key, "paths.unknownkey");
        assert_eq!(pr.default, "");
        assert_eq!(pr.warnings.len(), 1);
        assert!(pr.warnings[0].contains("unknown key 'unknownkey'"));
        let out = read_value(tmp.path(), &pr.value_key, &pr.default, false);
        assert_eq!(out.value, "/custom/path");
    }

    #[test]
    fn path_legacy_bare_key_emits_rename_warning() {
        let tmp = repo_with(None, None);
        let pr = resolve_path(tmp.path(), "design_inventories", None, false);
        assert_eq!(pr.warnings.len(), 1);
        assert!(pr.warnings[0].contains("migration 0004"));
        assert!(pr.warnings[0].contains("research_design_inventories"));
    }

    #[test]
    fn path_legacy_override_probe_warns_only_when_alias_resolves_nonempty() {
        // Canonical key requested; config sets the legacy alias to a
        // non-empty value → warning naming the ignored key.
        let tmp = repo_with(
            Some("---\npaths:\n  design_inventories: meta/old\n---\n"),
            None,
        );
        let pr = resolve_path(tmp.path(), "research_design_inventories", None, false);
        assert!(
            pr.warnings
                .iter()
                .any(|w| w.contains("paths.design_inventories") && w.contains("ignored")),
            "warnings: {:?}",
            pr.warnings,
        );
    }

    #[test]
    fn path_legacy_override_probe_silent_when_alias_absent() {
        let tmp = repo_with(Some("---\nunrelated: x\n---\n"), None);
        let pr = resolve_path(tmp.path(), "research_design_inventories", None, false);
        assert!(
            !pr.warnings.iter().any(|w| w.contains("ignored")),
            "warnings: {:?}",
            pr.warnings,
        );
    }

    // ── read_context ────────────────────────────────────────────────────

    #[test]
    fn context_empty_when_no_config() {
        let tmp = repo_with(None, None);
        assert_eq!(read_context(tmp.path(), false), CommandOutput::default());
    }

    #[test]
    fn context_team_body_under_header() {
        let tmp = repo_with(
            Some("---\nkey: value\n---\n\nThis is the project context.\n"),
            None,
        );
        let out = read_context(tmp.path(), false);
        assert_eq!(
            out.stdout,
            "## Project Context\n\nThe following project-specific context has been provided. Take this into\naccount when making decisions, selecting approaches, and generating output.\n\nThis is the project context.\n",
        );
        assert!(out.stderr.is_empty());
    }

    #[test]
    fn context_both_team_first() {
        let tmp = repo_with(
            Some("---\nkey: value\n---\n\nTeam context.\n"),
            Some("---\nkey: value\n---\n\nPersonal context.\n"),
        );
        let out = read_context(tmp.path(), false);
        assert!(out.stdout.ends_with("Team context.\n\nPersonal context.\n"));
    }

    #[test]
    fn context_empty_body_outputs_nothing() {
        let tmp = repo_with(Some("---\nkey: value\n---\n\n\n"), None);
        assert_eq!(read_context(tmp.path(), false), CommandOutput::default());
    }

    #[test]
    fn context_unclosed_frontmatter_outputs_nothing() {
        let tmp = repo_with(Some("---\nkey: value\nno closing\n"), None);
        assert_eq!(read_context(tmp.path(), false), CommandOutput::default());
    }

    // ── read_skill_section ──────────────────────────────────────────────

    fn skill_file(tmp: &tempfile::TempDir, skill: &str, file: &str, body: &str) {
        let p = tmp
            .path()
            .join(".accelerator/skills")
            .join(skill)
            .join(file);
        write(&p, body);
    }

    #[test]
    fn skill_context_missing_is_empty() {
        let tmp = repo_with(None, None);
        let out = read_skill_section(tmp.path(), "create-plan", &SkillSection::Context);
        assert_eq!(out, CommandOutput::default());
    }

    #[test]
    fn skill_context_renders_section() {
        let tmp = repo_with(None, None);
        skill_file(&tmp, "create-plan", "context.md", "Some context content.\n");
        let out = read_skill_section(tmp.path(), "create-plan", &SkillSection::Context);
        assert_eq!(
            out.stdout,
            "## Skill-Specific Context\n\nThe following context is specific to the create-plan skill. Apply this\ncontext in addition to any project-wide context above.\n\nSome context content.\n",
        );
    }

    #[test]
    fn skill_context_whitespace_only_is_empty() {
        let tmp = repo_with(None, None);
        skill_file(&tmp, "create-plan", "context.md", "   \n\n  \n");
        let out = read_skill_section(tmp.path(), "create-plan", &SkillSection::Context);
        assert_eq!(out, CommandOutput::default());
    }

    #[test]
    fn skill_context_trims_surrounding_blanks() {
        let tmp = repo_with(None, None);
        skill_file(
            &tmp,
            "review-pr",
            "context.md",
            "\n\nTrimmed content.\n\n\n",
        );
        let out = read_skill_section(tmp.path(), "review-pr", &SkillSection::Context);
        assert!(out.stdout.ends_with("\n\nTrimmed content.\n"));
    }

    #[test]
    fn skill_instructions_renders_section() {
        let tmp = repo_with(None, None);
        skill_file(
            &tmp,
            "review-pr",
            "instructions.md",
            "Always check for tests.\n",
        );
        let out = read_skill_section(tmp.path(), "review-pr", &SkillSection::Instructions);
        assert_eq!(
            out.stdout,
            "## Additional Instructions\n\nThe following additional instructions have been provided for the\nreview-pr skill. Follow these instructions in addition to all\ninstructions above.\n\nAlways check for tests.\n",
        );
    }

    #[test]
    fn skill_section_reads_only_its_own_dir() {
        let tmp = repo_with(None, None);
        skill_file(&tmp, "create-plan", "context.md", "Plan context.\n");
        skill_file(&tmp, "review-pr", "context.md", "PR context.\n");
        let plan = read_skill_section(tmp.path(), "create-plan", &SkillSection::Context);
        assert!(plan.stdout.contains("Plan context."));
        assert!(!plan.stdout.contains("PR context."));
    }
}
