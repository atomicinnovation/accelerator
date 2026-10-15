---
type: "work-item"
id: "0304"
title: "Budget Web Searches and Halt on Exhaustion"
date: "2026-10-09T17:58:47+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "ready"
kind: "story"
priority: "high"
parent: "work-item:0121"
blocked_by: ["work-item:0303", "work-item:0310"]
relates_to: ["work-item:0282", "work-item:0283", "work-item:0307", "work-item:0308", "work-item:0309"]
tags: ["research", "deep-research", "conduct", "web-profile"]
last_updated: "2026-10-15T19:06:05+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-890"
---

# 0304: Budget Web Searches and Halt on Exhaustion

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Size each web researcher's `WebSearch` budget from the session cap, and when
the quota still runs out, fail those nodes and stop spawning web researchers,
so a user running a deep `conduct` gets search-backed notes for every web
node or a clear way to recover.

Today deep runs exhaust Claude Code's session-wide quota, after which web
researchers silently fall back to WebFetch of URLs they already know, so
coverage depends on scheduling order rather than the question. Budgets make
an overrun unlikely, with a warning up front when even one search per node
will not fit. A new halting-failure class in `conduct`, whose only member is
quota exhaustion, stops the spawns that would fail, finishes the run's other
work, ends the run cleanly and prints the recovery.

## Context

A `--depth 3` run with 8 web pairs and 2 openalex pairs (130 nodes) exhausted
the quota around node 25–35; the ~90 web nodes after that worked only from
WebFetch of known primary sources. Discovery collapsed: later notes skewed
to well-known docs and missed community writing, issue threads and newer
material.

The quota is documented: 200 `WebSearch` calls per interactive terminal
session, shared by the main conversation and every subagent (Claude Code
v2.1.212+). `/clear` resets it; `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION`
raises it but cannot turn it off; from v2.1.290 it refills, by default at
~100 calls per hour (`CLAUDE_CODE_WEB_SEARCH_REFILLS_PER_HOUR`). At the cap
Claude Code raises no error: each further `WebSearch` call returns, in
place of results, a session-limit notice telling the model the search
limit is reached and to continue with what it has. So the web profile's
Denied outcome does not fire, and nothing reaches the orchestrator.

A `conduct` run researches pairs — one focus area under one source profile.
A web pair at depth above 1 is a tree of nodes, each writing a level note;
its frontier is the nodes not yet researched, and a composer then writes
the pair's finding from its level notes. A depth-1 (`single_pass`) pair
writes its finding directly.

The CLI can read `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` through the Bash
tool's environment, as `research topic outstanding` (`outstanding` below)
already reads
`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS` to bound concurrency. So `outstanding`
can know the cap, the resolved depth and the web pairs a run may still spawn.
It lists only each pair's current frontier, but the worst-case tree below
it follows from the depth and the follow-up caps — everything needed to
size per-node budgets and to tell before the first spawn whether the run
fits.

Falling back to known sources or background knowledge is not acceptable: a
note researched without search is lower quality and indistinguishable from a
searched one. Failing the node leaves it outstanding so a later run with
fresh quota researches it properly.

Quota exhaustion leaves the orchestrator running: it makes no `WebSearch`
calls of its own, so it can stop spawning web researchers, carry on with
the rest of the run, run step 7 (end the run, tick the outline, update
the manifest) and summarise.
That distinguishes it from an agent usage limit: researchers and composers
inherit the orchestrator's model, so a limit that stops them also refuses
the orchestrator's next turn, and recovery from that is 0310's, on the next
run. The halting-failure class therefore covers only failures the
orchestrator survives.

## Requirements

- **Search cap**: `outstanding` resolves the session search cap from
  `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION`, read as Claude Code parses it
  — Claude Code's behaviour is authoritative; it documents a positive whole
  number with no upper bound — else 200. Any other
  value, `0` included, falls back to 200, and when the run has web pairs
  `outstanding --start` reports it in a notice naming the value as
  unhonoured and the cap as 200 from the default.
  `outstanding` sizes against the cap in every environment, terminal or
  not, whether or not Claude Code enforces it there.
