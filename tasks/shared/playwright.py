"""Single shared source of truth for the Docker Playwright pins.

Every value that must stay aligned between the local Docker task and the CI
visual job lives here exactly once: the image tag (derived from the resolved
``@playwright/test`` version in the committed lockfile), the Chromium
``channel``, the rendering ``locale``, the shell glibc locale, and the image
platform/flavour. The TS Docker config never re-declares any of these — the
invoke task passes them into the container as env vars read via ``process.env``
(mirroring how ``BASE_URL`` already flows), so nothing can drift.
"""

import json
import re
from pathlib import Path

from tasks.shared.paths import FRONTEND

PLAYWRIGHT_IMAGE_FLAVOUR = "noble"
PLAYWRIGHT_PLATFORM = "linux/amd64"
# The locale that shapes the committed pixels is Chromium's, set explicitly and
# OS-independently via Playwright's `locale` option — single-valued en-US across
# every run. The container/host glibc locale (LANG/LC_ALL) is pinned to C.UTF-8,
# which every Playwright image ships pre-generated, only for deterministic
# shell/npm behaviour; it does NOT affect the rendered baseline, so we never
# locale-gen inside the image. Both values live here so this module is the sole
# source and neither can drift.
BROWSER_LOCALE = "en-US"
E2E_LANG = "C.UTF-8"
# Chromium channel pinned so the headless build is deterministic regardless of
# Playwright's default-headless changes (e.g. the v1.49 shift). Consumed by the
# Docker config via the CHROMIUM_CHANNEL env var the invoke task sets — defined
# here so this module is the sole source.
CHROMIUM_CHANNEL = "chromium"

# A Docker image tag must be a valid version component: digits, dots, and
# (for pre-releases) the usual semver punctuation. Reject anything that would
# produce a silently-malformed tag reaching `docker run`.
_VERSION_RE = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?$")

_LOCKFILE_ENTRY = "node_modules/@playwright/test"


def resolved_playwright_version(frontend: Path = FRONTEND) -> str:
    """Read the exact @playwright/test version from the committed lockfile.

    Install-independent: reads the resolved version recorded in
    ``package-lock.json`` rather than the caret range in ``package.json``.
    Raises a clear, distinct ``ValueError`` for each malformed input rather than
    leaking an unhelpful ``KeyError``/``JSONDecodeError``.
    """
    lockfile = frontend / "package-lock.json"
    try:
        raw = lockfile.read_text()
    except OSError as exc:
        raise ValueError(
            f"Playwright lockfile not readable at {lockfile}: {exc}"
        ) from exc
    try:
        lock = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise ValueError(
            f"Playwright lockfile at {lockfile} is not valid JSON: {exc}"
        ) from exc
    packages = lock.get("packages")
    if not isinstance(packages, dict):
        # A malformed lockfile is a value problem, not a caller type error —
        # keep the exception type uniform (ValueError) across every input.
        raise ValueError(  # noqa: TRY004
            f"Playwright lockfile at {lockfile} has no 'packages' object; "
            "is it a lockfileVersion the tasks understand?"
        )
    entry = packages.get(_LOCKFILE_ENTRY)
    if not isinstance(entry, dict):
        raise ValueError(  # noqa: TRY004
            f"Playwright lockfile at {lockfile} has no '{_LOCKFILE_ENTRY}' "
            "entry; run `npm install` in the frontend to refresh it."
        )
    version = entry.get("version")
    if not isinstance(version, str) or not version:
        raise ValueError(
            f"'{_LOCKFILE_ENTRY}' in {lockfile} has no 'version' string."
        )
    if not _VERSION_RE.match(version):
        raise ValueError(
            f"Resolved @playwright/test version {version!r} is not a valid "
            "image-tag version component (would produce a malformed tag)."
        )
    return version


def playwright_image(frontend: Path = FRONTEND) -> str:
    version = resolved_playwright_version(frontend)
    return f"mcr.microsoft.com/playwright:v{version}-{PLAYWRIGHT_IMAGE_FLAVOUR}"
