"""Resolve a plan's acceptance-criterion map to test locations.

Usage: criteria_map.py PLAN_MD REPO_ROOT OUT_JSON

Reads the plan's "Acceptance Criterion Map" table (criterion, tests, phase)
and resolves every backticked test name to the function that declares it
under REPO_ROOT. A name containing an ellipsis matches by its prefix and
suffix; a name ending in a source extension matches a file.
"""

import json
import re
import sys
from collections.abc import Iterator
from pathlib import Path

MAP_HEADING = "### Acceptance Criterion Map:"
SOURCE_SUFFIXES = {
    ".rs",
    ".py",
    ".ts",
    ".tsx",
    ".js",
    ".jsx",
    ".go",
    ".java",
    ".kt",
    ".rb",
    ".swift",
}
DECLARATION = re.compile(
    r"\s*(?:pub\s+)?(?:async\s+)?(?:fn|def|func|function)\s+"
    r"([A-Za-z_][A-Za-z_0-9]*)\s*[(<]"
)
SKIPPED_DIRECTORIES = {".git", ".jj", "node_modules", "target", ".venv"}
MAX_HITS = 6


def table_rows(plan: str) -> Iterator[list[str]]:
    if MAP_HEADING not in plan:
        return
    table = plan.split(MAP_HEADING, 1)[1].strip().split("\n\n", 1)[0]
    lines = [
        line
        for line in table.splitlines()
        if line.startswith("|") and not line.startswith("|---")
    ]
    for line in lines[1:]:
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) >= 3:
            yield cells


def source_files(repo: Path) -> Iterator[Path]:
    for path in sorted(repo.rglob("*")):
        relative = path.relative_to(repo).parts
        if SKIPPED_DIRECTORIES.intersection(relative):
            continue
        if path.is_file() and path.suffix in SOURCE_SUFFIXES:
            yield path


def declarations(repo: Path) -> list[tuple[str, str, int]]:
    found = []
    for path in source_files(repo):
        relative = path.relative_to(repo).as_posix()
        with path.open(errors="replace") as source:
            for number, line in enumerate(source, 1):
                match = DECLARATION.match(line)
                if match:
                    found.append((match[1], relative, number))
    return found


def matches(name: str, function: str) -> bool:
    prefix, ellipsis, suffix = name.partition("…")
    if not ellipsis:
        return function in {name, f"test_{name}"}
    prefix, suffix = prefix.rstrip("_"), suffix.lstrip("_")
    return function.startswith(prefix) and function.endswith(suffix)


def resolve(name: str, index: list, repo: Path) -> list[dict]:
    if Path(name).suffix in SOURCE_SUFFIXES and "/" not in name:
        files = sorted(path for path in source_files(repo) if path.name == name)
        return [
            {"name": name, "file": path.relative_to(repo).as_posix(), "line": 1}
            for path in files[:MAX_HITS]
        ]
    hits = [
        {"name": function, "file": file, "line": line}
        for function, file, line in index
        if matches(name, function)
    ]
    return hits[:MAX_HITS]


def criteria(plan: str, repo: Path) -> list[dict]:
    """Return one row per criterion, each test resolved to its hits."""
    rows = list(table_rows(plan))
    index = declarations(repo) if rows else []
    result = []
    for number, (label, tests, phase, *_) in enumerate(rows, 1):
        references = [
            ref
            for ref in re.findall(r"`([^`]+)`", tests)
            if "." not in ref or Path(ref).suffix in SOURCE_SUFFIXES
        ]
        note = re.sub(r"`[^`]+`", "", tests).strip(" ,()") or None
        result.append(
            {
                "n": number,
                "label": label,
                "phase": phase,
                "tests": [
                    {"ref": ref, "hits": resolve(ref, index, repo)}
                    for ref in references
                ],
                "note": note,
            }
        )
    return result


def main(argv: list[str]) -> None:
    plan, repo, out = Path(argv[0]), Path(argv[1]), Path(argv[2])
    rows = criteria(plan.read_text(), repo)
    out.write_text(json.dumps(rows, indent=1, ensure_ascii=False))
    tests = [test for row in rows for test in row["tests"]]
    resolved = sum(1 for test in tests if test["hits"])
    print(f"{len(rows)} criteria, {resolved} of {len(tests)} tests resolved")


if __name__ == "__main__":
    main(sys.argv[1:])
