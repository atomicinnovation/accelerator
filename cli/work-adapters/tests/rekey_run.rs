//! The `work rekey` use case: computing, recording and applying a run plan,
//! finishing an interrupted one, and discarding one a revert has undone.

mod support;

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use corpus::lock::ExclusiveLock as _;
use corpus::lock::LockName;
use corpus::store::RecoveryCopies as _;
use corpus::StoreError;
use corpus_adapters::FileCorpusStore;
use corpus_adapters::LockdirLock;
use store::lock::LockOptions;
use work::rekey::fingerprint::IdentityFingerprint;
use work::rekey::outcome::NotRekeyed;
use work::rekey::outcome::SyncBlocker;
use work::rekey::run_plan::InProgressPlan;
use work::rekey::run_plan::RunPlan;
use work::rekey::write_set::Base;
use work::retirement::Retirement;
use work::retirement::RetirementRecord;
use work_adapters::identity_run::HeldIdentityRun;
use work_adapters::identity_run::IdentityRunAttempt;
use work_adapters::identity_run::IdentityRunLockError;
use work_adapters::identity_run::IdentityRunPort;
use work_adapters::rekey::execute::write_set;
use work_adapters::rekey::run::Forcing;
use work_adapters::rekey::run::RekeyAction;
use work_adapters::rekey::run::RekeyReport;
use work_adapters::rekey::run_plan_store::RunPlanStore as _;
use work_adapters::retirement_records::FileRetirementRecords;
use work_adapters::retirement_records::RetirementRecords as _;

use support::halted_at;
use support::renumbered;
use support::request;
use support::restore;
use support::scheme;
use support::Fault;
use support::Mutations;
use support::Ports;
use support::Recorder;
use support::Repo;
use support::ScriptedRepository;
use support::TestError;
use support::FIRST;
use support::FIRST_TARGET;
use support::PLAN;
use support::RUN_PLAN;
use support::SYNCED;
use support::SYNCED_TARGET;

fn clean(repo: &Repo) -> ScriptedRepository {
    ScriptedRepository::clean_at(repo, "c1")
}

fn base(commit: &str) -> Base {
    Base::new(vec![commit.to_owned()])
}

fn discard(
    ports: &Ports<'_>,
    repo: &Repo,
    forcing: Forcing,
    progress: &Recorder,
) -> Result<RekeyReport, NotRekeyed> {
    ports.rekey(
        &request(RekeyAction::Discard, forcing),
        &clean(repo),
        progress,
        None,
    )
}

/// Halts an apply at its `at`th mutation, leaving the plan in progress.
fn halt_at(repo: &Repo, at: usize) -> Result<(), TestError> {
    let halting = Mutations::new(Fault::HaltAt(at));
    let ports = Ports::new(repo, &halting);
    match halted_at(|| ports.apply(&clean(repo))) {
        Err(Some(halted)) if halted == at => Ok(()),
        other => Err(format!("expected a halt at {at}: {other:?}").into()),
    }
}

fn mutations_of_a_whole_run() -> Result<usize, TestError> {
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    Ports::new(&repo, &mutations).apply(&clean(&repo))?;
    Ok(mutations.count())
}

fn converged() -> Result<BTreeMap<String, String>, TestError> {
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    Ports::new(&repo, &mutations).apply(&clean(&repo))?;
    Ok(repo.tree())
}

#[test]
fn every_entry_is_retired_and_the_plan_removed() -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);
    let progress = Recorder::default();

    let report = ports.rekey(
        &request(RekeyAction::Apply, Forcing::Unforced),
        &clean(&repo),
        &progress,
        None,
    )?;

    assert_eq!(report, RekeyReport::Rekeyed { resumed: false });
    assert_eq!(repo.read(RUN_PLAN), None);
    assert_eq!(
        *progress.lines.borrow(),
        vec![
            format!("0041 → ACC-0041\t{FIRST_TARGET}"),
            format!("PP-760 → ACC-0042\t{SYNCED_TARGET}"),
        ]
    );
    Ok(())
}

#[test]
fn a_converged_corpus_rekeys_to_nothing() -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);
    ports.apply(&clean(&repo))?;
    let after = repo.tree();

    assert_eq!(ports.apply(&clean(&repo))?, RekeyReport::AlreadyConverged);
    assert_eq!(repo.tree(), after);
    Ok(())
}

