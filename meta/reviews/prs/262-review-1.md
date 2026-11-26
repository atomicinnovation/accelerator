---
type: "pr-review"
id: "262-review-1"
title: "[PP-995] Parse a research document's Sources section in the research crate"
date: "2026-11-24T14:24:50+00:00"
author: "Toby Clemson"
producer: "review-pr"
status: "complete"
target: "pr:262"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "standards", "usability"]
review_number: 1
pr_number: 262
tags: []
last_updated: "2026-11-24T14:24:50+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Code Review: #262 - [PP-995] Parse a research document's Sources section in the research crate

**Verdict:** COMMENT

The parser is a small, pure functional core with a total parse, one shared line rule behind numbering and byte offsets, and a behaviour-led suite of 39 public-API tests. There is one real misparse, which happens when a venue holds both a link and a ` — `. A level-one heading does not close the section. The other findings are about the shape of the public API, which is cheapest to change before PP-1000 starts depending on it.

### Cross-Cutting Themes

- **`Present` does not promise entries** (flagged by: code-quality, usability): `closed()` returns `Present` for a heading with nothing under it, or with only unread lines. Every consumer must re-check `entries().is_empty()`, and PP-1000's rendering table already needs a row for that case.
- **`LineSpan` is detached from its body** (flagged by: architecture, usability, test-coverage): `split` and `remove_from` take any `&str` and return a plausible but wrong cut when given a different body, for example one with frontmatter stripped. No test checks the documented no-panic guarantee in that case.
- **Public API shape is locked in by the snapshot** (flagged by: architecture, usability, code-quality): there are three access styles. Some types use `pub` fields, some private fields with getters, and some both public variant fields and accessors. `span`/`unread` are repeated across two variants, and `remove_from` has no planned caller.
- **The tier vocabulary lives away from `Tier`** (flagged by: architecture, code-quality): `BASE_TIERS` hand-copies `Tier`'s variants and parses them by matching against `Display`. `TierText` sits under `topic` although it models `sources` concepts, so a future writer in `sources` would have to import from `topic`.

### Tradeoff Analysis

- **Encapsulation vs construction ergonomics**: Architecture wants `pub(crate)` fields with accessors, as `Record` and `TierText` have. The integration tests and PP-1000's fixtures build `CitedSource` values with struct literals, which private fields would rule out without a constructor. The recommendation is to keep the read models as plain public data, document that choice, and settle on one access style for `SourcesSection`.
- **Smaller surface vs convenience**: Architecture would drop `remove_from` until a caller needs it, while Usability counts it as a convenience. Since PP-1000 now uses `split`, dropping it costs one `format!` at any future call site.

### Strengths

- ✅ `SourcesSection::parse` is a pure, total function that never panics or returns `Result`. Problems come back as data (`UnreadLine`, `TierReading::Rejected`), so a half-broken section still renders.
- ✅ `lines::numbered` and `lines::offset_of` share one `split_inclusive('\n')` rule, so line numbers and byte offsets cannot disagree and every cut lands on a character boundary.
- ✅ `BlockAdjacency` and `SectionLine::classify` keep `read` a flat match. The rule that blank lines keep adjacency and anything else breaks it reads straight from the code.
- ✅ `TierText` shares its separator constants between parsing and `Display`, and its canonical-order check holds `parse(s)?.to_string() == s`. A test pins this over every accepted text.
- ✅ The tests drive public exports only and compare whole values (`CitedSource`, `UnreadLine`, span tuples). They cover CRLF, lone or doubled `\r`, EOF with and without a final newline, and near-miss headings.
- ✅ The change only adds: no new manifest dependency, nothing existing changes in the public-API snapshot, and `Tier` is reused rather than redefined.
- ✅ Naming follows the crate's domain vocabulary and module layout, and the doc comments state only preconditions and invariants.

### General Findings

- 🔵 **Test Coverage**: no committed contract test runs the parser over the repository's real Sources sections. The eval corpus under `skills/research/research-topic/evals/project/meta/research/topics/llm-agent-planning/` has metadata blocks, `tier-3 (retracted)`, `None found.` and sections followed by other content. A test asserting no unread lines, no rejected tiers and a clean `split` round trip would close the PR's one open checklist item.
- 🔵 **Usability**: the new types mix three access styles (`pub` fields, private fields with getters, and both on `SourcesSection`). Say on each opaque type why it is opaque, or pick one documented way to read a section.
- 🔵 **Usability**: an entry that leaves out its tier (`- [T](url) — blog.example`) reports its venue as `Rejected("blog.example")`. This follows the grammar, but the rule that the first segment is always the tier slot should be documented so PP-1000 can word its label.

### Additional Findings

