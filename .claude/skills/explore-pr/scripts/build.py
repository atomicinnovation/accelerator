"""Assemble an explore-pr page from a workspace.

Usage: build.py WORKSPACE [--out FILE] [--hosted DIR] [--page FILE]

WORKSPACE holds `explore.json` (the manifest), `content/*.json`,
`data/diff.json`, `data/commits.json` and `views/*.js`. By default the
result is one self-contained HTML file, with the libraries, data and custom
views inlined, that opens from `file://`. `--hosted DIR` writes the split
layout a static host or a claude.ai artifact serves instead: `index.html`,
`content.json`, `diff.json` and, when the workspace has them,
`commits.json`, with the libraries loaded from their CDNs. `--page`
replaces the bundled page shell with a workspace copy.
"""

import argparse
import html
import json
import re
import sys
from collections.abc import Iterator
from pathlib import Path

ASSETS = Path(__file__).resolve().parent.parent / "assets"
DATA_MARKER = "<!-- explore-pr:data -->"
VIEWS_MARKER = "<!-- explore-pr:views -->"
MODELS_MARKER = "<!-- explore-pr:models -->"
COVERAGE_MODEL = ASSETS / "review-coverage.js"
GRAPH_FOCUS_MODEL = ASSETS / "graph-focus.js"
GRAPH_ROUTES_MODEL = ASSETS / "graph-routes.js"
PROSE_REFERENCES_MODEL = ASSETS / "prose-references.js"
TITLE_PLACEHOLDER = "__EXPLORE_PR_TITLE__"
LIBRARIES = {
    "marked": (
        '<script src="https://cdn.jsdelivr.net/npm/marked@12.0.2/'
        'marked.min.js"></script>',
        "vendor/marked.min.js",
    ),
    "highlight": (
        '<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/'
        '11.9.0/highlight.min.js"></script>',
        "vendor/highlight.min.js",
    ),
}
DEFAULT_LAYERS = (
    {"id": "prod", "label": "Production code", "fallback": True},
    {
        "id": "tests",
        "label": "Tests",
        "match": (
            r"(^|/)(tests?|specs?|__tests__)/",
            r"(^|/)test_[^/]+$",
            r"[._-](test|spec)\.[^/.]+$",
        ),
    },
    {
        "id": "build",
        "label": "Build, CI and config",
        "match": (
            r"(^|/)\.github/",
            r"(^|/)(Makefile|Dockerfile|Justfile)$",
            r"\.(lock|toml|ya?ml|json|cfg|ini|gradle)$",
            r"(^|/)go\.(mod|sum)$",
        ),
    },
    {
        "id": "records",
        "label": "Docs and records",
        "match": (r"\.(md|mdx|rst|adoc|txt)$", r"^(docs?|meta)/"),
    },
)
SKELETON_HEAD = (
    '<!doctype html><html lang="en-GB"><head><meta charset="utf-8">'
    '<meta name="viewport" content="width=device-width,initial-scale=1,'
    'viewport-fit=cover"><style>:root{color-scheme:light;'
    "padding-top:env(safe-area-inset-top);"
    "padding-bottom:env(safe-area-inset-bottom)}"
    "body{margin:0;font:14px system-ui}img{max-width:100%}"
    "[hidden]{display:none!important}</style>"
)
SKELETON_TAIL = "</body></html>\n"


def load_json(path: Path, default: object) -> object:
    return json.loads(path.read_text()) if path.exists() else default


def layer_of(path: str, layers: list[dict]) -> str:
    for layer in layers:
        if layer.get("fallback"):
            continue
        if any(re.search(rule, path) for rule in layer.get("match", ())):
            return layer["id"]
    return next(layer["id"] for layer in layers if layer.get("fallback"))


def last_line(entry: dict, side: str) -> int:
    last = 0
    for hunk in entry.get("hunks", []):
        old, new = hunk["o"], hunk["n"]
        for line in hunk["l"]:
            old += line[0] != "+"
            new += line[0] != "-"
        last = max(last, (new if side == "new" else old) - 1)
    return last


def ordered_chapters(manifest: dict, content: Path) -> list[dict]:
    available = {
        path.stem.removeprefix("chapter-"): path
        for path in sorted(content.glob("chapter-*.json"))
    }
    order = [name for name in manifest.get("chapters", []) if name in available]
    order += [name for name in available if name not in order]
    return [json.loads(available[name].read_text()) for name in order]