#[test]
fn a_run_killed_at_every_write_converges_like_an_uninterrupted_run(
) -> Result<(), TestError> {
    let total = mutations_of_a_whole_run()?;
    let expected = converged()?;
    assert!(total > 0);

    for at in 0..total {
        let repo = Repo::new()?;
        halt_at(&repo, at)?;

        let mutations = Mutations::new(Fault::Never);
        let ports = Ports::new(&repo, &mutations);
        ports.apply(&clean(&repo)).map_err(|stopped| {
            format!("resuming after a halt at {at}: {stopped}")
        })?;

        assert_eq!(repo.tree(), expected, "halted at mutation {at}");
        assert_eq!(repo.read(RUN_PLAN), None, "halted at mutation {at}");
    }
    Ok(())
}

#[test]
fn rekey_holds_the_create_lock_for_its_whole_run() -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);
    let progress = Recorder::default();
    let create_was_free = Rc::new(std::cell::Cell::new(false));
    let seen = Rc::clone(&create_was_free);
    let work_dir = repo.work_dir();
    *progress.on_applied.borrow_mut() = Some(Box::new(move || {
        let contender = LockdirLock::with_options(
            &work_dir,
            LockOptions {
                ceiling_ms: 1,
                base_ms: 1,
                cap_ms: 1,
            },
        );
        if contender.acquire(&LockName::Create).is_ok() {
            seen.set(true);
        }
    }));

    ports.rekey(
        &request(RekeyAction::Apply, Forcing::Unforced),
        &clean(&repo),
        &progress,
        None,
    )?;

    assert!(!create_was_free.get());
    Ok(())
}

#[test]
fn a_fresh_mapping_over_a_partly_applied_corpus_refuses_naming_both_items(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    repo.write(
        FIRST_TARGET,
        "---\nid: \"ACC-0041\"\naliases: [\"0041\"]\n---\n\n# ACC-0041: First\n",
    )?;
    let mutations = Mutations::new(Fault::Never);
    let before = repo.tree();

    let outcome = Ports::new(&repo, &mutations).apply(&clean(&repo));

    assert_eq!(
        outcome,
        Err(NotRekeyed::PartlyApplied {
            items: vec![repo.path(FIRST), repo.path(FIRST_TARGET)],
            recoveries: Vec::new(),
        })
    );
    assert_eq!(repo.tree(), before);
    assert_eq!(repo.read(RUN_PLAN), None);
    Ok(())
}

#[test]
fn a_fresh_mapping_over_a_restore_pending_recovery_child_refuses_naming_it(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);
    let dir = PathBuf::from("retirement-recovery/0041--ACC-0041");
    ports.recovery.prepare(&dir)?;
    ports
        .recovery
        .mark_restore_pending(&dir, &[repo.path(PLAN)])?;

    let outcome = ports.apply(&clean(&repo));

    assert_eq!(
        outcome,
        Err(NotRekeyed::PartlyApplied {
            items: Vec::new(),
            recoveries: vec![repo.state_dir().join(&dir)],
        })
    );
    Ok(())
}

#[test]
fn an_unfinished_sync_retirement_is_reported_as_sync_unfinished_not_partly_applied(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);
    let retirement = Retirement {
        old_id: "draft-k7mq3x",
        new_id: "ENG-42",
        new_external_id: Some("ENG-42"),
    };
    let writer = FileCorpusStore::new(repo.root());
    FileRetirementRecords::new(&repo.state_dir(), &writer)
        .save(&RetirementRecord::of(&retirement))?;
    ports.recovery.prepare(&retirement.recovery_dir())?;

    let outcome = ports.apply(&clean(&repo));

    assert_eq!(
        outcome,
        Err(NotRekeyed::SyncUnfinished(vec![SyncBlocker::Retirement {
            old: "draft-k7mq3x".to_owned(),
            new: "ENG-42".to_owned(),
        }]))
    );
    Ok(())
}

struct WritesAPlanWhileAcquiring<'p> {
    ports: &'p Ports<'p>,
    plan: RunPlan,
}

