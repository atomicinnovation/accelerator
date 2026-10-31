"""Test doubles and record builders for the build-measurement suites."""

import contextlib
import datetime as dt
import json
from collections.abc import Iterator, Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path

from tasks.shared.build_measurement.analysis import BuildResult
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
from tasks.shared.build_measurement.session import (
    BuildPorts,
    OrphanedBuild,
    measure_build,
)
from tasks.shared.build_measurement.settings import BuildSettings
from tasks.shared.build_measurement.timeline import (
    CargoUnits,
    StampedLine,
    TaskFigures,
    TaskOutcome,
    stamped_text,
)
from tasks.shared.build_measurement.tracked_processes import (
    DEFAULT_TRACKED_PROCESSES,
    PS_ARGV,
    TrackedProcess,
)
from tests.unit.tasks.shared.doubles import FakeClock

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


@dataclass
class FakeHistory:
    chain: tuple[str, ...] = (MEASURED_COMMIT,)
    changes: Mapping[str, tuple[str, ...]] = field(
        default_factory=lambda: {MEASURED_COMMIT: ("tasks/measure.py",)}
    )
    working_copy: tuple[str, ...] = ()
    later_working_copies: Sequence[tuple[str, ...] | Exception] = ()
    checks: int = 0

    def first_parent_chain(self) -> Iterator[str]:
        return iter(self.chain)

    def changed_paths(self, commit: str) -> tuple[str, ...]:
        return self.changes.get(commit, ())

    def working_copy_changes(self, commit: str) -> tuple[str, ...]:
        del commit
        self.checks += 1
        later = self.checks - 2
        if 0 <= later < len(self.later_working_copies):
            answer = self.later_working_copies[later]
            if isinstance(answer, Exception):
                raise answer
            return answer
        return self.working_copy


def fake_diagnostics(table: Mapping[tuple[str, ...], str], calls=None):
    def diagnostics(argv):
        argv = tuple(argv)
        if calls is not None:
            calls.append(argv)
        if argv not in table:
            raise FileNotFoundError(argv[0])
        answer = table[argv]
        if isinstance(answer, Exception):
            raise answer
        return answer

    return diagnostics


@dataclass(frozen=True)
class BuildScript:
    lines: tuple[tuple[float, str], ...] = ()
    exit_status: int = 0
    wall_clock: float = 300.0
    raises: type[BaseException] | None = None


def succeeded(task, at, seconds, *output):
    return (
        (at, f"[{task}] $ invoke {task}"),
        *((at + 0.01, f"[{task}] {line}") for line in output),
        (at + seconds, f"[{task}] Finished in {seconds}s"),
    )


def failed(task, at, status, *output):
    return (
        (at, f"[{task}] $ invoke {task}"),
        *((at + 0.01, f"[{task}] {line}") for line in output),
        (at + 0.5, f"[{task}] exited with status {status}"),
    )


def skipped(task, at):
    return ((at, f"[{task}] sources up-to-date, skipping"),)


def _write_stamped(log: Path, lines) -> None:
    log.write_text("".join(f"{stamped_text(line)}\n" for line in lines))


class ScriptedBuildRunner:
    def __init__(self, clock, scripts, events=None):
        self.clock = clock
        self.scripts = list(scripts)
        self.events = events if events is not None else []
        self.calls: list[tuple[tuple[str, ...], Path]] = []

    def run(self, argv, *, epoch, log):
        script = self.scripts[len(self.calls)]
        self.calls.append((tuple(argv), log))
        self.events.append(f"build {log.name}")
        if script.raises is not None:
            raise script.raises()
        offset = self.clock.now() - epoch
        lines = tuple(
            StampedLine(at=offset + at, text=text) for at, text in script.lines
        )
        _write_stamped(log, lines)
        self.clock.t += script.wall_clock
        return BuildResult(
            lines=lines,
            exit_status=script.exit_status,
            wall_clock_seconds=script.wall_clock,
        )


@dataclass(frozen=True)
class SamplerStart:
    epoch: float
    log: Path
    tracked: tuple | None


