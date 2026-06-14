---
type: plan
id: "2026-06-14-release-concurrency-approval-gate-split"
title: "Release Concurrency Approval-Gate Split Implementation Plan"
date: "2026-06-14T12:02:57+00:00"
author: "Toby Clemson"
producer: create-plan
status: ready
derived_from: ["issue-research:2026-06-14-release-concurrency-group-blocks-prereleases"]
tags: [ci, github-actions, release, concurrency]
revision: "f4da84e73fa4cafea1ef809ad8239b9278361550"
repository: "miscellaneous"
last_updated: "2026-06-14T13:50:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Release Concurrency Approval-Gate Split Implementation Plan

## Overview

The `release` job in `.github/workflows/main.yml` carries *both* the
`environment: release` approval gate and the `accelerator-release` concurrency
group. Because GitHub Actions acquires a concurrency group at **queue time**
(not at approval time), a `release` job sitting in the "Waiting" approval state
holds the lock for its entire, unbounded approval wait — blocking every
subsequent `prerelease` from another push until a human approves or cancels.

This plan implements **Option A** from the root-cause analysis (split the
approval gate from the release work) and hardens it with the
**`queue: max`** concurrency option (the user-selected variant; GA-since-
2026-05-07 per GitHub, but **confirm before relying** — see Phase 2). The
result: the approval wait no longer holds the release lock, and an approved
release can never be silently evicted from the pending slot by a later-queued
prerelease.

A further hardening addresses review findings: a parser-based regression test
plus a pinned, unconditional `actionlint` (Phase 3) lock the new topology
against a future edit silently reintroducing the bug. (The `approve-release` job
holds **no** concurrency group: a group on a Waiting approval-gated job would
recreate the original blocking bug in the approval lane and prevent picking
which pipeline to release. The "two approved releases race" hazard a serialising
group would have addressed is instead handled by the `release` lock plus the
late-binding finalise, which make concurrent approvals cut sequential,
HEAD-correct releases — see Phase 1.)

## Current State Analysis

The relevant section of `.github/workflows/main.yml`:

- **`prerelease` job** (`:184-234`): `runs-on: macos-latest`, `if:
  github.event_name == 'push'`, `needs:` all eight check jobs. Holds
  `concurrency: { group: accelerator-release, cancel-in-progress: false }`
  (`:202-204`). Runs `prerelease:prepare` → attest → `prerelease:finalise`.
- **`release` job** (`:236-291`): `runs-on: macos-latest`, `if:
  github.event_name == 'push'`, `needs: prerelease`,
  **`environment: release`** (`:241`, the approval gate), and the **same**
  `concurrency: { group: accelerator-release, cancel-in-progress: false }`
  (`:244-246`). Runs `release:prepare` → attest → `release:finalise`, then
  re-cuts a post-stable prerelease (`prerelease:prepare` → attest →
  `prerelease:finalise`, `:278-291`).

The genuine mutual-exclusion requirement is narrow: only `_publish` in
`tasks/release.py:23-29` (commit → tag → **push** → create release → upload)
truly races across release types. `release_prepare` (`tasks/release.py:56-70`)
does `git pull` + `version.bump(FINALISE)` at execution time, so a late-approved
release correctly finalises against current `main` HEAD — this plan does not
change or harm that.

### Key Discoveries:

- **Concurrency is acquired at queue time, decoupled from environment
  approval** (`main.yml:241` + `:244`). A "Waiting" approval-gated job holds its
  group for the whole wait — this is the root cause.
- **`cancel-in-progress: false` is correct and must stay** (`main.yml:204`,
  `:246`). It stops an in-flight release being killed mid-push. Hypothesis 2 in
  the research eliminated it as the cause.
- **`queue: max` is documented as GA (2026-05-07) but the claim rests on a
  single recent source and must be confirmed before relying on it.** The
  root-cause research flagged it as "shipped but verify before relying"; this
  plan does not upgrade that to fact. Per the docs it sits as a sibling of
  `group:` / `cancel-in-progress:` and turns the single-pending-slot model into
  a FIFO queue of up to 100 pending runs (ordered by time entered the wait
  state). **Hard constraint:** `queue: max` combined with
  `cancel-in-progress: true` is a workflow validation error — it is only valid
  when `cancel-in-progress` is `false` or omitted. Our blocks already use
  `false`, so it is compatible. **Assumption:** the deployment target is
  github.com, not GitHub Enterprise Server (GHES feature GA lags github.com).
  Phase 2 gates on a throwaway-workflow acceptance check before merge, and is
  independently revertible if the feature is rejected (see Phase 2 + Migration
  Notes).
