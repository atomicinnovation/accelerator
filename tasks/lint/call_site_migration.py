"""Gate for the config-command call-site migration (ADR-0048 Python guardrail).

Proves that no retained file still *invokes* a removal-set config script
(Grep A-functional), that no SKILL.md names one under ``scripts/`` (Grep B), and
that ``--allow-legacy-layout`` stays confined to the migration engine. The
corpus is fixed here so a scope chosen at verification time cannot be narrowed
until it passes.

Grep A-functional is gated to zero OUTSIDE a ``PENDING_PHASE7`` allowlist (empty
at the final state). A textual *mention* — a comment or prose reference — is
reported by :func:`mention_count`, never gated.
"""

import os
import re
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.sources import repo_root

# The removal set (Phase 7 §1), by basename.
_REMOVAL_SET_BASENAMES: tuple[str, ...] = (
    "config-read-value.sh",
    "config-read-path.sh",
    "config-read-all-paths.sh",
    "config-read-doc-type-paths.sh",
    "config-read-work.sh",
    "config-read-agents.sh",
    "config-read-agent-name.sh",
    "config-read-context.sh",
    "config-read-review.sh",
    "config-read-skill-context.sh",
    "config-read-skill-instructions.sh",
    "config-read-template.sh",
    "config-list-template.sh",
    "config-show-template.sh",
    "config-eject-template.sh",
    "config-diff-template.sh",
    "config-reset-template.sh",
    "config-dump.sh",
    "config-summary.sh",
)

# The removal-set paths (basenames plus init.sh), excluded from the corpus as
# retained members whose self-references are not this story's to remove.
_REMOVAL_SET_PATHS: frozenset[str] = frozenset(
    {f"scripts/{name}" for name in _REMOVAL_SET_BASENAMES}
    | {"skills/config/init/scripts/init.sh"}
)

# Files whose functional removal-set references belonged to a later phase. Empty
# at the final state; each entry carries a known-positive floor (it MUST still
# be a functional hit) so a cleaned-up entry fails rather than rotting.
PENDING_PHASE7: frozenset[str] = frozenset()

# Directory names pruned at any depth, and top-level trees pruned wholesale —
# matching the shell `find` prune set (meta/ prose and docs/ would false-match).
_PRUNE_DIRS: frozenset[str] = frozenset({".git", ".jj", "node_modules"})
_PRUNE_PREFIXES: tuple[str, ...] = ("cli/target/", "meta/", "docs/")
_CORPUS_SUFFIXES: tuple[str, ...] = (".sh", ".rs", ".md")

_COMMENT = re.compile(r"^\s*(#|//|\*)")
_BASENAME_ALT = "|".join(re.escape(name) for name in _REMOVAL_SET_BASENAMES)
_ANY_BASENAME = re.compile(f"({_BASENAME_ALT})")
# Functional shapes: basename preceded by a separator or quote, or reached
# through an invocation keyword / resolver helper.
_FUNCTIONAL = re.compile(
    rf"([/\"']({_BASENAME_ALT}))"
    rf"|((bash|exec|source|require_script|\.join\(|\.arg\()"
    rf".*({_BASENAME_ALT}))"
)


def _is_excluded_path(rel: str) -> bool:
    """Return whether a corpus path is excluded from the Grep A corpus.

    Removal-set members and retained siblings whose self-references are out of
    scope are excluded.
    """
    if rel == "scripts/config-common.sh":
        return (
            False  # kept in the corpus; any Phase-7 refs would be allowlisted
        )
    # The gates that DEFINE the removal set enumerate it as data, not as calls.
    if rel in {
        "scripts/check-inventory.sh",
        "scripts/check-call-site-migration.sh",
    }:
        return True
    if rel in _REMOVAL_SET_PATHS:
        return True
    if rel == "scripts/config-read-browser-executor.sh":  # 0173 owns it
        return True
    return rel.startswith("scripts/test-shims/")  # deleted with the suite


def _corpus(root: Path) -> list[str]:
    """Repo-relative shell/rust/markdown files in the Grep A corpus."""
    out: list[str] = []
    for dirpath, dirnames, filenames in os.walk(root):
        rel_dir = Path(dirpath).relative_to(root)
        dirnames[:] = [d for d in dirnames if d not in _PRUNE_DIRS]
        prefix = "" if rel_dir == Path() else f"{rel_dir.as_posix()}/"
        dirnames[:] = [
            d
            for d in dirnames
            if not any(
                (prefix + d + "/").startswith(p) for p in _PRUNE_PREFIXES
            )
        ]
        for filename in filenames:
            if not filename.endswith(_CORPUS_SUFFIXES):
                continue
            rel = f"{prefix}{filename}"
            if rel == "CHANGELOG.md":
                continue
            out.append(rel)
    return sorted(out)


