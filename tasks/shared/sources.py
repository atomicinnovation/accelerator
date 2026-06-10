"""Shared source-file discovery for the format and lint task families.

Both families scan an identical set of shell files (`*.sh`, minus fixtures, jj
workspaces, and the sourced-only `test-helpers.sh`) so format and lint never
disagree about what is in scope.

Discovery is a filesystem walk that honours the repository-root `.gitignore`,
deliberately VCS-agnostic: `git ls-files` is blind inside a jj workspace (git
resolves to the parent repo, whose index does not track the workspace), which
silently emptied the scan and let unformatted scripts reach CI. A plain walk
behaves identically under git checkouts (CI) and jj workspaces (local).

Only the root `.gitignore` is honoured, not nested ones — sufficient here
because every ignored shell script lives under a root-ignored tree (e.g.
`node_modules/`). Revisit with per-directory spec layering if a nested
`.gitignore` ever needs to hide a `.sh` file.
"""

import os
from pathlib import Path

import pathspec


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _keep(rel: str) -> bool:
    """True when a repo-relative `.sh` path should be formatted/linted."""
    if not rel:
        return False
    parts = rel.split("/")
    if "test-fixtures" in parts:
        return False
    if parts[0] == "workspaces":
        return False
    if parts[-1] == "test-helpers.sh":
        return False
    return True


def _ignore_spec(repo: Path) -> pathspec.GitIgnoreSpec:
    """Gitignore matcher from the repo-root `.gitignore`, plus VCS metadata dirs.

    `.git`/`.jj` are never listed in `.gitignore` but must never be walked.
    """
    gitignore = repo / ".gitignore"
    lines = gitignore.read_text().splitlines() if gitignore.is_file() else []
    lines += [".git/", ".jj/"]
    return pathspec.GitIgnoreSpec.from_lines(lines)


def shell_sources(root: Path | None = None) -> list[str]:
    """`.sh` files (repo-relative, sorted) with the exclusion set applied.

    Walks the tree, pruning gitignored directories in place so large ignored
    trees (e.g. `node_modules`) are never descended into, then keeps `.sh`
    files that are neither gitignored nor excluded by `_keep`.
    """
    repo = root or repo_root()
    spec = _ignore_spec(repo)
    out: list[str] = []
    for dirpath, dirnames, filenames in os.walk(repo):
        rel_dir = Path(dirpath).relative_to(repo)
        dirnames[:] = [
            d
            for d in dirnames
            if not spec.match_file(f"{d}/" if rel_dir == Path(".") else f"{rel_dir / d}/")
        ]
        for filename in filenames:
            if not filename.endswith(".sh"):
                continue
            rel = filename if rel_dir == Path(".") else str(rel_dir / filename)
            if spec.match_file(rel):
                continue
            if _keep(rel):
                out.append(rel)
    return sorted(out)
