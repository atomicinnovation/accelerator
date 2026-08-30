---
type: "plan-review"
id: "2026-08-28-0174-empty-scripts-retire-shell-tooling-review-1"
title: "Plan Review: Empty scripts/ and Retire Shell Tooling and CI Guards"
date: "2026-08-28T08:13:06+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
parent: "plan:2026-08-28-0174-empty-scripts-retire-shell-tooling"
target: "plan:2026-08-28-0174-empty-scripts-retire-shell-tooling"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["correctness", "architecture", "code-quality", "test-coverage", "safety", "portability", "compatibility"]
review_number: 1
review_pass: 3
tags: ["shell", "tooling", "ci", "cleanup", "scripts"]
last_updated: "2026-08-28T12:44:34+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Empty scripts/ and Retire Shell Tooling and CI Guards

**Verdict:** REVISE

The plan is arithmetically rigorous where it matters most: correctness verified the two lockstep couplings drain exactly (`SHELL_LIBRARIES` 13 across P2/P4/P6/P7/P10, config floor 14→0 across P2/P4/P6/P7) and the shell source-dependency graph never orphans a live consumer. Three concrete defects nonetheless break the plan's own central invariant — that every intermediate commit is independently `mise run`-green — and a fourth removes CI enforcement of the two surviving shell files without a replacement lane. The remaining majors cluster on two under-specified areas: the Phase 9 awk→Python regex translation (fidelity risk on the only guard enforcing the bash-3.2 floor) and the Phase 7 nine-guard port (coverage-parity and binary-provisioning risk on the 768-line conformance guard).

### Cross-Cutting Themes

- **Write-back duplication in Phase 1** (flagged by: architecture, code-quality) — the prose says `run_link_external_id` "lifts `link_external_id`'s body verbatim" but the code snippet *calls* `link_external_id(...)`, which is today a `LocalAuthor` trait method. Read literally, an implementer duplicates the ~20-line parse/`Mapping::set`/`AtomicWrite` sequence into `main.rs` — creating two copies with no drift guard, at the very moment the plan deletes every drift oracle.
- **Phase 10 removes shell CI enforcement without re-homing it** (flagged by: safety, compatibility) — deleting `check-scripts` drops the only CI lane running `scripts:check`. The rescoped shfmt/ShellCheck/bashisms task survives but nothing invokes it in CI, so a bash-4 construct in `bin/accelerator` could ship undetected — the exact regression class ADR-0049 exists to catch.
- **Phase 9 bashisms translation fidelity** (flagged by: portability, correctness) — the plan singles out `[[:alnum:]]→[A-Za-z0-9]` as the one hazard, but the awk source uses four POSIX classes, unanchored `~` search semantics, and locale-sensitive byte reading. Any one mistranslation silently weakens the sole guard enforcing the 3.2 floor on the survivors.
- **Phase 7 conformance-guard port under-specified** (flagged by: architecture, test-coverage, safety, code-quality) — the 768-line guard drives the compiled corpus validator (needs the `ACCELERATOR_CORPUS_BIN` overlay the old integration lane supplied), is re-homed into a unit lane whose siblings have no binary dependency, is specified only by a one-line-per-guard table with no assertion-parity inventory, and is deleted in the same commit its port appears (no differential window).

### Tradeoff Analysis

- **Commit granularity vs differential verification**: The plan's small-commit strategy keeps the floor/frozenset couplings honest, but for the highest-blast-radius guard (Phase 7 conformance) safety argues for the opposite — land the pytest port *while keeping the shell guard live* for one green commit, then delete in the next. The two goals conflict only for that one guard; elsewhere the granularity is correct.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Correctness**: Phase 10 removes `shell_sources()` but leaves a live import/test in `test_bootstrap_coverage.py`
  **Location**: Phase 10 §1 vs Phase 9 §3
  `test_bootstrap_coverage.py:13` imports `shell_sources` and `:27` calls it (`assert _BOOTSTRAP in shell_sources()`). Phase 9 rewrites only the bashisms-discovery test, leaving the shfmt/shellcheck one bound; Phase 10's edit list omits the file entirely. The commit fails at collection time — a red intermediate commit, the exact failure the granularity strategy exists to prevent.

- 🟡 **Architecture / Code Quality**: Phase 1 write-back is duplicated, not shared
  **Location**: Phase 1 §2 (Dispatch and lifted writeback)
  The snippet calls `link_external_id(path, &external_id)` but that symbol is a `LocalAuthor` trait method (`sync_author.rs:139`), uncallable as a free function without extraction. "Lifts verbatim" invites copying the write-back into `main.rs`, leaving two divergent copies with no drift oracle (all three are being deleted this story). The lift also depends on module-private `fn failed` and six `use` imports `main.rs` lacks — pushing domain logic into the thin clap-dispatch layer.

