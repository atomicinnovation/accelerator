---
type: "work-item"
id: "0278"
title: "Topic-Research Visualiser Doc Type and Indexer"
date: "2026-09-08T11:42:24+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "ready"
kind: "story"
priority: "high"
parent: "work-item:0121"
blocks: ["work-item:0279", "work-item:0280", "work-item:0281", "work-item:0284"]
relates_to: ["work-item:0277"]
external_id: "PP-862"
tags: ["research", "visualiser", "infrastructure"]
last_updated: "2026-09-10T20:48:55+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0278: Topic-Research Visualiser Doc Type and Indexer

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

As an Accelerator user, I want my topic-research sets to appear as cards in the
visualiser library that open to their manifest, so that I can find and read the
topic-research sets the engine (0277) produces (sub-document navigation within a
set is 0284). This story registers the umbrella `topic-research` doc type in the
visualiser, indexes each research set as one library entry via its
`manifest.md`, and collapses the manifest's separate `research_status` field
onto its base `status` so the library card reports real progress. It also
renames the existing `research` doc type's wire key (its API/URL token) to
`codebase-research`, aligning that last surface with the `codebase-research` name
its typed-linkage and `type:` frontmatter already use (the `research_codebase`
config key and `meta/research/codebase/` directory keep their own names) — and
relabels its display name to "Codebase research", disambiguating it from the new
type. It is the visualiser half of epic Slice 1 and must co-land with the 
engine (0277) so the vertical demo — build a set, browse it — lands whole; the 
engine's artifacts are reader-observable only once this lands.

## Context

The engine (0277) writes contract-conforming artifacts to disk but registers no
visualiser doc type, so its sets are invisible until this story lands. The six
doc `kind` values (`manifest`, `brief`, `outline`, `finding`, `synthesis`,
`report`) are a frontmatter discriminator on one umbrella type, so this
registration cost is paid once; the set-level detail page (0284) supersedes the
shared flat `LibraryDocView` later, and this slice renders through that same
`LibraryDocView`.

Two facts from investigation reshape the earlier draft. First, the `DocTypeKey`
registry lives in the shared `cli/corpus/src/doc_type.rs` crate, not
`server/src/docs.rs` as the epic's notes state. Second, 0277's plan has already
landed the corpus/config scaffolding (`paths.research_topics`, templates,
linkage rules, the `(type, kind)` schema rows) on this branch — so this story
additionally absorbs a delta over 0277's landed code: collapsing the manifest's
`research_status` onto its base `status`, because the library card reads base
`status` and a manifest is otherwise `complete` from birth, which would make
every card read "complete".

The wire-key rename is the third pillar. Only the `wire_str()` token still reads
`research` — the linkage name, `type:` frontmatter, config key, and directory
already read `codebase-research` (the historical `m0004` migration moved them) —
so aligning it disambiguates the type from the new `topic-research` and removes
the one surface where the old name leaks into the API and URL.

## Requirements

Doc-type registration (`cli/corpus/src/doc_type.rs`):

- A `TopicResearch` `DocTypeKey` variant with `all()`, `label()`,
  `wire_str()`/`from_wire_str()` (wire `topic-research`), `config_path_key()` →
  `Some("research_topics")`, and `linkage_type_name()` → `Some("topic-research")`.
- The predicates mirror `DesignInventories` (the other nested-manifest Discover
  type) except: `nested_manifest_filename()` → `Some("manifest.md")` — a
  non-boolean method, not a predicate — and, among the booleans, not virtual and
  not in kanban. Mirror `DesignInventories`' predicate set exactly for every
  remaining boolean value rather than enumerating from memory.
- Every count/parity assertion bumped: the `[Self; 14]` array in `all()`, the
  `all()` exhaustiveness test, `parity.rs`'s hand-maintained row table,
  `server/tests/api_types.rs` (`len == 14`), and a `DOC_TYPES` catalogue row in
  `cli/config/src/catalogue.rs` with its `len` bump and the
  `doc_type_single_source.rs` cross-check (topic-research is linkage-bearing).

Indexing:

- Nested-manifest indexing flows from `nested_manifest_filename()`: the
  `file_driver.rs` `list` branch emits exactly one entry per
  `meta/research/topics/<slug>/` directory keyed on `manifest.md`, skipping
  dot-prefixed in-flight directories; `indexer.rs` `build_entry` derives the
  slug from the parent directory and the title from the manifest's `title`,
  falling back to the first H1, then the humanised directory name (the canonical
  `humanise_slug` title-cases each `-`-separated word, e.g.
  `model-context-protocol` → "Model Context Protocol").

Library placement and rendering:

- Discover-phase membership in `server/src/api/library.rs` (`PHASES`) plus test
  updates.
- Frontend registry parity in `frontend/src/api/types.ts` (`DocTypeKey` union,
  `DOC_TYPE_KEYS`, labels) and every compiler-enforced `Record<DocTypeKey, …>`
  map (`DOC_TYPE_HUE`, `TYPE_COPY`, `EMPTY_TYPE_PLURALS`, `DOC_TYPE_TOKEN_KEY`,
  `DOC_TYPE_COLOR_VAR`, `ICON_COMPONENTS`, `BIG_GLYPHS`, `DETAIL_ROUTE_SLUGS`,
  `DETAIL_ROUTE_RENDERS_ARTICLE`, `DOC_TYPE_LABELS`/`_SINGULAR`).
- A small glyph component (layered sheets under a dossier cover, strands
  converging into one node; 24×24, single-colour `currentColor` stroke, weight
  1.25) and a big-glyph hero (the gathered-dossier illustration), both matching
  the prototype's `TYPE_ICONS['topic-research']` (`src/ui.jsx`) and
  `big-glyphs.jsx`. The prototype's `TYPE_META` short code (`TRS`) is metadata
  the current app's glyph does not render, so no short-code label ships and the
  `research` relabel touches no short code.
- Light colour tokens derived at hue 132, foreground lightness ~34%
  (`rgb(28,146,51)`, per the prototype's `TYPE_META['topic-research']`) +
  `global.css` mirror + framed-background CSS rule (dark tokens are fixed
  `#ffffff`/`#1d2030`); the light pair clears ≥3:1 contrast and the hue sits ≥15°
  from `codebase-research` (28), `design-inventories` (185), and `design-gaps` (95) —
  104°/53°/37° respectively.
- A status-chip colour mapping giving each of the five lifecycle states
  (`briefed`, `outlined`, `researching`, `synthesised`, `complete`) a distinct,
  non-grey tone (chip fill HSL saturation ≥ 15%) at ≥3:1 text contrast, verified
  by a direct unit test across all five states. This deliberately deviates from
  the prototype's `StatusBadge` (neutral/grey for `briefed`, indigo reused
  across states); the deviation is design-signed-off per the design criterion.
- Clicking a card opens the set's `manifest.md` through the shared
  `LibraryDocView` — no `primary`-pointer resolution and no set-level
  navigation (both 0284).

Rename the `research` doc-type wire key to `codebase-research`:

- The rename aligns the one API/URL surface still carrying the bare token
  `research` with the `codebase-research` name the linkage vocabulary, on-disk
  `type:` frontmatter, catalogue `DOC_TYPES` row, and schema rows already use;
  the config path key (`research_codebase`) and its `meta/research/codebase/`
  directory bind the same corpus but keep their own names. It is a wire-token
  change, not an on-disk one.
- Rust: `DocTypeKey::Research::wire_str()` → `"codebase-research"` in
  `cli/corpus/src/doc_type.rs` (`from_wire_str()` derives from `wire_str()` and
  auto-follows). Keep the Rust variant identifier `Research` — renaming the
  identifier would shift the `cargo-public-api` snapshot and every
  `DocTypeKey::Research` call site, whereas a pure wire-value change shifts
  neither. Update the `parity.rs` row so its wire field reads
  `"codebase-research"` (the config field `research_codebase` is unchanged).
- Pipeline `completeness.present` vocabulary — move in lockstep: the
  hand-maintained `STAGE_PUSH_ORDER` token in `cli/corpus/src/cluster.rs`, its
  server test mirror in `clusters.rs`, and the frontend `CANONICAL_PRESENT_ORDER`
  in `pipeline-step-parity.test.ts` all carry a literal `research` that must
  become `codebase-research`, because `LIFECYCLE_PIPELINE_STEPS[…].docType` is a
  `DocTypeKey` (so it becomes `codebase-research`) and the cross-language parity
  test compares the two.
