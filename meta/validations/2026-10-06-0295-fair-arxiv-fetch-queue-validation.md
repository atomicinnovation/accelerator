---
type: "plan-validation"
id: "2026-10-06-0295-fair-arxiv-fetch-queue-validation"
title: "Validation Report: Fair arXiv Fetch Queue Implementation Plan"
date: "2026-10-06T20:18:21+00:00"
author: "Toby Clemson"
producer: "implement-plan"
status: "complete"
result: "pass"
target: "plan:2026-10-06-0295-fair-arxiv-fetch-queue"
tags: ["research", "arxiv", "pacing", "fetch"]
last_updated: "2026-10-06T20:18:21+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Fair arXiv Fetch Queue Implementation Plan

Result: pass. The three attended researcher checks pass 3 of 3, the planner
offers exactly 24 arXiv nodes at the default concurrency, and the re-run of
0283's step 17 loses 0 of 24 nodes to `lock_contention`. The first step 17
run produced no `waiting` outcome, so it was not evidence and was repeated;
the second is the judged run.

### Method

Each check ran as a headless `claude -p` session (Claude Code 2.1.292,
`claude-opus-5-5`) with `--plugin-dir` pointed at this tree, using the
contributor launcher override (`ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER=1`,
`ACCELERATOR_LAUNCHER_BIN`, and `ACCELERATOR_<SUB>_BIN` for each sub-binary
built from this tree) and `--permission-mode bypassPermissions`. The
installed plugin was disabled through `--settings`. Each session ran in its
own fresh scratch git repository, so no other session or older binary shared
its `arxiv.lock` or queue.

- **Attended checks.** One `arxiv` pair at `--depth 1`, so each session
  spawned one researcher. `ACCELERATOR_RESEARCH_BIN` pointed at a stub that
  `exec`s the real `accelerator-research` for every subcommand but `fetch`,
  so the research guard hook and `topic outstanding` ran for real. `fetch`
  answered from a per-repository call counter and logged its arguments.
- **Step 17.** Three `arxiv` pairs at `--depth 3` with default concurrency.
  For each pair, `1.md` was seeded with 4 follow-ups and every `2-k.md` with
  2, all 12 distinct from each other and from the pair question. Seeded
  notes took their path, `id` and `question` from `outstanding --depth 3`,
  and validate. `ACCELERATOR_RESEARCH_BIN` pointed at a timing wrapper that
  runs the real binary unchanged and logs each `fetch` invocation's start,
  end, arguments and printed status.
- **Response time.** For each invocation that settled `ok` or
  `unavailable`, its last request's send time comes from
  `arxiv-requests.log`, and its response time is the invocation's end minus
  that send.
- **Host.** Each step 17 run ran under `caffeinate -i`, so the host stayed
  awake.
- **Cost.** 9 attended sessions, about $4.65; 2 step 17 runs, about $18.33.

### Attended researcher checks

| Check | Result | Evidence |
|---|---|---|
| Re-presentation | pass 3 of 3 | invocations 2–4 carry `--ticket 7-a1b2c3` with unchanged query and `--limit`; 1 note each |
| `lock_contention` | pass 3 of 3 | 1 invocation; no note; summary gives the reason row's re-run remedy |
| Changed ticket | pass 3 of 3 | invocations 2–3 carry `--ticket 7-a1b2c3`, invocation 4 exactly one `--ticket 8-d4e5f6`; 1 note each |

In all 6 runs that reached `ok`, the researcher went on to make one further
search with a new query and no `--ticket`, within the profile's 3-search
cap. The plan's "4 `fetch` invocations" assumed the researcher would stop
after the settled search. The property the check targets holds in every
run: each waiting ticket was re-presented at once with unchanged arguments,
the latest ticket replaced the earlier one, no ticket leaked into a later
call, and exactly one note was written. Every written note and manifest
validates.

### Planner guard

`outstanding --start --limit 24` over the seeded step 17 set offers exactly
24 level-3 spawns, all `arxiv`, with `remaining` 0, and
`research.topic.concurrency` resolves to 24. In both runs the orchestrator's
first plan offered 24 level-3 nodes and it spawned all 24 as batch 1; in
run 2 the 24th waited for a subagent slot, as Observations describes.
`a_24_limit_offers_24_arxiv_spawns_and_counts_the_rest` and the catalogue
default assertion remain the enforcing tests.

### Step 17

| Measure | Run 1 | Run 2 |
|---|---|---|
| Median arXiv response | 0.33 s | 0.15 s |
| `waiting` outcomes / re-presentations | 0 / 0 | 5 / 5 |
| Verdict | not evidence | judged |
| Nodes lost to `lock_contention` | 0 of 24 | 0 of 24 |
| Other losses | 0 | 0 |
| Un-re-presented `waiting` tickets | 0 | 0 |
| `fetch` invocations / arXiv requests | 134 / 134 | 133 / 128 |
| `arxiv-contention.log` lines | 0 | 0 |
| Wall-clock, first fetch to last level-3 note | 498 s | 454 s |
| Fetch window | 459 s | 428 s |
| Peak concurrent `fetch` invocations | 19 | 20 |
| Longest / median invocation | 71.5 s / 37.9 s | 74.8 s / 36.5 s |
| Notes / findings written | 24 / 3 | 24 / 3 |

The median response time was well under the 3 s threshold in both runs, and
was read before either outcome.

Run 2's 5 `waiting` outcomes each stepped aside after about 67 s of queueing,
at position 2 or 3, without sending a request. Each was re-presented once
with unchanged arguments and returned `ok`. No call ran out of budget
mid-call, so no search was repeated. After the run, `arxiv-queue/` held only
`queue.lock`, no run ledger remained, and whole-corpus `accelerator corpus
frontmatter validate` exits 0.

For information:

- **arXiv idle time.** About 33 s over run 2's 128 requests, summing each
  gap minus the median response and the 3 s spacing. The longest gap was
  18.7 s; the shortest 3.03 s.
- **Re-presentations per node.** 5 over 24 nodes, about 0.2.
- **Repeated lookups.** 9 arXiv IDs were looked up by more than one
  researcher. These are separate nodes citing the same preprint, not
  repeats after mid-call exhaustion.
- **Researcher tokens.** The transcripts report usage for the orchestrator
  only; the whole run cost about $9.16.

`waiting` counts are not comparable with 0283's contention-log entries: a
`waiting` outcome keeps its place and is re-presented, where a 0283
contention entry lost its node.

### Observations

- 🟡 **Claude Code caps concurrent subagents at 20.** In run 2 the
  orchestrator hit the limit and launched the 24th researcher once a slot
  freed, and the peak of 20 concurrent fetches reflects it. The queue was
  therefore loaded by at most 20 researchers at once rather than 24. 0283's
  step 17 ran on 2.1.282; whether it met the same cap is not recorded.
  `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` raises it.
- 🟡 **Researchers fill truncated abstracts from memory.** The orchestrator
  reported that 10 of 24 researchers said they supplied mechanism details
  from their own knowledge because the CLI's abstracts are truncated, and
  that most notes do not flag it. This is outside 0295's scope.
</content>
</invoke>
