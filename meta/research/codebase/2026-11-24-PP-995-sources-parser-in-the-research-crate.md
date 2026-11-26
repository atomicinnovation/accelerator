---
type: "codebase-research"
id: "2026-11-24-PP-995-sources-parser-in-the-research-crate"
title: "What PP-995's Sources and tier-text parsers build on or change"
date: "2026-11-24T00:30:41+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "PP-995"
parent: "work-item:PP-995"
topic: "What PP-995's Sources and tier-text parsers build on or change"
tags: ["research", "codebase", "research-crate", "sources", "tier-text", "citing-once", "finding-outputter", "eval"]
revision: "c7d80a5677b676f2ba20ef1a9f6f50b83186dd64"
repository: "accelerator"
last_updated: "2026-11-24T00:30:41+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# What PP-995's Sources and tier-text parsers build on or change

**Date**: 2026-11-24T00:30:41+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: c7d80a5677b676f2ba20ef1a9f6f50b83186dd64
**Branch**: detached HEAD (jj working copy)
**Repository**: accelerator

## Research Question

What does PP-995 (`meta/work/PP-995-sources-parser-in-the-research-crate.md`)
build on or change in the codebase today? This covers four things:

- where the Sources grammar and tier text are defined, in the citing-once and
  finding-outputter skills;
- the Python eval tooling that parses them today;
- the current shape of the `research` crate under `cli/`, and how its parsers
  are laid out and tested;
- how a Sources parser and a tier-text parser would fit there.

The work item's acceptance criteria and line rules are taken as the
specification. The two work-item reviews (`meta/reviews/work/PP-995-*`, both
APPROVE) are extra context.

Decisions made without a user present:

- I took the work item as authoritative and did not re-litigate its rules.
- Where the codebase offers more than one convention, I name the one that best
  fits and say why.
- I did not add GitHub permalinks. The working copy is a jj change on a
  detached HEAD and is not known to be pushed, so the references are local
  paths.

## Summary

- **No Rust code parses Sources or tier text.** The `research` crate only
  produces tier vocabulary:
  - `Tier` with `Display` (`cli/research/src/sources/record.rs:9-24`);
  - the classifiers `openalex_tier` and `arxiv_tier`
    (`cli/research/src/sources/tier.rs:8-28`);
  - `RECOMMENDED_KEYS` (`cli/research/src/sources/metadata.rs:13-33`).

  The visualiser has no handling of `## Sources`, `None found.` or tier text.
  PP-995 adds the first reader.
- **The grammar's definition lives in two skills.**
  - Tier text: `skills/research/outputters/citing-once/SKILL.md:25-30`
    (base and suffix), `:64-70` (the `also` merge) and `:74-82` (the
    retraction order).
  - The entry, `None found.` and the metadata block:
    `skills/research/outputters/finding-outputter/SKILL.md:80-154`.
  - Neither skill defines where the section opens or closes, line endings,
    blank lines, or whether `None found.` may sit beside entries. Those rules
    exist only in PP-995.
- **The Python reference is regex-based and looser.** It lives in
  `tasks/eval/documents/sources.py` (100 lines). It also:
  - raises on the first unread line;
  - fails the whole document on a bad tier;
  - silently finds no section in a CRLF document;
  - attaches block lines to the last entry with no adjacency check.

  Every difference the work item lists is confirmed in the code.
- **The `research` crate cannot use `regex`.** cargo-pup limits it to std,
  `kernel::Error`, `network` and itself (`cli/pup.ron:99-114`). Its Cargo
  manifest has only `network` (`cli/research/Cargo.toml:17-18`), and it has no
  dev-dependencies. Both parsers must be hand-written with `str` methods,
  which is what every line parser in `cli/` already does.
