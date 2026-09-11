---
type: "topic-research"                       # artifact-type discriminator
id: "{subject-slug}"                          # the set directory slug
title: "Research Topic: {Subject}"
date: "{ISO timestamp from accelerator corpus metadata derive}"
author: "{author from VCS}"
producer: "research-topic"
status: "briefed"                             # briefed | outlined | researching | synthesised | complete
kind: "manifest"                              # (type, kind) discriminator
slug: "{subject-slug}"                        # the set directory name
round_count: 0
finding_count: 0
primary: "brief.md"                           # the set's current lead document
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty)
parent: ""                                    # typed-linkage ref: "work-item:NNNN" or ""
relates_to: []                                # typed-linkage list: ["topic-research:<slug>", ...] or []
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

- **Rounds**: [count]
- **Findings**: [count]
- **Primary document**: [the document a reader should open first]
