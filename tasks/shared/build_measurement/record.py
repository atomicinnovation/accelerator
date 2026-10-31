import datetime as dt
import re
from dataclasses import dataclass
from pathlib import Path

from pydantic import ValidationError

from tasks.shared.build_measurement.cache_states import BuildCacheState
from tasks.shared.build_measurement.errors import (
    KeptRecordExistsError,
    UnreadableRecordError,
)
from tasks.shared.build_measurement.measured_build import (
    HostSettings,
    MeasuredBuild,
)
from tasks.shared.files import atomic_write_text
from tasks.shared.paths import MEASUREMENTS_RELATIVE

RUN_ID_PREFIX = "build_"
RECORD_NAME = "record.json"
_KEPT_TEMPORARY_NAME = "kept.json.tmp"
_NOT_A_HOSTNAME_CHARACTER = re.compile(r"[^A-Za-z0-9-]")


def run_id(
    started_at: dt.datetime,
    hostname: str,
    *,
    cache_state: BuildCacheState,
    host_settings: HostSettings,
) -> str:
    fields = (
        f"{started_at.astimezone(dt.UTC):%Y-%m-%dT%H%M%SZ}",
        _NOT_A_HOSTNAME_CHARACTER.sub("-", hostname),
        str(cache_state),
        str(host_settings),
    )
    return RUN_ID_PREFIX + "_".join(fields)


def record_text(measured: MeasuredBuild) -> str:
    return measured.model_dump_json(indent=2) + "\n"


def parse_record(text: str, *, source: Path) -> MeasuredBuild:
    try:
        return MeasuredBuild.model_validate_json(text)
    except ValidationError as error:
        first = error.errors()[0]
        location = ".".join(str(part) for part in first["loc"]) or "record"
        raise UnreadableRecordError(
            f"{source} is not a readable build record: {location}: "
            f"{first['msg']}"
        ) from error


@dataclass(frozen=True, slots=True)
class WrittenRecord:
    record: Path
    kept: Path | None


def kept_path(measured: MeasuredBuild, *, root: Path) -> Path:
    return root / MEASUREMENTS_RELATIVE / f"{measured.run_id}.json"


def _keep(text: str, *, kept: Path, run_directory: Path) -> None:
    kept.parent.mkdir(parents=True, exist_ok=True)
    temporary = run_directory / _KEPT_TEMPORARY_NAME
    temporary.write_text(text)
    try:
        kept.hardlink_to(temporary)
    except FileExistsError as error:
        raise KeptRecordExistsError(
            f"{kept} already exists and was left as it was; this session's "
            f"record is {run_directory / RECORD_NAME}"
        ) from error
    finally:
        temporary.unlink(missing_ok=True)


def write_measured_build(
    measured: MeasuredBuild, *, root: Path, keep: bool
) -> WrittenRecord:
    run_directory = root / measured.run_directory
    run_directory.mkdir(parents=True, exist_ok=True)
    text = record_text(measured)
    record = run_directory / RECORD_NAME
    atomic_write_text(record, text)
    if not keep or measured.fetch_failures:
        return WrittenRecord(record=record, kept=None)
    kept = kept_path(measured, root=root)
    _keep(text, kept=kept, run_directory=run_directory)
    return WrittenRecord(record=record, kept=kept)
