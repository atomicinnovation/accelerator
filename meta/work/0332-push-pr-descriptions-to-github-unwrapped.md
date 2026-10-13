---
type: "work-item"
id: "0332"
title: "Push PR Descriptions to GitHub Unwrapped"
date: "2026-10-13T13:34:32+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
relates_to: ["work-item:0320", "work-item:0197"]
external_id: "PP-885"
tags: []
last_updated: "2026-10-13T13:34:32+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0332: Push PR Descriptions to GitHub Unwrapped

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a developer reading a pull request on GitHub, I want the description
`describe-pr` posts to flow at the full width of the page, so that its
prose is not broken into ragged 80-column lines.

`describe-pr` writes its artefact under repository conventions, which
here and in many repositories means hard-wrapping Markdown at 80
columns, and posts that text verbatim. GitHub renders each of those
newlines as a visible line break. This story unwraps the body at the
point of push, unconditionally, while leaving the artefact in `meta/prs/`
wrapped as the repository's configuration dictates.

## Context

`describe-pr` saves its description to `{prs directory}/{number}-description.md`,
strips the frontmatter into `{tmp directory}/pr-body-{number}.md`, and hands
that file to `accelerator collaboration pr update-body`, which PATCHes the
body through the GitHub REST API unchanged.

Nothing in the skill asks for wrapping. It arises from the template body
(hard-wrapped near 80, as every built-in template is), from repository
conventions such as `.editorconfig` `max_line_length` and CLAUDE.md line
width rules, and from the model's habit of hard-wrapping Markdown prose.
Instructing the model not to wrap would conflict with those conventions
for the artefact and would remain probabilistic for the posted body.

`review-pr` and `respond-to-pr` already render full width: they post
JSON payloads whose `body` strings carry no newlines inside paragraphs.

## Requirements

- `accelerator collaboration pr update-body` unwraps the body before
  sending it: within each paragraph, list item, and blockquote paragraph,
  soft line breaks become a single space, so each occupies one physical
  line.
- The unwrap is Markdown-aware, parsing the body rather than joining
  lines textually.
- The unwrap reads no repository configuration (`.editorconfig`,
  CLAUDE.md, `.accelerator/` skill instructions, a custom
  `pr-description` template) and offers no opt-out.
- The artefact in `meta/prs/` continues to follow the repository's
  conventions; this story does not change how it is written.
- The unwrap leaves byte-identical: fenced and indented code blocks,
  tables, HTML blocks, headings, thematic breaks, link reference
  definitions, explicit hard breaks (two trailing spaces or a trailing
  `\`), and the blank lines between blocks.
- The unwrap preserves list markers, nesting indentation, task-list
  checkboxes, and blockquote `>` prefixes.
- The unwrap is idempotent.

## Acceptance Criteria

- [ ] Given a description artefact hard-wrapped at 80 columns, when
      `describe-pr` posts it, then no paragraph, list item, or blockquote
      paragraph in the PATCHed body contains an internal newline.
- [ ] Given a repository whose `.editorconfig` sets
      `max_line_length = 80` and whose CLAUDE.md requires 80-column
      Markdown, when `describe-pr` runs, then the artefact in `meta/prs/`
      is wrapped to 80 columns and the posted body is unwrapped.
- [ ] Given a body containing a fenced code block, an indented code
      block, a table, an HTML block, and a line ending in a hard break,
      when it is unwrapped, then each of those constructs is
      byte-identical in the output.
- [ ] Given a body with nested lists, task-list items, and multi-line
      blockquotes, when it is unwrapped, then markers, indentation,
      checkboxes, and `>` prefixes are preserved and each item's text is
      on one line.
- [ ] Given any body in a fixture corpus, when it is unwrapped, then the
      GFM block tree of the output equals that of the input once soft
      breaks are treated as spaces.
- [ ] Given an already unwrapped body, when it is unwrapped again, then
      the output equals the input.
- [ ] Given a `review-pr` or `respond-to-pr` run, when it posts to
      GitHub, then its bodies render full width as they do today.

## Open Questions

## Dependencies

- Blocked by: none
- Blocks: none

## Assumptions

- GitHub renders single newlines in pull request bodies as visible line
  breaks, which is what makes hard-wrapped prose render narrow.
- Placing the unwrap in `update-body`, rather than in a `describe-pr`-only
  step, is intended: unwrapped bodies are a property of pushing to
  GitHub, so any future caller inherits it.

## Technical Notes

- The PATCH lives in `cli/collaboration/src/update_body.rs`
  (`update_pull_request_body`); the CLI entry point is in
  `cli/collaboration-cli/src/cli.rs`.
- Built-in template bodies stay wrapped at about 80 columns; all 19 are
  already consistent, so no template changes are needed.
- 0320 meets the same hard-wrapping problem on the Linear push path; an
  unwrap module shareable between the two would avoid duplicating the
  Markdown handling.

## Drafting Notes

- "Never wrap" is read as one physical line per paragraph, list item,
  and blockquote paragraph, not as removing every newline.
- "Regardless of configuration" applies to the posted body only; the
  artefact keeps honouring repository configuration.
- "Consistent across built-in templates" is treated as already satisfied,
  since templates stay wrapped and only the push changes.
- Frontmatter is out of scope because `describe-pr` strips it before
  posting.

## References

- Related: 0320 (Linear Push Corrupts Markdown Content)
- Related: 0197 (accelerator-collaboration: PR Helper CLI)
- Source: `skills/github/describe-pr/SKILL.md`
