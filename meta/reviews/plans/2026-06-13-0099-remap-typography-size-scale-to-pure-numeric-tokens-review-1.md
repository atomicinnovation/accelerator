---
type: plan-review
id: "2026-06-13-0099-remap-typography-size-scale-to-pure-numeric-tokens-review-1"
title: "Plan Review: Remap Typography Size Scale To Pure-Numeric Tokens Implementation Plan"
date: "2026-06-13T17:15:51+00:00"
author: Toby Clemson
producer: review-plan
status: complete
target: "plan:2026-06-13-0099-remap-typography-size-scale-to-pure-numeric-tokens"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [correctness, code-quality, architecture, test-coverage, documentation, standards, usability]
review_number: 1
review_pass: 2
tags: [visualiser, design-tokens, typography, refactor, adr]
last_updated: "2026-06-13T19:13:51+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Remap Typography Size Scale To Pure-Numeric Tokens Implementation Plan

**Verdict:** REVISE

This is an unusually rigorous, well-researched plan: the mechanical core of
the rename (the px×10 mapping, the delimiter-anchored substring-collision
strategy, the two documented find-replace traps, and the reliance on the
auto-tracking `var()`-resolves oracle) is sound, and every agent independently
confirmed the cited line numbers, the single ADR-0036 reference, and the AC
grep semantics against the real tree. The plan does not earn an APPROVE for two
reasons that recur across lenses: (1) the ADR supersession modelling — a *full*
supersession of ADR-0036 — orphans ADR-0036's transitive supersession of
ADR-0026 and collides with the immutability lifecycle the plan itself cites
(0091 "amending" an accepted ADR-0043 is not permitted); and (2) the
value-preservation guarantee ("AC4 by construction") has no executable oracle
beyond ~17 pinned Playwright selectors, so a wrong-but-valid token swap or a
duplicated value typo across the 19-token mapping ships green. None of the
findings is critical, and several majors have cheap, high-leverage fixes (a
one-line px×10 decode assertion; a prose clause in ADR-0043).

### Cross-Cutting Themes

- **ADR supersession lineage & immutability** (flagged by: architecture,
  documentation, standards) — Full supersession of ADR-0036 produces two
  related gaps. ADR-0036 holds the only writable record of its supersession of
  ADR-0026's typography clauses; transitioning it to `superseded` freezes that
  edge inside a superseded node, and ADR-0043 does not re-assert it — so the
  live governance of ADR-0026's typography rows becomes undiscoverable from the
  graph. Separately, the plan repeatedly says 0091 will later *amend* ADR-0043,
  but ADR-0031 permits content edits only on `proposed` ADRs — an accepted
  ADR-0043 can only be superseded, not amended.

- **Value preservation is not machine-enforced** (flagged by: test-coverage,
  correctness) — AC4 byte-identity rests on a manual line-by-line mapping check
  plus AC3 routing. But a find-replace landing a consumer on the *wrong but
  still-declared* token (`--size-xs`→`--size-145` instead of `--size-140`), or
  the *same* value typo duplicated across `global.css` and the `tokens.ts`
  mirror, passes the oracle, parity, the AC5 ratchet, and every AC grep — caught
  only if the surface is one of ~17 selectors pinned in the resolved-size
  Playwright spec. For ~113 other consumers the only net is screenshot
  baselines, which the plan itself labels "secondary" and which (per project
  memory) lag platform.

- **"Optional" §6 edits vs the src-wide "no retired name" grep** (flagged by:
  correctness, code-quality, test-coverage) — Phase 2's verification grep
  asserts no retired name survives anywhere under `.../frontend/src`, but
  `extractAcDeclarations.test.ts:35` (`--size-md`) lives under `src/` and is
  labelled an *optional* §6 edit, so "suite green" and "grep-clean" disagree.
  (The two visual-regression spec comments are genuinely optional — they sit
  under `tests/`, outside the grep root.)

- **Numeric-suffix family collision & discoverability** (flagged by: usability,
  standards, architecture) — After the rename three numeric-suffix families
  coexist with different numbering bases: `--size-<px×10>` (120 = 12px),
  `--radius-<px>` (12 = 12px), and the ordinal `--sp-N`. The plan mitigates with
  a disambiguation comment (mirroring ADR-0039), but the collision and the loss
  of intent-bearing names remain a least-surprise and discoverability hazard.

### Tradeoff Analysis

- **Discoverability (semantic names) vs value-recoverability (numeric names)**:
  Usability flags that numeric names answer "what px is this token?" but not the
  more common authoring question "which token do I want?". However, the work
  item *consciously* chose numeric-direct with no alias layer, and two agents
  found the retired semantic names already misdescribe most of their consumers
  (`--size-subtitle` on toast bodies, `--size-eyebrow` on count indicators,
  a `WorkItemCard` comment admitting it picked "the nearest token"). The core
  decision is settled and defensible; the actionable residue is a *mitigation* —
  a short intent→token cheat-sheet in ADR-0043/the comment, or sequencing behind
  the planned DevDesignSystem reference page.

