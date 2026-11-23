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
blocked_by: ["work-item:PP-936"]
relates_to: ["work-item:PP-864", "work-item:PP-865", "work-item:PP-892", "work-item:PP-897"]
tags: ["research", "deep-research", "web-profile", "openalex", "tiers"]
last_updated: "2026-11-23T16:13:36+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-891"
aliases: ["0305"]
---

# PP-891: Tier DOI Links by Venue, Not by the [doi.org](<http://doi.org>) Resolver

**Kind**: Bug
**Status**: In Progress
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The same DOI appeared at tier-1 and tier-3 in one dossier. The issue blamed
`fetch openalex` tiering `search` and `lookup` differently, but that does not
reproduce: the CLI tiers all three reported DOIs tier-1 by both paths. The
likely cause is web-profile researchers tiering a `https://doi.org/…` URL by
its domain — `doi.org` is a resolver, not a venue, so peer-reviewed papers
drop to tier-3. Add an `accelerator research fetch doi metadata` command,
queued like arXiv's, that reports the venue and publisher Crossref holds for
a DOI; have the web profile run it on any DOI a cited page declares or its
resolver URL names, and tier the source by that venue; and lock in
search/lookup parity with a regression test.

## Context

In a `research-topic conduct` run on `1.24.0-pre.74`, three DOIs appeared at
tier-1 in some notes and tier-3 in others:

| DOI | One record | Other record |
| -- | -- | -- |
| `10.1109/icdcsw.2005.79` | tier-1, venue ICDCSW | tier-3, no venue |
| `10.1145/371920.372066` | tier-1, venue WWW | tier-3, venue `doi.org` |
| `10.1145/3448016.3457556` | tier-1, venue SIGMOD | tier-3, venue `doi.org` |

Re-checked 2026-10-09: `accelerator research fetch openalex lookup` returns
tier-1 with the correct conference venue for all three, whether given a bare
DOI, a `https://doi.org/` DOI or a W-id; `search` agrees. The raw OpenAlex
API returns the same work and `primary_location` for `/works/doi:` and
`filter=doi:`, and every request path uses the same `OPENALEX_SELECT`. The
field-selection hypothesis does not hold.

A venue of `doi.org` matches the web profile's behaviour: it tiers by
"domain or venue" and records the domain, and `doi.org` is not a recognised
venue. The web profile has no rule that a DOI resolver URL stands for the
publication behind it. The profiles say "Never re-judge a tier", so composers
carry the tier-3 forward and it conflicts with the openalex tier-1 (PP-892).

## Requirements

* The web profile names `doi.org` and `dx.doi.org` as resolvers that are
  never recorded as a venue.
* A new `accelerator research fetch doi metadata '<DOI>'` command:
  * accepts a bare DOI or a `doi.org` / `dx.doi.org` URL;
  * on success prints `{"status":"ok","metadata":{…},"venue":"…"}`, where
    `venue` is the container title (journal or proceedings) Crossref holds
    for the DOI, omitted when Crossref holds none, and `metadata` carries
    only what the DOI registry adds and a page cannot declare — `publisher`
    today — never a key `fetch web metadata` prints; a DOI Crossref holds no
    record for is `ok` with empty `metadata` and no `venue`;
  * prints only metadata keys that follow every key `fetch web metadata` can
    print in canonical order, so the two maps written one after the other
    stay in canonical order;
  * reports a venue, never a tier;
  * calls only Crossref, with no key — identifying itself with a configured
    contact address so Crossref serves it from the polite pool, and calling
    anonymously when none is configured;
  * waits its turn in a project-wide Crossref queue, paced to the limits of
    the pool in use; a call that cannot be served within its 100 s deadline
    prints `{"status":"waiting","source":"crossref","ticket":…,"position":…}`
    for the caller to re-present with `--ticket`, and an unreachable
    Crossref or an exhausted budget prints
    `{"status":"unavailable","source":"crossref","reason":…}`; `ok`,
    `unavailable` and `waiting` all exit 0;
  * exits 2 with `E_DOI_MALFORMED` for a DOI it cannot parse, its only
    non-zero exit.
* A new optional `crossref.mailto` setting holds that contact address,
  resolved from the `ACCELERATOR_CROSSREF_MAILTO` env var, then
  `config.local.md`, then `config.md`. An address that is not a plausible
  email draws a `warning:` and the call goes anonymous; the setting never
  makes a call fail. `/configure` documents the `crossref` section.
