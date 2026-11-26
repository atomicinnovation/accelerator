import pytest

from tasks.shared.build_measurement.duration_logs import timed_runs
from tasks.shared.build_measurement.errors import MeasurementRefusedError

_PASS = "        PASS [   1.000s] (   1/1) a x\n"


def _stamped_run(directory, number):
    (directory / f"run-{number}-test_unit_cli.log").write_text(
        f"{float(number)} [test:unit:cli] {_PASS}"
    )


class TestReadingARecord:
    def test_reads_its_runs_in_run_number_order(self, tmp_path):
        for number in (10, 2, 1):
            _stamped_run(tmp_path, number)

        runs = timed_runs(record=tmp_path, logs=())

        assert [
            next(iter(run.passes.values())).finished_at for run in runs
        ] == [1.0, 2.0, 10.0]

    def test_a_record_with_no_run_logs_is_refused_naming_it(self, tmp_path):
        with pytest.raises(MeasurementRefusedError, match=str(tmp_path)):
            timed_runs(record=tmp_path, logs=())


class TestReadingAloneRunLogs:
    def test_reads_every_log_a_glob_matches(self, tmp_path):
        for name in ("one.log", "two.log"):
            (tmp_path / name).write_text(_PASS)

        runs = timed_runs(record=None, logs=[str(tmp_path / "*.log")])

        assert len(runs) == 2

    def test_a_pattern_matching_nothing_is_refused_naming_it(self, tmp_path):
        pattern = str(tmp_path / "*.log")

        with pytest.raises(MeasurementRefusedError, match="no log matches"):
            timed_runs(record=None, logs=[pattern])


@pytest.mark.parametrize(("record", "logs"), [(None, ()), ("both", ("a.log",))])
def test_exactly_one_source_is_read(tmp_path, record, logs):
    with pytest.raises(
        MeasurementRefusedError, match="either --record or --log"
    ):
        timed_runs(record=None if record is None else tmp_path, logs=list(logs))
