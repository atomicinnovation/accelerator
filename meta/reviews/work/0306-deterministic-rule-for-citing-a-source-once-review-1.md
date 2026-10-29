---
type: "work-item-review"
id: "0306-deterministic-rule-for-citing-a-source-once-review-1"
title: "Work Item Review: Deterministic Rule for Conflicting Source Tiers"
date: "2026-10-24T05:11:44+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0306"
work_item_id: "0306"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 6
tags: []
last_updated: "2026-10-24T09:04:29+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Deterministic Rule for Conflicting Source Tiers

**Verdict:** REVISE

0306 is a well-bounded story with a concrete merge rule, exact expected
output strings, and clean hand-offs to 0305, 0311, 0281 and 0284. It falls
short on its own promise of determinism: the suffix-order wording contradicts
the defined tier ordering, the order-independence criterion has no
verification procedure for an LLM composer, and several rule clauses (venue
tied to the chosen tier, DOI identity, same-tier duplicates, case handling)
have no defined expected output.

### Cross-Cutting Themes

- **Unresolved case handling in the venue tie-break** (flagged by: clarity,
  completeness, testability) — "lexicographically first" is deferred to the
  plan, the only tie-break criterion is case-insensitive, and Open Questions
  is empty, so the gap is both undecided and hidden.
- **DOI identity under-specified** (flagged by: clarity, testability,
  dependency) — it is unclear whether a DOI is parsed from a `doi.org` URL or
  must be recorded separately, the DOI criterion asserts no observable
  output, and nothing names which profile guarantees the DOI is recorded.
- **Same-tier duplicates** (flagged by: scope, testability) — the identity
  and venue rules reach citations with no tier conflict, but the Summary
  frames the item around tier conflicts and no criterion defines the
  same-tier output.
- **Retracted/withdrawn semantics** (flagged by: clarity, testability) —
  whether the suffix is a property of the source or of one citation is
  unstated, and no `synthesise` criterion checks it survives a merge.
- **Upstream contracts not recorded** (flagged by: completeness,
  dependency) — 0283 (composer, level notes), 0280 (suffix vocabulary) and
  0121 (carry-forward contract) are absent from Dependencies; 0281 and 0284
  are absent from References.

### Findings

#### Major

- 🟡 **Clarity**: 'Descending order' contradicts the stated tier ordering
  **Location**: Requirements
  The rule defines `tier-3 < tier-2 < tier-1`, so "descending" would list
  tier-1 first, but the examples list `tier-2, tier-1`.
- 🟡 **Testability**: Order-independence criterion is unbounded and has no
  verification procedure for a non-deterministic agent
  **Location**: Acceptance Criteria
  "In any order" quantifies over all permutations of an LLM composer's input
  with no fixture set, permutation count, or run count.
- 🟡 **Testability**: DOI-merge criterion does not specify the observable
  form of the merged citation
  **Location**: Acceptance Criteria
  "Treated as one source" gives no tiers and no expected identifier, so any
  single citation passes.
- 🟡 **Testability**: Venue tied to the chosen tier has no criterion
  **Location**: Requirements
  Both venue criteria are same-tier ties; a composer that keeps a tier-1
  venue on a tier-3 entry passes both.

#### Minor

- 🔵 **Clarity / Completeness / Testability**: Venue tie-break leaves case
  sensitivity open, and Open Questions is empty
  **Location**: Requirements, Drafting Notes, Open Questions
  Byte-wise and case-folded comparison disagree for mixed-case venues; the
  decision is deferred and not surfaced as an open question.
- 🔵 **Clarity**: 'Carry the same DOI' is ambiguous for `doi.org` URLs
  **Location**: Requirements
  Unclear whether a DOI is extracted from a `doi.org` URL or must be a
  separately recorded field.
- 🔵 **Clarity**: Unclear which citation a carried retracted/withdrawn suffix
  belongs to
  **Location**: Requirements
  `tier-3 (retracted; also tier-1)` reads as the tier-3 citation being
  retracted even when the tier-1 one was.
- 🔵 **Clarity**: Unexplained phrases in the conflict evidence
  **Location**: Context
  "Root note's" is undefined and "(or the higher)" obscures what the three
  observed strategies were.
- 🔵 **Scope**: Identity and venue rules also cover duplicates with no tier
  conflict
  **Location**: Requirements
  The venue criterion is same-tier, so the item in effect introduces general
  same-source deduplication beyond what the title promises.
