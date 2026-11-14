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
revision: "8e6ceb9c92509679fa44ccb90fa53fb1519f1d64"
repository: "accelerator"
last_updated: "2026-11-01T21:06:04+00:00"
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
  appended to `outline.md` as a new round only on confirmation.
- **`ask`** — answers in conversation and writes nothing unless a proposal
  is confirmed.
- **`report`** — on acceptance writes the report, bumps the manifest's
  `report_count`, and offers to rewrite an existing report on the same
  question in place rather than add a near-duplicate.
- `conduct`, `synthesise` and `finalise` now take their counts and the
  freshness gate from the CLI instead of counting files in prose.
- A scoped `Read` grant on the citing-once rule removes its permission
  prompt.

### CLI (`cli/research*`, `cli/corpus*`)

- `research topic report-path SLUG QUESTION` — the path a new report takes:
  a question slug cut before the word that would pass 60 characters, with
  the first free `-2`, `-3`, … suffix (`ReportSlug`, `layout/slug.rs`).
- `research topic counts SLUG` — disk-derived round, finding and report
  counts beside the manifest's and synthesis's, with fixed wording per
  mismatch (`topic/counts.rs`).
- `research topic consultable SLUG --profiles-dir DIR` — the gates in order
  status → freshness → fully conducted, printing the first failure or the
  documents to read (`topic/consultation.rs`).
- A `report` document kind and `topic-research-report.md` template; a
  closed `coverage` vocabulary enforced by the new `BAD-EXTRA-VALUE`
  violation; `extra_vocabs` on `print-schema` rows; optional `report_count`
  on the manifest.
- `corpus metadata derive` prints an `Author:` line from the checkout's
  configured `user.name`.

### Eval and test infrastructure (`tasks/eval/`, `tests/`)

- `mise run eval:ask-and-report` — 111 multi-turn headless sessions across
  57 scenarios (`evals/scenarios/*.json`) over a reference corpus
  (`evals/project/`), each turn judged against the criteria it declares plus
  global read-bound and write checks. Start-up gates: marker, version,
  pre-flight, permission probe. Supports `--check-only` and
  `--rerun-errored`.
- `test:integration:eval-fixtures` (in the test roll-up) pre-flights the
  reference corpus on the real CLI with no model: every variant validates
  and every scenario reaches its declared gate.
- Headless-session running, transcript reading and document helpers are
  extracted out of `citing_once` into `tasks/eval/{headless,documents}` and
  shared by both evals.

### Docs and meta

- `docs-site/.../research.md` documents `ask`, `report`, `report-path`,
  `counts` and `consultable`; CHANGELOG entries under Added, Changed and
  Upgrade notes.
- Plan, research, plan review and validation for 0281; 0121, 0284 and 0330
  brought into line.

## Context

- Work item: `meta/work/0281-corpus-consumption-ask-and-report.md`
- Plan: `meta/plans/2026-10-30-0281-corpus-consumption-ask-and-report.md`
- Research: `meta/research/codebase/2026-10-29-0281-corpus-consumption-ask-and-report.md`
- Validation: `meta/validations/2026-10-30-0281-corpus-consumption-ask-and-report-validation.md`
  (result: pass; the last six commits fix every "potential issue" it
  raised)

## Testing

- [x] `mise run` (full local CI mirror) exited 0 at `28f83f9ec3`, before the
  six post-validation fix commits
- [x] `mise run check` (the CI read-only set) exited 0 on the branch tip
- [x] `mise run eval:ask-and-report` run 5 passed 111/111 on the final skill
  text (~15 min); the commits since touch no skill prose
- [x] Permission probe rows recorded on Claude Code 2.1.144 and 2.1.316
- [ ] Manual smoke test on a scratch copy: `report` accept-then-decline,
  `ask` confirming a proposal appends only the next round, `report` on a
  `researching` set refuses

## Notes for Reviewers

- ⚠️ **Mixed-version teams:** an older binary's whole-corpus `validate`
  rejects the `report` kind as `UNKNOWN-KIND`, so a shared corpus should be
  upgraded together before anyone commits a report (CHANGELOG Upgrade
  notes).
- **Permission prompts after a reply:** until Claude Code keeps a skill's
  grant across a reply (0330), the commands `ask` and `report` run after the
  user answers prompt for permission and are denied headless. The eval
  pre-grants `Bash(accelerator corpus *)` and `Bash(accelerator research
  topic *)`; the CHANGELOG tells users to do the same.
- The eval is deliberately outside `check`, `default` and `test`
  (`test_mise.py` pins this): it needs the `claude` CLI, network and model
  usage.
- Most of the +20.8k lines are the plan (~3.7k), plan review (~1.8k),
  reference corpus fixtures and scenario JSON. The production surface is
  `SKILL.md`, `cli/research{,-adapters,-cli}` and the frontmatter validation
  changes in `cli/corpus`.
- 0281's frontmatter `status` is still `ready`; move it on after merge.
