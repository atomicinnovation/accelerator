"""What the operator is told before, during and after a build session."""

from collections.abc import Iterator, Sequence
from dataclasses import dataclass
from pathlib import Path

from tasks.shared.build_measurement.across_runs import (
    Absent,
    Caveat,
    Flagged,
    Measured,
    ProcessAcrossRuns,
)
from tasks.shared.build_measurement.fetch_failures import FetchFailure
from tasks.shared.build_measurement.measured_build import (
    HostSettings,
    MeasuredBuild,
    RunFigures,
)
from tasks.shared.build_measurement.record_model import NoReading
from tasks.shared.build_measurement.settings import TimedRun, log_name
from tasks.shared.build_measurement.timeline import TaskOutcome
from tasks.shared.build_measurement.tracked_processes import (
    IDE,
    TrackedProcess,
)

AC_POWER = "AC Power"
RETAKE_WHEN_MISSING = ("syspolicyd", IDE, "fsnotifier")
_RETAKE_CAVEATS = frozenset({Caveat.ABSENT_IN_SOME_RUNS, Caveat.UNREADABLE})
_PATHS_SHOWN = 5


def _plural(count: int, noun: str) -> str:
    return f"{count} {noun}" if count == 1 else f"{count} {noun}s"


@dataclass(frozen=True, slots=True)
class Preflight:
    commit: str
    differing_paths: tuple[str, ...]
    host_settings: HostSettings
    power_source: str | NoReading
    mise_jobs: int | NoReading
    tracked: tuple[TrackedProcess, ...]
    running: tuple[str, ...]
    ide_commands: tuple[str, ...]
    keep: bool

    @property
    def matches(self) -> bool:
        return not self.differing_paths


def _working_copy(preflight: Preflight) -> str:
    if preflight.matches:
        return "working copy matches"
    paths = preflight.differing_paths
    shown = ", ".join(paths[:_PATHS_SHOWN])
    more = ", …" if len(paths) > _PATHS_SHOWN else ""
    return (
        f"working copy differs in {_plural(len(paths), 'path')}: {shown}{more}"
    )


def _preflight_warnings(preflight: Preflight) -> Iterator[str]:
    if preflight.host_settings is HostSettings.UNSPECIFIED:
        yield "the host settings are unspecified; pass --host-settings"
    if not preflight.matches:
        yield "the working copy does not match the measured commit"
    if preflight.power_source != AC_POWER:
        yield f"the host is on {preflight.power_source}, not {AC_POWER}"
    for name in RETAKE_WHEN_MISSING:
        if name not in preflight.running:
            yield f"{name} is not running"


def preflight_report(preflight: Preflight) -> str:
    not_running = [
        each.name
        for each in preflight.tracked
        if each.name not in preflight.running
    ]
    lines = [
        "preflight:",
        f"  commit: {preflight.commit}, {_working_copy(preflight)}",
        f"  host settings: {preflight.host_settings}",
        f"  power: {preflight.power_source}",
        f"  mise jobs: {preflight.mise_jobs}",
        f"  tracked processes running: {', '.join(preflight.running)}",
    ]
    if not_running:
        lines.append(
            f"  tracked processes not running: {', '.join(not_running)}"
        )
    if preflight.ide_commands:
        lines.append("  IDE command lines matched:")
        lines.extend(f"    {command}" for command in preflight.ide_commands)
    if preflight.keep:
        lines.extend(
            f"warning: with --keep, {warning}"
            for warning in _preflight_warnings(preflight)
        )
    return "\n".join(lines)


def _input_names(fetch_failures: Sequence[FetchFailure]) -> str:
    return ", ".join(
        dict.fromkeys(str(each.external_input) for each in fetch_failures)
    )


def run_end_line(
    number: int,
    total: int,
    wall_clock_seconds: float,
    *,
    failed: Sequence[str],
    fetch_failures: Sequence[FetchFailure],
    keep: bool,
) -> str:
    line = f"timed run {number}/{total}: {wall_clock_seconds:.1f} s, "
    line += (
        f"{len(failed)} failed ({', '.join(failed)})"
        if failed
        else "no task failed"
    )
    if fetch_failures:
        line += f"; fetch failure ({_input_names(fetch_failures)})"
        if keep:
            line += ": with --keep this record will not be kept"
            if number < total:
                line += "; press Ctrl-C to stop, then rerun"
    return line


@dataclass(frozen=True, slots=True)
class SessionOutcome:
    measured: MeasuredBuild
    keep: bool


