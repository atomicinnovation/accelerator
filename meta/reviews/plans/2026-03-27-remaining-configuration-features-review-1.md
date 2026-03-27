---
date: "2026-03-28T00:30:00+0000"
type: plan-review
skill: review-plan
target: "meta/plans/2026-03-27-remaining-configuration-features.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, code-quality, correctness, compatibility, test-coverage, usability]
review_pass: 2
status: complete
---

## Plan Review: Remaining Configuration Features

**Verdict:** REVISE

The plan is well-structured with clear phase boundaries, thorough verification
criteria, and sound architectural reasoning. It correctly prioritises
deterministic agent resolution as the highest-value change and extends existing
patterns rather than inventing new mechanisms. However, three issues warrant
revision before implementation: (1) the plan claims "deterministic" resolution
but the labeled-variable approach still relies on LLM interpretation — and
removing `config-read-agent-name.sh` actually regresses the two skills that
currently have truly deterministic resolution; (2) `config-read-template.sh`
has a hardcoded error message that will reject the new `pr-description` key;
and (3) the plan does not specify updates to `test-config.sh` which will break
after the script output format changes.

### Cross-Cutting Themes

- **LLM-interpreted variable references are more reliable but not deterministic**
  (flagged by: Architecture, Code Quality, Correctness, Usability) — The plan's
  central mechanism (`{agent-name agent}` variable references) is a significant
  improvement over the override table, but multiple lenses note it is not truly
  deterministic. Removing the inline `config-read-agent-name.sh` calls from
  `review-pr` and `review-plan` actually downgrades those skills from
  deterministic to LLM-interpreted resolution.

- **Test suite updates are missing from the plan** (flagged by: Compatibility,
  Test Coverage) — `test-config.sh` has assertions on the output format of
  `config-read-agents.sh`, the existence of `config-read-agent-name.sh`, agent
  skill counts, and the empty-output behaviour of `config-read-review.sh`. All
  of these will break after Phases 1 and 3, but the plan does not specify
  replacement test cases.

- **Artifact target field replacement is vague** (flagged by: Architecture,
  Code Quality, Correctness, Usability) — The proposed change from
  `target: "meta/plans/{plan-stem}.md"` to
  `target: "{the actual plan path provided by the user}"` is a natural language
  instruction, not a variable reference. This contradicts the plan's
  deterministic-resolution goal and is inconsistent with the `{variable name}`
  convention used everywhere else.

- **Dual review output sections create redundancy** (flagged by: Code Quality,
  Compatibility) — Adding a "Review Settings" section alongside the existing
  "Review Configuration" section produces overlapping information with
  inconsistent formatting.

### Tradeoff Analysis

- **Determinism vs Latency**: The plan chose labeled variables over per-agent
  `config-read-agent-name.sh` calls to avoid latency. This is the right
  trade-off for prose references, but for the `subagent_type` parameter (where
  correctness is critical), retaining the inline preprocessor call is
  worth the ~20ms cost.

### Findings

#### Critical

- 🔴 **Correctness**: config-read-template.sh will reject 'pr-description' as
  an unknown template name
  **Location**: Phase 4, Section 3
  The script's error message at line 85 hardcodes the available template list
  as `plan, research, adr, validation`. While the script doesn't validate
  against a whitelist, the error message is misleading and needs updating. The
  plan says "no changes expected" which is incorrect.

- 🔴 **Compatibility**: config-read-template.sh error message rejects
  'pr-description' key
  **Location**: Phase 4, Section 3
  Same issue from the compatibility perspective — the hardcoded error list
  should either include `pr-description` or be generated dynamically from the
  `templates/` directory.

#### Major

- 🟡 **Architecture**: Removing config-read-agent-name.sh eliminates the only
  truly deterministic agent resolution path
  **Location**: Phase 1, Section 4
  `review-pr` line 279 and `review-plan` line 248 currently resolve the
  reviewer name at preprocessor time. Replacing them with `{reviewer agent}`
  variable references is a regression in resolution reliability.

