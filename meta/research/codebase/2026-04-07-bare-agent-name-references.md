---
date: 2026-04-07T11:41:20+01:00
researcher: Toby Clemson
git_commit: 508ca24b973d8c742e52e829c557f0c62f81076d
branch: (detached HEAD / working copy)
repository: accelerator
topic: "Bare agent name references that need plugin prefix"
tags: [ research, agents, configuration, skills, naming ]
status: complete
last_updated: 2026-04-07
last_updated_by: Toby Clemson
---

# Research: Bare Agent Name References Requiring Plugin Prefix

**Date**: 2026-04-07T11:41:20+01:00
**Researcher**: Toby Clemson
**Git Commit**: 508ca24b973d
**Repository**: accelerator

## Research Question

Where in the codebase are agent names referenced as bare names (e.g.,
`reviewer`) rather than fully-prefixed names (e.g., `accelerator:reviewer`),
and what changes are needed to add the prefix everywhere?

## Summary

Agent names appear in **four categories** of locations that all need updating:

1. **Agent definition frontmatter** (`agents/*.md`) — the `name:` field
2. **Scripts** — `config-read-agents.sh`, `config-read-agent-name.sh`,
   `config-dump.sh`, and their test file
3. **Skill fallback defaults** — the "use these defaults" line in 10 SKILL.md
   files
4. **Skill body references** — `{agent name}` template variables and
   `subagent_type` parameter values in SKILL.md files

There are **7 agent definitions**, **10 skills** that reference agents, and
**4 scripts** (plus 1 test file) that produce or validate agent names.

## Detailed Findings

### 1. Agent Definition Files (`agents/*.md`) — NO CHANGES NEEDED

Each agent's `name:` frontmatter field uses the bare name (`reviewer`,
`codebase-locator`, etc.). This is correct — the plugin framework automatically
prefixes agent names based on the plugin that provides them. The bare name in
the definition is the canonical name; `accelerator:` is added by the runtime.

### 2. Scripts

#### `scripts/config-read-agents.sh`

This is the primary script that generates the "Agent Names" markdown block
injected into skills at runtime. It contains:

- **Lines 33-39**: The `AGENT_KEYS` array listing all 7 bare agent names
- **Line 108-111**: Default value logic — when no override is configured, it
  uses `val="$key"` (the bare name)
- **Line 111**: Output format — `- **${display_name} agent**: ${val}`

The script output currently looks like:

```
- **reviewer agent**: reviewer
- **codebase locator agent**: codebase-locator
```

It should instead output:

```
- **reviewer agent**: accelerator:reviewer
- **codebase locator agent**: accelerator:codebase-locator
```

#### `scripts/config-read-agent-name.sh`

- **Line 25**: Calls `config-read-value.sh "agents.$DEFAULT" "$DEFAULT"` — the
  second argument is the fallback, which is the bare name. This is used inline
  by `review-pr` and `review-plan` for their `subagent_type` parameter.

#### `scripts/config-dump.sh`

- **Lines 134-140**: Agent config keys (`"agents.reviewer"`, etc.)
- **Lines 144-150**: Default values array with bare names (`"reviewer"`, etc.)

#### `scripts/test-config.sh`

Contains extensive test assertions that validate bare agent names. Key sections:

- **Lines 273-274**: `assert_eq "outputs default" "reviewer" "$OUTPUT"`
- **Lines 293-339**: Override tests using bare names
- **Lines 751-757**: Assertions checking the output format of
  `config-read-agents.sh` (e.g., `\- \*\*reviewer agent\*\*: reviewer`)
- **Lines 961+**: Tests for `config-read-agent-name.sh`
- **Lines 990-991**: `assert_eq "outputs default" "reviewer" "$OUTPUT"`
- **Lines 1008-1009**: Override assertion
- **Lines 1117-1119**: Placement tests checking that `review-pr` and
  `review-plan` contain `config-read-agent-name.sh reviewer`

### 3. Skill Fallback Default Lines

All 10 skills that use agents have an identical fallback block immediately after
the `config-read-agents.sh` invocation:

```
If no "Agent Names" section appears above, use these defaults: reviewer,
codebase-locator, codebase-analyser, codebase-pattern-finder,
documents-locator, documents-analyser, web-search-researcher.
```

These appear in:

| Skill             | File:Line                                       |
|-------------------|-------------------------------------------------|
| research-codebase | `skills/research/research-codebase/SKILL.md:17` |
| create-plan       | `skills/planning/create-plan/SKILL.md:16`       |
| implement-plan    | `skills/planning/implement-plan/SKILL.md:17`    |
| review-plan       | `skills/planning/review-plan/SKILL.md:17`       |
| stress-test-plan  | `skills/planning/stress-test-plan/SKILL.md:17`  |
| validate-plan     | `skills/planning/validate-plan/SKILL.md:17`     |
| review-pr         | `skills/github/review-pr/SKILL.md:17`           |
| create-adr        | `skills/decisions/create-adr/SKILL.md:18`       |
| extract-adrs      | `skills/decisions/extract-adrs/SKILL.md:18`     |
| review-adr        | `skills/decisions/review-adr/SKILL.md:19`       |

