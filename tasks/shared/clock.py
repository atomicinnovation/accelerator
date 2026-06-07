"""An injectable wall/monotonic clock for sleep/now seams in polling loops."""

import dataclasses
import time
from collections.abc import Callable


@dataclasses.dataclass(frozen=True)
class Clock:
    sleep: Callable[[float], None] = time.sleep
    now: Callable[[], float] = time.monotonic
