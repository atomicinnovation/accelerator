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
