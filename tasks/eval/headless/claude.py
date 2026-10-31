"""Run one headless Claude Code session against this repository's plugin."""

import json
import subprocess
from collections.abc import Sequence
from pathlib import Path

INSTALLED_PLUGIN = "accelerator@atomic-innovation-prerelease"
SESSION_TIMEOUT = 1800


def common_flags(plugin_root: Path, allowed_tools: Sequence[str]) -> list[str]:
    """Return the flags every eval session passes, whatever its grant."""
    return [
        "--plugin-dir",
        str(plugin_root),
        "--settings",
        json.dumps({"enabledPlugins": {INSTALLED_PLUGIN: False}}),
        "--output-format",
        "stream-json",
        "--verbose",
        "--permission-mode",
        "acceptEdits",
        "--allowedTools",
        ",".join(allowed_tools),
    ]


def prepare_repository(root: Path) -> None:
    for command in (
        ["git", "init", "-q"],
        ["git", "config", "user.name", "Eval Author"],
        ["git", "config", "user.email", "eval@example.invalid"],
    ):
        subprocess.run(command, cwd=root, check=True)


def run_session(
    command: list[str], root: Path, log: Path, env: dict[str, str]
) -> int:
    with log.open("w") as out, log.with_suffix(".err").open("w") as err:
        return subprocess.run(
            command,
            cwd=root,
            env=env,
            stdout=out,
            stderr=err,
            timeout=SESSION_TIMEOUT,
            check=False,
        ).returncode