- 🔵 **Testability**: Same-tier duplicates have no defined expected output
  **Location**: Acceptance Criteria
  Nothing says two tier-2 citations become one `tier-2` with no suffix.
- 🔵 **Testability**: Most URL normalisation rules are not exercised
  **Location**: Requirements
  Only scheme and trailing slash are tested; nothing checks `www.`, fragment,
  host case, or that query and path case keep sources distinct.
- 🔵 **Testability**: `synthesise` has no criterion for suffix precedence
  across findings
  **Location**: Acceptance Criteria
  `synthesise` could drop a `(retracted)` suffix while merging and still pass.
- 🔵 **Completeness**: References omits 0281 and 0284
  **Location**: References
  Both appear in frontmatter and Dependencies; 0283's relevance is never
  explained.
- 🔵 **Dependency**: Reliance on shipped 0283 composer and 0280 tier suffixes
  not recorded
  **Location**: Dependencies
  "Blocked by: none known" is accurate, but the contracts this rule builds on
  are unrecorded.
- 🔵 **Dependency**: DOI-based identity depends on profiles recording the DOI
  **Location**: Assumptions
  0305 changes web-profile DOI recording; the identity rule needs the DOI to
  stay visible.
- 🔵 **Dependency**: Change to the epic 0121's tier carry-forward contract
  not recorded
  **Location**: Requirements
  0121 describes verbatim carry-forward; no "Updates 0121" line.

#### Suggestions

- 🔵 **Clarity**: Relationship to the composer's 'never re-tier' instruction
  is unstated
  **Location**: Context
  Picking the lowest tier arguably re-tiers; say how the instruction is
  amended.
- 🔵 **Clarity**: Garbled phrase in the 0281 dependency
  **Location**: Dependencies
  "Whose `report` Sources and coverage judgement" — name the two consumers
  separately.
- 🔵 **Scope**: Same-source identity is a separate capability 0281 relies on
  **Location**: Requirements
  No split needed; name the identity rule in the Summary so consumers see
  this item owns it.
- 🔵 **Testability**: Documentation criterion could name the required content
  **Location**: Acceptance Criteria
  Require the outputter docs to include the two combined-suffix examples.
- 🔵 **Dependency**: Composer eval coverage (0161) may need to follow the
  contract change
  **Location**: Dependencies
  Check whether 0161 asserts verbatim tier carry-forward.

### Strengths

- ✅ Summary is a proper user story, and Context grounds the problem in a real
  depth-3 run (10 composers, three resolution strategies, named sources).
- ✅ Scope is explicit: "this item fixes the merge rule only", with drift and
  DOI tiering handed to 0311 and 0305, and `report` application to 0281.
- ✅ Same-source identity has an operational URL normalisation recipe.
- ✅ Merge-rule criteria give exact expected strings, so pass/fail is a
  literal comparison, including the two-suffix and `synthesise` re-merge
  cases.
- ✅ The 0281 blocking edge is reciprocated, and the item confirms no shipped
  CLI or visualiser code parses tier text.
- ✅ Drafting Notes separate user decisions from author choices and define
  "named venue".

### Recommended Changes

1. **Restate the suffix order unambiguously** (addresses: 'Descending order'
   contradicts the stated tier ordering)
   E.g. "the other tiers, next-lowest first (`tier-2` before `tier-1`)".
2. **Bound the determinism criterion** (addresses: Order-independence
   criterion is unbounded)
   Name a fixture set (e.g. the brandur.org and psycopg conflicts), the
   permutations (forward, reverse, root-note-last), and a run count with a
   pass threshold.
3. **Define DOI identity and pin the DOI criterion's output** (addresses:
   DOI-merge criterion; 'Carry the same DOI'; DOI-based identity depends on
   profiles)
   Say whether a DOI is parsed from `doi.org` URLs, recorded separately, or
   both; give the two citations different tiers and state the exact merged
   tier, identifier and venue; note the coupling to 0305.
4. **Add a cross-tier venue criterion** (addresses: Venue tied to the chosen
   tier)
   Tier-1 `Example Journal` + tier-3 `example.org` → `tier-3 (also tier-1)`,
   venue `example.org`.
