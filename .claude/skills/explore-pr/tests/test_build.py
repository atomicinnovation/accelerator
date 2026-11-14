import json
import re

PAGE = """<title>__EXPLORE_PR_TITLE__</title>
<style>body { color: red }</style>
<header>shell</header>
<script src="https://cdn.jsdelivr.net/npm/marked@12.0.2/marked.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
<!-- explore-pr:data -->
<!-- explore-pr:models -->
<script>window.APP = true;</script>
<!-- explore-pr:views -->
"""

DIFF = {
    "src/core.rs": {
        "status": "modified",
        "add": 1,
        "del": 1,
        "hunks": [{"o": 1, "n": 1, "l": [" a", "-b", "+c", " d"]}],
    },
    "tests/core_test.rs": {
        "status": "added",
        "add": 2,
        "del": 0,
        "hunks": [{"o": 0, "n": 1, "l": ["+x", "+y"]}],
    },
    "Cargo.lock": {"status": "modified", "add": 0, "del": 0, "hunks": []},
}


def workspace(root, manifest=None, chapters=None, findings=None, ledger=None):
    (root / "content").mkdir(parents=True)
    (root / "data").mkdir()
    (root / "views").mkdir()
    (root / "data/diff.json").write_text(json.dumps(DIFF))
    manifest = manifest or {
        "title": "PR 9 Explorer",
        "pr": {"number": 9, "repo": "o/r", "head": "h" * 40, "base": "b" * 40},
        "chapters": ["second", "first"],
    }
    (root / "explore.json").write_text(json.dumps(manifest))
    for chapter in chapters or [
        {"id": "first", "title": "First", "stops": []},
        {"id": "second", "title": "Second", "stops": []},
    ]:
        (root / f"content/chapter-{chapter['id']}.json").write_text(
            json.dumps(chapter)
        )
    (root / "content/findings.json").write_text(json.dumps(findings or []))
    (root / "content/ledger.json").write_text(json.dumps(ledger or []))
    (root / "content/files.json").write_text(
        json.dumps(
            [
                {"path": "src/core.rs", "risk": "high", "summary": "s"},
                {"path": "tests/core_test.rs", "risk": "low", "summary": "t"},
            ]
        )
    )
    return root


def test_chapters_follow_the_manifest_order(tmp_path, script):
    build = script("build")

    content, _ = build.assemble(workspace(tmp_path))

    assert [c["id"] for c in content["chapters"]] == ["second", "first"]


def test_files_get_a_layer_from_the_default_rules(tmp_path, script):
    build = script("build")

    content, _ = build.assemble(workspace(tmp_path))

    layers = {f["path"]: f["layer"] for f in content["files"]}
    assert layers == {
        "src/core.rs": "prod",
        "tests/core_test.rs": "tests",
        "Cargo.lock": "build",
    }


def test_manifest_layers_replace_the_defaults(tmp_path, script):
    build = script("build")
    manifest = {
        "title": "t",
        "pr": {"number": 9},
        "layers": [
            {"id": "core", "label": "Core", "match": ["^src/"]},
            {"id": "rest", "label": "Rest", "fallback": True},
        ],
    }

    content, _ = build.assemble(workspace(tmp_path, manifest=manifest))

    assert {f["path"]: f["layer"] for f in content["files"]}["Cargo.lock"] == (
        "rest"
    )
    assert content["layers"] == [
        {"id": "core", "label": "Core"},
        {"id": "rest", "label": "Rest"},
    ]


def test_a_file_without_a_summary_is_skimmed_and_reported(tmp_path, script):
    build = script("build")

    content, warnings = build.assemble(workspace(tmp_path))

    lock = next(f for f in content["files"] if f["path"] == "Cargo.lock")
    assert lock["risk"] == "skim"
    assert any("Cargo.lock" in w and "summary" in w for w in warnings)


def test_a_stop_outside_its_file_is_reported(tmp_path, script):
    build = script("build")
    chapter = {
        "id": "first",
        "title": "First",
        "stops": [{"id": "s1", "file": "src/core.rs", "start": 2, "end": 9}],
    }

    _, warnings = build.assemble(workspace(tmp_path, chapters=[chapter]))

    assert any("s1" in w and "outside 1-3" in w for w in warnings)