* `accelerator research fetch --help` lists the `doi` family and its
  `metadata` verb, and names `doi` among the families `--ticket` serves.
* For every cited page, the web profile:
  * runs `fetch web metadata` first, then `fetch doi metadata` on the DOI it
    printed, or — when it printed none and the cited URL is a resolver URL —
    on that URL; a source reaching neither branch is tiered by its own
    domain, as today;
  * re-presents a `waiting` call with `--ticket` until it settles; only
    `unavailable` leads to the fallback below;
  * writes the entry's metadata block as the `web metadata` entries followed
    by the `doi metadata` entries, each copied unchanged in emitted order,
    following an explicit template and a worked example in the profile;
  * records and tiers the source by the `venue` `doi metadata` prints;
    without one, by the `publisher` it prints; when the call fails, by the
    host its page fetch landed on — never by the resolver;
  * keeps the resolver URL as the entry's link, never replacing it with the
    landing page's URL, so the citing-once rule can still read the DOI from
    it whatever `fetch web metadata` returned.
* The finding outputter's metadata block rule allows entries from more than
  one CLI command for a source, written one after another in the order the
  profile names, each copied unchanged.
* A regression test in `cli/research` asserts that `search` and `lookup` (by
  DOI, by `https://doi.org/` DOI, and by W-id) produce the same tier, venue
  and venue signals for the same work.

## Acceptance Criteria

- [ ] The web profile's text names `doi.org` and `dx.doi.org` as resolvers
      that are never recorded as a venue.
- [ ] The web profile's text directs a researcher to run
      `fetch web metadata` first for every cited page, then
      `fetch doi metadata` on the DOI it printed, or — when it printed none
      and the cited URL is a resolver URL — on that URL; a source reaching
      neither branch is tiered by its own domain.
- [ ] The web profile's text directs a researcher to record and tier a
      source that went through `fetch doi metadata` by the printed `venue`,
      then the printed `publisher`, then the host its page fetch landed on.
- [ ] The web profile's text includes a Waiting section, modelled on the
      arXiv profile's, under which a `waiting` call is re-presented with
      `--ticket` until it settles and only `unavailable` leads to the
      fallback.
- [ ] The web profile's text includes a template for a source cited by DOI,
      with `web metadata` entries first and `doi metadata` entries after,
      and a worked example whose link is a `https://doi.org/…` URL and whose
      venue is not `doi.org`.
- [ ] The web profile's text directs the researcher to keep the resolver URL
      as the link of a source cited by DOI.
- [ ] The finding outputter's metadata block rule allows entries from more
      than one CLI command, written one after another in the order the
      profile names.
- [ ] Given a recorded Crossref response for `10.1145/371920.372066`, when
      `fetch doi metadata '10.1145/371920.372066'` runs, then it exits 0,
      `venue` is the WWW 2001 proceedings title, and `metadata` holds only
      `publisher: ACM`.
- [ ] Given the same recorded response, when the command is given
      `https://doi.org/10.1145/371920.372066` or
      `https://dx.doi.org/10.1145/371920.372066`, then its output is
      identical to the bare-DOI output.
- [ ] Given a recorded Crossref response with a publisher but no container
      title, when the command runs, then it exits 0 with no `venue` and
      `metadata` holding the `publisher`.
- [ ] Given Crossref answers 404, when the command runs, then it exits 0
      and prints `{"status":"ok","metadata":{}}` with no `venue`.
- [ ] Given Crossref cannot be reached, when the command runs, then it
      exits 0 and prints `{"status":"unavailable","source":"crossref",…}`
      with a reason of `upstream_error`.
- [ ] Given Crossref answers 429 with a `Retry-After`, when the command
      runs, then it waits as that header asks before retrying, and other
      callers in the project defer to the same wait.
- [ ] Given the project's Crossref queue cannot serve a call within its
      100 s deadline, when the command runs, then it exits 0 and prints
      `{"status":"waiting","source":"crossref","ticket":…,"position":…}`;
      re-presenting the same call with `--ticket` settles it as `ok` or
      `unavailable`.
- [ ] Given a page whose `web metadata` declares a DOI but whose URL is not
      a resolver URL, when the web profile is followed, then that DOI goes
      through `fetch doi metadata` (profile text check).
