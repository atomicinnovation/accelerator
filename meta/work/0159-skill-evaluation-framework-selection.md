---
type: work-item
id: "0159"
title: "Skill Evaluation Framework Selection"
date: "2026-06-27T12:23:42+00:00"
author: Toby Clemson
status: done
kind: spike
priority: high
external_id: "PP-182"
tags: [spike, evaluation, skills, tooling]
last_updated: "2026-06-27T12:23:42+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0159: Skill Evaluation Framework Selection

**Kind**: Spike
**Status**: Done
**Priority**: High
**Author**: Toby Clemson

> **Ported from luminosity.** This spike was conducted in the downstream
> luminosity project and is imported here as a faithful historical record of the
> research that feeds Accelerator's skill-evaluation decision. Its `## Spike
> Outcome` and `## Recommendation` are preserved as captured (including the
> toolchain count and `configure`-not-yet-built framing as they stood in
> luminosity); bare luminosity work-item numbers have been removed. The
> luminosity original is linked in `## References`.

## Summary

Survey the landscape of skill-evaluation approaches and recommend one, so the
`configure` skill can be the first concrete evaluation target and skill quality
becomes measurable. Time-boxed to 2 days.

## Context

The Rust-CLI direction requires skill evaluation but the author is not familiar
with skill-evaluation approaches. The decision is genuinely open: the spike
surveys the available frameworks broadly rather than choosing from a fixed
shortlist. `skill-creator`, promptfoo, and DeepEval are known starting points,
not the limit of what is considered.

## Requirements

Research questions to resolve, time-boxed to **2 days**:

- Survey the landscape of skill / agent-instruction evaluation frameworks and
  weigh them on merit for: fit with the skills-as-product model, dev-time and
  CI usage, task/assertion authoring effort, variance reporting, and
  maintenance/longevity risk. The following are known starting points and useful
  reference shapes, not an exhaustive shortlist — actively look for others during
  the spike:
  - skill-creator — the one purpose-built for evaluating a skill as a skill
    (native with-skill vs baseline, evals.json tasks, assertion grading, pass-
    rate/token/time deltas as mean ± stddev); interactive/subagent-driven, not
    CI-native as-is.
  - promptfoo — declarative matrix (skill-vs-no-skill is one extra prompt row),
    --repeat for variance, g-eval/llm-rubric/select-best grading, Claude Agent
    SDK provider, native exit codes. Longevity flag: OpenAI acquisition
    (Mar 2026); still MIT.
  - DeepEval — pytest-style authoring, GEval/custom LLM-judge metrics, regression
    comparison; with/without is hand-built; no first-class repeat/variance.
- Recommend an approach (a single framework or a hybrid), and define how a
  `configure` pass-rate / benchmark result is produced and surfaced. Note: the
  preference is to run evals during development with version-controlled benchmark
  results rather than on every CI build, to control token cost — the
  recommendation should account for that execution model.

Output: a written recommendation with a decision, recorded in this spike work
item, feeding the eval-framework ADR. The "Apply the eval framework to the
`configure` skill" work references this spike.

## Acceptance Criteria

- [x] When the spike concludes, this work item contains a Recommendation section
      naming the chosen framework (or hybrid) and the rationale, including any
      longevity risks. The rationale addresses each weighing dimension (fit with
      the skills-as-product model, dev-time and CI usage, task/assertion
      authoring effort, variance reporting, maintenance/longevity risk) for each
      surveyed framework — e.g. as a comparison table or per-dimension note.
- [x] The Recommendation lists every framework considered — at least
      skill-creator, promptfoo, and DeepEval, plus any surfaced during the
      survey — each with a one-line reason for inclusion or dismissal.
- [x] The recommendation defines how a `configure` pass-rate / benchmark result
      is produced and run during development with results committed to the repo
      (not on every CI build). The definition names the command / entry point
      used to run the eval, the repo path where committed benchmark results live,
      and the result file format.
- [x] The recommendation states an explicit final floor (task count and
      pass-rate) with a written rationale, referencing the 2025–2026 guidance
      (20–50 real-failure-derived tasks with multi-trial variance) for why the
      provisional floor (≥ 3 tasks, ≥ 80% pass-rate) was kept or changed.
      The 20–50 figure is the external benchmark the provisional floor is
      reconsidered against, not a competing requirement.
- [x] The eval-framework ADR cites this work item.

## Spike Outcome

