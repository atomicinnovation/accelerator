r"""Tracked host processes and their CPU, read from stamped ``ps`` ticks.

``ps-tick.golden`` is one tick captured on macOS 26.3 with IntelliJ IDEA open
on the project and PyCharm open alongside. A full tick lists every process on
the host with its arguments, so only whole lines were kept: every tracked
line, plus untracked near-misses (the IDEs' bundle helpers, PyCharm's main
executable, the other Xprotect services) and a few system daemons. No line
was edited. Re-capture it with::

    golden=tests/unit/tasks/fixtures/build-measurement/ps-tick.golden
    ps -ww -Ao pid=,ppid=,time=,command= > "$golden"
    tracked='(/usr/libexec/syspolicyd'
    tracked+='|.*/MacOS/XprotectService'
    tracked+='|.*/Contents/MacOS/(idea|rustrover)'
    tracked+='|.*/fsnotifier)(\s|$)'
    grep -cE "^\s*\S+\s+\S+\s+\S+\s+${tracked}" "$golden"

then delete whole untracked lines down to a similar sample, and set
``_TRACKED_LINES_IN_GOLDEN`` to the count ``grep`` prints.
"""

import pytest

from tasks.shared.build_measurement.record_model import NoReading
from tasks.shared.build_measurement.timeline import read_stamped
from tasks.shared.build_measurement.tracked_processes import (
    DEFAULT_TRACKED_PROCESSES,
    PS_ARGV,
    ObservedProcess,
    ProcessSamples,
    TrackedProcess,
    cpu_used,
    gap_marker,
    parse_ps_line,
    process_snapshots,
    running_tracked,
    sample_marker,
    tracked_line_filter,
    with_ide_patterns,
)
from tasks.shared.sources import repo_root

_GOLDEN = (
    repo_root() / "tests/unit/tasks/fixtures/build-measurement/ps-tick.golden"
)
_TRACKED_LINES_IN_GOLDEN = 5

_SYSPOLICYD = "/usr/libexec/syspolicyd"
_IDEA = "/Applications/IntelliJ IDEA.app/Contents/MacOS/idea"
_IDEA_FSNOTIFIER = "/Applications/IntelliJ IDEA.app/Contents/bin/fsnotifier"
_PYCHARM_FSNOTIFIER = "/Applications/PyCharm.app/Contents/bin/fsnotifier"


def _tracked(name: str) -> TrackedProcess:
    return next(t for t in DEFAULT_TRACKED_PROCESSES if t.name == name)


def _ps(pid: int, time: str, command: str) -> str:
    return f"{pid:>5} {1:>5} {time:>9} {command}"


def _samples(*ticks: tuple[str, ...] | None) -> ProcessSamples:
    lines: list[str] = []
    for at, tick in enumerate(ticks):
        if tick is None:
            lines.append(gap_marker(float(at)))
        else:
            lines.append(sample_marker(float(at)))
            lines.extend(f"{float(at)} {raw}" for raw in tick)
    return process_snapshots(read_stamped("\n".join(lines) + "\n"))


def _syspolicyd(pid: int, time: str) -> tuple[str, ...]:
    return (_ps(pid, time, _SYSPOLICYD),)


class TestPsTime:
    def test_ps_time_parses_minutes_beyond_sixty(self):
        observed = parse_ps_line(_ps(1, "115:00.06", _SYSPOLICYD))

        assert observed is not None
        assert observed.cpu_seconds == pytest.approx(6900.06)

    def test_ps_time_parses_hours_and_days(self):
        observed = parse_ps_line(_ps(1, "1-02:03:04.50", _SYSPOLICYD))

        assert observed is not None
        assert observed.cpu_seconds == pytest.approx(
            86400 + 2 * 3600 + 3 * 60 + 4.5
        )


class TestSnapshots:
    def test_the_markers_are_read_as_the_sampler_writes_them(self):
        samples = process_snapshots(
            read_stamped(f"{sample_marker(1.5)}\n{gap_marker(2.5)}\n")
        )

        assert [snapshot.at for snapshot in samples.snapshots] == [1.5]
        assert samples.gaps == 1

    def test_a_gap_marker_is_counted_not_read_as_a_snapshot(self):
        samples = _samples(_syspolicyd(412, "1:00.00"), None)

        assert len(samples.snapshots) == 1
        assert not samples.complete

    def test_a_tick_with_no_match_is_still_a_snapshot(self):
        samples = _samples((), ())

        assert len(samples.snapshots) == 2
        assert samples.complete