- 🟡 **Test Coverage**: Deleting the `parity.rs` bash case drops the only coverage of two `infer` behaviours
  **Location**: Phase 4 (doc-type-inference cutover)
  `doc_type_inference_matches_the_bash_matcher` is the sole test exercising the exact-length tie-break (first entry wins) and interior-segment matching. The native `doc_type.rs` unit tests cover longest-wins, whole-segment, and no-match, but neither a tie nor an interior segment. Mutating `>` to `>=` or removing the `embedded` branch would pass the suite after deletion. The plan asserts `infer` is "already natively unit-tested".

- 🟡 **Test Coverage**: The nine-guard port has no assertion-parity inventory
  **Location**: Phase 7 (Nine-guard Python port)
  The port of the 768-line conformance guard and its `:407-765` design appendix is specified only by a one-line-per-guard table plus generic "fail on negatives, pass on conforming". Individual `assert_*` blocks can be silently dropped while coarse criteria still go green.

- 🟡 **Architecture**: Phase 7 re-homes a binary-driving guard into a lane that may not provision the binary
  **Location**: Phase 7 §1
  `test-skill-frontmatter-conformance` drives `accelerator corpus frontmatter validate`, needing the `ACCELERATOR_CORPUS_BIN` overlay and launcher build the old `test:integration:config` lane supplied. `tests/unit/tasks/` siblings are pure-Python scanners with no binary dependency — while Phase 8 correctly routes its binary-driving hook guards to `tests/integration/hooks/`. Inconsistent, and possibly non-functional in the unit lane.

- 🟡 **Test Coverage**: The new `link_external_id` test pins only the scalar, not round-trip or error paths
  **Location**: Phase 1 (Success Criteria)
  The lifted write-back re-renders the whole document through `document::render` + `AtomicWrite`. The test asserts only that the `external_id` scalar is written — not that surrounding frontmatter and body survive the round-trip, and not the two error paths (non-mapping frontmatter, unreadable file) that must yield `ExitCode::FAILURE`. A swallowed error returning SUCCESS or a render that reorders keys would pass.

- 🟡 **Safety / Compatibility**: Phase 10 deletes the only CI lane enforcing the shell survivors
  **Location**: Phase 10 §4
  `check-scripts` is the sole CI job running `mise run scripts:check` (rescoped shfmt + ShellCheck + Python bashisms over the two survivors). No surviving component job invokes it, and the project is CI-only (no pre-commit hooks). A bash-4 construct or format drift in `bin/accelerator` — the launcher every skill invokes — could merge and release undetected. The success criteria only `grep -c check-scripts` and list-assert; neither runs the linters in CI.

- 🟡 **Portability**: Phase 9 names one POSIX-class hazard, but four classes need translating
  **Location**: Phase 9 (pattern-translation table)
  The awk source uses `[[:alnum:]]`, `[[:alpha:]]` (nameref, letters-only), `[[:digit:]]`, and `[[:space:]]`. Python `re` supports none as bracket expressions; the shorthands mistranslate (`\w`/`\d`/`\s` admit `_`/Unicode digits/Unicode whitespace the C-locale awk does not). This scanner is the only guard enforcing the 3.2 floor on the survivors — shfmt/ShellCheck do not check bash version.

- 🟡 **Portability**: `re.search` vs `re.match` and scan granularity are unspecified
  **Location**: Phase 9
  awk's `~` is an unanchored per-record search. The plan never states `re.search` (not `match`/`fullmatch`) applied line-by-line. `re.match` anchors at column zero (no pattern anchors there → every line passes); whole-file text without `re.MULTILINE` changes the `(^|…)` / `$` anchors. Wrong mode fails the guard open.