- **Reserve**: `outstanding` keeps ⌈cap / 20⌉ (5%, rounded up, in integer
  arithmetic) back for searches outside
  the run — the main conversation, `outline`, and anything the session spent
  before `conduct` began.
- **Budget allocation**: a plan is one `outstanding` invocation; the first
  plan of a run is its `outstanding --start`, and later plans in the same
  run are the invocations after it. On the sizing plan, `outstanding` sizes
  a per-level `WebSearch` budget over the worst-case tree of web nodes the
  run may still spawn, across every outstanding web pair, not only the
  first batch, to the resolved depth. It works at any depth:
  1. The worst-case tree counts, once each, every node of an outstanding
     web pair that has no level note on disk — frontier nodes, failed ones
     included, and follow-ups declared by written notes — and below each
     such node the follow-up caps to the resolved depth: 4 per level-1
     node, 2 per level-2 node, and 1 per node at every deeper level.
  2. Level 1 weighs 4, level 2 weighs 2, and every deeper level weighs 1.
  3. Only levels with remaining web nodes take a budget. The worst-case
     demand of a set of budgets is the sum, over those levels, of the
     level's worst-case node count times its budget. A shared scale factor
     `k > 0` sets each level's budget to `max(1, ⌊k · weight⌋)`. The
     candidate values of `k` are those at which `k · weight` is a whole
     number for some level with remaining web nodes; take the largest
     candidate whose budgets' worst-case demand fits within
     `cap − reserve`. If none fits, every budget is 1, step 4 is skipped
     and the floor warning applies.
  4. Then, while some level with remaining web nodes can rise by 1 without
     the total worst-case demand breaching `cap − reserve` and without its
     budget exceeding that of a shallower level, raise the deepest level
     that can.
- There is no ceiling: a budget is a maximum, not a target, so a small run
  may grant large budgets.
- Budgets are fixed for the run: the sizing plan stores them in the run
  ledger and later plans in the same run read them back rather than
  resizing. A re-run — a new run started after a halt — sizes afresh, so
  its budgets may differ. A ledger with no budgets, such as one written
  before this item, decodes as having none stored; the next plan in that
  run sizes and stores them. The sizing plan is whichever plan sizes a
  run's budgets: its first plan, or that next plan.
- **Plan report**: the sizing plan reports the per-level budgets, the
  worst-case demand, and the cap with its source; a run with no web pairs
  reports no budgets and no demand.
- Non-web nodes (openalex, arxiv) and composers draw no search budget and
  add no demand.
- **Budget delivery**: each web spawn carries its node's budget, and the web
  profile states "at most {search budget} `WebSearch` calls" in the same form
  as the openalex profile's call cap. The budget has one source: the plan.
- **Budget provenance**: a web level note records its node's budget as
  `search_budget` in its frontmatter, an allowance rather than a count of
  searches made. Other profiles' level notes omit it, and the level-note
  frontmatter schema admits it as an optional field. Findings, single-pass
  web ones included, carry no `search_budget`; a depth-1 run's single
  budget is in its plan report.
- **Floor warning**: when budgets of 1 at every level still exceed
  `cap − reserve`, `outstanding` keeps every budget at 1 and warns. The
  warning names the worst-case demand, the cap and its source, and the
  remedies: raise
  `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION`, lower `--depth`, or expect a
  halt and re-run. Only the sizing plan emits it; `conduct` prints it and
  proceeds.
- The web profile adds a **Quota exhausted** outcome: when a `WebSearch` call
  returns Claude Code's session-limit notice, the researcher writes no file
  — even if it already gathered pages — and its reply says "web search
  quota exhausted". It never completes the note from WebFetch of known URLs
  or from memory.
