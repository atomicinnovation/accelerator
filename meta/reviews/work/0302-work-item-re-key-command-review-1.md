---
type: "work-item-review"
id: "0302-work-item-re-key-command-review-1"
title: "Work Item Review: Work Item Re-Key Command"
date: "2026-10-15T18:30:06+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0302"
work_item_id: "0302"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-10-15T22:15:06+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Work Item Re-Key Command

**Verdict:** REVISE

The work item is dense, precise and coherent. Nearly every requirement has a
concrete Given/When/Then criterion, and Out of scope delegates cleanly to 0343
and 0345. It needs revising on five major points. The `/rekey-work-items` skill
has no acceptance criterion. One Assumption contradicts an acceptance
criterion. A pronoun in the retirement requirement is ambiguous. Two of the
safety mechanisms, plan execution and the dirty-tree exemption on resume, have
criteria that a weaker implementation would also pass.

### Cross-Cutting Themes

- **The skill is unverified** (flagged by: completeness, testability, scope).
  The `/rekey-work-items` requirement has no acceptance criterion and is not in
  the Summary.
- **`external_id` under numeric patterns** (flagged by: clarity, testability).
  The last Assumption says "targets never derive from `external_id`", but the
  `{key}` unreachable-tracker AC says "deriving targets from the stored
  `external_id`".
- **The dirty-tree exemption on resume** (flagged by: clarity, testability).
  It is unclear whether the exemption is blanket or limited to the plan's own
  paths, and only the permissive case is tested.
- **Frontmatter dependency edges are missing** (flagged by: completeness,
  dependency). The item has no `blocks: [0343]` or `blocked_by: [0230]`.
  "Blocked by: none" contradicts 0230's `blocks: [0302]`.
- **Bundled side work** (flagged by: scope, dependency). The
  `ACCELERATOR_FORCE` rename reaches into `migrate`, and the deprecated alias
  has no follow-up item that owns its removal.

### Findings

#### Major

- 🟡 **Completeness / Testability**: `/rekey-work-items` skill has no
  acceptance criterion
  **Location**: Acceptance Criteria
  The skill is a shipped surface. A version that skips the preview or ignores
  a declined confirmation would pass every criterion.
- 🟡 **Clarity / Testability**: Assumption contradicts the `{key}`
  unreachable-tracker AC
  **Location**: Assumptions / Acceptance Criteria
  Under numeric patterns, `external_id` only classifies an item as a tracker
  key; it never supplies the target. The AC also names no resulting `id`.
- 🟡 **Clarity**: "rewrite it within `meta/`" has two possible referents
  **Location**: Requirements
  The nearest noun is "the new `id`", but the intended referent is the old
  ID's references. This is the most consequential side effect in the item.
- 🟡 **Testability**: "Executes exactly the previewed mapping" cannot tell plan
  execution from recomputation
  **Location**: Acceptance Criteria
  On an unchanged corpus, recomputing gives the same mapping. The precondition
  needs to make recomputation diverge, for example a key that moves between
  preview and apply.
- 🟡 **Testability / Clarity**: The dirty-tree exemption on resume is ambiguous
  and tested only permissively
  **Location**: Requirements / Acceptance Criteria
  An implementation that skips the dirty check on every resume passes. There is
  no AC where unrelated dirty changes still cause a refusal.

#### Minor

- 🔵 **Completeness / Dependency**: Frontmatter omits `blocks` and `blocked_by`
  **Location**: Frontmatter / Dependencies
  0230 and 0343 declare edges that 0302 does not mirror, and "Blocked by: none"
  contradicts 0230.
- 🔵 **Dependency**: Reliance on vendors resolving old keys is not recorded
  **Location**: Assumptions
  The refresh depends on Linear's `previousIdentifiers` and Jira's handling of
  moved keys. 0230 states this; 0302 does not.
- 🔵 **Dependency**: Ordering against sync's identity records and
  `pending_push` markers is not stated
  **Location**: Requirements
  A half-applied sync `RetirementRecord`, or a `created` marker, could compete
  with a rekey target.
- 🔵 **Dependency**: Removal of the deprecated `ACCELERATOR_MIGRATE_FORCE` has
  no owner
  **Location**: Requirements / Out of scope
  It is "to be removed in a future release", with no item or release named.
- 🔵 **Scope**: The `ACCELERATOR_FORCE` rename bundles a change to `migrate`
  **Location**: Requirements
  It could ship independently. Its inclusion is deliberate per the Drafting
  Notes.
- 🔵 **Scope**: The `external_id` refresh and the `fetch_all` contract are a
  separable increment
  **Location**: Requirements
  Five ACs reach into the tracker abstraction. If they stay, record why stale
  keys are unacceptable.
