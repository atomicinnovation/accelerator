"""playwright-core provenance: registry signature, integrity binding, SLSA.

Three checks, each failing the release rather than a user's run. npm's registry
signature covers the packument metadata string ``<name>@<version>:<integrity>``,
not the tarball — so verifying it proves only that the registry asserted an
integrity value. The check is completed by recomputing the tarball's sha512 and
comparing it against the ``integrity`` inside the *signed* message; without that
second step the npm guarantee collapses onto SLSA alone. The SLSA provenance
check is only as strong as its predicate, so the expected owner, repository and
workflow identity are asserted explicitly and the exact ``gh`` argv is pinned by
a test.

The registry key validating the signature is committed as
``keys/npm-registry.pem`` rather than fetched — fetching it over the channel it
validates would reproduce the problem one level up. Populating it is a
trust-anchor operation.
"""

from __future__ import annotations

import base64
import hashlib
import subprocess
from collections.abc import Callable
from pathlib import Path

from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec

# Injected in tests so the argv is pinned without a real `gh`; production passes
# a subprocess runner. Returns the process exit code.
SlsaRunner = Callable[[list[str]], int]

_CHUNK = 64 * 1024


def signed_message(name: str, version: str, integrity: str) -> str:
    """Return the packument string npm's registry signs for a version."""
    return f"{name}@{version}:{integrity}"


def verify_registry_signature(
    *, message: str, signature_b64: str, public_key_pem: bytes
) -> None:
    """Verify npm's ECDSA P-256 signature over ``message``.

    Raises unless the base64 signature verifies under the committed registry
    public key. This binds the packument metadata; the tarball itself is bound
    by :func:`assert_integrity_binds_tarball`.
    """
    key = serialization.load_pem_public_key(public_key_pem)
    if not isinstance(key, ec.EllipticCurvePublicKey):
        raise TypeError("npm registry key is not an EC public key")
    try:
        key.verify(
            base64.b64decode(signature_b64),
            message.encode(),
            ec.ECDSA(hashes.SHA256()),
        )
    except InvalidSignature as exc:
        raise ValueError("npm registry signature does not verify") from exc


def assert_integrity_binds_tarball(*, integrity: str, tarball: Path) -> None:
    """Fail unless ``tarball``'s sha512 equals the signed ``integrity`` value.

    ``integrity`` is npm's ``sha512-<base64>`` form; anything else is refused
    rather than silently accepted as a weaker digest.
    """
    algorithm, _, encoded = integrity.partition("-")
    if algorithm != "sha512":
        raise ValueError(
            f"npm integrity is not sha512: {integrity.split('-', 1)[0]!r}"
        )
    expected = base64.b64decode(encoded)
    if _sha512_file(tarball) != expected:
        raise ValueError(
            "tarball sha512 does not match the registry-signed integrity"
        )


def verify_slsa(
    *,
    tarball: Path,
    owner: str,
    repo: str,
    signer_workflow: str,
    runner: SlsaRunner | None = None,
) -> None:
    """Fail unless ``tarball`` carries SLSA provenance from the pinned builder.

    ``gh attestation verify`` derives and matches the subject digest from the
    tarball path, so the identity predicates (owner, repo, workflow) are what
    stop it accepting an attestation from any builder.
    """
    run = runner or _run_gh
    argv = [
        "gh",
        "attestation",
        "verify",
        str(tarball),
        "--owner",
        owner,
        "--repo",
        repo,
        "--signer-workflow",
        signer_workflow,
    ]
    if run(argv) != 0:
        raise ValueError("SLSA provenance verification failed")


def _run_gh(argv: list[str]) -> int:
    return subprocess.run(argv, check=False, capture_output=True).returncode


def _sha512_file(path: Path) -> bytes:
    hasher = hashlib.sha512()
    with path.open("rb") as handle:
        while chunk := handle.read(_CHUNK):
            hasher.update(chunk)
    return hasher.digest()