impl IdentityRunPort for WritesAPlanWhileAcquiring<'_> {
    fn try_acquire(
        &self,
        _command: &str,
    ) -> Result<IdentityRunAttempt, StoreError> {
        self.ports
            .plan_store()
            .save(&self.plan)
            .map_err(|stopped| StoreError::Validation {
                detail: stopped.to_string(),
            })?;
        Ok(IdentityRunAttempt::Acquired(HeldIdentityRun::new(())))
    }

    fn acquire_within(
        &self,
        _command: &str,
        _ceiling: Duration,
    ) -> Result<HeldIdentityRun, IdentityRunLockError> {
        Ok(HeldIdentityRun::new(()))
    }
}

#[test]
fn a_plan_written_while_the_lock_is_acquired_is_seen() -> Result<(), TestError>
{
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);
    let first_only = vec![renumbered("0041", "ACC-0041")];
    let baseline = ports.baseline();
    let recorded =
        write_set(&first_only, &ports.corpus(&baseline), Some(&base("c1")))?;
    let racing = WritesAPlanWhileAcquiring {
        ports: &ports,
        plan: RunPlan::InProgress(InProgressPlan::new(
            IdentityFingerprint::of(&scheme(), &[]),
            first_only,
            recorded,
        )),
    };

    let report = ports.rekey(
        &request(RekeyAction::Apply, Forcing::Unforced),
        &clean(&repo),
        &Recorder::default(),
        Some(&racing),
    )?;

    assert_eq!(report, RekeyReport::Rekeyed { resumed: true });
    assert!(repo.read(FIRST_TARGET).is_some());
    assert!(repo.read(SYNCED).is_some());
    Ok(())
}

#[test]
fn discard_with_no_plan_exits_0() -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);

    let report =
        discard(&ports, &repo, Forcing::Unforced, &Recorder::default());

    assert_eq!(report, Ok(RekeyReport::NothingToDiscard));
    Ok(())
}

#[test]
fn discard_removes_an_in_progress_plan_whose_write_set_matches_its_start_digests(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let before = repo.tree();
    halt_at(&repo, 5)?;
    restore(&repo, &before)?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);

    let report =
        discard(&ports, &repo, Forcing::Unforced, &Recorder::default());

    assert_eq!(report, Ok(RekeyReport::Discarded));
    assert_eq!(repo.read(RUN_PLAN), None);
    Ok(())
}

#[test]
fn discard_refuses_an_in_progress_plan_whose_write_set_changed_naming_the_paths_and_both_remedies(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    halt_at(&repo, 5)?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);

    let outcome =
        discard(&ports, &repo, Forcing::Unforced, &Recorder::default());

    let Err(refusal @ NotRekeyed::NotReverted(_)) = outcome else {
        return Err(format!("expected not reverted: {outcome:?}").into());
    };
    let shown = refusal.to_string();
    assert!(shown.contains("Run `accelerator work rekey` to finish it"));
    assert!(shown.contains("re-run `accelerator work rekey --discard`"));
    assert!(shown.contains(&format!("    {PLAN}")), "{shown}");
    assert!(repo.read(RUN_PLAN).is_some());
    Ok(())
}

#[test]
fn not_reverted_lists_paths_to_restore_and_to_remove_and_names_the_base_revision(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    halt_at(&repo, 5)?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);

    let outcome =
        discard(&ports, &repo, Forcing::Unforced, &Recorder::default());

    let Err(NotRekeyed::NotReverted(unreverted)) = outcome else {
        return Err(format!("expected not reverted: {outcome:?}").into());
    };
    assert_eq!(
        unreverted.restore.keys().collect::<Vec<_>>(),
        vec![&Some(base("c1"))]
    );
    assert!(unreverted.restore[&Some(base("c1"))].contains(&PLAN.to_owned()));
    assert!(unreverted.remove.contains(&FIRST_TARGET.to_owned()));
    let shown = NotRekeyed::NotReverted(unreverted).to_string();
    assert!(shown.contains("  restore from c1:"), "{shown}");
    assert!(shown.contains("  remove:"), "{shown}");
    Ok(())
}

