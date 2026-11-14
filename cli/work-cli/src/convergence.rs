//! Refusing to read or write IDs over a corpus holding an item re-key would
//! move.

use std::path::Path;

use work::id_scheme::IdScheme;
use work::rekey::convergence::refuse_divergence;
use work::well_formed::refuse_malformed;
use work::work_item_files::identities;
use work::work_item_files::WorkItemFiles;
use work_adapters::filesystem::FilesystemWorkItemFiles;

use crate::exit_codes;

/// A command's refusal over a diverged corpus: its stderr line and exit
/// code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diverged {
    pub message: String,
    pub code: u8,
}

/// Refuses when any item under `work_dir` is off the scheme, or any file
/// there is a work item whose IDs cannot be read, naming paths relative to
/// `root`. A corpus that cannot be listed is left to the
/// command's own reading to report.
///
/// # Errors
///
/// [`Diverged`] carrying `E_REKEY_REQUIRED` or `E_WORK_ITEM_MALFORMED`, and
/// exit `4`.
pub fn refuse_diverged_corpus(
    scheme: &IdScheme,
    root: &Path,
    work_dir: &Path,
) -> Result<(), Diverged> {
    let Ok(files) = FilesystemWorkItemFiles::new(work_dir).files() else {
        return Ok(());
    };
    let refused = |message: String| Diverged {
        message: format!("Error: {message}"),
        code: exit_codes::UNRESOLVED,
    };
    refuse_divergence(scheme, &identities(&files))
        .map_err(|refusal| refused(refusal.relative_to(root).to_string()))?;
    refuse_malformed(&files)
        .map_err(|refusal| refused(refusal.relative_to(root).to_string()))
}