- 🔵 **Clarity**: Tracker-adapter jargon is used before it is explained
  **Location**: Requirements
  `found`, `absent`, `indeterminate`, `locate`, "retrieval strategy" and exit 70
  are glossed only in Technical Notes, or not at all.
- 🔵 **Clarity**: Several names for the run plan and its staleness check
  **Location**: Requirements
  The item says proposed, recorded, previewed and run-plan record, and
  identities versus identity fingerprint.
- 🔵 **Clarity**: "Promoting prose ahead of 0297 is accepted" conflicts with
  Out of scope
  **Location**: Dependencies
  It does not say that the promotion belongs to 0343.
- 🔵 **Clarity**: "No pattern change is a no-op" conflicts with the refresh
  and recovery behaviour
  **Location**: Requirements
  A moved tracker key or an outstanding plan changes files even when the
  pattern has not changed.
- 🔵 **Clarity**: The "earlier numeric pattern" classification is given only by
  example
  **Location**: Requirements
  `ACC-0042` and `PP-760` share a shape, so the item needs a stated rule.
- 🔵 **Testability**: Failure exit codes are specified unevenly
  **Location**: Acceptance Criteria
  The unreachable-tracker AC says "non-zero", but the bulk-`locate` AC says 70,
  for the same condition.
- 🔵 **Testability**: The collision ACs use abstract fixtures
  **Location**: Acceptance Criteria
  They need concrete shapes, such as `0042` and `ACC-0042` under
  `{number:04d}`, and the `id` and alias cases should be split.
- 🔵 **Testability**: The draft-conversion AC asserts only the location
  **Location**: Acceptance Criteria
  It does not check the `id`, H1, `aliases`, link rewrites or the old file's
  removal.
- 🔵 **Testability**: The tie-break and `draft-` prose rewriting lack examples
  **Location**: Acceptance Criteria
  There is no same-date example, no `draft-` prose AC and no whole-token
  boundary AC.

#### Suggestions

- 🔵 **Clarity**: The example ID `0230` clashes with the real work item 0230
  **Location**: Acceptance Criteria
- 🔵 **Dependency**: The new `fetch_all` obligation binds future adapters
  (0181)
  **Location**: Requirements
- 🔵 **Scope**: Note candidate split points for this large story (27 ACs)
  **Location**: Drafting Notes
- 🔵 **Testability**: The force rename has no negative or regression checks
  **Location**: Acceptance Criteria
  It does not check that `MIGRATE_FORCE` is ignored by `rekey`, or that
  `migrate` keeps treating a failed probe as clean.

### Strengths

- ✅ Concrete fixtures (IDs, dates, paths) let each AC become a test.
- ✅ A worked example pins the allocation order (`draft-k3x9` becomes
  `ACC-0042`, `PP-760` becomes `ACC-0043`).
- ✅ The alias round trip directly checks the self-alias stripping.
- ✅ The prose-rewrite rule is tested from both sides and stated consistently,
  with its reason given.
- ✅ Each failure path asserts "writes nothing".
- ✅ Out of scope is tight. Alias normalisation and bare-number review are
  already split to 0343 and 0345.
- ✅ The Technical Notes name existing primitives and the precise gaps, so the
  item is ready to plan.

### Recommended Changes

1. **Add skill ACs** (addresses: the skill has no acceptance criterion)
   Cover three cases: preview shown and confirmation asked; decline leaves
   `meta/` unchanged and the proposed plan unexecuted; confirm applies the
   previewed plan. Mention the skill in the Summary.
2. **Reconcile `external_id` under numeric patterns** (addresses: Assumption
   contradicts the AC)
   State that `external_id` only classifies tracker keys. Rewrite the `{key}`
   unreachable-tracker AC with a concrete outcome, such as `PP-760` becoming
   `ACC-0042` with no tracker request.
3. **Name the subject of the reference rewrite** (addresses: "rewrite it")
   Say that references to the old ID within `meta/` are rewritten to the new
   ID.
4. **Make the plan-execution AC discriminating** (addresses: executes the
   previewed mapping)
   Use a key that moves between preview and apply, and assert that the
   previewed target wins.
5. **Specify the resume exemption and test its refusal side** (addresses: the
   dirty-tree exemption)
   Limit the exemption to the plan's paths. Add an AC where an unrelated dirty
   edit still causes a refusal.
6. **Fix the frontmatter edges and the Dependencies prose** (addresses:
   frontmatter omissions)
   Add `blocks: ["work-item:0343"]` and `blocked_by: ["work-item:0230"]`, and
   change "Blocked by" to name 0230 as done.
