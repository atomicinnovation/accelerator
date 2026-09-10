---
type: "topic-research"                       # artifact-type discriminator
id: "{filename-stem}"                         # filename without .md
title: "Synthesis: {Subject}"
date: "{ISO timestamp from accelerator corpus metadata derive}"
author: "{author from VCS}"
producer: "research-topic"
status: "complete"                            # complete
kind: "synthesis"                             # (type, kind) discriminator
rounds_covered: 1
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty)
parent: ""                                    # typed-linkage ref: "work-item:NNNN" or ""
relates_to: []                                # typed-linkage list: ["topic-research:NNNN", ...] or []
tags: ["research", "topic-research", "synthesis"]
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---

# Synthesis: [Subject]

## Overview
[The dossier's answer to the brief, in standalone prose. No round narration.]

## Findings
[The synthesised findings, carrying each source's reputation tier and recorded
domain forward.]

## Open Threads
[Questions the research did not close, and where a next round would look.]

## Sources
- [Title](url) — tier-1 — {source domain/venue}
