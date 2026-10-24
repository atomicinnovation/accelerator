---
type: "work-item"
id: "0349"
title: "Fill Bibliographic Source Metadata"
date: "2026-10-24T11:25:22+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "low"
parent: "work-item:0121"
blocked_by: ["work-item:0348"]
relates_to: ["work-item:0280"]
external_id: "PP-924"
tags: ["research", "deep-research", "sources", "metadata", "cli"]
last_updated: "2026-10-24T11:36:49+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0349: Fill Bibliographic Source Metadata

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a future consumer of the research corpus, I want each cited source's
metadata block to carry its bibliographic and source-specific details, so
that sources can be cited, filtered and audited without re-fetching them.

0348 defines the block, its recommended keys and their value forms, but
fills only the identifier and date keys. This story makes the CLI fill the
remaining recommended keys for OpenAlex and arXiv records and in the web
metadata command, as far as each source allows.

## Context

0348 left `type`, `authors`, `publisher`, `volume`, `issue`, `pages`,
`license`, `pmid`, `issn`, `journal_ref`, `primary_category`, `oa_status`
and `pdf_url` unfilled, leaving their value forms to this story. No
work item reads these keys yet; the aim is a complete corpus for consumers
to come.

The sources differ in what they give:

- OpenAlex holds most of these keys, but its author names are single
  display strings, its ids are URLs, and its `select` takes only top-level
  fields.
- The arXiv Atom feed gives authors as single strings, `journal_ref`,
  `primary_category` and a PDF link, but no licence.
- Web pages declare them across Highwire Press `citation_*` tags, schema.org
  JSON-LD and Dublin Core; only schema.org gives structured family and given
  names, and a page's ISSNs never say which is the linking ISSN (ISSN-L).

## Requirements

- **OpenAlex**: `type` from `type`, mapped below; `authors` from
  `authorships[].author.display_name` in `author_position` order;
  `publisher` from `primary_location.source.host_organization_name`;
  `volume`, `issue` and `pages` from `biblio`; `license` and `pdf_url` from
  the chosen location; `pmid` from `ids.pmid` with its URL prefix stripped;
  `issn` from `primary_location.source.issn_l`; `oa_status` from
  `open_access.oa_status`.
- **arXiv**: `type` is `article`; `authors` from each `author/name`;
  `journal_ref` from `arxiv:journal_ref`; `primary_category` from
  `arxiv:primary_category/@term`; `pdf_url` from `link[@title="pdf"]/@href`;
  `license` from the `arXivRaw` record if it carries one.
- **Web metadata command**, each key from the first scheme below declaring
  a usable value, in 0348's precedence:
  - `type`: schema.org `@type`, then Highwire inference, else `webpage`;
  - `authors`: schema.org `author`, `citation_author`, `DC.creator`;
  - `publisher`: `citation_publisher`, schema.org `publisher.name`,
    `DC.publisher`;
  - `volume`, `issue`: `citation_volume`/`citation_issue`, schema.org
    `volumeNumber`/`issueNumber`, `DC.citation.volume`/`DC.citation.issue`;
  - `pages`: `citation_firstpage`/`citation_lastpage`, schema.org
    `pageStart`/`pageEnd`, `DC.citation.spage`/`DC.citation.epage`;
  - `license`: schema.org `license`, `DCTERMS.license`;
  - `pdf_url`: `citation_pdf_url`, schema.org `encoding.contentUrl`;
  - `pmid`: `citation_pmid`;
  - `issn` is not filled.
- **OpenAlex location**: `license` and `pdf_url` come from one location:
  `primary_location` when its `pdf_url` is set, else `best_oa_location`
  when present, else `primary_location`.
- **OpenAlex `select`**: widens by `biblio`, `ids`, `open_access` and
  `best_oa_location`.
