---
type: "work-item"
id: "0306"
title: "Deterministic Rule for Citing a Source Once"
date: "2026-10-09T17:58:47+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "ready"
kind: "story"
priority: "high"
parent: "work-item:0121"
blocks: ["work-item:0281"]
blocked_by: ["work-item:0348"]
relates_to: ["work-item:0280", "work-item:0283", "work-item:0284", "work-item:0305", "work-item:0311"]
tags: ["research", "deep-research", "composer", "synthesise", "tiers"]
last_updated: "2026-10-24T09:02:51+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-892"
---

# 0306: Deterministic Rule for Citing a Source Once

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

As a reader of a research finding or dossier (`synthesis.md`), I want each
source cited once at a deterministic tier, so that the same evidence always
reads the same way and disagreement between tiers stays visible.

When the same source appears in several level notes at different tiers, the
composer has no rule for which tier to cite, and in one run ten composers
resolved conflicts three different ways. Composers and `synthesise` will cite
every source once, at its lowest tier (the weakest, tier-3 before tier-2
before tier-1), recording any other tiers in a suffix. The item also
defines when two citations are the same source, and which citation's title,
venue, URL and metadata the merged entry keeps.

## Context

The composer is told to "carry each source's tier text verbatim … Never add
a source, and never re-tier one", which assumes each source has one tier. In
a depth-3 `conduct` run (10 pairs, 130 notes), every composer met at least
one conflict. A pair is one focus area researched through one source
profile, composed by its own composer; depth is how many levels of notes
0283's tree grows. For example:

- the event-driven.io article on Postgres sequence ordering (tier-2 vs
  tier-3);
- `github.com/eugene-khyst/postgresql-event-sourcing` (tier-1 or tier-2 vs
  tier-3);
- psycopg issues #340, #962 and #1091 (tier-1 vs tier-2);
- `brandur.org/notifier` (tier-2 vs tier-3).

Most composers kept the lowest tier, one kept the root (level-1) note's, and
two kept the tier that came with a named venue. The conflicts come from web
researchers tiering the same URL independently (0311), from the missing
rule for citing a source once, and from DOI resolver tiering (0305). This
item adds that rule only.

Tier text already supports a parenthesised suffix (`tier-3 (retracted)`), so
the conflict record follows that form.

## Requirements

The requirements below, from **Same source** to **Full ties**, are together
the **citing-once rule**. **Merge rule** and **Retraction suffixes** are its
tier-text parts. Wherever the rule orders or compares citations by tier, it
uses the base tier (`tier-3`) and ignores any suffix.

- **Same source**: two citations are the same source when they carry the
  same DOI, or the same URL after normalisation — lowercase host, scheme
  dropped, leading `www.` dropped, fragment dropped, the path's trailing
  slash dropped, path and query otherwise kept. Matches chain: citations
  linked by any sequence of DOI or URL matches are one source.
- **Carrying a DOI**: a citation carries a DOI through its `doi` source
  metadata, or when its URL is on `doi.org` or `dx.doi.org`, whose path is
  the DOI. DOIs compare case-insensitively. Composers and `synthesise` read
  the same metadata, from notes and findings respectively.
- **Merge rule**: citations that are one source are cited once, at the
  lowest tier (tier-3 < tier-2 < tier-1), suffixed with each other distinct
  tier once, next-lowest first: `tier-3 (also tier-2)`,
  `tier-3 (also tier-2, tier-1)`. The suffix is omitted when there is no
  other tier.
- **Retraction suffixes**: a `(retracted)` or `(withdrawn)` suffix on any
  citation of the source is kept and comes first, e.g.
  `tier-3 (retracted; also tier-1)`. When both appear, both are kept,
  `retracted` before `withdrawn`: `tier-3 (retracted; withdrawn; also tier-1)`.
  Retraction and withdrawal are properties of the source: once any citation
  carries one, the merged citation carries it. Because 0280 tiers every
  retracted or withdrawn record tier-3, they never change the chosen tier.
- **Citation order**: a source's citations are ordered by tier, lowest
  first, then by venue, then by URL. Venues order a named venue
  (`Example Journal`) before a bare domain, then case-folded, then
  byte-wise, and a citation with no venue sorts after every venue. A bare
  domain is a venue of two or more dot-separated labels, each made only of
  letters of either case, digits and hyphens (`example.org`,
  `b.example.org`, `arXiv.org`). Every other venue is a named venue,
  including one with spaces (`J. Chem. Phys.`) and a one-word venue
  (`GitHub`, `arXiv`). URLs order `https://` before `http://` before no
  scheme, then byte-wise.
