---
type: "work-item-review"
id: "0245-invoke-accelerator-directly-in-skills-review-1"
title: "Work Item Review: Invoke Accelerator Directly In Skills"
date: "2026-09-05T23:48:43+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0245"
work_item_id: "0245"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-09-05T23:48:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Invoke Accelerator Directly In Skills

**Verdict:** REVISE

The item is well-structured and complete — every expected section is present
and substantively populated, the scope is a single coherent mechanical
conversion, and the motivation is well-grounded in the epic-0136 migration.
Two structural issues nonetheless warrant revision before implementation: an
internal contradiction between the absolute "no path prefix anywhere" rule and
the Assumptions' explicit-path fallback, and the lint-enforcement requirement
that overlaps a separate item (0107) without being recorded as a coordination
dependency. The acceptance criteria also under-verify the stated intent —
two of three forbidden call forms have no check, and the identical-output
guarantee rests on a tautological "no regressions" clause.

### Cross-Cutting Themes

- **Lint-enforcement ownership overlaps 0107** (flagged by: scope, dependency)
  — the lint rule is bundled into this mechanical conversion as a Requirement
  and Acceptance Criterion, yet the Drafting Notes admit it overlaps 0107 and
  Dependencies still records "Blocked by: none". Both lenses converge on the
  same fix: resolve ownership and record the coupling before implementing.
- **The absolute rule contradicts its own exception** (flagged by: clarity,
  testability) — Requirements/AC assert bare `accelerator` everywhere with a
  zero-match grep, while Assumptions permit a call site to keep the explicit
  path where PATH is unreliable, and no criterion tests the PATH precondition
  the whole change depends on. The definition of done is ambiguous and its key
  precondition is unverified.

### Findings

#### Major