- **Date**: 2026-06-24
- **Time spent**: ~0.5 day of the 2-day box (research-led, no prototype — the
  eval target `configure` did not yet exist when this ran, so the harness is
  selected on mechanism rather than a live run; see Residual Risks).
- **Verdict**: Adopt **UK AISI Inspect** as the committed, threshold-gating
  skill-evaluation harness, run as a new `tests/evals/` test tier via `mise` and
  deliberately excluded from CI. `skill-creator` is retained as an optional
  complementary authoring aid, not the gating harness. promptfoo is the strong
  runner-up.

## Recommendation

**Adopt Inspect (UK AISI, `inspect.aisi.org.uk`) as the skill-evaluation
harness.** Skill evals are treated as a third **test tier** — `tests/evals/` —
alongside `unit/` and `integration/`, with both the eval definitions and their
committed results living under the test path, never under `skills/`. Evals run
on demand during development via `mise` and are **excluded from the default CI
sweep** to control token cost. `skill-creator` is kept as an optional, interactive
authoring aid; promptfoo is the documented runner-up.

This reverses the original epic's leading hypothesis (`skill-creator` as the likely
choice, Technical Note §"Skill evals"). The decisive constraint, surfaced during
the spike, is the requirement to **run evals from a local `mise`+invoke build task
and fail at a threshold**. `skill-creator`'s task-quality loop is interactive and
subagent-driven inside a live Claude Code session with a human reviewing a browser
viewer — there is no headless command that runs tasks, grades them, and exits
non-zero below a bar (verified in the installed source,
`skills/skill-creator/SKILL.md:163-269`). It therefore cannot be the automation
harness, though it remains useful for interactive authoring.

### Per-dimension comparison (frameworks weighed on merit)

The four frameworks that can plausibly serve as the harness, scored against the
brief's five weighing dimensions:

| Dimension | **Inspect** (chosen) | promptfoo (runner-up) | skill-creator | DeepEval |
|---|---|---|---|---|
| **Fit with skills-as-product** | A/B is hand-written (skill-loaded vs baseline solver over one dataset); can drive Claude Code as an external agent. Good, not turnkey. | **Best**: native `skill-used` assertion + Claude Agent SDK provider + documented "test agent skills" workflow. | **Purpose-built** for evaluating a skill *as a skill* (with-skill vs baseline is the core concept). | A/B hand-built; agent-native metrics but not skill-specific. |
| **Dev-time / CI usage** | **Python-native, in-process** `eval()` inside an invoke task; no new toolchain. Threshold gate is ~3 lines (read `pass_k` off `log.results`, `SystemExit(1)`). | Headless `promptfoo eval` with **built-in** `PROMPTFOO_PASS_RATE_THRESHOLD`→exit code, but a **Node** subprocess (4th toolchain). | **Not automatable** for task pass-rate — interactive/in-session only. | pytest-native exit codes; Python. |
| **Authoring effort** | Python `@task` + dataset; more code, but **same language as the build system** (lints under existing ruff/pyrefly). | Declarative YAML matrix — lowest effort. | `evals.json` (low effort) but only usable via the in-session loop. | pytest-style Python. |
| **Variance reporting** | **Best**: native `pass_at(k)` **and `pass_k(k)`** (all-k-succeed = pass^k), plus `stderr`/`bootstrap_stderr` via `--epochs`. | `--repeat N` pools trials into a global **mean** pass-rate; **no native pass^k** (open issue #5847). | `aggregate_benchmark.py` gives mean ± stddev over k runs/config + delta; no pass^k. | **None native** — no repeat/variance (verified still true mid-2026). |
| **Maintenance / longevity** | **Lowest risk**: MIT, UK-government-backed (AISI), very active. | MIT but **acquired by OpenAI (9 Mar 2026)**; still actively shipping (added Claude support Jun 2026) — risk is strategic drift, not abandonment. | Anthropic first-party, ships with Claude Code; updated only occasionally. | Apache-2.0; small (seed-stage) company — runway risk, though OSS core would survive. |

**Why Inspect wins for this repo**: it satisfies the hard requirement
(headless, threshold-gating, from a build task) on the *correct* metric — pass^k
is the methodologically right statistic for a hard regression gate (2026
guidance), and Inspect is the only candidate that computes it natively. It adds
**no new language toolchain** (the repo is deliberate about its three: Python /
shell / Rust; Inspect is Python, installs via `uv`, runs in-process in an invoke
task), and carries the lowest longevity risk. promptfoo's headline advantage (a
built-in exit-code gate) gates on a pooled *mean*, not pass^k, and costs a Node
toolchain plus acquisition-drift risk; its genuine win is turnkey skill A/B.

