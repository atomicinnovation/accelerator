import json
import os
from pathlib import Path

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
from .shared.paths import (
    DISPATCHED_SUBBINARIES,
    PINS_TOML,
    RELEASE_MANIFEST,
    TREE_ARTIFACTS,
    tree_artifact_asset_path,
)
from .shared.targets import TARGETS
from .vendor.assemble import assert_matches_pin

# git status --porcelain markers for artifacts that must never reach the
# version-bump commit: a materialised signing secret, anything under the
# gitignored staging tree, or a symbolication archive (each present only if its
# .gitignore entry regressed).
_ARTIFACT_MARKERS = (".sec", "dist/release/", "dist/", ".debug.tar.gz")


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
    # `-uall` is load-bearing: porcelain's default untracked mode collapses a
    # wholly-untracked directory to one line, so a regressed archive rule would
    # show up as `?? skills/.../bin/` with no `.debug.tar.gz` to match on.
    result = context.run("git status --porcelain -uall", hide=True, warn=True)
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


def _assert_staged_manifest_is_current(version: str) -> None:
    """Refuse to publish a manifest that describes a different release.

    `*:finalise` is separately invocable and `dist/release/` is never cleaned,
    so a manifest from an earlier cut is reachable. The version comparison is
    what catches it — the registry changes once per sub-binary story, so a
    stale manifest has the same token set.
    """
    if not RELEASE_MANIFEST.exists():
        raise RuntimeError(
            f"{RELEASE_MANIFEST} is absent — run the prepare and sign steps "
            "before finalise"
        )
    staged = json.loads(RELEASE_MANIFEST.read_text())
    listed = set(staged["binaries"])
    if listed != set(DISPATCHED_SUBBINARIES):
        raise RuntimeError(
            f"staged manifest lists {sorted(listed)} but this release "
            f"dispatches {sorted(DISPATCHED_SUBBINARIES)} — a signed manifest "
            "promising an asset that was never uploaded cannot be recalled"
        )
    if staged["version"] != version:
        raise RuntimeError(
            f"staged manifest is version {staged['version']} but this "
            f"release is {version} — dist/release/ is from an earlier cut; "
            "re-run the prepare and sign steps"
        )
    staged_artifacts = staged.get("artifacts")
    if staged_artifacts is not None:
        expected = {
            (name, platform)
            for name in TREE_ARTIFACTS
            for _triple, platform in TARGETS
        }
        actual = {
            (name, platform)
            for name, entry in staged_artifacts.items()
            for platform in entry.get("platforms", {})
        }
        if actual != expected:
            raise RuntimeError(
                "staged manifest artifacts cover "
                f"{sorted(actual)} but this release assembles "
                f"{sorted(expected)} — a partially-assembled artifact set "
                "must not reach a signed, published manifest"
            )


def _tree_artifacts_staged() -> bool:
    return any(
        tree_artifact_asset_path(name, platform).exists()
        for name in TREE_ARTIFACTS
        for _triple, platform in TARGETS
    )


def _assert_assembled_matches_pins(pins_path: Path = PINS_TOML) -> None:
    """Gate every assembled archive against its reviewed digest before signing.

    Runs from the release job's own clean checkout against bytes that arrived as
    an opaque workflow artifact. When no archive is staged the release took the
    skip-tree-artifacts escape and there is nothing to gate; when any is staged
    the whole set must be present, so a partial assembly fails closed.
    """
    if not _tree_artifacts_staged():
        return
    for name in TREE_ARTIFACTS:
        for _triple, platform in TARGETS:
            archive = tree_artifact_asset_path(name, platform)
            if not archive.exists():
                raise RuntimeError(
                    f"{name}/{platform} archive is missing from a partial "
                    "tree-artifact assembly"
                )
            assert_matches_pin(
                archive,
                artifact=name,
                platform=platform,
                pins_path=pins_path,
            )


def _sign(context: Context) -> None:
    """Sign the staged binaries and emit the signed manifest.

    The only task that receives the signing secret. Fails closed: an absent
    secret raises inside `resolve_secret_key` rather than silently skipping.
    Tree artifacts are signed and collected only when staged, so the
    skip-tree-artifacts escape emits a manifest with no `artifacts` key.
    """
    resolved_version = str(version.read(context, print_to_stdout=False))
    with signing.resolve_secret_key() as key:
        signing.sign_staged_binaries(key)
        artifacts = None
        if _tree_artifacts_staged():
            signing.sign_tree_artifacts(key)
            artifacts = manifest.collect_artifact_entries()
        manifest.emit_manifest(
            RELEASE_MANIFEST,
            resolved_version,
            manifest.collect_entries(),
            key,
            artifacts=artifacts,
        )


def _publish(context: Context) -> None:
    resolved_version = str(version.read(context, print_to_stdout=False))
    _assert_no_leaked_artifacts(context)
    _assert_staged_manifest_is_current(resolved_version)
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
    _assert_assembled_matches_pins()
    build.create_debug_archives(context)


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
    _assert_assembled_matches_pins()
    build.create_debug_archives(context)


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
