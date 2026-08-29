"""Cross-engine validation of cli/vcs-test-support/fixtures/masks.toml.

Mirrored by cli/vcs-test-support/tests/masks.rs, which runs the identical
positive/negative samples through the Rust `regex` crate. A pattern that
Python's `re` and Rust's `regex` interpret differently fails exactly one of
the two suites, catching engine-syntax drift that a single shared file does
not by itself prevent.
"""

import re
import tomllib
from pathlib import Path

MASKS_PATH = (
    Path(__file__).parents[3]
    / "cli"
    / "vcs-test-support"
    / "fixtures"
    / "masks.toml"
)

EXPECTED_PATTERN_NAMES = {
    "hex_object_id",
    "jj_change_id",
    "iso8601_timestamp",
    "jj_space_timestamp",
    "relative_age",
    "fixture_tempdir_path",
    "author_identity",
}


def _load_patterns() -> list[dict]:
    with MASKS_PATH.open("rb") as handle:
        return tomllib.load(handle)["pattern"]


def test_masks_toml_defines_every_named_category() -> None:
    names = {pattern["name"] for pattern in _load_patterns()}
    missing = EXPECTED_PATTERN_NAMES - names
    assert not missing, f"masks.toml is missing pattern(s): {missing}"


def test_each_pattern_matches_its_positive_sample_and_not_negative() -> None:
    for pattern in _load_patterns():
        compiled = re.compile(pattern["regex"])
        assert compiled.search(pattern["sample_match"]), (
            f"{pattern['name']}: pattern did not match its own positive "
            f"sample {pattern['sample_match']!r}"
        )
        assert not compiled.search(pattern["sample_no_match"]), (
            f"{pattern['name']}: pattern matched its own negative sample "
            f"{pattern['sample_no_match']!r}"
        )
