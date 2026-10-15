//! Runs whose outcome turns on body equivalence: cosmetic rewrites that are
//! never a change, metadata edits that send only the title or nothing, and
//! the remedies a conflict over legacy or polluted content names.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use tracker_test_support::Call;
use tracker_test_support::Rewrite;
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
        &Recorded::legacy(&original(), &projection),
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
        &Recorded::legacy(&residue, &mention),
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
        &Recorded::legacy(&verbatim, &mention),
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

const TITLE: &str = "Add search";
const BODY: &str =
    "# Add search\n\nSee README.md for details.\n\n- one\n- two\n";
const COSMETIC_BODY: &str =
    "# Add search\n\nSee README.md for details.\n\n* one\n* two\n";
const READY: &str = "status: ready\n";
const DONE: &str = "status: done\n";
const JIRA_BODY_TEXT: &str = "{\"content\":[{\"content\":[{\
    \"text\":\"Body text\",\"type\":\"text\"}],\"type\":\"paragraph\"}],\
    \"type\":\"doc\",\"version\":1}";

fn linear(title: &str, body: &str) -> String {
    format!("{title}\n{body}")
}

fn item(title: &str, extra: &str, body: &str) -> String {
    work_item(1, title, extra, body)
}

/// A Linear item whose recipe-1 entry recorded `recorded_content` and the
/// remote holding `BODY` under `TITLE`, now holding `content` against
/// `remote`, stamped `stamp`.
fn linear_split(content: &str, remote: &str, stamp: &str) -> Repo {
    Repo::with_item(
        Tracker::Linear,
        1,
        content,
        issue(1, remote, stamp),
        &Recorded::split(&item(TITLE, READY, BODY), &linear(TITLE, BODY)),
    )
}

/// Applies after a preview, asserting the two report the same state.
fn applied(repo: &Repo) -> RunReport {
    let previewed = outcome_of(&repo.preview(), 1).0;
    let report = repo.sync_once();
    assert_eq!(
        outcome_of(&report, 1).0,
        previewed,
        "preview and apply report the same state"
    );
    report
}

fn settles_item_one(repo: &Repo) {
    assert_eq!(
        outcome_of(&repo.sync_once(), 1),
        (SyncState::Synced, Action::Noop),
        "the next run reports synced with nothing to do"
    );
}

fn legacy_sides(
    local: Option<&str>,
    projection: Option<&str>,
) -> RecordedSides {
    RecordedSides::Legacy(
        local.map(str::to_owned),
        projection.map(str::to_owned),
    )
}

fn split_sides(local: Option<&str>, projection: Option<&str>) -> RecordedSides {
    RecordedSides::Split(
        local.map(str::to_owned),
        projection.map(str::to_owned),
    )
}

type Sides = fn(Option<&str>, Option<&str>) -> RecordedSides;

const fn recipes() -> [Sides; 2] {
    [legacy_sides, split_sides]
}

/// A recorded pair, owned so a test can build one per recipe.
enum RecordedSides {
    Legacy(Option<String>, Option<String>),
    Split(Option<String>, Option<String>),
}

impl RecordedSides {
    fn recorded(&self) -> Recorded<'_> {
        match self {
            Self::Legacy(local, projection) => Recorded::Legacy {
                local: local.as_deref(),
                projection: projection.as_deref(),
            },
            Self::Split(local, projection) => Recorded::Split {
                local: local.as_deref(),
                projection: projection.as_deref(),
            },
        }
    }
}

fn recovering(remote_body: &str, sides: &RecordedSides) -> Repo {
    Repo::with_item(
        Tracker::Linear,
        1,
        &item(TITLE, READY, BODY),
        issue(1, &linear(TITLE, remote_body), STAMP),
        &sides.recorded(),
    )
}

