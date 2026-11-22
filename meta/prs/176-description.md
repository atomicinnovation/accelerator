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
revision: "f6906eec73605a9ce5888a608434c287135b96bf"
repository: "accelerator"
last_updated: "2026-11-22T22:06:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0281] Ask questions of and write reports from a research set

## Summary

Adds two verbs to `/accelerator:research-topic` for using a finished research
set. `ask SLUG QUESTION` answers in conversation from the set's findings and
synthesis. `report SLUG QUESTION` writes the answer to
`reports/<question-slug>.md`. Both refuse unless the set is synthesised, fresh
and fully conducted. Both state how well the findings cover the question, and
for a partial or unanswered question they offer to add a new outline round.
Until now, the research-topic verbs could build a set but offered no way to
consult it and find what it was missing.

## Changes

### Skill (`skills/research/research-topic/SKILL.md`)

- **Consulting the corpus** holds the steps `ask` and `report` share:
  - resolve the set, then run the three gates through
    `research topic consultable`;
  - read only the documents it lists;
  - answer with each part of the question marked `answered`, `partial` or
    `unanswered`, naming the sources and their tiers;
  - offer focus areas for a new round, appended to `outline.md` only when
    the user confirms.
- **Level notes:** before calling a part of the question a gap, both verbs
  read the level notes `consultable` lists under the findings bearing on it.
- **Re-check before writing:** before the first write after each reply, the
  verb takes the verdict again. It stops if the set is no longer
  consultable, or if its reports or highest round have changed since the
  last verdict, which means another session changed the set meanwhile. A
  report the verb itself wrote doesn't count as a change.
- **Refusals:** every refusal ends by naming the retry,
  `…then {verb} {slug} {question} again.` A held-back item's refusal names
  the fix for each reason it is held back.
- **Command rules:** the consulting verbs run each command on its own, with
  no redirect, and run no command the skill doesn't name. They write their
  reply straight after the last command, and never print it through a shell
  command.
- **`ask`** answers in conversation and writes nothing unless the user
  confirms a proposed round.
- **`report`** writes the report when the user accepts it, then sets the
  manifest's `report_count` from `research topic counts`. If an existing
  report asks the same question, `report` offers to rewrite it in place
  rather than add a near-duplicate.
- **Counts:** `conduct`, `synthesise` and `finalise` now take their counts
  and the freshness gate from the CLI instead of counting files.
- **Permissions:** a scoped `Read` grant on the citing-once rule removes that
  prompt. When a command after a reply is denied, the skill names four
  narrow allow rules.

### CLI (`cli/research*`, `cli/corpus*`)

- **`research topic report-path SLUG QUESTION`** prints the path a new
  report would take, in `layout/slug.rs`:
  - the question slug is cut before the word that would pass 60
    characters, or at 60 if that would leave 40 or fewer;
  - finding stems now follow the same rule;
  - a taken name gets the first free `-2`, `-3`, … suffix.
- **`research topic counts SLUG`** prints the round, finding and report
  counts derived from disk beside the ones the manifest and synthesis
  record. Each disagreement gets a fixed sentence. A drifted `report_count`
  is only a notice and never makes the set stale.
- **`research topic consultable SLUG`** applies the gates in order (status,
  freshness, fully conducted) and prints the first that fails:
  - For an open set it lists the documents to read, the level notes each
    composed finding came from, and `highest_round`, which a proposal
    numbers its round from.
  - Level notes are read only once every gate is open, through a lazy
    reader passed to `Consultation::judge`.
  - Each held-back item carries its `reasons` and the `recoveries` that fix
    them.
- **Plugin root:** `consultable` and `outstanding` find the research profiles
  under the plugin root, read through
  `config_adapters::plugin_root_from_env()`. Without it they fail with
  `E_TOPIC_RESEARCH_PLUGIN_ROOT`, and with no profiles directory, with
  `E_TOPIC_RESEARCH_PROFILES`.
- **Planner:** each outline item now records why it is held back
  (`HoldBack`). A question asked in several rounds is held back once, with
  the skipped profiles from every asking. A profile added when a question is
  asked again is researched under the first asking's index.
- **Domain types:**
  - `SetStatus` models the set's statuses, and an adapter test keeps it in
    step with the schema's `status_vocab`.
  - `topic::layout` names the set's documents and directories once.
  - A profile name no stem can use is now a plan `Warning`.
- **`report` document kind:**
  - a `report` kind and the `topic-research-report.md` template;
  - a closed `coverage` vocabulary, enforced by the new `BAD-EXTRA-VALUE`
    violation;
  - `extra_vocabs` on `print-schema` rows;
  - an optional `report_count` on the manifest.

  A bare `status:` is now `BAD-STATUS`.
- **`corpus metadata derive`** prints an `Author:` line from the checkout's
  configured `user.name`.

### Eval and test infrastructure (`tasks/eval/`, `tests/`)

- **`mise run eval:ask-and-report`:**
  - It runs 119 multi-turn headless sessions across 57 scenarios
    (`evals/scenarios/*.json`) over a reference corpus (`evals/project/`).
  - Each turn is judged against the criteria it declares, plus checks on
    what it reads and writes.
  - Before any session it checks the dev-launcher marker, the Claude Code
    version, the reference corpus (pre-flight) and a permission probe.
  - Run modes are `--check-only` (re-judge a finished run, which needs
    `--out`) and `--rerun-errored`. Both together are refused.
  - It refuses a `--family`, `--verb` or `--scenario` name that matches no
    scenario, and fails when nothing is selected, instead of passing with
    nothing judged.
