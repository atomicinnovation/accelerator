---
type: work-item-review
id: "0101-unified-dev-task-for-visualiser-review-1"
title: "Work Item Review: Unified Managed dev Task for Visualiser Server and Frontend"
date: "2026-06-06T13:32:59+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0101"
relates_to: ["work-item:0100"]
work_item_id: "0101"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-06-06T17:26:45+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Unified Managed dev Task for Visualiser Server and Frontend

**Verdict:** REVISE

This is a strong, thoroughly populated task: every standard section carries
substantive content, requirements map cleanly to acceptance criteria, and the
real startup-ordering constraint (server must write `server-info.json` before
Vite's `resolveApiPort()` reads it) is captured concretely with named files
and code paths. The verdict is REVISE rather than APPROVE solely because three
major testability gaps would let core behaviour pass review while broken:
unquantified timeout/return thresholds, an unverifiable "behave identically"
cross-platform claim, and an investigation phase with no exit criterion. None
are structural — they are tractable wording/criteria additions.

### Cross-Cutting Themes

- **Investigation phase is under-specified** (flagged by: completeness, scope,
  dependency, testability) — the gating investigation has no time-box
  (completeness), is folded into the delivery item so implementation is blocked
  on its outcome (scope), may introduce a new Python package not reflected in
  Dependencies (dependency), and has no enumerable exit criterion (testability).
  This is the single most reinforced concern across lenses.
- **Unverifiable / unquantified outcomes** (flagged by: testability, clarity) —
  "within a few seconds", "within the timeout", "behave identically", and
  "equivalents of build/deps" all lack a definite pass/fail boundary or precise
  meaning.
- **`kind: task` may under-signal the work** (flagged by: completeness, scope) —
  the breadth (four lifecycle commands, cross-platform process-group
  supervision, a decision-bearing investigation, a possible ADR) reads closer
  to a story.
- **Open questions leave behaviour and scope unsettled** (flagged by: clarity,
  scope) — the fail-loud-vs-reuse decision materially changes both what
  `mise run dev` does and how large the item is, yet Requirement 1 reads as if
  settled.
- **Relationship to work item 0100 is advisory, not actionable** (flagged by:
  clarity, dependency) — "stay consistent on stop semantics" states a goal
  without a concrete requirement or an ordering implication.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: Readiness timeout and return-time thresholds are unquantified
  **Location**: Acceptance Criteria (criteria 1 and 4), Requirements: 7
  `mise run dev` must return "within a few seconds" and the server must be ready
  "within the timeout", but no concrete numeric threshold is given. Without a
  bound, a verifier cannot conclusively pass/fail, and a startup regression
  could slip through.

- 🟡 **Testability**: "Behave identically on macOS and Linux" is not directly verifiable
  **Location**: Acceptance Criteria (cross-platform criterion), Requirements: 10
  "Identical" is undefined — identical exit codes? surfaced fields? child-process
  teardown? The criterion can be argued met or unmet depending on which
  differences are deemed acceptable.

- 🟡 **Testability**: Investigation phase has no enumerable exit criterion
  **Location**: Requirements: Investigation (resolve before implementation)
  The mandated investigation ("Choose the supervision approach… Record the
  decision"; "Confirm the detach mechanism… on macOS and Linux") has no
  acceptance criterion tying off when it is complete, so it could be claimed
  done without a recorded decision or demonstrated cross-platform check.

#### Minor

- 🔵 **Clarity**: Unresolved stop/reuse semantics leave Requirement 1 behaviour ambiguous
  **Location**: Open Questions / Requirements: 1
  The Open Questions ask whether `dev` should fail loudly or reuse an existing
  session, but Requirement 1 reads as fully settled. A reader cannot tell what
  `mise run dev` does when a session is already up.

- 🔵 **Clarity**: "Stay consistent on stop semantics" with 0100 is underspecified
  **Location**: Dependencies / Context
  It is not stated what "consistent" concretely requires — whether `dev:stop`
  must invoke the server's own stop path, signal the process group, or
  coordinate with 0100's configurable timeout.

- 🔵 **Completeness**: Investigation phase lacks a time-box or effort constraint
  **Location**: Requirements: Investigation
  Spike-like investigative work normally carries a bound; without one the
  supervision-approach investigation could absorb disproportionate effort.

- 🔵 **Completeness**: `kind: task` may understate the investigation-then-build shape
  **Location**: Frontmatter: kind
  The item gates a decision-bearing investigation (possibly adding a dependency
  and warranting an ADR) ahead of implementation — closer to a spike-plus-task.

- 🔵 **Dependency**: Potential new third-party package dependency not reflected in Dependencies
  **Location**: Investigation / Open Questions / Dependencies
  The body raises adding circus/supervisor/honcho (supervisor being Unix-only),
  yet Dependencies says "Blocked by: none" and never flags the possible new
  runtime dependency or its cross-platform veto.

- 🔵 **Dependency**: 0100 relationship notes consistency but leaves ordering implication unresolved
  **Location**: Dependencies (Related: work item 0100)
  Both items touch the same auto-shutdown/`--owner-pid 0` machinery, but whether
  either must land first is left implicit — a hidden sequencing risk.

- 🔵 **Scope**: Scope described is story-sized rather than task-sized
  **Location**: Frontmatter: kind
  Four lifecycle commands, process-group supervision with signal escalation, PID
  identity checks, stale-PID cleanup, a readiness gate, cross-platform parity,
  and a dependency/ADR decision is closer to a story than an atomic task.

- 🔵 **Scope**: Investigation/decision phase folded into the delivery item
  **Location**: Requirements: Investigation
  An unresolved architectural decision (with its own ADR deliverable) is coupled
  to implementation, so the item cannot be cleanly planned/estimated until the
  spike resolves, and a dependency-adding outcome could expand scope mid-delivery.

- 🔵 **Testability**: Run-prerequisites requirement has no corresponding Acceptance Criterion
  **Location**: Requirements: 8
  No criterion verifies that `build:server:dev` / `deps:install:node` run before
  launch — criterion 1 explicitly assumes "dependencies installed", side-stepping
  the path. It could regress unseen.

- 🔵 **Testability**: Separate-log-files requirement is unverified by any criterion
  **Location**: Requirements: 5
  No criterion checks that two distinct, discoverable per-process log files exist
  and contain the expected output after `dev` returns.

- 🔵 **Testability**: "Frontend still resolves the (possibly new) server port" lacks an observable check
  **Location**: Acceptance Criteria (dev:restart criterion)
  No stated signal confirms resolution succeeded; it should be tied to the same
  observable `/api`-proxying success used in criterion 1.

#### Suggestions

- 🔵 **Clarity**: "Equivalents of" build/deps prerequisites is vague
  **Location**: Requirements: 8
  "Equivalents of `build:server:dev` and `deps:install:node`" could mean run
  those exact tasks, inline equivalent logic, or a superset. Say which.

- 🔵 **Clarity**: "Both processes" referent depends on a prior dev start
  **Location**: Requirements: 2 / Acceptance Criteria
  Outside the Summary, "both processes" relies on the reader carrying forward
  "API server and Vite frontend". Name them explicitly on first use.

- 🔵 **Dependency**: Build/install prerequisites are couplings, not flagged as such
  **Location**: Requirements: 8 / Dependencies
  The dev task cannot start until the Rust debug binary is built and node deps
  installed — optionally record this task-graph coupling in Dependencies.

- 🔵 **Scope**: Unresolved reuse-vs-fail semantics could shift scope
  **Location**: Open Questions
  Reuse/adoption is a meaningfully larger behaviour than fail-fast; resolving it
  (or scoping to fail-loud and deferring reuse) fixes the delivery boundary.

### Strengths

- ✅ Requirements consistently name the actor and trigger (e.g. "`mise run dev`
  starts both…"), avoiding actor-obscuring passive voice.
- ✅ The startup-ordering constraint is described with concrete, observable
  outcomes ("never falls through to port 0") and named files/code paths
  (`resolveApiPort()`, `server-info.json`, `vite.config.ts`).
- ✅ Seven specific Given/When/Then acceptance criteria, most mapping cleanly to
  a distinct requirement and comfortably exceeding the minimum.
- ✅ Domain jargon (PID files, `flock`, SIGTERM/SIGKILL escalation, start-time
  identity check, HMR, pty) is anchored to named reference files
  (`launcher-helpers.sh`, `launch-server.sh`) the implementer can locate.
- ✅ Scope boundaries are explicit: Assumptions fence out auto-restart/file-watch
  supervision and deprecation of the per-process tasks; an Open Question
  correctly defers `dev:logs` rather than silently absorbing it.
- ✅ Dependencies correctly states no work-item blockers/consumers, and the 0100
  relationship is characterised as "related/adjacent" rather than a hard
  dependency, keeping the item independently deliverable.
- ✅ Open Questions, Assumptions, Technical Notes, and References are genuinely
  populated rather than placeholder stubs.

### Recommended Changes

1. **Quantify the timeout and return-time thresholds** (addresses: "Readiness
   timeout and return-time thresholds are unquantified")
   Replace "within a few seconds" and "within the timeout" with concrete values
   in both Acceptance Criteria and Requirement 7 (e.g. "`dev` returns within
   5s"; "server-readiness poll times out after N seconds"). The Technical Notes
   "~5s" can become the binding default.

2. **Make the cross-platform criterion observable** (addresses: "'Behave
   identically on macOS and Linux' is not directly verifiable")
   Decompose into concrete per-platform checks — e.g. on both OSes `dev:stop`
   leaves no orphaned node/server processes (process listing), and `dev:status`
   reports the same field set with the same exit codes.

3. **Add an exit criterion for the investigation phase** (addresses:
   "Investigation phase has no enumerable exit criterion"; "lacks a time-box";
   "folded into the delivery item")
   Add an acceptance criterion such as "a recorded decision (work item or ADR)
   names the chosen supervision approach with rationale, and a documented manual
   check confirms detach/signalling on both macOS and Linux." Consider a
   time-box, or splitting the decision into a preceding spike so implementation
   runs against a fixed approach.

4. **Surface the potential new dependency and 0100 ordering in Dependencies**
   (addresses: "Potential new third-party package dependency not reflected";
   "0100 relationship… ordering implication unresolved")
   Note in Dependencies that the investigation may introduce a Python package
   (with supervisor's Unix-only constraint as a cross-platform veto), and state
   explicitly whether 0100 imposes any ordering (likely "none — this task only
   needs `--owner-pid 0` to keep disabling auto-shutdown").

5. **Add acceptance criteria for the untested requirements** (addresses:
   "Run-prerequisites… no Acceptance Criterion"; "Separate-log-files…
   unverified"; "'Frontend still resolves the port' lacks an observable check")
   Add criteria for: prerequisites running from a clean (unbuilt) checkout;
   two distinct non-empty per-process log files existing after `dev` returns;
   and `dev:restart` proving success via `/api` proxying (no port-0 fallback).

6. **Resolve or cross-reference the open behavioural questions** (addresses:
   "stop/reuse semantics leave Requirement 1 ambiguous"; "reuse-vs-fail could
   shift scope"; "'stay consistent on stop semantics' underspecified")
   Decide fail-loud vs reuse (or scope to fail-loud now, defer reuse), and
   cross-reference the open question from Requirement 1. State concretely what
   "consistent with 0100" requires.

7. **Confirm or adjust `kind`** (addresses: "Scope described is story-sized";
   "`kind: task` may understate the investigation-then-build shape")
   Either reclassify as a `story` or confirm team norms treat dev-tooling of
   this breadth as a task.

8. **Tighten remaining clarity wording** (addresses: "'Equivalents of'… vague";
   "'Both processes' referent")
   Replace "equivalents of" with the intended meaning, and name the two
   processes explicitly on first use in Requirements.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is unusually clear and well-structured: actors and
triggers are named, requirements map cleanly to acceptance criteria, and the
startup-ordering constraint is explained concretely with named files and code
paths. The main clarity concerns are a handful of self-described unresolved
decisions presented as open questions that leave the stop/reuse semantics
ambiguous, and minor referent slippage around what "the dev path" and "both
processes" denote. No internal contradictions rise to critical, and domain
terms are consistently anchored to named files or definitions.

**Strengths**:
- Requirements consistently name the actor and trigger, avoiding
  actor-obscuring passive voice.
- Domain jargon is anchored to concrete named files (`launcher-helpers.sh`,
  `launch-server.sh`), so a developer can locate the referenced definitions.
- The startup-ordering constraint is described with an unambiguous, observable
  outcome ("never falls through to port 0").
- Summary, Requirements, and Acceptance Criteria describe the same scope
  coherently.

**Findings**:
- 🔵 minor (confidence: medium) — **Unresolved stop/reuse semantics leave
  Requirement 1 behaviour ambiguous** (Open Questions / Requirements: 1): The
  Open Questions ask whether `dev` should fail loudly or reuse an existing
  session, but Requirement 1 states the start behaviour as if settled. A reader
  cannot tell what `mise run dev` does when a session is already up. Suggestion:
  cross-reference the open question from Requirement 1.
- 🔵 minor (confidence: medium) — **"Stay consistent on stop semantics" with
  0100 is underspecified** (Dependencies / Context): It is not stated what
  "consistent" concretely requires. Suggestion: state the concrete requirement
  or reword as advisory awareness.
- 🔵 suggestion (confidence: low) — **"Equivalents of" build/deps prerequisites
  is vague** (Requirements: 8): "Equivalents" could mean run those exact tasks,
  inline equivalent logic, or a superset. Suggestion: replace with the intended
  meaning.
- 🔵 suggestion (confidence: low) — **"Both processes" referent depends on a
  prior dev start** (Requirements: 2 / Acceptance Criteria): Outside the Summary,
  "both" relies on the reader carrying forward "API server and Vite frontend".
  Suggestion: name them explicitly on first use.

### Completeness

**Summary**: Work item 0101 is a thoroughly populated task with substantive
content in every standard section: a clear Summary, a rich Context, ten numbered
Requirements, seven specific Acceptance Criteria, plus populated Open Questions,
Dependencies, Assumptions, Technical Notes, and References. An implementer could
begin without follow-up questions. The only structural gap is that the embedded
investigation phase lacks an explicit time-box or effort constraint, and
`kind: task` slightly understates the investigation-plus-implementation nature.

**Strengths**:
- Summary is a clear, unambiguous action statement naming exactly what is built
  and how it behaves.
- Context fully explains the motivating problem and the real startup-ordering
  constraint, going beyond restating the Summary.
- Seven specific Given/When/Then acceptance criteria, each mapped to a distinct
  requirement.
- Kind-appropriate for a task: Requirements define the work concretely.
- Open Questions, Assumptions, Dependencies, Technical Notes, References all
  genuinely populated.
- Frontmatter carries all required fields with recognised values.

**Findings**:
- 🔵 minor (confidence: medium) — **Investigation phase lacks a time-box or
  effort constraint** (Requirements: Investigation): Spike-like work normally
  carries a bound so exploration does not expand open-endedly. Suggestion: add a
  brief effort constraint (e.g. a half-day time-box).
- 🔵 minor (confidence: low) — **`kind: task` may understate the
  investigation-then-build shape** (Frontmatter: kind): The item bundles a
  decision-bearing investigation (possibly adding a dependency, warranting an
  ADR) ahead of implementation. Suggestion: confirm `kind: task` is intended or
  consider splitting the decision into a preceding spike.

### Dependency

**Summary**: The work item is well dependency-mapped for an internal dev-tooling
task: the real startup-ordering constraint is explicitly captured in Context and
as a requirement, and Dependencies correctly states there are no external
work-item blockers or downstream consumers. The main gaps are the investigation
phase's dependency on a potential new third-party package not being framed as a
coupling, and the relationship with work item 0100 being noted as a consistency
concern without a clear ordering implication being resolved.

**Strengths**:
- The internal startup-ordering constraint is explicitly and accurately captured
  (server → `server-info.json` → Vite `resolveApiPort()`), with a programmatic
  readiness gate required rather than relying on convention.
- "Blocked by: none / Blocks: none" is correct — a self-contained dev-tooling
  task.
- The launcher prior art is named precisely as the pattern to mirror.
- The relationship to 0100 is captured with a concrete rationale.

**Findings**:
- 🔵 minor (confidence: high) — **Potential new third-party package dependency
  not reflected in Dependencies** (Investigation / Open Questions /
  Dependencies): The body raises adding circus/supervisor/honcho (supervisor
  Unix-only), yet Dependencies says "Blocked by: none". Suggestion: add a note
  that the investigation may introduce a new package, cross-referencing the
  cross-platform requirement as a gating factor.
- 🔵 minor (confidence: medium) — **0100 relationship notes consistency but
  leaves ordering implication unresolved** (Dependencies): Both items touch the
  same auto-shutdown/`--owner-pid 0` machinery; whether either must land first is
  implicit. Suggestion: state explicitly whether an ordering constraint exists.
- 🔵 suggestion (confidence: low) — **Build/install prerequisites are couplings,
  not flagged as such** (Requirements: 8 / Dependencies): Requirement 8's
  prerequisites are genuine upstream task couplings. Suggestion: optionally note
  the task-graph coupling in Dependencies.

### Scope

**Summary**: The work item describes a cohesive capability — a single managed
dev-stack lifecycle for the visualiser — so its requirements are orthogonal-free
and serve one purpose. The main scope concern is sizing: an embedded
investigation phase that may add a dependency and warrant an ADR, combined with
cross-platform process-group supervision and stale-PID handling, is heavier than
the declared `task` kind typically implies and arguably reads as a story. A
secondary concern is that the investigation/decision sub-scope could be split
out so implementation isn't blocked on an unresolved approach choice.

**Strengths**:
- All four lifecycle commands plus log files, ordering, teardown, and
  cross-platform behaviour serve one unified capability — highly coherent with
  no bundled independent concerns.
- Summary, Requirements, and Acceptance Criteria describe the same scope
  consistently.
- Scope boundaries are explicitly stated (Assumptions fence out
  auto-restart/file-watch; an Open Question defers `dev:logs`).
- The 0100 relationship is correctly characterised as related/adjacent, keeping
  the item independently deliverable.

**Findings**:
- 🔵 minor (confidence: medium) — **Scope described is story-sized rather than
  task-sized** (Frontmatter: kind): Four lifecycle commands, process-group
  supervision with signal escalation, PID identity checks, stale-PID cleanup, a
  readiness gate, cross-platform parity, and a dependency/ADR decision is closer
  to a story. Suggestion: consider reclassifying as `story` or confirm team
  norms.
- 🔵 minor (confidence: medium) — **Investigation/decision phase folded into the
  delivery item** (Requirements: Investigation): An unresolved architectural
  decision (with its own ADR deliverable) is coupled to implementation.
  Suggestion: split the decision into a preceding spike or acknowledge the
  spike-like uncertainty in sizing.
- 🔵 suggestion (confidence: low) — **Unresolved reuse-vs-fail semantics could
  shift scope** (Open Questions): Reuse/adoption is meaningfully larger than
  fail-fast. Suggestion: resolve it or scope to fail-loud and defer reuse.

### Testability

**Summary**: The Acceptance Criteria are unusually strong for a task: most are
framed as Given/When/Then with observable, procedure-verifiable outcomes. The
main gaps are an undefined readiness/return timeout threshold, the unverifiable
"identical behaviour" cross-platform claim, the absence of any exit criterion
for the mandated investigation phase, and untested requirements (run
prerequisites, separate log files) that no Acceptance Criterion covers.

**Strengths**:
- Most Acceptance Criteria specify a precondition, action, and observable
  outcome (process listing checks, port-0 fallback avoidance, non-zero exit on
  timeout).
- The stale/recycled PID criterion is concretely testable.
- The `dev:status` criterion enumerates the exact data that must surface.

**Findings**:
- 🟡 major (confidence: high) — **Readiness timeout and return-time thresholds
  are unquantified** (Acceptance Criteria): "Within a few seconds" / "within the
  timeout" have no numeric threshold, so a verifier cannot conclusively
  pass/fail. Suggestion: specify the thresholds.
- 🟡 major (confidence: high) — **"Behave identically on macOS and Linux" is not
  directly verifiable** (Acceptance Criteria, cross-platform criterion):
  "Identical" has no observable definition. Suggestion: decompose into concrete,
  per-platform checks.
- 🟡 major (confidence: high) — **Investigation phase has no enumerable exit
  criterion** (Investigation): No criterion ties off when the phase is complete.
  Suggestion: add a criterion requiring a recorded decision (or ADR) plus a
  documented cross-platform detach/signalling check.
- 🔵 minor (confidence: high) — **Run-prerequisites requirement has no
  corresponding Acceptance Criterion** (Requirements: 8): No criterion verifies
  build/install-before-launch; criterion 1 assumes deps installed. Suggestion:
  add a clean-checkout criterion.
- 🔵 minor (confidence: medium) — **Separate-log-files requirement is unverified
  by any criterion** (Requirements: 5): No criterion checks that two distinct
  discoverable log files exist with expected output. Suggestion: add a log-file
  criterion.
- 🔵 minor (confidence: medium) — **"Frontend still resolves the (possibly new)
  server port" lacks an observable check** (Acceptance Criteria, dev:restart):
  No stated signal confirms resolution succeeded. Suggestion: tie to successful
  `/api` proxying.

## Re-Review (Pass 2) — 2026-06-06

**Verdict:** REVISE

Re-ran all five lenses against the revised work item. **Every Pass-1 major and
minor finding is resolved** by the revision (thresholds quantified, cross-platform
criterion decomposed, supervision approach decided as `circus`, reuse semantics
fixed, missing acceptance criteria added, dependencies surfaced, 0100 ordering
clarified). However, the two new decisions introduced fresh definitional gaps
that two lenses independently flagged at major severity — hence a REVISE verdict
on the pass. **All three new majors were then addressed by follow-up edits**
(see "Edits Applied After Re-Review" below), leaving only low-impact
minors/suggestions.

### Previously Identified Issues

- 🟡 **Testability**: Readiness timeout / return-time thresholds unquantified —
  **Resolved** (30 s readiness timeout, 100 ms poll, 10 s return now stated).
- 🟡 **Testability**: "Behave identically on macOS and Linux" not verifiable —
  **Resolved** (decomposed into concrete per-platform checks).
- 🟡 **Testability**: Investigation phase has no exit criterion — **Resolved**
  (supervision approach decided as `circus`; investigation removed).
- 🔵 **Clarity**: Stop/reuse semantics ambiguous in Requirement 1 — **Resolved**
  (reuse decided and stated; open question removed).
- 🔵 **Clarity**: "Stay consistent with 0100" underspecified — **Resolved**
  (concrete "no ordering dependency" statement added).
- 🔵 **Completeness**: Investigation lacks a time-box — **Resolved** (decision
  made up front, no open-ended investigation).
- 🔵 **Completeness / Scope**: `kind: task` understates the work — **Still
  present (accepted)**; author elected to keep `kind: task` per team norms.
- 🔵 **Dependency**: New package not reflected in Dependencies — **Resolved**.
- 🔵 **Dependency**: 0100 ordering implication unresolved — **Resolved**.
- 🔵 **Scope**: Story-sized scope / investigation folded in — **Resolved**
  (investigation resolved to a decision; sizing accepted as a `task`).
- 🔵 **Testability**: Run-prerequisites / separate-log-files / restart-proxy had
  no acceptance criteria — **Resolved** (criteria added for all three).

### New Issues Introduced

- 🟡 **Clarity + Testability**: "Healthy session" reuse predicate was undefined —
  flagged by both lenses (the reuse short-circuit hinged on an undefined term).
  **Addressed**: Requirement 1 now defines "healthy" (arbiter reachable AND both
  watchers running) and the reuse criterion references it.
- 🟡 **Testability**: Cross-platform criterion depended on an undefined "grace
  period" duration. **Addressed**: criterion now states the **2 s SIGTERM grace
  period**.
- 🔵 **Clarity / Testability**: Return-time anchored to the unobservable "server
  becoming ready". **Addressed**: re-anchored to `server-info.json` being
  written.
- 🔵 **Testability**: `dev:stop` "all child processes / process listing" lacked a
  scoped verification procedure. **Addressed**: changed to the recorded
  server/frontend process groups (children of recorded PIDs, not a global grep).
- 🔵 **Dependency + Scope**: ADR called for in prose but not an explicit
  deliverable. **Addressed**: added an ADR acceptance criterion and a "Produces"
  entry in Dependencies.
- 🔵 **Clarity**: "start-time identity check" / "recycled PID" referenced without
  a gloss; "detached" vs the long-lived arbiter slightly in tension.
  **Addressed**: added a start-time-identity-check gloss in Requirement 9 and
  clarified in Requirement 1 that `dev` launches the arbiter and detaches from
  it.
- 🔵 **Testability**: `dev:status` exit codes asserted for parity but not
  enumerated. **Addressed**: enumerated in Requirement 4 and the status
  criterion (0 = both running, 3 = one running, 4 = neither; identical across
  platforms).
- 🔵 **Completeness**: referenced work item 0100 is a thin stub; `dev:logs`
  open question still unresolved. **Open (suggestion)** — informational only.

### Edits Applied After Re-Review

1. Defined "healthy session" in Requirement 1 and referenced it from the reuse
   acceptance criterion.
2. Promoted the 2 s SIGTERM grace period into the cross-platform criterion.
3. Re-anchored the return-time criterion to `server-info.json` being written.
4. Scoped the `dev:stop` teardown check to the recorded process groups.
5. Added an ADR acceptance criterion and a Dependencies "Produces" entry.
6. Glossed the start-time identity check (Req 9) and clarified arbiter/detach
   semantics (Req 1).
7. Enumerated `dev:status` exit codes (Req 4 + status criterion).

### Assessment

The revision cleared every Pass-1 finding. The re-review surfaced three new
major definitional gaps that were a direct consequence of the reuse and
cross-platform decisions; all three were edited in, along with the two residual
clarity/testability minors. The only outstanding items are informational
suggestions (referenced work item 0100 is a thin stub; the `dev:logs` helper
remains an open question deferred to a follow-up). The work item is ready for
planning.

**Verdict updated to APPROVE (2026-06-06)** — all Pass-1 findings, the
re-review majors, and both residual minors are resolved; only informational
suggestions remain, which do not block. The work item has been transitioned to
`status: ready`.

