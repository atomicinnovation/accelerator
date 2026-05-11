---
work_item_id: "0052"
title: "Make documents-locator Agent Paths Config-Driven via Preloaded Skill"
date: "2026-05-07T21:50:34+00:00"
author: Toby Clemson
type: story
status: done
priority: medium
parent: ""
tags: [agents, configuration, documents-locator]
---

# 0052: Make documents-locator Agent Paths Config-Driven via Preloaded Skill

**Type**: Story
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As an operator with remapped artifact directories, I want the documents-locator
agent to search the paths I have configured rather than the plugin defaults, so
that agent invocations in my project find documents in the correct locations.

The agent currently embeds hardcoded paths (`meta/research/`, `meta/plans/`,
`meta/work/`, etc.) in its definition. These match the plugin defaults but
ignore any `paths.*` overrides set in `.accelerator/config.md` or
`.accelerator/config.local.md`. The fix is to preload a bang-command-processed
path-resolution skill into the agent via the `skills:` frontmatter key, injecting
all configured paths before the agent acts.

## Context

Agent definition files (`agents/*.md`) are static Markdown injected into the
model's context at invocation time. Unlike SKILL.md files, they do not support
inline bang-command (`!`) preprocessing directly — bang preprocessing is the
harness mechanism that executes shell commands embedded in skill files (lines
starting with `!`) and replaces them with their output before injection. Claude
Code's `skills:` frontmatter key — currently supported in SKILL.md files, and
extended to agent definitions as part of this story — allows named skills to be
preloaded into an agent's context at startup. Preloaded skills are
bang-preprocessed before injection, which means a thin path-resolution skill can
resolve all configured paths via the existing `config-read-path.sh`
infrastructure and emit them as a structured block the agent can reference.

This approach is reliable (resolution happens before the agent acts, not through
LLM instruction-following), and supports auto-discovery of new document types: a
`config-read-all-paths.sh` script enumerates all configured path keys at runtime,
so new types added to config appear in the preloaded block without any edit to
the agent definition or the resolution skill.

A survey of all `agents/*.md` files should confirm whether any other agent
definitions contain hardcoded paths; any found must be resolved under the same
mechanism.

The tech debt was first documented in
`meta/notes/2026-04-26-agents-hardcode-default-directory-locations.md`.

## Requirements

1. A new `scripts/config-read-all-paths.sh` script that reads all configured
   `paths.*` keys (with their defaults) from `.accelerator/config.md` and
   `.accelerator/config.local.md` and emits them as a structured block suitable
   for injection into agent context.
2. A new path-resolution skill at `skills/config/paths/SKILL.md` that calls
   `config-read-all-paths.sh` via a bang command, emitting all resolved paths as
   a structured block. The skill must not set `disable-model-invocation: true` —
   the harness preload pipeline skips any skill with `disable-model-invocation: true` set before
   injecting it, which would prevent the path block from being available.
3. `agents/documents-locator.md` gains a `skills:` frontmatter entry referencing
   the path-resolution skill.
4. The agent body is updated to treat the preloaded path block as authoritative,
   using configured paths in place of any hardcoded defaults.
5. When a `paths.*` key is absent from config or config is not present, the
   plugin default for that key is used — no behaviour change for unconfigured
   projects.
6. When a new path type is added to config (e.g. `paths.specs: meta/specs`), the
   agent includes it in its search scope automatically — without editing
   `agents/documents-locator.md` or the path-resolution skill.
7. All `agents/*.md` files are surveyed for hardcoded directory paths. Any found
   are resolved under the same mechanism and the survey result is captured in
   the implementation.
8. A `global` path key is added to `scripts/config-read-path.sh` with default
   `meta/global`, covering the `meta/global/` directory already referenced in
   `agents/documents-locator.md`. The init process is updated to recognise this
   key when configuring path overrides.
9. The harness is extended to support the `skills:` frontmatter key in agent
   definition files (`agents/*.md`): listed skills are bang-preprocessed and
   injected into the agent's context before the agent acts.

## Acceptance Criteria

