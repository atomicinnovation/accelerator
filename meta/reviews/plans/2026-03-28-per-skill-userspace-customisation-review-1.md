---
date: "2026-03-28T13:15:00+0000"
type: plan-review
skill: review-plan
target: "meta/plans/2026-03-28-per-skill-userspace-customisation.md"
review_number: 1
verdict: APPROVE
lenses: [architecture, code-quality, correctness, standards, usability, test-coverage, documentation]
review_pass: 2
status: complete
---

## Plan Review: Per-Skill Userspace Customisation

**Verdict:** COMMENT

The plan is well-structured, follows established codebase patterns closely,
and provides a clean extension of the existing customisation system. The
two-file model (context.md + instructions.md) with clear injection points is
architecturally sound, and the four-phase rollout is sensible. The main
concerns are around silent failure modes for typos in skill directory names,
missing test coverage for the Phase 2 skill integration, and documentation
gaps around troubleshooting and the context composition model. All findings
are addressable without changing the overall design.

### Cross-Cutting Themes

- **Silent failure on mistyped skill names** (flagged by: architecture,
  usability, documentation) — The plan explicitly defers skill name validation,
  but three lenses independently identified that users creating directories
  with typos get no feedback. This is the single most impactful usability gap.
  A lightweight warning in `config-summary.sh` would address it without
  changing the open/extensible design.

- **Script duplication** (flagged by: architecture, code-quality) — The two
  new scripts are structurally identical, differing only in filename and header
  text. Both lenses suggest either a parameterised shared script or accepting
  the duplication with a comment. Given the scripts are ~20 lines each, either
  approach is reasonable.

- **Shell idiom inconsistency** (flagged by: code-quality, standards) — Both
  scripts use `cat "$FILE" | config_trim_body` instead of the established
  `printf '%s\n' "$var" | config_trim_body` pattern. Minor but worth fixing
  for consistency.

### Tradeoff Analysis

- **Validation vs. extensibility**: Adding skill name validation in
  `config-summary.sh` improves the debugging experience but couples the
  summary script to a known list of skill names. This is acceptable since the
  13 skill names are already enumerated in multiple places (configure skill,
  config-read-agents.sh), and the warning would be advisory only.

### Findings

#### Critical

(none)

#### Major

- 🟡 **Correctness**: Summary reports skills with empty/whitespace-only files
  **Location**: Phase 1, Section 3: config-summary.sh update
  The summary checks `[ -f "$skill_dir/context.md" ]` but the reader scripts
  skip empty files via `config_trim_body`. An empty placeholder file would
  appear in the session summary but have no effect.

- 🟡 **Usability/Architecture/Documentation**: Silent failure on mistyped
  skill directory names
  **Location**: What We're NOT Doing; Phase 3 documentation
  No feedback mechanism when a directory name doesn't match any known skill.
  Three lenses flagged this independently as the primary usability concern.

- 🟡 **Test Coverage**: No preprocessor placement tests for Phase 2 skill
  integration
  **Location**: Phase 4: Testing
  The existing test suite has preprocessor placement tests for
  config-read-context.sh. Phase 4 proposes no equivalent for the two new
  preprocessor lines across 13 skills.

- 🟡 **Test Coverage**: No test for config-detect.sh hook integration with
  per-skill data
  **Location**: Phase 4: Testing
  The testing strategy mentions hook integration but Phase 4 includes no
  actual test case for per-skill data appearing in the hook's JSON output.

- 🟡 **Documentation**: No explanation of per-skill context interaction with
  global context
  **Location**: Phase 3: Per-Skill Customisation section
  The documentation says content is "injected after global project context"
  but doesn't explain the composition model or what happens when per-skill
  and global context conflict.

#### Minor

- 🔵 **Code Quality/Standards**: `cat "$FILE" | config_trim_body` diverges
  from established piping pattern
  **Location**: Phase 1, Sections 1-2: Reader scripts
  Should use the read-then-pipe pattern matching `config-read-context.sh`.

- 🔵 **Architecture/Code Quality**: Two nearly identical scripts could share
  a common implementation
  **Location**: Phase 1, Sections 1-2: Reader scripts
  The scripts differ only in filename (`context.md` vs `instructions.md`) and
  section header text.

