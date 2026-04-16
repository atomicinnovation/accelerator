from invoke import Context, task

from . import changelog, git, marketplace, version

@task
def prerelease(context: Context):
    """Prepare a release candidate."""
    git.configure(context)
    git.pull(context)

    version.bump(context, release_type="pre")
    git.commit_version(context)
    git.tag_version(context)

    git.push(context)


@task
def release(context: Context):
    """Prepare a release."""
    git.configure(context)
    git.pull(context)

    release_version = version.bump(context, release_type="minor")
    marketplace.update_version(context, plugin="accelerator", version=str(release_version))
    changelog.release(context)
    git.commit_version(context)
    git.tag_version(context)

    version.bump(context, release_type="pre")
    git.commit_version(context)
    git.tag_version(context)

    git.push(context)
