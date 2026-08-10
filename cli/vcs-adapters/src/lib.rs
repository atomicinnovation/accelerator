//! The outbound VCS adapters, and the composition root over them.
//!
//! [`library`] answers every port a repository is probed through — including
//! [`facts`] — by reading both idioms in the calling process, and carries the
//! taxonomy queries besides. [`subprocess`] survives only for `status`/`log`,
//! the two human-facing renderings with no library equivalent. Keeping them
//! apart is what lets [`library`] carry an import rule denying `std::process`
//! while [`subprocess`] spawns by design.
//!
//! The ancestor walk and the marker reading live in a third, private module
//! that [`library`] delegates *to*.

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
