//! The `design.browser_path` hatch's hand-off from the composition root, which
//! owns the consent policy, to the runtime resolution, which knows no config.

use std::path::PathBuf;

/// The browser the consent policy admitted, and what it reported on the way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HatchDecision {
    /// The canonical executable to launch, or `None` for the bundled browser.
    pub browser: Option<PathBuf>,
    /// Rendered refusals, never fatal: a refused hatch falls back to the
    /// bundled browser.
    pub warnings: Vec<String>,
    /// The rendered notice that an environment override chose the browser.
    pub notice: Option<String>,
}
