import json

DIFF = {"src/a.rs": {"status": "added", "add": 1, "del": 0, "hunks": []}}

FLOW = {
    "id": "f-flow",
    "kind": "flow",
    "title": "Who calls whom",
    "nodes": [{"id": "a", "label": "A"}, {"id": "b", "label": "B"}],
    "edges": [{"from": "a", "to": "b"}],
}


def workspace(root, chapters, overview=None):
    (root / "content").mkdir(parents=True)
    (root / "data").mkdir()
    (root / "data/diff.json").write_text(json.dumps(DIFF))
    (root / "explore.json").write_text(json.dumps({"pr": {"number": 1}}))
    (root / "content/files.json").write_text(
        json.dumps([{"path": "src/a.rs", "summary": "s", "risk": "low"}])
    )
    for chapter in chapters:
        (root / f"content/chapter-{chapter['id']}.json").write_text(
            json.dumps(chapter)
        )
    (root / "content/overview.json").write_text(
        json.dumps(overview or {"figures": [dict(FLOW, id="f-over")]})
    )
    return root


def warnings_for(tmp_path, script, chapters, overview=None):
    _, warnings = script("build").assemble(
        workspace(tmp_path, chapters, overview)
    )
    return warnings


def test_a_chapter_without_a_figure_is_reported(tmp_path, script):
    warnings = warnings_for(
        tmp_path, script, [{"id": "plain", "title": "Plain"}]
    )

    assert any("chapter plain" in w and "no figure" in w for w in warnings)


def test_an_overview_without_a_figure_is_reported(tmp_path, script):
    warnings = warnings_for(
        tmp_path,
        script,
        [{"id": "c", "title": "C", "figures": [FLOW]}],
        overview={"lede": "words only"},
    )

    assert any("overview" in w and "no figure" in w for w in warnings)


def test_a_well_formed_figure_raises_no_figure_warning(tmp_path, script):
    warnings = warnings_for(
        tmp_path, script, [{"id": "c", "title": "C", "figures": [FLOW]}]
    )

    assert not [w for w in warnings if "figure" in w]


def test_an_unknown_figure_kind_is_reported(tmp_path, script):
    odd = {"id": "f-odd", "kind": "hologram", "title": "?"}

    warnings = warnings_for(
        tmp_path, script, [{"id": "c", "title": "C", "figures": [odd]}]
    )

    assert any("f-odd" in w and "hologram" in w for w in warnings)


def test_a_flow_edge_to_a_missing_node_is_reported(tmp_path, script):
    broken = dict(FLOW, edges=[{"from": "a", "to": "ghost"}])

    warnings = warnings_for(
        tmp_path, script, [{"id": "c", "title": "C", "figures": [broken]}]
    )

    assert any("f-flow" in w and "ghost" in w for w in warnings)


def test_a_sequence_step_naming_a_missing_actor_is_reported(tmp_path, script):
    sequence = {
        "id": "f-seq",
        "kind": "sequence",
        "title": "Order of events",
        "actors": [{"id": "client", "label": "Client"}],
        "steps": [
            {"from": "client", "to": "server", "label": "request"},
            {"over": ["client", "db"], "note": "waits"},
        ],
    }

    warnings = warnings_for(
        tmp_path, script, [{"id": "c", "title": "C", "figures": [sequence]}]
    )

    assert any("f-seq" in w and "server" in w for w in warnings)
    assert any("f-seq" in w and "db" in w for w in warnings)


def test_a_matrix_whose_cells_do_not_fit_its_axes_is_reported(tmp_path, script):
    matrix = {
        "id": "f-mx",
        "kind": "matrix",
        "title": "Decisions",
        "rows": ["r1", "r2"],
        "cols": ["c1", "c2"],
        "cells": [["a", "b"], ["c"]],
    }

    warnings = warnings_for(
        tmp_path, script, [{"id": "c", "title": "C", "figures": [matrix]}]
    )

    assert any("f-mx" in w and "row 2" in w for w in warnings)


def test_figures_inside_a_comparison_and_on_a_stop_are_checked(
    tmp_path, script
):
    broken = dict(FLOW, id="f-after", edges=[{"from": "a", "to": "nowhere"}])
    compare = {
        "id": "f-cmp",
        "kind": "compare",
        "title": "Before and after",
        "before": dict(FLOW, id="f-before"),
        "after": broken,
    }
    stop = {
        "id": "s1",
        "file": "src/a.rs",
        "start": 1,
        "end": 1,
        "figure": dict(FLOW, id="f-stop", kind="cloud"),
    }
    chapter = {"id": "c", "title": "C", "figures": [compare], "stops": [stop]}

    warnings = warnings_for(tmp_path, script, [chapter])

    assert any("f-after" in w and "nowhere" in w for w in warnings)
    assert any("f-stop" in w and "cloud" in w for w in warnings)


def test_a_figure_id_used_twice_is_reported(tmp_path, script):
    chapters = [
        {"id": "c1", "title": "C1", "figures": [FLOW]},
        {"id": "c2", "title": "C2", "figures": [FLOW]},
    ]

    warnings = warnings_for(tmp_path, script, chapters)

    assert any("duplicate figure id f-flow" in w for w in warnings)
