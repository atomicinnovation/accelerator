---
type: spike
status: ready
priority: medium
---

## Summary
Evaluate message queue options for the order processing pipeline

## Context
Order processing is currently synchronous — the checkout API waits for inventory checks, payment capture, and fulfilment handoff to complete before returning a response. Peak traffic is causing p95 response times above 8 seconds. The team wants to decouple order processing via async messaging, but hasn't selected a technology yet.

## Requirements
1. Evaluate at least three message queue or event streaming solutions
2. Consider operational complexity, cost, and existing team familiarity
3. Assess fit with the current AWS-hosted infrastructure

## Acceptance Criteria
- The team has a good understanding of the available options and their trade-offs
- There is a clear recommendation for which technology to pursue
- The risks of each option have been considered
- The spike output provides enough context to begin the implementation story

## Open Questions
- Is any message queue already used elsewhere in the organisation?
- What SLA do we need for order processing latency post-decoupling?