def concatenated(content: Path, pattern: str) -> list[dict]:
    rows: list[dict] = []
    for path in sorted(content.glob(pattern)):
        rows += json.loads(path.read_text())
    return rows


def pr_details(manifest: dict, diff: dict) -> dict:
    pr = dict(manifest.get("pr", {}))
    if pr.get("repo"):
        pr.setdefault("repoUrl", f"https://github.com/{pr['repo']}")
        pr.setdefault("url", f"{pr['repoUrl']}/pull/{pr.get('number')}")
    pr["files"] = len(diff)
    pr["additions"] = sum(entry.get("add", 0) for entry in diff.values())
    pr["deletions"] = sum(entry.get("del", 0) for entry in diff.values())
    return pr


def file_entries(diff: dict, summaries: list[dict], layers: list) -> list:
    by_path = {entry["path"]: entry for entry in summaries}
    files = []
    for path, entry in diff.items():
        summary = by_path.get(path, {"risk": "skim", "summary": ""})
        files.append(
            {
                "chapter": None,
                **summary,
                "path": path,
                "layer": layer_of(path, layers),
                "status": entry.get("status", "modified"),
                "binary": entry.get("binary", False),
                "add": entry.get("add", 0),
                "del": entry.get("del", 0),
            }
        )
    return files


COMMIT_SUMMARY = ("sha", "subject", "message", "author", "date", "merge")


def commit_summaries(commits: dict) -> list[dict]:
    return [
        {key: commit.get(key) for key in COMMIT_SUMMARY}
        | {
            "files": len(commit["files"]),
            "add": sum(f.get("add", 0) for f in commit["files"]),
            "del": sum(f.get("del", 0) for f in commit["files"]),
        }
        for commit in commits.get("commits", [])
    ]


def changed_keys(entry: dict) -> set[str]:
    keys, old, new = set(), 0, 0
    for hunk in entry.get("hunks", []):
        old, new = hunk["o"], hunk["n"]
        for line in hunk["l"]:
            if line[0] == "+":
                keys.add(f"new:{new}")
            elif line[0] == "-":
                keys.add(f"old:{old}")
            old += line[0] != "+"
            new += line[0] != "-"
    return keys


def commit_warnings(commits: dict, diff: dict) -> list[str]:
    """Return a warning for every traced line the whole diff leaves alone."""
    warnings = []
    for commit in commits.get("commits", []):
        for file in commit["files"]:
            whole = changed_keys(diff.get(file["headPath"], {}))
            warnings += [
                f"commit {commit['sha'][:12]}: {file['headPath']} {fate} "
                "is not a changed line of the whole diff"
                for fate in file.get("fate", [])
                if ":" in fate and fate not in whole
            ]
    return warnings


def assemble(workspace: Path) -> tuple[dict, list[str]]:
    """Merge the workspace's content model; return it and its warnings."""
    manifest = load_json(workspace / "explore.json", {})
    content = workspace / "content"
    diff = load_json(workspace / "data/diff.json", {})
    commits = load_json(workspace / "data/commits.json", {})
    layers = list(manifest.get("layers") or DEFAULT_LAYERS)
    summaries = concatenated(content, "files*.json")
    pr = pr_details(manifest, diff)
    model = {
        "title": manifest.get("title") or f"PR {pr.get('number')} explorer",
        "pr": pr,
        "overview": load_json(content / "overview.json", {}),
        "design": load_json(content / "design.json", {}),
        "chapters": ordered_chapters(manifest, content),
        "ledger": concatenated(content, "ledger*.json"),
        "ledgerMeta": manifest.get("ledger", {}),
        "files": file_entries(diff, summaries, layers),
        "findings": load_json(content / "findings.json", []),
        "findingsMeta": manifest.get("findings", {}),
        "tourMeta": manifest.get("tour", {}),
        "breaking": load_json(content / "breaking.json", []),
        "ci": load_json(content / "ci.json", {"checks": []}),
        "acceptedRisks": load_json(content / "accepted-risks.json", []),
        "acs": load_json(content / "criteria.json", []),
        "moduleMap": load_json(content / "modmap.json", None),
        "layers": [{"id": x["id"], "label": x["label"]} for x in layers],
        "pathAliases": manifest.get("pathAliases", []),
        "commits": commit_summaries(commits),
    }
    model["keyAbstractions"] = key_abstractions(
        model["design"], model["chapters"]
    )
    model["collaboratorKinds"] = collaborator_kinds(
        model["design"], model["chapters"]
    )
    summarised = {entry["path"] for entry in summaries}
    warnings = check(model, diff) + commit_warnings(commits, diff)
    warnings += [
        f"{path}: no summary in files*.json; shown as skim"
        for path in diff
        if path not in summarised
    ]
    return model, warnings


