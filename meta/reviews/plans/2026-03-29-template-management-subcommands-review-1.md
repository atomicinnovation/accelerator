---
date: "2026-03-29T17:30:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-03-29-template-management-subcommands.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, code-quality, correctness, test-coverage, standards, usability, safety]
review_pass: 2
status: complete
---

## Plan Review: Template Management Subcommands

**Verdict:** REVISE

The plan is well-structured with sound phase ordering, clear scope boundaries, and consistent adherence to existing script conventions. The hybrid approach (scripts for deterministic operations, prompt-only for interactive reset) is a pragmatic design decision, and the extraction of `config_enumerate_templates()` into shared utilities demonstrates good instinct for reducing duplication. However, the plan's primary weakness -- flagged independently by 6 of 7 lenses -- is the extensive duplication of three-tier resolution logic across four new scripts plus the existing `config-read-template.sh`. This creates a significant maintenance burden and drift risk. Additionally, the prompt-only `reset` subcommand raises safety concerns around destructive file deletion without script-level safeguards, and the eject `--all` exit code aggregation has a correctness bug with inverted severity ordering.

### Cross-Cutting Themes

- **Three-tier resolution logic duplication** (flagged by: architecture, code-quality, correctness, usability, test-coverage, standards) -- The core resolution algorithm (config path -> templates directory -> plugin default) is implemented inline in 4 new scripts plus `config-read-template.sh`. The plan extracts `config_enumerate_templates()` but stops short of extracting the resolution logic itself, which is the higher-value abstraction. This is the single most impactful improvement to make.

- **Available-templates formatting duplication** (flagged by: code-quality) -- The `config_enumerate_templates | tr | sed | sed` formatting pipeline appears in every error path across 4 scripts, compounding the duplication issue.

- **Prompt-only reset lacks safeguards** (flagged by: safety, usability, test-coverage) -- The `reset` subcommand delegates destructive file deletion to LLM prompt adherence with no backup mechanism, no automated tests, and resolution logic that may diverge from the script-backed subcommands.

### Tradeoff Analysis

- **Script consistency vs prompt flexibility for reset**: Safety and usability lenses favour a script-backed reset with backup; the plan's prompt-only approach reduces script count but introduces reliability and testing gaps. Recommendation: implement as a thin script with `--confirm` flag for the actual deletion, keeping confirmation interactive while making resolution deterministic.

- **Verbose paths vs clean output in list**: Usability notes that absolute plugin paths in the list table add noise; however, the architecture lens notes the paths are useful for debugging resolution. Recommendation: show relative paths for user-owned files and a short label for plugin defaults.

### Findings

#### Critical

No critical findings.

#### Major

- 🟡 **Architecture / Code Quality / Correctness / Usability**: Three-tier resolution logic duplicated across four scripts
  **Location**: Phases 2-5
  The same ~20-line resolution cascade is implemented inline in `config-list-templates.sh`, `config-show-template.sh`, `config-diff-template.sh`, and partially in `config-eject-template.sh`, plus already exists in `config-read-template.sh`. Extract a `config_resolve_template()` helper into `config-common.sh` that returns both source and path.

- 🟡 **Code Quality**: Available-templates formatting logic duplicated in every error path
  **Location**: Phases 2-5
  The `tr | sed | sed` pipeline with `(none found)` fallback appears in 4 scripts. Extract a `config_format_available_templates()` helper to `config-common.sh`.

- 🟡 **Correctness**: Exit code aggregation in eject `--all` uses inverted severity ordering
  **Location**: Phase 4, lines 493-500
  `[ "$code" -gt "$exit_code" ]` means exit 2 (exists) dominates exit 1 (error), masking genuine errors. Track states separately so error (1) takes precedence over exists (2).

- 🟡 **Safety**: Reset operation delegates destructive file deletion to LLM with no script-level safeguard
  **Location**: Phase 6
  No backup before delete, no soft-delete, and confirmation depends on LLM prompt adherence. Implement as a script with backup or at minimum show file contents before confirming deletion.

