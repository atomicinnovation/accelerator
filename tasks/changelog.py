import keepachangelog
from invoke import Context, task

from tasks.version import read as version_read


@task
def release(context: Context):
    """Mark unreleased changelog entries with the current version."""
    version = version_read(context, print_to_stdout=False)
    keepachangelog.release("CHANGELOG.md", new_version=str(version))
