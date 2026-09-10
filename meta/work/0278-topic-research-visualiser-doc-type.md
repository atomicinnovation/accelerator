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
blocks: ["work-item:0279", "work-item:0284"]
relates_to: ["work-item:0277"]
external_id: "PP-862"
tags: ["research", "visualiser", "infrastructure"]
last_updated: "2026-09-10T11:33:49+00:00"
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
research sets the loop produces (sub-document navigation within a set is 0284). This story registers the umbrella `topic-research` doc type in the
visualiser, indexes each research set as one library entry via its
`manifest.md`, and collapses the manifest's separate `research_status` field
onto its base `status` so the library card reports real progress. It is the
visualiser half of epic Slice 1 and must co-land with the engine (0277) so the
vertical demo — build a set, browse it — lands whole; the engine's artifacts are
reader-observable only once this lands.

## Context

The engine (0277) writes contract-conforming artifacts to disk but registers no
visualiser doc type, so its sets are invisible until this story lands. The six
`kind` values are a frontmatter discriminator on one umbrella type, so this
registration cost is paid once; the set-level detail page (0284) supersedes the
shared flat `LibraryDocView` later, and this slice renders through that same
view.

Two facts from investigation reshape the earlier draft. First, the `DocTypeKey`
registry lives in the shared `cli/corpus/src/doc_type.rs` crate, not
`server/src/docs.rs` as the epic's notes state. Second, 0277's plan has already
landed the corpus/config scaffolding (`paths.research_topics`, templates,
linkage rules, the `(type, kind)` schema rows) on this branch — so this story
additionally absorbs a delta over 0277's landed code: collapsing the manifest's
`research_status` onto its base `status`, because the library card reads base
`status` and a manifest is otherwise `complete` from birth, which would make
every card read "complete".

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
  falling back to the first H1, then the humanised directory name.

Library placement and rendering:

- Discover-phase membership in `server/src/api/library.rs` (`PHASES`) plus test
  updates.
- Frontend registry parity in `frontend/src/api/types.ts` (`DocTypeKey` union,
  `DOC_TYPE_KEYS`, labels) and every compiler-enforced `Record<DocTypeKey, …>`
  map (`DOC_TYPE_HUE`, `TYPE_COPY`, `EMPTY_TYPE_PLURALS`, `DOC_TYPE_TOKEN_KEY`,
  `DOC_TYPE_COLOR_VAR`, `ICON_COMPONENTS`, `BIG_GLYPHS`, `DETAIL_ROUTE_SLUGS`,
  `DETAIL_ROUTE_RENDERS_ARTICLE`, `DOC_TYPE_LABELS`/`_SINGULAR`).
- A small glyph component and a big-glyph hero; light colour tokens +
  `global.css` mirror + framed-background CSS rule (dark tokens are fixed
  `#ffffff`/`#1d2030`); the light pair clears ≥3:1 contrast and the hue sits ≥15°
  from each of `research` (hue 28), `design-inventories`, and `design-gaps`.
- A status-chip colour mapping for the five lifecycle states so the card chip
  renders legibly rather than default-grey.
- Clicking a card opens the set's `manifest.md` through the shared
  `LibraryDocView` — no `primary`-pointer resolution and no set-level
  navigation (both 0284).

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
- A checked-in fixture document carrying `relates_to: ["topic-research:<slug>"]`
  targeting the fixture set's slug, so the whole-corpus dangling-reference check
  has a concrete referencing document and confirms the reference resolves once
  the set is indexed.
- A Claude Design prompt (see Technical Notes) that updates the existing
  prototype to cover topic-research; the implemented glyph, big-glyph, and
  colour match the updated prototype.
- Visual-regression (VR) baselines regenerated and committed via the pinned
  Docker/Linux harness (10 PNGs: 8 glyph-showcase across 4 sizes × 2 themes, 2
  big-glyph across themes); `mise run check` green and the Docker VR spec green.

## Acceptance Criteria

- [ ] Given the checked-in topic-research fixture set, when the library loads,
      then the `topic-research` doc type appears under the Discover phase with
      its glyph and framed background, and the Docker/Linux VR baselines pass.
- [ ] Given a `meta/research/topics/<slug>/` directory, when the indexer runs,
      then it emits exactly one library entry keyed on `manifest.md`; individual
      findings and reports are not separately indexed, and dot-prefixed
      in-flight directories are skipped.
- [ ] Given the fixture manifest with `title: <X>`, when its card renders, then
      the card title is `<X>` and the entry's slug equals the parent directory
      name; when `title` is absent, the title falls back to the first H1, then
      to the humanised directory name.
