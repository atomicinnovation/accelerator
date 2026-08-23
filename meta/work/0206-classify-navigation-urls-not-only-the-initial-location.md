---
type: work-item
id: "0206"
title: "Classify Navigation URLs, Not Only The Initial Location"
date: "2026-08-12T23:21:12+00:00"
author: Toby Clemson
producer: implement-plan
status: ready
kind: story
priority: high
parent: "work-item:0196"
derived_from: ["plan:2026-08-11-0196-design-cli-migration"]
relates_to: ["work-item:0196"]
tags: [design, security, playwright, ssrf]
last_updated: "2026-08-12T23:21:12+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-736
---

# Classify Navigation URLs, Not Only The Initial Location

## Context

`accelerator design validate-source` hardens the location a crawl starts from.
It parses the host as an address, classifies its reachability, and refuses
private, link-local and reserved destinations unless `--allow-internal` is
passed — closing decimal, hexadecimal and octal encodings, IPv6 transition
forms and the IPv4-mapped addresses the shell implementation let through.

That is the front door, and only the front door. The daemon's `navigate`
command takes an arbitrary `url` per request and calls `page.goto(req.url)`
with no classification at all (`lib/daemon.js:165-167`), and the `links`
command hands the agent a crawlable set whose `same_origin` flag drives which
routes get followed. So an attacker-influenced page, or a redirect, can steer a
crawl at an internal endpoint that the front door would have refused.

The module documentation states this limit explicitly rather than leaving it to
be inferred, so nobody reads the hardened entry point as covering the whole
navigation surface. But stating it is not fixing it.

## We need to

Plumb the `AccessPolicy` verdict — including whatever `--allow-internal` the
invocation carried — through the executor into `navigate`, and into the
`same_origin` decision `links` reports.

The domain code already exists and is well tested: `design::host_reach` and
`design::access_policy` are pure functions over an injected host, so the work
is carrying the allowances from the CLI invocation down to the daemon and
applying the same verdict per request rather than once at the start.

This is deliberately not a migration-time change. It alters the behaviour of
every crawl — a redirect that previously succeeded may now be refused — so it
wants its own change, its own note in the changelog, and its own decision about
what a mid-crawl refusal should look like to the agent driving it.

## We need to decide

- Whether a refused navigation is an error envelope or a recorded skip. An
  error stops the crawl; a skip lets it continue with a gap it can report.
- Whether the allowances travel per request or are fixed for the daemon's
  lifetime. Per request is more flexible; fixed is harder to get wrong.

## Acceptance criteria

- [ ] `navigate` classifies its URL with the same code path `validate-source`
      uses, and refuses what that would refuse
- [ ] The allowances from the original invocation reach that decision
- [ ] `links` does not report `same_origin: true` for a destination the policy
      would refuse
- [ ] A test drives a redirect from a public host to a link-local address and
      asserts the crawl does not reach it
- [ ] The advisory wording in `host_reach`'s module docs and the design page is
      replaced, since the check is then a boundary
- [ ] `mise run` exits 0