- **Representative citation**: the first citation in citation order. The
  merged citation takes its title, venue and URL as recorded, except as
  **Full ties** decides; normalisation decides identity only and never
  rewrites a cited URL.
- **Metadata merge**: the merged citation carries every source metadata key
  any of its citations carries, each with the value from the first citation
  in citation order that carries that key, except as **Full ties** decides.
  Keys are ordered as 0348 orders a metadata block.
- **Full ties**: when citations tie on tier, venue and URL, the title, and
  each metadata value among the tied citations that carry that key, are
  taken separately as the first value in case-folded, then byte-wise order,
  so ISO dates such as `accessed` keep the earliest.
- The composer agent definition and the `synthesise` instructions both state
  every part of the citing-once rule. `synthesise` applies it to every
  citation across the findings it reads, including duplicates within one
  finding, treating an existing `also` suffix as more tiers to merge.
- The composer's verbatim-tier instruction is narrowed, not removed: every
  tier in the merged tier text, chosen or suffixed, comes from a note, and
  **Merge rule** and **Retraction suffixes** are the only transformations a
  composer applies to tier text.
- The finding outputter documents the `also` suffix alongside the existing
  tier suffixes.
- `synthesise` cites a source with exactly one citation across all the
  findings it reads with its tier text unchanged.

## Acceptance Criteria

"Cited once as X" means the finding's or dossier's Sources section holds
exactly one entry for the source, whose tier text is X; venue, URL, title
and metadata assertions refer to that entry.

- [ ] Given notes citing `https://brandur.org/notifier` at tier-2 and
      `brandur.org/notifier/` at tier-3, when a composer writes the finding,
      then the source is cited once as `tier-3 (also tier-2)`.
- [ ] Given notes citing one source at tier-1, tier-2 and tier-3, when a
      composer writes the finding, then it is cited as
      `tier-3 (also tier-2, tier-1)`.
- [ ] Given two notes citing `brandur.org/notifier` at tier-2, when a
      composer writes the finding, then it is cited once as `tier-2`.
- [ ] Given notes citing one source at tier-1 and `tier-3 (retracted)`, when
      a composer writes the finding, then it is cited as
      `tier-3 (retracted; also tier-1)`.
- [ ] Given notes citing one source at tier-3, tier-2 and tier-2, when a
      composer writes the finding, then it is cited as `tier-3 (also tier-2)`.
- [ ] Given notes citing one source as `tier-3 (retracted)` with venue
      `example.org` and as `tier-3` with venue `Example Journal`, when a
      composer writes the finding, then it is cited as `tier-3 (retracted)`
      with venue `Example Journal`.
- [ ] Given notes citing one source at tier-1, `tier-3 (withdrawn)` and
      `tier-3 (retracted)`, when a composer writes the finding, then it is
      cited as `tier-3 (retracted; withdrawn; also tier-1)`.
- [ ] Given notes citing `doi.org/10.x/y` at tier-3 and a publisher URL
      with source metadata `doi: 10.x/y` at tier-1, when a composer writes
      the finding, then the source is cited once as `tier-3 (also tier-1)`
      with URL `doi.org/10.x/y`.
- [ ] Given notes citing `doi.org/10.x/y` at tier-3 and
      `https://dx.doi.org/10.X/Y` at tier-1, when a composer writes the
      finding, then the source is cited once as `tier-3 (also tier-1)`.
- [ ] Given notes citing `doi.org/10.x/y` at tier-1, `pub.example/p` with
      source metadata `doi: 10.x/y` at tier-2, and
      `https://www.pub.example/p/` with no metadata at tier-3, when a
      composer writes the finding, then the source is cited once as
      `tier-3 (also tier-2, tier-1)`.
- [ ] Given three findings citing `doi.org/10.x/y` at tier-1,
      `pub.example/p` with source metadata `doi: 10.x/y` at tier-2, and
      `https://www.pub.example/p/` with no metadata at tier-3, when
      `synthesise` writes the dossier, then the source is cited once as
      `tier-3 (also tier-2, tier-1)`.
