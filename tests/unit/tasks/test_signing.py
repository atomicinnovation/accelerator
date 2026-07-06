"""Signing primitives: minisign helpers, secret-key lifecycle, key generation.

The sign → verify round-trips run against the real `minisign` CLI and the real
`accelerator-verify` shim built from `cli/`. Both are mise-provisioned, so a
missing tool is a CI provisioning regression (fail) rather than a local skip.
"""

import os
import shutil
import subprocess
from pathlib import Path
from unittest.mock import MagicMock

import pytest
from invoke import Context

from tasks import signing
from tasks.shared.errors import SigningError
from tasks.signing import (
    SECRET_KEY_ENV,
    generate,
    resolve_secret_key,
    sign_file,
)

_REPO_ROOT = Path(__file__).resolve().parents[3]


def _in_ci() -> bool:
    return bool(os.environ.get("CI") or os.environ.get("GITHUB_ACTIONS"))


def _require(name: str) -> None:
    if shutil.which(name):
        return
    message = f"{name} not on PATH"
    if _in_ci():
        pytest.fail(f"{message} — provisioning regression in CI")
    pytest.skip(message)


@pytest.fixture
def ctx():
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(return_code=0, stdout="")
    return m


@pytest.fixture(scope="module")
def shim_bin() -> Path:
    """Build and return the real `accelerator-verify` shim from `cli/`."""
    _require("cargo")
    subprocess.run(
        [
            "cargo",
            "build",
            "--quiet",
            "-p",
            "accelerator-verify",
            "--manifest-path",
            str(_REPO_ROOT / "cli/Cargo.toml"),
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    shim = _REPO_ROOT / "cli/target/debug/accelerator-verify"
    if not (shim.exists() and os.access(shim, os.X_OK)):
        pytest.fail(f"shim not built: {shim}")
    return shim


def _keypair(tmp_path: Path, name: str = "release") -> tuple[Path, Path]:
    _require("minisign")
    pub = tmp_path / f"{name}.pub"
    sec = tmp_path / f"{name}.sec"
    generate(MagicMock(spec=Context), pub_path=str(pub), sec_path=str(sec))
    return pub, sec


def _verify_via_shim(shim: Path, pub: Path, sig: Path, target: Path) -> int:
    return subprocess.run(
        [str(shim), str(pub), str(sig), str(target)],
        check=False,
        capture_output=True,
        text=True,
    ).returncode


# ── sign_file() ───────────────────────────────────────────────────────


class TestSignFile:
    def test_signs_to_the_exact_path_and_verifies(
        self, tmp_path: Path, shim_bin: Path
    ):
        pub, sec = _keypair(tmp_path)
        target = tmp_path / "launcher"
        target.write_bytes(b"launcher bytes")
        signature = tmp_path / "custom-name.minisig"

        returned = sign_file(sec, target, signature)

        assert returned == signature
        assert signature.exists()
        assert _verify_via_shim(shim_bin, pub, signature, target) == 0

    def test_manifest_signs_to_manifest_minisig(self, tmp_path: Path):
        _, sec = _keypair(tmp_path)
        manifest = tmp_path / "manifest.json"
        manifest.write_text('{"schema_version":1}')
        signature = tmp_path / "manifest.minisig"

        sign_file(sec, manifest, signature)

        assert signature.name == "manifest.minisig"
        assert not (tmp_path / "manifest.json.minisig").exists()

    def test_signature_has_the_four_line_shape(self, tmp_path: Path):
        _, sec = _keypair(tmp_path)
        target = tmp_path / "payload"
        target.write_bytes(b"data")
        signature = tmp_path / "payload.minisig"

        sign_file(sec, target, signature)

        lines = signature.read_text().splitlines()
        assert len(lines) == 4
        assert lines[0].startswith("untrusted comment:")
        assert lines[2].startswith("trusted comment:")

    def test_default_prehash_is_not_legacy(self, tmp_path: Path):
        _, sec = _keypair(tmp_path)
        target = tmp_path / "payload"
        target.write_bytes(b"data")
        signature = tmp_path / "payload.minisig"

        sign_file(sec, target, signature)

        assert "hashed" in signature.read_text().splitlines()[2]

    def test_non_zero_exit_raises_signing_error_with_stderr(
        self, tmp_path: Path, mocker
    ):
        mocker.patch.object(
            signing.subprocess,
            "run",
            return_value=MagicMock(
                returncode=1, stderr="minisign: bad secret key\n"
            ),
        )
        with pytest.raises(SigningError, match="bad secret key"):
            sign_file(
                tmp_path / "k.sec", tmp_path / "t", tmp_path / "t.minisig"
            )


# ── resolve_secret_key() ──────────────────────────────────────────────


class TestResolveSecretKey:
    def test_yields_dev_key_when_env_unset(self, tmp_path: Path, monkeypatch):
        monkeypatch.delenv(SECRET_KEY_ENV, raising=False)
        dev_key = tmp_path / "dev.sec"
        dev_key.write_text("secret")
        with resolve_secret_key(dev_key=dev_key) as key:
            assert key == dev_key

    def test_unset_env_and_no_dev_key_fails_closed(
        self, tmp_path: Path, monkeypatch
    ):
        monkeypatch.delenv(SECRET_KEY_ENV, raising=False)
        with (
            pytest.raises(SigningError, match=SECRET_KEY_ENV),
            resolve_secret_key(dev_key=tmp_path / "absent.sec"),
        ):
            pass

    def test_materialises_env_secret_to_0600_and_cleans_up(
        self, tmp_path: Path, monkeypatch
    ):
        monkeypatch.setenv(SECRET_KEY_ENV, "materialised-secret")
        seen: list[Path] = []
        with resolve_secret_key(dev_key=tmp_path / "unused.sec") as key:
            seen.append(key)
            assert key.read_text() == "materialised-secret"
            mode = key.stat().st_mode & 0o777
            assert mode == 0o600, oct(mode)
        assert not seen[0].exists()

    def test_multiple_signs_share_one_materialised_key(
        self, tmp_path: Path, shim_bin: Path, monkeypatch
    ):
        pub, sec = _keypair(tmp_path)
        monkeypatch.setenv(SECRET_KEY_ENV, sec.read_text())
        targets = []
        for i in range(3):
            target = tmp_path / f"bin-{i}"
            target.write_bytes(f"bin {i}".encode())
            targets.append(target)

        live_key: list[Path] = []
        with resolve_secret_key() as key:
            live_key.append(key)
            for target in targets:
                sign_file(key, target, target.with_suffix(".minisig"))

        for target in targets:
            sig = target.with_suffix(".minisig")
            assert _verify_via_shim(shim_bin, pub, sig, target) == 0
        assert not live_key[0].exists()

    def test_materialised_key_cleaned_up_on_exception(
        self, tmp_path: Path, monkeypatch
    ):
        monkeypatch.setenv(SECRET_KEY_ENV, "secret")
        seen: list[Path] = []
        with (
            pytest.raises(RuntimeError, match="boom"),
            resolve_secret_key() as key,
        ):
            seen.append(key)
            assert key.exists()
            raise RuntimeError("boom")
        assert not seen[0].exists()


# ── keys.generate() ───────────────────────────────────────────────────


class TestGenerate:
    def test_produces_a_parsing_key_without_touching_the_tracked_pub(
        self, tmp_path: Path, ctx, shim_bin: Path
    ):
        tracked = _REPO_ROOT / "keys/accelerator-release.pub"
        before = tracked.read_bytes()

        pub = tmp_path / "generated.pub"
        sec = tmp_path / "generated.sec"
        generate(ctx, pub_path=str(pub), sec_path=str(sec))

        assert pub.exists()
        assert sec.exists()
        target = tmp_path / "payload"
        target.write_bytes(b"round-trip")
        signature = tmp_path / "payload.minisig"
        sign_file(sec, target, signature)
        assert _verify_via_shim(shim_bin, pub, signature, target) == 0

        assert tracked.read_bytes() == before

    def test_failure_raises_signing_error(self, tmp_path: Path, ctx, mocker):
        mocker.patch.object(
            signing.subprocess,
            "run",
            return_value=MagicMock(returncode=1, stderr="keygen failed\n"),
        )
        with pytest.raises(SigningError, match="key generation failed"):
            generate(
                ctx,
                pub_path=str(tmp_path / "p.pub"),
                sec_path=str(tmp_path / "s.sec"),
            )
