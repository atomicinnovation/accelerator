from invoke import Context, Exit, task

from tasks.build import _assert_magic_bytes, vendor_shim_marker_digest
from tasks.shared.paths import VENDOR_SHIM_MARKER, vendored_shim_path
from tasks.shared.targets import TARGETS


@task
def check(context: Context) -> None:
    """Guard the committed verify shims against their build inputs.

    The shims are non-reproducible root-of-trust binaries. Fails when a shim is
    missing, non-executable, or the wrong platform, or when the recorded marker
    no longer matches the current `cli/verify` build inputs (a `minisign-verify`
    bump or lockfile change trips this even though it never edits the shims).
    """
    for triple, platform in TARGETS:
        shim = vendored_shim_path(platform)
        if not shim.is_file():
            raise Exit(f"vendored verify shim missing: {shim}", code=1)
        if not shim.stat().st_mode & 0o111:
            raise Exit(
                f"vendored verify shim is not executable: {shim}", code=1
            )
        _assert_magic_bytes(shim, triple)

    if not VENDOR_SHIM_MARKER.is_file():
        raise Exit(
            f"vendor marker missing: {VENDOR_SHIM_MARKER} — run "
            "`mise run build:vendor-verify-shims` and commit",
            code=1,
        )
    recorded = VENDOR_SHIM_MARKER.read_text().strip()
    current = vendor_shim_marker_digest()
    if recorded != current:
        raise Exit(
            "vendored verify shims are stale — the cli/verify build inputs "
            "changed since they were last vendored. Re-run "
            "`mise run build:vendor-verify-shims` and commit the refreshed "
            "shims + marker.",
            code=1,
        )
