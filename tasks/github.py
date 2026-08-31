import json
import shlex
import subprocess
import tempfile
from collections.abc import Callable, Iterable, Mapping
from functools import partial
from pathlib import Path
from typing import TYPE_CHECKING, NamedTuple

import semver
from invoke import Context, task

if TYPE_CHECKING:
    from tasks.manifest import Manifest

from tasks.shared.errors import InvalidVersionError
from tasks.shared.hashing import compute_sha256
from tasks.shared.paths import (
    ATTRIBUTION_ARTEFACT_STAGED,
    DEBUG_ARCHIVE_DIRS,
    DISPATCHED_SUBBINARIES,
    RELEASE_MANIFEST,
    RELEASE_MANIFEST_SIG,
    RELEASE_PUBLIC_KEY,
    TREE_ARTEFACTS,
    cli_binary_path,
    debug_archive_path,
    subbinary_asset_path,
    tree_artifact_asset_path,
    vendored_shim_path,
)
from tasks.shared.targets import TARGETS, host_platform


def is_prerelease_version(version: str) -> bool:
    try:
        parsed = semver.Version.parse(version)
    except (ValueError, TypeError) as exc:
        raise InvalidVersionError(f"not a valid semver: {version!r}") from exc
    return bool(parsed.prerelease)


def _emit_forensic_alert(
    context: Context, tag: str, track: str, message: str
) -> None:
    print(f"::error title={track} release {tag}::{message}", flush=True)


class AssetVerificationError(Exception):
    pass


@task
def check_auth(context: Context) -> None:
    """Verify the GitHub CLI is authenticated."""
    result = context.run("gh auth status", warn=True, hide=True)
    if result.return_code != 0:
        raise RuntimeError(
            "gh auth status failed — run 'gh auth login' or set GH_TOKEN"
        )


@task
def create_release(context: Context, target_version: str | None = None) -> None:
    """Create a draft GitHub release for the current version.

    Passes --prerelease for pre-release versions (X.Y.Z-suffix) and
    --draft unconditionally so no assets are visible until
    upload_and_verify_release has verified every asset and published it.
    """
    from tasks import version

    resolved_version = str(
        target_version or version.read(context, print_to_stdout=False)
    )
    tag = f"v{resolved_version}"
    cmd = [
        "gh",
        "release",
        "create",
        tag,
        "--draft",
        "--generate-notes",
        "--title",
        tag,
    ]
    if is_prerelease_version(resolved_version):
        cmd.append("--prerelease")
    context.run(shlex.join(cmd), pty=True)


@task
def upload_release_asset(context: Context, tag: str, path: Path) -> None:
    """Upload a single asset file to a GitHub release."""
    context.run(f"gh release upload {tag} {path}", pty=True)


@task
def download_release_asset(
    context: Context, tag: str, asset_name: str, output_path: Path
) -> None:
    """Download a single asset from a GitHub release to output_path."""
    result = subprocess.run(
        [
            "gh",
            "release",
            "download",
            tag,
            "--pattern",
            asset_name,
            "--output",
            str(output_path),
            "--clobber",
        ],
        capture_output=True,
        text=True,
        timeout=120,
        check=False,
    )
    if result.returncode != 0:
        raise AssetVerificationError(
            f"gh release download failed: {result.stderr.strip()}"
        )


@task
def verify_release_asset(
    context: Context, path: Path, expected_hex: str
) -> None:
    """Verify the SHA-256 of a local file matches expected_hex."""
    actual = compute_sha256(path)
    if actual != expected_hex:
        raise AssetVerificationError(
            f"{path.name}: expected sha256:{expected_hex}, got sha256:{actual}"
        )


@task
def download_and_verify(
    context: Context, release_tag: str, asset_name: str, expected_hex: str
) -> None:
    """Download a release asset to a temp file and verify its SHA-256."""
    with tempfile.NamedTemporaryFile(delete=False) as tmp:
        tmp_path = Path(tmp.name)
    try:
        try:
            download_release_asset(context, release_tag, asset_name, tmp_path)
        except subprocess.TimeoutExpired as exc:
            raise AssetVerificationError(
                f"gh release download timed out for {asset_name}"
            ) from exc
        verify_release_asset(context, tmp_path, expected_hex)
    finally:
        tmp_path.unlink(missing_ok=True)


# ── unified launcher + manifest + sub-binary publish ──────────────────

_PRESERVE_MESSAGE = "AssetVerificationError — draft + tag PRESERVED for triage"


class _Reverify(NamedTuple):
    track: str
    run: Callable[[], None]


def _sig(binary: Path) -> Path:
    return binary.with_name(binary.name + ".minisig")


def _mktemp() -> Path:
    with tempfile.NamedTemporaryFile(delete=False) as tmp:
        return Path(tmp.name)