- [ ] Given a manifest whose base `status` is a lifecycle value (e.g.
      `synthesised`), when its card renders, then the status chip shows that
      value — not "complete". A distinct chip colour is mapped for each of the
      five lifecycle states (`briefed`, `outlined`, `researching`,
      `synthesised`, `complete`) — none default grey — and every state's chip
      text meets ≥3:1 contrast against its chip fill.
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
- [ ] The work item carries a Claude Design prompt that adds topic-research
      across glyph, big-glyph, colour, the Discover card, and the manifest
      detail view; the prototype is updated to cover topic-research; and the
      implemented glyph component, big-glyph hero, and colour tokens exist and
      pass a human design-review sign-off against the updated prototype, recorded
      as a PR approval or a work-item note naming the reviewer. The
      "match" judgement is explicitly human-judged (a subjective visual-fidelity
      call, per epic 0121's output-quality-gate precedent), not a mechanical
      pixel diff — the VR baselines guard drift from the implementation, not
      fidelity to the prototype.

- [ ] Given the implemented light colour tokens, the chosen foreground/background
      pair measures ≥3:1 contrast against the page background, and its hue sits
      ≥15° from each of `research`'s hue (28), `design-inventories`' hue, and
      `design-gaps`' hue.
- [ ] Given the co-landed indexer registers the checked-in fixture set, the
      checked-in fixture document that carries
      `relates_to: ["topic-research:<slug>"]` targeting that set's slug passes
      whole-corpus dangling-reference integrity (the reference is no longer
      dangling) — the verification 0277 defers to this co-land.

## Open Questions

- None blocking. Glyph and big-glyph geometry and the exact colour hue are
  intentionally deferred to the Claude Design prototype pass, constrained by the
  ≥3:1 contrast rule and the ≥15° hue separation from `research` (28),
  `design-inventories`, and `design-gaps`.

## Dependencies

- Blocked by: 0277 — recorded canonically on 0277's `blocks`; this story's
  indexer keys on the `manifest.md` the engine writes.
- Co-land: must merge together with 0277 so the vertical demo is not lost;
  recorded machine-readably as `relates_to: work-item:0277` (a reciprocal
  `blocked_by` is omitted to avoid a block cycle, matching 0277's decision).
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
  slice renders reputation tiers and the consumption slice renders the `report`
  kind, both through this frontend registration — though their ordering is
  transitively satisfied by 0277 blocking them.
- External/tooling gates, all of which gate completion: the Claude Design canvas
  (run via the `design` skill) must update the prototype *before*
  implementation, since the implemented glyph, big-glyph and colour are
  sign-off-judged against it; a human design reviewer must be available to
  perform that subjective sign-off; and the pinned Docker/Linux visual-regression
  harness must be available to regenerate and commit the baselines. If any is
  unavailable, the design-sign-off criterion and the VR-baseline criterion cannot
  be met.

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

## Technical Notes

- Registry surface: enum + method arms + the six predicates in
  `cli/corpus/src/doc_type.rs`; presentation via `server/src/doc_type_view.rs`
  (iterates `all()`, no per-variant edit); `PHASES` in `api/library.rs`.
  Rust↔TS parity is hand-synced (no codegen) — each side guards itself
  (`parity.rs`, `global.test.ts`).
- Card fields: title from manifest `title` (fallback: first H1, then humanised
  directory); status chip from base `status`; slug from parent directory; no
  counts (deferred to 0284).
- VR: `mise run test:e2e:visualiser:docker:update` regenerates baselines; native
  `mise run test` never runs VR. `fixture-coverage.spec.ts` iterates
  `DOC_TYPE_KEYS` and navigates `/library/<type>` and `/library/<type>/<slug>`,
  so the server fixture and `DETAIL_ROUTE_SLUGS` entry are mandatory or native
  specs fail.
- Claude Design prompt (run via the `design` skill against
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full`):

```text
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

Keep tokens, typography, and interaction patterns consistent with the existing
prototype. Output the updated prototype.
```

## Drafting Notes

- Enriched interactively on 2026-09-10 against epic 0121 and sibling 0277,
  grounded by two codebase investigations. Scope expanded from pure
  visualiser/indexer to also absorb the manifest-status collapse (the
  0277-delta) at the user's direction.
- Card status: chose to collapse `research_status` onto base `status` over
  redirecting the card to read `research_status` — one authoritative status
  field, and the card needs no per-type status logic.
- Click-target reverted to `manifest.md` (design-inventories precedent),
  dropping the earlier `primary`-pointer redirect, which moves to 0284.
- Corrected the epic's and the prior draft's "passing on darwin and linux" VR
  language: the harness renders only in a pinned Docker/Linux container with a
  single canonical baseline set.
- Enum predicates mirror `design-inventories` per user direction.
- Agreed follow-up: update epic 0121's artifact contract to match the collapse —
  the manifest frontmatter row, the `research_status` state-transition table,
  and the "never infer set progress from the manifest's base `status`" rule.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 1, visualiser half);
  `meta/work/0277-single-round-web-research-engine.md` (engine sibling)
- Prototype:
  `meta/research/design-inventories/2026-05-21-015231-claude-design-prototype/prototype-full`
- Internal: `cli/corpus/src/doc_type.rs`,
  `cli/corpus/src/frontmatter_validation/schema.rs`,
  `cli/visualiser/server/src/file_driver.rs`,
  `cli/visualiser/server/src/indexer.rs`,
  `cli/visualiser/server/src/api/library.rs`,
  `cli/visualiser/frontend/src/api/types.ts`,
  `cli/visualiser/frontend/src/components/Glyph/`,
  `templates/topic-research-manifest.md`,
  `skills/research/research-topic/SKILL.md`
