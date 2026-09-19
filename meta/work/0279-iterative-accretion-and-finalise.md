---
type: "work-item"
id: "0279"
title: "Iterative Accretion and Finalise"
date: "2026-09-08T11:42:24+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "ready"
kind: "story"
priority: "high"
parent: "work-item:0121"
blocks: ["work-item:0282", "work-item:0284"]
relates_to: ["work-item:0278"]
tags: ["research", "skills", "deep-research"]
last_updated: "2026-09-19T15:38:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-863"
---

# 0279: Iterative Accretion and Finalise

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Grow the knowledgebase across multiple rounds without clobbering, and close a
subject off. Adds next-round outline appending, gap-detection `conduct`
re-invocation, wholesale synthesis rewrite under anti-changelog discipline (the
findings and synthesis read as one current dossier, never a round-by-round log),
the full five-state base `status` lifecycle on `manifest.md` with reopen
regression, and the `finalise` verb. Synthesis staleness is a derived property of
that lifecycle — a `synthesised` set reopened by `outline` or `conduct` regresses
to `researching`, and `finalise` refuses any set not currently `synthesised` — so
no separate staleness field exists. Accretion and `finalise` ship as one
increment because reopen-regression and the finalise gate are two faces of that
same lifecycle.

## Context

An Accelerator user can grow a subject's knowledgebase across multiple rounds
without losing earlier findings, and close it off once it is researched enough.
Slice 1 (0277) runs the loop once; this makes it iterative and closable. No
visualiser work is required — round and focus-area structure becomes visible in
the Slice 6 detail page (0284), not before. The round loop runs at `depth: 1`;
automatic intra-finding recursion is the separate child 0283, whose `depth > 1`
recursion is independent of this slice's `conduct` changes, so 0283 is not
blocked by this item — though both amend `conduct`, so whichever of the two lands
second carries a light integration cost over the first.

## Requirements

- `outline` gains next-round appending: when the highest `## Round N` has been
  conducted (has at least one finding), a new `## Round N+1` checklist of focus
  areas is appended without clobbering earlier rounds. When the highest round is
  still pending (no findings), `outline` revises that round in place rather than
  appending — a round is a conducted cycle, so an un-conducted round is
  re-planned, not duplicated.
- `conduct` gains gap-detection re-invocation: researches only outstanding focus
  areas, never rewrites existing findings, flips outline checkboxes as findings
  land (finding-existence is ground truth; the checkbox is reconciled to match).
- `synthesise` rewrites `synthesis.md` wholesale with anti-changelog discipline
  over the findings and synthesis (`outline.md` exempt — it is the working log),
  stamping `rounds_covered` to match `manifest.md`'s `round_count`.
- `outline`/`conduct`/`synthesise`/`finalise` keep `manifest.md`'s base
  `status`, `round_count`, `finding_count` and `primary` in sync.
- The five-state lifecycle `briefed → outlined → researching → synthesised →
  complete`, carried on `manifest.md`'s base `status`, with reopen regression to
  `researching`.
- `finalise` closes a subject: refuses any set whose base `status` is not
  `synthesised` (covering both an absent and a stale synthesis), then sets base
  `status: complete` — the only path to that value, reversible by a later
  `outline` or `conduct`.

## Acceptance Criteria

- [ ] Round append after conduct — given the highest `## Round N` in
      `outline.md` has at least one finding, when `outline` runs, then it appends
      a `## Round N+1` checklist of new focus areas, leaves every earlier round's
      heading and items unchanged, and leaves base `status` at `researching` —
      unchanged if the set was already `researching`, regressed from
      `synthesised`/`complete` per the reopen criterion.
- [ ] Pending-round revision — given the highest `## Round N` has no findings,
      when `outline` runs, then it revises Round N in place and appends no new
      round; the revision leaves base `status` unchanged — `outlined` when the set
      was `outlined`, `researching` when a prior `outline`/`conduct` had already
      reopened it.
- [ ] Gap-only conduct — given a set with outstanding (unchecked, finding-less)
      focus areas alongside researched ones, when `conduct` runs, then it writes
      findings only for the outstanding areas and leaves every existing finding
      file unchanged.
