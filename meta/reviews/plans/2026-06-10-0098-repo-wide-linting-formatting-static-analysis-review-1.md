---
type: plan-review
id: "2026-06-10-0098-repo-wide-linting-formatting-static-analysis-review-1"
title: "Plan Review: Repo-Wide Linting, Formatting, And Static Analysis Guardrails Implementation Plan"
date: "2026-06-10T20:08:49+00:00"
author: Toby Clemson
producer: review-plan
status: complete
target: "plan:2026-06-10-0098-repo-wide-linting-formatting-static-analysis"
reviewer: Toby Clemson
verdict: "APPROVE"
lenses: [architecture, code-quality, test-coverage, correctness, standards, usability, safety, compatibility]
review_number: 1
review_pass: 4
tags: []
last_updated: "2026-06-11T00:24:02+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Repo-Wide Linting, Formatting, And Static Analysis Guardrails Implementation Plan

**Verdict:** REVISE

This is a strong, well-grounded plan: it resolves all eight open questions from
the research, verifies its load-bearing technical claims against the actual
codebase (the tsc no-op, the duplicate `types` key, the embed-dist `build.rs`
panic, the tomlkit round-trip, the clippy priority idiom, the ruff D-rule
ignore set all check out), correctly mirrors the established two-pass clippy
precedent, and faithfully encodes every hard work-item requirement around
strictness, suppression rationale, and `.editorconfig` cross-referencing. The
verdict is REVISE rather than APPROVE because of one **critical** config defect
— the literal `.x` version pins are mise fuzzy/prefix matches (range
operators), directly contradicting the exact-pin AC the plan exists to satisfy
— plus a cluster of majors that are mostly **targeted, one-edit fixes** rather
than structural rework: a multi-phase window where the new CI jobs don't
actually gate the release path, two existing test suites the plan's own edits
will turn red without acknowledgement, and a handful of DX/cross-platform gaps.

### Cross-Cutting Themes

- **CI gates aren't actually blocking during the staged rollout** (flagged by:
  architecture, correctness, safety) — Phases 3–5 each add a `check-<component>`
  job but only Phase 2 edits `prerelease.needs`, and the wiring is deferred to
  Phase 6. Between a component slice landing and Phase 6, the new job runs but
  does not gate the `prerelease`/`release` path, contradicting the "blocking
  the moment it lands" guarantee. Compounded by the `check`→`check-scripts`
  rename potentially desyncing the GitHub branch-protection required-check name.
- **The plan's own edits will break existing tests it doesn't mention** (flagged
  by: test-coverage, code-quality) — Phase 2 widens shell scope (breaking three
  assertions in the existing `tests/unit/tasks/shared/test_sources.py`) and
  drops the shellcheck CLI flags (breaking `test_lint.py::TestShellcheckTask.
  test_command`). Both files are absent from the plan, yet each phase's success
  criterion is `mise run test:unit:tasks` exits 0 — unachievable as written.
- **The `default` task rewire is under-specified** (flagged by: architecture,
  correctness, standards, usability) — Phase 6 rewrites `format:fix`/`lint:check`
  but only conditionally mentions `default` (`mise.toml:220-222`). The result is
  ambiguous: `mise run default` would either silently skip the new `types:*`
  checks (so "passes locally, fails in CI") or balloon into a whole-repo in-place
  reformat plus a double Rust compile. Four lenses independently flagged it.
- **Pin reproducibility and cross-platform determinism** (flagged by:
  compatibility) — the `.x` pins, the unspecified pyrefly backend
  (`pipi:` vs `cargo:`), and ubuntu-only lint jobs against an ubuntu+macOS dev
  base together leave room for "green in CI, red on a developer's Mac" drift —
  the exact failure mode the exact-pin mandate exists to prevent.
- **Task-tree consistency and developer ergonomics** (flagged by: standards,
  usability) — the `check:<component>` naming, the absence of a single
  "fix everything" command, non-actionable error messages, and inconsistent
  folding depth across components raise cognitive load for the tree's daily users.

### Tradeoff Analysis

- **Per-component independence vs the embed-dist build requirement**:
  `lint:server:check` must `depends = ["build:frontend"]` because `build.rs`
  panics without `dist/` under `embed-dist`, which cross-couples the Rust lint
  gate to the entire frontend toolchain (node install + Vite build) and gives a
  "server" job a frontend-build failure mode. The coupling is forced, but the
  architecture lens suggests narrowing it: run the default-feature clippy pass
  without `dist/`, or gate on a lightweight `dist/index.html` stub task rather
  than a full production Vite build. Worth an explicit decision in Phase 4.

### Findings

#### Critical

- 🔴 **Compatibility**: Literal `.x` version pins are mise range operators, not exact pins
  **Location**: Phase 3, Section 2 (`mise.toml` — pins)
  The snippet pins `ruff = "0.15.x"` and `pyrefly = "1.0.x"`. In mise, `.x` (and
  a bare `0.15` prefix) is a fuzzy match resolving to the latest patch at install
  time — a range operator. With `select = ["ALL"]`, a new ruff patch can add or
  change rules, so a lint green in CI today fails for a developer tomorrow with
  no code change. The Phase 3 grep ("pinned exact, no range operators") won't
  catch `.x` because the plan never specifies a regex that rejects it. Commit the
  full triple (e.g. `ruff = "0.15.7"`) and tighten the check to reject `.x`.

#### Major

- 🟡 **Architecture / Correctness / Safety** (merged): New per-component CI jobs aren't added to `prerelease.needs` until Phase 6
  **Location**: Phases 3–5 (CI job additions) vs Phase 6, Section 4 (CI tidy)
  Only Phase 2 edits `prerelease.needs` (`check` → `check-scripts`). Phases 3–5
  add `check-python`/`check-server`/`check-frontend` without adding each to the
  `needs:` list; Phase 6 merely "confirms" it. In the multi-PR interim a failing
  new lint/type job would still let the push-triggered `prerelease` (and binary
  attestation) proceed — the "blocking the moment it lands" guarantee isn't
  structurally enforced. Fix: each phase adds its own job to `prerelease.needs`
  in the same PR; Phase 6 only asserts completeness.

- 🟡 **Safety**: Renaming the `check` CI job risks silently dropping the branch-protection merge gate
  **Location**: Phase 2, Section 5 (`check` → `check-scripts`)
  Branch-protection required-status-checks match by job **name** and live in
  GitHub settings (no ruleset file exists in-repo). Renaming `check` to
  `check-scripts` either blocks PRs forever on a check that never reports
  (fail-safe but disruptive) or, if matched loosely, silently stops enforcing
  (fail-open). Make the required-check-name update a mandatory Phase 2 step (not
  the conditional Migration Note) and verify a deliberately-failing run actually
  blocks merge.

- 🟡 **Architecture**: `lint:server:check` cross-couples to the whole frontend toolchain
  **Location**: Phase 4, Section 3 (two-pass, frontend-gated clippy)
  Both clippy tasks `depends = ["build:frontend"]`, so the Rust lint gate
  transitively pulls in node install + Vite build and inherits a frontend-build
  failure mode, blurring the per-component independence the slice decomposition
  relies on. See Tradeoff Analysis for the narrowing options.

- 🟡 **Test Coverage / Code Quality** (merged): Phase 2 scope change breaks the existing `test_sources.py`, unacknowledged
  **Location**: Phase 2, Section 7 (Scope coverage test)
  Phase 2 proposes a "new" scope test, but `tests/unit/tasks/shared/test_sources.py`
  already exists and asserts the OLD behaviour (`test_excludes_fixtures_at_any_depth`,
  `test_excludes_test_helpers`, `test_excludes_fixtures_workspaces_helpers_keeps_normal`).
  Widening scope turns these three red, so the Phase 2 success criterion
  (`mise run test:unit:tasks` exits 0) is unachievable until they're inverted.
  Point the step at the existing file and invert the assertions, rather than
  adding a parallel test.

- 🟡 **Test Coverage**: Dropping the shellcheck CLI flags breaks an existing `test_lint.py` assertion
  **Location**: Phase 2, Section 2 (`tasks/lint/scripts.py`)
  `TestShellcheckTask.test_command` asserts `cmd.startswith("shellcheck -x
  --severity=warning ")`. Changing the command to bare `shellcheck {args}` fails
  it, and the plan lists only the pass gate, not the assertion update. Add an
  explicit step to update the test to assert the new prefix and the absence of
  `-x`/`--severity` (the unit-level proof that flag ownership moved to the rc).

