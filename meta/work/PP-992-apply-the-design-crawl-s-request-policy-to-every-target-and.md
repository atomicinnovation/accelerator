---
type: "work-item"
id: "PP-992"
external_id: "PP-992"
aliases: ["draft-tdjepd"]
title: "Apply the Design Crawl's Request Policy to Every Target and Initiator"
date: "2026-11-22T12:15:57+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
relates_to: ["work-item:PP-957", "work-item:PP-736", "work-item:PP-739"]
tags: ["design", "playwright", "security"]
last_updated: "2026-11-22T12:15:57+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# PP-992: Apply the Design Crawl's Request Policy to Every Target and Initiator

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As someone running the design skills against an authenticated application, I
want every request the crawl's browser makes to pass the navigation policy and
to carry the bearer only when the application itself asks, so that neither a
popup nor an embedded third-party frame can reach a refused host or borrow my
credential.

The Playwright executor judges requests through CDP's `Fetch` domain on the
first page's own target only, and it attaches the bearer by request URL alone.
That leaves two gaps, both found while validating PP-957 (PR #253).

## Context

PP-957 repaired the design-automation runtime lane and replaced Playwright's
`page.route` with per-hop `Fetch` interception (`lib/request-interception.js`),
because `page.route` judged only a redirect chain's first hop and carried the
bearer onto later hops. That fixed redirects. It did not change which targets
are intercepted or which initiators may carry the bearer:

- **Unintercepted targets.** `interceptRequests` opens one CDP session on the
  first page and enables `Fetch` there. Popups (`window.open`, `target=_blank`),
  shared workers and websocket upgrades run on other targets or outside
  `Fetch.requestPaused`, so the navigation classifier never sees them. The
  bearer fails closed there: those requests get no auth header. The navigation
  policy fails open.
- **Targets kept on the page by launch options.** PR #253's review found that
  a single-target session misses an out-of-process iframe's later navigations,
  which `page.route` had caught. The shipped chrome-headless-shell does not
  site-isolate, so it had not regressed, but desktop Chrome through
  `design.browser_path` does. The daemon now launches with
  `--disable-site-isolation-trials`, which keeps cross-site frames on the page
  target, and blocks service workers with `serviceWorkers: 'block'`, since
  neither approach intercepted them. Dedicated workers' requests already pass
  through the page target.
- **Initiator-blind bearer.** `shouldAttachHeader(request.url, expectedOrigin)`
  attaches the bearer to any request whose URL is on the location origin,
  whoever initiated it. A cross-origin iframe on the crawled page that
  `fetch`es the location origin gets the bearer added. `page.route` behaved the
  same, so this predates PP-957.

## Requirements

1. Every request the crawl's browser makes, on any target it creates, passes
   the navigation classifier before it leaves the browser. The targets are:
   - the first page;
   - popups and new pages;
   - out-of-process iframes, if site isolation is ever turned back on;
   - shared workers.
2. A target the executor cannot intercept is refused rather than allowed to
   run unjudged. One example is a target type that `Target.setAutoAttach` does
   not cover.
3. The bearer is attached only to requests whose initiator is one the security
   decision allows (see Open Questions), as well as having a URL on the
   location origin.
4. Websocket upgrades to a refused host are refused. If the location origin
   should authenticate them, that is decided explicitly rather than inherited.
5. A refusal on any target other than the first page's main frame never
   replaces the main frame's result, as today for sub-frames.

## Acceptance Criteria

- [ ] Given a crawled page that opens a popup to a refused host, when the crawl
      runs, then the popup's navigation is blocked and recorded as refused.
- [ ] Given a crawled page with an out-of-process iframe whose document
      navigates to a refused host, when the crawl runs, then that navigation is
      blocked.
- [ ] Given a crawled page that registers a service worker, when the crawl
      runs, then the worker is not registered, as today.
- [ ] Given a cross-origin iframe on the crawled page that `fetch`es the
      location origin, when the request is sent, then it carries no bearer.
- [ ] Given the crawled page's own script fetching the location origin, when
      the request is sent, then it carries the bearer, as today.
- [ ] Given a target type the executor cannot attach to, when the page creates
      one, then the crawl refuses it and reports why, rather than letting it
      run.
- [ ] The design-automation runtime lane covers each case above against
      loopback HTTP fixtures, and its case floor rises to match.

## Open Questions

- Which initiators may carry the bearer? There are three candidates, from
  tightest to loosest:
  - the main frame's document only;
  - any frame whose own origin is the location origin;
  - any same-site frame.

  This is a security decision. It may warrant an ADR.
- Should popups and new pages be crawled under the same allowances, or refused
  outright as outside the crawl's scope?
- Should site isolation come back, with `Fetch` enabled on every attached
  target, or should the crawl keep running without it? Running without it gives
  up isolation's defence against cross-site memory leaks in a short-lived
  browser that crawls one app.

## Dependencies

- Blocked by: none
- Blocks: none

## Assumptions

- Treating websockets as in scope. `Fetch` does not pause frames after the
  upgrade, so only the upgrade request can be judged; if that is not enough,
  the scope changes.
- Treating "fails closed" for the bearer on unintercepted targets as correct
  behaviour to keep, not a defect.

## Technical Notes

- `Target.setAutoAttach` with `flatten: true` and `waitForDebuggerOnStart: true`
  lets the executor enable `Fetch` on each new target before it runs, which is
  how to meet Requirement 2 without a race.
- `Fetch.requestPaused` gives `frameId` but not the initiator. Getting the
  initiator means either correlating with `Network.requestWillBeSent`
  (`initiator`, `documentURL`) or mapping each `frameId` to its frame's origin
  through `Page.frameNavigated`.
- Relevant code: `skills/design/inventory-design/scripts/playwright/lib/request-interception.js`,
  `lib/auth-header.js` (`shouldAttachHeader`), `lib/daemon.js` (`ensureBrowser`).

## Drafting Notes

- Raised from the PP-957 validation's "Potential Issues" at the user's request.
  The two gaps are combined into one item because both are about which
  requests the interceptor's policy reaches. They could be split if the bearer
  decision should ship first.
- Priority medium rather than high: both gaps need a hostile page or embed
  inside the application the user chose to crawl, and the bearer gap predates
  PP-957. Raise it if crawls run against applications that embed untrusted
  third-party frames.
- Kind story rather than bug, because Requirement 3 needs a decision about
  intended behaviour, not only a fix.

## References

- Source: `meta/validations/2026-11-18-PP-957-build-the-vcs-test-fixtures-once-per-test-run-validation.md`
- Related: PP-957, PP-736, PP-739
- PR: #253
