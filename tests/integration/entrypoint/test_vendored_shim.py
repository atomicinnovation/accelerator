"""The committed verify shim for this host, checked against per-run keys."""

import os
import subprocess
from pathlib import Path

import pytest

from tasks.shared.paths import vendored_shim_path
from tests.integration.support.installation import sig_path, sign

pytest_plugins = ("tests.integration.support.installation_fixtures",)

_REFUSAL = "signature verification failed"


@pytest.fixture
def vendored_shim(host_platform: str) -> Path:
    shim = vendored_shim_path(host_platform)
    assert os.access(shim, os.X_OK), f"{shim} is not executable"
    return shim


@pytest.fixture
def payload(tmp_path: Path) -> Path:
    payload = tmp_path / "payload"
    payload.write_bytes(b"launcher bytes")
    return payload


def _verify(
    shim: Path, public_key: Path, target: Path
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [shim, public_key, sig_path(target), target],
        capture_output=True,
        text=True,
        check=False,
    )


def _assert_refused(verification: subprocess.CompletedProcess[str]) -> None:
    assert verification.returncode == 1, verification.stderr
    assert _REFUSAL in verification.stderr


def test_the_vendored_shim_accepts_a_release_key_signature(
    vendored_shim: Path, keys: Path, payload: Path
) -> None:
    sign(keys / "release.key", payload)

    verified = _verify(vendored_shim, keys / "release.pub", payload)

    assert verified.returncode == 0, verified.stderr


def test_the_vendored_shim_refuses_a_non_release_key_signature(
    vendored_shim: Path, keys: Path, payload: Path
) -> None:
    sign(keys / "attacker.key", payload)

    _assert_refused(_verify(vendored_shim, keys / "release.pub", payload))


def test_the_vendored_shim_refuses_a_payload_changed_after_signing(
    vendored_shim: Path, keys: Path, payload: Path
) -> None:
    sign(keys / "release.key", payload)
    payload.write_bytes(b"tampered bytes")

    _assert_refused(_verify(vendored_shim, keys / "release.pub", payload))
