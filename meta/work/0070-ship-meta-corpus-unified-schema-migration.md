---
id: "0070"
title: "Ship `meta/` Corpus Unified-Schema Migration"
date: "2026-05-17T17:16:35+00:00"
author: Toby Clemson
kind: story
status: ready
priority: high
parent: "work-item:0057"
tags: [migration, frontmatter, schema, dogfood]
last_updated: "2026-06-06T23:20:24+00:00"
last_updated_by: Toby Clemson
type: work-item
schema_version: 1
blocked_by: ["work-item:0060", "work-item:0033", "work-item:0061", "adr:ADR-0033"]
relates_to: ["adr:ADR-0034", "adr:ADR-0038", "work-item:0057", "work-item:0056", "adr:ADR-0023", "adr:ADR-0033", "adr:ADR-0037", "adr:ADR-0040", "codebase-research:2026-05-24-0068-related-documents-inference-accuracy"]
---

# 0070: Ship `meta/` Corpus Unified-Schema Migration

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Ship the numbered Accelerator migration that rewrites every existing artifact
under `meta/` to the unified frontmatter schema and populates structured linkage
frontmatter where free-form body sections allow confident inference. Inferences
that are confidently resolvable apply mechanically; ambiguous ones are surfaced
through the migration framework's interactive validation hook. Dogfood the
migration against this repo's own `meta/` corpus and fix any gaps surfaced. This
story is the integration point that closes out epic 0057.

The beneficiaries are the downstream visualiser-graph epic — which can only
build a reliable artifact graph once existing artifacts carry structured,
typed linkage frontmatter — and every userspace repo that runs
`/accelerator:migrate`, which needs its accumulated `meta/` corpus brought into
line with the now-unified producer schema rather than left stranded in
per-skill legacy shapes.

## Context

Per 0057, frontmatter had evolved per-skill into an inconsistent state —
field-name conflicts, shape inconsistencies, missing discriminators, absent
`schema_version`. The producer-side stories (0063, 0064, 0065, 0066, 0067) have
**shipped**, so new artifacts are now born unified; but existing artifacts under
`meta/` still need a corresponding rewrite. This story owns that rewrite.

The strategy fork that earlier drafts hedged ("interactive vs. post-run report")
is now decided. Spike 0068 measured the body-section linkage parser at 11.3%
wrong — above the ≤5% threshold — and returned a verdict of **interactive
hooks**. ADR-0038 parameterises the framework interactive contract (ADR-0037,
implemented by 0069) for this migration. This migration is the named first
consumer of that contract.

The migration is numbered after the latest applied (current head determined at
implementation time) and follows the migration-framework conventions of ADR-0023
as extended by 0069.

## Requirements

