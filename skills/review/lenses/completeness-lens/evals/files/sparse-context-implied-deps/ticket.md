---
title: "Integrate Stripe for payment processing"
type: story
status: ready
priority: high
---

# Integrate Stripe for Payment Processing

## Summary

Add Stripe as the payment processing provider to enable credit card payments in
the checkout flow.

## Context

The team needs to add Stripe to enable credit card payments at checkout.

## Requirements

1. The checkout flow must accept credit card payments via the Stripe API using
   Stripe Elements for card input.
2. The Stripe webhook endpoint must validate each incoming event's signature
   using the webhook signing secret before processing it.
3. Successful payment events received via webhook must trigger an order
   confirmation event to the order management system.
4. Failed payment events must be retried by Stripe's built-in retry mechanism;
   after all retries are exhausted, the order must be marked as payment-failed
   in the order management system.
5. Payment card data must not be stored on our servers; all sensitive data must
   be tokenised and handled exclusively by Stripe.

## Acceptance Criteria

- A user can complete a purchase using a Visa, Mastercard, or American Express
  card on the checkout page.
- Webhook events with invalid signatures are rejected with HTTP 400 and logged.
- A successful payment transitions the order to the confirmed state in the order
  management system within 30 seconds of the charge being created.
- No card numbers, CVVs, or expiry dates appear in application logs or the
  database.

## Dependencies

## Technical Notes

Use Stripe's official Node.js SDK (`stripe` npm package). Test API keys are
available in the shared secrets manager under the `stripe/test` path. The
webhook signing secret is stored separately under `stripe/webhook-secret`.
