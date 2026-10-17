//! Beginning a guarded command's identity run at its root, and how a
//! refusal to begin is reported.

use std::path::Path;

use corpus_adapters::FileCorpusStore;
use work::rekey::outcome::NotRekeyed;
use work_adapters::identity_run::begin_identity_run;
use work_adapters::identity_run::waiting_line;
use work_adapters::identity_run::IdentityRunGuard;
use work_adapters::identity_run::IdentityRunRefusal;
use work_adapters::identity_run_lock::IdentityRunLock;
use work_adapters::rekey::run_plan_store::FileRunPlanStore;

use crate::exit_codes;

/// Begins `command`'s identity run in the repository at a root, the seam a
/// test replaces to watch how often a command begins one.
pub type IdentityRunStart<'a> =
    dyn Fn(&Path, &str) -> Result<IdentityRunGuard, IdentityRunRefusal> + 'a;

/// Takes the identity-run lock under `root`'s state directory, printing a
/// waiting line while another command holds it, and refuses while a re-key
/// is in progress or its plan cannot be read.
///
/// # Errors
///
/// [`IdentityRunRefusal`] when the run may not begin; nothing has been
/// read or written.
pub fn begin(
    root: &Path,
    command: &str,
) -> Result<IdentityRunGuard, IdentityRunRefusal> {
    let state_dir = root.join(crate::sync::STATE_DIR);
    let store = FileCorpusStore::new(root);
    let lock = IdentityRunLock::new(&state_dir, &store);
    let plans = FileRunPlanStore::new(&state_dir, root, &store, &store);
    begin_identity_run(&lock, &plans, command, &|holder| {
        eprintln!("{}", waiting_line(holder));
    })
}

/// The stderr line and exit code a guarded command reports a refusal with.
#[must_use]
pub fn refused(refusal: IdentityRunRefusal) -> (String, u8) {
    match refusal {
        IdentityRunRefusal::Lock(error) => (
            format!("Error: the identity-run lock could not be taken: {error}"),
            exit_codes::ERROR,
        ),
        other => {
            let refusal = NotRekeyed::from(other);
            (format!("Error: {refusal}"), refusal.exit_code())
        }
    }
}