- **Test rigor vs mechanical-rename simplicity**: Test Coverage wants an
  executable value-preservation oracle; the plan deliberately adds no new tests
  ("the existing guardrails are the executable spec"). These reconcile cheaply:
  a one-line invariant decoding each numeric name and asserting it equals its
  declared value makes value-preservation machine-checkable without abandoning
  the "guardrails-as-spec" posture.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Architecture / Documentation / Standards**: ADR-0043 full supersession
  orphans ADR-0036's transitive supersession of ADR-0026
  **Location**: Phase 1 §1 (ADR-0043 Frontmatter/References); Overview ("fully
  supersedes ADR-0036")
  ADR-0036 currently holds `supersedes: ["adr:ADR-0026"]` — the only writable
  channel for the ADR-0036→ADR-0026 typography supersession. Transitioning
  ADR-0036 to `superseded` freezes that edge inside a superseded node; ADR-0043
  does not re-assert it, so a reader traversing from ADR-0026's still-present
  typography rows lands on a superseded ADR with no forward pointer to the live
  governor. ADR-0039 treats exactly this multi-supersession lineage as
  load-bearing prose.

- 🟡 **Architecture**: "0091 amends ADR-0043" collides with the ADR-0031
  immutability lifecycle the plan cites
  **Location**: Phase 1 §1 (Consequences → Neutral); Migration Notes → Downstream
  ADR-0031 permits content edits only on `proposed` ADRs; once ADR-0043 is
  `accepted` it can only be superseded, not amended. Because full supersession
  makes ADR-0043 the sole carrier of the consumption rule, a future 0091
  successor would have to re-state the entire rule just to record a unit
  decision. Either reword "amend" to "supersede with a successor ADR", or narrow
  ADR-0043 so the consumption rule lives in a more stable home.

- 🟡 **Test Coverage**: A wrong-but-valid token swap passes every gate except
  ~17 pinned Playwright selectors
  **Location**: What We're NOT Doing (no per-surface spec); Phase 2 AC4
  A consumer mis-typed to a different *declared* numeric token (e.g. 14px→14.5px)
  passes the `var()`-resolves oracle, parity, the AC5 count ratchet, and AC1/AC2.
  Only `typography-resolved-sizes.spec.ts` (~17 selectors, self-described as not
  covering every site) would catch it; for the other ~113 consumers the sole net
  is "secondary" screenshot baselines.

- 🟡 **Test Coverage**: AC4 "by construction" value-preservation is not
  machine-checked; a duplicated value typo defeats parity
  **Location**: Desired End State ("AC4 … by construction"); Phase 2 AC4
  The parity test only checks `global.css` matches the mirror; the *same*
  transcription error made in both blocks (edited from one mental mapping in
  steps 1 and 3) keeps parity green while the value drifts. A one-line assertion
  decoding each `size-NNN` name and asserting it equals its declared value
  (`size-145` ⇒ `14.5px`) closes this cheaply.

- 🟡 **Usability**: Discoverability regresses — numeric names give no answer to
  "which size do I want?"
  **Location**: What We're NOT Doing ("No semantic alias layer"); Phase 2 §4
  The scheme optimises value-recovery from a name a developer already has, but
  removes the only intent-bearing names with no compensating discoverability
  surface (the DevDesignSystem page is still only a plan). Mitigate with an
  intent→token cheat-sheet, or sequence behind the reference page. (Tradeoff —
  the core no-alias decision is already settled in the work item.)

#### Minor

- 🔵 **Correctness / Code Quality / Test Coverage**: "Optional" §6 edits vs the
  src-wide "no retired name" grep disagree
  **Location**: Phase 2 §6 (Optional consistency edits) vs Phase 2 Automated
  Verification (the `… src` grep)
  `extractAcDeclarations.test.ts:35` is under `src/` and trips the zero-match
  grep, yet is labelled optional — promote it to a required §5 edit (or scope the
  grep to exclude `src/styles/testing/`). The two `tests/`-rooted spec comments
  are genuinely optional.

- 🔵 **Documentation**: Convention comment omits the actual consumption rule
  (unlike the radius sibling it claims to mirror)
  **Location**: Phase 2 §2 (global.css convention comment)
  The proposed comment states only the naming encoding; ADR-0039's radius comment
  states the binding rule inline ("every border-radius … must use a
  `var(--radius-*)`; no literals"). Add a one-line rule clause so the comment is
  self-introducing.

- 🔵 **Documentation**: Migration Notes don't note that 0091 already
  self-documents the stale-name drift
  **Location**: Migration Notes → Downstream
  0091's Technical Notes already defer the refresh of `--size-h3`/`--size-prose`/
  `--size-body` to itself; the plan implies an action 0091 has anticipated.
  State that this PR is not expected to edit 0091.

- 🔵 **Standards**: Verification-command paths diverge from the work item ACs
  (`skills/…/frontend/src/…` vs `src/…`)
  **Location**: Phase 1 & 2 Success Criteria vs work item AC1/AC2
  The plan's repo-root-relative paths are the correct runnable form (mise tasks
  run from repo root); the work item's frontend-relative paths only resolve from
  inside the frontend dir. Note this is an intentional correction.

- 🔵 **Standards**: AC1 grep regex intent is narrower than its prose claim
  **Location**: Phase 2 AC1 automated verification
  `(?![0-9]+\s*:)` only requires the name to *begin* with digits-then-colon, not
  that the whole name is numeric. Correct for this rename set, but the prose
  ("not purely numeric") over-claims. Tighten the prose.

- 🔵 **Standards**: CSS value-column alignment diverges from the existing hand-
  maintained convention
  **Location**: Phase 2 §1 (global.css declarations)
  The Biome CSS formatter is explicitly disabled, so alignment is manual. The
  proposed block aligns at column 22 vs the file's current column 23; pick the
  column intentionally and keep it internally consistent (no formatter will fix
  it).

- 🔵 **Standards**: ADR frontmatter convention is `id:`, not `adr_id`
  **Location**: Phase 1 §1 (ADR-0043 frontmatter)
  On-disk ADRs use `id:`; `create-adr` handles this, but manual verification
  should check against the on-disk `id:`/`supersedes:`/`relates_to:` shape, not
  ADR-0030's prose field names.

- 🔵 **Correctness**: ADR-0043 id is assumed-available but allocated
  interactively
  **Location**: Phase 1 §1 / Phase 2 §2
  The comment, the migration.test.ts repoint, and the Phase 1 grep AC all embed
  the literal "ADR-0043". If `create-adr` allocates a different id, the dangling
  reference is caught by no automated gate. Consume the id `create-adr` actually
  returns, and add a manual check that the embedded id matches the created file.

- 🔵 **Correctness**: The repointed AUTHORITATIVE comment carries surrounding
  load-bearing prose the plan under-surfaces
  **Location**: Phase 1 §2; migration.test.ts:1905-1933
  The block also states "any structural edit to the regexes here requires
  re-deriving the AC2 sweep guidance in the same commit" and references a
  "Phase 8.1" glob. The repoint touches only the ADR id (correct), but confirm
  the AC2 sweep guidance and the FONT_SIZE/SHORTHAND regexes are left unchanged.

- 🔵 **Correctness**: Stated "38 `*.module.css` files" / per-token counts
  diverge from the working tree
  **Location**: Current State Analysis / Phase 2 §4
  The count table reads like a checklist but is stale (0095 shifted counts); only
  the green oracle + the src-wide retired-name grep are authoritative. Label the
  counts non-authoritative orientation only.

- 🔵 **Test Coverage**: AC2 only proves retired names are *absent*, not that the
  required comment content is *present*
  **Location**: Phase 2 AC2; Phase 2 §2
  Nothing executable fails if the comment omits an example or cites the wrong
  ADR. Optionally add a lightweight `global.test.ts` assertion that the size
  comment matches `/px.{0,3}10/` and references ADR-0043.

- 🔵 **Test Coverage**: Phase-1 ADR-0036 grep can pass while leaving the rule's
  authoritative reference stale
  **Location**: Phase 1 §2; Phase 1 Automated Verification
  The grep proves the ADR-0036 *string* is gone, not that the font-size ban is
  tied to ADR-0043. Behaviourally safe (the ban is name-agnostic); optionally
  strengthen to a positive check that ADR-0043 is present in the ban block.

- 🔵 **Architecture**: Supersession *granularity* (full) differs from the
  ADR-0039 model (partial) without being flagged
  **Location**: Overview / Phase 1 vs structural model ADR-0039
  ADR-0039 left ADR-0026 `accepted` (partial supersession); this fully
  supersedes ADR-0036. The divergence is appropriate but should be stated
  explicitly in ADR-0043's scope prose so reviewers don't expect a partial edge.

- 🔵 **Architecture**: The test↔ADR binding remains unchecked prose
  **Location**: Phase 1 §2; migration.test.ts:1905
  The link from the font-size ban to its governing ADR is a comment, so it can
  silently rot on every future supersession. Out of scope to fix, but worth a
  follow-up note (assert the cited ADR is not a `superseded` id).

- 🔵 **Architecture**: px×10 removes the naming friction that discouraged scale
  proliferation
  **Location**: What We're NOT Doing / Phase 1 §1 (Scale-extension policy)
  Under px×10 any 0.5px step is trivially nameable, so the gate against
  proliferation now rests entirely on the prose policy. Make ADR-0043 explicit
  that ease-of-naming ≠ ease-of-admission.

- 🔵 **Documentation**: Escape-valve carry-forward drops the
  EXCEPTIONS-vs-escape-valve rationale
  **Location**: Phase 1 §1 (Escape valve bullet)
  ADR-0036 devotes a whole section to why the two ledgers stay separate (routine
  vs exceptional admissions); AC8 keeps both live. Carry the *distinction*
  forward, not just the array.

- 🔵 **Documentation**: ADR-0043 spec has no Negative consequence for the
  relearning/vocabulary cost
  **Location**: Phase 1 §1 (Consequences)
  Both ADR-0036 and ADR-0039 record the vocabulary-relearning cost of a rename.
  A 19-token t-shirt→numeric switch is precisely this; add a Negative consequence
  with the global.css comment as the mitigation.

- 🔵 **Usability**: Mirror keys drop the `--` prefix — a search blind spot that
  survives the rename
  **Location**: Current State Analysis; Phase 2 §3 (tokens.ts mirror)
  `grep --size-110` still misses `"size-110"`. Document at the point of use
  (ADR-0043 or the tokens.ts block comment) that name sweeps must search both
  forms.

- 🔵 **Usability**: px×10 vs `--sp-N` ordinal collision remains a least-surprise
  hazard
  **Location**: Phase 2 §2 (convention comment); Desired End State
  A comment mitigates but cannot prevent a developer assuming the suffixes are
  commensurable. Consider a side-by-side example ("12px is `--size-120`,
  `--radius-12`, and `--sp-3`").

#### Suggestions

- 🔵 **Usability**: Have ADR-0043 record that the retired semantic names already
  misdescribed most consumers (cite 2-3, mirroring ADR-0039's `--radius-block`
  analysis) — reframes the change as removing a *false* signal.
- 🔵 **Correctness**: Mandate delimiter-anchored replacement as the *only* path
  (the order-dependent shorter-first path is the only one with a failure mode).
- 🔵 **Code Quality**: Add one worked find/replace example for a collision pair
  (`var(--size-3xs)`→`var(--size-100)` and `var(--size-3xs-lg)`→`var(--size-105)`)
  so the anchoring strategy is copy-pasteable, not just described.
- 🔵 **Code Quality**: Drop the "excluding `--size-foo`" clause from the src-wide
  grep — `foo` is not in the alternation, so the exclusion is a no-op (keep the
  standalone "leave :2027 untouched" instruction in §5).
- 🔵 **Usability**: Make the comment's half-step example call out the two-digit
  small end explicitly ("`--size-95` is 9.5px, NOT 95px") to preempt the
  riskiest misread.

### Strengths

- ✅ px-value preservation is verifiable line-by-line: every new token declares
  the exact px its old name carried, all 19 values are distinct, and the
  encoding is literal px×10 — confirmed against `global.css:173-191` and
  `tokens.ts:183-201`.
- ✅ The `var()`-resolves-to-declared oracle (`migration.test.ts:1649-1688`) is a
  genuine completeness gate for consumers — it derives the declared set from
  `Object.keys(TYPOGRAPHY_TOKENS)`, and the order-agnostic parity test
  independently catches declaration/mirror drift.
- ✅ The AC5 ratchet is correctly identified as name-agnostic (counts the
  `var(--` prefix), so a pure rename provably preserves the count with no floor
  bump.
- ✅ The substring-collision hazard (`--size-3xs` ⊂ `--size-3xs-lg`) is correctly
  diagnosed, and the delimiter-anchoring mitigation is genuinely order-
  independent.
- ✅ The hardcoded test-string sweep is accurate — agents verified the exact line
  references in `migration.test.ts`, `Chip.test.tsx`, and
  `FrontmatterTable.test.tsx`, and confirmed ADR-0036 appears exactly once
  (line 1905).
- ✅ Intentional fixtures (`--size-foo` at :2027; NEGATIVE_SHORTHAND at
  2037-2038) are correctly identified as leave-untouched.
- ✅ The px×10 scheme is consistent with the sibling px-encoded `--radius-N`
  family already in active use, and the plan mandates the `--sp-N` disambiguation
  comment from ADR-0039 — strong point-of-use onboarding.
- ✅ Dropping the semantic alias layer is the architecturally cleaner choice (a
  flat dependency graph, no second drift surface, a single-source oracle).
- ✅ ADR-0043 is the correct next sequential id and the supersession is correctly
  modelled on ADR-0031's permitted `accepted → superseded` transition via
  `create-adr --supersedes`, refusing to hand-edit ADR-0036's body.
- ✅ ADR-0043's planned section list matches the ADR-0030 template ordering, and
  the value-mutation policy (change a value by adding a step + re-pointing) is
  correctly carried from the ADR-0039 model.
- ✅ The "no dynamic/computed `--size-*` names" assumption was verified (no
  production runtime concatenation), validating the mechanical-find-replace
  approach.

### Recommended Changes

1. **Fix the ADR supersession lineage in ADR-0043** (addresses: "ADR-0043 full
   supersession orphans ADR-0036's transitive supersession of ADR-0026";
   "supersession granularity differs from ADR-0039"). In ADR-0043's
   Scope-of-supersession prose: (a) state that ADR-0043 assumes governance of the
   ADR-0026 typography clauses ADR-0036 previously superseded (so the chain
   ADR-0026 → ADR-0036 → ADR-0043 is discoverable from the live end), and
   (b) note this is a *full* replacement, unlike ADR-0039's partial supersession
   of ADR-0026.

2. **Reconcile the "0091 amends ADR-0043" path with immutability** (addresses:
   "'0091 amends ADR-0043' collides with ADR-0031"). Reword the Neutral
   consequence and Migration Notes so "amend" reads "supersede with a successor
   ADR" — or narrow ADR-0043 so a future unit decision supersedes a smaller
   surface than the whole consumption rule.

3. **Add a one-line px×10 value-preservation assertion** (addresses: "AC4 by
   construction is not machine-checked"; "wrong-but-valid token swap"). Add a
   vitest invariant that decodes each `size-NNN` mirror key and asserts it equals
   its declared value (`size-145` ⇒ `14.5px`). Additionally pin the specific
   target token in the red-first hardcoded assertions for the heaviest files
   (Sidebar, lifecycle, FilterPill, MarkdownRenderer) so a wrong-target swap
   there turns the suite red rather than relying on screenshot diffs.

4. **Resolve the optional-vs-required edit contradiction** (addresses:
   "'Optional' §6 edits vs the src-wide grep"). Promote
   `extractAcDeclarations.test.ts:35` to a required §5 edit (it is under `src/`
   and trips the zero-match grep), and keep the two `tests/`-rooted spec comments
   as genuinely optional. Drop the no-op "excluding `--size-foo`" clause.

5. **Strengthen the global.css convention comment** (addresses: "comment omits
   the consumption rule"; "px×10 vs --sp-N collision"; "two-digit misread").
   Add a one-line statement of the binding rule (every font-size resolves to a
   `var(--size-*)`; no px/rem/em literals), and make the half-step example call
   out `--size-95` = 9.5px (not 95px).

6. **Complete the ADR-0043 carry-forward** (addresses: "escape-valve rationale
   dropped"; "no Negative consequence for relearning cost"; "scale proliferation
   friction"). Carry forward the EXCEPTIONS-vs-escape-valve *distinction*, add a
   Negative consequence for the one-time vocabulary relearning cost, and state
   that ease-of-naming ≠ ease-of-admission. Optionally cite 2-3 consumers the old
   semantic names already misdescribed.

7. **Tighten verification commands and ADR-id handling** (addresses: "ADR-0043 id
   assumed-available"; "verification paths diverge"; "AC1 regex prose
   over-claims"; "frontmatter id: vs adr_id"). Consume the id `create-adr`
   returns rather than the literal "ADR-0043"; note the repo-root-relative paths
   are an intentional correction of the work item; tighten the AC1 prose; verify
   ADR frontmatter against the on-disk `id:` convention.

8. **Add a discoverability mitigation** (addresses: "discoverability regresses";
   "mirror keys drop the -- prefix"). Add a short intent→token cheat-sheet to
   ADR-0043/the comment (or sequence behind the DevDesignSystem page), and
   document that name sweeps must search both `--size-N` and `"size-N"`.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Correctness

**Summary**: This is an unusually rigorous, value-preserving rename plan whose
correctness rests on three verifiable invariants — exact px preservation, the
`var()`-resolves completeness oracle, and substring-collision-safe replacement —
all of which the agent confirmed hold against the actual source. The mapping
table is internally consistent (all 19 px values distinct, encoding is literal
px×10), the cited guardrail line numbers and the ADR-0036 single-reference claim
are accurate, and the AC1/AC2 regexes behave correctly against both old and new
names with no false positives or false negatives. The principal correctness
risks are not in the mechanics but in completeness-scope edges: the standing
oracle does not scan files outside `src/`, the whole-tree verification grep is
scoped to `src/` only, and the atomicity invariant depends on a manual
replacement discipline that has one ordering-sensitive trap.

**Strengths**:
- px-value preservation verifiable line-by-line; all 19 values distinct so px×10
  produces no collisions — AC4 holds by construction.
- The `var()`-resolves oracle (`migration.test.ts:1649-1688`) is a complete
  completeness gate for consumers; the parity test independently catches
  global.css↔mirror drift.
- The AC5 ratchet is name-agnostic (`/var\(\s*--/g`), so a pure rename preserves
  the count (currently `AC5_FLOOR=989`).
- The substring-collision hazard is correctly diagnosed; delimiter-anchoring is
  order-independent.
- The hardcoded test-string sweep is accurate (verified exact line references);
  ADR-0036 appears exactly once (line 1905).
- `:2027 "--size-foo: 11px;"` correctly identified as a NEGATIVE_LITERAL
  placeholder to leave untouched.

**Findings**:
- 🔵 minor / high — *The completeness verification grep is scoped to `src/` and
  misses the `tests/visual-regression` specs the plan itself edits in §6*
  (Phase 2 Success Criteria). The `tests/`-rooted spec edits in §6 are outside
  the `src`-scoped grep and the oracle, so skipping them leaves stale retired
  names while every automated gate passes green. Extend the grep root to the
  frontend root, or reclassify the §6 comment edits as required.
- 🔵 minor / medium — *ADR-0043 id is assumed-available but allocated
  interactively; a collision invalidates both the Phase 1 grep AC and the Phase 2
  comment text* (Phase 1 §1 / Phase 2 §2). Every embedded "ADR-0043" string would
  point at a nonexistent ADR, caught by no automated gate. Consume the returned
  id; add a manual check.
- 🔵 minor / medium — *The AUTHORITATIVE comment block carries a stale "Phase 8.1"
  glob reference and a derive-in-same-commit obligation the plan does not
  surface* (Phase 1 §2; migration.test.ts:1922-1933/:1915). No regex is being
  edited, so the obligation is not triggered, but confirm the AC2 sweep guidance
  and the FONT_SIZE/SHORTHAND regexes are left unchanged.
- 🔵 minor / high — *The stated "38 `*.module.css` files" and per-token counts
  diverge from the working tree; only the oracle is authoritative* (Current State
  Analysis / Phase 2 §4). The count table reads like a checklist but is stale;
  label it non-authoritative orientation only.
- 🔵 suggestion / high — *The atomicity invariant is correctly stated but relies
  on a single manual find-replace; the ordering trap has no automated guard
  mid-edit* (Implementation Approach / Migration Notes). A naive shorter-first
  replace would produce undeclared keys (loud failure, not silent). State that
  delimiter-anchored replacement is mandatory, not an equal alternative.

### Code Quality

**Summary**: A well-researched, value-preserving mechanical rename with
unusually precise instructions: exact line anchors, a complete mapping table, an
explicit substring-collision strategy, and a documented set of find-replace
traps. The pure-numeric px×10 scheme is a clear maintainability improvement over
the three interleaved naming families it replaces, and it is structurally
consistent with the sibling ADR-0039 radius ladder. The main code-quality
concern is one internal inconsistency: the plan labels the
`extractAcDeclarations.test.ts` edit "optional" yet its own zero-match
verification grep over `src` makes that edit mandatory.

**Strengths**:
- px×10 makes design intent fully recoverable from the name and sorts
  numerically, eliminating the t-shirt/semantic/tween ambiguity.
- The convention comment is explicitly modelled on the sibling ADR-0039 radius
  comment, including the `--sp-N` disambiguation note.
- The two find-replace traps and the substring-collision pairs are identified up
  front with an order-safe delimiter-anchoring strategy.
- The executable completeness oracle and parity gate are correctly identified;
  the counts are explicitly marked re-count-at-implementation-time.
- Phase 2's atomicity rationale is sound and prevents a broken interim state.

**Findings**:
- 🔵 minor / high — *`extractAcDeclarations.test.ts` edit is labelled optional but
  is mandatory for the src-wide zero-match gate* (Phase 2 §6 vs Phase 2 Automated
  Verification). The file lives under `src/styles/testing/`, so its `--size-md`
  literal is in the grep's scope and will trip the gate. Promote it to the
  mandatory §5 edits, or scope the grep to exclude `src/styles/testing/`.
- 🔵 suggestion / medium — *The `--size-foo` exclusion caveat is unnecessary given
  the anchored retired-name alternation* (Phase 2 Automated Verification / Key
  Discoveries). `foo` is not in the alternation, so the exclusion is a no-op.
  Drop the clause; keep the standalone "leave :2027 untouched" instruction.
- 🔵 suggestion / medium — *Delimiter-anchored find-replace is described but not
  given a concrete reproducible recipe* (Phase 2 §4 / Migration approach).
  Include one worked find/replace pair for a collision pair so the strategy is
  copy-pasteable and verifiable.

### Architecture

**Summary**: Structurally sound as a value-preserving rename: it correctly
identifies the auto-tracking `var()`-resolves oracle and global.css↔tokens.ts
parity test as the executable completeness gates, leans on them rather than
inventing new structure, and the px×10 scheme is consistent with the sibling
px-encoded radius family while explicitly disambiguating from the ordinal
`--sp-N` family. The main architectural risks are in the ADR supersession
modelling: a "full supersession" of ADR-0036 collides with the immutability
lifecycle the plan cites (0091 cannot "amend" an accepted ADR-0043) and freezes/
orphans ADR-0036's own canonical `supersedes` edge to ADR-0026. The two-phase
same-PR coupling and atomic Phase 2 are correctly justified.

**Strengths**:
- Leans on existing auto-tracking guardrails as the completeness contract; the
  oracle self-adjusts to the mirror keys (no edit required, no new SPOF).
- Atomicity is derived from a real invariant, not convenience; the substring trap
  is handled by order-independent anchoring.
- px×10 is consistent with the sibling `--radius-<px>` family and mandates the
  same `--sp-N` disambiguation comment.
- The value-mutation policy is correctly carried from the ADR-0039 model.
- Dropping the semantic alias layer keeps the dependency graph flat and
  single-source.

**Findings**:
- 🔴 major / high — *"0091 amends ADR-0043" collides with the ADR-0031
  immutability lifecycle* (Phase 1 §1 Consequences→Neutral; Migration Notes→
  Downstream). ADR-0031 permits content edits only on `proposed` ADRs; an
  accepted ADR-0043 can only be superseded. Full supersession makes ADR-0043 the
  sole carrier of the consumption rule, so 0091 would have to re-state the whole
  rule. Reword "amend" to "supersede with a successor ADR", or narrow ADR-0043's
  scope.
- 🔴 major / high — *ADR-0043 full supersession orphans ADR-0036's `supersedes`
  edge to ADR-0026* (Phase 1 §1 Frontmatter/References; Overview). ADR-0036's
  edge to ADR-0026 freezes inside a superseded node; ADR-0043 does not re-assert
  it, so the live governance of ADR-0026's typography rows becomes undiscoverable.
  Have ADR-0043 inherit/restate the ADR-0026 typography supersession.
- 🔵 minor / high — *Supersession granularity (full) differs from the ADR-0039
  model (partial) without being flagged* (Overview / Phase 1 vs ADR-0039). The
  divergence is appropriate but should be stated explicitly in ADR-0043's scope
  prose.
- 🔵 minor / medium — *The sole linkage from the font-size ban gate to its
  governing ADR is unchecked prose* (Phase 1 §2; migration.test.ts:1905). It can
  silently rot on every future supersession. Out of scope; worth a follow-up
  (assert the cited ADR is not a `superseded` id).
- 🔵 minor / medium — *px×10 removes the naming friction that discouraged scale
  proliferation* (What We're NOT Doing / Phase 1 §1). The gate now rests entirely
  on the prose policy. Make ADR-0043 explicit that ease-of-naming ≠
  ease-of-admission.

### Test Coverage

**Summary**: The plan rests almost entirely on three existing auto-tracking
guardrails (the `var()`-resolves oracle, the global.css↔tokens.ts parity loop,
and the AC5 var-count ratchet) plus a hand-edited set of name-pinned assertions,
and for the failure modes those tests cover the strategy is sound. However, the
central risk of a value-preserving rename — a find-replace that swaps a consumer
onto the *wrong but still-declared* numeric token — passes every vitest gate,
every AC1/AC2 grep, and the parity/count ratchets silently; it is caught only if
that surface is one of ~17 selectors pinned in the Playwright resolved-size spec.
The "no per-surface regression spec" decision means the executable safety net for
value preservation across the other ~113 consumers is the screenshot baselines
alone, which lag platform and are explicitly "secondary".

**Strengths**:
- Correctly identifies the `var()`-resolves oracle as a genuine completeness
  oracle for the surviving-old-consumer failure (verified against
  `migration.test.ts:1649-1688`).
- The parity loop (`global.test.ts:104-119`) catches a mirror-key renamed without
  its declaration (or vice versa).
- Accurately enumerates the hardcoded name-pinned strings a CSS sweep misses and
  prescribes the red-first posture.
- Verified the "no dynamic/computed `--size-*` names" assumption.
- Correctly flags the intentional negative fixtures to leave untouched.
- Correctly notes the font-size ban is name-agnostic and its ADR-0036 reference
  is prose, handled by an explicit Phase 1 repoint.

**Findings**:
- 🔴 major / high — *Wrong-but-valid token swap passes every gate except ~17
  pinned Playwright selectors* (What We're NOT Doing; Phase 2 AC4; Testing
  Strategy). A mis-typed-but-declared token (14px→14.5px) passes the oracle,
  parity, ratchet, and AC greps; only ~17 pinned selectors would catch it. Add a
  per-consumer resolved-px snapshot assertion, or pin target tokens on the
  highest-count files.
- 🟡 major / high — *AC4 "by construction" depends on a manual line-by-line
  mapping check with no executable enforcement* (Desired End State; Phase 2 AC4).
  The same value typo duplicated across global.css and the mirror defeats parity.
  Add a one-line assertion decoding each numeric name and asserting it equals its
  declared value.
- 🔵 minor / high — *AC2 only proves retired names are absent, not that the
  required comment content is present* (Phase 2 AC2; Phase 2 §2). Optionally add a
  `global.test.ts` assertion that the comment matches `/px.{0,3}10/` and
  references ADR-0043.
- 🔵 minor / medium — *Stale old-name references survive green in non-asserting
  fixture/comment strings* (Key Discoveries; Phase 2 §5/§6). The broad-grep AC and
  the test suite disagree about completeness. Promote the §6 edits to required, or
  carve the known non-asserting occurrences into the grep exclusion list.
- 🔵 minor / medium — *Phase-1 ADR-0036 grep gate can pass while leaving the
  rule's authoritative reference stale* (Phase 1 §2; Phase 1 Automated
  Verification). Behaviourally safe; optionally strengthen to a positive check
  that ADR-0043 is present in the ban block.

### Documentation

**Summary**: A documentation-heavy plan that is unusually strong on that front:
the ADR-0043 content spec enumerates each ADR-0036 policy to carry forward, the
convention-comment rewrite is fully specified with both examples and the `--sp-N`
disambiguation, and the single stale ADR-0036 test reference is correctly
identified and repointed. Two carry-forward gaps are worth tightening: the
transitive ADR-0026 supersession lineage (which ADR-0039 treats as load-bearing)
is not addressed, and ADR-0036's explicit EXCEPTIONS-vs-escape-valve rationale is
not flagged for preservation. The downstream work-item:0091 staleness is
correctly noted and 0091 already self-documents it.

**Strengths**:
- The ADR-0043 content spec explicitly enumerates every ADR-0036 policy to carry
  forward, each tied to an AC.
- The convention-comment rewrite spec is complete and accurate (whole-step +
  half-step examples, ADR-0043 reference, `--sp-N` disambiguation).
- Stale-reference handling is precise (migration.test.ts:1905 is the only
  ADR-0036 reference and is prose, repointed by hand with a grep success
  criterion).
- The plan distinguishes load-bearing token edits from cosmetic prose edits.
- It reconciles its own count divergence from the work item (0095 landed in
  between).

**Findings**:
- 🔴 major / high — *ADR-0043 spec is silent on ADR-0036's transitive supersession
  of ADR-0026's typography clauses* (Phase 1 §1). A reader traversing from
  ADR-0026's typography rows could land on superseded ADR-0036 with no forward
  pointer. Add a Scope-of-supersession clause mirroring ADR-0039.
- 🔵 minor / medium — *Escape-valve carry-forward drops the EXCEPTIONS-vs-escape-
  valve rationale* (Phase 1 §1 Escape valve bullet). Carry the distinction
  forward, not just the array.
- 🔵 minor / medium — *No Negative consequence for the relearning/vocabulary cost*
  (Phase 1 §1 Consequences). Both ADR-0036 and ADR-0039 record this; add one with
  the global.css comment as the mitigation.
- 🔵 minor / high — *The convention comment omits the actual consumption-rule
  statement, unlike the radius sibling it claims to mirror* (Phase 2 §2). Add a
  short rule clause so the comment is self-introducing.
- 🔵 minor / high — *Migration Notes don't note that 0091 already self-documents
  the stale-name drift* (Migration Notes / Phase 1 Success Criteria). State that
  this PR is not expected to edit 0091.

### Standards

**Summary**: Strong conformance to project conventions: the px×10 scheme is
consistent with the sibling px-encoded `--radius-N` family, it correctly mirrors
ADR-0039's structure and `--sp-N` disambiguation comment, allocates the correct
next ADR id (ADR-0043, following ADR-0042), and respects the ADR-0031
supersession-only edit rule. The main standards concerns are a verification-path
inconsistency between the plan and the work item ACs, an AC1 grep regex whose
prose over-claims, and a CSS value-alignment column that diverges from the
existing hand-maintained convention (relevant because the Biome CSS formatter is
explicitly disabled).

**Strengths**:
- px×10 continues the established px-encoded measurement-token convention
  (`--radius-N`).
- The convention comment disambiguates the px-derived suffix from the ordinal
  `--sp-N` suffix, mirroring the radius comment and ADR-0039.
- ADR-0043 is the correct next id; supersession is modelled on ADR-0031's
  permitted transition via `create-adr --supersedes`, without hand-editing
  ADR-0036's body.
- The `superseded_by: "adr:ADR-0043"` quoted form matches the established
  frontmatter format.
- ADR-0043's section list matches the ADR-0030 template ordering.

**Findings**:
- 🔵 minor / high — *AC1 grep regex intent is more narrowly expressed than the
  prose claims* (Phase 2 AC1). `(?![0-9]+\s*:)` only requires the name to begin
  with digits-then-colon. Correct for this set; tighten the prose.
- 🔵 minor / high — *Verification greps use `skills/…/frontend/src/…` while the
  work item ACs use `src/…`* (Phase 1 & 2 Success Criteria vs work item AC1/AC2).
  The plan's repo-root-relative paths are the correct runnable form; note this is
  an intentional correction.
- 🔵 minor / medium — *global.css value-column alignment diverges from the existing
  convention* (Phase 2 §1). The Biome CSS formatter is disabled, so alignment is
  manual; pick the column intentionally and keep it consistent.
- 🔵 minor / medium — *ADR frontmatter convention is `id:`, not `adr_id`* (Phase 1
  §1). `create-adr` handles this; manual verification should check against the
  on-disk `id:` convention.
- 🔵 suggestion / medium — *ADR-0043 supersedes-edge vs the ADR-0026
  multi-supersession precedent* (Phase 1 §1). Consider noting in ADR-0043's
  References that it carries forward the typography rule ADR-0036 took from
  ADR-0026, so the chain is discoverable from the live end.

### Usability

**Summary**: From a developer-experience standpoint the plan trades semantic
token names for a pure-numeric px×10 scheme, and on the whole the trade-off is
sound: the codebase already proves the supposed "semantic intent" is largely
illusory (`--size-subtitle`, `--size-eyebrow`, `--size-row` already misdescribe
most of their consumers), and the sibling `--radius-N` px-encoded ladder is
already in daily use, so the new scheme improves consistency rather than
degrading it. The standout DX weakness is discoverability: a developer who needs
"a slightly-bigger-than-body label" can no longer grep or guess a meaningful
name, and the only onboarding surfaces are a global.css comment plus an ADR. The
remaining concern is the plan-acknowledged risk that `--size-<px×10>` and
`--sp-N` (ordinal) share a numeric-suffix shape but not a numbering basis.

**Strengths**:
- Follows an established in-repo convention (`--radius-N` px-encoded), so the
  mental model transfers directly.
- Carries forward ADR-0039's `--sp-N` disambiguation and proposes both a
  whole-step and a half-step worked example.
- Numeric names make value and ordering fully recoverable and let any 0.5px step
  be named without a fourth ad-hoc family.
- Value preservation is total, so there is zero behavioural surprise while
  relearning.
- The value-mutation policy keeps the name↔value contract truthful.

**Findings**:
- 🟡 major / high — *Discoverability regresses: numeric names give no answer to
  "which size do I want?"* (What We're NOT Doing; Phase 2 §4). The intent-bearing
  names are removed with no compensating discoverability surface. Add an
  intent→token cheat-sheet, or sequence behind the DevDesignSystem page.
  (Tradeoff — the core no-alias decision is settled in the work item.)
- 🔵 minor / high — *Mirror keys drop the `--` prefix — an inconsistency that
  survives the rename and trips name-based search* (Current State Analysis; Phase
  2 §3). Document at the point of use that sweeps must search both `--size-N` and
  `"size-N"`.
- 🔵 minor / medium — *px×10 vs `--sp-N` ordinal collision is mitigated by a
  comment but remains a least-surprise hazard* (Phase 2 §2; Desired End State).
  Consider a side-by-side example ("12px is `--size-120`, `--radius-12`, and
  `--sp-3`").
- 🔵 suggestion / medium — *Variable-width numeric names hurt scannability; the
  comment should preempt the "95 vs 950" misread* (Phase 2 §2). Make the
  half-step example call out `--size-95` = 9.5px, NOT 95px.
- 🔵 suggestion / high — *ADR should record that the retired semantic names
  already misdescribed most consumers* (Phase 1 §1 Context; What We're NOT Doing).
  Cite 2-3 mismatched consumers (mirroring ADR-0039's `--radius-block` analysis)
  to reframe the change as removing a false signal.

## Re-Review (Pass 2) — 2026-06-13

**Verdict:** APPROVE

The pass-1 edits resolved all 5 majors and the full minor/suggestion set. The
re-review then caught one real defect *introduced* by those edits — the new
Phase 2 §7 value-preservation assertion iterated over all `TYPOGRAPHY_TOKENS`
(which contains non-size `ac-font-*`/`lh-*`/`tracking-caps` keys), so the
`size-` decode produced `NaN` and the test would fail on a *correct* rename
(flagged by correctness, code-quality, test-coverage, and standards;
test-coverage rated it critical). That defect, plus a new architecture minor
(ADR-0043 lacked a live typed edge to ADR-0026), were both **fixed within this
pass**. Only two low-priority suggestions remain open, both acceptable as-is.

### Previously Identified Issues

**Architecture** (2 major, 3 minor → all resolved)
- 🟡 "0091 amends ADR-0043" vs ADR-0031 immutability — **Resolved** (reworded to "supersede with a successor ADR", citing ADR-0031).
- 🟡 Full supersession orphans ADR-0036's edge to ADR-0026 — **Resolved** (Scope-of-supersession bullet makes the ADR-0026 → ADR-0036 → ADR-0043 chain discoverable; further hardened in pass 2 with a live `relates_to` edge).
- 🔵 Supersession granularity (full vs partial) not flagged — **Resolved**.
- 🔵 Test↔ADR binding unchecked prose — **Resolved** (acknowledged as out-of-scope follow-up).
- 🔵 px×10 removes scale-proliferation friction — **Resolved** ("ease-of-naming ≠ ease-of-admission" clause added).

**Test Coverage** (2 major, 3 minor)
- 🟡 Wrong-but-valid token swap ships green — **Accepted tradeoff** (per the "decode only" decision; documented, Playwright specs are the secondary net).
- 🟡 AC4 "by construction" not machine-checked — **Resolved** (the §7 decode assertion closes the duplicated-typo gap; its iteration bug was fixed this pass).
- 🔵 AC2 proves absence not presence of comment content — **Partially resolved** (presence remains a manual-verification check, by design).
- 🔵 Stale names survive green in non-asserting strings — **Resolved** (`extractAcDeclarations` promoted to required; §6 scope clarified).
- 🔵 Phase-1 ADR-0036 grep can pass while reference stale — **Resolved** (positive `grep ADR-0043` AC added).

**Documentation** (1 major, 4 minor → all resolved)
- 🟡 ADR-0043 silent on transitive ADR-0026 supersession — **Resolved** (clause citation verified accurate against ADR-0036/0026).
- 🔵 Escape-valve rationale dropped — **Resolved**.
- 🔵 No Negative consequence for relearning cost — **Resolved** (misdescribing-consumer citations verified accurate).
- 🔵 Convention comment omitted the consumption rule — **Resolved**.
- 🔵 Migration Notes didn't note 0091 self-documents drift — **Resolved**.

**Usability** (1 major, 2 minor, 2 suggestion)
- 🟡 Discoverability regression — **Accepted tradeoff** (honest Negative-consequence record added, no cheat-sheet, per decision).
- 🔵 Mirror keys drop `--` prefix (search blind spot) — **Deferred** (conscious, per the "minimal" steer).
- 🔵 px×10 vs `--sp-N` collision hazard — **Resolved** (three-family 12px side-by-side added to the comment).
- 🔵 Variable-width "95 vs 950" misread — **Resolved** ("`--size-95` is 9.5px, NOT 95px" callout).
- 🔵 ADR should cite already-misdescribing consumers — **Resolved**.

**Correctness** (4 minor, 1 suggestion → all resolved)
- 🔵 Verification grep scope vs §6 specs — **Resolved**.
- 🔵 ADR-0043 id assumed-available — **Resolved** (allocation note added).
- 🔵 AUTHORITATIVE comment surrounding prose — **Resolved** ("touch only the id" note).
- 🔵 Per-token counts diverge / read as checklist — **Resolved** (labelled non-authoritative).
- 🔵 Atomicity ordering trap — **Resolved** (anchored replacement made mandatory + worked example).

**Code Quality** (1 minor, 2 suggestion → all resolved)
- 🔵 `extractAcDeclarations` mislabelled optional — **Resolved**.
- 🔵 No-op `--size-foo` exclusion clause — **Resolved** (clause removed).
- 🔵 Anchored find-replace lacked a recipe — **Resolved** (worked example added).

**Standards** (4 minor, 1 suggestion → all resolved)
- 🔵 AC1 regex prose over-claim — **Resolved**.
- 🔵 Verification-path inconsistency — **Resolved** (repo-root-relative path note).
- 🔵 CSS alignment vs hand-maintained convention — **Resolved** (Biome-CSS-disabled note).
- 🔵 `id:` vs `adr_id` frontmatter — **Resolved**.
- 🔵 ADR-0026 multi-supersession precedent — **Resolved**.

### New Issues Introduced

- 🔴 → ✅ **Correctness / Code Quality / Test Coverage / Standards**: the new §7
  decode assertion iterated all `TYPOGRAPHY_TOKENS` keys, so `Number(key.slice(5))`
  was `NaN` on the non-size `ac-font-*`/`lh-*`/`tracking-caps` keys and the test
  failed on a correct rename — **Fixed this pass**: loop now filters to
  `/^size-\d+$/`, is wrapped in an idiomatic `it.each` block, and uses a
  `parseFloat` comparison (also addressing the bare-loop, placement, and
  float-formatting sub-points).
- 🔵 → ✅ **Architecture**: ADR-0043 claimed governance of ADR-0026's typography
  clauses but carried no live typed edge to ADR-0026 (silent divergence from the
  ADR-0039 model) — **Fixed this pass**: `adr:ADR-0026` added to `relates_to` as
  a first-class live-node edge, with the deliberate divergence called out
  explicitly.
- 🔵 **Documentation** (suggestion, open/accepted): the consumption rule is now
  stated in three places with slightly varying phrasing — accepted as intentional
  self-introduction at the declaration site (consistent with the radius sibling).
- 🔵 **Usability** (suggestion, open/accepted): the §2 convention comment has
  grown to ~16 body lines (vs the 6-line `--radius` sibling) — content is correct
  and complete; optional tightening for scannability deferred.

### Assessment

The plan is sound and ready for implementation. Every major from pass 1 is
resolved or accepted by explicit decision; the one real defect the edits
introduced (the §7 test iteration bug) and the one new architecture minor were
both fixed in this pass. The two remaining items are low-priority suggestions
that are acceptable as-is. No further review pass is required.
