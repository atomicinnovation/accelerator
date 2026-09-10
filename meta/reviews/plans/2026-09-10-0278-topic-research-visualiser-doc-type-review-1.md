---
type: "plan-review"
id: "2026-09-10-0278-topic-research-visualiser-doc-type-review-1"
title: "Plan Review: Topic-Research Visualiser Doc Type and Indexer Implementation Plan"
date: "2026-09-10T23:05:30+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-10-0278-topic-research-visualiser-doc-type"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "correctness", "test-coverage", "code-quality", "compatibility", "usability", "documentation"]
review_number: 1
review_pass: 4
tags: ["visualiser", "doc-type", "topic-research", "rename"]
last_updated: "2026-09-11T09:01:51+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Topic-Research Visualiser Doc Type and Indexer Implementation Plan

**Verdict:** REVISE

The plan is unusually well-researched: its counts, line references,
precedent-mirroring, no-migration reasoning, and the `humanise_slug`
title-casing correction are largely exact against the working tree, and all
seven lenses open with genuine praise. But it does not reach its own "green on
`mise run`" bar as written — the Phase 1 rename and the Phase 3 count bumps
both omit real enum-consuming and count-pinning sites — and the status-chip
lifecycle mapping (the visible payoff of the `research_status` collapse) is
both semantically wrong (healthy states painted in the shared error/warn
palette) and mechanically unrealisable (the `violet` variant is never emitted,
`synthesised → indigo` bypasses the fill lever the plan pulls, and the
saturation-test infrastructure does not exist). Verification of two acceptance
criteria — dangling-reference resolution and the five-state chip test — rests
on unsound assumptions, and the 0277 co-land contract divergence is deferred
to a follow-up the block graph cannot enforce.

### Cross-Cutting Themes

