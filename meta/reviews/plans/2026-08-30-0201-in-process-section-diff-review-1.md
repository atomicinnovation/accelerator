---
type: "plan-review"
id: "2026-08-30-0201-in-process-section-diff-review-1"
title: "Plan Review: In-Process Section Diff"
date: "2026-08-30T15:04:32+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-08-30-0201-in-process-section-diff"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "compatibility", "standards"]
review_number: 1
review_pass: 2
tags: []
last_updated: "2026-08-30T17:04:14+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: In-Process Section Diff

**Verdict:** COMMENT

The plan is sound and ready to implement. Its highest-risk assertions — the exact `similar` hunk-header shapes (`@@ -1 +1 @@`, `@@ -0,0 +1 @@`, `@@ -1 +0,0 @@`), the unreachable `\ No newline` marker, the byte-exact `EXPECTED` golden, and the losslessness of the infallible-render migration — were all verified against `similar` 2.7.0 source and hold. Two major findings are worth addressing before implementation, neither structural: Phase 3's zero-spawn rule ships without the probe-pair every other `pup.ron` rule carries, and it enforces the invariant with an import rule the plan itself documents as blind to inline-qualified spawns. The remaining findings are small — a missed docstring, a dead defensive guard flagged by three separate lenses, and a caret-pin-vs-exact-golden tension.

Plan is acceptable but could be improved — see the two major findings below.

### Cross-Cutting Themes

- **The trailing-newline guard is dead, untested, and mis-rationalised** (flagged by: Code Quality, Test Coverage, Correctness) — three lenses independently landed on the same three lines in Phase 2's `render`. The `if !body.is_empty() && !body.ends_with('\n')` branch never fires for production inputs (`differing_sections` fields are always `\n`-terminated, and `similar`'s non-empty body ends in `\n`); no proposed test provokes it; and Correctness proved the plan's stated rationale is wrong — a non-terminated field would trigger `similar`'s `\ No newline at end of file` marker, not a missing newline, so the guard does not protect the case it claims to.
- **Phase 3 zero-spawn enforcement lacks rigour** (flagged by: Architecture, Standards) — Architecture questions the import-rule mechanism (blind to inline `std::process::Command::new()`, diverging from the `corpus-adapters` runtime-harness precedent the research recommended); Standards flags the missing probe-pair registration in `test_import_rule.py` that every shipped rule carries. Different fixes, same weak spot: the invariant AC7 exists to make load-bearing is only guarded against the common case, and only until the next edit.
- **`similar` caret pin conflicts with byte-exact goldens** (flagged by: Compatibility, Test Coverage) — `similar = "2"` permits minor bumps, yet the inline `EXPECTED` golden and the AC3 test assert exact hunk bytes. The workspace exact-pins output-sensitive crates (clap, tar, jj-lib) and reserves carets for output-insensitive ones. A benign `similar` reformat would red CI as a non-regression.
- **0213 inherits a doubly-stale contract** (flagged by: Architecture, Compatibility) — the rename plus resignature leaves the unimplemented 0213 research pointing at `work_adapters::diff_shellout::render` returning `Result` over a `---/+++` body. Both suggest one breadcrumb line in Migration Notes.

### Findings

#### Major

- 🟡 **Standards**: New `pup.ron` rule omits the documented probe-pair registration
  **Location**: Phase 3 — Changes Required / Success Criteria
  Phase 3 adds `work_adapters_is_zero_spawn` to `cli/pup.ron` but registers no matching probe pair in `tests/integration/pup/test_import_rule.py`, relying on a one-off manual check. Every other shipped rule has a violation/compliant probe pair driven against the real `pup.ron`, and `tasks/README.md` documents this as the only guard — a mistyped or deleted rule is otherwise silent.

