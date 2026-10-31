"""Real jj and git repositories in a test's tmp_path, isolated from the
contributor's own configuration."""

import os
import subprocess
from collections.abc import Mapping
from pathlib import Path

import pytest

from tests.support.tools import require


def hermetic_environment(tmp_path: Path) -> dict[str, str]:
    empty_config = tmp_path / "empty-jj-config.toml"
    empty_config.touch()
    return {
        "GIT_CONFIG_NOSYSTEM": "1",
        "GIT_CONFIG_GLOBAL": os.devnull,
        "GIT_AUTHOR_NAME": "Test",
        "GIT_AUTHOR_EMAIL": "test@example.com",
        "GIT_COMMITTER_NAME": "Test",
        "GIT_COMMITTER_EMAIL": "test@example.com",
        "GIT_CEILING_DIRECTORIES": str(tmp_path),
        "JJ_CONFIG": str(empty_config),
        "JJ_USER": "Test",
        "JJ_EMAIL": "test@example.com",
    }


def isolate(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    for name, value in hermetic_environment(tmp_path).items():
        monkeypatch.setenv(name, value)


def _run(argv: list[str], root: Path) -> str:
    return subprocess.run(
        argv, cwd=root, check=True, capture_output=True, text=True
    ).stdout.strip()


def _write(root: Path, files: Mapping[str, str]) -> None:
    for relative, content in files.items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)


class JjRepository:
    def __init__(self, root: Path) -> None:
        require("jj")
        self.root = root
        root.mkdir(parents=True, exist_ok=True)
        _run(["jj", "git", "init", "--no-colocate", str(root)], root)

    def commit(self, files: Mapping[str, str], message: str) -> str:
        _write(self.root, files)
        _run(["jj", "commit", "-m", message], self.root)
        return self._parent_of_working_copy()

    def checkout(self, *revisions: str) -> None:
        _run(["jj", "new", *revisions], self.root)

    def merge(self, left: str, right: str) -> str:
        self.checkout(left, right)
        _run(["jj", "commit", "-m", f"merge {right} into {left}"], self.root)
        return self._parent_of_working_copy()

    def _parent_of_working_copy(self) -> str:
        return _run(
            ["jj", "log", "--no-graph", "-r", "@-", "-T", "commit_id"],
            self.root,
        )


class GitRepository:
    def __init__(self, root: Path) -> None:
        require("git")
        self.root = root
        root.mkdir(parents=True, exist_ok=True)
        _run(["git", "init", "--quiet", str(root)], root)

    def commit(self, files: Mapping[str, str], message: str) -> str:
        _write(self.root, files)
        _run(["git", "add", "--all"], self.root)
        _run(
            ["git", "commit", "--quiet", "--allow-empty", "-m", message],
            self.root,
        )
        return self._head()

    def checkout(self, revision: str) -> None:
        _run(["git", "checkout", "--quiet", "--detach", revision], self.root)

    def merge(self, left: str, right: str) -> str:
        self.checkout(left)
        _run(
            ["git", "merge", "--quiet", "--no-ff", "-m", "merge", right],
            self.root,
        )
        return self._head()

    def _head(self) -> str:
        return _run(["git", "rev-parse", "HEAD"], self.root)
