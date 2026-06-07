import time

from tasks.shared.clock import Clock


class TestClock:
    def test_defaults_wire_to_the_real_time_module(self):
        clock = Clock()
        assert clock.sleep is time.sleep
        assert clock.now is time.monotonic
