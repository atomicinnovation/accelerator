"""Node runtime verification: GPG-signed checksums plus an exact digest match.

The GPG predicate itself is tested in test_vendor_gpg; these cover the pieces
nodejs.py adds — matching the tarball's digest against its SHASUMS256.txt line
by exact filename (never searching the file for a digest), and failing closed
on a rejected signature or a digest mismatch. The gpg runner is injected, so no
host GnuPG or crafted keyring is needed.
"""

import hashlib

import pytest

from tasks.shared.vendor.nodejs import digest_for_filename, verify_node_runtime

_FINGERPRINT = "AAAABBBBCCCCDDDDEEEEFFFF0000111122223333"


def _good_runner(_signature, _target, _keyring):
    return [
        "[GNUPG:] GOODSIG DEADBEEF Node.js Release",
        "[GNUPG:] VALIDSIG "
        + _FINGERPRINT
        + " 2024-01-01 0 4 0 22 8 00 "
        + _FINGERPRINT,
    ]


def _rejecting_runner(_signature, _target, _keyring):
    return [
        "[GNUPG:] REVKEYSIG DEADBEEF Node.js Release",
        "[GNUPG:] VALIDSIG "
        + _FINGERPRINT
        + " 2024-01-01 0 4 0 22 8 00 "
        + _FINGERPRINT,
    ]


def _shasums(path, filename, digest):
    other = "ff" * 32
    path.write_text(
        f"{other}  node-vOTHER-linux-x64.tar.xz\n{digest}  {filename}\n"
    )
    return path


def test_the_digest_is_matched_by_exact_filename(tmp_path):
    text = (
        "aa" * 32
        + "  node-v20-linux-x64.tar.xz\n"
        + "bb" * 32
        + ("  node-v20-darwin-arm64.tar.gz\n")
    )
    assert (
        digest_for_filename(text, "node-v20-darwin-arm64.tar.gz") == "bb" * 32
    )


def test_a_filename_absent_from_the_manifest_raises(tmp_path):
    with pytest.raises(ValueError, match="absent"):
        digest_for_filename(
            "aa" * 32 + "  node-other.tar.xz\n", "node-x.tar.xz"
        )


def test_a_matching_signature_and_digest_pass(tmp_path):
    data = b"node tarball bytes"
    tarball = tmp_path / "node.tar.xz"
    tarball.write_bytes(data)
    filename = "node-v20-linux-x64.tar.xz"
    shasums = _shasums(
        tmp_path / "SHASUMS256.txt", filename, hashlib.sha256(data).hexdigest()
    )
    verify_node_runtime(
        tarball=tarball,
        filename=filename,
        shasums=shasums,
        signature=tmp_path / "SHASUMS256.txt.asc",
        keyring=tmp_path / "keyring.gpg",
        fingerprints=(_FINGERPRINT,),
        runner=_good_runner,
    )


def test_a_rejected_signature_fails_the_release(tmp_path):
    data = b"node tarball bytes"
    tarball = tmp_path / "node.tar.xz"
    tarball.write_bytes(data)
    filename = "node-v20-linux-x64.tar.xz"
    shasums = _shasums(
        tmp_path / "SHASUMS256.txt", filename, hashlib.sha256(data).hexdigest()
    )
    with pytest.raises(ValueError, match="signature"):
        verify_node_runtime(
            tarball=tarball,
            filename=filename,
            shasums=shasums,
            signature=tmp_path / "sig.asc",
            keyring=tmp_path / "keyring.gpg",
            fingerprints=(_FINGERPRINT,),
            runner=_rejecting_runner,
        )


def test_a_digest_mismatch_fails_the_release(tmp_path):
    tarball = tmp_path / "node.tar.xz"
    tarball.write_bytes(b"tampered tarball")
    filename = "node-v20-linux-x64.tar.xz"
    shasums = _shasums(tmp_path / "SHASUMS256.txt", filename, "cc" * 32)
    with pytest.raises(ValueError, match="sha256"):
        verify_node_runtime(
            tarball=tarball,
            filename=filename,
            shasums=shasums,
            signature=tmp_path / "sig.asc",
            keyring=tmp_path / "keyring.gpg",
            fingerprints=(_FINGERPRINT,),
            runner=_good_runner,
        )
