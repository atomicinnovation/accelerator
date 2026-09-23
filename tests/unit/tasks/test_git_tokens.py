"""Tests for the git-token guard in ``tasks/lint/git_tokens.py``.

Synthetic ``tmp_path`` skill trees exercise the blocklisted subcommands and a
clean skill; the real-tree assertion pins that ``skills/`` carries no direct git
token.
"""

from pathlib import Path
from unittest.mock import MagicMock

import pytest
from invoke import Context, Exit

from tasks.lint import git_tokens
from tasks.shared.sources import repo_root


@pytest.fixture
def ctx():
    return MagicMock(spec=Context)


def _skill(root: Path, name: str, body: str) -> None:
    path = root / "skills" / name / "SKILL.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(f"---\nname: {name}\n---\n{body}\n")


@pytest.mark.parametrize("subcommand", ["log", "rev-parse", "config"])
def test_a_blocklisted_git_subcommand_is_flagged(
    tmp_path: Path, subcommand: str
) -> None:
    _skill(tmp_path, "demo", f"Run `git {subcommand}` to inspect state.")
    found = git_tokens.violations(tmp_path)
    assert any("skills/demo/SKILL.md" in v for v in found)


def test_a_clean_skill_yields_none(tmp_path: Path) -> None:
    _skill(
        tmp_path,
        "demo",
        "Use the session's VCS log command (see the reference).",
    )
    assert git_tokens.violations(tmp_path) == []


def test_check_raises_when_violations_exist(ctx, mocker) -> None:
    mocker.patch.object(
        git_tokens, "violations", return_value=["skills/x/SKILL.md:1: y"]
    )
    with pytest.raises(Exit):
        git_tokens.check(ctx)


def test_check_passes_when_clean(ctx, mocker) -> None:
    mocker.patch.object(git_tokens, "violations", return_value=[])
    git_tokens.check(ctx)


def test_the_real_skills_tree_passes() -> None:
    assert git_tokens.violations(repo_root()) == []
