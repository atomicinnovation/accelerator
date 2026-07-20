---
type: work-item-review
id: "0169-vcs-subdomain-and-hooks-migration-review-1"
title: "Work Item Review: VCS Subdomain and Hooks Migration"
date: "2026-07-20T09:34:16+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0169"
parent: "work-item:0136"
work_item_id: "0169"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-07-20T09:53:52+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: VCS Subdomain and Hooks Migration

**Verdict:** REVISE

This is a well-structured, densely populated story: every expected section is
present and substantive, the scope boundary (three hook scripts migrated,
migrate-discoverability deferred to 0172) is stated consistently across five
sections, and the bundling decision is consciously reasoned. The reasons to
revise are concentrated in two areas — verification coverage and a missing
upstream dependency. Three acceptance criteria attach to concrete parity gates,
but the four-subcommand VCS surface and the explicitly load-bearing
`classify_checkout` ordering are under-verified, and the 0164 fetch-verify-cache
model that AC #4 depends on is relied upon in the body yet absent from
Dependencies.

### Cross-Cutting Themes

- **`classify_checkout` is flagged as load-bearing but is neither fully
  disambiguated nor verified** (flagged by: clarity, testability) — Clarity
  notes the term carries overlapping descriptors ("6-line record" vs "full
  taxonomy"), so a reader cannot tell whether the taxonomy extends the shell's
  classification set or re-expresses it. Testability notes that this
  most-fragile behaviour has no acceptance criterion pinning arm order or
  taxonomy coverage. Together they mean the item's own highest-risk area is both
  ambiguously described and untested.
- **AC1 names four subcommands but the story only anchors verification for
  `detect`** (flagged by: testability, scope) — Testability flags that
  `status`, `log`, and `guard` have no stated pass/fail procedure; Scope
  observes that `status`/`log` aren't consumed by the hooks this story migrates
  and serve the skills instead. The gap in verification tracks the seam where
  the two bundled deliverables meet.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Dependency**: 0164 fetch-verify-cache dependency relied on in body but absent from Dependencies
  **Location**: Dependencies
  Requirements, Drafting Notes, and AC #4 all rely on the 0164 warmed-cache
  model, yet 0164 appears in neither `blocked_by`/`relates_to` nor the prose
  Dependencies section. A scheduler reading Dependencies would not know AC #4
  cannot be validated until 0164 lands.

- 🟡 **Testability**: AC1 verifies four subcommands against a detect-only parity gate
  **Location**: Acceptance Criteria
  AC1 requires `vcs detect|status|log|guard` all "reproduce the shell
  behaviours", but the only anchor named is the detect-focused
  `hooks/test-vcs-detect.sh`. There is no stated pass/fail procedure for
  `status`, `log`, or `guard` — no verification value for 75% of the named
  surface.

- 🟡 **Testability**: Load-bearing classify_checkout arm ordering and full taxonomy has no verifying criterion
  **Location**: Technical Notes / Acceptance Criteria
  `classify_checkout` first-match-wins ordering is called out as "load-bearing —
  preserve first-match-wins semantics exactly" and the crates must be extended
  with the full taxonomy (worktree/submodule/bare/`GIT_DIR`), yet no AC
  enumerates or verifies these arms or the ordering. The most fragile behaviour
  can regress while every AC still passes.

#### Minor

- 🔵 **Dependency**: Downstream stories 0172/0174 depend on this story's hooks.json changes but are captured only as 'related'
  **Location**: Dependencies
  This story rewrites `hooks.json` and leaves the shell tail that 0174 retires
  and the migrate-discoverability hook 0172 inherits. Both consume this story's
  output and must come after it, but they are recorded as "Related" with no
  explicit "Blocks" ordering, so they won't surface as unblocked when this
  closes.

- 🔵 **Scope**: Story bundles two independently-deliverable halves across a one-way dependency seam
  **Location**: Requirements
  Building the `accelerator-vcs` hexagon and migrating the hooks are joined by a
  clean one-way seam; `vcs status`/`log` are not consumed by the hooks this
  story migrates, weakening the "hooks are the primary consumer" rationale. The
  two halves could be delivered and verified independently, making this a large,
  harder-to-roll-back-atomically story.

- 🔵 **Testability**: AC3 'correct hook I/O envelope' lacks a defined expected reference
  **Location**: Acceptance Criteria
  AC3 requires emitting "the correct hook I/O envelope via `--format=hook`" but
  references no expected-envelope fixture or protocol shape, leaving "correct" to
  the verifier's judgement.

- 🔵 **Testability**: 'Keep the wrapper bash-3.2-safe' requirement has no verification criterion
  **Location**: Requirements
  The bash-3.2-safe requirement has no AC capturing it as a checkable outcome, so
  a bash-4 construct could slip past the definition of done — a recurring failure
  mode for this repo.

- 🔵 **Clarity**: Summary overstates SessionStart migration scope
  **Location**: Summary
  The Summary says the story migrates "the SessionStart/PreToolUse hook logic",
  implying all SessionStart logic moves, but only VCS and config detection are
  ported (migrate-discoverability deferred to 0172). A reader stopping at the
  Summary forms a broader mental model than the Requirements deliver.

#### Suggestions

- 🔵 **Testability**: AC4 'no sub-binary fetch' would benefit from a stated observation method
  **Location**: Acceptance Criteria
  AC4 is observable in principle but doesn't state how absence of a fetch is
  detected, risking flaky or hand-wavy verification of a caching behaviour.

- 🔵 **Scope**: Cross-subdomain `config detect` port is folded into a VCS-framed story
  **Location**: Requirements
  Beyond VCS + hooks, the story ports `accelerator config detect` (a config
  subdomain concern). Reasonable given the SessionStart fan-out, but surfacing it
  in the Summary/title would make the story's true span visible up front.

- 🔵 **Clarity**: 'Resolved Q4' is an undefined external referent
  **Location**: Context
  "Resolved Q4 fixes the mechanism" points to a refinement question numbered
  elsewhere that a reader without that source cannot resolve.

- 🔵 **Clarity**: 'classify_checkout' carries three overlapping descriptors
  **Location**: Requirements
  The same term is used for a fixed-shape "6-line record", a "full taxonomy", and
  "load-bearing arm ordering", leaving unclear whether the taxonomy extends the
  shell's classification set or re-expresses it.

- 🔵 **Clarity**: 'The four target platforms' are not named
  **Location**: Assumptions
  "The four target platforms are Unix" never enumerates which four, leaving the
  assumption's boundary ambiguous without epic-0136 context.

- 🔵 **Completeness**: Beneficiary of the migration is implied rather than stated
  **Location**: Summary / Context
  Context frames motivation via the ADR-0048 mandate but doesn't name who
  benefits (the epic-0136 shell retirement goal), a minor context gap for a story.

### Strengths

- ✅ The three-not-four script-removal scope boundary is stated identically in
  Context, Requirements, Acceptance Criteria, Technical Notes, and Drafting
  Notes — a reader cannot mistake which SessionStart handler is deferred.
- ✅ Upstream blockers carry live status context (0166 complete, 0167 in
  progress, 0179 complete on the feature branch), letting a scheduler see
  readiness at a glance.
- ✅ All expected sections are present and densely populated with substantive
  content rather than placeholders; frontmatter integrity is strong (recognised
  kind/status/priority and parent/blocked_by/relates_to relationships).
- ✅ Open Questions is explicitly reconciled ("None outstanding") with the
  resolution history captured in Drafting Notes, so the closed scope boundary was
  deliberate, not omitted.
- ✅ The bundling decision is consciously reasoned (hooks are the subdomain's
  primary consumer) and previously-open sizing questions (built-in vs sub-binary
  guard) were resolved and closed.
- ✅ AC3–AC5 bind reproduction and removal to concrete, observable anchors
  (parity gates, hook tests, named script removal, test-floor adjustment).

### Recommended Changes

1. **Add 0164 to Dependencies and frontmatter** (addresses: 0164 fetch-verify-cache
   dependency absent) — Add 0164 to the prose Dependencies section and to
   `blocked_by`/`relates_to` as appropriate, noting AC #4's warmed-cache
   guarantee is gated on 0164's fetch-verify-cache model.

2. **Give the four VCS subcommands per-subcommand verification anchors**
   (addresses: AC1 detect-only parity gate) — Split AC1 or name a pass/fail
   procedure for `status`, `log`, and `guard` (e.g. golden-output tests for
   `status`/`log`, an allow/deny fixture set for `guard`) so every named
   subcommand has a defined check.

3. **Add an AC that verifies `classify_checkout` arms and ordering** (addresses:
   classify_checkout untested; classify_checkout ambiguous descriptors) — Require
   a fixture covering each arm (worktree, submodule, bare, `GIT_DIR`, plain) and
   asserting first-match-wins precedence for at least one ambiguous checkout. While
   editing, state whether the "full taxonomy" extends the shell's classification
   set or re-expresses the 6-line record.

4. **Anchor AC3 to an expected hook I/O envelope** (addresses: AC3 lacks expected
   reference) — Reference a golden JSON fixture per hook type or the specific
   Claude Code hook I/O protocol fields the envelope must contain.

5. **Add an AC for the bash-3.2 wrapper constraint** (addresses: bash-3.2-safe has
   no criterion) — Require the wrapper to pass the bashisms linter / bash-3.2 gate.

6. **Capture downstream ordering to 0172/0174** (addresses: downstream stories only
   'related') — Add a "Blocks: 0172, 0174" entry or annotate the Related lines so
   the forward unblocking is explicit.

7. **(Optional) Tighten Summary and clarify referents** (addresses: Summary
   overstates scope; 'Resolved Q4'; four platforms; beneficiary) — Qualify the
   Summary to note SessionStart covers VCS + config detection only; drop or link
   the "Q4" label; name the four platforms or link 0136; consider surfacing the
   config-detect port in the Summary/title.

## Per-Lens Results

### Clarity

**Summary**: The work item is dense but generally well-disciplined: pronouns
resolve cleanly, the scope boundary is stated consistently, and ADR references
are linked. The main clarity risks are a Summary that overstates the SessionStart
migration scope, a couple of external referents used without in-document
definition ("Resolved Q4", "the four target platforms"), and the multi-descriptor
use of "classify_checkout".

**Strengths**:
- The three-not-four script-removal boundary is stated identically across five
  sections.
- Pronoun usage is tight — "it" and "those crates" resolve to a single named
  referent.
- Acronyms (ADR-0048, ADR-0053) are linked in References.

**Findings**:
- 🔵 minor / medium — **Summary overstates SessionStart migration scope**
  (Summary): "migrate the SessionStart/PreToolUse hook logic" implies all
  SessionStart logic moves, but only VCS + config detection are ported. Suggest
  qualifying the Summary.
- 🔵 suggestion / medium — **'Resolved Q4' is an undefined external referent**
  (Context): "Q4" is never defined in the work item. Suggest dropping or linking
  it.
- 🔵 suggestion / low — **'The four target platforms' are not named**
  (Assumptions): the four platforms are never enumerated. Suggest naming them or
  linking 0136.
- 🔵 suggestion / low — **'classify_checkout' carries three overlapping
  descriptors** (Requirements): "6-line record", "full taxonomy", and "arm
  ordering" leave the record/taxonomy relationship ambiguous. Suggest stating
  whether the taxonomy extends the shell's set.

### Completeness

**Summary**: Structurally and informationally complete for a story: every
expected section is present and substantively populated, frontmatter is
well-formed, Context explains the motivating forces, and the five acceptance
criteria concretely define done. No completeness gaps rise to major severity.

**Strengths**:
- All expected sections carry substantive content rather than placeholders.
- Frontmatter integrity is strong (kind, status, priority, relationships all
  present and valid).
- Context explains the "why" rather than restating the summary.
- Open Questions is explicitly reconciled with resolution history in Drafting
  Notes.
- Acceptance Criteria each tie to a concrete verifiable artefact.

**Findings**:
- 🔵 suggestion / low — **Beneficiary of the migration is implied rather than
  stated** (Summary / Context): motivation is framed via the ADR mandate without
  naming who benefits (the epic-0136 shell retirement). Suggest a short clause
  naming the beneficiary.

### Dependency

**Summary**: The Dependencies section is well-populated and disciplined — each
blocker carries a live status annotation and the deferred migrate-discoverability
boundary is explicit. The one material gap is 0164: its fetch-verify-cache model
is relied upon twice and is the sole enabler of AC #4, yet appears nowhere in
frontmatter or the prose Dependencies section. Downstream 0172/0174 are captured
only as "related" rather than an explicit "blocks" ordering.

**Strengths**:
- Upstream blockers captured with live status context.
- The migrate-discoverability boundary and its ownership by 0172 is explicit in
  four sections.
- External tools jj/git named behind an outbound port; platform assumption
  stated.

**Findings**:
- 🟡 major / high — **0164 fetch-verify-cache dependency relied on in body but
  absent from Dependencies** (Dependencies): AC #4 is verifiable only if 0164's
  cache model exists, yet 0164 is in neither frontmatter nor the prose section.
  Add 0164 to Dependencies and `blocked_by`/`relates_to`.
- 🔵 minor / medium — **Downstream stories 0172/0174 depend on this story's
  hooks.json changes but are captured only as 'related'** (Dependencies): both
  consume this story's output and must follow it, but no "Blocks" ordering is
  recorded. Add a "Blocks: 0172, 0174" entry.

### Scope

**Summary**: Thematically coherent — the Drafting Notes explicitly justify
bundling the VCS subdomain with the hooks migration, and the in/out boundaries
are drawn deliberately and confirmed with the author. The main observation is that
the story spans two deliverables with a clean dependency seam and folds in a
cross-subdomain `config detect` port; `status`/`log` are built here but consumed
by skills, not the hooks. Defensible bundles rather than a scattered grab-bag.

**Strengths**:
- Scope boundaries drawn explicitly and defended (three scripts removed, not
  four; author-confirmed).
- The bundling decision is consciously reasoned rather than accidental, with
  sizing questions resolved and closed.
- The unit of work is well-anchored in its epic with clear upstream/downstream
  seams.

**Findings**:
- 🔵 minor / medium — **Story bundles two independently-deliverable halves across
  a one-way dependency seam** (Requirements): `status`/`log` aren't consumed by
  the migrated hooks, weakening the "primary consumer" rationale; the halves could
  split at the existing seam. If the bundle is intentional for delivery
  efficiency, the Drafting Notes justification suffices.
- 🔵 suggestion / medium — **Cross-subdomain `config detect` port folded into a
  VCS-framed story** (Requirements): reasonable given the SessionStart fan-out and
  acknowledged in Context/Drafting Notes; consider surfacing it in the
  Summary/title so the true span is visible.

### Testability

**Summary**: Most acceptance criteria anchor to concrete pre-existing harnesses
(the repointed `test-vcs-detect.sh` parity gate, the config-detect hook test,
observable script removal), making them genuinely testable. The weak spots are
coverage gaps: AC1 names four subcommands but cites only a detect-focused parity
gate, and the load-bearing `classify_checkout` arm ordering and full taxonomy
have no criterion pinning them down.

**Strengths**:
- AC1/AC2 bind "reproduce the shell behaviours" to named parity gates.
- AC4 turns the caching requirement into an observable pass/fail.
- AC5 is concrete and mechanically checkable (three named scripts removed, floor
  adjusted, migrate-discoverability retained).

**Findings**:
- 🟡 major / high — **AC1 verifies four subcommands against a detect-only parity
  gate** (Acceptance Criteria): no pass/fail procedure for `status`, `log`,
  `guard`. Split AC1 or name a verification anchor per subcommand.
- 🟡 major / high — **Load-bearing classify_checkout arm ordering and full
  taxonomy has no verifying criterion** (Technical Notes / Acceptance Criteria):
  the most fragile behaviour can regress while every AC passes. Add an AC covering
  each arm and asserting first-match-wins precedence.
- 🔵 minor / medium — **AC3 'correct hook I/O envelope' lacks a defined expected
  reference** (Acceptance Criteria): "correct" is left to verifier judgement.
  Anchor to a golden envelope fixture or explicit protocol fields.
- 🔵 minor / medium — **'Keep the wrapper bash-3.2-safe' requirement has no
  verification criterion** (Requirements): a bash-4 construct could slip past the
  definition of done. Add an AC that the wrapper passes the bashisms/bash-3.2
  gate.
- 🔵 suggestion / low — **AC4 'no sub-binary fetch' would benefit from a stated
  observation method** (Acceptance Criteria): specify the check (e.g. zero fetch
  invocations against a stubbed fetcher across N guard calls after warm cache).

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-07-20

**Verdict:** APPROVE

All twelve findings from the initial review were addressed by edits, and a fresh
five-lens pass confirmed their resolution. That pass surfaced a handful of new
items — two majors, both of which have since been fixed by follow-up edits (one
was an imprecision introduced by the initial edit). The remaining items are all
suggestion-level and either deliberately left or already acknowledged.

### Previously Identified Issues
- 🟡 **Dependency**: 0164 fetch-verify-cache dependency absent — Resolved (0164 added
  to `blocked_by` and Dependencies with an AC-gating rationale).
- 🟡 **Testability**: AC1 verifies four subcommands against a detect-only gate —
  Resolved (split into per-subcommand ACs: detect parity gate, status/log golden
  tests, guard allow/deny fixtures).
- 🟡 **Testability**: classify_checkout arm ordering/taxonomy unverified — Resolved
  (dedicated AC enumerating each arm and asserting first-match-wins precedence).
- 🔵 **Dependency**: 0172/0174 captured only as 'related' — Resolved (`blocks`
  frontmatter field + Blocks entry in Dependencies).
- 🔵 **Testability**: AC3 envelope lacked expected reference — Resolved (golden
  envelope fixture per hook type pinning protocol fields).
- 🔵 **Testability**: bash-3.2 requirement had no criterion — Resolved (AC binding
  the wrapper to `lint-bashisms.sh` + shfmt/ShellCheck).
- 🔵 **Clarity**: Summary overstated SessionStart scope — Resolved (qualified to VCS
  + config detection only).
- 🔵 **Testability**: AC4 lacked observation method — Resolved (zero fetch
  invocations against a stubbed/instrumented fetcher).
- 🔵 **Scope**: config-detect folded into VCS-framed story — Resolved (surfaced in
  the Summary).
- 🔵 **Clarity**: 'Resolved Q4' undefined referent — Resolved (rephrased to
  "resolved during refinement of the parent epic 0136").
- 🔵 **Clarity**: 'classify_checkout' overlapping descriptors — Resolved ("full
  taxonomy" defined inline as reproducing the shell's set, not extending it).
- 🔵 **Clarity**: four platforms not named — Resolved (exact target triples from
  `tasks/shared/targets.py`).
- 🔵 **Completeness**: beneficiary implied — Resolved (Context names the epic-0136
  shell retirement as the goal served).

### New Issues Introduced (by re-review) and their disposition
- 🟡 **Testability**: status/log golden test input set ("the fixture repos")
  undefined — an imprecision from the initial edit; **fixed** (AC2 now enumerates a
  minimum fixture-repo set: clean/dirty git, ahead-behind, detached-HEAD, clean/dirty
  jj).
- 🟡 **Dependency**: config-subdomain upstream for the `config detect` port not
  captured — **fixed** (the 0167 Dependencies line now states 0167 delivers the
  built-in `config` command this story extends; 0167 was already a blocker).
- 🔵 **Testability**: guard allow/deny classifications not enumerated — **fixed** (AC
  now requires covering each Bash-call class the shell guard distinguishes, not just
  one allow + one deny).
- 🔵 **Clarity**: Context sentence splice from the beneficiary edit — **fixed**
  (capitalised the following sentence).
- 🔵 **Testability**: 'comparable' warm-cache cost unbounded — **fixed** (Assumptions
  now states zero-fetch is the only gated guarantee; latency parity is an assumption).
- 🔵 **Clarity** (suggestion): '6-line record' count vs five-variant taxonomy — left
  as-is; "6-line" is the author's precise term for the record shape and changing it
  risks introducing an inaccuracy.
- 🔵 **Clarity** (suggestion): 'port' used as both hexagonal noun and migration verb —
  left as-is; domain readers resolve it from context (nitpick).
- 🔵 **Scope** (suggestion): subdomain build + hooks migration bundled; status/log
  built ahead of consumers — left as-is; the bundling is defensible and
  author-justified, and status/log consumers land in later skill-migration stories
  (0170, 0173).

### Assessment
The work item is ready for implementation. Every substantive finding (all three
original majors and both re-review majors) is resolved; the acceptance criteria now
give each subcommand, the load-bearing `classify_checkout` ordering, the hook
envelope, the warmed-cache guard, and the bash-3.2 constraint a concrete pass/fail
procedure, and the dependency graph (0164/0166/0167/0179 upstream, 0172/0174
downstream) is consistent across frontmatter and prose. The unresolved items are
suggestion-level polish that do not block planning.
