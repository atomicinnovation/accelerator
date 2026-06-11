import shlex

from invoke import Context, Exit, task

from tasks.shared.sources import repo_root, shell_sources

# An empty match set means scope discovery broke (a glob/`_keep` regression),
# not that there is nothing to lint — so every task below fails loudly rather
# than passing green (fail-closed, not fail-open).
_EMPTY_SCOPE = "no shell sources matched — scope discovery is broken"


def _sources_args() -> str | None:
    sources = shell_sources()
    if not sources:
        return None
    return " ".join(shlex.quote(s) for s in sources)


@task
def shellcheck(context: Context) -> None:
    """Lint every shell source with ShellCheck (config in .shellcheckrc)."""
    args = _sources_args()
    if args is None:
        raise Exit(f"shellcheck: {_EMPTY_SCOPE}", code=1)
    with context.cd(str(repo_root())):
        result = context.run(f"shellcheck {args}", warn=True, pty=False)
    if result.exited != 0:
        raise Exit(
            "shellcheck reported findings — fix them, or add a justified "
            "`# shellcheck disable=`/`source=` directive",
            code=1,
        )


@task
def bashisms(context: Context) -> None:
    """Guard the bash-3.2 floor by scanning for denylisted bash-4 constructs."""
    args = _sources_args()
    if args is None:
        raise Exit(f"bashisms: {_EMPTY_SCOPE}", code=1)
    with context.cd(str(repo_root())):
        result = context.run(
            f"bash scripts/lint-bashisms.sh {args}", warn=True, pty=False
        )
    if result.exited != 0:
        raise Exit("lint-bashisms found bash-4 constructs", code=1)
