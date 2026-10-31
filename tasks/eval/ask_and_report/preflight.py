"""Prove, with the real CLI and no model, that each scenario's setup holds."""

import json
import os
import subprocess
import tempfile
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any

from tasks.eval.ask_and_report.projects import (
    OVERRIDE_SECTIONS,
    SLUG,
    build_project,
)
from tasks.eval.ask_and_report.scenarios import (
    Gate,
    Scenario,
    ScenarioFile,
    Setup,
)
from tasks.eval.toolchain import Toolchain
from tasks.shared.sources import repo_root


def gate_of(verdict: Mapping[str, object]) -> Gate:
    """Name the scenario gate a ``topic consultable`` verdict reaches."""
    if verdict["consultable"]:
        return "open"
    match verdict["gate"]:
        case "status" | "freshness" as gate:
            return gate
        case "conducted" if verdict["unanswered"]:
            return "conducted"
        case "conducted" if verdict["held_back"]:
            return "held_back"
    raise ValueError(f"no scenario gate reads this verdict: {dict(verdict)}")


class CommandFailedError(Exception):
    pass


def _output(command: Sequence[object], root: Path) -> str:
    ran = subprocess.run(
        [str(part) for part in command],
        cwd=root,
        env=os.environ | {"ACCELERATOR_PLUGIN_ROOT": str(repo_root())},
        capture_output=True,
        text=True,
        check=False,
    )
    if ran.returncode != 0:
        raise CommandFailedError(
            f"{' '.join(map(str, command[1:]))} exited {ran.returncode}: "
            f"{ran.stderr.strip()}"
        )
    return ran.stdout


def _research_topic(
    toolchain: Toolchain, root: Path, verb: str
) -> dict[str, Any]:
    profiles = repo_root() / "skills/research/profiles"
    return json.loads(
        _output(
            [
                toolchain.research,
                "topic",
                verb,
                SLUG,
                "--profiles-dir",
                profiles,
            ],
            root,
        )
    )


def gate_problems(
    scenario: Scenario, root: Path, toolchain: Toolchain
) -> list[str]:
    reached = gate_of(_research_topic(toolchain, root, "consultable"))
    if reached == scenario.gate:
        return []
    return [f"reaches the {reached} gate, not the {scenario.gate} gate"]


def _breadth_problems(
    breadth: int, root: Path, toolchain: Toolchain
) -> list[str]:
    resolved = _output(
        [toolchain.launcher, "config", "get", "research.topic.breadth"], root
    ).strip()
    if resolved == str(breadth):
        return []
    return [f"research.topic.breadth resolves to {resolved}, not {breadth}"]


def _template_problems(
    setup: Setup, root: Path, toolchain: Toolchain
) -> list[str]:
    resolved = _output(
        [
            toolchain.launcher,
            "config",
            "template",
            "topic-research",
            "--kind",
            "report",
        ],
        root,
    )
    winner = "config" if "config" in setup.templates else "user"
    return [
        f"the resolved report template {verdict} ## {section}"
        for tier in setup.templates
        for section in [OVERRIDE_SECTIONS[tier]]
        for verdict in ["lacks" if tier == winner else "still carries"]
        if (f"\n## {section}\n" in resolved) != (tier == winner)
    ]


def _skip_problems(
    skips: Sequence[str], root: Path, toolchain: Toolchain
) -> list[str]:
    plan = _research_topic(toolchain, root, "outstanding")
    skipped = {pair["question"] for pair in plan["skipped"]}
    complete = {item["question"] for item in plan["items"] if item["complete"]}
    return [
        f"{item!r} {problem}"
        for item in skips
        for problem, holds in (
            ("has no skipped pair", item in skipped),
            ("is not complete", item in complete),
        )
        if not holds
    ]


def setup_problems(setup: Setup, root: Path, toolchain: Toolchain) -> list[str]:
    """List what the built project fails to set up that ``setup`` declares."""
    problems: list[str] = []
    if setup.breadth is not None:
        problems += _breadth_problems(setup.breadth, root, toolchain)
    if setup.templates:
        problems += _template_problems(setup, root, toolchain)
    if setup.skips:
        problems += _skip_problems(setup.skips, root, toolchain)
    return problems


def preflight_problems(
    files: Sequence[ScenarioFile], evals: Path, toolchain: Toolchain
) -> list[str]:
    """One line per scenario whose built project misses its gate or setup."""
    problems: list[str] = []
    for scenario in (s for file in files for s in file.scenarios):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            build_project(scenario.setup, root, evals)
            try:
                found = gate_problems(scenario, root, toolchain)
                found += setup_problems(scenario.setup, root, toolchain)
            except CommandFailedError as failure:
                found = [str(failure)]
        problems += [f"{scenario.name}: {problem}" for problem in found]
    return problems
