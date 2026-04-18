import os
from pathlib import Path

from invoke import Context, task

# Helper files that happen to match the `test-*.sh` pattern but are
# sourced, not run. Belt-and-braces alongside the executable-bit
# filter: the name check catches cases where exec bits are synthesised
# uniformly (e.g. WSL-mounted NTFS) and the exec-bit check catches
# contributors who forget to `chmod +x` a new suite.
EXCLUDED_HELPER_NAMES = {"test-helpers.sh"}


@task
def integration(context: Context):
    """Run integration tests — auto-discovers every executable test-*.sh."""
    repo = Path(__file__).resolve().parent.parent
    suites = sorted(
        p.relative_to(repo).as_posix()
        for p in repo.glob("**/test-*.sh")
        if p.is_file()
        and p.name not in EXCLUDED_HELPER_NAMES
        and os.access(p, os.X_OK)
    )
    if not suites:
        raise RuntimeError("No executable test-*.sh suites discovered")
    for suite in suites:
        print(f"Running {suite}...")
        context.run(suite)
        print()
