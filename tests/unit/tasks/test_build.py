import json
import shutil
from pathlib import Path
from unittest.mock import MagicMock

import pytest
from invoke import Context

import tasks.build as tb
from tasks.build import (
    VersionCoherenceError,
    _assert_static_elf,
    _is_statically_linked,
    create_checksums,
    update_checksums_json,
    validate_version_coherence,
    vendor_shim_marker_digest,
)
from tasks.shared.errors import InvalidVersionError
from tasks.shared.paths import cli_binary_path, vendored_shim_path
from tasks.shared.targets import TARGETS

_FIXTURES = Path(__file__).resolve().parent / "fixtures"
_REPO_ROOT = Path(__file__).resolve().parents[3]

_PLATFORMS = tuple(platform for _, platform in TARGETS)


@pytest.fixture
def ctx():
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(return_code=0, stdout="")
    return m


@pytest.fixture
def fake_binaries(fake_repo_tree: Path) -> Path:
    bin_dir = fake_repo_tree / "skills/visualisation/visualise/bin"
    for _, platform in TARGETS:
        (bin_dir / f"accelerator-visualiser-{platform}").write_bytes(
            b"\x00" * 4
        )
    return fake_repo_tree


def _patch_paths(mocker, base: Path) -> None:
    bin_dir = base / "skills/visualisation/visualise/bin"
    mocker.patch.object(tb, "BIN_DIR", bin_dir)
    mocker.patch.object(tb, "CHECKSUMS", bin_dir / "checksums.json")


_VENDORED_SHIM_DIR = _REPO_ROOT / "bin"


# ── static-linking assertion ──────────────────────────────────────────


class TestIsStaticallyLinked:
    @pytest.mark.parametrize(
        "output",
        [
            "ELF 64-bit LSB executable, x86-64, statically linked, stripped",
            "ELF 64-bit LSB pie executable, aarch64, static-pie linked",
            "ELF 64-bit LSB executable, not a dynamic executable",
        ],
    )
    def test_accepts_static_phrasings(self, output):
        assert _is_statically_linked(output) is True

    def test_rejects_dynamic_phrasing(self):
        dynamic = (
            "ELF 64-bit LSB pie executable, x86-64, ..., dynamically linked, "
            "interpreter /lib64/ld-linux-x86-64.so.2, ..., stripped"
        )
        assert _is_statically_linked(dynamic) is False


class TestAssertStaticElf:
    def test_accepts_a_real_static_musl_binary(self):
        # The committed linux-x64 shim is a real static musl ELF; anchors the
        # parser to real `file` output rather than to itself.
        _assert_static_elf(_VENDORED_SHIM_DIR / "accelerator-verify-linux-x64")

    def test_rejects_a_real_non_static_binary(self):
        # The committed darwin shim is a Mach-O — real `file` output that is not
        # "statically linked", so the assertion must reject it.
        with pytest.raises(RuntimeError, match="not statically linked"):
            _assert_static_elf(
                _VENDORED_SHIM_DIR / "accelerator-verify-darwin-arm64"
            )

    def test_fails_closed_when_file_reader_absent(self, tmp_path, mocker):
        mocker.patch.object(tb.shutil, "which", return_value=None)
        target = tmp_path / "binary"
        target.write_bytes(b"\x7fELF")
        with pytest.raises(RuntimeError, match="not on PATH"):
            _assert_static_elf(target)


# ── cli path helpers ──────────────────────────────────────────────────


class TestCliPathHelpers:
    def test_cli_binary_path_default_staging(self):
        path = cli_binary_path("accelerator", "linux-x64")
        assert path.name == "accelerator-linux-x64"
        assert path.parent == _REPO_ROOT / "dist" / "release"

    def test_cli_binary_path_custom_dir(self, tmp_path):
        path = cli_binary_path("accelerator-verify", "darwin-arm64", tmp_path)
        assert path == tmp_path / "accelerator-verify-darwin-arm64"

    def test_vendored_shim_path(self):
        path = vendored_shim_path("linux-arm64")
        assert path == _REPO_ROOT / "bin/accelerator-verify-linux-arm64"


