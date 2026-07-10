---
type: plan-review
id: "2026-07-07-0178-config-crates-native-yaml-reader-review-1"
title: "Plan Review: config and config-adapters Crates with Native YAML Reader"
date: "2026-07-07T09:35:11+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-07-07-0178-config-crates-native-yaml-reader"
target: "plan:2026-07-07-0178-config-crates-native-yaml-reader"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, compatibility, safety, standards, portability]
review_number: 1
review_pass: 3
tags: [rust, config, config-adapters, serde, yaml, cargo-deny, cargo-pup, hexagonal]
last_updated: "2026-07-07T12:55:12+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: config and config-adapters Crates with Native YAML Reader

**Verdict:** REVISE

This is a mature, well-grounded plan: the two-crate hexagon is architecturally
sound, serde is correctly confined to the adapter (which is what makes the
cargo-deny/cargo-pup gates meaningful), phasing is disciplined and independently
mergeable, and the enforcement harnesses ship real positive controls. The
findings cluster in two places, and both matter because this task freezes the
public API and the Model-1 composition template that six-plus consumer crates
(0167, 0169–0173) will copy: (1) the differential parity harness has a
tautology at its core and under-specifies value encoding, so several genuine
bash-vs-Rust divergences would pass CI green; (2) two real defects — the legacy
guard rooting at CWD instead of the discovered project root, and `FoundSequence`
not being wired into the precedence short-circuit — would propagate to every
consumer. No criticals, but fourteen major findings across seven lenses put this
firmly at REVISE.

### Cross-Cutting Themes

- **Parity harness proves less than it claims** (flagged by: test-coverage,
  correctness, compatibility, architecture) — The differential test feeds the
  Rust catalogue's *own* default into the bash oracle (tautological), never
  defines the `Resolved`→string projection it compares against, and omits
  fixtures for the value-encoding cases where a real YAML parser must diverge
  from bash's opaque-string handling (`{number:04d}`, quoted scalars, quoted
  array elements, `null` vs empty). The "depth ≤2 parity over every recognised
  key" guarantee is therefore overstated.

- **The Model-1 template carries a latent guard bug** (flagged by: architecture,
  correctness) — The composition root runs `assert_no_legacy_layout(&cwd)` then
  `FileConfigStore::discover(&cwd)` (which walks up). Run from a subdirectory the
  guard checks the wrong directory and silently fails open, diverging from the
  bash guard's project-root rooting. The black-box test seeds files at CWD, so
  the gap is never exercised — and this is the exact bin consumers mirror.

- **`FoundSequence` splits the "presence, not value" invariant across two
  variants** (flagged by: correctness, code-quality, architecture) — Adding
  `FoundSequence` to `Resolved` without widening the precedence `matches!` means
  a personal inline-array silently falls through to the team value. Every
  presence-reasoning site (precedence, present-empty shadowing, future consumers)
  must now match two arms and keep them in sync.

- **cargo-pup rule enumerates modules and is only parse-checked** (flagged by:
  architecture, test-coverage, standards) — The rule lists seven module names
  (so a future domain module silently escapes enforcement), and the regression
  exercises a hand-written probe rule rather than the real shipped regex, so a
  typo in the alternation would pass every test.

- **`serde-saphyr 0.0.29` risks are asserted, not verified** (flagged by:
  compatibility, safety) — Licence coverage under the deliberately-pruned
  `deny.toml` allow-list is claimed without checking the transitive closure; the
  exact pin on a `0.0.x` crate is fragile against the `yanked = "deny"` /
  `unmaintained = "all"` policy; and the visualiser's YAML panic-sandbox is
  dropped on the reasoning "pure-Rust" without an adversarial-input fail-safe
  test.

### Tradeoff Analysis

- **Fail-loud vs graceful degradation on malformed config**: The ported reader
  fails the whole `get` loudly if either level is malformed; bash warns and skips
  the bad file, resolving the other level or the default. Safety favours
  fail-loud in principle, but *availability* favours bash's degradation — after
  cutover a single stray character in `config.local.md` would block every skill.
  The recommendation is not to pick a side blindly but to make it a **tested,
  documented decision** (like the `ACCELERATOR_MIGRATION_MODE` fail-closed
  choice already is), with a malformed-file fixture pinning the intended
  behaviour.

- **Enumerated pup regex vs whole-crate match**: Enumerating modules is more
  explicit and mirrors the existing `version::core` rules, but the `config` crate
  is *entirely* domain (no adapter modules inside it), so a whole-crate match
  (`^config($|::)`) is both simpler and future-proof. Prefer the whole-crate
  match here — the module-rooted style is a carry-over from single-crate
  hexagons that does not apply to a fully-domain crate.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Architecture + Correctness**: Legacy guard roots at CWD, not the
  discovered project root — diverges from bash and fails open from a subdirectory
  **Location**: Phase 3, Section 1 (config_reader.rs main)
  `main` runs `assert_no_legacy_layout(&current_dir())` (no walk) then
  `FileConfigStore::discover(&current_dir())` (walks up). The bash guard checks at
  `config_project_root`. Run from a subdir of a legacy repo, the Rust guard finds
  nothing and passes while discovery reads the real root — the fail-closed
  guarantee silently breaks, and this is the template every consumer copies.

- 🟡 **Correctness + Code Quality + Architecture**: `FoundSequence` must be added
  to the precedence short-circuit, not just the `Resolved` enum
  **Location**: Phase 1, Section 4 (application service)
  Luminosity decides personal-over-team with `matches!(resolved, Found(_))`.
  Adding `FoundSequence` without widening that predicate means a personal inline
  array (e.g. a locally-set `review.core_lenses`) does not short-circuit and
  silently falls through to team. Code-quality argues the deeper fix is a single
  presence-bearing `Found(Value)` so "present" is one arm.

- 🔴 **Test Coverage + Architecture**: Default-fallback parity is tautological —
  the 42 catalogue defaults are never independently verified
  **Location**: Phase 2, Section 7 (differential tests) / Phase 1, Section 5
  The harness runs `config-read-value.sh <key> <catalogue-default>`, passing the
  Rust catalogue's own default as bash's default argument. `config-read-value.sh`
  never reads `config-defaults.sh`/`config-dump.sh`, so both readers echo the same
  supplied value and the assertion holds trivially. A drifted default in
  `catalogue.rs` passes the whole suite; only a manual read-through guards it.

