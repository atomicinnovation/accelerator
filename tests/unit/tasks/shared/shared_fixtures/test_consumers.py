import re

from tasks.shared.shared_fixtures.consumers import CONSUMERS, consumers_of
from tasks.shared.shared_fixtures.families import Family

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