def _header(measured: MeasuredBuild) -> Iterator[str]:
    conditions = measured.conditions
    runs = _plural(len(measured.runs), "run")
    yield (
        f"measure:build {measured.run_id} ({conditions.cache_state}, "
        f"{conditions.host_settings}, {runs})"
    )


def _commit(measured: MeasuredBuild) -> Iterator[str]:
    commit = measured.commit
    if commit.matched:
        match = "working copy matched at start and end"
    elif not commit.matched_at_start:
        match = "working copy did not match at start"
    elif commit.matched_at_end is None:
        match = "working copy match at end unknown (the end check failed)"
    else:
        match = "working copy did not match at end"
    yield f"commit: {commit.id}, {match}"


def _idle_load(measured: MeasuredBuild) -> Iterator[str]:
    load = measured.idle_load_percent
    yield f"idle load: {'not sampled' if load is None else f'{load:.1f}%'}"


def _wall_clock(measured: MeasuredBuild) -> Iterator[str]:
    spread = measured.summary.wall_clock_seconds
    yield (
        f"wall-clock: median {spread.median:.1f} s "
        f"(min {spread.minimum:.1f}, max {spread.maximum:.1f})"
    )


def _process_figure(name: str, across: ProcessAcrossRuns) -> str:
    presence = across.presence
    if isinstance(presence, Measured) and across.cpu_seconds is not None:
        return f"{name} {across.cpu_seconds.median:.1f} s"
    if isinstance(presence, Flagged):
        return f"{name} flagged ({', '.join(presence.caveats)})"
    return f"{name} absent"


def _retake_reason(across: ProcessAcrossRuns) -> str | None:
    presence = across.presence
    if isinstance(presence, Absent):
        return "was absent"
    if isinstance(presence, Flagged):
        caveats = [each for each in presence.caveats if each in _RETAKE_CAVEATS]
        if caveats:
            return f"was flagged {', '.join(caveats)}"
    return None


def _processes(measured: MeasuredBuild) -> Iterator[str]:
    processes = measured.summary.processes
    yield "processes: " + ", ".join(
        _process_figure(name, across) for name, across in processes.items()
    )
    for name, across in processes.items():
        reason = _retake_reason(across)
        if name in RETAKE_WHEN_MISSING and reason is not None:
            yield (
                f"retake: {name} {reason}; a baseline or condition record "
                "needs it measured in every run"
            )


def _first_run_failing(measured: MeasuredBuild, task: str) -> RunFigures:
    return next(run for run in measured.runs if task in run.failed())


def _failures(measured: MeasuredBuild, run_directory: Path) -> Iterator[str]:
    for task in measured.failed_tasks:
        run = _first_run_failing(measured, task)
        log = run_directory / log_name(TimedRun(run.index))
        flagged = sum(each.after_failure for each in run.tasks.values())
        yield (
            f"failed: {task} (run {run.index}, see {log}) — "
            f"downstream flagged: {_plural(flagged, 'task')}"
        )


def _output_contract_drift(measured: MeasuredBuild) -> Iterator[str]:
    for run in measured.runs:
        if run.exit_status != 0:
            continue
        for task, figures in run.tasks.items():
            if figures.outcome is TaskOutcome.NOT_RUN:
                yield (
                    f"warning: mise exited 0 in run {run.index}, yet {task} "
                    f"has no verdict; mise {measured.mise.version} may have "
                    "changed its output"
                )


def _fetch_failures(measured: MeasuredBuild) -> Iterator[str]:
    if measured.fetch_failures:
        yield "fetch failures: " + ", ".join(
            f"{each.external_input} ({each.task}, run {each.run})"
            for each in measured.fetch_failures
        )


def _paths(
    outcome: SessionOutcome, record_path: Path, kept_path: Path | None
) -> Iterator[str]:
    yield f"record: {record_path}"
    if kept_path is not None:
        yield f"kept: {kept_path}"
    elif outcome.keep and (failures := outcome.measured.fetch_failures):
        where = ", ".join(
            f"{each.external_input} in run {each.run}" for each in failures
        )
        yield (
            f"not kept: {_plural(len(failures), 'fetch failure')} "
            f"({where}); retake"
        )


def summary_report(
    outcome: SessionOutcome, *, record_path: Path, kept_path: Path | None
) -> str:
    measured = outcome.measured
    sections = (
        _header(measured),
        _commit(measured),
        _idle_load(measured),
        _wall_clock(measured),
        _processes(measured),
        _failures(measured, record_path.parent),
        _output_contract_drift(measured),
        _fetch_failures(measured),
        _paths(outcome, record_path, kept_path),
    )
    return "\n".join(line for section in sections for line in section)
