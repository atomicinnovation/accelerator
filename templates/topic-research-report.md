---
type: "topic-research"                       # artifact-type discriminator
id: "{set-slug}-report-{report-stem}"         # unique across the corpus
title: "{Question}"
date: "{ISO timestamp from accelerator corpus metadata derive}"
author: "{author from VCS}"
producer: "research-topic"
status: "complete"                            # complete
kind: "report"                                # (type, kind) discriminator
question: "{question}"
rounds_covered: 1
coverage: "answered"                          # answered | partial
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty)
parent: ""                                    # typed-linkage ref: "work-item:NNNN" or ""
relates_to: []                                # typed-linkage list: ["topic-research:NNNN", ...] or []
tags: ["research", "topic-research", "report"]
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---

# [Question]

## Answer
[Standalone prose answering the question from the findings. No round narration.]

## Coverage
[The coverage judgement, answered or partial, and the sources that cover each
part of the question.]

## Gaps
[Partial reports only: each part of the question the corpus leaves open, and
the sources behind it. Delete this section when coverage is answered.]

## Sources
- [Title](url) — tier-1 — {source domain/venue}
  - {source-metadata}
