"""Guard: no code in cli/vcs-adapters constructs a jj ``UserSettings``.

``jj_lib::workspace::Workspace::load`` needs a fully-populated ``UserSettings``
whose defaults are private to jj-lib and were discovered one panic at a time —
the attempt was abandoned after five successive panics with the chain never
exhausted. ``DefaultWorkspaceLoaderFactory`` is public and
``WorkspaceLoader::{workspace_root, repo_path}`` need no settings, so nothing in
the detection paths requires it.

The statement is crate-wide, deliberately wider than the detection paths
strictly need, so the guard is a simple one. A spike established that jj-lib
0.43 offers no read-only, settings-free route to the working-copy commit id
either, which is why the jj half of ``revision`` is out of scope rather than a
reason to narrow this.

**Why this is not a cargo-pup ``denied`` clause.** ``RestrictImports`` resolves
``use`` paths, so it cannot see a fully-qualified
``jj_lib::settings::UserSettings::from_config(...)`` or a ``Workspace::load``
method call. cargo-pup owns import prohibitions; this owns the usage
prohibitions imports cannot express.

Comments are stripped before matching. The model this copies matches a regex
against every raw line, which would make it impossible to document *why*
``UserSettings`` is avoided in the very crate the guard covers — and that reason
is exactly the kind of extremely-non-obvious fact this repo's comment bar
admits. Without the strip, an implementer hits the self-flagging problem
immediately and works around it by mangling the word.
"""

import re
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.sources import repo_root

CRATE = "cli/vcs-adapters"

# The two constructs the work item forbids by name.
_FORBIDDEN = re.compile(r"\bUserSettings\b|\bWorkspace::load\b")

_LINE_COMMENT = re.compile(r"//.*$")
_BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.DOTALL)


def strip_comments(source: str) -> str:
    """``source`` with block and line comments blanked, lines preserved.

    Newlines survive so reported line numbers still point at the real line.
    """
    without_blocks = _BLOCK_COMMENT.sub(
        lambda match: "\n" * match.group().count("\n"), source
    )
    return "\n".join(
        _LINE_COMMENT.sub("", line) for line in without_blocks.splitlines()
    )


def violations(root: Path) -> list[str]:
    """Repo-relative ``path:line`` for each forbidden construct in the crate."""
    found: list[str] = []
    for path in sorted((root / CRATE).rglob("*.rs")):
        rel = path.relative_to(root).as_posix()
        source = strip_comments(path.read_text())
        for number, line in enumerate(source.splitlines(), start=1):
            if _FORBIDDEN.search(line):
                found.append(f"{rel}:{number}")
    return found


@task
def check(context: Context) -> None:
    """Fail if cli/vcs-adapters names UserSettings or Workspace::load."""
    offenders = violations(repo_root())
    if offenders:
        raise Exit(
            "cli/vcs-adapters must not construct a jj UserSettings or call "
            "Workspace::load — its defaults are private to jj-lib and the "
            "detection paths need neither. Use "
            "DefaultWorkspaceLoaderFactory and WorkspaceLoader instead:\n  "
            + "\n  ".join(offenders),
            code=1,
        )
