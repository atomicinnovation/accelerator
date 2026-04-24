---
title: "Add CSV export to reports dashboard and migrate to new grid framework"
type: story
status: ready
priority: medium
---

# Add CSV Export to Reports Dashboard and Migrate to New Grid Framework

## Summary

Add a CSV export button to the reports dashboard so users can download filtered
report data, and migrate the dashboard from the legacy table component to the
new grid framework.

## Context

Users have requested CSV export functionality for months — it is the most
upvoted item in the feedback tracker. The new grid framework migration is
required for the upcoming multi-column sort feature.

## Requirements

1. Add a "Download CSV" button to the reports dashboard toolbar. Clicking it
   should export the currently filtered view (respecting all active filters and
   date ranges) as a CSV file, prompting a browser download.
2. CSV column headers should match the visible column labels. Timestamps should
   be ISO 8601 format. Currency values should be unformatted numeric strings.
3. Replace the existing `<LegacyDataTable>` component with `<GridView>` from
   the new grid framework. The new framework requires a complete rewrite of the
   sorting, pagination, and column-resize logic, estimated at 3–4 weeks.
4. All existing dashboard filter functionality must continue to work after the
   migration.

## Acceptance Criteria

- The CSV export button appears in the dashboard toolbar and is visible to all
  users with read access.
- Exporting a filtered view produces a correctly formatted CSV with expected
  data (verified by automated test with a fixture dataset).
- The dashboard renders correctly with the new `<GridView>` component, with all
  filters, sorting, and pagination functional.
- No regression in report loading time after the migration (within 10%).

## Dependencies

- New grid framework package `@company/grid-view` >= 2.0 (available in npm)
- CSV export backend endpoint (already exists at `/api/reports/export`)
