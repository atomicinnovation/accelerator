---
type: "work-item"
id: "0209"
title: "Wire Up Or Retire The Header-Auth Path"
date: "2026-08-12T23:21:12+00:00"
author: "Toby Clemson"
producer: "implement-plan"
status: "ready"
kind: "bug"
priority: "high"
parent: "work-item:0196"
derived_from: ["plan:2026-08-11-0196-design-cli-migration"]
relates_to: ["work-item:0196"]
tags: ["design", "security", "playwright", "auth"]
last_updated: "2026-08-12T23:21:12+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-739"
---

# Wire Up Or Retire The Header-Auth Path

## Context

`inventory-design` documents an auth-header origin allowlist as
security-critical: a bearer token placed in `ACCELERATOR_BROWSER_AUTH_HEADER`
is supposed to be injected only on navigations whose origin matches the
resolved location or the login URL, and stripped on any cross-origin
navigation.

None of that happens. `makeAuthHeaderHandler` is imported at `daemon.js:11` and
never called, and its second required input,
`ACCELERATOR_BROWSER_LOCATION_ORIGIN`, is set nowhere in the repository. The
header path is doubly dead.

The consequence has two halves, and both are bad. Users are told to place real
bearer tokens into the environment of a browser-driving daemon for a feature
that never applies them. And an authenticated crawl silently produces an
*unauthenticated* inventory — the pages behind the login are simply missing,
with nothing saying so.

The CLI migration deliberately preserved this rather than changing behaviour
inside a port, but corrected the documentation: both SKILL.md files and
`design resolve-auth`'s help text now say the path is inert and that a live
credential should not be placed there yet.

## We need to

Decide between wiring it up and retiring it, and carry the decision through
every surface that mentions it.

**Wiring it up** means calling the handler on navigation, sourcing the origin
allowlist from the resolved `[location]` rather than an unset variable, and
proving the cross-origin strip with a test that navigates off-site and asserts
the header is absent. The allowlist is the whole security property, so it needs
a test that fails when the strip is removed, not merely one that shows the
header arriving on the happy path.

**Retiring it** means removing `auth-header.js`, the `header` mode from
`design resolve-auth`, the `ACCELERATOR_BROWSER_AUTH_HEADER` variable, its
scrub rule in `leaked_credentials`, and the documentation that describes it —
leaving form login as the only authenticated crawl.

## Acceptance criteria

- [ ] The decision is recorded, with its reasoning, before implementation
- [ ] If wired up: a test navigates cross-origin and asserts the header is not
      sent, and fails when the strip is removed
- [ ] If retired: no surface still names `ACCELERATOR_BROWSER_AUTH_HEADER`, and
      `design resolve-auth` no longer has a `header` mode
- [ ] The warning callout in `inventory-design/SKILL.md` is removed or replaced
      to match the outcome
- [ ] `mise run` exits 0
