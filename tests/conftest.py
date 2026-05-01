import shutil
from pathlib import Path

import pytest

_REPO_ROOT = Path(__file__).resolve().parent.parent
_TASKS_FIXTURES = _REPO_ROOT / "tests/tasks/fixtures"


@pytest.fixture
def fake_repo_tree(tmp_path: Path) -> Path:
    (tmp_path / ".claude-plugin").mkdir()
    (tmp_path / ".claude-plugin/plugin.json").write_text(
        '{"name":"accelerator","version":"1.20.0"}'
    )
    cargo_dir = tmp_path / "skills/visualisation/visualise/server"
    cargo_dir.mkdir(parents=True)
    (cargo_dir / "Cargo.toml").write_text(
        '[package]\nname = "x"\nversion = "1.20.0"\n'
    )
    bin_dir = tmp_path / "skills/visualisation/visualise/bin"
    bin_dir.mkdir(parents=True)
    shutil.copy(_TASKS_FIXTURES / "checksums.example.json",
                bin_dir / "checksums.json")
    return tmp_path