- 🟡 **Test Coverage**: No automated tests for the reset subcommand
  **Location**: Phase 6 / Phase 8
  The highest-risk operation (file deletion) has zero automated test coverage. At minimum, test the resolution logic for reset-relevant scenarios.

- 🟡 **Usability**: Two-pass eject confirmation adds latency and LLM dispatch complexity
  **Location**: Phase 7, eject dispatch instructions
  Running the script twice (once to check, once with `--force`) is fragile for LLM orchestration. Consider a `--dry-run` flag or structured output on exit code 2.

- 🟡 **Test Coverage**: No test for multi-source resolution in a single list invocation
  **Location**: Phase 8, config-list-templates.sh tests
  Missing a test where different templates resolve from different tiers simultaneously (config path, user override, plugin default) in one run.

#### Minor

- 🔵 **Correctness**: `diff -u || true` swallows genuine diff errors (exit code 2)
  **Location**: Phase 5, line 610
  Capture the exit code and only suppress 1 (files differ), not 2 (trouble).

- 🔵 **Correctness / Code Quality**: Argument parser silently accepts multiple positional arguments
  **Location**: Phase 4, eject argument parsing
  `config-eject-template.sh plan research` silently ignores `plan`. Error on unexpected extra arguments.

- 🔵 **Code Quality / Architecture**: Eject `--all` exit code 2 conflates partial success with full "exists"
  **Location**: Phase 4
  The skill layer's two-pass approach becomes awkward with `--all` where some may exist and others succeed.

- 🔵 **Standards**: Variable casing inconsistent between list script (lowercase) and other new scripts (UPPERCASE)
  **Location**: Phase 2 vs Phases 3-5
  Existing convention is UPPERCASE for script-level variables.

- 🔵 **Standards**: Dispatch heading style deviates from existing pattern
  **Location**: Phase 7, SKILL.md `### templates subcommands`
  Existing headings are `` ### `view` ``, `` ### `create` ``. Use `` ### `templates` `` without trailing text.

- 🔵 **Standards**: Inconsistent singular/plural in template script filenames
  **Location**: Phases 2-5
  `config-list-templates.sh` (plural) vs `config-show-template.sh` (singular). Existing convention is singular.

- 🔵 **Usability**: Absolute file paths in list output may be noisy
  **Location**: Phase 2
  Plugin default paths are long and not actionable. Show relative paths for user files, short label for defaults.

- 🔵 **Usability**: Diff "no override" message goes to stdout instead of stderr
  **Location**: Phase 5
  Mixes informational messages with diff content channel.

- 🔵 **Safety**: Eject `--force` overwrites without preserving the existing file
  **Location**: Phase 4
  Minor risk of accidental loss for customised templates. Consider `.bak` or a warning.

- 🔵 **Test Coverage**: Missing test for `--all` partial-failure verifying successful ejects still written
  **Location**: Phase 8, eject tests

- 🔵 **Test Coverage**: Diff output content direction not verified
  **Location**: Phase 8, diff tests
  Test should assert a known added line appears with `+` prefix to verify diff argument order.

- 🔵 **Test Coverage**: Integration tests described in Testing Strategy but not enumerated in Phase 8
  **Location**: Phase 8 / Testing Strategy
  Implementer may omit them. Add as explicit test entries.

- 🔵 **Correctness**: `config-read-path.sh` called per iteration in list script instead of once
  **Location**: Phase 2, line 243
  Hoist above the loop since the result is invariant.

#### Suggestions

- 🔵 **Architecture**: Flat dispatch structure in SKILL.md may need routing pattern as subcommands grow
  **Location**: Phase 7

- 🔵 **Standards**: Skill description frontmatter does not mention templates
  **Location**: Phase 7

