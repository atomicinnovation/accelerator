---
date: "2026-05-08T10:18:00Z"
type: plan-review
skill: review-plan
target: "meta/plans/2026-05-08-0030-centralise-path-defaults.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, code-quality, test-coverage, correctness, standards]
review_pass: 2
status: complete
---

## Plan Review: Centralise PATH and TEMPLATE Config Arrays

**Verdict:** COMMENT

The plan is a tightly-scoped, TDD-driven refactor that is architecturally
sound, line-number-accurate against HEAD, and faithful to the repo's
shared-bash-module conventions. Reviewers from all five lenses agree that
the dependency direction (config-common.sh sources config-defaults.sh,
arrays propagate transitively to existing callers) is correct, the AC2
grep encodes the centralisation invariant as an executable fitness
function, and the scope deferrals (DIR_KEYS/DIR_DEFAULTS, AGENT_KEYS,
review DEFAULTS, consumer-site refactoring) are honest. The plan is
acceptable as-is; the findings below describe quality improvements rather
than blockers — chiefly hardening the structural test against future
declaration-form drift, closing a regression-suite gap that lets
sourcing-chain breakage pass automated tests, and tightening a few
file/test cosmetics.

### Cross-Cutting Themes

- **AC2 grep is fragile to declaration-form drift** (flagged by:
  test-coverage, correctness, standards, architecture) — The pattern
  `'PATH_KEYS=\|PATH_DEFAULTS=\|TEMPLATE_KEYS='` only matches bare `=`
  assignments. Future regressions using `declare -a`, `readonly`,
  `export`, `local`, or `+=` would slip past the invariant test. Several
  reviewers also note the pattern matches anywhere on a line (including
  comments and fixture strings) and is sensitive to repo-wide layout
  changes. The work-item review pass 3 already raised this concern; the
  plan acknowledges the constraint but doesn't harden the test.
- **`grep -rln` output is filesystem-traversal-dependent and not
  normalised in the in-test assertion** (flagged by: code-quality,
  test-coverage, correctness) — The Phase 2 success-criteria command
  pipes through `sort -u`, but the embedded test assertion at lines
  320-326 does not. Failure messages will be order-unstable across
  environments when the invariant is breached.
- **`config-defaults.sh` name implies broader scope than delivered**
  (flagged by: architecture, code-quality) — The file holds path/template
  keys plus path defaults but excludes review DEFAULTS, AGENT_KEYS,
  AGENT_DEFAULTS, and DIR_DEFAULTS. The deferral is rational but the name
  signals a domain the file does not own.

### Tradeoff Analysis

- **Test brittleness vs scope creep** — Hardening the AC2 grep against
  alternative declaration forms (test-coverage, correctness, standards
  recommend) widens the regex and adds maintenance surface. The
  current narrow form is unambiguous and matches the project's
  consistent bare-`=` style. Recommendation: broaden the regex once
  to cover `declare`/`readonly`/`export`/`local`/`+=`, then leave it
  alone — the cost is low and the failure mode it prevents is silent.
- **Direct subshell sourcing in Phase 1 tests vs end-to-end transitive
  validation** — The plan validates `config-defaults.sh` in isolation
  (`bash -c "source ..."`) which gives clean failure messages; it does
  not exercise the production source chain. Adding one assertion against
  `config-dump.sh` output (paths.*/templates.* row presence) closes the
  gap without abandoning the isolation pattern.

### Findings

#### Critical

_(none)_

#### Major

- 🟡 **Test Coverage**: Regression suite does not assert paths.* or templates.* rows appear in config-dump output
  **Location**: Phase 2 Section 1 (AC2 test) and Testing Strategy / Regression Tests
  The existing `=== config-dump.sh ===` block enumerates `review.*`
  keys explicitly and asserts one `agents.*` row but never checks that
  any `paths.*` or `templates.*` row appears. If Phase 2's source line
  is omitted, mistyped, or sourced before its dependencies, config-dump
  could silently emit no path/template rows and the regression suite
  would not catch it — only the manual smoke check would.

#### Minor

- 🔵 **Architecture**: Module name and contents are mismatched in scope
  **Location**: Phase 1 Section 2 / Desired End State
  `config-defaults.sh` holds PATH/TEMPLATE keys + PATH defaults but not
  review DEFAULTS, AGENT_KEYS, AGENT_DEFAULTS, or DIR_DEFAULTS. The name
  implies a centralisation the file does not deliver.

