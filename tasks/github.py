import json
import shlex
import subprocess
import tempfile
from pathlib import Path

from invoke import Context, task

from tasks.shared.paths import CHECKSUMS, binary_path, debug_archive_path
from tasks.shared.releases import compute_sha256, is_prerelease_version
from tasks.shared.targets import TARGETS


class AssetVerificationError(Exception):
    pass


@task
def check_auth(context: Context):
    """Verify the GitHub CLI is authenticated."""
    result = context.run("gh auth status", warn=True, hide=True)
    if result.return_code != 0:
        raise RuntimeError(
            "gh auth status failed — run 'gh auth login' or set GH_TOKEN"
        )


@task
def create_release(context: Context, target_version: str | None = None):
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
    cmd = ["gh", "release", "create", tag,
           "--draft", "--generate-notes", "--title", tag]
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
        ["gh", "release", "download", tag, asset_name,
         "--output", str(output_path), "--clobber"],
        capture_output=True, text=True, timeout=120,
    )
    if result.returncode != 0:
        raise AssetVerificationError(
            f"gh release download failed: {result.stderr.strip()}"
        )


@task
def verify_release_asset(context: Context, path: Path, expected_hex: str) -> None:
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
    """Upload all release artefacts, verify their SHA-256, then publish the draft."""
    tag = f"v{version}"
    checksums = json.loads(CHECKSUMS.read_text())
    hashes = {
        platform: digest.removeprefix("sha256:")
        for platform, digest in checksums["binaries"].items()
    }
    binaries = {platform: binary_path(platform) for _, platform in TARGETS}
    archives = {platform: debug_archive_path(platform) for _, platform in TARGETS}
    missing = [
        p for p in list(binaries.values()) + list(archives.values())
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
            context, tag,
            "AssetVerificationError — draft + tag PRESERVED for triage",
        )
        raise
    except Exception:
        context.run(
            f"gh release delete {tag} --cleanup-tag --yes",
            warn=True, timeout=120,
        )
        raise


def _emit_forensic_alert(context: Context, tag: str, message: str) -> None:
    print(f"::error title=Visualiser release {tag}::{message}", flush=True)
