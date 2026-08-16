//! Pins `exit_codes.rs`'s 70-73 constants against the `E_DISPATCH_*` lines
//! the bridge scripts declare.
//!
//! `accelerator-work` is bin-only, so both sides are parsed textually
//! rather than one of them imported.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::path::Path;
use std::path::PathBuf;

type TestError = Box<dyn std::error::Error>;

fn repo_root() -> Result<PathBuf, TestError> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

fn bash_codes() -> Result<Vec<(String, u8)>, TestError> {
    let source = std::fs::read_to_string(
        repo_root()?.join("skills/work/scripts/work-item-bridge-codes.sh"),
    )?;
    Ok(source
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("readonly E_DISPATCH_")?;
            let (name, value) = rest.split_once('=')?;
            Some((name.to_owned(), value.parse().ok()?))
        })
        .collect())
}

fn rust_codes() -> Result<Vec<(String, u8)>, TestError> {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/exit_codes.rs"),
    )?;
    Ok(source
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("pub const ")?;
            let (name, rest) = rest.split_once(':')?;
            let (kind, value) = rest.trim().split_once('=')?;
            if kind.trim() != "u8" {
                return None;
            }
            let value = value.trim().trim_end_matches(';');
            Some((name.to_owned(), value.parse().ok()?))
        })
        .collect())
}

#[test]
fn every_bash_dispatch_code_matches_its_rust_constant() -> Result<(), TestError>
{
    let bash = bash_codes()?;
    let rust = rust_codes()?;

    assert_eq!(
        bash.len(),
        4,
        "a dispatch code was added or removed in bash without a matching \
         update here"
    );

    for (name, value) in bash {
        let rust_value = rust
            .iter()
            .find(|(rust_name, _)| *rust_name == name)
            .unwrap_or_else(|| {
                panic!(
                    "exit_codes.rs has no RETRYABLE-shaped constant for {name}"
                )
            })
            .1;
        assert_eq!(value, rust_value, "{name} has drifted from bash");
    }

    Ok(())
}
