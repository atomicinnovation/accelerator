import re
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import ClassVar, Self

from tasks.shared.build_measurement.cache_states import BuildCacheState
from tasks.shared.build_measurement.errors import InvalidSettingsError
from tasks.shared.build_measurement.measured_build import HostSettings, Mode
from tasks.shared.build_measurement.task_graph import TaskGraph
from tasks.shared.build_measurement.tracked_processes import (
    DEFAULT_TRACKED_PROCESSES,
    TrackedProcess,
    with_ide_patterns,
)

DEFAULT_TASK = "default"
_KNOWN_LABELS = tuple(
    label for label in HostSettings if label is not HostSettings.UNSPECIFIED
)


@dataclass(frozen=True, slots=True)
class Priming:
    log_stem: ClassVar[str] = "priming"


@dataclass(frozen=True, slots=True)
class TimedRun:
    number: int

    @property
    def log_stem(self) -> str:
        return f"run-{self.number}"


type SessionRun = Priming | TimedRun


def log_name(run: SessionRun, suffix: str = "") -> str:
    return f"{run.log_stem}{suffix}.log"


@dataclass(frozen=True, slots=True)
class Invocation:
    argv: tuple[str, ...]
    log_suffix: str
    graph: TaskGraph
    reported: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class FullBuild:
    mode: ClassVar[Mode] = Mode.FULL
    isolated_tasks: ClassVar[tuple[str, ...]] = ()

    def invocations(self, graph: TaskGraph) -> tuple[Invocation, ...]:
        return (
            Invocation(
                argv=(
                    "mise",
                    "run",
                    "--output",
                    "prefix",
                    "--continue-on-error",
                    DEFAULT_TASK,
                ),
                log_suffix="",
                graph=graph,
                reported=tuple(sorted(graph.closure(DEFAULT_TASK))),
            ),
        )


type BuildScope = FullBuild


def _host_settings(label: str) -> HostSettings:
    try:
        return HostSettings(label)
    except ValueError:
        known = ", ".join(_KNOWN_LABELS)
        raise InvalidSettingsError(
            f"unknown --host-settings '{label}'; known: {known}"
        ) from None


def _compiled(pattern: str) -> str:
    try:
        re.compile(pattern)
    except re.error as error:
        raise InvalidSettingsError(
            f"--ide-pattern '{pattern}' is not a regular expression: {error}"
        ) from None
    return pattern


def _tracked(ide_patterns: Sequence[str]) -> tuple[TrackedProcess, ...]:
    if not ide_patterns:
        return DEFAULT_TRACKED_PROCESSES
    return with_ide_patterns(
        DEFAULT_TRACKED_PROCESSES, [_compiled(each) for each in ide_patterns]
    )


@dataclass(frozen=True, slots=True)
class BuildSettings:
    root: Path
    runs: int
    keep: bool
    host_settings: HostSettings
    tracked: tuple[TrackedProcess, ...]
    scope: BuildScope
    cache_state: BuildCacheState = BuildCacheState.WARM_NO_OP

    @classmethod
    def of(
        cls,
        *,
        root: Path,
        runs: int,
        keep: bool,
        host_settings: str,
        ide_patterns: Sequence[str],
    ) -> Self:
        if runs < 1:
            raise InvalidSettingsError("--runs must be at least 1")
        return cls(
            root=root,
            runs=runs,
            keep=keep,
            host_settings=_host_settings(host_settings),
            tracked=_tracked(ide_patterns),
            scope=FullBuild(),
        )
