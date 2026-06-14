---
type: work-item-review
id: "0048-linear-integration-review-1"
title: "Work Item Review: Linear Integration"
date: "2026-06-14T18:06:20+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0048"
work_item_id: "0048"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 6
tags: []
last_updated: "2026-06-14T20:57:08+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Linear Integration

**Verdict:** REVISE

This is a strong, richly-specified story — structurally complete, internally
consistent, well-bounded around a single coherent capability, and explicit
about how it parallels and diverges from the established Jira integration. It
falls short of APPROVE on two counts: half of the eight skills (search, show,
update, comment) have no acceptance criterion despite the Summary promising a
"full CRUD lifecycle", and two acceptance criteria (pagination, rate-limit
handling) use unbounded or subjective language that cannot be definitively
verified. A dependency on the separate "configurable integrations path" change
is also buried in Drafting Notes rather than recorded in Dependencies.

### Cross-Cutting Themes

- **`work_item_id` writeback target is unidentified** (flagged by: clarity,
  testability) — Both lenses independently note that the create criterion writes
  `work_item_id` "in the local work item file" without saying *which* file or
  its precondition. This is the work item's flagged net-new capability, so a
  precise referent matters most here.
- **Rate-limit handling under-specified** (flagged by: clarity, testability) —
  Clarity notes the signal is described two ways (GraphQL error extension vs.
  HTTP header); testability notes "surfaced clearly" has no measurable threshold.
  Both point at the same criterion needing a concrete, observable contract.
- **"Full CRUD" claim vs. actual coverage** (flagged by: clarity, testability) —
  The Summary's "full CRUD lifecycle" overshoots both the enumerated operations
  (no delete) and the acceptance criteria (only 4 of 8 skills covered).

### Findings

#### Critical

- *(none)*

#### Major

- 🟡 **Testability**: Four of eight skills have no Acceptance Criterion
  **Location**: Acceptance Criteria
  The Summary promises eight skills covering "the full CRUD lifecycle", but the
  AC only verify `init-linear`, `create-linear-issue`, `transition-linear-issue`,
  and `attach-linear-issue`. There is no criterion for `search-linear-issues`,
  `show-linear-issue`, `update-linear-issue`, or `comment-linear-issue` — half
  the deliverables can be marked done without any defined check.

- 🟡 **Testability**: Pagination criterion uses unbounded "all pages"
  **Location**: Acceptance Criteria
  "All pages are retrieved transparently" has no defined bound or fixture, and
  the 10,000 complexity-point cap could truncate large result sets. A verifier
  can claim a pass against a two-page fixture while large/capped sets still fail.

- 🟡 **Testability**: Rate-limit criterion's "surfaced clearly" has no threshold
  **Location**: Acceptance Criteria
  "Surfaced clearly, not treated as a generic failure" is subjective — no exit
  code, message format, or distinguishable error type to assert against, so the
  criterion cannot conclusively pass or fail.

- 🟡 **Dependency**: Configurable-integrations-path dependency absent from
  Dependencies
  **Location**: Drafting Notes
  Drafting Notes say "the integrations path is being made configurable in a
  separate change", and Requirements/AC repeatedly rely on "the configured
  integrations path". This cross-work coupling appears only in Drafting Notes,
  not in the Dependencies section — a blocker discoverable only at
  implementation time.

#### Minor

- 🔵 **Clarity**: Referent of "the local work item file" is unspecified
  **Location**: Acceptance Criteria
  The writeback criterion does not identify which file the remote identifier is
  written to — a passed-in existing file, the current item, or a newly created
  one — leaving the file's identity to interpretation.

- 🔵 **Clarity**: Acronym "ADF" used without definition
  **Location**: Context
  "ADF" appears in Context and Technical Notes without expansion; a reader of
  this work item alone must guess it means Atlassian Document Format.

- 🔵 **Clarity**: Rate-limit signal described as both GraphQL extension and HTTP
  header
  **Location**: Context
  RATELIMITED is framed as a GraphQL error extension while the backoff input is
  framed as the `X-RateLimit-Requests-Reset` HTTP header; it is not stated where
  the reset value actually lives.

- 🔵 **Completeness**: Context describes Linear's API mechanics but not the
  motivation
  **Location**: Context
  Context explains *how* Linear's API behaves but not *why* the integration is
  wanted or whom it serves — the motivation is implicit in the parent epic.

- 🔵 **Dependency**: External Linear GraphQL API not named in Dependencies
  **Location**: Dependencies
  Every skill depends on `api.linear.app/graphql` with hard rate-limit ceilings,
  but this external-system coupling and its SLA implications live only in Context
  and Technical Notes, not Dependencies.

- 🔵 **Scope**: Story-kind sizing is at the upper bound for an L integration
  **Location**: Technical Notes: Size
  Declared `kind: story` but self-described as Size L, ~30–40 files paralleling
  Jira's footprint; the comparable Jira integration is itself a top-level epic
  stream. Sits at the boundary where decomposition might give better checkpoints.