- 🔵 **Usability**: Argument hint becoming long; consider shorter top-level hint with progressive disclosure
  **Location**: Phase 7

- 🔵 **Safety**: No explicit guard against `reset --all` being interpreted creatively by LLM
  **Location**: Phase 6
  Add instructions that reset operates on single templates only, with individual confirmations.

### Strengths

- ✅ Follows established `config-*` script conventions precisely (`set -euo pipefail`, source `config-common.sh`, markdown to stdout, errors to stderr)
- ✅ Hybrid approach (scripts for deterministic ops, prompt-only for interactive) is well-justified
- ✅ Phase ordering is sound: foundation first, then scripts with increasing complexity, then integration, then tests
- ✅ Clear scope boundaries with explicit exclusions (no versioning, no resolution order changes)
- ✅ Comprehensive Phase 8 test plan covering happy paths, error cases, and edge cases for all scripts
- ✅ Commands map to a natural template customisation workflow: discover -> inspect -> customise -> compare -> revert
- ✅ Error messages consistently list available template keys, reducing trial-and-error friction
- ✅ `--all` and `--force` flags follow well-established CLI conventions
- ✅ Eject command defaults to refusing overwrites (safe-by-default)
- ✅ Help subcommand update includes clear summary table for template management commands

### Recommended Changes

1. **Extract `config_resolve_template()` into `config-common.sh`** (addresses: resolution duplication, drift risk, subprocess overhead)
   Add a function that takes a template key and plugin root, returns the resolved path and source label. Refactor all scripts (including `config-read-template.sh`) to use it. This is the highest-impact change.

2. **Extract `config_format_available_templates()` into `config-common.sh`** (addresses: error path duplication)
   Centralise the `tr | sed | sed` pipeline with `(none found)` fallback into a single helper function.

3. **Fix eject `--all` exit code aggregation** (addresses: inverted severity ordering)
   Track `had_error` and `had_exists` booleans; prefer exit 1 (error) over exit 2 (exists) in the final exit code.

4. **Implement reset as a thin script** (addresses: safety, testability, consistency)
   Create `config-reset-template.sh` that resolves the override path, outputs what it found, and accepts `--confirm` for actual deletion. Keep confirmation interactive via the skill prompt but make resolution deterministic and testable.

5. **Add `--dry-run` flag to eject script** (addresses: two-pass dispatch complexity)
   Output what would happen without writing files. The skill layer can run `--dry-run` first, present results, then run the actual eject if confirmed.

6. **Fix `diff -u || true` to only suppress exit code 1** (addresses: error swallowing)
   Use `diff -u ...; rc=$?; [ "$rc" -le 1 ] || exit 1`.

7. **Standardise variable casing and filename pluralisation** (addresses: standards findings)
   Use UPPERCASE for script-level variables in all new scripts. Use singular form consistently (`config-list-template.sh`).

8. **Add multi-source resolution test and integration tests to Phase 8** (addresses: test coverage gaps)
   Include a test with config path, user override, and plugin default for different keys in a single list run. Promote integration tests from Testing Strategy prose to Phase 8 entries.

9. **Add explicit "reset operates on single templates only" instruction to SKILL.md** (addresses: safety guard for bulk reset)

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally well-structured, following established patterns consistently and making sensible decisions about script vs prompt-only boundaries. The primary concern is the significant duplication of three-tier resolution logic across four new scripts, which creates a maintenance burden and coupling risk. The shared helper approach in config-common.sh is a good start but stops short of extracting the resolution logic itself.

**Strengths**:
- Follows the established config-* script conventions precisely, maintaining architectural consistency across the module
- The hybrid approach (scripts for deterministic operations, prompt-only for interactive reset) is a well-justified architectural decision
- Extracting config_enumerate_templates() into config-common.sh demonstrates good instinct for centralising shared logic
- Phase ordering is sound: foundation first, then scripts with increasing complexity, then skill integration, then tests
- Clear scope boundaries -- explicitly excludes template versioning, resolution order changes, and consumption-side changes

