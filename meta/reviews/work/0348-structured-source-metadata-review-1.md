---
type: "work-item-review"
id: "0348-structured-source-metadata-review-1"
title: "Work Item Review: Structured Source Metadata"
date: "2026-10-24T09:43:21+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0348"
work_item_id: "0348"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-10-24T11:37:32+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Structured Source Metadata

**Verdict:** REVISE

0348 is detailed and mostly precise. It has a literal block example, an
ordered key table, worked value forms and concrete Given/When/Then
criteria, and it draws its boundaries with 0306, 0284 and 0281 explicitly.
It still needs revising for three reasons. Its sections disagree about who
does what: the CLI or the researcher, the web profile or the web
researcher, and who records `accessed`. Many of its stated rules have no
acceptance criterion, and the agent-driven criteria never say how they are
verified. It blocks 0306 on a 24-key vocabulary when 0306 reads only
`doi`, and one criterion relies on `report`, which cannot exist until
after this item ships.

### Cross-Cutting Themes

- **Who records `accessed`** (flagged by: clarity, completeness, scope,
  testability): the web-profile requirement records it, the Assumptions
  give it to the CLI for academic records, and the CLI requirement and
  criteria never mention it. Its precision (date or datetime) and the
  meaning of "this run" are undefined.
- **Actor naming** (flagged by: clarity, scope): the Summary says the CLI
  fills the block, while the Requirements have the CLI emit a `metadata`
  map that a researcher copies. "Web profile" and "web researcher" name
  the same actor without being linked. The Drafting Notes count "three
  profiles" but only the web one is named.
- **Untrusted values** (flagged by: clarity, dependency, testability): the
  item neither links nor names "the outputters' untrusted-content
  contract", "no value is acted on" is ambiguous and seems to clash with
  0306 reading `doi`, and no criterion checks the inert-recording
  behaviour.
- **Verbatim carry-forward vs key order** (flagged by: clarity,
  testability): the item does not say whether a composer copies an
  out-of-order block byte-for-byte or re-sorts it.
- **Size and the split** (flagged by: scope, completeness): the split of
  CLI emission is deferred to planning while Open Questions says "None".
  The whole vocabulary sits on 0306's critical path.

### Findings

#### Major

- 🟡 **Clarity / Completeness / Scope / Testability**: Who records
  `accessed` is inconsistent and its format is unpinned
  **Location**: Requirements: CLI, Web profile / Assumptions / Acceptance
  Criteria
  The Assumptions say the CLI records `accessed` for academic records, but
  the CLI requirement and the OpenAlex/arXiv criteria omit it. Precision
  and "this run" are undefined.
- 🟡 **Clarity**: The Summary says the CLI fills the block, but the
  Requirements have the researcher write it
  **Location**: Summary
  The CLI emits a `metadata` map, and a researcher copies it into the
  level note. The Summary describes a different implementation surface.
- 🟡 **Scope**: 0306 is blocked on the full vocabulary but needs only
  `doi`
  **Location**: Requirements: Open vocabulary / Dependencies
  No referenced item consumes the bibliographic or source-specific keys,
  or the Dublin Core, JSON-LD and OpenGraph extraction, yet
  `blocks: [0306]` ties 0306 to all of it.
- 🟡 **Dependency**: A criterion exercises `report`, which ships only
  after this item
  **Location**: Acceptance Criteria
  `report` arrives in 0281, and the chain 0348 → 0306 → 0281 means it
  cannot exist when 0348 is verified.
- 🟡 **Testability**: Several value-form rules have no acceptance
  criterion
  **Location**: Requirements: Value forms
  `authors`, `pages`, `issn` (ISSN-L), `type` (CSL) and reduced-precision
  dates are untested. An implementation that pads `2023` to `2023-01-01`
  passes every criterion.
- 🟡 **Testability**: The agent-behaviour criteria have no verification
  procedure
  **Location**: Acceptance Criteria
  Five criteria depend on LLM researcher or composer output. The item
  names no fixture run, eval or inspection method.
- 🟡 **Testability**: "No value is acted on" has no criterion
  **Location**: Requirements: Untrusted values
  Only line-break collapsing is tested. Nothing checks that an
  injected-instruction value is recorded inertly.
