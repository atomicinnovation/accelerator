//! The `## Effective Configuration` table and its `--fail-safe` notice.

use crate::config_command::core::dump::{Cell, Row, Source};
use crate::config_command::render::Rendered;

#[must_use]
pub fn render(rows: &[Row]) -> Rendered {
    let mut stdout = String::from(
        "## Effective Configuration\n\n\
         | Key | Value | Source |\n\
         |-----|-------|--------|\n",
    );
    for row in rows {
        stdout.push_str("| `");
        stdout.push_str(&row.key);
        stdout.push_str("` | ");
        stdout.push_str(&cell(&row.cell));
        stdout.push_str(" | ");
        stdout.push_str(source(row.source));
        stdout.push_str(" |\n");
    }
    Rendered::new(stdout)
}

fn cell(cell: &Cell) -> String {
    match cell {
        Cell::Value(value) => format!("`{value}`"),
        Cell::Hidden => "*(set — hidden)*".to_owned(),
        Cell::NotSet => "*(not set)*".to_owned(),
    }
}

const fn source(source: Source) -> &'static str {
    match source {
        Source::Local => "local (.accelerator/config.local.md)",
        Source::Team => "team (.accelerator/config.md)",
        Source::Default => "default",
    }
}

#[must_use]
pub fn render_unavailable() -> Rendered {
    Rendered::new("## Effective Configuration Unavailable\n".to_owned())
}
