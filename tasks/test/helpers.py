import os
from pathlib import Path

from invoke import Context

EXCLUDED_HELPER_NAMES = {"test-helpers.sh"}


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def run_shell_suites(context: Context, subtree: str) -> None:
    """Glob-discover and run every executable test-*.sh inside a subtree.

    The exec-bit filter excludes `scripts/test-helpers.sh` (sourced,
    not run); the name-level filter is belt-and-braces for
    filesystems that synthesise exec bits uniformly.
    """
    repo = repo_root()
    root = repo / subtree
    if not root.exists():
        return
    suites = sorted(
        p.relative_to(repo).as_posix()
        for p in root.glob("**/test-*.sh")
        if p.is_file()
        and p.name not in EXCLUDED_HELPER_NAMES
        and os.access(p, os.X_OK)
    )
    for suite in suites:
        print(f"Running {suite}...")
        context.run(suite)
        print()