**Findings**:

1. **Three-tier template resolution logic duplicated across four scripts** (major, high confidence)
   Location: Phases 2-5: Resolution Logic Duplication
   The three-tier resolution logic is implemented inline in all four new scripts plus exists in `config-read-template.sh`. Extract a `config_resolve_template()` helper into `config-common.sh` that returns both the resolved path and source label.

2. **Each resolution tier spawns external scripts for config reads** (minor, high confidence)
   Location: Phases 2-5: Subprocess Invocation Pattern
   Each new script invokes `config-read-value.sh` and `config-read-path.sh` as external subprocesses per template key. Consistent with existing patterns; no immediate change needed.

3. **Eject exit code semantics may not compose well with --all** (minor, medium confidence)
   Location: Phase 4: config-eject-template.sh Script
   Exit code 2 dominates even if other templates succeeded. Consider `--skip-existing` as alternative to `--force`.

4. **Flat dispatch structure may benefit from routing helper as subcommands grow** (suggestion, medium confidence)
   Location: Phase 7: Configure Skill SKILL.md Updates
   The SKILL.md is approaching a size where the LLM may not weight all dispatch instructions equally. Worth monitoring.

### Code Quality

**Summary**: The plan follows existing codebase conventions consistently. The primary code quality concern is significant duplication of the three-tier template resolution logic across four new scripts, which will create a maintenance burden. The shared helper extraction in Phase 1 is a good start but does not go far enough.

**Strengths**:
- Follows the established config-* script naming convention consistently
- Extracts config_enumerate_templates() into config-common.sh, correctly identifying duplication
- The eject script uses a well-designed inner function (_eject_one) with clear exit code semantics
- Phase ordering is logical: foundation first, then scripts in dependency order
- The decision to make reset prompt-only is pragmatic

**Findings**:

1. **Three-tier resolution logic duplicated across four scripts** (major, high confidence)
   Location: Phases 2-5
   Each copy has ~20 lines of resolution code with subtle variations. Extract a `config_resolve_template()` helper.

2. **Available-templates formatting logic duplicated in every error path** (major, high confidence)
   Location: Phases 2-5
   The `tr | sed | sed` pipeline appears identically in 4 scripts. Add `config_format_available_templates()` helper.

3. **Subprocess invocation per template key in list script** (minor, medium confidence)
   Location: Phase 2, lines 230-231
   10 subprocesses for 5 templates. Would become function calls if resolution is extracted.

4. **Argument parser silently accepts multiple positional arguments** (minor, medium confidence)
   Location: Phase 4, argument parsing
   `config-eject-template.sh plan research` silently ignores `plan`. Error on extra arguments.

5. **Exit code aggregation conflates error types** (minor, medium confidence)
   Location: Phase 4, --all exit code logic
   Highest code wins, but 2 > 1 doesn't match severity. Track states separately.

6. **Inconsistent variable casing between scripts** (suggestion, medium confidence)
   Location: Phase 3, variable naming
   List uses lowercase, show/diff use UPPERCASE. Match existing UPPERCASE convention.

### Correctness

**Summary**: The plan is generally well-structured with correct three-tier resolution logic that faithfully mirrors the existing config-read-template.sh pattern. The main correctness concerns are: an inverted exit code severity ordering in the eject --all aggregation logic, a diff error-swallowing pattern, and duplicated resolution logic that could drift between scripts.

**Strengths**:
- Three-tier resolution logic faithfully mirrors the existing pattern
- Argument validation consistently handled across all new scripts
- Eject script correctly uses mkdir -p before cp
- Diff script correctly handles diff exit code 1 to avoid triggering set -e failure
- config_enumerate_templates helper correctly handles missing templates directory

**Findings**:

1. **Exit code aggregation uses inverted severity ordering** (major, high confidence)
   Location: Phase 4, lines 493-500
   Exit code 2 (exists) treated as more severe than 1 (error). Track states separately.

