//! Runs whose outcome turns on body equivalence: cosmetic rewrites that are
//! never a change, metadata edits that send only the title or nothing, and
//! the remedies a conflict over legacy or polluted content names.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use tracker_test_support::Call;
use work::sync::Action;
use work::sync::Resolution;
use work::sync::SyncState;
use work_adapters::sync::apply::FailureClass;
use work_adapters::sync::baseline::Entry;
use work_adapters::sync::digest;
use work_adapters::sync::run::render_dossier;
use work_adapters::sync::run::DossierRender;
use work_adapters::sync::run::ItemOutcome;
use work_adapters::sync::run::RunReport;

use crate::support::repo::issue;
use crate::support::repo::outcome_of;
use crate::support::repo::row;
use crate::support::repo::work_item;
use crate::support::repo::Recorded;
use crate::support::repo::Repo;
use crate::support::repo::Tracker;
use crate::support::repo::MOVED_STAMP;
use crate::support::repo::STAMP;

const JIRA_PANEL: &str = "{\"content\":[{\"attrs\":{\
    \"panelType\":\"info\"},\"content\":[],\"type\":\"panel\"}],\
    \"type\":\"doc\",\"version\":1}";
const JIRA_MENTION: &str = "{\"content\":[{\"content\":[{\
    \"attrs\":{\"id\":\"u1\",\"text\":\"@someone\"},\"type\":\"mention\"}],\
    \"type\":\"paragraph\"}],\"type\":\"doc\",\"version\":1}";
const JIRA_EARLIER: &str = "{\"content\":[{\"content\":[{\
    \"text\":\"Earlier\",\"type\":\"text\"}],\"type\":\"paragraph\"}],\
    \"type\":\"doc\",\"version\":1}";
const JIRA_RENDERED: &str = "{\"content\":[{\"content\":[{\
    \"text\":\"Rendered\",\"type\":\"text\"}],\"type\":\"paragraph\"}],\
    \"type\":\"doc\",\"version\":1}";

fn projected(title: &str, description: &str) -> String {
    format!("{title}\n{description}\n")
}

fn unrenderable_dossier(report: &RunReport) -> String {
    let DossierRender::Unrenderable(dossier) =
        render_dossier(&report.dossiers[0], &|_| String::new())
    else {
        panic!("the dossier has no sections to render");
    };
    dossier
}

fn failure_class(report: &RunReport, index: usize) -> FailureClass {
    match &row(report, index).outcome {
        ItemOutcome::Failed(error) => error.class(),
        _ => panic!("item {index} did not fail"),
    }
}

fn settles(repo: &Repo) {
    let report = repo.sync_once();
    assert_eq!(
        outcome_of(&report, 9),
        (SyncState::Synced, Action::Noop),
        "the next run reports no conflict"
    );
}

const ORIGINAL_BODY: &str = "Original body\n";

fn original() -> String {
    work_item(9, "Item", "status: ready\n", ORIGINAL_BODY)
}

/// A Jira item whose legacy entry recorded `original()` over a panel the
/// plugin cannot display, now holding `content` locally.
fn legacy_over_a_panel(content: &str) -> Repo {
    let projection = projected("Item", JIRA_PANEL);
    Repo::with_item(
        Tracker::Jira,
        9,
        content,
        issue(9, &projection, STAMP),
        &Recorded::Legacy {
            local: &original(),
            projection: &projection,
        },
    )
}

#[test]
fn a_legacy_conflict_over_an_unrenderable_remote_writes_a_legacy_unrenderable_dossier(
) {
    let repo = legacy_over_a_panel(&work_item(
        9,
        "Item",
        "status: ready\n",
        "Edited locally\n",
    ));

    let report = repo.sync_once();

    assert_eq!(
        outcome_of(&report, 9),
        (SyncState::Conflict, Action::Prompt)
    );
    let dossier = unrenderable_dossier(&report);
    assert!(dossier.contains("status: unrenderable\n"), "{dossier}");
    assert!(
        dossier.contains("reason: legacy-unrenderable\n"),
        "{dossier}"
    );
    assert!(dossier.contains("revert"), "{dossier}");
    assert!(dossier.contains("--resolve 0009=remote"), "{dossier}");
}

#[test]
fn resolving_local_on_a_legacy_unrenderable_item_sends_nothing() {
    let repo = legacy_over_a_panel(&work_item(
        9,
        "Item",
        "status: ready\n",
        "Edited locally\n",
    ));

    let report = repo.resolving(9, Resolution::PushLocal);

    assert_eq!(failure_class(&report, 9), FailureClass::Unrenderable);
    assert_eq!(repo.mutations(), []);
}

