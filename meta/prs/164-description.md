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
revision: "9d8b475ed08c3e0322aa35556af8557641868524"
repository: "accelerator"
last_updated: "2026-10-14T23:59:36+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0332] Post PR descriptions to GitHub with soft line breaks unwrapped

## Summary

GitHub renders every newline in a pull request body as a line break, so
descriptions hard-wrapped to this repository's 80 columns show up ragged.
`accelerator collaboration pr update-body` now replaces each line break
GitHub reads as soft with a single space before PATCHing. The body file and
the `meta/prs/` artefact stay wrapped. This description is posted through
the unwrap by a dev build of the binary.

## Changes

- **New `markdown` technical library** (`cli/markdown`), one module per
  dialect. Its public API is `markdown::github::unwrap_soft_line_breaks(&str)
  -> String`, which parses with `pulldown-cmark` 0.13.4 (exact pin, default
  features off). It replaces each soft-break span with a space and leaves
  every other byte as written. A span runs from the trailing whitespace
  before the line ending to the continuation line's content, past its `>`
  markers and indentation. That span machinery sits at the crate root,
  ready for 0320's Linear dialect to share.
- **Whitespace breaks GitHub reads as soft are unwrapped too.**
  `pulldown-cmark` treats any two trailing whitespace bytes as a hard break.
  `cmark-gfm` needs two spaces or a `\`. So `foo\t\t\nbar` and
  `foo \t\nbar` are joined, while `foo\t  \nbar` stays a hard break.
- **Math is posted as written.** GitHub renders `$…$` and `$$…$$` as math
  on PR pages, so the parser enables math too. A math span carries no soft
  break, so a `%` comment in display math cannot swallow joined lines.
- **A `PullRequestForge` port bundle** in `collaboration`. A new
  `PullRequestBodyPublishing` port prepares a body for the forge that renders
  it. `PullRequestForge` is a supertrait over it and the lookup, existence
  and body-update ports. `update_pull_request_body(root, origin, recognizer,
  forge, pull_number, body)` takes one forge value, so a body is always
  prepared for the forge that receives it, and its `too_many_arguments`
  allow is gone. `github`'s `GitHubPullRequestBodyPublishing` implements the
  unwrap, and `accelerator-collaboration`'s `BlockingGitHubClient` delegates
  to it. Both use cases now name the existence port `existence`.
- **Gates for the new crate:**
  - a crate-role declaration
  - a `denied`-only `cargo-pup` rule (no fs, process, transport or async)
    with a probe pair
  - a public-API pin
  - a `cargo-deny` ban on `pulldown-cmark` outside `markdown`
  - a third-party notices entry
- **The command-runner `pup` rule now fails closed.** `markdown` and the
  previously missed `graphql_test_support` join the rule's crate
  alternation, and the `tasks/README.md` library checklist gains that step.
  A new test fails when any production lib or bin target other than
  `config` and `config_adapters` falls outside the alternation.
- **`accelerator-collaboration` CLI surface pinned** by a `--help` golden,
  confirming that `update-body`'s flags are unchanged.
- **Docs:** `describe-pr` and `docs-site/.../collaboration.md` say what is
  posted, and the CHANGELOG files the change under Breaking. `describe-pr`
  now lists an unreadable body file under exit 2, which is what the binary
  returns.

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
- Validation (pass):
  `meta/validations/2026-10-13-0332-push-pr-descriptions-to-github-unwrapped-validation.md`
- Review: `meta/reviews/prs/164-review-1.md`, all ten threads addressed

This replaces 0071's byte-for-byte contract: the posted body now differs from
the body file at soft line breaks only. 0320 (the Linear push path) needs
code-span newlines collapsed too, so it should add a `markdown::linear`
module beside `markdown::github` rather than reuse this one.

### Corpus verification record

The unwrap ran over every `meta/prs/` description and a set of divergence
probes. Each result was checked against GitHub's own renderer
(`POST /markdown`).

- **Method:** a scratch verifier crate (not committed) stripped
  frontmatter, ran the unwrap, and checked idempotence and normalised
  event-stream equality. It used the normaliser and the extension set
  copied verbatim from the crate. Each input and output was then rendered
  with `mode=markdown`, and each output again with `mode=gfm`.
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
  | inline math | REST renders `$…$` as text; superseded by math passthrough | `inline_math_keeps_its_line_break` |
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
- **Math passthrough check:** the REST endpoint does not render math, so it
  cannot judge this change. Instead, the unwrap ran offline over all 117
  current descriptions with math parsing on and off. The outputs are
  identical, including the 23 descriptions that contain `$`.

## Testing

- [x] `markdown` fixtures: every acceptance-criterion input, plus escapes,
      link-closing breaks, lazy lines, trailing whitespace, whitespace breaks
      GitHub reads as soft, multibyte text, display and inline math, dollar
      amounts and probe-derived cases. Each runs under LF and CRLF with
      idempotence and event-stream checks, after a characterisation of
      `SoftBreak` and `HardBreak` offset ranges.
- [x] `collaboration`: the use case PATCHes the body as the forge prepared
      it, through a fake forge that composes per-port fakes.
- [x] `github`: `GitHubPullRequestBodyPublishing` unwraps a blockquote
      continuation.
- [x] `accelerator-collaboration` end-to-end tests against a mock GitHub
      server:
  - the PATCHed body is unwrapped
  - the body file's bytes are unchanged
  - the PATCHed body is the same with and without a repository that
    configures 40-column wrapping (`.editorconfig`, `CLAUDE.md`,
    `.accelerator/` config, describe-pr instructions and template)
- [x] `cargo-pup` probe pair for `markdown_transforms_text_only`, and the
      command-runner coverage test, shown to fail with `markdown` removed
      from the alternation.
- [x] `mise run` exits 0 end-to-end, `docs:check` included.
- [ ] This body renders with its paragraphs at full width on GitHub, while
      `meta/prs/164-description.md` stays wrapped. The posted body was
      fetched back, but the rendered page still needs checking by eye.
- [ ] Math renders on the PR page after the unwrap. The REST endpoint cannot
      show it, so this needs a `$$` block checked on a real PR.

## Notes for Reviewers

- The span rule in `continuation_start` (`cli/markdown/src/lib.rs`) rests on
  two subtle invariants:
  - `min(next event start, prefix scan)` stops deep indentation before a
    `>` being swallowed as a container prefix.
  - The fallback ignores an `End` event whose range starts back at its
    element's opening, as in `[a\n](u)`.

  The fixtures carry both, so neither needs a code comment.
- The event-stream check in the `markdown` tests uses the production
  `renders_as_soft` on purpose. It asks whether anything besides
  GitHub-soft breaks changed. Every branch of the classifier is pinned
  separately by exact-output fixtures.
- `prepare_for_publishing` returns `String`, not `Result`. A forge that must
  refuse a body would need a port change. That is deliberate while one forge
  exists.
- The command-runner coverage test ignores binaries under `tests/fixtures/`.
  Seven fixture binaries sit outside the rule today.
- A line that must render on its own, such as a trailer or `Closes #n`, now
  needs a hard break (two trailing spaces or `\`) or a list item.
