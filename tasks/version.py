import json
from enum import StrEnum

import tomlkit
import semver
from invoke import Context, task

from .shared.paths import CARGO_TOML, CHECKSUMS, PLUGIN_JSON
from .shared.files import _atomic_write_text


class BumpType(StrEnum):
    MAJOR = "major"
    MINOR = "minor"
    PATCH = "patch"
    PRE = "pre"
    FINALISE = "finalise"
    NEXT_MINOR = "next-minor"


def read_plugin_metadata():
    return json.loads(PLUGIN_JSON.read_text())


def _render_plugin_json(version: str) -> str:
    data = read_plugin_metadata()
    data["version"] = version
    return json.dumps(data, indent=2)


def _render_cargo_toml(version: str) -> str:
    data = tomlkit.parse(CARGO_TOML.read_text())
    data["package"]["version"] = version
    return tomlkit.dumps(data)


def _render_checksums_version(version: str) -> str:
    data = json.loads(CHECKSUMS.read_text())
    data["version"] = version
    return json.dumps(data, indent=2) + "\n"


@task
def read(_context: Context, print_to_stdout: bool = True):
    """Read plugin version."""
    plugin_metadata = read_plugin_metadata()
    current_version = plugin_metadata["version"]
    if print_to_stdout:
        print(current_version)
    return semver.Version.parse(current_version)


@task
def write(_context: Context, version: str):
    """Write plugin version to plugin.json, Cargo.toml, and checksums.json."""
    rendered_plugin_json = _render_plugin_json(version)
    rendered_cargo_toml = _render_cargo_toml(version)
    rendered_checksums = _render_checksums_version(version)

    _atomic_write_text(PLUGIN_JSON, rendered_plugin_json)
    _atomic_write_text(CARGO_TOML, rendered_cargo_toml)
    _atomic_write_text(CHECKSUMS, rendered_checksums)


@task(iterable=["bump_type"])
def bump(_context: Context, bump_type=None):
    """Bump plugin version."""
    prerelease_token = "pre"
    current_version = read(_context, print_to_stdout=False)
    new_version = current_version

    bump_types = bump_type or (BumpType.PRE,)
    for bt in bump_types:
        match bt:
            case BumpType.MAJOR:
                new_version = new_version.bump_major()
            case BumpType.MINOR:
                new_version = new_version.bump_minor()
            case BumpType.PATCH:
                new_version = new_version.bump_patch()
            case BumpType.PRE:
                new_version = new_version.bump_prerelease(
                    token=prerelease_token
                )
            case BumpType.FINALISE:
                new_version = new_version.finalize_version()
            case BumpType.NEXT_MINOR:
                new_version = new_version.next_version(
                    part="minor",
                    prerelease_token=prerelease_token
                )

    write(_context, str(new_version))

    return new_version
