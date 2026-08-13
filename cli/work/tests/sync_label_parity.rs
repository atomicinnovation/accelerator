//! Runs `work::sync::classify_external_id`, the `RenderableState` narrowing
//! and `work::sync::label` against the shared golden also loaded by the
//! bash suite, `skills/work/scripts/test-fixtures/work-item-sync-label.golden`.
//!
//! `[DEFAULT]` has no typed Rust counterpart — the composition is the
//! script's own — so that section is count-asserted only, a deliberate
//! asymmetry rather than an oversight.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::path::Path;
use std::path::PathBuf;

use work::sync::classify_external_id;
use work::sync::label;
use work::sync::RenderableState;
use work::sync::SyncPresence;
use work::sync::SyncState;

type TestError = Box<dyn std::error::Error>;

const EXPECTED_CLASSIFY_ROWS: usize = 11;
const EXPECTED_LABEL_ROWS: usize = 8;
const EXPECTED_DEFAULT_ROWS: usize = 2;

fn repo_root() -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

enum Section {
    Classify,
    Label,
    Default,
}

fn golden_lines() -> Result<Vec<String>, TestError> {
    let path = repo_root()?
        .join("skills/work/scripts/test-fixtures/work-item-sync-label.golden");
    Ok(std::fs::read_to_string(path)?
        .lines()
        .map(str::to_owned)
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .collect())
}

fn unescape(raw: &str) -> &str {
    if raw == "(empty)" {
        ""
    } else {
        raw
    }
}

#[test]
fn every_row_matches_the_golden() -> Result<(), TestError> {
    let mut section = Section::Classify;
    let mut classify_ran = 0;
    let mut label_ran = 0;
    let mut default_ran = 0;

    for line in golden_lines()? {
        match line.as_str() {
            "[CLASSIFY]" => {
                section = Section::Classify;
                continue;
            }
            "[LABEL]" => {
                section = Section::Label;
                continue;
            }
            "[DEFAULT]" => {
                section = Section::Default;
                continue;
            }
            _ => {}
        }

        match section {
            Section::Classify => {
                let (raw, expected) =
                    line.split_once('|').expect("malformed CLASSIFY row");
                let raw = unescape(raw);
                let expected = match expected {
                    "synced" => SyncPresence::Synced,
                    "unsynced" => SyncPresence::Unsynced,
                    other => panic!("unrecognised presence: {other}"),
                };
                assert_eq!(classify_external_id(raw), expected, "row: {line}");
                classify_ran += 1;
            }
            Section::Label => {
                let fields: Vec<&str> = line.split('|').collect();
                let [status, exit, expected_output] = fields[..] else {
                    panic!("malformed LABEL row: {line}");
                };
                let renderable = SyncState::from_keyword(status)
                    .and_then(|state| RenderableState::try_from(state).ok());
                match (exit, renderable) {
                    ("0", Some(state)) => {
                        assert_eq!(
                            label(state),
                            expected_output,
                            "row: {line}"
                        );
                    }
                    ("0", None) => {
                        panic!("row {line} expects exit 0 but has no renderable state")
                    }
                    ("1", _) => {
                        assert_eq!(expected_output, "(empty)", "row: {line}");
                    }
                    (other, _) => panic!("unrecognised exit code: {other}"),
                }
                label_ran += 1;
            }
            Section::Default => {
                default_ran += 1;
            }
        }
    }

    assert_eq!(classify_ran, EXPECTED_CLASSIFY_ROWS);
    assert_eq!(label_ran, EXPECTED_LABEL_ROWS);
    assert_eq!(default_ran, EXPECTED_DEFAULT_ROWS);
    Ok(())
}

#[test]
fn every_renderable_state_narrows_from_its_sync_state() {
    let pairs = [
        (SyncState::Synced, RenderableState::Synced),
        (SyncState::Unsynced, RenderableState::Unsynced),
        (SyncState::LocallyModified, RenderableState::LocallyModified),
        (
            SyncState::RemotelyModified,
            RenderableState::RemotelyModified,
        ),
        (SyncState::Conflict, RenderableState::Conflict),
    ];
    for (state, expected) in pairs {
        assert_eq!(RenderableState::try_from(state), Ok(expected));
    }
    assert_eq!(RenderableState::try_from(SyncState::RemoteAbsent), Err(()));
    assert_eq!(RenderableState::try_from(SyncState::Indeterminate), Err(()));
}
