---
work_item_id: "0077"
title: "Shadow and Dark-Accent Token Audit"
date: "2026-05-21T09:16:34+00:00"
author: Toby Clemson
type: task
status: draft
priority: medium
parent: ""
tags: [design, frontend, tokens, audit]
---

# 0077: Shadow and Dark-Accent Token Audit

**Type**: Task
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Audit the current app's `--ac-shadow-soft` / `--ac-shadow-lift` values
and the dark-theme `--ac-accent` / `--ac-accent-2` values against the
prototype, then either align them with the prototype's elevation curve
and brighter dark accents, or document the intentional divergence.

## Context

The prototype defines `--ac-shadow-soft` and `--ac-shadow-lift` with
light- and dark-theme variants (`0 1px 2px rgba(10,17,27,0.04),
0 8px 28px rgba(10,17,27,0.06)` and `0 2px 4px rgba(10,17,27,0.06),
0 20px 60px rgba(10,17,27,0.10)`); the current app's
`global.css:172-173,239-240` declares the same token names per theme but
the values were not captured in the inventory, indicating either drift
or undocumented parity.

The prototype's dark theme remaps `--ac-accent` to `#8A90E8` and
`--ac-accent-2` to `#E86A6B` so accents preserve contrast on the
deep-night surface. The current app's dark mirror is documented from
source only and was not visually verified, so we need to confirm whether
the dark accent in the current app actually shifts and, if it does not,
migrate it to the brighter prototype values.

## Requirements

- Read the current `--ac-shadow-soft` / `--ac-shadow-lift` values from
  `global.css:172-173,239-240` for both light and dark themes.
- Compare against the prototype's declared values.
- Either align the current values with the prototype's elevation curve,
  or document the intentional divergence in the PR description.
- Verify the current app's dark `--ac-accent` and `--ac-accent-2`
  values visually (not just from source).
- If the dark accents do not actually shift in the current app, migrate
  them to the prototype values (`#8A90E8`, `#E86A6B`).

## Acceptance Criteria

- [ ] Documented comparison of current vs prototype shadow values for
  both themes.
- [ ] Shadow values either match the prototype or carry a documented
  divergence justification.
- [ ] Dark `--ac-accent` and `--ac-accent-2` visually verified;
  migrated to the brighter prototype values if not already.
- [ ] Visual-regression baselines captured in light and dark for any
  surface where shadow or accent rendering changes.

## Open Questions

- If shadow values diverge from the prototype, what is the justification
  — accessibility, brand intent, or oversight?

## Dependencies

- Blocked by: 0033 (shadow tokens delivered) and 0034 (dark-theme
  toggle wired so verification is possible).
- Blocks: none directly; downstream design polish depends on this audit
  for accurate elevation expectations.

## Assumptions

- The prototype's shadow elevation curve is the intended target; any
  current-app divergence is drift unless documented otherwise.

## Technical Notes

- Verification needs Playwright or manual capture under `data-theme="dark"`.

## Drafting Notes

- Extracted from source documents without interactive enrichment.
  Acceptance criteria, dependencies, and type may need refinement before
  promoting from `draft` to `ready`.

## References

- Source: `meta/research/design-gaps/2026-05-21-current-app-vs-claude-design-prototype.md`
- Related: 0033, 0034
