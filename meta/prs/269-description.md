---
type: "pr-description"
id: "269"
title: "Note that wide tables render badly in the visualiser"
date: "2026-11-26T00:20:38+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/269"
pr_number: 269
tags: ["visualiser", "notes"]
revision: "58a70687ac3c1fcd74afc0ba97eb05ce10a0eee2"
repository: "accelerator"
last_updated: "2026-11-26T00:20:38+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Note that wide tables render badly in the visualiser

## Summary

Adds a note recording that Markdown tables wider than the document column
don't render properly in the visualiser's library document view. It captures
the defect so it can become a work item later.

## Changes

- Adds `meta/notes/2026-11-25-visualiser-wide-tables-render-badly.md`.
- The note names `/library/decisions/doc-types-as-declared-data` (ADR-0078)
  as the example. That ADR's "Set shapes" section has a six-column table whose
  longest row is 165 characters.

## Context

No work item covers this yet. Run `/extract-work-items` on the note, or
`/create-work-item`, when the fix is scheduled.

## Testing

- [x] `accelerator corpus frontmatter validate --file` passes on the note.
- [ ] `mise run check` was not run. The PR changes only `meta/` Markdown,
      which none of the check components cover.

## Notes for Reviewers

- ADR-0078 is not on `main` yet: it is proposed in in-flight work for PP-994.
  So the note names it in the text rather than as a typed `relates_to` link,
  and the example route only resolves once that work lands.
- The note records that the table renders badly, not how. Nobody has checked
  whether it overflows the column, is clipped or wraps cell text, and the
  person who picks up the fix should start there.