class RunningScriptedSampler:
    def __init__(self, log, lines, events, name):
        self.log = log
        self.lines = lines
        self.events = events
        self.name = name
        self.stops = 0

    def stop(self):
        if self.stops == 0:
            self.events.append(f"stop {self.name} {self.log.name}")
            _write_stamped(self.log, self.lines)
        self.stops += 1
        return self.lines


class ScriptedSampler:
    def __init__(self, clock, name, lines=(), events=None, start_delay=0.0):
        self.clock = clock
        self.name = name
        self.lines = tuple(StampedLine(at=at, text=text) for at, text in lines)
        self.events = events if events is not None else []
        self.start_delay = start_delay
        self.started: list[SamplerStart] = []
        self.running: list[RunningScriptedSampler] = []

    def start(self, *, epoch, log, tracked=None):
        self.started.append(
            SamplerStart(
                epoch, log, None if tracked is None else tuple(tracked)
            )
        )
        self.events.append(f"start {self.name} {log.name}")
        self.clock.t += self.start_delay
        running = RunningScriptedSampler(
            log, self.lines, self.events, self.name
        )
        self.running.append(running)
        return running


class FakeBuildGroup:
    def __init__(self, orphan=None, events=None):
        self.orphan = orphan
        self.events = events if events is not None else []
        self.remembered: list[tuple[int, float]] = []
        self.forgotten = 0

    def remember(self, pgid, create_time):
        self.remembered.append((pgid, create_time))

    def forget(self):
        self.forgotten += 1

    def orphaned(self):
        self.events.append("orphan check")
        return self.orphan


class FakeSessionLock:
    def __init__(self, events, refusal=None):
        self.events = events
        self.refusal = refusal

    @contextlib.contextmanager
    def held(self):
        self.events.append("lock")
        if self.refusal is not None:
            raise self.refusal
        try:
            yield
        finally:
            self.events.append("unlock")


def _untouched(name):
    def fail(*args, **kwargs):
        raise AssertionError(f"the {name} port was touched")

    return fail


class _Untouchable:
    def __init__(self, name):
        self.name = name

    def __getattr__(self, attribute):
        return _untouched(f"{self.name}.{attribute}")


def untouchable_ports(**overrides) -> BuildPorts:
    ports = {
        "system": _untouched("system"),
        "build": _Untouchable("build"),
        "cpu_sampler": _Untouchable("cpu_sampler"),
        "process_sampler": _Untouchable("process_sampler"),
        "history": _Untouchable("history"),
        "diagnostics": _untouched("diagnostics"),
        "kernel_hostname": _untouched("kernel_hostname"),
        "mise_tasks": _untouched("mise_tasks"),
        "clock": _Untouchable("clock"),
        "utc_now": _untouched("utc_now"),
        "progress": _untouched("progress"),
        "session_lock": _untouched("session_lock"),
        "build_group": _Untouchable("build_group"),
    }
    return BuildPorts(**(ports | overrides))


