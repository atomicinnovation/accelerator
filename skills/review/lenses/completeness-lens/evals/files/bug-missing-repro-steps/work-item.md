---
title: "Payment confirmation email not sent after successful checkout"
type: bug
status: ready
priority: high
---

# Payment Confirmation Email Not Sent After Successful Checkout

## Summary

Customers are not receiving payment confirmation emails after completing a
purchase. The payment is processed successfully and the order appears in the
admin panel, but the confirmation email is never delivered.

## Context

This was first reported by three customers on 2026-04-15. The affected
customers all made purchases between 14:00 and 16:00 UTC. Orders placed outside
this window appear to have confirmation emails sent correctly. The issue may be
related to the email service deployment at 13:45 UTC on 2026-04-15, which
upgraded the template rendering engine from version 2 to version 3.

## Requirements

1. Confirmation emails must be sent to the customer's registered email address
   within 5 minutes of a successful payment.
2. The email must include the order number, itemised receipt, and estimated
   delivery date.
3. If email delivery fails, the failure must be logged to the error tracking
   system with the order number and customer email.

## Acceptance Criteria

- After completing a checkout, the customer receives a confirmation email within
  5 minutes.
- The confirmation email contains the order number, item list, totals, and
  estimated delivery date.
- If the email service is unavailable, the order is still completed and the
  delivery failure is logged with full context.

## Technical Notes

The email sending is handled by `OrderEmailService` in
`src/services/email/OrderEmailService.ts`. The template upgrade changed the
rendering interface from `render(template, data)` to
`render({ template, data, version })`. A mismatch between the old call sites
and the new interface is the likely cause.
