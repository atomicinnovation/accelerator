//! The working-copy check a re-key must pass: an apply has to be undoable
//! through version control.

use std::path::Path;

use vcs::guarded_working_copy::observe_guarded;
use vcs::guarded_working_copy::working_copy_seen_from;
use vcs::guarded_working_copy::GuardedObservation;
use vcs::RepositoryProbe;
use work::rekey::outcome::NotRekeyed;
use work::rekey::write_set::Base;
use work::rekey::write_set::WriteSet;

/// A lock sentinel or atomic-write temporary a killed run can leave beside
/// corpus files; neither holds user content.
fn is_machinery(path: &str) -> bool {
    let mut segments = path.rsplit('/');
    let name = segments.next().unwrap_or_default();
    let in_a_lockdir = segments
        .next()
        .is_some_and(|parent| parent.ends_with(".lockdir"));
    store::is_atomic_temporary(name)
        || (in_a_lockdir && store::lock::is_lock_sentinel(name))
}

/// A version-control tool reports a directory nothing in it is tracked in
/// as the directory alone, so one holding a path the run writes is the
/// run's.
fn written_by_the_run(path: &str, exempt: &WriteSet) -> bool {
    exempt.recorded(path).is_some()
        || (path.ends_with('/')
            && exempt.iter().any(|(written, _)| written.starts_with(path)))
}

/// The base commits of the working copy at `root`, once no path under
/// `scopes` is dirty except those `exempt` names.
///
/// # Errors
///
/// [`NotRekeyed::WorkingCopyUnprobeable`] when `root` is under no version
/// control or its status cannot be read, and
/// [`NotRekeyed::DirtyWorkingCopy`] naming every other dirty path.
pub fn guarded_working_copy(
    root: &Path,
    repository: &dyn RepositoryProbe,
    scopes: &[String],
    exempt: &WriteSet,
) -> Result<Base, NotRekeyed> {
    let probed = working_copy_seen_from(repository, root).and_then(|state| {
        state.ok_or_else(|| {
            kernel::Error::Failed(format!(
                "{} is under no version control",
                root.display()
            ))
        })
    });
    let scopes: Vec<&str> = scopes.iter().map(String::as_str).collect();
    let state = match observe_guarded(probed, &scopes) {
        GuardedObservation::Observed(state) => state,
        GuardedObservation::Unprobeable { reason } => {
            return Err(NotRekeyed::WorkingCopyUnprobeable(reason));
        }
    };
    let dirty: Vec<String> = state
        .dirty_paths
        .into_iter()
        .filter(|path| !is_machinery(path))
        .filter(|path| !written_by_the_run(path, exempt))
        .collect();
    if dirty.is_empty() {
        Ok(Base::new(state.base_commits))
    } else {
        Err(NotRekeyed::DirtyWorkingCopy(dirty))
    }
}
