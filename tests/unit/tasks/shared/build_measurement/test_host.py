from collections.abc import Mapping, Sequence

import pytest

from tasks.shared.build_measurement.errors import UnsupportedPlatformError
from tasks.shared.build_measurement.host import (
    CoreCount,
    HostDescription,
    MiseDescription,
    PowerState,
    observed_build_host,
    observed_mise,
    require_macos,
)
from tasks.shared.build_measurement.record_model import NoReading

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

_REFERENCE_HOST: Mapping[tuple[str, ...], str] = {
    _HOSTNAME: "engywook",
    _MODEL: "Mac16,5",
    _CHIP: "Apple M4 Max",
    _OS_VERSION: "26.3",
    _OS_BUILD: "25D125",
    _CORES: "16\n12\n4",
    _POWER_SOURCE: (
        "Now drawing from 'AC Power'\n"
        " -InternalBattery-0 (id=22347875)\t100%; charged; 0:00 remaining"
        " present: true"
    ),
    _POWER_SETTINGS: (
        "System-wide power settings:\nCurrently in use:\n"
        " standby              1\n powermode            0\n"
        " sleep                1"
    ),
    _MISE_VERSION: "2026.10.8 macos-arm64 (2026-10-11)",
    _MISE_JOBS: "8",
}


_PROBES: Mapping[str, tuple[str, ...]] = {
    "hostname": _HOSTNAME,
    "model": _MODEL,
    "chip": _CHIP,
    "os_version": _OS_VERSION,
    "os_build": _OS_BUILD,
    "cores": _CORES,
    "power_source": _POWER_SOURCE,
    "power_settings": _POWER_SETTINGS,
    "mise_version": _MISE_VERSION,
    "mise_jobs": _MISE_JOBS,
}


def _diagnostics(**overrides: str | None):
    readings = dict(_REFERENCE_HOST)
    for probe, value in overrides.items():
        argv = _PROBES[probe]
        if value is None:
            del readings[argv]
        else:
            readings[argv] = value

    def runner(argv: Sequence[str]) -> str:
        key = tuple(argv)
        if key not in readings:
            raise FileNotFoundError(argv[0])
        return readings[key]

    return runner


def test_the_host_is_described_from_its_probes():
    diagnostics = _diagnostics()
    assert observed_build_host(diagnostics) == HostDescription(
        hostname="engywook",
        model="Mac16,5",
        chip="Apple M4 Max",
        os_version="26.3",
        os_build="25D125",
        cores=CoreCount(total=16, performance=12, efficiency=4),
        power=PowerState(source="AC Power", mode="0"),
    )
    assert observed_mise(diagnostics) == MiseDescription(
        version="2026.10.8 macos-arm64 (2026-10-11)", jobs=8
    )


def test_a_probe_that_fails_names_what_it_tried():
    host = observed_build_host(_diagnostics(model=None))
    assert host.model == NoReading(argv=_MODEL)
    assert str(host.model) == "no reading from sysctl -n hw.model"


@pytest.mark.parametrize(
    ("probe", "output", "reading"),
    [
        ("cores", "16\n12", lambda host, _: host.cores.efficiency),
        ("power_settings", " standby 1", lambda host, _: host.power.mode),
        ("power_source", "Now drawing", lambda host, _: host.power.source),
        ("mise_jobs", "eight", lambda _, mise: mise.jobs),
        ("hostname", "", lambda host, _: host.hostname),
        ("model", "", lambda host, _: host.model),
        ("chip", "", lambda host, _: host.chip),
        ("os_version", "", lambda host, _: host.os_version),
        ("os_build", "", lambda host, _: host.os_build),
        ("mise_version", "", lambda _, mise: mise.version),
    ],
)
def test_each_unreadable_reading_names_what_it_tried(probe, output, reading):
    diagnostics = _diagnostics(**{probe: output})
    observed = reading(
        observed_build_host(diagnostics), observed_mise(diagnostics)
    )
    assert observed == NoReading(argv=_PROBES[probe])


def test_the_hostname_comes_from_scutil_local_host_name():
    host = observed_build_host(_diagnostics(hostname="tobys-laptop"))
    assert host.hostname == "tobys-laptop"


def test_power_source_comes_from_pmset_ps():
    host = observed_build_host(
        _diagnostics(power_source="Now drawing from 'Battery Power'")
    )
    assert host.power.source == "Battery Power"


def test_power_mode_comes_from_pmset_g():
    host = observed_build_host(
        _diagnostics(power_settings=" powermode            2")
    )
    assert host.power.mode == "2"


def test_the_job_limit_comes_from_mise_settings():
    assert observed_mise(_diagnostics(mise_jobs="4")).jobs == 4


def test_a_non_macos_platform_is_refused_naming_macos():
    with pytest.raises(
        UnsupportedPlatformError,
        match="measure:build supports macOS only; this host is Linux",
    ):
        require_macos("Linux")


def test_macos_is_accepted():
    require_macos("Darwin")
