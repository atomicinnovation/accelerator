//! The default test profile selects no live-tenant binary.
//!
//! `accelerator-work`'s `work sync` resolves real Jira and Linear clients, so
//! the one place a stray live API call could enter `mise run` is the contract
//! harness. `cli/.config/nextest.toml` filters it out of the default profile;
//! this pins that filter beside the binary it protects.
//!
//! The exact-match form matters: `binary(=contract)` excludes only the binary
//! named exactly `contract`, where the substring `binary(contract)` would also
//! pull a future `contract_helpers` or `contract_smoke` into the same fate and
//! read green while doing something subtler. Asserting the pinned value proves
//! exact-name exclusion.
//!
//! This is the mechanical property. The behavioural check — enumerating the
//! default profile's selection through `cargo nextest list` and asserting no
//! `contract` binary appears — costs a full workspace test-binary build and is
//! held by `tests/unit/tasks/test_nextest_filter.py` and by each contract
//! binary failing closed on its own `ACCELERATOR_TRACKER_CONTRACT` gate.

use std::path::Path;

type TestError = Box<dyn std::error::Error>;

const EXPECTED_DEFAULT_FILTER: &str = "not binary(=contract)";

fn nextest_config() -> Result<String, TestError> {
    let config =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../.config/nextest.toml");
    Ok(std::fs::read_to_string(&config)
        .map_err(|error| format!("read {}: {error}", config.display()))?)
}

/// The `default-filter` value declared under `[profile.<name>]`, comments and
/// surrounding quotes stripped.
fn profile_filter(config: &str, profile: &str) -> Option<String> {
    let header = format!("[profile.{profile}]");
    let mut in_section = false;
    for raw in config.lines() {
        let line = match raw.split_once('#') {
            Some((before, _)) => before,
            None => raw,
        }
        .trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            in_section = line == header;
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            if key.trim() == "default-filter" {
                return Some(value.trim().trim_matches(['\'', '"']).to_owned());
            }
        }
    }
    None
}

#[test]
fn the_default_profile_excludes_the_contract_binary_by_exact_match(
) -> Result<(), TestError> {
    let config = nextest_config()?;
    let filter = profile_filter(&config, "default")
        .ok_or("the default profile declares no default-filter")?;

    assert_eq!(
        filter, EXPECTED_DEFAULT_FILTER,
        "the default profile's filter has drifted from the exact-match form \
         — the substring binary(contract) would select the live-tenant \
         contract binary during mise run"
    );
    Ok(())
}

#[test]
fn the_contract_profile_selects_exactly_the_contract_binary(
) -> Result<(), TestError> {
    let config = nextest_config()?;
    let filter = profile_filter(&config, "contract")
        .ok_or("the contract profile declares no default-filter")?;

    assert_eq!(filter, "binary(=contract)");
    Ok(())
}
