from invoke import Context, task

from . import changelog, git, version

@task
def prerelease(context: Context):
    """Prepare a release candidate."""
    git.configure(context)
    git.pull(context)

    version.bump(context, release_type="pre")
    git.tag_version(context)
    git.commit_version(context)

    git.push(context)


@task
def release(context: Context):
    """Prepare a release."""
    git.configure(context)
    git.pull(context)

    version.bump(context, release_type="minor")
    changelog.release(context)
    git.tag_version(context)
    git.commit_version(context)

    version.bump(context, release_type="pre")
    git.tag_version(context)
    git.commit_version(context)

    git.push(context)
