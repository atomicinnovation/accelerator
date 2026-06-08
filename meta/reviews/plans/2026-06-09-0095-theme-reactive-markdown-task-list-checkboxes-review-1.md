---
type: plan-review
id: "2026-06-09-0095-theme-reactive-markdown-task-list-checkboxes-review-1"
title: "Plan Review: Theme-Reactive Markdown Task-List Checkboxes Implementation Plan"
date: "2026-06-09T06:31:46+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
parent: "plan:2026-06-09-0095-theme-reactive-markdown-task-list-checkboxes"
target: "plan:2026-06-09-0095-theme-reactive-markdown-task-list-checkboxes"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability, compatibility]
review_number: 1
review_pass: 3
tags: [visualiser, markdown, theme, checkbox, accessibility, plan-review]
last_updated: "2026-06-09T07:43:28+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Theme-Reactive Markdown Task-List Checkboxes Implementation Plan

**Verdict:** REVISE

The plan is unusually well-researched and disciplined — it correctly identifies
the react-markdown override map as the extension point, mirrors the 0094
verification template, catches the banned two-arg `var()` form, and gets every
piece of arithmetic right (the `EXCEPTIONS` ledger counts, the `AC5_FLOOR`
981→988 bump, and the contrast figures all independently verified). However, it
carries **one ship-blocking correctness defect**: the `li`/`ul` overrides handle
only **tight** task lists. For a **loose** task list (items separated by a blank
line — common in real plan/research markdown), `mdast-util-to-hast` keeps the
`<input>` nested inside a `<p>`, so the override neither reads `checked` nor
strips the native control, re-introducing the exact dark-mode bug being fixed —
and every proposed test uses a tight list, so it ships green. Two further
structural issues (a Rules-of-Hooks violation from a conditionally-called
`useId()`, and the `aria-disabled` vs `aria-readonly` semantics for a read-only
checkbox) and several test-coverage gaps should be addressed before
implementation.

### Cross-Cutting Themes

- **Tight-list-only handling / loose-list defect** (flagged by: correctness 🔴,
  compatibility 🟡, architecture 🔵, test-coverage 🟡) — The override reads the
  `<input>` and label as direct `<li>` children, which only holds for tight
  lists (`list-item.js:70-71` unwraps the `<p>` only when `!loose`). A loose
  list keeps the `<p>` wrapper, so `node.children.find(tagName==='input')`
  returns `undefined` (→ `checked` collapses to `false`) and the
  `Children.filter(type==='input')` fails to remove the input (→ native control
  renders inside the label). All task-list test inputs are tight, so the defect
  is invisible to CI.
- **`useId()` called conditionally → Rules of Hooks** (flagged by: architecture
  🟡, code-quality 🟡, correctness 🔵) — `useId()` is called after the early
  `if (!isTaskItem(node)) return …`. react-markdown reuses one `li` component
  for every list item, so a document mixing task and non-task items varies the
  hook count and can throw "rendered more/fewer hooks".
- **Coupling to dependency internals under caret ranges** (flagged by:
  compatibility 🟡, architecture 🔵, code-quality 🔵) — The override depends on
  `mdast-util-to-hast`'s internal `<p>`-unwrapping, the `task-list-item` class
  name, and `c.type === 'input'` (only correct because no `input` override
  exists), all behind `^9`/`^4`/`^13` ranges with no pin or guard test.