def stop_warnings(chapter: dict, diff: dict) -> list[str]:
    warnings = []
    for stop in chapter.get("stops", []):
        side = stop.get("side", "new")
        if stop["file"] not in diff:
            warnings.append(f"stop {stop['id']}: {stop['file']} not in diff")
            continue
        last = last_line(diff[stop["file"]], side)
        if not 1 <= stop["start"] <= stop["end"] <= last:
            warnings.append(
                f"stop {stop['id']}: lines {stop['start']}-{stop['end']} "
                f"outside 1-{last} of {stop['file']}"
            )
    return warnings


def ledger_warnings(row: dict, diff: dict) -> list[str]:
    name, path, call = f"ledger {row['id']}", row["file"], row["line"]
    if path not in diff:
        return [f"{name}: {path} not in diff"]
    last = last_line(diff[path], "new")
    start, end = row.get("start", call), row.get("end", call)
    if not 1 <= call <= last:
        return [f"{name}: line {call} outside"]
    if not 1 <= start <= end <= last:
        return [f"{name}: lines {start}-{end} outside 1-{last} of {path}"]
    if not start <= call <= end:
        return [f"{name}: lines {start}-{end} do not contain line {call}"]
    return []


def finding_references(model: dict) -> list[tuple[str, str]]:
    references = []
    for chapter in model["chapters"]:
        where = f"chapter {chapter['id']}"
        references += [
            (where, c["id"]) for c in chapter.get("findingChecks", [])
        ]
        for item in chapter.get("stops", []) + chapter.get("risks", []):
            ids = item.get("findings", []) + item.get("refs", [])
            references += [(where, ref) for ref in ids]
    for row in model["ledger"]:
        references += [
            (f"ledger {row['id']}", r) for r in row.get("findings", [])
        ]
    for risk in model["acceptedRisks"]:
        references += [
            (f"risk {risk['id']}", r) for r in risk.get("findings", [])
        ]
    return references


FIGURE_KINDS = frozenset(
    {"flow", "sequence", "tree", "matrix", "timeline", "compare", "svg"}
)


def placed_figures(model: dict) -> list[tuple[str, dict]]:
    placed = [("overview", f) for f in model["overview"].get("figures", [])]
    placed += [("design", f) for f in model["design"].get("figures", [])]
    for chapter in model["chapters"]:
        where = f"chapter {chapter['id']}"
        placed += [(where, f) for f in chapter.get("figures", [])]
        placed += [
            (f"stop {stop['id']}", stop["figure"])
            for stop in chapter.get("stops", [])
            if stop.get("figure")
        ]
    return placed


def flattened(figure: dict) -> list[dict]:
    if figure.get("kind") != "compare":
        return [figure]
    sides = [figure.get(side) for side in ("before", "after")]
    nested = [
        f for side in sides if isinstance(side, dict) for f in flattened(side)
    ]
    return [figure, *nested]


def flow_problems(figure: dict) -> list[str]:
    nodes = {node["id"] for node in figure.get("nodes", [])}
    layers = {layer["id"] for layer in figure.get("layers", [])}
    problems = [
        f"edge names missing node {end}"
        for edge in figure.get("edges", [])
        for end in (edge.get("from"), edge.get("to"))
        if end not in nodes
    ]
    if layers:
        problems += [
            f"node {node['id']} names missing layer {node['layer']}"
            for node in figure.get("nodes", [])
            if node.get("layer") and node["layer"] not in layers
        ]
    return problems


def sequence_problems(figure: dict) -> list[str]:
    actors = {actor["id"] for actor in figure.get("actors", [])}
    named: list[str] = []
    for step in figure.get("steps", []):
        named += step.get("over", [])
        named += [step[end] for end in ("from", "to") if end in step]
    return [
        f"step names missing actor {actor}"
        for actor in dict.fromkeys(named)
        if actor not in actors
    ]