- **Status-chip lifecycle mapping** (flagged by: Architecture, Code Quality,
  Usability, Test Coverage, Correctness) — the single most-flagged area. Two
  independent problems compound: a *design* problem (mapping the healthy
  ordinal lifecycle onto a categorical error/warn/ok palette, e.g. `red` on
  `briefed`) and a *mechanism* problem (the described "raise the shared 8%
  fill mix" edit cannot make the mapping satisfy its own test).
- **Incomplete edit-site enumeration that blocks green** (flagged by:
  Compatibility, Correctness) — the plan's exhaustive-looking inventory misses
  real consumers: Phase 1 omits `cluster-via-label.ts`, `template-tier.ts`,
  and many `data-stage="research"` test fixtures; Phase 3 omits `api_smoke.rs`
  and `compose_contract.rs`, which hard-code the doc-type count. Both phases
  fail `mise run check`/`test` as written.
- **0277 co-land contract divergence** (flagged by: Documentation,
  Compatibility, Architecture) — 0277, 0279, and epic 0121 still assert
  `research_status` and the "never infer set progress from base `status`" rule
  this change contradicts; the block graph permits 0277 to merge alone; the
  reconciliation is deferred to an unowned out-of-band follow-up.
- **Verification soundness** (flagged by: Correctness, Test Coverage) — the
  dangling-reference criterion and the five-state chip test may each pass
  vacuously or fail as specified, because they rest on the wrong index
  (`slug:` vs `(type, id)`) and on non-existent colour-resolution utilities.

### Tradeoff Analysis

- **Type stability vs name clarity**: keeping the Rust identifier `Research`
  avoids a `cargo-public-api` shift and call-site churn, but leaves it as the
  second type (after `PrDescriptions`) whose identifier matches none of its
  user-facing names. Recommendation: keep the stability choice, add a
  doc-comment at the variant recording that its wire/linkage names are
  `codebase-research` and why the identifier is retained.
- **Reuse the five existing variants vs a dedicated lifecycle scale**: reusing
  `green|indigo|amber|red|violet` avoids new variants but forces a healthy
  state onto `red` and, per Code Quality/Test Coverage, is not even reachable
  (no `violet` branch; `indigo` bypasses the mix). Recommendation: introduce a
  dedicated ordinal lifecycle scale decoupled from the ok/warn/err tones, or at
  minimum keep `red`/`amber` off every healthy state.
- **URL continuity — slug-dropping redirect vs slug-preserving alias**:
  Compatibility rates the slug-drop minor (localhost dev tool, one consumer);
  Usability rates it major (bookmarks/shared links break to `/library`).
  Recommendation: add the slug-preserving alias — the plan already commits to
  continuity via the `localStorage` rewrite, and the cost is one redirect that
  rewrites only the type segment.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Architecture / Code Quality / Usability**: Lifecycle states painted in
  the shared semantic error/warn/ok palette
  **Location**: Phase 3, Section 6: Status-chip lifecycle mapping
  `api/status-variant.ts` is a deliberately doc-type-agnostic lexicon where
  `red`→`--ac-err` (blocked/rejected/deprecated/abandoned) and
  `amber`→`--ac-warn`. Mapping `briefed → red` and `outlined → amber` paints
  perfectly-healthy states as errors/warnings, injects lifecycle words into a
  global set every doc type shares, and gives the assignment no rule a reader
  can infer.

- 🟡 **Code Quality / Test Coverage / Usability / Correctness**: The
  status-chip mechanism is under-specified and partly unrealisable as written
  **Location**: Phase 3, Section 6; Testing Strategy › Status chips
  Four compounding facts: `statusToVariant` never emits `violet` today (no set,
  no branch, `__SETS_FOR_TEST` is `[GREEN, INDIGO, AMBER, RED]`), so
  `researching → violet` needs new code, not a word added to a set;
  `synthesised → indigo` fills from `var(--ac-accent-faint)`, not the
  `color-mix(… 8%, --ac-bg)` pattern, so raising the shared 8% mix cannot lift
  its saturation; the ≥15%-saturation test needs a color-mix parser and an
  sRGB→HSL function that do not exist and is mis-placed in the pure-TS
  `status-variant.test.ts`; and ≥15% saturation vs ≥3:1 contrast pull in
  opposite directions with no verified percentage. The load-bearing red→green
  chain cannot reach green.

- 🟡 **Compatibility**: `cluster-via-label.ts` switch consumes the old
  `DocTypeKey` value and is absent from the Phase 1 edit list
  **Location**: Phase 1, Changes Required §3
  `routes/lifecycle/cluster-via-label.ts:18` has `case "research":` over
  `entry.type: DocTypeKey`. Once `research` leaves the union TypeScript rejects
  the label (TS2678); left in place, `codebase-research` falls through to
  `default` and gets the wrong lifecycle-cluster attribution. Its fixture
  `cluster-via-label.test.ts:14` also hard-codes `"research"`.

- 🟡 **Compatibility**: `template-tier.ts` `STEM_TO_GLYPH` glyph *value* (and
  its tests) still target the old glyph key
  **Location**: Phase 1, Changes Required §3
  `routes/library/template-tier.ts:40` maps `research: "research"` where the
  *value* is a `GlyphDocType` (= `DocTypeKey`). The plan's "every map key"
  framing misses this value-side rename: after the rename `"research"` is not
  assignable (compile error), and `glyphKeyForTemplate("codebase-research")`
  must resolve to the `codebase-research` glyph.
  `LibraryTemplatesIndex.test.tsx:77,84` assert the old value.

- 🟡 **Compatibility**: `data-stage` / `docType` test fixtures keyed on
  `"research"` are not enumerated
  **Location**: Phase 1, Changes Required §5; Testing Strategy
  `Pipeline.test.tsx`, `PipelineMini.test.tsx`, `WorkItemCard.test.tsx`,
  `Sidebar.test.tsx`, `SearchResultsPanel.test.tsx`,
  `LibraryOverviewHub.test.tsx`, and `use-unseen-doc-types.test.ts` reference
  `research` as a stage/docType token — runtime element-not-found or compile
  errors where `present` is `DocTypeKey[]`. `frontend:check` is not green until
  they move.

- 🟡 **Correctness**: Phase 3 count-site enumeration omits two server
  integration tests that hard-code 14/13
  **Location**: Phase 3, Section 1: Rust registry (count/parity bumps)
  `server/tests/api_smoke.rs:85` asserts the `/api/types` array length is 14,
  and `server/tests/compose_contract.rs:48`
  (`resolves_all_thirteen_doc_paths_…`) asserts a 13-key list built from
  `DOC_TYPES`. Adding the catalogue row takes both to 15/14; neither is in the
  bump list, so Phase 3 fails `server:check`/`test`.

- 🟡 **Correctness / Test Coverage**: Dangling-reference resolution conflates
  the visualiser indexer with the `(type, id)` corpus index
  **Location**: Phase 3, Section 7 (fixtures); Testing Strategy › Dangling
  references
  The whole-corpus check resolves references against an index keyed by each
  document's `(type, id)` — id from `id:`/`work_item_id:`/filename-stem, never
  `slug:` or the visualiser indexer. The corpus-cli fixture uses
  `id: "manifest"`, so its key is `topic-research:manifest`, not
  `topic-research:<slug>` — the reference would remain DANGLING (and a constant
  `id: manifest` collides as DuplicateId across sets). No concrete test/corpus
  is named and there is no negative control, so it may also pass vacuously.

- 🟡 **Documentation / Compatibility / Architecture**: The 0277 co-land
  `research_status` reconciliation is under-specified and not graph-enforceable
  **Location**: Migration Notes; Implementation Approach
  0277 (lines 88/93/98/103/219, including "never infer set progress from base
  `status`"), 0279 (lines 32/52/57/73), and epic 0121's artifact contract still
  assert `research_status`. The block graph permits 0277 to merge without 0278;
  the reconciliation is recorded only as prose/manual-verification with no
  checkboxes, owner, or line references, so a reviewer can complete every box
  with contradictory contracts still on disk.

- 🟡 **Test Coverage**: The `localStorage` rewrite ships with weaker coverage
  than its own precedent
  **Location**: Phase 1, Change 4; Success Criteria › Manual Verification
  The `prs → pr-descriptions` precedent has a dedicated `parseStored migration`
  unit test. The `research → codebase-research` rewrite lists only manual
  verification and specifies no automated test, and none for its
  idempotency/no-clobber guard.

- 🟡 **Test Coverage**: The indexer integration test is weaker than the
  precedent it claims to mirror
  **Location**: Phase 3, Sections 2 & 7; Testing Strategy › Indexer
  `design_inventories_indexed_from_nested_directories` uses two real sets (to
  prove multiplicity) and an empty directory (to prove the missing-manifest
  skip). The plan's primary fixture has one real set and offers the fallback
  sets as possible unit cases, so the test may assert `len == 1` vacuously and
  drops the missing-manifest skip entirely.

- 🟡 **Correctness**: The phase-ordering justification contradicts the "no
  validation gate over visualiser fixtures" claim
  **Location**: Implementation Approach (P2→P3); What We're NOT Doing (report
  row)
  The visualiser server fixtures are never schema-validated
  (`this_repositorys_own_corpus_is_clean` walks `<repo-root>/meta/`, not the
  fixtures dir), so `status: synthesised` there is not rejected pre-P2 — the
  stated P2→P3 dependency is unenforced where claimed. Conversely, if the
  dangling-reference requirement forces a real `meta/research/topics/` set into
  the corpus, that set *is* validated, and a `kind: report` sub-document there
  is rejected UNKNOWN-KIND once topic-research is registered.

- 🟡 **Usability**: The old research URL redirects to `/library`, dropping the
  slug; no slug-preserving alias
  **Location**: Phase 1, Change 5; Migration Notes
  `router.ts` throws `redirect({ to: "/library" })` for any unknown type, so a
  bookmarked `/library/research/<slug>` lands on the library root rather than
  `/library/codebase-research/<slug>` — recoverable (the slug is unchanged) but
  not mapped. Inconsistent with the care taken for `localStorage` continuity.
  (Compatibility rates this minor given the single localhost consumer; see
  Tradeoff Analysis.)

#### Minor

- 🔵 **Architecture**: Phase 3's "atomic vertical" bundles the global
  status-chip restyle, which is not part of the breakage it must avoid
  **Location**: Implementation Approach / Phase 3 Overview
  The atomicity argument (half-registered type breaks the visualiser) justifies
  landing the registries together, not the global `Chip.module.css` fill raise;
  bundling the higher-blast-radius chip change into the same review/rollback
  boundary dilutes cohesion.

- 🔵 **Architecture**: Wire-key↔VR-baseline-filename coupling is guarded only
  by the out-of-loop Docker VR lane
  **Location**: Phase 1, Section 6 / Phase 3, Section 8
  The wire key is encoded into 10 baseline PNG filenames, but VR runs only via
  `mise run test:e2e:visualiser:docker`, excluded from the fast `check` loop —
  a rename can pass the whole fast mirror with stale/missing baselines.

- 🔵 **Code Quality**: `in_lifecycle() == true` is implicit and diverges from
  `participates_in_lifecycle() == false` opaquely
  **Location**: Phase 3, Section 1 / Key Discoveries (predicate mirror)
  `TopicResearch` gets `in_lifecycle` true only by omission from a negation,
  yet `participates_in_lifecycle` is false. Have the planned predicate-mirror
  test assert TopicResearch's *full* boolean profile equals DesignInventories'.

- 🔵 **Documentation**: Template inline `# complete` comment left stale by the
  status-value change
  **Location**: Phase 2, Change 2 (Template)
  Line 8 `status: "complete"  # complete` becomes `status: "briefed"`; the plan
  is silent on the trailing comment, which would then contradict the value, and
  the four-state vocab hint on the removed `research_status` line disappears.

- 🔵 **Documentation**: The plan's description of the template body Status
  section does not match the resulting edit
  **Location**: Phase 2, Change 2 (body "Research status" line)
  Removing body line 35 leaves the `## Status` section with no lifecycle value
  shown, contradicting the plan's "the Status section reads base `status`".
  State whether a body line is kept (reading base `status`) or omitted.

- 🔵 **Documentation**: The collapse overloads `status` in the skill without
  disambiguating the manifest
  **Location**: Phase 2, Change 3 (skill verbs/preconditions)
  Every set document carries a base `status` (including `brief.md`, written
  `draft`/`complete` by the same skill), so "requires `status: briefed`" is
  ambiguous. Keep the reference explicit ("the manifest's base `status`").

- 🔵 **Documentation**: The plan states "six files" but enumerates (and only
  five exist)
  **Location**: Key Discoveries — "research_status surface is six files"
  A repo-wide grep finds the literal in five files; template lines 8/35 are the
  base `status` value and the body heading, not `research_status` literals.

- 🔵 **Test Coverage**: The slug empty-guard is untested
  **Location**: Phase 3, Section 1 (slug.rs identity arm)
  The arm is `Some(stem.to_owned()).filter(|s| !s.is_empty())` but only the
  happy path is asserted; removing `.filter(...)` would leave tests green while
  returning `Some("")`. Add a `derive(TopicResearch, ".md", …) → None` case.

- 🔵 **Test Coverage**: The blanket TDD claim overstates what is achievable for
  compile-coupled tests
  **Location**: Implementation Approach (red-green claim); Testing Strategy
  Only the count assertions are genuinely red-first; the indexer test, slug
  test, `parity.rs` row, and `api_types` row reference `DocTypeKey::TopicResearch`
  and cannot compile (let alone fail an assertion) before the variant exists.
  State the co-land ordering per behavioural test.

- 🔵 **Test Coverage**: Phase 2 adds no negative assertion that the collapse
  took effect
  **Location**: Phase 2, Changes 1 & 4
  The plan verifies only that base `status: briefed`/`synthesised` validate; add
  a case asserting a manifest still carrying `research_status` (or an
  out-of-vocab status) now fails, confirming the field was retired.

- 🔵 **Usability**: The two research types are separated in the Discover order
  and distinguished by a vague qualifier
  **Location**: Phase 3, Change 3 / Desired End State
  `TopicResearch` is inserted at the front of the Discover group, with the two
  design types between it and `Research`, so the related pair sits at opposite
  ends; "Topic research" is a generic qualifier for "external-subject
  deep-research". Consider placing the two research types adjacent.

- 🔵 **Usability / Documentation**: No reusable doc-type registration checklist;
  several edit sites fail silently
  **Location**: References / Testing Strategy; Phase 3
  The `tasks/README.md` checklists cover sub-binaries/crates, not doc types;
  0280 and 0281 consume this same registration and must re-derive the ~15-site
  map. Some sites fail silently (the `Glyph.module.css` selector, VR filenames,
  the `localStorage` rewrite). Capture a "registering a doc type" checklist.

- 🔵 **Usability**: The design sign-off gate has no reference artifact for the
  deviated chip mapping
  **Location**: Phase 3, Manual Verification
  The chip mapping deliberately deviates from the prototype, yet the gate asks
  the reviewer to sign it off "against the prototype" — the one contentious
  surface has no target. Give the chip sign-off its own acceptance bar/swatch.

- 🔵 **Correctness**: The shared chip-fill raise must satisfy ≥15% saturation
  and ≥3:1 contrast simultaneously for all five variants
  **Location**: Phase 3, Section 6 (Chip.module.css mix raise)
  A higher accent percentage raises saturation but can cut text-vs-fill
  contrast; a single global percentage may not clear both bounds for the palest
  and deepest tones. Verify a concrete percentage (or per-variant fills)
  numerically before committing.

#### Suggestions

- 🔵 **Architecture / Code Quality**: Record why `Research` diverges from its
  user-facing names
  **Location**: Phase 1, Section 1
  After the rename `DocTypeKey::Research` has wire/linkage `codebase-research`
  and label "Codebase research" while the identifier and config key keep
  `research`. Add a doc-comment at the variant so the divergence is discoverable
  at the definition site.

- 🔵 **Code Quality**: The exhaustiveness test name encodes a magic count
  **Location**: Phase 3, Section 1 / Testing Strategy
  The rename `all_returns_fourteen…` → `all_returns_fifteen…` bakes the count
  into the identifier and goes stale on every variant addition. Since the plan
  already renames it, consider a count-free name (the `assert_eq!(…, 15)` still
  pins the number).

- 🔵 **Code Quality**: Plan-embedded trailing comments contradict the repo's
  comment convention
  **Location**: Phase 3, Section 1 (Rust registry snippet)
  The snippet annotates each arm (`// config_path_key`, `// wire_str`, …) — the
  exact what-not-why comments the repo removes from plans. Move the
  disambiguation into prose and drop the trailing comments.

### Strengths

- ✅ The plan's cited line numbers, current values, and no-migration reasoning
  are exact against the tree: the four enumerated count/parity sites
  (`doc_type.rs:28/244`, `catalogue.rs:275`, `api_types.rs:32`,
  `parity.rs:61/93`), the six `research_status` skill sites, the schema/TSV
  row, and the corpus-cli fixture all check out.
- ✅ It extends the doc-type system through its designed seam: a single
  `nested_manifest_filename() → Some("manifest.md")` drives both `file_driver`
  enumeration and `indexer` slug/title derivation, making `TopicResearch` a
  genuine data-mirror of `DesignInventories` with almost no new control flow.
- ✅ The slug identity arm, the indexer exclusion logic (one entry per set,
  dot-dir skip, sub-documents never descended), the DesignInventories predicate
  mirror, and the idempotent `localStorage` guard are all verified correct.
- ✅ It correctly separates the wire token from the Rust type identity (keeping
  `DocTypeKey::Research`), avoiding a `cargo-public-api` shift and call-site
  churn — the right decomposition of "rename the API surface" vs "rename the
  type".
- ✅ Collapsing `research_status` onto base `status` establishes a single
  authoritative status field and removes per-type status logic from the card.
- ✅ The no-migration claim holds (linkage name and config key already read
  `codebase-research` via `m0004`; the wire token never touches disk), the
  `status_vocab` widening rejects no existing manifest, and the redirect for the
  old route is genuinely implemented, not merely hoped for.
- ✅ It corrects the work item's `humanise_slug` expectation (title-case per
  hyphen segment, `model-context-protocol → "Model Context Protocol"`), rather
  than carrying the wrong sentence-case assertion forward.
