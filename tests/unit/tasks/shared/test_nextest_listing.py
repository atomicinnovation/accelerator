import json

from tasks.shared.nextest_listing import NextestListing
from tests.unit.tasks.shared_fixture_doubles import (
    CAPTURED_BINARIES,
    captured_listing,
)


def test_an_unfiltered_listing_selects_every_listed_binary():
    listing = NextestListing.parse(captured_listing(lambda _b, _n: True))

    assert listing.binaries == CAPTURED_BINARIES
    assert not listing.selects_nothing


def test_a_binary_none_of_whose_tests_match_is_not_selected():
    listing = NextestListing.parse(
        captured_listing(lambda binary, _n: binary == "vcs-adapters::scrub")
    )

    assert listing.binaries == {"vcs-adapters::scrub"}


def test_a_listing_matching_no_test_selects_nothing():
    listing = NextestListing.parse(captured_listing(lambda _b, _n: False))

    assert listing.binaries == frozenset()
    assert listing.selects_nothing


def test_an_ignored_test_is_never_selected():
    listing = json.loads(captured_listing(lambda _b, _n: False))
    listing["rust-suites"]["vcs-adapters::scrub"]["testcases"]["ignored"] = {
        "kind": "test",
        "ignored": True,
        "filter-match": {"status": "mismatch", "reason": "ignored"},
    }

    assert NextestListing.parse(json.dumps(listing)).selects_nothing
