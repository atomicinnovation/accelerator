---
type: "plan-review"
id: "2026-09-23-0241-migrate-false-dirty-tree-detection-review-1"
title: "Plan Review: False Dirty-Tree Detection on jj Repositories Implementation Plan"
date: "2026-09-23T21:48:45+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-23-0241-migrate-false-dirty-tree-detection"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "correctness", "test-coverage", "code-quality", "compatibility", "safety", "usability", "security"]
review_number: 1
review_pass: 2
tags: ["migration", "vcs", "jj", "preflight"]
last_updated: "2026-09-23T22:51:08+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: False Dirty-Tree Detection on jj Repositories Implementation Plan

**Verdict:** REVISE

The core design is sound. Deriving the jj run base from `@`'s sorted parent
ids is stable across snapshots and `jj describe @`. All three consumers pick
up the fixes through the single `working_copy_diff` seam. Pure resolution
(`excludes_file_path`, `MaxNewFileSize`) is kept apart from environment I/O.
The plan needs revision in four places:

- the new jj config and excludes errors feed the fail-open pre-flight, and
  the consumer tests cannot tell an error from a clean tree;
- the pre-upgrade recovery scenario and its `CHANGELOG.md` promise do not
  hold against the current decisions-file validation and m0007 semantics;
- the limit read skips jj's conditional-scope resolution;
- a sort expectation, an error `Display` and part of the rename are
  internally inconsistent.

### Cross-Cutting Themes

