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
