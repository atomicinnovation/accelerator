"""Shared test doubles for the tasks helper suites."""

from pathlib import Path

PIN_GUARD_FIXTURES = (
    Path(__file__).resolve().parents[1] / "fixtures" / "pin-guard"
)


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
    """In-memory ``IssueTracker`` seeded with open and closed issues.

    Every issue it opens is listed by later reads as open and authored by
    ``author``, so one tracker carries a guard's issues from run to run.
    """

    def __init__(
        self,
        issues=(),
        *,
        label_exists=True,
        failing=(),
        author="github-actions[bot]",
    ):
        self.existing = list(issues)
        self.author = author
        self.label_exists = label_exists
        self.failing = set(failing)
        self.opened = []
        self.labels_created = 0
        self.calls = []

    def issues(self):
        from tasks.shared.vendor.pin_guard.issues import ExistingIssue

        self.calls.append("issues")
        return [
            *self.existing,
            *(
                ExistingIssue(
                    1001 + index,
                    is_open=True,
                    author=self.author,
                    body=draft.body,
                )
                for index, (draft, _) in enumerate(self.opened)
            ),
        ]

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


def listing_lister(fixture_name):
    """A ``KeyLister`` answering any keyring with a recorded colon listing."""
    from tasks.shared.vendor.gpg import listed_keys

    listing = (PIN_GUARD_FIXTURES / fixture_name).read_text().splitlines()
    return lambda _keyring: listed_keys(listing)


def fake_ports(**overrides):
    """``GuardPorts`` built from fakes only, describing a clear world.

    Every feed answers as it would for ``clear_repository``'s pins, so a
    later default that drifts into a finding or a feed failure shows here.
    """
    from tasks.shared.clock import Clock
    from tasks.shared.vendor.pin_guard.guard import GuardPorts

    clock = FakeClock()
    defaults = {
        "tracker": FakeIssueTracker(),
        "key_lister": listing_lister("clear-keyring.colons"),
        "feeds": clear_feeds(),
        "clock": Clock(sleep=clock.sleep, now=clock.now),
    }
    return GuardPorts(**(defaults | overrides))


CLEAR_KEV_CVE = "CVE-2025-10585"


def clear_feeds(playwright=CLEAR_PLAYWRIGHT_VERSION):
    """Feeds answering every check with a populated, non-matching document.

    The one KEV Chromium CVE was fixed in ``140.0.7339.185``, just below the
    browser version the clear ``playwright-core`` declares.
    """
    feeds = FakeFeeds(
        {
            osv_batch(playwright): osv_fixture("querybatch-empty.json"),
            vuln_core_url(): {
                "1": vuln_core_entry(vulnerable="20.x", patched="^20.99.0")
            },
            kev_url(): kev_document(kev_entry(CLEAR_KEV_CVE)),
            osv_record(CLEAR_KEV_CVE): osv_fixture(f"{CLEAR_KEV_CVE}.json"),
        }
    )
    return publish_playwright_core(feeds, playwright_tarball(), playwright)


class Successive:
    """Responses a ``FakeFeeds`` request gives in turn, the last repeating."""

    def __init__(self, responses):
        self.responses = list(responses)

    def next(self):
        if len(self.responses) > 1:
            return self.responses.pop(0)
        return self.responses[0]


def successive(*responses):
    return Successive(responses)


class FakeFeeds:
    """A ``FeedClient`` answering recorded documents keyed by request.

    A GET is keyed by its URL and a POST by ``(url, json.dumps(body))``. A
    response is a document, a raw ``str``/``bytes`` body (decoded as JSON for
    the JSON calls, so ``"not json"`` is unparseable), an exception to raise,
    or ``successive(...)`` of those.
    """

    def __init__(self, responses=None):
        self.responses = dict(responses or {})
        self.calls = []

    def answer(self, key, response):
        self.responses[key] = response
        return self

    def get_json(self, url):
        self.calls.append(("GET", url))
        return self._json(self._respond(url))

    def post_json(self, url, body):
        import json

        self.calls.append(("POST", url))
        return self._json(self._respond((url, json.dumps(body))))

    def get_bytes(self, url, max_bytes):
        from tasks.shared.vendor.pin_guard.feeds import FeedDocumentError

        self.calls.append(("GET", url))
        body = self._respond(url)
        payload = body.encode() if isinstance(body, str) else body
        if len(payload) > max_bytes:
            raise FeedDocumentError(f"payload over {max_bytes} bytes")
        return payload

    def _respond(self, key):
        if key not in self.responses:
            raise AssertionError(f"FakeFeeds has no response for {key!r}")
        response = self.responses[key]
        if isinstance(response, Successive):
            response = response.next()
        if isinstance(response, BaseException):
            raise response
        return response

    @staticmethod
    def _json(response):
        import json

        from tasks.shared.vendor.pin_guard.feeds import FeedDocumentError

        if not isinstance(response, str | bytes):
            return response
        try:
            return json.loads(response)
        except ValueError as error:
            raise FeedDocumentError("unparseable") from error


