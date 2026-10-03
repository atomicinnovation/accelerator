---
type: "work-item-review"
id: "0230-tracker-owned-work-item-id-generation-review-1"
title: "Work Item Review: Tracker-Owned Work Item ID Generation"
date: "2026-09-24T23:04:07+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0230"
work_item_id: "0230"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 4
tags: []
last_updated: "2026-09-26T00:47:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Tracker-Owned Work Item ID Generation

**Verdict:** REVISE

0230 is structurally complete and precise: every section is populated,
commands and paths are named exactly, the draft-ID shape is pinned by regex,
and 16 Given/When/Then criteria cover failure paths as well as the happy path.
It needs revision for five major findings: two internal contradictions (the
draft ID must both vanish from `meta/` and persist in `aliases`; `loud-terminal`
is both "no tracker issue" and "may have created one"), an untestable redraw
rule, a 0291 blocker justified by a capability 0291 does not commit to, and a
story that bundles four separately deliverable slices.

### Cross-Cutting Themes

- **Draft-ID rewrite vs. `aliases`** (flagged by: clarity, testability) — "no
  occurrence of the draft ID remains in `meta/`" contradicts appending it to
  the promoted item's `aliases`; the criterion can never pass as written and
  checks only absence, not correct replacement.
- **Unverified Linear key grammar** (flagged by: completeness, dependency,
  testability) — the draft ID's collision guarantee rests on `<KEY>-<digits>`,
  unverified for Linear, with no open question, prerequisite, or verification
  step.
- **`aliases` scope undecided** (flagged by: completeness, scope, dependency) —
  whether `aliases` joins the canonical frontmatter standard decides whether
  the story reaches every artifact type; no default or owner is given.
- **Story size and Summary coverage** (flagged by: scope, clarity) — eight
  requirement groups and 16 ACs; the Summary omits pull adoption, legacy
  validation, `external_id` resolution, and batch extraction, and overstates
  the `id == external_id` guarantee.
- **Parent-edge handling across 0291** (flagged by: dependency, scope,
  clarity) — only batch extraction needs 0291, 0291 does not promise
  parent-at-create, and it is unclear who sets children's remote parent after
  promotion.

### Findings

#### Major

- 🟡 **Clarity + Testability**: Removing every occurrence of the draft ID conflicts with keeping it in `aliases`
  **Location**: Requirements: Promotion / Acceptance Criteria
  The promoted file lives in `meta/work/` and carries the draft ID in
  `aliases`, so "no occurrence remains in `meta/`" is unsatisfiable; the
  absence-only check would also pass a deletion instead of a replacement.
- 🟡 **Clarity**: `loud-terminal` is listed as "no tracker issue" yet may have created one
  **Location**: Requirements: Drafts / Promotion
  Drafts groups `loud-terminal` with outcomes yielding no issue, while
  promotion adopts a recorded key and the batch summary warns an issue "may
  already exist"; promotion behaviour depends on which is true.
- 🟡 **Testability**: Redraw rule has no criterion and is unobservable under a random draw
  **Location**: Requirements: Drafts
  All-digit and collision redraws are never exercised; criteria need an
  injectable draw source to force them.
- 🟡 **Dependency**: The 0291 blocker depends on a capability 0291 does not commit to
  **Location**: Dependencies
  0291 adds parent read/write and parent-first push (create then link), not
  parent-on-create, and leaves the Jira parent mechanism open; 0230 also
  relies on 0291's update-side parent push without naming it.
- 🟡 **Scope**: Story bundles several separately deliverable slices
  **Location**: Requirements
  Config + online create + pull adoption, drafts, promotion, and batch
  extraction could each ship and be verified independently; the Drafting Notes
  already flag decomposition.

#### Minor

- 🔵 **Completeness + Dependency + Testability**: Linear key grammar unverified and untracked
  **Location**: Assumptions
  The draft-ID guarantee rests on it; add a prerequisite or verification step.
