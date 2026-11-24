---
type: "plan"
id: "2026-11-23-PP-906-escape-linear-bound-markdown-on-push"
title: "Escape Linear-Bound Markdown on Push Implementation Plan"
date: "2026-11-23T21:16:00+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "in-progress"
work_item_id: "work-item:PP-906"
parent: "work-item:PP-906"
derived_from: ["codebase-research:2026-11-23-PP-906-linear-push-corrupts-markdown-content"]
relates_to: ["plan:2026-10-13-PP-885-push-pr-descriptions-to-github-unwrapped", "plan:2026-10-10-PP-880-sync-round-trip-defects"]
tags: ["sync", "linear", "markdown", "linear-client", "markdown-crate"]
revision: "22356e9220263ebd479e1bb378a05df2f4b87d2d"
repository: "accelerator"
last_updated: "2026-11-24T14:00:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Escape Linear-Bound Markdown on Push Implementation Plan

## Overview

Linear's API parser rewrites two kinds of pushed Markdown so that their meaning
changes: a lone number on a wrapped continuation line becomes an ordered list
(`X09`), and a bare dotted token whose last label is a TLD becomes a link
(`R12`). This plan adds a push-side escaper, `markdown::linear::escape_for_push`.
It inserts `\` before the `.` or `)` that triggers each rewrite. The escape
is Linear's Markdown dialect, applied where a description becomes a Linear
request, in `LinearClient::create_op` and `update_op`, as Jira's client
converts Markdown to ADF in its own `create_op` and `update_op`. So every
command that sends a description to Linear sends the same bytes for the same
Markdown: `work`, `sync`, promotion and the raw `accelerator linear create`
and `update`. `--verbatim` on the raw commands opts out, so Linear's parser
can be captured again. Local files and digests are unchanged.

## Current State Analysis

- **Send path.** Every work-item push reaches Linear through `impl
  RemoteTracker for LinearClient` (`cli/linear-client/src/client.rs:892-918`).
  `create` forwards the body to `create_op`, and `update` puts it in
  `UpdateFields.description` for `update_op`. The raw `accelerator linear
  create` (both its positional work-item mode and `--title`/`--body-file`)
  and `update` commands call `create_op` and `update_op` directly
  (`cli/linear-cli/src/main.rs:306-426`); `/create-linear-issue` uses the
  positional mode. `transition` also calls `update_op`, with no description.
- **Jira precedent.** `jira-client`'s `create_op` and `update_payload` convert
  the Markdown body to ADF (`cli/jira-client/src/mutation.rs:89`, `:209`), so
  the raw `jira` commands and `work`/`sync` send identical descriptions.
- **Comparison.** `LinearClient`'s `BodyDialect` overrides only
  `is_wire_document` (`client.rs:885-889`), so `as_pushed` is identity. An
  inserted `\.` resolves away in the parse, and PP-880's canonical form merges
  adjacent text (`cli/corpus/src/body/canonical.rs:270-284`). An escaped and an
  unescaped body therefore have the same digest.
- **Parser access.** Only `document` and `markdown` may depend on
  `pulldown-cmark` (`cli/deny.toml:178`, pinned `=0.13.6`). `markdown` exists
  for PP-885 and holds `continuation_start` (`cli/markdown/src/lib.rs:18-34`),
  which finds the first content byte of a continuation line after its `>` and
  indentation. `markdown::github` uses a wider option set (footnotes, math,
  GFM) than PP-880's `document::markdown` (`cli/document/src/markdown.rs:66-70`:
  tables, task lists, strikethrough). `document::markdown::events`
  (`:57-65`) already exposes PP-880's reading as `(MarkdownEvent, Range)`
  pairs, free of `pulldown_cmark` types, and both crates are
  `technical-library`, so `markdown` may depend on `document`.
- **Crate roles.** `linear-client` is an `adapter`. `github`, also an adapter,
  already depends on `markdown` (`cli/github/Cargo.toml:18`), so the new edge
  passes `tasks/lint/crate_dependencies.py`.
- **Raw update.** `UpdateArgs` (`cli/linear-cli/src/cli.rs:58-73`) has
  `--description <STRING>` but no `--body-file`. `CreateArgs.body_file` reads
  its file in `resolve_create_inputs` (`main.rs:399-410`) and reports
  `E_CREATE_NO_FILE` on failure.
- **Capture.** `cli/work-adapters/tests/fixtures/linear-rewrites/` holds
  `pushed.md` (fragments `R01`-`R13`, `X01`-`X09`, 113 lines), `read-back.json`
  and a `README.md` whose tables cite line ranges. Three tests consume it:
  - `work-adapters/tests/linear_rewrites.rs:45-102` folds simulated
    `Rewrite`s over each listed fragment and compares the result with the
    read-back. `X09` is absent because no cosmetic rewrite reproduces it.
  - `corpus-adapters/tests/body_equivalence.rs:270-281` asserts the whole
    capture is equivalent once `R12` and `X09` are excluded.
  - `tracker-test-support/src/rewrite.rs:181-192` simulates Linear's
    autolinker as "the last label is two or more lowercase letters". That
    links `foo.toml` and misses `Crates.IO`.
- **Push-path harness.** `work-adapters/tests/sync_run_real_client.rs` builds
  real clients against `MockHTTPServer` (`linear_client` at `:207`) and loops
  `enum Tracker` × `enum PullEntryPoint` (`:1191-1350`). `work create --push`
  and `work update --push` live in `work-cli`. They are tested in-process with
  `FixedRegistry::holding` (`cli/work-cli/src/test_support.rs:101-116`),
  because a CLI subprocess refuses a loopback base.

## Desired End State

- `markdown::linear::escape_for_push` applies the digit rule and the
  dotted-token rule to parsed text and changes nothing else. Its output
  differs from its input only by inserted backslashes, and it is idempotent.
  If an escape would change the CommonMark reading, it returns the input
  unchanged.
- After every body push, to any tracker, the read-back is judged against the
  body sent under PP-880's equivalence, which cannot see a stray list whose
  number keeps its digits (a known limit pinned in phase 7). A rewrite is reported as an item
  notice and a `warning: <id>: rewritten on push: <sections>` block showing
  each line as sent and as stored, the baseline is recorded as before, and
  the entry carries a rewrite mark until a faithful push, a pull, or a sync
  that finds the bodies equivalent clears it. `work list` and every later
  sync show the mark.
- For every fragment of `linear-rewrites/pushed.md`, `escape_for_push`
  returns its fragment in `linear-rewrites/escaped/pushed.md` byte for byte.
- Every description sent to Linear is escaped: `create_op` and `update_op`
  escape a `Description::Markdown` and send a `Description::Verbatim` as
  given. Every `RemoteTracker` push and every raw `accelerator linear create`
  and `update` sends `Markdown` unless `--verbatim` is passed. Jira and
  Shortcut send bodies as before.
- `accelerator linear update --body-file <path>` sends the file as the
  description, matching `linear create`, `jira update` and `shortcut update`,
  and conflicts with `--description`.
- The two captures are committed. Linear's read-back of the escaped capture is
  equivalent to the unescaped fragments, and its web UI shows no backslash.
- After merge, every affected Linear item has been repaired with
  `accelerator work update <path> --push`.

Verify with `mise run` (the full default task) exiting 0, plus the manual
checks recorded in phases 2, 3, 10, 11 and 12.

### Key Discoveries:

- `continuation_start` clamps to the next event's start only when that event
  is on the continuation line (`markdown/src/lib.rs:28-33`). An `End(Link)`
  after a soft break starts at the link's opening (`markdown/src/tests.rs:94-105`),
  so the digit rule reads source bytes from `continuation_start`, not from
  the next event.
- An interrupting `1. text` is a new list in CommonMark, not a continuation
  (`markdown/src/tests.rs:160-165`), so the digit rule never sees it.
- Code and HTML blocks, code spans, inline HTML tags and angle-bracket
  autolinks each arrive as their own event kind or inside a `Link`. Skipping
  them is a match on the event, not a text scan. Text between inline tags
  (`<span>crates.io</span>`) is ordinary `Text`, where `\.` renders as `.`,
  so it is escaped like any other text; only the tags are raw HTML.
- Nobody has pinned how `pulldown-cmark` 0.13.6 ranges `Text` events around a
  backslash escape. The dotted-token rule maps characters back to source
  offsets and detects existing escapes from that gap, so phase 4 pins it first.
- Linear's read-back already carries `0160\.` (`R11`), and a pull writes it
  locally. Escaping parsed text rather than raw text is what keeps that from
  becoming `0160\\.`.
- `MockHTTPServer::last_body` and `bodies` (`graphql-test-support/src/lib.rs:108-120`)
  expose every request, so the push-path matrix asserts the wire bytes.
- `cli/work-adapters/tests/fixtures/linear-autolink-probes/` records 519
  probes and a sweep of every IANA TLD, pushed to Linear on 2026-11-24. Its
  README states the observed rules. Four of them shape the escaper:
  - Linear links every IANA last label except bare `md` and `merck`.
  - Labels may hold Unicode letters (`café.io` is linked).
  - `http`, `https` and `ftp` URLs and `//` are linked after any boundary,
    quotes and `URL:` included, and an escape inside one cuts the link short.
  - Linear stores no escape it receives: `crates\.io` reads back as plain,
    unlinked `crates.io`.
- An escape where Linear would not have linked is invisible, so the escaper
  may over-escape. It must never miss a token Linear links, and must never
  escape inside a URL Linear links.
- A lone `<digits>.` or `<digits>)` becomes a list item under every list
  item kind, after a tab, on a lazy line at column 0 (where CommonMark also
  reads a new list, so no escape is due), for `0.` and `00.`,
  and on each of several consecutive lines. It does not in a blockquote, in a
  later paragraph of an item, at top level, for ten digits, or with text
  after the number. An empty `1.` is a lone line too: it cannot interrupt a
  paragraph in CommonMark, so it continues it.

## What We're NOT Doing

- Absorbing cosmetic re-serialisations, including code-span indentation
  (`X07`, `X08`).
- Changing Jira or Shortcut sends, or checking Shortcut's parser.
- Changing any local file, or rewriting remote descriptions in bulk.
- Stripping stored escapes on pull. The probes show Linear stores none of the
  escapes the rules insert; if that changes, PP-880's equivalence still
  treats `crates\.io` and `crates.io` as one. Phase 3's capture (`TES-37`)
  confirmed it: `escaped/README.md` records no stored escape.
- Matching Linear's autolinker exactly. The escaper over-escapes the classes
  listed under phase 5's oracle, such as bare `.md` files and tokens after
  `/`, because Linear stores those escapes as plain text.
- Unwrapping soft line breaks before sending. It would remove every stray
  list too, but a pull after an edit in Linear would then write unwrapped
  lines into `meta/work/`.
- Image re-hosting to `uploads.linear.app`, which the probes also found.
- Remedying entity decoding in this plan. Phase 11 only probes whether a
  decoded entity changes meaning, and raises a work item for any remedy.
- Changing `BodyDialect::as_pushed` for Linear, `DigestRecipe::CURRENT` or
  `migrate-cli`'s Linear dialect mapping.
- Converting a Unicode last label to punycode to look it up. A last label
  holding any non-ASCII letter counts as a TLD, so `foo.рф` is escaped and so,
  needlessly, is `naïve.café`.
- Repairing PP-714 (0184), which reads back intact.
- Reporting the parser's behaviour to Linear.

## Implementation Approach

The work runs in twelve phases, each mergeable alone:

```text
1 linear update --body-file ──┐
2 unescaped capture + TLDs ───┼─► 3 escaped capture ─► 4 digit rule ─► 5 dotted-token rule ─► 6 wiring ─► 7 rewrite notice ─► 8 rewrite mark ─► 12 post-merge repair
                                                                       │                      │
                                                                       ▼                      ▼
                                                                       9 simulator            10 comments
11 entity probe (independent)
```

Phase 12 runs after the merge of phases 1-8. Phases 9-11 need not wait for
it; any of them may also land in the same PR.

Phase 1 is independent and lands first. Phases 2 and 3 include live captures
that a credentialed person runs by hand against the scratch `TES` team, never
the `PP` team. `create_op` creates in `linear.team_id`, falling back to the
catalogue's base team (`cli/linear-client/src/auth.rs:87-96`); `team_key`
only scopes identifiers. So each capture:

1. Saves a copy of `.accelerator/config.local.md`.
2. Takes the `TES` team's id from `accelerator linear init list-teams`, and
   sets both `linear.team_id: "<that id>"` and `linear.team_key: "TES"` in
   `config.local.md`.
3. Before any `create`, runs `list-teams` again and checks the id in the
   file is the one listed against key `TES`.
4. Confirms each returned identifier starts `TES-`.
5. Deletes each issue once its read-back is saved. The CLI has no delete
   command, so this is a GraphQL `issueDelete` with the configured token,
   which moves the issue to Linear's trash; `issue(id:)` then reports
   `trashed: true`.
6. Restores the saved `config.local.md` and `cmp`s it against the copy, so
   no later push goes to `TES`. This step runs on every exit: if a `create`,
   `show` or delete fails, restore first, then deal with the failure.

The fixtures, README tables and test changes around them are code. Each has a gate:

| Phase | Gate | If it fails |
|---|---|---|
| 2 | Every token Linear links ends in a snapshot label, and every unlinked one that does falls in a recorded over-escape class | Dotted-token rule returns to drafting; phase 3 escapes digits only; phase 5 is dropped |
| 3 | The web UI shows no backslash before `.` or `)` | The whole item returns to drafting |

The escaper is built inside out: parser characterisation, then each rule
red-green-refactor, then the byte-identical fixture test, and only then
wiring. Until phase 6, `markdown::linear` has no production caller, so phases
4 and 5 change no behaviour.

---

## Phase 1: `accelerator linear update --body-file`

### Overview

The flag gives `linear update` the same `--body-file` input as `linear
create`, `jira update` and `shortcut update`; the inline flag stays
`--description`, and unlike Jira's `--body` it refuses to be combined with a
file. It sends the file through `update_op`
like `--description`, so from phase 6 it is escaped like every other Linear
description.

### Changes Required:

#### 1. Flow tests first

**File**: `cli/linear-cli/tests/flow_update.rs`
**Changes**: Add failing tests following `input_sent_for` and
`assert_refused`:

- `a_body_file_is_sent_as_the_description`: seed a file holding Markdown
  that needs no escape (`"# Plan\n\n- one\n- two\n"`), run `update BLA-1
  --body-file <path>`, and assert `{"description": <exact file contents>}`.
- `a_body_file_alone_names_a_field`: `--body-file` with no other flag is not
  refused with `E_UPDATE_NO_OPS`.
- `a_body_file_and_a_description_conflict`: both flags exit with clap's usage
  code `2`, and `server.hits(&graphql())` is `0`.
- `an_unreadable_body_file_is_refused`: a missing path exits with the new
  `UPDATE_NO_FILE` code, stderr contains `E_UPDATE_NO_FILE` and the path, and
  nothing is sent.

In `cli/linear-cli/tests/flow_create.rs`, add
`an_unreadable_body_file_names_its_path`: stderr for `E_CREATE_NO_FILE`
contains the path.

#### 2. Arguments

**File**: `cli/linear-cli/src/cli.rs`

```rust
#[derive(Args)]
#[command(group = ArgGroup::new("description_source").args(["description", "body_file"]).multiple(false))]
pub struct UpdateArgs {
    pub id: String,
    #[arg(long)]
    pub title: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    /// A Markdown file sent as the description.
    #[arg(long)]
    pub body_file: Option<PathBuf>,
    // ... unchanged
}
```

The `description_source` group states the conflict once; phase 6's
`--verbatim` requires the same group.

`names_no_field` also checks `self.body_file.is_none()`.

#### 3. Reading the file

**File**: `cli/linear-cli/src/main.rs`
**Changes**: Extract the read-and-report block from `resolve_create_inputs`
into a helper both commands call, then have `run_update` resolve the
description before building the client:

```rust
struct BodyFileRefusal {
    code: &'static str,
    exit: u8,
}

fn read_body_file(path: &Path, refusal: BodyFileRefusal) -> Result<String, ExitCode> {
    std::fs::read_to_string(path).map_err(|error| {
        eprintln!(
            "{}: could not read --body-file {}: {error}",
            refusal.code,
            path.display()
        );
        ExitCode::from(refusal.exit)
    })
}

fn requested_description(args: &UpdateArgs) -> Result<Option<String>, ExitCode> {
    match (&args.description, &args.body_file) {
        (Some(description), None) => Ok(Some(description.clone())),
        (None, Some(path)) => read_body_file(path, UPDATE_BODY_FILE_REFUSAL).map(Some),
        (None, None) => Ok(None),
        (Some(_), Some(_)) => unreachable!("description_source admits one member"),
    }
}
```

`resolve_create_inputs` calls `read_body_file` with `E_CREATE_NO_FILE`, so
its message gains the path too; update `flow_create.rs`'s expectation if it
pins the old wording. `UpdateFields.description` borrows from the resolved
`Option<String>`.

#### 4. Exit code

**Files**: `cli/linear-cli/src/exit_codes.rs`,
`cli/linear-cli/tests/exit_codes_parity.rs`,
`cli/linear-cli/tests/stderr_diagnostics.rs`
**Changes**: Declare `UPDATE_NO_FILE: u8 = 115` beside `UPDATE_BAD_STATE`
(114), noted as a new code rather than a captured remap, as `SEARCH_CAP_HIT`
is. Do not add it to `tests/fixtures/captured-exit-codes.txt`, which records
only the codes captured from the retired shell flows. Test first:
`the_update_no_file_code_is_115_and_uncontested` in `exit_codes_parity.rs`,
following `the_search_cap_hit_code_is_79_and_uncontested`, and an
`E_UPDATE_NO_FILE` case in `stderr_diagnostics.rs`.

#### 5. Surface, skill and CHANGELOG

**Files**: `cli/linear-cli/tests/fixtures/cli_surface.golden`,
`skills/integrations/linear/update-linear-issue/SKILL.md`, `CHANGELOG.md`
**Changes**: Regenerate the golden so update's options list `--body-file`. In
the skill, add `--body-file PATH` to the `argument-hint` and Step 1, carry
the "an empty description clears it" warning over to an empty file in Step
2, list `E_UPDATE_NO_FILE` among the refusals before any write in Step 4,
and note that `--description` and `--body-file` cannot be combined. Add an
**Added** entry under `[Unreleased]` for `--body-file` and
`E_UPDATE_NO_FILE` (exit 115).

### Success Criteria:

#### Automated Verification:

- [x] Flow tests pass: `mise run test:unit:cli --filter 'binary(flow_update) | binary(flow_create)'`
- [x] Exit-code parity holds: `mise run test:unit:cli --filter 'binary(exit_codes_parity)'`
- [x] Diagnostics and surface hold: `mise run test:unit:cli --filter 'binary(stderr_diagnostics) | binary(cli_surface)'`
- [x] `mise run cli:check` exits 0
- [x] `mise run` exits 0

#### Manual Verification:

- [x] `accelerator linear update --help` lists `--body-file` beside `--description`

---

## Phase 2: Extended unescaped capture and the TLD snapshot

### Overview

Capture Linear's rewrite of every dotted-token class and boundary shape, and
of a lone `<digits>)` continuation line. Pin the IANA TLD list, and make the
simulated autolinker follow it, so that a test shows whether the snapshot
predicts what Linear linked.

### Changes Required:

#### 1. New fragments

**File**: `cli/work-adapters/tests/fixtures/linear-rewrites/pushed.md`
**Changes**: Append after line 113, so existing README line ranges hold:

```markdown

## X10 File names whose extension is a TLD

Remove server.pid and foo.md first.

## X11 File name with a line number

See jql.rs:167 for the filter.

## X12 File name whose extension is not a TLD

Edit foo.toml before the release.

## X13 Version and abbreviation

Upgrade to v1.2.3 first, e.g. tomorrow.

## X14 Multi-part tokens

Both a.b.c and a.b.io resolve.

## X15 Mixed case

Published to Crates.IO last week.

## X16 Sentence-final dotted token

Published to crates.io.

## X17 Bracketed dotted token

Published (crates.io) last week.

## X18 Path ending in a dotted token

Edit src/client.rs today.

## X19 Email address

Mail a@b.io today.

## X20 Wrapped parenthesised number holding only the number

- The tier infrastructure and the migration of existing evals — both owned by
  0160)
- Running evals on every CI build.

## X21 Dotted token between inline HTML tags

Published to <span>crates.io</span> last week.

## X22 Dotted token in bold and in a heading

Published to **crates.io** last week.

### Notes on crates.io

## X23 Dotted token in a table cell

| Registry | Use |
| - | - |
| crates.io | Rust |

## X24 Wrapped number in a blockquote

> The tier infrastructure and the migration of existing evals — both owned by
> 0160.

## X25 Quoted scheme URL

See "https://crates.io/x" and URL:https://crates.io for details.
```

`R12` already covers the bare domain name class. `X21`-`X25` put escaped
contexts the rules reach in front of the phase 3 web-UI check.

#### 2. Live capture (manual)

1. `accelerator linear create --title "PP-906 unescaped capture" --body-file
   cli/work-adapters/tests/fixtures/linear-rewrites/pushed.md` with the
   `TES` overrides set as above. Confirm the identifier starts `TES-`.
2. `accelerator linear show <identifier>`, projected through the `jq`
   allowlist in `work-item-sync-recovery/README.md`, into `read-back.json`.
3. Diff the old and new `read-back.json` for `R01`-`X09`. Any change to an
   existing fragment is recorded in the README and stops the phase for
   review, because Linear's parser has moved.
4. Delete the issue, then restore `config.local.md`.

Taken on 2026-11-24 as `TES-32`, since trashed. `R01`-`X09` read back
byte-identical to the 2026-10-13 capture. Linear linked `server.pid`,
`jql.rs:167` (port included), `a.b.io`, `Crates.IO`, `crates.io` in `X16`,
`X17`, `X22` (bold and heading) and `X23`, `a@b.io` as `mailto`, and both
`X25` URLs whole. It left `foo.md`, `foo.toml`, `v1.2.3`, `e.g.`, `a.b.c`,
`src/client.rs` and `<span>crates.io</span>` unlinked. `X20` became a list
item `160.`; `X24` was unchanged.

The two unlinked TLD tokens (`foo.md` and `src/client.rs`) prompted a probe
run the same day: `TES-33` and `TES-34` (310 probes, by create and by
update), `TES-35` (every IANA TLD) and `TES-36` (209 follow-ups), all since
trashed. Their bodies and read-backs are committed as
`cli/work-adapters/tests/fixtures/linear-autolink-probes/`, and the rules
they show revised the gate, phase 3's hand escapes and phases 4 and 5.

#### 3. README

**File**: `cli/work-adapters/tests/fixtures/linear-rewrites/README.md`
**Changes**: Update the capture date and identifier. Add a "Dotted tokens and
boundaries" table for `X10`-`X19` with the columns Fragment, Pushed, Read
back, Container and Observed rewrite. Its Observed rewrite column names each
token Linear linked (`[t](<http://t>)`) or `none`. Add `X21`-`X25` to the
same table, recording for `X25` whether Linear linked each quoted URL and
over what span. Add `X20` to the Variants
table with its rewrite (ordered list, escape, or none). For every linked
token, record its exact span and link form: `http` (`[t](<http://t>)`) or
`mailto` (`[t](<mailto:t>)`). Add one sentence per gate outcome under
"Against 0296's Equivalence table": whether every linked token's last label
is in the snapshot, whether each unlinked token whose last label is in the
snapshot falls in an over-escape class of
`../linear-autolink-probes/README.md` (`foo.md`: bare `md`;
`src/client.rs`: after `/`; `X21`: touching an HTML tag), and whether each
linked span in `X10`-`X19` holds exactly the dotted token the rule finds in
it. Point to the probe corpus for the rules behind each class.

Add an "Audit self-check codes" section listing four sets that phase 12's
audit checks itself against, each code with its stored line: the codes
whose unescaped read-back holds a linked dotted token (at least `R12`) or a
stray list line (`X09` and `X20`; Linear left `X24` alone), and, once
phase 3 has run, the codes whose escaped read-back must
match neither (at least `R12`, `R11`, `X09`, `X20` and `X24`). Retaking a capture
updates these lists in the same change.

Add a "Refreshing the TLD snapshot" section: download
`https://data.iana.org/TLD/tlds-alpha-by-domain.txt` over
`cli/markdown/src/linear/tlds-alpha-by-domain.txt`, run
`the_snapshot_is_iana_shaped` and the capture tests, and add a capture
fragment for any token whose verdict changes. A refresh is due whenever a
`rewritten on push` warning or mark, or a
phase 12-style audit finds a newly linked token.

#### 4. Pinned TLD snapshot

**Files**: `cli/markdown/src/linear/tlds-alpha-by-domain.txt` (new),
`cli/markdown/src/linear/top_level_domains.rs` (new),
`cli/markdown/src/linear.rs` (new), `cli/markdown/src/lib.rs`
**Changes**: Download `https://data.iana.org/TLD/tlds-alpha-by-domain.txt`
verbatim, keeping its `# Version …` line. Tests first, in
`cli/markdown/src/linear/tests.rs`:

- `the_snapshot_is_iana_shaped`: the first line starts with `# Version`, every
  other line is a non-empty label of ASCII upper-case letters, digits and `-`
  (so `XN--P1AI` passes), and there are at least 1,400 of them.
- `top_level_domains_ignore_case`: `io`, `IO`, `rs`, `md` and `pid` are TLDs,
  and `toml`, `yaml`, `g` and `3` are not. If `pid` is absent from the
  snapshot, this assertion moves to the "not" side and the README records
  that Linear's linking of `server.pid` contradicts the snapshot (gate fails).

```rust
pub mod linear;
```

```rust
// cli/markdown/src/linear.rs
mod top_level_domains;

pub use top_level_domains::is_top_level_domain;

#[cfg(test)]
mod tests;
```

```rust
// cli/markdown/src/linear/top_level_domains.rs
use std::collections::HashSet;
use std::sync::LazyLock;

const IANA_ROOT_ZONE: &str = include_str!("tlds-alpha-by-domain.txt");

static TOP_LEVEL_DOMAINS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    IANA_ROOT_ZONE
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect()
});

/// Whether `label` is a top-level domain in the pinned IANA root zone
/// snapshot, ignoring ASCII case.
#[must_use]
pub fn is_top_level_domain(label: &str) -> bool {
    TOP_LEVEL_DOMAINS.contains(label.to_ascii_uppercase().as_str())
}
```

Run `mise run public-api:update` and check that the diff adds only
`markdown::linear` and `is_top_level_domain`.

#### 5. Simulated autolinker follows the snapshot

**Files**: `cli/tracker-test-support/Cargo.toml`,
`cli/tracker-test-support/src/rewrite.rs`, `cli/pup.ron`
**Changes**: Add `markdown = { path = "../markdown" }`, and add
`"^markdown::linear(::|$)"` to the `tracker_test_support_imports_only_permitted`
allowlist in `cli/pup.ron`, which otherwise refuses the import. Test first: extend
`recorded_rewrites()` in `work-adapters/tests/linear_rewrites.rs` with
`X10`, `X12`, `X13` and `X15` mapped to `Rewrite::AutolinkDottedTokens`.
`X10` fails because the current "two or more lowercase letters" last-label
test links `foo.md`, `X12` fails because it links `foo.toml`, and `X15`
(`Crates.IO`) fails because it rejects `IO`. All three pass once the
last-label test is `markdown::linear::is_top_level_domain(tld)` and the
label is not one Linear leaves bare:

```rust
const LEFT_BARE_BY_LINEAR: [&str; 2] = ["md", "merck"];
```

compared ignoring ASCII case. The exception lives in the simulator, not in
`markdown`, because the escaper deliberately escapes those tokens too. The
name test already accepts upper case through `is_ascii_alphanumeric`. `X13`
(`v1.2.3`, `e.g.`) already passes and pins existing behaviour.

`X11` (`jql.rs:167`), `X14` (`a.b.io`), `X16`-`X19`, `X20` and `X21`-`X25`
stay out of
`recorded_rewrites()`, as `X09` does, and its doc comment says why: the
simulator splits on whitespace and takes one dot-free name before the last
label, so it cannot model ports, multi-label names or boundaries. Phase 5's
capture test checks them against Linear's read-back directly. Once phase 5
lands, the simulator can call phase 5's dotted-token recogniser instead, so
one grammar serves both. Phase 9 makes that change; phase 5 exposes the
recogniser it needs.

#### 6. Equivalence exclusions

**File**: `cli/corpus-adapters/tests/body_equivalence.rs`
**Changes**: Add every new fragment the README records as linked or as an
ordered list to the `without(&[...])` list at `:276`: `X10`, `X11`, `X14`-`X17`,
`X19`, `X20`, `X22` and `X23`. Add a `captured_row_differs` test for each
and a `captured_row_is_noise` test for each fragment rewritten only
cosmetically. `X25`'s links have text equal to their destination, as `R06`'s
do, and `quoted_scheme_urls_are_noise` confirms it is noise.

### Success Criteria:

#### Automated Verification:

- [x] Snapshot and lookup tests pass: `mise run test:unit:cli --filter 'package(markdown)'`
- [x] The simulator reproduces `X10`, `X12`, `X13` and `X15`: `mise run test:unit:cli --filter 'binary(linear_rewrites)'`
- [x] Capture equivalence tests pass: `mise run test:unit:cli --filter 'binary(body_equivalence)'`
- [x] Public API snapshot matches: `mise run public-api:check`
- [x] `mise run` exits 0

#### Manual Verification:

- [x] Live capture taken, allowlisted, committed and its issue deleted
- [x] README records the rewrite of `X10`-`X20` and the gate outcome
- [x] Gate: every linked token in `X10`-`X19` ends in a snapshot label, and
      every unlinked token ending in one falls in an over-escape class the
      probe corpus records. If not, stop and return the dotted-token rule to
      drafting.
- [x] Gate: each linked span in `X10`-`X19` holds exactly the dotted token
      the rule finds in it, compared as phase 5's oracle compares them (a
      `mailto` link over `a@b.io` holds `b.io`, which passes). A linked span
      holding no rule token, or a different one, returns that shape to
      drafting before phase 3 escapes it; the README records which.

---

## Phase 3: Escaped capture

### Overview

Push each fragment escaped by hand, read it back, and check the web UI. This
settles the item's first assumption before any escaper code exists.

### Changes Required:

#### 1. Hand-escaped fragments

**File**: `cli/work-adapters/tests/fixtures/linear-rewrites/escaped/pushed.md` (new)
**Changes**: A copy of `pushed.md` with only these insertions:

- **Digit rule.** `\` before the `.` or `)` of `<digits>.` or `<digits>)`
  that opens a paragraph continuation line, lazy lines included, and is
  followed by a space, a tab or end of line. That gives `R11`, `X04`-`X06`,
  `X09` (both occurrences), `X20` and `X24`; `X24` is an over-escape.
- **Dotted-token rule.** `\` before every unescaped `.` of each bare dotted
  token whose last label is in the snapshot or holds a non-ASCII letter,
  including `X10`'s `foo.md`, `X18` and `X21`-`X23`. Inside a scheme URL
  nothing is escaped, so `R06` and both `X25` URLs are left alone. `X19`
  becomes `a@b\.io`.

The file must equal what the phase 5 escaper produces, so each insertion
follows the rules as written in the work item, not Linear's observed
behaviour.

#### 2. Live capture (manual)

1. `accelerator linear create --title "PP-906 escaped capture" --body-file
   …/escaped/pushed.md` with the `TES` overrides set as above. Confirm the
   identifier starts `TES-`.
2. `accelerator linear show <identifier>`, projected through the allowlist,
   into `escaped/read-back.json`.
3. Open the issue in Linear's web UI. Check `R11`, `X04`-`X06`, `X09`,
   `X20`, `X21`-`X25` and every escaped token for a visible backslash, a
   stray list and a link.
4. Delete the issue after the check is recorded, then restore
   `config.local.md`.

Taken on 2026-11-24 as `TES-37`, since trashed. Linear linked no escaped
token, listed no lone number, and stored none of the inserted escapes; the
`0160\.` in `R11` and `X04`-`X06` is Linear's own, as in phase 2. The web UI
showed no backslash, `X21` included.

#### 3. README

**File**: `cli/work-adapters/tests/fixtures/linear-rewrites/escaped/README.md` (new)
**Changes**: The capture's date, identifier and method; a table of fragment,
pushed escapes and read-back form; and the web-UI check's date and result per
fragment. Record whether the read-back keeps any escape (`crates\.io`,
`0160\.`), since a later pull writes the stored form into the local file;
the probe corpus predicts it keeps none.

#### 4. Equivalence tests

**File**: `cli/corpus-adapters/tests/body_equivalence.rs`
**Changes**: Extend `Capture` to load the `escaped/` pair. Add:

- `each_escaped_fragment_is_equivalent_to_its_unescaped_fragment`: for every
  code in `pushed.md`, the `escaped/pushed.md` fragment is equivalent to it.
- `each_escaped_read_back_is_equivalent_to_its_unescaped_fragment`: for every
  code, the `escaped/read-back.json` fragment is equivalent to the
  `pushed.md` fragment.
- `the_escaped_capture_covers_every_fragment`: both files hold the same codes.

The second test is the automated half of "no ordered list, and no link other
than a scheme URL's", since PP-880's equivalence dissolves only links
whose text equals their destination.

### Success Criteria:

#### Automated Verification:

- [x] Escaped-capture equivalence passes: `mise run test:unit:cli --filter 'binary(body_equivalence)'`
- [x] `mise run` exits 0

#### Manual Verification:

- [x] `escaped/read-back.json` committed after the allowlist projection
- [x] Gate: the web UI shows no backslash before `.` or `)`, no stray list and
      no link on an escaped token; the `escaped/` README records it. If a
      backslash shows, stop and return the item to drafting. One narrower
      outcome stops only its context: a backslash in `X21` stops escaping
      text between inline HTML tags, and phase 5's rows for it change to
      match before it starts.
- [x] `X25`'s two URLs read back linked whole, as in phase 2
- [x] `\)` (`X20`) reads back without a backslash

---

## Phase 4: The digit rule

### Overview

Add `markdown::linear::escape_for_push` with the digit rule only, after
pinning the parser behaviour both rules depend on.

### Changes Required:

#### 1. Characterisation tests

**File**: `cli/markdown/src/linear/tests.rs`
**Changes**: Pin, against `document::markdown::events`, the stream both
rules read:

- `an_escaped_dot_splits_the_text_around_its_backslash`: `crates\.io` gives
  `MarkdownEvent::Text` ranges whose gap is exactly the backslash's byte.
  Write the expected ranges from what `document` over `pulldown-cmark`
  0.13.6 produces; the test exists so that a parser bump
  fails here with a precise message. If 0.13.6 instead ranges the escaped
  character as `\.`, phase 5 still treats it the same way: a `Text` whose
  source is `\` plus one ASCII punctuation character and whose text is that
  character continues the run, with the character marked escaped.
- `an_escaped_backslash_before_a_dot_leaves_the_dot_bare`: `crates\\.io`
  gives text `crates\.io`, with the `.` not escaped.
- `a_multi_line_inline_tag_arrives_as_written`: pin the events for
  `a <span title="owned by\n0160. x">t</span>\n`, so the digit rule's
  skip of breaks inside a tag is written against what 0.13.6 produces.
- `an_entity_yields_text_unlike_its_source`: `a&amp;b` gives text whose bytes
  differ from its source range. Phase 5 treats such text as a token boundary.

#### 2. Digit-rule tests first

**File**: `cli/markdown/src/linear/tests.rs`
**Changes**: An `assert_escapes(input, expected)` helper modelled on
`assert_unwraps` (`markdown/src/tests.rs:50-78`). For LF and CRLF it checks:

- **Rules.** `with_escapes(input, proposed_escapes(input))` gives
  `expected`, before any fallback. An unchanged case therefore asserts the
  rules propose nothing, so a rule that over-reaches into code or an
  attribute fails here even though the fallback would hide it.
- **Output.** `escape_for_push(input) == expected`.
- **Idempotence.** `escape_for_push(expected) == expected`.
- **Insertions only.** Walking both strings together, `input` is a
  subsequence of `expected` and every byte of `expected` not matched is `\`.
  This holds when the input already carries escapes.
- **Equivalence.** The test module's own normalised event streams of
  `input` and `expected` are equal, merging adjacent text as
  `normalised_events` does in `markdown/src/tests.rs:32-48`. It does not
  call the production `reads_alike`, so the oracle stays independent.

Cases, one test each:

| Input | Expected |
|---|---|
| `- owned by\n  0160.\n` | `- owned by\n  0160\.\n` |
| `- owned by\n  0160)\n` | `- owned by\n  0160\)\n` |
| `owned by\n0160.\n` | `owned by\n0160\.\n` |
| `owned by\n0160. more text\n` | `owned by\n0160\. more text\n` |
| `- owned by\n  0160. more text\n` | `- owned by\n  0160\. more text\n` |
| `- owned by\n  0160) more text\n` | `- owned by\n  0160\) more text\n` |
| `> owned by\n> 0160.\n` | `> owned by\n> 0160\.\n` |
| `- a\n  - owned by\n    0160)\n` | `- a\n  - owned by\n    0160\)\n` |
| `1. owned by\n   0160.\n` | `1. owned by\n   0160\.\n` |
| `- owned by\n  1234567890.\n` | `- owned by\n  1234567890\.\n` |
| `hard\\\n0160.\n` | `hard\\\n0160\.\n` |
| `hard  \n0160.\n` | `hard  \n0160\.\n` |
| `foo\n0160.\tbar\n` | `foo\n0160\.\tbar\n` |
| `- owned by\n\t0160.\n` | `- owned by\n\t0160\.\n` |
| `- [ ] owned by\n  0160.\n` | `- [ ] owned by\n  0160\.\n` |
| `- owned by\n  1.\n` | `- owned by\n  1\.\n` |
| `- owned by\n  0.\n` | `- owned by\n  0\.\n` |
| `- owned by\n  0160.\n  0161.\n` | `- owned by\n  0160\.\n  0161\.\n` |

The blockquote, top-level, ten-digit and text-after rows are over-escapes:
Linear lists none of them, and the escape reads back as plain text.

Unchanged cases (`assert_escapes(x, x)`): `foo\n1. text\n`, `1. a\n2. b\n`,
`foo\n2026.11 bar\n`, `0160.\n` (no preceding line), `foo\n0160\.\n`,
`[link\n0160. text](u)\n`, `![a\n0160. x](u)\n`, `` `a\n0160.` ``,
`    a\n    0160.\n`,
` ```\na\n0160.\n``` `, `<div>\na\n0160.\n</div>\n`,
`a <span title="owned by\n0160. x">t</span>\n`, `> owned by\n0160.\n`,
`- owned by\n0160.\n`.

The last two were drafted as escapes. `pulldown-cmark` 0.13.6 ends the
container at the unindented `0160.` and opens a list `160.`, with no break
before it, so CommonMark already reads the local file as a list. Linear's
read-back of probe `Q290` is the same list, so Linear changes nothing, and
the escape would change the CommonMark reading that the fallback protects.

#### 3. Implementation

**Files**: `cli/markdown/Cargo.toml`, `cli/markdown/src/lib.rs`,
`cli/markdown/src/github.rs`, `cli/markdown/src/linear.rs`,
`cli/markdown/src/linear/digit_rule.rs` (new)

```toml
document = { path = "../document" }
```

`markdown` reads Linear-bound Markdown through `document::markdown::events`,
so the escaper sees exactly the paragraphs, text and breaks PP-880's
equivalence sees. Both crates are `technical-library`, and `document` does
not depend on `markdown`.

```rust
// cli/markdown/src/linear.rs
mod digit_rule;
mod top_level_domains;

use std::ops::Range;

use document::markdown::{MarkdownEvent, events};

use crate::{Splice, spliced};

pub use top_level_domains::is_top_level_domain;

/// `markdown` with a backslash before each `.` or `)` that Linear's parser
/// would read as an ordered-list marker or part of a link.
///
/// Only backslashes are inserted. If any would change how `document` reads
/// the body, `markdown` is returned unchanged.
#[must_use]
pub fn escape_for_push(markdown: &str) -> String {
    escaped_or_original(markdown, proposed_escapes(markdown))
}

fn proposed_escapes(markdown: &str) -> Vec<Escape> {
    let events = events(markdown);
    digit_rule::escapes(markdown, &events)
}

fn escaped_or_original(markdown: &str, escapes: Vec<Escape>) -> String {
    let candidate = with_escapes(markdown, escapes);
    if reads_alike(markdown, &candidate) {
        candidate
    } else {
        markdown.to_owned()
    }
}

fn with_escapes(markdown: &str, mut escapes: Vec<Escape>) -> String {
    escapes.sort_unstable();
    escapes.dedup();
    spliced(markdown, escapes.into_iter().map(Escape::splice))
}

type Parsed = [(MarkdownEvent, Range<usize>)];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Escape {
    before: usize,
}

impl Escape {
    fn splice(self) -> Splice {
        Splice { replaced: self.before..self.before, with: "\\" }
    }
}
```

The splice loop moves out of `github::unwrap_soft_line_breaks` into a
crate-private helper in `lib.rs`, so both dialects share the crate's
"preserve every byte not deliberately changed" invariant. Refactor `github`
onto it first, with its tests green, then add the linear module:

```rust
// cli/markdown/src/lib.rs
pub(crate) struct Splice {
    pub(crate) replaced: Range<usize>,
    pub(crate) with: &'static str,
}

/// `markdown` with each splice applied, in ascending, non-overlapping order.
pub(crate) fn spliced(markdown: &str, splices: impl IntoIterator<Item = Splice>) -> String {
    let mut result = String::with_capacity(markdown.len());
    let mut preserved_from = 0;
    for splice in splices {
        result.push_str(&markdown[preserved_from..splice.replaced.start]);
        result.push_str(splice.with);
        preserved_from = splice.replaced.end;
    }
    result.push_str(&markdown[preserved_from..]);
    result
}
```

`reads_alike` compares the two bodies' `document` event streams with
adjacent text merged. It cannot be PP-880's equivalence itself, because
`markdown` is a `technical-library` and may not depend on `corpus`; the two
are kept in agreement on real bodies by phase 3's `body_equivalence.rs`
tests, which judge the escaped capture under PP-880. Unit-test it per event kind it must tell apart
(`Text`, `Code`, `InlineHtml`, `HtmlBlock`, `CodeBlock`, a link
destination), and test the fallback directly: `an_escape_that_changes_the_reading_is_abandoned` calls
`escaped_or_original` with an `Escape` inside a code span (where `\.` stays
literal) and expects the original back.

`digit_rule::escapes` walks `events.windows(2)`, tracking link and image
depth from `Start`/`End` events. For each `SoftBreak` or `HardBreak` outside
a link, it takes `line = continuation_start(markdown, break.end, next.start)`
and returns an `Escape` before the delimiter when `markdown[line..]` is one
or more ASCII digits, then `.` or `)`, then a space, a tab, `\r`, `\n` or
end of input. A code span or a code block cannot start with a digit, so it
needs no case of its own. Neither does a multi-line inline tag: 0.13.6
emits it as one `InlineHtml` event holding the line break
(`a_multi_line_inline_tag_arrives_as_written`), so no break event falls
inside an attribute value.

Run `mise run public-api:update`; the diff adds only `escape_for_push`.

⚠️ `cargo-deny`'s wrapper list (`cli/deny.toml:178`) is unchanged: `markdown`
keeps its direct `pulldown-cmark` dependency for `github`.

### Success Criteria:

#### Automated Verification:

- [x] Characterisation and digit-rule tests pass: `mise run test:unit:cli --filter 'package(markdown)'`
- [x] Public API snapshot matches: `mise run public-api:check`
- [x] `mise run cli:check` exits 0
- [x] `mise run` exits 0

#### Manual Verification:

- [x] None; nothing calls the escaper yet

---

## Phase 5: The dotted-token rule

### Overview

Add the dotted-token rule, with scheme URLs exempt, and prove the whole
escaper against both captures and the probe corpus.

### Changes Required:

#### 1. Tests first

**File**: `cli/markdown/src/linear/tests.rs`
**Changes**: Using `assert_escapes`, one test per row:

| Input | Expected |
|---|---|
| `lib.rs README.md a.b.io:12` | `lib\.rs README\.md a\.b\.io:12` |
| `config.yaml v1.2.3 e.g. a.b.c` | unchanged |
| `Crates.IO` | `Crates\.IO` |
| `a.b\.io` | `a\.b\.io` |
| `crates.io.` | `crates\.io.` |
| `(crates.io)` | `(crates\.io)` |
| `src/client.rs` | `src/client\.rs` |
| `a@b.io` | `a@b\.io` |
| `www.crates.io` | `www\.crates\.io` |
| `jql.rs:167 and jql.rs:x` | `jql\.rs:167 and jql\.rs:x` |
| `**crates.io**` | `**crates\.io**` |
| `# crates.io` | `# crates\.io` |
| `\| crates.io \|\n\| - \|\n` | `\| crates\.io \|\n\| - \|\n` |
| `https://crates.io/x.` | unchanged |
| `(see https://crates.io)` | unchanged |
| `mailto:a@b.io` | unchanged |
| `see https://crates.io/x. then crates.io` | `see https://crates.io/x. then crates\.io` |
| `(https://crates.io). then crates.io` | `(https://crates.io). then crates\.io` |
| `*https://crates.io*` | unchanged |
| `_https://crates.io_` | unchanged |
| `~https://crates.io~` | unchanged |
| `"https://crates.io/x"` | unchanged |
| `URL:https://crates.io` | unchanged |
| `HTTPS://crates.io ftp://crates.io` | unchanged |
| `https://crates.io>x` | unchanged |
| `ssh://github.com xhttps://crates.io` | `ssh://github\.com xhttps://crates\.io` |
| `https://crates.io). then crates.io` | `https://crates.io). then crates\.io` |
| `naïve — crates.io` | `naïve — crates\.io` |
| `café.io 日本.jp` | `café\.io 日本\.jp` |
| `foo.рф` | `foo\.рф` |
| `~crates.io` | `~crates\.io` |
| `www.foo` | `www\.foo` |
| `foo.md/x foo.md:1 a@b.md` | `foo\.md/x foo\.md:1 a@b\.md` |
| `Note:crates.io` | `Note:crates\.io` |
| `foo..bar.io` | `foo..bar\.io` |
| `PP-906:server.pid` | `PP-906:server\.pid` |
| `crates\\.io` | `crates\\.io` |
| `crates&#46;io` | `crates&#46;io` |
| `a@1.2.3.4` | `a@1\.2\.3\.4` |
| `1.2.3.4 127.0.0.1:8080` | unchanged |

Unchanged, one per construct: `` `crates.io` ``, a fenced block, an indented
block, `[crates.io](u)`, `[t](https://crates.io)`, `<https://crates.io>`,
`<div>\ncrates.io\n</div>\n`, `<a href="https://crates.io">t</a>` and
`crates\.io`. Text between inline tags is escaped:
`<span>crates.io</span>` escapes to `<span>crates\.io</span>`. An entity is a boundary:
`a&amp;crates.io` escapes to `a&amp;crates\.io`.

Labels hold Unicode letters, as Linear's do, so `café.io` is one token.
`www.foo` is escaped because `foo` is an IANA label. `ssh://` and `xhttps://`
are not schemes Linear links, so their hosts are ordinary tokens after `/`:
an over-escape. `README.md`, `src/client.rs`, `Note:crates.io`,
`foo..bar.io` and `<span>crates.io</span>` are over-escapes too.
Linear links `a@1.2.3.4` as `mailto:` (`S079`) but leaves a bare `1.2.3.4`
(`Q207`), so a numeric token straight after `@` is escaped too.

Then the capture tests, in `cli/work-adapters/tests/linear_captures.rs`
(new), beside the fixtures and the other capture consumers, reusing
`linear_rewrites.rs`'s fragment loader (move it to a shared test module):

- `each_fragment_escapes_to_its_hand_escaped_capture`: for every code,
  `escape_for_push(pushed[code]) == escaped[code]`.