class TestCpuUsed:
    def test_a_pid_in_the_first_snapshot_counts_from_that_snapshot(self):
        samples = _samples(
            _syspolicyd(412, "1:00.00"), _syspolicyd(412, "1:30.00")
        )

        assert cpu_used(samples, _tracked("syspolicyd")) == pytest.approx(30.0)

    def test_a_pid_first_seen_mid_run_counts_from_zero(self):
        samples = _samples((), _syspolicyd(412, "0:12.00"))

        assert cpu_used(samples, _tracked("syspolicyd")) == pytest.approx(12.0)

    def test_a_failed_first_tick_makes_the_cpu_unreadable(self):
        samples = _samples(None, _syspolicyd(412, "51:03.22"))

        assert cpu_used(samples, _tracked("syspolicyd")) == NoReading(
            argv=PS_ARGV
        )

    def test_a_gap_mid_run_makes_the_cpu_unreadable(self):
        samples = _samples(
            _syspolicyd(412, "1:00.00"), None, _syspolicyd(412, "1:30.00")
        )

        assert isinstance(cpu_used(samples, _tracked("syspolicyd")), NoReading)

    def test_a_pid_that_exits_mid_run_keeps_its_last_seen_time(self):
        samples = _samples(
            _syspolicyd(412, "1:00.00"),
            _syspolicyd(412, "1:05.00"),
            _syspolicyd(412, "1:09.00"),
            (),
            (),
        )

        assert cpu_used(samples, _tracked("syspolicyd")) == pytest.approx(9.0)

    def test_a_matching_pid_with_no_increase_is_zero_not_absent(self):
        samples = _samples(
            _syspolicyd(412, "1:00.00"), _syspolicyd(412, "1:00.00")
        )

        assert cpu_used(samples, _tracked("syspolicyd")) == 0.0

    def test_matching_pids_are_summed(self):
        samples = _samples(
            (_ps(10, "0:01.00", _IDEA_FSNOTIFIER),),
            (
                _ps(10, "0:03.00", _IDEA_FSNOTIFIER),
                _ps(20, "0:04.00", _PYCHARM_FSNOTIFIER),
            ),
        )

        assert cpu_used(samples, _tracked("fsnotifier")) == pytest.approx(6.0)

    def test_a_process_never_seen_is_absent_not_zero(self):
        samples = _samples(_syspolicyd(412, "1:00.00"), ())

        assert cpu_used(samples, _tracked("XprotectService")) is None


class TestCatalogue:
    def test_the_ide_is_matched_by_its_main_executable_not_its_bundle(self):
        ide = _tracked("ide")

        assert ide.matches(_IDEA)
        assert not ide.matches(_IDEA_FSNOTIFIER)

    def test_every_fsnotifier_counts(self):
        fsnotifier = _tracked("fsnotifier")

        assert fsnotifier.matches(_IDEA_FSNOTIFIER)
        assert fsnotifier.matches(_PYCHARM_FSNOTIFIER)

    def test_ide_patterns_replace_only_the_ide_entry(self):
        overridden = with_ide_patterns(
            DEFAULT_TRACKED_PROCESSES, (r"/MacOS/clion(?:\s|$)",)
        )

        assert {t.name: t for t in overridden} == {
            **{t.name: t for t in DEFAULT_TRACKED_PROCESSES},
            "ide": TrackedProcess("ide", (r"/MacOS/clion(?:\s|$)",)),
        }


class TestPsLines:
    def test_a_ps_line_parses_into_pid_parent_time_and_command(self):
        observed = parse_ps_line(
            "  745     1 476:58.11 /Applications/IntelliJ IDEA.app"
            "/Contents/MacOS/idea -Xmx4g"
        )

        assert observed == ObservedProcess(
            pid=745,
            parent_pid=1,
            cpu_seconds=pytest.approx(28618.11),
            command=f"{_IDEA} -Xmx4g",
        )

    @pytest.mark.parametrize(
        "raw",
        ["", "412 1 1:00.00", "pid 1 1:00.00 /bin/x", "412 1 soon /bin/x"],
    )
    def test_an_unreadable_ps_line_parses_to_none(self, raw):
        assert parse_ps_line(raw) is None

    def test_running_tracked_names_each_tracked_process_present(self):
        output = "\n".join(
            (
                _ps(412, "1:00.00", _SYSPOLICYD),
                _ps(
                    800,
                    "0:10.00",
                    "/Applications/CLion.app/Contents/MacOS/clion",
                ),
                _ps(745, "9:00.00", _IDEA),
            )
        )
        tracked = with_ide_patterns(
            DEFAULT_TRACKED_PROCESSES, (r"/MacOS/clion(?:\s|$)",)
        )

        assert running_tracked(output, tracked) == ("syspolicyd", "ide")

    def test_the_filter_matches_the_command_behind_padded_columns(self):
        keep = tracked_line_filter(DEFAULT_TRACKED_PROCESSES)

        assert keep("  412     1  51:03.22 /usr/libexec/syspolicyd")

    def test_the_filter_drops_untracked_lines(self):
        keep = tracked_line_filter(DEFAULT_TRACKED_PROCESSES)

        assert not keep("  401     1 354:01.90 /usr/sbin/WindowServer")

    def test_the_filter_keeps_exactly_the_tracked_lines_of_a_real_tick(self):
        keep = tracked_line_filter(DEFAULT_TRACKED_PROCESSES)

        kept = [raw for raw in _GOLDEN.read_text().splitlines() if keep(raw)]

        assert len(kept) == _TRACKED_LINES_IN_GOLDEN
