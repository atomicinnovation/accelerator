from types import MappingProxyType

import pytest

from tasks.shared.build_measurement.cpu_load import (
    CpuBucket,
    CpuReading,
    cpu_buckets,
    cpu_readings,
    idle_load,
)
from tasks.shared.build_measurement.timeline import (
    CargoUnits,
    StampedLine,
    TaskFigures,
    TaskOutcome,
    Timeline,
)

_DISK_HEADER = "              disk0       cpu    load average"
_COLUMN_HEADER = "    KB/t  tps  MB/s  us sy id   1m   5m   15m"


def _iostat(*lines: tuple[float, str]) -> tuple[StampedLine, ...]:
    return tuple(StampedLine(at=at, text=text) for at, text in lines)


def _report(us: int, sy: int) -> str:
    return f"   10.77  944  9.93  {us} {sy} {100 - us - sy}  2.47 2.56 2.62"


def _readings(*pairs: tuple[float, float]) -> tuple[CpuReading, ...]:
    return tuple(CpuReading(at=at, busy_percent=busy) for at, busy in pairs)


def _task(
    outcome: TaskOutcome, start: float | None, end: float | None
) -> TaskFigures:
    return TaskFigures(
        outcome=outcome,
        exit_status=None,
        start_seconds=start,
        duration_seconds=None,
        end_seconds=end,
        units=CargoUnits(),
        lock_wait_seconds=0.0,
        nextest_phase_seconds=None,
        queueing_delay_seconds=None,
        after_failure=False,
    )


def _timeline(**tasks: TaskFigures) -> Timeline:
    return Timeline(MappingProxyType(tasks))


_NOTHING_RUNNING = _timeline()


def test_iostat_busy_is_user_plus_system():
    readings = cpu_readings(
        _iostat((0.0, _report(1, 1)), (2.0, _report(46, 4)))
    )

    assert readings == (CpuReading(at=2.0, busy_percent=50.0),)


def test_the_first_iostat_report_is_discarded_as_a_since_boot_average():
    readings = cpu_readings(
        _iostat(
            (0.0, _DISK_HEADER),
            (0.0, _COLUMN_HEADER),
            (0.0, _report(11, 9)),
            (2.0, _report(43, 57)),
        )
    )

    assert [reading.at for reading in readings] == [2.0]


def test_iostat_header_lines_are_skipped_wherever_they_repeat():
    readings = cpu_readings(
        _iostat(
            (0.0, _DISK_HEADER),
            (0.0, _COLUMN_HEADER),
            (0.0, _report(11, 9)),
            (2.0, _report(20, 5)),
            (4.0, _DISK_HEADER),
            (4.0, _COLUMN_HEADER),
            (4.0, _report(30, 10)),
        )
    )

    assert readings == _readings((2.0, 25.0), (4.0, 40.0))


def test_buckets_average_the_readings_inside_each_30_seconds():
    buckets = cpu_buckets(
        _readings((2.0, 10.0), (28.0, 30.0), (32.0, 80.0)),
        _NOTHING_RUNNING,
        until=60.0,
    )

    assert [bucket.busy_percent for bucket in buckets] == [20.0, 80.0]


def test_a_reading_on_a_boundary_belongs_to_the_later_bucket():
    buckets = cpu_buckets(_readings((30.0, 70.0)), _NOTHING_RUNNING, until=60.0)

    assert [bucket.busy_percent for bucket in buckets] == [None, 70.0]


def test_the_final_partial_bucket_is_kept():
    buckets = cpu_buckets(_readings((70.0, 50.0)), _NOTHING_RUNNING, until=75.0)

    assert [bucket.start_seconds for bucket in buckets] == [0.0, 30.0, 60.0]
    assert buckets[-1].busy_percent == 50.0


def test_a_bucket_with_no_reading_has_no_busy_figure():
    (bucket,) = cpu_buckets((), _NOTHING_RUNNING, until=30.0)

    assert bucket == CpuBucket(start_seconds=0.0, busy_percent=None, running=())


def test_a_bucket_lists_the_tasks_running_at_its_midpoint():
    buckets = cpu_buckets(
        (),
        _timeline(
            build=_task(TaskOutcome.SUCCEEDED, 0.0, 50.0),
            lint=_task(TaskOutcome.SUCCEEDED, 20.0, 50.0),
            docs=_task(TaskOutcome.SKIPPED, 15.0, 15.0),
        ),
        until=60.0,
    )

    # Midpoints at 15 and 45.
    assert [bucket.running for bucket in buckets] == [
        ("build",),
        ("build", "lint"),
    ]


def test_a_silent_task_is_never_running():
    buckets = cpu_buckets(
        (),
        _timeline(lint=_task(TaskOutcome.SILENT, None, None)),
        until=30.0,
    )

    assert buckets[0].running == ()


def test_a_failed_task_runs_until_its_last_counted_line():
    buckets = cpu_buckets(
        (),
        _timeline(test=_task(TaskOutcome.FAILED, 0.0, 40.0)),
        until=90.0,
    )

    # Midpoints at 15, 45 and 75.
    assert [bucket.running for bucket in buckets] == [("test",), (), ()]


def test_idle_load_is_the_mean_of_the_readings():
    assert idle_load(_readings((2.0, 20.0), (4.0, 30.0))) == pytest.approx(25.0)


def test_idle_load_with_no_reading_is_unknown():
    assert idle_load(()) is None
