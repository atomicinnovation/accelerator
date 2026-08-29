"""Coverage guard for the Python lint / type-check file set.

Unlike the shell surface — an explicit two-file `SURVIVING_SHELL_SOURCES` list —
ruff's `extend-exclude` and pyrefly's `project-includes` drive file discovery
implicitly, so a mis-scoped exclude (ruff) or a too-narrow
include (pyrefly) could silently leave files unchecked while every command still
exits 0 (a vacuous pass). This turns that risk into a standing guard with two
parts:

1. Config-set assertion: ruff's excludes and pyrefly's include scope each equal
   exactly the justified set (no silent drift), and the in-scope `.py` walk is
   non-empty.
2. Sentinel probe: a deliberate violation written at a real in-scope path is
   actually reported by `ruff check` / `pyrefly check` run with no path args —
   proving config-driven discovery reaches that location (not merely that the
   CLI lints a named file).

The walk is VCS-agnostic (the same gitignore-honouring `walk_files` traversal),
NOT `git ls-files '*.py'` — that is blind in a jj workspace, making this guard
vacuous/spurious locally.
"""

import shutil
import subprocess
import tomllib
from pathlib import Path

import pytest

from tasks.shared.sources import repo_root, walk_files

REPO = repo_root()

# The justified scope — kept in lockstep with pyproject.toml. The point of
# pinning it here is that any change to discovery must also change this test, so
# no file silently drops out of coverage.
RUFF_JUSTIFIED_EXCLUDES = {"workspaces"}
# pyrefly scopes by include, not exclude: project-excludes only filters the
# matched-file set, it does not prune the directory walk, so it cannot keep
# pyrefly out of node_modules (which it would otherwise readdir while expanding
# `**/*.py*`, racing with `deps:install:node`). Rooting the walk at tasks/ — the
# sole first-party Python tree — avoids the walk entirely. Narrowing this set
# would silently drop files, so it is pinned.
PYREFLY_JUSTIFIED_INCLUDES = {
    "tasks/**/*.py",
    "tasks/**/*.pyi",
}

# A padded comment forces a ruff E501; the mistyped assignment forces a pyrefly
# bad-assignment. Either tool reporting this path proves discovery reached it.
_SENTINEL_SRC = (
    '"""Coverage sentinel — written into an isolated temp project."""\n'
    "# E501 padding xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
    "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\n"
    '_sentinel: int = "a str is not an int — pyrefly bad-assignment"\n'
)


def _pyproject() -> dict:
    return tomllib.loads((REPO / "pyproject.toml").read_text())


def _py_files() -> set[str]:
    """Repo-relative `.py` paths: gitignore-honoured, `.venv` pruned.

    Mirrors what ruff/pyrefly discover — ruff excludes `.venv` by default and
    pyrefly is scoped to `tasks/` via `project-includes`. `.venv` is kept out
    twice over, by `.gitignore` and by `walk_files`' prune list.
    """
    return {rel for rel in walk_files(REPO) if rel.endswith(".py")}


def _tool(name: str) -> str:
    path = shutil.which(name)
    if path is None:
        pytest.skip(f"{name} not on PATH (run via `mise run test:unit:tasks`)")
    return path


class TestConfiguredScope:
    def test_ruff_extend_exclude_is_exactly_justified(self):
        cfg = _pyproject()
        configured = set(cfg["tool"]["ruff"]["extend-exclude"])
        assert configured == RUFF_JUSTIFIED_EXCLUDES

    def test_pyrefly_project_includes_is_exactly_justified(self):
        cfg = _pyproject()
        configured = set(cfg["tool"]["pyrefly"]["project-includes"])
        assert configured == PYREFLY_JUSTIFIED_INCLUDES


class TestInScopeSet:
    def test_walk_nonempty_and_excludes_only_justified(self):
        py = _py_files()
        assert py, "no .py files discovered — the walk is broken"
        # A core build-system module is in scope.
        assert "tasks/build.py" in py
        # workspaces/ is gitignored, so the walk never surfaces it.
        assert not any(p.startswith("workspaces/") for p in py)
        # .venv holds thousands of vendored .py files; either the gitignore
        # entry or walk_files' prune alone is enough to keep them out.
        assert not any(p.startswith(".venv/") for p in py)


def _run_sentinel_probe(
    tool: str, tmp_path: Path
) -> subprocess.CompletedProcess:
    """Run `<tool> check` (config-driven, no path args) in an ISOLATED copy of
    the real config with a sentinel at an in-scope path.

    The probe must NOT write into the live `tasks/` tree: under `mise run`,
    `test:unit:tasks` runs concurrently with `lint:build-system:check` /
    `types:build-system:check`, which scan `tasks/` — an in-tree sentinel makes
    those tasks flake on the deliberate violation. Copying the real
    `pyproject.toml` into `tmp_path` exercises the SAME config's discovery,
    race-free.
    """
    binary = _tool(tool)
    shutil.copy(REPO / "pyproject.toml", tmp_path / "pyproject.toml")
    sentinel = tmp_path / "tasks" / "_sentinel.py"
    sentinel.parent.mkdir(parents=True)
    sentinel.write_text(_SENTINEL_SRC)
    return subprocess.run(
        [binary, "check"],
        cwd=tmp_path,
        capture_output=True,
        text=True,
        check=False,
    )


class TestSentinelDiscovery:
    """A sentinel at a real in-scope path must be found by config-only runs.

    Runs in an isolated temp project (the real config copied in) so it never
    races with a concurrent scan of the live `tasks/` tree under `mise run`.
    """

    def test_ruff_reports_in_scope_sentinel(self, tmp_path: Path):
        result = _run_sentinel_probe("ruff", tmp_path)
        assert result.returncode != 0
        assert "_sentinel.py" in result.stdout + result.stderr

    def test_pyrefly_reports_in_scope_sentinel(self, tmp_path: Path):
        result = _run_sentinel_probe("pyrefly", tmp_path)
        assert result.returncode != 0
        assert "_sentinel.py" in result.stdout + result.stderr
