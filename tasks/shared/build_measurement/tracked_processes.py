import re
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass, replace

from tasks.shared.build_measurement.record_model import NoReading
from tasks.shared.build_measurement.timeline import StampedLine

IDE = "ide"


@dataclass(frozen=True, slots=True)
class TrackedProcess:
    name: str
    patterns: tuple[str, ...]

    def matches(self, command: str) -> bool:
        return any(re.search(pattern, command) for pattern in self.patterns)


DEFAULT_TRACKED_PROCESSES = (
    TrackedProcess("syspolicyd", (r"^/usr/libexec/syspolicyd(?:\s|$)",)),
    TrackedProcess("XprotectService", (r"/MacOS/XprotectService(?:\s|$)",)),
    TrackedProcess(
        IDE,
        (
            r"/Contents/MacOS/idea(?:\s|$)",
            r"/Contents/MacOS/rustrover(?:\s|$)",
        ),
    ),
    TrackedProcess("fsnotifier", (r"/fsnotifier(?:\s|$)",)),
)


@dataclass(frozen=True, slots=True)
class ObservedProcess:
    pid: int
    parent_pid: int
    cpu_seconds: float
    command: str


@dataclass(frozen=True, slots=True)
class ProcessSnapshot:
    at: float
    processes: tuple[ObservedProcess, ...]


@dataclass(frozen=True, slots=True)
class ProcessSamples:
    snapshots: tuple[ProcessSnapshot, ...]
    gaps: int

    @property
    def complete(self) -> bool:
        return self.gaps == 0


PS_ARGV = ("ps", "-ww", "-Ao", "pid=,ppid=,time=,command=")

_SAMPLE = "sample"
_GAP = "gap"
_PS_FIELDS = 4
_PS_TIME = re.compile(
    r"(?:(?:(?P<days>\d+)-)?(?P<hours>\d+):)?(?P<minutes>\d+):"
    r"(?P<seconds>\d+(?:\.\d+)?)"
)
_SECONDS_PER_MINUTE = 60
_SECONDS_PER_HOUR = 60 * _SECONDS_PER_MINUTE
_SECONDS_PER_DAY = 24 * _SECONDS_PER_HOUR


def sample_marker(at: float) -> str:
    return f"{at} {_SAMPLE}"


def gap_marker(at: float) -> str:
    return f"{at} {_GAP}"


def _ps_time(text: str) -> float | None:
    time = _PS_TIME.fullmatch(text)
    if time is None:
        return None
    return (
        int(time["days"] or 0) * _SECONDS_PER_DAY
        + int(time["hours"] or 0) * _SECONDS_PER_HOUR
        + int(time["minutes"]) * _SECONDS_PER_MINUTE
        + float(time["seconds"])
    )


def parse_ps_line(raw: str) -> ObservedProcess | None:
    fields = raw.split(maxsplit=_PS_FIELDS - 1)
    if len(fields) != _PS_FIELDS:
        return None
    pid, parent_pid, time, command = fields
    cpu_seconds = _ps_time(time)
    if not (pid.isdigit() and parent_pid.isdigit()) or cpu_seconds is None:
        return None
    return ObservedProcess(
        pid=int(pid),
        parent_pid=int(parent_pid),
        cpu_seconds=cpu_seconds,
        command=command,
    )


def process_snapshots(lines: Iterable[StampedLine]) -> ProcessSamples:
    snapshots: list[ProcessSnapshot] = []
    gaps = 0
    for line in lines:
        if line.text == _SAMPLE:
            snapshots.append(ProcessSnapshot(at=line.at, processes=()))
        elif line.text == _GAP:
            gaps += 1
        elif snapshots and (observed := parse_ps_line(line.text)) is not None:
            latest = snapshots[-1]
            snapshots[-1] = replace(
                latest, processes=(*latest.processes, observed)
            )
    return ProcessSamples(snapshots=tuple(snapshots), gaps=gaps)


def _is_tracked(
    observed: ObservedProcess | None, tracked: Sequence[TrackedProcess]
) -> bool:
    return observed is not None and any(
        each.matches(observed.command) for each in tracked
    )


def tracked_line_filter(
    tracked: Sequence[TrackedProcess],
) -> Callable[[str], bool]:
    return lambda raw: _is_tracked(parse_ps_line(raw), tracked)


def _commands(ps_output: str) -> tuple[str, ...]:
    return tuple(
        observed.command
        for raw in ps_output.splitlines()
        if (observed := parse_ps_line(raw)) is not None
    )


def running_tracked(
    ps_output: str, tracked: Sequence[TrackedProcess]
) -> tuple[str, ...]:
    commands = _commands(ps_output)
    return tuple(
        each.name
        for each in tracked
        if any(each.matches(command) for command in commands)
    )


def matching_commands(
    ps_output: str, tracked: TrackedProcess
) -> tuple[str, ...]:
    return tuple(
        command for command in _commands(ps_output) if tracked.matches(command)
    )


def with_ide_patterns(
    tracked: Sequence[TrackedProcess], patterns: Sequence[str]
) -> tuple[TrackedProcess, ...]:
    return tuple(
        TrackedProcess(IDE, tuple(patterns)) if each.name == IDE else each
        for each in tracked
    )


def cpu_used(
    sampled: ProcessSamples, tracked: TrackedProcess
) -> float | NoReading | None:
    if not sampled.complete:
        return NoReading(argv=PS_ARGV)
    first_seen: dict[int, float] = {}
    last_seen: dict[int, float] = {}
    for index, snapshot in enumerate(sampled.snapshots):
        for observed in snapshot.processes:
            if tracked.matches(observed.command):
                if index == 0:
                    first_seen[observed.pid] = observed.cpu_seconds
                last_seen[observed.pid] = observed.cpu_seconds
    if not last_seen:
        return None
    return sum(
        cpu_seconds - first_seen.get(pid, 0.0)
        for pid, cpu_seconds in last_seen.items()
    )
