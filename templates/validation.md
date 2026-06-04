---
type: plan-validation                        # artifact-type discriminator
id: "{filename-stem}"                        # e.g. "2026-05-18-0042-some-plan-validation"
title: "Validation Report: {Plan Name}"
date: "{ISO timestamp}"
author: "{author from VCS}"
producer: validate-plan
status: complete                             # complete
result: ""                                   # pass | partial | fail (filled by validate-plan)
# typed-linkage slots — omit-when-empty in artifacts (drop any left empty)
parent: ""                                   # typed-linkage ref: "plan:NNNN" or ""
target: ""                                   # typed-linkage ref: "plan:NNNN" or ""
relates_to: []                               # typed-linkage list: ["plan-validation:NNNN", ...] or []
tags: []
last_updated: "{ISO timestamp}"
last_updated_by: "{author from VCS}"
schema_version: 1
---

## Validation Report: [Plan Name]

### Implementation Status

✓ Phase 1: [Name] - Fully implemented
✓ Phase 2: [Name] - Fully implemented
⚠️ Phase 3: [Name] - Partially implemented (see issues)

### Automated Verification Results

✓ Build passes: `make build`
✓ Tests pass: `make test`
✗ Linting issues: `make lint` (3 warnings)

### Code Review Findings

#### Matches Plan:

- Database migration correctly adds [table]
- API endpoints implement specified methods
- Error handling follows plan

#### Deviations from Plan:

- Used different variable names in [file:line]
- Added extra validation in [file:line] (improvement)

#### Potential Issues:

- Missing index on foreign key could impact performance
- No rollback handling in migration

### Manual Testing Required:

1. UI functionality:
  - [ ] Verify [feature] appears correctly
  - [ ] Test error states with invalid input

2. Integration:
  - [ ] Confirm works with existing [component]
  - [ ] Check performance with large datasets

### Recommendations:

- Address linting warnings before merge
- Consider adding integration test for [scenario]
- Document new API endpoints