- **The crate has strong local precedents for both parsers.**
  - `Outline::parse` (`cli/research/src/topic/outline.rs:28-41`) is an
    infallible, line-by-line whole-document parse with 1-based line numbers.
  - `Stem` (`cli/research/src/topic/layout/stem.rs:14-33,85-89,119-127`) keeps
    its text as written, so `parse(s)?.to_string() == s`.
  - Hand-rolled error enums carry the raw input
    (`cli/research/src/sources/request.rs:17-96`).
- **The public-API snapshot gates the export.** The check is
  `tasks/public_api.py` (`check` at `:132-162`, `update` at `:165-177`), over
  `cli/research/tests/fixtures/public-api.txt`. The `Tier` lines are
  `:1216-1230` and the `Record` lines `:1250-1269`; they must stay unchanged.

## Detailed Findings

### 1. Where the grammar is defined (the skills)

#### citing-once (`skills/research/outputters/citing-once/SKILL.md`)

- **Citation shape** (`:18-23`): `- [Title](url) — tier text — venue`, with
  `  - key: value` lines under it. The separator is a spaced em dash (U+2014).
- **Tier text** (`:25-30`):
  - A base `tier-1`, `tier-2` or `tier-3`, "optionally followed by a
    parenthesised suffix".
  - The single space before `(` is never stated; every example has it.
- **Merge rule** (`:64-70`):
  - Other distinct tiers go into an `also` suffix, next-lowest first.
  - The `, ` joiner inside the `also` list is shown only by example.
- **Retraction suffixes** (`:74-80`):
  - Parts appear in the order `retracted`, then `withdrawn`, then the `also`
    list, each present only when it applies, joined by `; `.
  - `:82` says these are "the only changes to tier text". This is the closed
    vocabulary PP-995's tier-text parser enforces on shape.
- **Every tier-text example in the file**, which the tier-text tests must
  include under PP-995's acceptance criteria:
  - `:27` `tier-3 (also tier-1)`
  - `:68` `tier-3 (also tier-2)`, `tier-3 (also tier-2, tier-1)`
  - `:70` `tier-2`
  - `:77` `tier-3 (retracted)`
  - `:78` `tier-3 (retracted; also tier-1)`
  - `:79` `tier-3 (retracted; withdrawn; also tier-1)`
  - worked-example inputs: `:163` `tier-2`, `:165` `tier-3 (retracted)`,
    `:167` `tier-1`
  - worked-example output: `:176` `tier-3 (retracted; also tier-2, tier-1)`
- **Worked example** (`:158-181`):
  - Three input entries with blocks, one deliberately out of canonical key
    order (`:167-169`).
  - One merged entry with a four-line block (`:176-180`).
  - This is a ready-made Sources fixture for an integration test.
- **Venue may be absent**: `:92` reads "A citation with no venue sorts after
  every venue". This supports PP-995's optional venue.
- **Duplicates**: `:14` expects input documents to hold two citations of one
  source. This supports "a source listed twice is two entries".
- **Block order** (`:130-156`): table order, then other keys byte-wise. Values
  are kept "exactly as recorded". The key table is at `:138-156`.

#### finding-outputter (`skills/research/outputters/finding-outputter/SKILL.md`)

- **Sources bullet** (`:80-90`): one bullet per source, carrying its tier,
  venue and optional block. In the skill the example is shown indented inside
  a list item; the real shape starts at column 0.
- **`None found.`** (`:92-93`): this text replaces the entries when nothing
  was relevant.
- **Tiering and suffixes** (`:95-101`):
  - `:96-97` allows "any suffix the profile prescribes", which is looser
    wording than citing-once's closed set.
  - The profiles in fact prescribe only `(retracted)` and `(withdrawn)`.
- **Metadata block** (`:105-154`):
  - The block sits "directly under its line" and is optional (`:105-108`).
  - "Keys are lowercase `snake_case`, values are single-line, and a key with
    no value is omitted" (`:113-114`).
  - `url`, title and venue never appear in the block (`:115-116`).
  - The key table is at `:130-150`, and the same 19 keys are in
    `RECOMMENDED_KEYS`.
  - All 19 keys match PP-995's `[a-z][a-z0-9_]*`. The skill itself does not
    state that pattern.

