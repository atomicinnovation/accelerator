---
type: "pr-description"
id: "99"
title: "[0206] Classify navigation URLs, not only the initial location"
date: "2026-09-05T11:09:59+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0206"
parent: "work-item:0206"
relates_to: ["work-item:0209"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/99"
pr_number: 99
tags: ["design", "security", "ssrf", "playwright", "executor", "access-policy"]
revision: "e78f61612c8e4ecb184b52901dbca685e5e2a294"
repository: "accelerator"
last_updated: "2026-09-05T11:09:59+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0206] Classify navigation URLs, not only the initial location

## Summary

Carries the `AccessPolicy` verdict — reachability and scheme together — from the
`validate-source` front door down to per-request `navigate` and `links`
classification in the design crawl. Previously the verdict was enforced once, at
the location a crawl started from; every navigation, redirect hop and followed
link after that was unclassified, so an attacker-influenced page or a redirect
could steer a crawl at an internal endpoint the front door would refuse.

The classifier that must now run per request lives in the Node Playwright
daemon, where the Rust domain does not exist, so it is reimplemented in
JavaScript. To stop the two implementations drifting, both are tested against a
single language-neutral vector corpus — either side diverging fails CI.

## Changes

- **Shared classification corpus** — one JSON fixture
  (`cli/design/tests/fixtures/host-classification-vectors.json`) of reach,
  canonicalisation-error and access-policy vectors. The Rust classifier is bound
  to it by a new integration test; the inline vector loops in `host_reach`,
  `host` and `access_policy` are thinned to the pinned prose tests the corpus
  cannot express.
- **JavaScript classifier** — `lib/access-policy.js`, a faithful port of host
  canonicalisation, reachability and scheme classification down to the IPv6
  transition unwraps (6to4, Teredo bitwise inversion, NAT64, IPv4-mapped /
  -compatible), driven test-first against the same corpus.
- **Allowances plumbed through the executor** — `--allow-internal` /
  `--allow-insecure-scheme` flags on the `executor` command, injected into every
  forwarded request body by a merge that refuses a payload pre-setting either
  key. Both browser agents forward the flags they were spawned with on every
  call; the skill passes the operator's `validate-source` allowances into the
  agent prompts.
- **`navigate` enforcement** — one page-lifetime `page.route` handler classifies
  each main-frame navigation and redirect hop under a per-request,
  deny-all-default `currentAllowances`, aborts a refused one before its fetch,
  and surfaces a non-retryable `navigation-refused` envelope (host and pathname
  only) whose `details.classification` names the reach or scheme class. It fails
  closed on a thrown classifier error and `fallback()`s on the allow path so a
  later handler can still act.
- **`links` enforcement** — a policy-refused destination folds into the existing
  `same_origin: false` skip. The browser-resolved host is classified in Node and
  stripped before the response, so no new host authority leaves the browser.
- **Docs and protocol** — `PROTOCOL.md`, the design page, both agent docs, the
  module docs and `CHANGELOG.md` reconciled to per-request coverage, naming
  subresource SSRF and DNS rebinding as open residuals rather than implying
  end-to-end SSRF closure.

## Context

- Work item: `meta/work/0206-classify-navigation-urls-not-only-the-initial-location.md`
- Plan: `meta/plans/2026-08-31-0206-classify-navigation-urls.md`
- Research: `meta/research/codebase/2026-08-31-0206-classify-navigation-urls.md`
- Coordinating story: 0209 (cross-origin auth-header stripping) extends the same
  route seam this PR installs.

## Testing

- [x] `mise run cli:check` — rustfmt + clippy clean across the `cli/` workspace
- [x] `cargo test -p design -p accelerator-design` — green
- [x] `mise run test:unit:design-automation` — 85 cases green; the Rust and JS
      classifiers agree on the shared corpus (verified a one-character edit to
      the JS Teredo unwrap turns a corpus case red)
- [x] `mise run docs:check` — builds; internal links valid
- [x] Full `mise run` — green **except** one pre-existing, unrelated
      release-signing test (`test_signs_and_emits_manifest_under_secret_context`,
      needs `/tmp/key.sec`); no file in this PR touches the signing/release path
- [ ] `daemon-runtime.test.js` browser-level cases (redirect hop, per-request
      scope, sub-frame isolation, out-of-window redirect, click-triggered
      navigation) — written to the existing harness but the opt-in
      `test:integration:design-automation` lane is unrunnable locally: a
      pre-existing `runtime-preflight.js` breakage (it still reads a removed
      `package-lock.json`). The pure decisions are unit-tested against the corpus

## Notes for Reviewers

- **Anti-drift corpus is the spine.** The security-critical classifier now
  exists twice, in Rust and JS. Review the corpus coverage and the JS port's
  fidelity to the Rust domain, especially the IPv6 transition unwraps.
- **Deviations from the plan**, each verified: the corpus loader lives in an
  integration-test crate (the `design` crate's cargo-pup rule forbids a
  `serde_json` import in-module); the clap flag-after-command contract was
  corrected (trailing collection only begins after the first argument, so the
  flags must precede `command`, which is how the agents emit them — a page
  influences only the body, never argv); agents forward allowances on every
  call, not only `navigate`, because a `click`/`type` can itself navigate.
- **Residuals are documented, not closed.** Classification stays pre-resolution
  (DNS rebinding out of scope) and does not cover page subresources — stated
  plainly in the docs so per-request coverage is not read as end-to-end SSRF
  safety.
- **Branch noise.** The `Record Linear sync: external_ids for 54 newly-created
  issues` commit predates this work and touches 56 `meta/work/*.md` files with a
  single `external_id` line each. It is unrelated to 0206; consider whether it
  belongs in this PR before merge.
- **Follow-up.** The runtime integration lane needs its `runtime-preflight`
  package-lock breakage fixed and a provisioned Playwright runtime before the
  browser-level cases can gate on merge.
