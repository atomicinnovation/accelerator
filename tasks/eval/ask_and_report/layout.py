"""Where the reference set sits in every project a scenario builds."""

from pathlib import Path

SLUG = "llm-agent-planning"
SET = Path("meta/research/topics") / SLUG
REFERENCE_CORPUS = Path("skills/research/research-topic/evals")


def is_finding(path: str) -> bool:
    """Whether a set-relative path names a finding the research CLI reads."""
    directory, _, name = path.rpartition("/")
    index, separated, _ = name.partition("-")
    return (
        directory == "findings"
        and not name.startswith(".")
        and name.lower().endswith(".md")
        and bool(separated)
        and index.isascii()
        and index.isdigit()
    )