#### Other places that restate the shape

None of these need to change for PP-995, but they produce the input:

- **Templates** with `## Sources` as their final section, holding
  `- [Title](url) — tier-1 — {source domain/venue}` and
  `  - {source-metadata}`:
  - `templates/topic-research-finding.md:31-33`
  - `templates/topic-research-level-note.md:34-36`
  - `templates/topic-research-report.md:35-37`
  - `templates/topic-research-synthesis.md:32-34`

  Because Sources is last in every template, the section usually runs to the
  end of the document. That makes the span-end-at-EOF case (line count + 1)
  the common case, not an edge case.
- **Outputters and agents**:
  - `skills/research/outputters/level-note-outputter/SKILL.md:17-20` defers to
    the finding outputter's rules.
  - `agents/composer.md:29-35` composes by citing-once.
- **Profiles**:
  - `skills/research/profiles/openalex-profile/SKILL.md:63` writes
    `tier-3 (retracted)`.
  - `skills/research/profiles/arxiv-profile/SKILL.md:71-72` writes
    `tier-3 (withdrawn)`.
  - `skills/research/profiles/web-profile/SKILL.md:28-40,100-101` covers the
    block and `None found.`.
- **research-topic**: `skills/research/research-topic/SKILL.md:978-980`
  renders `{source} ({tier text})` in ask output. That text is not Sources
  grammar, but it is a place that displays tier text.

### 2. The Python eval tooling that parses it today

All of this is in `tasks/eval/documents/sources.py`.

- **Patterns**:
  - `ENTRY` (`:7-10`):
    `^- \[(?P<title>.*)\]\((?P<url>[^)\s]*)\) — (?P<tier>tier-[123](?: \([^)]*\))?)(?: — (?P<venue>.+))?$`
  - `BLOCK_LINE` (`:11`): `^  - (?P<key>[a-z0-9_]+): (?P<value>.*)$`
  - Constants: `ALSO = "also "` (`:12`) and `NONE_FOUND = "None found."`
    (`:13`).
- **`TierText`** (`:16-49`):
  - A frozen dataclass with `base: int`, `marks: tuple[str, ...]` and
    `also: tuple[int, ...]`.
  - `parse` (`:24-37`) uses `fullmatch` on `tier-([123])(?: \((.*)\))?` and
    splits the suffix on `"; "`.
  - Any part not starting with `also ` becomes a mark. Mark names, order and
    `also` tier range are never checked.
  - `__str__` (`:43-49`) writes the marks, then one `also` part.
  - `tier-1 ()` parses as plain `tier-1`, so it does not round-trip.
  - `also x` raises a bare `int()` `ValueError`.
- **`Citation`** (`:52-66`) holds `title`, `url`, `tier`, `venue` and `block`.
  `render()` writes the entry back as text.
- **`sources_section`** (`:69-73`):
  - Uses `^## Sources\n(.*?)(?=^## |\Z)` with MULTILINE and DOTALL.
  - Only the first heading counts, and the section closes at the next `## `.
  - A CRLF heading, a trailing space, or a heading at EOF with no newline
    yields `""`.
- **`parse_sources`** (`:76-100`):
  - An entry line becomes a `Citation`, with the tier parsed inline.
  - A block line attaches to `citations[-1]` with no adjacency check.
  - Any other line that does not strip to `""` or `None found.` raises
    `ValueError` (`:98-99`).
  - Because the tier is built into `ENTRY`, a bad tier means the whole line is
    a non-entry, so it raises.
  - It returns no section state, no unread lines and no span.

Tests:

- `tests/unit/tasks/test_citing_once_rule.py`:
  - parse: `:64-78`
  - render round trip: `:81-87`
  - no venue: `:90-93`
  - `None found.`: `:96-97`
  - section bounded by `## `: `:100-106`
  - the skill's worked example: `:250-264`