- **No environment-scoped secrets are referenced by the `release` job — but
  the `release` environment carries more than secrets, and those move too.**
  The job's steps only use `secrets.GITHUB_TOKEN` (repo-level, always
  available) and OIDC (`id-token: write`). Two non-secret consequences of
  moving `environment: release` off the work job must be confirmed before
  merge (see Manual Verification):
  - **OIDC `sub`-claim scoping.** GitHub injects `environment:<name>` into the
    OIDC `sub` claim only for jobs that declare `environment:`. The `release`
    job mints OIDC tokens for `actions/attest-build-provenance`; after the
    split its `sub` changes from an `environment:release`-scoped subject to a
    ref-scoped one. If any attestation relying-party pins `environment:release`
    this breaks provenance trust — verify the minted `sub` / `job_workflow_ref`
    is not relied upon in that form.
  - **Other environment protection rules.** Deployment-branch restrictions and
    custom protection rules attached to the `release` environment now apply to
    `approve-release`, not the work job. Enumerate all such rules and confirm
    each still meaningfully constrains the privileged operation.
- **No automated GHA-workflow linter exists in this repo.** The four component
  checks (`frontend`, `server`, `build-system`, `scripts`) do not cover
  `.github/workflows/`. Verification of concurrency *behaviour* is necessarily
  observational in CI.

## Desired End State

The release pipeline behaves as originally intended:

- A prerelease runs on every push to `main`.
- A stable release is approved in-pipeline "when ready" via the `release`
  environment.
- **An unapproved stable release never blocks subsequent prereleases.**
- The actual commit/tag/push window remains mutually exclusive across release
  types (no two pipelines race their pushes or collide on a version).
- An approved-and-queued stable release is never silently dropped by a
  later-queued prerelease (it FIFO-queues instead).
- Every push's stable-release approval is an **independent, simultaneously
  visible** gate that can be approved in any order — you can pick which pipeline
  to release. The gate holds no concurrency group, so it never blocks a later
  push's gate (a serialising group would, by holding the group through the
  approval wait — the original bug in the approval lane). Approving several does
  not race two *simultaneous* releases: the `release` lock + late-binding
  finalise make concurrent approvals cut **sequential, HEAD-correct** stable
  versions (each approved push gets its release, by design). Declining a gate
  (or letting it time out) simply means that push never releases.
- Release queue order is governed by lock-entry (= approval) time, not push
  time; the late-binding finalise (`release_prepare` re-pulls and re-bumps
  against current HEAD) makes any resulting interleaving safe.
- The new topology is protected by an automated regression guard, so a future
  workflow edit cannot silently reintroduce the blocking or eviction bug.

Verifiable by: workflow YAML parses and (if available) passes `actionlint`; CI
observation that a push landing while a stable release sits unapproved still
produces a prerelease; and that an approved release serialises behind any
running prerelease rather than being evicted.

## What We're NOT Doing

- **Not** changing `cancel-in-progress: false` — it is correct and required for
  `queue: max` compatibility.
- **Not** moving the post-stable prerelease re-cut (`main.yml:278-291`) out of
  the `release` job — per decision, it stays inside the release work job, within
  the same lock.
- **Not** adopting Option B (a separate `workflow_dispatch` release workflow) —
  Option A + `queue: max` was selected.
- **Not** modifying `tasks/release.py` or its version/publish logic — the
  stacked-approval hazard is addressed by serialising the approval gate (a
  concurrency group), not by adding already-published guards to the release
  task.
- **Not** building a general-purpose workflow linter. Phase 3 pins `actionlint`
  and adds a *narrow* parser-based regression test asserting this change's
  specific invariants (gate placement, `needs` wiring, concurrency settings);
  it does not attempt to validate `.github/workflows/` comprehensively.

## Implementation Approach

Three independently mergeable phases. Phases 1 and 2 edit only
`.github/workflows/main.yml` in non-overlapping ways; Phase 3 adds tooling.

1. **Phase 1** restructures the jobs: a new no-op `approve-release` job holds
   `environment: release` and **no concurrency group**; the `release` work job
   drops `environment` and instead `needs: approve-release`, keeping the
   `accelerator-release` lock. This makes the reported blocking disappear — the
   approval wait no longer holds the *release* lock — while keeping every push's
   approval gate independent and pickable (a group on the gate would re-block in
   the approval lane). **Phase 1 is a strict improvement but not the complete
   fix**: the residual single-pending-slot eviction risk (Hypothesis 3) remains
   until Phase 2.
2. **Phase 2** adds `queue: max` to both concurrency blocks referencing
   `accelerator-release`, eliminating the residual eviction risk. It is valuable
   and mergeable on its own and does not depend on Phase 1's structure, but
   carries the `queue: max` platform-feature risk (gated on a pre-merge
   acceptance check; independently revertible).
3. **Phase 3** locks the new topology in place: pin `actionlint` and run it
   unconditionally, and add a narrow parser-based regression test asserting the
   gate/lock/`needs`/concurrency invariants. This converts the one-shot
   verification into permanent protection and is independent of Phases 1–2
   (though its assertions describe their end state).

Each phase leaves the workflow valid and CI green, so any can merge alone.

The end-state concurrency topology (two distinct scopes):

| Group | Member job(s) | cancel-in-progress | queue |
|-------|---------------|--------------------|-------|
| `accelerator-release` (release lock) | `prerelease`, `release` | `false` | `max` (FIFO) |
| *(none)* | `approve-release` (gate), the eight check jobs | — | — |

