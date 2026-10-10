import datetime as dt
import time

from tasks.shared.clock import Clock, today_or_now


class TestClock:
    def test_defaults_wire_to_the_real_time_module(self):
        clock = Clock()
        assert clock.sleep is time.sleep
        assert clock.now is time.monotonic


class TestTodayOrNow:
    def test_an_iso_date_is_parsed(self):
        assert today_or_now("2026-10-09") == dt.date(2026, 10, 9)

    def test_no_date_is_the_current_utc_date(self):
        assert today_or_now(None) == dt.datetime.now(tz=dt.UTC).date()
