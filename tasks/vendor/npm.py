"""playwright-core provenance: registry signature, integrity binding, SLSA.

Three checks, each failing the release rather than a user's run. npm's registry
signature covers the packument metadata string ``<name>@<version>:<integrity>``,
not the tarball — so verifying it proves only that the registry asserted an
integrity value. The check is completed by recomputing the tarball's sha512 and
comparing it against the ``integrity`` inside the *signed* message; without that
second step the npm guarantee collapses onto SLSA alone.

The SLSA provenance is npm's own: a Sigstore bundle published to the registry
(``dist.attestations.url``), whose subject is keyed by the tarball sha512 and a
``pkg:npm`` PURL — not by the sha256 a GitHub-stored attestation would carry. So
``gh attestation verify`` cannot verify it (it matches subjects by the file's
sha256, and queries GitHub's attestation store, which has no such record); the
bundle is instead verified with ``cosign verify-blob-attestation``, whose
certificate-identity and OIDC-issuer predicates pin it to the builder's GitHub
Actions workflow. The exact ``cosign`` argv is pinned by a test.

The registry key validating the signature is committed as
``keys/npm-registry.pem`` rather than fetched — fetching it over the channel it
validates would reproduce the problem one level up. Populating it is a
trust-anchor operation.
"""

import base64
import hashlib
import subprocess
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec

# Injected in tests so the argv is pinned without a real `cosign`; production
# passes a subprocess runner. Returns the process exit code.
SlsaRunner = Callable[[list[str]], int]

# The SLSA provenance predicate npm's attestation bundle carries.
SLSA_PREDICATE_TYPE = "https://slsa.dev/provenance/v1"

_CHUNK = 64 * 1024


@dataclass(frozen=True)
class DistInfo:
    """The fields of a packument version's ``dist`` the checks consume."""

    tarball: str
    integrity: str
    signature_b64: str
    attestations_url: str


def packument_dist(packument: dict[str, Any], version: str) -> DistInfo:
    """Extract one version's tarball URL, integrity, signature and attestations.

    Refuses a version carrying no registry signature or no attestations URL
    rather than silently falling through to a weaker set of checks.
    """
    versions = packument.get("versions", {})
    if version not in versions:
        raise ValueError(f"{version} is absent from the packument")
    dist = versions[version]["dist"]
    signatures = dist.get("signatures", [])
    if not signatures:
        raise ValueError(f"{version} carries no registry signature")
    attestations_url = dist.get("attestations", {}).get("url")
    if not attestations_url:
        raise ValueError(f"{version} carries no provenance attestations")
    return DistInfo(
        tarball=dist["tarball"],
        integrity=dist["integrity"],
        signature_b64=signatures[0]["sig"],
        attestations_url=attestations_url,
    )


def provenance_bundle(attestations: dict[str, Any]) -> dict[str, Any]:
    """Return the SLSA provenance Sigstore bundle from npm's attestations doc.

    Refuses a document with no SLSA provenance rather than accepting the npm
    publish attestation alone.
    """
    for attestation in attestations.get("attestations", []):
        if attestation.get("predicateType") == SLSA_PREDICATE_TYPE:
            return attestation["bundle"]
    raise ValueError("no SLSA provenance attestation in the npm bundle")


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
    bundle: Path,
    identity_regexp: str,
    oidc_issuer: str,
    predicate_type: str = SLSA_PREDICATE_TYPE,
    runner: SlsaRunner | None = None,
) -> None:
    """Fail unless ``bundle`` is valid SLSA provenance for ``tarball``.

    ``cosign verify-blob-attestation`` checks the Sigstore bundle's DSSE
    signature and transparency-log inclusion, binds the subject digest to the
    tarball, and — via ``identity_regexp``/``oidc_issuer`` — pins the signing
    certificate to the builder's GitHub Actions workflow, so an attestation from
    any other identity is rejected.
    """
    run = runner or _run_cosign
    argv = [
        "cosign",
        "verify-blob-attestation",
        "--bundle",
        str(bundle),
        "--new-bundle-format",
        "--certificate-identity-regexp",
        identity_regexp,
        "--certificate-oidc-issuer",
        oidc_issuer,
        "--type",
        predicate_type,
        str(tarball),
    ]
    if run(argv) != 0:
        raise ValueError("SLSA provenance verification failed")


def _run_cosign(argv: list[str]) -> int:
    return subprocess.run(argv, check=False, capture_output=True).returncode


def _sha512_file(path: Path) -> bytes:
    hasher = hashlib.sha512()
    with path.open("rb") as handle:
        while chunk := handle.read(_CHUNK):
            hasher.update(chunk)
    return hasher.digest()