`approve-release` deliberately holds **no** concurrency group: any group on a
Waiting approval-gated job would hold that group for the whole approval wait and
block every *later* push's approval gate from reaching its own prompt — the
original bug, recreated in the approval lane, and the thing that would stop you
picking which pipeline to release. The release lock + late-binding finalise (not
an approval-lane lock) is what keeps concurrent approved releases safe.

## Phase 1: Split the approval gate from the release work

### Overview

Introduce an `approve-release` job that carries the `environment: release`
approval gate and **no concurrency group at all** — gated on `needs:
prerelease`. Repoint the `release` work job to `needs: approve-release` and
**remove** its `environment: release`, so the release lock is acquired only when
the *approved* release work is queued — not during the approval wait. Because
the gate holds no group, every push's approval gate is independent and
simultaneously visible, so any pending release can be approved in any order
("pick which pipeline to release") and the gate can never block a later push's
gate or any prerelease.

Giving the gate its own concurrency group was considered and rejected: a Waiting
approval-gated job holds its group for the whole approval wait, so *any* group
on this job would recreate the original blocking bug inside the approval lane —
the oldest pending approval would hold the group and stop every later push's gate
from even reaching its prompt, forcing strict oldest-first handling and
defeating the "pick which pipeline" goal. The protection that group was meant to
provide (no two simultaneous independently-approved releases) is already supplied
one layer down by the `release` lock (`accelerator-release`, `queue: max`) plus
the late-binding finalise (`release_prepare` re-pulls + re-bumps against current
HEAD), so concurrent approvals cut sequential, HEAD-correct releases rather than
racing.

### Changes Required:

#### 1. Add the `approve-release` job

**File**: `.github/workflows/main.yml`
**Changes**: Insert a new job between `prerelease` (ends `:234`) and `release`
(starts `:236`). It is a no-op whose sole purpose is to host the approval gate
without holding the concurrency lock.

```yaml
  approve-release:
    name: Approve release
    # Cheapest host for a no-op gate: this job runs one trivial step and needs
    # no toolchain (macOS is only required for the cross-compile/attest steps on
    # the prerelease/release work jobs).
    runs-on: ubuntu-latest
    if: github.event_name == 'push'
    needs: prerelease
    # The approval gate lives HERE and carries NO concurrency group — on
    # purpose. GitHub Actions acquires a concurrency group at queue time, so a
    # job in the "Waiting" approval state holds its group for the entire,
    # unbounded approval wait. Giving this gate ANY group would recreate the
    # original bug inside the approval lane: a Waiting approval would hold the
    # group and stop every later push's approval gate from even reaching its own
    # prompt — you could only ever act on the oldest pending approval, never
    # pick which pipeline to release.
    #
    # So each push spawns an independent, simultaneously-visible gate and any
    # can be approved in any order. Approving several does NOT race two
    # simultaneous releases: the `release` work job's accelerator-release lock
    # (queue: max) serialises the publish window, and release:prepare re-pulls +
    # re-bumps against current HEAD at execution time, so each approved release
    # cuts a sequential, HEAD-correct stable version. That is the intended "each
    # approved push gets its release" semantics, not a hazard.
    environment: release
    steps:
      # No-op: this job exists solely to host the environment approval gate.
      - name: Approval gate (no-op; the gate is the environment, not this step)
        run: echo "Stable release approved"
```

> Whether independent approval gates behave exactly as described (all visible,
> independently approvable, none blocking another) is a GitHub Actions
> behaviour that cannot be verified locally — confirm it during CI observation
> (Testing Strategy).

#### 2. Rewire the `release` job: drop `environment`, depend on `approve-release`

**File**: `.github/workflows/main.yml`
**Changes**: On the existing `release` job (`:236-291`), remove
`environment: release` (`:241`) and change `needs: prerelease` (`:240`) to
`needs: approve-release`. Keep the concurrency block and all steps unchanged.
Update the surrounding comment to document the pitfall.

On gate **rejection or timeout**, `approve-release` does not succeed, so
`release` (which `needs:` it) is skipped: it never queues for the
`accelerator-release` group and therefore never holds the release lock —
subsequent prereleases are unaffected. The happy path (approve → release runs)
is otherwise as before.

The `release` job deliberately retains its two responsibilities — cut the
stable release *and* re-cut the post-stable prerelease (`:278-291`) — inside one
locked, approved job. This is an accepted cohesion tradeoff, not an oversight:
the post-stable prerelease must run *after* the stable push and *within the same
serialised `accelerator-release` window*, otherwise a concurrent pipeline could
bump/push between the stable cut and the pre.0 re-cut and break version
monotonicity. The comment below records that rationale.