- Frontend registry: the `DocTypeKey` union member, `DOC_TYPE_KEYS`, and every
  compiler-enforced `Record<DocTypeKey, …>` map key enumerated above move
  `research` → `codebase-research` (`frontend/src/api/types.ts` plus the maps in
  `styles/tokens.ts`, `routes/library/empty-descriptions.ts`,
  `components/Glyph/Glyph.constants.ts`, `Glyph.tsx`, `BigGlyph.tsx`, and the
  `tests/lib/detail-route-slugs.ts` fixtures). The glyph component, the hue value
  (28), and the detail-route slug value are unchanged — only the map keys move.
- CSS tokens are derived from the key at runtime, so they move with it: the
  `--ac-stage-<key>` token (consumed as `var(--ac-stage-${docType})`) becomes
  `--ac-stage-codebase-research`, and the `data-doc-type`/`data-stage` attribute
  values become `codebase-research`. Rename `--ac-doc-<key>` and
  `--ac-doc-bg-<key>` to match, keeping the key-derived contrast/token guards in
  `global.test.ts` green (`global.css` + `tokens.ts` light/dark blocks).
- URL surface: the doc-type route segment is the wire key itself
  (`router.ts` validates it via `isDocTypeKey`, with no separate slug map), so
  the public URL changes from `/library/research/<slug>` to
  `/library/codebase-research/<slug>`; `/library/research/...` no longer resolves.
- Last-seen continuity: a one-shot `localStorage` key rewrite
  `research` → `codebase-research` in `use-unseen-doc-types.ts` (mirroring the
  `prs` → `pr-descriptions` precedent) carries each viewer's "seen" state across
  the rename; without it the renamed type reads as all-unseen once. This is a
  frontend-storage concern, not an `accelerator migrate` migration.
- No migration: on-disk `type: codebase-research` documents validate unchanged,
  the `meta/research/codebase/` directory and config key `research_codebase` are
  untouched, and no bare `research:<slug>` linkage references exist — so no
  `accelerator migrate` migration, directory move, or frontmatter rewrite ships.
- Display label: `research`'s display name becomes "Codebase research" (from
  "Research"), and the new type's is "Topic research", disambiguating the two.
  The doc-type count is unchanged (14) — a rename adds and removes no variant.

The 0277-delta — collapse `research_status` → base `status`:

- `schema.rs` manifest row: `status_vocab` →
  `["briefed","outlined","researching","synthesised","complete"]`; remove
  `research_status` from `extras` (keep `slug`, `round_count`, `finding_count`,
  `primary`).
- `templates/topic-research-manifest.md`: base `status` becomes the lifecycle
  (initial `briefed`); remove the `research_status` line.
- `skills/research/research-topic/SKILL.md`: the `brief`/`outline`/`conduct`/
  `synthesise` transitions and the `outline`/`conduct` preconditions read and
  write base `status`, not `research_status`.
- `cli/corpus-cli/tests/fixtures/topic-research-set/manifest.md`: base `status`
  carries a lifecycle value (`synthesised`, mirroring the visualiser fixture);
  `research_status` removed.

Fixtures, prototype, and gates:

- A checked-in visualiser fixture set at
  `cli/visualiser/server/tests/fixtures/meta/research/topics/<slug>/manifest.md`
  (base `status: synthesised` so the card chip is a real in-flight value), wired
  into `DETAIL_ROUTE_SLUGS`/`DETAIL_ROUTE_RENDERS_ARTICLE` so the native
  `fixture-coverage.spec.ts` passes. The fixture set also contains a
  `findings/<nn>-<slug>.md` document, a `reports/<slug>.md` document, and a
  dot-prefixed in-flight sibling directory, so the indexer test positively
  confirms exactly one entry is emitted (sub-documents not separately indexed)
  and the in-flight directory is skipped — rather than passing vacuously as a
  manifest-only fixture would.
- Two further checked-in fixture manifests (or documented `build_entry` test
  cases) exercise the title-fallback branches: one with no `title` but a leading
  `# <H1>` (asserting the H1 fallback) and one with neither `title` nor an H1
  (asserting the humanised directory name), so all three title-derivation
  branches are positively covered rather than only the `title`-present path.
