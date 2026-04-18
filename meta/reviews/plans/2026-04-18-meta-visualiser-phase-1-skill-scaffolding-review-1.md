---
date: "2026-04-18T16:30:00+01:00"
type: plan-review
skill: review-plan
target: "meta/plans/2026-04-18-meta-visualiser-phase-1-skill-scaffolding.md"
review_number: 1
verdict: COMMENT
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability, portability]
review_pass: 3
status: complete
---

## Plan Review: Meta Visualiser — Phase 1: Skill Scaffolding and No-Op Preprocessor

**Verdict:** REVISE

The plan is well-structured, TDD-disciplined, and faithfully mirrors
established canonical patterns (SKILL.md preamble, bash test harness,
plugin-manifest shape, skill directory layout). Forward-compatibility
seams are considered and the sub-phase decomposition is sound.
However, one critical issue makes the plan's own Phase 1.5 Success
Criterion unsatisfiable as written, and several major concerns cluster
around three themes: the `resolve_self` portability design, the 11-path
preamble carrying no Phase 1 consumer, and usability of the placeholder
URL that looks like a real endpoint.

### Cross-Cutting Themes

Four themes were flagged by two or more lenses and deserve the most
attention during revision:

- **Test-harness helper duplication** (flagged by: architecture,
  code-quality, test-coverage) — `assert_eq` / `assert_exit_code` /
  PASS-FAIL bookkeeping is about to be copied for a third and fourth
  time across the test suites. Extraction into a single shared
  `test-helpers.sh` is cheapest now, while only two net-new suites are
  being added.
- **`resolve_self` portability design** (flagged by: code-quality,
  correctness, portability, test-coverage) — the `readlink -f /` probe
  is ambiguous across BSD `readlink` versions, the `perl` fallback is
  not guaranteed on all supported hosts (nor enforced by
  `mise.toml`), and CI runs Linux-only so the fallback branch has zero
  coverage. A pure-bash `cd … && pwd -P` + symlink-walk idiom removes
  the probe, the dependency, and the untested branch in one move.
- **SKILL.md resolves 11 path keys that Phase 1 never consumes**
  (flagged by: architecture, correctness, usability) — the preamble
  renders ~11 directory paths to the user while the only functional
  payload is the stub URL. Described by the plan as
  forward-compatibility insurance, but Phase 2 may want paths written
  to `config.json` rather than surfaced in the skill body, so the
  insurance is paid against a risk that may not materialise.
- **Placeholder URL looks like a real endpoint** (flagged by:
  architecture, usability) — `http://localhost:0000` is visually
  indistinguishable from a failed-to-start server. A savvy user who
  pastes it into a browser hits `ERR_CONNECTION_REFUSED` rather than a
  clear "scaffold, not yet running" signal. The downstream consumption
  contract between `launch-server.sh` stdout and the SKILL.md's
  embedded `!`-invocation will also change shape in Phase 2 and is not
  documented as a known churn point.

### Tradeoff Analysis

- **Forward-compatibility vs. YAGNI** — The plan argues for resolving
  all 11 path keys now so the preamble shape is stable through Phase
  2+. The correctness, usability, and architecture lenses argue the
  opposite: defer until Phase 2 has a real consumer. Recommendation:
  keep the preamble but explicitly annotate it as forward-compat
  scaffolding, or push the resolutions out of the visible SKILL body
  until they are load-bearing.
- **Red-first TDD rigour vs. process overhead** — The test-coverage
  lens wants the red-first discipline to be auditable (separate
  commit, mutation smoke-test), which adds mild process friction. The
  plan currently relies on an honour-system checklist item. For a
  scaffolding phase the honour system may be acceptable; raise the bar
  in Phase 2 when real behaviour starts accumulating.

### Findings

#### Critical

- 🔴 **Correctness**: Adding the visualise SKILL.md breaks three hard-coded count invariants in `scripts/test-config.sh`
  **Location**: Phase 1.3 + Phase 1.5
  The existing `scripts/test-config.sh` (already wired into
  `tasks/test.py`) hard-codes `"13"` as the expected skill count in
  three separate assertions (lines 1025, 2859, 2863) and does not
  include `visualisation/visualise` in its `ALL_SKILLS` / `CONTEXT_SKILLS`
  arrays. The plan's own Success Criterion
  (`mise run test:integration` exits 0) is therefore unsatisfiable as
  written — the first existing suite fails before the new suites run.

#### Major

- 🟡 **Architecture**: Plain-text URL contract between stub and SKILL.md has no abstraction layer
  **Location**: Phase 1.3 `**Visualiser URL**` line
  Stub stdout is surfaced verbatim via `!`-prefixed bash. Phase 2's
  richer output (URL, PID, status, log path) cannot fit the single-line
  shape without the SKILL.md's consumption changing. The plan should
  explicitly flag this as a known Phase 2 churn point.

- 🟡 **Code Quality**: Third and fourth copy of `assert_eq` / `assert_exit_code` / PASS-FAIL bookkeeping
  **Location**: Phase 1.1 + Phase 1.2 test harnesses
  `scripts/test-config.sh` and `skills/decisions/scripts/test-adr-scripts.sh`
  already duplicate the helpers. Phase 1 adds copies 3 and 4 in a
  single phase. Extract to a sourced `scripts/test-helpers.sh` now
  while the refactor cost is amortised.

- 🟡 **Test Coverage**: No meta-enforcement that new test suites get registered
  **Location**: Phase 1.5 `tasks/test.py`
  The hand-curated list means any future phase that forgets to
  register a `test-*.sh` produces a silently-unrun suite. Discovery
  via glob (`**/test-*.sh`) or a meta-test that fails on unregistered
  suites guarantees the enrolment invariant.

- 🟡 **Test Coverage**: Red-first TDD discipline is verified by honour system only
  **Location**: Phase 1.1 and 1.2 Manual Verification
  The plan's central claim ("tests fail before implementation") is a
  checklist item with no automated audit. Either commit tests and
  implementation separately (so `git log` preserves the transition) or
  add a mutation smoke-test that temporarily removes the stub and
  asserts the suite fails.

- 🟡 **Usability**: Placeholder `http://localhost:0000` looks like a real URL
  **Location**: Phase 1.1 stub + Phase 1.3 SKILL.md body
  Visually indistinguishable from a failed-to-start server. Consider a
  clearly-fake scheme (`placeholder://phase-1-scaffold-not-yet-running`)
  or relabel the line as `**Visualiser URL (not yet running)**`.

- 🟡 **Usability**: Eleven resolved paths rendered to the user but never used in Phase 1
  **Location**: Phase 1.3 SKILL.md body — 11 path-key `!`-invocations
  A user invoking `/accelerator:visualise` sees ~11 resolved
  directory paths with no functional role and a one-line payload.
  First-run impression is busy and fragmented.

