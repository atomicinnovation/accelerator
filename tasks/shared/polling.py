"""Filesystem polling with an injectable clock."""

import time
from pathlib import Path


def wait_for_file(
    path: Path | str,
    *,
    timeout: float = 30.0,
    interval: float = 0.1,
    sleep=time.sleep,
    now=time.monotonic,
) -> bool:
    """Poll for ``path`` to exist, returning ``True`` as soon as it does.

    Loop semantics are pinned so the boundary is testable with an injected
    clock: check at t=0, then while ``now() < deadline`` sleep
    ``min(interval, deadline - now())`` and re-check, with a final check after
    the loop. The last sleep is clamped so the poll never sleeps past the
    deadline.
    """
    path = Path(path)
    deadline = now() + timeout
    if path.exists():
        return True
    while now() < deadline:
        sleep(min(interval, deadline - now()))
        if path.exists():
            return True
    return path.exists()