7. **Record the remaining assumptions and ordering** (addresses: vendor
   reliance, sync ordering, deprecation owner)
   Add a vendor-redirect Assumption, a rule for interplay with sync
   `RetirementRecord` and `pending_push`, and an owner for removing
   `ACCELERATOR_MIGRATE_FORCE`.
8. **Tighten the terms and the remaining ACs** (addresses: the minor clarity
   and testability findings)
   Use one term for the run plan and one for the fingerprint. Gloss the tracker
   terms. Give a precise no-op condition and a numeric-pattern classification
   rule. Use exit 70 for unreachable. Give concrete collision fixtures, a full
   draft-conversion AC, and tie-break and `draft-` prose examples. Use a
   neutral example ID instead of `0230`.

## Per-Lens Results

### Clarity

**Summary**: Precise and dense, with concrete examples. Clarity suffers where
an Assumption contradicts an AC, where a pronoun has two possible referents,
where tracker jargon goes unexplained, and where the run plan has several
names.

**Strengths**: A single framing for how IDs change; worked examples; Out of
scope points to 0343 and 0345; the prose rule is consistent and its reason is
given.

**Findings**:
- major/high — Assumptions: targets-never-from-`external_id` contradicts the
  `{key}` unreachable AC.
- major/high — Requirements: "rewrite it within `meta/`" has two possible
  referents.
- minor/high — Requirements: tracker-adapter jargon is unexplained; exit codes
  are worded inconsistently.
- minor/medium — Requirements: several names for the run plan and fingerprint.
- minor/medium — Dependencies: prose promotion "accepted" conflicts with Out of
  scope.
- minor/medium — Requirements: the no-op condition conflicts with refresh and
  recovery.
- minor/medium — Requirements: the scope of the dirty-tree exemption is
  ambiguous.
- minor/low — Requirements: the "earlier numeric pattern" rule is given only by
  example.
- suggestion/medium — Acceptance Criteria: the example ID `0230` clashes with
  work item 0230.

### Completeness

**Summary**: Very complete. All sections are present and well filled, and the
ACs cover nearly every requirement. The gaps are the skill's missing AC and the
frontmatter edges.

**Strengths**: A user-story Summary; Context explains why; the ACs cover
nearly everything; the supporting sections are substantive; the Technical
Notes name the gaps.

**Findings**:
- major/high — Acceptance Criteria: the `/rekey-work-items` skill has no AC.
- minor/medium — Frontmatter: `blocks` and `blocked_by` are missing;
  `last_updated` is earlier than 0343's creation.

### Dependency

**Summary**: The main couplings are well mapped. The gaps are typed
frontmatter edges, the vendor redirect reliance, the ordering against sync
records, and ownership of the deprecated variable's removal.

**Strengths**: Names 0343 as downstream; 0230 visibly shipped; the 0297
relation is justified; tracker availability is handled explicitly.

**Findings**:
- minor/high — Frontmatter: `blocks` and `blocked_by` are missing; "Blocked by:
  none" contradicts 0230.
- minor/high — Assumptions: the reliance on vendor old-key resolution is not
  recorded.
- minor/medium — Requirements: ordering against sync `RetirementRecord` and
  `pending_push`.
- minor/medium — Requirements / Out of scope: the deprecated alias removal has
  no owner.
- suggestion/low — Requirements: the `fetch_all` obligation binds future
  adapters (0181).

### Scope

**Summary**: One coherent capability with a tight out-of-scope list, but a
large one (27 ACs). It bundles the `migrate` force rename and the tracker
refresh, either of which could ship separately.

**Strengths**: An explicit out-of-scope list; already decomposed once (0343);
the safety mechanisms are justified as a unit; the Summary, Context and
Requirements are consistent.

**Findings**:
- minor/medium — Requirements: the `ACCELERATOR_FORCE` rename bundles a change
  to `migrate`.
- minor/medium — Requirements: the `external_id` refresh and the `fetch_all`
  contract are separable.
- suggestion/medium — Acceptance Criteria: the skill is in Requirements but not
  in the ACs.
- suggestion/low — Frontmatter: kind: note split seams for a large story.

### Testability

**Summary**: Unusually testable. The gaps are the skill having no AC, two
safety mechanisms whose ACs a weaker implementation would pass, and a few loose
outcomes or preconditions.

**Strengths**: Consistent Given/When/Then with concrete fixtures; a worked
ordering example; the alias round trip; prose tested from both sides; a
contract test for adapters; "writes nothing" on every failure path.

