---
type: bug
status: ready
priority: high
---

## Summary
CSV export fails for large datasets

## Context
Multiple enterprise customers have reported that exporting records to CSV fails intermittently. The issue began appearing shortly after the pagination refactor shipped on 2026-04-08. Failures have only been reported by customers with large datasets.

## Requirements
1. The export must complete successfully for datasets of all sizes
2. If the export fails, a user-visible error message must be displayed
3. The exported file must contain all records matching the applied filter

## Acceptance Criteria
- The export completes successfully without error
- Users can download the exported file after triggering the export
- The export produces the correct output
- Large dataset exports do not time out

## Technical Notes
- Export logic lives in `src/services/ExportService.ts` — `exportToCsv()`
- Axios request timeout is 30 seconds in `src/config/http.ts`
- The pagination refactor changed how `offset`/`limit` are passed to the query builder
