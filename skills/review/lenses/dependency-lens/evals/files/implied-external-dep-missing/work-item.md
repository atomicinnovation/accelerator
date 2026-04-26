---
title: "Handle charge.refunded webhook from Stripe"
type: story
status: ready
priority: high
---

# Handle charge.refunded Webhook from Stripe

## Summary

Add support for the `charge.refunded` webhook event so the platform
records refunds automatically when Stripe issues them, removing the
need for manual reconciliation.

## Context

Finance has reported that refunds processed in Stripe are not reflected
in the platform's transaction ledger. Currently, support staff must
manually enter refund records after the customer contacts us. Automating
this will eliminate the reconciliation lag and reduce support overhead.

## Requirements

1. Register a new webhook handler for the `charge.refunded` event type
   in the payments service.
2. Parse the incoming `charge.refunded` payload and extract the refund
   amount, currency, and the original `charge_id`.
3. Write a new ledger entry of type `refund` linked to the original
   transaction record identified by `charge_id`.
4. Return HTTP 200 to Stripe to acknowledge receipt; return HTTP 400 for
   malformed payloads.

## Acceptance Criteria

- A `charge.refunded` event delivered by Stripe results in a `refund`
  ledger entry appearing in the platform within 5 seconds of delivery.
- A `charge.refunded` event with a `charge_id` that does not match any
  known transaction is logged as an anomaly and acknowledged (200) rather
  than rejected (400).
- Replaying a previously processed `charge.refunded` event (same event
  ID) is idempotent — no duplicate ledger entries are created.

## Dependencies

_None identified._

## Assumptions

- The payments service already has a webhook ingestion endpoint; this
  story adds a new event handler to that endpoint.
- The ledger schema already supports a `refund` entry type.
