//! Agent-name override resolution (`config-read-agents`).
//!
//! Ports the single-awk-pass parser and the override/default rendering from
//! [`scripts/config-read-agents.sh`](../../../../scripts/config-read-agents.sh).
//! The command always emits the full "Agent Names" block (skills reference
//! these variables); unknown keys are warned to stderr and ignored.

use std::fmt::Write as _;
use std::path::Path;

use crate::frontmatter::{self, Frontmatter};
use crate::{files, repo, CommandOutput};

/// Valid agent keys in display order — the canonical override list. Order
/// determines the output row order. Mirrors the bash `AGENT_KEYS` array (also
/// asserted against the `agents/*.md` files by the test suite).
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

/// Prefix applied to the default (un-overridden) agent name.
pub const AGENT_PREFIX: &str = "accelerator:";

const AGENTS_HEADER: &str = "## Agent Names\n\nThe following agent names are configured for this project. Always use\nthe name shown for each role as the `subagent_type` parameter when\nspawning agents via the Agent/Task tool.\n";

/// One parsed line from an `agents:` section.
enum Entry {
    /// A recognised key with its resolved value.
    Override(String, String),
    /// An unrecognised key (warned and ignored).
    Unknown(String),
}

/// Is `line` an indented key line (`^[ \t]+[a-zA-Z]`)?
fn is_indented_key_line(line: &str) -> bool {
    let trimmed = line.trim_start_matches([' ', '\t']);
    trimmed.len() < line.len()
        && trimmed
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
}

/// Strip one matched layer of surrounding `"`/`'` (awk `^".*"$ || ^'.*'$`).
fn strip_one_quote_layer(val: &str) -> &str {
    let b = val.as_bytes();
    if b.len() >= 2 {
        let (first, last) = (b[0], b[b.len() - 1]);
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &val[1..val.len() - 1];
        }
    }
    val
}

/// Parse the first `agents:` section of a frontmatter block. Mirrors the awk:
/// the section opens on a line starting `agents:`, parsing stops entirely
/// (`exit`) at the next non-indented non-blank line, and only indented lines
/// whose first non-space char is a letter are treated as key entries.
fn parse_section(fm: &str) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut in_section = false;
    for line in fm.split('\n') {
        if line.starts_with("agents:") {
            in_section = true;
            continue;
        }
        if !in_section {
            continue;
        }
        // `^[^ \t]`: a line whose first char is neither space nor tab ends the
        // scan. A blank line (no first char) does not.
        match line.chars().next() {
            Some(c) if c != ' ' && c != '\t' => break,
            _ => {}
        }
        if !is_indented_key_line(line) {
            continue;
        }
        let stripped = line.trim_start_matches([' ', '\t']);
        let (key, val) = match stripped.find(':') {
            Some(i) => (
                &stripped[..i],
                strip_one_quote_layer(stripped[i + 1..].trim_start_matches([' ', '\t'])),
            ),
            // No colon: awk's subs leave both key and val as the whole token.
            None => (stripped, stripped),
        };
        if AGENT_KEYS.contains(&key) {
            out.push(Entry::Override(key.to_string(), val.to_string()));
        } else {
            out.push(Entry::Unknown(key.to_string()));
        }
    }
    out
}

/// Look up the last-set override for `key`, or `None`.
fn resolved<'a>(overrides: &'a [(String, String)], key: &str) -> Option<&'a str> {
    overrides
        .iter()
        .rev()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

