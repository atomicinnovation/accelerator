---
date: "2026-05-07T01:00:00+00:00"
type: work-item-review
skill: review-work-item
target: "meta/work/0034-theme-and-font-mode-toggles.md"
work_item_id: "0034"
review_number: 1
verdict: REVISE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 2
status: complete
---

## Work Item Review: Theme and Font-Mode Toggles

**Verdict:** REVISE

The work item is well-authored overall — the summary is precise, the context is substantive, and the Technical Notes section is unusually thorough with concrete implementation guidance including localStorage key names, the boot-script placement requirement, and a reference pattern to follow for React context. However, three major findings prevent approval: the `[data-font="mono"]` CSS block is claimed as delivered by 0033 in the Context section but explicitly stated as missing in Technical Notes, creating a contradiction that misrepresents the upstream dependency's completion status; two of the most technically complex acceptance criteria (token resolution coverage and boot-script placement) lack the specificity or defined verification procedure needed to be testable. Two further cross-cutting themes — each flagged by multiple lenses — reinforce that the "every component" token sweep language needs tightening throughout.

### Cross-Cutting Themes

- **`[data-font="mono"]` delivery gap** (flagged by: clarity, dependency, completeness) — The Context section says 0033 ships the `[data-font="mono"]` CSS block; Technical Notes say it does not exist. This inconsistency means the upstream blocker is only partially satisfied, and the missing block's authorship is only discoverable deep in Technical Notes rather than in Requirements or Dependencies.
- **"Every component consumes `--ac-*` tokens" is unbounded** (flagged by: clarity, scope, testability) — This requirement appears in Summary, Requirements, and implicitly in AC1, but is never bounded to a scope, anchored to a verification method, or matched to a dedicated acceptance criterion. Three lenses independently identified it as problematic for different reasons (ambiguous referent, unbounded scope, untestable claim).

---

### Findings

#### Major

- 🟡 **Clarity + Dependency**: `[data-font="mono"]` CSS block — Context contradicts Technical Notes; 0033 is an incomplete blocker
  **Location**: Context; Dependencies; Technical Notes
  The Context section states "0033 ships a `[data-font="mono"]` font-mode swap that repoints `--ac-font-display` and `--ac-font-body` to Fira Code, awaiting the same wiring." The Technical Notes directly contradict this: "`[data-font="mono"]` CSS block is missing: `global.css` has no `[data-font="mono"]` selector. Despite the work item stating 0033 ships this block, it was not delivered. The 0034 implementer must write it." This means (a) the two sections contradict each other on pre-existing versus in-scope work, creating a trap for anyone who reads Context but not Technical Notes; and (b) the Dependencies section lists 0033 as a satisfied blocker without acknowledging that its font-mode deliverable is incomplete, making the dependency record misleading. An implementer trusting the Context will scope their work incorrectly and may omit writing the CSS block entirely.

- 🟡 **Testability**: Token resolution AC is unbounded and unverifiable as stated
  **Location**: Acceptance Criteria (criterion 1)
  The first AC states "every `--ac-*` token resolves to its theme-appropriate value." The word "every" introduces an unbounded enumeration — there is no defined set of tokens, no reference to the token block that defines completeness, and no specified verification method. A tester cannot write a complete pass/fail check without independently discovering the full token catalogue. The Technical Notes already reference a parity-test pattern in `global.test.ts:167` and name the specific dark-token block at `global.css:138–160` — the criterion should reference these rather than leaving the scope open-ended.

- 🟡 **Testability**: Boot-script placement criterion lacks a defined verification procedure
  **Location**: Acceptance Criteria (criterion 6)
  The sixth AC states the inline boot script must be the first element in `<head>`, before any `<link rel="stylesheet">` tags, and `suppressHydrationWarning` must be set on `<html>`. While structurally clear, the criterion provides no verification procedure — there is no mention of a DOM inspection test, build-artefact check, or browser devtools step. As written, it functions as an implementation instruction rather than a verifiable outcome, and two implementers could disagree on whether it passes.

#### Minor

- 🔵 **Clarity**: "When the toggle fires" is a redundant, undefined trigger
  **Location**: Acceptance Criteria (criteria 1 and 2)
  Both the theme and font-mode ACs use "when the toggle fires" as the When clause, which simply restates the Given clause ("the user clicks the topbar toggle"). This makes the Given and When indistinguishable, weakening the BDD structure and making the criterion slightly harder to verify mechanically.