- Author a new numbered migration under the migration framework that:
  - (Plan `work-item:` → `work_item_id:` and research/RCA `researcher:` →
    `author:` renames are **owned by migration 0006**, authored under story
    0064. The visualiser server's transitional `work-item:` fallback ships with
    0064 and **must be removed by this story** in the same release that closes
    out 0070 — by then every userspace repo will have run
    `/accelerator:migrate` at least once.)
  - (Work-item `type:` → `kind:` rename is **owned by migration 0005**, authored
    under story 0063. This migration must not duplicate that rewrite — 0005 has
    already migrated the corpus by the time this migration runs in any repo.)
  - Adds the unified base fields (`type`, identity, `title`, `date`, `author`,
    `status`, `tags`, `last_updated`, `last_updated_by`, `schema_version`) with
    sensible defaults where missing. Here `type:` is ADR-0033's artifact-type
    discriminator (`work-item`, `plan`, `adr`, `note`, …) and is **distinct
    from** the work-item `kind:` subtype (story/bug/spike/epic); `kind:` is
    owned by migration 0005 and not rewritten here. Defaults applied when a
    base field is absent: `tags: []`, `schema_version: 1`,
    `last_updated`/`last_updated_by` seeded from the artifact's existing
    `date`/`author` (see the seeding requirement below), and `author` resolved
    per the notes-backfill rule for files that lack it. Where `status` is
    absent the migration leaves it unset rather than inventing a lifecycle
    state.
  - Renames the producer field `skill:` → `producer:` per ADR-0033 (distinct
    from `author:`).
  - Migrates each artifact's **own-identity** field to `id:` as a **quoted YAML
    string** per ADR-0033's identity contract: the work-item's own-identity
    `work_item_id:` → `id:`, and the ADR's own-identity `adr_id:` → `id:`. A
    **foreign** `work_item_id:` reference (one keyed `<type>_id` to point at
    another artifact — e.g. a plan's `work_item_id:` naming its parent work
    item) is left in place. The qualifiers "own-identity", "foreign", and
    "review-template alias" disambiguate the three roles `work_item_id:` plays
    across this migration; see also the alias-removal requirement below.
  - Adds the provenance bundle (`revision`, `repository`) to
    code-state-anchored artifacts (plans, codebase-research, issue-research/RCA,
    design-inventory, pr-description) and removes `git_commit` / `branch`.
  - Adds per-artifact extras per ADR-0033 / 0057.
  - Records `schema_version: 1` per artifact type (the unified schema is v1; no
    type has been bumped past 1).
  - Parses the corpus's five de-facto linkage-bearing body sections —
    `## References`, `## Dependencies`, `## Historical Context`,
    `## Related Research`, and `## Source References` (the authoritative set
    confirmed by the Implementation anchors; `## Related Documents` does not
    appear in the corpus) — and populates typed linkage frontmatter, emitting
    the typed `"doc-type:id"` reference form (never bare `"NNNN"`) and writing
    only the canonical side of each bidirectional pair (`blocks`, `supersedes`)
    per ADR-0034.
  - Classifies each body-section inference into one of two bands per the rule
    in ADR-0038 (derived from spike 0068's accuracy measurement): an inference
    is **resolved** when the parser maps it to exactly one (source-type, key,
    target-type) tuple in ADR-0034's published table with no competing
    candidate; it is **ambiguous** when zero or more than one tuple matches
    (e.g. a bare number resolvable to either a work-item or an ADR). Applies
    **resolved**-band inferences mechanically and routes **ambiguous**-band
    inferences through the interactive validation hook (ADR-0037 / ADR-0038) —
    there is no non-interactive fallback. Deterministic field renames and shape
    normalisation are not inferences and always apply mechanically.
  - Encodes the three spike-mandated parser fixes before band classification: a
    template-path blocklist (literal placeholders like `ADR-NNNN.md`), a
    tightened `\bblocks?\b` regex (so "code-block" prose is not matched), and
    `\bsibling\b` → `relates_to` (ADR-0038).
  - Disambiguates prose references by normalising onto ADR-0034's published
    (source-type, key, target-type) table — e.g. a plan's `"Source:"` line
    resolves to `parent` for a work-item target, `derived_from` for a research
    target. Genuinely ambiguous references (e.g. a bare number that could be a
    work-item or an ADR) classify `ambiguous` and route to the hook rather than
    being guessed.
  - Emits frontmatter omit-when-empty per ADR-0040 — typed-linkage and optional
    keys are written only when non-empty; never as empty `parent: ""` /
    `blocks: []` placeholders. Base fields (including `tags: []`) and
    always-valued per-type extras are always present.
  - Tolerates `pr:` references when migrating `pr-review` artifacts (the `pr`
    external-entity prefix is not yet in ADR-0034's published vocabulary — a
    separate supplementary ADR is pending — so the migration must not flag
    `target: "pr:<n>"` as broken or uncertain).
  - Seeds `last_updated` from each artifact's existing `date` value where the
    field is first being set (a mechanical schema rewrite is not a content
    update).
- Remove the transitional **review-template `work_item_id:` alias** on the
  `work-item-review` template (the own-identity alias consumed by the visualiser
  server — distinct from the **foreign** `work_item_id:` references on plans,
  which are retained per the identity bullet above) and close the 0065 dual-key
  read-path fallback that accepted both `work_item_id` and `id`
  "during the 0065→0070 transition" — this story ends that transition. The
  dual-key read path spans three visualiser-server sites that must all be
  closed: the `work-item:` fallback in `frontmatter.rs:read_ref_keys`, the
  `parent_or_legacy_id` path in `cluster_key.rs`, and the filename fallback in
  `indexer.rs` (see Implementation anchors).
- Add baseline frontmatter to existing hand-written files under `meta/notes/`:
  `type: note`, `schema_version`, an `id`/`title`/`date`/`topic` inferred from
  the filename and H1, and `author` resolved from VCS history with the
  conservative literal fallback `Unknown` where history is unavailable. The
  emitted `note` baseline must match the `note` frontmatter shape `create-note`
  (0067) produces for new notes, so migrated and newly-created notes stay
  shape-consistent.
- Dogfood against this repo's own corpus, then fix any gaps surfaced —
  including rewriting ADR-0033's own legacy `adr_id:` frontmatter to the quoted
  `id:` form (it predates the identity contract it defines).

## Acceptance Criteria

- [ ] The migration runner exits 0 against this repo's `meta/` corpus with zero
      `REFUSE` / `MALFORMED` diagnostics; every migrated file validates against
      the unified-schema validator; and an immediate re-run reports no further
      changes. Any remaining `DIVERGE` is recorded in the dogfood gap-fix log
      with a one-line rationale, and the migration report lists **zero
      un-annotated `DIVERGE` lines** — so "accepted" is a checkable condition
      (an inspectable log entry) rather than an unverifiable assertion.
- [ ] Plan files already have their foreign `work_item_id` reference (quoted)
      and research/RCA files already have `author` — guaranteed by migration
      0006 from story 0064.
      Work-item files already have `kind:` — guaranteed by migration 0005 from
      story 0063. This migration does not duplicate those rewrites.
- [ ] Each artifact's own-identity field is `id:` as a quoted string
      (own-identity `work_item_id:` → `id:` on work-items, own-identity
      `adr_id:` → `id:` on ADRs); foreign `<type>_id` references are unchanged.
- [ ] The producer field is named `producer:` (renamed from `skill:`) on every
      artifact that carried one.
- [ ] Code-state-anchored artifacts have `revision` + `repository`; no
      `git_commit` or `branch` remains.
- [ ] `schema_version: 1` is set per artifact type.
- [ ] Base fields absent from a source artifact are filled with the defaults
      enumerated in Requirements (`tags: []`, `schema_version: 1`,
      `last_updated`/`last_updated_by` seeded from `date`/`author`, `author`
      per the notes-backfill rule, `status` left unset when absent) — no field
      is filled with an ad-hoc or undocumented default.
- [ ] Typed linkage frontmatter is populated from body sections: resolved-band
      inferences applied mechanically, ambiguous-band inferences surfaced
      through the interactive hook. All linkage values use the typed
      `"doc-type:id"` form and only the canonical bidirectional side is written.
- [ ] Band classification is verified against a fixture set of references with
      their expected band (resolved vs ambiguous); and on the dogfood corpus the
      mechanically-applied wrong-rate is measured by the same procedure spike
      0068 used: draw a stratified sample of at least 150 resolved-band linkages
      across the five de-facto header types (`## References`, `## Dependencies`,
      `## Historical Context`, `## Related Research`, `## Source References`),
      classify each correct/wrong by comparing the emitted linkage against the
      source prose, and require the wrong-rate to be at or below the ≤5%
      threshold. (Where the corpus holds fewer than 150 resolved-band linkages
      in total, the sample is the full set.) The interactive session log
      (`.accelerator/state/migrations-<id>-session.jsonl`) records every
      reference routed to the hook, providing the audit trail for which
      references were classified ambiguous.
- [ ] Ambiguous-band routing has a verifiable terminal state, not just logging:
      after the interactive session every reference recorded in the session log
      reaches `APPLIED_CONFIRM` (none left in `PROMPT` / `VALIDATE_ERR` /
      `DRIFT`), and a fixture set of known-ambiguous references (e.g. a bare
      number resolvable to either a work-item or an ADR) produces the expected
      linkage after a scripted hook decision. This gives the ambiguous band —
      the higher-risk path per spike 0068's 11.3% — a correctness gate, not only
      the resolved band's ≤5% wrong-rate check.
- [ ] No empty-placeholder keys remain — optional and typed-linkage keys are
      omitted when empty per ADR-0040; `tags: []` and always-valued extras are
      retained.
- [ ] The three spike-mandated parser fixes are encoded and each exercised by a
      fixture proving the behaviour change: a literal template path like
      `ADR-NNNN.md` produces no linkage (template-path blocklist); prose
      containing "code-block" produces no `blocks` linkage (tightened
      `\bblocks?\b` regex); and a reference using "sibling" produces a
      `relates_to` linkage (`sibling` → `relates_to`).
- [ ] The visualiser server's transitional `work-item:` fallback (introduced by
      0064 in `frontmatter.rs:read_ref_keys`) is removed, along with the test
      that pinned it.
