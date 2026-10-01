"""Every fetch an academic profile shows passes the built research guard.

A profile teaches the researcher its calls by example, so a documented
invocation the guard blocks would leave the researcher unable to follow its
own profile. Both the fenced commands and the worked single-quoted queries are
run through the guard as a researcher's `Bash` call.
"""

import json
import os
import re
import subprocess
from pathlib import Path

import pytest

from tasks.shared.skill_parsing import fenced_block_commands

REPO_ROOT = Path(__file__).resolve().parents[3]
GUARD = Path(
    os.environ.get(
        "ACCELERATOR_RESEARCH_BIN",
        REPO_ROOT / "cli/target/debug/accelerator-research",
    )
)
WORKED_QUERY = re.compile(r"`'([^`]*)'`")


def _invocations(family: str) -> list[str]:
    text = (
        REPO_ROOT / f"skills/research/profiles/{family}-profile/SKILL.md"
    ).read_text(encoding="utf-8")
    worked = [
        f"accelerator research fetch {family} search '{query}'"
        for query in WORKED_QUERY.findall(text)
    ]
    return fenced_block_commands(text) + worked


def _guard(command: str, cwd: Path) -> subprocess.CompletedProcess[str]:
    payload = {
        "agent_id": "profile-invocations",
        "agent_type": "accelerator:researcher",
        "tool_name": "Bash",
        "cwd": str(cwd),
        "tool_input": {"command": command},
    }
    return subprocess.run(
        [str(GUARD), "guard"],
        input=json.dumps(payload),
        capture_output=True,
        text=True,
        check=False,
    )


def _require_guard() -> None:
    assert GUARD.is_file(), (
        f"{GUARD} is absent; build it (build:cli:dev) — this lane fails "
        "rather than skips"
    )


def test_the_guard_confines_the_researcher_here(tmp_path: Path) -> None:
    _require_guard()
    assert _guard("ls", tmp_path).returncode == 2


@pytest.mark.parametrize("family", ["openalex", "arxiv"])
def test_every_profile_invocation_passes_the_guard(
    family: str, tmp_path: Path
) -> None:
    _require_guard()
    invocations = _invocations(family)
    assert len(invocations) > 2, (
        f"the {family} profile shows too few invocations: {invocations}"
    )
    for invocation in invocations:
        result = _guard(invocation, tmp_path)
        assert result.returncode == 0, (
            f"the {family} profile shows a call the guard blocks: "
            f"{invocation}\n{result.stderr}"
        )
