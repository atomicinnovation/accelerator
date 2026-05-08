---
date: "2026-05-08T00:00:00Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-08-0030-remove-inline-path-defaults-from-consumers.md"
review_number: 1
verdict: COMMENT
lenses: [correctness, test-coverage, code-quality, architecture, compatibility, safety]
review_pass: 2
status: complete
---

## Plan Review: Remove Inline Path Defaults from Consumer Call Sites

**Verdict:** REVISE

The plan is well-conceived and structurally sound: the three-phase TDD approach, the correct scoping of the fix to `config-read-path.sh` alone, and the thorough exclusion rationale for test-config.sh and migration scripts all reflect careful thinking. The core lookup-loop logic is correct and the backward-compatibility guarantee is properly preserved. However, four major issues require plan changes before implementation: the structural invariant test — the keystone enforcement mechanism of Phase 3 — has a regex that will silently pass all 13 bash consumer files unchecked; the plan deliberately violates `config-defaults.sh`'s own governance comment without updating it; the Phase 2 test block omits the critical combination of no-`$2` with a config-set value; and the plan leaves unknown-key empty-string failures silent after stripping consumer fallback defaults.

### Cross-Cutting Themes

- **Structural invariant test regex gap** (flagged by: Correctness, Compatibility, Test Coverage, Safety) — The grep pattern requires whitespace immediately after `config-read-path.sh`, but every bash consumer wraps the path in double-quotes (`"$VAR/scripts/config-read-path.sh"` → `"` not `[[:space:]]` follows `.sh`). The test fires for SKILL.md files only. All 13 bash consumers plus the multiline `jira-common.sh` case are invisible to the structural test. This also affects the AC verification greps in the Verification section.

- **config-defaults.sh governance comment** (flagged by: Code Quality, Architecture, Compatibility, Safety) — Line 19 of `config-defaults.sh` reads "Do not source this file directly — source config-common.sh instead." Phase 2 deliberately breaks this. The plan justifies the exception but leaves the comment in place, creating a trap for future contributors who will find a violated rule with no explanation.

### Findings

#### Major