- 🔵 **Dependency**: CSP coupling named in Assumptions but absent from Dependencies
  **Location**: Assumptions; Dependencies
  The Assumptions section identifies a real prerequisite: the deployment's Content Security Policy must permit the inline boot script. Without this, the "before any paint" flash-prevention guarantee (and AC6) cannot be met. This is a cross-team or infrastructure-level action that belongs in Dependencies alongside 0033 and 0035 so it is visible during sprint planning.

- 🔵 **Dependency**: Token-migration prerequisite not reflected in Dependencies
  **Location**: Dependencies; Context
  The referenced design gap document's suggested sequencing implies a per-component token sweep must precede the theme swap. If that sweep is not yet complete, the theme toggle will silently fail to swap colours in components still using hard-coded hex values. It is unclear whether this sweep is considered complete as of 0033 or remains an untracked prerequisite.

- 🔵 **Scope**: Token-compliance sweep requirement is unbounded relative to story scope
  **Location**: Requirements (last bullet)
  "Ensure every component consumes `--ac-*` tokens…" could span the entire component library. If a full sweep is already delivered by prior work, the requirement should be reframed as a verification gate; if it is genuinely new work, it needs explicit scoping.

- 🔵 **Testability**: Font-mode AC does not specify the expected CSS property value
  **Location**: Acceptance Criteria (criterion 2)
  The criterion states `--ac-font-display` / `--ac-font-body` "repoint to Fira Code" but does not give the expected computed value. A verifier must cross-reference Technical Notes to know to assert `"Fira Code"` rather than `var(--ac-font-mono)` or another form, making the criterion not self-contained.

- 🔵 **Testability**: Token-consumption requirement has no corresponding acceptance criterion
  **Location**: Requirements; Acceptance Criteria
  The requirement that every component uses `--ac-*` tokens (no hard-coded colours) appears in Requirements and Summary but has no AC to verify it. A component using hard-coded hex values would satisfy all six criteria while violating this requirement — the intent is silently unverified.

---

### Strengths

- ✅ The dependency on 0033 is explained with precise technical detail — what tokens exist, what attribute name is used, and why they are currently inert — removing all ambiguity about the pre-condition for this story.
- ✅ Acceptance criteria follow a strict Given/When/Then structure throughout, naming the actor, trigger, and observable system state with specificity.
- ✅ Technical Notes resolve potentially ambiguous design decisions (localStorage key names, boot-script placement, `suppressHydrationWarning`) before they become implementation guesswork.
- ✅ The Assumptions section explicitly names the CSP constraint and its consequence, making a non-obvious risk visible to the implementer.
- ✅ Both upstream blockers (0033, 0035) and the 'Blocks: none' declaration are explicit and correct.
- ✅ The Drafting Notes proactively justify bundling theme and font-mode into one story on tight-coupling grounds — the decision is well-reasoned and documented.
- ✅ The localStorage unavailability AC is unusually thorough — it names the exact exception type (SecurityError) and specifies no uncaught exception, giving a clear pass/fail observable.
- ✅ The OS fallback AC covers both dark and non-dark branches explicitly, avoiding ambiguity about the else case.

---

### Recommended Changes

1. **Reconcile the `[data-font="mono"]` CSS block gap** (addresses: `[data-font="mono"]` major finding)
   Remove the claim from Context that 0033 ships the `[data-font="mono"]` block. Add an explicit requirement bullet covering authoring that block. Update Dependencies to note that 0033 is partially complete and the missing CSS block is an in-scope deliverable of this story.

2. **Add a verification procedure to the boot-script placement AC** (addresses: boot-script testability major)
   Reframe AC6 as a verifiable outcome, e.g. "Verified by inspecting the rendered HTML and confirming the inline `<script>` appears before all `<link>` tags; `suppressHydrationWarning` confirmed on `<html>` in the React component tree; no React hydration warning in the console."

3. **Anchor the token resolution AC to the known token block** (addresses: token resolution testability major)
   Replace "every `--ac-*` token resolves to its theme-appropriate value" with a reference to the specific block: "all tokens in the `[data-theme="dark"]` block at `global.css:138–160` resolve to their overridden values, confirmed by a parity assertion in `global.test.ts`."

4. **Tighten or scope the "every component" requirement** (addresses: unbounded scope cross-cutting theme)
   If the component token sweep is already complete, reframe the requirement as a verification gate ("no `--color-*` or hard-coded hex values appear in components touched by this story"). If it is new work, scope it explicitly. Remove or qualify the same language in the Summary. Add an AC to make it testable.

