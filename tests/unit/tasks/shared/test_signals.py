import os
import signal

import pytest

from tasks.shared.signals import UNWIND_SIGNALS, unwinding_signals


class TestUnwindingSignals:
    def test_all_three_signals_are_handled(self):
        assert set(UNWIND_SIGNALS) == {
            signal.SIGINT,
            signal.SIGTERM,
            signal.SIGHUP,
        }

    @pytest.mark.parametrize(
        "number",
        [signal.SIGINT, signal.SIGTERM, signal.SIGHUP],
        ids=["sigint", "sigterm", "sighup"],
    )
    def test_each_unwinding_signal_raises_keyboard_interrupt(self, number):
        reached_finally = False
        with pytest.raises(KeyboardInterrupt), unwinding_signals():
            try:
                os.kill(os.getpid(), number)
            finally:
                reached_finally = True
        assert reached_finally

    def test_the_previous_handlers_are_restored(self):
        before = {number: signal.getsignal(number) for number in UNWIND_SIGNALS}
        with unwinding_signals():
            pass
        assert {
            number: signal.getsignal(number) for number in UNWIND_SIGNALS
        } == before
