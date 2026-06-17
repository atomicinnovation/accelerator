---
type: codebase-research
id: "2026-06-17-0114-migration-0007-incomplete-mechanical-normalisation"
title: "Research: Migration 0007 incomplete mechanical normalisation (work item 0114)"
date: "2026-06-17T21:52:32+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0114"
parent: "work-item:0114"
relates_to: ["issue-research:2026-06-17-migration-0007-incomplete-mechanical-normalisation"]
topic: "How migration 0007's mechanical passes diverge from its validator, and exactly where each of the six fixes must land"
tags: [research, codebase, migrate, frontmatter, validator, unified-schema, "0007", awk]
revision: "3ecb810f52d725b09c23747f29784ba78097ecb7"
repository: "accelerator"
last_updated: "2026-06-17T21:52:32+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Migration 0007 incomplete mechanical normalisation (work item 0114)

**Date**: 2026-06-17 21:52 UTC
**Author**: Toby Clemson
**Git Commit**: 3ecb810f52d725b09c23747f29784ba78097ecb7
**Branch**: HEAD (detached)
**Repository**: accelerator

## Research Question

Ground the fix for work item 0114 ("Migration 0007 Incomplete Mechanical
Normalisation") in fresh, line-verified codebase evidence. The work item and its
RCA already prescribe six gaps to close; this research independently re-verifies
every cited line number against the current tree, confirms the prescribed hook
points, and surfaces anything the work item's scope misses or gets wrong.

## Summary

Every claim in the RCA and work item holds up against the live code, with three
additions worth folding into the plan before implementation:

1. **The `extras_for_type` off-by-one is real *and* the function is dead code.**
   `extras_for_type` (`0007-…sh:52`) does `cut -f5`, but `extras` is column **4**
   (`status_vocab` is column 5). It is currently invoked nowhere, so the bug has
   no runtime effect today — but the required-extras backfill (gap 4) is exactly
   what would wire it in, so the fix must switch it to `cut -f4` and treat the
   `-` sentinel as empty *as part of* gap 4, not as a separate cleanup.

2. **The `topic` backfill gap is wider than "fenced notes".** The work item's
   gap 4 frames `topic` backfill as a note concern. In fact the schema requires
   `topic` for **three** types — `note`, `codebase-research`, `issue-research`
   (`templates-schema.tsv:7,8,14`) — and the *fence-less* backfill
   (`backfill_file:306-310`) only seeds `topic` for `note`. So both the
   fenced-file backfill (gap 4) **and** the existing fence-less backfill need to
   cover all three `topic`-bearing types, or a fence-less research file will
   still fail `MISSING-EXTRA`.

3. **`pr-review` has required extras the fix cannot mechanically derive.**
   `pr-review` extras are `reviewer verdict lenses review_number pr_number`;
   `reviewer` is optional (in `FM_OPTIONAL_EXTRAS`) but **`verdict` and `lenses`
   are not** — they are required and have no filename-derivable default. The
   work item only plans to backfill `pr_number`/`review_number`. A legacy
   `pr-review` missing `verdict`/`lenses` would still trip `MISSING-EXTRA` and
   re-abort the migration. This is a genuine scope hole — see Open Questions.

Everything else — the byte-identical `infer_type_from_path` duplication, the
schema column indices, the `FM_TYPED_REF_RE` tolerance of `pr:`, the awk hook
points, the test harness shape — is confirmed exactly as the work item states.

## Detailed Findings

### Area 1 — `infer_type_from_path` triplication (gap 1)

Confirmed **byte-identical** between the migration
(`skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh:82-101`)
and the validator (`scripts/validate-corpus-frontmatter.sh:79-98`). Every `case`
arm matches character-for-character; only the one-line comment header above each
copy differs. Both have a `*/reviews/prs/*` → `pr-review` arm but **no** bare
`*/prs/*` arm and **no** `meta/docs/` handling; an unmatched path returns `""`.

The third encoding is the awk `path_to_typed`
(`0007-frontmatter-rewrite.awk:71-90`): independent regex arms plus id
derivation. Its `else id = base` default (`:87`) means adding
`else if (p ~ /^meta\/prs\//) type = "pr-description"` is sufficient — the
full-stem id derivation applies automatically (so `meta/prs/240-description.md` →
id `240-description`).

A shared-sourcing channel already exists: the migration sources four helpers from
`$PLUGIN_ROOT/scripts/` (`0007-…sh:9-12`) and the validator sources
`frontmatter-emission-rules.sh` via an env-overridable path
(`validate-corpus-frontmatter.sh:24-28`) as its declared single source. So a
`scripts/`-level bash helper holding `infer_type_from_path` can be sourced by
both bash copies; the awk takes the type via the existing `-v type` channel
(computed shell-side). This satisfies gap 1 + the prevention item at once.

### Area 2 — Forbidden own-id keys & legacy keys (gaps 2, 3)

- `own_id_key_for_type` (`0007-…sh:57-63`) knows only `work-item`→`work_item_id`
  and `adr`→`adr_id`; returns `""` for pr types. **Nothing in the migration
  reads the schema's `forbidden_own_id_key` column (TSV field 6).**
- The awk own-id rename arm (`0007-frontmatter-rewrite.awk:247-254`) renames the
  passed `own_id_key` → `id` and sets `emitted_id=1`. The model for an
  **unconditional drop** is the `branch:` rule at `:241` (`if (key=="branch") {
  next }`). The model for a **rename** is the `skill:`→`producer:` rule at `:244`.
- The membership-helper model for a new `is_forbidden(k)` is `in_vocab`
  (`:145-149`): split a pipe-joined `-v` variable, trim, compare. Pass the
  schema's `forbidden_own_id_key` column in as a new `-v forbidden=…`; place the
  drop arm before the catch-all `print $0` at `:303`.
- `pr_title`→`title` **fold**: needs presence-awareness like `emitted_id`. The
  closing-fence block emits `title` only when `!has_title` (`:194`). So the
  key-line handler should, on encountering `pr_title` with no existing `title`,
  emit `title: <value>` and set an `emitted_title` state flag; with an existing
  `title`, just `next` (drop). Mirror the `emitted_id` pattern used at
  `:187/:249/:253`.
- Legacy-key drop (`ticket`/`ticket_id`): the validator forbids these
  cross-cutting on **every** valid type via `OBSOLETE_LEGACY_KEYS=(ticket
  ticket_id)` (`validate-corpus-frontmatter.sh:47`, loop `:330-338`), **not** via
  the per-type forbidden column. So the awk drop must be unconditional (model:
  `:241`), independent of type. Migration 0001
  (`0001-rename-tickets-to-work.sh:54-70`) is confirmed scoped to `meta/tickets/`
  only (line-anchored `sed`, handles the both-keys-present case), so 0007's
  unconditional drop is complementary and won't double-handle the tickets dir's
  rename.

### Area 3 — Required-extras backfill (gap 4)

- **Off-by-one + dead code**: `extras_for_type` (`0007-…sh:52`) cuts `-f5`
  (status_vocab) instead of `-f4` (extras), and is invoked nowhere. The
  required-extras backfill is the call site that fixes it; also treat a `-`
  cell as empty.
- **Backfill site**: the closing-fence emission block
  (`0007-frontmatter-rewrite.awk:185-225`), just before `in_fm = 0` (`:223`). The
  direct template is the work-item `priority: medium` backfill at `:212`
  (`if (type=="X" && !has_KEY) print "KEY: default"`). Each new extra needs a
  matching `-v has_KEY` flag computed caller-side; the awk has no defaulting for
  unset flags (an unset `has_KEY` is falsey, so the backfill would fire — the
  caller must always pass it).
- **Caller does not currently know which extras are optional.** The migration
  does **not** source `frontmatter-emission-rules.sh`; only the validator does.
  To compute "required extras = schema `extras` (col 4) − `FM_OPTIONAL_EXTRAS`"
  the migration must either source that file (preferred — extends the
  single-source posture) or duplicate the optional list (drift risk).
  `FM_OPTIONAL_EXTRAS` = `external_id reviewer pr_url merge_commit
  decision_makers work_item_id` (`frontmatter-emission-rules.sh:74`).
- **Defaults**: `topic` ← title (the fence-less note path already does this at
  `:309`); `pr_number` ← leading filename number; `review_number` ← `1`. These
  are the only three with derivable defaults among the required extras.
- **`topic` is also required for research types** (`templates-schema.tsv:7,8`),
  and the fence-less backfill (`backfill_file:306-310`) only seeds it for `note`
  — so both passes need to cover `note`/`codebase-research`/`issue-research`.

### Area 4 — Non-canonical linkage shapes (gap 5)

- `normalize_paths` (`:95-107`) and `normalize_bare` (`:131-143`) both match
  **double-quoted** tokens only (`/"meta\/[^"]*\.md"/` and `/"[0-9]+"/`). A
  `target: "PR #416"` value arrives quoted, so a sibling
  `normalize_pr_ref` matching `/"PR #[0-9]+"/` and `/"#[0-9]+"/` fits the
  existing pattern (iterate `match`, rewrite each token to `"pr:N"`).
- **Chain site**: `:291`,
  `newval = normalize_bare(normalize_paths(val), type, key)`. Wrap as
  `normalize_pr_ref(normalize_bare(normalize_paths(val), type, key))` so the `#`
  is consumed before the bare-number loop. (Order matters: `normalize_bare` only
  matches fully-bare `"[0-9]+"`, so `"PR #416"` won't be mis-grabbed by it, but
  resolving `pr:` first is cleanest.)
- **Linkage values bypass `fm_refuses`** (the `is_linkage_key` arm at `:288-298`
  never routes through the refusal check), so reshaping a `#`-bearing value here
  is safe and won't trip a REFUSE.
- **Output validates**: `FM_TYPED_REF_RE`
  (`frontmatter-emission-rules.sh:88`, via `FM_SOURCE_TYPE_RE` at `:41`) includes
  `pr` as a tolerated prefix, and `pr:*` refs are explicitly skipped from the
  `DANGLING-REF` index lookup (`validate-corpus-frontmatter.sh:389-390`). So
  `pr:416` passes both shape and referential checks. The id part
  `[A-Za-z0-9.-]+` forbids `/` and `#`.

### Area 5 — `meta/docs/` out-of-scope (gap 6)

`out_of_scope` excludes only `*/specs/*`, `*/talks/*`, `*/global/*` in **both**
the migration (`0007-…sh:103-108`) and the validator
(`validate-corpus-frontmatter.sh:101-106`). Adding `*/docs/*` to both makes the
migration skip those files (so `rewrite_file` never touches them) and the
validator never judge them — closing the largest single violation bucket
(76 `INVALID-TYPE`, mostly `meta/prs/` + `meta/docs/`).

### Area 6 — Schema TSV columns (precondition for gaps 2 & 4)

`scripts/templates-schema.tsv` column order confirmed (1-based): `1 template`,
`2 type`, `3 code_state_anchored`, `4 extras`, `5 status_vocab`,
`6 forbidden_own_id_key`, `7 typed_linkage_keys`. There is **no** positive
`own_id_key` column. The `-` sentinel = empty cell. Relevant rows:

| type | extras (f4) | forbidden_own_id_key (f6) | anchored |
|------|-------------|---------------------------|----------|
| `pr-description` | `pr_url pr_number merge_commit` | `pr_title` | yes |
| `pr-review` | `reviewer verdict lenses review_number pr_number` | `pr_title review_pass` | no |
| `note` | `topic` | `-` | yes |
| `codebase-research` | `topic` | `-` | yes |
| `issue-research` | `topic` | `-` | yes |

Required (= extras − `FM_OPTIONAL_EXTRAS`): pr-description → `pr_number` only
(`pr_url`/`merge_commit` optional); pr-review → `verdict lenses review_number
pr_number` (`reviewer` optional). `pr-description` is anchored, so the existing
anchored backfill already supplies `revision`/`repository`.

### Area 7 — Test harness shape (acceptance-criteria fixtures)

Test file: `skills/config/migrate/scripts/test-migrate-0007.sh` (under
`scripts/`, not a `tests/` dir). It builds throwaway repos under one `mktemp -d`
root, seeds `meta/**/*.md` via heredocs, `git_init`s (`:27-33`), runs the
migration through the real driver via `run_0007` (`:36-41`), then asserts. There
is **no `assert_validates` helper** — validator-clean is an inline idiom
(`vrc=0; "$VALIDATOR" "$REPO/meta" … || vrc=$?; if [ "$vrc" -eq 0 ]…`, e.g.
`:142-152`). The negative path asserts the migration *refusing* (`:178-184`),
not specific violation strings. The **path-shape block** (`:230-310`) is the
direct model for new fixtures: seed files at varied `meta/` paths, run, assert
on individual `fm_line` outputs, then gate on a validator-clean run over
`$REPO/meta`. Existing coverage is organised as `=== … ===` labelled blocks
(happy path, validator-clean, idempotency, REFUSE, frag parity, protocol
hygiene, path-shape linkage, interactive linkage) — no per-case functions.

## Code References

- `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh:82-101` — `infer_type_from_path` (no `*/prs/*`, no `meta/docs/`)
- `…/0007-…sh:57-63` — `own_id_key_for_type` (work-item/adr only)
- `…/0007-…sh:103-108` — `out_of_scope` (add `*/docs/*` here)
- `…/0007-…sh:48-55` — `schema_row`/`extras_for_type` (the `cut -f5`→`-f4` off-by-one, dead code)
- `…/0007-…sh:9-12, 26-29` — shared-sourcing channel + schema/awk path vars
- `…/0007-…sh:306-310` — fence-less `topic` backfill (note-only; widen to research types)
- `…/0007-…sh:348-352` — `rewrite_file` empty-type early return
- `…/0007-…sh:415-427` — awk `-v` argument list (where new `-v forbidden=`/`-v has_KEY` go)
- `…/0007-…sh:629-651` — orchestration; `set -e` abort at `self_validate_structural` (`:640`)
- `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk:71-90` — `path_to_typed` (add `meta/prs/` arm)
- `…/0007-frontmatter-rewrite.awk:95-143` — `normalize_paths`/`normalize_bare` (sibling for `PR #N`)
- `…/0007-frontmatter-rewrite.awk:145-149` — `in_vocab` (model for `is_forbidden`)
- `…/0007-frontmatter-rewrite.awk:185-225` — closing-fence emission (required-extras backfill site; `priority` template at `:212`)
- `…/0007-frontmatter-rewrite.awk:228-304` — key-rewrite rules (`branch` drop `:241`, `skill`→`producer` `:244`, own-id rename `:247-254`, catch-all `:303`)
- `…/0007-frontmatter-rewrite.awk:288-298` — linkage-key arm; chain site `:291`
- `scripts/validate-corpus-frontmatter.sh:79-98` — duplicate `infer_type_from_path` (byte-identical)
- `scripts/validate-corpus-frontmatter.sh:47, 330-338` — `OBSOLETE_LEGACY_KEYS` cross-cutting drop
- `scripts/validate-corpus-frontmatter.sh:245-250, 322-344, 384-400` — `INVALID-TYPE`/`FORBIDDEN-OWN-ID`/`MISSING-EXTRA`/`BAD-LINKAGE-SHAPE` checks
- `scripts/frontmatter-emission-rules.sh:41, 74, 88` — `FM_SOURCE_TYPE_RE`, `FM_OPTIONAL_EXTRAS`, `FM_TYPED_REF_RE`
- `scripts/templates-schema.tsv:1, 5, 7, 8, 13, 14` — header + pr-description/research/note/pr-review rows
- `skills/config/migrate/scripts/test-migrate-0007.sh:27-41, 142-152, 230-310` — harness helpers, validator-clean idiom, path-shape model
- `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh:54-70` — scoped `ticket_id`→`work_item_id` rename

## Architecture Insights

- **Three independent encodings of one fact.** Type-by-path lives in two
  byte-identical bash copies plus a third awk regex encoding. The fix touches all
  three; single-sourcing the two bash copies (and feeding the awk via `-v type`,
  which already happens) is the cleanest way to satisfy gap 1 and prevent
  recurrence in one move.
- **Schema-driven vs hard-coded divergence.** The schema declares
  `forbidden_own_id_key` and `extras`, but the migration consumes neither for the
  failing cases — it hard-codes own-id (`own_id_key_for_type`) and has no extras
  path at all (`extras_for_type` is dead). Driving gaps 2 & 4 off the TSV columns
  (passed into the awk as `-v`) keeps the transform aligned with the contract.
- **The validator is the single source for "what's optional".** Because only the
  validator sources `frontmatter-emission-rules.sh`, the migration is blind to
  `FM_OPTIONAL_EXTRAS`. Gap 4 should bring the migration under the same source
  rather than re-listing the optional extras.
- **The `set -euo pipefail` abort is correct** and out of scope (RCA Hypothesis
  3). The fix belongs entirely in the mechanical passes so the corpus is
  validator-clean by the time `self_validate_structural` runs.

## Historical Context

- `meta/research/issues/2026-06-17-migration-0007-incomplete-mechanical-normalisation.md` — the RCA this work item is based on; every line reference re-verified here.
- `meta/work/0070-ship-meta-corpus-unified-schema-migration.md` + `meta/plans/2026-06-07-0070-…md` + `meta/research/codebase/2026-06-07-0070-…md` — built migration 0007.
- `meta/validations/2026-06-09-0070-meta-corpus-migration-dogfood.md` — the dogfood validation run; the corpus it exercised lacked `meta/prs/`, `ticket:` notes, and `PR #N` linkage, which is why these shapes were never caught (RCA Contributing Factors).
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`, `ADR-0034-typed-linkage-vocabulary.md`, `ADR-0040-omit-when-empty-frontmatter-emission-supplement-to-adr-0033.md`, `ADR-0042-reconciling-pre-schema-status-values.md` — the schema/linkage/emission contracts the validator enforces.
- `meta/work/0103-…md` (producer-side emission) and `meta/work/0105-…md` (validator blind-spots) — sibling, already-`done` efforts; deliberately out of scope here.
- `meta/reviews/work/0114-…-review-1.md` — existing review of the work item (verdict COMMENT).

## Related Research

- `meta/research/codebase/2026-06-15-0105-corpus-validator-provenance-linkage-blind-spots.md`
- `meta/research/codebase/2026-06-09-0103-skill-frontmatter-emission-audit.md`
- `meta/research/codebase/2026-06-07-0070-meta-corpus-unified-schema-migration.md`

## Open Questions

1. **`pr-review` `verdict`/`lenses` required-extras hole.** These are required
   (not in `FM_OPTIONAL_EXTRAS`) but have no filename-derivable default, and the
   work item only plans to backfill `pr_number`/`review_number`. A legacy
   `pr-review` lacking `verdict`/`lenses` would still fail `MISSING-EXTRA` and
   re-abort. Options: (a) add them to `FM_OPTIONAL_EXTRAS` if they're legitimately
   absent on legacy reviews; (b) backfill a sentinel/`DIVERGE` value; (c) accept
   that affected corpora need manual cleanup. Needs a decision before the
   "validator-clean by construction" guard can hold on real pr-review corpora.
2. **`topic` backfill scope.** Confirm the fix widens `topic` backfill to
   `codebase-research`/`issue-research` (both fence-less and fenced), not just
   `note` — otherwise a fence-less research file still fails `MISSING-EXTRA`.
3. **`pr-description` id derivation.** With the new `meta/prs/` arm, id defaults
   to the full filename stem (`240-description`), while `pr_number` is the parsed
   leading number (`240`). Is the full-stem id acceptable, or should
   pr-description id be the bare number? (Not required by any AC; flag only.)
