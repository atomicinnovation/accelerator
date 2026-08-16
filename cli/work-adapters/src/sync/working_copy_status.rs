//! The real, `vcs`/`vcs-adapters`-backed implementation of
//! [`WorkingCopyStatus`].
//!
//! One whole-tree diff, taken once at construction and answered from memory
//! thereafter, so a corpus-sized run probes the repository exactly once. The
//! repository is located and classified by `vcs` itself, so a colocated
//! checkout is read through jj and a jj secondary workspace roots at its own
//! working copy.
//!
//! Every unanswerable case — no repository above the start path, a diff the
//! pinned libraries could not compute, a path outside the working copy —
//! folds to [`Dirtiness::Unknown`] rather than a guess.

use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use tracing::warn;
use vcs_adapters::library::InProcessProbe;
use work::sync::Dirtiness;

use crate::sync::fetch::WorkingCopyStatus;

pub struct VcsWorkingCopyStatus {
    root: PathBuf,
    dirty: Option<BTreeSet<String>>,
}

impl VcsWorkingCopyStatus {
    /// Diffs the working copy containing `start`.
    #[must_use]
    pub fn probed_from(start: &Path) -> Self {
        let Some(facts) = vcs_adapters::facts(start) else {
            return Self {
                root: start.to_path_buf(),
                dirty: None,
            };
        };
        let dirty = InProcessProbe
            .dirty_paths(&facts.root, facts.kind)
            .map(|paths| paths.into_iter().collect())
            .map_err(|error| {
                warn!(
                    root = %facts.root.display(),
                    %error,
                    "could not compute the working-copy diff; every item's \
                     dirtiness is unknown"
                );
            })
            .ok();
        Self {
            root: facts.root,
            dirty,
        }
    }
}

impl WorkingCopyStatus for VcsWorkingCopyStatus {
    fn is_dirty(&self, path: &Path) -> Dirtiness {
        let Some(dirty) = &self.dirty else {
            return Dirtiness::Unknown;
        };
        let Some(relative) = repo_relative(&self.root, path) else {
            return Dirtiness::Unknown;
        };
        if dirty.contains(&relative) {
            Dirtiness::Dirty
        } else {
            Dirtiness::Clean
        }
    }
}

/// `vcs-adapters` reports canonicalised roots and forward-slash-separated
/// repo-relative paths; the caller's path arrives however the corpus walk
/// built it, so both sides are brought to that shape before comparison.
fn repo_relative(root: &Path, path: &Path) -> Option<String> {
    let canonical = path.canonicalize();
    let path = canonical.as_deref().unwrap_or(path);
    let relative = path.strip_prefix(root).ok()?;
    Some(
        relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
    )
}
