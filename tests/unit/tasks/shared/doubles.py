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


class FakeProc:
    def __init__(self, ct, parent=None, ignore_sigterm=False, unkillable=False):
        self.ct = ct
        self.alive = True
        self.parent = parent
        self.ignore_sigterm = ignore_sigterm
        self.unkillable = unkillable


class FakeProcs:
    """In-memory process tree implementing the ProcessOps protocol."""

    def __init__(self):
        self.procs: dict[int, FakeProc] = {}
        self.terminated: list[int] = []
        self.killed: list[int] = []

    def add(self, pid, ct=1000.0, parent=None, **kw):
        self.procs[pid] = FakeProc(ct, parent=parent, **kw)
        return pid

    def is_alive(self, pid):
        p = self.procs.get(pid)
        return bool(p and p.alive)

    def create_time(self, pid):
        p = self.procs.get(pid)
        return p.ct if p and p.alive else None

    def identity_matches(self, pid, start):
        p = self.procs.get(pid)
        return bool(p and p.alive and abs(p.ct - start) <= 0.5)

    def children(self, pid):
        out = []

        def rec(par):
            for cp, p in list(self.procs.items()):
                if p.parent == par and p.alive:
                    out.append((cp, p.ct))
                    rec(cp)

        rec(pid)
        return out

    def terminate(self, pid):
        self.terminated.append(pid)
        p = self.procs.get(pid)
        if p and p.alive and not p.ignore_sigterm:
            p.alive = False

    def kill(self, pid):
        self.killed.append(pid)
        p = self.procs.get(pid)
        if p and not p.unkillable:
            p.alive = False
