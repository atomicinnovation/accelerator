---
type: "work-item-review"
id: "0334-tree-leases-are-not-held-on-the-cold-path-or-honoured-when-review-1"
title: "Work Item Review: Tree leases are not held on the cold path or honoured when liveness is unknown"
date: "2026-10-11T17:30:50+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:0136"
target: "work-item:0334"
work_item_id: "0334"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 5
tags: []
last_updated: "2026-10-12T02:48:29+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Tree leases are not held on the cold path or honoured when liveness is unknown

**Verdict:** REVISE

The work item is precise and well anchored: each defect is tied to exact
functions and file locations, the blast radius is limited to shared
`ACCELERATOR_CACHE_DIR` roots, and the Given/When/Then criteria name observable
outcomes. It needs revising because of two major gaps. The age thresholds (the
14-day claim window, the 1h `AGE_BACKSTOP`, and an unqualified "age") are never
reconciled. The cross-launcher prune criterion omits the shared-root
precondition, so it could pass trivially. The remaining findings concern the
undecided home for the lease primitive, the "two defects" framing, undefined
terms and missing reproduction steps.

### Cross-Cutting Themes

- **Lease primitive relocation is hidden in Technical Notes** (flagged by:
  completeness, scope, dependency) — moving `hold_shared_lease` into a crate
  that both the launcher and `design-cli` can depend on is a structural
  prerequisite with no decided home. It appears in neither Requirements,
  Dependencies nor Open Questions.
- **Summary says "two defects" but scope has three parts** (flagged by:
  clarity, scope) — the warm-path inheritance proof is in-scope test work that
  only Drafting Notes acknowledges.
- **No observable reproduction** (flagged by: completeness, testability) — the
  defects are described by code inspection, with no command sequence or visible
  symptom for a red test to capture.

### Findings

#### Major

- 🟡 **Clarity**: Several age thresholds and "age" subjects are not reconciled
  **Location**: Context / Expected behaviour / Acceptance Criteria / Assumptions
  The 14-day claim window, the 1h `AGE_BACKSTOP` against a generation's "age",
  and "a daemon older than 1h with a stale claim" sit side by side. The item
  never says which timestamp age is measured from, or whether `Unknown` must
  pass both the claim window and the backstop.
- 🟡 **Testability**: Cross-launcher prune criterion omits the shared cache-root precondition
  **Location**: Acceptance Criteria
  The criterion does not require both launchers to share one
  `ACCELERATOR_CACHE_DIR` root. On per-version roots the second launcher never
  sees the daemon's generations, so the test would pass before and after the fix.

#### Minor

- 🔵 **Dependency**: Extracting a shared lease crate is a prerequisite step that Dependencies does not record
  **Location**: Technical Notes
  The cold-path fix cannot land until the lease primitive moves to a shared
  crate, and Dependencies records this neither as a blocker nor as an ordered
  step.
- 🔵 **Scope**: Relocating the lease primitive is structural work that appears only in Technical Notes
  **Location**: Technical Notes
  A probable new library crate with registration overhead is hidden inside a
  low-priority bug, so the item risks being under-sized.
- 🔵 **Completeness**: Open Questions omits the undecided home for the shared lease primitive
  **Location**: Open Questions
  "None blocking" conflicts with the unresolved crate-structure decision in
  Technical Notes.
- 🔵 **Testability**: "Daemon has exited" precondition ignores other lease holders
  **Location**: Acceptance Criteria
  The executor holds the lease across the client `exec`, so the client process
  may still hold it after the daemon exits, and the expected result is
  therefore ambiguous.
- 🔵 **Testability**: No criterion verifies the lease is taken before the daemon spawns
  **Location**: Acceptance Criteria
  The criterion only probes while the daemon runs, so a late or daemon-opened
  lease would pass despite the ordering and inheritance contract.
- 🔵 **Testability**: Bug states mechanism but not the observable broken outcome
  **Location**: Requirements
  No command sequence or user-visible symptom is given for a red test to
  capture.
- 🔵 **Completeness**: No concrete reproduction sequence for the bug
  **Location**: Requirements
  The setup, commands, expected outcome and actual outcome are scattered
  across Context, Requirements and Acceptance Criteria.
