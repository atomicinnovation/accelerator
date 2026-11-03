"""Turn a review-pr review into the explore-pr findings list.

Usage: parse_review.py REVIEW_MD CHANGED_FILES OUT_JSON [--prefix R1]

Each lens's comments merge into the inline comment they belong to, general
findings are kept without a location, and the result is ordered by severity
and numbered PREFIX-01, PREFIX-02, ... CHANGED_FILES lists the PR's paths, one
per line, so the short paths a review uses expand to repository paths.
"""

import argparse
import json
import re
from pathlib import Path

SEVERITY_ORDER = {"critical": 0, "major": 1, "minor": 2, "suggestion": 3}
INLINE_HEADING = re.compile(r"`([^`]+)` — (.*)")
SEVERITY_LINE = re.compile(
    r"\*\*Severity\*\*: (\w+) \| \*\*Confidence\*\*: (\w+) \| "
    r"\*\*Lens\*\*: ([^\n]+)"
)
LENS_COMMENT = re.compile(
    r"^- `([^`]+)` \(RIGHT\), (\w+)/(\w+): (.*)$", re.MULTILINE
)
GENERAL_FINDING = re.compile(
    r"^- (critical|major|minor|suggestion): (.*)$", re.MULTILINE
)
MERGE_NOTE = re.compile(
    r"\*\((Merged inline|Inline|Merged into the ([\w -]+) inline)\.?\)\*"
)
ANY_NOTE = re.compile(r"\s*\*\([^)]*\)\*\.?")
TRAILING_LENSES = re.compile(r"\s*\([a-z, -]+\)$")
LOCATION = re.compile(r":(\d+)(?:-(\d+))?$")


def lens_key(name: str) -> str:
    return name.strip().lower().replace(" ", "-")


def expand(location: str, changed: list[str]) -> str:
    short = location.partition(":")[0]
    hits = [p for p in changed if p == short or p.endswith("/" + short)]
    return hits[0] if hits else short


def lines_of(location: str) -> tuple[int | None, int | None]:
    match = LOCATION.search(location)
    if not match:
        return None, None
    start = int(match[1])
    return start, int(match[2] or start)


def section(text: str, start: str, end: str | None = None) -> str:
    if start not in text:
        return ""
    tail = text.split(start, 1)[1]
    return tail.split(end, 1)[0] if end else tail


def inline_comments(text: str, changed: list[str]) -> list[dict]:
    part = section(text, "## Inline Comments", "## Per-Lens Results")
    comments = []
    for block in re.split(r"\n### ", "\n" + part)[1:]:
        heading, _, body = block.partition("\n")
        match = INLINE_HEADING.match(heading)
        severity = SEVERITY_LINE.search(body)
        if not match or not severity:
            continue
        location, title = match.groups()
        start, end = lines_of(location)
        comments.append(
            {
                "file": expand(location, changed),
                "start": start,
                "end": end,
                "title": TRAILING_LENSES.sub("", title),
                "severity": severity[1],
                "confidence": severity[2],
                "lenses": [lens_key(x) for x in severity[3].split(",")],
                "body": SEVERITY_LINE.sub("", body)
                .strip()
                .removesuffix("---")
                .strip(),
                "kind": "inline",
                "summary": None,
            }
        )
    return comments


def lens_findings(text: str, changed: list[str]) -> list[dict]:
    findings = []
    lenses = section(text, "## Per-Lens Results")
    for block in re.split(r"\n### ", lenses)[1:]:
        name, _, body = block.partition("\n")
        lens = lens_key(name)
        comments = section(body, "**Comments**:", "**General Findings**")
        for location, severity, confidence, raw in LENS_COMMENT.findall(
            comments
        ):
            note = MERGE_NOTE.search(raw)
            start, end = lines_of(location)
            findings.append(
                {
                    "file": expand(location, changed),
                    "start": start,
                    "end": end,
                    "severity": severity,
                    "confidence": confidence,
                    "lenses": [lens],
                    "summary": ANY_NOTE.sub("", raw).strip(),
                    "kind": "comment",
                    "merge_into": (lens_key(note[2] or lens) if note else None),
                }
            )
        general = section(body, "**General Findings**:")
        for severity, raw in GENERAL_FINDING.findall(general):
            if "*(Merged" in raw:
                continue
            findings.append(
                {
                    "file": None,
                    "start": None,
                    "end": None,
                    "severity": severity,
                    "confidence": None,
                    "lenses": [lens],
                    "summary": raw.strip(),
                    "kind": "general",
                    "merge_into": None,
                }
            )
    return findings


def merge_target(finding: dict, inlines: list[dict]) -> dict | None:
    candidates = [i for i in inlines if i["file"] == finding["file"]]
    if not candidates:
        return None
    return min(
        candidates,
        key=lambda i: (
            finding["merge_into"] not in i["lenses"],
            abs((i["start"] or 0) - (finding["start"] or 0)),
        ),
    )


def parse(text: str, changed: list[str], prefix: str) -> list[dict]:
    """Return the review's findings, merged, ordered and numbered."""
    inlines = inline_comments(text, changed)
    result = list(inlines)
    for finding in lens_findings(text, changed):
        target = (
            merge_target(finding, inlines) if finding["merge_into"] else None
        )
        if target is None:
            result.append(finding)
            continue
        if target["summary"] is None:
            target["summary"] = finding["summary"]
        target["lenses"] += [
            lens for lens in finding["lenses"] if lens not in target["lenses"]
        ]
    result.sort(
        key=lambda f: (
            SEVERITY_ORDER.get(f["severity"], 9),
            f["kind"] != "inline",
            f["file"] or "~",
            f["start"] or 0,
        )
    )
    for number, finding in enumerate(result, 1):
        finding["id"] = f"{prefix}-{number:02d}"
        finding.pop("merge_into", None)
        finding["summary"] = finding["summary"] or finding.get("title")
    return result


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("review", type=Path)
    parser.add_argument("changed_files", type=Path)
    parser.add_argument("out", type=Path)
    parser.add_argument("--prefix", default="R1")
    args = parser.parse_args(argv)
    changed = [
        line.strip()
        for line in args.changed_files.read_text().splitlines()
        if line.strip()
    ]
    findings = parse(args.review.read_text(), changed, args.prefix)
    args.out.write_text(json.dumps(findings, indent=1, ensure_ascii=False))
    print(f"{len(findings)} findings")


if __name__ == "__main__":
    main()
