---
type: "topic-research"                       # artifact-type discriminator
id: "{filename-stem}"                         # filename without .md
title: "Brief: {Subject}"
date: "{ISO timestamp from accelerator corpus metadata derive}"
author: "{author from VCS}"
producer: "research-topic"
status: "draft"                               # draft | complete
kind: "brief"                                 # (type, kind) discriminator
source_profiles: ["web"]                      # the source families this research draws on
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty)
parent: ""                                    # typed-linkage ref: "work-item:NNNN" or ""
relates_to: []                                # typed-linkage list: ["topic-research:NNNN", ...] or []
tags: ["research", "topic-research", "brief"]
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---

# Brief: [Subject]

## Subject
[The subject under research, in one or two sentences.]

## Motivation
[Why this research is being conducted and what decision it informs.]

## Scope
[What is in scope and what is explicitly out of scope.]

## Questions
[The open questions this research must answer.]

## Success Criteria
[What a complete, satisfying dossier looks like.]
