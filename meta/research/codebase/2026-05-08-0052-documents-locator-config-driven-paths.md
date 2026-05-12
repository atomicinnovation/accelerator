---
date: 2026-05-08T21:53:59+01:00
researcher: Toby Clemson
git_commit: bfd6098c7c35911633de3ecd790632bcf5ce0c96
branch: HEAD
repository: accelerator
topic: "Implementation research for 0052: Make documents-locator Agent Paths Config-Driven via Preloaded Skill"
tags: [research, codebase, config, agents, documents-locator, paths, harness]
status: complete
last_updated: 2026-05-08
last_updated_by: Toby Clemson
---

# Research: Implementation of 0052 — documents-locator Config-Driven Paths

**Date**: 2026-05-08T21:53:59+01:00
**Researcher**: Toby Clemson
**Git Commit**: bfd6098c7c35911633de3ecd790632bcf5ce0c96
**Branch**: HEAD
**Repository**: accelerator

## Research Question

What are the precise implementation details for each requirement of work item
0052 (`meta/work/0052-make-documents-locator-paths-config-driven.md`)? Covers:
`config-read-all-paths.sh`, `skills/config/paths/SKILL.md`,
`agents/documents-locator.md` updates, the `global` path key (Req 8), and the
harness extension for `skills:` frontmatter in agent definitions (Req 9).

## Summary

All nine requirements in 0052 can be implemented against the current codebase.
The infrastructure for path resolution (`config-defaults.sh`, `config-read-path.sh`,
`config-read-value.sh`, and the bang-preprocessing pipeline) is already in
place and well-understood. The only genuinely novel piece is Requirement 9 —
extending the harness to support `skills:` frontmatter in agent definition
files. No SKILL.md or agent file in the repo currently uses a `skills:`
frontmatter key; the mechanism needs to be built, with the `additionalContext`
injection via `hooks/config-detect.sh` being the most natural injection point.

The agent survey is conclusive: only `agents/documents-locator.md` contains
hardcoded `meta/` paths — no other agent file is in scope.

## Detailed Findings

### 1. `scripts/config-defaults.sh` — Current State

**File**: `scripts/config-defaults.sh` (70 lines)

The file defines two positionally-aligned parallel arrays, `PATH_KEYS` and
`PATH_DEFAULTS`, sourced transitively via `config-common.sh` and directly by
`config-read-path.sh` (to avoid VCS detection overhead). Currently 15 entries:

| Index | PATH_KEYS                  | PATH_DEFAULTS              |
|-------|----------------------------|----------------------------|
|  0    | `paths.plans`              | `meta/plans`               |
|  1    | `paths.research`           | `meta/research`            |
|  2    | `paths.decisions`          | `meta/decisions`           |
|  3    | `paths.prs`                | `meta/prs`                 |
|  4    | `paths.validations`        | `meta/validations`         |
|  5    | `paths.review_plans`       | `meta/reviews/plans`       |
|  6    | `paths.review_prs`         | `meta/reviews/prs`         |
|  7    | `paths.review_work`        | `meta/reviews/work`        |
|  8    | `paths.templates`          | `.accelerator/templates`   |
|  9    | `paths.work`               | `meta/work`                |
| 10    | `paths.notes`              | `meta/notes`               |
| 11    | `paths.tmp`                | `.accelerator/tmp`         |
| 12    | `paths.integrations`       | `.accelerator/state/integrations` |
| 13    | `paths.design_inventories` | `meta/design-inventories`  |
| 14    | `paths.design_gaps`        | `meta/design-gaps`         |

**`global` is absent** — Requirement 8 adds it at the end:
- Append `"paths.global"` to `PATH_KEYS` (after line 42, before the `)`)
- Append `"meta/global"` to `PATH_DEFAULTS` (after line 60, before the `)`)

No changes to `config-read-path.sh` itself — its lookup loop is fully generic
and will resolve `global` once the arrays contain it.

### 2. `scripts/config-read-path.sh` — Resolution Chain

The script accepts a bare key (`$1`, e.g. `plans`) and an optional explicit
default (`$2`). It sources `config-defaults.sh` directly (line 19), scans
`PATH_KEYS` for `"paths.${key}"` to find the default, then
`exec`s into `config-read-value.sh "paths.${key}" "${default}"`.

`config-read-value.sh` sources `config-common.sh` (which triggers VCS
detection), splits the key on `.` to get `SECTION=paths` and `SUBKEY=<key>`,
reads `.accelerator/config.md` then `.accelerator/config.local.md` (last
writer wins), and prints the resolved value or the default if absent.

