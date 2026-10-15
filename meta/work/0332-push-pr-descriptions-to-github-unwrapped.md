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
relates_to: ["work-item:0320", "work-item:0197", "work-item:0071", "work-item:0152"]
external_id: "PP-885"
tags: []
last_updated: "2026-10-14T13:05:00+00:00"
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
wrapped as the repository's conventions dictate.

## Context

`describe-pr` saves its description to `{prs directory}/{number}-description.md`,
strips the frontmatter into `{tmp directory}/pr-body-{number}.md`, and hands
that file to `accelerator collaboration pr update-body`, which PATCHes the
body through the GitHub REST API unchanged.

GitHub's documentation confirms the rendering: in issues, pull requests,
and discussions a single newline renders as a line break, whereas the same
text in a `.md` file renders on one line. GitHub has treated newlines in
paragraph-like content this way since the original GFM.

Nothing in the skill asks for wrapping. It arises from the template body
(hard-wrapped near 80, as every built-in template is), from repository
conventions such as `.editorconfig` `max_line_length` and CLAUDE.md line
width rules, and from the model's habit of hard-wrapping Markdown prose.
Instructing the model not to wrap would conflict with those conventions
for the artefact and would remain probabilistic for the posted body.

`describe-pr` is the only skill that calls `update-body`; `raise-pr`
reaches it through `describe-pr`, and future PR skills (0152) that post
bodies through `update-body` inherit the unwrap. 0071 required the posted
body to match the body file byte-for-byte; this story deliberately
replaces that contract, so the posted body differs from the file at soft
breaks only.

`review-pr` and `respond-to-pr` are out of scope: they post JSON payloads
through other paths, and their `body` strings already carry no newlines
inside paragraphs.

## Requirements

- `accelerator collaboration pr update-body` applies the unwrap to the
  body before sending it. The unwrap is an operation in its own right that
  replaces each GFM soft line break with a single space. The replaced
  span is any whitespace before the line ending, the line ending (`\n` or
  `\r\n`), the following line's container prefix (`>` markers, list
  continuation indentation), and its leading whitespace; a lazy
  continuation line has no prefix, so the span stops at its leading
  whitespace. Soft breaks inside paragraphs, list items, blockquotes,
  footnote definitions, and setext headings are all replaced.
- A line break is a hard break only as GitHub reads one: two or more
  spaces, or a `\`, immediately before the line ending. Any other
  trailing whitespace before a line ending, such as two tabs or two
  spaces followed by a tab, is a soft break and replaced as one, even
  where the unwrap's parser reads it as hard.
- Every byte outside a replaced span is preserved. As a guarantee in its
  own right, not only a consequence of the span rule, the unwrap leaves
  code blocks, code spans, tables, HTML blocks and inline HTML, link
  reference definitions, the setext underline, the `[!TYPE]` line of an
  alert, hard breaks (two trailing spaces or a trailing `\`), list markers,
  task-list checkboxes, and the blank lines between blocks untouched.
- The unwrap operates on a parse of the body, not on raw lines, so it
  joins exactly the lines GFM treats as paragraph continuation. A line
  that cannot interrupt a paragraph, such as `2. x`, `-x`, `#5`, or a
  `> x` indented by four or more spaces, is joined as text; a line that
  does interrupt one, such as `- x`, `1. x`, `# x`, or a `> x` indented
  by up to three spaces, starts its own block and is not joined.
- The parse recognises the GFM constructs GitHub renders in PR bodies,
  so alerts and footnotes are treated as such rather than as paragraph
  text and link reference definitions.
- The unwrap is idempotent.
- The unwrap is unconditional: it reads no repository configuration
  (`.editorconfig`, CLAUDE.md, `.accelerator/` skill instructions, a
  custom `pr-description` template) and offers no opt-out.
- `update-body` does not modify the body file; the artefact in `meta/prs/`
  continues to follow the repository's conventions.
- `describe-pr`'s instructions no longer claim the description shown to
  the user is exactly what gets posted; they say the posted body is the
  same text with soft line breaks unwrapped.

## Acceptance Criteria

The first three criteria exercise the unwrap through `update-body`, and
the fourth covers `describe-pr`'s instructions; the rest exercise the
unwrap directly.

- [ ] Given a body file containing
      `Wrapped\nparagraph  \nhard\nbreak\n\n```\ncode\nline\n```\n`,
      when `update-body` runs, then the PATCHed `body` is exactly
      `Wrapped paragraph  \nhard break\n\n```\ncode\nline\n```\n`.
- [ ] Given a body file, when `update-body` runs, then the file's bytes
      are unchanged afterwards.
- [ ] Given a repository whose `.editorconfig` sets `max_line_length`,
      whose CLAUDE.md sets a line-width rule, and whose `.accelerator/`
      config carries `describe-pr` instructions and a custom
      `pr-description` template, when `update-body` runs on a wrapped
      body, then the PATCHed `body` equals the one produced with none of
      them present, and `update-body --help` lists exactly the options
      it lists before this story.
