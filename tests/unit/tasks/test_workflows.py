"""Regression guard for the release-pipeline concurrency topology.

Encodes the invariants from the approval-gate split
(meta/plans/2026-06-14-release-concurrency-approval-gate-split.md). The core,
name-agnostic property is: no `environment`-gated job may carry the
`accelerator-release` lock — a Waiting gated job holds its concurrency group
for the whole, unbounded approval wait, so gating a lock-holding job IS the
original blocking bug. The two lock members must also stay symmetric
(`queue: max` + `cancel-in-progress: false`), and the approval gate must hold
**no** concurrency group at all — any group on a Waiting approval-gated job
would hold it through the approval wait and block every later push's gate from
reaching its prompt (the original bug, recreated in the approval lane), which
would also prevent picking which pipeline to release.

These are textual-structure assertions, not behaviour: they fail CI the moment
someone re-couples the gate and lock, drops `queue: max`, or puts a group back
on the gate, but they cannot prove the GHA runtime semantics — that is verified
observationally in CI (see the plan's Testing Strategy).
"""

import copy
from pathlib import Path

import pytest
import yaml

REPO_ROOT = Path(__file__).resolve().parents[3]
WORKFLOW = REPO_ROOT / ".github/workflows/main.yml"

LOCK_GROUP = "accelerator-release"


def _needs(job):
    # `needs` may be a scalar string or a list.
    n = job.get("needs", [])
    return [n] if isinstance(n, str) else list(n)


def _conc(job):
    # `concurrency` may be a dict OR the string-shorthand `concurrency: group`.
    c = job.get("concurrency")
    if isinstance(c, str):
        return {"group": c}
    return c if isinstance(c, dict) else {}


def _invariants(wf):
    """Raise AssertionError if the workflow violates a topology invariant."""
    jobs = wf["jobs"]

    # Core invariant (name-agnostic): no environment-gated job may carry the
    # release lock. Forbids re-gating the release work job, yet permits a safe
    # defence-in-depth gate on any lock-free job — the only place one could go
    # without reintroducing the bug.
    for name, job in jobs.items():
        if job.get("environment") is not None:
            assert _conc(job).get("group") != LOCK_GROUP, (
                f"{name} gates AND holds the release lock — the original bug"
            )

    # The gate is present where we put it.
    assert jobs["approve-release"].get("environment") == "release"

    # Wiring (named — deliberate): release is gated via approve-release, which
    # sits behind prerelease. prerelease and release carry identical write
    # permissions, so a permissions-based selector would be ambiguous; renaming
    # these jobs is a reviewable act the name-agnostic core invariant backstops.
    assert "approve-release" in _needs(jobs["release"])
    assert "prerelease" in _needs(jobs["approve-release"])

    # The gate holds NO concurrency group. Any group on a Waiting approval-gated
    # job would hold it for the whole approval wait and block every later push's
    # gate from reaching its prompt (the original bug in the approval lane), and
    # would stop you picking which pipeline to release. Stricter than the core
    # invariant above (which only forbids the accelerator-release lock): this
    # forbids ANY group on the gate.
    assert "concurrency" not in jobs["approve-release"], (
        "approve-release must hold no concurrency group"
    )

    # Symmetry: exactly the two lock members exist (so dropping the lock from
    # one job fails here rather than passing on the survivor), each with
    # queue: max and cancel-in-progress: false.
    blocks = [
        _conc(j) for j in jobs.values() if _conc(j).get("group") == LOCK_GROUP
    ]
    assert len(blocks) == 2, f"expected 2 lock members, got {len(blocks)}"
    for c in blocks:
        assert c.get("queue") == "max", "lock block must declare queue: max"
        assert c.get("cancel-in-progress") is False, "must stay false"


SIGN_SECRET = "ACCELERATOR_RELEASE_SECRET_KEY"


def _all_steps(jobs):
    for job_name, job in jobs.items():
        for step in job.get("steps") or []:
            yield job_name, step


def _references_secret(step):
    env = step.get("env") or {}
    return any(SIGN_SECRET in str(value) for value in env.values())


@pytest.fixture
def wf():
    return yaml.safe_load(WORKFLOW.read_text())


# --- Release signing secret scope + attestation coverage ---------------


