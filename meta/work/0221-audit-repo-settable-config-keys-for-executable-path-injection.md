---
type: work-item
id: "0221"
title: "Audit Repo-Settable Config Keys for Executable-Path Injection"
date: "2026-08-20T00:00:00+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: task
priority: medium
relates_to: ["work-item:0196"]
tags: [security, config, visualiser, design]
last_updated: "2026-08-20T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0221: Audit Repo-Settable Config Keys for Executable-Path Injection

**Kind**: Task
**Status**: Draft
**Priority**: Medium

## Summary

Audit every configuration key whose value can name an executable or path the
tooling runs, and hold each to the standard the vendored-runtime work applied to
`design.browser_path`: settable only at the personal (gitignored) level, and
refusing a value that resolves inside the repository being operated on.
`visualiser.editor` — the precedent `design.browser_path` was modelled on —
still fails that standard.

## Context

🔒 A repo-tracked config value that names an executable the tooling later runs is
an untrusted-input-to-code-execution path: cloning a hostile repository would be
enough to have the tooling execute an attacker-named binary.

The vendored-runtime work
(`plan:2026-08-11-0196-design-vendored-runtime-distribution`) closed this for
`design.browser_path` — restricting it to `.accelerator/config.local.md` and
refusing a value canonicalising inside the inventoried repository — but did not
ship the corresponding fix for the key it copied the shape from.
`visualiser.editor` remains settable from a repo-tracked `.accelerator/config.md`.

## Requirements

- Enumerate the config keys whose value can name an executable or a path the
  tooling executes (start from `visualiser.editor` and its siblings).
- For each, decide whether it should be personal-level-only and/or refuse a
  value resolving inside the operated-on repository, applying the same two
  barriers `design.browser_path` now carries.
- Apply the barrier where warranted; document the rationale where a key is
  judged safe as-is.

## Acceptance Criteria

- [ ] Given the audit, then every executable-or-path-naming config key is either
      restricted to the personal level (and refuses a repo-inside value) or is
      documented as safe with a rationale.
- [ ] Given a repo-tracked `.accelerator/config.md` sets a restricted key, when
      the tooling reads it, then the value is ignored with a warning naming the
      personal route.

## Dependencies

- Relates to: 0196 — which established the standard on `design.browser_path`.

## References

- Surfaced by:
  `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md` (Removal
  sweep, follow-up work items)
- Related: 0196
