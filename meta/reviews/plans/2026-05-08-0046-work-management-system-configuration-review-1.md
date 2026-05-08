---
date: "2026-05-08T22:57:42+00:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-08-0046-work-management-system-configuration.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability, documentation]
review_pass: 2
status: complete
---

## Plan Review: 0046 Work Management System Configuration

**Verdict:** REVISE

The plan is structurally sound: it applies a well-established centralisation
pattern (mirror of paths.* / templates.*), sequences six phases along the
natural dependency graph, and uses strict TDD with structural regression
guards to lock in invariants. However, the same patterns the plan claims to
mirror are diverged from in three quietly load-bearing places (WORK_KEYS
storage form, unknown-key handling, and the silent-error idiom in the new
helper), and the AC5 warning is bound to the Jira-specific consumer in a
way that will not survive the addition of Linear/Trello/GitHub-Issues
stories. There is also notable documentation drift risk because the canonical
edits in `configure/SKILL.md` are not extended to the README or the
integration-skill docs.

### Cross-Cutting Themes

- **WORK_KEYS / unknown-key behaviour diverges from `config-read-path.sh`
  while claiming to mirror it** (flagged by: architecture, code-quality,
  correctness, standards) — `PATH_KEYS` stores fully-dotted entries; the
  plan uses bare keys and labels this a "match". The wrapper also exits
  early on unknown keys, where `config-read-path.sh` warns and still
  delegates with empty default. Two real divergences hidden behind a
  "mirrors exactly" claim.
