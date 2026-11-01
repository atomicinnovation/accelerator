import io

from tasks.shared.build_measurement.log_files import StampedLog, open_log
from tasks.shared.build_measurement.timeline import StampedLine, read_stamped
from tests.unit.tasks.shared.doubles import FakeClock


def _followed(tmp_path, output: bytes) -> tuple[StampedLine, ...]:
    stamped = StampedLog(open_log(tmp_path / "followed.log"))
    stamped.follow(io.BytesIO(output), FakeClock(start=12.5), epoch=10.0)
    return stamped.close()


def test_each_line_is_stamped_against_the_epoch(tmp_path):
    assert _followed(tmp_path, b"one\ntwo\n") == (
        StampedLine(2.5, "one"),
        StampedLine(2.5, "two"),
    )


def test_a_lone_carriage_return_stays_inside_its_line(tmp_path):
    assert _followed(tmp_path, b"[cargo] 1/9\r[cargo] 9/9\n") == (
        StampedLine(2.5, "[cargo] 1/9\r[cargo] 9/9"),
    )


def test_an_invalid_utf8_byte_is_replaced(tmp_path):
    assert _followed(tmp_path, b"bad \xff byte\n") == (
        StampedLine(2.5, "bad � byte"),
    )


def test_the_log_holds_the_lines_it_returns(tmp_path):
    lines = _followed(tmp_path, b"one\r\ntwo\n")
    log = (tmp_path / "followed.log").read_text(newline="")
    assert read_stamped(log) == lines
