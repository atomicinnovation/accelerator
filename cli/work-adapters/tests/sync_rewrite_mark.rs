//! How each baseline write a sync makes treats a rewrite mark the item's
//! entry already carries: a write that confirms the two bodies agree clears
//! it, and every other write that pushes no body keeps it.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use tracker_test_support::Rewrite;
use work::sync::Action;
use work::sync::Resolution;
use work::sync::SyncState;
use work_adapters::sync::push_fidelity::RewriteKind;
use work_adapters::sync::rewrite_mark::MarkState;
use work_adapters::sync::run::RunReport;

use crate::support::repo::external;
use crate::support::repo::issue;
use crate::support::repo::outcome_of;
use crate::support::repo::work_item;
use crate::support::repo::Recorded;
use crate::support::repo::Repo;
use crate::support::repo::Tracker;
use crate::support::repo::MOVED_STAMP;
use crate::support::repo::STAMP;

const SENT: &str = "See crates.io.\n";
const LINKED: &str = "See [crates.io](<http://crates.io>).\n";

fn projected(title: &str, body: &str) -> String {
    format!("{title}\n{body}")
}

fn as_pushed() -> String {
    work_item(1, "Item", "", SENT)
}

/// A rewritten push of `as_pushed()`, marked, with the file now holding
/// `local` and the tracker `remote` at `stamp`.
fn after_a_rewritten_push(local: &str, remote: &str, stamp: &str) -> Repo {
    let stored = projected("Item", LINKED);
    Repo::with_item(
        Tracker::Linear,
        1,
        local,
        issue(1, remote, stamp),
        &Recorded::split(&as_pushed(), &stored),
    )
    .marked(1)
}

struct Case {
    name: &'static str,
    repo: Repo,
    expected: (SyncState, Action),
    resolution: Option<Resolution>,
}

fn run(case: &Case) {
    let report = case.resolution.map_or_else(
        || case.repo.sync_once(),
        |resolution| case.repo.resolving(1, resolution),
    );
    assert_eq!(outcome_of(&report, 1), case.expected, "{}", case.name);
    assert!(report.failures().next().is_none(), "{}", case.name);
}

#[test]
fn every_clearing_write_clears_the_mark() {
    let fixed_by_hand = projected("Item", SENT);
    let edited_remotely = projected("Item", "Rewritten by a person\n");
    let cases = [
        Case {
            name: "a remote fixed by hand",
            repo: after_a_rewritten_push(
                &as_pushed(),
                &fixed_by_hand,
                MOVED_STAMP,
            ),
            expected: (SyncState::Synced, Action::RecordBaseline),
            resolution: None,
        },
        Case {
            name: "a local frontmatter edit beside a remote fixed by hand",
            repo: after_a_rewritten_push(
                &work_item(1, "Item", "status: done\n", SENT),
                &fixed_by_hand,
                MOVED_STAMP,
            ),
            expected: (SyncState::LocallyModified, Action::RecordBaseline),
            resolution: None,
        },
        Case {
            name: "a local title edit beside a remote fixed by hand",
            repo: after_a_rewritten_push(
                &work_item(1, "Renamed", "", SENT),
                &fixed_by_hand,
                MOVED_STAMP,
            ),
            expected: (SyncState::LocallyModified, Action::Retitle),
            resolution: None,
        },
        Case {
            name: "a pull",
            repo: after_a_rewritten_push(
                &as_pushed(),
                &edited_remotely,
                MOVED_STAMP,
            ),
            expected: (SyncState::RemotelyModified, Action::Pull),
            resolution: None,
        },
        Case {
            name: "a conflict resolved to the remote",
            repo: after_a_rewritten_push(
                &work_item(1, "Item", "", "Edited locally\n"),
                &edited_remotely,
                MOVED_STAMP,
            ),
            expected: (SyncState::Conflict, Action::Pull),
            resolution: Some(Resolution::AcceptRemote),
        },
        Case {
            name: "a recovery of agreeing sides",
            repo: Repo::with_item(
                Tracker::Linear,
                1,
                &as_pushed(),
                issue(1, &fixed_by_hand, MOVED_STAMP),
                &Recorded::Split {
                    local: None,
                    projection: Some(&projected("Item", LINKED)),
                },
            )
            .marked(1),
            expected: (SyncState::Synced, Action::RecordBaseline),
            resolution: None,
        },
    ];
    for case in &cases {
        assert!(
            matches!(case.repo.mark(1), MarkState::Marked(_)),
            "{}",
            case.name
        );

        run(case);

        assert_eq!(case.repo.mark(1), MarkState::Absent, "{}", case.name);
    }
}

