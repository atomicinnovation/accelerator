//! `vcs root`: the working-copy root of the checkout, resolved in-process
//! for git and jj via `RepoRoot::discover`. In a jj secondary workspace
//! this is the workspace root, not the shared main repository.

use std::path::Path;

use vcs::RepoRoot;

/// # Errors
///
/// When `start` is not inside a repository.
pub fn run<P>(start: &Path, probe: &P) -> Result<String, kernel::Error>
where
    P: RepoRoot,
{
    probe
        .discover(start)
        .map(|root| root.display().to_string())
        .ok_or_else(|| {
            kernel::Error::Failed(format!(
                "not inside a repository (searched from {})",
                start.display()
            ))
        })
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use vcs::RepoRoot;

    use super::run;

    struct StubRoot(Option<PathBuf>);

    impl RepoRoot for StubRoot {
        fn discover(&self, _start: &Path) -> Option<PathBuf> {
            self.0.clone()
        }
    }

    #[test]
    fn prints_the_discovered_working_copy_root(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let probe = StubRoot(Some(PathBuf::from("/tmp/some-repo")));

        assert_eq!(
            run(Path::new("/tmp/some-repo/meta/work"), &probe)?,
            "/tmp/some-repo"
        );
        Ok(())
    }

    #[test]
    fn errors_naming_the_searched_from_path_outside_a_repository(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let probe = StubRoot(None);

        match run(Path::new("/tmp/loose"), &probe) {
            Ok(root) => Err(format!("expected an error, got {root}").into()),
            Err(error) => {
                let message = error.to_string();
                assert!(message.contains("not inside a repository"));
                assert!(message.contains("/tmp/loose"));
                Ok(())
            }
        }
    }
}
