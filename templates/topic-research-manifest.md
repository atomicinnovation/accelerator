---
type: "topic-research"                       # artifact-type discriminator
id: "{filename-stem}"                         # filename without .md
title: "Research Topic: {Subject}"
date: "{ISO timestamp from accelerator corpus metadata derive}"
author: "{author from VCS}"
producer: "research-topic"
status: "complete"                            # complete
kind: "manifest"                              # (type, kind) discriminator
slug: "{subject-slug}"                        # the set directory name
research_status: "briefed"                    # briefed | outlined | researching | synthesised
round_count: 0
finding_count: 0
primary: "brief.md"                           # the set's current lead document
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty)
parent: ""                                    # typed-linkage ref: "work-item:NNNN" or ""
relates_to: []                                # typed-linkage list: ["topic-research:NNNN", ...] or []
tags: ["research", "topic-research", "manifest"]
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---

# Research Topic: [Subject]

## Set

- **Brief**: `brief.md`
- **Outline**: `outline.md`
- **Findings**: `findings/`
- **Synthesis**: `synthesis.md`

## Status

- **Research status**: [briefed | outlined | researching | synthesised]
- **Rounds**: [count]
- **Findings**: [count]
- **Primary document**: [the document a reader should open first]
