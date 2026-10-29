---
name: finding-outputter
description: Output contract for a research finding. Owns the finding's
  frontmatter, sections, tier-tagged sources, and write sink. Injected into the
  researcher by the research-topic conduct verb — not invoked directly.
user-invocable: false
disable-model-invocation: true
---

# Finding Outputter

You write **one finding** for a single focus area. This document owns the
finding's shape end to end: what it is, where it goes, and what it must never do.

## Sink

Write the finding to the **single output path injected into your task prompt**,
and to no other path.

## Injected Values

Your task prompt injects everything the frontmatter needs — do not invent or
derive it:

- **finding template** — the resolved `topic-research-finding` template to render.
- **output path** — where to write the finding.
- **round** — the round number this finding belongs to.
- **depth** — the resolved depth the finding was researched or composed under.
- **question** — the focus area's question.
- **source profile** — the profile this finding was researched through.
- **timestamp** — the ISO datetime for `date` and `last_updated`.
- **author** — the value for `author` and `last_updated_by`.

## Shape

The `topic-research-finding` template is the finding's shape — frontmatter and
body skeleton both — and is injected into your task prompt. Fill its
placeholders from the injected values. If no template was injected, refuse
rather than improvise a shape.

Fill the template from the injected values, observing the deltas it cannot
express on its own:

- `status` is always `complete`.
- `source_profile` is always the injected source profile, and `question` the
  injected question byte for byte, each overriding any value the template
  carries.
- Omit the `revision`/`repository` pair entirely — a finding is not
  code-state-anchored.
- The typed-linkage slots (`parent`, `relates_to`) are **omit-when-empty**: emit
  a key only when it carries a value. Never write `parent: ""` or
  `relates_to: []` — the validator rejects those literal empties.

## Body

The template you loaded is authoritative for the finding's sections — follow
its structure. The guidance below is about the *content* to write, not a fixed
set of headings: where the template has a matching section, fill it out this
way, and use your judgement for any section the template shapes differently.

- **Question** — the focus area's question, restated.
- **Findings** — standalone research prose that answers the question. It reads
  on its own: **no round narration**, no "in this round", no changelog of what
  you did. A reader who opens only this finding must understand it.
- **Sources** — one bullet per source, each carrying its reputation tier and
  recorded domain or venue, and the source's metadata block when it has one
  (see **Sources Metadata Block**):

  ```text
  - [Title](url) — tier-1 — {source domain/venue}
    - doi: 10.7717/peerj.4375
    - openalex_id: W2741809807
    - published: 2018-02-13
    - accessed: 2026-10-24
  ```

  When the profile's sources held nothing relevant, Sources reads
  `None found.` instead.

Tier every source `tier-1`/`tier-2`/`tier-3` by the source profile's vocabulary,
from venue identity alone. The tier text carries any suffix the profile
prescribes, such as `tier-3 (retracted)`.

A composed finding or dossier cites each source once, under the citing-once
rule, and lists the other tiers the source was cited at in an `also` suffix,
after any `retracted` or `withdrawn` mark: `tier-3 (also tier-2, tier-1)`,
`tier-3 (retracted; withdrawn; also tier-1)`.

## Sources Metadata Block

A Sources entry may carry a **metadata block**: a nested `  - key: value`
list directly under its line. The block is optional, and an entry without one
is valid. A researcher writes each block from the CLI's output, under the
rules below.

- Block lines are the CLI's `metadata` entries for the cited source, copied
  unchanged and in emitted order. Never write a block line from anywhere
  else: not from a page, an abstract, a record's other fields, or memory.
- Keys are lowercase `snake_case`, values are single-line, and a key with no
  value is omitted.
- `url`, title and venue never appear in the block: they stay on the entry
  line.
- The CLI emits only recommended keys, in table order. Copying in emitted
  order keeps that order.
- The template's `  - {source-metadata}` slot line repeats under every
  Sources entry. Under each, it becomes one `  - key: value` line per entry
  of that source's `metadata` map, in emitted order. When the source has no
  map, delete the whole slot line: never leave it as written, and never fill
  it from anything else.

A composer writes each block from the notes' blocks instead, under the
citing-once rule's **Metadata merge**, **Full ties** and **Block order**.
Every line still comes from a block a note carries.

The recommended keys, in order:

| Key | Group | Value form |
|---|---|---|
| `doi` | identifier | bare and lowercase: `10.7717/peerj.4375` |
| `arxiv_id` | identifier | bare, with the version retrieved: `2305.12345v2` |
| `openalex_id` | identifier | the bare `W` ID: `W2741809807` |
| `pmid` | identifier | |
| `issn` | identifier | |
| `published` | date | ISO 8601 at the precision the source declares: `2023`, `2023-05` or `2023-05-12` |
| `updated` | date | as `published` |
| `accessed` | date | the UTC date of the CLI call the map came from: `2026-10-24` |
| `type` | bibliographic | |
| `authors` | bibliographic | |
| `publisher` | bibliographic | |
| `volume` | bibliographic | |
| `issue` | bibliographic | |
| `pages` | bibliographic | |
| `license` | bibliographic | |
| `journal_ref` | source-specific | |
| `primary_category` | source-specific | |
| `oa_status` | source-specific | |
| `pdf_url` | source-specific | |

The CLI fills the core keys, those with a value form above, wherever the
source provides them. `journal_ref` is a citation string, not a venue, so it
does not duplicate the entry line.