- [ ] Checkbox and count reconciliation — given a `conduct` run lands new
      findings, when it completes, then it reconciles each focus area's checkbox
      to finding-existence (a finding present flips its box to `- [x]`, findings
      on disk being ground truth), sets `manifest.md`'s `finding_count` to the
      number of finding files on disk, and sets `round_count` to the highest
      `round` stamped on any finding — so a gap-fill `conduct` within an existing
      round leaves `round_count` unchanged while a conduct of a newly appended
      round raises it.
- [ ] Wholesale, idempotent synthesis — given findings spanning multiple rounds,
      when `synthesise` runs, then it rewrites `synthesis.md` wholesale over all
      findings and stamps `rounds_covered` equal to `round_count`; a second
      `synthesise` with no intervening findings leaves `rounds_covered` unchanged
      and reproduces the same cited findings and sources and the same section
      structure (the generated prose need not be byte-identical).
- [ ] Single dossier — given a multi-round set, when a reader opens
      `synthesis.md` and the findings, then neither `synthesis.md` nor any finding
      carries per-round section headings or round-sequencing language (e.g. "in
      the first round…", "Round 2 added…") — a finding's `round` frontmatter stamp
      is the immutable ledger, not its prose; `outline.md` is exempt and retains
      its round-grouped checklist. Holistic readability is judged by the epic's
      Slice 3 human output-quality gate, not here.
- [ ] Lifecycle transitions (happy path) — given a set at each transition's
      starting state, when the build verb runs, then it sets `manifest.md`'s base
      `status`: `brief` (new set)→`briefed`, `outline` from `briefed`/`outlined`
      →`outlined`, `conduct` from `outlined`/`researching`→`researching`,
      `synthesise` from `researching`→`synthesised`, `finalise` from
      `synthesised`→`complete`. Reopen regression and the finalise refusal are
      covered by their own criteria below.
- [ ] Reopen is the sole staleness signal — given a `synthesised` or `complete`
      set, when `outline` or `conduct` runs, then base `status` regresses to
      `researching` and no `stale`/`staleness` frontmatter field appears on
      `manifest.md`, `outline.md`, `synthesis.md`, or any finding.
- [ ] Finalise refuses a non-synthesised set — given a set whose base `status`
      is not `synthesised`, when `finalise` runs, then it exits non-zero with a
      message naming the current status and mutates nothing; the single gate
      covers both an absent synthesis (never reached `synthesised`) and a stale
      one (regressed to `researching`).
- [ ] Finalise completes a synthesised set — given a set whose base `status` is
      `synthesised`, when `finalise` runs, then it sets base `status: complete`
      (the only transition that yields `complete`), and a subsequent `outline` or
      `conduct` returns the set to `researching`.
- [ ] `primary` stays on the dossier once one exists — given a set that has been
      `synthesised` at least once, when `outline` or `conduct` reopens it to
      `researching`, then `manifest.md`'s `primary` remains `synthesis.md` (the
      stale dossier is still the reader's landing document until the next
      `synthesise`), and `finalise` likewise leaves `primary` at `synthesis.md`.

## Open Questions

- None. The synthesis-staleness mechanism — the sole question this slice carried
  — is resolved: staleness is a derived property of the base `status` lifecycle,
  not a stored flag or a `round`/`rounds_covered` comparison (see Technical
  Notes).

## Dependencies

