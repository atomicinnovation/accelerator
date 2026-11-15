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
tags: ["work", "cli", "allocation", "create-work-item", "corpus", "visualiser", "rekey", "migrate"]
revision: "404cca765b4e12dbaf87ebb5b5520be5bfa1c4c7"
repository: "accelerator"
last_updated: "2026-11-15T12:16:33+00:00"
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
grammar and one ID scheme. That move is most of the diff. The response to
the first review then made the scheme coherent end to end: a corpus off the
configured pattern is now refused until re-keyed, live linkage reads through
the scheme, each tracker owns its key shape, and the visualiser keeps
serving when IDs cannot be trusted. Every behaviour change is listed under
"Behaviour changes".

## Changes

### `work create --id`

- **Shape**: the ID must be exactly what `work.id_pattern` renders: the
  `{number}` padding exactly, a key wherever the pattern has `{key}`, and a
  number from 1 to the pattern's cap. A short or unpadded number is refused,
  never normalised. The key may be any `[A-Za-z][A-Za-z0-9]*`, not only
  `work.key`, and is written in the case given.
- **Refusals**, in this order, writing and sending nothing:

  | Code | Exit | When | Message names |
  | --- | --- | --- | --- |
  | `E_ID_WITH_PROJECT` | 1 | `--id` beside `--project` | both flags |
  | `E_ID_TRACKER_OWNED` | 1 | `{tracker}` pattern | the pattern |
  | `E_ID_DRAFT_RESERVED` | 1 | `draft-` + 6 Crockford chars, ≥1 letter | `draft-` |
  | `E_ID_PATTERN_MISMATCH` | 1 | not the pattern's exact shape | the pattern |
  | `E_ID_TAKEN` | 4 | held by any item | the holder's path |

  `E_ID_TAKEN` exits 4 like every other collision, so a script can tell a
  bad ID from a taken one by its exit code.
- **Taken rule**: an ID is held by any item's `id`, `aliases` entry or
  `external_id`, and by any filename's ID prefix at any width. Parseable IDs
  compare by key (ignoring case) and number, so `400-x.md` and `id: "400"`
  both take `0400`, under every pattern including keyless literal ones. One
  `ItemIdentity::fields_holding` reading serves the taken rule and the
  holder and linker lookups.
- **Ordering**: the identity-run guard, the corpus checks (`E_REKEY_REQUIRED`,
  `E_WORK_ITEM_MALFORMED`), checks 1–4, then under `--push` the pending-push
  refusals, then the taken check under the create lock, so a racing create
  of the same ID is caught. One `work_adapters::creation::refuse_requested_id`
  owns the last three steps.
- **`--dry-run --id`** goes through the same `refuse_requested_id`, without
  the lock, before the tracker preview, so it now lists the corpus first as
  the create does. A pass leaves the dry-run output unchanged.
- **Allocation** stays max+1 per key, ignoring gaps an `--id` leaves, and now
  counts a number held in any item's `id`, aliases or `external_id`, drafts
  included, so it never hands out an ID `--id` would refuse.

### The `work id` verb group

- **`work id canonicalise`** and **`work id next`** replace
  `work canonicalise-id` and `work next-number`, with no alias. The old
  names run nothing, print `Error: E_VERB_RETIRED:` naming their
  replacement, and exit 2.
- **`work id check <ID>`** prints one tab-separated verdict, documented as an
  output contract in its help: `exact`, `other-key` (naming `work.key`) or
  `corrected` (padded, `work.key` added, or the configured key's spelling
  restored, so `pp-0400` becomes `PP-0400`), or refuses with the `E_ID_*`
  code `work create --id` would give, at the same exit code. It writes and
  reserves nothing.
- **One vocabulary**: every refusal of the three verbs starts `Error:`;
  `id canonicalise` reports a bad `work.key` as `E_WORK_KEY_INVALID`, as its
  siblings do, and keeps its own `E_PATTERN*` codes for pattern errors.
  `id check` and `id next` take the identity-run lock before reading the
  scheme, as `create` does.

### `/create-work-item --id`

- **Settling**: `--id` is parsed before `work resolve`, so `0400` is never
  read as a reference to item 0400. The skill settles it with `work id check`:
  an exact ID is used as given; a correction or an `other-key` ID is proposed
  in full and used only on confirmation, since a key typo would start a new
  sequence; a refusal is relayed before any drafting. An `--id` in the
  middle of the argument is asked about rather than read as topic text.
