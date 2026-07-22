"""Verify the 0167 inventory records (ADR-0048 Python guardrail).

Reconciles the deletion ledger, divergence record, and behaviour-inventory
member 4 against a pinned pre-deletion revision, so the check stays meaningful
after the removal set is deleted: a working-tree extraction would yield nothing
once the scripts are gone and pass trivially at exactly the moment it matters,
so the removal-set floor and the covering-suite audit read from the pinned
revision via ``jj file show``, never the working tree.

Not part of ``mise run check`` (it needs jj history a git-only CI checkout may
lack); run it at phase boundaries with ``mise run lint:inventory:check``.
"""

import re
import shutil
from collections.abc import Callable
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.sources import repo_root

# A commit where the removal set is whole. change_ids are stable across rebases.
PINNED_REV = "vnorwskwqlrv"

# The canonical removal set (20 paths) — the single list the gate reconciles the
# inventory against.
REMOVAL_SET: tuple[str, ...] = (
    "scripts/config-read-value.sh",
    "scripts/config-read-path.sh",
    "scripts/config-read-all-paths.sh",
    "scripts/config-read-doc-type-paths.sh",
    "scripts/config-read-work.sh",
    "scripts/config-read-agents.sh",
    "scripts/config-read-agent-name.sh",
    "scripts/config-read-context.sh",
    "scripts/config-read-review.sh",
    "scripts/config-read-skill-context.sh",
    "scripts/config-read-skill-instructions.sh",
    "scripts/config-read-template.sh",
    "scripts/config-list-template.sh",
    "scripts/config-show-template.sh",
    "scripts/config-eject-template.sh",
    "scripts/config-diff-template.sh",
    "scripts/config-reset-template.sh",
    "scripts/config-dump.sh",
    "scripts/config-summary.sh",
    "skills/config/init/scripts/init.sh",
)

REMOVAL_SET_FLOOR = 20

# Config suites that (directly or via a sourced helper) exercise a removal-set
# script — the set that keeps member 4 empty.
COVERING_SUITES: tuple[str, ...] = (
    "scripts/test-config.sh",
    "scripts/test-config-read-doc-type-paths.sh",
    "skills/config/init/scripts/test-init.sh",
)

# Divergence record test refs map a short file token to its source file.
_SOURCE_FILES: dict[str, str] = {
    "read.rs": "cli/launcher/tests/config_read.rs",
    "parity.rs": "cli/config-adapters/tests/parity.rs",
    "store.rs": "cli/config-adapters/src/store.rs",
    "compose.rs": "cli/config-adapters/src/compose.rs",
}
_DIVERGENCE_REF = re.compile(r"(?:read|parity|store|compose)\.rs::[a-z_]+")

# A pinned-revision reader: (rev, repo-relative path) -> content or None.
Fetch = Callable[[str, str], str | None]
# A working-tree reader: repo-relative path -> content ("" if absent).
ReadSource = Callable[[str], str]


def removal_set_floor_violations(fetch: Fetch) -> list[str]:
    """Removal-set paths absent at the pinned revision, plus a floor breach."""
    found: list[str] = []
    present = 0
    for path in REMOVAL_SET:
        if fetch(PINNED_REV, path) is None:
            found.append(f"removal-set path absent at {PINNED_REV}: {path}")
        else:
            present += 1
    if present < REMOVAL_SET_FLOOR:
        found.append(
            f"removal-set floor: found {present}, expected >= "
            f"{REMOVAL_SET_FLOOR}"
        )
    return found


def divergence_violations(text: str, read_source: ReadSource) -> list[str]:
    """Divergence-record test refs that do not resolve to a real ``fn``."""
    refs = sorted(set(_DIVERGENCE_REF.findall(text)))
    if not refs:
        return ["divergences record names no tests"]
    found: list[str] = []
    for ref in refs:
        token, name = ref.split("::", 1)
        source = _SOURCE_FILES.get(token)
        if source is None:
            found.append(f"divergences names an unknown test file: {token}")
            continue
        if not re.search(rf"fn {re.escape(name)}\b", read_source(source)):
            found.append(
                f"divergences names a test that does not resolve: {ref}"
            )
    return found


def deletion_ledger_omissions(text: str) -> list[str]:
    """Removal-set paths with no row in the deletion ledger."""
    return [f"deletion ledger omits: {p}" for p in REMOVAL_SET if p not in text]


def member_four_violations(fetch: Fetch) -> list[str]:
    """Return removal-set scripts not named by any covering suite.

    Reads each covering suite at the pinned revision; member 4 must be empty.
    """
    suites = {s: (fetch(PINNED_REV, s) or "") for s in COVERING_SUITES}
    uncovered = [
        Path(path).name[: -len(".sh")]
        for path in REMOVAL_SET
        if not any(
            Path(path).name[: -len(".sh")] in content
            for content in suites.values()
        )
    ]
    if uncovered:
        return [
            "member 4 is not empty — uncovered removal-set scripts: "
            + " ".join(uncovered)
        ]
    return []


@task
def check(context: Context) -> None:
    """Fail if the inventory records do not reconcile at the pinned revision."""
    if shutil.which("jj") is None:
        raise Exit(
            "jj is not on PATH — the pinned-revision extraction cannot run",
            code=1,
        )
    root = repo_root()

    def fetch(rev: str, path: str) -> str | None:
        result = context.run(
            f"jj file show -r {rev} {path}", warn=True, hide=True
        )
        return result.stdout if result is not None and result.ok else None

    def read_source(rel: str) -> str:
        candidate = root / rel
        return candidate.read_text() if candidate.is_file() else ""

    inventories = root / "meta/inventories"
    ledger = (inventories / "0167-deletion-ledger.md").read_text()
    divergences = (inventories / "0167-divergences.md").read_text()

    offenders = [
        *removal_set_floor_violations(fetch),
        *divergence_violations(divergences, read_source),
        *deletion_ledger_omissions(ledger),
        *member_four_violations(fetch),
    ]
    if offenders:
        raise Exit(
            "check-inventory found violation(s):\n  " + "\n  ".join(offenders),
            code=1,
        )