- 🟡 **Test Coverage**: pyrefly (and ruff) coverage cross-check is manual-only, with no automated guard
  **Location**: Phase 3, Success Criteria — Manual Verification
  Unlike shell (a single inspectable `shell_sources()` backed by a regression
  test), pyrefly's `project-excludes`-driven discovery and ruff's `extend-exclude`
  are verified only by a human eyeballing two file lists. A misconfigured exclude
  could leave files silently unchecked while every automated criterion exits 0 —
  the vacuous-pass mode the work-item pass-3 review raised. Add an automated
  coverage test diffing `git ls-files '*.py'` (minus the two justified excludes)
  against the files ruff/pyrefly actually report on.

- 🟡 **Correctness**: Bringing stray files into `tsc -b` likely needs `composite: true` on referenced projects
  **Location**: Phase 5, Section 4 (type-check coverage for stray files)
  In build mode, every referenced project must set `composite: true`; none of
  tsconfig.app/node/e2e.json currently do, and tsconfig.e2e.json lacks a
  `tsBuildInfoFile`. Adding a new referenced project (or expanding
  tsconfig.node.json) will surface composite/emit requirements the plan hasn't
  budgeted for, potentially blocking the `types:frontend:check` criterion. Run
  `tsc -b --noEmit` against the proposed layout during the report-only step and
  record the exact project assignment + composite settings per stray file.

- 🟡 **Standards**: `check:<component>` aggregate naming breaks the otherwise-uniform tree
  **Location**: Phases 2–5 (each `check:<component>`) + Phase 6
  The new families consistently place the component at position 2
  (`format:python:check`), but the roll-up flips to `check:python` (verb-first,
  no `:check`/`:fix` action suffix). There's no existing `check:*` namespace to
  anchor it, so the tree the plan works hard to make uniform gains a fourth
  family with inverted ordering. Either name the roll-ups `<component>:check`,
  or justify the `check:<component>` form in the same "What We're NOT Doing" note
  that defends the family split. (Lower-confidence than the count suggests —
  `check` is already the CI aggregate, so "the check task, scoped to python" is
  defensible; flag it for an explicit decision.)

- 🟡 **Usability**: No single "fix everything before I push" command; the closest one silently omits shell
  **Location**: Phase 6, Section 1 (`format:fix` / `lint:fix`) + Phase 2
  There is no top-level `fix` aggregate, so a developer must run two commands in
  order, and `lint:fix` silently drops scripts (the `# scripts: no autofixers`
  note is invisible in `mise tasks`). The only both-fixer command is `default`,
  which also rebuilds and runs the full test suite. Add a `fix` aggregate
  (`depends = ["format:fix", "lint:fix"]`) whose description names the shell
  autofixer gap.

- 🟡 **Usability**: New invoke wrappers regress to non-actionable error messages
  **Location**: Phase 2/3/4, invoke wrappers
  The wrappers raise bare `Exit("ruff reported findings")` / `"clippy reported
  findings"` / `"shellcheck reported findings"`, whereas the existing
  `format:scripts:check` already models the better pattern (`"...run \`mise run
  format:scripts:fix\`"`). With `--severity` also stripped from shellcheck, a
  developer gets info/style noise and no remediation pointer. Mirror the existing
  shfmt wrapper: name the fix command where a fixer exists, name the manual step
  where it doesn't.

- 🟡 **Compatibility**: pyrefly backend (`pipi:` vs `cargo:`) version-consistency across platforms is unspecified
  **Location**: Phase 3, Section 2 + Key Discoveries (pyrefly install path)
  A `cargo:` source build and a `pipi:` wheel can resolve to different artefacts
  across ubuntu CI and a developer's macOS, so a strict type-check could surface
  different diagnostics by platform — for the one tool with no fallback channel.
  After confirming the channel, assert the same backend + exact version
  everywhere and capture `pyrefly --version` in the report-only checkpoint.

- 🟡 **Compatibility**: New lint jobs are ubuntu-only while tests and developers are ubuntu+macOS
  **Location**: Phases 2–5 CI jobs (`runs-on: ubuntu-latest`)
  Test jobs run an ubuntu+macos matrix and developers work on macOS, but the four
  lint jobs are ubuntu-only. Tool behaviour can differ subtly by platform
  (shfmt/shellcheck locale handling, clippy host-target diagnostics), compounded
  by any pin looseness. Either document that determinism rests entirely on exact
  pins resolving identically on both OSes (and make every tool exact-pinned), or
  add macos to at least one lint job.

#### Minor

- 🔵 **Architecture / Correctness / Standards / Usability** (merged): The `default` task rewire is under-specified
  **Location**: Phase 6, Section 1 + `mise.toml:220-222` (`default`)
  `default` depends on `format:fix` + `lint:check`. After Phase 6, `lint:check`
  no longer transitively covers type-checks, and `format:fix` fans out to all
  four in-place formatters plus a double Rust compile via `lint:check`. The plan
  only conditionally mentions updating `default`. State the exact post-rewire
  dependency list and confirm the intended semantics (likely: `default` →
  `check`, or add `types:check`), so `mise run` neither silently skips the new
  type-checks nor surprises developers with a whole-repo reformat.

- 🔵 **Code Quality**: Two-pass clippy loop fails fast, diverging from the collect-all-failures precedent
  **Location**: Phase 4, Section 3 (`tasks/lint/server.py`)
  The loop raises on the first failing pass, so a developer never sees the
  default-feature diagnostics when the all-features pass fails — lengthening the
  remediation loop. `tasks/test/unit.py`'s `templates` task already models
  collect-then-raise; consider matching it.

- 🔵 **Correctness**: `clippy --fix` run twice across differing feature sets can conflict
  **Location**: Phase 4, Section 3 (`lint:server:fix`)
  `cargo clippy --fix` rewrites source and refuses a dirty tree; running it under
  `--all-features` then default can produce conflicting `cfg`-gated edits or
  error without `--allow-dirty`. Specify the fix task's feature handling
  explicitly (e.g. fix once under default features, then re-check both passes).

- 🔵 **Correctness**: The duplicate `types` key fix doesn't say which key to keep
  **Location**: Phase 5, Section 3 (tsconfig.e2e.json)
  JSON semantics make the second (`["node", "vite/client"]`, line 13) the
  effective value. "Keep one" should specify keeping line 13's value to preserve
  the current type environment (behavioural risk is currently nil, but avoids a
  latent break).

- 🔵 **Standards**: New invoke modules bypass the canonical `tasks.shared.paths` constants
  **Location**: Phase 4, Section 3 (and Phase 3/5 wrappers)
  `tasks/lint/server.py` re-derives the crate path as a string literal and uses
  `context.cd`, whereas `tasks/build.py`/`test/unit.py` import `SERVER`/`CARGO_TOML`
  from `tasks.shared.paths` and pass `--manifest-path`. Reuse the constants; a
  future directory move otherwise silently misses these modules.

- 🔵 **Standards**: ruff/rustfmt cross-reference comments mirror only line-length, not indent
  **Location**: Phase 3, Section 1 + Phase 4, Section 2
  The work item's `.editorconfig`-sync AC names "line-length/indent". The configs
  duplicate+cross-reference only line-length; indent is satisfied only by
  coincidence of tool defaults. Either set+cross-reference indent explicitly or
  note it's intentionally left at the matching default.

- 🔵 **Standards**: AC→task mapping in the PR description isn't a durable in-repo artefact
  **Location**: Phase 6, Section 2
  Because the plan diverges from the work item's flat `lint:<language>` names,
  traceability rests on the AC→task mapping. A PR description evaporates after
  merge; commit it as a comment block in `mise.toml` (the durable form the repo
  already uses for task semantics).

- 🔵 **Usability**: Folding depth is inconsistent across components
  **Location**: Implementation Approach (component→tools→families) + aggregates
  `check:python` folds format+lint+types but `check:scripts`/`check:server` fold
  only format+lint; `lint:scripts:check` is itself a sub-aggregate (shellcheck +
  bashisms) while `lint:python:check` is a leaf. Partly inherent (Rust/shell have
  no type-checker), so the remedy is documenting the folding rule in one place
  (the Phase 6 mapping note).