- ✅ The phase dependency graph is minimal and the frontend registry is
  fail-closed by construction (every total `Record<DocTypeKey, …>` map is a
  compile error until the key is added).

### Recommended Changes

1. **Rework the status-chip lifecycle mapping end-to-end** (addresses: the two
   status-chip majors, plus the CVD, contrast-conflict, and sign-off minors).
   Decouple lifecycle from the semantic error/warn/ok palette — a dedicated
   ordinal scale (e.g. the type's own hue deepening briefed→complete, or a
   grey→indigo→violet→green positive ramp), or at minimum keep `red`/`amber`
   off every healthy state. Specify the actual code: a new `VIOLET` set + a
   `return "violet"` branch + the `__SETS_FOR_TEST` update; fix `indigo`'s
   `--ac-accent-faint` fill (or move `synthesised` off `indigo`) so the
   saturation lever reaches it. Specify the missing test infrastructure (a
   color-mix resolver over the real tokens + an sRGB→HSL function), place it
   with the `global.css`-parsing tests rather than `status-variant.test.ts`,
   and verify a concrete percentage clears ≥15% saturation *and* ≥3:1 contrast
   for all five variants. Give the chip sign-off its own acceptance bar.

2. **Complete the Phase 1 rename edit list** (addresses: the three
   Compatibility majors). Add `cluster-via-label.ts` (`case "research"` →
   `codebase-research`) and its test; move `template-tier.ts`'s `STEM_TO_GLYPH`
   *value* (stem key retained) and the two `LibraryTemplatesIndex.test.tsx`
   assertions; sweep the frontend test tree for `research` used as a
   wire/stage/docType token and fold those fixtures into Phase 1 — keeping the
   deliberately-untouched namespaces (template stem, `hasResearch` serde field,
   `DETAIL_ROUTE_SLUGS` value) distinct.

3. **Complete the Phase 3 count-site list** (addresses: the Correctness count
   major). Add `api_smoke.rs:85` (14→15) and `compose_contract.rs:48` (add
   `research_topics`, rename the "thirteen" test) to the bump list.

4. **Fix the dangling-reference fixture and verification** (addresses:
   Correctness/Test Coverage). Pin the manifest's `id:` to equal the set slug
   so `topic-research:<slug>` resolves; verify against
   `build_index`/`resolve_own_type_id`, not the visualiser indexer; name the
   concrete test and corpus; add a negative control (dangling when the set is
   absent).

5. **Make the 0277 co-land enforceable and reconcile the contracts**
   (addresses: Documentation/Compatibility/Architecture). Prefer one merge unit
   (single PR / squashed train) or a mutual PR-blocking gate over out-of-band
   coordination; promote the `research_status` reconciliation to tracked,
   checkbox-verified tasks citing 0277 lines 88/93/98/103/219, 0279 lines
   32/52/57/73, and the epic-0121 contract, and retire the "never infer set
   progress from base `status`" rule wherever it appears.

6. **Strengthen the under-specified tests** (addresses: the Test Coverage
   minors). Add the `parseStored migration` + idempotency/no-clobber tests for
   the `localStorage` rewrite; give the indexer fixture ≥2 real sets and a
   missing-manifest set to skip; add the slug empty-guard case; add the Phase 2
   negative case asserting a leftover `research_status` now fails validation.

7. **Fix the phase-ordering rationale and the report-kind risk** (addresses:
   Correctness). State the real ordering mechanism (the corpus-cli / real-corpus
   fixtures carry validated `status: synthesised`, not the visualiser fixtures),
   and guarantee the `kind: report` sub-document lives only in the unvalidated
   visualiser fixtures, never in a real corpus set walked by the whole-corpus
   validator.

8. **Apply the documentation and convention fixes** (addresses: the
   Documentation minors and Code Quality suggestions). Resolve the template
   `# complete` comment and the body Status-section decision; make the six skill
   sites say "the manifest's base `status`"; correct "six files" → five; add the
   `Research` variant doc-comment; drop the trailing comments from the Rust
   snippet; decide the slug-preserving URL alias (or record the slug-drop as an
   explicit decision); consider de-magicking the exhaustiveness test name.

