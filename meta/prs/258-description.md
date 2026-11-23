---
type: "pr-description"
id: "258"
title: "[PP-891] Tier DOI-cited web sources by their Crossref venue, not by doi.org"
date: "2026-11-23T01:34:16+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "PP-891"
parent: "work-item:PP-891"
relates_to: ["work-item:PP-888", "work-item:PP-897", "work-item:PP-936"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/258"
pr_number: 258
tags: ["research", "research-cli", "queue", "crossref", "web-profile", "config"]
revision: "644120cb2bc7a2d8e4f77017963213c59d7592c3"
repository: "accelerator"
last_updated: "2026-11-23T16:06:41+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [PP-891] Tier DOI-cited web sources by their Crossref venue, not by doi.org

## Summary

The web profile tiered a `https://doi.org/…` link by its domain, so
peer-reviewed papers were recorded with the venue `doi.org`. This PR adds
`accelerator research fetch doi metadata '<DOI>'`, which reports the venue
and publisher Crossref holds for a DOI, and teaches the web profile to tier
any source cited by DOI by that venue, never by the resolver. To pace the
new Crossref calls, the arXiv-only fetch queue becomes an engine shared by
every queued service.

## Changes

### CLI: `fetch doi metadata`

- **Command.** `fetch doi metadata` accepts a bare DOI, a `doi:` DOI, or a
  `doi.org` or `dx.doi.org` URL.
- **Output for a journal or proceedings article.**
  `{"status":"ok","metadata":{"publisher":…},"venue":…}`, where `venue` is
  the first container title.
- **Output for other records.** Any other Crossref type prints no `venue`.
  A DOI Crossref holds no record for (a 404) prints
  `{"status":"ok","metadata":{}}`.
- **Crossref's responses.**
  - A `429` is retried, honouring `Retry-After` for every process in the
    project.
  - A `5xx` or a transport failure is retried.
  - Any other `4xx`, an unfollowed redirect, or a body that cannot be read
    ends as `unavailable` with `upstream_error`, not exit 1. An unreadable
    body also writes one diagnostic naming the DOI and the decoder's reason.
- **Exit 2.** Only `E_DOI_MALFORMED` and `E_DOI_TICKET_*`, both before any
  request.
- **Tickets.** A DOI ticket matches its call whatever the DOI's letter case.
  A ticket re-presented after `crossref.mailto` changed which pool serves
  the call joins that pool afresh, with one diagnostic naming the pool that
  issued it, rather than losing its place silently.

### Queue engine (`research`, `research-adapters`)

- **Services.** `Service` is `Arxiv | Crossref(Public | Polite)`. Each
  service declares its pacing (`Pacing { interval, spaced_from }`), its
  `source` and its ticket family. The queue path and the retry loop moved
  out of `fetch_arxiv` into a shared `queued` engine returning
  `Queued<T>`, which renders `waiting`, rejections and `unavailable` once
  for every family.
- **Serving slots.** A ticket is served while fewer than `slots` unended
  tickets are ahead of it. Each slot is a `serving-<i>.lock`.
- **Pacing.** A short-lived `pacing.lock` replaces holding one lock for a
  whole call. A caller reserves a send time, sleeps outside the lock, then
  confirms against the last actual send and any deferral before sending.
- **Spacing per service.**
  - arXiv keeps 3 s from the previous finish.
  - Crossref spaces from the previous start: 210 ms with one slot (the
    public pool, 5/s) or 102 ms with three (the polite pool, 10/s).
- **State location.** All queue and pacing state moved to
  `<paths.tmp>/research/services/<service>/`.
- **Upgrade overlap.**
  - A ticket from the previous release, or one another family issued, is
    rejected with `E_<FAMILY>_TICKET_MISMATCH`. The CLI finds which by
    listing the other queues' directories read-only.
  - While serving, the arXiv gate also takes the legacy `arxiv.lock` as a
    companion lock, so old and new releases keep one arXiv connection at a
    time. `PreviousRelease::arxiv_layout` names those legacy paths in the
    adapters, not the composition root.
- **Ports.** The engine's ports (`Clock`, `PacingGate`, `Transport`, …) are
  reached only through `sources::ports`; `sources::fetch` no longer
  re-exports them. A `cli/pup.ron` rule,
  `research_queue_engine_never_reaches_fetch`, stops `ports`, `serving` and
  `admission` importing the fetch workflow, replacing three `include_str!`
  text checks.
- **Single source of truth.** `Family::is_queued` derives which families
  take `--ticket` from `Service::ALL`, and `DoiAdapters.service` comes from
  the `ServiceAdapters` bundle built for it.

### Config: `crossref.mailto`

- **Polite pool.** The new open setting `crossref.mailto` moves calls to
  Crossref's polite pool. The address travels only in the `User-Agent`.
- **Resolution.** `ACCELERATOR_CROSSREF_MAILTO`, then `config.local.md`,
  then `config.md`; the first non-blank value wins. This uses a new generic
  `open_value` ladder in `config`.
- **Implausible or unreadable value.** The call goes to the public pool
  with one `warning:` line, never a failure. Neither an implausible value
  nor an unreadable file falls through to a lower rung, and a rung that
  wins leaves the files below it unread.
- **Shared addresses.** `/configure` advises putting a personal address in
  `config.local.md` or the environment, since one in `config.md` goes out on
  every collaborator's calls.
- **Credential marker.** The catalogue now marks plaintext credentials, so
  a credential can never reach the open ladder.

### Skills and docs

- **Web profile.**
  - Run order: `fetch web metadata`, then a `WebFetch` of a resolver URL to
    learn its landing host, then `fetch doi metadata` on the declared DOI
    (or the resolver URL when `web metadata` printed none or failed).
  - Tied DOIs: a non-resolver page's declared DOI is used only when a
    `WebFetch` of `https://doi.org/{doi}` lands on the page's own host. An
    untied DOI stays in the metadata block as inert text, so a page cannot
    raise its tier by declaring another publication's DOI.
  - Tiering: a printed venue is tier-1, and a printed publisher alone is
    tier-2. Otherwise the landing host is tiered under the domain rules,
    and with no landing host known the source gets no venue at tier-3.
  - The profile also adds Waiting, warning and `E_DOI_*` handling, a
    combined metadata template and five worked examples, one of them an
    untied page. `E_DOI_TICKET_LIVE` is handled as an `unavailable` source;
    a metadata call never ends the node, so there is no Failed outcome.
- **Finding outputter.** A metadata block may come from several CLI
  commands.
- **`research-topic`.** Its recovery rows cover DOI tickets and the
  `crossref.mailto` warnings. `conduct` lists each distinct `warning:` line
  once in its final summary.
- **Docs.** `research.md` rewrites its Pacing section and adds the
  `doi metadata` section and `E_DOI_*` codes. `/configure` gains a
  `crossref` section. `CHANGELOG.md` records the command, the setting, the
  web-profile tiering and the arXiv state move.

### Tests and tooling

- **arXiv characterisation first.** Phase 2 pinned arXiv's queue behaviour
  in black-box CLI tests, including whole-line stderr assertions and a
  fixture captured from the previous release's queue state. Those tests
  then carried the engine refactor with only `support/` changes, plus one
  documented budget change.
- **Real-time tests.**
  - Multi-process rate and concurrency tests release a barrier of held
    tickets, then measure.
  - Rate and spread are taken from the gate's recorded sends; overlap and
    deferrals from the mock server's arrival times.
  - The tight liveness tests run alone with two nextest retries.
- **Parity.** An OpenAlex test pins identical `tier`, `venue` and
  `venue_signals` across `search` and every form of `lookup`.
- **Mock server.** `http-test-support` now answers `Delayed` routes
  concurrently; before, it held its routes lock while sleeping.
- **Skill parsing.** `skill_parsing.fenced_blocks` now returns each block's
  language tag, so `markdown` template blocks are not mistaken for commands.

## Context

- Work item: `meta/work/PP-891-tier-doi-links-by-venue-not-resolver.md`
  (its Baseline and Closing Run sections hold the manual evidence)
- Plan: `meta/plans/2026-11-21-PP-891-tier-doi-links-by-venue-not-resolver.md`
- Validation:
  `meta/validations/2026-11-21-PP-891-tier-doi-links-by-venue-not-resolver-validation.md`
  (result `pass`)
- Research:
  `meta/research/codebase/2026-11-20-PP-891-tier-doi-links-by-venue-not-resolver.md`

## Testing

- [x] At `8c177921` (before the review response), the bare `mise run`
      exits 0: 8479 of 8479 CLI tests, CLI line coverage 94.42%.
- [x] At `8c177921`, `mise run public-api:check`, `pup:check`,
      `docs:check` and `test:integration:release-guard` pass.
- [ ] CI on the current tip, which adds the review response: pending when
      this description was written. Not re-run locally.
- [x] `mise run test:integration:arxiv-queue-stress` (opt-in, real clock)
      passed in 117 s during phase 3.
- [x] Mutation check: narrowing the polite interval to 90 ms fails the
      polite rate test on every retry with 12 sends in a second.
- [x] Live checks:
  - `fetch doi metadata '10.1145/371920.372066'` printed the expected venue
    and publisher;
  - the DataCite DOI `10.48550/arXiv.1706.03762` printed empty metadata;
  - ten live calls in a loop drew no 429;
  - Crossref reported `x-api-pool` `polite-single` or `public-single` as
    expected.
- [x] Baseline run reproduced the bug (`doi.org` as the venue); the
      closing web-profile run recorded each ACM DOI's proceedings at
      tier-1, `arxiv.org` at tier-2 and `Nature` at tier-1, with no
      `doi.org` venue and no tier conflict.
- [ ] `/accelerator:configure` shows the `crossref` section in an installed
      plugin. Only the skill source and the generated docs were checked.

## Notes for Reviewers

- **Review 1.** `meta/reviews/prs/258-review-1.md` (verdict `COMMENT`). The
  commits after the first description address its untied-DOI and
  `E_DOI_TICKET_LIVE` findings, the missing changelog, the silent pool
  flip, the ports re-exported through `fetch`, the restated `Service` and
  queued families, and the untested ladder paths.
- **Serving slot during backoff.** Left as is: a Crossref call keeps its
  slot through the 3/6/12 s backoff, which can stall the single public slot
  for up to 21 s. `meta/notes/2026-11-23-release-the-serving-slot-during-backoff.md`
  records why and what data would justify changing it.
- **Where to start.**
  - The pacing domain: `cli/research/src/sources/pacing.rs`, the methods
    `reserve`, `confirm`, `release` and `trust`.
  - The file gate's reserve/confirm loop and companion locks:
    `cli/research-adapters/src/pacing.rs`.
  - Most of the remaining risk sits in these two files.
- **Upgrade behaviour.**
  - arXiv's state moves to `services/arxiv/`. The old `arxiv-pacing`,
    `arxiv-queue/` and logs are ignored, and `research.md` says they can be
    deleted.
  - A ticket printed by the previous release gets `E_ARXIV_TICKET_MISMATCH`.
    The profile's existing usage-error handling drops `--ticket` and
    re-runs.
- **Accepted risks.**
  - Old and new releases keep separate pacing state during the overlap.
  - Crossref limits apply per IP, but pacing is coordinated only per
    project.
  - Variation in connection setup can put limit + 1 arrivals into one
    second at Crossref, which the retry path absorbs.
- **Web-profile ordering.** The work item requires the resolver `WebFetch`
  to run before `fetch doi metadata`. In the closing run the researcher ran
  them the other way round; the tier did not depend on it. The closing run
  predates the tied-DOI rule, which no live run has exercised.
- **Releases.** Phases 4–7 should ship in one release: the command (phase 4)
  is unused until the profile text (phase 7) calls it.
- **Incidental changes.**
  - `.accelerator/state/integrations/linear/last-sync.json` is Linear sync
    state.
  - `meta/reviews/` holds the work-item and plan reviews.
