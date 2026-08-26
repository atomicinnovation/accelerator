"""Verify the vendored Node runtime against its GPG-signed checksums.

Node publishes ``SHASUMS256.txt`` and a detached ``SHASUMS256.txt.sig`` (the
``.asc`` beside it is a clearsigned copy, not a detached signature).
Verification is: the signature is good under the committed keyring (delegated to
:mod:`tasks.shared.vendor.gpg`, which rejects revoked keys that still emit
``VALIDSIG``), and the fetched tarball's sha256 equals the digest its
``SHASUMS256.txt`` line records. The digest is matched by **exact filename**,
not by searching the file for a digest — a search would accept a line describing
a different artifact listed in the same signed manifest.

The trust anchor is the committed keyring, not anything fetched over the channel
being verified. Populating ``keys/nodejs-release.asc`` and
``NODE_RELEASE_FINGERPRINTS`` is a trust-anchor operation gated by
second-person review, under the refresh procedure recorded in RELEASING.md.
"""

import hashlib
from collections.abc import Iterable
from pathlib import Path

from tasks.shared.vendor import gpg

# The primary-key fingerprints of the Node release keys, checked against the
# committed keyring by a build-system consistency test. Filled under the
# trust-anchor refresh procedure, never in a routine version bump. These are the
# active release signers from the nodejs/release-keys README; verify each
# out-of-band against nodejs.org before trusting.
NODE_RELEASE_FINGERPRINTS: tuple[str, ...] = (
    "108F52B48DB57BB0CC439B2997B01419BD92F80A",
    "5BE8A3F6C8A5C01D106C0AD820B1A390B168D356",
    "655F3B5C1FB3FA8D1A0CA6BDE4A7D232B936D2FD",
    "890C08DB8579162FEE0DF9DB8BEAB4DFCF555EF4",
    "8FCCA13FEF1D0C2E91008E09770F7A9A5AE15600",
    "A363A499291CBBC940DD62E41F10027AF002F8B0",
    "C82FA3AE1CBEDC6BE46B9360C43CEC45C17AB93C",
    "CC68F5A3106FF448322E48ED27F5E38D5B0A215F",
    "DD792F5973C6DE52C432CBDAC77ABFA00DDBF2B7",
)

_CHUNK = 64 * 1024


def digest_for_filename(shasums_text: str, filename: str) -> str:
    """Return the sha256 ``SHASUMS256.txt`` records for exactly ``filename``."""
    for line in shasums_text.splitlines():
        parts = line.split()
        if len(parts) == 2 and parts[1] == filename:
            return parts[0]
    raise ValueError(f"{filename} is absent from SHASUMS256.txt")


def verify_node_runtime(
    *,
    tarball: Path,
    filename: str,
    shasums: Path,
    signature: Path,
    keyring: Path,
    fingerprints: Iterable[str],
    runner: gpg.Runner | None = None,
) -> None:
    """Fail the release unless the Node tarball is signed and matches.

    ``runner`` is injected in tests so the wiring is exercised without a host
    GnuPG; production passes ``None`` and the committed keyring is used.
    """
    verdict = gpg.verify_detached(
        signature, shasums, keyring, fingerprints, runner=runner
    )
    if not verdict.trusted:
        raise ValueError(f"SHASUMS256.txt signature rejected: {verdict.reason}")
    expected = digest_for_filename(shasums.read_text(), filename)
    actual = _sha256_file(tarball)
    if actual != expected:
        raise ValueError(f"{filename}: sha256 {actual} != signed {expected}")


def _sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as handle:
        while chunk := handle.read(_CHUNK):
            hasher.update(chunk)
    return hasher.hexdigest()