- **Creating**: the settled ID is shown as the projected ID and passed via
  `--id` on both the push-gate dry-run and the real create, with no
  `work id next` call. A refusal after drafting resumes with the approved
  draft unchanged.
- **Recovery**: after a declined proposal, a refusal or an ID with no exact
  form, the skill waits for the user to name another ID, ask for automatic
  allocation or cancel. Under `{tracker}` only automatic allocation remains.
  `E_REKEY_REQUIRED` and `E_WORK_ITEM_MALFORMED` stop the skill with their
  remedy.
- **Evals**: 34 new cases (39–72) with a fresh benchmark, plus re-run
  benchmarks for `extract-work-items` and `refine-work-item` after their
  move onto the `work id` verbs. The 13 evals the review response touched
  were re-run (see Testing).
- **Eval guards**: `test_create_work_item_id_evals.py` now covers every eval
  in `evals.json` and fails a run graded against expectations the eval no
  longer holds; `eval_contract.rs` holds the refusals and verdicts the
  prompts quote to the exit codes and fields the CLI prints.

### Convergence: a corpus off the pattern is refused

- **`work::rekey::convergence`**: an item is converged when its `id` is in
  the pattern's exact shape under any key or, under `{tracker}`, is a draft
  or its issue's key. `create`, `create-batch`, `id next`, `id check`,
  `list`, `resolve` and `promote` refuse a corpus holding any other item with
  `E_REKEY_REQUIRED` (exit 4), naming one and pointing to
  `/rekey-work-items`. `sync`, `rekey` and `id canonicalise` still run.
- **`work::well_formed`**: the same commands refuse a work item whose
  filename does not start with its `id` and a slug, or that names an ID
  with no `id`, as `E_WORK_ITEM_MALFORMED` (exit 4). A file with no
  frontmatter is not a work item and is left alone.
- **`identity_key`** loses its `-`-joining fallback, so a reference an
  earlier pattern wrote joins nothing until re-keyed.

### Work-item identity moves into `work`

- **`tracker`** owns the tracker-key shapes: `TrackerKeys::{Jira, Linear,
  AnySupported}` replaces the generic `is_tracker_key`, so each tracker
  decides its own key shape and a Linear team key may not hold `_`.
- **`corpus`** keeps two general ports, `IdConvention` and `LinkageIds`,
  for slugs, clusters and linkage; finding bare IDs in a line is now part of
  `LinkageIds`. `corpus-adapters` loses its scanner, pattern compiler,
  canonicaliser and its `regex` dependency.
- **`work::id_pattern`** is the one ID-pattern grammar, replacing the
  regex-built scanners: a 255-byte bound, a width cap of 19, a restricted
  literal alphabet and one tokeniser (`{tracker}` is a segment). An
  exhaustive test pins that every accepted pattern splits a value at most
  one way.
- **`work::id_scheme`** (`IdScheme`) dispatches every reading to
  `TrackerIds` or `PatternIds` through one `IdReadings` trait grouped as
  display, join, occupancy and rendering. It implements both corpus ports,
  and carries the tracker keys `work.integration` names. `displayed_id`
  replaces `normalise_id` and reads through the pattern; `HeldPrefix` says
  where a filename's ID ends. `requested_id` models an `--id` and its
  settlement.
- **`work_adapters::config`** reads the scheme once for every command
  (`read_scheme`, `require_key`, `resolve_scheme`) through one
  `scheme_from_settings`, reporting through `SchemeError` with a `KeySource`
  naming the setting a bad key came from, and warns once about a malformed
  `work.key` the pattern does not use.
- **`migrate`** owns the frozen legacy readings (`LegacyNumericIds`, the
  lenient `references_key`) that m0007 and m0009 read directly; its
  `WorkItemIdentity` port, wired in `migrate-cli`, now only canonicalises,
  through `scheme_from_settings` in both the shipped adapter and the test
  composition.
- **Consumers**: `corpus linkage extract` reads through the configured
  scheme; the visualiser, re-key, retirement and sync resolve IDs through
  it; `draft_id::is_token_safe`, kept edits and recovery labels take
  `IdScheme`, and `ConfiguredNames` names work-item files through
  `id_in_filename` instead of a scan regex.
- **Guard**: `lint:corpus-identity:check` fails if a `corpus` source names a
  work-item identity item again.

