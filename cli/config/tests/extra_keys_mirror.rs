//! Pins the bash `EXTRA_KEYS` registry against the Rust catalogue.
//!
//! The two lists are hand-duplicated across `cli/config/src/catalogue.rs` and
//! `scripts/config-defaults.sh`, and nothing else compares them: a key added
//! to one alone is surfaced by one consumer and invisible to the other.

#![allow(clippy::expect_used)]

use std::path::Path;
use std::path::PathBuf;

fn defaults_script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/config-defaults.sh")
}

fn bash_extra_keys() -> Vec<String> {
    let raw = std::fs::read_to_string(defaults_script())
        .expect("config-defaults.sh is readable");
    let body = raw
        .split_once("EXTRA_KEYS=(")
        .expect("config-defaults.sh declares EXTRA_KEYS")
        .1
        .split_once(')')
        .expect("the EXTRA_KEYS array is closed")
        .0;
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.trim_matches('"').to_owned())
        .collect()
}

#[test]
fn the_bash_registry_lists_exactly_what_the_catalogue_does() {
    let rust: Vec<String> = config::catalogue::EXTRA_KEYS
        .iter()
        .map(|key| (*key).to_owned())
        .collect();

    assert_eq!(
        bash_extra_keys(),
        rust,
        "the bash EXTRA_KEYS registry has drifted from the Rust catalogue"
    );
}

#[test]
fn the_provider_client_keys_are_registered() {
    for key in ["jira.allowed_sites", "jira.site", "linear.team_id"] {
        assert!(
            config::catalogue::EXTRA_KEYS.contains(&key),
            "{key} must be dumpable"
        );
    }
}
