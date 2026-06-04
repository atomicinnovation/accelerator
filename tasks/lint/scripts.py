import shlex

from invoke import Context, Exit, task

from tasks.shared.sources import repo_root, shell_sources


def _sources_args() -> str | None:
    sources = shell_sources()
    if not sources:
        return None
    return " ".join(shlex.quote(s) for s in sources)


@task
def shellcheck(context: Context):
    """Lint every shell source with ShellCheck (-x, --severity=warning)."""
    args = _sources_args()
    if args is None:
        return
    with context.cd(str(repo_root())):
        result = context.run(
            f"shellcheck -x --severity=warning {args}", warn=True, pty=False
        )
    if result.exited != 0:
        raise Exit("shellcheck reported findings", code=1)


@task
def bashisms(context: Context):
    """Guard the bash-3.2 floor by scanning for a denylist of bash-4 constructs."""
    args = _sources_args()
    if args is None:
        return
    with context.cd(str(repo_root())):
        result = context.run(f"bash scripts/lint-bashisms.sh {args}", warn=True, pty=False)
    if result.exited != 0:
        raise Exit("lint-bashisms found bash-4 constructs", code=1)
