//! Beginning a guarded command's identity run at its root, and how a
//! refusal to begin is reported.

use std::path::Path;

use corpus_adapters::FileCorpusStore;
use work::identity_run::begin_identity_run;
use work::identity_run::waiting_line;
use work::identity_run::IdentityRunGuard;
use work::identity_run::IdentityRunRefusal;
use work::rekey::outcome::NotRekeyed;
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

/// The stderr line and exit code a guarded command reports a refusal with,
/// naming any path relative to `root`.
#[must_use]
pub fn refused(refusal: IdentityRunRefusal, root: &Path) -> (String, u8) {
    match refusal {
        IdentityRunRefusal::Lock(error) => (
            format!("Error: the identity-run lock could not be taken: {error}"),
            exit_codes::ERROR,
        ),
        other => {
            let refusal = NotRekeyed::from(other);
            let code = refusal.exit_code();
            (format!("Error: {}", refusal.relative_to(root)), code)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use work::identity_run::IdentityRunRefusal;
    use work::rekey::run_plan::Unreadability;
    use work::rekey::run_plan::UnreadablePlan;

    use super::refused;

    #[test]
    fn an_unreadable_plan_is_named_relative_to_the_root() {
        let root = Path::new("/repo");

        let (message, _) = refused(
            IdentityRunRefusal::RekeyPlanUnreadable(UnreadablePlan {
                path: root.join(".accelerator/state/rekey/run-plan.json"),
                cause: Unreadability::Malformed("bad".to_owned()),
            }),
            root,
        );

        assert!(
            message.contains(" .accelerator/state/rekey/run-plan.json "),
            "{message}"
        );
        assert!(!message.contains("/repo"), "{message}");
    }
}