- [ ] The transitional `work_item_id:` alias on `work-item-review` and the 0065
      dual-key read-path fallback are removed. The fallback removal is confirmed
      at each of its three sites — no `work-item:` fallback remains in
      `frontmatter.rs:read_ref_keys`, no `parent_or_legacy_id` path remains in
      `cluster_key.rs`, and no filename fallback remains in `indexer.rs` (a grep
      for `parent_or_legacy_id` and the legacy keys returns nothing).
- [ ] ADR-0033's own frontmatter is rewritten from `adr_id:` to quoted `id:`.
- [ ] Existing `meta/notes/` files carry baseline frontmatter (`type: note`,
      `schema_version`, inferred identity/title/date/topic, resolved `author`).
      `author` resolution is verified on both branches: a note with VCS history
      gets the original committing author, and a note whose history is
      unavailable gets the conservative fallback value `Unknown`.
- [ ] `last_updated` is seeded from each artifact's original `date` where first
      set.
- [ ] Re-running the migration is a no-op against an already-migrated corpus:
      deterministic transforms self-detect completion, and the interactive
      subset skips any transformation already recorded in the session log keyed
      on `(artifact_path, source_anchor)` (ADR-0037 / ADR-0038).

## Open Questions

_All three prior open questions are now resolved and folded into the
requirements: (a) existing `meta/notes/` files receive baseline frontmatter;
(b) `last_updated` is seeded from the artifact's original `date`; (c) ambiguous
references are normalised onto ADR-0034's type-pair table, with genuinely
ambiguous cases routed to the interactive hook._