- 🟡 **Architecture**: Agent name resolution shifts from one non-deterministic
  mechanism to another
  **Location**: Phase 1
  The labeled-variable approach is more reliable than the override table but
  is not truly "deterministic" as the plan claims. The plan should acknowledge
  this distinction.

- 🟡 **Code Quality**: Always-emit pattern changes script's contract without
  updating all consumers
  **Location**: Phase 1, Section 1
  The plan does not verify whether `test-config.sh` or other infrastructure
  depends on the empty-output behavior of `config-read-agents.sh`.

- 🟡 **Code Quality**: Dual output sections create confusing redundancy
  **Location**: Phase 3, Section 1
  "Review Settings" (always emitted, labeled variables) alongside "Review
  Configuration" (conditionally emitted, override info) produces overlapping
  information with inconsistent formatting.

- 🟡 **Correctness**: Vague replacement for artifact target fields loses
  deterministic path construction
  **Location**: Phase 2, Section 5
  `target: "{the actual plan path provided by the user}"` is a prose
  instruction, not a variable reference. Use `{plans directory}/{plan-stem}.md`
  instead.

- 🟡 **Compatibility**: Deleting config-read-agent-name.sh breaks existing
  test assertions
  **Location**: Phase 1, Section 4
  `test-config.sh` lines 856-910 test this script; lines 1012-1020 assert it
  exists in `review-pr` and `review-plan`. These tests will fail.

- 🟡 **Compatibility**: Changing config-read-agents.sh output format breaks
  existing test expectations
  **Location**: Phase 1, Section 1
  Tests assert the old table format and empty-output behavior. The plan must
  include test updates.

- 🟡 **Compatibility**: Removing config-read-agent-name.sh is a breaking
  change for external callers
  **Location**: Phase 1, Section 4
  Users with custom skills referencing this script would get hard failures
  with no migration path.

- 🟡 **Test Coverage**: No new automated tests for changed
  config-read-agents.sh output format
  **Location**: Phase 1, Success Criteria
  The new always-emit labeled format needs test cases for: no config, partial
  override, and local-overrides-team precedence.

- 🟡 **Test Coverage**: No new automated tests for always-emitted Review
  Settings section
  **Location**: Phase 3, Success Criteria
  The min_lenses=4 fix and mode-specific output need regression tests.

- 🟡 **Test Coverage**: Skill integration tests need updating for new agent
  count expectations
  **Location**: Phase 1, Success Criteria
  Count assertion changes from 8 to 10; exclusion lists need updating.

- 🟡 **Usability**: Inconsistent variable naming convention between agent and
  path references
  **Location**: Phase 1
  Agent variables use `{hyphenated-name agent}` while path labels use
  `{Title Case directory}`. Consider aligning the convention.

- 🟡 **Usability**: No error feedback when numeric config values are invalid
  after variable injection
  **Location**: Phase 3
  Confirm that the always-emitted values go through validation first.

#### Minor

- 🔵 **Architecture**: Artifact template target fields use ambiguous dynamic
  references
  **Location**: Phase 2, Section 5

- 🔵 **Architecture**: config-read-review.sh output grows unconditionally,
  increasing context consumption
  **Location**: Phase 3

- 🔵 **Code Quality**: Vague target field replacement undermines deterministic
  resolution
  **Location**: Phase 2, Section 5

- 🔵 **Code Quality**: Generic argument hints reduce discoverability
  **Location**: Phase 5, Section 5

- 🔵 **Code Quality**: Deletion marked as optional may leave dead code
  **Location**: Phase 1, Section 4

- 🔵 **Correctness**: New 'Review Settings' section placement before
  has_config check may produce partial output on script error
  **Location**: Phase 3, Section 1

- 🔵 **Correctness**: Argument hints changed to generic paths lose
  discoverability
  **Location**: Phase 5, Section 5

- 🔵 **Compatibility**: Test count assertion for config-read-agents.sh will
  need updating
  **Location**: Phase 1, Section 3

