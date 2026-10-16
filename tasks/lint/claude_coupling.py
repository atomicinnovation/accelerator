"""Guard: no plugin entry point may require a ``CLAUDE_*`` variable.

``CLAUDE_PLUGIN_ROOT`` is Claude Code's to define, and it is substituted into
skill content rather than exported to the Bash tool or the ``!`` preprocessor.
A launcher-layer reader of it therefore works only when some caller happens to
have set it — which is what broke every CLI-invoking skill. The rename set —
``cli/**``, the ``bin/accelerator`` entry point, and the four out-of-tree
writers that feed those readers — names ``ACCELERATOR_PLUGIN_ROOT`` instead,
and this makes that non-negotiable.

Everything outside the rename set is out of scope by construction, not by
exemption: the adapter layer (``hooks/``, the SKILL.md-owned skill scripts),
the Claude Code matcher model in ``skill_permissions.py``, and the SKILL.md
substitution tokens all legitimately name the variable. That is why there is
no file allowlist — the first exemption added under pressure could be the very
file the bug was in.

Inside the rename set, a ``CLAUDE_*`` name is admitted only if the CLI works
without it, with the default stated where it is read. Admission is by name and
file together, never by file alone: an admitted file naming any other
``CLAUDE_*`` variable is still flagged.
"""

import re
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.sources import repo_root, walk_files

_SHAPE = re.compile(r"CLAUDE_[A-Z0-9_]*")

# Speed only; the correctness rule is the decode below.
_SKIP_SUFFIXES = (".png", ".ico", ".woff2", ".lock", ".snap", ".bin")

_SUBTREES = ("cli",)
_FILES = (
    "bin/accelerator",
    "tasks/dev.py",
    "tasks/shared/dev/circus.py",
    "tasks/test/helpers.py",
    "tests/integration/dev/dev_integration_driver.py",
)

_OPTIONAL_HARNESS_VARIABLES = {
    "CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS": (
        "cli/research-cli/src/subagent_cap.rs",
        "cli/research-cli/tests/topic_outstanding.rs",
    ),
    "CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION": (
        "cli/research-cli/src/search_cap.rs",
        "cli/research-cli/tests/topic_outstanding.rs",
    ),
}

# A silently-emptied scan reads as cleanliness in both the lint task and the
# paired `violations(REPO_ROOT) == []` assertion, so the discovered set has a
# floor. cli/ alone carries several hundred text files.
_MIN_SCANNED = 200


def _in_scope(root: Path) -> list[Path]:
    """Absolute paths in the rename set, with unreadable suffixes skipped."""
    paths: list[Path] = []
    for subtree in _SUBTREES:
        paths.extend(
            root / rel
            for rel in walk_files(root, subtree=subtree)
            if not rel.endswith(_SKIP_SUFFIXES)
        )
    paths.extend(_named_file(root, rel) for rel in _FILES)
    _require_admitted_files(root)
    return paths


def _named_file(root: Path, rel: str) -> Path:
    _require_file(root, rel)
    return root / rel


def _require_admitted_files(root: Path) -> None:
    for files in _OPTIONAL_HARNESS_VARIABLES.values():
        for rel in files:
            _require_file(root, rel)


def _require_file(root: Path, rel: str) -> None:
    if not (root / rel).is_file():
        raise Exit(
            f"{rel} is named in tasks/lint/claude_coupling.py but is not "
            f"a file under {root} — it was renamed, moved or split, and "
            f"has silently dropped out of the guard's scope",
            code=1,
        )


def _admitted(name: str, rel: str) -> bool:
    return rel in _OPTIONAL_HARNESS_VARIABLES.get(name, ())


def violations(root: Path, min_scanned: int = _MIN_SCANNED) -> list[str]:
    """Repo-relative ``path:line:text`` for every unadmitted CLAUDE_* name.

    Files that cannot be decoded as UTF-8 are skipped: the rule is "in scope
    unless it is unreadable as text", so a binary asset landing under ``cli/``
    must not turn the check into a traceback.

    ``min_scanned`` is lowered only by this guard's own tests, which build
    deliberately tiny trees; the real-tree floor is ``_MIN_SCANNED``.
    """
    paths = _in_scope(root)
    if len(paths) < min_scanned:
        raise Exit(
            f"only {len(paths)} files matched — scope discovery is broken "
            f"(expected at least {min_scanned})",
            code=1,
        )
    found: list[str] = []
    for path in sorted(set(paths)):
        try:
            text = path.read_text(encoding="utf-8", errors="strict")
        except UnicodeDecodeError:
            continue
        rel = path.relative_to(root).as_posix()
        for number, line in enumerate(text.splitlines(), start=1):
            names = _SHAPE.findall(line)
            if not all(_admitted(name, rel) for name in names):
                found.append(f"{rel}:{number}:{line.strip()}")
    return found


@task
def check(context: Context) -> None:
    """Fail if anything in the rename set names a CLAUDE_* variable."""
    offenders = violations(repo_root())
    if offenders:
        raise Exit(
            "the rename set must not name a CLAUDE_* variable — remove the "
            "reference (see tasks/lint/claude_coupling.py for the "
            "invariant):\n  " + "\n  ".join(offenders),
            code=1,
        )
