---
type: "plan-review"
id: "2026-09-06-0245-invoke-accelerator-directly-in-skills-review-1"
title: "Plan Review: Invoke Accelerator Directly In Skills"
date: "2026-09-06T15:04:04+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-06-0245-invoke-accelerator-directly-in-skills"
parent: "plan:2026-09-06-0245-invoke-accelerator-directly-in-skills"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["correctness", "test-coverage", "architecture", "code-quality", "standards", "compatibility", "security"]
review_number: 1
review_pass: 2
tags: ["skills", "cli", "lint"]
last_updated: "2026-09-06T16:11:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Invoke Accelerator Directly In Skills

**Verdict:** REVISE

The plan's convention change is well-motivated and its core coupling insight —
that the parser recognition and the body invocation form must flip together in
one atomic phase — is correct, with the `EXPECTED_INJECTION_SKILLS = 42` census
and the empirical PATH precondition gate showing good risk discipline. But the
Current State Analysis undercounts the coupled surface: it names two modules,
where at least four production/support modules and three-plus test files
independently encode the `/bin/accelerator` convention. As specified, Phase 1
leaves `mise run check` red (via `skill_write_gate.py`) and the full suite red
(via `skill_corpus.py` and two unit contract-test files), directly
contradicting the "each phase independently mergeable and green" premise. A
secondary theme — the bin-on-PATH assumption has no established version floor
and no fallback once the lint forbids the pathed form — needs an explicit
decision before conversion.

### Cross-Cutting Themes

- **Coupled-surface enumeration is incomplete** (flagged by: correctness,
  test-coverage, architecture) — The plan's "two lint modules encode the
  opposite convention" is the root cause of every build-breaking finding.
  Verified: `tasks/shared/skill_write_gate.py:32`, `tests/integration/support/skill_corpus.py:28-39`,
  `tests/unit/tasks/shared/test_skill_parsing.py:70-72`,
  `tests/unit/tasks/shared/test_dispatch_coherence.py`, and `tasks/test/helpers.py`
  all carry the convention independently. A repo-wide grep finds `/bin/accelerator`
  in 11 Python files; the plan addresses two.
- **TDD notes and Phase 1 verification understate the blast radius** (flagged
  by: test-coverage, correctness) — Only `test_skill_permissions.py` is named,
  and Phase 1 Automated Verification runs `build-system:check` (format + lint +
  types, no tests), so the red unit and integration suites are invisible to the
  documented inner loop and only surface in the heavy `default` run.
- **The bin-on-PATH assumption is load-bearing, unversioned, and
  fallback-free** (flagged by: compatibility, security) — The gate proves the
  bare form resolves on the developer's version and the `!` surface, not the
  v2.1.144 floor, not PATH precedence, and not the Bash-tool grant surface. The
  new lint then forbids restoring the pathed form, removing the escape hatch if
  Claude Code regresses.

### Tradeoff Analysis

