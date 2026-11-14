import json
import shutil

import pytest

pytestmark = pytest.mark.skipif(
    shutil.which("node") is None, reason="needs node"
)

DIFF = {
    "src/core.rs": {
        "status": "added",
        "add": 3,
        "del": 0,
        "hunks": [{"o": 0, "n": 1, "l": ["+fn a() {", "+    b();", "+}"]}],
    },
}
GRAPH = {
    "id": "g1",
    "kind": "dependency-graph",
    "title": "Core needs ports",
    "modules": [{"id": "core", "status": "new"}, {"id": "ports"}],
    "dependencies": [{"from": "core", "to": "ports"}],
}
CHAPTER = {
    "id": "first",
    "title": "First",
    "figures": [GRAPH],
    "decisions": [{"id": "d1", "decision": "d"}],
    "stops": [],
}


def graph(**fields):
    return {**GRAPH, **fields}


def chapter(**fields):
    return {"chapter-first": {**CHAPTER, **fields}}


def assembled(root, script, **content):
    (root / "content").mkdir(parents=True)
    (root / "data").mkdir()
    (root / "data/diff.json").write_text(json.dumps(DIFF))
    (root / "explore.json").write_text(json.dumps({"pr": {"number": 1}}))
    files = [{"path": "src/core.rs", "summary": "s", "risk": "high"}]
    base = {
        "chapter-first": CHAPTER,
        "overview": {"figures": [graph(id="g0")]},
        "design": {"summary": "s"},
        "files": files,
    }
    for name, value in {**base, **content}.items():
        (root / f"content/{name}.json").write_text(json.dumps(value))
    return script("build").assemble(root)


def first_figure(model):
    return model["chapters"][0]["figures"][0]


def test_a_diagram_of_a_kind_the_kit_draws_is_laid_out(tmp_path, script):
    model, warnings = assembled(tmp_path, script)

    scene = first_figure(model)["scene"]
    assert [node["id"] for node in scene["nodes"]] == ["core", "ports"]
    assert len(scene["edges"][0]["points"]) >= 2
    assert warnings == []


def test_a_laid_out_diagram_carries_its_legend_and_remarks(tmp_path, script):
    looped = graph(dependencies=[{"from": "core", "to": "core"}])

    model, _ = assembled(tmp_path, script, **chapter(figures=[looped]))

    figure = first_figure(model)
    assert "new in this change" in figure["legend"]
    assert figure["remarks"] == ["Cycle: core → core"]


def test_a_problem_the_kit_finds_is_a_warning_on_the_figure(tmp_path, script):
    broken = graph(dependencies=[{"from": "core", "to": "nowhere"}])

    model, warnings = assembled(tmp_path, script, **chapter(figures=[broken]))

    assert warnings == [
        "chapter first: figure g1: dependency core->nowhere: "
        'no module "nowhere"'
    ]
    assert first_figure(model)["scene"] is None


def test_a_kind_nothing_draws_is_reported_with_the_kinds_that_are(
    tmp_path, script
):
    odd = {"id": "f-odd", "kind": "hologram", "title": "t"}

    _, warnings = assembled(tmp_path, script, **chapter(figures=[odd]))

    (warning,) = warnings
    assert warning.startswith(
        'chapter first: figure f-odd: no figure of kind "hologram"; '
    )
    assert "dependency-graph" in warning
    assert "sequence" in warning


def test_a_diagram_inside_a_comparison_or_on_a_stop_is_laid_out(
    tmp_path, script
):
    compare = {
        "id": "c1",
        "kind": "compare",
        "before": graph(id="g-before"),
        "after": graph(id="g-after"),
    }
    stop = {
        "id": "s1",
        "file": "src/core.rs",
        "start": 1,
        "end": 3,
        "anchor": "fn a()",
        "fit": "f",
        "figure": graph(id="g-stop"),
    }

    model, warnings = assembled(
        tmp_path, script, **chapter(figures=[compare], stops=[stop])
    )

    assert warnings == []
    assert first_figure(model)["after"]["scene"]["nodes"]
    assert model["chapters"][0]["stops"][0]["figure"]["scene"]["nodes"]


