import json
from pathlib import Path

import pytest

from tasks.shared.build_measurement.cache_states import BuildCacheState
from tasks.shared.build_measurement.cpu_load import CpuBucket
from tasks.shared.build_measurement.errors import (
    KeptRecordExistsError,
    UnreadableRecordError,
)
from tasks.shared.build_measurement.fetch_failures import (
    ExternalInput,
    FetchFailure,
)
from tasks.shared.build_measurement.host import CoreCount, HostDescription
from tasks.shared.build_measurement.locations import RUN_DIRECTORY_ROOT
from tasks.shared.build_measurement.measured_build import (
    CONDITIONS_MIRRORING_THE_HOST,
    CommitMatch,
    HostSettings,
)
from tasks.shared.build_measurement.record import (
    RUN_ID_PREFIX,
    parse_record,
    record_text,
    run_id,
    write_measured_build,
)
from tasks.shared.build_measurement.record_model import NoReading
from tasks.shared.build_measurement.timeline import CargoUnits, TaskOutcome
from tasks.shared.paths import MEASUREMENTS_RELATIVE, REPO_ROOT
from tests.unit.tasks.shared.build_measurement.doubles import (
    REFERENCE_HOST,
    STARTED_AT,
    measured_build,
    run_figures,
    task_figures,
)

_SOURCE = Path("record.json")
_BASELINE: Path | None = None


def _committed_build_records() -> list[Path]:
    return sorted(
        (REPO_ROOT / MEASUREMENTS_RELATIVE).glob(f"{RUN_ID_PREFIX}*.json")
    )


def _document(measured=None) -> dict:
    return json.loads(record_text(measured or measured_build()))


def _parsed(document: dict):
    return parse_record(json.dumps(document), source=_SOURCE)


class TestRunId:
    def test_the_run_id_names_the_time_host_cache_state_and_label(self):
        assert (
            run_id(
                STARTED_AT,
                "engywook",
                cache_state=BuildCacheState.WARM_NO_OP,
                host_settings=HostSettings.DEVTOOLS,
            )
            == "build_2026-10-30T162000Z_engywook_warm-no-op_devtools"
        )

    @pytest.mark.parametrize(
        ("hostname", "expected"),
        [("toby's_mac.local", "toby-s-mac-local"), ("engy-wook", "engy-wook")],
    )
    def test_the_hostname_is_reduced_to_letters_digits_and_hyphens(
        self, hostname, expected
    ):
        identifier = run_id(
            STARTED_AT,
            hostname,
            cache_state=BuildCacheState.WARM_NO_OP,
            host_settings=HostSettings.DEVTOOLS_AND_IDE_EXCLUSION,
        )
        assert identifier.split("_")[2] == expected
        assert identifier.endswith("_devtools+ide-exclusion")

    @pytest.mark.parametrize(
        ("hostname", "label"),
        [
            ("engy-wook", HostSettings.DEVTOOLS),
            ("engywook", HostSettings.DEVTOOLS_AND_IDE_EXCLUSION),
        ],
    )
    def test_each_field_of_a_run_id_is_recovered_by_splitting_on_underscores(
        self, hostname, label
    ):
        identifier = run_id(
            STARTED_AT,
            hostname,
            cache_state=BuildCacheState.WARM_NO_OP,
            host_settings=label,
        )
        assert identifier.split("_") == [
            "build",
            "2026-10-30T162000Z",
            hostname,
            "warm-no-op",
            str(label),
        ]


