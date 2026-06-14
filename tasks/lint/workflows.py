import shlex

from invoke import Context, Exit, task

from tasks.shared.paths import REPO_ROOT

_WORKFLOW = ".github/workflows/main.yml"

# actionlint's bundled schema predates the GA `concurrency.queue` key (shipped
# 2026-05-07) and rejects `queue: max` as an unknown sub-key. That key is valid
# and deliberate (see .github/workflows/main.yml). actionlint's role here is
# general workflow-syntax hygiene, NOT concurrency-sub-key validation — the
# parser test in tests/unit/tasks/test_workflows.py is the guard for the
# queue/gate-placement invariants — so suppress only that one stale-schema
# false positive and let every other finding fail loud. Drop this once
# actionlint's schema learns the `queue` key.
_QUEUE_SCHEMA_LAG = 'unexpected key "queue" for "concurrency" section'


@task
def actionlint(context: Context) -> None:
    """Lint GitHub Actions workflows with actionlint (fail-loud)."""
    ignore = shlex.quote(_QUEUE_SCHEMA_LAG)
    with context.cd(str(REPO_ROOT)):
        result = context.run(
            f"actionlint -ignore {ignore} {_WORKFLOW}", warn=True, pty=False
        )
    if result.exited != 0:
        raise Exit("actionlint reported findings — fix them", code=1)