def _run_shim(signature: Path, binary: Path, asset: str) -> None:
    # Host-arch shim; macos-latest is darwin-arm64. Verified against the
    # committed keys/accelerator-release.pub (the same file build.rs embeds), so
    # the check genuinely guards "signed by the key launchers embed" rather than
    # passing tautologically against a key derived from the signing secret.
    shim = vendored_shim_path(host_platform())
    result = subprocess.run(
        [str(shim), str(RELEASE_PUBLIC_KEY), str(signature), str(binary)],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise AssetVerificationError(f"{asset}: minisign verification failed")


def _reverify_via_shim(
    context: Context, tag: str, asset: str, sig_asset: str
) -> None:
    binary = _mktemp()
    signature = _mktemp()
    try:
        download_release_asset(context, tag, asset, binary)
        download_release_asset(context, tag, sig_asset, signature)
        _run_shim(signature, binary, asset)
    finally:
        binary.unlink(missing_ok=True)
        signature.unlink(missing_ok=True)


def _reverify_subbinary(
    context: Context, tag: str, asset: str, expected_sha: str, inline_sig: str
) -> None:
    binary = _mktemp()
    signature = _mktemp()
    try:
        download_release_asset(context, tag, asset, binary)
        actual = compute_sha256(binary)
        if actual != expected_sha:
            raise AssetVerificationError(
                f"{asset}: expected sha256:{expected_sha}, got sha256:{actual}"
            )
        signature.write_text(inline_sig)
        _run_shim(signature, binary, asset)
    finally:
        binary.unlink(missing_ok=True)
        signature.unlink(missing_ok=True)


def _subbinary_uploads(
    tokens: Iterable[str] = DISPATCHED_SUBBINARIES,
) -> list[Path]:
    uploads: list[Path] = []
    for token in tokens:
        for _triple, platform in TARGETS:
            asset = subbinary_asset_path(token, platform)
            uploads.append(asset)
            uploads.append(_sig(asset))
    return uploads


def _tree_artifact_uploads(
    tree_tokens: Iterable[str] = TREE_ARTEFACTS,
) -> list[Path]:
    """Every tree archive and its three sidecars.

    Four files per artifact per platform: the archive, its `.minisig`, its
    `.sealed` attestation, and that document's `.sealed.sig`. The `.sealed.sig`
    is produced by the publishing job (the signing key lives only here), so it
    is uploaded from `dist/release/` like the others.
    """
    uploads: list[Path] = []
    for token in tree_tokens:
        for _triple, platform in TARGETS:
            archive = tree_artifact_asset_path(token, platform)
            sealed = archive.with_name(archive.name + ".sealed")
            uploads.append(archive)
            uploads.append(_sig(archive))
            uploads.append(sealed)
            uploads.append(sealed.with_name(sealed.name + ".sig"))
    return uploads


def _release_uploads(
    tokens: Iterable[str] = DISPATCHED_SUBBINARIES,
    debug_dirs: Mapping[str, Path] = DEBUG_ARCHIVE_DIRS,
    tree_tokens: Iterable[str] = (),
) -> list[Path]:
    uploads: list[Path] = []
    for _triple, platform in TARGETS:
        # Each sub-binary is published once, as the shared
        # accelerator-<token>-<platform> manifest asset below; only its debug
        # archive ships from the sub-binary's committed bin/ tree here.
        for token, directory in debug_dirs.items():
            uploads.append(debug_archive_path(token, platform, directory))
        launcher = cli_binary_path("accelerator", platform)
        uploads.append(launcher)
        uploads.append(_sig(launcher))
    uploads.append(RELEASE_MANIFEST)
    uploads.append(RELEASE_MANIFEST_SIG)
    # Unsigned: the notice is not a trust anchor, so it carries no `.minisig`
    # and no _release_reverifies() entry. SLSA provenance covers it via the
    # dist/release/accelerator-* attest glob.
    uploads.append(ATTRIBUTION_ARTEFACT_STAGED)
    uploads.extend(_subbinary_uploads(tokens))
    uploads.extend(_tree_artifact_uploads(tree_tokens))
    return uploads


def _release_reverifies(
    context: Context,
    tag: str,
    tokens: Iterable[str] = DISPATCHED_SUBBINARIES,
    tree_tokens: Iterable[str] = (),
) -> list[_Reverify]:
    items: list[_Reverify] = []
    for _triple, platform in TARGETS:
        launcher = cli_binary_path("accelerator", platform)
        items.append(
            _Reverify(
                "Launcher/manifest",
                partial(
                    _reverify_via_shim,
                    context,
                    tag,
                    launcher.name,
                    _sig(launcher).name,
                ),
            )
        )
    items.append(
        _Reverify(
            "Launcher/manifest",
            partial(
                _reverify_via_shim,
                context,
                tag,
                "manifest.json",
                "manifest.minisig",
            ),
        )
    )
    items.extend(_subbinary_reverifies(context, tag, tokens))
    items.extend(_tree_artifact_reverifies(context, tag, tree_tokens))
    return items


def _tree_artifact_reverifies(
    context: Context,
    tag: str,
    tree_tokens: Iterable[str] = TREE_ARTEFACTS,
) -> list[_Reverify]:
    """Re-verify each tree archive and its `.sealed` attestation.

    The archive carries an inline signature in the manifest, so it re-verifies
    like a sub-binary; the attestation's signature is detached (`.sealed.sig`),
    so it re-verifies via the shim. An artifact whose attestation failed to
    upload would otherwise publish a tree no launcher could resolve.
    """
    names = tuple(tree_tokens)
    if not names:
        return []
    manifest: Manifest = json.loads(RELEASE_MANIFEST.read_text())
    items: list[_Reverify] = []
    for name in names:
        entry = manifest["artifacts"][name]
        for _triple, platform in TARGETS:
            archive = tree_artifact_asset_path(name, platform)
            plat = entry["platforms"][platform]
            items.append(
                _Reverify(
                    "Launcher/manifest",
                    partial(
                        _reverify_subbinary,
                        context,
                        tag,
                        archive.name,
                        plat["sha256"].removeprefix("sha256:"),
                        plat["signature"],
                    ),
                )
            )
            sealed = archive.name + ".sealed"
            items.append(
                _Reverify(
                    "Launcher/manifest",
                    partial(
                        _reverify_via_shim,
                        context,
                        tag,
                        sealed,
                        sealed + ".sig",
                    ),
                )
            )
    return items


def _subbinary_reverifies(
    context: Context,
    tag: str,
    tokens: Iterable[str] = DISPATCHED_SUBBINARIES,
) -> list[_Reverify]:
    names = tuple(tokens)
    if not names:
        return []
    manifest: Manifest = json.loads(RELEASE_MANIFEST.read_text())
    items: list[_Reverify] = []
    for name in names:
        entry = manifest["binaries"][name]
        for _triple, platform in TARGETS:
            asset = subbinary_asset_path(name, platform).name
            plat = entry["platforms"][platform]
            items.append(
                _Reverify(
                    "Launcher/manifest",
                    partial(
                        _reverify_subbinary,
                        context,
                        tag,
                        asset,
                        plat["sha256"].removeprefix("sha256:"),
                        plat["signature"],
                    ),
                )
            )
    return items


def _upload_clobber(context: Context, tag: str, path: Path) -> None:
    context.run(f"gh release upload {tag} {path} --clobber", pty=True)


@task
def upload_and_verify_release(context: Context, version: str) -> None:
    """Upload every release asset, re-verify, then publish once.

    Owns the single `--draft=false` transition, flipped only after every asset
    (launcher shim-minisig, manifest shim-minisig, sub-binary and tree-archive
    sha256 + inline signature, tree attestation shim-minisig) re-verifies. Any
    failure — verification or otherwise — preserves the draft with a forensic
    alert and re-raises; no path deletes the tag, because `_publish` has already
    pushed the version bump and the marketplace ref, so a delete would break
    installs for every user. Uploads are `--clobber` so a preserved draft can be
    re-driven to green without manual asset deletion.
    """
    tag = f"v{version}"
    # Resolved once and threaded: the "every asset uploaded" and "every asset
    # re-verified before --draft=false" lists cannot derive from two values.
    # Tree tokens come from the manifest's own artifacts map, so a release that
    # omitted them (the skip-tree-artifacts escape) uploads/re-verifies none.
    tokens = DISPATCHED_SUBBINARIES
    tree_tokens = tuple(
        json.loads(RELEASE_MANIFEST.read_text()).get("artifacts", {})
    )
    uploads = _release_uploads(tokens, tree_tokens=tree_tokens)
    missing = [p for p in uploads if not p.exists()]
    if missing:
        raise FileNotFoundError(
            f"Expected release artefacts not found: {[str(p) for p in missing]}"
        )
    reverifies = _release_reverifies(context, tag, tokens, tree_tokens)
    try:
        for path in uploads:
            _upload_clobber(context, tag, path)
        for item in reverifies:
            try:
                item.run()
            except AssetVerificationError:
                _emit_forensic_alert(
                    context, tag, item.track, _PRESERVE_MESSAGE
                )
                raise
        context.run(f"gh release edit {tag} --draft=false", pty=True)
    except AssetVerificationError:
        raise
    except Exception:
        # Never delete a tag here: by the time this runs, `_publish` has pushed
        # the version bump and the marketplace `source.ref`, so deleting the tag
        # would break fresh installs and `/plugin update` for every user until a
        # correction is pushed. A preserved draft is re-drivable with --clobber,
        # so preserve it and alert for triage rather than tearing it down.
        _emit_forensic_alert(
            context, tag, "Launcher/manifest", _PRESERVE_MESSAGE
        )
        raise
