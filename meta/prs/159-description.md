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
relates_to: ["work-item:0300", "work-item:0337", "work-item:0335"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/159"
pr_number: 159
tags: ["sync", "jira", "linear", "adf", "tracker"]
revision: "415a9c9fa94dbf9f6ca2b93453885ce90db52de7"
repository: "accelerator"
last_updated: "2026-10-12T23:39:40+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0296] Fix sync round-trip defects with a tracker body dialect

## Summary

`work sync` treated a tracker's projected body (`"{title}\n{description}"`)
as local content. Every pull wrote the tracker title as a stray first line
above the H1, and a Jira pull wrote the raw ADF JSON into the work item.
This PR delivers Phases 1–4 of the 0296 plan: pulls strip the projected
title, Jira descriptions render to Markdown through an extended ADF
converter, and tracker-specific body rules sit behind a new `BodyDialect` on
the tracker port. Phases 5–11 (body equivalence, digest recipe `1`,
equivalence-aware classification) follow in later PRs.

## Breaking changes

- **`work sync` exit codes.** A local file read or write failure now exits
  `71` (`terminal`) where it exited `0`. A pull whose remote body could not be
  fetched exits `70` (`retryable`) instead of writing an empty body. A `show`
  that fails after the remote stamp moved leaves the item `indeterminate`
  (exit `4`).
- **New `75` details.** A pull or push over a Jira description with no
  Markdown form fails as `unrenderable`. A send from a file that an earlier
  release's pull left raw ADF JSON in fails as `pull-residue`. In both cases
  nothing is written or sent.
- **Jira Markdown output.** `jira show` and `jira search --render-adf` now
  render strike, blockquotes, pipe tables, rules, nested lists and later
  list-item children. They also backslash-escape any text that would read as
  syntax, including ordinary `*`, `_` and `[` in prose.
- **Jira Markdown input.** `jira create`, `update` and `comment` now accept
  blockquotes, pipe tables, rules and nested lists that used to exit `41`.
  They also read `~~x~~` as strike, honour backslash escapes and keep an
  ordered list's start number.

Scripts that parse `work sync` exit codes, or that snapshot `jira show`
output, need updating. The `CHANGELOG` carries all four entries.

## Changes

### Title split (Phase 1)

- `remote-projection` gains one splitter that takes the title line off a
  projected body by position. The reconcile pull, `--resolve <id>=remote`,
  targeted pull-create and the conflict dossier all use it, so none of them
  writes the title line.
- Local I/O failures are classified as terminal, and a failed `show` after a
  stamp change is classified as indeterminate.

### `adf` crate (Phases 2–3)

- `cli/jira-client/src/adf/` moves to a new `cli/adf` technical library. It
  is a pure codec over `serde_json`, held there by a `pup.ron` rule that
  forbids spawning processes and filesystem access, and it has its own
  public-API pin.
- The converter extends past the frozen bash oracle in both directions:
  blockquote, pipe table, rule, nested lists, multi-paragraph list items and
  `~~strike~~`. Text the tokeniser would read as syntax is escaped on the way
  out, so every strict render reads back as itself.
- `adf_differential.rs` keeps the bash-oracle comparison, with an exact list
  `DELIBERATE_DIVERGENCES` naming each case the extension changes and why.
- A strict render, `document_to_markdown_strict`, refuses any node with no
  Markdown form instead of emitting an `[unsupported ADF node: …]`
  placeholder. A mark with no Markdown form, such as `underline`, is still
  dropped silently.

### Tracker port and Jira pulls (Phase 4)

- `tracker` gains `BodyDialect`, a supertrait of `RemoteTracker`. It turns a
  description into the local Markdown body and returns a pushed body as the
  tracker would hold it after reading it back. `PlainDialect` (Linear) passes
  bodies through unchanged; `JiraBodyDialect` goes through the ADF
  conversion.
- `RemoteTracker::retitle` changes the title without resending the
  description. This avoids the round-trip loss a full update would cause
  (Linear `issueUpdate { title }`, Jira `PUT fields.summary`). It is pinned
  by a new `retitle_keeps_the_description` contract property.
- In `work-adapters`, `BodyComparison` is the single place that applies body
  rules for sync and promotion. A new `pup` rule forbids `work-adapters` from
  importing `adf` by name.
- `SendRefusal` checks each planned send before it reaches the tracker, and
  refuses residue, an unrenderable remote under a push, or an unfetched
  remote. Preview reports the refusal the run would meet.
- A conflict over an unrenderable remote writes a dossier carrying
  `reason: remote-unrenderable` and no sections. The `sync-work-items` skill
  then offers a reduced `[local/skip]` prompt and gains a table of failure
  details.

### Outside the plan

- **Visualiser watcher.** Each rescan now compares the index before and
  after itself and broadcasts every change it applied. Before this, a rescan
  for another file, or an aborted burst task, could absorb a change so that
  no SSE event was sent.
- **`implement-plan` instructions.** This repository's `implement-plan`
  instructions now ask for small, frequent commits.
- **New work items.** 0337 (a rich document model for corpus documents) and
  0335 (remove the legacy digest recipe) capture follow-up work.

## Context

- Work item: `meta/work/0296-sync-round-trip-defects-between-local-work-items-and-the-tracker.md`
- Plan: `meta/plans/2026-10-10-0296-sync-round-trip-defects.md` (Phases 1–4
  complete; the Phase 0 fixture capture is deferred because only Phase 5
  onwards reads the fixtures)
- Research: `meta/research/codebase/2026-10-10-0296-sync-round-trip-defects.md`
- Plan review: `meta/reviews/plans/2026-10-10-0296-sync-round-trip-defects-review-1.md`
- Review PR: #156

## Testing

Run while writing this description, on the branch tip:

- [x] `mise run check` (the CI read-only set) exits 0
- [x] `mise run cli:check` and `cargo test -p adf --test adf_oracle_manifest`
      pass

Recorded in the plan as passing at each phase's commit, but not re-run for
this description:

- [x] `mise run` (the full default task) exits 0 after each of Phases 1–4
- [x] `mise run test:integration:pup`, which `mise run` does not include,
      passes with the new `adf` and `work-adapters` probe pairs
- [x] `mise run public-api:check` after reviewing the snapshot diffs for
      `adf`, `remote-projection`, `tracker` and `work`
- [x] Jira contract run against sandbox project `KAN` (4 tests, including
      `retitle_keeps_the_description`) and Linear contract run against
      sandbox team `TES` (5 tests)
- [x] `accelerator jira create` with a table, blockquote and nested list
      renders correctly in the Jira UI, and `jira show --render-adf` reads it
      back to the same Markdown
- [x] `work sync --target KAN-18` writes readable Markdown with no title
      line, and a second `--preview` reports the item `synced`
- [x] `work sync --preview` on this repository: Linear dossiers no longer show
      the title line

Not done:

- [ ] Phase 0 fixture capture (needs credentials for the Linear workspace;
      Phases 1–4 don't read the fixtures)

## Merge-time corpus preview

`accelerator work sync --preview` on 2026-10-15 with the branch's
`accelerator-work`, `last-sync.json` byte-identical afterwards: 332 items
`synced`, one pull and three conflicts. The 2026-10-10 preview of the same
corpus planned 7 pulls and 25 conflicts.

- **0184's pull is real, and harmful.** Linear's copy (`PP-714`, updated
  2026-09-22) has lost the first characters of every code-span table cell
  (`` `config templates list` `` reads `onfig templates list\``), a
  meaning-changing rewrite 0320 owns. Applying the pull would write that
  damage into the local file.
- **Each conflict's dossier names a real difference.** 0227 lists only
  Dependencies, where the local side drops two entries. 0296 and 0299 have
  no baseline entry, and every section differs because both were rewritten
  locally after their last push; 0296 also shows its title change.
- **No legacy entry migrates in a preview.** None of the 332 entries carries
  `digest_recipe` yet; the first applied run rewrites them.

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
pending 0320: a sync before this branch recorded entries for them, so their
meaning-changing rewrites now sit inside the baseline.

## Notes for Reviewers

- **Diff size.** 458 files change, but 287 of them are ADF fixture renames
  from `jira-client` to `adf`, and 75 are added fixtures. The code that
  matters is in `cli/adf/src`, `cli/tracker/src/lib.rs`,
  `cli/jira-client/src/body_dialect.rs`,
  `cli/work-adapters/src/sync/{body_comparison,send_refusal,run}.rs` and
  `cli/work/src/sync/{decide,plan,pull_residue}.rs`.
- **Reconcile pulls fail until 0300.** Production wires `SyncPorts.writer` to
  the `FileCorpusStore` rooted at the baseline, which refuses every work-item
  write from a reconcile pull as an unsafe path. That refusal now exits `71`
  where it exited `0`, and
  `a_pulled_item_syncs_again_as_synced_with_no_update` is deferred until 0300
  lands.
- **Escaping is narrower than CommonMark.** The escape set covers what this
  tokeniser reads, plus `_` and `~`. Other CommonMark constructs (`<`, `&`, a
  setext underline after a hard break, block markers indented by 1–3 spaces,
  a heading ending in ` #`) stay bare. Phase 6's equivalence check is meant to
  absorb those differences.
- **Overwrite naming.** `Action::Overwrite` (a conflict resolved in the local
  side's favour) is a domain action of its own, but the `work sync` report
  still shows it as `push`.
- **Flaky e2e test.** `kanban-drag-overlay.spec.ts` failed several local
  `mise run` attempts and then passed on re-run. Its cause is unidentified.

https://claude.ai/code/session_01VRwQKbqR5bGxwMidwSk3SR
