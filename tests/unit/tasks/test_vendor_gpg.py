"""The GPG status-line classifier over recorded GnuPG --status-fd fixtures.

Every combination is exercised here rather than through the subprocess, so the
predicate is tested without crafting revoked or expired keyrings or depending on
a particular host GnuPG.
"""

from pathlib import Path

from tasks.shared.vendor.gpg import classify_status_lines, verify_detached

# A plausible Node release primary-key fingerprint and a subkey under it.
PRIMARY = "4ED778F539E3634C779C87C6D7062848A1AB005C"
SUBKEY = "141F07595B7B3FFE74309A937405533BE57C7D57"
STRANGER = "0000000000000000000000000000000000000000"
ALLOWED = {PRIMARY}


def _good(primary: str = PRIMARY, signing: str = SUBKEY) -> list[str]:
    """A clean GOODSIG + VALIDSIG pair, the signature made by a subkey."""
    return [
        "[GNUPG:] NEWSIG",
        f"[GNUPG:] GOODSIG {signing[-16:]} Node.js (Release signing) <no@ne>",
        f"[GNUPG:] VALIDSIG {signing} 2026-01-01 1787000000 0 4 0 22 8 00 "
        f"{primary}",
        "[GNUPG:] TRUST_UNDEFINED 0 pgp",
    ]


def test_a_clean_subkey_signature_is_trusted() -> None:
    verdict = classify_status_lines(_good(), ALLOWED)
    assert verdict.trusted, verdict.reason


def test_a_single_key_signature_matches_on_its_own_fingerprint() -> None:
    # A signature made directly by the primary repeats it in the last field.
    lines = _good(primary=PRIMARY, signing=PRIMARY)
    assert classify_status_lines(lines, ALLOWED).trusted


def test_a_revoked_key_is_rejected_even_with_validsig() -> None:
    lines = _good()
    # GnuPG replaces GOODSIG with REVKEYSIG but still emits VALIDSIG.
    lines[1] = f"[GNUPG:] REVKEYSIG {SUBKEY[-16:]} Node.js"
    verdict = classify_status_lines(lines, ALLOWED)
    assert not verdict.trusted
    assert "revoked" in verdict.reason


def test_an_expired_key_is_accepted() -> None:
    # An expired key signs nothing new, but the signatures it made while valid
    # stand; GnuPG emits EXPKEYSIG in place of GOODSIG and still emits VALIDSIG.
    lines = _good()
    lines[1] = f"[GNUPG:] EXPKEYSIG {SUBKEY[-16:]} Node.js"
    verdict = classify_status_lines(lines, ALLOWED)
    assert verdict.trusted, verdict.reason


def test_an_expired_key_off_the_allowlist_is_still_rejected() -> None:
    # Accepting EXPKEYSIG must not bypass the fingerprint allowlist.
    lines = _good(primary=STRANGER, signing=STRANGER)
    lines[1] = f"[GNUPG:] EXPKEYSIG {STRANGER[-16:]} Node.js"
    assert not classify_status_lines(lines, ALLOWED).trusted


def test_an_expired_signature_is_rejected() -> None:
    lines = _good()
    lines[1] = f"[GNUPG:] EXPSIG {SUBKEY[-16:]} Node.js"
    assert not classify_status_lines(lines, ALLOWED).trusted


def test_a_missing_public_key_is_rejected() -> None:
    lines = [
        "[GNUPG:] NEWSIG",
        f"[GNUPG:] NO_PUBKEY {SUBKEY[-16:]}",
    ]
    verdict = classify_status_lines(lines, ALLOWED)
    assert not verdict.trusted
    assert "keyring" in verdict.reason


def test_a_subkey_whose_primary_is_not_allowlisted_is_rejected() -> None:
    # A valid signature by a subkey whose primary differs from the allowlist —
    # the case a signing-subkey-only check would wrongly accept.
    lines = _good(primary=STRANGER, signing=SUBKEY)
    verdict = classify_status_lines(lines, ALLOWED)
    assert not verdict.trusted
    assert "allowlist" in verdict.reason


def test_a_good_signature_with_no_validsig_is_rejected() -> None:
    lines = [
        "[GNUPG:] GOODSIG " + SUBKEY[-16:] + " Node.js",
    ]
    assert not classify_status_lines(lines, ALLOWED).trusted


def test_a_bad_signature_is_rejected() -> None:
    lines = [f"[GNUPG:] BADSIG {SUBKEY[-16:]} Node.js"]
    assert not classify_status_lines(lines, ALLOWED).trusted


def test_non_status_lines_are_ignored() -> None:
    noise = ["gpg: Signature made ...", "gpg: Good signature from ..."]
    assert classify_status_lines(noise + _good(), ALLOWED).trusted


def test_verify_detached_fails_closed_when_gpg_is_absent() -> None:
    verdict = verify_detached(
        Path("sig"),
        Path("target"),
        Path("keyring"),
        ALLOWED,
        runner=lambda _s, _t, _k: None,
    )
    assert not verdict.trusted
    assert "gpg is not available" in verdict.reason


def test_verify_detached_feeds_the_runner_output_to_the_classifier() -> None:
    verdict = verify_detached(
        Path("sig"),
        Path("target"),
        Path("keyring"),
        ALLOWED,
        runner=lambda _s, _t, _k: _good(),
    )
    assert verdict.trusted
