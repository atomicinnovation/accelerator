"""Guard against a direct git token in any ``skills/**/SKILL.md``.

Every skill expresses VCS operations in backend-neutral terms — the session's
VCS command, or the SessionStart VCS Command Reference — never a raw ``git``
subcommand, which breaks in a pure-jj checkout with no ``.git``. This scans each
SKILL.md line for ``git <subcommand>`` from the blocklist and fails with the
offending file:line.

Scoped to ``SKILL.md`` only: ``evals/*.json`` fixtures narrate ``git config
user.name`` as historical prose, not a command invocation, so they are out of
scope by construction. The blocklist is git-only by design — a blanket jj sweep
would false-positive on the ``jj restore`` that ``update-work-item``
deliberately keeps.
"""

import re
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.sources import repo_root

_GIT_SUBCOMMAND = re.compile(
    r"\bgit (status|diff|add|commit|log|branch|checkout|switch|merge|rebase|"
    r"reset|stash|show|rev-parse|config)\b"
)


def violations(root: Path) -> list[str]:
    """Every direct git-token breach across ``skills/**/SKILL.md``.

    One ``<rel>:<line>: <token>`` string per hit.
    """
    found: list[str] = []
    for path in sorted((root / "skills").rglob("SKILL.md")):
        rel = path.relative_to(root).as_posix()
        for number, line in enumerate(path.read_text().splitlines(), start=1):
            match = _GIT_SUBCOMMAND.search(line)
            if match:
                found.append(
                    f"{rel}:{number}: direct git token '{match.group()}' — "
                    "use the session's VCS command or the SessionStart "
                    "VCS Command Reference"
                )
    return found


@task
def check(context: Context) -> None:
    """Fail if any SKILL.md issues a direct git subcommand."""
    offenders = violations(repo_root())
    if offenders:
        raise Exit(
            "check-git-tokens found violation(s):\n  " + "\n  ".join(offenders),
            code=1,
        )
