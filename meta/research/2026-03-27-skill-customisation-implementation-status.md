---
date: "2026-03-27T22:15:25+0000"
researcher: Toby Clemson
git_commit: 28643b69b3e8785dac181a9b36b779b4efb0cc76
branch: main
repository: accelerator
topic: "Skill customisation implementation status and remaining improvements"
tags: [ research, configuration, customisation, overrides, userspace, implementation-review ]
status: complete
last_updated: "2026-03-27"
last_updated_by: Toby Clemson
---

# Research: Skill Customisation Implementation Status and Remaining Improvements

**Date**: 2026-03-27T22:15:25+0000
**Researcher**: Toby Clemson
**Git Commit**: 28643b69b3e8785dac181a9b36b779b4efb0cc76
**Branch**: main
**Repository**: accelerator

## Research Question

Have the findings and recommendations from the skill customisation research at
`meta/research/2026-03-22-skill-customisation-and-override-patterns.md` been
fully implemented? What remains, and what further improvements could be made to
the plugin's configurability?

## Summary

The original research identified **Strategy D (Hybrid)** as the recommended
approach combining structured settings, context injection, template overrides,
custom lenses, and a setup skill. This strategy has been **substantially
implemented** across four sequential plans, delivering 30 discrete customisation
points (7 agent keys, 9 review keys, 10 path keys, 4 template keys) plus
free-form project context injection.

The core infrastructure is solid: a two-tier config file system
(`.claude/accelerator.md` for team config, `.claude/accelerator.local.md` for
personal overrides), shell-based YAML parsing, SessionStart hook integration, a
`/accelerator:configure` skill, and per-skill preprocessor integration.

However, several **gaps and inconsistencies** remain between what the research
envisioned and what was implemented:

- **Agent name overrides are partially non-deterministic**: Only 2 of 10 skills
  that spawn agents resolve names at preprocessor time; the rest rely on LLM
  interpretation of an override table.
- **Hardcoded path references persist**: 3 skills have functionally hardcoded
  `meta/` paths that break if users override `paths.*` config, and ~14 example
  references across 7 skills would be misleading.
- **Numeric defaults are duplicated in prose**: Review skills hardcode default
  values like "10" and "6 to 8" in instructional text that can drift from actual
  configured values.
- **Several lower-priority customisation points remain hardcoded**: emoji
  severity prefixes, response style/tone, output format schemas, and file naming
  conventions.

## Detailed Findings

### 1. What Was Recommended vs. What Was Implemented

The original research (2026-03-22) recommended five components under Strategy D.
Here is the implementation status of each:

| Component           | Research Recommendation                                         | Status                      | Notes                                                                                                                   |
|---------------------|-----------------------------------------------------------------|-----------------------------|-------------------------------------------------------------------------------------------------------------------------|
| Structured settings | `.claude/accelerator.local.md` with YAML frontmatter            | **Fully implemented**       | Two-tier: team (`.md`) + personal (`.local.md`) with last-writer-wins precedence                                        |
| Context injection   | Markdown body for tech-stack guidance                           | **Fully implemented**       | 13 skills inject context via `config-read-context.sh`                                                                   |
| Template overrides  | `.claude/accelerator/templates/` for full file replacement      | **Implemented differently** | Three-tier resolution via `config-read-template.sh` (config path → templates dir → plugin default), not directory-based |
| Custom lenses       | `.claude/accelerator/lenses/` for domain-specific review lenses | **Fully implemented**       | Auto-discovery with frontmatter validation, collision checks, and integration into lens catalogue                       |
| Setup skill         | `/accelerator:configure` for interactive configuration          | **Fully implemented**       | Supports create, view, dump, and explanatory actions                                                                    |

### 2. Configuration Infrastructure (Plan 1)

**Status**: Fully implemented.

The foundation layer provides:

- **Config file format**: YAML frontmatter + markdown body in two files with
  clear precedence rules
- **Core scripts**: `config-common.sh` (shared utilities),
  `config-read-value.sh`
  (single key reader with dot notation), `config-read-context.sh` (markdown body
  reader), `config-summary.sh` (session summary)
