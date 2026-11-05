"""Build the per-file diff that the explore-pr page renders.

Usage: extract_diff.py BASE_DIR HEAD_DIR CHANGED_FILES OUT_JSON

BASE_DIR and HEAD_DIR hold the base and head copies of the changed files at
their repository paths; a path missing from one side was added or deleted.
CHANGED_FILES lists the repository paths, one per line. Source files keep
their whole body as context, so the page can show any line of them; bulk
files (lockfiles, minified or generated output, very long files) keep five
lines either side of each change.
"""

import difflib
import json
import sys
from fnmatch import fnmatch
from pathlib import Path

WHOLE_FILE = 1_000_000
BULK_CONTEXT = 5
BULK_LINE_THRESHOLD = 4000
BULK_PATTERNS = (
    "*.lock",
    "*-lock.json",
    "*-lock.yaml",
    "*.lock.json",
    "go.sum",
    "*.min.js",
    "*.min.css",
    "*.map",
    "*.snap",
    "*.svg",
    "*.csv",
)


def read_lines(path: Path) -> list[str] | None:
    """Return the file's lines, or None for a binary file."""
    data = path.read_bytes()
    if b"\0" in data:
        return None
    try:
        return data.decode("utf-8").splitlines()
    except UnicodeDecodeError:
        return None


def is_bulk(path: str, longest: int) -> bool:
    name = path.rsplit("/", 1)[-1]
    matches_pattern = any(fnmatch(name, pattern) for pattern in BULK_PATTERNS)
    return matches_pattern or longest > BULK_LINE_THRESHOLD


def hunks_of(old: list[str], new: list[str], context: int) -> list[dict]:
    matcher = difflib.SequenceMatcher(
        None, old, new, autojunk=len(old) > BULK_LINE_THRESHOLD
    )
    hunks = []
    for group in matcher.get_grouped_opcodes(context):
        _, first_old, _, first_new, _ = group[0]
        lines: list[str] = []
        for tag, i1, i2, j1, j2 in group:
            if tag == "equal":
                lines += [" " + line for line in old[i1:i2]]
                continue
            lines += ["-" + line for line in old[i1:i2]]
            lines += ["+" + line for line in new[j1:j2]]
        hunks.append({"o": first_old + 1, "n": first_new + 1, "l": lines})
    return hunks


def status_of(base: Path, head: Path) -> str:
    if not base.exists():
        return "added"
    if not head.exists():
        return "deleted"
    return "modified"


def file_diff(base: Path, head: Path, path: str) -> dict:
    status = status_of(base, head)
    old = read_lines(base) if base.exists() else []
    new = read_lines(head) if head.exists() else []
    if old is None or new is None:
        return {
            "status": status,
            "binary": True,
            "add": 0,
            "del": 0,
            "hunks": [],
        }
    bulk = is_bulk(path, max(len(old), len(new)))
    hunks = hunks_of(old, new, BULK_CONTEXT if bulk else WHOLE_FILE)
    lines = [line for hunk in hunks for line in hunk["l"]]
    return {
        "status": status,
        "binary": False,
        "full": not bulk,
        "add": sum(1 for line in lines if line.startswith("+")),
        "del": sum(1 for line in lines if line.startswith("-")),
        "hunks": hunks,
    }


def diff_tree(base_dir: Path, head_dir: Path, paths: list[str]) -> dict:
    return {
        path: file_diff(base_dir / path, head_dir / path, path)
        for path in paths
    }


def main(argv: list[str]) -> None:
    base_dir, head_dir, listing, out = (Path(arg) for arg in argv)
    paths = [line.strip() for line in listing.read_text().splitlines()]
    diff = diff_tree(base_dir, head_dir, [path for path in paths if path])
    out.write_text(json.dumps(diff, separators=(",", ":")))
    added = sum(entry["add"] for entry in diff.values())
    removed = sum(entry["del"] for entry in diff.values())
    print(f"{len(diff)} files, +{added} -{removed}")


if __name__ == "__main__":
    main(sys.argv[1:])