- 🟡 **Testability**: Only Highwire tags are tested, and precedence
  between vocabularies is unspecified
  **Location**: Requirements: Web profile
  Dublin Core, JSON-LD, OpenGraph and URL-prefixed DOIs have no criteria.
  No winner is defined when vocabularies disagree.

#### Minor

- 🔵 **Clarity / Scope**: "Web profile" vs "web researcher", and "three
  profiles" with only one named
  **Location**: Requirements: Web profile / Drafting Notes
  The item does not say whether the work changes profile text, researcher
  behaviour or both, or which profiles are in scope.
- 🔵 **Clarity / Dependency**: The untrusted-content contract is unnamed
  and "acted on" is undefined
  **Location**: Requirements: Untrusted values / Dependencies
  It is not linked to an owner, and a strict reading conflicts with 0306
  using a web-declared `doi` for identity.
- 🔵 **Clarity / Testability**: Verbatim carry-forward vs key order
  **Location**: Requirements: Carry-forward, Key order / Acceptance
  Criteria
  The `dataset_version` fixture is already in order, so it cannot tell
  copying apart from re-sorting.
- 🔵 **Scope / Completeness**: The split decision is deferred to planning
  despite a clean seam
  **Location**: Drafting Notes / Open Questions
  The CLI `metadata` map is an additive field that can be tested on its
  own. Decide now, or record the split as an open question.
- 🔵 **Completeness**: The 0121 contract update appears only in
  Dependencies
  **Location**: Requirements
  No requirement or criterion covers updating 0121's Sources contract.
- 🔵 **Completeness**: Context justifies the DOI but not the wider
  vocabulary
  **Location**: Context
  No consumer is named for the dates, bibliographic and source-specific
  keys.
- 🔵 **Testability**: The expected forms of `openalex_id` and `arxiv_id`
  are not pinned
  **Location**: Requirements: Value forms / Acceptance Criteria
  OpenAlex returns `https://openalex.org/W…`, and Value forms does not
  cover `openalex_id`.
- 🔵 **Testability**: The no-duplication rule has no criterion
  **Location**: Requirements: No duplication
  Copying `og:title` or `og:url` into the block passes every criterion.
- 🔵 **Dependency**: The ordering between 0348 and 0305 is not recorded
  **Location**: Dependencies
  0305 requires keeping `doi` in source metadata from 0348, yet records
  no blocker.
- 🔵 **Dependency**: Reliance on the shipped composer, `synthesise` and
  finding outputter is not recorded
  **Location**: Dependencies
  The "Relies on" line names only 0280 and 0283's level-note template.

#### Suggestions

- 🔵 **Completeness**: Technical Notes cover only part of the story's
  surface
  **Location**: Technical Notes
  Add paths for the profiles, the composer, `synthesise`, the normalisers
  and the untrusted-content contract.
- 🔵 **Clarity**: Unexpanded standards terms and the "version read"
  phrasing
  **Location**: Requirements: Value forms / Web profile
  Expand CSL, ISSN-L, Highwire and OpenGraph, and reword `arxiv_id` as
  "including the version suffix".
- 🔵 **Testability**: "Cited as before" needs a concrete baseline
  **Location**: Acceptance Criteria
  Name a fixture finding and the expected one-line dossier entries.
- 🔵 **Dependency**: CLI metadata may need OpenAlex and arXiv fields not
  yet fetched
  **Location**: Requirements: CLI
  State whether only fields the CLI already fetches are used.
- 🔵 **Dependency**: Concurrent web-profile edits in 0311 are not
  mentioned
  **Location**: Dependencies
  Add 0311 as Related.

### Strengths

- ✅ The literal block example matches the key-order rule, and the value
  forms carry before/after examples that leave little room for
  misreading.
- ✅ Scope edges are handed to their owners: merging goes to 0306,
  rendering to 0284, Sources validation to 0281, and retraction stays a
  tier suffix.
- ✅ The 0306 Blocks edge is reciprocated in 0306's `blocked_by`, and 0284
  carries its side of the consumer coupling.
- ✅ Context establishes that no shipped code parses Sources and that
  findings stay immutable, which removes migration and breakage risk.
- ✅ The CLI criteria pin exact inputs and outputs and include a negative
  case (no `arxiv:doi`, so no `doi` key). The carry-forward criterion
  exercises a non-recommended key.