- 🟡 **Portability**: Locale-default file decoding can crash the lint
  **Location**: Phase 9 (file reading)
  `Path.read_text()` with no explicit encoding uses the locale-default codec. The scanned scripts contain non-ASCII (em-dashes in headers); under a forced `LANG=C`/`LC_ALL=C` (documented elsewhere in this repo's tooling) Python's default codec resolves to ASCII and raises `UnicodeDecodeError`, aborting the lint on a valid file.

#### Minor

- 🔵 **Correctness**: `walk_files` is a live shared dependency — the "(if unused elsewhere)" removal is a trap
  **Location**: Phase 10 §1
  `claude_coupling.py:52` and `test_python_coverage.py:70` both call `walk_files`. An implementer following the parenthetical literally deletes it, breaking two live guards. Only `shell_sources`, `_keep`, and `_EXTRA_SHELL_SOURCES` are safe to drop.

- 🔵 **Test Coverage**: Phase 2 drops the config-path-key drift oracle with no native replacement named
  **Location**: Phase 2 (Config source-chain deletion)
  `every_config_path_key_exists_in_the_config_schema` pins every `DocTypeKey::config_path_key()` against the schema; the surviving case only checks registration counts. After bash is gone, a `config_path_key()` renamed out of step would silently drop a doc type with no test.

- 🔵 **Test Coverage**: Phase 8's launcher-dispatch smoke drops the boundary-header-absent assertion
  **Location**: Phase 8 §1
  `test-vcs-detect.sh:234-238` asserts absence of the three prohibition phrases *and* the `WORKSPACE BOUNDARY DETECTED` header. The port lists only the three phrases.

- 🔵 **Safety**: Phase 8's golden-retention gate runs a `bash-parity`-gated test that the default lane skips
  **Location**: Phase 8 (Success Criteria)
  `detect_goldens.rs` is `#![cfg(feature = "bash-parity")]`; the "still reads its four goldens and passes: `mise run test:unit:cli`" gate likely does not execute in the default lane, making the safeguard illusory.

- 🔵 **Safety**: Phase 7 deletes nine guards in the same commit their ports appear (no differential window)
  **Location**: Phase 7 (Delete step + Atomic bundle)
  Equivalence rests entirely on fixture carry-over; the highest-blast-radius conformance guard has no green intermediate state proving the port catches everything the shell original did.

- 🔵 **Architecture**: `templates-schema.tsv` lands in `tests/` but its consumer is a `#[cfg(test)]` src-tree unit test
  **Location**: Phase 3 §1
  Relocating to `cli/corpus/tests/` forces an `include_str!("../../tests/...")` traversal across the src→root→tests boundary. Co-locating beside the module (`cli/corpus/src/frontmatter_validation/`) sits the data with its reader.

- 🔵 **Architecture**: Phase 10's survivor set is dual-represented (constant + README) with a prose-parsing equality test
  **Location**: Phase 10 §5
  Asserting the constant equals a README enumeration couples the test to README formatting and leaves authority ambiguous. Name the constant authoritative; have the test tolerate prose (assert each path appears as a backticked token).

- 🔵 **Code Quality**: The shared frontmatter-rules Python module has no name or home specified
  **Location**: Phase 7 §2
  The DRY linchpin both ports consume is described only as "a Python module". Unspecified name/placement risks a convention-violating name or landing shared logic inside a test file. Name it (e.g. `tasks/lint/frontmatter_rules.py`) mirroring the `skill_permissions.py` module-plus-test split.

- 🔵 **Code Quality**: No explicit red-green ordering for the guard ports
  **Location**: Phase 7 / Phase 1 (Success Criteria)
  The plan lists end-states and tests-in-bundle but never states failing-test-first. Risk: port the guard, then assert against the live tree, without ever observing the port fail — weakening the net that justifies small commits.

- 🔵 **Compatibility**: The `document::render` write-back may reposition/reformat frontmatter vs the retired byte-preserving bash edit
  **Location**: Phase 1 §2 (Frontmatter-safety note)
  `Mapping::set` appends the key and the YAML re-render can normalise untouched fields, producing spurious VCS churn in a cross-tool artefact. Not a functional break (it is the production sync path), but assert byte-identical neighbours in the test.

#### Suggestions

- 🔵 **Architecture**: Consider porting the guards (P7) before relocating `templates-schema.tsv` (P3)
  **Location**: Phase 3 vs Phase 7 ordering
  The current order forces a transient `scripts/`→`cli/` cross-boundary reach (option (b)) that exists only for the P3–P7 window and must be undone at P7. Reordering removes the repoint entirely; if kept, note the transient coupling is deliberate.

- 🔵 **Correctness**: Replace the blanket POSIX rule with a per-pattern class mapping
  **Location**: Phase 9 §1
  None of the eight patterns use a bare `[[:alnum:]]`; the mapfile boundary uses `[^[:alnum:]_]` and case-modification uses `[[:alnum:]_\[\]@*]` (both include `_`, so `\w` is exact there), while nameref uses `[[:alpha:]]` (letters only). Map per-pattern from the awk source verbatim.

- 🔵 **Correctness**: Prove the de-gated `bash-parity` cases pass in the default lane before the de-gate commit
  **Location**: Phase 4 / Phase 5
  These cases appear compiled-but-never-executed until de-gating (the zero-spawn lanes enable the feature but filter execution); the P5 case spawns the launcher. Run each standalone under the default-lane feature set first and record it as a precondition.

- 🔵 **Code Quality**: Resolve the `test.unit.templates` either/or in the plan
  **Location**: Phase 7 §4
  "repoint … or delete the task if the ports subsume it" leaves an ad-hoc decision for coding time. Determine whether the two ports fully cover the task and prescribe the outcome.

### Strengths

- ✅ The per-phase deletion tally reconciles exactly against the on-disk file set (28 `.sh` = 13 libraries + `lint-bashisms.sh` + 14 counted `test-*.sh`, `test-helpers.sh` excluded as a library) — verified by correctness.
- ✅ Source-dependency ordering is provably safe: the P2-deleted chain's only consumers die in the same commit; `test-helpers.sh` is genuinely the last leaf and is deleted last.
- ✅ The crate-boundary decision is correct — homing `link-external-id` in `work-cli` (which already owns the document/corpus/tracker stack) rather than the remote-only `jira-cli` avoids inverting the dependency direction and adds zero crate edges.
- ✅ The fail-closed transition in Phase 9 is correctly preserved: the retired shell script failed *open* on empty scope, and the `_EMPTY_SCOPE` wrapper guard keeps the Python task fail-closed.
- ✅ Rescope-not-delete is applied correctly to `extra_keys_mirror.rs`, `parity.rs`, and `doc_type_single_source.rs` — each surviving non-shell case is named and its imports pruned.
- ✅ Splitting the conformance guard's design-structure appendix into its own module cleanly separates a distinct concern, and homing the eight pure content-scanners in `tests/unit/tasks/` matches that directory's de-facto shape.
- ✅ The retained goldens are honoured: Phase 8 keeps `hooks/test-fixtures/vcs-detect/*.json` and deletes only the already-dead `regenerate.sh`.
- ✅ The `check-scripts` `needs:` edge is genuinely the sole reference (main.yml:147,163,587), so the prerelease gate graph has no other dangling dependency after removal.

### Recommended Changes

1. **Add `test_bootstrap_coverage.py` to Phase 10's atomic bundle** (addresses: Phase 10 orphans `shell_sources()` consumer). Repoint the shfmt/shellcheck-discovery assertion onto `SURVIVING_SHELL_SOURCES`; ideally have Phase 9 target the task's exposed scan set so the seam does not move twice.
2. **State that `link_external_id` is promoted to a shared free function** (addresses: Phase 1 duplication). Extract it in `sync_author.rs` alongside `failed`; the trait method and `main.rs`'s thin `run_link_external_id` both delegate. One source of truth, no domain logic in the dispatch layer.
3. **Re-home the rescoped `scripts:check` into a surviving CI job before deleting `check-scripts`** (addresses: Phase 10 drops shell enforcement). Add a success-criterion asserting the survivors are shfmt/ShellCheck/bashisms-checked in *some* CI lane, not merely list-asserted.
4. **Specify the Phase 9 translation per-pattern and lane-safe** (addresses: four POSIX classes, search mode, locale decoding, per-pattern mapping). Translate every class to explicit ASCII ranges and compile with `re.ASCII`; specify `re.search` line-by-line over `splitlines()`; read with explicit `encoding="utf-8"`; add one golden fixture per pattern including an indented/mid-line and a near-miss case.
5. **Give Phase 7 an assertion-parity inventory and resolve its lane** (addresses: no parity inventory, binary provisioning, module naming, differential window). Enumerate each shell assertion→pytest assertion, carry every negative fixture; either home the binary-driving conformance guard in an integration lane or state how the unit lane provisions the corpus binary; name the shared rules module; land the port green while the shell guard is still live, then delete in the next commit.
6. **Strengthen the Phase 4 and Phase 1 tests** (addresses: `infer` edge coverage, `link_external_id` round-trip). Add native `doc_type.rs` tests for the exact-length tie and interior-segment match before deleting the bash case; extend the `link_external_id` test to assert full-document preservation plus at least one error-path (non-zero exit).
7. **Fix the smaller correctness traps** (addresses: `walk_files`, config-path-key oracle, de-gate preconditions, Phase 8 golden gate). Explicitly retain `walk_files` (naming its consumers); confirm or add a native config-path-key test; run the de-gated cases standalone first; replace the `bash-parity`-gated Phase 8 golden gate with one that actually executes.

## Per-Lens Results

### Correctness

**Summary**: The plan's core invariant — every intermediate commit independently green on a floor/frozenset mismatch — is arithmetically sound: the 13 `SHELL_LIBRARIES` entries drain exactly across P2(4)+P4(1)+P6(5)+P7(2)+P10(1), the 14 config-floor `test-*.sh` drain across P2(2)+P4(1)+P6(2)+P7(9), and the source-dependency graph never leaves a surviving consumer of a deleted library. The one real defect is a missed consumer in Phase 10: removing `shell_sources()` orphans an import and a test in `test_bootstrap_coverage.py` that Phase 9 leaves bound to it.

**Strengths**:
- Deletion tally reconciles exactly against the on-disk file set (28 `.sh` = 13 libraries + `lint-bashisms.sh` + 14 counted `test-*.sh`, `test-helpers.sh` excluded).
- Source-dependency ordering provably safe: P2 chain's only consumers deleted same-commit; `test-helpers.sh` deleted last.
- Phase 1's clean-lift claim verified against `sync_author.rs:139-161` — body touches only `path` and `external_id`.
- Fail-closed transition in Phase 9 correct: shell fails open on empty scope, `_EMPTY_SCOPE` wrapper raises.
- `call_site_migration.py` P1/P2 split validated: `config-common.sh` is an allowed substring, not a required call site.

**Findings**:
- 🟡 (major, high) Phase 10 §1 vs Phase 9 §3 — Phase 10 removes `shell_sources()` but `test_bootstrap_coverage.py:13,27` still imports/calls it; commit fails at collection. Add the file to P10's bundle, repointing onto `SURVIVING_SHELL_SOURCES`.
- 🔵 (minor, high) Phase 10 §1 — `walk_files` is live (`claude_coupling.py:52`, `test_python_coverage.py:70`); the "(if unused elsewhere)" instruction is a trap. Retain it explicitly.
- 🔵 (minor, medium) Phase 4/5 — de-gated `bash-parity` cases execute in the default lane for the first time; preconditions unproven (P5 spawns the launcher). Run standalone first.
- 🔵 (suggestion, medium) Phase 9 §1 — the blanket `[[:alnum:]]→[A-Za-z0-9]` rule mismatches the actual patterns (nameref uses `[[:alpha:]]`; two classes include `_`). Map per-pattern from the awk source.

### Architecture

**Summary**: Structurally rigorous — the two lockstep couplings are traced through an explicit per-phase drain tally that reconciles to zero, and the three ordering constraints are respected. The crate-boundary decision and the conformance-guard module split are sound. Two structural-integrity concerns stand out: a probable duplication of the write-back at the moment its drift oracles are deleted, and a binary-driving guard re-homed into a unit lane that may not provision the binary.

**Strengths**:
- Per-phase deletion tally reconciles exactly and keeps both couplings honest.
- Crate-boundary reasoning correct: `link-external-id` in `work-cli`, not `jira-cli`.
- Data files relocated to their consuming crate (corpus, design).
- Conformance appendix split into its own module separates a distinct concern.
- Eight content-scanning guards homed in `tests/unit/tasks/` matches that directory's shape.

**Findings**:
- 🟡 (major, medium) Phase 1 §2 — internally inconsistent: prose says "lifts verbatim", snippet calls a trait method. Promote to a shared free function.
- 🟡 (major, medium) Phase 7 §1 — conformance guard needs `ACCELERATOR_CORPUS_BIN`/launcher build; re-homed into a unit lane whose siblings have no binary dependency, inconsistent with Phase 8's integration routing.
- 🔵 (minor, medium) Phase 3 §1 — `templates-schema.tsv` to `cli/corpus/tests/` forces `include_str!("../../tests/...")` across boundaries; co-locate beside the `#[cfg(test)]` src consumer.
- 🔵 (suggestion, medium) Phase 3 vs 7 — relocating before the guards are ported forces a transient `scripts/`→`cli/` reach; consider P7 before P3.
- 🔵 (minor, medium) Phase 10 §5 — dual representation (constant + README) with a prose-parsing equality test; name the constant authoritative.

### Code Quality

**Summary**: Well-structured and DRY-conscious (shared frontmatter-rules module, rescope-not-delete preservation, splitting the 768-line appendix), and `run_link_external_id`'s error handling matches the established `run_*` pattern. The main concerns are the placement and potential duplication of the lifted write-back, an under-specified "verbatim" lift that depends on module-private symbols, an unnamed shared module, and no explicit red-green ordering.

**Strengths**:
- Conformance appendix split into a convention-compliant `test_design_structure.py`.
- `frontmatter-emission-rules.sh` re-expressed once as a shared module, not duplicated per port.
- Rescope-not-delete preserves the pure-Rust cases.
- `run_link_external_id` error handling (eprintln + `ExitCode::FAILURE`) matches siblings.
- Deliberate small-commit granularity with a per-phase tally.

**Findings**:
- 🟡 (major, medium) Phase 1 §2 — verbatim lift of a trait-method body risks two divergent copies of the write-back invariant. Extract-and-delegate.
- 🔵 (minor, high) Phase 1 §2 — the no-new-imports claim is crate-level, not module-level; the body needs private `fn failed` + six `use`s `main.rs` lacks. Home it in `sync_author.rs`.
- 🔵 (minor, medium) Phase 7 §2 — the shared frontmatter-rules module has no name/home; name it (e.g. `tasks/lint/frontmatter_rules.py`).
- 🔵 (minor, medium) Phase 7/1 — no stated red-green ordering; land the negative-fixture test red first.
- 🔵 (suggestion, high) Phase 7 §4 — resolve the `test.unit.templates` either/or in the plan.

### Test Coverage

**Summary**: Unusually disciplined about coverage preservation — rescopes rather than deletes the three drift-oracle files and gates every phase behind `mise run test:unit:cli` / `mise run` (workspace build, `--all-features`, so compiled-launcher and bash-parity cases actually run). The principal risks are localised: deleting the `parity.rs` bash-matcher drops the only coverage of two `infer` edge cases, the nine-guard port has no assertion-parity inventory, and the new `link_external_id` test only pins the written scalar.

**Strengths**:
- Success criteria use `mise run test:unit:cli` / `mise run`, and the plan flags (P5) the surviving case spawns the launcher — so "each phase green" is genuinely verifiable.
- Rescope-not-delete applied correctly to all three drift oracles.
- Per-guard P7 criteria demand negative-fail + conforming-pass + live-tree; `-self` meta-tests folded, not dropped.
- The vcs classification matrix is genuinely mirrored in `classify.rs`; `detect_goldens.rs` retained.

**Findings**:
- 🟡 (major, high) Phase 4 — deleting `doc_type_inference_matches_the_bash_matcher` drops the only coverage of the exact-length tie-break and interior-segment match. Add native `doc_type.rs` tests first.
- 🟡 (major, medium) Phase 7 — no assertion-by-assertion parity inventory for the 768-line guard; sub-assertions can be dropped silently. Enumerate and carry every negative fixture.
- 🟡 (major, medium) Phase 1 — the `link_external_id` test asserts only the scalar, not round-trip preservation or the two error paths. Extend it.
- 🔵 (minor, medium) Phase 2 — drops `every_config_path_key_exists_in_the_config_schema` with no native replacement named.
- 🔵 (minor, low) Phase 8 — the ported smoke drops the `WORKSPACE BOUNDARY DETECTED`-absent assertion.

### Safety

**Summary**: A low-criticality tooling cleanup whose principal safety property is per-commit greenness under two lockstep couplings, handled well: deletions sequenced after cutovers, `include_str!` relocations same-commit, `test-helpers.sh` deleted last, the bashisms fail-closed guard preserved. The one real hazard is guard-continuity: Phase 10 removes the sole CI job enforcing the survivors without re-homing that lane, and Phase 7 deletes nine guards in the same commit their ports appear.

**Strengths**:
- `test-helpers.sh` deletion-last verified against the live consumer set.
- The fail-open→fail-closed hazard explicitly identified and mitigated.
- The two `include_str!` couplings treated as hard same-commit constraints.
- `detect_goldens.rs` dependency honoured (keeps `*.json`, deletes only dead `regenerate.sh`).
- Each destructive phase independently revertible with a bare `mise run` gate.

**Findings**:
- 🔴/🟡 (major, high) Phase 10 §4 — deleting `check-scripts` leaves the survivors (incl. `bin/accelerator`'s 3.2 floor) with no automated CI enforcement and no release gate. Re-home `scripts:check` first.
- 🔵 (minor, medium) Phase 7 — nine guards deleted in the same commit their ports appear; no differential window for the conformance long pole. Keep the shell guards live for one green commit.
- 🔵 (minor, low) Phase 3 (b) — repointed shell guards could fail open on a mis-resolved relocated TSV. Fail-closed on an empty/unreadable file.
- 🔵 (minor, medium) Phase 8 — the golden-retention gate runs a `bash-parity`-gated test the default lane skips; the safeguard is illusory.

### Portability

**Summary**: Fundamentally a portability improvement — it retires a bash+awk+git subprocess (behaviour differs between BSD/one-true-awk and gawk; the no-args `git ls-files` path is blind in jj workspaces) for in-process Python scanning, and correctly preserves the bash-3.2 floor for the survivors. But the Phase 9 awk→Python translation is under-specified: the plan singles out `[[:alnum:]]` as the one hazard, when three more classes plus search/anchoring and file-encoding differences each carry silent-mistranslation risk.

**Strengths**:
- Replacing `bash lint-bashisms.sh` with in-process Python removes a real cross-platform hazard (awk dialect + jj-blind `git ls-files`).
- The 3.2 floor consciously preserved for the survivors (shfmt, ShellCheck, `.shellcheckrc`, editorconfig block, ADR-0049 documented, a can-fail test).
- The fail-closed `_EMPTY_SCOPE` guard explicitly carried into the Python task.

**Findings**:
- 🔴/🟡 (major, high) Phase 9 — four POSIX classes (`alnum`, `alpha`, `digit`, `space`), not one; `\w`/`\d`/`\s` mistranslate. Use explicit ASCII ranges + `re.ASCII`; golden fixture per pattern.
- 🟡 (major, medium) Phase 9 — awk `~` is unanchored search; specify `re.search` line-by-line, not `match`/`fullmatch`/whole-file.
- 🟡 (major, medium) Phase 9 — locale-default decoding crashes on non-ASCII under `LANG=C`; read with explicit `encoding="utf-8"`.
- 🔵 (minor, medium) Phase 9 — bracket-escape/word-boundary sub-patterns carry dialect subtleties; keep explicit complement form, add fixtures.
- 🔵 (minor, medium) Phase 9 — reproduce the naive comment-strip (`count=1`) and raw-line opt-out exactly, not an "improved" shell-aware version.

### Compatibility

**Summary**: The core contract changes are sound: the SKILL.md WF-4 writeback is repointed onto an additive `work link-external-id` whose positional order (path, external-id) and exit-code/stderr shape match the consuming flow, edited in the same atomic bundle so no consumer is left dangling. The subcommand lives in the `work-cli` binary crate (no public-API surface, zero new deps), and `work` is already a dispatched skill-bound token. The one material gap is a CI-graph integrity regression in Phase 10.

**Strengths**:
- The SKILL.md repoint is a same-commit bundle; source-chain deletion deferred to P2.
- The new positional order matches the retired `config_upsert_frontmatter_field <file> external_id <KEY>`.
- Contained to the `work-cli` binary crate — `public-api:check` not tripped.
- `work` already a dispatched token, so `dispatch-coherence` stays satisfied.
- The `check-scripts` `needs:` edge is genuinely the sole reference (main.yml:147,163,587).

**Findings**:
- 🔴/🟡 (major, high) Phase 10 §4 — deleting `check-scripts` drops the only lane running `scripts:check`; no other job runs it. Re-home into a surviving job and add a criterion that the linters actually run in CI.
- 🔵 (minor, low) Phase 1 §2 — `document::render` after `Mapping::set` may reposition the key / reformat neighbours vs the byte-preserving bash edit, causing spurious VCS churn. Assert byte-identical neighbours in the test.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-08-28

**Verdict:** REVISE (all pass-1 findings resolved; pass-2 surfaced new majors, now also addressed in edits — a light third verification is advisable)

All ten pass-1 majors are resolved by the pass-1 edits. Re-running the seven lenses against the revised plan surfaced a fresh set of majors — two of them consequences of the pass-1 edits themselves (the Phase 7 "integration lane" left unregistered; the Phase 10 CI re-home left mechanically under-specified), and three high-confidence correctness breaks exposed by a deeper analysis of the CLI test lane. A load-bearing cross-lens conflict was resolved by direct verification: `tasks/test/cli.py` runs `--all-features` (bash-parity ON) under `warnings = "deny"`, confirmed against the file. This refuted the safety/test-coverage "orphaned `detect_goldens.rs`" majors (the default lane does run it) and confirmed the correctness import-prune majors (bash-parity test files compile in the default lane, so any orphaned import reds the commit). All surviving pass-2 findings were then addressed in a second edit round.

### Previously Identified Issues (Pass 1)

- 🟡 **Correctness**: Phase 10 orphans `shell_sources()` consumer in `test_bootstrap_coverage.py` — **Resolved** (added to Phase 10 bundle; Phase 9 targets the task scan-set so the seam moves once).
- 🟡 **Architecture / Code Quality**: Phase 1 write-back duplicated — **Resolved** (promoted to a `pub(crate)` shared free function; trait method delegates).
- 🟡 **Test Coverage**: Phase 4 drops `infer` tie/interior-segment coverage — **Resolved** (native `doc_type.rs` tests mandated first).
- 🟡 **Test Coverage**: Phase 7 no assertion-parity inventory — **Resolved** (per-guard checklist; carry every negative fixture).
- 🟡 **Architecture**: Phase 7 binary-driving guard in a no-binary lane — **Resolved in principle** (routed to an integration lane) but the lane was left unregistered → new pass-2 major, now fixed.
- 🟡 **Test Coverage**: Phase 1 test pins only the scalar — **Resolved** (whole-file byte-identity + error path).
- 🟡 **Safety / Compatibility**: Phase 10 drops shell CI enforcement — **Resolved in principle** (re-home) but the fold mechanism was under-specified → new pass-2 major, now fixed.
- 🟡 **Portability** ×3 (Phase 9 POSIX classes, search mode, decoding) — **Resolved** (four numbered hazards, per-pattern ASCII mapping, `re.ASCII`, `split("\n")`, `encoding="utf-8"`).
- 🔵 Minors (walk_files trap, config-path-key oracle, boundary-header assertion, TSV placement, README dual-representation, module naming, red-green ordering) — **Resolved**.

### New Issues Introduced or Surfaced (Pass 2) — all addressed

- 🟡 **Correctness** (high): Phase 4 pruned only `tempdir`/`TempDir`, leaving five orphaned imports (`std::fs`, `Command`, `Path`/`PathBuf`, `require_file`, top-level `DocTypeKey`) → hard compile error under `--all-features` + `warnings=deny`. **Fixed** — full prune list.
- 🟡 **Correctness** (high): Phase 2 removes `Command`'s only user but did not prune it; Phase 5 still listed it. **Fixed** — prune in Phase 2, dropped from Phase 5.
- 🟡 **Correctness** (high): Phase 10 missed `test_format.py` (4 shfmt tests) and `test_lint.py::TestShellcheckTask` (3 tests) that patch `shell_sources`. **Fixed** — added to lockstep.
- 🟡 **Architecture** (medium): Phase 7 replacement integration lane never registered (task/mise/CI). **Fixed** — concrete `@task` + overlay + `build:cli:dev` + roll-up + CI wiring.
- 🟡 **Safety / Compatibility / Architecture** (converged): Phase 10 fold mechanism under-specified. **Fixed** — explicit `run: mise run scripts:check` step (not a `depends` edge), component boundary preserved.
- 🟡 **Compatibility** (medium): removing `check-scripts` may hang PRs if it is a required branch-protection check. **Fixed** — manual handoff step.
- ❌ **Safety / Test Coverage**: "orphaned `detect_goldens.rs`" (medium) — **Refuted** by direct verification of `tasks/test/cli.py --all-features`; the pass-1 Phase 8 edit that assumed the opposite was reverted.
- 🔵 Minors (two-commit window framing, config-path-key exhaustiveness, `[[:space:]]` narrowing, `splitlines` vs `split("\n")`, whole-file byte-identity, exec-bit survivor assertion, `pub(crate)` + `main.rs` imports) — **Addressed**.

### Assessment

The plan is now substantively sound: every pass-1 finding is closed, the false "default lane skips bash-parity" premise is corrected throughout, and the import-prune completeness that per-commit greenness depends on is now enumerated per phase. The two edit rounds demonstrate the plan's own thesis — small changes to a lockstep-coupled plan surface follow-on breaks — so the pass-2 edits (especially the Phase 7 lane registration and Phase 10 CI fold) warrant one light verification read before implementation, but no structural rework remains.

---
*Re-review generated by /accelerator:review-plan*

## Re-Review (Pass 3) — 2026-08-28

**Verdict:** APPROVE (targeted correctness + architecture verification of the pass-2 edits; the single surviving major was fixed in place)

A scoped pass — only correctness and architecture, aimed at the two areas the pass-2 edits touched most (the Rust import-prune completeness under `--all-features` + `warnings=deny`, and the Phase 7 lane / Phase 10 CI re-home structure). Both lenses verified the edits against the actual sources. The Rust prune lists are exhaustive and correctly scoped (no orphan left, nothing over-pruned), Phase 1's `pub(crate)` promotion and `main.rs` symbol availability check out (`main.rs` already imports `Path`; `tracker::ExternalId` is fully-qualified), and the Phase 9→10 bootstrap-coverage seam is correctly split. One narrow break remained and was fixed in place.

### Previously Identified Issues (Pass 2)

- 🟡 **Correctness** ×3 (Phase 4/2/5 import prunes, Phase 10 test mocks) — **Verified resolved** for the Rust prunes; the Phase 10 test-mock rewrite was **partially resolved** (see below).
- 🟡 **Architecture / Safety / Compatibility** (Phase 7 lane registration, Phase 10 CI fold mechanism, branch-protection) — **Verified resolved**: the new `@task` matches the retired `config` lane's overlay + `build:cli:dev` wiring; the explicit `run:` CI step preserves the component boundary and the prerelease gate; `check-scripts` removal is fully traced (`:147,163,587`, no `release` edge).
- 🟡 **Correctness / Portability** (Phase 1 promotion, Phase 9 translation) — **Verified resolved**.

### New Issues Introduced (Pass 3) — addressed

- 🟡 **Correctness** (medium): Phase 10 §2's `test_lint.py` rewrite named only `TestShellcheckTask`, but `TestBashismsTask` (`test_raises_on_findings`, `test_raises_on_empty_source_set`) shares the same `patch.object(lint, "shell_sources", …)` idiom and would `AttributeError` when `shell_sources` is removed. **Fixed** — widened the target to both classes plus a grep-for-the-full-set instruction so no patch site is missed.
- 🔵 **Architecture** (suggestion): Phase 7 implied a separate CI `test-integration` edit that does not exist (roll-up membership *is* the CI wiring). **Fixed** — reworded to the single mise.toml roll-up action.
- 🔵 **Architecture** (minor): folding the shell lane into `check-build-system` collapses its CI status signal. **Addressed** — name the step ("Run script checks") for log attribution; the branch-protection handoff already covers the tradeoff.

### Assessment

The plan is approved. Three passes produced monotonically shrinking findings (10 majors → 6 new majors → 1 narrow major), each round's residue a consequence of the prior round's edits rather than a fresh structural flaw — the expected signature of a lockstep-coupled retirement plan converging. Every correctness break that per-commit greenness depends on is now enumerated per phase and verified against source; both structural re-homings (Phase 7 lane, Phase 10 CI) are verified to match the actual task-tree and workflow shape. The pass-3 fixes are low-risk (a test-enumeration widening backed by a grep instruction, and wording clarifications). No further review pass is warranted before implementation.

---
*Re-review generated by /accelerator:review-plan*
