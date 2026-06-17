---
type: work-item
id: "0114"
title: "Migration 0007 Incomplete Mechanical Normalisation"
date: "2026-06-17T21:06:54+00:00"
author: Toby Clemson
producer: create-work-item
status: ready
kind: bug
priority: high
parent: "work-item:0057"
relates_to: ["work-item:0070"]
source: "issue-research:2026-06-17-migration-0007-incomplete-mechanical-normalisation"
tags: [migrate, frontmatter, validator, unified-schema, "0007"]
last_updated: "2026-06-18T00:22:06+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0114: Migration 0007 Incomplete Mechanical Normalisation

**Kind**: Bug
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Migration `0007-unify-meta-corpus-frontmatter` cannot pass its own structural
validation gate on real-world corpora. Its mechanical normalisation passes
(fence-less backfill + the awk rewrite) are incomplete relative to the rule set
its validator (`validate-corpus-frontmatter.sh`) enforces, so five
schema-conformance transforms (plus one out-of-scope directory addition) are
never applied. On any corpus containing the
unhandled shapes the migration mutates files, fails the gate, exits before its
interactive harness, and — because it is never recorded as applied — repeats
identically on every re-run, permanently blocking the upgrade.

## Context

`validate-corpus-frontmatter.sh` is the contract the migration must satisfy; the
backfill + awk rewrite are supposed to bring every in-scope file into compliance
before the structural gate runs. Five required transforms are missing or
under-scoped, and one secondary directory (`meta/docs/`) — freeform docs the
plugin does not own — is currently swept in by the migration despite having no
schema type, when it should be excluded as out of scope. Six numbered gaps in
total (see Requirements).

Observed first on a downstream repo running plugin `1.23.0-pre.1`:
migrations 0001 and 0003–0006 applied cleanly; 0007 mutated ~147 `meta/*.md`
files (correctly) then reported **197 violations** across ~136 files and
aborted. Full root-cause analysis in
`meta/research/issues/2026-06-17-migration-0007-incomplete-mechanical-normalisation.md`.

The structural gate aborting before `harness_run` is **correct behaviour** — you
do not want to run interactive body-linkage on a structurally-broken corpus, and
the interactive step would not fix any of these violations anyway. The defect is
upstream, in the mechanical passes; gate reordering is explicitly out of scope.

Two orientation notes for the sections below. First, the validator's violation
codes (`INVALID-TYPE`, `FORBIDDEN-OWN-ID`, `OBSOLETE-LEGACY-KEY`, `MISSING-EXTRA`,
`BAD-LINKAGE-SHAPE`) are its rule names, defined in
`validate-corpus-frontmatter.sh` and tabulated in the linked RCA. Second, two
distinct PR artefact types are in play: `pr-description` files live in `meta/prs/`,
while `pr-review` files live in `meta/reviews/prs/`; both can carry the forbidden
`pr_title:` key, but only `pr-review` can carry `review_pass:`.

## Requirements

**Reproduction**

Environment: plugin `1.23.0-pre.1`, with migrations 0001 and 0003–0006 already
applied. Run `/accelerator:migrate` against a corpus containing any of:

1. Empty-`type:` PR descriptions under `meta/prs/` (e.g. `meta/prs/240-description.md`).
2. A note with a `ticket: "PROJ-XXXX"` key and no `topic:`.
3. `pr-review` / `pr-description` files carrying `pr_title:` (and `review_pass:`
   on pr-reviews), in both variants — *with* and *without* a pre-existing
   `title:`.
4. A linkage value of the shape `target: "PR #416"`.
5. A freeform doc under `meta/docs/` (e.g. `meta/docs/logging-guide.md`) that has
   no schema type and so trips an `INVALID-TYPE` violation. (This is the largest
   single contributor to the original failure — see the RCA's violation
   breakdown.)

**Actual behaviour**

`run_rewrite` normalises what it can but skips the untypeable `meta/prs/` files
and passes the forbidden/legacy keys and non-canonical linkage shapes through
unchanged. `self_validate_structural` then reports ~197 violations and the script
exits 1 (under `set -euo pipefail`) before `harness_run`. The runner does not
record 0007 as applied, so re-runs repeat from scratch.

**Expected behaviour**

After 0007's mechanical passes the in-scope corpus is validator-clean (modulo
genuinely-ambiguous body-section linkage, which the interactive harness owns), so
`self_validate_structural` passes and the migration completes.