# ── vendor_shim_marker_digest() ───────────────────────────────────────


class TestVendorShimMarkerDigest:
    def test_matches_committed_marker(self):
        recorded = (
            (_VENDORED_SHIM_DIR / "accelerator-verify.vendored.sha256")
            .read_text()
            .strip()
        )
        assert vendor_shim_marker_digest() == recorded

    def test_ignores_a_release_version_bump(self, tmp_path, mocker):
        # Copy the cli tree, bump the accelerator-verify lock version, and
        # assert the digest is unchanged: a version bump is not shim drift.
        baseline = vendor_shim_marker_digest()
        cli_src = _REPO_ROOT / "cli"
        cli_dst = tmp_path / "cli"
        shutil.copytree(
            cli_src, cli_dst, ignore=shutil.ignore_patterns("target")
        )
        lock = cli_dst / "Cargo.lock"
        lock.write_text(
            lock.read_text().replace(
                'name = "accelerator-verify"\nversion = "',
                'name = "accelerator-verify"\nversion = "99.',
                1,
            )
        )
        assert vendor_shim_marker_digest(root=tmp_path) == baseline

    def test_detects_a_minisign_verify_bump(self, tmp_path):
        baseline = vendor_shim_marker_digest()
        cli_dst = tmp_path / "cli"
        shutil.copytree(
            _REPO_ROOT / "cli",
            cli_dst,
            ignore=shutil.ignore_patterns("target"),
        )
        cargo = cli_dst / "Cargo.toml"
        cargo.write_text(
            cargo.read_text().replace(
                'minisign-verify = "=0.2.5"', 'minisign-verify = "=0.2.6"'
            )
        )
        assert vendor_shim_marker_digest(root=tmp_path) != baseline


# ── create_checksums() ────────────────────────────────────────────────


class TestCreateChecksums:
    def _common(self, mocker, fake_binaries):
        _patch_paths(mocker, fake_binaries)
        mocker.patch("tasks.build.validate_version_coherence")
        mock_update = mocker.patch("tasks.build.update_checksums_json")
        mocker.patch("tasks.build.compute_sha256", return_value="a" * 64)
        return mock_update

    def test_writes_all_platform_hashes(self, ctx, mocker, fake_binaries):
        mock_update = self._common(mocker, fake_binaries)
        create_checksums(ctx, "1.20.0")
        _, _, hashes_arg = mock_update.call_args.args
        assert set(hashes_arg.keys()) == set(_PLATFORMS)

    def test_debug_archives_not_in_checksums_manifest(
        self, ctx, mocker, fake_binaries
    ):
        mock_update = self._common(mocker, fake_binaries)
        create_checksums(ctx, "1.20.0")
        _, _, hashes_arg = mock_update.call_args.args
        assert all(not k.endswith(".debug.tar.gz") for k in hashes_arg)

    def test_version_drift_aborts_before_disk_writes(
        self, ctx, mocker, fake_binaries
    ):
        _patch_paths(mocker, fake_binaries)
        mocker.patch(
            "tasks.build.validate_version_coherence",
            side_effect=VersionCoherenceError("mismatch"),
        )
        mock_update = mocker.patch("tasks.build.update_checksums_json")

        with pytest.raises(VersionCoherenceError):
            create_checksums(ctx, "1.20.0")

        mock_update.assert_not_called()

    def test_no_real_filesystem_writes(self, ctx, mocker, fake_binaries):
        real_checksums = tb.CHECKSUMS
        before = real_checksums.read_bytes()
        self._common(mocker, fake_binaries)

        create_checksums(ctx, "1.20.0")

        assert real_checksums.read_bytes() == before


# ── update_checksums_json() ───────────────────────────────────────────


