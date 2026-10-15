---
type: "pr-description"
id: "159"
title: "[0296] Fix sync round-trip defects with a tracker body dialect"
date: "2026-10-12T23:39:40+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0296"
parent: "work-item:0296"
relates_to: ["work-item:0300", "work-item:0320", "work-item:0337", "work-item:0335"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/159"
pr_number: 159
tags: ["sync", "jira", "linear", "adf", "tracker", "equivalence", "baseline"]
revision: "f5e8334b7b0d535b3947e5891bb99f8a9166fc91"
repository: "accelerator"
last_updated: "2026-10-15T10:17:41+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0296] Fix sync round-trip defects with a tracker body dialect

## Summary

`work sync` treated a tracker's projected body (`"{title}\n{description}"`)
as local content and compared bodies byte by byte. Pulls wrote the tracker
title above the H1, Jira pulls wrote raw ADF JSON, and Linear's cosmetic
re-serialisation showed up as changes and conflicts: on 2026-10-10 a preview
of this repository planned 7 pulls and 25 conflicts. This PR delivers the
whole 0296 plan (Phases 0–11). It adds a tracker body dialect, an extended ADF
converter, CommonMark body equivalence, and digest recipe `1`, which
classifies by meaning. The 2026-10-15 preview of the same corpus planned one
pull and three conflicts, each a real difference.

## Breaking changes

The `CHANGELOG` carries each of these.

- **`work sync` exit codes.**
  - A local file read or write failure exits `71`, where it exited `0`.
  - A pull whose remote body was not fetched exits `70` instead of writing an
    empty body.
  - A failed `show` after the remote stamp moved leaves the item
    `indeterminate` (exit `4`).
  - An item whose local frontmatter does not parse fails (exit `71`) and
    sends or writes nothing.
- **New `75` details.** `unrenderable` covers a pull over a Jira description
  with no Markdown form. `pull-residue` covers a send from a file an earlier
  pull left raw ADF in.
- **New report rows.**
  - `record-baseline` for a metadata-only local change.
  - `noop\tindeterminate\tunsupported-recipe` for a baseline entry a newer
    release wrote.
  - `failed\tsynced\tretryable` for a local file that changed mid-run.
- **Pending-push records move to schema 3**, which earlier releases refuse.
- **Jira Markdown in both directions.**
  - `jira show` and `--render-adf` render strike, blockquotes, pipe tables,
    rules and nested lists, and escape text that reads as syntax.
  - `jira create`, `update` and `comment` accept those constructs, which used
    to exit `41`.

## Migration

The first applied sync rewrites every entry it classifies in `last-sync.json`
under digest recipe `1`, as one large diff. It changes no work-item file and
sends nothing to the tracker.

- **Legacy keys kept.** Recipe-1 entries keep `local_hash` and `remote_hash`
  with their legacy meaning until 0335, so an older binary still reads them.
  An older binary that writes the file drops the recipe-1 keys, and the next
  newer run re-migrates them.
- **This repository.** 6 of 337 entries carry the marker today.

## Changes

### Title split and pulls (Phases 0, 1, 4)

- **Title strip.** `remote-projection` owns `split_projected`, which strips
  the title line by position. Every pull entry point and every dossier uses
  it.
- **Body dialect.**
  - `tracker` gains `BodyDialect`, a supertrait of `RemoteTracker`.
  - `local_body` renders a description to Markdown, and `as_pushed` predicts
    what a pushed body reads back as. `rejects` names bodies the tracker
    would refuse to store.
  - `PlainDialect` (Linear) is the identity; `JiraBodyDialect` goes through
    ADF.
- **Retitle.** `RemoteTracker::retitle` changes the title without resending
  the description; contract property `retitle_keeps_the_description` pins
  it.
- **Send refusal.** `SendRefusal` stops residue, unfetched remotes and
  pushes over an unrenderable remote. Preview reports the failure a planned
  pull or send would meet.
- **Fixtures.** Phase 0 captured Linear read-backs (`linear-rewrites/`) and
  five recovery fixtures (`work-item-sync-recovery/`) for 0275, 0276, 0293,
  0294 and 0302.

### `adf` crate (Phases 2–3)

- **Own crate.** `cli/adf` is a pure codec with its own public-API pin and
  `pup` rule. `work-adapters` may not import it by name.
- **Extended converter.** Both directions handle blockquotes, pipe tables,
  rules, nested lists, multi-paragraph list items and strike. Text that would
  read as syntax is escaped on output.
- **Strict render.** `document_to_markdown_strict` refuses any node with no
  Markdown form. It also refuses three bodies the dialect cannot read back:
  - a hard break in a table cell;
  - code text holding a backtick;
  - a code block holding a fence line.

### Body model and equivalence (Phases 5–6)

- **Parser.** `document::markdown` wraps `pulldown-cmark`, exact-pinned at
  `=0.13.4` with default features off, and does not expose it publicly.
- **Model.** `corpus::body` holds a minimal body model with sections and
  `CanonicalBody`. `corpus-adapters` implements the `BodyParser` port between
  the two.
- **Equivalence.** Bodies are equivalent when their canonical forms are
  equal, so the hash and the predicate cannot disagree. The canonical form
  ignores the eleven cosmetic rewrites Linear makes; fenced code, ordered
  start numbers and table alignment still count.

### Digest recipe 1 and classification (Phases 7–8)

- **Entry shape.** A baseline entry is legacy, split (`digest_recipe: 1`) or
  unsupported. Split entries hold separate frontmatter, body and field hashes
  per side.
- **Classification.** `classify` decides changes by equivalence:
  - noise is never a change;
  - equivalent bodies record a baseline without a file write;
  - a title change is sent alone, never with a stale description;
  - items with no usable baseline are recovered by equivalence.
- **Golden digests.** `golden_digests.rs` pins 40 body pairs and fails when
  `Cargo.lock`'s parser version changes without a recipe bump.
- **Promotion.** `BodyComparison` is the one place `work-adapters` applies
  body rules; promotion reuses it.

### Dossiers and promotion (Phases 9–10)

- **Dossiers and `work diff`.** Both list only sections whose meaning
  differs, each with its heading on both sides. A dossier header reads
  `title: <local> → <remote>` when the titles differ.
- **Promotion.** Promotion treats a remote as untouched when the titles match
  exactly and the bodies are equivalent.

### Fixes from plan validation

- **Strict ADF render.** Refuses the three bodies above, which otherwise
  wrote corrupt Markdown on pull.
- **Unparseable local file.** An item whose local frontmatter does not parse
  fails. Before, `--resolve <id>=local` sent `Update { title: "", body: "" }`
  and `=remote` wrote empty frontmatter.
- **Rejected Jira body.** A local body Jira's converter rejects is never
  equivalent.
- **Preview.** Fails a planned pull the run would fail.
- **Tests.** Adds the tests Phase 8 left unwritten, and brings 0296 in line
  with the plan's amendments.

### Outside the plan

- **Visualiser watcher.** Each rescan broadcasts every change it applied,
  where a rescan could absorb a change without sending an SSE event.
- **Real-repository suites.** The `bash-parity` feature that gated the git
  and jj suites is gone, so those suites always run.
- **`rustls` bump.** Pinned at 0.23.46 to clear RUSTSEC-2026-0346 to 0349.
- **Corpus.** A push-only sync linked 33 work items to Linear (`external_id`
  added).
- **Follow-up work items.** 0320, 0321, 0335, 0336, 0337 and 0338.

## Context

- Work item: `meta/work/0296-sync-round-trip-defects-between-local-work-items-and-the-tracker.md`
- Plan: `meta/plans/2026-10-10-0296-sync-round-trip-defects.md` (status
  `done`)
- Validation: `meta/validations/2026-10-10-0296-sync-round-trip-defects-validation.md`
  (`pass`)
- Research: `meta/research/codebase/2026-10-10-0296-sync-round-trip-defects.md`
- Review PR: #156

## Testing

Run on 2026-10-15 at the branch tip:

- [x] `test:unit:cli`: 5,675 passed, 2 skipped
- [x] `test:unit:frontend`: 2,615 passed
- [x] Every lint, type-check and `public-api:check` task in `mise run`
- [x] `mise run test:e2e:visualiser` on its own: 355 passed
- [x] `mise run` exits 0 (355 e2e tests passed)

Recorded in the plan as passing, not re-run for this description:

- [x] `test:integration:pup`, `test:integration:deny` and
      `test:integration:conformance`
- [x] Jira contract run against sandbox `KAN` and Linear against sandbox
      `TES`. Both include `retitle_keeps_the_description` and
      `update_without_a_title_keeps_the_title`.
- [x] `work sync --target KAN-18` writes readable Markdown, and a second
      preview reports `synced`
- [x] A promotion against Linear completes without a spurious retitle
- [x] Phase 7 migration on a scratch copy:
  - preview leaves `last-sync.json` byte-identical;
  - the applied run gives 276 of 300 entries recipe 1;
  - no work-item file changes.

## Merge-time corpus preview

`accelerator work sync --preview` on 2026-10-15 with the branch's
`accelerator-work` left `last-sync.json` byte-identical. It reported 332
items `synced`, one pull and three conflicts; the 2026-10-10 preview of the
same corpus planned 7 pulls and 25 conflicts. It predates the validation
fixes, which change no classification.

- **0184's pull was real, and harmful.** Linear's copy (`PP-714`) had lost
  the first characters of every code-span table cell, a rewrite 0320 owns, so
  applying the pull would have written that damage locally. A later commit
  pushed 0184's local body instead; whether Linear stored it intact was not
  re-checked.
- **Each conflict names a real difference.**
  - 0227: the local side drops two Dependencies entries.
  - 0296 and 0299: both were rewritten locally after their last push.
  - A later commit pushed all three, with 0184, and recorded their
    baselines.

| Item | 2026-10-10 | 2026-10-15 | Since 2026-10-10 |
|---|---|---|---|
| 0183 | pull | `synced` | |
| 0184 | pull | still differs | |
| 0197 | pull | `synced` | |
| 0204 | pull | `synced` | |
| 0209 | pull | `synced` | |
| 0229 | pull | `synced` | |
| 0257 | pull | `synced` | |
| 0065 | conflict | `synced` | |
| 0136 | conflict | `synced` | edited, pushed 2026-10-14 |
| 0158 | conflict | `synced` | |
| 0161 | conflict | `synced` | |
| 0169 | conflict | `synced` | |
| 0203 | conflict | `synced` | |
| 0207 | conflict | `synced` | |
| 0215 | conflict | `synced` | pushed 2026-10-14 |
| 0216 | conflict | `synced` | |
| 0217 | conflict | `synced` | pushed 2026-10-14 |
| 0227 | conflict | still differs | |
| 0241 | conflict | `synced` | pushed 2026-10-14 |
| 0263 | conflict | `synced` | |
| 0275 | conflict | `synced` | |
| 0276 | conflict | `synced` | |
| 0280 | conflict | `synced` | pushed 2026-10-14 |
| 0281 | conflict | `synced` | pushed 2026-10-14 |
| 0283 | conflict | `synced` | pushed 2026-10-14 |
| 0284 | conflict | `synced` | |
| 0286 | conflict | `synced` | pushed 2026-10-14 |
| 0292 | conflict | `synced` | |
| 0293 | conflict | `synced` | |
| 0294 | conflict | `synced` | |
| 0296 | conflict | still differs | edited |
| 0299 | conflict | still differs | edited |

0161, 0203, 0276 and 0293 report `synced` rather than staying conflicted
pending 0320. A sync before this branch recorded entries for them, so their
meaning-changing rewrites now sit inside the baseline.

## Notes for Reviewers

- **Diff size.**
  - 629 files change: 292 are renames (mostly ADF fixtures moving from
    `jira-client` to `adf`) and 151 are additions, many of them fixtures.
  - The code that matters is in:
    - `cli/adf/src`
    - `cli/tracker/src/lib.rs`
    - `cli/corpus/src/body`
    - `cli/document/src/markdown.rs`
    - `cli/work/src/sync/classify.rs`
    - `cli/work-adapters/src/sync/{body_comparison,baseline,run,send_refusal}.rs`
- **Reconcile pulls fail until 0300.** Production roots the work-item writer
  at the baseline directory, so every reconcile pull is refused as an unsafe
  path, and that refusal now exits `71`. The end-to-end pull-then-sync test
  waits on 0300. A `work-adapters` test covers the criterion for both
  trackers meanwhile.
- **Known gaps kept on purpose.**
  - A remote-only retitle is absorbed into the baseline without being
    reported, until 0290.
  - One item whose digest fails still aborts the run, as on `main`.
- **Flaky e2e test fixed.** The Escape-cancel test in
  `kanban-drag-overlay.spec.ts` wrote the fixture file and waited for the OS
  file watcher, which a loaded machine delivered late or not at all. It now
  edits and restores through the frontmatter PATCH, which broadcasts at once;
  breaking the cancel path still fails it. Separately, the watcher ignores
  FSEvents' "rescan" notification for dropped events, so a dropped burst is
  never re-indexed; that is left for a follow-up.
- **Branch base.** The branch sits one release bump and PR #163 behind
  `main`.

https://claude.ai/code/session_01MacKcLTsf6DzPrTVxm4PV2