def test_a_link_inside_a_diagram_is_checked_like_any_citation(tmp_path, script):
    modules = [
        {"id": "core", "link": "src/core.rs:2"},
        {"id": "ports", "link": "src/core.rs:99"},
        {"id": "docs", "link": "https://example.test/docs"},
        {"id": "tour", "link": "chapter:first"},
        {"id": "lost", "link": "chapter:nope"},
    ]
    linked = graph(modules=modules, dependencies=[])

    _, warnings = assembled(tmp_path, script, **chapter(figures=[linked]))

    assert warnings == [
        "chapter first: figures[g1].modules[ports]: link src/core.rs:99 "
        "is outside 1-3 of src/core.rs",
        "chapter first: figures[g1].modules[lost]: names missing chapter nope",
    ]


def test_the_abstractions_are_drawn_as_a_class_diagram_of_what_uses_what(
    tmp_path, script
):
    abstractions = [
        {
            "id": "a1",
            "name": "`Claims`",
            "kind": "type",
            "status": "new",
            "where": "src/core.rs:1",
            "collaborators": ["Clock", {"name": "Disk", "kind": "port"}],
        },
        {"id": "a2", "name": "Clock", "kind": "port", "status": "existing"},
    ]

    model, warnings = assembled(
        tmp_path, script, **chapter(abstractions=abstractions)
    )

    relations = model["design"]["relations"]
    assert warnings == []
    assert relations["kind"] == "class-diagram"
    assert [
        (c["id"], c.get("stereotype"), c.get("status"), c.get("link"))
        for c in relations["classes"]
    ] == [
        ("Claims", "type", "new", "chapter:first"),
        ("Clock", "port", "existing", "chapter:first"),
        ("Disk", "port", "existing", None),
    ]
    assert [
        (r["from"], r["to"], r["kind"]) for r in relations["relationships"]
    ] == [
        ("Claims", "Clock", "dependency"),
        ("Claims", "Disk", "dependency"),
    ]
    assert relations["scene"]["nodes"]


def related(relation):
    return [
        {
            "id": "a1",
            "name": "DiskStore",
            "kind": "adapter",
            "collaborators": [
                {"name": "Store", "kind": "port", "relation": relation}
            ],
        },
        {"id": "a2", "name": "Store", "kind": "port"},
    ]


@pytest.mark.parametrize(
    ("relation", "drawn_as"),
    [
        ("implements", "realisation"),
        ("extends", "generalisation"),
        ("owns", "composition"),
        ("holds", "aggregation"),
        ("uses", "dependency"),
    ],
)
def test_a_collaborator_is_drawn_by_how_it_is_related(
    tmp_path, script, relation, drawn_as
):
    model, warnings = assembled(
        tmp_path, script, **chapter(abstractions=related(relation))
    )

    [relationship] = model["design"]["relations"]["relationships"]
    assert warnings == []
    assert (relationship["from"], relationship["to"]) == ("DiskStore", "Store")
    assert relationship["kind"] == drawn_as


def test_a_relation_the_page_has_no_word_for_is_reported(tmp_path, script):
    _, warnings = assembled(
        tmp_path, script, **chapter(abstractions=related("wraps"))
    )

    assert any(
        'relation "wraps" is not one of implements, extends, owns, holds, uses'
        in warning
        for warning in warnings
    )


def test_two_declarations_that_disagree_keep_the_first_and_say_so(
    tmp_path, script
):
    again = dict(related("uses")[0], id="a3")

    model, warnings = assembled(
        tmp_path,
        script,
        **chapter(abstractions=[*related("implements"), again]),
    )

    [relationship] = model["design"]["relations"]["relationships"]
    assert relationship["kind"] == "realisation"
    assert [type(w).__name__ for w in warnings] == ["Note"]
    assert "DiskStore" in warnings[0] and "Store" in warnings[0]
    assert "implements" in warnings[0] and "uses" in warnings[0]


def test_abstractions_that_use_nothing_have_no_graph(tmp_path, script):
    lone = [{"id": "a1", "name": "Claims"}, {"id": "a2", "name": "Clock"}]

    model, _ = assembled(tmp_path, script, **chapter(abstractions=lone))

    assert "relations" not in model["design"]


