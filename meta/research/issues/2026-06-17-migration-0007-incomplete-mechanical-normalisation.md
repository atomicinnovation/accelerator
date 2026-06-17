---
type: issue-research
id: "2026-06-17-migration-0007-incomplete-mechanical-normalisation"
title: "Investigation: Migration 0007 fails self-validation on real corpora (incomplete mechanical normalisation)"
date: "2026-06-17T20:59:08+00:00"
author: "Toby Clemson"
producer: research-issue
status: complete
topic: "Why migration 0007 cannot pass its own structural validation gate on a real meta/ corpus"
tags: [research, debugging, migrate, 0007, frontmatter, validator]
revision: "9d0bc5f4847178976ab8e996de31a60b6308f027"
repository: "accelerator"
last_updated: "2026-06-17T20:59:08+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Investigation: Migration 0007 fails self-validation on real corpora (incomplete mechanical normalisation)

**Date**: 2026-06-17 20:59 UTC
**Author**: Toby Clemson
**Git Commit**: 9d0bc5f4847178976ab8e996de31a60b6308f027
**Branch**: HEAD (detached)
**Repository**: accelerator

## Issue Description

Running `/accelerator:migrate` against a downstream repo
applies migrations 0001 and 0003–0006 cleanly, but `0007-unify-meta-corpus-frontmatter`
aborts with **197 frontmatter violations** at its `self_validate_structural`
gate. The migration mutates ~147 `meta/*.md` files during its backfill/rewrite
passes (correctly), then its own validator rejects ~136 files and the script
exits 1 before reaching the interactive harness. Because 0007 is never recorded
as applied, a re-run repeats from scratch (it is idempotent), so the repo is
stuck — 0007 can never complete on this corpus as shipped.

Violation breakdown reported by the user:

| Count | Code | Surface cause |
|-------|------|---------------|
| 76 | `INVALID-TYPE '<absent>'` | `meta/prs/*` (127 PR-descriptions) + `meta/docs/*` can't be typed |
| 56 | `FORBIDDEN-OWN-ID 'pr_title'` | PR review/description files carry `pr_title:` |
| 46 | `OBSOLETE-LEGACY-KEY 'ticket'` | Notes etc. still carry `ticket: "PROJ-XXXX"` |
| 14 | `MISSING-EXTRA` | Notes missing `topic`; some reviews missing `review_number`/`pr_number` |
| 5 | `BAD-LINKAGE-SHAPE` | `target: "PR #416"` instead of `target: "pr:416"` |

## Input Classification

Mixed — a structured failure log (validator violation codes + counts) plus a
behavioural description of the migration aborting.

## Affected Components

- `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh:82-101`
  — `infer_type_from_path`: no arm for `*/prs/*` (PR descriptions) and no arm
  for `meta/docs/*`.
- `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh:57-63`
  — `own_id_key_for_type`: only knows `work-item`/`adr`; returns `""` for
  `pr-description`/`pr-review`, so their forbidden `pr_title`/`review_pass`
  keys are never renamed or dropped.
- `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh:103-108`
  — `out_of_scope`: excludes only `specs`/`talks`/`global`, so `meta/docs/` is
  currently swept in by the migration but is untypeable.
- `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk:228-304` — the
  awk key-rewrite rules: no rule strips forbidden own-id keys, no rule drops
  obsolete legacy keys (`ticket`/`ticket_id`), no rule backfills required
  type-extras (`topic`/`review_number`/`pr_number`) on already-fenced files.
- `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk:95-143` —
  `normalize_paths`/`normalize_bare`: handle only `"meta/…md"` path tokens and
  bare `"NNNN"` numbers; the `"PR #416"` / `#416` shapes match neither.
- `scripts/validate-corpus-frontmatter.sh:235-405` — `validate_file`: the rule
  set the rewrite must satisfy. The migration's `infer_type_from_path`
  (`:79-98`) duplicates the same `meta/prs/` gap.
- `scripts/templates-schema.tsv` — declares `pr-description` (forbids
  `pr_title`) and `pr-review` (forbids `pr_title review_pass`); `note`,
  `codebase-research`, `issue-research` require `topic`.
- `scripts/frontmatter-emission-rules.sh:74` — `FM_OPTIONAL_EXTRAS` does **not**
  include `topic`, `review_number`, or `pr_number`, so they are required.