- 🔵 **Compatibility**: New 'Review Settings' output section may conflict with
  existing 'Review Configuration' heading
  **Location**: Phase 3, Section 1

- 🔵 **Test Coverage**: No automated test for pr-description template
  resolution chain
  **Location**: Phase 4, Success Criteria

- 🔵 **Test Coverage**: No automated test for respond-to-pr path variable
  injection
  **Location**: Phase 2, Success Criteria

- 🔵 **Usability**: Deleting config-read-agent-name.sh without deprecation
  path
  **Location**: Phase 1, Section 4

- 🔵 **Usability**: Vague artifact target replacement
  **Location**: Phase 2, Section 5

#### Suggestions

- 🔵 **Architecture**: Growing number of preprocessor script calls suggests
  future consolidation opportunity
  **Location**: Implementation Approach

- 🔵 **Architecture**: Argument-hint frontmatter cannot use dynamic variables
  **Location**: Phase 5, Section 5

- 🔵 **Code Quality**: Consider documenting the variable reference convention
  centrally
  **Location**: Phase 1

- 🔵 **Test Coverage**: Manual testing steps could benefit from a lightweight
  smoke test script
  **Location**: Testing Strategy

- 🔵 **Usability**: Default PR description template could be more opinionated
  **Location**: Phase 4

- 🔵 **Usability**: Generic argument-hints lose discoverability
  **Location**: Phase 5, Section 5

### Strengths

- ✅ Correctly prioritises deterministic agent resolution as the highest-value
  change, directly addressing the reliability gap from the original research
- ✅ Each phase is independently testable with clear automated and manual
  verification criteria
- ✅ Extends existing patterns (labeled variables, config-read-*.sh scripts)
  rather than inventing new mechanisms
- ✅ Explicit "What We're NOT Doing" section with justified scope boundaries
- ✅ Performance impact analysis is thorough with per-phase latency accounting
- ✅ Fixes the factually wrong "6 to 8" default (actual: 4 to 8) as part of
  the variable injection work
- ✅ Preserves existing config key format — no user configuration files need
  to change
- ✅ The describe-pr template alignment adds a sensible plugin default,
  removing "create a template first" friction

### Recommended Changes

1. **Retain `config-read-agent-name.sh` for `subagent_type` parameters**
   (addresses: Architecture — removing deterministic path, Compatibility —
   breaking change)
   Keep the inline preprocessor calls in `review-pr` line 279 and
   `review-plan` line 248. Use the bulk labeled output from
   `config-read-agents.sh` for all other prose references. This gives the best
   of both approaches: bulk output for prose (no latency increase), inline
   resolution for the critical spawn parameter (deterministic). Optionally keep
   the script for external consumers.

2. **Add test-config.sh updates to Phases 1, 3, and 4** (addresses: all
   Compatibility and Test Coverage findings)
   Each phase that modifies script output must include corresponding test
   updates: new format assertions for `config-read-agents.sh`, new always-emit
   assertions for `config-read-review.sh`, updated skill counts (8→10),
   updated exclusion lists, a `pr-description` template resolution test, and
   a `respond-to-pr` path injection structural test.

3. **Update config-read-template.sh error message** (addresses: Correctness
   and Compatibility — template rejection)
   Either add `pr-description` to the hardcoded list on line 85, or better,
   dynamically generate the list from `ls templates/*.md`.

4. **Use `{plans directory}/{plan-stem}.md` for artifact target fields**
   (addresses: Architecture, Code Quality, Correctness, Usability — vague
   target replacement)
   Replace the prose instruction `{the actual plan path provided by the user}`
   with the concrete variable pattern `{plans directory}/{plan-stem}.md` in
   both `validate-plan` and `review-plan`.

5. **Consolidate Review Settings and Review Configuration into one section**
   (addresses: Code Quality and Compatibility — dual output redundancy)
   Emit a single always-present section with all labeled variable definitions,
   annotating which values differ from defaults. Remove the separate
   conditional section.

