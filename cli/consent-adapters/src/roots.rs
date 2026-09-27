//! What "inside the repository" means to the consent policy: the config root,
//! and every root of every repository enclosing the working directory.

use std::path::Path;

use config::consent::RepositoryRoots;
use vcs::tracking::RootsAnswer;

/// The roots the command runner's `PATH` filter and working directory, and
/// the path checks, judge against.
#[must_use]
pub fn repository_roots(config_root: &Path, cwd: &Path) -> RepositoryRoots {
    beside_config_root(config_root, vcs_adapters::repository_roots(cwd))
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

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use vcs::tracking::RootsAnswer;

    use super::beside_config_root;

    #[test]
    fn each_answer_keeps_its_completeness_beside_the_config_root(
    ) -> std::io::Result<()> {
        let config_root = std::env::temp_dir().canonicalize()?;
        for complete in [true, false] {
            let roots = beside_config_root(
                &config_root,
                RootsAnswer {
                    roots: vec![PathBuf::from("/work/repo")],
                    complete,
                },
            );

            assert_eq!(roots.is_complete(), complete);
            assert!(roots.contains(&config_root));
            assert!(roots.contains(Path::new("/work/repo/bin")));
        }
        Ok(())
    }
}