- **New scan errors feed the fail-open pre-flight** (flagged by: safety,
  test-coverage, usability, security, code-quality, compatibility) — Phases 3
  and 4 add new ways for `working_copy_diff` to error: an invalid limit,
  malformed TOML in any jj layer, an unreadable excludes file, or a gix
  interpolation error. `VcsDirtyPathScanner` treats any error as a clean tree
  (`dirty_path_scanner.rs:36-44`, verified). The migrate pre-flight then
  truncates the manifest, and by default the warning is invisible without
  `ACCELERATOR_LOG`. The positive consumer tests ("exit 0", "path not
  listed") also pass when the computation errors.
- **Pre-upgrade recovery does not work as written** (flagged by: correctness,
  usability, safety) — Validation fails closed before apply. A decisions file
  covering only reference 1 of 2 is rejected with `missing a decision for
  position 2` (`decisions_file.rs:80-128`, verified), so assertion 3 cannot
  pass. Force also resets the manifest to `[]` (`preflight.rs:71-75`,
  verified). m0007 never rewrites files the old run already migrated, so they
  stay unowned and the next force-free resume is refused.
- **Conditional `--when` / `[[--scope]]` config is not resolved** (flagged
  by: compatibility, correctness) — the raw `StackedConfig` read misses scoped
  limits and applies other repositories' `conf.d` files. This breaks
  requirement 4 parity silently.
- **Byte-sort expectation is wrong** (flagged by: correctness, test-coverage)
  — `["meta/a.md", ".accelerator/b"]` is not byte-sorted, since `.` < `m`.
- **The rename stops halfway** (flagged by: code-quality, architecture) —
  `run_id`, `write_run_id` and `base_revision_matches` survive next to
  `run_base`.
- **`core.excludesFile` via `trusted_path` differs from jj-cli** (flagged by:
  compatibility, correctness, security, code-quality) — jj-cli reads the raw
  string and expands only `~/`. `trusted_path` adds trust filtering and
  interpolation errors that abort the scan. Security prefers the trust
  filter; parity prefers the raw read (see Tradeoff Analysis).
- **Run base and dirty scan are two loads** (flagged by: architecture,
  correctness) — "one view for base and diff" is asserted but not enforced;
  `ctx.run_base()` is read before the lock.
- **Duplicate jj config-stack builders** (flagged by: code-quality,
  architecture) — `jj_user_name` keeps its own system+user stack, and its
  "disproportionate machinery" doc becomes false.

### Tradeoff Analysis

- **Parity vs trust (the `core.excludesFile` read)**: jj-cli honours the raw
  value regardless of gix trust, so parity says read the raw string. Security
  says keep gix's ownership trust so an untrusted repo-local config cannot
  hide changes. Recommendation: match jj-cli's raw read, because jj itself
  honours the value and the guard should agree with `jj status`. Record the
  accepted risk in the plan.
- **Parity vs fail-closed (invalid limit)**: parity with `jj status` would
  mean erroring. Safety says fall back to `u64::MAX` / empty ignores plus a
  `warn`, which over-reports dirt. Recommendation: fall back and warn. `jj
  status` itself fails loudly on an invalid limit, so the user already sees
  the problem, and the fallback avoids the fail-open branch without changing
  the out-of-scope policy.

### Findings

#### Major

- 🟡 **Safety / Test Coverage / Usability / Security**: New config and
  excludes errors widen the fail-open path, and the tests cannot detect it
  **Location**: Phase 3 §3, Phase 4 §§1-3 and the invalid-limit consumer
  test; What We're NOT Doing
  The invalid-limit test requires migrate to "exit 0, proceed" over an
  unowned `meta/a.md`. The positive exclude and size consumer tests pass even
  if `base_ignores` or `resolve` errors.
- 🟡 **Correctness**: A decisions file answering only the first of two
  prompts is rejected, so the run never reaches the second stall
  **Location**: Phase 2 §5, pre-upgrade stall assertions 3-4
  `decisions_file::validate` fails closed before any migration is applied.
- 🟡 **Correctness / Usability / Safety**: After one forced run, the next
  resume is still refused over the pre-upgrade output
  **Location**: Phase 2 §6 `CHANGELOG.md`; Migration Notes
  Force resets the manifest, and m0007 skips files that are already fenced,
  so the old run's output is never recorded. The recovery text is also not
  copy-pasteable, and it does not tell users to check for their own edits
  first.
- 🟡 **Compatibility / Correctness**: Conditional `[[--scope]]` /
  `--when` config is not resolved, so the limit can differ from `jj status`
  **Location**: Phase 4 §§1-2
  `jj_lib::config_resolver::resolve` (0.43.0, `config_resolver.rs:190`) is
  public and settings-free.
- 🟡 **Correctness / Test Coverage**: The expected unowned-path order is not
  byte-sorted
  **Location**: Phase 1 §2 domain tests
  Expected should be `[".accelerator/b", "meta/a.md"]`. The scanner double
  should return unsorted input so the sort is mutation-protected.
- 🟡 **Code Quality**: `Error::JjConfig`'s `Display` cannot name
  `snapshot.max-new-file-size`
  **Location**: Phase 4 §2
  Its `Display` is the fixed text "could not read the jj configuration"
  (`library.rs:159-160`, verified), and consumers log `%error`.
- 🟡 **Code Quality**: The rename to "run base" stops halfway
  **Location**: Terminology; Phase 2 §4
  `ManifestStore::run_id`/`write_run_id`, `classify(.., base_revision_matches)`
  and the manifest docs keep "run id" and "base revision".
- 🟡 **Test Coverage**: In-process dirty-path tests become
  environment-dependent after phases 3 and 4
  **Location**: Implementation Approach; Phase 2 §5 parity assertion
  The `guarded_resume.rs` parity check and the jj cases in `dirty_paths.rs`
  call `working_copy_diff` in the test process, which now reads `HOME`, XDG,
  `JJ_CONFIG` and the global git config.
- 🟡 **Usability**: A stale-manifest refusal is indistinguishable from a
  genuine unowned-change refusal
  **Location**: Phase 1 §2; Phase 2 §6
  After a rebase or a pre-upgrade stall, the run's own output is listed as
  "Unowned changes" with no hint that a previous run went stale.

#### Minor

- 🔵 **Test Coverage**: `for_workspace` layer selection is not unit-testable,
  and the read-only test touches the real config dir
  **Location**: Phase 4 §§1-2 (Code Quality raised the same hidden
  environment reads).
- 🔵 **Test Coverage**: The deferred jj "owned path omitted" refusal test is
  never scheduled
  **Location**: Phase 1 §2 / Phase 2 §5.
- 🔵 **Test Coverage**: Existing refusal edge-case tests don't assert
  `paths`; there is no manifest-present plus `None`-run-base case
  **Location**: Phase 1 §2.
- 🔵 **Test Coverage**: No negative tests for the broadened `-decisions.txt`
  suffix (`is_session_log`, malformed ids, a non-state path)
  **Location**: Phase 2 §2.
- 🔵 **Test Coverage**: The run-base error and `VcsKind::None` branches are
  untested
  **Location**: Phase 2 §3.
- 🔵 **Test Coverage / Code Quality**: The `base_ignores` non-git-backend
  branch is untested and speculative, and `.ok()` swallows every
  `get_git_repo` error
  **Location**: Phase 3 §3.
- 🔵 **Correctness**: The `library.rs` test comparing `revision` with
  `jj log -r @` on a dirty `@` depends on call order
  **Location**: Phase 2 §3.
- 🔵 **Architecture / Correctness**: The run base and the dirty scan are two
  separate loads, so "one view" is not guaranteed
  **Location**: Key Discoveries; Phase 2 §4.
- 🔵 **Compatibility / Correctness / Security**: `trusted_path` semantics
  differ from jj-cli, and config errors abort the scan
  **Location**: Phase 3 §3.
- 🔵 **Compatibility / Safety**: `CHANGELOG.md` misses the excludes change and
  the default 1 MiB limit, which now applies unconfigured. Files hidden this
  way lose pre-flight protection
  **Location**: Migration Notes; Phase 2 §6.
- 🔵 **Compatibility**: The `manifest-states/*/stderr` refusal goldens are
  unlisted, and `regenerate.sh`'s `gr_base_rev` uses a third run-base
  definition (`change_id`)
  **Location**: Phase 1 §2 Golden updates.
- 🔵 **Architecture**: Migrate vocabulary and the `+`-join encoding leak into
  `vcs-adapters`
  **Location**: Phase 2 §3.
- 🔵 **Architecture**: A pre-flight-only capability stays on the
  migration-wide `MigrationContext` port
  **Location**: Phase 2 §4.
- 🔵 **Architecture**: The decisions-file naming rule is split between
  `render.rs` and `manifest.rs`, and `SessionArtefact` is stretched to cover
  user-authored input
  **Location**: Phase 2 §2.
- 🔵 **Code Quality**: Two jj config-stack builders, and `jj_user_name`'s doc
  becomes wrong
  **Location**: Phase 4 §1.
- 🔵 **Code Quality**: `run_base`'s warn is invisible in migrate until phase 4
  wires up logging
  **Location**: Phase 2 §3.
- 🔵 **Code Quality**: The `ACCELERATOR_LOG` gating is copied into more
  binaries
  **Location**: Phase 3 §1; Phase 4 §4.
- 🔵 **Code Quality**: `classify` is called with a literal `true`
  **Location**: Phase 1 §2.
- 🔵 **Usability**: The "Unowned changes:" header exposes internal jargon
  **Location**: Terminology; Phase 1 §2.
- 🔵 **Usability**: An unbounded path list can bury the actionable
  instruction
  **Location**: Phase 1 §2.
- 🔵 **Security**: In-repo legacy jj config can switch off the guard through
  fail-open
  **Location**: Phase 4 §1.

#### Suggestions

- 🔵 **Architecture**: `work-adapters-fixture`'s `required-features` departs
  from the sibling fixture-binary convention.
- 🔵 **Code Quality**: Consider a `RunBase` newtype, a less misleading name
  than `secure_config_file`, and extracting `snapshot_options` from
  `working_copy_diff`.
- 🔵 **Compatibility**: Tie `secure_config_file`'s id format and layout to
  jj-lib constants so a pin bump fails loudly.
- 🔵 **Test Coverage**: Add a real rename parity fixture, extract shared
  migrate binary-test helpers, and name the file for the exclude and size
  migrate consumer tests.
- 🔵 **Safety**: Record why `clean` is safe for `work sync` pulls (the hash
  baseline).
- 🔵 **Security**: Escape control characters in the rendered path list.
- 🔵 **Usability**: Replace the tri-state `Option` `Hermetic` builders with
  intention-revealing ones. The macOS manual check path
  (`~/.config/jj/repos`) may be wrong.

### Strengths

- ✅ Sorted parent ids joined by `+` give an order-independent run base that
  matches `parents(@)` and stays stable across snapshots, owned edits and
  `jj describe @`.
- ✅ The `unowned_changes` rewrite preserves every existing accept/refuse
  decision, stays fail-closed when the run is not current, and replaces the
  `ForeignDirt` unit variant that computed its paths and then dropped them.
- ✅ The fixes sit in the one shared `working_copy_diff`, so migrate,
  `work sync` and the renderer get parity with no consumer-specific code.
- ✅ Functional core / imperative shell: the pure `excludes_file_path` and
  the `MaxNewFileSize` value type keep settings-free modules outside the
  settings lint.
- ✅ Config resolution is read-only, backed by hash tests. `secure_config_file`
  validates the 20-hex id, which blocks path traversal.
- ✅ The latent git bug is caught: the stall-named decisions file blocked its
  own resume.
- ✅ The refusal is additive: the existing text and `Display` are kept, and
  the git run base and corpus stamps are unchanged.
- ✅ Tests are strongly red-first, with an oracle whose parser has its own
  unit tests, inclusive-boundary size fixtures, op-heads-unchanged checks,
  and an honest list of the cases that cannot run hermetically.
- ✅ Each phase can be merged independently, with the domain reshape landing
  first.

### Recommended Changes

1. **Stop the new errors reaching the fail-open branch** (addresses: widened
   fail-open; untestable consumer positives; invisible invalid-limit warn;
   in-repo legacy config)
   - When the limit fails to resolve, `warn!` naming the key and fall back to
     `u64::MAX`.
   - When the base ignores fail to build, `warn!` and fall back to
     `GitIgnoreFile::empty()`.
   - Change the invalid-limit migrate test to expect a refusal that lists
     `meta/a.md`.
   - Give every positive consumer case a non-excluded control path that must
     be listed, and assert there is no `WARN` / `(status unavailable)`.
2. **Rewrite the pre-upgrade recovery** (addresses: the two-prompt decisions
   file is rejected; the forced run leaves unowned output; the CHANGELOG is
   not copy-pasteable; check own edits first)
   - Seed a stall that has already written migrated `meta/` files.
   - Choose between committing or discarding the partial output and then
     re-running, or having the forced run record the in-scope dirty paths.
   - Update the `CHANGELOG.md` text to an exact command, and tell users to
     verify the listed paths before forcing.
3. **Resolve conditional config** (addresses: scopes not resolved)
   - Pass the stack through `config_resolver::resolve` with a
     `ConfigResolutionContext` (home, repo path, workspace root, command
     `status`).
   - Add unit and parity cases for a scoped limit.
   - Alternatively, list scopes under What We're NOT Doing.
4. **Fix the sort expectation** (addresses: not byte-sorted)
   - Expect `[".accelerator/b", "meta/a.md"]`.
   - Have the double return unsorted input.
   - Assert line order in the binary test.
5. **Plan the error shape for the limit** (addresses: `JjConfig` `Display`)
   — add a dedicated variant naming the key, or make `JjConfig` print its
   source, and note the effect on `jj_user_name`.
6. **Complete the rename** (addresses: rename stops halfway; literal `true`)
   - Add Terminology rows for `run_id`/`write_run_id` → recorded run base and
     `base_revision_matches` → `run_base_matches`.
   - Keep `migrations-run.id` on disk.
   - Pass `current_run` instead of `true`.
7. **Make environment-dependent dirty-path tests hermetic** (addresses:
   in-process tests; `for_workspace` not unit-testable)
   - Route the `guarded_resume.rs` parity check and the jj
     `dirty_paths.rs` cases through `vcs-adapters-fixture` under `env.apply`.
   - Inject a `JjConfigEnvironment` into `for_workspace`.
8. **Tell a stale run apart in the refusal** (addresses: stale refusal
   indistinguishable) — carry a staleness flag on `UnownedChanges` and print a
   line explaining that a previous run's base moved.
9. **Settle the `core.excludesFile` read** (addresses: `trusted_path`
   semantics)
   - Read the raw string through `excludes_file_path`, and treat an
     unreadable global config as no excludes.
   - Match only the "not a git backend" error.
   - Drop or test the non-git-backend branch.
10. **Broaden the release notes** (addresses: the `CHANGELOG.md` gaps; files
    hidden from VCS lose protection) — cover the excludes change and the
    1 MiB default, and note that ignored or oversized `meta/` files are no
    longer guarded.
11. **Tighten the remaining tests** (addresses: the deferred jj owned-path
    test; edge-case `paths`; the negative decisions suffix; run-base
    error/`None`; call-order dependency; the `manifest-states` goldens)
    - Schedule the phase 2 jj owned-path case.
    - Assert the full `paths` vector in the existing edge-case tests.
    - Add the negative suffix cases.
    - Use `jj log --ignore-working-copy`.
    - List the `manifest-states` goldens and fix `gr_base_rev`.
12. **Structural tidy-ups** (addresses: the decisions naming split; two
    config builders; logging init placement and duplication; the
    `MigrationContext` port)
    - Move the decisions path into the migrate domain.
    - Route `jj_user_name` through `JjConfigSources`.
    - Add `kernel::logging::init_if_requested()` and wire it in phase 2.
    - Consider moving `run_base` onto the pre-flight's VCS port and computing
      it after the lock.

## Per-Lens Results

### Architecture

**Summary**: The plan is structurally sound. The jj fixes stay inside
vcs-adapters, where all three consumers pick them up through
`working_copy_diff`. Pure resolution is separated from I/O, and the
settings-lint boundary is respected. The weaker spots are domain alignment
and placement:

- migrate vocabulary leaks into the VCS adapter;
- a pre-flight-only capability stays on `MigrationContext`;
- the decisions-file naming rule is split across layers;
- the rename leaves "revision"/"run id" terms behind.

**Strengths**:
- A single shared `working_copy_diff` seam gives parity for every consumer.
- Functional core / imperative shell: `excludes_file_path` is pure and
  `MaxNewFileSize::resolve` takes injected sources.
- The new modules are settings-free, so the lint needs no new exemption.
- The path helpers move into one cohesive `jj_config.rs`.
- The run base is kept separate from `jj_revision`, so corpus stamps keep
  their meaning.
- Tradeoffs (fail-open, stale pre-upgrade stalls, one setting read) are
  explicit.
- The phases can be merged independently, with the domain reshape first.

**Findings**:
- 🔵 minor (high) — *Phase 2 §3* — **Migrate-domain vocabulary and the
  run-id encoding leak into the VCS adapter.** `run_base` and the `+`-join
  are migrate concepts. Expose `working_copy_base`/`base_commits` in VCS
  terms, and join in `FileMigrationContext`.
- 🔵 minor (medium) — *Phase 2 §4* — **A pre-flight-only capability stays on
  the `MigrationContext` port.** Every migration double carries `run_base`.
  Move it onto the pre-flight's VCS port (for example `DirtyPathScanner` or a
  `WorkingCopy` port) that already knows the root and kind.
