import shutil
import tarfile
from pathlib import Path
from unittest.mock import MagicMock

import pytest
from invoke import Context

import tasks.build as tb
from tasks.build import (
    VersionCoherenceError,
    _assert_no_e2e_insecure,
    _assert_static_elf,
    _debug_archive_targets,
    _is_statically_linked,
    _write_debug_archives,
    assert_staged_launcher_versions,
    validate_version_coherence,
    vendor_shim_marker_digest,
)
from tasks.shared.errors import InvalidVersionError
from tasks.shared.targets import TARGETS

_REPO_ROOT = Path(__file__).resolve().parents[3]


@pytest.fixture
def ctx():
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(return_code=0, stdout="")
    return m


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


# ── assert_staged_launcher_versions() ─────────────────────────────────


class TestAssertStagedLauncherVersions:
    def _stage(self, mocker, tmp_path):
        mocker.patch.object(
            tb,
            "cli_binary_path",
            side_effect=lambda n, p: tmp_path / f"{n}-{p}",
        )

    def test_passes_when_every_launcher_embeds_the_version(
        self, mocker, tmp_path
    ):
        self._stage(mocker, tmp_path)
        for _, platform in TARGETS:
            (tmp_path / f"accelerator-{platform}").write_bytes(
                b"prefix 1.21.0-pre.4 suffix"
            )
        assert_staged_launcher_versions("1.21.0-pre.4")

    def test_raises_when_a_launcher_embeds_the_wrong_version(
        self, mocker, tmp_path
    ):
        self._stage(mocker, tmp_path)
        for _, platform in TARGETS:
            (tmp_path / f"accelerator-{platform}").write_bytes(
                b"prefix 1.21.0-pre.4 suffix"
            )
        # One stale binary embeds an older version.
        (tmp_path / "accelerator-linux-x64").write_bytes(b"1.21.0-pre.3")
        with pytest.raises(RuntimeError, match="does not embed"):
            assert_staged_launcher_versions("1.21.0-pre.4")


class TestAssertNoE2eInsecure:
    def test_passes_when_marker_absent(self, tmp_path):
        artifact = tmp_path / "visualiser-linux-x64"
        artifact.write_bytes(b"\x00\x01harmless release bytes\x00")
        _assert_no_e2e_insecure(artifact)  # must not raise

    def test_ignores_other_visualiser_env_symbols(self, tmp_path):
        # Phase-3 config reading embeds these in every release binary; a prefix
        # scan would false-positive and block all releases.
        artifact = tmp_path / "visualiser-linux-x64"
        artifact.write_bytes(
            b"ACCELERATOR_VISUALISER_IDLE_TIMEOUT\x00"
            b"ACCELERATOR_VISUALISER_EDITOR\x00"
        )
        _assert_no_e2e_insecure(artifact)  # must not raise

    def test_raises_when_insecure_symbol_present(self, tmp_path):
        artifact = tmp_path / "visualiser-linux-x64"
        artifact.write_bytes(b"x\x00ACCELERATOR_VISUALISER_E2E_INSECURE\x00y")
        with pytest.raises(RuntimeError, match="E2E_INSECURE"):
            _assert_no_e2e_insecure(artifact)


class TestDebugArchiveTargets:
    def _dirs(self, tmp_path: Path) -> dict[str, Path]:
        return {token: tmp_path / token / "bin" for token in ("alpha", "beta")}

    def test_one_pair_per_token_per_target(self, tmp_path):
        dirs = self._dirs(tmp_path)
        targets = _debug_archive_targets(
            dirs, ("alpha", "beta"), tmp_path / "staging"
        )

        assert len(targets) == len(dirs) * len(TARGETS)
        for token, directory in dirs.items():
            for _triple, platform in TARGETS:
                pair = (
                    tmp_path / "staging" / f"accelerator-{token}-{platform}",
                    directory / f"accelerator-{token}-{platform}.debug.tar.gz",
                )
                assert pair in targets

    def test_an_undispatched_registry_key_raises(self, tmp_path):
        # Nothing cross-compiles it, so the archive source would be absent.
        with pytest.raises(RuntimeError, match="undispatched token"):
            _debug_archive_targets(self._dirs(tmp_path), ("alpha",), tmp_path)

    def test_a_non_bin_directory_raises(self, tmp_path):
        dirs = {"alpha": tmp_path / "alpha" / "artefacts"}
        with pytest.raises(RuntimeError, match="must be `bin/` trees"):
            _debug_archive_targets(dirs, ("alpha",), tmp_path)