- 🟡 **Clarity**: Absolute "no path prefix" rule contradicts the explicit-path
  fallback in Assumptions
  **Location**: Requirements / Acceptance Criteria vs Assumptions
  Requirements and AC state an absolute rule ("no path prefix, no absolute
  path") with a zero-match grep, while Assumptions say "if false for a context,
  that call site keeps the explicit path" — a retained path would fail the
  zero-match criterion and trip the lint. A reader cannot tell whether the
  target is "no prefix anywhere" or "no prefix except where PATH is unreliable".

- 🟡 **Dependency + Scope**: Lint-rule enforcement overlaps 0107 and is not
  recorded as a coordination dependency
  **Location**: Requirements / Dependencies
  The lint rule (Requirements bullet 5, AC #3) is bundled into this otherwise
  mechanical conversion, and the Drafting Notes flag that it overlaps 0107 and
  must be reconciled "before implementing". Dependencies still records "Blocked
  by: none" and lists 0107 only under "Relates to", so an implementer sees no
  gate and risks building a duplicate rule — the exact collision the notes warn
  of. Decide ownership and promote it to a blocking/coordination constraint.

- 🟡 **Testability**: Absolute-path and bash-wrapper variants have no defined
  verification procedure
  **Location**: Acceptance Criteria
  AC1 forbids three forms (the `${CLAUDE_PLUGIN_ROOT}/bin/` prefix, absolute
  paths, `bash` wrappers) but only the first has a check (AC2's grep). A
  verifier could pass AC2 with zero matches while absolute-path or bash-wrapped
  calls remain. Add grep/lint checks for `bin/accelerator` and `bash
  .*accelerator` that must also return zero matches.

- 🟡 **Testability**: "No invocation regressions" is unbounded and does not
  verify the identical-output guarantee
  **Location**: Acceptance Criteria
  Requirements assert "each converted call resolves to the same binary and
  produces identical output", but AC4 only asks the suite pass "with no
  invocation regressions" — a tautology if the suite does not exercise each
  converted call. State that the suite exercises every converted invocation, or
  add a before/after output-equality check.

#### Minor

- 🔵 **Clarity**: The word "bare" carries two meanings
  **Location**: Context / Summary / Dependencies
  Context and Dependencies use "bare-path" for the 0106 state (path retained,
  `bash` dropped); Summary and Requirements use "bare command" for this item's
  target (path removed). A reader tracking the migration could conflate the
  two. Distinguish the labels or add a one-line clarification.

- 🔵 **Dependency**: Core assumption rests on 0182 but 0182 is absent from
  Dependencies
  **Location**: Technical Notes
  Technical Notes cite 0182 as documenting that bare invocation still needs
  `CLAUDE_PLUGIN_ROOT` set — the item's central Assumption — yet 0182 appears
  in neither Dependencies nor `relates_to`. Add it as an informing link so the
  precondition source is visible.

- 🔵 **Testability**: AC1 relies on unspecified manual "inspection"
  **Location**: Acceptance Criteria
  AC1 defines the target state but leaves verification as manual "inspection"
  with no exhaustiveness guarantee across the file set. Reframe as a mechanical
  allowlist grep/lint asserting every invocation matches the bare form.

- 🔵 **Testability**: PATH-resolution precondition is untested by any criterion
  **Location**: Open Questions
  The Open Question flags that bare `accelerator` only resolves if
  `${CLAUDE_PLUGIN_ROOT}/bin` is on `PATH` in every context (main session and
  subagents), but no AC verifies runtime resolution there. All ACs could pass
  while a bare call silently fails in a subagent. Add a criterion that
  exercises a live bare invocation in each named context.

#### Suggestions

- 🔵 **Dependency**: No downstream Blocks entry for the epic-0136 convergence
  this completes
  **Location**: Dependencies
  Context frames this as completing the single-call-form convergence within
  epic 0136, but no downstream consumer (the epic milestone, a follow-on
  allowlist simplification) is listed as a Blocks entry. If something is
  genuinely waiting, name it; if not, the empty Blocks is correct.

### Strengths

- ✅ Structurally and informationally complete: every expected section is
  present and genuinely populated, with intact frontmatter and a recognised
  `kind: task`.
- ✅ Single unified purpose — all requirements describe one mechanical
  normalisation, with boundaries drawn explicitly (SKILL.md bodies in scope;
  hooks and the launcher bootstrap out).
- ✅ Upstream precedents (0106, 0167, 0212) are named and confirmed done, and
  the critical PATH / `CLAUDE_PLUGIN_ROOT` precondition is surfaced as both an
  Open Question and an Assumption with a stated fallback.
- ✅ AC2 and AC3 are concrete and runnable — an exact grep with a zero-match
  pass condition, and a lint check with a specified input and observable
  failure output.

### Recommended Changes

1. **Reconcile the absolute rule with the explicit-path exception** (addresses:
   Clarity contradiction; Testability PATH precondition) — either make the rule
   unconditional by requiring the PATH Open Question to resolve "yes" before the
   item proceeds, or enumerate the excepted contexts in Requirements and AC so
   the grep and lint account for them.

2. **Resolve lint ownership with 0107 and record the coupling** (addresses:
   Dependency + Scope lint overlap) — decide whether the lint rule lives here or
   in 0107; if here, note 0107 as superseded and promote the reconciliation into
   Dependencies as a coordination/blocking constraint; if there, drop the lint
   Requirement and AC #3 and defer to 0107.

3. **Extend the acceptance criteria to cover all three forbidden forms and the
   identical-output guarantee** (addresses: Testability absolute/bash variants,
   no-regression tautology, AC1 manual inspection) — add zero-match checks for
   `bin/accelerator` and `bash .*accelerator`, reframe AC1 as a mechanical
   allowlist check, and either assert the suite exercises every converted call
   or add a before/after output-equality check.

4. **Add 0182 as an informing dependency** (addresses: Dependency 0182 absent) —
   link 0182 in Dependencies and/or `relates_to` so the PATH-precondition source
   is visible alongside the Assumption it underwrites.

5. **Disambiguate "bare"** (addresses: Clarity "bare" overloaded) — use distinct
   labels for the 0106 "bash-free path invocation" state and this item's
   "no-path invocation" target.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is written in clear, consistent domain language and
its referents mostly resolve unambiguously. The main clarity problem is an
internal contradiction: Requirements and Acceptance Criteria assert an absolute
"every skill, no path prefix" rule, while Assumptions permit individual call
sites to retain the explicit path, leaving the intended scope unclear. A
secondary issue is the word "bare" carrying two distinct meanings.

**Strengths**:
- Domain terms (`${CLAUDE_PLUGIN_ROOT}`, `PATH`, launcher bootstrap, `!`
  preprocessor, `hooks.json`) are used consistently and well-grounded.
- The Summary's stated scope matches the Requirements and most Acceptance
  Criteria closely.
- Assumptions and Open Questions make the PATH-availability precondition
  explicit rather than implicit.

**Findings**:
- **Major** (high confidence) — Requirements/AC vs Assumptions: the absolute
  no-prefix rule with a zero-match grep contradicts the Assumptions' "keeps the
  explicit path" fallback; both cannot hold. Reconcile by making the rule
  unconditional or enumerating excepted contexts.
- **Minor** (medium confidence) — Context/Summary/Dependencies: "bare" means
  the path-retaining 0106 state in some places and the path-removed target in
  others; a reader could conflate them. Distinguish the labels.

### Completeness

**Summary**: This task work item is structurally and informationally complete.
All expected sections are present and substantively populated, and the
frontmatter is intact with a recognised kind. For a task, the work is clearly
and specifically defined, giving an implementer everything needed to start.

**Strengths**:
- Summary states the work as a single unambiguous action with rationale.
- Context fully explains the motivation and situates the change within epic
  0136 and its precedent items.
- Requirements are specific and actionable; four concrete Given/When/Then
  Acceptance Criteria are present.
- Frontmatter is complete and correct; Open Questions, Assumptions,
  Dependencies, and Drafting Notes are all genuinely populated.

**Findings**: none.

### Dependency

**Summary**: For a mechanical, self-contained conversion this item captures its
couplings unusually well: precedents are named and confirmed done, the
PATH/`CLAUDE_PLUGIN_ROOT` precondition is surfaced twice, and hooks are scoped
out with a reason. The one genuine gap is the lint-rule overlap with 0107 — a
coupling captured only in prose while Dependencies records "Blocked by: none".
A secondary gap is that the core assumption rests on 0182, which is not carried
into Dependencies.

**Strengths**:
- Upstream precedents (0106, 0167, 0212) are named and confirmed done,
  substantiating the "Blocked by: none" claim on the conversion itself.
- The critical PATH / `CLAUDE_PLUGIN_ROOT` precondition is captured as both an
  Open Question and an Assumption with a stated fallback.
- Hooks are deliberately excluded with the coupling reason given.

**Findings**:
- **Major** (medium confidence) — Dependencies: the lint enforcement (Req 5,
  AC #3) overlaps 0107 per the Drafting Notes and must be reconciled before
  implementing, but Dependencies shows no gate. Promote to a
  coordination/blocking constraint.
- **Minor** (medium confidence) — Technical Notes: the central Assumption rests
  on 0182's documented behaviour, yet 0182 is in neither Dependencies nor
  `relates_to`. Add it as an informing link.
- **Suggestion** (low confidence) — Dependencies: no downstream Blocks entry for
  the epic-0136 convergence this completes. Name it if something waits;
  otherwise the empty Blocks is correct.

### Scope

**Summary**: A well-scoped, coherent Task: every requirement serves the single
purpose of normalising skill-body invocations to bare `accelerator`, and the
boundaries are drawn explicitly (SKILL.md bodies only; hooks and launcher
bootstrap out). Summary, Requirements, and Acceptance Criteria describe the same
scope. The only scope tension is the bundled lint-enforcement requirement,
which overlaps separate item 0107.

**Strengths**:
- Single unified purpose — all requirements describe one mechanical
  normalisation.
- Boundaries stated explicitly; in-scope and out-of-scope surfaces are clear.
- Task kind is appropriate for a repo-local, no-behavioural-change conversion.
- No drift between Summary, Requirements, and Acceptance Criteria.

**Findings**:
- **Minor** (medium confidence) — Requirements: the lint rule bundles 0107's
  arguable deliverable into this conversion; the conversion delivers value
  independently. Decide whether the rule belongs here or in 0107 before
  implementing.

### Testability

**Summary**: For a task, the Acceptance Criteria are largely concrete — AC2
gives an exact grep and AC3 specifies a lint input and expected failure output.
The main gaps: two of three forbidden invocation forms (absolute paths, `bash`
wrappers) have no verification procedure, and the central "identical output"
guarantee is backed only by a loosely-worded "no invocation regressions" clause.

**Strengths**:
- AC2 gives an exact, runnable command with a definitive pass condition.
- AC3 specifies both input and observable outcome, admitting a concrete
  pass/fail test.
- Forbidden and required call forms are enumerated explicitly.

**Findings**:
- **Major** (high confidence) — Acceptance Criteria: absolute-path and
  bash-wrapper variants have no defined check; AC2 alone could pass while they
  remain. Add zero-match greps for `bin/accelerator` and `bash .*accelerator`.
- **Major** (medium confidence) — Acceptance Criteria: "no invocation
  regressions" (AC4) is unbounded and tautological; it does not verify the
  identical-output guarantee unless the suite exercises each converted call.
  State that it does, or add an output-equality check.
- **Minor** (medium confidence) — Acceptance Criteria: AC1 relies on manual
  "inspection" with no exhaustiveness guarantee. Reframe as a mechanical
  allowlist check.
- **Minor** (medium confidence) — Open Questions: the PATH-resolution
  precondition is untested by any criterion; all ACs could pass while a bare
  call fails to resolve in a subagent. Add a live-invocation criterion per
  context.

## Re-Review (Pass 2) — 2026-09-05

**Verdict:** APPROVE

Re-ran the four lenses that had findings (clarity, dependency, scope,
testability). All eight original pass-1 findings are resolved. The pass itself
surfaced two new major findings in testability — both introduced by the pass-1
AC4/AC5 rewrite — which were corrected in place immediately after this pass, so
no major findings remain against the current work item.

### Previously Identified Issues

- 🟡 **Clarity**: Absolute "no path prefix" rule vs explicit-path fallback — Resolved. The bare form is now stated as unconditional across Requirements, Assumptions, and the Open Question, with no per-call-site fallback.
- 🟡 **Dependency + Scope**: Lint-rule overlap with 0107 — Resolved. Dependencies now records an explicit "Owns / supersedes 0107" coordination entry; scope re-review downgraded this to a "no change required" suggestion.
- 🟡 **Testability**: Absolute-path and bash-wrapper variants unverified — Resolved. AC2 now greps all three forbidden forms for zero matches.
- 🟡 **Testability**: "No invocation regressions" tautology — Partially resolved then corrected. The pass-1 rewrite replaced it with an unverifiable coverage claim (see New Issues); the corrective edit reframes AC4 as binary resolve/exit with a same-binary rationale and trims AC5 to a plain regression gate.
- 🔵 **Clarity**: "bare" overloaded — Resolved. Context and Dependencies now distinguish "bash-free path invocation" (0106) from "no-path" (this item).
- 🔵 **Dependency**: 0182 absent — Resolved. 0182 added to Dependencies, `relates_to`, and References as the PATH-precondition source.
- 🔵 **Testability**: AC1 manual inspection — Resolved. AC1 reframed as a mechanical allowlist lint.
- 🔵 **Testability**: PATH precondition untested — Resolved. A per-context live-invocation criterion was added (AC4).

### New Issues Introduced

- 🟡 **Testability**: AC4 comparison lacked a defined baseline/procedure — Corrected. Narrowed to "resolves and exits successfully"; output equality now follows by construction (same on-`PATH` binary), removing the need for a captured baseline.
- 🟡 **Testability**: AC5 embedded an unverifiable coverage claim ("suite exercises every converted invocation") — Corrected. Trimmed to a plain "`mise run check` and tests pass with no regressions" gate; AC1–AC3 carry the conversion-specific verification.
- 🔵 **Clarity / Testability**: `bin/accelerator` parenthetical mislabelled "absolute-path variants" and the pattern set was under-specified — Corrected. AC2 now reads "`/bin/accelerator` path suffix (catching fully expanded absolute-path renderings)" with the three patterns pinned.
- 🔵 **Clarity**: "subagent" (singular) in AC4 vs "subagents" in the Open Question — Corrected. AC4 now reads "subagents".
- 🔵 **Dependency**: downstream allowlist consumer named generically — Addressed. The Blocks entry now annotates it as an anticipated, not-yet-tracked consumer to be linked by id once raised.

### Assessment

The work item is ready for implementation. Every pass-1 finding is resolved, and
the two major AC regressions the re-review surfaced were corrected in place. The
residual scope observations (bundled lint, precondition coupled to execution)
were both rated "no change required" by the scope lens and reflect a deliberate,
defensible bundling.