## Per-Lens Results

### Architecture

**Summary**: Architecturally disciplined — it extends the doc-type system
through its intended seams (a new `DocTypeKey` variant + the data-driven
`nested_manifest_filename` switch mirroring `DesignInventories`), keeps the
frontend fail-closed via compiler-enforced total maps, and separates the wire
rename from the Rust type identity to avoid public-API churn. The main
structural concerns are not in the new type but in how two changes reach into
shared, doc-type-agnostic surfaces (the global status lexicon and the shared
chip fill) to satisfy one type's visual requirement, and in the hand-synced
parity fabric whose only safety net for some couplings (VR baseline filenames)
sits outside the fast CI loop.

**Strengths**:
- Extends the registry through its designed seam; the new type is a genuine
  data-mirror of `DesignInventories` with almost no new control flow.
- Frontend registry fail-closed by construction (compile error until the key is
  added; wildcard-free Rust match arms).
- Correctly separates the wire token from the Rust type identity.
- The `research_status` → base `status` collapse yields a single authoritative
  status field.
- Minimal, honest phase dependency graph isolating the one real ordering
  constraint.
- No corpus migration needed (`m0004` already moved the persisted surfaces).

**Findings**:
- **major** (high) — _Phase 3, Section 6 (status-variant.ts)_: Per-type
  lifecycle palette overloads the shared, doc-type-agnostic status lexicon and
  inverts the `red`/error semantics. A single type's visual requirement mutates
  a global vocabulary; `red` no longer reliably means error, and other types
  whose status word is `briefed`/`researching`/etc. silently inherit the tones.
  Resolve the mapping in a topic-research-scoped layer or a lifecycle-specific
  variant set.
- **minor** (medium) — _Phase 3, Section 6 (Chip.module.css)_: Raising the
  shared chip fill mix to meet one type's saturation threshold couples every
  chip to topic-research's requirement. Scope the raised fill to the lifecycle
  variants.
- **minor** (medium) — _Implementation Approach / Phase 3 Overview_: The
  "atomic vertical" bundles the global status-chip restyle, which is not part of
  the breakage it must avoid. Split the lifecycle chip palette into a
  separately-reviewable change (still sequenced after Phase 2).
- **minor** (medium) — _Phase 1 §6 / Phase 3 §8_: Wire-key↔baseline-filename
  coupling is guarded only by the out-of-loop Docker VR lane. Consider a cheap
  in-loop assertion that baseline filenames match `DOC_TYPE_KEYS`.
- **minor** (medium) — _Implementation Approach / Migration Notes_: The plan
  hard-depends on 0277 scaffolding, but the dependency model can only express it
  out-of-band. Prefer landing 0277 + 0278 as one merge unit.
- **suggestion** (medium) — _Phase 1, Section 1_: Post-rename the `Research`
  identifier diverges permanently from every user-facing name it now carries.
  Add a doc-comment recording the wire/linkage names and why the identifier is
  retained.

### Correctness

**Summary**: Core arithmetic and logic are largely sound — the four enumerated
count/parity sites, the slug identity arm, the nested-manifest exclusion logic,
the DesignInventories predicate mirror, and the idempotent `localStorage`
rewrite all check out exactly. But the Phase-3 count-site enumeration is
incomplete (it misses `api_smoke.rs:85` and `compose_contract.rs:48`), so Phase
3 would not be green as claimed; the phase-ordering justification and the
dangling-reference criterion also rest on an internal inconsistency about which
fixtures are schema-validated and on conflating the visualiser indexer with the
corpus reference index.

**Strengths**:
- Count arithmetic for the four enumerated sites is exact (14→15/15/14 as
  planned; the parity row self-checks `rows.len() == all().len()`).
- The slug identity arm is genuinely required (`strip_prefix_date…` returns
  `None` for a bare subject name) and correct.
- The indexer exclusion claim holds by construction.
- The `localStorage` guard is correct and safe against double-application.
- The DesignInventories predicate mirror is verified exact.
- The `humanise_slug` title-casing correction is right.

**Findings**:
- **major** (high) — _Phase 3, Section 1_: Phase 3 count-site enumeration omits
  two server integration tests that hard-code 14/13 — `api_smoke.rs:85`
  (`len == 14`) and `compose_contract.rs:48` (13-key list from `DOC_TYPES`).
  Both fail once the catalogue row is added. Add them to the bump list.
- **major** (medium) — _Implementation Approach (P2→P3) / What We're NOT
  Doing_: The phase-ordering justification contradicts the "no validation gate
  over visualiser fixtures" claim. The visualiser fixtures are never
  schema-validated, so `status: synthesised` there is not rejected pre-P2; a
  real `meta/research/topics/` set, by contrast, *is* validated and a
  `kind: report` sub-doc there breaks the build (UNKNOWN-KIND).
- **major** (medium) — _Phase 3, Section 7 / Testing Strategy_: Dangling-
  reference resolution conflates the visualiser indexer with the `(type, id)`
  corpus index. The corpus-cli fixture's `id: "manifest"` keys as
  `topic-research:manifest`, so `topic-research:<slug>` remains DANGLING (and
  `id: manifest` collides across sets). Pin the manifest `id:` to the set slug;
  verify against `build_index`/`resolve_own_type_id`.
- **minor** (low) — _Phase 3, Section 6_: The shared chip-fill raise must
  satisfy ≥15% saturation *and* ≥3:1 contrast simultaneously for all five
  variants; the constraints pull against each other. Verify a concrete
  percentage numerically.

### Test Coverage

**Summary**: The plan leans on compiler-enforced total maps and count/parity
assertions for strong structural coverage almost for free, and correctly plans
positive fixtures for all three title-fallback branches. But the load-bearing
five-state status-chip test rests on colour-resolution infrastructure that does
not exist (and cannot be satisfied for one of five states by the described
edit), the `localStorage` rewrite drops an available unit-test precedent, the
indexer test is weaker than the precedent it claims to mirror, and several new
branches (empty-slug arm, dangling-reference resolution, `research_status`
removal) are asserted rather than concretely covered.

**Strengths**:
- Count/parity coverage is genuinely red-first and compiler-backed.
- All three title-derivation branches are positively planned, with the correct
  title-cased output pinned.
- The indexer test is modelled on a proven precedent; the schema↔TSV parity
  test guards the Phase 2 collapse for free.
- Reusing `fixture-coverage.spec.ts` gives end-to-end route coverage.

**Findings**:
- **major** (high) — _Phase 3, Section 6; Testing Strategy › Status chips_: The
  five-state chip test rests on non-existent utilities (no color-mix parser, no
  sRGB→HSL), the `synthesised → indigo` state cannot pass the saturation raise
  (`indigo` fills from `--ac-accent-faint`), and the assertions are mis-placed
  in `status-variant.test.ts`. Specify the infra, place it with the CSS-parsing
  tests, and fix or re-route `indigo`.
- **major** (high) — _Phase 1, Change 4_: The `localStorage` rewrite lists only
  manual verification despite the `prs → pr-descriptions` precedent having a
  dedicated `parseStored migration` test; no idempotency/no-clobber test. Add
  both.
- **major** (medium) — _Phase 3, Sections 2 & 7_: The indexer test may see only
  one set and assert `len == 1` vacuously, and drops the missing-manifest skip
  case the precedent was shaped to catch. Use ≥2 real sets + a manifest-less
  directory.
- **minor** (high) — _Phase 3, Section 1 (slug.rs)_: The empty-slug guard is
  untested; add `derive(TopicResearch, ".md", …) → None`.
- **minor** (medium) — _Phase 3, Section 7 / Success Criteria_: The
  dangling-reference check names no test/corpus and has no negative control, so
  it may pass vacuously.
- **minor** (medium) — _Implementation Approach / Testing Strategy_: The blanket
  red-green claim overstates compile-coupled tests, which cannot fail an
  assertion before the variant exists.
- **minor** (medium) — _Phase 2, Changes 1 & 4_: No negative assertion that a
  manifest still carrying `research_status` is now rejected.

### Code Quality

