"""Run one headless Claude Code session against this repository's plugin."""

import json
import subprocess
from pathlib import Path

from tasks.eval.citing_once.projects import Project

INSTALLED_PLUGIN = "accelerator@atomic-innovation-prerelease"
ALLOWED_TOOLS = (
    "Read",
    "Write",
    "Edit",
    "Glob",
    "Grep",
    "Agent",
    "Task",
    "Skill",
    "Bash(accelerator:*)",
)
FINDING_OUTPUTTER = "skills/research/outputters/finding-outputter/SKILL.md"
RULE = "skills/research/outputters/citing-once/SKILL.md"
FINDING_TEMPLATE = "templates/topic-research-finding.md"
SESSION_TIMEOUT = 1800


def session_command(prompt: str, plugin_root: Path) -> list[str]:
    return [
        "claude",
        "-p",
        prompt,
        "--plugin-dir",
        str(plugin_root),
        "--settings",
        json.dumps({"enabledPlugins": {INSTALLED_PLUGIN: False}}),
        "--output-format",
        "stream-json",
        "--verbose",
        "--no-session-persistence",
        "--permission-mode",
        "acceptEdits",
        "--allowedTools",
        ",".join(ALLOWED_TOOLS),
    ]


def composer_prompt(project: Project, plugin_root: Path) -> str:
    notes = "".join(f"  - {path}\n" for path in project.inputs)
    template = (plugin_root / FINDING_TEMPLATE).read_text()
    return (
        "Spawn exactly one subagent with subagent_type "
        '"accelerator:composer", as research-topic\'s conduct verb does for '
        "a compose pair, and wait for it. Read, write and run nothing "
        "yourself. Give it this prompt, verbatim:\n\n"
        "Compose this pair's finding.\n\n"
        f"- Finding outputter: {plugin_root / FINDING_OUTPUTTER}\n"
        "- Citing-once rule, which every Sources entry of the finding "
        f"follows: {plugin_root / RULE}\n"
        f"- Finding template:\n\n```markdown\n{template}```\n\n"
        f"- Question: {_question(project)}\n"
        "- Source profile: web\n"
        "- Depth: 2\n"
        "- Round: 1\n"
        "- Timestamp: 2026-10-29T00:00:00+00:00\n"
        "- Author: Eval Author\n"
        f"- Notes to read:\n{notes}"
        f"- Output path: {project.output}\n"
    )


def _question(project: Project) -> str:
    for line in project.inputs[0].read_text().splitlines():
        if line.startswith("question: "):
            return json.loads(line.removeprefix("question: "))
    raise ValueError(f"{project.inputs[0]} has no question")


def synthesise_prompt(project: Project) -> str:
    return f"/accelerator:research-topic synthesise {project.slug}"


def prepare_repository(project: Project) -> None:
    for command in (
        ["git", "init", "-q"],
        ["git", "config", "user.name", "Eval Author"],
        ["git", "config", "user.email", "eval@example.invalid"],
    ):
        subprocess.run(command, cwd=project.root, check=True)


def run_session(command: list[str], project: Project, log: Path) -> int:
    with log.open("w") as out, log.with_suffix(".err").open("w") as err:
        return subprocess.run(
            command,
            cwd=project.root,
            stdout=out,
            stderr=err,
            timeout=SESSION_TIMEOUT,
            check=False,
        ).returncode
