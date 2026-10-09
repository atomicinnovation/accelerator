---
type: "work-item"
id: "0311"
title: "Reduce Web-Profile Tier Drift with Canonical Domain Classes"
date: "2026-10-09T17:58:47+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "low"
parent: "work-item:0121"
relates_to: ["work-item:0305", "work-item:0306"]
tags: ["research", "deep-research", "web-profile", "tiers"]
last_updated: "2026-10-09T17:58:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0311: Reduce Web-Profile Tier Drift with Canonical Domain Classes

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

Web researchers tier each URL independently by judging venue identity, so
the same blog or GitHub repository drifts between tier-2 and tier-3 across
notes in one run. Add a canonical domain-class table to the web profile so
independent researchers reach the same tier for the same kind of source.

## Context

In a depth-3 `conduct` run, every composer met tier conflicts originating in
web researchers judging the same source differently, for example
`github.com/eugene-khyst/postgresql-event-sourcing` (tier-1 or tier-2 vs
tier-3), the event-driven.io article (tier-2 vs tier-3), and
`brandur.org/notifier` (tier-2 vs tier-3). The composer merge rule (0306)
makes the outcome deterministic, but each conflict still lands at the lowest
tier with an `also` suffix; fewer conflicts at the source means fewer of
both.

## Requirements

- The web profile's Reputation Tiers section carries this table, refining
  the venue-identity rule: a source matching a class takes that class's
  tier; an unlisted source is still judged by venue identity.

  | Domain class | Tier | Condition |
  |---|---|---|
  | Project's own docs site, repo README or docs | tier-1 | only about that same project |
  | Source code in the project's own repo | tier-1 | only about that project's behaviour |
  | Maintainer comment in the project's issue tracker or mailing list | tier-1 | author is a listed maintainer or committer |
  | Other issue-tracker or mailing-list content | tier-3 | — |
  | Third-party GitHub repo (example or sample project) | tier-3 | about anything other than itself |
  | Vendor or company engineering blog | tier-2 | — |
  | Recognised practitioner reference | tier-2 | named in the profile, not judged ad hoc |
  | Personal blog, Medium, dev.to, Substack | tier-3 | unless named as a recognised practitioner |
  | Q&A sites (Stack Overflow) | tier-3 | — |

- The profile names its recognised practitioners in an explicit list; a
  personal site not on the list is tier-3.

## Acceptance Criteria

- [ ] The web profile's Reputation Tiers section contains the domain-class
      table above, with each class mapped to exactly one tier under its
      stated condition.
- [ ] The profile states that a matching class overrides an ad hoc venue
      judgement, and that unlisted sources fall back to venue identity.
- [ ] The profile has an explicit recognised-practitioner list and states
      that personal sites not on it are tier-3.
- [ ] Given `github.com/eugene-khyst/postgresql-event-sourcing` cited for a
      claim about PostgreSQL, when a web researcher tiers it, then the table
      yields tier-3 (third-party repo about something other than itself).

## Open Questions

- Who is on the recognised-practitioner list at launch, and how does it
  grow? An empty list is a valid start: every personal site is tier-3.

## Dependencies

- Blocked by: none known
- Blocks: none known
- Related: 0306 (composer tier-conflict rule), 0305 (DOI resolver tiering).

## Assumptions

- A prose table applied by the model reduces drift noticeably. Not
  measured.

## Technical Notes

- Web profile tiers: `skills/research/profiles/web-profile/SKILL.md`,
  "Reputation Tiers".

## Drafting Notes

- Split out of 0306 at the user's direction; it has no GitHub issue of its
  own.
- The mapping is my proposal, accepted by the user as the starting point;
  tuning individual rows is expected during implementation.
- The mapping stays in profile prose rather than a CLI-applied data file, at
  the user's direction.

## References

- Source: https://github.com/atomicinnovation/accelerator/issues/143
- Related: 0121, 0305, 0306
