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

# cli/-scoped Python guards ride in cli:check, which is what CI runs, *and* in
# lint:check, which is the only path the bare `default` task reaches: `default`
# depends on lint:check but not on check, so a cli:check-only guard is green
# locally however badly the invariant is broken. Nothing else pins their
# placement, so either roll-up would otherwise happily lose one.
_CLI_CHECK_GATES = [
    "lint:vendor-shims:check",
    "lint:store-duplication:check",
    "lint:claude-coupling:check",
]

_LAUNCHER = "build:cli:dev"
_INTEGRATION_PREFIX = "test:integration:"

# Integration tasks that reach the compiled launcher and so MUST carry the
# build:cli:dev edge. All six drive shell suites through accelerator_env().
_LAUNCHER_DEPENDENTS = [
    "test:integration:config",
    "test:integration:decisions",
    "test:integration:migrate",
    "test:integration:work",
    "test:integration:integrations",
    "test:integration:visualiser",
]

# Integration tasks that deliberately need no prebuilt launcher, each with a
# reason. Every test:integration:* task must appear in exactly one of the two.
_NO_LAUNCHER_NEEDED = {
    "test:integration:entrypoint": "builds it in-fixture, mirroring shim_bin",
    "test:integration:skill-invocation": "builds it in-fixture, same harness",
    "test:integration:tasks": "pytest over the invoke task modules",
    "test:integration:dev": "drives circusd with Python fake processes",
    "test:integration:deny": "cargo-deny over offline fixtures",
    "test:integration:pup": "cargo-pup, built through build:frontend:stub",
    "test:integration:hooks": "shell suites run with no accelerator_env",
    "test:integration:github": "shell suites run with no accelerator_env",
}


def _task_depends(mise: dict, task: str) -> list[str]:
    return mise["tasks"][task].get("depends", [])


def _integration_tasks(mise: dict) -> set[str]:
    return {t for t in mise["tasks"] if t.startswith(_INTEGRATION_PREFIX)}


@pytest.fixture
def mise() -> dict:
    return tomllib.loads(MISE_TOML.read_text())


@pytest.mark.parametrize("gate", _CHECK_GATES)
def test_gate_wired_into_check(mise, gate):
    assert gate in _task_depends(mise, "check"), (
        f"{gate} is not in check.depends — the gate is unwired from the "
        f"read-only CI-mirror"
    )


@pytest.mark.parametrize("gate", _CLI_CHECK_GATES)
def test_gate_wired_into_cli_check(mise, gate):
    assert gate in _task_depends(mise, "cli:check"), (
        f"{gate} is not in cli:check.depends — the guard is unwired from the "
        f"roll-up CI runs"
    )


@pytest.mark.parametrize("gate", _CLI_CHECK_GATES)
def test_gate_wired_into_lint_check(mise, gate):
    assert gate in _task_depends(mise, "lint:check"), (
        f"{gate} is not in lint:check.depends — the guard is unreachable from "
        f"the bare `default` task, which depends on lint:check and not on check"
    )


@pytest.mark.parametrize("task", _LAUNCHER_DEPENDENTS)
def test_launcher_dependent_carries_the_build_edge(mise, task):
    assert _LAUNCHER in _task_depends(mise, task), (
        f"{task} reaches the compiled launcher but does not depend on "
        f"{_LAUNCHER} — it would run against a stale or absent binary"
    )


@pytest.mark.parametrize("task", sorted(_NO_LAUNCHER_NEEDED))
def test_task_needing_no_launcher_omits_the_build_edge(mise, task):
    assert _LAUNCHER not in _task_depends(mise, task), (
        f"{task} is recorded as needing no prebuilt launcher "
        f"({_NO_LAUNCHER_NEEDED[task]}) but depends on {_LAUNCHER}"
    )


def test_every_integration_task_declares_its_launcher_need(mise):
    classified = set(_LAUNCHER_DEPENDENTS) | set(_NO_LAUNCHER_NEEDED)
    assert _integration_tasks(mise) == classified, (
        "every test:integration:* task must appear in exactly one of "
        "_LAUNCHER_DEPENDENTS or _NO_LAUNCHER_NEEDED"
    )


def test_the_two_launcher_sets_are_disjoint():
    assert not set(_LAUNCHER_DEPENDENTS) & set(_NO_LAUNCHER_NEEDED)


# test:integration.depends is curated rather than derived, so membership is an
# explicit decision — and an omission means a whole suite ships green and never
# runs. Each exclusion carries its reason.
_NOT_IN_INTEGRATION_ROLLUP = {
    "test:integration:pup": "needs the isolated nightly toolchain lane",
}


def test_every_integration_task_is_in_the_rollup_or_excluded_with_a_reason(
    mise,
):
    rollup = set(_task_depends(mise, "test:integration"))
    assert rollup | set(_NOT_IN_INTEGRATION_ROLLUP) == _integration_tasks(mise)
    assert not rollup & set(_NOT_IN_INTEGRATION_ROLLUP)