- A checked-in fixture document carrying `relates_to: ["topic-research:<slug>"]`
  targeting the fixture set's slug, so the whole-corpus dangling-reference check
  has a concrete referencing document and confirms the reference resolves once
  the set is indexed.
- The Claude Design prototype has been updated to cover topic-research
  (`2026-09-10-174311`, see Technical Notes and References); the implemented
  glyph, big-glyph, and colour match that prototype.
- Visual-regression (VR) baselines regenerated and committed via the pinned
  Docker/Linux harness (10 PNGs for topic-research: 8 glyph-showcase across 4
  sizes × 2 themes, 2 big-glyph across themes). The `research` rename also renames
  its 10 existing baseline PNGs `research-*` → `codebase-research-*` (pixels
  identical — the spec names them from `DOC_TYPE_KEYS`); `mise run check` green
  and the Docker VR spec green.

## Acceptance Criteria

- [ ] Given the checked-in topic-research fixture set, when the library loads,
      then the `topic-research` doc type appears under the Discover phase with
      its glyph and framed background, and the Docker/Linux VR baselines pass —
      the 8 glyph-showcase baselines (4 sizes × 2 themes) and the 2 big-glyph
      baselines are committed and green, so the coverage matrix, not just the
      pass state, is verified.
- [ ] Given the library sidebar/menu, when it lists the Discover-phase types,
      then the renamed type is labelled "Codebase research" (not "Research") and
      the new type is labelled "Topic research"; the renamed type's glyph and
      colour (hue 28) are visually unchanged.
- [ ] Given the rename, the `research` doc type's wire token is
      `codebase-research` end-to-end: `wire_str()`/`from_wire_str()` round-trip
      it, `parity.rs` pins it, and the frontend `DocTypeKey` union plus every
      `Record<DocTypeKey, …>` map key use it. The Rust variant identifier stays
      `Research`, the doc-type count is unchanged (14), `cargo-public-api` does
      not shift, and `mise run check` passes end-to-end (including the
      `pipeline-step-parity` cross-language check, whose `research` token moved to
      `codebase-research` in lockstep).
- [ ] Given the renamed wire key, the library route resolves
      `/library/codebase-research/<slug>` (and its detail route) while
      `/library/research/...` no longer validates; the 10 existing glyph and
      big-glyph VR baselines are renamed `research-*` → `codebase-research-*`
      (pixels identical) and pass the pinned Docker/Linux harness.
- [ ] Given the rename touches no persisted state, no `accelerator migrate`
      migration ships: on-disk `type: codebase-research` documents validate
      unchanged, and the `meta/research/codebase/` directory and config key
      `research_codebase` are untouched. Given a viewer whose stored last-seen set
      contains a `research` entry, when the app next loads, a one-shot
      `localStorage` rewrite re-keys it to `codebase-research` with its seen value
      preserved, so the renamed type does not surface as newly-unseen.
- [ ] Given a `meta/research/topics/<slug>/` directory, when the indexer runs,
      then it emits exactly one library entry keyed on `manifest.md`; individual
      findings and reports are not separately indexed, and dot-prefixed
      in-flight directories are skipped.
- [ ] Given the fixture manifest with `title: <X>`, when its card renders, then
      the card title is `<X>` and the entry's slug equals the parent directory
      name. Given a fixture manifest with no `title` but a leading `# <H1>`, the
      card title is `<H1>`; given a fixture manifest with neither `title` nor an
      H1, the card title is the humanised directory name — the canonical
      `humanise_slug` title-cases each `-`-separated word (e.g.
      `model-context-protocol` → "Model Context Protocol"). A checked-in fixture
      exercises each fallback branch, so all three title-derivation paths are
      asserted.
- [ ] Given a manifest whose base `status` is a lifecycle value (e.g.
      `synthesised`), when its card renders, then the status chip shows that
      value — not "complete". A distinct chip colour is mapped for each of the
      five lifecycle states (`briefed`, `outlined`, `researching`,
      `synthesised`, `complete`) — none default grey, each chip fill carrying
      HSL saturation ≥ 15% — and every state's chip text meets ≥3:1 contrast
      against its chip fill. The five-way status→chip mapping is unit-tested
      directly across all five states (distinctness, the ≥15% non-grey bound,
      and ≥3:1 text contrast), not only via the rendered `synthesised` fixture.
      This mapping deviates from
      the prototype (which uses neutral for `briefed` and reuses indigo); the
      deviation is covered by the design sign-off below.