- **Uniformity vs defence-in-depth (permission grants)**: The bare grant
  `Bash(accelerator config *)` authorises whatever `accelerator` PATH resolves
  first; the pathed grant pinned the shipped plugin binary. The residual risk
  is low (it presupposes an actor who can already reorder PATH or drop a binary
  on the developer's machine) but real, and the new lint forecloses re-pinning.
  Recommendation: accept the tradeoff for uniformity, but record it explicitly
  and confirm Claude Code *prepends* (not appends) the plugin `bin/` to PATH.

### Findings

#### Critical

- 🔴 **Correctness**: `skill_write_gate.py` breaks `lint:integration-skills:check`,
  so `mise run check` fails after Phase 1
  **Location**: Phase 1 Success Criteria ("Build-system component green") / Current State Analysis
  `tasks/shared/skill_write_gate.py:32` matches mutations with `rf"/bin/accelerator {provider} (?:{verbs})\b"`, a literal the plan never updates. After conversion `_mutation_offset` returns `None` for every write skill, tripping the anti-vacuity guard so `lint:integration-skills:check` (a dependency of `build-system:check`, `mise.toml:506`, itself a dependency of `check`, `mise.toml:664`) exits non-zero. Verified against the file. The phase is not landable in isolation as claimed.

- 🔴 **Correctness / Test Coverage / Architecture**: `skill_corpus.py`'s
  independent markers and `argv()` substitution collapse the integration
  conformance suite
  **Location**: Desired End State ("the full suite pass") / Phase 1 TDD Notes
  `tests/integration/support/skill_corpus.py` carries its own `CONFIG_MARKER = "/bin/accelerator config "` (:28), `INSTRUCTIONS_MARKER`/`CONTEXT_MARKER` (:29-30), and the `_INLINE_SITE` regex (:39). After conversion `extract()` returns an empty corpus (vacuous parametrisation) and `argv()` no longer has a `PLUGIN_PREFIX` to substitute, so `argv[0]` becomes `"accelerator"` not the installed `bin/accelerator` path — the suite's execution model breaks, not just its markers. This suite uniquely exercises the real bootstrap fetch→verify→cache→exec chain the item depends on.

- 🔴 **Test Coverage / Correctness**: Redefining `LAUNCHER`/`BARE_LAUNCHER`
  breaks unit contract tests not named in the TDD notes
  **Location**: Phase 1, Change 1 / TDD Notes
  `tests/unit/tasks/shared/test_skill_parsing.py:70-72` asserts `covered_by(BARE_LAUNCHER, "${CLAUDE_PLUGIN_ROOT}/*")` and `…bin/*` are True — both invert to False once the sentinel is `accelerator zz-external-subcommand-zz`. `test_dispatch_coherence.py`'s over-broad case pairs a now-bare command with `${CLAUDE_PLUGIN_ROOT}/bin/*` rules whose coverage relationship inverts. These are the primary mutation-killing contract tests for the matcher the permissions and release-gating apparatus depend on.

#### Major

- 🟡 **Correctness**: Coupled-surface enumeration is incomplete — root cause of
  the criticals
  **Location**: Current State Analysis ("Two lint modules encode the opposite convention")
  The convention is independently encoded in `skill_write_gate.py`, `skill_corpus.py`, `tasks/test/helpers.py`, and multiple test files, plus `dispatch_coherence.py` consumes `LAUNCHER`/`BARE_LAUNCHER`. Because the "must land in one change" scope is derived from the two-module claim, the atomic phase is scoped too narrowly.

- 🟡 **Test Coverage**: Phase 1 verification omits the test suites in the blast
  radius
  **Location**: Phase 1 Success Criteria — Automated Verification
  `build-system:check` is format + lint + types only — it runs no tests, so the breakage in `test_skill_parsing.py`, `test_dispatch_coherence.py`, and the conformance suite is invisible to the stated inner loop and gives a false green.

- 🟡 **Test Coverage / Standards**: Wrapper detection is narrower than the 0107
  criteria Phase 3 supersedes
  **Location**: Phase 2 (new lint) / Phase 3 (close 0107)
  0107's acceptance criteria require flagging `bash`/`sh`/`env`-wrapped invocations; the plan's Phase 2 lint and Phase 1 grep only catch `bash …accelerator`, missing `sh` and `env`. Closing 0107 while shipping a strict subset of its guard leaves a real escape vector permanently unguarded and untested.

- 🟡 **Test Coverage**: No regression test that the lint spares the five
  legitimate `${CLAUDE_PLUGIN_ROOT}/skills/…` references
  **Location**: Phase 2 TDD Notes
  The TDD notes specify a forbidden-form fixture and a clean bare-form fixture, but no negative case proving the lint does not false-positive on the surviving lens / output-format path references.

- 🟡 **Standards**: New skills-tree guard not pinned in `test_mise.py`
  `_BUILD_SYSTEM_CHECK_GATES`
  **Location**: Phase 2, Section 2 (five-step wiring)
  Every peer skills-tree guard (`lint:dispatch-coherence:check`, `lint:integration-skills:check`) is pinned in that list, driving `test_gate_wired_into_build_system_check` / `test_gate_wired_into_lint_check`. The plan's wiring omits the sixth step, leaving the new gate's placement unguarded against silent unwiring.

- 🟡 **Standards**: `superseded` is not a valid work-item status
  **Location**: Phase 3, Section 1 (Supersede 0107)
  The work-item status vocabulary (`cli/corpus/src/frontmatter_validation/schema.rs:21-29`) is `draft, ready, in-progress, review, done, blocked, abandoned` — `superseded` is valid only for `plan`/`adr`/`design-inventory`. Setting it fails the phase's own `frontmatter validate` success criterion. Use `done` or `abandoned` with the supersession in prose / `relates_to`.

- 🟡 **Compatibility**: No version floor established for plugin-bin-on-PATH
  against the declared v2.1.144 minimum
  **Location**: Precondition gate
  The gate verifies the bare form on the developer's version and the `!` surface only. Bin-on-PATH is a newer Claude Code feature of undocumented introduction version; if it postdates v2.1.144, every converted skill fails at load for the lower half of the supported range — the 0182-class silent breakage. Trace the introducing version (as 0182 did for `${CLAUDE_PLUGIN_DATA}`) or raise the floor.

- 🟡 **Compatibility**: Removing the explicit-path fallback while the lint
  forbids restoring it leaves no escape hatch for a Claude Code regression
  **Location**: Phase 2 Overview / What We're NOT Doing
  Bin-on-PATH is an external dependency no repo guard can detect regressing. If a future Claude Code stops adding `bin/` to the `!` PATH, all 42+ injection skills break at load simultaneously and the lint blocks the obvious mitigation. Record it as a tracked external dependency and state the revert path, or confirm wholesale breakage is the accepted trade-off.

#### Minor

- 🔵 **Architecture**: Convention literal remains duplicated instead of
  centralised on the shared `LAUNCHER` constant
  **Location**: Phase 1, Changes 1-2
  The permissions markers are retargeted to fresh hardcoded literals rather than derived from `LAUNCHER` (e.g. `f"{LAUNCHER} config "`); the same literal is re-declared again in `skill_corpus.py`. The change reproduces the multi-site coupling that made it hazardous.

- 🔵 **Architecture**: Forbidden-form literal has no shared source after
  `LAUNCHER` is redefined
  **Location**: Phase 2, Change 1
  The banned `${CLAUDE_PLUGIN_ROOT}/bin/accelerator` was previously the `LAUNCHER` constant; after the redefinition the new lint hardcodes a fresh unshared copy. Consider a named `FORBIDDEN_LAUNCHER` in `skill_parsing.py`.

- 🔵 **Architecture**: Fourth skills-scanning lint expands wiring surface
  instead of consolidating
  **Location**: Phase 2 / What We're NOT Doing
  `bare_invocation.py` and `call_site_migration.py` are the same shape; the `rglob("SKILL.md")` walk is now duplicated a fifth time. Consider hosting the rule in the existing call-site guard and/or extracting a shared skill-iteration helper.

- 🔵 **Code Quality**: Raw whole-line substring forbidding bakes in a
  documentation trap
  **Location**: Phase 2, Change 1
  Scanning every line (prose included) for `/bin/accelerator` and `bash …accelerator` means any future skill documenting the bootstrap path trips the lint with no escape hatch. The `visualise` skill already carries launcher prose. Prefer the shared invocation-context primitives, or define an opt-out.

- 🔵 **Code Quality**: New lint's helper name diverges from the sibling
  `violations()` convention
  **Location**: Phase 2, Change 1
  Both sibling guards expose `violations(root) -> list[str]`; the plan names it `scan_forbidden_forms`. Match the convention for a uniform module shape.

- 🔵 **Security**: Bare-name grant removes the plugin-binary PATH pin; the
  tradeoff is unacknowledged
  **Location**: Phase 1, Change 3 / Migration Notes
  `Bash(accelerator config *)` authorises whatever `accelerator` PATH resolves first, and the `!` preprocessor runs it unconditionally at load. Low likelihood, but the "behaviour-preserving by construction" claim overstates equivalence. Document the tradeoff and confirm PATH *prepend* ordering.

- 🔵 **Compatibility / Security**: "Behaviour-preserving by construction"
  overstates equivalence
  **Location**: Migration Notes
  The equivalence holds only given the PATH precondition. The pathed form depends on `${CLAUDE_PLUGIN_ROOT}` textual substitution (older, guaranteed); the bare form depends on plugin-bin on the `!` shell PATH. Restate as conditional on the verified precondition.

- 🔵 **Compatibility**: Gate covers the `!` surface but not the Bash-tool
  permission-grant surface
  **Location**: Precondition gate / Phase 1 Manual Verification
  The conversion also rewrites 97 frontmatter grants. A model-issued Bash-tool `accelerator` call must both resolve on PATH and match the bare grant without prompting. Extend the manual check to a model-issued invocation.

- 🔵 **Compatibility**: Mixed convention (skills bare, hooks pathed) is correct
  but not durably documented
  **Location**: What We're NOT Doing
  Nothing prevents a future contributor from "unifying" hooks onto the bare form and breaking them. Add a durable note (hooks CLAUDE.md / `hooks.json` context) that hooks intentionally retain the pathed form.

- 🔵 **Standards**: Work-item linkage vocabulary has no `supersedes`/`superseded_by`
  key
  **Location**: Phase 3, Section 2
  Work-item typed-linkage keys are `parent, blocks, blocked_by, derived_from, relates_to, source` (`schema.rs:31-38`). Reaching for `superseded_by` by ADR analogy fails validation. Record the relationship via `relates_to` + prose.

- 🔵 **Standards**: Inconsistent `--file` argument form between the two Phase 3
  validate criteria
  **Location**: Phase 3 Success Criteria
  One passes a shell glob (`--file meta/work/0107-*.md`), the other a full filename. `Validate.file` takes one value per `--file`; the glob is only well-formed at a single match. Use the fully-qualified `0107` filename.

- 🔵 **Test Coverage**: Widened `is_plugin_invocation` lacks a negative-boundary
  test
  **Location**: Phase 1, Change 1
  Add a case asserting `is_plugin_invocation("accelerator-verify-darwin-arm64 x")` is False (sibling binary) and a bare `accelerator config …` is True.

#### Suggestions

- 🔵 **Code Quality**: Dropping the `/bin/` anchor loosens the permissions
  substring markers' precision
  **Location**: Phase 1, Change 2
  With `is_plugin_invocation` widened and markers still matched by `in`, a future command could coincidentally match mid-string. Consider matching the subcommand structurally via `launcher_token`.

- 🔵 **Security**: The precondition gate proves resolution, not PATH precedence
  **Location**: Precondition gate
  Capture `command -v accelerator` and the resolved PATH ordering during `!` execution to confirm the plugin directory precedes typical user/system bin locations.

### Strengths

- ✅ Correctly identifies the tight coupling between parser recognition
  (`is_plugin_invocation`, `LAUNCHER`) and the body form, and keeps the
  `${CLAUDE_PLUGIN_ROOT}/` branch alive so the five non-accelerator plugin-script
  references stay covered — an additive widening, the safe way to change a
  recogniser.
- ✅ Uses the `EXPECTED_INJECTION_SKILLS = 42` census equality as a deliberate
  tripwire, turning a silent recognition-drop into a hard failure.
- ✅ Gates the whole conversion on an empirical PATH-resolution precondition for
  the `!` preprocessor with an explicit stop condition, rather than asserting it
  silently.
- ✅ Anchors the 427-site mechanical replace on the full unique literal, verified
  not to mangle surviving `${CLAUDE_PLUGIN_ROOT}/skills/…` paths or prose forms
  like `/accelerator:visualise`, `.accelerator/config.md`.
- ✅ Correctly scopes hooks out (they receive `${CLAUDE_PLUGIN_ROOT}` as a real
  env var and may lack `bin` on PATH), and correctly treats the new lint as a
  plain Python task, not a dispatched sub-binary.
- ✅ Phase 2 follows genuine TDD with a known-positive seeded fixture, satisfying
  0107's known-positive requirement.

### Recommended Changes

1. **Enumerate the full coupled surface before Phase 1** (addresses: enumeration
   incomplete; the three criticals). Grep the whole repo for `/bin/accelerator`,
   `bin/accelerator`, `PLUGIN_PREFIX`, `LAUNCHER`, `is_plugin_invocation`, and
   list every hit as an explicit Phase 1 change. Confirmed hits beyond the two
   named modules: `tasks/shared/skill_write_gate.py:32`,
   `tests/integration/support/skill_corpus.py:28-39,83,100`,
   `tasks/test/helpers.py`, `tests/unit/tasks/shared/test_skill_parsing.py:70-72`,
   `tests/unit/tasks/shared/test_dispatch_coherence.py`.

2. **Rework `skill_corpus.py`'s binary resolution, not just its markers**
   (addresses: conformance-suite critical). Retarget the three markers and
   `_INLINE_SITE`, and redesign `Command.argv()` / the `argv[0] == …/bin/accelerator`
   assertion — the bare form resolves via PATH, so the harness must locate the
   binary differently than by `${CLAUDE_PLUGIN_ROOT}` substitution.

3. **Expand Phase 1 TDD Notes and Automated Verification to the real blast
   radius** (addresses: TDD/verification gaps). Name `test_skill_parsing.py`,
   `test_dispatch_coherence.py`, and the `skill-invocation` integration suite;
   add `pytest` runs of them (not just `build-system:check`, which runs no
   tests) to Success Criteria; keep `test_dispatch_coherence.py::test_the_real_skills_tree_passes`
   green on the converted tree.

4. **Broaden wrapper detection to `bash`/`sh`/`env`** (addresses: 0107 subset).
   Assert the first token is bare `accelerator` (or match `(bash|sh|env)\s+…accelerator`),
   and add `sh`- and `env`-wrapped fixtures before implementing.

5. **Add the two missing wiring/registration steps** (addresses: gate pinning,
   negative-case coverage). Add `"lint:bare-invocation:check"` to
   `_BUILD_SYSTEM_CHECK_GATES` in `test_mise.py`; add a `${CLAUDE_PLUGIN_ROOT}/skills/…`
   negative fixture to `test_bare_invocation.py`.

6. **Fix Phase 3 to the work-item schema** (addresses: invalid status, linkage
   key, glob). Use `done`/`abandoned` (not `superseded`), express supersession
   via `relates_to` + prose, and use the fully-qualified `0107` filename in the
   validate command.

7. **Resolve the bin-on-PATH version and fallback questions before conversion**
   (addresses: version floor, no escape hatch, overstated equivalence). Trace
   the introducing Claude Code version and confirm it is at/below v2.1.144 or
   raise the floor; record bin-on-PATH as a tracked external dependency with a
   stated revert path; restate "behaviour-preserving" as conditional on the
   verified precondition; extend the gate to capture PATH precedence and a
   model-issued Bash-tool call.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Correctness

**Summary**: The plan's central claim — that Phase 1 is "independently
mergeable and green" and leaves `mise run check` passing — is false. The
Current State Analysis enumerates only two modules, but at least two more
consumers hold their own independent literals: `tasks/shared/skill_write_gate.py`
(feeds `lint:integration-skills:check`, inside `mise run check`) and
`tests/integration/support/skill_corpus.py` (feeds the integration conformance
suite). After conversion these fail closed, so the coupled surface is
materially undercounted and the build goes red.

**Strengths**:
- Correctly identifies the coupling between parser recognition and body form,
  and keeps the `${CLAUDE_PLUGIN_ROOT}/` branch alive for the five
  non-accelerator references.
- Flags `EXPECTED_INJECTION_SKILLS = 42` as a loud tripwire.
- Gates conversion on an empirical PATH precondition with an explicit stop.

**Findings**:
- 🔴 critical (high) — Phase 1 Success Criteria / Current State: `skill_write_gate.py:32`
  matches `/bin/accelerator {provider} …`; unmodified, it makes
  `lint:integration-skills:check` (→ `build-system:check` → `check`) exit
  non-zero. `mise run check` is RED after Phase 1.
- 🔴 critical (high) — Desired End State / Phase 1 TDD: `skill_corpus.py`'s own
  markers (:28-30), `_INLINE_SITE` (:39), and `argv()` PLUGIN_PREFIX
  substitution (:83) collapse the conformance suite; `argv[0]` assertion (:104)
  and `==42` counts (:141/146) fail.
- 🟡 major (high) — Phase 1 TDD: `BARE_LAUNCHER` flip breaks `test_skill_parsing.py:70-72`
  and the `test_dispatch_coherence.py` over-broad case; TDD Notes omit both.
- 🟡 major (medium) — Current State: coupled-surface enumeration incomplete;
  grep the whole repo for `/bin/accelerator`, `PLUGIN_PREFIX`, `LAUNCHER`,
  `is_plugin_invocation` and list every hit.

### Test Coverage

**Summary**: The plan uses the census tripwire well and writes Phase 2's lint
test-first, but its TDD notes and Success Criteria dramatically understate the
blast radius of changing the shared `LAUNCHER` constant. Only
`test_skill_permissions.py` is named, yet `test_skill_parsing.py`,
`test_dispatch_coherence.py`, and the entire integration conformance corpus
bake in the prefix semantics and break or silently self-disable on conversion.
The lint's wrapper coverage is narrower than the 0107 criteria Phase 3
supersedes.

**Strengths**:
- Uses `EXPECTED_INJECTION_SKILLS = 42` as a deliberate Phase 1 tripwire.
- Phase 2 follows genuine TDD with forbidden-form and clean-form fixtures.
- Recognises the parser flip and body conversion are coupled, tests-first.

**Findings**:
- 🔴 critical (high) — Phase 1 conversion silently empties the integration
  conformance corpus (`skill_corpus.py` markers + `argv()` model).
- 🔴 critical (high) — Changing `LAUNCHER` breaks contract tests
  (`test_skill_parsing.py`, `test_dispatch_coherence.py`) not named in TDD.
- 🟡 major (high) — Phase 1 verification runs `build-system:check` (no tests),
  so red suites give a false green.
- 🟡 major (high) — Wrapper detection (`bash …accelerator`) misses `sh`/`env`
  that 0107 required.
- 🟡 major (medium) — No fixture proving the lint spares the five legitimate
  `${CLAUDE_PLUGIN_ROOT}/skills/…` references.
- 🔵 minor (medium) — Widened `is_plugin_invocation` lacks a negative-boundary
  (sibling-binary) test.

### Architecture

**Summary**: The core coupling insight and phase decomposition are sound, but
the impact map is incomplete — a third live module (`skill_corpus.py`)
hardcodes the same convention and feeds an active conformance suite that
collapses on conversion. The plan also perpetuates the duplication it laments
(retargeting hardcoded markers in place rather than centralising on `LAUNCHER`)
and adds a fifth copy of the skills-tree walk / a fourth separately-wired lint.

**Strengths**:
- Correctly identifies the atomic-phase coupling of recognition and body form.
- Leverages the census equality as a loud tripwire.
- Precondition gate validates the distinct `!` execution surface empirically.
- Phase ordering (convert-then-guard) is correct.

**Findings**:
- 🔴 critical (high) — `skill_corpus.py` unmapped; `extract()` returns empty and
  parametrisation collapses with no census tripwire. Add it to the Phase 1
  coupled set.
- 🟡 major (high) — Convention literal stays duplicated across three modules;
  derive markers from the shared `LAUNCHER` constant.
- 🔵 minor (medium) — Forbidden-form literal has no shared source after
  `LAUNCHER` is redefined; introduce a named `FORBIDDEN_LAUNCHER`.
- 🔵 minor (medium) — Fourth skills-scanning lint expands wiring surface; consider
  hosting in `call_site_migration.py` and extracting a shared walk helper.

### Code Quality

**Summary**: Well-structured for maintainability — reuses the pure-helper +
thin-@task shape, leans on the existing census, anchors the mechanical replace
on a unique literal, and follows red-green ordering. Reservations are minor: the
new lint forbids raw substrings across whole lines including prose (a
documentation trap and false-match surface); the helper name diverges from the
sibling `violations()` convention; and dropping the `/bin/` anchor loosens the
permissions markers.

**Strengths**:
- Reuses `EXPECTED_INJECTION_SKILLS = 42` as a tripwire.
- Mechanical conversion anchored on the full unique literal; verified not to
  mangle surviving references or prose forms.
- New module follows the established `call_site_migration.py` shape and file:line
  error format.
- TDD red-green ordering is spelled out.

**Findings**:
- 🔵 minor (medium) — Raw whole-line substring forbidding bakes in a
  documentation trap with no escape hatch (`visualise` already carries launcher
  prose).
- 🔵 minor (high) — Helper named `scan_forbidden_forms` diverges from the sibling
  `violations()` entry-point convention.
- 🔵 suggestion (medium) — Dropping the `/bin/` anchor loosens the permissions
  substring markers; match the subcommand structurally via `launcher_token`.

### Standards

**Summary**: Follows the repo's task-registration conventions well (naming,
`Collection.from_module` wiring, plain-Python-task scoping, over-80 mise
descriptions are convention-compliant), but violates two codified conventions:
it omits pinning the new skills-tree guard in `test_mise.py`'s
`_BUILD_SYSTEM_CHECK_GATES`, and proposes a `superseded` status for work-item
0107 that is outside the work-item status vocabulary.

