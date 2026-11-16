---
type: "pr-description"
id: "176"
title: "[0281] Ask questions of and write reports from a research set"
date: "2026-11-01T21:06:04+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0281"
parent: "work-item:0281"
relates_to: ["work-item:0121", "work-item:0284", "work-item:0330"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/176"
pr_number: 176
tags: ["research", "research-topic", "ask", "report", "eval"]
revision: "34b7d2125394b8132e9a6422427b73385f2dfac5"
repository: "accelerator"
last_updated: "2026-11-16T23:55:20+00:00"
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
  appended to `outline.md` as a new round only on confirmation. The
  `freshness` and `conducted` refusals end by naming the retry:
  `…then {verb} {slug} {question} again.`
- **`ask`** — answers in conversation and writes nothing unless a proposal
  is confirmed.
- **`report`** — on acceptance writes the report, sets the manifest's
  `report_count` to the `derived.report_count` that `research topic counts`
  prints, and offers to rewrite an existing report on the same question in
  place rather than add a near-duplicate.
- `conduct`, `synthesise` and `finalise` now take their counts and the
  freshness gate from the CLI instead of counting files in prose.
- A scoped `Read` grant on the citing-once rule removes its permission
  prompt. A denied post-reply command names the narrow allow rules the skill
  itself grants.

### CLI (`cli/research*`, `cli/corpus*`)

- `research topic report-path SLUG QUESTION` — the path a new report takes:
  a question slug cut before the word that would pass 60 characters (or at
  60 when that cut would keep only the first word), with the first free
  `-2`, `-3`, … suffix (`ReportSlug`, `layout/slug.rs`).
- `research topic counts SLUG` — disk-derived round, finding and report
  counts beside the manifest's and synthesis's, with fixed wording per
  mismatch. Only `FreshnessCount`s gate freshness; `report_count` is
  informational (`topic/counts.rs`).
- `research topic consultable SLUG --profiles-dir DIR` — the gates in order
  status → freshness → fully conducted, printing the first failure or the
  documents to read (`topic/consultation.rs`). The plan marks an item that
  repeats an earlier item's question, so only a question's first asking can
  be held back.
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

- `mise run eval:ask-and-report` — 111 multi-turn headless sessions across
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
  allow rules; CHANGELOG entries under Added, Changed and Upgrade notes.
- Plan, research, plan review, validation and the first PR review for
  0281; 0121, 0284 and 0330 brought into line.

## Context

- Work item: `meta/work/0281-corpus-consumption-ask-and-report.md`
- Plan: `meta/plans/2026-10-30-0281-corpus-consumption-ask-and-report.md`
- Research: `meta/research/codebase/2026-10-29-0281-corpus-consumption-ask-and-report.md`
- Validation: `meta/validations/2026-10-30-0281-corpus-consumption-ask-and-report-validation.md`
  (result: pass)
- Review: `meta/reviews/prs/176-review-1.md` (verdict: comment). Every
  inline thread is answered and resolved, and the body's findings are
  answered in one PR comment.

## Testing

- [x] `mise run` (full local CI mirror, including `check` and the
  eval-fixtures pre-flight) exited 0 at `a50bcb38`, the branch tip before
  this description
- [ ] `mise run eval:ask-and-report` — run 5 passed 111/111 before the
  review fixes. Since then the refusal recovery lines, the allow-rule
  wording and a preamble sentence in `SKILL.md` changed and the harness was
  refactored, so a re-run (at least `--family G2,G3,R1,E2,P1`) is due
  before merge
- [x] Permission probe rows recorded on Claude Code 2.1.144 and 2.1.316
- [ ] Manual smoke test on a scratch copy: `report` accept-then-decline,
  `ask` confirming a proposal appends only the next round, `report` on a
  `researching` set refuses

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
- **Follow-up, not in this PR:** `conduct` never plans the profiles a
  repeated question adds in a later round. That behaviour predates this PR;
  `consultable` now agrees with it rather than reporting an empty hold-back.
- 0281's frontmatter `status` is still `ready`; move it on after merge.
