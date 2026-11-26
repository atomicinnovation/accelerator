---
type: "codebase-research"
id: "2026-11-23-PP-906-linear-push-corrupts-markdown-content"
title: "Where and how to escape Linear-bound Markdown on push (PP-906)"
date: "2026-11-23T17:39:16+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "PP-906"
parent: "work-item:PP-906"
topic: "Where and how to escape Linear-bound Markdown on push (PP-906)"
tags: ["research", "codebase", "sync", "linear", "markdown", "linear-client", "markdown-crate", "body-dialect"]
revision: "97f17b737cfe6dbb8ae2123d757473b01b7afd19"
repository: "accelerator"
last_updated: "2026-11-23T18:05:00+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Added follow-up research on the open questions: repair route for the affected items and the 0184 damage"
schema_version: 1
---

# Where and how to escape Linear-bound Markdown on push (PP-906)

**Date**: 2026-11-23T17:39:16+00:00
**Author**: Toby Clemson
**Git Commit**: 97f17b737cfe6dbb8ae2123d757473b01b7afd19 (not pushed; local
references only)
**Branch**: detached working copy (jj)
**Repository**: accelerator

## Research Question

What in the codebase does PP-906 touch, what can it reuse, and what constrains
where a push-side Linear escaper lives and how it is tested?

## Summary

The escaper fits in three places, and nothing else needs to move:

1. **`markdown::linear`** in `cli/markdown/` holds the escape. The crate exists
   because of PP-885, has landed, and is one of only two crates `deny.toml`
   lets depend on `pulldown-cmark` (`=0.13.6`). It already rewrites by byte
   offset and keeps every other byte, as the escaper must.
2. **`impl RemoteTracker for LinearClient`** applies it. Its `create` and
   `update` (`cli/linear-client/src/client.rs:892-918`) are the only route
   by which any work-sync path reaches Linear. The raw `accelerator linear`
   commands call `create_op` and `update_op` directly, so they stay verbatim
   without extra code. Jira and Shortcut have their own impls and are
   untouched.
3. **`BodyDialect::as_pushed` stays identity for Linear.** An inserted `\.`
   or `\)` resolves away in the parse, so the canonical body, the
   equivalence and every digest are unchanged. No recipe bump is expected;
   the golden-digest and equivalence tests should prove it.

The work item has four factual errors:

- **`accelerator linear update` has no `--body-file`.** It takes
  `--description <STRING>` (`cli/linear-cli/src/cli.rs:58-73`), so one
  acceptance criterion names a flag that does not exist.
- **Pending-push adoption sends no body.** Both adopt paths only compare
  bodies. The sync path that does send one is the pending-push create
  (`create_from_local`).
- **`work update --push` is a fifth push path** the item does not list
  (`cli/work-cli/src/update.rs:243`). A wrapper-level escaper covers it
  anyway.
- **The four named items may not be conflicted.** PP-880's plan records
  PP-180, PP-733, PP-860 and PP-870 (0161, 0203, 0276, 0293) reporting
  `synced` since 2026-10-14, because an earlier sync baked their rewrites
  into the baseline. `--resolve <id>=local` cannot settle a non-conflict,
  so the post-merge repair route needs re-checking.

## Detailed Findings

### Push paths and the choke point

Every work-sync push resolves a `Box<dyn RemoteTracker>` from
`ConfiguredTrackers::resolve` (`cli/work-cli/src/tracker_registry.rs:301-337`;
Linear at `:322-325`) and calls `create` or `update` on it.

| Path | Entry point | Send call | Sends body |
|---|---|---|---|
| `work create --push`, local ID | `work-adapters/src/creation.rs:802` → `push_legacy_item` (`:558`) | `remote_create::send_create` → `tracker.create` (`remote_create.rs:43`) | yes |
| Draft promotion | `work-adapters/src/promotion.rs:398` → `create_issue` (`:541`), then `retitle_remote` (`:697`) | `tracker.create`, then `tracker.update` (`:732-739`) | yes, twice |
| `sync` update | `work-adapters/src/sync/run.rs:1491` → `apply.rs:917` `push` | `tracker.update` (`apply.rs:932-936`) | yes |
| `sync` create of unsynced item | `run.rs:1766` → `apply.rs:1207`/`:1221` → `create_marking_progress` | `tracker.create` (`apply.rs:1519-1520`) | yes |
| `work update --push` | `work-cli/src/update.rs:216-245` | `tracker.update` | yes |
| Adoption (sync and promotion) | `apply.rs:1372-1440`; `promotion.rs:614-645` | `locate` only | no |
| `accelerator linear create` | `linear-cli/src/main.rs:359-395` | `client.create_op` (`:368`) | yes, verbatim |
| `accelerator linear update` | `linear-cli/src/main.rs:306-334` | `client.update_op` (`:320-327`) | yes, verbatim |

