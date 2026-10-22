---
type: "pr-description"
id: "166"
title: "[0304] Cap web researchers at a per-level search budget and halt on exhausted quota"
date: "2026-10-17T17:38:52+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0304"
parent: "work-item:0304"
relates_to: ["work-item:0303", "work-item:0307", "work-item:0309", "work-item:0310"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/166"
pr_number: 166
tags: ["research", "deep-research", "conduct", "web-profile", "research-cli", "run-ledger"]
revision: "ac4fd932dd0e6276b89b3f174b011dc45c688965"
repository: "accelerator"
last_updated: "2026-10-22T11:50:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0304] Cap web researchers at a per-level search budget and halt on exhausted quota

## Summary

A deep `conduct` run could spend Claude Code's per-session `WebSearch` cap
part-way through, after which every web researcher received the search-limit
notice, followed its advice to "continue with the information already
gathered", and wrote notes from `WebFetch` or memory that `conduct` accepted as
research. This PR sizes a per-level `WebSearch` budget for every web researcher
so a run fits the cap, and when the quota still runs out, fails that node and
halts web researcher spawns for the rest of the run instead of degrading
silently.

## Changes

**Budget sizing (domain, `cli/research`)**

- `SearchCap` and `SearchBudgets` in `conduct/search_budget.rs`: keep a reserve
  of a twentieth of the cap, then binary-search the largest budgets with fixed
  level weights 4/2/1 whose demand over the worst-case tree fits, and top up
  deepest-first. When even budgets of 1 overrun, every budget is 1 and the plan
  warns. Budgets remember the cap they were sized against.
- `searches(profile)` is the one owner of the rule that only `web` researchers
  spend the cap; the sizing and the plan renderer both ask it.
- `WorstCaseTree` / `unsearched_tree` in `topic/tree.rs`: counts every web node
  a run may still spawn below the frontier, as a pure walk over a cloned
  `known` set so it records no trims.
- `Depth::requested` bounds a requested depth to 1–10; `Depth::new` stays
  unbounded so findings stamped deeper still read correctly.

**Search cap from the environment (`research-cli`)**

- `search_cap.rs` reads `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` with the
  same parse Claude Code uses (default 200). The shared parse moved out of
  `subagent_cap.rs` into `harness_setting.rs`. ADR-0073 admits the variable to
  the coupling guard.
- Every plan reports `search_cap` and its source. A `--start` with web pairs
  warns when the variable holds a value Claude Code would not honour.

**Budgets and withholding in the run ledger**

- Only a `--start` plan sizes the budgets. It stores them in
  `.conduct-run.json` as `search_budgets: {levels, demand, cap}` (optional,
  defaulting when absent, the `owed_quarantine` precedent), and later plans in
  the run reuse them; there is no mid-run resize. A ledger begun before budgets
  existed gets budgets of 1 with a warning, rather than a full-cap sizing after
  the session has already spent searches. A level the sizing never covered,
  such as one a later plan's larger `--depth` adds, gets 1 with a warning.
- Run plans report `search_budgets` as `{levels, demand, cap}`, and a
  `search_budget` on every web `single_pass` pair and node.
- `--withhold-profile P` (with `--run`) records `P` in the ledger, and the
  concurrency window then offers no researcher spawn of that profile while
  still offering non-web researchers and composers. A profile with no skill
  under `--profiles-dir` exits 1 with `E_TOPIC_RESEARCH_WITHHOLD_PROFILE` and
  leaves the ledger untouched. Run plans report `withheld`, and the ledger
  always writes it.
- Every plan lists its full outstanding set as `unanswered`; `pairs` stays the
  spawns offered now.
- `topic_command::planned` opens the run as an `OpenedRun` (`Query`,
  `Starting`, `Continuing`) whose methods carry the per-mode work, and returns
  its notices rather than pushing them through a shared vector. 0307, 0309 and
  0310 each add a method there, not another `RunMode` branch.

**Researchers and the skill**

- The web profile caps `WebSearch` calls at the injected budget, searching as
  the question needs when none is injected, and adds a **Quota exhausted**
  outcome: on the search-limit notice (session or per-turn wording) it writes
  nothing and replies "web search quota exhausted".
- Web level notes record `search_budget`, a new optional level-note extra
  (schema, template and outputter).
- `research-topic` defines halting failures in one table. A quota-exhausted
  spawn quarantines anything it wrote; from the halt on, every continue plan
  carries `--withhold-profile web` and must see it echoed in `withheld`;
  capacity-refused web spawns are not retried; step 7 still runs. The summary
  opens with a halt notice per withheld profile: the spawns that reported it,
  that profile's pairs left unfinished, and the recovery. The skill says why
  `conduct` proceeds past a predicted halt.
- One recovery clause reads alike in the skill, `configure`, the floor warning
  and docs-site, pinned by a structure test: run `/clear`, then re-run
  `conduct`; for later runs, raise `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION`
  and restart Claude Code, or lower the depth.
- `research-topic` clamps a configured depth above 10 to 10 with a warning;
  `configure` documents the new variable and the Claude Code version (v2.1.301)
  its default was read from.

**Docs and meta**

- `CHANGELOG.md` gives budgets and the halt their own Added bullet and puts the
  template-override step under Migrations. `docs-site/.../research.md`
  describes budgets, the halt (in a "Web search quota" section the `conduct`
  description links to, explaining why `/clear` recovers and waiting does not)
  and the depth ceiling.
- The work item, plan, its reviews, validation report, ADR-0073 and the PR
  review land here; the 0303, 0307, 0309 and 0310 work items are updated to
  hand over the integration points.

## Context

- Work item: `meta/work/0304-fail-web-nodes-on-websearch-quota-exhaustion.md`
- Plan: `meta/plans/2026-10-15-0304-budget-web-searches-and-halt-on-exhaustion.md`
- Validation: `meta/validations/2026-10-15-0304-budget-web-searches-and-halt-on-exhaustion-validation.md`
- ADR: `meta/decisions/ADR-0073-admit-the-session-search-cap-as-an-optional-harness-variable.md`
- PR review: `meta/reviews/prs/166-review-1.md`

## Testing

- [x] `mise run` (full local CI mirror) exits 0 at the branch tip
- [x] `mise run docs:check`: the site builds and every internal link resolves
- [x] New domain tests: `cli/research/tests/search_budget.rs` reproduces the
  work item's numeric acceptance criteria and property-tests fit, floor, shape
  and maximality, and covers the floor, the recorded cap and unbudgeted levels;
  ledger tests cover withholding, re-offered batches, the stored cap and
  non-canonical level keys
- [x] CLI integration: `cli/research-cli/tests/topic_outstanding.rs` covers
  `search_cap` sources, unhonoured values, sizing only on `--start`, the
  legacy-ledger floor, the recorded cap, unbudgeted levels, budget reuse, the
  floor warning, `withheld` (unknown profiles refused; the halted plan
  asserted and mutation-checked), `unanswered`, and the `--depth` ceiling
- [x] Skill structure and coupling: `test_research_structure.py` and
  `test_claude_coupling.py`, including the recovery clause and the depth
  ceiling read alike across the skill, `configure`, docs-site and the CLI
- [x] Manual halt run (headless, cap set low) recorded as done in the plan. It
  predates the review round, so the halt notice's labels and the withhold
  protocol as they now stand are covered by structure tests only
- [ ] Re-check at merge time that 0307 and 0309 are still not `done`; if
  either has landed, apply the plan's phase 4 step 8 or phase 5 step 4

## Notes for Reviewers

- ⚠️ `research topic outstanding --depth` now refuses a value above 10; a
  configured `research.topic.depth` above 10 clamps to 10 with a warning.
- ⚠️ A project that overrides `topic-research-level-note.md` should add the
  `search_budget` line, since the shared template must carry every optional
  extra.
- ⚠️ The budget fields ship nested as `search_budgets: {levels, demand, cap}`;
  the plan JSON is additive-only once released, so this shape is final. A
  ledger from an unreleased build of this branch, holding budgets without a
  `cap`, reads as corrupt: a `--start` replaces it with a notice, and a `--run`
  exits with `E_TOPIC_RESEARCH_RUN_LEDGER`.
- Waiting for the quota to refill is deliberately not a recovery: a re-run in
  the same session sizes against the full cap, not the remainder, so the
  recovery is `/clear`.
- ADR-0073 was edited after acceptance, inside this PR, to record the quota
  refill and why its variable is not admitted.
- Most of the line count is `meta/` (plan, reviews, validation); the code
  change is concentrated in `search_budget.rs`, `tree.rs`, `window.rs`, the
  two ledger modules and `topic_command.rs`.
