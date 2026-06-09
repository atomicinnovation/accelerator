---
date: "2026-05-21T20:25:00Z"
type: plan-review
producer: review-plan
target: "plan:2026-05-21-0078-detail-page-frontmatter-table"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability]
review_pass: 2
status: complete
id: "2026-05-21-0078-detail-page-frontmatter-table-review-1"
title: "2026-05-21-0078-detail-page-frontmatter-table-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-21T20:25:00Z"
last_updated_by: Toby Clemson
---

## Plan Review: Detail-Page Frontmatter Table Implementation Plan

**Verdict:** REVISE

The plan is structurally strong — a clean three-phase TDD decomposition that
correctly extends the existing `wiki-links.ts` / `useWikiLinkResolver`
primitives, preserves the chip strip, and contains its blast radius to a
single additive component. However, multiple lenses independently flagged
the same load-bearing issues: the table silently collapses the resolver's
tri-state contract (resolved / unresolved / pending) into a binary one
that diverges visibly from the markdown body during cache warm-up; the
Phase 3 integration test spec calls a non-existent API (`fetchDoc`) and
ignores the existing `LibraryDocView.test.tsx`; and the headline AC
("links route the same as markdown body wiki-links") has no
end-to-end test. Several user-facing concerns (no visual affordance on
linkified scalars, repeated `aria-label="empty"` announcements, table
suppression for malformed frontmatter) also need attention before
implementation.

### Cross-Cutting Themes

- **Pending resolver state handling** (flagged by: architecture,
  correctness, test-coverage) — `renderScalar` only branches on
  `result.kind === 'resolved'` and falls through to plain text for
  `'pending'`, creating a visible divergence from the markdown body's
  `wiki-link-pending` styling during the cache-warming window. No test
  pins the chosen behaviour either way.
- **Object value handling is asymmetric** (flagged by: architecture,
  code-quality, correctness, usability) — objects bypass `splitByBareIds`
  via `JSON.stringify`, so embedded `WORK-ITEM-NNNN` tokens inside
  objects are inert text; arrays of objects render as `[object Object]`;
  there's no `try/catch` for circular references. The "full-fidelity
  surface" framing is undermined.
- **Bracketed-token test name vs assertion** (flagged by: code-quality,
  correctness, test-coverage) — the test titled `does not match
  bracketed tokens` actually asserts that bracketed inner tokens DO
  match. Future readers will be misled.
- **CSS source-string assertions are brittle** (flagged by:
  code-quality, test-coverage) — eight `expect(css).toMatch(...)`
  assertions duplicate the `migration.test.ts` policing and couple
  tests to formatting choices that don't affect behaviour.
- **Empty array-element / `aria-label="empty"` selector** (flagged by:
  code-quality, standards, usability) — used as a test hook on a
  non-interactive span; breaks the sentence-case aria-label
  convention; will be announced up to nine times per page by
  screen readers.

### Tradeoff Analysis

- **Architecture: hook surface growth vs cohesion** — the plan adds
  `bareIdPattern` as a third field on `useWikiLinkResolver`. This keeps
  related state co-located but starts a pattern of accumulating regex
  surfaces on a hook whose original responsibility was resolver
  composition. The architecture lens flags this as a long-term smell
  rather than a blocker; acceptable for now if the table is the only
  near-term consumer.
- **Standards: token reuse vs designer's literal** — 11.5px already
  exists as `--size-chip-md`, but the table's designer specified the
  value independently. Reusing the existing token tightens the design
  system at the cost of borrowing a chip-named token for a frontmatter
  surface; introducing a new alias (`--size-meta`) cleanly fixes that
  but adds a token. Either fix is preferable to a misleading
  EXCEPTIONS entry.

### Findings

#### Major

- 🟡 **Architecture + Correctness**: Table collapses tri-state Resolver contract into plain text for `pending`/`unresolved`
  **Location**: Phase 2 § 1 — `renderScalar`
  `renderScalar` only honours `result.kind === 'resolved'`; both
  `'pending'` and `'unresolved'` fall through to inert text. During the
  cache-warming window, the same `(prefix, id)` token will render as a
  styled pending marker in the markdown body but as inert text in the
  table immediately above it.

- 🟡 **Test Coverage**: Phase 3 integration test uses non-existent API and bypasses existing test file
  **Location**: Phase 3 § 3 — `LibraryDocView.test.tsx`
  Plan spies on `fetchModule.fetchDoc` but the real API is
  `fetchDocContent` (see `src/api/fetch.ts:73`). It also stubs global
  `fetch` while the existing `LibraryDocView.test.tsx` already mocks at
  the `fetchModule` boundary, and the plan says "new — if absent" when
  the file already exists with established conventions.

- 🟡 **Test Coverage**: Headline shared-resolver AC is not verified end-to-end
  **Location**: Phase 2 § 3 + Phase 3 § 3
  Phase 2 uses a hand-rolled `passThroughResolver`; Phase 3 only
  asserts `findByLabelText('Frontmatter')` is present. No test
  confirms that the *real* `useWikiLinkResolver` wired through
  `LibraryDocView` produces a working link with the correct `href`
  for a known work-item-by-ID parent value.

