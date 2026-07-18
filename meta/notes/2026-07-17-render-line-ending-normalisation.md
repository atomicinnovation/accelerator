---
type: note
id: "2026-07-17-render-line-ending-normalisation"
title: "Render round-trip emits mixed line endings for CRLF documents"
date: "2026-07-17T14:29:19+00:00"
author: "Toby Clemson"
producer: create-note
status: captured
topic: "Document render line-ending normalisation"
tags: ["cli-document", "render", "line-endings", "tech-debt"]
revision: "f1e67c168e1d42ce03ec676e492faaa9a565ef67"
repository: "accelerator"
last_updated: "2026-07-17T14:29:19+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Render round-trip emits mixed line endings for CRLF documents

Rendering a document that uses `\r\n` line endings produces a file with mixed
line endings: the frontmatter region is LF, the body is CRLF.

**Where it comes from** (`cli/document/src/render.rs`):

- The **frontmatter is regenerated** from the parsed `Yaml` tree by `emit`
  (`serde_saphyr::to_string`, which emits `\n`; the `ends_with('\n')` guard also
  uses bare `\n`). Any CRLF inside scalar values is normalised to LF by the
  parse-then-serialise cycle.
- The **fences are hardcoded LF**: `format!("---\n{}---\n{body}", …)`.
- The **body is preserved verbatim** as `content[body_start..]` by
  `fence::split`, so its original `\r\n` endings survive untouched.

Net result for a fully-CRLF input: everything up to and including the closing
`---\n` is LF, everything from the first body byte on stays CRLF. The seam is at
the closing fence.

Scope: only a *mixture* when there is a preserved body from a CRLF source. A
fresh render (`existing: None`) has an empty body and is all-LF. An LF body with
CRLF frontmatter also normalises to all-LF, since the frontmatter is always
regenerated.

**Future change**: normalise line endings on output so the body and frontmatter
agree — either detect the existing document's dominant line ending and match it,
or normalise everything to LF. Add a test pinning the round-trip of a CRLF
document either way. Relevant code: `emit` and `render` in
`cli/document/src/render.rs`, and `fence::split` in `cli/document/src/fence.rs`.
