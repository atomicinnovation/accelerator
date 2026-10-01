"""The research guard's `PreToolUse` registration, smoke-run end to end.

Each run goes through the real `bin/accelerator` bootstrap into the launcher
built from this tree, reached through the contributor override in a temporary
plugin root, since the bootstrap admits an override only inside its own
`cli/target/`. Integrity refusals are exercised in the launcher's
`dispatch_failure_policy.rs`: a real signature failure cannot be produced
through `bin/accelerator` offline.
"""

import json
import os
import platform
import shutil
import subprocess
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
HOOKS_JSON = REPO_ROOT / "hooks/hooks.json"
BUILT_LAUNCHER = REPO_ROOT / "cli/target/debug/accelerator"
BUILT_RESEARCH = REPO_ROOT / "cli/target/debug/accelerator-research"

GUARD_COMMAND = (
    "${CLAUDE_PLUGIN_ROOT}/bin/accelerator research guard "
    "--fail-safe --non-blocking"
)
UNVERIFIED_WARNING = "accelerator: WARNING execing unverified launcher"

_PLATFORMS = {
    ("Darwin", "arm64"): "darwin-arm64",
    ("Darwin", "x86_64"): "darwin-x64",
    ("Linux", "aarch64"): "linux-arm64",
    ("Linux", "x86_64"): "linux-x64",
}


def _pre_tool_use_groups(matcher: str) -> list[dict]:
    data = json.loads(HOOKS_JSON.read_text(encoding="utf-8"))
    return [
        group
        for group in data["hooks"]["PreToolUse"]
        if group.get("matcher") == matcher
    ]


@pytest.mark.parametrize(
    "matcher", ["Bash", "Write|Edit|MultiEdit|NotebookEdit"]
)
def test_the_guard_is_registered_once_per_matcher(matcher: str) -> None:
    registered = [
        hook
        for group in _pre_tool_use_groups(matcher)
        for hook in group["hooks"]
        if hook.get("command") == GUARD_COMMAND
    ]
    assert len(registered) == 1, (
        f"expected one research guard under {matcher!r}, found "
        f"{len(registered)}"
    )
    assert registered[0].get("type") == "command"


def _plugin_root(tmp_path: Path) -> Path:
    for built in (BUILT_LAUNCHER, BUILT_RESEARCH):
        assert built.is_file(), (
            f"{built} is absent; build it (build:cli:dev) — this lane fails "
            "rather than fetching a release"
        )
    alias = _PLATFORMS[(platform.system(), platform.machine())]
    root = tmp_path / "plugin"
    (root / ".claude-plugin").mkdir(parents=True)
    shutil.copy(
        REPO_ROOT / ".claude-plugin/plugin.json",
        root / ".claude-plugin/plugin.json",
    )
    (root / "bin").mkdir()
    for name in ("accelerator", f"accelerator-verify-{alias}"):
        shutil.copy2(REPO_ROOT / "bin" / name, root / "bin" / name)
    (root / "keys").mkdir()
    shutil.copy(
        REPO_ROOT / "keys/accelerator-release.pub",
        root / "keys/accelerator-release.pub",
    )
    (root / ".accelerator-dev-launcher").touch()
    launcher = root / "cli/target/debug/accelerator"
    launcher.parent.mkdir(parents=True)
    shutil.copy2(BUILT_LAUNCHER, launcher)
    return root


def _run_guard(
    tmp_path: Path, research_bin: Path, payload: dict
) -> subprocess.CompletedProcess[str]:
    root = _plugin_root(tmp_path)
    project = tmp_path / "project"
    (project / ".accelerator").mkdir(parents=True)
    env = {
        key: value
        for key, value in os.environ.items()
        if not key.startswith("ACCELERATOR_")
    }
    env.update(
        {
            "ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER": "1",
            "ACCELERATOR_LAUNCHER_BIN": str(
                root / "cli/target/debug/accelerator"
            ),
            "ACCELERATOR_RESEARCH_BIN": str(research_bin),
            "ACCELERATOR_CACHE_DIR": str(tmp_path / "cache"),
        }
    )
    payload = {"cwd": str(project), **payload}
    result = subprocess.run(
        [
            str(root / "bin/accelerator"),
            "research",
            "guard",
            "--fail-safe",
            "--non-blocking",
        ],
        cwd=project,
        env=env,
        input=json.dumps(payload),
        capture_output=True,
        text=True,
        check=False,
    )
    assert UNVERIFIED_WARNING in result.stderr, (
        "the bootstrap did not reach the built launcher, so this case would "
        f"pass as a bootstrap refusal: {result.stderr}"
    )
    return result


def _researcher_bash(command: str) -> dict:
    return {
        "agent_id": "agent-1",
        "agent_type": "accelerator:researcher",
        "tool_name": "Bash",
        "tool_input": {"command": command},
    }


def test_a_researcher_block_survives_both_launcher_flags(
    tmp_path: Path,
) -> None:
    result = _run_guard(tmp_path, BUILT_RESEARCH, _researcher_bash("ls"))
    assert result.returncode == 2, result.stderr
    assert "E_RESEARCH_GUARD_COMMAND" in result.stderr


def test_a_researcher_fetch_passes(tmp_path: Path) -> None:
    result = _run_guard(
        tmp_path,
        BUILT_RESEARCH,
        _researcher_bash("accelerator research fetch arxiv search 'x'"),
    )
    assert result.returncode == 0, result.stderr
    assert "E_RESEARCH_GUARD" not in result.stderr


def test_an_unresolvable_guard_never_blocks(tmp_path: Path) -> None:
    missing = tmp_path / "absent/accelerator-research"
    result = _run_guard(tmp_path, missing, _researcher_bash("ls"))
    assert result.returncode == 0, result.stderr
    assert f"failed to exec {missing}" in result.stderr, result.stderr
