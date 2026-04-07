---
date: "2026-04-07T13:30:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-07-add-accelerator-prefix-to-default-agent-names.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, code-quality, test-coverage, correctness, compatibility]
review_pass: 2
status: complete
---

## Plan Review: Add `accelerator:` Prefix to Default Agent Names

**Verdict:** REVISE

The plan is well-researched with precise file paths and line numbers verified
against the actual codebase. The phased approach (scripts, skills, tests) is
sound and the boundary analysis (what to change vs. what to leave alone) is
thorough. However, the plan has an incomplete inventory of test assertions
requiring update -- two additional test locations will fail after Phase 1
changes, directly contradicting the stated success criterion of zero test
failures. Additionally, the config-dump.sh defaults change has no test
coverage at all.

### Cross-Cutting Themes

- **Incomplete test assertion inventory** (flagged by: Architecture,
  Correctness, Test Coverage) -- Lines 799 and 941 of `test-config.sh` both
  assert bare default agent names but are not listed in Phase 3. This is the
  most impactful gap: the plan will not achieve its own success criteria as
  written.
- **Hardcoded prefix duplication** (flagged by: Architecture, Code Quality,
  Compatibility) -- The string `accelerator:` will appear as a literal in
  ~20 locations across 14 files. All three lenses independently suggest
  extracting a shared constant in `config-common.sh` for the script layer.

### Findings

#### Critical

- 🔴 **Correctness / Test Coverage**: Missing test update at line 799
  (partial-override default check)
  **Location**: Phase 3: Test Assertions
  When one agent is overridden, the test checks that other agents show
  defaults. Line 799 asserts `codebase-locator` (bare) which will become
  `accelerator:codebase-locator` after Phase 1.

- 🔴 **Correctness / Test Coverage**: Missing test update at line 941
  (no-agents-section default check)
  **Location**: Phase 3: Test Assertions
  When config has no `agents` section, the test checks default output. Line
  941 asserts `reviewer` (bare) which will become `accelerator:reviewer`
  after Phase 1.

#### Major

- 🟡 **Test Coverage**: No test verifies config-dump.sh AGENT_DEFAULTS values
  **Location**: Phase 3: Test Assertions / config-dump.sh
  Phase 1 modifies `AGENT_DEFAULTS` in config-dump.sh but no existing test
  asserts specific agent default values. A regression (typo, missed entry)
  would go undetected.

#### Minor

- 🔵 **Architecture / Code Quality / Compatibility**: Hardcoded prefix
  duplicated across 14 files
  **Location**: Phase 1: Script Defaults
  The `accelerator:` literal will appear in 3 scripts, 10 skill files, and
  test assertions. Extracting `AGENT_PREFIX="accelerator:"` in
  `config-common.sh` would reduce script-layer duplication.

- 🔵 **Compatibility**: Default value change warrants a CHANGELOG entry
  **Location**: Overview / Implementation Approach
  This changes observable output for all users with no overrides. A patch
  version bump and CHANGELOG entry would help users track when defaults
  changed.

- 🔵 **Code Quality**: Phase 3 changes 3 and 4 show identical code blocks
  **Location**: Phase 3: Test Assertions
  Both show the same command/assertion, differing only in test context (line
  990 has a codebase-locator override fixture). A brief distinguishing note
  would prevent implementer confusion.

- 🔵 **Test Coverage**: Identity-override test (line 894) becomes
  semantically ambiguous
  **Location**: Phase 3: Test Assertions
  After the change, a user setting `reviewer: reviewer` explicitly chooses
  the bare name over the prefixed default. The test still passes but its
  intent as an override-passthrough test is undocumented.

#### Suggestions

- 🔵 **Architecture**: Skill fallback lines are a redundant resolution path
  **Location**: Current State Analysis
  If the preprocessor always runs, the 10 fallback lines are dead code.
  Eliminating them in a follow-up would remove a whole class of sync issues.

- 🔵 **Test Coverage**: No automated test for skill fallback line content
  **Location**: Testing Strategy
  Phase 2 changes rely on manual grep verification. A structural test
  asserting 10 matches of the updated fallback pattern would strengthen
  coverage.

### Strengths

- ✅ Clean separation of resolution paths: fixing the two script paths
  propagates automatically to template-style references, avoiding
  unnecessary changes
- ✅ Well-ordered phasing enables incremental verification (scripts, skills,
  tests)
- ✅ Thorough "What We're NOT Doing" section demonstrates careful boundary
  analysis
- ✅ User-provided overrides correctly left untouched, preserving the
  pass-through contract
- ✅ All 10 skill file locations and line numbers verified as accurate
- ✅ Correctly identifies that config-read-value.sh test (line 273-274)
  needs no change

