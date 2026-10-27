//! The working-copy changes a command that rewrites accelerator state must
//! see before it runs, and the one override that lets it run regardless.

use std::path::Path;

use crate::RepositoryProbe;
use crate::WorkingCopyState;

/// The repo-relative prefixes holding accelerator state.
///
/// A plain string prefix, so `.claude/accelerator` covers
/// `.claude/accelerator.md` and `.claude/accelerator.local.md` but not
/// `.claude/settings.json`.
pub const GUARDED_SCOPES: [&str; 3] =
    ["meta/", ".claude/accelerator", ".accelerator/"];

pub const FORCE_VARIABLE: &str = "ACCELERATOR_FORCE";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardedObservation {
    Observed(WorkingCopyState),
    Unprobeable { reason: String },
}

/// The probed working copy with its dirty paths narrowed to `scopes`.
///
/// What an unprobeable working copy means is the caller's policy: one
/// command may treat it as clean, another must refuse.
#[must_use]
pub fn observe_guarded(
    probed: Result<WorkingCopyState, kernel::Error>,
    scopes: &[&str],
) -> GuardedObservation {
    match probed {
        Ok(state) => GuardedObservation::Observed(WorkingCopyState {
            base_commits: state.base_commits,
            dirty_paths: state
                .dirty_paths
                .into_iter()
                .filter(|path| {
                    scopes.iter().any(|scope| path.starts_with(scope))
                })
                .collect(),
        }),
        Err(error) => GuardedObservation::Unprobeable {
            reason: error.to_string(),
        },
    }
}

/// The working copy containing `root`, its dirty paths relative to `root`,
/// which may sit below the repository's own root; `None` when `root` is
/// under no version control.
///
/// # Errors
///
/// [`kernel::Error`] when `root` lies outside the repository it resolves
/// to, or the working copy cannot be read.
pub fn working_copy_seen_from(
    probe: &dyn RepositoryProbe,
    root: &Path,
) -> Result<Option<WorkingCopyState>, kernel::Error> {
    let Some(facts) = probe.facts_at(root) else {
        return Ok(None);
    };
    let config_root = probe.canonical(root);
    let within = config_root
        .strip_prefix(probe.canonical(&facts.root))
        .map_err(|_| {
            kernel::Error::Failed(format!(
                "{} lies outside the repository at {}",
                root.display(),
                facts.root.display()
            ))
        })?;
    probe
        .working_copy_state(&facts.root, facts.kind)
        .map(|state| Some(state.seen_from(within)))
}