def matrix_problems(figure: dict) -> list[str]:
    labels, columns = figure.get("rows", []), len(figure.get("cols", []))
    cells = figure.get("cells", [])
    problems = []
    if len(cells) != len(labels):
        problems.append(f"has {len(cells)} rows of cells for {len(labels)}")
    problems += [
        f"row {number} has {len(row)} cells for {columns} columns"
        for number, row in enumerate(cells, 1)
        if len(row) != columns
    ]
    return problems


KIND_PROBLEMS = {
    "flow": flow_problems,
    "sequence": sequence_problems,
    "matrix": matrix_problems,
}


def figure_warnings(model: dict) -> list[str]:
    """Return warnings for missing, malformed or duplicated figures."""
    warnings = []
    ids = []
    for where, placed in placed_figures(model):
        for figure in flattened(placed):
            name, kind = figure.get("id", "?"), figure.get("kind")
            ids.append(name)
            if kind not in FIGURE_KINDS:
                warnings.append(
                    f"{where}: figure {name} has unknown kind {kind}"
                )
                continue
            problems = KIND_PROBLEMS.get(kind, lambda _: [])(figure)
            warnings += [f"{where}: figure {name} {p}" for p in problems]
    warnings += [
        f"duplicate figure id {name}"
        for name in sorted(set(ids))
        if ids.count(name) > 1
    ]
    warnings += [
        f"chapter {chapter['id']}: no figure; lead every chapter with one"
        for chapter in model["chapters"]
        if not chapter.get("figures")
    ]
    if not model["overview"].get("figures"):
        warnings.append("overview: no figure; open with the change at a glance")
    return warnings


DESIGN_KEYS = ("abstractions", "decisions")


def abstraction_name(item: dict) -> str:
    return item.get("name", "").replace("`", "").strip()


def collaborator_name(collaborator: str | dict) -> str:
    if isinstance(collaborator, dict):
        return collaborator.get("name", "")
    return collaborator


def declared_abstractions(design: dict, chapters: list[dict]) -> list[dict]:
    return design.get("abstractions", []) + [
        item for chapter in chapters for item in chapter.get("abstractions", [])
    ]


def collaborator_kinds(design: dict, chapters: list[dict]) -> dict[str, str]:
    """Return the kind of each collaborator that was given one."""
    return {
        collaborator["name"]: collaborator["kind"]
        for item in declared_abstractions(design, chapters)
        for collaborator in item.get("collaborators", [])
        if isinstance(collaborator, dict) and collaborator.get("kind")
    }


def key_abstractions(design: dict, chapters: list[dict]) -> list[dict]:
    """Each abstraction once, as first declared, with every chapter it is in."""
    declarations = [
        (item, [item["chapter"]] if item.get("chapter") else [])
        for item in design.get("abstractions", [])
    ] + [
        (item, [chapter["id"]])
        for chapter in chapters
        for item in chapter.get("abstractions", [])
    ]
    merged: dict[str, dict] = {}
    for item, chapter_ids in declarations:
        entry = merged.setdefault(
            abstraction_name(item),
            {"item": dict(item, collaborators=[]), "chapters": []},
        )
        collaborators = entry["item"]["collaborators"]
        collaborators += [
            name
            for name in map(collaborator_name, item.get("collaborators", []))
            if name not in collaborators
        ]
        entry["chapters"] += [
            chapter_id
            for chapter_id in chapter_ids
            if chapter_id not in entry["chapters"]
        ]
    return list(merged.values())


def design_items(model: dict) -> list[tuple[str, dict]]:
    items = [
        ("design", item)
        for key in DESIGN_KEYS
        for item in model["design"].get(key, [])
    ]
    for chapter in model["chapters"]:
        items += [
            (f"chapter {chapter['id']}", item)
            for key in DESIGN_KEYS
            for item in chapter.get(key, [])
        ]
    return items