- `conduct` records a Quota exhausted node as failed with that reason, and
  the node stays outstanding.
- **Halting-failure class**: `conduct` defines, in one place, the failure
  reasons the orchestrator survives but after which some spawns in this
  session would fail or produce degraded work until the user takes a
  recovery step. Each reason names its recovery and the spawns it halts.
  Its only member is "web search quota exhausted", which halts web
  researcher spawns.
- **Halting**: a run becomes halted for a halting reason as soon as any
  spawn — from the batch message or a capacity-refusal round — ends with
  it. A halted run keeps going; it stops only the spawns that reason
  halts. From then on `conduct` spawns none of them, in the
  rest of step 4's (spawn the batch) refusal rounds and in every later
  batch. Step 5 (check each spawn) records the failed spawns as usual.
  Later batches still spawn the remaining non-web researchers, and
  composers for pairs whose level notes are all written, since neither
  makes `WebSearch` calls. Step 7 runs in full as normal —
  `end-run`, tick from disk, manifest update — so the re-run starts with no
  stale ledger.
- **Retry rounds**: in a halted run, capacity-refused spawns that the
  halting reason halts are not retried and keep the
  concurrent-subagent-limit reason; other refused spawns are retried as
  normal.
- **Run summary**: a halted run's summary opens with a halt notice, above
  Failures, naming the halting reason, the spawns that ended with it, the
  web pairs left unfinished — every web pair with outstanding web nodes
  when step 7 runs, whatever left them outstanding — and that reason's
  recovery. The spawns that ended with
  the halting reason are also listed under Failures.
- **Recovery**: for web search quota exhausted, run `/clear`, then re-run
  `conduct <slug>`; raise `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` for
  larger runs in later sessions. Waiting for the refill is not offered: a
  same-session re-run sizes its budgets against the full cap, not the
  refilled remainder, so it would halt again.
- The `conduct` failure-reason table has a row for each halting reason with
  its recovery, distinct from `rate_limited`. The "unfinished run" row and
  the "run … did not end" report of step 2 (report a run that did not end)
  are unchanged.
- The configure skill documents the search cap, its variable, the reserve
  and the floor warning beside "Concurrency and the subagent cap"; the
  docs-site `research topic outstanding` reference documents the plan's
  new fields, the unhonoured-value notice and the floor warning.

## Acceptance Criteria

- [ ] Given `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` unset and 10 web pairs
      at depth 3, when `conduct` starts, then the plan reports budgets of
      3/2/1 and a worst-case demand of 190.
- [ ] Given the cap unset and 3 web pairs at depth 3, when `conduct` starts,
      then the plan reports budgets of 15/6/3 and a worst-case demand of
      189.
- [ ] Given `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION=500` and 10 web pairs at
      depth 3, when `conduct` starts, then the plan reports budgets of
      11/5/2, a worst-case demand of 470, and a cap of 500 from
      `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION`.
- [ ] Given `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION=500` and 10 web pairs at
      depth 5, when `conduct` starts, then the plan reports budgets of
      7/4/1/1/1 and a worst-case demand of 470.
- [ ] Given the cap unset and 10 web pairs at depth 2, when `conduct`
      starts, then the plan reports budgets of 7/3 and a worst-case demand
      of 190.
- [ ] Given the cap unset and 2 web pairs at depth 5, when `conduct` starts,
      then the plan reports budgets of 11/5/3/3/2 and a worst-case demand of
      190.
- [ ] Given any run with web pairs whose budgets of 1 fit within
      `cap − reserve`, when `conduct` starts, then the reported demand is
      at most `cap − reserve`, every budget is at least 1, no level's
      budget exceeds a shallower level's, and no level can rise by 1
      without breaking one of those.
- [ ] Given the cap unset and 20 web pairs at depth 3, when `conduct` starts,
      then every budget is 1, the warning names a worst-case demand of 260
      against a cap of 200 from the default and the remedies — raise
      `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION`, lower `--depth`, or expect
      a halt and re-run — and the first batch is still spawned; later
      plans in the same run emit no floor warning.