**Strengths**:
- Five-step wiring mirrors existing Python lint tasks exactly.
- Correctly scopes the change as a plain Python lint task (13-point checklist
  does not apply).
- Long single-line mise `description` is consistent with existing gate
  descriptions and not subject to the 80-column check.

**Findings**:
- 🟡 major (high) — New guard not added to `_BUILD_SYSTEM_CHECK_GATES` in
  `test_mise.py`; placement left unpinned against the repo's own convention.
- 🟡 major (high) — `superseded` is not a valid work-item status
  (`schema.rs:21-29`); would fail the phase's own `frontmatter validate`. Use
  `done`/`abandoned`.
- 🔵 minor (medium) — No `supersedes`/`superseded_by` linkage key for
  work-items; use `relates_to` + prose.
- 🔵 minor (low) — Inconsistent `--file` glob vs filename between the two Phase 3
  validate criteria; the glob is brittle.

### Compatibility

**Summary**: The plan swaps the always-available `${CLAUDE_PLUGIN_ROOT}/bin/accelerator`
textual-substitution path for bare `accelerator`, which depends on Claude Code
adding the plugin `bin/` to the `!`-preprocessor shell's PATH. The central risk
is that this behaviour has no established version floor against the declared
v2.1.144 minimum, and the plan removes the working fallback and adds a lint that
forbids restoring it. The manual gate validates only one version and the `!`
surface.

