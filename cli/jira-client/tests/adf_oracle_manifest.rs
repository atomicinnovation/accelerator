//! Digest-pins the frozen ADF oracle corpus.
//!
//! With the bash drivers retired, byte-identity of `oracle.out`, `oracle.err`
//! and `oracle-status.txt` rests on this manifest plus the rule that the
//! corpus is regenerated only by re-running
//! `tests/support/capture-adf-oracle.sh` against the drivers at the recorded
//! revision — never from this crate's output. A silently dropped, added or
//! edited frozen file reds this test.

#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use sha2::Digest as _;
use sha2::Sha256;

const EXPECTED_CASE_COUNT: usize = 57;
const FROZEN_FILES: [&str; 3] =
    ["oracle.out", "oracle.err", "oracle-status.txt"];

fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/adf")
}

fn digest(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap_or_else(|_| {
        panic!("frozen file is readable: {}", path.display())
    });
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

/// The `<hex>  <relpath>` rows of the manifest, comments and blanks skipped.
fn manifest_rows() -> Vec<(String, String)> {
    let text = std::fs::read_to_string(corpus().join("oracle-manifest.txt"))
        .expect("the oracle manifest is committed");
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (hex, rel) = line
                .split_once("  ")
                .expect("a manifest row is `<hex>  <relpath>`");
            (hex.to_owned(), rel.to_owned())
        })
        .collect()
}

fn case_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(corpus())
        .expect("the corpus is readable")
        .filter_map(|entry| {
            let path = entry.expect("a corpus entry").path();
            let name = path.file_name()?.to_string_lossy().into_owned();
            (path.is_dir()
                && (name.starts_with("render-")
                    || name.starts_with("assemble-")))
            .then_some(path)
        })
        .collect();
    dirs.sort();
    dirs
}

#[test]
fn the_manifest_matches_every_frozen_file() {
    for (expected, rel) in manifest_rows() {
        let path = corpus().join(&rel);
        assert_eq!(digest(&path), expected, "{rel}: digest drifted");
    }
}

#[test]
fn every_frozen_file_is_listed_and_the_case_count_is_pinned() {
    let dirs = case_dirs();
    assert_eq!(
        dirs.len(),
        EXPECTED_CASE_COUNT,
        "a case was added or removed without updating this pin"
    );

    let listed: BTreeSet<String> =
        manifest_rows().into_iter().map(|(_, rel)| rel).collect();
    let mut on_disk = BTreeSet::new();
    for dir in dirs {
        let case = dir.file_name().expect("a case name").to_string_lossy();
        for frozen in FROZEN_FILES {
            let rel = format!("{case}/{frozen}");
            assert!(
                dir.join(frozen).is_file(),
                "{rel}: missing — the capture is incomplete"
            );
            on_disk.insert(rel);
        }
    }

    assert_eq!(
        on_disk, listed,
        "the manifest and the frozen corpus disagree on which files exist"
    );
}