class TestRecordText:
    def test_the_top_level_keys_come_in_the_documented_order(self):
        assert list(_document()) == [
            "schema_version",
            "run_id",
            "run_directory",
            "started_at",
            "conditions",
            "commit",
            "host",
            "mise",
            "idle_load_percent",
            "task_set",
            "tracked_processes",
            "priming",
            "runs",
            "summary",
            "fetch_failures",
            "failed_tasks",
        ]

    def test_started_at_is_written_with_an_offset_and_whole_seconds(self):
        assert _document()["started_at"] == "2026-10-30T16:20:00+00:00"

    def test_the_record_names_its_run_directory_relative_to_the_root(self):
        document = _document()
        assert document["run_directory"] == str(
            RUN_DIRECTORY_ROOT / document["run_id"]
        )

    def test_one_run_serialises_as_the_record_example_shows(self):
        run = run_figures(
            tasks={
                "test:unit:cli": task_figures(
                    units=CargoUnits(compiling=44), nextest=149.2
                )
            },
            cpu_buckets=[
                CpuBucket(
                    start_seconds=0.0,
                    busy_percent=91.2,
                    running=("test:unit:cli",),
                )
            ],
            process_cpu={
                "syspolicyd": 51.3,
                "XprotectService": None,
                "ide": 301.0,
                "fsnotifier": 12.4,
            },
        )
        assert _document(measured_build(runs=[run]))["runs"] == [
            {
                "index": 1,
                "wall_clock_seconds": 309.4,
                "exit_status": 0,
                "contains_failure": False,
                "tasks": {
                    "test:unit:cli": {
                        "outcome": "succeeded",
                        "exit_status": 0,
                        "start_seconds": 10.2,
                        "duration_seconds": 297.1,
                        "end_seconds": 10.2 + 297.1,
                        "units": {
                            "compiling": 44,
                            "checking": 0,
                            "documenting": 0,
                        },
                        "lock_wait_seconds": 0.0,
                        "nextest_phase_seconds": 149.2,
                        "queueing_delay_seconds": 0.4,
                        "after_failure": False,
                    }
                },
                "unit_totals": {
                    "compiling": 44,
                    "checking": 0,
                    "documenting": 0,
                },
                "cpu_buckets": [
                    {
                        "start_seconds": 0.0,
                        "busy_percent": 91.2,
                        "running": ["test:unit:cli"],
                    }
                ],
                "process_cpu_seconds": {
                    "syspolicyd": 51.3,
                    "XprotectService": None,
                    "ide": 301.0,
                    "fsnotifier": 12.4,
                },
                "fetch_failures": [],
            }
        ]

    def test_the_summary_block_carries_spreads_and_standings(self):
        runs = [
            run_figures(
                index,
                tasks={"test:unit:cli": task_figures(duration=duration)},
                wall_clock=wall_clock,
                process_cpu={"syspolicyd": cpu, "XprotectService": None},
            )
            for index, duration, wall_clock, cpu in (
                (1, 297.1, 309.4, 51.3),
                (2, 291.8, 305.1, 44.0),
                (3, 304.6, 322.0, 78.2),
            )
        ]
        assert _document(measured_build(runs=runs))["summary"] == {
            "wall_clock_seconds": {
                "minimum": 305.1,
                "median": 309.4,
                "maximum": 322.0,
            },
            "tasks": {
                "test:unit:cli": {
                    "presence": {"standing": "measured"},
                    "duration_seconds": {
                        "minimum": 291.8,
                        "median": 297.1,
                        "maximum": 304.6,
                    },
                    "nextest_phase_seconds": None,
                }
            },
            "processes": {
                "syspolicyd": {
                    "presence": {"standing": "measured"},
                    "cpu_seconds": {
                        "minimum": 44.0,
                        "median": 51.3,
                        "maximum": 78.2,
                    },
                },
                "XprotectService": {
                    "presence": {"standing": "absent", "absence": "not-seen"},
                    "cpu_seconds": None,
                },
            },
        }

    def test_the_record_carries_every_required_metadata_field(self):
        document = _document()
        assert document["commit"]["id"]
        assert document["commit"]["matched"] is True
        assert document["conditions"]["hostname"] == "engywook"
        assert document["conditions"]["model"] == "Mac16,5"
        assert document["conditions"]["os_version"] == "26.3"
        assert document["host"]["cores"]["total"] == 16
        assert document["mise"]["version"]
        assert document["conditions"]["mise_jobs"] == 8
        assert document["idle_load_percent"] == 26.8
        assert document["conditions"]["cache_state"] == "warm-no-op"
        assert document["conditions"]["host_settings"] == "devtools"
        assert document["task_set"] == ["build:cli"]


