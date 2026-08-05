#!/usr/bin/env python3
"""Capture `scripts/vcs-status.sh` / `scripts/vcs-log.sh` output as goldens.

Builds nine repo states (one of which — git ahead/behind — yields two golden
pairs, for ten pairs total) in a scratch tempdir, runs the real shell scripts
against each, masks the volatile fields per `masks.toml`, and writes the
result to `hooks/test-fixtures/vcs-status-log/<state>-status.txt` /
`<state>-log.txt`.

Run via: uv run python hooks/test-fixtures/generate_vcs_goldens.py
"""

from __future__ import annotations

import os
import re
import subprocess
import tempfile
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
FIXTURES_DIR = Path(__file__).resolve().parent
OUTPUT_DIR = FIXTURES_DIR / "vcs-status-log"
MASKS_PATH = FIXTURES_DIR / "masks.toml"
STATUS_SCRIPT = REPO_ROOT / "scripts" / "vcs-status.sh"
LOG_SCRIPT = REPO_ROOT / "scripts" / "vcs-log.sh"

GIT_IDENTITY_ENV = {
    "GIT_AUTHOR_NAME": "T",
    "GIT_AUTHOR_EMAIL": "t@e.x",
    "GIT_COMMITTER_NAME": "T",
    "GIT_COMMITTER_EMAIL": "t@e.x",
}


def load_masks() -> list[dict]:
    with MASKS_PATH.open("rb") as handle:
        return tomllib.load(handle)["pattern"]


def mask(text: str, patterns: list[dict]) -> str:
    for pattern in patterns:
        compiled = re.compile(pattern["regex"])
        text = compiled.sub(f"<{pattern['name'].upper()}>", text)
    return text


def run(args: list[str], cwd: Path, env: dict[str, str]) -> None:
    subprocess.run(
        args, cwd=cwd, env=env, check=True, capture_output=True, text=True
    )


def run_hook(script: Path, cwd: Path, env: dict[str, str]) -> str:
    result = subprocess.run(
        [str(script)],
        cwd=cwd,
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    return result.stdout


def git_init(directory: Path, env: dict[str, str]) -> None:
    run(["git", "init", "-q"], directory, env)


def git_commit_allow_empty(
    directory: Path, env: dict[str, str], message: str
) -> None:
    run(["git", "commit", "--allow-empty", "-q", "-m", message], directory, env)


def write_and_commit(
    directory: Path,
    env: dict[str, str],
    filename: str,
    content: str,
    message: str,
) -> None:
    (directory / filename).write_text(content)
    run(["git", "add", filename], directory, env)
    run(["git", "commit", "-q", "-m", message], directory, env)


def jj_git_init(
    directory: Path, env: dict[str, str], colocate: bool = False
) -> None:
    # This mise-pinned jj defaults `git.colocate` to true, so a genuinely
    # pure (non-colocated, no top-level .git) repo requires the explicit
    # override rather than just omitting --colocate.
    args = ["jj", "--config", f"git.colocate={'true' if colocate else 'false'}"]
    args += ["git", "init", "--quiet"]
    run(args, directory, env)
    run(["jj", "config", "set", "--repo", "user.name", "T"], directory, env)
    run(
        ["jj", "config", "set", "--repo", "user.email", "t@e.x"], directory, env
    )


def build_states(work: Path, env: dict[str, str]) -> dict[str, Path]:
    states: dict[str, Path] = {}

    clean_git = work / "clean-git"
    clean_git.mkdir()
    git_init(clean_git, env)
    git_commit_allow_empty(clean_git, env, "init")
    states["clean-git"] = clean_git

    dirty_git = work / "dirty-git"
    dirty_git.mkdir()
    git_init(dirty_git, env)
    write_and_commit(dirty_git, env, "a.txt", "A\n", "init")
    (dirty_git / "a.txt").write_text("A\nchanged\n")
    (dirty_git / "untracked.txt").write_text("untracked\n")
    (dirty_git / "staged.txt").write_text("staged\n")
    run(["git", "add", "staged.txt"], dirty_git, env)
    states["dirty-git"] = dirty_git

    seed = work / "seed"
    seed.mkdir()
    git_init(seed, env)
    write_and_commit(seed, env, "f.txt", "1\n", "commit-1")
    origin = work / "origin.git"
    run(["git", "clone", "-q", "--bare", str(seed), str(origin)], work, env)

    git_ahead = work / "git-ahead"
    run(["git", "clone", "-q", str(origin), str(git_ahead)], work, env)
    write_and_commit(git_ahead, env, "g.txt", "2\n", "commit-2")
    write_and_commit(git_ahead, env, "h.txt", "3\n", "commit-3")
    states["git-ahead"] = git_ahead

    git_behind = work / "git-behind"
    run(["git", "clone", "-q", str(origin), str(git_behind)], work, env)
    write_and_commit(seed, env, "i.txt", "4\n", "commit-4")
    run(["git", "push", "-q", str(origin), "HEAD:refs/heads/main"], seed, env)
    states["git-behind"] = git_behind

    detached = work / "detached-head-git"
    detached.mkdir()
    git_init(detached, env)
    write_and_commit(detached, env, "d1.txt", "1\n", "commit-1")
    first_sha = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=detached,
        env=env,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    write_and_commit(detached, env, "d2.txt", "2\n", "commit-2")
    run(["git", "checkout", "-q", first_sha], detached, env)
    states["detached-head-git"] = detached

    clean_jj = work / "clean-jj"
    clean_jj.mkdir()
    jj_git_init(clean_jj, env)
    states["clean-jj"] = clean_jj

    dirty_jj = work / "dirty-jj"
    dirty_jj.mkdir()
    jj_git_init(dirty_jj, env)
    (dirty_jj / "new-file.txt").write_text("new content\n")
    states["dirty-jj"] = dirty_jj

    colocated = work / "colocated"
    colocated.mkdir()
    git_init(colocated, env)
    git_commit_allow_empty(colocated, env, "init")
    jj_git_init(colocated, env, colocate=True)
    states["colocated"] = colocated

    jj_main = work / "jj-secondary-main"
    jj_main.mkdir()
    jj_git_init(jj_main, env)
    jj_secondary = work / "jj-secondary"
    run(["jj", "workspace", "add", "--quiet", str(jj_secondary)], jj_main, env)
    states["jj-secondary"] = jj_secondary

    no_repo = work / "no-repo"
    no_repo.mkdir()
    states["no-repo"] = no_repo

    return states


def main() -> None:
    patterns = load_masks()
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory(
        prefix="vcs-status-log-goldens-"
    ) as raw_work:
        work = Path(raw_work).resolve()
        env = {
            **os.environ,
            **GIT_IDENTITY_ENV,
            "GIT_CEILING_DIRECTORIES": str(work),
        }
        states = build_states(work, env)

        for name, directory in states.items():
            status_output = run_hook(STATUS_SCRIPT, directory, env)
            log_output = run_hook(LOG_SCRIPT, directory, env)
            (OUTPUT_DIR / f"{name}-status.txt").write_text(
                mask(status_output, patterns)
            )
            (OUTPUT_DIR / f"{name}-log.txt").write_text(
                mask(log_output, patterns)
            )

    print(f"Captured {len(states)} state pairs into {OUTPUT_DIR}")


if __name__ == "__main__":
    main()