- Blocked by: 0277 (the single-round engine — recorded on 0277's `blocks`).
- Blocks: 0282 (the depth/breadth tunability knob wires override flags into this
  round loop), 0284 (the detail page shows this slice's round structure); also
  gates 0281's gap-proposal (recorded on 0281's `blocked_by`).
- Relates to: 0278 (already merged) — its manifest-status collapse and the
  `complete`-admitting `(topic-research, manifest)` schema row are what
  `finalise`'s `status: complete` write depends on; recorded as `relates_to`
  rather than a block because 0278 is done.

## Assumptions

- Anti-changelog discipline applies to findings and synthesis only; `outline.md`
  remains the freely-rewritten working log.
- Gap-detection `conduct` inherits 0277's live-web (WebFetch) coupling: verifying
  or demoing re-invocation needs live external web access and is subject to
  external site availability.

## Technical Notes

- `synthesised` asserts only that a current dossier exists, not that every focus
  area is researched (interim synthesis is allowed); completeness is what
  `finalise` asserts.
- Staleness is derived, not stored. `status` reaches `synthesised` only through
  `synthesise`, which rewrites `synthesis.md` wholesale over all current findings
  — so `synthesised` implies a current dossier by construction. The only verbs
  that then add findings or focus areas, `conduct` and `outline`, regress
  `status` to `researching`, which is itself the staleness mark; `finalise`'s
  gate is therefore the single condition base `status == synthesised`. A stored
  `stale` flag was rejected — it would reintroduce the dual-source-of-truth the
  0278 manifest-status collapse removed. `synthesis.md`'s `rounds_covered`
  (stamped to `round_count` on each `synthesise`) stays reader-facing provenance
  and a corroborating check — a finding whose `round` exceeds `rounds_covered`
  confirms staleness — but is never the authoritative gate.
- A round is a conducted cycle. `round_count` is derived as the highest `round`
  stamped on any finding on disk, so only `conduct` can advance it — by landing
  findings in a newly appended round — and `outline` never does; a `## Round N`
  checklist is only a pending plan until findings land against it, and a gap-fill
  `conduct` that adds findings to an existing round leaves `round_count`
  unchanged. Re-running `outline` before that first `conduct` revises the pending
  round in place — legitimate because `outline.md` is anti-changelog-exempt and
  no immutable findings exist yet — and a new round is appended only once the
  current one has been conducted.
- `primary` tracks whether a dossier exists at all, not whether it is fresh:
  `synthesise` flips it to `synthesis.md` on first run and nothing flips it back,
  so a reopened set keeps `synthesis.md` as the reader's landing document until
  the next `synthesise` overwrites it.

## Drafting Notes

- Staleness mechanism resolved (2026-09-19): derived from the base `status`
  lifecycle, not a stored flag or a `round`/`rounds_covered` comparison. This
  supersedes the earlier framing that left it as an Open Question; the resolution
  leans on the 0278 manifest-status collapse, which post-dated this item's
  extraction.
- Reconciled `research_status` → base `status` throughout (2026-09-19), per the
  0278 manifest-status collapse recorded on epic 0121; 0277's validation flagged
  this sibling reconciliation as outstanding. There is no distinct
  `research_status` field — the set lifecycle lives on `manifest.md`'s base
  `status`.
- Acceptance criteria split from five bundled checks into ten atomic
  Given/When/Then criteria (2026-09-19); the `outline`-on-a-pending-round
  behaviour (revise in place, not append) was pinned down in the same pass. An
  eleventh criterion (the `primary`-on-reopen check) was added in Review 1 below.
- Runs at `depth: 1`; the recursion engine is the separate child 0283.
- Interactively enriched on 2026-09-19 against the parent epic (0121) and slice 1
  (0277). Kind (`story`), priority (`high`) and status (`draft`) reviewed and
  kept; promoting to `ready` remains a downstream decision.
- Review 1 (2026-09-19, clarity/completeness/dependency/scope/testability)
  returned REVISE and its findings were worked through in the same session.
  Resolved: the lifecycle-transition and pending-round criteria were scoped to
  their starting states so they no longer contradict reopen regression or the
  finalise refusal; `round_count` was pinned to the highest `round` stamped on
  any finding (a gap-fill `conduct` leaves it unchanged); the idempotent-synthesis
  and single-dossier criteria were re-anchored to observable checks with holistic
  readability deferred to the Slice 3 human gate; a `primary`-on-reopen criterion
  was added (stays `synthesis.md`); the finalise refusal was given an observable
  signal (non-zero exit naming the current status) and the staleness-field
  negative was bounded to the set's documents; 0278 was recorded as `relates_to`
  (already merged, so not a block) and 0283 noted as independent of this slice's
  `conduct`; and a beneficiary clause, the `anti-changelog`/`the knob` glosses,
  and the inherited WebFetch coupling were folded in.

## References

- Source: `meta/work/0121-topic-research-skillset.md` (Slice 2)
- Parent epic: 0121
- Related: `meta/work/0277-single-round-web-research-engine.md` (Slice 1 engine;
  the base-`status` lifecycle and artifact contract this slice extends)
