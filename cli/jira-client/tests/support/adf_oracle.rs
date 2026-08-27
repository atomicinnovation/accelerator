//! Reads the frozen bash-ADF oracle corpus and compares it with this crate's.
//!
//! Each case directory carries the bash pipeline's captured `oracle.out`,
//! `oracle.err` and `oracle-status.txt`, frozen by
//! `tests/support/capture-adf-oracle.sh` while the drivers still existed. The
//! comparison reads the corpus rather than running `jira-adf-to-md.sh` /
//! `jira-md-to-adf.sh`, so it survives their deletion; the corpus is never
//! regenerated from this crate's output.
//!
//! Shared by `adf_differential.rs` and by `adf_differential_self_test.rs`,
//! which proves the comparison can fail.

#![allow(dead_code, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;

pub fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/adf")
}

/// Every case directory, sorted, so a failure names a stable case.
pub fn cases(prefix: &str) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(fixtures())
        .expect("the fixture corpus is readable")
        .filter_map(|entry| {
            let path = entry.expect("a fixture entry").path();
            let name = path.file_name()?.to_string_lossy().into_owned();
            (path.is_dir() && name.starts_with(prefix)).then_some(path)
        })
        .collect();
    found.sort();
    found
}

pub fn case_name(case: &Path) -> String {
    case.file_name()
        .expect("a case directory has a name")
        .to_string_lossy()
        .into_owned()
}

pub struct Run {
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

/// Reads one case's frozen oracle output.
///
/// Fails rather than skips on a missing corpus file or an empty/unparseable
/// status: a truncated capture must red the differential loudly, not turn a
/// case into a silent no-op. An empty `oracle.out` is legitimate — the
/// empty-document render and every abort/reject case emit no stdout — so the
/// status file is the signal that distinguishes them.
pub fn frozen_oracle(case: &Path) -> Run {
    let name = case_name(case);
    let stdout = std::fs::read(case.join("oracle.out"))
        .unwrap_or_else(|_| panic!("{name}: the frozen oracle.out is missing"));
    let stderr = std::fs::read_to_string(case.join("oracle.err"))
        .unwrap_or_else(|_| panic!("{name}: the frozen oracle.err is missing"));
    let raw = std::fs::read_to_string(case.join("oracle-status.txt"))
        .unwrap_or_else(|_| {
            panic!("{name}: the frozen oracle-status.txt is missing")
        });
    let status = raw.trim().parse::<i32>().unwrap_or_else(|_| {
        panic!("{name}: oracle-status.txt is empty or not an integer: {raw:?}")
    });
    Run {
        status,
        stdout,
        stderr,
    }
}

/// The render direction's comparison: the oracle's `jq -r` adds exactly one
/// trailing newline to a non-empty document and nothing at all to an empty
/// one, while [`jira_client::adf::render::to_markdown`] returns the joined
/// blocks — so the newline is added back here rather than baked into the
/// renderer.
pub fn render_disagreement(
    case: &str,
    oracle_stdout: &[u8],
    rust_markdown: &str,
) -> Option<String> {
    let expected = String::from_utf8_lossy(oracle_stdout).into_owned();
    let actual = if rust_markdown.is_empty() {
        String::new()
    } else {
        format!("{rust_markdown}\n")
    };
    if expected == actual {
        return None;
    }
    Some(format!(
        "{case}: the render direction disagrees with the oracle\n  \
         oracle: {expected:?}\n  rust:   {actual:?}"
    ))
}

/// The assemble direction's comparison.
///
/// Byte identity is unavailable by construction: `jq` emits object keys in
/// insertion order and `serde_json` without `preserve_order` emits them
/// sorted, and enabling that feature is forbidden — it would change
/// `Value::Number`'s representation for every other crate in the workspace.
/// Both sides are therefore canonicalised through one serialiser, which
/// compares the document rather than the byte order of its keys.
pub fn assemble_disagreement(
    case: &str,
    oracle_stdout: &[u8],
    rust_document: &Value,
) -> Option<String> {
    let oracle: Value = serde_json::from_slice(oracle_stdout)
        .expect("the oracle emits JSON on success");
    if canonical(&oracle) == canonical(rust_document) {
        return None;
    }
    Some(format!(
        "{case}: the assemble direction disagrees with the oracle\n  \
         oracle: {}\n  rust:   {}",
        canonical(&oracle),
        canonical(rust_document)
    ))
}

pub fn canonical(value: &Value) -> String {
    serde_json::to_string(value).expect("a Value re-serialises")
}
