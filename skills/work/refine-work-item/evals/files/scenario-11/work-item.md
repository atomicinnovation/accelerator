---
work_item_id: "0060"
title: "Add Email Notifications for Order Status"
date: "2026-04-01T10:00:00+00:00"
author: "Test Author"
type: story
status: draft
priority: medium
parent: ""
tags: [notifications, orders]
---

# 0060: Add Email Notifications for Order Status

**Type**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Test Author

## Summary

As a customer, I want to receive email notifications when my order status changes so that I stay informed without checking the app.

## Context

Currently customers must manually check the app for order status updates.

## Requirements

1. Send an email when an order transitions to: confirmed, shipped, delivered, or cancelled.
2. Email template must include order number, status, and a link to the order detail page.
3. Emails must be sent asynchronously via the existing job queue.

## Acceptance Criteria

- [ ] Given an order status transition to "shipped", the customer receives an email within 60 seconds containing the order number and a valid tracking link.
- [ ] Given a failed email delivery, the job retries up to 3 times with exponential backoff before marking the notification as failed.

## Open Questions

## Dependencies

- Blocked by:
- Blocks:

## Assumptions

- The job queue supports delayed retry with configurable backoff.

## Technical Notes

## Drafting Notes

## References
