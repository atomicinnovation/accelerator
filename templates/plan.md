---
type: plan                                   # artifact-type discriminator
id: "{filename-stem}"                        # filename without .md (e.g. "2026-05-30-0042-some-feature")
title: "{Feature/Task Name} Implementation Plan"
date: "{ISO timestamp}"
author: "{author from VCS}"
producer: create-plan
status: draft                                # draft | ready | in-progress | done
work_item_id: ""                             # foreign reference; omitted when no linked work item
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty)
parent: ""                                   # typed-linkage ref: "work-item:NNNN" or ""
blocks: []                                   # typed-linkage list: ["plan:NNNN", ...] or []
blocked_by: []                               # typed-linkage list: ["plan:NNNN", ...] or []
# inverse of blocks — producers SHOULD prefer writing blocks: on the canonical side
derived_from: []                             # typed-linkage list: ["codebase-research:NNNN", ...] or []
relates_to: []                               # typed-linkage list: ["plan:NNNN", ...] or []
reviewer: ""                                 # omitted until reviewed
tags: []
revision: "{commit hash from artifact-derive-metadata.sh}"
repository: "{repo name from artifact-derive-metadata.sh}"
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---

# [Feature/Task Name] Implementation Plan

## Overview

[Brief description of what we're implementing and why]

## Current State Analysis

[What exists now, what's missing, key constraints discovered]

## Desired End State

[A Specification of the desired end state after this plan is complete, and how to verify it]

### Key Discoveries:

- [Important finding with file:line reference]
- [Pattern to follow]
- [Constraint to work within]

## What We're NOT Doing

[Explicitly list out-of-scope items to prevent scope creep]

## Implementation Approach

[High-level strategy and reasoning]

## Phase 1: [Descriptive Name]

### Overview

[What this phase accomplishes]

### Changes Required:

#### 1. [Component/File Group]

**File**: `path/to/file.ext`
**Changes**: [Summary of changes]

```[language]
// Specific code to add/modify
```

### Success Criteria:

#### Automated Verification:

- [ ] Migration applies cleanly: `make migrate`
- [ ] Unit tests pass: `make test-component`
- [ ] Type checking passes: `npm run typecheck`
- [ ] Linting passes: `make lint`
- [ ] Integration tests pass: `make test-integration`

#### Manual Verification:

- [ ] Feature works as expected when tested via UI
- [ ] Performance is acceptable under load
- [ ] Edge case handling verified manually
- [ ] No regressions in related features

---

## Phase 2: [Descriptive Name]

[Similar structure with both automated and manual success criteria...]

---

## Testing Strategy

### Unit Tests:

- [What to test]
- [Key edge cases]

### Integration Tests:

- [End-to-end scenarios]

### Manual Testing Steps:

1. [Specific step to verify feature]
2. [Another verification step]
3. [Edge case to test manually]

## Performance Considerations

[Any performance implications or optimizations needed]

## Migration Notes

[If applicable, how to handle existing data/systems]

## References

- Original work item: `meta/work/NNNN-title.md`
- Related research: `meta/research/codebase/[relevant].md`
- Similar implementation: `[file:line]`
