---
type: story
status: ready
priority: medium
---

## Summary
Add full-text search to the admin user management dashboard

## Context
The admin dashboard has no search capability. Support team members manually scroll through paginated user lists to find accounts. With 50,000+ users, this can take up to 15 minutes per support ticket. Adding search should bring that down to under 30 seconds per lookup.

## Requirements
1. The admin dashboard must include a search field in the user management section
2. Search must support querying by email address, username, and display name
3. Results must update as the user types, without requiring a form submission
4. Search must be functional across all major browsers

## Acceptance Criteria
- Search results appear quickly when the user starts typing
- The search handles edge cases and unusual input appropriately
- Results are relevant to the query the user entered
- The feature works correctly on all supported browsers

## Dependencies
- User management API (read-only, existing)
- Admin frontend React app (existing)

## Assumptions
- The user management API already supports filtering by email, username, and display name
