//! Holds `exit_codes.rs` to `tests/fixtures/captured-exit-codes.txt`.
//!
//! Shortcut has no shell predecessor to capture from, so the fixture was
//! written by hand. It is the contract a skill branches on: renumbering a
//! constant without editing the fixture fails here.

#![allow(clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::Path;

use cli_test_support::parse_u8_consts;

fn read(relative: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(relative),
    )
    .expect("readable")
}

fn rust_codes() -> BTreeMap<String, u8> {
    parse_u8_consts(&read("src/exit_codes.rs"))
        .into_iter()
        .collect()
}

fn fixture_codes() -> BTreeMap<String, u8> {
    read("tests/fixtures/captured-exit-codes.txt")
        .lines()
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
fn every_code_equals_its_fixture_row() {
    assert_eq!(rust_codes(), fixture_codes());
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

#[test]
fn no_two_codes_share_a_value_except_the_process_codes() {
    let mut seen: BTreeMap<u8, String> = BTreeMap::new();
    for (name, value) in rust_codes() {
        if let Some(other) = seen.insert(value, name.clone()) {
            panic!("{name} and {other} share {value}");
        }
    }
}
