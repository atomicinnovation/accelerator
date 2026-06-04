import subprocess
from pathlib import Path

from tasks.shared.sources import _keep, shell_sources


def _git(cwd: Path, *args: str) -> None:
    subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True)


class TestKeepPredicate:
    def test_keeps_a_normal_script(self):
        assert _keep("scripts/foo.sh")

    def test_excludes_fixtures_at_any_depth(self):
        assert not _keep("skills/x/test-fixtures/seed.sh")
        assert not _keep("test-fixtures/a.sh")

    def test_excludes_workspaces(self):
        assert not _keep("workspaces/ws/a.sh")

    def test_excludes_test_helpers(self):
        assert not _keep("scripts/test-helpers.sh")


class TestShellSourcesDiscovery:
    def test_excludes_fixtures_workspaces_helpers_keeps_normal(self, tmp_path: Path):
        (tmp_path / "scripts").mkdir()
        (tmp_path / "scripts/normal.sh").write_text("#!/usr/bin/env bash\n")
        (tmp_path / "scripts/test-helpers.sh").write_text("#!/usr/bin/env bash\n")
        (tmp_path / "scripts/test-fixtures").mkdir()
        (tmp_path / "scripts/test-fixtures/seed.sh").write_text("#!/usr/bin/env bash\n")
        (tmp_path / "workspaces").mkdir()
        (tmp_path / "workspaces/ws.sh").write_text("#!/usr/bin/env bash\n")
        # A non-shell file must not appear regardless.
        (tmp_path / "scripts/readme.md").write_text("x\n")

        _git(tmp_path, "init", "-q")
        _git(tmp_path, "add", "-A")

        assert shell_sources(root=tmp_path) == ["scripts/normal.sh"]
