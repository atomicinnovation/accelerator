//! A re-key runs only over a working copy whose guarded paths are clean, or
//! dirty only where the run itself has written.

use std::path::Path;
use std::path::PathBuf;

use vcs::RepoFacts;
use vcs::RepositoryProbe;
use vcs::VcsKind;
use vcs::WorkingCopyState;
use vcs::WorkingCopyStateProbe;
use work::rekey::outcome::NotRekeyed;
use work::rekey::write_set::Recorded;
use work::rekey::write_set::StartState;
use work::rekey::write_set::WriteSet;
use work_adapters::rekey::working_copy::guarded_working_copy;

struct ScriptedRepository {
    root: Option<PathBuf>,
    state: Result<WorkingCopyState, String>,
}

impl WorkingCopyStateProbe for ScriptedRepository {
    fn working_copy_state(
        &self,
        _root: &Path,
        _kind: VcsKind,
    ) -> Result<WorkingCopyState, kernel::Error> {
        self.state.clone().map_err(kernel::Error::Failed)
    }
}

impl RepositoryProbe for ScriptedRepository {
    fn facts_at(&self, _start: &Path) -> Option<RepoFacts> {
        self.root.clone().map(|root| RepoFacts {
            root,
            name: "repo".to_owned(),
            kind: VcsKind::Git,
            revision: None,
        })
    }

    fn user_name_at(&self, _start: &Path) -> Option<String> {
        None
    }
}

fn scopes(extra: &[&str]) -> Vec<String> {
    vcs::guarded_working_copy::GUARDED_SCOPES
        .iter()
        .chain(extra)
        .map(|scope| (*scope).to_owned())
        .collect()
}

fn absent_at(paths: &[&str]) -> WriteSet {
    WriteSet::default().extended_with(paths.iter().map(|path| {
        (
            (*path).to_owned(),
            Recorded {
                state: StartState::Absent,
                base: None,
            },
        )
    }))
}

#[test]
fn a_failed_probe_refuses_naming_it() {
    let repository = ScriptedRepository {
        root: Some(PathBuf::from("/repo")),
        state: Err("the index is locked".to_owned()),
    };

    let checked = guarded_working_copy(
        Path::new("/repo"),
        &repository,
        &scopes(&[]),
        &WriteSet::default(),
    );

    assert!(
        matches!(
            &checked,
            Err(NotRekeyed::WorkingCopyUnprobeable(reason))
                if reason.contains("the index is locked")
        ),
        "{checked:?}"
    );
}

#[test]
fn scoped_dirty_paths_refuse_and_others_are_ignored() {
    let repository = ScriptedRepository {
        root: Some(PathBuf::from("/repo")),
        state: Ok(WorkingCopyState {
            base_commits: vec!["c1".to_owned()],
            dirty_paths: vec![
                "README.md".to_owned(),
                "meta/work/0042-a.md".to_owned(),
            ],
        }),
    };

    let checked = guarded_working_copy(
        Path::new("/repo"),
        &repository,
        &scopes(&[]),
        &WriteSet::default(),
    );

    assert_eq!(
        checked,
        Err(NotRekeyed::DirtyWorkingCopy(vec![
            "meta/work/0042-a.md".to_owned()
        ]))
    );
}

#[test]
fn an_untracked_directory_holding_only_write_set_targets_is_exempt() {
    let repository = ScriptedRepository {
        root: Some(PathBuf::from("/repo")),
        state: Ok(WorkingCopyState {
            base_commits: vec!["c1".to_owned()],
            dirty_paths: vec!["meta/work/drafts/".to_owned()],
        }),
    };

    let checked = guarded_working_copy(
        Path::new("/repo"),
        &repository,
        &scopes(&[]),
        &absent_at(&["meta/work/drafts/draft-k3x9mq-idea.md"]),
    );

    assert!(checked.is_ok(), "{checked:?}");
}

mod in_a_real_repository {
    use std::fs;
    use std::path::Path;