- 🟡 **Portability**: `perl` fallback is not guaranteed on supported hosts
  **Location**: Phase 1.2 CLI wrapper `resolve_self`
  `mise.toml` does not pin `perl`; Alpine and minimal Docker/distroless
  images commonly omit it. Neither branch of `resolve_self` has a
  graceful degradation path. Pure-bash symlink-walk using only
  `readlink` / `cd` / `pwd -P` / `dirname` / `basename` avoids the
  dependency entirely.

- 🟡 **Portability**: `readlink -f /` probe is an unreliable indicator of GNU-style `readlink`
  **Location**: Phase 1.2 CLI wrapper `resolve_self` probe
  BSD `readlink` behaviour varies across macOS versions (pre-12.3
  rejects `-f`; 12.3+ accepts it with subtly different semantics). The
  probe gives different branch selection on different macOS versions.
  Sidestep via pure-bash or probe with a purpose-built temp symlink.

- 🟡 **Portability**: CI runs Linux only; macOS branch of `resolve_self` is never exercised
  **Location**: Phase 1.5 wiring + `.github/workflows/main.yml`
  Ubuntu-only CI means the BSD/Perl branch has zero coverage despite
  being introduced specifically for macOS. Add a macOS matrix entry or
  force-fallback test on Linux.

#### Minor

- 🔵 **Architecture**: `assert_eq` / `assert_exit_code` duplicated across three test suites with no shared library
  **Location**: Phase 1.1 & 1.2 test harnesses
  (overlaps with the code-quality and test-coverage findings above;
  kept separate here because the architectural concern is coupling
  propagation to future phases).

- 🔵 **Architecture**: SKILL.md resolves 11 path keys but Phase 1 stub consumes none
  **Location**: Phase 1.3 SKILL.md preamble
  Forward-compat insurance against a Phase 2 risk that may not
  materialise. Mark as forward-compat explicitly or defer.

- 🔵 **Architecture**: CLI wrapper's plugin-root resolution is not tested end-to-end
  **Location**: Phase 1.2 wrapper
  `SKILL_ROOT` resolution relies on the `cli/ ↔ scripts/` sibling
  contract; no test asserts it works from a relocated tree. Phase 2
  refactors of `scripts/` could silently break the wrapper.

- 🔵 **Code Quality**: `readlink -f /` capability probe is clever but opaque
  **Location**: Phase 1.2 `resolve_self`
  (overlaps with portability finding). Add an inline comment, use
  `readlink --version 2>/dev/null | grep -q GNU`, or drop the fast-path.

- 🔵 **Code Quality**: Executable-bit test duplicates PASS/FAIL bookkeeping inline
  **Location**: Phase 1.1 & 1.2 Test 1
  Hand-rolled counter logic despite `assert_*` helpers being declared
  in the same file. Factor into an `assert_file_executable` helper.

- 🔵 **Code Quality**: `assert_exit_code` silences stderr/stdout, obscuring CI failures
  **Location**: Phase 1.1 Tests 2 & 5, Phase 1.2 Tests 2 & 5
  On failure the harness reports only `Expected / Actual` exit codes;
  the actual error message is discarded. Capture and print stderr on
  failure.

- 🔵 **Code Quality**: Per-suite `context.run` invocations will scale poorly
  **Location**: Phase 1.5 `tasks/test.py`
  By Phase 5 this task will be 10+ near-identical three-line blocks.
  Iterate `(banner, path)` tuples or discover `**/test-*.sh`.

- 🔵 **Code Quality**: `mktemp -d` and `trap` created for tests that don't use them
  **Location**: Phase 1.2 test harness setup
  Only symlink tests (4 & 5) consume `$TMPDIR_BASE`. Move inline or
  comment its scope.

- 🔵 **Code Quality**: Executable-bit discipline relies on human prose, not automation
  **Location**: Phase 1.1 & 1.2 "Must be `chmod +x`"
  Plan narrates the requirement and tests assert it, but no step
  actually *sets* the bit. Add `chmod +x` as an explicit checklist
  item in each sub-phase.

- 🔵 **Test Coverage**: Parity test does not prove delegation
  **Location**: Phase 1.2 Test 3
  `wrapper_output = stub_output` would still pass if someone inlined
  `echo http://localhost:0000` into the wrapper. Use a sentinel
  (UUID → temp file) to prove the wrapper actually exec'd the stub.

- 🔵 **Test Coverage**: No assertion that stderr is silent
  **Location**: Phase 1.1 & 1.2 happy-path tests
  Stderr noise would leak into Claude's rendered skill expansion.
  Add one `STDERR=$(bash ... 2>&1 >/dev/null)` check per script.

- 🔵 **Test Coverage**: Cross-platform `readlink` fallback (perl branch) is never exercised
  **Location**: Phase 1.2 `resolve_self` branches
  (overlaps with portability CI finding). Force the fallback branch on
  Linux by stripping `readlink` from `PATH` in a dedicated test.

- 🔵 **Test Coverage**: Argument forwarding through the wrapper is untested
  **Location**: Phase 1.2 wrapper + tests
  `exec "..." "$@"` quoting mistakes would only surface in Phase 2.
  Temporarily echo `$@` from the stub, or assert exit 0 with `--foo bar`.

- 🔵 **Test Coverage**: No assertion that the URL line lacks leading/trailing whitespace or BOM
  **Location**: Phase 1.1 Test 3
  `$(...)` strips trailing newlines; whitespace contamination would
  pass. Use a byte-exact comparison (`grep -x -F`, `od`, or `xxd`).

- 🔵 **Correctness**: Prose claims 'alphabetical' ordering but the manifest is not alphabetically ordered
  **Location**: Phase 1.4 prose
  The existing array is workflow-grouped, not alphabetical; the
  plan's "alphabetically after output-formats" phrasing is inaccurate.

- 🔵 **Correctness**: Resolving all 11 paths introduces failure surface without corresponding value
  **Location**: Phase 1.3 SKILL.md preamble
  (overlaps with architecture + usability). A regression in
  `config-read-path.sh` surfaces 11× in a skill that consumes none of
  the values.

- 🔵 **Correctness**: Perl fallback assumes perl is on PATH in every supported environment
  **Location**: Phase 1.2 wrapper
  (overlaps with portability finding). Replace with pure-bash or emit
  a clear diagnostic on dependency absence.

- 🔵 **Standards**: `argument-hint` diverges from the only existing no-argument precedent
  **Location**: Phase 1.3 frontmatter
  `skills/config/init/SKILL.md` uses `"(no arguments — safe to run repeatedly)"`.
  Align to preserve the nascent convention.

- 🔵 **Standards**: Description over-promises Phase 1 behaviour
  **Location**: Phase 1.3 frontmatter `description`
  "Launches a companion-window server and prints its URL" is
  aspirational — Phase 1 ships no server. Use a stub-accurate
  description now and upgrade in Phase 2.