- 🔵 minor (high) — *Phase 2 §2* — **The decisions-file naming rule is split
  between the CLI render layer and the domain classifier.** `render.rs`
  builds the path and `manifest.rs` owns the suffix, and `SessionArtefact` is
  stretched to cover user input. Add `manifest::decisions_file_path(id)`, and
  consider a `ResumeInput` ownership class.
- 🔵 minor (high) — *Terminology* — **The rename leaves "revision" and "run
  id" alongside "run base".** Extend the table to cover
  `base_revision_matches` and `write_run_id`, or state that the run id is the
  persisted run base.
- 🔵 minor (medium) — *Key Discoveries* — **"Same view for base and diff" is
  asserted but not guaranteed.** There are two loads, and the run base is
  read before the lock. Derive both from one load, or accept the window
  explicitly.
- 🔵 suggestion (medium) — *Phase 4 §1* — **Two jj config-stack resolvers
  with different layer coverage.** Route `jj_user_name` through
  `JjConfigSources` or name its narrower scope, and run the parity cases on
  every jj pin bump.
- 🔵 suggestion (medium) — *Phase 3 §1* — **The new `work-adapters-fixture`
  departs from the fixture-binary convention.** The siblings have no
  `required-features`. Follow the convention or justify the gate.

### Correctness