- 🔵 **Architecture**: Parallel data definitions in init.sh remain a known architectural inconsistency
  **Location**: What We're NOT Doing — DIR_KEYS/DIR_DEFAULTS in init.sh
  After this plan lands, `init.sh` still defines its own bare-key
  default arrays that overlap conceptually with PATH_KEYS/PATH_DEFAULTS.
  AC2 fitness function does not detect drift between them.

- 🔵 **Code Quality**: Hardcoded expected-array literals in tests duplicate the data under test
  **Location**: Phase 1.1 (test snippet lines 187-200)
  The new tests re-state the literal entries inline. Catches reordering
  and deletion but is essentially `assert(file_says_X) == X` and
  requires lockstep edits to add a key.

- 🔵 **Code Quality**: Deleting `# Path keys` / `# Template keys` markers reduces in-file readability
  **Location**: Phase 2.3 (Remove inline array definitions)
  After deletion, three iteration loops in config-dump.sh sit
  back-to-back with no in-file breadcrumb to the new definition site.

- 🔵 **Code Quality**: Single-definition-site test is sensitive to grep ordering and exact path format
  **Location**: Phase 2.1 (lines 320-326)
  `MATCHES` from `grep -rln` is compared by exact string equality; no
  `sort -u`; failure diagnostics are obfuscated when the invariant is
  breached.

- 🔵 **Test Coverage**: AC2 grep is fragile to alternative array-definition forms
  **Location**: Phase 2 Section 1
  Pattern only matches bare `=`. Future use of `declare -a`,
  `readonly`, `export`, or `+=` would silently bypass the invariant.

- 🔵 **Test Coverage**: AC2 test relies on grep traversal order without normalisation
  **Location**: Phase 2 Section 1 (test snippet)
  Phase 2 success-criteria uses `sort -u`, but the embedded test does
  not — divergent contracts between the manual check and the assertion.

- 🔵 **Test Coverage**: Array tests don't independently verify length
  **Location**: Phase 1 Section 1
  Header announces "expected 11 entries" but only joined-string
  equality is asserted. Whitespace-free entries make this currently
  safe; not robust to future entries with embedded spaces.

- 🔵 **Correctness**: config-common.sh insertion displaces existing blank line; "line 9" assertion depends on it
  **Location**: Phase 2 Section 2 / Manual Verification
  At HEAD line 9 of config-common.sh is blank. The verification "line 9
  is `source config-defaults.sh`" only holds if the implementer
  replaces the blank rather than inserting after it.

- 🔵 **Correctness**: AC2 pattern only matches bare `=`, not `declare -a`/`readonly`/`local` forms
  **Location**: Phase 2 Section 1
  Mirror of the test-coverage finding from the correctness lens — the
  invariant could be silently broken by a non-bare future definition.

- 🔵 **Correctness**: AC2 assertion uses exact string match against grep -rln output
  **Location**: Phase 2 Section 1
  Filesystem-order-dependent if the assertion ever fires on multiple
  matches; brittle across macOS APFS vs Linux ext4.

- 🔵 **Correctness**: Content assertions implicitly depend on default IFS in `bash -c` subshell
  **Location**: Phase 1 Section 1
  `${PATH_KEYS[*]}` joins on first char of IFS; works because `bash -c`
  starts fresh. Latent fragility if the test is later refactored to
  share a shell.

- 🔵 **Correctness**: Plan does not call out `set -e` interaction with `bash -c` subshells on first failing run
  **Location**: Overview / Phase 1 prerequisites
  The first run before file creation is correct (assignment-via-cmd-sub
  suppresses set -e), but the cadence is not stated and could confuse a
  manual TDD walk-through.

- 🔵 **Standards**: New sourceable module omits shebang line, departing from sibling convention
  **Location**: Phase 1 Section 2
  Every other shared module in `scripts/` (`config-common.sh`,
  `vcs-common.sh`, `atomic-common.sh`, `log-common.sh`,
  `work-item-common.sh`) starts with `#!/usr/bin/env bash`. The plan
  explicitly directs this file to omit it.

- 🔵 **Standards**: Grep invariant test couples integration suite to filesystem layout
  **Location**: Phase 2 Section 1
  The structural-lint pattern is novel in `test-config.sh` — the rest
  of the file tests behaviour, not source layout. A test-only file
  mentioning `PATH_KEYS=` in a fixture string would fail.

