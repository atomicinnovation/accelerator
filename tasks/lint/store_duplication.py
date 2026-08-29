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
        # Publishes a materialised tree: one rename relocates a fresh
        # generation *directory* into place, the other is a 0600 pointer
        # publish paired with it — cache.rs's shape, not a whole-file write.
        "cli/launcher/src/launch/outbound/resolve/tree/resolver.rs",
        # Renames a directory as a stale-lock claim, not a write at all.
        "cli/corpus-adapters/src/lock.rs",
        # A test-only rename simulating a watcher file-move event; the indexer
        # is a read/index module and performs no atomic writes (those route
        # through the file driver onto store::atomic_write).
        "cli/visualiser/server/src/indexer.rs",
        # Grafts a `.jj` *directory* into the colocated fixture, mirroring the
        # shell suite's `mv`. Both `git worktree add` and `jj workspace add`
        # refuse an existing non-empty target, so the workspace is built
        # elsewhere and moved — a directory relocation, not a whole-file write.
        "cli/vcs-test-support/src/fixtures.rs",
        # `merge_move`: relocates a file or directory onto a destination,
        # merging directories recursively — a relocation of existing content,
        # never a whole-file replacement of new bytes.
        "cli/migrate-adapters/src/merge_move.rs",
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