5. **Add the expected CSS value to the font-mode AC** (addresses: font-mode testability minor)
   Update AC2 to state the explicit expected value: "`--ac-font-display` and `--ac-font-body` both compute to `"Fira Code"` on `document.documentElement`."

6. **Move the CSP constraint to Dependencies** (addresses: dependency minor)
   Add a Dependencies entry: "Requires confirmation that the deployment's CSP permits the inline boot script (see Assumptions)."

7. **Clarify the When clause in ACs 1 and 2** (addresses: clarity minor)
   Replace "when the toggle fires" with the specific observable event, or collapse Given/When into a single precondition clause to avoid the redundancy.

---

*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is written with above-average clarity: requirements are specific, the context section accurately sets up the problem, and most technical details are concrete and actionable. Two notable issues exist: a contradiction between the Context section and the Technical Notes section about what 0033 actually delivered (the `[data-font="mono"]` CSS block), and a referent ambiguity around what "the toggle fires" means in two acceptance criteria.

**Strengths**:
- The dependency on 0033 is explained with precise technical detail, removing all ambiguity about the pre-condition for this story.
- Acceptance criteria follow a strict Given/When/Then structure throughout with specificity.
- Technical Notes resolve potentially ambiguous design decisions before they become implementation guesswork.
- The Assumptions section explicitly names the CSP constraint and its consequence.
- Actor identity is consistent throughout.

**Findings**:

- **Severity**: major | **Confidence**: high | **Location**: Context
  **Title**: Context and Technical Notes contradict each other on what 0033 delivered
  Context states 0033 ships the `[data-font="mono"]` CSS block; Technical Notes explicitly state it is missing and must be written by the 0034 implementer. An implementer who reads Context but not Technical Notes will omit the block, causing the font-mode toggle to have no visual effect — a silent, hard-to-diagnose failure.

- **Severity**: minor | **Confidence**: high | **Location**: Acceptance Criteria
  **Title**: "When the toggle fires" is an undefined trigger in two criteria
  Both ACs 1 and 2 use "when the toggle fires" as the When clause, which simply restates the Given. The Given and When are indistinguishable, making the criteria slightly harder to verify mechanically.

- **Severity**: minor | **Confidence**: medium | **Location**: Requirements
  **Title**: "Every component consumes `--ac-*` tokens" is an unbounded universal claim
  Could mean all components in the repo, all touched by this story, or all already migrated. Depending on interpretation, this either massively expands scope or is trivially true.

- **Severity**: suggestion | **Confidence**: medium | **Location**: Requirements
  **Title**: "Context hook" used without prior definition
  Recoverable from Technical Notes for the target audience, but a brief parenthetical on first use would improve self-containment.

---

### Completeness

**Summary**: Work item 0034 is substantially complete and well-populated across all required sections for a story type. One minor gap: the `[data-font="mono"]` prerequisite issue is buried in Technical Notes rather than surfaced as an explicit requirement or Open Question.

**Strengths**:
- Summary is precise and action-oriented.
- Context clearly explains why the work exists.
- Six acceptance criteria covering happy paths and the localStorage-unavailable edge case.
- Dependencies section is explicit and bidirectional.
- Technical Notes are unusually thorough.
- Assumptions surface a genuine non-obvious CSP constraint.
- Frontmatter is fully populated.

**Findings**:

- **Severity**: minor | **Confidence**: medium | **Location**: Technical Notes
  **Title**: Unresolved prerequisite gap buried in Technical Notes rather than Open Questions or Requirements
  The missing `[data-font="mono"]` block is noted deep in Technical Notes. An implementer reading only Requirements and ACs would not know they are responsible for authoring it, risking omission during review or estimation.

---

### Dependency

**Summary**: Primary blockers 0033 and 0035 are captured. However, 0033 is listed as a satisfied blocker despite Technical Notes revealing its font-mode deliverable is incomplete; the CSP constraint is in Assumptions rather than Dependencies; and the per-component token sweep implied by the design gap document is an unnamed prerequisite.

**Strengths**:
- Both upstream blockers (0033, 0035) are explicitly named with "Blocked by" notation.
- "Blocks: none" is declared explicitly and correctly.
- The CSP constraint is at least mentioned in Assumptions.

**Findings**:

- **Severity**: major | **Confidence**: high | **Location**: Dependencies
  **Title**: 0033 listed as a satisfied blocker but Technical Notes reveal it delivered incomplete output
  0033 has not fully discharged its role — the `[data-font="mono"]` CSS block is absent. An implementer reading only Dependencies will believe 0033 is complete and discover the gap mid-sprint.

