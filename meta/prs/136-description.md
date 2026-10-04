---
type: "pr-description"
id: "136"
title: "[0283] Deepen research-topic findings recursively through level notes and a composer"
date: "2026-09-28T11:44:59+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0283"
parent: "work-item:0283"
relates_to: ["work-item:0280", "work-item:0282", "work-item:0295"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/136"
pr_number: 136
tags: ["research", "skills", "deep-research"]
revision: "1517c709352fcd37619be0ec1af13e2d09e1be7a"
repository: "accelerator"
last_updated: "2026-10-04T10:10:37+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0283] Deepen research-topic findings recursively through level notes and a composer

## Summary

At `depth > 1`, `research-topic`'s `conduct` now researches each (focus area,
profile) pair as a tree instead of a single researcher pass. Each node writes a
level note under `findings/<stem>.levels/<lineage>.md` recording capped
follow-up questions; `research topic outstanding --depth N` derives the
next level from the notes on disk, and a fetch-less `composer` agent writes the
pair's one finding once the tree is complete. Every batch of spawns, at any
depth, is capped by a new `concurrency` knob (default 24). `depth: 1` keeps
today's single-researcher shape, now stamped `depth: 1`.

## Changes

### The `research` domain

The crate is grouped into `sources` (the `research fetch` client), `topic`
(the set, its trees and the round plan) and `conduct` (coordinating a run),
and each topic-research concept is modelled once.

- **`topic/tree.rs`** — pure derivation of a pair's `Stage` from its notes:
  a single pass at depth 1 with no `.levels/`, otherwise the
  `MissingNode`s still to research or the lineages to compose from.
  Follow-up cap 4 at level 1 then `ceil(parent / 2)`, trim-then-dedupe in
  lineage order against the pair question, shallower nodes and earlier
  siblings, level barrier within a pair, pruning on `follow_ups: []`.
- **`topic/layout/`** — the set layout: indexed `[a-z0-9-]` finding stems,
  well-formed lineages (`L` then `L − 1` positive positions), note paths and
  ids, question slugs, and the finding and level-note path checks. They
  move out of `corpus::topic_research`, so `corpus` stays generic over
  document types and `research` keeps its std-only pup rule with no
  `corpus` dependency.
- **`topic/question.rs`** — question normalisation, now after NFKC
  compatibility folding, behind a `UnicodeText` port.
- **`topic/claims.rs`** — a quarantined finding or a `.levels/` directory
  claims its `<nn>` index for the question it names, and a run keeps every
  index it plans at, so a resumed pair keeps its stem.
- **`topic/evidence.rs`, `topic/plan.rs`** — the findings and level notes
  read from disk, and the `RoundPlan` derived from them.
- **`conduct/`** — batch planning across pairs (`window.rs`; researchers
  and composers share batches), what a run remembers and has seen
  (`memory.rs`, `observed.rs`), and a transient `<set>/.conduct-run.json`
  ledger (`ledger.rs`) that detects notes written outside the acknowledged
  batch, digest changes, and superseded runs
  (`E_TOPIC_RESEARCH_RUN_SUPERSEDED`).

### CLI and adapters

- **`outstanding`** gains `--depth`, `--limit`, `--start`, `--run` and
  `--spawned`, and reports each pair's `stage` (`single_pass`,
  `research_nodes`, `compose`), `nodes` or `notes`, `trims`, `shallow`
  findings, `unfinished` spawns and `unexpected` writes. It validates every
  note it reads and treats a failing one as missing. A new `end-run` verb
  removes the ledger. The wording of warnings, trims, skips and refused
  notes lives in `research-cli`, not the domain.
- **`research-adapters`** mirrors the domain: `topic` reads a set's round
  inputs, and `conduct::ledger` stores the run ledger.
- **`finding_count` / `round_count`** count only top-level `<nn>-*.md`
  findings; `.levels/` is never read by `synthesise`.
- **`UnicodeTables`** — implements `UnicodeText` from one ICU4X release
  (`icu_normalizer`, `icu_properties`, both exact-pinned `=2.2.0`): NFKC,
  the hidden general categories and `Default_Ignorable_Code_Point`. Both
  crates and their compiled data were already in the build through
  `idna_adapter`, so there is no hand-copied Unicode table to keep in step.
- **Generic `corpus` ports** — `DirectoryProbe` lets the round reader spot
  `.levels/` directories, and `FileRemove` lets `end-run` delete the ledger.
  `corpus-adapters` implements both; a removal escaping the store root is
  refused. The repository ignores `**/.conduct-run.json`, as the changelog
  recommends to users.

### Schema and templates

- **`templates-schema.tsv` retired** — `SCHEMA` is the one source of schema
  rows; `print-schema` emits every row under `rows` in place of its
  `optional_extras` bank.
- **Per-row extras** — each `SchemaRow` splits `required_extras` from
  `optional_extras`, replacing the global `OPTIONAL_EXTRAS` bank (`migrate`'s
  m0007 follows). `follow_ups: []` is exempt from the empty-placeholder
  check, and `Violation::schema_key` lets `outstanding` name a failing note's
  key without echoing one taken from the untrusted file.
- **`topic-research-level-note.md`** — new `kind: level-note` row with
  `level`, `depth`, `follow_ups`; findings gain optional `depth` (legacy
  findings read as `depth: 1`).

### Write guard

- **Role-keyed confinement** — the researcher may also write well-formed
  level-note paths; the new composer role (`accelerator:composer`, or
  `agents.composer`) may write only `findings/<nn>-<name>.md` and run no
  commands. A confined write must now name an indexed `[a-z0-9-]` stem.