**Qualitative-analysis loop**: Inspect covers the "read what happened → improve
the skill" loop. Inspect View (`inspect view` / VS Code extension) shows full
per-sample transcripts, tool calls, and model-grader rationales
(`Score.explanation`); `read_eval_log` exposes the same programmatically, so
Claude in a skill-writing session reads committed logs and proposes edits. What
Inspect lacks vs `skill-creator` is a packaged *UX*: typed-freeform-feedback
capture and a blind A/B version comparator. That keeps `skill-creator` a useful
optional convenience for hands-on authoring, not a necessity.

### Every framework considered

Serious harness candidates (weighed above) plus everything surfaced by the
landscape survey, each with a one-line include/dismiss reason:

- **Inspect (UK AISI)** — *Chosen.* Python-native, native pass^k threshold
  gating, lowest longevity risk, no new toolchain.
- **promptfoo** — *Runner-up.* Best turnkey skill A/B + built-in exit-code gate,
  but Node toolchain, no native pass^k, OpenAI-acquisition drift risk.
- **skill-creator (Anthropic)** — *Retained, optional.* Purpose-built for skills
  and first-party, but its task-quality loop is interactive/in-session and cannot
  be a headless gating harness; useful for interactive authoring + blind A/B.
- **DeepEval (Confident AI)** — *Dismissed.* pytest-native and Python, but no
  native variance/repeat (verified) and small-company longevity risk.
- **OpenAI Evals / simple-evals** — *Dismissed.* OpenAI-centric; Claude is not a
  first-class provider.
- **LangSmith (LangChain)** — *Dismissed.* Best-documented skills A/B *case
  study*, but the eval store is SaaS-centric (not a committed-file model).
- **agentevals / OpenEvals (LangChain OSS)** — *Dismissed.* Useful trajectory
  evaluators but a pre-1.0 library, not a runner; no A/B or gating of its own.
- **Braintrust** — *Dismissed.* Strong CI/git-metadata story but commercial
  platform; committed-file model weaker than Inspect.
- **Langfuse** — *Dismissed.* Self-hostable OSS, but observability-first; heavier
  than needed for committed skill A/B.
- **Arize Phoenix** — *Dismissed.* Trace/observability-led; lighter on pass@k/^k
  statistics.
- **TruLens (Snowflake)** — *Dismissed.* Agent-trace feedback functions, but
  observability-oriented and Snowflake-adjacent.
- **Ragas** — *Dismissed.* RAG-evaluation heritage; weak fit for skill A/B.
- **Galileo (Cisco)** — *Dismissed.* Commercial production-monitoring platform;
  no committed-file model.
- **Maxim AI / Latitude** — *Dismissed.* Newer hosted/commercial agent-eval
  platforms; SaaS-centric.
- **Benchmarks (SkillsBench, SkillTester, OctoBench, Terminal-Bench)** — *Noted
  as methodology references, not harnesses* — useful for eval-design inspiration,
  not for running our suite.

### Operational model (how a `configure` benchmark is produced and surfaced)

- **Entry point / command**: `mise run eval:skills:configure` (a leaf invoke
  task), with a roll-up `mise run eval` for the whole tier. The invoke task calls
  Inspect's `eval()` **in-process** with `epochs=Epochs(k, pass_k(k))`, reads the
  `pass_k` metric from `log.results`, and exits non-zero below the floor. The
  eval tier is **excluded from the default `mise run` / `check` sweep** so it
  never runs on every CI build.
- **Repo path (definitions + committed results)** — both under the test path,
  organised by skill under test:

  ```
  tests/evals/skills/configure/
  ├── configure_eval.py     # Inspect @task: dataset + with-skill/baseline solvers + scorer
  ├── dataset.jsonl         # the eval tasks (≥ floor)
  └── results/
      └── <timestamp>.json  # committed Inspect log (--log-format json)
  ```

- **Result file format**: Inspect eval log written as **JSON** (`--log-format
  json`, not the default binary `.eval`, so it diffs in VCS), committed by hand
  during development.
- **Implementation note**: name eval files so pytest does not collect them
  (Inspect `@task` files, not `test_*`), keeping the eval tier on the Inspect
  runner while sharing the `tests/` root.