**Summary**: Well-grounded, reuses the proven `DesignInventories` precedent to
keep the new type low-complexity, and respects the repo's deliberately
hand-synced parity convention. The dominant concern is the status-chip mapping:
it forces an ordinal lifecycle onto a colour-named variant vocabulary whose CSS
binds those names to semantic error/warn/ok tokens, and the plan's remediation
lever ("raise the shared 8% fill mix") both mischaracterises the Chip CSS and
under-specifies the new set/branch the mapping requires. Naming (magic count in
a test name) and plan-embedded comments are minor deviations from repo idiom.

**Strengths**:
- The `research_status` collapse removes a duplicated status primitive.
- `TopicResearch` is a faithful mirror of `DesignInventories`, so indexing needs
  no bespoke server code.
- The plan respects the hand-synced parity design rather than inventing codegen.
- Explicit red-green framing and a planned predicate-mirror test.

**Findings**:
- **major** (high) — _Phase 3, Section 6_: Lifecycle states mapped onto semantic
  error/warn variant tokens (`red`→`--ac-err`, `amber`→`--ac-warn`,
  `green`→`--ac-ok`), so `briefed → red` renders a healthy state in the error
  colour, non-monotonically. Introduce a dedicated ordinal lifecycle scale or
  rename variants to role-based identifiers.
- **major** (high) — _Phase 3, Section 6_: The "shared 8% fill mix" framing
  mischaracterises `Chip.module.css` and misses `indigo`. Only
  green/amber/red/violet use the 8% `color-mix`; `indigo` uses
  `var(--ac-accent-faint)`, so raising the mix does nothing for
  `synthesised → indigo`. Address indigo separately and pin the exact
  percentage(s).
- **minor** (high) — _Phase 3, Section 6_: "Reuse exactly the five existing
  variants" understates the mapper change — `statusToVariant` never returns
  `violet` today (no set, no branch, `__SETS_FOR_TEST` omits it). Spell out the
  new set, branch, and test-export update.
- **suggestion** (high) — _Phase 3, Section 1_: The exhaustiveness test name
  encodes a magic count; de-magic it while renaming.
- **suggestion** (medium) — _Phase 3, Section 1_: Plan-embedded trailing
  comments in the Rust snippet contradict the repo's comment convention; move to
  prose.
- **suggestion** (medium) — _Phase 3, Section 1 / Key Discoveries_:
  `in_lifecycle=true` is implicit and diverges from `participates_in_lifecycle`;
  have the mirror test assert the full boolean profile.

### Compatibility

**Summary**: Core instincts sound — the wire rename persists nothing,
`from_wire_str` auto-follows `wire_str`, the `status_vocab` widening rejects no
manifest, and server+frontend ship as one binary (no version skew). But the
central "in lockstep across every map key" claim is incomplete: at least two
production consumers of the enum value (`cluster-via-label.ts`,
`template-tier.ts`) and several stage-token fixtures live outside the edit list,
and because `GlyphDocType === DocTypeKey` and switch cases are
comparability-checked, these are compile/runtime breaks. The 0277 co-land is a
genuine coupling risk the block graph cannot enforce.

**Strengths**:
- The no-migration claim holds; no on-disk corpus data is orphaned.
- The `localStorage` continuity story covers all doc-type-keyed persisted state
  (only `SEEN_DOC_TYPES_STORAGE_KEY` is doc-type-keyed).
- The `status_vocab` widening is backward-compatible.
- The old-URL redirect is genuinely implemented.
- Single-binary distribution → atomic rename at deploy.

**Findings**:
- **major** (high) — _Phase 1 §3_: `cluster-via-label.ts:18` `case "research":`
  over `DocTypeKey` is not in the edit list — TS2678 once the union changes, and
  wrong lifecycle attribution if left in place. Add it and its test.
- **major** (high) — _Phase 1 §3_: `template-tier.ts:40` `STEM_TO_GLYPH` glyph
  *value* still targets `research` — compile error and wrong glyph resolution.
  Move the value; update `LibraryTemplatesIndex.test.tsx:77,84`.
- **major** (medium) — _Phase 1 §5 / Testing Strategy_: `data-stage`/`docType`
  fixtures keyed on `research` across `Pipeline.test.tsx`, `PipelineMini`,
  `WorkItemCard`, `Sidebar`, `SearchResultsPanel`, `LibraryOverviewHub`,
  `cluster-via-label.test`, `use-unseen-doc-types.test` are unlisted. Sweep and
  fold into Phase 1.
- **major** (medium) — _Migration Notes / Implementation Approach_: The 0277
  co-land / `research_status` divergence is not graph-enforceable; 0277 can
  merge alone. Use a shared merge train / mutual PR gate and update the ACs and
  epic-0121 contract in the same change set.
- **minor** (low) — _Desired End State / Phase 1 Success Criteria_: The old URL
  redirects to `/library` but drops the slug; no alias to the renamed route. Low
  blast radius (localhost dev tool); record the trade-off explicitly.

### Usability

**Summary**: Exceptionally thorough for the maintainer audience — enumerated
edit sites, compiler-enforced maps, `localStorage` continuity, so onboarding is
largely a pit of success. For the end user, the status-chip lifecycle mapping is
the weak point: it forces healthy lifecycle states onto the shared palette's
error (`red`) and warning (`amber`) tones, and maps the two adjacent states onto
two adjacent, low-saturation, CVD-confusable hues. Two continuity/onboarding
gaps (URL rename drops the destination; no reusable registration checklist)
round out the findings.

**Strengths**:
- The chip always renders the status word as text, so state is conveyed by more
  than hue.
- Registration leans on compiler-enforced maps + self-guarding tests → loud
  failures.
- The table-driven enumeration is an excellent maintainer roadmap.
- The one-shot `localStorage` rewrite preserves last-seen state.
- The relabel + distinct glyph/hue/TYPE_COPY give progressive disclosure.

**Findings**:
- **major** (high) — _Phase 3, Section 6_: `briefed → red` and
  `outlined → amber` reuse the exact error/warn tones, so healthy sets read as
  broken/needing-attention — a least-surprise violation. Use a sequential ramp;
  at minimum avoid `red` on any healthy state.
- **major** (medium) — _Phase 3, Section 6 (accessibility)_: `researching →
  violet` and `synthesised → indigo` are adjacent blue-purples at ~15%
  saturation that read alike; `indigo` bypasses the mix so the saturation test
  may not even pass; `briefed`(red)/`complete`(green) are the CVD-confusable
  extremes. Pick hues with clear separation; verify under CVD simulation.
- **major** (medium) — _Phase 1, Change 5 / Migration Notes_: The bookmarked
  `/library/research/<slug>` is silently dropped to `/library`, losing the
  destination, despite the same rename getting a `localStorage` continuity
  rewrite. Add a slug-preserving redirect.
- **minor** (medium) — _Phase 3, Change 3 / Desired End State_: The two research
  types sit at opposite ends of the Discover group, distinguished by the vague
  qualifier "Topic". Place them adjacent; consider a more concrete label.
- **minor** (high) — _References / Testing Strategy_: No reusable doc-type
  registration checklist (the `tasks/README.md` ones are for sub-binaries), and
  several sites fail silently. Capture a "registering a doc type" checklist.
- **minor** (medium) — _Phase 3, Manual Verification_: The design sign-off asks
  the reviewer to judge the deliberately-deviated chip mapping "against the
  prototype", which has no such target. Give the chip sign-off its own bar.

### Documentation

**Summary**: As documentation for an implementer, the plan is largely
trustworthy: every cited line number for the doc-facing targets matches the
files exactly, and all six `research_status` producers/consumers in the skill
body are caught. The weakest area is documentation reconciliation across the
co-landing corpus: 0277, 0279, and parts of epic 0121 still assert
`research_status` and an explicit "never infer set progress from base `status`"
rule this change contradicts, yet the reconciliation is recorded only as
loosely-scoped prose. Two smaller template-consistency omissions round out the
findings.

**Strengths**:
- Cited line numbers for every documentation target are accurate.
- All six `research_status` sites in the skill are enumerated (grep confirms no
  seventh).
