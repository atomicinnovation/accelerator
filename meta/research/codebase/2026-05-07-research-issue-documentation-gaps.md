---
date: "2026-05-07T18:10:33+02:00"
researcher: jonassvalin
git_commit: 986954db59e2eb6d9c9a7426c4f52852d00f1afb
branch: research-issue-skill
repository: accelerator
topic: "Documentation and registration gaps for the new research-issue skill"
tags: [research, skills, research-issue, documentation, conventions]
status: complete
last_updated: "2026-05-07"
last_updated_by: jonassvalin
---

# Research: Documentation and Registration Gaps for research-issue Skill

**Date**: 2026-05-07T18:10:33+02:00
**Researcher**: jonassvalin
**Git Commit**: 986954db59e2eb6d9c9a7426c4f52852d00f1afb
**Branch**: research-issue-skill
**Repository**: accelerator

## Research Question

The new `research-issue` skill has been added at
`skills/research/research-issue/SKILL.md`. What documentation, READMEs, and
registrations are missing compared to existing skill conventions?

## Summary

The skill implementation and test-config registration are already in place.
Three files still need updating: **README.md**, **CHANGELOG.md**, and the
**`rca.md` template** (which exists in the source repo but is not yet mentioned
in the CHANGELOG). No `plugin.json` change is needed because the
`./skills/research/` directory is already registered.

## Detailed Findings

### Already Done

| File | Status |
|------|--------|
| `skills/research/research-issue/SKILL.md` | Created |
| `scripts/test-config.sh` | Already lists `research/research-issue` in skill arrays (lines 1101, 1137, 3543) and has dedicated test cases (lines 3077–3091) |
| `.claude-plugin/plugin.json` | No change needed — `./skills/research/` directory already registered |
| `templates/rca.md` | Created |

### Missing: README.md

The README documents skills in two places:

1. **The development loop diagram** (line 40) — currently only shows
   `research-codebase`. Should be updated to mention `research-issue` as an
   alternative entry point or in a separate "Issue Investigation" section.

2. **The meta directory table** (line 79) — the `research/` row currently lists
   only `research-codebase` as the writing skill. Should add `research-issue`.

Pattern from existing skills — the README uses this table format:

```markdown
| Directory   | Purpose                                 | Written by              |
|-------------|-----------------------------------------|-------------------------|
| `research/` | Research findings with YAML frontmatter | `research-codebase`     |
```

Should become:

```markdown
| `research/` | Research findings with YAML frontmatter | `research-codebase`, `research-issue` |
```

### Missing: CHANGELOG.md

No mention of `research-issue` in CHANGELOG. Convention for a new skill in an
existing category (from recent entries):

```markdown
### Added

- **`/accelerator:research-issue` skill**: Hypothesis-driven root cause analysis
  for production issues and bugs. Accepts stacktraces, logs, error messages, or
  vague behavioral descriptions and produces an RCA document in `meta/research/codebase/`.
  Uses parallel sub-agents to investigate multiple hypotheses concurrently.
  New `rca` template added for structured output.
```

### Missing: README skill table or section

The README has no explicit skill table in the research section (unlike work-item
skills). The research workflow is described narratively. At minimum, the
narrative at line 45 should be extended or a sibling entry added for
`research-issue`.

### Template: rca.md

The `templates/rca.md` file exists but is not documented in the CHANGELOG. It
should be mentioned as part of the `research-issue` CHANGELOG entry (as shown
above).

## Action Items

| # | File | Action |
|---|------|--------|
| 1 | `README.md` | Add `research-issue` to the meta directory table and narrative |
| 2 | `CHANGELOG.md` | Add entry under `## [Unreleased] > ### Added` |
| 3 | (Optional) `README.md` | Consider a dedicated "Issue Investigation" subsection if the workflow is distinct enough from general research |

## Architecture Insights

- The plugin uses **convention-based discovery**: `plugin.json` lists category
  directories, and any `SKILL.md` within is auto-discovered. No explicit
  per-skill registration is needed.
- `scripts/test-config.sh` serves as a de facto skill manifest — it hardcodes
  arrays of all expected skills for validation. This was already updated.
- Skills in the same category share infrastructure (e.g., `research-issue` uses
  `research-codebase/scripts/research-metadata.sh` via its `allowed-tools`).

## Related Research

- `meta/plans/2026-05-05-research-issue-skill.md` — Implementation plan for
  this skill
- `meta/research/codebase/2026-05-05-debug-issue-skill-design.md` — Design exploration
  that led to this skill

## Open Questions

- Should `research-issue` appear in the main development-loop diagram, or is it
  better positioned as a standalone "investigation" workflow separate from the
  research→plan→implement loop?
