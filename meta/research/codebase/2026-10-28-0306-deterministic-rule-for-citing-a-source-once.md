---
type: "codebase-research"
id: "2026-10-28-0306-deterministic-rule-for-citing-a-source-once"
title: "Deterministic Rule for Citing a Source Once (0306): current citation pipeline and implementation surface"
date: "2026-10-28T12:29:31+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0306"
parent: "work-item:0306"
relates_to: ["codebase-research:2026-10-24-0348-structured-source-metadata", "codebase-research:2026-09-26-0283-recursive-finding-deepening", "codebase-research:2026-09-23-0280-academic-source-profiles"]
topic: "Deterministic Rule for Citing a Source Once (0306): current citation pipeline and implementation surface"
tags: ["research", "codebase", "composer", "synthesise", "research-topic", "finding-outputter", "sources", "tiers", "metadata"]
revision: "a57f211f711650d49eab1d2c08ef7c60c20efd96"
repository: "accelerator"
last_updated: "2026-10-28T15:33:12+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Added follow-up research for open-question decisions and a shared citing-once rule file"
schema_version: 1
---

# Deterministic Rule for Citing a Source Once (0306): current citation pipeline and implementation surface

**Date**: 2026-10-28T12:29:31+00:00
**Author**: Toby Clemson
**Git Commit**: a57f211f711650d49eab1d2c08ef7c60c20efd96
**Branch**: none (detached jj working copy above `trunk()`)
**Repository**: accelerator

## Research Question

What does the codebase do today with citations, tier text and source metadata
blocks when the composer writes a finding and `synthesise` writes a dossier,
and what must change, and where, to implement 0306's citing-once rule
(`meta/work/0306-deterministic-rule-for-citing-a-source-once.md`)?

## Summary

0306 is a prose-only change. Every part of the citing-once rule is carried
out by an LLM following instructions: no Rust code renders, parses or
deduplicates a Sources entry, its tier text or its metadata block. The
composer has only `Read` and `Write` and cannot call a CLI.

The change touches five files and one Python test module:

| File | Lines | Change |
|---|---|---|
| `agents/composer.md` | 28-35 | narrow verbatim-tier and byte-for-byte rules; state the citing-once rule |
| `skills/research/research-topic/SKILL.md` | 645-656 | same, for `synthesise`, plus `also`-suffix and in-finding duplicate merging |
| `skills/research/outputters/finding-outputter/SKILL.md` | 80-82 | document the `also` suffix beside `(retracted)` |
| `docs-site/src/content/docs/research.md` | 751-756 | drop "byte-for-byte"; describe merged blocks and canonical order |
| `tests/unit/tasks/test_research_structure.py` | 1405-1422 | invert the byte-for-byte carrier test; add citing-once phrase tests |

The test at `test_research_structure.py:1405-1418` asserts exactly the
phrases 0306 removes, so it is the natural red step. The behavioural
acceptance criteria can only be checked by hand-run headless sessions, the
way 0348 was validated. No retained eval exists for research skills.

## Detailed Findings

### Composer instructions today

`agents/composer.md:28-35`, step 3 of "How You Work", is the only citation
rule the composer has:

> **Cite only sources that appear in the injected notes**, carrying each
> source's tier text verbatim, suffix included, and its recorded domain or
> venue. Never add a source, and never re-tier one. Carry each source's
> metadata block byte-for-byte: the same keys, in the same order, with the
> same values, even when it is not in canonical order. Never merge, reorder
> or complete a block. Replace the template's `- {source-metadata}` slot
> line under each source with the note's block lines, or delete that whole
> line where the note has none.

- No dedup rule: nothing says "cite once", "same URL" or "same source". The
  only "merge" word forbids merging *blocks*. It was added during 0348's
  build because merging duplicates belongs to 0306
  (`meta/plans/2026-10-24-0348-structured-source-metadata.md:1700-1701`).
- `(retracted)` and `(withdrawn)` are never named. They are covered only by
  "suffix included".
- Scope limits: read only the outputter and the note paths
  (`agents/composer.md:20-22`, `:50-52`); `tools: Read, Write`
  (`:7`); "You fetch nothing and run nothing" (`:16`).

