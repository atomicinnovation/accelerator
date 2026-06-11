---
type: adr                                    # artifact-type discriminator
id: "ADR-NNNN"                               # always a quoted string
title: "Title as Short Noun Phrase"
date: "YYYY-MM-DDTHH:MM:SS+00:00"
author: Author Name
producer: create-adr
status: proposed                             # proposed | accepted | rejected | superseded | deprecated
decision_makers: []                          # omitted when empty
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty)
parent: ""                                   # typed-linkage ref: "work-item:NNNN" or ""
supersedes: []                               # typed-linkage list: ["adr:ADR-NNNN", ...] or []
relates_to: []                               # typed-linkage list: ["adr:ADR-NNNN", ...] or []
tags: [tag1, tag2]
last_updated: "YYYY-MM-DDTHH:MM:SS+00:00"
last_updated_by: Author Name
schema_version: 1
---

# ADR-NNNN: Title as Short Noun Phrase

**Date**: YYYY-MM-DD
**Status**: Proposed
**Author**: Author Name

## Context

[Forces at play — technological, political, social, project-specific.
Value-neutral language describing facts, not advocating.]

## Decision Drivers

- [Driver 1]
- [Driver 2]

## Considered Options

1. **Option A** — Brief description
2. **Option B** — Brief description
3. **Option C** — Brief description

## Decision

[The chosen option and why, stated in active voice: "We will..."]

## Consequences

### Positive

- [Consequence 1]

### Negative

- [Consequence 1]

### Neutral

- [Consequence 1]

## References

- `meta/research/codebase/YYYY-MM-DD-topic.md` — Related research
- `meta/decisions/ADR-NNNN.md` — Related/superseded ADR