#[test]
fn a_legacy_unrenderable_conflict_keeps_its_local_hash() {
    let repo = legacy_over_a_panel(&work_item(
        9,
        "Item",
        "status: done\n",
        ORIGINAL_BODY,
    ));

    repo.sync_once();

    assert_eq!(
        repo.entry(9).and_then(|entry| entry.legacy().cloned()),
        Some(work::sync::LegacyHashes {
            local: Some(digest::local(&original()).unwrap()),
            remote: Some(digest::remote_body(&projected("Item", JIRA_PANEL))),
        })
    );
}

#[test]
fn each_named_legacy_unrenderable_remedy_settles_the_item() {
    let status_edit = work_item(9, "Item", "status: done\n", ORIGINAL_BODY);
    let title_edit = work_item(9, "Renamed", "status: ready\n", ORIGINAL_BODY);
    for (edited, reapplied) in [
        (&status_edit, Action::RecordBaseline),
        (&title_edit, Action::Retitle),
    ] {
        let repo = legacy_over_a_panel(edited);
        assert_eq!(
            outcome_of(&repo.sync_once(), 9),
            (SyncState::Conflict, Action::Prompt)
        );

        repo.write(9, &original());
        assert_eq!(
            outcome_of(&repo.sync_once(), 9),
            (SyncState::Synced, Action::RecordBaseline),
            "reverting the edit lets the entry migrate"
        );
        assert!(matches!(repo.entry(9), Some(Entry::Split { .. })));

        repo.write(9, edited);
        assert_eq!(
            outcome_of(&repo.sync_once(), 9),
            (SyncState::LocallyModified, reapplied)
        );
        settles(&repo);
    }

    let repo = legacy_over_a_panel(&work_item(
        9,
        "Item",
        "status: ready\n",
        "Edited locally\n",
    ));
    repo.sync_once();
    repo.tracker.edit_remotely(issue(
        9,
        &projected("Item", JIRA_RENDERED),
        MOVED_STAMP,
    ));
    let report = repo.resolving(9, Resolution::AcceptRemote);
    assert_eq!(row(&report, 9).planned.action, Action::Pull);
    assert!(repo.read(9).contains("Rendered"), "{}", repo.read(9));
    settles(&repo);
}

#[test]
fn each_named_local_pull_residue_remedy_settles_the_item() {
    let mention = projected("Item", JIRA_MENTION);

    let residue = work_item(
        9,
        "Item",
        "status: ready\n",
        &projected("Item", JIRA_EARLIER),
    );
    let repo = Repo::with_item(
        Tracker::Jira,
        9,
        &residue,
        issue(9, &mention, STAMP),
        &Recorded::Legacy {
            local: &residue,
            projection: &mention,
        },
    );
    let dossier = unrenderable_dossier(&repo.sync_once());
    assert!(
        dossier.contains("reason: local-pull-residue\n"),
        "{dossier}"
    );
    repo.tracker.edit_remotely(issue(
        9,
        &projected("Item", JIRA_RENDERED),
        MOVED_STAMP,
    ));
    repo.resolving(9, Resolution::AcceptRemote);
    assert!(repo.read(9).contains("Rendered"), "{}", repo.read(9));
    settles(&repo);

    let verbatim = work_item(9, "Item", "status: ready\n", &mention);
    let edited = work_item(9, "Item", "status: done\n", &mention);
    let repo = Repo::with_item(
        Tracker::Jira,
        9,
        &edited,
        issue(9, &mention, STAMP),
        &Recorded::Legacy {
            local: &verbatim,
            projection: &mention,
        },
    );
    let dossier = unrenderable_dossier(&repo.sync_once());
    assert!(
        dossier.contains("reason: local-pull-residue\n"),
        "{dossier}"
    );
    assert!(dossier.contains("revert"), "{dossier}");
    repo.write(9, &verbatim);
    assert_eq!(
        outcome_of(&repo.sync_once(), 9),
        (SyncState::Synced, Action::RecordBaseline)
    );
    repo.write(9, &edited);
    assert_eq!(
        outcome_of(&repo.sync_once(), 9),
        (SyncState::LocallyModified, Action::RecordBaseline)
    );
    settles(&repo);
    assert!(
        !repo
            .mutations()
            .iter()
            .any(|call| matches!(call, Call::Update { .. })),
        "the residue is never sent"
    );
}
