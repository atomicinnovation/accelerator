//! The classification-stability oracle: for each
//! `tests/fixtures/work-item-sync-baseline/case-*`
//! fixture, asserts the digest recipes reproduce the committed
//! `expected.json`, which is captured independently of this code.
//!
//! Digest equality is necessary but not sufficient for classification
//! stability: a wrong branch order or a read-back bug in `classify` could
//! still mass-reclassify a real corpus with every digest here matching.
//! `cli/work-cli/tests/cli_sync.rs` closes that gap end to end.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::path::Path;
use std::path::PathBuf;

use remote_projection as project_remote;
use serde_json::Value;
use work_adapters::sync::digest;

type TestError = Box<dyn std::error::Error>;

const EXPECTED_CASE_COUNT: usize = 11;

fn corpus_root() -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/work-item-sync-baseline")
        .canonicalize()?)
}

fn case_dirs() -> Result<Vec<PathBuf>, TestError> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(corpus_root()?)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .and_then(std::ffi::OsStr::to_str)
                    .is_some_and(|name| name.starts_with("case-"))
        })
        .collect();
    dirs.sort();
    Ok(dirs)
}

fn expected(dir: &Path) -> Result<Value, TestError> {
    Ok(serde_json::from_str(&std::fs::read_to_string(
        dir.join("expected.json"),
    )?)?)
}

#[test]
fn every_case_matches_its_bash_generated_expectation() -> Result<(), TestError>
{
    let dirs = case_dirs()?;
    assert_eq!(
        dirs.len(),
        EXPECTED_CASE_COUNT,
        "a fixture case was added or removed without updating this floor"
    );

    for dir in dirs {
        let name = dir
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .expect("case directory has a name")
            .to_owned();
        let expected = expected(&dir)?;

        if expected["error"].as_bool() == Some(true) {
            let local_path = dir.join("local.md");
            let content = std::fs::read_to_string(&local_path)?;
            assert!(
                digest::local(&content).is_err(),
                "case {name}: expected an error, Rust produced a hash"
            );
            continue;
        }

        if let Some(local_hash) = expected["local_hash"].as_str() {
            let content = std::fs::read_to_string(dir.join("local.md"))?;
            let actual = digest::local(&content)?;
            assert_eq!(actual, local_hash, "case {name}: local_hash");
            continue;
        }

        let integration_name = expected["integration"]
            .as_str()
            .unwrap_or_else(|| panic!("case {name} names no integration"));
        let integration = project_remote::parse_integration(integration_name)
            .unwrap_or_else(|| {
                panic!(
                    "case {name}: unrecognised integration {integration_name}"
                )
            });
        let remote: Value = serde_json::from_str(&std::fs::read_to_string(
            dir.join("remote.json"),
        )?)?;

        let updated = project_remote::project(
            integration,
            project_remote::Op::Updated,
            &remote,
        );
        assert_eq!(
            updated,
            expected["updated"]
                .as_str()
                .expect("expected.updated is present"),
            "case {name}: updated"
        );

        let body = project_remote::project(
            integration,
            project_remote::Op::Body,
            &remote,
        );
        let remote_hash = digest::remote_body(&body);
        assert_eq!(
            remote_hash,
            expected["remote_hash"]
                .as_str()
                .expect("expected.remote_hash is present"),
            "case {name}: remote_hash"
        );
    }
    Ok(())
}