#[test]
fn a_missing_entry_with_an_equivalent_remote_is_baselined_without_a_write() {
    let repo = recovering(COSMETIC_BODY, &legacy_sides(None, None));
    let repo = Repo::with_item(
        Tracker::Linear,
        1,
        &repo.read(1),
        issue(1, &linear(TITLE, COSMETIC_BODY), STAMP),
        &Recorded::Nothing,
    );

    let report = applied(&repo);

    assert_eq!(
        outcome_of(&report, 1),
        (SyncState::Synced, Action::RecordBaseline)
    );
    assert!(!repo.wrote_an_item());
    assert_eq!(repo.mutations(), []);
    assert!(matches!(repo.entry(1), Some(Entry::Split { .. })));
    settles_item_one(&repo);
}

#[test]
fn a_missing_entry_with_a_differing_remote_stays_absent_and_conflicted() {
    let repo = Repo::with_item(
        Tracker::Linear,
        1,
        &item(TITLE, READY, BODY),
        issue(1, &linear(TITLE, "Different\n"), STAMP),
        &Recorded::Nothing,
    );

    let report = applied(&repo);

    assert_eq!(
        outcome_of(&report, 1),
        (SyncState::Conflict, Action::Prompt)
    );
    assert!(repo.entry(1).is_none());
    assert!(!repo.wrote_an_item());
}

#[test]
fn an_empty_local_hash_with_an_equivalent_remote_is_baselined() {
    let remote = linear(TITLE, COSMETIC_BODY);
    for recipe in recipes() {
        let repo = recovering(COSMETIC_BODY, &recipe(None, Some(&remote)));

        assert_eq!(
            outcome_of(&applied(&repo), 1),
            (SyncState::Synced, Action::RecordBaseline)
        );
        assert!(!repo.wrote_an_item());
        settles_item_one(&repo);
    }
}

#[test]
fn an_empty_local_hash_with_a_differing_remote_is_unchanged() {
    let remote = linear(TITLE, "Different\n");
    for recipe in recipes() {
        let repo = recovering("Different\n", &recipe(None, Some(&remote)));
        let before = repo.entry(1);

        assert_eq!(
            outcome_of(&applied(&repo), 1),
            (SyncState::Conflict, Action::Prompt)
        );
        assert_eq!(
            repo.entry(1).map(|e| e.legacy().cloned()),
            before.map(|e| e.legacy().cloned())
        );
        assert!(!repo.wrote_an_item());
    }
}

#[test]
fn an_empty_remote_hash_with_an_equivalent_remote_is_baselined() {
    let local = item(TITLE, READY, BODY);
    for recipe in recipes() {
        let repo = recovering(COSMETIC_BODY, &recipe(Some(&local), None));

        assert_eq!(
            outcome_of(&applied(&repo), 1),
            (SyncState::Synced, Action::RecordBaseline)
        );
        assert!(!repo.wrote_an_item());
        settles_item_one(&repo);
    }
}

#[test]
fn an_empty_remote_hash_with_a_differing_remote_has_its_local_hash_emptied() {
    let local = item(TITLE, READY, BODY);
    for recipe in recipes() {
        let repo = recovering("Different\n", &recipe(Some(&local), None));

        assert_eq!(
            outcome_of(&applied(&repo), 1),
            (SyncState::Conflict, Action::Prompt)
        );
        let entry = repo.entry(1).expect("the entry stays");
        assert_eq!(
            entry.legacy().and_then(|hashes| hashes.local.clone()),
            None
        );
        if let Entry::Split { split, .. } = entry {
            assert_eq!(split.local, None);
        }
    }
}

fn pushed_fragments() -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/linear-rewrites/pushed.md"),
    )
    .expect("the captured body")
    .trim_start()
    .to_owned()
}

const COSMETIC_REWRITES: [Rewrite; 11] = [
    Rewrite::EscapeTildes,
    Rewrite::BoldPerLine,
    Rewrite::BoldOffCode,
    Rewrite::AngleBracketDestinations,
    Rewrite::AutolinkUrls,
    Rewrite::StarBullets,
    Rewrite::UppercaseTaskMarkers,
    Rewrite::SpacedTableDelimiters,
    Rewrite::EscapeWrappedNumbers,
    Rewrite::JoinCodeSpanLines,
    Rewrite::BlankLineBeforeList,
];

