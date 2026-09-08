---
type: "work-item"
id: "0277"
title: "Single-Round Web Research Engine"
date: "2026-09-08T11:42:24+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "ready"
kind: "story"
priority: "high"
parent: "work-item:0121"
blocks: ["work-item:0278", "work-item:0279", "work-item:0280", "work-item:0281"]
relates_to: ["work-item:0278"]
tags: ["research", "skills", "deep-research"]
last_updated: "2026-09-09T07:51:08+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-861"
---

# 0277: Single-Round Web Research Engine

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

The walking-skeleton engine: run the full build loop once over web sources —
`brief` → `outline` → `conduct` (single round) → `synthesise` — producing a
contract-conforming *set* (the on-disk document collection for one subject,
rooted at `meta/research/topics/<slug>/`). This is the engine half of epic
Slice 1; the visualiser registration and nested-manifest indexer are the sibling
child (0278), which must co-land so the vertical demo lands whole. The engine's
artifacts are only reader-observable once 0278 co-lands.

## Context

An Accelerator user gains an end-to-end, observable research loop whose output —
a citation-backed dossier on an arbitrary external subject — they can read and
judge. The epic's central design risk is whether the loop produces a dossier
worth reading, and no single verb puts that question in front of a user — so the
loop must be observable end to end. This child writes conforming artifacts to disk
(judged later by the Slice 3 output-quality gate in 0280); 0278 makes them
browsable. This engine is built on the reusable-infrastructure seam: a generic
`researcher` agent specialised at spawn time by an injected
`(source_profile, question)` pair, mirroring the `reviewer` agent.

## Requirements

- `paths.research_topics` config key defaulting to `meta/research/topics`, so the
  skill locates the set directory (the server-side `config_path_key` wiring
  belongs to 0278).
- Manifest / brief / outline / finding / synthesis templates (web-only)
  resolving through the 3-tier override (config path → user override → plugin
  default), plus per-skill `instructions.md`/`context.md` wiring.
- A dedicated `manifest.md` aggregate root written at `brief` time, carrying the
  `primary` pointer (`brief.md` before synthesis, `synthesis.md` after).
- The generic `researcher` agent granted WebFetch of arbitrary URLs, plus a web
  source profile that captures each source URL inline in findings with its
  reputation tier drawn from a closed vocabulary — `tier-1` (authoritative
  primary: official docs, standards bodies, peer-reviewed), `tier-2` (reputable
  secondary: established outlets, recognised experts), `tier-3` (unvetted: blogs,
  forums, wikis, marketing) — so the corpus conforms from the first file written.
  The tier reflects venue standing only, never a claim's correctness.
- The `research-topic` skill dispatching `brief` + `outline` + `conduct`
  (single round) + `synthesise` on the `configure` pattern, resolving a slug to
  the set root via the general `accelerator corpus resolve --type topic-research
  <slug>` subcommand (see Technical Notes) on every verb but `brief`.
- An `outline` prompt carrying the effort-scaling rubric (1 focus area for a
  simple subject, 2–4 for comparisons, 10+ for broad subjects), sizing the round
  at or beneath `breadth` as a ceiling — so at the hardcoded `breadth: 8` the
  rubric's 10+ band is unreachable and the ceiling always wins.
- `conduct` writing one immutable finding per focus area into `findings/`;
  `synthesise` writing `synthesis.md` inline from the findings.
- Hardcoded defaults `breadth: 8`, `depth: 1` (made tunable in 0282). `depth`
  governs intra-finding recursion and is inert at `depth: 1` in this single-round
  slice — recursion arrives with 0282.
- `topic-research` registered as a typed-linkage source type so other documents
  can reference a set via `relates_to: ["topic-research:<slug>"]` (or `parent:`,
  etc.), decoupled from the 0278 doc-type registration.

## Acceptance Criteria

