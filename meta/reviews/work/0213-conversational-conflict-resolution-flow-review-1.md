---
type: work-item-review
id: "0213-conversational-conflict-resolution-flow-review-1"
title: "Work Item Review: Conversational Conflict Resolution Flow for Sync"
date: "2026-08-17T11:17:18+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
parent: "work-item:0171"
target: "work-item:0213"
work_item_id: "0213"
relates_to: ["work-item-review:0171-jira-and-linear-integrations-review-1"]
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: [skills, sync, conflicts]
last_updated: "2026-08-17T12:20:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Conversational Conflict Resolution Flow for Sync

**Verdict:** REVISE

0213 is the best-scoped child by a distance — one coherent flow in one SKILL.md
body, gated by none of 0171's prerequisites, independently valuable, correctly
identified as landing first. Every lens said so. Its problems are all about
verification and standalone readability: the manual walkthrough that is its only
behavioural check names no fixture path, no injection seam and no pass predicate
beyond field presence; exit `0` is described as "clean" yet required to carry
unresolved lines, and is never exercised; and it is the only child with neither
Context nor Assumptions, so its motivation and its one size-bounding unknown are
both discoverable only from the parent.

This review was conducted as one pass over all four children of 0171. The
cross-child critical — the three port-less bridge capabilities landing in no
child — is stated in full in 0212's review and does not touch this child.

### Findings

#### Major

- 🟡 **Testability**: The manual walkthrough specifies no fixture, no injection
  seam and no pass predicate
  **Location**: Acceptance Criteria
  The only behavioural criterion names no path for the two-conflict fixture
  report, no content for it (which fields, which conflict shapes, which ids), and
  — critically — no mechanism by which the SKILL body's `accelerator work sync`
  invocation yields that fixture and the stated exit code instead of a live run.
  The child simultaneously claims it needs no credentialed target, so the seam (a
  stub binary on `PATH`, an `ACCELERATOR_*` override, a recorded transcript) is
  load-bearing and unspecified. Nothing asserts the re-invocation it constructs
  is actually *accepted* by the binary, so a malformed `--resolve` id would pass
  both the static template check and the walkthrough. Two verifiers can reach
  opposite verdicts.

- 🟡 **Clarity / Testability**: Exit `0` is described as clean yet required to
  carry unresolved lines, and is never exercised
  **Location**: Requirements / Acceptance Criteria
  The Requirements demand reading `unresolved` lines "on exits `0`, `4` and `71`
  alike" and in the same sentence say "`sync` exits `0` clean". The walkthrough
  then runs only `4` and `71`. An implementer cannot tell whether exit `0` can
  genuinely carry conflicts — in which case "clean" is wrong and a case is
  missing — or whether reading on `0` is merely defensive.

- 🟡 **Completeness**: No Assumptions section, so the child's one size bound is
  unstated
  **Location**: Assumptions
  Both substantive criteria assume 0194's shipped conflict report already carries
  all six render fields on all three exit codes. Dependencies only says to
  confirm the artefacts *exist*, not that they carry those fields. Its siblings
  each carry a ⚠️-marked unconfirmed assumption naming the consequence; this
  child states nothing equivalent. If the report omits a field — the title or
  either timestamp are the plausible candidates — the child stops being a
  SKILL.md-only change and grows into 0194's binary, invalidating its stated
  independence and its ability to land first.

#### Minor

- 🔵 **Testability**: `mise run` exits 0 is the only non-manual criterion, and it
  is unrelated to the added behaviour — a SKILL.md body edit passes it whether or
  not the conflict loop works. The child ships with no automated regression
  guard, and 0212 edits the same file.
  **Location**: Acceptance Criteria

- 🔵 **Completeness / Clarity**: No Context section; the motivation sits as a
  subordinate clause inside a Dependencies bullet, and the child does not state
  its position in the parent's ordering or why it is high priority under a
  medium-priority parent.
  **Location**: Context

- 🔵 **Completeness**: The walkthrough criterion does not name where its evidence
  is recorded, though 0171's `## Decisions` already holds a matching *pending*
  entry awaiting exactly this.
  **Location**: Acceptance Criteria

- 🔵 **Dependency**: The sole prerequisite carries no edge — `relates_to: 0194`
  only, no `blocked_by` — while 0194's record as visible from this workspace reads
  `ready`. The contingency instruction lives only in the parent, and this child
  declares no `blocks` despite discharging one of 0171's criteria outright.
  **Location**: Dependencies