- The lifecycle-vocabulary decision is explicitly recorded and used
  consistently.
- The Phase 2 schema/TSV diff is shown column-for-column against the on-disk
  row.

**Findings**:
- **major** (high) — _Migration Notes; Phase 2 Manual Verification_: Co-land
  documentation reconciliation is under-specified and not verifiably owned —
  0277 (88/93/98/103/219), 0279 (32/52/57/73), and epic 0121 still assert
  `research_status`. Promote to tracked, checkbox-verified tasks with file/line
  references; retire the "never infer set progress" rule everywhere.
- **minor** (high) — _Phase 2, Change 2 (Template)_: The inline `# complete`
  comment on line 8 is left stale by the value change, and the vocab hint on the
  removed line disappears (it also listed only four of five states). Specify the
  comment treatment.
- **minor** (medium) — _Phase 2, Change 2 (body Status line)_: Removing body
  line 35 leaves the `## Status` section with no lifecycle value, contradicting
  the plan's wording. State whether a body line is kept or omitted.
- **minor** (medium) — _Phase 2, Change 3 (skill verbs)_: The collapse overloads
  `status` without disambiguating the manifest from sub-documents (e.g.
  `brief.md`). Keep the reference explicit ("the manifest's base `status`").
- **minor** (high) — _Key Discoveries_: The plan states "six files" but
  enumerates five (and only five exist). Correct the count and clarify template
  lines 8/35 are related edits, not `research_status` literals.
- **suggestion** (low) — _Phase 3 / Implementation Approach_: The doc-type
  registration surface is not captured as a reusable checklist; consider one as
  a follow-up so future additions (0280, 0281) don't re-derive the inventory.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-09-10

**Verdict:** REVISE

The revision resolved essentially all of the initial review's twelve majors — the
status-chip semantic/mechanism rework, the localStorage and indexer tests, the
slug guard, the doc fixes, and the Code Quality items all verify clean against the
tree, and the co-land line references are byte-exact. But two areas were only
partially closed, and the status-chip redesign introduced a fresh cluster of
majors: the dedicated lifecycle scale needs a doc-type-aware **dispatch seam**
that four lenses independently flagged as unspecified; its hue-132 ordinal ramp
collides with the app-wide "green = done" semantic; the two "complete the edit
list" fixes are each still missing sites; and the `id: <slug>`
dangling-reference scheme contradicts the shipped manifest template. Still REVISE
— but the remaining work is now a smaller, well-bounded set, most of it
mechanical.

### Previously Identified Issues

- 🟡 **Architecture / Code Quality / Usability**: Lifecycle states in the
  semantic error/warn palette — **Resolved**. Replaced by a dedicated hue-132
  ordinal `lifecycle-*` scale; no healthy state uses red/amber; the shared
  lexicon and fills are untouched.
- 🟡 **Code Quality / Test Coverage / Usability / Correctness**: Status-chip
  mechanism unrealisable (violet, indigo faint-token, 8% mix) — **Resolved**.
  Dedicated fills; exhaustive lifecycle `Record`; the indigo-unreachable problem
  is gone.
- 🟡 **Compatibility**: `cluster-via-label.ts` switch — **Resolved** (added to
  §2 with its test).
- 🟡 **Compatibility**: `template-tier.ts` glyph value — **Resolved** (added to
  §3 with `LibraryTemplatesIndex` tests).
- 🟡 **Compatibility**: `data-stage`/`docType` test fixtures — **Partially
  resolved**. The §6 sweep covers most, but `Glyph.test.tsx` and three more
  `DevDesignSystem.tsx` sites (plus a wrong path in §2) are still missed.
- 🟡 **Correctness**: Phase 3 count-site enumeration incomplete — **Partially
  resolved**. `api_smoke.rs` and `compose_contract.rs` added, but the launcher
  `doctypes.golden` and two `config_read.rs` `== 13` assertions are still
  unlisted.
- 🟡 **Correctness / Test Coverage**: Dangling-reference `(type, id)` conflation
  — **Partially resolved**. The mechanism and negative control are now correct,
  but the `id: <slug>` scheme contradicts the shipped template (`id:
  "{filename-stem}"` = `"manifest"`), and the concrete test/corpus is still
  unnamed.
- 🟡 **Documentation / Compatibility / Architecture**: 0277 co-land unenforceable
  / under-specified — **Resolved** (checkboxes with byte-exact line refs;
  residual graph risk acknowledged and, per Compatibility, largely mitigated —
  0277-alone stays self-consistent). One stale checkbox introduced (see New
  Issues).
- 🟡 **Test Coverage**: localStorage under-tested — **Resolved** (migration +
  idempotency tests).
- 🟡 **Test Coverage**: Indexer test weaker than precedent — **Resolved** (≥2
  real sets + missing-manifest skip).
- 🟡 **Correctness**: Phase-ordering rationale inconsistent — **Resolved** (the
  validated corpus-cli / real-corpus fixtures enforce the order; `kind: report`
  confined to the unvalidated visualiser fixtures).
- 🟡 **Usability / Compatibility**: URL slug-drop — **Resolved** as an explicit
  accepted decision (Usability still notes a slug-preserving alias is more
  forgiving; recorded).
- 🔵 **Code Quality** (all four minors/suggestions): comments removed,
  exhaustiveness test de-magicked, full predicate-profile mirror, no-comment call
  on the `Research` divergence — **Resolved**.
- 🔵 **Documentation** (template `# complete`, body Status section, skill
  `status` overload, "six files" → five) — **Resolved**; one wording slip
  introduced (see New Issues).

### New Issues Introduced

- 🟡 **Architecture / Code Quality / Usability / Test Coverage** (4 lenses):
  **Status-chip dispatch seam unspecified.** `statusToVariant` is
  doc-type-agnostic (consumed by `LibraryTypeView.tsx:286`, `StatusBadge.tsx:22`)
  and `complete` belongs to both the shared `GREEN` set and the lifecycle vocab,
  so §6's value-only map never routes topic-research cards to the `lifecycle-*`
  variants — a `synthesised` card renders grey and `complete` shared-green while
  the isolated map test passes. Needs a doc-type-aware `lifecycleToVariant`
  invoked only when `type === "topic-research"`, the `ChipVariant` union edit in
  `Chip.tsx`, and a render-level test.
- 🟡 **Usability**: **Green-ramp collides with "green = done".** The ordinal ramp
  sits on hue 132 (green), so a mid-flight `researching` chip reads as complete
  against the app-wide green=done convention; a lone chip has no reference ramp;
  `briefed`-as-faintest reads as inactive. A design-encoding decision.
