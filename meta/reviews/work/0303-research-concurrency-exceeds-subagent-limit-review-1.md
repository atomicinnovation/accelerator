---
type: "work-item-review"
id: "0303-research-concurrency-exceeds-subagent-limit-review-1"
title: "Work Item Review: Default Research Concurrency Exceeds the Subagent Limit"
date: "2026-10-10T20:14:48+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0303"
work_item_id: "0303"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 4
tags: []
last_updated: "2026-10-10T20:49:50+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Default Research Concurrency Exceeds the Subagent Limit

**Verdict:** REVISE

The work item is complete and internally consistent. The reproduction, the
clamp rules and the verbatim strings are precise enough to implement without
asking the author anything. Its weak point is the capacity-refusal retry
path: the condition for continuing is ambiguous, nothing exercises the path
behaviourally, and the path couples to harness behaviour and to sibling 0310
without recording either in Dependencies. Four major findings exceed the
threshold of two, so the verdict is REVISE.

### Cross-Cutting Themes

- **Retry path is under-specified and unverified** (flagged by: clarity,
  testability, scope) — the continuation condition's "them" does not fit the
  first retry. No procedure triggers a capacity refusal. The path is about
  half the acceptance criteria and could ship separately.
- **Rerun recovery is unverified** (flagged by: testability, completeness,
  dependency) — the Assumptions mark it "Not yet verified", yet the
  exhaustion path relies on it. Sibling 0310 asserts the same behaviour.
- **"No warning names the env var" cannot fail** (flagged by: testability,
  clarity) — the only defined warning never names the env var.
- **Harness version is not stated where it is needed** (flagged by:
  dependency, completeness) — the cap from v2.1.217, the parser and the
  refusal text appear only in Assumptions, not in Dependencies or the
  reproduction.

### Findings

#### Major

- 🟡 **Clarity**: The retry-continuation condition has an unclear referent and does not fit the first retry
  **Location**: Requirements
  "For as long as the previous round had at least one of them accepted" is
  either vacuous or never true on the first retry. If the original batch was
  fully refused, it is unclear whether any retry happens.
- 🟡 **Dependency**: Claude Code harness cap, parser and refusal message are an uncaptured external dependency
  **Location**: Dependencies
  The fix depends on the cap of 20, the env-var parser, the v2.1.217
  introduction and the exact refusal text, yet Dependencies says "none
  known". A change to the refusal text would quietly disable the retry
  backstop.
- 🟡 **Testability**: Retry and exhaustion behaviour has no procedure that triggers it
  **Location**: Acceptance Criteria
  Four criteria cover refusals, but the only behavioural check is a manual
  reproduction where, once the clamp is in place, no refusal should occur. A
  prose rule the orchestrator ignores would still pass every check.
- 🟡 **Dependency**: Uncaptured coupling with sibling 0310 over step 5 failure classification
  **Location**: Dependencies
  0310 redefines the same step 5 check and failure-reason table to add a
  halting-failure class. Capacity exhaustion needs an agreed classification
  and an order for landing the two items.

#### Minor

- 🔵 **Clarity**: "Step 4", "step 5" and "step 5's refusal case" depend on SKILL.md numbering the item never describes
  **Location**: Requirements
  The item never says what those steps do or what the existing refusal case
  covers.
- 🔵 **Clarity**: It is unclear where a failure is recorded, and how that relates to the re-plan's `unfinished` entry
  **Location**: Acceptance Criteria
  "Not recorded a second time" has two readings.
- 🔵 **Clarity**: The exhaustion reason names a different cause from the one the retry path was meant to cover
  **Location**: Drafting Notes
  The Drafting Notes cite a lowered cap, but the reason string says
  "another subagent or fork". "Fork" is undefined.
- 🔵 **Testability**: The invalid-env-var criterion checks for a warning nothing in the spec can emit
  **Location**: Acceptance Criteria
  The check is always true. A case with the knob at 24 and the env var at
  `abc` would prove the value was ignored.
- 🔵 **Testability**: The manual reproduction passes without proving anything if batches never exceed 20
  **Location**: Acceptance Criteria
  Carry over the precondition "batches exceed 20 spawns" from Requirements.
- 🔵 **Testability**: Documentation criterion has no fixed content to check against
  **Location**: Acceptance Criteria
  List the required facts, or pin them with a structural test.
