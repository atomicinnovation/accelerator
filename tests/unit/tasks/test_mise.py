"""Regression guard for the mise task topology.

Textual-structure assertions (mirroring test_workflows.py's style) that the
Rust enforcement gates are wired into the aggregate `check` task, so a gate
cannot be silently unwired from the read-only CI-mirror.
"""

import tomllib
from pathlib import Path

import pytest

from tasks.shared.dev_builds import DEV_BUILDS

REPO_ROOT = Path(__file__).resolve().parents[3]
MISE_TOML = REPO_ROOT / "mise.toml"

# Gates that MUST be reachable from the aggregate `check` task.
_CHECK_GATES = [
    "cli:check",
    "deny:check",
    "pup:check",
    "public-api:check",
    "notices:check",
]

# cli/-scoped Python guards ride in cli:check, which is what CI runs, *and* in
# lint:check, which is the only path the bare `default` task reaches: `default`
# depends on lint:check but not on check, so a cli:check-only guard is green
# locally however badly the invariant is broken. Nothing else pins their
# placement, so either roll-up would otherwise happily lose one.
_CLI_CHECK_GATES = [
    "lint:vendor-shims:check",
    "lint:store-duplication:check",
    "lint:claude-coupling:check",
    "lint:vcs-settings:check",
    "lint:config-test-support:check",
    "lint:crate-dependencies:check",
]

# The dispatch guard is a skills-tree guard, so it cannot join _CLI_CHECK_GATES
# — that would additionally force it into cli:check. It rides build-system:check
# because that is the roll-up CI runs, and lint:check for the same
# bare-`default` reason as the cli/-scoped guards above.
_BUILD_SYSTEM_CHECK_GATES = [
    "lint:dispatch-coherence:check",
    "lint:integration-skills:check",
    "lint:bare-invocation:check",
    "lint:git-tokens:check",
    "lint:skill-cli-refs:check",
]

_LAUNCHER = "build:cli:dev"
_INTEGRATION_PREFIX = "test:integration:"

# The one lane whose dependencies are wholly external: a live tracker, its
# credentials, and network egress.
_CONTRACT_LANE = "test:integration:tracker-contract"

# Integration tasks that run compiled cli/ binaries, each with exactly the debug
# builds it needs. A lane whose builds are absent runs against a stale or
# missing binary; a lane with extra builds contends on cargo's target lock for
# nothing.
_CLI_DEV_BUILDS = {
    "test:integration:conformance": {_LAUNCHER, "build:cli:corpus:dev"},
    "test:integration:hooks": {
        _LAUNCHER,
        "build:cli:vcs:dev",
        "build:cli:research:dev",
    },
    "test:integration:research": {"build:cli:research:dev"},
    "test:integration:research-exhaustive": {"build:cli:research:dev"},
    "test:integration:visualiser": {_LAUNCHER},
    "test:integration:characterisation": {"build:cli:characterisation:dev"},
}

# Integration tasks that deliberately need no prebuilt launcher, each with a
# reason. Every test:integration:* task must appear in exactly one of the two.
_NO_LAUNCHER_NEEDED = {
    "test:integration:entrypoint": "builds it in-fixture, mirroring shim_bin",
    "test:integration:skill-invocation": "builds it in-fixture, same harness",
    "test:integration:tasks": "pytest over the invoke task modules",
    "test:integration:dev": "drives circusd with Python fake processes",
    "test:integration:deny": "cargo-deny over offline fixtures",
    "test:integration:pup": "cargo-pup, built through build:frontend:stub",
    "test:integration:tracker-contract": "cargo nextest against a live "
    "tracker; reaches no accelerator binary",
    "test:integration:design-automation": "node --test against a Playwright "
    "runtime; reaches no accelerator binary",
    "test:integration:measure": "fetches the released launcher; builds nothing",
    # A build-linkage guard rehomed into the roll-up, not an integration test:
    # it depends only on build:cli:fixture-size and reaches no launcher.
    "test:integration:fixture-size": "depends-only shim for the gix/jj-lib "
    "link-ratio guard; reaches no launcher",
}