#[test]
fn forced_discard_of_a_changed_plan_sets_it_aside_and_prints_the_mapping_and_the_unreverted_paths(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    halt_at(&repo, 5)?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);

    let report = discard(&ports, &repo, Forcing::Forced, &Recorder::default())?;

    let RekeyReport::SetAside {
        aside,
        mapping,
        unreverted,
    } = report
    else {
        return Err(format!("expected a plan set aside: {report:?}").into());
    };
    assert!(aside.is_file());
    assert_eq!(repo.read(RUN_PLAN), None);
    assert_eq!(mapping, vec!["0041 → ACC-0041", "PP-760 → ACC-0042"]);
    assert!(!unreverted.is_empty());
    Ok(())
}

fn half_applied_with_an_unreadable_plan(repo: &Repo) -> Result<(), TestError> {
    repo.write(
        FIRST_TARGET,
        "---\nid: \"ACC-0041\"\naliases: [\"0041\"]\n---\n\n# ACC-0041: First\n",
    )?;
    repo.write(RUN_PLAN, "{\"schema\": 1, \"state\":")?;
    Ok(())
}

#[test]
fn discard_refuses_an_unreadable_plan_unless_forced_printing_partly_applied_evidence(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    half_applied_with_an_unreadable_plan(&repo)?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);
    let progress = Recorder::default();

    let outcome = discard(&ports, &repo, Forcing::Unforced, &progress);

    assert!(
        matches!(outcome, Err(NotRekeyed::PlanUnreadable(_))),
        "{outcome:?}"
    );
    assert_eq!(
        *progress.lines.borrow(),
        vec![
            format!("item {}", repo.path(FIRST).display()),
            format!("item {}", repo.path(FIRST_TARGET).display()),
        ]
    );
    assert!(repo.read(RUN_PLAN).is_some());
    Ok(())
}

#[test]
fn forced_discard_of_an_unreadable_plan_sets_it_aside_naming_the_file(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    half_applied_with_an_unreadable_plan(&repo)?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);
    let progress = Recorder::default();

    let report = discard(&ports, &repo, Forcing::Forced, &progress)?;

    let RekeyReport::UnreadableSetAside { aside } = report else {
        return Err(format!("expected a plan set aside: {report:?}").into());
    };
    assert_eq!(fs::read_to_string(&aside)?, "{\"schema\": 1, \"state\":");
    assert_eq!(repo.read(RUN_PLAN), None);
    assert_eq!(progress.lines.borrow().len(), 2);
    Ok(())
}

#[test]
fn a_resumed_path_is_listed_under_the_base_of_its_resume(
) -> Result<(), TestError> {
    const LATE: &str = "meta/notes/late.md";
    let repo = Repo::new()?;
    halt_at(&repo, 3)?;
    repo.write(LATE, "Committed after the halt: work-item:0041.\n")?;
    repo.write(
        "meta/work/ACC-0042-other.md",
        "---\nid: \"ACC-0042\"\n---\n\n# ACC-0042: Other\n",
    )?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);
    let resumed_at = ScriptedRepository::clean_at(&repo, "c2");
    let refused = ports.apply(&resumed_at);
    assert!(
        matches!(refused, Err(NotRekeyed::EntryRefused { .. })),
        "{refused:?}"
    );
    fs::remove_file(repo.path("meta/work/ACC-0042-other.md"))?;

    let outcome =
        discard(&ports, &repo, Forcing::Unforced, &Recorder::default());

    let Err(NotRekeyed::NotReverted(unreverted)) = outcome else {
        return Err(format!("expected not reverted: {outcome:?}").into());
    };
    assert_eq!(
        unreverted.restore.get(&Some(base("c2"))),
        Some(&vec![LATE.to_owned()])
    );
    Ok(())
}

fn preview(
    ports: &Ports<'_>,
    repository: &dyn vcs::RepositoryProbe,
    identity_run: Option<&dyn IdentityRunPort>,
) -> Result<RekeyReport, NotRekeyed> {
    ports.rekey(
        &request(RekeyAction::Preview, Forcing::Unforced),
        repository,
        &Recorder::default(),
        identity_run,
    )
}

