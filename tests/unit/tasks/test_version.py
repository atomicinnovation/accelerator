import json
import tomllib
from pathlib import Path
from unittest.mock import MagicMock

import pytest
from invoke import Context

import tasks.build as tb
import tasks.version as tv
from tasks.build import validate_version_coherence

REPO_ROOT = Path(__file__).resolve().parents[3]
SERVER_DIR = REPO_ROOT / "skills/visualisation/visualise/server"
CLI_DIR = REPO_ROOT / "cli"


@pytest.fixture
def ctx():
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(return_code=0, stdout="")
    return m


def _patch_paths(mocker, base: Path) -> None:
    mocker.patch.object(tv, "PLUGIN_JSON", base / ".claude-plugin/plugin.json")
    mocker.patch.object(
        tv,
        "CARGO_TOML",
        base / "skills/visualisation/visualise/server/Cargo.toml",
    )
    mocker.patch.object(tv, "CLI_WORKSPACE_CARGO_TOML", base / "cli/Cargo.toml")
    mocker.patch.object(
        tv,
        "CHECKSUMS",
        base / "skills/visualisation/visualise/bin/checksums.json",
    )


# ── write() ───────────────────────────────────────────────────────────


class TestWrite:
    def test_updates_all_three_files(self, ctx, mocker, fake_repo_tree):
        _patch_paths(mocker, fake_repo_tree)
        tv.write(ctx, "1.21.0")

        plugin_json = json.loads(
            (fake_repo_tree / ".claude-plugin/plugin.json").read_text()
        )
        cargo_toml = (
            fake_repo_tree / "skills/visualisation/visualise/server/Cargo.toml"
        ).read_text()
        checksums = json.loads(
            (
                fake_repo_tree
                / "skills/visualisation/visualise/bin/checksums.json"
            ).read_text()
        )

        assert plugin_json["version"] == "1.21.0"
        assert 'version = "1.21.0"' in cargo_toml
        assert checksums["version"] == "1.21.0"

    def test_checksums_binaries_map_preserved(
        self, ctx, mocker, fake_repo_tree
    ):
        _patch_paths(mocker, fake_repo_tree)
        checksums_before = json.loads(
            (
                fake_repo_tree
                / "skills/visualisation/visualise/bin/checksums.json"
            ).read_text()
        )
        tv.write(ctx, "1.21.0")
        checksums_after = json.loads(
            (
                fake_repo_tree
                / "skills/visualisation/visualise/bin/checksums.json"
            ).read_text()
        )
        assert checksums_after["binaries"] == checksums_before["binaries"]

    def test_cargo_toml_structure_preserved(self, ctx, mocker, fake_repo_tree):
        cargo_path = (
            fake_repo_tree / "skills/visualisation/visualise/server/Cargo.toml"
        )
        cargo_path.write_text(
            '[package]\nname = "accelerator-visualiser"\nversion = "1.20.0"\n\n'
            '[dependencies]\naxum = { version = "0.7" }\n'
        )
        _patch_paths(mocker, fake_repo_tree)
        tv.write(ctx, "1.21.0")
        result = cargo_path.read_text()
        assert 'version = "1.21.0"' in result
        assert 'axum = { version = "0.7" }' in result

    def test_idempotent(self, ctx, mocker, fake_repo_tree):
        _patch_paths(mocker, fake_repo_tree)
        tv.write(ctx, "1.21.0")
        content_after_first = {
            "plugin_json": (
                fake_repo_tree / ".claude-plugin/plugin.json"
            ).read_bytes(),
            "cargo_toml": (
                fake_repo_tree
                / "skills/visualisation/visualise/server/Cargo.toml"
            ).read_bytes(),
            "checksums": (
                fake_repo_tree
                / "skills/visualisation/visualise/bin/checksums.json"
            ).read_bytes(),
        }
        tv.write(ctx, "1.21.0")
        content_after_second = {
            "plugin_json": (
                fake_repo_tree / ".claude-plugin/plugin.json"
            ).read_bytes(),
            "cargo_toml": (
                fake_repo_tree
                / "skills/visualisation/visualise/server/Cargo.toml"
            ).read_bytes(),
            "checksums": (
                fake_repo_tree
                / "skills/visualisation/visualise/bin/checksums.json"
            ).read_bytes(),
        }
        assert content_after_first == content_after_second

    def test_coherence_passes_after_write(self, ctx, mocker, fake_repo_tree):
        _patch_paths(mocker, fake_repo_tree)
        mocker.patch.object(tb, "REPO_ROOT", fake_repo_tree)
        tv.write(ctx, "1.21.0")
        validate_version_coherence("1.21.0", repo_root=fake_repo_tree)

    def test_updates_cli_workspace_version(self, ctx, mocker, fake_repo_tree):
        _patch_paths(mocker, fake_repo_tree)
        tv.write(ctx, "1.21.0")
        workspace = tomllib.loads(
            (fake_repo_tree / "cli/Cargo.toml").read_text()
        )
        assert workspace["workspace"]["package"]["version"] == "1.21.0"


# ── cli/ workspace manifest render ────────────────────────────────────