- [ ] Given the cap unset and 15 web pairs at depth 3, when `conduct` starts,
      then every budget is 1 and the floor warning fires, because the
      worst-case demand of 195 exceeds `cap − reserve` of 190 though not
      the cap of 200.
- [ ] Given `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION=192` and 14 web pairs
      at depth 3, when `conduct` starts, then the reserve is 10, the plan
      reports budgets of 1/1/1 and a worst-case demand of 182, and no floor
      warning is printed.
- [ ] Given the cap unset, 8 web pairs and 2 openalex pairs at depth 3, when
      `conduct` starts, then the plan reports budgets of 7/2/1 and a
      worst-case demand of 184, counting web nodes only.
- [ ] Given `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION=abc` and 1 web pair at
      depth 3, when `conduct` starts, then the plan reports budgets of
      42/19/9 and a worst-case demand of 190, a notice names `abc` as
      unhonoured and 200 from the default, and later plans in the same run
      print no such notice.
- [ ] Given `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION=0` and 20 web pairs at
      depth 3, when `conduct` starts, then a notice names `0` as unhonoured
      and the floor warning names a cap of 200 from the default.
- [ ] Given `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION=abc` and a run with
      only openalex and arxiv pairs, when `conduct` starts, then the plan
      reports no budgets and no demand, and no search-cap warning or notice
      is printed.
- [ ] Given the cap unset and one web pair at depth 3 re-run after a halt,
      with its level-1 note and 2 of its 4 level-2 notes written, the two
      written level-2 notes declaring 1 and 2 follow-ups respectively, when
      `conduct` starts, then the remaining
      worst-case tree is 2 level-2 and 7 level-3 nodes, the plan reports
      budgets of 35 at level 2 and 17 at level 3 and a worst-case demand of
      189, and level 1, having no remaining web nodes, takes no budget and
      is not raised.
- [ ] Given `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION=201` and 1 web pair at
      depth 3, when `conduct` starts, then the reserve is 11 and the plan
      reports budgets of 42/19/9 and a worst-case demand of 190.
- [ ] Given `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION=60` and 1 web pair at
      depth 3, when `conduct` starts, then the reserve is 3 and the plan
      reports budgets of 13/5/3 and a worst-case demand of 57.
- [ ] Given the cap unset and 10 web pairs at depth 1, when `conduct`
      starts, then the plan reports a budget of 19 and a worst-case demand
      of 190, and the findings written carry no `search_budget`.
- [ ] Given a run ledger with no budgets, when `outstanding` reads it for
      the replaced-ledger report or a later plan, then it decodes without
      error, and a later plan sizes the budgets, stores them and reports
      them, emitting the floor warning when budgets of 1 do not fit.
- [ ] Given a web level note, when it is written, then its frontmatter
      carries `search_budget` equal to its node's budget; openalex and arxiv
      level notes carry no `search_budget`; and
      `accelerator corpus frontmatter validate` accepts both.
- [ ] Given the cap unset and 10 web pairs at depth 3 whose first plan
      sized budgets of 3/2/1, when every level-1 note declares 1 follow-up
      and a later plan in the same run is made, then the ledger still holds
      3/2/1 and every level-2 spawn carries a budget of 2, although resizing
      the remaining tree would grant 9/5.
- [ ] Given any web spawn, when its prompt is built, then it carries that
      node's search budget and the web profile limits `WebSearch` calls to
      it.
- [ ] The web profile's outcomes define Quota exhausted: on the
      session-limit notice the researcher writes no file, even with pages
      already gathered, never completes the note from WebFetch or memory,
      and replies "web search quota exhausted".
- [ ] Given a batch in which one or more spawns report quota exhaustion, when
      `conduct` checks the batch, then those nodes are recorded as failed
      with reason "web search quota exhausted" and no further web
      researcher is spawned in the run.