- 🔴 **Correctness**: **`id: <slug>` contradicts the shipped manifest template.**
  `templates/topic-research-manifest.md:3` hard-codes `id: "{filename-stem}"` =
  `"manifest"`, so every real set keys as `topic-research:manifest` — real
  references dangle and multi-set corpora collide on `DuplicateId`. The template
  `id:` (and 0277's writer, at co-land) must move to the subject slug.
- 🔴 **Correctness**: **Count-site list still incomplete.** The `DOC_TYPES` row
  also breaks the launcher `doctypes.golden` (13 rows), `config_read.rs:743`,
  `config_read.rs:773` (`== 13`, named `...thirteen_rows`), and leaves stale
  "13" comments (`compose.rs:122`, `config/src/paths.rs:1`, launcher
  `cli.rs:217`).
- 🟡 **Test Coverage**: **Phase 2 `research_status`-rejection test can't reach
  green.** The validator has no unknown-key violation (`check_obsolete_legacy_keys`
  fires only for `["ticket", "ticket_id"]`), so a stray key is silently ignored.
  Add `research_status` to `OBSOLETE_LEGACY_KEYS`, or switch to a positive
  "template/fixture omit it" assertion.
- 🟡 **Compatibility**: **Two more missed rename consumers.** `Glyph.test.tsx`
  (`docType="research"` at :64,:72 — compile break; `var(--ac-doc-research)` at
  :74 — assertion break) and three silent `DevDesignSystem.tsx` present-token
  sites (:757,:964,:996) beyond :732; §2's path citation is wrong
  (`src/dev/…` vs `src/components/DevDesignSystem/…`).
- 🟡 **Documentation**: **Epic-0121 co-land checkbox is stale.** 0121 already
  reflects the collapse (its live contract `:101-112`, drafting note `:214`), and
  the "never infer set progress" rule it names actually lives in 0277 (`:219`).
  Also, the 0277 checkbox omits `0277:224` ("complete once the file exists").
- 🔵 **Code Quality**: No Rust↔TS parity guard on the lifecycle vocabulary (unlike
  `pipeline-step-parity`), so a future `status_vocab` change silently drops the TS
  map to grey.
- 🔵 **Code Quality / Test Coverage**: The `color-mix` resolver may be unnecessary
  if the lifecycle fills are plain hex tokens (only sRGB→HSL is needed); either
  way the new colour helpers are load-bearing but self-untested (add golden-value
  tests).
- 🔵 **Compatibility**: No `topic-research` `STEM_TO_GLYPH` stem, so the new
  type's own templates resolve to no glyph (or the `codebase-research` glyph).
- 🔵 **Architecture / Usability**: The VR-filename↔`DOC_TYPE_KEYS` guard is a soft
  "Consider", not a committed in-loop check; and the registration checklist is
  deferred past its first consumers (0280/0281).
- 🔵 **Documentation**: Key Discoveries mislabels template line 35 as "the body
  heading" (it is a body bullet); the template `# complete` comment fix is left as
  an undecided either/or.
- 🔵 **Usability**: A five-step single-hue lightness ramp is hard to read as a
  lone chip; the acceptance bar checks a side-by-side swatch, not a lone chip;
  "Topic research" remains a generic label.

### Assessment

Needs another iteration, but a bounded one. The prior review's structural
findings are genuinely closed; what remains is a tight set, most of it mechanical
and safe to apply directly: the dispatch seam (specify doc-type-aware dispatch +
`ChipVariant` union edit + render test), the two incomplete edit lists (launcher
golden + `config_read`; `Glyph.test.tsx` + `DevDesignSystem` sites), the
`research_status`-rejection test fix, the stale 0121 checkbox, the missing
`STEM_TO_GLYPH` stem, and the doc slips. Two items are genuine decisions, not
edits: the chip **encoding** (an ordinal ramp on green collides with green=done —
move it off green, keep it and lean on the label, or revisit the five-distinct-
non-grey AC), and the `id:` scheme (moving the manifest template's `id:` to the
subject slug is the right direction but is cross-cutting to 0277's writer). Both
warrant an author call before the next edit pass.

## Re-Review (Pass 3) — 2026-09-11

**Verdict:** REVISE

Pass 2's decisions were applied (chip ramp off green; `id:`→slug + 0277 writer;
all mechanical fixes). This pass verifies those hold — the `id:`→slug scheme is
correct end-to-end (`resolve_own_type_id` reads `id:` first; no clash with the
visualiser indexer), `OBSOLETE_LEGACY_KEYS` is the right rejection mechanism, the
wire-rename sweep is now near-exhaustive, and every doc line-citation is
byte-accurate. The new findings are narrower and concentrate in the two surfaces
the pass-2 edits most reworked: the **status-chip dispatch** and the several
**precise-mechanism claims** those edits added. Notably, four lenses converge on
the observation that the doc-type dispatch seam is the wrong shape — and that a
**value-only** map (let `complete` resolve to shared `green`, map only the four
unique in-flight states) dissolves it entirely.

### Previously Identified Issues (Pass 2)

- 🟡 Status-chip dispatch seam unspecified — **Resolved, then re-opened
  narrower**: the seam is now specified but incomplete (see New Issues — it only
  covers the card site, not the detail/`StatusBadge` or `FilterPill` sites).
- 🟡 Green-ramp collides with "green = done" — **Resolved** (ramp moved off green;
  Usability confirms).
- 🔴 `id: <slug>` contradicts the shipped template — **Resolved** (template +
  fixture + 0277 writer all move to the slug; Correctness/Compatibility verify the
  manifest case end-to-end).
- 🔴 Count-site list incomplete — **Partially resolved**: launcher golden +
  `config_read` added and verified, but the `cargo-public-api` golden is still
  unlisted (see New Issues).
- 🟡 Phase 2 `research_status`-rejection test unspecifiable — **Resolved**
  (`OBSOLETE_LEGACY_KEYS` raises `OBSOLETE-LEGACY-KEY`; verified sound and
  currently harmless).
- 🟡 Two more missed rename consumers — **Resolved** (`Glyph.test.tsx`,
  `DevDesignSystem` path + three sites verified against the tree).
- 🟡 Epic-0121 checkbox stale — **Resolved** (re-scoped; every line ref verified
  accurate).
- 🔵 Lifecycle vocab parity guard — **Partially resolved**: an assertion was
  added, but the cited precedent is a TS-only mirror, so it does not guard Rust
  drift as claimed (see New Issues).
- 🔵 `color-mix` resolver unnecessary / helpers self-untested — **Resolved**
  (dropped the resolver; sRGB→HSL helper gets golden tests).
- 🔵 Missing `STEM_TO_GLYPH` stem — **Resolved by intent, broken in mechanism**:
  the stem was added but cannot resolve the suffix-named templates (see New
  Issues).
- 🔵 VR guard soft / registration checklist deferred — **Resolved** (firmed to a
  committed in-loop test; checklist pulled before 0280/0281).
- 🔵 Doc slips (body heading, template comment) — **Resolved** (with two small new
  comment nits, see New Issues).

### New Issues Introduced

- 🟡 **Architecture / Code Quality / Test Coverage** (3 lenses): **Chip dispatch
  is incomplete and mis-located.** `type` is in scope at `LibraryTypeView` (card)
  but not at `StatusBadge`/`FrontmatterChips` (detail view) or `FilterPill:160` —
  all call `statusToVariant` with no type. As written, the manifest's chip renders
  grey/green on the detail view and in status facets, and the render test covers
  only the card. **Best fix (Architecture insight):** the seam exists *only*
  because `complete` overloads the shared `GREEN` set; the other four states are
  unique to the lifecycle vocab. Let `complete` resolve to shared `green` (correct
  for "done") and map the four in-flight states by value — a **value-only** map
  with no doc-type dispatch, which resolves every site at once and is open-closed
  for 0280/0281.
- 🟡 **Usability**: **A neutral-slate ramp reads as disabled/muted** for all
  states (low chroma is the conventional disabled tone). Pick a distinct non-green
  hue with chroma comfortably above the bare 15% floor, and have the acceptance
  bar check all five states read as first-class, not just `briefed`.
- 🔴 **Correctness**: **The `cargo-public-api` golden is an unlisted count
  consumer.** `cli/corpus/tests/fixtures/public-api.txt` pins `all() -> [Self; 14]`
  and `OBSOLETE_LEGACY_KEYS: [&str; 2]`, so Phase 2 and Phase 3 must regenerate it;
  and the Desired End State self-contradicts ("cargo-public-api does not shift"
  beside "all() is 15") — scope the "no shift" claim to the rename dimension only.
- 🟡 **Test Coverage**: **The dangling-reference test is bound to the wrong
  golden.** `the_committed_topic_research_set_validates_clean` runs `validate
  --file` from an empty tempdir, so `build_index` indexes nothing and a
  `relates_to` there would flag DANGLING. Specify a whole-corpus staged test
  (root `meta/research/topics/<slug>/`, no `--file`) instead, with the negative
  control staging the referencing doc without the set.
- 🟡 **Test Coverage**: **The Rust↔TS lifecycle-vocab "parity" assertion isn't a
  real cross-language check** — the `pipeline-step-parity` precedent is a hardcoded
  TS literal linked to Rust only by a comment. Downgrade the claim to "TS-internal
  consistency", or make it genuine (Rust emits `status_vocab` into a committed
  fixture the TS reads + a Rust-side divergence test).
- 🟡 **Compatibility**: **The `STEM_TO_GLYPH` prefix stem cannot resolve the
  suffix-named templates.** `glyphKeyForTemplate` tests end-anchored suffixes, so
  `topic-research-manifest` yields candidates `manifest`/`research-manifest`/
  `topic-research-manifest` — never the added prefix `topic-research`. Register the
  full template names (or the doc-kind suffix stems) as exact keys.