/// A plan started with every path recorded as it is now, after which one
/// path the run would write is changed.
fn a_started_plan_whose_write_set_changed(
    repo: &Repo,
    ports: &Ports<'_>,
) -> Result<RunPlan, TestError> {
    let baseline = ports.baseline();
    let recorded =
        write_set(&support::entries(), &ports.corpus(&baseline), None)?;
    repo.write(PLAN, "Edited after the run started.\n")?;
    Ok(RunPlan::InProgress(InProgressPlan::new(
        IdentityFingerprint::of(&scheme(), &[]),
        support::entries(),
        recorded,
    )))
}

#[test]
fn discard_does_not_remove_a_plan_started_while_its_lock_was_acquired(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);
    preview(&ports, &clean(&repo), None)?;
    let racing = WritesAPlanWhileAcquiring {
        ports: &ports,
        plan: a_started_plan_whose_write_set_changed(&repo, &ports)?,
    };

    let outcome = ports.rekey(
        &request(RekeyAction::Discard, Forcing::Unforced),
        &clean(&repo),
        &Recorder::default(),
        Some(&racing),
    );

    assert!(
        matches!(outcome, Err(NotRekeyed::NotReverted(_))),
        "{outcome:?}"
    );
    assert_eq!(ports.plan_store().load(), Ok(Some(racing.plan)));
    Ok(())
}

#[test]
fn preview_does_not_replace_a_plan_started_while_its_lock_was_acquired(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);
    let racing = WritesAPlanWhileAcquiring {
        ports: &ports,
        plan: a_started_plan_whose_write_set_changed(&repo, &ports)?,
    };

    let outcome = preview(&ports, &clean(&repo), Some(&racing));

    assert_eq!(outcome, Err(NotRekeyed::InProgress(None)));
    assert_eq!(ports.plan_store().load(), Ok(Some(racing.plan)));
    Ok(())
}

#[test]
fn a_preview_records_a_proposed_plan_and_prints_its_lines(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let before = repo.tree();
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);

    let report = preview(&ports, &clean(&repo), None)?;

    assert_eq!(
        report,
        RekeyReport::Previewed {
            lines: vec![
                "0041 → ACC-0041".to_owned(),
                "PP-760 → ACC-0042".to_owned(),
            ],
            dirty: Vec::new(),
        }
    );
    assert!(matches!(
        ports.plan_store().load(),
        Ok(Some(RunPlan::Proposed(_)))
    ));
    assert_eq!(repo.tree(), before);
    Ok(())
}

/// Something that would stop a re-key, set up in an otherwise clean
/// repository.
#[derive(Debug, Clone, Copy)]
enum Obstacle {
    UnfinishedSyncRetirement,
    RestorePendingRecovery,
    DirtyWorkingCopy,
}

impl Obstacle {
    fn set_up(
        self,
        repo: &Repo,
        ports: &Ports<'_>,
        repository: &ScriptedRepository,
    ) -> Result<(), TestError> {
        match self {
            Self::UnfinishedSyncRetirement => {
                let writer = FileCorpusStore::new(repo.root());
                FileRetirementRecords::new(&repo.state_dir(), &writer).save(
                    &RetirementRecord::of(&Retirement {
                        old_id: "draft-k7mq3x",
                        new_id: "ENG-42",
                        new_external_id: Some("ENG-42"),
                    }),
                )?;
            }
            Self::RestorePendingRecovery => {
                let dir = PathBuf::from("retirement-recovery/0041--ACC-0041");
                ports.recovery.prepare(&dir)?;
                ports
                    .recovery
                    .mark_restore_pending(&dir, &[repo.path(PLAN)])?;
            }
            Self::DirtyWorkingCopy => {
                repository.dirty.borrow_mut().push(PLAN.to_owned());
            }
        }
        Ok(())
    }
}

fn applied_over(
    obstacle: Obstacle,
    previewed_first: bool,
) -> Result<Result<RekeyReport, NotRekeyed>, TestError> {
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);
    let repository = clean(&repo);
    if previewed_first {
        preview(&ports, &repository, None)?;
    }
    obstacle.set_up(&repo, &ports, &repository)?;
    Ok(ports.apply(&repository))
}

