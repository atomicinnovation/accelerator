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
is root-anchored), and `.venv` is ignored nowhere. Revisit with per-directory
spec layering if a nested `.gitignore` ever needs to hide a source file.

`shell_sources` layers the shell policy on top: `*.sh`, minus `workspaces/` (jj
checkouts), plus the explicitly-listed extensionless CLI script, so format and
lint never disagree about what is in scope.
"""

import os
from collections.abc import Iterator
from pathlib import Path

import pathspec


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _keep(rel: str) -> bool:
    """Return True if a repo-relative shell path should be formatted/linted."""
    if not rel:
        return False
    parts = rel.split("/")
    # workspaces/ holds jj workspace checkouts — full copies of the repo.
    # Linting them would re-lint every tracked script N times over, and they
    # are never edited directly, so they are the one permanent exclusion.
    return parts[0] != "workspaces"


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


# The plugin entry point is a bash script with no .sh extension (Claude Code
# invokes the bare command), so the walk's `.sh` filter never matches it.
# Include it explicitly so it is linted/formatted like any other script;
# shfmt/shellcheck detect bash from its `#!/usr/bin/env bash`.
_EXTRA_SHELL_SOURCES = ("bin/accelerator",)


def shell_sources(root: Path | None = None) -> list[str]:
    """`.sh` files (repo-relative, sorted) with the exclusion set applied.

    Keeps `.sh` files from `walk_files` that are not excluded by `_keep`. The
    extensionless extras in `_EXTRA_SHELL_SOURCES` are appended afterwards (the
    walk's `.sh` filter never matches them), which needs its own `_ignore_spec`
    since `walk_files` does not expose the one it built.
    """
    repo = root or repo_root()
    out = [
        rel for rel in walk_files(repo) if rel.endswith(".sh") and _keep(rel)
    ]
    # Gate on existence under `repo` (so a tmp_path test root that lacks them
    # does not pick up the literal) and still respect gitignore + _keep
    # (defensive: a future workspaces-relative extra would be filtered, though
    # the current literal never is).
    spec = _ignore_spec(repo)
    out.extend(
        s
        for s in _EXTRA_SHELL_SOURCES
        if (repo / s).is_file() and _keep(s) and not spec.match_file(s)
    )
    return sorted(out)
