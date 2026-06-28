---
type: work-item
id: "0167"
title: "Built-in config Command and Invocation-Contract Migration"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: draft
kind: story
priority: high
parent: "work-item:0136"
blocks: ["work-item:0169", "work-item:0173"]
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
relates_to: ["work-item:0106", "work-item:0107"]
tags: [rust, config, skills, invocation-contract, allowed-tools]
last_updated: "2026-06-28T17:01:56+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-188"
---

# 0167: Built-in config Command and Invocation-Contract Migration

**Kind**: Story
**Status**: Draft
**Priority**: High
**Author**: Toby Clemson

## Summary

Wire the full `accelerator config` command into the launcher over the shared
`config`/`corpus` crates, then migrate every skill from bare script-path
invocations to `accelerator …` calls — rewriting all SKILL.md call sites,
`allowed-tools` globs, and the 0107 lint guard in lockstep behind the stable bash
bootstrap path. This is the highest-blast-radius story in the epic.

## Context

ADR-0047 makes the CLI the native config reader and names `config get/set`;
ADR-0045 names the `configure` skill as the first proof of the skills-vs-CLI
division. The bash config cluster is the most-invoked code at skill-load time, and
every skill addresses it by bare path matched against `allowed-tools` prefix globs
(0106/0107). Moving to one `accelerator` command requires changing every call site
and glob together. Mirrors luminosity 0011 (configuration feature parity), but here
parity is with our own shell library plus the contract rewrite.

## Requirements

- Implement the built-in `config` subcommand family (compiled into the launcher, no
  sub-binary fetch) reaching parity with the bash surface: `get`/`set`, `path`,
  `paths`, `context`, `agents`, `agent`, `template`, `templates
  list|show|eject|diff|reset` (ADR-0021, 0/1/2 exit codes), `doc-type-paths`,
  `work`, `review`, `dump`, `summary`, `skill-context`/`skill-instructions`
  (ADR-0020), `browser-executor`, `init`. `config set` is net-new.
- Apply the interface-redesign principle (resolved Q7): clap named args, a
  `--format` switch, structured output for machine-like consumers — not a 1:1
  transliteration — while preserving prose-for-injection blocks (`## Agent Names`,
  `## Project Context`, `## Review Configuration`) and the meaningful exit codes.
- Provide the stable bash **bootstrap path** under `${CLAUDE_PLUGIN_ROOT}` that
  skills (and hooks) invoke; rewrite every SKILL.md `!`-preprocessor call site from
  script paths to `accelerator …`; update every `allowed-tools` glob to the new
  bootstrap-path shape; update the 0106 bare-path contract and the 0107 lint guard.
- Move the SessionStart config summary to `accelerator config summary` (with the
  `config-detect` hook handled via 0169's wrapper model).
- Prove the round trip end-to-end on the `configure` skill first (ADR-0045).

## Acceptance Criteria

- [ ] `accelerator config …` covers every behaviour the bash readers provided,
      verified against the repointed shell suites as a black-box parity gate.
- [ ] Every SKILL.md call site invokes `accelerator …` (no bare script paths to the
      migrated config cluster); every corresponding `allowed-tools` glob matches the
      new bootstrap path with no added permission prompts.
- [ ] The 0107 lint guard recognises and enforces the new invocation shape; 0106's
      contract is updated to match.
- [ ] `config set` writes config with re-parse/read-back integrity, gitignoring
      `config.local.md` appropriately.
- [ ] `mise run check` and the bare `mise run` pass with the config cluster's shell
      scripts removed and its suite floor decremented in the same change.

## Open Questions

- Exact new `allowed-tools` glob shape for the bootstrap path (single
  `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator *)` vs narrower per-subcommand globs)
  — decided during implementation, balancing permission tightness vs churn.

## Dependencies

- Blocked by: 0166 (shared config/corpus crates), 0164 (the bootstrap + launcher).
- Blocks: 0169 (hooks reuse the wrapper + contract), 0173 (subdomain call-site
  rewrites follow the established pattern).
- Relates to: 0106 (bare-path invocation), 0107 (lint guard) — both must change
  here.
- Parent: epic 0136.

## Assumptions

- The bootstrap path stays under `${CLAUDE_PLUGIN_ROOT}` so permission matches hold
  (resolved Q3).

## Technical Notes

- Source bash surface: the `config-read-*` family, `config-dump.sh`,
  `config-summary.sh`, template-management scripts, per-skill readers,
  `config-read-browser-executor.sh`, `skills/config/init/scripts/init.sh`.
- This story carries the largest behavioural-parity risk; the repointed shell
  suites are the oracle (resolved Q7 hybrid strategy).

## Drafting Notes

- Treated as the Phase 4 story; deliberately bundles the `config` command with the
  invocation-contract rewrite because they must land together for skills to keep
  working.

> Extracted from source documents without interactive enrichment.
> Acceptance criteria, dependencies, and kind may need refinement before
> promoting from `draft` to `ready`.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0045, ADR-0047, ADR-0020, ADR-0021
- Related: `meta/work/0106-invoke-plugin-scripts-by-bare-path.md`, `meta/work/0107-lint-skill-body-script-invocations.md`
- Mirrors (luminosity): https://github.com/atomicinnovation/luminosity/blob/main/meta/work/0011-configuration-feature-parity-with-accelerator.md