- 🔵 **Standards**: `cli/` subdirectory is a new skill-layout convention
  **Location**: Phase 1.2 wrapper path
  First skill to ship a `cli/` directory alongside `scripts/`.
  Document the intent (user-facing entry points intended for `$PATH`
  vs internal scripts).

- 🔵 **Standards**: Manifest insertion point not justified against a stated ordering rule
  **Location**: Phase 1.4 manifest array
  Existing order is workflow-grouped, not alphabetical. State the
  rule or adopt alphabetical explicitly.

- 🔵 **Usability**: CLI wrapper's `$PATH` install path is a silent oral tradition
  **Location**: Phase 1.2 manual verification + Desired End State item 2
  Users have no discoverable way to learn the CLI exists. Add a
  "How to use as a terminal command" line to the SKILL.md status
  block, and/or flag the Phase 12 README update as the commit-point.

- 🔵 **Usability**: `argument-hint` misses an opportunity to signal Phase 1 state
  **Location**: Phase 1.3 frontmatter
  (overlaps with the standards finding). Consider
  `"(no arguments — Phase 1 scaffold; no server yet)"` until Phase 2.

- 🔵 **Usability**: Status block speaks to Claude about phases, not to the end user
  **Location**: Phase 1.3 SKILL.md status section
  "Phase 2" / "server bootstrap" is insider jargon for end users.
  Rephrase as user-facing copy (e.g. "the visualiser UI isn't ready
  yet — this is a scaffold release").

- 🔵 **Usability**: No guidance for the executable-bit-missing failure mode
  **Location**: Phase 1.1 / 1.2 implementation
  If `chmod +x` is lost in transit, the user sees an unexplained
  `Permission denied`. Invoke the stub via `bash "$SCRIPT"` from
  SKILL.md to sidestep the dependency.

- 🔵 **Portability**: Symlink-based test can fail on filesystems without symlink support
  **Location**: Phase 1.2 `test-cli-wrapper.sh`
  WSL-on-NTFS and some container bind-mounts reject `ln -s`. Wrap
  the symlink test in a capability check that skips rather than fails
  on EPERM/EINVAL.

#### Suggestions

- 🔵 **Architecture**: New skill category registered but category rationale not documented
  **Location**: Phase 1.4
  Add a one-line note explaining why `visualisation/` is its own
  category vs co-located under an existing one.

- 🔵 **Code Quality**: Four-levels-up bootstrap is called out but unused in Phase 1 code
  **Location**: Current State Analysis, Key Discoveries
  No Phase 1 script derives `PLUGIN_ROOT`. Defer the discussion to
  Phase 2 or explicitly annotate it as "noted for Phase 2".

- 🔵 **Test Coverage**: Test harness duplication between the two new suites (and the existing one)
  **Location**: Phase 1.1 & 1.2 harnesses
  (overlaps with code-quality major). Extract to
  `scripts/test-helpers.sh`.

- 🔵 **Test Coverage**: Executable-bit tests check subjects, not the test scripts themselves
  **Location**: Phase 1.1 Test 1, Phase 1.2 Test 1
  Contributors can forget `chmod +x` on the harness itself. Add a
  `[ -x "${BASH_SOURCE[0]}" ]` self-check or a Phase 1.5 lint.

- 🔵 **Standards**: Extension-less executable name diverges from the all-`.sh` convention
  **Location**: Phase 1.2 `accelerator-visualiser`
  Justified (meant for `$PATH`), but undocumented. Add a one-line
  comment explaining the departure.

- 🔵 **Standards**: `## Status` heading is a novel section name
  **Location**: Phase 1.3 SKILL.md body
  No other SKILL.md uses `## Status`. Note as ephemeral or fold into
  existing section conventions.

- 🔵 **Usability**: Windows users get no signal at invocation time
  **Location**: Spec-level
  Not a Phase 1 defect (Windows is scoped out), but a clear platform
  guard or explicit "What We're NOT Doing" note would reduce
  confusion for Windows users discovering the plugin.

- 🔵 **Portability**: Prefer the pure-bash `cd … && pwd -P` idiom as the single portability strategy
  **Location**: Phase 1.2 `resolve_self`
  The existing `scripts/config-*.sh` pattern already uses the
  canonical pure-bash idiom. Adopting it here removes the probe, the
  perl dependency, and the untested branch simultaneously.

### Strengths

- ✅ Two entry points (slash command and CLI wrapper) converge on a
  single `launch-server.sh` — clean shared-implementation seam.
- ✅ Forward-compatibility for Phase 2 is built into the stub's
  signature (Test 5 asserts `--foo bar` still exits 0).
- ✅ Directory layout (`skills/visualisation/visualise/{SKILL.md,scripts/,cli/}`)
  matches existing category/skill nesting.
- ✅ `exec` semantics in the CLI wrapper avoid inserting an extra
  process layer between user shell and server process.
- ✅ TDD red-green sequencing per sub-phase; wiring into
  `tasks/test.py` means CI enforcement is in scope.
- ✅ Stub `launch-server.sh` is appropriately minimal (no premature
  JSON schema or argument parsing).
- ✅ "What We're NOT Doing" is exhaustive and deferrals are tagged to
  specific later phases.
- ✅ Preamble faithfully matches canonical ordering from
  `skills/config/init/SKILL.md`.
- ✅ British spellings (`visualisation`, `visualiser`, `visualise`)
  align with existing tone.
- ✅ Manifest entry follows `./skills/{category}/` convention.
- ✅ Symlink test explicitly exercises `BASH_SOURCE` resolution
  through an indirection.
- ✅ Platform scope is consistent with the spec's stated macOS/Linux
  targeting; plan does not silently expand.
- ✅ Portable command choices: `mktemp -d` without template, `wc -l
  | tr -d ' '`, `printf '%q'` inside bash-specific scripts.
- ✅ `allowed-tools` whitelist minimally and correctly scoped.

### Recommended Changes

1. **Update `scripts/test-config.sh` in Phase 1.3 (or a new Phase 1.6)** (addresses: Critical Correctness finding)
   - Change the `"13"` literals to `"14"` at lines 1025, 2859, 2863.
   - Append `"visualisation/visualise"` to the `CONTEXT_SKILLS` array
     (line 1032) and the `ALL_SKILLS` array (line 2866).
   - Preferred: generalise the counts to derive from the manifest so
     future skill additions don't require literal bumps.

2. **Replace `resolve_self` with a pure-bash symlink-walk idiom**
   (addresses: Portability `perl` fallback, Portability `readlink -f /`
   probe, Portability CI coverage gap, Code Quality opaque probe,
   Correctness perl-availability, Test Coverage untested fallback)
   - Use `while [ -L "$p" ]; do ... done` with `readlink` (no `-f`),
     `cd`, `pwd -P`, `dirname`, `basename`. All POSIX; works identically
     on BSD and GNU; no runtime probe; no `perl`.
   - Remove the `resolve_self` function entirely in favour of an
     inline block matching the canonical `scripts/config-*.sh` pattern.

3. **Extract `assert_eq` / `assert_exit_code` into a shared helper**
   (addresses: Code Quality major, Architecture minor, Test Coverage
   suggestion)
   - Create `scripts/test-helpers.sh` containing the helpers plus a
     new `assert_file_executable` and an `assert_stderr_empty`.
   - Have all four harnesses (`test-config.sh`, `test-adr-scripts.sh`,
     `test-launch-server.sh`, `test-cli-wrapper.sh`) source it.
   - Capture stderr in `assert_exit_code` and print on failure.

4. **Make the placeholder unambiguously non-functional**
   (addresses: Usability major, Architecture major)
   - Change stub output to a clearly-fake scheme like
     `placeholder://phase-1-scaffold-not-yet-running`, or relabel the
     SKILL.md line to `**Visualiser URL (not yet running)**:`.
   - Add a forward-compat note in "Migration Notes" that the
     `launch-server.sh` stdout shape (and the SKILL.md line consuming
     it) will change in Phase 2.

5. **Decide the fate of the 11-path preamble**
   (addresses: Architecture minor, Correctness minor, Usability major)
   - Either defer the path resolutions to Phase 2 when they are
     load-bearing, OR keep them with an explicit "forward-compat
     scaffolding; unused in Phase 1" annotation in the plan and in the
     SKILL body.

6. **Automate red-first TDD and test-suite enrolment**
   (addresses: Test Coverage majors)
   - Commit tests separately from implementation so `git log --reverse`
     demonstrates the red/green transition.
   - In Phase 1.5, switch `tasks/test.py` to a glob-based discovery
     (`**/test-*.sh`) or add a meta-test that fails if any `test-*.sh`
     is not referenced in the runner.

7. **Discoverability of the CLI wrapper** (addresses: Usability minor)
   - Add a "How to use as a terminal command" block to SKILL.md's
     status section, or an entry in Phase 12's README-update scope
     cross-referenced from this plan.

8. **Small polish items** (addresses: remaining minors / suggestions)
   - Align `argument-hint` with `skills/config/init/SKILL.md`.
   - Stub-accurate `description` field (defer the aspirational copy
     to Phase 2).
   - One-line explanation of the `cli/` convention in Key Discoveries.
   - Drop "alphabetically" claim in Phase 1.4 prose or explicitly
     document the ordering rule.
   - Add `chmod +x` as an explicit checklist item per sub-phase.
   - Remove or defer the four-levels-up bootstrap discussion from
     Phase 1 Key Discoveries.

---
*Review generated by /review-plan*

## Per-Lens Results

### Architecture

**Summary**: Phase 1 establishes a clean, minimal architectural seam:
a single `launch-server.sh` stub converged on by two entry points
(slash command and CLI wrapper) with forward-compatible signature
(ignored extra args) and TDD-first harnesses that mirror the
established `test-adr-scripts.sh` pattern. Module boundaries match
existing skill categories, the `cli/` vs `scripts/` split inside the
skill keeps the user-facing wrapper separate from internal scripts,
and forward-compatibility for Phase 2+ is deliberately scoped. The
main architectural concerns are (1) the plain-text stdout contract
between `launch-server.sh` and its two callers will need to evolve
in Phase 2 and that boundary is not abstracted anywhere, (2)
test-harness boilerplate is now being copied a third time with no
shared library, and (3) the SKILL.md resolves all 11 path keys
despite the Phase 1 stub using none — creating an unused-for-now
dependency surface that the plan explicitly justifies but should be
revisited if the real URL shape changes.

**Strengths**:
- Two entry points converge on a single `launch-server.sh` — a
  shared-implementation seam keeping behaviour identical.
- Forward-compatibility for Phase 2 is built into the stub signature
  (Test 5 asserts `--foo bar` still exits 0).
- Directory layout matches `skills/vcs/commit/`, `skills/research/research-codebase/`
  category/skill nesting.
- CLI wrapper uses `exec` — no extra process layer between user
  shell and server.
- `allowed-tools` whitelist minimal and correctly scoped.
- TDD-first sequencing plus `tasks/test.py` wiring brings scaffolding
  under CI discipline from day one.

**Findings**:
- 🟡 **major**: Plain-text URL contract between stub and SKILL.md has
  no abstraction layer (Phase 1.3)
- 🔵 **minor**: `assert_eq` / `assert_exit_code` duplicated across
  three test suites with no shared library (Phase 1.1 & 1.2)
- 🔵 **minor**: SKILL.md resolves 11 path keys but Phase 1 stub
  consumes none (Phase 1.3)
- 🔵 **minor**: CLI wrapper's plugin-root resolution is not tested
  end-to-end (Phase 1.2)
- 🔵 **suggestion**: New skill category registered but no rationale
  documented (Phase 1.4)

### Code Quality

**Summary**: The plan is appropriately scoped for Phase 1 scaffolding
— stubs are minimal and the TDD discipline is clear — but it
copy-pastes the `assert_eq` / `assert_exit_code` / PASS-FAIL
bookkeeping for a third and fourth time, cementing a DRY problem that
was already visible in the existing two harnesses. The CLI wrapper's
`resolve_self` has a cryptic `readlink -f /` capability probe that
hurts readability, and the integration-test wiring adds a separate
`context.run` per harness instead of treating them as a discoverable
set.

**Strengths**:
- Stub `launch-server.sh` is a two-line echo — appropriately minimal.
- Test harnesses colocated with the code they test.
- TDD red-green discipline explicit per sub-phase.
- "What We're NOT Doing" section is exhaustive with phase tags.
- Symlink resolution test directly covers `resolve_self`'s reason to
  exist.
- Success criteria are mostly executable bash one-liners.

**Findings**:
- 🟡 **major**: Third and fourth copy of `assert_eq` / `assert_exit_code`
  / PASS-FAIL bookkeeping (Phase 1.1 & 1.2)
- 🔵 **minor**: `readlink -f /` capability probe is clever but opaque
  (Phase 1.2)
- 🔵 **minor**: Executable-bit test duplicates PASS/FAIL bookkeeping
  inline (Phase 1.1 & 1.2)
- 🔵 **minor**: `assert_exit_code` silences stderr/stdout (Phase 1.1
  & 1.2)
- 🔵 **minor**: Per-suite `context.run` invocations will scale poorly
  (Phase 1.5)
- 🔵 **minor**: `mktemp -d` and trap created for tests that don't
  use them (Phase 1.2)
- 🔵 **minor**: Executable-bit discipline relies on human prose, not
  automation (Phase 1.1 & 1.2)
- 🔵 **suggestion**: Four-levels-up bootstrap is called out but unused
  in Phase 1 code (Key Discoveries)

### Test Coverage

**Summary**: The plan follows the existing bash test-harness
convention faithfully and coverage is proportional to the trivial
complexity of Phase 1 stubs. However, the TDD discipline the plan
bills as central is enforced only by honour system, the `test.py`
wiring is a manually-curated list with no meta-check that new suites
get registered, and a handful of obvious behavioural assertions are
missing — particularly stderr silence, wrapper delegation (vs.
reimplementation), and exercise of the cross-platform readlink
fallback.

**Strengths**:
- Test-first sequencing explicit per sub-phase.
- Coverage matched to the stub's behavioural surface.
- Symlink resolution is specifically tested.
- New suites wired into `tasks/test.py` in scope.
- Harness mirrors `test-adr-scripts.sh` idiom for consistency.

**Findings**:
- 🟡 **major**: No meta-enforcement that new test suites get
  registered (Phase 1.5)
- 🟡 **major**: Red-first TDD discipline is verified by honour
  system only (Phase 1.1 & 1.2 Manual Verification)
- 🔵 **minor**: Parity test does not prove delegation (Phase 1.2 Test 3)
- 🔵 **minor**: No assertion that stderr is silent (Phase 1.1 & 1.2)
- 🔵 **minor**: Cross-platform readlink fallback (perl branch) is
  never exercised (Phase 1.2)
- 🔵 **minor**: Argument forwarding through the wrapper is untested
  (Phase 1.2)
- 🔵 **minor**: No assertion that the URL line lacks leading/trailing
  whitespace or BOM (Phase 1.1 Test 3)
- 🔵 **suggestion**: Test harness duplication between the two new
  suites (Phase 1.1 & 1.2)
- 🔵 **suggestion**: Executable-bit tests only check subject scripts,
  not the test scripts themselves (Phase 1.1 & 1.2 Test 1)

### Correctness

**Summary**: The plan is generally logically sound at the
shell-script level — the readlink-vs-perl probe, `BASH_SOURCE`
handling, `wc -l` assertion, `allowed-tools` glob, and SKILL.md
bold-label format are all consistent with existing patterns. However,
the plan has a critical blind spot: adding a new SKILL.md that invokes
`config-read-context.sh`, `config-read-skill-context.sh`, and
`config-read-skill-instructions.sh` will break three hard-coded
`==13 skills` invariant assertions in the existing
`scripts/test-config.sh` suite, making the plan's own
`mise run test:integration` Success Criterion unsatisfiable as written.

**Strengths**:
- BSD-vs-GNU `readlink` probe is logically correct.
- `BASH_SOURCE[0]` is correctly populated for `bash path/to/wrapper`.
- `echo | wc -l` correctly returns `"1"`.
- `allowed-tools` glob shape is proven to work.
- Bold-label placeholder format matches canonical.
- Symlink test exercises the right scenario.

**Findings**:
- 🔴 **critical**: Adding the visualise SKILL.md breaks three
  hard-coded count invariants in `scripts/test-config.sh`
  (Phase 1.3 + 1.5)
- 🔵 **minor**: Prose claims 'alphabetical' ordering but the
  manifest array is not alphabetically ordered (Phase 1.4)
- 🔵 **minor**: Resolving all 11 paths when none are consumed
  introduces failure surface (Phase 1.3)
- 🔵 **minor**: Perl fallback assumes perl is on PATH in every
  supported environment (Phase 1.2)

### Standards

**Summary**: The plan generally adheres to established Accelerator
conventions — canonical SKILL.md preamble, ordered path-resolution
block, `!`-prefixed bash, bold-label placeholders, comma-separated
`allowed-tools`, and the `./skills/{category}/` plugin-manifest shape
are all followed. A few small standards divergences warrant attention:
the `argument-hint` text diverges slightly from the one existing
precedent for no-argument skills; the introduction of a `cli/`
subdirectory under a skill is a new layout convention worth
documenting; and the `description` field over-promises Phase 1
behaviour.

**Strengths**:
- SKILL.md preamble faithfully matches canonical ordering.
- `allowed-tools` shape mirrors existing skills that invoke both
  shared and skill-local scripts.
- Correctly omits `config-read-agents.sh` (no sub-agents spawned).
- Script naming follows kebab-case and `test-*.sh` conventions.
- British-English spelling consistent with repo tone.
- Plugin manifest entry format matches existing pattern.
- Slash-command namespace follows `/accelerator:{skill}` convention.
- All 11 path keys resolved in canonical order.

**Findings**:
- 🔵 **minor**: `argument-hint` diverges from the only existing
  no-argument precedent (Phase 1.3)
- 🔵 **minor**: Description over-promises Phase 1 behaviour
  (Phase 1.3 frontmatter)
- 🔵 **minor**: `cli/` subdirectory is a new skill-layout convention
  (Phase 1.2)
- 🔵 **minor**: Manifest insertion point is not explicitly justified
  against a stated ordering rule (Phase 1.4)
- 🔵 **suggestion**: Extension-less executable name diverges from the
  all-`.sh` convention (Phase 1.2)
- 🔵 **suggestion**: `## Status` heading is a novel section name
  (Phase 1.3)

### Usability

**Summary**: Phase 1 is a scaffolding milestone and the mechanics it
establishes (TDD harness, canonical preamble, plugin-manifest
registration) are solid from a developer-ergonomics standpoint.
However, the user-facing surfaces it exposes — a slash command that
emits a placeholder URL that looks like a real URL, a CLI that users
are expected to symlink onto `$PATH` without any documented path to
discover that fact, and a SKILL.md that renders 11 resolved paths it
doesn't use — create several avoidable DX papercuts.

**Strengths**:
- SKILL.md follows canonical preamble shape for consistent user
  mental model.
- Symlink test validates the real user-facing entry path.
- BSD-vs-GNU `readlink` guard prevents a class of confusing errors.
- `allowed-tools` whitelist avoids permission-prompt surprises.
- TDD discipline ensures DX doesn't silently degrade in future phases.

**Findings**:
- 🟡 **major**: Placeholder `http://localhost:0000` looks like a real
  URL (Phase 1.1 + 1.3)
- 🟡 **major**: Eleven resolved paths rendered to user but never used
  in Phase 1 (Phase 1.3)
- 🔵 **minor**: CLI wrapper's `$PATH` install path is a silent oral
  tradition (Phase 1.2)
- 🔵 **minor**: `argument-hint` misses an opportunity to signal state
  (Phase 1.3)
- 🔵 **minor**: Status block speaks to Claude about phases, not to the
  end user (Phase 1.3)
- 🔵 **minor**: No guidance for the executable-bit-missing failure
  mode (Phase 1.1 / 1.2)
- 🔵 **suggestion**: Windows users get no signal at invocation time
  (spec-level)

### Portability

**Summary**: The plan engages meaningfully with macOS/Linux bash
portability — the shebang, `set -euo pipefail`, `mktemp -d`, and
`wc -l | tr -d ' '` idioms are all correct, and the platform scope
(macOS + Linux only, no Windows) is consistent with prior spec and
research commitments. However, the `resolve_self` helper in the CLI
wrapper introduces two concrete portability risks: the `readlink -f /`
capability probe has ambiguous behaviour across BSD readlink
versions, and the `perl -MCwd=abs_path` fallback assumes `perl` is
present on every supported host, which is not enforced by `mise.toml`
and is not universally true on minimal Linux containers.
Additionally, CI runs Ubuntu-only, so the macOS code paths in
`resolve_self` are never exercised.

**Strengths**:
- Platform scope consistent with prior commitments; no silent scope
  expansion.
- `#!/usr/bin/env bash` + `set -euo pipefail` is a solid portable
  baseline.
- Portable command choices: `mktemp -d`, `wc -l | tr -d ' '`,
  `printf '%q'`.
- Consciously avoids requiring GNU `coreutils` on macOS.
- Symlink test is a good portability check.
- `${CLAUDE_PLUGIN_ROOT}` follows established plugin convention.

**Findings**:
- 🟡 **major**: `perl` fallback is not guaranteed on supported hosts
  and isn't enforced by `mise.toml` (Phase 1.2)
- 🟡 **major**: `readlink -f /` probe is an unreliable indicator of
  GNU-style `readlink` behaviour (Phase 1.2)
- 🟡 **major**: CI runs Linux only; the macOS branch of
  `resolve_self` is never exercised (Phase 1.5 + CI workflow)
- 🔵 **suggestion**: Prefer the pure-bash `cd … && pwd -P` idiom as
  the single portability strategy (Phase 1.2)
- 🔵 **minor**: Symlink-based test can fail on filesystems without
  symlink support (Phase 1.2)

## Re-Review (Pass 2) — 2026-04-18

**Verdict:** COMMENT

Plan is acceptable — the pass-1 critical and the large majority of
majors/minors are resolved. The re-review surfaces one new major
(a test-design bug that can corrupt the working tree) and a spread
of minor polish items. None of them block implementation, but the
sentinel-swap trap issue is worth fixing before the plan is executed.

### Previously Identified Issues

**Critical**
- 🔴 **Correctness**: Hard-coded `"13"` count invariants in `test-config.sh` — **Resolved**. Phase 1.4 step 2 bumps literals to `"14"` and appends `"visualisation/visualise"` to both arrays in the same commit as the SKILL.md creation. Line references (1025, 2859, 2863; 1032; 2866) independently verified against the current file.

**Major (pass 1)**
- 🟡 **Architecture**: Plain-text URL contract between stub and SKILL.md — **Resolved (as flagged tradeoff)**. Migration Notes now explicitly lists the `launch-server.sh` stdout shape as a known Phase 2 churn point.
- 🟡 **Code Quality**: Third/fourth copy of `assert_*` helpers — **Resolved**. New Phase 1.1 extracts `scripts/test-helpers.sh` and migrates both existing harnesses.
- 🟡 **Test Coverage**: No meta-enforcement of test-suite registration — **Resolved**. Phase 1.6 replaces the hand-curated list with glob-discovery filtered by executable bit; raises if zero suites are discovered.
- 🟡 **Test Coverage**: Red-first TDD on honour system — **Partially resolved**. Plan now offers separate-`jj`-commit OR mutation-smoke-test; path (a) is auditable post-hoc, path (b) is transient.
- 🟡 **Usability**: Placeholder `http://localhost:0000` looks like a real URL — **Resolved**. Stub emits `placeholder://phase-1-scaffold-not-yet-running`; SKILL.md label is now `**Visualiser URL (not yet running)**`.
- 🟡 **Usability**: Eleven resolved paths rendered but unused — **Partially resolved**. User confirmed intentional forward-compat; annotations added in Key Discoveries, SKILL.md HTML comment, and "Notes on the preamble". However, all three annotations are maintainer-facing — the first-time user still sees 11 unused paths with no in-body cue.
- 🟡 **Portability**: `perl` fallback not guaranteed — **Resolved**. Entire `perl` branch deleted.
- 🟡 **Portability**: `readlink -f /` probe unreliable — **Resolved**. No capability probe. Pure-bash `readlink` (no `-f`) + `cd … && pwd -P` has identical semantics on BSD and GNU.
- 🟡 **Portability**: CI runs Linux only; macOS branch never exercised — **Changed shape**. The macOS-specific branch no longer exists (risk neutralised at the code level), but `.github/workflows/main.yml` remains Ubuntu-only so the wrapper is still unexercised on macOS.

**Minor/Suggestion (pass 1)** — summary
- Architecture (3 minors, 1 suggestion): all resolved or acknowledged; one residual (SKILL_ROOT untested across tree relocation) carried as a new minor below.
- Code Quality (6 minors, 1 suggestion): all resolved.
- Test Coverage (5 minors, 2 suggestions): 4 resolved, 2 partially resolved (byte-exact URL check, test-script self exec-bit check), 1 changed shape (perl branch no longer applicable).
- Correctness (3 minors): all resolved.
- Standards (4 minors, 2 suggestions): 4 resolved; 2 partially resolved (`cli/` directory convention still not documented; `## Status` heading remains novel).
- Usability (4 minors, 1 suggestion): 3 resolved; 2 partially resolved (Status block still mixes phase jargon; exec-bit failure mode unchanged).
- Portability (1 suggestion, 1 minor): both resolved.

### New Issues Introduced

**Major**
- 🟡 **Correctness**: Sentinel-swap test pattern is not failure-safe — Phase 1.3 `test-cli-wrapper.sh` delegation and argument-forwarding tests `mv` the real `launch-server.sh` aside and restore it at the end of each test. Under `set -euo pipefail`, any intermediate failure (e.g. wrapper regression, assertion mismatch) aborts the harness before the restore runs, leaving the real file replaced by the throwaway fixture. A subsequent `test-launch-server.sh` run would then observe the corrupted tree. Fix: wrap each swap in a `trap 'mv -f "$BACKUP" "$LAUNCH_SERVER" 2>/dev/null || true' EXIT` (or redirect the wrapper at a tempdir copy via an env var rather than mutating the real file).

**Minor**
- 🔵 **Architecture**: `SKILL_ROOT` derivation still untested across tree relocation — the `cli/ ↔ scripts/` sibling contract is asserted only by the happy path; a Phase 2 move of `scripts/` would silently break the wrapper while existing tests still pass.
- 🔵 **Correctness / Portability**: Pure-bash symlink-walk loop has no cycle detection — `ln -sf a b; ln -sf b a` would hang the wrapper. Add a 40-hop depth limit mirroring `SYMLOOP_MAX` in BSD/GNU `readlink -f`.
- 🔵 **Correctness / Portability**: `os.access(..., os.X_OK)` filter in Phase 1.6 is dual-use and fragile — it intentionally excludes `test-helpers.sh` via the non-exec bit, but also silently skips any new harness a contributor forgets to `chmod +x`. WSL-on-NTFS mounts report every file as executable, which would flip the exclusion on that filesystem. Suggest belt-and-braces: either rename helpers to not match the `test-*.sh` pattern, or add a name-level exclusion.
- 🔵 **Standards**: Phase jargon has leaked into user-facing `description` and `argument-hint` frontmatter fields (`"Phase 1 scaffold entry point for…"`, `"(no arguments — Phase 1 scaffold; no server yet)"`). The only prior no-argument precedent (`config/init`) uses a timeless qualifier (`"safe to run repeatedly"`); other descriptions are verb-first imperative. These fields render in Claude Code's slash-command palette.
- 🔵 **Usability**: The new `ln -s "${CLAUDE_PLUGIN_ROOT}/…"` install tip in the Status block uses a variable that is only set inside Claude Code's skill environment. A user copy-pasting into their shell gets an unexpanded empty string and creates a dangling symlink at `/skills/visualisation/…`. Pre-resolve via `!`-prefixed bash so the rendered SKILL.md shows an absolute path, or call out the environment explicitly.
- 🔵 **Usability**: Status block's factual paragraph retains phase numbers that Claude may relay verbatim to the user; the relay instruction doesn't forbid phase-number mentions.
- 🔵 **Usability**: Executable-bit-missing failure mode unchanged — the SKILL.md still invokes the stub directly rather than via `bash "$SCRIPT"`, so a lost exec bit produces `Permission denied` in-line instead of a diagnostic.
- 🔵 **Code Quality**: Phase 1.1's `PLUGIN_ROOT=…$(…additional ../ as needed…)…` migration snippet is written as pseudocode rather than the two concrete literals (`"$SCRIPT_DIR/.."` and `"$SCRIPT_DIR/../../.."`). Easy to get wrong at implementation time.
- 🔵 **Test Coverage**: The delegation test forward-compat check covers `--foo bar` but not arguments containing spaces or empty strings — the quoting regressions most likely to bite Phase 2 (`exec "$X" $@` vs `exec "$X" "$@"`) still aren't exercised.

**Suggestion**
- 🔵 **Architecture**: Hand-counted `../` depths in each harness create a per-harness contract; worth documenting the depth convention or providing an up-tree walk helper before more harnesses accumulate.
- 🔵 **Standards**: `## Status` heading remains a novel section name (no other SKILL.md uses it); consider renaming to `## Availability` / `## Important Notes` or explicitly flagging as transitional in Migration Notes.
- 🔵 **Standards**: `cli/` directory is a new skill-layout convention but not documented as a rule (vs. `scripts/`). Add a one-sentence note in Key Discoveries.
- 🔵 **Usability**: `~/.local/bin` is assumed on `$PATH` — not true by default on macOS. Add a one-line caveat after the `ln -s`.
- 🔵 **Test Coverage**: Glob auto-enrolment invariant is verified once manually; consider a standing meta-test that compares discovered suites against a `git ls-files` grep.
- 🔵 **Test Coverage**: Add one byte-exact URL assertion (`od` or `printf '...\n'` with explicit sentinel terminator) to guard against BOM/CR contamination of the stub's stdout.

### Assessment

The plan is in substantially better shape. One critical, nine
majors, and the majority of minors from pass 1 are now resolved or
satisfactorily acknowledged. The remaining items are either
small-scope polish (phase jargon in palette fields, `${CLAUDE_PLUGIN_ROOT}`
in user-copyable commands) or higher-order concerns the plan now
names openly (CI platform coverage, hand-counted `../` depths).

One new major finding is worth addressing before implementation:
the sentinel-swap pattern in Phase 1.3's delegation and
argument-forwarding tests can corrupt the working tree on a failed
run and cascade into confusing failures in unrelated suites. A
single `trap` on the swap-and-restore — or, better, redirecting the
wrapper at a tempdir copy via an env var — would close this cleanly
and is consistent with the test-harness hygiene improvements
Phase 1.1 already lands.

Subject to that fix (and optional polish on the user-facing
`description` / `argument-hint` phase jargon), the plan is ready to
implement.

## Re-Review (Pass 3) — 2026-04-18

**Verdict:** COMMENT

Pass-3 edits successfully resolve all 9 pass-2 follow-up items
(1 major + 8 minors). The tempdir-copy test restructure closes the
sentinel-swap corruption risk and the SKILL_ROOT tree-relocation
gap simultaneously; the 40-hop cycle counter, pure-bash literals in
the migration snippets, `bash`-prefixed stub invocation,
`${CLAUDE_PLUGIN_ROOT}` pre-resolution, phase-jargon removal,
`EXCLUDED_HELPER_NAMES` belt-and-braces, and `## Availability`
rename all landed as intended.

However, one new major finding surfaced — flagged independently by
**three lenses** (correctness, test-coverage, portability) — that
would break the plan's own Success Criterion at implementation
time.

### Previously Identified Issues (from Pass 2)

- 🟡 **Correctness (pass 2 major)**: Sentinel-swap test unsafe — **Resolved**. Tempdir-copy pattern confines all mutation to `$TMPDIR_BASE/skill-copy/…`. The new Success Criterion ("real `launch-server.sh` is byte-identical after any test run") locks the invariant in.
- 🔵 **Architecture**: SKILL_ROOT untested across tree relocation — **Resolved**. The "wrapper works from a relocated tree" test exercises the `cli/↔scripts/` sibling contract from an arbitrary path.
- 🔵 **Correctness / Portability**: Symlink-walk cycle detection missing — **Counter added but its test is broken** (see new major below). The 40-hop guard itself is correct.
- 🔵 **Correctness / Portability**: `os.X_OK` filter fragile on non-POSIX FS — **Resolved**. `EXCLUDED_HELPER_NAMES` provides belt-and-braces.
- 🔵 **Standards**: Phase jargon in `description` and `argument-hint` — **Resolved**. Both fields now verb-first and timeless.
- 🔵 **Usability**: `${CLAUDE_PLUGIN_ROOT}` in copy-paste command — **Resolved**. Pre-resolved via `!`printf``.
- 🔵 **Usability**: Status block mixed jargon — **Resolved**. Phase context confined to HTML comment; relay instruction forbids phase references.
- 🔵 **Usability**: Exec-bit failure mode — **Resolved**. SKILL.md invokes stub via `bash` prefix.
- 🔵 **Code Quality**: PLUGIN_ROOT pseudocode — **Resolved**. Two concrete literal blocks.
- 🔵 **Test Coverage**: Argument-forwarding edge cases — **Resolved**. Now includes spaces and empty strings.

### New Issues Introduced

**Major (1, cross-cutting — flagged by correctness, test-coverage, and portability lenses independently)**

- 🟡 **Symlink-cycle test exercises kernel ELOOP, not the wrapper's 40-hop counter** — The new "wrapper aborts on symlink cycle" test creates `CYCLE_A → CYCLE_B → CYCLE_A` (mutually-linked paths that contain no executable) and asserts `assert_exit_code … 1 "$CYCLE_A"`. But `execve()` on a symlink cycle returns `ELOOP` at the kernel level before any bash process starts; macOS and Linux shells both surface this as exit **126** (or sometimes 127), not 1. The wrapper's `while [ -L "$SELF" ]` loop never runs, the 40-hop counter is never executed, **and the test will always fail under `mise run test:integration`** — blocking the plan's own Success Criterion for Phase 1.3.

  **Fix options**:
  - (a) Remove the test; the 40-hop guard is defence-in-depth and unit-testing it reliably requires invoking the wrapper via `bash "$CYCLE_A"` so the `BASH_SOURCE[0]`-driven walk runs.
  - (b) Invoke via `bash "$CYCLE_A"` instead of directly; bash reads the symlinked script, the wrapper's own loop triggers, and exit code 1 plus a `symlink loop detected` stderr match proves the counter fired.
  - (c) Relax to any non-zero exit (`! "$CYCLE_A"`) — cheap but weaker proof of the code path.

  Option (b) is the most faithful test. If time is tight, option (a) is safer than shipping a test that doesn't do what it claims.

**Minor — New**

- 🔵 **Code Quality**: `TEMP_STUB` is overwritten by the delegation then argv-printer tests in sequence. Any future test placed after these that depends on the original stub contents would observe the wrong fixture. Add an inline comment flagging the ordering, or reset `TEMP_STUB` from `REAL_STUB` before each mutation block.
- 🔵 **Code Quality**: Magic number `40` duplicated between the conditional and the diagnostic. Extract to `readonly MAX_SYMLINK_HOPS=40`.
- 🔵 **Code Quality**: Adjacent heredocs use different quoting (`<<EOF` for `$SENTINEL` expansion vs `<<'EOF'` for `$@` preservation) — copy-paste hazard. One-line comment above each would suffice.
- 🔵 **Code Quality**: Argument-forwarding assertion relies on `$(...)` stripping one trailing newline on both sides — works today but fragile. Inline comment or separator-joined comparison would document the contract.
- 🔵 **Code Quality**: `bash`-prefixed SKILL.md invocation may or may not match the `allowed-tools` glob `Bash(${CLAUDE_PLUGIN_ROOT}/skills/visualisation/visualise/scripts/*)` — worth verifying in manual testing, potentially broadening the glob.
- 🔵 **Test Coverage**: Real wrapper's delegation is now proven only by parity, not by sentinel (tempdir-copy holds the sentinel path). Optional `cmp -s "$REAL_CLI" "$TEMP_CLI"` would close the inferential loop.
- 🔵 **Correctness**: `EXCLUDED_HELPER_NAMES` matches basename only — a future skill's own `test-helpers.sh` at a different path would also be silently excluded. Consider path-qualified exclusion or an inline comment stating the convention.
- 🔵 **Standards**: `## Availability` is still a novel heading — the existing 4-skill precedent uses `## Important Notes`. Either rename to match or note as a new deliberate convention in Migration Notes.
- 🔵 **Standards**: Two in-body HTML comments are a new SKILL.md pattern with no precedent. Consider routing Claude-only context through the existing `config-read-skill-instructions.sh visualise` mechanism.
- 🔵 **Standards**: `!`bash …/launch-server.sh`` and `!`printf …`` both diverge from the shared-script convention for `!`-targets. Consider extracting the install-command rendering into `scripts/visualiser-install-command.sh` or document the departure.
- 🔵 **Standards**: `cli/` subdirectory convention still undocumented (carried over from pass 1 and pass 2).
- 🔵 **Usability**: Install command renders inline after `**Install command**:` — harder to copy cleanly than a fenced code block. Consider wrapping in a `bash` code fence if `!`-expansion works inside one.
- 🔵 **Usability**: The body under `## Availability` opens with `Tell the user, …` — it's an imperative to Claude, not user copy. If any rendering path surfaces the SKILL body verbatim, the user sees meta-instruction text. Consider writing natural user-facing prose and moving the "no phase numbers" constraint into the HTML comment.
- 🔵 **Usability**: Troubleshooting note says "add … to your shell rc" — jargon that doesn't tell the user which file. Name `~/.zshrc` (macOS default) and `~/.bashrc` (typical Linux) explicitly.
- 🔵 **Portability**: 40 matches Linux's `SYMLOOP_MAX` but is slightly over Darwin's (32). Either lower to 32 or tighten the comment to state the over-approximation.

**Suggestion — Carried over, still open**

- 🔵 **Test Coverage**: Exact-byte URL assertion (BOM/CRLF guard) still not added.
- 🔵 **Test Coverage**: Harness self-exec-bit check not added.
- 🔵 **Test Coverage**: Standing meta-test for glob-discovery invariant not added.
- 🔵 **Architecture**: Hand-counted `../` depths across harnesses — worth factoring into an up-tree walk when the fifth harness lands.
- 🔵 **Architecture**: Skill-category rationale for top-level `visualisation/` still not documented.
- 🔵 **Portability**: CI remains Ubuntu-only — the bash 3.2 / BSD paths the wrapper targets have no automated coverage.
- 🔵 **Usability**: "also a placeholder today" copy will age; consider wording that doesn't pin to "today".

### Assessment

Nine of nine pass-2 findings are resolved. One new major concern
emerged — the cycle-detection test was constructed in a way that
tests the kernel's exec loader rather than the wrapper's code path,
and it will reliably fail in CI. The counter itself is correctly
implemented; only the test needs to invoke via `bash "$CYCLE_A"`
(so the `BASH_SOURCE`-driven walk runs in the wrapper's bash
process) or be removed.

Everything else on the list is polish — new-pattern standards
concerns (novel section heading, HTML comment pattern, ad-hoc
`!`-targets) and several carried-over suggestions (exact-byte URL
match, harness self-exec-bit, CI platform matrix).

Subject to the cycle-test fix, the plan is ready to implement. If
you want to close more of the remaining minors, the highest-leverage
ones are: renaming `## Availability` → `## Important Notes` to match
precedent, moving the Claude-only `Tell the user, …` imperative into
the HTML comment and rewriting the body as natural user prose, and
fencing the install command in a `bash` code block. Everything else
is optional.