class TestUpdateChecksumsJson:
    def _load(self, path: Path) -> dict:
        return json.loads(path.read_text())

    def test_all_four_platforms_updated(self, tmp_path: Path):
        manifest = tmp_path / "checksums.json"
        shutil.copy(_FIXTURES / "checksums.with_sentinels.json", manifest)
        hashes = {
            "darwin-arm64": "a" * 64,
            "darwin-x64": "b" * 64,
            "linux-arm64": "c" * 64,
            "linux-x64": "d" * 64,
        }
        update_checksums_json(manifest, "1.20.0", hashes)
        expected = json.loads(
            (_FIXTURES / "checksums.example.json").read_text()
        )
        assert self._load(manifest) == expected

    def test_single_platform_preserves_others(self, tmp_path: Path):
        manifest = tmp_path / "checksums.json"
        shutil.copy(_FIXTURES / "checksums.example.json", manifest)
        update_checksums_json(manifest, "1.20.0", {"darwin-arm64": "e" * 64})
        data = self._load(manifest)
        assert data["binaries"]["darwin-arm64"] == f"sha256:{'e' * 64}"
        assert data["binaries"]["darwin-x64"] == f"sha256:{'b' * 64}"
        assert data["binaries"]["linux-arm64"] == f"sha256:{'c' * 64}"
        assert data["binaries"]["linux-x64"] == f"sha256:{'d' * 64}"

    def test_none_platform_hashes_only_updates_version(self, tmp_path: Path):
        manifest = tmp_path / "checksums.json"
        shutil.copy(_FIXTURES / "checksums.example.json", manifest)
        original_binaries = self._load(manifest)["binaries"]
        update_checksums_json(manifest, "1.21.0")
        data = self._load(manifest)
        assert data["version"] == "1.21.0"
        assert data["binaries"] == original_binaries

    def test_missing_manifest_raises(self, tmp_path: Path):
        with pytest.raises(FileNotFoundError):
            update_checksums_json(tmp_path / "nonexistent.json", "1.20.0")

    def test_atomic_write_failure_preserves_original(
        self, tmp_path: Path, mocker
    ):
        manifest = tmp_path / "checksums.json"
        shutil.copy(_FIXTURES / "checksums.example.json", manifest)
        original = manifest.read_bytes()
        mocker.patch.object(
            Path, "write_text", side_effect=OSError("disk full")
        )
        with pytest.raises(OSError):
            update_checksums_json(
                manifest, "1.20.0", {"darwin-arm64": "e" * 64}
            )
        assert manifest.read_bytes() == original
        assert not (tmp_path / "checksums.json.tmp").exists()


# ── validate_version_coherence() ─────────────────────────────────────


class TestValidateVersionCoherence:
    def test_all_match_returns_none(self, fake_repo_tree: Path):
        result = validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert result is None

    def test_cargo_toml_mismatch_raises(self, fake_repo_tree: Path):
        cargo = (
            fake_repo_tree / "skills/visualisation/visualise/server/Cargo.toml"
        )
        cargo.write_text('[package]\nname = "x"\nversion = "0.9.0"\n')
        with pytest.raises(VersionCoherenceError) as exc_info:
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert "Cargo.toml" in str(exc_info.value)
        assert "0.9.0" in str(exc_info.value)

    def test_plugin_json_mismatch_raises(self, fake_repo_tree: Path):
        plugin = fake_repo_tree / ".claude-plugin/plugin.json"
        plugin.write_text('{"name":"accelerator","version":"0.9.0"}')
        with pytest.raises(VersionCoherenceError) as exc_info:
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert "plugin.json" in str(exc_info.value)

    def test_checksums_json_mismatch_raises(self, fake_repo_tree: Path):
        checksums = (
            fake_repo_tree / "skills/visualisation/visualise/bin/checksums.json"
        )
        data = json.loads(checksums.read_text())
        data["version"] = "0.9.0"
        checksums.write_text(json.dumps(data))
        with pytest.raises(VersionCoherenceError) as exc_info:
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert "checksums.json" in str(exc_info.value)

    def test_missing_file_raises_file_not_found(self, fake_repo_tree: Path):
        (fake_repo_tree / ".claude-plugin/plugin.json").unlink()
        with pytest.raises(FileNotFoundError):
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)

    def test_empty_expected_version_raises_invalid_version(
        self, fake_repo_tree: Path
    ):
        with pytest.raises(InvalidVersionError):
            validate_version_coherence("", repo_root=fake_repo_tree)


