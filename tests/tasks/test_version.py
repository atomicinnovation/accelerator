import json
from pathlib import Path
from unittest.mock import MagicMock

import pytest

from invoke import Context

import tasks.build as tb
import tasks.version as tv
from tasks.build import validate_version_coherence


@pytest.fixture
def ctx():
    m = MagicMock(spec=Context)
    m.run.return_value = MagicMock(return_code=0, stdout="")
    return m


def _patch_paths(mocker, base: Path) -> None:
    mocker.patch.object(tv, "PLUGIN_JSON", base / ".claude-plugin/plugin.json")
    mocker.patch.object(tv, "CARGO_TOML",
                        base / "skills/visualisation/visualise/server/Cargo.toml")
    mocker.patch.object(tv, "CHECKSUMS",
                        base / "skills/visualisation/visualise/bin/checksums.json")


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
            (fake_repo_tree / "skills/visualisation/visualise/bin/checksums.json").read_text()
        )

        assert plugin_json["version"] == "1.21.0"
        assert 'version = "1.21.0"' in cargo_toml
        assert checksums["version"] == "1.21.0"

    def test_checksums_binaries_map_preserved(self, ctx, mocker, fake_repo_tree):
        _patch_paths(mocker, fake_repo_tree)
        checksums_before = json.loads(
            (fake_repo_tree / "skills/visualisation/visualise/bin/checksums.json").read_text()
        )
        tv.write(ctx, "1.21.0")
        checksums_after = json.loads(
            (fake_repo_tree / "skills/visualisation/visualise/bin/checksums.json").read_text()
        )
        assert checksums_after["binaries"] == checksums_before["binaries"]

    def test_cargo_toml_structure_preserved(self, ctx, mocker, fake_repo_tree):
        cargo_path = fake_repo_tree / "skills/visualisation/visualise/server/Cargo.toml"
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
            "plugin_json": (fake_repo_tree / ".claude-plugin/plugin.json").read_bytes(),
            "cargo_toml": (fake_repo_tree / "skills/visualisation/visualise/server/Cargo.toml").read_bytes(),
            "checksums": (fake_repo_tree / "skills/visualisation/visualise/bin/checksums.json").read_bytes(),
        }
        tv.write(ctx, "1.21.0")
        content_after_second = {
            "plugin_json": (fake_repo_tree / ".claude-plugin/plugin.json").read_bytes(),
            "cargo_toml": (fake_repo_tree / "skills/visualisation/visualise/server/Cargo.toml").read_bytes(),
            "checksums": (fake_repo_tree / "skills/visualisation/visualise/bin/checksums.json").read_bytes(),
        }
        assert content_after_first == content_after_second

    def test_coherence_passes_after_write(self, ctx, mocker, fake_repo_tree):
        _patch_paths(mocker, fake_repo_tree)
        mocker.patch.object(tb, "REPO_ROOT", fake_repo_tree)
        tv.write(ctx, "1.21.0")
        validate_version_coherence("1.21.0", repo_root=fake_repo_tree)
