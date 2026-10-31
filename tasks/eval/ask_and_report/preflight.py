"""Prove, with the real CLI and no model, that each scenario's setup holds."""

import json
import os
import subprocess
import tempfile
from collections.abc import Mapping, Sequence
from pathlib import Path
from typing import Any

from tasks.eval.ask_and_report.layout import SET, SLUG
from tasks.eval.ask_and_report.projects import (
    OVERRIDE_SECTIONS,
    build_project,
)
from tasks.eval.ask_and_report.scenarios import (
    Gate,
    Scenario,
    ScenarioFile,
    Setup,
)
from tasks.eval.ask_and_report.verdicts import (
    Criterion,
    Refusal,
    Report,
    Sources,
    Writes,
)
from tasks.eval.documents.rule import cite_once, key_order
from tasks.eval.documents.sources import Citation, parse_sources
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


RULE = "skills/research/outputters/citing-once/SKILL.md"


def _criteria(scenario: Scenario) -> list[Criterion]:
    return [criterion for turn in scenario.turns for criterion in turn.expect]


def _merged_sources(set_root: Path) -> list[Citation]:
    citations = [
        citation
        for finding in sorted((set_root / "findings").glob("*.md"))
        if not finding.name.startswith(".")
        for citation in parse_sources(finding.read_text())
    ]
    rule = (repo_root() / RULE).read_text()
    return cite_once(citations, key_order(rule))


def _source_problems(sources: Sources, merged: Sequence[Citation]) -> list[str]:
    problems = []
    for expected in sources.entries:
        selected = [c for c in merged if expected.marker in c.render()]
        if len(selected) != 1:
            problems.append(
                f"sources: {expected.marker} selects {len(selected)} of the "
                "rule's merged sources, not one"
            )
            continue
        if str(selected[0].tier) != expected.tier:
            problems.append(
                f"sources: the rule merges {expected.marker} to "
                f"{selected[0].tier}, not {expected.tier}"
            )
        if expected.block is not None and selected[0].block != expected.block:
            problems.append(
                f"sources: the rule gives {expected.marker} the block "
                f"{selected[0].block}, not {expected.block}"
            )
    return problems


def _report_path(scenario: Scenario, root: Path, toolchain: Toolchain) -> str:
    printed = _output(
        [toolchain.research, "topic", "report-path", SLUG, scenario.question],
        root,
    ).strip()
    return (
        Path(printed).resolve().relative_to((root / SET).resolve()).as_posix()
    )


def _new_report_problems(
    writes: Writes, scenario: Scenario, root: Path, toolchain: Toolchain
) -> list[str]:
    new = [
        path
        for path in writes.paths
        if path.startswith("reports/") and not (root / SET / path).exists()
    ]
    if not new:
        return []
    named = _report_path(scenario, root, toolchain)
    return [
        f"writes {path}, but report-path names {named}"
        for path in new
        if path != named
    ]


def _kept_problems(report: Report, root: Path) -> list[str]:
    began = (root / SET / report.path).read_text()
    frontmatter = began.split("\n---\n", 1)[0]
    return [
        f"keeps {key}, which {report.path} does not have at the start"
        for key in report.kept
        if not any(
            line.startswith(f"{key}: ") for line in frontmatter.splitlines()
        )
    ]


def expectation_problems(
    scenario: Scenario, root: Path, toolchain: Toolchain
) -> list[str]:
    """List each expectation the CLI or the citing-once rule contradicts.

    A Sources expectation must be the rule's merge of every finding's
    citations, a freshness refusal must name exactly the mismatches
    ``consultable`` prints, a new report must take the path ``report-path``
    prints, and a kept field must be in the report before any turn.
    """
    problems: list[str] = []
    criteria = _criteria(scenario)
    if any(isinstance(c, Sources) for c in criteria):
        merged = _merged_sources(root / SET)
        for sources in (c for c in criteria if isinstance(c, Sources)):
            problems += _source_problems(sources, merged)
    if scenario.gate == "freshness":
        mismatches = _research_topic(toolchain, root, "consultable")[
            "mismatches"
        ]
        problems += [
            f"refusal names {list(r.names)}, not the mismatches {mismatches}"
            for r in criteria
            if isinstance(r, Refusal) and list(r.names) != mismatches
        ]
    for criterion in criteria:
        if isinstance(criterion, Writes):
            problems += _new_report_problems(
                criterion, scenario, root, toolchain
            )
        if isinstance(criterion, Report) and criterion.kept:
            problems += _kept_problems(criterion, root)
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
                found += expectation_problems(scenario, root, toolchain)
            except CommandFailedError as failure:
                found = [str(failure)]
        problems += [f"{scenario.name}: {problem}" for problem in found]
    return problems
