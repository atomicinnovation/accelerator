import os
from pathlib import Path

from invoke import Context

# test-helpers.sh is sourced, not run. test-jira-scripts.sh is an umbrella
# runner that sequentially re-invokes every individual test-jira-*.sh suite;
# under per-file discovery it would run the whole Jira subtree a second time, so
# it is excluded and the individual suites gate on their own.
EXCLUDED_HELPER_NAMES = {"test-helpers.sh", "test-jira-scripts.sh"}


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def ensure_accelerator_bin() -> None:
    """Point ACCELERATOR_BIN/CLAUDE_PLUGIN_ROOT at the pre-built debug launcher.

    Repointed production shell scripts read config through
    ``"${ACCELERATOR_BIN:-$PLUGIN_ROOT/bin/accelerator}" config …``. Setting
    ACCELERATOR_BIN keeps a subtree's suites on the compiled binary rather than
    the signed-release bootstrap; children of ``run_shell_suites`` inherit it.
    CLAUDE_PLUGIN_ROOT is exported for template resolution.

    The launcher itself is produced by the ``build:cli:dev`` mise dependency of
    the test tasks, so build ordering lives in the task graph rather than in an
    ad-hoc cargo call here. A caller-supplied ACCELERATOR_BIN (a stub or a
    release build) is honoured.
    """
    repo = repo_root()
    os.environ.setdefault(
        "ACCELERATOR_BIN",
        str(repo / "cli" / "target" / "debug" / "accelerator"),
    )
    os.environ.setdefault("CLAUDE_PLUGIN_ROOT", str(repo))


def run_shell_suites(context: Context, subtree: str) -> list[str]:
    """Glob-discover and run every executable test-*.sh inside a subtree.

    The exec-bit filter excludes `scripts/test-helpers.sh` (sourced,
    not run); the name-level filter is belt-and-braces for
    filesystems that synthesise exec bits uniformly.

    Returns the sorted list of suites that were discovered and run, so a
    caller can assert a non-zero discovery count and fail loudly if an
    exec bit was dropped (e.g. on an exec-bit-lossy filesystem) rather
    than silently skipping its regression net.
    """
    repo = repo_root()
    root = repo / subtree
    if not root.exists():
        return []
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
    return suites