#### Suggestions

- 🔵 **Architecture**: Phase 1 tests source config-defaults.sh directly, not through the production chain
  **Location**: Phase 1 Section 1 / Phase 2 Section 1
  Add one in-process assertion (`${#PATH_KEYS[@]} == 11`) after
  test-config.sh's existing `source config-common.sh` to validate the
  transitive chain.

- 🔵 **Architecture**: Fitness-function test couples to repo-wide layout via grep
  **Location**: Phase 2 Section 1
  Consider scoping the grep to `scripts/` or documenting the
  exclusion contract in a comment.

- 🔵 **Code Quality**: File name `config-defaults.sh` is broader than its contents
  **Location**: Phase 1 Section 2
  Either rename (e.g. `config-path-keys.sh`) or expand the banner
  comment to enumerate what is *not* in this file.

- 🔵 **Code Quality**: Three repeated `# shellcheck disable=SC2034` directives could be collapsed
  **Location**: Phase 1 Section 2
  Use a single file-scope directive after the banner comment.

- 🔵 **Test Coverage**: Smoke test for transitive sourcing is manual; could be automated cheaply
  **Location**: Phase 2 Manual Verification
  Convert manual step 2 (paths.plans override fixture) into an
  automated assertion — ~15 lines following the existing config-dump
  test pattern.

- 🔵 **Standards**: Banner comment style differs from sibling modules
  **Location**: Phase 1 Section 2
  Restructure to one-line purpose, blank line, then rationale —
  matching `atomic-common.sh:1-12` and friends.

- 🔵 **Standards**: `bash -c` isolation pattern is novel in test-config.sh
  **Location**: Phase 1 Section 1
  Replace with `(...)` subshell — same isolation, simpler quoting,
  matches the rest of the file.

### Strengths

- ✅ Phases follow strict TDD — each phase's failing test captures the
  invariant being established, and Phase 2's AC2 grep test is what
  drives the deletion step.
- ✅ Dependency direction is correct: config-common.sh (function module)
  sources config-defaults.sh (data module) and propagates transitively
  to all existing callers without per-consumer edits.
- ✅ Sourcing pattern reuses the established `$SCRIPT_DIR` self-resolution
  idiom — no `${CLAUDE_PLUGIN_ROOT}` introduced.
- ✅ Every line-number citation (config-dump.sh:175-187, 189-201,
  212-219; comment lines 174 and 211; config-common.sh:8;
  test-config.sh:2426, 2606, 22, 20) verifies cleanly at HEAD.
- ✅ Array contents and order in the proposed file mirror the inline
  definitions byte-for-byte across all three arrays.
- ✅ Iteration loops at config-dump.sh:203-209 and 221-229 reference
  only PATH_KEYS, PATH_DEFAULTS, TEMPLATE_KEYS, READ_VALUE, and
  get_source — none broken by the deletion.
- ✅ Scope deferrals (DIR_KEYS/DIR_DEFAULTS, AGENT_KEYS, review
  DEFAULTS, consumer-site refactoring, TEMPLATE_DEFAULTS,
  config-read-path.sh comment-only block) are explicit and rationalised.
- ✅ Single-definition-site invariant is encoded as an executable fitness
  function — future drift is detected by CI, not just discouraged by
  convention.
- ✅ Per-line `# shellcheck disable=SC2034` placement mirrors the
  precedent at config-common.sh:10.
- ✅ Test block insertion anchor is correct (immediately before
  `=== config-dump.sh ===` at line 2426) and uses the assert_eq /
  inline `if [ -f ]` mix found in surrounding blocks.

### Recommended Changes

