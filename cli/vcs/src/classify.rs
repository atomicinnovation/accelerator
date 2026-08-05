//! The seven-arm checkout taxonomy, composed over `CheckoutProbe`'s four
//! queries.
//!
//! Mirrors `scripts/vcs-common.sh`'s `classify_checkout` cascade, with the
//! arm order load-bearing: `Colocated` must precede both `Nested*` arms
//! because a true colocated checkout also satisfies both nested predicates.

use std::path::Path;
use std::path::PathBuf;

use crate::checkout::DualRoots;
use crate::checkout::JjRepositoryFacts;
use crate::checkout::JjWorkspaceRole;
use crate::checkout::WorktreeFacts;

/// The checkout kind containing a start directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Classification {
    Main,
    JjSecondary {
        boundary: PathBuf,
        jj_parent: PathBuf,
    },
    GitWorktree {
        boundary: PathBuf,
        git_parent: PathBuf,
    },
    Colocated {
        boundary: PathBuf,
        jj_parent: PathBuf,
        git_parent: PathBuf,
    },
    NestedJjInGit {
        boundary: PathBuf,
        jj_parent: PathBuf,
        git_parent: PathBuf,
    },
    NestedGitInJj {
        boundary: PathBuf,
        jj_parent: PathBuf,
        git_parent: PathBuf,
    },
    None,
}

/// The four `InProcessProbe` queries `classify` needs.
///
/// Narrowed from its full six-query surface: `jj_workspace_root` duplicates
/// `dual_roots().jj`'s walk, and `superproject` is submodule-specific and
/// unused by this cascade.
pub trait CheckoutProbe {
    /// # Errors
    ///
    /// When a repository is present but cannot be opened or configured.
    fn is_bare(&self, start: &Path) -> Result<Option<bool>, kernel::Error>;

    /// # Errors
    ///
    /// When a repository is present but its directories cannot be read.
    fn worktree(
        &self,
        start: &Path,
    ) -> Result<Option<WorktreeFacts>, kernel::Error>;

    /// # Errors
    ///
    /// When the workspace cannot be loaded, or its store does not resolve to
    /// a jj repository layout.
    fn jj_repository(
        &self,
        start: &Path,
    ) -> Result<Option<JjRepositoryFacts>, kernel::Error>;

    /// Never itself an `Err` — see [`DualRoots`].
    fn dual_roots(&self, start: &Path) -> DualRoots;
}