- 🔵 **Testability**: Binary upload criterion's "embedded in the issue" lacks a
  target
  **Location**: Acceptance Criteria
  "The resulting URL is embedded in the issue" does not say where (description,
  comment, or attachment record), unlike the precise parallel link criterion.

- 🔵 **Testability**: `work_item_id` writeback criterion does not specify file or
  precondition
  **Location**: Acceptance Criteria
  No specified target file or starting state, so a tester cannot set up a
  deterministic fixture to assert the writeback against the correct file.

#### Suggestions

- 🔵 **Clarity**: "full CRUD lifecycle" label vs. the listed operations
  **Location**: Summary
  CRUD implies a delete; Requirements list create/read/update/transition/
  comment/attach with no delete. "Full CRUD" may read as an omission.

- 🔵 **Dependency**: Bare-number dependencies not annotated
  **Location**: Dependencies
  "Blocked by: 0046" / "Blocks: 0051" carry no titles or reasons, so the nature
  of each coupling is not self-evident from this work item.

- 🔵 **Scope**: `attach-linear-issue` bundles two independent attachment
  mechanisms
  **Location**: Requirements: attach-linear-issue
  Link-based (`attachmentCreate`) and binary (pre-signed `fileUpload`) have no
  shared API surface and could be delivered/verified incrementally.

### Strengths

- ✅ Acceptance Criteria use explicit Given/When/Then with named skills as
  actors, so the performer and trigger of each action are unambiguous.
- ✅ Frontmatter is fully populated and valid (kind, status, priority, parent,
  blocked_by/blocks); upstream blocker (0046) and downstream consumer (0051) are
  captured in both frontmatter and the Dependencies section.
- ✅ Requirements enumerate all eight skills with concrete per-skill definitions,
  and the densely-populated Assumptions/Technical Notes capture the non-obvious
  implementation facts (auth header format, native Markdown, team-scoped
  WorkflowState caching, pre-signed upload flow).
