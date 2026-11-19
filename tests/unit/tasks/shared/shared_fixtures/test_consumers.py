import re
import tomllib
from collections.abc import Callable

from tasks.shared.nextest_listing import NextestListing
from tasks.shared.paths import REPO_ROOT
from tasks.shared.shared_fixtures.consumers import (
    CONSUMERS,
    consumers_of,
    families_consumed,
)
from tasks.shared.shared_fixtures.families import Family
from tests.unit.tasks.shared_fixture_doubles import CAPTURE, captured_listing

_BINARY_ID = re.compile(r"[a-z][a-z0-9-]*::[a-z][a-z0-9_]*")


def test_every_consumer_is_a_nextest_binary_id():
    assert all(_BINARY_ID.fullmatch(binary) for binary in CONSUMERS)


def test_every_consumer_consumes_a_shared_family():
    assert all(isinstance(family, Family) for family in CONSUMERS.values())


def test_the_matrix_consumers_are_the_four_binaries_that_adopt_it():
    assert consumers_of(Family.MATRIX) == {
        "vcs-adapters::classify",
        "vcs-adapters::queries",
        "vcs-adapters::scrub",
        "vcs-test-support::matrix",
    }


def _consumed_by(matching: Callable[[str, str], bool]) -> frozenset[Family]:
    return families_consumed(NextestListing.parse(captured_listing(matching)))


def test_the_capture_was_taken_with_the_pinned_nextest():
    tools = tomllib.loads((REPO_ROOT / "mise.toml").read_text())["tools"]

    assert (CAPTURE / "nextest-list.version").read_text().strip() == (
        tools["aqua:nextest-rs/nextest/cargo-nextest"]
    )


def test_every_consumer_matching_consumes_the_matrix():
    assert _consumed_by(lambda _binary, _name: True) == {Family.MATRIX}


def test_only_a_non_consumer_matching_consumes_nothing():
    consumed = _consumed_by(
        lambda binary, _name: binary == "vcs-adapters::detection"
    )

    assert consumed == frozenset()


def test_a_consumer_s_test_that_uses_no_fixture_still_consumes_its_family():
    consumed = _consumed_by(
        lambda _binary, name: (
            name == "the_jj_binary_matches_the_resolved_library"
        )
    )

    assert consumed == {Family.MATRIX}


def test_one_consumer_binary_matching_consumes_its_family():
    consumed = _consumed_by(
        lambda binary, _name: binary == "vcs-adapters::classify"
    )

    assert consumed == {Family.MATRIX}


def test_nothing_matching_consumes_nothing():
    assert _consumed_by(lambda _binary, _name: False) == frozenset()