- 🟡 **Compatibility**: **Set sub-documents keep colliding generic ids.** The
  `id:`→slug fix covers only the manifest; `brief.md`/`outline.md`/`synthesis.md`
  keep ids `brief`/`outline`/… so two real sets collide as `topic-research:brief`
  under `DuplicateId`. Extend the co-land contract so 0277 scopes sub-document ids
  to the set (e.g. `<slug>-brief`), and add a ≥2-set validated fixture.
- 🔵 **Compatibility**: `empty-descriptions.ts:42` `TYPE_COPY` value uses
  `DOC_TYPE_HUE.research` — a value-side reference that breaks the compile on the
  key rename; add it to the value-side move list. And the template `relates_to`
  comment (`:17`) still shows the numeric `NNNN` form — update to `<slug>`.
- 🔵 **Correctness**: The stale-"13"-comment citation `config/src/paths.rs:1` is
  wrong; the real one is `cli/launcher/src/config_command/core/paths.rs:1`.
- 🔵 **Test Coverage**: The VR baseline-filename in-loop test must target
  `DOC_TYPE_KEYS.filter(isPhysicalDocTypeKey)` (virtual keys have no baselines).
- 🔵 **Documentation**: The template `id:` line's `# filename without .md` comment
  is left stale by the slug change; a leftover "four-state hint moves" clause
  contradicts the decided five-state hint; and the `id: <slug>` convention is not
  yet recorded in the durable 0121 contract (add a co-land line).

### Assessment

The plan is structurally settled and converging. Every core finding from passes
1–2 is closed; pass 3's new issues are narrower and cluster in the two surfaces
the pass-2 edits most reworked — the chip dispatch and the precise-mechanism
claims those edits introduced — with several being incompletenesses in my own
pass-2 edits (the dispatch's second/third render sites, `STEM_TO_GLYPH`'s lookup
shape, the dangling-ref golden, the parity guard, `public-api.txt`, sub-document
ids). That pattern signals diminishing returns from further full re-review passes:
the design is right, and what remains is precise test/registration wiring that the
implementation phase (and `validate-plan`) will surface as red tests.

One genuine design decision remains — the **value-only** chip map (let `complete`
= shared green, drop the doc-type dispatch), which four lenses point to and which
dissolves the dispatch cluster in one move. The rest are concrete, bounded edits:
`public-api.txt` regeneration + the Desired End State scope fix; the whole-corpus
dangling-ref test; the parity-claim honesty fix; `STEM_TO_GLYPH` exact keys; the
sub-document id co-land scope; the non-disabled hue guidance; and the small
Compatibility/Documentation nits. Recommend applying those and finalising, rather
than a fourth full re-review.

## Re-Review (Pass 4) — 2026-09-11

**Verdict:** COMMENT

The plan has converged. Pass 3's decisions were applied — the chip dispatch is
centralised in a single `chipVariantFor(type, status)` selector over a total,
compile-enforced `Record<DocTypeKey, "lifecycle" | "semantic">` routed at all
three chip sites; the ramp hue moved off near-grey slate; the `cargo-public-api`
golden, whole-corpus dangling-ref test, genuine Rust↔TS drift guard,
`STEM_TO_GLYPH` exact keys, and `id:`→slug (manifest + set-scoped sub-documents)
are all in. This pass verifies those hold against the tree (every count/arity,
line-citation, and lookup-shape claim checks out) and drops from **REVISE** to
**COMMENT**: one loud, test-guarded major and a set of bounded minors, none
structural.

### Previously Identified Issues (Pass 3)

- 🟡 Chip dispatch incomplete/mis-located (3 lenses) — **Resolved**: one
  `chipVariantFor` selector over a total classifier `Record`, routed at card,
  detail, and facet; render tests at each site.
- 🟡 Neutral-slate reads as disabled — **Resolved**: guidance mandates a distinct,
  adequately-chromatic non-green hue; sign-off checks all five states.
- 🔴 `cargo-public-api` golden unlisted — **Resolved**: Phase 2 (`[&str; 2→3]`) and
  Phase 3 (`[Self; 14→15]` + variant) regeneration listed; Desired End State
  scoped; verified against the golden's actual lines.
- 🟡 Dangling-ref bound to the wrong golden — **Resolved**: re-based on a
  whole-corpus staged test with a negative control; verified feasible.
- 🟡 Parity assertion not a real cross-language check — **Resolved**: genuine
  Rust-emit-into-fixture mechanism (`frontmatter print-schema` exists), committed
  in Success Criteria.
- 🟡 `STEM_TO_GLYPH` prefix stem unreachable — **Resolved**: full template names
  registered as exact keys; verified against the suffix-first lookup.
- 🟡 Sub-document id collisions — **Resolved (contract)**: 0277 writer scopes
  sub-doc ids to the set; recorded in the co-land checkbox (fixture deliverable
  tightened this pass — see New Issues).
- 🔵 Value-side `empty-descriptions.ts:42`, stale-comment mis-cite, VR-filter
  subset, template comment nits, id=slug in 0121 — **Resolved** (with two small
  residuals, below).

### New Issues Introduced / Residual

- 🟡 **Correctness** (major, high — loud/test-guarded): `BigGlyph.test.tsx:69`
  hard-codes `Object.keys(BIG_GLYPHS).length === 14`; Phase 3 §5 takes it to 15.
  Unlisted count consumer — **now added** to §5.
- 🔵 **Compatibility**: a second value-side dot-access, `DOC_TYPE_COLOR_VAR`'s
  `var(--${DOC_TYPE_TOKEN_KEY.research})` (`Glyph.constants.ts:49`), needs the
  bracket-access move — **now added** to the Phase 1 value-side callout.
- 🔵 **Test Coverage / Compatibility**: the ≥2-set `DuplicateId` guard was named
  only in Testing Strategy; **now given a concrete Phase 2 §4 deliverable**
  (re-key sub-docs to `<slug>-brief`, add a second validated set).
- 🔵 **Architecture / Code Quality**: the dispatch threaded a raw `DocTypeKey`
  through three generic presenters. **Refined** to pass a resolved
  `variant?: ChipVariant` override from the type-aware parent (presenters stay
  doc-type-agnostic), with the `FilterPill` single-type precondition recorded.
- 🔵 **Code Quality**: `lifecycleToVariant` was a bare `Record`; **refined** to
  mirror `statusToVariant`'s normalise + `neutral` fallback.
- 🔵 **Test Coverage**: Change 6 kept a "TS-internal fallback" clause the Success
  Criteria forbid — **removed**; the genuine mechanism is now the single
  requirement.
- 🔵 **Documentation**: the id=slug reconciliation was only in the 0277 checkbox;
  **added** to the 0121 checkbox too.
- 🔵 **Usability**: the presenter override's default is a future-site footgun;
  **added** a note to the registration checklist that new lifecycle sites must
  pass the resolved variant.

### Assessment

Implementation-ready. Four passes have driven the plan from twelve structural
majors to a single loud, test-guarded count bump and a handful of interface and
fixture refinements — all now applied. The remaining precision (the exact
non-green hue, the `frontmatter print-schema` fixture wiring, the second corpus-cli
set) is the kind `implement-plan` executes and `validate-plan` confirms as green
tests, not plan-level design work. No further full re-review is warranted; the
plan is ready for implementation.

---
*Re-reviews (passes 2–4) generated by /accelerator:review-plan*

## Approval — 2026-09-11

**Verdict:** APPROVE

Approved by Toby Clemson after pass 4 converged to COMMENT with all findings
applied. The pass-4 COMMENT residuals were the loud, test-guarded
`BigGlyph.test.tsx` count bump and a set of interface/fixture refinements, all
resolved in the plan. The remaining precision (the exact non-green chip hue at
design sign-off, the `frontmatter print-schema` fixture wiring, the second
corpus-cli set) is implementation-time work verified by `validate-plan`, not
plan-level design. The plan is marked `ready` for `implement-plan`.
