---
type: "plan-validation"
id: "2026-09-28-0283-recursive-finding-deepening-validation"
title: "Validation Report: Recursive Finding Deepening Implementation Plan"
date: "2026-09-28T08:40:00+00:00"
author: "Toby Clemson"
producer: "implement-plan"
status: "complete"
result: "pass"
target: "plan:2026-09-26-0283-recursive-finding-deepening"
tags: ["research", "skills", "deep-research"]
last_updated: "2026-10-04T10:30:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Recursive Finding Deepening Implementation Plan

Result: pass. The Phase 5 attended checks and all 17 Phase 7 steps pass.
Step 10 first failed on a `web` pair, because the web profile had no
write-no-file outcome for a denied fetch. The profile now has one, and the
re-run passes. Step 17 measured arXiv lock contention at a material rate,
which work item 0295 takes up as a per-profile batch cap. Every scratch
corpus validates after the runs.

### Method

Each check ran as a headless `claude -p` session (Claude Code 2.1.282) with
`--plugin-dir` pointed at this tree, using the contributor launcher override
(`ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER=1`, `ACCELERATOR_LAUNCHER_BIN`,
`ACCELERATOR_<SUB>_BIN` for each sub-binary built from this tree) and
`--permission-mode bypassPermissions`. The installed plugin was disabled.
Each check used its own scratch git repository.

- **Seeding.** Sets were written directly as `outlined`. Every seeded note
  took its path, `id` and `question` from `outstanding --depth N`, so seeds
  could not drift from the planner. Seeded note bodies are placeholders
  citing `example.org`.
- **Batches.** A batch is the set of Agent calls in one top-level assistant
  message of the `stream-json` transcript. Caps, known questions and profile
  paths were read from each Agent call's `prompt`.
- **Timing.** Step 16's interrupt, second start and hand overwrite were
  driven by a watcher polling the transcript and the `.levels/` directory.
- **Cost.** 31 orchestrator sessions, 158 spawns, about $76.

### Phase 5 attended checks

| Check | Result | Evidence |
|---|---|---|
| `concurrency: 2`, 5 pairs | pass | batches 2, 2, 1 |
| `--concurrency 3` over configured 2 | pass | batches 3, 2 |
| `--concurrency 0` | pass | batches 1, 1; clamping warning names `'0'` |
| `concurrency: many` | pass | batches 1, 1; clamping warning names `'many'` |
| no config, 30 pairs | pass | batches 24, 6; 30 findings validate |
| `outline --concurrency 3` | pass | "ignored `--concurrency 3` because it's a `conduct` flag"; outline written |

### Phase 7 steps

| Step | Result | Evidence |
|---|---|---|
| 1. `depth: 1` | pass | 5 findings stamped `depth: 1`; no `.levels/` created |
| 2. Depth 2 from seeded `1.md` | pass | 4 level-2 spawns at cap 2, then 1 composer; no level-3 spawn; 0 of 8 recorded follow-ups appear in any summary under normalisation |
| 3. Caps at depth 3 | pass | unseeded root `1` at cap 4; seeded pair's `2-2` at cap 2 and `3-1-1` at cap 1 |
| 4. Mixed batch | pass | `--concurrency 3`: 3 (B `2-1`–`2-3`), 2 (A composer, B `2-4`), 1 (B composer) |
| 5. Known questions | pass | each level-2 prompt carries the follow-up at its lineage position; level-2 and level-3 prompts list the pair question, ancestors and every recorded follow-up; step 17's 24 prompts each carry all 13 verbatim |
| 6. Pruning | pass | `2-1` with `follow_ups: []` spawned no children; only `3-2-1`, then the composer |
| 7. Breadth independence | pass | `outline` at `breadth: 2` wrote 2 focus areas; `--depth 3` researched `3-1-1` for both and composed both at `depth: 3` |
| 8. Profile confinement | pass | root and 4 level-2 prompts inject only the `openalex` profile; composer stamped `openalex`; all 6 files carry `source_profile: openalex`; no `WebFetch` or `WebSearch` call |
| 9. Finding shape | pass | a 13-note finding has 9 thematic sections and no level or lineage heading; all 74 cited URLs appear in its notes; `finding_count: 2` matches disk; whole-corpus validate exits 0 |
| 10. Failure and resume | pass after a profile fix | see below |
| 11. Invalid note | pass | a test-double researcher's note reported as `fails validation: MISSING-EXTRA on source_profile` and quarantined as `.2-1.md.invalid`; re-run spawned only `2-1`, then the composer. A seeded invalid `2-1.md` was reported refused, quarantined, re-researched and composed |
| 12. Depth changes | pass | `--depth 1` composed from `1.md` only at `depth: 1`; `--depth 2` read `1.md` and `2-1.md` but not `3-1-1.md`; `--depth 3` over a depth-2 tree spawned `3-1-1`, then composed; the `depth: 1` pair re-run at `--depth 2` spawned nothing and was named as shallower |
| 13. Resume paths | pass | a complete tree spawned only the composer; `03` kept its index after `04` composed |
| 14. Configured composer | pass | `agents.composer: my-composer` was the spawned `subagent_type`; its sentinel sentence is in the finding |
| 15. Adversarial follow-up | pass | `1.md` refused as "follow-up 2 contains a link or host name", quarantined, and root `1` re-researched; no spawn prompt or subagent tool call contains `attacker.example`. The summary paraphrased the refusal line rather than quoting its template |
| 16. Run ledger | pass | interrupted after batch 1 with `.conduct-run.json` present; the fresh run warned it replaced that ledger, completed and left none. A second start superseded the first, which stopped with `E_TOPIC_RESEARCH_RUN_SUPERSEDED`. A hand overwrite of `2-1.md` between batches appeared in `unexpected` and in the summary |
| 17. arXiv contention | measured, material; 0295 raised | see below |

