import os
import signal
import threading
import time

import pytest

from tasks.shared.signals import (
    UNWIND_SIGNALS,
    deferred_signals,
    unwind_then_defer,
    unwinding_signals,
)


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


@pytest.fixture
def unwinding():
    with unwinding_signals():
        yield


@pytest.fixture
def live_helper_thread():
    stop = threading.Event()
    helper = threading.Thread(target=stop.wait, daemon=True)
    helper.start()
    yield helper
    stop.set()
    helper.join(timeout=5)


@pytest.mark.usefixtures("unwinding")
class TestDeferredSignals:
    def test_a_signal_during_the_block_is_raised_after_it(
        self, live_helper_thread
    ):
        assert live_helper_thread.is_alive()
        finished_block = False
        with pytest.raises(KeyboardInterrupt), deferred_signals(UNWIND_SIGNALS):
            os.kill(os.getpid(), signal.SIGTERM)
            time.sleep(0.05)
            finished_block = True
        assert finished_block

    def test_two_signals_during_the_block_raise_once(self):
        raised = 0
        try:
            with deferred_signals(UNWIND_SIGNALS):
                os.kill(os.getpid(), signal.SIGINT)
                os.kill(os.getpid(), signal.SIGTERM)
        except KeyboardInterrupt:
            raised += 1
        assert raised == 1

    def test_the_previous_handlers_are_restored(self):
        before = {number: signal.getsignal(number) for number in UNWIND_SIGNALS}
        with deferred_signals(UNWIND_SIGNALS):
            pass
        assert {
            number: signal.getsignal(number) for number in UNWIND_SIGNALS
        } == before

    def test_a_quiet_block_raises_nothing(self):
        with deferred_signals(UNWIND_SIGNALS):
            pass


@pytest.mark.usefixtures("unwinding")
class TestUnwindThenDefer:
    def test_after_the_first_unwind_later_signals_are_held(self):
        raised = 0
        finally_finished = False
        try:
            with unwind_then_defer(UNWIND_SIGNALS) as interruption:
                try:
                    os.kill(os.getpid(), signal.SIGINT)
                finally:
                    os.kill(os.getpid(), signal.SIGTERM)
                    time.sleep(0.05)
                    finally_finished = True
        except KeyboardInterrupt:
            raised += 1
        assert raised == 1
        assert finally_finished
        assert interruption.arrived

    def test_a_quiet_block_records_no_interruption(self):
        with unwind_then_defer(UNWIND_SIGNALS) as interruption:
            pass
        assert not interruption.arrived

    def test_the_previous_handlers_are_restored_after_an_interrupt(self):
        before = {number: signal.getsignal(number) for number in UNWIND_SIGNALS}
        with (
            pytest.raises(KeyboardInterrupt),
            unwind_then_defer(UNWIND_SIGNALS),
        ):
            os.kill(os.getpid(), signal.SIGINT)
        assert {
            number: signal.getsignal(number) for number in UNWIND_SIGNALS
        } == before
