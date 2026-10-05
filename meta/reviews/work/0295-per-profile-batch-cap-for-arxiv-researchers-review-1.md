---
type: "work-item-review"
id: "0295-per-profile-batch-cap-for-arxiv-researchers-review-1"
title: "Work Item Review: Fair arXiv Fetch Queue That Waits Across Calls"
date: "2026-10-05T21:16:55+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0295"
work_item_id: "0295"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 4
tags: ["research", "deep-research", "arxiv"]
last_updated: "2026-10-05T22:51:50+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Fair arXiv Fetch Queue That Waits Across Calls

**Verdict:** REVISE

The rescoped story is well motivated and densely specified. Its Context
explains, with numbers, why per-call waiting rather than queue length is the
real risk, and most acceptance criteria are concrete Given/When/Then cases.
It is not ready to plan for three reasons. The vocabulary for "lock",
"call" and "holder" is overloaded. Two rules have no defined behaviour: the
admission threshold (the serving window) and how the 300 s expiry interacts
with the 900 s cap. The researcher-side re-presentation loop, which is what
makes tickets survive across calls, has no acceptance criterion.

### Cross-Cutting Themes

- **Serving window undefined** (flagged by: testability, clarity,
  completeness, scope) — admission and its acceptance criterion depend on an
  Open Question, so neither can be implemented or tested, and the story
  cannot be sized.
- **Round-duration cap left in limbo** (flagged by: completeness, scope,
  clarity) — the Drafting Notes call it open, the Requirements rule it out,
  and Open Questions does not list it.
- **Requirements without criteria** (flagged by: testability, completeness)
  — five requirements have no criterion: the researcher re-presentation loop,
  cross-run sharing, reuse of cached verdicts after mid-call exhaustion, no
  spawn cap, and OpenAlex/web unaffected.
- **Stale batch-cap identity** (flagged by: scope, dependency) — the file
  slug and 0283's plan and validation still describe a per-profile batch cap.

### Findings

#### Major

- 🟡 **Testability / Clarity / Completeness / Scope**: Admission depends on
  an undefined "serving window"
  **Location**: Requirements, Acceptance Criteria, Open Questions
  The admission requirement and its criterion hinge on a window that Open
  Questions has not settled. Any threshold would pass, and the story cannot
  be sized until it is decided.
- 🟡 **Testability**: Precedence between 300 s expiry and 900 s cap is
  unspecified
  **Location**: Requirements
  Take a ticket issued more than 900 s ago and last presented more than 300 s
  ago. One criterion says it rejoins at the back; the other says it returns
  `rate_limited`.
- 🟡 **Testability / Completeness**: Researcher re-presentation loop has no
  acceptance criterion
  **Location**: Acceptance Criteria
  Every criterion except the end-to-end run tests only the Rust side. The
  profile and agent could mishandle `waiting` and every unit-level criterion
  would still pass.
- 🟡 **Clarity**: Summary promises nodes are never lost, but the 900 s cap
  keeps a `lock_contention` failure path
  **Location**: Summary
  "Never lose nodes" and "a ticket past 900 s fails" are two different
  success bars.
- 🟡 **Clarity**: "Lock" names three different mechanisms
  **Location**: Requirements, Acceptance Criteria
  The word covers the shared pacing lock, the per-ticket `flock`, and an
  admitted call's whole-call hold. "When the lock frees" is ambiguous, and
  the item never says whether the existing gate survives or is replaced.
- 🟡 **Clarity**: "Call" and "holder" switch between one invocation and a
  ticket's lifetime
  **Location**: Requirements
  It is unclear when a node takes a new ticket rather than re-presenting one.
  "Holder" means both the researcher node and the OS process.

#### Minor

- 🔵 **Completeness / Testability**: Several requirements lack criteria
  **Location**: Acceptance Criteria
  No criteria cover cross-run sharing, OpenAlex/web being unaffected, the
  absence of a spawn cap, or what the new reason-row remedy says (the
  existing criterion is negative only).