- 🔵 **Usability**: CI-only enforcement leaves no documented local "run before you push" loop
  **Location**: Migration Notes / What We're NOT Doing (no pre-commit hooks)
  With hooks deliberately excluded, the plan never names the pre-push command.
  Add a one-line onboarding note (e.g. "run `mise run fix` then `mise run check`
  before pushing; CI runs the same `check:<component>` tasks").

- 🔵 **Usability**: Slow `lint:server:check` expectation is set for CI but not the local developer
  **Location**: Phase 4, Section 3 + Performance Considerations
  Two cold clippy compiles + a frontend build land on local runs too; the task
  `description` doesn't set the time expectation. Note the cost in the description.

- 🔵 **Test Coverage**: Falsification probes prove command wiring, not file-set coverage
  **Location**: Testing Strategy / Phases 2–5
  An in-tree probe violation can't detect a tool wired-but-checking-zero-files.
  Note that probes verify wiring/blocking only (breadth is the coverage
  cross-check's job), and place at least one probe in a file that's in-scope only
  because of the scope-widening.

- 🔵 **Test Coverage**: The Cargo.toml guard test under-specifies the comment-preservation assertion
  **Location**: Phase 4, Section 4
  The stub should assert a verbatim comment string survives `_render_cargo_toml`
  (not just the table), since comment survival is the property the justification-
  comment policy depends on. The existing `fake_repo_tree` Cargo.toml has no
  `[lints.clippy]`, so a bespoke fixture is needed.

- 🔵 **Safety**: Empty file-set causes lint tasks to silently exit zero (fail-open)
  **Location**: Phase 2, Section 2 + Implementation Approach
  Shell tasks return early on an empty set, and bare `ruff check`/`biome lint .`
  pass with zero diagnostics if an over-broad exclude empties their input. Only
  shell has a non-emptiness guard. Fail the task (or assert a minimum count) when
  the matched set is empty so a scope collapse fails loudly.

- 🔵 **Safety / Test Coverage** (merged): "Mechanical-only, no behavioural change" rests on implicit tool defaults
  **Location**: Implementation Approach step 3 + Phase 5/6
  The behaviour-preserving property rests on each tool's safe-fix default (ruff
  safe-only, biome `--write` excludes unsafe, clippy machine-applicable), stated
  nowhere as a guard, with only a frontend spot-check. State explicitly that
  `--unsafe`/`--unsafe-fixes` is never passed, keep the mechanical-diff spot-check
  for every component, and add `test:e2e:visualiser` + visual-regression to
  Phase 5's gates (the largest, riskiest reformat).

- 🔵 **Usability / Safety** (merged): Reformat-sweep rebase impact is coordinated privately, not broadcast, with no recovery aid
  **Location**: Migration Notes
  The frontend sweep alone rewrites ~237 files and will conflict with essentially
  every open frontend branch. Broadcast each imminent sweep to the team (not just
  named owners) and recommend re-running `format:fix`/`lint:fix` after rebasing
  rather than hand-resolving reformat conflicts (deterministic regeneration).

#### Suggestions

- 🔵 **Correctness**: `.shellcheckrc` `external-sources=true` is the plan's only self-flagged unverified shell claim
  **Location**: Phase 2, Section 1
  Low risk (shellcheck 0.11.0 honours it from rc), but make the report-only run an
  explicit gate that diffs findings under `-x` vs rc-only before dropping `-x`,
  keeping the per-file `source=`/`disable=` fallback ready (as the plan notes).

- 🔵 **Correctness / Compatibility** (merged): `[*] max_line_length = 80` and the frontend EditorConfig glob have edge gaps
  **Location**: Phase 1, Section 1
  The unscoped `[*]` width means Biome applies 80-col wrapping to committed JSON
  (`package.json`, `tsconfig*.json`) — confirm that's wanted or scope the width to
  source languages. Separately, the frontend glob omits `.mts`/`.cts`, so a future
  such file would format with Biome's tab default; add them to the glob.

- 🔵 **Code Quality**: Ten near-identical wrapper modules duplicate the run/check/raise idiom
  **Location**: Phase 3 §3 + Phase 5 §5
  Idiomatic, but the count is high enough that a drift (a forgotten `warn=True`)
  would be silent. Consider a tiny `run_check(context, cmd, *, on_failure)` helper
  in `tasks/shared/`, or decide deliberately to keep the flat idiom.

- 🔵 **Code Quality**: Several wrappers are specified only as one-line comments, leaving error handling unverified
  **Location**: Phase 3 §3 + Phase 5 §5
  `ruff format --check`, `cargo fmt --check`, `tsc -b --noEmit` all signal via
  non-zero exit that must be caught the same way; state explicitly that every
  check wrapper uses `warn=True` + a descriptive `Exit`.

- 🔵 **Code Quality**: Redundant `_keep` filter on the hardcoded CLI source path
  **Location**: Phase 2, Section 3
  `_keep` can never reject the fixed non-workspace CLI literal; either drop the
  guard with a comment or note it's defensive.

- 🔵 **Standards**: Make the widened `sources.py` module-docstring rewrite explicit
  **Location**: Phase 2, Section 3
  The current docstring names fixtures/test-helpers as excluded; a stale docstring
  would misrepresent the lint scope. Show the revised text (workspaces-only +
  appended CLI script).

- 🔵 **Architecture**: Asymmetric fix-task coverage breaks the format/lint family symmetry
  **Location**: Phase 6, Section 1 (`lint:fix` omits scripts)
  Acceptable (shellcheck has no autofix) but make the asymmetry explicit in a
  Phase 6 comment so a future "fix everything" driver needn't special-case it.

- 🔵 **Compatibility**: Document the mixed action-pinning regimes as a recorded choice
  **Location**: Phase 1, Section 2
  `jdx/mise-action` becomes exact-pinned while `actions/checkout@v5` /
  `attest-build-provenance@v2` stay on floating major tags by design — add a
  one-line note so the inconsistency reads as a decision, not an oversight.

### Strengths

- ✅ The plan resolves all eight research open questions and verifies its
  load-bearing technical claims against the actual codebase (tsc no-op,
  duplicate `types` key, `build.rs` embed-dist panic, tomlkit round-trip, clippy
  priority idiom, ruff D-rule/formatter-conflict ignore lists) — noted across
  correctness, architecture, and standards.
- ✅ The two-pass clippy design is logically complete: `--all-features` and the
  default pass together lint both the `cfg(dev-frontend)` and
  `cfg(not(dev-frontend))` arms, correctly mirroring the established two-pass
  unit-test precedent, with `build:frontend` correctly sequenced first.
- ✅ The Cargo.toml silent-guardrail-loss landmine is defended with an explicit
  characterisation guard test on `_render_cargo_toml` plus a belt-and-braces
  `version:bump` dry-run — exactly the right fail-loud defence (safety, test-coverage).
- ✅ Per-component slice decomposition gives each phase a single cohesive reason
  to change and lets slices land in any order; deferring shared-aggregate
  rewiring to Phase 6 is a deliberate, well-reasoned coupling control (architecture).
- ✅ Every new task carries a `description`, leaf names follow the `:check`/`:fix`
  convention, aggregates are pure `depends` fan-outs, and CI jobs follow the
  one-mise-task-per-job shape — the plan is exceptionally disciplined about
  existing conventions (standards, usability).
- ✅ Tool strictness is faithful to the mandate (`select = ["ALL"]`, `enable=all`,
  pyrefly `strict`, clippy pedantic at warn/priority -1 with `-D warnings`),
  each justified against documented tool behaviour; the bash-3.2 floor (ADR-0016)
  is explicitly preserved as a sibling check (standards, safety, compatibility).
- ✅ rustfmt `edition = "2021"` matches the crate, the Biome `$schema` is coupled
  to the installed package, and the report-only checkpoints give a real sizing
  signal — each phase is independently revertable via VCS (compatibility, safety).

### Recommended Changes

1. **Commit exact version triples and harden the pin check** (addresses: `.x`
   range-operator pins; pyrefly backend consistency). Replace `ruff = "0.15.x"` /
   `pyrefly = "1.0.x"` with resolved full versions; tighten the Phase 3 grep to
   reject `.x`/bare-prefix forms; after picking the pyrefly backend, assert the
   same backend+version on ubuntu and macOS and record `pyrefly --version` in the
   report-only checkpoint.

2. **Wire each CI job into `prerelease.needs` in its own phase, and make the
   branch-protection rename explicit** (addresses: CI jobs not blocking until
   Phase 6; `check` rename desyncing the required-check name). Each of Phases 3–5
   adds its job to `needs:`; Phase 2 makes updating the GitHub required-check name
   a mandatory step with a verify-it-blocks check; Phase 6 only asserts completeness.

3. **Account for the existing tests the plan's edits break** (addresses:
   `test_sources.py`; `test_lint.py`). Point Phase 2 at
   `tests/unit/tasks/shared/test_sources.py` and invert its three old-behaviour
   assertions; add a step to update `TestShellcheckTask.test_command` to the new
   bare-`shellcheck` prefix.

4. **Specify the `default` task rewire** (addresses: under-specified `default`
   across four lenses). State the exact post-Phase-6 dependency list and confirm
   the intended semantics — likely `default` → `check` (or `+ types:check`) — so
   `mise run` neither skips the new type-checks nor triggers a whole-repo reformat.