### 4. Agent References Within Skill Bodies

Skills reference agents in two distinct ways:

#### A. Template-style `{agent name}` references (prose context)

These are human-readable references that guide the LLM on which agent to use.
They appear as `{codebase locator agent}`, `{reviewer agent}`, etc. and are
resolved by matching against the "Agent Names" section output. Found in:

| Skill               | Agents Referenced                                                                                                          | Example Lines    |
|---------------------|----------------------------------------------------------------------------------------------------------------------------|------------------|
| `research-codebase` | codebase-locator, codebase-analyser, codebase-pattern-finder, documents-locator, documents-analyser, web-search-researcher | SKILL.md:67-82   |
| `create-plan`       | codebase-locator, codebase-analyser, codebase-pattern-finder, documents-locator, documents-analyser                        | SKILL.md:76-151  |
| `stress-test-plan`  | codebase-locator, codebase-analyser                                                                                        | SKILL.md:42-43   |
| `review-plan`       | reviewer                                                                                                                   | SKILL.md:216     |
| `review-pr`         | reviewer                                                                                                                   | SKILL.md:240     |
| `create-adr`        | documents-locator, codebase-locator                                                                                        | SKILL.md:76-204  |
| `review-adr`        | documents-locator, codebase-locator                                                                                        | SKILL.md:110-111 |
| `extract-adrs`      | documents-locator, documents-analyser                                                                                      | SKILL.md:56-77   |
| `respond-to-pr`     | reviewer (in output format only, not as agent spawn)                                                                       | SKILL.md:234-250 |

**Note**: The `respond-to-pr` references to `{reviewer}` are in an output
format template, not agent spawn instructions. These do NOT need the plugin
prefix — they refer to the reviewer's name in the PR comment context.

#### B. Inline `subagent_type` references (deterministic spawn)

Only 2 skills use the inline `config-read-agent-name.sh` script for the
`subagent_type` parameter:

- `skills/github/review-pr/SKILL.md:286`:
  ```
  `subagent_type: "!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agent-name.sh reviewer`"`.
  ```
- `skills/planning/review-plan/SKILL.md:255`:
  ```
  `subagent_type: "!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agent-name.sh reviewer`"`.
  ```

These are resolved at preprocess time by `config-read-agent-name.sh`, which
currently outputs bare `reviewer` as the default.

### 5. Skills That Do NOT Reference Agents

The following skills do not invoke `config-read-agents.sh` and have no agent
references:

- `skills/config/configure/SKILL.md`
- `skills/config/init/SKILL.md`
- `skills/github/describe-pr/SKILL.md`
- `skills/github/respond-to-pr/SKILL.md`
- `skills/vcs/commit/SKILL.md`
- All 13 lens skills under `skills/review/lenses/*/SKILL.md`
- Both output format skills under `skills/review/output-formats/*/SKILL.md`

## Architecture Insights

The system uses a **dual-resolution strategy** for agent names:

1. **Bulk resolution** (`config-read-agents.sh`): Generates a markdown "Agent
   Names" section that is injected at the top of skill prompts. Skills then
   reference agents via `{agent name}` template syntax in prose. The LLM reads
   the agent names table and uses the value shown when spawning agents.

2. **Inline resolution** (`config-read-agent-name.sh`): Used at exactly 2
   spawn points (`review-pr:286`, `review-plan:255`) where the `subagent_type`
   parameter must be set deterministically at preprocess time.

Both resolution paths currently default to **bare names**. The fix needs to
ensure both paths produce prefixed names.

## Changes Required

### Core changes (scripts):

1. **`scripts/config-read-agents.sh:108`** — Change the default from
   `val="$key"` to `val="accelerator:$key"` (or parameterise the prefix)
2. **`scripts/config-read-agent-name.sh:25`** — Change the fallback default
   from `"$DEFAULT"` to `"accelerator:$DEFAULT"`
3. **`scripts/config-dump.sh:144-150`** — Update the defaults array to use
   prefixed names

### Skill fallback lines (10 files):

Update the "use these defaults" line in all 10 SKILL.md files to use prefixed
names:

```
If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator, ...
```

### Agent definition frontmatter (7 files) — NO CHANGES NEEDED:

The `name:` field in `agents/*.md` files should remain as bare names. The
plugin framework handles prefixing automatically.

### Test updates:

Update `scripts/test-config.sh` to expect prefixed names in all assertions.

## Resolved Questions

1. **Should user overrides also be prefixed?** No. User-provided override
   values should be passed through unchanged — the user is specifying the exact
   `subagent_type` value they want (which could be a project-local agent, a
   different plugin's agent, etc.).

2. **Is the prefix always `accelerator:`?** Hardcode `accelerator:` as the
   prefix. There is no `CLAUDE_PLUGIN_NAME` environment variable available to
   scripts (only `CLAUDE_PLUGIN_ROOT` and `CLAUDE_PLUGIN_DATA`). The plugin
   name is fixed and not designed to be renamed.

3. **Agent definition `name:` field**: Confirmed — the bare name in the
   definition is correct. The plugin framework adds the prefix automatically.
   No changes needed to `agents/*.md` files.
