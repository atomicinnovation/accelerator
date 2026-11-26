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


def test_every_consumer_consumes_one_or_more_shared_families():
    assert all(
        families and all(isinstance(family, Family) for family in families)
        for families in CONSUMERS.values()
    )


def test_the_matrix_consumers_are_the_five_binaries_that_adopt_it():
    assert consumers_of(Family.MATRIX) == {
        "vcs-adapters::classify",
        "vcs-adapters::queries",
        "vcs-adapters::scrub",
        "vcs-test-support::matrix",
        "vcs-test-support::matrix_private_build",
    }


def test_the_status_log_consumers_are_the_binaries_that_adopt_or_copy_it():
    assert consumers_of(Family.STATUS_LOG) == {
        "accelerator-vcs::status_log_goldens",
        "vcs-adapters::conflicts",
        "vcs-test-support::copy_fidelity",
        "vcs-test-support::status_log_parts",
        "vcs-test-support::status_log_states",
    }


def test_the_stalled_migration_consumer_is_the_guarded_resume_binary():
    assert consumers_of(Family.STALLED_MIGRATION) == {
        "accelerator-migrate::guarded_resume",
    }


def _consumed_by(matching: Callable[[str, str], bool]) -> frozenset[Family]:
    return families_consumed(NextestListing.parse(captured_listing(matching)))


def test_the_capture_was_taken_with_the_pinned_nextest():
    tools = tomllib.loads((REPO_ROOT / "mise.toml").read_text())["tools"]

    assert (CAPTURE / "nextest-list.version").read_text().strip() == (
        tools["aqua:nextest-rs/nextest/cargo-nextest"]
    )


def test_every_consumer_matching_consumes_both_families():
    assert _consumed_by(lambda _binary, _name: True) == {
        Family.MATRIX,
        Family.STATUS_LOG,
    }


def test_a_private_set_test_in_a_consumer_binary_consumes_its_family():
    consumed = _consumed_by(
        lambda _binary, name: name == "the_log_is_capped_at_five_entries"
    )

    assert consumed == {Family.STATUS_LOG}


def test_only_the_parity_binary_matching_consumes_nothing():
    consumed = _consumed_by(
        lambda binary, _name: binary == "accelerator-vcs::status_log_parity"
    )

    assert consumed == frozenset()


def test_only_the_conflicts_binary_matching_consumes_the_status_log_set():
    consumed = _consumed_by(
        lambda binary, _name: binary == "vcs-adapters::conflicts"
    )

    assert consumed == {Family.STATUS_LOG}


def test_a_classify_test_and_the_conflicts_binary_consume_both_families():
    consumed = _consumed_by(
        lambda binary, name: (
            binary == "vcs-adapters::conflicts"
            or name
            == "a_broken_jj_store_pointer_fails_rather_than_misclassifies"
        )
    )

    assert consumed == {Family.MATRIX, Family.STATUS_LOG}


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
