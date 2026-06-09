---
date: "2026-05-21T21:42:00+00:00"
type: work-item-review
producer: review-work-item
target: "work-item:0076"
work_item_id: "0076"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 2
status: complete
id: "0076-code-block-syntax-highlight-palette-review-1"
title: "0076-code-block-syntax-highlight-palette-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-21T21:42:00+00:00"
last_updated_by: Toby Clemson
---

## Work Item Review: 0076 — Code-Block Syntax-Highlight Tokens and Renderer Adoption

**Verdict:** REVISE

The work item is structurally complete, well-motivated, and unusually concrete for a token-introduction story: every requirement maps to a specific file, the palette is enumerated by name and value, and acceptance criteria reach for computed-style assertions rather than visual inspection. The story is right-sized around a single coherent outcome (one shared `--tk-*` mapping consumed by both the markdown renderer and templates preview) and Drafting Notes pre-empt the obvious "why" questions. What pushes the verdict to REVISE is a small but load-bearing cluster of gaps where multiple lenses converge: the templates-preview coupling with 0042/0089 is recorded as "Related" but is actually an ordering dependency on the same selectors; the set of hljs classes that MUST be mapped is implementer-defined rather than enumerated, so AC2 cannot produce a definitive pass/fail; and the "no regression" clause of AC4 is unbounded.

### Cross-Cutting Themes

- **Templates-preview migration coupling is underspecified** (flagged by: dependency, clarity, testability, scope) — the work touches `LibraryTemplatesView.module.css:153-213`, which is the same surface as 0042 and 0089; the relationship is listed as "Related" but is an ordering constraint, the AC's selector pattern (`.previewBody :global(.hljs-*)`) is informal, and AC5 references "the preview's computed-style assertions" without naming the test that must exist.
- **Selector coordination with 0075 left unsequenced** (flagged by: dependency, scope) — 0075 migrates `<pre>` radius and other code-block CSS; both stories editing the same selectors in flight is a sequencing problem, not a loose relation.
- **Prototype-constants source of truth not pinned** (flagged by: dependency, testability) — AC1 asserts byte parity against `prototype-standalone.html`, but it is not stated whether the test reads the prototype HTML directly, snapshots a fixture, or hard-codes the values — each weakens the parity guarantee differently.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Dependency**: Templates-preview migration coupling with 0042 / 0089 not captured as ordering
  **Location**: Dependencies
  Both 0042 and 0089 touch the same `LibraryTemplatesView.module.css` selectors this story rewrites. Listing them as "Related" understates the coupling — if they land independently against the old local mapping they conflict with or get silently overwritten by this story.

- 🟡 **Testability**: "Without regression" is unbounded; conditional test-addition clause is discretionary
  **Location**: Acceptance Criteria (AC4)
  AC4 says existing markdown tests "continue to pass without regression" and asks the implementer to add tests "where existing coverage is insufficient". Without an enumerated list of behaviours that must have at least one assertion after this lands, a reviewer cannot tell whether the second clause was honoured — a green run only proves already-written tests pass.

- 🟡 **Testability**: Set of hljs classes asserted in AC2 is not pinned down
  **Location**: Acceptance Criteria (AC2) and Assumptions
  AC2 says "one span per mapped hljs class", but the Assumptions section permits `--tk-*` tokens to ship without an active mapping. The set of "mapped hljs classes" is therefore implementer-defined; a sparse mapping covering only `hljs-string` and `hljs-comment` could legitimately claim to pass AC2.

#### Minor

- 🔵 **Clarity**: Selector pattern `.previewBody :global(.hljs-*)` is ambiguous about which rules are removed
  **Location**: Acceptance Criteria (5th bullet)
  The `.hljs-*` glob is an informal pattern, not a CSS selector. Reader cannot tell whether this means every hljs-prefixed rule, only those scoped under `.previewBody`, or some other subset — Context cites lines 153-213 but the AC does not.

- 🔵 **Clarity**: "Matches the new palette" is an underspecified outcome relative to earlier criteria
  **Location**: Acceptance Criteria (5th bullet)
  AC5's phrasing drops the precision of AC2/AC3 ("`getComputedStyle(span).color` resolves to the rgb equivalent…"), leaving the templates-preview assertion open to interpretation about what "match" means and which spans must be checked.

