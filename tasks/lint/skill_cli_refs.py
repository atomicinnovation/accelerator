"""Bind skill VCS references to real subcommands; render-lock the idioms (0286).

Two invariants, one guard:

* subcommand resolution — a two-way binding: every ``accelerator vcs <sub>``
  token a ``SKILL.md`` names must be a real ``accelerator-vcs`` subcommand.
  ``VCS_SUBCOMMANDS`` is pinned against the clap ``Command`` enum by a
  cross-language test, as ``dispatch_coherence`` pins its own built-in set.
* idiom render-lock — a one-way existence guard: the diff-range and
  user-identity idiom anchors must still appear in each reference const in
  ``detect.rs``. Skills name these idioms in free prose (no machine-checkable
  token), so this guards the reference content across the Python/Rust boundary,
  not a dynamic skill->idiom binding.
"""

import re
from pathlib import Path

from invoke import Context, Exit, task

from tasks.shared.skill_parsing import LAUNCHER
from tasks.shared.sources import repo_root

# Pinned against the clap `Command` enum in cli/vcs-cli/src/cli.rs by a
# cross-language test; not compile-enforced from this side.
VCS_SUBCOMMANDS = frozenset({"detect", "status", "log", "root", "guard"})

_VCS_REFERENCE = re.compile(rf"{re.escape(LAUNCHER)} vcs ([a-z][a-z0-9-]*)")

_DETECT_RS = "cli/vcs-cli/src/detect.rs"

# Minimal, most-stable anchors per reference block: the command plus one short
# descriptive phrase per idiom. `git config user.name` is required inside the jj
# block (the identity fallback clause) as well as the git block, so dropping the
# jj fallback fails even while the git block still carries it. Kept minimal to
# avoid duplicating detect.rs's own `.contains(...)` render tests.
_JJ_ANCHORS = (
    "fork_point(trunk() | @)",
    "trunk divergence point",
    "git config user.name",
    "user identity",
)
_GIT_ANCHORS = (
    "<trunk>...HEAD",
    "trunk divergence point",
    "git config user.name",
    "user identity",
)


def skill_ref_violations(root: Path) -> list[str]:
    """Every ``accelerator vcs <sub>`` reference naming an unknown subcommand.

    One ``<rel>:<line>: <token>`` string per hit.
    """
    found: list[str] = []
    for path in sorted((root / "skills").rglob("SKILL.md")):
        rel = path.relative_to(root).as_posix()
        for number, line in enumerate(path.read_text().splitlines(), start=1):
            found.extend(
                f"{rel}:{number}: `{LAUNCHER} vcs {subcommand}` names "
                "no accelerator-vcs subcommand"
                for subcommand in _VCS_REFERENCE.findall(line)
                if subcommand not in VCS_SUBCOMMANDS
            )
    return found


def _const_body(text: str, name: str) -> str:
    match = re.search(
        rf"const {name}: &str = concat!\((.*?)\n\);", text, re.DOTALL
    )
    return match.group(1) if match else ""


def render_lock_violations(root: Path) -> list[str]:
    """Every missing idiom anchor in the ``detect.rs`` reference consts.

    One string per anchor absent from its block, or one when the source is gone.
    """
    path = root / _DETECT_RS
    if not path.is_file():
        return [f"{_DETECT_RS}: reference source not found"]
    text = path.read_text()
    found: list[str] = []
    for const, anchors in (
        ("JJ_COMMAND_REFERENCE", _JJ_ANCHORS),
        ("GIT_REFERENCE", _GIT_ANCHORS),
    ):
        body = _const_body(text, const)
        found.extend(
            f"{_DETECT_RS}: {const} missing idiom anchor {anchor!r}"
            for anchor in anchors
            if anchor not in body
        )
    return found


def violations(root: Path) -> list[str]:
    """Every unknown subcommand and missing idiom anchor, combined."""
    return skill_ref_violations(root) + render_lock_violations(root)


@task
def check(context: Context) -> None:
    """Fail on an unknown vcs subcommand reference or a missing idiom anchor."""
    offenders = violations(repo_root())
    if offenders:
        raise Exit(
            "check-skill-cli-refs found violation(s):\n  "
            + "\n  ".join(offenders),
            code=1,
        )
