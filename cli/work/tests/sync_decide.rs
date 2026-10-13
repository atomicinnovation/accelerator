//! Runs `work::sync::decide` and `resolve_conflict_token` against the
//! committed golden in `tests/fixtures/work-item-sync-decide.golden`.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::path::Path;

use work::sync::decide;
use work::sync::resolve_conflict_token;
use work::sync::Action;
use work::sync::Classification;
use work::sync::Dirtiness;
use work::sync::Resolution;
use work::sync::SyncDirection;
use work::sync::SyncState;
use work::sync::SyncedReconciliation;

type TestError = Box<dyn std::error::Error>;

const EXPECTED_DECIDE_ROWS: usize = 31;
const EXPECTED_TOKEN_ROWS: usize = 6;

enum Section {
    Decide,
    Token,
}

fn golden_lines() -> Result<Vec<String>, TestError> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/work-item-sync-decide.golden");
    Ok(std::fs::read_to_string(path)?
        .lines()
        .map(str::to_owned)
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .collect())
}

fn direction(raw: &str) -> SyncDirection {
    match raw {
        "bidirectional" => SyncDirection::Bidirectional,
        "push-only" => SyncDirection::PushOnly,
        "pull-only" => SyncDirection::PullOnly,
        other => panic!("unrecognised direction: {other}"),
    }
}

fn dirtiness(raw: &str) -> Dirtiness {
    match raw {
        "0" => Dirtiness::Clean,
        "1" => Dirtiness::Dirty,
        "unknown" => Dirtiness::Unknown,
        other => panic!("unrecognised dirtiness: {other}"),
    }
}

/// The golden's own vocabulary — `prompt`, not the wire keyword
/// `unresolved` that `Action`'s `Display`/`from_keyword` render.
fn action(raw: &str) -> Action {
    match raw {
        "push" => Action::Push,
        "pull" => Action::Pull,
        "skip-conflict" => Action::SkipConflict,
        "skip-dirty" => Action::SkipDirty,
        "prompt" => Action::Prompt,
        "noop" => Action::Noop,
        "record-baseline" => Action::RecordBaseline,
        other => panic!("unrecognised action keyword: {other}"),
    }
}

/// The golden's states, each the classification that reports it; a synced
/// row is one whose baseline already stands under the current recipe.
fn classification(raw: &str) -> Classification {
    if raw == "synced/record" {
        return Classification::Synced(SyncedReconciliation::RecordOnly);
    }
    match SyncState::from_keyword(raw) {
        Some(SyncState::Synced) => {
            Classification::Synced(SyncedReconciliation::ByContent)
        }
        Some(SyncState::Unsynced) => Classification::Unsynced,
        Some(SyncState::LocallyModified) => Classification::LocallyModified,
        Some(SyncState::RemotelyModified) => Classification::RemotelyModified,
        Some(SyncState::Conflict) => Classification::Conflict,
        Some(SyncState::RemoteAbsent) => Classification::RemoteAbsent,
        Some(SyncState::Indeterminate) => Classification::Indeterminate,
        None => panic!("unrecognised state: {raw}"),
    }
}

fn resolution(raw: &str) -> Resolution {
    match raw {
        "accept-remote" => Resolution::AcceptRemote,
        "push-local" => Resolution::PushLocal,
        "skip" => Resolution::Skip,
        other => panic!("unrecognised expected resolution: {other}"),
    }
}

fn unescape_token(raw: &str) -> String {
    if raw == "(empty)" {
        String::new()
    } else {
        raw.replace("(nbsp)", "\u{00A0}")
    }
}

#[test]
#[allow(clippy::panic, clippy::too_many_lines)]
fn every_decide_and_token_row_matches_the_golden() -> Result<(), TestError> {
    let mut section = Section::Decide;
    let mut decide_ran = 0;
    let mut token_ran = 0;

    for line in golden_lines()? {
        match line.as_str() {
            "[DECIDE]" => {
                section = Section::Decide;
                continue;
            }
            "[TOKEN]" => {
                section = Section::Token;
                continue;
            }
            _ => {}
        }

        match section {
            Section::Decide => {
                let fields: Vec<&str> = line.split('|').collect();
                let [raw_direction, raw_state, raw_dirty, raw_expected] =
                    fields[..]
                else {
                    panic!("malformed DECIDE row: {line}");
                };
                let actual = decide(
                    direction(raw_direction),
                    &classification(raw_state),
                    dirtiness(raw_dirty),
                );
                assert_eq!(actual, action(raw_expected), "row: {line}");
                decide_ran += 1;
            }
            Section::Token => {
                let fields: Vec<&str> = line.split('|').collect();
                let [raw_token, raw_expected] = fields[..] else {
                    panic!("malformed TOKEN row: {line}");
                };
                let token = unescape_token(raw_token);
                // The golden pins the observable resolution, which carries
                // no recognised/unrecognised distinction, so the `None` a
                // caller warns on folds to `Skip` here.
                let actual =
                    resolve_conflict_token(&token).unwrap_or(Resolution::Skip);
                let expected = resolution(raw_expected);
                assert_eq!(actual, expected, "row: {line}");
                token_ran += 1;
            }
        }
    }

    assert_eq!(decide_ran, EXPECTED_DECIDE_ROWS);
    assert_eq!(token_ran, EXPECTED_TOKEN_ROWS);
    Ok(())
}