### Recommended Changes

1. **Add line 799 to Phase 3** (addresses: missing test at line 799)
   Update the assertion to expect `accelerator:codebase-locator` instead of
   bare `codebase-locator`.

2. **Add line 941 to Phase 3** (addresses: missing test at line 941)
   Update the assertion to expect `accelerator:reviewer` instead of bare
   `reviewer`.

3. **Add a config-dump.sh default values test** (addresses: untested
   AGENT_DEFAULTS)
   Add a test that runs config-dump.sh with minimal config and verifies at
   least one agent key shows the `accelerator:` prefix as its default value.

4. **Add a distinguishing note for Phase 3 changes 3 and 4** (addresses:
   identical code blocks)
   Note that change 3 tests with no config and change 4 tests with a
   codebase-locator override fixture.

5. **Consider extracting `AGENT_PREFIX` in config-common.sh** (addresses:
   hardcoded prefix duplication)
   Define `AGENT_PREFIX="accelerator:"` and reference it from the three
   scripts to reduce script-layer scatter.

6. **Add a CHANGELOG entry** (addresses: version tracking)
   Document the default agent name change under a new patch or minor
   version.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is a straightforward, well-scoped mechanical change.
The structural approach is sound -- fixing resolution at the source so
downstream template references automatically pick up correct values. However,
the incomplete test inventory will cause failures.

**Strengths**:
- Clean separation of resolution paths avoids unnecessary changes
- Phasing is well-ordered for incremental validation
- "What We're NOT Doing" section demonstrates thorough analysis
- User-provided overrides correctly left untouched

**Findings**:

- **Critical** (high confidence): Incomplete test update inventory -- at
  least 2 additional assertions will fail
  - Location: Phase 3: Test Assertions
  - Lines 799 and 941 assert bare default names but are not listed for
    update. Line 799 checks `codebase-locator` for a non-overridden agent;
    line 941 checks `reviewer` with no agents section.

- **Minor** (medium confidence): Hardcoded prefix string duplicated across
  3 scripts and 10 skill files
  - Location: Phase 1: Script Defaults
  - ~13 locations will contain the literal `accelerator:`. A comment or
    shared variable in config-common.sh would ease future maintenance.

- **Suggestion** (medium confidence): Skill fallback lines are a redundant
  resolution path that could be eliminated
  - Location: Current State Analysis
  - If the preprocessor always runs, fallback lines are dead code. Consider
    removing in a follow-up.

### Code Quality

**Summary**: The changes are simple string replacements with minimal risk.
The main concern is hardcoding the prefix across 20+ locations without
extracting a shared constant.

**Strengths**:
- Exceptionally well-researched with precise file paths and line numbers
- Clear phase ordering enables incremental verification
- Explicit "What We're NOT Doing" prevents scope creep
- Override pass-through correctly preserved

**Findings**:

- **Minor** (high confidence): Hardcoded prefix repeated across multiple
  files without shared constant
  - Location: Phase 1: Script Defaults
  - `config-common.sh` already exists as a shared constants file. Defining
    `AGENT_PREFIX="accelerator:"` there would reduce scatter.

- **Suggestion** (medium confidence): Tests 3 and 4 describe different
  scenarios but show identical code blocks
  - Location: Phase 3: Test Assertions
  - The test context differs (line 990 has a codebase-locator override
    fixture) but the plan's code snippets are identical, risking implementer
    confusion.

### Test Coverage

**Summary**: Phase 3 identifies only 4 test assertion groups but misses at
least 2 additional tests that also assert bare defaults. The config-dump.sh
AGENT_DEFAULTS change lacks any test verification.

**Strengths**:
- Correctly identifies config-read-value.sh test needs no change
- Comprehensive existing test suite covers override precedence well
- Testing Strategy includes both automated and manual verification

**Findings**:

- **Critical** (high confidence): Two additional tests assert bare default
  names but are not listed for update
  - Location: Phase 3: Test Assertions
  - Line 799: partial-override test, `codebase-locator` (bare). Line 941:
    no-agents-section test, `reviewer` (bare). Both will fail.

- **Major** (high confidence): No test verifies config-dump.sh
  AGENT_DEFAULTS values
  - Location: Phase 3: Test Assertions / config-dump.sh
  - The AGENT_DEFAULTS array is modified but never tested. A regression
    would go undetected.

- **Minor** (medium confidence): Identity override test becomes semantically
  ambiguous after prefix change
  - Location: Phase 3: Test Assertions, line 894
  - Setting `reviewer: reviewer` now implicitly tests that overrides are
    not auto-prefixed, but the intent isn't documented.

