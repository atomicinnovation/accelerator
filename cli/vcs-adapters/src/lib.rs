//! The outbound VCS adapters, and the composition root that picks one.
//!
//! Two implementations of the same two ports live here, side by side and
//! deliberately in separate files:
//!
//! - [`subprocess`] runs the `jj`/`git` binaries in a child process, bounded by
//!   a time cap and a scrubbed environment. It is what [`facts`] uses.
//! - [`library`] reads both idioms in the calling process through `gix` and
//!   `jj-lib`, and additionally carries the taxonomy queries the subprocess pair
//!   has no equivalent for. It ships unwired: nothing here routes a caller to
//!   it yet.
//!
//! Keeping them apart is what lets [`library`] carry a cargo-pup rule denying
//! `std::process` while [`subprocess`] spawns by design — one module could not
//! be both. This crate root holds no adapter code of its own, so the file layout
//! now says the same thing `pup.ron` does, and retiring the subprocess pair is a
//! file deletion.
//!
//! What both agree on — the ancestor walk and the marker reading — lives in a
//! third, private module that each delegates *to*, so removing either adapter
//! does not strand it.

pub mod library;
mod markers;
pub mod subprocess;

use std::path::Path;

use vcs::RepoFacts;

use crate::subprocess::{CommandProbe, MarkerWalkRoot};

/// The facts for the repository containing `start`, probed against the real
/// filesystem and the real VCS binaries.
#[must_use]
pub fn facts(start: &Path) -> Option<RepoFacts> {
    vcs::facts(start, &MarkerWalkRoot, &CommandProbe::new())
}

#[cfg(test)]
mod tests {
    use super::facts;

    #[test]
    fn a_tree_with_no_marker_has_no_facts(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // The marker walk needs no VCS binary, so this stays outside the
        // `bash-parity` detection fixtures and runs on a bare machine.
        let loose = tempfile::Builder::new().prefix("vcs-loose-").tempdir()?;

        assert_eq!(
            facts(loose.path()),
            None,
            "a tree with no .jj or .git must be representable as absent"
        );

        Ok(())
    }
}