**Strengths**:
- Correctly scopes hooks out (they receive `${CLAUDE_PLUGIN_ROOT}` as a real env
  var).
- Preserves `PLUGIN_PREFIX` recognition (additive widening).
- Treats the PATH assumption as a gating precondition to verify empirically.
- Notes existing precedent (browser agents invoke `accelerator design executor`
  bare).

**Findings**:
- 🟡 major (medium) — No version floor established for bin-on-PATH vs v2.1.144;
  trace the introducing version as 0182 did for `${CLAUDE_PLUGIN_DATA}`.
- 🟡 major (high) — Removing the explicit-path fallback while the lint forbids
  restoring it leaves no escape hatch for a Claude Code regression.
- 🔵 minor (high) — "Behaviour-preserving by construction" overstates
  equivalence; it is conditional on the PATH precondition.
- 🔵 minor (medium) — Gate covers the `!` surface but not the Bash-tool grant
  surface (97 grants); a model-issued call could still prompt.
- 🔵 minor (medium) — Mixed convention (skills bare, hooks pathed) is correct but
  not durably documented as an intentional boundary.

### Security

**Summary**: The plan swaps a path-pinned launcher grant for a bare-name grant
and permanently bans the pathed form. The core effect is a modest, self-inflicted
defence-in-depth reduction: the bare grant authorises whatever `accelerator`
PATH resolves first, run unconditionally at skill load. The over-broad-rule,
metacharacter, and `--fail-safe` controls are preserved, so residual risk is
real but low for a developer-tooling surface and only material under a
pre-existing PATH-shadowing compromise.