5. **Add an automated Python coverage guard and confirm the tsc project layout**
   (addresses: pyrefly/ruff manual-only cross-check; `tsc -b` composite). Add a
   test (or `mise` helper) diffing `git ls-files '*.py'` minus the justified
   excludes against the files ruff/pyrefly report on; run `tsc -b --noEmit`
   against the proposed stray-file project layout in the report-only step and
   record the `composite:`/`tsBuildInfoFile` settings each referenced project needs.

6. **Improve the developer-facing surface** (addresses: no fix-everything command;
   non-actionable errors; folding inconsistency; no local pre-push loop). Add a
   top-level `fix` aggregate whose description names the shell gap; give each
   wrapper an actionable `Exit` message (fix command where one exists); document
   the folding rule and the recommended pre-push loop in the Phase 6 mapping note;
   set the `lint:server:check` time expectation in its description.

7. **Confirm the server lint coupling and the cross-platform lint matrix**
   (addresses: server→frontend coupling; ubuntu-only lint jobs). Decide in Phase 4
   whether the default-feature clippy pass can run without `dist/` (or gate on a
   `dist/index.html` stub); decide whether to add macos to one lint job or to
   document that determinism rests entirely on exact pins.

8. **Tighten the remaining mechanical-sweep and config edges** (addresses:
   empty-file-set fail-open; mechanical-only safe-fix guard; rebase broadcast;
   which `types` key; `max_line_length` on JSON; `.mts`/`.cts` glob; durable
   AC→task mapping). Mostly one-line edits — see the Minor/Suggestion findings.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan extends the repo's established task-tree architecture
coherently — thin invoke wrappers, aggregates as pure `depends` fan-outs, one
mise task per CI job — and adds a third `types:*` family that mirrors the
existing `format:*`/`lint:*` split rather than adopting the work item's flat
`lint:<language>` names, a divergence explicitly acknowledged and traced in
Phase 6. Slice decomposition is strong: each per-component phase is
self-contained and independently mergeable, and deferring shared-aggregate
rewiring to Phase 6 keeps slices conflict-free and order-independent. The
principal architectural concerns are the cross-component coupling the server
clippy step introduces via `build:frontend` and an under-specified CI gating
coupling where each newly-blocking job must be wired into `prerelease.needs`
during staged landing.

**Strengths**:
- The three-family tree mirrors the existing convention exactly (colon-namespaced
  leaves ending `:check`/`:fix`, descriptions on every task, aggregates as pure
  `depends` fan-outs delegating to Python invoke).
- Per-component slice decomposition gives each component a single cohesive reason
  to change and lets slices land in any order — high cohesion, low inter-slice coupling.
- Deferring shared-aggregate rewiring to Phase 6 is a deliberate coupling control:
  per-component phases never touch shared lines.
- The Cargo.toml `[lints.clippy]` survivability concern is correctly identified as
  a coupling to `tasks/version.py`'s tomlkit round-trip and locked in with a guard test.
- Shell scope stays centralised in `tasks/shared/sources.py` — extended rather
  than duplicated.

**Findings**:
- 🟡 major (high) — **Server lint gate cross-couples to the entire frontend toolchain via `build:frontend`** (Phase 4 / Implementation Approach). `lint:server:check`/`fix` both depend on `build:frontend` (build.rs panics without dist/ under embed-dist), so the Rust lint gate pulls in node-install + Vite build and inherits a frontend-build failure mode. Suggestion: acknowledge the tradeoff; consider running the default-feature pass without dist/, or gate on a lighter dist/ stub.
- 🟡 major (medium) — **Newly-blocking per-component CI jobs not wired into `prerelease.needs` until Phase 6** (Phases 3-5 / Phase 6 §4). Between a slice landing and Phase 6 the new job runs but doesn't gate the release path. Suggestion: each phase adds its job to `needs:`; Phase 6 verifies completeness.
- 🔵 minor (high) — **Phase 6 widens `format:fix`, silently changing the `default` task** (Phase 6 §1 + default). `default` would now auto-apply Biome/rustfmt/ruff and pull in node install. Suggestion: state the intended post-rewire `default` semantics explicitly.
- 🔵 minor (high) — **`types:*` third family diverges from the work item's flat `lint:<language>` contract — coherent but adds a mapping seam** (What We're NOT Doing). Keep the divergence; ensure the Phase 6 AC→task mapping lives in a durable location (mise.toml comment, not only the PR description).
- 🔵 minor (medium) — **Asymmetric fix-task coverage breaks the format/lint family symmetry** (Phase 6 §1). `lint:fix` omits scripts; acceptable but make the asymmetry explicit so a generic "fix everything" driver needn't special-case it.

### Code Quality

**Summary**: Unusually disciplined for a tooling change: it faithfully mirrors
the established thin-wrapper idiom (`with context.cd(...)`, `warn=True`,
`raise Exit(...)`), preserves guard-clause style in the `_keep`/`shell_sources`
rewrite, and respects the no-underscore-module rule. The main maintainability
concerns are unaddressed test-file placement conflicting with an existing
`test_sources.py`, the two-pass clippy loop's fail-fast behaviour diverging from
the codebase's collect-all-failures precedent, and the ~10 near-identical
wrapper modules whose duplication, while idiomatic, has no shared helper.

**Strengths**:
- New wrappers faithfully mirror `tasks/lint/scripts.py` / `format/scripts.py`
  (repo-root cd, `warn=True`, `pty=False`, descriptive `Exit`).
- The `_keep` rewrite removes two obsolete branches and keeps the guard-clause
  structure; the `workspaces/` exclusion gains a substantive justification comment.
- Config files are authored with genuine explanatory comments rather than bare settings.
- The Cargo.toml templating guard test is a well-judged characterisation test.
- Testability is preserved: every wrapper follows the existing `MagicMock(spec=Context)` pattern.

**Findings**:
- 🔵 minor (high) — **Scope test placement ignores the existing `test_sources.py` that already covers `_keep`/`shell_sources`** (Phase 2 §7). The existing `test_excludes_fixtures_*` cases will fail after the `_keep` rewrite; the plan doesn't flag this red→green churn. Suggestion: point at the existing file and invert the cases.
- 🔵 minor (high) — **Two-pass clippy loop fails fast, diverging from the collect-all-failures precedent** (Phase 4 §3). `tasks/test/unit.py`'s `templates` task accumulates failures; consider matching it.
- 🔵 suggestion (medium) — **Ten near-identical wrapper modules duplicate the run/check/raise idiom with no shared anchor** (Phase 3 §3 / Phase 5 §5). Consider a tiny `run_check` helper, or decide deliberately to keep the flat idiom.
- 🔵 suggestion (medium) — **`repo_root`/path helpers now defined and imported from three places** (Phase 3 §3 / Phase 5 §5). Standardise the new wrappers on `tasks.shared.paths` constants.
- 🔵 suggestion (low) — **Redundant `_keep` filter applied to the hardcoded CLI source path** (Phase 2 §3). The filter can never exclude the fixed literal; drop the guard or note it's defensive.
- 🔵 suggestion (medium) — **Several wrappers specified only as one-line comments, leaving error-handling shape unverified** (Phase 3 §3 / Phase 5 §5). State explicitly that every check wrapper uses `warn=True` + a descriptive `Exit`.

### Test Coverage

**Summary**: A genuinely strong testing backbone: it gates each sweep on the
relevant existing suites as automated criteria, adds two purpose-built unit
tests (the shell scope test and the Cargo.toml templating guard), and resolves
the work-item pass-3 major by tightening the CI probe to one violation per check
command. The two material gaps are that the plan is unaware of an
already-existing `tests/unit/tasks/shared/test_sources.py` that encodes the OLD
shell-scope behaviour and will turn red, and that the pyrefly file-set
cross-check is left as an unmechanised manual check. The probe strategy is sound
for proving wiring, not coverage.

**Strengths**:
- Each per-component phase lists the relevant existing suites as automated success
  criteria, so "keep existing suites green" is a real gate.
- The CI falsification probe is per-check-command granularity, resolving the
  work-item pass-3 major.
- The shell scope coverage test is a falsifiable, TDD-first assertion.
- The `_render_cargo_toml` guard test protects the load-bearing tomlkit assumption.
- Report-only checkpoints give a sizing signal; the two-pass clippy mirrors the
  established two-pass precedent.