- [ ] Given a run with web and openalex pairs in which a web spawn reports
      quota exhaustion, when `conduct` continues, then later batches still
      spawn the outstanding openalex nodes and the composers of pairs whose
      level notes are all written.
- [ ] Given a batch where other spawns succeeded alongside a halting failure,
      when `conduct` checks the batch, then the successful spawns are
      retained and validated as normal.
- [ ] Given a run halted by a halting failure, when `conduct` ends, then
      `end-run` has removed the run's ledger, the outline checkboxes and
      manifest counts reflect disk, and the run summary opens with the halt
      notice naming the reason, the spawns that ended with it, the web
      pairs left unfinished and the recovery, and those spawns are also
      listed under Failures.
- [ ] Given 3 web pairs at depth 3 where pair A's level-1 note is written,
      pair B's level-1 spawn ends with quota exhaustion, and pair C is
      never spawned, when the halted run ends, then the halt notice names
      B's level-1 spawn as ending with the reason and A, B and C as left
      unfinished.
- [ ] Given a run halted by quota exhaustion, when `conduct` ends, then the
      halt notice offers `/clear` and a re-run, and raising
      `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` for later sessions, and
      does not offer waiting for the refill as a recovery.
- [ ] Given a batch message with capacity-refused web and openalex spawns
      and a spawn that ended with quota exhaustion, when `conduct` handles
      refusals, then the refused web spawns are not retried and are
      recorded with the concurrent-subagent-limit reason, and the refused
      openalex spawns are retried.
- [ ] Given a capacity-refusal round in which a re-issued web spawn ends
      with quota exhaustion, when `conduct` handles the spawns still
      refused, then the refused web spawns are not retried and the refused
      non-web spawns are.
- [ ] Given a run halted by quota exhaustion that has ended, when the next
      `outstanding --start` runs, then it reports no replaced ledger and
      lists the quota-failed nodes as outstanding.
- [ ] If 0307 has landed when this item lands, 0307's node query prints a
      web node's `search_budget` from the ledger snapshot, and a
      `single_pass` web spawn carries its budget inline.
- [ ] If 0309 has landed when this item lands, the web profile states the
      rule against writing from memory once, covering both the Denied and
      Quota exhausted outcomes.
- [ ] The skill defines halting failures in one place, with "web search
      quota exhausted" as its only member, and the failure-reason table has
      a row for it with its recovery.
- [ ] The "unfinished run" row and the step 2 "run … did not end" report are
      unchanged.
- [ ] The configure skill documents the search cap,
      `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION`, the reserve and the floor
      warning beside "Concurrency and the subagent cap"; the docs-site
      `research topic outstanding` reference documents the budgets, demand
      and cap fields, the notice and the floor warning.

## Open Questions

None — settled during drafting and review; see Drafting Notes.

## Dependencies

- Blocked by: 0303 (done) — capacity-refusal rounds and subagent-cap
  resolution; 0310 (done) — step 7 and the replaced-ledger report.
- Blocks: none known
- External: Claude Code's session search limit, as documented at
  code.claude.com when drafted — the default cap of 200,
  `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` and how it is parsed, the
  refill from v2.1.290, and the session-limit notice. A change to the
  notice can stop the Quota exhausted outcome firing and silently bring
  back the fallback. Before v2.1.212 there is no limit; the plugin supports
  v2.1.144, so on older versions budgets are needlessly small and no halt
  occurs.
- Related: 0303, landed, whose capacity-refusal rounds this item cuts short
  for halted spawns and whose subagent-cap resolution the search cap
  mirrors.
- Related: 0307, which narrows research-node spawns to a fixed set of
  references and adds snapshot fields to the run ledger. Whichever lands
  second makes 0307's node query print a web node's `search_budget`, and
  `single_pass` web spawns (depth-1 pairs researched in one pass, with no
  level notes) carry it inline. The ledger's budget field
  defaults when absent, so a ledger written before this item still decodes
  for 0310's report.