    use corpus::lock::ExclusiveLock as _;
    use corpus::lock::LockName;
    use corpus_adapters::FileCorpusStore;
    use corpus_adapters::LockdirLock;
    use vcs_adapters::library::InProcessProbe;
    use vcs_test_support::hermetic::assert_git_is_recent_enough;
    use vcs_test_support::hermetic::assert_jj_matches;
    use vcs_test_support::hermetic::assert_no_repository_ancestor;
    use vcs_test_support::hermetic::Hermetic;
    use work::rekey::outcome::NotRekeyed;
    use work::rekey::run_plan::ProposedPlan;
    use work::rekey::run_plan::RunPlan;
    use work::rekey::write_set::Base;
    use work::rekey::write_set::WriteSet;
    use work_adapters::identity_run::IdentityRunAttempt;
    use work_adapters::identity_run::IdentityRunPort as _;
    use work_adapters::identity_run_lock::IdentityRunLock;
    use work_adapters::rekey::run_plan_store::FileRunPlanStore;
    use work_adapters::rekey::run_plan_store::RunPlanStore as _;
    use work_adapters::rekey::working_copy::guarded_working_copy;

    use super::absent_at;
    use super::scopes;

    type TestError = Box<dyn std::error::Error>;

    #[derive(Clone, Copy)]
    enum Vcs {
        Git,
        Jj,
    }

    struct Repo {
        dir: tempfile::TempDir,
    }

    impl Repo {
        fn committed(vcs: Vcs) -> Result<Self, TestError> {
            let repo = Self {
                dir: tempfile::tempdir()?,
            };
            repo.write("meta/work/0042-a.md", "---\nid: \"0042\"\n---\n")?;
            repo.write("docs/notes/x.md", "Notes.\n")?;
            repo.write("README.md", "Readme.\n")?;
            let env = Hermetic::rooted_at(repo.root())?;
            match vcs {
                Vcs::Git => {
                    assert_git_is_recent_enough()?;
                    env.git(&["init", "--quiet"], repo.root())?;
                    env.git(&["add", "."], repo.root())?;
                    env.git(&["commit", "--quiet", "-m", "init"], repo.root())?;
                }
                Vcs::Jj => {
                    assert_jj_matches("0.43.0")?;
                    env.jj(&["git", "init", "--no-colocate"], repo.root())?;
                    env.jj(&["commit", "-m", "init"], repo.root())?;
                }
            }
            Ok(repo)
        }

        fn root(&self) -> &Path {
            self.dir.path()
        }

        fn write(&self, path: &str, content: &str) -> Result<(), TestError> {
            let path = self.root().join(path);
            fs::create_dir_all(path.parent().ok_or("a parent")?)?;
            fs::write(path, content)?;
            Ok(())
        }

        fn check(
            &self,
            extra_scopes: &[&str],
            exempt: &WriteSet,
        ) -> Result<Base, NotRekeyed> {
            guarded_working_copy(
                self.root(),
                &InProcessProbe,
                &scopes(extra_scopes),
                exempt,
            )
        }
    }

    fn each_vcs(
        check: impl Fn(Repo) -> Result<(), TestError>,
    ) -> Result<(), TestError> {
        for vcs in [Vcs::Git, Vcs::Jj] {
            check(Repo::committed(vcs)?)?;
        }
        Ok(())
    }

    #[test]
    fn scoped_dirty_paths_refuse() -> Result<(), TestError> {
        each_vcs(|repo| {
            repo.write("meta/work/0042-a.md", "edited\n")?;
            repo.write("README.md", "edited\n")?;

            assert_eq!(
                repo.check(&[], &WriteSet::default()),
                Err(NotRekeyed::DirtyWorkingCopy(vec![
                    "meta/work/0042-a.md".to_owned()
                ]))
            );
            Ok(())
        })
    }

    #[test]
    fn a_clean_working_copy_passes_naming_its_base() -> Result<(), TestError> {
        each_vcs(|repo| {
            let base = repo.check(&[], &WriteSet::default())?;

            assert_eq!(base.commits().len(), 1, "{base:?}");
            Ok(())
        })
    }

    #[test]
    fn a_dirty_path_under_a_configured_corpus_root_outside_meta_refuses(
    ) -> Result<(), TestError> {
        each_vcs(|repo| {
            repo.write("docs/notes/x.md", "edited\n")?;

            assert_eq!(
                repo.check(&["docs/notes/"], &WriteSet::default()),
                Err(NotRekeyed::DirtyWorkingCopy(vec![
                    "docs/notes/x.md".to_owned()
                ]))
            );
            Ok(())
        })
    }