- 🟡 **Architecture**: Zero-spawn enforcement diverges from the harness precedent and relies on a guard with a self-documented gap
  **Location**: Phase 3; What We're NOT Doing
  The plan enforces AC7 with a crate-wide `RestrictImports` rule rather than the runtime harness (`corpus-adapters/tests/zero_spawn.rs`) the research recommended, and documents that the rule catches `use`-path imports but not an inline `std::process::Command::new()`. A future inline-qualified spawn would pass CI and silently reintroduce the process dependency this item removes.

#### Minor

- 🔵 **Code Quality**: `diff.rs` module docstring still names the deleted `diff_shellout` module
  **Location**: Phase 2, Section 3 (`cli/work-cli/src/diff.rs`)
  The edit list changes only the `use` line and the `RunOutcome` enum, but the module docstring (lines 1-4) still reads `work_adapters::diff_shellout::render`. The plan catches the parallel stale docstring in `lib.rs` but overlooks this one.

- 🔵 **Test Coverage**: The `last_updated`-only case loses its end-to-end render assertion
  **Location**: Testing Strategy → Integration Tests (final paragraph)
  The retired `cli_diff_parity` case asserted the CLI *renders* a `last_updated`-only frontmatter edit into the body; the `differing_sections` unit test the plan points to only asserts *detection*. The new AC3 golden uses a `## Summary` heading change, so no surviving test proves the detect→render pipeline for that `IGNORE_KEYS`-adjacent field.

- 🔵 **Test Coverage**: Deleting the failing-renderer test leaves the sync Renderable body-concatenation unasserted
  **Location**: Phase 2, Change 4 (`cli/work-adapters/src/sync/run.rs`)
  `a_failing_renderer_downgrades_to_unrenderable_with_raw_values` was the only `render_dossier` test passing non-empty sections; the surviving Renderable tests use `Vec::new()` sections and assert only `status: renderable`. Deleting the `sections.push_str(&render(section))` loop body would fail no automated test.

- 🔵 **Test Coverage / Code Quality / Correctness**: The trailing-newline guard is dead code with an inaccurate rationale
  **Location**: Phase 2, Change 2 (`render` body)
  The `if !body.is_empty() && !body.ends_with('\n')` branch never fires for any input `from_lines` can produce; no test exercises it; and its "guards a non-terminated field" rationale is wrong — such a field would emit a `\ No newline` marker (body still ends in `\n`), not a missing newline. Drop the guard and unconditionally `push('\n')`, or back it with a test that provokes it.

- 🔵 **Compatibility**: `similar` caret-pinned but the golden asserts exact bytes
  **Location**: Phase 2, Section 1 (workspace dependency)
  `similar = "2"` permits minor upgrades, yet the inline `EXPECTED` golden asserts exact hunk bytes. The workspace exact-pins behaviour-sensitive crates and reserves carets for output-insensitive ones. Either exact-pin with a rationale, or note that the golden intentionally guards `similar`'s output so an update surfaces as a re-bless.

- 🔵 **Architecture**: A now-pure renderer stays in the "outbound adapters" crate
  **Location**: Phase 2, Step 2
  After the change `render` is pure (`&SectionDiff -> String`, no I/O), yet it remains in `work-adapters`, whose docstring the plan rewords to advertise "in-process section diffing" alongside genuine adapters. Keeping it there (to avoid adding `similar` to the domain crate) is defensible, but the tradeoff is settled implicitly rather than stated.

#### Suggestions

- 🔵 **Standards**: `pup:check` task name is verifiable — state it rather than hedge
  **Location**: Phase 3, Success Criteria
  The criterion hedges "confirm the exact task name via `mise tasks`". `pup:check` is declared in `mise.toml` and documented in `tasks/README.md`; drop the caveat.

- 🔵 **Standards**: Explanatory inline comment in the new AC3 CLI test breaks the low-comment convention
  **Location**: Phase 2, Section 7
  `// Two sections; only "## Summary" differs …` describes what the two `fs::write` calls already express; `cli_diff.rs` constructs identical fixtures without narration. Drop it.