- `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh:629-646`
  — orchestration: backfill/rewrite/validate run inside `{ … } >&2` under
  `set -euo pipefail`; the structural gate's non-zero exit aborts before
  `harness_run`.

## Timeline / Reproduction

1. `run-migrations.sh` invokes 0007.
2. `precondition_prepass` passes (no REFUSEs).
3. `run_backfill` fences any fence-less file (correct).
4. `run_rewrite` runs the awk over every fenced in-scope file. For files it can
   type it normalises base fields, identity, provenance, status, and linkage
   path/bare shapes — and writes them (≈147 files mutated).
5. `self_validate_structural` (line 640) runs `validate-corpus-frontmatter.sh`
   over the whole in-scope corpus. It finds 197 violations and exits 1.
6. `set -e` propagates the non-zero exit; the script aborts **before**
   `harness_run` (line 646) and before `self_validate_referential` (line 651).
7. The runner does not record 0007 as applied → re-run repeats step 1.

## Hypotheses

### Hypothesis 1: The rewrite's normalisation is incomplete relative to the validator's rule set
- **Evidence for**: Every violation category maps to a validator rule that has
  **no corresponding transform** in the awk/script:
  - `FORBIDDEN-OWN-ID 'pr_title'`: validator forbids it
    (`validate-corpus-frontmatter.sh:323-328`, schema cols for `pr-description`
    /`pr-review`), but the awk only renames `own_id_key` (work_item_id/adr_id)
    to `id` (`0007-frontmatter-rewrite.awk:247-250`); `pr_title` falls through
    to the catch-all `print $0` (`:303`). `own_id_key_for_type` returns `""`
    for these types (`0007-…sh:57-63`), so nothing touches it.
  - `OBSOLETE-LEGACY-KEY 'ticket'`: validator forbids `ticket`/`ticket_id` on
    any type (`validate-corpus-frontmatter.sh:47,334-338`), but the awk has no
    drop rule — `ticket:` passes through via `:303`. Migration 0001 only renames
    `ticket_id:`→`work_item_id:` **inside `meta/tickets/`**
    (`0001-…sh:56-69`); a JIRA-style `ticket: "PROJ-1234"` on a note elsewhere is
    never touched by any migration.
  - `MISSING-EXTRA 'topic'/'review_number'/'pr_number'`: validator requires
    non-optional extras (`validate-corpus-frontmatter.sh:341-344`;
    `frontmatter-emission-rules.sh:74` omits these from `FM_OPTIONAL_EXTRAS`).
    The awk's closing-fence block emits **base** fields only
    (`0007-frontmatter-rewrite.awk:185-225`) — it never backfills type-extras on
    already-fenced files. `topic` is only added for *fence-less* notes during
    backfill (`0007-…sh:306-310`).
  - `BAD-LINKAGE-SHAPE 'PR #416'`: validator demands a quoted `doc-type:id` ref
    (`validate-corpus-frontmatter.sh:359-405`, `FM_TYPED_REF_RE`).
    `normalize_paths`/`normalize_bare` only coerce `"meta/…md"` and `"NNNN"`
    (`0007-frontmatter-rewrite.awk:95-143`); `"PR #416"` matches neither and is
    emitted verbatim.
- **Evidence against**: None. The transforms are simply absent.
- **Verdict**: **Confirmed.**

### Hypothesis 2: `pr-description` is a first-class schema type with no path-inference arm; `meta/prs/` and `meta/docs/` are unmapped
- **Evidence for**: `templates-schema.tsv` declares `pr-description.md →
  pr-description`. But **neither** `infer_type_from_path` (migration `:82-101`
  *and* validator `:79-98`) nor `path_to_typed` (`0007-frontmatter-rewrite.awk:71-90`)
  has a `*/prs/*` → `pr-description` arm — only `*/reviews/prs/*` → `pr-review`.
  Downstream PR descriptions live in `meta/prs/` with an **empty** `type:` field
  (user confirmed `type:` with nothing after it). Empty `type:` → falls to
  path-inference → empty → `rewrite_file` returns early (`0007-…sh:351-353`) →
  file untouched → validator sees absent type → `INVALID-TYPE` (76 of these:
  127 PR files minus those already typed, plus `meta/docs/logging-guide.md`).
  `meta/docs/` is neither a schema type, a path-inference arm, nor in
  `out_of_scope` (`0007-…sh:103-108`) — so the migration currently sweeps it in
  even though it is untypeable.
