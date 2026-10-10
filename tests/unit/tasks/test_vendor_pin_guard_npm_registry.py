import datetime as dt

import pytest

from tasks.shared.vendor.pin_guard.chromium_advisories import (
    chromium_advisories,
)
from tasks.shared.vendor.pin_guard.chromium_versions import ChromiumVersion
from tasks.shared.vendor.pin_guard.feeds import (
    CheckName,
    Feed,
    FeedUnreachableError,
)
from tasks.shared.vendor.pin_guard.local_inputs import Pin, PinName
from tasks.shared.vendor.pin_guard.npm_registry import (
    MAXIMUM_MANIFEST_BYTES,
    MAXIMUM_TARBALL_BYTES,
    PinInconsistencyError,
    PinnedBrowser,
    pinned_browser_build,
)
from tests.unit.tasks.shared.doubles import (
    browsers_document,
    clear_feeds,
    fake_session,
    kev_url,
    npm_tarball_url,
    npm_version_document,
    npm_version_url,
    playwright_tarball,
    publish_playwright_core,
)

BUMPED = dt.date(2026, 5, 18)
PLAYWRIGHT_CORE = Pin(PinName.PLAYWRIGHT_CORE, "1.55.1", BUMPED)
CHROMIUM = Pin(PinName.CHROMIUM, "1193", BUMPED)


def test_the_pinned_revision_yields_its_browser_version():
    browser = pinned_browser_build(
        PLAYWRIGHT_CORE, CHROMIUM, fake_session(clear_feeds())
    )
    assert browser == PinnedBrowser(
        "1193", ChromiumVersion.parse("140.0.7339.186")
    )


def test_a_manifest_without_the_pinned_revision_aborts_naming_it():
    feeds = publish_playwright_core(
        clear_feeds(), playwright_tarball(browsers_document(revision="1200"))
    )
    with pytest.raises(PinInconsistencyError, match=r"chromium\.revision 1193"):
        pinned_browser_build(PLAYWRIGHT_CORE, CHROMIUM, fake_session(feeds))


def _document(**dist):
    document = npm_version_document(playwright_tarball())
    document["dist"] |= dist
    return document


def _without_tarball():
    document = npm_version_document(playwright_tarball())
    del document["dist"]["tarball"]
    return document


def _with_version(version):
    return npm_version_document(playwright_tarball()) | {"version": version}


def _published(tarball):
    return publish_playwright_core(clear_feeds(), tarball)


def _answered(document):
    return clear_feeds().answer(npm_version_url(), document)


def _headless_shell(**fields):
    return {
        "browsers": [
            {"name": "chromium-headless-shell", "revision": "1193"} | fields
        ]
    }


FAILURES = {
    "unreachable version document": (
        lambda: _answered(FeedUnreachableError("HTTP 503")),
        npm_version_url(),
        "unreachable",
    ),
    "version document for another version": (
        lambda: _answered(_with_version("1.55.0")),
        npm_version_url(),
        "unexpected version 1.55.0",
    ),
    "no tarball": (
        lambda: _answered(_without_tarball()),
        npm_version_url(),
        "missing field tarball",
    ),
    "tarball on another host": (
        lambda: _answered(_document(tarball="https://evil.test/p.tgz")),
        npm_version_url(),
        "tarball on unexpected host evil.test",
    ),
    "tarball over plain http": (
        lambda: _answered(
            _document(tarball=npm_tarball_url().replace("https", "http"))
        ),
        npm_version_url(),
        "tarball on unexpected host registry.npmjs.org",
    ),
    "integrity mismatch": (
        lambda: _answered(_document(integrity="sha512-AAAA")),
        npm_tarball_url(),
        "integrity mismatch",
    ),
    "tarball over the cap": (
        lambda: _published(b"\0" * (MAXIMUM_TARBALL_BYTES + 1)),
        npm_tarball_url(),
        f"payload over {MAXIMUM_TARBALL_BYTES} bytes",
    ),
    "tarball not gzip": (
        lambda: _published(playwright_tarball(gzipped=False)),
        npm_tarball_url(),
        "unreadable tarball",
    ),
    "no browsers.json member": (
        lambda: _published(playwright_tarball(member="package/other.json")),
        npm_tarball_url(),
        "missing member package/browsers.json",
    ),
    "browsers.json over the cap": (
        lambda: _published(
            playwright_tarball(b" " * (MAXIMUM_MANIFEST_BYTES + 1))
        ),
        npm_tarball_url(),
        f"package/browsers.json over {MAXIMUM_MANIFEST_BYTES} bytes",
    ),
    "unparseable browsers.json": (
        lambda: _published(playwright_tarball("not json")),
        npm_tarball_url(),
        "unparseable browsers.json",
    ),
    "empty browsers array": (
        lambda: _published(playwright_tarball({"browsers": []})),
        npm_tarball_url(),
        "unexpected browsers.json shape",
    ),
    "headless shell without browserVersion": (
        lambda: _published(playwright_tarball(_headless_shell())),
        npm_tarball_url(),
        "missing field browserVersion",
    ),
    "browserVersion not four-part": (
        lambda: _published(
            playwright_tarball(_headless_shell(browserVersion="140.0.7339"))
        ),
        npm_tarball_url(),
        "unparseable browser version 140.0.7339",
    ),
}


@pytest.mark.parametrize(
    ("feeds", "request_label", "reason"),
    list(FAILURES.values()),
    ids=list(FAILURES),
)
def test_each_registry_failure_skips_the_rest_of_the_chromium_check(
    feeds, request_label, reason
):
    fake = feeds()
    outcome = chromium_advisories(PLAYWRIGHT_CORE, CHROMIUM, fake_session(fake))
    assert outcome.findings == ()
    assert [
        (f.feed, f.check, f.request, f.reason) for f in outcome.failures
    ] == [
        (
            Feed.NPM_REGISTRY,
            CheckName.CHROMIUM_ADVISORIES,
            request_label,
            reason,
        )
    ]
    assert ("GET", kev_url()) not in fake.calls
    assert not any("osv.dev" in url for _, url in fake.calls)