- [ ] Given a topic-research card, when it is clicked, then the set's
      `manifest.md` opens through the shared `LibraryDocView`, with no set-level
      sub-document navigation.
- [ ] Given the collapse, `schema.rs`'s manifest row admits the five lifecycle
      states as base `status` and no longer lists `research_status`; the
      template, the `research-topic` skill verbs, and the corpus-cli fixture
      write base `status`; and `accelerator corpus frontmatter validate` passes
      for a manifest with base `status: briefed` (and `synthesised`).
- [ ] Given the new variant, the Rust registry compiles with every count/parity
      assertion updated, the frontend `Record<DocTypeKey, …>` maps are complete,
      and `mise run check` passes end-to-end.
- [ ] Given the topic-research `TYPE_COPY` entry, its purpose, when, and examples
      strings match the values pinned in Technical Notes (purpose "Subject
      dossiers — brief, outline, findings and synthesis accreted into one citable
      set."), asserted by a test — the compiler guarantees the map key exists, not
      its content.
- [ ] Given the `2026-09-10-174311` prototype covers topic-research across
      glyph, big-glyph, colour, the Discover card, and the manifest detail view,
      the implemented glyph component, big-glyph hero, colour tokens, and status-
      chip mapping (including its deliberate deviation from the prototype's chip
      tones) pass a human design-review sign-off against that prototype, recorded
      as a PR approval or a work-item note naming the reviewer. The "match"
      judgement is explicitly human-judged (a subjective visual-fidelity call,
      per epic 0121's output-quality-gate precedent), not a mechanical pixel
      diff — the VR baselines guard drift from the implementation, not fidelity
      to the prototype.

- [ ] Given the implemented light colour tokens derived at hue 132, foreground
      lightness ~34% (`rgb(28,146,51)`) per the prototype, the foreground/
      background pair measures ≥3:1 contrast against the page background, and the
      hue sits ≥15° from `codebase-research` (28), `design-inventories` (185), and
      `design-gaps` (95) — 104°/53°/37° respectively.
- [ ] Given the co-landed indexer registers the checked-in fixture set, the
      checked-in fixture document that carries
      `relates_to: ["topic-research:<slug>"]` targeting that set's slug passes
      whole-corpus dangling-reference integrity (the reference is no longer
      dangling) — the verification 0277 defers to this co-land.

## Open Questions

- None blocking. Glyph/big-glyph geometry and the colour hue are now pinned by
  the `2026-09-10-174311` prototype (hue 132, `rgb(28,146,51)`); the ≥3:1
  contrast and ≥15° hue-separation constraints are met by construction.

## Dependencies

- Blocked by: 0277 — recorded canonically on 0277's `blocks`; this story's
  indexer keys on the `manifest.md` the engine writes.
- Co-land: must merge together with 0277 so the vertical demo is not lost;
  recorded machine-readably as `relates_to: work-item:0277` (a reciprocal
  `blocked_by` is omitted to avoid a block cycle, matching 0277's decision). The
  block graph cannot express the simultaneity without a cycle, so the co-land is
  enforced out-of-band — a shared merge train or mutual PR link — rather than by
  the graph alone, which otherwise permits 0277 to merge without 0278.
- Co-land reconciliation: 0277's own acceptance criteria and artifact-shape spec
  still assert `research_status`, which this collapse supersedes; they must move
  to base `status` in lockstep at co-land, alongside the epic-0121 contract
  update already recorded as a follow-up. Otherwise verifying 0277 against its
  published criteria contradicts the behaviour this delta ships.
- Blocks: 0279 — Slice 2's `finalise`, reopen-regression and count-syncing build
  on the base-`status` lifecycle and the `complete` vocab this story provisions
  via the `research_status` collapse, so 0279 must not land before this delta.
  Recorded on this story's `blocks`.
- Blocks: 0284 — the set-level detail page builds on this umbrella type and
  nested indexing.
- This story modifies corpus/skill code that 0277's plan landed on this branch
  (the `research_status` collapse).
- Shared downstream: the umbrella `topic-research` doc-type registration (Rust +
  frontend) is a shared artefact later slices also consume — the academic-sources
  slice (0280) renders reputation tiers and the consumption slice (0281) renders
  the `report` kind, both through this frontend registration. Both are now
  recorded directly on this story's `blocks` rather than relying on the
  transitive-through-0277 ordering, which would hold only while the co-land
  holds.
- External/tooling gates: the Claude Design prototype has already been updated to
  cover topic-research (`2026-09-10-174311`), so that pre-implementation gate is
  met. The remaining completion gates are a human design reviewer to sign off the
  implementation — including the status-chip deviation — against that prototype,
  and the pinned Docker/Linux visual-regression harness to regenerate and commit
  the baselines. If either is unavailable, the design-sign-off criterion and the
  VR-baseline criterion cannot be met.
- Out of scope (per the 2026-09-10 design-gap analysis): 0278 adds
  topic-research's accent, renames the `research` doc-type wire key to
  `codebase-research`, and relabels its display name. It does not re-derive
  `research`'s hue (the gap analysis moves it to `rgb(188,107,36)`; the rename
  keeps hue 28 and only moves the token's key/name),
  nor pull in the top-bar chrome, global search overlay, external-edit toast,
  kanban reshape, lifecycle hexchain, typed-doc-viewer, or routing changes — each
  is a separate design-convergence work item that analysis enumerates.

## Assumptions

- Collapsing `research_status` onto base `status` is safe: only `schema.rs`
  references it by literal in Rust, and the indexer parses frontmatter
  generically.
- The manifest `status_vocab` includes all five lifecycle states for
  forward-compatibility, though `complete` is only written once 0279's
  `finalise` lands.
- The design-inventories nested-manifest indexer precedent transfers directly.
- VR renders only in the pinned Docker/Linux harness; there is no per-platform
  baseline pair, so the epic's "darwin and linux" phrasing does not describe the
  harness.
- Adding topic-research's accent does not entail re-deriving the existing
  `research` accent; the gap analysis's per-doc-kind accent re-derivation
  (`research` → `rgb(188,107,36)`) belongs to the token-convergence work it
  sequences first.
- Renaming the `research` wire key to `codebase-research` needs no `meta/`
  migration: the `type:` frontmatter, the `meta/research/codebase/` directory,
  and every typed-linkage reference already use `codebase-research` (bound to the
  linkage name and the config key `research_codebase`, not the wire key). Only
  the API/URL token and its frontend consumers move; the Rust variant identifier
  `Research`, the config key, and the doc-type count stay put.

## Technical Notes

- Registry surface: enum + method arms + the six predicates in
  `cli/corpus/src/doc_type.rs`; presentation via `server/src/doc_type_view.rs`
  (iterates `all()`, no per-variant edit); `PHASES` in `api/library.rs`.
  Rust↔TS parity is hand-synced (no codegen) — each side guards itself
  (`parity.rs`, `global.test.ts`).
- `research` rename mechanics: `DocTypeKey::Research` carries three independent
  names — `wire_str()` ("research", the API/URL token), `linkage_type_name()`
  ("codebase-research", the `type:`/linkage value), and `config_path_key()`
  ("research_codebase", the directory binding). Only `wire_str()` still reads
  `research`; the linkage name and config key were moved to `codebase-research`
  by the historical `m0004` migration, so this story only aligns the wire token.
  Because `type:`, the directory, and linkage refs bind to the other two names,
  the rename persists nothing and needs no migration. The trap surfaces (leave
  unchanged): the camelCase `hasResearch` completeness/serde field, the
  `research` template stem, the server fixture directory/filenames, and the
  `DETAIL_ROUTE_SLUGS` slug value are separate namespaces, not the wire key.
- Card fields: title from manifest `title` (fallback: first H1, then humanised
  directory); status chip from base `status`; slug from parent directory; no
  counts (deferred to 0284).
- VR: `mise run test:e2e:visualiser:docker:update` regenerates baselines; native
  `mise run test` never runs VR. `fixture-coverage.spec.ts` iterates
  `DOC_TYPE_KEYS` and navigates `/library/<type>` and `/library/<type>/<slug>`,
  so the server fixture and `DETAIL_ROUTE_SLUGS` entry are mandatory or native
  specs fail.
- Prototype surface to match (`2026-09-10-174311`): `src/ui.jsx`
  (`TYPE_META['topic-research']` hue 132 / l 34, short `TRS`;
  `TYPE_ICONS['topic-research']`; the `StatusBadge` map), `src/big-glyphs.jsx`
  (topic-research hero), `src/type-copy.jsx` (`TYPE_COPY`), `src/data.jsx`
  (`DOC_TYPES`, `LIBRARY_GROUPS` discover order, manifest frontmatter shape —
  `kind: manifest`, `slug`, `round_count`, `finding_count`, `primary`),
  `src/view-library.jsx` (file aside → `manifest.md`). `TYPE_COPY` text —
  purpose: "Subject dossiers — brief, outline, findings and synthesis accreted
  into one citable set."; when: "Open one when an external subject will be
  researched iteratively and cited from later work."; examples: "Model Context
  Protocol", "Prompt caching economics".
- The prototype's chip labels use `gathering`/`monitoring`; this story keeps the
  engine/schema vocab `researching`/`complete` (0277 landed) and keeps five
  distinct non-grey chip tones rather than the prototype's neutral-`briefed`/
  indigo-reuse. Realigning the prototype's labels and tones is a later design
  touch-up, not this story.
- Claude Design prompt (already executed via the `design` skill against the prior
  `2026-05-21-015231` prototype, producing the `2026-09-10-174311` prototype —
  see References; retained here as the executed spec):

```text
Context — the problem this solves. Accelerator's topic-research skillset (the
parent epic) adds an iterative, citation-backed deep-research loop that builds a
durable knowledgebase on an arbitrary external subject. Each subject is a *set*
of documents — a brief, an outline, many findings, and a synthesis — accreted
under meta/research/topics/<slug>/, so research done once is citable and reused
rather than dying in a chat session. The research engine already writes these
sets to disk, but they are invisible in the visualiser: no document type
represents them, so there is nothing to browse. This story closes that gap — it
registers the umbrella `topic-research` document type so each set surfaces as one
library card that opens to its manifest, making the loop's output observable end
to end.

Update the Accelerator visualiser design prototype to add a new document type,
`topic-research`, modelled on the existing Discover-phase types (`research`,
`design-inventories`, `design-gaps`). Add it consistently across the prototype:

1. Registry (src/data.jsx): a `topic-research` entry in DOC_TYPES (label
   "Topic research") and in the `discover` group of LIBRARY_GROUPS, beside
   research / design-inventories / design-gaps.
2. Small glyph (24x24, single-colour currentColor stroke, matching the other
   glyphs' line weight): a research-SET motif — layered documents unified under
   one cover / a collection converging into one dossier — distinct from the
   plain `research` glyph, which reads as a single write-up.
3. Big-glyph hero (src/big-glyphs.jsx) in the same illustrative style.
4. Colour: a hue harmonious with the palette but clearly distinct from
   research (hue 28) and the design-* types; light foreground + light
   background-tint clearing >=3:1 contrast against the page background; dark
   theme uses the standard monochrome treatment.
5. TYPE_COPY (src/type-copy.jsx): purpose / when / examples for a subject
   dossier (brief -> outline -> findings -> synthesis under
   meta/research/topics/<slug>/).
6. Library card: one card per set, titled from the subject, with a status chip
   showing the research lifecycle — use a `synthesised` example so the chip is
   a lifecycle value, not "complete".
7. Manifest detail view: opening the card shows the manifest (set contents:
   brief, outline, findings, synthesis) through the shared doc view, with no
   bespoke set-level navigation (that is a later story).
8. Rename the existing `research` document type's display label from "Research"
   to "Codebase research" wherever the sidebar / library menu and type headings
   render it (its DOC_TYPES label and any menu/label maps), to disambiguate it
   from the new "Topic research" type. Only the display label changes — the
   `research` type's wire name, glyph, colour, and routing stay as they are.

Keep tokens, typography, and interaction patterns consistent with the existing
prototype. Output the updated prototype.
```

## Drafting Notes

- Enriched interactively on 2026-09-10 against epic 0121 and sibling 0277,
  grounded by two codebase investigations. Scope expanded from pure
  visualiser/indexer to also absorb the manifest-status collapse (the
  0277-delta) at the author's direction.
- Card status: chose to collapse `research_status` onto base `status` over
  redirecting the card to read `research_status` — one authoritative status
  field, and the card needs no per-type status logic.
- Click-target reverted to `manifest.md` (design-inventories precedent),
  dropping the earlier `primary`-pointer redirect, which moves to 0284.
- Corrected the epic's and the prior draft's "passing on darwin and linux" VR
  language: the harness renders only in a pinned Docker/Linux container with a
  single canonical baseline set.
- Enum predicates mirror `design-inventories` per the author's direction.
- Agreed follow-up: update epic 0121's artifact contract to match the collapse —
  the manifest frontmatter row, the `research_status` state-transition table,
  and the "never infer set progress from the manifest's base `status`" rule.
- Re-enriched on 2026-09-10 against the newer `2026-09-10-174311` prototype and
  the 2026-09-10 design-gap analysis. Colour (hue 132, `rgb(28,146,51)`), glyph
  motifs, and `TYPE_COPY` are now pinned from the prototype; the earlier
  `2026-05-21` prototype reference is superseded.
- Lifecycle vocabulary: kept the engine/schema vocab (`researching`/`complete`)
  over the prototype's `gathering`/`monitoring`, since 0277 landed the former and
  the terminal-state model (`complete` vs continuous `monitoring`) is a larger
  cross-cutting change; prototype-label realignment deferred.
- Status-chip colours: kept the five-distinct-non-grey requirement over the
  prototype's neutral-`briefed`/indigo-reuse mapping; the deviation is flagged
  for design sign-off.
- Scope boundary from the gap analysis: 0278 adds topic-research, renames the
  `research` wire key to `codebase-research`, and relabels its display name — not
  the research re-colour, top-bar chrome, global search, external-edit toast,
  kanban reshape, lifecycle hexchain, typed-doc-viewer, or the broader routing
  convergence.
- The `research` → `codebase-research` wire-key rename was added at the author's
  direction on 2026-09-10, expanding the earlier display-label-only relabel.
  Three codebase investigations established it is wire-token-only: the linkage
  name, `type:` frontmatter, config key, and directory already read
  `codebase-research` (via the historical `m0004` migration), so no `meta/`
  migration, directory move, or frontmatter/linkage rewrite is required. The
  change touches the API/URL token, the frontend `Record<DocTypeKey, …>` maps,
  key-derived CSS tokens, the pipeline `completeness.present` vocabulary (in
  lockstep across `cluster.rs`, `clusters.rs`, and `pipeline-step-parity.test.ts`),
  the 10 renamed VR baselines, and a `localStorage` last-seen rewrite.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 1, visualiser half);
  `meta/work/0277-single-round-web-research-engine.md` (engine sibling)
- Prototype:
  `meta/research/design-inventories/2026-09-10-174311-claude-design-prototype/prototype-full`
- Design gap:
  `meta/research/design-gaps/2026-09-10-current-app-vs-claude-design-prototype.md`
  (net-new "Topic research as a distinct corpus kind"; the per-doc-kind accent
  re-derivation it calls for is out of scope here)
- Internal: `cli/corpus/src/doc_type.rs`,
  `cli/corpus/src/frontmatter_validation/schema.rs`,
  `cli/corpus/src/cluster.rs`,
  `cli/visualiser/server/src/file_driver.rs`,
  `cli/visualiser/server/src/indexer.rs`,
  `cli/visualiser/server/src/api/library.rs`,
  `cli/visualiser/server/tests/parity.rs`,
  `cli/visualiser/frontend/src/api/types.ts`,
  `cli/visualiser/frontend/src/api/pipeline-step-parity.test.ts`,
  `cli/visualiser/frontend/src/api/use-unseen-doc-types.ts`,
  `cli/visualiser/frontend/src/router.ts`,
  `cli/visualiser/frontend/src/components/Glyph/`,
  `cli/migrate/src/migrations/m0004.rs` (historical research restructure),
  `templates/topic-research-manifest.md`,
  `skills/research/research-topic/SKILL.md`