- 🔵 **Testability**: Recovery by rerunning `conduct` is unverified and has no criterion
  **Location**: Assumptions
  If rerunning does not re-offer the spawn, exhausted subtrees are lost.
- 🔵 **Completeness**: Reproduction does not state which Claude Code version shows the bug
  **Location**: Requirements
  The cap exists only from v2.1.217.
- 🔵 **Dependency**: Reversals of decisions in done items 0282/0283 not characterised in Dependencies
  **Location**: Dependencies
  This item supersedes 0283's default of 24 and overrides 0282's no-upper-cap
  rule for concurrency.
- 🔵 **Dependency**: Possible downstream consumers of the changed spawn-step contract are unnamed
  **Location**: Dependencies
  0161's evals and 0307's "current offered batch" may assume single-message
  batches.
- 🔵 **Scope**: Retry protocol is separable from the default/clamp fix
  **Location**: Requirements
  The one-line default fix is tied to the more involved retry design.

#### Suggestions

- 🔵 **Testability**: Missing boundary case: configured value equal to the cap
  **Location**: Acceptance Criteria
- 🔵 **Testability**: Flag-clamp criterion leaves the env-var state unstated
  **Location**: Acceptance Criteria
- 🔵 **Completeness**: Unverified assumption has no Open Questions section to track it
  **Location**: Assumptions
- 🔵 **Scope**: Kind 'bug' understates the new behaviour being introduced
  **Location**: Frontmatter: kind
- 🔵 **Scope**: Exactly replicating the harness's env-var parser widens scope
  **Location**: Requirements
- 🔵 **Clarity**: Orchestration jargon and harness terms are used without definition or link
  **Location**: Context

### Strengths

- ✅ The bug has all four reproduction parts, plus evidence from a real run
  (5 of 7 batches each had 4 spawns refused on 1.24.0-pre.74).
- ✅ The clamp decision table has concrete Given/When/Then criteria,
  including parser edge cases (`007`, `+8`, ` 20 `, `1e2`, `1,000`).
- ✅ The warning and the exhaustion reason are given verbatim, so structural
  tests can pin them.
- ✅ Scope boundaries are explicit: breadth and depth stay unbounded,
  `outstanding --limit` is untouched, and the env var stays out of the CLI.
- ✅ Technical Notes list every artefact to touch, including the golden
  fixture and the pinned catalogue test.
- ✅ Drafting Notes record the user decisions behind the clamp location and
  the retry scope.

### Recommended Changes

1. **Rewrite the retry-continuation rule** (addresses: retry-continuation
   referent; failure recording)
   State that the first re-issue always happens. Then continue while the
   latest re-issue round had at least one spawn accepted. Name where the
   failure is recorded, and say the re-plan's `unfinished` entry is not
   logged again.
2. **Add a forced-refusal manual reproduction** (addresses: retry has no
   trigger; rerun recovery unverified)
   Hold slots with background subagents before running `conduct`, with one
   partial variant and one fully held variant. Add a rerun-recovery
   criterion, or verify recovery now.
3. **Populate Dependencies** (addresses: harness dependency; 0310 coupling;
   0282/0283 reversals; downstream consumers)
   Add an External entry for the Claude Code harness: the cap, the parser,
   the refusal text, and the versions (v2.1.217 and later, verified on
   2.1.296). Add 0310 with an order and a classification (capacity
   exhaustion is not halting). Record that this item supersedes 0283 and
   0282 for concurrency. Check 0161 and 0307.
4. **Tighten the acceptance criteria** (addresses: always-true warning
   check; weak manual reproduction; docs criterion; boundary; flag env
   state)
   Make each check one that can fail, carry over the >20 precondition, list
   the required doc facts, add an equal-to-cap case, and state the env-var
   state in the flag criterion.
5. **Name steps by function and define terms** (addresses: step numbering;
   jargon; exhaustion reason cause)
   Bring the reason string in line with the Assumptions' cause list. Define
   "fork", "full batch", "harness" and "ultracode", and point to 0283 for
   the ledger terms.
6. **Decide on bundling** (addresses: retry separable; kind)
   Either split the retry into a sibling, or add a Drafting Note on why the
   three parts ship together.

## Per-Lens Results