class TestParseRecord:
    @pytest.mark.parametrize(
        "measured",
        [
            measured_build(
                runs=[
                    run_figures(
                        1,
                        tasks={
                            "deny:check": task_figures(
                                TaskOutcome.FAILED, exit_status=1
                            ),
                            "build:cli": task_figures(after_failure=True),
                            "format:check": task_figures(TaskOutcome.SKIPPED),
                            "check": task_figures(TaskOutcome.SILENT),
                        },
                        exit_status=1,
                        process_cpu={"XprotectService": None},
                    )
                ],
                idle_load_percent=None,
            ),
            measured_build(
                host=REFERENCE_HOST.model_copy(
                    update={
                        "chip": NoReading(argv=("sysctl", "-n", "chip")),
                        "cores": CoreCount(
                            total=NoReading(argv=("sysctl", "-n", "hw.ncpu")),
                            performance=12,
                            efficiency=4,
                        ),
                    }
                )
            ),
            measured_build(
                runs=[
                    run_figures(
                        process_cpu={"syspolicyd": NoReading(argv=("ps",))}
                    )
                ]
            ),
        ],
        ids=["failed-skipped-absent", "host-no-reading", "ps-gap"],
    )
    def test_a_record_reads_back_equal_to_what_was_written(self, measured):
        assert parse_record(record_text(measured), source=_SOURCE) == measured

    def test_a_record_without_a_schema_version_is_refused(self):
        document = _document()
        del document["schema_version"]
        with pytest.raises(UnreadableRecordError, match="schema_version"):
            _parsed(document)

    def test_an_unknown_schema_version_is_refused_naming_the_file(self):
        document = _document()
        document["schema_version"] = 2
        with pytest.raises(UnreadableRecordError, match=str(_SOURCE)):
            _parsed(document)

    def test_a_malformed_field_is_refused_naming_its_location(self):
        document = _document()
        document["runs"][0]["wall_clock_seconds"] = "309.4"
        with pytest.raises(
            UnreadableRecordError, match=r"runs\.0\.wall_clock_seconds"
        ):
            _parsed(document)

    def test_a_warm_dispatch_record_is_refused(self):
        with pytest.raises(UnreadableRecordError):
            parse_record(
                json.dumps({"schema": 1, "arms": {}, "samples": []}),
                source=Path("warm-dispatch-1.json"),
            )

    def test_a_truncated_record_is_refused(self):
        with pytest.raises(UnreadableRecordError):
            parse_record(record_text(measured_build())[:200], source=_SOURCE)

    @pytest.mark.parametrize("field", sorted(CONDITIONS_MIRRORING_THE_HOST))
    def test_a_condition_disagreeing_with_the_host_is_refused(self, field):
        document = _document()
        document["conditions"][field] = (
            9 if field == "mise_jobs" else "elsewhere"
        )
        with pytest.raises(UnreadableRecordError, match="conditions"):
            _parsed(document)

    def test_a_top_level_fetch_failure_missing_from_the_runs_is_refused(self):
        document = _document()
        document["fetch_failures"] = [
            {
                "run": 1,
                "task": "deny:check",
                "external_input": "advisory-database",
                "line": "x",
            }
        ]
        with pytest.raises(UnreadableRecordError, match="fetch_failures"):
            _parsed(document)

    def test_a_runs_fetch_failure_missing_from_the_top_level_is_refused(self):
        failure = FetchFailure(
            run=1,
            task="deny:check",
            external_input=ExternalInput.ADVISORY_DATABASE,
            line="x",
        )
        document = _document(
            measured_build(runs=[run_figures(fetch_failures=[failure])])
        )
        document["fetch_failures"] = []
        with pytest.raises(UnreadableRecordError, match="fetch_failures"):
            _parsed(document)

    def test_a_listed_failed_task_that_did_not_fail_is_refused(self):
        document = _document()
        document["failed_tasks"] = ["build:cli"]
        with pytest.raises(UnreadableRecordError, match="failed_tasks"):
            _parsed(document)

    def test_a_failed_task_not_listed_is_refused(self):
        document = _document(
            measured_build(
                runs=[
                    run_figures(
                        tasks={
                            "deny:check": task_figures(
                                TaskOutcome.FAILED, exit_status=1
                            )
                        },
                        exit_status=1,
                    )
                ]
            )
        )
        document["failed_tasks"] = []
        with pytest.raises(UnreadableRecordError, match="failed_tasks"):
            _parsed(document)

    @pytest.mark.parametrize(
        ("path", "value"),
        [
            (("commit", "matched"), False),
            (("runs", 0, "contains_failure"), True),
            (("runs", 0, "unit_totals", "compiling"), 3),
        ],
        ids=["matched", "contains-failure", "unit-totals"],
    )
    def test_a_stored_derived_field_that_disagrees_is_refused(
        self, path, value
    ):
        document = _document()
        holder = document
        for key in path[:-1]:
            holder = holder[key]
        holder[path[-1]] = value
        with pytest.raises(UnreadableRecordError):
            _parsed(document)