- **SessionStart hook**: `hooks/config-detect.sh` injects config awareness via
  `additionalContext`
- **Configure skill**: `skills/config/configure/SKILL.md` documents all keys and
  supports create/view/dump operations
- **Test infrastructure**: `scripts/test-config.sh` for validation

Key scripts and their locations:

| Script                              | Purpose                                                                |
|-------------------------------------|------------------------------------------------------------------------|
| `scripts/config-common.sh`          | Shared utilities (frontmatter extraction, file finding, array parsing) |
| `scripts/config-read-value.sh`      | Core single-value reader with dot notation                             |
| `scripts/config-read-context.sh`    | Markdown body concatenation                                            |
| `scripts/config-read-agents.sh`     | Bulk agent override table                                              |
| `scripts/config-read-agent-name.sh` | Single agent name resolution                                           |
| `scripts/config-read-path.sh`       | Output path resolution                                                 |
| `scripts/config-read-template.sh`   | Three-tier template resolution                                         |
| `scripts/config-read-review.sh`     | Review settings with validation and lens discovery                     |
| `scripts/config-summary.sh`         | Session context summary                                                |
| `scripts/config-dump.sh`            | Full config dump with source attribution                               |

### 3. Context and Agent Customisation (Plan 2)

**Status**: Implemented with a known reliability gap.

**Context injection**: All 13 user-facing skills include
`config-read-context.sh` via preprocessor. This is the most universally applied
customisation and works reliably.

**Agent name overrides**: Two mechanisms exist:

1. **Override table** (indirect, 10 skills): `config-read-agents.sh` outputs a
   markdown table instructing the LLM to substitute agent names. The actual
   agent names remain as literal strings in skill prose. This relies on Claude
   reading the table and mentally applying the substitution when spawning
   agents.
2. **Inline resolution** (direct, 2 skills): `config-read-agent-name.sh`
   resolves the agent name at preprocessor time, producing the configured name
   directly in the `subagent_type` parameter. Only `review-pr` (line 279) and
   `review-plan` (line 248) use this approach.

**Gap**: The override table approach is non-deterministic. If Claude fails to
apply the mapping, the wrong agent gets spawned. The original research noted
this risk (Strategy A, section 6b) and recommended preprocessor injection
(approach 1) as "more reliable since it produces deterministic output." Only the
reviewer agent benefits from this reliability; all other agents rely on the
table.

**Skills missing agent config entirely**: `implement-plan`, `validate-plan`,
`describe-pr`, `respond-to-pr`, and `commit` do not use `config-read-agents.sh`
at all. Of these, `implement-plan` is the most notable since it can spawn
sub-tasks.

### 4. Review System Customisation (Plan 3)

**Status**: Fully implemented with one prose consistency issue.

All 9 review configuration keys are supported with validation:

| Config Key                           | Default                                                    | Scope     |
|--------------------------------------|------------------------------------------------------------|-----------|
| `review.max_inline_comments`         | `10`                                                       | PR only   |
| `review.dedup_proximity`             | `3`                                                        | PR only   |
| `review.pr_request_changes_severity` | `critical`                                                 | PR only   |
| `review.plan_revise_severity`        | `critical`                                                 | Plan only |
| `review.plan_revise_major_count`     | `3`                                                        | Plan only |
| `review.min_lenses`                  | `4`                                                        | Both      |
| `review.max_lenses`                  | `8`                                                        | Both      |
| `review.core_lenses`                 | `[architecture, code-quality, test-coverage, correctness]` | Both      |
| `review.disabled_lenses`             | `[]`                                                       | Both      |

**Custom lenses**: Fully supported via `.claude/accelerator/lenses/*/SKILL.md`
with auto-discovery, frontmatter validation (`name` required, `auto_detect`
optional), collision checking against 13 built-in lenses, and integration into
the lens catalogue.

