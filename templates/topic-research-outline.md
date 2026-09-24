---
type: "topic-research"                       # artifact-type discriminator
id: "{filename-stem}"                         # filename without .md
title: "{Subject}"
date: "{ISO timestamp from accelerator corpus metadata derive}"
author: "{author from VCS}"
producer: "research-topic"
status: "complete"                            # complete
kind: "outline"                               # (type, kind) discriminator
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty)
parent: ""                                    # typed-linkage ref: "work-item:NNNN" or ""
relates_to: []                                # typed-linkage list: ["topic-research:NNNN", ...] or []
tags: ["research", "topic-research", "outline"]
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---

# [Subject]

## Focus Areas

[One focus area per line; each becomes a finding. This is the working log —
checkboxes track which focus areas have been researched.]

## Round 1

- [ ] [Focus area question] — profiles: [profile, …]
- [ ] [Focus area question] — profiles: [profile, …]