- 🔵 **Clarity**: "Ships as a defined token without an active mapping" is slightly ambiguous
  **Location**: Assumptions
  Not stated whether the shared mapping file must contain inert selectors for unmapped tokens or omit them entirely; either pattern is legitimate, leading to inconsistency.

- 🔵 **Dependency**: 0075 listed as Related but implies same-file ordering on code-block CSS
  **Location**: Dependencies
  Coordinating on the same selectors is an ordering constraint, not a loose relation; whichever lands second must rebase on the other's selector changes.

- 🔵 **Dependency**: `global.test.ts` cross-surface assertion has an implicit fixture dependency
  **Location**: Acceptance Criteria
  AC1 requires the test to read prototype constants from `meta/research/design-inventories/.../prototype-standalone.html`. The dependency on that external file's stable path and contents from the visualiser test environment is not captured in Dependencies or Technical Notes.

- 🔵 **Dependency**: DevDesignSystem (0083) showcase consumption left as Related, not Blocks
  **Location**: Dependencies
  0083 is a downstream consumer waiting on this story's output — closing this story does not surface 0083 as unblocked without moving it to Blocks.

- 🔵 **Scope**: Templates-preview migration could plausibly be a sibling story
  **Location**: Requirements / Acceptance Criteria
  Bundling defensible (single source of truth), but the templates-preview migration could be a thin follow-on. No change required if the team prefers atomic landing — flagged only for explicit decision.

- 🔵 **Testability**: Parity against "prototype constants" assumes a defined extraction procedure
  **Location**: Acceptance Criteria (AC1)
  If the test hard-codes prototype values in its assertions, AC1 degrades to "test values match implementation values"; prototype drift would not be detected.

- 🔵 **Testability**: "rgb equivalent" of hex tokens leaves colour-space conversion implicit
  **Location**: Acceptance Criteria (AC2 and AC3)
  Prototype values include both hex and rgba; the criterion does not specify the conversion function or how alpha-bearing tokens map, so two reasonable test authors could disagree on the expected string.

- 🔵 **Testability**: AC5 conflates structural removal with behavioural assertion without naming the test
  **Location**: Acceptance Criteria (AC5)
  References "the preview's computed-style assertions" as if a known test file exists, but does not name it or specify which classes the preview test must assert.

#### Suggestions

- 🔵 **Scope**: Coordination overlap with 0075 on `<pre>` styling
  **Location**: Dependencies
  Pick an explicit ordering with 0075 (typography size-scale consumption) and record it in Dependencies rather than leaving "coordinate on the same selectors" as the only signal.

### Strengths

- ✅ Frontmatter complete and well-formed; type/status/priority/author/date/tags all valid.
- ✅ Summary opens with explicit "As a reader… I want… so that…" user-story framing; user and motivation named.
- ✅ Context section explains forces behind the work — prototype palette source, current renderer state, and rationale for keeping the renderer.
- ✅ Acceptance Criteria are unusually concrete for a token story: file paths, hljs class targets, computed-style assertions in both themes, explicit token-name lists.
- ✅ Drafting Notes pre-empt likely reader questions (why theme-independent, why expanded token set, why one shared layer, why diff tokens included, why keep `react-markdown`).
- ✅ Token names enumerated explicitly with prototype values (e.g. `--code-bg #0E1320`); no ambiguity about scope.
- ✅ Renderer scope deliberately constrained: picks up only the palette, keeps existing pipeline — avoids accidental renderer-replacement effort.
- ✅ Out-of-scope items explicitly called out (per-template variable wrapper in `template-highlight.tsx`).
- ✅ Open Questions section explicitly resolved with a pointer to Drafting Notes rather than left dangling.
- ✅ External libraries named (`react-markdown`, `remark-gfm`, `remarkWikiLinks`, `rehype-highlight`, hljs) with explicit assumption that hljs continues to emit standard class names.
- ✅ Dependencies section densely populated with blocked-by, related, and blocks edges (0033, 0042, 0089, 0083, 0075, 0088).
- ✅ Technical Notes explicitly closes the gap with 0033's AC1 by noting `--code-*` / `--tk-*` were not in that scope.