5. **Decide case handling** (addresses: case-sensitivity theme)
   Pick byte-wise or case-folded in Requirements and add a mixed-case
   criterion, or move it to Open Questions.
6. **Cover same-tier duplicates explicitly** (addresses: Same-tier
   duplicates; Identity and venue rules also cover duplicates)
   State in Summary/Requirements that every citation sharing an identity
   merges to one, and add a same-tier criterion with no suffix.
7. **State retraction is a source property and add a `synthesise`
   precedence criterion** (addresses: retracted/withdrawn citation;
   `synthesise` suffix precedence)
   E.g. `tier-3 (retracted)` + `tier-2 (also tier-1)` →
   `tier-3 (retracted; also tier-2, tier-1)`.
8. **Add URL normalisation pair criteria** (addresses: Most URL
   normalisation rules are not exercised)
   A short same/different table, including query and path-case negatives.
9. **Tidy Dependencies, References and Context** (addresses: References
   omits 0281 and 0284; Reliance on 0283/0280; 0121 contract; root note;
   never re-tier; garbled 0281 phrase; 0161)
   Add "Relies on" 0283 and 0280, an "Updates 0121" line, 0281/0284 to
   References, define the three observed strategies, and say how the "never
   re-tier" instruction changes.

## Per-Lens Results

### Clarity

**Summary**: The work item is largely precise — its merge rule, URL
normalisation and venue tie-break are spelled out, and concrete acceptance
criteria pin the expected forms. The main clarity defect is a contradiction
between the stated tier ordering and the word "descending" for suffix order.
Smaller ambiguities concern which citation a retracted/withdrawn suffix
belongs to, what "carry the same DOI" means for doi.org URLs, a few
unexplained Context phrases, and a self-acknowledged case-sensitivity gap.

**Strengths**:
- Same-source identity is defined operationally with an explicit URL
  normalisation recipe.
- Every suffix form in the Requirements has a worked example, and the
  acceptance criteria repeat those exact strings.
- Actors are consistently named, and Context states plainly that only the
  merge rule is in scope.
- Drafting Notes separate user decisions from author choices and define
  "named venue".

**Findings**:
- 🟡 major / high — Requirements — 'Descending order' contradicts the stated
  tier ordering. The rule defines `tier-3 < tier-2 < tier-1` and says the
  other tiers follow "in descending order", but `tier-3 (also tier-2,
  tier-1)` is ascending under that ordering. An implementer following the
  defined ordering would emit `tier-3 (also tier-1, tier-2)`. State the order
  in terms that cannot conflict, e.g. "next-lowest first".
- 🔵 minor / medium — Requirements — Unclear which citation a carried
  retracted/withdrawn suffix belongs to. If the tier-1 citation was the
  retracted one, `tier-3 (retracted; also tier-1)` reads as the tier-3 one
  being retracted. State that retraction is a property of the source and
  never affects the lowest-tier choice.
- 🔵 minor / medium — Requirements — 'Carry the same DOI' is ambiguous for
  doi.org URLs. Define whether "carries a DOI" means a recorded DOI field, a
  DOI parsed from a `doi.org` URL, or both.
- 🔵 minor / high — Drafting Notes — Venue tie-break leaves case sensitivity
  open. Decide byte-wise or case-folded, or move it to Open Questions.
- 🔵 minor / medium — Context — Unexplained phrases: "root note's" and "(or
  the higher)". Name the three observed strategies explicitly and define or
  link "root note".
- 🔵 suggestion / medium — Context — Relationship to the composer's "never
  re-tier" instruction is unstated. Say how the existing wording relates to
  the merge rule.
- 🔵 suggestion / low — Dependencies — Garbled phrase in the 0281 dependency.
  Name its `report` Sources section and its coverage judgement separately.

### Completeness

**Summary**: This is a well-populated story: a proper user-story Summary, a
Context that explains the problem with concrete run evidence, implementable
Requirements, and ten Given/When/Then criteria covering each requirement.
Remaining gaps are minor: Open Questions is empty while an unresolved
decision sits in Drafting Notes, and References omits two related items
named elsewhere.

**Strengths**:
- Summary is a proper user story followed by a short problem/fix statement.
- Context gives concrete evidence and scopes the item to the merge rule.
- Requirements define every term needed to implement the rule.
- Each requirement maps to at least one acceptance criterion.
- Dependencies names blockers, what is blocked, and consumers.
- Drafting Notes separate user decisions from author choices.
- Frontmatter is complete and valid.