Promotion is reached from `work create --push` with tracker-owned IDs
(`work-cli/src/create.rs:624-690`), `accelerator work promote`
(`work-cli/src/promote.rs:273`) and sync's draft promotion
(`work-adapters/src/sync/identity_settlement.rs:462-478`).

The wrappers in `client.rs:892-918` are thin: `create` forwards to
`create_op` (`:677-715`, builds `{"input": {"teamId", "title",
"description": body}}`), `update` builds `UpdateFields { description:
Some(body), .. }` for `update_op` (`linear-client/src/update.rs:57-73`).
`retitle` (`:920-934`) sends no description. Escaping in `create_op` or
`update_op` would be wrong: the raw CLI shares them and `update_op` also
serves `transition`. Escaping at the engine's five call sites would need a
per-tracker branch at each.

### `BodyDialect` and why `as_pushed` stays identity

`BodyDialect` (`cli/tracker/src/lib.rs:616-645`) is a supertrait of
`RemoteTracker` (`:669`). Its `as_pushed` means "the body as the tracker
would hold it once pushed and read back"; it is never on a send path.
Production callers are all in `cli/work-adapters/src/sync/body_comparison.rs`:
`local_body_hash` (`:36-38`), `equivalence_to_rendered` (`:83`) and
`differing_sections` (`:105`).

Implementors: `LinearClient` (`client.rs:885-889`) and `ShortcutClient`
(`shortcut-client/src/client.rs:679-683`) override only `is_wire_document`;
`JiraBodyDialect` (`jira-client/src/body_dialect.rs:31-61`) round-trips
through ADF. Jira's send-side conversion lives in the client
(`jira-client/src/mutation.rs:89`, `:208-209`), not in the dialect. That is
the precedent for putting Linear's escape in the client too.

Escaping on send while `as_pushed` stays identity keeps the baseline
consistent:

- Local hashes are taken over the unescaped local body and never see the
  sent string (`baseline_digester.rs:78-97`, `apply.rs:939-943`).
- Remote hashes are taken over Linear's read-back.
- The next sync compares `parse(local)` with `parse(read-back)`. If Linear
  stores `crates\.io` and returns it, both parse to the text `crates.io`.

This rests on Linear storing the escape as sent or as something that parses
to the same text. The escaped capture PP-906 requires is what proves it. If
it does not hold, Linear needs an `as_pushed` that models the round trip,
and `migrate-cli/src/sync_baselines.rs:24-30`, which maps `"linear"` to
`PlainDialect`, would need the same change.

The pending-push and promotion request digests
(`pending_push::request_digest`, used at `apply.rs:721-723`,
`creation.rs:533`, `promotion.rs:531-535`) are over the unescaped body and
are unaffected.

### Parser and the `markdown` crate