/// Resolve and render the agent-name block (`config-read-agents`). Reads team
/// then local config (last-writer-wins per key), warns on unknown keys, and
/// always emits the full block. No legacy-layout guard (matching bash).
pub fn read_agents(start: &Path, migration_mode: bool) -> CommandOutput {
    let root = repo::project_root(start);
    let mut overrides: Vec<(String, String)> = Vec::new();
    let mut stderr = String::new();
    for file in files::config_files(&root, migration_mode) {
        let Ok(contents) = std::fs::read_to_string(&file) else {
            continue;
        };
        let Frontmatter::Closed(fm) = frontmatter::extract(&contents) else {
            continue;
        };
        if fm.is_empty() {
            continue;
        }
        for entry in parse_section(&fm) {
            match entry {
                Entry::Override(key, val) => overrides.push((key, val)),
                Entry::Unknown(key) => {
                    let _ = writeln!(
                        stderr,
                        "Warning: unknown agent key '{key}' in {} — ignoring",
                        file.display()
                    );
                }
            }
        }
    }

    let mut lines = String::new();
    for key in AGENT_KEYS {
        let val = resolved(&overrides, key)
            .filter(|v| !v.is_empty())
            .map_or_else(|| format!("{AGENT_PREFIX}{key}"), ToString::to_string);
        let display = key.replace('-', " ");
        let _ = writeln!(lines, "- **{display} agent**: {val}");
    }

    CommandOutput {
        stdout: format!("{AGENTS_HEADER}\n{lines}"),
        stderr,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn repo_with(team: Option<&str>, local: Option<&str>) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join(".git")).unwrap();
        fs::create_dir_all(tmp.path().join(".accelerator")).unwrap();
        if let Some(t) = team {
            fs::write(tmp.path().join(".accelerator/config.md"), t).unwrap();
        }
        if let Some(l) = local {
            fs::write(tmp.path().join(".accelerator/config.local.md"), l).unwrap();
        }
        tmp
    }

    #[test]
    fn no_config_outputs_all_defaults() {
        let tmp = repo_with(None, None);
        let out = read_agents(tmp.path(), false);
        assert!(out.stdout.starts_with("## Agent Names\n\n"));
        assert!(out
            .stdout
            .contains("- **reviewer agent**: accelerator:reviewer\n"));
        assert!(out
            .stdout
            .contains("- **codebase locator agent**: accelerator:codebase-locator\n"));
        assert!(out
            .stdout
            .contains("- **web search researcher agent**: accelerator:web-search-researcher\n"));
        assert!(out.stderr.is_empty());
    }

    #[test]
    fn overrides_are_applied() {
        let tmp = repo_with(
            Some("---\nagents:\n  reviewer: my-custom-reviewer\n  codebase-locator: my-locator\n---\n"),
            None,
        );
        let out = read_agents(tmp.path(), false);
        assert!(out
            .stdout
            .contains("- **reviewer agent**: my-custom-reviewer\n"));
        assert!(out
            .stdout
            .contains("- **codebase locator agent**: my-locator\n"));
    }

    #[test]
    fn local_overrides_team() {
        let tmp = repo_with(
            Some("---\nagents:\n  reviewer: team-reviewer\n---\n"),
            Some("---\nagents:\n  reviewer: local-reviewer\n---\n"),
        );
        let out = read_agents(tmp.path(), false);
        assert!(out
            .stdout
            .contains("- **reviewer agent**: local-reviewer\n"));
        assert!(!out.stdout.contains("team-reviewer"));
    }

    #[test]
    fn non_overlapping_overrides_both_appear() {
        let tmp = repo_with(
            Some("---\nagents:\n  reviewer: custom-reviewer\n---\n"),
            Some("---\nagents:\n  codebase-locator: custom-locator\n---\n"),
        );
        let out = read_agents(tmp.path(), false);
        assert!(out
            .stdout
            .contains("- **reviewer agent**: custom-reviewer\n"));
        assert!(out
            .stdout
            .contains("- **codebase locator agent**: custom-locator\n"));
    }

    #[test]
    fn unknown_key_warns_and_is_ignored() {
        let tmp = repo_with(
            Some("---\nagents:\n  reviewer: my-reviewer\n  unknown-agent: something\n---\n"),
            None,
        );
        let out = read_agents(tmp.path(), false);
        assert!(out.stderr.contains("unknown-agent"));
        assert!(out.stderr.contains("Warning"));
        assert!(!out.stdout.contains("unknown-agent"));
    }

    #[test]
    fn identity_override_shows_given_value() {
        // reviewer: reviewer → shows "reviewer" (the override is applied as-is,
        // even though it equals the bare key without the prefix).
        let tmp = repo_with(
            Some("---\nagents:\n  reviewer: reviewer\n  codebase-locator: my-locator\n---\n"),
            None,
        );
        let out = read_agents(tmp.path(), false);
        assert!(out.stdout.contains("- **reviewer agent**: reviewer\n"));
        assert!(out
            .stdout
            .contains("- **codebase locator agent**: my-locator\n"));
    }

    #[test]
    fn rows_in_agent_keys_order() {
        let tmp = repo_with(
            Some("---\nagents:\n  web-search-researcher: w\n  reviewer: r\n---\n"),
            None,
        );
        let out = read_agents(tmp.path(), false);
        let first = out.stdout.lines().find(|l| l.starts_with("- **")).unwrap();
        let last = out
            .stdout
            .lines()
            .filter(|l| l.starts_with("- **"))
            .next_back()
            .unwrap();
        assert!(first.contains("reviewer agent"));
        assert!(last.contains("web search researcher agent"));
    }

    #[test]
    fn frontmatter_without_agents_section_outputs_defaults() {
        let tmp = repo_with(Some("---\nreview:\n  max_count: 5\n---\n"), None);
        let out = read_agents(tmp.path(), false);
        assert!(out
            .stdout
            .contains("- **reviewer agent**: accelerator:reviewer\n"));
        assert!(out.stderr.is_empty());
    }

    #[test]
    fn quoted_override_value_is_stripped() {
        let tmp = repo_with(
            Some("---\nagents:\n  reviewer: \"quoted-rev\"\n---\n"),
            None,
        );
        let out = read_agents(tmp.path(), false);
        assert!(out.stdout.contains("- **reviewer agent**: quoted-rev\n"));
    }
}
