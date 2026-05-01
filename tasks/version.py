import json
from enum import StrEnum

import semver
from invoke import Context, task

from .shared.paths import PLUGIN_JSON


class BumpType(StrEnum):
    MAJOR = "major"
    MINOR = "minor"
    PATCH = "patch"
    PRE = "pre"
    FINALISE = "finalise"
    NEXT_MINOR = "next-minor"


def read_plugin_metadata():
    return json.loads(PLUGIN_JSON.read_text())


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
    """Write plugin version."""
    plugin_metadata = read_plugin_metadata()
    plugin_metadata["version"] = version
    PLUGIN_JSON.write_text(json.dumps(plugin_metadata, indent=2))


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