class TestCommitMatch:
    @pytest.mark.parametrize(
        ("at_start", "at_end"), [(True, False), (False, True), (True, None)]
    )
    def test_matched_needs_both_checks_to_pass(self, at_start, at_end):
        assert not CommitMatch.of("c", at_start=at_start, at_end=at_end).matched

    def test_matched_when_both_checks_pass(self):
        assert CommitMatch.of("c", at_start=True, at_end=True).matched


class TestConditions:
    def test_identical_conditions_are_comparable(self):
        assert measured_build().conditions.comparable_with(
            measured_build().conditions
        )

    def test_conditions_holding_the_same_no_reading_are_not_comparable(self):
        unread = HostDescription.model_validate(
            {
                **dict(REFERENCE_HOST),
                "model": NoReading(argv=("sysctl", "-n", "hw.model")),
            }
        )
        first = measured_build(host=unread).conditions
        second = measured_build(host=unread).conditions
        assert first == second
        assert not first.comparable_with(second)

    def test_a_differing_label_is_named_with_both_values(self):
        none = measured_build(host_settings=HostSettings.NONE).conditions
        devtools = measured_build().conditions
        [difference] = none.differences(devtools)
        assert difference.field == "host_settings"
        assert (difference.previous, difference.current) == (
            HostSettings.NONE,
            HostSettings.DEVTOOLS,
        )


class TestWriteMeasuredBuild:
    def test_the_record_is_written_to_the_run_directory(self, tmp_path):
        measured = measured_build()
        written = write_measured_build(measured, root=tmp_path, keep=False)
        assert written.record == tmp_path / measured.run_directory / (
            "record.json"
        )
        assert written.record.read_text() == record_text(measured)
        assert [p.name for p in written.record.parent.iterdir()] == [
            "record.json"
        ]

    def test_without_keep_nothing_is_written_under_measurements(self, tmp_path):
        written = write_measured_build(
            measured_build(), root=tmp_path, keep=False
        )
        assert written.kept is None
        assert not (tmp_path / MEASUREMENTS_RELATIVE).exists()

    def test_keep_also_writes_the_record_under_measurements(self, tmp_path):
        measured = measured_build()
        written = write_measured_build(measured, root=tmp_path, keep=True)
        assert written.kept == (
            tmp_path / MEASUREMENTS_RELATIVE / f"{measured.run_id}.json"
        )
        assert written.kept.read_text() == record_text(measured)

    def test_keep_never_overwrites_an_existing_record(self, tmp_path):
        measured = measured_build()
        kept = tmp_path / MEASUREMENTS_RELATIVE / f"{measured.run_id}.json"
        kept.parent.mkdir(parents=True)
        kept.write_text("earlier")
        with pytest.raises(KeptRecordExistsError, match="left as it was"):
            write_measured_build(measured, root=tmp_path, keep=True)
        assert kept.read_text() == "earlier"

    def test_no_temporary_file_is_ever_left_under_measurements(self, tmp_path):
        measured = measured_build()
        write_measured_build(measured, root=tmp_path, keep=True)
        assert [
            p.name for p in (tmp_path / MEASUREMENTS_RELATIVE).iterdir()
        ] == [f"{measured.run_id}.json"]
        assert [
            p.name for p in (tmp_path / measured.run_directory).iterdir()
        ] == ["record.json"]

    def test_keep_withholds_a_record_with_fetch_failures(self, tmp_path):
        failure = FetchFailure(
            run=1,
            task="deny:check",
            external_input=ExternalInput.ADVISORY_DATABASE,
            line="x",
        )
        measured = measured_build(runs=[run_figures(fetch_failures=[failure])])
        written = write_measured_build(measured, root=tmp_path, keep=True)
        assert written.kept is None
        assert written.record.exists()
        assert not (tmp_path / MEASUREMENTS_RELATIVE).exists()


def test_the_run_directory_is_gitignored():
    assert RUN_DIRECTORY_ROOT.parts[0] == ".accelerator-measure"
    assert "/.accelerator-measure/" in (
        (REPO_ROOT / ".gitignore").read_text().splitlines()
    )


@pytest.mark.parametrize(
    "path", _committed_build_records(), ids=lambda path: path.name
)
def test_every_committed_build_record_is_readable(path):
    parse_record(path.read_text(), source=path)


@pytest.mark.skipif(_BASELINE is None, reason="no baseline committed yet")
def test_the_baseline_is_among_the_committed_records():
    assert _BASELINE in _committed_build_records()
