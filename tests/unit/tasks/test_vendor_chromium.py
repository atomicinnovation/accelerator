"""Chromium is pinned, not verified — the committed digest is the anchor.

There is no publisher signature to check, so these assert the committed
per-platform byte digest is enforced and a mismatch fails the release.
"""

import hashlib

import pytest

from tasks.shared.vendor.chromium import assert_chromium_bytes


def _pins(path, sha256):
    path.write_text(f'[chromium.sha256]\nlinux-x64 = "{sha256}"\n')
    return path


def _archive(path, data):
    path.write_bytes(data)
    return path


def test_bytes_matching_the_pinned_digest_pass(tmp_path):
    data = b"chromium zip bytes"
    assert_chromium_bytes(
        _archive(tmp_path / "c.zip", data),
        platform="linux-x64",
        pins_path=_pins(
            tmp_path / "pins.toml", hashlib.sha256(data).hexdigest()
        ),
    )


def test_wrong_bytes_fail_the_release(tmp_path):
    with pytest.raises(ValueError, match="sha256"):
        assert_chromium_bytes(
            _archive(tmp_path / "c.zip", b"tampered"),
            platform="linux-x64",
            pins_path=_pins(tmp_path / "pins.toml", "cd" * 32),
        )