- [ ] Given notes citing `pub.example/a` at tier-3 with `doi: 10.x/y` and at
      tier-1 with `doi: 10.x/z`, when a composer writes the finding, then it
      is cited once as `tier-3 (also tier-1)` carrying `doi: 10.x/y`.
- [ ] Given notes citing one source at tier-3 with source metadata
      `openalex_id: W1` listed before one at tier-1 with `doi: 10.x/y`, when
      a composer writes the finding, then the cited source's block lists
      `doi` before `openalex_id`.
- [ ] Given notes citing one source at tier-3 with source metadata
      `openalex_id: W1` and at tier-1 with `doi: 10.x/y` and
      `openalex_id: W2`, when a composer writes the finding, then the cited
      source carries `doi: 10.x/y` and `openalex_id: W1`.
- [ ] Given notes citing one source at tier-3 with no `published`, at tier-1
      with venue `Example Journal` and `published: 2019`, and at tier-1 with
      venue `example.org` and `published: 2020`, when a composer writes the
      finding, then the cited source carries `published: 2019`.
- [ ] Given notes citing one source at tier-3 titled `Notifier` with venue
      `Example Journal` and at tier-3 titled `The Notifier Pattern` with
      venue `example.org`, when a composer writes the finding, then the
      cited title is `Notifier`.
- [ ] Given notes citing one source at tier-3 with venues `example.org` and
      `Example Journal`, when a composer writes the finding, then the
      recorded venue is `Example Journal`.
- [ ] Given notes citing one source at tier-3 with venues `b.example.org`
      and `a.example.org`, when a composer writes the finding, then the
      recorded venue is `a.example.org`.
- [ ] Given notes citing one source at tier-3 with venues `Zeta Journal`
      and `acta Journal`, when a composer writes the finding, then the
      recorded venue is `acta Journal`.
- [ ] Given notes citing one source at tier-3 with venues `example journal`
      and `Example Journal`, when a composer writes the finding, then the
      recorded venue is `Example Journal`.
- [ ] Given two findings citing one source as `tier-2` and
      `tier-3 (also tier-1)`, when `synthesise` writes the dossier, then the
      source is cited as `tier-3 (also tier-2, tier-1)`.
- [ ] Given two findings citing `https://www.brandur.org/notifier` at tier-2
      and `brandur.org/notifier/` at tier-3, when `synthesise` writes the
      dossier, then the source is cited once as `tier-3 (also tier-2)`.
- [ ] Given two findings citing one source as `tier-3 (retracted; also
      tier-2)` and `tier-2 (also tier-1)`, when `synthesise` writes the
      dossier, then it is cited as `tier-3 (retracted; also tier-2, tier-1)`.
- [ ] Given two findings citing `doi.org/10.x/y` at tier-3 and a publisher
      URL with source metadata `doi: 10.x/y` at tier-1, when `synthesise`
      writes the dossier, then the source is cited once as
      `tier-3 (also tier-1)` with URL `doi.org/10.x/y`.
- [ ] Given two findings citing one source at tier-1 with venue
      `Example Journal` and `openalex_id: W2`, and at tier-3 with venue
      `example.org` and `openalex_id: W1`, when `synthesise` writes the
      dossier, then it is cited as `tier-3 (also tier-1)` with venue
      `example.org` and `openalex_id: W1`.
- [ ] Given one finding citing a source as `tier-3 (also tier-1)` and no
      other finding citing it, when `synthesise` writes the dossier, then the
      source is cited as `tier-3 (also tier-1)`.