## Dependencies

- Blocked by: 0060 (base schema / ADR-0033), 0061 (linkage vocabulary /
  ADR-0034), 0062 (interactive-validation parameters / ADR-0038), 0063
  (work-item `kind:` rename + migration 0005), 0064 (`work_item_id` / `author`
  canonicalisation + migration 0006), 0065 (templates updated), 0066
  (review-skill frontmatter moved into templates), 0067 (`create-note` skill —
  owns the notes hand-off to this story), 0068 (spike — verdict mandates the
  interactive path), 0069 (interactive validation hooks in the migration
  runner; this story is its first consumer). **All are `done` — 0070 is
  unblocked.**
- Blocks: future visualiser-graph epic (which consumes the structured linkages
  this migration writes).
- Runtime migration ordering (distinct from the story-level blockers above,
  which only guarantee the migrations *exist*): this migration (0007) requires
  migrations 0005 (`kind:` rename) and 0006 (`work_item_id`/`author`
  canonicalisation) to have already been **applied** against the corpus before
  it runs in any repo. Its awk transforms assume `kind:` is present and foreign
  `work_item_id:` is already quoted; running out of order would mis-transform
  or refuse. The runner's ordered ledger replay (run-migrations.sh discovery
  glob + `sort -z`) enforces this sequencing.
- Cross-repo / cross-consumer coupling: removing the visualiser-server
  transitional read-path fallbacks (the `work-item:` fallback and the 0065
  dual-key path) depends on every consuming userspace repo having run
  `/accelerator:migrate` at least once before the fallback-removal release
  ships. The fallbacks exist precisely to tolerate un-migrated repos, so their
  removal cannot precede that condition; this story assumes the migrate-on-use
  contract guarantees it by the time the closing release ships.
- Pending: a supplementary ADR is expected to add the `pr:` external-entity
  prefix to ADR-0034's published vocabulary. Until it lands, this migration
  tolerates `pr:` references rather than validating them; the carve-out should
  be reconciled once that ADR is accepted.
- Related: 0057 (parent epic), 0056 (precedent for frontmatter-aware
  migration), 0067 (`create-note` — the notes baseline frontmatter this story
  emits must match the `note` schema `create-note` produces for new notes, so
  migrated and newly-created notes stay shape-consistent).

