"""Coverage guard for the Python lint / type-check file set.

Unlike shell — where `shell_sources()` is a single inspectable function with its
own regression test — ruff's `extend-exclude` and pyrefly's `project-excludes`
drive file discovery implicitly, so a mis-scoped exclude could silently leave
files unchecked while every command still exits 0 (a vacuous pass). This turns
that risk into a standing guard with two parts:

1. Config-set assertion: the configured excludes equal exactly the justified set
   (no silent additions), and the in-scope `.py` walk is non-empty.
2. Sentinel probe: a deliberate violation written at a real in-scope path is
   actually reported by `ruff check` / `pyrefly check` run with no path args —
   proving config-driven discovery reaches that location (not merely that the
   CLI lints a named file).

The walk is VCS-agnostic (the same gitignore-honouring approach as
`shell_sources`), NOT `git ls-files '*.py'` — that is blind in a jj workspace,
making this guard vacuous/spurious locally.
"""

import os
import shutil
import subprocess
import tomllib
from pathlib import Path

import pytest

from tasks.shared.sources import _ignore_spec, repo_root

REPO = repo_root()
MOCK_JIRA = "skills/integrations/jira/scripts/test-helpers/mock-jira-server.py"
MOCK_LINEAR = (
    "skills/integrations/linear/scripts/test-helpers/mock-linear-server.py"
)

# The justified excludes — kept in lockstep with pyproject.toml. The point of
# pinning them here is that adding a NEW exclude must also change this test, so
# no file silently drops out of coverage.
RUFF_JUSTIFIED_EXCLUDES = {"workspaces", MOCK_JIRA, MOCK_LINEAR}
PYREFLY_JUSTIFIED_EXCLUDES = {
    "**/workspaces/**",
    "**/.venv/**",
    f"**/{MOCK_JIRA}",
    f"**/{MOCK_LINEAR}",
    "**/tests/**",
    # JS dep trees hold no first-party Python; pyrefly ignores .gitignore, so
    # without this it walks node_modules and races with `deps:install:node`.
    "**/node_modules/**",
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
    pyrefly via `project-excludes`, but `.venv` is not in `.gitignore`, so it is
    pruned here explicitly alongside the gitignore-matched directories.
    """
    spec = _ignore_spec(REPO)
    out: set[str] = set()
    for dirpath, dirnames, filenames in os.walk(REPO):
        rel_dir = Path(dirpath).relative_to(REPO)
        dirnames[:] = [
            d
            for d in dirnames
            if d != ".venv"
            and not spec.match_file(
                f"{d}/" if rel_dir == Path() else f"{rel_dir / d}/"
            )
        ]
        for name in filenames:
            if not name.endswith(".py"):
                continue
            rel = name if rel_dir == Path() else str(rel_dir / name)
            if not spec.match_file(rel):
                out.add(rel)
    return out


def _tool(name: str) -> str:
    path = shutil.which(name)
    if path is None:
        pytest.skip(f"{name} not on PATH (run via `mise run test:unit:tasks`)")
    return path


class TestConfiguredExcludes:
    def test_ruff_extend_exclude_is_exactly_justified(self):
        cfg = _pyproject()
        configured = set(cfg["tool"]["ruff"]["extend-exclude"])
        assert configured == RUFF_JUSTIFIED_EXCLUDES

    def test_pyrefly_project_excludes_is_exactly_justified(self):
        cfg = _pyproject()
        configured = set(cfg["tool"]["pyrefly"]["project-excludes"])
        assert configured == PYREFLY_JUSTIFIED_EXCLUDES


class TestInScopeSet:
    def test_walk_nonempty_and_excludes_only_justified(self):
        py = _py_files()
        assert py, "no .py files discovered — the walk is broken"
        # A core build-system module is in scope.
        assert "tasks/build.py" in py
        # The 3.9-floor mock servers are in the tree but excluded from ruff.
        assert MOCK_JIRA in py
        assert MOCK_LINEAR in py
        # workspaces/ is gitignored, so the walk never surfaces it.
        assert not any(p.startswith("workspaces/") for p in py)
        ruff_in_scope = py - {MOCK_JIRA, MOCK_LINEAR}
        assert ruff_in_scope, "ruff in-scope set is empty after excludes"
        assert MOCK_JIRA not in ruff_in_scope
        assert MOCK_LINEAR not in ruff_in_scope


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
