"""Regression guard for the mise task topology.

Textual-structure assertions (mirroring test_workflows.py's style) that the
Rust enforcement gates are wired into the aggregate `check` task, so a gate
cannot be silently unwired from the read-only CI-mirror. Extended per phase as
each gate lands (cli:check here; deny:check / pup:check in Phases 3-4).
"""

import tomllib
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
MISE_TOML = REPO_ROOT / "mise.toml"

# Gates that MUST be reachable from the aggregate `check` task.
_CHECK_GATES = ["cli:check", "deny:check", "pup:check"]


def test_docs_build_wired_into_default(mise):
    assert "docs:build" in _task_depends(mise, "default"), (
        "docs:build is not in default.depends — the docs site is not built "
        "by the full local CI mirror"
    )


def test_docs_check_stays_out_of_aggregate_check(mise):
    # docs:check writes gitignored artefacts and needs network + a Chromium
    # install, so the docs CI lane owns it — the aggregate check must stay
    # read-only and hermetic.
    assert "docs:check" not in _task_depends(mise, "check"), (
        "docs:check is in check.depends — it breaks the read-only/hermetic "
        "contract of the aggregate check; the docs CI lane owns it"
    )


def _task_depends(mise: dict, task: str) -> list[str]:
    return mise["tasks"][task].get("depends", [])


@pytest.fixture
def mise() -> dict:
    return tomllib.loads(MISE_TOML.read_text())


@pytest.mark.parametrize("gate", _CHECK_GATES)
def test_gate_wired_into_check(mise, gate):
    assert gate in _task_depends(mise, "check"), (
        f"{gate} is not in check.depends — the gate is unwired from the "
        f"read-only CI-mirror"
    )
