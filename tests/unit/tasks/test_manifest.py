"""The manifest emitter: schema conformance, description sourcing, signing.

The non-empty end-to-end test emits a real manifest with the real producer and
proves the producer→consumer contract using the same `minisign-verify` the
launcher uses (the built `accelerator-verify` shim) plus the frozen JSON schema
and a sha256 cross-check, rather than an in-process `FetchVerifyCacheResolver`
call, which would need a fragile cross-language artifact handoff. The existing
Rust `resolution.rs` / `manifest.rs` tests parse the identical shape, so the two
together cover the full parse → verify → resolve path against producer bytes.
"""

import hashlib
import json
import os
import shutil
import subprocess
from pathlib import Path
from unittest.mock import MagicMock

import pytest
from invoke import Context
from jsonschema import Draft202012Validator

from tasks.build import VersionCoherenceError, validate_version_coherence
from tasks.manifest import (
    BinaryEntry,
    build_manifest,
    collect_entries,
    emit_manifest,
)
from tasks.shared.errors import ManifestError
from tasks.shared.targets import TARGETS
from tasks.signing import generate, sign_file

_REPO_ROOT = Path(__file__).resolve().parents[3]
_SCHEMA = _REPO_ROOT / "cli/launcher/tests/fixtures/manifest.schema.json"
_REAL_VERSION = json.loads(
    (_REPO_ROOT / ".claude-plugin/plugin.json").read_text()
)["version"]
_PLATFORMS = tuple(platform for _, platform in TARGETS)


def _in_ci() -> bool:
    return bool(os.environ.get("CI") or os.environ.get("GITHUB_ACTIONS"))


def _require(name: str) -> None:
    if shutil.which(name):
        return
    message = f"{name} not on PATH"
    if _in_ci():
        pytest.fail(f"{message} — provisioning regression in CI")
    pytest.skip(message)


@pytest.fixture(scope="module")
def shim_bin() -> Path:
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


def _keypair(tmp_path: Path) -> tuple[Path, Path]:
    _require("minisign")
    pub = tmp_path / "release.pub"
    sec = tmp_path / "release.sec"
    generate(MagicMock(spec=Context), pub_path=str(pub), sec_path=str(sec))
    return pub, sec


def _verifies(shim: Path, pub: Path, sig: Path, target: Path) -> bool:
    return (
        subprocess.run(
            [str(shim), str(pub), str(sig), str(target)],
            check=False,
            capture_output=True,
            text=True,
        ).returncode
        == 0
    )


def _cargo_toml_with_description(tmp_path: Path, description: str) -> Path:
    manifest = tmp_path / "Cargo.toml"
    manifest.write_text(
        f'[package]\nname = "foo"\ndescription = "{description}"\n'
    )
    return manifest


def _stage_and_sign(staging: Path, name: str, sec: Path) -> dict[str, bytes]:
    staging.mkdir(parents=True, exist_ok=True)
    payloads: dict[str, bytes] = {}
    for platform in _PLATFORMS:
        binary = staging / f"{name}-{platform}"
        payload = f"{name} {platform} bytes".encode()
        binary.write_bytes(payload)
        payloads[platform] = payload
        sign_file(sec, binary, binary.with_name(binary.name + ".minisig"))
    return payloads


# ── build_manifest() shape ────────────────────────────────────────────


class TestBuildManifest:
    def test_empty_binaries_is_valid(self):
        manifest = build_manifest(_REAL_VERSION, {})
        Draft202012Validator(json.loads(_SCHEMA.read_text())).validate(manifest)
        assert manifest["schema_version"] == 1
        assert manifest["version"] == _REAL_VERSION
        assert manifest["binaries"] == {}

    def test_non_empty_validates_against_schema(self):
        entry = BinaryEntry(
            description="Frobnicate things",
            platforms={
                platform: {"sha256": "a" * 64, "signature": "sig"}
                for platform in _PLATFORMS
            },
        )
        manifest = build_manifest(_REAL_VERSION, {"foo": entry})
        Draft202012Validator(json.loads(_SCHEMA.read_text())).validate(manifest)
        assert manifest["binaries"]["foo"]["description"] == "Frobnicate things"


# ── collect_entries() ─────────────────────────────────────────────────