- [ ] Given a fixture of level notes citing the sources below, with their
      paths listed to the composer in three orders (root note first, then
      the others in 0283's node-path order; the others in node-path order,
      then the root note; root note first, then the others in reverse
      node-path order) and composed three
      times per order, then all nine findings cite each source with the
      expected tier text and the representative citation's venue and URL.
      The root note carries each source's strongest tier (closest to
      tier-1); child notes carry
      the lower ones:

      - event-driven.io ordering article at tier-2 (root) and tier-3 →
        `tier-3 (also tier-2)`;
      - `eugene-khyst/postgresql-event-sourcing` at tier-1 (root) and
        tier-2, both as
        `https://github.com/eugene-khyst/postgresql-event-sourcing` with
        venue `github.com`, and at tier-3 twice — once as
        `github.com/eugene-khyst/postgresql-event-sourcing` with venue
        `github.com`, once as
        `https://github.com/eugene-khyst/postgresql-event-sourcing/` with
        venue `GitHub` → `tier-3 (also tier-2, tier-1)` at venue `GitHub`
        and URL `https://github.com/eugene-khyst/postgresql-event-sourcing/`;
      - psycopg #340, #962 and #1091, three sources each at tier-1 and
        tier-2 → `tier-2 (also tier-1)` each;
      - `brandur.org/notifier` as `https://brandur.org/notifier/` at tier-2
        (root),
        `brandur.org/notifier/` at tier-3 and `https://brandur.org/notifier`
        at tier-3, all with one venue → `tier-3 (also tier-2)` at URL
        `https://brandur.org/notifier`;
      - one retracted work at tier-1 and `tier-3 (retracted)` →
        `tier-3 (retracted; also tier-1)`.

      Sources other than eugene-khyst and `brandur.org/notifier` use one URL
      and one venue throughout, so only their tier text is under test. The
      fixture is a one-off acceptance check, not a retained eval.
- [ ] Given two findings citing `brandur.org/notifier` as `tier-2` and
      `tier-3 (also tier-1)`, presented in both orders and synthesised three
      times per order, then all six dossiers cite it as
      `tier-3 (also tier-2, tier-1)`.
- [ ] Given notes citing `https://brandur.org/notifier` and
      `brandur.org/notifier/`, both at tier-2 with the same venue, when a
      composer writes the finding, then the cited URL is
      `https://brandur.org/notifier`.
- [ ] Given notes citing one source at tier-1 with venue `Example Journal`
      and at tier-3 with venue `example.org`, when a composer writes the
      finding, then it is cited as `tier-3 (also tier-1)` with recorded venue
      `example.org`.
- [ ] Given two findings citing one source as `tier-3 (retracted)` and
      `tier-2 (also tier-1)`, when `synthesise` writes the dossier, then the
      source is cited as `tier-3 (retracted; also tier-2, tier-1)`.
- [ ] Given, for each row below, a separate composition of two notes citing
      the row's URLs at tier-2, when a composer writes the finding, then it
      holds one citation or two as shown:

      | URL A                                | URL B                   | Sources |
      |--------------------------------------|-------------------------|---------|
      | `https://WWW.Brandur.org/notifier#x` | `brandur.org/notifier/` | one     |
      | `http://example.org/a`               | `https://example.org/a` | one     |
      | `example.org/a?id=1`                 | `example.org/a?id=2`    | two     |
      | `example.org/A`                      | `example.org/a`         | two     |
      | `example.org/a/?id=1`                | `example.org/a?id=1`    | one     |
- [ ] Given notes citing one source at tier-2 as `http://example.org/a` and
      `example.org/a` with one venue, when a composer writes the finding,
      then the cited URL is `http://example.org/a`.
- [ ] Given notes citing one source at tier-2 as
      `https://www.brandur.org/notifier/` and `https://brandur.org/notifier`
      with one venue, when a composer writes the finding, then the cited URL
      is `https://brandur.org/notifier`.
- [ ] The finding outputter documents the `also` suffix with at least the
      examples `tier-3 (also tier-2, tier-1)` and
      `tier-3 (retracted; withdrawn; also tier-1)`.
- [ ] Given two tier-2 notes citing `brandur.org/notifier` with one venue,
      one with `accessed: 2026-10-01` and one with `accessed: 2026-10-02`,
      when a composer writes the finding, then the cited source carries
      `accessed: 2026-10-01`.
- [ ] Given notes citing one source at tier-3 with no venue and titled `A`,
      and at tier-3 with venue `example.org` and titled `B`, when a composer
      writes the finding, then the cited title is `B` and the recorded venue
      is `example.org`.
- [ ] Given two findings citing one source as `tier-3 (withdrawn; also
      tier-2)` and `tier-3 (retracted; also tier-1)`, when `synthesise`
      writes the dossier, then it is cited as
      `tier-3 (retracted; withdrawn; also tier-2, tier-1)`.
- [ ] Given two tier-2 notes citing `brandur.org/notifier` with one venue,
      titled `notifier` and `Notifier`, when a composer writes the finding,
      then the cited title is `Notifier`.
- [ ] Given notes citing `pub.example/a` with `doi: 10.x/y` and
      `pub.example/b` with `doi: 10.x/z`, both at tier-2, when a composer
      writes the finding, then they are cited as two sources.