- [ ] Given only `paths.work: work-items` is set in `.accelerator/config.md`
  (all other path keys absent), when documents-locator is invoked, then the
  agent searches `work-items/` for work items and the plugin-default paths for
  all other document types
- [ ] Given no config override present, when invoked, then the agent searches the
  plugin-default paths (e.g. `meta/plans`, `meta/work`) — identical behaviour
  to today for unconfigured projects
- [ ] Given `paths.specs: meta/specs` added to config, when documents-locator is
  invoked, then the agent searches `meta/specs/` as part of its document
  discovery without editing `agents/documents-locator.md` or the path-resolution
  skill
- [ ] Given `paths.global: custom-global` in `.accelerator/config.md`, when
  documents-locator is invoked, then the agent searches `custom-global/` rather than
  `meta/global/`
- [ ] Given all `agents/*.md` files have been surveyed, then the survey result —
  listing each file examined and whether hardcoded paths were found — is recorded
  in the Technical Notes of the implementation
- [ ] Given `skills: [paths]` is set in the documents-locator frontmatter and
  `paths.work: custom-work` is in `.accelerator/config.md`, when documents-locator
  is invoked, then the agent searches `custom-work/` for work items (confirming the
  preloaded path block was acted upon)
- [ ] Given `skills: [paths]` is added to a second agent definition's frontmatter
  (not documents-locator) and `paths.work: custom-work` is in
  `.accelerator/config.md`, when that agent is invoked, then the agent searches
  `custom-work/` — confirming the harness extension is agent-agnostic
- [ ] Given the init process is run on a project with no existing config, when
  the operator reaches the path-configuration step, then `paths.global` is offered
  as a configurable key with default `meta/global`, consistent with all other path
  keys
- [ ] The path-resolution skill body contains no reference to `documents-locator`
  or any document type by name (verifiable via grep), and its entry can be added
  to any other agent definition's `skills:` list without modifying
  `skills/config/paths/SKILL.md`

## Open Questions

- _Resolved_: `config-read-all-paths.sh` emits only the subset of path keys
  relevant to document discovery — `plans`, `research`, `decisions`, `prs`,
  `validations`, `review_plans`, `review_prs`, `review_work`, `work`, `notes`,
  `global` (11 keys). The non-document keys `tmp`, `templates`, and `integrations`
  are excluded to keep the injected block minimal. Alignment with 0030's
  `PATH_DEFAULTS` will cover this subset only.

## Dependencies

- Blocked by: none (the required harness extension, Requirement 9, is included in
  this story's scope; note that Requirements 2–4 depend on Requirement 9 being
  complete before they can be tested end-to-end)
- Follow-up required when 0030 lands: `config-read-all-paths.sh` will need
  updating to source `config-defaults.sh` rather than maintaining its own key
  list (see Related entry for 0030 below)
- Related: `meta/work/0030-centralise-path-defaults.md` (in draft) — the
  centralised `PATH_DEFAULTS` it proposes is the natural source of truth for the
  key vocabulary emitted by `config-read-all-paths.sh`. This story can be
  implemented independently but should align with 0030's vocabulary once that
  lands.

## Assumptions

- Only `documents-locator.md` contains hardcoded artifact directory paths in
  `agents/`; the survey confirms this — no other agent files were found. Scope
  does not expand.
- The fallback for each path key is the same default already defined in
  `config-read-path.sh` — identical to today's behaviour for unconfigured
  projects.
- "Auto-discovery" means the agent definition and path-resolution skill require
  zero edits when new `paths.*` keys appear in config. This is the binding
  constraint that requires `config-read-all-paths.sh` rather than individual
  per-key bang commands.

## Technical Notes

- `scripts/config-read-all-paths.sh` — new script, no arguments, emits the
  document-discovery subset of path keys (11 keys: `plans`, `research`,
  `decisions`, `prs`, `validations`, `review_plans`, `review_prs`, `review_work`,
  `work`, `notes`, `global`) as a Markdown list of `key: resolved-value` entries
  wrapped in a labelled fenced block (e.g. `## Configured Paths`). Non-document
  keys (`tmp`, `templates`, `integrations`) are excluded. Uses the same defaults
  as `config-read-path.sh` for each key.
