from pathlib import Path

import pytest

from tests.integration.support.characterisation import Repository
from tests.integration.support.tracker_blocks import (
    block_case_id,
    block_cases,
)


def _write_team_config(repository: Repository, content: str) -> None:
    repository.write(".accelerator/config.md", content)
    repository.commit()


def _dump(run, binaries, cwd: Path, *flags: str):
    return run(binaries.launcher, "config", "dump", *flags, cwd=cwd)


@pytest.mark.parametrize("case", block_cases(), ids=block_case_id)
def test_dump_renders_tracker_blocks(
    repository, binaries, run, matches_golden, case
):
    tracker, _, blocks = case
    _write_team_config(repository, blocks.frontmatter(tracker))

    matches_golden(_dump(run, binaries, repository.root))


@pytest.mark.parametrize("case", block_cases(), ids=block_case_id)
def test_dump_under_fail_safe(repository, binaries, run, matches_golden, case):
    tracker, _, blocks = case
    _write_team_config(repository, blocks.frontmatter(tracker))

    matches_golden(_dump(run, binaries, repository.root, "--fail-safe"))


@pytest.mark.parametrize("tracker", ["jira", "linear"])
def test_a_personal_block_replaces_the_team_block(
    repository, binaries, run, matches_golden, tracker
):
    noun = "projects" if tracker == "jira" else "teams"
    _write_team_config(
        repository,
        f"---\nwork:\n  integration: {tracker}\n{tracker}:\n"
        f"  pull:\n    additional_{noun}: [team]\n    max_items: 5\n---\n",
    )
    repository.untrack(".accelerator/config.local.md")
    personal = repository.write(
        ".accelerator/config.local.md",
        f"---\n{tracker}:\n  pull:\n    all_{noun}: true\n---\n",
    )
    personal.chmod(0o600)

    matches_golden(_dump(run, binaries, repository.root))