- 🔵 **Clarity**: Summary says "two defects" but Requirements lists three current-behaviour items
  **Location**: Summary / Requirements
  Item 3 is a verification gap, not a defect, and the Summary does not mention
  it.
- 🔵 **Clarity**: "The design executor" and "the client exec" have no clear referent
  **Location**: Requirements: Expected behaviour / Technical Notes
  The item names `design-adapters`, `accelerator-design` and `design-cli`
  separately, but never says which of them is the executor or which process is
  the client.
- 🔵 **Clarity**: Domain terms used without definition
  **Location**: Summary / Requirements / Acceptance Criteria
  The terms "cold path", "warm path", "consumed generation" and "single-flight
  lock" are not defined or linked.
- 🔵 **Dependency**: Protection on a shared root depends on which release each co-resident launcher is running
  **Location**: Context
  Daemons started by already-released launchers still hold no lease, so a fixed
  launcher's `prune` can still reclaim their trees.
- 🔵 **Scope**: Two independently deliverable defects bundled in one bug
  **Location**: Summary
  The `Unknown` predicate fix is small and launcher-local. The cold-path fix
  needs a crate extraction. Either split them or justify the bundling.

#### Suggestions

- 🔵 **Scope**: Summary names two defects but Requirements list three items
  **Location**: Summary
  Mention the warm-path proof in the Summary.
- 🔵 **Testability**: No regression criterion for `Free` and `Held` liveness in the changed functions
  **Location**: Acceptance Criteria
  A Held/Free/Unknown × under/over `AGE_BACKSTOP` table would stop the backstop
  from leaking into the `Free` case.
- 🔵 **Clarity**: "AGE_BACKSTOP follows the probe" is ambiguous
  **Location**: Requirements: Current behaviour item 2
  Say which liveness values send `reap_orphans` to the age check.
- 🔵 **Dependency**: Parent epic 0136 and sibling cache work 0218 are not listed in Dependencies
  **Location**: Dependencies
  0218 also touches cache reclamation, so record whether the two items overlap.

### Strengths

- ✅ Current and expected behaviour are separated per defect and anchored to
  exact functions and file locations.
- ✅ Context limits exposure to shared `ACCELERATOR_CACHE_DIR` roots and explains
  why the per-version root is safe, which justifies the low priority.
- ✅ The acceptance criteria use Given/When/Then with mechanically checkable
  outcomes. `--older-than 0` removes the claim window as a confounder.
- ✅ The `Unknown` criterion is bounded on both sides of `AGE_BACKSTOP`, and
  Technical Notes names the probe-injection seam needed because `ENOLCK` cannot
  be reproduced locally.
- ✅ The out-of-scope list (version-aware daemon reuse, cross-root footprint,
  `${CLAUDE_PLUGIN_DATA}` relocation) and the warm-path inclusion rationale
  give the item firm boundaries.
- ✅ The scope traces cleanly to 0223's "Lease safety on reclamation" hand-over,
  and Assumptions name the accepted trade-off for the `Unknown` fallback.

### Recommended Changes

1. **Reconcile the age rules** (addresses: Several age thresholds and "age"
   subjects are not reconciled; "AGE_BACKSTOP follows the probe" is ambiguous)
   State which timestamp `AGE_BACKSTOP` is measured from for `Unknown`
   generations. Say whether the 14-day claim window still applies before the
   backstop. Define "claim" or link to where it is defined, and say how
   `reap_orphans` combines the probe with the age check.
2. **Add the shared-root precondition and a control to the prune criterion**
   (addresses: Cross-launcher prune criterion omits the shared cache-root
   precondition) Start the criterion with "Given both launchers share one
   `ACCELERATOR_CACHE_DIR` root…". Optionally add a control in which an
   unleased generation is removed.
3. **Decide and surface the lease primitive's home** (addresses: the
   prerequisite, relocation and Open Questions findings) Name the target crate
   in Requirements or Technical Notes and record the extraction as an ordered
   first step under Dependencies. Alternatively, make it an Open Question with
   candidate options, or a separate blocking work item.