```yaml
  release:
    name: Create release
    runs-on: macos-latest
    if: github.event_name == 'push'
    needs: approve-release
    # The release lock lives HERE, deliberately WITHOUT `environment`. This
    # group must never enclose the approval gate (see approve-release) — a
    # Waiting, approval-gated job holds its concurrency group for its whole
    # wait, which would block every subsequent prerelease. By the time this job
    # is queued the release is already approved, so the lock covers only the
    # active commit/tag/push window (tasks/release.py:_publish), exactly as
    # intended.
    # Keep in sync with the other accelerator-release block: both members must
    # declare identical settings (group, cancel-in-progress: false, queue: max).
    concurrency:
      group: accelerator-release
      cancel-in-progress: false
    permissions:
      id-token: write
      contents: write
      attestations: write

    steps:
      # Unchanged: checkout, install, release:prepare, attest, release:finalise,
      # then the post-stable prerelease prepare/attest/finalise. The post-stable
      # re-cut stays INSIDE this locked, approved job on purpose: it must run
      # after the stable push and within the same accelerator-release window so
      # no concurrent pipeline can bump/push between the two cuts (version
      # monotonicity). Do not hoist it into a separate job.
```

### Success Criteria:

#### Automated Verification:

- [x] Workflow YAML parses and the **invariants** hold (asserted against the
      parsed document). The *core* anti-bug invariant is name-agnostic; the
      *wiring* checks reference the named jobs deliberately (a rename of those is
      a reviewable act, and the name-agnostic core is the safety net). This is a
      throwaway subset of the permanent Phase 3 test
      (`tests/unit/tasks/test_workflows.py`), which is the single source of
      truth — keep the assertion wording aligned. Run:
      ```bash
      python - <<'PY'
      import yaml
      wf = yaml.safe_load(open(".github/workflows/main.yml"))
      jobs = wf["jobs"]
      def needs_of(j):  # needs may be a scalar or a list
          n = j.get("needs", [])
          return [n] if isinstance(n, str) else list(n)
      def conc(j):  # concurrency may be a dict OR the string-shorthand form
          c = j.get("concurrency")
          if isinstance(c, str):
              return {"group": c}
          return c if isinstance(c, dict) else {}
      # Core invariant (name-agnostic): NO environment-gated job may carry the
      # accelerator-release lock. A Waiting gated job holds its group for the
      # whole approval wait, so gating a lock-holding job IS the original bug.
      # This forbids re-gating the release work job, yet permits a safe future
      # defence-in-depth gate on any lock-FREE job (the accepted re-run-bypass).
      for n, j in jobs.items():
          if j.get("environment") is not None:
              assert conc(j).get("group") != "accelerator-release", \
                  f"{n} gates AND holds the release lock — the original bug"
      # The gate is present where we put it.
      assert jobs["approve-release"].get("environment") == "release"
      # Wiring: release is gated via approve-release; approval sits behind prerelease.
      assert "approve-release" in needs_of(jobs["release"])
      assert "prerelease" in needs_of(jobs["approve-release"])
      # The gate holds NO concurrency group: any group on a Waiting approval-
      # gated job would hold it for the whole approval wait and block every
      # later push's gate from reaching its prompt (the original bug in the
      # approval lane). Independent gates are what let you pick which pipeline
      # to release.
      assert "concurrency" not in jobs["approve-release"], \
          "approve-release must hold no concurrency group"
      print("Phase 1 invariants OK")
      PY
      ```
- [x] Existing repo checks remain green: `mise run check`

#### Manual Verification:

- [ ] **OIDC `sub`-claim scoping is not relied upon as `environment`-scoped.**
      Inspect the `sub` / `job_workflow_ref` the `release` job mints for
      `attest-build-provenance` and confirm no relying-party (sigstore/Fulcio
      attestation identity, any registry/signing trust policy) pins
      `environment:release`. If it does, the gate must stay on (or be
      re-asserted at) the work job — treat this as a **blocking prerequisite**.
- [ ] **All `release` environment protection rules still constrain the
      privileged operation.** In repo settings, enumerate every rule on the
      `release` environment — required reviewers, wait timer, deployment-branch
      restrictions, env-scoped secrets, custom rules — and confirm each still
      meaningfully applies after the gate moves to `approve-release`. In
      particular confirm **no env-scoped secret** the work job needs is lost
      (the job uses only repo-level `GITHUB_TOKEN` + OIDC); if any required
      secret/branch rule is found, re-scope it or keep the gate on the work job.
- [ ] On a push to `main`, the approval prompt now appears on the
      **Approve release** job, and approving it triggers the **Create release**
      job as before.
- [ ] While a stable release sits **unapproved**, a *new* push to `main` still
      produces a prerelease (it is no longer blocked) — the core fix.
- [ ] Approving a long-pending `approve-release` still produces a correct stable
      release finalised against current `main` HEAD. **A successful end-to-end
      stable release here is the binding proof that no env-scoped secret was
      lost** (a settings inspection can miss a secret consumed indirectly inside
      `tasks/release.py`).
- [ ] **Re-run bypass (accepted, documented risk).** Re-run the **Create
      release** job alone from the Actions UI and record whether GitHub
      re-presents the approval prompt. The `needs: approve-release` edge is a
      weaker control than an `environment:` gate on the work job, so a re-run
      may re-dispatch the privileged push/publish/attest steps without
      re-approval. This residual risk is **accepted** — but the acceptance rests
      on re-run rights being maintainer-only, so confirm that in repo settings
      (it is an observed setting, not an assumption). Also record whether the
      re-run produces any Environments deployment/approval record at all, so the
      audit gap is characterised. Document the observed behaviour so it is known.
