---
type: "work-item-review"
id: "0197-accelerator-collaboration-pr-helper-cli-review-1"
title: "Work Item Review: accelerator-collaboration: PR Helper CLI"
date: "2026-08-06T00:51:57+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0197"
work_item_id: "0197"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-08-06T01:13:07+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: accelerator-collaboration: PR Helper CLI

**Verdict:** COMMENT

Work item is acceptable but could be improved — see the major finding below.
0197 is a well-bounded, well-documented split of a single sub-binary
migration: Dependencies are unusually thorough, Acceptance Criteria are
mostly concrete and verifiable, and the item is appropriately sized as a
single-team story. The dominant issue, raised independently by three lenses,
is an unresolved contradiction over whether work-item:0150 (the
github→collaboration directory rename) is actually complete — the item's own
Summary and Dependencies sections disagree with its Context section on this
point. A handful of minor precision gaps (glob-shaped AC scoping, Requirements
missing two AC-gated obligations, an underspecified verification method)
round out the findings.

### Cross-Cutting Themes

- **work-item:0150's completion status is inconsistently characterised**
  (flagged by: clarity, dependency, scope) — Dependencies lists 0150 among
  "resolved" prior blockers, but 0150's own frontmatter is `status: draft`
  with an unchecked AC. Context separately calls the github→collaboration
  rename "an in-progress initiative," and Summary claims this item is
  "completing" that rename — three different framings of the same
  relationship, none reconciled. Since this item's Requirements, AC, and
  Technical Notes all still reference the pre-rename `skills/github/**`
  paths, the ordering and scope relationship to 0150 needs to be stated once,
  consistently, rather than three times, differently.