- 🔵 **Clarity + Scope**: Summary overstates "always matches" and under-covers the Requirements
  **Location**: Summary
  Drafts and legacy items do not have `id == external_id`; batch extraction,
  pull adoption, and `external_id` resolution are not mentioned.
- 🔵 **Testability**: Several requirements have no acceptance criterion
  **Location**: Requirements: Drafts / Promotion / Online creation
  Draft marker in listings, nothing rewritten outside `meta/`, creation-home
  entity, legacy push sets only `external_id`, "No, save as draft" wording.
- 🔵 **Clarity**: Project terms used without definition
  **Location**: Summary / Context / Requirements
  `local-save`, `loud-terminal`, `pending_push`, "stub-creates",
  "creation-home entity", "immutability tension".
- 🔵 **Clarity**: Unclear who sets children's remote parent after promotion
  **Location**: Requirements: Batch extraction
  Promotion's listed effects omit it; also "create fails" is not tied to the
  Drafts outcome list.
- 🔵 **Scope**: Batch extraction brings in the 0291 blocker for the whole story
  **Location**: Dependencies
  Only batch parent-at-create needs 0291; the core could depend on 0228 alone.
- 🔵 **Scope**: Resolver/alias work is an independent slice 0295 waits on
  **Location**: Requirements: Identity
  `aliases` and `external_id` resolution deliver value on today's corpus.
- 🔵 **Clarity**: "Explicit re-key" used as a live mechanism but out of scope
  **Location**: Requirements: Identity / Out of scope
  State that 0295 delivers re-key; 0230 supplies only `aliases` and resolution.
- 🔵 **Dependency**: 0291 does not record that it blocks 0230
  **Location**: Dependencies
  0291 says "Blocks: none."
- 🔵 **Dependency**: New config-validation rule not linked to 0227 `config validate`
  **Location**: Requirements
  0228 requires command-time and load-time validation to stay in agreement.
- 🔵 **Testability**: Online-creation criterion exercised only with Linear
  **Location**: Acceptance Criteria
  No Jira case and no positive validation case.
- 🔵 **Testability**: `id`-immutability rule checked only against `work sync`
  **Location**: Requirements: Identity
  Name the commands that must preserve `id`.
- 🔵 **Testability**: Expected validation error content undefined
  **Location**: Acceptance Criteria
  "Naming the violated rule" has no stable rule names or example inputs.
- 🔵 **Testability**: Batch outcome summary has no defined outcome values
  **Location**: Acceptance Criteria
  Enumerate outcome categories.
- 🔵 **Testability**: Batch ordering has no observation point
  **Location**: Acceptance Criteria
  Observe recorded create calls; replace "no subsequent sync is needed" with
  an end-state assertion.
- 🔵 **Completeness**: Context does not state the user cost of `id` ≠ `external_id`
  **Location**: Context
  The immutability tension is only explained in 0146.
- 🔵 **Completeness**: Draft filename format unspecified
  **Location**: Requirements: Drafts
  E.g. `meta/work/drafts/<draft-id>-<slug>.md`, plus the H1.

#### Suggestions

- 🔵 **Completeness + Scope + Dependency**: Resolve the `aliases` open question with a default
  **Location**: Open Questions
  Default to work-item-only for 0230; defer widening to 0295; name the
  frontmatter-standard owner if widened.
- 🔵 **Clarity**: No rationale for rejecting all-digit draft suffixes
  **Location**: Requirements: Drafts
  Tie it to the key-grammar assumption.

### Strengths

- ✅ Well-formed user story Summary; Context grounded in corpus evidence (294
  items, `0230` ↔ `PP-760`).
- ✅ Requirements grouped by concern, with an explicit, specific Out of scope
  list deferring re-key to 0295.
- ✅ Commands, flags, paths, and config keys named exactly; draft ID pinned by
  regex plus a not-all-digits rule.
