---
type: "plan"
id: "2026-10-13-0332-push-pr-descriptions-to-github-unwrapped"
title: "Push PR Descriptions to GitHub Unwrapped Implementation Plan"
date: "2026-10-13T22:25:25+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "in-progress"
work_item_id: "work-item:0332"
parent: "work-item:0332"
derived_from: ["codebase-research:2026-10-13-0332-push-pr-descriptions-to-github-unwrapped"]
tags: ["collaboration", "github", "markdown", "pulldown-cmark", "describe-pr"]
revision: "4f52d32c20ed096f05b4e9ef2a0f830a8373d3cc"
repository: "accelerator"
last_updated: "2026-10-14T11:53:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Push PR Descriptions to GitHub Unwrapped Implementation Plan

## Overview

`accelerator collaboration pr update-body` PATCHes the body file verbatim, and
GitHub renders every single newline in a PR body as a line break. This plan
does three things:

- It adds a `markdown` technical-library crate whose one public function
  unwraps GFM soft line breaks on a `pulldown-cmark` parse.
- It verifies that function once against the 115 descriptions in
  `meta/prs/`.
- It makes the `collaboration` use case publish every body through a new
  intent-named port, `PullRequestBodyPublishing`. The `github` adapter
  implements that port with the unwrap.

The body file and the `meta/prs/` artefact stay wrapped.

## Current State Analysis

The body travels as a plain `&str` and is never transformed:

```text
run_update_body (collaboration-cli/src/main.rs:215-251)
  │ std::fs::read_to_string(body_file)         -- exit 2 on failure
  ▼
update_pull_request_body (collaboration/src/update_body.rs:31-60)
  │ resolve_base_repository → body_update.update_body(owner, repo, n, body)
  ▼
OctocrabClient::update_body (github/src/octocrab_client.rs:136-151)
  │ pulls().update(n).body(body).send()
  ▼
PATCH /repos/{owner}/{repo}/pulls/{n}  {"body": …}
```

- No Markdown parser exists in the `cli/` workspace. Existing Markdown
  handling is line-based and not reusable (`cli/document/src/fence.rs`,
  `cli/work/src/section_diff.rs`, `cli/jira-client/src/adf/`).
- `collaboration` is a domain crate that depends only on `kernel`. ADR-0069
  rule 1 lets it reach a technical library only through a port of its own.
- The end-to-end test `update_body_succeeds`
  (`cli/collaboration-cli/tests/end_to_end.rs:185-233`) never inspects the
  PATCH payload. `MockHTTPServer::last_body`
  (`cli/http-test-support/src/lib.rs:234-239`) returns it.
- `skills/github/describe-pr/SKILL.md:145-146` says the user will "see what
  gets posted to GitHub". SKILL.md:180-181 wrongly lists a body-file read
  failure under exit 1, when the code exits 2 (`Refusal`).

## Desired End State

- `update-body` PATCHes the body with every GFM soft line break replaced by
  a single space. Every byte outside a replaced span is preserved.
- The body file's bytes are unchanged.
- The behaviour reads no repository configuration, and `update-body --help`
  is unchanged.
- `markdown::unwrap_soft_line_breaks` passes every acceptance-criterion
  fixture under LF and CRLF, is idempotent, and preserves the normalised
  event stream.
- The one-off corpus verification has run over every `meta/prs/`
  description, and its outcome is recorded in this plan's "Corpus
  verification record" section for the PR description.
- `describe-pr` and `docs-site/src/content/docs/collaboration.md` say that
  the posted body is the shown text with soft line breaks unwrapped.
- `mise run` exits 0.

### Key Discoveries:

- `pulldown-cmark` 0.13.4 with `default-features = false` needs only
  `bitflags` 2, `memchr` and `unicase`. All three are already in
  `cli/Cargo.lock` (`bitflags` 2.13.1, `memchr` 2.8.3, `unicase` 2.9.0), so
  the only new package is `pulldown-cmark`. `pulldown-cmark-escape`, which
  the research lists as new, is optional behind the `html` feature and stays
  out of the lockfile with default features off.
- A `SoftBreak` range covers the line ending alone (`\n` or `\r\n`). It
  excludes the trailing whitespace before it and the next line's container
  prefix. The replaced span is
  `[line ending start − trailing whitespace, min(next event start, end of a
  forward scan over '>', ' ', '\t'))`. Trailing whitespace is the parser's
  own class: space, tab, VT and FF. Using the next event's start alone
  breaks `foo\n\*bar`. Using the scan alone breaks `foo\n    > bar`.
- An `End` event's range is its whole element's, so it starts at the
  element's opening. In `[a\n](u)` the event after the `SoftBreak` (`2..3`)
  is `End(Link)` at `0..7`. The next event bounds the span only when it
  starts on the continuation line; otherwise the scan alone does. Without
  that, the span is reversed and the output duplicates the input.
- `pulldown-cmark` makes any two or more trailing whitespace bytes a hard
  break, tabs included, where `cmark-gfm` needs two spaces. A line ending in
  `\t\t` is therefore left wrapped while GitHub still renders a break. Phase
  2's residual-wrap check and `foo\t\t\nbar` probe confirm and record it.
  The same backward scan crosses a continuation line's stripped leading
  space, so a line holding only VT or FF (`foo\n \x0B\nbar`) becomes a
  hard break starting at the line start and fails event-stream equality.
  VT and FF do not occur in PR descriptions; if a corpus or probe input
  hits this shape, it is a parser quirk, not a span-logic failure.