4. **Add a reproduction sequence** (addresses: No concrete reproduction
   sequence; Bug states mechanism but not the observable broken outcome) Give
   the setup (a shared root and two launcher digests), the commands (cold
   `cache ensure`, spawn the daemon, `prune --older-than 0`), and the expected
   and actual outcomes.
5. **Tighten the lease-release and lease-timing criteria** (addresses: "Daemon
   has exited" precondition ignores other lease holders; No criterion verifies
   the lease is taken before the daemon spawns; No regression criterion for
   `Free` and `Held`) Name every process that must have exited. Either add a
   check at spawn time or narrow the "before it spawns" wording. Extend the
   `Unknown` criterion into a liveness × age table.
6. **Align the Summary and define terms** (addresses: the "two defects" vs
   three items findings; "The design executor" has no clear referent; Domain
   terms used without definition) Mention the warm-path proof in the Summary.
   Name the crate or binary that is the design executor and say which process
   is the client. Define cold path, warm path and consumed generation in one
   line each.
7. **Record residual and neighbouring couplings** (addresses: Protection on a
   shared root depends on which release…; Parent epic 0136 and sibling 0218…;
   Two independently deliverable defects bundled) Add an Assumption that
   daemons from older releases remain exposed. List 0136 and 0218 under
   Relates to. Add a Drafting Note on why the two fixes ship together, or split
   them.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Precise and well anchored, with exact functions, locations and
liveness states. The main gaps are the unreconciled age measures (14-day claim
window, 1h `AGE_BACKSTOP`, unqualified "age"), undefined components and terms
(design executor, cold/warm path, claim, consumed generation), and the "two
defects" versus three items mismatch.

**Strengths**:
- Current and expected behaviour are cleanly separated and each defect is tied
  to an exact function and file location.
- The `Unknown` rule is stated precisely ("not known to be free", reclaimed only
  once age reaches `AGE_BACKSTOP`), and Assumptions name the trade-off.
- Context explains why only shared roots are affected.
- The acceptance criteria name observable states.

**Findings**:
- 🟡 major / medium — Context / Expected behaviour / Acceptance Criteria /
  Assumptions — **Several age thresholds and "age" subjects are not
  reconciled.** The item does not say whether "age" means time since claim,
  since materialisation, or daemon runtime, or whether the 14-day window still
  applies to `Unknown`. An implementer could tie the backstop to the wrong
  timestamp. Define the timestamp, the combination rule and "claim".
- 🔵 minor / medium — Requirements: Expected behaviour / Technical Notes —
  **"The design executor" and "the client exec" have no clear referent.** The
  item names `design-adapters`, `accelerator-design` and `design-cli`
  separately, so it is unclear which crate owns lease acquisition.
- 🔵 minor / high — Summary / Requirements — **Summary says "two defects" but
  Requirements lists three items.** Item 3 is a verification gap and is
  reconciled only in Drafting Notes.
- 🔵 minor / medium — Summary / Requirements / Acceptance Criteria — **Domain
  terms used without definition.** The terms are cold/warm path, consumed
  generation and single-flight lock.
- 🔵 suggestion / low — Requirements item 2 — **"AGE_BACKSTOP follows the
  probe" is ambiguous.** "Follows" could mean that the check runs after the
  probe or that it is the fallback for `Unknown`.

### Completeness

**Summary**: A well-populated bug report. Every section is present and
substantive. The gaps are minor: there is no step-by-step reproduction, and
Open Questions says "None blocking" despite the undecided lease-primitive home.

**Strengths**:
- Frontmatter is complete and consistent with the body.
- Each defect is split into current and expected behaviour, with precise
  anchors.
- Context explains the forces and limits exposure, and Drafting Notes tie the
  priority to that limit.
- The acceptance criteria cover cold path, warm path, cross-launcher prune,
  post-exit reclamation and the `Unknown` backstop.
- Assumptions and the out-of-scope list are explicit.

**Findings**:
- 🔵 minor / medium — Requirements — **No concrete reproduction sequence for
  the bug.** Add steps to reproduce, with setup, commands, expected and actual
  outcomes, and note that `Unknown` needs an injected probe.