GRAPH_JSON = json.dumps(
    [
        {"name": "default", "depends": ["report", "test"], "run": []},
        {"name": "build", "depends": [], "run": ["invoke build"]},
        {"name": "test", "depends": ["build"], "run": ["invoke test"]},
        {"name": "report", "depends": ["test"], "run": ["invoke report"]},
        {"name": "measure:build", "depends": [], "run": ["invoke m"]},
    ]
)
IDE_COMMAND = "/Applications/IntelliJ IDEA.app/Contents/MacOS/idea"
PS_OUTPUT = (
    "  479     1   3:21.94 /usr/libexec/syspolicyd\n"
    f"  900     1  10:00.00 {IDE_COMMAND}\n"
    "  901   900   0:01.00 /Applications/IntelliJ IDEA.app/Contents/bin/"
    "fsnotifier\n"
)
PROBES = {
    ("scutil", "--get", "LocalHostName"): "engywook",
    ("sysctl", "-n", "hw.model"): "Mac16,5",
    ("sysctl", "-n", "machdep.cpu.brand_string"): "Apple M4 Max",
    ("sw_vers", "-productVersion"): "26.3",
    ("sw_vers", "-buildVersion"): "25D125",
    (
        "sysctl",
        "-n",
        "hw.ncpu",
        "hw.perflevel0.logicalcpu",
        "hw.perflevel1.logicalcpu",
    ): "16\n12\n4",
    ("pmset", "-g", "ps"): "Now drawing from 'AC Power'",
    ("pmset", "-g"): " powermode            0",
    ("mise", "--version"): "2026.10.8 macos-arm64 (2026-10-11)",
    ("mise", "settings", "get", "jobs"): "8",
    PS_ARGV: PS_OUTPUT,
}
CLEAN_BUILD = BuildScript(
    lines=(
        *succeeded("build", 0.0, 2.0),
        *succeeded("test", 2.5, 3.0),
        *succeeded("report", 5.6, 0.4),
    ),
    wall_clock=6.5,
)
TEST_FAILS = BuildScript(
    lines=(
        *succeeded("build", 0.0, 2.0),
        *failed("test", 2.5, 3, "boom"),
        *succeeded("report", 5.6, 0.4),
    ),
    exit_status=3,
    wall_clock=6.5,
)
ADVISORY_FAILS = BuildScript(
    lines=(
        *CLEAN_BUILD.lines,
        (6.1, "[report] curl: (6) Could not resolve host"),
    ),
    wall_clock=6.5,
)
CPU_LINES = (
    (0.0, "  us sy id"),
    (0.0, " 1 2 3 4 5 6 7 8 9"),
    (2.0, " 1 2 3 4 20 6 74 1 1 1"),
)


@dataclass
class FakeSession:
    tmp_path: object
    scripts: list = field(default_factory=lambda: [CLEAN_BUILD] * 4)
    history: FakeHistory = field(default_factory=FakeHistory)
    probes: dict = field(default_factory=lambda: dict(PROBES))
    orphan: OrphanedBuild | None = None
    lock_refusal: Exception | None = None
    sampler_start_delay: float = 0.0
    events: list = field(default_factory=list)
    progress: list = field(default_factory=list)
    diagnostics_calls: list = field(default_factory=list)

    def __post_init__(self):
        self.clock = FakeClock(start=100.0)
        self.build = ScriptedBuildRunner(self.clock, self.scripts, self.events)
        self.cpu_sampler = ScriptedSampler(
            self.clock,
            "cpu",
            CPU_LINES,
            self.events,
            start_delay=self.sampler_start_delay,
        )
        self.process_sampler = ScriptedSampler(
            self.clock,
            "processes",
            events=self.events,
            start_delay=self.sampler_start_delay,
        )
        self.build_group = FakeBuildGroup(self.orphan, self.events)
        self.lock = FakeSessionLock(self.events, self.lock_refusal)

    def ports(self, **overrides) -> BuildPorts:
        def system():
            self.events.append("platform")
            return "Darwin"

        def progress(line):
            self.events.append(f"progress {line.splitlines()[0]}")
            self.progress.append(line)

        ports = {
            "system": system,
            "build": self.build,
            "cpu_sampler": self.cpu_sampler,
            "process_sampler": self.process_sampler,
            "history": self.history,
            "diagnostics": fake_diagnostics(
                self.probes, self.diagnostics_calls
            ),
            "kernel_hostname": lambda: "kernel-host",
            "mise_tasks": lambda: GRAPH_JSON,
            "clock": self.clock,
            "utc_now": lambda: dt.datetime(
                2026, 10, 30, 16, 20, 0, 123456, tzinfo=dt.UTC
            ),
            "progress": progress,
            "session_lock": self.lock.held,
            "build_group": self.build_group,
        }
        return BuildPorts(**(ports | overrides))

    def settings(self, *, runs=3, keep=False, ide_patterns=()):
        return BuildSettings.of(
            root=self.tmp_path,
            runs=runs,
            keep=keep,
            host_settings="devtools",
            ide_patterns=ide_patterns,
        )

    def measure(self, **settings):
        return measure_build(self.settings(**settings), self.ports())

    @property
    def run_directory(self):
        [directory] = (self.tmp_path / RUN_DIRECTORY_ROOT).iterdir()
        return directory
