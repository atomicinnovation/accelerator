---
type: work-item                              # artifact-type discriminator
id: "NNNN"                                   # from work-item-next-number.sh; always a quoted string
title: "Title as Short Noun Phrase"
date: "YYYY-MM-DDTHH:MM:SS+00:00"
author: Author Name
producer: create-work-item
status: draft                                # draft | ready | in-progress | review | done | blocked | abandoned
kind: story                                  # story | epic | task | bug | spike
priority: medium                             # high | medium | low
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty)
parent: ""                                   # typed-linkage ref: "work-item:NNNN" or ""
blocks: []                                   # typed-linkage list: ["work-item:NNNN", ...] or []
blocked_by: []                               # typed-linkage list: ["work-item:NNNN", ...] or []
# inverse of blocks — producers SHOULD prefer writing blocks: on the canonical side
derived_from: []                             # typed-linkage list: ["plan:NNNN", ...] or []
relates_to: []                               # typed-linkage list: ["work-item:NNNN", ...] or []
source: ""                                   # typed-linkage ref: "issue-research:NNNN" or ""
external_id: ""                              # cross-system pointer (e.g. Jira/Linear key); omitted when not linked
tags: []
last_updated: "YYYY-MM-DDTHH:MM:SS+00:00"
last_updated_by: Author Name
schema_version: 1
---

# NNNN: Title as Short Noun Phrase

**Kind**: Story | Epic | Task | Bug | Spike
**Status**: Draft
**Priority**: High | Medium | Low
**Author**: Author Name

## Summary

[1-3 sentence description of what this work item is about and why it matters]

## Context

[Background information, forces at play, relevant constraints.
Link to source documents if extracted.]

## Requirements

[For stories/tasks: specific requirements to be met]
[For epics: high-level goals and themes]
[For bugs: reproduction steps, expected vs actual behaviour]
[For spikes: research questions and time-box]

## Acceptance Criteria

- [ ] [Criterion 1 — specific, testable, measurable]
- [ ] [Criterion 2]
- [ ] [Criterion 3]

[For stories, prefer Given/When/Then format where applicable:

- Given [precondition], when [action], then [expected result]]

## Open Questions

- [Question 1 — unresolved business or scope question that affects how work should proceed]

## Dependencies

- Blocked by: [work-item references or external dependencies]
- Blocks: [work items that depend on this one]

## Assumptions

- [Business or technical assumptions that may require validation or clarification]

## Technical Notes

[Optional: implementation hints, relevant code references, architectural considerations discovered during refinement]

## Drafting Notes

- [Interpretations made while drafting — business-context calls, scope decisions, or technical choices that someone should review if they turn out to be wrong]

## References

- Source: `path/to/source-document.md`
- Related: NNNN, NNNN
- Research: `meta/research/codebase/YYYY-MM-DD-topic.md`