- 🔵 **Correctness**: Per-skill files with YAML frontmatter will include raw
  frontmatter in output
  **Location**: Phase 1, Sections 1-2: Reader scripts
  Unlike `config-read-context.sh` which uses `config_extract_body`, the new
  scripts pass all content through including any frontmatter delimiters.

- 🔵 **Standards**: Variable name `SKILL_OVERRIDES_DIR` uses inconsistent
  terminology
  **Location**: Phase 1, Section 3: config-summary.sh
  The plan calls the feature "customisations" throughout but the variable
  uses "overrides".

- 🔵 **Documentation**: No documentation on removing or temporarily
  disabling per-skill customisations
  **Location**: Phase 3: Per-Skill Customisation section

- 🔵 **Documentation**: No mention that the configure skill is excluded
  **Location**: Phase 3: Per-Skill Customisation section
  Users may notice the 13-skill table is missing `configure` without
  explanation.

- 🔵 **Usability**: No guidance on when to use context.md vs instructions.md
  vs global context
  **Location**: Phase 3: Per-Skill Customisation section

- 🔵 **Test Coverage**: No test verifying exact output format (section header
  and wrapper text)
  **Location**: Phase 4: config-read-skill-context.sh tests

- 🔵 **Documentation**: No mention of shared/personal split for per-skill
  files
  **Location**: Phase 3: Per-Skill Customisation section
  Users may wonder whether to commit or gitignore per-skill directories.

#### Suggestions

- 🔵 **Usability**: Session summary could show file paths for easy editing
- 🔵 **Usability**: Skill names could note relationship to slash command names
  in documentation
- 🔵 **Test Coverage**: Exit code should be verified for missing argument
  error path

### Strengths

- ✅ Follows established codebase patterns consistently — script naming,
  directory conventions, shell boilerplate, and preprocessor integration all
  match existing infrastructure
- ✅ Clean separation between context (positioned after global context) and
  instructions (positioned at end of skill) gives users a clear mental model
- ✅ Graceful degradation — missing directories or files produce no output, so
  skills work identically when no customisation exists
- ✅ Comprehensive skill name reference table and practical examples in
  documentation
- ✅ Sensible exclusion of the configure skill from per-skill customisation
- ✅ Good edge case coverage in proposed tests (empty, whitespace, isolation)
- ✅ Performance analysis demonstrates negligible overhead

### Recommended Changes

1. **Add content checks to config-summary.sh** (addresses: empty file
   inconsistency)
   In the per-skill detection loop, check that files have non-empty trimmed
   content before reporting them, matching the reader scripts' behavior.

2. **Add skill name validation warning to config-summary.sh** (addresses:
   silent failure on typos)
   When listing per-skill customisations, emit a stderr warning for directory
   names that don't match any of the 13 known skill names. Include the known
   names in the warning for quick reference.

3. **Add preprocessor placement tests to Phase 4** (addresses: missing
   integration tests)
   Mirror the existing preprocessor placement tests: verify all 13 skills
   have both new preprocessor lines, verify correct skill name arguments,
   verify ordering (skill-context after global context, skill-instructions
   at EOF).

4. **Add config-detect.sh hook integration test** (addresses: missing hook
   test)
   Create per-skill files, run config-detect.sh, assert per-skill data
   appears in the JSON additionalContext.

5. **Document context composition model** (addresses: interaction gap)
   Add a sentence explaining that per-skill context supplements global
   context (both visible to the skill), and that per-skill instructions
   appear later in the prompt.

6. **Fix shell idiom** (addresses: cat piping pattern)
   Use `config_trim_body < "$FILE"` or the read-then-pipe pattern instead
   of `cat "$FILE" | config_trim_body`.

7. **Add troubleshooting note to documentation** (addresses: typo debugging)
   Note that directory names must match exactly and point users to the
   session start summary for verification.

8. **Rename `SKILL_OVERRIDES_DIR` to `SKILL_CUSTOMISATIONS_DIR`** (addresses:
   terminology inconsistency)

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is well-structured and follows established codebase
patterns closely, extending the existing convention-based directory scanning
approach from custom lenses to per-skill customisation. The two-script
decomposition with clear injection points is clean. Minor concerns around
growing duplication and silent failure modes.

