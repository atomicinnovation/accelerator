"""GPG detached-signature verification for the vendored Node runtime.

The predicate is a pure function over GnuPG ``--status-fd`` lines, kept separate
from invoking ``gpg`` so every combination is a table-driven unit test over
recorded fixture output rather than a crafted keyring on a particular host.

The subtlety this exists to get right: ``gpg``'s exit code is 0 for a
well-formed signature from a key merely present in the keyring, and GnuPG emits
``VALIDSIG`` for signatures made by expired and revoked keys too — those replace
``GOODSIG`` with ``EXPKEYSIG``/``REVKEYSIG`` rather than suppressing
``VALIDSIG``. So a ``VALIDSIG``-plus-fingerprint check alone would accept a
manifest signed by a since-revoked Node release key, the single case where
rotation matters most. The predicate therefore requires ``GOODSIG``, rejects the
degraded variants explicitly, and matches the allowlist against ``VALIDSIG``'s
*primary-key* fingerprint rather than the signing subkey's.
"""

from __future__ import annotations

import shutil
import subprocess
from collections.abc import Callable, Iterable
from dataclasses import dataclass
from pathlib import Path

_STATUS_PREFIX = "[GNUPG:] "


@dataclass(frozen=True)
class Verdict:
    """Whether the signature is trusted, and why not when it is not."""

    trusted: bool
    reason: str = ""

    @classmethod
    def ok(cls) -> Verdict:
        return cls(trusted=True)

    @classmethod
    def rejected(cls, reason: str) -> Verdict:
        return cls(trusted=False, reason=reason)


def classify_status_lines(
    lines: Iterable[str], allowed_fingerprints: Iterable[str]
) -> Verdict:
    """Classify GnuPG ``--status-fd`` output against a fingerprint allowlist.

    Trusted iff a ``GOODSIG`` is present, no degraded or missing-key status
    appears, and a ``VALIDSIG`` names a primary-key fingerprint in the
    allowlist. Every other shape is a named rejection.
    """
    allowed = {fingerprint.upper() for fingerprint in allowed_fingerprints}
    statuses: list[tuple[str, list[str]]] = []
    for line in lines:
        if not line.startswith(_STATUS_PREFIX):
            continue
        parts = line[len(_STATUS_PREFIX) :].split()
        if parts:
            statuses.append((parts[0], parts[1:]))

    keywords = {keyword for keyword, _ in statuses}

    # The degraded good-signature variants and hard failures, each named so a
    # reviewer of a failed release sees which one fired.
    for keyword, reason in (
        ("REVKEYSIG", "the signing key has been revoked"),
        ("EXPKEYSIG", "the signing key has expired"),
        ("EXPSIG", "the signature has expired"),
        ("BADSIG", "the signature does not verify"),
        ("ERRSIG", "the signature could not be checked"),
        ("NO_PUBKEY", "the signing key is not in the committed keyring"),
    ):
        if keyword in keywords:
            return Verdict.rejected(reason)

    if "GOODSIG" not in keywords:
        return Verdict.rejected("no good signature was produced")

    primaries = [
        _primary_fingerprint(args)
        for keyword, args in statuses
        if keyword == "VALIDSIG"
    ]
    if not primaries:
        return Verdict.rejected("no VALIDSIG line was produced")
    if not any(primary in allowed for primary in primaries if primary):
        return Verdict.rejected(
            "the signature's primary-key fingerprint is not allowlisted"
        )
    return Verdict.ok()


# The status-fd producer, injected in tests so the classifier is exercised
# without a real gpg or keyring.
Runner = Callable[[Path, Path, Path], "list[str] | None"]


def verify_detached(
    signature: Path,
    target: Path,
    keyring: Path,
    allowed_fingerprints: Iterable[str],
    runner: Runner | None = None,
) -> Verdict:
    """Verify ``target`` against its detached ``signature`` under ``keyring``.

    Runs ``gpg`` against the committed keyring alone — never the host's
    default keyring — with ``--status-fd`` parsed by ``classify_status_lines``.
    An absent ``gpg`` is a hard failure, not a skip: the release depends on it.
    """
    run = runner or _run_gpg
    status = run(signature, target, keyring)
    if status is None:
        return Verdict.rejected("gpg is not available to verify the signature")
    return classify_status_lines(status, allowed_fingerprints)


def _run_gpg(signature: Path, target: Path, keyring: Path) -> list[str] | None:
    if shutil.which("gpg") is None:
        return None
    result = subprocess.run(
        [
            "gpg",
            "--no-default-keyring",
            "--keyring",
            str(keyring),
            "--status-fd",
            "1",
            "--verify",
            str(signature),
            str(target),
        ],
        check=False,
        capture_output=True,
        text=True,
        timeout=60,
    )
    return result.stdout.splitlines()


def _primary_fingerprint(args: list[str]) -> str | None:
    """Return the primary-key fingerprint from a ``VALIDSIG`` line.

    GnuPG's ``VALIDSIG`` carries the primary-key fingerprint as its tenth field
    when the signature was made by a subkey; a single-key signature repeats the
    signing fingerprint there. The first field is always the signing key's, so
    matching the allowlist against the last field is what refuses a subkey whose
    primary differs from an allowlisted one.
    """
    if not args:
        return None
    return (args[9] if len(args) >= 10 else args[0]).upper()