**Prose inconsistency**: Both `review-pr/SKILL.md` (line 175) and
`review-plan/SKILL.md` (line 151) state the default lens range is "6 to 8",
while `config-read-review.sh` uses `DEFAULT_MIN_LENSES=4` and
`DEFAULT_MAX_LENSES=8`. Additionally, `review-pr/SKILL.md` hardcodes "10" in
multiple prose locations (lines 347, 397, 470, 498, 614, 635) that would not
update if the user changes `max_inline_comments`.

### 5. Template and Path Customisation (Plan 4)

**Status**: Implemented with residual hardcoded path references.

**Paths**: All 10 path keys are configurable via `config-read-path.sh`:

| Config Key           | Default              |
|----------------------|----------------------|
| `paths.plans`        | `meta/plans`         |
| `paths.research`     | `meta/research`      |
| `paths.decisions`    | `meta/decisions`     |
| `paths.prs`          | `meta/prs`           |
| `paths.validations`  | `meta/validations`   |
| `paths.review_plans` | `meta/reviews/plans` |
| `paths.review_prs`   | `meta/reviews/prs`   |
| `paths.templates`    | `meta/templates`     |
| `paths.tickets`      | `meta/tickets`       |
| `paths.notes`        | `meta/notes`         |

**Templates**: Four templates use three-tier resolution via
`config-read-template.sh` (explicit config path → templates directory → plugin
default): `plan`, `research`, `adr`, `validation`.

**Residual hardcoded paths** (functionally broken if paths are overridden):

| File                                      | Line(s) | Issue                                                                                                      |
|-------------------------------------------|---------|------------------------------------------------------------------------------------------------------------|
| `skills/github/respond-to-pr/SKILL.md`    | 74, 217 | Hardcodes `meta/reviews/prs/` when looking up review artifacts; breaks if `paths.review_prs` is overridden |
| `skills/planning/implement-plan/SKILL.md` | 3, 15   | Hardcodes `meta/plans/` in description and instructions; does not use `config-read-path.sh`                |
| `skills/planning/review-plan/SKILL.md`    | 397     | Hardcodes `mkdir -p meta/reviews/plans` in bash example                                                    |
| `skills/github/review-pr/SKILL.md`        | 413     | Hardcodes `mkdir -p meta/reviews/prs` in bash example                                                      |

**Hardcoded paths in examples** (misleading but not functionally broken): ~14
instances across 7 skill files where example paths use `meta/plans/`,
`meta/tickets/`, etc. in illustrative text.

### 6. Known Issues and Deferred Items

| Item                                              | Rationale for Deferral                                                                                                               |
|---------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------|
| Sentinel value for unsetting team config in local | Deferred; no current mechanism to say "unset this key"                                                                               |
| Per-skill configuration                           | Global-only config; no per-skill overrides                                                                                           |
| Context injection into sub-agents                 | Known Claude Code platform limitation                                                                                                |
| Output format schema overrides                    | Complex; PR and plan review output formats are deeply integrated                                                                     |
| File naming convention overrides                  | Out of scope; date-prefix conventions are hardcoded                                                                                  |
| `describe-pr` template                            | Uses convention-based lookup (`{templates dir}/pr-description.md`) rather than `config-read-template.sh`; no plugin default fallback |

### 7. Coverage Assessment Against Original Research

The original research identified customisation points in 6 categories. Here is
coverage:

#### 7a. Agent names (7 agents, "High value")

**Coverage: 100% of keys, ~20% deterministic resolution.**

All 7 agent names are configurable via the `agents.*` config section. However,
only the `reviewer` agent is resolved deterministically at preprocessor time (in
2 skills). The other 6 agents rely on LLM interpretation of an override table
in 8 skills.

#### 7b. Numeric limits ("Medium value")

**Coverage: 100% of review limits.**

All review numeric limits are configurable via `config-read-review.sh` with
validation. Non-review limits (e.g., `gh pr list --limit 10`, GraphQL
`comments(first: 100)`) are not configurable, though these are operational
parameters unlikely to need user override.

#### 7c. Review lens catalogue ("High value")