`pulldown-cmark` is exact-pinned at `=0.13.6` (`cli/Cargo.toml:128`; the
PP-880 plan's `=0.13.4` is stale). `cli/deny.toml:178` allows only
`document` and `markdown` to depend on it, so neither `linear-client` nor
`work-adapters` can parse Markdown directly.

Two wrappers exist, with different extension sets:

| Wrapper | Purpose | Options | Exposes |
|---|---|---|---|
| `document::markdown` (`cli/document/src/markdown.rs:57-71`) | PP-880 equivalence | `TABLES`, `TASKLISTS`, `STRIKETHROUGH` | `events(source) -> Vec<(MarkdownEvent, Range<usize>)>` |
| `markdown::github` (`cli/markdown/src/github.rs:19-62`) | PP-885 PR descriptions | adds `FOOTNOTES`, `MATH`, `GFM` | `unwrap_soft_line_breaks(&str) -> String` |

The digit rule is defined against "PP-880's pinned CommonMark parser", so
`markdown::linear` should use `document::markdown`'s option set, either by
duplicating it with a test that the two agree or by having `markdown`
depend on `document`. ⚠️ Using `markdown::github`'s set would let footnote
or math syntax change what counts as a paragraph continuation.

The PP-885 plan and PR #164 (`meta/prs/164-description.md:34-41`, `:98-100`)
anticipated a `markdown::linear` module beside `markdown::github`, with the
span helpers at the crate root for sharing (`cli/markdown/src/lib.rs:9-34`):

- `continuation_start` skips a continuation line's `>`/space/tab container
  prefix and clamps to the next event's start. It is the detector the digit
  rule needs.
- `trailing_whitespace_start` is specific to unwrapping and is not needed.

That hand-off note predates dropping Defect 3; PP-906 now escapes rather
than unwraps, so the crate, the root helpers and the test pattern carry
over, not `unwrap_soft_line_breaks`.

### Event stream properties the escaper relies on

From `cli/document/tests/markdown.rs`, `cli/markdown/src/tests.rs:85-116` and
the PP-885 plan (l.97-121, 380-387):

- **Soft breaks.** A paragraph continuation is a `SoftBreak` whose range
  covers only the line ending; the next event's range starts after the
  container prefix. `"one\ntwo\n"` gives Text `0..3`, SoftBreak `3..4`, Text
  `4..7`.
- **`End` events.** The event after a `SoftBreak` can be an `End(Link)`
  whose range starts at the element's opening, so "next event start" needs
  the `>= line_start` guard `continuation_start` already has.
- **Opaque blocks.** Code and HTML blocks arrive as one event spanning the
  block (`document/src/markdown.rs:132-152`), so skipping them is free.
- **Autolinks.** `pulldown-cmark` has no bare-URL autolinking; `<u>` arrives
  as a `Link`. The escaper must detect bare scheme URLs itself, using GFM's
  extended-autolink boundary rules as the work item states.
- **Interrupting lists.** `1. text` after a paragraph line is a new list, not
  a continuation, so the digit rule leaves it alone with no special case.

⚠️ No test pins how `Text` ranges fall around a backslash escape.
`pulldown-cmark` is expected to split `crates\.io` into Text `crates` and
Text `.io` with the backslash between their ranges. The dotted-token rule
works on merged text runs and must map each `.` back to a source offset, and
"a token with some dots already escaped has its remaining dots escaped"
depends on seeing that gap. A characterisation test should pin it first, as
PP-885 did for soft breaks.

### Equivalence and digests

`cli/corpus/src/body/canonical.rs` never touches the parser. `CommonMarkBody`
(`cli/corpus-adapters/src/body.rs`) folds events into `corpus::body::Body`
and discards inline spans; `canonical.rs` reduces that to `CanonicalBody`.

- **Text merging.** Soft breaks become spaces (`canonical.rs:215`), and
  `Segments::push_text` (`:270-284`) merges adjacent text with equal marks.
  An escape therefore vanishes from the canonical form, which is why
  `0160\.` versus `0160.` (`R11`) is noise.
- **Links.** `Segments::push_link` (`:286-300`) dissolves a title-less link
  whose text equals its destination, so `[u](<u>)` is noise and
  `[crates.io](<http://crates.io>)` is not (`R12`).
- **Recipe.** `DigestRecipe::CURRENT = 1` (`work-adapters/src/sync/baseline.rs:17-30`).
  `work-adapters/tests/golden_digests.rs:243-349` fails if `Cargo.lock`'s
  parser version moves without a recipe bump. PP-876 and PP-919 will each
  bump it; neither has landed.

Equivalence is consumed by `work/src/sync/classify.rs` (`recover` `:370-378`,
`classify_split` `:462-465`, `classify_legacy` `:513-525`), by adoption
(`apply.rs:1468-1477`) and by promotion (`promotion.rs:670-673`).

### The capture

`cli/work-adapters/tests/fixtures/linear-rewrites/` holds `README.md`,
`pushed.md` and `read-back.json`; no `escaped/` directory exists. It was taken
by hand on 2026-10-13 (`README.md:3-6`) as issue `TES-17` and deleted.

| Fragment | Container | Observed rewrite | Meaning changes |
|---|---|---|---|
| `R11` | top level | `0160.` → `0160\.` | no |
| `R12` | top level | `crates.io` → `[crates.io](<http://crates.io>)` | yes |
| `X04`–`X06` | top level, list item | `0160.` → `0160\.` | no |
| `X09` | list item, then top level | in item `0160.` → `160. `; top level unchanged | yes, in item |

Consumers:

- **`work-adapters/tests/linear_rewrites.rs:22-103`.** Splits fragments on
  `## ` by code, folds `tracker_test_support::Rewrite`s over each, and
  asserts the read-back. `recorded_rewrites()` panics on a code missing from
  either side.
- **`corpus-adapters/tests/body_equivalence.rs`.** `captured_row_is_noise`
  and `captured_row_differs` (`:99-116`). The whole-capture test (`:270-281`)
  excludes `R12` and `X09`; every new meaning-changing fragment must join
  that list.
- **`work-adapters/tests/sync_equivalence.rs:489-573`.** Runs 11
  `COSMETIC_REWRITES` on both sides.

`tracker-test-support/src/rewrite.rs` simulates Linear in tests.
`autolink_dotted_tokens` (`:181-192`) links any word whose last label is two
or more lowercase ASCII letters; that is wider than a TLD list (`foo.toml`)
and narrower than case-insensitive matching (`Crates.IO`). Once the new
fragments are captured it should follow the TLD rule.
`escape_wrapped_numbers` (`:216-235`) does not reproduce `X09`.

The README tables cite absolute line ranges in `pushed.md`, so appending
fragments at the end avoids renumbering. The allowlist
(`work-item-sync-recovery/README.md:15-31`) is a `jq` projection applied by
hand. It keeps `id`, `identifier`, `title`, `description`, `updatedAt` and
`state.name`, and redacts nothing inside the description. The
`test:integration:fixture-size` guard checks a binary, not fixtures
(`tasks/build.py:198-229`), so it does not limit the capture.

### Test doubles for the push criteria

There are two kinds of recording double, and only one sees what Linear is
sent:

- **Port level.** `tracker_test_support::RecordingTracker`
  (`tracker-test-support/src/lib.rs:47-87`, `:139-181`, `calls()` at `:527`)
  records `Call::Create { body }` and `Call::Update { body }`. It replaces the
  whole `RemoteTracker`, so it never runs `LinearClient`'s wrapper, and with
  the escaper in the wrapper it cannot show an escaped body.
- **Wire level.** `graphql_test_support::MockGraphQLServer` routes by
  operation name and keeps every body (`bodies(op)` `:108`, `last_body(op)`
  `:120`). `http_test_support::MockHTTPServer` does the same for Jira and
  Shortcut (`bodies` `:265`). Real clients are built against them in
  `work-adapters/tests/sync_run_real_client.rs` (`jira_client` `:184`,
  `linear_client` `:207`, `shortcut_client` `:1758`).

⚠️ Subprocess CLI tests cannot reach a mock: `from_config` refuses a loopback
base (`work-cli/tests/cli_create_push.rs:1-8`,
`work-cli/tests/sync_resolves_real_client.rs:1-12`). The push-path matrix
therefore belongs in `work-adapters/tests/`, or in `work-cli` unit tests
using `FixedRegistry::holding(real client)`
(`work-cli/src/test_support.rs:101-116`).

The closest template is
`every_pull_entry_point_writes_the_description_without_a_title_line`
(`sync_run_real_client.rs:1446-1536`). It loops over `enum Tracker { Jira,
Linear, Shortcut }` (`:1191-1314`) and `enum PullEntryPoint`
(`:1328-1350`) and asserts byte-exact file content. A push twin with `enum
PushEntryPoint` covers the "Linear escapes, Jira and Shortcut do not, local
file byte-identical" criteria in one test. Per-path harnesses for richer
cases are:

- **Promotion.** `work-adapters/tests/promotion.rs:268-396`.
- **Pending push.** `work-adapters/tests/sync_create.rs`: `creating`
  `:4187`, `seed_attempted_marker` `:4131`.
- **`create --push`.** `PushingRepo` in `work-cli/src/test_support.rs:121-345`.

### Pinned TLD snapshot

No TLD, public-suffix or IANA data exists in `cli/`, and no `phf` is used.
Patterns to copy:

- **Data file with a drift test.** `include_str!` with `#` comments skipped
  (`design/tests/cue_phrase_drift.rs:10-32`;
  `work/src/corpus_trust.rs:102-104`).
- **Fixture with a case-count floor.** `linear-client/src/upload.rs:286-314`
  reads `accept|reject` lines and asserts a minimum count.
- **Generated constant.** `launcher/build.rs:57-101` turns a reviewed
  `pins.toml` into a `const` slice with `cargo:rerun-if-changed`.

IANA's `tlds-alpha-by-domain.txt` already starts with a `# Version …` comment
line and lists one upper-case label per line, so `include_str!` plus a
`HashSet` built once, compared upper-cased, fits the first pattern. The
snapshot belongs in `cli/markdown/` beside the rule that reads it.

## Code References

- `cli/linear-client/src/client.rs:885-918` — Linear `BodyDialect` and `RemoteTracker` wrappers; the escape site
- `cli/linear-client/src/client.rs:677-715` — `create_op`, shared with the raw CLI
- `cli/linear-client/src/update.rs:32-73` — `UpdateFields` and `update_op`
- `cli/linear-cli/src/cli.rs:58-73` — `UpdateArgs`: `--description`, no `--body-file`
- `cli/linear-cli/src/main.rs:306-427` — raw `linear update` and `linear create`
- `cli/tracker/src/lib.rs:616-669` — `BodyDialect`, `PlainDialect`, `RemoteTracker`
- `cli/jira-client/src/mutation.rs:85-209` — Jira's send-side conversion precedent
- `cli/work-adapters/src/sync/body_comparison.rs:36-140` — hashing and equivalence over `as_pushed`
- `cli/work-adapters/src/sync/apply.rs:917-944,1207-1537` — sync update and create sends
- `cli/work-adapters/src/promotion.rs:398-762` — promotion create and retitle update
- `cli/work-adapters/src/creation.rs:558-823` — legacy `create --push`
- `cli/work-cli/src/update.rs:216-255` — `work update --push`
- `cli/work-cli/src/tracker_registry.rs:301-337` — tracker resolution
- `cli/markdown/src/lib.rs:9-34` — shared span helpers (`continuation_start`)
- `cli/markdown/src/github.rs:19-62` — offset-splicing rewriter to model on
- `cli/markdown/src/tests.rs:50-116` — characterisation and LF/CRLF/idempotence test pattern
- `cli/document/src/markdown.rs:57-152` — equivalence parser options and event spans
- `cli/corpus/src/body/canonical.rs:134-300` — canonical form, text merging, link dissolving, digest input
- `cli/Cargo.toml:119-128` — parser pin and its rationale
- `cli/deny.toml:178` — parser wrappers allowlist
- `cli/work-adapters/src/sync/baseline.rs:17-30` — `DigestRecipe::CURRENT`
- `cli/work-adapters/tests/golden_digests.rs:133-349` — digest pins and parser-version guard
- `cli/work-adapters/tests/fixtures/linear-rewrites/` — capture
- `cli/work-adapters/tests/linear_rewrites.rs:22-103` — capture versus simulated rewrites
- `cli/corpus-adapters/tests/body_equivalence.rs:99-281` — capture equivalence tests
- `cli/tracker-test-support/src/rewrite.rs:181-235` — simulated autolinking and digit escaping
- `cli/tracker-test-support/src/lib.rs:47-657` — `RecordingTracker`
- `cli/graphql-test-support/src/lib.rs:92-158` — `MockGraphQLServer`
- `cli/work-adapters/tests/sync_run_real_client.rs:184-329,1191-1536` — real-client harness and entry-point matrix
- `cli/migrate-cli/src/sync_baselines.rs:24-30` — Linear mapped to `PlainDialect`

## Architecture Insights

- **Send-side transforms live in the client, comparison rules in the
  dialect.** Jira converts in `mutation.rs` and predicts the read-back in
  `JiraBodyDialect::as_pushed`. Linear should follow suit: escape in the
  `RemoteTracker` wrapper, and leave `as_pushed` identity because the escape
  is invisible to the canonical form.
- **The inherent `*_op` methods are the raw wire API; the trait methods are
  the work-item API.** That split gives PP-906's verbatim raw commands for
  free.
- **Parsing is fenced into two technical-library crates.** `markdown` is the
  home for push-side transforms, one module per dialect. Its pup rule
  forbids I/O, and its public API must expose no `pulldown_cmark` type
  (`tasks/public_api.py` pins it). A new `pub fn` changes that snapshot, so
  run `mise run public-api:update` and read the diff.
- **Byte preservation by offset splicing.** The escaper copies source
  between insertion points and inserts only `\`, so the output differs from
  the input by backslashes alone. That keeps "byte-identical to
  `escaped/pushed.md`" a sharp test.
- **Characterise the parser before relying on it.** PP-885 pinned soft-break
  ranges in a test so a parser bump fails with a precise message; the
  backslash-escape split needs the same.

## Historical Context

- `meta/plans/2026-10-10-PP-880-sync-round-trip-defects.md` — `BodyDialect` design (l.137-151, 842-924), canonical body (l.1360-1391), recipe guard (l.1482-1501), and the note that 0161, 0203, 0276 and 0293 report `synced` (l.2742-2771)
- `meta/prs/159-description.md` — PP-880 delivery; parser pin rationale (l.130-131); the 32-item conflict table (l.288-313)
- `meta/plans/2026-10-13-PP-885-push-pr-descriptions-to-github-unwrapped.md` — `markdown` crate, offset-splicing gotchas (l.97-121, 290-387), hand-off to a `markdown::linear` module (l.148-151)
- `meta/prs/164-description.md` — span helpers placed at the crate root for PP-906 (l.34-41, 98-100)
- `meta/research/codebase/2026-10-13-PP-885-push-pr-descriptions-to-github-unwrapped.md` — choice of `pulldown-cmark` over `comrak` (l.384-389)
- `meta/plans/2026-11-13-0391-shortcut-integration.md` — Shortcut stores bodies byte for byte (l.920-925)
- `meta/reviews/work/PP-906-linear-push-corrupts-markdown-content-review-1.md` — final verdict APPROVE (l.569-576); the body still says REVISE at l.24. Open from pass 4: the post-merge `synced` check depends on state outside this item; "pinned parser" names no crate; the read-back equivalence requirement (l.141-142) is broader than what is verified.
- `meta/decisions/` — no ADR covers `BodyDialect`, parser choice or body normalisation.

## Related Research

- `meta/research/codebase/2026-10-10-PP-880-sync-round-trip-defects.md`
- `meta/research/codebase/2026-10-13-PP-885-push-pr-descriptions-to-github-unwrapped.md`
- `meta/research/codebase/2026-09-26-PP-760-tracker-owned-work-item-id-generation.md`
- `meta/research/codebase/2026-08-17-PP-740-provider-client-crates-over-the-tracker-port.md`

## Open Questions

- ❓ **Raw update criterion.** `accelerator linear update` has no
  `--body-file`. Change the criterion to `--description`, or add
  `--body-file` (scope growth)?
- ❓ **Adoption criterion.** Adoption sends no body. Replace "pending-push
  adoption" with the pending-push create (`create_from_local`), and add
  `work update --push`?
- ❓ **Post-merge repair route.** Are PP-180, PP-733, PP-860 and PP-870 still
  conflicted? PP-880 recorded them `synced` from 2026-10-14. If they are,
  `--resolve <id>=local` will refuse, and repair needs a local edit followed
  by a push.
- ❓ **0184 (PP-714) damage.** PP-880 attributed its code-span table-cell
  truncation to "a rewrite 0320 owns". PP-906 dropped code-span handling as
  cosmetic. Does that damage fall under `X07`/`X08`, or is it now unowned?
- **Escape spans.** How `pulldown-cmark` 0.13.6 ranges `Text` across a
  backslash escape is not pinned by any test.
- **Option set.** Should `markdown::linear` reuse `document::markdown`'s
  options through a dependency, or duplicate them with an agreement test?
- **TLD claims unchecked.** That `.pid` is in the IANA root zone, as the work
  item asserts, has not been checked here; the pinned snapshot settles it.
- **Linear's stored form unknown.** What Linear stores for `\.` and `\)` is
  unknown until the escaped capture. If it is not text-equivalent, Linear
  needs a non-identity `as_pushed`.

## Follow-up Research 2026-11-23T18:05:00+00:00

### Decisions taken on the open questions

- **Raw update.** Add `--body-file` to `accelerator linear update`. It
  joins `--description` in `UpdateArgs` (`cli/linear-cli/src/cli.rs:58-73`),
  mirroring `CreateArgs.body_file` (`:51-53`), and is read in `run_update`
  (`cli/linear-cli/src/main.rs:306-334`) before `update_op`. It must stay
  verbatim, so it goes through `update_op` and never through the
  `RemoteTracker` wrapper. The two flags should conflict.
- **Push paths.** Replace "pending-push adoption" with the pending-push create
  (`create_from_local`, `cli/work-adapters/src/sync/apply.rs:1519`), and add
  `work update --push` (`cli/work-cli/src/update.rs:175-281`) to the paths
  that escape and to the recording-tracker criterion.

### Repair route for the affected items

A read-only check on 2026-11-23 (`accelerator work sync --preview` targeted at
the five items, then `accelerator linear show` on each) found the following:

| Item | Sync reports | Remote still corrupted | Corruption |
|---|---|---|---|
| PP-180 (0161) | `synced` | yes | line 39 `  160. ` (`X09`) |
| PP-733 (0203) | `synced` | yes | `[crates.io](<http://crates.io>)` twice; `[crates.io/repository](<http://crates.io/repository>)` |
| PP-860 (0276) | `synced` | yes | `[server.pid](<http://server.pid>)` |
| PP-870 (0293) | `synced` | yes | `[jql.rs:167](<http://jql.rs:167>)` |
| PP-714 (0184) | `synced` | no | none |

The four items report `synced` because an earlier sync recorded a baseline
whose remote hash is the corrupted read-back. Neither side has changed since,
so sync sees nothing to do. It will never push them on its own, and
`--resolve <id>=local` refuses an item that is not in conflict.

**Repair with `accelerator work update <path> --push` and no other flags.**
`push_update` (`cli/work-cli/src/update.rs:175-281`) sends the whole rendered
body through `tracker.update` whatever the baseline holds. It then re-reads the
remote and records a fresh baseline. With the escaper in
`LinearClient::update`, that sends the escaped body and the new baseline
records the repaired read-back. One thing is unverified: that rendering
unchanged frontmatter with `document::render(Some(&content), &yaml)` (`:391`)
writes the local file back byte-identical. The post-merge check should diff
it.

The post-merge verification should change accordingly:

- Each item is repaired with `accelerator work update <path> --push`, not
  `--resolve <id>=local`.
- "Next sync reports `synced`" is not evidence on its own, because the items
  already report it. The check is the remote description: no `](<http://`
  around a dotted token, no lone-number list item, and no visible backslash
  in the web UI.
- Finding further affected items by "conflicted" misses absorbed corruption
  like these four. Search every Linear-synced item's remote description for
  the two patterns instead.

### The 0184 (PP-714) damage

The damage is no longer present and does not belong to PP-906.

- **Origin.** On 2026-10-10 Linear's copy of PP-714 had lost the first
  characters of every code-span cell in its table (`meta/prs/159-description.md:265-269`).
  The table sits inside a list item, indented two spaces
  (`meta/work/PP-714-template-enumeration-swallows-a-wrong-plugin-root.md:278-285`).
- **Repair.** It was repaired on 2026-10-15 by pushing the local body with
  `accelerator linear update PP-714 --description …`; that push is what found
  PP-921 (`meta/work/PP-921-linear-update-overwrites-fields-it-was-not-given.md:38-41`).
  PR #159 records that nobody re-checked whether Linear stored it intact.
- **Current state.** On 2026-11-23 the remote table holds every code span
  intact. The only difference is the cosmetic `| -- | -- | -- |` delimiter
  row. The item reports `synced` and was last updated 2026-11-19.

A verbatim push of the same indented table read back intact, so Linear's
parser did not cause the truncation. The likeliest cause is an earlier write
path that predates PP-880, but the corpus does not say which, and I did not
trace it. Nothing is owed by PP-906; the PP-880 plan (l.2767-2771) and
validation (l.96-97) notes that assign it to 0320 are stale.
