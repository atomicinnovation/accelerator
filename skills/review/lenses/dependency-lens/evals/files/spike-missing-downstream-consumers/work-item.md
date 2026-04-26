---
title: "Choose a message queue vendor for async job processing"
type: spike
status: ready
priority: high
---

# Choose a Message Queue Vendor for Async Job Processing

## Summary

Evaluate three message queue vendors — AWS SQS, RabbitMQ, and Redis
Streams — for use as the async job processing backbone, and recommend
one with a documented rationale.

## Context

Three upcoming features require reliable async job processing: background
report generation (feature ticket #1201), user data export (feature ticket
#1202), and scheduled email delivery (feature ticket #1203). All three are
currently blocked on a vendor decision — they cannot be designed or
estimated until the queue infrastructure is chosen.

The platform runs on AWS. The team has experience with Redis but no
production experience with RabbitMQ or SQS. A managed service is preferred
to minimise operational overhead.

## Requirements

1. Evaluate AWS SQS, RabbitMQ (self-hosted on ECS), and Redis Streams
   against the following criteria: at-least-once delivery guarantees,
   dead-letter queue support, visibility timeout / ack semantics,
   observability (metrics, tracing), and estimated operational cost at
   10k jobs/day.
2. Prototype a minimal producer/consumer pair using the top candidate to
   validate integration ergonomics.

## Acceptance Criteria

- A decision memo committed to `meta/research/` names the recommended
  vendor, summarises each candidate against the evaluation criteria, and
  states the rationale.
- A comparison matrix (Markdown table) lists all three vendors against
  each criterion with a pass/fail/partial rating.
- The prototype code is committed to a sandbox branch.

## Time-box

4 working days.

## Dependencies

_None identified._

## Assumptions

- The team has AWS credentials with SQS permissions available in the
  sandbox account.
- Redis is already running in the sandbox environment for the prototype.