def test_signing_secret_only_in_sign_steps(wf):
    # The secret is scoped to the dedicated Sign* step so it is never in the
    # environment during the cargo-zigbuild compile (untrusted build scripts).
    for job_name, step in _all_steps(wf["jobs"]):
        if _references_secret(step):
            name = step.get("name", "")
            assert name.startswith("Sign"), (
                f"{name!r} in {job_name} carries the signing secret but is "
                "not a Sign step"
            )


def test_every_sign_step_carries_the_secret(wf):
    sign_steps = [
        step
        for _job, step in _all_steps(wf["jobs"])
        if step.get("name", "").startswith("Sign")
    ]
    assert sign_steps, "no Sign* steps found"
    for step in sign_steps:
        assert _references_secret(step), (
            f"{step.get('name')!r} is a Sign step but does not reference "
            "the signing secret"
        )


def test_prepare_steps_never_carry_the_secret(wf):
    for _job, step in _all_steps(wf["jobs"]):
        if step.get("name", "").startswith("Prepare"):
            assert not _references_secret(step)


def test_attest_globs_include_the_launcher_binaries(wf):
    attest_steps = [
        step
        for _job, step in _all_steps(wf["jobs"])
        if str(step.get("uses", "")).startswith(
            "actions/attest-build-provenance"
        )
    ]
    assert attest_steps
    for step in attest_steps:
        subject = step.get("with", {}).get("subject-path", "")
        assert "dist/release/accelerator-*" in subject
        assert "accelerator-visualiser-*" in subject


def test_workflow_topology_invariants_hold(wf):
    _invariants(wf)


# --- Releaser app-token wiring -----------------------------------------
#
# The publishing jobs push commits + tags (release.py:_publish). A push made
# with the default GITHUB_TOKEN neither re-triggers Main CI nor satisfies
# branch protection, so each publishing job mints a GitHub App token via
# actions/create-github-app-token and checks the repo out WITH that token.
# Both jobs push, so both must be wired — the guard forbids the "only wired
# one job" asymmetry that shipped before this test existed.

APP_TOKEN_ACTION = "actions/create-github-app-token"
APP_TOKEN_OUTPUT = "steps.app-token.outputs.token"
# Jobs that call release.py:_publish (commit + tag + push) and therefore need
# an app-token-authenticated checkout.
PUBLISHING_JOBS = ("prerelease", "release")


def _step_action(step):
    return str(step.get("uses", "")).split("@", 1)[0]


def _app_token_step(job):
    for step in job.get("steps") or []:
        if _step_action(step) == APP_TOKEN_ACTION:
            return step
    return None


def _checkout_step(job):
    for step in job.get("steps") or []:
        if _step_action(step) == "actions/checkout":
            return step
    return None


@pytest.mark.parametrize("job_name", PUBLISHING_JOBS)
def test_publishing_job_mints_releaser_app_token(wf, job_name):
    job = wf["jobs"][job_name]
    step = _app_token_step(job)
    assert step is not None, (
        f"{job_name} pushes commits/tags but has no {APP_TOKEN_ACTION} step"
    )
    assert step.get("id") == "app-token", (
        f"{job_name}'s app-token step must have id: app-token so the "
        "checkout can consume its output"
    )
    with_ = step.get("with") or {}
    # Identify the app by client-id (the app-id input is deprecated upstream).
    assert "${{ vars.ACCELERATOR_RELEASER_CLIENT_ID }}" in str(
        with_.get("client-id", "")
    ), f"{job_name}'s app-token step must pass the releaser client-id"
    assert "${{ secrets.ACCELERATOR_RELEASER_SECRET }}" in str(
        with_.get("private-key", "")
    ), f"{job_name}'s app-token step must pass the releaser private-key secret"


@pytest.mark.parametrize("job_name", PUBLISHING_JOBS)
def test_publishing_job_checks_out_with_app_token(wf, job_name):
    job = wf["jobs"][job_name]
    checkout = _checkout_step(job)
    assert checkout is not None, f"{job_name} has no checkout step"
    token = str((checkout.get("with") or {}).get("token", ""))
    assert APP_TOKEN_OUTPUT in token, (
        f"{job_name} must check out with the releaser app token "
        f"({APP_TOKEN_OUTPUT}), not the default GITHUB_TOKEN"
    )


# --- Encoded negative tests: each mutation breaks exactly one invariant, so
#     the guard's own discriminating power is under test and cannot rot. ---


def _gate_on_lock_dict(jobs):
    # Re-gate the release work job (it already holds the lock as a dict).
    jobs["release"]["environment"] = "release"