## Assumptions

- VCS revert remains the migration's safety net — no inverse migration is built
  (ADR-0023).
- Producer-side updates (0063–0067) have already landed, so artifacts created
  during/after the migration are already unified.
- The `specs/` and `global/` directories are out of scope (deferred per 0057's
  assumptions).

## Technical Notes

**Size**: XL — five distinct workstreams (net-new body-section linkage parser, interactive-migration authoring on the 0069 contract, the base-field/identity/provenance awk rewrite, a coupled Rust visualiser-server removal, and notes backfill) plus an end-to-end corpus dogfood and gap-fix pass.

**On the XL sizing (deliberate, kept whole):** the workstreams are bound by a single acceptance gate — the end-to-end dogfood, which must exercise all five together against one corpus, because the linkage parser, the awk base-field rewrite, and the notes backfill all write into the *same* files and the visualiser-server fallback removal is only safe *after* that combined rewrite produces a corpus with no legacy keys left to fall back to. Splitting the Rust removal into a follow-on story would mean shipping a release where the fallback is gone but some artifacts may still be un-migrated — the exact breakage the cross-repo coupling above guards against; the removal must land in the *same release that closes 0070*, and keeping it in the same story is the simplest way to guarantee that ordering. The notes backfill is small but shares the dogfood gate: the gap-fix pass must validate notes alongside structured artifacts, so extracting it would split that single validation pass. The XL size is therefore accepted as the cost of this being epic 0057's closing integration item, not an accumulation to be decomposed.

- The migration is numbered after the latest applied at implementation time
  (next after migration 0006 — likely 0007, but verify against the applied
  ledger; do not hard-code).
- Reconcile the migration state-file path at implementation time: ADR-0023
  references `meta/.migrations-applied` while ADR-0037 / ADR-0038 reference
  `.accelerator/state/migrations-applied` — the state directory appears to have
  moved since ADR-0023.
- The interactive session log is pinned by ADR-0038:
  `.accelerator/state/migrations-<migration-id>-session.jsonl`, line-delimited
  JSON, one record per inferential transformation, appended atomically
  (temp-then-rename), keyed for resumability on `(artifact_path,
  source_anchor)`.
- The body-section parser is shared with 0068's spike prototype where
  practical, with the three spike fixes encoded before band classification.
- All mutations use atomic temp-file-then-rename, behind a clean-tree
  pre-flight (ADR-0023).

### Implementation anchors

- **Runner & state ledger**: `skills/config/migrate/scripts/run-migrations.sh`. The applied
  ledger is at `.accelerator/state/migrations-applied` (run-migrations.sh:39); the runner also
  warns when an ID appears in both that file and the legacy `.migrations-applied`
  (run-migrations.sh:198). This resolves the state-path reconciliation note above — the
  directory has already moved to `.accelerator/state/`; ADR-0023's `meta/.migrations-applied`
  is the legacy path the runner now bridges, not a live target.
- **Numbering**: 0006 is the current head in `skills/config/migrate/migrations/`, so this
  migration is `0007-*`. Discovery globs `[0-9][0-9][0-9][0-9]-*.sh` then `sort -z`
  (run-migrations.sh:161-164). Still verify against the applied ledger rather than hard-coding.
- **Migration template to follow**: `migrations/0006-canonicalise-work-item-id-and-author.sh`
  is the closest precedent — an embedded awk state machine separating the frontmatter fence
  region from the pre-first-`## H2` body region, writing via `atomic_write` only when `cmp -s`
  shows a change (0006:271-280), emitting `0006-DIVERGE`/`0006-REFUSE`/`0006-MALFORMED`
  diagnostics. Mechanical idempotency is free: `rewrite_file` early-returns when no target keys
  match (0006:254-256). For "stays pending" semantics, the `MIGRATION_RESULT: no_op_pending`
  sentinel (run-migrations.sh:280-291) is distinct from idempotent re-runs.