/// Whether a [`FORCE_VARIABLE`] value asks to force.
///
/// Only `1` or `true`, ignoring case, force: a generic name is easily left
/// exported, so `0` and `false` must not.
#[must_use]
pub fn forces(value: Option<&str>) -> bool {
    value
        .is_some_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use super::forces;
    use super::observe_guarded;
    use super::working_copy_seen_from;
    use super::GuardedObservation;
    use super::GUARDED_SCOPES;
    use crate::RepoFacts;
    use crate::RepositoryProbe;
    use crate::VcsKind;
    use crate::WorkingCopyState;
    use crate::WorkingCopyStateProbe;

    fn state(paths: &[&str]) -> WorkingCopyState {
        WorkingCopyState {
            base_commits: vec!["abc123".to_owned()],
            dirty_paths: paths.iter().map(|path| (*path).to_owned()).collect(),
        }
    }

    fn observed(paths: &[&str]) -> GuardedObservation {
        GuardedObservation::Observed(state(paths))
    }

    #[test]
    fn only_dirty_paths_under_a_guarded_scope_are_reported() {
        assert_eq!(
            observe_guarded(
                Ok(state(&["meta/a.md", "src/main.rs", ".accelerator/b"])),
                &GUARDED_SCOPES,
            ),
            observed(&["meta/a.md", ".accelerator/b"])
        );
    }

    #[test]
    fn a_legacy_config_file_is_reported_by_prefix() {
        assert_eq!(
            observe_guarded(
                Ok(state(&[".claude/accelerator.md"])),
                &GUARDED_SCOPES
            ),
            observed(&[".claude/accelerator.md"])
        );
    }

    #[test]
    fn a_sibling_path_outside_every_guarded_scope_is_not_reported() {
        assert_eq!(
            observe_guarded(
                Ok(state(&[".claude/settings.json"])),
                &GUARDED_SCOPES
            ),
            observed(&[])
        );
    }

    #[test]
    fn a_failed_probe_is_unprobeable_naming_the_failure() {
        assert_eq!(
            observe_guarded(
                Err(kernel::Error::Failed("corrupt index".to_owned())),
                &GUARDED_SCOPES,
            ),
            GuardedObservation::Unprobeable {
                reason: "corrupt index".to_owned()
            }
        );
    }

    struct Probed {
        root: Option<&'static str>,
        state: Result<WorkingCopyState, String>,
    }

    impl WorkingCopyStateProbe for Probed {
        fn working_copy_state(
            &self,
            _root: &Path,
            _kind: VcsKind,
        ) -> Result<WorkingCopyState, kernel::Error> {
            self.state.clone().map_err(kernel::Error::Failed)
        }
    }

    impl RepositoryProbe for Probed {
        fn facts_at(&self, _start: &Path) -> Option<RepoFacts> {
            self.root.map(|root| RepoFacts {
                root: PathBuf::from(root),
                name: "repo".to_owned(),
                kind: VcsKind::Git,
                revision: None,
            })
        }

        fn user_name_at(&self, _start: &Path) -> Option<String> {
            None
        }

        fn canonical(&self, path: &Path) -> PathBuf {
            path.strip_prefix("/var").map_or_else(
                |_| path.to_owned(),
                |rest| Path::new("/private/var").join(rest),
            )
        }
    }

    fn probed(root: Option<&'static str>, paths: &[&str]) -> Probed {
        Probed {
            root,
            state: Ok(state(paths)),
        }
    }

    #[test]
    fn a_root_under_no_version_control_has_no_working_copy() {
        assert_eq!(
            working_copy_seen_from(&probed(None, &[]), Path::new("/var/x"))
                .map_err(|error| error.to_string()),
            Ok(None)
        );
    }

    #[test]
    fn a_root_below_the_repository_sees_only_its_own_paths_relative_to_it() {
        let repository = probed(
            Some("/private/var/repo"),
            &["sub/meta/a.md", "meta/elsewhere.md"],
        );

        assert_eq!(
            working_copy_seen_from(&repository, Path::new("/var/repo/sub"))
                .map_err(|error| error.to_string()),
            Ok(Some(state(&["meta/a.md"])))
        );
    }

    #[test]
    fn a_root_outside_the_repository_it_resolved_to_is_an_error_naming_both() {
        let outcome = working_copy_seen_from(
            &probed(Some("/elsewhere/repo"), &[]),
            Path::new("/var/repo"),
        );

        assert_eq!(
            outcome.map_err(|error| error.to_string()),
            Err("/var/repo lies outside the repository at /elsewhere/repo"
                .to_owned())
        );
    }

    #[test]
    fn a_failed_state_read_is_an_error() {
        let repository = Probed {
            root: Some("/var/repo"),
            state: Err("corrupt index".to_owned()),
        };

        assert_eq!(
            working_copy_seen_from(&repository, Path::new("/var/repo"))
                .map_err(|error| error.to_string()),
            Err("corrupt index".to_owned())
        );
    }

    #[test]
    fn only_one_or_true_forces_ignoring_case() {
        for value in ["1", "true", "TRUE", "True"] {
            assert!(forces(Some(value)), "{value}");
        }
        for value in ["0", "false", "", "yes", "2"] {
            assert!(!forces(Some(value)), "{value}");
        }
        assert!(!forces(None));
    }
}
