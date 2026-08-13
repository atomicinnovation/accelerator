//! Runs `work::sync::push_decide` against the shared golden also loaded by
//! the bash suite,
//! `skills/work/scripts/test-fixtures/work-item-push-decide.golden`.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::path::Path;
use std::path::PathBuf;

use work::sync::push_decide;
use work::sync::PushOutcome;

type TestError = Box<dyn std::error::Error>;

const EXPECTED_ROWS: usize = 12;

fn repo_root() -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

fn outcome(raw: &str) -> PushOutcome {
    match raw {
        "write-once" => PushOutcome::WriteOnce,
        "retry" => PushOutcome::Retry,
        "local-save" => PushOutcome::LocalSave,
        "loud-terminal" => PushOutcome::LoudTerminal,
        other => panic!("unrecognised outcome: {other}"),
    }
}

#[test]
fn every_row_matches_the_golden() -> Result<(), TestError> {
    let path = repo_root()?
        .join("skills/work/scripts/test-fixtures/work-item-push-decide.golden");
    let content = std::fs::read_to_string(path)?;
    let mut ran = 0;

    for line in content.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('|').collect();
        let [code, attempt, write_failed, expected] = fields[..] else {
            panic!("malformed row: {line}");
        };
        let code: u8 = code.parse()?;
        let attempt: u8 = attempt.parse()?;
        let write_failed = write_failed == "1";

        let actual = push_decide(code, attempt, write_failed);
        assert_eq!(actual, outcome(expected), "row: {line}");
        assert_eq!(actual.keyword(), expected, "row: {line}");
        ran += 1;
    }

    assert_eq!(ran, EXPECTED_ROWS);
    Ok(())
}
