//! Pins `exit_codes.rs`'s dispatch constants against a frozen expectation.
//!
//! The integers are a live contract the work skills branch on, so the guard
//! holds them against literals committed here — the independent oracle the
//! deleted `work-item-bridge-codes.sh` used to be — rather than re-deriving
//! them from the `exit_codes.rs` constants it guards, which would be a
//! tautology no accidental renumbering could red.
//!
//! `accelerator-work` is bin-only, so the constants are parsed textually
//! rather than imported.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::path::Path;

type TestError = Box<dyn std::error::Error>;

const FROZEN_DISPATCH_CODES: &[(&str, u8)] = &[
    ("RETRYABLE", 70),
    ("TERMINAL", 71),
    ("NOT_AVAILABLE", 72),
    ("UNRECOGNISED", 73),
    ("UNCONFIGURED", 74),
];

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
fn every_dispatch_code_matches_its_frozen_expectation() -> Result<(), TestError>
{
    let rust = rust_codes()?;

    for (name, value) in FROZEN_DISPATCH_CODES {
        let rust_value = rust
            .iter()
            .find(|(rust_name, _)| rust_name == name)
            .unwrap_or_else(|| {
                panic!("exit_codes.rs has no constant named {name}")
            })
            .1;
        assert_eq!(*value, rust_value, "{name} has drifted from its contract");
    }

    Ok(())
}
