//! `vcs log`: the five most recent commits as short id plus subject, in the
//! ADR-0066 format, computed in-process for git and jj.

use std::path::Path;

use vcs::VcsReporter;

use crate::report;

/// Never fails — folds any adapter failure to `(log unavailable)`.
#[must_use]
pub fn run(start: &Path, reporter: &dyn VcsReporter) -> String {
    report::run(
        start,
        reporter,
        |reporter, dir, kind| reporter.log_report(dir, kind),
        vcs::log::render,
        "log",
        "(log unavailable)",
    )
}
