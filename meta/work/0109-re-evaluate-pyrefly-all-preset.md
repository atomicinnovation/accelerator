---
type: work-item
id: "0109"
title: "Re-Evaluate Pyrefly's All Preset When v1.1 Ships"
date: "2026-06-11T12:00:00+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: task
priority: low
relates_to: ["work-item:0098"]
tags: [tooling, static-analysis, pyrefly, types, build-system]
last_updated: "2026-06-11T12:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0109: Re-Evaluate Pyrefly's All Preset When v1.1 Ships

**Kind**: Task
**Status**: Draft
**Priority**: Low

## Summary

0098 adopted pyrefly at the `strict` preset for the repo-root Python build
system (`[tool.pyrefly] preset = "strict"` in `pyproject.toml`, pinned to
`pipx:pyrefly 1.0.0`). The work item noted that pyrefly's stricter `all` preset
should be re-evaluated once it stabilises (it was not production-ready at 1.0).
This is the follow-up to make that assessment when pyrefly 1.1 ships.

## Why

`strict` governs 0098 and is the correct baseline today. `all` enables
additional, more opinionated checks that were either unstable or
disproportionately noisy at 1.0. When 1.1 lands, the cost/benefit of `all` (or
of individual `all`-only error kinds promoted to errors) should be reassessed
against the `tasks/` corpus, rather than left implicitly at `strict` forever by
default.

## Acceptance criteria

- On pyrefly 1.1 release, run `pyrefly check` under `preset = "all"` against the
  in-scope Python (`tasks/`, the coverage guard's set) and record the residual.
- Decide, with rationale, whether to: adopt `all`, adopt `strict` + a curated
  subset of `all`-only error kinds promoted to `error`, or stay on `strict`.
- If the pin moves, update `mise.toml` (`pipx:pyrefly`) and re-run
  `mise run build-system:check`; keep every new suppression justified.

## References

- Parent guardrails work item: `meta/work/0098-repo-wide-linting-formatting-static-analysis.md`
- Plan: `meta/plans/2026-06-10-0098-repo-wide-linting-formatting-static-analysis.md`
- pyrefly presets: https://pyrefly.org/en/docs/configuration/
