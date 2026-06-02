---
type: work-item
id: "0095"
title: "Markdown Checkboxes Always Styled For Dark Mode"
date: "2026-06-02T12:11:27+00:00"
author: Toby Clemson
producer: create-work-item
status: draft
kind: bug
priority: medium
parent: ""
external_id: ""
tags: [visualiser, markdown, theme, dark-mode, bug]
last_updated: "2026-06-02T12:11:27+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0095: Markdown Checkboxes Always Styled For Dark Mode

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Task-list checkboxes in rendered artifact markdown always use dark-mode
styling, regardless of whether the active theme is light or dark. In light
mode they appear visually broken or low-contrast.

## Context

The visualiser supports light and dark themes via theme tokens, but the
rendered markdown checkbox does not react to the active theme — it renders
with dark-mode appearance in both modes.

## Requirements

- Checkbox rendering respects the active theme, consuming theme tokens so
  it adapts to both light and dark mode.
- Both checked and unchecked states render correctly in each mode.

## Acceptance Criteria

- [ ] In light mode, rendered checkboxes use light-mode-appropriate colours
  with adequate contrast.
- [ ] In dark mode, rendered checkboxes remain correct.
- [ ] Checked and unchecked states are both correct in each mode.

## Open Questions

- Is the checkbox styled with a hardcoded colour, or referencing a
  dark-only token rather than a theme-reactive one?

## Dependencies

- Related: 0034 (theme and font mode toggles), 0077 (shadow and dark accent
  token audit).

## Drafting Notes

- Captured as a stub without interactive enrichment. Acceptance criteria
  and priority may need refinement before promoting from `draft` to
  `ready`.

## References

- Related: 0034, 0077
