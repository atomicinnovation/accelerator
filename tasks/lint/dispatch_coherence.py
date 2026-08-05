"""Guard the two-way binding between dispatch tokens and consuming skills.

The same check runs on the release path from ``tasks/manifest.py``. It rides a
lint leaf as well because failing at manifest generation means failing after
four cross-compiles and a use of the signing secret — a skill author should
learn about a broken binding on their PR instead.
"""

from invoke import Context, Exit, task

from tasks.shared.dispatch_coherence import violations
from tasks.shared.paths import (
    DISPATCHED_SUBBINARIES,
    SKILL_EXEMPT_SUBBINARIES,
)
from tasks.shared.sources import repo_root


@task
def check(context: Context) -> None:
    """Fail if any dispatched token is unbound, or any invocation unshipped."""
    offenders = violations(
        repo_root(),
        tokens=DISPATCHED_SUBBINARIES,
        exempt=SKILL_EXEMPT_SUBBINARIES,
    )
    if offenders:
        raise Exit(
            "check-dispatch-coherence found problem(s):\n  "
            + "\n  ".join(offenders),
            code=1,
        )