### `synthesise` instructions today

`skills/research/research-topic/SKILL.md:643-656` (`### synthesise`):

> Carry each tier exactly as its finding writes it, parenthesised suffix
> included — a `tier-3 (retracted)` or `tier-3 (withdrawn)` source stays so
> marked. Carry each cited source's metadata block byte-for-byte from its
> finding, the same keys, order and values even when it is not in canonical
> order, in place of the template's `- {source-metadata}` slot line under
> each source. Never merge, reorder or complete a block. A source cited
> one-line in its finding stays one-line, with that whole slot line deleted.

It differs from the composer in three ways:

- It has no "never add a source" or "never re-tier" sentence.
- It names both suffixes explicitly.
- It does not specify a reading order for findings. It reads top-level
  `findings/` only, never `.levels/`, and runs inline in the orchestrator,
  spawning nothing (`:645-647`).

`synthesise` is the orchestrator skill itself, so it *could* call a CLI. Its
`allowed-tools` (`SKILL.md:16-20`) already include
`Bash(accelerator research topic *)`. The composer could not use such a
command (see Architecture Insights).

### Sources entry and metadata block format

All three templates share one Sources shape:

```markdown
## Sources
- [Title](url) — tier-1 — {source domain/venue}
  - {source-metadata}
```

The shape appears at these locations:

- `templates/topic-research-finding.md:31-33`
- `templates/topic-research-level-note.md:34-36`
- `templates/topic-research-synthesis.md:32-34`

Each part of an entry sits in a fixed place:

- **Tier text** is the second em-dash field. Today it is `tier-N` or
  `tier-N (suffix)`. 0306 adds `(also …)` and the semicolon-joined form.
- **The block** is a two-space-indented `  - key: value` list. `url`, title
  and venue never appear in it (`finding-outputter/SKILL.md:86-96`).
- **Block lines** are the CLI's `metadata` entries, "copied unchanged and in
  emitted order" (`finding-outputter/SKILL.md:90-92`). This rule binds the
  researcher writing a note or a depth-1 finding. It is not the composer's
  rule, and 0306 should leave it alone.
- **Level notes** inherit the finding-outputter's Sources rules unchanged
  (`level-note-outputter/SKILL.md:14-18`), so note and finding citations
  have the same shape.

The metadata table at `finding-outputter/SKILL.md:105-127` is tested
against `RECOMMENDED_KEYS` (`test_research_structure.py:1365-1366`).

### Tier text and its suffixes

The CLI never emits a suffix. `Tier`'s `Display` prints `tier-1`, `tier-2`
and `tier-3` only (`cli/research/src/sources/record.rs:16-24`).
`retracted` and `withdrawn` are separate JSON booleans
(`cli/research-cli/src/render.rs:158-160`, `:173-175`). Profile prose adds
the suffix:

- `skills/research/profiles/openalex-profile/SKILL.md:63`:
  `tier-3 (retracted)`.
- `skills/research/profiles/arxiv-profile/SKILL.md:71-72`:
  `tier-3 (withdrawn)`.
