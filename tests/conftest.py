from pathlib import Path

import pytest


@pytest.fixture
def fake_repo_tree(tmp_path: Path) -> Path:
    (tmp_path / ".claude-plugin").mkdir()
    (tmp_path / ".claude-plugin/plugin.json").write_text(
        '{"name":"accelerator","version":"1.20.0"}'
    )
    cli_dir = tmp_path / "cli"
    cli_dir.mkdir()
    (cli_dir / "Cargo.toml").write_text(
        "[workspace]\n"
        'members = ["launcher", "visualiser/server"]\n\n'
        "[workspace.package]\n"
        'version = "1.20.0"\n'
    )
    launcher_dir = cli_dir / "launcher"
    launcher_dir.mkdir()
    (launcher_dir / "Cargo.toml").write_text(
        '[package]\nname = "launcher"\nversion.workspace = true\n'
    )
    server_dir = cli_dir / "visualiser/server"
    server_dir.mkdir(parents=True)
    (server_dir / "Cargo.toml").write_text(
        '[package]\nname = "accelerator-visualiser"\nversion.workspace = true\n'
    )
    return tmp_path