**Strengths**:
- Injection-safety controls (`has_metacharacter`, `--fail-safe`) preserved
  verbatim.
- `dispatch_coherence._is_over_broad` keeps its semantics under the bare form.
- `launcher_token`'s trailing-separator check prevents sibling-binary
  mis-tokenisation.
- A precondition gate confirms the mechanism rather than assuming it.

**Findings**:
- 🔵 minor (medium) — Bare-name grant removes the plugin-binary PATH pin; the
  tradeoff is unacknowledged and "behaviour-preserving" overstates equivalence.
  Document it and confirm PATH prepend ordering.
- 🔵 suggestion (medium) — The gate proves resolution, not PATH precedence;
  capture `command -v accelerator` and PATH ordering during `!` execution.

## Re-Review (Pass 2) — 2026-09-06

**Verdict:** APPROVE

The plan was edited to address every finding, then re-reviewed through all seven
lenses. All three criticals and all eight majors from Pass 1 are resolved and
independently verified against the real files — most importantly the coupled
surface is now complete (`skill_write_gate.py`, `skill_corpus.py`, and the four
coupled test files are all Phase 1 changes, with the dependency chain
`skill_write_gate → lint:integration-skills:check → build-system:check → check`
confirmed). Pass 2 surfaced ten smaller issues (one major, the rest
minor/suggestion), each a precision refinement of the edits; all ten have since
been applied. The remaining open items are genuinely external (the bin-on-PATH
version floor, resolvable only by a changelog trace during implementation) and
are recorded as gating preconditions, not plan defects.