- **Severity**: minor | **Confidence**: high | **Location**: Assumptions / Dependencies
  **Title**: CSP coupling named in Assumptions but absent from Dependencies
  The CSP prerequisite is a cross-team action item that belongs in Dependencies alongside 0033 and 0035 so it is visible during sprint planning.

- **Severity**: minor | **Confidence**: medium | **Location**: Context / Dependencies
  **Title**: Per-component token-migration prerequisite not reflected in Dependencies
  The design gap document's sequencing implies a component token sweep must precede theming. If incomplete, the toggle will silently fail in unmigrated components. Whether this is done or a remaining prerequisite is not stated.

---

### Scope

**Summary**: Well-scoped overall. The bundling of theme and font-mode is justified in Drafting Notes. One minor issue: the "every component consumes `--ac-*` tokens" requirement adds unbounded cross-cutting scope relative to the rest of the story.

**Strengths**:
- Drafting Notes explicitly justify bundling both toggles as one tightly-coupled story.
- Summary, Requirements, and ACs are in tight alignment (with the noted exception).
- Story size is appropriate — larger than a chore, clearly below epic.
- Service boundary ownership is clean: single frontend codebase, single owning context.
- Dependency relationships correctly bound what is in-scope here versus sibling stories.

**Findings**:

- **Severity**: minor | **Confidence**: medium | **Location**: Requirements
  **Title**: Token-compliance sweep requirement adds unbounded cross-cutting scope
  "Ensure every component consumes `--ac-*` tokens" could span the entire component library. If a full sweep is pre-existing, the requirement should be reframed as a verification gate. If new work, it needs explicit scoping.

---

### Testability

**Summary**: Six concrete Given/When/Then criteria cover primary behaviours well. Two major gaps: the token-resolution AC uses unbounded "every" language without a defined verification method; the boot-script placement AC is an implementation instruction without a testable outcome. Two minor gaps around font-mode expected value and the untestable token-consumption requirement.

**Strengths**:
- All six ACs use Given/When/Then consistently with explicit preconditions, triggers, and outcomes.
- The localStorage-unavailability AC names the exact exception type (SecurityError) and specifies no uncaught exception — a clear pass/fail observable.
- The OS fallback AC covers both dark and non-dark branches explicitly.
- The persistence AC specifies "before any paint" rather than "eventually", anchoring it to a concrete timing guarantee.

**Findings**:

- **Severity**: major | **Confidence**: high | **Location**: Acceptance Criteria (criterion 1)
  **Title**: Token resolution clause is unbounded and unverifiable as stated
  "Every `--ac-*` token resolves to its theme-appropriate value" has no defined enumeration, no reference value, and no verification method. The Technical Notes already reference `global.css:138–160` and `global.test.ts:167` — the AC should reference these directly.

- **Severity**: major | **Confidence**: high | **Location**: Acceptance Criteria (criterion 6)
  **Title**: Boot-script placement criterion lacks a defined verification procedure
  The structural requirement is clear but there is no mention of how to verify it — no DOM inspection test, build-artefact check, or browser devtools step. It functions as an implementation instruction, not a verifiable outcome.

- **Severity**: minor | **Confidence**: medium | **Location**: Acceptance Criteria (criterion 2)
  **Title**: Font-mode AC does not specify what "repoint to Fira Code" is verifiable against
  The expected CSS computed value is implicit. A verifier must cross-reference Technical Notes to know to assert `"Fira Code"` rather than a variable reference, making the criterion not self-contained.

- **Severity**: minor | **Confidence**: medium | **Location**: Requirements / Acceptance Criteria
  **Title**: Token-consumption requirement has no corresponding acceptance criterion
  The requirement that every component uses `--ac-*` tokens (no hard-coded colours) has no AC. A component with hard-coded hex values would satisfy all six criteria while violating this requirement.

## Re-Review (Pass 2) — 2026-05-07

**Verdict:** REVISE

### Previously Identified Issues