- 🟡 **Correctness / Compatibility / Test Coverage / Safety**: Structural invariant test regex does not match bash consumer call sites
  **Location**: Phase 3: Add structural invariant test; Verification section AC greps
  The pattern `config-read-path\.sh[[:space:]]+[a-zA-Z_]...` requires whitespace after `.sh`, but all 13 bash consumers use quoted paths (`"$PLUGIN_ROOT/scripts/config-read-path.sh" key default`) where `"` immediately follows `.sh`. The pattern never matches any `.sh` file. The jira-common.sh multiline case (key on line 73, default on line 74 after `\`) is additionally invisible to single-line grep. A Phase 3 implementation that updates only SKILL.md files and skips all bash consumers would produce a passing structural test. The AC greps in the Verification section have the same flaw.

- 🟡 **Code Quality / Architecture / Compatibility / Safety**: Direct sourcing of `config-defaults.sh` contradicts its own governance comment without updating it
  **Location**: Phase 2: Update `scripts/config-read-path.sh`
  `config-defaults.sh` line 19 explicitly forbids direct sourcing. Phase 2 proposes `source "$SCRIPT_DIR/config-defaults.sh"` in `config-read-path.sh` — the plan correctly justifies this as safe and lightweight, but does not include updating the governance comment. Future contributors reading the comment may either distrust the `config-read-path.sh` approach or "fix" it by switching to `config-common.sh`, inadvertently pulling in VCS detection overhead.

- 🟡 **Test Coverage**: No test for no-`$2` + config-set value combination
  **Location**: Phase 2: test additions (`=== config-read-path.sh (no-default lookup) ===`)
  Every test in the new block calls `bash "$READ_PATH" <key>` against a repo with no config, verifying the centralized default is returned. There is no test for the combination of absent `$2` with a configured value in `.accelerator/config.md`. This means a quoting or delegation bug in the new lookup branch (e.g., the resolved `default` is passed to `config-read-value.sh` in a way that shadows a configured value) would go undetected.

- 🟡 **Safety**: Unknown key silently resolves to empty string after consumer defaults are stripped
  **Location**: Phase 2: `config-read-path.sh` implementation; Phase 3: bash consumer scripts
  After Phase 3 strips hardcoded defaults from all consumers, a typo'd or future-unregistered key returns an empty string with exit 0. Consumers that previously had an explicit fallback (e.g., `config-read-path.sh tmp .accelerator/tmp`) were protected; after stripping, `launch-server.sh` constructs `cd "$PROJECT_ROOT/"` on empty `TMP_REL`, targeting the project root. The current explicit-default style gave callers a safety net; the new style removes it without adding any server-side guard.

#### Minor

- 🔵 **Test Coverage / Architecture**: Unknown-key empty-output behaviour is manual-only
  **Location**: Testing Strategy: Manual Testing Steps (step 3)
  Manual step 3 specifies empty output for an unknown key — this is a defined contract of the new loop — but there is no automated assertion. A future change to the loop that introduces a default string or non-zero exit for unknown keys would not be caught by the suite.

- 🔵 **Code Quality**: Consumer update table entry for `run.sh` is ambiguous
  **Location**: Phase 3: bash consumer scripts table — `inventory-design/scripts/playwright/run.sh` line 21
  The Before/After for this row ends with `...` rather than showing the complete replacement. All other rows show the full After string. This line has additional arguments after the default (`2>/dev/null || echo '.accelerator/tmp'`) that the implementer must independently determine how to handle.

- 🔵 **Compatibility**: Explicit empty-string `$2` silently becomes equivalent to absent `$2`
  **Location**: Phase 2: Update `scripts/config-read-path.sh` (proposed code)
  The guard `if [ -n "${2:-}" ]` treats `bash config-read-path.sh plans ""` the same as `bash config-read-path.sh plans` — both trigger the centralized lookup. The current behaviour passes the empty string through as the default. No current caller passes `""`, but the contract narrowing is undocumented in the usage comment.

- 🔵 **Safety**: Orphaned `|| echo '.accelerator/tmp'` fallback in `run.sh` becomes permanently unreachable
  **Location**: Phase 3: bash consumer scripts table — `inventory-design/scripts/playwright/run.sh` line 21
  The plan says to "drop only the default" on this line, leaving the `|| echo '.accelerator/tmp'` branch in place. After Phase 2, `config-read-path.sh tmp` always succeeds (exit 0) with the centralized default, so the `||` branch can never fire under normal conditions. It now silently masks real errors (e.g., script not found, sourcing failure in `config-defaults.sh`).

- 🔵 **Code Quality**: Structural grep pattern could produce false positives for backtick-quoted defaults
  **Location**: Phase 3: structural invariant test (INLINE_DEFAULT_PATTERN)
  The exclusion set `[^$"\n[:space:]]` catches `$` and `"` but not backtick `` ` ``. A SKILL.md call written as `` config-read-path.sh plans `meta/plans` `` would match `m` as the first character of the default argument, producing a false positive. Low risk given stable call syntax, but worth a comment explaining the exclusion choices.

- 🔵 **Test Coverage**: No-default test block omits the `templates` key
  **Location**: Phase 2: test additions — key sample selection
  `templates` is among the most-used keys and has a structurally different default path (`.accelerator/templates`, an `.accelerator/` prefix, not `meta/`). A typo in the Phase 1 array extension could go undetected by Phase 2 tests. Low risk if the Phase 1 content assertions are relied upon, but one additional test line would close the gap.

### Strengths

- ✅ The three-phase TDD ordering (extend data → fix resolver → strip consumers) mirrors the dependency graph and limits the blast radius of each phase to a single, clearly-bounded concern.
- ✅ The fix is correctly scoped entirely to `config-read-path.sh`; `config-read-value.sh` is untouched, preserving the existing module boundary.
- ✅ The backward-compatibility guarantee for callers passing explicit `$2` is correctly implemented (`if [ -n "${2:-}" ]`) and covered by both existing regression tests and the new explicit-override test in Phase 2.
- ✅ The Phase 3 structural grep test is a high-value architectural enforcement mechanism — the concept is right; the regex needs fixing, but the approach of encoding the end-state invariant as an executable assertion is exactly correct.
- ✅ The `PATH_KEYS[i]`/`PATH_DEFAULTS[i]` parallel-array index convention is preserved correctly in Phase 1, and the loop in Phase 2 reads paired entries at the same index, making the lookup logically sound.
- ✅ The exclusion rationale for `test-config.sh`, migration scripts, and `init.sh` line 34 is thorough and well-documented in the "What We're NOT Doing" section.
- ✅ The deferral of `DIR_KEYS`/`DIR_DEFAULTS` unification is well-reasoned and explicitly acknowledged, preventing scope creep.

### Recommended Changes

1. **Fix the structural invariant test regex to match quoted-path bash invocations** (addresses: structural invariant test regex gap finding)
   Update `INLINE_DEFAULT_PATTERN` to also match `"...config-read-path.sh"` forms. One approach: `'(config-read-path\.sh"|config-read-path\.sh)[[:space:]]+[a-zA-Z_][a-zA-Z0-9_]*[[:space:]]+[^$"\n[:space:]]'`. Also add handling for the multiline `jira-common.sh` case — either restructure the call to single-line before stripping, or add a dedicated assertion for that file. Apply the same fix to the AC greps in the Verification section.

2. **Update the `config-defaults.sh` governance comment** (addresses: governance comment finding)
   In the plan's Phase 2 steps, add an explicit step to update or remove the "Do not source this file directly" comment in `config-defaults.sh`, noting that `config-read-path.sh` sources it directly because it must avoid the VCS detection overhead pulled in by `config-common.sh`.

3. **Add a no-`$2` + configured-value test** (addresses: test coverage gap finding)
   Append to the Phase 2 test block: set `paths.work: docs/work-items` in a repo's config, call `bash "$READ_PATH" work` with no `$2`, and assert the output is `docs/work-items`.

4. **Address unknown-key silent empty output** (addresses: silent failure finding)
   Either add a guard to the `config-read-path.sh` body that emits a warning to stderr when `default` is still empty after the loop (key genuinely unknown), or explicitly document in the plan that the silent-empty contract is intentional and note which consumers depend on it. If a guard is added, the manual test step 3 should become an automated test.

5. **Add automated test for unknown-key empty-output behaviour** (addresses: manual-only contract finding)
   Add to the Phase 2 test block: `OUTPUT=$(cd "$REPO" && bash "$READ_PATH" unknown_key)` → `assert_eq "unknown key returns empty" "" "$OUTPUT"`.

6. **Show complete Before/After for `run.sh` in the consumer update table** (addresses: ambiguous table entry)
   Replace the `...` row with a full Before/After showing exactly which token is dropped and whether the `|| echo` fallback is also removed.

7. **Document the empty-string `$2` edge case in the usage comment** (addresses: contract narrowing)
   In the updated `config-read-path.sh` comment block, clarify that an absent or empty `[default]` both trigger the centralized lookup.

---

## Per-Lens Results

### Correctness

**Summary**: The plan is logically sound for its core mechanism: the Phase 2 lookup loop correctly matches `paths.${key}` against `PATH_KEYS` and the explicit-`$2`-overrides-lookup path is handled correctly. However, the structural invariant test introduced in Phase 3 has a regex that does not match the quoting style used in any of the 13 bash consumer scripts, meaning the test will pass immediately for bash files even before any Phase 3 bash changes are made. This leaves the bash consumer changes unenforced by the structural test. Additionally, the `config-defaults.sh` file comment 'Do not source this file directly' will become incorrect after Phase 2 makes `config-read-path.sh` source it directly, and the plan does not flag this comment for update.

**Strengths**:
- The Phase 2 lookup logic is correct: `paths.${key}` is built consistently, the loop iterates `PATH_KEYS` by index, `PATH_DEFAULTS[$i]` is accessed at the matched index, and the loop breaks on first match.
- The explicit-override guard `if [ -n "${2:-}" ]` correctly distinguishes absent/empty `$2` from a supplied non-empty `$2`, preserving full backward compatibility.
- Phase 1 correctly extends both `PATH_KEYS` and `PATH_DEFAULTS` in parallel, preserving the index-pairing invariant that Phase 2's loop depends on.
- The plan correctly identifies that `unknown_key` returns empty output — consistent with existing behavior.
- The plan correctly excludes `test-config.sh`, migration scripts, and variable-argument call sites from the consumer cleanup scope.

**Findings**:

- **Severity**: major | **Confidence**: high
  **Location**: Phase 3: Add structural invariant test
  **Title**: Structural invariant test regex does not match any bash consumer call pattern
  The structural test regex `config-read-path\.sh[[:space:]]+[a-zA-Z_][a-zA-Z0-9_]*[[:space:]]+[^$"\n[:space:]]` requires that the key argument immediately follows `config-read-path.sh` and whitespace. Every bash consumer script wraps the script path in double-quotes (e.g., `"$PLUGIN_ROOT/scripts/config-read-path.sh" tmp .accelerator/tmp`), so the character immediately after `config-read-path.sh` in source is `"` — not whitespace. The pattern never matches any bash `.sh` file. This means the structural test will report PASS for bash files even before Phase 3 changes are applied, providing no enforcement that the 13 bash consumer sites were actually updated.

- **Severity**: minor | **Confidence**: high
  **Location**: Phase 2: Update scripts/config-read-path.sh
  **Title**: config-defaults.sh comment 'Do not source this file directly' becomes incorrect
  The file `scripts/config-defaults.sh` contains the comment "Do not source this file directly — source config-common.sh instead." Phase 2 deliberately sources `config-defaults.sh` directly from `config-read-path.sh`, which is correct technically, but the change list does not include updating this comment.

- **Severity**: minor | **Confidence**: medium
  **Location**: Phase 3: Success Criteria / Automated Verification
  **Title**: AC verification grep commands also miss quoted-path bash consumer pattern
  The plan's Verification commands use the pattern `config-read-path\.sh [a-z_]+ [^$"\n]` for bash `.sh` files. Like the structural test regex, this does not account for bash consumers that use quoted paths. The AC commands will report "no output" (success) for `.sh` files even if all bash consumers still carry inline defaults.

---

### Test Coverage

**Summary**: The plan's TDD structure is solid: Phase 1 extends array assertions before adding entries, Phase 2 adds a targeted no-default lookup block before touching config-read-path.sh, and Phase 3's structural grep test permanently codifies the no-inline-default invariant. Two gaps stand out: the no-`$2` path with a config-set value is missing from the Phase 2 test block, and the unknown-key silent-empty-output behaviour is only documented in a manual step rather than automated.

**Strengths**:
- TDD ordering is enforced throughout: test additions precede implementation changes in every phase.
- The explicit-override regression path is well-covered by both the existing test block and the new explicit-override test in Phase 2.
- Phase 3's structural grep test is a high-value persistent invariant that will catch future regressions.
- The plan correctly identifies and preserves test-config.sh explicit-default regression tests.
- The representative sample in Phase 2 deliberately includes keys from both array groups.

**Findings**:

- **Severity**: major | **Confidence**: high
  **Location**: Phase 2: test additions (`=== config-read-path.sh (no-default lookup) ===`)
  **Title**: No test for no-`$2` + config-set value combination
  Every Phase 2 test calls `bash "$READ_PATH" <key>` against a repo with no config. There is no test for the case where `$2` is absent but the key IS configured in `.accelerator/config.md`. If a quoting or delegation bug causes the resolved default to shadow a configured value, no test catches it.

- **Severity**: minor | **Confidence**: high
  **Location**: Testing Strategy: Manual Testing Steps (step 3)
  **Title**: Unknown-key silent-empty-output behaviour is manual-only
  Manual step 3 specifies empty output for an unknown key — a defined contract of the new implementation — but there is no automated assertion. A future change that introduces a default string or non-zero exit for unknown keys would not be caught.

- **Severity**: minor | **Confidence**: medium
  **Location**: Phase 3: structural invariant test (INLINE_DEFAULT_PATTERN)
  **Title**: Structural grep pattern may miss multi-line or quoted call sites
  The pattern performs single-line matching. The `jira-common.sh` call at lines 73-74 uses a line continuation with the default on line 74. The grep invariant would not catch this call site if the edit were accidentally omitted or partially applied.

- **Severity**: minor | **Confidence**: medium
  **Location**: Phase 2: test additions — key sample selection
  **Title**: No-default test block does not cover the `templates` key
  `templates` (default `.accelerator/templates`) is widely used and has a structurally different default path prefix than the other tested keys. A typo in the Phase 1 array extension could go undetected by Phase 2 tests.

---

### Code Quality

**Summary**: The plan is well-structured, clearly reasoned, and applies TDD discipline throughout. The proposed `config-read-path.sh` implementation is simple, readable, and proportionate to the problem. One code smell requires attention: the plan proposes sourcing `config-defaults.sh` directly but leaves intact the file's "Do not source this file directly" header comment, creating a contradictory convention signal for future maintainers.

**Strengths**:
- The loop-based default lookup in `config-read-path.sh` is minimal and readable — 7 lines, single responsibility, clear exit condition.
- The plan preserves backward compatibility by treating explicit `$2` as an override.
- The structural grep test in Phase 3 is a well-chosen mechanical invariant.
- The phased TDD approach is correctly scoped, with a single failing condition per phase.
- The "What We're NOT Doing" section is explicit and well-reasoned.

**Findings**:

- **Severity**: major | **Confidence**: high
  **Location**: Phase 2: Update `scripts/config-read-path.sh`
  **Title**: Sourcing `config-defaults.sh` directly contradicts its own header comment
  `config-defaults.sh` line 19 explicitly states: "Do not source this file directly — source config-common.sh instead." Phase 2 adds `source "$SCRIPT_DIR/config-defaults.sh"` in `config-read-path.sh`, but the plan does not include updating that comment. A future maintainer reading `config-defaults.sh` may "fix" it by switching to `config-common.sh`, introducing VCS detection overhead into a lightweight path-resolution utility.

- **Severity**: minor | **Confidence**: medium
  **Location**: Phase 3: structural invariant test (grep pattern)
  **Title**: Structural grep pattern excludes backtick terminator — could produce false positives
  The exclusion set `[^$"\n[:space:]]` does not exclude backtick. A SKILL.md call written as `` config-read-path.sh plans `meta/plans` `` would match `m` as the first character of the default argument. Low risk given stable call syntax, but a brief comment explaining the exclusion choices would aid maintainability.

- **Severity**: minor | **Confidence**: high
  **Location**: Phase 3: bash consumer scripts table — `inventory-design/scripts/playwright/run.sh` line 21
  **Title**: Consumer update table says "drop only the default" — ambiguous instruction for multi-argument call
  This is the only row that ends with `...` rather than showing the complete After string. Unlike every other entry, the implementer must independently determine which arguments to preserve and whether to remove the `|| echo` fallback.

---

### Architecture

**Summary**: The plan establishes a clean single-source-of-truth for path defaults and eliminates a broad class of duplication with a well-phased TDD approach. The structural invariant test in Phase 3 is a strong architectural enforcement mechanism. One significant issue exists: Phase 2 proposes sourcing `config-defaults.sh` directly from `config-read-path.sh`, which directly contradicts the governance comment on `config-defaults.sh` itself, creating a split in the module's access contract without resolving it.

**Strengths**:
- The three-phase TDD structure mirrors the dependency graph cleanly.
- The fix is correctly scoped entirely to `config-read-path.sh`, leaving `config-read-value.sh` untouched and respecting existing module boundaries.
- The Phase 3 structural grep test encodes an architectural invariant in the test suite itself.
- The explicit exclusion of `test-config.sh` and migration scripts correctly distinguishes regression harnesses from production consumers.
- The deferral of `DIR_KEYS`/`DIR_DEFAULTS` unification is well-reasoned and explicitly acknowledged.
- The backward-compatible design means all existing callers outside scope continue to work.

**Findings**:

- **Severity**: major | **Confidence**: high
  **Location**: Phase 2: Update `scripts/config-read-path.sh`
  **Title**: Direct sourcing of `config-defaults.sh` contradicts its own governance contract
  `config-defaults.sh` carries an explicit governance comment: "Do not source this file directly — source config-common.sh instead." Phase 2 sources `config-defaults.sh` directly. The plan acknowledges this in "Key Discoveries" but does not update the governance comment or explain why the contract should be relaxed for this caller. The plan should include an explicit step to update this comment.

- **Severity**: minor | **Confidence**: medium
  **Location**: Phase 2: Success Criteria / Testing Strategy
  **Title**: No automated assertion for unknown-key empty-output behaviour
  Manual test step 3 describes the expected empty-output behaviour for an unrecognised key, but there is no corresponding automated assertion. This is the silent-fallback boundary of the new loop, and its correctness is structurally important.

---

### Compatibility

**Summary**: The plan's backward-compatibility guarantee for callers passing explicit `$2` defaults is correctly preserved for all practical call patterns. However, the structural invariant test used to verify consumer cleanup has a regex gap that means it only catches SKILL.md consumers and provides no enforcement over the 13 bash script consumers. There is also a minor contract tension in the proposed direct sourcing of `config-defaults.sh`.

**Strengths**:
- The plan preserves full backward compatibility for all documented caller patterns.
- The explicit-override test case is included in the Phase 2 test block.
- `test-config.sh` regression tests are explicitly excluded from cleanup.
- The `init.sh` line 34 exclusion is well-reasoned.
- Migration scripts are excluded with a clear rationale.

**Findings**:

- **Severity**: major | **Confidence**: high
  **Location**: Phase 3: Structural invariant test (Step 1)
  **Title**: Structural grep test pattern misses all bash consumer call sites
  The pattern `config-read-path\.sh[[:space:]]+[a-zA-Z_]...` requires whitespace after `.sh`. Every bash consumer uses a quoted path form — `"$PLUGIN_ROOT/scripts/config-read-path.sh" tmp .accelerator/tmp` — where `"` follows `.sh`, not whitespace. The pattern therefore never matches any bash script call site. The same gap affects the AC grep in the Verification section. All 13 bash consumer scripts could be left un-updated and the structural test would still pass.

- **Severity**: minor | **Confidence**: high
  **Location**: Phase 2: Update `scripts/config-read-path.sh`
  **Title**: Direct sourcing of `config-defaults.sh` violates its own "Do not source directly" directive
  The directive becomes stale documentation. Future contributors relying on it when considering changes to `config-defaults.sh` (e.g., adding a sourcing dependency) might introduce a circular or broken sourcing chain without an obvious failure signal.

- **Severity**: minor | **Confidence**: medium
  **Location**: Phase 2: Update `scripts/config-read-path.sh` (proposed code)
  **Title**: Explicit empty-string `$2` argument changes behaviour silently
  The guard `if [ -n "${2:-}" ]` treats `bash config-read-path.sh plans ""` the same as `bash config-read-path.sh plans`, triggering the centralized lookup. Current behaviour passes the empty string through as the default. No current consumer passes `""`, but the contract narrowing is undocumented.

---

### Safety

**Summary**: The plan is a well-scoped refactor of a developer tooling system with low blast radius — failures affect local repository scaffolding and visualiser launches, not production data. The TDD phasing and structural invariant test provide solid safety netting. Two concrete risks warrant attention: silent empty-string resolution when an unknown or typo'd key is passed after callers drop their explicit defaults, and the structural grep invariant test's inability to catch the multiline `jira-common.sh` call site if it were to contain a missed or malformed edit.

**Strengths**:
- TDD phasing ensures each change is independently verified before the next phase begins.
- The structural invariant grep test provides a durable enforcement mechanism for future regressions.
- Explicit backward-compatibility preservation means un-updated callers remain safe throughout.
- The "What We're NOT Doing" section explicitly carves out migration scripts.
- Phase 3 manual verification steps include a smoke test against a project with no config.

**Findings**:

- **Severity**: major | **Confidence**: high
  **Location**: Phase 2: `config-read-path.sh` implementation; Phase 3: bash consumer scripts
  **Title**: Unknown key silently resolves to empty string with no caller-visible error
  After Phase 3 strips hardcoded defaults from all consumers, a typo'd or unregistered key returns empty string with exit 0. For example, `launch-server.sh` uses the result in `cd "$PROJECT_ROOT/$TMP_REL"` — an empty `TMP_REL` resolves to the project root, and subsequent `mkdir -p` or `rm -rf` operations would target the wrong directory. The current explicit-default style gave callers a safety net; the new style removes it without a server-side guard.

- **Severity**: major | **Confidence**: high
  **Location**: Phase 3: Structural invariant test; bash consumer scripts table — `jira-common.sh` lines 73-74
  **Title**: Structural invariant grep cannot verify the multiline `jira-common.sh` call site
  The `jira-common.sh` call is split across a line continuation — script name on line 73, default `.accelerator/state/integrations` on line 74. Single-line grep never sees both tokens together. If this edit is omitted or partially applied, the structural test still passes.

- **Severity**: minor | **Confidence**: high
  **Location**: Phase 2: `config-read-path.sh` — `source config-defaults.sh` directly
  **Title**: Direct sourcing of `config-defaults.sh` contradicts its own "Do not source directly" convention
  The contradictory comment is a trap for future contributors who might refactor `config-read-path.sh` to source `config-common.sh` instead, inadvertently introducing `find_repo_root` and VCS detection overhead into every path resolution call.

- **Severity**: minor | **Confidence**: medium
  **Location**: Phase 3: bash consumer scripts table — `inventory-design/scripts/playwright/run.sh` line 21
  **Title**: Orphaned `|| echo '.accelerator/tmp'` fallback in `run.sh` becomes permanently unreachable
  The plan says to "drop only the default," leaving the `|| echo '.accelerator/tmp'` branch in place. After Phase 2, `config-read-path.sh tmp` always succeeds with the centralized default, so the `||` branch can never fire under normal conditions. It now silently masks real errors (e.g., script not found, sourcing failure in `config-defaults.sh`).

---
*Review generated by /review-plan*

## Re-Review (Pass 2) — 2026-05-08

**Verdict:** COMMENT

All 10 findings from pass 1 are resolved or addressed. The two cross-cutting themes (structural test regex gap; governance comment contradiction) are fully resolved. Architecture and compatibility returned clean — no new findings. One major new issue was introduced by the edits: the Phase 2 `config-set value` test fixture has two bugs that would cause it to fail at setup rather than test the intended behaviour. Three minor issues were also found.

### Previously Identified Issues

- ✅ **Correctness / Compatibility / Test Coverage / Safety**: Structural invariant test regex — **Resolved**. Pattern updated to `config-read-path\.sh"?[[:space:]]+...` covering both invocation styles; dedicated JIRA_MATCHES check added for the multiline `jira-common.sh` case.
- ✅ **Code Quality / Architecture / Compatibility / Safety**: Governance comment contradiction — **Resolved**. Phase 2 Step 3 explicitly rewrites the `config-defaults.sh` line 19 comment.
- ✅ **Test Coverage**: No test for no-`$2` + config-set value — **Resolved** (with a new implementation defect, see below).
- ✅ **Safety**: Unknown key silent empty output — **Resolved**. `config-read-path.sh` now emits a stderr warning; an automated test covers the case.
- ✅ **Test Coverage / Architecture**: Unknown-key empty-output manual-only — **Resolved**. Automated assertion added with `2>/dev/null`.
- ✅ **Code Quality**: `run.sh` table entry ambiguous — **Resolved**. Complete Before/After now shown including removal of the `|| echo` fallback.
- ✅ **Compatibility**: Empty-string `$2` contract change undocumented — **Resolved**. Documented in both the usage comment and "What We're NOT Doing".
- ✅ **Safety**: Orphaned `|| echo` fallback in `run.sh` — **Resolved**. Phase 3 table now explicitly drops it.
- ✅ **Code Quality**: Backtick exclusion gap — **Resolved** (acknowledged). Inline comment added explaining the limitation.
- ✅ **Test Coverage**: `templates` key missing from test sample — **Resolved**. Test added.

### New Issues Introduced

- 🟡 **Correctness / Test Coverage**: Config-set value test fixture is broken — missing `mkdir -p` and wrong YAML format.
  The new test appends `paths.work: docs/work-items` to `$REPO/.accelerator/config.md` without first creating `.accelerator/`. Under `set -euo pipefail` this aborts the entire test runner. Additionally the flat dotted-key format (`paths.work: value`) is not what the config parser reads — the existing suite uses nested YAML (`paths:\n  work: value`). Both issues would cause the test to fail at setup rather than assert the intended behaviour.

- 🔵 **Code Quality**: Unknown-key test missing `|| true` guard — if `config-read-value.sh` exits non-zero for an unknown key with empty default, the command substitution propagates the non-zero exit and `set -e` aborts the runner.

- 🔵 **Safety**: `JIRA_MATCHES` check passes vacuously if `jira-common.sh` is absent or relocated — `|| true` in the grep means a missing file silently produces no failure.

- 🔵 **Code Quality**: Structural test comment slightly misdescribes the backtick exclusion (says "not matched" but the pattern actually does match defaults *followed by* a backtick; the gap is only for defaults *enclosed in* backticks).

### Assessment

The plan is in good shape. All the structural concerns from pass 1 are resolved. One major item needs a quick plan fix before implementation: the `config-set value` test fixture needs `mkdir -p` and correct nested YAML format. The three minors are low risk but straightforward to address.
