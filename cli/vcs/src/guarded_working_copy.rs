//! The working-copy changes a command that rewrites accelerator state must
//! see before it runs, and the one override that lets it run regardless.

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
    use super::forces;
    use super::observe_guarded;
    use super::GuardedObservation;
    use super::GUARDED_SCOPES;
    use crate::WorkingCopyState;

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
