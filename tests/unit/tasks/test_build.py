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
    _assert_no_test_loopback,
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


class TestAssertNoTestLoopback:
    def test_passes_when_marker_absent(self, tmp_path):
        artifact = tmp_path / "accelerator-linear-linux-x64"
        artifact.write_bytes(b"\x00\x01an ordinary release binary\x00")
        _assert_no_test_loopback(artifact)  # must not raise

    def test_raises_for_the_linear_marker(self, tmp_path):
        artifact = tmp_path / "accelerator-linear-linux-x64"
        artifact.write_bytes(
            b"x\x00ACCELERATOR_LINEAR_TEST_LOOPBACK_MARKER\x00y"
        )
        with pytest.raises(RuntimeError, match="test-loopback"):
            _assert_no_test_loopback(artifact)

    def test_raises_for_the_jira_marker(self, tmp_path):
        # The shared suffix covers jira before its binary phase lands, so no
        # per-provider edit is needed when it joins the release set.
        artifact = tmp_path / "accelerator-jira-linux-x64"
        artifact.write_bytes(b"ACCELERATOR_JIRA_TEST_LOOPBACK_MARKER\x00")
        with pytest.raises(RuntimeError, match="test-loopback"):
            _assert_no_test_loopback(artifact)


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


def _seed_digest_inputs(tmp_path: Path) -> Path:
    """Copy just the tree `vendor_shim_marker_digest` reads into `tmp_path`.

    It reads `cli/verify/**`, the `minisign-verify` pin in `cli/Cargo.toml`
    and `cli/Cargo.lock` — three inputs, ~12 KB. Copying the whole of `cli/`
    instead walked 46k files including the ~200 MB
    `cli/visualiser/frontend/node_modules`, following its symlinks, so the
    copy raced any concurrently-running frontend task and raised
    `shutil.Error` on a link whose target moved. That was the macOS CI flake.

    Each caller asserts against a baseline digest taken over the *real* tree,
    so an input missed here shows up as a mismatch rather than a false pass.
    """
    cli_dst = tmp_path / "cli"
    cli_dst.mkdir(parents=True, exist_ok=True)
    shutil.copytree(_REPO_ROOT / "cli" / "verify", cli_dst / "verify")
    for name in ("Cargo.toml", "Cargo.lock"):
        shutil.copy2(_REPO_ROOT / "cli" / name, cli_dst / name)
    return cli_dst


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
        cli_dst = _seed_digest_inputs(tmp_path)
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
        cli_dst = _seed_digest_inputs(tmp_path)
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
        cli_dst = _seed_digest_inputs(tmp_path)
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


class TestFixtureSizeFloor:
    """The guard against this story's headline false pass: dead-code
    elimination letting the musl and size checks succeed while linking almost
    none of gix/jj-lib.

    Threshold logic only — no real tb. The cross-compile that otherwise
    exercises it runs solely in the release pipeline, which is exactly why the
    comparison is a pure function.
    """

    MUSL = "aarch64-unknown-linux-musl"
    DARWIN = "aarch64-apple-darwin"

    def test_the_measured_musl_figures_pass(self) -> None:
        # 2,422,864 vs 391,416 — the delivered two-binary shape.
        tb.assert_fixture_size_floor(2_422_864, 391_416, triple=self.MUSL)

    def test_the_measured_darwin_figures_pass(self) -> None:
        tb.assert_fixture_size_floor(2_031_288, 391_416, triple=self.DARWIN)

    def test_a_collapsed_ratio_fails_on_every_triple(self) -> None:
        for triple in (self.MUSL, self.DARWIN):
            with pytest.raises(RuntimeError, match=r"below the .* floor"):
                tb.assert_fixture_size_floor(1_000_000, 400_000, triple=triple)

    def test_the_absolute_floor_is_musl_only(self) -> None:
        # A wide ratio but a small delta: musl rejects, darwin accepts. This is
        # the scoping rule — `[profile.release] strip = true` means every triple
        # is stripped and the darwin delta clears the floor by only ~9%, so
        # gating darwin would put a 9%-margin heuristic on the release path.
        with pytest.raises(RuntimeError, match="bytes larger"):
            tb.assert_fixture_size_floor(1_600_000, 400_000, triple=self.MUSL)
        tb.assert_fixture_size_floor(1_600_000, 400_000, triple=self.DARWIN)

    def test_a_zero_sized_stub_is_rejected_rather_than_dividing_by_zero(
        self,
    ) -> None:
        with pytest.raises(RuntimeError, match="cannot compare"):
            tb.assert_fixture_size_floor(2_000_000, 0, triple=self.MUSL)

    def test_the_fixture_binaries_are_never_release_artefacts(self) -> None:
        # They print absolute repository paths and are not product. Keeping them
        # out of _CLI_RELEASE_BINARIES is what keeps them out of dist/release/,
        # the tree the signed manifest is assembled from.
        overlap = set(tb._CLI_FIXTURE_BINARIES) & set(tb._CLI_RELEASE_BINARIES)
        assert not overlap, f"fixture binaries staged as product: {overlap}"

    def test_no_fixture_binary_carries_the_attested_prefix(self) -> None:
        # dist/release/accelerator-* is provenance-attested by a glob.
        for name in tb._CLI_FIXTURE_BINARIES:
            assert not name.startswith("accelerator-"), name

    def test_no_fixture_binary_is_a_release_upload(self) -> None:
        # The direct assertion, not just the constants: _release_uploads()
        # enumerates assets explicitly rather than globbing, so nothing would be
        # published today — but these binaries print absolute repository paths
        # and must stay one deliberate decision away from being product.
        from tasks.github import _release_uploads

        names = {path.name for path in _release_uploads()}
        for fixture in tb._CLI_FIXTURE_BINARIES:
            assert not any(fixture in name for name in names), (
                f"{fixture} is enumerated as a release upload"
            )