2. **diff error exit code (2) is swallowed by || true** (minor, high confidence)
   Location: Phase 5, line 610
   Capture exit code and only suppress 1, not 2.

3. **Argument parser silently accepts multiple positional arguments** (minor, medium confidence)
   Location: Phase 4, argument parsing

4. **Three-tier resolution logic duplicated across four scripts risks drift** (major, medium confidence)
   Location: Phases 2, 3, 5

5. **config-read-path.sh called per iteration instead of once** (minor, medium confidence)
   Location: Phase 2, line 243-244
   Hoist above the loop.

6. **Glob pattern edge case verified as correct** (minor, medium confidence -- no action needed)
   Location: Phase 1, line 147

### Test Coverage

**Summary**: The plan includes a dedicated Phase 8 for comprehensive test coverage that follows existing patterns closely. The test catalogue is thorough for happy paths and error cases. However, there are notable gaps in integration test coverage, the resolution logic duplication creates untested seams, and the prompt-only reset subcommand lacks any automated verification.

**Strengths**:
- Phase 8 provides detailed, categorised test plan covering each script with specific cases
- Correctly follows existing test-config.sh patterns
- Integration tests explicitly called out in Testing Strategy
- Edge cases enumerated for each script
- Skill integration checks included

**Findings**:

1. **No automated tests for the reset subcommand** (major, high confidence)
   Location: Phase 6
   Highest-risk operation has zero automated test coverage.

2. **No test for three-tier resolution order correctness in list script** (major, high confidence)
   Location: Phase 8, config-list-templates.sh tests
   Missing multi-source test with different tiers for different keys in one run.

3. **Missing test for --all with mixed existing/new files verifying successful writes** (minor, high confidence)
   Location: Phase 8, eject tests

4. **Diff output content not verified beyond existence** (minor, medium confidence)
   Location: Phase 8, diff tests
   Should assert direction of diff (+ prefix for additions).

5. **No test for config_enumerate_templates with only non-.md files** (minor, medium confidence)
   Location: Phase 1 / Phase 8

6. **Integration tests described but not enumerated in Phase 8** (suggestion, medium confidence)
   Location: Testing Strategy vs Phase 8
   Promote to explicit Phase 8 entries.

### Standards

**Summary**: The plan follows established project conventions well overall. There are a few naming and casing inconsistencies between the proposed new scripts, and one deviation from the SKILL.md dispatch heading convention.

**Strengths**:
- All new scripts follow established config-* naming convention and patterns
- Test plan follows existing test-config.sh patterns
- Internal helper functions use underscore-prefix convention
- SKILL.md dispatch uses H3 headings under Available Actions
- Shared helper follows config_* function naming convention

**Findings**:

1. **Variable casing inconsistent with existing scripts** (minor, high confidence)
   Location: Phase 2 vs Phases 3-5
   List uses lowercase; existing convention and other new scripts use UPPERCASE.

2. **Dispatch heading style deviates from existing pattern** (minor, medium confidence)
   Location: Phase 7, SKILL.md
   Use `` ### `templates` `` without trailing text.

3. **Skill description frontmatter does not mention templates** (suggestion, medium confidence)
   Location: Phase 7

4. **Inconsistent singular/plural in template script filenames** (minor, medium confidence)
   Location: Phases 2-5
   `config-list-templates.sh` (plural) vs singular in others. Use singular consistently.

### Usability

**Summary**: The plan delivers well-structured template management subcommands that follow existing CLI patterns with good progressive disclosure. The command hierarchy maps naturally to a standard customisation workflow. Key concerns include resolution logic duplication risking inconsistent user experience, the two-pass eject confirmation design, and the prompt-only reset approach.

