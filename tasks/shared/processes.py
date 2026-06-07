"""Process identity and control backed by psutil + ``os.kill``."""

import psutil

# A start time recorded by one psutil ``create_time()`` call and compared
# against another from the same source is expected to match exactly; this
# tolerance only absorbs float round-tripping (e.g. through JSON).
START_TIME_TOLERANCE = 0.5


def pid_identity_matches(pid: int, expected_start: float, *, tolerance: float) -> bool:
    """Return whether the live process at ``pid`` has the expected start time.

    Guards against a recycled PID belonging to an unrelated process. Returns
    ``False`` if no process holds ``pid``. ``create_time()`` is captured and
    compared from the same psutil source on both platforms, so ``tolerance`` is
    an explicit argument (no baked-in ±1 s) — callers pass the value validated
    for the platform, defaulting to a sub-second match.
    """
    try:
        actual = psutil.Process(pid).create_time()
    except psutil.NoSuchProcess:
        return False
    return abs(actual - expected_start) <= tolerance
