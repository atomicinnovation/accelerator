"""Finding the nextest logs whose per-test durations are measured."""

from collections.abc import Sequence
from pathlib import Path

from tasks.shared.build_measurement.errors import MeasurementRefusedError
from tasks.shared.build_measurement.test_durations import TimedRun
from tasks.shared.build_measurement.timeline import read_stamped


def timed_runs(*, record: Path | None, logs: Sequence[str]) -> list[TimedRun]:
    """Read the runs of a build record's directory, or of alone-run logs.

    A record's `run-N-test_unit_cli.log` files are read in run order, each
    line stamped with its time; an alone-run log carries no stamps.
    """
    if (record is None) == (not logs):
        raise MeasurementRefusedError("read from either --record or --log")
    if record is not None:
        paths = sorted(record.glob("run-*-test_unit_cli.log"), key=_run_number)
        if not paths:
            raise MeasurementRefusedError(
                f"no run-*-test_unit_cli.log under {record}"
            )
        return [TimedRun.parse(read_stamped(_text(path))) for path in paths]
    return [TimedRun.parse_unstamped(_text(path)) for path in _matched(logs)]


def _run_number(path: Path) -> int:
    return int(path.name.split("-")[1])


def _matched(patterns: Sequence[str]) -> list[Path]:
    paths: list[Path] = []
    for pattern in patterns:
        path = Path(pattern)
        matched = sorted(path.parent.glob(path.name))
        if not matched:
            raise MeasurementRefusedError(f"no log matches {pattern}")
        paths.extend(matched)
    return paths


def _text(path: Path) -> str:
    return path.read_text(errors="replace", newline="")
