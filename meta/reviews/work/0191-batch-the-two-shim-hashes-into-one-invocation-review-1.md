---
type: "work-item-review"
id: "0191-batch-the-two-shim-hashes-into-one-invocation-review-1"
title: "Work Item Review: Batch the bootstrap's two shim hashes into one sha256 invocation"
date: "2026-08-22T21:14:53+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0191"
parent: "work-item:0136"
work_item_id: "0191"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: ["shell", "performance", "bootstrap", "bash-3.2"]
last_updated: "2026-08-22T22:32:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Batch the bootstrap's two shim hashes into one sha256 invocation

**Verdict:** REVISE

The item is structurally excellent — every section present and substantively populated, an atomic and correctly-kinded task, and unusually thorough dependency mapping. Two testability gaps push it to REVISE: the change's own raison d'être (a warm-path saving) has no pass threshold, and the cross-backend correctness the Requirements mandate is confirmed nowhere in the Acceptance Criteria. A clarity gap compounds them — the measurement notation `median(G)`/`median(B)` and the `C5` label carrying the whole evidence argument are never defined in this item or its References.

### Cross-Cutting Themes

- **The measurement narrative is under-specified** (flagged by: clarity, testability) — `G`/`B` and `C5` are undefined, the measurement AC records figures without gating them, and cross-backend behaviour is required but never asserted. The Context's evidence argument and the ACs that verify it both lean on notation and thresholds that aren't pinned down.
- **Standalone value is contingent on siblings 0215/0216** (flagged by: scope, dependency) — the second Open Question asks whether the residual is worth removing once 0215 (removes a warm-path hash) and 0216 (cuts digest cost) are weighed. Both lenses want that turned from an open question into an explicit sequencing decision a scheduler can act on.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Testability**: Cross-backend confirmation required but not covered by any criterion
  **Location**: Acceptance Criteria
  Requirements mandate confirming the batched output format and missing-file exit semantics on Apple, GNU coreutils, and `shasum`, and the first Open Question flags it verified only on Apple — yet no AC asks a verifier to confirm the parse on GNU coreutils (the backend the linux CI lane resolves) or `shasum`. The digest-to-path assignment correctness hinges on this.

- 🟡 **Testability**: Performance criterion records figures but sets no pass threshold
  **Location**: Acceptance Criteria
  The measurement AC only requires that before/after figures be recorded — no threshold such as `after < before`. Unlike 0186's analogous `after ≤ 0.5 × before`, the item could be marked done even if the after-median were unchanged or worse, so the stated saving is never gated.

- 🟡 **Clarity**: `median(G)` and `median(B)` used throughout without definition
  **Location**: Context
  `G` and `B` carry the entire measurement argument — the evidence table, `G ≤ 1.1 × B`, the AC5 ratio, the Assumptions — but are never expanded, and the listed References don't define them either (the definition lives in 0169, only a `relates_to` link). An inverted reading flips the pass/fail conclusion.

#### Minor

- 🔵 **Dependency**: 0205 is the baseline source but absent from the Dependencies enumeration
  **Location**: Dependencies
  0205 supplies the entire measured baseline (the `median` figures, the 1.3260 ratio, the 0.747 ms shortfall, `warm-dispatch-3.json`) and appears in frontmatter `relates_to` and throughout Context, but the Dependencies section's relates-to list (0186, 0169, 0189, 0215, 0216) omits it.

- 🔵 **Clarity**: Current net position buried under three stacked dated layers
  **Location**: Context
  The original claim, a "Retracted 2026-08-13" block, and an "Amended 2026-08-17" block each supersede the prior, but the retracted conclusion ("not a latency-gate co-requisite at all") remains in the present tense. A scanning reader can lift a superseded conclusion and act on the opposite of the item's current standing.

- 🔵 **Testability**: Cold-run "behaves as today" lacks a defined expected outcome
  **Location**: Acceptance Criteria
  "A cold run … behaves as today" verifies against an implicit baseline rather than a stated exit code and output. "No spurious diagnostic" is measurable, but "behaves as today" gives no captured reference, so two verifiers could disagree on what counts as spurious.

