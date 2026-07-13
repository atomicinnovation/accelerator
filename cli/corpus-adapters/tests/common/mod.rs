//! Shared rigging for the differential suites: locating the repository, and
//! resolving the doc-type table through the live bash config chain.
//!
//! Each integration test is its own crate, so helpers one binary does not use
//! would otherwise read as dead code.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use corpus::DocTypeKey;

pub type TestError = Box<dyn std::error::Error>;

pub fn repo_root() -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

/// Asserts `path` exists and is executable, hard-failing with a naming
/// diagnostic rather than letting an absent tool surface as a mismatch.
pub fn require_script(relative: &str) -> Result<PathBuf, TestError> {
    let script = repo_root()?.join(relative);
    if !script.is_file() {
        return Err(format!(
            "{relative} not found at {} — the differential harness has drifted \
             from the scripts it shells to",
            script.display()
        )
        .into());
    }
    Ok(script)
}

/// The doc-type table, resolved through the bash config chain and keyed back to
/// `DocTypeKey`. Every emitted type name must map to a variant, so the linkage
/// vocabulary is single-sourced in the crate rather than re-encoded in bash.
pub fn doc_type_table() -> Result<Vec<(DocTypeKey, PathBuf)>, TestError> {
    let script = require_script("scripts/config-read-doc-type-paths.sh")?;
    let output = Command::new("bash")
        .arg(&script)
        .arg(repo_root()?)
        .output()
        .map_err(|error| {
            format!("could not run the doc-type resolver (is bash present?): {error}")
        })?;
    if !output.status.success() {
        return Err(format!(
            "config-read-doc-type-paths.sh failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    let mut table = Vec::new();
    for line in String::from_utf8(output.stdout)?.lines() {
        let Some((name, dir)) = line.split_once('\t') else {
            continue;
        };
        let kind =
            DocTypeKey::from_linkage_type_name(name).ok_or_else(|| {
                format!(
                    "the doc-type registry emits '{name}', which no DocTypeKey \
                     claims — the vocabulary has drifted from the crate"
                )
            })?;
        table.push((kind, PathBuf::from(dir)));
    }

    if table.is_empty() {
        return Err("the doc-type registry resolved no directories".into());
    }
    Ok(table)
}