**Gaps to close** (each pinned in the RCA):

1. `pr-description` path inference: add the `meta/prs/` arm to **all three**
   type-inference sites — both `infer_type_from_path` copies (migration +
   validator) gain `*/prs/*` → `pr-description`, and the awk `path_to_typed`
   gains `meta/prs/` → `pr-description`.
2. Drop the schema's `forbidden_own_id_key`s (`pr_title`; `review_pass` on
   pr-review) — driven by the schema TSV column, not hard-coded. When dropping
   `pr_title`, fold its value into `title:` if the file has no `title:`,
   otherwise discard it.
3. Drop obsolete legacy keys `ticket` / `ticket_id` on any type. This is
   complementary to migration 0001, which renames `ticket_id:` → `work_item_id:`
   inside `meta/tickets/`; 0007 runs after 0001 and only discards the stray
   legacy keys 0001 leaves untouched elsewhere.
4. Backfill required type-extras on already-fenced files, not just fence-less
   backfill. Each default is applied **only when the key is absent** (existing
   values are never overwritten): `topic` ← the title (as the fence-less note
   backfill already does); `pr_number` ← the leading number parsed from the
   filename (`pr-430-review.md` → `430`); `review_number` ← `1`.
5. Coerce non-canonical linkage shapes `PR #N` / `#N` → `pr:N`.
6. Add `meta/docs/` to `out_of_scope` (freeform docs the plugin does not own; no
   schema type) so the migration and validator both skip it.

## Acceptance Criteria

- [ ] Given a corpus with empty-`type:` files under `meta/prs/`, when 0007 runs,
      then they are typed `pr-description` and pass validation.
- [ ] Given files carrying `pr_title:` (and `review_pass:` on pr-reviews), when
      0007 runs, then those forbidden own-id keys are removed. Verified to be
      schema-driven by a fixture whose forbidden key is declared **only** in the
      schema TSV's `forbidden_own_id_key` column and appears in no hard-coded
      drop list, so a hard-coded implementation would fail the test.
- [ ] Given a `pr-description`/`pr-review` with `pr_title:` and no `title:`,
      when 0007 runs, then `pr_title`'s value is promoted to `title:` and the
      `pr_title` key is removed; given the same file with an existing `title:`,
      `pr_title` is dropped and `title:` is left unchanged.
- [ ] Given a note with `ticket: "PROJ-1234"` and a non-note type carrying
      `ticket_id:`, when 0007 runs, then both the `ticket` and `ticket_id` keys
      are dropped regardless of type or value; and given a `meta/tickets/` file
      with `ticket_id:`, that key is left for migration 0001's scoped rename
      rather than double-handled.
- [ ] Given a fenced note missing `topic`, when 0007 runs, then `topic` is
      backfilled from the title.
- [ ] Given a fenced `pr-review` whose filename encodes the PR number (e.g.
      `pr-430-review.md`) and which lacks `pr_number`/`review_number`, when 0007
      runs, then `pr_number` is set to `430` and `review_number` is set to `1`.
- [ ] Given a `meta/docs/` file, when 0007 runs, then it is skipped as out of
      scope: its bytes are left unchanged and it produces no `INVALID-TYPE`
      violation.
- [ ] Given `target: "PR #416"`, when 0007 runs, then it becomes
      `target: "pr:416"`.
- [ ] Given a corpus containing **every** reproduction shape above (the fixture
      corpus below), when 0007's mechanical passes run, then
      `validate-corpus-frontmatter.sh` exits 0 and `harness_run` is reached.
      (Binding to the fixture corpus is deliberate: the dogfood corpus lacks
      `meta/prs/`, `ticket:` notes, and `PR #N` linkage, so passing against it
      alone would not exercise the fix.)
- [ ] `test-migrate-0007.sh` gains a fixture for **every** acceptance outcome
      above and they pass in CI — at minimum: an empty-`type:` `meta/prs/` file;
      a `ticket:`-with-no-`topic:` note; a non-note type carrying `ticket_id:`; a
      `meta/tickets/` file with `ticket_id:` (left for 0001);
      a `pr_title:`+`review_pass:` pr-review
      *with* and *without* a pre-existing `title:`; a fenced note missing
      `topic`; a fenced `pr-review` whose filename encodes the PR number; a
      `target: "PR #416"` linkage; a schema-TSV-only forbidden key; and a
      `meta/docs/` file.
