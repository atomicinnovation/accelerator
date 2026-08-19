"""Chromium is pinned, not verified — the committed digest is the anchor.

There is no publisher signature to check, so these assert the two committed pins
(revision and per-platform byte digest) are enforced and a mismatch fails the
release.
"""

import hashlib

import pytest

from tasks.vendor.chromium import verify_chromium


def _browsers_json(path, revision="1181"):
    path.write_text(
        '{"browsers": ['
        '{"name": "chromium-headless-shell", "revision": "'
        + revision
        + '"}]}\n'
    )
    return path


def _pins(path, revision="1181", sha256="ab" * 32):
    path.write_text(
        f'[chromium]\nrevision = "{revision}"\n\n'
        f'[chromium.sha256]\nlinux-x64 = "{sha256}"\n'
    )
    return path


def _archive(path, data=b"chromium zip bytes"):
    path.write_bytes(data)
    return path


def test_matching_revision_and_bytes_pass(tmp_path):
    data = b"chromium zip bytes"
    verify_chromium(
        _archive(tmp_path / "c.zip", data),
        platform="linux-x64",
        browsers_json=_browsers_json(tmp_path / "browsers.json"),
        pins_path=_pins(
            tmp_path / "pins.toml", sha256=hashlib.sha256(data).hexdigest()
        ),
    )


def test_a_wrong_revision_fails_the_release(tmp_path):
    with pytest.raises(ValueError, match="revision"):
        verify_chromium(
            _archive(tmp_path / "c.zip"),
            platform="linux-x64",
            browsers_json=_browsers_json(tmp_path / "browsers.json", "1180"),
            pins_path=_pins(tmp_path / "pins.toml", revision="1181"),
        )


def test_wrong_bytes_fail_the_release(tmp_path):
    with pytest.raises(ValueError, match="sha256"):
        verify_chromium(
            _archive(tmp_path / "c.zip", b"tampered"),
            platform="linux-x64",
            browsers_json=_browsers_json(tmp_path / "browsers.json"),
            pins_path=_pins(tmp_path / "pins.toml", sha256="cd" * 32),
        )