- ✅ Criteria cover failure paths (`local-save`, `loud-terminal`, declined push,
  `pending_push` adoption, failed batch parent) and invariants (`--no-promote`,
  non-draft `id` stability, alias and `external_id` resolution).
- ✅ Frontmatter complete; `blocked_by`/`blocks` match Dependencies and the
  reciprocal edges in 0228 and 0295.
- ✅ Legacy corpus explicitly preserved rather than migrated; tracker
  unavailability designed around rather than a delivery blocker.
- ✅ Drafting Notes record the rationale for non-obvious choices.

### Recommended Changes

1. **Decompose into sibling stories under 0146** (addresses: Story bundles
   several slices; Batch extraction brings in 0291; Resolver/alias slice;
   Summary under-covers)
   Candidate split: (0) `aliases` + alias/`external_id` resolution; (a)
   `{tracker}` config + online create + pull adoption; (b) drafts + promotion
   incl. `pending_push`; (c) batch extraction, blocked by 0291. State (a)'s
   unreachable-tracker behaviour until (b) lands.
2. **Exempt the promoted item's `aliases` from the rewrite and assert the
   replacement** (addresses: draft ID vs. `aliases`)
   Scope "every occurrence" to outside the promoted item's `aliases`; assert
   the typed link reads `work-item:<tracker-id>` and the prose contains the
   tracker ID.
3. **Define `loud-terminal` and `pending_push` precisely** (addresses:
   `loud-terminal` contradiction; undefined terms)
   Say whether the marker holds a confirmed key or only a possibility, and
   what promotion does when no key is recorded. Gloss or link every project
   term on first use.
