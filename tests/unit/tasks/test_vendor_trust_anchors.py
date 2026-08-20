"""The release-lane trust-anchor guard.

The guard's whole job is to distinguish a placeholder anchor from a real one, so
the tests exercise that discrimination against fixtures rather than asserting
the repository's live anchors are ready — which, pre-refresh, they are not.
"""

import pytest

from tasks.vendor.trust_anchors import (
    TrustAnchorsNotReadyError,
    assert_ready,
    placeholder_reasons,
)

_REAL_DIGEST = "0123456789abcdef" * 4
_PLACEHOLDER_PINS = """\
[assembled_sha256.browser]
linux-x64 = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"

[assembled_sha256.driver]
linux-x64 = "8888888888888888888888888888888888888888888888888888888888888888"

[chromium]
revision = "0000000"

[chromium.sha256]
linux-x64 = "0000000000000000000000000000000000000000000000000000000000000000"

[node]
version = "0.0.0"
"""

_REAL_PINS = f"""\
[assembled_sha256.browser]
linux-x64 = "{_REAL_DIGEST}"

[assembled_sha256.driver]
linux-x64 = "{_REAL_DIGEST}"

[chromium]
revision = "1181205"

[chromium.sha256]
linux-x64 = "{_REAL_DIGEST}"

[node]
version = "20.18.0"
"""


def _keys(tmp_path):
    keys_dir = tmp_path / "keys"
    keys_dir.mkdir()
    (keys_dir / "nodejs-release.asc").write_text("-----BEGIN PGP PUBLIC KEY-\n")
    (keys_dir / "npm-registry.pem").write_text("-----BEGIN PUBLIC KEY-----\n")
    return keys_dir


def _pins(tmp_path, body):
    pins = tmp_path / "pins.toml"
    pins.write_text(body)
    return pins


def test_placeholder_pins_and_absent_keys_are_all_flagged(tmp_path):
    reasons = placeholder_reasons(
        _pins(tmp_path, _PLACEHOLDER_PINS), tmp_path / "keys"
    )
    joined = "\n".join(reasons)
    assert "assembled_sha256.browser.linux-x64" in joined
    assert "assembled_sha256.driver.linux-x64" in joined
    assert "chromium.revision" in joined
    assert "chromium.sha256.linux-x64" in joined
    assert "node.version" in joined
    assert "keys/nodejs-release.asc" in joined
    assert "keys/npm-registry.pem" in joined


def test_real_anchors_and_present_keys_pass(tmp_path):
    reasons = placeholder_reasons(_pins(tmp_path, _REAL_PINS), _keys(tmp_path))
    assert reasons == []


def test_assert_ready_raises_on_placeholders_naming_the_procedure(tmp_path):
    with pytest.raises(TrustAnchorsNotReadyError, match=r"RELEASING\.md"):
        assert_ready(_pins(tmp_path, _PLACEHOLDER_PINS), tmp_path / "keys")


def test_assert_ready_is_silent_when_every_anchor_is_real(tmp_path):
    assert_ready(_pins(tmp_path, _REAL_PINS), _keys(tmp_path))


def test_an_empty_key_file_counts_as_absent(tmp_path):
    keys_dir = _keys(tmp_path)
    (keys_dir / "npm-registry.pem").write_text("")
    reasons = placeholder_reasons(_pins(tmp_path, _REAL_PINS), keys_dir)
    assert reasons == ["keys/npm-registry.pem is absent or empty"]