- 🔵 minor / medium — Open Questions — **Open Questions omits the undecided
  home for the shared lease primitive.** Record the intended home or list the
  options.

### Dependency

**Summary**: The item records its origin (0223) and the source of the lease
model (0196), and correctly states it has no blockers. It misses three
couplings: the shared-crate extraction prerequisite, the residual exposure from
already-released launchers, and links to parent 0136 and sibling 0218.

**Strengths**:
- 0223 and 0196 match the spike's follow-up entry.
- "Blocked by: none" and "Blocks: none" are explicit.
- Technical Notes surfaces the cross-crate coupling.
- The out-of-scope list prevents hidden dependencies.

**Findings**:
- 🔵 minor / high — Technical Notes — **Extracting a shared lease crate is a
  prerequisite step that Dependencies does not record.** Record it as an
  ordered first step, or split it out as a blocker.
- 🔵 minor / medium — Context — **Protection on a shared root depends on which
  release each co-resident launcher is running.** Older releases' daemons stay
  unleased. Add an Assumption stating the accepted residual risk.
- 🔵 suggestion / medium — Dependencies — **Parent epic 0136 and sibling cache
  work 0218 are not listed.** 0218 also changes cache reclamation, so record
  whether they overlap.

### Scope

**Summary**: One clear purpose, but two independently deliverable defects that
touch different crates. Bundling them is defensible, since the cross-launcher
criterion needs both, but the item should say so or split them. The
lease-primitive relocation is structural work that appears only in Technical
Notes.

**Strengths**:
- Clear out-of-scope list.
- The warm-path proof's inclusion is explicitly justified.
- Context limits exposure to shared roots.
- The item traces cleanly to the 0223 hand-over.

**Findings**:
- 🔵 minor / medium — Summary — **Two independently deliverable defects
  bundled in one bug.** Split the `Unknown` fix, or add a Drafting Note on why
  the two fixes ship together.
- 🔵 minor / medium — Technical Notes — **Relocating the lease primitive is
  structural work that appears only in Technical Notes.** Bring it into
  Requirements, or make it a prerequisite item.
- 🔵 suggestion / high — Summary — **Summary names two defects but
  Requirements list three items.** Mention the warm-path proof.

### Testability

**Summary**: Strongly verifiable overall, with Given/When/Then criteria,
observable outcomes and a named probe seam. The gaps are preconditions: the
shared root is missing from the cross-launcher criterion, the "daemon exited"
precondition does not cover all lease holders, and no observable broken
outcome is stated.

**Strengths**:
- Each criterion has an explicit precondition, action and outcome, and every
  outcome can be checked mechanically.
- The `Unknown` criterion is bounded on both sides of `AGE_BACKSTOP`.
- The `ENOLCK` limitation and the probe seam are stated up front.
- The warm-path proof was included deliberately, to make the criteria set
  complete.
- `--older-than 0` removes the claim window as a confounder.

**Findings**:
- 🟡 major / high — Acceptance Criteria — **Cross-launcher prune criterion
  omits the shared cache-root precondition.** Without a shared root, the test
  passes trivially. Add the precondition and a control case.
- 🔵 minor / medium — Acceptance Criteria — **"Daemon has exited"
  precondition ignores other lease holders.** The client process may still hold
  the inherited fd. Name every process that must have exited.
- 🔵 minor / medium — Requirements — **Bug states mechanism but not the
  observable broken outcome.** Add a reproduction with the actual symptom.
- 🔵 minor / medium — Acceptance Criteria — **No criterion verifies the lease
  is taken before the daemon spawns.** A late or daemon-opened lease would
  pass. Add a spawn-time check or narrow the wording.
- 🔵 suggestion / low — Acceptance Criteria — **No regression criterion for
  `Free` and `Held` liveness in the changed functions.** Add a liveness × age
  table.

