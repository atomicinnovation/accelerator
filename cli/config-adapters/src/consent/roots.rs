//! What "inside the repository" means to the consent policy: the config root,
//! and every root of every repository enclosing the working directory.

use std::path::Path;

use config::consent::RepositoryRoots;
use vcs::tracking::RepositoryTracking;
use vcs::tracking::RootsAnswer;

/// The roots the command runner's `PATH` filter and working directory, and
/// the path checks, judge against.
#[must_use]
pub fn repository_roots(
    tracking: &dyn RepositoryTracking,
    config_root: &Path,
    cwd: &Path,
) -> RepositoryRoots {
    beside_config_root(config_root, tracking.repository_roots(cwd))
}

fn beside_config_root(
    config_root: &Path,
    answer: RootsAnswer,
) -> RepositoryRoots {
    let mut roots = answer.roots;
    let mut complete = answer.complete;
    match std::fs::canonicalize(config_root) {
        Ok(canonical) if roots.contains(&canonical) => {}
        Ok(canonical) => roots.insert(0, canonical),
        Err(_) => complete = false,
    }
    if complete {
        RepositoryRoots::complete(roots)
    } else {
        RepositoryRoots::incomplete(roots)
    }
}
