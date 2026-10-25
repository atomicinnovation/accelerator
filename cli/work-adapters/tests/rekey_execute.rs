//! Applying a run plan entry by entry, recording every path it writes
//! before writing it, and finishing whatever an earlier run left.

mod support;

use std::cell::RefCell;

use sha2::Digest as _;
use sha2::Sha256;
use work::rekey::fingerprint::IdentityFingerprint;
use work::rekey::outcome::NotRekeyed;
use work::rekey::run_plan::InProgressPlan;
use work::rekey::run_plan::RunPlan;
use work::rekey::write_set::Base;
use work::rekey::write_set::Digest;
use work::rekey::write_set::Recorded;
use work::rekey::write_set::StartState;
use work::rekey::write_set::WriteSet;
use work::retirement::RetirementRefusal;
use work_adapters::rekey::execute::execute;
use work_adapters::rekey::execute::write_set;
use work_adapters::rekey::run_plan_store::RunPlanStore as _;
use work_adapters::retirement::acquire_retirement_lock;

use support::entries;
use support::halted_at;
use support::renumbered;
use support::Fault;
use support::Mutations;
use support::Ports;
use support::Repo;
use support::TestError;
use support::BASELINE;
use support::FIRST;
use support::FIRST_CONTENT;
use support::FIRST_TARGET;
use support::PLAN;
use support::RUN_PLAN;
use support::SYNCED;
use support::SYNCED_TARGET;

fn base() -> Base {
    Base::new(vec!["c0ffee".to_owned()])
}

fn digest(content: &str) -> Digest {
    Digest::new(hex::encode(Sha256::digest(content.as_bytes())))
}

fn fingerprint() -> IdentityFingerprint {
    IdentityFingerprint::of(&corpus::WorkItemIdScheme::numeric(), &[])
}

/// The run plan a fresh run records for `entries`, saved as `ports` would
/// save it before its first write.
fn started(
    ports: &Ports<'_>,
    entries: Vec<work::rekey::entry::RekeyEntry>,
) -> Result<InProgressPlan, TestError> {
    let baseline = ports.baseline();
    let recorded =
        write_set(&entries, &ports.corpus(&baseline), Some(&base()))?;
    let plan = InProgressPlan::new(fingerprint(), entries, recorded);
    ports
        .plan_store()
        .save(&RunPlan::InProgress(plan.clone()))?;
    Ok(plan)
}

type Applied = RefCell<Vec<String>>;

fn executed(
    ports: &Ports<'_>,
    plan: InProgressPlan,
    applied: &Applied,
) -> Result<InProgressPlan, NotRekeyed> {
    let baseline = ports.baseline();
    let Ok(lock) = acquire_retirement_lock(&ports.locks) else {
        unreachable!("the retirement lock is free");
    };
    execute(
        plan,
        Some(&base()),
        &ports.plan_store(),
        &ports.corpus(&baseline),
        &lock,
        &|entry, path| {
            applied.borrow_mut().push(format!(
                "{} → {}\t{path}",
                entry.old,
                entry.target.id()
            ));
        },
    )
}

fn keys(write_set: &WriteSet) -> Vec<&str> {
    write_set.iter().map(|(path, _)| path.as_str()).collect()
}

fn in_progress(ports: &Ports<'_>) -> Result<InProgressPlan, TestError> {
    match ports
        .plan_store()
        .load()
        .map_err(|plan| format!("{plan:?}"))?
    {
        Some(RunPlan::InProgress(plan)) => Ok(plan),
        other => Err(format!("no plan in progress: {other:?}").into()),
    }
}

fn items_with_id(repo: &Repo, id: &str) -> usize {
    repo.tree()
        .iter()
        .filter(|(path, content)| {
            path.starts_with("meta/work/")
                && content.contains(&format!("\nid: \"{id}\"\n"))
        })
        .count()
}

#[test]
fn the_write_set_holds_every_from_to_and_rewritten_path(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations).without_an_integration();

    let plan = started(&ports, entries())?;

    assert_eq!(
        keys(plan.write_set()),
        vec![PLAN, FIRST, FIRST_TARGET, SYNCED_TARGET, SYNCED]
    );
    Ok(())
}

