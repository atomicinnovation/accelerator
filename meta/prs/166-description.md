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
revision: "f2bc6a49d931db27e0fec611d60babb9e166f1e2"
repository: "accelerator"
last_updated: "2026-10-17T17:38:52+00:00"
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
  warns.
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

- A run's first plan sizes the budgets and stores them in
  `.conduct-run.json` (`search_budgets`, optional, defaulting when absent, the
  `owed_quarantine` precedent). Later plans in the run reuse them; there is no
  mid-run resize.
- Run plans report `search_budgets`, `search_demand`, and a `search_budget` on
  every web `single_pass` pair and node.
- `--withhold-profile P` (with `--run`) records `P` in the ledger, and the
  concurrency window then offers no researcher spawn of that profile while still
  offering non-web researchers and composers. Run plans report `withheld`.
- Every plan lists its full outstanding set as `unanswered`; `pairs` stays the
  spawns offered now.

**Researchers and the skill**

- The web profile caps `WebSearch` calls at the injected budget, and adds a
  **Quota exhausted** outcome: on the search-limit notice (session or per-turn
  wording) it writes nothing and replies "web search quota exhausted".
- Web level notes record `search_budget`, a new optional level-note extra
  (schema, template and outputter).
- `research-topic` defines halting failures in one table. A quota-exhausted
  spawn quarantines anything it wrote, the next plan carries
  `--withhold-profile web`, capacity-refused web spawns are not retried, step 7
  still runs, and the summary opens with the halt, the unfinished web pairs, and
  the recovery (`/clear`, then re-run `conduct`).
- `research-topic` clamps a configured depth above 10 to 10 with a warning;
  `configure` documents the new variable.

**Docs and meta**

- `CHANGELOG.md` and `docs-site/.../research.md` describe budgets, the halt and
  the depth ceiling.
- The work item, plan, its reviews, validation report and ADR-0073 land here;
  the 0303, 0307, 0309 and 0310 work items are updated to hand over the integration
  points.

## Context

- Work item: `meta/work/0304-fail-web-nodes-on-websearch-quota-exhaustion.md`
- Plan: `meta/plans/2026-10-15-0304-budget-web-searches-and-halt-on-exhaustion.md`
- Validation: `meta/validations/2026-10-15-0304-budget-web-searches-and-halt-on-exhaustion-validation.md`
- ADR: `meta/decisions/ADR-0073-admit-the-session-search-cap-as-an-optional-harness-variable.md`

## Testing

- [x] `mise run` (full local CI mirror) exits 0 at the branch tip
- [x] New domain tests: `cli/research/tests/search_budget.rs` reproduces the
      work item's numeric acceptance criteria and property-tests fit, floor,
      shape and maximality; ledger tests cover withholding, re-offered batches
      and non-canonical level keys
- [x] CLI integration: `cli/research-cli/tests/topic_outstanding.rs` covers
      `search_cap` sources, unhonoured values, sizing on first plan, budget reuse,
      the floor warning, `withheld`, `unanswered`, and the `--depth` ceiling
- [x] Skill structure and coupling: `test_research_structure.py` and
      `test_claude_coupling.py`
- [x] Manual halt run (headless, cap set low) recorded as done in the plan
- [ ] Re-check at merge time that 0307 and 0309 are still not `done`; if either
      has landed, apply the plan's phase 4 step 8 or phase 5 step 4

## Notes for Reviewers

- ⚠️ `research topic outstanding --depth` now refuses a value above 10; a
  configured `research.topic.depth` above 10 clamps to 10 with a warning.
- ⚠️ A project that overrides `topic-research-level-note.md` should add the
  `search_budget` line, since the shared template must carry every optional
  extra.
- Waiting for the quota to refill is deliberately not a recovery: a re-run in
  the same session sizes against the full cap, not the remainder, so the
  recovery is `/clear`.
- Known limitations accepted in the plan and validation: a level with no stored
  budget (only reachable by a hand run with a larger `--depth`) gets 1 without
  re-checking demand; `--withhold-profile` does not validate profile names
  (`conduct` only passes `web`).
- Most of the line count is `meta/` (plan, reviews, validation); the code
  change is concentrated in `search_budget.rs`, `tree.rs`, `window.rs`, the
  two ledger modules and `topic_command.rs`.