**Findings**:
- 🔴/🟡 major (high) — **Phase 2 scope test ignores the existing `test_sources.py`, which encodes the old behaviour and will break** (Phase 2 §7). Three assertions fail after scope-widening, making `mise run test:unit:tasks` exit 0 unachievable. Suggestion: invert the existing assertions rather than adding a parallel test.
- 🟡 major (high) — **Dropping shellcheck CLI flags breaks an existing `test_lint.py` assertion not in the keep-green list** (Phase 2 §2). `TestShellcheckTask.test_command` asserts the old prefix. Suggestion: add an explicit step to update it.
- 🟡 major (medium) — **pyrefly coverage cross-check is manual-only, with no test analogous to the shell scope test** (Phase 3 Manual Verification). A misconfigured exclude could leave files silently unchecked. Suggestion: add an automated coverage diff.
- 🔵 minor (medium) — **Falsification probes prove command wiring, not file-set coverage** (Testing Strategy). An in-tree probe can't detect a tool checking zero files. Suggestion: note the limitation; place a probe in a scope-widened file.
- 🔵 minor (medium) — **Cargo.toml guard test under-specifies the comment-preservation assertion** (Phase 4 §4). Specify asserting a verbatim comment survives, not just the table.
- 🔵 minor (low) — **"Mechanical-only, no behavioural change" for the frontend sweep rests on spot-checking plus Vitest** (Phase 5 Manual Verification). Add `test:e2e:visualiser` + visual-regression to Phase 5's gates.

### Correctness

**Summary**: The plan's load-bearing technical claims are largely sound and
verified against the codebase: the tsc no-op, the duplicate `types` key, the
embed-dist `build.rs` panic, the tomlkit round-trip, the clippy priority idiom,
and the ruff D-rule/formatter-conflict ignore lists all check out against the
actual files and the pinned tool versions (TypeScript 5.9.3 supports
`tsc -b --noEmit`). The two-pass clippy logic correctly covers both cfg arms.
The most material correctness risks are in the staged CI wiring and two
under-specified mechanical edits — which `types` key survives, and whether the
newly-referenced stray-file project needs `composite: true`.

**Strengths**:
- The two-pass clippy design is logically complete across both cfg arms (verified against `src/assets.rs`).
- The clippy `[lints.clippy]` priority idiom is correct (priority -1 pedantic, priority-0 allows override).
- `tsc -b --noEmit` holds against the installed TypeScript 5.9.3; the no-op diagnosis is accurate.
- The ruff ignore list is internally consistent (formatter-conflicting rules off; D211+D212 kept, D203+D213 ignored).
- `build:frontend` precedence is correctly wired on both clippy passes.
- The tomlkit round-trip claim is verified (`_render_cargo_toml` mutates only the version).

**Findings**:
- 🔴/🟡 major (medium) — **New per-component CI jobs not wired into `prerelease.needs` until Phase 6** (Phase 2 / Phase 6 §4). A failing job wouldn't block the push-triggered prerelease during the interim. Suggestion: wire `needs:` per phase.
- 🟡 major (medium) — **Bringing stray files into a `tsc -b` referenced project requires `composite: true`, which none of the existing projects set** (Phase 5 §4). Could fail the `types:frontend:check` criterion. Suggestion: run `tsc -b --noEmit` against the proposed layout in the report-only step.
- 🔵 minor (high) — **Plan says "keep one" `types` key without specifying which; the effective value is the second** (Phase 5 §3). Specify keeping `["node", "vite/client"]` (line 13).
- 🔵 minor (medium) — **Rewiring `format:fix` expands the `default` task to reformat Rust/Python/frontend in place** (Phase 6 §1 + default). Explicitly confirm the broadened `default` behaviour is intended.
- 🔵 minor (medium) — **`clippy --fix` run twice across differing feature sets can conflict** (Phase 4 §3). Specify the fix task's feature handling and `--allow-dirty`/ordering.
- 🔵 minor (low) — **`external-sources` honoured from `.shellcheckrc` is the plan's only self-flagged unverified shell claim** (Phase 2 §1). Make the report-only run a diff gate (`-x` vs rc-only) before dropping `-x`.
- 🔵 minor (low) — **Frontend EditorConfig glob omits `.mts`/`.cts`; the global `[*]` sets no indent_style, so Biome's tab default could apply** (Phase 1 §1). Add `mts,cts` to the glob.

### Standards

**Summary**: Exceptionally disciplined about the repo's conventions: it honours
the `<verb>:<scope>:<subscope>` + `:check`/`:fix` naming, puts a `description` on
every task, keeps aggregates pure `depends` fan-outs, follows the
one-mise-task-per-job CI shape, and faithfully encodes every hard work-item
requirement (adjacent suppression rationale, exact pins, `.editorconfig`
cross-references, CI executing mise tasks, the bash-3.2 floor, mandated
strictness). The one substantive convention question is the `check:<component>`
aggregate naming, which inverts the established ordering. A few smaller
deviations are mostly justified but should be tightened for traceability.

**Strengths**:
- Every new task carries a `description`, matching the universal convention and the documentation AC.
- All aggregates are pure `depends` fan-outs with no `run`.
- Leaf names follow `<verb>:<scope>:<subscope>` ending `:check`/`:fix`.
- CI jobs follow the checkout → mise-action → single `mise run` shape.
- Every standards-related work-item requirement is encoded (suppression rationale, exact pins, cross-references, bash-3.2 floor).
- Tool strictness levels are faithful and justified.
- The divergence from flat `lint:<language>` names is explicitly called out with the AC→task mapping deferred to Phase 6.

**Findings**:
- 🟡 major (high) — **`check:<component>` aggregate naming inverts the established `<verb>:<scope>` ordering** (Phases 2-5 + Phase 6). No existing `check:*` namespace anchors it. Suggestion: name roll-ups `<component>:check` or justify the form in "What We're NOT Doing".
- 🔵 minor (high) — **New invoke modules bypass the canonical `tasks.shared.paths` constants** (Phase 4 §3). Reuse `SERVER`/`CARGO_TOML` with `--manifest-path`; standardise the `repo_root` import source.
- 🔵 minor (medium) — **`default` task rewire is under-specified and risks dropping format/types from the local default flow** (Phase 6 §1). State the exact `default` dependency list.
- 🔵 minor (medium) — **ruff/rustfmt cross-reference comments mirror only line-length, not the indent value** (Phase 3 §1). Set+cross-reference indent or note it's intentionally at the matching default.
- 🔵 minor (medium) — **AC→task mapping in the PR description is not a durable in-repo artefact** (Phase 6 §2). Commit it as a mise.toml comment block.
- 🔵 minor (low) — **The widened `sources.py` module docstring must stay aligned with the new scope semantics** (Phase 2 §3). Show the revised docstring text.

### Usability

**Summary**: Competent and convention-following, but it leaves three ergonomic
gaps that bite daily users: no single "fix everything before I push" command and
no single discoverable "what will CI complain about" command, inconsistent
folding semantics across components (so behaviour can't be guessed from one task
to another), and invoke error messages that regress from the actionable hint the
existing shfmt wrapper already provides. Because enforcement is CI-only, these
local-workflow gaps determine whether a developer finds a violation before or
after a failed CI run.

**Strengths**:
- Every new task carries a `description` (an explicit success criterion), keeping `mise tasks` discoverable.
- The three-family + `check:<component>` shape is internally regular and mirrors the existing `format:scripts:*` shape.
- Separate `:check`/`:fix` leaves give a clear "checks never mutate" mental model.
- CI and local invocation are identical (every CI job runs a `mise run` task), so "what CI runs" is reproducible locally.
- pyrefly `--output-format github` and clippy annotations surface inline CI errors.

**Findings**:
- 🟡 major (high) — **No single "fix everything before I push" command, and the closest one silently omits shell** (Phase 6 §1 + Phase 2). Add a top-level `fix` aggregate whose description names the shell autofixer gap.
- 🟡 major (high) — **Invoke wrappers regress to non-actionable "tool reported findings" error messages** (Phase 2/3/4). Mirror the existing shfmt wrapper: include the remediation command where a fixer exists, name the manual step where it doesn't.
- 🟡 major (medium) — **Task-tree folding semantics are inconsistent across components** (Implementation Approach + aggregates). Partly inherent; document the folding rule in the Phase 6 mapping note. *(Curated down to minor in the aggregated findings — remedy is documentation.)*
- 🔵 minor (high) — **CI-only enforcement leaves no documented local "run before you push" workflow** (Migration Notes). Add a one-line onboarding note naming the pre-push loop.
- 🔵 minor (medium) — **Slow `lint:server:check` expectation is set for CI but not the local developer** (Phase 4 §3). Note the cost in the task description.
- 🔵 minor (medium) — **Overlapping `check` and `default` entry points blur which command to use** (Phase 6 §1 vs default). Make `default` depend on the same guardrail set as `check`, or document the division of labour.
- 🔵 minor (medium) — **Developer impact of the reformat sweeps (mass rebase) is noted for owners but not surfaced to the wider team** (Migration Notes). Broadcast each sweep and the re-apply command.

### Safety