#[test]
fn each_cosmetic_remote_rewrite_alone_is_synced_with_unchanged_hashes() {
    let body = pushed_fragments();
    let content = work_item(1, "Rewrites", READY, &body);
    let projection = linear("Rewrites", &body);
    for rewrite in COSMETIC_REWRITES {
        let repo = Repo::with_rewritten_item(
            rewrite,
            1,
            &content,
            issue(1, &projection, MOVED_STAMP),
            &Recorded::split(&content, &projection),
        );
        let before = repo.entry(1);

        let report = repo.sync_once();

        assert_eq!(
            outcome_of(&report, 1),
            (SyncState::Synced, Action::Noop),
            "{rewrite:?}"
        );
        assert_eq!(repo.mutations(), [], "{rewrite:?}");
        assert!(!repo.wrote_an_item(), "{rewrite:?}");
        let fingerprints = |entry: Option<Entry>| match entry {
            Some(Entry::Split { split, .. }) => split,
            other => panic!("a recipe-1 entry, got {other:?}"),
        };
        assert_eq!(
            fingerprints(repo.entry(1)),
            fingerprints(before),
            "{rewrite:?}"
        );
    }
}

#[test]
fn each_cosmetic_local_rewrite_alone_is_synced_with_no_update_or_write() {
    let body = pushed_fragments();
    let content = work_item(1, "Rewrites", READY, &body);
    let projection = linear("Rewrites", &body);
    for rewrite in COSMETIC_REWRITES {
        let repo = Repo::with_item(
            Tracker::Linear,
            1,
            &work_item(1, "Rewrites", READY, &rewrite.apply(&body)),
            issue(1, &projection, STAMP),
            &Recorded::split(&content, &projection),
        );

        let report = repo.sync_once();

        assert_eq!(
            outcome_of(&report, 1),
            (SyncState::Synced, Action::Noop),
            "{rewrite:?}"
        );
        assert_eq!(repo.mutations(), [], "{rewrite:?}");
        assert!(!repo.wrote_an_item(), "{rewrite:?}");
    }
}

#[test]
fn a_frontmatter_only_edit_is_locally_modified_with_no_update() {
    let edited = item(TITLE, DONE, BODY);
    let repo = linear_split(&edited, &linear(TITLE, BODY), STAMP);

    assert_eq!(
        outcome_of(&applied(&repo), 1),
        (SyncState::LocallyModified, Action::RecordBaseline)
    );
    assert_eq!(repo.mutations(), []);
    assert_eq!(repo.read(1), edited);
    settles_item_one(&repo);
}

#[test]
fn a_frontmatter_only_edit_with_a_real_remote_edit_pulls_the_body_and_keeps_the_frontmatter(
) {
    let repo = linear_split(
        &item(TITLE, DONE, BODY),
        &linear(TITLE, "Changed remotely\n"),
        MOVED_STAMP,
    );

    assert_eq!(
        outcome_of(&applied(&repo), 1),
        (SyncState::RemotelyModified, Action::Pull)
    );
    assert_eq!(repo.read(1), item(TITLE, DONE, "Changed remotely\n"));
    settles_item_one(&repo);
}

#[test]
fn a_remote_title_edit_with_cosmetic_body_noise_is_synced_and_not_overwritten()
{
    let content = item(TITLE, READY, BODY);
    let repo = linear_split(
        &content,
        &linear("Renamed remotely", COSMETIC_BODY),
        MOVED_STAMP,
    );

    assert_eq!(
        outcome_of(&applied(&repo), 1),
        (SyncState::Synced, Action::RecordBaseline)
    );
    assert_eq!(repo.mutations(), []);
    assert_eq!(repo.read(1), content);
    settles_item_one(&repo);
}

#[test]
fn an_equivalent_change_with_a_differing_local_title_retitles_without_an_update(
) {
    let repo = linear_split(
        &item("Renamed", READY, COSMETIC_BODY),
        &linear(TITLE, BODY),
        STAMP,
    );

    assert_eq!(
        outcome_of(&applied(&repo), 1),
        (SyncState::LocallyModified, Action::Retitle)
    );
    assert_eq!(
        repo.mutations(),
        [Call::Retitle {
            id: crate::support::repo::external(1),
            title: "Renamed".to_owned(),
        }]
    );
    settles_item_one(&repo);
}