- ✅ **Clarity**: "When the toggle fires" redundant trigger — Resolved (removed from AC1 and AC2)
- ✅ **Dependency**: CSP coupling absent from Dependencies — Resolved (added as explicit blocked-by entry)
- ✅ **Testability**: Font-mode AC lacked expected computed value — Resolved (AC2 now specifies `"Fira Code"` on `document.documentElement`)
- 🟡 **Clarity + Dependency**: `[data-font="mono"]` contradiction / 0033 incomplete blocker — Partially resolved. Context and Technical Notes are now consistent, and Dependencies names what 0033 did and did not deliver. However, the Drafting Notes still contain the contradicting claim ("0033 shipped...font-mode swap token values"), which reintroduces the inconsistency for any reader who reaches that section.
- 🟡 **Testability**: Token resolution AC unbounded — Partially resolved. AC1 now references `global.css:138–160` and `global.test.ts`, but the "parity assertion" is not defined in the criterion itself, leaving a reader scanning AC1 in isolation unable to confirm whether the test already exists or must be authored.
- 🟡 **Testability**: Boot-script placement criterion lacked verification — Partially resolved. AC6 now includes concrete verification steps, but the two checks (HTML script ordering; React hydration warning) are bundled into a single pass/fail criterion and operate on different artefacts, making independent failure diagnosis harder.
- 🔵 **Dependency**: Token-migration prerequisite not reflected — Partially resolved (CSP now in Dependencies). The broader sequencing concern — which downstream component re-skin stories depend on this one — remains unaddressed.
- 🔵 **Scope**: Token-compliance sweep unbounded — Partially resolved. Scoped to "components touched by this story" but this set is undefined, leaving the boundary ambiguous.
- 🔵 **Testability**: Token-consumption requirement had no AC — Partially resolved. The requirement is now scoped to components touched by this story, but still has no corresponding acceptance criterion.
- 🔵 **Clarity**: "Context hook" undefined on first use — Not addressed (suggestion-level; acceptable for the target audience).

### New Issues Introduced

- 🟡 **Clarity**: Drafting Notes contradict the corrected Context — After updating Context to state the `[data-font="mono"]` block was not delivered by 0033, the Drafting Notes were not updated and still read "0033 shipped...font-mode swap token values", reintroducing the exact contradiction that was removed from Context.
- 🟡 **Dependency**: Bidirectional coupling with 0035 not resolved — The Dependencies section lists 0035 as a blocker (Topbar must exist to host the toggles), but Requirements also states that the topbar controls are "delivered by 0035" — implying 0035 depends on the hooks from 0034. Neither the ordering nor the handoff boundary (which story owns what) is defined, creating a potential circular dependency that will surface at sprint planning.
- 🟡 **Dependency**: Downstream component re-skin work not captured as Blocks — The source design-gap document explicitly states that all component re-skin work is sequenced after theme and font-mode wiring. "Blocks: none" is therefore inaccurate; at minimum the relationship should be acknowledged.
- 🟡 **Testability**: AC3 "before any paint" sub-clause has no verification procedure — The criterion states preferences are applied "before any paint" but provides no method to verify the timing guarantee (Performance trace, automated test, etc.), making the primary flash-prevention claim unverifiable as written.
- 🔵 **Clarity**: "Parity assertion" in AC1 is an undefined referent — A reader scanning AC1 cannot confirm whether the parity assertion pre-exists in `global.test.ts` or must be authored as part of this story, without cross-referencing the Technical Notes.
- 🔵 **Clarity**: "Components touched by this story" is an underspecified set — No rule or list defines which components are in scope for the token-compliance verification, leaving different implementers to draw the boundary differently.
- 🔵 **Dependency**: CSP blocker lacks an owner or resolution path — The blocked-by entry was added but names no team or individual responsible for confirmation, leaving the blocker without a resolution mechanism.
- 🔵 **Scope**: Topbar requirement delegates ownership to 0035 — The Requirements list includes "Surface a Toggle theme and a Toggle font control in the Topbar (delivered by 0035)", which is work owned by a different story. This creates ambiguity about which work item owns verification of the toggle controls' presence.
- 🔵 **Testability**: AC6 bundles two independent verification concerns — HTML script-ordering check and React hydration-warning check are structurally independent and can fail independently; bundling them prevents targeted pass/fail recording.
- 🔵 **Testability**: AC3 does not specify which localStorage keys a tester should set to exercise the persistence path — The Technical Notes suggest `"ac-theme"` and `"ac-font-mode"` but frame these as suggestions, not canonical names, so the criterion is not reliably reproducible without reading the implementation.

### Assessment

Significant progress was made: three pass-1 findings are fully resolved and three more are partially resolved. However, the edits also surfaced previously hidden issues — most notably an unresolved circular dependency between 0034 and 0035 that needs architectural clarification, a "Blocks: none" claim that the design-gap document contradicts, and a new Drafting Notes inconsistency introduced by the Context fix. Four new major findings prevent approval. The most impactful next steps are: (1) update the Drafting Notes to match the corrected Context; (2) resolve the 0034/0035 ordering and handoff boundary; (3) add a verification procedure to AC3's "before any paint" clause; and (4) clarify the downstream Blocks relationship.