1. **Add a paths.* / templates.* row presence assertion** (addresses:
   "Regression suite does not assert paths.* or templates.* rows", "Phase
   1 tests source config-defaults.sh directly", "Smoke test for
   transitive sourcing is manual")
   In the new `=== config-defaults.sh ===` block (or by extending the
   existing `=== config-dump.sh ===` block), run config-dump.sh against
   a minimal fixture and assert at least one `paths.*` and one
   `templates.*` row appears. Closes the regression-suite gap and
   converts the manual smoke check into an automated guard. This is
   the highest-value change.

2. **Harden the AC2 grep to cover alternative declaration forms**
   (addresses: "AC2 grep is fragile to alternative declaration forms",
   "AC2 pattern only matches bare `=`")
   Broaden the regex to also match `declare -a PATH_KEYS`, `readonly
   PATH_KEYS`, `export PATH_KEYS`, `local PATH_KEYS`, and `PATH_KEYS+=`
   for each of the three names. Use `grep -E` and a single combined
   pattern. Cheap, future-proofs the invariant.

3. **Pipe the in-test grep through `sort -u`** (addresses: "AC2 test
   relies on grep traversal order", "Single-definition-site test is
   sensitive to grep ordering")
   Make the test assertion contract match the Phase 2 success-criteria
   verification command. One-line change.

4. **Loosen the "line 9" manual-verification check** (addresses:
   "config-common.sh insertion displaces existing blank line")
   Reword to "config-common.sh sources config-defaults.sh immediately
   after vcs-common.sh" — line-number-agnostic, robust to the existing
   blank line.

5. **Add `#!/usr/bin/env bash` shebang to config-defaults.sh** (addresses:
   "New sourceable module omits shebang line")
   Match every other `*-common.sh` sibling. Harmless on a sourced file;
   removes a stand-out inconsistency.

6. **Preserve in-file breadcrumb comments in config-dump.sh** (addresses:
   "Deleting `# Path keys` / `# Template keys` markers reduces
   readability")
   Replace each deleted marker with a one-line comment such as
   `# Path keys (defined in config-defaults.sh)` above the surviving
   loops at 203-209 and 221-229.

7. **Replace `bash -c` with `(...)` subshell in Phase 1 tests** (addresses:
   "`bash -c` isolation pattern is novel in test-config.sh", "Content
   assertions implicitly depend on default IFS")
   `ACTUAL_PATH_KEYS=$( source "$DEFAULTS_FILE" && echo "${PATH_KEYS[*]}" )`
   gives the same isolation with simpler quoting and matches the
   surrounding test style.

8. **Document scope in config-defaults.sh banner** (addresses: "Module
   name and contents are mismatched in scope", "File name is broader
   than its contents")
   Add one sentence to the banner comment enumerating what is not
   included (review DEFAULTS, AGENT_KEYS/AGENT_DEFAULTS, DIR_KEYS in
   init.sh) and why.

9. **Collapse the three SC2034 directives to a single file-scope
   directive** (addresses: "Three repeated `# shellcheck disable=SC2034`
   directives could be collapsed")
   One directive after the banner, before the first array. Reduces
   noise; future arrays inherit the suppression.

10. **Add explicit length assertions** (addresses: "Array tests don't
    independently verify length")
    Prefix each content assertion with an `assert_eq "<NAME> length"
    "11" "<actual>"` so the announced "expected 11 entries" is checked
    independently of the joined-string contents.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: This is a tightly-scoped, architecturally sound refactor
that improves cohesion by centralising shared array definitions and uses
an enforceable structural invariant (the AC2 grep) as a fitness
function. The dependency direction is correct (common depends on
defaults; consumers get them transitively), the existing $SCRIPT_DIR
sourcing pattern is preserved, and scope deferrals (init.sh's DIR_KEYS,
AGENT_KEYS, review DEFAULTS) are explicitly acknowledged. Main
observations are about asymmetric extraction scope, the temporary
parallel-data condition with init.sh, and a couple of design questions
around the module's name and how the test exercises the source chain.

**Strengths**:
- Dependency direction is correct: config-common.sh (function module)
  sources config-defaults.sh (data module) and propagates transitively
  to existing callers without per-consumer edits.
- Uses an architectural fitness function — the AC2 grep invariant is
  encoded as an automated test, so future drift back to inline
  definitions is detected, not just discouraged by convention.
- Sourcing pattern reuses the established $SCRIPT_DIR self-resolution
  idiom and explicitly avoids ${CLAUDE_PLUGIN_ROOT}.
- Scope is honest: the plan calls out that consumer-site defaults,
  DIR_KEYS/DIR_DEFAULTS in init.sh, and TEMPLATE_DEFAULTS are
  explicitly out of scope.
- TDD sequencing is well-thought-out.

**Findings**:
- **minor / high — Module name and contents are mismatched in scope**
  (Phase 1 Section 2 / Desired End State): name signals broader
  domain than file owns; suggest banner comment or rename.
- **minor / high — Parallel data definitions in init.sh remain a known
  architectural inconsistency** (What We're NOT Doing): AC2 fitness
  function doesn't detect divergence between PATH_DEFAULTS and
  init.sh's DIR_DEFAULTS.
- **suggestion / medium — Phase 1 tests source config-defaults.sh
  directly, not through the production chain** (Phase 1 Section 1):
  add one in-process assertion to validate transitive source.
- **suggestion / medium — Fitness-function test couples to repo-wide
  layout via grep** (Phase 2 Section 1): scope grep to `scripts/` or
  document exclusion contract.

### Code Quality

**Summary**: The plan is a small, focused refactor with a clean TDD
structure and clear separation between Phase 1 (introduce + populate)
and Phase 2 (collapse to single definition site). The proposed code
mirrors existing sibling-module conventions. A few minor maintainability
concerns relate to test brittleness (literal expected strings), the
loss of in-file section markers in `config-dump.sh`, and a slight
name/scope mismatch.

**Strengths**:
- Phases follow strict TDD — each phase produces a self-contained,
  reversible step.
- New module fits established convention (no shebang [disputed by
  standards lens], sourced not executed, SC2034 disable mirroring
  config-common.sh:10, $SCRIPT_DIR pattern after vcs-common.sh).
- Single-definition-site invariant is encoded as an executable test.
- Scope is explicit and disciplined.
- Banner comment explains the rationale.

**Findings**:
- **minor / medium — Hardcoded expected-array literals in tests
  duplicate the data under test** (Phase 1.1, lines 187-200): test is
  essentially `assert(file_says_X) == X`.
- **minor / medium — Deleting `# Path keys` / `# Template keys` section
  markers reduces in-file readability** (Phase 2.3).
- **suggestion / low — File name `config-defaults.sh` is broader than
  its contents** (Phase 1.2).
- **minor / medium — Single-definition-site test is sensitive to grep
  output ordering and exact path format** (Phase 2.1, lines 320-326):
  no `sort -u` in test; success criteria has it.
- **suggestion / low — Three repeated `# shellcheck disable=SC2034`
  directives could be collapsed** (Phase 1.2): single file-scope
  directive.

### Test Coverage

**Summary**: The plan is genuinely TDD-driven and the structural tests
are well-targeted at the immediate refactor. However, the regression
suite has a latent gap — the existing config-dump.sh tests do not
assert that paths.* or templates.* rows appear, so a Phase 2 mistake
that breaks transitive sourcing could pass automated tests. The AC2
grep test is also brittle: it only matches bare `=` assignment forms
and does not normalise grep's traversal-order output.

**Strengths**:
- Each phase introduces a failing test before the production change.
- Array assertions verify both length (implicitly) and order.
- Subshell isolation pattern correctly avoids contaminating test state.
- Existing config-read-path.sh test block provides solid behavioural
  coverage of every paths.* key.

**Findings**:
- **major / high — Regression suite does not assert paths.* or
  templates.* rows appear in config-dump output** (Phase 2 Section 1
  / Testing Strategy): existing tests only check `review.*` and one
  `agents.*` row.
- **minor / high — AC2 grep is fragile to alternative array-definition
  forms** (Phase 2 Section 1): bare `=` only; misses `declare -a`,
  `readonly`, `export`, `+=`.
- **minor / medium — AC2 test relies on grep traversal order without
  normalisation** (Phase 2 Section 1): no `sort -u` in the embedded
  test.
- **minor / medium — Array tests don't independently verify length**
  (Phase 1 Section 1): joined-string equality only.
- **suggestion / medium — Smoke test for transitive sourcing is
  manual; could be automated cheaply** (Phase 2 Manual Verification):
  ~15 lines following the existing pattern.

### Correctness

**Summary**: Correctness foundations are solid: every line-number
citation verifies cleanly at HEAD; the AC2 grep pattern matches exactly
the three current definition sites; the source chain ordering is
correct. Phase ordering, TDD cadence, and array contents/order all
check out byte-for-byte. A small number of minor edge cases warrant
note but none block correctness.

**Strengths**:
- All line-number citations accurate at HEAD.
- Array contents and order match inline definitions byte-for-byte.
- AC2 grep pattern produces no false positives.
- Source-chain insertion point at config-common.sh:8 is correct.
- Iteration loops at config-dump.sh:203-209 and 221-229 don't reference
  any other variables that would break after deletion.
- Phase ordering is internally consistent.
- assert_eq at test-helpers.sh:19 implements expected semantics.

**Findings**:
- **minor / high — config-common.sh insertion displaces existing blank
  line; "line 9" assertion depends on that displacement** (Phase 2
  Section 2 / Manual Verification).
- **minor / medium — AC2 grep pattern only matches bare `=`
  assignments, not `declare -a` / `readonly` / `local` forms** (Phase
  2 Section 1).
- **minor / medium — AC2 assertion uses exact string match against
  grep -rln output, which is filesystem-order-dependent** (Phase 2
  Section 1).
- **minor / high — Content assertions implicitly depend on default IFS
  in the bash -c subshell** (Phase 1 Section 1): currently correct;
  latent fragility on refactor.
- **minor / high — Plan does not assert that test-config.sh's `set
  -euo pipefail` cohabits with `bash -c` subshells that may exit
  non-zero on first failing run** (Overview / Phase 1 prereqs):
  documentation hygiene only.

### Standards

**Summary**: The plan generally aligns with the repo's bash conventions
for shared modules (universal `source "$SCRIPT_DIR/<file>"` form, bare
array assignment, per-variable SC2034 directives, no
`set -euo pipefail` in sourced files). Two notable deviations: the new
file is specified to omit the `#!/usr/bin/env bash` shebang where every
sibling has one; and the test block uses `bash -c "source ..."` rather
than the simpler `(source ... && echo ...)` subshell idiom seen
elsewhere.

**Strengths**:
- Sourcing line uses universal `source "$SCRIPT_DIR/..."` form.
- Insertion point at config-common.sh:8 follows layered-source pattern.
- Per-line `# shellcheck disable=SC2034` placement mirrors precedent
  at config-common.sh:10.
- Preserves bare `NAME=( ... )` array form.
- Test block insertion uses correct anchor and matching style.

**Findings**:
- **minor / high — New sourceable module omits shebang line, departing
  from sibling convention** (Phase 1 Section 2): every other
  `*-common.sh` sibling starts with `#!/usr/bin/env bash`.
- **suggestion / medium — Banner comment style differs from sibling
  modules** (Phase 1 Section 2): one-line purpose, blank, rationale.
- **suggestion / medium — `bash -c` isolation pattern is novel in this
  test file** (Phase 1 Section 1): use `(...)` subshell instead.
- **minor / medium — Grep invariant test couples test suite to
  filesystem layout** (Phase 2 Section 1): consider tightening regex
  or moving to a lint task.

## Re-Review (Pass 2) — 2026-05-08T10:18:00Z

**Verdict:** COMMENT

The plan's edits resolve the major regression-suite gap (paths.* /
templates.* row presence is now an automated assertion) and address
nearly every minor finding from pass 1. A critical defect was
introduced in pass 1 — the row-presence grep used `^` anchors against
config-dump.sh output that begins with `| \``, so the assertion would
have failed in both phases of the TDD sequence. That defect was fixed
mid-re-review (regex changed to `grep -qF '| \`paths.plans\` |'`).
Remaining concerns are all minor or suggestion-level and do not block
implementation.

### Previously Identified Issues

**Architecture**
- 🔵 **architecture**: Module name and contents are mismatched in scope — **Resolved** (banner Scope note enumerates excluded arrays)
- 🔵 **architecture**: Parallel data definitions in init.sh — **Acknowledged** (already deferred; now visible in three places)
- 🔵 **architecture**: Phase 1 tests source config-defaults.sh directly — **Partially resolved** (row-presence test exercises transitive chain end-to-end in Phase 2; still architecturally indirect)
- 🔵 **architecture**: Fitness-function test couples to repo-wide layout — **Still present** (suggestion to scope grep to `scripts/` was not adopted; `--exclude-dir=workspaces` retained)

**Code Quality**
- 🔵 **code-quality**: Hardcoded expected-array literals — **Still present** (intentional; literals are the test specification)
- 🔵 **code-quality**: Deleting `# Path keys` / `# Template keys` markers — **Resolved** (breadcrumbs `(defined in config-defaults.sh)` preserved)
- 🔵 **code-quality**: File name broader than contents — **Resolved** (banner Scope note)
- 🔵 **code-quality**: Single-definition-site test sensitive to grep ordering — **Resolved** (`sort -u` added to in-test assertion)
- 🔵 **code-quality**: Three repeated SC2034 directives — **Resolved** (collapsed to single file-scope directive)

**Test Coverage**
- 🟡 **test-coverage**: Regression suite does not assert paths.* / templates.* rows — **Resolved** after pass 2 fix (assertion added; initial regex bug corrected mid-review)
- 🔵 **test-coverage**: AC2 grep fragile to alternative declaration forms — **Mostly resolved** (declare/readonly/export/local/+= covered; `declare -ga` and `typeset` still slip through)
- 🔵 **test-coverage**: AC2 traversal-order normalisation — **Resolved**
- 🔵 **test-coverage**: Array tests don't independently verify length — **Resolved** (explicit length assertions added)
- 🔵 **test-coverage**: Manual smoke check duplicates automated coverage — **Partially resolved** (no-config and override-renders cases automated; templates.* override still manual)

**Correctness**
- 🔵 **correctness**: config-common.sh "line 9" assertion — **Resolved** (line-number-agnostic wording)
- 🔵 **correctness**: AC2 pattern only matches bare `=` — **Resolved**
- 🔵 **correctness**: AC2 exact string match against grep -rln output — **Resolved** (`sort -u`)
- 🔵 **correctness**: Content assertions implicitly depend on default IFS — **Resolved** (`(...)` subshell pattern adopted)
- 🔵 **correctness**: `set -e` interaction with subshell on first failing run — **Resolved implicitly** (cmd-substitution suppresses set -e in `VAR=$(...)` form; new `(...)` subshell pattern preserves the same behaviour)

**Standards**
- 🔵 **standards**: Shebang missing — **Resolved** (`#!/usr/bin/env bash` added)
- 🔵 **standards**: Banner style differs from siblings — **Resolved** (structured form adopted; new finding notes it is somewhat verbose)
- 🔵 **standards**: `bash -c` novel in test file — **Resolved** (`(...)` subshell)
- 🔵 **standards**: Grep invariant couples to filesystem layout — **Resolved** (anchored at line start; matches comments/literals no longer trip)

### New Issues Introduced

- 🔴 **test-coverage / correctness**: Row-presence grep used `^paths\.plans` / `^templates\.plan` anchors against config-dump output that begins with `| \`` — assertion would have failed in Phase 1 (when it should pass trivially) and Phase 2. **Fixed mid-re-review**: pattern changed to `grep -qF '| \`paths.plans\` |' && grep -qF '| \`templates.plan\` |'` so the test now matches the actual rendered table-row format.

- 🔵 **test-coverage**: AC2 declaration-form alternation accepts only flag letters `[aArx]` and excludes `typeset`. A future contributor writing `declare -ga PATH_KEYS=...` (global+array) or `typeset -a PATH_KEYS=...` would silently bypass the invariant. Suggested fix: broaden flag class to `[a-zA-Z]+` and add `typeset` as a sibling alternation.

- 🔵 **standards**: File-scope `# shellcheck disable=SC2034` placement is a one-off in the codebase — every other `*-common.sh` sibling places SC2034 disables on the line directly above each variable. The plan's collapse to one directive is correct shellcheck behaviour but introduces a placement style that doesn't match siblings.

- 🔵 **suggestion / standards**: Banner is ~14 lines — significantly longer than the 2-9 line banners in sibling modules. Most of the scope-note content duplicates information in the plan's "What We're NOT Doing" and the work item.

- 🔵 **suggestion / code-quality**: Six repeated `( source "$DEFAULTS_FILE" && echo ... )` subshells — one per length/contents assertion per array. Could consolidate to one subshell that emits a delimited record. Readability vs cleverness trade-off.

- 🔵 **suggestion / test-coverage**: Joined-string equality assertion produces hard-to-read diffs on failure. Per-index loop would localise drift to a specific index. Marginal.

- 🔵 **suggestion / architecture**: Transitive-chain assertion is architecturally indirect — depends on config-dump.sh's rendering rather than the chain itself. Adding `${#PATH_KEYS[@]} == 11` after test-config.sh:20's existing source line would directly assert chain delivery into the test process.

### Assessment

The plan is implementation-ready. The critical regex bug was introduced
during pass 1 edits and has been fixed; the underlying TDD sequence
now works as the plan describes. Remaining minors and suggestions are
quality improvements that the implementer can fold in or defer at
their discretion. Most impactful unaddressed item: broadening the AC2
regex to cover `declare -ga` and `typeset` (one-line change, prevents
silent invariant breakage). Recommend leaving the architectural
suggestions and stylistic minors as-is unless the user prefers a
further pass.
