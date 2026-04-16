import json
from enum import StrEnum

import semver
from invoke import Context, task


class ReleaseType(StrEnum):
    MAJOR = "major"
    MINOR = "minor"
    PATCH = "patch"
    PRE = "pre"


def read_plugin_metadata():
    return json.load(open(".claude-plugin/plugin.json"))


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
    json.dump(
        plugin_metadata,
        open(".claude-plugin/plugin.json", "w"),
        indent=2
    )


@task
def bump(_context: Context, release_type: ReleaseType = ReleaseType.PRE):
    """Bump plugin version."""
    current_version = read(_context, print_to_stdout=False)

    match release_type:
        case ReleaseType.MAJOR:
            new_version = current_version.bump_major()
        case ReleaseType.MINOR:
            new_version = current_version.bump_minor()
        case ReleaseType.PATCH:
            new_version = current_version.bump_patch()
        case ReleaseType.PRE:
            new_version = current_version.bump_prerelease(token="pre")

    write(_context, str(new_version))

    return new_version