#[test]
fn the_write_set_holds_the_sync_baseline_when_an_integration_is_configured(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);

    let plan = started(&ports, entries())?;

    assert!(keys(plan.write_set()).contains(&BASELINE));
    Ok(())
}

#[test]
fn the_write_set_records_each_paths_start_digest_or_absence(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);

    let plan = started(&ports, entries())?;

    assert_eq!(
        plan.write_set().recorded(FIRST),
        Some(&Recorded {
            state: StartState::Content(digest(FIRST_CONTENT)),
            base: Some(base()),
        })
    );
    assert_eq!(
        plan.write_set().recorded(FIRST_TARGET),
        Some(&Recorded {
            state: StartState::Absent,
            base: Some(base()),
        })
    );
    Ok(())
}

#[test]
fn a_target_path_occupied_by_a_non_item_file_refuses_before_any_write(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    repo.write(FIRST_TARGET, "Just notes.\n")?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);
    let baseline = ports.baseline();

    let recorded = write_set(&entries(), &ports.corpus(&baseline), None);

    assert_eq!(
        recorded,
        Err(NotRekeyed::TargetExists(repo.path(FIRST_TARGET)))
    );
    assert_eq!(mutations.count(), 0);
    Ok(())
}

#[test]
fn every_entry_is_retired() -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);
    let plan = started(&ports, entries())?;
    let applied = Applied::default();

    executed(&ports, plan, &applied)?;

    assert_eq!(repo.read(FIRST), None);
    assert_eq!(repo.read(SYNCED), None);
    assert_eq!(items_with_id(&repo, "ACC-0041"), 1);
    assert_eq!(items_with_id(&repo, "ACC-0042"), 1);
    assert_eq!(
        repo.read(PLAN).as_deref(),
        Some(
            "Builds on work-item:ACC-0042 and \
             [first](../work/ACC-0041-first.md).\n"
        )
    );
    assert_eq!(
        *applied.borrow(),
        vec![
            format!("0041 → ACC-0041\t{FIRST_TARGET}"),
            format!("PP-760 → ACC-0042\t{SYNCED_TARGET}"),
        ]
    );
    Ok(())
}

#[test]
fn an_interrupted_run_finishes_without_duplicating_an_item(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let synced_only = vec![renumbered("PP-760", "ACC-0042")];
    let plan = {
        let mutations = Mutations::new(Fault::Never);
        started(&Ports::new(&repo, &mutations), synced_only)?
    };
    let halting = Mutations::new(Fault::HaltAt(3));
    let halted = halted_at(|| {
        executed(&Ports::new(&repo, &halting), plan, &Applied::default())
    });
    assert_eq!(halted.err(), Some(Some(3)));
    assert!(repo.read(SYNCED).is_some());
    assert!(repo.read(SYNCED_TARGET).is_some());
    assert!(repo
        .read(PLAN)
        .is_some_and(|plan| plan.contains("ACC-0042")));

    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);
    executed(&ports, in_progress(&ports)?, &Applied::default())?;

    assert_eq!(repo.read(SYNCED), None);
    assert_eq!(items_with_id(&repo, "ACC-0042"), 1);
    assert_eq!(items_with_id(&repo, "PP-760"), 0);
    Ok(())
}

#[test]
fn a_completed_entry_is_skipped_on_resume() -> Result<(), TestError> {
    for integrated in [true, false] {
        let repo = Repo::new()?;
        let mutations = Mutations::new(Fault::Never);
        let ports = Ports::new(&repo, &mutations);
        let ports = if integrated {
            ports
        } else {
            ports.without_an_integration()
        };
        let plan = started(&ports, entries())?;
        let finished = executed(&ports, plan, &Applied::default())?;
        let after = repo.tree();

        let refusing = Mutations::new(Fault::FailFrom(0));
        let resumed = Ports::new(&repo, &refusing);
        let resumed = if integrated {
            resumed
        } else {
            resumed.without_an_integration()
        };
        executed(&resumed, finished, &Applied::default())?;

        assert_eq!(refusing.count(), 0, "integrated: {integrated}");
        assert_eq!(repo.tree(), after);
    }
    Ok(())
}