**Findings**:
- major/high — Requirements: the skill has no AC.
- major/medium — Acceptance Criteria: "executes exactly the previewed mapping"
  does not discriminate between plan execution and recomputation.
- major/medium — Acceptance Criteria: the dirty-tree exemption on resume is
  tested only permissively.
- minor/medium — Acceptance Criteria: the `{key}` unreachable-tracker AC has
  no concrete outcome and contradicts the Assumption.
- minor/high — Acceptance Criteria: exit codes are uneven.
- minor/medium — Acceptance Criteria: the collision fixtures are abstract.
- minor/medium — Acceptance Criteria: the draft-conversion AC asserts only the
  location.
- minor/medium — Acceptance Criteria: the tie-break and `draft-` prose lack
  examples.
- suggestion/low — Acceptance Criteria: the force rename has no negative or
  regression checks.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-10-15T20:56:52+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Completeness / Testability**: Skill has no acceptance criterion — Resolved
- 🟡 **Clarity / Testability**: Assumption contradicts the `{key}` unreachable AC — Resolved
- 🟡 **Clarity**: "rewrite it within `meta/`" referent — Resolved
- 🟡 **Testability**: Plan-execution AC indistinguishable from recomputation — Resolved
- 🟡 **Testability / Clarity**: Dirty-tree exemption on resume — Resolved
- 🔵 **Completeness / Dependency**: Frontmatter `blocks`/`blocked_by` — Resolved
  (0228 still absent from `relates_to`)
- 🔵 **Dependency**: Vendor old-key resolution — Resolved in Assumptions;
  dependency now also wants an External entry
- 🔵 **Dependency**: Ordering against sync state — Partially resolved (no AC
  for the retirement-record branch; reverse direction unstated)
- 🔵 **Dependency**: Deprecated alias owner — Resolved (0346)
- 🔵 **Scope**: `ACCELERATOR_FORCE` rename bundled — Still present (accepted
  per Drafting Notes)
- 🔵 **Scope**: Tracker refresh separable — Still present, raised to major as
  numeric vs `{tracker}` re-keying
- 🔵 **Clarity**: Tracker jargon — Partially resolved (sync terms and
  "synced" still undefined)
- 🔵 **Clarity**: Run-plan naming — Partially resolved (states and their
  transitions unnamed)
- 🔵 **Clarity**: 0297 prose-promotion line — Resolved
- 🔵 **Clarity**: No-op condition — Resolved, but exposed the moved-key
  contradiction below
- 🔵 **Clarity**: Numeric-pattern classification — Resolved
- 🔵 **Testability**: Exit codes uneven — Partially resolved ("refuses" still
  unspecified)
- 🔵 **Testability**: Collision fixtures — Resolved
- 🔵 **Testability**: Draft-conversion AC — Resolved
- 🔵 **Testability**: Tie-break and `draft-` prose — Resolved (term varies:
  "lexically" vs "byte-wise")
- 🔵 **Testability**: Force rename negatives — Resolved
- 🔵 **Clarity**: Example ID `0230` — Resolved
- 🔵 **Dependency**: `fetch_all` obligation binds future adapters — Still
  present

### New Issues Introduced

- 🟡 **Clarity / Testability**: Moved key on an item whose `id` equals
  `external_id` — the refresh and no-op requirements imply re-keying to the
  new key; the Assumption and AC say unchanged.
- 🟡 **Clarity**: Run-plan lifecycle unspecified — whether resume runs the
  fingerprint check, whether `--preview` finishes an interrupted plan, and what
  happens to a declined proposed plan.
- 🟡 **Clarity**: "Key" overloaded — the prose rule's "`<KEY>-<number>` key"
  could mean shape or tracker identity, deciding whether `ACC-0042` prose is
  rewritten.
- 🟡 **Testability**: No AC for refusal on an unfinished sync retirement
  record.
- 🟡 **Scope**: Numeric and `{tracker}` re-keying are separable increments
  (~⅓ of 36 ACs are `{tracker}`-only).
- 🔵 **Clarity**: Collision rule omits the own-`id`/own-alias exclusion the
  round-trip AC needs.
- 🔵 **Clarity**: Probe-failure-as-dirty reads as applying to `migrate` too.
- 🔵 **Clarity**: "The apply writes the refreshed `external_id`" — unclear
  whether the run plan carries preview-time refreshed keys.
- 🔵 **Testability**: Preview output — unclear whether unchanged items print.
- 🔵 **Testability**: "Writes nothing" — unclear whether it covers the run
  plan; a refused run might leave an executable plan.
- 🔵 **Testability**: `--preview` on a dirty tree or unreachable tracker has
  no AC.
