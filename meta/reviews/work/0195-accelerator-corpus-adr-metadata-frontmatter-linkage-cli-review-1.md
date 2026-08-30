---
type: "work-item-review"
id: "0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli-review-1"
title: "Work Item Review: accelerator-corpus: ADR, Metadata, Frontmatter Validation, and Linkage CLI"
date: "2026-08-05T19:24:34+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:0136"
target: "work-item:0195"
work_item_id: "0195"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-08-06T00:06:12+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: accelerator-corpus: ADR, Metadata, Frontmatter Validation, and Linkage CLI

**Verdict:** REVISE

0195 is a well-formed split from the abandoned 0173 — its Dependencies section
resolves every prior blocker with provenance, its Acceptance Criteria are
mostly concrete and enumerable, and it carries forward the resolved Q5
grouping rationale transparently. The dominant problem is a single unreconciled
loose end inherited from the split: a Technical Notes aside about an
"interface-redesign rewrite" for `artifact-derive-metadata`'s output contract
contradicts the parity/"reproduces" framing used everywhere else, and three
lenses independently flagged it as leaving the actual pass/fail baseline for
that subcommand ambiguous. A second, unrelated gap is a runtime dependency —
how the compiled work-item-id regex reaches the standalone binary — that isn't
named.

### Cross-Cutting Themes

- **The "interface-redesign rewrite" aside is unreconciled with the
  "reproduces" framing used everywhere else** (flagged by: clarity,
  completeness, testability) — Technical Notes says to "treat its output
  contract carefully under the interface-redesign rewrite" for
  `artifact-derive-metadata`, implying the contract may change, while Summary,
  Requirements, and Acceptance Criteria all describe the CLI's job as
  reproducing existing behaviour. Since this subcommand is "invoked by many
  skills," the ambiguity is correctness-critical, not cosmetic — three
  independent lenses reached the same conclusion from different angles
  (undefined jargon, missing requirement, unverifiable acceptance criterion).