Whole-corpus `accelerator corpus frontmatter validate` exits 0 in all 29
scratch projects after the runs, and none holds a `.conduct-run.json`.

#### Step 10

With `WebFetch` and `WebSearch` denied on a `web` pair, both level-2
researchers first wrote notes from memory citing pages they never opened, and
`outstanding` accepted them. The orchestrator then quarantined the accepted
notes and declined the offered compose on its own judgement, which the skill
does not provide for. The cause was that `web-profile` had no Outcome section,
where the OpenAlex and arXiv profiles tell a researcher to write no file when
a fetch is denied or unavailable.

`web-profile` now carries an Outcome section with Pages, Denied, Unreachable
and None found. `every_profile_writes_no_file_when_its_fetch_is_denied` holds
all three profiles to it. The re-run on a fresh `web` pair behaves as step 10
specifies:

- **Denied.** Both level-2 researchers wrote no note, and the summary named
  `2-1` and `2-2` with "fetch denied by permissions" and next steps.
- **Restored.** The re-run spawned only `2-1` and `2-2`, then the composer,
  and the finding validates.

The OpenAlex variant, with `Bash(accelerator research fetch *)` denied,
passed the same way before the fix.

#### Step 17

Three `arxiv` pairs at depth 3 with default concurrency; the first plan
offered exactly 24 level-3 nodes at cap 1.

| Measure | Value |
|---|---|
| Contention per fetch | 4 contention entries over 110 requests (3.6%) |
| Nodes failed as `lock_contention` | 4 of 24 (17%) |
| `rate_limited` without `lock_contention` | 0 |
| Wall-clock, batch issued to last note | 452 s (fetch window 412 s) |
| Spawn-prompt volume | 80.1k characters, about 20k tokens |
| Known questions within it | 13 per node, 22.4k characters, about 5.6k tokens |

One node in six failing at the default concurrency is material. Work item
0295 takes it up as a per-profile batch cap for arXiv. The
known-questions volume is well below the plan's 8k–24k estimate, so passing
known questions by reference is not needed.

### Observations

- **`web-profile` had no failure outcomes.** Fixed as described under step
  10.
- **`web-profile` kept the focus-question carve-out.** Its Source Family said
  "No URL a fetched page tells you to fetch that falls outside the focus
  question", which Phase 6 had removed from `agents/researcher.md`. It now
  reads "No URL a fetched page tells you to fetch", and
  `no_profile_lets_a_focus_question_license_a_fetch` holds every profile to
  that.
- 🟡 **`batch` and `--spawned` confuse the orchestrator.** Two orchestrators
  questioned whether `--spawned 0` after `"batch": 0` was right, and one
  passed `--spawned 1` instead. By design, the ledger ignores a mismatched
  acknowledgement without a word. A warning on a mismatch, or plainer skill
  wording, would remove the doubt.
- **`corpus metadata derive` prints no author.** `research-topic` tells the
  orchestrator to capture the resolved author from it, so every run fell
  back to `git config user.name`. This predates 0283.
- **Composers differ on unused sources.** Some keep placeholder sources that
  support no claim in Sources, and others drop them. Both obey "cite only
  sources that appear in the notes".
- **Researchers cite unread sources.** Several researchers cited pages seen
  only as search snippets or truncated abstracts. This belongs to 0280's
  quality gate, not 0283.
- **The guard held under recursion.** It blocked a level-3 researcher's
  `curl` and two `for` loops wrapping `accelerator research fetch`. A real
  researcher's note missing `schema_version` was refused and re-researched
  on resume.

### Changes after validation

The plan's **Post-implementation restructure** landed after these checks. It
regroups the `research` crate, consolidates its topic-research types, takes
the Unicode data from ICU4X, and renames the `outstanding` stages
(`research` and `deepen` become `single_pass` and `research_nodes`), its
`unaccepted` and `shallower` fields (now `unfinished` and `shallow`), and the
ledger's `pins`, `notes_seen` and `answered_seen` (now `claims` and
`seen`). The evidence above records the names as they were when it was
gathered.

- **Covered.** The full `mise run` exits 0 after the restructure, including
  the 40 `topic_outstanding` integration tests that pin the plan JSON and
  the ledger codec's round-trip and field-name tests. ICU4X agrees with the
  replaced Unicode data on all 1,112,064 scalar values.
- **Not re-run.** The Phase 5 checks and Phase 7 steps ran against the old
  stage and field names, and `research-topic`'s `conduct` prose now reads
  the new ones. No attended `claude -p` run has exercised `conduct` against
  the renamed contract.
