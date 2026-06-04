---
type: pr-description                         # artifact-type discriminator
id: "{pr_number}"                            # PR number as a quoted YAML string
title: "{PR Title}"
date: "{ISO timestamp}"
author: "{author from VCS}"
producer: describe-pr
status: complete                             # complete
work_item_id: ""                             # foreign reference; omitted when no linked work item
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty)
parent: ""                                   # typed-linkage ref: "work-item:NNNN" or ""
relates_to: []                               # typed-linkage list: ["work-item:NNNN", ...] or []
pr_url: ""                                   # omitted until populated from `gh pr view`
pr_number: {number}                          # bare integer
merge_commit: ""                             # omitted until merged
tags: []
revision: "{commit hash from artifact-derive-metadata.sh}"
repository: "{repo name from artifact-derive-metadata.sh}"
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---

# {PR Title}

## Summary

[1-3 sentence overview of what this PR does and why]

## Changes

- [Key change 1]
- [Key change 2]
- [Key change 3]

## Context

[Link to relevant work item, plan, or research document if applicable]

## Testing

- [ ] [How the changes were tested]
- [ ] [Edge cases considered]

## Notes for Reviewers

[Any specific areas to focus on, known limitations, or follow-up work planned]
