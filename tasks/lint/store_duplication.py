"""Guard: keep the atomic-write primitive consolidated in the store crate.

A temp-file-plus-rename write shape anywhere under ``cli/**/src`` other than
``cli/store/`` means a second ``atomic_write`` was (re)introduced. Two renames
are genuine non-duplicates and are allowlisted: the launcher cache publisher (a
0600 write plus a paired signature, not a whole-file replacement) and the
mkdir-lock's directory rename-as-claim (not a write at all).
"""

import re
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.sources import repo_root

# Genuine non-duplicate renames, each with its reason. Repo-relative paths.
ALLOWLIST: frozenset[str] = frozenset(
    {
        # 0600 publish + a paired signature, not a whole-file replacement.
        "cli/launcher/src/launch/outbound/resolve/cache.rs",
        # Renames a directory as a stale-lock claim, not a write at all.
        "cli/corpus-adapters/src/lock.rs",
    }
)

# The shapes a whole-file temp-write-then-rename primitive leaves behind.
_SHAPE = re.compile(r"fs::rename\(|NamedTempFile|\.persist\(")


def violations(root: Path) -> list[str]:
    """Repo-relative ``path:line`` for every temp-and-rename shape to flag.

    Scans ``cli/**/src`` Rust sources, excluding the ``cli/store`` crate that
    owns the primitive and the allowlisted non-duplicate renames.
    """
    found: list[str] = []
    for path in sorted((root / "cli").rglob("*.rs")):
        rel = path.relative_to(root).as_posix()
        if "/src/" not in rel or rel.startswith("cli/store/src/"):
            continue
        if rel in ALLOWLIST:
            continue
        for number, line in enumerate(path.read_text().splitlines(), start=1):
            if _SHAPE.search(line):
                found.append(f"{rel}:{number}")
    return found


@task
def check(context: Context) -> None:
    """Fail if a temp-file-plus-rename write appears outside cli/store/."""
    offenders = violations(repo_root())
    if offenders:
        raise Exit(
            "a temp-file-plus-rename write belongs in cli/store/ "
            "(store::atomic_write). If a genuine non-duplicate, add it to "
            "ALLOWLIST in tasks/lint/store_duplication.py with a reason:\n  "
            + "\n  ".join(offenders),
            code=1,
        )
