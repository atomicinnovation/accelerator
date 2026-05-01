import json

from invoke import Context, task

from . import version
from .shared.paths import MARKETPLACE_JSON


def read_metadata():
    return json.loads(MARKETPLACE_JSON.read_text())


def write_metadata(metadata):
    MARKETPLACE_JSON.write_text(json.dumps(metadata, indent=2))


@task
def update_version(_context: Context, plugin: str, target_version: str | None = None):
    """Update marketplace plugin ref to the given version."""
    resolved_version = target_version or version.read(_context, print_to_stdout=False)
    marketplace = read_metadata()
    for entry in marketplace["plugins"]:
        if entry["name"] == plugin:
            entry["source"]["ref"] = f"v{resolved_version}"
    write_metadata(marketplace)
