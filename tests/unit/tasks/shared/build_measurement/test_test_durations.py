import pytest

from tasks.shared.build_measurement.errors import MeasurementRefusedError
from tasks.shared.build_measurement.test_durations import (
    Charge,
    DurationFigures,
    PassedTest,
    RunFigure,
    TestId,
    TimedRun,
)
from tasks.shared.build_measurement.timeline import StampedLine, read_stamped
from tasks.shared.shared_fixtures.families import Family
from tasks.shared.sources import repo_root

_GOLDEN_RUNS = (
    repo_root() / "tests/unit/tasks/fixtures/build-measurement/test-durations"
)

_GUARDED_RESUME = "accelerator-migrate::guarded_resume"
_MATRIX_PRIVATE_BUILD = "vcs-test-support::matrix_private_build"
_REKEY_RUN = "work-adapters::rekey_run"


def _record_run(*texts: str, start: float = 1.0) -> TimedRun:
    return TimedRun.parse(
        StampedLine(at=start + index, text=f"[test:unit:cli] {text}")
        for index, text in enumerate(texts)
    )


def _pass(seconds: float, position: int, binary: str, test: str) -> str:
    return (
        f"        PASS [{seconds:>8.3f}s] ({position:>4}/8113) {binary} {test}"
    )


def _golden_runs() -> list[TimedRun]:
    return [
        TimedRun.parse(
            read_stamped(
                (_GOLDEN_RUNS / f"run-{run}-test_unit_cli.log").read_text()
            )
        )
        for run in (1, 2, 3)
    ]


class TestPassLines:
    def test_a_record_line_parses_with_nextests_padding(self):
        [line] = read_stamped(
            "9.73 [test:unit:cli]         PASS [   0.014s] (   1/8113) a b\n"
        )
        run = TimedRun.parse([line])
        assert run.passes == {
            TestId("a", "b"): PassedTest(
                id=TestId("a", "b"),
                seconds=0.014,
                position=1,
                finished_at=9.73,
            )
        }

    def test_an_alone_line_parses_without_a_finishing_time(self):
        run = TimedRun.parse_unstamped(
            "        PASS [   2.474s] (  12/  43) a b::c\n"
        )
        assert run.passes[TestId("a", "b::c")].finished_at is None
        assert run.passes[TestId("a", "b::c")].seconds == 2.474

    def test_an_alone_line_may_carry_its_tasks_prefix(self):
        run = TimedRun.parse_unstamped(
            "[test:unit:cli]         PASS [   2.474s] (  12/  43) a b\n"
        )
        assert run.passes[TestId("a", "b")].position == 12

    @pytest.mark.parametrize(
        "status", ["PASS", "TRY 2 PASS", "LEAK", "\x1b[32mPASS\x1b[0m"]
    )
    def test_pass_like_statuses_count_as_passes(self, status):
        run = _record_run(f"  {status} [   1.500s] (   3/   9) a b")
        assert run.passes[TestId("a", "b")].seconds == 1.5

    @pytest.mark.parametrize(
        "line",
        [
            "        FAIL [   1.500s] (   3/   9) a b",
            "  TRY 1 FAIL [   1.500s] (   3/   9) a b",
            "        SLOW [> 60.000s] (   3/   9) a b",
            "     SIGTERM [   1.500s] (   3/   9) a b",
            "   LEAK-FAIL [   1.500s] (   3/   9) a b",
        ],
    )
    def test_other_statuses_are_not_passes(self, line):
        assert _record_run(line).passes == {}

    def test_a_retried_test_keeps_its_final_attempts_duration(self):
        run = _record_run(
            "  TRY 1 FAIL [   4.000s] (   3/   9) a b",
            "  TRY 2 PASS [   1.250s] (   3/   9) a b",
        )
        assert run.passes[TestId("a", "b")].seconds == 1.25

    def test_a_test_id_reads_the_way_nextest_prints_it(self):
        assert TestId.parse("a::b c::d e") == TestId("a::b", "c::d e")

    def test_a_test_id_without_a_test_is_refused(self):
        with pytest.raises(MeasurementRefusedError, match="a::b"):
            TestId.parse("a::b")


class TestPhaseLine:
    def test_the_summary_gives_the_phase_duration_and_end(self):
        run = _record_run(
            "     Summary [ 126.068s] 8113 tests run: 8113 passed", start=140.5
        )
        assert run.phase_seconds == 126.068
        assert run.phase_end == 140.5

    def test_an_alone_summary_has_no_phase_end(self):
        run = TimedRun.parse_unstamped("     Summary [   7.512s] 1 test run\n")
        assert run.phase_seconds == 7.512
        assert run.phase_end is None


class TestFamilyBuilds:
    @pytest.mark.parametrize(
        "parse",
        [
            _record_run,
            lambda line: TimedRun.parse_unstamped(f"{line}\n"),
        ],
    )
    def test_a_family_build_line_gives_that_familys_build(self, parse):
        run = parse("shared VCS fixtures: status-log set built in 2.4 s")
        assert run.family_builds == {Family.STATUS_LOG: 2.4}

    def test_the_combined_build_line_names_no_family(self):
        run = _record_run(
            "shared VCS fixtures: built in 4.6 s, digested in 0.2 s"
        )
        assert run.family_builds == {}

    def test_family_build_takes_the_median_across_runs(self):
        figures = DurationFigures(
            runs=[
                _record_run(f"shared VCS fixtures: matrix built in {seconds} s")
                for seconds in (3.0, 1.0, 2.0)
            ]
        )
        assert figures.family_build(Family.MATRIX).median == 2.0