def fake_session(feeds, clock=None):
    """A ``FeedSession`` over ``feeds`` with the run's deadline far off."""
    from tasks.shared.clock import Clock
    from tasks.shared.vendor.pin_guard.feeds import (
        GUARD_FEED_DEADLINE_SECONDS,
        FeedBudget,
        FeedSession,
    )

    clock = clock or FakeClock()
    return FeedSession(
        feeds,
        FeedBudget(
            clock.now() + GUARD_FEED_DEADLINE_SECONDS,
            Clock(sleep=clock.sleep, now=clock.now),
        ),
    )


def osv_batch(version):
    """The ``FakeFeeds`` key of the OSV batch query for ``version``."""
    import json

    from tasks.shared.vendor.pin_guard.osv import QUERYBATCH_URL, batch_query

    return (QUERYBATCH_URL, json.dumps(batch_query(version)))


def osv_record(advisory_id):
    from tasks.shared.vendor.pin_guard.osv import record_url

    return record_url(advisory_id)


def osv_fixture(name):
    return (PIN_GUARD_FIXTURES / "osv" / name).read_text()


def vuln_core_url():
    from tasks.shared.vendor.pin_guard.node_advisories import VULN_CORE_URL

    return VULN_CORE_URL


def vuln_core_entry(
    *,
    vulnerable="22.x",
    patched="^22.23.0",
    environments=("linux",),
    cves=("CVE-2026-0001",),
):
    """One ``vuln/core`` entry, by default affecting the clear Node pin."""
    return {
        "cve": list(cves),
        "vulnerable": vulnerable,
        "patched": patched,
        "affectedEnvironments": list(environments),
    }


def vuln_core_fixture(name):
    return (PIN_GUARD_FIXTURES / "vuln-core" / name).read_text()


def kev_url():
    from tasks.shared.vendor.pin_guard.chromium_advisories import KEV_URL

    return KEV_URL


def kev_entry(cve, product="Chromium V8", vendor="Google"):
    return {"cveID": cve, "vendorProject": vendor, "product": product}


def kev_document(*entries):
    return {"vulnerabilities": list(entries)}


def kev_fixture(name):
    return (PIN_GUARD_FIXTURES / "kev" / name).read_text()


def chromium_record(cve, *fixes):
    """An OSV CVE record whose unresolved ranges close at each of ``fixes``."""
    ranges = [
        {"events": [{"introduced": "0"}, {"fixed": fix}]} for fix in fixes
    ]
    return {
        "id": cve,
        "affected": [{"database_specific": {"unresolved_ranges": ranges}}],
    }


def browsers_document(browser_version="140.0.7339.186", revision="1193"):
    """A ``browsers.json`` declaring one headless-shell build."""
    return {
        "browsers": [
            {
                "name": "chromium-headless-shell",
                "revision": revision,
                "browserVersion": browser_version,
            }
        ]
    }


def playwright_tarball(
    browsers=None, *, member="package/browsers.json", gzipped=True
):
    """A ``playwright-core`` tarball holding ``browsers`` at ``member``.

    ``browsers`` defaults to the recorded 1.55.1 ``browsers.json``; a mapping
    is serialised and ``str`` or ``bytes`` is stored as given.
    """
    import gzip
    import io
    import json
    import tarfile

    if browsers is None:
        browsers = (PIN_GUARD_FIXTURES / "browsers-1.55.1.json").read_bytes()
    if isinstance(browsers, dict):
        browsers = json.dumps(browsers)
    content = browsers.encode() if isinstance(browsers, str) else browsers
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w") as archive:
        info = tarfile.TarInfo(member)
        info.size = len(content)
        archive.addfile(info, io.BytesIO(content))
    archive_bytes = buffer.getvalue()
    return gzip.compress(archive_bytes, mtime=0) if gzipped else archive_bytes


def integrity_of(payload):
    import base64
    import hashlib

    digest = hashlib.sha512(payload).digest()
    return "sha512-" + base64.b64encode(digest).decode()


def npm_version_url(version=CLEAR_PLAYWRIGHT_VERSION):
    from tasks.shared.vendor.pin_guard.npm_registry import version_url

    return version_url(version)


def npm_tarball_url(version=CLEAR_PLAYWRIGHT_VERSION):
    return (
        "https://registry.npmjs.org/playwright-core/-/"
        f"playwright-core-{version}.tgz"
    )


def npm_version_document(tarball, version=CLEAR_PLAYWRIGHT_VERSION):
    """The recorded 1.55.1 version document, re-pointed at ``tarball``."""
    import json

    document = json.loads(
        (PIN_GUARD_FIXTURES / "npm" / "playwright-core-1.55.1.json").read_text()
    )
    document["version"] = version
    document["dist"]["tarball"] = npm_tarball_url(version)
    document["dist"]["integrity"] = integrity_of(tarball)
    return document


def publish_playwright_core(feeds, tarball, version=CLEAR_PLAYWRIGHT_VERSION):
    """Answer the npm registry with ``tarball`` as ``version``'s release."""
    return feeds.answer(
        npm_version_url(version), npm_version_document(tarball, version)
    ).answer(npm_tarball_url(version), tarball)
