//! Drives the bash ADF pipeline and compares its output with this crate's.
//!
//! Shared by `adf_differential.rs` and by `adf_differential_self_test.rs`,
//! which proves the comparison can fail.

#![allow(dead_code, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

pub const ADF_TO_MD: &str =
    "skills/integrations/jira/scripts/jira-adf-to-md.sh";
pub const MD_TO_ADF: &str =
    "skills/integrations/jira/scripts/jira-md-to-adf.sh";

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root resolves")
}

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

/// Runs one of the bash drivers over `input`.
///
/// Fails rather than skips when bash or jq is unavailable: a gate that passes
/// when its tool is missing proves nothing, and both platforms CI runs on ship
/// them.
pub fn run_oracle(script: &str, input: &[u8], seeded: bool) -> Run {
    let mut command = Command::new("bash");
    command
        .arg(repo_root().join(script))
        .current_dir(repo_root())
        .stdin(std::process::Stdio::piped());
    if seeded {
        command.env("JIRA_ADF_LOCALID_SEED", "1");
    } else {
        command.env_remove("JIRA_ADF_LOCALID_SEED");
    }
    let mut child = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect(
            "bash runs: this differential fails rather than skips when the \
             oracle is unavailable",
        );
    {
        use std::io::Write as _;
        child
            .stdin
            .as_mut()
            .expect("stdin is piped")
            .write_all(input)
            .expect("the input is written");
    }
    let output = child.wait_with_output().expect("the oracle terminates");
    Run {
        status: output.status.code().unwrap_or(-1),
        stdout: output.stdout,
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
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