- [ ] A regression guard exists asserting that `run_backfill + run_rewrite`
      leaves zero validator violations on the fixture corpus above (which
      contains every reproduction shape) — the "rewrite output is
      validator-clean by construction" invariant.

## Open Questions

All resolved during refinement (2026-06-17); decisions folded into Requirements
and Acceptance Criteria:

- `meta/docs/` → added to `out_of_scope` (freeform, plugin-unowned; no schema
  type).
- `review_number` / `pr_number` on legacy reviews → `pr_number` derived from the
  filename's leading number; `review_number` defaults to `1`.
- Dropped `pr_title` → folded into `title:` when `title:` is absent, otherwise
  discarded.

## Dependencies

- Blocked by: none.
- Blocks: completion of the 1.23 migration for any downstream repo whose corpus
  contains the unhandled shapes.
- Internal coupling: `infer_type_from_path` is triplicated across the migration,
  the validator, and the awk `path_to_typed` — all three must change in lockstep
  (or be single-sourced) or the fix re-introduces the exact drift that caused
  this bug.
- Relates to: 0070 (built migration 0007); under epic 0057 (unified artifact
  frontmatter and typed cross-linking).

## Assumptions

- This corpus shape (PR descriptions in `meta/prs/`, `ticket:` notes, `pr_title:`
  reviews, `PR #N` linkage) is common enough across downstream repos to justify
  `high` priority; downgrade if it turns out rare.
- The six gaps (five transforms plus the `meta/docs/` out-of-scope addition) are
  each deterministic and judgment-free once the three Open Questions are
  answered — no new interactive prompts are required.
- The schema TSV (`scripts/templates-schema.tsv`) already declares
  `forbidden_own_id_key` for `pr-description`/`pr-review` and lists
  `topic`/`pr_number`/`review_number` as required extras. The schema-driven
  transforms no-op silently if this is not true, so it is a precondition of the
  fix rather than something the fix establishes — verify the column exists before
  editing the awk.

## Technical Notes

**Size**: M — one awk (`0007-frontmatter-rewrite.awk`: schema-driven forbidden-key drop, `ticket`/`ticket_id` drop, required-extras backfill, `pr_title`→`title` fold, `PR #N`→`pr:N` rule) plus the driver and a single-sourced `infer_type_from_path` shared with the validator (three call sites), and an in-memory fixture corpus in `test-migrate-0007.sh`. Bounded to the migrate subsystem + validator; no cross-component reach. The `extras_for_type` `-f5`/`-f4` off-by-one is the one trap to dodge.

Key code references (from the RCA):

- `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`
  - `infer_type_from_path` (`:82-101`) — add `*/prs/*` arm; decide `meta/docs/`.
  - `own_id_key_for_type` (`:57-63`) — only knows work-item/adr; the forbidden
    drop should instead be schema-driven via the awk.
  - `out_of_scope` (`:103-108`) — candidate home for `meta/docs/`.
  - Orchestration (`:629-646`) — the structural gate is correctly placed; do
    not reorder it.
- `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk`
  - Key-rewrite rules (`:228-304`) — add forbidden-key drop (pass the schema's
    `forbidden_own_id_key` column in as `-v forbidden=…`), `ticket`/`ticket_id`
    drop (mirror the `branch:` handling at `:241`), and required-extra backfill
    in the closing-fence block (`:185-225`).
  - `normalize_paths` / `normalize_bare` (`:95-143`) — add a `PR #N` / `#N` →
    `pr:N` token rule.
  - `path_to_typed` (`:71-90`) — add `meta/prs/` → `pr-description`.
- `scripts/validate-corpus-frontmatter.sh`
  - `infer_type_from_path` (`:79-98`) — the duplicated copy; same `meta/prs/`
    gap. Prefer single-sourcing the two copies to remove the drift class.
- `scripts/templates-schema.tsv` — declares `pr-description` (forbids
  `pr_title`) and `pr-review` (forbids `pr_title review_pass`); `note`,
  `codebase-research`, `issue-research` require `topic`.
- `scripts/frontmatter-emission-rules.sh:74` — `FM_OPTIONAL_EXTRAS` excludes
  `topic`/`review_number`/`pr_number`, so they are required extras.