**Summary**: The core design holds up. The sorted-parents run base is stable
where it should be and changes where it should. `unowned_changes` keeps the
accept/refuse decisions. Three things are wrong: a sort expectation, the
pre-upgrade scenario (which conflicts with decisions validation and m0007
semantics), and the raw config read, which ignores conditional scopes.

**Strengths**:
- `unowned_changes` matches `fully_owned` in every case.
- The sorted and joined parents are order-independent and match the oracle,
  with a root parent shown as 40 zeros.
- The `-decisions.txt` suffix is owned only when the base matches, and
  `is_session_log` is unchanged.
- Parity records `dirty_paths` before `jj status` snapshots.
- The size boundaries match jj's inclusive check.

**Findings**:
- 🟡 major (high) — *Phase 2 §5 assertions 3-4* — **A decisions file
  answering only the first of two prompts is rejected.**
  `decisions_file::validate` fails closed with `missing a decision for
  position 2` before apply. Restructure the scenario, for example force with
  no decisions file and then answer the remaining prompts.
- 🟡 major (medium) — *Phase 2 §6; Migration Notes* — **After one forced
  run, the next resume can still be refused over the pre-upgrade output.**
  Force resets the manifest, and m0007 skips files that are already fenced,
  so those files are never recorded. Test with pre-migrated files, then fix
  the instruction or record the in-scope dirt under force.