- [ ] **Independent, pickable approvals.** With two unapproved `approve-release`
      gates pending (two pushes), confirm **both** are simultaneously awaiting a
      decision (neither blocks the other from showing its prompt) and that you
      can approve the **second** while leaving the first pending — i.e. you can
      pick which pipeline releases. Then confirm approving both does not race two
      simultaneous stable releases: the two `release` jobs serialise on the
      `accelerator-release` lock and each finalises against current HEAD,
      yielding sequential, distinct stable versions (the intended "each approved
      push gets its release"). Declining/ignoring a gate must leave subsequent
      pushes' prereleases and approvals unaffected.

---

## Phase 2: Harden against pending-slot eviction with `queue: max`

### Overview

With both job types in one concurrency group, the default single-pending-slot
rule means a just-approved `release` pending behind a *running* `prerelease` can
be evicted if another `prerelease` queues (Hypothesis 3). Add `queue: max` to
both concurrency blocks referencing `accelerator-release` so pending runs FIFO-
queue (up to 100 deep) instead of evicting one another. Requires
`cancel-in-progress: false` (already set), or GitHub rejects the workflow.

**`queue: max` is a recent platform feature and its acceptance must be
confirmed before this phase is relied upon.** A workflow-level validation error
does not fail a single job — GitHub refuses to start the *entire* run, so a
rejected or unsupported `queue` key would block prereleases and releases alike:
the exact outcome this plan exists to prevent. None of the local checks (YAML
parse, greps, even `actionlint`, which has historically not validated
`concurrency` sub-keys) can detect this. Therefore:

- **Pre-merge acceptance check (acceptance only):** push a throwaway workflow
  on a non-`main` branch containing only
  `concurrency: { group: tmp, cancel-in-progress: false, queue: max }` and
  confirm GitHub *accepts* it (no validation error). This proves the key is
  valid; it does **not** prove the FIFO/no-eviction *behaviour* — that is a
  distinct contract verified post-merge by the contended-window observation
  (Testing Strategy step 4). If you want behaviour confirmation pre-merge too,
  queue three runs on the throwaway `tmp` group (with a `sleep`) and confirm
  none show "Canceled". Only after acceptance, merge Phase 2.
- **Contingency:** if GitHub rejects or silently ignores `queue: max`, revert
  Phase 2 **independently** (Phase 1 already removes the blocking) and track the
  residual eviction risk as accepted. This assumes the target is github.com;
  on GHES, confirm the feature has reached the installed version first.

### Changes Required:

#### 1. Add `queue: max` to the `prerelease` concurrency block

**File**: `.github/workflows/main.yml`
**Changes**: Extend the `prerelease` job's concurrency block (`:202-204`). Fold
the existing above-block serialisation comment (`:197-201`) and the new
`queue: max` rationale into one coherent comment block above the `concurrency:`
key, rather than leaving the rationale split between an above-block preamble and
an interleaved note — matching the placement of the existing comment.

```yaml
    # Serialise all release pipelines onto one queue: prepare (pull + bump) and
    # finalise (commit + tag + push) are steps in this single job, so a queued
    # pipeline cannot start its prepare until the running one has pushed — which
    # stops two pipelines bumping to the same version and racing their pushes.
    # cancel-in-progress: false makes runs wait rather than abort; queue: max
    # then FIFO-queues pending runs (up to 100) instead of evicting the single
    # pending slot, so an approved release queued behind a running prerelease is
    # never dropped. (queue: max with cancel-in-progress: true is a validation
    # error — keep it false.)
    # Keep in sync with the other accelerator-release block: both members must
    # declare identical settings (group, cancel-in-progress: false, queue: max).
    concurrency:
      group: accelerator-release
      cancel-in-progress: false
      queue: max
```

#### 2. Add `queue: max` to the `release` concurrency block

**File**: `.github/workflows/main.yml`
**Changes**: Add the same `queue: max` line to the `release` job's concurrency
block (the one introduced/kept in Phase 1). Both members of the group must
declare it so the queueing behaviour is consistent.

```yaml
    concurrency:
      group: accelerator-release
      cancel-in-progress: false
      queue: max
```

### Success Criteria:

#### Automated Verification:

- [ ] Symmetry holds, asserted against the parsed document: **every**
      `accelerator-release` concurrency block declares `queue: max` and
      `cancel-in-progress: false` (never `true`). Run:
      ```bash
      python - <<'PY'
      import yaml
      wf = yaml.safe_load(open(".github/workflows/main.yml"))
      def conc(j):  # dict or string-shorthand form
          c = j.get("concurrency")
          return {"group": c} if isinstance(c, str) else (c if isinstance(c, dict) else {})
      blocks = [conc(j) for j in wf["jobs"].values()
                if conc(j).get("group") == "accelerator-release"]
      # Expect exactly the two members (prerelease + release). A future edit that
      # drops the lock from one job must fail here, not pass on the survivor.
      assert len(blocks) == 2, f"expected 2 accelerator-release members, got {len(blocks)}"
      for c in blocks:
          assert c.get("queue") == "max", "every block must declare queue: max"
          assert c.get("cancel-in-progress") is False, "must stay false"
      print("2 accelerator-release blocks, all queue: max + false")
      PY
      ```
- [ ] Existing repo checks remain green: `mise run check`
- [ ] (After Phase 3 lands) `actionlint` passes unconditionally:
      `mise run lint:workflows`.

#### Manual Verification:

- [ ] **Pre-merge:** the throwaway-workflow acceptance check (Phase 2 Overview)
      passed — GitHub accepts `queue: max` with `cancel-in-progress: false`.
- [ ] On the next push after merge, GitHub accepts the real workflow (no
      validation error about `queue` / `cancel-in-progress`). If it is rejected,
      execute the contingency: revert Phase 2 independently.
- [ ] Reproduce the contended case rather than waiting to observe it: widen the
      contention window (prefer a long-running real release; if you must add a
      `sleep`/slow step to the prerelease finalise to do it, **do so on a
      throwaway branch only, never merge it, and remove it after** — a leftover
      delay would widen the very commit/tag/push race this plan narrows).
      Approve a release so it queues behind a running prerelease, then fire a
      second push in quick succession. Strengthen the oracle beyond "no
      Canceled": via `gh run list` / per-run timestamps, confirm all three runs
      reach **success**, the approved release's start time is **after** the
      prerelease's (it genuinely queued behind, not merely ran later), and
      nothing shows a "Canceled" state — so a missed-contention-window run is
      distinguishable from a real FIFO queue.

---

## Phase 3: Lock in the invariant (regression guard + deterministic actionlint)

### Overview

The fix's correctness now rests on a multi-job invariant that no check
enforces (gate on `approve-release` only; release lock never on a gated job;
both `accelerator-release` blocks symmetric with `queue: max` +
`cancel-in-progress: false`). A future workflow edit could silently reintroduce
the blocking or eviction bug — the exact production-only failure this plan
exists to remove. Phase 3 converts the one-shot checks above into permanent,
deterministic protection: a pinned, unconditionally-run `actionlint`, and a
narrow parser-based test asserting the specific invariants.

### Changes Required:

#### 1. Pin `actionlint` and run it unconditionally

**Files**: `mise.toml`, `tasks/lint/workflows.py` (new), `tasks/lint/__init__.py`
**Changes**:
- Add `actionlint` to `mise.toml` `[tools]` pinned to an **exact** version (not
  a `1.7.x` range — match the exact-pin discipline of every other tool, since a
  floating patch could resolve to a build that errors on the `queue:` sub-key on
  one runner but not another). Unlike the existing tools
  (uv/python/gh/rust/node/shellcheck/shfmt/jj/jq), actionlint is **not** a mise
  core/registered tool, so it must declare a backend explicitly — e.g.
  `"aqua:rhysd/actionlint" = "1.7.7"` (or `ubi:rhysd/...`). A bare
  `actionlint = "x.y.z"` may not resolve. Confirm **on a CI runner** (the
  shared install step feeds all check jobs) that (a) the chosen *backend*
  resolves under CI conditions — aqua-registry reachable / ubi not
  rate-limited, linux-x86_64 asset present — and (b) the pinned version does not
  itself *reject* `queue: max` (a version predating the key could error on the
  unknown sub-key). If neither backend resolves cleanly, fall back to invoking a
  pinned actionlint another way rather than wedging the shared install.
- Add a `tasks/lint/workflows.py` with an `actionlint` task that runs
  `actionlint .github/workflows/main.yml` (following the fail-loud `Exit`
  pattern in `tasks/lint/scripts.py`). Register the new module in
  `tasks/lint/__init__.py` (add `workflows` to both the import line and
  `__all__`, mirroring the existing modules). Expose a `lint:workflows` mise
  task and a `lint:workflows:check`. **CI never runs the aggregate `mise run
  check`** — each of the four component jobs runs its own `<component>:check`
  (e.g. `scripts:check`) directly. So to actually run in CI, fold
  `lint:workflows:check` into the **`build-system:check`** roll-up (it is a
  Python `tasks/` lint, and that component job already runs on `ubuntu-latest`)
  — *not* the top-level `check`, which CI does not invoke. Drop the "if
  available" hedge entirely.
- **Scope note:** actionlint's role here is general GHA-syntax hygiene. It does
  **not** validate `concurrency` sub-keys, so it is *not* the guard for the
  `queue: max` / gate-placement invariants — the parser-based test (below) is.

#### 2. Add a narrow parser-based regression test

**File**: `tests/unit/tasks/test_workflows.py` (new)
**Changes**: A pytest (sitting alongside `test_lint.py`, no `__init__.py` per
the repo's pytest importlib convention) with a single **`_invariants(wf)`**
helper that **raises `AssertionError`** on any violation (matching the heredoc
style; the negative test below uses `pytest.raises(AssertionError)`). Parse once
via `yaml.safe_load`, normalise `needs` (scalar or list), and normalise
`concurrency` (it may be a **dict OR the string-shorthand** `concurrency:
groupname` — coerce a string to `{"group": s}`; the dict-only assumption is a
real blind spot). Assert:

- **Core invariant — gate never on the release lock (name-agnostic):** iterate
  all jobs and assert **no** `environment`-bearing job carries normalised
  `concurrency.group == "accelerator-release"`. A Waiting gated job holds its
  group for the whole approval wait, so gating a lock-holding job *is* the
  original blocking bug. Expressed without hard-coding job names so a rename
  can't evade it — this is the primary safety net.
- **Gate present:** `approve-release.environment == "release"`.
- **Wiring (named — deliberate):** `approve-release` is in `release`'s
  normalised `needs`, and `prerelease` is in `approve-release`'s normalised
  `needs`. Reference these jobs by name: `prerelease` carries the **same**
  write `permissions` as `release`, so a permissions-based selector would be
  ambiguous (it matches both) — and renaming these jobs is a reviewable act the
  name-agnostic core invariant still backstops.
- **Gate holds no group:** `"concurrency" not in approve-release`. Any group on
  a Waiting approval-gated job would hold it for the whole approval wait and
  block every later push's gate from reaching its prompt (the original bug in
  the approval lane), and would prevent picking which pipeline to release. This
  is stricter than the name-agnostic core invariant (which only forbids the
  `accelerator-release` lock specifically) — it forbids *any* group on the gate,
  including a re-introduced `accelerator-release-approval`.
- **Symmetry:** exactly the two `accelerator-release` members exist and each
  declares `queue == "max"` and `cancel-in-progress` `False` (assert the
  count, so dropping the lock from one job fails rather than passing on the
  survivor).
- **Do NOT** assert "exactly one `environment: release`" or the absence of
  `environment` on the release work job. The core invariant already forbids the
  *unsafe* second gate (on the lock-holding release job) while permitting a
  *safe* defence-in-depth gate on any lock-free job — the only place one could
  go without reintroducing the bug. Keep the Phase 1 heredoc and this test
  aligned; this test is the single source of truth (the heredoc's named-wiring
  form matches the named-wiring above — an accepted, literal subset).
- **Encoded negative tests (parametrised):** feed `_invariants` a set of
  in-memory `wf` dicts each mutated to one known-bad shape — gate-on-the-lock,
  `queue` dropped from a lock block, `accelerator-release` in string-shorthand
  on a gated job, **any group added to the gate**, `cancel-in-progress: true`,
  missing `prerelease`→`approve-release` edge
  — and assert each raises (`with pytest.raises(AssertionError):`). Each
  asserted invariant gets at least one mutation, so the guard's discriminating
  power is itself under test and cannot silently rot. (Supersedes the one-shot
  manual mutation check.)

These are textual-structure assertions, not behaviour — they cannot prove the
GHA runtime semantics (that is observational, below), but they fail CI the
moment someone re-couples the gate and lock or drops `queue: max`.

### Success Criteria:

#### Automated Verification:

- [ ] `actionlint` is pinned in `mise.toml` and `mise run lint:workflows`
      passes against `.github/workflows/main.yml`.
- [ ] `lint:workflows:check` is reached by `mise run build-system:check` (the
      roll-up CI actually invokes), so it runs in CI — confirm by inspecting the
      `build-system:check` task graph, not just the top-level `check`.
- [ ] The new test passes, including its **encoded negative test** that feeds a
      known-bad in-memory workflow (gate on the `accelerator-release` group;
      `queue` dropped) to `_invariants` and asserts it is rejected: `uv run
      pytest tests/unit/tasks/test_workflows.py -v`.
- [ ] Full local CI mirror is green: `mise run`.

#### Manual Verification:

- [ ] None — Phase 3 is entirely statically verifiable.

---

## Testing Strategy

There is no unit-testable surface here — the change is workflow job topology and
concurrency configuration, and GHA concurrency behaviour cannot be exercised
locally. The testing strategy is therefore: static validation locally, then
observation in CI.

### Static (local) validation:

- YAML parses cleanly.
- `actionlint` (pinned in `mise.toml` from Phase 3, run unconditionally via
  `mise run lint:workflows`) passes. Its role is general GHA-syntax hygiene; it
  must not *reject* the workflow, but it does **not** validate `concurrency`
  sub-keys and is therefore not the guard for the `queue: max` / gate-placement
  invariants — the parser test below is.
- The parser-based regression test (`tests/unit/tasks/test_workflows.py`,
  Phase 3) is the standing guard: it asserts the invariants **by property** —
  no `environment`-bearing job on the `accelerator-release` lock (the core
  anti-bug property), the gate present on `approve-release`, the
  release-privileged job transitively gated via `approve-release` (itself behind
  `prerelease`), the gate holding **no** concurrency group, and `queue: max` +
  `cancel-in-progress: false` symmetric across all `accelerator-release` blocks
  — plus an encoded negative test. It keys off job *properties*, not names, so a
  rename cannot slip a recoupling past it.
- Pre-merge only: the throwaway-workflow acceptance check for `queue: max`
  (Phase 2 Overview), since no local tool validates the `queue` key.

### Observational (CI) verification:

1. Push to `main`; confirm the prerelease runs and the approval prompt appears
   on **Approve release**.
2. Leave the release unapproved; push again; confirm the second prerelease runs
   (not blocked) — the primary success condition.
3. Approve the original release; confirm it finalises correctly against current
   HEAD and serialises behind any running prerelease rather than racing it. A
   successful end-to-end release here also proves no env-scoped secret was lost.
4. (Phase 2) Reproduce the contended window deliberately (prefer a long-running
   real release; any `sleep`/slow step to force contention goes on a throwaway
   branch only, is never merged, and is removed after). Approve a release behind
   a running prerelease, fire a second push, and confirm via `gh run list` /
   per-run timestamps that all three runs succeed, the approved release starts
   *after* the prerelease (genuinely queued behind it), and none shows a
   "Canceled" state — FIFO-queued, not evicted.
5. With two `approve-release` gates pending, confirm they are **both**
   independently awaiting a decision (neither blocks the other) and that
   approving the second while leaving the first pending releases that pipeline —
   you can pick which to release. Confirm approving both yields sequential,
   distinct stable releases (serialised by the `release` lock), not a race. Also
   record whether re-running the `release` job alone re-requests approval (the
   accepted re-run-bypass observation).

## Performance Considerations

Negligible. The added `approve-release` job is a single-step no-op on
`ubuntu-latest`; its only "cost" is the (intended) approval wait, which holds no
concurrency group, so it never delays prereleases or other approvals. `queue:
max` adds no runtime cost and only changes how pending runs are ordered. Phase
3's `actionlint` run is a fast static check on one file.

## Migration Notes

- No data migration. Phases 1–2 are workflow-only; Phase 3 adds a pinned tool
  (`mise.toml`), a lint task, and a test — all additive, no migration.
- The GitHub "Environments" deployment record for the `release` environment will
  now be associated with the `approve-release` job rather than `release`. This
  is a cosmetic/tracking change only; environment protection rules (required
  reviewers, wait timers, branch rules) continue to apply because they attach to
  any job referencing the environment — but confirm the OIDC `sub`-claim and
  deployment-branch implications first (see Phase 1 Manual Verification).
- **Rollback distinguishes two things.** Reverting the workflow change (VCS
  revert) fixes all *future* runs and has no external state to undo. But the
  failure modes this change could introduce (a stale-HEAD finalise, an evicted
  approved release, or — when several gates are approved — sequential stable
  cuts) manifest as artifacts a faulty run *already published*: a pushed
  tag and a created GitHub release. A workflow revert does **not** undo those;
  recovery is manual (delete the tag, delete/yank the GitHub release, restore
  version files). Two existing guards in `_publish` (push(atomic) →
  `create_release` → `upload_and_verify`, `tasks/release.py:23-29`) shape this:
  - `git push --atomic` binds the branch+tag refs, so a push failure never
    orphans a tag.
  - `create_release` creates the release as a **draft** (`--draft`), and
    `upload_and_verify` only flips it live (`gh release edit --draft=false`)
    *after* every asset is uploaded (`--clobber`) and SHA-verified — so a
    consumer-visible release with missing assets is precluded by design, and a
    re-upload is idempotent. On a generic upload failure it auto-runs
    `gh release delete --cleanup-tag` (removing draft + tag); on an
    `AssetVerificationError` it **deliberately preserves** the draft + tag for
    forensic triage. (See `tasks/github.py`.)
  The only state genuinely needing manual recovery is a hard process-kill
  *between* the push and the cleanup handler: a tag with a leftover draft.
  Recovery is to delete that tag + draft and re-cut. **Do not blindly delete a
  preserved draft+tag — it may be the intentional `AssetVerificationError`
  triage artifact.** (This window pre-exists this change; recorded for
  completeness.)
- If `queue: max` is rejected by the platform, Phase 2 is reverted
  independently of Phases 1 and 3 (see Phase 2 Overview contingency).

## References

- Root-cause analysis:
  `meta/research/issues/2026-06-14-release-concurrency-group-blocks-prereleases.md`
- Current workflow: `.github/workflows/main.yml:184-291`
- Release task split: `tasks/release.py:23-101` (`_publish` at `:23-29`;
  `release_prepare` `git pull` + finalise bump at `:56-70`)
- GitHub Actions `concurrency.queue` (GA 2026-05-07):
  - Changelog: https://github.blog/changelog/2026-05-07-github-actions-concurrency-groups-now-allow-larger-queues/
  - Syntax reference: https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax
  - Concurrency how-to: https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency
- Community confirmation that a Waiting approval-gated job holds its concurrency
  group: GitHub community Discussion #17401 (cited in the root-cause analysis)