def test_the_module_map_is_drawn_as_a_dependency_graph(tmp_path, script):
    modmap = {
        "title": "How the modules fit",
        "caption": "c",
        "layers": [
            {"id": "entry", "label": "Entry"},
            {"id": "core", "label": "Core"},
        ],
        "nodes": [
            {
                "id": "start_up",
                "layer": "entry",
                "label": "start_up",
                "sub": "claims",
                "isNew": True,
                "chapter": "first",
            },
            {"id": "retention", "layer": "core", "label": "retention"},
            {"id": "loose", "label": "loose"},
        ],
        "edges": [["start_up", "retention"]],
    }

    model, warnings = assembled(tmp_path, script, modmap=modmap)

    drawn = model["moduleMap"]
    assert warnings == []
    assert (drawn["kind"], drawn["title"]) == (
        "dependency-graph",
        "How the modules fit",
    )
    assert [(g["id"], g["label"]) for g in drawn["modules"][:2]] == [
        ("layer:entry", "Entry"),
        ("layer:core", "Core"),
    ]
    assert drawn["modules"][0]["modules"] == [
        {
            "id": "start_up",
            "label": "start_up",
            "detail": "claims",
            "status": "new",
            "link": "chapter:first",
        }
    ]
    assert drawn["modules"][2] == {"id": "loose", "label": "loose"}
    assert drawn["dependencies"] == [{"from": "start_up", "to": "retention"}]
    assert drawn["scene"]["nodes"]


def test_both_layouts_carry_the_kit_that_draws_diagrams(tmp_path, script):
    build = script("build")
    page = (
        "<style>body{}</style><!-- explore-pr:data -->"
        "<!-- explore-pr:models --><script>window.APP = true;</script>"
        "<!-- explore-pr:views -->"
    )

    standalone = build.standalone(page, {}, {}, [], {}, "t")
    build.hosted(page, tmp_path, {}, {}, [], "t")

    for html in (standalone, (tmp_path / "index.html").read_text()):
        assert (
            html.index("FONT_METRICS")
            < html.index("svgOf")
            < html.index("window.APP")
        )
        assert "attachTracing" in html
        assert ".dk .edge .line" in html
        assert "org.eclipse.elk" not in html


def test_only_the_standalone_page_embeds_the_fonts_diagrams_are_sized_for(
    tmp_path, script
):
    build = script("build")
    page = "<style>body{}</style><!-- explore-pr:models -->"

    standalone = build.standalone(page, {}, {}, [], {}, "t")
    build.hosted(page, tmp_path, {}, {}, [], "t")

    assert standalone.count("src:url(data:font/woff2;base64,") == 4
    assert 'font-family:"JetBrains Mono"' in standalone
    assert "data:font/woff2" not in (tmp_path / "index.html").read_text()


SEQUENCE = {
    "id": "s1",
    "kind": "sequence-diagram",
    "title": "Core asks the port",
    "participants": [{"id": "core"}, {"id": "port"}],
    "steps": [
        {
            "from": "core",
            "to": "port",
            "label": "read()",
            "reply": "bytes",
            "link": "src/core.rs:2",
            "steps": [
                {"from": "port", "to": "port", "label": "retry"},
                {"note": "At most once.", "over": ["port"]},
            ],
        }
    ],
}


def test_a_sequence_diagram_is_laid_out_by_the_kit(tmp_path, script):
    model, warnings = assembled(tmp_path, script, **chapter(figures=[SEQUENCE]))

    scene = first_figure(model)["scene"]
    assert warnings == []
    assert [edge["label"] for edge in scene["edges"]] == [
        "1: read()",
        "2: retry",
        "bytes",
    ]


def test_a_link_on_a_message_is_checked_however_deep_it_is(tmp_path, script):
    nested = SEQUENCE["steps"][0]["steps"][0]
    deep = {
        **SEQUENCE,
        "steps": [
            {
                **SEQUENCE["steps"][0],
                "steps": [{**nested, "link": "src/core.rs:40"}],
            }
        ],
    }

    _, warnings = assembled(tmp_path, script, **chapter(figures=[deep]))

    assert warnings == [
        "chapter first: figures[s1].steps[0].steps[0]: link src/core.rs:40 "
        "is outside 1-3 of src/core.rs"
    ]


def test_the_sequence_figure_this_page_once_drew_is_pointed_at_its_successor(
    tmp_path, script
):
    old = {"id": "f-old", "kind": "sequence", "title": "t", "actors": []}

    _, warnings = assembled(tmp_path, script, **chapter(figures=[old]))

    (warning,) = warnings
    assert 'no figure of kind "sequence"' in warning
    assert "sequence-diagram" in warning