- 🔵 **Test Coverage**: AC3 exact-match golden is brittle against the caret-pinned differ
  **Location**: Phase 2, Change 7
  `assert_eq!(stdout, EXPECTED)` on deliberately-unfrozen `similar` output can red CI on a benign minor bump. The same test already asserts structurally via `contains("@@ -1 +1 @@")`.

- 🔵 **Code Quality**: The `similar` dependency comment restates facts `deny.toml` already governs
  **Location**: Phase 2, Section 1
  Trim to the single non-obvious point — why default features only — and drop the licence/allow-list restatement that can drift from `deny.toml`.

- 🔵 **Architecture / Compatibility**: 0213 inherits a stale call site and body shape
  **Location**: Migration Notes / What We're NOT Doing
  The rename + resignature leaves the 0213 research pointing at `diff_shellout::render` returning `Result` over a `---/+++` body. Add one Migration Notes line recording that both are superseded, so the future consumer adopts `work_adapters::diff::render(&SectionDiff) -> String` and the hunk-only body deliberately.

### Strengths

- ✅ Every high-risk `similar` output claim verified against 2.7.0 source: the hunk-header numbering (`len==1` omits `,len`; empty range rewrites start to `beginning-1`), the empty-string-tokenizes-to-zero-lines behaviour, the forced `newline_terminated=true` making the `\ No newline` marker unreachable, and the byte-exact `EXPECTED` golden.
- ✅ The infallible migration is genuinely lossless: every `DiffUnavailable` trigger (tempdir/write/spawn/timeout/pipe-read) ceases to exist with the subprocess; `DossierRender::Unrenderable` correctly survives via its orthogonal `local_unreadable` trigger, and `raw_sections`' only caller was the removed `Err` arm.
- ✅ The "feed fields verbatim, do not re-append `\n`" insight is load-bearing and correct — re-appending would widen `@@ -1 +1 @@` to `@@ -1,2 +1,2 @@` with a phantom trailing context line.
- ✅ Coupling analysis is thorough: the signature change ripples atomically through all four consumer sites, and the `similar` dependency is confined to one module — a single future point of coupling.
- ✅ Phase decomposition (retire parity → migrate → add rule) is independently mergeable, each phase keeping the build green, and Phase 1 clears dead test surface before Phase 2's signature change touches it.
- ✅ The 10→8 baseline-guard realignment is verifiably honest — exactly two `test` rows removed, five `hash` rows and the fixture corpus deleted in lockstep so the guards stay non-vacuous.
- ✅ The module rename `diff_shellout` → `diff` drops an implementation-incidental suffix now that no shell-out remains, and the plan is diligent about rewording every stale docstring, error message, and comment the subprocess left behind.
- ✅ Framing contract provably preserved: the new `render` reproduces the current header format string and trailing-blank logic verbatim, pinned byte-exact by the identical-sides test.

### Recommended Changes

