---
type: plan-review
id: "2026-06-14-release-concurrency-approval-gate-split-review-1"
title: "Plan Review: Release Concurrency Approval-Gate Split"
date: "2026-06-14T12:31:42+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
target: "plan:2026-06-14-release-concurrency-approval-gate-split"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [correctness, architecture, safety, security, compatibility, test-coverage, code-quality]
review_number: 1
review_pass: 3
tags: [ci, github-actions, release, concurrency]
last_updated: "2026-06-14T13:50:26+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Release Concurrency Approval-Gate Split

**Verdict:** REVISE

The plan's core reasoning is sound and well-grounded: it correctly diagnoses
that GitHub acquires a concurrency group at *queue time*, so hoisting the
`environment: release` gate onto a lock-free `approve-release` job genuinely
releases the `accelerator-release` lock during the unbounded approval wait —
and the verified claim that `release_prepare` re-pulls and re-bumps at execution
time means a late-approved release still finalises against current HEAD. What
holds it back from approval is a cluster of major concerns that converge from
several lenses: Phase 2 rests entirely on an **unverified external feature**
(`queue: max`) whose own source research flagged it as unconfirmed; the new
lock-free approval stage introduces a **multiple-approval / duplicate-release**
hazard the plan never analyses; moving `environment:` off the work job has
**security side-effects** (OIDC `sub`-claim scoping and a `needs`-edge re-run
bypass) that the env-secrets check alone doesn't cover; and the **verification
strategy is weaker than the irreversible-publish stakes warrant**, with no
standing regression guard.

### Cross-Cutting Themes

- **`queue: max` is an unverified external dependency** (flagged by:
  correctness, safety, compatibility) — All of Phase 2's value rests on
  `queue: max` being GA, schema-valid, and FIFO-ordered "by time entered the
  wait state". The root-cause research this plan derives from explicitly said
  to *verify before relying on it*, yet the plan upgrades that to "real, GA,
  and documented" with no reproducible check. If the claim is wrong the
  workflow either hard-fails (blocking **all** releases and prereleases — the
  exact outcome the plan exists to prevent) or silently no-ops (leaving the
  eviction risk live while the team believes it fixed).

- **No standing guard protects the new invariant** (flagged by: architecture,
  security, test-coverage) — Correctness now depends on a multi-job invariant
  (`environment:` only on `approve-release`; lock only on non-gated jobs;
  `queue: max` on both group members) that no CI check enforces. The plan's
  own greps are run once at implementation time; a routine future workflow edit
  could silently reintroduce the blocking bug, the eviction bug, or an
  ungated privileged job, surfacing only as another production stall.

- **Accumulating, lock-free approvals can cut multiple stable releases**
  (flagged by: correctness, safety) — Because `approve-release` deliberately
  has no concurrency group, every push spawns an independent gate that sits
  Waiting in parallel. The plan only analyses the single-pending-release case;
  approving two stacked gates queues two `release` jobs, the second
  re-finalising against the post-stable HEAD the first left behind — plausibly
  producing a duplicate or second-distinct stable tag/release with only manual
  recovery.

- **Verification is weaker than the stakes warrant** (flagged by:
  test-coverage, safety, compatibility) — The one semantic check (`actionlint`)
  is gated behind "if available" and is pinned/installed nowhere, so it almost
  never runs; the structural greps match comment prose and brittle `-A3`
  windows; and every behavioural guarantee is observable only in the live
  production pipeline against already-pushed tags and releases.

### Tradeoff Analysis

- **Releasing the lock (correctness/architecture/safety) vs. protecting the
  privileged work (security)**: The plan's entire premise is moving the
  approval gate *off* the job that pushes, tags, publishes, and mints OIDC
  attestation tokens — which is exactly the job the security lens argues the
  human gate should stay *on*. Both pressures are real. The resolution is not
  to abandon the split but to (a) confirm the release OIDC `sub` claim is not
  relied upon as `environment:release`-scoped by any attestation relying-party,
  and (b) decide explicitly whether the `needs`-edge re-run bypass is
  acceptable or needs mitigation. If either check fails, the gate may need to
  stay on (or be re-asserted at) the work job, and Phase 1's topology revisited.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Correctness / Safety / Compatibility**: `queue: max` is an unverified
  external dependency that Phase 2 entirely depends on
  **Location**: Phase 2 Overview; Key Discoveries (`queue: max`); References
  The plan asserts `queue: max` is "real, GA since 2026-05-07, and documented"
  and FIFO-orders up to 100 pending runs by wait-entry time, but the source
  research flagged this as unconfirmed and no local check validates it. If the
  feature/semantics differ or it is unavailable on the account/GHES tier, the
  workflow either hard-fails (blocking the whole pipeline) or silently no-ops
  (eviction risk remains).

- 🟡 **Correctness / Safety**: Accumulating lock-free approvals can cut
  multiple/duplicate stable releases
  **Location**: Phase 1, Change 1 (Add the `approve-release` job)
  `approve-release` has no concurrency group, so gates accumulate in parallel.
  Approving two queues two `release` jobs; the second re-runs `release_prepare`
  (`git pull` + `version.bump(FINALISE)`) against the post-stable `pre.1` HEAD
  the first left behind, plausibly publishing a second distinct (or conflicting
  duplicate) stable tag/release — irreversible, manual-recovery-only.

