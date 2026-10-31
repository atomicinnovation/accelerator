"""Test doubles and record builders for the build-measurement suites."""

import datetime as dt
from collections.abc import Mapping, Sequence

from tasks.shared.build_measurement.cache_states import BuildCacheState
from tasks.shared.build_measurement.cpu_load import CpuBucket
from tasks.shared.build_measurement.fetch_failures import FetchFailure
from tasks.shared.build_measurement.host import (
    CoreCount,
    HostDescription,
    MiseDescription,
    PowerState,
)
from tasks.shared.build_measurement.locations import RUN_DIRECTORY_ROOT
from tasks.shared.build_measurement.measured_build import (
    CommitMatch,
    HostSettings,
    MeasuredBuild,
    Mode,
    PrimingRun,
    RunFigures,
)
from tasks.shared.build_measurement.record_model import NoReading
from tasks.shared.build_measurement.timeline import (
    CargoUnits,
    TaskFigures,
    TaskOutcome,
)
from tasks.shared.build_measurement.tracked_processes import (
    DEFAULT_TRACKED_PROCESSES,
    TrackedProcess,
)

REFERENCE_HOST = HostDescription(
    hostname="engywook",
    model="Mac16,5",
    chip="Apple M4 Max",
    os_version="26.3",
    os_build="25D125",
    cores=CoreCount(total=16, performance=12, efficiency=4),
    power=PowerState(source="AC Power", mode="0"),
)
REFERENCE_MISE = MiseDescription(
    version="2026.10.8 macos-arm64 (2026-10-11)", jobs=8
)
STARTED_AT = dt.datetime(2026, 10, 30, 16, 20, tzinfo=dt.UTC)
MEASURED_COMMIT = "d55cad6d85"


def task_figures(
    outcome: TaskOutcome = TaskOutcome.SUCCEEDED,
    *,
    start: float = 10.2,
    duration: float = 297.1,
    units: CargoUnits | None = None,
    nextest: float | None = None,
    after_failure: bool = False,
    exit_status: int | None = None,
) -> TaskFigures:
    printed = outcome in {TaskOutcome.SUCCEEDED, TaskOutcome.FAILED}
    if exit_status is None and outcome is TaskOutcome.SUCCEEDED:
        exit_status = 0
    return TaskFigures(
        outcome=outcome,
        exit_status=exit_status,
        start_seconds=start if printed else None,
        duration_seconds=duration if printed else None,
        end_seconds=start + duration if printed else None,
        units=units or CargoUnits(),
        lock_wait_seconds=0.0,
        nextest_phase_seconds=nextest,
        queueing_delay_seconds=0.4 if printed else None,
        after_failure=after_failure,
    )


def run_figures(
    index: int = 1,
    *,
    tasks: Mapping[str, TaskFigures] | None = None,
    wall_clock: float = 309.4,
    exit_status: int = 0,
    cpu_buckets: Sequence[CpuBucket] = (),
    process_cpu: Mapping[str, float | NoReading | None] | None = None,
    fetch_failures: Sequence[FetchFailure] = (),
) -> RunFigures:
    return RunFigures.derived(
        index=index,
        wall_clock_seconds=wall_clock,
        exit_status=exit_status,
        tasks=tasks if tasks is not None else {"build:cli": task_figures()},
        cpu_buckets=tuple(cpu_buckets),
        process_cpu_seconds=process_cpu
        if process_cpu is not None
        else {each.name: 10.0 for each in DEFAULT_TRACKED_PROCESSES},
        fetch_failures=tuple(fetch_failures),
    )


def measured_build(
    *,
    runs: Sequence[RunFigures] | None = None,
    host: HostDescription = REFERENCE_HOST,
    mise: MiseDescription = REFERENCE_MISE,
    cache_state: BuildCacheState = BuildCacheState.WARM_NO_OP,
    mode: Mode = Mode.FULL,
    isolated_tasks: tuple[str, ...] = (),
    host_settings: HostSettings = HostSettings.DEVTOOLS,
    commit: CommitMatch | None = None,
    idle_load_percent: float | None = 26.8,
    priming: PrimingRun | None = None,
    tracked: Sequence[TrackedProcess] = DEFAULT_TRACKED_PROCESSES,
    started_at: dt.datetime = STARTED_AT,
    run_id: str = "build_2026-10-30T162000Z_engywook_warm-no-op_devtools",
) -> MeasuredBuild:
    runs = tuple(runs) if runs is not None else (run_figures(1),)
    return MeasuredBuild.derived(
        run_id=run_id,
        run_directory=RUN_DIRECTORY_ROOT / run_id,
        started_at=started_at,
        cache_state=cache_state,
        mode=mode,
        isolated_tasks=isolated_tasks,
        host_settings=host_settings,
        commit=commit
        or CommitMatch.of(MEASURED_COMMIT, at_start=True, at_end=True),
        host=host,
        mise=mise,
        idle_load_percent=idle_load_percent,
        task_set=tuple(sorted(runs[0].tasks)),
        tracked=tracked,
        priming=priming
        if priming is not None
        else PrimingRun(wall_clock_seconds=318.2, discarded=True),
        runs=runs,
    )
