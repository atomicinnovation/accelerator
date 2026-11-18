//! Refusing to read or write IDs over a corpus whose IDs cannot be trusted.

use std::path::Path;

use work::id_scheme::IdScheme;
use work_adapters::corpus_trust::refuse_untrusted_under;

use crate::exit_codes;
use crate::refusal::Refusal;

/// Refuses when the work items under `work_dir` cannot be trusted for their
/// IDs, naming paths relative to `root`.
///
/// # Errors
///
/// A [`Refusal`] carrying `E_REKEY_REQUIRED` or `E_WORK_ITEM_MALFORMED`, and
/// exit `4`.
pub fn refuse_untrusted_corpus(
    scheme: &IdScheme,
    root: &Path,
    work_dir: &Path,
) -> Result<(), Refusal> {
    refuse_untrusted_under(scheme, work_dir).map_err(|refusal| Refusal {
        message: format!("Error: {}", refusal.relative_to(root)),
        code: exit_codes::UNRESOLVED,
    })
}