def design_warnings(model: dict) -> list[str]:
    """Return warnings for a page that explains lines but not design."""
    warnings = []
    if not model["design"].get("summary"):
        warnings.append("design: no summary; explain the design first")
    warnings += [
        f"chapter {chapter['id']}: no abstractions or decisions; "
        "say what it adds to the design"
        for chapter in model["chapters"]
        if not any(chapter.get(key) for key in DESIGN_KEYS)
    ]
    warnings += [
        f"stop {stop['id']}: no fit; say where it sits in the bigger picture"
        for chapter in model["chapters"]
        for stop in chapter.get("stops", [])
        if not stop.get("fit")
    ]
    known = {
        abstraction_name(entry["item"]) for entry in model["keyAbstractions"]
    } | set(model["collaboratorKinds"])
    warnings += [
        f"abstraction {abstraction_name(entry['item'])}: collaborator "
        f'{name} has no kind; give it as {{"name", "kind"}}'
        for entry in model["keyAbstractions"]
        for name in entry["item"]["collaborators"]
        if name.replace("`", "").strip() not in known
    ]
    ids = [item["id"] for _, item in design_items(model) if item.get("id")]
    warnings += [
        f"duplicate design id {name}"
        for name in sorted(set(ids))
        if ids.count(name) > 1
    ]
    return warnings


LOCATION = re.compile(r"^([^:\s]+):(\d+)(?:-(\d+))?")
LOCATION_KEYS = ("where", "changelog")
CHAPTER_RUN = re.compile(
    r"\bchapters?\s+((?:`[^`]+`(?:\s*,\s*|\s*,?\s+(?:and|or|to|through)\s+)?)+)",
    re.IGNORECASE,
)
CHAPTER_NAMED = re.compile(r"`([^`]+)`\s+chapters?\b", re.IGNORECASE)
REFERENCE_ID = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*")


def sources(model: dict) -> list[tuple[str, object]]:
    return [
        ("overview", model["overview"]),
        ("design", model["design"]),
        *((f"chapter {c['id']}", c) for c in model["chapters"]),
        ("ledger", model["ledger"]),
        ("breaking", model["breaking"]),
        ("accepted risks", model["acceptedRisks"]),
        ("module map", model["moduleMap"] or {}),
    ]


def located(value: object, trail: str = "") -> Iterator[tuple[str, object]]:
    """Yield every dict and string under a value, with the path to it."""
    if isinstance(value, dict):
        yield trail, value
        for key, child in value.items():
            yield from located(child, within(trail, key))
    elif isinstance(value, list):
        for position, child in enumerate(value):
            name = (
                child.get("id", position)
                if isinstance(child, dict)
                else position
            )
            yield from located(child, f"{trail}[{name}]")
    elif isinstance(value, str):
        yield trail, value


def changed_files_named(path: str, diff: dict) -> list[str]:
    if path in diff:
        return [path]
    return [changed for changed in diff if changed.endswith(f"/{path}")]


def length_of(entry: dict) -> int:
    return last_line(entry, "new") or last_line(entry, "old")


def unresolved_file(reference: str, path: str, diff: dict) -> str | None:
    named = changed_files_named(path, diff)
    if not named:
        return f"{reference} is in {path}, not a changed file; unverified"
    if len(named) > 1:
        return f"{reference} matches more than one changed file: " + ", ".join(
            named
        )
    return None


def location_problem(key: str, value: str, diff: dict) -> str | None:
    reference = f"{key} {value}"
    located_at = LOCATION.match(value)
    if not located_at:
        return f"{reference} is not path:line"
    path, start = located_at[1], int(located_at[2])
    end = int(located_at[3] or start)
    unresolved = unresolved_file(reference, path, diff)
    if unresolved:
        return unresolved
    (changed,) = changed_files_named(path, diff)
    last = length_of(diff[changed])
    if not 1 <= start <= end <= last:
        return f"{reference} is outside 1-{last} of {changed}"
    return None


def head_text(entry: dict) -> str:
    return "\n".join(
        line[1:]
        for hunk in entry.get("hunks", [])
        for line in hunk["l"]
        if line[0] != "-"
    )


def test_problem(test: str | dict, diff: dict) -> str | None:
    if isinstance(test, dict):
        path, name, line = (
            test.get("file", ""),
            test.get("name"),
            test.get("line"),
        )
    else:
        path, _, qualified = test.partition("::")
        name, line = qualified.rpartition("::")[2], None
    reference = f"test {name or path}"
    unresolved = unresolved_file(reference, path, diff)
    if unresolved:
        return unresolved
    (changed,) = changed_files_named(path, diff)
    if name and name not in head_text(diff[changed]):
        return f"{reference} not found in {changed}"
    last = length_of(diff[changed])
    if line is not None and not 1 <= line <= last:
        return f"{reference} line {line} is outside 1-{last} of {changed}"
    return None