- 🟡 major (high) — *Phase 1 §2* — **The expected unowned-path order is not
  byte-sorted.** Use `[".accelerator/b", "meta/a.md"]`.
- 🟡 major (medium) — *Phase 4 §1* — **Reading the jj config ignores
  `--when` scopes.** Resolve with `config_resolver::resolve`, or list scopes
  as unsupported.
- 🔵 minor (medium) — *Phase 2 §3* — **Comparing `InProcessProbe.revision`
  with `jj log -r @` on a dirty `@` depends on order.** Use
  `--ignore-working-copy`, or run `jj status` first.
- 🔵 minor (low) — *Key Discoveries; Phase 2 §4* — **The run base and the
  dirty scan come from separate loads.** Drop the invariant, or compute the
  run base after the lock from the same load.
- 🔵 minor (low) — *Phase 3 §3* — **gix `trusted_path` differs from jj-cli's
  raw-string read.** Read the raw string so `excludes_file_path` is the only
  resolver.

### Test Coverage

**Summary**: The test-first discipline is strong, the oracle is itself
tested, and the pure logic is unit-tested. The main weakness is that the
fail-open scanner and the renderer fallback let the positive consumer tests
pass on error. The sort expectation is also wrong, and several in-process
tests become environment-dependent.

**Strengths**:
- Red-first steps with named tests, including a direct #97 reproduction on
  both colocation modes.
- The oracle's parsing is unit-tested.
- Parent-order independence and literal root zeros are checked.
- The size fixtures sit on inclusive boundaries.
- The requirement 7 test is thorough.
- Non-mutation is asserted (op heads, config hashes).
- The plan is honest about the gaps it cannot cover hermetically.

**Findings**:
- 🟡 major (high) — *Phases 3-4 consumer tests* — **The positive exclude and
  size consumer tests pass even when the computation errors.** Add controls
  that must be listed, and assert no WARN and no `(status unavailable)`.
- 🟡 major (high) — *Phase 1 §2* — **The sort expectation is not
  byte-sorted, and unsorted input is not required.** Fix the vector, feed
  unsorted input, and assert line order.
- 🟡 major (medium) — *Implementation Approach; Phase 2 §5* — **In-process
  dirty-path tests become environment-dependent.** Route them through
  `vcs-adapters-fixture` under `env.apply`.
- 🔵 minor (medium) — *Phase 4 §§1-2* — **`for_workspace` layer assembly is
  not unit-testable, and the read-only test touches the real config dir.**
  Split out a pure constructor with a system-root parameter.
