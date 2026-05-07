import os

from invoke import Context, task

from . import build, changelog, git, github, marketplace, version


def _refuse_under_ci(task_name: str) -> None:
    """Raise if called from a CI environment.

    Local-dev convenience tasks skip SLSA attestation because they run outside
    GitHub Actions. CI must use the prepare/finalise split so the workflow can
    interleave actions/attest-build-provenance between build and publish.
    """
    if os.environ.get("GITHUB_ACTIONS") or os.environ.get("CI"):
        raise RuntimeError(
            f"{task_name} is the local-dev convenience task; CI must use "
            f"the prepare/finalise split (mise run prerelease:prepare + "
            f"prerelease:finalise). Bypassing the split skips SLSA attestation."
        )


def _publish(context: Context) -> None:
    resolved_version = str(version.read(context, print_to_stdout=False))
    git.commit_version(context)
    git.tag_version(context)
    git.push(context)
    github.create_release(context, target_version=resolved_version)
    github.upload_and_verify(context, resolved_version)


# ── CI split tasks ────────────────────────────────────────────────────


@task
def prerelease_prepare(context: Context):
    """CI prerelease halve 1: bump version, build binaries, compute checksums."""
    git.configure(context)
    git.pull(context)
    version.bump(context, bump_type=[version.BumpType.PRE])
    resolved_version = str(version.read(context, print_to_stdout=False))
    marketplace.update_prerelease_version(context, plugin="accelerator")
    build.frontend(context)
    build.server_cross_compile(context)
    build.create_debug_archives(context)
    build.create_checksums(context, resolved_version)


@task
def prerelease_finalise(context: Context):
    """CI prerelease halve 2: commit, tag, push, create release, upload, publish."""
    _publish(context)


@task
def release_prepare(context: Context):
    """CI stable release halve 1: finalise version, update marketplace and changelog, build binaries."""
    git.configure(context)
    git.pull(context)
    version.bump(context, bump_type=[version.BumpType.FINALISE])
    resolved_version = str(version.read(context, print_to_stdout=False))
    marketplace.update_version(context, plugin="accelerator")
    changelog.release(context)
    build.frontend(context)
    build.server_cross_compile(context)
    build.create_debug_archives(context)
    build.create_checksums(context, resolved_version)


@task
def release_finalise(context: Context):
    """CI stable release halve 2: commit, tag, push, create release, upload, publish."""
    _publish(context)


# ── Local-dev convenience wrappers ───────────────────────────────────


@task
def prerelease(context: Context):
    """Local-dev only: full prerelease flow without SLSA attestation."""
    _refuse_under_ci("prerelease")
    prerelease_prepare(context)
    prerelease_finalise(context)


@task
def release(context: Context):
    """Local-dev only: full stable release flow without SLSA attestation.

    Runs: release prepare → release finalise → prerelease prepare →
    prerelease finalise (the post-stable pre.0 cut is a standard prerelease).
    """
    _refuse_under_ci("release")
    release_prepare(context)
    release_finalise(context)
    prerelease_prepare(context)
    prerelease_finalise(context)