6. **Align agent variable naming convention with path convention** (addresses:
   Usability — naming inconsistency, Correctness — reference mismatch risk)
   Use spaces throughout: `**codebase locator agent**: codebase-locator`
   referenced as `{codebase locator agent}`. Add a verification grep for
   bare agent names without the ` agent` suffix.

7. **Use descriptive argument-hint placeholders** (addresses: Code Quality and
   Usability — discoverability)
   Instead of `@path/to/ADR-NNNN.md`, use `@<decisions-dir>/ADR-NNNN.md` to
   hint at the concept without hardcoding the path.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is well-structured and architecturally consistent with
the existing preprocessor-based configuration system. It correctly identifies
the non-deterministic agent resolution as the highest-priority gap and proposes
a uniform labeled-variable pattern that eliminates the fragile override-table
approach. Two architectural concerns merit attention: the plan introduces a
second LLM-interpretation layer that, while more reliable than the override
table, still lacks full determinism, and the growing number of preprocessor
script calls per skill suggests future consolidation.

**Strengths**:
- Correctly prioritises deterministic agent resolution as highest-value change
- Each phase independently testable with clear verification criteria
- Maintains architectural consistency by extending existing patterns
- Explicit scope boundaries in "What We're NOT Doing"
- Thorough performance impact analysis

**Findings**:
- 🟡 major/high — Agent name resolution shifts from one non-deterministic
  mechanism to another (Phase 1)
- 🟡 major/medium — Removing config-read-agent-name.sh eliminates the only
  truly deterministic resolution path (Phase 1, Section 4)
- 🔵 minor/medium — Artifact template target fields use ambiguous dynamic
  references (Phase 2, Section 5)
- 🔵 minor/medium — config-read-review.sh output grows unconditionally (Phase 3)
- 🔵 suggestion/medium — Growing preprocessor calls suggest future
  consolidation (Implementation Approach)
- 🔵 suggestion/low — Argument-hint frontmatter cannot use dynamic variables
  (Phase 5, Section 5)

### Code Quality

**Summary**: The plan follows established patterns consistently with clear
phase boundaries and strong verification criteria. Primary concerns are around
the variable reference syntax, dual output sections in config-read-review.sh,
and a few areas where prescribed changes could introduce inconsistencies.

**Strengths**:
- Consistent application of existing patterns (DRY/KISS)
- Each phase independently testable
- Correctly identifies override table as a code smell
- Performance impact explicitly quantified

**Findings**:
- 🟡 major/medium — Always-emit pattern changes script's contract without
  updating all consumers (Phase 1)
- 🟡 major/medium — Dual output sections create confusing redundancy (Phase 3)
- 🔵 minor/high — Vague target field replacement undermines deterministic
  resolution (Phase 2, Section 5)
- 🔵 minor/high — Generic argument hints reduce discoverability (Phase 5)
- 🔵 minor/medium — Deletion marked as optional may leave dead code (Phase 1)
- 🔵 suggestion/medium — Consider documenting variable reference convention
  centrally (Phase 1)

### Correctness

**Summary**: The plan is logically well-structured with clear phase
dependencies. Primary correctness concerns involve the hardcoded error message
in config-read-template.sh, variable naming convention risks, and the vague
artifact target field replacement.

**Strengths**:
- Correctly identifies early-exit as root cause and replaces with always-emit
- Phase dependencies well-ordered
- Correctly identifies "6 to 8" as a bug and fixes via variables
- Verification criteria are concrete and grep-based

**Findings**:
- 🔴 critical/high — config-read-template.sh will reject 'pr-description'
  (Phase 4, Section 3)
- 🟡 major/high — review-plan mkdir uses hardcoded path while ls uses dynamic
  variable (Phase 2, Section 4) — confirmed as real bug the plan correctly
  catches
- 🟡 major/medium — Agent variable naming convention differs from established
  pattern (Phase 1)
- 🟡 major/medium — Vague replacement for artifact target fields (Phase 2,
  Section 5)
- 🔵 minor/high — Review Settings placement may produce partial output on
  error (Phase 3)
