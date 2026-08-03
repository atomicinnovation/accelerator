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
CLI_DIR = REPO_ROOT / "cli"


@pytest.fixture
def ctx():
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(return_code=0, stdout="")
    return m


def _patch_paths(mocker, base: Path) -> None:
    mocker.patch.object(tv, "PLUGIN_JSON", base / ".claude-plugin/plugin.json")
    mocker.patch.object(tv, "CLI_WORKSPACE_CARGO_TOML", base / "cli/Cargo.toml")


# ── write() ───────────────────────────────────────────────────────────


class TestWrite:
    def test_updates_plugin_version(self, ctx, mocker, fake_repo_tree):
        _patch_paths(mocker, fake_repo_tree)
        tv.write(ctx, "1.21.0")

        plugin_json = json.loads(
            (fake_repo_tree / ".claude-plugin/plugin.json").read_text()
        )

        assert plugin_json["version"] == "1.21.0"

    def test_idempotent(self, ctx, mocker, fake_repo_tree):
        _patch_paths(mocker, fake_repo_tree)
        tv.write(ctx, "1.21.0")
        content_after_first = {
            "plugin_json": (
                fake_repo_tree / ".claude-plugin/plugin.json"
            ).read_bytes(),
            "workspace": (fake_repo_tree / "cli/Cargo.toml").read_bytes(),
        }
        tv.write(ctx, "1.21.0")
        content_after_second = {
            "plugin_json": (
                fake_repo_tree / ".claude-plugin/plugin.json"
            ).read_bytes(),
            "workspace": (fake_repo_tree / "cli/Cargo.toml").read_bytes(),
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

    def test_syncs_the_cargo_lock(self, ctx, mocker, fake_repo_tree):
        # The lock carries a copy of the version per workspace member, so a
        # manifest-only write leaves them disagreeing — and clippy runs
        # `--locked`. `cargo metadata` is the minimal update; generate-lockfile
        # would re-resolve the whole closure.
        _patch_paths(mocker, fake_repo_tree)
        tv.write(ctx, "1.21.0")

        commands = [call.args[0] for call in ctx.run.call_args_list]
        assert any(
            "cargo metadata" in command and "cli/Cargo.toml" in command
            for command in commands
        ), f"write() did not sync the lock: {commands}"
        assert not any("generate-lockfile" in c for c in commands), commands


class TestLockVersionCoherence:
    """The lock's workspace-member versions must match the manifest.

    This is the guard for the drift the sync above prevents. It reads the real
    files and needs no cargo, so it fails in `test:unit:tasks` with a named
    diagnostic rather than as a clippy `--locked` complaint in a Rust job, far
    from the bump that caused it.
    """

    @staticmethod
    def _member_package_names() -> set[str]:
        # From each member's own [package].name, not its directory: three of
        # them differ (launcher -> accelerator, verify -> accelerator-verify,
        # visualiser/server -> accelerator-visualiser), and the lock keys on the
        # package name.
        cargo = tomllib.loads((CLI_DIR / "Cargo.toml").read_text())
        return {
            tomllib.loads((CLI_DIR / member / "Cargo.toml").read_text())[
                "package"
            ]["name"]
            for member in cargo["workspace"]["members"]
        }

    @staticmethod
    def _locked_path_packages() -> dict[str, str]:
        # Workspace members are the lock entries with no `source` — registry and
        # git packages both carry one.
        lock = tomllib.loads((CLI_DIR / "Cargo.lock").read_text())
        return {
            package["name"]: package["version"]
            for package in lock["package"]
            if "source" not in package
        }

    def test_every_member_entry_matches_the_workspace_version(self):
        cargo = tomllib.loads((CLI_DIR / "Cargo.toml").read_text())
        expected = cargo["workspace"]["package"]["version"]

        # Every member inherits `version.workspace = true`, with no exceptions,
        # so all of them must read alike — including accelerator-visualiser.
        stale = {
            name: version
            for name, version in self._locked_path_packages().items()
            if version != expected
        }
        assert not stale, (
            f"cli/Cargo.lock disagrees with the workspace version {expected}: "
            f"{stale}. Run `cargo metadata --manifest-path cli/Cargo.toml` to "
            "sync it (the minimal update), never `cargo generate-lockfile`."
        )

    def test_the_assertion_covers_every_member(self):
        # Non-vacuity: a lock that stopped listing members, or a member renamed
        # out of it, would leave the assertion above passing over a smaller set
        # than it claims. Equality both ways, so a member added to the workspace
        # without reaching the lock fails here too.
        assert self._locked_path_packages().keys() == (
            self._member_package_names()
        )

    def test_every_member_inherits_the_workspace_version(self):
        # The premise of the exact comparison above. A member that pinned its
        # own version would legitimately differ, and would have to be excluded
        # rather than silently failing.
        # `version.workspace = true` parses to {"workspace": True}; a member
        # that pins its own writes a plain string. That is the discriminator —
        # not presence, which every member satisfies either way.
        cargo = tomllib.loads((CLI_DIR / "Cargo.toml").read_text())
        pinned = [
            member
            for member in cargo["workspace"]["members"]
            if isinstance(
                tomllib.loads((CLI_DIR / member / "Cargo.toml").read_text())[
                    "package"
                ].get("version"),
                str,
            )
        ]
        assert not pinned, (
            f"these members pin their own version: {pinned}. The lock "
            "coherence assertion assumes workspace inheritance throughout."
        )


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

    def test_cli_cargo_and_rustfmt_editions_match(self):
        # The workspace edition and the rustfmt edition are two hand-duplicated
        # literals; same drift hazard as the server pair above (a direct-rustfmt
        # caller silently falling back to edition 2015), guarded here now both
        # operands exist.
        cargo = tomllib.loads((CLI_DIR / "Cargo.toml").read_text())
        rustfmt = tomllib.loads((CLI_DIR / "rustfmt.toml").read_text())
        assert cargo["workspace"]["package"]["edition"] == rustfmt["edition"]