# ── cli/ workspace coherence ──────────────────────────────────────────


class TestCliWorkspaceCoherence:
    def _cli_cargo(self, root: Path) -> Path:
        return root / "cli/Cargo.toml"

    def _launcher_cargo(self, root: Path) -> Path:
        return root / "cli/launcher/Cargo.toml"

    def test_workspace_match_and_member_inherits_passes(
        self, fake_repo_tree: Path
    ):
        # The launcher member inherits (version.workspace = true), so it
        # contributes no entry and can never be a mismatch.
        result = validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert result is None

    def test_workspace_version_mismatch_names_cli_cargo_toml(
        self, fake_repo_tree: Path
    ):
        self._cli_cargo(fake_repo_tree).write_text(
            "[workspace]\n"
            'members = ["launcher"]\n\n'
            "[workspace.package]\n"
            'version = "0.9.0"\n'
        )
        with pytest.raises(VersionCoherenceError) as exc_info:
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert "cli/Cargo.toml" in str(exc_info.value)
        assert "0.9.0" in str(exc_info.value)

    def test_member_pinning_drifting_version_is_named(
        self, fake_repo_tree: Path
    ):
        self._launcher_cargo(fake_repo_tree).write_text(
            '[package]\nname = "launcher"\nversion = "0.9.0"\n'
        )
        with pytest.raises(VersionCoherenceError) as exc_info:
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert "cli/launcher/Cargo.toml" in str(exc_info.value)
        assert "0.9.0" in str(exc_info.value)

    def test_member_pinning_matching_version_passes(self, fake_repo_tree: Path):
        # A member may opt out of inheritance and still be coherent if it pins
        # the same version.
        self._launcher_cargo(fake_repo_tree).write_text(
            '[package]\nname = "launcher"\nversion = "1.20.0"\n'
        )
        assert (
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
            is None
        )

    def test_empty_members_is_a_no_op(self, fake_repo_tree: Path):
        # No members to enumerate, but the workspace version is still checked;
        # this must not silently pass while masking absent coverage.
        self._cli_cargo(fake_repo_tree).write_text(
            "[workspace]\n"
            "members = []\n\n"
            "[workspace.package]\n"
            'version = "1.20.0"\n'
        )
        assert (
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
            is None
        )

    def test_missing_workspace_package_version_raises(
        self, fake_repo_tree: Path
    ):
        self._cli_cargo(fake_repo_tree).write_text(
            "[workspace]\nmembers = []\n"
        )
        with pytest.raises(VersionCoherenceError) as exc_info:
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert "cli/Cargo.toml" in str(exc_info.value)
        assert "[workspace.package].version" in str(exc_info.value)

    def test_missing_workspace_members_key_raises(self, fake_repo_tree: Path):
        self._cli_cargo(fake_repo_tree).write_text(
            '[workspace]\n\n[workspace.package]\nversion = "1.20.0"\n'
        )
        with pytest.raises(VersionCoherenceError) as exc_info:
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert "[workspace].members" in str(exc_info.value)

    def test_listed_but_absent_member_manifest_raises(
        self, fake_repo_tree: Path
    ):
        self._launcher_cargo(fake_repo_tree).unlink()
        with pytest.raises(VersionCoherenceError) as exc_info:
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert "cli/launcher/Cargo.toml" in str(exc_info.value)