- [ ] Given a resolver URL whose landing page refuses the fetch, so
      `web metadata` exits 1, when the web profile is followed, then the
      resolver URL still goes through `fetch doi metadata` (profile text
      check).
- [ ] Given `not-a-doi`, when the command runs, then it exits 2 with
      `E_DOI_MALFORMED`.
- [ ] Given `crossref.mailto` is `research@example.org`, when
      `fetch doi metadata` runs, then the Crossref request's `User-Agent`
      carries `mailto:research@example.org` and no request URL contains the
      address.
- [ ] Given no contact address is configured, when the command runs, then
      the request carries no `mailto` and the output is the same as with
      one.
- [ ] Given `crossref.mailto` is `not-an-address`, when the command runs,
      then it prints one `warning:` line on stderr, calls anonymously, and
      its exit status and stdout are unaffected.
- [ ] Given addresses in the env var, `config.local.md` and `config.md`,
      when the command runs, then it uses the env var's; given only
      `config.local.md` and `config.md`, it uses `config.local.md`'s.
- [ ] `/configure` documents `crossref.mailto`, its precedence and the
      anonymous fallback.
- [ ] A CLI test asserts that every key `fetch doi metadata` can print
      follows every key `fetch web metadata` can print in `RECOMMENDED_KEYS`
      order.
- [ ] `accelerator research fetch --help` names the `doi` family and its
      `metadata` verb.
- [ ] Given recorded OpenAlex fixtures for one work, when the CLI tiers it
      via `search`, `lookup <DOI>`, `lookup https://doi.org/<DOI>` and
      `lookup <W-id>`, then all four records carry identical `tier`, `venue`
      and `venue_signals`.

## Open Questions

None outstanding.

## Dependencies

* Blocked by: none
* Blocks: none known
* Related: PP-936 (done), which defines the metadata keys and canonical
  order both commands emit; PP-892 (done), whose citing-once rule reads a
  DOI from a resolver URL and handles whatever tier conflicts remain once
  this source of them is removed; PP-897, neighbouring domain-class work for
  web tiers, kept separate.

## Assumptions

* The `doi.org`-venue records came from web-profile notes. Inferred from the
  venue string; not confirmed against the original run's notes, which are
  not in this repository.
* A page declares its own DOI, so any DOI `web metadata` reports may go
  through `fetch doi metadata`. Preprint servers are believed to declare
  their own DOIs (arXiv's `10.48550/…` are DataCite DOIs Crossref does not
  hold, so they fall back to `arxiv.org`). Accepted risk: a page declaring
  a DOI that is not its own takes that DOI's venue and tier.
* Accepted risk: the profile's acceptance criteria check its text only, so
  no test demonstrates that the original tier conflict is gone.

## Technical Notes

* Web profile tiering and the metadata fetch:
  `skills/research/profiles/web-profile/SKILL.md`, "Reputation Tiers". The
  profile already holds `Bash(accelerator research fetch *)`, so the new
  command needs no new permission.
* Metadata block rule: `skills/research/outputters/finding-outputter/SKILL.md`,
  "Sources Metadata Block". DOI identity from a resolver URL:
  `skills/research/outputters/citing-once/SKILL.md`, "Carrying a DOI".
* Metadata keys and canonical order: `RECOMMENDED_KEYS` in
  `cli/research/src/sources/metadata.rs`. `fetch web metadata` prints `doi`,
  `published`, `updated` and `accessed`, all before `publisher`; `volume`,
  `issue`, `pages` and `license` would also qualify for `doi metadata`
  later, `issn` and `type` would not.
* `fetch web metadata` takes a `doi` only from what the landing page
  declares (`cli/research/src/sources/web/extract.rs`); publisher sites
  often refuse automated fetches.
* Crossref: `https://api.crossref.org/works/{doi}` returns
  `message.container-title` (a list) and `message.publisher`. Its base URL
  would sit beside the OpenAlex and arXiv ones in
  `cli/research/src/sources/request.rs`, and `Doi` there already parses bare
  and resolver-URL forms.
