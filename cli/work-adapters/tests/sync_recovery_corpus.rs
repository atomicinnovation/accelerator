//! Linked work items captured with no baseline entry: a run baselines those
//! whose remote already agrees, and leaves the rest as conflicts.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use std::path::Path;

use work::sync::Action;
use work::sync::SyncState;

use crate::support::repo::issue;
use crate::support::repo::outcome_of;
use crate::support::repo::Recorded;
use crate::support::repo::Repo;
use crate::support::repo::Tracker;

/// The captured item `number` with no baseline entry, against its captured
/// remote.
fn captured(number: usize) -> Repo {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/work-item-sync-recovery")
        .join(format!("{number:04}"));
    let local = std::fs::read_to_string(directory.join("local.md"))
        .expect("the captured work item");
    let remote: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(directory.join("remote.json"))
            .expect("the captured remote"),
    )
    .expect("JSON");
    let field = |name: &str| {
        remote["data"]["issue"][name]
            .as_str()
            .unwrap_or_else(|| panic!("{number:04}: a captured {name}"))
            .to_owned()
    };
    Repo::with_item(
        Tracker::Linear,
        number,
        &local,
        issue(
            number,
            &format!("{}\n{}", field("title"), field("description")),
            &field("updatedAt"),
        ),
        &Recorded::Nothing,
    )
}

#[test]
fn items_0275_and_0294_classify_as_synced() {
    for number in [275, 294] {
        let repo = captured(number);

        assert_eq!(
            outcome_of(&repo.sync_once(), number),
            (SyncState::Synced, Action::RecordBaseline),
            "{number:04}"
        );
        assert!(!repo.wrote_an_item(), "{number:04}");
        assert_eq!(repo.mutations(), [], "{number:04}");
    }
}

#[test]
fn items_0276_0293_and_0302_classify_as_unresolved_conflicts() {
    for number in [276, 293, 302] {
        let repo = captured(number);

        assert_eq!(
            outcome_of(&repo.sync_once(), number),
            (SyncState::Conflict, Action::Prompt),
            "{number:04}"
        );
        assert!(!repo.wrote_an_item(), "{number:04}");
        assert_eq!(repo.mutations(), [], "{number:04}");
    }
}
