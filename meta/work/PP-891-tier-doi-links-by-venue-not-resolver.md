---
type: "work-item"
id: "PP-891"
title: "Tier DOI Links by Venue, Not by the doi.org Resolver"
date: "2026-10-09T17:58:47+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "in-progress"
kind: "bug"
priority: "medium"
parent: "work-item:PP-22"
relates_to: ["work-item:PP-864", "work-item:PP-865", "work-item:PP-892", "work-item:PP-897", "work-item:PP-936"]
tags: ["research", "deep-research", "web-profile", "openalex", "tiers", "cli", "crossref"]
last_updated: "2026-11-23T16:06:41+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-891"
aliases: ["0305"]
---

# PP-891: Tier DOI Links by Venue, Not by the doi.org Resolver

**Kind**: Bug
**Status**: In Progress
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The same DOI appeared at tier-1 and tier-3 in one dossier (the synthesised
document a `research-topic` subject ends in). The issue blamed
`fetch openalex` tiering `search` and `lookup` differently, but that does not
reproduce: the CLI tiers all three reported DOIs tier-1 by `search` and by
every form of `lookup`. The likely cause of two of the three conflicts is
web-profile researchers tiering a `https://doi.org/…` URL by its domain —
`doi.org` is a resolver, not a venue, so peer-reviewed papers drop to
tier-3. Add an `accelerator research fetch doi metadata` command that
reports the venue and publisher Crossref holds for a DOI, queued through a
project-wide engine generalised from arXiv's and optionally identified to
Crossref by a new `crossref.mailto` setting; have the web profile run it on
any DOI a cited page declares or its resolver URL names, and tier the source
by that venue — an intended change that also re-tiers publisher pages that
declare their DOI, not only resolver URLs; and lock in search/lookup parity
with a regression test.

## Context

In a `research-topic conduct` run on `1.24.0-pre.74`, three DOIs appeared at
tier-1 in some notes and tier-3 in others:

| DOI | One record | Other record |
|---|---|---|
| `10.1109/icdcsw.2005.79` | tier-1, venue ICDCSW | tier-3, no venue |
| `10.1145/371920.372066` | tier-1, venue WWW | tier-3, venue `doi.org` |
| `10.1145/3448016.3457556` | tier-1, venue SIGMOD | tier-3, venue `doi.org` |

