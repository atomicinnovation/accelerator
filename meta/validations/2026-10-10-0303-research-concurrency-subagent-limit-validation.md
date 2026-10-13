---
type: "plan-validation"
id: "2026-10-10-0303-research-concurrency-subagent-limit-validation"
title: "Validation Report: Research Concurrency Within the Subagent Limit Implementation Plan"
date: "2026-10-12T02:33:04+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-10-10-0303-research-concurrency-subagent-limit"
tags: ["research", "research-topic", "conduct", "concurrency"]
last_updated: "2026-10-12T02:33:04+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Research Concurrency Within the Subagent Limit Implementation Plan

### Implementation Status

✓ Phase 1: Default of 20 - Fully implemented
✓ Phase 2: The Concurrency Rule in the Research Domain - Fully implemented
✓ Phase 3: `outstanding` Resolves Concurrency - Fully implemented
(its `--subagent-cap` transport was superseded by Phase 4, as planned)
✓ Phase 4: The CLI Reads the Subagent Cap - Fully implemented
✓ Phase 5: Capacity-Refusal Retry Rounds - Fully implemented

Every automated and manual criterion in the plan is checked off, and the
code matches each claim checked here.

### Automated Verification Results

✓ `mise run` exits 0 (373.7s). The run covered every check and test lane,
including `docs:build`, `docs:generate:check`, `docs:audit:check`,
`lint:claude-coupling:check`, `lint:skill-permissions:check`,
`public-api:check`, `pup:check` and `deny:check`. The working copy stayed
clean after the formatters and safe fixes ran.
✓ `rg -n -- '--subagent-cap|--limit \{concurrency\}|E_TOPIC_RESEARCH_LIMIT' skills cli/research-cli/src docs-site/src/content/docs/research.md CHANGELOG.md` prints nothing.
✓ `rg -ln CLAUDE_ cli` prints exactly `cli/research-cli/src/subagent_cap.rs`
and `cli/research-cli/tests/topic_outstanding.rs`.

### Code Review Findings

#### Matches Plan:

- `cli/research/src/conduct/concurrency.rs` matches the Phase 2 model plus
  Phase 4's `SubagentCap::first_honoured`. That covers the JavaScript-trim
  parity (U+0085 kept, U+FEFF stripped), the saturating digit fold, the
  silent clamp of the built-in default, and `Concurrency::new` as
  `pub(crate)`, so only resolution produces a `Concurrency` outside the
  crate.
- `window` takes a `Concurrency`. Its unit tests use the shared
  `test_support::{concurrency, unbounded}` helpers, and no `limit` remains
  in `window.rs`.
- In `topic_command.rs`, `resolve_concurrency` never reads config when a
  flag is given. Concurrency warnings are emitted on `RunMode::Start` only,
  ahead of the replaced-ledger notice. `PlanOutput` renders
  `"concurrency"` beside `"depth"`.
- `main.rs` resolves concurrency before `resolve_set` and before any
  ledger write, so an empty value exits 1 without writing a ledger.
- `subagent_cap::from_environment` reads `ACCELERATOR_SUBAGENT_CAP`, then
  `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`. A value the harness would ignore
  falls through to the next variable. The integration tests clear both
  variables in one `command()` helper (`topic_outstanding.rs:157-159`).
- The coupling guard admits by name and file. A line is flagged unless
  every `CLAUDE_*` name on it is admitted, and a missing admitted file
  raises, as a missing `_FILES` entry does.
- In `SKILL.md`, every plan command passes `{concurrency flag}` with no
  shell variable. The tick plan carries it too. The capacity-refusal
  rounds, the step 5 exclusion and the reason-table row match the Phase 5
  text exactly.
- `configure/SKILL.md`, `docs-site/.../research.md` and `CHANGELOG.md`
  carry the cap precedence and the default of 20.

#### Deviations from Plan:

- `subagent_cap::from_environment` has no doc comment, although the plan's
  code block gives it one. Commit `88a3dab7` removed it under the
  repository's comment policy. This is an improvement.
- In the Phase 5 partial-hold run, the holders were released at the first
  re-plan. The criterion's window (the batch message plus three one-spawn
  rounds) still completed before that point, as the plan records.
- ADR-0071 is drafted at status `proposed`, as the plan specifies, and has
  not been accepted.

#### Potential Issues:

- A plain `outstanding` query, which is the tick plan, now exits 1 when
  `research.topic.concurrency` is empty. Before this change the query read
  no research config. The Migration Notes document this, but a project
  with an empty value now fails at tick time rather than only at
  `--start`. The `--start` plan fails first in practice, so the exposure
  is small.
- Concurrency is resolved before the set is resolved. An unknown slug with
  an empty configured concurrency therefore reports the concurrency error,
  not the missing set. This is harmless, but the error ordering is a
  choice nothing pins.
- `std::env::var(..).ok()` treats a non-UTF-8 cap value as unset and falls
  through silently. That matches the "ignored value" rule, but no test
  covers it.

### Manual Testing Required:

The plan records all manual runs as completed on Claude Code 2.1.296. I did
not re-run them. After a harness upgrade, repeat these:

1. Cap reaches the CLI:
  - [ ] With `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS=8` exported, the first `conduct` plan reports `concurrency` 8 and raises no permission prompt
  - [ ] With `ACCELERATOR_SUBAGENT_CAP=4` also exported, it reports 4

2. Capacity refusals:
  - [ ] Recheck that the harness's refusal text still reads "Concurrent subagent limit reached"
  - [ ] Recheck that `a_cap_is_parsed_as_the_harness_parses_it` still matches the harness's parser

### Recommendations:

- Review and accept ADR-0071 with `/accelerator:review-adr`. It is still
  `proposed`, while the guard already enforces its decision.
- Move work item 0303 out of `draft`, which no longer reflects an
  implemented and validated item.
- Optionally pin the error ordering for an empty concurrency against an
  unknown slug, or resolve the set first, so that the error naming the
  more basic problem wins.
- Follow-ups 0330 (skill grants lapsing across queries) and 0331 are
  already captured. 0310's halting-failure early stop for retry rounds
  belongs to whichever of 0303 and 0310 lands second.