### Clarity

**Summary**: The work item is precise for a bug report. Its Summary,
Requirements and Acceptance Criteria tell one consistent story. The
ambiguities are in the retry path: the continuation rule's "them", the step
numbering, how failures are recorded, and the exhaustion reason's cause.
Some project and harness jargon is undefined.

**Strengths**: The sections agree on scope. The env-var parsing rule is
pinned with examples. When the clamp warns depends on where the value came
from. The strings are given verbatim.

**Findings**:
- major/high — Requirements — The retry-continuation condition has an
  unclear referent and does not fit the first retry.
- minor/high — Requirements — "Step 4", "step 5" and "step 5's refusal
  case" depend on SKILL.md numbering the item never describes.
- minor/medium — Acceptance Criteria — It is unclear where a failure is
  recorded, and how that relates to the re-plan's `unfinished` entry.
- minor/medium — Drafting Notes — The exhaustion reason names a different
  cause from the one the retry path was meant to cover. "Fork" is
  undefined.
- minor/medium — Acceptance Criteria — The "no warning names the env var"
  assertion implies a warning the item never defines.
- suggestion/medium — Context — Orchestration jargon (run ledger, offered,
  re-plan, full batch, harness, ultracode) is undefined.

### Completeness

**Summary**: Very complete for a bug. Every section has real content, and
the frontmatter is valid. The gaps are the Claude Code version missing from
the reproduction and one unverified assumption that nothing tracks.

**Strengths**: A four-part reproduction. A Context that explains the cause.
A requirement split into four parts, each mapped to criteria. A detailed
Assumptions section. Drafting Notes that record decisions.

**Findings**:
- minor/medium — Requirements — Reproduction does not state which Claude
  Code version shows the bug.
- suggestion/medium — Assumptions — Unverified assumption has no Open
  Questions section to track it.

### Dependency

**Summary**: Parent and related items are recorded. Dependencies says "none
known" despite real coupling with the Claude Code harness and with sibling
0310. No upstream blocker stops work starting.

**Strengths**: The harness version and parser are recorded. Clamping in the
skill bounds the harness coupling. All file-level touch points are listed.

**Findings**:
- major/high — Dependencies — Claude Code harness cap, parser and refusal
  message are an uncaptured external dependency.
- major/medium — Dependencies — Uncaptured coupling with sibling 0310 over
  step 5 failure classification.
- minor/medium — Dependencies — Reversals of decisions in done items
  0282/0283 are not characterised.
- minor/low — Dependencies — Possible downstream consumers (0161 evals, 0307
  batch semantics) are unnamed.

### Scope

**Summary**: A coherent unit with explicit boundaries. It bundles three
separable fixes, and the bug label understates the new behaviour. Both are
sizing points, not delivery risks.

**Strengths**: Explicit in and out boundaries. Justified bundling. A single
component. Consistent scope across sections.

**Findings**:
- minor/medium — Requirements — Retry protocol is separable from the
  default/clamp fix.
- suggestion/medium — Frontmatter: kind — Kind "bug" understates the new
  behaviour being introduced.
- suggestion/low — Requirements — Exactly replicating the harness's env-var
  parser widens scope.

### Testability

**Summary**: Highly testable for the clamp, with concrete values and
verbatim strings. The retry path is verified only structurally, and the
manual reproduction never triggers a refusal.

**Strengths**: A complete reproduction. Clamp criteria that cover the
decision table. Mechanical parser cases. Verbatim strings. A named test file.

**Findings**:
- major/high — Acceptance Criteria — Retry and exhaustion behaviour has no
  procedure that triggers it.
- minor/high — Acceptance Criteria — The manual reproduction passes without
  proving anything if batches never exceed 20.
- minor/medium — Acceptance Criteria — The invalid-env-var criterion checks
  for a warning nothing in the spec can emit.
- minor/medium — Acceptance Criteria — Documentation criterion has no fixed
  content to check against.
- minor/medium — Assumptions — Recovery by rerunning `conduct` is unverified
  and has no criterion.
- suggestion/medium — Acceptance Criteria — Missing boundary case:
  configured value equal to the cap.
- suggestion/low — Acceptance Criteria — Flag-clamp criterion leaves the
  env-var state unstated.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-10-10T20:31:04+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity**: Retry-continuation condition — Resolved