- 🔵 minor (medium) — *Phase 1 §2; Phase 2 §5* — **The deferred jj
  owned-path test is never scheduled.** Add it to phase 2.
- 🔵 minor (medium) — *Phase 1 §2* — **Existing refusal edge cases don't
  assert the paths.** Assert the full vector, and add a manifest plus
  `None`-base case.
- 🔵 minor (medium) — *Phase 2 §2* — **No negative test for the
  `-decisions.txt` suffix.** Cover `is_session_log`, `migrations--decisions.txt`
  and `meta/x-decisions.txt`.
- 🔵 minor (low) — *Phase 2 §3* — **The run-base error and no-VCS paths are
  untested.** Add a corrupt-`.jj` case with a warn, and a `VcsKind::None`
  case.
- 🔵 minor (medium) — *Phase 3 §3* — **The non-git-backend branch of
  `base_ignores` has no test.** Test it, make it injectable, or state the
  gap.
- 🔵 suggestion (medium) — *Testing Strategy* — **No parity fixture produces
  a rename.** Add one with and without a shared prefix.
- 🔵 suggestion (low) — *Phases 2-4* — **The binary-test scaffolding is
  duplicated and the consumer-test home is unspecified.** Extract helpers,
  name the file, and trim the colocation matrix.

### Code Quality

**Summary**: The structure is good: a single `unowned_changes` computation,
one pure resolver, and a `MaxNewFileSize` value type. The loose ends are:

- leftover vocabulary from the rename;
- an error `Display` that cannot name the key;
- hidden environment reads in a constructor meant to be unit-tested;
- a duplicate jj config-stack builder.

**Strengths**:
- `unowned_changes` replaces the boolean chain.
- `UnownedChanges { paths }` removes the discard-after-compute smell.
- A pure `excludes_file_path` is the only place resolution happens.
- `MaxNewFileSize` encodes the default and the "0 means unlimited" rule.
- `run_base` reuses `head_commit`.
- The helpers move into a settings-free module.

**Findings**:
- 🟡 major (high) — *Terminology; Phase 2 §4* — **The rename stops halfway:
  `run_id` and `base_revision_matches` remain.** Add rows for
  `recorded_run_base`/`record_run_base` and `run_base_matches`.
- 🟡 major (high) — *Phase 4 §2* — **`Error::JjConfig`'s `Display` cannot
  name `snapshot.max-new-file-size`.** Add a dedicated variant, or include
  the source in `Display`.
- 🔵 minor (medium) — *Phase 4 §1* — **`for_workspace` hides its
  environment reads.** Inject a `JjConfigEnvironment`.
- 🔵 minor (high) — *Phase 4 §1* — **Two stack builders, and
  `jj_user_name`'s doc becomes wrong.** Unify them and fix the doc.
- 🔵 minor (medium) — *Phase 3 §3* — **`base_ignores` swallows every
  `get_git_repo` error, and its fallback is speculative.** Match only the
  "not a git backend" error, and consider YAGNI on the fallback.
- 🔵 minor (high) — *Phase 2 §3* — **`run_base`'s warn is invisible in
  migrate until phase 4.** Move the logging init to phase 2.
- 🔵 minor (medium) — *Phase 3 §1; Phase 4 §4* — **The `ACCELERATOR_LOG`
  gating is copied.** Add `kernel::logging::init_if_requested()`.
- 🔵 minor (medium) — *Phase 1 §2* — **`classify` is called with a literal
  `true`.** Pass `current_run`, or split the rule.
- 🔵 suggestion (medium) — *Phase 2 §§3-4* — **The run base is
  `Option<String>` with a `+` encoding.** Consider a `RunBase` newtype.
- 🔵 suggestion (low) — *Phase 4 §1* — **`secure_config_file` is a
  misleading name.** Rename it to `scoped_config_file` or similar.
- 🔵 suggestion (low) — *Phase 4 §3* — **`working_copy_diff` keeps
  growing.** Extract `snapshot_options` using one root.

### Compatibility

**Summary**: The plan is careful: the git run base, corpus stamps and the
refusal text all survive, and the pre-upgrade break has a recovery. The gaps
are conditional-scope resolution, excludes error handling that diverges from
jj-cli, and release notes that undersell the jj behaviour change (the 1 MiB
default now applies when nothing is configured).

**Strengths**:
- The git run base stays `HEAD`, so git stalls resume across the upgrade.
- `jj_revision` and corpus stamps are unchanged.
- The refusal is additive.
- The pre-upgrade stall is tested end to end.
- Config reads are read-only.
- On a colocated repo the jj run base equals git `HEAD`.
- Goldens run under `Hermetic::apply`.

**Findings**:
- 🟡 major (medium) — *Phase 4 §§1-2* — **Conditional `[[--scope]]` tables
  are not resolved.** Use `config_resolver::resolve` with a
  `ConfigResolutionContext`, and add scoped cases.
- 🔵 minor (high) — *Migration Notes; Phase 2 §6* — **The changelog misses
  the excludes change and the default limit.** Add a `### Changed` entry,
  and fix the Migration Notes.
