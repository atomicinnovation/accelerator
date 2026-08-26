"""The committed pin data shared across the language boundary.

ASSEMBLED_SHA256 is the reviewed anchor: one digest per tree artifact per
platform, gated by the release job before signing and embedded by the launcher
as its compiled-in expected map. It lives in ``pins.toml`` rather than here so a
Rust build step and this module read one file, held in agreement with the
launcher's compiled-in map by a drift test.
"""

import tomllib
from collections.abc import Mapping
from pathlib import Path
from typing import Any

from tasks.shared.paths import PINS_TOML


def _load(path: Path = PINS_TOML) -> Mapping[str, Mapping[str, str]]:
    with path.open("rb") as handle:
        document = tomllib.load(handle)
    return document.get("assembled_sha256", {})


def assembled_sha256(path: Path = PINS_TOML) -> Mapping[str, Mapping[str, str]]:
    """Return the ``{artifact: {platform: digest}}`` map from ``pins.toml``."""
    return _load(path)


def expected_digest(
    artifact: str, platform: str, path: Path = PINS_TOML
) -> str:
    """Return the reviewed digest for one artifact on one platform.

    Raises KeyError if the pin is absent, so a missing anchor fails loudly at
    the point the release would otherwise sign unpinned bytes.
    """
    return _load(path)[artifact][platform]


def _document(path: Path = PINS_TOML) -> dict[str, Any]:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def chromium_revision(path: Path = PINS_TOML) -> str:
    """Return the pinned Chromium revision (cross-checked at assembly)."""
    return str(_document(path)["chromium"]["revision"])


def chromium_sha256(platform: str, path: Path = PINS_TOML) -> str:
    """Return the reviewed Chromium byte digest for one platform."""
    return str(_document(path)["chromium"]["sha256"][platform])


def node_version(path: Path = PINS_TOML) -> str:
    """Return the pinned Node version (mirrors the driver's pairing)."""
    return str(_document(path)["node"]["version"])