**Summary**: This is a tooling/guardrails plan for a development repository, not
a production data system — the worst-case blast radius (a botched sweep or a
mis-wired CI gate) is bounded and VCS-revertable, which the plan correctly leans
on. The genuine risks are silent-failure modes: a renamed/added CI job silently
dropping a merge gate (fail-open), a lint task silently passing on an empty
file-set, an auto-fixer introducing behavioural change under the "mechanical-only"
banner, and the multi-phase window where the release pipeline's `needs:` doesn't
yet reference the new jobs. The Cargo.toml landmine is well-defended; per-phase
VCS revertability is real.

**Strengths**:
- The Cargo.toml silent-guardrail-loss landmine is defended with an explicit guard test plus a version:bump dry-run.
- Auto-fix sweeps are split into distinct mechanical commits, each independently revertable via VCS.
- The bash-3.2 floor is explicitly protected (no bash-4 constructs; bashisms remains a sibling gate covering the widened set).
- shfmt is invoked with no style flags and doesn't reflow logic; shellcheck runs report-only.
- Falsification probes verify each gate fails closed before it's trusted.

**Findings**:
- 🟡 major (medium) — **Renaming the `check` CI job risks silently dropping the merge gate** (Phase 2 §5 / Phase 6 §4). Branch-protection matches by job name (external to the repo). Make the required-check-name update a mandatory Phase 2 step with a verify-it-blocks check.
- 🟡 major (high) — **New per-component CI jobs not added to `prerelease` needs until Phase 6** (Phases 3-5 vs Phase 6 §4). A failing new job wouldn't block the push-triggered prerelease/attestation. Suggestion: add each job to `needs:` in its own PR.
- 🔵 minor (medium) — **Empty file-set causes lint tasks to silently exit zero (fail-open)** (Phase 2 §2). Fail loudly (or assert a minimum count) when the matched set is empty.
- 🔵 minor (medium) — **"Mechanical-only, no behavioural change" rests on implicit tool defaults** (Implementation Approach step 3 + Phase 6 lint:fix). State that `--unsafe`/`--unsafe-fixes` is never passed; keep the mechanical-diff spot-check for every component.
- 🔵 minor (medium) — **In-flight-work coordination relies on manual enumeration with no recovery aid** (Migration Notes). Recommend landing sweeps when the area is quiescent and re-running `format:fix`/`lint:fix` after rebase rather than hand-resolving conflicts.

### Compatibility

**Summary**: The plan treats version pinning and cross-platform/cross-version
behaviour as first-class and gets most of it right: rustfmt `edition = "2021"`
matches the crate, the Biome `$schema` is coupled to the installed package, the
two-pass clippy covers feature-gated arms, and the bash-3.2 floor is preserved.
The most serious risk is that the literal `.x` pins are mise fuzzy matches
(range operators), contradicting the exact-pin AC and undermining `select=ALL`
reproducibility. Secondary risks cluster around cross-environment determinism:
the pyrefly backend choice and the ubuntu-only lint jobs against an ubuntu+macOS base.

**Strengths**:
- rustfmt.toml hard-codes `edition = "2021"` matching the crate, preventing an edition-mismatch reformat.
- The two-pass clippy correctly addresses the `--all-features` feature-gating gap.
- The Biome `$schema` is pinned to the installed package, avoiding schema/binary skew.
- The bash-3.2 floor is explicitly protected.
- The mise-action and node pins are surfaced as explicit Phase 1 work with verification greps.

**Findings**:
- 🔴 critical (high) — **Literal `.x` version pins are mise range operators, not exact pins** (Phase 3 §2). With `select=ALL`, rule sets drift on upgrade; the Phase 3 grep won't catch `.x`. Commit full triples and tighten the regex.
- 🟡 major (medium) — **pyrefly install-path backend (`pipi:` vs `cargo:`) version consistency unspecified** (Phase 3 §2 + Key Discoveries). Different channels can resolve different artefacts across ubuntu CI and macOS. Assert the same backend+version everywhere; capture `pyrefly --version`.
- 🟡 major (medium) — **New lint jobs are ubuntu-only while tests/devs are ubuntu+macOS** (Phases 2-5 CI jobs). Tool behaviour can differ by platform. Document that determinism rests on exact pins, or add macos to one lint job.
- 🔵 minor (medium) — **`target-version = "py314"` global vs the 3.9-floor `mock-jira-server.py`** (Phase 3 §1). `ruff format` and other target-version-sensitive rules aren't file-scoped; confirm the per-file ignore set is complete or exclude the file (as done for pyrefly).
- 🔵 minor (medium) — **`max_line_length = 80` under `[*]` applies to JSON via Biome** (Phase 1 §1). Confirm 80-col JSON wrapping is intended or scope the width to source languages.
- 🔵 minor (high) — **`jdx/mise-action` exact pin vs `actions/checkout@v5` tag pin inconsistency** (Phase 1 §2). Keep the scope decision but record it as a deliberate choice in the plan.

## Re-Review (Pass 2) — 2026-06-10

**Verdict:** REVISE

All eight lenses were re-run against the revised plan (which addressed every
pass-1 finding plus the author's four design decisions and the `python` →
`build-system` component rename). The pass-1 findings are substantially
resolved, but the revisions introduced one new critical (a broken verification
grep) and a cluster of new majors — most notably a `build:frontend:stub`
shared-artifact race flagged independently by three lenses. **All new
critical/major findings and most minors were then addressed in the same
iteration** (see the Closing Note).

### Previously Identified Issues

- 🔴 **Compatibility**: `.x` range-operator pins — **Resolved** (full
  `MAJOR.MINOR.PATCH` triples + a hardened reject-`.x` grep; the grep itself had
  a bug, see New Issues).
- 🟡 **Architecture/Correctness/Safety**: CI jobs not in `prerelease.needs` until
  Phase 6 — **Resolved** (per-phase `needs:` wiring; Phase 6 §4 is now a
  completeness assertion).
- 🟡 **Safety**: `check`→`check-scripts` rename risks dropping the merge gate —
  **Resolved** (mandatory branch-protection rename + verify-it-blocks step).
- 🟡 **Architecture**: server lint coupled to the whole frontend toolchain —
  **Resolved** via the `build:frontend:stub` (decouples from node/Vite) — but the
  stub introduced a new race (see New Issues).
- 🟡 **Test-Coverage/Code-Quality**: Phase 2 breaks the existing `test_sources.py`
  — **Resolved** (points at the existing file, inverts the three assertions).
- 🟡 **Test-Coverage**: dropping shellcheck flags breaks `test_lint.py` —
  **Resolved** (explicit assertion-update step).
- 🟡 **Test-Coverage**: pyrefly/ruff coverage cross-check manual-only —
  **Resolved** (automated guard added; the guard's *mechanism* was unbuildable as
  first written, see New Issues).
- 🟡 **Correctness**: stray-file `tsc -b` needs `composite: true` — **Resolved**
  (report-only verification of the project layout added).
- 🟡 **Standards**: `check:<component>` naming — **Resolved** (renamed to
  `<component>:check` per the author's decision; the justification was wrong, see
  New Issues).
- 🟡 **Usability**: no single "fix everything" command — **Resolved** (top-level
  `fix` aggregate).
- 🟡 **Usability**: non-actionable error messages — **Resolved** (actionable
  `Exit` messages naming the fix command).
- 🟡 **Compatibility**: pyrefly backend determinism — **Resolved** (assert same
  backend+version; capture `pyrefly --version`).
- 🟡 **Compatibility**: lint jobs ubuntu-only — **Resolved** (documented
  determinism-rests-on-pins decision).
- 🔵 **Architecture/Correctness/Standards/Usability**: `default` task
  under-specified — **Resolved** (`default` repointed to add `types:check`; cost
  documented).
- 🔵 (other pass-1 minors: types-key, invoke-paths, indent cross-ref, AC-mapping
  durability, empty-file-set, safe-fix guard, `.mts`/`.cts`, JSON width, sweep
  broadcast, mise-action note) — **Resolved**.

### New Issues Introduced (by the revisions)

- 🔴 **Compatibility**: the hardened reject-`.x` grep used escaped pipes (`\|`),
  which are *literal* in ERE — the guard matched nothing and failed open. **Fixed**
  (bare alternation + a sanity-check-against-a-`.x`-fixture instruction).