- 🟡 **Architecture / Security / Test Coverage**: No standing guard enforces
  the new multi-job invariant
  **Location**: Phase 1 & 2 Success Criteria; Testing Strategy
  Correctness now rests on `environment:` living only on `approve-release`, the
  lock living only on non-gated jobs, and `queue: max` on both group members —
  none enforced by CI. A future edit can silently reintroduce the blocking or
  eviction bug, or add an ungated privileged job, with nothing failing.

- 🟡 **Security**: OIDC token `sub` claim loses its `environment:release`
  segment, potentially breaking SLSA trust scoping
  **Location**: Phase 1, Change 2 (Rewire the `release` job)
  GitHub injects `environment:<name>` into the OIDC `sub` claim only for jobs
  declaring `environment:`. The privileged `release` job mints OIDC tokens for
  `attest-build-provenance`; after the split its `sub` silently changes from an
  `environment:release`-scoped subject to a ref-scoped one. Any relying-party
  pinning `environment:release` may reject the token, or the human-approval
  trust boundary on the minted subject is lost.

- 🟡 **Security**: `needs: approve-release` is a weaker gate than
  `environment:` and may be bypassable on re-run
  **Location**: Phase 1, Change 2; Migration Notes
  An `environment` gate is re-evaluated each time the gated job is dispatched;
  a `needs:` edge asserts the upstream reached success once. Re-running the
  `release` job from the Actions UI may re-dispatch the privileged
  push/publish/attest steps without GitHub re-presenting the approval prompt.

- 🟡 **Compatibility**: Local automated checks cannot detect an unsupported or
  silently-ignored `queue` key
  **Location**: Phase 2 Success Criteria (Automated Verification)
  `yaml.safe_load` accepts any key, the greps only confirm the literal text is
  present, and actionlint historically does not validate `concurrency`
  sub-keys (and is conditional here). So an unsupported/renamed `queue` option
  passes every local gate; the first signal is a failed real release run.

- 🟡 **Test Coverage**: The `actionlint` gate is conditional and will almost
  never run
  **Location**: Phase 1 & 2 Success Criteria (Automated Verification)
  Both phases gate the only semantic check behind "if `actionlint` is
  available". It is not pinned in `mise.toml`, not in any task, and not in CI,
  so it is silently skipped everywhere — leaving YAML parse (syntax-only) and
  greps (text-only) as the sole automated coverage.

- 🟡 **Test Coverage**: Structural greps are imprecise and can give false
  confidence
  **Location**: Phase 1 & 2 Success Criteria; Testing Strategy
  `grep "environment: release"` also matches the explanatory comments the plan
  itself adds, so the "single hit" assertion can be satisfied by prose or
  broken by a comment; `grep -A3` after the group line is brittle to key
  reordering. The checks don't reliably prove the structural invariant.

- 🟡 **Architecture**: The `release` job conflates the stable release and the
  post-stable prerelease re-cut under one lock and one gate
  **Location**: Phase 1, Change 2; What We're NOT Doing
  The post-stable prerelease re-cut (`main.yml:278-291`) stays inside the
  `release` job, so it is now gated behind the human approval meant for the
  stable release and shares its single lock acquisition. This is an explicitly
  *accepted* decision; the actionable gap is that the cohesion cost / rationale
  ("must run after the stable push within the same serialised window") is not
  documented at the job.

#### Minor

- 🔵 **Correctness**: Post-stable re-cut's interaction with FIFO ordering of
  queued prereleases is asserted-correct rather than traced
  **Location**: What We're NOT Doing; Phase 2 Overview
  Version monotonicity through the (stable cut → post-stable pre → dequeued
  prereleases) chain holds, but the plan never demonstrates it.

- 🔵 **Correctness**: Gate rejection/timeout state transition is undocumented
  **Location**: Phase 1, Change 2
  The happy path is described; the plan doesn't state that on rejection/timeout
  the `release` job is skipped and never acquires the lock (it is, but this
  closes the error branch of the state machine).

- 🔵 **Architecture / Compatibility**: `approve-release` uses `ubuntu-latest`
  while the rest of the release chain uses `macos-latest`
  **Location**: Phase 1, Change 1
  Harmless and cheaper for a no-op, but an unexplained divergence in an
  otherwise uniform pipeline; a one-line rationale comment would resolve it.

- 🔵 **Architecture / Code Quality**: Phase-1-alone leaves a live eviction
  window; "either phase mergeable alone" understates the correctness coupling
  **Location**: Implementation Approach; Phase 2 Overview
  Phase 1 is a strict improvement but not the complete fix — shipping it
  without Phase 2 trades blocking for the (still-silent) eviction failure.

- 🔵 **Architecture**: FIFO queue ordering is governed by approval time, not
  push time — an emergent behaviour left uncharacterised
  **Location**: Phase 2 Overview; Desired End State
  Safe for correctness (late-binding finalise), but the ordering semantics
  under contention could surprise an operator.

- 🔵 **Security**: The env-scoped-secrets check is necessary but not
  sufficient — it misses deployment-branch and custom protection rules
  **Location**: Current State Analysis; Phase 1 Manual Verification
  A `release` environment can also enforce deployment-branch restrictions and
  custom rules, which now attach to the no-op job rather than the work job.

- 🔵 **Compatibility**: Asymmetric `queue: max` across the two group members is
  an unverified contract
  **Location**: Phase 2, Changes 1 & 2
  If one block carries `queue: max` and the other doesn't (partial application
  or future edit), the group's behaviour is undefined; the grep criteria check
  each block independently, not for symmetry.

- 🔵 **Test Coverage**: The FIFO no-eviction case has no reproduction recipe
  **Location**: Testing Strategy (Observational), steps 3-4
  The most important Phase 2 behaviour depends on a hard-to-construct timing
  race with no recipe, so it may default to "trust the docs".