**Strengths**:
- Five subcommands map to natural workflow: discover -> inspect -> customise -> compare -> revert
- Error messages consistently list available keys
- `--all` and `--force` flags follow well-established CLI conventions
- Nesting under `configure templates` avoids namespace sprawl
- Help subcommand update includes clear summary table

**Findings**:

1. **Resolution logic duplication creates inconsistency risk** (major, high confidence)
   Location: Phases 2-5
   Different subcommands could disagree about resolution, confusing users.

2. **Two-pass eject confirmation adds latency and dispatch complexity** (major, medium confidence)
   Location: Phase 7, eject dispatch
   Consider `--dry-run` flag.

3. **Inconsistent implementation approach between reset and other subcommands** (minor, high confidence)
   Location: Phase 6
   Consider thin script for deterministic resolution.

4. **Absolute file paths in list output may be noisy** (minor, medium confidence)
   Location: Phase 2
   Show relative paths for user files, short label for defaults.

5. **Diff "no override" message goes to stdout** (minor, medium confidence)
   Location: Phase 5
   Should go to stderr to avoid mixing with content.

6. **Argument hint becoming long** (suggestion, medium confidence)
   Location: Phase 7
   Consider shorter top-level hint.

### Safety

**Summary**: The plan introduces template management commands with appropriate safety measures for most operations. The eject command correctly guards against overwrites. However, the prompt-only reset operation lacks sufficient safeguards against accidental data loss -- it delegates file deletion to an LLM without backup or script-level confirmation.

**Strengths**:
- Eject defaults to refusing overwrites (safe-by-default)
- All read-only operations are scripts with no modification risk
- Plugin defaults are never modified
- Error paths consistently report available templates

**Findings**:

1. **Reset delegates destructive deletion to LLM with no script-level safeguard** (major, high confidence)
   Location: Phase 6
   No backup before delete, no soft-delete. Implement as script with backup.

2. **Eject --force overwrites without preserving existing file** (minor, medium confidence)
   Location: Phase 4
   Consider `.bak` copy before overwrite.

3. **Eject --all with partial failures may confuse dispatchers** (minor, medium confidence)
   Location: Phase 4, lines 492-500
   Document best-effort behaviour in SKILL.md.

4. **No guard against `reset --all` being interpreted by LLM** (suggestion, low confidence)
   Location: Phase 6
   Add explicit single-template-only instruction.

## Re-Review (Pass 2) — 2026-03-29

**Verdict:** COMMENT

### Previously Identified Issues

- ✅ **Architecture/Code Quality/Correctness/Usability**: Three-tier resolution logic duplicated across four scripts — **Resolved**. Extracted `config_resolve_template()` into `config-common.sh`; all scripts now use the shared helper.
- ✅ **Code Quality**: Available-templates formatting duplicated in every error path — **Resolved**. Extracted `config_format_available_templates()` into `config-common.sh`.
- ✅ **Correctness**: Eject `--all` exit code aggregation inverted severity — **Resolved**. Now uses `HAD_ERROR`/`HAD_EXISTS` booleans with error (1) taking precedence.
- ✅ **Safety**: Reset delegates destructive deletion to LLM with no safeguard — **Resolved**. Implemented as `config-reset-template.sh` script with `.bak` backup and `--confirm` flag.
- ✅ **Test Coverage**: No automated tests for reset subcommand — **Resolved**. Phase 8 now includes comprehensive reset tests.
- ✅ **Usability**: Two-pass eject confirmation fragile for LLM dispatch — **Resolved**. Added `--dry-run` flag to eject script.
- ✅ **Test Coverage**: No multi-source resolution test in list — **Resolved**. Added to Phase 8.
- ✅ **Correctness**: `diff || true` swallows genuine errors — **Resolved**. Now captures exit code and only suppresses 1.
- ✅ **Correctness/Code Quality**: Argument parser silently accepts multiple positional args — **Resolved**. Strict argument parsing with error on extras.
- ✅ **Standards**: Variable casing inconsistent — **Resolved**. All scripts use UPPERCASE.
- ✅ **Standards**: Dispatch heading style deviation — **Resolved**. Now uses `` ### `templates` `` without trailing text.
- ✅ **Standards**: Inconsistent singular/plural in filenames — **Resolved**. All use singular form.
- ✅ **Safety**: No guard against `reset --all` — **Resolved**. SKILL.md explicitly states single-template-only with individual confirmations.
- ✅ **Standards**: Skill description doesn't mention templates — **Resolved**. Updated in Phase 7.
- ✅ **Usability**: Argument hint too long — **Resolved**. Shortened to `[view | create | help | templates ...]`.
- ✅ **Test Coverage**: Integration tests not in Phase 8 — **Resolved**. Promoted to explicit Phase 8 entries.