#[test]
fn a_proposed_apply_runs_the_same_preflight_as_a_fresh_apply(
) -> Result<(), TestError> {
    for obstacle in [
        Obstacle::UnfinishedSyncRetirement,
        Obstacle::RestorePendingRecovery,
        Obstacle::DirtyWorkingCopy,
    ] {
        let fresh = applied_over(obstacle, false)?;
        let proposed = applied_over(obstacle, true)?;

        assert!(fresh.is_err(), "{obstacle:?}: {fresh:?}");
        assert_eq!(
            proposed.as_ref().map_err(NotRekeyed::code),
            fresh.as_ref().map_err(NotRekeyed::code),
            "{obstacle:?}"
        );
    }
    Ok(())
}

mod in_a_real_repository {
    use vcs_adapters::library::InProcessProbe;
    use vcs_test_support::hermetic::assert_git_is_recent_enough;
    use vcs_test_support::hermetic::assert_jj_matches;
    use vcs_test_support::hermetic::Hermetic;
    use work::rekey::outcome::NotRekeyed;
    use work_adapters::rekey::run::Forcing;
    use work_adapters::rekey::run::RekeyAction;
    use work_adapters::rekey::run::RekeyReport;

    use super::halted_at;
    use super::mutations_of_a_whole_run;
    use super::request;
    use super::Fault;
    use super::Mutations;
    use super::Ports;
    use super::Recorder;
    use super::Repo;
    use super::TestError;

    #[derive(Clone, Copy, Debug)]
    enum Vcs {
        Git,
        Jj,
    }

    fn committed(vcs: Vcs) -> Result<(Repo, Hermetic), TestError> {
        let repo = Repo::new()?;
        let env = Hermetic::rooted_at(repo.root())?;
        match vcs {
            Vcs::Git => {
                assert_git_is_recent_enough()?;
                env.git(&["init", "--quiet"], repo.root())?;
            }
            Vcs::Jj => {
                assert_jj_matches("0.43.0")?;
                env.jj(&["git", "init", "--no-colocate"], repo.root())?;
            }
        }
        commit_everything(vcs, &repo, &env)?;
        Ok((repo, env))
    }

    fn commit_everything(
        vcs: Vcs,
        repo: &Repo,
        env: &Hermetic,
    ) -> Result<(), TestError> {
        match vcs {
            Vcs::Git => {
                env.git(&["add", "-A"], repo.root())?;
                env.git(&["commit", "--quiet", "-m", "snapshot"], repo.root())?;
            }
            Vcs::Jj => {
                env.jj(&["commit", "-m", "snapshot"], repo.root())?;
            }
        }
        Ok(())
    }

    fn halt_at(repo: &Repo, at: usize) -> Result<(), TestError> {
        let halting = Mutations::new(Fault::HaltAt(at));
        let ports = Ports::new(repo, &halting);
        match halted_at(|| ports.apply(&InProcessProbe)) {
            Err(Some(halted)) if halted == at => Ok(()),
            other => Err(format!("expected a halt at {at}: {other:?}").into()),
        }
    }

    #[test]
    fn discard_refuses_a_committed_half_run() -> Result<(), TestError> {
        for vcs in [Vcs::Git, Vcs::Jj] {
            let (repo, env) = committed(vcs)?;
            halt_at(&repo, 5)?;
            commit_everything(vcs, &repo, &env)?;
            let mutations = Mutations::new(Fault::Never);
            let ports = Ports::new(&repo, &mutations);

            let outcome = ports.rekey(
                &request(RekeyAction::Discard, Forcing::Unforced),
                &InProcessProbe,
                &Recorder::default(),
                None,
            );

            assert!(
                matches!(outcome, Err(NotRekeyed::NotReverted(_))),
                "{vcs:?}: {outcome:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn a_resume_after_a_completed_synced_entry_is_not_refused_as_dirty(
    ) -> Result<(), TestError> {
        let last = mutations_of_a_whole_run()? - 1;
        for vcs in [Vcs::Git, Vcs::Jj] {
            let (repo, _env) = committed(vcs)?;
            halt_at(&repo, last)?;
            let mutations = Mutations::new(Fault::Never);
            let ports = Ports::new(&repo, &mutations);

            let report = ports.apply(&InProcessProbe);

            assert_eq!(
                report,
                Ok(RekeyReport::Rekeyed { resumed: true }),
                "{vcs:?}"
            );
        }
        Ok(())
    }
}
