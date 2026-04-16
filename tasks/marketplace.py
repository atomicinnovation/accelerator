import json

from invoke import Context, task

from . import version


def read_metadata():
    return json.load(open(".claude-plugin/marketplace.json"))


def write_metadata(metadata):
    json.dump(
        metadata,
        open(".claude-plugin/marketplace.json", "w"),
        indent=2
    )


@task
def update_version(_context: Context, plugin: str, target_version: str | None = None):
    """Update marketplace plugin ref to the given version."""
    resolved_version = target_version or version.read(_context, print_to_stdout=False)
    marketplace = read_metadata()
    for entry in marketplace["plugins"]:
        if entry["name"] == plugin:
            entry["source"]["ref"] = f"v{resolved_version}"
    write_metadata(marketplace)