4. **Pin down the 0291 contract** (addresses: 0291 capability; 0291 missing
   `blocks`; who sets children's remote parent)
   Name parent-on-create, parent update on existing issues, and a resolved
   Jira mechanism; get them into 0291 and add `blocks: ["work-item:0230"]`
   there. Name the component that pushes children's parent after promotion,
   for both `work sync` and `work promote`.
5. **Make the draft-ID guarantee verifiable** (addresses: redraw untested;
   Linear grammar; all-digit rationale; draft filename)
   Add criteria driven by an injectable draw source; add Linear grammar
   verification as a prerequisite; state the rationale; specify the draft
   filename and H1.
6. **Close the criterion gaps** (addresses: missing ACs; Linear-only;
   `id`-immutability; validation errors; batch summary; batch ordering)
   One AC per uncovered requirement; cover Jira and a positive validation
   case; concrete error expectations; enumerated outcome values; observable
   ordering.
7. **Tighten Summary, Context, and Identity wording** (addresses: "always
   matches"; Context user cost; explicit re-key; 0227 coupling; `aliases`
   open question)
   Scope the guarantee to items created or promoted under `{tracker}`; state
   the user-facing cost; attribute re-key to 0295; link 0227; default
   `aliases` to work-item-only.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: Tightly written with precise commands, paths, and config keys;
problems are internal contradictions (`loud-terminal` vs. "no tracker issue",
rewrite vs. `aliases`, "always matches" vs. drafts/legacy) and undefined
project terms.

**Strengths**: exact naming of entry points; regex-pinned draft ID; clear
requirement grouping; rationale in Drafting Notes.

**Findings**:
- 🟡 major / high — Requirements: Promotion / AC — Removing every occurrence of
  the draft ID conflicts with keeping it in `aliases`. Exempt the `aliases`
  entry from the rewrite.
- 🟡 major / high — Requirements: Drafts / Promotion — `loud-terminal` listed as
  "no tracker issue" yet may have created one. Define `loud-terminal` and
  `pending_push` contents; cover the no-key case.
- 🔵 minor / high — Summary — "always matches" contradicted by drafts and
  legacy items.
- 🔵 minor / medium — Requirements: Batch extraction — who sets children's
  remote parent after promotion; "create fails" undefined.
- 🔵 minor / medium — Requirements: Identity / Out of scope — "explicit
  re-key" referenced but deferred to 0295.
- 🔵 minor / high — Summary / Context / Requirements — `local-save`,
  `loud-terminal`, `pending_push`, "stub-creates", "creation-home entity",
  "immutability tension" undefined.
- 🔵 suggestion / medium — Requirements: Drafts — no rationale for the
  all-digit rejection.

### Completeness

**Summary**: Structurally very complete for a story; small gaps in Context
motivation, draft filename, and tracking of the Linear assumption.

**Strengths**: well-formed user story; evidence-grounded Context; grouped
requirements with explicit Out of scope; 16 Given/When/Then ACs; all
supporting sections populated; consistent frontmatter.

**Findings**:
- 🔵 minor / medium — Context — user-facing cost of `id` ≠ `external_id` and
  the immutability tension not stated.
- 🔵 minor / medium — Requirements: Drafts — draft filename format
  unspecified.
- 🔵 minor / medium — Assumptions — unverified Linear grammar not tracked as
  an open question.
- 🔵 suggestion / low — Open Questions — `aliases` question has no default or
  resolution point.

### Dependency

**Summary**: Main couplings captured with reciprocal edges for 0228 and 0295;
gaps in the grounding of the 0291 edge, a missing 0227 coupling, and external
precondition tracking.

**Strengths**: frontmatter matches Dependencies with reasons; reciprocal
edges hold for 0228 and 0295; 0228 marked done; re-key deferral explicit;
tracker availability handled by design.

**Findings**:
- 🟡 major / medium — Dependencies — 0291 does not commit to parent-at-create
  and leaves the Jira mechanism open; update-side parent push unnamed.
- 🔵 minor / high — Dependencies — 0291 lacks `blocks: ["work-item:0230"]`.
- 🔵 minor / medium — Requirements — `{tracker}` validation rule not linked to
  0227 `config validate`.
- 🔵 minor / medium — Assumptions — Linear grammar should be a pre-start
  prerequisite.
- 🔵 suggestion / medium — Open Questions — `aliases` ownership implicates the
  frontmatter standard.

### Scope

**Summary**: Every requirement serves one purpose with well-drawn boundaries,
but the item is epic-sized; batch extraction and identity/resolver work could
ship independently on different dependency timelines.

**Strengths**: single coherent purpose; specific Out of scope; legacy corpus
preserved; dependency edges consistent within 0146.

**Findings**:
- 🟡 major / high — Requirements — bundles four separately deliverable slices;
  split into sibling stories.
- 🔵 minor / medium — Dependencies — batch extraction brings in 0291 for the
  whole story.
- 🔵 minor / medium — Requirements: Identity — resolver/alias slice could lead
  and unblock 0295 earlier.
- 🔵 minor / medium — Summary — under-covers the Requirements.
- 🔵 suggestion / medium — Open Questions — `aliases` placement could widen
  scope beyond work items.

### Testability

**Summary**: Largely verifiable with concrete fixtures and an exact regex;
gaps are the self-contradicting promotion criterion, uncovered requirements,
and vague terms.

**Strengths**: consistent Given/When/Then with concrete values; mechanical
draft-ID check; failure paths covered; invariants stated observably; explicit
Out of scope.

**Findings**:
- 🟡 major / high — Acceptance Criteria — promotion criterion contradicts
  `aliases` and checks absence only.
- 🟡 major / high — Requirements: Drafts — redraw rule has no criterion and is
  unobservable without an injectable draw source.
- 🔵 minor / high — Requirements — five requirements without an AC.
- 🔵 minor / medium — Acceptance Criteria — online creation Linear-only; no
  positive validation case.
- 🔵 minor / medium — Requirements: Identity — immutability checked only
  against `work sync`.
- 🔵 minor / medium — Acceptance Criteria — validation error content
  undefined.
- 🔵 minor / medium — Acceptance Criteria — batch outcome values undefined.
- 🔵 minor / low — Acceptance Criteria — batch ordering has no observation
  point.
- 🔵 suggestion / low — Assumptions — Linear grammar has no verification step.

## Re-Review (Pass 2) — 2026-09-25T01:37:17+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity + Testability**: Rewrite vs. `aliases` contradiction — Resolved
- 🟡 **Clarity**: `loud-terminal` "no issue" vs. "may have created one" — Partially resolved (marker states `created`/`attempted` not mapped to create outcomes; draft write after a local-write failure unexplained)
- 🟡 **Testability**: Redraw rule untested — Resolved (existing-`id` collision case still untested)
- 🟡 **Dependency**: 0291 blocker relies on uncommitted parent-at-create — Resolved (0291 dependency removed; linkage moved to 0291)
- 🟡 **Scope**: Story bundles separately deliverable slices — Still present (accepted by author decision to keep one story; now 30 ACs)
- 🔵 **Completeness + Dependency + Testability**: Linear key grammar unverified — Resolved (verified; guarantee rests on the issue-number suffix)
- 🔵 **Clarity + Scope**: Summary overstates / under-covers — Resolved (Summary omits the `loud-terminal` draft trigger)
- 🔵 **Testability**: Requirements without ACs — Resolved
- 🔵 **Clarity**: Undefined project terms — Resolved
- 🔵 **Clarity**: Who sets children's remote parent — Resolved (0291)
- 🔵 **Scope**: Batch extraction brings in 0291 — Resolved
- 🔵 **Scope**: Resolver/alias slice independent — Still present (accepted)
- 🔵 **Clarity**: "Explicit re-key" out of scope — Partially resolved ("re-key" now names both the 0295 command and the shared operation; 0295 still lists only two `id`-changing paths)
- 🔵 **Dependency**: 0291 missing reciprocal edge — Resolved via `relates_to`
- 🔵 **Dependency**: 0227 link missing — Partially resolved (`relates_to`; 0227 not updated)
- 🔵 **Testability**: Online creation Linear-only — Resolved
- 🔵 **Testability**: `id` immutability checked only against sync — Resolved
- 🔵 **Testability**: Validation error content undefined — Resolved
- 🔵 **Testability**: Batch outcome values undefined — Partially resolved (values defined; AC does not tie inputs to outcomes)
- 🔵 **Testability**: Batch ordering unobservable — Resolved
- 🔵 **Completeness**: Context user cost — Resolved
- 🔵 **Completeness**: Draft filename unspecified — Resolved
- 🔵 **Completeness + Scope + Dependency**: `aliases` open question — Resolved
- 🔵 **Clarity**: All-digit rationale — Resolved

### New Issues Introduced

- 🟡 **Dependency + Testability + Completeness**: Tracker-side key change rests on unverified Jira behaviour (old-key resolution) and Linear team-key renames, recorded only as Assumptions with Open Questions "None"; only a Linear move is tested.
- 🟡 **Testability**: No specified outcome when a promotion's create returns `local-save` or `loud-terminal` during sync.
- 🟡 **Clarity**: "Declined push" has no named actor and is presented as a `work create --push` outcome; `work create` without `--push` under `{tracker}` unspecified.
- 🟡 **Dependency**: 0291's draft-deferral requirement consumes 0230's draft form, yet 0230 records 0291 only as "Relates to" (0230 should block that portion of 0291).
- 🔵 **Dependency**: 0227 should be "Blocks: the `{tracker}`-validation portion of 0227", mirroring 0228.
- 🔵 **Scope + Clarity**: 0295 disagrees on `id`-changing paths and rewrites typed links only; ownership of the shared re-key operation needs reconciling in both items.
- 🔵 **Clarity**: Unclear whether `external_id` follows key changes outside `{tracker}`.
- 🔵 **Testability**: Whole-token rule tested only for digit adjacency; "nowhere else in `meta/`" conflicts with preserved adjacent occurrences.
- 🔵 **Testability**: "No local number is allocated" lacks an observable; resolution precedence and case-insensitivity untested; `--create` marker clearing and bad `--adopt` key unasserted.
- 🔵 **Scope**: 0146's child entry and offline-creation Open Question are stale.
- 🔵 Suggestions: name the `list-work-items` draft indicator distinctly from the `pending_push` marker; scope "every local `parent`" to items this story creates or promotes; enumerate item shapes in the `id`-stability AC.

### Assessment

Pass 1's two contradictions, the 0291 coupling, and the testability gaps are resolved. The new major findings stem mostly from decisions taken during iteration — tracker-side re-keying (unverified Jira behaviour, no Jira AC) and the 0291 split (edge direction) — plus two gaps the richer text now exposes (promotion failure during sync, the decline path). The item needs one more focused iteration before planning; the scope finding stands as an accepted trade-off.

## Re-Review (Pass 3) — 2026-09-25T14:06:06+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Dependency + Testability + Completeness**: Unverified tracker behaviour behind key-change following — Resolved (Jira verified; not-found fallback; Jira AC)
- 🟡 **Testability**: Promotion failure during sync unspecified — Resolved (rollback-on-failure criterion still missing; see new issues)
- 🟡 **Clarity**: "Declined push" had no actor — Resolved
- 🟡 **Dependency**: 0291 edge direction — Resolved (partial blocks; frontmatter keeps `relates_to` per 0228 precedent, which completeness and clarity now flag as inconsistent)
- 🟡 **Scope**: Story size — Still present (accepted by author decision; now ~43 ACs)
- 🔵 **Dependency**: 0227 edge — Resolved
- 🔵 **Scope + Clarity**: 0295 disagreement / "re-key" overloaded — Resolved (ID retirement)
- 🔵 **Clarity**: Marker states / loud-terminal write failure — Resolved (marker naming on the create path now unclear)
- 🔵 **Clarity**: `external_id` scope outside `{tracker}` — Resolved (every pattern; 0295 refreshes)
- 🔵 **Testability**: Whole-token coverage — Resolved (case sensitivity unstated)
- 🔵 **Testability**: Batch outcomes, redraw on `id`, next-number, resolve ambiguity/case, `--adopt`/`--create` — Resolved
- 🔵 **Scope**: 0146 stale — Resolved
- 🔵 Suggestions (draft indicator, enumerated shapes) — Resolved

### New Issues Introduced

- 🟡 **Scope**: Identity changes that apply without `{tracker}` (every-pattern `external_id` tracking, resolve by `external_id`, ambiguity errors) ride in an opt-in story — consequence of the pass-2 decision; candidate prerequisite story.
- 🟡 **Testability**: No criterion resolves a tracker-owned item by its own key, where `id` and `external_id` match the same item.
- 🟡 **Testability**: No criterion asserts the draft is unchanged immediately after a failed local ID retirement.
- 🔵 **Clarity**: `--no-promote` vs. the non-zero exit when drafts are left unpromoted.
- 🔵 **Clarity + Completeness**: Frontmatter `relates_to` vs. prose "Blocks" for the partial 0227/0291 edges.
- 🔵 **Clarity**: When the draft ID is minted relative to the create, and what a draft-less `created` marker is named by.
- 🔵 **Clarity**: Batch "create fails" overlaps created-but-unwritten items; their children's `parent` unstated.
- 🔵 **Clarity**: Case sensitivity of prose rewriting.
- 🔵 **Clarity**: Criteria relying on neighbouring context (`{tracker}`, push accepted, `id_pattern`).
- 🔵 **Dependency**: `aliases` widening handed to 0295, which does not record it.
- 🔵 **Dependency**: Shared topological ordering and cycle handling with 0291 unowned.
- 🔵 **Dependency**: Tracker old-key resolution recorded only as an assumption.
- 🔵 **Testability**: Retry-succeeds path, create-time marker state per outcome, exit code on stopped drafts, preview-failure fallback, Sync column rendering, bare-numeric retirement, Jira creation-home, case-insensitive alias.
- 🔵 **Scope**: `--adopt`/`--create` and batch extraction separable.
- 🔵 Suggestions: sibling ordering with 0229/0292; other corpus readers; "key" overloaded; list surface and push-preview terms.

### Assessment

All pass-2 majors are resolved. Of the four pass-3 majors, two are scope findings that follow from deliberate decisions (one story; every-pattern `external_id` tracking) and two are narrow testability gaps with direct fixes. Excluding the accepted scope findings, the item sits at the REVISE major-count threshold; addressing the two testability majors and the `--no-promote` exit ambiguity would bring it to COMMENT.

## Re-Review (Pass 4) — 2026-09-25T17:11:05+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability**: Resolving a `{tracker}` item by its own key — Resolved
- 🟡 **Testability**: Draft state after failed ID retirement — Resolved (all-or-nothing retirement)
- 🟡 **Scope**: Epic-sized story; identity foundations bundled — Still present (accepted by decision, recorded in Drafting Notes)
- 🔵 **Clarity**: `--no-promote` exit, draft-ID minting, batch "create fails", rewrite case, partial-block convention, self-contained criteria — Resolved
- 🔵 **Dependency**: `aliases` widening, ordering ownership, old-key evidence, 0229 — Resolved (ordering ownership reopened; see new issues)
- 🔵 **Testability**: Retry-succeeds, marker per outcome, preview fall-back, Sync column, bare-numeric retirement, Jira creation-home, case-insensitive alias — Resolved
- 🔵 **Completeness**: 0146 story list — Partially resolved (scope bullet refreshed; Stories list still reads "stub-mint" and omits 0291/0295)

### New Issues Introduced

- 🟡 **Clarity**: Unqualified `loud-terminal` in the draft-landing criterion conflates the tracker-error and post-creation write-failure sub-cases, contradicting the `PP-900` criteria.
- 🟡 **Testability**: No rule for an ID retirement whose new ID already exists as an `id` or alias (e.g. `--adopt PP-900` after sync pulled `PP-900`).
- 🟡 **Dependency**: Ordering ownership assumes 0230 ships first, but 0291 is higher priority; no rule for out-of-order delivery. Same shape for 0227's `{tracker}` validation.
- 🔵 **Clarity**: Context's 0146-boundary claim omits the 0295 re-key exception; sync's adoption of orphaned `created` markers unspecified; legacy item's old `external_id` fate after a move; create-time marker rule sits under Promotion; retry of failed ID retirement; unnamed push command for legacy items; "sync baseline" undefined.
- 🔵 **Testability**: Non-`{tracker}` creation unchanged untested; literal text around `{tracker}` (`ACC-{tracker}`); promotion `work_item_id` rewrite, loud-terminal promotion output, write-failed parent's children, cycle in a declined batch.
- 🔵 **Dependency**: Jira and Linear APIs not listed as external systems; visualiser drafts and typed-link push deferrals unlinked to work items.
- 🔵 **Completeness**: No documentation requirement for the new config value, commands, and flags.
- 🔵 **Scope**: Batch extraction and `--adopt`/`--create` separable; shared ordering risks growing to 0291's needs.

### Assessment

Every pass-3 finding is resolved, but each pass surfaces new edges because each round of fixes adds surface — the item has grown from 16 to 51 criteria. Three new majors remain, all narrow: disambiguating `loud-terminal`, a collision rule for ID retirement, and ownership of shared components when siblings ship first. The recurring scope majors are the structural signal behind this churn.

## Verdict Change — 2026-09-26T00:47:47+00:00

**Verdict:** APPROVE

Changed from REVISE by the author after the three pass-4 majors were addressed
(named `loud-terminal` cases, the ID-retirement collision refusal, and
first-to-ship ownership of shared ordering and `{tracker}` validation). Those
fixes were not re-reviewed. The recurring scope findings are accepted, and the
remaining pass-4 minors are left open.
