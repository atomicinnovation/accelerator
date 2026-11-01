---
type: "plan-validation"
id: "2026-10-30-0281-corpus-consumption-ask-and-report-validation"
title: "Validation Report: Corpus Consumption: Ask and Report Implementation Plan"
date: "2026-11-01T19:33:43+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-10-30-0281-corpus-consumption-ask-and-report"
tags: ["research", "research-topic", "ask", "report", "eval"]
last_updated: "2026-11-01T19:33:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Corpus Consumption: Ask and Report Implementation Plan

### Implementation Status

✓ Phase 1: The report document contract - Fully implemented
✓ Phase 2: Report paths from the CLI - Fully implemented
✓ Phase 3: Counts and consultation gates in the CLI - Fully implemented
✓ Phase 4: The reference corpus - Fully implemented
✓ Phase 5: The ask-and-report eval - Fully implemented
✓ Phase 6: `ask` and the shared consulting steps - Fully implemented
✓ Phase 7: `report` - Fully implemented

All 62 plan criteria and all 39 of 0281's acceptance criteria are ticked.
Every 0281 criterion maps to a row of the plan's "Acceptance criteria →
verification" table.

### Automated Verification Results

✓ Full local CI mirror: `mise run` exited 0 at `28f83f9ec3`, and its
formatters left the working tree unchanged.

The run was not repeated by hand:

- `mise run eval:ask-and-report`. It is on-demand and needs about 18 minutes
  of model usage. The plan records run 5 passing 111/111. The one commit
  after it, `28f83f9ec3`, touches no skill text and only removes docstrings.
- The manual CLI, probe and skill checks. Their results are recorded in each
  phase's "As built" note.

### Code Review Findings

#### Matches Plan:

- **Phase 1.** It adds:
  - `ExtraVocab` with the report row's closed `coverage` vocabulary;
  - `BadExtraValue`, plus its goldens and violation corpus;
  - the report and manifest templates, with an optional `report_count`;
  - the catalogue entry, the visualiser glyph and the CHANGELOG entry.
- **Phase 2.** It adds `ReportSlug` with the 61-character boundary tests,
  `research topic report-path` with free-name suffixes, `list_reports`, and
  its docs section.
- **Phase 3.** It adds:
  - `counts.rs`, `entry.rs` and `consultation.rs`, with lazy readers and the
    gate order status, then freshness, then outline;
  - the `counts` and `consultable` subcommands;
  - prose that makes conduct, synthesise and finalise take their counts from
    the CLI.
- **Phase 4.** The document helpers are shared, mutations form a closed set,
  and `gate_of` and the pre-flight are in place. The scenario files hold 57
  scenarios and 111 sessions.
- **Phase 5.** All 14 criteria are present, along with the global checks,
  the start-up gates (marker, then version, then pre-flight, then probe),
  `--check-only` and `--rerun-errored`. `test_mise.py:350-357` keeps the eval
  out of `check`, `default` and `test`.
- **Phases 6–7.** The `ask` and `report` prose matches the plan's quoted
  sections. The refusal and prompt templates in `formats.py` match SKILL.md
  exactly and are pinned as whole lines. The skill's grants cover every
  command the verbs prescribe before the reply, and the declared `Read` of
  the citing-once rule sits at `SKILL.md:23`.

#### Deviations from Plan:

None beyond those the "As built" notes record. `Toolchain.bootstrap`
(`tasks/eval/toolchain.py:24`) is the Phase 7 `PATH` fix. `list_reports`
returning `Vec<String>` (`cli/research-adapters/src/topic.rs:179-184`)
behaves as planned.

#### Potential Issues:

- **`--check-only` overwrites the run's recorded settings.**
  `tasks/eval/__init__.py:185,193` builds `RunSettings` with an empty version
  and with the CLI's `--model`, which is empty by default. `save_results` then
  overwrites the original `results.json`. A later `--rerun-errored` reads that
  empty model (`runs.py:347`), so a rerun can mix models within one run.
  A narrowed `--check-only` also rewrites `results.json` with only its subset.
- **A float count's message is misleading.** `topic.rs:304` prints the stored
  float with `f64::to_string`, so `round_count: 3.0` reads "records
  round_count '3', which is not a whole number".
- **A directory named like a document counts as one.** `is_visible_markdown`
  (`cli/research/src/topic/layout/entry.rs:22-27`) checks names only. A
  directory `reports/b.md` or `findings/01-x-web.md` is therefore counted,
  and `consultable` lists it as a file to read.
- **The probe accepts some denials.** `tasks/eval/ask_and_report/startup.py:63`
  fails the authentication probe only on `Read`, `Write` or `Edit` denials,
  not on any denial.
- **The eval's read-bounds checks have gaps** (`checks.py:62-99`):
  - a non-leading `..` passes;
  - a Grep's `glob` resolves against the project root, not its `path`;
  - a pathless Grep in `files_with_matches` mode can reach `.levels/`.
  A model is unlikely to take these paths, but the global checks would not
  catch one that did.
- **0281 contradicts itself.** Requirement
  `meta/work/0281-corpus-consumption-ask-and-report.md:85-92` still says the
  skipped-pair refusal names its recovery "instead of `conduct SLUG`". Its
  edited criterion and `SKILL.md:858-860` close with `conduct` then
  `synthesise`.
- **A docs section misses `report`.** `docs-site/src/content/docs/research.md:782`
  lists only `conduct`, `synthesise` and `finalise` as callers of `counts`.
  The intro at `:15` includes `report`.

### Manual Testing Required:

1. Interactive smoke test, from the plan's Manual Testing Steps:
  - [ ] Run `report llm-agent-planning How do agents plan tasks, and how do they recover from failed steps?` on a scratch copy under the dev launcher. Accept the answer, decline the proposal, and read the report and the manifest diff.
  - [ ] Run `ask` with C6, confirm the proposal, and check that `outline.md` gains only `## Round 4`.
  - [ ] Run `report` on a set in `researching` and read the refusal.

2. Eval:
  - [ ] Optionally re-run `mise run eval:ask-and-report` before release, under the marker.

### Recommendations:

- Make `--check-only` keep the recorded `RunSettings`, as `--rerun-errored`
  does, and test that a check-only run leaves `results.json`'s model intact.
- Render a stored float count with its decimal (`{number:?}`), or treat a
  whole-valued float as a type error with its own wording.
- Filter count entries to files at the `list_*` adapter, so directories
  never reach `CountedReports` or `is_finding_entry`.
- Fix 0281's requirement at `:85-92` and `research.md:782` to match what
  shipped.
- 0281's frontmatter `status` is still `ready`. Move it on once the PR lands.
