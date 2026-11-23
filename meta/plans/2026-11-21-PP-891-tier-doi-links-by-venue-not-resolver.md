---
type: "plan"
id: "2026-11-21-PP-891-tier-doi-links-by-venue-not-resolver"
title: "Tier DOI Links by Venue, Not by the doi.org Resolver Implementation Plan"
date: "2026-11-21T10:27:09+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "in-progress"
work_item_id: "work-item:PP-891"
parent: "work-item:PP-891"
derived_from: ["codebase-research:2026-11-20-PP-891-tier-doi-links-by-venue-not-resolver"]
relates_to: ["work-item:PP-864", "work-item:PP-865", "work-item:PP-888", "work-item:PP-892", "work-item:PP-897", "work-item:PP-936", "plan:2026-10-06-PP-882-fair-arxiv-fetch-queue"]
tags: ["research", "research-cli", "queue", "arxiv", "crossref", "openalex", "web-profile", "config"]
revision: "f2aa3b66cc9d73c448de8124189608fddb95e9a9"
repository: "accelerator"
last_updated: "2026-11-23T00:35:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Tier DOI Links by Venue, Not by the doi.org Resolver Implementation Plan

## Overview

Web-profile researchers tier a `https://doi.org/…` URL by its domain, so
peer-reviewed papers drop to tier-3 with venue `doi.org`. This plan makes four
changes:

1. It generalises the project-wide arXiv queue into an engine any service can
   use, with pacing set per service.
2. It adds `accelerator research fetch doi metadata '<DOI>'`, which reports the
   venue and publisher Crossref holds for a DOI and queues its calls on that
   engine.
3. It adds an opt-in `crossref.mailto` setting that moves the calls into
   Crossref's polite pool.
4. It teaches the web profile to tier any source cited by DOI by that venue,
   never by the resolver.

It also locks OpenAlex search/lookup parity in with a regression test. Two
manual runs bracket the code: a baseline that gates the building, and a closing
run that confirms the fix.

## Current State Analysis

The research
(`meta/research/codebase/2026-11-20-PP-891-tier-doi-links-by-venue-not-resolver.md`)
was verified against the code at this revision.

- **Queue policy is generic; queue mechanics are arXiv-shaped.**
  - These are service-neutral and stay as they are:
    - the ticket rules (`cli/research/src/sources/queue.rs:17-24, 258-294`);
    - the 100 s and 900 s bounds and the carried last-retryable reason;
    - `Attempts::until_settled` (`cli/research/src/sources/fetch.rs:579-634`).
  - These are arXiv-only:
    - `Binding` (`queue.rs:111-114`) and the `ArxivQueue` port (`queue.rs:412`);
    - the file names (`cli/research-adapters/src/pacing.rs:53-55`,
      `cli/research-adapters/src/queue.rs:52`);
    - the `E_ARXIV_TICKET_*` formatting
      (`cli/research-cli/src/render.rs:57-95`);
    - `parse_ticket` (`cli/research/src/sources/request.rs:123-134`).
- **Only one ticket can be served at a time.**
  - The front rule `is_front_given` (`queue.rs:345-358`) counts a ticket as
    front only when every ticket ahead of it has ended. Three calls can never
    be in flight at once.
  - `FilePacingGate` holds a single exclusive `flock` on `arxiv.lock` for a
    whole call (`pacing.rs:140-156`), and reads and writes its pacing state
    under that lock.
- **Spacing runs from the previous request's finish.**
  `PacingState::wait_at` (`pacing.rs:209-224`) waits from the last finish plus
  3 s, or from the last send plus 3.1 s when that send never finished.
  Crossref's limits are requests per second plus a concurrency limit, so they
  need spacing from the previous request's *start*.
- **An unknown ticket joins afresh** (`queue.rs:264-273`), and a ticket's text
  carries no family. Nothing can yet tell a Crossref ticket, or one from before
  the upgrade, from a ticket that was simply wiped.
- **`classify::arxiv` fails a 404 or other 4xx with exit 1**
  (`cli/research/src/sources/classify.rs:190-205`). Crossref needs:
  - 404 → `ok` with empty metadata;
  - any other 4xx except 429 → `unavailable`;
  - 429 → throttling.
- **`SourceMetadata` has no `record_publisher`**
  (`cli/research/src/sources/metadata.rs:46-68`). `publisher` sits at rank 10,
  after every key `web metadata` prints (ranks 0, 5, 6 and 7).
- **`Doi` already parses** bare, `doi:`, `doi.org` and `dx.doi.org` forms
  (`request.rs:355-409`), but its `parse` returns an `Option` with no error of
  its own.
- **`ConfigAccess` reads no environment variables**
  (`cli/config/src/service.rs:391-403`), and no existing key resolves as "first
  non-empty of env, `config.local.md`, `config.md`".
- **The test helpers centralise the state paths.** About 200 hard-coded tickets
  like `"1-aaaaaa"` exist across the tests, but all of them are seeded through
  `Project::queue_dir()` (`cli/research-cli/tests/support/mod.rs:179`). Roughly
  29 references to the arXiv state paths sit in test files, most in `support`
  modules.
- **The web profile** (`skills/research/profiles/web-profile/SKILL.md`) has no
  resolver rule, template, worked example, waiting handling or `E_*` handling.
  `tests/unit/tasks/test_research_structure.py:1784-1792` requires every fenced
  block in it to be a `web metadata` call.

## Desired End State

- `accelerator research fetch doi metadata '<DOI>'`:
  - prints `{"status":"ok","metadata":{"publisher":…},"venue":…}` for a journal
    or proceedings article;
  - prints `ok` without `venue` for any other Crossref type, and `ok` with
    empty metadata for a miss;
  - prints `unavailable` or `waiting` exactly as the work item specifies;
  - exits 2 only for `E_DOI_MALFORMED` and `E_DOI_TICKET_*`.
- arXiv and Crossref share one queue engine. Each service supplies its own
  pacing (interval, where that interval is spaced from, and its number of
  serving slots), its `source` and its ticket family.
- arXiv's observable behaviour is unchanged, except that its state lives at new
  paths. A ticket from before the upgrade, or one from another family, gets
  `*_TICKET_MISMATCH`.
- `crossref.mailto` selects the polite pool (10/s, 3 concurrent). Otherwise
  calls use the public pool (5/s, 1 concurrent). The two pools are queued and
  paced independently.
- The web profile tiers every source cited by DOI by its Crossref venue, then
  its publisher, then its landing host, then no venue at tier-3. It never
  tiers by `doi.org`. The finding outputter allows the combined block.
- A regression test pins identical `tier`, `venue` and `venue_signals` across
  OpenAlex search and every form of lookup.
- `mise run` exits 0. The closing manual run shows no `doi.org` venue and no
  tier conflict for the three checked DOIs.

### Key Discoveries

- The serving window comes only from `PacingGate::spacing()` (`fetch.rs:377`,
  `fetch.rs:589`). A per-service pacing value therefore reaches admission and
  retries without changing their arithmetic.
- `Attempts::deferral` shares a deferral only for a throttling verdict
  (`fetch.rs:636-648`). Classifying Crossref's 429 as
  `Retry { RateLimited }` is enough for every caller to honour its
  `Retry-After`.
- `http_test_support::MockHTTPServer` already exposes `queries`, `last_header`
  and `hit_instants` (`cli/http-test-support/src/lib.rs:242-312`). The "no
  query parameters" and header criteria need no harness change.
- `ACCELERATOR_RESEARCH_TEST_CLOCK_LOG`, `…_CLOCK_EPOCH` and
  `…_CALL_BUDGET_MS` (`cli/research-cli/src/loopback.rs:194-247`) cover every
  single-process clock criterion. The multi-process criteria run in real time,
  with request starts taken from the gate's request log.
- `cli/pup.ron:95-114` keeps serde out of `research`, so the Crossref JSON
  decoder lives in `research-adapters`.

## What We're NOT Doing

- Row 1 of the work item's Context table (`10.1109/icdcsw.2005.79`, "tier-3, no
  venue").
- Any change to `fetch web metadata` or its output.
- Any OpenAlex call from `doi metadata`, and any tier in its output.
- Migrating old queue state or tickets. They are invalidated, as the item
  requires.
- Moving `arxiv-withdrawals.json`. It is a confirmation cache, not queue or
  pacing state, and it is valid across releases.
- PP-888's remaining follow-ups, apart from what this plan needs: per-service
  pacing replaces its `ServingWindow` question, and the queue engine's generic
  outcome answers its `FetchOutcome` question for Crossref.
- PP-897's domain classes. Whichever lands second reconciles the Reputation
  Tiers text.
- A Crossref Plus token, or a `mailto` query parameter.

## Implementation Approach

**Phase order.** The order follows the work item's:

1. baseline;
2. arXiv characterisation;
3. engine generalisation;
4. the anonymous-pool command;
5. the polite pool;
6. parity;
7. profile text;
8. closing run.

Every phase can be merged on its own. Phase 4 ships a command nothing calls
yet, and phase 7 ships the profile text that calls it. Releases are cut from
`main`, so phases 4 and 7 reach users in the same release only if the
implementer does not release between them. Release phases 4–7 together.

**Agreed design choices.**

- **Ticket provenance by directory.** A ticket's text stays `<n>-<nonce>`.
  Each service queue keeps its own directory. A presented ticket the queue
  cannot find in its own directory is looked up, read-only, in two places:
  - the other families' queue directories → `IssuedElsewhere`;
  - the previous release's `arxiv-queue/` → `IssuedBeforeUpgrade`.

  Either one is reported as `E_<FAMILY>_TICKET_MISMATCH`, using the prefix of
  the command it was presented to. A ticket found nowhere joins, as today.
- **Move all queue and pacing state** to `<paths.tmp>/research/services/<service>/`.
  This covers arXiv's lock, pacing, request log, contention log and ticket
  directory. The new release never reads or writes the old release's pacing
  or queue contents. It only lists ticket names in the old `arxiv-queue/`
  for provenance. While serving, the arXiv gate also takes the legacy
  `<paths.tmp>/research/arxiv.lock` exclusively, as a lock only, so
  processes of both releases still keep one arXiv connection at a time
  during the upgrade overlap. `main.rs` passes that lock to the arXiv gate
  as a companion lock; the gate itself knows no service by name. The
  companion is opened or created, as the old release opens it, so the two
  releases can never race on its first creation; `arxiv.lock` therefore
  persists until the companion is retired. Their pacing state is not
  shared, an accepted risk.
- **Multi-process timing criteria run in real time.** Request starts and
  in-flight overlap are read from the mock server's `hit_instants`, against a
  fixed response delay. The service's `requests.log` is used only to show
  which pool served a call. Every multi-process test first seeds and holds
  one live ticket per serving slot (`service.pacing().slots()`) at the front
  of each service's `queue/` as a barrier, launches
  its invocations, waits until all their ticket files exist, then releases
  the barrier and measures from the first hit after the release, so process
  start-up and joining fall outside the measured window. The
  "project-scoped test clock" is the real clock shared by processes in one
  project. Exact reservation arithmetic is pinned separately, under a fake
  clock, in adapter tests.
- **Services and pools.** `Service` is `Arxiv | Crossref(CrossrefPool)`, with
  `CrossrefPool` `Public | Polite`. The two pools share one `source` and one
  ticket family, and each has its own pacing and state directory. Phase 3
  adds all three variants, with their pacing and directory code only, so
  the start-spaced and three-slot behaviour is test-driven where it is
  built; phases 4 and 5 wire the Crossref variants to commands.
- **Pacing is a domain value, with one source.** It is
  `Pacing { interval, spaced_from }`, where `spaced_from` is
  `PreviousFinish` or `PreviousStart { slots }`. `PreviousFinish` always
  serves one call at a time, so pacing it across several slots cannot be
  written down. `Service::pacing()` is the only place the values live,
  apart from `Pacing::UNPACED`, which OpenAlex's `NoPacing` gate returns, and
  `Service::code()` the only place the directory name lives. Gates, queues
  and slot locks derive both from the `Service` they are built with.
  - arXiv: 3 s from the previous finish, so the same rule as today;
  - Crossref public: 210 ms from the previous start, 1 slot;
  - Crossref polite: 102 ms from the previous start, 3 slots.

  10 calls can still start inside 2 s (1.89 s) and 1 s (0.918 s). Spacing
  is guaranteed between recorded sends. When requests reach Crossref also
  depends on each request's setup latency (DNS, TCP and TLS in a fresh
  process), which the margins over 200 ms and 100 ms absorb only up to
  50 ms (public) and 20 ms (polite) of variation across a window. Larger
  variation can occasionally put limit + 1 requests into one second at
  Crossref, drawing a 429 the retry path absorbs; this is an accepted risk,
  since the work item's 10-in-1 s criterion confines the polite interval to
  (100, 111] ms. The liveness budget is 110 ms (public) and 82 ms (polite)
  across nine gaps, about 12 ms and 9 ms of hand-off overhead per gap.
- **Reservations and sends are kept apart, and every send is confirmed.**
  The pacing state holds the last actual send (`last_sent`) separately from
  the next free reservation (`next_free`). A caller reserves a send time
  under `pacing.lock`, which advances `next_free`, then releases it and
  sleeps. On waking it takes `pacing.lock` again and confirms the send
  against `last_sent + interval` and `not_before` only, never against
  reservations, in this order:
  1. a deferral covers `now` and began after its reserved time → it gives up
     its reservation and reserves again;
  2. spacing or a deferral still forbids `now` → it keeps its reservation
     and waits until both allow it;
  3. otherwise → it records `last_sent = now` and sends.

  The deadline is checked before every reservation and every wait. A call
  that gives up hands back its reservation when it is still the latest.
  Spacing is therefore guaranteed between actual sends, a lone caller that
  wakes on time always sends, and a `Retry-After` written while a caller
  sleeps is honoured by every slot.
- **The admission poll follows pacing.** A waiting caller re-checks admission
  every `interval / 4`, truncated to whole milliseconds and kept between
  10 ms and 100 ms. arXiv keeps 100 ms. Crossref waiters re-check every 52 ms
  (public) and 25 ms (polite).
- **The front rule counts slots.** A ticket is served while fewer than `slots`
  unended tickets are ahead of it. With one slot this is exactly today's rule.
- **One queued outcome for every family.** The engine returns `Queued<T>`, and
  every `unavailable` lives in `Queued::Unavailable`, whether the queue or
  the upstream caused it. arXiv returns `Queued<ArxivAnswer>`, where
  `ArxivAnswer` is `Records | Failed`, and DOI returns `Queued<DoiMetadata>`.
  `FetchOutcome` loses its `Waiting` and `Rejected` variants, so both
  families render queue states through one path.
- **`Binding` stays a closed enum.** Each new service adds a variant to
  `Binding` and to `StoredBinding`. A third queued service is the point at
  which to revisit this.