- **Read-only checkbox ARIA semantics** (flagged by: usability 🟡, standards
  🟡) — `aria-disabled` models the native `disabled`, but the intent ("shows
  state, cannot be changed") is `aria-readonly`; some screen readers
  de-emphasise/skip disabled controls, suppressing the very state announcement
  the override exists to preserve. Also a divergence from FilterPill's
  role-on-parent pattern that the plan should reconcile.
- **Acceptance-criterion coverage gaps** (flagged by: test-coverage 🟡🟡) —
  AC3's "no list marker" and AC4's "not-done label is normal" halves are not
  asserted by any real-cascade check.

### Tradeoff Analysis

- **Prototype/FilterPill parity vs WCAG contrast**: the `#ffffff` tick is ~2.9:1
  on the dark accent (below 3:1). Keeping it matches the frozen prototype and
  the accepted FilterPill precedent (the user's chosen resolution); usability
  asks only that the manual-verification gate be made concrete (confirm the
  state is distinguishable at 17px and note the redundant cues — accent fill +
  struck label — that carry state alongside the tick). No code change needed;
  tighten the manual note.
- **ARIA placement — role-on-box vs role-on-parent**: usability judges the
  plan's role-on-box choice *better* than FilterPill's role-on-`<li>` (it
  preserves list-item semantics and item count), while standards flags the
  inconsistency between the two faux-checkboxes. Recommendation: keep role-on-box
  but record the one-line rationale (read-only standalone control vs FilterPill's
  interactive menuitem) so the divergence is deliberate and documented.

### Findings

#### Critical

- 🔴 **Correctness**: Override handles only tight task lists; loose lists
  re-render the native `<input>` and lose checked state
  **Location**: Phase 1, Section 4: `li` and `ul` overrides
  The `<input>` is unshifted into a `<p>` that is only unwrapped for tight lists
  (`list-item.js:43-48` vs `:70-71`). A loose task list leaves the input nested
  in a `<p>`, so both the hast `checked` read and the React child-filter miss
  it — a raw native `disabled` checkbox renders inside the label and every
  loose `[x]` is mislabelled unchecked, reintroducing the dark-mode bug. Fix by
  recursing into the `<p>` wrapper for both the `checked` read and the label
  filter, and add loose-list cases to the unit test and the Phase 2 fixture.

#### Major

- 🟡 **Architecture / Code Quality**: `useId()` called conditionally after an
  early return violates the Rules of Hooks
  **Location**: Phase 1, Section 4: `li` override
  Lift the task-item branch into a dedicated `TaskListItem` child component that
  always calls `useId()` at its top, so hook order is stable regardless of item
  type.

- 🟡 **Correctness**: All task-list test inputs are tight, so the loose-list
  defect ships green
  **Location**: Phase 2, Section 1 (fixture) + Sections 2/3 (specs)
  The unit input `'- [x] done\n- [ ] todo\n'` and the appended fixture are both
  tight. Add a blank-line-separated loose list to both, with an
  `input[type="checkbox"]` `toHaveCount(0)` assertion over the loose list.

- 🟡 **Test Coverage**: AC3 "no list marker" is asserted nowhere
  **Location**: Phase 2, Section 2 (resolved-styles spec)
  Only the CSS-as-text guard checks the `list-style: none` *string*; no
  real-cascade check asserts the rendered `ul`/`li` computes
  `list-style-type: none`. A dropped/mis-scoped `.tasklist` rule would
  reintroduce bullets with every test green.

- 🟡 **Test Coverage**: The "not-done label renders as normal text" half of AC4
  is untested
  **Location**: Phase 2, Section 2 (resolved-styles spec)
  Only the done label is asserted; CSS applying muting/line-through
  unconditionally would pass. Add the negative assertion (unchecked label:
  `text-decoration-line: none`, non-muted colour).

- 🟡 **Test Coverage**: Unconditional `CHECKED_BOX`/`DONE_LABEL` locators don't
  fail loudly on wrong structure
  **Location**: Phase 2, Section 2 (resolved-styles spec)
  Add `toHaveCount(1)` on `CHECKED_BOX`, `UNCHECKED_BOX`, and `DONE_LABEL`
  before `.evaluate(...)`, mirroring the explicit native-input `toHaveCount(0)`,
  so a missing checked box or a duplicate fails behaviourally rather than via a
  generic locator timeout.

- 🟡 **Usability**: Read-only checkbox should use `aria-readonly`, not
  `aria-disabled`
  **Location**: Accessibility design; Phase 1 §4 + §1
  `aria-disabled` can cause some AT to de-emphasise/skip the control,
  suppressing the checked/unchecked announcement the override exists to
  preserve. `aria-readonly="true"` is the canonical semantic for a
  state-bearing non-editable checkbox; update the unit assertion and make the
  manual screen-reader step confirm the announcement on NVDA/VoiceOver.

- 🟡 **Standards**: ARIA placement diverges from the FilterPill precedent
  without recorded rationale
  **Location**: Accessibility design; Phase 1 §4
  FilterPill puts `role="menuitemcheckbox"`+`aria-checked` on the parent `<li>`
  with an `aria-hidden` box; this plan puts `role="checkbox"` on the box span.
  Record the one-line rationale (standalone read-only control vs interactive
  menuitem) so two patterns for one visual primitive are intentional.

- 🟡 **Compatibility**: Override depends on `mdast-util-to-hast` internal
  `<p>`-unwrapping under a caret range
  **Location**: Phase 1, Section 4; Current State Analysis
  Same root as the critical loose-list defect, viewed as upgrade fragility:
  the unwrapping is an undocumented internal of a transitive `^13` dependency.
  Resilient recursive child handling (the critical fix) also resolves this;
  consider pinning or a guard test.

- 🟡 **Compatibility**: `c.type === 'input'` filter is contingent on no `input`
  override existing
  **Location**: Phase 1, Section 4 (`li` override child filter)
  Correct only because the components map registers no `input` key; adding one
  later silently stops the match. Prefer filtering the hast `node.children`
  (drop the `input` element, render the rest) over post-render React-element
  `.type`.

- 🟡 **Compatibility**: Caret-ranged markdown deps with no guard against
  breaking upgrades
  **Location**: References / Migration Notes
  `react-markdown ^9`, `remark-gfm ^4`, transitive `mdast-util-to-hast ^13` are
  unpinned (only the lockfile freezes them) while the plan adds three new
  structural couplings. Pin to exact versions (as `@dnd-kit/*` already are) or
  add a tight+loose unit guard so a floated upgrade fails loudly.

#### Minor

- 🔵 **Architecture**: Override couples to a private `mdast-util-to-hast`
  implementation detail — note the tight/loose assumption explicitly as a
  constraint. **Location**: Current State Analysis / Phase 1 §4
- 🔵 **Architecture / Usability**: Mixed-list fallback (`items.every(isTaskItem)`)
  degrades silently and is untested — add a unit case pinning the chosen
  degradation (markers retained, items still boxed, no crash).
  **Location**: Testing Strategy → Key edge cases
- 🔵 **Architecture**: First per-element-branching override — centralise the
  `isTaskItem` predicate and the `ul`/`li` agreement in one named helper so the
  coupling is explicit. **Location**: Implementation Approach / Phase 1 Overview
- 🔵 **Code Quality**: Repeated inline structural casts on `node.children`
  obscure intent — import `hast` `Element`/`ElementContent` and write one typed
  extraction helper. **Location**: Phase 1 §4
- 🔵 **Code Quality**: Magic strings `'input'` / `'task-list-item'` carry
  load-bearing meaning — extract to named constants with a comment pointing at
  `list-item.js`. **Location**: Phase 1 §4
- 🔵 **Code Quality / Correctness**: Task branch renders `<li className=…>`
  without forwarding `{...rest}`, dropping any other props react-markdown passes
  — spread `{...rest}` on the task branch too. **Location**: Phase 1 §4
- 🔵 **Correctness**: `useId()` placement is also a re-render hazard if a list
  item flips task↔non-task at the same position (the existing resolver-rotation
  `useMemo` re-runs the pipeline) — call `useId()` unconditionally.
  **Location**: Phase 1 §4
- 🔵 **Test Coverage / Compatibility**: Exact `borderTopWidth === '1.5px'`
  comparison risks sub-pixel/DPR flakiness — relax to a ~0.1px tolerance or rely
  on the screenshot for the width. **Location**: Phase 2 §2
- 🔵 **Test Coverage**: Mixed/loose/non-task forwarding branches (which underpin
  the "zero baseline impact" claim) have no unit coverage — add a plain-list
  case asserting zero `role="checkbox"` boxes and a real marker.
  **Location**: Phase 1 §4 / Testing Strategy
- 🔵 **Test Coverage**: Focused VR screenshot at `maxDiffPixelRatio: 0.05` over a
  label-dominated crop may not discriminate small box/tick defects — tighten the
  ratio or confirm box area during baseline review. **Location**: Phase 2 §3
- 🔵 **Test Coverage**: Add a `textContent` assertion on each task `li` so the
  children-filter behaviour is pinned beyond the normalised accessible name.
  **Location**: Phase 1 §1
- 🔵 **Standards**: Read-only `role="checkbox"` may be announced as actionable —
  confirm `aria-disabled`/`aria-readonly` suppresses the affordance in target
  ATs or document the intent. **Location**: Phase 1 §4
- 🔵 **Standards**: The merged `5px` ledger reason mixes a spacing literal and a
  radius literal under one `(file, literal)` line — name both roles in the
  reason. **Location**: Phase 1 §6
- 🔵 **Standards**: The `#ffffff` ledger reason uses different phrasing from
  FilterPill's identical white-on-accent entry — align the two for consistency.
  **Location**: Phase 1 §6
- 🔵 **Standards**: Note that the task classes intentionally follow the
  `.codeblock` bare-class convention (not `.markdown`-scoped) and confirm label
  prose-typography inheritance. **Location**: Phase 1 §5
- 🔵 **Compatibility**: `useId()` introduces a React 18+ floor in MarkdownRenderer
  and emits colon-containing ids — fine under the React 19 pin; note the floor in
  a comment. **Location**: Phase 1 §4

#### Suggestions

- 🔵 **Usability**: Consider a redundant visually-hidden "checked"/"not checked"
  text cue on the label as belt-and-braces against AT-specific read-only-checkbox
  handling, and confirm the state announces on ≥2 screen readers.
  **Location**: Phase 1 §4 / §1; Desired End State

### Strengths

- ✅ Correctly reuses the react-markdown `components` override map (the `pre`
  precedent) rather than a heavier remark transform, keeping the change
  module-scoped and consistent with the existing renderer.
- ✅ All quantitative claims independently verify: the `EXCEPTIONS` literal
  counts (`1.5px`×1, `2px`×2, `5px` 1→2, `6px`×1, `9px`×1, `17px`×2, `#ffffff`×1),
  the `AC5_FLOOR` +7 bump, and the WCAG contrast figures (2.90:1 dark / ~5.37:1
  light) are all accurate.
- ✅ Catches the banned two-arg `var(--token, fallback)` form and converts to
  single-arg `var()`, with the moot-fallback reasoning spelled out.
- ✅ Test-first throughout: the obsolete native-checkbox unit test is rewritten,
  tests query by ARIA role/state rather than hashed CSS-module class names, and
  the cross-theme divergence assertion guards against theme-invariant tokens
  passing both branches trivially.
- ✅ Accessibility is treated as a first-class requirement: `aria-labelledby` +
  `useId()` supplies an accessible name (with a `getByRole('checkbox', {name})`
  assertion), the tick is `aria-hidden`, and the role sits on the box so
  list-item semantics survive.
- ✅ Clean, independently-mergeable two-phase split with the no-fixture-yet
  observation justifying why Phase 1 moves no baselines; append-only fixture edit
  preserves the 0094 `.first()` locator-stability invariant.
- ✅ Reliability hygiene in the VR specs: `document.fonts.ready`,
  `animations: 'disabled'`, shared `setTheme`/`resolveToken`/`applyTheme` helpers.

### Recommended Changes

1. **Handle loose task lists** (addresses: the critical correctness finding +
   the compatibility internal-coupling finding). In the `li` override, when the
   matched `<li>` child is a `<p>`, recurse one level: scan that `<p>`'s children
   for the `input` (for the hast `checked` read) and filter the input out of the
   flattened label children. Add a blank-line-separated loose task list to both
   the Phase 1 unit test and the Phase 2 fixture, with an explicit
   `input[type="checkbox"]` `toHaveCount(0)` over the loose list.

2. **Make `useId()` unconditional** (addresses: the architecture + code-quality
   major and the correctness minor). Extract the task-item rendering into a
   `TaskListItem` component that calls `useId()` at its top; the `li` override
   only chooses between `TaskListItem` and a plain forwarded `<li>`.

3. **Switch read-only ARIA to `aria-readonly` and firm up the a11y rationale**
   (addresses: usability major + standards majors). Use `aria-readonly="true"`
   (in place of, or alongside, `aria-disabled`), update the unit assertion, add a
   one-line rationale for role-on-box vs FilterPill's role-on-parent, and make the
   manual screen-reader step confirm the state announcement.

4. **Close the AC3 / AC4 coverage gaps** (addresses: the two test-coverage
   majors). Assert computed `list-style-type: none` on the rendered task
   `ul`/`li`, and add the unchecked-label negative assertion
   (`text-decoration-line: none`, non-muted colour).

5. **Add locator cardinality guards** (addresses: the locator test-coverage
   major). `toHaveCount(1)` on `CHECKED_BOX`, `UNCHECKED_BOX`, and `DONE_LABEL`
   before evaluating computed styles.

6. **Decide dependency-pin posture** (addresses: the caret-range compatibility
   major). Either pin `react-markdown`/`remark-gfm`/`mdast-util-to-hast` to exact
   versions, or rely on the new tight+loose guard test as the loud-failure
   mechanism — and prefer filtering the hast `node.children` over the
   `c.type === 'input'` React-element check to drop the no-`input`-override
   coupling.

7. **Apply the minor polish** (addresses: the minor findings). Forward `{...rest}`
   on the task `<li>`; extract `'input'`/`'task-list-item'` to named constants
   with a `list-item.js` comment; import `hast` types for a single typed
   extraction helper; relax the `1.5px` assertion to a tolerance; add a
   mixed-list and a plain-list unit case; refine the `5px`/`#ffffff` ledger reason
   text; and note the bare-class convention + React 18+ floor.

## Per-Lens Results

### Architecture

**Summary**: Architecturally sound at the structural level — correctly reuses
the `components` override map (the `pre` precedent), keeps the change
module-scoped, preserves DOM boundaries by destructuring `node` away, and uses a
well-reasoned independently-mergeable two-phase split. The most consequential
concern is a Rules-of-Hooks violation in the `li` override; secondary concerns
are coupling to a private node_modules detail and the silently-degrading,
untested mixed-list fallback.

**Strengths**:
- Reuses the established override map rather than inventing a mechanism; first
  override beyond `pre`, consistent with precedent.
- Module-scoped styling consistent with the rest of MarkdownRenderer and the
  `.codeblock` precedent; the global-`ac-md-*` rejection is justified.
- Destructures the hast `node` away from the spread so AST shape never leaks.
- Theme-reactivity by token consumption only; no dark-block edit ("dark mode is
  free").
- Independently-mergeable two-phase decomposition; the no-fixture-yet observation
  justifies Phase 1 moving no baselines.

**Findings**:
- 🟡 (high) **useId() called conditionally after an early return violates the
  Rules of Hooks** — Phase 1 §4. react-markdown invokes `li` as a component for
  every item, so a mixed document varies the hook count → "rendered more/fewer
  hooks" or state corruption. Extract a `TaskListItem` child that always calls
  `useId()`.
- 🔵 (high) **Override couples to private mdast-util-to-hast `<p>`-unwrapping** —
  Current State / Phase 1 §4. Loose lists keep the `<p>`, breaking the `checked`
  read and label filter. Note the tight/loose assumption and test the loose case.
- 🔵 (medium) **Mixed-list fallback degrades silently and is untested** —
  Testing Strategy. Add a unit test pinning markers-retained + items-still-boxed
  + no-crash.
- 🔵 (medium) **First per-element-branching override sets a precedent worth
  naming** — Implementation Approach. Centralise `isTaskItem` and the `ul`/`li`
  agreement in one helper module.

### Code Quality

**Summary**: Unusually well-researched, closely mirrors the `pre`-override and
0094 conventions, with clear phasing and explicit ledger/ratchet discipline. The
main risk is the `li` override's conditional `useId()` plus repeated inline
structural casts and undocumented magic strings that hurt maintainability.

**Strengths**:
- Follows the `pre` override conventions (destructured `node`, verbatim
  forwarding of non-task lists).
- Tests written first and query by ARIA/structure, not hashed class names.
- Local `CheckIcon` modelled on the existing SortPill icon.
- Clear, self-contained two-phase split; ledger/ratchet changes enumerated with
  exact counts and reasons.
- Design deviations explicitly acknowledged with rationale.

**Findings**:
- 🟡 (high) **useId() after a conditional return** — Phase 1 §4. Same as
  architecture; lift to a dedicated component.
- 🔵 (high) **Repeated inline structural casts on node.children obscure intent**
  — Phase 1 §4. Import `hast` `Element`/`ElementContent`; one typed extraction
  helper.
- 🔵 (medium) **Magic strings 'input'/'task-list-item' are load-bearing but
  unexplained** — Phase 1 §4. Extract to named constants with a `list-item.js`
  comment.
- 🔵 (medium) **Task-detection logic duplicated between `ul` and `li`** — Phase 1
  §4. Keep the shared predicate and note both call sites must agree.

### Test Coverage

**Summary**: Rigorous and test-first; reuses the proven 0094 template (CSS-text
guards, real-cascade resolved-styles in both themes, focused screenshot,
cross-theme divergence) and maps each AC to a check. Gaps: AC3's no-marker and
AC4's not-done-label halves are not asserted in the real cascade, a couple of
locator/assertion robustness risks, and the mixed/loose/non-task branches are
uncovered.

**Strengths**:
- Genuinely test-first; obsolete unit test rewritten; assertions on ARIA/structure
  not hashed classes.
- Each AC traced to a check; correct jsdom-vs-Playwright split.
- Cross-theme divergence test resists theme-invariant-token false-passes.
- CSS-text guards + EXCEPTIONS/var-ban/AC5 ratchets as an independent second
  layer; the floor bump is anticipated.
- Reliability hygiene: append-only fixture, `fonts.ready`, `animations:'disabled'`,
  shared helpers.

**Findings**:
- 🔴 (high) **AC3 "no list marker" asserted nowhere** — Phase 2 §2. Only a
  CSS-text presence check exists; add a computed `list-style-type: none`
  assertion. *(Aggregated at major in the cross-lens summary; the loose-list
  critical dominates.)*
- 🟡 (high) **AC4 "not-done label is normal" untested** — Phase 2 §2. Add the
  negative assertion.
- 🟡 (medium) **Unconditional `CHECKED_BOX`/`DONE_LABEL` locators don't fail
  loudly** — Phase 2 §2. Add `toHaveCount(1)` guards.
- 🔵 (medium) **Exact `1.5px` border-width risks sub-pixel flakiness** — Phase 2
  §2. Tolerance or rely on screenshot.
- 🔵 (medium) **Mixed/loose/non-task branches uncovered; "zero baseline impact"
  unguarded** — Phase 1 §4. Add a plain-list unit case.
- 🔵 (medium) **Focused screenshot may not discriminate small box/tick defects**
  — Phase 2 §3. Tighten ratio or confirm during baseline review.
- 🔵 (low) **Add `textContent` assertion to pin children-filter behaviour** —
  Phase 1 §1.

### Correctness

**Summary**: Arithmetic is sound — the EXCEPTIONS counts, the AC5_FLOOR +7 bump,
and the contrast figures (white = 2.90:1 dark, ~5.37:1 light) all independently
verify. But there is one provable ship-blocking error: the override reads
`checked` and strips the input only for tight lists; loose lists keep the `<p>`
wrapper, collapsing `checked` to false and re-rendering a native `<input>` inside
the label. Phase 2 uses only tight lists, so it ships undetected.

**Strengths**:
- All quantitative claims verify exactly.
- Hast-vs-React consistency correctly reasoned for the tight case.
- `ul` override correctly excludes inter-element whitespace text nodes.
- `aria-checked`/`aria-disabled` render to the asserted strings; `useId()` keeps
  labelledby ids unique.

**Findings**:
- 🔴 (high) **Override handles only tight lists; loose lists re-render the native
  input and mislabel state** — Phase 1 §4. Recurse into the `<p>` wrapper for both
  the `checked` read and the label filter; add loose-list cases.
- 🟡 (high) **All task-list test inputs are tight, so the defect ships green** —
  Phase 2 §1/§2/§3. Add a loose list to the unit input and fixture with a
  `toHaveCount(0)` native-input assertion.
- 🔵 (medium) **`useId()` placement is a re-render hazard if an item flips
  task↔non-task** — Phase 1 §4. Call it unconditionally.
- 🔵 (medium) **Task branch doesn't forward `{...rest}`** — Phase 1 §4. Spread it
  on the task `<li>` too.

### Standards

**Summary**: Highly disciplined on the project's CSS-token conventions (banned
two-arg var() caught, EXCEPTIONS declared==observed with per-literal reasons,
strict AC5 ratchet bumped, the `#ffffff` deviation documented). Naming/placement
mirror established patterns. Main concerns are accessibility-convention
deviations (role-on-box vs FilterPill's role-on-parent, unreconciled) and minor
ledger-reason inconsistencies.

**Strengths**:
- Catches the banned two-arg `var()` and converts to single-arg with reasoning.
- New 0095 CSS-text guards added beside the 0094 block per convention.
- Exact EXCEPTIONS discipline with `kind:'irreducible'` + per-literal reasons.
- Strict AC5_FLOOR bump per the documented protocol.
- Module-scoped names mirror `.codeblock`; `#ffffff` documented as a conscious
  ADR-0026 appendix departure; append-only fixture preserves the locator
  invariant.

**Findings**:
- 🟡 (medium) **Deviates from FilterPill's role-on-parent ARIA pattern without
  recorded rationale** — Accessibility design / Phase 1 §4. Align or document the
  divergence.
- 🔵 (high) **Read-only `role="checkbox"` may be announced as actionable** —
  Phase 1 §4. Confirm `aria-disabled`/`aria-readonly` suppresses the affordance
  or document the intent.
- 🔵 (high) **`#ffffff` ledger reason differs in phrasing from FilterPill's
  identical case** — Phase 1 §6. Align the two entries.
- 🔵 (medium) **Merged `5px` ledger reason conflates spacing + radius
  categories** — Phase 1 §6. Name both roles in the reason.
- 🔵 (medium) **Task classes are bare (unscoped) like `.codeblock` but the plan
  doesn't state it's intentional** — Phase 1 §5. Add a note; confirm label prose
  inheritance.

### Usability

**Summary**: Deliberate about accessibility — preserves state via
`role="checkbox"`+`aria-checked`, supplies a name via `aria-labelledby`/`useId`,
hides the decorative tick, and keeps the role off the `<li>` (better than
FilterPill). The main concern is `aria-disabled` for a read-only control, which
can cause some screen readers to suppress the state announcement the plan is
trying to preserve.

**Strengths**:
- Accessible name explicitly engineered and unit-asserted (`getByRole('checkbox',
  {name})`).
- Role on the box, not the `<li>`, so list semantics/item count survive.
- Decorative tick is `aria-hidden`.
- State preservation is a first-class requirement with unit + manual coverage.
- Tests query by ARIA role/state, keeping the a11y contract under test.

**Findings**:
- 🟡 (medium) **Read-only checkbox should use `aria-readonly`, not
  `aria-disabled`** — Accessibility design / Phase 1 §4/§1. Disabled controls may
  be de-emphasised/skipped by AT, suppressing the announcement; switch to
  `aria-readonly` and confirm on NVDA/VoiceOver.
- 🔵 (medium) **Dark-theme ~2.9:1 tick contrast needs a firmer manual gate** —
  Key Discoveries / Phase 2 Manual Verification. Make the step concrete (state
  distinguishable at 17px; note redundant accent-fill + struck-label cues; record
  the measured ratio).
- 🔵 (high) **Mixed task/non-task lists render bullet + box inconsistently** —
  Testing Strategy / Phase 1 §4. Acceptable but document with a fixture/unit case.
- 🔵 (suggestion, medium) **Consider a redundant visually-hidden state cue** —
  Phase 1 §4/§1. Belt-and-braces against AT-specific read-only-checkbox handling.

### Compatibility

**Summary**: Verified the override's assumptions hold for the resolved versions
(react-markdown 9.1.0, remark-gfm 4.0.1, mdast-util-to-hast 13.2.1) and that
`c.type === 'input'` is correct because no `input` override is registered. The
chief concern is that every coupling is to an internal/implementation detail of
caret-ranged dependencies, with no pin and no guard test.

**Strengths**:
- Correctly identifies the live tight-list DOM shape and reads `checked` from the
  hast node.
- Assertions query by ARIA/`class*=` rather than hashed names (robust to CSS-module
  hash churn).
- Purely additive consumer-facing output; tokens exist in both themes; no VR
  movement on non-task lists.
- Preserves the announced checked/unchecked state for AT.

**Findings**:
- 🟡 (high) **Depends on mdast-util-to-hast internal `<p>`-unwrapping under `^13`**
  — Phase 1 §4 / Current State. Same root as the loose-list critical; recursive
  handling fixes it; consider pinning/guarding.
- 🟡 (high) **`c.type === 'input'` filter contingent on no `input` override** —
  Phase 1 §4. Filter the hast `node.children` instead, or comment + assert no
  `input` override.
- 🟡 (high) **Caret-ranged markdown deps with no upgrade guard** — References /
  Migration Notes. Pin to exact versions (as `@dnd-kit/*`) or add a tight+loose
  guard test.
- 🔵 (medium) **Sub-pixel `1.5px` assertion may not be stable across engine/DPR**
  — Phase 2 §2. Assert tokens; treat exact width as screenshot-covered.
- 🔵 (medium) **`useId()` introduces a React 18+ floor and colon-bearing ids** —
  Phase 1 §4. Fine under the React 19 pin; note the floor in a comment.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-06-09

**Verdict:** REVISE

The review-1 critical (tight-list-only handling) is **resolved** — the
`input → null` override + recursive `findCheckbox` + `TaskListItem` redesign is
the right approach and every review-1 major was addressed. However, the redesign
and the radius literal introduced **two new criticals**, so the plan needs one
more revision pass before it is implementation-ready.

### Previously Identified Issues

- 🔴 **Correctness**: Override handled only tight task lists — **Resolved**.
  `input → null` removes the control in both shapes; `findCheckbox` reads
  `checked` recursively; loose-list unit + e2e + fixture coverage added.
- 🟡 **Architecture / Code Quality**: conditional `useId()` — **Resolved**.
  Hoisted into `TaskListItem`, called unconditionally.
- 🟡 **Correctness**: tight-only test inputs — **Resolved**. Tight/loose/mixed/
  plain unit cases + loose fixture list + whole-root native-input e2e assertion.
- 🟡 **Test Coverage**: AC3 "no marker" untested — **Resolved**. Computed
  `list-style-type: none` assertion added.
- 🟡 **Test Coverage**: AC4 not-done half untested — **Resolved** (strengthen
  to positive equality recommended — see new minor).
- 🟡 **Test Coverage**: locator cardinality — **Resolved**. `toHaveCount`
  guards added (extend to `todoLabel` — new minor).
- 🟡 **Usability**: `aria-disabled` vs `aria-readonly` — **Resolved**. Switched
  to `aria-readonly` with rationale.
- 🟡 **Standards**: FilterPill ARIA divergence rationale — **Resolved**.
- 🟡 **Compatibility**: mdast `<p>`-unwrapping + `c.type==='input'` couplings —
  **Resolved**. Designed out via the `input → null` override.
- 🟡 **Compatibility**: caret-range deps — **Resolved**. Guard-over-pin posture
  recorded.
- All review-1 minors (hast types, named constants, `{...rest}` forwarding,
  `1.5px` tolerance, mixed/plain cases, ledger reason text, bare-class note,
  React 18+ floor, firmer contrast gate) — **Resolved**.

### New Issues Introduced

- 🔴 **Correctness — `{...rest}` clobbers the module className** (Phase 1 §4,
  `TaskListItem`). `{...rest}` is spread *after* the explicit `className`, and
  `rest` carries the hast-derived `className="task-list-item"`; JSX last-wins
  overwrites `styles.task`/`styles.taskDone`, so the `.taskDone` cascade (accent
  fill, strikethrough, flex layout) never applies. Introduced by the review-1
  "forward `{...rest}`" fix. Fix: destructure `className` out of the props
  (type with `ComponentPropsWithoutRef<'li'>`) and don't let it overwrite.
- 🔴 **Standards — `border-radius: 5px` violates the exception-less ADR-0039
  literal ban** (Phase 1 §5). `migration.test.ts:758-788`
  (`BORDER_RADIUS_LITERAL_RE`) rejects any literal `border-radius` with no
  EXCEPTIONS escape; the ladder has no 5px step (`--radius-0/1/2/3/4/6/8/12`).
  A second gate (`:824-838`) also fails if any irreducible reason contains the
  word "radius". Latent in the original plan; surfaced now. Fix: use
  `var(--radius-4)` (4px, within ADR-0026 ±2px tolerance); drop the `5px` ledger
  bump (no longer a second `5px` occurrence) and never put "radius" in a reason.
- 🟡 **Architecture / Compatibility — `input → null` is a global checkbox
  suppressor** (Phase 1 §4). It drops *every* `type="checkbox"` input, scoped
  only by the assumption "markdown produces no other input." Fix: also guard on
  `props.disabled` (the GFM task checkbox is always `disabled`), narrowing the
  blast radius to the GFM-injected control.
- 🟡 **Architecture — implicit cross-entry invariant**: `li`/`TaskListItem`
  correctness silently depends on the sibling `input → null` entry. Mitigated by
  the disabled-guard + a comment naming the invariant + the tight/loose count
  tests; acceptable with those.
- 🟡 **Code Quality — `TaskListItem` prop type omits the `...rest` it spreads**
  (Phase 1 §4). Same root as the className critical; fixed by typing props with
  `ComponentPropsWithoutRef<'li'>`.
- 🟡 **Test Coverage — loose list has no VR baseline** (Phase 2 §3). The focused
  screenshot captures only the tight list; the loose-list `<p>`-in-label layout
  the plan itself flags is only structurally covered. Fix: add a loose-list
  focused baseline (the regenerated full-page `library-doc-view` also captures
  it, but a focused baseline is the intended gate).
- 🟡 **Usability — loose-list label nests block `<p>` inside inline `<span>`**
  (Phase 1 §4). Invalid nesting that can perturb the `aria-labelledby`
  accessible name across AT. Fix: render the label as a block element
  (`<div className={styles.taskLabel}>`) and reset the inner `<p>` margins.

### New Minors / Suggestions

- 🔵 **Code Quality**: the illustrative React import line drops `Children`/
  `isValidElement` still used by the `pre` path — state the change as additive.
- 🔵 **Code Quality**: redundant `child.children as ElementContent[]` cast in
  `findCheckbox` (already `ElementContent[]` after the element guard).
- 🔵 **Test Coverage**: strengthen the not-done label to positive equality
  (`toBe(--ac-fg)`); add `toHaveCount(1)` to `todoLabel`; assert the mixed-list
  `<ul>` does NOT carry the `tasklist` class; assert the two boxes'
  `aria-labelledby` ids are distinct; assert checked-bg ≠ unchecked-bg (the
  redundant-cue guarantee the contrast deviation relies on).
- 🔵 **Compatibility**: note `mdast-util-to-hast` is transitive (governed by
  `react-markdown`'s range, needs an `overrides` entry to pin) and that the
  guard posture assumes lockfile-deterministic installs (`npm ci`).
- 🔵 **Usability**: the read-only box is non-focusable; the SR manual step should
  specify browse/virtual-cursor navigation, not Tab.

### Assessment

The architecture is now right and the loose-list defect is genuinely fixed. The
two new criticals are both narrow and mechanical (one className-merge fix, one
token substitution) — not a design problem. Once the `var(--radius-4)`
substitution, the className-merge, the `disabled`-scoped `input` override, the
block label element, and the loose-list baseline are applied (with the cheap
test hardenings), the plan should be ready. Recommend a final targeted re-check
of correctness + standards after these edits.

## Re-Review (Pass 3, targeted: correctness + standards) — 2026-06-09

**Verdict:** APPROVE

The pass-2 criticals are both verified resolved at the source, and the one new
issue they exposed has been corrected.

### Previously Identified Issues (pass 2)

- 🔴 **Correctness — `{...rest}` className clobber** — **Resolved**.
  `TaskListItem` destructures `className` and composes
  `[styles.task, checked && styles.taskDone, className].filter(Boolean).join(' ')`;
  `{...rest}` no longer carries `className`, so the `.taskDone` cascade applies.
  Props typed `{ checked } & ComponentPropsWithoutRef<'li'>`.
- 🔴 **Standards — `border-radius: 5px` literal ban** — **Resolved**. Corner is
  `var(--radius-4)` (declared token, 4px); no literal `border-radius`; the `5px`
  ledger entry stays at 1; no EXCEPTIONS reason mentions "radius"; `AC5_FLOOR`
  bumped 981→989 (+8 var refs incl. `var(--radius-4)`). Both reviewers
  re-verified the EXCEPTIONS arithmetic and the AC5 count independently.
- 🟡 **Architecture/Compatibility — global checkbox suppressor** — **Resolved**.
  `input → null` scoped to `type==='checkbox' && disabled`.
- 🟡 **Usability — invalid `<p>`-in-`<span>` nesting** — **Resolved**. Label is a
  block `<div>` with an inner-`<p>` margin reset.
- 🟡 **Code Quality — `TaskListItem` prop typing** — **Resolved**.
- 🟡 **Test Coverage — loose list no VR baseline** — **Resolved**. Added
  `task-list-loose-*` focused baselines.
- All pass-2 minors (additive import, dropped cast, not-done positive equality,
  `todoLabel` cardinality, mixed-list marker assertion, distinct
  `aria-labelledby`, checked≠unchecked bg, transitive-dep/`npm ci` note, SR
  browse-mode step) — **Resolved**.

### New Issues Introduced (by the pass-2 fixes)

- 🔴→✅ **Stale `5px` assertion** (flagged by correctness 🔴 + standards 🟡): the
  radius CSS moved to `var(--radius-4)` (4px) but the Phase 2 resolved-styles
  spec still asserted `borderTopLeftRadius` `'5px'`, and the Desired End State
  still said "5px radius" — a self-inconsistent, guaranteed-red test. **Fixed**:
  the spec now asserts `'4px'` (with comment), and the End State reads
  `var(--radius-4)` (4px).

### Residual (accepted, non-blocking)

- 🔵 **`--radius-4` (4px) vs `--radius-6` (6px)**: the prototype's 5px is
  equidistant from both and both are within ADR-0026's ±2px tolerance. Kept
  `--radius-4` (tighter corner suits the 17px box; FilterPill's faux-checkbox
  uses an even tighter `--radius-2`). Either is convention-compliant; no CI gate
  distinguishes them.
- 🔵 **Confirm observed `17px`/`#ffffff` counts** when the failing hygiene test
  first runs (the correctness reviewer read the live CSS and confirmed neither
  literal pre-exists in `MarkdownRenderer.module.css`, so declared==observed
  should hold) — already an implementation step in Phase 1 §6.

### Assessment

The plan is implementation-ready. Both criticals are resolved at the source, the
fix-induced stale assertion is corrected, and only two trivial, explicitly-
accepted minors remain (a within-tolerance radius choice and a routine
observed-count confirmation that is already a Phase 1 step). No further review
pass is warranted before implementation.
