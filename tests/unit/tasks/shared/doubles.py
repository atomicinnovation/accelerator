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


CLEAR_PLAYWRIGHT_VERSION = "1.55.1"
CLEAR_CHROMIUM_REVISION = "1193"
CLEAR_NODE_VERSION = "22.22.2"
OWNER_LINE = "Owner: Toby Clemson (@tobyclemson)"
BUMP_SUBJECTS = ("playwright-core", "chromium", "node", "keyring")


def pins_text(
    bumped,
    *,
    chromium_revision=CLEAR_CHROMIUM_REVISION,
    node_version=CLEAR_NODE_VERSION,
):
    def line(subject):
        return f"bumped = {bumped[subject]}\n" if subject in bumped else ""

    return (
        f"[playwright-core]\n{line('playwright-core')}\n"
        f'[chromium]\nrevision = "{chromium_revision}"\n{line("chromium")}\n'
        f'[node]\nversion = "{node_version}"\n{line("node")}\n'
        f"[keyring]\n{line('keyring')}"
    )


def clear_repository(
    root,
    today,
    *,
    bumped=None,
    pins=None,
    owner_line=OWNER_LINE,
    playwright=CLEAR_PLAYWRIGHT_VERSION,
):
    """Write a repository whose every local input is valid and fresh."""
    import json

    from tasks.shared.vendor.pin_guard.local_inputs import LocalInputPaths

    dates = dict.fromkeys(BUMP_SUBJECTS, today) | (bumped or {})
    paths = LocalInputPaths(
        pins=root / "pins.toml",
        package_json=root / "package.json",
        releasing=root / "RELEASING.md",
        node_keyring=root / "nodejs-release.asc",
    )
    paths.pins.write_text(pins if pins is not None else pins_text(dates))
    paths.package_json.write_text(
        json.dumps({"dependencies": {"playwright": playwright}})
    )
    paths.releasing.write_text(f"# Releasing\n\n{owner_line}\n")
    paths.node_keyring.write_text("-----BEGIN PGP PUBLIC KEY BLOCK-----\n")
    return paths


class FakeIssueTracker:
    """In-memory ``IssueTracker`` seeded with open and closed issues."""

    def __init__(self, issues=(), *, label_exists=True, failing=()):
        self.existing = list(issues)
        self.label_exists = label_exists
        self.failing = set(failing)
        self.opened = []
        self.labels_created = 0
        self.calls = []

    def issues(self):
        self.calls.append("issues")
        return list(self.existing)

    def ensure_label(self):
        self.calls.append("ensure_label")
        if not self.label_exists:
            self.label_exists = True
            self.labels_created += 1

    def open_issue(self, draft, assignee):
        from tasks.shared.vendor.pin_guard.issues import IssueTrackerError

        self.calls.append("open_issue")
        if not self.label_exists:
            raise IssueTrackerError("label runtime-pin-guard does not exist")
        if draft.marker in self.failing:
            raise IssueTrackerError(f"refused {draft.title}")
        self.opened.append((draft, assignee))
        return 1000 + len(self.opened)

    @property
    def opened_markers(self):
        return [draft.marker for draft, _ in self.opened]


class StubFinding:
    """A finding whose identity is one name, for reconciliation tests."""

    def __init__(self, name):
        self.name = name

    @property
    def marker(self):
        from tasks.shared.vendor.pin_guard.markers import (
            IssueMarker,
            MarkerKind,
        )

        return IssueMarker(MarkerKind.FINDING, ("stub", self.name))

    def draft(self, owner):
        from tasks.shared.vendor.pin_guard.findings import (
            IssueDraft,
            issue_body,
        )

        return IssueDraft(
            self.marker,
            f"Stub {self.name}",
            issue_body(self.marker, f"Stub finding {self.name}."),
        )


def fake_ports(**overrides):
    """``GuardPorts`` built from fakes only, describing a clear world."""
    from tasks.shared.vendor.pin_guard.guard import GuardPorts

    defaults = {"tracker": FakeIssueTracker()}
    return GuardPorts(**(defaults | overrides))