    #[test]
    fn a_repository_without_version_control_refuses() -> Result<(), TestError> {
        let dir = tempfile::tempdir()?;
        assert_no_repository_ancestor(dir.path())?;

        let checked = guarded_working_copy(
            dir.path(),
            &InProcessProbe,
            &scopes(&[]),
            &WriteSet::default(),
        );

        assert!(
            matches!(checked, Err(NotRekeyed::WorkingCopyUnprobeable(_))),
            "{checked:?}"
        );
        Ok(())
    }

    #[test]
    fn an_in_progress_plan_exempts_its_write_set_but_not_other_paths(
    ) -> Result<(), TestError> {
        each_vcs(|repo| {
            repo.write("meta/work/ACC-0042-a.md", "retired\n")?;
            fs::remove_file(repo.root().join("meta/work/0042-a.md"))?;
            repo.write("meta/notes/unrelated.md", "new\n")?;
            let exempt =
                absent_at(&["meta/work/ACC-0042-a.md", "meta/work/0042-a.md"]);

            assert_eq!(
                repo.check(&[], &exempt),
                Err(NotRekeyed::DirtyWorkingCopy(vec![
                    "meta/notes/unrelated.md".to_owned()
                ]))
            );
            Ok(())
        })
    }

    #[test]
    fn a_write_set_target_in_a_directory_the_run_created_is_exempt(
    ) -> Result<(), TestError> {
        each_vcs(|repo| {
            repo.write("meta/work/drafts/draft-k3x9mq-a.md", "retired\n")?;
            let exempt = absent_at(&["meta/work/drafts/draft-k3x9mq-a.md"]);

            let checked = repo.check(&[], &exempt);

            assert!(checked.is_ok(), "{checked:?}");
            Ok(())
        })
    }

    #[test]
    fn the_run_plan_directory_is_ignored_by_git_and_jj() -> Result<(), TestError>
    {
        each_vcs(|repo| {
            let writer = FileCorpusStore::new(repo.root());
            let store = FileRunPlanStore::new(
                &repo.root().join(".accelerator/state"),
                repo.root(),
                &writer,
                &writer,
            );
            store.save(&RunPlan::Proposed(ProposedPlan::new(
                work::rekey::fingerprint::IdentityFingerprint::of(
                    &corpus::WorkItemIdScheme::numeric(),
                    &[],
                ),
                Vec::new(),
            )))?;

            let checked = repo.check(&[], &WriteSet::default());

            assert!(checked.is_ok(), "{checked:?}");
            Ok(())
        })
    }

    #[test]
    fn the_check_passes_while_rekey_holds_its_locks() -> Result<(), TestError> {
        each_vcs(|repo| {
            let writer = FileCorpusStore::new(repo.root());
            let identity_run = IdentityRunLock::new(
                &repo.root().join(".accelerator/state"),
                &writer,
            );
            let IdentityRunAttempt::Acquired(_identity) =
                identity_run.try_acquire("rekey")?
            else {
                return Err("the identity run was held".into());
            };
            let locks = LockdirLock::new(repo.root().join("meta/work"));
            let _retirement = locks.acquire(&LockName::Retirement)?;
            let _create = locks.acquire(&LockName::Create)?;

            let checked = repo.check(&[], &WriteSet::default());

            assert!(checked.is_ok(), "{checked:?}");
            Ok(())
        })
    }

    #[test]
    fn a_stale_for_file_lockdir_and_an_atomic_temporary_are_not_dirt(
    ) -> Result<(), TestError> {
        each_vcs(|repo| {
            repo.write(
                "meta/work/0042-a.md.lockdir/owner.0123456789abcdef",
                "1",
            )?;
            repo.write(
                &format!("meta/work/{}0042-a.md.x1y2", kernel::TEMP_PREFIX),
                "half-written",
            )?;

            let checked = repo.check(&[], &WriteSet::default());

            assert!(checked.is_ok(), "{checked:?}");
            Ok(())
        })
    }
}