**Strengths**:
- Follows the established architectural pattern from custom lenses
- Clean separation between context and instructions injection points
- Sensible exclusion of the configure skill
- Independently testable scripts with no side effects
- Graceful degradation for missing files/directories

**Findings**:
- 🔵 minor/high: Growing duplication across directory-scanning scripts —
  third instance of scan-subdirectories pattern
- 🔵 minor/high: Silent failure on mistyped skill directory names
- 🔵 minor/medium: Skill name hardcoded as string argument rather than
  derived from frontmatter
- 🔵 suggestion/medium: Two nearly identical scripts could share
  implementation

### Code Quality

**Summary**: The plan follows established conventions closely. Main concerns
are the near-identical duplication between the two new scripts and a useless
use of cat.

**Strengths**:
- Scripts follow the exact same structure as existing config readers
- Single responsibility per script with simple linear flow
- Clear scope boundaries via "What We're NOT Doing"
- Summary additions integrate naturally with existing structure

**Findings**:
- 🔵 minor/high: Near-identical scripts could share common implementation
- 🔵 minor/high: Useless use of cat — should use input redirection or
  established piping pattern
- 🔵 minor/medium: String concatenation for customisations matches existing
  style but adds cognitive complexity
- 🔵 suggestion/medium: Skill name as string literal creates coupling
- 🔵 suggestion/medium: Test plan describes cases but not implementations

### Correctness

**Summary**: The plan is logically sound with correct shell scripting patterns.
Two notable issues: summary detection doesn't match reader behavior for empty
files, and frontmatter in user files would pass through unstripped.

**Strengths**:
- Reader scripts correctly handle empty/whitespace edge cases
- Glob expansion edge case handled by directory guard
- Correct placement of context and instructions in skills
- All 13 user-facing skills correctly identified

**Findings**:
- 🟡 major/high: Summary reports skills with empty/whitespace-only files
- 🔵 minor/medium: Per-skill files with YAML frontmatter include raw
  frontmatter in output
- 🔵 minor/medium: Skill name passed with no automated validation against
  frontmatter
- 🔵 suggestion/low: Special characters in skill names untested

### Standards

**Summary**: Strong adherence to established conventions. Script naming,
directory conventions, and shell boilerplate all match existing patterns.
Minor inconsistencies in shell idiom and variable naming.

**Strengths**:
- Script naming follows `config-read-<thing>.sh` convention
- `.claude/accelerator/skills/` extends existing `/lenses/` hierarchy
- Shell boilerplate matches established pattern exactly
- Skill name table verified against frontmatter
- Test and documentation structure follow existing style

**Findings**:
- 🔵 minor/high: Useless use of cat diverges from established piping pattern
- 🔵 minor/medium: Variable name `SKILL_OVERRIDES_DIR` uses inconsistent
  terminology
- 🔵 minor/high: Header comment style has minor divergence (acceptable)
- 🔵 suggestion/medium: No skill name argument validation (consistent with
  existing pass-through pattern)

### Usability

**Summary**: Well-structured feature with intuitive two-file model and
thorough documentation. Main gap is silent failure on typos and lack of
feedback when customisations are loaded.

**Strengths**:
- Context vs instructions distinction well-explained with examples
- Consistent with custom lenses convention
- Session start reporting provides visibility
- Complete skill name reference table
- Graceful empty file handling

**Findings**:
- 🟡 major/high: Silent failure on mistyped skill directory names
- 🔵 minor/medium: No runtime feedback that customisations were loaded
- 🔵 minor/medium: No guidance on context.md vs instructions.md decision
- 🔵 minor/medium: Create action doesn't scaffold per-skill files
- 🔵 suggestion/low: Skill names not self-evident from slash command names
- 🔵 suggestion/medium: Session summary could show file paths

### Test Coverage

**Summary**: Phase 4 covers core happy paths and edge cases for the new
scripts, following established test patterns. Notable gaps around Phase 2
preprocessor placement verification and hook integration testing.

**Strengths**:
- Test cases follow established test-config.sh patterns
- Good empty/whitespace edge case coverage
- Isolation test catches realistic bugs
- Summary tests cover combinatorial cases