- ✅ Every section is substantively filled, and the Drafting Notes give
  provenance for each decision.

### Recommended Changes

1. **Settle `accessed`** (addresses: the `accessed` finding)
   Decide whether the CLI's `metadata` map includes `accessed`, and make
   the CLI requirement, the Assumptions and the CLI criteria agree. Pin
   its form, for example an ISO 8601 UTC date, and define the fetch event
   it records.
2. **Name actors consistently** (addresses: the Summary/CLI finding,
   web profile vs web researcher, three profiles)
   The Summary should say that the CLI emits `metadata` and the
   researcher writes the block. Use one phrase such as "the researcher,
   following the web profile" throughout, and list each profile in scope
   with its role.
3. **Narrow the 0306 critical path or split** (addresses: the 0306
   blocking finding, the split decision, Context for the vocabulary)
   Either slice the first deliverable to the block format, carry-forward,
   the identifier keys, `published` and `accessed`, and DOI extraction,
   and block 0306 on that slice alone; or justify the full vocabulary in
   Context and remove the conditional split note.
4. **Drop `report` from the backward-compatibility criterion**
   (addresses: the `report` finding, the "cited as before" baseline)
   Limit it to `synthesise` with a named fixture, and leave the `report`
   behaviour to 0281.
5. **Define and test the untrusted-values rule** (addresses: the
   untrusted-content contract, "no value is acted on")
   Name the contract and its owner, say that values are never fetched or
   followed but may be compared as identifiers, and add an
   injected-instruction criterion.
6. **Fill the criterion gaps** (addresses: value forms, the web
   vocabularies and precedence, no duplication, the ID forms)
   Add criteria for `authors`, `pages`, `issn`, `type`, reduced-precision
   dates, each web vocabulary plus one conflict case with a precedence
   order, the absence of title, URL and venue, and bare `openalex_id`
   (add it to Value forms).
7. **State how the agent criteria are verified** (addresses: the
   agent-behaviour finding)
   Name the method, such as a fixture page or record run through the
   researcher and compared against an expected block. Separate the
   deterministic CLI and documentation checks from the agent-run checks.
8. **Resolve verbatim vs key order** (addresses: the carry-forward
   conflict)
   State byte-for-byte copying and use an out-of-order fixture, or state
   re-sorting and expect sorted output.
9. **Tidy Dependencies and Technical Notes** (addresses: 0305 ordering,
   0121 update, composer/`synthesise` reliance, 0311, Technical Notes)
   Record the 0305 ordering, make the 0121 update a requirement, extend
   "Relies on", add 0311, and add the missing paths.

## Per-Lens Results

### Clarity

**Summary**: Mostly precise, with a concrete example, ordering and value
forms. The main problems are inconsistent naming of actors (CLI vs
researcher, web profile vs web researcher, who records `accessed`) plus a
few phrases and acronyms that can be read more than one way.

**Strengths**:
- The block example matches the stated key-order rule.
- The value forms carry concrete before/after examples.
- The 0306 boundary is stated consistently in both directions.
- Context ties the problem directly to the solution.

**Findings**:
- 🟡 major / high — Requirements: Web profile / Assumptions / Acceptance
  Criteria — Who records `accessed` is inconsistent. Three sections give
  three answers, the CLI map in the criteria omits `accessed`, and "this
  run" is undefined. Fix by stating whether the CLI map carries it,
  aligning the Assumptions and defining the event.
- 🟡 major / medium — Summary — The Summary says the CLI fills the block,
  but the Requirements have the researcher write it. Rewrite it so the
  CLI emits structured metadata and the researcher writes the block.
- 🔵 minor / medium — Requirements: Web profile — "Web profile" and "web
  researcher" are not linked, and three profiles are counted but one is
  named. Name the actor once and list the profiles.
- 🔵 minor / medium — Requirements: Untrusted values — The contract is
  not linked and "acted on" is ambiguous, conflicting with 0306 reading
  `doi`. Link the contract and say what is excluded.
- 🔵 minor / low — Requirements: Carry-forward / Key order — "Verbatim"
  may conflict with key order. State whether verbatim means
  byte-identical.