- 🟡 **Dependency**: Harness contract uncaptured — Resolved
- 🟡 **Testability**: Retry has no triggering procedure — Resolved
  (partial-hold and full-hold manual runs)
- 🟡 **Dependency**: 0310 coupling — Resolved (mirrored on 0310)
- 🔵 **Clarity**: Step numbering — Resolved
- 🔵 **Clarity**: Failure recording and `unfinished` — Resolved
- 🔵 **Clarity**: Exhaustion reason cause — Partially resolved ("fork"
  still unglossed)
- 🔵 **Testability**: Always-true env-var warning check — Resolved
- 🔵 **Testability**: Weak manual reproduction — Resolved
- 🔵 **Testability**: Docs criterion unpinned — Resolved
- 🔵 **Testability**: Rerun recovery unverified — Resolved
- 🔵 **Completeness**: Claude Code version missing — Resolved
- 🔵 **Dependency**: 0282/0283 reversals — Resolved in prose; frontmatter
  still lists them as relates_to only
- 🔵 **Dependency**: Downstream consumers — Partially resolved (0307
  added; edge not mirrored on 0307)
- 🔵 **Scope**: Retry separable — Still present by decision (bundling
  justified in Drafting Notes)
- 🔵 **Scope**: Kind "bug" — Still present by decision
- 🔵 **Scope**: Parser exactness — Resolved (justified in Drafting Notes)

### New Issues Introduced

- 🟡 **Testability**: Behavioural criteria have no verification procedure
  beyond prose pinning — about ten parse/clamp Given/When/Then criteria are
  exercised by no manual run.
- 🔵 **Completeness / Clarity**: Failure-reason table edit mentioned only in
  Dependencies — no requirement says what row this item adds.
- 🔵 **Scope / Clarity / Dependency**: Ownership of the 0307 query change is
  unassigned, and 0307 does not reference 0303.
- 🔵 **Testability**: Manual-run preconditions depend on undefined setup
  (fixture briefs, holder recipe).
- 🔵 **Testability**: Partial-hold expectation depends on unstated
  batch-return timing.
- 🔵 **Testability**: Manual runs lack named observation points.
- 🔵 **Clarity**: "Without waiting" and "round" are undefined relative to
  the original batch.
- 🔵 **Clarity**: "Behavioural criteria" is not defined.
- 🔵 **Completeness**: Changelog and golden fixture updates appear only in
  Technical Notes.
- 🔵 Suggestions: unqualified "SKILL.md"; structural pins lack anchor
  phrases; reproduction omits "no other subagents running"; "valid
  resolved value" wording; harness jargon glosses.

### Assessment

All four pass-1 major findings are resolved. One new major finding
remains: most parse and clamp criteria are verified only by prose pinning.
That is acceptable if stated as an accepted risk, or closable with a short
manual resolution matrix. The work item is acceptable for planning; the
minor findings are worth a short tidy-up pass, chiefly the failure-reason
table requirement and the 0307 ownership.

## Re-Review (Pass 3) — 2026-10-10T20:41:47+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability**: Behavioural criteria verified only by prose pinning —
  Resolved (knob-resolution manual run plus a stated accepted risk)
- 🔵 **Completeness / Clarity**: Failure-reason table edit — Resolved
- 🔵 **Scope / Clarity / Dependency**: 0307 ownership — Resolved (0307
  owns it; mirrored on 0307)
- 🔵 **Testability**: Undefined manual-run setup — Partially resolved
  (holder recipe and hold-run briefs still unproven)
- 🔵 **Testability**: Batch-return timing — Resolved (stated assumption)
- 🔵 **Testability**: Observation points — Partially resolved (offered
  spawns and `--spawned` ordering not captured)
- 🔵 **Clarity**: "Without waiting", "round", "behavioural criteria" —
  Resolved
- 🔵 **Completeness**: Changelog and golden fixture — Resolved
- 🔵 **Dependency**: 0282/0283 supersession only in prose — Still present
  by decision (done items left untouched)
- 🔵 **Scope**: Retry bundled with the default fix — Still present by
  decision

### New Issues Introduced

- 🟡 **Clarity**: Context says 0283 defines the run ledger, offered
  spawns, re-plan, `outstanding --spawned` and `unfinished`; the 0283 work
  item does not. They are defined in the 0283 plan
  (`meta/plans/2026-09-26-0283-recursive-finding-deepening.md`).