def _gate_on_lock_string(jobs):
    # A gated job (approve-release) carrying the lock via string-shorthand —
    # the form a naive dict-only check would miss.
    jobs["approve-release"]["concurrency"] = LOCK_GROUP


def _drop_queue_from_lock(jobs):
    jobs["release"]["concurrency"].pop("queue")


def _group_on_approval_gate(jobs):
    # Any group on the gate recreates the approval-lane blocking bug — even a
    # benign-looking dedicated approval group.
    jobs["approve-release"]["concurrency"] = {
        "group": "accelerator-release-approval",
        "cancel-in-progress": False,
    }


def _cancel_in_progress_true(jobs):
    jobs["release"]["concurrency"]["cancel-in-progress"] = True


def _missing_approval_edge(jobs):
    jobs["approve-release"]["needs"] = []


_BAD_MUTATIONS = [
    _gate_on_lock_dict,
    _gate_on_lock_string,
    _drop_queue_from_lock,
    _group_on_approval_gate,
    _cancel_in_progress_true,
    _missing_approval_edge,
]


@pytest.mark.parametrize("mutate", _BAD_MUTATIONS)
def test_invariants_reject_known_bad_shapes(wf, mutate):
    bad = copy.deepcopy(wf)
    mutate(bad["jobs"])
    with pytest.raises(AssertionError):
        _invariants(bad)


# --- Nightly-lane isolation: cargo-pup runs on a pinned nightly, and that
#     toolchain must stay confined to a single job so a nightly break gates the
#     architecture check alone, never a stable-lane check or the product build.

# A job consumes the nightly iff its steps run any of these (name-agnostic
# detection, so renaming the job cannot smuggle a second consumer past the
# guard).
_NIGHTLY_MARKERS = ("pup:check", "deps:install:pup", "+nightly")
_NIGHTLY_JOB = "check-architecture"
# The release-pipeline aggregators MAY gate on check-architecture (you should
# not ship with a red required check); everything else must not couple to it.
_RELEASE_JOBS = {"prerelease", "approve-release", "release"}


def _job_run_text(job):
    return "\n".join(step.get("run", "") for step in job.get("steps") or [])


def _nightly_consumers(jobs):
    return {
        name
        for name, job in jobs.items()
        if any(marker in _job_run_text(job) for marker in _NIGHTLY_MARKERS)
    }


def _isolation_invariants(wf):
    jobs = wf["jobs"]

    # Exactly one nightly consumer, and it is check-architecture.
    consumers = _nightly_consumers(jobs)
    assert consumers == {_NIGHTLY_JOB}, (
        f"nightly consumers must be exactly {{{_NIGHTLY_JOB}}}, got {consumers}"
    )

    # The sole regression cannot be silently dropped: the one host job invokes
    # BOTH pup:check and its behavioural regression.
    text = _job_run_text(jobs[_NIGHTLY_JOB])
    assert "pup:check" in text, "check-architecture must run pup:check"
    assert "test:integration:pup" in text, (
        "check-architecture must run the pup regression"
    )

    # No stable-lane / product job couples to the nightly lane via needs.
    for name, job in jobs.items():
        if name == _NIGHTLY_JOB or name in _RELEASE_JOBS:
            continue
        assert _NIGHTLY_JOB not in _needs(job), (
            f"{name} needs {_NIGHTLY_JOB} — couples a stable job to the "
            f"nightly lane"
        )


def test_nightly_lane_isolation_holds(wf):
    _isolation_invariants(wf)


def _needs_edge_into_stable_job(jobs):
    # A stable check job made to depend on the nightly lane.
    jobs["check-cli"]["needs"] = [_NIGHTLY_JOB]


def _nightly_step_in_another_job(jobs):
    # A +nightly step smuggled into a stable job.
    jobs["check-cli"].setdefault("steps", []).append(
        {"name": "sneaky", "run": "cargo +nightly-2026-01-22 build"}
    )


_BAD_ISOLATION_MUTATIONS = [
    _needs_edge_into_stable_job,
    _nightly_step_in_another_job,
]


@pytest.mark.parametrize("mutate", _BAD_ISOLATION_MUTATIONS)
def test_isolation_rejects_known_bad_shapes(wf, mutate):
    bad = copy.deepcopy(wf)
    mutate(bad["jobs"])
    with pytest.raises(AssertionError):
        _isolation_invariants(bad)