- 🟡 **Correctness + Compatibility + Test Coverage**: Value-encoding parity is
  unspecified — the `Resolved`→string projection is undefined and YAML-special /
  quoted values are untested
  **Location**: Phase 2, Sections 6–7 (fixtures + differential tests)
  Bash treats every value as an opaque string; a real parser types and unescapes.
  `work.id_pattern: {number:04d}` (the catalogue default's own shape) reads as the
  literal string under bash but parses as a flow mapping (→ `Absent`) under YAML.
  The plan never defines the string projection used for comparison (Null→? vs
  bash empty vs literal `null`; leading-zero `04`→`Int(4)`; bool casing), and no
  fixture covers quoted scalars (bash strips one quote pair) or quoted array
  elements (bash keeps them literal; serde parses them).

- 🟡 **Test Coverage + Safety + Correctness**: Malformed-frontmatter divergence is
  untested and unreconciled
  **Location**: Phase 2, Sections 6–7 / Phase 1, Section 4
  Bash warns and skips a malformed file (resolving the other level/default);
  the ported reader fails the whole `get` loudly. No fixture exercises this, so a
  real, user-visible difference (malformed team file + valid local value: bash
  resolves, Rust errors) is neither tested nor documented — and after cutover a
  single corrupt personal config would block every skill.

- 🟡 **Test Coverage**: The pup regression does not guard the real shipped rule's
  discrimination
  **Location**: Phase 4, Section 4 (crate-rooted probe regression)
  The probe writes its own `_PROBE_PUP_RON` against a probe crate with a `core`
  module (which the real `config` crate does not have); the real `cli/pup.ron` is
  only parse-checked by `test_real_cli_pup_ron_loads`. A typo in the seven-module
  alternation (`servcie`, or omitting `catalogue`) would let a serde import slip
  past with every test green. The plan's manual claim that "reverting the pup.ron
  rule makes test:integration:pup fail" is therefore incorrect.

- 🟡 **Architecture + Standards**: The pup rule enumerates domain modules, so new
  modules silently escape enforcement
  **Location**: Phase 4, Section 1 (cargo-pup crate-rooted rule)
  The whole `config` crate is domain, yet the rule matches
  `^config::(node|key|level|error|service|catalogue|legacy)`. A future domain
  module (e.g. `defaults.rs`) falls outside the regex and loses import
  restriction with nothing failing. Match the crate root
  (`^config($|::)` with `^crate($|::)` self-allow) instead.

- 🟡 **Standards**: The `[[bans.deny]]` array-of-tables collides with the existing
  inline `deny = [...]` structure and would fail to parse
  **Location**: Phase 4, Section 2 (cargo-deny wrappers ban)
  `cli/deny.toml` defines bans as an inline array under `[bans]`. TOML cannot
  define `bans.deny` as both an inline-array value and an array-of-tables — the
  proposed block is a redefinition conflict that breaks `deny:check` rather than
  activating the ban. Append an inline table:
  `{ crate = "serde-saphyr", wrappers = ["config-adapters"] }`.

- 🟡 **Compatibility**: `serde-saphyr` licence coverage under `deny.toml` is
  asserted, not verified
  **Location**: Phase 2, Section 1 (workspace dependency)
  The plan states "already covered by deny.toml's allow-list, so no licence edit"
  as settled, but the allow-list is deliberately pruned to exactly the current
  closure's licences and fails on any outside it. `serde-saphyr`'s transitive
  closure (saphyr, saphyr-parser, deps) is unverified — an unlisted permissive
  licence (0BSD, MIT-0, …) would fail Phase 2 and block all downstream consumers.

- 🟡 **Compatibility**: Exact `=0.0.29` pin is fragile against the
  yanked/unmaintained advisory policy
  **Location**: Phase 2, Section 1 (workspace dependency)
  With `yanked = "deny"` / `unmaintained = "all"` and an exact `0.0.x` pin cargo
  has no in-range float: a yank of exactly `0.0.29`, or a RustSec advisory on it
  or a transitive dep, turns the whole `cli` deny gate red with no automatic
  recovery, blocking all consumers until a manual (possibly API-breaking) bump.
  Keep the pin but document the risk and that the adapter boundary localises it.

- 🔴 **Safety**: The YAML panic-sandbox is dropped for a `0.0.x` parser with no
  adversarial-input fail-safe test
  **Location**: Performance Considerations / Phase 2, Section 3
  Pure-Rust does not imply panic-free — an early-stage parser can still panic or
  stack-overflow (uncatchably) on deeply-nested/malformed input. No fixture proves
  adversarial YAML resolves to a clean `ConfigError` rather than a crash. Once cut
  over (0167 unblocks immediately), a hostile config could abort the read path
  every skill depends on.

- 🟡 **Test Coverage + Architecture + Correctness**: The mode-dependent
  `review.min_lenses` default cannot be exercised by the claimed fixture
  **Location**: Phase 2, Section 6 (three extra review keys) / Phase 1, Section 5
  The 4-for-pr/plan vs 3-for-work-item default is applied by
  `config-read-review.sh`, only when the key is absent, and the Rust reader has no
  notion of mode. A fixture that *sets* `review.min_lenses` bypasses every
  default; a `default_for(key)` map can encode only one value. The AC-linked
  fixture is claimed to cover a behaviour it structurally cannot.

- 🟡 **Test Coverage**: Doc-type array parity mechanism is underspecified
  **Location**: Phase 2, Section 7 (recognised-key loop)
  AC-1 requires fixtures for both 13-entry doc-type arrays, but the harness only
  invokes `config-read-value.sh <key>`, which resolves flat `paths.*`/`review.*`
  keys and does not perform the doc-type→path-key lookup (that lives in
  higher-level bash helpers). It is unclear how the parallel-array mapping is
  differentially verified at all.

- 🟡 **Standards**: `config::…::core` naming is inconsistent with the crate's flat
  module layout
  **Location**: Desired End State / Phase 4, Section 4
  The Overview and probe describe guarding `config::…::core` and a probe crate
  "named config with a core module", but the `config` crate has no `core`
  submodule (its modules are flat). The `::core` framing is a carry-over from the
  module-rooted `version::core` convention and could lead an implementer to create
  a spurious module or test a name the real rule never matches.

- 🟡 **Portability**: "Skip if bash unavailable but fail in CI" does not translate
  to Rust's test harness — it becomes a silent PASS
  **Location**: Phase 2, Section 7 (differential tests)
  The referenced `_require_tools` convention is pytest-specific
  (`pytest.skip`/`pytest.fail`). Rust's stable harness has no runtime skip: an
  early return registers as a green PASS. On an environment without `bash` the
  depth-≤2 parity assertions silently do not run yet appear passing. Specify the
  exact mechanism (probe PATH; `panic!` when `CI`/`GITHUB_ACTIONS` is set;
  otherwise `eprintln!` + return, noting it is a silent pass).

#### Minor

- 🔵 **Architecture**: `FoundSequence(Vec<Scalar>)` leaves sequences-of-non-scalars
  undefined and bakes a two-variant "found" contract into the public API.
  **Location**: Phase 1, Section 4 — specify the outcome for a sequence whose
  elements are not all scalars (error vs `Absent`).

- 🔵 **Correctness**: Full-stack `get` fails loud if either level is malformed;
  bash warns and skips — note as a deliberate divergence or match bash's per-file
  tolerance.
  **Location**: Phase 1, Section 4 / Phase 2. (Overlaps the malformed-frontmatter
  major above; same underlying divergence.)

- 🔵 **Correctness**: Array-valued recognised keys (`review.core_lenses`,
  `review.disabled_lenses`) sit in *both* the depth-≤2 string-parity loop and the
  typed-sequence set — the plan does not say how the loop reconciles a bash string
  against a `FoundSequence`.
  **Location**: Phase 2, Section 7 — exclude array keys from the string-parity loop
  or define a string re-projection.

- 🔵 **Correctness**: `discover` roots at `.accelerator/` *or* `.git`; bash roots at
  `.git` only. A nested `.accelerator/` inside a git repo, or the no-marker
  fallback, can root the two readers differently.
  **Location**: Phase 2, Section 4 — confirm the fallback matches bash's `$PWD`
  and align/​pin the marker precedence.

- 🔵 **Code Quality**: Catalogue comments pin rows to bash source line numbers
  (`config-defaults.sh:26-64`), which violates the no-comments/no-source-ref policy
  and rots on any bash edit.
  **Location**: Phase 1, Section 5 — drop the line refs; encode provenance as a
  row-equality test instead.

- 🔵 **Code Quality**: Catalogue modelled as positional `&[(&str, &str)]` tuples;
  `.0`/`.1` access is not self-documenting and `default_for` acquires per-group
  branching (agent prefix, template-no-default).
  **Location**: Phase 1, Section 5 — a small named record / per-row default enum
  makes the tables self-describing.

- 🔵 **Test Coverage**: No parity fixture for the quote-stripping asymmetry the
  research flags as load-bearing (one pair stripped on scalars, none on array
  elements).
  **Location**: Phase 2, Section 6. (Folded into the value-encoding major above.)

- 🔵 **Test Coverage**: The normal-layout positive-control fixture must actually set
  `paths.work` (the bin exits 1 on `NotFound` — it applies no default), else the
  control is unstable.
  **Location**: Phase 3, Sections 1–2.

- 🔵 **Safety**: The demo `config-reader` `[[bin]]` has a generic, shippable-looking
  name kept out of releases only by `publish = false` + convention; `cargo build
  --workspace` still emits `target/release/config-reader`.
  **Location**: Phase 3, Section 1 — mirror luminosity's `-fixture` suffix and/or
  assert its exclusion in the distribution checks.

- 🔵 **Safety**: The `WriteConfigLevel` path ships enabled but is under-specified
  (same-dir temp / fsync-before-rename) and unexercised beyond unit tests.
  **Location**: Phase 2, Section 4 — confirm same-directory temp + fsync, or note
  write correctness is deferred to 0167.

- 🔵 **Portability**: The bash oracle is shelled out without pinning the locale;
  awk POSIX-class/whitespace behaviour can vary under non-C `LANG`/`LC_ALL`,
  making the differential's ground truth environment-dependent.
  **Location**: Phase 2, Section 7 — set `LANG=C`/`LC_ALL=C` in the child env.

- 🔵 **Portability**: `CARGO_MANIFEST_DIR → ../../scripts` bakes in a two-levels-up
  layout assumption; a relocation breaks with an opaque "command not found".
  **Location**: Phase 2, Section 7 — assert the resolved script path exists at
  test start with a diagnostic.

#### Suggestions

- 🔵 **Compatibility**: Mark the consumer-facing enums (`Resolved`, `ConfigError`)
  `#[non_exhaustive]` now, so later variant additions (`FoundMapping`, new error
  cases) remain backward-compatible across the six-plus consumer crates.

- 🔵 **Compatibility**: Add a rationale comment to the `serde-saphyr = "=0.0.29"`
  workspace line matching the convention every other `=` pin follows (clap,
  vergen, reqwest…), explaining the `0.0.x`-is-breaking pin.

- 🔵 **Standards**: `config-adapters/Cargo.toml` snippet shows only
  `[dependencies]` — note it also carries the `[package]` `.workspace = true`
  inheritance and `[lints] workspace = true` block like every other crate.

- 🔵 **Architecture**: Record the `serde-saphyr` swap-out seam (the
  `Parsed`/`to_node` boundary) and how the accelerator copy stays in sync with the
  moving external `../luminosity` reference.

- 🔵 **Safety**: Record 0172 (mid-migration read fallback) as an explicit hard
  prerequisite for any Rust config-consumer cutover that can execute during a
  migration, so the fail-closed sequencing promise is tracked, not left to
  reviewer memory.

- 🔵 **Standards**: Crate naming is mixed (`config`/`config-adapters` unprefixed vs
  the existing `accelerator-verify`) — state the unprefixed choice is deliberate so
  it does not read as an oversight.

- 🔵 **Portability**: Note native-Windows is out of scope (inherited POSIX-only:
  bash oracle, temp-then-rename not atomic-over-existing on Windows) so the lock-in
  is acknowledged.

### Strengths

- ✅ The core split is right: `config` depends only on `kernel`, all serde/YAML/fs
  live in `config-adapters` — which is precisely what makes the cargo-deny
  `wrappers` ban and the cargo-pup rule enforceable (the domain crate literally
  cannot have the YAML crate in its closure).
- ✅ Clean functional-core / imperative-shell separation: the legacy-guard decision
  is a pure `const fn is_blocked(bool, bool)` in the domain; filesystem stats live
  in the adapter, so the core is unit-testable with no I/O.
- ✅ The hexagon confines the volatile pre-1.0 `serde-saphyr` to one crate, and
  choosing it over the repo's C-backed `serde_yml` removes the libyml panic hazard
  the visualiser had to `catch_unwind` around.
- ✅ Disciplined phasing: each phase leaves `mise run` green and is independently
  mergeable, with an accurate dependency graph (1→2 sequential; 3 and 4 both
  depend only on 1–2 and are independent of each other).
- ✅ Both enforcement harnesses ship explicit positive controls (clean deny fixture
  named `config-adapters`, compliant pup probe) plus a committed violating canary,
  so a green run means "evaluated and allowed/bit", not "evaluated nothing" — and
  the canary runs the real `cli/deny.toml` via `--config` against an offline stub.
- ✅ The parity scope is honestly bounded to depth ≤2 (where the oracle can
  represent the data), with declared-value verification at depth ≥3 and for typed
  sequences.
- ✅ Strong test isolation: each fixture is materialised into a fresh
  `CARGO_TARGET_TMPDIR` subdir seeded with a `.git` marker, so neither reader's
  upward walk can escape into the accelerator repo's own config.
- ✅ The deliberate fail-closed decision (dropping the `ACCELERATOR_MIGRATION_MODE`
  bypass) is soundly justified against the actual bash migrate flow and gets a
  dedicated positive+negative black-box test.

### Recommended Changes

1. **Discover the project root first, then guard and build the store against that
   same root; add a subdirectory-CWD test** (addresses: legacy-guard-CWD). This is
   the Model-1 template consumers copy — fix it here and exercise it from a nested
   directory so the divergence can't recur downstream.

2. **Widen the precedence short-circuit to include `FoundSequence` (or collapse to
   a single `Found(Value)` presence variant); add a personal-sequence-over-team
   precedence test** (addresses: FoundSequence-precedence, two-variant-found).
   Prefer the single-`Found(Value)` shape so "presence, not value" stays one arm.

3. **Give the catalogue defaults an independent oracle** (addresses:
   default-fallback-tautology, dual-source-of-truth). Assert the Rust catalogue
   defaults equal the bash arrays (source `config-defaults.sh`/`config-dump.sh` or
   drive `config-dump.sh`, which reads them), and add a bidirectional
   key-set-equality check so drift in either reader fails a test.

4. **Specify the `Resolved`→string projection and add value-encoding fixtures**
   (addresses: value-encoding-parity, quote-stripping, {number:04d}). Define
   Null/bool/int/float rendering; add present-value fixtures for `{number:04d}`, a
   flow-indicator-leading value, a quoted scalar, and a quoted array element;
   assert agreement or document each intentional divergence.

5. **Add a malformed-frontmatter fixture and decide the semantic explicitly**
   (addresses: malformed-divergence, fail-loud-degradation). Choose skip-and-warn
   (match bash's degradation) or fail-loud, document it alongside the
   `ACCELERATOR_MIGRATION_MODE` decision, and pin it with a fixture.

6. **Make the pup rule whole-crate and test the real shipped rule's discrimination**
   (addresses: pup-enumerates-modules, pup-regression-gap, config-core-naming).
   Match `^config($|::)`; drive a probe crate literally named `config` through the
   *real* `cli/pup.ron` (as the deny test uses `--config`), and drop the `::core`
   framing throughout.

7. **Fix the `deny.toml` ban syntax** (addresses: deny-toml-conflict). Append
   `{ crate = "serde-saphyr", wrappers = ["config-adapters"] }` to the existing
   inline `deny = [...]` array — do not add an `[[bans.deny]]` block.

8. **Verify `serde-saphyr`'s transitive licences and document the pin/advisory
   risk** (addresses: licence-unverified, yank-pin-fragility). Run
   `cargo deny --config cli/deny.toml check licenses` against the resolved graph,
   add any missing permissive licence in Phase 2, and record that a yank/advisory
   on the pinned crate is a whole-layer build-stop the adapter boundary localises.

9. **Add adversarial-input YAML fixtures asserting a clean `ConfigError` (not a
   crash)** (addresses: panic-sandbox-dropped). If `serde-saphyr` lacks a
   recursion-depth bound, cap depth in the adapter or retain a `catch_unwind`
   boundary until its panic-safety on untrusted input is verified.

10. **Resolve the mode-dependent `min_lenses` and doc-type-array coverage claims**
    (addresses: min-lenses-fixture, doc-type-underspecified). Scope mode-dependent
    defaults to the consumer (drop the misleading fixture claim; keep the catalogue
    default at 4 and assert the reader returns Absent), and state which bash
    entrypoint is the oracle for doc-type resolution (or confirm doc-types are pure
    catalogue data with no resolution step here).

11. **Specify the Rust bash-availability mechanism and pin the oracle's locale**
    (addresses: bash-silent-pass, oracle-locale). Probe `bash` on PATH, `panic!`
    when `CI`/`GITHUB_ACTIONS` is set, note the non-CI path is a silent pass, and
    set `LANG=C`/`LC_ALL=C` in the `Command` child env.

12. **Smaller hardening** (addresses: non-exhaustive-enums, pin-comment,
    config-adapters-manifest, positive-control-fixture, demo-bin-name,
    catalogue-comments): mark `Resolved`/`ConfigError` `#[non_exhaustive]`; add the
    `serde-saphyr` pin rationale comment; show the `config-adapters` `[package]`/
    `[lints]` inheritance; require the normal-layout fixture to set `paths.work`;
    give the demo bin a `-fixture`/`-demo` suffix; drop the bash-line-number
    comments from the catalogue.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: A well-formed two-crate hexagon that faithfully mirrors the version
slice, keeps serde/YAML/fs strictly in the adapter, and correctly makes the crate
boundary the thing that makes the cargo-deny `wrappers` ban and cargo-pup rule
enforceable. Boundaries, dependency direction, and functional-core/imperative-shell
separation are sound. The main risks are in enforcement and composition wiring
rather than the crate split: an enumerated-module pup regex that silently loses
coverage, a legacy-guard sequence that roots at CWD rather than the discovered
root, and a 42-key catalogue duplicated across bash and Rust with weak drift
detection. (Luminosity was not accessible in the agent's environment; structural
decisions were evaluated against the accelerator codebase patterns.)

**Strengths**:
- The core split is right: `config` depends only on `kernel`; serde/YAML/fs live in
  `config-adapters`, which is exactly what makes the `wrappers` ban and pup rule
  meaningful.
- Clean functional-core / imperative-shell separation (pure `is_blocked`, fs stats
  in the adapter).
- The hexagon confines pre-1.0 `serde-saphyr`; the `wrappers` ban bounds the blast
  radius of ever swapping the YAML backend to `config-adapters` alone.
- Disciplined, independently-mergeable phasing with an accurate dependency graph.
- Enforcement reuses existing harnesses and adds a violating canary + clean control.
- Parity correctly bounded to depth ≤2 with declared-value checks beyond.

**Findings**:
- 🟡 major (medium) — Phase 4 §1: pup rule enumerates domain modules, so new
  modules silently escape enforcement. The whole crate is domain; match
  `^config($|::)` with `^crate($|::)` self-allow instead.
- 🟡 major (medium) — Phase 3 §1: legacy guard roots at CWD, not the discovered
  project root, diverging from the bash oracle; never exercised because tests seed
  files at CWD. Propagates to every Model-1 consumer.
- 🟡 major (medium) — Phase 1 §5: the 42-key catalogue becomes a dual source of
  truth; the parity test iterates the Rust catalogue so drift is caught only by a
  manual checkbox. Add a bidirectional completeness check.
- 🔵 minor (medium) — Phase 1 §5: the mode-dependent `review.min_lenses` default has
  no home in the static catalogue model; decide domain vs out-of-scope explicitly.
- 🔵 minor (medium) — Phase 1 §4: `FoundSequence(Vec<Scalar>)` leaves non-scalar
  sequences undefined and hardcodes a two-variant "found" contract; consider
  `Found(Value)`.
- 🔵 suggestion (low) — Phase 2 §1: the sole YAML backend is a pinned pre-1.0 crate;
  isolation is good but record the swap-out seam and external-reference sync.

### Code Quality

**Summary**: Unusually well-structured for maintainability — mirrors an established
hexagonal template, keeps serde out of the domain, injects adapters through ports,
works test-first, and gives error variants that carry path/detail context.
Concerns are localised: the `Resolved` twin `Found`/`FoundSequence` variants create
a precedence invariant every consumer must honour twice; the catalogue is
positional tuples with rot-prone line-number comments; and small readability/error
smells in the demo bin and the boolean predicate.

**Strengths**:
- Strong testability by design (ports-as-traits, fakes, pure predicate, black-box
  bin test via `CARGO_BIN_EXE`).
- Serde/YAML/fs confined to `config-adapters`; boundary self-enforcing.
- Context-rich error taxonomy + `From<ConfigError> for kernel::Error` boundary.
- `default_for` nominated as the single source of catalogue defaults.
- Clear, independently-mergeable phases.

**Findings**:
- 🔴 major (medium) — Phase 1 §4: `Resolved` encodes "presence, not value" across
  two present-variants; a site checking `Found` but not `FoundSequence` silently
  breaks personal-over-team precedence for arrays. Prefer a single `Found(Value)`.
- 🔵 minor (medium) — Phase 1 §5: catalogue comments pin rows to bash line numbers,
  violating the no-comments policy and rotting on any bash edit; encode as a test.
- 🔵 minor (medium) — Phase 1 §5: positional `&[(&str, &str)]` tuples aren't
  self-documenting and `default_for` acquires per-group branching; use a named
  record / per-row default enum.
- 🔵 suggestion (medium) — Phase 1 §6: `is_blocked(bool, bool)` has transposable
  adjacent flag args; a `LayoutFacts` struct removes the hazard.
- 🔵 suggestion (low) — Phase 3 §1: demo `main` mixes `.expect(...)` panics with the
  `ExitCode` contract and prints `{resolved:?}` (Debug); fold cwd failure into the
  exit path and print via Display.

### Test Coverage

**Summary**: A genuinely strong skeleton — isolated-tempdir differential harness,
built-in positive controls, a dedicated fail-closed `ACCELERATOR_MIGRATION_MODE`
negative test, and bespoke `FoundSequence` tests. But the differential harness has
a tautology at its core (it feeds the Rust catalogue's own default into the bash
oracle), the pup regression exercises a hand-written probe rather than the real
shipped regex, and a real behavioural divergence on malformed frontmatter is
neither tested nor reconciled.

**Strengths**:
- Isolated `CARGO_TARGET_TMPDIR` + seeded `.git` + atomic counter gives strong
  isolation and determinism.
- Explicit positive controls in both enforcement harnesses.
- Dedicated negative test for the intentional fail-closed divergence.
- The one reference deviation (`FoundSequence`) is covered by new tests.
- The deny canary runs the real `cli/deny.toml` against an offline stub.
- Test-first discipline with ported luminosity suites as executable spec.

**Findings**:
- 🔴 major (high) — Phase 2 §7: default-fallback parity is tautological; catalogue
  defaults never independently verified. Give the default path an independent
  oracle.
- 🔴 major (high) — Phase 4 §4: the pup regression does not guard the real shipped
  rule; a typo in the alternation passes every test. The manual "revert makes it
  fail" claim is incorrect.
- 🟡 major (medium) — Phase 2 §6–7: malformed-frontmatter divergence untested and
  unreconciled; the "depth ≤2 parity over every recognised key" claim is overstated.
- 🟡 major (medium) — Phase 2 §6: the mode-dependent `min_lenses` default cannot be
  exercised by a present-value fixture; the reader has no notion of mode.
- 🟡 major (medium) — Phase 2 §7: doc-type array parity mechanism is underspecified;
  `config-read-value.sh` does not do the doc-type→path-key lookup.
- 🔵 minor (medium) — no parity fixture for the quote-stripping asymmetry the
  research flags as load-bearing.
- 🔵 minor (medium) — Phase 3 §1–2: the normal-layout positive control must set
  `paths.work` or the bin exits 1.
- 🔵 suggestion (medium) — thin coverage of presence-not-value shadowing variants
  (team-empty, local-null, local-empty-string) across levels.

### Correctness

**Summary**: Unusually well-grounded — correctly identifies the load-bearing bash
semantics and maps them onto luminosity's verified resolver. The risk is
concentrated where the Rust reader must *diverge* from the reference
(`FoundSequence`) and where a strict YAML parser cannot faithfully reproduce a
lenient line-oriented awk oracle: unquoted YAML-special values, typed-scalar
rendering, and null/empty ambiguity all produce results bash does not, and several
are not exercised by the fixture plan as written.

**Strengths**:
- Correctly captures presence-not-value shadowing and cites luminosity's proofs.
- The `!team && legacy` predicate is an exact match for the bash guard, with a
  positive+negative black-box test.
- Sound fixture isolation via the seeded `.git` marker.
- Parity honestly bounded to depth ≤2.

**Findings**:
- 🔴 major (high) — Phase 1 §4: `FoundSequence` must be added to the precedence
  short-circuit; otherwise a personal inline-array falls through to team.
- 🟡 major (medium) — Phase 2 §6 / §3: unquoted YAML-special values (`{number:04d}`)
  diverge — bash returns the literal string, serde parses a collection → `Absent`.
- 🟡 major (medium) — Phase 2 §7: the `Resolved`→string projection is undefined and
  has irreconcilable cases (null vs empty vs typed scalars, leading zeros, bool
  casing).
- 🟡 major (medium) — Phase 3 §1: the legacy guard runs at cwd, not the discovered
  root — bypassable from a subdirectory, and never caught (tests seed at cwd).
- 🔵 minor (medium) — full-stack `get` fails loud if either level is malformed; bash
  warns and skips. Note the divergence or match bash tolerance.
- 🔵 minor (medium) — Phase 2 §7: array-valued keys sit in both the string-parity
  loop and the typed-sequence set; the loop's reconciliation is unspecified.
- 🔵 minor (low) — Phase 2 §4: `discover` roots at `.accelerator/` or `.git`; bash
  roots at `.git` only — nested-marker and no-marker fallbacks unspecified.
- 🔵 suggestion (low) — Phase 1 §5: `review.min_lenses` mode-dependent default
  cannot be represented by a single-value catalogue; keep the reader mode-agnostic.

### Compatibility

**Summary**: The core strength is architectural — quarantining the volatile
`serde-saphyr 0.0.29` behind `config-adapters` so later consumers bind to an owned,
stable API, and treating the bash reader as an explicit parity oracle. The main
risks are dependency-management ones the plan asserts rather than verifies: licence
coverage under the pruned allow-list, and the brittleness of an exact `=0.0.29` pin
against the yanked/unmaintained advisory policy. A secondary risk is that the
bash-parity contract may not cover value-quoting edge cases.

**Strengths**:
- Hexagonal quarantine of the volatile `0.0.x` dependency; consumers depend on the
  owned public API.
- The differential test makes the compatibility contract explicit and executable.
- Pure-Rust `serde-saphyr` avoids the libyml C-panic hazard.
- Zero backward-compat risk to current consumers (bash stays the shipping reader).
- `multiple-versions = "warn"` makes duplicate-version risk non-fatal.

**Findings**:
- 🟡 major (medium) — Phase 2 §1: `serde-saphyr` licence coverage under `deny.toml`
  is asserted, not verified; an unlisted transitive licence fails the gate and
  blocks all consumers.
- 🟡 major (medium) — Phase 2 §1: exact `=0.0.29` pin is fragile against
  `yanked = "deny"` / `unmaintained = "all"` — a yank/advisory reddens the whole
  gate with no float.
- 🟡 major (medium) — Phase 2 §6–7: the parity contract may diverge on value quoting
  (bash strips one pair; serde does real YAML unescaping); no fixture covers it.
- 🔵 minor (medium) — Phase 2 §7: the awk oracle is platform-variant
  (BSD/one-true-awk vs mawk/gawk); a fixture could pass on darwin and fail on linux.
- 🔵 minor (high) — Phase 2 §1: the `=0.0.29` pin lacks the rationale comment every
  other `=` pin carries, and tightens luminosity's caret without recording why.
- 🔵 suggestion (low) — Phase 1 §4/§7: mark consumer-facing enums (`Resolved`,
  `ConfigError`) `#[non_exhaustive]` so later variants stay backward-compatible.

### Safety

**Summary**: A low-blast-radius task — a native reader alongside the still-shipping
bash reader, no wired consumer, only a non-shipped test bin — so nothing here can
harm production today. The fail-closed guard is the right default and its
off-critical-path proof checks out against the actual bash migrate flow; the
deny/pup canary reuses the proven isolated-fixture pattern. The two surviving
concerns are forward-looking but worth resolving now because this sets the reader's
safety contract for imminent cutover: the panic sandbox is dropped on a `0.0.x`
parser without an adversarial test, and fail-loud reads remove bash's graceful
degradation.

**Strengths**:
- Fail-closed guard is the safe default; dropping the `ACCELERATOR_MIGRATION_MODE`
  bypass is soundly justified (verified against `run-migrations.sh:632`,
  `interactive-lib.sh:434,745`) and genuinely off the migration critical path.
- The deny canary reuses the proven offline-isolation pattern; cannot pollute the
  real graph or ship.
- Blast radius effectively zero (bash stays shipping; the bin is `publish = false`,
  non-staged).
- Positive-control discipline throughout.
- Atomic temp-then-rename write + verbatim body preservation carried over with unit
  coverage.

**Findings**:
- 🔴 major (medium) — Perf Considerations / Phase 2 §3: the panic sandbox is dropped
  for a `0.0.x` parser with no adversarial-input fail-safe test; a hostile config
  could crash the read path after cutover.
- 🟡 major (medium) — Phase 1 §4 / Phase 2 §7: fail-loud reads remove bash's
  graceful degradation; a single stray character would block every skill.
- 🔵 minor (medium) — Phase 3 §1: the generically-named non-shipped bin relies on
  convention, not a guard, to stay out of releases.
- 🔵 minor (low) — Phase 2 §4: the write path ships enabled but under-specified
  (fsync/same-dir) and unexercised beyond unit tests.
- 🔵 suggestion (medium) — Migration Notes: no cross-task guard prevents a pre-0172
  Rust consumer from running during a migration; record 0172 as a hard prerequisite.

### Standards

**Summary**: Largely convention-faithful — the `config` manifest uses
`.workspace = true` + `[lints] workspace = true`, the demo bin follows the
hyphen-name/underscore-file convention, and verification routes through `mise run`.
Two real consistency problems: the proposed `[[bans.deny]]` array-of-tables
collides with `deny.toml`'s existing inline `deny = [...]`, and the plan refers to
a `config::…::core` module the crate's flat layout does not have.

**Strengths**:
- The `config` manifest correctly opts into workspace inheritance and shared lints.
- Demo bin naming follows the `accelerator-fixture` → `accelerator_fixture.rs`
  convention.
- Verification consistently uses `mise run` tasks, reserving raw cargo for the
  documented escape hatch.
- The new pup rule reuses the existing anchoring style and `kernel::Error` allowance.
- Planned manifest/RON comments align with the established convention (Rust source
  stays comment-free).

**Findings**:
- 🟡 major (high) — Phase 4 §2: `[[bans.deny]]` array-of-tables conflicts with the
  inline `deny = [...]`; would fail to parse. Append an inline table instead.
- 🟡 major (high) — Desired End State / Phase 4 §4: `config::…::core` is
  inconsistent with the crate's flat module layout; drop the `::core` framing and
  align the probe with a module the real regex matches.
- 🔵 minor (medium) — Phase 2 §2: the `config-adapters` snippet omits the
  `[package]` `.workspace = true` inheritance and `[lints] workspace = true` block.
- 🔵 suggestion (medium) — Phase 4 §1: the enumerated pup regex is a maintenance
  trap; match the whole crate.
- 🔵 suggestion (low) — Phase 1 §1 / Phase 2 §1: crate naming is mixed vs
  `accelerator-verify`; state the unprefixed choice is deliberate.

### Portability

**Summary**: Portability-conscious for its actual targets (macOS + Linux). The
standout win is pure-Rust `serde-saphyr` over C-backed `serde_yml` — no C
toolchain, no `catch_unwind`, clean cross-compile. The main gaps are the Rust
parity harness's environment coupling: the pytest "skip locally, fail in CI"
convention does not translate to Rust's harness, and the bash oracle is shelled out
without locale pinning.

**Strengths**:
- `serde-saphyr` is pure-Rust — no C toolchain, no panic sandbox, clean
  cross-compile under zigbuild.
- Isolated-tempdir + seeded `.git` root discovery is correct on both targets.
- The bash oracle sources its libraries via `BASH_SOURCE`-derived `SCRIPT_DIR`, not
  cwd, so the isolated-tempdir cwd doesn't break sourcing.
- Reuses the already-shipping bash-3.2-clean scripts as the oracle — no new shell
  portability surface.
- Same-directory temp-then-rename is POSIX-atomic on both targets.

**Findings**:
- 🟡 major (medium) — Phase 2 §7: "skip if bash unavailable but fail in CI" does not
  translate to Rust — an early return is a silent PASS, masking the parity check.
- 🔵 minor (medium) — Phase 2 §7: the bash oracle is shelled out without pinning the
  locale; awk POSIX-class behaviour can vary under non-C `LANG`/`LC_ALL`.
- 🔵 minor (low) — Phase 2 §7: `CARGO_MANIFEST_DIR → ../../scripts` bakes in a
  layout assumption; assert the resolved path exists with a diagnostic.
- 🔵 minor (low) — overall: the config subsystem is POSIX-only by design; note
  native-Windows is out of scope so the lock-in is acknowledged.

## Re-Review (Pass 2) — 2026-07-07

**Verdict:** REVISE

The revision is a large step up: **13 of the 14 prior major findings are fully
resolved and the 14th is substantially addressed**, and all four judgment-call
decisions (single `Found(Value)`, fail-loud, fixtures-only panic stance,
whole-crate pup rule) landed cleanly. The re-run did not resurface the resolved
issues — but reviewing the tightened plan fresh surfaced a new, focused set of
majors, two of them **high-confidence correctness/parity bugs** that the first
pass did not reach because the parity harness it critiqued was under-specified.
The most important: the single-arm precedence claim is not bash-equivalent for a
key whose value is a *non-scalar node* (bash counts it textually present and
shadows; Rust resolves `Absent` and falls through), and `discover_root`'s marker
set drops `.jj` — which `find_repo_root` honours (`vcs-common.sh:11`, verified) —
so a jj-workspace checkout without a colocated `.git` roots differently. Because
several new majors are genuine (not nitpicks) and two are real divergences in a
jj-first repo, the verdict stays REVISE — but this is a converging tail of
well-understood, mostly-quick fixes, not a rework.

### Previously Identified Issues

- 🟡 **Architecture + Correctness**: Legacy guard roots at CWD — **Resolved.**
  Phase 3 §1 now discovers the root once and guards against it; a nested-subdir
  black-box test was added. (New wrinkle below: the marker *set* used by discovery
  diverges from bash — a different bug than the CWD one.)
- 🟡 **Correctness + Code Quality + Architecture**: `FoundSequence` precedence —
  **Resolved.** Single `Found(Value)` with a one-arm `matches!(_, Found(_))` and a
  personal-sequence-over-team test.
- 🔴 **Test Coverage + Architecture**: Default-fallback tautology / dual source —
  **Resolved.** Sentinel default in the parity loop + bidirectional catalogue
  drift test; test-coverage confirms the tautology is gone.
- 🟡 **Correctness + Compatibility + Test Coverage**: Value-encoding parity /
  projection unspecified — **Partially resolved.** The projection is now defined
  and value-encoding fixtures added, but correctness + compatibility flag it is
  still lossy for classes the fixture list omits (comments, `null`/`~`, bool case,
  non-canonical numbers) and is not a *shared exported* function (see new issues).
- 🟡 **Test Coverage + Safety + Correctness**: Malformed-frontmatter divergence —
  **Resolved** (documented + fixture). Minor follow-ups below (only the Rust side
  is asserted; read-order not pinned).
- 🟡 **Test Coverage**: Pup regression doesn't guard the real rule — **Resolved.**
  Probe now drives the real `cli/pup.ron` via `--config`.
- 🟡 **Architecture + Standards**: Pup rule enumerates modules — **Resolved.**
  Whole-crate `^config(::|$)`.
- 🟡 **Standards**: `[[bans.deny]]` parse conflict — **Resolved.** Inline-table
  append; standards confirms it corrects the work item's own note.
- 🟡 **Compatibility**: serde-saphyr licence asserted not verified — **Resolved.**
  In-phase `cargo deny check licenses` step with allow-list update.
- 🟡 **Compatibility**: Exact-pin yank/advisory fragility — **Resolved** (rationale
  comment + fragility note). New suggestion: name the red-gate recovery lever.
- 🔴 **Safety**: Panic sandbox dropped — **Resolved per decision** (fixtures-only
  characterise). New minor: the "revisit if a fixture panics" trigger under-covers
  stack-overflow-abort / OOM (not catchable by `catch_unwind`).
- 🟡 **Test Coverage + Architecture + Correctness**: Mode-dependent `min_lenses` —
  **Resolved.** Reader is mode-agnostic; misleading fixture claim dropped.
- 🟡 **Test Coverage**: Doc-type array parity underspecified — **Resolved.**
  Catalogue-drift equality, no resolution step, documented.
- 🟡 **Standards**: `config::…::core` naming — **Resolved.** `::core` framing
  dropped throughout.
- 🟡 **Portability**: bash-unavailable silent pass — **Resolved.** Probe + `panic!`
  under `CI`/`GITHUB_ACTIONS`, documented non-CI silent pass.

### New Issues Introduced

- 🔴 **Correctness** (high, Phase 1 §4): **Non-scalar-node presence divergence.**
  bash's awk matches the key line itself — a `work:` line that merely opens a
  nested block resolves found-empty and shadows the team value/default — whereas
  Rust `resolve` returns `Absent` for a `Mapping` or non-scalar `Sequence`. So the
  single-arm `Found(_)` precedence *inverts* relative to bash for this input class,
  which is exercisable even for a recognised scalar key. Either mark a
  present-but-non-addressable node as present, or add a fixture documenting it as
  an accepted divergence.
- 🔴 **Correctness (high) + Portability (medium)** (Phase 2 §4 / Phase 3 §1):
  **`discover_root` drops the `.jj` marker.** Verified: `find_repo_root` stops at
  `.jj` **or** `.git` (`vcs-common.sh:11`), and this repo is jj-colocated. Rust's
  `.accelerator/`-or-`.git` set roots a `.jj`-only workspace checkout differently
  (and adds `.accelerator/`, which bash never uses as a stop marker), a real
  parity gap the colocated-`.git` fixtures cannot catch. Align the marker set to
  `.jj`/`.git` (VCS-only) and add a `.jj`-only fixture, or record it as an
  explicit divergence in Platform scope.
- 🟡 **Architecture + Compatibility** (medium, Phase 1 §3–4): **`Value`/`Scalar`
  lack `#[non_exhaustive]`** while `Resolved`/`ConfigError` have it — yet `Value`
  is the enum most likely to grow (mapping addressability → `Value::Mapping`), and
  the plan's own `Resolved::FoundMapping` justification contradicts the single-arm
  design (mappings should grow `Value`). Apply `#[non_exhaustive]` to `Value` and
  `Scalar` too.
- 🟡 **Compatibility + Test Coverage + Correctness** (medium, Phase 1 §5 / §7):
  **Array-key default type inconsistency.** `default_for` returns
  `review.core_lenses` as a bracketed *string*, but a present value resolves to
  `Value::Sequence` — so a consumer sees a different shape across the presence
  boundary, and no test pins the default-path shape. Also the get-applies-default
  contract is stated inconsistently (Phase 2 §7 vs the bin's exit-1-on-absent).
  Parse array defaults into `Value::Sequence` so both paths yield the same shape,
  and pin where defaults apply.
- 🟡 **Correctness + Compatibility + Code Quality** (medium, Phase 2 §7 / Phase 3):
  **The `Resolved`→string projection has no shared home and is still lossy.** It is
  described identically in `parity.rs` and the bin but never stated to live in one
  exported function (drift risk), and omits value classes (comments, `null`/`~`,
  bool case, `007`/`1.0`/`1e3`/`+5`) where parse-then-format diverges from bash's
  literal passthrough. Export one projection function; add numeric/comment/null
  fixtures as tested or documented divergences.
- 🟡 **Standards** (high, Phase 3 §1): **Fixture bin in `src/bin/`, not
  `tests/fixtures/`.** The repo's only precedent (`accelerator-fixture`,
  `launcher/Cargo.toml:19-21`) places fixture bins at `tests/fixtures/` with an
  explicit `[[bin]] path`; `src/bin/` is auto-built into `target/release/`,
  re-introducing the release-pickup risk the convention avoids. Also the name
  `config-reader-fixture` breaks the `<crate>-fixture` pattern
  (`config-adapters-fixture`). Move it to `tests/fixtures/` and reconcile the name.
- 🟡 **Code Quality** (medium, Phase 3 §1): **Composition protocol re-implemented
  by six consumers.** discover-root-once → guard → build-store-at-same-root is a
  subtle ordered protocol each of 0167/0169–0173 must hand-copy; expose a thin
  shared `compose(cwd)` helper in `config-adapters` (still Model-1 — each binary
  calls it at its own root) so the protocol is one tested unit.
- 🟡 **Test Coverage** (medium, Phase 2 §7): **Per-key parity can go vacuous.** For
  a recognised key never actually *set* in its fixture, both readers echo the
  sentinel and the assertion is `SENTINEL == SENTINEL` — verifying no resolution.
  Assert each scalar key against a fixture where it is genuinely set, and fail if a
  key's check only ever hit the miss branch.
- 🟡 **Architecture** (medium, Migration Notes): **Fail-loud is a shared-layer
  SPOF** — a re-raise of the *accepted* tradeoff (user chose fail-loud): a single
  corrupt team file fails every read across six consumers. Not a regression;
  recorded so the cutover (0167) explicitly owns degradation UX. Safety concurs it
  is acceptable for this no-consumer task.

Plus a tail of minor/suggestion items: three extra review-key defaults not sourced
by the drift test; adversarial fixtures asserting recorded-behaviour vs a hard
contract; malformed fixture asserts only the Rust side; struct-variant field
additions not protected by enum-level `#[non_exhaustive]`; directory-fsync-after-
rename gap for 0172; CI-detection keyed only on `CI`/`GITHUB_ACTIONS`; pup anchor
ordering cosmetic (`^config(::|$)` vs `($|::)`).

### Assessment

The plan is now materially stronger and internally consistent on every point the
first pass raised. The verdict remains **REVISE** only because the fresh read of
the tightened parity story exposed two high-confidence correctness divergences
(non-scalar-node presence; the `.jj` marker) plus a coherent cluster around the
value-type contract (`#[non_exhaustive]` on `Value`/`Scalar`, array-default shape,
shared projection) and one concrete standards fix (fixture bin location). These
are a convergent, mostly-quick tail — none demands re-architecting — and once the
two correctness parity gaps are either fixed or explicitly documented as accepted
divergences (as the fail-loud/fail-closed ones already are), the plan is ready to
implement. A third pass should be scoped to just these deltas.

## Re-Review (Pass 3) — 2026-07-07

**Verdict:** APPROVE

Every pass-2 finding is confirmed resolved by the lenses that raised it (the
non-scalar-node present-and-shadow resolution was independently verified
bash-equivalent against `config-read-value.sh:73-89`; the `.jj` marker against
`vcs-common.sh:11`). This pass surfaced a **diminishing tail** — no criticals, four
majors, the rest minor/suggestion — characteristic of a converged review: the
findings are precision/consistency issues in the plan's own sketches and edge-case
documentation, none touching architecture or approach. **All were addressed in the
same iteration** (no decisions required), so the verdict is APPROVE.

### Previously Identified Issues (pass 2)

- 🔴 Non-scalar-node presence divergence — **Resolved** (per decision: match bash;
  `Found(Value::Scalar(Scalar::Null))` → `""`, verified bash-equivalent).
- 🔴 `discover_root` drops `.jj` — **Resolved** (marker set + `.jj`-only tests;
  verified against `find_repo_root`).
- 🟡 `Value`/`Scalar` lack `#[non_exhaustive]` — **Resolved.**
- 🟡 Array-key default type inconsistency — **Resolved** (`default_for` returns typed
  `Value`; shape-parity fixture).
- 🟡 Projection unshared / lossy — **Resolved** (exported `render_value`/`render_resolved`;
  null/bool/number/comment fixtures).
- 🟡 Fixture bin in `src/bin/` — **Resolved** (`tests/fixtures/`, `config-adapters-fixture`).
- 🟡 Composition protocol copied by six consumers — **Resolved** (`compose(cwd)` helper).
- 🟡 Per-key parity vacuous sentinel — **Resolved** (genuinely-set-fixture floor).
- 🟡 Fail-loud SPOF — **Resolved** (re-raise of an accepted tradeoff; cutover owns UX).

### New Issues Introduced — all addressed this pass

- 🔴 **Correctness** (high): the bin `main()` sketch returned `SUCCESS` on
  `Ok(Resolved::Absent)`, contradicting the "Absent → exit 1" prose. **Fixed** —
  the bin matches `Found`/`Absent` explicitly (Found→print+0, Absent→stderr+1), with
  a companion normal-layout-without-`paths.work` black-box case exercising the arm.
- 🟡 **Test Coverage** (medium): the adversarial "no-hang" floor was unenforceable —
  `cargo test` has no default per-test timeout, so a billion-laughs hang would wedge
  the suite. **Fixed** — adversarial parses run under a bounded-time guard
  (worker-thread `recv_timeout` or subprocess wall-clock kill).
- 🟡 **Compatibility** (medium): the recorded yank-recovery lever (`[advisories]
  ignore` by advisory id) cannot clear a *yank* (no advisory id). **Fixed** — a yank
  uses a PackageSpec ignore (`serde-saphyr@0.0.29`); the pinned cargo-deny version is
  confirmed to support it, distinct from the advisory-id form.
- 🟡 **Correctness** (medium): block-authored `review.core_lenses` diverges (bash
  found-empty→default; Rust parses the list) — untested and undocumented. **Fixed** —
  block-sequence fixture + Documented Divergences entry (inline form stays parity).
- 🔵 **Architecture/Compat/Correctness** (minor, ×3 lenses): `Resolved`
  `#[non_exhaustive]` contradiction between §3 and §4. **Fixed** — `Resolved` is a
  deliberately-closed `Found`/`Absent` enum; growth lives in the `#[non_exhaustive]`
  `Value`; dropped from the §3 list.
- 🔵 **Code Quality** (minor): `render_resolved`'s `Sequence` arm was undefined.
  **Fixed** — `render_value` is total (`Sequence`→`[a, b, c]`); `render_resolved`
  layers on it.
- 🔵 **Test Coverage / Safety** (minor): malformed divergence fixtured only for the
  team file. **Fixed** — symmetric malformed-personal fixture added.
- 🔵 **Correctness** (minor): absent `config.local.md` must yield an empty level, not
  `Io`. **Fixed** — `ReadConfigLevel` semantics specified + unit test.
- 🔵 **Test Coverage** (minor): doc-type drift must compare ordered pairs. **Fixed** —
  zip-equality against the parallel arrays.
- 🔵 **Compatibility** (minor): serde-saphyr MSRV unchecked vs `rust-version 1.90.0`.
  **Fixed** — MSRV check folded into the Phase 2 verification step.
- 🔵 **Standards** (minor): fixture bin + test files omit the crate-level clippy-allow
  headers. **Fixed** — headers specified per the `accelerator-fixture` convention.
- 🔵 **Code Quality** (suggestion): drift via driving `config-dump.sh`, not parsing.
  **Fixed** — committed to behavioural extraction.
- 🔵 **Portability** (suggestion): child `Command` should pin `PWD`. **Fixed.**
- 🔵 **Architecture** (suggestions): `config-adapters` cohesion + `compose` guard-policy
  seam for 0172. **Fixed** — `compose`/`render` in dedicated modules; the three guard
  primitives stay public so 0172 composes rather than re-copies.
- 🔵 **Standards** (suggestion): config data fixtures nested under
  `tests/fixtures/configs/`. **Fixed.**

### Assessment

The plan has converged. Pass 3 confirmed every prior finding resolved and found only
a diminishing tail of sketch-consistency and edge-case-documentation items, all of
which are now addressed in the plan with no open decisions. There is no architectural
or approach-level concern outstanding. The plan is **APPROVE** and ready for
`/implement-plan`; a fourth lens pass is not warranted (the remaining changes were
mechanical tightenings, not design changes).