class TestLastFinishers:
    def test_the_last_finishers_have_the_highest_positions(self):
        run = _record_run(
            _pass(1.0, 4, "a", "fourth"),
            _pass(1.0, 1, "a", "first"),
            _pass(1.0, 3, "a", "third"),
            _pass(1.0, 2, "a", "second"),
        )
        assert [each.id.test for each in run.last_finishers(3)] == [
            "fourth",
            "third",
            "second",
        ]


class TestCombiningRuns:
    def runs(self, *per_run: dict[str, float]) -> list[TimedRun]:
        return [
            _record_run(
                *(
                    _pass(seconds, position, "a", test)
                    for position, (test, seconds) in enumerate(
                        tests.items(), start=1
                    )
                )
            )
            for tests in per_run
        ]

    def test_binary_seconds_sums_each_run_then_takes_the_median(self):
        figures = DurationFigures(
            runs=self.runs(
                {"x": 10.0, "y": 1.0},
                {"x": 1.0, "y": 10.0},
                {"x": 2.0, "y": 2.0},
            )
        )
        assert figures.binary_seconds("a") == RunFigure(
            per_run=(11.0, 11.0, 4.0), median=11.0, missing_from=()
        )

    def test_a_charge_adds_the_checkers_duration_to_the_targets(self):
        figures = DurationFigures(
            runs=self.runs(
                {"target": 10.0, "checker": 1.0},
                {"target": 1.0, "checker": 10.0},
                {"target": 2.0, "checker": 2.0},
            ),
            charges=[
                Charge(
                    checker=TestId("a", "checker"), target=TestId("a", "target")
                )
            ],
        )
        assert figures.per_test("a")[TestId("a", "target")].median == 11.0

    def test_a_charge_leaves_the_binarys_seconds_alone(self):
        figures = DurationFigures(
            runs=self.runs({"target": 10.0, "checker": 1.0}),
            charges=[Charge.parse("a checker=a target")],
        )
        assert figures.binary_seconds("a").median == 11.0

    def test_a_charge_naming_a_test_with_no_pass_line_is_refused(self):
        with pytest.raises(MeasurementRefusedError, match="a absent"):
            DurationFigures(
                runs=self.runs({"target": 1.0}),
                charges=[Charge.parse("a absent=a target")],
            )

    def test_a_charge_without_an_equals_sign_is_refused(self):
        with pytest.raises(MeasurementRefusedError, match="a checker"):
            Charge.parse("a checker")

    def test_a_served_familys_build_is_added_within_each_run(self):
        runs = [
            _record_run(
                f"shared VCS fixtures: status-log set built in {build} s",
                _pass(tests, 1, "a", "x"),
            )
            for build, tests in ((10.0, 1.0), (1.0, 10.0), (2.0, 2.0))
        ]
        figures = DurationFigures(runs=runs, served_by={"a": Family.STATUS_LOG})
        assert figures.binary_seconds("a").median == 11.0

    def test_a_test_missing_from_a_run_takes_the_median_of_the_others(self):
        figures = DurationFigures(
            runs=self.runs({"x": 1.0}, {"y": 5.0}, {"x": 3.0})
        )
        assert figures.per_test("a")[TestId("a", "x")] == RunFigure(
            per_run=(1.0, None, 3.0), median=2.0, missing_from=(2,)
        )

    def test_the_slowest_test_has_the_highest_charged_median(self):
        figures = DurationFigures(
            runs=self.runs({"x": 3.0, "y": 2.0, "z": 2.0}),
            charges=[Charge.parse("a z=a y")],
        )
        slowest, figure = figures.slowest_test("a")
        assert slowest == TestId("a", "y")
        assert figure.median == 4.0


class TestTheBaselineRecord:
    def test_it_reproduces_the_baseline_medians(self):
        figures = DurationFigures(runs=_golden_runs())
        guarded_resume = figures.per_test(_GUARDED_RESUME)
        rekey_run = figures.per_test(_REKEY_RUN)
        kill = TestId(
            _REKEY_RUN,
            "in_a_real_repository::"
            "a_run_killed_at_every_write_resumes_over_its_own_writes",
        )
        edit_refusal = TestId(
            _REKEY_RUN,
            "in_a_real_repository::"
            "a_resume_refuses_an_edit_since_the_halt_to_a_path_not_yet_written",
        )
        [matrix] = figures.per_test(_MATRIX_PRIVATE_BUILD).values()
        assert round(figures.binary_seconds(_GUARDED_RESUME).median, 1) == 197.5
        assert round(figures.slowest_test(_GUARDED_RESUME)[1].median, 1) == 28.2
        assert round(matrix.median, 1) == 33.2
        assert round(rekey_run[kill].median, 1) == 23.3
        assert round(rekey_run[edit_refusal].median, 1) == 17.9
        assert len(guarded_resume) == 13

    def test_every_run_has_each_binarys_pass_lines(self):
        for run in _golden_runs():
            binaries = [each.binary for each in run.passes]
            assert binaries.count(_GUARDED_RESUME) == 13
            assert binaries.count(_REKEY_RUN) == 29
            assert binaries.count(_MATRIX_PRIVATE_BUILD) == 1

    def test_the_tail_tests_finish_last(self):
        for run in _golden_runs():
            assert [each.position for each in run.last_finishers(3)] == [
                8269,
                8268,
                8267,
            ]
