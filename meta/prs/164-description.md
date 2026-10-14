---
type: "pr-description"
id: "164"
title: "[0332] Post PR descriptions to GitHub with soft line breaks unwrapped"
date: "2026-10-14T15:06:25+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "work-item:0332"
parent: "work-item:0332"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/164"
pr_number: 164
tags: []
revision: "7b00ed03934d26287562fd2ed52ebb9fcbf85fac"
repository: "accelerator"
last_updated: "2026-10-14T15:10:24+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0332] Post PR descriptions to GitHub with soft line breaks unwrapped

## Summary

GitHub renders every newline in a pull request body as a line break, so
descriptions hard-wrapped to this repository's 80 columns show up ragged.
`accelerator collaboration pr update-body` now replaces each GFM soft line
break with a single space before PATCHing. The body file and the
`meta/prs/` artefact stay wrapped. This description is the first one posted
through the unwrap, by a dev build of the binary.

## Changes

- **New `markdown` technical library** (`cli/markdown`). Its one public
  function, `unwrap_soft_line_breaks(&str) -> String`, parses with
  `pulldown-cmark` 0.13.4 (exact pin, default features off). It replaces
  each soft-break span with a space and preserves every other byte. A span
  runs from the trailing whitespace before the line ending to the
  continuation line's content, past its `>` markers and indentation.