- 🔵 minor/medium — Argument hints lose discoverability (Phase 5)

### Compatibility

**Summary**: The plan is largely additive and internal. Main compatibility
concerns are the template error message, test suite breakage from output format
changes, and deletion of config-read-agent-name.sh as a breaking change.

**Strengths**:
- Preserves existing config key format and YAML structure
- Extends established preprocessor variable pattern
- Phase ordering well-considered
- Corrects "6 to 8" vs "4 to 8" discrepancy

**Findings**:
- 🔴 critical/high — config-read-template.sh error message rejects
  'pr-description' (Phase 4, Section 3)
- 🟡 major/high — Deleting config-read-agent-name.sh breaks test assertions
  (Phase 1, Section 4)
- 🟡 major/high — Changing config-read-agents.sh output format breaks test
  expectations (Phase 1, Section 1)
- 🟡 major/medium — Removing config-read-agent-name.sh is breaking change for
  external callers (Phase 1, Section 4)
- 🔵 minor/high — Test count assertion needs updating (Phase 1, Section 3)
- 🔵 minor/medium — Review Settings/Configuration heading confusion (Phase 3)

### Test Coverage

**Summary**: The plan relies heavily on `test-config.sh` but does not specify
new test cases for significant behavioural changes. Automated verification
criteria are mostly grep-based structural checks rather than behavioural tests.

**Strengths**:
- Each phase has explicit automated and manual verification criteria
- Consistently references test-config.sh as a gate
- Grep-based negative checks are pragmatic
- Manual steps cover full config precedence chain

**Findings**:
- 🟡 major/high — No new automated tests for config-read-agents.sh output
  format (Phase 1)
- 🟡 major/high — No new automated tests for Review Settings section (Phase 3)
- 🟡 major/high — Skill integration tests need count updates (Phase 1)
- 🔵 minor/medium — No automated test for pr-description template resolution
  (Phase 4)
- 🔵 minor/medium — No automated test for respond-to-pr path injection
  (Phase 2)
- 🔵 suggestion/medium — Manual tests could benefit from smoke test script
  (Testing Strategy)

### Usability

**Summary**: The plan systematically addresses configuration consistency gaps
that directly impact developer experience. Primary concerns are variable naming
convention inconsistency and error feedback gaps.

**Strengths**:
- Eliminates fragile override table pattern — significant DX improvement
- Fixes factually wrong "6 to 8" default
- describe-pr template alignment removes friction for new users
- Performance analysis demonstrates net-neutral latency

**Findings**:
- 🟡 major/high — Inconsistent variable naming convention (Phase 1)
- 🟡 major/medium — No error feedback for invalid numeric config after
  injection (Phase 3)
- 🔵 minor/medium — Deleting config-read-agent-name.sh without deprecation
  (Phase 1)
- 🔵 minor/medium — Vague artifact target replacement (Phase 2)
- 🔵 suggestion/medium — Default PR template could be more opinionated
  (Phase 4)
- 🔵 suggestion/low — Generic argument-hints lose discoverability (Phase 5)

## Re-Review (Pass 2) — 2026-03-28

**Verdict:** COMMENT

### Previously Identified Issues

