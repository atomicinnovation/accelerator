//! Refusing to read or write IDs over a corpus holding an item re-key would
//! move.

use std::path::Path;

use work::id_scheme::IdScheme;
use work::rekey::convergence::refuse_divergence;
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

/// Refuses when any item under `work_dir` is off the scheme, naming paths
/// relative to `root`. A corpus that cannot be listed is left to the
/// command's own reading to report.
///
/// # Errors
///
/// [`Diverged`] carrying `E_REKEY_REQUIRED` and exit `4`.
pub fn refuse_diverged_corpus(
    scheme: &IdScheme,
    root: &Path,
    work_dir: &Path,
) -> Result<(), Diverged> {
    let Ok(files) = FilesystemWorkItemFiles::new(work_dir).files() else {
        return Ok(());
    };
    refuse_divergence(scheme, &identities(&files)).map_err(|refusal| Diverged {
        message: format!("Error: {}", refusal.relative_to(root)),
        code: exit_codes::UNRESOLVED,
    })
}