**Performance note for `config-read-all-paths.sh`**: each `config-read-path.sh`
invocation forks a subprocess that triggers VCS detection in
`config-common.sh`. For 11 keys that is 11 VCS detections. The recommended
implementation instead sources `config-common.sh` once and calls
`config-read-value.sh` (or its internal lookup) in a loop — paying VCS
detection once.

### 3. `agents/documents-locator.md` — Current Hardcoded Paths

**File**: `agents/documents-locator.md` (141 lines)

Current frontmatter (lines 1–5):
```yaml
---
name: documents-locator
description: Discovers relevant documents in meta/ directory …
tools: Grep, Glob, LS
---
```

There is **no `skills:` key** in the frontmatter.

Hardcoded `meta/` path occurrences by location:

| Lines     | Context                        | Paths referenced |
|-----------|-------------------------------|-----------------|
| 7, 13     | Body prose preamble            | `meta/` (general) |
| 15–21     | Core responsibilities list     | `meta/research/codebase/`, `meta/plans/`, `meta/decisions/`, `meta/reviews/`, `meta/validations/`, `meta/global/` |
| 49–59     | ASCII directory tree diagram   | All 8 subdirectories as literal strings |
| 75–100    | Output-format example block    | `meta/work/`, `meta/research/codebase/`, `meta/plans/`, `meta/notes/`, `meta/decisions/`, `meta/reviews/plans/`, `meta/reviews/prs/`, `meta/validations/`, `meta/prs/` |
| 140       | Closing reminder line          | `meta/` (general) |

The directory tree (lines 49–59) and example block (lines 75–100) are the
most structurally locked parts — they need to be either updated dynamically or
replaced with prose instructions that defer to the preloaded path block.

### 4. Agent Survey — Hardcoded `meta/` Paths

All nine agent files were grepped for `meta/` path references:

| File | Hardcoded `meta/` paths? |
|------|--------------------------|
| `agents/documents-locator.md` | **Yes** — see §3 above |
| `agents/codebase-analyser.md` | No |
| `agents/codebase-locator.md` | No |
| `agents/codebase-pattern-finder.md` | No |
| `agents/documents-analyser.md` | No |
| `agents/reviewer.md` | No |
| `agents/web-search-researcher.md` | No |
| `agents/browser-analyser.md` | No |
| `agents/browser-locator.md` | No |

**Conclusion**: Only `documents-locator.md` is in scope. The work item
assumption is confirmed.

### 5. `skills/config/` Directory — Current Structure

```
skills/config/
├── configure/
│   └── SKILL.md
├── init/
│   ├── SKILL.md                    ← bang commands on lines 20–31
│   └── scripts/
│       ├── init.sh
│       └── test-init.sh
└── migrate/
    ├── SKILL.md
    ├── migrations/
    │   ├── 0001-rename-tickets-to-work.sh
    │   ├── 0002-rename-work-items-with-project-prefix.sh
    │   └── 0003-relocate-accelerator-state.sh
    └── scripts/
        ├── run-migrations.sh
        ├── test-migrate.sh
        └── test-fixtures/
```

There is no `skills/config/paths/` directory yet — this is the new directory
for Requirement 2.

### 6. Bang Command Preprocessing — Syntax and Conventions

The `!` bang preprocessing is a **native Claude Code feature**: any line in a
SKILL.md body of the form `` !`<shell-command>` `` is executed before the
skill content reaches the model, with stdout substituting the line.

**Example from `skills/config/init/SKILL.md` lines 20–31**:
```
**Plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans`
**Research directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh research`
...
**Design gaps directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh design_gaps`
```

At preprocessing time each `` !`...` `` is replaced with the resolved value
(e.g. `meta/plans`), so the model sees: `**Plans directory**: meta/plans`.

**Structural test conventions** (verified by `scripts/test-config.sh`):
- `config-read-skill-instructions.sh <name>` must be the **last** bang line
- `config-read-context.sh` and `config-read-skill-context.sh` must appear at
  the top, in that order
- The `name:` argument to each preprocessor call must match the SKILL.md's
  `name:` frontmatter
- Init and configure skills are exempt from the context/instructions
  preprocessor lines

The new `skills/config/paths/SKILL.md` is a config skill (analogous to
`init`) — it may also be exempt from the context/skill-instructions
preprocessor requirement. This needs verification against the test suite when
writing the new skill.

### 7. Init Process — Changes for Requirement 8 (`global` key)

Three files need edits, all following the exact same pattern as existing keys:

**`scripts/config-defaults.sh`** (Requirement 8 + used by all consumers):
```bash
# Append to PATH_KEYS after line 42:
  "paths.global"