- These constructs emit no `SoftBreak`, so the rewrite leaves them alone
  without any special case:
  - code spans, inline HTML and hard breaks
  - the alert marker line and the setext underline
  - link reference definitions
  - indented and fenced code, and HTML blocks
- A technical-library crate owes six obligations (`tasks/README.md:790-854`).
  `remote-projection` is the precedent for its `denied`-only pup rule
  (`cli/pup.ron:203-221`) and probe pair
  (`tests/integration/pup/test_import_rule.py:916-993`).
- `cli/pup.ron:443` enumerates the crates the command-runner rule covers in
  one regex: every crate except `config` and `config_adapters`, which it
  deliberately omits, and `graphql_test_support`, which it misses. The
  README checklist does not mention it, so Phase 1 adds the step and closes
  the gap.
- Workspace dependencies carry a rationale comment and an explicit pin
  policy. `similar = "=2.7.0"` (`cli/Cargo.toml:92-96`) is the exact-pin
  precedent for output that is golden-tested.

## What We're NOT Doing

- No opt-out flag, no configuration key, and no reading of `.editorconfig`,
  CLAUDE.md or `.accelerator/` by the unwrap.
- No changes to built-in templates, the `meta/prs/` artefacts or
  frontmatter handling.
- No unwrap on `review-pr` or `respond-to-pr` payloads.
- No shared module for 0320. `unwrap_soft_line_breaks` is fixed to
  GitHub's dialect and deliberately leaves code-span newlines alone, which
  0320's Linear defects need collapsed. 0320 should expect to add a
  dialect-aware entry point to this crate rather than reuse this one as is.
- No fallible preparation. `prepare_for_publishing` returns `String`, so a
  forge that must refuse a body, such as one over a size limit, needs a
  port change: a `Result`, a new `UpdateBodyOutcome` variant and a pinned
  public-API diff. That is deliberate while one forge exists.
- No regrouping of `update_pull_request_body`'s forge ports into one
  parameter. The use case grows to nine parameters under its existing
  `clippy::too_many_arguments` allow; grouping is a separate refactor.
- No committed corpus tooling and no test that reads `meta/prs/`.
- No editing of the generated
  `docs-site/src/content/docs/reference/skills/github/describe-pr.md`.

## Implementation Approach

The work runs in three phases:

1. Build the mechanism as a pure, pinned technical library first, test-first
   from the acceptance-criterion fixtures.
2. Harden it against the real corpus before anything posts through it.
3. Make the behaviour visible in the domain. The `collaboration` use case
   says a body is prepared for publishing. The GitHub adapter knows that
   preparing for GitHub means unwrapping soft line breaks.

Each phase merges on its own. Phase 1 adds an unused crate, phase 2 adds
only fixtures and any span-rule fix they demand, and phase 3 switches the
behaviour on.

## Phase 1: The `markdown` technical library

### Overview

Add `cli/markdown` with one public function,
`unwrap_soft_line_breaks(&str) -> String`, and register it with every
workspace gate. Nothing consumes it yet.

### Changes Required:

#### 1. Workspace dependency

**File**: `cli/Cargo.toml`
**Changes**: Add `"markdown"` to `[workspace].members`. Add
`pulldown-cmark` under `[workspace.dependencies]`:

```toml
# CommonMark/GFM parser behind the soft-line-break unwrap. Default features
# off: `getopts` serves only its binary and `html` its renderer. Exact-pinned
# because which bytes a SoftBreak's offset range covers is observed
# behaviour, not documented contract, so a release must arrive as a
# deliberate re-run of the fixtures. `markdown` is the sole permitted wrapper
# (see deny.toml).
pulldown-cmark = { version = "=0.13.4", default-features = false }
```

Sync the lockfile with
`cargo metadata --manifest-path cli/Cargo.toml --format-version 1`, never
`cargo generate-lockfile`.

#### 2. Crate manifest

**File**: `cli/markdown/Cargo.toml` (new)

```toml
[package]
name = "markdown"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[package.metadata.accelerator]
role = "technical-library"

[lints]
workspace = true

[dependencies]
pulldown-cmark = { workspace = true }
```

#### 3. Tests first

**File**: `cli/markdown/src/tests.rs` (new; `#[cfg(test)] mod tests;` in
`lib.rs`, so the tests, as a child module, share the private parser
options)

Write these tests in red-green order. Each step adds the minimum production
code that turns it green.

1. **Characterisation of the parser.** Write
   `a_soft_break_covers_only_its_line_ending` first. It asserts the observed
   `SoftBreak` ranges for `foo \nbar` (`4..5`), `> first line\n> second line`
   (`12..13`) and `> a  \n>   b\r\n>\t c` (`11..13`). It also asserts that
   the `End(Link)` following the `SoftBreak` in `[a\n](u)` starts at `0`. It
   pins the parser behaviour the span rule depends on, so a bump that
   changes it fails here first, with a precise message.
2. **A shared assertion.** `assert_unwraps(input, expected)` checks four
   things:
   - `unwrap_soft_line_breaks(input) == expected`
   - the unwrap is idempotent on `expected`
   - the normalised event streams of `input` and `expected` are equal
   - the same three hold for the CRLF variants, made by replacing every
     `\n` with `\r\n`

   Normalisation reads each `SoftBreak` as `Text(" ")` and merges adjacent
   `Text` events into owned `CowStr`s. `assert_unchanged(input)` is
   `assert_unwraps(input, input)`. Both are `#[track_caller]`, and each
   assertion's message names the check, the line-ending variant and the
   input, so a failure points at the fixture and says which check broke.
