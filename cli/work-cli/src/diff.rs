//! Adapter/binary wiring for `work diff`: this module owns the CLI-level
//! error arms only. Extraction and comparison come from
//! `work::section_diff::differing_sections`, and each differing section's
//! rendering from `work_adapters::diff_shellout::render`.

use std::path::Path;

use work::section_diff::differing_sections;
use work_adapters::diff_shellout::render;

pub enum RunOutcome {
    Rendered(String),
    NonFileArgument,
    DiffUnavailable,
}

/// # Errors
///
/// Never returns `Err` itself; failures are reported through [`RunOutcome`].
#[must_use]
pub fn run(local: &Path, remote: &Path) -> RunOutcome {
    if !local.is_file() || !remote.is_file() {
        return RunOutcome::NonFileArgument;
    }
    let Ok(local_content) = std::fs::read_to_string(local) else {
        return RunOutcome::NonFileArgument;
    };
    let Ok(remote_content) = std::fs::read_to_string(remote) else {
        return RunOutcome::NonFileArgument;
    };

    let diffs = differing_sections(&local_content, &remote_content);
    if diffs.is_empty() {
        return RunOutcome::Rendered(
            "(no differing sections after normalisation)\n".to_owned(),
        );
    }

    let mut out = String::new();
    for diff in &diffs {
        match render(diff) {
            Ok(rendered) => out.push_str(&rendered),
            Err(_) => return RunOutcome::DiffUnavailable,
        }
    }
    RunOutcome::Rendered(out)
}