#[test]
fn a_title_only_change_calls_retitle_and_leaves_the_description() {
    let linear_repo = linear_split(
        &item("Renamed", READY, BODY),
        &linear(TITLE, BODY),
        STAMP,
    );
    let jira_projection = projected(TITLE, JIRA_BODY_TEXT);
    let jira_repo = Repo::with_item(
        Tracker::Jira,
        1,
        &item("Renamed", READY, "Body text\n"),
        issue(1, &jira_projection, STAMP),
        &Recorded::split(&item(TITLE, READY, "Body text\n"), &jira_projection),
    );

    for (repo, description) in [
        (&linear_repo, BODY.to_owned()),
        (&jira_repo, format!("{JIRA_BODY_TEXT}\n")),
    ] {
        assert_eq!(
            outcome_of(&applied(repo), 1),
            (SyncState::LocallyModified, Action::Retitle)
        );
        assert!(repo
            .mutations()
            .iter()
            .all(|call| matches!(call, Call::Retitle { .. })));
        assert_eq!(repo.remote(1), format!("Renamed\n{description}"));
        settles_item_one(repo);
    }
}

#[test]
fn a_local_status_edit_with_a_concurrent_remote_retitle_sends_no_update() {
    let repo = linear_split(
        &item(TITLE, DONE, BODY),
        &linear("Renamed remotely", BODY),
        MOVED_STAMP,
    );

    assert_eq!(
        outcome_of(&applied(&repo), 1),
        (SyncState::LocallyModified, Action::RecordBaseline)
    );
    assert_eq!(repo.mutations(), []);
    settles_item_one(&repo);
}

#[test]
fn a_local_title_edit_with_a_concurrent_remote_retitle_is_a_conflict_and_sends_no_update(
) {
    let repo = linear_split(
        &item("Renamed locally", READY, BODY),
        &linear("Renamed remotely", BODY),
        MOVED_STAMP,
    );

    assert_eq!(
        outcome_of(&applied(&repo), 1),
        (SyncState::Conflict, Action::Prompt)
    );
    assert_eq!(repo.mutations(), []);
    assert_eq!(
        outcome_of(&repo.sync_once(), 1),
        (SyncState::Conflict, Action::Prompt),
        "the title conflict is still reported on the next run"
    );
}

#[test]
fn a_local_status_edit_after_a_recorded_remote_retitle_sends_no_title() {
    let repo = linear_split(
        &item(TITLE, READY, BODY),
        &linear("Renamed remotely", BODY),
        MOVED_STAMP,
    );
    assert_eq!(
        outcome_of(&repo.sync_once(), 1),
        (SyncState::Synced, Action::RecordBaseline)
    );

    repo.write(1, &item(TITLE, DONE, BODY));
    assert_eq!(
        outcome_of(&repo.sync_once(), 1),
        (SyncState::LocallyModified, Action::RecordBaseline)
    );

    assert_eq!(repo.mutations(), []);
    assert!(repo.remote(1).starts_with("Renamed remotely\n"));
}

#[test]
fn a_body_push_after_a_recorded_remote_retitle_leaves_the_remote_title() {
    let repo = linear_split(
        &item(TITLE, READY, BODY),
        &linear("Renamed remotely", BODY),
        MOVED_STAMP,
    );
    repo.sync_once();

    repo.write(1, &item(TITLE, READY, "Edited locally\n"));
    assert_eq!(
        outcome_of(&repo.sync_once(), 1),
        (
            SyncState::LocallyModified,
            Action::Push(work::sync::FieldDelivery::Withhold)
        )
    );

    assert!(matches!(
        repo.mutations().as_slice(),
        [Call::Update { title: None, .. }]
    ));
    let remote = repo.remote(1);
    assert!(remote.starts_with("Renamed remotely\n"), "{remote}");
    assert!(remote.contains("Edited locally"), "{remote}");
    settles_item_one(&repo);
}