- 🔵 **Clarity**: "Consistent with the 7.05 ms row once netted off" asserts a relationship the figures don't show
  **Location**: Context
  Netting the stated 2.02 ms baseline off the 7.05 ms row yields ~5.03 ms, not the 3.824 ms quoted, and the two figures come from different sessions — so the arithmetic behind "consistent" isn't visible to the reader. (Low confidence.)

#### Suggestions

- 🔵 **Scope**: Re-measurement AC carries a secondary objective tied to 0189's threshold
  **Location**: Acceptance Criteria
  AC5 and its framing weave in tightening 0189's C5 from 1.4 back to 1.3 "on evidence." The item states reaching 1.3 is not a pass condition, so nothing is bundled, but attaching the rationale to another item's tuning could invite scope creep. Consider tracking the 0189 tightening as a follow-up on 0189.

- 🔵 **Scope**: Standalone value is contingent on sibling items 0215/0216
  **Location**: Open Questions
  If a sibling lever supersedes the saving, the task could be delivered for a benefit that no longer exists. Resolve the sequencing against 0215/0216 before scheduling, or explicitly gate this item behind them.

- 🔵 **Dependency**: Potential redundancy with 0215/0216 raised but not resolved into a sequencing decision
  **Location**: Open Questions
  If 0215 removes one of the two warm-path hashes, this item's premise — batching *two* hashes — is partly superseded. Record an explicit ordering note in Dependencies (gated on the 0215/0216 decision, or intentionally independent) rather than leaving it as an open question.

