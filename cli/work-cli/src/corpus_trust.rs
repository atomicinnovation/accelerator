//! Refusing to read or write IDs over a corpus whose IDs cannot be trusted.

use std::path::Path;

use work::id_scheme::IdScheme;
use work_adapters::corpus_trust::refuse_untrusted_under;

use crate::exit_codes;

/// A command's refusal over an untrusted corpus: its stderr line and exit
/// code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Untrusted {
    pub message: String,
    pub code: u8,
}

/// Refuses when the work items under `work_dir` cannot be trusted for their
/// IDs, naming paths relative to `root`.
///
/// # Errors
///
/// [`Untrusted`] carrying `E_REKEY_REQUIRED` or `E_WORK_ITEM_MALFORMED`, and
/// exit `4`.
pub fn refuse_untrusted_corpus(
    scheme: &IdScheme,
    root: &Path,
    work_dir: &Path,
) -> Result<(), Untrusted> {
    refuse_untrusted_under(scheme, work_dir).map_err(|refusal| Untrusted {
        message: format!("Error: {}", refusal.relative_to(root)),
        code: exit_codes::UNRESOLVED,
    })
}
