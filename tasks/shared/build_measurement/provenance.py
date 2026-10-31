from collections.abc import Iterable, Iterator
from pathlib import PurePosixPath
from typing import Protocol

from tasks.shared.build_measurement.errors import NoMeasurableCommitError
from tasks.shared.paths import MEASUREMENTS_RELATIVE


class History(Protocol):
    def first_parent_chain(self) -> Iterator[str]: ...
    def changed_paths(self, commit: str) -> tuple[str, ...]: ...
    def working_copy_changes(self, commit: str) -> tuple[str, ...]: ...


def outside_measurements(paths: Iterable[str]) -> tuple[str, ...]:
    return tuple(
        p
        for p in paths
        if not PurePosixPath(p).is_relative_to(MEASUREMENTS_RELATIVE)
    )


def measured_commit(history: History) -> str:
    for commit in history.first_parent_chain():
        if outside_measurements(history.changed_paths(commit)):
            return commit
    raise NoMeasurableCommitError(
        "no commit on the first-parent chain behind the working copy "
        f"changes anything outside {MEASUREMENTS_RELATIVE}/"
    )


def working_copy_matches(history: History, commit: str) -> bool:
    return not outside_measurements(history.working_copy_changes(commit))