**Findings**:
- 🔵 minor / high — Open Questions — Open Questions is empty while an
  unresolved decision sits in Drafting Notes. Move the case-sensitivity
  question there, resolve it, or write "None".
- 🔵 minor / high — References — References omits 0281 and 0284, and 0283's
  relevance is never stated. Add them and one phrase on 0283.

### Dependency

**Summary**: 0306 maps its principal couplings well: the downstream blocker
0281 is reciprocated, consumers of the `also` suffix are named, and 0305 and
0311 are framed as non-blocking. Gaps are minor: it does not record reliance
on artefacts shipped by 0283 and 0280, the coupling to wherever a note
records a DOI, or the change to epic 0121's carry-tiers-verbatim contract.

**Strengths**:
- The 0281 Blocks edge is recorded in frontmatter and body, with the reason,
  and reciprocated in 0281's `blocked_by`.
- Consumers of the `also` suffix are named and no shipped code parses tier
  text.
- The 0305/0311 relationship is scoped clearly and matches those items.
- 0284 lists 0306 in `relates_to`, so the visualiser coupling is visible on
  both sides.

**Findings**:
- 🔵 minor / medium — Dependencies — Reliance on shipped 0283 composer and
  0280 tier suffixes not recorded. Add a "Relies on" line and 0283 to the
  Related line.
- 🔵 minor / medium — Assumptions — DOI-based identity depends on profiles
  recording the DOI. Name the profile/template behaviour that provides it and
  note 0305 must keep the DOI visible.
- 🔵 minor / medium — Requirements — Change to epic 0121's tier carry-forward
  contract not recorded. Add an "Updates 0121" line or state 0121 needs no
  change.
- 🔵 suggestion / low — Dependencies — Composer eval coverage (0161) may need
  to follow the contract change. Check and list 0161 if relevant.

### Scope

**Summary**: 0306 is a single coherent story: one deterministic rule for a
source cited at multiple tiers, stated in the composer and `synthesise` and
documented in the finding outputter. Boundaries are drawn explicitly against
0311, 0305, 0281 and 0284. The main scope signal is that the same-source
identity and venue rules reach citations with no tier conflict, slightly
beyond what the title and Summary promise.

**Strengths**:
- Sibling boundaries are explicit and reciprocated.
- `synthesise` stays in scope as the same rule; `report` is left to 0281.
- No shipped CLI or visualiser code parses tier text, keeping the change in
  prose.
- Summary, Requirements and Acceptance Criteria describe the same rule.
- Story is the right kind.

**Findings**:
- 🔵 minor / medium — Requirements — Identity and venue rules also cover
  duplicates that have no tier conflict. Either state that every citation
  sharing an identity merges, or confine the venue criterion to a
  tier-conflict case.
- 🔵 suggestion / low — Requirements — Same-source identity is a separate
  capability 0281 also relies on. Name the identity rule in the Summary.

### Testability

**Summary**: Testability is strong in shape: most criteria are concrete
Given/When/Then pairs with exact expected tier strings. The main gaps are
that composers and `synthesise` are LLM agents with no defined run or
permutation count for proving determinism, the DOI-merge and venue criteria
leave part of the observable output unspecified, and several requirement
clauses have no exercising criterion.

**Strengths**:
- Merge-rule criteria give exact expected output strings.
- Each requirement is largely matched by a positive example criterion.
- Drafting Notes define "named venue" and flag the case-sensitivity gap.
- Fixtures come from real observed conflicts.

**Findings**:
- 🟡 major / high — Acceptance Criteria — Order-independence criterion is
  unbounded and has no verification procedure for a non-deterministic agent.
  Name a fixture set, N permutations, K runs each, or a defined eval harness
  with a pass threshold.
- 🟡 major / medium — Acceptance Criteria — DOI-merge criterion does not
  specify the observable form of the merged citation. Give different tiers
  and state the exact expected output, identifier and venue.
- 🟡 major / medium — Requirements — Venue tied to the chosen tier has no
  criterion. Add tier-1 `Example Journal` + tier-3 `example.org` → venue
  `example.org`.
- 🔵 minor / high — Requirements — Most URL normalisation rules are not
  exercised, including the negative "query kept" case. Add a same/different
  URL-pair table.