1. **Register a `work_adapters_is_zero_spawn` probe pair in `tests/integration/pup/test_import_rule.py`** (addresses: Standards major) — a synthetic `work-adapters` crate whose module imports `std::process::Command` asserting `is denied` plus the rule name, and a compliant control, mirroring the `remote-projection` spawn probe. Note the sibling `work_adapters_filesystem_reads_in_process` rule lacks one too — close both.
2. **Resolve the Phase 3 enforcement mechanism explicitly** (addresses: Architecture major) — either add the runtime zero-spawn harness alongside the import rule (matching `corpus-adapters`/`vcs_adapters` precedent) for regression protection against a future inline spawn, or state in the plan that the residual inline-spawn gap is an accepted risk given the crate's small surface — rather than framing the import rule as equivalent to the harness.
3. **Drop the trailing-newline guard** (addresses: the dead-guard cross-cutting finding) — unconditionally `out.push('\n')` after the body and remove the prose caveat; if retained, back it with a test that provokes the branch. Remove the "guards a non-terminated field" rationale either way.
4. **Settle the `similar` pin against the goldens** (addresses: Compatibility minor, Test Coverage suggestion) — exact-pin `similar` with a rationale comment matching the workspace's output-sensitive discipline, or keep the caret and lean the AC3 test on its structural `@@`/`-`/`+` assertions.
5. **Add the `diff.rs` module docstring to Phase 2's edit list** (addresses: Code Quality minor) — reword `diff_shellout::render` → `diff::render`.
6. **Cover the two coverage gaps** (addresses: Test Coverage minors) — point the AC3 golden (or add a case) at a `last_updated`-only change to keep the detect→render pipeline asserted, and give one surviving `render_dossier` Renderable test a non-empty section that asserts the rendered body lands under the header.
7. **Apply the small standards fixes** (addresses: Standards suggestions) — drop the AC3 inline comment and the `pup:check` hedge; trim the `similar` dependency comment; add the 0213 breadcrumb to Migration Notes.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally sound and well-sequenced: it collapses a three-mode subprocess failure (`DiffUnavailable`) into an infallible pure function, isolates the new `similar` dependency to a single module, and correctly traces the coupling ripple through all four consumer sites and the surviving dual-trigger `DossierRender::Unrenderable` variant. Two structural tensions stand out — the crate-wide zero-spawn enforcement diverges from the established harness precedent (and the research's own recommendation) toward an import rule the plan itself documents as having a spawn-detection gap, and the now-pure renderer sits in an "outbound adapters" crate whose identity it no longer matches. Both are defensible but the first weakens the very invariant AC7 exists to establish.

**Strengths**:
- Infallible `render(&SectionDiff) -> String` cleanly collapses three failure modes (temp-dir/write, spawn, timeout) into a single pure computation, propagated atomically through all four edit sites.
- The `similar` dependency is confined to one module (`diff.rs`) — a future breaking change touches exactly one file.
- Coupling analysis is thorough and correct: `DossierRender::Unrenderable` survives via `local_unreadable`, `raw_sections` is safely removable, and removing the timeout introduces no new failure boundary.
- Phase decomposition is independently mergeable with each phase green, and the ordering avoids touching dead test surface during the signature change.

**Findings**:
- 🟡 major (confidence medium) — *Phase 3; What We're NOT Doing* — Zero-spawn enforcement diverges from the harness precedent and relies on a guard with a self-documented gap. AC7's goal is a genuinely zero-spawn crate, but Phase 3 uses a crate-wide `RestrictImports` rule rather than the runtime harness (`corpus-adapters/tests/zero_spawn.rs`) the research recommended. The plan documents the rule catches `use`-path imports but not an inline `std::process::Command::new()`, and the sibling `vcs_adapters` comment already states "the harness is what establishes zero-spawn". A future inline spawn would pass CI and silently reintroduce the process dependency. Suggestion: add the harness alongside the rule, or state the residual gap is an accepted risk — rather than framing the import rule as equivalent to the harness.
- 🔵 minor (confidence medium) — *Phase 2, Step 2* — A now-pure renderer stays in the "outbound adapters" crate, blurring the functional-core/imperative-shell boundary. `render` is pure yet remains in `work-adapters`; the domain crate already owns the pure `SectionDiff`/`differing_sections`. Keeping it there avoids adding `similar` to the dependency-light domain crate — reasonable, but state the tradeoff rather than leave it an unexamined consequence of the in-place rename.
- 🔵 suggestion (confidence low) — *Phase 2, Step 7; Migration Notes* — Body format now implicitly coupled to `similar`'s `unified_diff` shape ahead of the planned 0213 consumer. The body "free to change" today becomes a de-facto contract once 0213 parses it. Note that the body is a `similar` implementation detail until 0213 formalises a contract.

### Code Quality

**Summary**: A strong, simplifying plan: it collapses a subprocess+temp-file+poll-loop renderer into a small pure function, makes the renderer infallible, and ripples that infallibility through the call chain to delete an error variant, an outcome arm, a fallback renderer, and several tests. The design is more testable and the plan is notably diligent about rewording stale doc comments and error messages. Two minor concerns: one module docstring the edit list overlooks, and a carried-over defensive branch that becomes dead code.

**Strengths**:
- Making `render` infallible removes `Result<String, DiffUnavailable>` threading through four files and deletes the `RunOutcome::DiffUnavailable` arm, the `DossierRender` diff-failure trigger, and `raw_sections`.
- The replacement is a small pure function with no injectable test knobs (`cap`, `configure`) — strictly simpler than the old `render_with`/`run_capped` machinery.
- Unusually careful comment/message hygiene: rewords the `lib.rs` docstring, the `unrenderable_header` message, the `Cargo.toml` comment, and the stale `pup.ron` rule comment.
- Clean domain naming: `diff_shellout` → `diff` drops an implementation-incidental suffix.

**Findings**:
- 🔵 minor (confidence high) — *Phase 2, Section 3 (`cli/work-cli/src/diff.rs`)* — The edit list changes only the `use` line and the enum, but the module docstring (lines 1-4) still reads `work_adapters::diff_shellout::render`. The plan catches the parallel stale docstring in `lib.rs` but overlooks this one. Add it to the edit list.
- 🔵 suggestion (confidence medium) — *Phase 2, Section 2 (`render` body)* — The carried-over guard `if !body.is_empty() && !body.ends_with('\n')` can never fire for real inputs (bodies always end in `\n`). It is dead code that makes a maintainer reason about an impossible case. Drop it and unconditionally `push('\n')`, or back it with a test.
- 🔵 suggestion (confidence low) — *Phase 2, Section 1* — The four-line `similar` dependency comment restates facts `deny.toml` already governs (licence, allow-list) and can drift. Trim to the single non-obvious point — why default features only.

### Test Coverage

**Summary**: The coverage-retirement bookkeeping is largely sound: the 10→8 baseline-guard realignment is honest, the fixture corpus and hash rows are deleted in lockstep so the guards stay meaningful, and the one-sided cases are re-covered by strong new `diff.rs` unit tests. However, the mapping from retired cases to survivors is overstated in two places — the `last_updated`-only case loses its integration-level render assertion, and deleting the failing-renderer test leaves the sync Renderable section-body concatenation without a direct assertion. A couple of assertions are also mutation-weak.

**Strengths**:
- The `recorded.len()` 10→8 realignment is verifiably honest: exactly the two section-diff rows removed, five hash rows plus the fixture directory deleted in lockstep so the guards stay non-vacuous.
- The one-sided fixture case is genuinely re-covered: new `diff.rs` tests assert `@@ -0,0 +1 @@` / `@@ -1 +0,0 @@` with explicit `!contains("No newline …")` guards — stronger than the retired suite.
- The renderer-injection seam is a legitimate mock boundary; the real `render` is exercised by the `diff.rs` unit tests and the AC3 golden.
- Preserves red-green-refactor discipline and keeps each phase green.

**Findings**:
- 🔵 minor (confidence high) — *Testing Strategy → Integration Tests* — The `last_updated`-only case's `differing_sections` unit test asserts only detection; the retired parity case asserted the CLI renders `last_updated` into the body. The new AC3 golden uses a `## Summary` change, so no surviving test proves the detect→render pipeline for that field. Point AC3 (or add a case) at a `last_updated`-only change.
- 🔵 minor (confidence medium) — *Phase 2, Change 4* — Deleting `a_failing_renderer_downgrades_to_unrenderable_with_raw_values` removes the only `render_dossier` test with non-empty sections. Surviving Renderable tests use `Vec::new()` and assert only `status: renderable`; deleting the `push_str(&render(section))` loop body would fail no test. Give one surviving test a section and assert the body lands under the header.
- 🔵 minor (confidence medium) — *Phase 2, Change 2* — The trailing-newline guard's true branch is never taken by any proposed test — dead to the suite. Drop it or add a test feeding a non-terminated body.
- 🔵 suggestion (confidence medium) — *Phase 2, Change 7* — `assert_eq!(stdout, EXPECTED)` on deliberately-unfrozen `similar` output while `similar = "2"` is caret-pinned is brittle; a minor bump reformatting hunks would red CI for a non-regression. Pin tightly, or lean on the structural `@@`/`-`/`+` assertions.

### Correctness

**Summary**: The plan's high-risk assertions about `similar`'s exact output are all verifiably correct against `similar` 2.7.0: the `@@ -1 +1 @@` / `@@ -0,0 +1 @@` / `@@ -1 +0,0 @@` hunk headers, the absence of the `\ No newline` marker for newline-terminated fields, the empty-body-for-identical case, and the byte-exact `EXPECTED` golden all hold. The infallible-render migration is genuinely lossless — every removed `DiffUnavailable` failure mode ceases to exist once the subprocess, tempfiles, and time cap are gone, and the surviving `Unrenderable` trigger (`local_unreadable`) plus its `raw_sections`-free path are preserved correctly. No correctness defect that would produce wrong output for any production input; only a minor inaccuracy in the stated rationale for a defensive guard.

Verification against `similar` 2.7.0 source:
- Hunk-header numbering (`UnifiedDiffHunkRange::Display`): `len == 1` prints just the start; `len == 0` prints `beginning-1,0`. Reproduces GNU exactly.
- Empty string tokenizes to zero lines (not one empty line) — confirms the `-0,0`/`+0,0` framing.
- `from_lines`/`diff_lines` always sets `newline_terminated: true`, making `missing_newline()` always false for `\n`-terminated fields — the `\ No newline` marker is unreachable.
- Identical sides → empty body, proven by `similar`'s own `test_empty_unified_diff`.
- The `EXPECTED` golden matches byte-for-byte.
- Every `DiffUnavailable` trigger ceases to exist once the subprocess is gone; `raw_sections`' only caller was the removed `Err` arm.

**Strengths**:
- Hunk-header numbering claims are exactly right.
- The "feed fields verbatim, do not re-append `\n`" insight is load-bearing and correct.
- The `\ No newline` marker is provably unreachable for production inputs.
- The infallible migration removes no real failure signal.
- Empty-string handling is correct at the tokenizer level.

**Findings**:
- 🔵 minor (confidence medium) — *Phase 2, Change 2 (guard rationale)* — The plan justifies keeping the guard as protection "against a future non-terminated field". This is inaccurate: because `from_lines` hard-codes `newline_terminated=true`, a non-terminated field triggers `similar`'s `missing_newline()` path, emitting a `\ No newline at end of file` line — a body still ending in `\n`. The guard still never fires; the real (non-production) consequence would be a leaked `\ No newline` marker, not a missing trailing newline. Reword the rationale to a pure defensive no-op, and drop the "guards a non-terminated field" framing.

### Compatibility

**Summary**: From a compatibility standpoint the plan is careful and well-grounded: it preserves the frozen header/blank-line framing byte-identically, correctly establishes that no live consumer parses the un-frozen diff body (only the unimplemented 0213 flow), and verifies the `similar` dependency is licence-clean and dependency-free with an explicit `cargo deny`/`cargo tree` gate. The removals (`DiffUnavailable`, work-cli's orphaned `bash-parity` feature, the reworded unrenderable message) were each checked against their consumers and are safe. The only residual concerns are a dependency-pin tightness question and a stale forward reference in the 0213 research.

**Strengths**:
- Framing contract provably preserved: the new `render` reproduces the current header format string and trailing-blank logic verbatim, pinned byte-exact by the identical-sides test.
- The body-format change is correctly scoped as safe — no runtime reader parses the body, only the unimplemented 0213 flow.
- Dependency addition is licence-clean and additive: `similar` is Apache-2.0 (already in the allow-list), its default `text` feature pulls no transitive packages, and the plan bakes in `cargo tree` + `cargo deny check licenses` gates.
- The reworded `unrenderable_header` prose is compatibility-safe: only the machine-facing `status: unrenderable` marker is asserted, never the prose.
- Removal of work-cli's own `bash-parity` feature is verified — `cli_diff_parity.rs` was its sole consumer, and `--all-features` tolerates a feature ceasing to exist.

**Findings**:
- 🔵 minor (confidence medium) — *Phase 2, Section 1* — `similar = "2"` (caret) yet the inline `EXPECTED` golden asserts exact hunk bytes. The workspace exact-pins output-sensitive crates (clap, tar, jj-lib, octocrab) and reserves carets for output-insensitive ones. A minor bump altering `similar`'s formatting would red the golden — bounded to deliberate updates, CI-caught. Exact-pin with a rationale, or note the golden intentionally guards `similar`'s output.
- 🔵 suggestion (confidence low) — *Migration Notes / What We're NOT Doing* — The rename + resignature leaves the 0213 research referencing `work_adapters::diff_shellout::render` and a `---/+++` body. Add one Migration Notes line recording both are superseded so the future consumer adopts the new path and format deliberately.

### Standards

**Summary**: The plan is strongly convention-aware: its workspace-dependency declaration style, the rationale comment on the new `similar` pin, the module rename, and the removal of the stale `0212` reference from the baseline-guard message all match established conventions, and its pup rule follows the shipped `denied`-list shape used by the other crate-wide zero-spawn rules. The one material gap is that Phase 3 adds a new `pup.ron` rule without the probe-pair that every other shipped rule carries and that `tasks/README.md` documents as the only guard. A secondary deviation is an explanatory inline comment in the new AC3 CLI test.

**Strengths**:
- The new `similar` workspace-dependency comment matches the established `cli/Cargo.toml` rationale-comment convention.
- `similar = { workspace = true }` and `similar = "2"` follow the existing declaration style precisely.
- The `diff_shellout` → `diff` rename improves navigability and drops the retired-mechanism suffix.
- Phase 1 removes the stale `0212` work-item reference from the baseline-guard message.
- The `work_adapters_is_zero_spawn` rule follows the shipped `denied`-list shape and crate-root regex convention.

**Findings**:
- 🟡 major (confidence high) — *Phase 3 — Changes Required / Success Criteria* — The new rule registers no probe pair in `tests/integration/pup/test_import_rule.py`, relying on a one-off manual step. Every other shipped rule has a violation/compliant pair driven against the real `pup.ron`, and `tasks/README.md` documents this as required — a mistyped or deleted rule is otherwise silent. Add a probe pair (synthetic `work-adapters` crate importing `std::process::Command`, asserting `is denied` + rule name, plus a compliant control), mirroring the `remote-projection` probe. The sibling `work_adapters_filesystem_reads_in_process` lacks one too — close both.
- 🔵 minor (confidence high) — *Phase 2, Section 7* — The AC3 test carries `// Two sections; only "## Summary" differs …` above the two `fs::write` calls that already express it. `cli_diff.rs` constructs identical fixtures without narration. Drop the comment.
- 🔵 suggestion (confidence high) — *Phase 3, Success Criteria* — The criterion hedges "confirm the exact task name via `mise tasks`". `pup:check` is declared in `mise.toml` and documented in `tasks/README.md`. Replace with the confirmed instruction and drop the caveat.

## Re-Review (Pass 2) — 2026-08-30

**Verdict:** APPROVE

All six lenses re-ran against the edited plan with their prior findings supplied for delta assessment. Every one of the eleven prior findings is **Resolved** — both majors closed. Correctness verified the guard removal is byte-identical for all production inputs and that the new `last_updated`-only case genuinely produces a differing section (`differing_sections` uses `trim_lines`, not the `IGNORE_KEYS`-stripping filter); Compatibility confirmed the exact `=2.7.0` pin has no other workspace consumer and needs no `deny.toml` edit; Standards confirmed the probe-pair snippet matches the real `test_import_rule.py` helpers and that every task name is real. The fresh findings are all minor/suggestion and were addressed in a follow-up edit pass, so the plan is ready to implement.

### Previously Identified Issues

- 🟡 **Standards**: New `pup.ron` rule omits the probe-pair registration — **Resolved** (Phase 3 §2 adds the pair, reusing `_write_shared_crate_probe`/`_SPAWN_VIOLATION`/`_PROJECTION_COMPLIANT`, asserting `is denied` + rule name).
- 🟡 **Architecture**: Enforcement diverges from harness precedent; import rule has an inline-spawn gap — **Resolved** (Phase 3 now separates three complementary guards and adds the runtime harness).
- 🔵 **Code Quality**: `diff.rs` docstring still names `diff_shellout` — **Resolved** (Phase 2 §3 diff added).
- 🔵 **Test Coverage / Code Quality / Correctness**: Trailing-newline guard dead/untested/mis-rationalised — **Resolved** (guard removed; rationale corrected).
- 🔵 **Test Coverage**: `last_updated`-only render assertion lost — **Resolved** (dedicated CLI case added; Testing Strategy no longer conflates detection with rendering).
- 🔵 **Test Coverage**: Sync Renderable body-concatenation unasserted — **Resolved** (one surviving Renderable test gets a non-empty section asserting the body).
- 🔵 **Compatibility / Test Coverage**: Caret pin vs exact-byte golden — **Resolved** (exact-pinned `=2.7.0` with rationale).
- 🔵 **Architecture**: Pure renderer's crate placement implicit — **Resolved** (tradeoff stated, rejected alternative named).
- 🔵 **Standards**: AC3 inline comment — **Resolved** (removed).
- 🔵 **Standards**: `pup:check` hedge — **Resolved** (stated decisively).
- 🔵 **Code Quality**: `similar` dep comment restates `deny.toml` — **Resolved** (trimmed).
- 🔵 **Architecture / Compatibility**: 0213 stale references — **Resolved** (Migration Notes breadcrumb added).

### New Issues Introduced (and addressed in the follow-up edit)

- 🔵 **Architecture / Correctness / Test Coverage**: Harness under-specified — the "modelled on `corpus-adapters`" framing wrongly implied a fixture `[[bin]]` and degradation-comparison that do not port to a pure renderer, and the harness covered only two render paths. **Addressed** — Phase 3 §3 rewritten as an explicit in-process marker-only tripwire (no fixture binary, tripwire dir holds an executable named `diff`, nextest per-test isolation noted), with the `author.rs`/`filesystem.rs` residual boundary stated as deliberate and the `PATH`-resolved-only limitation documented.
- 🔵 **Code Quality / Test Coverage**: Redundant `contains("@@ -1 +1 @@")` in the AC3 exact-match test — **Addressed** (assertion dropped; `assert_eq!(stdout, EXPECTED)` covers it).
- 🔵 **Standards**: Residual hedge on `test:integration:pup` — **Addressed** (qualifier dropped; confirmed a real task).
- 🔵 **Architecture**: Crate-wide scope is the workspace's first — **Addressed** (Phase 3 §1 notes it as a deliberate tightening that also spans in-`src` test modules).
- 🔵 **Correctness**: Golden hunk headers load-bearing but hand-reconstructed — **Addressed** (Phase 2 §7 instructs transcribing `EXPECTED` from a real `render` run).
- 🔵 **Compatibility**: Optional MSRV note on the exact pin — no change (agent flagged as "no change needed"; `similar`'s MSRV sits below the 1.90 floor).

### Assessment

The plan is sound and ready for implementation. The two majors that kept the first pass at COMMENT are closed, the load-bearing `similar` and infallible-migration claims are independently verified against source, and the newly-added Phase 3 enforcement surface is now specified concretely enough to implement without over-building.
