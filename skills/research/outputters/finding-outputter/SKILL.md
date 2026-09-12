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
- **question** — the focus area's question.
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
- `source_profile` is always `web`.
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
  recorded domain or venue:

  ```text
  - [Title](url) — tier-1 — {source domain/venue}
  ```

Tier every source `tier-1`/`tier-2`/`tier-3` by the source profile's vocabulary,
from venue identity alone.
