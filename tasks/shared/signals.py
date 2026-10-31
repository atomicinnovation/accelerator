"""Turning termination signals into exceptions so ``finally`` blocks run."""

import contextlib
import signal
from collections.abc import Callable, Iterable, Iterator
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