- `the_whole_capture_escapes_to_the_hand_escaped_capture`:
  `escape_for_push(pushed.md) == escaped/pushed.md`, so headings and the
  bytes between fragments are covered and the file pushed in phase 3 is what
  the escaper produces.
- `every_token_linear_linked_is_escaped`: for every code, the linked tokens
  are a subset of the escaped tokens. **Escaped tokens** are the maximal
  dotted tokens in the escaped fragment holding an inserted `\.`.
  **Linked tokens** are, for each `[t](<http://t>)` or `[t](<mailto:t>)`
  link in the unescaped read-back that is not a scheme URL, the dotted
  tokens the rule finds in `t`. So a `mailto` link over `a@b.io` yields
  `b.io`, matching the escape in `a@b\.io`, and `X11` gives `jql.rs:167` on
  both sides. Tokens compare by host, without a port, since an existing
  `\:` ends Linear's link but not the rule's token (`S133`). A destination
  that punycodes a non-ASCII `t` (`Q150`) or has an empty host (`S167`)
  still links `t`. A link the pushed body already holds, such as
  `<a@b.io>` (`S081`), is not Linear's. Escaped tokens are found in the
  text `document` reads, adjacent text events merged, so emphasis
  (`_crates.io_`) and an existing escape (`a.b\.io`) do not hide a token.