- **Whitespace breaks GitHub reads as soft are unwrapped too.**
  `pulldown-cmark` treats any two trailing whitespace bytes as a hard break.
  `cmark-gfm` needs two spaces or a `\`. So `foo\t\t\nbar` and
  `foo \t\nbar` are joined, while `foo\t  \nbar` stays a hard break.
- **`PullRequestBodyPublishing` port** in `collaboration`. The
  `update_pull_request_body` use case posts the body as prepared for
  publishing. `github`'s `GitHubPullRequestBodyPublishing` implements the
  port with the unwrap, and `accelerator-collaboration` wires it in.
- **Gates for the new crate:**
  - a crate-role declaration
  - a `denied`-only `cargo-pup` rule (no fs, process, transport or async)
    with a probe pair
  - a public-API pin
  - a `cargo-deny` ban on `pulldown-cmark` outside `markdown`
  - a third-party notices entry
- **Command-runner `pup` rule gap closed.** `markdown` and the previously
  missed `graphql_test_support` join the rule's crate alternation, and the
  `tasks/README.md` library checklist gains that step.
- **`accelerator-collaboration` CLI surface pinned** by a `--help` golden,
  confirming that `update-body`'s flags are unchanged.
- **Docs:** `describe-pr` and `docs-site/.../collaboration.md` say what is
  posted. `describe-pr` now lists an unreadable body file under exit 2,
  which is what the binary returns.

### Outside 0332's scope: `rustls` 0.23.46

`deny:check` began failing on RUSTSEC-2026-0346 to 0349, published against
`rustls` 0.23.45 partway through this work. The workspace's exact pin moves to
`=0.23.46` (`cargo update -p rustls --precise 0.23.46`, no other package
moves) in its own commit, with the notices regenerated. It changes the TLS
stack of every networked binary. Reverting it brings the advisories back.

## Context

- Work item: `meta/work/0332-push-pr-descriptions-to-github-unwrapped.md`
  (PP-885)
- Plan:
  `meta/plans/2026-10-13-0332-push-pr-descriptions-to-github-unwrapped.md`
- Research:
  `meta/research/codebase/2026-10-13-0332-push-pr-descriptions-to-github-unwrapped.md`

This replaces 0071's byte-for-byte contract: the posted body now differs from
the body file at soft line breaks only. 0320 (the Linear push path) needs
code-span newlines collapsed too, so it should add a dialect-aware entry point
to `markdown` rather than reuse this one.

### Corpus verification record

The unwrap ran over every `meta/prs/` description and a set of divergence
probes. Each result was checked against GitHub's own renderer
(`POST /markdown`).

- **Method:** a scratch verifier crate (not committed) stripped
  frontmatter, ran `unwrap_soft_line_breaks`, and checked idempotence and
  normalised event-stream equality. It used the normaliser and
  `github_extensions` copied verbatim from the crate. Each input and output
  was then rendered with `mode=markdown`, and each output again with
  `mode=gfm`.
  - **Structure check:** compares normalised input and output HTML.
  - **Residual-wrap check:** counts excess `<br` in the `gfm` render over
    the `markdown` render.
- **Calibration:**
  - Two renders each of the footnote and math probes normalise
    identically.
  - `foo\n- bar` against `foo - bar` fails the structure check, as
    required.
  - `a\nb` shows a residual-wrap excess of 1, and `a  \nb` an excess of 0.
- **Normaliser rules:** drop every `id`, `aria-labelledby` and `href="#…"`,
  drop any attribute that differs between two renders of one input, and
  collapse whitespace outside `<pre>` and `<code>`.
- **First run:** 115 descriptions and 7 probes (366 renderer calls). All 122
  pass event-stream equality, idempotence and the structure check. There
  were no span-logic failures and no structural divergences. The only
  residual wrap was the `foo\t\t\nbar` probe, the predicted tab hard-break
  divergence, which the whitespace-break change above then addressed.
- **Probe outcomes:**

  | Probe | Outcome | Fixture |
  |---|---|---|
  | table interrupting a paragraph | matches | `a_table_interrupting_a_paragraph_stays_intact` |
  | lowercase alert | matches | `a_lowercase_alert_keeps_its_marker_line` |
  | unknown alert kind | matches | `an_unknown_alert_kind_joins_as_quoted_text` |
  | non-pipe lines after a table | matches, stay rows | `lines_after_a_table_stay_its_rows` |
  | footnote definition's lazy continuation | matches | `a_footnote_definitions_lazy_continuation_joins` |
  | inline math | matches, `$…$` renders as text | `dollar_delimited_math_joins_as_text` |
  | two trailing tabs | diverged, then fixed | `whitespace_github_renders_as_soft_joins` |

- **Checked by hand:** the table probe renders, before and after, as the
  paragraph `Some wrapped text` followed by a table. The footnote probe's
  input render contains a `<section data-footnotes>`. The REST endpoint
  renders neither alert probe as an alert, so alert rendering on the PR page
  is unconfirmed. The unwrap keeps the marker line on its own line in both
  alert-kind cases.
- **Re-run after the whitespace-break change:** the same 115 descriptions,
  plus 15 probes (the original 7 and one per new fixture input), 390 calls.
  All 130 pass event-stream, idempotence and structure checks. None shows a
  residual wrap, `foo\t\t\nbar` included. `foo\t  \nbar` is left as written
  and still renders one `<br>`.

## Testing

- [x] `markdown` fixtures: every acceptance-criterion input, plus escapes,
      link-closing breaks, lazy lines, trailing whitespace, whitespace breaks
      GitHub reads as soft, multibyte text and probe-derived cases. Each runs
      under LF and CRLF with idempotence and event-stream checks, after a
      characterisation of `SoftBreak` and `HardBreak` offset ranges.
- [x] `collaboration`: the use case PATCHes the port's prepared body.
- [x] `github`: `GitHubPullRequestBodyPublishing` unwraps a blockquote
      continuation.
- [x] `accelerator-collaboration` end-to-end tests against a mock GitHub
      server:
  - the PATCHed body is unwrapped
  - the body file's bytes are unchanged
  - the PATCHed body is the same with and without a repository that
    configures 40-column wrapping (`.editorconfig`, `CLAUDE.md`,
    `.accelerator/` config, describe-pr instructions and template)
- [x] `cargo-pup` probe pair for `markdown_transforms_text_only`.
- [x] `mise run` exits 0 end-to-end, `docs:check` included.
- [ ] This body renders with its paragraphs at full width on GitHub, while
      `meta/prs/164-description.md` stays wrapped. Confirm on the PR page.

## Notes for Reviewers

- The span rule in `cli/markdown/src/lib.rs` rests on two subtle
  invariants:
  - `min(next event start, prefix scan)` stops deep indentation before a
    `>` being swallowed as a container prefix.
  - The fallback ignores an `End` event whose range starts back at its
    element's opening, as in `[a\n](u)`.

  The fixtures carry both, so neither needs a code comment.
- `prepare_for_publishing` returns `String`, not `Result`. A forge that must
  refuse a body would need a port change. That is deliberate while one forge
  exists.
- A line that must render on its own, such as a trailer or `Closes #n`, now
  needs a hard break (two trailing spaces or `\`) or a list item.
