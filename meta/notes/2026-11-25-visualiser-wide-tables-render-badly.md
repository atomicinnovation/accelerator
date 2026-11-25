---
type: "note"
id: "2026-11-25-visualiser-wide-tables-render-badly"
title: "Wide tables don't render properly in the visualiser"
date: "2026-11-25T22:21:41+00:00"
author: "Toby Clemson"
producer: "create-note"
status: "captured"
topic: "Wide Markdown tables in the visualiser's document view"
tags: ["visualiser", "markdown", "tables", "rendering"]
revision: "98f6f98e194d0102bebf10eeb851daea56e43f24"
repository: "accelerator"
last_updated: "2026-11-25T22:21:41+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Wide tables don't render properly in the visualiser

Markdown tables wider than the document column don't render properly in the
visualiser's library document view.

Example: `/library/decisions/doc-types-as-declared-data`, which is ADR-0078,
Doc Types as Declared Data. Its "Set shapes" section has a six-column table
whose longest row is 165 characters, twice the 80-column line width the
corpus is written to.

Any document with a table of five or more columns, or long cell text, is
likely to hit the same problem.
