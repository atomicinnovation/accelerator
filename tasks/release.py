import os

from invoke import Context, task

from . import (
    build,
    changelog,
    git,
    github,
    manifest,
    marketplace,
    signing,
    version,
)
from .shared.paths import RELEASE_MANIFEST

# git status --porcelain markers for artifacts that must never reach the
# version-bump commit: a materialised signing secret, or anything under the
# gitignored staging tree (present only if the .gitignore entry regressed).
_ARTIFACT_MARKERS = (".sec", "dist/release/", "dist/")


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


def _assert_no_leaked_artifacts(context: Context) -> None:
    result = context.run("git status --porcelain", hide=True, warn=True)
    offenders = [
        line
        for line in result.stdout.splitlines()
        if any(marker in line for marker in _ARTIFACT_MARKERS)
    ]
    if offenders:
        raise RuntimeError(
            "refusing to commit: build artifacts or a signing secret would be "
            f"swept into the version-bump commit:\n{chr(10).join(offenders)}"
        )


def _sign(context: Context) -> None:
    """Sign the staged binaries and emit the signed manifest.

    The only task that receives the signing secret. Fails closed: an absent
    secret raises inside `resolve_secret_key` rather than silently skipping.
    """
    resolved_version = str(version.read(context, print_to_stdout=False))
    with signing.resolve_secret_key() as key:
        signing.sign_staged_binaries(key)
        manifest.emit_manifest(
            RELEASE_MANIFEST,
            resolved_version,
            manifest.collect_entries(),
            key,
        )


def _publish(context: Context) -> None:
    resolved_version = str(version.read(context, print_to_stdout=False))
    _assert_no_leaked_artifacts(context)
    git.commit_version(context)
    git.tag_version(context)
    git.push(context)
    github.create_release(context, target_version=resolved_version)
    github.upload_and_verify_release(context, resolved_version)


# ── CI split tasks ────────────────────────────────────────────────────


@task
def prerelease_prepare(context: Context) -> None:
    """CI prerelease step 1: bump version, cross-compile, checksum."""
    git.configure(context)
    git.pull(context)
    version.bump(context, bump_type=[version.BumpType.PRE])
    resolved_version = str(version.read(context, print_to_stdout=False))
    marketplace.update_prerelease_version(context, plugin="accelerator")
    build.frontend(context)
    build.server_cross_compile(context)
    build.cli_cross_compile(context)
    build.assert_staged_launcher_versions(resolved_version)
    build.create_debug_archives(context)
    build.create_checksums(context, resolved_version)


@task
def prerelease_sign(context: Context) -> None:
    """CI prerelease step 2: sign the staged binaries and manifest."""
    _sign(context)


@task
def prerelease_finalise(context: Context) -> None:
    """CI prerelease step 3: commit, tag, push, release, publish."""
    _publish(context)


@task
def release_prepare(context: Context) -> None:
    """CI stable release step 1: finalise version and cross-compile.

    Also updates the marketplace version and changelog before building.
    """
    git.configure(context)
    git.pull(context)
    version.bump(context, bump_type=[version.BumpType.FINALISE])
    resolved_version = str(version.read(context, print_to_stdout=False))
    marketplace.update_version(context, plugin="accelerator")
    changelog.release(context)
    build.frontend(context)
    build.server_cross_compile(context)
    build.cli_cross_compile(context)
    build.assert_staged_launcher_versions(resolved_version)
    build.create_debug_archives(context)
    build.create_checksums(context, resolved_version)


@task
def release_sign(context: Context) -> None:
    """CI stable release step 2: sign the staged binaries and manifest."""
    _sign(context)


@task
def release_finalise(context: Context) -> None:
    """CI stable release step 3: commit, tag, push, release, publish."""
    _publish(context)


# ── Local-dev convenience wrappers ───────────────────────────────────


@task
def prerelease(context: Context) -> None:
    """Local-dev only: full prerelease flow without SLSA attestation."""
    _refuse_under_ci("prerelease")
    prerelease_prepare(context)
    prerelease_sign(context)
    prerelease_finalise(context)


@task
def release(context: Context) -> None:
    """Local-dev only: full stable release flow without SLSA attestation.

    Runs: release prepare → sign → finalise → prerelease prepare → sign →
    finalise (the post-stable pre.0 cut is a standard prerelease).
    """
    _refuse_under_ci("release")
    release_prepare(context)
    release_sign(context)
    release_finalise(context)
    prerelease_prepare(context)
    prerelease_sign(context)
    prerelease_finalise(context)