- `every_unneeded_escape_is_a_recorded_over_escape`: each escaped token that
  is not linked falls in one class of `is_recorded_over_escape`, a test
  helper naming the classes the probe corpus found:
  - the last label is bare `md` or `merck`;
  - the token follows `-`, `_`, `:`, `@`, `/`, `\` or `.`;
  - the token is followed by `_`, `~`, `-`, `+`, `=`, `$`, `|`, `^` or `<`;
  - a label holds `_`, starts or ends with `-`, or exceeds 63 characters;
  - the token touches an inline HTML tag or an entity, or sits in
    strikethrough;
  - the token already holds an escape or follows an escaped character
    (`S122`, `S142`);
  - the last label holds a non-ASCII letter (`Q180`), which the rule counts
    as a TLD rather than converting it to punycode.
- `every_scheme_url_linear_linked_is_sent_unchanged`: each URL Linear linked
  whole (`R06`, `X25`) is byte-identical in the escaped fragment.

The same three tests run over every probe in
`linear-autolink-probes/` (`probes.md`, `follow-ups.md` and
`tld-sweep.md` against their read-backs), with two more:

- `every_lone_line_linear_listed_is_escaped`: each probe whose read-back
  turned a continuation line into a list item has a `\` before that line's
  `.` or `)` in the escaped probe. Linear stores such a line as an empty
  item, `N. ` with a trailing space, which CommonMark reads back as text,
  so the test looks for that line rather than for a list. A probe whose line `document` already
  reads as a list, such as `Q290`'s lazy line, is exempt.
- `every_unneeded_digit_escape_is_a_recorded_over_escape`: each other digit
  escape is in a blockquote, a top-level paragraph, a later paragraph of an
  item, a number of ten or more digits, or a line with text after the
  number.

`work-adapters` gains `markdown` as a dev-dependency.

The repository sweep lives in the `markdown` crate, where `proposed_escapes`
is visible: `every_work_item_body_escapes_safely` in
`cli/markdown/src/linear/tests.rs` reads every `meta/work/*.md` body through
`env!("CARGO_MANIFEST_DIR")/../../meta/work`, and for each asserts that
`escape_for_push` does not panic, is idempotent, makes insertions only, reads
alike, and that splicing `proposed_escapes` alone equals `escape_for_push`'s
output, so the fallback never fires on a real body. It asserts it read at
least 300 bodies (402 exist today), so a wrong path cannot pass vacuously,
and a failure names the offending work-item path, since new work items can
fail this lane without any code change. `proposed_escapes`, `with_escapes`
and `Escape` stay private to `markdown::linear`.

The CommonMark equivalence of each escaped fragment with its unescaped one is
already pinned by phase 3's `body_equivalence.rs` test, against the same file
these tests prove the escaper reproduces.

#### 2. Implementation

**Files**: `cli/markdown/src/linear/dotted_tokens.rs` (new),
`cli/markdown/src/linear/scheme_urls.rs` (new),
`cli/markdown/src/linear/text_runs.rs` (new), `cli/markdown/src/linear.rs`

`proposed_escapes` concatenates `digit_rule::escapes` and
`dotted_tokens::escapes`.

`dotted_tokens::escapes` builds **text runs**: maximal sequences of `Text`
events outside links, images and code blocks, with no other event between
them. Each character carries its source offset and whether a `\` immediately
precedes it in the source outside any `Text` range. A `Text` whose bytes
differ from its source slice ends the run. Within a run:

1. `scheme_urls::spans` marks the URLs Linear links whole: `http://`,
   `https://`, `ftp://` or `mailto:` in any case, not preceded by an ASCII
   letter or digit, and `//` at the run start or after whitespace. Each runs
   to whitespace, `<` or `>`, then repeatedly sheds a trailing
   `. , : ; ! ? " ' *` or an unbalanced `)` until neither applies. Marked
   characters are boundaries.
2. Words are maximal sequences of Unicode letters and digits
   (`char::is_alphanumeric`), `-`, `_`, `.` and `:`. Scanning a word from
   the left, a dotted token matches `label ( "." label )+ ( ":" digits )?`.
   A `.` not followed by a label character, or a `:` not followed by digits,
   ends any token in progress, and the scan resumes at the next label
   character, so `Note:crates.io` and `foo..bar.io` each hold one token.
3. If the last label passes `is_top_level_domain` or holds a non-ASCII
   character, or the token is all digits and dots and starts a word that
   follows `@`, each unescaped `.` in the token yields an escape.

Each step is its own pure function, unit-tested on plain strings as well as
through `escape_for_push`:

```rust
impl TextRun {
    fn from_events(markdown: &str, events: &Parsed) -> Vec<TextRun>;
}
impl SchemeUrl {
    fn spans(characters: &[RunCharacter]) -> Vec<Range<usize>>;
}
impl DottedToken {
    fn in_word(characters: &[RunCharacter]) -> Vec<DottedToken>;
}
```

`TextRun::from_events` owns the offset and escape-gap mapping; `escapes`
only composes the three. Expose the recogniser as
`markdown::linear::dotted_tokens(text: &str) -> impl Iterator<Item = Range<usize>>`
over plain text, so phase 9 needs no further API change, and
add one sentence to the crate's module docs: each dialect reads Markdown as
its comparison partner does, `github` with GitHub's options and `linear`
through `document` for PP-880's equivalence.

#### 3. Simulated autolinker stays in step

**File**: `cli/tracker-test-support/src/rewrite.rs`
**Changes**: None. `X11`, `X14` and `X16`-`X20` stay out of
`recorded_rewrites()`; the capture tests above cover them, and phase 9
brings the simulator onto this recogniser.

### Success Criteria:

#### Automated Verification:

- [x] Rule tests pass: `mise run test:unit:cli --filter 'package(markdown)'`
- [x] Capture and probe tests pass: `mise run test:unit:cli --filter 'binary(linear_captures)'`
- [x] `mise run cli:check` and `mise run public-api:check` exit 0
- [x] `mise run` exits 0

#### Manual Verification:

- [x] None; nothing calls the escaper yet

---

## Phase 6: Escaping every Linear description

### Overview

`create_op` and `update_op` escape every Markdown description, and
`--verbatim` on `accelerator linear create` and `update` opts out. Tests
prove that every push path and raw command escapes, `--verbatim` and other
trackers do not, and the local file stays byte-identical.

### Changes Required:

#### 1. Client tests first

**File**: `cli/linear-client/tests/update.rs`, `cli/linear-client/tests/`
create tests (add `description_dialect.rs` if none fits)
**Changes**: Against `MockGraphQLServer`, with `NEEDS_BOTH_ESCAPES =
"- owned by\n  0160.\n\nSee crates.io.\n"` and its escaped form:

- `create_op_escapes_a_markdown_description`
- `update_op_escapes_a_markdown_description`
- `create_op_and_update_op_send_a_verbatim_description_as_given`
- `remote_tracker_create_and_update_send_the_escaped_body`
- `transition_sends_no_description` (if `tests/transition.rs` does not
  already pin it)

Each reads `last_body("IssueCreate")` or `last_body("IssueUpdate")` and
compares `variables.input.description` byte for byte.

#### 2. Implementation

**Files**: `cli/linear-client/Cargo.toml`, `cli/linear-client/src/description.rs`
(new), `cli/linear-client/src/client.rs`, `cli/linear-client/src/update.rs`,
`cli/linear-client/src/lib.rs`

```toml
markdown = { path = "../markdown" }
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Description<'a> {
    Markdown(&'a str),
    /// For capturing what Linear's parser does to an unescaped body.
    Verbatim(&'a str),
}

impl Description<'_> {
    pub(crate) fn wire(self) -> Cow<'_, str> {
        match self {
            Self::Markdown(markdown) => Cow::Owned(markdown::linear::escape_for_push(markdown)),
            Self::Verbatim(text) => Cow::Borrowed(text),
        }
    }
}
```

`create_op` takes `description: Description<'_>` in place of `body: &str`,
and `UpdateFields.description` becomes `Option<Description<'a>>`; both send
`description.wire()`. `RemoteTracker::create` and `update` pass
`Description::Markdown(body)`. `Description` derives `PartialEq, Eq` so
`UpdateFields` keeps its derive list. Update the other callers of the two
signatures: `cli/linear-client/tests/discriminant.rs` (`:35`, `:57`, `:73`),
the `UpdateFields` literals in `tests/update.rs` and `tests/transition.rs`,
and `cli/linear-cli/src/main.rs`. `linear-client` has no public-API
snapshot, so there is nothing to regenerate.

#### 3. `--verbatim` on the raw commands

**Files**: `cli/linear-cli/src/cli.rs`, `cli/linear-cli/src/main.rs`,
`cli/linear-cli/tests/flow_create.rs`, `cli/linear-cli/tests/flow_update.rs`,
`cli/linear-cli/tests/fixtures/cli_surface.golden`
**Changes**: Tests first, each with `NEEDS_BOTH_ESCAPES`:

- `a_work_item_file_is_sent_escaped`: `create <work-item-file>`.
- `a_body_file_is_sent_escaped`: `create --title t --body-file <path>` and
  `update BLA-1 --body-file <path>`.
- `a_description_is_sent_escaped`: `update BLA-1 --description <text>`.
- `verbatim_sends_the_description_as_given`: each of the above with
  `--verbatim`.
- `verbatim_without_a_description_is_refused`: `update BLA-1 --title t
  --verbatim` and `create --title t --verbatim` each exit with clap's usage
  code `2` and send nothing.

`CreateArgs` and `UpdateArgs` gain:

```rust
/// Send the description exactly as given, without the escaping that keeps
/// Linear from reading dotted tokens as links or wrapped numbers as lists.
/// For capturing what Linear's parser does.
#[arg(long)]
pub verbatim: bool,
```

On `UpdateArgs` it carries `requires = "description_source"`, phase 1's
group. On `CreateArgs` it requires a separate group,
`ArgGroup::new("description_input").args(["file", "body_file"]).multiple(true)`,
which is only a requirement target: `resolve_create_inputs`
(`main.rs:397-427`) still decides what `create <file> --body-file x` means,
and `a_file_and_a_body_file_behave_as_before` pins that. One helper serves
both commands:

```rust
fn description(text: &str, verbatim: bool) -> Description<'_> {
    if verbatim { Description::Verbatim(text) } else { Description::Markdown(text) }
}
```

Regenerate `cli_surface.golden`. In
`update-linear-issue/SKILL.md` and `create-linear-issue/SKILL.md`, say the
description is escaped for Linear and that `--verbatim` exists only for
captures.

Future captures pass `--verbatim`. Phases 2 and 3 run before this phase, so
they need no flag; the hand-escaped capture would pass through the escaper
unchanged in any case.

#### 4. Sync and promotion push matrix

**File**: `cli/work-adapters/tests/sync_run_real_client.rs`
**Changes**: A push twin of
`every_pull_entry_point_writes_the_description_without_a_title_line`
(`:1446-1536`):

```rust
#[derive(Debug, Clone, Copy)]
enum PushEntryPoint {
    SyncUpdate,
    SyncCreateOfUnsyncedItem,
    PromotionCreate,
    PromotionRetitle,
}
```

`every_push_entry_point_escapes_for_linear_alone` loops over `Tracker` ×
`PushEntryPoint`. Each case seeds a local item whose body is
`NEEDS_BOTH_ESCAPES` and routes the tracker's create or update. Linear's
mock read-back returns the form `escaped/read-back.json` records for the
same constructs, plain `0160.` and unlinked `crates.io`, not the escaper's
own output. The test asserts:

- **Linear.** The sent `description` is the escaped body. For
  `PromotionRetitle`, the create request carries the escaped draft body, the
  retitle request (`promotion.rs:735-739` sends the body with the title)
  carries the escaped promoted body, and the promotion advances past its
  retitle stage.
- **Jira.** The sent ADF equals `jira-client`'s own Markdown-to-ADF
  conversion of `NEEDS_BOTH_ESCAPES`. This pins that Jira's sends are
  unchanged; it cannot show that no escape leaked, because the ADF tokeniser
  unescapes backslash escapes (`cli/adf/src/assemble.rs:407`). The Shortcut
  byte assertion is what catches a leak at a shared layer.
- **Shortcut.** The sent `description` equals `NEEDS_BOTH_ESCAPES`.
- **Local file.** Byte-identical to the file before the push, for every
  tracker.

For Linear, a following targeted sync over the same mock reports the item
`synced`. That shows the read-back is equivalent to the local body and that
the baseline digest is unchanged. Reuse the promotion harness
(`work-adapters/tests/promotion.rs:268-396`) and the pending-push helpers
(`sync_create.rs:4131-4187`) where the matrix's own fixtures fall short.

#### 5. `work create --push` and `work update --push`

**Files**: `cli/work-cli/src/create.rs` tests, `cli/work-cli/src/update.rs` tests
**Changes**: In-process tests using `PushingRepo` configured for each of
`linear`, `jira` and `shortcut`, and `FixedRegistry::holding` with a real
client over `MockHTTPServer`:

- `create_push_escapes_for_linear_alone`
- `update_push_escapes_for_linear_alone`
- `update_push_repairs_a_corrupted_remote`: the baseline's remote digest is
  taken over `[crates.io](<http://crates.io>)`, and `push_update` with no
  other flags sends the escaped body and leaves the file byte-identical.

If `PushingRepo` cannot build a real Linear client, add a `linear_client`
helper to `work-cli/src/test_support.rs` modelled on
`sync_run_real_client.rs:207-230`.

#### 6. CHANGELOG

**File**: `CHANGELOG.md`, under `[Unreleased]`
**Changes**:

- **Breaking.** `accelerator linear create` and `update` now escape the
  description so Linear does not read dotted tokens as links or wrapped
  numbers as ordered lists; pass `--verbatim` to send it as given.
- **Fixed.** Every push to Linear from `work`, `sync` and promotion escapes
  the same way.

Phase 1 adds an **Added** entry for `accelerator linear update --body-file`
and `E_UPDATE_NO_FILE` (exit 115); phase 7 adds one for the read-back
warning.

#### 7. Digests unchanged

No change. `work-adapters/tests/golden_digests.rs` must pass without a recipe
bump.

### Success Criteria:

#### Automated Verification:

- [x] Client tests pass: `mise run test:unit:cli --filter 'package(linear-client)'`
- [x] Push matrix passes: `mise run test:unit:cli --filter 'test(every_push_entry_point_escapes_for_linear_alone)'`
- [x] `work-cli` push tests pass: `mise run test:unit:cli --filter 'package(accelerator-work) & test(/push/)'`
- [x] Raw commands escape, and `--verbatim` opts out: `mise run test:unit:cli --filter 'package(linear-cli)'`
- [x] `CHANGELOG.md` holds the entries below
- [x] Golden digests unchanged: `mise run test:unit:cli --filter 'binary(golden_digests)'`
- [x] Dependency rules hold: `mise run test:integration:deny` and `mise run cli:check`
- [x] `mise run` exits 0

#### Manual Verification:

- [x] None before merge; phase 12 covers the live workspace

---

## Phase 7: Noticing a push the tracker rewrote

### Overview

Every push path reads the remote back and records it as the baseline, so a
rewrite at push time is silently agreed. This phase judges the read-back of
each body push against the body sent, under PP-880's equivalence, for every
tracker, and reports a rewrite as a notice that shows the changed lines. The
baseline is still recorded as before, so sync's decisions and exit codes do
not change. It needs no escaper and could land before phase 4, but follows
phase 6 so its Linear tests run against escaped pushes.

### Changes Required:

#### 1. Push fidelity, tests first

**Files**: `cli/work-adapters/src/sync/push_fidelity.rs` (new),
`cli/work-adapters/src/sync/body_comparison.rs`
**Changes**: Tests first, over `BodyComparison` with `PlainDialect`,
`CommonMarkBody` and, where named, Jira's dialect:

- `a_cosmetic_read_back_is_preserved`: `* one\n* two\n` read back as
  `- one\n- two\n`, and `See crates\.io.` sent but stored as
  `See crates.io.`, a dropped escape.
- `a_linked_dotted_token_is_a_rewrite`: `See crates.io.` read back as
  `See [crates.io](<http://crates.io>).` is `Rewritten`, with one
  `ChangedLine` holding the section, the sent line and the stored line.
- `a_stray_list_is_a_rewrite`: the `X09` fragment and its read-back give
  `Rewritten` with one section; the exact text is pinned by the line-pairing
  test below.
- `an_unread_remote_is_unjudged`: no projection gives `Unjudged`.
- `an_unrenderable_remote_is_unjudged`: a Jira projection whose ADF has no
  Markdown form gives `Unjudged`.
- `a_rejected_local_body_is_unjudged`: a body Jira's dialect rejects gives
  `Unjudged`, not `Rewritten`.

And for the line pairing, a pure function tested on its own:

- `only_lines_that_read_differently_are_reported`: a section holding a `*`
  bullet re-serialised as `-`, a paragraph between, and a `crates.io` linked
  by the tracker gives one `ChangedLine` whose `sent` is `See crates.io.`
  and whose `stored` is `See [crates.io](<http://crates.io>).`.
- `an_adjacent_cosmetic_line_rides_with_its_hunk`: the bullet directly above
  the linked line gives one `ChangedLine` holding both lines on each side,
  because `similar` joins them.
- `a_kept_escape_is_not_reported`: `crates.io` sent and `crates\.io` stored
  gives none.
- `a_stray_list_hunk_is_judged_in_context`: `X09`'s captured list
  (`pushed.md:108-110` against `read-back.json`) gives one `ChangedLine`
  whose `sent` is the three pushed lines and whose `stored` is the three
  stored lines, `*` bullets and the trailing space of `  160. ` included,
  although `  0160.` and `  160.` alone read alike.
- `a_blockquoted_stray_list_is_judged_in_context`: `> owned by\n> 0160.`
  sent and `> owned by\n> 160.` stored, with only the continuation line
  differing, gives one `ChangedLine`.
- `a_rewrite_with_no_surviving_hunk_reports_its_hunks`: the fallback is
  defensive, since no natural `Different` section is known to reach it, so
  it is tested directly: `changed_lines` under `PlainDialect`, given a
  section pair whose only hunk is cosmetic (a `*` bullet re-serialised as
  `-`), reports that hunk, and `meaningful_hunks`, the filter without the
  fallback, returns none for the same pair.
- `the_filter_alone_finds_every_captured_rewrite`: for every capture
  fragment pair that `push_fidelity` judges `Rewritten`, `meaningful_hunks`
  returns at least one hunk, so the fallback never fires on a real rewrite
  and a filter that drops one fails here.
- `a_rewritten_level_two_heading_is_one_change`: through
  `BodyComparison::push_fidelity` on whole bodies, `## Changes to config.rs`
  stored as `## Changes to [config.rs](<http://config.rs>)` gives exactly
  one `ChangedLine`, named for the heading's section, not a deleted section
  and an inserted one: an unmatched local section is paired with the
  unmatched remote section at the same position before `changed_lines`
  runs.
- `a_pure_delete_or_insert_is_a_hunk`: `changed_lines` given a pair whose
  only op is a `Delete`, and one whose only op is an `Insert`, reports one
  `ChangedLine` for each.
- `a_stray_list_without_a_leading_zero_reads_alike`: pins a known limit. A
  wrapped `42.` that Linear stores as a list item reads exactly as sent
  under CommonMark, which lets an ordered list interrupt a paragraph only
  at `1`, so `push_fidelity` returns `Preserved`. `X09` is caught only
  because Linear also drops the leading zero of `0160`. The escaper
  prevents this rewrite in every escaped push; a `--verbatim` push or a
  fallback can still produce it, and phase 12's audit pattern finds it.
- `jira_cosmetic_lines_are_not_reported`: under Jira's dialect, a section
  whose table alignment the ADF round trip drops and whose dotted token is
  linked gives one `ChangedLine`, for the link.

```rust
pub enum PushFidelity {
    Preserved,
    Rewritten { changes: Vec<ChangedLine> },
    Unjudged,
}

pub struct ChangedLine {
    pub section: String,
    pub sent: String,
    pub stored: String,
}

impl BodyComparison<'_> {
    pub fn push_fidelity(
        &self,
        local_content: &str,
        remote_projection: Option<&str>,
    ) -> PushFidelity;
}
```

`push_fidelity` lives on `BodyComparison`, which owns the dialect and the
equivalence it applies; push sites reach it through
`digester.comparison()`. It splits the frontmatter and returns `Unjudged`
for an absent projection or when `self.dialect.rejects(local_body)` holds,
since `equivalence_to_rendered` folds a rejected body into `Different`
(`body_comparison.rs:80-82`). Otherwise it calls `equivalence(local_body,
projection)`, maps `RemoteUnrenderable` to `Unjudged`, and on `Different`
builds the remote content with `LocalWorkItem::with_body`, takes
`differing_sections` (`work::section_diff::SectionDiff`, each side's text,
the pattern `sync/run.rs:968-979` uses), and passes each to
`changed_lines(section, comparison) -> Vec<ChangedLine>`. That function
aligns the two sides with `similar`'s line diff (already a `work-adapters`
dependency, `Cargo.toml:26`) and joins each replaced hunk, however many
lines each side holds, into one `sent`/`stored` pair. It keeps a pair only
if splicing the stored hunk into the sent section changes how the section
reads, with both the sent and the spliced section read through
`as_pushed` then the parser, the same reading that selected the section as
`Different` (`body_comparison.rs:104-105`). For Linear and Shortcut
`as_pushed` is identity; for Jira it is the ADF round trip, so a hunk that
round trip absorbs is never reported. A hunk is judged in its section,
never alone, because the digit rule's rewrite turns a continuation line
into a list item, and only the surrounding lines show that. Bullet markers,
code-span indentation and kept escapes, which PP-880 absorbs, are never
reported. If no hunk of a `Different` section survives, every replaced hunk
of that section is reported, so the warning always shows the text. The name avoids
`work::promotion::ReadBack`, which already means the recorded read-back.

#### 2. Push sites

A verdict is taken only from a read-back that directly follows a body push
that landed in the same run. Every other read-back gives `Unjudged`.

| Push path | Judged at | Against | `Unjudged` when |
|---|---|---|---|
| `sync` update | `apply.rs` `record_read_back` from the update steps (`:971-978`) | Local content pushed | The read fails |
| `sync` retitle | not judged: `retitle` sends the title alone on every client (`linear-client/src/client.rs:920-932`, `jira-client/src/client.rs:727-733`, `shortcut-client/src/client.rs:773-783`) | | Always |
| `sync` create of an unsynced item | `apply.rs:1646-1651` after a fresh create | Local content created | `UnorderedCreateDecision::ReuseId` (`apply.rs:1279-1281`), which reuses an earlier run's issue |
| `work create --push` | `creation.rs:851` after a fresh create | Content created | `PushNext::ReuseId` (`creation.rs:572-578`) |
| `work update --push` | `work-cli/src/update.rs:252-254` | Content pushed | The read fails |
| Promotion | `promotion.rs:752-757`, after a landed retitle | Promoted body sent | Every other route |

Every same-run promotion either retitles or is kept
(`work/src/promotion.rs:460-477`), so promotion is judged only by a landed
retitle; the create's own read (`promotion.rs:573-578`) is never judged.
The routes that give `Unjudged`:

- `Retitling::Keep(Edited)`: a person edited the remote after the create.
- `Retitling::Keep(NoHash)`, `UpdateFailed`: nothing was read, or the
  retitle did not land.
- `UserNamedAdopt`: this promotion pushed nothing.
- A promotion whose retitle landed in an earlier run.

`retitle_remote` returns the verdict to a local in `promote`'s stage loop
(`promotion.rs:398-448`); `Promoting` stays `&self`-only and nothing is
persisted, so `PromotionStage` does not change. The loop passes the verdict
explicitly to `retire` and on to `finish_promotion` (`promotion.rs:812-818`,
`:867-921`), which takes `fidelity: &PushFidelity` for phase 8's mark. Its
other callers, `already_promoted` (`:467`) and identity settlement
(`identity_settlement.rs:334`), pass `Unjudged`. `promote` returns the
verdict beside its `Result`, and `PromotionRow` gains a `fidelity` field
separate from `outcome`, so a `NotPromoted` row, such as a landed retitle
whose retirement then rewinds, still reports the notice. That case records
no mark: the retirement that finishes in a later run passes `Unjudged`.
`PromotionRow::of`'s callers change with it: `work-cli/src/create.rs:691`,
`work-cli/src/promote.rs:274` and `identity_settlement.rs:500`.

The `read_back_of` helpers at `promotion.rs:353` and `creation.rs:663-676`
return the `RemoteIssue` with the digest, so each verdict comes from the
same read as its baseline rather than from a second `show`.

#### 3. Reporting

**Files**: `cli/work-adapters/src/sync/report.rs`,
`cli/work-adapters/src/creation.rs` (`CreationOutcome`),
`cli/work-cli/src/report_records.rs`, `cli/work-cli/src/sync.rs`,
`cli/work-cli/src/create.rs`, `cli/work-cli/src/create_batch.rs`,
`cli/work-cli/src/update.rs`
**Changes**: `CreateNotice` stays as it is. `ReportedItem` gains a separate
field, `rewrite: Option<RewriteNotice>`, so a `--resolve <id>=create` with
no pending push (`apply.rs:1344-1349`) can carry both `NothingToSettle` and
a rewrite. `RewriteNotice` is data in `work-adapters`: a struct with a
`kind` (`OnPush`, or from phase 8 `StillOnPush`) and one bounded value,
`RewrittenLines { lines: Vec<ChangedLine>, more: usize }`, built once from
the verdict by `RewrittenLines::bounded(changes)`: at most `SHOWN_LINES`
(five) lines, each side cut to `LINE_BYTES` (200) bytes on a character
boundary with a trailing `…` when cut, and a count of the rest. Phase 8's mark holds the same value, so the first
notice and every later one show identical text, and the renderer does no
capping of its own. Each carries
the item's path and its external id, which `work-adapters` knows
(`knowledge.item(id)`, `run.rs:1677`), so the renderer needs no lookup of
its own. It is a notice on an applied action, not a
`RunFailure`, so no `FailureClass` or exit code changes. `CreationOutcome`
carries the verdict out of `creation.rs`, as `PromotionOutcome` does for
promotion.

The one renderer lives in `work-cli`'s `report_records.rs`, beside the
existing notice formatting (`sync.rs:948-972`), and passes every
interpolated value through `single_line` (`report_records.rs:36`), because
the stored lines come from the remote:

```rust
pub fn rewrite_lines(notice: &RewriteNotice, label: &str, tracker: &str) -> Vec<String>;
```

`label` is the id in sync output and the path elsewhere; the notice's path
fills the clearing command. A push prints:

```text
warning: <label>: rewritten on push: <section>, <section>
note: <label>: <section>: sent `<sent>`, stored `<stored>`
note: <label>: the local file is unchanged, but <tracker> stored these lines with a different meaning
note: <label>: to keep the meaning, put the sent text in backticks or reword it, then run `accelerator work update <path> --push`
note: <label>: please report it at https://github.com/atomicinnovation/accelerator/issues with the lines above
```

One `<section>: sent …, stored …` line is printed per `ChangedLine`, up to
five, then `note: <label>: … and <n> more changed lines; \`accelerator
<tracker> show <external id>\` shows the stored description`. The skills name where
that command puts the description (`.data.issue.description` for Linear,
the ADF at `.fields.description` for Jira, `.description` for Shortcut). Code spans are
never rewritten, so backticking or rewording the sent text makes the next
push faithful, which clears phase 8's mark. A pull also clears the mark, but
only by writing the tracker's rewritten form into the local file; the skills
say so rather than suggest it. `create-batch` prints the block once per
item.

Tests first. The `work-adapters` tests assert the `RewriteNotice` data on
`ReportedItem` and the outcome types; the `work-cli` tests render a report
or run a command and assert what is printed, since only `work-cli` holds the
renderer and the existing `--resolve` warning (`work-cli/src/sync.rs:965-968`).

In `work-adapters`, extending phase 6's matrix:

- `every_push_entry_point_notices_a_rewritten_read_back`: over `Tracker` ×
  `PushEntryPoint`, with a mock read-back that links `crates.io` whatever was
  sent, the item carries exactly one `OnPush` notice with the item's path
  and a `ChangedLine` naming the sent and stored lines, the baseline is
  recorded, and the run's exit code and failures are what they would be
  without it.
- `every_push_entry_point_is_silent_for_a_faithful_read_back`: over
  `Tracker` × `PushEntryPoint`, with Linear returning
  `escaped/read-back.json`'s form, Jira returning the ADF its client built
  from the body, and Shortcut returning the body, no notice is set.
- `an_unreadable_read_back_is_neither_noticed_nor_fatal`: a failing `show`
  after the push gives no notice and the push succeeds.
- `a_create_with_nothing_to_settle_carries_both_notices`: a
  `SyncCreateOfUnsyncedItem` under `--resolve <id>=create` with no marker,
  read back rewritten, carries both `NothingToSettle` and a rewrite.
- `a_title_only_retitle_is_unjudged` and `a_reused_create_is_unjudged`: a
  sync retitle, a sync `ReuseId` create and a `work create --push`
  `ReuseId` each give no `OnPush` notice, with a mock read-back that would
  otherwise count as rewritten.
- `a_promotion_is_judged_by_its_landed_retitle`: a rewritten retitle
  read-back gives one notice; a rewritten create read-back followed by a
  faithful retitle read-back gives none.
- `a_promotion_kept_or_adopted_is_unjudged`: `Keep(Edited)`, `Keep(NoHash)`,
  `UserNamedAdopt` and `UpdateFailed` each give no notice and leave any mark
  unchanged.
- `a_promotion_finished_elsewhere_is_unjudged`: finishing through
  `already_promoted`, identity settlement or a retirement resumed in a later
  run gives no rewrite notice and leaves a previously marked entry marked.
- `a_rewound_retirement_still_reports_its_notice`: a rewritten retitle whose
  retirement fails carries the notice on its `NotPromoted` row and records
  no mark; when the retirement finishes in the next run, the entry is still
  unmarked.

Also in `work-adapters`:

- `rewritten_lines_are_bounded`: a 200-byte side is left whole, a 201-byte
  side is cut with `…`, a side with `—` across byte 200 is cut at byte 199,
  and twelve lines give five and `more: 7`.

In `work-cli`:

- `a_multi_line_change_renders_on_one_line`: a notice built from `X09`'s
  three-line hunk prints exactly one `<section>: sent …, stored …` line,
  and every line of the block starts with `warning:` or `note:`.
- `the_overflow_note_names_the_external_id`: with more than five lines,
  the overflow note's `show` command names the external id, not the local
  id.
- `a_create_with_nothing_to_settle_prints_both_notices`: rendering that
  report prints the existing `--resolve` warning and the rewrite block.
- `every_note_carries_its_warnings_subject`: for one and for two rewritten
  items, every `note:` line has the subject of the `warning:` above it.
- `a_clearing_command_names_the_item_path`: in sync output, the `note:`
  starting `to keep the meaning` holds the item's real relative path, not
  its id.
- `changed_lines_are_capped_at_five`: table-driven over five (no overflow
  line), six (`… and 1 more`) and seven (`… and 2 more`) changed lines.
- `create_push_warns_when_the_read_back_is_rewritten`,
  `create_batch_warns_once_per_rewritten_item` and
  `update_push_warns_when_the_read_back_is_rewritten`: stderr carries the
  block above for each item, every `note:` with the warning's subject, the
  exit code is `0`, and the baseline is recorded.

#### 4. Skills and CHANGELOG

**Files**: `skills/work/create-work-item/SKILL.md` (its `write-once` row and
step 8 confirmation), `skills/work/extract-work-items/SKILL.md` (its batch
table), `skills/work/sync-work-items/SKILL.md` (a dispatch entry for the
notice), `tests/unit/tasks/test_sync_work_items_skill_structure.py`,
`CHANGELOG.md`
**Changes**: Each skill relays the `warning:` and `note:` lines verbatim in
its summary, quoted as data, and says the push succeeded. Add
`` `warning: <id>: rewritten on push:` `` to
`test_each_record_type_has_a_dispatch_entry`'s list; phase 8 adds
`` `warning: <id>: still rewritten on push:` ``. Add an **Added** entry for
the notice.

### Success Criteria:

#### Automated Verification:

- [ ] Verdict tests pass: `mise run test:unit:cli --filter 'package(work-adapters) & test(/preserved|unjudged|_a_rewrite/)'`
- [ ] Push-site and promotion tests pass: `mise run test:unit:cli --filter 'package(work-adapters) & test(/notices_a_rewritten_read_back|silent_for_a_faithful_read_back|unreadable_read_back|both_notices|reused_create|title_only_retitle|promotion|rewound_retirement/)'`
- [ ] Line pairing and bounding pass: `mise run test:unit:cli --filter 'test(/read_differently|rides_with_its_hunk|kept_escape|judged_in_context|no_surviving_hunk|filter_alone|level_two_heading|delete_or_insert|without_a_leading_zero|jira_cosmetic|lines_are_bounded/)'`
- [ ] Rendering and `work-cli` warnings pass: `mise run test:unit:cli --filter 'package(accelerator-work) & test(/warns|both_notices|its_warnings_subject|clearing_command|changed_lines|on_one_line|external_id/)'`
- [ ] Skill structure holds: `mise run test:unit:tasks --filter test_each_record_type_has_a_dispatch_entry`
- [ ] `mise run cli:check` exits 0
- [ ] `mise run` exits 0

#### Manual Verification:

- [ ] None; phase 12 runs against the live workspace

---

## Phase 8: Remembering a rewritten push

### Overview

Phase 7's notice appears only on the run whose push caused the rewrite. This
phase records it on the item's baseline entry as a **rewrite mark**, until a
push is stored faithfully or the local file and the remote are found to
agree, and surfaces it in `work list` and every later `sync`. The mark is
meant to persist until the body is fixed; there is no command to dismiss it,
and the skills say so.

### Changes Required:

#### 1. Baseline entry, tests first

**File**: `cli/work-adapters/src/sync/baseline.rs`
**Changes**: The mark is stored as one object, with keys alphabetical at
every level:

```json
"rewritten_on_push":{"lines":[{"section":"Notes","sent":"See crates.io.","stored":"See [crates.io](<http://crates.io>)."}],"more":7}
```

Tests first:

- `a_rewrite_mark_round_trips`: an `Entry::Split` with a `RewriteMark`
  renders exactly the literal above and parses back equal.
- `an_entry_without_the_mark_renders_as_before`: the rendered bytes of an
  unmarked entry equal a literal pre-change rendering.
- `an_older_entry_reads_as_unmarked`: an entry with no
  `"rewritten_on_push"` key parses with `None`.
- `mark_text_is_rendered_as_json_strings`: sections and lines holding `"`,
  `\`, a tab, a newline and non-ASCII text round-trip, and the rendered
  baseline parses as JSON.
- `a_malformed_mark_is_kept_raw`: a `"rewritten_on_push"` value without a
  `lines` array, or whose line objects lack `section`, `sent` or `stored`
  as strings, parses as `MarkState::Unreadable`, keeping the entry's
  digests, and a `Keep` write renders it back unchanged. A missing or
  non-integer `more` reads as `0`.
- `known_keys_are_not_unknown`: parsing then rendering a split entry with
  one genuinely unknown key renders every known key exactly once.
- `a_new_mark_replaces_a_malformed_one`: over a malformed raw mark, a
  `Rewritten` write renders exactly one `"rewritten_on_push"` key, holding
  the new mark, and a `Clear` write renders none.
- `extra_mark_fields_are_kept`: an extra field on the mark object or on a
  line object round-trips with the mark, so a later build's addition is not
  erased.
- `a_mark_is_bounded`: a verdict with twelve changed lines, one holding a
  line whose `—` spans bytes 199-201, persists exactly
  `RewrittenLines::bounded`'s output: five lines, `"more": 7`, and that line
  cut at byte 199 with a trailing `…`.
- `unknown_split_keys_survive_a_rewrite`: an entry holding a key this build
  does not know, one sorting before `local_body_hash`, renders it back in
  its alphabetical place.

`Baseline::render` builds JSON by hand, and an unparseable baseline reads as
empty (`baseline.rs:386-406`), so every string goes through the existing
`render_string` (`serde_json::to_string`), never concatenation.

The split variant becomes a newtype, `Entry::Split(SplitEntry)`.
`SplitEntry` keeps today's `stamps`, `legacy` and `split` behind accessors
and adds two private fields:

```rust
pub enum MarkState {
    Absent,
    Marked(RewriteMark),
    Unreadable(Value),
}
```

`mark: MarkState` holds the three states a stored mark can be in, so a
malformed `rewritten_on_push` is a state of its own rather than a raw value
parked among the unknown keys, and `render_split` writes the key from one
place. `unknown: BTreeMap<String, Value>` holds every key of the entry
object except the fixed set `render_split` writes (`digest_recipe`, the
five split digests, `local_hash`, `remote_hash`, `local_synced_at`,
`remote_updated_at`) and `rewritten_on_push`, so no key is ever rendered
twice. Rust makes enum variant fields public, so only a struct with private
fields stops another module setting or dropping a mark around the rule.

`SplitEntry` offers two transforms that carry both private fields through:
`with_local_fingerprints` (for `realign.rs:242-252`) and
`without_local_digests` (for `baseline.rs:158-172`). The watermark advance
(`baseline.rs:483-488`, through `stamps_mut`) and a rename
(`baseline_store.rs:145-151`, which moves the entry to a new key) keep the
entry whole and need none. `created_baseline.rs:34-38` and the
`baseline_store.rs:281` test build through `Entry::recorded`. Every
named-field `Entry::Split { .. }` pattern in the workspace moves to the
existing `Entry::split()`, `stamps()` and `legacy()` accessors
(`baseline.rs:103-129`) or to `Entry::Split(_)`: in `baseline.rs` (`:311`,
`:369`, `:542`, `:890`), `realign.rs:234`, `sync/fetch.rs:373`,
`sync/run.rs:684`, `sync/send_refusal.rs:24`, `tests/support/mod.rs:525`,
`tests/sync_realign.rs:253`, `tests/sync_equivalence.rs` (`:176`, `:395`,
`:483`, `:538`, `:923`, `:1009`, `:1071`), and `work-cli/src/create.rs:1463`,
`sync.rs:6490` and `update.rs:718`.

`render_split` merges the mark and the unknown keys into its alphabetical
key order (`baseline.rs:347-351`), so two builds writing the same entry
produce the same bytes. `RewriteMark { lines: Vec<MarkedLine>, more,
extra }`, with `MarkedLine { line: ChangedLine, extra }`, holds phase 7's
bounded lines plus each level's fields this build does not know, and
`RewriteMark::rewritten_lines()` gives the renderer its `RewrittenLines`,
so `ChangedLine` and `RewrittenLines` carry no `serde_json::Value`. That
keeps remote text added to the committed `last-sync.json` small. The key
`rewritten_on_push` belongs to `MarkState`: a malformed value parses as
`Unreadable`, which `Keep` carries unchanged and every other change
replaces, as §2 describes. `Entry::recorded` keeps its signature and records no mark.
`Entry::Legacy` holds no mark: a push recorded under the legacy recipe
reports its notice but cannot persist it.

`DigestRecipe::CURRENT` does not change: the mark is not a digest. A build
older than this phase ignores the key on read, but re-renders every entry
through a fixed key set (`baseline.rs:339-364`, `:444-456`), and
`last-sync.json` is committed. So one sync by such a build drops every mark
in the shared file, though no item's state.

#### 2. Setting, keeping and clearing

The mark follows one rule, in one pure function, and every write chooses
how it applies:

```rust
impl RewriteMark {
    fn after(fidelity: &PushFidelity, previous: Option<&Self>) -> Option<Self>;
}

pub enum MarkChange<'a> {
    Judged(&'a PushFidelity),
    Fresh(&'a PushFidelity),
    Clear,
    Keep,
}
```

`Rewritten` gives `Marked`, `Preserved` gives `Absent`, and `Unjudged`
keeps `previous`; an `Unreadable` previous counts as no mark for `after`,
and only `Keep` carries it unchanged. `Fresh` is `after(fidelity, None)`, for a create, including a
`--resolve <id>=create` that relinks an item to a new issue (`create_anew`,
`apply.rs:1482-1496`), so the old issue's mark never carries over.

Two mechanisms carry the mark and unknown keys, one for each way the
baseline is written:

- **Per-item writes** go through `BaselineStore::record(id, entry,
  MarkChange)`. It loads the previous entry once, resolves the change and
  copies the previous entry's unknown keys onto the new one. The writers
  are `ItemApplier::set_baseline` (`apply.rs:1109-1127`), which takes a
  required `MarkChange`, `record_created_baseline`
  (`created_baseline.rs:19-42`), and `work update --push`, which today calls
  `BaselineStore::set` directly (`work-cli/src/update.rs:256-270`).
  `BaselineStore::set` is renamed `seed`, and `Baseline::set`
  (`baseline.rs:458`) and `BaselineStore::replace` (`baseline_store.rs:75`)
  become `pub(crate)`; their only production caller is realign
  (`realign.rs:163`, `:181`), in the same crate. `seed` and a public
  `Baseline::seed` are compiled only with a new `test-support` feature of
  `work-adapters`, enabled through a self dev-dependency
  (`work-adapters = { path = ".", features = ["test-support"] }`, as
  `config` and `store` do, `store/Cargo.toml:15-26`) and by `work-cli`'s
  dev-dependencies. The seeders that call `set` today
  (`work-adapters/tests/support/mod.rs:155`, `tests/retirement.rs:202-204`,
  `tests/sync_settled.rs:193-207`, `work-cli/src/sync.rs:6421`, `:6876`,
  and `work-cli/src/list.rs`'s tests) keep working.
  `tasks/lint/config_test_support.py` becomes a table of guarded features,
  each with the crates allowed to enable it in `[dev-dependencies]`;
  `work-adapters/test-support` is allowed in `work-adapters` and
  `work-cli` only, with the same forwarding checks. Because the lint and
  test lanes build with `--all-features` (`tasks/lint/cli.py:7`,
  `tasks/test/cli.py:59`), phase 8's success criteria add a featureless
  `cargo check -p work-adapters -p work-cli`, which fails if production
  code calls `seed`.
- **Whole-document rewrites** keep the fields inside the `Entry` transform,
  because they write every entry in one all-or-none step and cannot load and
  write per item. `realigned_entry` and `without_local_digests` use
  `SplitEntry`'s two transforms, which carry `mark` and `unknown` unchanged
  by construction; the watermark advance and a rename keep the whole entry.

The classifier's knowledge of whether it compared bodies reaches `apply` as
a domain value, in `work::sync::classify`:

```rust
pub enum BodyAgreement {
    Agreed,
    Unconfirmed,
}

pub struct Classified {
    pub classification: Classification,
    pub body_agreement: BodyAgreement,
}
```

`classify` returns `Result<Classified, kernel::Error>` in place of
`Result<Classification, _>` (`classify.rs:539-544`). `Agreed` comes from
`recover`'s `Synced(RecordOnly)` branch (`classify.rs:370-374`; its
`Conflict` branch is `Unconfirmed`) and from the moved-but-equivalent
branch (`:462-463`), whichever reconciliation follows, `RecordOnly` or
`SendFields`. Every other route is `Unconfirmed`, including bodies that
were compared and differ: the name says the mark cannot be cleared, not
that nothing was compared. `plan()` (`plan.rs:287-320`) copies it onto the
`PlannedAction` with an additive builder, so `PlannedAction::new(id,
state, action)` keeps its signature and defaults to `Unconfirmed`:

```rust
impl PlannedAction {
    pub fn with_body_agreement(self, agreement: BodyAgreement) -> Self;
    pub fn body_agreement(&self) -> BodyAgreement;
}
```

Resolved, ordered and relinking actions keep `Unconfirmed`. `RecordRequest`
(`apply.rs:785-790`) and `RetitleRequest` (`apply.rs:513-518`) each gain an
`agreement: BodyAgreement` field, filled from `planned.body_agreement()` by
`record_outcome` (`run.rs:1608-1633`) and `send_outcome`
(`run.rs:1506-1511`) respectively. Carrying the agreement rather than a
`MarkChange` means neither request can hold a push verdict. Inside the
applier, one function, `MarkChange::after_reconciliation(BodyAgreement)`,
maps `Agreed` to `Clear` and `Unconfirmed` to `Keep`.
`ItemApplier::record_read_back` (`apply.rs:939-979`) is split into
`record_pushed_read_back`, which passes `Judged(verdict)`, and
`record_retitled_read_back`, which passes `after_reconciliation` of its
request's agreement, so the shared funnel never guesses.
`Action` and the reconciliation enums do not change.

| `MarkChange` | Writes |
|---|---|
| `Judged(verdict)` | `sync` update; `work update --push`; promotion's `finish_promotion` |
| `Fresh(verdict)` | `sync` create of an unsynced item; `work create --push`; a relinking `--resolve <id>=create` |
| `Clear` | Pull; discovered-remote authoring; equivalent adoption; a conflict resolved to the remote; a `RecordBaseline` or `sync` retitle whose `body_agreement` is `Agreed` |
| `Keep` | A `RecordBaseline` or `sync` retitle whose `body_agreement` is `Unconfirmed` |

Tests first:

- `rewrite_mark_after_follows_the_verdict`: a unit test of the three rows of
  `RewriteMark::after`, and that `Fresh` ignores a previous mark.
- `a_rewritten_push_marks_the_entry` and `a_faithful_push_clears_the_mark`:
  real tests, each run over every `Judged` and `Fresh` row through
  `every_push_entry_point_*` and the `work-cli` push tests, asserting the
  persisted mark.
- `an_unjudged_push_keeps_the_mark`, over the `sync` update and
  `work update --push` rows, which have a previous entry.
- `a_fresh_create_starts_unmarked`: the create rows, including a relinking
  `--resolve <id>=create` over a marked entry whose read fails, leave no
  mark unless the verdict is `Rewritten`.
- `every_clearing_write_clears_the_mark`: table-driven from a marked entry
  over pull, authoring, equivalent adoption, conflict-to-remote, `recover`,
  a remote fixed by hand (`Synced(RecordOnly)` via `:463`), a local
  frontmatter edit beside a remote fixed by hand (`LocallyModified` via
  `:463`), and a local title edit beside a remote fixed by hand (a retitle
  via `SendFields`).
- `every_keeping_write_keeps_the_mark`: table-driven from a marked entry
  over a local frontmatter-only edit, a remote fields-only change, a retitle
  with no body moved, realign, blanking local digests, a `finalise_run`
  watermark advance and a rename. Fetched-legacy syncs hold no mark and are
  left out.
- `unknown_keys_survive_every_write`: from an entry holding an unknown key,
  table-driven over one `Judged`, one `Fresh`, one `Clear` and one `Keep`
  write and every row of the keeping table's whole-document rewrites, each
  leaving the key in the rendered file.
- `a_legacy_entry_reports_the_rewrite_without_persisting_it`: the notice is
  printed and the legacy entry renders byte for byte as before.

`classify`, `PlannedAction` and the new types are part of `work`'s public
API. Run `mise run public-api:update` and check that the diff adds
`BodyAgreement`, `Classified`, `with_body_agreement` and `body_agreement`,
and changes `classify`'s return type. Update `classify`'s callers: `plan()`,
`IntendedBaseline::next_sync` (`work/src/promotion.rs:148-162`), which
keeps its `Result<SyncState, _>` signature by reading
`classified.classification.state()`, `cli/work/tests/sync_classify.rs` and
`cli/work-adapters/tests/sync_recipe_migration.rs:745`.
`PlannedAction::new`'s callers in `work-cli/src/sync.rs` and `finaliser.rs`
need no change.

#### 3. Surfacing

**Files**: `cli/work-cli/src/list.rs`, `cli/work-adapters/src/sync/run.rs`,
`skills/work/list-work-items/SKILL.md` (its "five-state label vocabulary"
sentence gains the suffix), `skills/work/sync-work-items/SKILL.md`,
`tests/unit/tasks/test_sync_work_items_skill_structure.py`, `CHANGELOG.md`
**Changes**: A run's notice for an item comes from its verdict and the mark
the run leaves: `Rewritten` gives an `OnPush` notice; any other outcome that
leaves a mark gives a `StillOnPush` notice; no mark gives nothing. Tests
first:

- `list_shows_a_marked_item_as_rewritten_on_push`: the sync column reads
  `🟢 synced · rewritten on push`. The suffix is composed in `list.rs`;
  `sync_label` and `work-item-sync-label.golden` keep their five states.
- `sync_notices_a_marked_item_on_every_run`: a sync that does not push the
  item prints `warning: <id>: still rewritten on push: <sections>`, its
  `sent …, stored …` lines, the overflow `note:` when the mark's `more` is
  above zero, and the clearing `note:`; the other two notes appear only on
  the push.
- `a_marked_item_with_an_unjudged_push_is_still_noticed`: a push whose read
  fails keeps the mark and prints exactly one `StillOnPush` block.
- `a_marked_item_re_pushed_still_rewritten_gets_one_notice`: one
  `OnPush` notice with the new lines, and no `StillOnPush` one.
- `a_marked_item_pulled_in_the_run_gets_no_notice`: the mark is cleared and
  nothing is printed.

The skills relay both blocks, give the clearing step (backticks or
rewording, then `accelerator work update <path> --push`), say a pull clears
the mark only by adopting the tracker's form, and say the mark has no
dismiss command because it persists until the body is fixed. Add
`` `warning: <id>: still rewritten on push:` `` to the skill structure
test. In `CHANGELOG.md`, add the mark to the **Added** entry, and a
**Migrations** bullet: everyone who syncs a repository should upgrade
together, because a sync by an older build drops every `rewritten_on_push`
mark in the committed `last-sync.json`.

### Success Criteria:

#### Automated Verification:

- [ ] Baseline tests pass: `mise run test:unit:cli --filter 'package(work-adapters) & test(/rewrite_mark|mark_text|unknown_split_keys|older_entry|malformed_mark|replaces_a_malformed|extra_mark_fields|mark_is_bounded|renders_as_before/)'`
- [ ] Mark rules pass: `mise run test:unit:cli --filter 'test(/mark_after|marks_the_entry|clears_the_mark|keeps_the_mark|starts_unmarked|unknown_keys_survive|legacy_entry_reports/)'`
- [ ] Public API snapshots match: `mise run public-api:check`
- [ ] No production code calls `seed`: `cargo check -p work-adapters -p work-cli` with no features
- [ ] The test-support guard holds: `mise run test:unit:tasks --filter test_support`
- [ ] Surfacing tests pass: `mise run test:unit:cli --filter 'test(/rewritten_on_push|still_noticed|marked_item/)'`
- [ ] `mise run cli:check` exits 0
- [ ] `mise run` exits 0

#### Manual Verification:

- [ ] None; phase 12 runs against the live workspace

---

## Phase 9: The simulator reads tokens as the escaper does

### Overview

`Rewrite::AutolinkDottedTokens` finds tokens with phase 5's recogniser and
leaves bare exactly the tokens the probe corpus shows Linear leaves bare. So
one grammar serves the escaper, its oracle and the simulator, and the
simulator models ports, multi-label names and boundaries.

### Changes Required:

#### 1. Tests first

**File**: `cli/work-adapters/tests/linear_rewrites.rs`
**Changes**: Extend `recorded_rewrites()` with `X11`, `X14`, `X16`, `X17`
and `X18` mapped to `Rewrite::AutolinkDottedTokens`. `X11` fails because the
simulator does not carry the port, `X14` because it takes one dot-free name,
and `X16` and `X17` because the trailing `.` and the brackets join the word.
`X18` passes today and pins the `/` boundary. Update the doc comment: only
`X09`, `X19`-`X25` stay out, since the simulator works line by line and
models neither `mailto` links nor emphasis, headings, tables or HTML.

Add `the_simulator_links_what_linear_linked_in_every_dotted_token_probe`:
for every probe in `linear-autolink-probes/` in the groups `ext`,
`md-name`, `rs-name`, `before`, `after`, `case`, `labels`, `numeric` and
`ports-paths`, and every line of `tld-sweep.md`, the link texts the
simulator produces equal the link texts in Linear's read-back. Link texts
rather than whole lines, because Linear's other escapes (`\_`, `\*`, `\[`)
are not dotted-token rewrites.

#### 2. One model of what Linear leaves bare

**Files**: `cli/tracker-test-support/src/linear_autolinker.rs` (new),
`cli/tracker-test-support/src/rewrite.rs`,
`cli/work-adapters/tests/linear_captures.rs`
**Changes**: Phase 5's `is_recorded_over_escape` test helper moves into
`tracker-test-support` as `pub fn linear_leaves_bare(text: &str, token:
Range<usize>) -> bool`, with `LEFT_BARE_BY_LINEAR` folded into it.
`linear_captures.rs`'s oracle calls it instead of its own copy, so the
oracle and the simulator cannot drift apart. Two of its classes describe
escapes rather than Linear: a token already holding an escape, and a
non-ASCII last label, which the escaper counts as a TLD. Before folding them
in, settle what the simulator does with a non-ASCII last label: Linear left
`crates.ioé` bare (`Q180`), but the probes pushed no IDN top-level domain
such as `рф`.

#### 3. Implementation

**File**: `cli/tracker-test-support/src/rewrite.rs`
**Changes**: `autolink_dotted_tokens` wraps each range of
`markdown::linear::dotted_tokens(line)` for which `linear_leaves_bare` is
false as `[t](<http://t>)`. `autolink_urls` keeps `link_words`; the
dotted-token path stops using it. `cli/pup.ron` already permits
`markdown::linear`.

### Success Criteria:

#### Automated Verification:

- [ ] Simulator and probe tests pass: `mise run test:unit:cli --filter 'binary(linear_rewrites)'`
- [ ] The oracle still passes on the shared model: `mise run test:unit:cli --filter 'binary(linear_captures)'`
- [ ] `mise run cli:check` and `mise run pup:check` exit 0
- [ ] `mise run` exits 0

#### Manual Verification:

- [ ] None; test support only

---

## Phase 10: Escaping Linear comments

### Overview

Capture whether Linear's comment parser rewrites Markdown as its description
parser does. If it does, `LinearClient::add_comment` escapes comment bodies
as `create_op` and `update_op` escape descriptions, and `--verbatim` on
`accelerator linear comment add` opts out.

### Changes Required:

#### 1. Live capture (manual)

With the `TES` procedure from the Implementation Approach:

1. `accelerator linear create --verbatim --title "PP-906 comment capture"
   --body-file <a one-line stub>`. Confirm the identifier starts `TES-`.
2. `accelerator linear comment add <identifier> --verbatim --body-file
   cli/work-adapters/tests/fixtures/linear-rewrites/pushed.md`, then the
   same with `escaped/pushed.md`. `--verbatim` does not exist on `comment
   add` until this phase lands, so before then omit it; the body is sent as
   given either way.
3. `accelerator linear show <identifier> --comments 2`, projected through
   the allowlist extended with `comments: {nodes: [.comments.nodes[].body]}`,
   into `linear-rewrites/comment-read-back.json`. Record the extended
   allowlist in `../work-item-sync-recovery/README.md`.
4. Delete the issue, then restore `config.local.md`.

#### 2. Gate

Compare each fragment of the first comment with the same fragment of
`read-back.json`, and each fragment of the second with
`escaped/read-back.json`. The README records the result per fragment.

| Outcome | Then |
|---|---|
| Comments are read as descriptions are | Sections 3-5 below |
| Comments are rewritten, but differently | Return this phase to drafting with the capture |
| No meaning-changing rewrite in comments | Record it; drop sections 3-5; the phase ends here |

#### 3. Equivalence tests

**File**: `cli/corpus-adapters/tests/body_equivalence.rs`
**Changes**: Load `comment-read-back.json` in `Capture`. Add
`each_escaped_comment_fragment_reads_alike_after_linear`, the comment twin
of phase 3's escaped read-back test.

#### 4. Client and command

**Files**: `cli/linear-client/src/comment.rs`,
`cli/linear-client/src/description.rs`, `cli/linear-cli/src/cli.rs`,
`cli/linear-cli/src/main.rs`, `cli/linear-cli/tests/`,
`cli/linear-cli/tests/fixtures/cli_surface.golden`,
`skills/integrations/linear/comment-linear-issue/SKILL.md`
**Changes**: First rename phase 6's `Description` to `MarkdownBody`, since
it now carries comments too, with every test green. Then, tests first:

- `add_comment_escapes_a_markdown_body` and
  `add_comment_sends_a_verbatim_body_as_given`, against `MockGraphQLServer`,
  reading `variables.input.body` byte for byte.
- `a_comment_body_is_sent_escaped` and `verbatim_sends_the_comment_as_given`
  in a `linear-cli` flow test, for both `--body` and `--body-file`.

`add_comment` takes `body: MarkdownBody<'_>` and sends `body.wire()`.
`comment add` gains `--verbatim`, with the same doc text as `create` and
`update`. Regenerate the surface golden, and say in the skill that comments
are escaped for Linear.

#### 5. CHANGELOG

**File**: `CHANGELOG.md`
**Changes**: A **Changed** entry under `[Unreleased]`: Linear comments are
escaped as descriptions are, and `comment add --verbatim` sends one as
given.

### Success Criteria:

#### Automated Verification:

- [ ] Comment equivalence passes: `mise run test:unit:cli --filter 'binary(body_equivalence)'`
- [ ] Client and command tests pass: `mise run test:unit:cli --filter 'package(linear-client) | package(linear-cli)'`
- [ ] `mise run cli:check` exits 0
- [ ] `mise run` exits 0

#### Manual Verification:

- [ ] Comment capture taken, allowlisted, committed and its issue trashed
- [ ] Gate outcome recorded in the README

---

## Phase 11: Whether decoded entities change meaning

### Overview

Linear decodes `&lt;`, `&gt;`, `&amp;` and `&#46;` on store (`Q262`-`Q264`).
A pull then writes the decoded form into the local file, where CommonMark may
read it differently: `&lt;b&gt;` stored as `<b>` would be a live tag. Probe
which entities change meaning, and decide the remedy.

### Changes Required:

#### 1. Probe body

**File**: `cli/work-adapters/tests/fixtures/linear-autolink-probes/entities.md` (new)
**Changes**: One `#### E<nnn> entities` probe per case: `&lt;b&gt;bold&lt;/b&gt;`,
`&lt;!-- c --&gt;`, `&lt;https://crates.io&gt;`, `&amp;lt;b&amp;gt;`,
`&#42;emph&#42;`, `&#95;emph&#95;`, `&#91;t&#93;(https://x.io)`, `&#96;code&#96;`,
`&#124;` in a table cell, and at a line start `&#35; heading`, `&#45; item`
and `&#49;. item`. Also `&nbsp;` and `&copy;`, plus the backslash forms `\<b>`,
`\*emph\*` and `\# heading` as candidate remedies.

#### 2. Live capture (manual)

With the `TES` procedure, `accelerator linear create --verbatim --title
"PP-906 entity probe" --body-file …/entities.md`, `show` through the
allowlist into `entities.read-back.json`, then delete the issue and restore
`config.local.md`. Before phase 6 lands there is no `--verbatim`, and
none is needed: the body is sent as given.

#### 3. Tests

**File**: `cli/corpus-adapters/tests/body_equivalence.rs`
**Changes**: `each_entity_probe_reads_as_recorded`: for every `E` probe,
the pushed and read-back forms are equivalent, except the probes the README
lists as changing meaning, which must differ. A future change in Linear's
decoding fails here.

#### 4. README and decision

**File**: `cli/work-adapters/tests/fixtures/linear-autolink-probes/README.md`
**Changes**: An "Entities" section: each probe's stored form and whether it
reads alike. Then, by outcome:

| Outcome | Then |
|---|---|
| Every probe reads alike | Record it; the non-goal stands |
| Some change meaning, and a backslash form survives as text | Raise a work item to substitute it on push; this breaks the escaper's insertions-only invariant, so it needs its own design |
| Some change meaning, and no form survives | Raise a work item to report on push, as phase 7 reports a rewrite |

### Success Criteria:

#### Automated Verification:

- [ ] Entity probe test passes: `mise run test:unit:cli --filter 'binary(body_equivalence)'`
- [ ] `mise run` exits 0

#### Manual Verification:

- [ ] Probe taken, allowlisted, committed and its issue trashed
- [ ] README records each probe and the decision; any work item it calls for is raised

---

## Phase 12: Post-merge verification

### Overview

Find and repair every Linear item whose remote description still carries
either rewrite, then confirm the repairs in the web UI. Run once by hand with
the released build, after that release has reached everyone who pushes to
the `PP` team, since an older build re-corrupts any item it pushes. Record
the outcome in the PR.

### Steps

The steps run four bash scripts, written once into the gitignored
`.accelerator/tmp/pp-906-repair/` and run from the repository root with
`bash`, whatever the operator's shell, so `read`, globbing, word splitting
and `pipefail` behave the same under zsh. Each starts with `set -euo
pipefail` and an `ERR` trap that names the failing line, checks every
condition with an explicit `if`, and ends by printing what it found.

1. Write `fetch.sh` and `refetch.sh`. `fetch.sh` saves the remote
   description of each id in a list into a directory that must not already
   exist, and rejects anything that is not a `PP-` id. `refetch.sh` fetches
   a list, then retries its failed reads a bounded number of times, each
   into a fresh `<dir>.retry-<n>`, merging what each retry saved into
   `<dir>`. It exits non-zero unless every id ends up saved or not found:

   ```bash
   mkdir -p .accelerator/tmp/pp-906-repair
   cat > .accelerator/tmp/pp-906-repair/fetch.sh <<'SCRIPT'
   set -euo pipefail
   trap 'echo "fetch: failed at line $LINENO: $BASH_COMMAND" >&2' ERR
   ids=$1; out=$2
   if [ -e "$out" ]; then echo "fetch: $out exists; choose a new directory" >&2; exit 1; fi
   mkdir -p "$out"; : > "$out/failed-reads.txt"; : > "$out/not-found.txt"
   while read -r id; do
     if ! [[ $id =~ ^PP-[0-9]+$ ]]; then echo "fetch: not an id: '$id'" >&2; exit 1; fi
     if accelerator linear show "$id" > "$out/$id.json.tmp" < /dev/null; then
       mv "$out/$id.json.tmp" "$out/$id.json"
     elif jq -e '.outcome == "not-found"' "$out/$id.json.tmp" > /dev/null 2>&1; then
       rm -f "$out/$id.json.tmp"; echo "$id" >> "$out/not-found.txt"
     else
       rm -f "$out/$id.json.tmp"; echo "$id" >> "$out/failed-reads.txt"
     fi
   done < "$ids"
   SCRIPT
   cat > .accelerator/tmp/pp-906-repair/refetch.sh <<'SCRIPT'
   set -euo pipefail
   trap 'echo "refetch: failed at line $LINENO: $BASH_COMMAND" >&2' ERR
   shopt -s nullglob
   here=$(dirname "$0"); ids=$1; out=$2; attempts=5
   bash "$here/fetch.sh" "$ids" "$out"
   n=1
   while [ -s "$out/failed-reads.txt" ] && [ "$n" -le "$attempts" ]; do
     sleep $((n * 10))
     retry="$out.retry-$n"
     bash "$here/fetch.sh" "$out/failed-reads.txt" "$retry"
     for json in "$retry"/*.json; do mv -n "$json" "$out/"; done
     sort -u "$out/not-found.txt" "$retry/not-found.txt" -o "$out/not-found.txt"
     cp "$retry/failed-reads.txt" "$out/failed-reads.txt"
     n=$((n + 1))
   done
   total=$(grep -c . "$ids" || true)
   saved=$(find "$out" -maxdepth 1 -name '*.json' | wc -l | tr -d ' ')
   missing=$(grep -c . "$out/not-found.txt" || true)
   failed=$(grep -c . "$out/failed-reads.txt" || true)
   echo "refetch: ids $total saved $saved not-found $missing failed $failed"
   cp "$ids" "$out/ids.txt"
   if [ "$failed" -gt 0 ]; then echo "refetch: $failed ids still unread after $attempts retries" >&2; exit 1; fi
   if [ $((saved + missing)) -ne "$total" ]; then echo "refetch: counts do not add up" >&2; exit 1; fi
   SCRIPT
   ```

   List the Linear-synced items and take the pre-repair backup:

   ```bash
   backup=.accelerator/tmp/pp-906-repair
   rg -l '^external_id: "PP-' meta/work \
     | xargs rg -o --no-filename --replace '$1' '^external_id: "(PP-[0-9]+)"' \
     > "$backup/ids.txt"
   bash "$backup/refetch.sh" "$backup/ids.txt" "$backup/before"
   ```

   Not-found items cannot be read and are listed in the PR. Keep `before/`
   until step 6 passes, and name its location in the PR.
2. Write `audit.sh`. It proves its patterns against the captures, refuses a
   directory whose fetch is incomplete, and lists the affected ids.
   `accelerator linear show` nests the description at
   `.data.issue.description`. An item is affected if its description has a
   link whose text holds a bare dotted token (an `http` or a `mailto` link),
   or a line holding only a number and delimiter after indentation or a
   blockquote marker, which is how Linear stores the stray list (`  160. ` in
   `X09`). Copy the four code lists from the "Audit self-check codes"
   section of the `linear-rewrites` README into the script:

   ```bash
   cat > .accelerator/tmp/pp-906-repair/audit.sh <<'SCRIPT'
   set -euo pipefail
   trap 'echo "audit: failed at line $LINENO: $BASH_COMMAND" >&2' ERR
   dir=$1; expect_none=${2:-}
   linked_codes=(R12 X10 X11 X14 X15 X16 X17 X19 X22 X23)
   listed_codes=(X09 X20)
   faithful_linked_codes=(R12 X10 X11 X14 X15 X16 X17 X19 X22 X23)
   faithful_listed_codes=(R11 X09 X20 X24)
   link='\[([A-Za-z0-9._%+-]+@)?[A-Za-z0-9_-]+(\.[A-Za-z0-9_-]+)+(:[0-9]+)?\]\(<(http|mailto):'
   lone='^[[:space:]>]*[0-9]+\.[[:space:]]+$'
   fixtures=cli/work-adapters/tests/fixtures/linear-rewrites
   for fixture in "$fixtures/read-back.json" "$fixtures/escaped/read-back.json"; do
     if [ ! -f "$fixture" ]; then echo "audit: fixture not found: $fixture; run from the repository root" >&2; exit 1; fi
   done
   for list in ids.txt failed-reads.txt not-found.txt; do
     if [ ! -f "$dir/$list" ]; then echo "audit: $dir/$list missing; fetch with refetch.sh" >&2; exit 1; fi
   done
   if [ -s "$dir/failed-reads.txt" ]; then echo "audit: $dir has unread items in failed-reads.txt" >&2; exit 1; fi
   description() { jq -r '.data.issue.description // empty' "$1"; }
   fragment() { description "$1" | awk -v code="$2" '/^## /{inside = ($2 == code)} inside'; }
   expect() {
     local file=$1 code=$2 pattern=$3 want=$4 name=$5 text got
     text=$(fragment "$file" "$code")
     if [ -z "$text" ]; then echo "self-check failed: $code not found in $file" >&2; exit 1; fi
     if printf '%s\n' "$text" | rg -e "$pattern" > /dev/null; then got=match; else got=none; fi
     if [ "$got" != "$want" ]; then echo "self-check failed: $name in $code: expected $want, got $got" >&2; exit 1; fi
   }
   for code in "${linked_codes[@]}"; do expect "$fixtures/read-back.json" "$code" "$link" match link; done
   for code in "${listed_codes[@]}"; do expect "$fixtures/read-back.json" "$code" "$lone" match lone; done
   for code in "${faithful_linked_codes[@]}"; do expect "$fixtures/escaped/read-back.json" "$code" "$link" none link; done
   for code in "${faithful_listed_codes[@]}"; do expect "$fixtures/escaped/read-back.json" "$code" "$lone" none lone; done
   shopt -s nullglob
   files=("$dir"/*.json)
   total=$(grep -c . "$dir/ids.txt" || true)
   missing=$(grep -c . "$dir/not-found.txt" || true)
   if [ $((${#files[@]} + missing)) -ne "$total" ]; then
     echo "audit: $dir holds ${#files[@]} saved and $missing not-found of $total ids" >&2; exit 1
   fi
   affected=0
   for json in "${files[@]}"; do
     if ! jq -e . "$json" > /dev/null; then echo "audit: $json is not JSON" >&2; exit 1; fi
     text=$(description "$json")
     if printf '%s\n' "$text" | rg -e "$link" -e "$lone" > /dev/null; then
       basename "$json" .json; affected=$((affected + 1))
     fi
   done
   echo "audit: self-check ok, ${#files[@]} read, $affected affected" >&2
   if [ "$expect_none" = --expect-none ] && [ "$affected" -gt 0 ]; then exit 1; fi
   SCRIPT
   backup=.accelerator/tmp/pp-906-repair
   bash "$backup/audit.sh" "$backup/before" > "$backup/affected.txt.tmp" \
     && mv "$backup/affected.txt.tmp" "$backup/affected.txt"
   ```

   `affected.txt` exists only if the audit passed, so a failed audit can
   never be read as an empty list. The first two loops prove each pattern
   still matches every fragment the unescaped capture records for it; the
   last two prove neither matches the escaped capture's faithful forms, so
   a repaired item is not reported as still corrupted. A missing fragment
   fails rather than passing silently. Linear stores a sent `0160\.` or
   `0160\)` as plain `0160.` or `0160)`, so `lone` matches only the form
   Linear writes for a stray list item: `N.` with a trailing space, zeros
   stripped, at any indent including a lazy line at column 0. Every stray
   item in the probe corpus has that form and no faithful line does.
   For each lone-number hit, confirm the local body has a wrapped
   continuation line there; Linear drops leading zeros, so compare by
   position, not by digits.
3. Record every item checked, each one found, each not-found item, the
   audit's `self-check ok` line and the commands run in the PR description.
4. Write `repair.sh`, which repairs one item by id. It validates the id,
   finds the item's path, appends to its own log, refuses an item it has
   already repaired, and checks, pushes and verifies, re-reading the remote
   straight before the push so an edit made during the review is not
   overwritten:

   ```bash
   cat > .accelerator/tmp/pp-906-repair/repair.sh <<'SCRIPT'
   set -euo pipefail
   if [ $# -ne 1 ] || ! [[ $1 =~ ^PP-[0-9]+$ ]]; then echo "usage: repair.sh PP-<n>" >&2; exit 2; fi
   backup=.accelerator/tmp/pp-906-repair
   id=$1
   if [ -f "$backup/$id.out" ] && grep -qx "repaired $id" "$backup/$id.out"; then
     echo "repair: $id is already repaired" >&2; exit 1
   fi
   exec > >(tee -a "$backup/$id.out") 2>&1
   fail() { echo "not repaired $id: $1"; exit 1; }
   trap 'fail "line $LINENO: $BASH_COMMAND"' ERR
   noop_synced() { printf '%s\n' "$1" | rg -q "^$id"$'\t'"noop"$'\t'"synced"; }
   matches=$(rg -l "^external_id: \"$id\"$" meta/work || true)
   [ "$(printf '%s' "$matches" | grep -c .)" = 1 ] || fail "expected one work item, found: $matches"
   path=$matches
   [ -s "$backup/before/$id.json" ] || fail "no backup"
   backed_up=$(jq -r .data.issue.updatedAt "$backup/before/$id.json")
   unchanged_since_backup() {
     local now
     now=$(accelerator linear show "$id" < /dev/null | jq -er .data.issue.updatedAt) || fail "could not read the remote"
     [ "$now" = "$backed_up" ]
   }
   preview=$(accelerator work sync --target "$id" --preview) || fail "preview failed"
   noop_synced "$preview" || fail "preview reports an action: $preview"
   unchanged_since_backup || fail "edited since the backup"
   { awk 'NR==1 && /^---$/ {print; inside=1; next}
          inside {print; if (/^---$/) exit}' "$path"
     jq -er '.data.issue.description' "$backup/before/$id.json"
   } > "$backup/$id.remote.md" || fail "could not build the remote copy"
   accelerator work diff "$path" "$backup/$id.remote.md" || fail "work diff failed"
   printf 'Every section differs only by the identified rewrites? [y/N] ' > /dev/tty
   read -r answer < /dev/tty
   [ "$answer" = y ] || fail "diff not confirmed"
   unchanged_since_backup || fail "edited during review"
   cp "$path" "$backup/$id.local.md"
   accelerator work update "$path" --push > "$backup/$id.push.log" 2>&1 || fail "push failed; see $id.push.log"
   if rg -q 'rewritten on push' "$backup/$id.push.log"; then fail "still rewritten on push"; fi
   cmp "$backup/$id.local.md" "$path" || fail "local file changed"
   after=$(accelerator work sync --target "$id" --preview) || fail "post-repair preview failed"
   noop_synced "$after" || fail "post-repair preview reports an action: $after"
   echo "repaired $id"
   SCRIPT
   ```

   Run it for each affected id: `bash .accelerator/tmp/pp-906-repair/repair.sh
   PP-180`. A `not repaired` line names why; record the item for manual
   reconciliation and move on. The post-repair check is a preview, so it can
   never write the local file after the `cmp`.
5. Fetch the repaired items afresh into their own directory and audit that,
   leaving `before/` untouched. With `--expect-none`, the audit fails if it
   lists any id:

   ```bash
   backup=.accelerator/tmp/pp-906-repair
   rg -o --no-filename --replace '$1' '^repaired (PP-[0-9]+)$' "$backup"/*.out \
     | sort -u > "$backup/repaired.txt"
   bash "$backup/refetch.sh" "$backup/repaired.txt" "$backup/after"
   bash "$backup/audit.sh" "$backup/after" --expect-none
   ```
6. View PP-180, PP-733, PP-860, PP-870 and each repaired item in Linear's web
   UI.
7. Record in the PR the minimum plugin version pushers must run, and raise a
   follow-up to re-audit a week after the repair, so any item an older build
   re-corrupted is found. From the repository root, regenerate `ids.txt`
   with step 1's `rg` pipeline, then run `bash
   .accelerator/tmp/pp-906-repair/refetch.sh
   .accelerator/tmp/pp-906-repair/ids.txt
   .accelerator/tmp/pp-906-repair/reaudit-<date>` and `bash
   .accelerator/tmp/pp-906-repair/audit.sh
   .accelerator/tmp/pp-906-repair/reaudit-<date> --expect-none`.

### Success Criteria:

#### Manual Verification:

- [ ] Every repaired item's description holds neither rewrite, its local file
      is byte-identical, and its next sync reports it `synced`
- [ ] PP-180, PP-733, PP-860, PP-870 and each repaired item show no stray
      list, no link on a dotted token and no backslash before `.` or `)`
- [ ] The PR lists every item checked, each one found, each unreadable one,
      each one skipped for manual reconciliation, where the saved remote
      descriptions live, and the minimum plugin version
- [ ] The re-audit follow-up is raised

---

## Testing Strategy

### Unit Tests:

- **Parser characterisation.** `MarkdownEvent::Text` ranges around a
  backslash escape and an escaped backslash, and entity text.
- **Digit rule.** Every container, unbounded digit runs, `.` and `)`,
  following text, and the negatives: interrupting lists, `2026.11`, code,
  links, HTML and existing escapes.
- **Dotted-token rule.** Every class and boundary shape, case, partial
  escapes, scheme URLs and their trailing punctuation, numeric email hosts,
  and inline markup; each of its steps on plain strings.
- **Properties.** On every case: LF and CRLF, idempotence, and output minus
  inserted backslashes equals input.
- **TLD snapshot.** Shape, size floor, and known members and non-members.

### Integration Tests:

- **Captures.** The escaper reproduces `escaped/pushed.md`; over both
  captures and the probe corpus, it escapes every token Linear linked and
  every lone number it listed, sends every scheme URL unchanged, and makes
  only recorded over-escapes; escaped fragments and their read-backs are
  equivalent to the unescaped fragments.
- **Repository sweep.** Every `meta/work` body escapes without the fallback
  firing.
- **Push paths.** The `Tracker` × `PushEntryPoint` matrix, plus the
  `work-cli` create and update push tests, over real clients.
- **Rewrite notice and mark.** Every push entry point notices a rewritten
  read-back once and stays silent for a faithful or unreadable one, on
  every tracker; the baseline mark is set, kept and cleared as specified,
  and surfaces in `work list` and later syncs.
- **Raw commands.** `linear create` in both modes and `linear update` with
  `--description` or `--body-file` send escaped descriptions, `--verbatim`
  sends them as given, and the two update flags conflict.

### Manual Testing Steps:

1. Phase 2 capture and its TLD gate.
2. Phase 3 capture and its web-UI gate.
3. Phase 10 comment capture and its gate.
4. Phase 11 entity probe and its decision.
5. Phase 12 search, repair and web-UI check.

## Performance Considerations

⏱️ Each Linear push parses the body three more times (escape, then the
fallback's reading of both bodies), and each push to any tracker parses the
local and remote bodies once more for the read-back check. Bodies are a few
KB and pushes are network-bound, so the cost does not register. The TLD set is
built once per process from the ~1,450-line snapshot.

## Migration Notes

There is no data migration: baseline entries without `rewritten_on_push`
read as unmarked. A sync by a build older than phase 8 drops every mark in
the committed `last-sync.json`; marks survive only while everyone runs a
build that knows them. Corrupted remotes are repaired one by one in phase 12.
Refreshing the TLD snapshot later can change which tokens are escaped. A
token Linear newly links is still agreed into the baseline, so it never
surfaces as a sync conflict; it surfaces as phase 7's notice on the push
that sent it, and phase 8's mark keeps it visible in `work list` and every
later sync until a faithful push, a pull, or a sync that finds the bodies
equivalent clears it. The fix is a new
capture fragment, a snapshot refresh and a re-push.

PP-876's description-sending path reaches Linear through `update_op`
whichever lands first, so it inherits the escape. PP-885 already owns the `markdown` crate;
no second parser crate is added.

## References

- Original work item: `meta/work/PP-906-linear-push-corrupts-markdown-content.md`
- Codebase research: `meta/research/codebase/2026-11-23-PP-906-linear-push-corrupts-markdown-content.md`
- PP-885 plan (the `markdown` crate and its test pattern): `meta/plans/2026-10-13-PP-885-push-pr-descriptions-to-github-unwrapped.md`
- PP-880 plan (equivalence and digests): `meta/plans/2026-10-10-PP-880-sync-round-trip-defects.md`
- Offset-splicing model: `cli/markdown/src/github.rs:19-62`
- Send site: `cli/linear-client/src/client.rs:892-918`
- Capture: `cli/work-adapters/tests/fixtures/linear-rewrites/README.md`
- Allowlist: `cli/work-adapters/tests/fixtures/work-item-sync-recovery/README.md`