### Skills, agents and docs

- **`agents/composer.md`** (`Read, Write` only), `level-note-outputter`, and
  the recursive `conduct` prose in `research-topic`: known-question
  injection, cap injection, batching to `concurrency`, per-node failure
  reporting by lineage.
- **`web-profile`** gains an Outcome section so a denied fetch writes no
  file, and drops the focus-question carve-out on fetching page-supplied
  URLs; a contract test holds all three profiles to both.
- **Surface** — dormant-depth notice and "no effect until 0283" caveat
  removed; `research.topic.concurrency` and the `composer` agent key are
  configurable and documented in the docs site.

### Build system

- **Dated `npm audit` ignores for the docs site** — `docs:audit:check` now
  reads `npm audit --json` and fails only on high or critical advisories
  that no live entry in `docs-site/audit-ignores.toml` covers. Each entry
  needs a reason carrying `review-by: YYYY-MM-DD`, mirroring the advisory
  ignores in `cli/deny.toml`; past that date the entry stops suppressing
  and the check fails naming it. The one entry is GHSA-ch52-4w7c-c8xp
  (`http-cache-semantics <=4.2.0`, reached through `astro`), which has no
  patched release and turned the default task red on every branch.

## Context

- Work item: `meta/work/0283-recursive-finding-deepening.md` (PP-867)
- Plan: `meta/plans/2026-09-26-0283-recursive-finding-deepening.md`
- Research: `meta/research/codebase/2026-09-26-0283-recursive-finding-deepening.md`
- Validation: `meta/validations/2026-09-28-0283-recursive-finding-deepening-validation.md`
- Follow-up: 0295 (per-profile arXiv batch cap), raised from the Phase 7
  contention measurement.

## Testing

- [x] `mise run check` exits 0 on the branch tip
- [x] Full `mise run` (default task) exits 0 on the branch tip after the
      crate restructure: 4447 CLI tests, 2611 frontend tests
      and 355 visualiser e2e specs pass, alongside the tree derivation, caps,
      dedupe, lineage ordering, level barrier, index retention, run ledger,
      guard role and profile contract suites
- [x] ICU4X agrees with the replaced `unicode-properties` categories and
      hand-copied ignorable table on all 1,112,064 scalar values, and with
      `unicode-normalization`'s NFKC on every single scalar value
- [x] `mise run test:integration:pup` passes (90 tests) with `research`'s
      domain rule back to std and `kernel::Error` only; `public-api:check`
      pins the moved items under `research`
- [x] 21 unit tests cover the audit ignores: severity threshold, per-advisory
      suppression, lapsed review-by dates, malformed entries, and the
      repository's own ignore list parsing
- [x] Attended verification: all 6 Phase 5 concurrency checks and all 17
      Phase 7 steps pass in headless `claude -p` runs (31 sessions, 158
      spawns, ~$76); whole-corpus `frontmatter validate` exits 0 in all 29
      scratch projects
- [x] Adversarial follow-up carrying a host name refused and quarantined;
      invalid notes quarantined as `.<lineage>.md.invalid` and re-researched
      on resume
- [x] Depth changes on resume (shallower composes from notes within depth;
      deeper extends from recorded follow-ups; validated findings never
      re-deepened)

## Notes for Reviewers

- ⚠️ **A configured `research.topic.depth` above 1 now takes effect.** A
  `depth: 3` config runs up to 13 researchers plus a composer per pair (5×
  at depth 2, 21× at depth 4). Call this out in release notes.
- ⚠️ **Deepened sets need every collaborator on this version.** An older
  plugin reports `.levels/` notes as an unknown kind and researches the pair
  again under a new stem.
- **`is_finding_path` narrows and moves** to
  `research::topic::layout::finding_path`: it admits only an indexed
  `[a-z0-9-]` stem, so a hand-named finding is now refused to a confined
  agent.
- 🔒 **Accepted residual risk** — the guard confines each role by path
  shape, not by assigned path, so a compromised researcher could forge
  another pair's not-yet-spawned note. The run ledger detects most such
  writes as `unexpected`; the undetected cases are listed in the plan's
  "What We're NOT Doing".
- ⏱️ **arXiv contention** — at depth 3 with default concurrency, 4 of 24
  arXiv nodes failed as `lock_contention`. 0295 tracks a per-profile batch
  cap.
- 🟡 **`batch` / `--spawned` wording** confused two orchestrators during
  validation; the ledger silently ignores a mismatched acknowledgement. A
  warning or plainer skill wording is a candidate follow-up.
- ⚠️ **GHSA-ch52-4w7c-c8xp is ignored until 2026-11-03.** The docs site is
  built statically, so the cross-user cache disclosure has no serving path;
  on that date `docs:audit:check` fails until the entry is re-checked
  upstream and removed or re-dated. The change is independent of 0283 and
  can be lifted into its own PR if preferred.
- Not in scope: semantic dedupe of follow-ups, a `fan_out` knob, visualiser
  navigation of level notes (0278, 0284), eval coverage of the orchestration
  contracts (0161), re-measuring the guard's 50 ms p95 budget under
  recursion.
- Suggested reading order: `cli/research/src/topic/layout/`, then
  `topic/tree.rs`, then `topic/plan.rs`, then `conduct/window.rs` /
  `ledger.rs`, then `confinement.rs`, then
  `skills/research/research-topic/SKILL.md`.
