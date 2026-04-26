---
title: "Add notification preferences to user settings"
type: story
status: ready
priority: medium
---

# Add Notification Preferences to User Settings

## Summary

Users currently receive all notification types with no ability to opt out. This
ticket adds a notification preferences screen to the user settings, allowing
users to enable or disable each notification category independently.

## Context

The support team has logged 47 complaints in Q1 about unwanted email
notifications. User research conducted in February confirmed that 68% of users
who receive more than three notification emails per week disable all
notifications via the email unsubscribe link, losing all email contact with
those users. Giving users granular control is expected to reduce full
unsubscribes while keeping engaged users informed.

## Requirements

1. The notification preferences screen must be accessible from the user
   settings page.
2. Each notification category (Marketing, Product Updates, Account Alerts,
   Weekly Digest) must have an independent on/off toggle.
3. The preference state must be persisted per user and survive logout/login
   cycles.
4. Changes to preferences must take effect within 5 minutes of being saved.
5. The notification service must check the user's stored preferences before
   dispatching any notification of any category.

## Acceptance Criteria

- The notification preferences screen is accessible from the settings page.

## Technical Notes

The notification service reads user preferences from the `user_preferences`
table. The existing `preferences` column stores a JSON blob; the new
notification preferences will be added as a `notifications` key within that
blob. The four notification categories are defined in the
`NotificationCategory` enum in `src/constants/notifications.ts`.
