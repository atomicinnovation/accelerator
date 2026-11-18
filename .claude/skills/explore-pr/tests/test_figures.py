import json

DIFF = {"src/a.rs": {"status": "added", "add": 1, "del": 0, "hunks": []}}

DRAWING = {
    "id": "f-drawing",
    "kind": "svg",
    "title": "What is left behind",
    "svg": "<svg viewBox='0 0 1 1'/>",
}
MISFIT = {
    "kind": "matrix",
    "title": "Decisions",
    "rows": ["r1", "r2"],
    "cols": ["c1"],
    "cells": [["a"]],
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
        json.dumps(overview or {"figures": [dict(DRAWING, id="f-over")]})
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
        [{"id": "c", "title": "C", "figures": [DRAWING]}],
        overview={"lede": "words only"},
    )

    assert any("overview" in w and "no figure" in w for w in warnings)


def test_a_well_formed_figure_raises_no_figure_warning(tmp_path, script):
    warnings = warnings_for(
        tmp_path, script, [{"id": "c", "title": "C", "figures": [DRAWING]}]
    )

    assert not [w for w in warnings if "figure" in w]


def test_an_unknown_figure_kind_is_reported(tmp_path, script):
    odd = {"id": "f-odd", "kind": "hologram", "title": "?"}

    warnings = warnings_for(
        tmp_path, script, [{"id": "c", "title": "C", "figures": [odd]}]
    )

    assert any("f-odd" in w and "hologram" in w for w in warnings)


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
    broken = dict(MISFIT, id="f-after")
    compare = {
        "id": "f-cmp",
        "kind": "compare",
        "title": "Before and after",
        "before": dict(DRAWING, id="f-before"),
        "after": broken,
    }
    stop = {
        "id": "s1",
        "file": "src/a.rs",
        "start": 1,
        "end": 1,
        "figure": dict(DRAWING, id="f-stop", kind="cloud"),
    }
    chapter = {"id": "c", "title": "C", "figures": [compare], "stops": [stop]}

    warnings = warnings_for(tmp_path, script, [chapter])

    assert any("f-after" in w and "rows of cells" in w for w in warnings)
    assert any("f-stop" in w and "cloud" in w for w in warnings)


def test_a_figure_id_used_twice_is_reported(tmp_path, script):
    chapters = [
        {"id": "c1", "title": "C1", "figures": [DRAWING]},
        {"id": "c2", "title": "C2", "figures": [DRAWING]},
    ]

    warnings = warnings_for(tmp_path, script, chapters)

    assert any("duplicate figure id f-drawing" in w for w in warnings)


def test_a_flow_is_reported_as_a_kind_the_page_no_longer_draws(
    tmp_path, script
):
    flow = {
        "id": "f-flow",
        "kind": "flow",
        "title": "Who calls whom",
        "nodes": [{"id": "a", "label": "A"}, {"id": "b", "label": "B"}],
        "edges": [{"from": "a", "to": "b"}],
    }

    warnings = warnings_for(
        tmp_path, script, [{"id": "c", "title": "C", "figures": [flow]}]
    )

    [warning] = [w for w in warnings if "f-flow" in w]
    assert "flow is no longer drawn" in warning
    assert "block-diagram" in warning
    assert "draw-diagram/reference/choosing.md" in warning