## Re-Review (Pass 2) — 2026-10-12T02:11:43+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: Several age thresholds and "age" subjects are not reconciled — Resolved
- 🟡 **Testability**: Cross-launcher prune criterion omits the shared cache-root precondition — Resolved
- 🔵 **Dependency / Scope / Completeness**: Lease primitive relocation hidden in Technical Notes — Resolved (`tree-lease` in Requirements and Dependencies ordering)
- 🔵 **Testability**: "Daemon has exited" precondition ignores other lease holders — Partially resolved (daemon and client named; descendants deferred to the plan)
- 🔵 **Testability**: No criterion verifies the lease is taken before the daemon spawns — Resolved, but the new criterion is hard to observe (see below)
- 🔵 **Testability / Completeness**: No reproduction or observable broken outcome — Partially resolved (steps added; cold-path trigger unstated)
- 🔵 **Clarity / Scope**: Summary says "two defects" but Requirements lists three items — Resolved
- 🔵 **Clarity**: "The design executor" and "the client exec" have no clear referent — Partially resolved (named, but the process model is introduced only in Technical Notes)
- 🔵 **Clarity**: Domain terms used without definition — Mostly resolved (generation's relation to tree and digest still implicit)
- 🔵 **Clarity**: "AGE_BACKSTOP follows the probe" is ambiguous — Resolved
- 🔵 **Dependency**: Protection depends on co-resident launcher releases — Resolved (Assumptions)
- 🔵 **Dependency**: 0136 and 0218 not listed — Resolved
- 🔵 **Scope**: Two defects bundled — Resolved (Drafting Note)
- 🔵 **Testability**: No regression criterion for `Free` and `Held` — Resolved (liveness × age table)

### New Issues Introduced

- 🟡 **Clarity**: "Guarantee on every filesystem" contradicts the accepted `Unknown` exposure — the Drafting Note and the cross-launcher criterion overclaim relative to Assumptions.
- 🟡 **Testability** (also completeness, clarity): No stated trigger for the warm versus cold path — a verifier cannot reliably force the cold path.
- 🟡 **Testability** (also scope, dependency, completeness): Expected outcome undefined if daemon descendants keep the lease fd — deferred to the plan with no scope boundary or Open Question.
- 🔵 **Testability / Clarity**: Cross-launcher criterion's "either path" reads as one path; ambiguous "it" in the spawn criterion.
- 🔵 **Testability**: Spawn-time criterion checks executor-internal state; restate as an observable outcome.
- 🔵 **Testability**: `tree-lease` extraction has no acceptance criterion.
- 🔵 **Clarity**: "Reclaim and repair paths" undefined; `reap_orphans` listed under the defect although it is the reference rule.
- 🔵 **Clarity**: Executor / client / daemon process model introduced only in Technical Notes.
- 🔵 **Dependency**: Liveness-probe injection seam missing from the Ordering bullet.
- 🔵 **Suggestions**: Summary omits the crate extraction; rollout coupling only in Assumptions; ADR-0064 amendment unstated; generation/tree/digest relation implicit; an end-to-end `prune` criterion for `Unknown`; plan the two fixes as separable phases.

### Assessment

The first pass's two major findings are resolved, and most minor findings are
resolved. The revision introduced three new major findings. Two are gaps the new
detail exposed: the warm and cold path triggers, and the outcome when daemon
descendants keep the lease. The third, the "every filesystem" claim, is
overclaiming that the revision added. All three are small edits once the
descendant scope is decided. The work item needs one more iteration before
planning.

## Re-Review (Pass 3) — 2026-10-12T02:18:18+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity**: "Guarantee on every filesystem" contradicts the accepted `Unknown` exposure — Partially resolved (criterion qualified; scope notes the bundling rationale no longer follows from the criteria)
- 🟡 **Testability / Completeness / Clarity**: No stated trigger for the warm versus cold path — Resolved (the mixed-export trigger lacks its own criterion)
- 🟡 **Testability / Scope / Dependency / Completeness**: Expected outcome undefined if daemon descendants keep the lease fd — Partially resolved (Open Question recorded and scoped out, but the reclaim-after-exit criterion still depends on its answer)
- 🔵 **Testability / Clarity**: "Either path" and ambiguous "it" — Resolved
- 🔵 **Testability**: Spawn-time criterion checks executor internals — Resolved
- 🔵 **Testability**: `tree-lease` extraction has no criterion — Partially resolved (registration not checked)
- 🔵 **Clarity**: "Repair paths" undefined; `reap_orphans` role — Resolved
- 🔵 **Clarity**: Process model only in Technical Notes — Resolved
- 🔵 **Dependency**: Probe seam missing from ordering — Resolved
- 🔵 **Suggestions** (crate extraction in Summary, rollout coupling, ADR status, generation definition, end-to-end `Unknown` criterion, separable phases) — Resolved

### New Issues Introduced

- 🟡 **Dependency / Scope / Testability** (merged): The reclaim-after-exit criterion depends on the deferred descendant-fd question — if browser processes keep the fd, the criterion fails or flakes. Fix by stopping the daemon's whole process group in the precondition, or by naming descendants among the processes that must have exited.
- 🔵 **Clarity**: "Superseded" and "orphan" generations are undefined, which obscures the `reap_superseded` to `reap_orphans` handoff.
- 🔵 **Clarity**: "Reference rule" could be read as applying the backstop to `Free` in `reclaim_unclaimed` too.
- 🔵 **Clarity**: Code citations use bare filenames.
- 🔵 **Testability**: The control criterion's "unleased generation of the first launcher's digest" setup is unclear and may exercise a different reclaim path.
- 🔵 **Testability**: The mixed "driver without browser" cold-path trigger has no criterion.
- 🔵 **Testability**: `tree-lease` checklist registration is not checked.
- 🔵 **Completeness**: Repro step 3 omits that the daemon must still be running, and how to obtain a second launcher digest.
- 🔵 **Dependency**: Conditional follow-up for descendant fd-closing is not shown as a downstream link.
- 🔵 **Scope**: The bundling rationale cites the cross-launcher guarantee, which is now limited to `flock` filesystems where `Unknown` never arises.
- 🔵 **Suggestions**: Summary omits the `Unknown` fix; sticky-failure marker undefined; substitute repro for `Unknown`; real versus stub daemon; deterministic mtime control; two-digest launcher fixture.

### Assessment

The two major themes from pass 2 are largely resolved. The one remaining major
concern, raised independently by three lenses, is that the reclaim-after-exit
criterion depends on the deferred question about descendant fds. A single
precondition change fixes it. The remaining findings are minor precision issues.
The work item is acceptable for planning as it stands, and one more targeted
edit would tighten it.

## Re-Review (Pass 4) — 2026-10-12T02:24:25+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Dependency / Scope / Testability**: The reclaim-after-exit criterion depends on the deferred descendant-fd question — Resolved (the process-group precondition decouples the criterion)
- 🔵 **Clarity**: "Superseded" and "orphan" undefined — Resolved
- 🔵 **Clarity**: "Reference rule" could extend the backstop to `Free` — Resolved
- 🔵 **Clarity**: Bare-filename citations — Resolved
- 🔵 **Testability**: Control criterion setup unclear — Resolved
- 🔵 **Testability**: The mixed-export cold-path trigger has no criterion — Resolved
- 🔵 **Testability**: `tree-lease` checklist registration not checked — Resolved
- 🔵 **Completeness**: Repro step 3 omits the running daemon; second-digest source — Resolved
- 🔵 **Dependency**: Conditional descendant follow-up not linked — Resolved
- 🔵 **Scope**: Bundling rationale mismatched the criteria — Resolved (the scope lens still records the bundling as a minor, acceptable trade-off)
- 🔵 **Suggestions** (`Unknown` in Summary, sticky marker, substitute repro, stub daemon, clock control, two-digest fixture) — Resolved

### New Issues Introduced

- 🔵 **Clarity**: The Summary's "not known to be free" reads as never reclaiming, unlike the age-gated rule.
- 🔵 **Clarity**: The warm-path proof is framed as already true; the item does not say whether a failure is fixed here or elsewhere.
- 🔵 **Testability**: The lease-release criterion does not require both warm and cold variants.
- 🔵 **Testability**: The daemon criteria lack a "no daemon already running" precondition, and a reused daemon could mask the path under test.
- 🔵 **Testability**: The `Held` criteria name `probe_liveness`, which the `design-adapters` tests cannot reach; restate them crate-neutrally or name the owning test harness.
- 🔵 **Completeness**: No stand-in reproduction for the `reap_superseded` half of the `Unknown` defect.
- 🔵 **Dependency**: Ordering omits that the per-path lease proofs and the two-digest fixture come before the cross-launcher criteria.
- 🔵 **Suggestions**: Define liveness values, `AGE_BACKSTOP`, "downgrades" and `process-probe` in Context; bind "its client"; state the verification level of the injected-`Unknown` `prune` criterion; lease release when the daemon spawn fails after leasing; list `flock` backend behaviour as an external coupling.

### Assessment

No critical or major findings remain, and every finding from pass 3 is resolved.
What is left is minor precision work: preconditions for the daemon criteria, the
test harness that owns the `Held` checks, and Summary wording. Planning can
absorb any of it. The work item is ready for implementation planning.

## Re-Review (Pass 5) — 2026-10-12T02:32:16+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🔵 **Clarity**: The Summary's "not known to be free" diverges from the age-gated rule — Resolved
- 🔵 **Clarity**: Warm-path proof framed as already true — Resolved (fix in scope if the test fails; scope now flags this as open-ended)
- 🔵 **Testability**: The lease-release criterion lacked warm and cold variants — Resolved
- 🔵 **Testability**: No "no daemon already running" precondition — Resolved
- 🔵 **Testability**: `Held` criteria named a probe in another crate — Resolved ("observed held")
- 🔵 **Completeness**: No `reap_superseded` stand-in reproduction — Resolved
- 🔵 **Dependency**: Ordering omitted the cross-launcher prerequisites — Resolved
- 🔵 **Suggestions** (liveness values, downgrade meaning, client identity, `process-probe`, `prune` verification level, spawn-failure leases, `flock` backend coupling) — Resolved

### New Issues Introduced

- 🟡 **Clarity**: "Defects matter only on a shared root" is argued for `prune` only. `reap_superseded` runs from `accelerator cache repair` on any root, so on a filesystem without `flock` a repair can remove a live daemon's tree on a per-version root too.
- 🔵 **Clarity**: The Summary implies the `Unknown` exposure is closed; Assumptions call it a narrow mitigation.
- 🔵 **Clarity**: "Tree", "lease" and "daemon" are each used in more than one sense; the executor client holds rather than inherits its leases.
- 🔵 **Completeness**: The `Unknown` stand-in reproduction needs a seam that does not exist yet; its "Actual" outcomes come from inspection.
- 🔵 **Testability**: Probe-injection wiring is proven for `prune` but not for the repair path that runs `reap_superseded`.
- 🔵 **Testability**: The window between `cache ensure` exiting and the executor taking its leases is unspecified.
- 🔵 **Scope**: The conditional warm-path fix is open-ended; two independently deliverable defects remain bundled (accepted trade-off).
- 🔵 **Suggestions**: Track the conditional follow-up; record the descendant-fd check's result; consider trimming the end-to-end matrix; gloss "design invocation", warm-path checks, browser override and the two-digest fixture; tie the sticky-marker downgrade to its trigger.

### Assessment

Every finding from pass 4 is resolved. One new major clarity finding stands, and
it is substantive: the repair path makes the `reap_superseded` half of the
`Unknown` defect independent of shared roots. Correcting the exposure
statement, and wiring the repair-path criterion, closes it. The remaining
findings are minor. The verdict stays COMMENT, and the work item is ready for
planning once the exposure statement is corrected.

## Verdict Change — 2026-10-12T02:48:29+00:00

**Verdict:** APPROVE

Toby Clemson changed the verdict from COMMENT to APPROVE after the pass 5 edits
had been applied. Those edits correct the exposure statement, add the `repair`
entry-point criterion, recast the `reclaim_unclaimed` change as a narrow
mitigation, accept the ensure-to-lease gap, bound the warm-path fix, and
settle the term usage. No review pass ran after them.
