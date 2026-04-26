---
title: "Fix password validation error message"
type: bug
status: ready
---

# Fix Password Validation Error Message

## Summary

When a user submits a password that fails validation, the error message shown
is generic and unhelpful. This ticket fixes the wording of the error message
to tell the user which specific validation rule was violated.

## Context

The current implementation shows "Invalid password" for all password validation
failures regardless of which rule was violated. Users cannot tell whether their
password is too short, lacks a special character, or contains disallowed
characters.

## Requirements

1. Parse the validation error code returned by the authentication service.
2. Map each error code to a specific, user-readable message (e.g., "Password
   must be at least 8 characters").
3. Refactor the authentication middleware to centralise all validation logic
   into a single module.
4. Add retry logic to the authentication service so transient network failures
   do not surface to the user as validation errors.
5. Deprecate the existing `auth_validate()` function and replace it with the
   new centralised module's API.
6. Update all callers of `auth_validate()` across the codebase to use the
   new API.
7. Display the specific message in the password field's error tooltip.

## Acceptance Criteria

- The error tooltip shows a specific message matching the violated rule.
- The authentication middleware has been refactored into a centralised module.
- All callers of `auth_validate()` have been migrated to the new API.

## Technical Notes

The validation error codes are documented in the authentication service API
reference. There are currently 6 distinct codes covering length, character
class, and disallowed patterns.