class TestWriteDebugArchives:
    def test_writes_one_archive_per_pair(self, tmp_path):
        staging = tmp_path / "staging"
        staging.mkdir()
        targets = []
        for index in range(3):
            binary = staging / f"accelerator-alpha-{index}"
            binary.write_bytes(b"\x00" * 8)
            targets.append(
                (binary, tmp_path / f"nested/{index}/bin/alpha.debug.tar.gz")
            )

        _write_debug_archives(targets)

        for binary, archive in targets:
            assert archive.is_file()
            with tarfile.open(archive) as tar:
                assert tar.getnames() == [binary.name]


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

    def test_ignores_a_dev_dependency_change(self, tmp_path):
        # A dev-dependency compiles only into the crate's tests, never the
        # shim, so adding one under [dev-dependencies] (and to the lock block)
        # must not register as drift.
        baseline = vendor_shim_marker_digest()
        cli_dst = tmp_path / "cli"
        shutil.copytree(
            _REPO_ROOT / "cli",
            cli_dst,
            ignore=shutil.ignore_patterns("target"),
        )
        manifest = cli_dst / "verify" / "Cargo.toml"
        manifest.write_text(
            manifest.read_text().rstrip() + '\nfastrand = "2"\n'
        )
        lock = cli_dst / "Cargo.lock"
        lock.write_text(
            lock.read_text().replace(
                ' "tempfile",\n', ' "tempfile",\n "fastrand",\n', 1
            )
        )
        assert vendor_shim_marker_digest(root=tmp_path) == baseline


# ── validate_version_coherence() ─────────────────────────────────────


class TestValidateVersionCoherence:
    def test_all_match_returns_none(self, fake_repo_tree: Path):
        result = validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert result is None

    def test_visualiser_inheriting_covered_by_workspace_version(
        self, fake_repo_tree: Path
    ):
        # The visualiser inherits its version, so a skewed visualiser version
        # reduces to a workspace-version skew; _read_workspace_version catches
        # it now the standalone member-literal reader is gone.
        (fake_repo_tree / "cli/Cargo.toml").write_text(
            "[workspace]\n"
            'members = ["launcher", "visualiser/server"]\n\n'
            "[workspace.package]\n"
            'version = "0.9.0"\n'
        )
        with pytest.raises(VersionCoherenceError) as exc_info:
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert "cli/Cargo.toml" in str(exc_info.value)
        assert "0.9.0" in str(exc_info.value)

    def test_visualiser_member_pinning_drift_is_named(
        self, fake_repo_tree: Path
    ):
        # If the visualiser opts out of inheritance and pins a drifting literal,
        # _pinned_member_versions must still name it — the member stays covered.
        server_cargo = fake_repo_tree / "cli/visualiser/server/Cargo.toml"
        server_cargo.write_text(
            '[package]\nname = "accelerator-visualiser"\nversion = "0.9.0"\n'
        )
        with pytest.raises(VersionCoherenceError) as exc_info:
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert "cli/visualiser/server/Cargo.toml" in str(exc_info.value)
        assert "0.9.0" in str(exc_info.value)

    def test_plugin_json_mismatch_raises(self, fake_repo_tree: Path):
        plugin = fake_repo_tree / ".claude-plugin/plugin.json"
        plugin.write_text('{"name":"accelerator","version":"0.9.0"}')
        with pytest.raises(VersionCoherenceError) as exc_info:
            validate_version_coherence("1.20.0", repo_root=fake_repo_tree)
        assert "plugin.json" in str(exc_info.value)

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