### Previously Identified Issues

- 🔴 **Correctness**: `skill_write_gate.py` breaks `mise run check` — **Resolved** (Phase 1 Change 3; regex `\baccelerator {provider}…`, chain verified)
- 🔴 **Correctness / Test Coverage / Architecture**: `skill_corpus.py` collapses the conformance suite — **Resolved** (Phase 1 Change 4; markers imported, `argv()` reworked, `argv[0]` assertion replaced)
- 🔴 **Test Coverage / Correctness**: `LAUNCHER` flip breaks unnamed contract tests — **Resolved** (TDD Notes name `test_skill_parsing.py`, `test_dispatch_coherence.py`)
- 🟡 **Correctness**: coupled-surface enumeration incomplete — **Resolved** (full inventory; out-of-scope hits correctly excluded)
- 🟡 **Test Coverage**: Phase 1 verification omits the test suites — **Resolved** (Success Criteria now run the coupled pytest files + conformance suite + three lints, not just `build-system:check`)
- 🟡 **Test Coverage / Standards**: wrapper detection narrower than 0107 — **Resolved** (`bash`/`sh`/`env`, with per-wrapper fixtures)
- 🟡 **Test Coverage**: no carve-out fixture for legitimate `${CLAUDE_PLUGIN_ROOT}/skills/…` refs — **Resolved**
- 🟡 **Standards**: new gate not pinned in `_BUILD_SYSTEM_CHECK_GATES` — **Resolved** (sixth wiring step)
- 🟡 **Standards**: `superseded` invalid work-item status — **Resolved** (`done`/`abandoned` + `relates_to`)
- 🟡 **Compatibility**: no version floor for bin-on-PATH — **Resolved structurally** (gate step 1 traces the changelog against v2.1.144; per-surface evidence, inconclusive `!`-surface trace → stop)
- 🟡 **Compatibility**: no escape hatch / fallback removed — **Resolved** (recorded as tracked external dependency with revert path; accepted trade-off)
- 🔵 Minor/suggestion (architecture duplication, forbidden-form source, code-quality helper name + doc trap + marker precision, security tradeoff + gate precedence, compatibility equivalence + grant surface + mixed convention, standards linkage key + glob, test-coverage sibling-boundary) — **All Resolved**