- **Suggestion** (medium confidence): No automated test for skill fallback
  line content
  - Location: Testing Strategy
  - Phase 2 relies on manual grep verification. A structural test would
    strengthen coverage.

### Correctness

**Summary**: The plan is largely correct in identifying the three scripts,
ten skill files, and test assertions that need updating. The logic of
prefixing defaults while passing through overrides is sound. However, two
test assertions are missed.

**Strengths**:
- Correctly identifies agent definition name: fields should not change
- Override pass-through is correctly preserved
- config-read-value.sh test correctly excluded
- Identity-override test at line 894 correctly left unchanged
- All 10 skill file locations verified accurate

**Findings**:

- **Critical** (high confidence): Missing test update at line 799
  (override-one-agent default check)
  - Location: Phase 3: Test Assertions
  - Asserts `codebase-locator` (bare) for a non-overridden agent. Will fail
    after Phase 1.

- **Critical** (high confidence): Missing test update at line 941
  (no-agents-section default check)
  - Location: Phase 3: Test Assertions
  - Asserts `reviewer` (bare) with no agents section. Will fail after
    Phase 1.

### Compatibility

**Summary**: The plan changes default output values, altering the observable
contract for consumers. User overrides are correctly preserved. The plan
omits versioning considerations.

**Strengths**:
- Override pass-through preserved, preventing double-prefixing
- Manual verification of override behaviour included
- Agent definition name: fields and template references correctly excluded
- Test assertions updated in lockstep with implementation

**Findings**:

- **Minor** (high confidence): config-dump.sh agent default assertions
  missing from test updates
  - Location: Phase 3: Test Assertions
  - No test verifies the specific default values output by config-dump for
    agent keys.

- **Minor** (medium confidence): Default value change warrants a version
  bump in CHANGELOG
  - Location: Overview / Implementation Approach
  - The project follows semver-style versioning. A CHANGELOG entry would
    help users track the change.

- **Suggestion** (medium confidence): Hardcoded prefix creates coupling to
  plugin name
  - Location: Phase 1, Change 1
  - Consider extracting to a single variable in config-common.sh.

## Re-Review (Pass 2) — 2026-04-07

**Verdict:** COMMENT

### Previously Identified Issues

- 🔴 **Correctness / Test Coverage**: Missing test update at line 799 — **Resolved** (now covered in Phase 3, section 2)
- 🔴 **Correctness / Test Coverage**: Missing test update at line 941 — **Resolved** (now covered in Phase 3, section 3)
- 🟡 **Test Coverage**: No test for config-dump.sh AGENT_DEFAULTS — **Resolved** (new test added in Phase 3, section 7)
- 🔵 **Code Quality**: Phase 3 changes 3/4 show identical code blocks — **Resolved** (sections 5/6 now have distinguishing headers and context)
- 🔵 **Architecture / Code Quality / Compatibility**: Hardcoded prefix duplication — **Still present** (acknowledged as intentional trade-off)
- 🔵 **Compatibility**: CHANGELOG entry missing — **Still present** (deferred to implementation time)
- 🔵 **Test Coverage**: Identity-override test ambiguity — **Still present** (minor, not addressed)
- 🔵 **Architecture**: Skill fallback redundancy — **Still present** (acknowledged as follow-up)
- 🔵 **Test Coverage**: No automated skill fallback test — **Still present** (acknowledged as acceptable gap)

### New Issues Introduced

- 🟡 **Test Coverage**: New config-dump test (section 7) uses `$DUMP` but the test file defines the variable as `$CONFIG_DUMP` (line 13). All existing config-dump tests use `bash "$CONFIG_DUMP"`. The test would fail at runtime.
- 🔵 **Code Quality / Test Coverage**: New config-dump test says "around line 720" but the config-dump test section starts at line 1810. The line number reference is incorrect.
- 🔵 **Test Coverage**: Identity-override test (line 894) name becomes misleading — it no longer tests the identity-override scenario since `reviewer` is no longer equal to the default `accelerator:reviewer`.
- 🔵 **Compatibility**: New config-dump test only spot-checks 2 of 7 agents, creating asymmetry with the config-read-agents test that checks all 7.

### Assessment

The plan is substantially improved. All critical and major findings from the initial review have been addressed. The correctness lens found no remaining issues — a thorough sweep of test-config.sh confirmed all bare default assertions are now covered. One new major finding was introduced: the config-dump test uses the wrong variable name (`$DUMP` instead of `$CONFIG_DUMP`), which would cause the test to fail. The incorrect line number reference ("around line 720" instead of "around line 1810") is a minor accuracy issue. Plan is acceptable but could be improved — see major finding above.
