---
type: codebase-research                      # artifact-type discriminator
id: "{filename-stem}"                        # filename without .md
title: "Research: {User's Question/Topic}"
date: "{ISO timestamp from artifact-derive-metadata.sh}"
author: "{author from VCS}"
producer: research-codebase
status: complete                             # complete
work_item_id: ""                             # foreign reference (optional)
topic: "{User's Question/Topic}"
tags: [research, codebase, relevant-component-names]
revision: "{commit hash from artifact-derive-metadata.sh}"
repository: "{repo name from artifact-derive-metadata.sh}"
last_updated: "{ISO timestamp}"
last_updated_by: "{Researcher name}"
schema_version: 1
---

# Research: [User's Question/Topic]

**Date**: [Current date and time with timezone from step 4]
**Author**: [Author name from VCS]
**Git Commit**: [Current commit hash from step 4]
**Branch**: [Current branch name from step 4]
**Repository**: [Repository name]

## Research Question
[Original user query]

## Summary
[High-level findings answering the user's question]

## Detailed Findings

### [Component/Area 1]
- Finding with reference ([file.ext:line](link))
- Connection to other components
- Implementation details

### [Component/Area 2]
...

## Code References
- `path/to/file.py:123` - Description of what's there
- `another/file.ts:45-67` - Description of the code block

## Architecture Insights
[Patterns, conventions, and design decisions discovered]

## Historical Context
[Relevant insights from research, plans, and decisions directories]
- `meta/decisions/some-decision.md` - Historical decision about X
- `meta/research/codebase/past-exploration.md` - Past exploration of Y

## Related Research
[Links to other research documents in the research directory]

## Open Questions
[Any areas that need further investigation]