def test_a_ledger_range_outside_its_file_is_reported(tmp_path, script):
    build = script("build")
    row = {"id": "L1", "file": "src/core.rs", "line": 2, "start": 1, "end": 9}

    _, warnings = build.assemble(workspace(tmp_path, ledger=[row]))

    assert any("L1" in w and "lines 1-9 outside 1-3" in w for w in warnings)


def test_a_ledger_range_must_contain_its_call(tmp_path, script):
    build = script("build")
    row = {"id": "L1", "file": "src/core.rs", "line": 3, "start": 1, "end": 2}

    _, warnings = build.assemble(workspace(tmp_path, ledger=[row]))

    assert any("L1" in w and "do not contain line 3" in w for w in warnings)


def test_a_ledger_range_inside_its_file_is_accepted(tmp_path, script):
    build = script("build")
    row = {
        "id": "L1",
        "file": "src/core.rs",
        "line": 2,
        "start": 1,
        "end": 3,
        "anchor": "c",
    }

    _, warnings = build.assemble(workspace(tmp_path, ledger=[row]))

    assert not any("L1" in w for w in warnings)


def test_an_unknown_finding_reference_is_reported(tmp_path, script):
    build = script("build")
    chapter = {
        "id": "first",
        "title": "First",
        "findingChecks": [{"id": "R1-07", "verdict": "confirmed"}],
    }

    _, warnings = build.assemble(
        workspace(tmp_path, chapters=[chapter], findings=[{"id": "R1-01"}])
    )

    assert any("R1-07" in w for w in warnings)


def test_the_standalone_page_inlines_libraries_data_and_views(script):
    build = script("build")

    html = build.standalone(
        PAGE,
        {"pr": {"number": 9}, "note": "</script><b>"},
        {"f": {}},
        ["window.VIEW = 1;"],
        {"marked": "window.marked = {};", "highlight": "window.hljs = {};"},
        "PR 9 Explorer",
    )

    assert "cdn.jsdelivr.net" not in html
    assert "cdnjs.cloudflare.com" not in html
    assert "window.marked = {};" in html
    assert "<\\/script><b>" in html
    assert html.index("__EXPLORE_CONTENT") < html.index("window.APP")
    assert html.index("window.APP") < html.index("window.VIEW")


def test_both_layouts_carry_the_review_coverage_model(tmp_path, script):
    build = script("build")

    page = build.standalone(PAGE, {}, {}, [], {}, "t")
    build.hosted(PAGE, tmp_path, {}, {}, [], "t")

    for html in (page, (tmp_path / "index.html").read_text()):
        assert "explore-pr:models" not in html
        assert html.index("ReviewCoverage") < html.index("window.APP")


def test_both_layouts_carry_the_graph_focus_model(tmp_path, script):
    build = script("build")

    page = build.standalone(PAGE, {}, {}, [], {}, "t")
    build.hosted(PAGE, tmp_path, {}, {}, [], "t")

    for html in (page, (tmp_path / "index.html").read_text()):
        assert html.index("GraphFocus") < html.index("window.APP")


def test_both_layouts_carry_the_prose_references_model(tmp_path, script):
    build = script("build")

    page = build.standalone(PAGE, {}, {}, [], {}, "t")
    build.hosted(PAGE, tmp_path, {}, {}, [], "t")

    for html in (page, (tmp_path / "index.html").read_text()):
        assert html.index("ProseReferences") < html.index("window.APP")


def test_both_layouts_carry_the_graph_routes_model(tmp_path, script):
    build = script("build")

    page = build.standalone(PAGE, {}, {}, [], {}, "t")
    build.hosted(PAGE, tmp_path, {}, {}, [], "t")

    for html in (page, (tmp_path / "index.html").read_text()):
        assert html.index("GraphRoutes") < html.index("window.APP")


