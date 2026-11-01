import re
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import ClassVar, Self

from tasks.shared.build_measurement.cache_states import BuildCacheState
from tasks.shared.build_measurement.cold_trash import TRASH_PREFIX
from tasks.shared.build_measurement.errors import (
    IncompatibleSettingsError,
    InvalidSettingsError,
)
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


def trash_name(run: TimedRun) -> str:
    return f"{TRASH_PREFIX}{run.number}"


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


def isolated_log_suffix(task: str) -> str:
    return "-" + task.replace(":", "_")


@dataclass(frozen=True, slots=True)
class IsolatedTasks:
    tasks: tuple[str, ...]
    mode: ClassVar[Mode] = Mode.ISOLATED

    @property
    def isolated_tasks(self) -> tuple[str, ...]:
        return self.tasks

    def invocations(self, graph: TaskGraph) -> tuple[Invocation, ...]:
        return tuple(
            Invocation(
                argv=("mise", "run", "--output", "prefix", "--skip-deps", task),
                log_suffix=isolated_log_suffix(task),
                graph=graph.isolated(task),
                reported=(task,),
            )
            for task in self.tasks
        )


type BuildScope = FullBuild | IsolatedTasks


def _host_settings(label: str) -> HostSettings:
    try:
        return HostSettings(label)
    except ValueError:
        known = ", ".join(_KNOWN_LABELS)
        raise InvalidSettingsError(
            f"unknown --host-settings '{label}'; known: {known}"
        ) from None


def _cache_state(state: str) -> BuildCacheState:
    try:
        return BuildCacheState(state)
    except ValueError:
        known = ", ".join(BuildCacheState)
        raise InvalidSettingsError(
            f"unknown --cache-state '{state}'; known: {known}"
        ) from None


def _compiled(pattern: str) -> str:
    try:
        re.compile(pattern)
    except re.error as error:
        raise InvalidSettingsError(
            f"--ide-pattern '{pattern}' is not a regular expression: {error}"
        ) from None
    return pattern


def _scope(isolate: Sequence[str]) -> BuildScope:
    if not isolate:
        return FullBuild()
    repeated = [
        task for task in dict.fromkeys(isolate) if isolate.count(task) > 1
    ]
    if repeated:
        named = ", ".join(f"'{task}'" for task in repeated)
        raise InvalidSettingsError(f"--isolate names {named} more than once")
    return IsolatedTasks(tuple(isolate))


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
    cache_state: BuildCacheState

    @classmethod
    def of(
        cls,
        *,
        root: Path,
        runs: int,
        keep: bool,
        host_settings: str,
        ide_patterns: Sequence[str],
        isolate: Sequence[str],
        cache_state: str,
    ) -> Self:
        if runs < 1:
            raise InvalidSettingsError("--runs must be at least 1")
        state = _cache_state(cache_state)
        scope = _scope(isolate)
        if isinstance(scope, IsolatedTasks) and state.deletions:
            raise IncompatibleSettingsError(
                f"--isolate does not support --cache-state {state}"
            )
        return cls(
            root=root,
            runs=runs,
            keep=keep,
            host_settings=_host_settings(host_settings),
            tracked=_tracked(ide_patterns),
            scope=scope,
            cache_state=state,
        )
