---
type: story
status: ready
priority: high
---

## Summary
Add two-factor authentication to the admin login flow

## Context
A recent security audit flagged that admin accounts are protected only by username and password. The audit recommendation requires that all admin accounts use a second factor. We will implement TOTP-based 2FA using an authenticator app.

## Requirements
1. Admin users must be prompted to enrol a TOTP authenticator app on their next login after the feature ships
2. After enrolment, login requires both the password and a valid 6-digit TOTP code
3. A recovery flow must exist for admins who lose access to their authenticator app
4. Failed authentication attempts must be logged

## Acceptance Criteria
- Two-factor authentication works correctly end-to-end
- The enrolment flow is secure and user-friendly
- The recovery process handles all cases properly
- No existing authentication tests are broken
- All edge cases around TOTP validation are handled correctly
- The implementation meets the security requirements

## Dependencies
- `speakeasy` TOTP library (to be added)
- Admin user table (existing — needs schema migration for TOTP secret storage)

## Assumptions
- Admin users have access to an authenticator app (Google Authenticator, Authy, etc.)
