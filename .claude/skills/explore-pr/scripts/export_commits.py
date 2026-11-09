"""Export each commit of a pull request, for the explore-pr By commit view.

Usage: export_commits.py OWNER/REPO NUMBER WORKSPACE

Writes `WORKSPACE/commits/<sha>/<path>` for every file a commit leaves
behind, `WORKSPACE/commits/base/<path>` for each file as the pull request's
first commit to touch it found it at the merge base, and
`WORKSPACE/data/commit-list.json`, the commits in order with the files each
one touches. Merge commits are listed but not exported.
"""

import json
import sys
from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from export_trees import WORKERS, Fetch, gh, gh_fetch, inside, revisions

FilesOf = Callable[[str], list[dict]]
ADDED = "added"
REMOVED = "removed"
MOVED = {"renamed", "copied"}


def pr_commits(repo: str, number: str) -> list[dict]:
    result = gh(
        "api",
        "--paginate",
        f"repos/{repo}/pulls/{number}/commits",
        "--jq",
        ".[] | {sha, parents: [.parents[].sha], "
        "message: .commit.message, author: .commit.author.name, "
        "date: .commit.author.date}",
    )
    result.check_returncode()
    commits = [json.loads(line) for line in result.stdout.splitlines() if line]
    for commit in commits:
        commit["subject"] = commit["message"].split("\n", 1)[0]
    return commits


def gh_files_of(repo: str) -> FilesOf:
    def files_of(sha: str) -> list[dict]:
        result = gh(
            "api",
            "--paginate",
            f"repos/{repo}/commits/{sha}",
            "--jq",
            ".files[] | {filename, status, previous_filename}",
        )
        result.check_returncode()
        return [json.loads(line) for line in result.stdout.splitlines() if line]

    return files_of


def touched(entry: dict) -> dict:
    moved = entry["status"] in MOVED
    return {
        "path": entry["filename"],
        "status": entry["status"],
        "previous": entry.get("previous_filename") if moved else None,
    }


def listing(commits: list[dict], files_of: FilesOf) -> list[dict]:
    listed = []
    for commit in commits:
        merge = len(commit.get("parents", [])) > 1
        files = [] if merge else [touched(e) for e in files_of(commit["sha"])]
        listed.append(
            {
                key: commit.get(key)
                for key in ("sha", "subject", "message", "author", "date")
            }
            | {"merge": merge, "files": files}
        )
    return listed


def fetches(listed: list[dict]) -> list[tuple[str, str, str]]:
    plan, seen = [], set()
    for commit in listed:
        for file in commit["files"]:
            source = file["previous"] or file["path"]
            if source not in seen and file["status"] != ADDED:
                plan.append(("base", "", source))
            seen.update({source, file["path"]})
            if file["status"] != REMOVED:
                plan.append((commit["sha"], commit["sha"], file["path"]))
    return plan


def export(
    commits: list[dict],
    files_of: FilesOf,
    fetch: Fetch,
    workspace: Path,
    base_ref: str,
) -> None:
    """Write every commit's files, their base bodies and the commit list."""
    listed = listing(commits, files_of)
    root = workspace / "commits"

    def write(job: tuple[str, str, str]) -> None:
        folder, ref, path = job
        destination = inside(root / folder, path)
        if destination is None:
            return
        body = fetch(ref or base_ref, path)
        if body is None:
            return
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(body)

    with ThreadPoolExecutor(WORKERS) as pool:
        list(pool.map(write, fetches(listed)))
    out = workspace / "data/commit-list.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(listed, indent=1))


def main(argv: list[str]) -> None:
    repo, number, workspace = argv[0], argv[1], Path(argv[2])
    base_ref, _ = revisions(repo, number)
    commits = pr_commits(repo, number)
    export(commits, gh_files_of(repo), gh_fetch(repo), workspace, base_ref)
    merges = sum(len(c.get("parents", [])) > 1 for c in commits)
    print(f"{len(commits)} commits ({merges} merges) from {base_ref[:12]}")


if __name__ == "__main__":
    main(sys.argv[1:])
