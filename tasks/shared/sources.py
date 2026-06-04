"""Shared source-file discovery for the format and lint task families.

Both families scan an identical set of shell files (tracked `*.sh`, minus
fixtures, jj workspaces, and the sourced-only `test-helpers.sh`) so format and
lint never disagree about what is in scope.
"""

import subprocess
from pathlib import Path


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _keep(rel: str) -> bool:
    """True when a repo-relative `.sh` path should be formatted/linted."""
    if not rel:
        return False
    parts = rel.split("/")
    if "test-fixtures" in parts:
        return False
    if parts[0] == "workspaces":
        return False
    if parts[-1] == "test-helpers.sh":
        return False
    return True


def shell_sources(root: Path | None = None) -> list[str]:
    """Tracked `.sh` files (repo-relative, sorted) with the exclusion set applied.

    Uses `git ls-files` so only tracked files are scanned — untracked scratch
    scripts and ignored trees are skipped without bespoke globbing.
    """
    repo = root or repo_root()
    listed = subprocess.run(
        ["git", "ls-files", "*.sh"],
        cwd=repo,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return sorted(rel for rel in listed.splitlines() if _keep(rel))