- 🟡 **Testability**: The full-load run can pass without ever issuing a
  full batch of 20, so it may not exercise the original trigger.
- 🔵 **Testability**: Structural token pins (`Do not retry`, `at least
  one`, `silently`) pass on prose that inverts the rule.
- 🔵 **Testability**: Hold runs lack a fixture-validation step and a named
  brief.
- 🔵 **Clarity**: Bare "refused" still overloads capacity refusal and agent
  refusal; the step 5 bullet mixes existing state with the change.
- 🔵 **Clarity**: The exhaustion reason names one cause while Assumptions
  name two (held slots, parser divergence).
- 🔵 **Clarity**: "Never offers a spawn twice" reads as contradicting
  rerun recovery; it holds within one run's ledger.
- 🔵 **Dependency**: How rounds interact with 0310's halting failures (a
  usage-limit failure mid-round) is unowned.
- 🔵 **Dependency**: The full-load brief is reachable only through issue
  #140.
- 🔵 **Completeness**: No Open Questions section.
- 🔵 **Scope**: Hold-run verification depends on unproven holder tooling.
- 🔵 Suggestions: reproduction step 4 should name the brief; knob-resolution
  runs should state "no override" and an abort point; the 0307 closing
  sentence is ambiguous.

### Assessment

All pass-2 issues are resolved or deliberately retained. The two new major
findings each need a one-paragraph fix: repoint the glossary to the 0283
plan, and require the full-load run to show a batch that offered exactly 20
spawns. The verdict is REVISE on the count threshold, not on structural
weakness; the item is close to ready.

## Re-Review (Pass 4) — 2026-10-10T20:49:50+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity**: Ledger terms attributed to 0283 — Resolved (points to
  the 0283 plan)
- 🟡 **Testability**: Full-load run can pass vacuously — Resolved (a batch
  of exactly 20 is required, else invalid)
- 🔵 **Testability**: Loose structural tokens — Resolved (verbatim
  sentences)
- 🔵 **Testability**: Hold-run fixture validation — Partially resolved
  (pre-run probe only; mid-run holder exit undetected)
- 🔵 **Clarity**: "Refused" overload — Partially resolved (criteria still
  use bare "refused" and "the batch returns")
- 🔵 **Clarity**: Exhaustion reason names one cause — Resolved
- 🔵 **Clarity**: "Never offers a spawn twice" — Resolved
- 🔵 **Dependency**: 0310 halting interaction — Partially resolved (rule
  recorded but owned by landing order, and lives only in Dependencies)
- 🔵 **Completeness**: No Open Questions — Resolved (holder fallback still
  unnamed)

### New Issues Introduced

- 🟡 **Testability**: Several parser cases cannot distinguish a correct
  parse — `=1e2`, `1,000`, `20.5` and ` 20 ` with no override all yield 20
  whether parsed correctly or not.
- 🔵 **Testability**: `--concurrency 0` cannot observe validation-before-
  clamp order; `50.5` can.
- 🔵 **Testability**: The accepted-risk statement overstates the pins
  (equal-to-cap, raised cap, trimming, fallback unpinned).
- 🔵 **Testability**: Failure-table and configure-doc pins assert less
  than the Requirements specify.
- 🔵 **Clarity**: Halting-stopped spawns would carry the capacity reason,
  which misstates their cause.
- 🔵 **Clarity**: Probe lifetime and stop-file actor implicit; an
  unfinished probe breaks the partial-hold sequence.
- 🔵 **Dependency**: The External entry omits further harness behaviours
  the item relies on (preprocessor env visibility, return timing,
  ultracode exemption); 0304 also edits the failure-reason table.
- 🔵 **Scope**: Retry could ship separately; the hold-run fixture question
  should gate only the retry half.
- 🔵 Suggestions: one line per knob-resolution configuration; changelog
  placement ambiguous.

### Assessment

The pass-3 major findings are resolved. The one remaining major finding is
narrow and mechanical: pair ignored env values with an override above 20,
and use ` 8 ` for trimming, so a misparse is observable. The work item is
acceptable for planning. Findings are converging on verification-detail
refinements rather than specification gaps.
