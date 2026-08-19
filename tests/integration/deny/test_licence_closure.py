"""The committed licence closure is acceptable and current.

Two assertions, neither a byte diff of `cli/licence-audit/new-trees.txt`:

- cargo-deny accepts the closure's licences (`cargo deny check licenses`
  exits zero against the real `cli/deny.toml`). This is the set-membership
  oracle: SPDX AND/OR/WITH satisfiability is decided by the tool that owns
  it, so a multi-licence crate whose expression carries a token outside the
  allow-list (e.g. adler2's `0BSD OR MIT OR Apache-2.0`) passes on an allowed
  alternative, and a crate whose expression has no allowed operand reds.
- the set of licence identifiers in the committed evidence equals the set in
  the live closure. Comparing the SET, not the bytes, keeps an unrelated
  version or count bump from redding a check that carries no licence
  information — the failure this file exists to avoid — while a genuine
  change to the licence set reds and prompts a re-review plus a regenerate.

The `list` layouts flatten away AND/OR structure, so the committed listing
alone cannot judge the OR tokens; that is why acceptability is delegated to
cargo-deny rather than reconstructed here.
"""

import os
import re
import shutil
import subprocess
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
_REPO_ROOT = _HERE.parents[2]
_CLI_DIR = _REPO_ROOT / "cli"
_EVIDENCE = _CLI_DIR / "licence-audit/new-trees.txt"

_CARGO = shutil.which("cargo")
_CARGO_DENY = shutil.which("cargo-deny")

_CRATE_LINE = re.compile(r"^\S+@\S+ \(\d+\): (?P<licences>.+)$")


def _in_ci() -> bool:
    return bool(os.environ.get("CI") or os.environ.get("GITHUB_ACTIONS"))


def _require_tools() -> None:
    missing = [
        name
        for name, path in (("cargo", _CARGO), ("cargo-deny", _CARGO_DENY))
        if path is None
    ]
    if not missing:
        return
    message = f"tools not on PATH: {', '.join(missing)}"
    if _in_ci():
        pytest.fail(f"{message} — provisioning regression in CI")
    pytest.skip(message)


def _licence_set(listing: str) -> set[str]:
    licences: set[str] = set()
    for line in listing.splitlines():
        match = _CRATE_LINE.match(line)
        if match is None:
            continue
        for token in match["licences"].split(","):
            licences.add(token.strip())
    return licences


def _live_listing() -> str:
    result = subprocess.run(
        ["cargo", "deny", "list", "--layout", "crate", "--format", "human"],
        cwd=_CLI_DIR,
        capture_output=True,
        text=True,
        check=True,
        env={**os.environ, "CARGO_NET_OFFLINE": "true"},
    )
    return result.stdout


def test_the_closure_licences_are_accepted_by_cargo_deny() -> None:
    _require_tools()
    result = subprocess.run(
        ["cargo", "deny", "check", "licenses"],
        cwd=_CLI_DIR,
        capture_output=True,
        text=True,
        check=False,
        env={**os.environ, "CARGO_NET_OFFLINE": "true"},
    )
    assert result.returncode == 0, result.stdout + result.stderr


def test_the_committed_evidence_licence_set_matches_the_live_closure() -> None:
    _require_tools()
    committed = _licence_set(_EVIDENCE.read_text())
    assert committed, "no licence lines parsed from the committed evidence"

    live = _licence_set(_live_listing())

    missing = committed - live
    extra = live - committed
    assert committed == live, (
        "cli/licence-audit/new-trees.txt is stale against the live closure "
        f"licence set — no longer present: {sorted(missing)}; newly present: "
        f"{sorted(extra)}. Regenerate with `cd cli && cargo deny list "
        "--layout crate --format human` and re-review the new licences."
    )