- **Evidence against**: None.
- **Verdict**: **Confirmed.** This is the largest single bucket and is a genuine
  coverage gap, not corpus rot.

### Hypothesis 3: The structural gate aborts the interactive path, so 0007 can never finish
- **Evidence for**: The pre-pass→backfill→rewrite→validate block is wrapped in
  `{ … } >&2` under `set -euo pipefail` (`0007-…sh:629-644`).
  `self_validate_structural` (`:451-460`) runs `bash "$VALIDATOR"` which exits 1
  on any violation; `set -e` propagates it, so `harness_run` (`:646`) never runs.
- **Evidence against**: This gate is **working as designed** — you would not want
  to run interactive body-linkage on a structurally-broken corpus, and the
  interactive step only resolves ambiguous **body-section** linkage references
  (`migration_emit_transformations`, `:509-545`); it would *not* fix any of the
  197 violations. Even if the gate were bypassed, `self_validate_referential`
  (`:651`) would still fail. So the gate is a symptom surface, not the root
  cause.
- **Verdict**: **Inconclusive as a root cause / Confirmed as correct behaviour.**
  The fix belongs upstream in the mechanical passes, not in the gate ordering.

## Root Cause

Migration 0007's **mechanical normalisation passes are incomplete relative to
the rule set its own validator enforces.** `validate-corpus-frontmatter.sh` is
the contract; the backfill + awk rewrite are supposed to bring every in-scope
file into compliance before the gate. Five distinct transforms required by the
contract are missing or under-scoped:

1. **`pr-description` path inference is absent.** `infer_type_from_path` (both
   the migration copy at `0007-…sh:82-101` and the validator copy at
   `validate-corpus-frontmatter.sh:79-98`) has no `*/prs/*` arm, and
   `path_to_typed` has no `meta/prs/` arm. Empty-`type:` PR descriptions in
   `meta/prs/` are skipped by the rewrite and rejected by the validator.
2. **Forbidden own-id keys are never stripped.** The awk only renames the
   per-type *own-id* key (`work_item_id`/`adr_id`) to `id`; it has no rule for
   the schema's `forbidden_own_id_key` column, so `pr_title` (pr-description,
   pr-review) and `review_pass` (pr-review) survive.
3. **Obsolete legacy keys are never dropped.** No rule removes `ticket`/
   `ticket_id`; migration 0001 only handled `ticket_id:` inside `meta/tickets/`.
4. **Required type-extras are not backfilled on fenced files.** The closing-fence
   block emits base fields only; `topic`/`review_number`/`pr_number` are never
   added to already-fenced notes/reviews that lack them.
5. **Non-canonical linkage shapes aren't coerced.** `"PR #416"`/`#416` is not
   recognised by `normalize_paths`/`normalize_bare`, so it never becomes
   `"pr:416"`.

Secondary: `meta/docs/` is currently swept in by the migration but has no schema
type — it must be either added to `out_of_scope` or mapped to a type.

## Causal Chain

1. Downstream corpus contains PR descriptions in `meta/prs/` (empty `type:`),
   notes with `ticket:` JIRA keys, reviews/descriptions with `pr_title:`, notes
   without `topic:`, and a `target: "PR #416"` linkage — all shapes the shipped
   migration never anticipated.
2. `run_rewrite` types and normalises what it can, but skips `meta/prs/`
   (untypeable) and passes the forbidden/legacy keys + odd linkage shapes
   through unchanged.
3. `self_validate_structural` evaluates the validator's full rule set against
   the partially-normalised corpus and finds 197 residual violations.
4. The validator exits 1; `set -e` aborts the migration before the interactive
   harness; 0007 is not recorded applied.
5. Re-run repeats identically — permanent stall.

## Contributing Factors

- **Two hand-maintained copies of `infer_type_from_path`** (migration and
  validator) drifted together but both omit `pr-description`/`meta/prs/` — a
  single shared source would have surfaced the gap once.
- The migration was validated against the dogfood corpus (this repo), which
  apparently has no `meta/prs/`, no bare `ticket:` notes, and no `PR #N` linkage
  — so these shapes were never exercised. (See `test-migrate-0007.sh` / fixtures.)
- The schema declares `pr-description` and its forbidden key but nothing in the
  migration consumes the `forbidden_own_id_key` column — the schema and the
  transform diverged.

## Fix Options

