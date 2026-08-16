//! Runs `work::sync::decide` and `resolve_conflict_token` against the
//! shared golden in
//! `skills/work/scripts/test-fixtures/work-item-sync-decide.golden`.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::path::Path;
use std::path::PathBuf;

use work::sync::decide;
use work::sync::resolve_conflict_token;
use work::sync::Action;
use work::sync::Dirtiness;
use work::sync::Resolution;
use work::sync::SyncDirection;
use work::sync::SyncState;

type TestError = Box<dyn std::error::Error>;

const EXPECTED_DECIDE_ROWS: usize = 24;
const EXPECTED_DECIDE_RUST_ONLY_ROWS: usize = 3;
const EXPECTED_TOKEN_ROWS: usize = 5;
const EXPECTED_TOKEN_RUST_ONLY_ROWS: usize = 1;

fn repo_root() -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

enum Section {
    Decide,
    DecideRustOnly,
    Token,
    TokenRustOnly,
}

fn golden_lines() -> Result<Vec<String>, TestError> {
    let path = repo_root()?
        .join("skills/work/scripts/test-fixtures/work-item-sync-decide.golden");
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
        other => panic!("unrecognised action keyword: {other}"),
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
    let mut decide_rust_only_ran = 0;
    let mut token_ran = 0;
    let mut token_rust_only_ran = 0;

    for line in golden_lines()? {
        match line.as_str() {
            "[DECIDE]" => {
                section = Section::Decide;
                continue;
            }
            "[DECIDE_RUST_ONLY]" => {
                section = Section::DecideRustOnly;
                continue;
            }
            "[TOKEN]" => {
                section = Section::Token;
                continue;
            }
            "[TOKEN_RUST_ONLY]" => {
                section = Section::TokenRustOnly;
                continue;
            }
            _ => {}
        }

        match section {
            Section::Decide | Section::DecideRustOnly => {
                let fields: Vec<&str> = line.split('|').collect();
                let [raw_direction, raw_state, raw_dirty, raw_expected] =
                    fields[..]
                else {
                    panic!("malformed DECIDE row: {line}");
                };
                let state =
                    SyncState::from_keyword(raw_state).unwrap_or_else(|| {
                        panic!("unrecognised state: {raw_state}")
                    });
                let actual = decide(
                    direction(raw_direction),
                    state,
                    dirtiness(raw_dirty),
                );
                assert_eq!(actual, action(raw_expected), "row: {line}");
                match section {
                    Section::Decide => decide_ran += 1,
                    Section::DecideRustOnly => decide_rust_only_ran += 1,
                    Section::Token | Section::TokenRustOnly => unreachable!(),
                }
            }
            Section::Token | Section::TokenRustOnly => {
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
                match section {
                    Section::Token => token_ran += 1,
                    Section::TokenRustOnly => token_rust_only_ran += 1,
                    Section::Decide | Section::DecideRustOnly => {
                        unreachable!()
                    }
                }
            }
        }
    }

    assert_eq!(decide_ran, EXPECTED_DECIDE_ROWS);
    assert_eq!(decide_rust_only_ran, EXPECTED_DECIDE_RUST_ONLY_ROWS);
    assert_eq!(token_ran, EXPECTED_TOKEN_ROWS);
    assert_eq!(token_rust_only_ran, EXPECTED_TOKEN_RUST_ONLY_ROWS);
    Ok(())
}