**Coverage: 100%.**

The lens catalogue is fully configurable: core lenses, disabled lenses,
min/max lens count, and custom lens discovery from
`.claude/accelerator/lenses/`.

#### 7d. Document templates and file paths ("High value")

**Coverage: ~90% of paths, ~80% of templates.**

10 path keys and 4 templates are configurable. Gaps: `describe-pr` uses a
different template mechanism without plugin default fallback; `respond-to-pr`
and `implement-plan` have hardcoded paths; default templates contain hardcoded
cross-references to other paths.

#### 7e. Verdict/decision rules ("Medium value")

**Coverage: 100%.**

PR and plan verdict thresholds are fully configurable via
`pr_request_changes_severity`, `plan_revise_severity`, and
`plan_revise_major_count`.

#### 7f. Response style and conventions ("Low value")

**Coverage: 0%.**

Emoji severity prefixes, response tone, commit attribution rules, and review
comment structure remain hardcoded. The research rated these as low value.

### 8. Opportunities for Further Improvement

#### 8a. High-impact improvements

1. **Deterministic agent name resolution in all skills**: Replace the override
   table pattern with inline `config-read-agent-name.sh` calls at each agent
   spawn point. This is the single highest-value improvement for configuration
   reliability.

2. **Fix hardcoded paths in `respond-to-pr` and `implement-plan`**: These skills
   will break if users override `paths.review_prs` or `paths.plans`. Adding
   `config-read-path.sh` calls would fix this.

#### 8b. Medium-impact improvements

4. **Eliminate hardcoded numeric defaults in prose**: Review skills mention
   "10", "6 to 8", and "3" as defaults in instructional text. These should
   reference the configured values (either via preprocessor injection or by
   removing the specific numbers from prose and relying on the config block).

5. **Align `describe-pr` template handling**: Use `config-read-template.sh`
   with a plugin default so the skill works without user-created templates.

6. **Dynamic cross-references in default templates**: The default plan template
   references `meta/research/` etc. Users who override paths but use the
   default templates get inconsistent references. Either make templates aware
   of configured paths (via preprocessor in templates) or document this
   limitation prominently.

7. **Add `config-read-agents.sh` to missing skills**: `implement-plan`,
   `validate-plan`, `describe-pr`, and `respond-to-pr` don't include agent
   config despite some spawning sub-tasks.

#### 8c. Lower-impact improvements

8. **Config unset sentinel**: Allow local config to explicitly unset a team
   config key (e.g., `agents.reviewer: ~` or `agents.reviewer: ""`).

9. **Output format schema customisation**: Allow users to override the PR
   review and plan review JSON output schemas. Complex but would enable custom
   review workflows.

10. **Response style configuration**: Allow teams to customise tone, emoji
    usage, and attribution rules. Low demand but straightforward to implement
    via context injection.

11. **File naming convention overrides**: Allow customisation of the
    date-prefix naming pattern for plans, research, etc.

12. **Per-skill configuration**: Allow config keys scoped to specific skills
    rather than global-only.

## Code References

- `scripts/config-common.sh` — Shared config utilities (frontmatter extraction,
  file finding)
- `scripts/config-read-value.sh` — Core single-value reader with dot notation
- `scripts/config-read-context.sh` — Markdown body concatenation for context
  injection
- `scripts/config-read-agents.sh` — Bulk agent override table generator
- `scripts/config-read-agent-name.sh` — Single agent name resolver
- `scripts/config-read-path.sh` — Output path resolver
- `scripts/config-read-template.sh` — Three-tier template resolver
- `scripts/config-read-review.sh` — Review settings reader with validation and
  custom lens discovery
- `scripts/config-summary.sh` — Session config summary for hook injection
- `scripts/config-dump.sh` — Full config dump with source attribution
- `hooks/config-detect.sh` — SessionStart hook for config awareness
- `hooks/hooks.json` — Hook registration
- `skills/config/configure/SKILL.md` — User-facing configuration skill
- `skills/github/respond-to-pr/SKILL.md:74,217` — Hardcoded `meta/reviews/prs/`
  paths
