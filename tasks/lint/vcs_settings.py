"""Guard: no code in cli/vcs-adapters constructs a jj ``UserSettings``.

``Workspace::load`` needs a fully-populated ``UserSettings`` whose defaults are
private to jj-lib and were discovered one panic at a time. The root/kind/
revision detection paths need neither: ``DefaultWorkspaceLoaderFactory`` is
public, and the checkout state and operation store are both reachable without
settings.

One narrow, deliberate exception: ``library/dirty_paths.rs``'s jj snapshot
genuinely cannot avoid ``UserSettings`` — snapshotting is not a read of
already-recorded state (what the settings-free routes above cover), it is
jj-lib re-deriving on-disk changes since the last operation, and that requires
a real ``TreeStateSettings`` (conflict-marker style, eol/exec-bit handling,
fsmonitor backend) that only ``UserSettings`` supplies. Confirmed against
jj-lib 0.43.0 and the real `jj` CLI's own snapshot path — there is no
lower-ceremony construction that reaches ``LockedWorkingCopy::snapshot``.

Not a cargo-pup ``denied`` clause because ``RestrictImports`` resolves ``use``
paths, so it cannot see a fully-qualified
``jj_lib::settings::UserSettings::from_config(...)`` or a ``Workspace::load``
method call. cargo-pup owns import prohibitions; this owns usage prohibitions.

Comments are stripped before matching, so the crate can document why it avoids
these without flagging itself.
"""

import re
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.sources import repo_root

CRATE = "cli/vcs-adapters"

_EXEMPT = frozenset({f"{CRATE}/src/library/dirty_paths.rs"})

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
        if rel in _EXEMPT:
            continue
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