- 🔵 **Clarity**: "Six fields" versus seven enumerated values — the Requirements
  list id, title, differing field, local value, remote value "and both
  timestamps", and say "at least", while the criterion asserts an exact six.
  **Location**: Acceptance Criteria

### Strengths

- ✅ Genuinely well-scoped: one conversational flow, one file, no crate, no
  fixture corpus, no deletion — the only child every lens called right-sized.
- ✅ Its Dependencies affirmatively records what it is *not* gated by (no
  credentialed target, no client crate, none of the three Open Questions), which
  is a stronger statement than an empty section.
- ✅ Correctly identified as landing first, with the reason given — the conflict
  report is unactionable today.
- ✅ The static half of its verification is a real mechanical check: the
  invocation template, the three exit codes, the named render fields.
- ✅ The overlap with 0212 on `sync-work-items/SKILL.md` is anticipated and
  localised rather than left as a hidden collision.
- ✅ It records the `--resolve` token set and the conflict-carrying exit codes as
  corrected against `cli/work-cli/src/cli.rs`, so the invocation shape is sourced
  rather than assumed.

### Recommended Changes

1. **Specify the walkthrough completely** (addresses: the verification major)
   Name the fixture path and its two conflict records, name the injection seam and
   the exact command sequence, state the pass predicate as a checklist (six fields
   present per conflict, one prompt per item, one `--resolve <id>=<choice>` per
   collected choice with ids matching the fixture), and assert the constructed
   re-invocation is accepted by `accelerator work sync` rather than rejected as a
   usage error.
2. **Settle exit `0`** (addresses: the contradiction) Either add a third
   walkthrough case with an empty `unresolved` set and the expected outcome
   stated — report no conflicts, issue no re-invocation — or drop `0` from the
   static assertion if a clean run genuinely carries no report.
3. **Add an Assumptions section** with a ⚠️-marked entry: the existing conflict
   report already carries all six fields on all three exit codes, to be confirmed
   against the report format before planning, with the consequence named.
4. **Add an automated guard** (addresses: no regression protection) A test
   asserting the SKILL body contains the `--resolve <id>=<remote|local|skip>`
   template and the three exit codes, wired into the existing skills test lane, so
   the static half survives as a guard rather than a one-off inspection —
   particularly since 0212 edits the same file.
5. **Add a short Context** stating the problem inline (a detected conflict cannot
   be resolved because the binary is non-interactive), its position in the
   ordering, and that it can land first.
6. **Record the evidence location in 0171's `## Decisions`**, add the concrete
   0194 artefact check with the action if it fails, add 0212 to `relates_to`, and
   reconcile the six-versus-seven field count.

## Per-Lens Summaries

- **Clarity**: the exit-`0` contradiction and the field-count mismatch; motivation
  reachable only through the parent.
- **Completeness**: proportionate to its size, but missing Context and Assumptions
  where its siblings carry both, and the second hides a real size bound.
- **Dependency**: correctly records what does *not* gate it; its one real
  prerequisite carries no edge.
- **Scope**: the best-scoped child in the set — no findings against its
  boundaries.
- **Testability**: the weakest verification of the four, and the only child whose
  sole automated criterion is unrelated to what it delivers.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-08-17

**Verdict:** REVISE

All five lenses re-ran over the four children as a set. **No criticals.** The
pass-1 critical is resolved and independently confirmed: the completeness lens
verified that every obligation the parent's Scope narrative and `## Decisions`
register still names resolves to a child requirement or criterion, including the
three port-less bridge capabilities now owned by 0212.

The verdict holds on major count. Three of the majors are defects the fix round
introduced; the rest are pre-existing gaps the tightened criteria exposed.

### Previously Identified Issues

- 🟡 → 🟡 **The walkthrough specified no fixture, seam or pass predicate** —
  **Largely resolved, one item self-defeating.** Three named fixtures, a
  stub-on-`PATH` seam and a four-part checklist now exist. But the predicate's last
  item — that the re-invocation be "accepted by `accelerator work sync` rather
  than rejected as a usage error" — cannot fail under the stated seam, because the
  stub *is* `accelerator work sync` and validates nothing.