Re-checked 2026-10-09: `accelerator research fetch openalex lookup` returns
tier-1 with the correct conference venue for all three, whether given a bare
DOI, a `https://doi.org/` DOI or a W-id (OpenAlex's work identifier);
`search` agrees. The raw OpenAlex
API returns the same work and `primary_location` for `/works/doi:` and
`filter=doi:`, and every request path uses the same `OPENALEX_SELECT`. The
field-selection hypothesis does not hold.

A venue of `doi.org` matches the web profile's behaviour: it tiers by
"domain or venue" and records the domain, and `doi.org` is not a recognised
venue. The web profile has no rule that a DOI resolver URL stands for the
publication behind it. The composer agents that build findings from
researchers' notes carry each note's tier into the finding, so the tier-3 is
carried forward. The conflict with the openalex tier-1 then surfaces where PP-892's
citing-once rule merges the two records.

Row 1 ("tier-3, no venue") does not fit the resolver explanation and is not
addressed by this item; see Open Questions.

Expected: every record of a peer-reviewed DOI carries its publication venue
and the tier that venue earns — for rows 2 and 3, tier-1 and the conference
venue — whichever profile cited it and whether by bare DOI or resolver URL.
For peer-reviewed journal and conference DOIs like rows 2 and 3, the tier
must agree across records whenever Crossref answers `ok` with a venue; this
is an expected outcome for such DOIs, not a general invariant over every
OpenAlex tiering. The venue string need not match: Crossref reports a full proceedings
title where OpenAlex reports a short name such as WWW. When Crossref is
`unavailable`, the landing-host and no-venue fallbacks below are accepted
degradations, and the tier they give may still disagree.

The baseline run (see Baseline) reproduced the web-profile route to a
`doi.org` venue directly; the manual run before closing confirms the fix.

Reproduction (confirmed by the baseline; see Baseline):

- Input: a web-profile `research-topic conduct` brief giving
  `https://doi.org/10.1145/371920.372066` and
  `https://doi.org/10.1145/3448016.3457556` as sources to cite, and naming
  nothing else about them.
- Action: run it on `1.24.0-pre.105`.
- Expected: each note records the conference venue at tier-1.
- Actual: each note records `doi.org` in its venue, for example
  `doi.org → dl.acm.org (ACM Digital Library; WWW10 proceedings)`, with no
  metadata block, because `fetch web metadata` gets HTTP 403 from
  `dl.acm.org`. The tier is tier-1, but only because the researcher
  improvised a `WebFetch` of `api.crossref.org` and cited the Crossref
  record as a tier-1 source in its own right. No profile rule produces
  that tier, so it is not reliable; the incident on `1.24.0-pre.74` gave
  tier-3.

## Requirements

- The web profile names `doi.org` and `dx.doi.org` as resolvers that are
  never recorded as a venue.
- A new `accelerator research fetch doi metadata '<DOI>'` command:
  - accepts a bare DOI or a `doi.org` / `dx.doi.org` URL;
  - on success prints `{"status":"ok","metadata":{…},"venue":"…"}`, where
    `venue` is the first element of the `container-title` list Crossref
    holds for the DOI, printed only when the record's Crossref `type` is
    `journal-article` or `proceedings-article`, and omitted when the list
    is empty or absent or the type is any other (a book chapter's
    `container-title` is its book, so a chapter gets no `venue`), and
    `metadata` carries
    only keys `fetch web metadata` never prints — `publisher` today; a DOI
    Crossref holds no record for is `ok` with empty `metadata` and no
    `venue`;
  - must print only metadata keys that sit after every key
    `fetch web metadata` can print in `RECOMMENDED_KEYS` order (PP-936's
    canonical metadata key order). As a result,
    the `web metadata` map followed by the `doi metadata` map is itself in
    canonical order;
  - reports a venue, never a tier;
  - calls only Crossref, with no API key. It identifies itself with a
    configured contact address so that Crossref serves it from the polite
    pool (Crossref's higher-limit pool for clients that send a contact
    address), and calls anonymously, from the public pool, when none is
    configured;
  - waits its turn in a project-wide Crossref queue, paced to the limits
    Crossref publishes for the pool in use: public, 5 requests per second
    and 1 concurrent; polite, 10 requests per second and 3 concurrent. The
    two pools are separate allowances: each call is paced to the pool it
    actually uses, which is the public pool whenever its address is unset
    or not plausible, so callers in one project that use different pools
    are paced independently. A
    `Retry-After` from Crossref is honoured by every caller in the project
    (every `accelerator` process working in the same project, which share
    the queue's state), as arXiv's is;
  - finishes each invocation within 100 s, as an arXiv call does. An
    invocation the queue cannot serve in that time prints
    `{"status":"waiting","source":"crossref","ticket":…,"position":…}` for
    the caller to re-present with `--ticket`. An invocation still in retry
    backoff when its 100 s runs out also prints `waiting`, carrying its last
    retry's reason forward to report if the ticket later expires. A ticket
    not settled within
    900 s of its issue ends as `unavailable`, as an arXiv ticket does. If
    the ticket's last attempt failed with a retryable reason, that reason
    is reported (for example `upstream_error` after a 503) and there is no
    `cause` field. Otherwise the `reason` is `rate_limited` and a `cause`
    field of `lock_contention` is added;
  - prints `{"status":"unavailable","source":"crossref","reason":…}` when
    the call's retries run out within its 100 s, or when Crossref answers
    a 4xx other than 404 and 429. The reason is that of the last attempt:
    `upstream_error` when Crossref was unreachable or answered 5xx or such
    a 4xx, `rate_limited` when it answered 429. Only the 100 s bound is
    contractual; the retry count and backoff are the shared engine's.
    Crossref has no quota, so `budget_exhausted` (OpenAlex's quota signal)
    never occurs. `ok`, `unavailable` and `waiting` all exit 0;
  - exits 2 with `E_DOI_MALFORMED` for a DOI it cannot parse. Its only
    other non-zero exits are the shared engine's ticket usage errors, also
    exit 2 and printed before any request, as arXiv's `E_ARXIV_TICKET_*`
    are today: a malformed ticket, a ticket presented with arguments other
    than those it was issued for, and a ticket another call is presenting
    right now. They are named under the `doi` family's prefix
    (`E_DOI_TICKET_MALFORMED`, `E_DOI_TICKET_MISMATCH`,
    `E_DOI_TICKET_LIVE`). A ticket issued by another family is a mismatch,
    reported under the prefix of the command it was presented to.
- The project-wide queue that arXiv calls use today becomes a
  service-agnostic engine that arXiv and Crossref share, each supplying its
  own pacing, `source` and ticket family (the set of tickets one service
  issues and accepts, so a Crossref ticket is never served by the arXiv
  queue). arXiv's observable behaviour
  (pacing, `waiting`/`--ticket`, `unavailable` reasons, ticket cap, exit
  codes) is unchanged, except across the upgrade itself. Queue state and
  tickets left by the previous release are invalidated, not migrated: a
  ticket issued before the upgrade gets a ticket usage error
  (`E_ARXIV_TICKET_MISMATCH`, exit 2), and the caller re-runs without
  `--ticket`. The new release never reads or writes the previous release's
  pacing or queue contents; it only lists ticket names in the previous
  `arxiv-queue/` for provenance and takes the previous `arxiv.lock` as a
  lock. Pacing is not shared while both releases run in one project, an
  accepted risk.
- Design choices settled in planning:
  - ticket provenance comes from the directory a ticket is found in: a
    ticket in another family's queue directory, or in the previous
    release's `arxiv-queue/`, is a mismatch;
  - all arXiv queue and pacing state moves to a per-service directory;
  - the multi-process timing criteria run in real time;
  - a Crossref 3xx or an undecodable body is `unavailable` with reason
    `upstream_error`;
  - a ticket re-presented after its pool changed joins the new pool
    afresh, with one diagnostic naming the pool that issued it;
  - a DOI ticket matches its call whatever the DOI's letter case;
  - the arXiv gate holds the legacy `arxiv.lock` while serving, so old and
    new releases keep one arXiv connection at a time;
  - the web profile repeats a CLI `warning:` line once in the researcher's
    summary, and `conduct` lists each distinct warning once in its final
    summary.
- A new optional `crossref.mailto` setting holds that contact address,
  resolved from the `ACCELERATOR_CROSSREF_MAILTO` env var, then
  `config.local.md`, then `config.md`. The first source that holds a
  non-empty value (after trimming) is the one used; an empty value counts as
  unset. A plausible address, once trimmed, has only visible ASCII with no
  `(`, `)` or `\` (so it can sit in a `User-Agent` comment), exactly one
  `@`, a non-empty local part containing no `:`, and a domain containing a
  dot that neither starts nor ends with one. When the value used is not
  plausible, the command prints a `warning:` and calls anonymously, without
  falling through to a lower-precedence source; the setting never makes a
  call fail. A config file that cannot be read stops the chain the same
  way, and a source that holds a value leaves the files below it unread.
  `/configure` documents the `crossref` section, and advises keeping a
  personal address out of the shared `config.md`.
- `accelerator research fetch --help` lists the `doi` family and its
  `metadata` verb, and names `doi` among the families `--ticket` serves.
- The web profile instructs the researcher, for every cited page, to:
  - run `fetch web metadata` first, then `fetch doi metadata` on the DOI it
    printed for a resolver URL or for a page that DOI is tied to, or — when
    it printed none, or failed (for example on a landing page that refuses
    the fetch), and the cited URL is a resolver URL — on that URL; a source
    reaching neither branch is tiered by its own domain, as today;
  - for a non-resolver page whose `web metadata` printed a DOI, `WebFetch`
    `https://doi.org/{doi}` without following its redirect. The DOI is
    **tied** to the page only when that fetch's landing host is the page's
    own host. An untied DOI stays in the metadata block as inert text, so a
    page never raises its own tier by declaring another publication's DOI;
  - for a resolver URL, `WebFetch` it after `fetch web metadata` and
    before `fetch doi metadata`, every time, to learn the landing host
    (step 3 below). The researcher never infers the landing host without
    this fetch;
  - re-present a `waiting` call at once with `--ticket`, never changing the
    DOI, until it prints anything other than `waiting`. After 20
    consecutive `waiting` results the researcher stops and handles the call
    as it would a printed `unavailable`. The researcher records no `reason`
    for this, since the CLI printed none;
  - on `E_DOI_TICKET_LIVE`, handle the source as for a printed
    `unavailable`, recording no `reason`, since a metadata call never ends
    the node. On `E_DOI_TICKET_MALFORMED` or
    `E_DOI_TICKET_MISMATCH`, correct the call and continue, as the arXiv
    profile does for its other usage errors. On `E_DOI_MALFORMED`, handle
    the source as for a printed `unavailable` (step 3 below);
  - write the entry's metadata block as the `web metadata` entries followed
    by the `doi metadata` entries, each copied unchanged in emitted order,
    following an explicit template and a worked example in the profile;
  - record the source's venue and tier, never by the resolver. Steps 1 and
    2 below are new, explicit rules that the profile's Reputation Tiers
    section gains, and they hold whatever its other rules would say. Step 3
    uses the section's existing domain rules. The venue, and the tier it
    earns, is the first that applies of:
    1. the `venue` an `ok` `doi metadata` prints: the command prints one
       only for a journal or proceedings article, a peer-reviewed
       publication, so tier-1;
    2. the `publisher` an `ok` `doi metadata` prints without a `venue`:
       tier-2, because the publisher is known but Crossref does not
       establish peer review (books, chapters, reports, datasets and any
       other non-article type);
    3. the landing host, when `doi metadata` is `ok` with neither (a
       Crossref miss) or `unavailable`. The landing host is the host of the
       first redirect `doi.org` returns, not of any later hop. The
       researcher's `WebFetch` of the resolver
       URL reports it, because `WebFetch` returns a cross-host redirect
       instead of following it, so the host is known even when it then
       refuses the fetch. For a DOI tied to a page that is not a
       resolver URL, the landing host is that page's host. The host is
       tiered by the profile's existing domain rules, so `arxiv.org` is
       tier-2, matching the arXiv profile's tier for every preprint;
    4. nothing, when no landing host is known (the DOI is unregistered or
       `doi.org` is unreachable): the venue is left unrecorded and the
       source is tier-3, cited by its resolver URL;
  - treat both kinds of source cited by DOI the same way: a resolver URL,
    and a non-resolver page whose declared DOI is tied to it. Both use the
    combined
    `web metadata` + `doi metadata` template. A resolver URL keeps the
    resolver URL as the entry's link, never the landing page's URL, so the
    citing-once rule can still read the DOI from it whatever
    `fetch web metadata` returned. A non-resolver page keeps its own URL,
    and its DOI travels in the `doi` key.
- The finding outputter's metadata block rule allows entries from more than
  one CLI command for a source, written one after another in the order the
  profile names, each copied unchanged. The citing-once rule needs no
  change: its metadata merge already takes every key any citation carries
  and rewrites the merged block in canonical order.
- A regression test in `cli/research-cli/tests/fetch_openalex.rs` asserts
  that `search` (the `/works` search endpoint with
  `title_and_abstract.search`) and `lookup` (by DOI, by `https://doi.org/`
  DOI, and by W-id) produce the same tier, venue
  and venue signals (the evidence about the venue that the CLI's tier is
  derived from) for the same work.

## Acceptance Criteria

- [x] The web profile's text names `doi.org` and `dx.doi.org` as resolvers
      that are never recorded as a venue.
- [x] The web profile's text directs a researcher to run
      `fetch web metadata` first for every cited page, then
      `fetch doi metadata` on the DOI it printed for a resolver URL or for
      a page that DOI is tied to, or — when it printed none, or failed, and
      the cited URL is a resolver URL — on that URL; a source reaching
      neither branch is tiered by its own domain.
- [x] The web profile's text directs the researcher, for a non-resolver
      page whose `web metadata` printed a DOI, to `WebFetch`
      `https://doi.org/{doi}` without following its redirect, treats the
      DOI as tied only when that fetch lands on the page's own host, and
      keeps an untied DOI in the metadata block as inert text.
- [x] The web profile's text directs a researcher to record and tier a
      source that went through `fetch doi metadata` by the first that
      applies of: the printed `venue`; the printed `publisher`; the landing
      host (the host `doi.org` redirects to, or the declaring page's host)
      when the call is `ok` with neither or `unavailable`; and, when no
      landing host is known, no venue at tier-3 — never `doi.org`. The text
      states the tier-1 rule for a journal or proceedings title and the
      tier-2 rule for a publisher alone as explicit Reputation Tiers rules.
- [x] The web profile's text directs the researcher to `WebFetch` every
      resolver URL, after `fetch web metadata` and before
      `fetch doi metadata`, to learn its landing host, and never to infer
      the landing host without that fetch.
- [x] The web profile's text directs the researcher, on
      `E_DOI_TICKET_LIVE`, to handle the source as for `unavailable`,
      recording no `reason`; on `E_DOI_TICKET_MALFORMED` or
      `E_DOI_TICKET_MISMATCH`, to
      correct the call and continue; and on `E_DOI_MALFORMED`, to handle
      the source as for `unavailable`.
- [x] The web profile's text defines the landing host as the host of the
      first redirect `doi.org` returns.
- [x] The web profile's text includes one worked example for each of the
      four venue branches, each with its resulting venue and tier:
      `10.1145/371920.372066`, venue `Proceedings of the 10th international
      conference on World Wide Web`, tier-1; a publisher-only DOI, venue
      its publisher, tier-2; `https://doi.org/10.48550/arXiv.…`, for which
      Crossref holds no record, venue `arxiv.org`, tier-2; and a resolver
      URL that
      `doi.org` cannot redirect, with Crossref `unavailable`, no venue at
      tier-3. Every resolver-URL example keeps the resolver URL as its
      link. A fifth example shows a page whose declared DOI is not tied to
      it, tiered by its own domain.
- [x] The web profile's text includes a Waiting section that: re-runs the
      same `fetch doi metadata` call at once with `--ticket` set to the
      latest `waiting` output's ticket; never changes the DOI while
      re-presenting; repeats until the status is not `waiting`; and after
      20 consecutive `waiting` results handles the call as a printed
      `unavailable`, recording no `reason` of its own.
- [x] The web profile's text directs the researcher to copy the
      `web metadata` and `doi metadata` entries unchanged and in emitted
      order.
- [x] The web profile's text includes a template for a source cited by DOI,
      with `web metadata` entries first and `doi metadata` entries after,
      and a worked example whose link is a `https://doi.org/…` URL and whose
      venue is not `doi.org`.
- [x] The web profile's text directs the researcher to keep the resolver URL
      as the link of a resolver-URL source, and its own URL as the link of
      a non-resolver page that declares a DOI, and to use the combined
      template for both.
- [x] The finding outputter's metadata block rule allows entries from more
      than one CLI command, written one after another in the order the
      profile names.
- [x] Given a recorded Crossref response for `10.1145/371920.372066`, when
      `fetch doi metadata '10.1145/371920.372066'` runs, then it exits 0,
      `venue` is `Proceedings of the 10th international conference on World
      Wide Web`, `metadata` holds only `publisher: ACM`, the output has no
      `tier` key, and the only outbound request goes to `api.crossref.org`
      with no `Authorization` or `Crossref-Plus-API-Token` header and no
      query parameters.
- [x] Given a recorded `journal-article` Crossref response whose
      `container-title` holds two titles, when the command runs, then
      `venue` is the first.
- [x] Given a recorded `book-chapter` Crossref response with a
      `container-title` and a `publisher`, when the command runs, then it
      exits 0 with no `venue` key and `metadata` holding the `publisher`.
- [x] Given the recorded `10.1145/371920.372066` response, when the
      command is given
      `https://doi.org/10.1145/371920.372066` or
      `https://dx.doi.org/10.1145/371920.372066`, then its output is
      identical to the bare-DOI output.
- [x] Given recorded Crossref responses with a publisher and with
      `container-title` absent, and with it set to `[]`, when the command
      runs, then each exits 0 with no `venue` key and `metadata` holding the
      `publisher`.
- [x] Given a recorded Crossref response with neither `container-title` nor
      `publisher`, when the command runs, then it exits 0 and prints
      `{"status":"ok","metadata":{}}` with no `venue`.
- [x] Given Crossref answers 404, when the command runs, then it exits 0
      and prints `{"status":"ok","metadata":{}}` with no `venue`.
- [x] Given an injected clock and Crossref unreachable, when the command
      runs, then it exits 0 no later than 100 s on that clock and prints
      `{"status":"unavailable","source":"crossref",…}` with a reason of
      `upstream_error`.
- [x] Given an injected clock and Crossref answering caller A's request with
      429 and `Retry-After: 5`, where A and B are separate processes in the
      same project reading one project-scoped test clock, when A and B next
      request Crossref, then neither request is issued before 5 s after A
      received the 429, on that clock.
- [x] Given Crossref answering 400, when the command runs, then it exits 0
      and prints `unavailable` with reason `upstream_error`.
- [x] Given an injected clock and Crossref answering every attempt with 503,
      when the command runs until its retries run out, then it exits 0 no
      later than 100 s on that clock and prints
      `{"status":"unavailable","source":"crossref","reason":"upstream_error"}`.
- [x] Given an injected clock and Crossref answering every attempt with 429,
      when the command runs until its retries run out, then it exits 0 no
      later than 100 s on that clock and prints `unavailable` with reason
      `rate_limited`.
- [x] Given an injected clock and Crossref answering 503 on every attempt
      but the last, which it answers 429, when retries run out, then the
      reason is `rate_limited`; with 429 on every attempt but a final 503,
      it is `upstream_error`.
- [x] Rate: given a project-scoped test clock read by at least two
      separate processes in the same project, Crossref requests that each
      complete 50 ms after they start, and 10 queued calls spread across
      those processes, when they run, then with no `crossref.mailto` no
      more than 5 requests start in any half-open 1 s window [t, t+1 s)
      and all 10 start within [0, 2 s). With `crossref.mailto` set, no
      more than 10 start in any such window and all 10 start within
      [0, 1 s). With `crossref.mailto` set to `not-an-address`, the public
      limits apply.
- [x] Mixed pools: given the same setup, with one process whose
      `ACCELERATOR_CROSSREF_MAILTO` is `research@example.org` and one with
      none, each queuing 10 calls, when they run, then the anonymous
      process's requests keep to the public limits and the polite
      process's to the polite limits, each independently of the other.
- [x] Concurrency: given the same setup but with requests that each
      complete 2 s after they start, when the 10 calls run, then with no
      `crossref.mailto` exactly one request is in flight at a time. With
      `crossref.mailto` set, no more than 3 are ever in flight, and 3 are
      in flight together at some point.
- [x] Given an injected clock and the arXiv queue honouring
      `Retry-After: 60` from t=0, when a Crossref call is issued at t=0
      with an empty Crossref queue, then its request starts before t=1 s.
      The same holds with the services swapped.
- [x] Given an injected clock and the queue held by another ticket past the
      invocation's 100 s, when the command runs, then it exits 0 no later
      than 100 s on that clock and prints
      `{"status":"waiting","source":"crossref","ticket":…,"position":…}`
      with a position of at least 1.
- [x] Given the `waiting` ticket from the previous criterion, once the
      other ticket holding the queue settles and Crossref returns the
      recorded `10.1145/371920.372066` response, when the call is
      re-presented with `--ticket`, then it prints the same `ok` output as
      an unqueued call.
- [x] Given a waiting Crossref ticket presented to an arXiv fetch, then the
      command exits 2 with `E_ARXIV_TICKET_MISMATCH`. Given a waiting arXiv
      ticket presented to `fetch doi metadata`, or a Crossref ticket
      presented with a different DOI, then it exits 2 with
      `E_DOI_TICKET_MISMATCH`. None of these issues a request.
- [x] Given a malformed `--ticket` value, when `fetch doi metadata` runs,
      then it exits 2 with `E_DOI_TICKET_MALFORMED` and issues no request.
- [x] Given a Crossref ticket another call is presenting, when it is
      presented again, then the command exits 2 with `E_DOI_TICKET_LIVE`
      and issues no request.
- [x] Given an injected clock and a Crossref ticket still unsettled 900 s
      after its issue, when it is re-presented, then the command exits 0 and
      prints `{"status":"unavailable","source":"crossref",…}` with a `cause`
      field of `lock_contention` and a `reason` of `rate_limited`.
- [x] Given an injected clock and a Crossref ticket whose last attempt
      answered 503, still unsettled 900 s after its issue, when it is
      re-presented, then the command prints `unavailable` with a `reason`
      of `upstream_error` and no `cause` field; when that last attempt
      answered 429, the `reason` is `rate_limited`, still with no `cause`.
- [x] Given an injected clock and an invocation still in retry backoff
      after a 503 when its 100 s runs out, when the command returns, then
      it prints `waiting`, and if that ticket then expires unsettled the
      `reason` is `upstream_error` with no `cause`.
- [x] The web profile's text directs `fetch doi metadata` on a DOI
      `fetch web metadata` printed for a non-resolver page only when that
      DOI is tied to the page.
- [x] Given a DOI ticket issued in one Crossref pool and re-presented after
      `crossref.mailto` selects the other, when the command runs, then the
      call joins the other pool afresh and prints one diagnostic naming
      the pool that issued the ticket.
- [x] The web profile's text directs that, when `fetch web metadata` fails
      on a resolver URL (for example exits 1 because the landing page
      refuses the fetch), the researcher still runs `fetch doi metadata` on
      that URL.
- [x] Given `not-a-doi`, when the command runs, then it exits 2 with
      `E_DOI_MALFORMED`.
- [x] Given `crossref.mailto` is `research@example.org`, when
      `fetch doi metadata` runs, then the Crossref request's `User-Agent`
      carries `mailto:research@example.org` and no request URL contains the
      address.
- [x] Given no contact address is configured, when the command runs, then
      the request carries no `mailto` and the output is the same as with
      one.
- [x] Given `crossref.mailto` is `not-an-address`, when the command runs,
      then it prints one `warning:` line on stderr, calls anonymously, and
      its exit status and stdout are unaffected. The same holds for `a@b`,
      `a@.org`, `a@example.`, `@example.org`, `a@b@example.org`,
      `research @example.org` and
      `mailto:x@example.org`, while ` research@example.org ` (trimmed) is
      accepted.
- [x] Given `crossref.mailto` set to an empty string or ` `, when the
      command runs, then it prints no `warning:` and calls anonymously, as
      if unset.
- [x] Given `ACCELERATOR_CROSSREF_MAILTO` set to ` ` and
      `research@example.org` in `config.md`, when the command runs, then it
      prints no `warning:` and the `User-Agent` carries
      `mailto:research@example.org`.
- [x] Given addresses in the env var, `config.local.md` and `config.md`,
      when the command runs, then it uses the env var's. Given only
      `config.local.md` and `config.md`, it uses `config.local.md`'s; given
      only `config.md`, it uses `config.md`'s.
- [x] Given `not-an-address` in the env var and `research@example.org` in
      `config.md`, when the command runs, then it prints one `warning:` and
      calls anonymously.
- [x] Given an unreadable `config.local.md` and `research@example.org` in
      `config.md`, when the setting resolves, then it reports the
      unreadable file and does not read `config.md`. Given a value in a
      higher source, an unreadable file below it is never read.
- [x] `/configure` documents `crossref.mailto`, its precedence and the
      anonymous fallback.
- [x] A CLI test asserts that every key `fetch doi metadata` can print
      follows every key `fetch web metadata` can print in `RECOMMENDED_KEYS`
      order.
- [x] Before the queue is generalised, each arXiv behaviour the queue
      requirement lists (pacing, `waiting`/`--ticket`, `unavailable`
      reasons, ticket cap, exit codes) has an arXiv test, with
      characterisation tests added for any gap. Before the refactor
      begins, a mapping from each listed behaviour (including each
      `unavailable` reason and each `E_ARXIV_TICKET_*` exit) to the named
      test that covers it is recorded in the plan. Afterwards, those tests
      pass unchanged against the shared queue engine.
- [x] Given an arXiv ticket and queue state captured from the previous
      release as a committed fixture, when the ticket is re-presented after
      the upgrade, then the command exits 2 with `E_ARXIV_TICKET_MISMATCH`
      and issues no request, and a fresh call without `--ticket` prints the
      same `ok` output an unqueued call gives for a recorded arXiv
      response.
- [x] `accelerator research fetch --help` names the `doi` family and its
      `metadata` verb, and lists `doi` among the families `--ticket` serves.
- [x] Given OpenAlex responses for `10.1145/3448016.3457556` recorded
      separately from the `/works` search endpoint (with
      `title_and_abstract.search`), `/works/doi:` and `/works/W…`, in a
      test in `cli/research-cli/tests/fetch_openalex.rs`, when the CLI tiers the work via `search`, `lookup <DOI>`,
      `lookup https://doi.org/<DOI>` and `lookup <W-id>`, then all four
      records carry identical `tier`, `venue` and `venue_signals`, with
      tier-1 and the exact venue string the recorded fixture holds.
- [x] Before any building, a baseline web-profile `research-topic conduct`
      run on the current plugin is given
      `https://doi.org/10.1145/371920.372066` and
      `https://doi.org/10.1145/3448016.3457556` in its brief as sources to
      cite, and its notes are kept as evidence. The baseline counts only if
      both URLs appear as cited web-profile entries; otherwise it is
      inconclusive and is run again; after three inconclusive runs, work
      stops and the brief is revisited. The implementer judges each run and
      records the result in this item, with the item author's sign-off
      before work stops or proceeds. If a counting baseline shows neither
      `doi.org` as a venue nor a tier below tier-1, work stops and the cause
      is revisited before anything is built. A counting baseline that
      reproduces the bug has its brief and observed venue and tier added to
      Context as the confirmed reproduction. The baseline notes also record
      the non-resolver page the closing run will use: its URL, the DOI it
      declares, and that DOI's Crossref venue and expected tier.
- [x] Before the item closes, a manual web-profile `research-topic conduct`
      run on the finished change is given the same two resolver URLs, a
      `https://doi.org/10.48550/arXiv.…` URL and a non-resolver page
      recorded in the baseline notes, as sources to cite. The arXiv URL's
      note records venue `arxiv.org` at tier-2. Each resolver URL's note records the Crossref venue at tier-1
      (the tier `fetch openalex lookup` reports for the same DOI) and keeps
      the resolver URL as its link. The page's note carries its DOI's
      Crossref venue at the tier the baseline notes expect. No entry
      records `doi.org` as a venue, and the run's
      dossier shows no tier conflict for `10.1145/371920.372066`,
      `10.1145/3448016.3457556` or the baseline-recorded page's DOI. If
      any of them goes uncited, the check fails rather than passing. The run's
      notes are kept beside the baseline's as evidence, recording for each
      resolver URL whether its `web metadata` succeeded or failed. If no
      resolver URL takes the failed branch, that is recorded as accepted
      rather than forced. A single passing run is accepted.

## Baseline

Run on Claude Code `2.1.330` with plugin `1.24.0-pre.105`, web profile only,
depth 1. Both runs count: each cites both resolver URLs as web-profile
entries. Their subjects were deleted afterwards as test residue, so this
section is the record of the runs.

| Run | Brief | Recorded venue and tier |
|---|---|---|
| 1 | names each paper's subject | `doi.org / ACM, WWW 2001 proceedings`, tier-1; `doi.org / ACM, SIGMOD 2021 proceedings`, tier-1 |
| 2 | names only the two URLs | `doi.org → dl.acm.org (ACM Digital Library; WWW10 proceedings)`, tier-1; `doi.org → dl.acm.org (ACM Digital Library; SIGMOD '21 proceedings)`, tier-1 |

- `doi.org` is recorded in every resolver URL's venue. The tier-3 drop did
  not reproduce; the tier-1 rests on an improvised Crossref lookup, cited
  as a separate tier-1 source, which no profile rule asks for.
- `fetch web metadata` on both resolver URLs fails with
  `E_WEB_STATUS: the page answered HTTP 403` from `dl.acm.org`, so the
  closing run is expected to take the failed-`web metadata` branch for
  both.
- `WebFetch` on Claude Code `2.1.330` returns a cross-host `302 Found` to
  `https://dl.acm.org/doi/<DOI>` for both resolver URLs, without following
  it, so the landing host is `dl.acm.org`. Claude Code `2.1.144`, the
  plugin's minimum, behaves the same in a headless run: both URLs report
  `REDIRECT DETECTED` with `Status: 302 Found` and the same redirect URL.
  There is no gap, so the profile needs no no-redirect sentence.
- Non-resolver page for the closing run:
  `https://www.nature.com/articles/nature14539`. `fetch web metadata`
  prints `doi: 10.1038/nature14539`. Crossref holds that DOI as a
  `journal-article` with `container-title[0]` `Nature` (publisher Springer
  Science and Business Media LLC), so the expected tier is tier-1.

Signed off by Toby Clemson on 2026-11-22: the baseline confirms the
reproduction, and building proceeds.

## Closing Run

Run on Claude Code `2.1.330` against this branch's working tree: the skills
loaded with `--plugin-dir` and `accelerator` resolved to the debug builds,
with the installed prerelease plugin disabled. Web profile only, depth 1.
One run, and it passes. Its subject was deleted afterwards as test residue,
so this section is the record of the run.

| Source | Venue and tier recorded | `fetch openalex lookup` |
|---|---|---|
| `https://doi.org/10.1145/371920.372066` | `Proceedings of the 10th international conference on World Wide Web`, tier-1, resolver URL as link | tier-1 |
| `https://doi.org/10.1145/3448016.3457556` | `Proceedings of the 2021 International Conference on Management of Data`, tier-1, resolver URL as link | tier-1 |
| `https://doi.org/10.48550/arXiv.1706.03762` | `arxiv.org`, tier-2 | — |
| `https://www.nature.com/articles/nature14539` | `Nature`, tier-1, as the baseline expected | tier-1 |

- No entry in the finding or the dossier records `doi.org` as a venue.
  Each of the three DOIs is cited once in the dossier, with no `also`
  suffix, so it shows no tier conflict.
- `fetch web metadata` failed (exit 1) on both resolver URLs, as the
  baseline predicted, so both took the failed branch. Their entries carry
  only `publisher: ACM`. It succeeded on the arXiv and Nature URLs.
- The researcher ran all four `fetch doi metadata` calls before its
  `WebFetch` of the resolver URLs, against the profile's order. The tier
  did not depend on it, since Crossref gave both ACM DOIs a venue.
- The researcher also fetched both ACM DOIs' `api.crossref.org` records
  itself and cited each as a separate tier-1 source, as in the baseline.
  The tiers no longer rest on them.
- A first attempt under auto mode wrote no finding: the auto mode classifier
  gave no verdict on the researcher's `Write`. The passing run used
  `acceptEdits`.
- The run predates the tied-DOI rule and the `E_DOI_TICKET_LIVE` handling
  that the PR #258 review added. No live run has exercised either; both
  rest on the web profile's structure pins.

## Open Questions

- Row 1 of the Context table (`10.1109/icdcsw.2005.79`, "tier-3, no
  venue") is unexplained and out of this item's scope. Settling it needs
  the original run's notes, which are not in this repository; if they turn
  up, file a separate item.

## Dependencies

- Blocked by: none
- Blocks: none known
- Related: PP-936 (done), which defines the metadata keys and canonical
  order both commands emit; PP-892 (done), whose citing-once rule reads a
  DOI from a resolver URL and handles whatever tier conflicts remain once
  this source of them is removed; PP-864 (done), whose arXiv fetch family owns the
  project-wide queue the Crossref calls share, and whose behaviour must not
  change; PP-865 (in progress), which reads a source's `doi` key from the
  metadata block this item lets two commands write. Neither blocks the
  other: `doi` is a `web metadata` key, which still comes first, and the
  citing-once merge already handles the combined block.
- Coordination: PP-897 (draft) also rewrites the web profile's "Reputation
  Tiers" section. Whichever lands second reconciles with the other's text,
  and PP-897's domain classes must not classify `doi.org` or `dx.doi.org`,
  and must keep preprint hosts such as `arxiv.org` at tier-2, the arXiv
  profile's tier, which this item's landing-host example relies on.
- Coupling: the arXiv profile's tier-2 for every preprint is relied on by
  the landing-host rule for `arxiv.org`. A change to that tier must be
  reconciled with the web profile's rule.
- External: the Crossref REST API (`api.crossref.org`), keyless; the polite
  pool depends on `crossref.mailto`. Crossref holds no records for DataCite
  DOIs such as arXiv's `10.48550/…`, and an outage or persistent rate
  limiting surfaces as `unavailable`. The pool limits paced to are
  Crossref's published ones
  (https://www.crossref.org/documentation/retrieve-metadata/rest-api/access-and-authentication/)
  and must be revisited if Crossref changes them. Live access to Crossref
  and `doi.org` is needed for the baseline and closing runs, and to capture
  the recorded Crossref fixtures (including finding a publisher-only DOI and
  one with two container titles); hand-built fixtures are acceptable where
  no live record fits. OpenAlex access is needed to capture the parity
  test's fixtures and for the closing run's `fetch openalex lookup`
  comparison; the shipped `doi metadata` command never calls OpenAlex.
- Platform: Claude Code's `WebFetch` returns a cross-host redirect instead
  of following it, which is how the researcher learns a landing host. If
  that changes, sources that need the landing host fall to step 4 (no venue,
  tier-3). The baseline run confirms the behaviour, and records the Claude
  Code version it ran on. It must also hold on the plugin's minimum
  supported version (v2.1.144); if it does not, that gap is recorded.
- Order within this item: the baseline run, which gates the rest; then
  arXiv characterisation tests for any uncovered queue behaviour; then
  generalise the queue engine; then the `doi metadata` command on the
  anonymous pool, verified end to end — the point at which
  `crossref.mailto` and the parity test could split into follow-up items
  if this one runs long; then `crossref.mailto` and polite pacing; then the
  profile and
  outputter text, which ships in the same plugin release as the command;
  then the manual closing run.

## Assumptions

- The `doi.org`-venue records came from web-profile notes. Inferred from the
  venue string; not confirmed against the original run's notes, which are
  not in this repository.
- A page declares its own DOI, so any DOI `web metadata` reports may go
  through `fetch doi metadata`. Preprint servers are believed to declare
  their own DOIs (arXiv's `10.48550/…` are DataCite DOIs Crossref does not
  hold, so they fall back to `arxiv.org`). Accepted risk: a page declaring
  a DOI that is not its own takes that DOI's venue and tier.
- Accepted risk: apart from the baseline and closing runs, the profile's
  acceptance criteria check its text only, so no repeatable test guards the
  fix against regression.

## Technical Notes

- Web profile tiering and the metadata fetch:
  `skills/research/profiles/web-profile/SKILL.md`, "Reputation Tiers". The
  profile already holds `Bash(accelerator research fetch *)`, so the new
  command needs no new permission.
- Metadata block rule: `skills/research/outputters/finding-outputter/SKILL.md`,
  "Sources Metadata Block". DOI identity from a resolver URL:
  `skills/research/outputters/citing-once/SKILL.md`, "Carrying a DOI".
- Metadata keys and canonical order: `RECOMMENDED_KEYS` in
  `cli/research/src/sources/metadata.rs`. `fetch web metadata` prints `doi`,
  `published`, `updated` and `accessed`, all before `publisher`; `volume`,
  `issue`, `pages` and `license` would also qualify for `doi metadata`
  later, `issn` and `type` would not.
- `fetch web metadata` takes a `doi` only from what the landing page
  declares (`cli/research/src/sources/web/extract.rs`); publisher sites
  often refuse automated fetches.
- Crossref: `https://api.crossref.org/works/{doi}` returns
  `message.container-title` (a list) and `message.publisher`. Its base URL
  would sit beside the OpenAlex and arXiv ones in
  `cli/research/src/sources/request.rs`, and `Doi` there already parses bare
  and resolver-URL forms.
- Crossref calls go through a project-wide queue like arXiv's, with the
  same `ok` / `unavailable` / `waiting` statuses (all exit 0) and `--ticket`
  re-presentation, paced to the limits of whichever Crossref pool the call
  uses (anonymous, or polite with `crossref.mailto`). A Crossref miss is
  `ok` with empty `metadata` and no `venue`, as an OpenAlex `lookup` miss is
  `ok` with no records; only malformed input exits non-zero
  (`E_DOI_MALFORMED`, exit 2).
- The queue engine is spread over `cli/research/src/sources/queue.rs` (with
  ticket handling in `fetch.rs` and `request.rs`),
  `cli/research-adapters/src/queue.rs` and `contention.rs`, and the
  `--ticket` plumbing in `cli/research-cli`. Crossref is its second
  consumer (see Requirements). It is generalised rather than duplicated
  because more low-funding, high-volume services like these are likely.
- Crossref's etiquette admits a client to the polite pool when it sends a
  contact address in its `User-Agent` or a `mailto` query parameter; the
  `User-Agent` keeps the address out of URLs, which tend to be logged.
- The `crossref` section mirrors `openalex` in
  `skills/config/configure/SKILL.md`. An address is not a secret, so unlike
  `openalex.api_key_cmd` it is not a consent key, and a team may commit a
  shared address in `config.md`.
- OpenAlex request paths all use `OPENALEX_SELECT` (`request.rs`); venue
  signals come from `venue_signals` in `cli/research/src/sources/openalex.rs`.
  The parity test belongs in `cli/research-cli/tests/fetch_openalex.rs`,
  which has none today.

## Drafting Notes

- Reframed from an OpenAlex CLI bug to a web-profile tiering bug at the
  user's direction, after the CLI failed to reproduce the issue.
- Kept the CLI parity regression test in scope even though the CLI appears
  correct — it turns the issue's claim into a guarantee.
- OpenAlex data changing between 2026-10-05 and 2026-10-09 was judged
  unlikely for 20-year-old records and not pursued.
- Venue resolution moved from the model into the CLI at the user's
  direction, as a new `doi` fetch family. `web metadata` reports what a page
  declares about itself and `doi metadata` what the DOI registry adds; a
  caller runs both and neither duplicates the other.
- Crossref is the venue source. Copying the tier from
  `fetch openalex lookup` was rejected: it couples the web profile to
  OpenAlex and spends its budget. The landing-host fallback stays with the
  researcher, whose page fetch already knows it, rather than a second
  redirect-follower in the CLI.
- `publisher` goes in `metadata` because it is a PP-936 key; the container
  title is not, so `venue` sits beside the map, as on OpenAlex records.
  Ordering `doi metadata` keys after every `web metadata` key, enforced by a
  test, makes the profile's template correct by construction rather than
  asking the model to sort.
- Falling back to the publisher when Crossref holds no container title
  (books, reports) was judged a better venue than the landing host.
- `web metadata` is unchanged for unreadable landing pages: the citing-once
  rule already reads a DOI from a resolver URL, so keeping that URL as the
  link is enough to preserve identity.
- The Crossref polite pool is opt-in at the user's direction, with the
  anonymous route as the default. The key name `crossref.mailto`, sending it
  in the `User-Agent` rather than the URL, allowing it in team `config.md`,
  and treating a malformed address as anonymous with a warning rather than
  an error were drafting choices.
- The stress test widened the trigger from resolver URLs to any DOI a
  cited page declares, so that a publisher's page cited directly is tiered
  by its venue, as OpenAlex tiers the same work, rather than by its host.
  That is the same class of cross-profile conflict as the resolver case.
  The stress test also replaced the `E_DOI_*` exit-1 contract with the
  arXiv-style shared queue, so rate limiting waits rather than silently
  falling back to a domain tier.
- Kept separate from PP-897 at the user's direction, though scope grew from
  a profile-text change to a new CLI command.
- Kept as one item after review, at the user's direction. The resolver rule
  on its own would only swap `doi.org` for the landing host, the outcome
  the Crossref route was chosen to avoid, so the pieces deliver the fix
  together. The queue generalisation was raised from Technical Notes into
  Requirements, with an arXiv non-regression criterion, so its scope and
  risk are visible to planning. The OpenAlex parity test shares no code
  with the rest and is bundled for convenience, not as a dependency.
  `kind` stays `bug`: the user-visible defect drives the work.
- Profile and outputter acceptance criteria check the skills' text only.
- Row 1 of the Context table ("tier-3, no venue", attributed to the CLI by a
  researcher) may have a different cause — a misreported `unavailable`
  response or a different identifier — but is unverifiable without the
  run's notes.

## References

- Source: https://github.com/atomicinnovation/accelerator/issues/142
- Related: PP-22, PP-864, PP-865, PP-892, PP-897, PP-936
- Crossref REST API: https://www.crossref.org/documentation/retrieve-metadata/rest-api/