#[test]
fn every_keeping_write_keeps_the_mark() {
    let stored = projected("Item", LINKED);
    let cases = [
        Case {
            name: "a local frontmatter-only edit",
            repo: after_a_rewritten_push(
                &work_item(1, "Item", "status: done\n", SENT),
                &stored,
                STAMP,
            ),
            expected: (SyncState::LocallyModified, Action::RecordBaseline),
            resolution: None,
        },
        Case {
            name: "a remote fields-only change",
            repo: after_a_rewritten_push(
                &as_pushed(),
                &projected("Renamed", LINKED),
                MOVED_STAMP,
            ),
            expected: (SyncState::Synced, Action::RecordBaseline),
            resolution: None,
        },
        Case {
            name: "a retitle with no body moved",
            repo: after_a_rewritten_push(
                &work_item(1, "Renamed", "", SENT),
                &stored,
                STAMP,
            ),
            expected: (SyncState::LocallyModified, Action::Retitle),
            resolution: None,
        },
    ];
    for case in &cases {
        let before = case.repo.mark(1);
        assert!(matches!(before, MarkState::Marked(_)), "{}", case.name);

        run(case);

        assert_eq!(case.repo.mark(1), before, "{}", case.name);
    }
}

fn notices(report: &RunReport) -> Vec<(RewriteKind, Vec<String>)> {
    report
        .reported
        .iter()
        .filter_map(|item| item.rewrite.as_ref())
        .map(|notice| {
            (
                notice.kind,
                notice
                    .lines
                    .lines
                    .iter()
                    .map(|line| line.sent.clone())
                    .collect(),
            )
        })
        .collect()
}

fn earlier_lines() -> Vec<String> {
    vec!["See crates.io.".to_owned()]
}

#[test]
fn sync_notices_a_marked_item_on_every_run() {
    let repo =
        after_a_rewritten_push(&as_pushed(), &projected("Item", LINKED), STAMP);

    for run in ["first", "second"] {
        let report = repo.sync_once();

        assert_eq!(
            outcome_of(&report, 1),
            (SyncState::Synced, Action::Noop),
            "{run}"
        );
        assert_eq!(
            notices(&report),
            [(RewriteKind::StillOnPush, earlier_lines())],
            "{run}"
        );
        let notice = report.reported[0].rewrite.as_ref().expect("a notice");
        assert_eq!(notice.external_id, external(1), "{run}");
        assert!(notice.path.ends_with("0001.md"), "{run}");
    }
}

#[test]
fn a_marked_item_re_pushed_still_rewritten_gets_one_notice() {
    let repo = Repo::with_rewritten_item(
        Rewrite::AutolinkDottedTokens,
        1,
        &work_item(1, "Item", "", "Now published to crates.io first.\n"),
        issue(1, &projected("Item", LINKED), STAMP),
        &Recorded::split(&as_pushed(), &projected("Item", LINKED)),
    )
    .marked(1);

    let report = repo.sync_once();

    assert_eq!(
        notices(&report),
        [(
            RewriteKind::OnPush,
            vec!["Now published to crates.io first.".to_owned()]
        )]
    );
}

#[test]
fn a_marked_item_pulled_in_the_run_gets_no_notice() {
    let repo = after_a_rewritten_push(
        &as_pushed(),
        &projected("Item", "Rewritten by a person\n"),
        MOVED_STAMP,
    );

    let report = repo.sync_once();

    assert_eq!(outcome_of(&report, 1).1, Action::Pull);
    assert_eq!(notices(&report), []);
    assert_eq!(repo.mark(1), MarkState::Absent);
}
