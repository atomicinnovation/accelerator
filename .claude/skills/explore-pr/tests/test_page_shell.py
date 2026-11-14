import re
from pathlib import Path

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


def test_a_figure_explains_tracing_once_however_many_graphs_it_draws():
    assert "traceHint" in renderer("figureHtml")


def test_the_page_lays_out_and_routes_no_graph_of_its_own():
    shell = SHELL.read_text()

    assert "function figFlow" not in shell
    assert "GraphRoutes" not in shell
    assert not (SHELL.parent / "graph-routes.js").exists()


def test_the_page_starts_tracing_graphs_once_it_loads():
    assert "DiagramKit.attachTracing(document)" in SHELL.read_text()


def test_rendered_markdown_links_its_chapter_and_stop_references():
    shell = SHELL.read_text()
    start = shell.index("function md(s, inline) {")
    markdown = shell[start : shell.index("\n}\n", start)]

    assert "ProseReferences.link(" in markdown
    assert markdown.count("return ") == markdown.count("return linked(") + 1


def test_a_figure_the_kit_laid_out_is_drawn_by_the_kit():
    assert "'scene' in f ? figDiagram" in renderer("figureBody")
    drawing = renderer("figDiagram")

    assert "DiagramKit.svgOf(f.scene" in drawing
    assert "resolveLink: diagramLink" in drawing


def test_a_diagram_link_opens_a_chapter_a_line_or_an_address():
    link = renderer("diagramLink")

    assert "'#tour-'" in link
    assert "'data-open': w.path" in link
    assert "https?:" in link


def test_the_page_draws_no_generated_graph_of_its_own():
    shell = SHELL.read_text()

    assert "function relationsFigure" not in shell
    assert "function moduleMap" not in shell
    assert "figureHtml(d.relations)" in shell
    assert shell.count("figureHtml(C.moduleMap)") == 2


def test_figures_appear_only_where_they_explain_something():
    shell = SHELL.read_text()

    assert "In pictures" not in shell
    assert "function vPictures" not in shell
    assert "function allFigures" not in shell


def test_the_page_names_every_view_that_draws_a_figure():
    views = renderer("figureViews")

    assert "'overview'" in views
    assert "'design'" in views
    assert "'tour-' + ch.id" in views
    assert "figureViews: () =>" in SHELL.read_text()


def test_the_page_draws_only_comparisons_and_raw_svg_itself():
    shell = SHELL.read_text()

    for gone in (
        "function figTree",
        "function figMatrix",
        "function figTimeline",
    ):
        assert gone not in shell, gone
    assert "const FIGURES = { svg: figSvg, compare: figCompare };" in shell


def test_a_figure_whose_notation_draws_itself_is_drawn_from_its_spec():
    body = renderer("figureBody")

    assert "DiagramKit.NOTATIONS" in body
    assert "htmlOf(f, { resolveLink: diagramLink })" in renderer("figSelfDrawn")