- 🔵 minor (medium) — *Phase 3 §3* — **The excludes lookup differs from
  jj-cli.** Read the raw string, and treat errors as no excludes.
- 🔵 minor (medium) — *Phase 1 §2* — **The `manifest-states` goldens and
  `regenerate.sh` will drift.** List the goldens, and fix or retire
  `gr_base_rev`.
- 🔵 suggestion (low) — *Phase 4 §1* — **The hand-copied secure-config
  layout could drift.** Tie it to jj-lib constants and add it to the pin
  re-verification list.

### Safety

**Summary**: The fix is read-only and the refusal stays fail-closed. The
main gap is that the new config and excludes errors reach the fail-open
pre-flight, and a test requires it. The forced recovery advice and the files
that are now invisible to the VCS need protective guidance.

**Strengths**:
- Fail-closed `unowned_changes`.
- An op-heads-unchanged test.
- A read-only config resolver with hash tests.
- A byte-identical refusal test.
- A `None` run base is fail-closed.
- A hermetic harness.
- The suffix change only widens classification.

**Findings**:
- 🟡 major (high) — *Phase 4; What We're NOT Doing* — **New config and
  excludes errors widen the fail-open path, and a test requires it.** Fall
  back to `u64::MAX` / empty ignores with a warn, and make the invalid-limit
  test expect a refusal.
- 🔵 minor (medium) — *Phase 2 §6; Migration Notes* — **The forced-run
  advice doesn't say to check for your own changes first.** Tell users to
  verify the listed paths, and mention `jj undo` / `jj op restore`.
- 🔵 minor (medium) — *Phases 3-4; Migration Notes* — **VCS-invisible files
  now pass the guard.** Document this, or record it as an accepted
  tradeoff.
- 🔵 suggestion (low) — *Phases 3-4 work sync tests* — **Record why `clean`
  is safe for pulls.** Add a hash-baseline test, or note it in the plan.

### Usability

**Summary**: The documented stall-then-resume flow now works, and the
refusal names the paths that block it. The weak edges:

- a stale refusal looks like a genuine one;
- the recovery lives only in the CHANGELOG;
- the invalid-limit warning is invisible by default;
- the header is jargon.

**Strengths**:
- The core flow is fixed, as is the latent git decisions-file bug.
- The paths are listed and sorted.
- `jj status` parity removes a class of surprises.
- The old refusal text is kept.
- The harness follows existing conventions.

**Findings**:
- 🟡 major (high) — *Phase 1 §2; Phase 2 §6* — **A stale-manifest refusal is
  indistinguishable from a genuine one.** Carry a staleness signal and print
  the recovery.
- 🟡 major (medium) — *Phase 4 §4* — **An invalid size limit silently
  disables the safety net by default.** Surface the warning by default, or
  document that it needs `ACCELERATOR_LOG`.
- 🔵 minor (medium) — *Terminology; Phase 1 §2* — **"Unowned changes:"
  exposes jargon.** Use, for example, "Changes not made by this migration
  run:" consistently.
- 🔵 minor (medium) — *Phase 1 §2* — **An unbounded list can bury the
  instruction.** Add a count, or a trailing instruction line.
- 🔵 minor (medium) — *Phase 2 §6* — **The CHANGELOG recovery is not
  copy-pasteable.** Give the exact command.
- 🔵 suggestion (low) — *Phase 3 §1* — **The tri-state `Option` builders
  are surprising.** Use `without_…` / `with_empty_…` builders.
- 🔵 suggestion (low) — *Phase 4 Manual Verification* — **`~/.config/jj/repos`
  may be wrong on macOS.** Use the directory that `jj config path --user`
  reports.

### Security

**Summary**: This is a local CLI with no network or secrets surface. Its
relevance is the new reads of partly in-repo config. They are well limited:
one numeric key, no execution, id validation and test-only binaries gated
out of release. The remaining gaps are the deferred trust-filter choice,
config-driven fail-open, and unescaped path output.

**Strengths**:
- Only `snapshot.max-new-file-size` is read, never `fsmonitor.backend`.
- The 20-hex id validation blocks traversal.
- The resolver is read-only.
- The fixture binary is gated out of release builds.
- The design uses `trusted_path`.
- Tests are hermetic.

**Findings**:
- 🔵 minor (medium) — *Phase 3 §3* — **The `trusted_path` versus raw choice
  is deferred, and it decides whether the trust filter applies.** Make the
  trust requirement explicit, and add an untrusted-source test.
- 🔵 minor (low) — *Phase 4 §1; What We're NOT Doing* — **In-repo legacy jj
  config can switch off the guard.** Record the accepted risk, or fail
  closed on `JjConfig` for migrate.
- 🔵 suggestion (low) — *Phase 1 §2* — **Unowned paths are printed
  unescaped.** Escape control characters, and add a test for a path
  containing a newline or ESC.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-09-23T22:16:26+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Safety / Test Coverage / Usability / Security**: New config and
  excludes errors widen the fail-open path — Resolved
