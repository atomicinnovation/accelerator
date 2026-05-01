import hashlib
import json
import tomllib
from pathlib import Path
from typing import Mapping

import semver

from .paths import CARGO_TOML, CHECKSUMS, PLUGIN_JSON, REPO_ROOT

_CARGO_TOML_RELATIVE  = CARGO_TOML.relative_to(REPO_ROOT)
_PLUGIN_JSON_RELATIVE = PLUGIN_JSON.relative_to(REPO_ROOT)
_CHECKSUMS_RELATIVE   = CHECKSUMS.relative_to(REPO_ROOT)


class ReleaseHelperError(Exception): ...
class VersionCoherenceError(ReleaseHelperError): ...
class InvalidVersionError(ReleaseHelperError): ...


def _atomic_write_text(path: Path, content: str) -> None:
    tmp = path.with_suffix(path.suffix + ".tmp")
    try:
        tmp.write_text(content)
        tmp.replace(path)
    except BaseException:
        tmp.unlink(missing_ok=True)
        raise


def _read_plugin_json_version(root: Path) -> str:
    data = json.loads((root / _PLUGIN_JSON_RELATIVE).read_text())
    return data["version"]


def _read_cargo_toml_version(root: Path) -> str:
    with open(root / _CARGO_TOML_RELATIVE, "rb") as f:
        data = tomllib.load(f)
    return data["package"]["version"]


def _read_checksums_json_version(root: Path) -> str:
    data = json.loads((root / _CHECKSUMS_RELATIVE).read_text())
    return data["version"]


def compute_sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(64 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def update_checksums_json(
    manifest_path: Path,
    version: str,
    platform_hashes: Mapping[str, str] | None = None,
) -> None:
    data = json.loads(manifest_path.read_text())
    data["version"] = version
    if platform_hashes:
        for platform, hex_digest in platform_hashes.items():
            data.setdefault("binaries", {})[platform] = f"sha256:{hex_digest}"
    _atomic_write_text(manifest_path, json.dumps(data, indent=2) + "\n")


def validate_version_coherence(
    expected_version: str,
    repo_root: Path | None = None,
) -> None:
    if not expected_version:
        raise InvalidVersionError("expected_version must not be empty")
    root = repo_root or REPO_ROOT
    found = {
        "plugin.json":    _read_plugin_json_version(root),
        "Cargo.toml":     _read_cargo_toml_version(root),
        "checksums.json": _read_checksums_json_version(root),
    }
    mismatches = {k: v for k, v in found.items() if v != expected_version}
    if mismatches:
        raise VersionCoherenceError(
            f"expected {expected_version!r}, found mismatches: {mismatches}"
        )


def is_prerelease_version(version: str) -> bool:
    try:
        parsed = semver.Version.parse(version)
    except (ValueError, TypeError) as exc:
        raise InvalidVersionError(f"not a valid semver: {version!r}") from exc
    return bool(parsed.prerelease)