- **Glob-shaped scope language where a named-file list would be
  unambiguous** (flagged by: scope, testability) — both Requirements'
  call-site bullet ("every skill under `skills/github/**`") and AC3 ("the
  migrated `skills/github/**` scripts are removed") use a directory glob where
  the Technical Notes' two named source scripts would give a verifier a
  self-contained boundary.

### Findings

#### Major

- 🟡 **Clarity / Dependency**: "Resolved" status of work-item:0150 contradicts
  its own draft status and the Context section's "in-progress" framing
  **Location**: Dependencies
  Dependencies groups work-item:0150 with three genuinely completed items
  under "Prior blockers are resolved," but 0150's frontmatter shows
  `status: draft` with an unchecked AC, and Context calls the same rename
  "in-progress." A reader cannot tell whether 0150 must land first, is
  merely a naming precedent, or represents an uncaptured path-collision risk
  with this item's own `skills/github/**` references.

#### Minor

- 🔵 **Scope**: Summary claims the domain rename is completing here, but no
  Requirement renames the skill directory
  **Location**: Summary
  Summary says this item is "completing the github→collaboration domain
  rename for this cluster," but the skill directory stays named `github`
  after this item ships (per AC2/Technical Notes) — the actual rename is
  work-item:0150's, still in draft.
- 🔵 **Scope**: Requirements scope call-site rewiring more broadly than
  Acceptance Criteria
  **Location**: Requirements
  Requirements' second bullet says "every skill under `skills/github/**`"
  while the corresponding AC narrows this to skills invoking the two named
  helper scripts specifically — the same set described at two different
  widths.
- 🔵 **Completeness**: Requirements omit the script-removal and
  registration-checklist obligations that Acceptance Criteria depend on
  **Location**: Requirements
  AC3 (script removal + floor decrement) and AC4 (registration checklist)
  have no corresponding Requirements bullet, so scoping the work from
  Requirements alone would miss two AC-gated obligations.
- 🔵 **Completeness**: Context explains the split and naming decision but not
  the substantive motivation for the migration itself
  **Location**: Context
  Context is entirely procedural (why the item was split, why the naming is
  settled) — it never restates why migrating these scripts to Rust is worth
  doing, leaving that entirely to the parent epic.
- 🔵 **Testability**: AC1 defers the exact `gh` invocation spec and leaves
  the required verification method ambiguous
  **Location**: Acceptance Criteria
  AC1's "repointed suites and/or characterization tests" doesn't say which
  method (or both) is authoritative, and the specific `gh` sub-commands/flags
  are deferred to implementation rather than enumerated now from the two
  named source scripts.
- 🔵 **Testability**: Suite-floor decrement in AC3 has no self-contained
  target value
  **Location**: Acceptance Criteria
  AC3 requires floors "decremented in lockstep (see work-item:0174)" without
  naming which suite(s) or by how much within this item, so verification
  depends entirely on 0174 being finalised and aligned at implementation time.
- 🔵 **Clarity**: "Repointed suites" is unglossed jargon
  **Location**: Acceptance Criteria
  The migration-specific term "repointed" is used without definition or a
  link to where the convention is established (e.g. work-item:0167).
- 🔵 **Dependency**: Possible uncaptured CI credential coupling for `gh`-CLI
  verification
  **Location**: Acceptance Criteria
  It's unclear whether AC1's characterization tests require live,
  authenticated `gh` CLI calls (implying a CI credential-provisioning
  coupling not named anywhere) or a mocked interface.
- 🔵 **Testability**: AC3's file scope for removal is stated as a glob
  rather than the two named files
  **Location**: Acceptance Criteria
  AC3's `skills/github/**` glob requires cross-referencing Technical Notes
  to know it's scoped to exactly two files rather than the whole tree.

#### Suggestions

- 🔵 **Completeness**: No Assumptions section despite reliance on
  characterization-test parity
  **Location**: Assumptions
  Given AC1 defers behavioural parity to characterization tests, an explicit
  assumption that the two named scripts represent the complete current
  PR-helper behaviour would surface an implicit reliance.
- 🔵 **Completeness**: Story kind — the beneficiary of the migration is not
  explicitly named
  **Location**: Acceptance Criteria
  As a `story`-kind item, the beneficiary (skills authors, or cross-domain
  naming consistency) is implicit rather than stated directly in
  Summary/Context.

### Strengths

- ✅ Referents to the sub-binary name, the two source scripts, and the
  `skills/github/**` paths are used consistently across Summary, Context,
  Requirements, and Technical Notes.
- ✅ Dependencies is unusually thorough: resolved prior blockers are cited
  with concrete evidence (done / merged via PR #42), the external `gh` CLI
  coupling is named with its precondition, and the downstream Blocks entry
  (0174) is tied to a specific AC bullet.
- ✅ Acceptance Criteria are mostly concrete and verifiable — AC2 bounds "all
  skills" to a named, concrete set, and AC4 points to a fixed, enumerable
  external checklist rather than a vague quality bar.
- ✅ The item is a clean, single-purpose split from an over-bundled
  predecessor (0173), with the split rationale documented explicitly rather
  than left implicit.
- ✅ Frontmatter is fully populated and internally consistent (kind, status,
  priority, parent, derived_from all present and sensible).

### Recommended Changes

1. **Reconcile work-item:0150's status across Summary, Context, and
   Dependencies** (addresses: the cross-cutting theme; both major findings).
   State once, consistently, whether 0150 is a completed naming precedent
   this item builds on, or a still-open rename this item's paths will need
   to track — and whether any ordering/coordination is required.
2. **Replace glob-shaped scope language with the two named files** in the
   Requirements call-site bullet and AC3 (addresses: the second cross-cutting
   theme, the two Scope/Testability findings on Requirements and AC3).
3. **Add two Requirements bullets** covering script removal/floor decrement
   and the registration checklist, so Requirements and Acceptance Criteria
   correspond 1:1 (addresses: Completeness finding on Requirements).
4. **Resolve AC1's "and/or"** into a single required verification method (or
   state both are required), and consider enumerating the specific `gh`
   sub-commands/flags now rather than deferring to implementation (addresses:
   Testability finding on AC1).
5. **Gloss "repointed suites"** on first use, and add one sentence to Context
   restating the substantive motivation for the migration (addresses:
   Clarity and Completeness findings on Context/AC).

## Per-Lens Results

### Clarity

**Summary**: The work item is largely clear: referents to the sub-binary,
the two PR-helper scripts, and the skill paths remain stable across
Summary, Context, Requirements, and Acceptance Criteria, and the Acceptance
Criteria describe observable outcomes rather than vague properties. The main
clarity problem is a genuine internal contradiction in how work-item:0150's
status is characterised — Context calls the github→collaboration rename an
"in-progress initiative" while Dependencies lists 0150 among "prior blockers
[that] are resolved," leaving the reader unable to tell whether 0150 must
complete before this item proceeds. A couple of unglossed process terms
("repointed suites") are minor secondary issues.

**Strengths**:
- Referents to the sub-binary name, the two source scripts (`pr-base-repo`,
  `pr-update-body`), and the `skills/github/**` paths are used consistently
  and identically across Summary, Context, Requirements, and Technical
  Notes — no drifting terminology.
- The Context section explicitly resolves a prior ambiguity (the "open" vs
  "fixed" naming wording flagged in 0173's review), rather than leaving it
  implicit, which is a good clarity practice.
- Acceptance Criteria are phrased as observable system states (the binary
  reproduces behaviours, skills call the new subcommand, scripts are
  removed, the checklist passes) rather than as vague desired properties.

**Findings**:
- 🟡 **Major** (confidence: high) — Location: Dependencies. "Resolved"
  status of work-item:0150 contradicts Context's "in-progress" framing.
  Dependencies states "Prior blockers are resolved" and then lists
  `work-item:0150 (github→collaboration rename precedent)` alongside three
  items explicitly marked done/merged. But Context describes the rename as
  "an in-progress initiative across the codebase," and the parenthetical for
  0150 conspicuously omits the word "done" the other three entries carry.
  Suggestion: separate 0150 from the resolved-blocker list, or state
  explicitly what about 0150 is resolved versus in-progress.
- 🔵 **Minor** (confidence: medium) — Location: Acceptance Criteria.
  "Repointed suites" is unglossed jargon. AC1's "repointed suites" is used
  without definition. Suggestion: gloss on first use or link to the
  migration-pattern document (e.g. 0167) where the term is established.

### Completeness

**Summary**: 0197 is a well-formed story with a clear, unambiguous Summary,
a populated Dependencies section, four specific and verifiable Acceptance
Criteria, and intact frontmatter. The main completeness gaps are in Context,
which is almost entirely procedural rather than restating the substantive
motivation for the migration itself, and a Requirements section that is
thinner than the Acceptance Criteria it should ground — two AC items (script
removal/floor decrement and the registration checklist) have no
corresponding Requirements bullet. No Assumptions section is present, which
is a minor gap given the behavioural-parity risk implicit in a bash-to-Rust
migration.

**Strengths**:
- Acceptance Criteria are specific and numerous, each tied to a concrete
  verification mechanism rather than vague outcome statements.
- Dependencies is thorough: it enumerates resolved prior blockers by ID with
  their resolution state, names the one open external coupling (gh CLI),
  and states what this item blocks and its parent.
- Frontmatter is fully populated and internally consistent.

**Findings**:
- 🔵 **Minor** (confidence: medium) — Location: Context. Context explains
  the split and naming decision but not the substantive motivation for the
  migration itself — a reader who lands on this story without first reading
  0136 has no sense of the motivating problem. Suggestion: add one sentence
  restating the substantive driver (e.g. replacing untestable bash under
  the bash 3.2 floor with typed Rust).
- 🔵 **Minor** (confidence: medium) — Location: Requirements. Requirements
  omit the script-removal and registration-checklist obligations that AC3
  and AC4 depend on. Suggestion: add two Requirements bullets so Requirements
  and Acceptance Criteria correspond 1:1.
- 🔵 **Suggestion** (confidence: low) — Location: Assumptions. No
  Assumptions section despite reliance on characterization-test parity.
  Suggestion: add a brief Assumptions section noting the two named source
  scripts are treated as the complete PR-helper behavioural surface.
- 🔵 **Suggestion** (confidence: low) — Location: Acceptance Criteria.
  Story kind: the beneficiary of the migration is not explicitly named.
  Suggestion: add a one-clause addition naming who benefits.

### Dependency

**Summary**: The Dependencies section is unusually well populated for a
story: it names three resolved technical blockers with concrete evidence, an
external `gh` CLI coupling carried forward from the source bash, a
correctly-named downstream Blocks entry (0174) tied to a specific AC bullet,
and the parent epic. The one significant gap is that work-item:0150 (the
actual github→collaboration skill-directory rename) is listed among
"resolved" prior blockers despite its own frontmatter showing `status: draft`
and an unchecked AC — an uncaptured ordering ambiguity between this item's
path-based Requirements/AC and 0150's still-pending rename of the very
directory they reference.

**Strengths**:
- Downstream consumer is explicitly captured: work-item:0174 is named as a
  Blocks entry and tied to the specific AC bullet.
- The external `gh` CLI dependency is named with its install/authentication
  precondition and explicitly flagged as a pre-existing coupling.
- Prior technical blockers (0166, 0167, 0187) are each cited with concrete
  completion evidence rather than a bare assertion.

**Findings**:
- 🟡 **Major** (confidence: high) — Location: Dependencies. work-item:0150
  listed as a resolved prior blocker despite still being in draft/unstarted
  status. Unlike the other three entries, 0150 is annotated only as a naming
  precedent, not resolution evidence, and this item's Requirements/Technical
  Notes still reference the pre-rename `skills/github/**` paths. Suggestion:
  separate 0150 from the resolved list, or add a genuine coordination note
  if directory-path collision is a real risk.
- 🔵 **Minor** (confidence: low) — Location: Acceptance Criteria. Possible
  uncaptured CI credential coupling for gh-CLI verification. It's unclear
  whether AC1's characterization tests require live, authenticated `gh`
  calls (implying a CI credential coupling not named anywhere) or a mocked
  interface. Suggestion: clarify in Technical Notes or Dependencies.

### Scope

**Summary**: 0197 is a well-bounded slice of the 0173 split: it covers
exactly one sub-binary migrating two PR-helper scripts, with Requirements,
Dependencies, and Technical Notes describing a single coherent unit of work.
Two internal wording mismatches create scope-boundary ambiguity worth
tightening before implementation, but neither rises to a bundling or
delivery-risk concern.

**Strengths**:
- The item is a clean, single-purpose split from an over-bundled predecessor
  (0173), with the split rationale explicitly documented.
- Scope is tightly bounded to one sub-binary and two named source scripts,
  consistent across Requirements, Technical Notes, and most of the
  Acceptance Criteria.
- Dependencies correctly scopes the Blocks relationship to 0174 without
  pulling in unrelated downstream work.

**Findings**:
- 🔵 **Minor** (confidence: medium) — Location: Requirements. Requirements
  scope call-site rewiring more broadly than Acceptance Criteria — the
  Requirements bullet says "every skill under `skills/github/**`" while the
  AC scopes the same work to callers of the two named scripts. Suggestion:
  align the Requirements bullet with the AC's precise framing.
- 🔵 **Minor** (confidence: medium) — Location: Summary. Summary claims the
  domain rename is completed, but no requirement renames the skill
  directory — the actual directory-level rename is tracked separately in
  work-item:0150, still in draft. Suggestion: narrow the Summary to describe
  adopting the `collaboration` name for this binary specifically, ahead of
  the full rename tracked in 0150.

### Testability

**Summary**: The acceptance criteria are largely testable: each ties to a
concrete artefact rather than a subjective outcome, and the scope is bounded
to two named helper scripts rather than open-ended language. The main gaps
are that AC1 defers the exact gh invocation enumeration to implementation
time and offers two alternative verification methods without saying which is
required, and AC3's file scope and suite-floor decrement rely on
cross-referencing other documents rather than stating a self-contained check.

**Strengths**:
- AC4 is a model testable criterion: it points to a fixed, enumerable
  external checklist rather than a vague quality bar.
- AC2 bounds "all skills" to a concrete, named set, avoiding the
  unbounded-scope trap.
- AC1 anchors "reproduces the PR-helper behaviours" to an existing,
  inspectable reference rather than a subjective standard.

**Findings**:
- 🔵 **Minor** (confidence: medium) — Location: Acceptance Criteria. AC1
  defers the exact gh invocation spec and offers unresolved alternative
  verification methods ("repointed suites and/or characterization tests")
  without saying which is authoritative. Suggestion: enumerate the specific
  gh sub-commands/flags directly, or remove the "and/or" ambiguity.
- 🔵 **Minor** (confidence: low) — Location: Acceptance Criteria. AC3's file
  scope for removal is stated as a glob rather than the two named files,
  requiring cross-reference to Technical Notes. Suggestion: restate AC3
  with the explicit file paths.
- 🔵 **Minor** (confidence: medium) — Location: Acceptance Criteria.
  Suite-floor decrement in AC3 has no self-contained target value — no
  suite(s) or amount named within this item. Suggestion: name the specific
  suite(s), even if the exact number is TBD pending 0174.

---

## Re-Review (Pass 2) — 2026-08-06T01:05:48+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity/Dependency**: "Resolved" status of work-item:0150 contradicts
  its draft status and Context's "in-progress" framing — Resolved.
  Dependencies now explicitly separates 0150 into a "Not a blocker" entry;
  both lenses cite the distinction as a strength on re-review.
- 🔵 **Scope**: Summary claims the domain rename is completing here — Resolved.
  Summary and Context now describe adopting the `collaboration` name for this
  binary specifically, ahead of 0150's directory rename.
- 🔵 **Scope**: Requirements scope call-site rewiring more broadly than AC —
  Resolved. Both now name the same two source scripts.
- 🔵 **Completeness**: Requirements omit script-removal/registration-checklist
  obligations — Resolved. Requirements now correspond 1:1 with Acceptance
  Criteria (see new suggestion below on the resulting duplication).
- 🔵 **Completeness**: Context omits the substantive migration motivation —
  Resolved. Context now states the bash-3.2-floor/typed-Rust rationale.
- 🔵 **Testability**: AC1's "and/or" verification method is unresolved —
  Resolved as stated (now "supplemented by"), though a narrower framing of
  the same underlying concern resurfaced (see new issues below).
- 🔵 **Testability**: AC3's suite-floor decrement has no named target — Still
  present; left open pending work-item:0174 as before (deliberately not
  addressed — no fabricated suite names).
- 🔵 **Clarity**: "Repointed suites" is unglossed jargon — Resolved for this
  specific term (now defined inline); two adjacent terms remain unglossed
  (see new issues below).
- 🔵 **Dependency**: Uncaptured CI credential coupling for `gh` verification —
  Resolved. Dependencies now states verification should use a
  mockable/injectable interface.
- 🔵 **Testability**: AC3's file scope stated as a glob — Resolved. AC3 now
  names the two files explicitly.
- 🔵 **Completeness**: No Assumptions section — Resolved. Assumptions section
  added.
- 🔵 **Completeness**: Story beneficiary not named — Resolved. Summary now
  names skills authors as the beneficiary.

### New Issues Introduced

- 🟡 **Dependency** (major): Sibling work-item:0196 (accelerator-design)
  carries an explicit Coordination note naming this item and work-item:0195
  as siblings registering sub-binaries via the same checklist around the
  same time, flagging a shared-state (dispatch manifest / CI floor config)
  merge-contention risk — but this item does not reciprocate that note, so
  an implementer picking up 0197 in isolation has no signal of the coupling.
- 🔵 **Dependency** (minor): The external `gh` CLI coupling names the
  install/auth precondition but not the availability/SLA implications of
  the underlying GitHub API (rate-limiting/outage behaviour) inherited by
  the migrated subcommands.
- 🔵 **Clarity/Testability** (minor): "Characterization tests" and "suite
  floors" remain unglossed, and AC1's parenthetical still defers the exact
  `gh` invocation enumeration to implementation time without stating
  whether that enumeration already exists or is produced as a first
  implementation sub-step — the same underlying "spec deferred to
  implementation" concern as before, now framed more precisely, including a
  request for a completion threshold on the characterization-test fallback.
- 🔵 **Completeness/Testability** (suggestion): Requirements 2–4 now closely
  restate their corresponding Acceptance Criteria (a natural consequence of
  closing the 1:1 gap); no Open Questions section; AC2's call-site
  verification method is implicit; call-site rewrite count is unbounded
  (explicitly noted as a sizing signal, not a scope concern).

### Assessment

The major cross-cutting issue from pass 1 (work-item:0150's status) is
fully resolved, along with every other pass-1 finding except the
deliberately-deferred suite-floor naming. One new major finding surfaced
during re-review: this item doesn't reciprocate a sibling coordination note
that work-item:0196 already carries about concurrent sub-binary
registration. It's a quick, low-risk addition (a single Dependencies
bullet) and worth adding before implementation, even though it doesn't by
itself cross the REVISE threshold. With that addition, the item is ready
for planning.

### Verdict Update — 2026-08-06T01:13:07+00:00

**Verdict:** APPROVE

The reciprocal Coordination note (naming siblings work-item:0195 and
work-item:0196) has been added to Dependencies, closing out the sole new
finding from Pass 2. All findings across both passes are now resolved or
deliberately deferred with reasoning recorded. Verdict updated to APPROVE.

---
*Review generated by /accelerator:review-work-item*
