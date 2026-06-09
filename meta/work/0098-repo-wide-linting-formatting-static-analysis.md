---
id: "0098"
title: "Repo-Wide Linting, Formatting, And Static Analysis Guardrails"
date: "2026-06-02T12:11:27+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: task
priority: medium
parent: ""
external_id: ""
tags: [tooling, linting, formatting, static-analysis, ci, guardrails]
last_updated: "2026-06-02T12:11:27+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0098: Repo-Wide Linting, Formatting, And Static Analysis Guardrails

**Kind**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Set up linting, formatting, and static-analysis tooling across the whole
repository — the visualiser frontend (TypeScript/React) and server (Rust),
plus the wider codebase (Python, shell scripts, and other file types where
it makes sense). The goal is two-fold: add durable guardrails so issues are
caught going forward, and fix all existing issues across the repository so
the tools pass cleanly from day one.

## Context

The repository is polyglot and currently lacks consistent automated quality
guardrails:

- **Visualiser frontend** — `skills/visualisation/visualise/frontend/`,
  TypeScript + React 19 + Vite + Vitest. No ESLint/Prettier (or Biome)
  configured; `tsc` type-checking runs only as part of `build`.
- **Visualiser server** — `skills/visualisation/visualise/server/`, Rust
  (Cargo). No enforced `rustfmt`/`clippy`.
- **Python** — ~700+ files across skills/scripts, with `pyproject.toml`
  present (pytest + uv configured) but no linter/formatter/type-checker
  (e.g. ruff, mypy).
- **Shell** — ~160 `.sh` scripts with no shellcheck/shfmt enforcement.
- Other candidates: CSS (stylelint), and `.editorconfig` already exists and
  should be honoured by chosen tooling.

## Requirements

- **Frontend**: configure a linter + formatter (ESLint + Prettier, or
  Biome) and enforce `tsc` type-checking as a standalone check.
- **Server (Rust)**: enforce `cargo fmt --check` and `cargo clippy` (deny
  warnings where reasonable).
- **Python**: configure ruff (lint + format) and a type-checker (e.g.
  mypy), wired through the existing `pyproject.toml`.
- **Shell**: configure shellcheck and shfmt for all `.sh` scripts.
- **Other**: add stylelint for CSS and any other low-cost analysers that
  fit; honour `.editorconfig`.
- **Fix all existing issues** so every configured tool passes cleanly
  across the entire repository.
- Provide a single, documented entry point to run all checks locally (e.g.
  a task runner / make target / script), and wire the checks into CI so
  regressions are blocked.

## Acceptance Criteria

- [ ] Frontend lint, format-check, and type-check commands exist and pass
  with zero errors.
- [ ] `cargo fmt --check` and `cargo clippy` pass with zero warnings on the
  server.
- [ ] Python lint, format-check, and type-check commands exist and pass
  with zero errors.
- [ ] shellcheck and shfmt pass on all `.sh` scripts.
- [ ] A single documented command runs the full suite of checks locally.
- [ ] CI runs all checks and fails the build on any violation.
- [ ] No pre-existing violations remain anywhere in the repository at the
  time of merge.

## Open Questions

- ESLint + Prettier vs Biome for the frontend — does the team have a
  preference?
- How strict should the initial rule sets be (e.g. clippy pedantic, mypy
  strict, ruff rule selection) — start lenient and ratchet, or start
  strict?
- Is there an existing CI system to wire these into, and should the same
  checks be enforced via a pre-commit hook?
- Should auto-fixable formatting be applied in one sweep commit (separate
  from the config-and-fix work) to keep diffs reviewable?

## Dependencies

- Related: none captured yet.

## Drafting Notes

- Captured as a stub without interactive enrichment. Tool selection
  (especially frontend ESLint/Prettier vs Biome), rule-set strictness, and
  CI wiring need decisions before promoting from `draft` to `ready`.
- Stack identified from a quick survey: frontend is TS/React+Vite, the
  visualiser server is Rust (Cargo), and the wider repo is predominantly
  Python plus shell scripts.
- Scoped as a single repo-wide task per the author's framing; it may warrant
  splitting into per-language work items (or an epic with children) given
  the breadth of fixing all existing issues.

## References

- Related: none
