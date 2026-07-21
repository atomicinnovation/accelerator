//! Pure renderers: functions of a view to the bytes a subcommand emits.
//!
//! A renderer returns [`Rendered`] — the stdout string plus any stderr warnings
//! held separately, so warning *content* is unit-assertable and the handler can
//! guarantee warnings precede the buffered stdout. Each block renderer also
//! carries a `render_unavailable` for its `--fail-safe` degraded notice, so the
//! notice bytes live beside the success output they mirror.

pub mod agents;
pub mod context;
pub mod instructions;
pub mod paths;

/// A rendered subcommand output.
pub struct Rendered {
    pub stdout: String,
    pub warnings: Vec<String>,
}

impl Rendered {
    #[must_use]
    pub const fn new(stdout: String) -> Self {
        Self {
            stdout,
            warnings: Vec::new(),
        }
    }
}

/// Writes a [`Rendered`]: warnings to stderr first so they always precede the
/// buffered stdout, then the stdout bytes verbatim.
pub fn emit(rendered: &Rendered) {
    for warning in &rendered.warnings {
        eprintln!("{warning}");
    }
    print!("{}", rendered.stdout);
}