### Visualiser

- **Degrades instead of refusing to boot**: under a scheme it cannot read it
  serves every other document, reads no work-item IDs, and reports the
  refusal in its log and as `problem` from `/api/work-item/config`, as it
  does a corpus needing a re-key or holding a malformed item (read at boot).
- **Joins**: the identity-key index is named after what it holds; two items
  sharing an identity key resolve to the lower path whatever the read order,
  and removing it hands the key on; `.MD` work items update live.
- **Shown IDs** are read through the pattern (`PP_0042`, `v001` now show).

### Hardening

- **Retirement records** refuse an `old`, `new` or `new_external_id`
  outside the ID alphabet, so a quote or newline cannot reach frontmatter;
  a record's or pending-push marker's `recovery_dir` must be one directory
  under `retirement-recovery/`.
- **Re-key** refuses an `id` outside the ID alphabet and escapes the item
  path in that refusal.

### Housekeeping

- **Work items**: 0146 gains a work-item ID model section; 0371
  (`--dry-run` invocation validation) is added, blocked by this. It was filed
  as 0351, but PR #169 filed a different 0351 and 0355 is taken on another
  branch, so it was renumbered to 0371.
- **Docs**: the configure skill, `docs-site` (`work-items.md`, `corpus.md`
  and the guides), `tasks/README.md` and the skills follow every change;
  `test_retired_id_verbs.py` keeps the old verb names out of the skills,
  the user docs and the task README.

## Behaviour changes

The full list is in `CHANGELOG.md`. The ones a script or repository may
notice:

- ⚠️ **Off-pattern corpus refused**: `create`, `create-batch`, `id next`,
  `id check`, `list`, `resolve` and `promote` exit 4 with `E_REKEY_REQUIRED`
  until `/rekey-work-items` converges the corpus, including after a
  width-only change (`{number:04d}` → `{number:05d}`) or a switch to
  `{tracker}`, which were tolerated before.
- ⚠️ **Malformed items refused**: the same commands exit 4 with
  `E_WORK_ITEM_MALFORMED` for a slugless filename such as `0400.md` or an
  item naming an ID without an `id`.
- ⚠️ **Newly refused patterns**: literal text outside ASCII letters, digits,
  `.`, `_` and `-` (including `{{`/`}}`), a digit beside a token, a width
  over 19 or a repeated token gives `E_WORK_ID_PATTERN_INVALID` from every
  `work` command, with no backwards compatibility; the visualiser reports it
  rather than refusing to boot.
- ⚠️ **Retired verbs**: `work canonicalise-id` and `work next-number` exit 2
  with `Error: E_VERB_RETIRED:`.
- ⚠️ **Exit codes and prefixes**: `E_ID_TAKEN` exits 4; every `work id`
  refusal starts `Error:`.
- ⚠️ **Visualiser `workItemId`** is read through the pattern: a bare `42`
  under `{key}` shows as `ENG-0042` where it showed `ENG-42`, and an `id` the
  pattern does not read shows nothing.
- **References from an earlier pattern** (`PP-42` under `{key}_{number:04d}`,
  `ENG-42` under `{number:04d}`) no longer join their item until re-keyed.
- **`corpus linkage extract`** reads work-item IDs through the configured
  pattern where it read only four digits; migration 0007 keeps the
  four-digit rules.
- **Linear identifiers** may not hold `_` in their team key.
- **Re-key** refuses an `id` holding a space or other character outside the
  ID alphabet.
- **Scheme validation**: every `work` command that reads the scheme refuses a
  malformed `work.id_pattern` or `work.key` at start; the new
  `E_WORK_KEY_INVALID` also covers `--project ""`, which gave
  `E_PATTERN_MISSING_KEY`. A malformed key the pattern does not use is warned
  about.
- **Allocation** counts a key's sequence ignoring case, counts every held
  value, and under a `{key}` pattern counts a legacy bare-number item in
  `work.key`'s sequence, so the next number may jump.
- **`work resolve`** reads a missing work directory as empty rather than
  exiting 1 for every input.
- **Literal pattern text** is kept: `v{number:03d}` allocates `v001` and
  `work rekey` re-keys it.
- **Short full IDs** canonicalise through the pattern: `PROJ-42` reads as
  `PROJ-0042` in `list`, `resolve`, `sync` and the visualiser's clusters.
- **`.MD` items** are read by every `work` command and the visualiser, and
  update the visualiser live.
