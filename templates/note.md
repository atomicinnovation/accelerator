---
type: note                                   # artifact-type discriminator
id: "{filename-stem}"                        # filename without .md
title: "{Note title}"
date: "{ISO timestamp from artifact-derive-metadata.sh}"
author: "{author from VCS}"
producer: create-note
status: captured                             # captured
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty)
parent: ""                                   # typed-linkage ref: "work-item:NNNN" or ""
relates_to: []                               # typed-linkage list: ["work-item:NNNN", ...] or []
topic: "{Note topic}"
tags: []
revision: "{commit hash from artifact-derive-metadata.sh}"
repository: "{repo name from artifact-derive-metadata.sh}"
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---

# {Note title}

{The note's body — a short-form observation, insight, or strategy snippet.}
