//! Rendering for `template` (fenced), `templates show` (with source header),
//! and `templates list` (table), plus the `--fail-safe` notice.

use config::ResolvedTemplate;

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