- **Interactive opt-in**: a migration declares `# INTERACTIVE: yes` in its first 5 lines
  (`interactive-lib.sh:21-33`); the runner dispatches via `is_interactive_migration`
  (run-migrations.sh:259-270). Transport is two named FIFOs (`migrations-<id>-r2m.fifo` /
  `-m2r.fifo`); the TAB-separated frame protocol is defined in
  `scripts/interactive-protocol.sh:9-46`. Per-transformation path:
  `PROMPT → DECIDE → (VALIDATE_ERR loop) → RECORDED → APPLY → APPLIED_CONFIRM`. Resume is keyed
  on `transformation_key`, replayed from the JSONL session log (`interactive-lib.sh:39-124`);
  a recorded transformation that no longer matches emits `DRIFT` and re-prompts. This is ~588
  lines of reusable 0069 contract — consume it, don't re-author.
- **Body-section parser is net-new**: the 0068 spike prototype (`/tmp/spike-0068/parser.py`,
  ~280 lines) was throwaway and never committed, so "shared with 0068's spike prototype where
  practical" above resolves to: build fresh, encoding the spike's failure-pattern catalogue.
  The corpus uses five de-facto headers, not `## Related Documents` (which never appears):
  `## References` (207), `## Dependencies` (62), `## Historical Context` (40),
  `## Related Research` (38), `## Source References` (29). The three cheap fixes target the
  measured 11.3% wrong-rate that drove the interactive verdict.
- **Visualiser-server removal**: the transitional `work-item:` fallback is
  `frontmatter.rs:334-341` (doc-comment at 299-304 / 327-329 marks it "removed in the release
  that closes story 0070"), pinned by test
  `read_ref_keys_reads_legacy_work_item_key_via_transitional_fallback` (frontmatter.rs:470-477).
  The 0065 dual-key read path also touches `cluster_key.rs:119-131` (`parent_or_legacy_id`) and
  `indexer.rs:1216-1233` (filename fallback) — audit these when closing the 0065 transition.

## Drafting Notes

- The strategy is committed to the interactive path (ADR-0037 / 0038); the
  earlier "interactive or post-run report" fork is removed because spike 0068's
  verdict and 0069's shipping resolved it.
- Absorbed three cleanup obligations that 0057 / 0065 / 0066 conditioned on
  "0070 having run": dropping the `work-item-review` `work_item_id:` alias,
  closing the 0065 dual-key read fallback, and fixing ADR-0033's own legacy
  `adr_id:` frontmatter. These were implied by the epic but not previously
  surfaced as 0070 acceptance criteria.
- Added the `skill:` → `producer:` rename and the own-identity `id:` migration,
  which were mandated by ADR-0033 but absent from earlier drafts of this story.
- Decisions recorded this session: existing notes receive baseline frontmatter
  (not skipped); `last_updated` is seeded from the original `date`. Neither is
  settled by an ADR — both are this story's calls and may be revisited.
- Left the file's own frontmatter in legacy shape (`work_item_id:`) rather than
  hand-converting it — converting it is precisely this migration's job, so
  reshaping it here would pre-empt the work being specified.
- Review-1 (REVISE) decisions folded in: clarified that `type:` (ADR-0033
  artifact discriminator) and `kind:` (work-item subtype) are distinct fields,
  not a contradiction; defined the resolved/ambiguous band vocabulary inline;
  surfaced the migration-ordering (0005/0006 applied first) and cross-repo
  fallback-removal couplings in Dependencies; kept the XL scope whole with an
  explicit indivisibility justification (single dogfood gate; fallback removal
  must land in the same release); tightened the headline, band-oracle,
  parser-fix, notes-author, defaults, and per-site 0065-removal acceptance
  criteria. The notes-author conservative fallback was set to the literal
  `Unknown` (this story's call, not settled by an ADR).

## References

- Source: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Decisions: `meta/decisions/ADR-0023-meta-directory-migration-framework.md`,
  `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`,
  `meta/decisions/ADR-0034-typed-linkage-vocabulary.md`,
  `meta/decisions/ADR-0037-optional-interactive-contract-supplement-to-adr-0023.md`,
  `meta/decisions/ADR-0038-interactive-validation-parameters-for-unified-schema-linkage-migration.md`,
  `meta/decisions/ADR-0040-omit-when-empty-frontmatter-emission-supplement-to-adr-0033.md`
- Research: `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`
- Related: 0056, 0057, 0060, 0061, 0062, 0063, 0064, 0065, 0066, 0067, 0068, 0069
