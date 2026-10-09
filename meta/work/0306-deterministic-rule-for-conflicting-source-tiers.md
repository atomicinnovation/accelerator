---
type: "work-item"
id: "0306"
title: "Deterministic Rule for Conflicting Source Tiers"
date: "2026-10-09T17:58:47+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "medium"
parent: "work-item:0121"
relates_to: ["work-item:0283", "work-item:0305", "work-item:0311"]
tags: ["research", "deep-research", "composer", "synthesise", "tiers"]
last_updated: "2026-10-09T17:58:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0306: Deterministic Rule for Conflicting Source Tiers

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

When the same source appears in several level notes at different tiers, the
composer has no rule for which tier to cite, and in one run ten composers
resolved conflicts three different ways. Composers and `synthesise` will cite
such a source once at its lowest tier, recording the other tiers in a
suffix, so the outcome is deterministic and the disagreement stays visible.

## Context

The composer is told to "carry each source's tier text verbatim … Never add
a source, and never re-tier one", which assumes each source has one tier. In
a depth-3 `conduct` run (10 pairs, 130 notes), every composer met at least
one conflict, for example:

- the event-driven.io article on Postgres sequence ordering (tier-2 vs
  tier-3);
- `github.com/eugene-khyst/postgresql-event-sourcing` (tier-1 or tier-2 vs
  tier-3);
- psycopg issues #340, #962 and #1091 (tier-1 vs tier-2);
- `brandur.org/notifier` (tier-2 vs tier-3).

Most composers kept the lowest tier, one kept the root note's, and two kept
the tier with a named venue (or the higher). The conflicts come from web
researchers tiering the same URL independently (0311), from the missing
merge rule, and from DOI resolver tiering (0305). This item fixes the merge
rule only.

Tier text already supports a parenthesised suffix (`tier-3 (retracted)`), so
the conflict record follows that form.

## Requirements

- **Same source**: two citations are the same source when they carry the
  same DOI, or the same URL after normalisation — lowercase host, scheme
  dropped, leading `www.` dropped, trailing slash and fragment dropped, path
  and query kept.
- **Merge rule**: a source cited at more than one tier is cited once, at the
  lowest tier (tier-3 < tier-2 < tier-1), suffixed with the other tiers in
  descending order: `tier-3 (also tier-2)`, `tier-3 (also tier-2, tier-1)`.
- **Existing suffixes win**: a `(retracted)` or `(withdrawn)` suffix on any
  citation of the source is kept and comes first, e.g.
  `tier-3 (retracted; also tier-1)`.
- **Recorded venue**: the cited source records the domain or venue that
  accompanied the chosen tier.
- The composer agent definition and the `synthesise` instructions both state
  the rule; `synthesise` applies it to sources that conflict across
  findings, treating an existing `also` suffix as more tiers to merge.
- The finding outputter documents the `also` suffix alongside the existing
  tier suffixes.

## Acceptance Criteria

- [ ] Given notes citing `https://brandur.org/notifier` at tier-2 and
      `brandur.org/notifier/` at tier-3, when a composer writes the finding,
      then the source is cited once as `tier-3 (also tier-2)`.
- [ ] Given notes citing one source at tier-1, tier-2 and tier-3, when a
      composer writes the finding, then it is cited as
      `tier-3 (also tier-2, tier-1)`.
- [ ] Given notes citing one source at tier-1 and `tier-3 (retracted)`, when
      a composer writes the finding, then it is cited as
      `tier-3 (retracted; also tier-1)`.
- [ ] Given two notes citing the same DOI, one as `doi.org/10.x/y` and one
      by publisher URL with the DOI recorded, when a composer writes the
      finding, then they are treated as one source.
- [ ] Given two findings citing one source as `tier-2` and
      `tier-3 (also tier-1)`, when `synthesise` writes the dossier, then the
      source is cited as `tier-3 (also tier-2, tier-1)`.
- [ ] Given the same set of conflicting notes in any order, the composer
      output for that source is identical — the rule does not depend on note
      order or lineage.
- [ ] The finding outputter documents the `also` suffix and its ordering.

## Open Questions

- Do any visualiser or validator parsers expect tier text to be exactly
  `tier-N` or `tier-N (retracted|withdrawn)`? If so they must accept the new
  suffix.

## Dependencies

- Blocked by: none known
- Blocks: none known
- Related: 0305 (DOI resolver tiering), 0311 (web-profile tier drift).

## Assumptions

- DOI-to-publisher-URL matching requires the note to record the DOI; a
  publisher URL with no recorded DOI is matched by URL only.

## Technical Notes

- Composer rule: `agents/composer.md:28-30`.
- Synthesise tier carry-forward:
  `skills/research/research-topic/SKILL.md:460-462`.
- Tier suffix documentation:
  `skills/research/outputters/finding-outputter/SKILL.md:76-77`.

## Drafting Notes

- Lowest-tier-with-record, normalised URL / DOI identity, and splitting
  drift reduction into 0311 were user decisions.
- The `also` wording, descending order, and `retracted`/`withdrawn`-first
  precedence are my choices for a deterministic form; any can change without
  affecting the rule.

## References

- Source: https://github.com/atomicinnovation/accelerator/issues/143
- Related: 0121, 0283, 0305, 0311