### New Issues Introduced

#### Major (4)

- 🟡 **Architecture / Code Quality / Usability**: Two-line stdout protocol for `config_resolve_template()` is fragile
  Callers parse with `head -1` / `tail -1`; any future output change silently breaks all 6 consumers. Consider delimiter-based single-line format or wrapper functions.

- 🟡 **Correctness / Code Quality**: `--all` exit code capture in eject script may not work under `set -e`
  The `_eject_one "$KEY" || { CODE=$?; ... }` pattern — `$?` inside `|| { }` may not reliably capture the function's exit code depending on shell version. Use `_eject_one "$KEY"; RC=$?; if [ "$RC" -ne 0 ]; then ...` or `local rc=0; _eject_one "$KEY" || rc=$?`.

- 🟡 **Safety**: `eject --force` overwrites user customizations without backup
  Unlike `reset` (which creates `.bak`), `eject --force` silently overwrites with no recovery path. `--all --force` amplifies this risk. Add backup before overwrite.

- 🟡 **Test Coverage**: Missing test assertion helpers for file existence/content
  Eject and reset tests need `assert_file_exists` / `assert_file_not_exists` / `assert_file_content_eq` — not in the current test harness.

#### Minor (10)

- 🔵 **Architecture**: Reset of config-path overrides leaves system in inconsistent state (stale config entry)
- 🔵 **Correctness**: Eject creates unreachable Tier 2 file when Tier 1 config path override is active — no warning
- 🔵 **Correctness**: Diff reports "no customised template" when config path file is missing (contradicts stderr warning)
- 🔵 **Code Quality**: Diff/reset scripts string-match on `"plugin default"` label — consider constants
- 🔵 **Standards**: `SCRIPT_DIR` parameter to `config_resolve_template` redundant (all scripts share same directory)
- 🔵 **Standards**: `--all` stored in `TEMPLATE_NAME` mixes flag/value namespaces
- 🔵 **Standards**: Exit code 0 for diff "no override" vs exit code 2 for reset "no override" — inconsistent
- 🔵 **Test Coverage**: No regression tests for refactored `config-read-template.sh`
- 🔵 **Test Coverage**: No test for diff exit code 0 when files differ
- 🔵 **Safety**: Repeated reset overwrites previous `.bak` without warning

#### Suggestions (4)

- 🔵 **Architecture**: Define source label constants in `config-common.sh`
- 🔵 **Standards**: H4 sub-dispatch is a new pattern (acceptable, just noting)
- 🔵 **Usability**: Use `diff --label` for human-readable diff headers
- 🔵 **Usability**: Add summary line to `--all` partial eject output

### Assessment

All 16 findings from the initial review have been addressed by the plan edits. The plan is now in good shape for implementation. The re-review identified 4 new major findings — the most impactful being the two-line stdout protocol fragility and the `--all` exit code capture bug, both of which are implementation-level concerns that can be fixed during coding. The `eject --force` backup gap is a quick addition that mirrors the existing `reset` pattern. The missing test assertion helpers should be addressed at the start of Phase 8.

Plan is acceptable as-is with these observations noted — none require structural changes to the plan, and all can be addressed during implementation.