def functional_hits(root: Path) -> list[str]:
    """Return ``path:line:text`` for each functional removal-set reference.

    References inside the ``PENDING_PHASE7`` allowlist are excluded.
    """
    hits: list[str] = []
    for rel in _corpus(root):
        if _is_excluded_path(rel) or rel in PENDING_PHASE7:
            continue
        text = (root / rel).read_text(errors="replace")
        for number, line in enumerate(text.splitlines(), start=1):
            if not _ANY_BASENAME.search(line):
                continue
            if _COMMENT.match(line):
                continue
            if _FUNCTIONAL.search(line):
                hits.append(f"{rel}:{number}:{line.strip()}")
    return hits


def pending_without_reference(root: Path) -> list[str]:
    """Return ``PENDING_PHASE7`` entries with no functional reference left.

    A cleaned-up allowlist entry must fail rather than rot silently.
    """
    stale: list[str] = []
    for rel in sorted(PENDING_PHASE7):
        path = root / rel
        text = path.read_text(errors="replace") if path.is_file() else ""
        found = any(
            _ANY_BASENAME.search(line)
            and not _COMMENT.match(line)
            and _FUNCTIONAL.search(line)
            for line in text.splitlines()
        )
        if not found:
            stale.append(rel)
    return stale


def grep_b_hits(root: Path) -> list[str]:
    """Return SKILL.md lines naming a removed ``scripts/config-`` script.

    The browser executor (0173) and config-common (0174) are permitted.
    """
    hits: list[str] = []
    allowed = ("config-read-browser-executor.sh", "config-common.sh")
    for path in sorted((root / "skills").rglob("SKILL.md")):
        rel = path.relative_to(root).as_posix()
        for number, line in enumerate(path.read_text().splitlines(), start=1):
            if "scripts/config-" in line and not any(
                a in line for a in allowed
            ):
                hits.append(f"{rel}:{number}:{line.strip()}")
    return hits


def stray_legacy_flag(root: Path) -> list[str]:
    """Return ``*.sh`` files naming ``--allow-legacy-layout`` out of bounds.

    Scans ``skills/`` and ``scripts/``; the migration engine (migrations/ and
    the allowlisted ``doc-type-table.sh``), tests, and this gate are permitted.
    """
    stray: list[str] = []
    for base in ("skills", "scripts"):
        for path in sorted((root / base).rglob("*.sh")):
            rel = path.relative_to(root).as_posix()
            if "allow-legacy-layout" not in path.read_text():
                continue
            if rel.startswith("skills/config/migrate/migrations/"):
                continue
            if rel == "scripts/doc-type-table.sh":
                continue
            name = Path(rel).name
            if (
                name.startswith("test-")
                or name == "check-call-site-migration.sh"
            ):
                continue
            stray.append(rel)
    return stray


def mention_count(root: Path) -> int:
    """Count non-functional references to removal-set names (reported only)."""
    count = 0
    for rel in _corpus(root):
        if _is_excluded_path(rel):
            continue
        for line in (root / rel).read_text(errors="replace").splitlines():
            if not _ANY_BASENAME.search(line):
                continue
            if _COMMENT.match(line) or not _FUNCTIONAL.search(line):
                count += 1
    return count


def violations(root: Path) -> list[str]:
    """Every gated failure across Grep A, its floor, Grep B, and the flag."""
    found: list[str] = []
    found.extend(f"Grep A-functional: {hit}" for hit in functional_hits(root))
    found.extend(
        f"PENDING_PHASE7 entry no longer references a script: {stale}"
        for stale in pending_without_reference(root)
    )
    found.extend(
        f"Grep B: SKILL.md names a removal-set script: {hit}"
        for hit in grep_b_hits(root)
    )
    found.extend(
        f"--allow-legacy-layout outside the migration engine: {rel}"
        for rel in stray_legacy_flag(root)
    )
    return found


@task
def check(context: Context) -> None:
    """Fail on any retained functional reference, Grep B hit, or stray flag."""
    root = repo_root()
    offenders = violations(root)
    if offenders:
        raise Exit(
            "check-call-site-migration found violation(s):\n  "
            + "\n  ".join(offenders),
            code=1,
        )
