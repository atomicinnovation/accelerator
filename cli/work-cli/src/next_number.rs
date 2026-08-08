//! Adapter/binary wiring for `work next-number`: a thin display wrapper
//! around `work::next_number::allocate` — never writes a file, never
//! commits a number. Exact behavioural match for
//! `work-item-next-number.sh --project <code> --count <count>`.

use std::path::Path;

use ::config::ConfigAccess;
use corpus_adapters::compile_scan_regex;
use corpus_adapters::RegexScanner;
use work::next_number::allocate;
use work::next_number::AllocationError;
use work::resolve::DirectoryLister;
use work_adapters::filesystem::FilesystemLister;

use crate::config::resolve_scheme;
use crate::config::resolve_work_dir;

pub enum RunOutcome {
    Allocated(Vec<String>),
    Overflow {
        partial: Vec<String>,
        message: String,
    },
    Failed(String),
}

fn overflow_message(
    highest: u64,
    highest_file: Option<&str>,
    cap: u64,
    pattern: &str,
) -> String {
    if highest > cap {
        format!(
            "E_PATTERN_OVERFLOW: out-of-width file '{}' has number \
             {highest} exceeding the pattern '{pattern}' cap of {cap}. \
             Rename the stray file or widen the pattern.",
            highest_file.unwrap_or("<unknown>")
        )
    } else {
        format!(
            "E_PATTERN_OVERFLOW: pattern '{pattern}' number space \
             exhausted (highest={highest}, cap={cap}). Archive completed \
             work items or widen the pattern."
        )
    }
}

fn allocation_message(error: &AllocationError, pattern: &str) -> String {
    match error {
        AllocationError::MissingProject => format!(
            "E_PATTERN_MISSING_PROJECT: pattern '{pattern}' contains \
             {{project}} but no value supplied — pass --project or set \
             work.default_project_code"
        ),
        AllocationError::ProjectUnused => format!(
            "E_PATTERN_PROJECT_UNUSED: --project is meaningless for \
             pattern '{pattern}' (no {{project}} token)"
        ),
        AllocationError::Overflow { .. } => {
            unreachable!("Overflow is handled by the caller separately")
        }
    }
}

/// # Errors
///
/// Never returns `Err`; every failure is reported through [`RunOutcome`].
#[must_use]
pub fn run(
    start: &Path,
    config: &dyn ConfigAccess,
    project: Option<&str>,
    count: u32,
) -> RunOutcome {
    let scheme = match resolve_scheme(config) {
        Ok(scheme) => scheme,
        Err(error) => return RunOutcome::Failed(error.to_string()),
    };
    let root = config_adapters::FileConfigStore::discover_root(start);
    let work_dir = match resolve_work_dir(config, &root) {
        Ok(dir) => dir,
        Err(error) => return RunOutcome::Failed(error.to_string()),
    };
    let project = project
        .map(str::to_owned)
        .or_else(|| scheme.default_project_code.clone());

    let filenames = FilesystemLister::new(&work_dir).filenames();
    let scan_regex = match compile_scan_regex(
        &scheme.id_pattern,
        project.as_deref().unwrap_or(""),
    ) {
        Ok(regex) => regex,
        Err(error) => return RunOutcome::Failed(error.to_string()),
    };
    let scanner = match RegexScanner::compile(&scan_regex) {
        Ok(scanner) => scanner,
        Err(error) => return RunOutcome::Failed(error.to_string()),
    };

    match allocate(&scheme, project.as_deref(), count, &filenames, &scanner) {
        Ok(ids) => RunOutcome::Allocated(ids),
        Err(AllocationError::Overflow {
            partial,
            highest,
            highest_file,
            cap,
        }) => RunOutcome::Overflow {
            partial,
            message: overflow_message(
                highest,
                highest_file.as_deref(),
                cap,
                &scheme.id_pattern,
            ),
        },
        Err(other) => {
            RunOutcome::Failed(allocation_message(&other, &scheme.id_pattern))
        }
    }
}