**Findings**:
- 🟡 major/high: No preprocessor placement tests for 13 skill integrations
- 🟡 major/high: No test for config-detect.sh hook integration
- 🔵 minor/medium: No test for special characters in skill name
- 🔵 minor/high: No test verifying exact output format
- 🔵 minor/medium: No test for ordering of multiple skills in summary
- 🔵 suggestion/medium: Exit code should be verified for missing argument

### Documentation

**Summary**: Phase 3 documentation is well-structured with a complete skill
name table and practical examples. Gaps around troubleshooting, context
composition, and lifecycle guidance.

**Strengths**:
- Complete 13-skill reference table
- Clear examples distinguishing context from instructions
- Directory tree illustration matches existing style
- Create action guidance correctly updated

**Findings**:
- 🟡 major/high: No guidance on diagnosing silent failures from typos
- 🟡 major/high: No explanation of per-skill context interaction with global
  context
- 🔵 minor/high: No documentation on removal/disabling
- 🔵 minor/medium: No mention of shared/personal split for per-skill files
- 🔵 suggestion/medium: Create action text buries per-skill among features
- 🔵 minor/high: No mention that configure skill is excluded

## Re-Review (Pass 2) — 2026-03-28

**Verdict:** APPROVE

### Previously Identified Issues

- 🟡 **Correctness**: Summary reports skills with empty/whitespace-only files — **Resolved**. Summary now uses `config_trim_body` content checks.
- 🟡 **Usability/Architecture/Documentation**: Silent failure on mistyped skill directory names — **Resolved**. `KNOWN_SKILLS` validation with stderr warnings added; troubleshooting documentation added.
- 🟡 **Test Coverage**: No preprocessor placement tests for Phase 2 — **Resolved**. 6 preprocessor placement tests added mirroring existing pattern.
- 🟡 **Test Coverage**: No test for config-detect.sh hook integration — **Resolved**. 2 hook integration tests added.
- 🟡 **Documentation**: No explanation of per-skill context interaction with global context — **Resolved**. "When to use which" section added with composition model.
- 🔵 **Code Quality/Standards**: Cat piping pattern — **Resolved**. Changed to `config_trim_body < "$FILE"`.
- 🔵 **Architecture/Code Quality**: Two nearly identical scripts — **Accepted**. Duplication kept as conscious tradeoff at ~20 lines each.
- 🔵 **Correctness**: YAML frontmatter in per-skill files — **Resolved**. Documentation now states "Do not add YAML frontmatter".
- 🔵 **Standards**: SKILL_OVERRIDES_DIR variable naming — **Resolved**. Renamed to `SKILL_CUSTOM_DIR`.
- 🔵 **Documentation**: No removal/disabling guidance — **Resolved**. Troubleshooting section includes rename pattern.
- 🔵 **Documentation**: No configure skill exclusion note — **Resolved**. Added after skill table.
- 🔵 **Usability**: No context vs instructions decision guide — **Resolved**. "When to use which" paragraph added.
- 🔵 **Documentation**: No shared/personal split mention — **Resolved**. "Shared vs personal" paragraph added.

### New Issues Introduced

- 🔵 **Architecture** (minor): `KNOWN_SKILLS` list is a new maintenance coupling point — a third location where skill names are enumerated. Should add a test verifying it stays in sync with actual skills.
- 🔵 **Correctness** (minor): `KNOWN_SKILLS` includes `configure` but the configure skill has no preprocessor lines. Files in `skills/configure/` would appear in summary with no effect. Consider removing `configure` from the list.
- 🔵 **Usability** (minor): Unrecognised skill name warning doesn't suggest the closest match or list valid options. Consider appending valid names or a help reference.
- 🔵 **Test Coverage** (minor): Plan references `assert_contains` and `assert_empty` helpers that don't exist in the current test harness. Need to add these or use `assert_eq`.
- 🔵 **Documentation** (minor): Troubleshooting references `/accelerator:configure view` but the plan doesn't update the `view` action to show per-skill customisations.

### Assessment

All 5 major findings from the initial review have been fully resolved. The plan
updates are thorough and well-integrated. The 5 new minor issues are small
refinements that do not affect the plan's overall soundness — they can be
addressed during implementation. The plan is ready for implementation.
