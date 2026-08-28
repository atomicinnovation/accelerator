"""Shared source-file discovery for the format and lint task families.

`walk_files` is the shared traversal: a gitignore-honouring walk that prunes
ignored and build-output directories in place. Suffix filtering, subtree scoping
and per-caller keep rules stay with the callers, so one function does not become
the single point of change for several unrelated discovery policies.

Discovery is a filesystem walk rather than a VCS query, deliberately: `git
ls-files` is blind inside a jj workspace (git resolves to the parent repo, whose
index does not track the workspace), which silently emptied the scan and let
unformatted scripts reach CI. A plain walk behaves identically under git
checkouts (CI) and jj workspaces (local).

Only the root `.gitignore` is honoured, not nested ones. That is why `prune`
carries build output the root file misses: `dist/` and `playwright-report/` are
ignored only by `cli/visualiser/frontend/.gitignore` (the root file's `/dist/`
is root-anchored). Revisit with per-directory spec layering if a nested
`.gitignore` ever needs to hide a source file.

`prune` deliberately overlaps the root file for `.venv` and `node_modules`,
which it also lists. The walk is called with `tmp_path` roots that carry no
`.gitignore` at all, so the prune is the only thing keeping a vendored tree out
of those scans.

The shell surface is now two thin-wrapper files, enumerated in
`SURVIVING_SHELL_SOURCES` rather than discovered by a tree walk: the format and
lint tasks feed it to shfmt, ShellCheck, and the Python bashisms scan.
"""

import os
from collections.abc import Iterator
from pathlib import Path

import pathspec

# The surviving thin-shell files (repo-relative), guarded by shfmt, ShellCheck,
# and the Python bashisms scan. `bin/accelerator` is the launcher bootstrap and
# `hooks/launcher-link-refresh.sh` the hook wrapper; both stay bash-3.2-safe
# (ADR-0049). Enumerated, not walk-discovered, now the wider shell surface is
# retired. tasks/README.md documents this set; a test pins them equal.
SURVIVING_SHELL_SOURCES: tuple[str, ...] = (
    "bin/accelerator",
    "hooks/launcher-link-refresh.sh",
)


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _ignore_spec(repo: Path) -> pathspec.GitIgnoreSpec:
    """Gitignore matcher from the repo `.gitignore`, plus VCS metadata dirs.

    `.git`/`.jj` are never listed in `.gitignore` but must never be walked.
    """
    gitignore = repo / ".gitignore"
    lines = gitignore.read_text().splitlines() if gitignore.is_file() else []
    lines += [".git/", ".jj/"]
    return pathspec.GitIgnoreSpec.from_lines(lines)


_BUILD_OUTPUT = (
    "dist",
    ".venv",
    "node_modules",
    "playwright-report",
    "coverage",
)


def walk_files(
    repo: Path,
    subtree: str | None = None,
    prune: tuple[str, ...] = _BUILD_OUTPUT,
) -> Iterator[str]:
    """Repo-relative paths, gitignore-honouring, pruning ignored dirs in place.

    The ignore spec is always read from ``repo`` and matched repo-relative,
    whatever ``subtree`` scopes the walk to — the root ``.gitignore``'s entries
    are root-anchored (``cli/target/``), so reading a spec at a subtree would
    silently match nothing.

    ``prune`` names directories to skip wherever they appear, and it *replaces*
    the default rather than adding to it, so a caller sees exactly what it gets.
    To keep the defaults and add to them, pass ``_BUILD_OUTPUT + (...)``.
    """
    spec = _ignore_spec(repo)
    start = repo / subtree if subtree else repo
    for dirpath, dirnames, filenames in os.walk(start):
        rel_dir = Path(dirpath).relative_to(repo)
        dirnames[:] = [
            d
            for d in dirnames
            if d not in prune
            and not spec.match_file(
                f"{d}/" if rel_dir == Path() else f"{rel_dir / d}/"
            )
        ]
        for filename in filenames:
            rel = filename if rel_dir == Path() else str(rel_dir / filename)
            if not spec.match_file(rel):
                yield rel