- 🔵 **Testability**: Interrupted-run AC under-specifies setup and end state.
- 🔵 **Dependency**: Run-plan path relies on an ignore rule in existing
  repositories.
- 🔵 **Dependency**: `work sync` duty toward an unfinished re-key plan
  unstated.
- 🔵 **Completeness**: References omits 0343/0345/0346 and labels 0230
  Related.
- 🔵 **Scope**: Re-key partly takes over sync's key-following.
- 🔵 Suggestions: skill-AC verification method, deprecation notice,
  pre-existing-alias exclusion wording, 0146 story list.

### Assessment

All five pass-1 majors are resolved. Pass 2 found five new majors, mostly
deeper in the design than pass 1 reached: the moved-key case for items already
on the pattern, the run-plan lifecycle, the overloaded "key", a missing
sync-retirement AC, and whether to split `{tracker}` re-keying out. The first
three are clarity decisions the user needs to make; the fourth is mechanical;
the fifth is a scope call. The item needs one more iteration before planning.

## Re-Review (Pass 3) — 2026-10-15T21:52:24+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity / Testability**: Moved key on an item whose `id` equals
  `external_id` — Resolved
- 🟡 **Clarity**: Run-plan lifecycle — Resolved (edges of `--discard`
  remain, see below)
- 🟡 **Clarity**: "Key" overloaded — Resolved
- 🟡 **Testability**: Unfinished sync retirement record AC — Resolved
- 🟡 **Scope**: Numeric vs `{tracker}` split — Resolved (kept together per
  user direction, recorded in Drafting Notes)
- 🔵 Collision self-exclusion, probe scope, tie-break wording, sync terms,
  aliased-ID wording — Resolved
- 🔵 Preview output, preview on dirty tree / unreachable tracker, interrupted
  run, refusal wording, deprecation notice, skill confirm — Resolved
- 🔵 Run-plan ignore rule, `work sync` reverse coupling, References, 0228,
  External entry, 0146 stories — Resolved
- 🔵 **Clarity**: Tracker-layer jargon — Partially resolved (exit 70,
  retrieval strategy, non-bulk path)
- 🔵 **Dependency**: 0181 — Partially resolved (in Dependencies, not in
  `relates_to` or References)
- 🔵 **Scope**: `migrate` rename and tracker refresh bundled — Still present
  (accepted)

### New Issues Introduced

- 🟡 **Clarity**: "Next allocated number" / "highest number" do not say
  which IDs count — legacy numbers being kept may be ignored, leading to
  collisions.
- 🟡 **Testability**: `work rekey --discard` has no direct AC; behaviour
  on an in-progress or absent plan is undefined.
- 🔵 **Clarity**: The tracker-refresh requirement does not name which
  invocations refresh.
- 🔵 **Clarity**: "No pattern change" has no reference point; state the
  no-op as observable corpus state.
- 🔵 **Clarity**: "Previewed plan" vs "proposed run plan"; whether exit 70
  is a refusal; "one item" should be "exactly one".
- 🔵 **Testability**: No AC for preview replacing a proposed plan, or for a
  refused run preserving a plan.
- 🔵 **Testability**: Plan-before-first-write checked only via a staged
  fixture.
- 🔵 **Testability**: Preview "exactly" has no line order.
- 🔵 **Testability**: Non-bulk retrieval strategy uncovered.
- 🔵 **Testability**: Dirty-tree scope boundary has no AC.
- 🔵 **Testability**: Fingerprint field coverage and body-only edits.
- 🔵 **Testability**: Undated items and prefixed IDs not equal to their
  `external_id`.
- 🔵 **Testability**: Skill ACs phrased as conversation, not command use.
- 🔵 **Dependency**: 0296 baseline-shape ordering; tracker rate limits;
  0344 one-way link.
- 🔵 **Scope**: Summary understates the reach outside `work rekey`;
  delivery-order note.
- 🔵 **Completeness**: 0181 missing from `relates_to` and References.

### Assessment

All pass-2 majors are resolved. The two new majors are narrow, and each needs
one decision plus a few lines. The minors mostly ask for more boundary ACs.
Returns are diminishing: the item has 46 ACs, and each pass adds about ten
more. Fix the two majors and the high-value minors, then stop iterating at
the work-item level and settle the remaining test boundaries in the plan.

## Verdict Change — 2026-10-15T22:15:06+00:00

**Verdict:** APPROVE

Approved by Toby Clemson after the post-pass-3 edits addressed both pass-3
majors (fresh-number floor, `--discard` cases) and the key minors (refresh
timing, no-op condition, run-plan wording, lifecycle ACs). Those edits were
not re-reviewed. The remaining pass-3 minors are deferred to planning.