class TestCollectEntries:
    def test_sources_description_and_signature(self, tmp_path):
        _require("minisign")
        _, sec = _keypair(tmp_path)
        staging = tmp_path / "dist"
        payloads = _stage_and_sign(staging, "foo", sec)
        cargo = _cargo_toml_with_description(tmp_path, "Frobnicator tool")

        entries = collect_entries(
            ["foo"], staging_dir=staging, manifest_for=lambda _n: cargo
        )

        assert entries["foo"].description == "Frobnicator tool"
        for platform in _PLATFORMS:
            asset = entries["foo"].platforms[platform]
            assert len(asset["sha256"]) == 64
            assert asset["signature"].startswith("untrusted comment:")
            assert payloads[platform]  # staged

    def test_missing_description_raises(self, tmp_path):
        _require("minisign")
        _, sec = _keypair(tmp_path)
        staging = tmp_path / "dist"
        _stage_and_sign(staging, "foo", sec)
        cargo = tmp_path / "Cargo.toml"
        cargo.write_text('[package]\nname = "foo"\n')

        with pytest.raises(ManifestError, match=r"package\.description"):
            collect_entries(
                ["foo"], staging_dir=staging, manifest_for=lambda _n: cargo
            )

    def test_empty_subbinaries_yields_no_entries(self):
        assert collect_entries([]) == {}


# ── emit_manifest() round-trip ────────────────────────────────────────


class TestEmitManifest:
    def test_signs_to_manifest_minisig_and_verifies(self, tmp_path, shim_bin):
        pub, sec = _keypair(tmp_path)
        out = tmp_path / "manifest.json"

        returned = emit_manifest(out, _REAL_VERSION, {}, sec)

        assert returned == out
        sig = tmp_path / "manifest.minisig"
        assert sig.exists()
        assert not (tmp_path / "manifest.json.minisig").exists()
        assert _verifies(shim_bin, pub, sig, out)

    def test_signature_covers_the_exact_written_bytes(self, tmp_path, shim_bin):
        pub, sec = _keypair(tmp_path)
        out = tmp_path / "manifest.json"
        emit_manifest(out, _REAL_VERSION, {}, sec)
        # Re-reading and re-serialising must not be needed to verify — the
        # signature covers the on-disk bytes verbatim.
        assert _verifies(shim_bin, pub, tmp_path / "manifest.minisig", out)
        assert out.read_text().endswith("}\n")

    def test_non_empty_end_to_end(self, tmp_path, shim_bin):
        pub, sec = _keypair(tmp_path)
        staging = tmp_path / "dist"
        payloads = _stage_and_sign(staging, "foo", sec)
        cargo = _cargo_toml_with_description(tmp_path, "Frobnicator")
        entries = collect_entries(
            ["foo"], staging_dir=staging, manifest_for=lambda _n: cargo
        )

        out = tmp_path / "manifest.json"
        emit_manifest(out, _REAL_VERSION, entries, sec)

        manifest = json.loads(out.read_text())
        Draft202012Validator(json.loads(_SCHEMA.read_text())).validate(manifest)
        # The manifest signature verifies the raw bytes (the launcher's first
        # check), via the same minisign-verify crate the launcher embeds.
        assert _verifies(shim_bin, pub, tmp_path / "manifest.minisig", out)

        foo = manifest["binaries"]["foo"]
        assert foo["description"] == "Frobnicator"
        for platform in _PLATFORMS:
            entry = foo["platforms"][platform]
            expected = hashlib.sha256(payloads[platform]).hexdigest()
            assert entry["sha256"] == expected
            # The inline signature verifies the sub-binary bytes (the launcher's
            # per-binary check).
            binary = staging / f"foo-{platform}"
            inline = tmp_path / f"{platform}.inline.minisig"
            inline.write_text(entry["signature"])
            assert _verifies(shim_bin, pub, inline, binary)


# ── version coherence with manifest.version ───────────────────────────


class TestManifestVersionCoherence:
    def _write_manifest(self, path: Path, version: str) -> Path:
        path.write_text(json.dumps({"version": version}))
        return path

    def test_agreement_passes(self, fake_repo_tree):
        manifest = self._write_manifest(
            fake_repo_tree / "manifest.json", "1.20.0"
        )
        validate_version_coherence(
            "1.20.0", repo_root=fake_repo_tree, manifest_path=manifest
        )

    def test_disagreement_raises_and_names_manifest(self, fake_repo_tree):
        manifest = self._write_manifest(
            fake_repo_tree / "manifest.json", "9.9.9"
        )
        with pytest.raises(VersionCoherenceError) as exc:
            validate_version_coherence(
                "1.20.0", repo_root=fake_repo_tree, manifest_path=manifest
            )
        assert "manifest.json" in str(exc.value)
        assert "9.9.9" in str(exc.value)

    def test_missing_manifest_raises(self, fake_repo_tree):
        with pytest.raises(FileNotFoundError):
            validate_version_coherence(
                "1.20.0",
                repo_root=fake_repo_tree,
                manifest_path=fake_repo_tree / "absent.json",
            )

    def test_without_manifest_path_is_unaffected(self, fake_repo_tree):
        validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
