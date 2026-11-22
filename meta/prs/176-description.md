---
type: "pr-description"
id: "176"
title: "[0281] Ask questions of and write reports from a research set"
date: "2026-11-01T21:06:04+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "PP-865"
parent: "work-item:PP-865"
relates_to: ["work-item:PP-22", "work-item:PP-868", "work-item:PP-916"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/176"
pr_number: 176
tags: ["research", "research-topic", "ask", "report", "eval"]
revision: "dd50ea19766b634bd4c0d370043933fe2812b372"
repository: "accelerator"
last_updated: "2026-11-22T09:13:59+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0281] Ask questions of and write reports from a research set

## Summary

Adds the two consuming verbs to `/accelerator:research-topic`: `ask SLUG
QUESTION` answers from a set's findings and synthesis in conversation, and
`report SLUG QUESTION` writes a standalone answer to
`reports/<question-slug>.md`. Both consult only a synthesised, fresh, fully
conducted set, state their coverage, and on a partial or unanswered question
propose a new outline round — closing the build → consult → find a gap →
refine loop that the build verbs left open.

## Changes

### Skill (`skills/research/research-topic/SKILL.md`)

- **Consulting the corpus** — shared steps for both verbs: resolve the set,
  run the three gates via `research topic consultable`, read only what it
  lists, answer with per-part coverage (`answered` / `partial` /
  `unanswered`) and tiered sources, and propose focus areas that are
  appended to `outline.md` as a new round only on confirmation. Before
  calling a part a gap, both read the level notes `consultable` lists under
  the findings bearing on it. Every refusal ends by naming the retry:
  `…then {verb} {slug} {question} again.` The verdict is taken again before
  the first write after a reply.
- **`ask`** — answers in conversation and writes nothing unless a proposal
  is confirmed.
- **`report`** — on acceptance writes the report, sets the manifest's
  `report_count` to the `derived.report_count` that `research topic counts`
  prints, and offers to rewrite an existing report on the same question in
  place rather than add a near-duplicate. `accept` writes a new report at
  either prompt.
- `conduct`, `synthesise` and `finalise` now take their counts and the
  freshness gate from the CLI instead of counting files in prose.
- A scoped `Read` grant on the citing-once rule removes its permission
  prompt. A denied post-reply command names four narrow allow rules, the
  research ones limited to `consultable` and `counts`.

### CLI (`cli/research*`, `cli/corpus*`)

- `research topic report-path SLUG QUESTION` — the path a new report takes:
  a question slug cut before the word that would pass 60 characters (or at
  60 when that cut would leave 40 or fewer, the rule finding stems now share),
  with the first free `-2`, `-3`, … suffix (`ReportSlug`, `TakenNames`,
  `layout/slug.rs`).
- `research topic counts SLUG` — disk-derived round, finding and report
  counts beside the manifest's and synthesis's, with fixed wording per
  mismatch. Only `FreshnessCount`s gate freshness; a drifted
  `report_count` is a non-gating `notices` sentence (`topic/counts.rs`).
- `research topic consultable SLUG` — the gates in order status →
  freshness → fully conducted, printing the first failure or the documents
  to read, level notes included (`topic/consultation.rs`). It and
  `outstanding` find the profiles under `ACCELERATOR_PLUGIN_ROOT`. Each plan
  item carries its own `HoldBack`, and a repeated question's added profile is
  planned under its first asking's index.
- `research::topic::lifecycle::SetStatus` models the set's statuses, pinned
  by an adapter test to the corpus schema's `status_vocab`;
  `topic::layout::{FINDINGS, REPORTS}` name the set's directories once.
- A `report` document kind and `topic-research-report.md` template; a
  closed `coverage` vocabulary enforced by the new `BAD-EXTRA-VALUE`
  violation; `extra_vocabs` on `print-schema` rows; optional `report_count`
  on the manifest. A bare `status:` is now `BAD-STATUS`, as a bare extra is
  `BAD-EXTRA-VALUE`.
- `corpus metadata derive` prints an `Author:` line from the checkout's
  configured `user.name`.

### Eval and test infrastructure (`tasks/eval/`, `tests/`)

- `mise run eval:ask-and-report` — 119 multi-turn headless sessions across
  57 scenarios (`evals/scenarios/*.json`) over a reference corpus
  (`evals/project/`), each turn judged against the criteria it declares plus
  global read-bound and write checks. Start-up gates: marker, version,
  pre-flight, permission probe. Supports `--check-only` and
  `--rerun-errored`. Its run lives in `ask_and_report/orchestration.py`;
  live turns and replayed judging build records through one `TurnHistory`;
  findings are read by `layout.is_finding`, which an integration test pins
  to the paths `consultable` lists.
- `test:integration:eval-fixtures` (in the test roll-up) pre-flights the
  reference corpus on the real CLI with no model: every variant validates,
  every scenario reaches its declared gate, and the harness's finding rule
  agrees with the CLI's.
- Headless-session running, transcript reading and document helpers are
  extracted out of `citing_once` into `tasks/eval/{headless,documents}` and
  shared by both evals. `eval:citing-once --check-only` no longer needs the
  dev-launcher marker, as `ask-and-report`'s does not.

### Docs and meta

- `docs-site/.../research.md` documents `ask`, `report`, `report-path`,
  `counts` and `consultable`, with a settings snippet for the post-reply
  allow rules; CHANGELOG entries under Breaking, Added, Changed and Upgrade
  notes.
- Plan, research, plan review, validation and the first PR review for
  0281; 0121, 0284 and 0330 brought into line. The plan's **Review
  amendments** section and the work item record what the second review
  pass changed.

## Context

- Work item: `meta/work/PP-865-corpus-consumption-ask-and-report.md`
- Plan: `meta/plans/2026-10-30-PP-865-corpus-consumption-ask-and-report.md`
- Research: `meta/research/codebase/2026-10-29-PP-865-corpus-consumption-ask-and-report.md`
- Validation: `meta/validations/2026-10-30-PP-865-corpus-consumption-ask-and-report-validation.md`
  (result: pass)
- Review: `meta/reviews/prs/176-review-1.md` (verdict: comment). Every
  inline thread is answered and resolved, and the body's findings are
  answered in one PR comment.
- Second review pass: a walkthrough of the PR triaged 7 review-1 findings
  and 36 new observations as fix-in-PR, resolved in 24 changes; the PR
  comment of 2026-11-22 maps each to its commit.

## Testing

- [x] `mise run` (full local CI mirror, including `check` and the
  eval-fixtures pre-flight) exited 0 at `09e0d6d586`; the one commit after
  it changes only the plan and the work item, whose frontmatter validates
- [x] `mise run eval:ask-and-report`, all 119 sessions on Claude Code
  2.1.330 after the second review pass: 116 passed and 3 failed, each by
  running a stray no-op shell command (`echo skip`, `true`, and
  `consultable … 2>/dev/null`) that the Bash allow-list check rejects. The
  38 sessions that errored at the account's usage limit were rerun with
  `--rerun-errored`; L1 passed 3/3 on a fresh run after its level note was
  given a sourced claim
- [x] `mise run eval:citing-once` passed 36/36 on Claude Code 2.1.330, its
  first run since its session code moved into `tasks/eval/headless`
- [x] Permission probe rows recorded on Claude Code 2.1.144 and 2.1.316; a
  probe turn on 2.1.330 ran the skill with no `Skill` grant and no denial
- [x] The smoke test's flows are eval scenarios on this head: `report`
  writes a report and `report_count` (R1), a second `report` lists the
  equivalent report and `decline` writes nothing (E1), a confirmed proposal
  appends one round and reopens the set (P1), and a `researching` set is
  refused (G1)

## Notes for Reviewers

- ⚠️ **Mixed-version teams:** an older binary's whole-corpus `validate`
  rejects the `report` kind as `UNKNOWN-KIND`, so a shared corpus should be
  upgraded together before anyone commits a report (CHANGELOG Upgrade
  notes).
- ⚠️ **Stricter `status`:** a document whose frontmatter has a bare
  `status:` with no value, which validated before, now fails with
  `BAD-STATUS`. No document in this repository has one.
- **Permission prompts after a reply:** until Claude Code keeps a skill's
  grant across a reply (0330), the commands `ask` and `report` run after the
  user answers prompt for permission and are denied headless. The eval
  passes the skill's own declared grant (less the `research fetch` rule
  neither verb runs) as `--allowedTools`; the CHANGELOG tells users to add
  the three rules that cover the post-reply commands.
- The eval is deliberately outside `check`, `default` and `test`
  (`test_mise.py` pins this): it needs the `claude` CLI, network and model
  usage.
- Most of the +21.8k lines (158 files) are the plan (~3.7k), plan review
  (~1.8k), reference corpus fixtures and scenario JSON. The production
  surface is `SKILL.md`, `cli/research{,-adapters,-cli}` and the
  frontmatter validation changes in `cli/corpus`.
- ⚠️ **Long finding stems:** a question slugging past 60 characters now cuts
  at a word. A `.levels/` tree an interrupted deep run left under the old
  stem is researched again (CHANGELOG Changed).

https://claude.ai/code/session_01HEhrwSoJjp31mZmAMH7LFR
