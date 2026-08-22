"""The release-lane guard on the vendored runtime's trust anchors.

A fresh checkout ships ``pins.toml`` with placeholder digests and no publisher
keys, so a release cut before a human runs the refresh procedure would sign a
manifest
whose tree digests are placeholders and verify its upstream inputs against keys
that do not exist. Both failures otherwise surface deep in assembly as a digest
mismatch or a missing-file traceback; this predicate names them up front, so the
operator is pointed at the refresh procedure rather than at a stack trace.

Pure over its inputs: the pins document and the keys directory are parameters,
so the check is exercised against fixtures without a real release.
"""

from __future__ import annotations

import tomllib
from pathlib import Path

from tasks.shared.paths import KEYS_DIR, PINS_TOML

# The publisher keys the upstream verification reads: Node's release keyring and
# the npm registry's signing key. Absent until the refresh procedure adds them.
PUBLISHER_KEYS = ("nodejs-release.asc", "npm-registry.pem")


class TrustAnchorsNotReadyError(RuntimeError):
    """Raised when a trust anchor is still a placeholder at release time."""


def _is_placeholder_digest(value: str) -> bool:
    # The placeholders are 64 characters of one repeated glyph (all zeros, all
    # nines, and so on); a real sha256 has many distinct characters. A value
    # that is not 64 characters is malformed, a placeholder for this purpose.
    return len(value) != 64 or len(set(value)) <= 1


def _document(pins_path: Path) -> dict[str, object]:
    with pins_path.open("rb") as handle:
        return tomllib.load(handle)


def _assembled_reasons(document: dict[str, object]) -> list[str]:
    assembled = document.get("assembled_sha256", {})
    if not isinstance(assembled, dict):
        return []
    reasons: list[str] = []
    for artifact, platforms in assembled.items():
        if not isinstance(platforms, dict):
            continue
        for platform, digest in platforms.items():
            if _is_placeholder_digest(str(digest)):
                reasons.append(
                    f"assembled_sha256.{artifact}.{platform} "
                    "is a placeholder digest"
                )
    return reasons


def _chromium_reasons(document: dict[str, object]) -> list[str]:
    chromium = document.get("chromium", {})
    if not isinstance(chromium, dict):
        return ["chromium is a placeholder"]
    reasons: list[str] = []
    revision = str(chromium.get("revision", ""))
    if not revision.isdigit() or set(revision) <= {"0"}:
        reasons.append("chromium.revision is a placeholder")
    sha256 = chromium.get("sha256", {})
    if isinstance(sha256, dict):
        for platform, digest in sha256.items():
            if _is_placeholder_digest(str(digest)):
                reasons.append(
                    f"chromium.sha256.{platform} is a placeholder digest"
                )
    return reasons


def _node_reasons(document: dict[str, object]) -> list[str]:
    node = document.get("node", {})
    version = str(node.get("version", "")) if isinstance(node, dict) else ""
    if not version or version.startswith("0."):
        return ["node.version is a placeholder"]
    return []


def _key_reasons(keys_dir: Path) -> list[str]:
    reasons: list[str] = []
    for key_name in PUBLISHER_KEYS:
        key_path = keys_dir / key_name
        if not key_path.is_file() or key_path.stat().st_size == 0:
            reasons.append(f"keys/{key_name} is absent or empty")
    return reasons


def placeholder_reasons(
    pins_path: Path = PINS_TOML, keys_dir: Path = KEYS_DIR
) -> list[str]:
    """Return one message per trust anchor still carrying a placeholder value.

    An empty list means every anchor is real and a release may proceed.
    """
    document = _document(pins_path)
    return [
        *_assembled_reasons(document),
        *_chromium_reasons(document),
        *_node_reasons(document),
        *_key_reasons(keys_dir),
    ]


def assert_ready(
    pins_path: Path = PINS_TOML, keys_dir: Path = KEYS_DIR
) -> None:
    """Raise if any trust anchor is still a placeholder.

    The message lists every offending anchor and names the refresh procedure, so
    the operator has the whole remediation in one failure rather than fixing one
    anchor per aborted release.
    """
    reasons = placeholder_reasons(pins_path, keys_dir)
    if not reasons:
        return
    listed = "\n".join(f"  - {reason}" for reason in reasons)
    raise TrustAnchorsNotReadyError(
        "the vendored-runtime trust anchors are still placeholders, so this "
        "release would ship inputs no launcher could verify:\n"
        f"{listed}\n"
        'Refresh them with the "Refreshing the vendored-runtime trust anchors" '
        "procedure in RELEASING.md before cutting a release."
    )