- Nothing tests CRLF, unread lines, a trailing ` — `, bracketed titles or
  malformed tiers.
- The tests run in the `test:unit:tasks` pytest lane
  (`tasks/test/unit.py:65-75`).

Callers. Each one shows the Python parser has to stay as it is, because evals
do not call Rust:

- `tasks/eval/documents/rule.py:11,63-74,133-167`:
  - `merged_tier` and `cite_once`.
  - `merged_tier` is the only place unknown marks are rejected, and it does
    that at merge time.
- `tasks/eval/citing_once/fixtures.py:23-34,92-94`
- `tasks/eval/citing_once/runs.py:202`
- `tasks/eval/citing_once/projects.py:39`
- `tasks/eval/ask_and_report/verdicts.py:465-481,510-534`: catches
  `ValueError` and reports it as "expected readable Sources".
- `tasks/eval/ask_and_report/preflight.py:163-171`

Confirmed differences from PP-995. The work item already lists all of these
(see its Dependencies section), and I found no extra ones that matter:

1. **Suffix parts.** Python accepts any `; `-separated parts in any order.
   That covers `withdrawn; retracted`, `unknown`, `also tier-9` and an empty
   `()`.
2. **Rejected tier.** In Python the entry is lost and the document fails.
3. **Unread lines.** Python raises on the first one instead of reporting it.
4. **Block adjacency.** Python attaches across blank lines and across
   `None found.`.
5. **Trailing ` — `.** Python treats the line as a non-entry, because the
   venue pattern needs `.+`.
6. **CRLF.** Python finds no section at all.
7. **Empty title or URL.** Python accepts these, because its patterns use
   `.*` and `[^)\s]*`.
8. **Block keys.** Python allows a leading digit or underscore
   (`[a-z0-9_]+`), and an empty value.

The title rule is the same in both: it ends at the last `](` that still makes
the line an entry. In Python that comes from greedy `.*` backtracking. A
hand-written Rust parser has to reproduce it explicitly.

### 3. The `research` crate today

- **Manifest**: `cli/research/Cargo.toml:1-18`.
  - Library only, with `role="domain"`, `context="research"` and
    `kind="product"` (`:9-12`).
  - Uses the workspace lints (`:14-15`): clippy pedantic and nursery, with
    `unwrap_used`, `expect_used` and `panic` set to warn
    (`cli/Cargo.toml:280-291`).
  - The only dependency is `network`, and there are no dev-dependencies.
  - There is no `kernel` dependency, so `kernel::Error` is allowed by pup but
    unused.
- **Module tree**:
  - `cli/research/src/lib.rs:9-12` has `pub mod conduct, confinement,
    sources, topic`, with no re-exports. Its doc comment at `:1-7` says "Pure
    logic only".
  - `cli/research/src/sources.rs:1-15` has `pub mod arxiv, classify, fetch,
    metadata, openalex, queue, record, request, schedule, tier, web`. Its
    module doc (`:1-3`) is about asking OpenAlex and arXiv for works and the
    tiered record.
  - `cli/research/src/topic.rs:7-16` holds the topic-set domain: claims,
    consultation, counts, evidence, layout, lifecycle, outline, plan,
    question and tree.
- **`Tier`** (`cli/research/src/sources/record.rs:9-24`):
  - `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`, with no `Ord` and no
    `Hash`.
  - `Display` writes `tier-1`, `tier-2` and `tier-3`.
  - There is no parse anywhere.
- **`Record`** (`:48-102`):
  - Fields are `pub(crate)`, with getters. `tier` is `:56`, `retracted` `:57`
    and `withdrawn` `:58`.
  - It is built only by normalisers. PP-995 must leave it unchanged.
- **`sources/tier.rs:1-28`**: tier classification only (`openalex_tier`,
  `arxiv_tier`). Its file name is taken, which bears on naming the new module
  (see section 5).
