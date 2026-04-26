---
work_item_id: "0045"
title: "Send Welcome Emails"
date: "2026-04-01T10:00:00+00:00"
author: "Test Author"
type: story
status: draft
priority: medium
parent: ""
tags: [email, onboarding, backend]
---

# 0045: Send Welcome Emails

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Test Author

## Summary

Automatically send a welcome email to new users when they complete registration. The email should introduce the product, include a call-to-action to complete their profile, and provide a link to the getting-started guide.

## Context

New user retention is lower than expected. Post-signup interviews reveal that many users don't know where to start. A welcome email with clear next steps is a standard onboarding pattern expected to improve day-1 engagement.

## Requirements

1. Trigger a welcome email when a new user account is created (both direct signup and OAuth signup paths).
2. The email must use the new user schema migration's `onboarding_status` field to track whether the welcome email has been sent; do not send duplicate emails if the trigger fires more than once.
3. Render the email using the Handlebars template in `templates/emails/welcome.hbs` (which must be created as part of this work item).
4. Send via the email service configured in `config/email.js`. The email service must be running and configured with valid SMTP credentials before this feature can be tested end-to-end.
5. Include an unsubscribe link that writes to the `email_preferences` table. This table must exist in the database before this work item can be completed.
6. Log a `welcome_email_sent` event to the analytics service for each email dispatched.

## Acceptance Criteria

- [ ] Given a new user registers, a welcome email is sent to their registered address within 60 seconds.
- [ ] Given the welcome email trigger fires twice for the same user, only one email is sent.
- [ ] The email contains the user's first name, a link to the getting-started guide, and an unsubscribe link.
- [ ] The unsubscribe link correctly updates the user's `email_preferences` record.

## Open Questions

- Should we send welcome emails to users who signed up via OAuth and may not have verified their email address?

## Dependencies

- Blocked by:
- Blocks:

## Assumptions

- The email service is already configured and operational.
- Users always have a valid email address at signup.

## Technical Notes

## Drafting Notes

- Analytics service integration is required; if not available, this work item is blocked.

## References

- Related: 0044
