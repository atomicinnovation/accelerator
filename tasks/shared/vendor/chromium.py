"""Chromium is pinned, not verified.

The Chrome for Testing headless shell is fetched from Playwright's CDN mirror
over TLS with no publisher signature, so provenance rests on a committed
per-platform sha256 that makes the bytes reviewable: a digest derived from
whatever the CDN served this release attests our own output rather than the
input, and committing it converts a trust-on-first-use into one reviewed moment.
It bounds blast radius; it does not establish provenance.

The revision is not chosen independently either: both the fetch and assembly
refuse a pinned revision that the vendored ``playwright-core``'s
``browsers.json`` does not declare (see :mod:`tasks.shared.vendor.browsers`).
"""

import hashlib
from pathlib import Path

from tasks.shared.paths import PINS_TOML
from tasks.shared.targets import Platform
from tasks.shared.vendor import pins

_CHUNK = 64 * 1024


def assert_chromium_bytes(
    archive: Path,
    *,
    platform: Platform,
    pins_path: Path = PINS_TOML,
) -> None:
    """Fail unless ``archive``'s sha256 matches its reviewed platform pin."""
    actual = _sha256_file(archive)
    expected = pins.chromium_sha256(platform, pins_path)
    if actual != expected:
        raise ValueError(
            f"Chromium {platform}: fetched sha256 {actual} != pinned {expected}"
        )


def _sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as handle:
        while chunk := handle.read(_CHUNK):
            hasher.update(chunk)
    return hasher.hexdigest()