- 🟡 **Test Coverage**: Word-boundary regex collision cases are not tested
  **Location**: Phase 1 § 2 — `buildBareIdPattern` cases
  Cases like `MY-ADR-0017`, `WORK-ITEM-0042-extra`, and
  `notes/WORK-ITEM-0042.md` are realistic frontmatter values that
  the current regex will match (yielding partial/wrong links) but
  no test pins the chosen behaviour.

- 🟡 **Usability**: `aria-label="empty"` on every em-dash creates screen-reader noise
  **Location**: Phase 2 § 1 — empty-value span
  Up to nine empty rows per page (per the plan's own example) will
  each announce "empty" to a screen-reader user, drowning out the
  rows that actually have values. Also, aria-label on a non-interactive
  `<span>` is largely ignored — it's functioning as a test hook.

- 🟡 **Usability**: Linkified scalars have no visual affordance
  **Location**: Phase 2 § 2 — `.value` rule, no anchor style
  No `text-decoration`, `color`, or hover/focus treatment is specified
  for the rendered `<a>`. Inside a monospace, sunken-background table,
  `WORK-ITEM-0041` (linked) looks identical to `WORK-ITEM-0041` (plain
  text). Keyboard focus is also undefined.

- 🟡 **Usability**: Suppressing the table for `malformed` frontmatter hides the most useful debugging surface
  **Location**: Phase 3 § 1 — `frontmatterState === 'parsed'` guard
  The table is precisely the surface that would help an author
  diagnose YAML errors; suppressing it forces authors back to the raw
  file. If suppression is deliberate, the plan should call out the
  trade-off explicitly.

- 🟡 **Standards**: 11.5px duplicates the existing `--size-chip-md` token
  **Location**: Phase 2 § 4 — Migration test EXCEPTIONS entry for 11.5px
  `--size-chip-md: 11.5px` already exists in `tokens.ts` and is
  consumed by `Chip.module.css`. Adding 11.5px as an "irreducible"
  EXCEPTIONS entry while a matching token exists weakens the
  migration-test invariant.

#### Minor

- 🔵 **Architecture**: Array vs object values use different formatting strategies — arrays of objects render as `[object Object]`
- 🔵 **Architecture**: Bare-ID regex matches inside brackets, blurring documented surface split
- 🔵 **Architecture**: Two siblings sharing `grid-area: body` relies on implicit auto-row stacking
- 🔵 **Code Quality**: `BareIdSegment` uses optional fields instead of a true discriminated union (`prefix!`/`id!` assertions at call sites)
- 🔵 **Code Quality + Correctness**: Test name `does not match bracketed tokens` contradicts its assertion (`expect(matches.length).toBe(1)`)
- 🔵 **Code Quality + Test Coverage**: Eight CSS source-string assertions duplicate `migration.test.ts` policing and couple tests to formatting
- 🔵 **Code Quality + Test Coverage**: Comma-joined array test does not assert the `, ` separators are rendered
- 🔵 **Code Quality**: Array-index React keys on text/match segments are acceptable but undocumented (or could be cleaned via `<Fragment key={i}>`)
- 🔵 **Code Quality**: Integration test stubs both `global.fetch` and `fetchModule` — pick one layer
- 🔵 **Correctness**: Object values bypass linkification entirely (`JSON.stringify` not routed through `splitByBareIds`)
- 🔵 **Correctness**: Empty plain-object `{}` is treated as non-empty by `isEmpty` (asymmetric with empty-array rule)
- 🔵 **Correctness**: Missing `g` flag on the input pattern to `splitByBareIds` would cause an infinite loop — no runtime guard
- 🔵 **Correctness**: Array elements bypass `isEmpty`, so YAML `~` entries render as literal `null`/`undefined`
- 🔵 **Correctness**: `Object.entries` source-order claim breaks for integer-string YAML keys
- 🔵 **Correctness**: Text segments returned without React keys yield reconciliation warnings — use `<Fragment key={i}>`
- 🔵 **Test Coverage**: `pending` resolver branch is not tested — silent plain-text degradation has no pinning test
- 🔵 **Test Coverage**: Missing edge cases for array-of-objects, `Date` values, numeric YAML scalars
- 🔵 **Test Coverage**: Global-regex `lastIndex` reuse risk for consumers using `.exec`/`.test` directly on the returned pattern
- 🔵 **Test Coverage**: Fixture-extension hedge (`if makeIndexEntry doesn't accept frontmatterState`) is unnecessary — fixture already supports it
- 🔵 **Standards**: `aria-label="empty"` breaks sentence-case label convention and acts as a fragile test hook
- 🔵 **Standards**: `aria-label="Frontmatter"` uses developer jargon — elsewhere `LibraryDocView` uses "Document metadata"
- 🔵 **Standards**: 12px / 14px EXCEPTIONS could collapse to `padding: var(--sp-3) 14px` and drop one EXCEPTIONS entry
- 🔵 **Standards**: Two parallel surfaces (chips vs table) use different state-handling shapes — discriminated union vs external gate
- 🔵 **Usability**: Chip strip + table create redundancy without a clear conceptual distinction for users
- 🔵 **Usability**: `JSON.stringify` for object values is developer-grade output in a user-facing surface — no `try/catch` for circular refs
- 🔵 **Usability**: Mobile / narrow-viewport behaviour is unspecified — `auto 1fr` and 720px cap may compete with aside
- 🔵 **Usability**: Dark-mode coverage is asserted only manually; `--ac-fg-strong` token existence is not audited

#### Suggestions

- 🔵 **Architecture**: Hook interface grows with each new linkification consumer — consider `useWikiLinkPatterns()` split
- 🔵 **Code Quality**: Extract a small `renderArray` helper that uses `flatMap` with a separator
- 🔵 **Correctness**: Add a third integration-test case asserting `frontmatterState === 'absent'` does not render the table
- 🔵 **Usability**: Array rendering as comma-separated text loses fidelity for elements containing commas — document trade-off

### Strengths

- ✅ Clean three-phase TDD decomposition with independent shippability — each phase ships a green test suite on its own.
- ✅ Bare-ID matching is factored into `wiki-links.ts` as siblings to `buildWikiLinkPattern`, preserving the existing matching/resolution separation.
- ✅ `useWikiLinkResolver` is extended with `bareIdPattern` derived from the same `defaultProjectCode` query — no new fetches, no new query keys, shared cache.
- ✅ `splitByBareIds` defensively rebuilds the regex with `new RegExp(pattern.source, pattern.flags)` to prevent `lastIndex` cross-call bleed — a known JS regex foot-gun handled up front.
- ✅ Semantic `<dl>/<dt>/<dd>` markup is the correct landmark for key/value pairs.
- ✅ Single mount point with `grid-area: body` reuse keeps the page layout contract unchanged.
- ✅ The chip strip and `MarkdownRenderer` are explicitly out of scope — clean blast-radius containment.
- ✅ The 720px literal duplication is acknowledged as interim and tied to follow-up work item 0088.
- ✅ Source-order rendering matches the YAML file the user authored, supporting a familiar mental model.
- ✅ The `isEmpty` predicate correctly treats scalar `0` and `false` as present (not empty), with explicit tests.
- ✅ Phase 3 includes explicit dark-mode and viewport-resize manual verification steps.
- ✅ Test coverage Phase 1 covers project-prefixed vs default-pattern fallback explicitly, mirroring existing `buildWikiLinkPattern` tests.

### Recommended Changes

1. **Decide and pin the `pending`/`unresolved` resolver-result behaviour**
   (addresses: pending resolver state handling, missing `pending` test)
   In `renderScalar`, either render a `wiki-link-pending`-equivalent
   span when `result.kind === 'pending'` (matching the markdown body)
   or explicitly document the plain-text fall-through and add a test
   asserting it. Pick one and write the test.

2. **Fix the Phase 3 integration test scaffolding**
   (addresses: non-existent API, bypasses existing test file)
   Update Phase 3 § 3 to extend the existing
   `skills/visualisation/visualise/frontend/src/routes/library/LibraryDocView.test.tsx`
   rather than create a new file. Replace `fetchModule.fetchDoc` with
   the real `fetchDocContent` export, and drop the bespoke
   `vi.stubGlobal('fetch', …)` in favour of the existing
   `fetchModule` spy convention.

3. **Add a real shared-resolver end-to-end assertion**
   (addresses: headline AC unverified)
   In the Phase 3 test file, add a case that mounts `LibraryDocView`
   with two real entries — a work item whose frontmatter `parent`
   value is `'WORK-ITEM-0001'` and another work item whose
   `workItemId` is `'0001'` — and assert the table contains
   `<a href="/library/work-items/0001-…">`.

4. **Add word-boundary collision tests**
   (addresses: regex collision cases untested)
   In Phase 1 § 2, add cases for `MY-ADR-0017`,
   `WORK-ITEM-0042-extra`, `notes/WORK-ITEM-0042.md`, and
   `WORK-ITEM-0042a` — decide the chosen behaviour and pin it.

5. **Rework empty-cell accessibility**
   (addresses: aria-label noise, aria-label naming standards)
   Replace `<span aria-label="empty">—</span>` with
   `<span aria-hidden="true">—</span>` plus a visually-hidden
   "no value" phrase if announcement is desired. Use `data-empty`
   or `.empty` class as the test selector. Also change the
   container `aria-label="Frontmatter"` to `"Document metadata"`
   to match the malformed banner wording.

6. **Style the linkified anchors**
   (addresses: no visual affordance, dark-mode coverage)
   Add an `.value a { … }` rule to `FrontmatterTable.module.css`
   with explicit `color: var(--ac-link)` (or design-system
   equivalent), underline / hover / focus states, and a visible
   focus ring against the sunken background in both light and
   dark modes.

7. **Reuse `--size-chip-md` (or introduce `--size-meta`) and drop the 11.5px EXCEPTIONS entry**
   (addresses: standards duplication)
   In `FrontmatterTable.module.css` use `var(--size-chip-md)` (or
   add a new `--size-meta: 11.5px` token alias) so the migration
   test passes without an EXCEPTIONS entry for 11.5px. Consider
   collapsing 12px to `var(--sp-3)` and keeping only 14px as
   irreducible.

8. **Resolve the object-value rendering asymmetry**
   (addresses: object values bypass linkification; `JSON.stringify` UX; empty `{}` not dashed)
   Either (a) route `JSON.stringify(value)` through `renderScalar`
   so embedded tokens linkify, plus pretty-print inside a `<pre>` and
   wrap in `try/catch` for circular references; or (b) document
   in "What We're NOT Doing" that objects are an escape-hatch
   surface only. Either way, extend `isEmpty` so `{}` is dashed
   symmetrically with `[]`.

9. **Decide malformed-frontmatter UX explicitly**
   (addresses: suppressing the table hides debugging surface)
   Either render an explicit "no parseable keys" empty-state row
   in the malformed case, or move the rationale into the plan's
   "What We're NOT Doing" section with a one-line justification.

10. **Convert `BareIdSegment` to a true discriminated union**
    (addresses: optional fields + `!` assertions)
    Define `BareIdSegment = { kind: 'text'; text: string } |
    { kind: 'match'; text: string; prefix: 'ADR' | 'WORK-ITEM';
    id: string }` so `seg.kind === 'match'` narrows without
    requiring non-null assertions at the call site.

11. **Tighten test names and assertions**
    (addresses: test name contradiction; missing separator assertion)
    Rename the `does not match bracketed tokens` test to reflect
    the actual assertion. In the comma-joined array test, add
    `expect(container.textContent).toContain('design, frontend, detail-page')`.

12. **Add `g`-flag defence to `splitByBareIds`**
    (addresses: missing `g` flag would infinite-loop)
    Either assert at function entry or force the `g` flag in the
    cloned regex: `new RegExp(pattern.source, pattern.flags.includes('g')
    ? pattern.flags : pattern.flags + 'g')`.

13. **Add narrow-viewport behaviour to the CSS module**
    (addresses: mobile / narrow-viewport unspecified)
    Cap the key column with `grid-template-columns: minmax(auto, 12rem) 1fr`,
    swap `overflow-wrap: anywhere` for `break-word`, and add a
    manual verification step at 320–480px widths.

14. **Trim brittle CSS source-string assertions**
    (addresses: brittle tests)
    Keep token-reference assertions (they pin the dark-mode
    contract); drop the literal-value source-string matches
    (`11.5px`, `12px 14px`, `auto 1fr`) — these are policed by
    `migration.test.ts` or are behaviour-agnostic formatting choices.

15. **Drop the `makeIndexEntry` hedge**
    (addresses: fixture already supports `frontmatterState`)
    Remove the conditional "If `makeIndexEntry` doesn't accept
    `frontmatterState`..." from Phase 3 § 3 — the fixture already
    supports the override via `Partial<IndexEntry>`.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan respects the existing resolution/matching split of
the wiki-link system by extending `wiki-links.ts` and
`useWikiLinkResolver` with parallel bare-ID primitives, reusing the
resolver and `defaultProjectCode` config surface. Component boundaries
are clean: `FrontmatterTable` is additive, gated by `frontmatterState
=== 'parsed'`, and does not entangle with `FrontmatterChips` or
`MarkdownRenderer`. The main architectural concern is that the table
silently collapses the resolver's tri-state contract
(resolved/unresolved/pending) into a binary one, producing UX that
diverges from the markdown body for the same `(prefix, id)` during
cache warm-up.

**Strengths**:
- Bare-ID matching factored into `wiki-links.ts` as siblings to
  `buildWikiLinkPattern`.
- `useWikiLinkResolver` extended with `bareIdPattern` from the same
  `defaultProjectCode` query — no new fetches.
- Single mount point with `grid-area: body` reuse — page layout
  contract unchanged.
- Each phase independently shippable with green tests.
- `splitByBareIds` rebuilds the RegExp per call to avoid `lastIndex`
  bleed.
- Explicit deferral of the `720px` literal to 0088, with duplication
  acknowledged.

**Findings**:
- 🟡 (major): Table collapses tri-state Resolver contract into binary,
  diverging from markdown body during cache warm-up.
- 🔵 (minor): Array vs object value rendering use different strategies
  — arrays of objects render `[object Object]`.
- 🔵 (minor): Bare-ID regex matches inside brackets, blurring the
  documented surface split.
- 🔵 (suggestion): Hook surface grows with each new linkification
  consumer.
- 🔵 (suggestion): Two siblings sharing `grid-area: body` relies on
  implicit auto-row stacking.

### Code Quality

**Summary**: Plan is well-structured, decomposes cleanly into three
TDD phases, and reuses existing primitives. A few small design choices
around the `BareIdSegment` discriminator, `JSON.stringify` for object
cells, and tight CSS source-string assertions could be tightened.

**Strengths**:
- Clear three-phase decomposition with explicit independence
  guarantees.
- Pure helpers isolated in the wiki-links module.
- Defensive RegExp clone inside `splitByBareIds`.
- Narrower prop surface than `FrontmatterChips` (no discriminator).
- Honest acknowledgement of duplicated `720px` literal as interim.
- Tests cover the `false` / `0` falsy-but-not-empty case explicitly.

**Findings**:
- 🔵 (minor): `BareIdSegment` uses optional fields instead of a true
  discriminated union.
- 🔵 (minor): Object frontmatter values bypass linkification via
  `JSON.stringify`.
- 🔵 (minor): Array-comma test does not assert the comma separators
  are rendered.
- 🔵 (minor): Eight CSS source-string assertions duplicate design
  spec and couple tests to formatting.
- 🔵 (minor): Array-index React keys on text/match segments are
  acceptable but undocumented.
- 🔵 (minor, low confidence): Test name `does not match bracketed
  tokens` contradicts its assertion.
- 🔵 (suggestion): Inline `, ` separator weaves a presentation concern
  into rendering.
- 🔵 (minor): Integration test stubs both global `fetch` and the
  `fetchDocs` spy.

### Test Coverage

**Summary**: Strong TDD intent and broad coverage of pattern grammar,
segmentation, rendering branches, and styling, but the Phase 3
integration test references a non-existent API (`fetchDoc`) and
parallels rather than extends the existing `LibraryDocView.test.tsx`;
the headline shared-resolver AC has no end-to-end test; CSS source
assertions over-test implementation details; regex word-boundary
edge cases for prefix collisions are missing; and the `pending`
resolver branch is unverified.

**Strengths**:
- TDD ordering explicit per phase.
- Pattern/segmentation/rendering tests cover the core happy paths plus
  null/undefined/empty-string/empty-array, 0/false-not-empty, and the
  unresolved-resolver case.
- Phase 1 covers project-prefixed vs default-pattern fallback explicitly.
- Phase 2 includes aria-label and empty-object null-return assertions.
- Migration test EXCEPTIONS are anticipated upfront.

**Findings**:
- 🟡 (major): Phase 3 integration test uses non-existent `fetchDoc`
  API and bypasses existing test file conventions.
- 🟡 (major): Headline shared-resolver AC ("auto-linkify routes the
  same as markdown body") is not verified end-to-end.
- 🟡 (major): Word-boundary regex collision cases (`MY-ADR-0017`,
  suffix collisions, path strings) are not tested.
- 🟡 (major): `pending` resolver branch is not tested — silent
  plain-text degradation has no pinning test.
- 🔵 (minor): CSS source-text assertions are brittle and couple tests
  to whitespace/literal choices.
- 🔵 (minor): Test name `does not match bracketed tokens` contradicts
  its assertion.
- 🔵 (minor): Missing edge cases for array-of-objects, `Date` values,
  multiline strings, numeric YAML scalars.
- 🔵 (minor): Global-regex `lastIndex` re-use risk for consumers using
  `.exec`/`.test` directly.
- 🔵 (minor): Fixture-extension hedge is unnecessary —
  `makeIndexEntry` already accepts `frontmatterState`.
- 🔵 (suggestion): Comma-joined array test does not assert separator
  placement.

### Correctness

**Summary**: Plan is logically sound in its core regex and segmentation
design. Several specific correctness concerns remain around
pending-state handling, empty-collection asymmetry, missing `g`-flag
defence, and the asymmetry between the empty-value rule and how
object/array contents are rendered.

**Strengths**:
- Correctly identifies that `WORK-ITEM` (hyphenated) is canonical.
- Bare-ID regex grammar produces the right `id` shapes (`'0042'` vs
  `'PROJ-0042'`) for the resolver.
- `splitByBareIds` defensively clones the regex.
- Empty-object guard prevents stray empty `<dl>`.
- `isEmpty` correctly treats `0` and `false` as present.
- Word-boundary anchoring correctly excludes `WORKBOOK-0042`-style
  false positives.

**Findings**:
- 🟡 (major): `pending` resolver result is silently rendered as plain
  text — divergence from markdown body during cache warm-up.
- 🔵 (minor): Object values bypass linkification entirely.
- 🔵 (minor): Empty plain-object value `{}` is treated as non-empty —
  asymmetric with empty-array rule.
- 🔵 (minor): Missing `g` flag on the input pattern would cause an
  infinite loop in `splitByBareIds`.
- 🔵 (minor): Array elements bypass `isEmpty`, so null/undefined
  entries become literal text.
- 🔵 (minor): `Object.entries` source-order claim breaks for
  integer-string YAML keys.
- 🔵 (minor): Test name contradicts its assertion (`does not match
  bracketed tokens`).
- 🔵 (minor): Text segments returned without React keys yield
  reconciliation warnings.
- 🔵 (minor, low confidence): Phase 3 guard does not cover `absent`
  state explicitly in tests.

### Standards

**Summary**: Plan generally follows established frontend conventions
(folder layout, prop names, CSS tokens, migration-test EXCEPTIONS
form). Several specific choices break with existing patterns: an
existing `--size-chip-md` token already encodes the new module's
font-size value; aria-label conventions diverge from sentence-case
user-facing labels; and the `aria-label="empty"` selector pattern is
used as a test hook on a non-interactive span.

**Strengths**:
- Folder layout mirrors `FrontmatterChips/` sibling exactly.
- Prop name `resolveWikiLink` and `Resolver` import follow the
  established convention.
- Migration-test EXCEPTIONS entries follow the documented per-occurrence
  model.
- Semantic `<dl>/<dt>/<dd>` markup.
- CSS token references map to declared tokens; corrects the story's
  mistaken `--ac-text-muted` to `--ac-fg-muted`.
- TDD-first ordering and `?raw` CSS source-assertion convention match
  existing precedent.

**Findings**:
- 🟡 (major): 11.5px duplicates the existing `--size-chip-md` token
  value.
- 🔵 (minor): `aria-label="empty"` breaks sentence-case label convention
  and acts as a test hook on a non-interactive span.
- 🔵 (minor): `aria-label="Frontmatter"` uses developer jargon —
  elsewhere `LibraryDocView` uses "Document metadata".
- 🔵 (minor): 12px / 14px rationales could collapse to
  `padding: var(--sp-3) 14px` and drop one EXCEPTIONS entry.
- 🔵 (minor): Two parallel surfaces (chips vs table) handle
  frontmatter state with different prop shapes.

### Usability

**Summary**: Plan delivers a focused additive UI surface and uses
semantic HTML with an accessible label, plus reuses the existing
wiki-link resolver for routing consistency. Several user-facing
concerns warrant attention: nine `aria-label="empty"` em-dashes per
page will create screen-reader noise; linkified scalars have no
visual affordance to distinguish them from plain text; suppressing
the table for malformed frontmatter removes a debugging signal; and
the chip-strip-plus-table combination risks redundancy without a
clear conceptual distinction.

**Strengths**:
- Semantic `<dl>/<dt>/<dd>` markup correctly labelled.
- Chip strip preserved — no regression for users who rely on it.
- Reuses shared wiki-link resolver — links route identically to body.
- Source-order rendering matches the user's authored file.
- Phase 3 includes dark-mode and viewport-resize manual verification.

**Findings**:
- 🟡 (major): `aria-label="empty"` on every em-dash will create
  screen-reader noise (up to nine repetitions per page).
- 🟡 (major): Linkified scalars have no visual affordance — same
  appearance as plain text on monospace sunken background.
- 🟡 (major): Suppressing the table for malformed frontmatter hides
  the most useful debugging surface.
- 🔵 (minor): Chip strip + table create redundancy without clear
  user model.
- 🔵 (minor): `JSON.stringify` for object values is developer-grade
  output; no `try/catch` for circular references.
- 🔵 (minor): Mobile/narrow viewport behaviour is unspecified.
- 🔵 (minor): Dark-mode coverage asserted manually only;
  `--ac-fg-strong` token existence not audited.
- 🔵 (suggestion, low confidence): Array rendering as comma-separated
  text loses fidelity for elements containing commas.

## Re-Review (Pass 2) — 2026-05-21T20:25:00Z

**Verdict:** APPROVE

The plan has been comprehensively revised. Every major finding from
pass 1 is materially addressed in the plan text — the pending-state
divergence is fixed by a `.pending` span mirroring the markdown body's
treatment; the Phase 3 integration test now uses `fetchDocContent` and
extends the existing test file with the right scaffolding; the
end-to-end shared-resolver assertion is in place; word-boundary
collision cases are pinned; the `aria-label="empty"` noise is replaced
with `aria-hidden="true"` + `data-empty`; anchor styling provides a
visible link colour and focus ring; the malformed-state suppression
trade-off is documented explicitly; and 11.5px is tokenised via
`var(--size-chip-md)`, dropping a redundant EXCEPTIONS entry. Minor
items resolved include the `BareIdSegment` discriminated union, the
explicit `g`-flag forcing in `splitByBareIds`, the renamed
bracketed-tokens test, the `Fragment`-with-key text segments, the
trimmed CSS source-string assertions, the empty-object `isEmpty`
branch, and the third `absent`-state integration test.

The remaining items are minor or accepted trade-offs and do not block
implementation.

**Note on agent fidelity**: Four of the six re-review agents
(architecture, test-coverage, correctness, standards) appear to have
read stale plan content — their reports describe issues that the
current file demonstrably does not contain (e.g. they report
`fetchDoc` is still referenced when the file uses `fetchDocContent`;
they report the pending span is absent when the file's `renderScalar`
explicitly handles `result.kind === 'pending'`). The aggregate
assessment below uses the file's actual current state as the ground
truth and treats the stale-read findings as resolved where the plan
demonstrably addresses them. The code-quality and usability agents
reviewed the current state accurately and their findings are carried
forward.

### Previously Identified Issues

- 🟡 **Architecture + Correctness**: Tri-state Resolver collapsed into binary — **Resolved** (`renderScalar` now branches on `'pending'` and renders a `.pending` span; a test pins the behaviour)
- 🟡 **Test Coverage**: Phase 3 uses non-existent `fetchDoc` and bypasses existing test file — **Resolved** (now uses `fetchDocContent`; extends the existing `LibraryDocView.test.tsx` with its `Wrapper`/`mockEntry` conventions)
- 🟡 **Test Coverage**: Shared-resolver AC unverified end-to-end — **Resolved** (fourth integration test seeds two entries and asserts the resolver-built `href`)
- 🟡 **Test Coverage**: Word-boundary collision cases untested — **Resolved** (four collision-case tests added: `MY-ADR-0017`, suffix collision, path-shaped, trailing word-char rejection)
- 🟡 **Test Coverage**: Resolver `pending` branch untested — **Resolved** (dedicated `renders a pending span` test added)
- 🟡 **Usability**: `aria-label="empty"` noise — **Resolved** (replaced with `aria-hidden="true"` + `data-empty` selector)
- 🟡 **Usability**: Linkified scalars have no visual affordance — **Resolved** (`.value a` styled with `--ac-link`, hover, `:focus-visible` outline)
- 🟡 **Usability**: Malformed-state suppression hides debugging surface — **Resolved as accepted trade-off** (explicit "What We're NOT Doing" entry with rationale)
- 🟡 **Standards**: 11.5px duplicates `--size-chip-md` — **Resolved** (uses `var(--size-chip-md)`; EXCEPTIONS entry dropped)
- 🔵 **Code Quality / Correctness**: `BareIdSegment` optional fields — **Resolved** (true discriminated union)
- 🔵 **Code Quality / Test Coverage**: Brittle CSS source-string assertions — **Resolved** (trimmed to token-reference assertions only; literal-value assertions removed with explicit rationale comment)
- 🔵 **Code Quality / Correctness / Test Coverage**: Test name contradicts assertion — **Resolved** (renamed to `captures the inner token even when wrapped in brackets`)
- 🔵 **Code Quality / Correctness**: Object values bypass linkification — **Resolved** (routed through `renderScalar` via `safeStringify`)
- 🔵 **Code Quality / Test Coverage**: Comma-joined array separator not asserted — **Resolved** (`dd?.textContent` pins the full joined string)
- 🔵 **Code Quality**: Array-index keys undocumented — **Resolved** (`Fragment` keys with inline rationale comment)
- 🔵 **Code Quality**: Integration test double-mocking — **Resolved** (now only `vi.spyOn(fetchModule, …)`)
- 🔵 **Correctness**: Empty `{}` non-empty — **Resolved** (`isEmpty` now handles plain empty objects)
- 🔵 **Correctness**: Missing `g`-flag defence in `splitByBareIds` — **Resolved** (cloned regex forces the `g` flag)
- 🔵 **Correctness**: Text segments without React keys — **Resolved** (`Fragment` with stable index key)
- 🔵 **Correctness / Test Coverage**: `absent` state not tested — **Resolved** (third integration test case added)
- 🔵 **Test Coverage**: Fixture-extension hedge — **Resolved** (hedge removed; plan asserts `mockEntry` already supports overrides)
- 🔵 **Standards**: `aria-label` jargon — **Resolved** (now `"Document metadata"`, matching the malformed-banner wording)
- 🔵 **Standards**: 12px / 14px padding entries — **Resolved** (consolidated to `padding: var(--sp-3) 14px`; 12px EXCEPTIONS dropped)
- 🔵 **Standards**: Chip / table asymmetry undocumented — **Resolved** (explicit "What We're NOT Doing" entry with cross-reference to 0084)
- 🔵 **Usability**: Mobile / narrow-viewport behaviour unspecified — **Resolved** (`minmax(auto, 12rem)` + `overflow-wrap: break-word`; ~360px manual check added)
- 🔵 **Usability**: Dark-mode token audit absent — **Resolved as pre-flight check** (plan calls out token existence verification for `--ac-link`, `--ac-link-hover`, `--ac-focus`, `--radius-xs` before CSS write)
- 🔵 **Architecture**: `grid-area: body` stacking implicit — **Resolved** (CSS comment now documents the auto-row contract and the migration path if a future row is added)
- 🔵 **Architecture (suggestion)**: Hook surface growth — **Still present, accepted** (third field on `useWikiLinkResolver`; revisit if more consumers materialise)
- 🔵 **Architecture (suggestion)**: Array-of-objects rendering — **Partially resolved** (array elements that are objects now route through `safeStringify`, then `renderScalar`, so embedded tokens linkify; the resulting JSON form is still developer-grade output)
- 🔵 **Architecture (minor)**: Bare-ID regex matches inside brackets — **Resolved as documented contract** (renamed test makes the consumer-scoping explicit)
- 🔵 **Correctness**: Integer-string YAML keys break source-order — **Resolved as documented caveat** (Key Discoveries notes the constraint; YAML-authored frontmatter does not produce such keys in practice)
- 🔵 **Usability**: Chip + table redundancy — **Acknowledged, deferred to 0084** (explicit "What We're NOT Doing" entry)

### New Issues Introduced

- 🔵 **Code Quality / Standards / Usability (minor)**: The CSS module references `--ac-link`, `--ac-link-hover`, `--ac-focus`, and `--radius-xs`, with a pre-flight check delegated to the implementer at coding time rather than verified now. The CSS source assertion in the test file pins the `--ac-link` name, so if the substitution branch is taken both files must be edited together. A quick `grep` over `tokens.ts` / `global.css` at plan time would pin this decision.
- 🔵 **Code Quality (suggestion)**: `safeStringify` uses a bare `catch {}` that silently swallows serialisation errors. YAML-sourced frontmatter cannot produce cycles in practice, but the silent fallback would produce a confusing `[object Object]`-style render if ever exercised. A single `console.warn` inside the catch would make the fallback observable in development.
- 🔵 **Correctness (minor)**: Array elements that are nullish (e.g. YAML `tags: [foo, ~, bar]` → `['foo', null, 'bar']`) still bypass `isEmpty` and render as the literal text `null` / `undefined` via `String(el)`. Low-frequency edge case in well-authored YAML, but if it appears the rendering is misleading. Either route array elements through `isEmpty` before `String(el)`, or document the trade-off in the existing Key Discoveries section.
- 🔵 **Test Coverage (suggestion)**: A handful of value-shape edge cases are still untested — `Date` instances (often emitted by YAML parsers for date fields), multiline string scalars (line continuation in YAML), large numeric scalars, and `bareIdPattern.test()` called twice in a row to guard against `lastIndex` carry on the exposed pattern. None are blockers; each would add a small assertion.
- 🔵 **Usability (minor)**: Pending-state styling is `font-style: italic` + muted colour, mirroring the markdown body. Inside a flat monospace cell the italic is unambiguous, but a `title="Resolving link…"` attribute (or note in the design rationale) would help future readers understand why italic appears in scalar text.

### Assessment

The plan is ready for implementation. Verdict shifts from **REVISE** to
**APPROVE**: every major finding is resolved, the remaining minor /
suggestion-level items are either deliberate trade-offs (chip/table
redundancy deferred to 0084, integer-string key constraint
documented), small follow-ups discoverable during implementation
(nullish array elements, `safeStringify` warn, Date/multiline tests),
or planning-quality improvements (pinning the link-token decision at
plan time). None require another revision pass.

Recommended next step: spend ~2 minutes greping `tokens.ts` for
`--ac-link`, `--ac-link-hover`, `--ac-focus`, and `--radius-xs` and
pin the decision in the plan, then proceed to `/implement-plan`. The
other minor items are best handled inline during TDD rather than
re-planned.

## Verification Pass (Pass 2-confirm) — 2026-05-21T20:35:00Z

A second re-run of all six lenses was requested with sharper prompts
demanding line-quoted evidence from the current file. The result
confirmed the same agent-fidelity issue as before: three lenses
(architecture, correctness, usability) again read stale content and
cited line numbers that do not match the current 1359-line file —
e.g. architecture cited `renderScalar` at "lines 443-461" claiming
no `pending` branch, but the current file has `result.kind ===
'pending'` at line 577 with a dedicated `.pending` span. Correctness
cited `splitByBareIds` at "lines 215-242" claiming no `g`-flag
defence, but the current implementation at the actual location
includes `const flags = pattern.flags.includes('g') ? pattern.flags
: pattern.flags + 'g'`. Usability cited `aria-label="empty"` at
"line 497", but line 634 of the current file has `<span … aria-hidden=
"true">—</span>`.

These claims were each disproved via direct `grep` against the file:

| Check | Expected | Actual |
|-------|----------|--------|
| `result.kind === 'pending'` | present | line 577 ✓ |
| `aria-hidden="true"` | present (component + test) | lines 634, 804 ✓ |
| `safeStringify` | present | lines 601, 614 ✓ |
| `minmax(auto, 12rem)` | present | line 671 ✓ |
| `var(--ac-link)` | present | line 693 ✓ |
| `Document metadata` | present | 10 occurrences ✓ |
| `fetchDocContent` | present | lines 1108, 1128, 1148, 1165, 1195 ✓ |

The three accurate-read lenses (code-quality, test-coverage,
standards) returned the same minor / suggestion-level findings as
the first re-review:

- **Code quality** (suggestion, low confidence): inline `, ` separator
  in array branch; `safeStringify` bare `catch {}` silently swallows
  errors; pre-flight token-existence check defers a decision into
  implementation.
- **Test coverage** (minor, medium confidence): a few value-shape
  edge cases still untested (array-of-objects, `Date` instances,
  multiline strings); no test pins `bareIdPattern` against
  `lastIndex` re-use for direct `.exec`/`.test` consumers.
- **Standards**: zero findings; verdict explicitly **clean** — all
  five prior standards findings resolved (token reuse of
  `--size-chip-md`, `aria-hidden` empty pattern, `Document metadata`
  label, `padding: var(--sp-3) 14px` consolidation, "What We're NOT
  Doing" entry for the chip/table asymmetry).

### Verdict (unchanged): APPROVE

The plan is ready for implementation. The remaining items are all
minor or suggestion-level and best handled inline during TDD. The
recommended pre-flight grep of `tokens.ts` for `--ac-link`,
`--ac-link-hover`, `--ac-focus`, `--radius-xs` remains the only
plan-time follow-up worth doing before `/implement-plan`.