- 🔵 **Completeness / Scope / Clarity**: Round-duration cap question is
  homeless and contradicts a Requirement
  **Location**: Drafting Notes, Open Questions
  Either list it as open or record that it was settled in favour of no cap.
- 🔵 **Testability**: Mid-call exhaustion criterion omits search-repeat and
  cache-reuse
  **Location**: Acceptance Criteria
  An implementation that re-runs cached confirmations would still pass.
- 🔵 **Testability**: End-to-end criterion relies on one stochastic run
  **Location**: Acceptance Criteria
  The baseline failure rate was 4 of 24, so one clean run is weak evidence.
  The criterion also does not say how non-contention failures count.
- 🔵 **Testability**: "Immediately" and "regardless of polling timing" lack
  bounds
  **Location**: Acceptance Criteria
  These need a tolerance and a way to drive polling order in tests.
- 🔵 **Testability**: Value of `waiting` position is undefined
  **Location**: Acceptance Criteria
  The item does not say whether absent tickets count or whether the position
  is 0- or 1-based.
- 🔵 **Clarity**: Start point of the 300 s resume window is ambiguous
  **Location**: Requirements
  The window could start when the last invocation began or when it ended,
  which differs by up to 100 s.
- 🔵 **Scope**: Whole-call admission is separable from cross-call tickets
  **Location**: Requirements
  Tickets alone may remove the losses. Admission adds backoff held under the
  lock and repeated searches; either split it out or justify bundling it.
- 🔵 **Scope**: Open design questions leave size unbounded
  **Location**: Open Questions
  The queue's on-disk form shapes liveness, expiry and cross-run ordering.
- 🔵 **Dependency**: 0283 recorded only as `relates_to`, not as a blocker
  **Location**: Dependencies
- 🔵 **Dependency**: arXiv API and its access terms not listed as an external
  dependency
  **Location**: Dependencies
- 🔵 **Dependency**: Claude Code Bash timeout and subagent re-invocation not
  listed as platform couplings
  **Location**: Technical Notes
- 🔵 **Dependency**: Consumers of the new `waiting` outcome not enumerated
  **Location**: Requirements
  `agents/researcher.md` is shared by every profile, so any caller that
  treats an unknown outcome as a failure would lose the node.

#### Suggestions

- 🔵 **Scope / Dependency**: Identity still describes the abandoned batch cap
  **Location**: Frontmatter / file name, Drafting Notes
  The slug, 0283's plan and 0283's validation all still say "per-profile
  batch cap".
- 🔵 **Clarity**: OAI and "pairs" used without definition
  **Location**: Context

### Strengths

- ✅ The Context explains its numbers: ~4.1 s per turn gives ~24 turns per
  100 s budget, which shows that per-call waiting is the binding constraint.
- ✅ The Drafting Notes trace the rescope from a planner cap to a fair queue
  and say why the earlier objection no longer holds.
