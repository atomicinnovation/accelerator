//! Cross-engine validation of `vcs-test-support/fixtures/masks.toml`.
//!
//! Mirrored by `tests/unit/vcs/test_masks.py`, which runs the
//! identical positive/negative samples through Python's `re`. A pattern
//! that the two engines interpret differently fails exactly one of the two
//! suites, catching engine-syntax drift that a single shared file does not
//! by itself prevent.

use std::path::PathBuf;

use vcs_test_support::masks;

type TestError = Box<dyn std::error::Error>;

const EXPECTED_PATTERN_NAMES: &[&str] = &[
    "hex_object_id",
    "jj_change_id",
    "iso8601_timestamp",
    "jj_space_timestamp",
    "relative_age",
    "fixture_tempdir_path",
    "author_identity",
];

fn masks_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/masks.toml")
}

#[test]
fn masks_toml_defines_every_named_category() -> Result<(), TestError> {
    let loaded = masks::load(&masks_path())?;
    let names: Vec<&str> =
        loaded.iter().map(|mask| mask.name.as_str()).collect();
    for expected in EXPECTED_PATTERN_NAMES {
        assert!(names.contains(expected), "missing mask pattern: {expected}");
    }
    Ok(())
}

#[test]
fn each_pattern_matches_its_positive_sample_and_not_its_negative_sample(
) -> Result<(), TestError> {
    let loaded = masks::load(&masks_path())?;
    for mask in &loaded {
        let compiled = regex::Regex::new(&mask.regex)?;
        assert!(
            compiled.is_match(&mask.sample_match),
            "{}: pattern did not match its own positive sample {:?}",
            mask.name,
            mask.sample_match
        );
        assert!(
            !compiled.is_match(&mask.sample_no_match),
            "{}: pattern matched its own negative sample {:?}",
            mask.name,
            mask.sample_no_match
        );
    }
    Ok(())
}
