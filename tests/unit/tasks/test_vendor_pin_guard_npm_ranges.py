import json

import pytest
import semver

from tasks.shared.vendor.pin_guard.npm_ranges import (
    RangeSyntaxError,
    VersionRange,
)
from tests.unit.tasks.shared.doubles import PIN_GUARD_FIXTURES


@pytest.mark.parametrize(
    ("text", "inside", "outside"),
    [
        ("10.x", ["10.0.0", "10.99.99"], ["9.99.99", "11.0.0"]),
        ("1.2.x", ["1.2.0", "1.2.99"], ["1.1.99", "1.3.0"]),
        ("10", ["10.0.0", "10.99.99"], ["9.99.99", "11.0.0"]),
        ("1.2", ["1.2.0", "1.2.99"], ["1.1.99", "1.3.0"]),
        ("8.5.0", ["8.5.0"], ["8.4.99", "8.5.1"]),
        ("^20.19.2", ["20.19.2", "20.99.99"], ["20.19.1", "21.0.0"]),
        ("^0.2.3", ["0.2.3", "0.2.99"], ["0.2.2", "0.3.0"]),
        ("^0.0.3", ["0.0.3"], ["0.0.2", "0.0.4"]),
        ("<= 10", ["0.0.0", "10.99.99"], ["11.0.0"]),
        ("<=10", ["10.99.99"], ["11.0.0"]),
        (">= 10.9.0", ["10.9.0", "99.0.0"], ["10.8.99"]),
        (
            ">=6.0.0 <6.2.0",
            ["6.0.0", "6.1.99"],
            ["5.99.99", "6.2.0"],
        ),
        (" 12.x || 14.x", ["12.0.0", "14.99.99"], ["13.0.0", "15.0.0"]),
        (
            "^4.8.2 || ^6.10.2  || 8.x",
            ["4.8.2", "6.10.2", "8.0.0"],
            ["4.8.1", "5.0.0", "6.10.1", "9.0.0"],
        ),
        (
            "^6.14.4 || ^8.11.4 || >= 10.9.0",
            ["6.14.4", "8.11.4", "10.9.0", "26.0.0"],
            ["6.14.3", "10.8.0"],
        ),
    ],
)
def test_each_form_bounds_the_versions_it_holds(text, inside, outside):
    version_range = VersionRange.parse(text)
    for version in inside:
        assert version_range.contains(semver.Version.parse(version)), version
    for version in outside:
        assert not version_range.contains(semver.Version.parse(version)), (
            version
        )


@pytest.mark.parametrize(
    "text",
    [
        "^0.x",
        "~1.2.3",
        "1.2.3 - 1.4.0",
        "",
        "   ",
        "1.x ||",
        ">=",
        "*",
        "x",
        "1.x.3",
        "v1.2.3",
        "1.2.3.4",
        "=1.2.3",
    ],
)
def test_syntax_beyond_the_feeds_subset_is_refused(text):
    with pytest.raises(RangeSyntaxError):
        VersionRange.parse(text)


def test_every_range_in_the_recorded_feed_parses():
    feed = json.loads(
        (PIN_GUARD_FIXTURES / "vuln-core" / "index.json").read_text()
    )
    for entry in feed.values():
        VersionRange.parse(entry["vulnerable"])
        VersionRange.parse(entry["patched"])
