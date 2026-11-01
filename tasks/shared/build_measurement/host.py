import re
from collections.abc import Callable

from tasks.shared.build_measurement.errors import UnsupportedPlatformError
from tasks.shared.build_measurement.record_model import NoReading, RecordModel
from tasks.shared.diagnostics import DiagnosticRunner

_HOSTNAME = ("scutil", "--get", "LocalHostName")
_MODEL = ("sysctl", "-n", "hw.model")
_CHIP = ("sysctl", "-n", "machdep.cpu.brand_string")
_OS_VERSION = ("sw_vers", "-productVersion")
_OS_BUILD = ("sw_vers", "-buildVersion")
_CORES = (
    "sysctl",
    "-n",
    "hw.ncpu",
    "hw.perflevel0.logicalcpu",
    "hw.perflevel1.logicalcpu",
)
_POWER_SOURCE = ("pmset", "-g", "ps")
_POWER_SETTINGS = ("pmset", "-g")
_MISE_VERSION = ("mise", "--version")
_MISE_JOBS = ("mise", "settings", "get", "jobs")

_QUOTED_POWER_SOURCE = re.compile(r"'([^']+)'")
_POWER_MODE = re.compile(r"^\s*powermode\s+(\S+)\s*$", re.MULTILINE)


class CoreCount(RecordModel):
    total: int | NoReading
    performance: int | NoReading
    efficiency: int | NoReading


class PowerState(RecordModel):
    source: str | NoReading
    mode: str | NoReading


class HostDescription(RecordModel):
    hostname: str | NoReading
    model: str | NoReading
    chip: str | NoReading
    os_version: str | NoReading
    os_build: str | NoReading
    cores: CoreCount
    power: PowerState


class MiseDescription(RecordModel):
    version: str | NoReading
    jobs: int | NoReading


def _reading[T](
    diagnostics: DiagnosticRunner,
    argv: tuple[str, ...],
    parse: Callable[[str], T | None],
) -> T | NoReading:
    try:
        output = diagnostics(argv).strip()
    except FileNotFoundError, PermissionError:
        return NoReading(argv=argv)
    parsed = parse(output) if output else None
    return NoReading(argv=argv) if parsed is None else parsed


def _verbatim(output: str) -> str:
    return output


def _integer(output: str) -> int | None:
    return int(output) if output.isdecimal() else None


def _first_quoted(output: str) -> str | None:
    first_line = output.splitlines()[0]
    match = _QUOTED_POWER_SOURCE.search(first_line)
    return match.group(1) if match else None


def _power_mode(output: str) -> str | None:
    match = _POWER_MODE.search(output)
    return match.group(1) if match else None


def _core_count(diagnostics: DiagnosticRunner) -> CoreCount:
    lines = _reading(diagnostics, _CORES, lambda output: output.splitlines())

    def line(index: int) -> int | NoReading:
        if isinstance(lines, NoReading) or index >= len(lines):
            return NoReading(argv=_CORES)
        count = _integer(lines[index].strip())
        return NoReading(argv=_CORES) if count is None else count

    return CoreCount(total=line(0), performance=line(1), efficiency=line(2))


def observed_build_host(diagnostics: DiagnosticRunner) -> HostDescription:
    return HostDescription(
        hostname=_reading(diagnostics, _HOSTNAME, _verbatim),
        model=_reading(diagnostics, _MODEL, _verbatim),
        chip=_reading(diagnostics, _CHIP, _verbatim),
        os_version=_reading(diagnostics, _OS_VERSION, _verbatim),
        os_build=_reading(diagnostics, _OS_BUILD, _verbatim),
        cores=_core_count(diagnostics),
        power=PowerState(
            source=_reading(diagnostics, _POWER_SOURCE, _first_quoted),
            mode=_reading(diagnostics, _POWER_SETTINGS, _power_mode),
        ),
    )


def observed_mise(diagnostics: DiagnosticRunner) -> MiseDescription:
    return MiseDescription(
        version=_reading(diagnostics, _MISE_VERSION, _verbatim),
        jobs=_reading(diagnostics, _MISE_JOBS, _integer),
    )


def require_macos(system: str) -> None:
    if system != "Darwin":
        raise UnsupportedPlatformError(
            f"measure:build supports macOS only; this host is {system}"
        )