### Recommended Changes

1. **Pin the templates-preview ordering relative to 0042 and 0089** (addresses: dependency-major, clarity-minor-on-AC5-selector, testability-minor-on-AC5)
   In Dependencies, move 0042 and 0089 out of "Related" into the appropriate `Blocked by` / `Blocks` edge (most likely: 0076 Blocks 0042; 0089 either Blocks 0076 or lands alongside with a stated choice). State which story owns the templates-preview CSS rewrite. Reword AC5 to reference the explicit line range from Context (`LibraryTemplatesView.module.css:153-213`) rather than the informal `.hljs-*` glob, and name the test file the preview's computed-style assertions live in (or add it as a sub-criterion).

2. **Enumerate the required hljs class → `--tk-*` mappings as a table** (addresses: testability-major-on-AC2, clarity-minor-on-Assumptions)
   In Requirements or AC2, add a table listing the exact hljs class names that MUST be mapped (e.g. `hljs-comment → --tk-com`, `hljs-string → --tk-str`, …). Reframe the Assumptions clause as "tokens listed in the table below are required mappings; any other `--tk-*` token may ship unmapped." This removes implementer discretion over what AC2 actually asserts and resolves whether the shared layer should contain inert placeholders.

3. **Convert AC4 from "no regression" into an enumerated coverage list** (addresses: testability-major-on-AC4)
   Replace "continue to pass without regression" + "where existing coverage is insufficient" with a list of behaviours that must have at least one assertion after this story lands — e.g. GFM task list inside a fenced block, `[[WORK-ITEM-NNNN]]` adjacent to a code block, auto-detected python and typescript fenced blocks each producing coloured `hljs-keyword` spans. AC4 then reduces to "these N tests exist and pass."

4. **Pin the prototype-constants source of truth for AC1** (addresses: testability-minor-on-AC1, dependency-minor-on-fixture)
   State explicitly whether `global.test.ts` reads `prototype-standalone.html` directly (and pins a copy or content hash so prototype drift surfaces as a test failure) or asserts against a JSON fixture committed alongside the test. Either way, name the artefact in Technical Notes. If the test reaches across the repo to the research directory, capture that as a dependency in Dependencies or Technical Notes.

