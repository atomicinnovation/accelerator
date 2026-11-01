"""Turning termination signals into exceptions so ``finally`` blocks run."""

import contextlib
import signal
from collections.abc import Callable, Iterable, Iterator
from dataclasses import dataclass
from types import FrameType

# SIGHUP included: a dropped ssh across a long unattended run would otherwise
# terminate by default and leave every artefact behind.
UNWIND_SIGNALS = (signal.SIGINT, signal.SIGTERM, signal.SIGHUP)

type _Handler = (
    Callable[[int, FrameType | None], object] | int | signal.Handlers | None
)


def _unwind(number: int, frame: FrameType | None) -> None:
    del frame
    raise KeyboardInterrupt(f"signal {number}")


@contextlib.contextmanager
def _handling(
    numbers: Iterable[signal.Signals], handler: _Handler
) -> Iterator[None]:
    previous = {number: signal.getsignal(number) for number in numbers}
    for number in previous:
        signal.signal(number, handler)
    try:
        yield
    finally:
        for number, handler_before in previous.items():
            with contextlib.suppress(ValueError, TypeError):
                signal.signal(number, handler_before)


@contextlib.contextmanager
def unwinding_signals() -> Iterator[None]:
    with _handling(UNWIND_SIGNALS, _unwind):
        yield


def _hold(number: int, frame: FrameType | None) -> None:
    del number, frame


@contextlib.contextmanager
def deferred_signals(numbers: Iterable[signal.Signals]) -> Iterator[None]:
    """Hold ``numbers`` for the block, then raise once if any arrived.

    A handler swap rather than ``pthread_sigmask``: the mask covers only the
    calling thread, so with sampler threads alive the kernel would deliver the
    signal to one of them and CPython would still run the handler here.
    """
    arrived: list[int] = []

    def record(number: int, frame: FrameType | None) -> None:
        del frame
        arrived.append(number)

    try:
        with _handling(numbers, record):
            yield
    finally:
        if arrived:
            raise KeyboardInterrupt(f"signal {arrived[0]}")


@dataclass(slots=True)
class Interruption:
    arrived: bool = False


@contextlib.contextmanager
def unwind_then_defer(
    numbers: Iterable[signal.Signals],
) -> Iterator[Interruption]:
    """Raise on the first of ``numbers``, then hold the rest until exit.

    Holding from the first signal on leaves no window in which a second one
    could abort the cleanup the first one started.
    """
    held = tuple(numbers)
    interruption = Interruption()

    def unwind_once(number: int, frame: FrameType | None) -> None:
        del frame
        interruption.arrived = True
        for each in held:
            signal.signal(each, _hold)
        raise KeyboardInterrupt(f"signal {number}")

    with _handling(held, unwind_once):
        yield interruption