#[test]
fn resolving_local_sends_the_title_with_the_description() {
    let repo = linear_split(
        &item(TITLE, READY, "Edited locally\n"),
        &linear(TITLE, "Edited remotely\n"),
        MOVED_STAMP,
    );
    assert_eq!(
        outcome_of(&repo.sync_once(), 1),
        (SyncState::Conflict, Action::Prompt)
    );

    repo.resolving(1, Resolution::PushLocal);

    assert!(matches!(
        repo.mutations().as_slice(),
        [Call::Update { title: Some(title), .. }] if title == TITLE
    ));
    settles_item_one(&repo);
}

#[test]
fn a_pull_only_run_leaves_a_pending_title_change_for_the_next_run() {
    let repo = linear_split(
        &item("Renamed", READY, BODY),
        &linear(TITLE, BODY),
        STAMP,
    );
    let before = repo.entry(1);

    let report = repo.sync(
        work::sync::SyncDirection::PullOnly,
        &std::collections::BTreeMap::new(),
        work_adapters::sync::run::RunMode::Apply,
    );

    assert_eq!(
        outcome_of(&report, 1),
        (SyncState::LocallyModified, Action::Noop)
    );
    assert_eq!(repo.entry(1), before, "the entry is left untouched");
    assert_eq!(
        outcome_of(&repo.sync_once(), 1),
        (SyncState::LocallyModified, Action::Retitle)
    );
    settles_item_one(&repo);
}

#[test]
fn a_local_title_edit_survives_a_pull_of_a_remote_body_edit() {
    let edited = item("Renamed", READY, BODY);
    let repo =
        linear_split(&edited, &linear(TITLE, "Edited remotely\n"), MOVED_STAMP);

    assert_eq!(
        outcome_of(&repo.sync_once(), 1),
        (SyncState::Conflict, Action::Prompt)
    );
    assert_eq!(repo.read(1), edited, "the local title is not pulled over");
    assert_eq!(
        outcome_of(&repo.sync_once(), 1),
        (SyncState::Conflict, Action::Prompt)
    );
}

/// A Linear item whose entry recorded a remote whose body differs from the
/// local one, as one stored differently would.
fn over_differing_bodies(content: &str, remote: &str, stamp: &str) -> Repo {
    Repo::with_item(
        Tracker::Linear,
        1,
        content,
        issue(1, remote, stamp),
        &Recorded::split(
            &item(TITLE, READY, BODY),
            &linear(TITLE, "Stored differently\n"),
        ),
    )
}

#[test]
fn a_frontmatter_only_edit_over_a_non_equivalent_body_records_the_baseline_and_sends_nothing(
) {
    let edited = item(TITLE, DONE, BODY);
    let repo = over_differing_bodies(
        &edited,
        &linear(TITLE, "Stored differently\n"),
        STAMP,
    );

    assert_eq!(
        outcome_of(&applied(&repo), 1),
        (SyncState::LocallyModified, Action::RecordBaseline)
    );
    assert_eq!(repo.mutations(), []);
    assert_eq!(repo.read(1), edited);
}

#[test]
fn a_status_edit_facing_a_remote_retitle_over_differing_bodies_writes_nothing()
{
    let edited = item(TITLE, DONE, BODY);
    let repo = over_differing_bodies(
        &edited,
        &linear("Renamed remotely", "Stored differently\n"),
        MOVED_STAMP,
    );

    assert_eq!(
        outcome_of(&applied(&repo), 1),
        (SyncState::LocallyModified, Action::RecordBaseline)
    );
    assert_eq!(repo.mutations(), []);
    assert_eq!(repo.read(1), edited);
}

#[test]
fn a_local_body_edit_facing_a_remote_retitle_is_a_conflict_and_sends_nothing() {
    let repo = linear_split(
        &item(TITLE, READY, "Edited locally\n"),
        &linear("Renamed remotely", BODY),
        MOVED_STAMP,
    );

    assert_eq!(
        outcome_of(&applied(&repo), 1),
        (SyncState::Conflict, Action::Prompt)
    );
    assert_eq!(repo.mutations(), []);
}