- **Parser conventions** in the crate:
  - **Inherent `parse`, not `FromStr`.**
    - `Option` parsers: `Stem::parse` (`topic/layout/stem.rs:23`),
      `Lineage::parse` (`topic/layout/lineage.rs:30`), `PartialDate::parse`
      (`sources/metadata/date.rs:41`), `RunId::parse` (`conduct/ledger.rs:35`)
      and `SpawnRef::parse` (`conduct/spawn.rs:20`).
    - `Result` parsers: `Family::parse` (`sources/request.rs:150`),
      `ArxivId::parse` (`:269`) and `PageUrl::parse`
      (`sources/web/page_url.rs:104`).
    - The only `impl FromStr` in the whole workspace is
      `cli/design/src/runtime/downgrade.rs:81-109`.
  - **Whole-document parse**: `Outline::parse(text) -> Self`
    (`topic/outline.rs:28-41`).
    - Uses `text.lines().enumerate()`, then `filter_map`, then a private
      per-line `OutlineItem::parse(index + 1, line)` (`:70-83`).
    - Line numbers are 1-based and relative to the whole file, frontmatter
      included (test `:224-228`).
    - The heading prefix is a `const` (`:10`).
    - Suspicious forms are kept as data: `ProfilesSuffix::Unseparated`
      (`:62-67,97-102`).
  - **Raw-text-carrying states**: `StoredCount { Count(u32), Missing,
    Unreadable(String) }` (`topic/counts.rs:81-86`). This is the nearest
    precedent for "parsed tier text, or the rejected text as written".
  - **Errors**:
    - Hand-rolled, with a manual `Display` and an empty
      `impl std::error::Error`.
    - `RequestError::MalformedArxivId(String)` and related variants carry the
      raw input (`sources/request.rs:17-96`).
    - `PageUrlError` deliberately does not carry it
      (`sources/web/page_url.rs:11-20`).
    - A tidy idiom for always returning the whole input is the `refused`
      closure in `cli/shortcut-client/src/shortcut_key.rs:71-105`:
      `let refused = || UnparseableKey(text.to_owned());`
  - **Identity round-trip**: `Stem` documents `parse(s)?.to_string() == s`
    (`stem.rs:21`) and tests it (`:119-127`).
  - **Imports**: one item per `use` line, never grouped. The pup
    `allowed_only` rule needs this (comment at `cli/pup.ron:598-602`).
- **Tests**:
  - `sources/` modules are mostly tested in `cli/research/tests/*.rs`, one
    file per module (`tier.rs`, `metadata_date.rs`, `page_url.rs`,
    `request.rs`, …).
  - `topic/` modules mostly use `#[cfg(test)] mod tests` in src.
  - Test names are behavioural sentences, e.g.
    `a_tier_renders_as_its_citation_label` (`tests/tier.rs:208-212`).
  - Table-driven cases are plain `for (raw, expected) in [...]` loops with
    the input as the assertion message (`tests/metadata_date.rs:13-36`).
  - There is no rstest. The `unwrap`/`expect` lints are handled either with
    `#![allow(clippy::expect_used)]` (`tests/metadata_date.rs:1`) or with
    `Result`-returning tests.
  - `tests/support/mod.rs` holds fetch doubles only.
- **Public-API snapshot**:
  - `tasks/public_api.py` pins `research` (`:15-33`, at `:27`).
  - `_render` (`:116-129`) runs `cargo +nightly public-api --omit
    blanket-impls,auto-trait-impls --include function-parameter-names -p
    research`.
  - `check` (`:132-162`, which is part of `mise run check`) fails on any
    byte difference. `update` (`:165-177`) regenerates the file.
  - The snapshot is one fully qualified item per line, and derived impls
    appear as `impl core::… for …` lines. So every derive on a new public
    type adds lines.
