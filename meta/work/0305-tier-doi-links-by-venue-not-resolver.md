---
type: "work-item"
id: "0305"
title: "Tier DOI Links by Venue, Not by the doi.org Resolver"
date: "2026-10-09T17:58:47+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "bug"
priority: "medium"
parent: "work-item:0121"
relates_to: ["work-item:0280", "work-item:0306", "work-item:0311"]
tags: ["research", "deep-research", "web-profile", "openalex", "tiers"]
last_updated: "2026-10-09T17:58:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0305: Tier DOI Links by Venue, Not by the doi.org Resolver

**Kind**: Bug
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The same DOI appeared at tier-1 and tier-3 in one dossier. The issue blamed
`fetch openalex` tiering `search` and `lookup` differently, but that does not
reproduce: the CLI tiers all three reported DOIs tier-1 by both paths. The
likely cause is web-profile researchers tiering a `https://doi.org/…` URL by
its domain — `doi.org` is a resolver, not a venue, so peer-reviewed papers
drop to tier-3. Make the web profile tier a DOI by the venue it resolves to,
and lock in search/lookup parity with a regression test.

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
DOI, a `https://doi.org/` DOI or a W-id; `search` agrees. The raw OpenAlex
API returns the same work and `primary_location` for `/works/doi:` and
`filter=doi:`, and every request path uses the same `OPENALEX_SELECT`. The
field-selection hypothesis does not hold.

A venue of `doi.org` matches the web profile's behaviour: it tiers by
"domain or venue" and records the domain, and `doi.org` is not a recognised
venue. The web profile has no rule that a DOI resolver URL stands for the
publication behind it. The profiles say "Never re-judge a tier", so composers
carry the tier-3 forward and it conflicts with the openalex tier-1 (0306).

## Requirements

- The web profile states that a DOI resolver URL (`doi.org`, `dx.doi.org`)
  is not a venue: a source cited by DOI is tiered and recorded by the venue
  it resolves to (journal, conference or publisher), never as `doi.org`.
- The web profile states how to find that venue (mechanism open; see below).
- A regression test in `cli/research` asserts that `search` and `lookup` (by
  DOI, by `https://doi.org/` DOI, and by W-id) produce the same tier, venue
  and venue signals for the same work.

## Acceptance Criteria

- [ ] The web profile names `doi.org` and `dx.doi.org` as resolvers that are
      never recorded as a venue.
- [ ] Given a web researcher citing `https://doi.org/10.1145/371920.372066`,
      when it tiers the source, then the recorded venue is the WWW
      conference (or ACM as publisher), not `doi.org`, and the tier follows
      from that venue.
- [ ] Given recorded OpenAlex fixtures for one work, when the CLI tiers it
      via `search`, `lookup <DOI>`, `lookup https://doi.org/<DOI>` and
      `lookup <W-id>`, then all four records carry identical `tier`, `venue`
      and `venue_signals`.

## Open Questions

- How should a web researcher resolve a DOI's venue?
  - From the landing page WebFetch redirects to (e.g. `dl.acm.org`),
    tiering by that publisher's domain.
  - By calling `accelerator research fetch openalex lookup <DOI>` and
    copying the CLI's tier — permitted by the research guard already, but it
    couples the web profile to OpenAlex and spends openalex budget.
- Does row 1 ("`lookup`: tier-3, no venue", attributed to the CLI by a
  researcher) have a different cause? Unverifiable without the run's notes;
  possibly a misreported `unavailable` response or a different identifier.

## Dependencies

- Blocked by: none known
- Blocks: none known
- Related: 0306, which handles whatever conflicts remain once this source of
  them is removed.

## Assumptions

- The `doi.org`-venue records came from web-profile notes. Inferred from the
  venue string; not confirmed against the original run's notes, which are
  not in this repository.

## Technical Notes

- Web profile tiering: `skills/research/profiles/web-profile/SKILL.md`,
  "Reputation Tiers".
- OpenAlex request paths: `cli/research/src/sources/request.rs:491-499`, all
  with `OPENALEX_SELECT`.
- Venue signals: `cli/research/src/sources/openalex.rs:83`.

## Drafting Notes

- Reframed from an OpenAlex CLI bug to a web-profile tiering bug at the
  user's direction, after the CLI failed to reproduce the issue.
- Kept the CLI parity regression test in scope even though the CLI appears
  correct — it turns the issue's claim into a guarantee.
- OpenAlex data changing between 2026-10-05 and 2026-10-09 was judged
  unlikely for 20-year-old records and not pursued.

## References

- Source: https://github.com/atomicinnovation/accelerator/issues/142
- Related: 0121, 0280, 0306, 0311