- [ ] Given a subject, `brief` interactively scopes it and produces
      `manifest.md` + `brief.md` conforming to the artifact contract, with
      `research_status: briefed`, `primary` pointing at `brief.md`, and
      `brief.md` declaring `source_profiles: ["web"]`.
- [ ] Given a slug, `outline` writes `outline.md` — a `## Round 1`
      checklist of focus areas whose count is at or beneath `breadth` (the
      effort-scaling rubric guides how far beneath, but the ceiling is the
      pass/fail) — and sets `research_status: outlined`; the user may hand-edit it.
- [ ] Given a slug, `conduct` runs one round over the outline's focus areas
      (in this single-round slice every focus area is outstanding, so no
      gap-detection is implied — that is 0279), writes one immutable finding per
      focus area into `findings/`, flips the corresponding checkboxes, and sets
      `research_status: researching`, `round_count: 1`, and `finding_count` equal
      to the number of focus areas researched. Each finding carries
      `kind: finding`, `round`, `question` (its focus area's question),
      and `source_profile: web`.
- [ ] Given a slug, `synthesise` writes `synthesis.md` from the findings,
      sets `research_status: synthesised`, and flips `primary` to
      `synthesis.md`.
- [ ] For a non-`brief` verb, `corpus resolve --type topic-research` accepts a
      slug (or the set directory, or a sub-document path) and resolves each to the
      same `meta/research/topics/<slug>/` set root. Because topic-research is
      registered as a doc type in 0278, this criterion is verified at the
      0277+0278 co-land; the generic resolver itself is verified in 0277 against an
      already-registered type.
- [ ] `accelerator corpus resolve --type <type> <slug>` is general-purpose: given
      a doc type and a slug it resolves to that document's root — a file for flat
      types, the set directory for nested-manifest types — canonicalised and
      confined under the type's configured path. 0277 delivers it and tests it
      against an already-registered doc type.
- [ ] `paths.research_topics` resolves to its documented default
      `meta/research/topics` when unset, and a bare slug resolves against it to the
      set root.
- [ ] With no override, `breadth` resolves to `8` (observable via the round
      ceiling) and `depth` resolves to `1`.
- [ ] Focus areas commissioned per round never exceed `breadth`; the rubric may
      only reduce the count beneath that ceiling, never raise it.
- [ ] Every source entry in every finding carries an inline reputation tier drawn
      from the closed set `{tier-1, tier-2, tier-3}` from the first round onwards,
      web-profile sources included, and those tiers are carried into
      `synthesis.md`.
- [ ] Templates resolve through the 3-tier override, keyed by `(type, kind)`: a
      `topic-research-<kind>` template is used when present, falling back to a
      general `topic-research` template. Users can override either level and add
      per-skill `instructions.md`/`context.md`.
- [ ] Exactly one generic `researcher` agent exists and is spawned with the web
      profile injected; no web-specific agent definition is present.
- [ ] Every document the loop writes (`manifest.md`, `brief.md`, `outline.md`,
      each finding, `synthesis.md`) passes `accelerator corpus frontmatter
      validate`. The validator resolves each document by its `(type, kind)` pair
      — `type: topic-research` plus its `kind`
      (`manifest`/`brief`/`outline`/`finding`/`synthesis`) — so per-kind required
      fields and the per-kind base `status` vocabulary are enforced, not merely
      the type-level standard.
- [ ] A `topic-research:<slug>` typed-linkage reference in another document passes
      the frontmatter shape check; full dangling-reference integrity (whole-corpus
      mode) co-lands with the 0278 indexer.
- [ ] `mise run check` passes end-to-end for the engine — the skill, generic
      `researcher` agent, templates, and the `paths.research_topics` config key
      land with format, lint, type-check, and tests green. These engine checks are
      green in isolation; the visualiser doc-type registration's checks belong to
      0278, so the two co-land for the vertical demo, not because either's checks
      require the other.

## Open Questions

- None specific to this slice — the epic resolved its design questions; the one
  still-open question (set-document API shape) is scoped to 0284.

## Dependencies

- Blocks: 0278, 0279, 0280, 0281 — all recorded on this item's `blocks`, the
  canonical side (0281's `blocked_by` carries the reverse edge).
- Co-land constraint: must merge together with 0278 so the vertical demo
  (engine + library entry) is not lost. `blocks` carries the ordering — 0278's
  indexer keys on the `manifest.md` this engine writes — while the co-land
  simultaneity is additionally recorded as `relates_to: 0278`, because a
  reciprocal `blocked_by` would assert a mutual block (a `blocks`/`blocked_by`
  cycle) and misrepresent the relationship.

## Assumptions

- Sources are external web only in this slice; academic sources arrive in 0280.
- The artifact contract's field names are stable; findings are immutable once
  written, so any later additions must be backward-compatible.
- The full `brief` → `outline` → `conduct` → `synthesise` loop depends on live
  external web access via WebFetch; the walking-skeleton demo and any full-loop
  verification are coupled to external site availability — distinct from the
  academic-API rate limits deferred to 0280.

## Technical Notes

- **Kind-discriminated schema and templates.** The artifact contract's
  discriminator is `kind` (mirroring the work-item `kind` field, renamed from the
  epic's earlier `research_kind`). The corpus frontmatter schema and template
  resolution both move from a per-`type` lookup to a `(type, kind)` lookup with a
  type-level fallback: a document resolves to its `(type, kind)` schema row, or to
  the type-default `(type, "")` row when no kind-specific row exists, so the
  existing kind-agnostic types resolve unchanged. Each `kind` therefore carries
  its own required fields and base-`status` vocabulary (findings `complete`,
  `brief.md` `draft`→`complete`, and so on), enforced by `corpus frontmatter
  validate`. Templates resolve the same way — `topic-research-<kind>` when
  present, else `topic-research`. This is a general mechanism, deliberately not
  topic-research-specific: it paves the way for per-work-item-kind templates
  (`work-item-<kind>` falling back to `work-item`).
- **Slug resolution.** A general `accelerator corpus resolve --type <type>
  <slug>` subcommand on the `corpus` binary (mirroring `work-cli/resolve.rs`)
  resolves a slug to a document's root for any doc type — a file for flat types,
  the set directory for nested-manifest types — reusing
  `config::paths::resolve_with_fallback` and canonicalise-under-root. The skill
  calls it with `--type topic-research`, accepting the tolerant forms (slug, set
  directory, or a sub-document path) and surfacing it in each subcommand's
  `argument-hint`. ⚠️ Type-driven resolution of `topic-research` needs the type
  registered (`DocTypeKey` + nested-manifest layout), which lands in 0278, so the
  generic resolver ships and is tested in 0277 while topic-research resolution is
  verified at co-land. ⚠️ For dated flat types (`YYYY-MM-DD-…-<slug>.md`) the slug
  is ambiguous — resolve with candidate handling mirroring `work resolve`.
- **Linkage-target registration.** `topic-research` is added to the typed-linkage
  source-type vocabularies — `LINKAGE_SOURCE_TYPES` (`schema.rs`) and
  `SOURCE_TYPES` (`template_shape.rs`) — plus the `cargo-public-api` snapshot.
  These arrays are independent of `DOC_TYPES`/`DocTypeKey`, so it lands cleanly in
  0277: the shape check passes in isolation, while the dangling-reference check
  (whole-corpus mode) needs the 0278 indexer and co-lands.
- **Source-profile packaging.** The web source profile is injected per spawn by
  path-passing (the ADR-0005 reviewer pattern), not a static `skills:` preload:
  the profile varies per spawn (academic + mixed rounds arrive in 0280), which a
  fixed preload cannot express. `conduct` passes the profile skill's path and the
  finding output-format path; the researcher Reads both in its own context.
  Profiles live at `skills/research/profiles/<profile>/SKILL.md` (web here),
  mirroring `skills/review/lenses/`.
- The skill writes a set into a temp dir and renames it in, because the indexer's
  lister skips dot-prefixed dirs and must never see a half-written set.
- Base `status` is per-document and distinct from the set-level
  `research_status`; never infer set progress from `manifest.md`'s base
  `status`. For the shapes this slice writes: findings are `complete` from first
  write (immutable, never `draft`); `synthesis.md` is `complete` once written;
  `outline.md` is `complete` once it exists (progress lives in its checkboxes);
  `brief.md` is `draft` during interactive scoping and `complete` once authored;
  `manifest.md` is `complete` once the file exists.
- Finding immutability is asserted from first write in this slice but is first
  *exercised* by the re-invocation / gap-detection criteria in 0279 — no
  single-round criterion here re-runs a verb over an existing finding.

## Drafting Notes

- Slice 1 was split into this engine child and the visualiser/indexer child
  (0278) at the user's request; the partition puts skill/agent/template/artifact
  here and doc-type registration + nested-manifest indexing in 0278. The two
  must co-land.
- `paths.research_topics` default lives here (skill-side path resolution); the
  server `config_path_key` moved to 0278.
- Kept `breadth: 8` / `depth: 1` hardcoded; tunability is 0282. Because
  `breadth` is a hard ceiling, the effort-scaling rubric's 10+ top band is
  unreachable in this slice — the rubric may only reduce beneath 8. This is
  deliberate; 0282 lifts the ceiling.
- Review 1 (2026-09-08, clarity/completeness/dependency/scope/testability)
  returned REVISE on three majors and its findings were worked through in the
  same session. Resolved: the reputation-tier vocabulary was enumerated as a
  closed set (`tier-1`/`tier-2`/`tier-3`) in Requirements and the AC; a
  three-form set-handle resolution criterion and a `paths.research_topics`
  default-resolution criterion were added; the co-land with 0278 was recorded
  machine-readably as `relates_to: 0278` (a reciprocal `blocked_by` was rejected
  as a cycle) and 0281 added to `blocks` for canonical-side parity; `conduct`'s
  expected counts and the "outstanding = all this round" reading were stated; the
  engine's `mise run check` was clarified as green in isolation; the inert
  `depth: 1`, a `set` gloss, a beneficiary sentence, the WebFetch coupling, and
  the 0279 immutability-verification note were folded in.
- Enriched interactively on 2026-09-08 against the full parent epic (0121): the
  Slice 1 artifact-contract conformance detail was folded in (base `status`
  per-shape values, finding frontmatter fields, the brief's `source_profiles`,
  and the `corpus frontmatter validate` + `mise run check` gates). Kind
  (`story`) and status (`draft`) reviewed and kept; promoting to `ready`
  remains a downstream decision.
- Codebase-research session (2026-09-08) resolved the validator-depth question:
  enforce per-shape frontmatter by extending the corpus schema to a `(type, kind)`
  lookup rather than a single flat `topic-research` row (which would have left
  four of five templates unguarded, since one type admits only one schema row).
  The discriminator was renamed `research_kind` → `kind` to mirror work items, and
  both schema and template resolution were generalised to `(type, kind)` with a
  type-level fallback — a mechanism that also paves the way for per-work-item-kind
  templates. Recorded in
  `meta/research/codebase/2026-09-08-0277-single-round-web-research-engine.md`;
  candidate for an ADR.
- Codebase-research session (2026-09-09) chose the `corpus` binary for slug
  resolution and generalised it: `accelerator corpus resolve --type <type>
  <slug>` resolves any doc type by slug, not just topic-research. Renamed the
  positional from "set handle" to "slug", reusing the codebase's cross-type
  identifier rather than a new term (`--type`, not `--root`). Because type-driven
  topic-research resolution couples to the 0278 doc-type registration, the generic
  resolver ships in 0277 and topic-research resolution is verified at co-land.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 1, engine half)
- Parent epic: 0121
