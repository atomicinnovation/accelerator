---
title: "Platform improvements Q2"
type: epic
status: ready
priority: high
---

# Platform Improvements Q2

## Summary

Deliver platform improvements across multiple product areas in Q2: user profile
rework, billing migration to Stripe, mobile app push notifications, and admin
audit logs.

## Context

Leadership has identified these four areas as the key improvements needed
before the Q3 roadmap planning session.

## Requirements

1. **User profile rework**: Replace the current profile page with a new design
   that supports avatar uploads, pronouns, timezone preferences, and
   notification settings.
2. **Billing migration to Stripe**: Migrate from Braintree to Stripe. Update
   payment processing, subscription management, invoice generation, and
   webhook handling. Coordinate with finance team on reporting changes.
3. **Mobile push notifications**: Implement push notification support for iOS
   and Android using Firebase Cloud Messaging. Users should be able to opt in
   per notification category.
4. **Admin audit logs**: Record all administrative actions (user creation,
   role changes, data exports) with acting user, timestamp, and action type.
   Security team requires this for SOC 2 compliance.

## Stories

- User profile rework (UI/UX, avatar upload, settings)
- Stripe migration planning and implementation
- Stripe webhook integration and testing
- Mobile push notification infrastructure (FCM setup)
- iOS push notification client integration
- Android push notification client integration
- Admin audit log service
- Audit log dashboard for security team

## Acceptance Criteria

- All four feature areas are shipped and verified in production by end of Q2.
- No regression in existing billing or user account functionality.

## Dependencies

- Stripe account credentials from finance team
- Firebase Cloud Messaging project setup
- SOC 2 audit timeline from security team
