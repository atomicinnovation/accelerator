"""Chromium is pinned, not verified.

Playwright's Chromium build is fetched from the CDN over TLS with no publisher
signature, so provenance rests on a committed per-platform sha256 that makes the
bytes reviewable: a digest derived from whatever the CDN served this release
attests our own output rather than the input, and committing it converts a
trust-on-first-use into one reviewed moment. It bounds blast radius; it does not
establish provenance.

The revision is not chosen independently either — it is read from the vendored
``playwright-core``'s ``browsers.json`` and cross-checked against the committed
pin, so an upstream Chromium bump cannot slip in under an unchanged Playwright
pin.
"""

import hashlib
from pathlib import Path

from tasks.shared.paths import PINS_TOML
from tasks.vendor import pins
from tasks.vendor.assemble import browser_revision

_CHUNK = 64 * 1024
_HEADLESS_SHELL = "chromium-headless-shell"


def assert_chromium_bytes(
    archive: Path,
    *,
    platform: str,
    pins_path: Path = PINS_TOML,
) -> None:
    """Fail unless ``archive``'s sha256 matches the reviewed per-platform pin.

    Used at fetch time, when only the revision (from the pins) is known; the
    revision cross-check against ``browsers.json`` happens at assembly, once the
    driver bundle is extracted.
    """
    actual = _sha256_file(archive)
    expected = pins.chromium_sha256(platform, pins_path)
    if actual != expected:
        raise ValueError(
            f"Chromium {platform}: fetched sha256 {actual} != pinned {expected}"
        )


def verify_chromium(
    archive: Path,
    *,
    platform: str,
    browsers_json: Path,
    pins_path: Path = PINS_TOML,
) -> None:
    """Fail the release unless the fetched Chromium matches both pins."""
    fetched_revision = browser_revision(browsers_json, _HEADLESS_SHELL)
    expected_revision = pins.chromium_revision(pins_path)
    if fetched_revision != expected_revision:
        raise ValueError(
            f"fetched Chromium revision {fetched_revision} != pinned "
            f"{expected_revision}"
        )
    assert_chromium_bytes(archive, platform=platform, pins_path=pins_path)


def _sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as handle:
        while chunk := handle.read(_CHUNK):
            hasher.update(chunk)
    return hasher.hexdigest()