- [ ] `skills/github/describe-pr/SKILL.md` no longer says the
      description shown to the user is exactly what gets posted, and
      says soft line breaks are unwrapped before posting.
- [ ] Given `> first line\n> second line`, when it is unwrapped, then the
      output is `> first line second line`.
- [ ] Given `- item\n  continued`, `- item\nlazy`, and
      `1. item\n   continued`, when each is unwrapped, then the outputs
      are `- item continued`, `- item lazy`, and `1. item continued`.
- [ ] Given `> - item\n>   continued`, `- a\n  - nested\n    more`,
      `> first\nsecond`, and `- [ ] task\n  wrapped`, when each is
      unwrapped, then the outputs are `> - item continued`,
      `- a\n  - nested more`, `> first second`, and
      `- [ ] task wrapped`.
- [ ] Given `foo\n2. bar`, `foo\n-bar`, `foo\n#5 bar`, and
      `foo\n    > bar`, when each is unwrapped, then the outputs are
      `foo 2. bar`, `foo -bar`, `foo #5 bar`, and `foo > bar`.
- [ ] Given `foo\n- bar`, `foo\n1. bar`, `foo\n# bar`, and
      `foo\n> bar`, when each is unwrapped, then each output equals its
      input.
- [ ] Given a body made only of a fenced code block, an indented code
      block, a table, an HTML block, a multi-line code span, a link
      reference definition, and lines ending in two spaces and in `\`,
      with no soft break outside them, when it is unwrapped, then the
      output equals the input byte-for-byte.
- [ ] Given the same constructs interleaved with wrapped paragraphs, when
      it is unwrapped, then the output equals the input with only those
      paragraphs' soft breaks replaced.
- [ ] Given `Some\nwrapped text\n| a | b |\n| - | - |`, when it is
      unwrapped, then the output is
      `Some wrapped text\n| a | b |\n| - | - |`, with the table intact;
      the expected rendering is confirmed once by hand against GitHub
      during the `meta/prs/` verification.
- [ ] Given `Release\nnotes\n=====`, when it is unwrapped, then the
      output is `Release notes\n=====`.
- [ ] Given `> [!NOTE]\n> wrapped\n> body`, when it is unwrapped, then
      the output is `> [!NOTE]\n> wrapped body` and still parses as a
      note alert.
- [ ] Given a footnote reference and a footnote definition whose text is
      wrapped across lines, when it is unwrapped, then the definition's
      text is on one line and still parses as a footnote definition.
- [ ] Given `foo \nbar` (one trailing space), when it is unwrapped, then
      the output is `foo bar`.
- [ ] Given `foo\t\t\nbar`, `foo \t\nbar`, and `foo  \t\nbar`, when
      each is unwrapped, then the output is `foo bar`; given
      `foo\t  \nbar`, when it is unwrapped, then the output equals the
      input.
- [ ] Given `foo <span\nclass="x">bar</span>`, when it is unwrapped,
      then the output equals the input.
- [ ] Given each fixture above with every `\n` replaced by `\r\n`, when
      it is unwrapped, then the output equals that fixture's expected
      output with every `\n` replaced by `\r\n`.
- [ ] Given each fixture above, when it is unwrapped, then the event
      stream that the unwrap's parser produces, with the GFM options in
      Technical Notes, is equal for output and input, where both streams
      are normalised by reading each soft break, and each hard break
      GitHub renders as soft, as a space and merging adjacent `Text`
      events; and unwrapping the output again returns it
      unchanged.
- [ ] Given each description in `meta/prs/` when implementation begins
      (115 at drafting time), with its frontmatter stripped as
      `describe-pr` strips it, when it is unwrapped in a one-off
      verification, then the event-stream and idempotence checks above
      hold, and GitHub's `POST /markdown` in `markdown` mode renders
      input and output to the same HTML once heading `id` and `href`
      anchors are removed and whitespace runs are collapsed everywhere
      except inside `<pre>` and `<code>`.
      - A failure of the event-stream or idempotence check is a
        span-logic failure: it is fixed in this story and becomes a
        named fixture.
      - A failure of the HTML check alone is a parser divergence: it
        becomes a follow-up work item.
      - The criterion passes when every description passes, or fails
        only through parser divergence that has a linked follow-up.
      - The description of the pull request implementing this story
        records the command used, the number checked, the fixture or
        follow-up each failure produced, and the outcome of the
        by-hand table check. No test reads `meta/prs/` directly.

## Open Questions

None.

## Dependencies

- Blocked by: none
- Blocks: none
- External: `pulldown-cmark` 0.11.0 or later (the first release with
  `ENABLE_GFM` alert parsing), a new `cli/` workspace dependency that must
  pass `cargo-deny`.
- External: GitHub's `POST /markdown` REST endpoint, for the one-off
  `meta/prs/` verification only; about 230 calls need an authenticated
  token, such as through `gh api`.
- External: GitHub's rendering of PR bodies (`cmark-gfm`, a single
  newline as a line break); the premise to re-check if GitHub changes it.
- Informs: 0320 — if 0320 adopts a push-side unwrap, it reuses the
  Markdown crate this story adds rather than adding another.

## Assumptions

- Authors never rely on a single newline for a visible line break in a
  posted description; lines meant to render separately, such as trailers,
  use a hard break or a list. If wrong, an opt-in to keep such breaks
  comes back into scope. A heuristic scan of the 115 descriptions in
  `meta/prs/` at drafting time found no counter-example.

## Technical Notes

- The PATCH lives in `cli/collaboration/src/update_body.rs`
  (`update_pull_request_body`); the CLI entry point is `run_update_body`
  in `cli/collaboration-cli/src/main.rs`, with arguments declared in
  `cli.rs`. End-to-end tests with a mock server live in
  `cli/collaboration-cli/tests/end_to_end.rs`; their PATCH route does
  not inspect the request today, but `MockHTTPServer::last_body`
  (`cli/http-test-support`) returns it for asserting the PATCHed
  `body`.
- `pulldown-cmark` fits a source-preserving rewrite: `into_offset_iter()`
  yields byte ranges per event and `Event::SoftBreak` is distinct. The
  options matching GitHub's `cmark-gfm` extensions are `ENABLE_TABLES`,
  `ENABLE_TASKLISTS`, `ENABLE_STRIKETHROUGH`, `ENABLE_FOOTNOTES`, and
  `ENABLE_GFM`; the last gates only alert tags (`[!NOTE]` and the
  rest), not tables or task lists. Without `ENABLE_GFM` an alert's
  marker line is joined into its body, which stops GitHub rendering the
  alert; without `ENABLE_FOOTNOTES` a footnote definition parses as a
  link reference definition and stays wrapped. Which bytes a SoftBreak
  range covers inside `>` and list containers is undocumented; pin it
  with a test first.
- `pulldown-cmark` reads any two or more trailing whitespace bytes,
  tabs included, as a hard break, where `cmark-gfm` needs two spaces or
  a `\`. The one-off `meta/prs/` verification's `foo\t\t\nbar` probe
  confirmed it: GitHub still renders that line break. The unwrap
  therefore decides hard breaks by GitHub's rule, not the parser's.
- `comrak` reports line/column positions only, documented as unreliable
  for inlines, and its CommonMark renderer re-serialises the whole
  document. `markdown-rs` has no soft-break node. Both fit worse.
- No Markdown crate is in the `cli/` workspace yet; adding one puts it
  through `cargo-deny`.
- The main residual risk is divergence between the Rust parser and
  GitHub's `cmark-gfm` on constructs such as tables interrupting a
  paragraph; the fixtures and the one-off `meta/prs/` verification cover
  the cases known today.
- Built-in template bodies stay wrapped at about 80 columns; all 19 are
  already consistent, so no template changes are needed.

## Drafting Notes

- The target is that no soft line break remains in a paragraph, list
  item, blockquote paragraph, setext heading, or footnote definition
  paragraph, not the removal of every newline; hard breaks, code spans,
  and inline HTML keep theirs.
- Setext headings are joined rather than preserved: GitHub renders a
  newline inside one as a line break, the raggedness this story removes.
- An alert's `[!TYPE]` line stays separate because GitHub only renders
  an alert when the marker sits on its own line.
- The `meta/prs/` corpus is a one-off verification, not a test, so the
  `cli/` suite stays hermetic and a new description cannot fail an
  unrelated build.
- Ignoring repository conventions applies to the posted body only; the
  artefact keeps honouring them.
- Built-in templates need no change, since they stay wrapped and only the
  push changes.
- Frontmatter is out of scope because `describe-pr` strips it before
  posting.
- The unwrap lives in `update-body`, unconditionally, rather than in a
  `describe-pr`-only step: unwrapped bodies are a property of pushing to
  GitHub.
- 0320 meets the same problem on the Linear push path and is handled
  separately; this story builds no shared module, but whichever lands
  second reuses or extracts the first's unwrap.
- The event-stream criterion uses the unwrap's own parser, so it cannot
  catch divergence from GitHub. GitHub's `POST /markdown` checks against
  GitHub's parser but needs network access, so it runs only in the
  one-off `meta/prs/` verification, not in the tests.
- Hard breaks are the escape hatch for an intended line break; no other
  is added.

## References

- Source: `skills/github/describe-pr/SKILL.md`
- Related: 0320 (Linear Push Corrupts Markdown Content)
- Related: 0197 (accelerator-collaboration: PR Helper CLI)
- Related: 0071 (byte-for-byte body contract this story replaces)
- Related: 0152 (Collaboration (PR) Skills — future `update-body` callers)
- GitHub Docs, [Basic writing and formatting syntax › Line breaks](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax)
- [CommonMark spec 0.31.2](https://spec.commonmark.org/0.31.2/)
