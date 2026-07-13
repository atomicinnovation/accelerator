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

/// Asserts a file the harness *reads* is present — a sourced bash library, an
/// awk program, a TSV table. Per the repo's exec-bit invariant these are not
/// executable, so no mode is asserted.
pub fn require_file(relative: &str) -> Result<PathBuf, TestError> {
    let path = repo_root()?.join(relative);
    if !path.is_file() {
        return Err(format!(
            "{relative} not found at {} — the differential harness has drifted \
             from the files it reads",
            path.display()
        )
        .into());
    }
    Ok(path)
}

/// Asserts an *entry-point* script is present **and executable**, hard-failing
/// with a naming diagnostic rather than letting an absent or unrunnable tool
/// surface as a mismatch. The exec bit is part of the contract here: the harness
/// spawns these directly, so a cleared bit would otherwise appear as an opaque
/// spawn failure rather than the invariant it breaks.
pub fn require_script(relative: &str) -> Result<PathBuf, TestError> {
    let script = require_file(relative)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = script.metadata()?.permissions().mode();
        if mode & 0o111 == 0 {
            return Err(format!(
                "{relative} is not executable (mode {mode:o}) — the harness \
                 spawns it directly; files that are only read go through \
                 require_file"
            )
            .into());
        }
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
