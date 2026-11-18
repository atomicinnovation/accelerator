"""The browser builds a ``playwright-core`` release pins in ``browsers.json``.

The vendored Chromium revision is not chosen independently: it is the
``chromium-headless-shell`` build the pinned ``playwright-core`` declares, so an
upstream Chromium bump cannot slip in under an unchanged Playwright pin.
"""

import json
import tarfile
from dataclasses import dataclass
from pathlib import Path
from typing import Self

HEADLESS_SHELL = "chromium-headless-shell"
PACKED_MANIFEST = "package/browsers.json"


class BrowsersManifestError(ValueError):
    """``browsers.json`` does not have the shape ``playwright-core`` ships."""


class UnpinnedRevisionError(Exception):
    """The manifest declares no headless-shell build at the pinned revision."""


@dataclass(frozen=True, slots=True)
class BrowserBuild:
    name: str
    revision: str
    browser_version: str | None

    @classmethod
    def parse(cls, entry: object) -> Self:
        if not isinstance(entry, dict):
            raise BrowsersManifestError(
                f"browser entry is not an object: {entry!r}"
            )
        name = entry.get("name")
        revision = entry.get("revision")
        if not isinstance(name, str) or not isinstance(revision, str):
            raise BrowsersManifestError(
                f"browser entry lacks a string name and revision: {entry!r}"
            )
        browser_version = entry.get("browserVersion")
        return cls(
            name,
            revision,
            browser_version if isinstance(browser_version, str) else None,
        )


@dataclass(frozen=True, slots=True)
class BrowsersManifest:
    builds: tuple[BrowserBuild, ...]

    @classmethod
    def parse(cls, document: object) -> Self:
        entries = (
            document.get("browsers") if isinstance(document, dict) else None
        )
        if not isinstance(entries, list) or not entries:
            raise BrowsersManifestError(
                "browsers.json must carry a non-empty browsers list"
            )
        return cls(tuple(BrowserBuild.parse(entry) for entry in entries))

    @classmethod
    def read(cls, path: Path) -> Self:
        try:
            document = json.loads(path.read_text())
        except json.JSONDecodeError as error:
            raise BrowsersManifestError(f"{path}: {error}") from error
        return cls.parse(document)

    @classmethod
    def read_packed(cls, tarball: Path) -> Self:
        """Read the manifest packed inside a ``playwright-core`` npm tarball."""
        with tarfile.open(tarball, "r:gz") as archive:
            try:
                member = archive.extractfile(PACKED_MANIFEST)
            except KeyError as error:
                raise BrowsersManifestError(
                    f"{tarball} packs no {PACKED_MANIFEST}"
                ) from error
            if member is None:
                raise BrowsersManifestError(
                    f"{PACKED_MANIFEST} in {tarball} is not a regular file"
                )
            try:
                document = json.loads(member.read())
            except json.JSONDecodeError as error:
                raise BrowsersManifestError(f"{tarball}: {error}") from error
        return cls.parse(document)

    def pinned_headless_shell(self, revision: str) -> BrowserBuild:
        """Return the headless-shell build at the pinned ``revision``.

        Raises ``UnpinnedRevisionError`` when there is none; a neighbouring
        build sharing the revision does not count.
        """
        for build in self.builds:
            if build.name == HEADLESS_SHELL and build.revision == revision:
                return build
        raise UnpinnedRevisionError(
            f"playwright-core declares no {HEADLESS_SHELL} build at the pinned "
            f"Chromium revision {revision}"
        )