def within(trail: str, part: str) -> str:
    return f"{trail}.{part}" if trail else part


def at(trail: str, problem: str) -> str:
    return f"{trail}: {problem}" if trail else problem


def tests_named_by(trail: str, item: dict) -> list[tuple[str, str | dict]]:
    named = [(trail, test) for test in item.get("pinnedBy", [])]
    tests = item.get("tests", [])
    for position, test in enumerate(tests if isinstance(tests, list) else []):
        if isinstance(test, str):
            named.append((trail, test))
        elif "file" in test:
            named.append((within(trail, f"tests[{position}]"), test))
    return named


def chapters_named_in(prose: str) -> list[str]:
    named = CHAPTER_NAMED.findall(prose)
    for run in CHAPTER_RUN.findall(prose):
        named += re.findall(r"`([^`]+)`", run)
    return [name for name in named if REFERENCE_ID.fullmatch(name)]


def stops_named_in(prose: str, chapters: set[str]) -> list[str]:
    if not chapters:
        return []
    stop_shaped = re.compile(
        rf"(?<![\w-])(?:{'|'.join(map(re.escape, chapters))})-s\d+(?![\w-])"
    )
    return stop_shaped.findall(prose)


def prose_problems(
    prose: str, chapters: set[str], stops: set[str]
) -> list[str]:
    unknown_stops = [
        f"names unknown stop {stop}"
        for stop in stops_named_in(prose, chapters)
        if stop not in stops
    ]
    unknown_chapters = [
        f"names unknown chapter {name}"
        for name in chapters_named_in(prose)
        if name not in chapters
    ]
    return list(dict.fromkeys(unknown_stops + unknown_chapters))


def item_problems(
    trail: str, item: dict, diff: dict, chapters: set[str]
) -> list[str]:
    problems = [
        (trail, location_problem(key, item[key], diff))
        for key in LOCATION_KEYS
        if isinstance(item.get(key), str)
    ]
    problems += [
        (where, test_problem(test, diff))
        for where, test in tests_named_by(trail, item)
    ]
    named = item.get("chapter")
    if isinstance(named, str) and named not in chapters:
        problems.append((trail, f"names missing chapter {named}"))
    return [at(where, problem) for where, problem in problems if problem]


def reference_warnings(model: dict, diff: dict) -> list[str]:
    """Return a warning for every citation the page could not open."""
    chapters = {chapter["id"] for chapter in model["chapters"]}
    stops = {
        stop["id"] for c in model["chapters"] for stop in c.get("stops", [])
    }
    warnings = []
    for source, content in sources(model):
        for trail, value in located(content):
            if isinstance(value, dict):
                problems = item_problems(trail, value, diff, chapters)
            else:
                problems = [
                    at(trail, problem)
                    for problem in prose_problems(value, chapters, stops)
                ]
            warnings += [f"{source}: {problem}" for problem in problems]
    warnings += [
        f"file {file['path']}: names missing chapter {file['chapter']}"
        for file in model["files"]
        if file.get("chapter") and file["chapter"] not in chapters
    ]
    return warnings


def check(model: dict, diff: dict) -> list[str]:
    """Return a warning for every reference the page could not honour."""
    warnings = []
    for chapter in model["chapters"]:
        warnings += stop_warnings(chapter, diff)
    known = {finding["id"] for finding in model["findings"]}
    warnings += [
        f"{where}: unknown finding {ref}"
        for where, ref in finding_references(model)
        if ref not in known
    ]
    for row in model["ledger"]:
        warnings += ledger_warnings(row, diff)
    warnings += figure_warnings(model)
    warnings += design_warnings(model)
    warnings += reference_warnings(model, diff)
    stop_ids = [s["id"] for c in model["chapters"] for s in c.get("stops", [])]
    ledger_ids = [row["id"] for row in model["ledger"]]
    for ids, kind in ((stop_ids, "stop"), (ledger_ids, "ledger row")):
        warnings += [
            f"duplicate {kind} id {i}"
            for i in sorted(set(ids))
            if ids.count(i) > 1
        ]
    return warnings


def inline_script(code: str) -> str:
    safe = re.sub(r"</(script)", r"<\\/\1", code, flags=re.IGNORECASE)
    return f"<script>\n{safe}\n</script>"