---

## Phase 1: Baseline Run and Work-Item Corrections

### Overview

This is a manual gate. Nothing is built until it passes. It confirms or refutes
the `doi.org` cause, and records what the closing run needs.

### Changes Required

#### 1. Baseline run

**Evidence**:
- The baseline subject's notes and dossier, kept as written by
  `research-topic` under `meta/research/topics/<slug>/`.
- `meta/work/PP-891-tier-doi-links-by-venue-not-resolver.md`, gaining a
  `## Baseline` section.

**Steps**:
1. On the current plugin, create a `research-topic` subject whose brief gives
   `https://doi.org/10.1145/371920.372066` and
   `https://doi.org/10.1145/3448016.3457556` as sources to cite. Outline it
   with the web profile only, then run `conduct`.
2. Judge the run:
   - It counts only if both URLs appear as cited web-profile entries.
   - Otherwise it is inconclusive and is re-run. After three inconclusive
     runs, stop and revisit the brief with the item's author.
3. If a counting run shows neither `doi.org` as a venue nor a tier below
   tier-1, stop and revisit the cause with the item's author.
4. If it reproduces the bug:
   - Replace the item's provisional reproduction with the brief and the
     observed venue and tier, as the confirmed reproduction.
   - Have the author sign off in the item.
5. In the same session, run `WebFetch` on each resolver URL and record that it
   returns a cross-host redirect naming the landing host. Record the Claude
   Code version. Repeat on v2.1.144 and record any gap. If v2.1.144 follows
   the redirect instead of reporting it, phase 7's profile text states that a
   `WebFetch` returning content without a reported redirect gives no landing
   host, so the source falls to the no-venue, tier-3 branch.
6. Choose a non-resolver page that declares a DOI Crossref holds as a
   `journal-article` or `proceedings-article`, for example an ACM or IEEE
   landing page that `fetch web metadata` can read. Record:
   - its URL;
   - the DOI it declares;
   - the DOI's Crossref `container-title[0]` (from
     `curl -s https://api.crossref.org/works/<doi>`);
   - the expected tier, tier-1.

#### 2. Work-item corrections

**File**: `meta/work/PP-891-tier-doi-links-by-venue-not-resolver.md`
**Changes**:
- Context: replace "The profiles say 'Never re-judge a tier'" with what
  actually carries the tier forward: the composers carry each note's tier into
  the finding.
- The parity acceptance criterion and requirement:
  - name the endpoint `search` actually uses: the `/works` search endpoint,
    with `title_and_abstract.search`, not `filter=doi:`;
  - place the test in `cli/research-cli/tests/fetch_openalex.rs`.
- Requirements: record the design choices this plan settles:
  - ticket provenance by directory;
  - all arXiv queue and pacing state moved;
  - real-time multi-process timing tests;
  - a Crossref 3xx or undecodable body → `unavailable`/`upstream_error`;
  - a ticket re-presented after its pool changed joins the new pool afresh;
  - a DOI ticket matches its call whatever the DOI's letter case;
  - rewrite the existing plausibility sentence in place, rather than adding
    a second rule: a plausible address, once trimmed, has only visible
    ASCII with no `(`, `)` or `\` (so it can sit in a `User-Agent`
    comment), exactly one `@`, a non-empty local part containing no `:`,
    and a domain containing a dot that neither starts nor ends with one;
  - the arXiv gate holds the legacy `arxiv.lock` while serving, so old and new
    releases keep one arXiv connection at a time;
  - the web profile repeats a CLI `warning:` line once in the researcher's
    summary, and `conduct` lists each distinct warning once in its final
    summary.
- Requirements: rewrite "never read each other's state" as: the new release
  never reads or writes the previous release's pacing or queue contents; it
  only lists ticket names in the previous `arxiv-queue/` for provenance and
  takes the previous `arxiv.lock` as a lock.

### Outcome

The work item's `## Baseline` section holds the full record. In brief:

- **Two counting runs**, on Claude Code `2.1.330` and plugin
  `1.24.0-pre.105`:
  - `meta/research/topics/pp-891-doi-resolver-baseline/`, whose brief
    described each paper's subject;
  - `meta/research/topics/pp-891-doi-resolver-baseline-2/`, re-run at the
    author's direction with a brief naming only the two URLs.

  Each set holds its notes only. No dossier was synthesised: each set has
  a single finding, so a dossier would add no evidence.
- **Result**: both runs record `doi.org` in each resolver URL's venue, at
  tier-1, not tier-3. The tier-1 rests on a Crossref lookup the researcher
  improvised with `WebFetch` and cited as its own tier-1 source. The author
  signed this off as the confirmed reproduction.
- **`fetch web metadata`** fails on both resolver URLs with
  `E_WEB_STATUS: the page answered HTTP 403` from `dl.acm.org`, so phase 8
  is expected to take the failed-`web metadata` branch for both.
- **`WebFetch`** reports a cross-host `302 Found` to `dl.acm.org` for both
  resolver URLs on `2.1.330` and on `2.1.144`. There is no gap, so phase 7
  adds no no-redirect sentence.
- **Non-resolver page**: `https://www.nature.com/articles/nature14539`,
  declaring `10.1038/nature14539`, a Crossref `journal-article` with
  `container-title[0]` `Nature`; expected tier-1.
- **Template ids**: the brief and outline templates' `id` (the bare
  filename stem) collided across the two sets, so each set's brief and
  outline ids carry the set slug as a prefix.

### Success Criteria

#### Automated Verification

- [x] The work item still validates:
      `accelerator corpus frontmatter validate --file meta/work/PP-891-tier-doi-links-by-venue-not-resolver.md`

#### Manual Verification

- [x] A counting baseline is recorded, with the item author's sign-off.
- [x] The `WebFetch` redirect behaviour and the Claude Code versions are
      recorded.
- [x] The non-resolver page, its DOI, its Crossref venue and the expected tier
      are recorded.

---

## Phase 2: arXiv Queue Characterisation

### Overview

Before the refactor, pin every arXiv queue behaviour the item lists in
black-box CLI tests, so phase 3 can be judged by unchanged CLI test bodies.
Domain and adapter tests in the table are supporting evidence. Phase 3 renames
or re-expresses some of them, and every mapped behaviour must also have a CLI
test. Route every state path in the tests through `support` helpers. Capture
the old release's queue state as a fixture.

### Behaviour → test mapping

`C` = `cli/research-cli/tests/`, `A` = `cli/research-adapters/tests/`,
`R` = `cli/research/tests/`. Tests marked **new** are written in this phase.

| Behaviour | Covering test(s) |
|---|---|
| Pacing: 3 s after the previous finish, across processes | `C/arxiv_pacing.rs` `a_second_process_waits_for_the_first_processs_response_to_finish`, `sequential_calls_are_spaced_three_seconds_apart`; `A/pacing.rs` spacing tests |
| Pacing: one request at a time | `C/arxiv_pacing.rs:121, 153`; `C/fetch_arxiv.rs:362` |
| Retry-After shared across processes | `C/fetch_arxiv.rs` `a_throttled_call_defers_the_next_calls_first_request`; **new** `C/arxiv_pacing.rs` `a_throttled_call_defers_a_concurrent_process` |
| `waiting` and `--ticket`, position | `C/fetch_arxiv.rs:405, 451, 473, 486, 505, 769`; `C/arxiv_queue.rs:114, 134, 158` |
| 100 s invocation bound | `R/arxiv_fetch.rs:548-626`; **new** `C/fetch_arxiv.rs` `a_call_queued_behind_a_live_ticket_returns_waiting_within_100_s` (LoggingClock, production budget) |
| `unavailable` `upstream_error` | `C/fetch_arxiv.rs:182`; **new** `C/fetch_arxiv.rs` `a_query_answered_503_until_retries_run_out_is_unavailable_upstream_error` |
| `unavailable` `rate_limited` | `C/fetch_arxiv.rs:314` |
| `unavailable` `rate_limited` + `lock_contention` | `C/fetch_arxiv.rs:723, 855` |
| Last retry's reason carried past the cap | `C/fetch_arxiv.rs:874` (`upstream_error`); **new** `C/fetch_arxiv.rs` `a_ticket_whose_last_attempt_was_throttled_reports_rate_limited_past_its_cap` |
| Ticket cap (900 s) | `C/fetch_arxiv.rs:707, 723`; `R/queue.rs:383, 393, 421` |
| `E_ARXIV_TICKET_MALFORMED`, exit 2 | `C/fetch_arxiv.rs:785`; **new** `C/fetch_arxiv.rs` `a_malformed_ticket_is_reported_as_exactly_one_line` |
| `E_ARXIV_TICKET_MISMATCH`, exit 2 | `C/fetch_arxiv.rs:548, 605, 620, 632`; **new** `C/fetch_arxiv.rs` `a_mismatch_is_reported_as_exactly_one_line_naming_the_issued_call` |
| `E_ARXIV_TICKET_LIVE`, exit 2 | `C/fetch_arxiv.rs:817`; **new** `C/fetch_arxiv.rs` `a_live_ticket_is_reported_as_exactly_one_line` |
| 404 / other 4xx exit 1 | **new** `C/fetch_arxiv.rs` `a_lookup_answered_404_fails_with_a_client_error` and `a_lookup_answered_400_fails_with_a_client_error` |
| Whole stderr summary lines | **new** `C/fetch_arxiv.rs`: `waiting` in `a_call_queued_behind_a_live_ticket_returns_waiting_within_100_s`; `unavailable (…) after N attempts` in `a_query_answered_503_until_retries_run_out_is_unavailable_upstream_error`; `failed after …` in the 404 and 400 tests |

The implementer re-checks the line references at the start of the phase and
updates this table if they have moved.

### Changes Required

#### 1. Path helpers

**Files**: `cli/research-cli/tests/support/mod.rs`,
`cli/research-adapters/tests/support/mod.rs`, and every test file that names
`arxiv.lock`, `arxiv-pacing`, `arxiv-requests.log`, `arxiv-contention.log` or
`arxiv-queue`. Those files are `arxiv_queue.rs`, `arxiv_queue_stress.rs`,
`fetch_openalex.rs`, `fetch_arxiv.rs`, `A/scratch.rs`, `A/contention.rs`,
`A/pacing.rs` and `A/queue.rs`.
**Changes**: add `Project::arxiv_state(StateFile)` (and an adapter-side
equivalent) returning each path, and replace every literal path in test bodies
with it. Rename the scratch-only names that merely resemble state paths
(`arxiv-queue-moved` at `A/queue.rs:947`, `nested("arxiv-queue")` at
`A/scratch.rs:25, 45`) to neutral literals. This is a refactor and must stay
green.

#### 1a. Constructor and state-writer helpers

**Files**: `cli/research/tests/support/mod.rs`,
`cli/research-adapters/tests/support/mod.rs`, and the test bodies that call
the constructors directly (`R/queue.rs:132, 511`, `R/arxiv_fetch.rs:90`,
`A/pacing.rs:42`, `A/contention.rs:22`, `A/queue.rs:77, 634, 1007`).
**Changes**: move every direct construction of `Queue`, the fake gate's
builder (including `.spacing(SPACING)`), `FilePacingGate`,
`FileContentionLog` and `FileArxivQueue` into named `support` helpers; route
every `queue.join(…, SPACING)` call in `A/queue.rs` (the `join_as` helper at
`:155-160` and the inline calls at `:302-307`, `:664-669`, `:699-704`,
`:895`, `:1013-1018`) through a `support` join helper; and move the
hand-written pacing-file JSON in `A/pacing.rs` into a `support` writer.
Phase 3 then changes those signatures and the file shape only inside
`support/`. This is a refactor and must stay green.

#### 2. New characterisation tests

**Files**: `cli/research-cli/tests/fetch_arxiv.rs`,
`cli/research-cli/tests/arxiv_pacing.rs`
**Changes**: the tests marked **new** above. Each must pass against the current
code. Any that fails has found a behaviour the item claims but the code lacks;
stop and raise it. Where the table names only a domain or adapter test for a
behaviour, add a CLI test for it too.

Phase 3 claims arXiv's output stays byte-identical, but today's tests match
stderr only with `contains` or `starts_with`. Add CLI tests that assert full
line equality for every arXiv stderr line:
- the `E_ARXIV_TICKET_MISMATCH`, `E_ARXIV_TICKET_MALFORMED` and
  `E_ARXIV_TICKET_LIVE` lines;
- the `waiting` summary line;
- the `unavailable (…) after N attempts` and `failed after …` summary lines.

#### 3. Legacy state fixture

**Files**:
- `cli/research-cli/tests/fixtures/legacy-arxiv-queue/` (new): the captured
  `arxiv-queue/` directory, `arxiv-pacing` and `ticket.txt`;
- `cli/research-cli/tests/fixtures/legacy-arxiv-queue/README.md`, giving the
  release it came from and how it was captured.

**Capture**:
1. In a scratch project, use the current `accelerator-research` binary built
   with `test-loopback`, pointed at a mock arXiv that answers `503` with
   `Retry-After: 30`. A stalling mock with a budget short enough to return
   `waiting` never sends, so it leaves no `arxiv-pacing` to capture.
2. Make one arXiv lookup with a 35 s call budget
   (`ACCELERATOR_RESEARCH_TEST_CALL_BUDGET_MS`): it sends once, cannot fit
   the retry, and returns `waiting`.
3. Copy `<paths.tmp>/research/arxiv-queue/` and `arxiv-pacing` into the
   fixture, and save the printed ticket in `ticket.txt`.
4. Delete the `.lock` files: a live lock cannot be captured.

### Outcome

Every new test passed against the code as it stood, so none found a
behaviour the item claims but the code lacks. In brief:

- **State path helpers** phase 3 retargets:
  - `StateFile` (`ServingLock`, `Pacing`, `RequestLog`, `ContentionLog`,
    `Queue`, and in the CLI support also `QueueLock`);
  - `Project::arxiv_state` and `Project::hold(StateFile)` in
    `cli/research-cli/tests/support/mod.rs`;
  - `Scratch::arxiv_state` in `cli/research-adapters/tests/support/mod.rs`.
    Its file helpers (`write`, `read`, `mkdir`, `open`, `lines`) accept
    either a name or the path that `arxiv_state` returns.
