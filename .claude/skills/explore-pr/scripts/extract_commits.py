"""Trace every commit's changed lines onto the pull request's whole diff.

Usage: extract_commits.py WORKSPACE

Reads `commits/` and `data/commit-list.json` (scripts/export_commits.py) and
`data/diff.json` (scripts/extract_diff.py), and writes `data/commits.json`:
each commit's hunks with three lines of context, and the fate of every line
it changes. A line's fate is its key in the whole diff (`new:<head line>` or
`old:<base line>`) when the whole diff shows it changed; `~<sha>` when the
commit named rewrites or removes a line this commit added; `^<sha>` when this
commit removes a line the commit named added; and `=` when later commits
restore what this one changed, so the whole diff shows no change there.
Where identical lines sit side by side, the whole diff can choose a
different one of them as the changed line; a line that would be `=` then
takes the nearest such line of the same text instead.
"""

import difflib
import json
import sys
from dataclasses import dataclass, field
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from extract_diff import read_lines

CONTEXT = 3
Origin = tuple


@dataclass
class Lineage:
    """A file's lines through the pull request, each with where it began."""

    path: str
    lines: list[str] = field(default_factory=list)
    origins: list[Origin] = field(default_factory=list)


@dataclass
class Change:
    sign: str
    origin: Origin
    lineage: Lineage
    text: str
    removed_by: str = ""


def changed_texts(entry: dict) -> dict[str, str]:
    """Return each changed line of a whole-diff entry by key, with its text."""
    texts = {}
    for hunk in entry.get("hunks", []):
        old, new = hunk["o"], hunk["n"]
        for line in hunk["l"]:
            if line[0] == "+":
                texts[f"new:{new}"] = line[1:]
            elif line[0] == "-":
                texts[f"old:{old}"] = line[1:]
            old += line[0] != "+"
            new += line[0] != "-"
    return texts


def body(path: Path) -> list[str] | None:
    return read_lines(path) if path.exists() else []


def from_base(base: Path, path: str) -> Lineage:
    lines = body(base / path) or []
    return Lineage(path, lines, [("base", n) for n in range(1, len(lines) + 1)])


def hunks_of(
    matcher: difflib.SequenceMatcher, old: list[str], new: list[str]
) -> list[dict]:
    hunks = []
    for group in matcher.get_grouped_opcodes(CONTEXT):
        _, first_old, _, first_new, _ = group[0]
        rows: list[str] = []
        for tag, i1, i2, j1, j2 in group:
            if tag == "equal":
                rows += [" " + line for line in old[i1:i2]]
                continue
            rows += ["-" + line for line in old[i1:i2]]
            rows += ["+" + line for line in new[j1:j2]]
        hunks.append({"o": first_old + 1, "n": first_new + 1, "l": rows})
    return hunks


class Tracer:
    def __init__(self, commits_dir: Path) -> None:
        self.dir = commits_dir
        self.live: dict[str, Lineage] = {}
        self.removed_by: dict[Origin, str] = {}

    def before(self, file: dict) -> Lineage:
        if file["status"] == "added":
            self.live.pop(file["path"], None)
            return Lineage(file["path"])
        source = file["previous"] or file["path"]
        known = self.live.pop(source, None)
        return known or from_base(self.dir / "base", source)

    def step(self, index: int, sha: str, file: dict) -> dict:
        lineage = self.before(file)
        lineage.path = file["path"]
        removed = file["status"] == "removed"
        after = [] if removed else body(self.dir / sha / file["path"])
        record = {**file, "binary": after is None}
        if after is None:
            lineage.lines, lineage.origins = [], []
            self.live[file["path"]] = lineage
            return record | {"add": 0, "del": 0, "hunks": [], "changes": []}
        old = lineage.lines
        matcher = difflib.SequenceMatcher(None, old, after, autojunk=False)
        origins: list[Origin] = []
        found: list[Change] = []
        for tag, i1, i2, j1, j2 in matcher.get_opcodes():
            if tag == "equal":
                origins += lineage.origins[i1:i2]
                continue
            for origin, text in zip(
                lineage.origins[i1:i2], old[i1:i2], strict=True
            ):
                found.append(Change("-", origin, lineage, text))
                if origin[0] == "pr":
                    self.removed_by[origin] = sha
            for j in range(j1, j2):
                origin = ("pr", index, sha, file["path"], j + 1)
                origins.append(origin)
                found.append(Change("+", origin, lineage, after[j]))
        hunks = hunks_of(matcher, old, after)
        lineage.lines, lineage.origins = after, origins
        if not removed:
            self.live[file["path"]] = lineage
        return record | {
            "add": sum(c.sign == "+" for c in found),
            "del": sum(c.sign == "-" for c in found),
            "hunks": hunks,
            "changes": found,
        }


