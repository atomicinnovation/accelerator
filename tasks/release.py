from invoke import Context, task

from . import version, git

@task
def prerelease(context: Context):
    """Prepare a release candidate."""
    version.bump(context, release_type="pre")
    git.tag_version(context)
    git.commit_version(context)
    git.push(context)