- 🔵 `cli/research/src/topic/sources_section.rs:23` — `span` and `unread` repeated across two variants (code-quality)
- 🔵 `cli/research/src/topic/sources_section.rs:257` — empty block key and wrongly indented block lines untested (test-coverage)
- 🔵 `cli/research/src/topic/sources_section/tier_text.rs:30` — one concept named three ways: refused, `Rejected`, `NotTierText` (standards)
- 🔵 `cli/research/src/topic/sources_section.rs:21` — the line-numbering rule is documented on the enum, not on `UnreadLine::line` or `LineSpan` (usability)
- 🔵 `cli/research/src/topic/sources_section/tier_text.rs:141` — the `NotTierText` message does not say what shape was expected (usability)

---
*Review generated by /accelerator:review-pr*

## Inline Comments

### `cli/research/src/topic/sources_section.rs:194-201` — Rightmost-first title split misparses a venue holding a link then ` — `
**Severity**: minor | **Confidence**: high | **Lens**: correctness

🔵 **Correctness**

`CitedSource::parse` tries `](` positions from the right and takes the first one that reads as an entry. Take `- [T](https://a.example/) — tier-2 — see [x](https://b.example/) — p. 4`. The rightmost candidate reads as an entry, with title ``T](https://a.example/) — tier-2 — see [x``, URL `https://b.example/`, tier `Rejected("p. 4")` and no venue. The correct split is never tried. `a_venue_may_hold_a_markdown_link` and `a_venue_may_itself_hold_the_separator` each cover one feature, but nothing covers the two together.

**Impact**: The result is an entry with the wrong title, URL and tier. It shows up as a tier problem rather than an unread line.

**Suggestion**: Prefer the rightmost candidate whose tier is `TierReading::Parsed`, and fall back to the rightmost candidate that reads at all. That keeps `the_title_ends_at_the_last_link_close_that_still_reads_as_an_entry` green. Add a test for the combined case.

---

### `cli/research/src/topic/sources_section.rs:169-180` — `Present` also covers a section with no entries (code-quality, usability)
**Severity**: minor | **Confidence**: medium | **Lens**: code-quality, usability

🔵 **Code Quality**

`closed()` returns `Present` whenever the section is not `DeclaredEmpty`. That includes a `## Sources` heading with nothing under it, and a section where every line was unread. The name `Present` suggests at least one cited source.

🔵 **Usability**

Matching on the variant does not tell a caller whether there is anything to render. PP-1000 needs a "Present with no entries" row that behaves like `DeclaredEmpty`, and every consumer must know to add that check.

**Suggestion**: Rename the variant so it does not promise entries (e.g. `Listed`), or give the empty, undeclared case its own variant. At minimum, document on `Present` that `entries` may be empty, and name that state in the empty-heading test.

---

### `cli/research/src/topic/sources_section.rs:309-322` — `LineSpan::split`/`remove_from` accept any body and fail silently (architecture, usability)
**Severity**: minor | **Confidence**: medium | **Lens**: architecture, usability

🔵 **Usability**

`split` and `remove_from` work only when given the exact text the span was parsed from. The doc states this, but nothing enforces it. A body with frontmatter stripped, line endings normalised or a fresh re-read returns a plausible but wrong cut with no error.

🔵 **Architecture**

The visualiser server may post-process the body between `parse` and `split`, so a mismatch would fail silently in another crate.

**Suggestion**: Make the common case impossible to get wrong by tying the cut to the parsed body. For example, `parse` could return a value that borrows `&'body str` and offers `split()` with no argument, or the section could keep its byte offsets. The detached `LineSpan` can stay for other uses. At minimum, state on `parse` that lines count from the first byte of the string passed in.

---

### `cli/research/src/topic/sources_section.rs:185-191` — Public fields diverge from the crate's encapsulation convention
**Severity**: minor | **Confidence**: medium | **Lens**: architecture

🔵 **Architecture**

`CitedSource`, `MetadataPair`, `UnreadLine` and the `SourcesSection` variants expose `pub` fields. `sources::record::Record`, and this PR's own `TierText` and `LineSpan`, keep their fields private behind accessors. `SourcesSection` also offers both public variant fields and `entries()`/`unread()`/`span()`.

**Impact**: Every field is now in the public-API snapshot. Downstream crates can build a `CitedSource` that was never parsed, and reshaping these types later is a breaking change across crates.

**Suggestion**: Either make the fields `pub(crate)` with accessors, or keep them public and document these types as plain read models. In both cases, give `SourcesSection` one access style.

---

### `cli/research/src/topic/sources_section/tier_text.rs:5` — Tier text sits under `topic` although it models the `sources` vocabulary
**Severity**: minor | **Confidence**: medium | **Lens**: architecture

🔵 **Architecture**

`TierText` models a base `Tier` plus `retracted`/`withdrawn` marks, which are the same concepts `sources::record::Record` holds. This import is the first dependency from `topic` on `sources`.

**Impact**: A writer that renders tier text from a `Record`, or a check that a cited tier matches its record, would need `sources` to import from `topic`. That creates a module cycle, or else a second copy of the grammar.

**Suggestion**: Put `TierText`/`NotTierText` beside `Tier` (e.g. `sources::tier_text`) and import them from `topic::sources_section`. If the reader-only placement is deliberate, say so in the plan.

---

### `cli/research/src/topic/sources_section.rs:311` — No test checks the no-panic guarantee of `split`/`remove_from`
**Severity**: minor | **Confidence**: medium | **Lens**: test-coverage

🔵 **Test Coverage**

The PR states that a mismatched body gives a wrong cut but never a panic. Every split test passes the same body that produced the span.

**Impact**: A refactor of `lines::offset_of` that counts chars, or slices on lengths after CR stripping, could land an offset past the end or inside a multi-byte character. That would make a public method panic, and no test would fail.

**Suggestion**: Parse a long body, then call `split`/`remove_from` with a shorter body, an empty body and a body with multi-byte characters before a `\n`. Assert that each call returns, and that `before.len() + after.len() <= body.len()`.

---

### `cli/research/src/topic/sources_section.rs:275-281` — No public way to build a `TierReading` from tier text
**Severity**: suggestion | **Confidence**: high | **Lens**: usability

🔵 **Usability**

`TierReading::of` is private. The crate's own `reading()` helper in `tests/sources_section.rs` rewrites the `TierText::parse` → `Parsed`/`Rejected` mapping by hand, and PP-1000's fixtures would have to do the same.

**Suggestion**: Make it public, as `pub fn of(text: &str) -> Self` or `impl From<&str> for TierReading`, and use it in the test helper.

---

### `cli/research/src/topic/sources_section.rs:46` — A level-one heading does not close the Sources section
**Severity**: suggestion | **Confidence**: medium | **Lens**: correctness

🔵 **Correctness**

The section closes only at a line starting `## `. A later `# Heading` stays inside it, along with everything after it. Those lines are reported as unread and included in the `LineSpan`.

**Impact**: `split` and `remove_from` would cut that heading and its content out with the section.

**Suggestion**: Close the section at any heading of level two or higher (`# ` or `## `). If writers never place a level-one heading after Sources, say so where the grammar is documented.

---

### `cli/research/src/topic/sources_section/tier_text.rs:12` — Tier parsing lives outside `Tier`
**Severity**: suggestion | **Confidence**: medium | **Lens**: code-quality

🔵 **Code Quality**

`BASE_TIERS` hand-copies every variant of `crate::sources::record::Tier`, and `base_tier` parses by matching against `Display` output.

**Impact**: If `Tier` gains or renames a variant, `BASE_TIERS` goes stale without warning, and the new tier text is refused as `NotTierText`.

**Suggestion**: Give `Tier` a `FromStr` impl, or an exhaustive `Tier::ALL`, beside its `Display` impl in `record.rs`, and call it from `base_tier`.

---

### `cli/research/src/topic/sources_section.rs:317-322` — `remove_from` enters the public API with no planned consumer
**Severity**: suggestion | **Confidence**: medium | **Lens**: architecture

🔵 **Architecture**

This PR moves PP-1000, the only planned consumer, from `remove_from` to `split`. That leaves `remove_from` as snapshot-locked public API with no caller.

**Suggestion**: Drop it until a consumer needs it, since `split` plus `format!` covers the case. If it stays, name its intended consumer in the plan.

---

## Per-Lens Results

### Architecture

**Summary**: The architecture is sound. A pure functional core with no I/O and no new dependency sits inside the research crate's pup import fence. PP-1000's consumer, the visualiser server, is a composition root allowed to depend on this crate. Four non-blocking points concern the shape and placement of the public surface.

**Strengths**:
- Pure functional core: `parse` and `split` take `&str` and return values.
- One private line rule in `lines.rs`, so consumers never count lines.
- Degrades gracefully through `UnreadLine` and `TierReading::Rejected`.
- The three-state `SourcesSection` puts domain distinctions in the type.
- Additive only: `Cargo.toml` untouched, `Tier` reused.
- Known tradeoffs are stated in the PR notes.

**Comments**:
- `sources_section.rs:185-191` (minor, medium): public fields diverge from the crate's encapsulation convention.
- `sources_section.rs:317-322` (suggestion, medium): `remove_from` has no planned consumer.
- `tier_text.rs:5` (minor, medium): tier text sits under `topic` although it models `sources` vocabulary.
- `sources_section.rs:309-315` (suggestion, low): the detached `LineSpan` relies on a caller precondition the types could enforce.

**General Findings**: none.

### Code Quality

**Summary**: The parser is small, readable and well decomposed, with short functions, guard clauses, named grammar constants and a contextual error type. The findings are minor modelling points: a data clump, a misleading state name, and `Tier` parsing outside `Tier`.

**Strengths**:
- `SectionLine::classify` keeps `read` a flat match.
- `BlockAdjacency` is an explicit enum rather than a bool.
- `lines.rs` gives one rule for numbering and offsets.
- `TierText` shares its constants between parsing and `Display`.
- Pure `Option`-returning helpers.
- Comments are sparse and justified.

**Comments**:
- `sources_section.rs:23-34` (suggestion, medium): `span` and `unread` are repeated across `DeclaredEmpty` and `Present`. Consider `Found(FoundSection { span, unread, content })` before PP-1000 depends on the shape.
- `sources_section.rs:169-180` (minor, medium): `Present` also covers a heading with nothing under it.
- `tier_text.rs:12` (suggestion, medium): tier parsing lives outside `Tier`.

**General Findings**: none.

### Test Coverage

**Summary**: The suite is strong and behaviour-focused: 39 table-driven public-API tests. Mutation probes on the key branches each fail a named test. The gaps are small: no contract test against real writer output, an untested no-panic guarantee, and one key-validation boundary.

**Strengths**:
- Public-API-only tests that survive refactors of private modules.
- Table-driven cases that echo their input in assertion messages.
- Exact whole-value assertions.
- CRLF equivalence checked by comparing whole parses.
- The rightmost-`](` fallback covered from both sides.
- The round-trip invariant pinned over every accepted tier text.

**Comments**:
- `sources_section.rs:311` (minor, medium): no test checks the no-panic guarantee of `split`/`remove_from`.
- `sources_section.rs:257-267` (suggestion, medium): an empty block key (`  - : 10.1/x`) and wrongly indented block lines (`    - doi: x`, ` - doi: x`) are untested. Mutating `is_some_and` to `is_none_or` would survive.

**General Findings**:
- (minor) No committed contract test runs over the repository's real Sources sections in the eval corpus.

### Correctness

**Summary**: The parser is sound overall. Span arithmetic holds at every boundary checked, `split` cannot panic, and `TierText`'s canonical-order check guarantees the round trip. One real misparse remains, plus a heading-level boundary point.

**Strengths**:
- Shared `split_inclusive('\n')` iteration keeps line numbers and offsets in step.
- `end: last + 1` is correct for empty sections, EOF with and without a final newline, and trailing blank lines.
- `next_if_eq` in fixed order refuses empty parentheses, trailing separators, repeated marks and an empty `also`.
- `BlockAdjacency` prevents a block line attaching after prose.

**Comments**:
- `sources_section.rs:194-201` (minor, high): the rightmost-first title split misparses a venue that holds a link followed by ` — `.
- `sources_section.rs:46` (suggestion, medium): a level-one heading does not close the section.

**General Findings**: none.

### Standards

**Summary**: The module closely follows the crate's conventions for imports, module layout, named token constants, infallible `parse`, `pub line: usize` fields, domain-named error types and behavioural test names. Keeping the tests in `tests/` is a deliberate choice to test public exports. One concept is named three ways.

**Strengths**:
- Module layout and `//!` docs match the sibling `topic` modules.
- Named token constants, as in `outline.rs`.
- Rich domain naming.
- Doc comments limited to real preconditions and invariants.
- The snapshot change only adds.
- Test files named after their modules.

**Comments**:
- `tier_text.rs:30` (suggestion, medium): one concept named three ways (`refused`, `Rejected`, `NotTierText`). Rename the local to `rejected`.

**General Findings**: none.

### Usability

**Summary**: The API is small and easy to pick up: one never-failing entry point, a three-state enum, and convenient accessors. The friction for PP-1000 is a detached span that fails silently, a `Present` state that also covers empty sections, a private `TierReading` constructor, and line-numbering docs on the wrong type.

**Strengths**:
- One obvious, never-failing entry point, with partial failures returned as values.
- Accessors cover the common case without a `match`.
- Borrowed `(before, after)` from `split` fits "render in place".
- `TierText` round-trips, and `NotTierText` follows Rust error conventions.
- CRLF is read as LF, so callers need not normalise.

**Comments**:
- `sources_section.rs:309-322` (minor, medium): `split`/`remove_from` accept any body and fail silently on the wrong one.
- `sources_section.rs:169-180` (minor, medium): `Present` also covers a section with no entries.
- `sources_section.rs:275-281` (suggestion, high): no public way to build a `TierReading` from tier text.
- `sources_section.rs:21` (suggestion, medium): the line-numbering rule is documented on the enum instead of on the line-number fields.
- `tier_text.rs:141-145` (suggestion, low): the `NotTierText` message does not say what shape was expected.

**General Findings**:
- (suggestion) The new types mix public fields and accessors.
- (suggestion) An entry that leaves out its tier reports its venue as a rejected tier.

---
*Review generated by /accelerator:review-pr*