3. **One test per acceptance-criterion fixture**, named for the behaviour:

| Test | Input → expected |
|---|---|
| `a_wrapped_paragraph_joins_but_a_hard_break_and_code_survive` | `Wrapped\nparagraph  \nhard\nbreak\n\n```\ncode\nline\n```\n` → `Wrapped paragraph  \nhard break\n\n```\ncode\nline\n```\n` |
| `a_blockquote_continuation_drops_its_marker` | `> first line\n> second line` → `> first line second line` |
| `list_item_continuations_join` | `- item\n  continued` → `- item continued`; `- item\n\tcontinued` → `- item continued`; `- item\nlazy` → `- item lazy`; `1. item\n   continued` → `1. item continued` |
| `nested_containers_join_inside_their_own_item` | `> - item\n>   continued` → `> - item continued`; `- a\n  - nested\n    more` → `- a\n  - nested more`; `> first\nsecond` → `> first second`; `- [x] task\n  wrapped` → `- [x] task wrapped` |
| `a_line_that_cannot_interrupt_a_paragraph_joins_as_text` | `foo\n2. bar` → `foo 2. bar`; `foo\n-bar` → `foo -bar`; `foo\n#5 bar` → `foo #5 bar` |
| `a_marker_after_deep_indentation_stays_literal_text` | `foo\n    > bar` → `foo > bar` |
| `a_line_that_interrupts_a_paragraph_starts_its_own_block` | unchanged: `foo\n- bar`, `foo\n1. bar`, `foo\n# bar`, `foo\n> bar` |
| `constructs_without_soft_breaks_are_byte_for_byte_unchanged` | unchanged: fenced code, indented code, a table, an HTML block, a multi-line code span, a link reference definition, and lines ending in two spaces and in `\` |
| `only_paragraph_soft_breaks_change_among_preserved_constructs` | the same constructs interleaved with wrapped paragraphs → only the paragraphs joined |
| `a_table_interrupting_a_paragraph_stays_intact` | `Some\nwrapped text\n| a | b |\n| - | - |` → `Some wrapped text\n| a | b |\n| - | - |` |
| `a_setext_heading_joins_above_its_underline` | `Release\nnotes\n=====` → `Release notes\n=====` |
| `an_alert_keeps_its_marker_line` | `> [!NOTE]\n> wrapped\n> body` → `> [!NOTE]\n> wrapped body`, and the output parses as `BlockQuote(Some(BlockQuoteKind::Note))` |
| `a_footnote_definition_joins_and_stays_a_definition` | `Text[^1].\n\n[^1]: a wrapped\n    definition` → `Text[^1].\n\n[^1]: a wrapped definition`, and the output still emits `Start(FootnoteDefinition("1"))` |
| `trailing_whitespace_before_a_soft_break_is_absorbed` | `foo \nbar`, `foo\t\nbar`, `foo\x0B\nbar` and `foo\x0C\nbar` → `foo bar` |
| `two_trailing_tabs_stay_a_hard_break` | unchanged: `foo\t\t\nbar` |
| `every_line_of_a_long_paragraph_joins` | `one\ntwo\nthree\nfour` → `one two three four`; `> one\n> two\n> three` → `> one two three` |
| `a_soft_break_beside_multibyte_text_joins` | `café —\nnaïve` → `café — naïve`; `> café —\n> naïve` → `> café — naïve` |
| `inline_html_keeps_its_newline` | unchanged: `foo <span\nclass="x">bar</span>` |
| `an_escape_at_line_start_survives` | `foo\n\*bar` → `foo \*bar` |
| `a_soft_break_closing_a_link_joins_inside_its_text` | `[a\n](u)` → `[a ](u)`; `![a\n](u)` → `![a ](u)`; `- [a\n  ](u)` → `- [a ](u)`; `[a\n][r]\n\n[r]: u` → `[a ][r]\n\n[r]: u` |
| `indented_lazy_and_inline_continuations_join` | `foo\n   bar` → `foo bar`; `[a\nb](u)` → `[a b](u)`; `~~a\nb~~` → `~~a b~~`; `> a\n  b` → `> a b` |

#### 4. Implementation

**File**: `cli/markdown/src/lib.rs` (new)

```rust
//! Text transformations for GitHub-flavoured Markdown that preserve every
//! byte they do not deliberately change.

use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser};

#[cfg(test)]
mod tests;

/// Replaces each GFM soft line break with a single space, leaving every
/// other byte as written.
///
/// A replaced span runs from any trailing whitespace before the line ending
/// to the start of the continuation line's content, past its `>` markers and
/// indentation.
#[must_use]
pub fn unwrap_soft_line_breaks(markdown: &str) -> String {
    let mut unwrapped = String::with_capacity(markdown.len());
    let mut preserved_from = 0;
    for span in soft_line_break_spans(markdown) {
        unwrapped.push_str(&markdown[preserved_from..span.start]);
        unwrapped.push(' ');
        preserved_from = span.end;
    }
    unwrapped.push_str(&markdown[preserved_from..]);
    unwrapped
}

fn github_extensions() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_GFM
}

fn soft_line_break_spans(markdown: &str) -> Vec<Range<usize>> {
    let events: Vec<_> = Parser::new_ext(markdown, github_extensions())
        .into_offset_iter()
        .collect();
    events
        .windows(2)
        .filter_map(|pair| match pair {
            [(Event::SoftBreak, line_ending), (_, continuation)] => Some(
                trailing_whitespace_start(markdown, line_ending.start)
                    ..continuation_start(
                        markdown,
                        line_ending.end,
                        continuation.start,
                    ),
            ),
            _ => None,
        })
        .collect()
}