### Final floor

- **Bootstrap floor for `configure`**: **≥ 3 tasks**, gated at **pass^k ≥ 0.8
  over k = 3 trials** (Inspect's `pass_k`).
- **Rationale**: the purpose here is to prove the harness end-to-end on
  `configure`, not to stand up a mature suite, so the provisional ≥ 3-task floor
  is **kept as an explicit bootstrap smoke-test**. The 2025–2026 guidance
  ("Demystifying evals for AI agents") recommends starting with **20–50 tasks
  drawn from real failures** and notes small samples suffice early because early
  changes have large effect sizes — so 20–50 is treated here as the **external
  benchmark the bootstrap floor is measured against, not a competing
  requirement**. The bar is expressed as **pass^k** rather than mean pass-rate
  because pass^k (all-k-succeed) is the correct statistic for a hard regression
  gate; k = 3 is the minimum recommended trial count.
- **Ramp commitment**: grow the `configure` suite toward **20–50
  real-failure-derived tasks** as the skill matures and real failures
  accumulate, retaining the pass^k gate (raising k toward 5–10 as token budget
  allows). The trigger to revisit is the first material `configure` behaviour
  change or the first real-world failure not covered by the bootstrap tasks.

## Residual Risks & Open Questions

- **No live prototype run.** The harness was selected on verified mechanism, not
  a run against `configure` (which did not yet exist when this spike ran — it is
  built and evaluated in the downstream configuration/eval-application work).
  *Risk*: an Inspect-driving-Claude-Code A/B integration detail bites during that
  work. *Trigger to revisit*: the first spike/throwaway run there; if the
  external-agent A/B proves awkward, promptfoo (turnkey skill A/B) is the
  pre-vetted fallback.
- **Authoring cost of hand-built A/B.** Inspect leaves the with-skill-vs-baseline
  setup and skill-invocation detection to the author (promptfoo provides these).
  Accepted as a one-time cost; mitigated by a shared eval-harness helper if a
  second skill is later evaluated.
- **promptfoo longevity is a moving target.** If OpenAI's stewardship later
  deprioritises non-OpenAI providers, the runner-up weakens further — reinforcing
  Inspect, but worth a re-check if the fallback is ever invoked.
- **Inspect has no built-in score-threshold gate** (only `--fail-on-error` for
  operational errors); the pass^k threshold check is wired by hand in the invoke
  task. Low risk (≈3 lines), noted so the eval-application work doesn't expect a
  turnkey flag.

## Open Questions

- None — the eval framework is recorded as its own ADR, and the recommendation
  home is this work item.

## Dependencies

- In luminosity (where this spike ran) it blocked the "apply the eval framework
  to the `configure` skill" work and the eval-framework ADR; those downstream
  work items are not ported into Accelerator.
- Blocked by: none.

## Assumptions

- The recommendation is not only decided but applied: the first eval target is
  the `configure` skill.

## Technical Notes

- Accepted 2025–2026 methodology (Anthropic, "Demystifying evals for AI agents",
  Jan 2026): tasks + multiple trials per task, track variance not just pass/fail
  (pass@k vs pass^k), combine code/model/human graders, grade outcomes over
  trajectory, baseline comparison to attribute improvement, start with 20–50
  real-failure tasks.
- skill-creator timing/token capture is one-shot from the task notification —
  must be captured at completion.

## Drafting Notes

- Kind set to `spike`: the work is framed as resolving an unknown with a written
  recommendation.
- Decision bias set to genuine, open-ended evaluation: the three named frameworks
  are examples/reference shapes, and the spike actively surveys the wider
  landscape rather than picking from a fixed shortlist.
- Eval framework recorded as its own ADR.
- Recommendation home kept consistent with the architecture spike — recorded in
  this work item.
- Enriched with web research on skill-creator/promptfoo/DeepEval current as of
  mid-2026.

## References

- **Ported from luminosity** — original spike:
  https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0003-skill-evaluation-framework-selection.md
- Research: Anthropic `skill-creator` Eval/Benchmark workflow
  (https://github.com/anthropics/skills/blob/main/skills/skill-creator/SKILL.md);
  "Demystifying evals for AI agents"
  (https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents);
  promptfoo (github.com/promptfoo/promptfoo); DeepEval
  (github.com/confident-ai/deepeval).
</content>
