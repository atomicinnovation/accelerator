//! The outbound VCS adapters, and the composition root over them.
//!
//! [`library`] answers every port a repository is probed through — including
//! [`facts`] and the `status`/`log` renderings — by reading both idioms in the
//! calling process, and carries the taxonomy queries besides. No adapter spawns
//! a child: the crate carries a crate-wide import rule denying `std::process`.
//!
//! The ancestor walk and the marker reading live in a second, private module
//! that [`library`] delegates *to*.

pub mod library;
mod markers;

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