fn trailing_whitespace_start(markdown: &str, line_ending: usize) -> usize {
    line_ending
        - markdown[..line_ending]
            .bytes()
            .rev()
            .take_while(|byte| {
                matches!(byte, b' ' | b'\t' | b'\x0B' | b'\x0C')
            })
            .count()
}

fn continuation_start(
    markdown: &str,
    line_start: usize,
    next_event_start: usize,
) -> usize {
    let container_prefix_end = line_start
        + markdown[line_start..]
            .bytes()
            .take_while(|byte| matches!(byte, b'>' | b' ' | b'\t'))
            .count();
    let next_event_is_on_continuation_line = next_event_start >= line_start;
    if next_event_is_on_continuation_line {
        container_prefix_end.min(next_event_start)
    } else {
        container_prefix_end
    }
}
```

Two subtle invariants live here. The `min` stops the prefix scan swallowing
literal `>` text after deep indentation, which
`a_marker_after_deep_indentation_stays_literal_text` carries. The
fallback ignores an `End` event whose range starts back at its element's
opening, which `a_soft_break_closing_a_link_joins_inside_its_text` carries.
Falling back to the scan rather than clamping to `line_start` keeps a list
item's indentation out of the link text. The fixtures carry both, so the
code needs no comment.

#### 5. Registrations

- **`tests/unit/tasks/test_crate_dependencies.py`.** Add
  `"markdown": ("technical-library", None, None, None),` to
  `_EXPECTED_DECLARATIONS`.
- **`cli/pup.ron`.** Add a `denied`-only rule beside
  `remote_projection_extracts_json_only`:

  ```ron
  // markdown transforms the text it is handed: no filesystem, no process
  // spawning, no transport or async runtime. `denied` for the
  // grouped-import reason recorded on the config-command rule below.
  Module((
      name: "markdown_transforms_text_only",
      matches: Module("^markdown($|::)"),
      rules: [
          RestrictImports(
              allowed_only: None,
              denied: Some([
                  "^std::process(::|$)",
                  "^std::fs(::|$)",
                  "^reqwest(::|$)",
                  "^hyper(::|$)",
                  "^tokio(::|$)",
              ]),
              severity: Error,
          ),
      ],
  )),
  ```

  Also add `markdown` and `graphql_test_support` to the crate alternation
  at `cli/pup.ron:443`. `graphql_test_support` is the existing gap that
  makes the comment above the rule, "it lists every crate but config and
  config_adapters", false.
- **`tests/integration/pup/test_import_rule.py`.** Add
  `test_markdown_rule_rejects_spawning`, driving
  `_write_shared_crate_probe(tmp_path, "markdown", _SPAWN_VIOLATION)` and
  asserting `"is denied"` and `"markdown_transforms_text_only"`. Add
  `test_markdown_rule_permits_text_transformation` with a compliant body
  that imports `std::ops::Range`. Generalise the comment above
  `_SHARED_CRATE_WORKSPACE`, which names only the remote-projection and
  tracker-support rules, so it lists no crates and claims only the rules
  that use the helper:

  ```python
  # Shared by the whole-crate `denied`-only rules whose probes need only a
  # spawn or transport violation. Each compliant control imports something
  # real, so a matcher that resolved nothing could not pass it silently.
  ```
- **`tasks/README.md`.** In "Registering a library crate", change "owes six
  things" to "owes seven things" and add after the `cli/pup.ron` rule
  bullet:

  ```markdown
  - **The command-runner alternation.** Add the crate to the module regex of
    `only_the_consent_policy_names_the_command_runner` in `cli/pup.ron`. The
    rule matches crates by name, so one missing from the alternation may
    name the production command runner unchecked.
  ```
- **`tasks/public_api.py`.** Add `"markdown"` to `_PINNED_CRATES` in
  alphabetical order, then generate
  `cli/markdown/tests/fixtures/public-api.txt` with
  `mise run public-api:update`. Read the diff: the surface must be exactly
  `pub fn markdown::unwrap_soft_line_breaks(&str) -> String`, with no
  `pulldown_cmark` type.
- **Supply chain.** In `cli/deny.toml`, add
  `{ crate = "pulldown-cmark", wrappers = ["markdown"] }` to `[bans].deny`
  beside the `serde-saphyr` entry, and extend the comment above the list:

  ```toml
  # infra-out-of-domain ban: the YAML crate is reachable only through the
  # document wrapper, so a direct domain import violates. pulldown-cmark is
  # banned the same way outside markdown, whose exact pin and
  # characterisation tests carry the soft-line-break span rule.
  ```

  Write the ban red first: a throwaway `pulldown-cmark` dependency on
  `collaboration` must fail `mise run deny:check`; remove it again. Then run
  `mise run deny:check`. No new `skip` entry is expected, because
  `bitflags` 2 is already the 2.x line. Run
  `mise run notices:check`. cargo-about reads the whole workspace graph,
  not only linked binaries, so the notices drift as soon as the crate
  exists: run `mise run notices:update` and read the diff, which adds only
  the `pulldown-cmark` entry under MIT.

### Success Criteria:

#### Automated Verification:

- [x] Crate tests pass:
      `cargo test --manifest-path cli/Cargo.toml -p markdown`
- [x] Workspace lint passes: `mise run cli:check`
- [x] Role declarations pass: `mise run lint:crate-dependencies:check` and
      `mise run test:unit:tasks`
- [x] Architecture rules pass: `mise run pup:check` and
      `mise run test:integration:pup`
- [x] Public API pin passes: `mise run public-api:check`
- [x] Supply chain passes: `mise run deny:check`
- [x] Notices are current: `mise run notices:check`
- [x] Full local CI passes: `mise run`

#### Manual Verification:

- [x] `cli/markdown/tests/fixtures/public-api.txt` names only
      `unwrap_soft_line_breaks`
- [x] No comment in `cli/markdown/src/` restates what the code says

---

## Phase 2: One-off corpus verification

### Overview

Run the unwrap over every description in `meta/prs/` and a set of
divergence probes from a throwaway scratch crate, and check each result
against GitHub's own renderer. Span-logic failures and structural
divergences become fixtures and block Phase 3; residual wraps become
follow-ups. Nothing in this phase is committed except the new fixtures,
any span-rule fix they demand, and the record below.

### Changes Required:

#### 1. Scratch verifier (not committed)

**Location**: `{scratchpad}/corpus-verify/`, a binary crate with
`markdown = { path = "<repo>/cli/markdown" }` and
`pulldown-cmark = { version = "=0.13.4", default-features = false }`.

For each `meta/prs/*-description.md` passed as an argument, the verifier:

1. Strips frontmatter the way `describe-pr` Step 9 does. If line 1 is
   `---`, it drops everything up to and including the next `---` line.
2. Runs `unwrap_soft_line_breaks`.
3. Checks idempotence and normalised event-stream equality. It copies the
   normaliser from `cli/markdown/src/tests.rs` and `github_extensions` from
   `cli/markdown/src/lib.rs` verbatim, so a corpus pass means what a fixture
   pass means.
4. Writes `in/<n>.md` and `out/<n>.md`, and prints one line per file:
   `PASS`, or `FAIL <check>` with the first differing event.

#### 2. GitHub rendering comparison

`pulldown-cmark` decides which newlines are soft breaks, but GitHub renders
the body with `cmark-gfm`. Two checks against GitHub's renderer catch the
two directions in which they can disagree. Both use the user's
authenticated `gh`, about 345 calls in all.

- **Structure check.** Render each input and output with
  `gh api -X POST /markdown -f mode=markdown -F text=@<file>`. Compare the
  HTML after a normaliser run with `python3 -I`, which:
  - removes every `id` attribute and `href="#…"` anchor
  - removes any further attribute that differs between two renders of the
    same input
  - collapses whitespace runs everywhere except inside `<pre>` and `<code>`

  A difference means the unwrap changed how GitHub parses the block
  structure.
- **Residual-wrap check.** Render each output again with `mode=gfm`, and
  count `<br>` elements in it and in its `mode=markdown` render.
  `markdown` mode renders only hard breaks as `<br>`, so any excess in
  `gfm` mode is a soft break GitHub still sees that `pulldown-cmark` did
  not. Count `<br` tag openings, so every serialisation matches.

Calibrate both checks before the corpus run, and stop if any control
misbehaves:

- **Normaliser stability.** Render the footnote and math probes twice each.
  The two normalised renders must be identical; extend the normaliser until
  they are.
- **Structure positive control.** The pair `foo\n- bar` against
  `foo - bar` must fail the structure check.
- **Residual-wrap controls.** `a\nb` must show an excess of exactly 1, and
  `a  \nb` an excess of 0.

#### 3. Divergence probes

Run these hand-written inputs through the verifier and both GitHub checks,
alongside the corpus:

| Probe | Construct |
|---|---|
| `Some\nwrapped text\n\| a \| b \|\n\| - \| - \|` | a table interrupting a paragraph |
| `> [!note]\n> a\n> b` | a lowercase alert marker |
| `> [!UNKNOWN]\n> a\n> b` | an unknown alert kind |
| `\| a \|\n\| - \|\nrow\nmore` | non-pipe lines after a table |
| `x[^1]\n\n[^1]: a\nb` | a referenced footnote definition's lazy continuation |
| `$a\nb$` | inline math |
| `foo\t\t\nbar` | two trailing tabs, a hard break only to `pulldown-cmark` |

Confirm by hand that the table probe renders, before and after, as a table
preceded by the paragraph `Some wrapped text`, and that the footnote probe's
input render contains a footnote section. Each probe whose GitHub
render matches the unwrap's intent becomes a named fixture in
`cli/markdown/src/tests.rs`. Each that diverges is triaged below.

#### 4. Triage

| Failure | Classification | Action |
|---|---|---|
| Event-stream or idempotence | span-logic | Add the minimal reproducing input to `cli/markdown/src/tests.rs` as a named fixture, red first, and fix the span rule |
| Structure check | structural divergence | Blocks Phase 3. Add the minimal reproducing input as a fixture, red first, then fix the span rule or leave that construct's soft breaks as written |
| Residual-wrap check alone | residual wrap | Create a follow-up work item with `/accelerator:create-work-item`, linked `relates_to: ["work-item:0332"]`. It renders no worse than today |

#### 5. Record

Fill in the "Corpus verification record" section at the end of this plan:

- the exact commands run
- the number of descriptions and probes checked
- each failure, with its fixture name or follow-up work-item ID
- the calibration controls' outcomes and the normaliser's rules
- each probe's outcome, including the tab hard-break divergence
- the outcome of the by-hand table and footnote checks

Phase 3's manual verification confirms the record reaches the PR
description.

### Success Criteria:

#### Automated Verification:

- [x] Any fixtures added pass:
      `cargo test --manifest-path cli/Cargo.toml -p markdown`
- [x] Full local CI passes: `mise run`

#### Manual Verification:

- [x] Every calibration control behaves as specified before the corpus
      run
- [x] Every description and probe passes the event-stream and idempotence
      checks
- [x] Every description and probe passes the structure check
- [ ] Every residual-wrap failure has a linked follow-up work item
- [x] The table renders as a table after the paragraph on GitHub
- [x] The "Corpus verification record" is complete

---

## Phase 3: Publishing bodies unwrapped

### Overview

Introduce the `PullRequestBodyPublishing` port in `collaboration`, implement
it in `github` with the unwrap, and wire it in at the composition root. Then
update the skill and the docs.

### Changes Required:

#### 1. Domain port, test first

**File**: `cli/collaboration/src/update_body.rs` (tests)
**Changes**:

- Add a `RecordingBodyUpdate` fake that stores the body it receives in a
  `RefCell<Option<String>>`.
- Add a `MarkedPublishing` fake whose `prepare_for_publishing` returns
  `format!("prepared:{body}")`.
- Write `posts_the_body_as_prepared_for_publishing`. It asserts that the
  recorded body is `prepared:new body` and that the outcome is `Updated`.
- Split the existing `update(...)` test helper. A new
  `update_published_by(publishing, …)` takes the publishing port, and
  `update(...)` delegates to it with an identity `UnchangedPublishing`
  fake, so the other tests keep their meaning and the new test goes
  through the helper rather than repeating the nine-argument call.

**File**: `cli/collaboration/src/lib.rs`
**Changes**: Add the port beside `PullRequestBodyUpdate`:

```rust
/// Prepares a pull request body for the forge that will render it. Compose
/// it only with that same forge's [`PullRequestBodyUpdate`].
pub trait PullRequestBodyPublishing {
    fn prepare_for_publishing(&self, body: &str) -> String;
}
```

**File**: `cli/collaboration/src/update_body.rs`
**Changes**: Add a `publishing: &dyn PullRequestBodyPublishing` parameter
after `body_update`, and PATCH the prepared body:

```rust
let prepared_body = publishing.prepare_for_publishing(body);
match body_update.update_body(
    &base.owner,
    &base.repo,
    pull_number,
    &prepared_body,
) {
```

Update the module doc and the function's doc to say what is now posted:

```diff
-//! Updating a pull request's body, resolving its base repository first.
+//! Updating a pull request's body as prepared for publishing on its forge,
+//! resolving its base repository first.
```

```diff
-/// Updates a pull request's body, resolving its base (upstream) repository
-/// first.
+/// Updates a pull request's body with the body as prepared for publishing
+/// on its forge, resolving its base (upstream) repository first.
```

**File**: `cli/collaboration/tests/fixtures/public-api.txt`
**Changes**: Regenerate it with `mise run public-api:update`. Read the diff
first: it should add only the trait, its method and the new parameter.

#### 2. GitHub adapter, test first

**File**: `cli/github/tests/pull_request_body_publishing.rs` (new)
**Changes**: Add
`preparing_for_publishing_unwraps_soft_line_breaks`, asserting that
`GitHubPullRequestBodyPublishing.prepare_for_publishing("> first line\n> second line")`
returns `"> first line second line"`. Exhaustive coverage stays in
`markdown`'s own fixtures.

**File**: `cli/github/Cargo.toml`
**Changes**: Add `markdown = { path = "../markdown" }`. The
adapter → technical-library edge passes `_adapter_rules`.

**File**: `cli/github/src/pull_request_body_publishing.rs` (new)

```rust
use collaboration::PullRequestBodyPublishing;

/// GitHub renders a single newline in a pull request body as a line break,
/// so a body is published with its soft line breaks unwrapped.
pub struct GitHubPullRequestBodyPublishing;

impl PullRequestBodyPublishing for GitHubPullRequestBodyPublishing {
    fn prepare_for_publishing(&self, body: &str) -> String {
        markdown::unwrap_soft_line_breaks(body)
    }
}
```

**File**: `cli/github/src/lib.rs`
**Changes**: Add `mod pull_request_body_publishing;` and
`pub use pull_request_body_publishing::GitHubPullRequestBodyPublishing;`.
Extend the crate doc:

```diff
 //! The GitHub forge implementation of `collaboration`'s ports: an
-//! `octocrab`-backed REST client and a `github.com` remote-URL recognizer.
+//! `octocrab`-backed REST client, a `github.com` remote-URL recognizer, and
+//! pull request body publishing that unwraps soft line breaks.
```

#### 3. Composition root, end-to-end tests first

**File**: `cli/collaboration-cli/Cargo.toml`
**Changes**: Add `serde_json = { workspace = true }` to
`[dev-dependencies]`.

**File**: `cli/collaboration-cli/tests/end_to_end.rs`
**Changes**: Add `resolving_server_accepting_patch(status)`, which builds
on the existing `resolving_server()` and adds only the PATCH route, and move
`update_body_succeeds` and `update_body_reports_a_patch_failure` onto it.
Add a `patched_body(server)` helper that parses `last_body` for the PATCH
key as JSON and returns `["body"]`. Then add:

- **`update_body_posts_soft_line_breaks_unwrapped`.** It writes
  `Wrapped\nparagraph  \nhard\nbreak\n\n```\ncode\nline\n```\n` and asserts
  that the patched body is
  `Wrapped paragraph  \nhard break\n\n```\ncode\nline\n```\n`.
- **`update_body_leaves_the_body_file_unchanged`.** It writes the same
  wrapped body, so writing the unwrapped body back would change the file.
  It asserts that the run exits 0, that the PATCH arrived, and that the
  file's bytes after the run equal those before it.
- **`update_body_ignores_repository_line_width_conventions`.** It runs the
  body
  `This paragraph is hard-wrapped to stay\nunder forty columns, and joins into one\nline on GitHub.\n`
  twice: once in a bare scratch repository, and once in a scratch
  repository that also contains:
  - an `.editorconfig` with `max_line_length = 40`
  - a `CLAUDE.md` with a line-width rule
  - a minimal valid `.accelerator/config.md`
  - `.accelerator/skills/describe-pr/instructions.md`
  - `.accelerator/templates/pr-description.md`

  It asserts that each run exits 0 and PATCHes exactly
  `This paragraph is hard-wrapped to stay under forty columns, and joins into one line on GitHub.\n`.
  The unwrapped line is 94 columns, so a skipped unwrap and a re-wrap to
  the configured 40 columns both fail.

**File**: `cli/collaboration-cli/tests/cli_surface.rs` (new) and
`cli/collaboration-cli/tests/fixtures/cli_surface.golden` (new)
**Changes**: Follow `cli/work-cli/tests/cli_surface.rs`.
`the_full_cli_surface_matches_the_committed_golden` concatenates the
`--help` output of `accelerator-collaboration`, `pr`, `pr base-repo` and
`pr update-body` under `=== … ===` headers and asserts it equals the
golden. Write it, and generate the golden from the binary, before any other
Phase 3 change: it pins the existing surface and must stay green
throughout.

**File**: `cli/collaboration-cli/src/main.rs`
**Changes**: Import `github::GitHubPullRequestBodyPublishing` and pass
`&GitHubPullRequestBodyPublishing` to `update_pull_request_body` after
`&client`.

#### 4. Skill and documentation

**File**: `skills/github/describe-pr/SKILL.md`
**Changes**:

```diff
-- Show the user the generated description (without frontmatter — they'll
-  see what gets posted to GitHub).
+- Show the user the generated description (without frontmatter). GitHub
+  receives the same text with its soft line breaks unwrapped, so wrapped
+  prose flows at the page's full width.
```

Add to Step 9, item 5, after the command block: "`update-body` unwraps soft
line breaks before posting; the body file and the saved description stay
wrapped. A line that must render on its own, such as a trailer or
`Closes #n`, needs a hard break (two trailing spaces or `\`) or a list
item." In the exit codes, move "body-file read failure" from **Exit 1**
to **Exit 2**.

**File**: `docs-site/src/content/docs/collaboration.md`
**Changes**: In the `update-body` paragraph (lines 34-38), add:

```markdown
Before PATCHing, it joins each soft-wrapped line onto the one before with a
single space, so prose hard-wrapped to repository conventions flows at full
width on GitHub. The space replaces the line ending together with any
trailing whitespace before it and the next line's `>` markers and
indentation. Everything else, including hard breaks, code and tables, is
posted byte-for-byte, and the body file is not modified. A line that must
render on its own, such as a trailer or `Closes #n`, needs a hard break (two
trailing spaces or `\`) or a list item.
```

#### 5. Notices

Run `mise run notices:check`. Phase 1 already added the `pulldown-cmark`
entry, so linking it into `accelerator-collaboration` should cause no
drift.

#### 6. Changelog

**File**: `CHANGELOG.md`
**Changes**: Under `## [Unreleased]`, add to `### Changed`:

```markdown
- **`describe-pr` posts pull request descriptions unwrapped.** `accelerator
  collaboration pr update-body` joins each soft-wrapped line onto the one
  before with a single space, dropping the line's `>` markers and
  indentation, so prose hard-wrapped to the repository's line width flows at
  full width on GitHub instead of breaking at every newline. Hard breaks,
  code and tables are posted byte-for-byte, and the body file and
  `meta/prs/` description stay wrapped. A line that must render on
  its own, such as a trailer or `Closes #n`, needs a hard break (two trailing
  spaces or `\`) or a list item.
```

and to `### Fixed`:

```markdown
- **`describe-pr` lists an unreadable body file under exit 2**, matching what
  `update-body` returns, where it listed it under exit 1.
```

### Success Criteria:

#### Automated Verification:

- [ ] Domain tests pass:
      `cargo test --manifest-path cli/Cargo.toml -p collaboration`
- [ ] Adapter tests pass:
      `cargo test --manifest-path cli/Cargo.toml -p github`
- [ ] End-to-end tests pass:
      `cargo test --manifest-path cli/Cargo.toml -p accelerator-collaboration`
- [ ] Public API pin passes: `mise run public-api:check`
- [ ] Architecture rules pass: `mise run lint:crate-dependencies:check` and
      `mise run pup:check`
- [ ] Notices are current: `mise run notices:check`
- [ ] Docs build passes: `mise run docs:check`
- [ ] Full local CI passes: `mise run`

#### Manual Verification:

- [ ] `describe-pr` SKILL.md no longer says the shown description is what
      gets posted, and says soft line breaks are unwrapped
- [ ] Running `/accelerator:describe-pr` on this story's PR posts a body
      whose paragraphs flow at full width on GitHub, while
      `meta/prs/<n>-description.md` stays wrapped
- [ ] The `[Unreleased]` changelog entries describe the unwrap, the
      hard-break guidance and the exit-code correction
- [ ] The posted PR description contains the Phase 2 corpus verification
      record; add it by hand if `describe-pr` summarised it away

---

## Testing Strategy

### Unit Tests:

- In `markdown`: every acceptance-criterion fixture, plus edge fixtures for
  escapes, link-closing breaks, inline spans and lazy lines, trailing
  whitespace, multi-line paragraphs, multibyte text, and any corpus- or
  probe-derived fixtures. Each runs under LF and CRLF with the idempotence
  and normalised event-stream checks, and is preceded by the `SoftBreak`
  range characterisation.
- In `collaboration`: the use case PATCHes the port's prepared body.
- In `github`: one delegation check for `GitHubPullRequestBodyPublishing`.

### Integration Tests:

- The `accelerator-collaboration` binary against `MockHTTPServer` covers:
  - the PATCHed `body`
  - the unchanged body file
  - independence from configuration
  - the unchanged `--help` surface, pinned by a golden
- The `cargo-pup` probe pair for `markdown_transforms_text_only`.

### Manual Testing Steps:

1. Run the Phase 2 corpus verification and divergence probes, and fill in
   the record.
2. Confirm by hand that the table probe renders as a table on GitHub.
3. Raise this story's PR through `raise-pr`. Confirm the description renders
   unwrapped on GitHub and the `meta/prs/` artefact stays wrapped.

## Performance Considerations

⏱️ One parse per `update-body` call over a body of a few kilobytes, which is
negligible next to the two GitHub API round trips the command already makes.

## Migration Notes

There is no data migration. This replaces 0071's byte-for-byte contract:
the posted body now differs from the body file at soft line breaks only.
Rolling back means reverting Phase 3. Phases 1 and 2 have no runtime effect.

## References

- Original work item: `meta/work/0332-push-pr-descriptions-to-github-unwrapped.md`
- Codebase research:
  `meta/research/codebase/2026-10-13-0332-push-pr-descriptions-to-github-unwrapped.md`
- ADR-0069:
  `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`
- Library-crate checklist: `tasks/README.md:790-854`
- Precedent technical library: `cli/remote-projection/`, `cli/pup.ron:203-221`
- Use case and port: `cli/collaboration/src/update_body.rs:31-60`,
  `cli/collaboration/src/lib.rs:106-117`
- Related: 0320 (Linear push path), 0296 (exact parser pin), 0071
  (byte-for-byte contract replaced)

## Corpus verification record

Run on 2026-10-14 against `markdown` at `pulldown-cmark` 0.13.4.

- Commands:
  - `corpus-verify <out> meta/prs/*-description.md probes/*.md`, a scratch
    binary crate depending on `cli/markdown`, with the normaliser and
    `github_extensions` copied verbatim from the crate
  - per file, `gh api -X POST /markdown -f mode=markdown -F text=@in/<n>.md`,
    the same for `out/<n>.md`, and `-f mode=gfm` for `out/<n>.md`: 366
    calls
  - `python3 -I normalise.py compare` for structure, and
    `normalise.py brs` (counts `<br` openings) for residual wraps
- Descriptions and probes checked: 115 descriptions and 7 probes, 122 in
  all. All 122 pass event-stream equality, idempotence and the structure
  check.
- Span-logic failures → fixtures: none.
- Structural divergences → fixtures and fixes: none. The span rule is
  unchanged.
- Residual wraps → follow-up work items: none in the corpus. The only
  residual wrap is the `foo\t\t\nbar` probe, the predicted tab
  hard-break divergence below.
- Calibration controls (normaliser stability, structure, residual-wrap):
  - normaliser stability: two renders each of the footnote and math probes
    normalise identically; no attribute differs between renders, so no
    unstable attribute is stripped
  - structure: `foo\n- bar` against `foo - bar` differs, as required
  - residual-wrap: `a\nb` shows an excess of 1 and `a  \nb` of 0
  - normaliser rules: drop every `id` and `aria-labelledby` attribute and
    every `href="#…"`, drop any attribute that differs between two renders
    of one input, and collapse whitespace runs outside `<pre>` and `<code>`
- Divergence probes → outcome (fixture name or follow-up), including the
  tab hard-break divergence:
  - table interrupting a paragraph: matches, already covered by
    `a_table_interrupting_a_paragraph_stays_intact`
  - lowercase alert: matches, `a_lowercase_alert_keeps_its_marker_line`
  - unknown alert kind: matches, `an_unknown_alert_kind_joins_as_quoted_text`
  - non-pipe lines after a table: matches, stay rows,
    `lines_after_a_table_stay_its_rows`
  - footnote definition's lazy continuation: matches,
    `a_footnote_definitions_lazy_continuation_joins`
  - inline math: matches, GitHub's `/markdown` renders `$…$` as text,
    `dollar_delimited_math_joins_as_text`
  - two trailing tabs: confirmed divergence. `pulldown-cmark` reads a hard
    break and leaves the line wrapped; GitHub's `gfm` render still shows one
    `<br>`. It renders no worse than today and is pinned by
    `two_trailing_tabs_stay_a_hard_break`.
  - The REST `/markdown` endpoint renders neither alert probe as an alert,
    so the structure check cannot confirm alert rendering on the PR page.
    The unwrap keeps the marker line on its own in both alert-kind cases
    `pulldown-cmark` recognises.
- By-hand table and footnote checks: the table probe renders, before and
  after, as the paragraph `Some wrapped text` followed by a table; the
  footnote probe's input render contains a `<section data-footnotes>`.