- **Hard-fail validation is silently swallowed by `2>/dev/null) || var=""`
  idiom** (flagged by: correctness, code-quality, usability) — Phase 3's
  `jira_resolve_default_project` reads the integration key with the
  defensive idiom. After Phase 2 introduces hard-fail, this idiom hides
  both AC4 (the error) and AC5 (the warning, because integration becomes
  empty and the cross-key guard fails). The most common real-world failure
  path (typo'd `work.integration`) is silently degraded.
- **AC5 warning lives under `skills/integrations/jira/`, making it
  per-integration rather than a property of the config system** (flagged
  by: architecture, correctness, usability) — Linear/Trello/GitHub-Issues
  consumers (stories 0048+) will need to re-implement the helper or AC5
  silently regresses. Even within Jira, only `search` and `create` flows
  call it; `update`, `comment`, `transition`, `attach` do not.
- **`jira_plugin_scripts_dir` is invented when `_JIRA_PLUGIN_ROOT` already
  exists** (flagged by: architecture, code-quality, standards) — the plan
  asks the implementer to introduce a path helper, but
  `jira-common.sh:42` already defines `_JIRA_PLUGIN_ROOT` and the rest of
  the file uses it. Two parallel idioms in the same file is exactly what
  the centralisation pattern is meant to avoid.
- **Documentation surface is too narrow** (flagged by: documentation,
  usability) — `README.md` references work.* keys in three places, the
  configure SKILL.md lead-in still says "Two keys", and integration skill
  docs (init-jira, create-jira-issue) say nothing about the new key. The
  natural search path doesn't lead users to the new docs.
- **Dump shows misconfigured values without flagging them** (flagged by:
  architecture, correctness, usability) — Phase 5 deliberately uses
  `config-read-value.sh` so the dump never crashes on bad input, but it
  also doesn't annotate invalid enum values. Users running
  `/accelerator:configure view` to debug see a clean-looking row even
  when their `work.integration` typo is the bug.

### Tradeoff Analysis

- **Strict-fail validation vs defensive consumer idiom**: Phase 2's
  `log_die` on invalid `work.integration` is the right call for AC4
  ("informative error"), but the plan also preserves the established
  `2>/dev/null) || x=""` defensive style at every consumer. The two
  policies fight each other — one decision must change. Recommended:
  drop `2>/dev/null` (and the `||` fallback) at integration-consumer call
  sites that read the `integration` key, where the validation is supposed
  to fire. Keep the defensive style only on local-only `id_pattern` /
  `default_project_code` reads where validation never triggers.
- **Centralised AC5 warning vs targeted consumer warning**: putting the
  warning in `config-read-work.sh` would fire on every read (noise);
  putting it in `jira_resolve_default_project` makes it Jira-only. The
  plan correctly chose the latter for noise reasons but accepted
  per-integration duplication in return. A middle ground — extracting the
  warning into a generic `work_resolve_default_project` in
  `scripts/work-common.sh` (or similar) — keeps the warning targeted
  while sharing it across integrations. Recommended pre-implementation.

### Findings

#### Critical
- 🔴 **Correctness**: Hard-fail validation is silently swallowed by `2>/dev/null) || var=""` idiom
  **Location**: Phase 3, Section 1: `jira_resolve_default_project` (and Phase 4 consumer migration)
  After Phase 2's `log_die`, `integration=$("$read_work" integration 2>/dev/null) || integration=""` suppresses both stderr and the non-zero exit. With `work.integration: jura` (typo) and no default project, the user gets `E_CREATE_NO_PROJECT` with no diagnostic — neither AC4's error nor AC5's warning surfaces.

#### Major
- 🟡 **Correctness**: Unknown-key behaviour diverges from the claimed mirror of `config-read-path.sh`
  **Location**: Phase 1, Section 2: config-read-work.sh wrapper
  Phase 1 exits 0 with empty stdout on unknown keys; `config-read-path.sh:37-42` warns and still delegates. A user who hand-sets `work.foo: bar` would silently get empty rather than `bar`.

- 🟡 **Correctness**: AC5 warning fires only in two Jira flows, not on every integration skill
  **Location**: Phase 3 — overall scope of AC5
  AC5 says "when a developer invokes an integration skill" — but show / update / comment / transition / attach never call `jira_resolve_default_project`, and Linear/Trello/GitHub-Issues are vacuously unsatisfied.

- 🟡 **Standards**: `WORK_KEYS` bare-key form diverges from `PATH_KEYS` dotted-key convention
  **Location**: Phase 1, Section 1: Centralise work.* defaults
  `PATH_KEYS=("paths.plans" …)` is dotted; `TEMPLATE_KEYS` is dotted; the plan stores `WORK_KEYS=(integration …)` bare and claims this matches the precedent. It does not.

- 🟡 **Test Coverage**: AC1 regression guard is fragile and prone to silent drift
  **Location**: Phase 6, Section 2: external-call regression guard
  The grep pattern `\b(curl|wget)\b|jira-(auth|api)\.sh|jira_(curl|api|auth)` is hardcoded today and explicitly described as needing tuning at implementation time. Future integrations (linear-api.sh, etc.) will silently bypass it.

- 🟡 **Test Coverage**: No end-to-end test that the Phase 3 refactor preserves jira-create / jira-search behaviour
  **Location**: Phase 3 Tests
  Only structural assertions and helper-isolation tests exist. A subtle helper change (warning to stdout instead of stderr, or non-zero exit on empty) would silently break the consuming flow's exit-code contract.

- 🟡 **Test Coverage**: Inline-default regex won't catch line-continuation invocations
  **Location**: Phase 4 consumer-migration tests
  `INLINE_DEFAULT_PATTERN` requires key + default on the same line; the very call sites being migrated use multiline `\` continuations. The path-equivalent test had to special-case this. Future regressions back to multiline form would pass silently.

- 🟡 **Test Coverage**: Missing precedence and explicit-empty tests
  **Location**: Phase 1 Tests
  No `local override of work.default_project_code` test; no explicit-empty-string-vs-missing test, despite the plan flagging empty-vs-missing semantics as load-bearing for AC5.

- 🟡 **Test Coverage**: Dump ordering and mixed-source attribution under-specified
  **Location**: Phase 5 Tests
  Success criterion claims "in that order" but no enumerated test verifies row ordering. No mixed-source test (integration local, id_pattern team, default_project_code default) exists.

- 🟡 **Usability**: Error message names valid values but not the file to fix
  **Location**: Phase 2 enum validation
  `Error: work.integration must be one of: jira, linear, trello, github-issues (got '${value}')` — no breadcrumb to `.accelerator/config.md` or `/accelerator:configure view`. The AC5 warning text is more helpful and is the model to follow.

- 🟡 **Usability**: AC5 warning is Jira-only; future integrations will silently regress
  **Location**: Phase 3 helper location
  Promoting the helper into `scripts/` (or a generic `work-common.sh`) would let all integrations share it.

- 🟡 **Documentation**: README.md references work.* keys in three places but is not updated
  **Location**: Phase 6 — scope of doc updates
  README.md:122, 280, 342 mention `work.id_pattern` / `work.default_project_code`. New readers landing on the README will not learn that `work.integration` exists.

- 🟡 **Documentation**: Configure SKILL.md lead-in still says "Two keys" after a third row is added
  **Location**: Phase 6, Section 1A
  Line 429 reads "Two keys are recognised:" — adding a third row creates immediate self-contradiction with the table directly below it.

- 🟡 **Documentation**: `work.integration` not discoverable from integration-skill docs
  **Location**: Phase 6 — doc reach
  `init-jira/SKILL.md`, `create-jira-issue/SKILL.md`, `search-jira-issues/SKILL.md` say nothing about it. The natural user search path runs into a wall.

#### Minor
- 🔵 **Architecture**: Plan invents `jira_plugin_scripts_dir` when `_JIRA_PLUGIN_ROOT` already exists at jira-common.sh:42.
  **Location**: Phase 3, Section 1

- 🔵 **Architecture**: Wrapper diverges from "thin wrapper" pattern (filter-style vs redirect-style); not documented as such.
  **Location**: Phase 1 vs Phase 2

- 🔵 **Architecture**: Allowed-integration enum hardcoded inline in `config-read-work.sh`, not in the central registry.
  **Location**: Phase 2

- 🔵 **Architecture**: Allowed-tools audit is conditional ("if a broad glob is in place, no change required") and may leave skills broken if a SKILL.md is missed.
  **Location**: Phase 4, Section 3

- 🔵 **Code Quality**: Migration-invariant regexes (`INLINE_DEFAULT_PATTERN`, `STALE_PATTERN`) are write-only — dense, double-escaped, hard to verify by inspection.
  **Location**: Phase 4 tests

- 🔵 **Code Quality**: Unknown-key path exits 0 with empty stdout — silent in `set -euo pipefail` consumers; programmer typos vanish.
  **Location**: Phase 1, Section 2

- 🔵 **Code Quality**: Phase 2 rewrites Phase 1's final line from `exec` to capture+echo — a small semantic shift (exit-code propagation) that warrants an explicit checkpoint test.
  **Location**: Phase 1 → Phase 2

- 🔵 **Standards**: Recognised-keys allow-list stays prose; not declaration-driven from `WORK_KEYS`. Will drift the next time the registry grows.
  **Location**: Phase 6, Section 1C

- 🔵 **Standards**: `allowed-tools` audit can be reduced to a single test asserting all SKILL.md invocations are permitted, rather than a per-file checklist.
  **Location**: Phase 3, Section 3 + Phase 4, Section 3

- 🔵 **Standards**: Five-array test message duplicates literal array names — both regex and assert message will eventually disagree.
  **Location**: Phase 1, Section 3 tests

- 🔵 **Correctness**: `WORK_KEYS` bare-key claim (architecture-level note in plan text) is technically incorrect about PATH_KEYS storage form.
  **Location**: Phase 1, Section 1

- 🔵 **Correctness**: `write-visualiser-config.sh` defensive `|| echo "{number:04d}"` tail's stated rationale is wrong (validation is scoped to integration; id_pattern reads cannot trip Phase 2 validation).
  **Location**: Phase 4, Section 1

- 🔵 **Correctness**: Dump uses `config-read-value.sh` to bypass validation but consumers can still hard-fail later — view output disagrees with skill behaviour about whether config is valid.
  **Location**: Phase 5

- 🔵 **Test Coverage**: `WORK_KEYS` structural test should pin index alignment with `WORK_DEFAULTS`, not just length+contents-string.
  **Location**: Phase 1 tests

- 🔵 **Test Coverage**: AC2 has no dedicated test — relies on existing unconditional fallback behaviour that no test pins positively.
  **Location**: Plan Overview / What We're NOT Doing

- 🔵 **Test Coverage**: AC3 has no dedicated automated test — covered only by the AC1 regression guard.
  **Location**: Phase 6

- 🔵 **Test Coverage**: Stderr-content assertion for valid-values list is over-loose (greps for the comma-separated string verbatim; brittle to formatting refactor).
  **Location**: Phase 2 tests

- 🔵 **Test Coverage**: Phase 3 test isolation under-specified (sourcing jira-common.sh in subshell with `_JIRA_PLUGIN_ROOT` resolved via path math; needs an explicit fixture pattern).
  **Location**: Phase 3 tests

- 🔵 **Test Coverage**: SKILL.md inline bash snippets are not parse-checked or executed; structural tests miss runtime errors.
  **Location**: Phase 4

- 🔵 **Usability**: Local-first paragraph is buried inside the work.id_pattern adjacent prose; users searching for "does this plugin auto-call Jira?" won't find it.
  **Location**: Phase 6, Section 1B

- 🔵 **Usability**: Invalid `work.integration` value appears verbatim in dump with no marker.
  **Location**: Phase 5

- 🔵 **Usability**: Team-vs-local override precedence not explained in user-facing docs even though manual test step 2 exercises it.
  **Location**: Phase 6 docs

- 🔵 **Usability**: Unknown-key warns / unknown-value errors — same family, asymmetric severity, undocumented in wrapper header.
  **Location**: Phase 1/2

- 🔵 **Documentation**: Local-first paragraph paraphrases AC1 but omits which skills the invariant applies to.
  **Location**: Phase 6, Section 1B

- 🔵 **Documentation**: Phase 6 Section 3 (status: ready → in-progress → done) is process, not documentation; bloats the phase.
  **Location**: Phase 6, Section 3

#### Suggestions
- 🔵 **Architecture**: Dump could call `config-read-work.sh` with stderr+exit captured and surface invalid values with `(invalid: …)` annotation.
- 🔵 **Architecture**: External-call regression guard could be inverted (assert local skills source no `skills/integrations/` paths) instead of pattern-matching.
- 🔵 **Code Quality**: `jira_resolve_default_project` could split warn/value-resolution into two helpers, or gate warn on a once-per-process flag.
- 🔵 **Standards**: Could extract `WORK_INTEGRATION_VALUES` array into `config-defaults.sh` for single-definition-site discipline on the enum.
- 🔵 **Test Coverage**: Test count headline (~43) doesn't reconcile with per-phase enumeration (~49); tighten or remove.
- 🔵 **Usability**: AC5 warning could append `or run /accelerator:init-jira` when integration is jira (mild integration-coupling tradeoff).
- 🔵 **Documentation**: Wrapper header could mention that cross-key warnings are emitted by integration consumers, not by the wrapper itself.
- 🔵 **Documentation**: ADR-0016/0017 references could be annotated with their bearing on this plan, matching the style of the centralisation/validation precedent entries above them.

### Strengths
- ✅ Phase ordering follows the natural dependency graph (registry → reader → validation → consumer migration → dump → docs); each phase is independently reviewable.
- ✅ Validation scoping (only on `integration` key) is explicitly justified with positive regression-guard tests — protects local-only skills from breaking on a misconfigured integration value.
- ✅ AC5 cross-key warning placement at the consumer (not the reader) is the right call to avoid noise on every read.
- ✅ Init-flow exclusion from the AC5 warning helper is correctly identified — its job is to *set* the value, so a pre-init warning would be perpetual noise.
- ✅ Phase 5 deliberately uses `config-read-value.sh` in the dump so a misconfigured value renders rather than crashes diagnostics — correct separation of concerns.
- ✅ Structural regression guards (no-inline-default, no-stale-references, local-skills-no-external-calls) lock in invariants without requiring reviewer vigilance.
- ✅ Strong DRY win: factoring the duplicate fallback at jira-search-flow.sh:207 / jira-create-flow.sh:179 into a single helper is a textbook DRY case.
- ✅ Error-handling categorisation is appropriate: `log_die` for unrecoverable enum errors (AC4), `log_warn` for recoverable cross-key advisory (AC5), with rationale recorded.
- ✅ Plan frontmatter, test section naming (`=== … ===`), log_die/log_warn use, and the centralisation pattern in `config-defaults.sh` all conform to existing conventions.
- ✅ The new `### work` section in `configure/SKILL.md` correctly lifts AC1 into reader-facing prose.

### Recommended Changes

1. **Drop `2>/dev/null) || x=""` from integration-key reads in `jira_resolve_default_project`** (addresses: Critical correctness finding)
   In Phase 3, change `integration=$("$read_work" integration 2>/dev/null) || integration=""` to `integration=$("$read_work" integration)` (let stderr propagate; let `set -euo pipefail` propagate the non-zero exit). Add a Phase 3 test that asserts an invalid `work.integration` value produces a visible AC4 error when reached via the helper. Keep the defensive idiom only on `default_project_code` reads where validation never triggers.

2. **Align unknown-key behaviour with `config-read-path.sh`** (addresses: Correctness finding on divergence + Standards finding + Architecture finding)
   In Phase 1, change `config-read-work.sh` to `exec "$SCRIPT_DIR/config-read-value.sh" "work.${key}" ""` after the warning, matching `config-read-path.sh:37-42` exactly. Update the plan's "mirrors exactly" claim. Adjust the Phase 1 test from `unknown work.* key -> exit 0 with empty stdout` to `unknown work.* key -> warning + delegated read with empty default`.

3. **Either dot WORK_KEYS or document the divergence explicitly** (addresses: Standards major finding)
   Preferred: store `WORK_KEYS=("work.integration" "work.id_pattern" "work.default_project_code")` to match PATH_KEYS exactly, and update Phase 5's loop and Phase 1's wrapper accordingly. Alternative: keep bare keys, but rewrite the plan's claim from "matches the convention" to "deliberately diverges, because…" and add a `config-defaults.sh` scope-note comment.

4. **Promote the AC5 warning helper above the Jira directory** (addresses: Architecture cross-cutting + Correctness AC5 scope finding + Usability future-integration regression finding)
   Move `resolve_default_project` (or just its warn portion) into a generic `scripts/work-resolve-project.sh` (or function in a new `scripts/work-common.sh`) so Linear/Trello/GitHub-Issues consumers share it. If full extraction is too much for this story, document explicitly in "What We're NOT Doing" that AC5 fires only for project-scoped Jira flows and add a regression-guard test that any new `skills/integrations/<x>/` directory must invoke the warning helper.

5. **Reuse `_JIRA_PLUGIN_ROOT` instead of inventing a new helper** (addresses: Architecture/Code-Quality/Standards minor cross-cutting finding)
   In Phase 3, change the prose from "introduce one if none exists" to "use `_JIRA_PLUGIN_ROOT` (already defined at jira-common.sh:42)". The new helper line becomes `read_work="$_JIRA_PLUGIN_ROOT/scripts/config-read-work.sh"`.

6. **Strengthen the Phase 2 error message with file pointer** (addresses: Usability major finding)
   Update the `log_die` text to: `Error: work.integration must be one of: jira, linear, trello, github-issues (got '${value}'). Update work.integration in .accelerator/config.md or run '/accelerator:configure view' to inspect the current value.`

7. **Replace the AC1 regression-guard pattern with a structural inverse assertion** (addresses: Test Coverage major finding + Architecture suggestion)
   Instead of grepping for current-known external-call entry points, assert that no file under `skills/work/<seven local skills>/` references any path under `skills/integrations/` or sources any `*-api.sh` / `*-auth.sh` file. The test grows automatically as new integrations land.

8. **Extend the inline-default invariant regex (or use awk to join continuations)** (addresses: Test Coverage major finding on multiline blind-spot)
   The current regex misses line-continuation calls — exactly the shape currently used in jira-create-flow.sh:179, jira-init-flow.sh:170. Either extend the regex to catch trailing `\` and check the next line, or pre-process via awk before grepping.

9. **Add a behavioural regression test for Phase 3 refactor** (addresses: Test Coverage major finding on Jira flow continuity)
   Add explicit Phase 3 success-criterion checkboxes for `bash skills/integrations/jira/scripts/test-jira-create.sh` Case 2 and an analogous case in `test-jira-search.sh` (positive AC2 path: integration set + project set, no warning, default used). Without these, a helper-internals change can break exit-code semantics silently.

10. **Add precedence + empty-string tests in Phase 1** (addresses: Test Coverage major finding)
    Add `Test: local override of work.default_project_code wins over team` and `Test: work.integration explicitly set to empty string -> empty value, no error, no warning`. Pin the empty-vs-missing contract that AC5 leans on.

11. **Add ordering and mixed-source-attribution tests in Phase 5** (addresses: Test Coverage major finding)
    Add `Test: work.* rows appear in WORK_KEYS declaration order` and `Test: mixed source attribution (integration local, id_pattern team, default_project_code default) — each row independent`.

12. **Annotate invalid `work.integration` values in dump output** (addresses: Architecture suggestion + Correctness minor + Usability minor cross-cutting)
    Validate the integration value cheaply against the central enum during the dump's WORK_KEYS loop and append `(invalid: must be jira, linear, trello, github-issues)` to the value cell when it does not match. Keeps dump non-fatal but makes typos discoverable at the diagnostic surface.

13. **Update README.md and integration-skill SKILL.md docs in Phase 6** (addresses: Documentation major findings on README + integration-skill discoverability)
    Add a Phase 6 step to extend README.md's existing `work.id_pattern` mentions with one sentence about `work.integration` and the local-first invariant. Add a short "Configuration" note to `init-jira/SKILL.md`, `create-jira-issue/SKILL.md`, `search-jira-issues/SKILL.md` linking to the `### work` section.

14. **Fix the configure SKILL.md lead-in** (addresses: Documentation major finding)
    In Phase 6 Section 1A, also rewrite line 429 from "Two keys are recognised:" to something like "Configure work-item identifiers and the active remote tracker. Three keys are recognised:".

15. **Move the work-item status update out of Phase 6** (addresses: Documentation minor finding)
    The `status: ready → in-progress → done` transition is process bookkeeping, not documentation. Either move it to a brief "Adoption" note or remove it (if `/accelerator:` workflow tooling already handles it).

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan applies a well-established centralisation pattern (mirror of paths.* / templates.*) to the work.* config family with thoughtful sequencing across six phases. The structural decisions — wrapper-per-domain, scoped enum validation, integration-aware warning at the consumer (not the reader) — are sound and explicitly defended against alternatives in the research. Minor concerns exist around divergence from the path-wrapper pattern, fragile invariant maintenance, and a few inaccurate references to non-existent helpers.

**Strengths**:
- Phase ordering follows the natural dependency graph; each phase is independently reviewable.
- Validation scoping is explicitly justified — protects local-only skills from breaking on a misconfigured integration value.
- Cross-key warning correctly placed at the integration consumer (jira_resolve_default_project) rather than inside config-read-work.sh.
- Init flow is explicitly excluded from the warning helper.
- Phase 5 deliberately uses config-read-value.sh in dump (not the validating wrapper) — correct separation of concerns.
- Structural regression guards lock in architectural invariants.

**Findings**:
- 🔵 Plan invents helper-path resolver that already exists (`_JIRA_PLUGIN_ROOT`). Phase 3, Section 1.
- 🔵 Wrapper diverges from "thin wrapper" pattern it claims to mirror (filter-style vs redirect-style).
- 🔵 Unknown-key handling exits early; differs from config-read-path.sh which delegates with empty default.
- 🔵 Allowed-integration enum hardcoded in three places; not in the central registry.
- 🔵 Cross-key warning logic named Jira but is integration-generic — duplication risk for stories 0048+.
- 🔵 Allowed-tools audit is conditional and may leave skills broken.
- 🔵 (Suggestion) Dump bypasses validating wrapper — two read paths for same data, view output may disagree with skill behaviour.
- 🔵 (Suggestion) External-call regex is approximate and may need ongoing tuning.

### Code Quality

**Summary**: The plan demonstrates strong code-quality discipline: it mirrors an established centralisation pattern, uses appropriate error helpers, keeps validation scope tight, and factors duplicated fallback blocks into a single helper. A few small concerns remain around helper naming/discovery in jira-common.sh, the readability of regex-based migration invariants, the fragility of the regression-guard pattern list, and the swallowed-validation issue in defensive `2>/dev/null) || …=""` tails after Phase 2 introduces hard-fail validation.

**Strengths**:
- Strong DRY: reuses established centralisation pattern rather than inventing new shape.
- Factor-out of duplicated fallback blocks into jira_resolve_default_project removes a real near-duplicate.
- Validation scoping justified with positive regression-guard tests.
- Error-handling categorisation appropriate (log_die for AC4; log_warn for AC5).
- Cross-cutting invariants codified as test assertions.
- Init-flow exclusion from AC5 warning explained — genuine consumer-side understanding.
- Phase decomposition keeps PRs small and independently reviewable.

**Findings**:
- 🔵 Plan invents helper that already exists (`_JIRA_PLUGIN_ROOT` at jira-common.sh:42). Phase 3.
- 🔵 `2>/dev/null) || x=""` defensive tails will silently swallow Phase 2 validation failures. Phase 4 / Phase 3 helper.
- 🔵 Migration-invariant regexes (INLINE_DEFAULT_PATTERN, STALE_PATTERN) are write-only — dense and double-escaped.
- 🔵 Hardcoded EXTERNAL_CALL_PATTERNS will silently rot.
- 🔵 Unknown-key path exits 0 silently — hides programmer typos in `set -euo pipefail` consumers.
- 🔵 Phase 1 → Phase 2 incremental file modification: exec-to-capture+echo shift warrants checkpoint test.
- 🔵 (Suggestion) jira_resolve_default_project has dual-purpose return semantics (warn + value).

### Test Coverage

**Summary**: The plan presents a comprehensive, phase-aligned test strategy with ~43+ tests that map directly onto the centralisation pattern, validation, dump wiring, and consumer migration. Coverage is strong for AC4 (validation) and AC5 (cross-key warning), and the plan correctly extends the existing single-definition-site and no-inline-default invariants. However, there are notable gaps around AC2/AC3 verification, fragility in the AC1 regression guard, missing precedence/ordering tests, and untested integration-test continuity for the migrated Jira flows.

**Strengths**:
- Strict TDD framing with red-then-green ordering called out per phase.
- Phase 2 includes scoped-validation regression guards.
- Phase 4 introduces complementary structural tests that lock in the migration durably.
- Plan correctly extends existing single-definition-site invariant.
- Phase 3 names specific integration values in warning-content tests.
- Phase 5 explicitly tests the design decision around dump-as-diagnostic.

**Findings**:
- 🟡 AC1 regression guard fragile and prone to silent drift. Phase 6.
- 🟡 No end-to-end test that Phase 3 refactor preserves jira-create / jira-search behaviour.
- 🟡 Missing local-vs-default and explicit-empty-vs-missing precedence tests. Phase 1.
- 🟡 Dump ordering and source-attribution mix tests under-specified. Phase 5.
- 🟡 Inline-default regex won't catch line-continuation invocations. Phase 4.
- 🔵 WORK_KEYS structural test should pin index alignment (not just length+contents).
- 🔵 AC2 has no dedicated test.
- 🔵 AC3 has no dedicated automated test.
- 🔵 Stderr-content assertion for valid-values list is over-loose.
- 🔵 Phase 3 test isolation under-specified (subshell + _JIRA_PLUGIN_ROOT resolution).
- 🔵 SKILL.md inline bash snippets not parse-checked or executed.
- 🔵 (Suggestion) Test count distribution doesn't reconcile (43 vs 49 tests).

### Correctness

**Summary**: The plan's logic is largely sound but has two correctness concerns worth raising. The most significant is that the new hard-fail validation in config-read-work.sh interacts poorly with the `2>/dev/null) || var=""` defensive idiom that the plan explicitly preserves at every consumer (and that jira_resolve_default_project introduces): an invalid work.integration value silently becomes empty, suppressing both AC4's error and AC5's warning. There is also a scope question around AC5 — the warning fires only inside jira_resolve_default_project (search + create flows), so other 'integration skills' (update/comment/transition/attach/show) and any non-Jira integration would not satisfy AC5 as written.

**Strengths**:
- Empty-vs-missing semantics handled correctly via `[ -z ]` / `[ -n ]` checks.
- Validation properly scoped to `integration` key with explicit regression tests.
- Phase 2 enum case statement matches work item's allowed values exactly.
- Init-flow correctly excluded from AC5 warning path.

**Findings**:
- 🔴 Hard-fail validation is silently swallowed by `2>/dev/null) || var=""` idiom. Phase 3 / Phase 4.
- 🟡 AC5 warning only fires in two Jira flows, not on every integration skill.
- 🟡 Unknown-key behaviour diverges from claimed mirror of config-read-path.sh.
- 🔵 WORK_KEYS uses bare keys but plan claims to match PATH_KEYS convention (PATH_KEYS is dotted).
- 🔵 Defensive `|| echo "{number:04d}"` tail in write-visualiser-config.sh is functionally dead — rationale wrong.
- 🔵 Dump uses config-read-value.sh to bypass validation, but consumers can still hard-fail post-dump.

### Standards

**Summary**: The plan adheres well to the project's thin-wrapper + central registry pattern, log_die/log_warn standardisation, test section naming, and plan frontmatter conventions. However, two convention divergences from the existing config-read-path.sh precedent should be reconciled: WORK_KEYS storing bare keys instead of fully-dotted entries, and the unknown-key branch exiting early instead of delegating with an empty default. A few smaller standards points concern the recognised-keys allow-list staying prose, and re-introducing a path helper that already exists.

**Strengths**:
- Plan frontmatter matches create-plan template.
- Test section naming follows `=== <name> ===` convention.
- Adopts log_die/log_warn rather than ad-hoc echo >&2 / exit.
- Extends existing single-definition-site and no-inline-default invariants.
- Centralisation in config-defaults.sh mirrors post-0030 pattern.
- Phase 5 keeps config-dump.sh on config-read-value.sh — matches dump-as-diagnostic convention.

**Findings**:
- 🟡 WORK_KEYS bare-key form diverges from PATH_KEYS dotted-key convention.
- 🔵 Unknown-key branch exits early; config-read-path.sh delegates with empty default.
- 🔵 Plan suggests introducing path helper that already exists (_JIRA_PLUGIN_ROOT).
- 🔵 Recognised-keys allow-list stays prose; not declaration-driven from WORK_KEYS.
- 🔵 allowed-tools audit can be reduced to a single rule.
- 🔵 Five-array test message duplicates literal array names.

### Usability

**Summary**: The plan delivers a clear, well-paved path for the common case (configure once via team/local config, view via /accelerator:configure view) and uses the existing log_die/log_warn idioms. However the recovery path from a misconfigured work.integration value is weak: the error names valid values but does not tell users where to fix it (which file/key), and the AC5 warning fires only inside Jira-specific helpers, leaving Linear/Trello/GitHub-Issues consumers (later stories) to re-implement the warning. The mixed-config team-vs-local override scenario is supported by the dump but not actively explained anywhere in the docs.

**Strengths**:
- All three work.* keys appear in /accelerator:configure view with team/local/default source attribution.
- Validation scoped to integration key — typo doesn't break local-only skills.
- Hard-fail on unrecognised work.integration surfaces typos immediately.
- jira-init-flow exemption from AC5 warning is the right call and explicitly documented.
- Phase 6 'Local-first storage' paragraph gives users a clear AC1 mental model.

**Findings**:
- 🟡 Error message names valid values but not the file to fix.
- 🟡 AC5 warning is Jira-only; future integrations will silently regress.
- 🔵 Local-first paragraph buried inside a work.id_pattern section.
- 🔵 Invalid work.integration appears verbatim in dump with no marker.
- 🔵 Team-vs-local override precedence not explained in user-facing docs.
- 🔵 Unknown-key warns but unknown-integration value errors — same family, asymmetric severity, undocumented.
- 🔵 (Suggestion) Warning could suggest /accelerator:init-jira as the discoverable fix.

### Documentation

**Summary**: The plan's documentation work concentrates on the canonical `skills/config/configure/SKILL.md` (which is appropriate as the single source of truth) and ignores several other places that already document or reference the `work.*` config keys today, creating real documentation drift risk. Specifically, `README.md` (lines 122, 280, 342) and integration-skill SKILL.md files reference `work.id_pattern` / `work.default_project_code` but will never see the new `work.integration` key — and integration consumers have no documentation surface that mentions the AC5 warning pre-emptively.

**Strengths**:
- Plan correctly identifies the canonical configure/SKILL.md work section as the primary doc surface.
- Header comment in new config-read-work.sh is genuinely useful (allowed enum values, defaults, unknown-key behaviour).
- Cross-references in References section are concrete (file:line).
- AC1 (`local-first storage`) gets a dedicated documentation paragraph.

**Findings**:
- 🟡 README.md references work.* keys in three places but is not updated.
- 🟡 Lead-in sentence still says "Two keys are recognised" after a third row is added.
- 🟡 New work.integration key is not discoverable from integration-skill docs.
- 🔵 Local-first paragraph paraphrases AC1 but omits its observable contract (which skills it applies to).
- 🔵 Status transitions are process, not documentation; conflates roles in Phase 6.
- 🔵 (Suggestion) Wrapper header omits the AC5/cross-key warning behaviour delegation.
- 🔵 (Suggestion) ADR references correctly named but cited without explaining their bearing.

---

## Re-Review (Pass 2) — 2026-05-08T22:57:42+00:00

**Verdict:** COMMENT

The plan is in good shape to implement. The Critical correctness finding is fully resolved and the vast majority of Major findings are either resolved or transformed into Minor cleanup items. One cross-cutting theme remains noteworthy across four lenses (enum duplication between `config-read-work.sh` and `config-dump.sh`), and one residual code-quality concern around a defensive `|| project=""` tail in `work_resolve_default_project` mirrors a pattern the plan critiques elsewhere. Both are addressable with small follow-up edits if desired but neither blocks implementation.

### Previously Identified Issues

#### Critical
- 🔴→✅ **Correctness**: Hard-fail validation swallowed by `2>/dev/null) || var=""` — **Resolved**. Phase 3 helper drops the redirect on the integration read with explicit rationale; Phase 4 drops dead defensive tails on local-key reads.

#### Major
- 🟡→✅ **Correctness**: AC5 warning Jira-only — **Resolved**. Helper promoted to `scripts/work-common.sh` as integration-agnostic `work_resolve_default_project`.
- 🟡→✅ **Correctness**: Unknown-key behaviour diverges from `config-read-path.sh` — **Resolved**. Wrapper now warns and delegates with empty default. (One residual nit: the wrapper uses a `found` flag rather than `[ -z "$default" ]`, which is actually a more correct design for `WORK_DEFAULTS` containing legitimate empty values — but the plan's "matches exactly" claim is now technically inaccurate.)
- 🟡→✅ **Standards**: `WORK_KEYS` bare-key form — **Resolved**. Now stores fully-dotted entries.
- 🟡→✅ **Test Coverage**: AC1 regression guard fragile — **Resolved**. Replaced with structural inverse assertion (`skills/integrations/` + `*-api.sh`/`*-auth.sh`) that grows automatically.
- 🟡→✅ **Test Coverage**: No e2e for Phase 3 refactor — **Resolved**. Phase 3 success criteria explicitly call out `test-jira-create.sh` Case 2 + new positive AC2 case.
- 🟡→✅ **Test Coverage**: Missing precedence/empty-string tests — **Resolved**. Phase 1 lists local-override and explicit-empty tests for all three keys.
- 🟡→✅ **Test Coverage**: Dump ordering and source-attribution mix — **Resolved**. Phase 5 enumerates ordering and mixed-source tests.
- 🟡→✅ **Test Coverage**: Inline-default regex misses line-continuations — **Resolved**. awk pre-pass joins continuations before grep; documented.
- 🟡→✅ **Usability**: Error message lacked file pointer — **Resolved**. Now names `.accelerator/config.md` and `/accelerator:configure view`.
- 🟡→✅ **Usability**: AC5 warning Jira-only — **Resolved** (same fix as the correctness item).
- 🟡→✅ **Documentation**: README not updated — **Resolved**. Phase 6 Section 2 added with concrete insertion text for line-280.
- 🟡→✅ **Documentation**: "Two keys are recognised" lead-in — **Resolved**. Phase 6 Section 1A rewrites to "Three keys are recognised:" with broader framing.
- 🟡→✅ **Documentation**: Integration-skill discoverability — **Resolved**. Phase 6 Section 3 adds notes to init-jira, create-jira-issue, search-jira-issues.

#### Minor (selected)
- 🔵→✅ **Architecture/Code-Quality/Standards**: `_JIRA_PLUGIN_ROOT` reuse — **Resolved**. No new helper invented.
- 🔵→✅ **Code-Quality**: Migration-invariant regexes — **Resolved**. Each pattern now has MATCHES/REJECTS comments.
- 🔵→✅ **Code-Quality**: Phase 1→2 exec-to-capture+echo shift — **Resolved**. Explicit checkpoint test added.
- 🔵→✅ **Test Coverage**: WORK_KEYS index alignment — **Resolved**. Per-key documented-default behaviour test added.
- 🔵→✅ **Test Coverage**: Stderr-content assertion over-loose — **Resolved**. Per-value independent substring assertions specified.
- 🔵→✅ **Test Coverage**: Phase 3 test isolation — **Resolved**. Documented subshell fixture pattern with `1>&3 3>&1` redirect for stdout/stderr separation.
- 🔵→✅ **Test Coverage**: SKILL.md inline bash snippets unparsed — **Resolved**. `bash -n` parse-check test added.
- 🔵→✅ **Correctness**: WORK_KEYS dotted/bare form — **Resolved** (covered by Standards Major resolution above).
- 🔵→✅ **Correctness**: Visualiser defensive-tail rationale wrong — **Resolved**. Phase 4 now correctly explains tails are functionally dead because validation cannot trip on those keys (though see new finding below).
- 🔵→✅ **Correctness**: Dump non-fatal validation rationale — **Resolved**. Now annotates inline.
- 🔵→✅ **Standards**: Recognised-keys allow-list as prose — **Partially resolved** (still prose but Phase 6 success criteria assert all three keys are listed; future drift remains possible).
- 🔵→✅ **Standards**: allowed-tools per-file audit — **Resolved**. Single structural assertion replaces checklist.
- 🔵→✅ **Usability**: Local-first paragraph buried — **Resolved**. Now names the seven skills the invariant applies to.
- 🔵→✅ **Usability**: Invalid value verbatim in dump — **Resolved**. Inline `(invalid: ...)` annotation.
- 🔵→✅ **Usability**: Override precedence undocumented — **Resolved**. Table description acknowledges precedence and points to `configure view`.
- 🔵→✅ **Documentation**: Local-first paragraph contract — **Resolved**. Now names skills explicitly.
- 🔵→✅ **Documentation**: Status transitions in Phase 6 — **Resolved**. Moved to dedicated `## Adoption` section.
- 🔵→✅ **Documentation**: Wrapper header missing AC5 delegation note — **Resolved** by header expansion.
- 🔵→✅ **Documentation**: ADR references uncited — **Resolved**. Each now annotated with bearing.

### New Issues Introduced

The principal new theme — flagged by four lenses — is enum duplication between the wrapper (Phase 2) and the dump (Phase 5):

#### Major
- 🟡 **Code Quality / Correctness / Standards (cross-cutting)**: Integration enum duplicated between `config-read-work.sh` and `config-dump.sh`
  **Location**: Phase 2 + Phase 5
  Both case statements hardcode `jira | linear | trello | github-issues`. Adding a fifth integration requires lockstep edits to two files; forgetting either silently desynchronises the dump's diagnostic from the wrapper's validation. Suggestion: extract `WORK_INTEGRATION_VALUES=("" jira linear trello github-issues)` to `config-defaults.sh`, source from both, and extend the single-definition-site invariant to cover it.

- 🟡 **Code Quality**: Helper retains `|| project=""` defensive tail the plan critiques elsewhere
  **Location**: Phase 3 Section 1, body of `work_resolve_default_project`
  After `WORK_DEFAULTS` supplies `""` for `default_project_code`, the `|| project=""` tail can only fire on a true script error from `config-read-work.sh` — the same failure mode the plan argues should propagate. Inconsistent with the rationale used to drop the tail on the integration read three lines above and on the visualiser script in Phase 4. Suggestion: drop the tail; let `set -euo pipefail` propagate.

#### Minor
- 🔵 **Architecture**: `jira-common.sh` sources `scripts/work-common.sh`, then flows source only `jira-common.sh` to gain access to `work_resolve_default_project` — works but inverts the layering. As Linear/Trello/GH-Issues integrations land, each `*-common.sh` will re-source `work-common.sh`. Flatter alternative: have flows source `work-common.sh` directly.
- 🔵 **Architecture / Code-Quality**: awk line-joining pre-pass is dense and inlined per test. Could be extracted to a shared `join_continuations()` helper near the top of `test-config.sh` and reused.
- 🔵 **Code Quality**: Wrapper drops the optional `[default]` second positional argument that `config-read-path.sh` accepts. The asymmetry should be a stated decision (in the wrapper header) rather than an oversight.
- 🔵 **Correctness**: Phase 1's claim that the wrapper "matches `config-read-path.sh:37-42` behaviour exactly" is technically inaccurate — the work wrapper uses a `found` flag rather than `[ -z "$default" ]` (which is actually more correct for `WORK_DEFAULTS` containing legitimate empty entries). Update the wording to "mirrors `config-read-path.sh` shape, with a stricter unknown-key check because WORK_DEFAULTS legitimately contains empty entries".
- 🔵 **Correctness**: "Functionally dead" framing for the visualiser tails is too strong — the tails also masked failures from `config_assert_no_legacy_layout` etc. Removing them under `set -euo pipefail` is a deliberate behavioural change, not a no-op. Reword Phase 4 to acknowledge this.
- 🔵 **Correctness**: AC4 hard-fail propagation depends on callers not using `local var=$(...)` (a known bash gotcha that masks substitution exit codes). Add a comment at migrated call sites or an end-to-end test.
- 🔵 **Correctness**: Helper omits the `cd "$repo_root"` wrapping that the original fallback used. If `config-read-value.sh` searches upward for `.accelerator/`, this is fine; if it inspects only CWD, AC2 may regress for users running flows from subdirectories. Verify and document or restore the `cd`.
- 🔵 **Standards**: New `## Adoption` top-level section is not part of the established plan template (other plans end at `## References`). Either fold into `## Migration Notes` or raise as a template change.
- 🔵 **Standards**: `scripts/work-common.sh` shares a domain prefix with `skills/work/scripts/work-item-common.sh`. Header cross-references would help; or rename to `work-config-common.sh`.
- 🔵 **Standards**: Phase 1's instruction to update `config-defaults.sh` scope-note is under-specified; needs explicit guidance to drop "only PATH and TEMPLATE keys" framing.
- 🔵 **Test Coverage**: awk pre-pass logic itself is untested — add 2-3 micro-fixture tests so the pre-pass behaviour is locked-in.
- 🔵 **Test Coverage**: Dump enum-annotation does not test edge values (whitespace, case-sensitivity).
- 🔵 **Test Coverage**: Source-in-subshell isolation pattern not codified as a reusable helper.
- 🔵 **Test Coverage**: allowed-tools assertion lacks a negative-case sanity test.
- 🔵 **Test Coverage**: curl/wget belt-and-braces guard has no negative-control or doc-prose allowlist.
- 🔵 **Usability**: Error message is now ~230 characters on one line — wraps awkwardly in narrow terminals. Splitting into 2-3 short lines with explicit `\n` breaks would help.
- 🔵 **Usability**: Neither the error nor the warning suggests `/accelerator:init-jira` as the canonical setup path.
- 🔵 **Usability**: Unknown-key (warn) vs unknown-value (die) asymmetry is now consistent with `config-read-path.sh` but not documented for users in the configure SKILL.md "Recognised keys" paragraph.
- 🔵 **Documentation**: README updates at lines 122 and 342 left as conditional sketch ("if list bullets... single-line tweak each"). Commit explicitly: either update or leave as-is.
- 🔵 **Documentation**: Integration-skill notes lack final copy and heading choice (Configuration vs Prerequisites; placement; init-jira's differentiated framing).

#### Suggestions
- 🔵 **Architecture**: Tighten the no-inline-default invariant by having `config-read-work.sh` reject a positional `[default]` outright (`exit 1` if `$# -gt 1`); the test then doesn't need awk parsing.
- 🔵 **Code-Quality**: Helper still couples value-return and warning side-effect (dual-purpose dropped from "Major" because tests now isolate streams cleanly).
- 🔵 **Usability**: Dump annotation register (`(invalid: must be...)`) differs from hard-fail wording (`must be one of:... (got ...)`) — align phrasing.
- 🔵 **Usability**: Per-skill integration notes risk drift; cap each at ~25 words and add a structural test asserting they don't enumerate allowed values.
- 🔵 **Documentation**: Configure SKILL.md table row description for `integration` packs four claims into one cell; lifting the auto-scoping/precedence claims into the prose block beneath the table would scan better.

### Assessment

The plan has moved from REVISE to COMMENT. The previous Critical finding is fully addressed and the major architectural and correctness gaps are closed. The remaining concerns are quality-of-life cleanups: enum-duplication is the most worth addressing before merge (small change, single source of truth, prevents a known drift), and the residual `|| project=""` tail is a one-line fix for consistency. Everything else is fine to defer or accept as-is. The plan is ready to implement.