fn linear_legacy(content: &str, remote: &str, stamp: &str) -> Repo {
    Repo::with_item(
        Tracker::Linear,
        1,
        content,
        issue(1, remote, stamp),
        &Recorded::legacy(&item(TITLE, READY, BODY), &linear(TITLE, BODY)),
    )
}

#[test]
fn a_legacy_entry_with_cosmetic_local_rewrites_sends_no_update_and_gains_the_recipe(
) {
    let repo = linear_legacy(
        &item(TITLE, READY, COSMETIC_BODY),
        &linear(TITLE, BODY),
        STAMP,
    );

    assert_eq!(
        outcome_of(&applied(&repo), 1),
        (SyncState::LocallyModified, Action::RecordBaseline)
    );
    assert_eq!(repo.mutations(), []);
    assert!(matches!(repo.entry(1), Some(Entry::Split { .. })));
    settles_item_one(&repo);
}

#[test]
fn a_legacy_item_edited_on_both_sides_is_a_conflict_and_writes_nothing() {
    let edited = item(TITLE, READY, "Edited locally\n");
    let repo = linear_legacy(
        &edited,
        &linear(TITLE, "Edited remotely\n"),
        MOVED_STAMP,
    );

    assert_eq!(
        outcome_of(&applied(&repo), 1),
        (SyncState::Conflict, Action::Prompt)
    );
    assert_eq!(repo.read(1), edited);
    assert_eq!(repo.mutations(), []);
}

#[test]
fn a_legacy_item_with_a_local_body_edit_is_a_conflict_resolvable_as_local() {
    let repo = linear_legacy(
        &item(TITLE, READY, "Edited locally\n"),
        &linear(TITLE, BODY),
        STAMP,
    );
    assert_eq!(
        outcome_of(&repo.sync_once(), 1),
        (SyncState::Conflict, Action::Prompt)
    );
    assert_eq!(repo.mutations(), []);

    repo.resolving(1, Resolution::PushLocal);

    assert!(repo.remote(1).contains("Edited locally"));
    settles_item_one(&repo);
}

#[test]
fn a_legacy_item_with_diverged_titles_and_a_status_edit_is_a_conflict_and_sends_nothing(
) {
    let repo = linear_legacy(
        &item(TITLE, DONE, &format!("Old title\n{BODY}")),
        &linear("Renamed remotely", BODY),
        MOVED_STAMP,
    );

    assert_eq!(
        outcome_of(&applied(&repo), 1),
        (SyncState::Conflict, Action::Prompt)
    );
    assert_eq!(repo.mutations(), []);
}

#[test]
fn a_conflicting_preview_leaves_last_sync_json_byte_identical() {
    let repo = linear_legacy(
        &item(TITLE, READY, "Edited locally\n"),
        &linear(TITLE, "Edited remotely\n"),
        MOVED_STAMP,
    );
    let before = repo.baseline_text();

    repo.preview();

    assert_eq!(repo.baseline_text(), before);
}

#[test]
fn an_untouched_legacy_item_with_an_autolinked_file_name_migrates_as_synced() {
    let content = item(TITLE, READY, BODY);
    let stored = linear(TITLE, &Rewrite::AutolinkDottedTokens.apply(BODY));
    let repo = Repo::with_rewritten_item(
        Rewrite::AutolinkDottedTokens,
        1,
        &content,
        issue(1, &linear(TITLE, BODY), STAMP),
        &Recorded::legacy(&content, &stored),
    );

    assert_eq!(
        outcome_of(&applied(&repo), 1),
        (SyncState::Synced, Action::RecordBaseline)
    );
    assert!(matches!(repo.entry(1), Some(Entry::Split { .. })));
    settles_item_one(&repo);
}

