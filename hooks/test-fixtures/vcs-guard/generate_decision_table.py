#!/usr/bin/env python3
"""Capture `hooks/vcs-guard.sh` decisions as a golden decision table.

Runs 34 command cases (13 blocked git subcommands, 7 allowed, `gh`, `rtk`,
and 12 compound-separator cases) against each of 4 repo modes (pure-jj,
colocated, git, non-repo), normalises the shell's legacy
`decision`/`hookSpecificOutput.systemMessage` shape into
`{repo_mode, command, decision, reason_pattern}`, and writes the 136
captured rows plus 2 hand-authored departure rows to decision-table.json.

Run via: uv run python hooks/test-fixtures/vcs-guard/generate_decision_table.py
"""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
OUTPUT_PATH = Path(__file__).resolve().parent / "decision-table.json"
GUARD_SCRIPT = REPO_ROOT / "hooks" / "vcs-guard.sh"

BLOCKED_SUBCOMMANDS = [
    "status",
    "diff",
    "add",
    "commit",
    "log",
    "branch",
    "checkout",
    "switch",
    "merge",
    "rebase",
    "reset",
    "stash",
    "show",
]

BLOCKED_COMMANDS = {
    "status": "git status",
    "diff": "git diff",
    "add": "git add file.txt",
    "commit": 'git commit -m "message"',
    "log": "git log",
    "branch": "git branch",
    "checkout": "git checkout main",
    "switch": "git switch main",
    "merge": "git merge main",
    "rebase": "git rebase main",
    "reset": "git reset",
    "stash": "git stash",
    "show": "git show",
}

ALLOWED_COMMANDS = [
    "git push",
    "git pull",
    "git fetch",
    "git remote -v",
    "git clone https://example.com/repo.git",
    "git config user.name",
    "git tag",
]

SEPARATORS = ["&&", "||", ";", "|"]


def compound_commands() -> list[tuple[str, str]]:
    cases = []
    for separator in SEPARATORS:
        cases.append(
            (
                f"compound-{separator}-match-first",
                f"git status {separator} echo done",
            )
        )
        cases.append(
            (
                f"compound-{separator}-match-later",
                f"echo start {separator} git status",
            )
        )
        cases.append(
            (
                f"compound-{separator}-no-match",
                f"echo start {separator} echo done",
            )
        )
    return cases


def all_commands() -> list[tuple[str, str]]:
    commands = [
        (f"blocked-{name}", cmd) for name, cmd in BLOCKED_COMMANDS.items()
    ]
    commands += [
        (f"allowed-{i}", cmd) for i, cmd in enumerate(ALLOWED_COMMANDS)
    ]
    commands.append(("gh", "gh pr view"))
    commands.append(("rtk", "rtk git status"))
    commands += compound_commands()
    return commands


def git_env() -> dict[str, str]:
    return {
        **os.environ,
        "GIT_AUTHOR_NAME": "T",
        "GIT_AUTHOR_EMAIL": "t@e.x",
        "GIT_COMMITTER_NAME": "T",
        "GIT_COMMITTER_EMAIL": "t@e.x",
    }


def run(args: list[str], cwd: Path, env: dict[str, str]) -> None:
    subprocess.run(
        args, cwd=cwd, env=env, check=True, capture_output=True, text=True
    )


def build_repo_modes(work: Path, env: dict[str, str]) -> dict[str, Path]:
    modes: dict[str, Path] = {}

    pure_jj = work / "pure-jj"
    pure_jj.mkdir()
    run(
        ["jj", "--config", "git.colocate=false", "git", "init", "--quiet"],
        pure_jj,
        env,
    )
    modes["pure-jj"] = pure_jj

    colocated = work / "colocated"
    colocated.mkdir()
    run(["git", "init", "-q"], colocated, env)
    run(["git", "commit", "--allow-empty", "-q", "-m", "init"], colocated, env)
    run(
        ["jj", "--config", "git.colocate=true", "git", "init", "--quiet"],
        colocated,
        env,
    )
    modes["colocated"] = colocated

    git_only = work / "git"
    git_only.mkdir()
    run(["git", "init", "-q"], git_only, env)
    run(["git", "commit", "--allow-empty", "-q", "-m", "init"], git_only, env)
    modes["git"] = git_only

    non_repo = work / "non-repo"
    non_repo.mkdir()
    modes["non-repo"] = non_repo

    return modes


def normalise(raw_stdout: str) -> tuple[str, str]:
    """Map the shell's legacy decision shape to (decision, reason_pattern)."""
    stripped = raw_stdout.strip()
    if not stripped:
        return "allow", ""
    payload = json.loads(stripped)
    if payload.get("decision") == "block":
        return "block", payload["reason"]
    system_message = payload.get("hookSpecificOutput", {}).get(
        "systemMessage", ""
    )
    if system_message:
        return "warn", system_message
    return "allow", ""


def run_guard(command: str, cwd: Path, env: dict[str, str]) -> tuple[str, str]:
    stdin_payload = json.dumps({"tool_input": {"command": command}})
    result = subprocess.run(
        ["bash", str(GUARD_SCRIPT)],
        cwd=cwd,
        env=env,
        input=stdin_payload,
        capture_output=True,
        text=True,
        check=False,
    )
    return normalise(result.stdout)


def captured_rows(modes: dict[str, Path], env: dict[str, str]) -> list[dict]:
    rows = []
    for _, command in all_commands():
        for repo_mode, directory in modes.items():
            decision, reason_pattern = run_guard(command, directory, env)
            rows.append(
                {
                    "repo_mode": repo_mode,
                    "command": command,
                    "decision": decision,
                    "reason_pattern": reason_pattern,
                }
            )
    return rows


def departure_rows() -> list[dict]:
    return [
        {
            "repo_mode": "colocated-git-as-file",
            "command": "git status",
            "decision": "warn",
            "reason_pattern": (
                "This is a jj-colocated repository. Prefer jj over git status. "
                "Suggested equivalent: jj status"
            ),
            "departure": True,
            "note": (
                'Today\'s shell mode check ([ -d "$REPO_ROOT/.git" ]) '
                "misreads a colocated checkout whose .git is a "
                "worktree/submodule FILE as pure-jj, so it wrongly blocks "
                "this case. The library-backed classifier's "
                "gix::discover-based git-presence check is file-aware, so "
                "the corrected behaviour is warn (colocated), not block."
            ),
        },
        {
            "repo_mode": "pure-jj",
            "command": 'git commit -m "build && test"',
            "decision": "block",
            "reason_pattern": (
                "This is a pure jujutsu repository. Use jj instead of git "
                'commit. Equivalent: jj commit -m "message"'
            ),
            "departure": True,
            "note": (
                "Today's shell compound-splitter is quote-blind (a plain "
                "sed-then-split), so it wrongly splits inside the quoted "
                'string and evaluates test" as a spurious second segment. '
                "The Rust port's splitter is quote-aware: the embedded && "
                "stays inside the quoted argument, so the whole string is "
                "evaluated as a single `git commit` invocation and blocked "
                "accordingly."
            ),
        },
    ]


def main() -> None:
    with tempfile.TemporaryDirectory(
        prefix="vcs-guard-decision-table-"
    ) as raw_work:
        work = Path(raw_work).resolve()
        env = {**git_env(), "GIT_CEILING_DIRECTORIES": str(work)}
        modes = build_repo_modes(work, env)
        rows = captured_rows(modes, env) + departure_rows()

    OUTPUT_PATH.write_text(json.dumps(rows, indent=2) + "\n")
    print(f"Captured {len(rows)} decision rows into {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