/// Classifies the checkout containing `start`.
///
/// `is_bare`, `worktree`, and `jj_repository` are hard-fallible: an `Err`
/// from any of them means the cascade cannot safely determine an arm at all,
/// so it propagates rather than guessing. `dual_roots`'s two sides are a
/// *comparison*, not a fact lookup — an `Err` on one side means "this side's
/// own boundary is not determinable", degrading the cascade to whichever
/// arm the *other*, still-available side supports (or to `Main`, if neither
/// is), never to a false equality/inequality or to `None`.
///
/// # Errors
///
/// When `is_bare`, `worktree`, or `jj_repository` fails.
pub fn classify(
    start: &Path,
    probe: &dyn CheckoutProbe,
) -> Result<Classification, kernel::Error> {
    let bare = probe.is_bare(start)?;
    let worktree = probe.worktree(start)?;
    let jj_repository = probe.jj_repository(start)?;
    let dual = probe.dual_roots(start);

    let in_git = matches!(bare, Some(false));
    let git_worktree = worktree.as_ref().is_some_and(|facts| facts.linked);
    let git_main_root = worktree
        .as_ref()
        .and_then(|facts| facts.main_worktree_root.clone());
    let git_boundary = dual.git.ok().flatten();

    let in_jj = jj_repository.is_some();
    let jj_secondary = jj_repository
        .as_ref()
        .is_some_and(|facts| matches!(facts.role, JjWorkspaceRole::Secondary));
    let jj_main_root =
        jj_repository.as_ref().map(|facts| facts.main_root.clone());
    let jj_boundary = dual.jj.ok().flatten();

    if !in_jj && !in_git {
        return Ok(Classification::None);
    }

    if jj_secondary && git_worktree {
        if let (Some(boundary), Some(jj_parent), Some(git_parent)) = (
            jj_boundary.clone(),
            jj_main_root.clone(),
            git_main_root.clone(),
        ) {
            return Ok(Classification::Colocated {
                boundary,
                jj_parent,
                git_parent,
            });
        }
    }

    if jj_secondary && in_git {
        if let (Some(boundary), Some(jj_parent), Some(git_parent)) = (
            jj_boundary.clone(),
            jj_main_root.clone(),
            git_main_root.clone(),
        ) {
            if jj_parent != git_parent {
                return Ok(Classification::NestedJjInGit {
                    boundary,
                    jj_parent,
                    git_parent,
                });
            }
        }
    }

    if git_worktree && in_jj {
        if let (Some(boundary), Some(jj_parent), Some(git_parent)) = (
            git_boundary.clone(),
            jj_main_root.clone(),
            git_main_root.clone(),
        ) {
            if jj_parent != git_parent {
                return Ok(Classification::NestedGitInJj {
                    boundary,
                    jj_parent,
                    git_parent,
                });
            }
        }
    }

    if jj_secondary {
        if let (Some(boundary), Some(jj_parent)) = (jj_boundary, jj_main_root) {
            return Ok(Classification::JjSecondary {
                boundary,
                jj_parent,
            });
        }
    }

    if git_worktree {
        if let (Some(boundary), Some(git_parent)) =
            (git_boundary, git_main_root)
        {
            return Ok(Classification::GitWorktree {
                boundary,
                git_parent,
            });
        }
    }

    Ok(Classification::Main)
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;

    use super::classify;
    use super::CheckoutProbe;
    use super::Classification;
    use crate::checkout::DualRoots;
    use crate::checkout::JjRepositoryFacts;
    use crate::checkout::JjWorkspaceRole;
    use crate::checkout::WorktreeFacts;

    type TestError = Box<dyn std::error::Error>;

    #[derive(Default, Clone)]
    struct StubProbe {
        is_bare: Option<Result<Option<bool>, String>>,
        worktree: Option<Result<Option<WorktreeFacts>, String>>,
        jj_repository: Option<Result<Option<JjRepositoryFacts>, String>>,
        dual_git: Option<Result<Option<PathBuf>, String>>,
        dual_jj: Option<Result<Option<PathBuf>, String>>,
    }

    fn to_kernel<T>(result: Result<T, String>) -> Result<T, kernel::Error> {
        result.map_err(kernel::Error::Failed)
    }

    /// `kernel::Error` has no `PartialEq`, so `assert_eq!` cannot compare a
    /// `Result<Classification, kernel::Error>` directly; tests propagate the
    /// error via `?` (into `TestError`) and compare the `Classification`
    /// alone.
    fn classified(probe: &StubProbe) -> Result<Classification, TestError> {
        classify(&start(), probe).map_err(|error| error.to_string().into())
    }

    impl CheckoutProbe for StubProbe {
        fn is_bare(
            &self,
            _start: &Path,
        ) -> Result<Option<bool>, kernel::Error> {
            to_kernel(self.is_bare.clone().unwrap_or(Ok(None)))
        }

        fn worktree(
            &self,
            _start: &Path,
        ) -> Result<Option<WorktreeFacts>, kernel::Error> {
            to_kernel(self.worktree.clone().unwrap_or(Ok(None)))
        }

        fn jj_repository(
            &self,
            _start: &Path,
        ) -> Result<Option<JjRepositoryFacts>, kernel::Error> {
            to_kernel(self.jj_repository.clone().unwrap_or(Ok(None)))
        }

        fn dual_roots(&self, _start: &Path) -> DualRoots {
            DualRoots {
                git: to_kernel(self.dual_git.clone().unwrap_or(Ok(None))),
                jj: to_kernel(self.dual_jj.clone().unwrap_or(Ok(None))),
            }
        }
    }

    fn start() -> PathBuf {
        PathBuf::from("/tmp/somewhere")
    }

    fn worktree_facts(
        linked: bool,
        main_worktree_root: Option<&str>,
    ) -> WorktreeFacts {
        WorktreeFacts {
            linked,
            git_dir: PathBuf::from("/tmp/somewhere/.git"),
            common_dir: PathBuf::from("/tmp/somewhere/.git"),
            main_worktree_root: main_worktree_root.map(PathBuf::from),
        }
    }

    fn jj_facts(role: JjWorkspaceRole, main_root: &str) -> JjRepositoryFacts {
        JjRepositoryFacts {
            role,
            main_root: PathBuf::from(main_root),
        }
    }

    #[test]
    fn neither_vcs_present_classifies_as_none() -> Result<(), TestError> {
        let probe = StubProbe {
            is_bare: Some(Ok(None)),
            jj_repository: Some(Ok(None)),
            ..StubProbe::default()
        };
        assert_eq!(classified(&probe)?, Classification::None);
        Ok(())
    }

    #[test]
    fn a_plain_git_checkout_classifies_as_main() -> Result<(), TestError> {
        let probe = StubProbe {
            is_bare: Some(Ok(Some(false))),
            worktree: Some(Ok(Some(worktree_facts(
                false,
                Some("/tmp/somewhere"),
            )))),
            jj_repository: Some(Ok(None)),
            dual_git: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
            ..StubProbe::default()
        };
        assert_eq!(classified(&probe)?, Classification::Main);
        Ok(())
    }

    #[test]
    fn a_bare_repository_does_not_count_as_present_git() -> Result<(), TestError>
    {
        let probe = StubProbe {
            is_bare: Some(Ok(Some(true))),
            worktree: Some(Ok(Some(worktree_facts(false, None)))),
            jj_repository: Some(Ok(None)),
            dual_git: Some(Ok(None)),
            ..StubProbe::default()
        };
        assert_eq!(classified(&probe)?, Classification::None);
        Ok(())
    }

    #[test]
    fn a_pure_jj_main_workspace_classifies_as_main() -> Result<(), TestError> {
        let probe = StubProbe {
            is_bare: Some(Ok(None)),
            jj_repository: Some(Ok(Some(jj_facts(
                JjWorkspaceRole::Main,
                "/tmp/somewhere",
            )))),
            dual_jj: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
            ..StubProbe::default()
        };
        assert_eq!(classified(&probe)?, Classification::Main);
        Ok(())
    }

    #[test]
    fn a_jj_secondary_workspace_carries_its_boundary_and_parent(
    ) -> Result<(), TestError> {
        let probe = StubProbe {
            is_bare: Some(Ok(None)),
            jj_repository: Some(Ok(Some(jj_facts(
                JjWorkspaceRole::Secondary,
                "/tmp/main",
            )))),
            dual_jj: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
            ..StubProbe::default()
        };
        assert_eq!(
            classified(&probe)?,
            Classification::JjSecondary {
                boundary: PathBuf::from("/tmp/somewhere"),
                jj_parent: PathBuf::from("/tmp/main"),
            }
        );
        Ok(())
    }

    #[test]
    fn a_git_linked_worktree_carries_its_boundary_and_parent(
    ) -> Result<(), TestError> {
        let probe = StubProbe {
            is_bare: Some(Ok(Some(false))),
            worktree: Some(Ok(Some(worktree_facts(true, Some("/tmp/main"))))),
            jj_repository: Some(Ok(None)),
            dual_git: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
            ..StubProbe::default()
        };
        assert_eq!(
            classified(&probe)?,
            Classification::GitWorktree {
                boundary: PathBuf::from("/tmp/somewhere"),
                git_parent: PathBuf::from("/tmp/main"),
            }
        );
        Ok(())
    }

    fn colocated_probe() -> StubProbe {
        StubProbe {
            is_bare: Some(Ok(Some(false))),
            worktree: Some(Ok(Some(worktree_facts(
                true,
                Some("/tmp/git-main"),
            )))),
            jj_repository: Some(Ok(Some(jj_facts(
                JjWorkspaceRole::Secondary,
                "/tmp/jj-main",
            )))),
            dual_git: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
            dual_jj: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
        }
    }

    #[test]
    fn a_secondary_workspace_that_is_also_a_linked_worktree_classifies_as_colocated(
    ) -> Result<(), TestError> {
        assert_eq!(
            classified(&colocated_probe())?,
            Classification::Colocated {
                boundary: PathBuf::from("/tmp/somewhere"),
                jj_parent: PathBuf::from("/tmp/jj-main"),
                git_parent: PathBuf::from("/tmp/git-main"),
            }
        );
        Ok(())
    }

    #[test]
    fn colocated_precedes_both_nested_arms_even_though_both_predicates_also_hold(
    ) -> Result<(), TestError> {
        // The colocated probe also satisfies nested-jj-in-git's predicate
        // (jj_secondary && in_git, with distinct main roots) and
        // nested-git-in-jj's (git_worktree && in_jj, with distinct main
        // roots) — arm order is load-bearing.
        let classification = classified(&colocated_probe())?;
        assert!(matches!(classification, Classification::Colocated { .. }));
        Ok(())
    }

    #[test]
    fn a_jj_secondary_nested_inside_a_distinct_git_checkout_is_nested_jj_in_git(
    ) -> Result<(), TestError> {
        let probe = StubProbe {
            is_bare: Some(Ok(Some(false))),
            worktree: Some(Ok(Some(worktree_facts(
                false,
                Some("/tmp/git-main"),
            )))),
            jj_repository: Some(Ok(Some(jj_facts(
                JjWorkspaceRole::Secondary,
                "/tmp/jj-main",
            )))),
            dual_git: Some(Ok(Some(PathBuf::from("/tmp/git-main")))),
            dual_jj: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
        };
        assert_eq!(
            classified(&probe)?,
            Classification::NestedJjInGit {
                boundary: PathBuf::from("/tmp/somewhere"),
                jj_parent: PathBuf::from("/tmp/jj-main"),
                git_parent: PathBuf::from("/tmp/git-main"),
            }
        );
        Ok(())
    }

    #[test]
    fn a_git_worktree_nested_inside_a_distinct_jj_checkout_is_nested_git_in_jj(
    ) -> Result<(), TestError> {
        let probe = StubProbe {
            is_bare: Some(Ok(Some(false))),
            worktree: Some(Ok(Some(worktree_facts(
                true,
                Some("/tmp/git-main"),
            )))),
            jj_repository: Some(Ok(Some(jj_facts(
                JjWorkspaceRole::Main,
                "/tmp/jj-main",
            )))),
            dual_git: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
            dual_jj: Some(Ok(Some(PathBuf::from("/tmp/jj-main")))),
        };
        assert_eq!(
            classified(&probe)?,
            Classification::NestedGitInJj {
                boundary: PathBuf::from("/tmp/somewhere"),
                jj_parent: PathBuf::from("/tmp/jj-main"),
                git_parent: PathBuf::from("/tmp/git-main"),
            }
        );
        Ok(())
    }

    #[test]
    fn is_bare_failing_propagates_rather_than_guessing_an_arm() {
        let probe = StubProbe {
            is_bare: Some(Err("could not open repository".to_owned())),
            ..StubProbe::default()
        };
        assert!(classify(&start(), &probe).is_err());
    }

    #[test]
    fn worktree_failing_propagates_rather_than_guessing_an_arm() {
        let probe = StubProbe {
            is_bare: Some(Ok(Some(false))),
            worktree: Some(Err("could not read git directories".to_owned())),
            ..StubProbe::default()
        };
        assert!(classify(&start(), &probe).is_err());
    }

    #[test]
    fn jj_repository_failing_propagates_rather_than_guessing_an_arm() {
        let probe = StubProbe {
            is_bare: Some(Ok(None)),
            jj_repository: Some(Err(
                "store layout is not a jj repository".to_owned()
            )),
            ..StubProbe::default()
        };
        assert!(classify(&start(), &probe).is_err());
    }

    #[test]
    fn an_unreadable_jj_boundary_degrades_to_the_git_side_single_vcs_arm(
    ) -> Result<(), TestError> {
        // dual_roots().jj errors even though jj_repository (hard-fallible,
        // already resolved) confirms a secondary workspace; the git side is
        // fully known and the two main roots coincide (so neither nested arm
        // applies either), so the cascade degrades to git-worktree rather
        // than propagating or misreading the jj side as absent.
        let probe = StubProbe {
            is_bare: Some(Ok(Some(false))),
            worktree: Some(Ok(Some(worktree_facts(
                true,
                Some("/tmp/git-main"),
            )))),
            jj_repository: Some(Ok(Some(jj_facts(
                JjWorkspaceRole::Secondary,
                "/tmp/git-main",
            )))),
            dual_git: Some(Ok(Some(PathBuf::from("/tmp/somewhere")))),
            dual_jj: Some(Err(
                "could not canonicalise the workspace root".to_owned()
            )),
        };
        assert_eq!(
            classified(&probe)?,
            Classification::GitWorktree {
                boundary: PathBuf::from("/tmp/somewhere"),
                git_parent: PathBuf::from("/tmp/git-main"),
            }
        );
        Ok(())
    }

    #[test]
    fn an_unreadable_jj_boundary_with_no_git_side_present_degrades_to_main_not_none(
    ) -> Result<(), TestError> {
        // The git side is confirmed absent (`Ok(None)`, not merely unread),
        // and the jj side is unparseable — this must not collapse to `None`,
        // which would misreport "no repository at all" for a location where
        // jj_repository (hard-fallible) confirms a repository IS present.
        let probe = StubProbe {
            is_bare: Some(Ok(None)),
            jj_repository: Some(Ok(Some(jj_facts(
                JjWorkspaceRole::Secondary,
                "/tmp/jj-main",
            )))),
            dual_git: Some(Ok(None)),
            dual_jj: Some(Err(
                "could not canonicalise the workspace root".to_owned()
            )),
            ..StubProbe::default()
        };
        assert_eq!(classified(&probe)?, Classification::Main);
        Ok(())
    }

    #[test]
    fn both_dual_roots_sides_unreadable_with_secondary_and_worktree_confirmed_degrades_to_main(
    ) -> Result<(), TestError> {
        // Neither side's own boundary is determinable, even though both hard
        // signals (jj_repository, worktree) confirm presence — asserted
        // explicitly per the plan's own instruction, rather than left
        // implicit alongside the single-sided cases above.
        let probe = StubProbe {
            is_bare: Some(Ok(Some(false))),
            worktree: Some(Ok(Some(worktree_facts(
                true,
                Some("/tmp/git-main"),
            )))),
            jj_repository: Some(Ok(Some(jj_facts(
                JjWorkspaceRole::Secondary,
                "/tmp/jj-main",
            )))),
            dual_git: Some(Err("could not open the repository".to_owned())),
            dual_jj: Some(Err(
                "could not canonicalise the workspace root".to_owned()
            )),
        };
        assert_eq!(classified(&probe)?, Classification::Main);
        Ok(())
    }

    #[test]
    fn the_classification_enum_has_exactly_seven_variants() {
        // A closed-set guard: fails to compile if a variant is added or
        // removed without this match arm list moving with it.
        let assert_exhaustive =
            |classification: Classification| match classification {
                Classification::Main
                | Classification::JjSecondary { .. }
                | Classification::GitWorktree { .. }
                | Classification::Colocated { .. }
                | Classification::NestedJjInGit { .. }
                | Classification::NestedGitInJj { .. }
                | Classification::None => {}
            };
        assert_exhaustive(Classification::Main);
    }
}
