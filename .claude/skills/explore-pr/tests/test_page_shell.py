import re
from pathlib import Path

import pytest

SHELL = Path(__file__).resolve().parent.parent / "assets/page.html"
INLINE_MARKDOWN_CONTAINER = re.compile(
    r'class="([^"]*)"[^>]*>\$\{md\([^)]*,\s*true\)'
)


def test_inline_markdown_never_sits_directly_in_a_markdown_block():
    containers = INLINE_MARKDOWN_CONTAINER.findall(SHELL.read_text())

    blocks = [classes for classes in containers if "md" in classes.split()]
    assert containers
    assert blocks == []


def chapter_layout():
    shell = SHELL.read_text()
    start = shell.index("function vChapter(ch) {")
    body = shell[start : shell.index("\n}\n", start)]
    return body[body.index("return `") :]


def test_a_chapter_defines_its_terms_before_the_prose_that_uses_them():
    layout = chapter_layout()

    glossary = layout.index("Terms this chapter introduces")
    assert layout.index('class="figs"') < glossary
    for prose in ("What it adds to the design", "Why it exists", "${ba}"):
        assert glossary < layout.index(prose), prose


def test_a_stop_places_itself_in_the_change_before_explaining_its_code():
    shell = SHELL.read_text()
    stop = shell[shell.index("const stops = (ch.stops") :]

    assert "s.fit" in stop
    assert stop.index("s.fit") < stop.index("md(s.note)")


def test_by_commit_sits_with_the_reference_views_after_all_files():
    shell = SHELL.read_text()
    routes = shell[shell.index("function routes() {") :]
    routes = routes[: routes.index("\n}\n")]

    by_commit = routes.index("['commits', 'By commit', 'Reference']")
    assert routes.index("['files', 'All files', 'Reference']") < by_commit


def renderer(name):
    shell = SHELL.read_text()
    start = shell.index(f"function {name}(")
    return shell[start : shell.index("\n}\n", start)]


@pytest.mark.parametrize("name", ["figFlow", "moduleMap"])
def test_a_graph_names_each_node_and_the_ends_of_each_edge(name):
    body = renderer(name)
    shell = SHELL.read_text()
    node_attrs = shell[shell.index("const nodeAttrs = ") :].split("\n")[0]

    assert "nodeAttrs(" in body
    assert "data-node=" in node_attrs
    assert "data-from=" in body and "data-to=" in body


@pytest.mark.parametrize(
    ("name", "marks_itself"),
    [("moduleMap", "data-graph="), ("figFlow", "edges.length > 0)")],
)
def test_a_graph_with_edges_marks_its_svg_for_tracing(name, marks_itself):
    assert marks_itself in renderer(name)
    assert "data-graph=" in renderer("svgFrame")


def test_a_figure_explains_tracing_once_however_many_graphs_it_draws():
    assert "traceHint" not in renderer("figFlow")
    assert "traceHint" in renderer("figureHtml")


@pytest.mark.parametrize("name", ["figFlow", "moduleMap"])
def test_a_linked_box_navigates_only_from_its_own_link_control(name):
    body = renderer(name)

    assert "nodeLink(" in body
    assert '<a href="#tour-' not in body


@pytest.mark.parametrize("name", ["figFlow", "moduleMap"])
def test_a_graph_routes_its_arrows_through_the_router(name):
    body = renderer(name)

    assert "GraphRoutes.plan(" in body
    assert "GraphRoutes.draw(" in body


def test_the_page_starts_tracing_graphs_once_it_loads():
    assert "GraphFocus.attach(document)" in SHELL.read_text()


def test_only_the_page_built_abstraction_diagram_rearranges_its_rows():
    assert "arrange: true" in renderer("relationsFigure")
    assert "f.arrange" in renderer("figFlow")


def test_every_box_in_the_abstraction_diagram_names_its_kind():
    body = renderer("relationsFigure")

    assert "sub: 'existing'" not in body
    assert "collaboratorKinds" in body


def test_the_legend_explains_an_unchanged_box_by_its_border():
    assert "existing: 'unchanged'" in renderer("legend")
