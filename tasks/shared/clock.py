"""An injectable wall/monotonic clock for sleep/now seams in polling loops."""

import dataclasses
import datetime as dt
import time
from collections.abc import Callable


@dataclasses.dataclass(frozen=True)
class Clock:
    sleep: Callable[[float], None] = time.sleep
    now: Callable[[], float] = time.monotonic


def today_or_now(text: str | None) -> dt.date:
    """Return the ISO date ``text`` names, or today's UTC date without one."""
    if text is None:
        return dt.datetime.now(tz=dt.UTC).date()
    return dt.date.fromisoformat(text)
