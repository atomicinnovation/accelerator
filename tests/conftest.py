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
    visualiser = tmp_path / "skills/visualisation/visualise"
    cargo_dir = visualiser / "server"
    cargo_dir.mkdir(parents=True)
    # Workspace root owns the single inherited version; the member manifest
    # inherits it (version.workspace = true) and carries no literal.
    (visualiser / "Cargo.toml").write_text(
        '[workspace]\nmembers = ["server"]\n\n'
        '[workspace.package]\nversion = "1.20.0"\nedition = "2021"\n'
    )
    (cargo_dir / "Cargo.toml").write_text(
        '[package]\nname = "x"\nversion.workspace = true\n'
    )
    bin_dir = tmp_path / "skills/visualisation/visualise/bin"
    bin_dir.mkdir(parents=True)
    shutil.copy(
        _TASKS_FIXTURES / "checksums.example.json", bin_dir / "checksums.json"
    )
    return tmp_path
