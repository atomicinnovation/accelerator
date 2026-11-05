"""Export a pull request's changed files at its merge base and its head.

Usage: export_trees.py OWNER/REPO NUMBER WORKSPACE

Writes `WORKSPACE/base/<path>` and `WORKSPACE/head/<path>` for every file the
pull request changes, and `WORKSPACE/data/changed-files.txt`. Bodies come from
the GitHub API through `gh`, so no local checkout of either revision is
needed. The base is the merge base, so the result matches the diff GitHub
shows. A renamed file's base body is written under its new name, so its
diff shows what changed rather than a whole-file addition.
"""

import json
import subprocess
import sys
from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from urllib.parse import quote

Fetch = Callable[[str, str], bytes | None]
WORKERS = 8
NO_BASE = {"added"}
NO_HEAD = {"removed"}
MOVED = {"renamed", "copied"}


def gh(*arguments: str) -> subprocess.CompletedProcess:
    return subprocess.run(["gh", *arguments], capture_output=True, check=False)


def gh_fetch(repo: str) -> Fetch:
    def fetch(ref: str, path: str) -> bytes | None:
        result = gh(
            "api",
            "-H",
            "Accept: application/vnd.github.raw",
            f"repos/{repo}/contents/{quote(path)}?ref={ref}",
        )
        return result.stdout if result.returncode == 0 else None

    return fetch


def pr_files(repo: str, number: str) -> list[dict]:
    result = gh(
        "api",
        "--paginate",
        f"repos/{repo}/pulls/{number}/files",
        "--jq",
        ".[] | {filename, status, previous_filename}",
    )
    result.check_returncode()
    return [json.loads(line) for line in result.stdout.splitlines() if line]


def revisions(repo: str, number: str) -> tuple[str, str]:
    """Return the pull request's merge base and head commit."""
    head = gh(
        "pr", "view", number, "-R", repo, "--json", "baseRefOid,headRefOid"
    )
    head.check_returncode()
    refs = json.loads(head.stdout)
    compare = gh(
        "api",
        f"repos/{repo}/compare/{refs['baseRefOid']}...{refs['headRefOid']}",
        "--jq",
        ".merge_base_commit.sha",
    )
    compare.check_returncode()
    return compare.stdout.decode().strip(), refs["headRefOid"]


def inside(root: Path, path: str) -> Path | None:
    target = (root / path).resolve()
    return target if target.is_relative_to(root.resolve()) else None


def fetches(entries: list[dict]) -> list[tuple[str, str, str]]:
    plan = []
    for entry in entries:
        path, status = entry["filename"], entry["status"]
        if status not in NO_BASE:
            source = entry.get("previous_filename") if status in MOVED else None
            plan.append(("base", source or path, path))
        if status not in NO_HEAD:
            plan.append(("head", path, path))
    return plan


def export(
    entries: list[dict],
    fetch: Fetch,
    workspace: Path,
    base_ref: str,
    head_ref: str,
) -> None:
    """Write both trees and the changed-file list under WORKSPACE."""
    refs = {"base": base_ref, "head": head_ref}

    def write(job: tuple[str, str, str]) -> None:
        side, source, target = job
        destination = inside(workspace / side, target)
        if destination is None:
            return
        body = fetch(refs[side], source)
        if body is None:
            return
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(body)

    with ThreadPoolExecutor(WORKERS) as pool:
        list(pool.map(write, fetches(entries)))
    listing = workspace / "data/changed-files.txt"
    listing.parent.mkdir(parents=True, exist_ok=True)
    listing.write_text("".join(f"{e['filename']}\n" for e in entries))


def main(argv: list[str]) -> None:
    repo, number, workspace = argv[0], argv[1], Path(argv[2])
    base_ref, head_ref = revisions(repo, number)
    entries = pr_files(repo, number)
    export(entries, gh_fetch(repo), workspace, base_ref, head_ref)
    print(
        f"{len(entries)} files at merge base {base_ref[:12]} "
        f"and head {head_ref[:12]}"
    )


if __name__ == "__main__":
    main(sys.argv[1:])