- 🔵 suggestion / medium — Requirements: Value forms / Web profile — CSL,
  ISSN-L, Highwire and OpenGraph are unexpanded, and "version read" reads
  awkwardly. Expand the terms and reword.

### Completeness

**Summary**: A well-populated story with every section present. The gaps
are small: obligations that appear outside Requirements (the 0121 update,
the CLI's `accessed`), Context that justifies only the DOI, and partial
Technical Notes.

**Strengths**:
- The frontmatter is complete and consistent.
- The Summary names beneficiary, capability and outcome.
- Context gives the concrete driver and the safety argument.
- The Requirements are detailed enough to act on.
- There are 11 concrete Given/When/Then criteria.
- Dependencies, Assumptions and Drafting Notes are substantive.

**Findings**:
- 🔵 minor / medium — Requirements — The 0121 Sources contract update is
  listed as a dependency but not as a requirement. Add a requirement and
  a criterion.
- 🔵 minor / medium — Requirements — The CLI's duty to record `accessed`
  appears only in Assumptions. Move it into the CLI requirement.
- 🔵 minor / low — Context — Context justifies the DOI but not the wider
  vocabulary. Name consumers for the date and bibliographic keys, or mark
  them speculative.
- 🔵 suggestion / medium — Technical Notes — Paths are missing for the
  profiles, the composer, `synthesise`, the normalisers and the
  untrusted-content contract.
- 🔵 suggestion / low — Open Questions — The possible split is in Drafting
  Notes but not recorded as an open question.

### Dependency

**Summary**: The main couplings are well mapped and reciprocated. The
gaps are about ordering: `report` belongs to a downstream item, the
0305 ordering is unrecorded, and reliance on the shipped composer,
`synthesise` and the untrusted-content contract is not stated.

**Strengths**:
- The 0306 Blocks edge carries its reason and is reciprocated.
- 0281 and 0284 are named with concrete obligations, and both record
  their side.
- There is a fallback until 0284 lands.
- No shipped code parses Sources, which removes the risk of hidden
  breakage.
- Reliance on 0280 and 0283 and the 0121 update are recorded.

**Findings**:
- 🟡 major / high — Acceptance Criteria — A criterion exercises `report`,
  which ships in 0281 after this item (0348 → 0306 → 0281). Limit it to
  `synthesise` and move `report` to 0281.
- 🔵 minor / high — Dependencies — The ordering between 0348 and 0305 is
  not recorded, although 0305 needs the `doi` key. Record the direction
  on both items.
- 🔵 minor / medium — Dependencies — Reliance on the shipped composer,
  `synthesise` and finding outputter is not recorded. Extend "Relies
  on".
- 🔵 minor / medium — Requirements — The untrusted-values requirement
  relies on an unnamed contract. Name it and its owner, or take on
  extending it.
- 🔵 suggestion / low — Requirements — The CLI metadata may need OpenAlex
  and arXiv fields that are not yet fetched. State the assumption or the
  external coupling.
- 🔵 suggestion / low — Dependencies — Concurrent web-profile edits in
  0311 are not mentioned. Add 0311 as Related.

### Scope

**Summary**: One clear purpose with explicit edges, but broad: Rust
normalisation, four web schemes, two outputters, the composer,
`synthesise` and 24 keys, while 0306 needs only `doi`. The split is
deferred, and the profile count and `accessed` ownership blur the
boundary.

**Strengths**:
- Out-of-scope edges are named and handed to their owners.
- Every requirement serves the one capability.
- No migration effort is hidden inside the story.
- The split seam is already named.

**Findings**:
- 🟡 major / medium — Requirements: Open vocabulary / Dependencies — 0306
  is blocked on the full vocabulary but needs only `doi`. Slice the
  first deliverable and block 0306 on it alone.
- 🔵 minor / medium — Drafting Notes — The split decision is deferred
  despite a clean seam. Decide now.
- 🔵 minor / medium — Requirements / Drafting Notes — "Three profiles" in
  the notes vs one named in the Requirements. List each profile with its
  role.
- 🔵 minor / medium — Assumptions — An assumption adds `accessed` to the
  CLI's scope while Requirements and criteria omit it. Align them.

### Testability

**Summary**: The CLI criteria are concrete and translate directly into
unit tests. The gaps are coverage (value forms, web vocabularies,
no-duplication, untrusted values) and determinism (agent criteria with
no verification method, plus unpinned `openalex_id` and `accessed`
forms).

**Strengths**:
- The CLI criteria give exact inputs and outputs.
- There is a negative case for the missing DOI.
- The web normalisation criterion pins before and after values.
- Carry-forward is tested with a non-recommended key.
- There is an explicit backward-compatibility criterion.

**Findings**:
- 🟡 major / high — Requirements: Value forms / Acceptance Criteria —
  The `authors`, `pages`, `issn`, `type` and reduced-precision date rules
  are untested. Add fixture criteria.
- 🟡 major / medium — Acceptance Criteria — The agent-behaviour criteria
  have no verification procedure. Name the method, or split the
  deterministic criteria from the agent-run ones.
- 🟡 major / high — Requirements: Untrusted values — "No value is acted
  on" has no criterion. Add an injected-instruction criterion.
- 🟡 major / medium — Requirements: Web profile — Only Highwire is
  tested, and there is no precedence between vocabularies. State an
  order and add a criterion per vocabulary plus a conflict case.
- 🔵 minor / high — Acceptance Criteria — The `openalex_id` and
  `arxiv_id` forms are not pinned. Add them to Value forms with concrete
  criteria.
- 🔵 minor / medium — Assumptions / Acceptance Criteria — The `accessed`
  format and CLI-side recording are untested. Fix the precision and use
  a fixed clock.
- 🔵 minor / medium — Acceptance Criteria — Carry-forward does not test
  verbatim vs key order. Use an out-of-order fixture.
- 🔵 minor / medium — Requirements: No duplication — No criterion. Add a
  check that the block carries no title, URL or venue.
- 🔵 suggestion / medium — Acceptance Criteria — "Cited as before" needs
  a concrete baseline fixture.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-10-24T10:24:00+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity / Completeness / Scope / Testability**: who records
  `accessed` — Resolved (a UTC date; the CLI stamps it from an injectable
  clock, the researcher from the run date)
- 🟡 **Clarity**: the Summary said the CLI fills the block — Resolved
- 🟡 **Scope**: 0306 was blocked on the full vocabulary — Resolved (only
  the core keys are filled; a follow-on fills the rest)
- 🟡 **Dependency**: a criterion exercised `report` — Resolved (moved to
  0281 under Consumers)
- 🟡 **Testability**: value-form rules had no criteria — Partially
  resolved (ID forms pinned; reduced-precision dates still untested)
- 🟡 **Testability**: no verification procedure for the agent criteria —
  Resolved (Verification paragraph)
- 🟡 **Testability**: "no value is acted on" had no criterion — Resolved
  (injection criterion)
- 🟡 **Testability**: only Highwire tested, no precedence — Resolved
- 🔵 **Clarity / Scope**: web profile vs web researcher, three profiles —
  Resolved
- 🔵 **Clarity / Dependency**: the untrusted-content contract was unnamed
  — Resolved
- 🔵 **Clarity / Testability**: verbatim vs key order — Resolved
  (byte-identical; out-of-order fixture)
- 🔵 **Scope / Completeness**: split decision deferred — Resolved
- 🔵 **Completeness**: 0121 update, Technical Notes, Context — Resolved
- 🔵 **Testability**: no-duplication and ID forms — Resolved
- 🔵 **Dependency**: 0305 ordering, composer/`synthesise` reliance, 0311 —
  Resolved

### New Issues Introduced

- 🟡 **Dependency**: web metadata extraction assumes the fetch tool exposes
  `<head>` metadata. The web profile fetches with `WebFetch`, which returns
  processed markdown, and the researcher's `Bash` is restricted to
  `accelerator research fetch`, so meta tags and JSON-LD are probably
  unreachable (confirmed against the profile and agent definition).
- 🟡 **Testability**: the reduced-precision, never-padded date rule has no
  criterion.
- 🔵 **Clarity / Testability**: "usable value" is not tied to the
  malformed-value rule, so whether an unusable Highwire value falls through
  to a later scheme is undefined and untested.
- 🔵 **Clarity / Dependency / Completeness**: the web `accessed` has two
  definitions (fetch date vs run date); the run date is the derived UTC
  timestamp `conduct` injects (`research-topic/SKILL.md:405`), but the item
  does not say so.
- 🔵 **Clarity**: "producer" is undefined, and "copies it, never reformats
  it" leaves key ordering between the CLI and the researcher unassigned.
- 🔵 **Scope / Completeness / Dependency**: the follow-on story has no ID;
  value forms for keys it alone fills are fixed here.
- 🔵 **Scope**: the core keys exceed "what 0306 needs" (`published`,
  `updated` and the record ids have no named consumer); the Summary
  promises citing without re-fetching, which needs the deferred keys.
- 🔵 **Dependency**: the 0306 Blocks line misstates the merge (the first
  citation in citation order, with the earliest `accessed` only on full
  ties); 0311, 0160 and 0161 are not in frontmatter both ways.
- 🔵 **Testability**: loose criteria — first-declaration-wins, line-break
  fixture, arXiv `doi` and web `accessed` values, verbatim level-note copy
  and arXiv coverage, `synthesise` byte-identity, OpenAlex null-DOI case.
- 🔵 **Completeness**: OpenAlex's source for `updated` (none) is unstated.

### Assessment

Pass 1's structural and decision-level issues are resolved. Two majors
remain, which meets the REVISE threshold of two. The fetch-tool gap needs
a design decision because it determines whether the web half of the story
can be delivered at all. The unpadded-date gap is a missing criterion. The
minors are tightening work.

## Re-Review (Pass 3) — 2026-10-24T11:30:14+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Dependency**: the fetch tool does not expose `<head>` metadata —
  Resolved (a CLI web metadata command; the CLI is the only producer)
- 🟡 **Testability**: the unpadded-date rule had no criterion — Resolved
- 🔵 **Clarity / Testability**: "usable" vs malformed-value fall-through —
  Partially resolved (fall-through across schemes is defined; the case of
  an unusable first declaration within a scheme is still ambiguous)
- 🔵 **Clarity / Dependency / Completeness**: two definitions of the web
  `accessed` — Resolved
- 🔵 **Clarity**: "producer" undefined; key-order duty unassigned —
  Resolved
- 🔵 **Scope / Completeness / Dependency**: the follow-on had no ID —
  Resolved (0349, PP-924)
- 🔵 **Scope**: core keys exceed 0306's needs; Summary over-promises —
  Partially resolved (rationale and Summary fixed; web-page dates still
  have no named consumer)
- 🔵 **Dependency**: 0306 merge misdescribed; one-sided Related links —
  Partially resolved (0306 wording and 0311 fixed; 0160 and 0161 still
  one-sided)
- 🔵 **Testability**: loose criteria from pass 2 — Resolved
- 🔵 **Completeness**: OpenAlex `updated` source unstated — Resolved

### New Issues Introduced

- 🟡 **Completeness / Testability / Clarity**: the web metadata command
  has no mapping from each scheme's fields to the core keys, so only the
  sampled fields can be verified and `updated` precedence cannot be
  checked.
- 🔵 **Clarity**: "never padded" reads as conflicting with the
  zero-padded `2010-05-12`.
- 🔵 **Testability**: non-UTC offsets, the set of accepted input date
  forms, the "public http(s)" restriction and the failure conditions are
  undefined.
- 🔵 **Testability**: both injection criteria pass on the usable-value
  filter alone; the web map's key order and line-break collapsing have no
  criteria.
- 🔵 **Clarity**: non-recommended keys and out-of-order blocks have no
  stated origin; the scope of "never change what is done next" and "the
  map" is loose.
- 🔵 **Scope**: the split is still conditional; provisional forms for
  0349's keys are documented in both items.
- 🔵 **Dependency**: publisher pages may refuse the CLI's fetch, which
  weakens 0306's DOI identity; there is no availability note.

### Assessment

The item now has one major issue: the missing field-to-key mapping, which
completeness, testability and clarity all flagged. That falls below the
REVISE threshold. The item is acceptable, but it would be improved by
adding the mapping table and tightening the web command's date and
failure rules before planning.

## Verdict Change — 2026-10-24T11:37:32+00:00

**Verdict:** APPROVE

After pass 3, every remaining finding was addressed in the work item: the
web field-to-key mapping, per-declaration fall-through, date rules, fetch
rules, keeping one story, and 0349 owning its keys' value forms, plus the
structural fixes. The user approved without a fourth lens pass.

