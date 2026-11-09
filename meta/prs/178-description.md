---
type: "pr-description"
id: "178"
title: "[0319] Let work create take an explicit work item ID"
date: "2026-11-02T18:35:08+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0319"
parent: "work-item:0319"
relates_to: ["work-item:0146", "work-item:0371", "work-item:0302", "work-item:0343", "work-item:0228", "work-item:0230"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/178"
pr_number: 178
tags: ["work", "cli", "allocation", "create-work-item", "corpus", "visualiser"]
revision: "a2ab614f809cf07657f421feff3d490929f6aae1"
repository: "accelerator"
last_updated: "2026-11-09T21:35:56+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0319] Let work create take an explicit work item ID

## Summary

`accelerator work create --id <ID>` writes a new work item under exactly the
ID given, through the same atomic write, `NNNN` substitution and `--push` as
an allocated create. Allocation only sees the local work directory, so
parallel branches clashed at merge, and the workaround of writing the file by
hand skipped validation and left the item unsynced. `/create-work-item` takes
the same ID through a new `--id` argument, settled by a new `work id check`.

To give one place to check an ID against, work-item identity moves out of
the general-purpose `corpus` crates into `work`, onto a single ID-pattern
grammar. That move is most of the diff and carries several behaviour changes
of its own, listed under "Behaviour changes".

## Changes

### `work create --id`

- **Shape**: the ID must be exactly what `work.id_pattern` renders: the
  `{number}` padding exactly, a key wherever the pattern has `{key}`, and a
  number from 1 to the pattern's cap. A short or unpadded number is refused,
  never normalised. The key may be any `[A-Za-z][A-Za-z0-9]*`, not only
  `work.key`, and is written in the case given.
- **Refusals**, in this order, each exit 1 with its own code and write and
  send nothing:

  | Code | When | Message names |
  | --- | --- | --- |
  | `E_ID_WITH_PROJECT` | `--id` beside `--project` | both flags |
  | `E_ID_TRACKER_OWNED` | `{tracker}` pattern | the pattern |
  | `E_ID_DRAFT_RESERVED` | `draft-` + 6 Crockford chars, ≥1 letter | `draft-` |
  | `E_ID_PATTERN_MISMATCH` | not the pattern's exact shape | the pattern |
  | `E_ID_TAKEN` | held by any item | the holder's path |

- **Taken rule**: an ID is held by any item's `id`, `aliases` entry or
  `external_id`, and by any filename's ID prefix at any width. Parseable IDs
  compare by key (ignoring case) and number, so `400-x.md` and `id: "400"`
  both take `0400`. Anything else compares ignoring case.
- **Ordering**: the re-key guard, then checks 1–4, then the collision check
  under the create lock, so a racing create of the same ID is caught. Under
  `--push`, every check runs before any tracker request; a push that fails
  afterwards behaves as it does without `--id`.
- **`--dry-run --id`** runs the same checks in the same order, without the
  lock, before the tracker preview. A pass leaves the dry-run output
  unchanged.
- **Allocation** stays max+1 per key and ignores gaps an `--id` leaves.

### The `work id` verb group

- **`work id canonicalise`** and **`work id next`** replace
  `work canonicalise-id` and `work next-number`, with unchanged behaviour and
  no alias. The old names run nothing, name their replacement and exit 2.
- **`work id check <ID>`** prints `exact`, `other-key` (naming `work.key`) or
  `corrected` (left-padded, or `work.key` added under a `{key}` pattern) with
  the ID to pass to `--id`, or refuses with the `E_ID_*` code
  `work create --id` would give. It writes and reserves nothing, and joins
  the identity run lock beside `work id next` and `work update --push`.

### `/create-work-item --id`

- **Settling**: `--id` is parsed before `work resolve`, so `0400` is never
  read as a reference to item 0400. The skill settles it with `work id check`:
  an exact ID is used as given, a correction is proposed and used only on
  confirmation, and a refusal is relayed before any drafting.
- **Creating**: the settled ID is shown as the projected ID and passed via
  `--id` on both the push-gate dry-run and the real create, with no
  `work id next` call. A refusal after drafting resumes with the approved
  draft unchanged.
- **Recovery**: after a declined proposal, a refusal or an ID with no exact
  form, the skill waits for the user to name another ID, ask for automatic
  allocation or cancel. Under `{tracker}` only automatic allocation remains.
- **Evals**: 34 new cases (39–72) with a fresh benchmark, plus re-run
  benchmarks for `extract-work-items` and `refine-work-item` after their
  move onto the `work id` verbs.

### Work-item identity moves into `work`

- **`tracker`** owns the tracker-key grammar.
- **`corpus`** keeps two general ports, `IdConvention` and `LinkageIds`,
  for slugs, clusters and linkage. `corpus-adapters` loses its scanner,
  pattern compiler, canonicaliser and its `regex` dependency.
- **`work::id_pattern`** is the one ID-pattern grammar, replacing the
  regex-built scanners: a 255-byte bound, a width cap of 19 and a restricted
  literal alphabet.
- **`work::id_scheme`** (`IdScheme`) implements `IdConvention`;
  `legacy_ids` (`LegacyNumericIds`) implements `LinkageIds`;
  `requested_id` models an `--id` and its settlement.
- **`work_adapters::config`** reads the scheme once for every command
  (`read_scheme`, `require_key`, `resolve_scheme`), reporting through
  `SchemeError`.
- **Consumers**: `migrate` gains a `WorkItemIdentity` port, wired in
  `migrate-cli`; the visualiser, re-key, retirement and sync all resolve IDs
  through the scheme.
- **Sync code from PR #169** moves onto the scheme with the rest:
  `draft_id::is_token_safe`, kept edits and recovery labels take
  `work::id_scheme::IdScheme`, and `ConfiguredNames` names work-item
  files through `id_in_filename` instead of compiling a scan regex, so it can
  no longer fail.
- **Guard**: `lint:corpus-identity:check` fails if a `corpus` source names a
  work-item identity item again.

### Housekeeping

- **Work items**: 0146 gains a work-item ID model section; 0371
  (`--dry-run` invocation validation) is added, blocked by this. It was filed
  as 0351, but PR #169 filed a different 0351 and 0355 is taken on another
  branch, so it was renumbered to 0371.
- **Docs**: `docs-site` `work-items.md`, `tasks/README.md` and the skills
  name the `work id` verbs; `test_retired_id_verbs.py` keeps old names out.

## Behaviour changes

The full list is in `CHANGELOG.md`. The ones a script or repository may
notice:

- ⚠️ **Retired verbs**: `work canonicalise-id` and `work next-number` exit 2.
- ⚠️ **Newly refused patterns**: literal text outside ASCII letters, digits,
  `.`, `_` and `-` (including `{{`/`}}`), a digit beside a token, a width
  over 19 or a repeated token gives `E_WORK_ID_PATTERN_INVALID` from every
  `work` command and from the visualiser at boot.
- **Scheme validation**: every `work` command that reads the scheme refuses a
  malformed `work.id_pattern` or `work.key` at start; the new
  `E_WORK_KEY_INVALID` also covers `--project ""`, which gave
  `E_PATTERN_MISSING_KEY`.
- **Allocation** counts a key's sequence ignoring case, and under a `{key}`
  pattern counts a legacy bare-number item in `work.key`'s sequence, so the
  next number may jump.
- **`work resolve`** reads a missing work directory as empty rather than
  exiting 1 for every input.
- **Literal pattern text** is kept: `v{number:03d}` allocates `v001` and
  `work rekey` re-keys it.
- **Short full IDs** canonicalise through the pattern: `PROJ-42` reads as
  `PROJ-0042` in `list`, `resolve`, `sync` and the visualiser's clusters.
- **`.MD` items** are read by every `work` command and the visualiser.
- **Visualiser**: keys match ignoring case, a cluster's `clusterKey` is the
  canonical form, and a `work-item:` reference in `parent:` or `related:`
  counts as an inbound link.
- **`E_PATTERN_OVERFLOW`** names the item holding the cap, when one does.

## Context

- Work item: `meta/work/0319-explicit-work-item-id-on-accelerator-work-create.md`
  (Linear PP-905), a child of 0146.
- Research: `meta/research/codebase/2026-10-29-0319-explicit-work-item-id-on-accelerator-work-create.md`
- Plan: `meta/plans/2026-10-29-0319-explicit-work-item-id-on-accelerator-work-create.md`
  (11 phases), with its review in `meta/reviews/plans/` and the work item's
  review in `meta/reviews/work/`.
- Validation: `meta/validations/2026-10-29-0319-explicit-work-item-id-on-accelerator-work-create-validation.md`
  (result `pass`).
- Follow-up: 0371 makes `--dry-run` run every invocation check the real
  create runs; it keeps this PR's five `--id` checks.

## Testing

- [x] `mise run` (the full default task, `docs:check` included) exits 0 end
      to end at the rebased tip.
- [x] `cargo nextest run --workspace --no-fail-fast` passes 7492 of 7492 at
      the rebased tip.
- [x] `cargo clippy --workspace --tests` is clean at every commit that
      touches `cli/`, checked after the rebase onto PR #169.
- [x] Every test the plan and its criteria traceability table name exists
      (88 of 88, per the validation report), across `cli_create.rs`,
      `cli_id_check.rs`, `cli_id_next.rs`, `cli_id_canonicalise.rs`,
      `requested_id.rs`, `id_pattern.rs` and the visualiser's tests.
- [x] `/create-work-item` evals 39–72 pass with the skill (with-skill mean
      0.9956); `test_create_work_item_id_evals.py` fails if any of them stops
      passing fully.
- [x] Manual checks: `/create-work-item --id 400 <topic>` in a scratch
      repository proposes `0400` and writes it after confirmation; with an
      integration configured and `0400` taken, the push gate relays
      `E_ID_TAKEN` and offers another ID or automatic allocation; with
      `9997-x.md` held, `work id next --count 3` prints `9998`, `9999`, then
      the neutral `E_PATTERN_OVERFLOW`. Run during implementation, per the
      plan's ticked manual checks; not re-run since.

## Notes for Reviewers

- **First review**: `meta/reviews/prs/178-review-1.md` records verdict
  `COMMENT`. None of its findings are addressed on this branch yet.
- **Review order**: the diff is 229 files, over a third of it the plan,
  its reviews and eval benchmarks. Reading `cli/work/src/id_pattern.rs`,
  `id_scheme.rs` and `requested_id.rs`, then `cli/work-cli/src/create.rs`
  and `id/check.rs`, gives the core before the consumer moves.
- **Rebased onto `main`** at PR #177. The rebase onto PR #169's sync work
  conflicted in 14 places; the resolutions follow `main`'s structure and carry
  this branch's changes into it.
- **`identity_key` separator**: its `keyed_digits` fallback always joins with
  `-` (`cli/work/src/id_scheme.rs:292-294`), so under `{key}_{number:04d}` a
  reference written `PP-42` joins as `PP-0042`. Outside the work item's
  criteria; ❓ worth deciding whether it should honour the pattern's
  separator.
- **`--dry-run --id` without `--push`** fails when `paths.integrations`
  cannot be read, where a plain dry-run succeeds. This is intended: the
  pending-push state feeds the collision check.
- **Unchanged ordering**: under `{tracker}` without `--id`,
  `E_PATTERN_KEY_UNUSED` still follows `create_dir_all`, as before.
- **`last-sync.json`** changes only because 0319 and 0371 were synced to
  Linear. Main's 0351 (PP-926) keeps its own entry.
