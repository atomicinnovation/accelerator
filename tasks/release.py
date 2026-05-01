from invoke import Context, task

from . import build, changelog, git, github, marketplace, version


@task
def prerelease(context: Context):
    """Prepare a release candidate."""
    git.configure(context)
    git.pull(context)

    version.bump(context, bump_type=[version.BumpType.PRE])
    git.commit_version(context)
    git.tag_version(context)
    git.push(context)

    resolved_version = version.read(context, print_to_stdout=False)
    github.create_release(context, target_version=resolved_version)
    build.create_checksums(context, resolved_version)
    github.upload_and_verify(context, resolved_version)


@task
def release(context: Context):
    """Prepare a release."""
    git.configure(context)
    git.pull(context)

    version.bump(context, bump_type=[version.BumpType.FINALISE])
    marketplace.update_version(context, plugin="accelerator")
    changelog.release(context)

    git.commit_version(context)
    git.tag_version(context)
    git.push(context)

    resolved_version = version.read(context, print_to_stdout=False)
    github.create_release(context, target_version=resolved_version)
    build.create_checksums(context, resolved_version)
    github.upload_and_verify(context, resolved_version)

    version.bump(
        context, bump_type=[version.BumpType.MINOR, version.BumpType.PRE]
    )
    git.commit_version(context)
    git.tag_version(context)
    git.push(context)
