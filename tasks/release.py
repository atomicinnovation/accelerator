import os

from invoke import Context, task

from . import build, changelog, git, github, marketplace, version


def _refuse_under_ci(task_name: str) -> None:
    """Raise if called from a CI environment.

    The local-dev convenience tasks (prerelease/release) skip SLSA attestation
    because they run outside GitHub Actions. CI must use the prepare/finalize
    split so the workflow can interleave actions/attest-build-provenance
    between build and publish.
    """
    if os.environ.get("GITHUB_ACTIONS") or os.environ.get("CI"):
        raise RuntimeError(
            f"{task_name} is the local-dev convenience task; CI must use "
            f"the prepare/finalize split (mise run release:prerelease-prepare "
            f"+ release:prerelease-finalize). Bypassing the split skips SLSA "
            f"attestation."
        )


def _publish(context: Context) -> None:
    resolved_version = str(version.read(context, print_to_stdout=False))
    git.commit_version(context)
    git.tag_version(context)
    git.push(context)
    github.create_release(context, target_version=resolved_version)
    github.upload_and_verify(context, resolved_version)


@task
def prerelease_prepare(context: Context):
    """CI prerelease halve 1: bump version, build binaries, compute checksums."""
    git.configure(context)
    git.pull(context)
    version.bump(context, bump_type=[version.BumpType.PRE])
    resolved_version = str(version.read(context, print_to_stdout=False))
    build.create_checksums(context, resolved_version)


@task
def prerelease_finalize(context: Context):
    """CI prerelease halve 2: commit, tag, push, create release, upload, publish."""
    _publish(context)


@task
def stable_prepare(context: Context):
    """CI stable halve 1: finalise version, update marketplace and changelog, build binaries."""
    git.configure(context)
    git.pull(context)
    version.bump(context, bump_type=[version.BumpType.FINALISE])
    resolved_version = str(version.read(context, print_to_stdout=False))
    marketplace.update_version(context, plugin="accelerator")
    changelog.release(context)
    build.create_checksums(context, resolved_version)


@task
def stable_publish(context: Context):
    """CI stable halve 2: commit, tag, push, create release, upload, publish."""
    _publish(context)


@task
def post_stable_prepare(context: Context):
    """CI post-stable halve 1: bump to next minor pre, build binaries."""
    version.bump(context, bump_type=[version.BumpType.MINOR, version.BumpType.PRE])
    resolved_version = str(version.read(context, print_to_stdout=False))
    build.create_checksums(context, resolved_version)


@task
def post_stable_publish(context: Context):
    """CI post-stable halve 2: commit, tag, push, create release, upload, publish."""
    _publish(context)


@task
def prerelease(context: Context):
    """Local-dev only: full prerelease flow without SLSA attestation."""
    _refuse_under_ci("prerelease")
    prerelease_prepare(context)
    prerelease_finalize(context)


@task
def release(context: Context):
    """Local-dev only: full stable release flow without SLSA attestation."""
    _refuse_under_ci("release")
    stable_prepare(context)
    stable_publish(context)
    post_stable_prepare(context)
    post_stable_publish(context)