def test_the_standalone_page_is_a_complete_document(script):
    build = script("build")

    html = build.standalone(PAGE, {}, {}, [], {}, "PR 9 Explorer")

    head = html[: html.index("</head>")]
    assert html.startswith("<!doctype html>")
    assert "<title>PR 9 Explorer</title>" in head
    assert "body { color: red }" in head
    assert "[hidden]{display:none!important}" in head
    assert html.rstrip().endswith("</html>")


def test_inlined_code_cannot_close_its_script_element(script):
    build = script("build")

    html = build.standalone(PAGE, {}, {}, ['const s = "</script>";'], {}, "t")

    assert 'const s = "<\\/script>";' in html


def test_the_hosted_layout_keeps_cdn_libraries_and_splits_the_data(
    tmp_path, script
):
    build = script("build")

    build.hosted(PAGE, tmp_path, {"a": 1}, {"b": 2}, ["window.V=1;"], "T")

    index = (tmp_path / "index.html").read_text()
    assert "cdn.jsdelivr.net" in index
    assert "window.V=1;" in index
    assert "__EXPLORE_CONTENT" not in index
    assert json.loads((tmp_path / "content.json").read_text()) == {"a": 1}
    assert json.loads((tmp_path / "diff.json").read_text()) == {"b": 2}


def test_the_command_line_builds_the_bundled_page(tmp_path, script):
    build = script("build")
    ws = workspace(tmp_path / "ws")
    (ws / "views/extra.js").write_text("window.EXTRA_VIEW = 1;")
    out = tmp_path / "out.html"

    build.main([str(ws), "--out", str(out)])

    html = out.read_text()
    assert re.search(r"<title>PR 9 Explorer</title>", html)
    assert "window.EXTRA_VIEW = 1;" in html
    assert "explore-pr:data" not in html


COMMITS = {
    "commits": [
        {
            "sha": "c1",
            "subject": "Change the core",
            "message": "Change the core\n\nBecause.",
            "author": "a",
            "date": "d",
            "merge": False,
            "files": [
                {
                    "path": "src/core.rs",
                    "headPath": "src/core.rs",
                    "status": "modified",
                    "previous": None,
                    "binary": False,
                    "add": 1,
                    "del": 1,
                    "hunks": [{"o": 1, "n": 1, "l": [" a", "-b", "+c"]}],
                    "fate": ["old:2", "new:9"],
                }
            ],
        }
    ]
}


def with_commits(root):
    ws = workspace(root)
    (ws / "data/commits.json").write_text(json.dumps(COMMITS))
    return ws


def test_the_page_carries_a_summary_of_each_commit(tmp_path, script):
    build = script("build")

    content, _ = build.assemble(with_commits(tmp_path))

    assert content["commits"] == [
        {
            "sha": "c1",
            "subject": "Change the core",
            "message": "Change the core\n\nBecause.",
            "author": "a",
            "date": "d",
            "merge": False,
            "files": 1,
            "add": 1,
            "del": 1,
        }
    ]


def test_a_traced_line_the_whole_diff_does_not_change_is_reported(
    tmp_path, script
):
    build = script("build")

    _, warnings = build.assemble(with_commits(tmp_path))

    assert any("c1" in w and "new:9" in w for w in warnings)
    assert not any("old:2" in w for w in warnings)


def test_both_layouts_carry_the_commits_when_there_are_some(tmp_path, script):
    build = script("build")

    page = build.standalone(PAGE, {}, {}, [], {}, "t", commits=COMMITS)
    build.hosted(PAGE, tmp_path, {}, {}, [], "t", commits=COMMITS)

    assert "__EXPLORE_COMMITS" in page
    hosted = json.loads((tmp_path / "commits.json").read_text())
    assert hosted == COMMITS


def test_a_page_without_commits_has_no_commit_data(tmp_path, script):
    build = script("build")

    content, _ = build.assemble(workspace(tmp_path / "ws"))
    build.hosted(PAGE, tmp_path / "out", {}, {}, [], "t")

    assert content["commits"] == []
    assert not (tmp_path / "out/commits.json").exists()