# Append to PATH_DEFAULTS after line 60:
  "meta/global"
```

**`skills/config/init/scripts/init.sh`** (lines 17–29):
```bash
# Add to DIR_KEYS after `design_gaps` (line 22):
  global

# Add to DIR_DEFAULTS after `meta/design-gaps` (line 28):
  meta/global
```

**`skills/config/init/SKILL.md`** (lines 17–50):
```
# Add after line 31 (after design_gaps bang line):
**Global directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh global`

# Update DIR_COUNT comment at line 40:
<!-- DIR_COUNT:13 -->  (was 12)

# Update prose at line 46:
The script creates 13 `meta/` directories  (was 12)
```

The summary block at lines 51–72 should also gain a `global` directory
confirmation entry — follow the pattern for the 12 existing entries.

### 8. `config-read-all-paths.sh` — Design for Requirement 1

The script should:
1. Source `config-common.sh` once (pays VCS detection once, not 11×)
2. Source `config-defaults.sh` for `PATH_KEYS`/`PATH_DEFAULTS`
3. Iterate the 11 document-discovery keys: `plans research decisions prs
   validations review_plans review_prs review_work work notes global`
4. For each key, call `config-read-value.sh "paths.${key}" "${default}"`
5. Emit a labelled Markdown block:

```markdown
## Configured Paths

- plans: meta/plans
- research: meta/research/codebase
- decisions: meta/decisions
- prs: meta/prs
- validations: meta/validations
- review_plans: meta/reviews/plans
- review_prs: meta/reviews/prs
- review_work: meta/reviews/work
- work: meta/work
- notes: meta/notes
- global: meta/global
```

Excluded keys: `tmp`, `templates`, `integrations`, `design_inventories`,
`design_gaps` — these are non-document keys or not yet in the document-discovery
vocabulary (design directories were added late; their inclusion can follow
once the agent body is updated).

The `global` key appears last so its addition lands cleanly after the existing
`notes` entry — no re-ordering needed.

**Alternative design** (11 subprocess calls): call `config-read-path.sh` in a
loop. Simpler but incurs 11 VCS detections. Acceptable for a first iteration;
the single-sourcing approach is a performance optimisation only.

### 9. Harness Extension for `skills:` in Agent Definitions (Requirement 9)

This is the most novel piece. Key facts established by research:

**Current state**:
- `!` bang preprocessing is a Claude Code native feature for SKILL.md files
- **No SKILL.md file in this repo uses a `skills:` frontmatter key** (grep: zero matches)
- **No agent `.md` file uses a `skills:` frontmatter key** (grep: zero matches)
- Agent `.md` files do not support inline `!` bang commands

**What exists that could support implementation**:
- `config-common.sh` exposes `config_extract_frontmatter()` for parsing
  frontmatter from any Markdown file
- `hooks/config-detect.sh` uses `hookSpecificOutput.additionalContext` to
  inject context into every session start — this is the established injection
  mechanism
- `SessionStart` hooks run once per session (top-level or subagent)

**Open question for Requirement 9**: Does the `SessionStart` hook event data
identify _which_ agent definition is being loaded for a subagent session? If
it does, a new hook script can:
1. Read the agent name from event data
2. Find `agents/<name>.md`
3. Parse its `skills:` frontmatter list
4. For each named skill, locate `skills/<name>/SKILL.md` or
   `skills/<category>/<name>/SKILL.md`
5. Process the skill's body (execute bang lines in order)
6. Inject the concatenated output via `additionalContext`

If Claude Code does not pass agent metadata to hooks, an alternative is to
resolve this statically: the `config-summary.sh` script already runs at session
start and could be extended to enumerate all agent definitions, detect their
`skills:` frontmatter, and inject skill content for all agents unconditionally
(wasteful but functional as a fallback).

**Practical first step for Requirement 9**: write a spike against a real
subagent invocation to confirm what event data the `SessionStart` hook receives
when a subagent is spawned via the `Agent` tool. The hook's stdin will contain
the JSON event payload — inspect it to determine whether agent name or
definition path is present.

## Code References

