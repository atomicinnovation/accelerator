//! `vcs status`: the working-copy change summary in the ADR-0066 format,
//! computed in-process for git and jj.

use std::path::Path;

use vcs::VcsReporter;

use crate::report;

/// Never fails — folds any adapter failure to `(status unavailable)`.
#[must_use]
pub fn run(start: &Path, reporter: &dyn VcsReporter) -> String {
    report::run(
        start,
        reporter,
        |reporter, dir, kind| reporter.status_report(dir, kind),
        vcs::status::render,
        "status",
        "(status unavailable)",
    )
}
