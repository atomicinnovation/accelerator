//! Black-box round-trip tests for the `RecordStore` port over the real lock and
//! atomic write. Each test isolates itself in a unique `tempfile::TempDir`.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use corpus::{Outcome, Record, RecordStore, StoreError};
use corpus_adapters::FileCorpusStore;
use tempfile::TempDir;

type TestError = Box<dyn std::error::Error>;

fn record(key: &str) -> Record {
    Record {
        transformation_key: key.to_owned(),
        schema_version: 1,
        outcome: Outcome::Edited,
        proposed_value: "value".to_owned(),
        user_value: None,
        timestamp: "2026-07-19T00:00:00+00:00".to_owned(),
        extras: Vec::new(),
    }
}

fn key_of(line: &str) -> Result<String, TestError> {
    let value: serde_json::Value = serde_json::from_str(line)?;
    Ok(value
        .get("transformation_key")
        .and_then(serde_json::Value::as_str)
        .ok_or("missing transformation_key")?
        .to_owned())
}

fn nonempty_lines(path: &Path) -> Result<Vec<String>, TestError> {
    Ok(fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

#[test]
fn adversarial_keys_round_trip_to_an_empty_file() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let path = dir.path().join("log.jsonl");
    let store = FileCorpusStore::new(dir.path());

    for key in ["a\\b", "c\"d", "e\tf", "g\x7fh"] {
        store.append_record(&path, &record(key))?;
        store.remove_by_key(&path, key)?;
    }
    assert!(nonempty_lines(&path)?.is_empty());
    Ok(())
}

#[test]
fn an_anchored_prefix_does_not_over_match() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let path = dir.path().join("log.jsonl");
    let store = FileCorpusStore::new(dir.path());

    store.append_record(&path, &record("foo"))?;
    store.append_record(&path, &record("foobar"))?;
    store.remove_by_key(&path, "foo")?;

    let lines = nonempty_lines(&path)?;
    assert_eq!(lines.len(), 1);
    assert_eq!(key_of(&lines[0])?, "foobar");
    Ok(())
}

#[test]
fn removing_one_key_leaves_the_other_record() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let path = dir.path().join("log.jsonl");
    let store = FileCorpusStore::new(dir.path());

    store.append_record(&path, &record("alpha"))?;
    store.append_record(&path, &record("beta"))?;
    store.remove_by_key(&path, "alpha")?;

    let lines = nonempty_lines(&path)?;
    assert_eq!(lines.len(), 1);
    assert_eq!(key_of(&lines[0])?, "beta");
    Ok(())
}

#[test]
fn removing_from_an_absent_file_is_a_no_op() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let path = dir.path().join("log.jsonl");
    FileCorpusStore::new(dir.path()).remove_by_key(&path, "anything")?;
    assert!(!path.exists());
    Ok(())
}

#[test]
fn removing_from_an_empty_file_is_a_no_op() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let path = dir.path().join("log.jsonl");
    fs::write(&path, b"")?;
    FileCorpusStore::new(dir.path()).remove_by_key(&path, "anything")?;
    assert!(nonempty_lines(&path)?.is_empty());
    Ok(())
}

#[test]
fn a_fresh_append_is_the_composed_line_plus_lf() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let path = dir.path().join("log.jsonl");
    FileCorpusStore::new(dir.path())
        .append_record(&path, &record("greeting"))?;

    let raw = fs::read(&path)?;
    assert_eq!(*raw.last().ok_or("empty file")?, b'\n');
    assert!(!raw.contains(&b'\r'), "carriage return leaked in");
    assert_eq!(nonempty_lines(&path)?.len(), 1);
    Ok(())
}

#[test]
fn a_newlineless_last_line_still_yields_two_lines() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let path = dir.path().join("log.jsonl");
    fs::write(&path, b"{\"transformation_key\":\"seed\"}")?;

    FileCorpusStore::new(dir.path()).append_record(&path, &record("second"))?;

    let lines = nonempty_lines(&path)?;
    assert_eq!(lines.len(), 2);
    assert_eq!(key_of(&lines[1])?, "second");
    Ok(())
}

#[test]
fn concurrent_appends_preserve_every_record() -> Result<(), TestError> {
    let dir = TempDir::new()?;
    let path = dir.path().join("log.jsonl");
    let store = FileCorpusStore::new(dir.path());

    let keys: Vec<String> = (0..12).map(|n| format!("key-{n}")).collect();
    std::thread::scope(|scope| -> Result<(), TestError> {
        let mut handles = Vec::new();
        for key in &keys {
            handles
                .push(scope.spawn(|| store.append_record(&path, &record(key))));
        }
        for handle in handles {
            handle
                .join()
                .map_err(|_| "thread panicked")?
                .map_err(|error: StoreError| error.to_string())?;
        }
        Ok(())
    })?;

    let lines = nonempty_lines(&path)?;
    assert_eq!(lines.len(), keys.len());
    let mut seen = BTreeSet::new();
    for line in &lines {
        seen.insert(key_of(line)?);
    }
    let expected: BTreeSet<String> = keys.into_iter().collect();
    assert_eq!(seen, expected);
    Ok(())
}