- ✅ The transition criterion is precisely testable ("without a live catalogue
  lookup"), and the rate-limit trigger pins the exact HTTP 400 + RATELIMITED
  condition.
- ✅ Scope boundaries are explicitly stated and defended in Drafting Notes
  (single-team scoping, eight skills vs. the epic's seven, no ADF layer), and
  the two net-new divergences from Jira (`work_item_id` writeback, dual attach)
  are clearly identified versus what is copied.

### Recommended Changes

1. **Add acceptance criteria for the four uncovered skills** (addresses: Four of
   eight skills have no Acceptance Criterion) — Add observable Given/When/Then
   pairs for `search-linear-issues`, `show-linear-issue`, `update-linear-issue`,
   and `comment-linear-issue`, e.g. "Given init-linear has been run, when
   search-linear-issues is invoked with a filter, then matching issues are
   returned."

2. **Bound the pagination criterion** (addresses: Pagination criterion uses
   unbounded "all pages") — Rephrase with a concrete fixture and termination
   condition, e.g. "Given a query returning 3 pages of 50 issues, all 150 are
   returned in one result set and pagination continues until `pageInfo.hasNextPage`
   is false", and state the expected behaviour when a single query exceeds the
   10,000-point complexity cap.

3. **Make the rate-limit criterion measurable** (addresses: "surfaced clearly"
   has no threshold; rate-limit signal described two ways) — Specify the
   observable distinction (e.g. "exits with the dedicated RATELIMITED transport
   exit code per EXIT_CODES.md, prints a message naming the rate limit, and
   reports the computed backoff in seconds") and clarify that RATELIMITED is read
   from the GraphQL error extensions while the reset epoch-ms is read from the
   `X-RateLimit-Requests-Reset` HTTP header.

4. **Record the configurable-integrations-path dependency** (addresses:
   Configurable-integrations-path dependency absent from Dependencies) — Add it
   to Dependencies as a Blocked-by (or soft prerequisite, parallel-developable)
   entry with its work item ID if one exists; optionally add the external Linear
   GraphQL API as an external-system dependency noting the rate-limit ceiling.

5. **Pin the `work_item_id` writeback target** (addresses: referent of "the
   local work item file" unspecified; writeback criterion does not specify file)
   — Name the input explicitly, e.g. "Given a local work item file with a numeric
   work_item_id, when create-linear-issue is invoked against it, then that file's
   work_item_id frontmatter is overwritten with the remote key (e.g. BLA-123)."

6. **Tidy minor clarity items** (addresses: ADF undefined; "full CRUD" label;
   binary upload target) — Expand ADF on first use; soften "full CRUD" to "the
   issue lifecycle" or qualify the absence of delete; name where the binary
   upload URL is embedded.

7. **Annotate dependencies and confirm sizing** (addresses: bare-number
   dependencies; story sizing at upper bound; dual-attach bundling) — Add
   titles/reasons to the 0046/0051 entries; either confirm the L story is
   intentionally indivisible (note why) or consider splitting a foundation story;
   optionally note the two attach branches may be delivered incrementally.

## Per-Lens Results

### Clarity

**Summary**: The work item is generally clear and internally consistent: actors
and triggers are well-named in the Given/When/Then criteria, and key mechanisms
(rate-limit detection, WorkflowState caching, dual-attach) are described
unambiguously. The main clarity concerns are an undefined acronym (ADF), an
under-specified referent for "the local work item file" in the writeback
criterion, and a minor terminology drift in how the rate-limit signal is
described across sections (GraphQL error extensions vs. an HTTP response header).

**Strengths**:
- Acceptance Criteria use explicit Given/When/Then with named skills as actors
  (init-linear, create-linear-issue, transition-linear-issue), so the performer
  and trigger of each action are unambiguous.
- The eight-vs-seven skills discrepancy with the parent epic is pre-empted and
  explained in Drafting Notes, removing a potential cross-document contradiction.
- The rate-limit mechanism (HTTP 400 + "code": "RATELIMITED") is defined in
  Context before being relied on in Acceptance Criteria.
- The "two divergences from the Jira pattern" section explicitly flags
  work_item_id writeback and dual-attach as net-new.

**Findings**:
- 🔵 minor (high) — Context — **Acronym 'ADF' used without definition**: "ADF"
  appears in Context and Technical Notes without expansion or link; a reader of
  this work item alone sees only the acronym. Suggestion: expand on first use,
  e.g. "the Jira ADF (Atlassian Document Format) integration".
- 🔵 minor (medium) — Acceptance Criteria — **Referent of 'the local work item
  file' is unspecified in writeback criterion**: the second AC writes the
  identifier "in the local work item file" with no antecedent establishing which
  file. Suggestion: name the referent, e.g. "the local work item file passed to
  create-linear-issue".
- 🔵 minor (medium) — Context — **Rate-limit signal described as both a GraphQL
  error extension and an HTTP header**: Context frames RATELIMITED as a GraphQL
  extension while backoff is computed from `X-RateLimit-Requests-Reset`; where
  the reset value lives is not stated. Suggestion: clarify the two sources are
  distinct.
- 🔵 suggestion (low) — Summary — **'full CRUD lifecycle' label vs. the listed
  operations**: Requirements list no delete; "full CRUD" may be read as an
  omission. Suggestion: soften to "the issue lifecycle" or qualify.

### Completeness

**Summary**: This story is structurally complete and richly populated: all
expected sections are present with substantive content, frontmatter is valid and
complete (kind: story, status: draft, priority, parent, blocked_by/blocks), and
the Requirements enumerate all eight skills with clear per-skill definitions
matched by seven specific Acceptance Criteria. The main completeness gap is that
the Context section explains the technical mechanics of Linear's API rather than
the motivation for the work, which is a minor weakness for a story whose 'why'
lives in the parent epic.

**Strengths**:
- Frontmatter is fully populated and valid; status (draft) is appropriate for an
  unstarted item.
- Requirements enumerate all eight skills with a concrete one-line definition for
  each.
- Acceptance Criteria contains seven specific Given/When/Then criteria, far
  exceeding the two-criterion minimum.
- Assumptions and Technical Notes are densely populated with the non-obvious
  facts an implementer needs; Drafting Notes explain the deviation from the epic.

**Findings**:
- 🔵 minor (medium) — Context — **Context describes Linear's API mechanics but
  not the motivation for the work**: Context explains *how* Linear's API behaves
  but not *why* the integration is wanted or what user need it serves.
  Suggestion: add a sentence or two stating the user-facing motivation, keeping
  the API-mechanics detail as supporting material.

### Dependency

**Summary**: The work item captures its primary work-item-level couplings
explicitly: it names an upstream blocker (0046) and a downstream consumer (0051)
in both frontmatter and the Dependencies section. However, two implied couplings
are left out of Dependencies: a configuration change that the storage paths
depend on (called out only in Drafting Notes), and the external Linear GraphQL
API whose rate limits and availability gate every skill.

**Strengths**:
- Upstream blocker (0046) and downstream consumer (0051) are explicitly captured
  in both frontmatter and the Dependencies section.
- The credential prerequisite (Linear personal API key via
  ACCELERATOR_LINEAR_TOKEN_CMD indirection) is explicitly stated in Assumptions.
- The Jira integration prerequisite is correctly treated as already-complete and
  the specific files/patterns to mirror are anchored concretely.

**Findings**:
- 🟡 major (high) — Drafting Notes — **Configurable-integrations-path dependency
  absent from Dependencies**: Drafting Notes say the integrations path is "being
  made configurable in a separate change", and Requirements/AC rely on "the
  configured integrations path", but the coupling appears only in Drafting Notes.
  Suggestion: add it to Dependencies as a Blocked-by entry (or soft prerequisite).
- 🔵 minor (high) — Dependencies — **External Linear GraphQL API not named in
  Dependencies**: every skill calls `api.linear.app/graphql` with hard rate
  limits; this external-system coupling lives only in Context and Technical
  Notes. Suggestion: add the API as an external-system dependency with its
  rate-limit ceiling and the non-standard 400 RATELIMITED behaviour.
- 🔵 suggestion (medium) — Dependencies — **Bare-number dependencies not
  annotated**: "Blocked by: 0046" / "Blocks: 0051" carry no titles or reasons.
  Suggestion: annotate each with the item's title and a short reason.

### Scope

**Summary**: This is a well-bounded story implementing one coherent capability —
a Linear integration as a sibling to the existing Jira integration — with a
clear, single delivery unit (eight skills plus shared transport/auth/state
helpers under one directory). All requirements serve the unified purpose of
CRUD-against-Linear, and the work item is correctly self-described as Size L with
a coherent rationale. The only scope-relevant signals are that the parent epic
lists this stream as a single 'story' that is realistically large for a story
kind, and that the eighth skill bundles two independent attachment mechanisms.

**Strengths**:
- All eight skills serve a single coherent purpose and share the same transport,
  auth, and cached-catalogue foundation.
- Summary, Requirements, and Acceptance Criteria describe the same scope
  consistently; AC bullets map cleanly back to specific requirements.
- Scope boundaries are explicitly stated and defended in Drafting Notes.
- The two net-new divergences from the Jira pattern are correctly identified.

**Findings**:
- 🔵 minor (medium) — Technical Notes: Size — **Story-kind sizing is at the upper
  bound for an L, multi-helper, ~30–40 file integration**: declared `kind: story`
  but self-describes as Size L paralleling Jira's ~42-file footprint; the
  comparable Jira integration is itself an epic stream. Suggestion: confirm
  whether it stays a single story or splits a foundation story; if whole, note
  the L sizing is intentionally indivisible.
- 🔵 suggestion (medium) — Requirements: attach-linear-issue —
  **attach-linear-issue bundles two independent attachment mechanisms**: link
  (`attachmentCreate`) and binary (pre-signed `fileUpload` → PUT) share no API
  surface. Suggestion: acceptable as one skill, but consider noting the branches
  may be delivered/verified incrementally.

### Testability

**Summary**: The Acceptance Criteria are largely well-formed: each is framed as a
Given/When/Then observable behaviour with concrete API mechanisms that admit a
definitive pass/fail. The main gaps are unbounded scope language in the
pagination criterion, a vague 'surfaced clearly' threshold in the rate-limit
criterion, and three/four skills named in Requirements that have no corresponding
Acceptance Criterion, leaving parts of the Summary's stated CRUD intent
unverified.

**Strengths**:
- Most criteria specify a concrete, observable API outcome.
- The transition criterion is precisely testable ("without a live catalogue
  lookup", a falsifiable condition).
- The rate-limit criterion pins the exact trigger (HTTP 400 + RATELIMITED).

**Findings**:
- 🟡 major (high) — Acceptance Criteria — **Pagination criterion uses unbounded
  'all pages' without a defined scope**: no bound or fixture, and the 10,000
  complexity-point cap could truncate large sets. Suggestion: rephrase with a
  concrete bound and state behaviour when a single query exceeds the cap.
- 🟡 major (medium) — Acceptance Criteria — **Rate-limit criterion's 'surfaced
  clearly' has no measurable threshold**: subjective with no exit code, message
  format, or error type to assert. Suggestion: tie it to the EXIT_CODES.md range
  and a named message + backoff delay.
- 🟡 major (high) — Acceptance Criteria — **Four of eight skills have no
  Acceptance Criterion**: only init, create, transition, attach are covered;
  search, show, update, comment have none. Suggestion: add observable
  input-output criteria for the uncovered skills.
- 🔵 minor (medium) — Acceptance Criteria — **Binary upload criterion's 'embedded
  in the issue' lacks a verifiable target**: does not say where the URL is
  embedded. Suggestion: name the target (description Markdown link, or attachment
  record).
- 🔵 minor (medium) — Acceptance Criteria — **work_item_id writeback criterion
  does not specify the file or precondition**: no target file or starting state.
  Suggestion: specify the input, e.g. "Given a local work item file with a
  numeric work_item_id, when create-linear-issue is invoked against it, then that
  file's work_item_id is overwritten with the remote key".

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-06-14T20:19:57+00:00

**Verdict:** REVISE

All five lenses were re-run against the edited work item. Every original
finding was resolved or adequately addressed, but the edit that added
acceptance criteria for the four previously-uncovered skills introduced three
new major testability findings: the new search/show/update criteria assert that
results are "returned", "displayed", or "updated" without any observable
assertion, so they are tautological. The Summary edit also introduced a minor
count mismatch (a six-verb parenthetical describing eight skills). The verdict
remains REVISE on the strength of the three new major findings (threshold: 2).

### Previously Identified Issues

- 🟡 **Testability**: Four of eight skills have no Acceptance Criterion —
  **Partially resolved**. Criteria were added for all four skills, but the
  search/show/update criteria are non-measurable (see new issues below); the
  structural gap is closed but the verification gap persists in a new form.
- 🟡 **Testability**: Pagination criterion uses unbounded "all pages" —
  **Resolved**. Now backed by a concrete 3-page/150-issue fixture and a
  `pageInfo.hasNextPage` termination condition.
- 🟡 **Testability**: Rate-limit criterion's "surfaced clearly" has no threshold
  — **Resolved**. Now names the dedicated exit code, message content, and the
  `X-RateLimit-Requests-Reset` header; the lens cites it as exemplary.
- 🟡 **Dependency**: Configurable-integrations-path dependency absent from
  Dependencies — **Resolved**. Recorded as an annotated Blocked-by 0046 entry,
  and Drafting Notes now name 0046 instead of a vague "separate change".
- 🔵 **Clarity**: Referent of "the local work item file" unspecified — **Resolved**.
- 🔵 **Clarity**: Acronym "ADF" used without definition — **Resolved** (expanded
  inline to "Atlassian Document Format").
- 🔵 **Clarity**: Rate-limit signal described two ways — **Resolved** (criterion
  now distinguishes the GraphQL extension code from the HTTP reset header).
- 🔵 **Completeness**: Context describes mechanics but not motivation —
  **Resolved** (motivation sentence added).
- 🔵 **Dependency**: External Linear GraphQL API not named in Dependencies —
  **Resolved** (added with its rate-limit ceiling).
- 🔵 **Scope**: Story sizing at the upper bound for an L — **Addressed**. The
  indivisibility rationale added to Technical Notes is accepted by the scope
  lens as sound; remains a noted-but-acceptable boundary.
- 🔵 **Testability**: Binary upload "embedded in the issue" lacks a target —
  **Resolved** (now the issue's `attachments` connection).
- 🔵 **Testability**: work_item_id writeback file unspecified — **Resolved**
  (precondition and target file now pinned).
- 🔵 **Suggestion / Clarity**: "full CRUD" label — **Resolved** (softened to the
  enumerated lifecycle).
- 🔵 **Suggestion / Dependency**: Bare-number dependencies — **Resolved**
  (annotated with titles and reasons).
- 🔵 **Suggestion / Scope**: attach-linear-issue bundles two mechanisms —
  **Addressed** (Drafting Notes now note incremental link-then-binary delivery).

### New Issues Introduced

- 🟡 **Testability** (major): `search-linear-issues` criterion has no observable
  pass/fail assertion — does not say what filter is applied or what "matching"
  means; an empty or unfiltered set could both be argued to pass.
- 🟡 **Testability** (major): `show-linear-issue` criterion does not state which
  fields must be displayed — any output mentioning the issue could be claimed as
  passing.
- 🟡 **Testability** (major): `update-linear-issue` criterion lacks a concrete
  field/value to verify — restates the action as the outcome with no re-read
  assertion.
- 🔵 **Testability** (minor): `comment-linear-issue` criterion does not verify
  comment content or retrievability.
- 🔵 **Testability** (minor): pagination criterion now bundles an
  under-specified complexity-cap behaviour (no trigger fixture / concrete
  observable).
- 🔵 **Testability** (minor): transition criterion does not state how the applied
  state is confirmed; init-linear criterion does not state what counts as a
  complete persisted catalogue.
- 🔵 **Clarity** (minor): Summary's lifecycle parenthetical lists six verbs but
  describes eight skills (omits init and search) — introduced by the Summary
  edit.
- 🔵 **Completeness** (minor): Open Questions remains an empty placeholder while
  deferred decisions live in Drafting Notes.

### Assessment

The work item improved substantially: every original finding is resolved or
adequately addressed, and the rate-limit, pagination, and dependency criteria
are now strong. However, it is not yet ready for implementation — the four
acceptance criteria added for search/show/update/comment need concrete fixtures
and observable assertions (matching the precision of the RATELIMITED and
pagination criteria), and the Summary parenthetical should be aligned to the
eight skills. These are quick, well-scoped edits; once applied, a further pass
should clear the verdict to APPROVE.

## Re-Review (Pass 3) — 2026-06-14T20:29:37+00:00

**Verdict:** REVISE

The Summary parenthetical was fixed (all eight skills now enumerated, resolving
the pass-2 clarity finding), and the four thin criteria were tightened. The
`search` criterion is now resolved (concrete fixture, exact expected result) and
`show`/`comment` are materially improved. However, the testability lens surfaced
three remaining major findings: the `update` criterion verifies only 1 of the 5
fields it now claims; the complexity-cap clause (added in pass 1) still lacks a
trigger fixture and observable signal; and the `transition` criterion's "without
a live catalogue lookup" clause is an internal mechanism, not an observable
outcome. The first two are direct artifacts of the iterative edits; the third is
notable lens variance — pass-1 testability praised the same clause as
"precisely testable". Verdict remains REVISE on three majors (threshold: 2).

### Previously Identified Issues (pass 2 → pass 3)

- 🟡 **Testability**: `search-linear-issues` criterion non-measurable —
  **Resolved**. Now a 5-issue fixture (2 in state X) returning exactly those 2
  by identifier.
- 🟡 **Testability**: `show-linear-issue` criterion non-measurable — **Partially
  resolved** (downgraded to minor). Now enumerates required fields; still tests
  field presence rather than values against a fixture.
- 🟡 **Testability**: `update-linear-issue` criterion non-measurable —
  **Partially resolved** (still major). Title round-trip is now verifiable, but
  the criterion claims a 5-field updatable set while exercising only title.
- 🔵 **Clarity**: Summary parenthetical six verbs vs eight skills — **Resolved**
  (all eight skills now enumerated by name).
- 🔵 **Testability**: `comment-linear-issue` doesn't verify content —
  **Resolved** (re-fetch comments, body equals submitted Markdown).
- 🔵 **Completeness**: Open Questions empty placeholder — **Still present**
  (minor; unchanged).

### New Issues Introduced / Newly Surfaced

- 🟡 **Testability** (major): `update-linear-issue` verifies only 1 of 5 named
  updatable fields — four-fifths of the claimed capability is unverified.
- 🟡 **Testability** (major): complexity-cap branch lacks a defined trigger
  fixture and observable pass condition (exit code / message).
- 🟡 **Testability** (major): `transition-linear-issue`'s "without a live
  catalogue lookup" asserts an unobservable internal mechanism — *lens variance*
  vs. pass 1, which praised it.
- 🔵 **Testability** (minor): `show` tests field presence not values; init-linear
  criterion doesn't pin catalogue contents; RATELIMITED backoff has no worked
  example to assert the ms→s conversion.
- 🔵 **Clarity** (minor): placeholder `X` reused for both a state value and a
  title string; storage location named two ways (body phrase vs. concrete
  `.accelerator/state/integrations/linear/`); WorkflowState name→UUID map shape
  lightly defined.
- 🔵 **Dependency** (minor): Linear API key provisioning and the init-linear-first
  ordering constraint are implied but not in Dependencies.

### Assessment

The work item is, in substance, implementation-ready: three full passes have
resolved every structural, clarity, completeness, and dependency finding, and
the strong criteria (RATELIMITED, pagination, create writeback, comment) are
exemplary. The residual REVISE rests on AC-precision nits, two of which are
genuine artifacts of the iterative tightening (the `update` 5-field over-claim
and the bundled complexity-cap clause) and one of which is lens variance on a
clause judged sound in pass 1. This is the point of diminishing returns: a
fourth pass risks surfacing further purist nits without materially de-risking
implementation. Recommended path — apply a final, small set of edits (scope the
`update` criterion to title as the representative field and move the field set
to Requirements; split the complexity-cap clause into its own criterion with a
fixture and exit code, or relocate it to Technical Notes; reframe the transition
clause around an observable state change) — then either accept or run one final
confirmatory pass.

## Re-Review (Pass 4) — 2026-06-14T20:39:11+00:00

**Verdict:** REVISE (mechanical) — recommend APPROVE override; see assessment.

The three pass-3 fixes landed: the Summary enumerates all eight skills, the
`update` criterion was scoped to title with the field set moved to Requirements,
the complexity-cap clause is now its own criterion with a trigger fixture and
exit code, and the transition criterion now asserts an observable state change
plus a stub-the-catalogue verification of the cached-UUID path. The mechanical
verdict remains REVISE, but the character of the findings has shifted decisively:
the residual "majors" are no longer 0048 quality defects.

### Previously Identified Issues (pass 3 → pass 4)

- 🟡 **Testability**: complexity-cap clause lacked trigger/observable —
  **Resolved**. Now a standalone criterion with a complexity-limit GraphQL-error
  fixture, dedicated exit code, named limit, and no-partial-result assertion.
- 🟡 **Testability**: transition "without a live catalogue lookup" unobservable —
  **Resolved**. Now asserts the re-fetched target state *and* makes the
  cached-UUID path verifiable by stubbing the catalogue endpoint to fail.
- 🟡 **Testability**: `update` over-claimed 5 fields, verified 1 — **Addressed**.
  Criterion scoped to title as the representative case; full field set moved to
  Requirements. (The testability lens still flags the four unverified fields —
  see below — but this is now a coverage-philosophy choice, not an over-claim.)
- 🔵 **Clarity**: Summary parenthetical six verbs vs eight skills — **Resolved**.
- 🔵 **Clarity**: placeholder `X` reused — **Resolved** (now `S` for state, `T`
  for title).

### Residual Findings — re-characterised

These are the findings keeping the mechanical verdict at REVISE. None is a defect
in 0048's own specification quality:

- 🟡 **Clarity / Dependency (cross-document)**: 0048's "configured integrations
  path" (`.accelerator/state/integrations/linear/`) contradicts the parent epic
  0045's repeated `meta/integrations/<system>/`. This is *epic staleness* — the
  epic predates work item 0046, which made the path configurable; 0048 is
  internally correct and even flags `meta/integrations/` as legacy/guard-banned.
  Fix belongs in 0045, not 0048.
- 🟡 **Dependency (cross-story)**: `work_item_id` writeback couples to the
  numeric-as-unsynced convention the epic assigns to sibling story 0047 (Core
  skills sync integration). A coordination note, not a 0048 defect.
- 🟡 **Testability (coverage philosophy)**: `update` verifies only title of five
  fields; `init-linear` persistence doesn't assert catalogue contents. Closing
  these means adding more round-trip fixtures — legitimate, but each pass with
  fresh agents surfaces a different criterion as "the" major, indicating lens
  variance rather than a stable defect.
- 🔵 Minor (recurring, genuinely worth a one-line fix each): Linear API key
  prerequisite and `EXIT_CODES.md` range coordination absent from Dependencies;
  `show`/create criteria assert field presence not fixture-equality; RATELIMITED
  backoff has no worked example; Open Questions empty.

### Assessment

Four passes confirm a diminishing-returns plateau. Every finding that concerns
0048's *own* clarity, completeness, scope, and internal testability has been
resolved; the strong criteria (RATELIMITED, pagination, complexity-cap, create
writeback, transition, comment) are exemplary. The remaining mechanical-REVISE
findings are (a) cross-document/cross-story coordination items whose fixes
belong in the epic 0045 and sibling 0047, and (b) AC-coverage purism where each
fresh-agent pass nominates a different criterion. Continuing to iterate 0048 in
isolation will not converge to APPROVE because the binding findings now live
outside this work item.

**Recommendation**: treat 0048 as implementation-ready and override the verdict
to APPROVE, with three cheap, genuinely-useful follow-ups captured separately:
(1) reconcile epic 0045 to eight Linear skills and the configurable integrations
path; (2) confirm in 0047 (or here) who owns the `work_item_id`/numeric-unsynced
contract; (3) optionally add the recurring one-line Dependencies notes (API key
prerequisite, EXIT_CODES.md range) and an init-linear catalogue-contents
assertion. None requires a further full lens pass to validate.

## Re-Review (Pass 5) — 2026-06-14T20:46:08+00:00

**Verdict:** REVISE (mechanical) — recommend APPROVE override; non-convergence
confirmed.

The pass-4 reconciliations landed: epic 0045 now lists eight Linear skills and
carries a configurable-path note; 0048 gained Dependencies entries for the API
key prerequisite, the `EXIT_CODES.md` range, the 0047 contract coupling, and the
init-linear ordering constraint; and the testability tightening (init catalogue
contents, show fixture values, second update field, attachment identity, backoff
worked example, create derivation) was applied. The pass-5 lenses confirmed all
pass-4 residuals resolved — and surfaced a *fresh* set of three majors on
criteria no prior pass had flagged.

### Previously Identified Issues (pass 4 → pass 5)

- 🟡 State-path contradicts epic — **Resolved** (epic 0045 reconciled: eight
  skills + configurable-path note; 0048 Drafting Notes states supersession).
- 🟡 `work_item_id` convention coupling uncaptured — **Resolved as prose**
  (Dependencies names 0047 as the owning story).
- 🟡 `update` 5-field over-claim / `init` persistence shape — **Addressed**
  (catalogue-contents assertion added; update scoped, field set in Requirements).
- 🔵 API key / EXIT_CODES.md / show-fixture / backoff-example — **Resolved**.

### New Issues Surfaced (pass 5)

- 🟡 **Dependency** (major): 0047 coupling in prose but not in `blocked_by` —
  *addressed post-pass-5*: Dependencies now states 0047 is a definitional
  reference, not an ordering blocker, hence intentionally absent from
  `blocked_by`.
- 🟡 **Testability** (major): init-linear multi-team discovery/selection not
  covered — *addressed post-pass-5*: multi-team selection criterion added.
- 🟡 **Testability** (major): `update` verifies 2 of 5 fields — *addressed
  post-pass-5*: criterion now states description/assignee/priority are covered by
  implementation-level tests, not acceptance criteria.
- 🔵 Minor (deferred as acceptable polish): create title/description mapping not
  exact-match; RATELIMITED scoped to "any Linear skill" rather than the
  `linear-graphql.sh` transport; comment criterion lacks a Markdown-construct
  fixture; Summary "become the work_item_id" vs the AC overwrite-numeric
  precondition; Technical Notes presents pagination placement as still-open.

### Assessment — non-convergence confirmed; closing recommendation

Five passes establish that this review will not mechanically reach APPROVE by
continued iteration. The major-finding count holds at ~3 per pass but on
**different criteria each time** (pass 1: missing ACs + pagination + rate-limit +
dependency; pass 2: the new ACs' tautology; pass 3: update/transition/complexity;
pass 4: cross-document; pass 5: 0047 frontmatter + multi-team + update coverage).
Two dynamics drive this: (1) **lens variance** — fresh agents nominate a
different "the major" each pass, and one clause (transition's no-live-lookup) was
praised in pass 1 then flagged in pass 3; (2) **unbounded coverage requests** —
the testability lens will always find an Nth field, skill path, or fixture not
yet pinned by an acceptance criterion, which is a coverage-philosophy question
(which behaviours belong in ACs vs. implementation tests), not a defect.

Every finding bearing on 0048's own substance — structure, clarity,
completeness, scope, and the core testable behaviours (auth/persist, create +
writeback, transition with cached-UUID proof, dual attach, rate-limit,
pagination, complexity-cap) — has been resolved across the five passes, the
parent epic has been reconciled, and the post-pass-5 edits address this pass's
three majors.

**Final recommendation**: override the verdict to APPROVE and treat 0048 as
implementation-ready. The residual minors are acceptable polish that an
implementing plan or PR review will naturally pin down; a sixth lens pass is
expected to surface a fresh set of variance-driven majors rather than converge.

## Re-Review (Pass 6) — 2026-06-14T20:52:07+00:00

**Verdict:** COMMENT — work item is acceptable as-is.

The sixth pass **converged**: across all five lenses there are **zero critical
and zero major findings** — only minor observations and suggestions. The
non-convergence prediction from pass 5 did not hold this round, because the
three post-pass-5 edits (0047 definitional-reference clarification, multi-team
selection criterion, update-field coverage statement) closed the pass-5 majors
and no fresh majors emerged. Under the configured thresholds (REVISE requires ≥1
critical or ≥2 major), the verdict drops from REVISE to **COMMENT**.

### Previously Identified Issues (pass 5 → pass 6)

- 🟡 0047 coupling not in `blocked_by` — **Resolved** (Dependencies states it is
  a definitional reference, intentionally not a `blocked_by` edge).
- 🟡 init-linear multi-team selection uncovered — **Resolved** (multi-team
  selection criterion added).
- 🟡 `update` verifies 2 of 5 fields — **Resolved** (criterion explicitly defers
  description/assignee/priority to implementation-level tests; the testability
  lens this pass accepts the scoping as transparent rather than a gap).

### Residual Minor Observations (non-blocking; optional polish)

- 🔵 **Clarity**: single-letter AC placeholders (X/Y/S/I/T/A/D) lack a stated
  "these are illustrative" convention; "the dedicated … exit code" resolves only
  via `EXIT_CODES.md` (value allocated at implementation); init-linear "team
  details … and WorkflowState catalogue" wording could read as two stores vs the
  AC's single catalogue file.
- 🔵 **Completeness**: Open Questions remains an empty placeholder; the
  user/value framing lives in Context rather than the Summary.
- 🔵 **Dependency**: EXIT_CODES.md coordination names no owning party/sibling
  claimants; `blocks: 0051` is contributory (Jira already satisfies "≥1
  integration") rather than a hard gate; the 0047 contract should be *specified*
  before create-linear-issue is implemented; the binary-attach pre-signed upload
  host is a second external endpoint distinct from the GraphQL API.
- 🔵 **Scope**: attach-linear-issue's link/binary bundling — acknowledged, no
  change required.
- 🔵 **Testability**: backoff "within a stated tolerance" is unquantified (give a
  concrete ±bound); create-linear-issue title/description derivation mapping is
  not pinned; no failure-path criteria (Bearer-prefixed token; double-push of an
  already-synced file); pagination criterion doesn't assert the request count /
  cursor-following so a single oversized fetch could pass.

### Assessment

The work item is **acceptable for implementation as-is**. Six passes have
resolved every critical/major finding; the parent epic 0045 is reconciled; and
the surviving items are minor, optional polish that an implementing plan or PR
review will naturally pin down (notably: a concrete backoff tolerance, the
file→issue title/description mapping, and a couple of failure-path criteria, if
the team wants belt-and-braces verification coverage). The verdict is COMMENT
rather than APPROVE only because non-empty (minor) observations remain; there is
no blocker to proceeding.

## Verdict Override → APPROVE — 2026-06-14T20:57:08+00:00

After pass 6 (COMMENT, zero majors), the reviewer applied the genuinely-useful
minors and overrode the verdict to **APPROVE**:

- **Testability**: backoff tolerance quantified (within ±2s of
  `(reset_epoch_ms − now_epoch_ms)/1000`); create-linear-issue title/description
  derivation pinned (title = work item title; description = rendered Markdown
  body below frontmatter); two failure-path criteria added (Bearer-prefixed
  token → auth failure; double-push of an already-synced file → no duplicate).
- **Dependency**: `blocks: 0051` reframed as a contributory relationship (Jira
  already satisfies the "≥1 integration" prerequisite), not a hard gate.

Remaining observations (single-letter placeholder convention, empty Open
Questions, Summary value-framing, EXIT_CODES.md coordination owner, pre-signed
upload host as a second external endpoint) are accepted as optional polish.
Work item 0048 is **approved for implementation**.