- Related: 0308, which also reads the run ledger; the budget field
  defaults when absent, so neither item depends on the other's encoding.
- Related: 0309, which generalises the web profile's rule against writing
  from memory, in the outcomes section this item extends. Whichever lands
  second states the rule once.
- Related: 0310, landed, which recovers on the next run from interruptions
  the orchestrator does not survive, agent usage limits included. This item
  adds no agent-usage-limit recovery, so 0310's "unfinished run" row keeps
  its inline wait-or-raise step, and the row and the step 2 "run … did not
  end" report are unchanged.

## Assumptions

- Claude Code's parse of `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` for
  edge inputs (sign, whitespace, leading zeros, decimals, overflow) is not
  documented; the implementation plan confirms it against Claude Code's
  behaviour and encodes each case as a test.
- A researcher can recognise the session-limit notice in its `WebSearch`
  result. Claude Code documents no exact wording for it, so the profile
  describes the notice rather than matching text.
- The CLI cannot see searches already spent in the session, including by an
  earlier `conduct` run, so the reserve is a guess and a run in a
  heavily-used session may still halt. The halt makes that recoverable.
- Claude Code documents the limit for interactive terminal sessions only.
  Budgets are sized against the cap everywhere, so outside the terminal they
  may be needlessly small and a floor warning may never be followed by a
  halt. Accepted.
- Budgets are prompt instructions, not enforced; a researcher that overruns
  its budget can exhaust the quota early. The halt makes that recoverable.
- Budgets differ between a halted run and its resumption, so sibling nodes
  may be researched with different budgets; `search_budget` records each.
  Accepted.
- On v2.1.290+ the refill can carry a long run past the cap, so budgets
  sized from the cap are conservative there.

## Technical Notes

- Web profile outcomes: `skills/research/profiles/web-profile/SKILL.md:57-70`
  (Denied at 62). The web profile has no `WebSearch` cap today.
- Budget phrasing to mirror:
  `skills/research/profiles/openalex-profile/SKILL.md:39` (hard-coded in the
  profile, not passed from the plan). The arxiv profile also forbids
  `WebSearch`: `skills/research/profiles/arxiv-profile/SKILL.md:49`.
- Cap resolution to mirror: `cli/research-cli/src/subagent_cap.rs`
  (variable parsing, `CapSource`) and
  `cli/research-cli/src/concurrency_wording.rs` (diagnostics that name
  their source). It reports an unhonoured value only for
  `ACCELERATOR_SUBAGENT_CAP`; a bad `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`
  falls back silently. Reporting an unhonoured
  `CLAUDE_CODE_MAX_WEB_SEARCHES_PER_SESSION` is new behaviour.
- Claude Code documents honouring only a positive whole number with no
  upper bound; anything else, `0` included, applies the default of 200.
- Cap notices are emitted only on `--start`
  (`cli/research-cli/src/topic_command.rs:262-269`, `start_notices` at
  429-442).
- Budget persistence: the run ledger `<set_root>/.conduct-run.json`
  (`cli/research-adapters/src/conduct/ledger.rs`;
  `cli/research/src/conduct/ledger.rs:45-49`) holds no budget, depth or cap
  today. `owed_quarantine` is the precedent for an optional field
  (`cli/research-adapters/src/conduct/ledger.rs:166-168`).
- `outstanding` lists only the current frontier
  (`cli/research/src/topic/tree.rs:161-184`); the worst-case tree is
  computed from depth and `follow_up_cap` (`tree.rs:48-57`). Follow-ups are
  deduplicated (`tree.rs:246`), so real counts never exceed the worst case.
  Sizing covers all of `plan.pairs`, not the window-narrowed batch
  (`cli/research/src/conduct/window.rs`).
