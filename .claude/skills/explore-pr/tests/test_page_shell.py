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


def stop_layout():
    shell = SHELL.read_text()
    start = shell.index("const stops = (ch.stops")
    return shell[start : shell.index("const ba = ", start)]


def test_a_stop_says_what_it_is_evidence_for_before_anything_else():
    stop = stop_layout()

    assert "EVIDENCE.carriedBy(s)" in stop
    assert stop.index("carriedLink") < stop.index("s.fit")


def test_a_stop_gives_its_role_then_why_then_the_risk_then_the_checks():
    stop = stop_layout()

    order = [
        stop.index(part)
        for part in ("md(s.fit)", "md(s.why)", "md(s.risk)", "stopChecks(s)")
    ]
    assert order == sorted(order)


def test_a_stop_no_longer_narrates_its_code():
    shell = SHELL.read_text()

    assert "md(s.note)" not in shell
    assert "s.check ?" not in shell


def test_a_check_asks_a_question_and_says_what_a_problem_looks_like():
    checks = renderer("stopChecks")

    assert "md(c.question)" in checks
    assert "whereBtn(c.where)" in checks
    assert "md(c.problem, true)" in checks


def test_a_stop_marks_the_lines_to_focus_on_in_its_code():
    assert "focus: s.focus" in stop_layout()
    slice_of_code = renderer("codeSlice")

    assert "focusRow(" in slice_of_code
    assert "JSON.stringify([path, start, end, side, opts])" in slice_of_code


def test_a_decision_and_an_abstraction_list_the_stops_that_show_them():
    assert "shownAt(d)" in renderer("decisionCard")
    assert "shownAt(a)" in renderer("abstractionCard")


def test_what_a_stop_carries_can_be_reached_from_it():
    shell = SHELL.read_text()

    for target in ('id="dec-', 'id="abs-', 'id="inv-', 'id="risk-'):
        assert target in shell, target
    assert "dataset.scrollTo" in shell


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


def test_how_to_use_a_figure_is_said_in_its_legend_not_by_the_page():
    shell = SHELL.read_text()

    assert "traceHint" not in shell
    assert "trace-hint" not in shell


def test_a_figure_shows_what_the_kit_wrote_about_its_symbols():
    assert "f.symbols || ''" in renderer("figDiagram")
    assert "f.symbols || ''" in renderer("figSelfDrawn")
    assert "symbols: live.symbolsOf(spec, scene)" in renderer("liveDiagram")


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


def test_a_view_can_have_any_diagram_laid_out_and_drawn_in_the_page():
    shell = SHELL.read_text()
    drawn = renderer("liveDiagram")

    assert "diagram: liveDiagram, drawInto" in shell
    assert "DiagramKit.live" in drawn
    assert "live.problemsIn(spec)" in drawn
    assert "await live.laidOut(spec)" in drawn
    assert "figureHtml(" in drawn


def test_a_diagram_that_cannot_be_drawn_says_why_where_it_would_be():
    drawn = renderer("liveDiagram")

    assert "built without the layout engine" in drawn
    assert "fig-error" in drawn


def test_an_element_being_drawn_into_says_so_until_it_is_done():
    into = renderer("drawInto")

    assert "data-drawing" in into
    assert "removeAttribute('data-drawing')" in into


def test_the_page_names_its_custom_views():
    assert "customViews: () =>" in SHELL.read_text()


def test_a_drawing_too_wide_for_half_the_row_is_given_the_whole_row():
    layout = renderer("figureHtml")
    spans = renderer("spansTheRow")

    assert "spansTheRow(f)" in layout
    assert "f.wide" in spans
    assert "f.scene.width > WIDEST_IN_HALF_A_ROW" in spans
    assert "const WIDEST_IN_HALF_A_ROW = " in SHELL.read_text()


def test_a_drawing_wider_than_its_row_shrinks_to_fit_within_a_limit():
    shell = SHELL.read_text()

    assert "--natural-width:${f.scene.width}px" in renderer("figDiagram")
    assert (
        "max-width: 100%" in shell[shell.index(".fig-scroll svg.dk {") :][:200]
    )
    assert "min-width: calc(var(--natural-width) * .6)" in shell


def test_a_comparison_names_each_side_once():
    side = renderer("figCompare")

    assert "withoutItsSide(sub.title, label)" in side
    stripped = renderer("withoutItsSide")
    assert "[:\u2014\u2013-]" in stripped


def test_views_use_the_width_of_the_screen_and_prose_keeps_a_readable_measure():
    shell = SHELL.read_text()
    view = shell[shell.index("\n.view {") :].split("\n")[1]

    assert "max-width" not in view
    assert ".md > :is(p, ul, ol, blockquote" in shell
    assert (
        "max-width: 72ch" in shell[shell.index(".md > :is(p, ul, ol") :][:160]
    )


def test_a_stop_with_a_wide_figure_shows_it_across_the_row_above_its_code():
    stop = chapter_layout_of_stops()

    assert "spansTheRow(s.figure)" in stop
    assert '<div class="stop-figure">' in stop
    assert stop.index('class="stop-figure"') < stop.index("codeSlice(")
    shell = SHELL.read_text()
    assert ".stop-figure, .stop-head { grid-column: 1 / -1; }" in shell


def chapter_layout_of_stops():
    shell = SHELL.read_text()
    start = shell.index("const stops = (ch.stops")
    return shell[start : shell.index("const ba = ", start)]
