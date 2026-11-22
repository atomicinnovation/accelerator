import pytest

from tasks.eval.ask_and_report.layout import is_finding


@pytest.mark.parametrize(
    "path", ["findings/01-a-web.md", "findings/07-a-web.MD"]
)
def test_a_numbered_visible_markdown_file_in_findings_is_a_finding(
    path: str,
) -> None:
    assert is_finding(path)


@pytest.mark.parametrize(
    "path",
    [
        "findings/notes.md",
        "findings/.03-c-web.md",
        "findings/01-a-web.txt",
        "findings/01-a-web.levels/1.md",
        "reports/01-a.md",
        "findings/4294967296-a-web.md",
    ],
)
def test_anything_else_is_no_finding(path: str) -> None:
    assert not is_finding(path)


def test_the_highest_index_the_cli_reads_is_a_finding() -> None:
    assert is_finding("findings/4294967295-a-web.md")