#[test]
fn a_title_edit_over_an_unrenderable_remote_retitles_and_keeps_the_description()
{
    let panel = projected(TITLE, JIRA_PANEL);
    let repo = Repo::with_item(
        Tracker::Jira,
        1,
        &item("Renamed", READY, BODY),
        issue(1, &panel, STAMP),
        &Recorded::split(&item(TITLE, READY, BODY), &panel),
    );

    assert_eq!(
        outcome_of(&applied(&repo), 1),
        (SyncState::LocallyModified, Action::Retitle)
    );
    assert_eq!(repo.remote(1), projected("Renamed", JIRA_PANEL));
    settles_item_one(&repo);
}

#[test]
fn a_status_edit_over_an_unrenderable_remote_records_the_baseline() {
    let panel = projected(TITLE, JIRA_PANEL);
    let repo = Repo::with_item(
        Tracker::Jira,
        1,
        &item(TITLE, DONE, BODY),
        issue(1, &panel, STAMP),
        &Recorded::split(&item(TITLE, READY, BODY), &panel),
    );

    assert_eq!(
        outcome_of(&applied(&repo), 1),
        (SyncState::LocallyModified, Action::RecordBaseline)
    );
    assert_eq!(repo.mutations(), []);
    settles_item_one(&repo);
}

#[test]
fn an_untouched_legacy_jira_item_over_an_unrenderable_remote_migrates_and_then_syncs_as_synced(
) {
    let panel = projected(TITLE, JIRA_PANEL);
    let content = item(TITLE, READY, BODY);
    let repo = Repo::with_item(
        Tracker::Jira,
        1,
        &content,
        issue(1, &panel, STAMP),
        &Recorded::legacy(&content, &panel),
    );

    assert_eq!(
        outcome_of(&repo.sync_once(), 1),
        (SyncState::Synced, Action::RecordBaseline)
    );
    assert!(!repo.wrote_an_item());
    assert_eq!(repo.mutations(), []);
    assert!(matches!(repo.entry(1), Some(Entry::Split { .. })));
    settles_item_one(&repo);
}

fn validated_calls(repo: &Repo) -> Vec<Call> {
    repo.tracker
        .calls()
        .into_iter()
        .filter(|call| matches!(call, Call::ValidateUpdate { .. }))
        .collect()
}

#[test]
fn a_previewed_retitle_validates_only_the_title() {
    let rejected = "Body \u{1e} text\n";
    let projection = projected(TITLE, JIRA_BODY_TEXT);
    let repo = Repo::with_item(
        Tracker::Jira,
        1,
        &item("Renamed", READY, rejected),
        issue(1, &projection, STAMP),
        &Recorded::split(&item(TITLE, READY, rejected), &projection),
    );

    let report = repo.preview();

    assert_eq!(
        outcome_of(&report, 1),
        (SyncState::LocallyModified, Action::Retitle)
    );
    assert_eq!(
        row(&report, 1).validation,
        Some(tracker::ValidationOutcome::Valid)
    );
    assert!(matches!(
        validated_calls(&repo).as_slice(),
        [Call::ValidateUpdate {
            body: None,
            title: Some(_),
            ..
        }]
    ));
}

#[test]
fn a_previewed_withheld_push_does_not_validate_the_title() {
    let repo = linear_split(
        &item(TITLE, READY, "Edited locally\n"),
        &linear(TITLE, BODY),
        STAMP,
    );

    repo.preview();

    assert!(matches!(
        validated_calls(&repo).as_slice(),
        [Call::ValidateUpdate {
            title: None,
            body: Some(_),
            ..
        }]
    ));
}

const WITHOUT_FRONTMATTER: &str = "# Add search\n\nNo frontmatter here.\n";

fn unbaselined_without_frontmatter() -> Repo {
    let repo = Repo::with_item(
        Tracker::Linear,
        1,
        WITHOUT_FRONTMATTER,
        issue(1, &linear(TITLE, BODY), STAMP),
        &Recorded::Nothing,
    );
    assert_eq!(
        outcome_of(&repo.sync_once(), 1),
        (SyncState::Conflict, Action::Prompt)
    );
    repo
}

#[test]
fn resolving_local_on_an_item_without_frontmatter_fails_it_and_sends_nothing() {
    let repo = unbaselined_without_frontmatter();

    let report = repo.resolving(1, Resolution::PushLocal);

    assert!(matches!(row(&report, 1).outcome, ItemOutcome::Failed(_)));
    assert_eq!(repo.mutations(), []);
}