- 🔵 **Test Coverage**: The secret-loss risk has no run-time positive assertion
  **Location**: Phase 1 Manual Verification
  A successful end-to-end release after the change is the real proof no
  env-scoped secret was lost; the plan should bind that observation to the risk.

#### Suggestions

- 🔵 **Code Quality**: The approval step is a bare `echo` whose intent lives
  only in the job comment
  **Location**: Phase 1, Change 1 (line 167)
  A self-documenting step name (e.g. "Approval gate (no-op; the gate is the
  environment)") reads better than a free-text echo.

- 🔵 **Code Quality**: The two `accelerator-release` concurrency blocks become
  near-identical duplicates that must stay in sync with no automated check
  **Location**: Phase 2, Changes 1 & 2
  Cross-reference each block's sibling in its comment and assert equality (not
  just per-block presence) in the success criteria.

- 🔵 **Code Quality**: The prerelease block's rationale is split between an
  above-block comment and a new interleaved `queue: max` note
  **Location**: Phase 2, Change 1
  Consolidate into one coherent comment block, matching the existing
  above-block placement.

- 🔵 **Code Quality**: If Phase 1 may dwell before Phase 2, the intermediate
  state is silently incomplete
  **Location**: Implementation Approach
  A one-line marker in Phase 1's release comment noting the eviction hardening
  is a follow-up would self-document the transient state.

### Strengths

- ✅ Correctly identifies the root cause — concurrency is acquired at queue
  time, not approval time — and the split genuinely releases the lock during
  the wait; the central logic is valid (correctness, architecture).
- ✅ The decoupling is at the right seam: the gate (unbounded human-time wait)
  and the lock (machine-time mutual exclusion) have incompatible lifetimes and
  shouldn't share a queue-time resource (architecture).
- ✅ Verified that `release_prepare` does `git pull` + `version.bump(FINALISE)`
  at execution time, backstopped by an `--atomic` push, so a late-approved
  release finalises against current HEAD rather than a stale snapshot
  (correctness, safety).
- ✅ Correctly retains `cancel-in-progress: false`, which both protects an
  in-flight push and is required for `queue: max` validity, and identifies the
  `queue: max` + `cancel-in-progress: true` validation error pre-emptively
  (correctness, compatibility, safety).
- ✅ Clean two-phase decomposition addressing two distinct failure modes
  (blocking vs. eviction), each leaving a valid workflow (architecture, code
  quality).
- ✅ Option B (separate `workflow_dispatch` workflow) is rejected with sound
  reasoning; pinned action versions are left untouched, adding no dependency
  risk (architecture, compatibility).
- ✅ The Phase 1 comments are genuinely load-bearing — they encode the
  non-obvious pitfall exactly where a future editor would be tempted to
  re-merge the gate and lock (code quality).
- ✅ Honestly scopes out unit testing as inapplicable and explains why, rather
  than inventing a hollow test (test-coverage).

### Recommended Changes

