---
title: "Implement order placement flow end-to-end"
type: story
status: ready
priority: high
---

# Implement Order Placement Flow End-to-End

## Summary

When an order is placed, implement the complete end-to-end flow: the inventory
service reserves stock, the billing service charges the payment card, and the
notification service sends the customer a confirmation email.

## Context

Order placement currently stops after persisting the order to the database.
The downstream services (inventory, billing, notifications) need to be
triggered as part of a single order placement.

## Requirements

1. When an order is placed, the inventory service must reserve the ordered
   quantity for each line item. If stock is insufficient for any item, the
   order must be rejected with an appropriate error.
2. After inventory reservation succeeds, the billing service must charge the
   payment method on file. If charging fails, inventory reservations must be
   rolled back.
3. After billing succeeds, the notification service must send a confirmation
   email to the customer with the order summary and estimated delivery date.
4. The entire flow must complete within 5 seconds under normal conditions.
5. Failures in the notification step should not roll back billing or inventory
   — log the failure and retry asynchronously.

## Acceptance Criteria

- Placing an order with sufficient stock and valid payment method results in:
  a successful reservation, a charge on the customer's card, and a
  confirmation email delivered within 30 seconds.
- Placing an order with insufficient stock results in a rejection with no
  charge and no email.
- A billing failure after inventory reservation results in inventory being
  released, no email, and an error returned to the caller.

## Dependencies

- Inventory service API (owned by: inventory team)
- Billing service API (owned by: payments team)
- Notification service API (owned by: platform team)
