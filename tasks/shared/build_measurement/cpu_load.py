"""CPU busy figures from ``iostat -w <interval>`` stamped lines.

A report line ends ``us sy id 1m 5m 15m``, so user and system time are the
sixth and fifth fields from the end whatever disk columns precede them.
"""

import math
from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from itertools import islice
from statistics import fmean

from tasks.shared.build_measurement.record_model import RecordModel
from tasks.shared.build_measurement.timeline import (
    StampedLine,
    TaskFigures,
    TaskOutcome,
    Timeline,
)

_USER_FROM_END = -6
_SYSTEM_FROM_END = -5


@dataclass(frozen=True, slots=True)
class CpuReading:
    at: float
    busy_percent: float


class CpuBucket(RecordModel):
    start_seconds: float
    busy_percent: float | None
    running: tuple[str, ...]


def _report_fields(text: str) -> tuple[float, ...] | None:
    try:
        fields = tuple(float(field) for field in text.split())
    except ValueError:
        return None
    return fields if len(fields) >= -_USER_FROM_END else None


def _busy_percent(fields: Sequence[float]) -> float:
    return fields[_USER_FROM_END] + fields[_SYSTEM_FROM_END]


def cpu_readings(lines: Iterable[StampedLine]) -> tuple[CpuReading, ...]:
    reports = (
        (line.at, fields)
        for line in lines
        if (fields := _report_fields(line.text)) is not None
    )
    since_boot_average_skipped = islice(reports, 1, None)
    return tuple(
        CpuReading(at=at, busy_percent=_busy_percent(fields))
        for at, fields in since_boot_average_skipped
    )


def idle_load(readings: Sequence[CpuReading]) -> float | None:
    if not readings:
        return None
    return fmean(reading.busy_percent for reading in readings)


def _is_running(figures: TaskFigures, at: float) -> bool:
    if figures.outcome is TaskOutcome.SKIPPED:
        return False
    start, end = figures.start_seconds, figures.end_seconds
    return start is not None and end is not None and start <= at < end


def _bucket(
    readings: Sequence[CpuReading],
    timeline: Timeline,
    start: float,
    end: float,
) -> CpuBucket:
    inside = [r.busy_percent for r in readings if start <= r.at < end]
    midpoint = (start + end) / 2
    return CpuBucket(
        start_seconds=start,
        busy_percent=fmean(inside) if inside else None,
        running=tuple(
            sorted(
                task
                for task, figures in timeline.tasks.items()
                if _is_running(figures, midpoint)
            )
        ),
    )


def cpu_buckets(
    readings: Sequence[CpuReading],
    timeline: Timeline,
    *,
    until: float,
    width: float = 30.0,
) -> tuple[CpuBucket, ...]:
    starts = (index * width for index in range(math.ceil(until / width)))
    return tuple(
        _bucket(readings, timeline, start, min(start + width, until))
        for start in starts
    )
