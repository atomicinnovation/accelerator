import json
import shlex
import subprocess
import tempfile
from collections.abc import Callable
from functools import partial
from pathlib import Path
from typing import NamedTuple

import semver
from invoke import Context, task

from tasks.shared.errors import InvalidVersionError
from tasks.shared.hashing import compute_sha256
from tasks.shared.paths import (
    CHECKSUMS,
    DISPATCHED_SUBBINARIES,
    RELEASE_MANIFEST,
    RELEASE_MANIFEST_SIG,
    RELEASE_PUBLIC_KEY,
    binary_path,
    cli_binary_path,
    debug_archive_path,
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
    --draft unconditionally so no assets are visible until upload_and_verify
    has verified every binary and published the release.
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


@task
def upload_and_verify(context: Context, version: str) -> None:
    """Upload release artefacts, verify SHA-256, then publish the draft."""
    tag = f"v{version}"
    checksums = json.loads(CHECKSUMS.read_text())
    hashes = {
        platform: digest.removeprefix("sha256:")
        for platform, digest in checksums["binaries"].items()
    }
    binaries = {platform: binary_path(platform) for _, platform in TARGETS}
    archives = {
        platform: debug_archive_path(platform) for _, platform in TARGETS
    }
    missing = [
        p
        for p in list(binaries.values()) + list(archives.values())
        if not p.exists()
    ]
    if missing:
        raise FileNotFoundError(
            f"Expected release artefacts not found: {[str(p) for p in missing]}"
        )
    try:
        for platform, path in binaries.items():
            upload_release_asset(context, tag, path)
            upload_release_asset(context, tag, archives[platform])
        for platform, asset_path in binaries.items():
            download_and_verify(context, tag, asset_path.name, hashes[platform])
        context.run(f"gh release edit {tag} --draft=false", pty=True)
    except AssetVerificationError:
        _emit_forensic_alert(
            context,
            tag,
            "Visualiser",
            "AssetVerificationError — draft + tag PRESERVED for triage",
        )
        raise
    except Exception:
        context.run(
            f"gh release delete {tag} --cleanup-tag --yes",
            warn=True,
            timeout=120,
        )
        raise


# ── unified launcher + manifest + visualiser publish ──────────────────

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


def _release_uploads() -> list[Path]:
    uploads: list[Path] = []
    for _triple, platform in TARGETS:
        uploads.append(binary_path(platform))
        uploads.append(debug_archive_path(platform))
        launcher = cli_binary_path("accelerator", platform)
        uploads.append(launcher)
        uploads.append(_sig(launcher))
    uploads.append(RELEASE_MANIFEST)
    uploads.append(RELEASE_MANIFEST_SIG)
    for name in DISPATCHED_SUBBINARIES:
        for _triple, platform in TARGETS:
            asset = cli_binary_path(name, platform)
            uploads.append(asset)
            uploads.append(_sig(asset))
    return uploads


def _release_reverifies(context: Context, tag: str) -> list[_Reverify]:
    checksums = json.loads(CHECKSUMS.read_text())
    hashes = {
        platform: digest.removeprefix("sha256:")
        for platform, digest in checksums["binaries"].items()
    }
    items: list[_Reverify] = []
    for _triple, platform in TARGETS:
        visualiser = binary_path(platform)
        items.append(
            _Reverify(
                "Visualiser",
                partial(
                    download_and_verify,
                    context,
                    tag,
                    visualiser.name,
                    hashes[platform],
                ),
            )
        )
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
    items.extend(_subbinary_reverifies(context, tag))
    return items


def _subbinary_reverifies(context: Context, tag: str) -> list[_Reverify]:
    if not DISPATCHED_SUBBINARIES:
        return []
    manifest = json.loads(RELEASE_MANIFEST.read_text())
    items: list[_Reverify] = []
    for name in DISPATCHED_SUBBINARIES:
        entry = manifest["binaries"][name]
        for _triple, platform in TARGETS:
            asset = cli_binary_path(name, platform).name
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
    """Upload every asset across both tracks, re-verify, then publish once.

    Owns the single `--draft=false` transition, flipped only after every asset
    (visualiser sha256, launcher shim-minisig, manifest shim-minisig, sub-binary
    sha256 + inline signature) re-verifies. An AssetVerificationError on either
    track preserves the draft with a track-labelled forensic alert; any other
    error deletes the release. Because the delete lives inside this pre-publish
    envelope, it can never run against an already-published release. Uploads are
    `--clobber` so a preserved draft can be re-driven to green without manual
    asset deletion.
    """
    tag = f"v{version}"
    uploads = _release_uploads()
    missing = [p for p in uploads if not p.exists()]
    if missing:
        raise FileNotFoundError(
            f"Expected release artefacts not found: {[str(p) for p in missing]}"
        )
    reverifies = _release_reverifies(context, tag)
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
        context.run(
            f"gh release delete {tag} --cleanup-tag --yes",
            warn=True,
            timeout=120,
        )
        raise
