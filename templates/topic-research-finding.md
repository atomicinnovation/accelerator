---
type: "topic-research"                       # artifact-type discriminator
id: "{filename-stem}"                         # filename without .md
title: "Finding: {Focus Area Question}"
date: "{ISO timestamp from accelerator corpus metadata derive}"
author: "{author from VCS}"
producer: "research-topic"
status: "complete"                            # complete
kind: "finding"                               # (type, kind) discriminator
round: 1
question: "{focus area question}"
source_profile: "web"
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty)
parent: ""                                    # typed-linkage ref: "work-item:NNNN" or ""
relates_to: []                                # typed-linkage list: ["topic-research:NNNN", ...] or []
tags: ["research", "topic-research", "finding"]
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---

# Finding: [Focus Area Question]

## Question
[The focus area's question, restated.]

## Findings
[Standalone research prose. No round narration — the finding reads on its own.]

## Sources
- [Title](url) — tier-1 — {source domain/venue}
