---
name: citing-once
description: The citing-once rule for a research Sources section. Decides
  when two citations are one source and what single entry cites it. Injected
  into the composer by the research-topic conduct verb and read by its
  synthesise verb — not invoked directly.
user-invocable: false
disable-model-invocation: true
---

# Citing Once

Cite every source once. Gather every citation in the notes or findings you
cite from, including two citations of one source in the same note or finding,
group them into sources by **Same source**, then write one Sources entry per
source.

A citation is one Sources entry and the metadata block under it:

```text
- [Title](url) — tier text — venue
  - key: value
```

Tier text is a base tier, `tier-1`, `tier-2` or `tier-3`, optionally
followed by a parenthesised suffix. A citation's tiers are its base tier and
every tier its `also` suffix lists: `tier-3 (also tier-1)` carries `tier-3`
and `tier-1`. Wherever this rule orders or compares citations by tier, it
uses the base tier and ignores the suffix. Tiers rank `tier-3` lowest, then
`tier-2`, then `tier-1`.

## Same source

Two citations are the same source when they carry the same DOI (see
**Carrying a DOI**) or the same URL after normalisation. Matches chain:
citations linked by any sequence of DOI or URL matches are one source.

To normalise a URL, drop its scheme, drop its fragment, lowercase its host,
drop a leading `www.` from the host, and drop the path's trailing slash. Keep
the path and query otherwise unchanged, case included.

| URL A | URL B | Sources |
|---|---|---|
| `https://WWW.Brandur.org/notifier#x` | `brandur.org/notifier/` | one |
| `http://example.org/a` | `https://example.org/a` | one |
| `example.org/a/?id=1` | `example.org/a?id=1` | one |
| `example.org/a?id=1` | `example.org/a?id=2` | two |
| `example.org/A` | `example.org/a` | two |

Normalisation decides identity only; it never rewrites a cited URL.

## Carrying a DOI

A citation carries a DOI through the `doi` key of its block, or when its
normalised URL's host is `doi.org` or `dx.doi.org`: that URL's path, without
its leading slash and percent-decoded, is the DOI. The query and fragment are
never part of it. Compare DOIs case-insensitively, so
`https://dx.doi.org/10.X/Y` and `doi.org/10.x/y` carry one DOI, as do
`doi.org/10.1016/S0140-6736%2820%2930183-5` and
`doi: 10.1016/s0140-6736(20)30183-5`.

## Merge rule

Cite a source at its lowest tier across every tier its citations carry. Add
each other distinct tier once, next-lowest first, in an `also` suffix:
`tier-3 (also tier-2)`, `tier-3 (also tier-2, tier-1)`. Omit the `also`
suffix when the source has no other tier, so two `tier-2` citations merge to
`tier-2`.

## Retraction suffixes

A `retracted` or `withdrawn` mark on any citation of a source is a property of
the source, and the merged tier text keeps it. The suffix lists its parts in
order, `retracted`, then `withdrawn`, then the `also` list, each part present
only when it applies, joined by `; `: `tier-3 (retracted)`,
`tier-3 (retracted; also tier-1)`,
`tier-3 (retracted; withdrawn; also tier-1)`. A retracted or withdrawn source
is always tiered `tier-3`, so a mark never changes the chosen tier.

**Merge rule** and **Retraction suffixes** are the only changes to tier text.
Every tier in the merged tier text comes from a citation. **Metadata merge**,
**Full ties** and **Block order** are the only changes to a metadata block.

## Citation order

Order a source's citations by tier, lowest first, then by venue, then by URL.

- **Venue.** A named venue sorts before a bare domain. Within each group,
  venues sort case-folded, then byte-wise. A citation with no venue sorts
  after every venue.
  - A bare domain is two or more dot-separated labels, each made only of
    letters of either case, digits and hyphens: `example.org`,
    `b.example.org`, `arXiv.org`.
  - Every other venue is a named venue, including one with spaces
    (`J. Chem. Phys.`) and one word (`GitHub`, `arXiv`).
  - So `Example Journal` precedes `example.org`, `a.example.org` precedes
    `b.example.org`, `acta Journal` precedes `Zeta Journal`, and
    `Example Journal` precedes `example journal`.
- **URL.** `https://` before `http://` before no scheme, then byte-wise on
  the URL as recorded. So `https://brandur.org/notifier` precedes
  `brandur.org/notifier/`, and `https://brandur.org/notifier` precedes
  `https://www.brandur.org/notifier/`.

## Representative citation

The first citation in citation order is the representative citation. The
merged entry takes its title, venue and URL exactly as recorded, except where
**Full ties** decides.

## Metadata merge

The merged block carries every key any citation of the source carries. Each
key takes its value from the first citation in citation order that carries
it, except where **Full ties** decides. Never add a key that no citation
carries.

## Full ties

Citations are fully tied when their base tier, venue and URL are identical as
recorded, byte for byte. When the representative citation is fully tied with
others, take the title from among them as the first in case-folded, then
byte-wise order. When a key's first carrier is fully tied with other carriers
of that key, take its value from among them the same way. So `Notifier` wins
over `notifier`, and `accessed: 2026-10-01` over `accessed: 2026-10-02`.

## Block order

List every block you write, merged or carried from a single citation, in
canonical order: the recommended keys in the table's order below, then any
other key in byte-wise order. Copy each value exactly as recorded, never put
into canonical form: no change of case, format or prefix, so `doi: 10.X/Y`
stays `doi: 10.X/Y`.

| Key |
|---|
| `doi` |
| `arxiv_id` |
| `openalex_id` |
| `pmid` |
| `issn` |
| `published` |
| `updated` |
| `accessed` |
| `type` |
| `authors` |
| `publisher` |
| `volume` |
| `issue` |
| `pages` |
| `license` |
| `journal_ref` |
| `primary_category` |
| `oa_status` |
| `pdf_url` |

## Worked example

Three citations of one source:

```text
- [The Notifier Pattern](https://blog.example/notifier/) — tier-2 — blog.example
  - accessed: 2026-10-02
- [Notifier](http://blog.example/notifier) — tier-3 (retracted) — Example Blog
  - published: 2019
- [Notifier](https://www.blog.example/notifier) — tier-1 — blog.example
  - openalex_id: W1
  - doi: 10.example/notifier
```

Citation order puts the `tier-3` citation first, then `tier-2`, then
`tier-1`. The merged entry is:

```text
- [Notifier](http://blog.example/notifier) — tier-3 (retracted; also tier-2, tier-1) — Example Blog
  - doi: 10.example/notifier
  - openalex_id: W1
  - published: 2019
  - accessed: 2026-10-02
```
