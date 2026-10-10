"""The Chromium build the pinned ``playwright-core`` release declares.

The guard ships nothing, so the tarball is bound to the registry's ``integrity``
only; the release lane alone verifies the registry signature and provenance.
"""

import io
import json
import tarfile
from dataclasses import dataclass
from typing import Self
from urllib.parse import urlsplit

from tasks.shared.vendor.browsers import (
    HEADLESS_SHELL,
    BrowserBuild,
    BrowsersManifest,
    BrowsersManifestError,
)
from tasks.shared.vendor.npm import assert_integrity_binds_bytes
from tasks.shared.vendor.pin_guard.aborts import GuardAbortError
from tasks.shared.vendor.pin_guard.chromium_versions import (
    ChromiumVersion,
    ChromiumVersionError,
)
from tasks.shared.vendor.pin_guard.feeds import (
    CheckName,
    Feed,
    FeedDocumentError,
    FeedSession,
    MissingFieldError,
    field,
)
from tasks.shared.vendor.pin_guard.local_inputs import Pin

NPM_REGISTRY_HOST = "registry.npmjs.org"
MAXIMUM_TARBALL_BYTES = 32 * 1024 * 1024
MAXIMUM_MANIFEST_BYTES = 1024 * 1024
BROWSERS_MEMBER = "package/browsers.json"
_CHECK = CheckName.CHROMIUM_ADVISORIES


class PinInconsistencyError(GuardAbortError):
    """The pinned ``playwright-core`` has no build at the pinned revision."""


@dataclass(frozen=True, slots=True)
class PinnedBrowser:
    revision: str
    browser_version: ChromiumVersion


type HeadlessShellVersions = dict[str, ChromiumVersion]


def version_url(version: str) -> str:
    return f"https://{NPM_REGISTRY_HOST}/playwright-core/{version}"


@dataclass(frozen=True, slots=True)
class PublishedTarball:
    url: str
    integrity: str

    @classmethod
    def of_version_document(cls, document: object, version: str) -> Self:
        published = field(document, "version", str)
        if published != version:
            raise FeedDocumentError(f"unexpected version {published}")
        dist = field(document, "dist", dict)
        url = field(dist, "tarball", str)
        location = urlsplit(url)
        if location.scheme != "https" or location.hostname != NPM_REGISTRY_HOST:
            raise FeedDocumentError(
                f"tarball on unexpected host {location.hostname}"
            )
        return cls(url, field(dist, "integrity", str))

    def headless_shell_versions(self, payload: object) -> HeadlessShellVersions:
        """Every headless-shell build's version, from the bound tarball."""
        if not isinstance(payload, bytes):
            raise FeedDocumentError("unreadable tarball")
        try:
            assert_integrity_binds_bytes(
                integrity=self.integrity, payload=payload
            )
        except ValueError as error:
            raise FeedDocumentError("integrity mismatch") from error
        manifest = _browsers_manifest(_browsers_member(payload))
        return {
            build.revision: _browser_version(build)
            for build in manifest.builds
            if build.name == HEADLESS_SHELL
        }


def pinned_browser_build(
    playwright_core: Pin, chromium: Pin, session: FeedSession
) -> PinnedBrowser:
    """Return the browser build ``playwright_core`` pins at ``chromium``.

    Raises ``FeedRequestError`` when the registry fails, and
    ``PinInconsistencyError`` when the release declares no such build.
    """
    url = version_url(playwright_core.version)
    tarball = session.request(
        Feed.NPM_REGISTRY,
        _CHECK,
        url,
        lambda client: client.get_json(url),
        lambda document: PublishedTarball.of_version_document(
            document, playwright_core.version
        ),
    )
    versions = session.request(
        Feed.NPM_REGISTRY,
        _CHECK,
        tarball.url,
        lambda client: client.get_bytes(tarball.url, MAXIMUM_TARBALL_BYTES),
        tarball.headless_shell_versions,
    )
    if chromium.version not in versions:
        raise PinInconsistencyError(
            f"chromium.revision {chromium.version}: playwright-core "
            f"{playwright_core.version} declares no {HEADLESS_SHELL} build at "
            "that revision"
        )
    return PinnedBrowser(chromium.version, versions[chromium.version])


def _browsers_member(payload: bytes) -> bytes:
    try:
        with tarfile.open(fileobj=io.BytesIO(payload), mode="r:gz") as archive:
            member = archive.getmember(BROWSERS_MEMBER)
            if member.size > MAXIMUM_MANIFEST_BYTES:
                raise FeedDocumentError(
                    f"{BROWSERS_MEMBER} over {MAXIMUM_MANIFEST_BYTES} bytes"
                )
            handle = archive.extractfile(member)
            if handle is None:
                raise FeedDocumentError(f"missing member {BROWSERS_MEMBER}")
            return handle.read()
    except KeyError as error:
        raise FeedDocumentError(f"missing member {BROWSERS_MEMBER}") from error
    except (tarfile.TarError, OSError, EOFError) as error:
        raise FeedDocumentError("unreadable tarball") from error


def _browsers_manifest(member: bytes) -> BrowsersManifest:
    try:
        document = json.loads(member)
    except ValueError as error:
        raise FeedDocumentError("unparseable browsers.json") from error
    try:
        return BrowsersManifest.parse(document)
    except BrowsersManifestError as error:
        raise FeedDocumentError("unexpected browsers.json shape") from error


def _browser_version(build: BrowserBuild) -> ChromiumVersion:
    if build.browser_version is None:
        raise MissingFieldError("browserVersion")
    try:
        return ChromiumVersion.parse(build.browser_version)
    except ChromiumVersionError as error:
        raise FeedDocumentError(
            f"unparseable browser version {build.browser_version}"
        ) from error
