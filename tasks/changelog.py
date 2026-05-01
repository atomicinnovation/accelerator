import keepachangelog
from invoke import Context, task

from . import version
from .shared.paths import CHANGELOG


@task
def release(context: Context):
    """Mark unreleased changelog entries with the current version."""
    current_version = version.read(context, print_to_stdout=False)
    keepachangelog.release(str(CHANGELOG), new_version=str(current_version))
