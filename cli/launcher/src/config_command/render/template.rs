//! Rendering for `template` (fenced), `templates show` (with source header),
//! and `templates list` (table), plus the `--fail-safe` notice and the
//! write-path messages for `eject`, `diff` and `reset`.

use config::{EjectOutcome, ResolvedTemplate, TemplateSource};

use crate::config_command::core::template::ListRow;
use crate::config_command::render::Rendered;

/// The template content wrapped in `markdown` fences, unless it is already
/// fenced (in which case it is emitted verbatim). Any tier-1 fallback note
/// rides on stderr.
#[must_use]
pub fn fenced(resolved: &ResolvedTemplate) -> Rendered {
    let stdout = if resolved.content.starts_with("```") {
        resolved.content.clone()
    } else {
        format!("```markdown\n{}```\n", resolved.content)
    };
    Rendered {
        stdout,
        warnings: resolved.warning.iter().cloned().collect(),
    }
}

/// The template content with a `Source:` header, unfenced.
#[must_use]
pub fn show(resolved: &ResolvedTemplate) -> Rendered {
    let stdout = format!(
        "Source: {} ({})\n---\n{}",
        resolved.source.label(),
        resolved.display_path,
        resolved.content
    );
    Rendered {
        stdout,
        warnings: resolved.warning.iter().cloned().collect(),
    }
}

#[must_use]
pub fn list(rows: &[ListRow]) -> Rendered {
    let mut stdout = String::from(
        "| Template | Source | Path |\n|----------|--------|------|\n",
    );
    for row in rows {
        stdout.push_str("| `");
        stdout.push_str(&row.key);
        stdout.push_str("` | ");
        stdout.push_str(&row.source);
        stdout.push_str(" | `");
        stdout.push_str(&row.display_path);
        stdout.push_str("` |\n");
    }
    Rendered::new(stdout)
}

#[must_use]
pub fn render_unavailable() -> Rendered {
    Rendered::new("## Template Unavailable\n".to_owned())
}

/// The one-line message an `eject` outcome prints, on the stream and with the
/// exit code the inbound handler selects.
#[must_use]
pub fn eject_text(
    outcome: EjectOutcome,
    key: &str,
    display: &str,
    available: &str,
) -> String {
    match outcome {
        EjectOutcome::Ejected => format!("Ejected: {key} -> {display}"),
        EjectOutcome::Overwritten => {
            format!("Overwritten: {key} -> {display}")
        }
        EjectOutcome::WouldEject => format!("Would eject: {key} -> {display}"),
        EjectOutcome::WouldOverwrite => {
            format!("Would overwrite: {key} -> {display}")
        }
        EjectOutcome::WouldSkip => format!(
            "Would skip: {key} (exists at {display}, use --force to overwrite)"
        ),
        EjectOutcome::Exists => {
            format!("Exists: {display} (use --force to overwrite)")
        }
        EjectOutcome::NoDefault => format!(
            "Error: No plugin default template for '{key}'. \
             Available: {available}"
        ),
    }
}

/// The `--all` aggregate message when some template could not be ejected.
pub const EJECT_ALL_ERROR: &str = "Some templates were not ejected. Fix the \
    errors above and re-run with --force to complete.";

/// The `--all` aggregate message when some target already existed.
pub const EJECT_ALL_EXISTS: &str =
    "Some templates already exist. Re-run with --force to overwrite.";

/// The full `diff` stdout: the header naming both paths, then either the
/// unified diff or the identical-templates line.
#[must_use]
pub fn diff_report(
    default: &ResolvedTemplate,
    user: &ResolvedTemplate,
) -> String {
    let mut out = format!(
        "Comparing plugin default vs user override:\n  Default: {}\n  \
         User:    {}\n\n",
        default.display_path, user.display_path
    );
    if default.content == user.content {
        out.push_str("Templates are identical.\n");
    } else {
        out.push_str(&unified_diff(&default.content, &user.content));
    }
    out
}

/// The `reset` dry-run report (no `--confirm`): the resolved source and path,
/// the outside-project warning, and the config-entry note where they apply.
#[must_use]
pub fn reset_found(
    resolved: &ResolvedTemplate,
    within_project: bool,
    name: &str,
) -> String {
    let mut out = format!(
        "Found override: {}\nPath: {}\n",
        resolved.source.label(),
        resolved.display_path
    );
    if !within_project {
        out.push_str("Warning: This file is outside the project directory (");
        out.push_str(&resolved.abs_path);
        out.push_str(").\n");
    }
    if resolved.source == TemplateSource::ConfigPath {
        out.push_str("Note: After deletion, also remove the 'templates.");
        out.push_str(name);
        out.push_str("' entry from your config.\n");
    }
    out
}

/// The `reset --confirm` report after the override is deleted.
#[must_use]
pub fn reset_confirmed(source: TemplateSource, name: &str) -> String {
    let mut out = format!("Reset: {name}\n");
    if source == TemplateSource::ConfigPath {
        out.push_str("Note: Also remove the 'templates.");
        out.push_str(name);
        out.push_str("' entry from your config.\n");
    }
    out
}

/// A line-oriented unified diff of `old` against `new`, common lines prefixed
/// with a space and changes with `-`/`+`, computed from the longest common
/// subsequence.
fn unified_diff(old: &str, new: &str) -> String {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let lcs = lcs_lengths(&a, &b);
    let mut out = String::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            push_line(&mut out, ' ', a[i]);
            i += 1;
            j += 1;
        } else if lcs[i + 1][j] >= lcs[i][j + 1] {
            push_line(&mut out, '-', a[i]);
            i += 1;
        } else {
            push_line(&mut out, '+', b[j]);
            j += 1;
        }
    }
    while i < a.len() {
        push_line(&mut out, '-', a[i]);
        i += 1;
    }
    while j < b.len() {
        push_line(&mut out, '+', b[j]);
        j += 1;
    }
    out
}

fn push_line(out: &mut String, prefix: char, line: &str) {
    out.push(prefix);
    out.push_str(line);
    out.push('\n');
}

fn lcs_lengths(a: &[&str], b: &[&str]) -> Vec<Vec<usize>> {
    let mut lengths = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lengths[i][j] = if a[i] == b[j] {
                lengths[i + 1][j + 1] + 1
            } else {
                lengths[i + 1][j].max(lengths[i][j + 1])
            };
        }
    }
    lengths
}