#[test]
fn resolving_remote_on_an_item_without_frontmatter_fails_it_and_writes_nothing()
{
    let repo = unbaselined_without_frontmatter();

    let report = repo.resolving(1, Resolution::AcceptRemote);

    assert!(matches!(row(&report, 1).outcome, ItemOutcome::Failed(_)));
    assert_eq!(repo.read(1), WITHOUT_FRONTMATTER);
}

const REJECTED_BY_JIRA: &str = "Body \u{1e} text\n";

fn jira_item_with_a_rejected_body(recorded: &Recorded<'_>) -> Repo {
    Repo::with_item(
        Tracker::Jira,
        1,
        &item(TITLE, READY, REJECTED_BY_JIRA),
        issue(1, &projected(TITLE, JIRA_BODY_TEXT), STAMP),
        recorded,
    )
}

#[test]
fn a_baselined_jira_item_with_a_rejected_local_body_and_no_edits_is_synced() {
    let projection = projected(TITLE, JIRA_BODY_TEXT);
    let repo = jira_item_with_a_rejected_body(&Recorded::split(
        &item(TITLE, READY, REJECTED_BY_JIRA),
        &projection,
    ));

    let report = applied(&repo);

    assert_eq!(outcome_of(&report, 1), (SyncState::Synced, Action::Noop));
    assert_eq!(repo.mutations(), []);
}

#[test]
fn an_unbaselined_jira_item_with_a_rejected_local_body_is_an_unresolved_conflict(
) {
    let repo = jira_item_with_a_rejected_body(&Recorded::Nothing);

    let report = applied(&repo);

    assert_eq!(
        outcome_of(&report, 1),
        (SyncState::Conflict, Action::Prompt)
    );
    assert!(repo.entry(1).is_none());
    assert!(!repo.wrote_an_item());
}

fn pulled_then_resynced(
    tracker: Tracker,
    recorded_body: &str,
    recorded_description: &str,
    remote_description: &str,
) -> Repo {
    let recorded_projection = projected(TITLE, recorded_description);
    let repo = Repo::with_item(
        tracker,
        1,
        &item(TITLE, READY, recorded_body),
        issue(1, &projected(TITLE, remote_description), MOVED_STAMP),
        &Recorded::split(
            &item(TITLE, READY, recorded_body),
            &recorded_projection,
        ),
    );
    assert_eq!(
        outcome_of(&repo.sync_once(), 1),
        (SyncState::RemotelyModified, Action::Pull)
    );
    repo
}

#[test]
fn a_pulled_item_syncs_again_as_synced_with_no_update() {
    for repo in [
        pulled_then_resynced(
            Tracker::Linear,
            "Earlier\n",
            "Earlier",
            "Rendered",
        ),
        pulled_then_resynced(
            Tracker::Jira,
            "Earlier\n",
            JIRA_EARLIER,
            JIRA_RENDERED,
        ),
    ] {
        assert!(repo.read(1).ends_with("\n\nRendered\n"), "{}", repo.read(1));
        settles_item_one(&repo);
        assert_eq!(repo.mutations(), []);
    }
}

#[test]
fn a_preview_fails_a_pull_over_an_unrenderable_remote_as_the_run_does() {
    let repo = Repo::with_item(
        Tracker::Jira,
        1,
        &item(TITLE, READY, "Earlier\n"),
        issue(1, &projected(TITLE, JIRA_PANEL), MOVED_STAMP),
        &Recorded::split(
            &item(TITLE, READY, "Earlier\n"),
            &projected(TITLE, JIRA_EARLIER),
        ),
    );

    let preview = repo.preview();

    assert_eq!(
        outcome_of(&preview, 1),
        (SyncState::RemotelyModified, Action::Pull)
    );
    assert_eq!(failure_class(&preview, 1), FailureClass::Unrenderable);
    assert_eq!(
        failure_class(&repo.sync_once(), 1),
        FailureClass::Unrenderable
    );
}