- 🔴 **Correctness**: config-read-template.sh rejects 'pr-description' — **Resolved** (dynamic error message generation from templates/ directory)
- 🔴 **Compatibility**: config-read-template.sh error message — **Resolved** (same fix)
- 🟡 **Architecture**: Removing config-read-agent-name.sh eliminates deterministic path — **Resolved** (script retained for subagent_type)
- 🟡 **Architecture**: Agent name resolution not truly deterministic — **Resolved** (plan renamed to "Reliable", inline calls retained for critical spawn points)
- 🟡 **Code Quality**: Always-emit changes contract without updating consumers — **Resolved** (test-config.sh updates specified per phase)
- 🟡 **Code Quality**: Dual output sections redundancy — **Resolved** (consolidated into single "Review Configuration" with _emit_value helper)
- 🟡 **Correctness**: Vague artifact target fields — **Resolved** (uses {plans directory}/{plan-stem}.md)
- 🟡 **Compatibility**: Deleting config-read-agent-name.sh breaks tests — **Resolved** (script retained)
- 🟡 **Compatibility**: Changing agents output breaks tests — **Resolved** (test updates specified)
- 🟡 **Compatibility**: Breaking change for external callers — **Resolved** (script retained)
- 🟡 **Test Coverage**: No tests for agents output format — **Resolved** (Phase 1 Step 5)
- 🟡 **Test Coverage**: No tests for Review Settings — **Resolved** (Phase 3 test updates)
- 🟡 **Test Coverage**: Skill count assertions — **Resolved** (8→10 explicitly addressed)
- 🟡 **Usability**: Inconsistent variable naming — **Resolved** (spaces convention throughout)
- 🟡 **Usability**: No error feedback for invalid values — **Resolved** (output placed after validation)
- 🔵 **Code Quality**: Vague target field — **Resolved**
- 🔵 **Code Quality**: Generic argument hints — **Resolved** (@\<decisions-dir\> placeholders)
- 🔵 **Code Quality**: Deletion marked optional — **Resolved** (retention, not deletion)
- 🔵 **Correctness**: Review Settings placement — **Resolved** (explicit placement after validation)
- 🔵 **Correctness**: Argument hints discoverability — **Resolved**
- 🔵 **Compatibility**: Test count assertion — **Resolved**
- 🔵 **Compatibility**: Dual heading conflict — **Resolved** (consolidated)
- 🔵 **Test Coverage**: No pr-description template test — **Resolved** (Phase 4)
- 🔵 **Test Coverage**: No respond-to-pr path test — **Resolved** (Phase 2)
- 🔵 **Usability**: Deletion without deprecation — **Resolved** (retained)
- 🔵 **Usability**: Vague artifact target — **Resolved**

### New Issues Introduced

- 🟡 **Correctness**: Review Configuration restructuring may silently drop
  core_lenses, disabled_lenses, and verdict override output logic. The plan's
  replacement code only shows _emit_value calls for numeric/severity values but
  does not specify where the existing core_lenses/disabled_lenses/verdict
  display logic (lines 300-340 of config-read-review.sh) should go in the
  restructured output.

- 🔵 **Correctness**: Plan emits `pr_request_changes_severity` as a labeled
  variable but no prose in review-pr references it as `{pr request changes
  severity}`. Harmless but inconsistent with the stated goal.

- 🔵 **Correctness**: Removing the `has_config` early exit means the Lens
  Catalogue section will also always be emitted even with no custom lenses.
  Confirm this is intentional.

### Suggestions (non-blocking)

- 🔵 **Architecture**: Growing preprocessor calls suggest future consolidation
  (acknowledged trade-off, no action needed)
- 🔵 **Code Quality**: Variable reference convention still undocumented
  centrally — consider adding a convention note to configure skill docs
- 🔵 **Test Coverage**: No smoke test script for manual steps (acceptable,
  defer to future); add explicit "partial override shows all 7 agents" test
  assertion
- 🔵 **Usability**: Default PR template could include a customisation hint
  comment
- 🔵 **Compatibility**: Argument hint text changes are user-visible but
  positive

### Assessment

The plan has been substantially improved. All 14 major findings and both
critical findings from the initial review are fully resolved. The plan now
correctly retains `config-read-agent-name.sh` for deterministic resolution at
critical spawn points, specifies test updates per phase, uses concrete variable
patterns for artifact targets, consolidates review output into a single section,
and aligns the variable naming convention.

One new major finding emerged: the Phase 3 restructuring of
`config-read-review.sh` should explicitly specify where the existing
core_lenses, disabled_lenses, and verdict override display logic goes in the
new output structure. This is a gap in the plan's specification rather than a
design flaw — the logic should be preserved after the _emit_value block within
the same consolidated section.

**Verdict: COMMENT** — Plan is acceptable and could be improved by addressing
the core_lenses/disabled_lenses output gap. All other findings are suggestions.