class TestWorkspaceCargoRender:
    """Guard the tomlkit round-trip preserving the [workspace.lints.clippy]
    table and its comments (the property the justification-comment policy and
    the cherry-picked restriction lints depend on)."""

    def test_round_trip_preserves_lints_table_and_comments(
        self, mocker, fake_repo_tree
    ):
        cargo_path = fake_repo_tree / "cli/Cargo.toml"
        cargo_path.write_text(
            "[workspace]\n"
            'members = ["launcher"]\n\n'
            "[workspace.package]\n"
            'version = "1.20.0"\n'
            'edition = "2021"\n\n'
            "[workspace.lints.clippy]\n"
            "# restriction is allow-by-default; cherry-picked opt-ins.\n"
            'unwrap_used = "warn"\n'
            'pedantic = { level = "warn", priority = -1 }\n'
        )
        mocker.patch.object(tv, "CLI_WORKSPACE_CARGO_TOML", cargo_path)
        result = tv._render_workspace_cargo_toml("1.21.0")
        assert 'version = "1.21.0"' in result
        assert "[workspace.lints.clippy]" in result
        assert 'unwrap_used = "warn"' in result
        assert 'pedantic = { level = "warn", priority = -1 }' in result
        # The load-bearing assertion: a plain dict-based writer would drop this.
        assert "# restriction is allow-by-default" in result


# ── bump() ────────────────────────────────────────────────────────────


class TestBump:
    def _bump(self, ctx, mocker, fake_repo_tree, start, bump_types):
        _patch_paths(mocker, fake_repo_tree)
        tv.write(ctx, start)
        return str(tv.bump(ctx, bump_type=bump_types))

    def test_pre_from_stable_cuts_next_minor_prerelease(
        self, ctx, mocker, fake_repo_tree
    ):
        # Post-stable cut must open a fresh line, not re-cut 1.21.0-pre.1
        # (which collides with the tags that led up to the 1.21.0 release).
        result = self._bump(
            ctx, mocker, fake_repo_tree, "1.21.0", [tv.BumpType.PRE]
        )
        assert result == "1.22.0-pre.1"

    def test_pre_from_prerelease_increments_prerelease(
        self, ctx, mocker, fake_repo_tree
    ):
        result = self._bump(
            ctx, mocker, fake_repo_tree, "1.22.0-pre.1", [tv.BumpType.PRE]
        )
        assert result == "1.22.0-pre.2"

    def test_pre_is_the_default_bump_type(self, ctx, mocker, fake_repo_tree):
        _patch_paths(mocker, fake_repo_tree)
        tv.write(ctx, "1.22.0-pre.1")
        assert str(tv.bump(ctx)) == "1.22.0-pre.2"

    def test_finalise_drops_prerelease_component(
        self, ctx, mocker, fake_repo_tree
    ):
        result = self._bump(
            ctx, mocker, fake_repo_tree, "1.21.0-pre.56", [tv.BumpType.FINALISE]
        )
        assert result == "1.21.0"

    def test_bump_persists_new_version_to_plugin_json(
        self, ctx, mocker, fake_repo_tree
    ):
        self._bump(ctx, mocker, fake_repo_tree, "1.21.0", [tv.BumpType.PRE])
        plugin_json = json.loads(
            (fake_repo_tree / ".claude-plugin/plugin.json").read_text()
        )
        assert plugin_json["version"] == "1.22.0-pre.1"


# ── [lints.clippy] templating + edition sync ──────────────────────────


class TestLintsTemplating:
    """Guard the tomlkit round-trip that 0098's clippy config depends on."""

    def test_render_cargo_toml_preserves_lints_table(
        self, mocker, fake_repo_tree
    ):
        cargo_path = (
            fake_repo_tree / "skills/visualisation/visualise/server/Cargo.toml"
        )
        # Bespoke input: a [lints.clippy] table AND an inline rationale comment.
        cargo_path.write_text(
            "[package]\n"
            'name = "accelerator-visualiser"\n'
            'version = "1.20.0"\n'
            'edition = "2021"\n\n'
            "[lints.clippy]\n"
            'pedantic = { level = "warn", priority = -1 }\n'
            'missing_errors_doc = "allow"  # why: no Errors doc mandated\n'
        )
        mocker.patch.object(tv, "CARGO_TOML", cargo_path)
        result = tv._render_cargo_toml("1.21.0")
        # The table and its values survive the round-trip...
        assert "[lints.clippy]" in result
        assert 'pedantic = { level = "warn", priority = -1 }' in result
        # ...and — the load-bearing assertion — so does the verbatim comment.
        # (A plain dict-based TOML writer would keep the table but DROP this; it
        # is the property the justification-comment policy depends on.)
        assert "# why: no Errors doc mandated" in result
        assert 'version = "1.21.0"' in result

    def test_cargo_and_rustfmt_editions_match(self):
        # Cargo.toml [package].edition and rustfmt.toml edition are two
        # hand-duplicated literals; this guards the drift hazard that would let
        # a direct-rustfmt caller silently fall back to edition 2015.
        cargo = tomllib.loads((SERVER_DIR / "Cargo.toml").read_text())
        rustfmt = tomllib.loads((SERVER_DIR / "rustfmt.toml").read_text())
        assert cargo["package"]["edition"] == rustfmt["edition"]

    def test_cli_cargo_and_rustfmt_editions_match(self):
        # The workspace edition and the rustfmt edition are two hand-duplicated
        # literals; same drift hazard as the server pair above (a direct-rustfmt
        # caller silently falling back to edition 2015), guarded here now both
        # operands exist.
        cargo = tomllib.loads((CLI_DIR / "Cargo.toml").read_text())
        rustfmt = tomllib.loads((CLI_DIR / "rustfmt.toml").read_text())
        assert cargo["workspace"]["package"]["edition"] == rustfmt["edition"]