def inline_json(name: str, value: object) -> str:
    text = json.dumps(value, separators=(",", ":"), ensure_ascii=False)
    text = text.replace("</", "<\\/").replace("<!--", "<\\!--")
    return f"<script>window.{name}={text};</script>"


def with_views(page: str, views: list[str]) -> str:
    return page.replace(VIEWS_MARKER, "".join(map(inline_script, views)))


def with_page_models(page: str) -> str:
    models = (
        COVERAGE_MODEL,
        GRAPH_FOCUS_MODEL,
        GRAPH_ROUTES_MODEL,
        PROSE_REFERENCES_MODEL,
    )
    scripts = "".join(inline_script(model.read_text()) for model in models)
    return page.replace(MODELS_MARKER, scripts)


def standalone(
    page: str,
    content: dict,
    diff: dict,
    views: list[str],
    vendor: dict[str, str],
    title: str,
    *,
    commits: dict | None = None,
) -> str:
    """Return one HTML document that needs no server and no network."""
    page = page.replace(TITLE_PLACEHOLDER, html.escape(title))
    for name, (tag, _) in LIBRARIES.items():
        if name in vendor:
            page = page.replace(tag, inline_script(vendor[name]))
    data = inline_json("__EXPLORE_CONTENT", content)
    data += inline_json("__EXPLORE_DIFF", diff)
    if commits:
        data += inline_json("__EXPLORE_COMMITS", commits)
    page = with_views(page.replace(DATA_MARKER, data), views)
    page = with_page_models(page)
    head, separator, body = page.partition("</style>")
    if not separator:
        head, body = "", page
    return f"{SKELETON_HEAD}{head}{separator}</head><body>{body}{SKELETON_TAIL}"


def hosted(
    page: str,
    out_dir: Path,
    content: dict,
    diff: dict,
    views: list[str],
    title: str,
    *,
    commits: dict | None = None,
) -> None:
    """Write the page, its content, its diff and any commits as files."""
    out_dir.mkdir(parents=True, exist_ok=True)
    page = page.replace(TITLE_PLACEHOLDER, html.escape(title))
    page = with_views(page.replace(DATA_MARKER, ""), views)
    page = with_page_models(page)
    (out_dir / "index.html").write_text(page)
    (out_dir / "content.json").write_text(
        json.dumps(content, separators=(",", ":"))
    )
    (out_dir / "diff.json").write_text(json.dumps(diff, separators=(",", ":")))
    if commits:
        (out_dir / "commits.json").write_text(
            json.dumps(commits, separators=(",", ":"))
        )


def view_sources(workspace: Path, manifest: dict) -> list[str]:
    views = workspace / "views"
    named = manifest.get("views")
    paths = [views / name for name in named] if named else []
    paths = paths or sorted(views.glob("*.js"))
    return [path.read_text() for path in paths]


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("workspace", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--hosted", type=Path)
    parser.add_argument("--page", type=Path)
    args = parser.parse_args(argv)
    workspace = args.workspace
    content, warnings = assemble(workspace)
    for warning in warnings:
        print(f"warning: {warning}", file=sys.stderr)
    manifest = load_json(workspace / "explore.json", {})
    page_path = args.page or workspace / "page.html"
    if not page_path.exists():
        page_path = ASSETS / "page.html"
    page = page_path.read_text()
    views = view_sources(workspace, manifest)
    diff = load_json(workspace / "data/diff.json", {})
    commits = load_json(workspace / "data/commits.json", {})
    title = content["title"]
    if args.hosted:
        hosted(page, args.hosted, content, diff, views, title, commits=commits)
        extra = ", commits.json" if commits else ""
        print(f"wrote {args.hosted}/index.html, content.json, diff.json{extra}")
        return
    vendor = {
        name: (ASSETS / path).read_text()
        for name, (_, path) in LIBRARIES.items()
    }
    number = content["pr"].get("number")
    out = args.out or workspace / f"explore-pr-{number}.html"
    out.write_text(
        standalone(page, content, diff, views, vendor, title, commits=commits)
    )
    stops = sum(len(c.get("stops", [])) for c in content["chapters"])
    print(
        f"wrote {out} ({out.stat().st_size / 1_048_576:.1f} MiB): "
        f"{len(content['chapters'])} chapters, {stops} stops, "
        f"{len(content['ledger'])} ledger rows, "
        f"{len(content['findings'])} findings, {len(warnings)} warnings"
    )


if __name__ == "__main__":
    main()