- [ ] Given one finding written before this rule shipped, citing
      `brandur.org/notifier` twice, at tier-2
      and tier-3, when `synthesise` writes the dossier, then it is cited
      once as `tier-3 (also tier-2)`.
- [ ] The composer agent definition and the `synthesise` instructions each
      state every part of the citing-once rule, with at least the example
      `tier-3 (also tier-2, tier-1)`, and the composer definition states
      that every tier in merged tier text comes from a note and that
      **Merge rule** and **Retraction suffixes** are the only changes it
      makes to tier text.

## Open Questions

None; review 1 settled the remaining questions (see Drafting Notes).

## Dependencies

- Blocked by: 0348, which adds an optional `key: value` list under each
  source line and the `doi` key **Carrying a DOI** reads.
- Blocks: 0281, whose `report` Sources section and coverage judgement both
  apply the citing-once rule over the findings a report cites; 0281
  inherits 0348 through this item.
- Relies on the composer agent and level notes as shipped by 0283; a change
  to the composer's inputs changes where the citing-once rule applies.
- Relies on the `tier-3 (retracted)` and `tier-3 (withdrawn)` suffixes as
  shipped by 0280; a change to that vocabulary changes **Retraction
  suffixes**.
- 0305 must keep a source's `doi` metadata when it tiers DOI-cited sources
  by resolved venue; without it, those citations match by URL only.
- Updates 0121: none needed. Its contract derives `synthesis.md`'s Sources
  from the findings' tiers, and the merged form is still derived from them.
- Consumers of the `also` suffix: 0281 and 0284 (visualiser set detail
  page). Both must accept the full suffix form, including the semicolon-joined
  `tier-3 (retracted; withdrawn; also tier-1)`. No current CLI or visualiser
  code parses tier text, so nothing shipped needs changing to accept it.
- Related: 0283 (level-note trees, several notes per pair, where the
  conflicts show up), 0305 (DOI resolver tiering), 0311 (web-profile tier
  drift), 0160 and 0161 (the Inspect eval tier, the future home of a
  retained determinism eval for the citing-once rule).

## Assumptions

- Notes and findings written before 0348 carry no source metadata; their
  citations carry a DOI only through a `doi.org` or `dx.doi.org` URL.

## Technical Notes

- Composer rule: `agents/composer.md:28-30`.
- Synthesise tier carry-forward:
  `skills/research/research-topic/SKILL.md:645-649`.
- Tier suffix documentation:
  `skills/research/outputters/finding-outputter/SKILL.md:76-77`.
- Tier text is produced only by `Tier`'s `Display` in
  `cli/research/src/sources/record.rs`; nothing parses it back.

## Drafting Notes

- Citing the lowest tier with the other tiers recorded in a suffix,
  normalised URL / DOI identity, splitting drift
  reduction into 0311, and named-venue-then-lexicographic venue choice were
  user decisions.
- The `also` wording, next-lowest-first order, `retracted`/`withdrawn`-first
  precedence, and `retracted` before `withdrawn` when both appear are my
  choices for a deterministic form; any can change without affecting the
  rule.
- The named-venue definition (a venue that is not a bare domain) is mine,
  added to make the venue ordering in **Citation order** testable.
- Review 1 (2026-10-24) settled, as user decisions: the bounded nine-run
  determinism check, DOI carrying and chained matching, merging same-tier
  duplicates, retraction as a source property, narrowing the composer's
  verbatim-tier instruction, and the case-folded-then-byte-wise tie-break.
- Review 1 pass 2 (2026-10-24): the user chose structured source metadata
  over URL-borne DOIs only, split into its own story that blocks this one;
  metadata merging follows the representative citation; the determinism
  fixture is a one-off acceptance check, not an eval, accepting no
  regression protection until the Inspect tier (0160, 0161) exists.
- Review 1 pass 3 (2026-10-24): the user approved taking each metadata key
  from the first citation in citation order that carries it, which
  generalises the representative citation into a full citation order.
- Review 1 pass 4 (2026-10-24): the user approved breaking full ties per
  field by case-folded then byte-wise order, and `synthesise` applying the
  citing-once rule to every citation, including duplicates within one
  older finding.

## References

- Source: https://github.com/atomicinnovation/accelerator/issues/143
- Related: 0121, 0280, 0281, 0283, 0284, 0305, 0311, 0348