- **"Reproduces" has no defined equivalence bar** (flagged by: testability,
  echoed by clarity's "per subcommand" ambiguity) — none of byte-identical
  output, exit-code parity, or "behaviour documented elsewhere" is stated as
  the comparison method, for any of the four subcommand groups, not only the
  flagged one.

### Findings

#### Major

- 🔴 **Clarity/Completeness/Testability**: "Interface-redesign rewrite" is
  undefined and contradicts the parity framing used elsewhere
  **Location**: Technical Notes; Acceptance Criteria
  Technical Notes states `artifact-derive-metadata`'s output contract should
  be treated carefully "under the interface-redesign rewrite," but no
  Requirement or Acceptance Criterion defines what this redesign entails, and
  every other section frames the CLI as reproducing existing behaviour
  byte-for-byte. An implementer cannot tell whether the output contract is
  expected to change or must be preserved, risking either broken downstream
  consumers or a missed intended improvement.

- 🔴 **Testability**: "Reproduces" lacks a defined equivalence bar
  **Location**: Acceptance Criteria
  AC1 requires the CLI to "reproduce" ADR numbering/status, artifact metadata,
  frontmatter validation, and linkage queries, but never states the comparison
  method (byte-identical stdout, exit-code parity, JSON-shape equivalence, or
  behaviour-only equivalence) — two implementers could disagree on whether a
  subtly different output still "reproduces" the original.

- 🟡 **Dependency**: Runtime source of the compiled work-item-id scan regex is
  not named as a dependency
  **Location**: Dependencies
  The sibling library item (0179) injects the work-item-id scan regex from an
  existing bash compiler at *test* time only, and explicitly defers the
  regex-producing DSL compiler to 0170/0167. Neither 0179 nor this item states
  what supplies the compiled regex at CLI *runtime* once `accelerator-corpus`
  is a standalone binary with no bash library to shell out to — if that
  source is 0170's not-yet-built compiler, this is an unnamed upstream
  blocker.

#### Minor

- 🔵 **Clarity**: "Resolved Q5" label is untraceable to a primary source
  within this item's own References
  **Location**: Context
  The resolution text is given inline and is clear on its own, but the "Q5"
  label implies a numbered question from an original source document that
  this item never names or links directly — only the abandoned 0173 traces
  back further.

- 🔵 **Scope**: Four subcommand groups bundled with asymmetric risk profiles
  **Location**: Requirements
  Technical Notes singles out `artifact-derive-metadata` as high fan-out and
  high-risk, while the other three subcommand groups appear narrower. Bundling
  a high-risk, high-fan-out piece with three lower-risk ones means the whole
  story's completion gates on the riskiest piece — a milder version of the
  concern that caused 0173 to be split.

- 🔵 **Testability**: "Per subcommand" is ambiguous against four listed
  clusters vs. five actual subcommands
  **Location**: Acceptance Criteria
  AC1's coverage floor is stated "per subcommand," but Requirements names five
  concrete subcommands while AC1 groups them into four categories ("ADR
  numbering/status" spans two) — it's unclear whether the floor applies per
  category or per individual subcommand.

- 🔵 **Testability**: AC2's skill call-site set is not enumerated
  **Location**: Acceptance Criteria
  AC2 requires "all skills previously invoking" the four named scripts to
  migrate, but no call-site list or discovery method is given, leaving the
  closed set to be independently rediscovered at verification time.

#### Suggestions

- 🔵 **Clarity**: "Typed-linkage" is used without linking to its defining ADR
  **Location**: References
  "Typed-linkage queries" is a core capability named in Summary, Requirements,
  and Acceptance Criteria, but ADR-0034 (its defining decision, per sibling
  0179) is absent from this item's ADR list.

- 🔵 **Completeness**: Context covers the split/grouping rationale but not the
  substantive case for migrating this cluster
  **Location**: Context
  The administrative history (why 0195 exists, why corpus ops are grouped) is
  clear, but the epic-level driver (bash 3.2 floor removal, cross-surface
  duplication) is not restated, so the item doesn't fully stand alone.

- 🔵 **Scope**: Total migration footprint may push this story toward
  epic-scale
  **Location**: Summary
  The combined footprint (four subcommand implementations, call-site rewrites
  across every consuming skill, removal, floor decrements, registration
  checklist) is substantial; if call-site counts turn out large, consider
  noting whether delivery will be phased across multiple PRs within the story.

- 🔵 **Clarity**: Ambiguous referent for "each" in the coverage clause
  **Location**: Acceptance Criteria
  AC1's "verified against repointed suites and characterization tests where
  none exist — each covering..." leaves unclear whether "each" binds to
  subcommand, test, or verification approach.

### Strengths

- ✅ The Dependencies section resolves every prior blocker individually with
  provenance (done status, what it supplies, PR numbers) rather than a bare
  list, and captures the one real downstream coupling (Blocks: 0174) with its
  specific causal mechanism.
- ✅ The five corpus subcommands are named identically and consistently across
  Summary, Requirements, and Acceptance Criteria, with no scope drift between
  sections.
- ✅ Acceptance Criteria are concrete and largely enumerable: a bounded minimum
  test-coverage floor (not unbounded exhaustiveness), grep-auditable script/
  skill references, and an anchor to a named, existing registration checklist.
- ✅ Context and Drafting Notes transparently carry forward the resolved Q5
  scope decision from the abandoned parent item, including its rationale —
  good scope hygiene showing the boundary was deliberated, not assumed.

### Recommended Changes

1. **Resolve the interface-redesign ambiguity for `artifact-derive-metadata`**
   (addresses: the merged clarity/completeness/testability finding). State
   explicitly in Requirements/Acceptance Criteria whether the output contract
   changes during this work or is preserved as-is; if it's preserved, replace
   or remove the Technical Notes phrase so it no longer conflicts with the
   "reproduces" framing.

2. **Define the equivalence bar for "reproduces"** (addresses: the
   "reproduces lacks a defined equivalence bar" finding). State the
   comparison technique — e.g. stdout/exit-code parity via characterization
   tests snapshotting the bash script's output as the golden baseline.

3. **Name how `accelerator-corpus` obtains the compiled work-item-id regex at
   runtime** (addresses: the dependency finding). Either state the dependency
   on 0170's DSL compiler explicitly, or confirm the regex is available
   another way (e.g. via the config crate) with no dependency on 0170.

4. **Disambiguate "per subcommand" in AC1** (addresses: the four-vs-five
   subcommand testability minor). Name the five subcommands explicitly, or
   state the coverage floor applies to each of the five listed in
   Requirements.

5. **Consider the asymmetric-risk bundling in Requirements** (addresses: the
   scope minor). If `artifact-derive-metadata`'s migration proves harder than
   expected during implementation, sequence it as a distinguishable sub-phase
   so the lower-risk subcommands aren't blocked by it.

6. **Tighten remaining minor/suggestion items**: add ADR-0034 to References;
   either drop the "Q5" label or cite its originating document; enumerate (or
   state the discovery method for) AC2's skill call-site set.

## Per-Lens Results

### Clarity

**Summary**: The work item is largely internally consistent — the five corpus
subcommands are named identically across Summary, Requirements, and
Acceptance Criteria, and the Dependencies section explicitly resolves each
prior blocker's status. However, a Technical Notes reference to an undefined
"interface-redesign rewrite" contradicts the parity language used everywhere
else ("reproduces"), and several cross-references (Q5, typed-linkage/
ADR-0034) require chasing an abandoned sibling work item to fully resolve.

**Strengths**:
- The five corpus subcommands are named identically and consistently across
  the Summary, Requirements, and Acceptance Criteria sections.
- The Dependencies section states the current resolution status of every
  prior blocker rather than leaving them as open questions.
- The Context section states the split rationale from 0173 and reproduces the
  "Resolved Q5" grouping decision inline.

**Findings**:
- 🔴 major/high — "Interface-redesign rewrite" is undefined and contradicts
  the parity framing used elsewhere (Technical Notes)
- 🔵 minor/medium — "Resolved Q5" label is untraceable to a primary source
  within this item's own References (Context)
- 🔵 suggestion/medium — "Typed-linkage" is used without linking to its
  defining ADR (References)
- 🔵 suggestion/low — Ambiguous referent for "each" in the coverage clause
  (Acceptance Criteria)

### Completeness

**Summary**: 0195 is a well-structured story: every expected section is
present and substantively populated, frontmatter is complete and internally
consistent, and the Acceptance Criteria and Dependencies sections are more
thorough than the abandoned parent item. The one notable gap is that a
requirement present in the predecessor item — an "interface-redesign"
principle for the artifact-metadata output contract — survives only as an
unexplained aside in Technical Notes.

**Strengths**:
- Acceptance Criteria are concrete and enumerable, an explicit improvement
  over the predecessor item per the Drafting Notes.
- Dependencies section resolves every prior blocker individually with status
  and rationale.
- Context clearly states the resolved grouping rationale (Q5) carried forward
  from the split.
- Frontmatter is complete and consistent with the body's header block.

**Findings**:
- 🔴 major/medium — Interface-redesign principle for artifact-metadata output
  is referenced but never specified as a requirement (Technical Notes)
- 🔵 suggestion/low — Context covers the split/grouping rationale but not the
  substantive case for migrating this cluster (Context)

### Dependency

**Summary**: The Dependencies section is unusually well-maintained: it
explicitly resolves each prior blocker with provenance and names the one real
downstream coupling (0174's floor-decrement lockstep) with its mechanism. The
main gap is a non-obvious upstream coupling inherited from the sibling
library item (0179): the runtime source of the compiled work-item-id scan
regex is never named.

**Strengths**:
- Each formerly-blocking work item is named individually with its resolution
  status and what it supplies.
- The single downstream coupling (Blocks: 0174) is captured with the specific
  causal mechanism.
- The skill call-site/allowed-tools rewrite is folded into this item's own
  scope rather than left as an implicit, uncaptured coupling.

**Findings**:
- 🟡 major/medium — Runtime source of the compiled work-item-id scan regex is
  not named as a dependency (Dependencies)

### Scope

**Summary**: This story is well-bounded relative to its declared purpose —
Summary, Requirements, Acceptance Criteria, Dependencies, and Assumptions all
consistently describe one binary built over one bounded context, and Context
documents a deliberate, resolved scope decision inherited from 0173. The four
subcommand groups share a plausible bounded-context rationale, but they carry
asymmetric risk profiles and a large total migration surface worth
re-checking against the same oversized-PR concern that triggered the original
split.

**Strengths**:
- Context and Drafting Notes explicitly carry forward a resolved scope
  decision (Q5) from the abandoned parent item, including its rationale.
- Summary, Requirements, and Acceptance Criteria are fully aligned around the
  same four subcommand areas with no drift between sections.
- Dependencies section cleanly delineates this item's boundary against
  sibling work items.

**Findings**:
- 🔵 minor/medium — Four subcommand groups bundled with asymmetric risk
  profiles (Requirements)
- 🔵 suggestion/low — Total migration footprint may push this story toward
  epic-scale (Summary)

### Testability

**Summary**: The acceptance criteria are largely verifiable: AC1 sets a
concrete minimum test-coverage floor, AC2/AC3 are grep-auditable, and AC4
anchors to a bounded, named external checklist. The main gaps are that
"reproduces" is never given a fidelity bar, and the "interface-redesign
rewrite" hint is never reconciled with AC1's parity-based framing, leaving the
pass/fail baseline for that subcommand ambiguous.

**Strengths**:
- AC1 defines a concrete minimum test-coverage floor rather than demanding
  unbounded or exhaustive coverage.
- AC2 and AC3 reference specific, named scripts and a specific migration
  contract, making them auditable by direct inspection.
- AC4 anchors verification to a concrete, existing checklist document.
- The Dependencies section grounds "blocked by: none" in concrete evidence.

**Findings**:
- 🔴 major/medium — "Reproduces" lacks a defined equivalence bar (Acceptance
  Criteria)
- 🔴 major/medium — Interface-redesign note is unreconciled with the
  "reproduces" framing (Technical Notes / Acceptance Criteria)
- 🔵 minor/medium — "Per subcommand" is ambiguous against the four listed
  clusters vs. five actual subcommands (Acceptance Criteria)
- 🔵 minor/low — AC2's skill call-site set is not enumerated (Acceptance
  Criteria)


## Re-Review (Pass 2) — 2026-08-06

**Verdict:** APPROVE

### Previously Identified Issues

- 🟡 **Clarity/Completeness/Testability**: Interface-redesign rewrite undefined
  — Resolved (Technical Notes now states the output contract is preserved
  as-is; the "interface-redesign" language only ever referred to the
  call-site rewrite)
- 🟡 **Testability**: "Reproduces" lacks a defined equivalence bar — Resolved
  (AC1 now specifies characterization tests snapshotting stdout, exit code,
  and stderr against the bash script's own output as the golden baseline)
- 🟡 **Dependency**: Runtime source of the compiled work-item-id regex not
  named — Resolved with a caveat. Direct source inspection
  (`cli/corpus-adapters/src/work_item_pattern.rs`) confirmed
  `compile_scan_regex` and `RegexScanner` already ship in `corpus-adapters`
  (0179), so 0195's Dependencies note is factually correct — but this
  surfaced that 0179's own document text still disclaims building the
  compiler (stale relative to what shipped). Left 0179's document untouched
  per reviewer decision; 0195's note stands as verified.
- 🔵 **Clarity**: "Resolved Q5" label untraceable — Resolved (dropped; inline
  resolution text stands alone)
- 🔵 **Scope**: Four subcommand groups bundled with asymmetric risk profiles
  — Still present (reviewer decision: accepted as intentional, no change)
- 🔵 **Testability**: "Per subcommand" ambiguous (4 categories vs. 5
  subcommands) — Resolved (Requirements/AC now use an explicit noun/verb
  structure naming all five subcommands: `adr next-number`, `adr read-status`,
  `metadata derive`, `frontmatter validate`, `linkage extract`)
- 🔵 **Testability**: AC2's skill call-site set not enumerated — Resolved
  (added an explicit discovery method, later tightened further in this pass)
- 🔵 **Clarity**: Typed-linkage missing ADR-0034 — Resolved (added ADR-0034
  and ADR-0038)
- 🔵 **Completeness**: Context missing epic-level motivation — Resolved
  (added bash-3.2-floor/ADR-0045 sentence)
- 🔵 **Scope**: Epic-scale footprint / phasing — Resolved (added phasing note,
  later extended with an explicit interim-state policy)
- 🔵 **Clarity**: Ambiguous "each" referent in AC1 — Resolved (superseded by
  the full AC1 rewrite)

### New Issues Introduced

This pass's edits (particularly the AC1 and AC2 rewrites) surfaced further
issues, all discussed and resolved in this same session:

- 🟡 **Clarity**: AC1's "none" referent and failure-path scope were ambiguous
  — Resolved (split into explicit repointed-suite vs. new-test clauses)
- 🟡 **Dependency**: AC2 (skills-only search) vs. AC3 (unconditional script
  removal) left non-skill callers unaddressed — Resolved (broadened to all
  callers, with an explicit exception carve-out)
- 🟡 **Testability**: The repo-wide search "closed set" would false-positive
  on documentation mentions of the script names — Resolved (scoped to
  executable invocations: `Bash(...)` calls, `allowed-tools` entries, shell
  `source`/exec sites)
- 🔵 **Clarity**: Summary said "typed-linkage queries", Requirements/subcommand
  name said "extraction"/`extract` — Resolved (aligned to "extraction")
- 🔵 **Completeness**: No beneficiary named — Resolved (added skill
  maintainers and the 0174 shell-retirement effort)
- 🔵 **Dependency**: Context's "migration framework" duplication claim implied
  an uncaptured 0172 coupling — Resolved (narrowed to bash library/visualiser,
  which is what this item actually touches)
- 🔵 **Scope**: Multi-PR phasing note didn't state the interim-state policy —
  Resolved (noun groups ship independently; mixed bash/Rust state mid-migration
  is acceptable)
- 🔵 **Testability**: "Golden baseline" was undefined — Resolved (defined
  inline as part of the AC1 rewrite)
- 🔵 **Clarity**: ADR-0053 has no gloss, unlike ADR-0034/ADR-0038 — Declined
  (reviewer decision: not worth the lookup)
- 🔵 **Testability**: "One failure path" doesn't name a specific condition per
  subcommand — Declined (reviewer decision: left to implementation)
- 🔵 **Scope**: No re-open trigger stated if call-site counts turn out large —
  Declined (reviewer decision: the phasing note already gives enough
  flexibility)

### Assessment

The work item is now ready for implementation. All major and critical-adjacent
findings across both review passes have been resolved through direct edits;
the handful of remaining minor/suggestion items were explicitly discussed and
accepted as intentional trade-offs rather than left as oversights. The one
open thread — 0179's document text disclaiming a compiler that its own shipped
code contains — was investigated, confirmed via direct source inspection not
to affect 0195's correctness, and deliberately left unaddressed in 0179 by
reviewer decision.

---
*Review generated by /accelerator:review-work-item*