def test_docs_tasks_stay_out_of_default_and_aggregate_check(mise):
    # The docs tasks write gitignored artefacts and need network + a
    # Chromium install, so the docs CI lane owns them — default and the
    # aggregate check must stay hermetic.
    assert "docs:build" not in _task_depends(mise, "default"), (
        "docs:build is in default.depends — it gives the local CI mirror a "
        "network + Chromium dependency; the docs CI lane owns the docs build"
    )
    assert "docs:check" not in _task_depends(mise, "check"), (
        "docs:check is in check.depends — it breaks the read-only/hermetic "
        "contract of the aggregate check; the docs CI lane owns it"
    )


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


@pytest.mark.parametrize("gate", _BUILD_SYSTEM_CHECK_GATES)
def test_gate_wired_into_build_system_check(mise, gate):
    assert gate in _task_depends(mise, "build-system:check"), (
        f"{gate} is not in build-system:check.depends — the guard is unwired "
        f"from the roll-up CI runs"
    )


@pytest.mark.parametrize("gate", _CLI_CHECK_GATES + _BUILD_SYSTEM_CHECK_GATES)
def test_gate_wired_into_lint_check(mise, gate):
    assert gate in _task_depends(mise, "lint:check"), (
        f"{gate} is not in lint:check.depends — the guard is unreachable from "
        f"the bare `default` task, which depends on lint:check and not on check"
    )


def _cli_dev_builds(mise: dict, task: str) -> set[str]:
    return {
        dependency
        for dependency in _task_depends(mise, task)
        if dependency.startswith("build:cli:") and dependency.endswith("dev")
    }


@pytest.mark.parametrize("task", sorted(_CLI_DEV_BUILDS))
def test_a_lane_depends_on_exactly_the_cli_builds_it_runs(mise, task):
    assert _cli_dev_builds(mise, task) == _CLI_DEV_BUILDS[task]


@pytest.mark.parametrize("task", sorted(_NO_LAUNCHER_NEEDED))
def test_task_needing_no_launcher_omits_the_build_edge(mise, task):
    assert not _cli_dev_builds(mise, task), (
        f"{task} is recorded as needing no prebuilt launcher "
        f"({_NO_LAUNCHER_NEEDED[task]}) but depends on a cli dev build"
    )


def test_every_integration_task_declares_its_launcher_need(mise):
    classified = set(_CLI_DEV_BUILDS) | set(_NO_LAUNCHER_NEEDED)
    assert _integration_tasks(mise) == classified, (
        "every test:integration:* task must appear in exactly one of "
        "_CLI_DEV_BUILDS or _NO_LAUNCHER_NEEDED"
    )


def test_the_two_launcher_sets_are_disjoint():
    assert not set(_CLI_DEV_BUILDS) & set(_NO_LAUNCHER_NEEDED)


def _dev_build_leaf(group: str) -> str:
    return _LAUNCHER if group == "launcher" else f"build:cli:{group}:dev"


@pytest.mark.parametrize("group", sorted(DEV_BUILDS))
def test_every_dev_build_has_a_leaf_running_it(mise, group):
    leaf = mise["tasks"][_dev_build_leaf(group)]
    assert leaf["run"] == f"invoke build.cli-dev --group {group}"


def test_every_cli_dev_leaf_names_a_dev_build(mise):
    leaves = {
        name
        for name in mise["tasks"]
        if name.startswith("build:cli:") and name.endswith(":dev")
    } | {_LAUNCHER}
    assert leaves == {_dev_build_leaf(group) for group in DEV_BUILDS}


# test:integration.depends is curated rather than derived, so membership is an
# explicit decision — and an omission means a whole suite ships green and never
# runs. Each exclusion carries its reason.
_NOT_IN_INTEGRATION_ROLLUP = {
    "test:integration:pup": "needs the isolated nightly toolchain lane",
    # Needs a bootstrapped Playwright runtime, which no CI lane provisions, so
    # in the roll-up it would fail every build. It fails rather than skips
    # without one, which keeps it runnable on demand and honest when the
    # runtime is absent.
    "test:integration:design-automation": "needs a Playwright runtime no CI "
    "lane provisions; fails rather than skips without one",
    # Membership would put it under `test` -> `default` and on both CI legs,
    # including macos-latest, which resolves no sha256sum. It dispatches through
    # the real bootstrap, so it needs network egress and a published signed
    # release for the tree's own version. Owned by its own non-blocking job.
    "test:integration:measure": "live release fetch; quiet-host harness, "
    "owned by its own lane",
    # Every provider client's contract binary talks to a real tenant, so the
    # lane depends on credentials no CI job holds and no contributor
    # necessarily has. Membership would put it under `test` -> `default` and
    # fail every bare `mise run` on an unconfigured machine. The port's
    # invariants are enforced offline instead, by each client's
    # contract_offline binary in the default nextest profile; this lane is the
    # live-tenant assurance beside it, and what proves it ran is the committed
    # evidence file rather than its exit status.
    "test:integration:tracker-contract": "live tracker credentials no CI job "
    "holds; the offline conformance run is the enforcing route",
    "test:integration:research-exhaustive": "splices at every position, so "
    "it runs for minutes; the fixed-seed sample is the enforcing route",
}


