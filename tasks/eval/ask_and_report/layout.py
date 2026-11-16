"""Where the reference set sits in every project a scenario builds."""

from pathlib import Path

SLUG = "llm-agent-planning"
SET = Path("meta/research/topics") / SLUG
REFERENCE_CORPUS = Path("skills/research/research-topic/evals")