#[test]
fn a_resume_clears_the_lock_a_killed_run_left_on_a_finished_entry(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);
    let plan = started(&ports, entries())?;
    let finished = executed(&ports, plan, &Applied::default())?;
    let abandoned = repo.path(&format!("{FIRST_TARGET}.lockdir"));
    std::fs::create_dir(&abandoned)?;
    std::fs::write(abandoned.join("owner.0123456789abcdef"), "2147483632")?;

    executed(&ports, finished, &Applied::default())?;

    assert!(!abandoned.exists());
    Ok(())
}

#[test]
fn a_referrer_committed_after_a_halt_joins_the_write_set_before_it_is_rewritten(
) -> Result<(), TestError> {
    const LATE: &str = "meta/notes/late.md";
    const LATE_CONTENT: &str = "Also builds on work-item:PP-760.\n";
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);
    let plan = started(&ports, entries())?;
    repo.write(LATE, LATE_CONTENT)?;

    executed(&ports, plan, &Applied::default())?;

    let recorded = in_progress(&ports)?;
    assert_eq!(
        recorded.write_set().recorded(LATE),
        Some(&Recorded {
            state: StartState::Content(digest(LATE_CONTENT)),
            base: Some(base()),
        })
    );
    let log = mutations.log();
    let saved = log.iter().position(|path| *path == repo.path(RUN_PLAN));
    let rewritten = log.iter().position(|path| *path == repo.path(LATE));
    assert!(
        saved.is_some() && saved < rewritten,
        "saved at {saved:?}, rewritten at {rewritten:?}"
    );
    Ok(())
}

#[test]
fn a_refused_entry_stops_the_run_and_leaves_the_plan_in_progress(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let mutations = Mutations::new(Fault::Never);
    let ports = Ports::new(&repo, &mutations);
    let plan = started(&ports, entries())?;
    repo.write(
        "meta/work/ACC-0042-other.md",
        "---\nid: \"ACC-0042\"\n---\n\n# ACC-0042: Other\n",
    )?;

    let outcome = executed(&ports, plan, &Applied::default());

    assert!(
        matches!(
            &outcome,
            Err(NotRekeyed::EntryRefused {
                entry,
                refusal: RetirementRefusal::IdTaken { .. },
            }) if entry.old == "PP-760"
        ),
        "{outcome:?}"
    );
    assert_eq!(repo.read(FIRST), None);
    assert!(repo.read(SYNCED).is_some());
    assert!(in_progress(&ports).is_ok());
    Ok(())
}

#[test]
fn a_rolled_back_entry_leaves_the_plan_in_progress() -> Result<(), TestError> {
    let repo = Repo::new()?;
    let plan = {
        let mutations = Mutations::new(Fault::Never);
        started(&Ports::new(&repo, &mutations), entries())?
    };
    let failing = Mutations::new(Fault::FailFrom(1));
    let ports = Ports::new(&repo, &failing);

    let outcome = executed(&ports, plan, &Applied::default());

    assert!(
        matches!(
            &outcome,
            Err(NotRekeyed::EntryRolledBack { entry, .. }) if entry.old == "0041"
        ),
        "{outcome:?}"
    );
    assert!(repo.read(FIRST).is_some());
    assert!(in_progress(&ports).is_ok());
    Ok(())
}

#[test]
fn an_incomplete_restore_leaves_the_plan_in_progress_naming_the_recovery_directory(
) -> Result<(), TestError> {
    let repo = Repo::new()?;
    let plan = {
        let mutations = Mutations::new(Fault::Never);
        started(
            &Ports::new(&repo, &mutations),
            vec![renumbered("0041", "ACC-0041")],
        )?
    };
    let failing = Mutations::new(Fault::FailAt(2));
    let plan_path = repo.path(PLAN);
    *failing.before_failing.borrow_mut() = Some(Box::new(move || {
        let _ = std::fs::write(&plan_path, "Edited by hand mid-run.\n");
    }));
    let ports = Ports::new(&repo, &failing);

    let outcome = executed(&ports, plan, &Applied::default());

    assert_eq!(
        outcome.map(|_| ()),
        Err(NotRekeyed::RestoreIncomplete {
            entry: renumbered("0041", "ACC-0041"),
            recovery: repo
                .state_dir()
                .join("retirement-recovery/0041--ACC-0041"),
        })
    );
    assert!(in_progress(&ports).is_ok());
    assert_eq!(
        repo.read(PLAN).as_deref(),
        Some("Edited by hand mid-run.\n")
    );
    Ok(())
}
