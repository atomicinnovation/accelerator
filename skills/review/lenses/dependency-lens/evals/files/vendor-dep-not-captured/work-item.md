---
title: "Fix broken CRM contact export"
type: bug
status: ready
priority: high
---

# Fix Broken CRM Contact Export

## Summary

The "Export contacts to CRM" feature is returning an error for all users.
The export calls the HubSpot CRM API and the response shape changed in
HubSpot's v3 API; the platform is still sending requests in v2 format and
parsing v2 responses.

## Context

The CRM export feature was built against HubSpot's v2 Contacts API.
HubSpot deprecated v2 in Q3 last year and removed it last month. All
export attempts now receive a 410 Gone response and the feature is
completely broken for all users on the Professional and Enterprise plans.

Users are reporting the issue through the in-app feedback form at a rate
of approximately 20 reports per day.

## Requirements

1. Update the HubSpot API client in the integrations service to send
   requests to the v3 `/crm/v3/objects/contacts` endpoint.
2. Update the response parser to handle the v3 response envelope (the
   `results` array and `paging` object replace the v2 `contacts` array
   and `hasMore` / `vidOffset` fields).
3. Update the request serialiser to use the v3 property format
   (`properties` object with dot-notation keys replaces the v2
   `properties` array of `{property, value}` pairs).

## Acceptance Criteria

- Exporting a contact to CRM from the Professional or Enterprise plan
  successfully creates or updates the contact in HubSpot.
- The export handles pagination correctly — contacts lists with more than
  100 entries are fully exported.
- Existing unit tests for the HubSpot client are updated to reflect the
  v3 API shape.

## Dependencies

_None identified._

## Assumptions

- HubSpot's v3 API credentials (API key / OAuth token) are already
  configured in the secrets manager; no credential rotation is needed.
- The v3 property mapping is a pure format change; no HubSpot workflow or
  property configuration changes are required.
