---
name: finding-outputter
description: Output contract for a research finding. Owns the finding's
  frontmatter, sections, tier-tagged sources, and write sink. Injected by
  research-topic's conduct verb into the researcher — not invoked directly.
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

- **output path** — where to write the finding.
- **round** — the round number this finding belongs to.
- **question** — the focus area's question.
- **timestamp** — the ISO datetime for `date` and `last_updated`.
- **author** — the value for `author` and `last_updated_by`.

## Frontmatter

The field contract is the `(topic-research, finding)` schema row and its
`topic-research-finding` template — follow those, do not restate them here. This
block is the authoring scaffold: `status` is always `complete`,
`source_profile` is `web`, and there is **no `revision`/`repository`** pair (a
finding is not code-state-anchored).

```text
---
type: "topic-research"
id: "{output filename without .md}"
title: "Finding: {focus question}"
date: "{injected timestamp}"
author: "{injected author}"
producer: "research-topic"
status: "complete"
kind: "finding"
round: {injected round}
question: "{injected question}"
source_profile: "web"
tags: ["research", "topic-research", "finding"]
last_updated: "{injected timestamp}"
last_updated_by: "{injected author}"
schema_version: 1
---
```

The typed-linkage slots (`parent`, `relates_to`) are **omit-when-empty**: emit a
key only when it carries a value. A written finding never carries `parent: ""`
or `relates_to: []` — the validator rejects those literal empties. Drop them.

## Body

Three sections, in this order:

- **`## Question`** — the focus area's question, restated.
- **`## Findings`** — standalone research prose that answers the question. It
  reads on its own: **no round narration**, no "in this round", no changelog of
  what you did. A reader who opens only this finding must understand it.
- **`## Sources`** — one bullet per source, each carrying its reputation tier
  and recorded domain or venue:

  ```text
  - [Title](url) — tier-1 — {source domain/venue}
  ```

Tier every source `tier-1`/`tier-2`/`tier-3` by the source profile's vocabulary,
from venue identity alone.