* Crossref calls go through a project-wide queue like arXiv's, with the
  same `ok` / `unavailable` / `waiting` statuses (all exit 0) and `--ticket`
  re-presentation, paced to the limits of whichever Crossref pool the call
  uses (anonymous, or polite with `crossref.mailto`). A Crossref miss is
  `ok` with empty `metadata` and no `venue`, as an OpenAlex `lookup` miss is
  `ok` with no records; only malformed input exits non-zero
  (`E_DOI_MALFORMED`, exit 2).
* The queue engine is spread over `cli/research/src/sources/queue.rs` (with
  ticket handling in `fetch.rs` and `request.rs`),
  `cli/research-adapters/src/queue.rs` and `contention.rs`, and the
  `--ticket` plumbing in `cli/research-cli`. Crossref is its second
  consumer: generalise it into a service-agnostic engine that arXiv and
  Crossref share, each supplying its own pacing, rather than duplicating
  it — more low-funding, high-volume services like these are likely.
* Crossref's etiquette admits a client to the polite pool when it sends a
  contact address in its `User-Agent` or a `mailto` query parameter; the
  `User-Agent` keeps the address out of URLs, which tend to be logged.
* The `crossref` section mirrors `openalex` in
  `skills/config/configure/SKILL.md`. An address is not a secret, so unlike
  `openalex.api_key_cmd` it is not a consent key, and a team may commit a
  shared address in `config.md`.
* OpenAlex request paths all use `OPENALEX_SELECT` (`request.rs`); venue
  signals come from `venue_signals` in `cli/research/src/sources/openalex.rs`.
  The parity test belongs beside `cli/research/tests/openalex_fetch.rs`,
  which has none today.

## Drafting Notes

* Reframed from an OpenAlex CLI bug to a web-profile tiering bug at the
  user's direction, after the CLI failed to reproduce the issue.
* Kept the CLI parity regression test in scope even though the CLI appears
  correct — it turns the issue's claim into a guarantee.
* OpenAlex data changing between 2026-10-05 and 2026-10-09 was judged
  unlikely for 20-year-old records and not pursued.
* Venue resolution moved from the model into the CLI at the user's
  direction, as a new `doi` fetch family. `web metadata` reports what a page
  declares about itself and `doi metadata` what the DOI registry adds; a
  caller runs both and neither duplicates the other.
* Crossref is the venue source. Copying the tier from
  `fetch openalex lookup` was rejected: it couples the web profile to
  OpenAlex and spends its budget. The landing-host fallback stays with the
  researcher, whose page fetch already knows it, rather than a second
  redirect-follower in the CLI.
* `publisher` goes in `metadata` because it is a PP-936 key; the container
  title is not, so `venue` sits beside the map, as on OpenAlex records.
  Ordering `doi metadata` keys after every `web metadata` key, enforced by a
  test, makes the profile's template correct by construction rather than
  asking the model to sort.
* Falling back to the publisher when Crossref holds no container title
  (books, reports) was judged a better venue than the landing host.
* `web metadata` is unchanged for unreadable landing pages: the citing-once
  rule already reads a DOI from a resolver URL, so keeping that URL as the
  link is enough to preserve identity.
* The Crossref polite pool is opt-in at the user's direction, with the
  anonymous route as the default. The key name `crossref.mailto`, sending it
  in the `User-Agent` rather than the URL, allowing it in team `config.md`,
  and treating a malformed address as anonymous with a warning rather than
  an error were drafting choices.
* The stress test widened the trigger from resolver URLs to any DOI a
  cited page declares, and replaced the `E_DOI_*` exit-1 contract with the
  arXiv-style shared queue so rate limiting waits rather than silently
  falling back to a domain tier.
* Kept separate from PP-897 at the user's direction, though scope grew from
  a profile-text change to a new CLI command.
* Profile and outputter acceptance criteria check the skills' text only.
* Row 1 of the Context table ("tier-3, no venue", attributed to the CLI by a
  researcher) may have a different cause — a misreported `unavailable`
  response or a different identifier — but is unverifiable without the
  run's notes.

## References

* Source: [https://github.com/atomicinnovation/accelerator/issues/142](<https://github.com/atomicinnovation/accelerator/issues/142>)
* Related: PP-22, PP-864, PP-892, PP-897, PP-936
* Crossref REST API: [https://www.crossref.org/documentation/retrieve-metadata/rest-api/](<https://www.crossref.org/documentation/retrieve-metadata/rest-api/>)