- **`E_PATTERN_OVERFLOW`** names the first item holding the cap, when one
  does.

## Context

- Work item: `meta/work/0319-explicit-work-item-id-on-accelerator-work-create.md`
  (Linear PP-905), a child of 0146.
- Research: `meta/research/codebase/2026-10-29-0319-explicit-work-item-id-on-accelerator-work-create.md`
- Plan: `meta/plans/2026-10-29-0319-explicit-work-item-id-on-accelerator-work-create.md`
  (11 phases), with its review in `meta/reviews/plans/` and the work item's
  review in `meta/reviews/work/`.
- Validation: `meta/validations/2026-10-29-0319-explicit-work-item-id-on-accelerator-work-create-validation.md`
  (result `pass`, against the plan before the review response).
- First review: `meta/reviews/prs/178-review-1.md`; its inline threads are
  answered and resolved, and the rest is answered in the PR's summary
  comment.
- Follow-up: 0371 makes `--dry-run` run every invocation check the real
  create runs; it keeps this PR's `--id` checks.

## Testing

- [x] `ACCELERATOR_COVERAGE=off mise run test:unit:cli` passes 7627 of 7627
      at the tip.
- [ ] `mise run` (the full default task) at the tip: every stage passed
      except `test:unit:cli`, which failed only on flakes in crates this PR
      does not touch: `launcher` `version_reports_build_baked_metadata` (a
      stale build timestamp) and `github::octocrab_client` timeouts under
      parallel load. Both pass on a rerun; a clean end-to-end run is still
      to be recorded.
- [x] `mise run check`, `pup:check` and `public-api:check` are clean at the
      tip.
- [x] `/create-work-item` evals pass in full, all 75 (with-skill mean 1.0).
      The 13 the review response touched (26, 39, 42, 45, 48, 49, 51, 52,
      59–62, 66) were re-run on 2026-11-15 by `claude-opus-5-5`, by
      executor agents barred from the evals and benchmark files and graded
      by separate agents from the outputs alone; eval 26, which had failed,
      now passes.
- [x] Every test the plan and its criteria traceability table name exists
      (88 of 88, per the validation report).
- [ ] Manual checks: `/create-work-item --id 400 <topic>` in a scratch
      repository proposes `0400` and writes it after confirmation; with an
      integration configured and `0400` taken, the push gate relays
      `E_ID_TAKEN` and offers another ID or automatic allocation; with
      `9997-x.md` held, `work id next --count 3` prints `9998`, `9999`, then
      the neutral `E_PATTERN_OVERFLOW`. Run during implementation; not
      re-run since the review response.

## Notes for Reviewers

- **Review order**: the diff is about 210 files, a third of it the plan, its
  reviews and eval benchmarks. Reading `cli/work/src/id_pattern.rs`,
  `id_scheme.rs` and `id_scheme/`, `rekey/convergence.rs`, `well_formed.rs`
  and `requested_id.rs`, then `cli/work-cli/src/create.rs`, `convergence.rs`
  and `id/check.rs`, gives the core before the consumer moves.
- **Convergence is any-key**: re-key folds every key onto `work.key`, but a
  corpus counts as converged with items under other keys, or every
  `--project` repository would refuse after its first other-key item.
- **Exempt from `E_REKEY_REQUIRED`**: `sync` and `rekey`, because a re-key
  refuses while a sync or `--push` create is unfinished and only sync
  clears that, and `id canonicalise`, which reads no corpus.
- **Visualiser `problem`** is reported in the log and the API only; the
  frontend does not show it yet, and it is read once at boot.
- **Eval re-runs** used this session's own executor and grader agents rather
  than skill-creator, and graded changes made in the same session; the
  benchmark notes record how.
- **`E_ID_TAKEN` exit code** is classified from the frozen code at the start
  of the refusal message (`exit_codes::of_failure`), since create's
  refusals travel as strings.
- **`--dry-run --id` without `--push`** fails when `paths.integrations`
  cannot be read, where a plain dry-run succeeds, because the preview reads
  the integration state; unchanged by this PR's review response.
- **Ordering**: under a keyless pattern, `E_PATTERN_KEY_UNUSED` follows
  `create_dir_all`, as it did under `{tracker}` before.
- **`last-sync.json`** changes only because 0319 and 0371 were synced to
  Linear. Main's 0351 (PP-926) keeps its own entry.