- **Dependency rules**:
  - `cli/pup.ron:95-114`, `research_domain_imports_only_permitted`, allows
    imports of `std|core|alloc`, `kernel::Error`, `network` and `crate`.
  - The crate-edge lint `tasks/lint/crate_dependencies.py` says a product
    context admits no other domain or adapter crate (`:92-93,384-389`), but a
    composition root may depend on it (`:392-397`).
  - Today's dependents are `research-adapters` and `research-cli` only.
  - The visualiser server (`cli/visualiser/server/Cargo.toml`, a composition
    root at `:10-11`) does not depend on `research`. PP-1002 adds that edge,
    so PP-995 adds no edge, as its acceptance criteria require.

### 4. Patterns elsewhere in `cli/` for the line parser

- **Section gating by a heading.** No existing scanner meets PP-995's exact
  rules:
  - `cli/corpus/src/frontmatter_validation/template_shape.rs:591-625` and
    `cli/corpus/src/linkage.rs:556-618` gate on `starts_with`/equality and
    close at the next `## `.
  - `cli/design/src/cue_phrase_audit.rs:54-71` splits a document into
    `(name, body)` H2 sections.

  The differences PP-995 needs are: an exact `## Sources` match, first section
  only, `### ` counted as unread, and close at the next `## `.
- **CRLF**:
  - `str::lines()` strips `\n` and `\r\n`. Several modules add a defensive
    `.strip_suffix('\r')`: `cli/adf/src/tokenise.rs:88-91` and
    `cli/migrate/src/decisions_file.rs:23`.
  - The test idiom for CRLF is re-running a case with
    `input.replace('\n', "\r\n")` (`cli/markdown/src/tests.rs:75-76`).
- **Spans**:
  - Byte spans use `Range<usize>` (`cli/corpus/src/body.rs:20-25`).
  - No line-span type exists. A `Range<usize>` of 1-based line numbers with an
    exclusive end would follow the house style.
- **Crate choice**:
  - `regex` is a workspace dependency (`cli/Cargo.toml:113`), used by
    `design-adapters`, `work`, `vcs-test-support` and the visualiser server.
  - `pulldown-cmark` is fenced to `document` and `markdown` (`cli/deny.toml:178`).
  - Neither is reachable from `research`.
- **Visualiser**:
  - No Sources handling in the server or the frontend.
  - Topic research is indexed only as a set (`DocTypeKey::TopicResearch`,
    `cli/visualiser/server/src/api/library.rs:90`).
  - The test fixture finding
    `cli/visualiser/server/tests/fixtures/meta/research/topics/prompt-caching-economics/findings/01-cost-model.md`
    has no Sources section.

### 5. How the two parsers would fit

This section is a recommendation drawn from the conventions above. The plan
decides.

- **Placement.** Put both under `cli/research/src/sources/`, beside `record`,
  which holds `Tier`. The work item asks for this.
  - The name `tier` is taken by the classifiers, so the tier-text parser needs
    a distinct module, e.g. `sources/tier_text.rs`.
  - The section parser fits a module named for the domain concept, e.g.
    `sources/cited.rs` or `sources/sources_section.rs`.
  - Either way, `sources.rs`'s module doc (`:1-3`) describes only fetching. It
    would need widening to cover reading a document's cited sources.
- **Shape.**
  - The section parser should be infallible: `parse(body: &str) -> Self`, like
    `Outline::parse`. It returns entries, unread lines, a three-state section
    state and an optional line span.
  - The tier-text parser should be fallible: `parse(text) -> Result<Self, E>`.
    The error carries the complete input, as `RequestError` variants and
    `UnparseableKey` do. Its `Display` must reproduce the input exactly, so it
    should either keep the text, like `Stem`, or render canonically from the
    parts. Strict shape checking makes canonical rendering identical to the
    input, which is the work item's rationale.
  - An entry's tier is "parsed or rejected-as-written". That is a two-variant
    enum like `StoredCount::Unreadable(String)`. It can wrap the parser's own
    error, since the error carries the full text anyway.
- **Errors.** Hand-roll a `Display` and an empty `std::error::Error` impl.
  There is no thiserror and no `kernel` dependency. Adding `kernel` only for
  `kernel::Error` would be a new Cargo dependency, which is pointless here.