- `scripts/config-defaults.sh:27–61` — `PATH_KEYS` and `PATH_DEFAULTS` arrays
- `scripts/config-defaults.sh:63–70` — `TEMPLATE_KEYS` array (not relevant to 0052)
- `scripts/config-read-path.sh:19` — sources `config-defaults.sh` directly
- `scripts/config-read-path.sh:27–40` — default lookup loop
- `scripts/config-read-path.sh:42` — `exec` delegation to `config-read-value.sh`
- `scripts/config-read-value.sh:33–39` — key splitting (section + subkey)
- `scripts/config-read-value.sh:117–124` — last-writer-wins file iteration
- `agents/documents-locator.md:1–5` — frontmatter (no `skills:` key)
- `agents/documents-locator.md:15–21` — hardcoded path instructions
- `agents/documents-locator.md:49–59` — ASCII directory tree
- `agents/documents-locator.md:75–100` — example output block
- `skills/config/init/SKILL.md:20–31` — 12 bang-command path resolution lines
- `skills/config/init/SKILL.md:40` — `<!-- DIR_COUNT:12 -->` to update to 13
- `skills/config/init/scripts/init.sh:17–29` — `DIR_KEYS`/`DIR_DEFAULTS` arrays
- `hooks/config-detect.sh:17–23` — `additionalContext` injection mechanism
- `scripts/config-common.sh:73–85` — `config_extract_frontmatter()` function
- `scripts/test-config.sh:3700–3735` — bang-preprocessing structural tests

## Architecture Insights

- The `config-defaults.sh` → `config-read-path.sh` → `config-read-value.sh`
  chain was intentionally designed so adding a new path key is a one-line edit
  to `config-defaults.sh` alone (no logic changes needed in the lookup scripts).
  This makes Requirement 8 (`global`) a clean mechanical addition.
- The `init.sh` / `init SKILL.md` duplication of key lists is tracked in
  `config-defaults.sh:14–17` as a known divergence earmarked for unification.
  Work item 0052 adds `global` to both lists, preserving the current pattern.
- The bang-preprocessing pipeline runs inside Claude Code natively — there is
  no harness-side script that drives it. The harness scripts are passive: they
  are called by bang lines but have no knowledge of which skill is being loaded.
- The `additionalContext` JSON field emitted by `hooks/config-detect.sh` is
  the only existing mechanism by which the harness can inject content into a
  session context. Requirement 9 must either use this field or identify a new
  injection mechanism.
- The distinction between `skills:` in SKILL.md files (Claude Code native) and
  `skills:` in agent definition files (harness extension) means the two
  behaviours may never be identical — agent file preprocessing must be driven
  by the harness because agent files are not processed by Claude Code's skill
  loader.

## Historical Context

- `meta/notes/2026-04-26-agents-hardcode-default-directory-locations.md` —
  first documented the tech debt; used an older config naming convention
  (`config.user.yaml`/`config.team.yaml`) now superseded by `.accelerator/config.md`
- `meta/research/codebase/2026-02-22-skills-agents-commands-refactoring.md` — lines
  56–57 define `skills:` frontmatter for agents; line 432 marks exact
  injection behaviour as an open question
- `meta/work/0030-centralise-path-defaults.md` — the related work item that
  proposed `config-defaults.sh`; now landed (commit `da6c42901`). The
  `config-read-all-paths.sh` script should source the landed `config-defaults.sh`
  rather than maintaining its own key list.

## Open Questions

1. **Requirement 9 injection trigger**: What JSON data does Claude Code send to
   `SessionStart` hooks when a subagent is started via the `Agent` tool? Is the
   agent definition name or path present? If not, what alternative injection
   mechanism should Requirement 9 use?

2. **`skills/config/paths/SKILL.md` preprocessor conventions**: Must the new
   skill include `config-read-context.sh` and `config-read-skill-instructions.sh`
   bang lines (like non-config skills), or is it exempt (like `init` and
   `configure`)? Verify against `scripts/test-config.sh` before writing the file.

3. **Design keys in document-discovery scope**: The 11-key subset in the work
   item excludes `design_inventories` and `design_gaps`. Should these be included
   if/when `documents-locator.md` is expected to search those directories? Not
   in scope for 0052, but the question will arise when design review skills land.

4. **`documents-locator.md` example block (lines 75–100)**: The example uses
   all literal paths. After the update, the example block should either be
   removed, replaced with placeholder paths, or dynamically generated from the
   preloaded path block. The simplest approach is to remove the rigid example
   and add a note that paths come from the preloaded context.