def test_a_state_machine_is_laid_out_with_what_the_kit_remarks_on(
    tmp_path, script
):
    machine = {
        "id": "m1",
        "kind": "state-machine",
        "title": "A claim is held, then gone",
        "states": [
            {"id": "start", "kind": "initial"},
            {"id": "held", "link": "src/core.rs:1"},
            {"id": "lost"},
        ],
        "transitions": [{"from": "start", "to": "held"}],
    }

    model, warnings = assembled(tmp_path, script, **chapter(figures=[machine]))

    figure = first_figure(model)
    assert warnings == []
    assert [node["shape"] for node in figure["scene"]["nodes"]] == [
        "initial",
        "state",
        "state",
    ]
    assert figure["remarks"] == [
        "lost cannot be reached",
        "held has no way out and is not final",
        "lost has no way out and is not final",
    ]


PAGE = (
    "<style>body{}</style><!-- explore-pr:data -->"
    "<!-- explore-pr:models --><script>window.APP = true;</script>"
    "<!-- explore-pr:views -->"
)
TREE = {
    "id": "t1",
    "kind": "tree",
    "title": "What is left behind",
    "root": "src/",
    "items": [{"path": "core.rs", "state": "added", "link": "src/core.rs:1"}],
}


def test_a_figure_the_kit_draws_as_html_is_checked_but_given_no_scene(
    tmp_path, script
):
    model, warnings = assembled(tmp_path, script, **chapter(figures=[TREE]))

    figure = first_figure(model)
    assert warnings == []
    assert "scene" not in figure
    assert figure["remarks"] == []


def test_a_problem_in_a_self_drawn_figure_is_a_warning_on_it(tmp_path, script):
    broken = dict(TREE, items=[{"path": "core.rs", "where": "src/core.rs:1"}])

    _, warnings = assembled(tmp_path, script, **chapter(figures=[broken]))

    assert warnings == [
        "chapter first: figure t1: item core.rs: where is now link"
    ]


def test_a_link_inside_a_self_drawn_figure_is_checked(tmp_path, script):
    far = dict(TREE, items=[{"path": "core.rs", "link": "src/core.rs:99"}])

    _, warnings = assembled(tmp_path, script, **chapter(figures=[far]))

    assert any(
        "figures[t1].items[0]" in w and "outside 1-3" in w for w in warnings
    )


def test_both_layouts_carry_the_notations_that_draw_themselves(
    tmp_path, script
):
    build = script("build")

    page = build.standalone(PAGE, {}, {}, [], {}, "t")
    build.hosted(PAGE, tmp_path, {}, {}, [], "t")

    for html in (page, (tmp_path / "index.html").read_text()):
        for kind in ("matrix", "tree", "timeline"):
            assert f"kind: '{kind}'" in html, kind
        assert html.index("inlineOf") < html.index("kind: 'matrix'")
        assert html.index("kind: 'timeline'") < html.index("window.APP")


DRAWING_VIEW = (
    "ExplorePR.registerView({ mount(root, api) { api.drawInto(root, {}); } });"
)
PLAIN_VIEW = "ExplorePR.registerView({ render: api => api.figure({}) });"


def test_a_page_with_a_view_that_draws_diagrams_carries_the_layout_engine(
    tmp_path, script
):
    build = script("build")

    page = build.standalone(PAGE, {}, {}, [DRAWING_VIEW], {}, "t")
    build.hosted(PAGE, tmp_path, {}, {}, [DRAWING_VIEW], "t")

    for html in (page, (tmp_path / "index.html").read_text()):
        assert "org.eclipse.elk" in html
        assert html.index("live: live") < html.index("window.APP")
        assert html.index("window.APP") < html.index("api.drawInto")


def test_a_page_whose_views_draw_no_diagram_stays_small(tmp_path, script):
    build = script("build")

    page = build.standalone(PAGE, {}, {}, [PLAIN_VIEW], {}, "t")

    assert "org.eclipse.elk" not in page
    assert len(page) < 400_000


def test_a_view_may_ask_for_a_diagram_by_either_call(script):
    build = script("build")

    assert build.draws_diagrams(["api.diagram(spec).then(show)"])
    assert build.draws_diagrams(["x", "api.drawInto(el, spec)"])
    assert not build.draws_diagrams(["api.figure(spec)", "diagramOf(spec)"])