- **Type mapping**: OpenAlex types map to Citation Style Language (CSL)
  types as follows, and an unmapped type yields no `type` key:

  | OpenAlex                                       | CSL                  |
  |------------------------------------------------|----------------------|
  | `article`, `data-paper`, `editorial`,          | `article-journal`    |
  | `erratum`, `letter`, `retraction`, `review`,   |                      |
  | `software-paper`                               |                      |
  | `book`                                         | `book`               |
  | `book-chapter`                                 | `chapter`            |
  | `book-review`                                  | `review-book`        |
  | `conference-abstract`, `conference-paper`      | `paper-conference`   |
  | `dataset`                                      | `dataset`            |
  | `dissertation`                                 | `thesis`             |
  | `libguides`                                    | `webpage`            |
  | `other`, `paratext`, `supplementary-materials` | `document`           |
  | `peer-review`                                  | `review`             |
  | `preprint`                                     | `article`            |
  | `reference-entry`                              | `entry-encyclopedia` |
  | `report`                                       | `report`             |
  | `software`                                     | `software`           |
  | `standard`                                     | `standard`           |

  Highwire inference for web pages: `citation_journal_title` →
  `article-journal`, `citation_conference_title` → `paper-conference`,
  `citation_dissertation_institution` → `thesis`,
  `citation_technical_report_institution` → `report`,
  `citation_inbook_title` → `chapter`, `citation_book_title` alone →
  `book`. schema.org `@type` maps `ScholarlyArticle` → `article-journal`,
  `Book` → `book`, `Chapter` → `chapter`, `Dataset` → `dataset`,
  `Thesis` → `thesis`, `Report` → `report`, `WebPage` → `webpage`.
- **Value forms** of the keys this story fills:
  - `authors`: each author is `Family, Given` when the source gives the
    parts (schema.org `familyName`/`givenName`, or a `citation_author`
    containing a comma), and otherwise the name as given; authors join
    with `; `. A single name string is never split.
  - `pages`: `first-last`, or one value when first equals last or only
    first is known (`e4375`).
  - `license`: a lowercase Creative Commons short code (`cc-by`,
    `cc-by-sa`, `cc-by-nc-nd`, `cc0`) when the licence is recognisably one,
    otherwise the licence URL.
  - `issn`: the ISSN-L only.
  - `type`: the CSL type from the mapping above.
  - `volume`, `issue`, `publisher`, `journal_ref`: the source value as
    given, on one line.
- **Rules carried from 0348**: scheme precedence, usable values, untrusted
  values, key order and no duplication apply unchanged. `pdf_url` is
  recorded, never fetched.
- The finding and level-note outputters document these value forms.

## Acceptance Criteria

- [ ] Given the OpenAlex PeerJ work `W2741809807`, when the CLI emits its
      record, then `metadata` carries `type: article-journal`, `authors`
      as its display names in `author_position` order joined by `; `,
      `publisher`, `volume: 6`, `pages: e4375`, `issn` as its ISSN-L,
      `pmid: 29456894`, `oa_status` and `license: cc-by`.
- [ ] Given an OpenAlex work whose `primary_location` has no `pdf_url` and
      whose `best_oa_location` has `pdf_url` `https://repo.example/x.pdf`
      and `license` `cc-by-sa`, when the CLI emits its record, then
      `metadata` carries `pdf_url: https://repo.example/x.pdf` and
      `license: cc-by-sa`.
- [ ] Given an OpenAlex work of type `dissertation`, then `metadata`
      carries `type: thesis`; given an unmapped type, then it has no
      `type` key.
- [ ] Given an OpenAlex work with `biblio.first_page` `12` and
      `biblio.last_page` `19`, then `metadata` carries `pages: 12-19`.
- [ ] Given an arXiv entry with `arxiv:journal_ref` `Nature 1, 2 (2024)`,
      `arxiv:primary_category` term `cs.CL`, authors `Ada Lovelace` and
      `Alan Turing`, and a PDF link, when the CLI emits its record, then
      `metadata` carries `type: article`,
      `authors: Ada Lovelace; Alan Turing`,
      `journal_ref: Nature 1, 2 (2024)`, `primary_category: cs.CL` and
      `pdf_url`.
