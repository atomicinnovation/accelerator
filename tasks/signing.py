import json
import os
import stat
import subprocess
import tempfile
from collections.abc import Iterable, Iterator, Mapping, Sequence
from contextlib import contextmanager
from pathlib import Path

from invoke import Context, task

from tasks.shared.errors import SigningError
from tasks.shared.paths import (
    DISPATCHED_SUBBINARIES,
    RELEASE_PUBLIC_KEY,
    RELEASE_SECRET_KEY,
    RELEASE_STAGING,
    TREE_ARTIFACTS,
    cli_binary_path,
    subbinary_asset_path,
    tree_artifact_asset_path,
)
from tasks.shared.targets import TARGETS
from tasks.shared.vendor.archive import read_archive_stats

SECRET_KEY_ENV = "ACCELERATOR_RELEASE_SECRET_KEY"  # noqa: S105 — env var name, not a secret


def sign_file(secret_key: Path, target: Path, signature: Path) -> Path:
    result = subprocess.run(
        [
            "minisign",
            "-S",
            "-s",
            str(secret_key),
            "-x",
            str(signature),
            "-m",
            str(target),
        ],
        check=False,
        capture_output=True,
        text=True,
        timeout=120,
    )
    if result.returncode != 0:
        raise SigningError(f"{target.name}: {result.stderr.strip()}")
    return signature


def _signature_path(binary: Path) -> Path:
    return binary.with_name(binary.name + ".minisig")


def _subbinary_signing_targets(
    tokens: Iterable[str] = DISPATCHED_SUBBINARIES,
) -> list[Path]:
    return [
        subbinary_asset_path(token, platform)
        for _triple, platform in TARGETS
        for token in tokens
    ]


def sign_staged_binaries(secret_key: Path) -> None:
    """Sign the launcher + every dispatched sub-binary across all four targets.

    Signs an explicit expected set — never a directory scan — so a partial
    cross-compile fails closed rather than silently signing a subset. The
    `accelerator-verify-{platform}` shims that Phase 2 also stages here are
    deliberately excluded: they ship committed in bin/, never as release assets.
    """
    expected = [
        cli_binary_path("accelerator", platform)
        for _triple, platform in TARGETS
    ]
    expected += _subbinary_signing_targets()
    missing = [binary for binary in expected if not binary.exists()]
    if missing:
        raise SigningError(
            f"expected staged binaries not found: {[str(p) for p in missing]}"
        )
    for binary in expected:
        sign_file(secret_key, binary, _signature_path(binary))


def _tree_artifact_signing_targets(
    tokens: Iterable[str] = TREE_ARTIFACTS,
    staging_dir: Path = RELEASE_STAGING,
    platforms: Sequence[tuple[str, str]] = TARGETS,
) -> list[Path]:
    return [
        tree_artifact_asset_path(name, platform, staging_dir)
        for _triple, platform in platforms
        for name in tokens
    ]


def _reverify_sealed(archive: Path) -> Path:
    """Re-derive the attestation from the archive, refusing any disagreement.

    Returns the `.sealed` path so the caller signs the document it verified,
    never one that could have been swapped between the check and the sign.
    """
    sealed = archive.with_name(archive.name + ".sealed")
    if not sealed.exists():
        raise SigningError(f"{archive.name}: missing .sealed attestation")
    document: Mapping[str, object] = json.loads(sealed.read_text())
    stats = read_archive_stats(archive)
    for field, actual in (
        ("archive_sha256", stats.archive_sha256),
        ("uncompressed_size", stats.uncompressed_size),
        ("entry_count", stats.entry_count),
        ("table_sha256", stats.table_sha256),
    ):
        if document.get(field) != actual:
            raise SigningError(
                f"{archive.name}: .sealed {field} disagrees with the archive"
            )
    return sealed


def sign_tree_artifacts(
    secret_key: Path,
    *,
    tokens: Iterable[str] = TREE_ARTIFACTS,
    staging_dir: Path = RELEASE_STAGING,
    platforms: Sequence[tuple[str, str]] = TARGETS,
) -> None:
    """Sign each tree archive and its re-verified `.sealed` attestation.

    Checks the whole expected set exists first, so a partial assembly fails
    closed rather than signing a subset, then signs the archive to `.minisig`
    and the attestation to `.sealed.sig`.
    """
    archives = _tree_artifact_signing_targets(tokens, staging_dir, platforms)
    missing = [archive for archive in archives if not archive.exists()]
    if missing:
        raise SigningError(
            f"expected tree archives not found: {[str(p) for p in missing]}"
        )
    for archive in archives:
        sealed = _reverify_sealed(archive)
        sign_file(secret_key, archive, _signature_path(archive))
        sign_file(secret_key, sealed, sealed.with_name(sealed.name + ".sig"))


@contextmanager
def resolve_secret_key(
    dev_key: Path = RELEASE_SECRET_KEY,
) -> Iterator[Path]:
    """Yield a usable secret-key path for the duration of a signing batch.

    With ACCELERATOR_RELEASE_SECRET_KEY set, materialises it to a mode-0600 file
    inside a TemporaryDirectory unlinked when the block exits (including on the
    exception path); otherwise yields the local dev key. An unset env var with
    no dev key fails closed rather than yielding a non-existent path.
    """
    materialised = os.environ.get(SECRET_KEY_ENV)
    if materialised:
        with tempfile.TemporaryDirectory() as tmpdir:
            key_path = Path(tmpdir) / "release.sec"
            key_path.write_text(materialised)
            key_path.chmod(stat.S_IRUSR | stat.S_IWUSR)
            yield key_path
        return
    if not dev_key.exists():
        raise SigningError(
            f"{SECRET_KEY_ENV} is not set and no dev key exists at "
            f"{dev_key} — cannot sign (see the key lifecycle in RELEASING.md)"
        )
    yield dev_key


@task
def generate(
    context: Context,
    pub_path: str = str(RELEASE_PUBLIC_KEY),
    sec_path: str = str(RELEASE_SECRET_KEY),
) -> None:
    """Generate a -W release signing keypair non-interactively.

    Does not print the secret: provision it straight from the written .sec file
    (e.g. piped into `gh secret set` without echoing) so it never lands in
    terminal scrollback or shell history.
    """
    pub = Path(pub_path)
    sec = Path(sec_path)
    result = subprocess.run(
        ["minisign", "-G", "-W", "-f", "-p", str(pub), "-s", str(sec)],
        check=False,
        capture_output=True,
        text=True,
        timeout=120,
    )
    if result.returncode != 0:
        raise SigningError(f"key generation failed: {result.stderr.strip()}")
    print(
        f"Generated release signing keypair:\n"
        f"  public: {pub}\n"
        f"  secret: {sec}\n\n"
        f"Provision the secret WITHOUT echoing it, e.g.:\n"
        f"  gh secret set {SECRET_KEY_ENV} < {sec}\n\n"
        f"Then commit the public half and ship a launcher built from that HEAD "
        f"before signing any release with the matching secret."
    )