- ✅ The criteria use named tickets (A, B, C), concrete thresholds (3 s,
  300 s, 900 s, 24 of 24) and explicit negative assertions ("makes no
  request").
- ✅ Scope boundaries are explicit: no spawn cap, default concurrency kept,
  OpenAlex and web unaffected.
- ✅ The Technical Notes name every touched component: `FilePacingGate`,
  `fetch_arxiv`, `CALL_BUDGET`, the arXiv profile, `agents/researcher.md` and
  the reason row.
- ✅ Holder-death liveness has its own criterion, separate from timed expiry.

### Recommended Changes

1. **Resolve the serving window** (addresses: admission depends on undefined
   serving window; open design questions leave size unbounded)
   Pick one definition, state it as a duration or formula where the term
   first appears, and give the admission criterion a paired at/below
   threshold case. Settle or time-box the on-disk form at the same time.
2. **Define the vocabulary once** (addresses: "lock" names three mechanisms;
   "call" and "holder" shift)
   Name the serving lock, the ticket liveness lock, the fetch invocation, the
   ticket and the ticket owner. State whether the existing `arxiv.lock`
   survives, and when a node takes a new ticket rather than re-presenting
   one. Use those names in every "when the lock frees" criterion.
3. **Specify the expiry/cap ordering** (addresses: precedence between expiry
   and cap; start point of the 300 s window)
   Say which check runs first, measure the 300 s from a named point, and add
   a criterion for the overlap.
4. **Add the missing criteria** (addresses: researcher re-presentation loop;
   several requirements lack criteria; mid-call exhaustion omits
   search-repeat)
   Cover the researcher loop (presents T again, writes no note while
   `waiting`), cross-run sharing, OpenAlex never returning `waiting`, no spawn
   cap, the positive remedy wording, and search-repeat with cache reuse.
5. **Align the Summary with the cap** (addresses: Summary promises nodes are
   never lost)
   Say that nodes are no longer lost within the 900 s per-ticket cap.
6. **Settle the round-duration cap** (addresses: round-duration cap question
   is homeless)
   Reword the Drafting Note as decided, or move it into Open Questions.
7. **Tighten the timing criteria** (addresses: "immediately" lacks bounds;
   `waiting` position undefined; end-to-end relies on one run)
   Bound liveness lag, define position semantics with an example, and back
   the attended run with a deterministic stress test.
8. **Fill in Dependencies** (addresses: 0283 as blocker; arXiv API;
   Claude Code platform coupling; consumers of `waiting`)
   List 0283 (satisfied once merged), arXiv search and OAI-PMH, the Bash
   timeout and subagent re-invocation, and every consumer of the fetch
   outcome.
9. **Decide on bundling whole-call admission** (addresses: whole-call
   admission is separable)
   Justify keeping it in this story or split it into a follow-up.
10. **Repair the batch-cap references** (addresses: identity still describes
    the abandoned batch cap; OAI and pairs undefined)
    Rename the slug or note the rescope in 0283's plan and validation; gloss
    OAI and "pair".

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is dense but mostly precise. The main problems
are three overloaded terms ("lock", "call", "holder") and two internal
inconsistencies: the Summary's "never loses nodes" against the 900 s cap
failure, and the Drafting Notes treating a cap as open when a Requirement
rules it out.

**Strengths**:
- The Context explains its numbers (~4.1 s per turn, ~24 turns per budget).
- Requirements name their actors, and outcomes are observable values.
- The Drafting Notes explain the mismatch between the file name and the
  title.
- Unresolved terms are openly listed in Open Questions.

**Findings**:
- 🟡 major / high — Summary: Summary promises nodes are never lost, but the
  Requirements keep a `lock_contention` failure path at 900 s. Align the
  Summary with the cap.
- 🟡 major / high — Requirements: "Lock" refers to three mechanisms (the
  pacing lock, the per-ticket `flock`, the whole-call hold). Give each a
  distinct name and say whether `arxiv.lock` survives.
- 🟡 major / medium — Requirements: "Call" and "holder" switch between one
  fetch invocation and a ticket's whole life. Define invocation, ticket and
  owner, and say when a new ticket is taken.
- 🔵 minor / high — Drafting Notes: they call a cap an open question that a
  Requirement already settles.
- 🔵 minor / medium — Requirements: the start point of the 300 s resume
  window is ambiguous.
- 🔵 minor / medium — Requirements: "serving window" is used as a firm
  condition while still undefined.
- 🔵 suggestion / medium — Context: OAI and arxiv "pairs" are used without
  definition.

### Completeness

**Summary**: The item is well populated for a story: the user story is
proper, the Context has a measured baseline, and the Requirements and
Acceptance Criteria are detailed. The gaps are small: some requirements have
no criterion, and one open question appears only in the Drafting Notes.

**Strengths**:
- The frontmatter is complete and valid.
- The Summary names the user, the need and the benefit.
- The Context gives the motivation and a measured baseline.
- The Requirements are detailed enough to start work.
- The Given/When/Then criteria include an end-to-end validation.
- The supporting sections are all populated, and the rescope is explained.

**Findings**:
- 🔵 minor / medium — Acceptance Criteria: several requirements have no
  matching criterion, notably the researcher re-presentation loop.
- 🔵 minor / high — Open Questions: the round-duration cap question from the
  Drafting Notes is missing.
- 🔵 suggestion / medium — Open Questions: the serving-window question
  underpins requirements but has no owner and no deadline.

### Dependency

**Summary**: The item is largely self-contained, with its couplings described
in the body, but Dependencies says "none". The gaps are bookkeeping: 0283 is
recorded only as `relates_to`, the arXiv service and the Claude Code timeout
appear only in prose, and 0283's documents still describe a batch cap.

**Strengths**:
- The Technical Notes name every touched component.
- The relationship to 0283 is recorded and referenced throughout.
- The OpenAlex and web boundaries are explicit.

**Findings**:
- 🔵 minor / high — Dependencies: prerequisite 0283 is recorded only as
  `relates_to`.
- 🔵 minor / high — Dependencies: the external arXiv service and its access
  terms are not captured.
- 🔵 minor / medium — Technical Notes: the Claude Code Bash timeout and
  subagent re-invocation are not captured as platform couplings.
- 🔵 minor / medium — Requirements: consumers of the new `waiting` outcome
  are not listed.
- 🔵 suggestion / medium — Drafting Notes: 0283's validation and plan still
  say 0295 delivers a batch cap.

### Scope

**Summary**: The story has one clear purpose and explicit boundaries, but it
is heavy. It bundles a FIFO ticket queue, whole-call admission, cross-call
persistence with expiry and a cap, a new outcome protocol, and a validation
re-run. Two design questions that shape it are still open.

**Strengths**:
- The rescope is deliberate and traceable.
- Boundaries are stated explicitly.
- The Summary, Requirements and Acceptance Criteria describe the same scope.
- It is clearly a follow-up increment under 0121 and related to 0283.

**Findings**:
- 🔵 minor / medium — Requirements: whole-call admission is a separable
  concern bundled with the cross-call ticket queue.
- 🔵 minor / medium — Open Questions: unresolved design questions leave the
  story's size unbounded.
- 🔵 minor / medium — Drafting Notes: the deferred round-duration cap
  question is not captured anywhere.
- 🔵 suggestion / high — Frontmatter / file name: the work item's identity
  still describes the abandoned batch-cap scope.

### Testability

**Summary**: Most criteria are clear Given/When/Then cases with concrete
thresholds. Three gaps remain: admission depends on an undefined serving
window, the precedence between expiry and the cap is unspecified, and
several Requirements have no criterion. The end-to-end criterion rests on
one stochastic run.

**Strengths**:
- Criteria use named actors that map directly to tests.
- Time thresholds are concrete.
- The end-to-end criterion reuses a defined baseline with a definitive pass
  condition.
- Negative assertions are explicit.
- Holder-death liveness has its own criterion.

**Findings**:
- 🟡 major / high — Acceptance Criteria: the admission criterion depends on
  an undefined "serving window".
- 🟡 major / high — Requirements: precedence between the 300 s expiry and
  the 900 s cap is unspecified.
- 🟡 major / high — Acceptance Criteria: the researcher re-presentation loop
  has no acceptance criterion.
- 🔵 minor / high — Acceptance Criteria: the mid-call exhaustion criterion
  omits search-repeat and cache reuse.
- 🔵 minor / medium — Acceptance Criteria: the end-to-end criterion relies on
  a single stochastic run.
- 🔵 minor / medium — Acceptance Criteria: "immediately" and "regardless of
  polling timing" lack verifiable bounds.
- 🔵 minor / medium — Acceptance Criteria: the expected value of the
  `waiting` position is not defined.
- 🔵 minor / high — Acceptance Criteria: cross-run sharing, unaffected
  sources, no spawn cap and the new remedy have no criteria.

## Re-Review (Pass 2) — 2026-10-05T21:22:23+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability / Clarity / Completeness / Scope**: Admission depends on an
  undefined serving window — Partially resolved (defined in Terms as one
  request plus its full retry schedule, but no duration or source is given)
- 🟡 **Testability**: Precedence between 300 s expiry and 900 s cap — Resolved
- 🟡 **Testability / Completeness**: Researcher re-presentation loop has no
  criterion — Resolved
- 🟡 **Clarity**: Summary promises nodes are never lost — Resolved
- 🟡 **Clarity**: "Lock" names three mechanisms — Resolved
- 🟡 **Clarity**: "Call" and "holder" shift meaning — Resolved
- 🔵 **Completeness / Testability**: Several requirements lack criteria —
  Resolved (a few smaller gaps remain; see new issues)
- 🔵 **Completeness / Scope / Clarity**: Round-duration cap question is
  homeless — Resolved
- 🔵 **Testability**: Mid-call exhaustion omits search-repeat and cache reuse
  — Resolved
- 🔵 **Testability**: End-to-end criterion relies on one stochastic run —
  Partially resolved (a stress test was added, but it is underspecified)
- 🔵 **Testability**: "Immediately" lacks bounds — Resolved
- 🔵 **Testability**: Value of `waiting` position undefined — Resolved
- 🔵 **Clarity**: Start point of the 300 s window — Resolved
- 🔵 **Scope**: Whole-call admission is separable — Resolved (rationale added)
- 🔵 **Scope**: Open design questions leave size unbounded — Resolved (the one
  remaining question is deferred to planning)
- 🔵 **Dependency**: 0283 recorded only as `relates_to` — Partially resolved
  (listed in the Dependencies prose, but not as a structured edge, and 0283
  still shows `ready`)
- 🔵 **Dependency**: arXiv API not listed — Resolved
- 🔵 **Dependency**: Claude Code platform couplings not listed — Resolved
- 🔵 **Dependency**: Consumers of `waiting` not enumerated — Resolved
- 🔵 **Scope / Dependency**: Identity still describes the batch cap — Still
  present (acknowledged in the Drafting Notes; file name and 0283 documents
  unchanged)
- 🔵 **Clarity**: OAI and "pairs" undefined — Partially resolved ("pair"
  glossed; OAI-PMH expanded but not explained)

### New Issues Introduced

- 🟡 **Clarity**: "Keeps the front" conflicts with skip-while-absent — after a
  ticket runs out of budget mid-call, it is unclear whether it reserves the
  serving lock until re-presented or only keeps its priority while later live
  tickets are served.
- 🟡 **Testability**: Stress-test criterion can pass without exercising the
  queue — latency and re-presentation are unspecified, and 30 `waiting`
  outcomes would satisfy "none returns `lock_contention`".
- 🟡 **Testability**: Live validation pass condition is ambiguous and need not
  exercise `waiting` — "all 24 notes" conflicts with "other causes judged
  separately", and no observed re-presentation is required.
- 🔵 **Testability**: "Spawns up to `concurrency`" is satisfied by any batch
  size, so a reinstated cap of 12 would pass.
- 🔵 **Testability**: The backoff criterion's precondition has no retry, so
  backoff may never be exercised.
- 🔵 **Testability**: The unserviceable-position criterion defines its
  precondition by its outcome.
- 🔵 **Clarity**: An unadmitted invocation exits on two different triggers:
  when remaining budget falls below one serving window, or when the budget is
  reached.
- 🔵 **Testability**: Gaps remain: expired tickets excluded from position, a
  killed invocation's ticket re-presented within 300 s, and the cap's effect
  on an invocation already waiting when its ticket crosses 900 s.
- 🔵 **Clarity**: Ticket states live, absent and unexpired are not in Terms.
- 🔵 **Clarity**: "Keeps no spawn cap" does not say whether a cap exists today.
- 🔵 **Dependency**: The validation run's need for live arXiv access is
  implied, not stated.
- 🔵 **Dependency**: Any effect on 0161's recursion eval is unconsidered
  (suggestion).
- 🔵 **Scope**: The story sits at the upper bound of one increment; the plan
  should split it into independently verifiable phases.
- 🔵 **Testability**: Time-based criteria imply an injectable clock
  (suggestion).
- 🔵 **Clarity**: The "Contention per fetch" row counts requests; withdrawal
  confirmation and liveness lag lack glosses (suggestions).

### Assessment

The first pass's structural problems are fixed: the vocabulary, the
expiry/cap ordering, the Summary wording, the researcher-loop criterion and
the dependencies. 3 new major findings keep the verdict at REVISE. One
design point needs a decision: what "keeps the front" means after a
mid-call budget run-out. The other two are criteria that the two empirical
tests can pass without exercising the queue. All three are narrow wording
fixes, not a rescope.

## Re-Review (Pass 3) — 2026-10-05T22:07:03+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: "Keeps the front" conflicts with skip-while-absent — Resolved
  (priority only; the serving lock is released)
- 🟡 **Testability**: Stress test can pass without exercising the queue —
  Resolved
- 🟡 **Testability**: Live validation pass condition ambiguous — Partially
  resolved (the pass condition is clear, but abnormal-latency reruns and
  other-cause losses are unbounded)
- 🔵 **Testability**: "Up to `concurrency`" — Resolved
- 🔵 **Testability**: Backoff precondition has no retry — Resolved
- 🔵 **Testability**: Unserviceable-position precondition circular — Resolved
- 🔵 **Clarity**: Two exit triggers — Resolved
- 🔵 **Testability**: Expired-in-position, killed re-presentation, cap during
  wait — Resolved
- 🔵 **Clarity**: Ticket states undefined — Resolved
- 🔵 **Clarity**: Spawn cap current state unclear — Resolved
- 🔵 **Dependency**: Live arXiv access implied — Resolved
- 🔵 **Testability**: Injectable clock — Resolved (the budget should also be
  settable; see new issues)
- 🔵 **Completeness / Clarity**: Serving window has no duration — Resolved
  (33 s; the full retry schedule was found to exceed the budget)
- 🔵 **Scope**: Story at upper bound of one increment — Still present
  (phasing deferred to the plan)
- 🔵 **Scope / Dependency**: Batch-cap identity in file name and 0283 docs —
  Still present
- 🔵 **Clarity**: "Contention per fetch" label, OAI-PMH and withdrawal glosses
  — Still present
- 🔵 **Dependency**: 0161 recursion eval unconsidered — Still present (not
  re-raised)

### New Issues Introduced

- 🟡 **Clarity**: "Fetch" means both a logical fetch and a `fetch`
  invocation, and what makes two invocations "the same fetch" is undefined
  (the ticket token alone, or the token plus the query).
- 🟡 **Testability**: No procedure is stated for verifying the LLM
  researcher's re-presentation and no-note behaviour; it could be signed off
  by reading the prose.
- 🟡 **Testability**: The live validation lets the verifier discard runs as
  "abnormal latency" and excuse unbounded other-cause losses.
- 🔵 **Testability**: "Served" has no observable meaning; the 1 s liveness
  bound can clash with 3 s spacing.
- 🔵 **Testability**: No boundary criterion for "backoff plus a serving
  window" before a retry.
- 🔵 **Clarity**: The end of a killed invocation, which expiry is measured
  from, is undefined.
- 🔵 **Clarity**: "Keeps its priority" lacks "unexpired"; the Summary's
  "waits past its 900 s cap" doesn't match the measured-from-issue rule.
- 🔵 **Clarity**: "Turn", "attempt" and "request" are used loosely.
- 🔵 **Clarity**: "Owner" is defined but unused; the researcher has several
  names.
- 🔵 **Completeness**: The re-presentation interface and the shape of the
  `waiting` outcome are unspecified.
- 🔵 **Dependency**: The research-topic reason row is missing from the
  consumers in Dependencies.
- 🔵 Suggestions: settable budget and stub payload in tests; name the
  validation document; `flock` release on process death as a platform
  assumption; attended-run cost; what `lock_contention` stays distinct from;
  the spawn-count criterion verifies a non-change.

### Assessment

Every pass-2 major is resolved or narrowed. The 3 new majors are a level
finer: one vocabulary split ("fetch") and two verification procedures. The
lenses now probe details the plan would normally settle. The design itself
has been stable since pass 2.

## Re-Review (Pass 4) — 2026-10-05T22:50:07+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity**: "Fetch" means both a logical fetch and an invocation —
  Resolved (request, attempt, fetch request and invocation are now separate
  terms; tickets are bound to their arguments)
- 🟡 **Testability**: No procedure for verifying researcher behaviour —
  Resolved (attended stub run with a transcript check; one run only, see new
  issues)
- 🟡 **Testability**: Live validation lets runs be discarded — Partially
  resolved (latency threshold and loss limit set, but an unre-presented
  `waiting` loss is classed as inconclusive and repeats are uncapped)
- 🔵 **Testability**: "Served" not observable — Resolved
- 🔵 **Testability**: No backoff-admission boundary — Resolved (38 s / 39 s)
- 🔵 **Clarity**: End of a killed invocation undefined — Resolved
- 🔵 **Clarity**: Priority and Summary cap wording — Partially resolved (the
  resume rule still lacks the cap exception)
- 🔵 **Clarity**: Turn, attempt and request loose — Resolved (but "lock turn"
  is now per-request; see new issues)
- 🔵 **Clarity**: Owner unused, several names for the researcher — Resolved
- 🔵 **Completeness**: Re-presentation interface and shape of `waiting` —
  Resolved (moved into the Open Question for planning)
- 🔵 **Dependency**: Reason row missing from consumers — Resolved
- 🔵 Suggestions from pass 3 (settable budget, validation document,
  `flock`, attended-run cost, `lock_contention` distinctness, spawn-count
  guard) — Resolved

### New Issues Introduced

- 🟡 **Testability**: The validation run classes a dropped `waiting`
  ticket, the defect under test, as inconclusive, and repeats are uncapped.
- 🔵 **Testability**: The attended researcher criterion rests on one run;
  the stub wiring is unstated.
- 🔵 **Testability**: The 300 s and 900 s boundaries are not pinned (exactly
  300 s / 301 s, 900 s / 901 s).
- 🔵 **Testability**: No criterion keeps arXiv throttling distinct from
  `lock_contention`.
- 🔵 **Testability**: The ticket-rejection outcome is unobservable.
- 🔵 **Clarity**: "Admitted" is undefined next to "served".
- 🔵 **Clarity**: "Lock turn" is per-request, but the lock is now held per
  invocation.
- 🔵 **Clarity**: The attended criterion says the first invocation
  "presents" T.
- 🔵 **Clarity**: "Frees its place immediately" vs "within 1 s".
- 🔵 **Clarity**: "Each further attempt" does not clearly cover the first
  attempt of later requests.
- 🔵 **Dependency**: 0161 is not in frontmatter `relates_to`; the fate of
  `arxiv-contention.log` is unstated.
- 🔵 **Completeness / Scope**: The 0283 annotation has no criterion; the 0161
  check could expand scope; whole-call admission may stand alone.
- 🔵 Suggestions: the OpenAlex/web criterion needs a contended precondition;
  "It" is ambiguous; 0161, "reason row" and OAI-PMH need glosses; mention the
  rescope near the top.

### Assessment

There are no critical findings and 1 major finding, which is below the
REVISE threshold of 2, so the verdict is COMMENT. The work item is
acceptable but could be improved. The major finding is a one-line fix to the
validation criterion and should be made before planning; the minors are
wording and boundary tightening.

## Verdict Change — 2026-10-05T22:51:50+00:00

**Verdict:** APPROVE

Changed from COMMENT by the reviewer after the pass-4 findings were applied
to the work item: the validation-run failure rule and repeat cap, the
pinned 300 s and 900 s boundaries, the throttling and rejection criteria,
and the wording and dependency fixes.
