"""Shared test doubles for the tasks helper suites."""


class FakeClock:
    """Deterministic clock: every ``sleep(dt)`` advances ``now`` by ``dt``."""

    def __init__(self, start: float = 0.0):
        self.t = start
        self.sleeps: list[float] = []

    def now(self) -> float:
        return self.t

    def sleep(self, dt: float) -> None:
        self.sleeps.append(dt)
        self.t += dt