- 🟡 **Architecture/Correctness/Safety**: `build:frontend:stub` and `build:frontend`
  write the same `frontend/dist/index.html` with no ordering edge — a TOCTOU race
  in the local `default` graph and stale-stub shadowing of later embed builds.
  **Mitigated**: CI `check` is race-free (never schedules a real build); added an
  existence-only/atomic implementation contract, a recognizable placeholder
  marker, the cfg refinement (pass-1 doesn't embed), and a `default` report-only
  verification with a documented fallback.
- 🟡 **Test-Coverage**: the fail-closed change also breaks
  `test_format.py::test_noop_when_no_sources` (not listed). **Fixed** (added to
  the inversion list).
- 🟡 **Test-Coverage**: the Python coverage guard's cited mechanism
  (`ruff check --show-files`/`--statistics`) can't enumerate the file set / is
  vacuous on a clean tree. **Fixed** (buildable two-part guard: config-set
  assertion + sentinel-in-scope probe).
- 🟡 **Compatibility**: `mock-jira-server.py` already uses `list[dict]` runtime
  annotations, so the `UP`-only ignore is insufficient for the 3.9 floor. **Fixed**
  (full `extend-exclude` of the file, mirroring pyrefly).
- 🟡 **Standards**: `<component>:check` justification cited `test:unit` (which is
  family-first, not scope-first). **Fixed** (cite the real `version:*`/`github:*`
  entity-first precedent; owned as a deliberate second namespace shape).
- 🟡 **Usability**: `build-system` is undiscoverable to a developer searching
  "python"; folding-rule/pre-push docs live only in a `mise.toml` comment.
  **Addressed** (descriptions name Python; a CONTRIBUTING/README contributor
  section + a `python`→`build-system` cross-ref required).
- 🟡 **Architecture**: `build:frontend:stub` sits in the `build:*` (artifact)
  namespace though it produces no deliverable. **Acknowledged** (name kept — it
  does write a stub dist — with the distinction documented in the folding-rule
  block).
- 🔵 **Code-Quality**: registration recipe omitted the package `__init__.py`
  imports — **Fixed**. Wrong `test/unit.py` precedent citation — **Fixed**.
- 🔵 **Safety**: verify-it-blocks not extended to the new required checks —
  **Fixed** (added to the per-component probe step). `lint:server:fix` partial-fix
  scope — **Fixed** (description). 
- 🔵 **Compatibility**: rustfmt/Cargo.toml edition are unsynchronised literals —
  **Fixed** (cross-ref comment + guard-test note). Biome `$schema` node_modules
  path / mise self-version pin — **noted, accepted** as-is.
- 🔵 **Code-Quality** (deferred): `tasks/lint/scripts.py` still imports
  `repo_root` from `tasks.shared.sources` (a third repo-root source) — accepted
  scope, not churned further. The clippy comprehension-vs-explicit-loop style and
  a dedicated two-pass-clippy unit test are open suggestions.

### Assessment

The revision is a strong net improvement: all thirteen pass-1 majors and the
critical are resolved, and the lenses now operate at a finer grain (verifying
the dist-stub against `build.rs`/`assets.rs`, the named tests/fixtures, and the
`tasks.shared.paths` constants against the real codebase). The mechanical
re-review verdict is REVISE because the revisions introduced a new critical (the
broken grep) plus the dist-stub race and several major verification gaps — but
**every one of those was addressed in the same iteration** (Closing Note below).
The residual open items are minor/suggestion-level (repo-root source
consolidation, a clippy-loop unit test, the comprehension style). The single
finding worth the author's explicit attention is the **`build:frontend:stub`
shared-artifact race**: it is now mitigated and CI is race-free, but if the
`default`-graph race proves real in practice the documented fallback (drop the
stub from `default`, or have `lint:server:*` clean up the stub it created)
should be taken — or reconsider whether the full `build:frontend` dependency is
simpler than the stub for the local path.

### Closing Note — 2026-06-10

The pass-2 new findings were addressed the same day: the broken reject-`.x` grep
was rewritten with bare alternation; the `build:frontend:stub` gained a
shared-artifact safety section (CI race-free, existence-only/atomic contract,
recognizable marker, cfg refinement, `default` report-only verification +
fallback); `test_format.py::test_noop_when_no_sources` was added to the inversion
list; the Python coverage guard was respecified as a buildable two-part
(config-set + sentinel-probe) check; `mock-jira-server.py` is now fully
`extend-exclude`d; the `<component>:check` justification now cites the real
`version:*`/`github:*` precedent; discoverability was improved (Python-naming in
descriptions, a required contributor section, a `build-system`↔`python`
cross-ref); the registration recipe gained the package `__init__.py` imports; the
`test/unit.py` citation, `lint:server:fix` scope, rustfmt edition cross-ref,
verify-it-blocks extension, `default` cost note, and test-file-diff review note
were all applied. With these, only minor/suggestion-level items remain open; a
confirming pass-3 would be expected to land **COMMENT/APPROVE**.

## Re-Review (Pass 3) — 2026-06-10

**Verdict:** REVISE

All eight lenses were re-run against the pass-2 revisions, with each lens
verifying its fixes against the **actual codebase** (not the research doc). The
pass-2 fixes hold up well, and pass 3 surfaced one **latent** issue earlier
passes missed by trusting the research's stale description — plus two stub
edge-cases that only emerge from reading the real build tasks. All were addressed
this iteration (Closing Note).

### Previously Identified Issues (pass-2 fixes verified)

- 🔴 **Compatibility**: broken escaped-pipe `.x` grep — **Resolved & verified**
  (bare alternation; the correctness/compat lenses mentally executed the new
  greps against the real `[tools]` block and confirmed they fire correctly).
- 🟡 **Arch/Correctness/Safety**: dist-stub TOCTOU — **Verified**: CI `check` is
  genuinely race-free (traced the DAG — `frontend:check` depends on
  `deps:install:node`, never a real `build:frontend`); the `cfg`-out claim for
  pass-1 is verified against `src/assets.rs`. Residual stub edge-cases found (see
  New Issues).
- 🟡 **Test-Coverage**: `test_format.py` inversion — **Verified** (the named test
  exists and asserts the old silent-noop exactly as described).
- 🟡 **Test-Coverage**: buildable coverage guard — **Resolved** (config-set +
  sentinel-probe is buildable; the `git ls-files` ground-truth needed a further
  fix, see New Issues).
- 🟡 **Compatibility**: mock-jira full `extend-exclude` — **Verified correct**
  (path matches the single real 3.9-floor file; full exclude is the right call).
- 🟡 **Standards**: `<component>:check` justification — **Verified**:
  `version:read`/`version:write`/`version:bump`/`github:check-auth` all exist and
  genuinely establish the entity-first precedent; the `test:unit` retraction is
  correct.
- 🟡 **Code-Quality/Standards**: registration recipe — **Verified accurate**
  against the real `tasks/format/__init__.py` (`from . import scripts` + `__all__`)
  and `tasks/__init__.py` (`Collection.from_module`).
- 🟡 **Usability**: discoverability, `fix` description, contributor section,
  `default` cost — **Verified resolved** (all five DX fixes landed; README
  confirmed to have no contributor section, so the new section is justified).
- 🔵 (rustfmt edition cross-ref, verify-it-blocks extension, `build-system`
  invoke namespace mapping) — **Verified** (the invoke `auto_dash_names`
  `build_system`→`build-system` mapping confirmed against invoke 2.2.1 source).

### New Issues Introduced / Surfaced (pass 3)

- 🔴 **Code-Quality/Test-Coverage/Safety** (latent, missed by passes 1–2): Phase 2
  §3 rewrote `shell_sources()` around `git ls-files '*.sh'`, but the **real**
  implementation is a deliberate `os.walk`/`pathspec` gitignore walk — its
  docstring records `git ls-files` was abandoned because it is **jj-workspace-blind**
  (silently emptied the scan). Following the plan would revert that fix and break
  five existing walk tests. **Fixed**: Phase 2 §3/§7 rewritten to preserve the
  walk (modify `_keep` + append the CLI script), invert only the three exclusion
  tests, keep the five walk tests green, add a CLI-script test. The research doc's
  `git ls-files` description is now flagged as stale.
- 🟡 **Test-Coverage**: the Python coverage guard inherited the same `git ls-files
  '*.py'` jj-blindness. **Fixed**: compute the expected set with a VCS-agnostic
  walk (factored to share one helper with the shell walk); two stale
  success-criteria/strategy references updated.
- 🟡 **Safety**: the stub "all real embed flows overwrite `dist/`" claim is **false**
  — `build:server:release` is embed-dist but has **no `build:frontend` dependency**
  (only `build:server:cross-compile` does), so a stale stub could be embedded into
  a local release binary. **Fixed**: add `build:frontend` to `build:server:release`'s
  `depends` (also hardens a pre-existing fragility); zero-byte `index.html` treated
  as absent.
- 🟡 **Arch/Correctness**: the `default`-graph stub/real-build race was mitigated
  only report-only. **Fixed**: promoted to a blocking Phase 6 success criterion
  (`mise run default` on a clean tree yields a non-stub `dist/index.html`).
- 🔵 **Test-Coverage/Safety**: fail-closed empty-set guard tested only for shfmt —
  **Fixed** (add `pytest.raises(Exit)` tests to `TestShellcheckTask`/`TestBashismsTask`).
- 🔵 **Correctness/Compatibility**: the `.x` rejection grep used a bare `^` anchor
  (brittle to indented keys) and only caught explicit `.x` — **Fixed**
  (`^[[:space:]]*`; documented the presence-grep AND-gate that catches bare
  prefixes).
- 🔵 **Standards**: two-layer registration spelled out only in Phase 3 —
  **Fixed** (Phase 4/5 back-reference the §3 recipe). **Test-Coverage**: edition-sync
  test added to Phase 4 §4; sentinel-probe fixture strategy specified (temp file at
  a real in-scope path + try/finally).
- 🔵 **Usability**: no `<component>:fix` symmetry; `default` "fast inner loop"
  points to a check — **Addressed** (folding-rule note documents the no-`:fix`
  shape and the entity-first/family-second dual role); the inner-loop pointer is
  left as-is (a check is a reasonable fast-verify target).
- 🔵 (deferred, accepted): `tasks/lint/scripts.py` still imports `repo_root` from
  `tasks.shared.sources` (a third repo-root source) — left as scoped; a
  two-pass-clippy command-issuance unit test and the clippy comprehension-vs-loop
  style remain open suggestions.

### Assessment

Pass 3 is the most valuable pass: by verifying against the live code rather than
the research doc, it caught a latent `git ls-files`-revert that would have
re-introduced a documented jj-workspace fail-open bug, plus a real
stub-into-release shadowing path. All are now fixed. The plan is now consistent
with the actual implementation on every load-bearing point the lenses checked
(the walk, the `cfg`-out embed mechanics, the `version:*` naming precedent, the
invoke namespace mapping, the registration wiring, the pin greps). The mechanical
verdict is REVISE (a critical + several majors were found this pass), but every
one was addressed same-iteration, and what remains is genuinely
minor/suggestion-level.

The one standing judgment call for the author: the `build:frontend:stub` has now
accrued mitigations across three passes (CI race-free by construction, a
`build:frontend` dep added to `build:server:release`, a blocking `default`
no-stub check, zero-byte handling, a recognizable marker). It is now sound, but
it is also the single most complexity-generating element in the plan, and its
only benefit is keeping the `check-server` CI job off a full Vite build. If that
complexity feels disproportionate, the documented fallback — revert
`lint:server:*` to `depends = ["build:frontend"]` (a cold SPA build per
check-server run; caching is out of scope) — trades CI time for the elimination
of every stub edge-case. Both are defensible; the stub as now hardened is safe to
proceed with.

### Closing Note — 2026-06-10

The pass-3 findings were addressed the same iteration: Phase 2 §3/§7 rewritten to
preserve the `os.walk`/`pathspec` shell discovery (no `git ls-files` revert), the
Python coverage guard moved to a VCS-agnostic walk, `build:frontend` added to
`build:server:release` with zero-byte stub handling, the `default` no-stub check
promoted to a blocking criterion, fail-closed tests added for shellcheck/bashisms,
the pin greps hardened (`^[[:space:]]*` + documented AND-gate), Phase 4/5
registration back-referenced, the edition-sync and sentinel-probe tests specified,
and the folding-rule comment extended with the namespace-shape/`:fix` notes. The
remaining open items are minor/suggestion-level. A confirming pass-4 would be
expected to land **APPROVE** (the only non-trivial open question is the author's
stub-vs-simplicity preference, which is a documented decision, not a defect).

## Re-Review (Pass 4) — 2026-06-10

**Verdict:** COMMENT

All eight lenses were re-run against the pass-3 revisions, verifying each fix
against the live codebase. **The plan has converged: zero critical and zero major
findings this pass** (down from 1 critical + 12 majors at pass 1). Compatibility
returned completely clean (no findings). The author confirmed keeping the
`build:frontend:stub`. The remaining items are all minor/suggestion-level polish,
and the highest-value ones were applied this iteration.

### Previously Identified Issues (pass-3 fixes verified against live code)

- 🔴→✅ **shell_sources `git ls-files` revert** — **Verified resolved**: Phase 2
  §3 now matches the real `os.walk`/`pathspec` module; the three exclusion tests
  and five walk tests all confirmed to exist with the names/split the plan states.
- 🟡→✅ **Python coverage guard jj-blindness** — **Verified resolved** (VCS-agnostic
  walk; the factoring caveat was refined this pass).
- 🟡→✅ **`build:server:release` stub-shadowing** — **Verified resolved**: the task
  confirmed to have no `depends` today; adding `build:frontend` is purely additive
  (no in-repo consumer breaks) and aligns it with `build:server:cross-compile`.
- 🟡→✅ **`default` race blocking criterion**, **fail-closed shellcheck/bashisms
  tests**, **hardened `^[[:space:]]*` pin greps** (BSD/GNU-portable, AND-gate
  verified), **registration back-refs**, **edition-sync test** — all **verified
  correct** against the real files.

### New / Residual Issues (this pass) — all minor/suggestion

- 🔵 **Correctness/Safety**: the stub/real-build co-scheduling extends to the local
  `test`/`default` aggregates (not just `check`), and the zero-byte-as-absent rule
  widens the TOCTOU window. **Addressed**: clarified that CI is race-free because
  each job runs a single aggregate on a fresh checkout (lint and test never share a
  tree in CI), the blast radius is a local test/dev build only (never a shipped
  binary), and the stub's atomic write (temp + `os.replace`) prevents partial-file
  reads.
- 🔵 **Usability**: `build:server:release` now runs a full Vite build unconditionally
  with no description hint. **Fixed** (description-update instruction added; rationale
  refined to note it's a standalone dev task, not the release path).
- 🔵 **Code-Quality**: the elided `shell_sources` walk body could be reconstructed
  wrong. **Fixed** ("leave the walk body byte-for-byte unchanged"). The `.py`-walk
  factoring was overstated. **Fixed** (downgraded to optional, with the real
  signature caveat).
- 🔵 **Test-Coverage**: `TestShellcheckTask.test_command` should assert flag
  *absence* explicitly. **Fixed** (`assert "-x" not in cmd` / `"--severity" not in
  cmd`). Open suggestions noted: a mutation test for the defensive `_EXTRA_SHELL_SOURCES`
  filter, and isolating the sentinel-probe path (gitignore it / integration-tier).
- 🔵 **Usability/Standards** (accepted): the per-component-fix recovery hint added to
  the contributor note; the `build:frontend` leaf-vs-`build:frontend:stub`-prefix
  naming collision is documented and accepted (a future `build:frontend:dist`
  rename is out of scope).

### Assessment

After four passes the plan is verified consistent with the actual implementation
on every load-bearing point (the walk, the cfg-out embed mechanics, the
`version:*` naming precedent, the invoke namespace mapping, the registration
wiring, the pin greps, the release graph). The mechanical verdict is **COMMENT**:
no critical, no major — the plan is acceptable and ready for implementation, with
the residual minors being optional polish (several already applied). The one
standing item is the documented `build:frontend:stub`-vs-simplicity decision,
which the author has settled in favour of keeping the (now thoroughly hardened)
stub.

### Closing Note — 2026-06-10

The pass-4 minors were largely applied the same iteration: the CI-race-free
reasoning was sharpened (separate jobs / fresh checkouts; local-only
co-scheduling with bounded blast radius; atomic stub write), `build:server:release`
gained a description-update instruction and a precise standalone-dev-task
rationale, the `shell_sources` walk-body-unchanged instruction was made explicit,
the `.py`-walk factoring was downgraded to an optional follow-up, the
`test_command` flag-absence assertions were made explicit, and the
per-component-fix recovery hint was added to the contributor note. The remaining
open items are genuinely optional (a sentinel-probe isolation refinement, a
mutation test for the `_EXTRA_SHELL_SOURCES` filter, a possible future
`build:frontend:dist` rename). **The plan is ready for implementation.**

## Manual Approval — 2026-06-11

The author (Toby Clemson) manually approved the plan, overriding the pass-4
closing verdict from **COMMENT** to **APPROVE**. After four review passes the
plan converged to zero critical and zero major findings, was verified against
the live codebase, and was re-validated against the rebased base
(`5f0375c3`) — the rebase touched none of the plan's structural foundations and
only sweep-scope counts plus two concrete inventory items (the two new
`scripts/visual-diff-ciede2000*` stray files and the untyped `pathspec`
dependency) were refreshed. The remaining open items are optional
suggestion-level polish. No findings remain open; the plan status is set to
`ready` for implementation.
