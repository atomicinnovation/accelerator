//! The byte-exact `## Review Configuration` block and lens catalogue.

use config::{catalogue, render_value, Value};

use crate::config_command::core::review::{Mode, ReviewView};
use crate::config_command::render::Rendered;

const PROSE: &str = "\
Use the paths below when constructing agent prompts. Always use the path
from this table rather than constructing paths from the lens name.";

/// The catalogue `review.core_lenses` default as a comma-joined display string
/// (its scalar items joined with `, `, not the bracketed sequence form).
fn default_core() -> String {
    match catalogue::default_for("review.core_lenses") {
        Some(Value::Sequence(items)) => items
            .into_iter()
            .map(|item| render_value(&Value::Scalar(item)))
            .collect::<Vec<_>>()
            .join(", "),
        _ => String::new(),
    }
}

#[must_use]
pub fn render(view: &ReviewView, mode: Mode) -> Rendered {
    let mut stdout = String::from("## Review Configuration\n\n");
    for value in &view.values {
        stdout.push_str("- **");
        stdout.push_str(value.label);
        stdout.push_str("**: ");
        stdout.push_str(&value.value);
        if value.value != value.default {
            stdout.push_str(" (default: ");
            stdout.push_str(&value.default);
            stdout.push(')');
        }
        stdout.push('\n');
    }
    if !view.core_lenses.is_empty() {
        stdout.push_str("- **Core lenses**: ");
        stdout.push_str(&view.core_lenses.join(", "));
        stdout.push_str("\n  (default: ");
        stdout.push_str(&default_core());
        stdout.push_str(")\n");
    }
    if !view.filtered_core_lenses.is_empty() {
        stdout.push_str("- **Filtered core lenses (not applicable to ");
        stdout.push_str(mode.label());
        stdout.push_str(" mode)**: ");
        stdout.push_str(&view.filtered_core_lenses.join(", "));
        stdout.push('\n');
    }
    if !view.disabled_lenses.is_empty() {
        stdout.push_str("- **Disabled lenses**: ");
        stdout.push_str(&view.disabled_lenses.join(", "));
        stdout.push_str(
            "\n  (these lenses should be skipped regardless of auto-detect)\n",
        );
    }
    for line in &view.verdict {
        stdout.push_str(line);
        stdout.push('\n');
    }
    stdout.push_str("\n### Lens Catalogue\n\n");
    stdout.push_str(PROSE);
    stdout.push_str("\n\n| Lens | Path | Source |\n|------|------|--------|\n");
    for lens in &view.builtin_lenses {
        stdout.push_str("| ");
        stdout.push_str(lens);
        stdout.push_str(" | skills/review/lenses/");
        stdout.push_str(lens);
        stdout.push_str("-lens/SKILL.md | built-in |\n");
    }
    for row in &view.custom_rows {
        stdout.push_str("| ");
        stdout.push_str(&row.name);
        stdout.push_str(" | ");
        stdout.push_str(&row.path);
        stdout.push_str(if row.always_include {
            " | custom (always include) |\n"
        } else {
            " | custom |\n"
        });
    }
    let mut warnings: Vec<String> = view
        .warnings
        .iter()
        .map(|warning| format!("Warning: {warning}"))
        .collect();
    warnings.extend(view.notes.iter().cloned());
    Rendered { stdout, warnings }
}

#[must_use]
pub fn render_unavailable() -> Rendered {
    Rendered::new("## Review Configuration Unavailable\n".to_owned())
}
