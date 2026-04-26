---
title: "Add feature flag support for the payment refund flow"
type: story
status: draft
priority: high
---

# Add Feature Flag Support for the Payment Refund Flow

## Summary

The payment refund flow needs to be gated behind a feature flag so that the
refund UI and API endpoint can be deployed independently of the downstream
refund processor's go-live date. The flag will allow the platform team to
enable refund functionality per-region as each regional processor comes online.

## Context

The payment processor vendor is rolling out refund support region by region over
Q2. Engineering needs to ship the refund code to production ahead of the
vendor's schedule so it can be tested in staging, but the feature must not be
visible to end users until the processor signals readiness. The flag service
(LaunchDarkly) is already integrated and used for three other features; this
work item follows the same integration pattern.

## Requirements

1. A new boolean flag `payment_refund_enabled` must be created in LaunchDarkly
   with a default value of `false` and targeting rules off.
2. The refund endpoint (`POST /api/v2/refunds`) must check the flag on each
   request; if the flag evaluates to `false` for the requesting user's context,
   the endpoint returns HTTP 404 (not 403, to avoid leaking that the feature
   exists).
3. The refund button in the transaction detail UI must be hidden when the flag
   is `false`; it should not be greyed out or rendered in a disabled state.
4. Flag evaluation must be synchronous within the request lifecycle. The flag
   value must not be cached longer than the LDCM-recommended TTL.
5. When a user's request is blocked by the flag, the event should be logged for
   observability.

## Acceptance Criteria

- With `payment_refund_enabled` set to `false` for all users: `POST
  /api/v2/refunds` returns HTTP 404 for an authenticated user with a valid
  refund payload.
- With `payment_refund_enabled` set to `true` for all users: `POST
  /api/v2/refunds` returns HTTP 200 for an authenticated user with a valid
  refund payload.
- With `payment_refund_enabled` set to `false`: the refund button is absent
  from the DOM in the transaction detail view (not hidden via CSS, not
  disabled).
- With `payment_refund_enabled` set to `true`: the refund button is present and
  interactive.
- When `payment_refund_enabled` evaluates to `false` for a request, the
  observability system receives an event within 2 seconds.
- Flag evaluation adds no more than 10ms p99 latency to the refund endpoint
  under a load of 50 rps.

## Dependencies

- LaunchDarkly integration (already live; owned by: platform team)
- Refund endpoint implementation (in-progress, tracked in work item 0041)
- Refund UI component (in-progress, same work item)

## Assumptions

- The platform team will create the `payment_refund_enabled` flag in LaunchDarkly
  as part of this work item; engineering does not need a separate provisioning step.
- The LDCM-recommended TTL for flag evaluation caching is 5 seconds; if this
  changes, the implementation must be updated separately.
- Returning HTTP 404 for a flag-blocked refund request is an agreed UX decision
  confirmed with the product team.

## Technical Notes

Use the existing `FeatureFlagClient` wrapper at `src/lib/feature-flags.ts`. The
flag key should match the snake_case convention used by existing flags
(`payment_refund_enabled`). See `src/api/routes/beta-feature.ts` for a working
example of the flag-gate pattern.
