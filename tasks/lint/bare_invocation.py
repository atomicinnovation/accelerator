"""Guard the bare ``accelerator`` invocation convention (work item 0245).

Every ``skills/**/SKILL.md`` invokes the launcher as bare ``accelerator …``.
This guard forbids regressing to either alternative:

* the pathed form — ``${CLAUDE_PLUGIN_ROOT}/bin/accelerator`` or any absolute
  ``/…/bin/accelerator`` render — caught by the ``/bin/accelerator`` suffix in
  one substring;
* a ``bash`` / ``sh`` / ``env`` wrapper at the command position, which resolves
  the launcher differently and bypasses the uniform call convention.

The ``${CLAUDE_PLUGIN_ROOT}/skills/…`` lens and output-format references are not
launcher invocations — they carry ``/skills/``, not ``/bin/accelerator``, so
keying on the suffix spares them by construction. The wrapper rule anchors the
``bash``/``sh``/``env`` token to the start of the invocation (after stripping a
``!``-preprocessor or fence prefix), so prose that merely names the launcher —
or contains ``sh`` inside a word like ``publish`` — is not flagged.
"""

import re
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.skill_parsing import FORBIDDEN_LAUNCHER, PLUGIN_PREFIX
from tasks.shared.sources import repo_root

# The pathed launcher's tail, `/bin/accelerator`, derived from the named
# constant rather than re-typed. Kept as a suffix so it also catches an
# absolute-path render, not only the ${CLAUDE_PLUGIN_ROOT} prefix form.
_PATHED_SUFFIX = FORBIDDEN_LAUNCHER.removeprefix(PLUGIN_PREFIX.rstrip("/"))

# A launcher wrapped in a shell, anchored to the leading command token so prose
# naming the launcher is not caught. `accelerator` must be a whole word (a
# trailing separator or end of line), so the `accelerator-verify` sibling binary
# does not match.
_WRAPPER = re.compile(r"^(?:bash|sh|env)\b.*\baccelerator(?=\s|$)")


def _wraps_launcher(line: str) -> bool:
    stripped = line.lstrip().removeprefix("!`")
    return bool(_WRAPPER.search(stripped))


def violations(root: Path) -> list[str]:
    """Every bare-invocation breach across ``skills/**/SKILL.md``.

    One ``<rel>:<line>: <form>`` string per hit.
    """
    found: list[str] = []
    for path in sorted((root / "skills").rglob("SKILL.md")):
        rel = path.relative_to(root).as_posix()
        for number, line in enumerate(path.read_text().splitlines(), start=1):
            if _PATHED_SUFFIX in line:
                found.append(
                    f"{rel}:{number}: pathed launcher '{_PATHED_SUFFIX}' — "
                    "invoke bare 'accelerator'"
                )
            if _wraps_launcher(line):
                found.append(
                    f"{rel}:{number}: shell-wrapped launcher — invoke "
                    "'accelerator' directly, not through bash/sh/env"
                )
    return found


@task
def check(context: Context) -> None:
    """Fail if any SKILL.md reintroduces a pathed or shell-wrapped launcher."""
    offenders = violations(repo_root())
    if offenders:
        raise Exit(
            "check-bare-invocation found violation(s):\n  "
            + "\n  ".join(offenders),
            code=1,
        )
