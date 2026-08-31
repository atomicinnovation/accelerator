---
type: "work-item"
id: "0206"
title: "Classify Navigation URLs, Not Only The Initial Location"
date: "2026-08-12T23:21:12+00:00"
author: "Toby Clemson"
producer: "implement-plan"
status: "ready"
kind: "story"
priority: "high"
parent: "work-item:0196"
derived_from: ["plan:2026-08-11-0196-design-cli-migration"]
relates_to: ["work-item:0209"]
tags: ["design", "security", "playwright", "ssrf"]
last_updated: "2026-08-31T13:52:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-736"
---

# 0206: Classify Navigation URLs, Not Only The Initial Location

## Summary

As an operator running `accelerator design` crawls against untrusted or
attacker-influenced pages, I want every navigation and every followed link
classified by the same reachability policy that guards the initial location,
so that a malicious page or a redirect cannot steer a crawl at internal
endpoints the front door would refuse.

The hardening on `validate-source` covers only where a crawl starts.
Per-request navigation and link-following remain unclassified, so the guarantee
the front door makes does not hold once the crawl is moving.

## Context

`accelerator design validate-source` hardens the location a crawl starts from.
It parses the host as an address, classifies its reachability, and refuses
private, link-local and reserved destinations unless `--allow-internal` is
passed — closing decimal, hexadecimal and octal encodings, IPv6 transition
forms and the IPv4-mapped addresses the shell implementation let through.

That is the front door, and only the front door. The daemon's `navigate`
command takes an arbitrary `url` per request and calls `page.goto(req.url)`
with no classification at all (the `navigate` handler in the playwright
`lib/daemon.js`), and the `links` command hands the agent a crawlable set whose
`same_origin` flag drives which routes get followed. So an attacker-influenced
page, or a redirect, can steer a crawl at an internal endpoint that the front
door would have refused.

The module documentation states this limit explicitly rather than leaving it to
be inferred, so nobody reads the hardened entry point as covering the whole
navigation surface. But stating it is not fixing it.

## Requirements

Plumb the `AccessPolicy` verdict — including whatever allowances the invocation
carried — through the executor into `navigate`, and into the `same_origin`
decision `links` reports. The domain code already exists and is well tested:
`design::host_reach` and `design::access_policy` are pure functions over an
injected host, so the work is carrying the allowances from the CLI invocation
down to the daemon and applying the same verdict per request rather than once
at the start.

Resolved design decisions (see Open Questions for what was decided and why):

- **Refusal shape.** A refused `navigate` returns an error envelope
  (`retryable: false`) whose `details` carry the reach classification, and the
  refused URL is never loaded. `links` reports `same_origin: false` for a
  destination the policy would refuse — a skip by omission, needing no new
  protocol concept. Neither shape forces a crawl abort at the process level,
  since all daemon outcomes return on stdout at exit 0; the agent driving the
  crawl decides whether to stop or continue with a reported gap.
- **Allowance scope.** Allowances travel per request, injected by the Rust
  executor from new `--allow-internal` / `--allow-insecure-scheme` flags on the
  executor command. They are not fixed for the daemon's lifetime: the daemon is
  long-lived and reused across invocations, so a lifetime-fixed allowance would
  leak into a later invocation that never asked for it.

Redirect enforcement classifies at request-interception time (a Playwright
route handler that aborts before the internal fetch). `page.goto` follows
redirects internally and reports only the final URL, so a post-hoc check on the
returned URL fires after the internal request has already been made.

This is deliberately not a migration-time change. It alters the behaviour of
every crawl — a redirect that previously succeeded may now be refused — so it
wants its own change, its own note in the changelog, and its own decision about
what a mid-crawl refusal should look like to the agent driving it.

## Acceptance Criteria

- [ ] `navigate` classifies its URL with the same code path `validate-source`
      uses, and refuses what that would refuse
- [ ] A refused `navigate` returns an error envelope (`retryable: false`) whose
      `details` carry the reach classification, and the refused URL is never
      loaded
- [ ] The allowances from the original invocation reach that decision, scoped
      per request — a daemon reused by a later invocation that did not pass
      `--allow-internal` does not grant internal access
- [ ] `links` does not report `same_origin: true` for a destination the policy
      would refuse
- [ ] A test drives a redirect from a public host to a link-local address and
      asserts the internal request is never issued
- [ ] The advisory wording in `host_reach`'s module docs and the design page is
      replaced, since the check is then a boundary
- [ ] `mise run` exits 0

## Open Questions

None outstanding. The two prior open decisions are resolved:

- Refused navigation is an **error envelope** for `navigate` and a
  `same_origin: false` **skip** for `links`.
- Allowances are **per request**, not daemon-lifetime.

## Dependencies

- Blocked by: none — parent 0196 (`validate-source`, `host_reach`,
  `access_policy`) is delivered.
- Blocks: none known.
- Relates to: 0209 (cross-origin navigation — auth-header stripping on the same
  daemon).

## Assumptions

- The threat model is a malicious or attacker-influenced page or redirect
  steering the crawl, not a malicious operator — an operator can always pass
  `--allow-internal` deliberately.
- Enforcement is pre-resolution classification only. `host_reach` never
  performs DNS, so a public hostname that resolves to an internal address (DNS
  rebinding) is out of scope here — the same residual limitation
  `validate-source` already carries.

## Technical Notes

- `design::host_reach::classify` and `design::access_policy::evaluate` are pure
  Rust functions over an injected host and `Allowances`; Rust-only today with no
  JS binding, so the wiring is greenfield on both sides of the Rust↔JS boundary.
- The daemon exposes two config channels: env vars (set once at spawn,
  immutable — daemon-lifetime) and per-request JSON args (forwarded verbatim
  into the request). The per-request channel is the one to use.
- `Command::Executor` has no allowance flags today; only `validate-source` does.
  The flags must be added to the executor command and injected into each
  forwarded request.
- Redirect enforcement needs a route handler, not a post-`goto` URL check, for
  the reason given in Requirements.
- The advisory "front door, not a boundary" wording lives in the `host_reach`
  module doc-comment and `access_policy`, plus the design page docs.

## Drafting Notes

- Resolved both open decisions during this refinement. Error-envelope-plus-skip
  was chosen because every daemon outcome returns on stdout at exit 0, so an
  error envelope does not itself abort the crawl. Per-request scope was chosen
  because the daemon is reused across invocations and a lifetime-fixed allowance
  would leak through the reuse path.
- Read the summary "role" as the operator running design crawls, or the agent
  driving them, against untrusted pages.
- Set `relates_to` to 0209 and dropped 0196 from it (0196 is already `parent`).
- Flagged DNS rebinding as an explicit out-of-scope residual, matching
  `host_reach`'s pre-resolution design.

## References

- Parent: 0196 — `validate-source`, `host_reach`, `access_policy`
  (initial-location hardening)
- Related: 0209 — cross-origin navigation, auth-header stripping
- Derived from: plan `meta/plans/2026-08-11-0196-design-cli-migration.md`
- Key code: `skills/design/inventory-design/scripts/playwright/lib/daemon.js`,
  `cli/design/src/host_reach.rs`, `cli/design/src/access_policy.rs`,
  `skills/design/inventory-design/PROTOCOL.md`
