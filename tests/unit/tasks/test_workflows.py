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


@pytest.fixture
def wf():
    return yaml.safe_load(WORKFLOW.read_text())


def test_workflow_topology_invariants_hold(wf):
    _invariants(wf)


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