- 🔵 minor / medium — Acceptance Criteria — Same-tier duplicates have no
  defined expected output. Add a two-tier-2 → `tier-2` criterion.
- 🔵 minor / medium — Acceptance Criteria — `synthesise` has only one
  criterion and none for suffix precedence across findings. Add a retracted
  merge criterion.
- 🔵 minor / medium — Drafting Notes — Lexicographic tie-break comparison is
  unspecified. Settle it or require a mixed-case criterion.
- 🔵 suggestion / low — Acceptance Criteria — Documentation criterion could
  name the required content (the two combined-suffix examples).

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-10-24T06:00:52+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: 'Descending order' contradicts the stated tier ordering —
  Partially resolved (Requirements fixed; Drafting Notes still says
  "descending")
- 🟡 **Testability**: Order-independence criterion is unbounded — Partially
  resolved (bounded to nine runs, but checks agreement rather than
  correctness over a loosely defined fixture)
- 🟡 **Testability**: DOI-merge criterion does not specify the observable
  form — Resolved
- 🟡 **Testability**: Venue tied to the chosen tier has no criterion —
  Resolved
- 🔵 **Clarity / Completeness / Testability**: Venue tie-break case
  sensitivity — Resolved
- 🔵 **Clarity**: 'Carry the same DOI' ambiguous for `doi.org` URLs —
  Resolved