- **No serialisable types.** The crate has no serde, so this holds by
  construction.
- **Tests.**
  - The acceptance criteria require an integration test in
    `cli/research/tests/` that uses only public exports. Following the
    one-file-per-module convention, that would be e.g. `tests/tier_text.rs`
    and `tests/sources_section.rs`, with table loops for the rejection,
    acceptance and round-trip lists.
  - The worked example at citing-once `:158-181` makes a natural whole-section
    fixture.
- **Snapshot.** Run `mise run public-api:update` after export, and confirm the
  diff holds additions only. Keep derives minimal, since each one adds
  snapshot lines. PP-1002 also edits this file, and whichever lands second
  regenerates it.

## Code References

- `skills/research/outputters/citing-once/SKILL.md:18-30` - citation shape and tier-text base and suffix
- `skills/research/outputters/citing-once/SKILL.md:64-82` - `also` merge, retraction order, closed vocabulary
- `skills/research/outputters/citing-once/SKILL.md:158-181` - worked example (inputs and merged output)
- `skills/research/outputters/finding-outputter/SKILL.md:80-101` - Sources bullet, `None found.`, suffix wording
- `skills/research/outputters/finding-outputter/SKILL.md:105-154` - metadata block rules and key table
- `templates/topic-research-finding.md:31-33` - Sources is the last section
- `tasks/eval/documents/sources.py:7-13` - `ENTRY`, `BLOCK_LINE`, constants
- `tasks/eval/documents/sources.py:16-49` - `TierText` parse and str
- `tasks/eval/documents/sources.py:69-100` - `sources_section`, `parse_sources`
- `tasks/eval/documents/rule.py:63-74` - `merged_tier`, where unknown marks are rejected
- `tests/unit/tasks/test_citing_once_rule.py:64-106,250-264` - Python parser tests
- `cli/research/Cargo.toml:9-18` - domain/product metadata; only dependency is `network`
- `cli/research/src/lib.rs:1-12` - crate root, no re-exports
- `cli/research/src/sources.rs:1-15` - sources module tree
- `cli/research/src/sources/record.rs:9-24` - `Tier` and its Display
- `cli/research/src/sources/record.rs:48-60` - `Record` with `retracted` and `withdrawn`
- `cli/research/src/sources/tier.rs:1-28` - tier classifiers (module name taken)
- `cli/research/src/sources/metadata.rs:13-33` - `RECOMMENDED_KEYS`
- `cli/research/src/topic/outline.rs:10,28-41,62-83` - whole-document line parser precedent
- `cli/research/src/topic/counts.rs:81-86` - `StoredCount::Unreadable(String)`
- `cli/research/src/topic/layout/stem.rs:14-33,119-127` - as-written round trip
- `cli/research/src/sources/request.rs:17-96,269-272` - error enum carrying raw input
- `cli/shortcut-client/src/shortcut_key.rs:71-105` - `refused` closure idiom
- `cli/research/tests/tier.rs:208-212` - `Tier` display test
- `cli/research/tests/metadata_date.rs:1,13-36` - table-loop test style
- `cli/research/tests/fixtures/public-api.txt:1216-1230,1250-1269` - `Tier` and `Record` snapshot lines
- `tasks/public_api.py:27,116-177` - snapshot render, check and update
- `cli/pup.ron:95-114` - research import allow-list
- `tasks/lint/crate_dependencies.py:92-93,384-397` - product-context edge rules
- `cli/markdown/src/tests.rs:75-76` - CRLF test idiom

## Architecture Insights

- **The two parsers belong to the research bounded context.** The work item's
  "grammar owned by one crate" goal fits ADR-0069's product-context rule: other
  domains cannot depend on `research`, but composition roots such as the
  visualiser server can.