- `skills/planning/implement-plan/SKILL.md:3,15` — Hardcoded `meta/plans/` paths
- `skills/github/review-pr/SKILL.md:175,279,413` — Lens default prose, agent
  name resolution, hardcoded mkdir
- `skills/planning/review-plan/SKILL.md:151,248,397` — Lens default prose, agent
  name resolution, hardcoded mkdir

## Architecture Insights

### The preprocessor-vs-instruction trade-off

The implementation reveals a fundamental architectural tension in Claude Code
plugin development. The `!` preprocessor provides deterministic configuration
(the configured value appears literally in the skill text), but it runs at skill
load time and cannot be applied retroactively within prose. Natural language
instructions ("check the override table and substitute agent names") are
flexible but non-deterministic.

The current codebase uses both approaches and the boundary between them is
principled: structured values that must be exact (agent names for the `reviewer`
agent, file paths, numeric limits, template content) use the preprocessor; agent
names that appear in instructional prose rely on the override table instruction.
The gap is that the instructional approach applies to 6 of 7 configurable
agents.

### Two-tier config is the right abstraction

The team/personal config split maps cleanly to real organisational needs: teams
share build system context, path conventions, and lens preferences; individuals
override for personal agent experiments or workflow preferences. The
`.local.md` auto-gitignore convention prevents accidental personal config
commits.

### Template resolution order is well-designed

The three-tier template resolution (explicit path → templates directory → plugin
default) follows the ecosystem consensus identified in the original research.
It provides a good balance of flexibility (per-template overrides), convention
(templates directory), and safety (plugin defaults as fallback).

## Historical Context

- `meta/research/2026-03-22-skill-customisation-and-override-patterns.md` —
  Original research identifying 40+ customisation points and recommending
  Strategy D (Hybrid)
- `meta/research/2026-03-15-context-management-approaches.md` — Earlier
  research on context management informing the context injection design
- `meta/plans/2026-03-23-config-infrastructure.md` — Plan 1: config file
  format, parsing scripts, SessionStart hook, configure skill
- `meta/plans/2026-03-23-context-and-agent-customisation.md` — Plan 2: context
  injection into 13 skills, agent override system
- `meta/plans/2026-03-23-review-system-customisation.md` — Plan 3: review
  settings, custom lenses, config dump
- `meta/plans/2026-03-23-template-and-path-customisation.md` — Plan 4:
  template resolution, path overrides, skill updates
- `meta/plans/2026-03-14-plugin-extraction.md` — Precursor plan establishing
  the plugin architecture

## Related Research

- `meta/research/2026-03-22-skill-customisation-and-override-patterns.md` —
  The research this document evaluates implementation of
- `meta/research/2026-03-14-plugin-extraction.md` — Plugin extraction research
  establishing the plugin system

## Open Questions

1. **Should all agent names use deterministic resolution?** The override table
   pattern is a conscious trade-off (simpler skill text vs. reliability). Is the
   non-determinism a real problem in practice, or does the LLM reliably apply
   the mapping?

2. **Are hardcoded example paths confusing enough to fix?** Example paths in
   skill prose (e.g., `meta/plans/2025-01-08-ENG-1478-feature.md`) are
   illustrative. Replacing them with generic placeholders would be more correct
   but less concrete. Is this worth the churn?

3. **Should `describe-pr` adopt `config-read-template.sh`?** Its current
   approach (convention-based lookup, no plugin default) means it fails
   gracefully when no template exists by asking the user to create one. Adding
   a plugin default would change this behaviour.

4. **Is config validation sufficient?** The current system validates review
   keys but has no schema for the full config file. Invalid keys are silently
   ignored. Should there be a `config validate` action that warns about
   unrecognised keys?

5. **How should default templates handle path cross-references?** The default
   plan template references `meta/research/` etc. If paths are overridden,
   these references become stale. Should templates use placeholders resolved at
   load time, or is this an acceptable documentation gap?
