import json
from enum import StrEnum
from typing import Any

import semver
import tomlkit
from invoke import Context, task

from .shared.files import atomic_write_text
from .shared.paths import (
    CLI_WORKSPACE_CARGO_TOML,
    PLUGIN_JSON,
)


class BumpType(StrEnum):
    MAJOR = "major"
    MINOR = "minor"
    PATCH = "patch"
    PRE = "pre"
    FINALISE = "finalise"
    NEXT_MINOR = "next-minor"


def read_plugin_metadata() -> dict[str, Any]:
    return json.loads(PLUGIN_JSON.read_text())


def _render_plugin_json(version: str) -> str:
    data = read_plugin_metadata()
    data["version"] = version
    return json.dumps(data, indent=2)


def _render_workspace_cargo_toml(version: str) -> str:
    # Assumes a well-formed [workspace.package] table (mirroring
    # _render_cargo_toml's [package] assumption): the validating read side
    # (validate_version_coherence) is what surfaces a malformed manifest.
    data = tomlkit.parse(CLI_WORKSPACE_CARGO_TOML.read_text())
    data["workspace"]["package"]["version"] = version
    return tomlkit.dumps(data)


@task
def read(_context: Context, print_to_stdout: bool = True) -> semver.Version:
    """Read plugin version."""
    plugin_metadata = read_plugin_metadata()
    current_version = plugin_metadata["version"]
    if print_to_stdout:
        print(current_version)
    return semver.Version.parse(current_version)


@task
def write(_context: Context, version: str) -> None:
    """Write plugin version to plugin.json and the cli/ workspace manifest.

    The visualiser server inherits its version from
    [workspace.package].version, so bumping the cli/ workspace manifest covers
    it; writing the member manifest would clobber that inheritance.
    """
    rendered_plugin_json = _render_plugin_json(version)
    rendered_workspace_cargo_toml = _render_workspace_cargo_toml(version)

    atomic_write_text(PLUGIN_JSON, rendered_plugin_json)
    atomic_write_text(CLI_WORKSPACE_CARGO_TOML, rendered_workspace_cargo_toml)


@task(iterable=["bump_type"])
def bump(
    _context: Context, bump_type: list[BumpType] | None = None
) -> semver.Version:
    """Bump plugin version."""
    # "pre" is the semver pre-release token (e.g. 1.2.3-pre.4), not a secret —
    # S105's "token"-named-string heuristic is a false positive here.
    prerelease_token = "pre"  # noqa: S105
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
                # A finalised release has no prerelease component. Bumping its
                # prerelease directly would re-cut <x.y.z>-pre.1, colliding
                # with the prerelease tags that led up to that release. Advance
                # to the next minor's first prerelease so the post-stable cut
                # opens a fresh line; an in-progress prerelease just increments.
                if new_version.prerelease is None:
                    new_version = new_version.bump_minor().bump_prerelease(
                        token=prerelease_token
                    )
                else:
                    new_version = new_version.bump_prerelease(
                        token=prerelease_token
                    )
            case BumpType.FINALISE:
                new_version = new_version.finalize_version()
            case BumpType.NEXT_MINOR:
                new_version = new_version.next_version(
                    part="minor", prerelease_token=prerelease_token
                )

    write(_context, str(new_version))

    return new_version