| Option | Description | Risk | Effort |
|--------|-------------|------|--------|
| A | **Complete the mechanical passes** to satisfy every validator rule: add `pr-description` (and decide `meta/docs/`) to both `infer_type_from_path` copies + `path_to_typed`; add awk rules to (a) drop the schema's `forbidden_own_id_key`s, (b) drop `ticket`/`ticket_id`, (c) backfill required type-extras on fenced files, (d) coerce `PR #N`/`#N` → `pr:N`. | Med | Med–High |
| B | Same as A but **single-source `infer_type_from_path`** into one shared script consumed by both migration and validator, removing the drift class entirely. | Med | High |
| C | Make the structural gate **non-fatal / advisory** and let the interactive harness drive remediation of residual violations. | High | Med |
| D | Pre-flight **REFUSE** with a clear remediation message when untypeable/forbidden shapes are detected, requiring manual cleanup first. | Low | Low |

## Recommended Fix

**Option A**, with the `infer_type_from_path` consolidation from **B** folded in
if cheap. Each missing transform is deterministic and judgment-free:

- `pr-description`: add `*/prs/*` → `pr-description` to both
  `infer_type_from_path` copies and `meta/prs/` → `pr-description` to
  `path_to_typed`. `pr-description` is `code_state_anchored=yes`, so the existing
  anchored backfill already supplies `revision`/`repository`.
- `pr_title`/`review_pass`: drive a **drop** off the schema's
  `forbidden_own_id_key` column (pass it into the awk as `-v forbidden=…` and add
  a key rule that `next`s on a match), so it stays schema-driven rather than
  hard-coded.
- `ticket`/`ticket_id`: add an unconditional drop rule (mirror the `branch:`
  handling at `0007-frontmatter-rewrite.awk:241`).
- `topic`/`review_number`/`pr_number`: extend the closing-fence emission to add
  any absent **required** extra for the type (sourced from the schema `extras`
  column minus `FM_OPTIONAL_EXTRAS`). `topic` can default to the title (as the
  fence-less note backfill already does at `0007-…sh:309`); `review_number`/
  `pr_number` may need a derived/`DIVERGE` value — confirm acceptable defaults.
- `PR #N`/`#N` → `pr:N`: add a token rule to `normalize_bare` (or a sibling) for
  the `PR #<digits>` and `#<digits>` shapes.
- `meta/docs/`: **decide** — likely add to `out_of_scope` (it is freeform docs,
  not a schema artifact) unless a `doc`/`reference` type is intended.

Reserve **D** as a fallback only for shapes that genuinely need human judgment
(none of the five above do, once defaults are agreed). Avoid **C** — the gate is
correctly protecting the interactive phase.

After the fixes, extend `test-migrate-0007.sh` fixtures with: an empty-`type:`
`meta/prs/` description, a note with a `ticket:` JIRA key and no `topic:`, a
`pr-review` with `pr_title:`/`review_pass:`, and a `target: "PR #416"` linkage —
so this corpus shape is exercised in CI.

## Prevention

- **Single-source `infer_type_from_path`** (and ideally the schema-derived
  forbidden/extras logic) between the migration and the validator so the
  transform and the contract cannot drift.
- **Property test the invariant directly**: for a representative corpus, assert
  that `run_backfill + run_rewrite` leaves zero `validate-corpus-frontmatter.sh`
  violations — i.e. the rewrite output is, by construction, validator-clean.
- Add fixtures covering every doc-type the schema declares (PR descriptions were
  declared but never fixture-tested through the migration).

## Recent Changes

`git log` on the affected files was not run as part of this investigation
(the failure is a static coverage gap, not a regression). Worth a quick
`git log -- skills/config/migrate/scripts/0007-frontmatter-rewrite.awk
scripts/templates-schema.tsv` before editing, to confirm the schema's
`pr-description`/`forbidden_own_id_key` columns predate the migration's awk and
were simply never wired in.

## Open Questions

- For `meta/docs/`: is it intended to be a schema artifact (needs a type) or
  freeform docs (add to `out_of_scope`)?
- For `review_number`/`pr_number` on legacy reviews lacking them: is a derived
  value (e.g. from the filename `pr-430-review.md` → `430`) acceptable, or should
  those route to `DIVERGE`/interactive?
- Should the `forbidden_own_id_key` drop preserve the dropped value anywhere
  (e.g. fold `pr_title` into `title` if `title` is absent), or discard silently?
