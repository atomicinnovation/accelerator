import shutil
from pathlib import Path

import pytest

_REPO_ROOT = Path(__file__).resolve().parent.parent
_TASKS_FIXTURES = _REPO_ROOT / "tests/unit/tasks/fixtures"


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
    bin_dir = tmp_path / "skills/visualisation/visualise/bin"
    bin_dir.mkdir(parents=True)
    shutil.copy(
        _TASKS_FIXTURES / "checksums.example.json", bin_dir / "checksums.json"
    )
    return tmp_path
