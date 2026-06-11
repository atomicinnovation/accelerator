import shlex

from invoke import Context, Exit, task
from invoke.runners import Result

from tasks.shared.sources import repo_root, shell_sources


def _shfmt(context: Context, op_flags: str) -> Result:
    """Run shfmt over every shell source with only operation flags.

    No formatting flags (-i, -ci, …) are passed so shfmt reads `.editorconfig`
    as the single source of truth for style — passing CLI style flags would
    make shfmt ignore EditorConfig entirely and reintroduce local-vs-CI drift.
    """
    sources = shell_sources()
    if not sources:
        # Fail loudly, not green: an empty match set means scope discovery broke
        # (a glob/`_keep` regression), not that there is nothing to format
        # (fail-closed, not fail-open). Mirrors the lint tasks' guard.
        raise Exit(
            "shfmt: no shell sources matched — scope discovery is broken",
            code=1,
        )
    args = " ".join(shlex.quote(s) for s in sources)
    # mise runs invoke tasks from the repo root and shell_sources() returns
    # repo-relative paths, so no cwd override is needed (invoke's run() does
    # not accept one anyway).
    with context.cd(str(repo_root())):
        return context.run(f"shfmt {op_flags} {args}", warn=True, pty=False)


@task
def check(context: Context) -> None:
    """Report shell files that are not shfmt-formatted (read-only)."""
    result = _shfmt(context, "-d")
    if result.exited != 0:
        raise Exit(
            "shfmt: formatting drift detected — run "
            "`mise run format:scripts:fix`",
            code=1,
        )


@task
def fix(context: Context) -> None:
    """Format shell files in place with shfmt, listing the changed files."""
    _shfmt(context, "-l -w")