- 🟡 → ✅ **Exit `0` described as clean yet required to carry unresolved lines** —
  **Resolved**; the flow branches on whether the report carries `unresolved`
  lines, `0` is stated to carry none, and a `clean-exit-0.txt` fixture exercises
  it.
- 🟡 → ✅ **No Assumptions section** — **Resolved**; the six-field premise is
  recorded as ⚠️ unconfirmed with its consequence named.
- 🔵 → ✅ **No automated guard** — **Resolved**; the static half is now an
  automated skills-lane assertion, explicitly because 0212 edits the same file.
- 🔵 → ✅ Context, the evidence location, the 0194 artefact check with its
  remedial edge, and the `relates_to` link — **all resolved**.

### New Issues Introduced

- 🟡 **Testability**: **The stub seam prevents the one predicate item that would
  catch a malformed `--resolve` template** (introduced by the fix round). This is
  the most likely defect in a `SKILL.md`-only change, and the check for it cannot
  fail. The fix is to split the predicate: render/prompt/emit against the stub,
  argument acceptance against the real binary with the stub off `PATH`.
- 🟡 **Clarity**: **The loop's unit alternates between "conflict" and "item".**
  Requirements say "collect a choice per item"; the predicate says "one prompt per
  conflict" and one `--resolve` order per choice, while `--resolve` is keyed by
  work-item id. These coincide only if a work item can carry at most one
  conflicting field, and the fixtures — two conflicts with *distinct* ids — avoid
  the case where they diverge. One reading silently drops a choice.
- 🔵 **Clarity / Completeness**: "The walkthrough" is introduced with a definite
  article, no antecedent and no actor — a committed harness, a documented manual
  procedure, or an automated test are all consistent with the text, and they imply
  different scope.
- 🔵 **Clarity**: Exit `71`'s meaning is never stated — only that a `71` run "may
  carry conflicts alongside its failure" — and its fixture's predicate is
  identical to exit `4`'s, so the two test the same path under two codes.
- 🔵 **Testability**: The walkthrough's evidence is a Decisions entry with no
  artefact at a named path, unlike 0210's contract-run criterion which requires a
  committed evidence file.
- 🔵 **Scope**: This child is `priority: high` and fixes live degradation, yet sits
  inside an epic held at `draft` by three Open Questions it does not depend on.
  Either state in 0171 that it may be picked up regardless, or re-parent it to
  0136.

### Assessment

Still the best-scoped child, and every pass-1 finding was addressed. The one
regression is instructive: adding the stub seam made the walkthrough deterministic
and simultaneously neutered its sharpest assertion. The conflict-versus-item unit
question is the more consequential open point — it decides whether the flow can
silently drop a user's choice.

## Acceptance — 2026-08-17

**Accepted.** Verdict changed from REVISE to APPROVE by Toby Clemson, and the
target work item moved to `ready`.

The open findings recorded above were **accepted rather than resolved**. They are
not withdrawn and remain the record of what is known to be imperfect; they carry
into planning rather than blocking it. Specifically still open at acceptance:

- The three Open Questions on 0171 — the credentialed target's secrets siting, the
  fate of the three port-less bridge capabilities, and the `EXIT_CODES.md` siting —
  plus the two ⚠️ size-bounding assumptions in 0211 and 0212.
- Three self-contradictions introduced during the fix rounds: 0211's mock-server
  deletion being simultaneously unconditional and deferrable, 0211's `jq`/`curl`
  requirement asserting a survivor set its own criterion says the child cannot
  reach, and 0213's stub-on-`PATH` seam defeating the one predicate item that would
  catch a malformed `--resolve` template.
- Two latent gaps better closed by implementation than by further specification:
  the non-port provider surface (five of eight flows) being owned by neither 0210
  nor 0211, and 0210 carrying no criterion for HTTP-status or GraphQL error
  classification or auth.

Rationale for accepting rather than iterating: across four review passes the
severity ceiling fell (3 criticals → 1 → 0 → 0) while the major count did not
converge, and each fix round introduced two or three new majors of one shape — a
requirement updated without its criteria, or the reverse. The two correctness traps
that mattered (the unowned port-less capabilities, and the `work-item-sync-label.sh`
ordering break) are both closed. Further speculative specification was judged less
valuable than starting 0210 and discovering the real shape of the client crates.