# The measurement harness needs a quiet host, network egress and several
# minutes, so no task reaching it may be reachable from the CI mirror. Keyed on
# the `run` string rather than the `measure:*` name prefix: the prefix form does
# not match `test:integration:measure`, so the one live-dispatch path the guard
# exists to contain would be invisible to it.
_MEASURE_MODULE = "tasks/measure.py"
_MEASURE_INVOCATION = "invoke measure."


def _tasks_reaching_measure(mise: dict) -> set[str]:
    return {
        name
        for name, body in mise["tasks"].items()
        if _MEASURE_INVOCATION in str(body.get("run", ""))
    }


def _transitive_depends(mise: dict, task: str) -> set[str]:
    """Every task reachable from `task` through `depends`, at any depth.

    A single-level assertion would stay green if a future edge added a
    measure-reaching task under any task already inside the closure — and
    `default` reaches work through `lint:check` rather than through `check`, so
    one-level assertions miss real indirection.
    """
    seen: set[str] = set()
    frontier = [task]
    while frontier:
        current = frontier.pop()
        for dependency in _task_depends(mise, current):
            if dependency not in seen and dependency in mise["tasks"]:
                seen.add(dependency)
                frontier.append(dependency)
            else:
                seen.add(dependency)
    return seen


def test_the_measure_module_is_reached_by_at_least_one_task(mise):
    # Otherwise the guards below are vacuous, and the module rots invisibly.
    assert _tasks_reaching_measure(mise) >= {
        "measure:warm-dispatch",
        "measure:teardown",
        "test:integration:measure",
    }


@pytest.mark.parametrize("root", ["check", "default"])
def test_no_measure_reaching_task_is_in_the_ci_mirror(mise, root):
    closure = _transitive_depends(mise, root)
    offenders = sorted(closure & _tasks_reaching_measure(mise))
    assert not offenders, (
        f"{offenders} reach {_MEASURE_MODULE} from the transitive closure of "
        f"{root}.depends — every `mise run` would become a benchmark needing a "
        f"quiet host and network egress"
    )


@pytest.mark.parametrize("root", ["default", "test", "check"])
def test_no_local_mirror_task_reaches_the_live_contract_lane(mise, root):
    # The lane needs a real tenant, credentials no CI job holds and network
    # egress. Reachable from any of these roots, an unconfigured machine could
    # not run the local CI mirror at all — and the harness fails rather than
    # skips by design, so there is no benign outcome. Roll-up membership alone
    # would not catch a future edge added somewhere else in the closure.
    closure = _transitive_depends(mise, root) | {root}

    assert _CONTRACT_LANE not in closure, (
        f"{_CONTRACT_LANE} is reachable from {root}.depends — every "
        f"`mise run {root}` would then need live tracker credentials"
    )


def test_every_integration_task_is_in_the_rollup_or_excluded_with_a_reason(
    mise,
):
    rollup = set(_task_depends(mise, "test:integration"))
    assert rollup | set(_NOT_IN_INTEGRATION_ROLLUP) == _integration_tasks(mise)
    assert not rollup & set(_NOT_IN_INTEGRATION_ROLLUP)


def test_fixture_size_leaf_reaches_the_guard(mise):
    # The leaf's entire protective value is one depends edge; pin it so a
    # future removal fails here rather than passing vacuously green.
    assert "build:cli:fixture-size" in _transitive_depends(
        mise, "test:integration:fixture-size"
    )


def test_fixture_size_guard_runs_in_the_integration_rollup(mise):
    # Pin the roll-up -> guard half of the chain: the roll-up is the guard's
    # only CI home, so a future exclusion must not silently relocate it out of
    # CI.
    assert "build:cli:fixture-size" in _transitive_depends(
        mise, "test:integration"
    )
