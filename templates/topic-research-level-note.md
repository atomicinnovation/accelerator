---
type: "topic-research"                       # artifact-type discriminator
id: "{set-slug}.{finding-stem}.{lineage}"     # unique across the corpus
title: "{Node Question}"
date: "{ISO timestamp from accelerator corpus metadata derive}"
author: "{author from VCS}"
producer: "research-topic"
status: "complete"                            # complete
kind: "level-note"                            # (type, kind) discriminator
round: 1
question: "{node question}"
source_profile: "{source profile}"
level: 1
depth: 1
follow_ups: ["{follow-up question}"]
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty)
parent: ""                                    # typed-linkage ref: "work-item:NNNN" or ""
relates_to: []                                # typed-linkage list: ["topic-research:NNNN", ...] or []
tags: ["research", "topic-research", "level-note"]
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---

# [Node Question]

## Question
[The node's question, restated.]

## Findings
[Standalone research prose answering the node's question.]

## Sources
- [Title](url) — tier-1 — {source domain/venue}