- 🔵 **Clarity**: "C5 threshold" referenced without definition
  **Location**: Acceptance Criteria
  `C5` (presumably 0189's fifth criterion, the `median(G)/median(B)` ratio) is never expanded and 0189 is not in References. Expand it on first use or add 0189 to References.

### Strengths

- ✅ The Summary is self-contained and unambiguous about the core change — batching two `sha256_file` calls into one `sha256sum f1 f2`, with an explicit assurance both digests are still computed and compared.
- ✅ Exceptionally complete: every expected section present and substantively populated, with intact frontmatter (kind=task, status=draft, priority, parent, relates_to, tags all recognised).
- ✅ Genuinely atomic and correctly kinded — every requirement serves the one optimisation named in the title, with in/out boundaries stated explicitly (what 0186 declined, what 0215/0216 own).
- ✅ Unusually thorough dependency mapping — the 0186 trace seam, the `measure:warm-dispatch` harness, the backend/environment coupling, and the not-a-blocker relationship to 0189 are all captured.
- ✅ AC1 and AC2 are strongly testable — one backend fork and zero `awk` from a `bash -x` trace, and three named regression tests that must stay green.
- ✅ AC5 uses an explicit ⚠️ callout to pre-empt the likely misreading of what is and is not a pass condition.

### Recommended Changes

1. **Add a cross-backend confirmation criterion** (addresses: Cross-backend confirmation required but not covered by any criterion)
   Add an AC requiring the multi-file output format and missing-second-input exit behaviour to be confirmed on GNU coreutils — and on `shasum` if the batched form is used there — with the observed output recorded, mirroring how AC4 records the resolved backend.

2. **Gate the measurement criterion** (addresses: Performance criterion records figures but sets no pass threshold)
   Add a pass condition to the before/after measurement AC, e.g. "the after-median is strictly less than the re-measured before-median on the resolved backend", keeping the absolute delta recorded-but-not-gating if host variance is a concern.

3. **Define the measurement notation** (addresses: `median(G)`/`median(B)` undefined; "C5 threshold" referenced without definition)
   Expand `G` and `B` on first use in Context (what each path is), expand `C5` on first use (0189's ratio criterion), and add 0189 to the References section so the definitions are reachable from this item.

4. **Add a current-position line to Context** (addresses: Current net position buried under three stacked dated layers)
   Lead Context with a one- or two-line settled-position statement, keeping the dated retraction/amendment blocks below as history, and reword the retracted present-tense conclusion so it can't be lifted as current.

5. **Record 0205 and the 0215/0216 sequencing in Dependencies** (addresses: 0205 absent from the Dependencies enumeration; 0215/0216 redundancy not resolved; standalone value contingent on siblings)
   Add 0205 to the Dependencies relates-to bullet as the baseline source, and add an explicit ordering note stating whether this item is gated on the 0215/0216 decision or intentionally independent of it.

6. **Pin the cold-run expected outcome** (addresses: Cold-run "behaves as today" lacks a defined expected outcome)
   State the concrete expected cold-run exit code and the absence of any stderr line referencing the missing second input, or require a captured before/after diff of the cold-run output.

7. **Reconcile or reword the "consistent" claim** (addresses: "Consistent with the 7.05 ms row once netted off")
   Show the reconciliation explicitly (which figures net against which, why the sessions are comparable) or reword to state the measurements are from different sessions and only broadly agree.

## Per-Lens Results

### Clarity

**Summary**: Precise about the core change and names concrete tests, files, and follow-ups, but the measurement narrative leans on undefined single-letter referents (`median(G)`, `median(B)`) and an undefined `C5` label that a reader cannot resolve from this item or its References. Internally chronologically consistent, yet Context stacks a retraction on an amendment on the original claim, making the current net position hard to extract in one read. Actor/outcome clarity in Requirements and Acceptance Criteria is strong.

**Strengths**:
- Self-contained, unambiguous Summary with an explicit both-digests-still-compared assurance.
- Related items 0215, 0216, 0205 glossed at point of first use.
- AC5's ⚠️ callout disambiguates what is/isn't a pass condition.
- Requirements and Technical Notes name exact guard tests and file paths.

**Findings**:
- 🟡 major (high) — Context: `median(G)`/`median(B)` used throughout without definition. The symbols carry the whole measurement argument; References don't define them (definition lives in 0169, only a relates_to link); an inverted reading flips the pass/fail conclusion. Suggestion: define `G`/`B` on first use or link 0169.
- 🔵 minor (medium) — Context: current net position buried under three stacked dated layers; the retracted "not a latency-gate co-requisite at all" remains present-tense and can be lifted as current. Suggestion: add a one/two-line current-position statement atop Context, history below.
- 🔵 suggestion (medium) — Acceptance Criteria: "C5 threshold" referenced without definition; 0189 not in References. Suggestion: expand `C5` on first use or add 0189 to References.
- 🔵 minor (low) — Context: "consistent with the 7.05 ms row once netted off" — netting 2.02 ms off 7.05 ms yields ~5.03 ms not 3.824 ms, and figures come from different sessions. Suggestion: show the reconciliation or reword to "broadly agree".

### Completeness

**Summary**: Exceptionally complete for a task kind — every expected section present and substantively populated, frontmatter intact with a recognised kind and appropriate status. Content fully satisfies what a task demands; an implementer could start without follow-up. No completeness gaps of consequence.

**Strengths**:
- Summary states the work as a single unambiguous action with mechanism and expected saving.
- Context thoroughly explains motivation with provenance, dated retraction/amendment trail, and measured figures on a named host.
- Acceptance Criteria covers fork count, the three named tests, cold-run behaviour, before/after measurement, re-measurement, and lint/build gates.
- Requirements name exact tests and backends; Technical Notes enumerate the two viable shapes plus bash-3.2 constraints.
- Frontmatter complete and correct.

**Findings**: None.

### Dependency

**Summary**: Exceptionally well dependency-mapped — the 0186 `run_bootstrap` trace seam, the `measure:warm-dispatch` harness, the backend/environment coupling per CI lane, and the downstream 0189 C5 relationship are all explicit, and the item states it blocks nothing and is not a blocker. Minor gaps: 0205 (source of the baseline figures) is omitted from the Dependencies enumeration, and the potential redundancy against 0215/0216 is raised as an open question rather than resolved into a sequencing decision.

**Strengths**:
- Coupling to 0186's test infrastructure explicit and precise.
- Downstream 0189 relationship captured with unusual clarity (not a blocker; enables tightening C5 on evidence).
- External sha256-backend coupling named with the ~3× cost swing and the confirm-per-lane instruction.
- Re-measurement coupling captured as an explicit AC tied to `measure:warm-dispatch`.
- Parent 0136 and the relates-to set enumerated with what 0215/0216 each change.

**Findings**:
- 🔵 minor (high) — Dependencies: 0205 is the baseline source but absent from the Dependencies enumeration. Suggestion: add 0205 as the baseline warm-dispatch measurement source.
- 🔵 suggestion (medium) — Open Questions: potential redundancy with 0215/0216 raised but not resolved into a sequencing decision. Suggestion: record an explicit ordering note (gated, or intentionally independent).

### Scope

**Summary**: Well-scoped, genuinely atomic task — collapse two shim-hash invocations into one sha256 call. All Requirements and Acceptance Criteria serve that one purpose, the `task` kind fits, and in/out boundaries are clear. The only scope-adjacent tensions are the secondary "tighten 0189's threshold on evidence" objective threaded through the item and the open question about whether the work is worth doing once 0215/0216 land.

**Strengths**:
- Single coherent purpose; no bundling of independent concerns.
- Appropriately sized and correctly kinded — a low-priority ~2.5 ms micro-optimisation is a natural task.
- Boundaries stated explicitly (in: the two shim hashes; out: 0186's exec probe, 0215's cache-hit hash, 0216's digest-cost cut).

**Findings**:
- 🔵 suggestion (medium) — Acceptance Criteria: re-measurement AC carries a secondary objective tied to 0189's threshold. Low delivery risk today, but the coupling could invite scope creep. Suggestion: track the 0189 tightening as a follow-up on 0189.
- 🔵 suggestion (medium) — Open Questions: standalone value contingent on 0215/0216. Suggestion: resolve the sequencing before scheduling, or explicitly gate behind those items.

### Testability

**Summary**: Most Acceptance Criteria are strongly testable — the fork-count criterion anchored to a concrete `bash -x` trace seam, three named regression tests, binary lint/build gates. Main weaknesses: the cross-backend confirmation named in Requirements has no corresponding AC, and the performance measurement criterion records figures without any pass/fail threshold, so the item's stated saving is never gated.

**Strengths**:
- AC1 defines a concrete observable pass/fail (one fork, zero awk) via the 0186 seam.
- AC2 names the three specific regression tests guarding the trust-boundary invariant.
- Lint and build criteria are binary and mechanically verifiable.
- Criteria framed as recorded checks and regression guards, appropriate for a task.

**Findings**:
- 🟡 major (high) — Acceptance Criteria: cross-backend confirmation required (GNU coreutils, shasum) but covered by no criterion; correctness hinges on the digest-to-path assignment across backends. Suggestion: add an AC confirming the format and missing-input exit behaviour on GNU coreutils (and shasum), with output recorded.
- 🟡 major (medium) — Acceptance Criteria: measurement criterion records figures but sets no pass threshold, unlike 0186's `after ≤ 0.5 × before`; the item could be done with an unchanged or worse after-median. Suggestion: add a gating condition (`after < before` on the resolved backend).
- 🔵 minor (medium) — Acceptance Criteria: cold-run "behaves as today" lacks a defined expected outcome (no captured exit code/output baseline). Suggestion: state the expected exit code and absence of the missing-input stderr line, or require a before/after diff.

---

## Re-Review (Pass 2) — 2026-08-22

**Verdict:** COMMENT

Re-ran the four lenses that had findings in pass 1 (clarity, dependency, scope, testability); completeness was clean and skipped. Every pass-1 finding is resolved except the one scope suggestion the author deliberately kept (AC5/AC6 0189-tightening rationale). No critical findings and only one major, so the item is acceptable as-is — the new findings are refinements surfaced by the pass-1 edits, most notably one genuine correctness-verification gap worth folding in.

### Previously Identified Issues

- 🟡 **Clarity**: `median(G)`/`median(B)` undefined — **Resolved** (defined on first use in Context, inheritance from 0169 stated).
- 🔵 **Clarity**: current net position buried — **Resolved** (settled-reading line leads Context; dated blocks labelled history).
- 🔵 **Clarity**: `C5` referenced without definition — **Resolved** (expanded as 0189's fifth criterion; 0189 added to References).
- 🔵 **Clarity**: "consistent … netted off" arithmetic — **Resolved** (reworded to different-session, broadly-agree).
- 🟡 **Testability**: cross-backend confirmation not covered by an AC — **Resolved** (cross-backend AC added).
- 🟡 **Testability**: measurement AC set no pass threshold — **Resolved** (after-median strictly less than before-median, delta recorded-not-gated).
- 🔵 **Testability**: cold-run "behaves as today" undefined — **Resolved** (pinned to same-exit-status + no new missing-input stderr, before/after diff).
- 🔵 **Dependency**: 0205 absent from Dependencies — **Resolved** (added, with `warm-dispatch-3.json` named).
- 🔵 **Scope / Dependency**: 0215/0216 contingency unresolved — **Resolved** (Dependencies → Sequencing note: intentionally independent).
- 🔵 **Scope**: AC re-measurement carries the 0189-tightening objective — **Still present** (deliberately kept by author decision; framing confirmed correct — re-measurement gated, 1.3 outcome not).

### New Issues Introduced

- 🟡 **Testability** (major, medium): the missing-input digest **mis-assignment** risk the item names as load-bearing is not gated — AC3 checks exit status and stderr, not that the surviving source digest is assigned to the source path. A batched impl could mis-assign on the one-input-missing path and still pass every criterion.
- 🔵 **Testability** (minor): the cross-backend AC records observed output but states no pass condition; the expected shape (one `<digest>  <path>` line per input in argument order, sane missing-input exit) lives only in Open Questions.
- 🔵 **Testability** (suggestion, low): single-session `after < before` may be noise-sensitive given ~2.48 ms saving against several-ms session drift; specify sample size or allow a re-run on a tie.
- 🔵 **Dependency** (minor, medium): 0189 is `done`, so the C5-tightening this item's evidence enables has no tracked downstream home (no Blocks entry or named follow-up).
- 🔵 **Dependency** (suggestion, low): AC4/AC6 environmental prerequisites (the 0205 harness, GNU-coreutils CI lane) are implied inline, not surfaced as dependencies.
- 🔵 **Clarity** (minor, high): Drafting Notes says "AC5's 0189-tightening rationale" but that rationale lives in the sixth AC bullet, not the fifth (criteria are unnumbered checkboxes).
- 🔵 **Clarity** (minor, medium): superseded ceiling (1.3) and three shortfall figures (~2.4 / 5.98 / 0.747 ms, different baselines) coexist in Context, disambiguated only by date and target.
- 🔵 **Clarity** (suggestion): "attempt 2 (invalid)" is not explained; "this host" is used before it is named as darwin-arm64.

### Assessment

The item is ready for planning at COMMENT. Two of the new findings are worth folding in before implementation: the **mis-assignment verification AC** (the only major — it gates the item's own stated correctness risk) and the trivial **Drafting Notes "AC5" → sixth-bullet** correction (a factual reference error introduced by the pass-1 edit). The remaining clarity items are properties of the deliberately-retained measurement history and are optional polish; the 0189-no-tracked-home dependency point is a real lifecycle question but sits outside this item's own body.

### Approval (2026-08-22)

Verdict raised COMMENT → **APPROVE** by author decision after applying the two recommended edits: the missing-input digest mis-assignment acceptance criterion (closing the sole major) and the Drafting Notes reference correction. The remaining items are optional polish on the retained measurement history and the 0189 lifecycle follow-up, none of which block planning.

---
*Review generated by /accelerator:review-work-item*
