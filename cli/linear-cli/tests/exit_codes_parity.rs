//! Pins `exit_codes.rs` against the captured bash contract.
//!
//! Non-allowlisted names must equal the value the retiring flow returned. The
//! only divergence is the search remap off the reserved `70`–`74` dispatch
//! band: each allowlisted name asserts the *remapped* Rust value while the
//! fixture keeps the original bash value. The count pin makes a silent
//! allowlist addition fail. The oracle is the committed fixture, never the
//! constants it guards.

#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::Path;

use cli_test_support::parse_u8_consts;

/// `(name, remapped-rust-value)` for the deliberate search divergence. The
/// fixture keeps the bash value; the binary emits the remapped one.
const ALLOWLIST: &[(&str, u8)] = &[
    ("SEARCH_BAD_FLAG", 75),
    ("SEARCH_BAD_LIMIT", 76),
    ("SEARCH_NO_CATALOGUE", 77),
    ("SEARCH_BAD_STATE", 78),
];

const EXPECTED_FIXTURE_COUNT: usize = 56;

fn rust_codes() -> BTreeMap<String, u8> {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/exit_codes.rs"),
    )
    .expect("exit_codes.rs is readable");
    parse_u8_consts(&source).into_iter().collect()
}

fn bash_codes() -> Vec<(String, u8)> {
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/bash-exit-codes.txt"),
    )
    .expect("the bash-exit-codes fixture is committed");
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (name, value) =
                line.split_once('=').expect("a NAME=INT fixture row");
            (name.to_owned(), value.parse().expect("an integer value"))
        })
        .collect()
}

#[test]
fn every_captured_code_matches_or_is_an_allowlisted_divergence() {
    let rust = rust_codes();
    let bash = bash_codes();
    assert_eq!(
        bash.len(),
        EXPECTED_FIXTURE_COUNT,
        "a captured code was added or removed without updating the pin"
    );

    let allowlist: BTreeMap<&str, u8> = ALLOWLIST.iter().copied().collect();
    for (name, bash_value) in &bash {
        let rust_value = rust.get(name).unwrap_or_else(|| {
            panic!("exit_codes.rs has no constant named {name}")
        });
        if let Some(remapped) = allowlist.get(name.as_str()) {
            assert_eq!(
                *rust_value, *remapped,
                "{name} must carry its remapped value"
            );
            assert_ne!(
                *rust_value, *bash_value,
                "{name} is allowlisted but did not actually diverge"
            );
        } else {
            assert_eq!(
                *rust_value, *bash_value,
                "{name} must equal the bash value it was captured at"
            );
        }
    }
}

#[test]
fn the_allowlist_is_count_pinned_and_moves_off_the_reserved_band() {
    assert_eq!(ALLOWLIST.len(), 4, "the search remap is exactly four codes");
    for (name, value) in ALLOWLIST {
        assert!(
            !(70..=74).contains(value),
            "{name} must not sit on the reserved 70-74 dispatch band"
        );
    }
}

#[test]
fn no_code_lands_on_the_reserved_dispatch_band() {
    for (name, value) in rust_codes() {
        assert!(
            !(70..=74).contains(&value),
            "{name}={value} sits on the reserved 70-74 dispatch band"
        );
    }
}
