from collections.abc import Iterator, Mapping
from dataclasses import dataclass, field

import pytest

from tasks.shared.build_measurement.errors import NoMeasurableCommitError
from tasks.shared.build_measurement.provenance import (
    measured_commit,
    outside_measurements,
    working_copy_matches,
)
from tasks.shared.paths import MEASUREMENTS_RELATIVE

_KEPT_RECORD = f"{MEASUREMENTS_RELATIVE}/build_x.json"


@dataclass(frozen=True)
class FakeHistory:
    chain: tuple[str, ...] = ()
    changes: Mapping[str, tuple[str, ...]] = field(default_factory=dict)
    working_copy: tuple[str, ...] = ()

    def first_parent_chain(self) -> Iterator[str]:
        return iter(self.chain)

    def changed_paths(self, commit: str) -> tuple[str, ...]:
        return self.changes.get(commit, ())

    def working_copy_changes(self, commit: str) -> tuple[str, ...]:
        del commit
        return self.working_copy


def test_the_nearest_commit_changing_code_is_measured():
    history = FakeHistory(
        chain=("c2", "c1"),
        changes={"c2": ("tasks/a.py",), "c1": ("tasks/b.py",)},
    )
    assert measured_commit(history) == "c2"


def test_a_commit_changing_only_measurements_is_skipped():
    history = FakeHistory(
        chain=("c2", "c1"),
        changes={"c2": (_KEPT_RECORD,), "c1": ("tasks/b.py",)},
    )
    assert measured_commit(history) == "c1"


def test_an_empty_commit_is_skipped():
    history = FakeHistory(
        chain=("c2", "c1"), changes={"c1": ("tasks/b.py",)}
    )
    assert measured_commit(history) == "c1"


def test_a_history_with_no_qualifying_commit_is_refused():
    history = FakeHistory(chain=("c1",), changes={"c1": (_KEPT_RECORD,)})
    with pytest.raises(
        NoMeasurableCommitError,
        match=(
            "no commit on the first-parent chain behind the working copy "
            "changes anything outside meta/measurements/"
        ),
    ):
        measured_commit(history)


def test_changes_only_under_measurements_still_match():
    history = FakeHistory(working_copy=(_KEPT_RECORD,))
    assert working_copy_matches(history, "c1")


def test_a_change_outside_measurements_does_not_match():
    history = FakeHistory(working_copy=(_KEPT_RECORD, "tasks/a.py"))
    assert not working_copy_matches(history, "c1")


def test_a_kept_record_never_counts_as_a_change():
    assert outside_measurements([_KEPT_RECORD, "meta/plans/p.md"]) == (
        "meta/plans/p.md",
    )