def place_of(change: Change, head_line: dict) -> str | None:
    """Return the line's key at base or head, or None if it has no place."""
    origin = change.origin
    if change.sign == "-":
        return f"old:{origin[1]}" if origin[0] == "base" else None
    line = head_line.get(origin)
    return f"new:{line}" if line is not None else None


def fate_of(change: Change, whole: dict[str, str], head_line: dict) -> str:
    if change.sign == "-" and change.origin[0] == "pr":
        return "^" + change.origin[2]
    place = place_of(change, head_line)
    if place is None:
        return "~" + change.removed_by
    return place if place in whole else "="


def number(key: str) -> int:
    return int(key.split(":")[1])


def reconcile(restored: list, whole: dict[str, str], claimed: set) -> None:
    """Give each restored line the nearest unclaimed whole-diff twin."""
    unclaimed = {key for key in whole if key not in claimed}
    for file, index, change, place in restored:
        side = place.split(":")[0]
        twins = [
            key
            for key in unclaimed
            if key.startswith(side + ":") and whole[key] == change.text
        ]
        if not twins:
            continue
        twin = min(twins, key=lambda key: abs(number(key) - number(place)))
        unclaimed.discard(twin)
        file["fate"][index] = twin


def trace(commits_dir: Path, listed: list[dict], diff: dict) -> dict:
    """Return each commit's hunks with the fate of every line it changes."""
    tracer = Tracer(commits_dir)
    out = []
    for index, commit in enumerate(listed):
        files = [tracer.step(index, commit["sha"], f) for f in commit["files"]]
        out.append({**commit, "files": files})
    head_line = {
        origin: n
        for lineage in tracer.live.values()
        for n, origin in enumerate(lineage.origins, 1)
    }
    whole = {path: changed_texts(entry) for path, entry in diff.items()}
    restored: dict[str, list] = {}
    for commit in out:
        for file in commit["files"]:
            changes = file.pop("changes")
            head_path = changes[0].lineage.path if changes else file["path"]
            in_whole = whole.get(head_path, {})
            for change in changes:
                change.removed_by = tracer.removed_by.get(change.origin, "")
            file["headPath"] = head_path
            file["fate"] = [fate_of(c, in_whole, head_line) for c in changes]
            restored.setdefault(head_path, []).extend(
                (file, i, c, place_of(c, head_line))
                for i, c in enumerate(changes)
                if file["fate"][i] == "="
            )
    for path, lines in restored.items():
        claimed = {
            fate
            for commit in out
            for file in commit["files"]
            if file["headPath"] == path
            for fate in file["fate"]
        }
        reconcile(lines, whole.get(path, {}), claimed)
    return {"commits": out}


def main(argv: list[str]) -> None:
    workspace = Path(argv[0])
    listed = json.loads((workspace / "data/commit-list.json").read_text())
    diff = json.loads((workspace / "data/diff.json").read_text())
    traced = trace(workspace / "commits", listed, diff)
    (workspace / "data/commits.json").write_text(
        json.dumps(traced, separators=(",", ":"))
    )
    fates = [
        f for c in traced["commits"] for x in c["files"] for f in x["fate"]
    ]
    kept = sum(":" in f for f in fates)
    print(
        f"{len(traced['commits'])} commits, {len(fates)} changed lines, "
        f"{kept} in the whole diff"
    )


if __name__ == "__main__":
    main(sys.argv[1:])