- 🟡 **Correctness**: A two-prompt decisions file is rejected before the
  second stall — Resolved
- 🟡 **Correctness / Usability / Safety**: The forced run leaves the
  pre-upgrade output unowned — Resolved
- 🟡 **Compatibility / Correctness**: Conditional scopes are not resolved —
  Partially resolved. `config_resolver::resolve` is now used, but the
  context has no hostname or environment (see below).
- 🟡 **Correctness / Test Coverage**: The unowned-path order is not
  byte-sorted — Resolved
- 🟡 **Code Quality**: `Error::JjConfig` cannot name the key — Resolved
- 🟡 **Code Quality / Architecture**: The rename stops halfway — Resolved
- 🟡 **Test Coverage**: In-process dirty-path tests become
  environment-dependent — Partially resolved. The jj cases in
  `sync_working_copy_status.rs` and the `library.rs` case remain in-process.
- 🟡 **Usability**: A stale refusal looks like a genuine one — Partially
  resolved. It is flagged as stale, but gives no next step, and the fixed
  text still suggests force.
- 🔵 **Usability**: "Unowned changes" jargon — Still present (deliberately
  kept as domain vocabulary from 0263)
- 🔵 **Compatibility**: The hand-copied per-id config layout could drift —
  Partially resolved (only added to the pin re-verification list)
- 🔵 All other minor findings and suggestions from pass 1 — Resolved

### New Issues Introduced

- 🟡 **Correctness**: The run-base file assertions read a file that a
  successful run deletes. `manifest_store.clear()` (`main.rs:286`) removes
  `migrations-run.id` after `run_pending` succeeds. The "clean-run base" and
  "fresh run" rows must read the file while the run is stalled, or assert
  through a `ManifestStore` double.
- 🔵 **Safety / Usability**: The stale refusal should give the recovery
  itself: check the paths, commit, and re-run without force.
- 🔵 **Safety**: `jj commit` with no paths also commits changes outside the
  refusal's scopes. Scope the command to
  `meta .accelerator .claude`.
- 🔵 **Correctness / Compatibility**: `ConfigResolutionContext` needs
  `hostname` and an `environment` snapshot. Add them to `JjConfigEnvironment`
  (hostname source and cargo-deny impact), or list those scopes as
  unsupported.
- 🔵 **Correctness**: `char::escape_default` escapes non-ASCII characters and
  quotes. Escape only characters where `is_control()` is true.
- 🔵 **Correctness**: "A move of `@`" overstates staleness. Moving onto the
  same parent keeps the run base, and a manifested path the user edits there
  is adopted. Reword the end state, and either pin this behaviour or accept
  it.
- 🔵 **Compatibility**: `RunBase` `FromStr` must be an opaque, infallible
  parse of existing `migrations-run.id` contents (40-hex, `@` id, empty,
  `stale-revision-sentinel`).
- 🔵 **Compatibility**: `cli/kernel/tests/fixtures/public-api.txt` needs
  regenerating for `init_if_requested`.
- 🔵 **Architecture / Code Quality**: `InProcessProbe::base_commits` has no
  production caller and invites two loads. Drop it and project from
  `working_copy_state`.
- 🔵 **Architecture / Correctness**: The one-read invariant on git is
  unspecified. Read `HEAD` from the same `gix::Repository` handle as the
  status, or narrow the doc.
- 🔵 **Test Coverage**: `stale_run` is not pinned on the absent-base and
  `None`-base cases. The `observe` error branch asserts only the warn. The
  WARN assertions need `ACCELERATOR_LOG=warn` set in the shared helper.
- 🔵 **Usability**: The CHANGELOG says an invalid limit logs a warning, but
  that only shows with `ACCELERATOR_LOG`.
- 🔵 **Safety**: No test pins that a failed read under force still proceeds
  and records `None`.
- 🔵 Suggestions:
  - a named `SnapshotDiff` return;
  - a single representation for "unlimited" in `MaxNewFileSize`;
  - `Error::JjSnapshotSetting` could be module-private;
  - the conditional no-git-backend parity assertion;
  - run `jj config path --user` outside the repository in the manual check.

### Assessment

Every critical-path problem from pass 1 is fixed, and the plan is acceptable
for implementation. One new major finding remains: the run-base file
assertions cannot pass as written. It is a test-design fix confined to two
rows of the phase 2 table. The remaining items are minor refinements to
fold in before or during implementation, and none changes the design.

## Verdict Change — 2026-09-23T22:51:08+00:00

**Verdict:** APPROVE

The pass-2 major finding and every pass-2 minor finding were folded into the
plan:

- The run-base assertions now read `migrations-run.id` while the run is
  stalled.
- Hostname and environment scopes are now resolved, through
  `JjConfigEnvironment` and a new `whoami` dependency.

The reviewer approved the plan for implementation without a further lens
pass.