Prevention (fold into the fix as regression guards): single-source
`infer_type_from_path` between migration and validator; add the
validator-clean-by-construction property test.

Refinement findings (codebase analysis, 2026-06-17):

- **Type-inference is triplicated, not duplicated.** Besides the two
  byte-identical bash `infer_type_from_path` copies (migration `:82-101`,
  validator `validate-corpus-frontmatter.sh:79-98`), the awk `path_to_typed`
  (`0007-frontmatter-rewrite.awk:71-90`) is a third independent encoding (regex
  arms + id derivation). All three need the `meta/prs/` → `pr-description` arm.
  A shared-sourcing channel already exists: the migration sources helpers from
  `$PLUGIN_ROOT/scripts/` (`:9-12`) and the validator sources
  `frontmatter-emission-rules.sh` as an explicit "single source" — so a
  `scripts/`-level bash helper can be sourced by both, and the awk can take the
  type as a `-v` computed shell-side (the driver already passes `-v type`).
- **Latent off-by-one to avoid.** `extras_for_type` (`0007-…sh:52`) cuts `-f5`,
  but `extras` is column 4 (column 5 is `status_vocab`). It is currently unused;
  the required-extras backfill is exactly what would call it — use `cut -f4`
  (and treat the `-` sentinel as empty) so the backfill does not feed the status
  vocab in as the extras list.
- **`forbidden_own_id_key` (TSV column 6) is read by nothing in the migration.**
  It hard-codes own-id via `own_id_key_for_type` (`:57-63`, work-item/adr only).
  Add `forbidden_keys_for_type() { schema_row "$1" | cut -f6; }`, thread it into
  the awk as `-v forbidden=`, add an awk `is_forbidden(k)` helper (mirroring
  `in_vocab` at awk `:145-149`), and place a drop arm before the catch-all
  `print $0` (awk `:303`), modelled on the unconditional `branch:` drop (awk
  `:241`).
- **`PR #N` → `pr:N`** is cleanest as a third sibling of
  `normalize_paths`/`normalize_bare`, chained
  `normalize_bare(normalize_pr_ref(normalize_paths(val)))` at awk `:291` so the
  `#` is consumed before the bare-number loop; the output passes the validator's
  `FM_TYPED_REF_RE` (`pr:` is a tolerated prefix).
- **`paths.prs` confirmed.** PR descriptions live at `meta/prs/` by config
  default (`config-defaults.sh:30`, `paths.prs` → `meta/prs`; review PRs at
  `paths.review_prs` → `meta/reviews/prs`), so typing `meta/prs/*` as
  `pr-description` is the documented layout, not a guess.
- **Tests have no on-disk fixtures.** `test-migrate-0007.sh` builds throwaway
  corpora via `mktemp -d`, `git_init`s them, runs `run_0007`, then gates on the
  validator over `$REPO/meta`. Add a coverage block (model on the path-shape
  block) seeding `meta/prs/`, a `ticket:`/no-`topic:` note, a
  `pr_title:`/`review_pass:` pr-review, and a `target: "PR #416"` linkage, ending
  with the validator-clean gate.

## Drafting Notes

- Priority set to `high` on the basis that the migration is permanently stuck
  for affected downstream repos (blocks the 1.23 upgrade); revisit if the
  triggering corpus shape is rare.
- Parented to the in-progress unified-schema epic 0057 rather than reopening the
  `done` 0070; flag if it should stand alone.
- Treated the structural-gate-aborts-before-harness behaviour as correct (per
  the RCA) and excluded gate reordering from scope.
- Folded the RCA's prevention items into the acceptance criteria as the fix's
  regression guard rather than leaving them as loose advice. These ship as one
  unit with the fix: single-sourcing the triplicated `infer_type_from_path` is
  the cleanest way to satisfy gap 1 across all three sites at once, so it is not
  a separable fast-follow. If the consolidation unexpectedly outgrows the `M`
  estimate, split it out so the high-priority unblock is not gated on the
  refactor.
- Producer-side emission (work item 0103) and validator blind-spots (0105) are
  separate, already-`done` efforts and are deliberately out of scope here.

## References

- Source: `meta/research/issues/2026-06-17-migration-0007-incomplete-mechanical-normalisation.md`
- Related: 0070 (shipped migration 0007), 0057 (parent epic), 0103, 0105
