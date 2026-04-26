---
title: "Add copy-to-clipboard button to code blocks in documentation site"
type: story
status: ready
priority: medium
---

# Add Copy-to-Clipboard Button to Code Blocks in Documentation Site

## Summary

Add a "Copy" button to every code block in the documentation site so users can
copy code snippets without manually selecting text.

## Context

Users frequently need to copy command-line examples and configuration snippets
from the documentation. Manual text selection is error-prone, especially for
multi-line snippets. Multiple support requests have cited copied-but-incomplete
snippets as the cause of setup failures.

## Requirements

1. Every code block rendered by the documentation site must display a "Copy"
   button in the top-right corner of the block.
2. Clicking the "Copy" button copies the full content of the code block to the
   system clipboard.
3. After a successful copy, the button label changes to "Copied!" for 2 seconds
   before reverting to "Copy".

## Acceptance Criteria

- A "Copy" button appears on every code block when the user hovers over it or
  focuses the block (visible by default on mobile where hover is unavailable).
- Clicking "Copy" on a multi-line bash snippet copies the exact text content
  of the block, including newlines, without HTML markup or extra whitespace.
- The "Copied!" confirmation state appears for 2 ± 0.5 seconds, verified by
  a Playwright snapshot test.
- The feature degrades gracefully in browsers without Clipboard API support:
  the button is hidden rather than shown in a broken state.

## Dependencies

- Documentation site uses the `@company/docs-components` package (already
  ships a `<CodeBlock>` component that will need modification)

## Assumptions

- The Clipboard API (`navigator.clipboard.writeText`) is available in all
  browsers we officially support (Chrome 88+, Firefox 90+, Safari 14+).
- No back-end changes are needed; this is a pure front-end addition.
