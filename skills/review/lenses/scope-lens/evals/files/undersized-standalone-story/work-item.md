---
title: "Rename user_email column to user_email_address"
type: story
status: ready
priority: low
---

# Rename user_email Column to user_email_address

## Summary

Rename the `user_email` database column to `user_email_address` in the users
table for naming consistency.

## Context

The style guide requires all email fields to use the `_address` suffix. This
column was missed during the initial schema review.

## Requirements

1. Rename the `user_email` column to `user_email_address` in the `users` table.

## Acceptance Criteria

- The `users` table has a column named `user_email_address` instead of
  `user_email`.
- All application code and tests that reference `user_email` are updated.