5. **Sequence with 0075 explicitly** (addresses: dependency-minor-on-0075, scope-suggestion)
   Decide ordering with 0075 (typography size-scale consumption) and reflect it in Dependencies — either `Blocked by: 0075` (this story rebases onto 0075's size-scale migration) or `Blocks: 0075`, with a one-line note on which selectors transfer.

6. **Specify the `getComputedStyle.color` comparison rule** (addresses: testability-minor-on-AC2/AC3-rgb)
   Add a Technical Note (or sub-clause in AC2) stating the canonical form for the assertion — e.g. "the test converts hex tokens to `rgb(r, g, b)` via a shared helper and compares strings exactly; rgba tokens compare via `rgba(r, g, b, a)` with alpha to two decimal places" — so colour-space conversion is uniform across tokens with and without alpha.

7. **Move 0083 from Related to Blocks** (addresses: dependency-minor-on-0083)
   0083 (DevDesignSystem reference page) is a downstream consumer of the primitives shipped here; surfacing it under `Blocks` makes its unblocking visible when this story closes.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item communicates its intent clearly: a self-contained `--code-*`/`--tk-*` palette is introduced, the renderer choice is preserved, and the shared mapping layer is described consistently across Summary, Context, Requirements, and Acceptance Criteria. Referents are unambiguous, actors are named (renderer, test, CSS layer), and outcomes are stated as observable computed-style results. A small number of minor ambiguities remain — chiefly around 'the templates preview' being referred to via a partial selector pattern and the phrase 'matches the new palette' in the last criterion.

**Strengths**:
- Summary, Context, Requirements, and AC all describe the same scope.
- Token names enumerated explicitly with prototype values.
- Actor and outcome concrete in AC — tests inspect specific hljs spans and assert `getComputedStyle(span).color`.
- Drafting Notes pre-empt likely reader questions.
- Renderer-pipeline jargon is standard within the project's frontend domain and consistently used.

**Findings**:
- 🔵 minor / medium — Selector pattern `.previewBody :global(.hljs-*)` is ambiguous about which rules are removed (Acceptance Criteria 5th bullet).
- 🔵 minor / medium — "Matches the new palette" is an underspecified outcome relative to earlier criteria (Acceptance Criteria 5th bullet).
- 🔵 minor / low — "Ships as a defined token without an active mapping" leaves the unmapped-token outcome slightly ambiguous (Assumptions).

### Completeness

**Summary**: Work item 0076 is structurally complete and richly populated for a story: it has substantive Summary, Context, Requirements, Acceptance Criteria, Dependencies, Assumptions, Technical Notes, Drafting Notes, and References sections. All frontmatter fields are present and valid. Type-appropriate story content (motivation, user perspective, definition of done) is present, and the Open Questions section is explicitly closed out rather than left dangling.

**Strengths**:
- Frontmatter complete and well-formed.
- Summary opens with explicit user-story framing.
- Context section substantial and explains the forces behind the work.
- AC contains five specific bullets covering token definition, hljs class mapping, diff tokens, regression coverage, and templates-preview migration.
- Requirements translates Context and AC into five concrete implementation directives.
- Open Questions explicitly resolved with pointer to Drafting Notes.
- Dependencies densely populated with blocked-by, related, and blocks edges.
- Technical Notes and Drafting Notes capture out-of-scope items, scope decisions, and rationale.

**Findings**: None.

### Dependency

**Summary**: The work item captures its primary upstream blocker (0033 token infrastructure) and a clear set of related/blocks couplings, and it explicitly names the renderer libraries and the shared CSS surfaces it must touch. The main dependency gap is the omission of 0042 (templates preview consumption) and 0089 (templates preview whitespace) from the upstream-blocker chain when in fact this story modifies the same templates-preview hljs mapping they touch — the ordering between these three stories is not unambiguous. A secondary concern is that 0075 is listed as merely 'Related' even though its overlap on `<pre>` radius / code-block CSS implies an ordering or coordination dependency.

**Strengths**:
- Explicitly names 0033 as the upstream blocker, with a Technical Note clarifying that 0033's AC1 left these tokens out of scope.
- Names 0088 (markdown body width harmonisation) as Blocks.
- Identifies 0083 (DevDesignSystem reference page) as related downstream consumer.
- External libraries named with explicit hljs class-name assumption.

**Findings**:
- 🟡 major / high — Templates-preview migration coupling with 0042 / 0089 not captured as ordering (Dependencies).
- 🔵 minor / medium — 0075 listed as Related but implies same-file ordering on code-block CSS (Dependencies).
- 🔵 minor / medium — `global.test.ts` cross-surface assertion has an implicit fixture dependency (Acceptance Criteria).
- 🔵 minor / low — DevDesignSystem (0083) showcase consumption left as Related, not Blocks (Dependencies).

### Scope

**Summary**: The work item is well-scoped around a single coherent capability: introduce the prototype's code-block syntax-highlight palette tokens and wire them into the existing renderer plus the templates preview through a shared mapping layer. The bundled work all serves one outcome — fenced code blocks and the templates preview consuming a single shared `--tk-*` palette — and the Drafting Notes explicitly justify keeping pieces together. Sizing is reasonable for a story; the touch points are bounded to token files, one shared stylesheet, and two consuming surfaces.

**Strengths**:
- Single unifying purpose across requirements and AC.
- Drafting Notes explicitly defend cohesion decisions.
- Renderer scope deliberately constrained.
- Out-of-scope explicitly called out.
- Dependencies identifies sibling and adjacent work items so coordination is visible.

**Findings**:
- 🔵 minor / medium — Templates-preview migration could plausibly be a sibling story (Requirements / Acceptance Criteria).
- 🔵 suggestion / low — Coordination overlap with 0075 on `<pre>` styling (Dependencies).

### Testability

**Summary**: The Acceptance Criteria are unusually concrete for a design-token story: they specify file paths, hljs class targets, computed-style assertions in both themes, and explicit token-name lists, making most criteria mechanically verifiable. Two areas weaken testability: AC4 leans on the open-ended phrase 'continue to pass without regression' with discretionary 'where coverage is insufficient' coverage, and AC2's enumeration ('one span per mapped hljs class') is governed by an Assumption that explicitly allows tokens to ship without active mappings — leaving the actual set of asserted classes ambiguous.

**Strengths**:
- AC1 specifies byte-level value parity against an authoritative source file.
- AC2 and AC3 frame verification as computed-style assertions on specific hljs class spans in both themes.
- AC5 specifies both structural change and behavioural check.
- Token lists enumerated by exact name and value.

**Findings**:
- 🟡 major / high — "Without regression" is unbounded; conditional test-addition clause is discretionary (AC4).
- 🟡 major / high — Set of hljs classes asserted in AC2 is not pinned down (AC2 and Assumptions).
- 🔵 minor / medium — Parity against "prototype constants" assumes a defined extraction procedure (AC1).
- 🔵 minor / medium — "rgb equivalent" of hex tokens leaves colour-space conversion implicit (AC2 and AC3).
- 🔵 minor / medium — AC5 conflates structural removal with behavioural assertion without naming the test (AC5).

## Re-Review (Pass 2) — 2026-05-21T21:42:00+00:00

**Verdict:** COMMENT

All eleven previously-identified issues across clarity, dependency, scope, and testability are resolved. The work item now defines an explicit hljs → `--tk-*` mapping table in Requirements, an enumerated AC4 coverage list of six behaviours, a committed `prototype-tokens.json` fixture with a sibling drift-detection test, a canonical `getComputedStyle.color` comparison rule via a named `hexToRgbString` helper, named test files for the templates-preview assertions, and corrected dependency edges (0042/0089/0083 promoted to Blocks, 0075 captured as Blocked-by with fallback). New minor findings — chiefly around precedence between general and language-scoped mapping rows, the `prototype-tokens.json` schema, and the `hexToRgbString` contract — surface implementation detail that can be settled during planning rather than blocking implementation.

### Previously Identified Issues

- 🔵 **Clarity**: `.previewBody :global(.hljs-*)` selector ambiguous — Resolved (AC5 cites lines 153-213 and adds grep/inspection check)
- 🔵 **Clarity**: "Matches the new palette" underspecified — Resolved (AC5 enumerates specific hljs classes and references AC2/AC3 canonical comparison)
- 🔵 **Clarity**: Unmapped tokens disposition ambiguous — Resolved (Requirements paragraph + Assumptions both state unmapped tokens ship without selectors)
- 🟡 **Dependency**: 0042 / 0089 templates-preview coupling — Resolved (both promoted to Blocks with ordering rationale)
- 🔵 **Dependency**: 0075 same-file ordering — Resolved (moved to Blocked-by with fallback clause)
- 🔵 **Dependency**: prototype-HTML fixture dependency — Resolved (named in Dependencies; fixture decouples unit-test fast path)
- 🔵 **Dependency**: 0083 left as Related — Resolved (moved to Blocks)
- 🟡 **Testability**: AC4 unbounded — Resolved (six enumerated behaviours, each requiring at least one test)
- 🟡 **Testability**: AC2 hljs class set not pinned — Resolved (Requirements mapping table is canonical)
- 🔵 **Testability**: AC1 extraction procedure undefined — Resolved (committed JSON fixture + drift-detection test against prototype HTML)
- 🔵 **Testability**: "rgb equivalent" colour-space implicit — Resolved (canonical hex→`rgb(r,g,b)` via `hexToRgbString`, rgba via two-decimal `rgba(r,g,b,a)`, exact string match)
- 🔵 **Testability**: AC5 test file not named — Resolved (names `LibraryTemplatesView.test.tsx`, enumerates required classes)
- 🔵 **Scope**: Templates-preview migration as sibling story — Resolved as judgment call (bundling now justified by mapping table + shared layer being one mechanism)
- 🔵 **Scope**: 0075 coordination overlap — Resolved (explicit ordering with fallback)

### New Issues Introduced

- 🔵 **Clarity**: `hljs-meta` appears in two mapping-table rows (general → `--tk-deco` and diff-scoped → `--tk-dhdr`) with no stated precedence rule — implementer cannot tell from the work item how to scope the diff variant in CSS (Requirements mapping table).
- 🔵 **Clarity**: AC4's "auto-detected python/typescript fenced code block" wording is ambiguous between an untagged fence relying on hljs auto-detection and a tagged ```python fence where hljs detects tokens within the declared language (AC4).
- 🔵 **Clarity**: AC5 lists `hljs-code`/`hljs-quote` but the mapping table only includes `hljs-quote` (paired with `hljs-comment`); `hljs-code` is undefined in the table; slash-separated AC5 entries are also ambiguous between "either" and "each" (AC5 vs Requirements mapping table).
- 🔵 **Clarity** (suggestion): "Shared layer" is referenced under multiple names ("shared CSS layer", "shared stylesheet rule", "shared mapping layer", "code-syntax.css") across Requirements, AC5, and Technical Notes — a canonical label on first use would speed cross-section verification (Requirements / AC / Technical Notes).
- 🔵 **Dependency**: The 0075 Blocked-by fallback clause inverts ordering but does not name the resulting reverse coupling (0075 would then become blocked by 0076); the bidirectional relationship is invisible under the fallback path (Dependencies).
- 🔵 **Dependency** (suggestion): The drift-detection test pins `prototype-standalone.html` at its current path; if a future cleanup moves the design-inventories directory the test breaks silently outside this story's scope. Worth flagging as a path/contract dependency in Dependencies or Assumptions.
- 🔵 **Dependency** (suggestion): 0088's "consumes the unified markdown code-block surface" rationale under Blocks does not match the explicit ordering language used for 0042/0089; unclear whether 0088 is strictly gated or only informationally coupled (Dependencies).
- 🔵 **Scope** (suggestion): Story now sits at the upper end of single-story sizing — five ACs (one with six sub-bullets), 24-row mapping table, four named new artefacts, two consumer migrations. Decomposition still not warranted (one mechanism), but expect higher capacity consumption and watch for slippage in the drift-detection test (Acceptance Criteria / Technical Notes).
- 🔵 **Scope** (suggestion): If the templates-preview test fixture (AC5) proves heavier than anticipated during planning, consider extracting AC5 into a follow-up — shared layer is designed to support that split cleanly. No structural change needed now (Requirements / AC).
- 🔵 **Testability**: AC2's "one span per row" instruction is ambiguous for two rows that contain multiple selectors — `hljs-function, hljs-title.function_` (class plus chained-class) and `hljs-meta.doctype` (qualified variant of `hljs-meta` which itself separately maps to `--tk-deco`). Need a stated rule on whether one span per class or per row, plus selector-precedence for the `hljs-meta`/`hljs-meta.doctype` overlap (AC2 + Requirements mapping table).
- 🔵 **Testability**: AC1 introduces `prototype-tokens.json` and a drift-detection test, but does not specify the fixture schema (flat map vs grouped) or the extraction rule (regex pattern, scoped to `:root` block, whitespace normalisation). Two reasonable implementations could produce non-equivalent fixtures (AC1 / Technical Notes).
- 🔵 **Testability**: AC4 enumerates six behaviours but leaves test placement undefined ("extend existing suites or add new ones as needed"). Each case is individually verifiable but a reviewer must discover test locations without a manifest (AC4).
- 🔵 **Testability**: `hexToRgbString` helper contract is not specified — input forms accepted (3-digit hex, uppercase), exact output format (must match `getComputedStyle` serialisation exactly), and whether rgba conversion is a sibling helper or branched behaviour. Without this, "exact string match" risks brittleness or tautological assertions (AC2 / Technical Notes).

### Assessment

The work item is ready for implementation. All major findings from the initial review are resolved and no critical or major findings remain — the suggested verdict is COMMENT rather than REVISE. The new minor findings cluster into implementation-detail decisions (precedence rules in the mapping table, fixture schema, helper contract) that are appropriate to settle during planning rather than blocking story acceptance. The story remains coherent and right-sized, with explicit ordering against all named dependencies. The single residual judgment call — whether the story is too large to deliver in one increment — is acknowledged in Drafting Notes and Scope's new suggestion; the unifying mechanism justifies keeping it as one story.

If desired, the planning phase can pick up the new minor findings — particularly the `hljs-meta` precedence, AC4 "auto-detected" wording, `hljs-code` table omission, and `hexToRgbString` contract — without re-opening the story for review.