- 🔵 **Clarity**: Retracted/withdrawn citation ownership — Resolved
- 🔵 **Clarity**: Unexplained Context phrases — Partially resolved ("root
  note" defined; "(or the higher)" still ambiguous)
- 🔵 **Scope**: Identity and venue rules cover same-tier duplicates —
  Resolved
- 🔵 **Testability**: Same-tier duplicates — Resolved
- 🔵 **Testability**: URL normalisation not exercised — Resolved
- 🔵 **Testability**: `synthesise` suffix precedence — Resolved
- 🔵 **Completeness**: References omits 0281 and 0284 — Resolved
- 🔵 **Dependency**: Reliance on 0283 and 0280 — Resolved
- 🔵 **Dependency**: DOI recording coupling — Partially resolved (recorded on
  this side only; source of DOI recording unnamed)
- 🔵 **Dependency**: 0121 contract — Resolved
- 🔵 **Clarity**: 'Never re-tier' relationship — Resolved
- 🔵 **Clarity**: Garbled 0281 phrase — Resolved
- 🔵 **Testability**: Documentation criterion content — Resolved
- 🔵 **Dependency**: 0161 eval coverage — Resolved (checked: 0161 covers
  prose shape only)

### New Issues Introduced

- 🟡 **Testability**: Chained DOI/URL matching has no acceptance criterion —
  a composer merging only direct pairs passes every criterion.
- 🟡 **Testability**: Determinism check asserts agreement, not correctness —
  "four sources" is six URLs (three psycopg issues), two conflicts read
  "tier-1 or tier-2", and no expected values or URL are asserted.
- 🟡 **Clarity**: "The citation that supplied the recorded venue" can be
  several citations — no URL tie-break when tier and venue both tie.
- 🟡 **Clarity**: "Its note" has no referent when `synthesise` merges
  findings — unclear whether findings carry the recorded DOI.
- 🔵 **Clarity**: Cited URL verbatim or normalised is unstated.
- 🔵 **Clarity**: Venue vocabulary (named venue, bare domain, bare host name)
  defined only in Drafting Notes.
- 🔵 **Testability**: Single-run criteria on an LLM agent can pass by chance.
- 🔵 **Testability**: Cited-URL selection and byte-wise tie-break untested.
- 🔵 **Testability**: `dx.doi.org` form untested.
- 🔵 **Testability**: Several `synthesise` and narrowing requirements lack a
  criterion (single-finding pass-through, identity at dossier level, no
  invented tier, composer/synthesise docs).
- 🔵 **Dependency**: 0284 not told about the combined retraction-and-`also`
  suffix forms.
- 🔵 **Dependency**: `relates_to` omits `work-item:0280`.
- 🔵 **Completeness**: Open Questions empty with no "None".
- 🔵 **Scope**: Title frames the item around tier conflicts; scope now covers
  every same-source citation.

### Assessment

The pass-1 edits resolved 15 of 20 findings, including three of four majors,
and scope and completeness are now clean. The four new majors are second-order
gaps the new rules opened: URL choice under a full tie, DOI identity at
dossier level, and a determinism fixture that needs exact inputs and expected
outputs. Each is a narrow edit; one further pass should reach COMMENT.

## Re-Review (Pass 3) — 2026-10-24T08:33:53+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability**: Chained DOI/URL matching has no criterion — Resolved
- 🟡 **Testability**: Determinism check asserts agreement, not correctness —
  Partially resolved (expected values asserted; note placement and most
  venues/URLs still unpinned)
- 🟡 **Clarity**: Several citations can supply the venue — Resolved
  (representative citation)
- 🟡 **Clarity**: "Its note" has no referent in `synthesise` — Resolved
  (`doi` metadata via 0348)
- 🔵 **Clarity**: Verbatim vs normalised cited URL — Resolved
- 🔵 **Clarity**: Venue vocabulary in Drafting Notes only — Partially
  resolved (examples in Requirements; "bare domain" boundary undefined)
- 🔵 **Clarity**: "Descending" in Drafting Notes — Resolved
- 🔵 **Clarity**: "(or the higher)" — Resolved
- 🔵 **Testability**: Single-run criteria — Resolved by decision (one-off
  fixture accepted)
- 🔵 **Testability**: Cited-URL and byte-wise tie-break untested — Partially
  resolved (scheme ordering only partly asserted)
- 🔵 **Testability**: `dx.doi.org` untested — Resolved
- 🔵 **Testability**: `synthesise` and narrowing criteria — Partially
  resolved (DOI identity, representative and metadata merge untested in
  `synthesise`)
- 🔵 **Dependency**: 0284 combined suffix forms — Partially resolved
  (recorded in 0306, not in 0284)
- 🔵 **Dependency**: `relates_to` omits 0280 — Resolved
- 🔵 **Completeness**: Open Questions empty — Resolved
- 🔵 **Scope**: Title — Resolved

### New Issues Introduced

- 🟡 **Clarity**: Pre-0348 Assumption contradicts the `doi.org`
  DOI-carrying rule.
- 🟡 **Clarity**: Metadata merge undefined when the representative citation
  lacks a disputed key.
- 🟡 **Clarity**: Merged citation's title unspecified.
- 🟡 **Testability**: Determinism fixture leaves note placement and venues
  unpinned, so it may not reproduce the root-note failure.
- 🟡 **Testability**: `synthesise` lacks DOI-identity, representative and
  metadata-merge criteria.
- 🟡 **Dependency**: 0305 does not record the constraint to keep `doi`
  metadata.
- 🔵 **Dependency**: 0281 inherits 0348 through 0306 without recording it.
- 🔵 **Testability**: Repeated non-chosen tiers and non-representative
  retraction untested.
- 🔵 **Clarity**: Fixture's "root note last" order underspecified; metadata
  examples off 0348's vocabulary; trailing slash before a query.
- 🔵 **Scope**: Hard block on all of 0348; Summary narrower than
  Requirements; story size.
- 🔵 **Completeness**: Metadata-merge example key `openalex` vs 0348's
  `openalex_id`.

### Assessment

Pass 2's majors are resolved or narrowed, but each pass's sharper rules
expose second-order edges: six majors again, all narrow. Five are fixable
inside 0306 with targeted criteria and two clauses; one needs an edit to
0305. The review is converging on detail rather than structure.

## Re-Review (Pass 4) — 2026-10-24T08:38:27+00:00

Targeted pass: clarity and testability only, after pass-3 fixes (which also
edited 0305, 0281 and 0284 to record their side of 0306's couplings).

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: Pre-0348 Assumption contradicts DOI carrying — Resolved
- 🟡 **Clarity**: Metadata merge when the representative lacks a key —
  Resolved (citation order)
- 🟡 **Clarity**: Merged title unspecified — Resolved
- 🟡 **Testability**: Fixture note placement unpinned — Partially resolved
  (placement pinned; venues/URLs pinned for 2 of 7 sources)
- 🟡 **Testability**: `synthesise` DOI/representative/metadata criteria —
  Resolved
- 🟡 **Dependency**: 0305 constraint — Resolved (recorded on 0305)
- 🔵 Repeated non-chosen tiers, non-representative retraction, scheme
  ordering, trailing slash before query, fixture orders, metadata example
  forms, Summary scope, 0281/0284 reciprocity — Resolved

### New Issues Introduced

- 🟡 **Clarity / Testability**: Citation order has no final tie-break;
  citations tied on tier, venue and URL (routine under 0348's `accessed`)
  have no single title or metadata value.
- 🟡 **Clarity**: "Merge rule" names both the tier-text bullet and the whole
  rule, so the documentation requirement can be met by stating part of it.
- 🟡 **Clarity / Testability**: Bare-domain definition does not settle
  single-label venues such as `GitHub`, on which the fixture depends.
- 🔵 **Clarity**: "Cite every source once" vs `synthesise` leaving
  single-finding sources unchanged (older findings may hold duplicates).
- 🔵 **Clarity**: Rule parts named inconsistently across sections.
- 🔵 **Testability**: No-venue-sorts-last untested; `synthesise` never fed
  `withdrawn`; fixture venues/URLs pinned for 2 of 7 sources.
- 🔵 **Clarity**: "depth-3", "pairs" unexplained.

### Assessment

Three narrow majors remain, one a design decision (the final tie-break), two
wording. The item is close; a further targeted pass should reach COMMENT.

## Re-Review (Pass 5) — 2026-10-24T08:46:17+00:00

Targeted pass: clarity and testability only.

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity / Testability**: Citation order has no final tie-break —
  Resolved (**Full ties**: per-field case-folded then byte-wise)
- 🟡 **Clarity**: "Merge rule" used in two senses — Resolved
  (**citing-once rule** names the whole)
- 🟡 **Clarity / Testability**: `GitHub` as bare domain — Resolved
- 🔵 Single-finding duplicates in `synthesise`, rule-part naming, no-venue
  criterion, `withdrawn` in `synthesise`, fixture venues, "pair"/"depth" —
  Resolved

### New Issues Introduced

- 🟡 **Clarity / Testability**: The citing-once rule's defined range ended
  at **Metadata merge**, excluding **Full ties**.
- 🟡 **Clarity**: "The merge rule is the only transformation" of tier text
  contradicted **Retraction suffixes**.
- 🔵 "Lowest tier" direction in Summary; base tier vs suffixed tier text in
  ordering; host-name label characters; who sets presentation order; title
  full tie untested; URL table rows not stated as separate compositions;
  eugene-khyst tier-1/2 inputs; no different-DOI negative case; "older
  finding" and "dossier" implicit.

### Assessment

Both majors were wording slips introduced by pass 4's edits and are fixed,
along with every minor finding, after this pass. Two suggestions were
declined: multi-run checks for single-scenario criteria (the user chose a
one-off fixture) and an uppercase metadata DOI case (0348 stores DOIs
lowercase). No design question remains open.

## Re-Review (Pass 6) — 2026-10-24T09:02:51+00:00

Targeted pass: clarity and testability only.

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity / Testability**: Citing-once rule's range excluded
  **Full ties** — Resolved
- 🟡 **Clarity**: "Only transformation" contradicted **Retraction
  suffixes** — Resolved
- 🔵 Lowest-tier direction, base tier in ordering, host-name labels,
  presentation order, title full tie, URL table compositions, fixture
  inputs, different-DOI negative, "older finding", "dossier" — Resolved

### New Issues Introduced

- 🟡 **Clarity**: Venue-classification sentence garbled by the pass-5 edit;
  named/bare-domain boundary not stated as exhaustive.
- 🔵 **Clarity**: Representative citation and Metadata merge read as
  absolute although **Full ties** overrides them; "cited once across its
  findings" ambiguous; 0283 described as the cause of conflicts.
- 🔵 **Testability**: URL match with conflicting DOIs untested; merged
  block key order unasserted; chaining untested in `synthesise`; "cited
  once" did not name the Sources section.

### Assessment

One major, below the REVISE threshold of two. All findings above were
fixed after this pass: the venue sentence rewritten with an exhaustive
named/bare split, both rules pointed at **Full ties**, the `synthesise`
pass-through reworded, 0283 re-described, and four criteria plus a
definition of "cited once" added. The work item is acceptable for planning.

## Verdict Change — 2026-10-24T09:04:29+00:00

**Verdict:** APPROVE

Changed from COMMENT by the reviewer after the pass-6 findings were
addressed; the work item is ready for implementation.
