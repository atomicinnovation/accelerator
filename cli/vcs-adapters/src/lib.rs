//! The outbound VCS adapters, and the composition root that picks one.
//!
//! [`library`] reads both idioms in the calling process and is what [`facts`]
//! uses; it additionally carries the taxonomy queries the subprocess side has
//! no equivalent for. [`subprocess`] runs the `jj`/`git` binaries in a child
//! process. Keeping them apart is what lets [`library`] carry an import rule
//! denying `std::process` while [`subprocess`] spawns by design.
//!
//! What both agree on — the ancestor walk and the marker reading — lives in a
//! third, private module that each delegates *to*.

pub mod library;
mod markers;
pub mod subprocess;

use std::path::Path;

use vcs::RepoFacts;

use crate::library::InProcessProbe;

/// The facts for the repository containing `start`.
#[must_use]
pub fn facts(start: &Path) -> Option<RepoFacts> {
    vcs::facts(start, &InProcessProbe, &InProcessProbe)
}

#[cfg(test)]
mod tests {
    use super::facts;

    #[test]
    fn a_tree_with_no_marker_has_no_facts(
    ) -> Result<(), Box<dyn std::error::Error>> {
        // The marker walk needs no VCS binary, so this runs on a bare machine.
        let loose = tempfile::Builder::new().prefix("vcs-loose-").tempdir()?;

        assert_eq!(
            facts(loose.path()),
            None,
            "a tree with no .jj or .git must be representable as absent"
        );

        Ok(())
    }
}
