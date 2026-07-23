//! The `## Configured Paths` block, the doc-type TSV, and the `--fail-safe`
//! notice. The block ends in a trailing newline, matching the bash reader.

use crate::config_command::core::paths::{ConfiguredPath, DocTypes};
use crate::config_command::render::Rendered;

#[must_use]
pub fn configured(paths: &[ConfiguredPath]) -> Rendered {
    let mut stdout = String::from("## Configured Paths\n\n");
    for path in paths {
        stdout.push_str("- ");
        stdout.push_str(&path.key);
        stdout.push_str(": ");
        stdout.push_str(&path.value);
        stdout.push('\n');
    }
    Rendered::new(stdout)
}

#[must_use]
pub fn doc_types(view: &DocTypes) -> Rendered {
    let mut stdout = String::new();
    for (doc_type, dir) in &view.rows {
        stdout.push_str(doc_type);
        stdout.push('\t');
        stdout.push_str(dir);
        stdout.push('\n');
    }
    let warnings = view
        .blanks
        .iter()
        .map(|blank| {
            format!(
                "paths.{} is blank; using default '{}' (blanking a path does \
                 not disable a doc-type)",
                blank.path_key, blank.default
            )
        })
        .collect();
    Rendered { stdout, warnings }
}

#[must_use]
pub fn render_unavailable() -> Rendered {
    super::unavailable("## Configured Paths Unavailable")
}