- `skills/research/outputters/finding-outputter/SKILL.md:80-82` is the
  general statement ("carries any suffix the profile prescribes, such as
  `tier-3 (retracted)`"). The `also` suffix documentation belongs here.

Tier derivation (`cli/research/src/sources/tier.rs:8-28`) forces tier-3 for
retracted and withdrawn records. That confirms the work item's claim that
retraction never changes the chosen tier.

Nothing parses tier text back. `Tier` has no `FromStr`. The only fixture
holding a suffixed tier is
`cli/corpus-cli/tests/fixtures/topic-research-multiprofile-set/findings/.01-how-do-attention-heads-specialise-openalex.md.invalid:29`.
So 0306 needs no Rust change for 0281 or 0284 to accept the new form.

### How notes reach the composer

`conduct` spawns the composer (`SKILL.md:439-448`) with these inputs:

- the finding-outputter path
- the finding template
- the question and profile
- the resolved depth
- the `notes` paths
- the output `path`

The CLI decides the order of `notes`:

```text
TreeWalk::walk (cli/research/src/topic/tree.rs:255-297)
  level 1:  1            ← root note, always first
  level 2:  2-1, 2-2, …  ← parents in order, follow-ups by position
  level 3:  3-1-1, …
        │
        ▼ rendered as absolute paths
topic_command.rs:920-924  →  findings/<stem>.levels/<lineage>.md
```

`Lineage` orders by level, then positions numerically: `2-2 < 2-10 < 3-1-1`
(`meta/plans/2026-09-26-0283-recursive-finding-deepening.md:605-639`).

⚠️ Nothing tells the composer that the first path is the root. Today's
observed behaviour of "one kept the root note's tier" therefore came from
reading order, not from an instruction. That is why the nine-run fixture
permutes path order.

### DOI and URL identity: what exists

No source-identity type and no source dedup exist anywhere in `cli/`. The
pieces that border on it are:

| Piece | Location | Relevance |
|---|---|---|
| `Doi::parse` | `cli/research/src/sources/request.rs:372-393` | bare / `doi:` / `http(s)://(dx.)doi.org/`, case-insensitive prefix, cut at `?`/`#`, percent-decode |
| `Doi::canonical()` | `request.rs:397-399` | ASCII-lowercase; this is the form `record_doi` writes into metadata (`metadata.rs:46`) |
| `Doi::url()` | `request.rs:406-408` | `https://doi.org/` + declared case, so cited URLs keep upper case |
| `PageUrl` | `cli/research/src/sources/web/page_url.rs:86-192` | lowercases scheme/host, drops fragment; used for fetching only, never comparison |

These matter to the prose rule in three ways:

- **Metadata DOIs** from the CLI are already lowercase. **URL DOIs** keep
  their declared case. The work item's "DOIs compare case-insensitively"
  covers both, and its `https://dx.doi.org/10.X/Y` criterion exercises the
  URL path.
- 0306's URL normalisation differs from `PageUrl`. It drops the scheme, the
  leading `www.` and the trailing slash, and keeps case in the path. The
  composer cannot call `PageUrl` anyway, so the prose must define the
  normalisation itself, as the work item already does.
- `Doi` derives `PartialEq` on the raw, declared-case string (`:357`). Any
  future Rust identity check must compare `canonical()` forms.

### Canonical key order versus "other keys byte-wise"

`RECOMMENDED_KEYS` is defined at `cli/research/src/sources/metadata.rs:13-33`
and has 19 entries: `doi, arxiv_id, openalex_id, pmid, issn, published,
updated, accessed, type, authors, publisher, volume, issue, pages, license,
journal_ref, primary_category, oa_status, pdf_url`.

`SourceMetadata::record` (`metadata.rs:76-92`) keeps first-write-wins and
inserts by rank. Unknown keys all rank 19 and would *not* sort byte-wise.
That path is unreachable, because `record` is private and only six typed
recorders exist (`:46-68`).

The shipped docs say the CLI emits no other key:

- `finding-outputter/SKILL.md:97-98`
- `docs-site/src/content/docs/research.md:756`

0306's **Block order** clause ("then any other keys byte-wise") is a rule
for agents re-ordering hand-edited or later-producer blocks. It does not
contradict those docs, but the new prose must not imply the CLI emits other
keys. 0349 owns any CLI byte-wise ordering
(`meta/prs/171-description.md:171-173`).

### Tests that pin today's wording

`tests/unit/tasks/test_research_structure.py` pins the current wording in
these tests:

- `:1399-1402` defines `_synthesise()`, the text between `### synthesise`
  and `### Reopening a closed set`.
- `:1405-1418`, `test_every_carrier_keeps_a_metadata_block_byte_for_byte`,
  is parametrised over the composer and `synthesise`. It asserts
  `"metadata block byte-for-byte"`,
  `"even when it is not in canonical order"`,
  `"Never merge, reorder or complete a block"` and the slot-line phrase.
  ❌ All but the slot phrase must disappear under 0306, and the last AC
  requires neither file to keep a byte-for-byte or never-reorder
  instruction. Rewrite this test first, as a negative assertion plus
  positive citing-once phrases.
- `:1421-1422`, `test_synthesise_leaves_a_one_line_citation_one_line`,
  survives only if narrowed. A source cited one-line in one finding but
  with a block in another gains that block under **Metadata merge**.
- `:1369-1379` is the finding-outputter's rule-phrase test, where the
  `also`-suffix examples belong.
- `:1344-1357`, `_recommended_keys()`, regex-reads the Rust constant. Reuse
  it if the composer or `synthesise` prose comes to name the key order
  rather than pointing at the outputter's table.

The style is `assert rule in text, f"... must state {rule!r}"` over
`_prose(path)` or `_section(...)`. The 0283 composer tests
(`test_the_composer_agent_grants_only_read_and_write`,
`test_the_composer_body_names_no_source_family`) constrain the new composer
prose too. The rule must not name OpenAlex, arXiv or other source families.

### Behavioural verification

The behavioural acceptance criteria (≈40 composer and `synthesise`
scenarios, the nine-run determinism fixture and the six-run `synthesise`
check) cannot be unit-tested. 0348's precedent
(`meta/plans/2026-10-24-0348-structured-source-metadata.md:1717-1749`) was:

- Run headless `claude -p --plugin-dir` sessions with `stream-json` logs.
- Run each fixture three times; all three must pass.
- Give each composer or `synthesise` set its own project, because
  `corpus frontmatter validate` refuses an `id` shared by two sets.

The only existing fixture with level notes is
`cli/corpus-cli/tests/fixtures/topic-research-deepened-set/findings/01-first-focus-web.levels/{1,2-1,2-2}.md`.
It can seed the shape of 0306's one-off fixture, but its Sources do not
exercise conflicts.

## Code References

- `agents/composer.md:7` — `tools: Read, Write`; the composer cannot run a CLI.
- `agents/composer.md:20-22` — read only the outputter and notes, as untrusted data.
- `agents/composer.md:28-35` — verbatim-tier and byte-for-byte block rule 0306 narrows.
- `skills/research/research-topic/SKILL.md:16-20` — `allowed-tools`, incl. `accelerator research topic *`.
- `skills/research/research-topic/SKILL.md:439-448` — composer spawn and its inputs.
- `skills/research/research-topic/SKILL.md:643-656` — `synthesise` tier and block carry-forward.
- `skills/research/outputters/finding-outputter/SKILL.md:65-103` — Sources entry and metadata block format.
- `skills/research/outputters/finding-outputter/SKILL.md:80-82` — tier-suffix documentation; `also` goes here.
- `skills/research/outputters/finding-outputter/SKILL.md:105-127` — canonical key table.
- `skills/research/outputters/level-note-outputter/SKILL.md:14-18` — notes inherit finding Sources rules.
- `skills/research/profiles/openalex-profile/SKILL.md:63` — `tier-3 (retracted)` origin.
- `skills/research/profiles/arxiv-profile/SKILL.md:71-72` — `tier-3 (withdrawn)` origin.
- `templates/topic-research-{finding,level-note,synthesis}.md` — shared Sources entry + slot line.
- `docs-site/src/content/docs/research.md:751-756` — user docs restating byte-for-byte and order.
- `cli/research/src/sources/metadata.rs:13-33` — `RECOMMENDED_KEYS`.
- `cli/research/src/sources/metadata.rs:76-92` — first-write-wins, rank insertion.
- `cli/research/src/sources/record.rs:9-24` — `Tier` and `Display` (no suffix).
- `cli/research/src/sources/tier.rs:8-28` — retracted/withdrawn force tier-3.
- `cli/research/src/sources/request.rs:355-409` — `Doi` parse, canonical, url.
- `cli/research/src/topic/tree.rs:255-297` — breadth-first note order.
- `cli/research-cli/src/topic_command.rs:920-924` — `notes` rendered as absolute paths.
- `tests/unit/tasks/test_research_structure.py:1334-1422` — 0348's prose tests 0306 rewrites.

## Architecture Insights

- **The rule lives in prose because the composer cannot run code.** A
  deterministic CLI implementation, such as an
  `accelerator research topic cite-once` over a Markdown body via
  `corpus::body`, would need four things:
  - Bash access for the composer, which breaks 0283's "fetches nothing,
    runs nothing" contract.
  - Post-processing by the orchestrator after each composer returns.
  - A `research` → `corpus` dependency. Today the `research` crate depends
    only on `network` (`cli/research/Cargo.toml:17-18`).
  - A new parser for tier text, which "nothing parses" today.

  The work item chose prose. The nine-run check measures how far prose
  determinism gets.
- **Two copies of one rule.** The AC requires both the composer and
  `synthesise` to state *every* part of the rule. Phrase tests parametrised
  over both carriers, as 0348 did, are the guard against drift between the
  copies.
- **Narrow, don't remove.** The composer's "never add a source" and "never
  re-tier" invariants stay. The new prose states that every tier in merged
  text comes from a note, and that **Merge rule** and **Retraction
  suffixes** are the only tier-text transformations. **Metadata merge**,
  **Full ties** and **Block order** become the only block transformations.
- **The researcher-side rule is untouched.** The finding-outputter's
  "copied unchanged and in emitted order" still governs writers of notes
  and depth-1 findings, and only the carriers change. A depth-1 finding is
  written by the researcher directly, with no composer, so in-note
  duplicates there reach only `synthesise`. That is why `synthesise` must
  merge duplicates within one finding.

## Historical Context

- `meta/plans/2026-10-24-0348-structured-source-metadata.md` — added the
  byte-for-byte rule and "Never merge…" explicitly deferring duplicate
  merging to 0306 (`:106-107`, `:1700-1701`); manual three-run validation
  method (`:1717-1749`).
- `meta/prs/171-description.md` — 0306 needs the `doi` key to match a
  publisher URL to its `doi.org` URL (`:32-34`); non-recommended byte-wise
  order left to 0349 (`:171-173`).
- `meta/reviews/prs/171-review-1.md:173-182` — DOI percent-decode failure
  paths untested; the canonical DOI is the key 0306 uses for identity.
- `meta/reviews/work/0306-deterministic-rule-for-citing-a-source-once-review-1.md`
  — settled: lowest tier + `also`, next-lowest first; retraction as a source
  property; chained DOI/URL identity; same-tier merging; named-venue-first;
  full citation order; full-tie per-field tie-break; one-off nine-run check.
- `meta/plans/2026-09-26-0283-recursive-finding-deepening.md` — lineage
  order (`:605-639`), composer inputs (`:2255-2258`), original
  verbatim-tier contract (`:2138-2139`).
- `meta/work/0305-…`, `meta/work/0311-…` — the other two causes of tier
  conflicts; 0305 must keep `doi` metadata for DOI identity to work.
- `meta/work/0281-…`, `meta/work/0284-…` — future consumers of the `also`
  suffix.

## Related Research

- `meta/research/codebase/2026-10-24-0348-structured-source-metadata.md`
- `meta/research/codebase/2026-09-26-0283-recursive-finding-deepening.md`
- `meta/research/codebase/2026-09-23-0280-academic-source-profiles.md`

## Open Questions

- ❓ **Docs-site scope**: `docs-site/src/content/docs/research.md:751-756`
  is not listed in the work item's Technical Notes but contradicts 0306
  once shipped. Updating it means running `mise run docs:check`.
- ❓ **Wording "other keys byte-wise"**: decide how to state it without
  implying that today's CLI emits non-recommended keys. Shipped docs say it
  emits none.
- ❓ **One-line test**: decide whether
  `test_synthesise_leaves_a_one_line_citation_one_line` is narrowed (only
  when no citation of the source carries a block) or deleted.
- ❓ **Where the canonical order is named**: the carriers could point at the
  finding-outputter's table rather than restating 19 keys. The composer
  already receives the outputter path. `synthesise` would need to read the
  outputter, which it does not do today.
- Not checked: whether any existing finding in a real `meta/research/topics/`
  set already has duplicate citations. The directory is empty in this
  repository.

## Follow-up Research 2026-10-28T15:33:12+00:00

### Decisions on the open questions

| Question | Decision |
|---|---|
| Docs-site scope | Update `docs-site/src/content/docs/research.md:751-756` in 0306; run `mise run docs:check` |
| "Other keys byte-wise" wording | State it as a property of the written block, not of the CLI (below) |
| One-line test | Delete `test_synthesise_leaves_a_one_line_citation_one_line` |
| Where the rule lives | One shared, path-injected rule file both carriers read (below) |

### Wording for "other keys byte-wise"

Keep the finding-outputter's CLI-facing sentence ("The CLI emits only
recommended keys, in table order") unchanged. Phrase **Block order** as a
property of the block the carrier writes:

```text
List a block's keys in table order; any key the table does not list
follows every table key, in byte-wise order.
```

The sentence is true whatever produced the key: a hand edit, or a later
producer such as 0349. It makes no claim about what the CLI emits.

### A shared citing-once rule file

Research skills already share instructions through **path-injected
contracts**. These are non-invocable skills (`user-invocable: false`,
`disable-model-invocation: true`) whose absolute path an orchestrator
passes to an agent:

- The composer gets
  `${CLAUDE_PLUGIN_ROOT}/skills/research/outputters/finding-outputter/SKILL.md`
  (`skills/research/research-topic/SKILL.md:442`).
- The researcher gets profile and outputter paths the same way (`:412-424`).

Agent definitions do not expand `!` commands. Only `agents/documents-locator.md`
uses a `skills:` preload, and that is for `accelerator:paths`. Path
injection is the established mechanism.

Proposed layout:

```text
skills/research/outputters/citing-once/SKILL.md   ← every part of the rule,
        ▲                        ▲                   key list, examples
        │ injected path          │ read inline
        │ (conduct spawn :442)   │ (synthesise :645)
agents/composer.md        research-topic synthesise
  keeps: cite only note     keeps: read findings/, never .levels/;
  sources; every tier       apply the rule across every citation,
  from a note; the rule's   including duplicates in one finding and
  transformations are the   existing `also` suffixes
  only ones
```

The rule file states **Same source** through **Block order**, with the
`tier-3 (also tier-2, tier-1)` and `tier-3 (retracted; withdrawn; also tier-1)`
examples. It lists the 19 keys in order, so `synthesise` needs no second
file. `_recommended_keys()` pins that list to `RECOMMENDED_KEYS`, in the
same way the outputter table is pinned today
(`test_research_structure.py:1365-1366`).

These changes follow from the move:

- **`research-topic` conduct spawn** (`SKILL.md:439-448`): inject the rule
  path beside the outputter path.
- **`synthesise`** (`SKILL.md:643-656`): read the rule path before writing
  `synthesis.md`.
- **Composer read scope** (`agents/composer.md:20-22`, `:50-52`): permit the
  rule path alongside the outputter and notes.
- **Conformance** (`tests/integration/conformance/test_conformance.py:50-58`):
  add the file to the injected-outputter list, exempt from the emitter
  check like `finding-outputter`.
- **Phrase tests**: assert every rule part once, on the rule file. Assert
  that both carriers reference its path and that neither keeps
  "byte-for-byte" or "never reorder". Assert the conduct spawn injects the
  path.

The rule lives under `outputters/` because it is part of the Sources output
contract and that directory already holds the injected contracts. A new
`skills/research/rules/` group would need the same registration and add a
second concept.

⚠️ The move contradicts 0306 as written in two places. The work item needs
amending before planning:

- The **Requirements** bullet at `meta/work/0306-…md:122-125` says the
  composer definition and `synthesise` "both state every part" of the rule.
- The last acceptance criterion (`:337-343`) says the same, plus that the
  composer definition states the narrowed tier invariants.

Reword both to: the two carriers apply one shared rule file, which states
every part. The composer definition keeps the narrowed invariants:

- every tier comes from a note;
- **Merge rule** and **Retraction suffixes** are the only tier-text changes;
- **Metadata merge**, **Full ties** and **Block order** are the only block
  changes.