1. **Gate Phase 2 on a real `queue: max` confirmation, with an explicit
   contingency** (addresses: "`queue: max` is an unverified external
   dependency", "Local checks cannot detect an unsupported `queue` key")
   Before relying on it, run a throwaway workflow on this exact repo/account
   with only `concurrency: { group: x, cancel-in-progress: false, queue: max }`
   and confirm GitHub *accepts* it and that queueing observably engages. State
   in Migration Notes that if rejected, Phase 2 is reverted independently
   (Phase 1 already removes the blocking) and the residual eviction risk is
   accepted/tracked. Confirm the target is github.com, not GHES.

2. **Analyse the multiple/stacked-approval case and add a duplicate-release
   guard or explicit semantics** (addresses: "Accumulating lock-free approvals
   can cut multiple/duplicate stable releases")
   Trace what happens when two `approve-release` gates are approved: confirm a
   no-op `FINALISE` / duplicate-tag push fails safe, or add a guard so a
   `release` job whose target version is already published no-ops. Add a Manual
   Verification step exercising two stacked approvals.

3. **Verify the OIDC `sub`-claim and re-run implications before merge**
   (addresses: "OIDC `sub` claim loses `environment:release`", "`needs` is a
   weaker gate than `environment:`")
   Inspect the `sub` / `job_workflow_ref` claims the `release` job mints
   pre- and post-change and confirm no attestation relying-party pins
   `environment:release`. Add a Manual Verification step that re-runs the
   `release` job alone and records whether approval is re-requested; decide
   explicitly whether the re-run bypass is acceptable or needs mitigation.

4. **Add a cheap standing regression guard for the invariant** (addresses: "No
   standing guard enforces the new multi-job invariant", "Structural greps are
   imprecise")
   Replace the one-shot greps with a small parser-based assertion (e.g. a
   pytest under `tests/unit/tasks/` or an addition to a shell suite) that loads
   `main.yml` and asserts: `release` has no `environment`, `approve-release`'s
   environment is `release`, `release.needs == ['approve-release']`, and both
   `concurrency` dicts carry `queue == 'max'` and `cancel-in-progress is
   False`. This converts a one-time check into permanent protection.

5. **Make `actionlint` deterministic** (addresses: "The `actionlint` gate is
   conditional and will almost never run")
   Pin actionlint in `mise.toml` and run it unconditionally, or state
   explicitly that semantic validation rests on GitHub's first-push acceptance
   and treat that as a hard gate.

6. **Expand the rollback/blast-radius and document accepted tradeoffs**
   (addresses: "`release` job conflates two responsibilities", safety
   rollback minor, FIFO-ordering minor)
   Distinguish reverting the workflow (fixes future runs) from cleaning up
   artifacts a faulty run already published; document *why* the post-stable
   re-cut stays inside the locked+gated job; and note that queue order is
   governed by approval time, made safe by late-binding finalise.

7. **Resolve the smaller documentation/consistency gaps** (addresses: the
   minor and suggestion findings)
   Add the `ubuntu-latest` rationale comment, the gate-rejection state
   sentence, the FIFO reproduction recipe, the concurrency-block sync
   cross-references, and the Phase-1-incomplete marker.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Correctness

**Summary**: The core concurrency reasoning is sound: moving the approval gate
to a no-op `approve-release` job with no concurrency group genuinely releases
the `accelerator-release` lock during the unbounded approval wait, since GitHub
acquires the group at queue time and the `release` work job only queues
post-approval. The claim that `release_prepare` finalises against current HEAD
(via `git pull` + `version.bump(FINALISE)`, backstopped by the `--atomic` push)
is verified and correct. The main correctness gaps are unaddressed
multi-approval / queue-ordering interactions and reliance on an
externally-asserted `queue: max` semantic that the source research itself
flagged as unverified.

**Strengths**:
- Correctly identifies that concurrency is acquired at queue time, not approval
  time, so splitting the gate onto a group-less job does release the lock.
- The claim that `release_prepare` does `git pull` + `version.bump(FINALISE)` at
  execution time (so a late approval finalises against current HEAD) is
  accurate; `git.push` is `--atomic`, so a stale-HEAD push is rejected.
- Correctly preserves `cancel-in-progress: false`, required for `queue: max`.
- Keeps the genuinely-exclusive window (`_publish`) inside the lock.

**Findings**:
- **major / medium** — *Independent approve-release jobs accumulate; multiple
  approvals cut multiple stable releases* (Phase 1: Add the approve-release
  job). `approve-release` has no concurrency group, so every push spawns an
  independent Waiting gate. Approving two queues two `release` jobs; the second
  re-runs `release_prepare` against the post-stable `pre.1` HEAD, producing a
  second distinct stable release. A reviewer clearing a backlog of stale gates
  silently publishes several stable versions. Either document "one approval ==
  one release" or guard against an already-finalised target; cover the
  multiple-pending case in Manual Verification.
- **major / medium** — *`queue: max` FIFO semantics asserted as verified but
  source research flagged them as unverified* (Phase 2 Overview / Key
  Discoveries). The research said "verify before relying on it"; the plan
  upgrades to "GA and documented" with no reproducible check. If ordering
  semantics differ, the no-eviction guarantee fails. Treat as a hypothesis to
  confirm in CI; make the contended-case Manual Verification a hard gate.
- **minor / medium** — *Post-stable re-cut inside the release lock interacts
  with FIFO ordering* (What We're NOT Doing / Phase 2). Monotonicity holds
  (dequeued prerelease pulls `pre.1` → `pre.2`) but is asserted, not traced.
  Add a one-line trace.
- **minor / high** — *`release` retains `if: github.event_name == 'push'`
  redundantly but harmlessly* (Phase 1, Change 2). Rejection/timeout path
  (release skipped, lock never acquired) is undocumented; add a sentence.

### Architecture

**Summary**: The plan applies a sound separation-of-concerns seam: the approval
gate (a human-time, unbounded-wait concern) is hoisted onto a dedicated no-op
job that holds no lock, while the concurrency lock (a machine-time
mutual-exclusion concern) stays on the work job where it covers only the active
push window. This is the correct seam. The two-phase, independently-mergeable
structure is well-judged and the rejected Option B tradeoff is reasoned
correctly; the main residual risk is the now-implicit invariant that the gate
and lock must never recombine, which the plan documents in comments but cannot
enforce.

**Strengths**:
- Splits gate and lock at exactly the right seam — distinct, incompatible
  lifetimes; the lock now scopes only the `_publish` window.
- The two-phase decomposition is genuinely orthogonal (blocking vs. eviction).
- Option B rejected with sound reasoning (avoids fragmenting the pipeline).
- Evolutionary fitness good: late-binding finalise semantics unchanged.
- Failure modes explicitly enumerated and mapped to research hypotheses;
  `cancel-in-progress: false` correctly retained as load-bearing.

**Findings**:
- **major / high** — *Gate/lock split creates an implicit multi-job invariant
  with no enforcement* (Phase 1 Changes 1 & 2; Phase 2 Changes 1 & 2). No
  actionlint rule or repo check detects a future edit recombining gate and
  lock. Promote the invariant from prose to an executable grep/parser assertion
  in CI.
- **major / high** — *Release job conflates two responsibilities (stable
  release + post-stable prerelease) under one lock and one gate* (Phase 1
  Change 2; What We're NOT Doing). The post-stable re-cut is now gated behind
  the stable-release approval and shares its lock — a cohesion cost the plan
  decides but doesn't justify. Document *why* the re-cut belongs inside the
  approved-and-locked job.
- **minor / medium** — *`approve-release` uses a different runner family*
  (`ubuntu-latest` vs `macos-latest`). Defensible for a no-op but undocumented;
  add a one-line rationale.
- **minor / medium** — *Phase-1-alone leaves a known eviction window;
  merge-order independence understates the coupling of correctness*
  (Implementation Approach; Phase 2). State that Phase 1 is a strict
  improvement but not the complete fix.
- **minor / low** — *FIFO queue ordering interacts with the approval gate in an
  unexamined way* (Phase 2 Overview; Desired End State). Queue position is
  approval-time-driven, not push-time; safe but uncharacterised. Add a sentence.

### Safety

**Summary**: The plan correctly identifies that the concurrency group is the
sole mutual-exclusion mechanism protecting the irreversible publish path
(commit → tag → atomic push → create release → upload binaries), and it
preserves the lock on the work job so the actual publish window stays
serialised. The core safety risk is not the split itself but the new unguarded
approval stage combined with `queue: max`: it opens a window where two distinct
approved stable releases can FIFO-queue and finalise in sequence, and it relies
on an unverifiable post-cutoff GitHub feature whose misbehaviour would silently
degrade the very protection it claims to add. Because all behaviour is
observable only in production CI against irreversible published tags/releases,
the verification plan is the weakest safety link.

**Strengths**:
- Correctly preserves `cancel-in-progress: false`, keeping the irreversible
  commit/tag/push window atomic.
- Keeps the lock on the work job, covering exactly the `_publish` window.
- Recognises late-binding finalise (`git pull` + `FINALISE` at execution time).
- Relies on `git push --atomic`, so a rejected branch push never orphans a tag.
- Documents rollback as a VCS revert with no external workflow state to undo.

**Findings**:
- **major / medium** — *Lock-free approval stage allows double-approval to
  produce duplicate/conflicting stable releases* (Phase 1, Section 1). A second
  approved release finalises a version the first already published;
  `finalize_version()` on an already-stable version plus a duplicate tag/release
  can collide. Recovery is manual tag/release deletion, not a clean revert.
  Reason about the two-approved case and add a Manual Verification step.
- **major / medium** — *Phase 2 rests on an unverifiable feature whose rejection
  blocks the whole pipeline* (Phase 2 / Key Discoveries). A wrong `queue: max`
  assumption converts a hardening change into a full release-pipeline outage.
  Gate Phase 2 on actionlint + a dry-run acceptance; treat a validation error
  as a fail-safe revert trigger, not a passive observation.
- **major / high** — *Every behavioural guarantee is verified only by
  observation in production CI against irreversible actions* (Manual
  Verification / Testing Strategy). A latent flaw is first discovered after it
  has mis-published or mis-ordered a real release. Add a low-risk validation
  path (a non-`main` branch copy with publish steps stubbed, or a scratch repo).
- **minor / medium** — *Env-scoped-secrets assumption fails late if wrong*
  (Phase 1, Section 2). Treat the secrets confirmation as a hard prerequisite
  before merge, with the fallback (keep the gate on the work job) stated.
- **minor / medium** — *Rollback story understates blast radius* (Migration
  Notes / Desired End State). A workflow revert doesn't undo already-pushed
  tags/releases; distinguish reverting the workflow from cleaning up artifacts.

### Security

**Summary**: The plan splits a human approval gate off the privileged release
work job onto a separate no-op `approve-release` job, leaving the job that
actually pushes commits/tags, creates GitHub releases, and mints OIDC tokens
for SLSA attestation gated only by `needs: approve-release`. This is a genuine
weakening of the supply-chain approval boundary: the `needs:` graph edge is a
weaker control than `environment:`, and detaching `environment:` from the work
job silently strips the `environment` segment from the OIDC `sub` claim and
removes environment-scoped protections from the job doing the sensitive work.
The plan's Manual Verification checks for env-scoped secrets but misses the OIDC
trust-policy and `needs`-edge bypass implications.

**Strengths**:
- Retains `cancel-in-progress: false`, preserving mutual exclusion around the
  tampering-relevant push.
- Keeps the work job's `id-token`/`contents`/`attestations: write` permissions
  unchanged — no silent permission broadening.
- The Manual Verification does ask about env-scoped secrets, showing awareness
  the environment carries more than the approval prompt.

**Findings**:
- **major / high** — *OIDC token `sub` claim loses its `environment:release`
  segment, breaking SLSA trust scoping* (Phase 1, Change 2). The `release` job
  mints OIDC tokens for `attest-build-provenance`; without `environment:` its
  `sub` changes from `environment:release`-scoped to ref-scoped. Relying-parties
  pinning `environment:release` may reject it, or the approval-gated trust
  boundary on the subject is lost. Inspect the minted `sub`/`job_workflow_ref`
  pre/post-change.
- **major / medium** — *`needs: approve-release` is a weaker gate than
  `environment:`; the work job can run via re-run/skip edge cases without
  re-approval* (Phase 1, Change 2; Migration Notes). Re-running the `release`
  job from the UI may re-dispatch privileged steps without re-presenting the
  approval. Keep the approval boundary on the sensitive job, or document/accept
  the bypass; add a re-run Manual Verification step.
- **minor / medium** — *Env-scoped-secrets check is necessary but not
  sufficient; misses branch policies and deployment-branch rules* (Current
  State; Phase 1 Manual Verification). Enumerate ALL `release` environment
  protection rules, not just secrets.
- **minor / high** — *No automated guard prevents a future edit from
  re-coupling the gate or leaving the work job ungated* (Phase 1 Success
  Criteria; Testing Strategy). Add a CI assertion that every privileged push/
  publish/attest step's transitive `needs` includes a `environment: release`
  job.

### Compatibility

**Summary**: The plan's core hardening depends on a single, recently-introduced
GitHub Actions platform feature (`concurrency.queue: max`, claimed GA
2026-05-07) whose availability and validation behaviour the plan asserts but
does not independently verify or guard against. The pinned action versions
(checkout@v5, mise-action@v4.1.0, attest-build-provenance@v2) are unchanged and
carry no compatibility risk, and Phase 1's job-split is built only from
long-stable GHA primitives. The main contract risk is concentrated in Phase 2.

**Strengths**:
- Phase 1 relies only on long-established GHA primitives, independent of
  `queue: max`.
- The two phases are independently mergeable, isolating the contract risk.
- Correctly identifies the `queue: max` + `cancel-in-progress: true` validation
  incompatibility and confirms the blocks use `false`.
- Pinned third-party action versions left untouched.
- Manual verification checks that GitHub accepts the workflow on next push.

**Findings**:
- **major / medium** — *`queue: max` GA rests on a single recent changelog claim
  with no independent confirmation* (Key Discoveries; References). If the
  feature isn't live / rolled back / not in the published schema, Phase 2
  hard-fails (blocking all releases) or silently no-ops. Run a throwaway
  workflow on this exact repo/account to confirm acceptance and behaviour first.
- **major / high** — *Automated checks cannot detect an unsupported `queue`
  key* (Phase 2 Success Criteria; Testing Strategy). `yaml.safe_load` accepts
  any key; greps confirm only text; actionlint is conditional and historically
  lenient on `concurrency` sub-keys. Pin a validating actionlint or add a schema
  check; treat first-push acceptance as a hard blocker.
- **major / medium** — *No contingency if `queue: max` is rejected or
  unavailable on the account/GHES tier* (Phase 2 Overview; Migration Notes). A
  workflow-level validation error refuses the entire run. Document an explicit
  revert-Phase-2-independently contingency; confirm target is github.com.
- **minor / high** — *`approve-release` uses `ubuntu-latest` while every release
  job uses `macos-latest`* (Phase 1, Change 1). Harmless; add a comment or use
  `macos-latest` for consistency.
- **minor / medium** — *Asymmetric `queue: max` during the edit window or
  partial application* (Phase 2). A mixed-config group has undefined behaviour;
  assert the count of `queue: max` equals the count of `group:
  accelerator-release`; treat Phase 2 as one atomic edit.

### Test Coverage

**Summary**: The plan correctly recognises that GHA concurrency behaviour has no
unit-testable surface and proposes a layered static-plus-observational
verification strategy, which is the right shape. However, the verification is
weaker than the stakes warrant: the actionlint check is conditionally skipped
(not pinned or installed anywhere), the structural greps are imprecise enough to
give false confidence, and there is no durable regression guard. The
observational CI plan also conflates a deterministic check (the OLD bug is gone)
with a genuinely hard-to-reproduce race (FIFO no-eviction).

**Strengths**:
- Honestly scopes out unit testing and explains why.
- Layered strategy matching the inverted pyramid workflow changes force.
- Phase 1 observational steps are concrete and target the reported bug; step 3
  verifies finalise-against-HEAD, not just job start.
- Calls out the invalid `queue: max` + `cancel-in-progress: true` combination.

**Findings**:
- **major / high** — *actionlint gate is conditional and will almost never run*
  (Phase 1 & 2 Automated Verification). Not pinned, not in any task, not in CI.
  Pin it and run unconditionally, or state the check is expected to be skipped.
- **major / high** — *Structural greps are imprecise and can give false
  confidence* (Success Criteria; Testing Strategy). `grep "environment:
  release"` matches the plan's own comments; `-A3` is brittle. Use a parser-based
  assertion on YAML semantics.
- **major / medium** — *No durable regression guard against reintroducing the
  bug* (Testing Strategy; What We're NOT Doing). All verification is one-shot.
  Add a tiny standing pytest/shell assertion parsing `main.yml`.
- **minor / medium** — *FIFO no-eviction verification is not reproducibly
  specified* (Observational steps 3-4). Add a concrete recipe (slow the
  prerelease finalise, fire two pushes, assert none shows "Canceled").
- **minor / medium** — *Negative coverage for the secret-loss risk is
  assertion-light* (Phase 1 Manual Verification). Bind observational step 3 (a
  successful end-to-end release) as the proof no env-scoped secret was lost.

### Code Quality

**Summary**: For a config-only change to one workflow file, the plan is highly
maintainable: the load-bearing comments are accurate, specific, and encode the
non-obvious 'concurrency acquired at queue time' pitfall directly where future
editors would be tempted to re-merge the gate and lock. The two-phase split is
clean and each phase leaves a valid workflow. The main maintainability risks are
the now-duplicated three-line concurrency blocks (with divergent comments) that
can drift, and the no-op approval step's reliance on a free-text echo.

**Strengths**:
- Phase 1 comments are genuinely load-bearing and well-placed, defending against
  the most likely future regression (re-merging environment and concurrency).
- Explicitly flags that both concurrency blocks must declare `queue: max`.
- Clean, independently-mergeable two-phase split, each leaving a valid workflow.
- Phase 2 comment ties the `queue: max` / `cancel-in-progress: false` constraint
  to a concrete consequence.

**Findings**:
- **minor / medium** — *Approval-gate step is a free-text `echo`* (Phase 1,
  Section 1, line 167). Make the step self-documenting (descriptive name +
  `run: 'true'`, or intent-phrased echo).
- **minor / high** — *The two concurrency blocks become near-identical
  duplicates that must stay in sync with no automated check* (Phase 2, Sections
  1 & 2). Cross-reference siblings in comments; assert equality in success
  criteria.
- **minor / medium** — *Prerelease concurrency rationale split across two
  comment fragments* (Phase 2, Section 1). Consolidate into one above-block
  comment.
- **minor / low** — *Transient Phase-1-only state is silently incomplete*
  (Implementation Approach). Add a one-line follow-up marker if phases may dwell.

## Re-Review (Pass 2) — 2026-06-14T12:51:57+00:00

**Verdict:** REVISE

The revisions land well: the prior pass's nine majors are substantially
resolved or mitigated. The fix is much stronger — but the edits introduced a
second, narrower cluster of majors, mostly *refinements of the new mechanisms*
rather than fresh fundamental flaws. Three lenses (security, test-coverage,
architecture) independently converged on the same gap in the headline new
deliverable: the Phase 3 regression test asserts **hard-coded job names** rather
than the underlying invariant, so the guard has a blind spot for exactly the
topology/rename edits most likely to silently re-couple the gate and lock.
Correctness and safety converged on a second: the new approval-group comment
asserts **two mutually contradictory single-slot semantics**. These are
focused, addressable refinements, not a redesign.

### Previously Identified Issues

- 🟡 **Correctness/Safety/Compatibility**: `queue: max` unverified external
  dependency — **Partially resolved.** The GA-as-fact assertion is gone and a
  pre-merge acceptance check + independent-revert contingency are added.
  Compatibility now notes the acceptance check proves *key acceptance* but not
  the *FIFO/eviction behaviour* actually relied on (see New Issues).
- 🟡 **Correctness/Safety**: Accumulating approvals → duplicate releases —
  **Resolved in approach, but the implementation introduced new semantic
  ambiguity.** Serialising via a dedicated approval group is the right move; the
  comment describing its behaviour is self-contradictory (see New Issues).
- 🟡 **Architecture/Security/Test Coverage**: No standing regression guard —
  **Resolved (guard now exists) but too narrow.** The Phase 3 parser test keys
  off job names, not the invariant (see New Issues — convergent across 3
  lenses).
- 🟡 **Security**: OIDC `sub`-claim scoping — **Resolved.** Now a blocking
  pre-merge prerequisite; cited as a strength. Residual: no guard against future
  drift (minor).
- 🟡 **Security**: `needs`-edge re-run bypass — **Still present (re-raised
  major).** Accepted+documented per decision, but security argues re-run rights
  are broader than approval rights, and the parser test actively *forbids* the
  defence-in-depth of also gating the work job (`assert "environment" not in
  release`).
- 🟡 **Compatibility**: Local checks can't detect an unsupported `queue` key —
  **Mostly resolved** via the throwaway acceptance check + parser test; residual
  noted (actionlint doesn't validate concurrency sub-keys — minor).
- 🟡 **Test Coverage**: actionlint conditional/never runs — **Resolved**
  (pinned + unconditional via `lint:workflows`). New residual: mise backend
  resolution (see New Issues).
- 🟡 **Test Coverage**: Structural greps imprecise — **Resolved.** Replaced with
  parser assertions on the parsed document; cited as a strength.
- 🟡 **Architecture**: Release-job two-responsibility cohesion — **Resolved.**
  The version-monotonicity rationale is now documented inline; cited as a
  strength.

### New Issues Introduced

- 🟡 **Correctness (high)**: The `approve-release` comment asserts both "a newer
  push supersedes an older un-acted pending approval" *and* "cancel-in-progress:
  false protects an approval a human is mid-decision on" — these cannot both
  hold under single-slot + `cancel-in-progress: false`. Pick the behaviour
  GitHub actually exhibits (confirm in CI) and rewrite the comment to one.
- 🟡 **Security/Test Coverage/Architecture (high)**: The Phase 3 regression test
  asserts specific job names, not the invariant. It should iterate all jobs:
  *no `environment`-bearing job carries the `accelerator-release` lock; exactly
  one job carries `environment: release`; every release-privileged job is
  transitively gated by it.* Drop the hard `"environment" not in release`
  assertion so defence-in-depth (also gating the work job) remains possible.
- 🟡 **Compatibility (high)**: The throwaway-workflow check verifies `queue:
  max` *acceptance* but not the FIFO/no-eviction *behaviour*. Either make the
  throwaway actually contend (3 runs + sleep, assert none "Canceled"), or scope
  the pre-merge check to acceptance-only and lean on the post-merge contended
  observation — and say which.
- 🟡 **Compatibility (medium)**: `actionlint` is not a registered mise core tool
  (unlike uv/python/gh/…); a bare `actionlint = "x.y.z"` may not resolve.
  Specify the backend (`aqua:rhysd/actionlint` or `ubi:rhysd/actionlint`) and
  verify on a CI runner, since the install step is shared across all check jobs.
- 🟡 **Safety (medium)**: Partial-publish state — `_publish` runs
  push(atomic) → create_release → upload, so an interruption after the push
  leaves a tag with no release / incomplete assets. The atomic guard doesn't
  cover this and the recovery notes stop at tag+release deletion. (Pre-existing;
  worth a recovery line, arguably out of scope.)
- 🔵 **Safety (medium)**: The approval group silently supersedes a pending
  approval. Fails safe (no double-cut) but invisibly; add a CI-observation
  criterion that the superseded gate shows a clear cancelled/superseded state.
- 🔵 **Test Coverage/Correctness (medium)**: `needs == "approve-release"`
  equality is fragile to the scalar-vs-list YAML form; normalise before
  comparing. Also add `approve-release.needs == "prerelease"` to the asserted
  set, and consider an encoded negative test (not just a one-shot manual
  mutation check).
- 🔵 **Safety (high)**: The FIFO reproduction recipe injects a temporary
  `sleep`/slow step into the release-critical workflow with no removal step —
  state it must be on a throwaway branch only and never merged.
- 🔵 **Code Quality (minor)**: The "keep in sync" rationale is now duplicated and
  worded inconsistently across the two `accelerator-release` blocks; make the
  cross-reference comments verbatim-identical. Also flag adding `workflows` to
  `tasks/lint/__init__.py`'s `__all__`, and name the precise `check` depends-edge
  for `lint:workflows`.
- 🔵 **Security (minor)**: The approval is decoupled from the artifact (no-op
  echo, no version/sha). Echo the resolved version + HEAD sha into the job
  summary so the deployment record anchors the human decision to a concrete
  subject.

### Assessment

The plan is materially better and the core design is sound and well-defended.
It is **not yet ready to approve**: a focused third round should (1) rewrite the
Phase 3 test to assert the invariant rather than job names — the single
highest-value change, flagged by three lenses; (2) reconcile the contradictory
approval-group comment to one CI-confirmed behaviour; (3) clarify that the
throwaway check is acceptance-only (or make it contend); (4) pin actionlint with
an explicit backend; and (5) sweep the smaller refinements (needs normalisation,
sleep-step warning, comment dedup, supersede visibility). None require revisiting
the chosen approach.

---

## Re-Review (Pass 3) — 2026-06-14T13:50:26+00:00

**Verdict:** APPROVE (after applying the pass-3 fixes below)

The pass-2 findings are all resolved. This final verification pass confirmed
that — but caught **three high-confidence new issues, two of them defects
introduced by the pass-2 edits themselves**, converging across multiple lenses.
All were addressed in a follow-up edit and the core invariant logic was executed
against good and eight bad workflow shapes to confirm it discriminates. The
chosen approach was never in question at any point in this pass.

### Previously Identified Issues (pass-2 round)

- 🟡 **Approval-group comment contradiction** (correctness/safety) — **Resolved.**
  Now one coherent semantics (active approval protected; later pushes
  wait/supersede in the single pending slot).
- 🟡 **Parser test keyed off job names** (security/test-coverage/architecture) —
  **Resolved**, then refined again this pass (see below): now a name-agnostic
  core invariant + deliberately-named wiring.
- 🟡 **Throwaway check ≠ FIFO behaviour** (compatibility) — **Resolved.** Scoped
  to acceptance-only; behaviour deferred to post-merge observation.
- 🟡 **actionlint backend** (compatibility) — **Resolved**, then tightened to an
  exact pin + CI-runner backend-resolution check this pass.
- 🔵 Sleep-step guard, comment dedup, supersede visibility, `needs`
  normalisation, partial-publish recovery — **Resolved.**

### New Issues Introduced (by pass-2 edits) — and fixed this pass

- 🔴 **Correctness/Security/Test-Coverage (high)**: the "identify the
  release-privileged job *by permissions, not name*" framing was **wrong** —
  `prerelease` and `release` carry byte-identical `permissions`, so the selector
  matched both (and `prerelease` is correctly ungated). **Fixed:** wiring now
  references the named jobs deliberately, with the name-agnostic core invariant
  as the backstop; rationale documented.
- 🔴 **Architecture/Code-Quality (high)**: routing `lint:workflows:check` into
  the **top-level `check`** would *not* run in CI — CI invokes the four
  component `:check` tasks directly, never the aggregate. **Fixed:** folded into
  `build-system:check` (the roll-up CI actually runs); success criterion
  corrected.
- 🔴 **Correctness (high)**: the `concurrency` parser helper treated the YAML
  **string-shorthand** form (`concurrency: groupname`) as no-group, so a
  string-form re-coupling would evade the guard. **Fixed:** helper normalises
  the string form in both the heredoc and the Phase 3 test; a string-shorthand
  mutation added to the negative tests. Verified by execution.
- 🟡 **Safety (high)**: the partial-publish recovery note **misdescribed the
  code** (releases are `--draft` until verified, with auto-cleanup on failure
  and deliberate preserve-on-`AssetVerificationError`). **Fixed** against the
  actual `tasks/github.py` behaviour, with a warning not to delete a preserved
  triage artifact.
- 🔵 Resolved minors: approval group must never carry `queue: max` (asserted +
  verified); `_invariants` pinned to raise `AssertionError` with
  `pytest.raises`; negative test parametrised over all bad shapes; symmetry
  asserts the **member count** (=2); re-run acceptance grounded in an actual
  settings check; a three-group topology table added for readability.

### Assessment

The plan is sound and **ready for implementation**. Every substantive finding
across three passes is resolved, and the central regression-guard logic was
executed against good + eight bad shapes (gate-on-lock in both dict and
string-shorthand form, dropped `queue`, wrong approval group,
`cancel-in-progress: true`, missing approval edge, `queue: max` on the approval
group, dropped lock member) — all correctly accepted/rejected.

Two caveats, both **correctly captured in the plan as blocking manual
prerequisites**, not defects: (1) the genuinely-unverifiable-locally items
(`queue: max` acceptance, the OIDC `sub`-claim, GHA approval/eviction semantics)
must be confirmed during implementation as the plan's Manual Verification
specifies; (2) the pass-3 fixes recorded here were applied directly in response
to this pass and were not themselves put through a further agent re-review —
a final confirming pass is available if belt-and-braces assurance is wanted.

---
*Re-review generated by /accelerator:review-plan*