- `skills/config/paths/SKILL.md` — minimal body; one bang call to
  `config-read-all-paths.sh` wrapped in a labelled section. Must not set
  `disable-model-invocation: true`. Can be preloaded by any agent or skill
  that needs config-resolved paths, not only documents-locator.
- `agents/documents-locator.md` frontmatter: add `skills: [paths]`. Body:
  replace hardcoded path references with an instruction to use the paths emitted
  in the preloaded section; add a fallback note for keys not present in the
  block.
- Preloaded skill content is bang-preprocessed and injected before the agent
  acts — resolution does not depend on LLM instruction-following.
- Work item 0030 (`centralise-path-defaults.md`) proposes extracting
  `PATH_KEYS`/`PATH_DEFAULTS` into `scripts/config-defaults.sh`. Once that
  lands, `config-read-all-paths.sh` should source that file to stay in sync
  rather than maintaining its own key list.

**Codebase survey — hardcoded path locations in `agents/documents-locator.md`:**
- Lines 15–21: search instructions (`meta/research/`, `meta/plans/`, `meta/decisions/`, `meta/reviews/`, `meta/validations/`, `meta/global/`)
- Lines 49–59: directory tree diagram (all paths as literals)
- Lines 75–100: example output block (all paths as literals)
- No other `agents/*.md` file contains hardcoded `meta/` paths — survey complete.

**`scripts/config-read-path.sh` key vocabulary — 14 keys and defaults (13 existing + `global` added by this story):**
`plans→meta/plans`, `research→meta/research`, `decisions→meta/decisions`,
`prs→meta/prs`, `validations→meta/validations`, `review_plans→meta/reviews/plans`,
`review_prs→meta/reviews/prs`, `review_work→meta/reviews/work`,
`templates→.accelerator/templates`, `work→meta/work`, `notes→meta/notes`,
`tmp→.accelerator/tmp`, `integrations→.accelerator/state/integrations`,
`global→meta/global`

**Decision — `meta/global/`:** Line 21 of `agents/documents-locator.md` references
`meta/global/`. A `global` path key with default `meta/global` is added to
`config-read-path.sh` as part of this story (Requirement 8), making it
configurable and managed by the init process alongside all other path keys.

**Harness extension — `skills:` frontmatter in agent files:** Zero files in the
repo currently use a `skills:` frontmatter key in agent definitions. The `!`
bang-preprocessor pattern is used in SKILL.md files (e.g. `skills/config/init/SKILL.md`
lines 20–31) but not in agent `.md` files. Requirement 9 extends this support to
agent definitions; implementation should target the same preprocessing pipeline.

## Drafting Notes

- Bang commands in agent definition bodies are not supported (SKILL.md-only
  feature); self-resolving via Grep was considered but carries instruction-follow
  reliability risk. The `skills:` preload mechanism is the correct path; extending
  it to agent definition files is included in this story's scope (Requirement 9).
- The note at `meta/notes/2026-04-26-agents-hardcode-default-directory-locations.md`
  called this "Option 2 (self-resolving)"; the `skills:` preload approach
  supersedes that framing — it achieves the same goal without relying on the
  agent to decide to read config mid-task.
- The skill is placed at `skills/config/paths/` rather than
  `skills/config/agent-paths/` because `skills/config/paths/` is accurate for
  the full key vocabulary (some keys, e.g. `tmp`, `templates`, resolve outside
  `meta/`) and the skill is reusable beyond agent definitions.
- Priority set to medium: the bug affects operators with remapped directories but
  has no impact on unconfigured (default) projects, which are the majority.

## References

- Source: `meta/notes/2026-04-26-agents-hardcode-default-directory-locations.md`
  (note: uses an older config file naming convention — `config.user.yaml` /
  `config.team.yaml` — the current system uses `.accelerator/config.md` and
  `.accelerator/config.local.md`)
- Related: `meta/work/0030-centralise-path-defaults.md`
- Related: `meta/decisions/ADR-0022-work-item-terminology.md`
