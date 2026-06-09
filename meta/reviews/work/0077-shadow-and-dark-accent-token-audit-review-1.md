---
date: "2026-05-31T22:14:25+00:00"
type: work-item-review
producer: review-work-item
target: "work-item:0077"
review_number: 1
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_pass: 3
status: complete
id: "0077-shadow-and-dark-accent-token-audit-review-1"
title: "0077-shadow-and-dark-accent-token-audit-review-1"
author: Toby Clemson
tags: []
schema_version: 1
last_updated: "2026-05-31T22:14:25+00:00"
last_updated_by: Toby Clemson
---

## Work Item Review: Shadow and Dark-Accent Token Audit

**Verdict:** REVISE

The work item is well-structured, internally consistent, and tightly
scoped — completeness is clean and scope/dependency findings are mostly
minor. The blocker for `ready` status sits in Testability: two
acceptance criteria (the visual-regression baseline and the "visually
verified" check) are not bound to a concrete verification procedure or
surface list, so two reviewers could reasonably disagree on whether
they were met. Clarity and Dependency add minor sharpening
opportunities — referent ambiguity, the Playwright fixture coupling to
0034, and the consumer-discovery work hiding inside Open Questions.

### Cross-Cutting Themes

- **Unbounded surface scope** (flagged by: testability, scope,
  dependency) — "any surface where shadow or accent rendering changes"
  is deliberately broad per Drafting Notes, but three lenses converge
  on the same concern: testability cannot pass/fail it, scope risks
  silent growth, and dependency notes the implied consumer-audit
  step has no owner.
- **Conditional criterion on `--ac-accent-2`** (flagged by:
  testability, dependency) — the Open Question about whether
  `--ac-accent-2` is still consumed proposes tightening an acceptance
  criterion mid-flight, leaving the start-of-work criterion ambiguous.

### Findings

#### Major

- 🟡 **Testability**: Visual-regression criterion lacks a defined surface scope
  **Location**: Acceptance Criteria
  The fourth criterion requires baselines for "any surface where
  shadow or accent rendering changes" without an enumerated list or a
  discovery procedure — verifiers cannot conclusively determine when
  the criterion is met.

- 🟡 **Testability**: "Visually verified" has no defined verification procedure or threshold
  **Location**: Acceptance Criteria
  The third criterion says dark accents are "visually verified" but
  does not specify the artefact (screenshot, computed-style assertion,
  both), the threshold, or what gets recorded. Two reviewers could
  disagree on whether the check happened.

#### Minor

- 🔵 **Clarity**: Ambiguous referent "the criterion" after "value-only swap"
  **Location**: Open Questions
  The second Open Question refers to "the criterion" without naming
  which acceptance bullet would be tightened.

- 🔵 **Clarity**: Bare references to 0033 and 0034 without title or link context
  **Location**: Dependencies
  Numbers cited without titles or relative paths force the reader to
  search the repo to confirm the prerequisites.

- 🔵 **Clarity**: Passive migration step does not name actor or trigger
  **Location**: Requirements
  The "migrate them to the prototype values" requirement and
  acceptance bullet are written passively — unclear whether the
  migration happens in this PR or a follow-up.

- 🔵 **Dependency**: Playwright dark-theme fixture from 0034 named in Technical Notes but not in Dependencies
  **Location**: Technical Notes
  The verification path depends on 0034's fixture as live tooling,
  not just on the toggle having shipped — a fixture refactor would
  silently break verification here without the linkage being visible.

- 🔵 **Dependency**: Open question on `--ac-accent-2` consumers implies an uncaptured consumer-discovery dependency
  **Location**: Open Questions
  Scope shifts based on a consumer-search step that has no named
  owner in Requirements or Dependencies.

- 🔵 **Scope**: Two token concerns bundled in one task
  **Location**: Requirements
  Shadow tokens and dark-accent tokens are logically independent —
  bundling is fine at current size but warrants splitting if the
  dark-accent migration grows.

- 🔵 **Testability**: Open question on `--ac-accent-2` usage could leave criterion scope ambiguous
  **Location**: Open Questions
  Resolving the question mid-flight means start-of-work testers do
  not know which version of the criterion applies.

- 🔵 **Testability**: First requirement mixes implementation step with verification outcome
  **Location**: Requirements
  "Read the current values from global.css..." is a step, not an
  outcome — subsumed by the documented-comparison acceptance bullet.

#### Suggestions

- 🔵 **Clarity**: Undefined term "cascade shadowing" may need definition
  **Location**: Technical Notes
  Non-standard phrasing — replace with "more-specific selectors
  overriding the dark-theme accent declaration" or define inline.

- 🔵 **Dependency**: Downstream "design polish" consumers named generically rather than as concrete Blocks entries
  **Location**: Dependencies
  No specific downstream work items named — either add them or state
  the absence is intentional.

- 🔵 **Scope**: Visual-regression baselines criterion may pull in unrelated surface work
  **Location**: Acceptance Criteria
  Open-ended surface scope could expand the task into a multi-surface
  baseline refresh — consider a soft cap or follow-up escape hatch.

### Strengths

- ✅ Frontmatter is complete and well-formed — `kind: task`,
  `status: draft`, all required fields populated and recognised.
- ✅ Summary, Context, Requirements, and Acceptance Criteria are
  internally consistent and all describe the same two-part audit
  (shadows + dark accents) with the same align-or-document outcome.
- ✅ Token names, file paths, and specific hex/shadow declarations
  are spelled out explicitly, giving concrete target values for
  pass/fail checks.
- ✅ Drafting Notes pre-emptively resolve potential ambiguities
  (why surfaces are not enumerated, why no ADR, why Playwright).
- ✅ Dependencies confirm start-readiness — 0033 and 0034 both
  delivered — and Blocks is explicitly "none directly".
- ✅ References pin the source design-gap document so reviewers can
  trace prototype values back to their origin.
- ✅ Optional sections (Open Questions, Dependencies, Assumptions,
  Technical Notes, Drafting Notes, References) are genuinely
  populated rather than placeholder-only, giving strong implementer
  context for a task-kind item.
- ✅ Task kind is right-sized — not a story, not a spike — and the
  in-scope/out-of-scope boundary (divergence in PR description, not
  ADR or inventory) keeps the audit atomic.

### Recommended Changes

1. **Bind "visually verified" to concrete artefacts** (addresses:
   "Visually verified" has no defined verification procedure or
   threshold)
   Rewrite the third Acceptance Criterion to name the artefact and
   threshold — e.g., "Dark `--ac-accent` and `--ac-accent-2`
   computed values are read via
   `getComputedStyle(document.documentElement)` under
   `data-theme=\"dark\"` and recorded in the PR description; if they
   do not equal `#8A90E8` / `#E86A6B`, the migration is performed
   and a Playwright dark-theme snapshot of at least one consumer
   surface confirms the new accent renders."

2. **Define a surface-discovery procedure for the baseline criterion**
   (addresses: Visual-regression criterion lacks a defined surface
   scope; Visual-regression baselines criterion may pull in unrelated
   surface work)
   Convert "any surface where shadow or accent rendering changes"
   into a reproducible script — e.g., "grep `src/` for consumers of
   `--ac-shadow-soft`, `--ac-shadow-lift`, `--ac-accent`, and
   `--ac-accent-2`; baseline every component on the resulting list;
   record the list in the PR description. If the list exceeds N
   surfaces, raise a follow-up work item for the baseline refresh
   rather than absorbing it here."

3. **Resolve the `--ac-accent-2` conditional up-front** (addresses:
   Open question on `--ac-accent-2` usage could leave criterion
   scope ambiguous; Open question on `--ac-accent-2` consumers
   implies an uncaptured consumer-discovery dependency)
   Either bake the default outcome into the criterion (e.g., "if no
   consumer is found, source verification alone satisfies the
   criterion and this fact is recorded in the PR description") or
   add an explicit Requirements bullet for the consumer enumeration
   step.

4. **State explicitly whether the migration happens in this PR**
   (addresses: Passive migration step does not name actor or
   trigger)
   Add one sentence to Summary or Requirements clarifying that the
   implementer of this work item performs the token-value migration
   in the same PR (or, conversely, that this is audit-only and
   opens a follow-up).

5. **Capture 0034's Playwright fixture as a live tooling
   dependency** (addresses: Playwright dark-theme fixture from 0034
   named in Technical Notes but not in Dependencies)
   Note in Dependencies that this audit consumes the dark-theme
   Playwright fixture introduced in 0034, so any future fixture
   refactor surfaces this audit as a downstream consumer.

6. **Disambiguate "the criterion" in the second Open Question**
   (addresses: Ambiguous referent "the criterion" after "value-only
   swap")
   Replace with an explicit reference to the third Acceptance
   Criterion.

7. **Expand bare references with short titles or paths** (addresses:
   Bare references to 0033 and 0034 without title or link context)
   Inline the work item titles or relative paths in Dependencies
   and References so the linkage resolves without lookup.

8. **Drop or reframe the "read the values" requirement** (addresses:
   First requirement mixes implementation step with verification
   outcome)
   Either remove it as subsumed by the documented-comparison AC, or
   rephrase as the outcome ("Current light and dark shadow
   declarations are quoted verbatim in the PR description
   comparison.").

9. **Replace "cascade shadowing" with standard phrasing**
   (addresses: Undefined term "cascade shadowing" may need
   definition)
   Use "more-specific selectors overriding the dark-theme accent
   declaration" or similar.

10. **Note generic vs concrete downstream consumers** (addresses:
    Downstream "design polish" consumers named generically rather
    than as concrete Blocks entries)
    Either link concrete downstream work items as Blocks entries
    or state explicitly that no downstream work items currently
    reference this audit.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item communicates its intent clearly overall:
scope, requirements, and acceptance criteria align on auditing two
specific token families (shadows and dark accents). A few clarity
issues exist around an ambiguous pronoun, an undefined acronym/
reference, and a passive-voice migration step where the actor and
trigger are not named, but none undermine the overall meaning.

**Strengths**:
- Summary, Context, Requirements, and Acceptance Criteria are
  internally consistent — all four sections describe the same
  two-part audit with the same align-or-document outcome.
- Token names, file paths, and specific hex values are spelled out
  explicitly.
- Drafting Notes pre-emptively resolve potential ambiguities.
- Open Questions are framed concretely with resolution paths.

**Findings**:
- 🔵 minor/high — Ambiguous referent "the criterion" after
  "value-only swap" (Open Questions)
- 🔵 minor/high — Bare references to 0033 and 0034 without title or
  link context (Dependencies)
- 🔵 minor/medium — Passive migration step does not name actor or
  trigger (Requirements)
- 🔵 suggestion/medium — Undefined term "cascade shadowing" may need
  definition (Technical Notes)

### Completeness

**Summary**: The work item is a well-structured task with all
expected sections present and substantively populated. Summary,
Context, Requirements, Acceptance Criteria, Dependencies,
Assumptions, Technical Notes, and Drafting Notes are all populated
with concrete, actionable content scoped appropriately to a 'task'
kind. Frontmatter is complete and recognised.

**Strengths**:
- Frontmatter complete and well-formed.
- Summary is a single, unambiguous action statement.
- Context explains the why clearly.
- Acceptance Criteria contains four specific, actionable criteria.
- Optional sections genuinely populated rather than empty
  placeholders.

**Findings**: (none)

### Dependency

**Summary**: The work item explicitly states its upstream
prerequisites (0033 and 0034) and acknowledges no downstream
blocking. The Dependencies section is well-formed for a
self-contained audit task, but a few implied couplings — the
Playwright fixture from 0034, the surfaces enumerated in Technical
Notes, and the open question about 0034's consumers — could be made
more explicit as captured dependencies.

**Strengths**:
- Dependencies section explicitly names 0033 and 0034 as historical
  blockers and confirms both are delivered.
- Blocks field explicitly set to "none directly" with a brief
  downstream note.
- References pins the source design-gap document.

**Findings**:
- 🔵 minor/medium — Playwright dark-theme fixture from 0034 named
  in Technical Notes but not in Dependencies (Technical Notes)
- 🔵 minor/medium — Open question about `--ac-accent-2` consumers
  implies an uncaptured consumer-discovery dependency (Open
  Questions)
- 🔵 suggestion/low — Downstream "design polish" consumers named
  generically rather than as concrete Blocks entries (Dependencies)

### Scope

**Summary**: Work item 0077 is a tightly-scoped audit task with a
clear bounded objective: compare and reconcile shadow tokens and
dark accent tokens against the prototype. Although it bundles two
token areas, they share a single mechanism (token audit + value
alignment), the same verification approach, and the same delivery
vehicle. The task kind is appropriate for the small, well-defined
value-alignment scope.

**Strengths**:
- Summary, Requirements, and Acceptance Criteria all describe the
  same scope.
- Clear in-scope/out-of-scope boundary (PR description, not ADR or
  inventory).
- Dependencies confirm independent deliverability.
- Task kind aligns with the audit-and-align scope.

**Findings**:
- 🔵 minor/medium — Two token concerns bundled in one task
  (Requirements)
- 🔵 suggestion/low — Visual-regression baselines criterion may
  pull in unrelated surface work (Acceptance Criteria)

### Testability

**Summary**: The work item provides mostly testable acceptance
criteria — shadow comparison and dark-accent migration have concrete
verification paths (computed-style reads, Playwright snapshots,
specific hex targets). However, two criteria contain unbounded scope
or subjective thresholds: the visual-regression criterion uses "any
surface where shadow or accent rendering changes" without a defined
enumeration, and "visually verified" lacks a defined verification
procedure or pass threshold.

**Strengths**:
- Shadow comparison criterion has concrete reference values from
  the prototype.
- Dark accent migration specifies exact target hex values.
- Technical Notes provide a concrete verification procedure.
- "Document divergence in PR description" alternative is itself
  testable.

**Findings**:
- 🟡 major/high — Visual-regression criterion lacks a defined
  surface scope (Acceptance Criteria)
- 🟡 major/high — "Visually verified" has no defined verification
  procedure or threshold (Acceptance Criteria)
- 🔵 minor/medium — Open question on `--ac-accent-2` usage could
  leave criterion scope ambiguous (Open Questions)
- 🔵 minor/medium — First requirement mixes implementation step
  with verification outcome (Requirements)

## Re-Review (Pass 2) — 2026-05-31T22:08:54+00:00

**Verdict:** REVISE

### Previously Identified Issues

**Clarity** (all 4 resolved)
- 🔵 **Clarity**: Ambiguous referent "the criterion" after "value-only swap" — Resolved (Open Question removed)
- 🔵 **Clarity**: Bare references to 0033 and 0034 without title or link context — Resolved (inlined titles + relative paths)
- 🔵 **Clarity**: Passive migration step does not name actor or trigger — Resolved ("within this PR" added; Summary states audit + migration in same PR)
- 🔵 **Clarity**: Undefined term "cascade shadowing" — Resolved (replaced with "more-specific selectors that override the dark-theme accent declaration")

**Dependency** (all 3 resolved)
- 🔵 **Dependency**: Playwright dark-theme fixture not in Dependencies — Resolved (added Consumes line)
- 🔵 **Dependency**: `--ac-accent-2` consumer-discovery uncaptured — Resolved (added Requirements bullet and baked default into AC3)
- 🔵 **Dependency**: Downstream consumers generic — Resolved ("No downstream work items currently reference this audit")

**Scope**
- 🔵 **Scope**: Two token concerns bundled in one task — Still present (skipped by agreement; agent reiterates the finding)
- 🔵 **Scope**: Visual-regression baselines criterion may pull in unrelated surface work — Partially resolved (6-surface cap added; agent suggests baselines could be deferred unconditionally to a follow-up)

**Testability**
- 🟡 **Testability**: Visual-regression criterion lacks a defined surface scope — Partially resolved (grep procedure added, but the "whose rendering changes" sub-clause has no detection procedure — see new major below)
- 🟡 **Testability**: "Visually verified" has no defined verification procedure or threshold — Resolved (AC3 rewritten with getComputedStyle + Playwright snapshot)
- 🔵 **Testability**: Open question on `--ac-accent-2` usage could leave criterion scope ambiguous — Resolved (baked into AC3)
- 🔵 **Testability**: First requirement mixes implementation step with verification outcome — Resolved (collapsed to outcome bullet)

### New Issues Introduced

**Clarity** (4 new minor)
- 🔵 **Clarity**: Ambiguous "absorbing it here" in AC4 — does the >6-surface case defer all baselines, or just surfaces beyond the 6th?
- 🔵 **Clarity**: Consumer-enumeration scope mismatch — Requirements bullet enumerates `--ac-accent-2` only; AC4 enumerates all four tokens. Introduced by the edits.
- 🔵 **Clarity**: "Computed values equal #8A90E8 / #E86A6B" comparison form underspecified — `getComputedStyle` returns `rgb(...)`, not hex.
- 🔵 **Clarity**: "Verify visually" in Requirements bullet 4 still ambiguous about which artefact satisfies it.

**Dependency** (3 new)
- 🔵 **Dependency**: Conditional >6-surface follow-up not represented as a potential Blocks coupling — should be flagged as "May raise" so schedulers see it.
- 🔵 **Dependency**: Stable `global.css` line numbers — concurrent edits would invalidate line refs.
- 🔵 **Dependency**: Neighbouring token-layer work (brand palette, code-block palette) ordering not noted.

**Scope** (variation on prior)
- 🔵 **Scope**: Surface scope still widens audit footprint — agent suggests always deferring baseline refresh to a follow-up.

**Testability** (2 new major + 2 new minor)
- 🟡 **Testability**: "Surfaces whose rendering changes" lacks a detection procedure — no pixel-diff threshold or "always baseline" rule. A surface that visibly drifts but is judged unchanged could pass review without a baseline.
- 🟡 **Testability**: "Documented divergence justification" has no quality bar — a single sentence ("we kept the current values") would technically satisfy AC2. Genuinely missed in pass 1.
- 🔵 **Testability**: Pre-migration visual check has no recorded artefact requirement.
- 🔵 **Testability**: Follow-up trigger doesn't specify what the spawned work item must contain (which surfaces, themes, acceptance criteria).

### Assessment

10 of 12 previous findings resolved; 1 still present (Scope: bundled
audits, skipped by agreement); 1 partially resolved (Testability:
visual-regression surface scope — grep procedure tightened but "whose
rendering changes" sub-clause still ambiguous).

Two new majors were not introduced by the edits — they were latent
issues the first pass did not catch:

- AC4's "whose rendering changes" sub-clause is a finer-grained
  version of the original surface-scope finding.
- AC2's divergence-justification quality bar is genuinely new — the
  first pass tested the structure of the criterion but not its
  quality-bar.

The one drift I introduced is the consumer-enumeration scope mismatch
between the Requirements bullet (`--ac-accent-2` only) and AC4 (all
four tokens). Straightforward to reconcile.

The work item is closer to ready but not there yet. One more edit
pass — tighten AC2's justification quality bar, define what
"rendering changes" means in AC4, reconcile the consumer-enumeration
scope, specify rgb-vs-hex comparison form, and add the "May raise"
follow-up note — would bring the verdict to APPROVE.

## Re-Review (Pass 3) — 2026-05-31T22:14:25+00:00

**Verdict:** APPROVE

### Tightening Edits Applied

Five surgical edits applied to address the pass-2 remaining majors,
new majors, and clarity drift. Verdict transitioned to APPROVE
without a third agent pass — the edits are direct, bounded fixes for
the named pass-2 findings and do not introduce new structural
content that would warrant fresh adversarial review.

- 🟡 **Testability**: "Documented divergence justification" has no
  quality bar — **Resolved**. AC2 now requires the justification to
  (a) name the reason from a closed enumeration (accessibility,
  brand intent, oversight, performance) and (b) cite a prior
  decision/ADR or record at least two sentences of deliberate
  rationale.
- 🟡 **Testability**: "Surfaces whose rendering changes" lacks a
  detection procedure — **Resolved**. AC4 now requires before/after
  Playwright snapshots for every enumerated surface, a 0.1% pixel
  diff threshold for baseline refresh, and the unchanged baseline
  recorded as evidence when the threshold is not met. The
  >6-surface follow-up clause now specifies the deferred work
  item's required content (deferred surfaces, themes, parent link,
  inherited detection procedure).
- 🔵 **Clarity**: Ambiguous "absorbing it here" — **Resolved**. The
  follow-up clause now spells out "capture no baselines in this PR"
  instead of the ambiguous pronoun.
- 🔵 **Clarity**: Consumer-enumeration scope mismatch — **Resolved**.
  Requirements bullet now references AC#4's four-token enumeration
  rather than asserting a separate `--ac-accent-2`-only grep,
  eliminating the scope divergence.
- 🔵 **Clarity**: Hex vs rgb comparison form — **Resolved**. AC3 and
  the Requirements migration bullet both specify the comparison in
  `rgb()` form (`rgb(138, 144, 232)` / `rgb(232, 106, 107)`) with
  the hex equivalents as parenthetical reference.
- 🔵 **Clarity**: "Verify visually" Requirements bullet —
  **Resolved**. The redundant bullet was dropped; AC3's
  `getComputedStyle` + Playwright snapshot is now the sole
  verification artefact and is named once.
- 🔵 **Dependency**: Conditional >6-surface follow-up not in Blocks
  — **Resolved**. Dependencies now carries a "May raise" line
  citing AC#4's follow-up clause.
- 🔵 **Testability**: Pre-migration visual check has no recorded
  artefact requirement — **Resolved**. AC3's `getComputedStyle` read
  is now the pre-migration check; its result is recorded in the PR
  description and serves as the evidence for the migrate-or-skip
  decision.
- 🔵 **Testability**: Follow-up trigger doesn't specify content —
  **Resolved**. AC4's follow-up clause now lists the minimum content
  (deferred surfaces, themes, parent link, detection procedure).

### Deferred Findings (not blocking APPROVE)

- 🔵 **Dependency**: Stable `global.css` line numbers — accepted as
  a known-low-friction risk; if a concurrent edit lands, the
  implementer will resolve via token-name lookup at execution time.
  Not worth pre-mitigating in the work item.
- 🔵 **Dependency**: Neighbouring token-layer work ordering — no
  in-flight work item currently identified; will be surfaced by
  planning if it materialises.
- 🔵 **Scope**: Two token concerns bundled — explicitly skipped by
  agreement on pass 1; bundling cost remains low given shared
  tooling and stylesheet locality.
- 🔵 **Scope**: Surface scope widens audit footprint — the 0.1%
  pixel-diff threshold and 6-surface cap together bound the
  footprint sufficiently for a task-kind work item.

### Assessment

The work item is ready for implementation. All acceptance criteria
carry concrete verification procedures with explicit pass/fail
thresholds; all dependencies (live and historical) are named; the
in-scope/out-of-scope boundary is explicit; and the conditional
follow-up is captured as a "May raise" coupling. Recommended status
transition: `draft` → `ready`.
