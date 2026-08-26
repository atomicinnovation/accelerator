"""GPG detached-signature verification for the vendored Node runtime.

The predicate is a pure function over GnuPG ``--status-fd`` lines, kept separate
from invoking ``gpg`` so every combination is a table-driven unit test over
recorded fixture output rather than a crafted keyring on a particular host.

The subtlety this exists to get right: ``gpg``'s exit code is 0 for a
well-formed signature from a key merely present in the keyring, and GnuPG emits
``VALIDSIG`` for signatures made by expired and revoked keys too — those replace
``GOODSIG`` with ``EXPKEYSIG``/``REVKEYSIG`` rather than suppressing
``VALIDSIG``. So a ``VALIDSIG``-plus-fingerprint check alone would accept a
manifest signed by a since-revoked Node release key, the case where rotation
matters most. The predicate therefore refuses ``REVKEYSIG`` explicitly and
matches the allowlist against ``VALIDSIG``'s *primary-key* fingerprint rather
than the signing subkey's. It accepts ``EXPKEYSIG`` — an expired key made no new
signature, but the ones it made while valid stand, and Node's release keys
expire faster than the upstream repo extends them.
"""

import shutil
import subprocess
import tempfile
from collections.abc import Callable, Iterable
from dataclasses import dataclass
from pathlib import Path
from typing import NamedTuple, Self

_STATUS_PREFIX = "[GNUPG:] "


@dataclass(frozen=True, slots=True)
class Verdict:
    """Whether the signature is trusted, and why not when it is not."""

    trusted: bool
    reason: str = ""

    @classmethod
    def ok(cls) -> Self:
        return cls(trusted=True)

    @classmethod
    def rejected(cls, reason: str) -> Self:
        return cls(trusted=False, reason=reason)


class Status(NamedTuple):
    """One parsed GnuPG ``--status-fd`` line: its keyword and remaining args."""

    keyword: str
    args: list[str]


def classify_status_lines(
    lines: Iterable[str], allowed_fingerprints: Iterable[str]
) -> Verdict:
    """Classify GnuPG ``--status-fd`` output against a fingerprint allowlist.

    Trusted iff a ``GOODSIG`` or ``EXPKEYSIG`` is present, no revoked/expired-
    signature or missing-key status appears, and a ``VALIDSIG`` names a primary-
    key fingerprint in the allowlist. Every other shape is a named rejection.
    """
    allowed = {fingerprint.upper() for fingerprint in allowed_fingerprints}
    statuses: list[Status] = []
    for line in lines:
        if not line.startswith(_STATUS_PREFIX):
            continue
        parts = line[len(_STATUS_PREFIX) :].split()
        if parts:
            statuses.append(Status(parts[0], parts[1:]))

    keywords = {keyword for keyword, _ in statuses}

    # The hard failures and actively-distrusted variants, each named so a
    # reviewer of a failed release sees which one fired. EXPKEYSIG is not here:
    # an expired key signs nothing new, but the signatures it made while valid
    # stand — Node's release keys expire and the upstream repo lags in extending
    # them, so rejecting EXPKEYSIG would refuse legitimately-signed releases.
    # REVKEYSIG (a revoked, actively-distrusted key) is refused.
    for keyword, reason in (
        ("REVKEYSIG", "the signing key has been revoked"),
        ("EXPSIG", "the signature has expired"),
        ("BADSIG", "the signature does not verify"),
        ("ERRSIG", "the signature could not be checked"),
        ("NO_PUBKEY", "the signing key is not in the committed keyring"),
    ):
        if keyword in keywords:
            return Verdict.rejected(reason)

    # GnuPG emits GOODSIG for a signature from a currently-valid key, EXPKEYSIG
    # (in its place) when that key has since expired; either is a signature made
    # by a key valid at signing time.
    if keywords.isdisjoint({"GOODSIG", "EXPKEYSIG"}):
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
type Runner = Callable[[Path, Path, Path], list[str] | None]


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
    # `--keyring` reads a binary keyring, not the committed armored `.asc`, and
    # the host's default keyring must never be consulted. So import the anchor
    # into an ephemeral home directory and verify there — isolated from the host
    # keys, and reading the armored file through the only path that parses it.
    with tempfile.TemporaryDirectory() as home:
        subprocess.run(
            [
                "gpg",
                "--homedir",
                home,
                "--batch",
                "--quiet",
                "--import",
                str(keyring),
            ],
            check=False,
            capture_output=True,
            text=True,
            timeout=60,
        )
        result = subprocess.run(
            [
                "gpg",
                "--homedir",
                home,
                "--batch",
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