- **Constructor and state-writer helpers**:
  - adapter support: `arxiv_gate`, `arxiv_contention_log`, `arxiv_queue`,
    `join_queue` (the only place arXiv's spacing reaches `join`),
    `Scratch::write_pacing(&PacingRecord)`,
    `Scratch::write_pacing_without_last_sent` (used only by the test phase 3
    retires) and `Scratch::stored_pacing`;
  - domain support: `queue_of` and `RecordingGate::paced_as_arxiv`.
- **New CLI tests**: as named in the mapping table, plus whole-line stderr
  tests `a_mismatch_is_reported_as_exactly_one_line_naming_the_issued_call`,
  `a_malformed_ticket_is_reported_as_exactly_one_line` and
  `a_live_ticket_is_reported_as_exactly_one_line`.
  `a_throttled_call_defers_a_concurrent_process` takes about 7 s in real
  time and passed 6 runs out of 6.
- **Legacy fixture**: captured on plugin `1.24.0-pre.105`. It holds the ended
  ticket `1-e85851` for `lookup 2608.21129`, with `last_retryable`
  `upstream_error`, and an `arxiv-pacing` file with `last_sent_ms` and
  `last_finish_ms` set and `not_before_ms` null, because a 503's
  `Retry-After` is not shared with other calls. `ticket.txt` holds the
  ticket.
- **Plan corrections**: the CLI crate's package is `accelerator-research`,
  not `research-cli`. Every phase's test commands now use that name.
- **`mise run check`**: `deny:check` failed, on trunk as well, because of
  RUSTSEC-2026-0592 against `url 2.5.8`. Updating to `url 2.5.9` cleared
  it, and `mise run check` now exits 0.

### Success Criteria

#### Automated Verification

- [x] The CLI tests pass:
      `cd cli && cargo nextest run -p research -p research-adapters -p accelerator-research --features accelerator-research/test-loopback`
- [x] No test body outside `support` names an arXiv state file:
      `rg -n '["/]arxiv(\.lock|-pacing|-requests\.log|-contention\.log|-queue/?")' cli/research-cli/tests cli/research-adapters/tests --glob '!**/support/**' --glob '!**/fixtures/**'`
      prints nothing.
- [x] `mise run cli:check` passes.

#### Manual Verification

- [x] The mapping table above is current, with every **new** test named as
      written.

---

## Phase 3: Generalise the Queue Engine

### Overview

Turn the arXiv queue into an engine any service can use, with arXiv as its only
consumer. Behaviour stays identical except for the state paths and the
provenance mismatches. Write a failing test first for each new behaviour:
- the N-slot front rule;
- start-spaced pacing, the confirmation before each send, reservation
  hand-back, and the `trust` repairs;
- the pacing-derived admission poll;
- the multi-slot file gate;
- `IssuedElsewhere` and `IssuedBeforeUpgrade`;
- the legacy fixture and the companion `arxiv.lock`.

The phase 2 CLI tests must pass with only their `support` helpers changed.
Domain and adapter tests may change only by the renames listed under
Success Criteria. The `A/pacing.rs` wait-rule cases stay unchanged as gate
characterisation, and their pure equivalents in `R/pacing.rs` are new tests.
One test is retired deliberately:
`a_state_written_before_last_sent_was_recorded_still_spaces_the_request`
pins a pacing-file shape from before `last_sent` existed, which the new
`services/arxiv/pacing` file, always written with `schema_version`, can
never have.

### Changes Required

#### 1. Domain: services and pacing

**File**: `cli/research/src/sources/service.rs` (new)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrossrefPool {
    Public,
    Polite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Service {
    Arxiv,
    Crossref(CrossrefPool),
}

impl Service {
    pub const ALL: [Self; 3] = [
        Self::Arxiv,
        Self::Crossref(CrossrefPool::Public),
        Self::Crossref(CrossrefPool::Polite),
    ];

    pub const fn pacing(self) -> Pacing { … }
    pub const fn source(self) -> &'static str { … }
    pub const fn ticket_family(self) -> Family { … }
    pub const fn code(self) -> &'static str { … }
}
```

Phase 3 adds all three variants with their pacing and code, so the
start-spaced and three-slot tests can name real services: `Arxiv` (3 s from
the previous finish, `arxiv`), `Crossref(Public)` (210 ms from the previous
start, 1 slot, `crossref-public`) and `Crossref(Polite)` (102 ms from the
previous start, 3 slots, `crossref-polite`). Nothing calls the Crossref
variants until phases 4 and 5. Tests (`R/service.rs`): each variant's pacing,
code, `source` and ticket family.

The Crossref variants' ticket family is `Family::Doi`, so phase 3 also adds
that variant to `Family` in `request.rs`, with its code `doi` and Display
`DOI`, for ticket provenance and rejection prefixes only. It stays out of
`Family::ALL`, `Family::parse` and `parse_ticket` until phase 4 wires the
command, so the CLI accepts no `doi` family and no family list or usage
message changes in phase 3. Tests (`R/request.rs`): `Family::Doi`'s code
and Display; `Family::parse("doi")` is still unknown.

**File**: `cli/research/src/sources/pacing.rs` (new). The pure pacing arithmetic
moves here from `research-adapters/src/pacing.rs:190-251`, generalised:

```rust
pub enum SpacedFrom {
    PreviousFinish,
    PreviousStart { slots: NonZeroU8 },
}

pub struct Pacing {
    interval: Duration,
    spaced_from: SpacedFrom,
}

impl Pacing {
    pub const UNPACED: Self;
    pub const fn slots(&self) -> NonZeroU8;
    pub fn admission_poll(&self) -> Duration;
}

pub struct PacingState { last_sent, next_free, last_finish, not_before }

pub struct Reservation { send_at: SystemTime }

pub enum Confirmation {
    Send(PacingState),
    WaitUntil(SystemTime),
    Deferred(PacingState),
}

impl PacingState {
    pub fn reserve(self, pacing: &Pacing, now: SystemTime)
        -> (Reservation, PacingState);
    pub fn confirm(self, pacing: &Pacing, reservation: &Reservation,
        now: SystemTime) -> Confirmation;
    pub fn release(self, pacing: &Pacing, reservation: &Reservation) -> Self;
    pub fn finish(self, at: SystemTime) -> Self;
    pub fn defer(self, until: SystemTime, now: SystemTime) -> Self;
    pub fn trust(self, pacing: &Pacing, now: SystemTime) -> Self;
}
```

These methods hold the whole policy, so the file gate never branches on
`SpacedFrom` and never decides on its own when to hand a reservation back.
Every method takes the state and returns the next one, named as a verb.
- `last_sent` is only ever an actual send, written by `confirm`. `next_free`
  is the reservation cursor, used only by `PreviousStart`.
- `reserve`:
  - `PreviousStart` sends at
    `max(now, next_free, last_sent + interval, not_before)`, truncated to
    whole milliseconds so the `Reservation` and the persisted `next_free_ms`
    share one resolution, and advances `next_free` to that time plus the
    interval;
  - `PreviousFinish` keeps today's rule exactly, including `SEND_MARGIN` for
    a send that has no finish, and leaves the state unchanged.
- `confirm` never reads `next_free`, so a caller is never held back by its
  own or a later reservation. It decides by explicit precedence:
  1. `not_before > now` and `not_before > reservation.send_at` →
     `Deferred`, carrying the already-released state;
  2. `max(last_sent + interval, not_before) > now` (or today's finish rule
     for `PreviousFinish`) → `WaitUntil` that time, keeping the reservation;
  3. otherwise → `Send`, with `last_sent = now`.

  A caller that wakes late after a deferral has already ended therefore
  sends at once, and one woken early by a clock step back waits.
- `release` hands a reservation back: it rolls `next_free` back by one
  interval when the reservation is still the latest, and otherwise leaves
  the state as it is.
- `trust` repairs values only a clock step or a corrupt file can produce:
  - a deferral beyond the 30 s ceiling is dropped, as today;
  - a `last_sent` or `last_finish` later than `now` is clamped to `now`,
    keeping their order: when the finish preceded the send, the finish is
    treated as absent, so the unfinished-send rule still applies. After a
    clock step back arXiv therefore waits exactly its 3 s (or 3.1 s for a
    send with no finish), as today;
  - a `next_free` more than `slots × interval` beyond
    `max(now, not_before, last_sent + interval)` is clamped to that bound.
    The bound exists only to free a pool wedged by a clock step; `confirm`
    guarantees spacing on its own, so a clamp that lands below a legitimate
    reservation only makes two reservations collide, which `WaitUntil`
    resolves.
- `admission_poll` is `interval / 4`, truncated to whole milliseconds and
  clamped to 10–100 ms. `UNPACED` has a zero interval, is returned only by
  OpenAlex's `NoPacing` gate, and so polls every 10 ms rather than spinning.
- `PacingGate::spacing()` becomes `PacingGate::pacing() -> Pacing`. The serving
  window (`fetch.rs:377, 589`) uses `pacing.interval()`, and `admitted` polls
  every `pacing.admission_poll()` instead of the fixed `ADMISSION_POLL`.
- **Tests** (`cli/research/tests/pacing.rs`, new):
  - Every `A/pacing.rs` wait-rule case stays in place, unchanged, as gate
    characterisation, including the two clock-step cases
    (`a_last_finish_in_the_future_waits_at_most_three_seconds`,
    `a_send_stamped_ahead_of_now_waits_no_more_than_one_spaced_send`),
    which exercise `trust` across `reserve` and `confirm`. Each spacing,
    unfinished-send and deferral-ordering case also gets a pure equivalent
    here for `PreviousFinish`, as a new test with the same seeded state and
    expected wait, asserted on the `Reservation` and `Confirmation`. The
    implementer lists the copied cases by name at the start of the phase.
  - `PreviousStart` spaces sends by the interval whenever finishes come.
  - A deferral outranks spacing.
  - One caller reserves, wakes at its send time and gets `Send`.
  - Three back-to-back reservations confirm in turn and all send within
    three intervals of the first.
  - An earlier sender that was 20 ms late makes the next `confirm` give
    `WaitUntil` 20 ms later, keeping the reservation.
  - A deferral written after the reservation gives `Deferred` with the
    reservation already released.
  - `confirm` precedence: a deferral that began after the reservation but
    has ended by `now` gives `Send`; a deferral covering `now` but beginning
    before the reservation gives `WaitUntil`; spacing and a deferral both
    forbidding `now` give `WaitUntil` at the later of the two.
  - `release` rolls `next_free` back only while the reservation is the
    latest, including when `now` had a sub-millisecond part at reservation.
  - `trust`: a deferral beyond 30 s is dropped; a future `last_finish`
    gives exactly a 3 s wait and a future `last_sent` with no finish exactly
    3.1 s; both stamps in the future with the finish before the send give
    exactly 3.1 s; with three slots, a `next_free` exactly
    `slots × interval` beyond the bound's base is kept and one just beyond
    is clamped to it;
    reservations chained after a 2 s deferral are kept.
  - `admission_poll` is 100 ms for arXiv, 52 ms for 210 ms, 25 ms for
    102 ms and 10 ms for `UNPACED`.

#### 2. Domain: queue generalised

**File**: `cli/research/src/sources/queue.rs`

- **`Binding`** gains per-service variants:

  ```rust
  pub enum Binding {
      ArxivSearch { query: ArxivQuery, limit: Limit },
      ArxivLookup(ArxivId),
  }
  ```

  Phase 4 adds `DoiMetadata(Doi)`. `differing` reports `Argument::Verb` for
  any pair of differing variants.
- **The front rule counts slots:** `is_front_given(ahead, now)` becomes
  `is_served_given(ahead, slots, now)`. It is true while fewer than `slots`
  contenders ahead have not ended.
  - Tests: one slot matches every existing `is_front_given` case; with three
    slots, the third unended ticket ahead blocks and two do not; ended
    tickets mixed among unended ones ahead never count, for one slot and for
    three.
- **Provenance** feeds `Queue`:

  ```rust
  pub enum Provenance {
      IssuedElsewhere(Family),
      IssuedBeforeUpgrade,
  }
  ```

  `Queue::new(standings, unparsed, foreign: Vec<(Ticket, Provenance)>)`.
  `present` checks `foreign` only when the ticket is absent from `standings`,
  so its own tickets always win:

  ```rust
  pub enum TicketRejection {
      Mismatch { ticket, issued, differing },
      Foreign { ticket, provenance: Provenance },
      AlreadyLive(Ticket),
  }
  ```

  Tests: a foreign ticket is rejected whether or not it has expired; an own
  ticket with the same text resumes; an unknown ticket joins.
- **`ArxivQueue` → `ServiceQueue`**, with
  `join(binding, presented, deadline)`. The queue reads its slot count from
  the `Service` it was built with.
- **Rejection lines become a domain `Display`:**
  `RejectedTicket { presented_to: Family, rejection }`. The text moves from
  `render.rs:57-95`, and the prefix comes from `presented_to`:
  - `E_ARXIV_TICKET_MISMATCH: ticket … was issued for <call> (differing: …)`
    — unchanged text;
  - `E_<FAMILY>_TICKET_MISMATCH: ticket … was issued by a <family> fetch; drop
    --ticket and re-run the call`;
  - `E_<FAMILY>_TICKET_MISMATCH: ticket … was issued before this release;
    drop --ticket and re-run the call`.

  `RequestError::TicketMalformed` carries the `Family`, so its prefix is per
  family. The `Display` matches on `provenance` for the two foreign texts.

#### 3. Domain: the queue engine

**File**: `cli/research/src/sources/serving.rs` (new). It holds the queue path
that today sits inside `fetch_arxiv` (`fetch.rs:299-348, 352-364, 366-419`) and
`Attempts` (`fetch.rs:566-674`):

```rust
pub struct QueuePorts<'a> {
    pub transport: &'a dyn Transport,
    pub clock: &'a dyn Clock,
    pub gate: &'a dyn PacingGate,
    pub queue: &'a dyn ServiceQueue,
    pub contention: &'a dyn ContentionLog,
}

pub enum Queued<T> {
    Served(T),
    Unavailable(Unavailable),
    Waiting { ticket: Ticket, position: Position },
    Rejected(TicketRejection),
}

pub fn queued<T>(
    binding: &Binding,
    presented: Option<&Ticket>,
    ports: &QueuePorts<'_>,
    deadline: &Deadline,
    serve: &mut dyn FnMut(&Attempts<'_>)
        -> Result<Result<T, Unavailable>, OutOfBudget>,
) -> Queued<T>
```

- `serve` returns only what the engine folds: a served `T` or an
  `Unavailable`. The engine maps `Err(Unavailable)` to `Queued::Unavailable`,
  so `unavailable` has one home. Each family's `serve` closure maps the
  retry loop's `Settled<T>` into that shape itself. arXiv's closure runs the
  whole of today's `served` (`fetch.rs:421-476`) in the turn: the query,
  then a withdrawal confirmation for each candidate. The query's
  `Delivered` becomes `ArxivAnswer::Records` after confirmations, its
  `Empty` empty records, its `Failed` `ArxivAnswer::Failed`; a
  confirmation's `Unavailable` becomes `Err(Unavailable)`, its `Failed`
  `ArxivAnswer::Failed`, and its `Empty` "not withdrawn". DOI maps
  `Delivered` to the normalised metadata and `Empty` (a 404) to
  `DoiMetadata::default()`. `Settled` stays `pub(crate)` inside the retry
  loop.
- `fetch_arxiv` returns `Queued<ArxivAnswer>`, where `ArxivAnswer` is
  `Records(Vec<Record>) | Failed(FetchError)`. `FetchOutcome` loses
  `Waiting` and `Rejected` and stays OpenAlex's outcome.
- `render.rs` renders `Waiting`, `Rejected` and `Unavailable` once, from
  `Queued<T>`, for every queued family, with the `source` taken from the
  `Service`. arXiv's stdout, stderr and exit codes stay byte-identical,
  which the phase 2 full-line CLI tests prove.
- The engine's port traits and shared values (`PacingGate`, `ServingTurn`,
  `Transport`, `Clock`, `Deadline`, `OutOfBudget`, `Unavailable`) move from
  `fetch.rs` to a new `cli/research/src/sources/ports.rs`; `FetchError`
  stays in `classify`. `ports` also gains a `Diagnostics` trait
  (`fn report(&self, line: &str)`), which `research-adapters` implements
  over its existing diagnostics writer. `admitted` moves to
  `cli/research/src/sources/admission.rs`. Module dependencies then run one
  way: `fetch` → `serving` → `admission` → `ports`, with `fetch` also
  depending on `ports`. `serving`, `admission` and `ports` import only from
  `ports` and `classify`, never from `fetch`; a unit test in each module
  reading its own source asserts it names no `sources::fetch` path. The
  moved items keep their public paths through re-exports from `fetch`, so
  the public API does not change for them.
- **Admission-poll wiring test**, written first, in `R/arxiv_fetch.rs`
  through the public `fetch_arxiv`, beside the existing busy-queue case at
  `:669` that asserts `[100 ms, 100 ms]`: give the support `RecordingGate`
  `Service::Crossref(Public).pacing()` and expect sleeps of 52 ms. This
  reuses the existing fakes and needs no crate-private type.
- Visibility: `Attempts`, `Attempts::until_settled`, `Settled`, `Step` and
  `admitted` become `pub(crate)`. `Verdict` stays `pub`: the public
  `classify::*` functions return it, and `R/classify.rs` and
  `research-adapters/src/confirmations.rs` use it.
- Public API changes for the pinned `research` crate. The implementer starts
  phase 3 with a dry-run `mise run public-api:update` diff and reconciles it
  with this list before writing code:
  - added: `Queued`, `ArxivAnswer`, `Service`, `CrossrefPool`,
    `Family::Doi`, `Pacing`, `SpacedFrom`, `PacingState`, `Reservation`,
    `Confirmation`, `Provenance`, `RejectedTicket`, and the `Diagnostics`
    port in `ports`. `queued` and `QueuePorts` stay `pub(crate)`, because
    their `serve` closure names the crate-private `Attempts` and both
    callers live in `research`;
  - removed: `FetchOutcome::Waiting`, `FetchOutcome::Rejected`;
  - renamed: `ArxivQueue` → `ServiceQueue`, `Binding::Search`/`Lookup` →
    `ArxivSearch`/`ArxivLookup`, `PacingGate::spacing` → `pacing`,
    `queue::is_front_given` → `is_served_given`;
  - signatures changed: `Queue::new` gains `foreign`; `ServiceQueue::join`
    loses its `spacing: Duration` parameter; `fetch_arxiv` returns
    `Queued<ArxivAnswer>`; `ArxivPorts` holds `&dyn ServiceQueue`;
    `TicketRejection` gains `Foreign`; `RequestError::TicketMalformed`
    carries a `Family`.

  The `public-api` snapshot diff is checked line by line against this list.
  `research-adapters` is not pinned, so its constructor changes under §4
  need no entry.
- `cli/pup.ron` gains rules for `serving`, `admission` and `ports` if the
  module rules need them.

#### 4. Adapters: state per service

**Files**: `cli/research-adapters/src/pacing.rs`, `queue.rs`, `contention.rs`,
`scratch.rs`

- Each service gets its own state directory:

  ```text
  <paths.tmp>/research/services/<service>/
    serving-<i>.lock   one per slot, i in 0..slots
    pacing.lock        held only to read-modify-write pacing
    pacing             {"schema_version":1,"last_sent_ms","next_free_ms","last_finish_ms","not_before_ms"}
    requests.log       one ms timestamp per send
    contention.log
    queue/{queue.lock, <n>-<nonce>.json, <n>-<nonce>.lock}
  ```

- `ServiceAdapters`, in `research-adapters`, builds the gate, queue and
  contention log together, so one call names the `Service` once and the
  three can never disagree. It has two constructors:
  `ServiceAdapters::for_arxiv(context: &AdapterContext,
  previous_release: Option<PreviousRelease>)` and
  `ServiceAdapters::for_crossref(pool: CrossrefPool, context:
  &AdapterContext)`, so the types allow previous-release support only for
  arXiv. `AdapterContext` groups `scratch`, `research`, `clock`,
  `diagnostics` and `nonces`, shared by every service. `PreviousRelease`
  holds the legacy queue directory and the legacy lock path. Both
  constructors derive the other-family queues from the service and
  `Service::ALL`. `main.rs` uses only these constructors.
- `FilePacingGate::new(service, scratch, clock, diagnostics,
  companion_locks: Vec<PathBuf>)` takes its pacing and slot count from
  `service.pacing()` and its directory from `service.code()`. The factory
  fills `companion_locks` from `PreviousRelease`:
  - `try_serve` takes the first free `serving-<i>.lock`, then every companion
    lock, all non-blocking. Each companion is opened or created:
    - a companion that is busy → release the serving lock and return `None`;
    - a companion that cannot be opened or locked for any other reason →
      keep the serving lock, write one diagnostic and serve;
    - a serving lock that degrades on an unlockable filesystem still tries
      its companions.
  - The gate never reads or writes the legacy `arxiv-pacing`.
  - `paced` runs one loop for every service, as two named steps:
    Every step reads the wall clock only after acquiring `pacing.lock`, so
    a send another process records while this caller waits for the lock is
    never clamped back to a stale `now`.
    - `reserve_send`: take `pacing.lock` (blocking, short), read the state
      and `now`, apply `trust` and call `reserve`. If the deadline does not
      admit the wait until `send_at`, release the lock and return
      `WaitTooLong`, writing nothing. Otherwise write the resulting state
      whenever it differs from the state read from disk, whether `trust`
      or `reserve` changed it, then release the lock and sleep until
      `send_at`, charging the sleep to the caller's deadline. A
      `PreviousFinish` reservation with no clock step changes nothing and
      writes nothing, so a one-slot arXiv send still writes the pacing file
      twice, as today. A clamped future stamp is written back at once, so
      `confirm_send` never re-clamps it to a later `now`.
    - `confirm_send`: take `pacing.lock` again, apply `trust` and call
      `confirm`, then write whatever state it returns:
      - `Send` → write it, release the lock, log the send and run the
        attempt;
      - `WaitUntil` → if the deadline admits it, release the lock, sleep
        and confirm again; otherwise write `release`'s state, release the
        lock and return `WaitTooLong`;
      - `Deferred` → write its carried state and reserve again.

    The loop reads `loop { reserve_send; match confirm_send { … } }`.
    Afterwards it records the finish and any deferral under `pacing.lock`.
  - An unlockable filesystem degrades as today, for every slot count.
- `FileServiceQueue::new(service, research, clock, diagnostics, nonces,
  foreign_sources)` (renamed from `FileArxivQueue`):
  - `foreign_sources` comes from the factory: `OtherFamilyQueues` (the
    `queue/` directory of every service in `Service::ALL` with a different
    `ticket_family()`) and, when a `PreviousRelease` is given,
    `PreviousReleaseQueue` (the legacy `<research>/arxiv-queue/`). Retiring
    the legacy support deletes the `PreviousRelease` argument.
  - The listing only reads names and parses tickets from `*.json` names. It
    never locks, writes or prunes those directories. A directory that is
    absent contributes nothing. A directory that cannot be listed
    contributes nothing and writes one diagnostic, and the call goes on.
  - Pools of the same family are not foreign. A doi ticket presented to the
    other pool joins afresh.
  - Provenance comes only from the directory. `StoredBinding` gains no
    `service` field.
- `FileContentionLog::new(service, …)` writes `services/<service>/contention.log`.
- Housekeeping and the schema filter are unchanged, scoped to the service's
  own `queue/`.
- **Adapter tests** (`A/pacing.rs`, `A/queue.rs`), written before the code.
  Timing tests use a new `HookedClock` test double in
  `cli/research-adapters/tests/support/mod.rs`: thread-safe, its `sleep`
  runs a registered closure and then advances time, so another gate's
  `paced` or a deferral write can happen inside a caller's sleep without
  real waiting.
  - with three slots, three `try_serve` calls succeed and a fourth returns
    `None`; dropping one frees exactly one slot;
  - inside A's sleep, B's `paced` reserves a distinct send time one interval
    after A's, and both then send;
  - a deferral written inside a caller's sleep holds that caller until the
    deferral ends;
  - the out-of-lock sleep is charged to the caller's deadline; a call refused
    by its deadline before reserving writes nothing, and one refused after
    waking hands its reservation back;
  - an unlockable filesystem still degrades with three slots;
  - a busy companion lock makes `try_serve` return `None` and frees the
    serving lock; an old-release holder of `arxiv.lock` blocks a new-release
    call, and vice versa; an unopenable companion path (a directory in its
    place) still serves, with one diagnostic; an absent companion is
    created, with no diagnostic; a serving lock degraded on an unlockable
    filesystem still returns `None` while its companion is busy;
  - `ServiceAdapters::for_arxiv` gives the gate the legacy lock as its
    companion only when a `PreviousRelease` is passed;
    `ServiceAdapters::for_crossref` never has a companion; both derive the
    service's other-family queues;
  - an unlistable foreign directory leaves the ticket to join, with one
    diagnostic;
  - `now` is read only after `pacing.lock` is held: the test takes
    `pacing.lock` itself and starts the caller's `paced` on a thread. A
    `HookedClock` hook on the caller's wall-clock reads signals the test;
    the test waits up to 200 ms for that signal, which a correct gate never
    sends while blocked on the lock. The test then writes another send's
    stamp at the current time, advances the clock and releases the lock.
    The caller must wait one interval after that send; a gate that read
    `now` before the lock clamps the send back to its stale `now` and fails;
  - a pacing file of another `schema_version`, or one missing
    `next_free_ms` or any other required field, counts as no previous
    request and reports one diagnostic per `paced` call. This is new
    behaviour: today a corrupt file is discarded silently. The test uses
    `Service::Crossref(Public)`, and
    `a_corrupt_state_file_counts_as_no_previous_request` is extended, as a
    new test beside it, to assert the same single diagnostic. A file that
    cannot be read at all (for example a directory in its place) still
    reports nothing, so the failed-write test keeps its count of 2;
  - each existing single-lock test
    (`try_serve_is_none_while_another_holder_has_the_lock`,
    `a_turn_holds_the_lock_between_attempts_until_dropped`,
    `an_in_lock_wait_is_spent_from_the_callers_deadline`) is kept for one
    slot, and its multi-slot successor is named beside it.

#### 5. CLI wiring and test helpers

**Files**: `cli/research-cli/src/main.rs:273-348`, `fetch_command.rs`,
`render.rs`, `tests/support/mod.rs`, `cli/research-adapters/tests/support/mod.rs`
**Changes**:
- Build the arXiv adapters through `ServiceAdapters::for_arxiv` with a
  `PreviousRelease` naming the legacy `arxiv-queue/` and `arxiv.lock`.
- `render::rejection` delegates to `RejectedTicket`.
- The phase 2 path helpers return the new paths. `Project::arxiv_state`
  becomes a thin wrapper over a new
  `Project::service_state(Service, StateFile)` (and its adapter-side
  equivalent), which the Crossref tests use.
- New `support` helpers for the previous release's state:
  `Project::legacy_arxiv_state(LegacyFile)` (the old `arxiv-queue/`,
  `arxiv-pacing` and `arxiv.lock`) and `Project::seed_legacy_fixture()`,
  with adapter-side equivalents. The legacy-fixture, held-`arxiv.lock` and
  old-release-holder tests use only these, so the `rg` check stays clean.
- `Project::hold_barrier(Service)` holds one live ticket per serving slot
  at the front of that service's queue and returns a guard that releases
  them together; every multi-process test uses it.

#### 6. Legacy fixture test

**File**: `cli/research-cli/tests/arxiv_queue.rs`
**Test** (written first; red until the steps above land):
`a_ticket_from_the_previous_release_is_a_mismatch_and_a_fresh_call_is_served`.

1. Copy the fixture into `<scratch>/arxiv-queue/` and `<scratch>/arxiv-pacing`.
2. Re-present `ticket.txt`'s ticket in an arXiv lookup. Expect exit 2, a stderr
   line starting `E_ARXIV_TICKET_MISMATCH`, and 0 mock hits.
3. Re-run without `--ticket`. Expect stdout equal to the unqueued `ok` golden
   for the recorded arXiv response.

Also add a cross-family test at the domain level, `R/queue.rs`: a ticket in
`foreign` as `IssuedElsewhere(Family::Doi)`, presented to arXiv, is rejected.
The CLI-level cross-family criteria wait for phase 4.

**Test** (written first), also in `C/arxiv_queue.rs`:
`an_arxiv_call_waits_while_an_older_release_holds_arxiv_lock`. Hold
`<scratch>/arxiv.lock` from the test, run an arXiv lookup under a short call
budget, and expect `waiting` with 0 mock hits. This proves `main.rs` passes
the legacy lock as the arXiv gate's companion. Phase 4 adds the matching
check that a `doi metadata` call is unaffected by the same held lock.

#### 7. Docs and skill text

**Files**:
- `docs-site/src/content/docs/research.md`, the `## Pacing` section
  (`:294-365`, including the last Limits bullet's "never spacing, which the
  lock enforces on every request"). Rewrite it for the new design, with
  arXiv as the worked case, not only its paths:
  - Pacing: per-service state under `services/<service>/`, serving slots,
    the short `pacing.lock`, and the confirmation before each send;
  - The fetch queue: admission while fewer than `slots` tickets ahead are
    unended;
  - Logs: the per-service `requests.log` and `contention.log`; drop the
    sentence about timestamp-only contention lines from earlier
    prereleases;
  - Limits: replace "an earlier plugin version still paces through
    `arxiv.lock`" with what holds now. Old and new releases still keep one
    arXiv connection at a time through the legacy `arxiv.lock`, but their
    pacing state is separate, so expect occasional throttling until older
    sessions end;
  - the `queue_unusable` remedy names `services/<service>/queue/`, with
    `services/arxiv/queue/` as the example;
  - a note that `arxiv-pacing`, `arxiv-queue/` and the old logs can be
    deleted once no older plugin version runs in the project, while
    `arxiv.lock` stays, recreated empty by every arXiv call, until a later
    release retires the companion lock.
- `research.md`, the E_* table: `E_ARXIV_TICKET_MISMATCH` also covers another
  family's ticket and a ticket issued before this release.
- `skills/research/research-topic/SKILL.md:619`: the contention-log path,
  `services/arxiv/contention.log`.
- Then `mise run docs:generate`.

Update any structure-test pin on the old path.

### Outcome

Phase 3 landed in seven commits, each green on its own, and a
follow-up test fix. The suites run 1219 tests; the opt-in stress run
passed in 117 s; a live `fetch arxiv lookup 1706.03762` wrote state only
under `services/arxiv/`, beside an empty `arxiv.lock`. The bare
`mise run` exits 0, with CLI line coverage at 94.33%. Its first run
failed the 92% floor at 91.62% only on stale profile data, which
`cargo llvm-cov clean --workspace` cleared. Departures from the plan:

- **Foreign-ticket wording**: "was issued by fetch `<code>`" (for
  example `fetch doi`), not "by a `<family>` fetch", which reads "a
  arxiv" for arXiv.
- **`MalformedTicket`**: `Ticket::parse` and `Nonce::from_hex` return a
  family-less `MalformedTicket`, and `parse_ticket` maps it to
  `RequestError::TicketMalformed(Family, String)`. The `R/queue.rs`
  malformed-ticket pattern therefore names `MalformedTicket`, not
  `TicketMalformed(Family::Arxiv, …)`.
- **Names kept**: `Place::is_front` keeps its name; `Deadline` stays in
  `schedule`. `serving`, `admission` and `ports` import from `schedule`,
  `queue`, `pacing` and `request` as well as `ports` and `classify`;
  each module's own test asserts it names no `sources::fetch` path.
- **Visibility**: `admission` is a private module, so it adds no public
  item.
- **Extra public items**: `MalformedTicket`, `Pacing::previous_finish`,
  `Pacing::previous_start`, `Display` for `Binding` (the call as typed,
  used by `RejectedTicket`).
- **`AdapterContext`** holds `research: ScratchDir` (which carries the
  project root), `clock`, `diagnostics` and `nonces: Rc<dyn Fn() ->
  Nonce>`. `FileServiceQueue::new` takes `Vec<ForeignSource>`, each a
  directory and its `Provenance`.
- **CLI rendering**: `FetchedRecords::outcome` is `RecordsOutcome`,
  `Direct(FetchOutcome)` or `Queued { service, queued }`. `render` reads
  both through one `Ending` view, and every queue state through a
  generic `queued_ending<T>` that phase 4 reuses.
- **Tests placed differently**: the `admission_poll` cases live in
  `R/service.rs`. The pure `PreviousFinish` equivalents in `R/pacing.rs`
  copy `a_first_request_waits_for_nothing`,
  `a_request_is_spaced_three_seconds_after_the_last_finish`,
  `a_last_finish_in_the_future_waits_at_most_three_seconds`, the three
  deferral cases, `a_stored_deferral_more_than_thirty_seconds_ahead_is_discarded`,
  the unfinished-send case, `a_send_stamped_ahead_of_now_waits_no_more_than_one_spaced_send`
  and `spacing_runs_from_the_last_finish_when_it_follows_the_last_send`.
- **`now` after `pacing.lock`**: under the virtual clock a gate that
  read `now` early sends at the same moment as a correct one, so the
  test's deciding assertion is that no wall-clock read happens while the
  lock is held elsewhere.
- **Legacy test**: the fresh call's stdout is pinned by a new golden,
  `arxiv-lookup-withdrawn.json`. The held-`arxiv.lock` test uses a 40 s
  budget; 32 s never admits arXiv's 3 s window plus a 30 s request, so it
  would pass without the companion.
- **Phase 2 test body changed**: under the full instrumented
  `mise run`, start-up alone could use up the 100 ms the throttled call
  in `a_throttled_call_defers_a_concurrent_process` had beyond arXiv's
  33 s serving window, so it returned `waiting` without sending. Its
  budget rose from 33.1 s to 36 s, which still admits its first attempt
  and still refuses the 5 s retry (which needs 40 s). This is the one
  change to a phase 2 CLI test body outside `support/`, a deliberate
  exception to the criterion below.

### Success Criteria

#### Automated Verification

- [x] The domain, adapter and CLI suites pass:
      `cd cli && cargo nextest run -p research -p research-adapters -p accelerator-research --features accelerator-research/test-loopback`
- [x] The phase 2 CLI tests pass with only `support/` files changed in
      `cli/research-cli/tests`, apart from the new legacy and provenance
      tests. Check with `jj diff --stat cli/research-cli/tests`.
- [x] `jj diff cli/research/tests cli/research-adapters/tests` shows only:
      - `is_front_given` → `is_served_given(…, 1, …)`;
      - `ArxivQueue` → `ServiceQueue`, `FileArxivQueue` → `FileServiceQueue`;
      - in `R/arxiv_fetch.rs`, outcomes re-wrapped with the same expected
        values: `FetchOutcome::Records`/`Failed` →
        `Queued::Served(ArxivAnswer::…)`, `FetchOutcome::Unavailable`/
        `Waiting`/`Rejected` → `Queued::Unavailable`/`Waiting`/`Rejected`;
      - the support fake's `spacing()` → `pacing()`;
      - `RequestError::TicketMalformed` patterns gaining `Family::Arxiv`
        (`R/queue.rs:203`);
      - no `A/pacing.rs` wait-rule case changes; their pure equivalents in
        `R/pacing.rs` are new tests;
      - `an_in_lock_wait_is_spent_from_the_callers_deadline` renamed to
        `an_out_of_lock_wait_is_spent_from_the_callers_deadline`, with its
        assertion unchanged;
      - the deliberately retired
        `a_state_written_before_last_sent_was_recorded_still_spaces_the_request`;
      - other `support/` changes (including the constructor and pacing-file
        helpers from phase 2 §1a), and new tests.
- [x] No test body outside `support` names a service state path:
      `rg -n '"services/|["/]arxiv(\.lock|-pacing|-requests\.log|-contention\.log|-queue/?")' cli/research-cli/tests cli/research-adapters/tests --glob '!**/support/**' --glob '!**/fixtures/**'`
      prints nothing.
- [x] The public API snapshot is refreshed and checked, and its diff names
      only the types listed under §3:
      `mise run public-api:update && mise run public-api:check`
- [x] `mise run pup:check`
- [x] The research structure tests pass:
      `uv run pytest tests/unit/tasks/test_research_structure.py`
- [x] `mise run docs:check`
- [x] `mise run check`

#### Manual Verification

- [x] `mise run test:integration:arxiv-queue-stress` passes (opt-in, real
      clock).
- [x] A real arXiv `fetch arxiv lookup` in a scratch project writes state only
      under `.accelerator/tmp/research/services/arxiv/`, apart from creating
      the empty companion `.accelerator/tmp/research/arxiv.lock`.

---

## Phase 4: `fetch doi metadata` on the Anonymous Pool

### Overview

Add the `doi` family on `Service::Crossref(CrossrefPool::Public)`, driving it
with tests first from the work item's CLI acceptance criteria. Recorded
Crossref bodies are captured live, or built by hand where no live record fits.

### Changes Required

#### 1. Domain: request and family

**File**: `cli/research/src/sources/request.rs`

- `Family::Doi`, added in phase 3 for tickets, now joins the CLI:
  - verbs `[Verb::Metadata]`;
  - `ALL` grows to 4, and `Family::parse` accepts `doi`.
- `UnknownFamily`, `LimitNotApplicable` and `TicketNotQueued` messages name
  the new family list. Each keeps its code and sentence shape; only the list
  of families changes. Update the existing assertions on those lines in
  `C/fetch_*.rs` and the matching `research.md` rows in the same change.
- `parse_ticket` accepts `Family::Arxiv | Family::Doi`.
- `DoiRequest(Doi)` and `FetchRequest::Doi`. Exactly one term is required.
- `Binding::DoiMetadata(Doi)`. `differing` compares DOIs by
  `Doi::canonical()`, and `StoredBinding` stores the canonical form, so a
  ticket matches its call whatever the DOI's letter case.
- New errors:
  - `RequestError::MissingDoi` →
    `E_RESEARCH_USAGE: metadata needs exactly one DOI`;
  - `RequestError::MalformedDoi(raw)` →
    `E_DOI_MALFORMED: expected a DOI such as 10.1145/371920.372066 or
    https://doi.org/10.1145/371920.372066 (got '…')`.
- `Endpoint::crossref_api()` defaults to `https://api.crossref.org`.
- `UpstreamRequest::crossref_work(api, doi, contact: Option<&ContactAddress>)`:
  - sends `GET {api}/works/{doi.encoded()}` with no query;
  - sends headers `Accept: application/json` and `User-Agent: USER_AGENT`;
  - sends no bearer.

  Phase 4 passes `None`.
- Tests (`R/request.rs`):
  - bare, `doi.org`, `dx.doi.org` and `doi:` forms parse to the same `Doi`;
  - `not-a-doi` → `MalformedDoi`;
  - a `crossref_work` URL has no `?`;
  - `Binding::DoiMetadata` for `10.48550/arXiv.1706.03762` and
    `10.48550/arxiv.1706.03762` does not differ.

#### 2. Domain: classification and normalisation

**File**: `cli/research/src/sources/classify.rs`

```rust
pub fn crossref(response: Response) -> Verdict {
    let received = match response {
        Response::Received(received) => received,
        Response::ConnectionFailed | Response::TimedOut => {
            return upstream_error(None)
        }
    };
    match received.status {
        200..=299 => Verdict::Deliver(received.body),
        404 => Verdict::Empty,
        429 => Verdict::Retry {
            reason: Reason::RateLimited,
            retry_after: received.retry_after,
        },
        500..=599 => upstream_error(received.retry_after),
        _ => Verdict::Unavailable(Reason::UpstreamError),
    }
}
```

Tests (`R/classify.rs`):
- 200 → deliver; 404 → empty;
- 429 → throttling, with retry-after;
- 503 → retry;
- 400, 403 and 301 → unavailable `upstream_error`;
- a connection failure → retry; a timeout → retry.

**File**: `cli/research/src/sources/crossref.rs` (new)

```rust
pub struct CrossrefWork {
    pub kind: Option<String>,
    pub container_titles: Vec<String>,
    pub publisher: Option<String>,
}

pub struct DoiMetadata {
    pub metadata: SourceMetadata,
    pub venue: Option<Venue>,
}

pub fn normalise(work: &CrossrefWork) -> DoiMetadata
```

- `venue` is the first container title, whitespace-collapsed and non-empty,
  and only when `kind` is `journal-article` or `proceedings-article`.
- `publisher` is recorded through a new `SourceMetadata::record_publisher`,
  whitespace-collapsed and skipped when blank. A Crossref miss
  (`Settled::Empty`) gives `DoiMetadata::default()`.

`CrossrefDecoder` port:
`fn work(&self, body: &[u8]) -> Result<CrossrefWork, DecodeFailure>`.

```rust
pub fn fetch_doi(
    fetch: &DoiFetch<'_>,
    ports: &DoiPorts<'_>,
    deadline: &Deadline,
) -> Queued<DoiMetadata>
```

- `fetch_doi` runs `queued` with `Binding::DoiMetadata`. Its `serve` closure
  maps a delivered body to the normalised metadata and a 404 to
  `DoiMetadata::default()`.
- An undecodable 2xx body becomes `Err(Unavailable)` with
  `Reason::UpstreamError`, which the engine folds into
  `Queued::Unavailable`, because only usage errors may exit non-zero. It
  also reports one line through the `Diagnostics` port naming the `doi`
  family, the DOI and the decode failure, so a Crossref schema change can be
  told apart from an outage. `DoiPorts` holds the `QueuePorts`, the
  `CrossrefDecoder` and that `Diagnostics`; `main.rs` passes the adapter
  crate's diagnostics writer.

Tests (`R/crossref.rs`):
- journal-article → venue;
- two titles → the first;
- book-chapter → no venue, with publisher;
- absent or `[]` container-title → no venue;
- no publisher → empty map.

**File**: `cli/research/src/sources/metadata.rs`
**Changes**: `record_publisher(&Publisher)`. A new CLI test pins the rule
that every key `fetch doi metadata` can print ranks after every key
`fetch web metadata` can print in `RECOMMENDED_KEYS`. It takes the keys from
real output, not a hand-picked list of recorders: `web metadata` on a fixture
page that fills every field it can extract, and `doi metadata` on a fixture
Crossref record that fills every field it reads. It then compares the
maximum rank on one side with the minimum on the other, so a recorder added
to either producer later is exercised by the same fixtures.

#### 3. Adapters

**Files**:
- `cli/research-adapters/src/crossref_json.rs` (new): `JsonCrossrefDecoder`,
  reading `message.type`, `message.container-title` and `message.publisher`.
- `cli/pup.ron`: add it to the decoders-spawn-nothing rule.
- `Service::Crossref(Public)` already exists from phase 3, with its pacing
  and code. The adapters gain no Crossref-specific code; they read both
  values from the `Service`.
- `cli/research-adapters/tests/crossref_json.rs` (new): each fixture decodes
  to the expected `CrossrefWork`. A missing `message` is a `DecodeFailure`.
  A `null` or non-array `container-title`, a `null` `publisher` and a
  missing `type` decode to empty fields.

**Fixtures** in `cli/research-adapters/tests/fixtures/crossref/`:
- `proceedings-371920-372066.json` (live);
- `journal-two-container-titles.json` (live if one is found, otherwise
  hand-built);
- `book-chapter.json` (live);
- `publisher-no-container-title.json` and
  `publisher-empty-container-title.json` (hand-built from a live record);
- `bare.json` (hand-built).

#### 4. CLI

**Files**:
- `cli/research-cli/src/fetch_command.rs`:
  - `SourceCall::Doi(DoiCall, DoiAdapters)`;
  - `Fetched::Doi(FetchedDoi { attempts, outcome: Queued<DoiMetadata> })`.
- `cli/research-cli/src/main.rs` `source_call`:
  - project and scratch;
  - `ServiceAdapters::for_crossref(CrossrefPool::Public, &context)`;
  - `JsonCrossrefDecoder`;
  - `loopback` override `ACCELERATOR_CROSSREF_API_URL`.
- `cli/research-cli/src/render.rs`: `doi_document`, built from
  `#[derive(Serialize)]` structs in field order:
  - `{"status":"ok","metadata":{…},"venue":"…"}`, with `venue` skipped when
    `None`;
  - `{"status":"unavailable","source":"crossref","reason":…[, "cause":…]}`.

  `waiting` and rejections go through the shared `Queued<T>` rendering from
  phase 3, with `source` `crossref`. The stderr summary line matches arXiv's
  form.
- `cli/research-cli/src/cli.rs`:
  - `FETCH_HELP` gains a `doi takes metadata (one DOI, bare or a doi.org URL)`
    clause and a paragraph describing the `doi metadata` document and its
    statuses;
  - the opening line and the subcommand's short about no longer say every
    call returns tiered scholarly records;
  - the `family` and `verb` arg docs list `doi` and `metadata`;
  - the `--ticket` paragraph and arg doc name arXiv and DOI calls and their
    `source` values;
  - the closing paragraph's waiting and `E_ARXIV_TICKET_LIVE` sentences cover
    both queued families (`E_<FAMILY>_TICKET_LIVE`).

  The help-pairing test (`cli.rs:153-200`) covers the new clause. Add tests
  that the `--ticket` paragraph, the `--ticket` arg doc and the `family` arg
  doc each name `doi`.

#### 5. CLI tests

**File**: `cli/research-cli/tests/fetch_doi.rs` (new), using
`MockHTTPServer`, `LoggingClock` and the `support` helpers. Each criterion
gets its own test, written before the code that passes it:

- Recorded `10.1145/371920.372066`:
  - exit 0;
  - venue `Proceedings of the 10th international conference on World Wide Web`;
  - metadata only `publisher: ACM`;
  - no `tier`;
  - exactly one request to `/works/10.1145%2F371920.372066` (whatever
    `encoded()` gives), with `queries` `[None]`, and no `Authorization` or
    `Crossref-Plus-API-Token` header.
- The same output for the bare DOI, `https://doi.org/…` and
  `https://dx.doi.org/…`.
- Two titles → the first; book-chapter → no venue.
- Absent or `[]` container-title → publisher only; bare record →
  `{"status":"ok","metadata":{}}`; 404 → the same.
- Unreachable endpoint (closed port), under LoggingClock:
  - exit 0;
  - real elapsed time plus logged waits ≤ 100 s;
  - `unavailable` `upstream_error`.
- 400 → `unavailable` `upstream_error` after one request.
- 503 on every attempt → `upstream_error`; 429 on every attempt →
  `rate_limited`; mixed last-attempt cases → the last attempt's reason. Each
  must end within 100 s.
- 429 with `Retry-After: 5` to caller A, as separate processes in real time.
  A's route is `Route::Sequence([429 with Retry-After: 5, 200 recorded
  body])`, so A's retry succeeds. B is spawned and has joined the queue
  before A's 429 is answered. Measured from the mock's `hit_instants` plus
  its fixed response delay, neither A's next request nor B's first arrives
  before A's 429 response + 5 s, and both arrive within that + 1 s, so a
  hung A or B also fails.
- arXiv's `pacing` seeded with `not_before = epoch + 60 s`, then a Crossref
  call under LoggingClock with the same epoch → no wait logged, so the request
  starts before 1 s. The swapped case uses the arXiv lookup path.
- A live seeded ticket held in `services/crossref-public/queue/` past 100 s →
  `waiting` with position ≥ 1. Releasing it and re-presenting → the unqueued
  `ok` output.
- With `<scratch>/arxiv.lock` held by the test, a `doi metadata` call is
  served normally, so the companion lock binds only the arXiv gate.
- Ticket mismatches, each with 0 hits:
  - a Crossref ticket presented to arXiv → `E_ARXIV_TICKET_MISMATCH`;
  - an arXiv ticket presented to doi → `E_DOI_TICKET_MISMATCH`;
  - a Crossref ticket with a different DOI → `E_DOI_TICKET_MISMATCH`.
- Malformed `--ticket` → `E_DOI_TICKET_MALFORMED`; a held ticket →
  `E_DOI_TICKET_LIVE`. Both exit 2 with no request.
- A ticket issued 901 s earlier (epoch moved):
  - with no last reason → `rate_limited` + `cause: lock_contention`;
  - with last reason `upstream_error` or `rate_limited` → that reason and no
    `cause`.
- 503 with `Retry-After: 30` on every attempt, under LoggingClock: the call
  returns `waiting` once the next 30 s wait no longer fits in 100 s. That
  ticket, re-presented past 900 s, → `upstream_error` with no `cause`.
- `not-a-doi` → exit 2, `E_DOI_MALFORMED`.
- A 200 with a body that is not JSON → exit 0, `unavailable`
  `upstream_error`, and one diagnostic line naming the DOI.
- A ticket re-presented with the DOI in a different letter case resumes
  rather than mismatching.
- Rate, public pool, real time: 10 invocations, one call each, spread over
  5 concurrent process slots, each answered after a 50 ms `Delayed`. Behind
  `hold_barrier`, all 10 join; after its release, from the mock's
  `hit_instants`: no 6 requests fall
  within any [t, t+1 s), and the 10th is < 2 s after the first. Every stamp
  is in `services/crossref-public/requests.log`.
- Concurrency, public pool: the work item's 10 calls with a 2 s `Delayed`.
  From `hit_instants`, no two requests overlap.

In `cli/.config/nextest.toml`, only the tests with tight liveness assertions
(the public and polite rate tests, `not-an-address` and mixed pools) get
`threads-required = "num-test-threads"`, so each runs alone. The
concurrency and `Retry-After` tests, whose assertions have wide slack, join
a `real-time` test group with `max-threads = 2`, so they overlap the rest of
the suite. Each liveness assertion reports its observed
slack (last start minus nine intervals) and the largest per-gap hand-off
overhead in its failure message, so drift shows before it flakes. The hard
"no N + 1 in a window" assertions stay strict.

Add `ACCELERATOR_CROSSREF_API_URL` to the loopback endpoint list and to the
release-guard checks, the same way the other `*_API_URL` variables are.

#### 6. Docs

**File**: `docs-site/src/content/docs/research.md`
**Changes**, passage by passage:
- the intro: `fetch` queries OpenAlex, arXiv and Crossref, and `doi metadata`
  reports a venue, not tiered records;
- the verb table's `fetch` row (`:21`): it also reports a DOI's venue and
  publisher;
- the verb and family tables (`:39-43`): a `doi` row with `metadata`;
- the `--ticket` paragraph (`:51-53`): it serves arXiv and DOI calls;
- the budget caveat (`:58-60`): "an arXiv or DOI call that prints
  `waiting`";
- Output: a `doi metadata` example for `ok`, `unavailable` and `waiting`;
- the `rate_limited` / `lock_contention` row (`:95`) and the waiting text
  (`:101-111`): "a queued arXiv or DOI fetch";
- Exit codes and the E_* table (`:117-136`): the exit-0 row (`:121`) names
  `ok`, `unavailable` and `waiting` from `doi metadata`; exit 2 names
  `E_<FAMILY>_TICKET_LIVE`; `E_RESEARCH_USAGE` adds "metadata needs exactly
  one DOI"; `E_DOI_MALFORMED` and `E_DOI_TICKET_*` rows;
  `E_RESEARCH_CLIENT_ERROR` (`:136`) notes that for a DOI call a 4xx other
  than 404 and 429, or an unfollowed redirect, is `unavailable`, not exit 1;
- Retries and deadlines (`:148-150`): a queued arXiv or DOI call stops early
  with `waiting`;
- the redirect rule (`:154-156`): a DOI call's unfollowed redirect is
  `unavailable` `upstream_error`, not a client error;
- Granting the fetch (`:498-505`): `web-profile` uses the grant for
  `fetch web metadata` and `fetch doi metadata`;
- Pacing: Crossref's public pool, 210 ms between request starts with one in
  flight, under `services/crossref-public/`, against arXiv's 3 s after the
  previous response ends; each send is confirmed against the last actual
  send;
- The fetch queue (`:306-330`): "arXiv and DOI calls take turns", with a
  queue directory per service, a serving window of the service's interval
  plus 30 s, and `E_DOI_TICKET_*` beside the arXiv codes;
- the Logs `queue_unusable` remedy (`:343`): names
  `services/crossref-public/queue/` among the per-service queues;
- Limits: Crossref's limits apply per IP, but pacing is coordinated only
  within one project, so parallel research in several projects can draw
  429s.

The implementer re-checks the line references before editing.

### Outcome

Phase 4 landed in six commits. The suites run 1296 tests, and
`mise run check` exits 0. Live calls printed the expected documents, and
ten in a loop each sent one request with no retry. The two-title journal
fixture is live (`10.4213/tmf1151`). Each commit refreshes its own share
of the `public-api` snapshot and passes `public-api:check` alone; the live
checks were re-run on a binary built from the rewritten tip. The bare
`mise run` has not been run for this phase. Departures from the plan:

- **No `contact` parameter yet**: `UpstreamRequest::crossref_work(api,
  doi)` takes no contact address; phase 5 adds `ContactAddress` and the
  parameter together.
- **Placement**: `CrossrefDecoder`, `DoiFetch`, `DoiPorts` and `fetch_doi`
  live in `fetch.rs` beside the other families. `DoiPorts` lists its ports
  flat, as `ArxivPorts` does, since `QueuePorts` is crate-private.
  `Publisher` is a value type in `metadata`, and `Venue` one in `crossref`.
- **`Argument::Doi`**: a DOI mismatch names `differing: doi`, and the
  issued call reads `metadata <canonical DOI>`.
- **`FetchedDoi` carries its `Service`**, so rendering takes `source` and
  the rejection prefix from it, and phase 5 can name the polite pool.
- **Usage lines**: `UnknownFamily` derives its list from `Family::ALL`.
  `LimitNotApplicable` is unchanged, as `--limit` still applies only to
  openalex and arxiv.
- **Release guard**: it scans only for the loopback marker and lists no
  `*_API_URL` variable, so `ACCELERATOR_CROSSREF_API_URL` joined only the
  loopback endpoints and the test `support` commands.
- **Granting the fetch**: the `research.md` sentence naming `fetch doi
  metadata` waits for phase 7, when the web profile first calls it.
- **Help**: the queue paragraph's last sentence reads "Treat any status
  other than ok, unavailable or waiting as unavailable", so the pinned
  phrase stays on one wrapped line.
- **Tests**: the backoff test re-presents its ticket at 250 s, 500 s and
  750 s before 901 s, since a ticket that ended over 300 s earlier expires
  and rejoins rather than passing its cap. The `Retry-After` test answers
  A's 429 after 1 s. Only the public rate test runs alone, and only the
  public concurrency and `Retry-After` tests join the `real-time` group;
  the polite, `not-an-address` and mixed-pool tests arrive in phase 5.

### Success Criteria

#### Automated Verification

- [x] `cd cli && cargo nextest run -p research -p research-adapters -p accelerator-research --features accelerator-research/test-loopback`
- [x] `mise run public-api:update && mise run public-api:check`
- [x] `mise run pup:check`
- [x] `mise run test:integration:release-guard`
- [x] `mise run docs:check`
- [x] `mise run check`

#### Manual Verification

- [x] A live `accelerator research fetch doi metadata '10.1145/371920.372066'`
      prints the expected venue and `publisher`.
- [x] A live `fetch doi metadata 'https://doi.org/10.48550/arXiv.1706.03762'`
      prints `{"status":"ok","metadata":{}}`.
- [x] Ten live calls in a loop never draw a 429.

---

## Phase 5: `crossref.mailto` and the Polite Pool

### Overview

Add the setting, its resolution order, the plausibility check and warning, and
the `User-Agent` contact. Add `Service::Crossref(Polite)`, chosen per call,
with its own queue and pacing.

### Changes Required

#### 1. Domain: contact address

**File**: `cli/research/src/sources/crossref.rs`

```rust
pub struct ContactAddress(String);

impl ContactAddress {
    pub fn parse(raw: &str) -> Result<Option<Self>, Implausible>;
}
```

`parse` trims the value. Blank → `Ok(None)`. Otherwise the address is
plausible when it has:
- only visible ASCII (0x21–0x7E), so no whitespace, control or non-ASCII
  characters;
- no `(`, `)` or `\`, which would end or escape the `User-Agent` comment it
  sits in;
- exactly one `@`;
- a non-empty local part with no `:`;
- a domain that contains `.` and does not start or end with it.

`UpstreamRequest::crossref_work`, which phase 4 shipped as
`crossref_work(api, doi)`, gains `contact: Option<&ContactAddress>` and sets
`User-Agent` to `"{USER_AGENT} (mailto:{address})"` when an address is
present. The research
domain holds only `ContactAddress` and `CrossrefPool`; it names no config
type, as `cli/pup.ron`'s research import rule requires.

Tests (`R/crossref.rs`): each accepted and rejected example in the item, the
trimmed ` research@example.org `, and the rejected `ré@example.org`,
`a(b)@example.org`, `a\b@example.org` and an address containing DEL.

#### 2. Config key

**File**: `cli/config/src/catalogue.rs`
**Changes**:
`ExtraKey::overridden("crossref.mailto", Trust::Open, &["ACCELERATOR_CROSSREF_MAILTO"])`.

Then update:
- `every_key_pins_its_overrides_and_recovery` (`catalogue.rs:646-692`);
- `cli/launcher/src/launch/help.rs:52-78` and
  `cli/launcher/tests/config_help.rs:62-72`;
- `cli/launcher/tests/fixtures/dump/dump.golden`.

#### 3. Open-setting resolution in config

**File**: `cli/config/src/open_setting.rs` (new)
**Changes**: a generic resolver for open-trust keys,
`open_value(environment: &dyn Environment, config: &dyn ConfigAccess, key: &OpenKey) -> Result<Option<SettingValue>, UnreadableLevel>`.
`OpenKey` can only be built from a catalogue `ExtraKey` whose trust is
`Trust::Open` and which is not marked as a credential, so consent-gated keys
and credentials can never reach this ladder; credentials keep the
`credentials` ladder, with its command rungs, consent and secret handling.
The catalogue gains that marker: `ExtraKey` records whether a key is a
plaintext credential, set on `openalex.api_key` and every `*.token` key
that a client pairs with a command key. `TokenKeys::declared` checks its
plaintext key carries the marker, so credential-ness is declared once, in
the catalogue, and the client crates' pairings cannot drift from it. Tests:
- `catalogue.rs`: every plaintext sibling of a `*_cmd` key is marked, and no
  other key is, which the catalogue can check by itself;
- `cli/config/tests/credentials.rs`, written first:
  `TokenKeys::declared("crossref.mailto", "jira.token_cmd")` is refused with
  `ConfigError::Invalid`, and a marked plaintext key is accepted.

```rust
pub enum SettingSource {
    Environment(&'static str),
    Level(Level),
}

pub struct SettingValue {
    pub value: String,
    pub source: SettingSource,
}

pub struct UnreadableLevel {
    pub level: Level,
    pub reason: ConfigError,
}
```

The existing `service::Source` names no environment rung and carries the
catalogue default and `Unset`, and `credentials::TokenSource` carries
command rungs; neither fits a plain, non-secret setting, so `SettingSource`
is the open ladder's own vocabulary. `Level` and `ConfigError` are reused as
they are.

1. Take the first non-blank, trimmed value of:
   - each variable in `key.overrides`, in order;
   - the key at `Level::Personal`;
   - the key at `Level::Team`.
2. Return it as a `SettingValue`, whose source displays as the variable's
   name, `config.local.md` or `config.md`. The caller judges plausibility;
   the resolver never falls through after choosing a value.
3. If a level cannot be read, return `UnreadableLevel`.

The environment variable's name lives only in the catalogue. The new types
join the config crate's public API in this phase. Tests
(`cli/config/tests/open_setting.rs`): the precedence ladder, blank values
counting as unset, the reported source for each rung, an unreadable level,
and `OpenKey` refusing a `Trust::Consent` key and a credential such as
`openalex.api_key`.

**File**: `cli/research-cli/src/crossref_contact.rs` (new)

```rust
pub enum CrossrefContact {
    Polite(ContactAddress),
    Anonymous,
    AnonymousBecause(ContactWarning),
}

pub enum ContactWarning {
    Implausible { value: String, source: SettingSource },
    Unreadable(UnreadableLevel),
}

impl CrossrefContact {
    pub fn resolved(environment: &dyn Environment, config: &dyn ConfigAccess)
        -> Self;
    pub fn pool(&self) -> CrossrefPool;
}
```

research-cli already depends on both crates, so it uses the config types
this phase defines (`SettingSource`, `UnreadableLevel`) directly, with no
mirror types.
`resolved` combines `open_value` with `ContactAddress::parse`. `pool()` gives
`CrossrefPool::Polite` for `Polite` and `Public` otherwise. Each
`ContactWarning` has its own `Display`:
- `warning: crossref.mailto from <source> ('<value>') is not a plausible
  address; calling Crossref anonymously without checking lower-precedence
  settings; correct or unset it (a blank value falls through to the next
  setting)`, where `<source>` is `ACCELERATOR_CROSSREF_MAILTO`,
  `config.local.md` or `config.md`;
- `warning: crossref.mailto could not be read from <file> (<reason>); calling
  Crossref anonymously; fix or remove <file> to use its settings`.

Unit tests beside it: each `resolved` outcome and each warning's full text.

**File**: `cli/research-cli/src/main.rs`
**Changes**: print the contact's warning when there is one, and build the
adapters with `ServiceAdapters::for_crossref(contact.pool(), &context)`.

#### 4. Polite pool

`Service::Crossref(Polite)` already exists from phase 3, with its pacing
(102 ms from the previous start, 3 slots) and code `crossref-polite`. This
phase wires it to calls whose contact resolves; the adapters need no change.
`main.rs` builds `ServiceAdapters::for_crossref` with the resolved pool and
passes the same `Service` as `DoiAdapters::service`, which `FetchedDoi`
already carries to rendering.

**File**: `cli/.config/nextest.toml`
**Changes**: the polite rate, `not-an-address` and mixed-pool tests join
the public rate test's `threads-required = "num-test-threads"` override;
the polite concurrency and polite `Retry-After` tests join the
`real-time` group.

**File**: `cli/research-cli/src/cli.rs`
**Changes**: one `FETCH_HELP` sentence saying `crossref.mailto` selects
Crossref's faster pool.

#### 5. Tests

**Files**:
- `cli/research-cli/tests/fetch_doi.rs`;
- `cli/research-cli/tests/support/mod.rs`:
  - `command()` removes `ACCELERATOR_CROSSREF_MAILTO`;
  - `config_with` takes a section name.

**Tests**:
- `research@example.org`:
  - the `User-Agent` carries `mailto:research@example.org`;
  - `queries` holds only `None`.
- Unset → no `mailto`, and the same stdout.
- Each implausible example → exactly one `warning:` line on stderr naming
  the source, an anonymous `User-Agent`, and unchanged stdout and exit
  status.
- An empty or ` ` value → no warning, anonymous.
- Env ` ` with `config.md` set → `config.md`'s address, no warning.
- The precedence ladder across env, `config.local.md` and `config.md`.
- `not-an-address` in env with a valid `config.md` → a warning naming
  `ACCELERATOR_CROSSREF_MAILTO`, anonymous.
- An unreadable `config.local.md` → the unreadable warning naming it,
  anonymous, exit 0.
- A ticket issued by the public pool, re-presented with
  `ACCELERATOR_CROSSREF_MAILTO` set → joins the polite pool afresh, with no
  mismatch.
- Rate, polite pool, real time, measured from `hit_instants` after all
  invocations have joined:
  - the work item's 10 calls across 5 concurrent process slots: all 10 start
    within 1 s of the first;
  - 21 calls: no 11 start within any [t, t+1 s), which 10 calls could never
    show.
- `not-an-address` keeps the public pool: the public-limit assertions hold,
  and every stamp is in `crossref-public/requests.log`.
- Mixed pools: 10 polite and 10 anonymous invocations launched together.
  - Each pool, told apart by its `requests.log`, meets its own limits.
  - The polite pool's 10 start within 1 s of its first, so the anonymous pool
    does not hold it back.
- Retry-After, polite pool: A, B and C look up different DOIs, so the mock
  routes them apart. A is launched alone, and its route is
  `Route::Sequence([429 with Retry-After: 2, 200 recorded body])`, answered
  at once, so A's retry draws no second deferral; A's second request must
  also arrive between A's 429 + 2 s and + 3 s. B and C are launched only
  once A's deferral is
  observable: the test polls
  `Project::service_state(Crossref(Polite), Pacing)` until `not_before_ms`
  is set. From `hit_instants`: B and C each issue a request, none before
  A's 429 + 2 s and each by A's 429 + 3 s, so a hung caller also fails. This
  test is deliberately exempt from the barrier rule, because it measures a
  deferral rather than a rate. It tests the reserve path across processes;
  a caller already asleep on a reservation is covered deterministically by
  the phase 3 `HookedClock` adapter test.
- Concurrency, polite pool: the work item's 10 calls with a 2 s `Delayed`.
  From `hit_instants`, at most 3 overlap, and some instant has 3
  overlapping.

#### 6. `/configure` docs

**File**: `skills/config/configure/SKILL.md`
**Changes**: a `### crossref` section after `### openalex`, mirroring its
shape:
- an intro naming `fetch doi metadata`;
- a settings table with `mailto`;
- the numbered resolution chain: `ACCELERATOR_CROSSREF_MAILTO` →
  `config.local.md` → `config.md`, first non-empty wins;
- one sentence for each difference from openalex's chain: an implausible
  value does not fall through to a lower rung, and `config.md` is read
  whenever no higher rung holds a non-blank value, even when
  `config.local.md` exists;
- the plausibility rule, including visible ASCII with no `(`, `)` or `\`;
- the anonymous fallback and its two warnings;
- a note that the address is sent in the `User-Agent`, never in a URL, and may
  be committed in `config.md`;
- an example YAML block;
- "Recognised keys".

Then `mise run docs:generate`.

**File**: `docs-site/src/content/docs/research.md`
**Changes**: a short "Crossref pools" subsection under Pacing. It gives
each pool's spacing between request starts (210 ms with one in flight;
102 ms with up to three), its state directory, names `crossref.mailto` and
`ACCELERATOR_CROSSREF_MAILTO`, and links to the configure skill's `crossref`
section rather than repeating the resolution chain. The Limits bullet notes
that `crossref.mailto` (3 concurrent) reduces contention across projects.

### Outcome

Phase 5 landed in seven commits. The workspace suite runs 8650 tests, and
`mise run check` and `mise run docs:check` exit 0. The bare `mise run` has
not been run for this phase. Departures from the plan:

- **No unreadable-config CLI test**: an unreadable or malformed
  `config.local.md` fails every queued call with exit 1 before the contact
  is resolved, because the project's `paths.tmp` is read first, as it is
  for arXiv. `ContactWarning::Unreadable` and its wording are pinned by
  unit tests only, as agreed with the plan's author.
- **Mock server fix**: `http-test-support` held its routes lock while
  sleeping out a `Delayed` route, so delayed responses were answered one
  at a time. A new harness test pins concurrent answers; the polite
  concurrency test depends on it.
- **Credential marker**: `ExtraKey::credential` is a `bool`, set through an
  `ExtraKey::credential` constructor. `TokenKeys::declared` names an
  unmarked key "not a declared credential".
- **`CrossrefContact`** also exposes `address()` and `warning()`. `DoiCall`
  carries the contact, and `DoiFetch` gains `contact`. `queued_context`
  takes the already opened `ProjectContext`.
- **Docs**: the configure skill's reference sits inside a fenced block on
  the generated page, so it has no `#crossref` anchor; `research.md` links
  the page and names the section. The admission window in "The fetch
  queue" gives the polite pool's 30.102 s.
- **Manual checks**: Crossref names the pools `polite-single` and
  `public-single` in `x-api-pool`, with `x-rate-limit-limit` 10 and 5 and
  `x-concurrency-limit` 3 and 1. Live calls from a build of this branch
  went to the polite pool with the address in the environment or
  `config.md`, and to the public pool with one warning for
  `not-an-address`. The `crossref` section was checked in the skill source
  and the generated docs page, not through an installed `/configure`.

### Success Criteria

#### Automated Verification

- [x] `cd cli && cargo nextest run -p research -p accelerator-research -p config -p accelerator --features accelerator-research/test-loopback`
- [x] `mise run public-api:update && mise run public-api:check`
- [x] `mise run pup:check`
- [x] `mise run docs:check`
- [x] `mise run check`

#### Manual Verification

- [x] With `crossref.mailto` set to a real address, a live call's response
      carries Crossref's polite-pool header `x-api-pool: polite`, seen with
      `curl -sI -A "accelerator-research/x (mailto:<address>)" https://api.crossref.org/works/10.1145/371920.372066`.
- [x] `/accelerator:configure` shows the `crossref` section.

---

## Phase 6: OpenAlex Search/Lookup Parity Test

### Overview

This phase is independent of the others: it adds tests only, so the TDD red
step is skipped. If the test fails, it has found a real bug, which must be
fixed under its own failing test before this phase merges.

### Changes Required

**Fixtures** (`cli/research-adapters/tests/fixtures/openalex/`, captured live
with `OPENALEX_SELECT`):
- `parity-search.json`: a `/works?filter=title_and_abstract.search:…` page
  whose first result is `10.1145/3448016.3457556`;
- `parity-doi.json` from `/works/doi:10.1145/3448016.3457556`;
- `parity-wid.json` from `/works/W…`.

**File**: `cli/research-cli/tests/fetch_openalex.rs`
**Test**: `search_and_every_form_of_lookup_agree_on_tier_venue_and_signals`.
1. Route the three paths, then run `search`, `lookup 10.1145/3448016.3457556`,
   `lookup https://doi.org/10.1145/3448016.3457556` and `lookup W…`.
2. Take the record for the DOI from each output.
3. Assert identical `tier`, `venue` and `venue_signals`, with `tier` equal to
   `tier-1` and `venue` equal to the fixture's
   `primary_location.source.display_name`.

### Outcome

Phase 6 landed in one commit, and the test passed on its first run, so it
found no bug. The work is `W3176171201`, with venue "ACM SIGMOD
International Conference on Management of Data (SIGMOD)". The search fixture
comes from the query `consistency completeness distributed stream processing
kafka`, which returns this work first of two. A mutated `listed_in` in
`parity-wid.json` made the test fail, so it detects divergence. The bare
`mise run` has not been run for this phase.

After the branch was rebased onto `main` at 1.24.0-pre.112, `mise run check`
exits 0 and phase 5's suites pass (2514 tests). Phase 5's command named the
launcher crate `launcher`; its package name is `accelerator`, and the
command now says so.

### Success Criteria

#### Automated Verification

- [x] `cd cli && cargo nextest run -p accelerator-research --features test-loopback -E 'test(search_and_every_form_of_lookup)'`
- [x] `mise run check`

---

## Phase 7: Web Profile and Finding Outputter Text

### Overview

Rewrite the web profile's metadata and tiering rules so a source cited by DOI
is tiered by its venue, never by `doi.org`. Allow a block from two commands in
the outputter. Text checks are written first, in
`tests/unit/tasks/test_research_structure.py`, and fail against today's text.

### Changes Required

#### 1. Web profile

**File**: `skills/research/profiles/web-profile/SKILL.md`

- **Source Family → Metadata.** For every cited page:
  1. run `fetch web metadata`;
  2. for a resolver URL (`doi.org` or `dx.doi.org`), `WebFetch` it to learn
     its landing host, once per source (an earlier fetch of the same URL in
     this research node counts), and never infer the host otherwise;
  3. run `fetch doi metadata` on the DOI `web metadata` printed. When it
     printed none, or failed, and the cited URL is a resolver URL, run it on
     that URL instead;
  4. a source reaching neither branch is tiered by its own domain.

  Include the fenced calls `accelerator research fetch doi metadata 'DOI'` and
  the `--ticket` form. State that the existing 120000 ms Bash `timeout` and
  the single-quote rule (each `'` written as `%27`) apply to every
  `fetch doi metadata` call, `--ticket` re-presentations included.
- **Landing host.** Define it as the host of the first redirect `doi.org`
  returns, as reported by `WebFetch`'s cross-host redirect. For a declaring
  page that is not a resolver URL, it is that page's own host. Phase 1
  found no gap on v2.1.144, so the text adds no rule for a `WebFetch` that
  returns content without reporting a redirect.
- **Resolver fetches and the fetch boundary.** The `WebFetch` of a resolver
  URL is a fetch of the cited source, not of a URL a fetched page names, so
  the Source Family boundary ("No URL a fetched page tells you to fetch")
  does not forbid it. The landing host comes from the redirect that fetch
  reports, and needs no second fetch. Fetch the landing page itself only to
  read content the note cites from it; that fetch is of the cited source
  too. One `WebFetch` of the resolver URL per source satisfies the rule; an
  earlier fetch of the same URL in the same research node counts.
- **Waiting.**
  - Re-run the same `fetch doi metadata` call at once, with `--ticket` set to
    the latest `waiting` output's ticket, and never change the DOI.
  - Repeat until the status is not `waiting`.
  - After 20 consecutive `waiting` results, handle the call as a printed
    `unavailable`, recording no `reason`. Unlike the arXiv profile, which
    records reason `waiting`, no reason is recorded because the CLI printed
    none.
- **Warnings.** A CLI `warning:` line on stderr never changes the outcome.
  Repeat it once, verbatim, in the researcher's summary, so a misconfigured
  `crossref.mailto` reaches the person running `conduct`.
- **Usage errors.**
  - `E_DOI_TICKET_LIVE` → write no file, and return the `E_*` line verbatim in
    the summary.
  - `E_DOI_TICKET_MALFORMED` or `E_DOI_TICKET_MISMATCH` → correct the call and
    continue.
  - `E_DOI_MALFORMED` → handle the source as for `unavailable`.
- **Metadata block for a source cited by DOI.**
  - The `web metadata` entries come first, then the `doi metadata` entries,
    each copied unchanged in emitted order.
  - Give a template and a worked example whose link is a `https://doi.org/…`
    URL and whose venue is not `doi.org`.
  - A resolver-URL source keeps the resolver URL as its link. A non-resolver
    page keeps its own URL. Both use the combined template.
- **Reputation Tiers.**
  - `doi.org` and `dx.doi.org` are resolvers, never recorded as a venue.
  - Add two explicit rules that override the domain rules:
    1. a printed `venue` (a journal or proceedings title) → tier-1;
    2. a printed `publisher` without a `venue` → tier-2, the publisher as
       venue.
  - Otherwise, use the landing host under the existing domain rules
    (`arxiv.org` → tier-2). With no landing host known, record no venue,
    tier-3, cited by the resolver URL.
  - Give four worked examples, each keeping the resolver URL as its link:
    - `10.1145/371920.372066` → `Proceedings of the 10th international
      conference on World Wide Web`, tier-1;
    - a publisher-only DOI → its publisher, tier-2;
    - `https://doi.org/10.48550/arXiv.1706.03762` → `arxiv.org`, tier-2;
    - a resolver URL `doi.org` cannot redirect, with Crossref `unavailable` →
      no venue, tier-3.
  - Avoid the backticked `` `tier-N (…)` `` form unless citing-once carries it
    (`test_research_structure.py:1525-1551`).

#### 2. Finding outputter

**File**: `skills/research/outputters/finding-outputter/SKILL.md:96-98`
**Changes**: the block rule becomes "Block lines are the `metadata` entries of
the CLI commands the profile names for the cited source. Write them one after
another in the order the profile names, each copied unchanged and in emitted
order." Avoid every citing-once `RULE_SIGNATURES` phrase
(`test_research_structure.py:1636-1680`).

#### 3. Research-topic recovery rows

**File**: `skills/research/research-topic/SKILL.md:619-620`
**Changes**:
- the `lock_contention` row: "an arXiv or DOI fetch waited past its 900 s
  cap … or its service's queue was unusable", naming
  `services/arxiv/contention.log`, `services/crossref-public/contention.log`
  and `services/crossref-polite/contention.log`;
- the `waiting never re-presented` row: "its arXiv or DOI ticket";
- a row for a warning beginning `warning: crossref.mailto`: the warning
  verbatim, then, for an implausible address, point to
  `/accelerator:configure`'s `crossref` section, or, for an unreadable file,
  say to fix or remove the named file. The generic "a warning → the warning
  verbatim" row stays for every other warning.

**File**: `skills/research/research-topic/SKILL.md`, conduct's spawn-summary
step (step 5)
**Changes**: conduct reads every spawn's summary, whether or not the spawn
succeeded, and collects each `warning:` line it repeats. Today it reads
summaries only for failure reasons.

**File**: `skills/research/research-topic/SKILL.md`, the final summary
(step 7.4)
**Changes**: add each distinct collected `warning:` line to the warnings
already printed after the summary sections, each followed by its next step
from the recovery table, so every warning appears in one place.

#### 4. research.md Sources metadata

**File**: `docs-site/src/content/docs/research.md`, Sources metadata
**Changes**: describe the combined block for a source cited by DOI (the
`web metadata` entries, then the `doi metadata` entries), say that
`publisher` comes from `doi metadata`, and say that every `doi`-recorded key
ranks after every `web`-recorded key, so the emitted order is the table
order. Give `publisher` its value form (the name Crossref holds, runs of
whitespace collapsed).

**File**: `docs-site/src/content/docs/research.md`, Granting the fetch
**Changes**: `web-profile` uses the grant for `fetch web metadata` and
`fetch doi metadata`, deferred from phase 4 until the profile calls it.

#### 5. Tests

**File**: `tests/unit/tasks/test_research_structure.py`
- `:1784-1792`: allow fenced blocks matching any of:
  - `accelerator research fetch web metadata '[^']+'`;
  - `accelerator research fetch doi metadata '[^']+'( --ticket \S+)?`;
  - a Markdown template or example block, tagged `markdown`.
- New pins, one per web-profile acceptance criterion:
  - resolver names;
  - the order of the run steps;
  - the four branches with their tiers;
  - the `WebFetch` rule;
  - the `E_DOI_*` handling;
  - the landing-host definition;
  - the four worked examples;
  - the Waiting section's phrases (mirroring the arXiv pins at `:819-874`);
  - the copy-unchanged rule;
  - the template and resolver-link rules;
  - the rule that runs `doi metadata` on any declared DOI;
  - the failed-`web metadata` branch;
  - the timeout and quoting rule for `fetch doi metadata`;
  - the resolver-fetch boundary sentence;
  - the warning-repeat rule.
- A pin on the outputter's multi-command rule.
- Pins on the generalised research-topic rows, replacing any pin on the
  arXiv-only wording; on the specific `crossref.mailto` row and the kept
  generic warning row; and on conduct collecting `warning:` lines from every
  spawn summary and listing each distinct one once.

**File**: `tests/integration/research/test_profile_invocations.py`
**Changes**: `FEWEST_INVOCATIONS["web"]` rises to the new count of fenced
commands. Every one must pass the built guard.

Then `mise run docs:generate`.

### Outcome

Phase 7 landed in five commits: the parser change, the web profile, the
finding outputter, research-topic, and `research.md`.

- **Parser.** `fenced_blocks` in `tasks/shared/skill_parsing.py` returns each
  block's language tag with its first line. It returns plain pairs, since the
  module is pinned to import only `re` and `fnmatch`. The structure test and
  the profile invocation test skip `markdown` blocks with it.
- **Examples.** The publisher-only example is `10.1007/978-1-349-06671-1_23`
  (`Palgrave Macmillan UK`, from the phase 4 book-chapter fixture). The
  combined-block example is `https://doi.org/10.1371/journal.pone.0000308`
  (`PLoS ONE`). Every landing host in the table was checked live against
  `doi.org`'s first redirect.
- **Failed outcome.** The web profile gains a **Failed** outcome for
  `E_DOI_TICKET_LIVE`. It also handles any other `doi metadata` status, any
  unnamed non-zero exit, and a refused call as `unavailable`.
- **Publisher value form.** `publisher` gets its value form in `research.md`
  only. The finding outputter's table is unchanged.

The first bare `mise run` failed only on the CLI coverage floor (lines
91.82%), with every test passing. After `cargo llvm-cov clean --workspace`,
coverage read 94.34%. One rerun of the lane hit a load flake in
`polite_pool_calls_start_no_more_than_ten_a_second` (11 starts in a second);
the test passed 6 of 6 runs alone. The next full `mise run` exited 0.

### Success Criteria

#### Automated Verification

- [x] `uv run pytest tests/unit/tasks/test_research_structure.py`
- [x] `mise run test:integration:research`
- [x] `mise run check`
- [x] `mise run` exits 0 (full local CI mirror, including `docs:check`).

#### Manual Verification

- [x] A reading of the web profile from start to finish gives one unambiguous
      path for each of the four branches.

---

## Phase 8: Closing Manual Run

### Overview

Confirm the fix end to end on the finished plugin.

### Steps

1. Run a web-profile `research-topic conduct` whose brief gives, as sources to
   cite:
   - the two resolver URLs;
   - `https://doi.org/10.48550/arXiv.1706.03762`;
   - the non-resolver page recorded in phase 1,
     `https://www.nature.com/articles/nature14539`.
2. Synthesise the dossier.
3. Check:
   - the arXiv URL's note: venue `arxiv.org`, tier-2;
   - each resolver URL's note: the Crossref venue at tier-1, equal to the tier
     `fetch openalex lookup` reports for the DOI, with the resolver URL kept as
     its link;
   - the page's note: venue `Nature`, tier-1;
   - no entry records `doi.org` as a venue;
   - the dossier shows no tier conflict for the three DOIs;
   - if any of them goes uncited, the check fails.
4. Keep the notes beside the baseline's. For each resolver URL, record whether
   its `web metadata` succeeded or failed. Phase 1 saw both fail with
   HTTP 403 from `dl.acm.org`, so both are expected to take the failed
   branch. If none took the failed branch, record that this was accepted
   rather than forced.
5. Tick the manual acceptance criteria in the work item.

### Outcome

The closing run passed on its first completed attempt, against this branch's
working tree, not a release. The skills loaded with `--plugin-dir`, and
`accelerator` resolved to the debug builds through the dev-launcher
overrides. Its subject was deleted afterwards as test residue, so the work
item's Closing Run section is the record of the run, not notes kept beside
the baseline's.

- Both resolver URLs took the failed `web metadata` branch, as phase 1
  predicted.
- The session must run in `acceptEdits`, as the eval tasks do. Under auto
  mode, the classifier gave no verdict on the researcher's `Write` of its
  finding, and the run wrote nothing.
- The researcher called `doi metadata` before the resolver `WebFetch`, against
  the profile's order. It also cited both ACM DOIs' `api.crossref.org` records
  as extra tier-1 sources. Neither affected a tier.

### Success Criteria

#### Manual Verification

- [x] Every check above passes in one run, and the evidence is recorded in the
      work item.

---

## Testing Strategy

### Unit Tests

- Pacing arithmetic for both spacing origins: reservation, confirmation
  (including a lone caller that wakes on time), hand-back, deferral
  ordering, the `trust` repairs at their boundaries, and the admission
  poll; and the engine's use of that poll.
- The front rule with 1 and 3 slots, with ended tickets mixed in.
- Provenance precedence over join, and own tickets winning.
- Crossref classification for every status class and both transport
  failures, and normalisation for every Crossref type and shape of
  `container-title`.
- Case-insensitive DOI bindings.
- Contact-address plausibility, in research; contact resolution and both
  warning texts, in research-cli.
- The open-setting resolver's ladder, sources, unreadable level and refusal
  of non-open keys.

### Adapter Tests

- The multi-slot file gate under `HookedClock`: slot limits, distinct
  reservations, a deferral landing mid-sleep, deadline charging, reservation
  hand-back, companion locks (busy, held across releases, unopenable,
  absent, and busy behind a degraded serving lock), the factory's
  `PreviousRelease` wiring, and degradation.
- Unlistable foreign directories.
- `JsonCrossrefDecoder` on every fixture and malformed shape.

### Integration Tests

- `cli/research-cli/tests/fetch_doi.rs`: every CLI acceptance criterion, under
  LoggingClock or in real time, plus the undecodable body, pool change,
  unreadable config and letter-case re-presentation.
- The order of `doi`-printed keys after `web`-printed keys, from real output.
- The arXiv CLI suite unchanged, with full-line stderr assertions, plus the
  legacy fixture and cross-family mismatches.
- OpenAlex parity.
- Profile invocations through the built guard.

Real-time tests measure from the mock's `hit_instants` after releasing a
barrier of one held ticket per serving slot that every invocation joined
behind (the polite `Retry-After` test excepted). The liveness tests run
alone (`threads-required = "num-test-threads"`); the wide-slack ones share a
`real-time` group. All report their observed slack on failure.

### Manual Testing Steps

1. The phase 1 baseline (gate).
2. Live `doi metadata` calls on both pools.
3. The phase 8 closing run.

## Performance Considerations

- Crossref calls add at most one request per cited DOI. In the public pool,
  pacing allows at most about 4.7 requests per second project-wide. With one
  slot, real throughput is lower: 1 / (Crossref latency + admission poll +
  queue bookkeeping).
- A waiting Crossref caller notices a free slot within 52 ms (public) or
  25 ms (polite) instead of 100 ms.
- Each send takes `pacing.lock` at least three times (reserve, confirm,
  record the finish), each a short read-modify-write. A late send delays
  the next by the same amount and lateness is never caught up, so it adds
  up across a window; the polite rate test's failure message reports the
  largest per-gap hand-off overhead, so a creep towards the ~9 ms per-gap
  budget shows before it fails.
- The real-time multi-process tests add about 40 s of their own time:

  | Test | Wall time | Runs |
  |---|---|---|
  | public concurrency (10 × 2 s, 1 slot) | ≥ 20 s | overlapped |
  | polite concurrency (10 × 2 s, 3 slots) | ~8 s | overlapped |
  | public and polite `Retry-After` | ~5 s and ~3 s | overlapped |
  | rate, `not-an-address` and mixed pools | ~2 s each | alone |

  Only the ~8 s of liveness tests run alone, each after nextest drains the
  running tests; the ~36 s in the `real-time` group overlaps the rest of
  the suite.

- Listing the other families' queue directories happens only when a presented
  ticket is unknown in its own directory. It reads names, never file contents.
- Not measured: `queue.lock` hold time for join and leave housekeeping at
  Crossref rates, and the unbounded growth of each `requests.log`. Both carry
  over from arXiv's design; revisit if the rate tests show hand-off latency
  near the 102 ms interval.

## Migration Notes

- No migration. The previous release's `arxiv-queue/`, `arxiv-pacing` and
  logs are ignored by the new release, apart from the read-only listing of
  `arxiv-queue/` for provenance. `arxiv.lock` is still taken, as a lock only,
  as the arXiv gate's companion lock.
- A ticket issued before the upgrade gets `E_ARXIV_TICKET_MISMATCH`. The caller
  drops `--ticket` and re-runs, as the arXiv profile already directs for usage
  errors.
- While processes of both releases run in one project, they still keep one
  arXiv connection at a time through `arxiv.lock`, but their pacing state is
  separate, an accepted risk.
- `research.md` gains a note (phase 3) that the old files except
  `arxiv.lock` can be deleted once no older plugin version runs in the
  project; `arxiv.lock` is recreated empty until the companion is retired.
- Retiring the legacy support later deletes the `PreviousRelease` argument,
  which removes both the legacy queue listing and the companion
  `arxiv.lock`. Both are safe once no older release can run in a project.

## References

- Work item: `meta/work/PP-891-tier-doi-links-by-venue-not-resolver.md`
- Codebase research:
  `meta/research/codebase/2026-11-20-PP-891-tier-doi-links-by-venue-not-resolver.md`
- Queue design: `meta/plans/2026-10-06-PP-882-fair-arxiv-fetch-queue.md`
- Metadata keys: `meta/plans/2026-10-24-PP-936-structured-source-metadata.md`
- Queue follow-ups:
  `meta/work/PP-888-fair-arxiv-queue-follow-ups-from-the-pr-148-review.md`
- Crossref access and pools:
  https://www.crossref.org/documentation/retrieve-metadata/rest-api/access-and-authentication/
