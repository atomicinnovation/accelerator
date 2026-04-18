---
ticket_id: NNNN                              # from ticket-next-number.sh
date: "YYYY-MM-DDTHH:MM:SS+00:00"            # date -u +%Y-%m-%dT%H:%M:%S+00:00
author: Author Name                          # your name or GitHub handle
type: story                                  # story | epic | task | bug | spike
status: draft                                # draft | ready | in-progress | review | done | blocked | abandoned
priority: medium                             # high | medium | low
parent: ""                                   # ticket number of the parent epic/story, or empty
tags: []                                     # YAML array, e.g. [backend, performance]
---

# NNNN: Title as Short Noun Phrase

**Type**: Story | Epic | Task | Bug | Spike
**Status**: Draft
**Priority**: High | Medium | Low
**Author**: Author Name

## Summary

[1-3 sentence description of what this ticket is about and why it matters]

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

## Dependencies

- Blocked by: [ticket references or external dependencies]
- Blocks: [tickets that depend on this one]

## Technical Notes

[Optional: implementation hints, relevant code references,
architectural considerations discovered during refinement]

## References

- Source: `path/to/source-document.md`
- Related: NNNN, NNNN
- Research: `meta/research/YYYY-MM-DD-topic.md`
