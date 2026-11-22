"""Where the reference set sits in every project a scenario builds."""

from pathlib import Path

SLUG = "llm-agent-planning"
SET = Path("meta/research/topics") / SLUG
REFERENCE_CORPUS = Path("skills/research/research-topic/evals")
CANARY = "Lanternfish"


HIGHEST_INDEX = 2**32 - 1


def is_finding(path: str) -> bool:
    """Whether a set-relative path names a finding the research CLI reads.

    The rule is the CLI's, copied so ``--check-only`` can judge a session's
    snapshots without running the CLI; an integration test pins the two
    together.
    """
    directory, _, name = path.rpartition("/")
    index, separated, _ = name.partition("-")
    return (
        directory == "findings"
        and not name.startswith(".")
        and name.lower().endswith(".md")
        and bool(separated)
        and index.isascii()
        and index.isdigit()
        and int(index) <= HIGHEST_INDEX
    )