- **Purity and the import fence force hand-written parsing.** This matches
  every other line parser in `cli/`. The trickiest piece without regex is the
  title rule: the title ends at the last `](` for which the rest still parses
  as an entry. The natural approach is to try `](` positions from the right
  until the remainder satisfies URL, then ` — `, then a non-empty tier. A URL
  `x_(y)` must make the line unread, and the venue may contain `](` and
  ` — `.
- **The crate's culture is to model rather than fail.** It keeps suspicious
  input as data (`ProfilesSuffix::Unseparated`, `StoredCount::Unreadable`).
  PP-995's "rejected tier keeps the entry" and "unread lines reported" follow
  the same culture.

## Historical Context

- `meta/work/PP-892-deterministic-rule-for-citing-a-source-once.md`, together
  with `meta/research/codebase/2026-10-28-PP-892-deterministic-rule-for-citing-a-source-once.md`
  and `meta/plans/2026-10-28-PP-892-deterministic-rule-for-citing-a-source-once.md`:
  origin of the citing-once rule, the `also` suffix and the Python
  `sources.py`.
- `meta/work/PP-936-structured-source-metadata.md`, together with
  `meta/research/codebase/2026-10-24-PP-936-structured-source-metadata.md` and
  `meta/plans/2026-10-24-PP-936-structured-source-metadata.md`: origin of the
  metadata block and `RECOMMENDED_KEYS`.
- `meta/research/codebase/2026-09-23-PP-864-academic-source-profiles.md`:
  tiers and the `Tier` enum.
- `meta/decisions/ADR-0069-crate-dependency-rules-for-the-hexagonal-cli-workspace.md`,
  with `meta/research/codebase/2026-10-06-PP-881-crate-dependencies-adr-0069.md`:
  the pup and crate-edge rules.
- `meta/reviews/work/PP-995-sources-parser-in-the-research-crate-review-1.md`
  and `-review-2.md`: both approved on 2026-11-24, after passes that added the
  line rules, the section state, the line span and the token rules.
- `meta/work/PP-1000-sources-rendering-in-the-visualiser.md`,
  `meta/work/PP-1002-derived-set-response-for-topic-research.md` and
  `meta/work/PP-994-generic-set-model-indexing-and-api.md`: the consumer, the
  item that shares the snapshot file, and the item that takes the entry shape
  as input.

## Related Research

- `meta/research/codebase/2026-10-28-PP-892-deterministic-rule-for-citing-a-source-once.md`
- `meta/research/codebase/2026-10-24-PP-936-structured-source-metadata.md`
- `meta/research/codebase/2026-09-23-PP-864-academic-source-profiles.md`
- `meta/research/codebase/2026-10-06-PP-881-crate-dependencies-adr-0069.md`

## Open Questions

These are for planning. None blocks the work.

1. **Line splitting and "one trailing carriage return".** PP-995 drops exactly
   one trailing `\r`. `str::lines()` already strips the `\r` of `\r\n`, so
   `.lines()` followed by `strip_suffix('\r')` would drop two on a line ending
   `\r\r\n`. The alternative, `split('\n')` with one `strip_suffix('\r')`,
   yields a phantom empty last element on a trailing newline. That element
   would distort the span end: the ten-line acceptance criterion expects an
   end of 11. Planning should pick a splitting rule that satisfies both, and
   test it.
2. **Line numbering basis.** The acceptance criteria number lines of "a
   document body". `Outline` numbers whole-file lines, frontmatter included.
   PP-1000 has to know whether the span is body-relative. The work item says
   "body", so the span is relative to whatever string is passed in. This
   should be stated in the type's docs.
3. **Module and type names.** `sources::tier` is taken. Names should follow
   the domain language ("tier text", "Sources section", "cited source",
   "unread line"). PP-994 takes the entry shape as an input, so the names
   become shared vocabulary.
4. **Skill wording drift (not in PP-995's scope).** Finding-outputter `:96-97`
   ("any suffix the profile prescribes") is looser than citing-once `:82` and
   the Rust parser. PP-995 treats the skills as the definition and enforces
   citing-once's closed set. A later skill edit could tighten the wording.
