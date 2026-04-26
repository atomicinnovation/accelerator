---
title: "Add audit logging for administrative actions"
type: story
status: ready
priority: medium
---

# Add Audit Logging for Administrative Actions

## Summary

Administrative actions (user creation, role changes, data exports) currently
leave no audit trail. This ticket adds structured audit logging so that all
in-scope administrative actions are recorded with the acting user, the action
type, the affected resource, and the timestamp.

## Context

The security team's Q1 audit identified the absence of an audit trail as a
compliance gap. The platform is pursuing SOC 2 Type II certification, and the
auditors require a demonstrable record of who performed what administrative
actions and when. The engineering team has agreed to use the existing structured
logging infrastructure (Datadog) rather than a separate audit database, as the
volume of administrative events is low enough to make search and retention
manageable within Datadog's current plan.

## Requirements

1. The API layer must emit a structured log entry for every in-scope
   administrative action, containing: the acting user's ID, the action name,
   the affected resource type and ID, and the UTC timestamp.
2. The following actions are in scope: creating or deactivating a user account,
   changing a user's role, revoking an API key, and exporting data.
3. Audit log entries must be written synchronously with the action: a failed
   action must never produce an audit entry, and a successful action must always
   produce one.
4. Audit log entries must be readable by the security team via the existing
   Datadog dashboard without requiring engineering involvement.

## Acceptance Criteria

- Creating a user account produces a Datadog log entry containing acting_user_id,
  action: "user.create", resource_id, and timestamp within 1 second of the action.
- Deactivating a user account produces a log entry with action: "user.deactivate",
  resource_id, and timestamp.
- Changing a user's role produces a log entry including the previous role and
  the new role values.
- Revoking an API key produces a log entry containing the key ID (not the key
  value) and the acting user ID.
- Exporting data produces a log entry including the export type and the number
  of records exported.
- If an administrative action fails before completing, no audit log entry is
  written.

## Dependencies

- Datadog logging infrastructure (already configured, owned by: platform team)
- Existing structured logger at `src/lib/logger.ts`

## Assumptions

- Datadog's current retention policy (90 days) is sufficient for compliance
  purposes; the security team has confirmed this in writing.
- Administrative actions are defined as operations performed by users with the
  `admin` or `superadmin` role; actions by other roles are out of scope.

## Technical Notes

Wrap each in-scope database transaction with an audit log emission step. Inject
an `AuditLogger` interface into the services that handle in-scope actions so
that the logging can be tested independently of the transport layer.