- [ ] Given a page whose JSON-LD `author` has `familyName` `Piwowar` and
      `givenName` `Heather`, when the web metadata command reads it, then
      the map carries `authors: Piwowar, Heather`.
- [ ] Given a page with `citation_author` `Smith, John` and
      `citation_author` `Jane Doe`, then the map carries
      `authors: Smith, John; Jane Doe`.
- [ ] Given a page with `citation_dissertation_institution` and no
      schema.org `@type`, then the map carries `type: thesis`; given a page
      declaring no scholarly metadata, then `type: webpage`.
- [ ] Given a page with `citation_firstpage` `12`, `citation_lastpage`
      `19`, `citation_volume` `6` and `citation_issn` `1234-5678`, then the
      map carries `pages: 12-19` and `volume: 6`, and no `issn` key.
- [ ] Given a page with schema.org `license`
      `https://creativecommons.org/licenses/by/4.0/`, then the map carries
      `license: cc-by`; given `https://example.org/terms`, then
      `license: https://example.org/terms`.
- [ ] The finding and level-note outputters document the value
      forms for `authors`, `pages`, `license`, `issn` and `type`.

Verification: every criterion but the last is a retained Rust test in
`cli/research` over recorded OpenAlex JSON, arXiv Atom and HTML fixtures;
the last is checked by inspection.

## Open Questions

None.

## Dependencies

- Blocked by: 0348, which defines the block, the recommended keys, the web
  metadata command and the core keys' value forms.
- Relies on 0280's normalisers and 0348's web metadata command as shipped.
- External: the OpenAlex Work object, the arXiv Atom feed and `arXivRaw`
  record, and the Highwire, schema.org and Dublin Core vocabularies.

## Assumptions

- Odd `biblio` values such as `Spring` or `Inside cover` are recorded as
  given, since they are what the source says.
- A paywalled work's `license` and `pdf_url` describe its open copy when
  `best_oa_location` supplies them.

## Technical Notes

- `OPENALEX_SELECT` in `cli/research/src/sources/request.rs`; OpenAlex's
  `select` rejects nested paths, so whole objects are added.
- OpenAlex `ids.pmid` is a `https://pubmed.ncbi.nlm.nih.gov/` URL;
  `type_crossref` is no longer returned.
- arXiv's `primary_category` value is an attribute; the Atom feed has no
  licence, and whether the `arXivRaw` record the CLI already fetches for
  withdrawal carries one is to be confirmed in planning.

## Drafting Notes

- Dropping `isbn`, `available`, `language`, `pmcid` and `version` from
  0348's recommended table was a user decision: no source gives `pmcid`
  reliably, and `version` means copy status in OpenAlex, a revision in
  arXiv (already in `arxiv_id`) and a release in CSL.
- Never splitting a single name string, and keeping structured parts where
  a source gives them, was a user decision.
- Filling each key as far as each source allows, for a complete corpus
  rather than a named consumer, was a user decision; low priority follows
  from having no consumer yet.
- The type mapping, the Highwire inference, the `webpage` fallback, the
  OpenAlex location rule and the licence short codes are my choices.
- Not filling `issn` for web pages is my choice: a page's ISSNs cannot be
  resolved to an ISSN-L without the ISSN-L table.

## References

- Source: 0348 and its review,
  `meta/reviews/work/0348-structured-source-metadata-review-1.md`
- Related: 0121, 0280, 0348
- OpenAlex Work, Location and Source objects and work types
  (help.openalex.org); CSL 1.0.2 specification, Appendix III; arXiv API
  user manual; Google Scholar inclusion guidelines; schema.org
  ScholarlyArticle, PublicationVolume, PublicationIssue, Person; DCMI
  Metadata Terms