### New Issues Introduced (all since applied)

- 🟡 **Test Coverage** (major): the conformance test file `test_skill_invocation_conformance.py:104` `argv[0]` assertion was not in the coupled red-first list — a Phase 1 success criterion would stay red. **Applied**: added to the coupled test surface and TDD Notes, with a replace-not-delete instruction.
- 🔵 **Code Quality**: an inline `# …` comment survived in the Phase 2 code block (violates the no-comments-in-plans rule); public marker imports not reconciled with the four `_`-prefixed references. **Applied**: comment removed, rename note added.
- 🔵 **Standards**: Phase 3 offered `hooks.json` (strict JSON) as a comment location. **Applied**: changed to a new `hooks/CLAUDE.md`.
- 🔵 **Correctness**: the `(bash|sh|env)…accelerator` wrapper regex over-matched prose (`sh` in `publish`); the Change 4 aside suggested `ACCELERATOR_BIN`, which would reintroduce the empty-environment gap the suite guards. **Applied**: anchored the wrapper to the leading command token; dropped the `ACCELERATOR_BIN` alternative.
- 🔵 **Compatibility / Security**: the local precedence check was framed as an ongoing mitigation. **Applied**: reframed — the real assurance is a fixed PATH prepend established from the changelog trace; added a cross-environment name-collision caveat.
- 🔵 **Test Coverage**: `test_launcher_token` sibling cases and the `TestOverBroadSkills` ancestor-glob params go vacuous under bare `LAUNCHER`. **Applied**: TDD Notes now specify a bare `test_launcher_token` case and a genuine bare-form ancestor-glob case.
- 🔵 **Architecture**: the config markers mildly broaden `skill_parsing`'s responsibility. **Applied**: note to extend the module docstring so the shared vocabulary is an intentional responsibility.

### Assessment

The plan is in good shape and ready for implementation. Two things carry into
implementation as gating preconditions rather than plan defects: the bin-on-PATH
**version floor** must be established by a changelog trace before bulk
conversion (with an inconclusive `!`-surface trace treated as stop), and the
**red-first TDD sequence** across the now-complete coupled test surface must be
followed so the atomic Phase 1 lands green. The Pass-2 refinements were applied
directly from the reviewers' stated suggestions and have not themselves been
independently re-reviewed.