- **`test:integration:eval-fixtures`** is part of the test roll-up and needs
  no model. It checks the reference corpus against the real CLI: every
  variant validates, every scenario reaches its declared gate, and the
  harness's rule for what counts as a finding agrees with the CLI's.
- **Shared helpers:** session running, transcript reading and document
  helpers moved out of `citing_once` into `tasks/eval/{headless,documents}`,
  and both evals use them. The pre-flight and the judges share one sources
  matcher and one frontmatter reader.

### Docs and meta

- **Docs:** `docs-site/.../research.md` documents `ask`, `report`,
  `report-path`, `counts` and `consultable`, including each one's JSON and
  exit codes, plus a settings snippet for the allow rules needed after a
  reply.
- **CHANGELOG:** entries under Breaking, Added, Changed and Upgrade notes.
- **Planning documents:** the plan, research, plan review and validation for
  0281, with 0121, 0284 and 0330 updated to match. The plan's
  **Review amendments**, the work item and the validation's
  **Since this validation** section record what each review pass changed.

## Context

- Work item: `meta/work/PP-865-corpus-consumption-ask-and-report.md`
- Plan: `meta/plans/2026-10-30-PP-865-corpus-consumption-ask-and-report.md`
- Research: `meta/research/codebase/2026-10-29-PP-865-corpus-consumption-ask-and-report.md`
- Validation: `meta/validations/2026-10-30-PP-865-corpus-consumption-ask-and-report-validation.md`
  (result: pass)
- First review: `meta/reviews/prs/176-review-1.md` (verdict: comment). Every
  inline thread is answered and resolved.
- Second review pass: a walkthrough of the PR. 7 findings from the first
  review and 36 new observations were fixed in this PR, in 24 changes.
- Third review: `meta/reviews/prs/176-review-2.md` (verdict: comment). All
  10 inline threads are answered and resolved. The 29 findings in its body
  are fixed in `46e50a49cd`..`aef1785c71`, and a PR comment maps each finding
  to its commit.

## Testing

- [x] `mise run` (the full local CI mirror, including `check` and the
  eval-fixtures pre-flight) exited 0 at `236cfa4249`. The commits after it
  change only `SKILL.md`, its structure test and `meta/`, and the Python
  suite passes on them.
- [x] `mise run eval:ask-and-report` after the third review, on Claude Code
  2.1.330: P1 6/6, R1 15/15 and G3 10/10. G3 first scored 9/10 twice:
  - one run filled the gap before its reply with `printf ''`;
  - after the first rule fix, one run printed its refusal through `printf`.

  Both are fixed by the rules that the reply comes straight after the last
  command and is never printed through a shell command.
- [x] `mise run eval:ask-and-report` ran all 119 sessions after the second
  review pass. 116 passed, and 3 failed by running a stray no-op shell
  command, the cause the third review's command rules address. L1 passed 3/3
  once its level note was given a sourced claim.
- [ ] The full 119-session eval has not been re-run since the third review.
- [x] `mise run eval:citing-once` passed 36/36 on Claude Code 2.1.330 after
  the second review pass.
- [x] Permission probe results are recorded for Claude Code 2.1.144 and
  2.1.316. On 2.1.330, a probe turn ran the skill with no `Skill` grant and
  no denial.
- [x] The smoke test's flows run as eval scenarios:
  - `report` writes a report and `report_count` (R1);
  - a second `report` lists the equivalent report, and `decline` writes
    nothing (E1);
  - a confirmed proposal appends one round and reopens the set (P1);
  - a `researching` set is refused (G1).

## Notes for Reviewers

- ⚠️ **Mixed-version teams:** an older binary's whole-corpus `validate`
  rejects the `report` kind as `UNKNOWN-KIND`. An older binary also computes
  a different stem for a long question, so teammates on either side of the
  upgrade redo each other's half-finished deep research. Upgrade a shared
  corpus together before anyone commits a report (CHANGELOG Upgrade notes).
- ⚠️ **Stricter `status`:** a frontmatter `status:` with no value, which
  passed validation before, now fails with `BAD-STATUS`. No document in this
  repository has one.
- ⚠️ **Long finding stems:** a question whose slug runs past 60 characters
  now cuts at a word. A `.levels/` tree that an interrupted deep run left
  under the old stem is researched again, and the old tree can be deleted
  (CHANGELOG Changed).
- **Permission prompts after a reply:** Claude Code keeps a skill's grant
  only until the user's next reply (0330 tracks this), so the commands `ask`
  and `report` run after a reply prompt for permission, and a headless
  session denies them. The CHANGELOG and docs give the four allow rules that
  avoid the prompt. The eval passes the skill's own declared grant, minus
  the `research fetch` rule neither verb runs, as `--allowedTools`.
- **Eval outside CI:** the eval is deliberately left out of `check`,
  `default` and `test`, and `test_mise.py` pins this. It needs the `claude`
  CLI, network access and model usage.
- **Size:** most of the +25.0k lines across 167 files are the plan (~3.9k
  lines), the plan review (~1.8k), the reference corpus fixtures and the
  scenario JSON. The production code is in `SKILL.md`,
  `cli/research{,-adapters,-cli}` and the frontmatter validation changes in
  `cli/corpus`.

https://claude.ai/code/session_01HEhrwSoJjp31mZmAMH7LFR
