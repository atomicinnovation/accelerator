---
type: note
id: "2026-05-19-config-read-path-missing-design-keys"
title: "Bug: `config-read-path.sh` has no defaults for `design_inventories` / `design_gaps`"
date: "2026-05-19T00:00:00+00:00"
author: Toby Clemson
producer: create-note
status: captured
topic: "Bug: `config-read-path.sh` has no defaults for `design_inventories` / `design_gaps`"
tags: []
revision: "11218123a1e4"
repository: "ticket-management"
last_updated: "2026-05-19T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Bug: `config-read-path.sh` has no defaults for `design_inventories` / `design_gaps`

## Problem (severity: low — noisy but non-blocking)

When the `inventory-design` skill (and presumably `analyse-design-gaps`)
resolve their configured directories during the preamble, they emit:

```
**Design gaps directory**: config-read-path.sh: warning: unknown key 'design_gaps' — no centralized default
**Design inventories directory**: config-read-path.sh: warning: unknown key 'design_inventories' — no centralized default
```

These warnings appear inline in the skill output where a resolved path
should be — confusing because the warning line *replaces* the path,
making it look as if no default exists at all.

Verified by direct invocation:

```
$ config-read-path.sh design_inventories
config-read-path.sh: warning: unknown key 'design_inventories' — no centralized default
```

The skill itself appears to fall back to a default path internally, so
the inventory still gets written — but the warning is shown to the user
on every invocation.

## Root cause

`scripts/config-read-path.sh` keeps a centralised allow-list of known
config keys plus their default relative paths. The design-skills feature
added two new keys (`design_inventories`, `design_gaps`) but the
script's allow-list was not updated.

## Suggested path forward

Add the two keys to `config-read-path.sh`'s centralised defaults:

- `design_inventories` → `meta/design-inventories`
- `design_gaps` → `meta/design-gaps`

Match whatever the skill currently falls back to. Trivial one-line
additions per key.

## References

- Script: `scripts/config-read-path.sh`
- Skills that resolve these keys: `skills/design/inventory-design`,
  `skills/design/analyse-design-gaps`
- Observed warning text contains hint `"no centralized default"`,
  pointing directly at the missing-allow-list cause.