- Level-note frontmatter: `templates/topic-research-level-note.md`,
  `skills/research/outputters/level-note-outputter/SKILL.md`, and the
  level-note row's `optional_extras` in
  `cli/corpus/src/frontmatter_validation/schema.rs:297`.
- Concurrency resolution in the skill:
  `skills/research/research-topic/SKILL.md:68-72`.
- Capacity-refusal rounds: `skills/research/research-topic/SKILL.md:424-446`.
- Spawn checks (step 5): `skills/research/research-topic/SKILL.md:448-460`.
- Step 7 and its summary order: `skills/research/research-topic/SKILL.md:476-531`.
- Failure-reason table: `skills/research/research-topic/SKILL.md:533-548`;
  "unfinished run" row (left unchanged) at 546.
- Step 2 report (left unchanged): `skills/research/research-topic/SKILL.md:269-290`.
- Configure docs: `skills/config/configure/SKILL.md:448-465`.
- Claude Code docs: `code.claude.com/docs/en/tools-reference` ("Session
  search limit"), `code.claude.com/docs/en/env-vars`.

## Drafting Notes

- The issue suggested a WebFetch fallback on exhaustion; the user rejected
  it in favour of failing the node, with `/clear` plus a re-run as the
  recovery.
- Budgets started as a fixed depth-scaled table (4/2/1); they became
  dynamic, sized from the resolved cap, so a run uses the capacity it has.
  No ceiling, per the user: a budget is a maximum, so unused capacity costs
  nothing.
- The 4/2/1 level weights mirror the follow-up caps' shape; the
  deepest-first top-up favours the many leaf nodes once each level has its
  weighted share. Both are my calls; any rule meeting shape, floor and fit
  would satisfy the user's intent.
- The reserve is 5% of the cap, rounded up, per the user, so it scales when
  the cap is raised.
- The cap warns rather than clamps because nothing can be clamped: budgets
  already shrink to fit, and below one search per node the only remedies
  are the user's.
- No `ACCELERATOR_*` override variable, unlike the subagent cap. The
  search limit is documented only for interactive terminal sessions, so
  elsewhere budgets may be needlessly small; the user chose to size against
  the cap everywhere and defer an opt-out until `conduct` runs off-terminal.
- The step 2 "run that did not end" report keeps its usage-limit line: it
  is printed verbatim and covers the account-wide limit the orchestrator
  did not survive, which remains 0310's case.
- The halting-failure class moved here from 0310 once it was clear an
  account-wide usage limit also stops the orchestrator, so halting works
  only for failures the orchestrator survives.
- "Agent usage limit" was dropped as a member during stress-testing:
  researchers and composers inherit the orchestrator's model, so the
  orchestrator rarely survives one, and keeping it would have forced a
  rewrite of 0310's row for a near-unreachable case.
- Waiting for the refill was dropped from the recovery during
  stress-testing: a same-session re-run cannot see the spent quota.
- Uneven budgets across a halt and re-run are accepted; `search_budget` in
  web level notes makes them visible. Searches made are not recorded: the
  model would self-report them, unverifiably.
- Running step 7 after a halt, stopping capacity-refusal rounds early, and
  placing the halt notice above Failures were agreed while reworking 0310.
- Kept as one story at review rather than splitting budget sizing from
  fail-and-halt: each half is the other's safety net — budgets make a halt
  rare, and the halt makes an overrun recoverable — so they ship together.
  The slug keeps the original framing; the id is the stable reference.
- A halt stops only the spawns its reason halts, per the user at review:
  non-web researchers and composers make no `WebSearch` calls, so quota
  exhaustion leaves them working and the run finishes them.

## References

- Source: https://github.com/atomicinnovation/accelerator/issues/141
- Related: 0121, 0282, 0283, 0303, 0307, 0308, 0309, 0310
- External: https://code.claude.com/docs/en/tools-reference,
  https://code.claude.com/docs/en/env-vars
