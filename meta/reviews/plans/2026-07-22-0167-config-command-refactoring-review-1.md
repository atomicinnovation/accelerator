---
type: plan-review
id: "2026-07-22-0167-config-command-refactoring-review-1"
title: "Plan Review: 0167 config Command Refactoring"
date: "2026-07-22T22:06:19+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "plan:2026-07-22-0167-config-command-refactoring"
target: "plan:2026-07-22-0167-config-command-refactoring"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, code-quality, correctness, test-coverage, compatibility, standards, safety]
review_number: 1
review_pass: 4
tags: [rust, config, cli, launcher, store, hexagon, refactoring]
last_updated: "2026-07-22T23:29:38+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: 0167 config Command Refactoring

**Verdict:** REVISE

This is an unusually well-constructed refactoring plan: it correctly
diagnoses a single missing domain seam as the root of ~10 duplicated tails,
isolates the one behaviour change to Phase 1, respects the pup import
boundaries phase-by-phase, and pins behaviour-neutrality to byte-identical
committed goldens with failing-test-first discipline. The consolidation is
architecturally sound and the code-quality direction is textbook DRY. The
reason for REVISE is not the strategy but one under-specified instruction:
Phase 3 tells the implementer to route the scalar subcommands
(`get`/`path`/`work`) through the new `effective` operation, and three lenses
independently show that a literal reading of that instruction would silently
change observable output — breaking the byte-identical contract the whole plan
rests on. That, plus a small cluster of API-specification gaps and one Phase 7
ordering hazard, needs tightening before implementation.

### Cross-Cutting Themes

- **Routing scalar `get`/`path`/`work` through `effective` changes observable
  output** (flagged by: architecture, correctness, compatibility) — This is the
  dominant finding, raised at high confidence by three lenses. `effective`
  bakes catalogue-default-on-absence into resolution, but `config get` has **no
  catalogue tier at all** (returns `--default` or empty), and `get`/`path`
  honour a caller-supplied `--default` that outranks the catalogue. A naive
  `effective(...).rendered()` at these sites would emit catalogue defaults for
  unset keys and drop explicit `--default` values — observable changes outside
  the sanctioned Phase 1 fix.

- **`--explain` provenance cannot be rebuilt from `Resolution.source()` alone**
  (flagged by: architecture, correctness) — Phase 3 says explain provenance is
  derived from `Resolution.source()` instead of per-level probes, but `source()`
  names only the *winner*; today's `explain_lines` reports set/not-set for *each*
  level. The phase's own success criterion ("names both levels") is unreachable
  from `source()`, so the both-levels-set explain golden cannot be reproduced.

- **The four-variant tail taxonomy is incomplete** (flagged by: correctness,
  compatibility) — The Current State taxonomy omits `config get`'s
  no-catalogue variant and the explicit-`--default`-over-catalogue precedence of
  `get`/`path`. This omission is the root cause of the routing risks above; an
  implementer following the taxonomy would route these tails uniformly and
  regress.

### Tradeoff Analysis

- **Seam uniformity vs. behaviour fidelity**: The plan's aesthetic goal is
  "every duplicated tail calls `effective`". But `get` (no catalogue), the
  explicit-`--default` paths, and `review::resolve` (caller-default, catalogue-
  bypassing) genuinely diverge. The right resolution is to make the divergence
  explicit — branch on `Resolution::from_config()` at these sites, or keep
  them off `effective` — rather than force uniformity and regress output. Fewer
  call sites collapsed, but no behaviour change.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Architecture / Correctness / Compatibility**: Routing `config get`
  through `effective` injects a catalogue default it must never apply
  **Location**: Phase 3: New scalar assemblers (`resolve_get`)
  `resolve_get` (`inbound/cli.rs:610-626`) returns `default.unwrap_or_default()`
  on absence and never consults the catalogue. `effective` applies the catalogue
  default on absence, so `config get paths.tmp` (unset) would return
  `.accelerator/tmp` where it returns empty today — a moved golden on a phase
  declared behaviour-neutral.

- 🟡 **Correctness / Compatibility**: Explicit `--default` precedence over the
  catalogue is dropped for `get`/`path`
  **Location**: Phase 3: New scalar assemblers (`path_fallback`/`work_fallback`)
  Today the precedence is config-value > explicit `--default` > catalogue >
  empty+warning. `effective` folds the catalogue default in immediately after
  the config value, so `config path tmp --default X` (key absent) would return
  `.accelerator/tmp` instead of `X`.

- 🟡 **Architecture / Correctness**: `--explain` provenance is not
  reconstructible from `Resolution.source()`
  **Location**: Phase 3: New scalar assemblers (`--explain` provenance)
  `source()` records only the winning level; `explain_lines` reports the
  set/not-set status of *both* levels. The both-levels-set case (Phase 3's own
  success criterion) cannot be rebuilt from `source()` alone, so the explain
  golden would move.

- 🟡 **Correctness**: `template::scalar`'s `Option<String>` contract is broken
  by `effective_nonempty`
  **Location**: Phase 2: Collapse the tails (`core/template.rs` scalar)
  `template::scalar` returns `Option<String>` where `None` means "no configured
  override", driving fall-through to the user-override/plugin tiers. For
  `templates.<name>` keys the catalogue has no default, so `effective_nonempty`
  yields `Unset`/rendered-`""`, not `None`; a naive
  `Some(effective_nonempty(...).rendered())` hands `resolve_template` `Some("")`
  and short-circuits the override tiers.

- 🟡 **Safety**: `store::read_within` fuses containment-check and read, but they
  must straddle the mkdir/lock in `append_record`
  **Location**: Phase 7: `store::read_within` adoption in corpus-adapters
  In `append_record` the containment check must precede `create_dir_all` (so a
  symlinked component cannot redirect the built tree) while the read must sit
  *after* the append lock. A single fused `read_within` between them cannot honour
  both; a naive merge either defeats the mkdir containment guard or introduces a
  lost-append race.

- 🟡 **Test Coverage**: Phase 5's relocated review verdict/revise prose is
  exercised by no fixture
  **Location**: Phase 5, Change 1: Prose out of core
  The baseline fixture sets no review severities, so `verdict_lines`/
  `revise_verdict` take the empty-output branch and the golden has no
  `**Verdict**` line. The `none`-disabled, custom-severity, and non-default-count
  branches are golden-pinned by nothing — the byte-identical guarantee is vacuous
  for exactly the prose being moved.

- 🟡 **Architecture**: The Phase 8 launcher→store dependency contradicts the
  plan's own store-boundary invariant
  **Location**: Phase 8: `cache.rs` uses `store::TEMP_PREFIX`
  The plan asserts "`store` remains importable only from the `*-adapters`
  crates" (Cross-cutting facts), then Phase 8 adds a direct launcher→store edge
  for `TEMP_PREFIX`. The two statements are in direct tension and one must give.

#### Minor

- 🔵 **Compatibility**: `ConfigError::Io` reword is a second observable change,
  contradicting the "one phase changes output" framing
  **Location**: Phase 8: Reword `ConfigError::Io`
  The reworded Display drops "config file" for genuine config paths too; the
  Overview's single-change claim understates the observable surface.

- 🔵 **Compatibility**: `DEFAULT_CORE` derived via `render_value` would gain
  brackets and move the review golden
  **Location**: Phase 4: `DEFAULT_CORE` from the catalogue
  `config::render_value` renders sequences as `[a, b]`; the current
  `DEFAULT_CORE` is a bare comma-joined string. Deriving via `render_value`
  moves the golden — extract scalar items and `join(", ")` instead.

- 🔵 **Test Coverage**: The umask "resolved once" success criterion is not
  observable by any test
  **Location**: Phase 7, Success Criteria
  The `PreserveOr` mode tests check only the resulting file mode; they cannot
  distinguish a once-at-construction read from a per-call read, so reverting to
  per-write computation leaves them green.

- 🔵 **Test Coverage**: No committed golden actually moves in Phase 1
  **Location**: Phase 1, Manual Verification
  The block `config agents` path already coalesces empty; the drift is only on
  the scalar `config agent <name>` path, verified by inline byte asserts (no
  `agent.golden`). The verification step misdescribes the evidence.

- 🔵 **Standards / Code Quality**: New `Source`/`Resolution` types omit the
  standard domain derive set
  **Location**: Implementation Approach / Phase 1
  `const fn source()` needs `Copy` and the Phase 1 tests need `PartialEq`/`Debug`,
  yet no `#[derive]` is specified. Siblings `Level`/`OnFailure` derive
  `Debug, Clone, Copy, PartialEq, Eq`. A literal reading fails to compile.

- 🔵 **Standards**: `core/scalar.rs` breaks the one-module-per-subcommand pattern
  and collides with `config::Scalar`
  **Location**: Phase 3: New scalar assemblers
  Bundling `get`/`path`/`agent`/`work` splits `config agent` from `config agents`
  (`agents.rs`) and `config path` from `config paths` (`paths.rs`), hurting
  discoverability; the name also clashes with the `config::Scalar` value type.

- 🔵 **Correctness**: The four-variant tail taxonomy is incomplete
  **Location**: Current State Analysis: Four behavioural variants
  `config get` (no catalogue) and `config path --default` (explicit-over-
  catalogue) are missing from the taxonomy — the root of the Phase 3 routing
  risks.

- 🔵 **Code Quality**: `review` resolution is left as a parallel helper, not
  folded onto the seam
  **Location**: Phase 4 (vs. Desired End State)
  `core/review.rs` keeps `resolve(config, key, default)`, re-implementing
  precedence+catalogue-fallback that `effective` already does — contradicting the
  Desired End State's "every duplicated tail calls it".

- 🔵 **Code Quality**: Scalar assemblers should inject specific ports, not the
  composite `ConfigStack`
  **Location**: Phase 3: New scalar assemblers
  Passing the 7-port `ConfigStack` over-injects and forces tests to stand up the
  whole stack; block assemblers inject only the ports they use.

- 🔵 **Safety**: The `Vec<u8>` read half must keep erroring on non-UTF-8, not
  decode lossily
  **Location**: Phase 7: `store::read_within` (config-adapters decode)
  Today's `read_within` uses `fs::read_to_string`, which errors on invalid UTF-8.
  Decode with `String::from_utf8` (not `from_utf8_lossy`) so a corrupt file fails
  safely rather than being mangled to U+FFFD and consumed as valid.

- 🔵 **Safety**: Replacing hard-coded path defaults with catalogue lookups
  removes the last-resort literal
  **Location**: Phase 2: Collapse tails (`summary::tmp_dir`,
  `template::templates_dir`)
  A future catalogue key regression would resolve these to `Unset`/`""`, and
  `absolutise("")` collapses to the project root — fail-open. Consider retaining
  the literal as an `Unset` fallback or asserting the catalogue default is
  non-empty.

- 🔵 **Architecture**: Phase 4 has an unstated ordering dependency on Phase 3
  **Location**: Implementation Approach / Phase 4
  Phase 4 change #3 edits `core/scalar.rs` ("moved to scalar.rs in Phase 3"),
  which does not exist until Phase 3 lands — so Phase 4 depends on Phase 3, not
  just Phase 1, contradicting the stated independence.

#### Suggestions

- 🔵 **Code Quality**: `Source::Unset` wrapping `Scalar::Null` is a null-sentinel
  that blurs "unset" and "empty"
  **Location**: Phase 1: `default_resolution`
  `render_value(Null)` is `""`, so an `Unset` resolution renders identically to a
  config empty string; callers must key off `source()`/`from_config()`, not
  `rendered().is_empty()`. Document this on `Source::Unset`, or model unset as
  `Option`-shaped.

- 🔵 **Standards**: `read_within` reuses write-centric `WriteError`/`WriteBounds`
  naming
  **Location**: Phase 7: `store::read_within`
  Now that these types span both halves of the containment contract, consider a
  neutral name (`ContainmentError`/`Bounds`) or note in the plan that the write-
  centric names are intentionally retained.

### Strengths

- ✅ Root-cause framing is sound: collapsing ~10 re-implemented
  resolve→default→render tails onto one domain `Resolution` operation improves
  cohesion and gives domain data a single home without crossing the config
  crate's pup boundary.
- ✅ Behaviour risk is well-contained: exactly one phase (Phase 1) is intended to
  change observable output, and it fixes a genuine correctness drift; every
  other phase is gated on byte-identical committed goldens.
- ✅ The `effective` None-branch correctly preserves fail-loud-on-either-level
  (both `get(Some(..))?` calls sequenced before the match), matching today's
  `get(None)`; read count is unchanged.
- ✅ Modelling empty-collapse as a second named method (`effective_nonempty`)
  rather than a boolean flag avoids the flag-argument smell and keeps each call
  site self-documenting.
- ✅ Phase-by-phase pup boundary reasoning is explicit and correct
  (`Resolution` in config, `store::read_within` in store, `render_node`→
  `config::render_value` within the config subdomain).
- ✅ Phase 5's bidirectional purity restoration (prose out of `core/`, LCS diff
  out of `render/`) is a genuine SRP improvement; the fail-closed refusal stays
  classified at the inbound boundary.
- ✅ Phase 7's umask read-once alignment closes the documented per-write race
  against the process-global umask while leaving the on-disk mode unchanged.
- ✅ Phase 8 shares only the `TEMP_PREFIX` constant, leaving `cache.rs`'s
  write-then-rename and 0600-before-publish semantics untouched.

### Recommended Changes

1. **Specify the scalar-resolution composition explicitly rather than "route
   through `effective`"** (addresses: the `config get` no-catalogue finding, the
   `--default` precedence finding, the incomplete taxonomy) — In Phase 3, state
   that `get` stays on raw `get()` + explicit-default-or-empty (never catalogue),
   and that `path`/`work` use `effective` only when `Resolution::from_config()`
   is true, otherwise apply the explicit `--default` first and fall to
   catalogue/warning only when no `--default` is supplied. Add the two missing
   variants (`get` no-catalogue; `path --default` explicit-over-catalogue) to the
   Current State taxonomy, and pin each with a test (`config get <catalogue-key>`
   unset stays empty; `config path <key> --default X` unset returns `X`).

2. **Keep `--explain` on a per-level probe, or extend `Resolution` to carry
   per-level presence** (addresses: the `--explain` provenance finding) — Drop
   the claim that explain derives solely from `source()`. Either retain the
   per-level `get(Some(..))` probes for the explain lines (using `source()` only
   to attribute the winner), or have the domain expose which levels were Found so
   both the winner and the per-level report are reconstructable.

3. **Fix the `template::scalar` mapping** (addresses: the `Option<String>`
   finding) — Specify that `scalar` maps a non-`from_config()` (`Unset`/
   `Catalogue`) Resolution back to `None`, preserving the override/plugin
   fall-through; note that `effective_nonempty` collapses to catalogue/`Unset`,
   not to `None`.

4. **Rework Phase 7's `read_within` adoption in corpus-adapters** (addresses: the
   containment/lock ordering finding, the UTF-8 decode finding) — Keep a
   standalone `ensure_contained` before `create_dir_all` and use `read_within`
   only for the post-lock read (its internal re-check is a harmless redundancy);
   add a test asserting the containment check precedes any directory creation and
   the read stays inside the lock. Decode config/skill bodies with
   `String::from_utf8` (map error to `ConfigError::Io`), not `from_utf8_lossy`.

5. **Golden-pin the review prose before relocating it** (addresses: the Phase 5
   untested-prose finding) — Before Phase 5's core→render move, add review
   fixtures covering a non-default verdict (`pr_request_changes_severity: none`, a
   custom severity, a non-default `*_revise_major_count`) so every
   `verdict_lines`/`revise_verdict`/`core_lenses_note` branch is golden-pinned.

6. **Reconcile the store-boundary invariant with Phase 8** (addresses: the
   launcher→store edge finding) — Either explicitly amend the "store importable
   only from `*-adapters`" invariant with the const-only-edge justification, or
   re-export `TEMP_PREFIX` through a crate the launcher already depends on so the
   boundary is preserved.

7. **Close the API-specification and framing gaps** (addresses: the derive-set,
   `core/scalar.rs` naming, `DEFAULT_CORE` brackets, `ConfigError::Io` framing,
   Phase 4 ordering, and observability minors) — Add
   `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` to `Source` and derive
   `Debug`/`Clone` on `Resolution`; rename `core/scalar.rs` (e.g. `resolve.rs`)
   or co-locate assemblers with their block siblings; derive `DEFAULT_CORE` by
   joining catalogue scalar items with `", "` (not `render_value`); acknowledge
   the `ConfigError::Io` reword as a second deliberate output change in the
   Overview; state the Phase 3→Phase 4 dependency; and either make the umask
   read-once observable via an injected counter or reclassify it as an untested
   structural change.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally strong: it correctly diagnoses a
single missing domain seam (combined precedence+catalogue resolution) as the
root of ~10 duplicated tails, isolates the one behaviour change to Phase 1, and
moves domain data into the config crate — improving cohesion and evolutionary
fitness while respecting the pup import boundary. Two concerns weaken the seam's
fit: the proposed `Resolution` carries only the winning source (cannot
reconstruct `--explain` per-level provenance), and `effective` bakes catalogue-
defaulting into resolution (conflicts with `config get`'s deliberately raw
semantics). A third concern is that Phase 8 introduces a launcher→store edge
contradicting the plan's own store-boundary invariant.

**Strengths**:
- Root-cause framing is sound — collapsing ~10 re-implemented tails onto one
  domain `Resolution` improves cohesion and gives domain data a single home,
  without crossing the config crate's pup boundary.
- Behaviour risk is well-contained: exactly one phase changes observable output
  and it fixes a genuine drift; every other phase is gated on byte-identical
  goldens.
- Phase 5 (core↔render purity) and Phase 3 (scalar/block symmetry) strengthen
  the functional-core / imperative-shell separation.
- Domain vocabulary alignment is good (`Resolution`, `Source`, `effective`/
  `effective_nonempty`, `Level::filename`, `is_valid_work_integration`).

**Findings**:
- 🟡 MAJOR (high): `Resolution.source()` cannot reconstruct per-level `--explain`
  provenance — Phase 3. `source()` carries only the winner; today's
  `explain_lines` reports set/not-set for each level plus the winner. The
  both-levels-set explain golden cannot be reproduced from `source()` alone.
- 🟡 MAJOR (medium): `effective` bakes catalogue-defaulting into a seam that
  `config get` must not have — Phase 3/1. Uniformly forcing scalar consumers
  through `effective` risks changing `config get` output and inverting explicit-
  default-vs-catalogue precedence.
- 🟡 MAJOR (medium): launcher→store dependency edge contradicts the plan's own
  store-boundary invariant — Phase 8. Re-export `TEMP_PREFIX` through an
  adapter crate, or amend the invariant with justification.
- 🔵 MINOR (high): Phase 4 has an unstated ordering dependency on Phase 3 — it
  edits `core/scalar.rs` which does not exist until Phase 3.

### Code Quality

**Summary**: An unusually well-constructed refactoring plan whose central move
— one domain `Resolution` operation collapsing ~10 duplicated tails — is exactly
the right DRY consolidation, correctly pinned by byte-identical goldens plus
failing-test-first discipline. The two-named-methods design avoids a boolean
flag, and the core↔render purity restoration is a clean SRP win. The main gaps
are minor: the `review` resolution path is left as a parallel helper (undercutting
"every tail calls it"), and Phase 3's scalar-assembler signature is specified
loosely in a way that would over-inject dependencies.

**Strengths**:
- Keystone consolidation is textbook DRY; the four behavioural variants are
  enumerated so none is silently dropped.
- Modelling empty-collapse as `effective_nonempty` avoids the flag-argument
  smell.
- Phase 5's bidirectional purity restoration is a genuine SRP improvement.
- `Resolution` encapsulates fields behind accessors; phases are independently
  mergeable.

**Findings**:
- 🔵 MINOR (medium): `review` resolution left as a parallel helper, not folded
  onto the new seam — Phase 4. `resolve(key, catalogue_default)` re-implements
  what `effective` already does; contradicts the Desired End State.
- 🔵 MINOR (medium): scalar assemblers should inject specific ports, not the
  composite `ConfigStack` — Phase 3. Over-injection weakens test isolation.
- 🔵 SUGGESTION (low): `Source::Unset` wrapping `Scalar::Null` is a null-sentinel
  blurring "unset" and "empty" — Phase 1.
- 🔵 SUGGESTION (low): `Source` enum sketch omits the derives its own API and
  success criteria require — Implementation Approach.

### Correctness

**Summary**: The keystone `effective`/`effective_nonempty` operation is sound
and correctly preserves fail-loud-on-either-level (both levels read eagerly
before matching, matching `get(None)`), and the agent-drift fix and
`Scalar::Null`→empty rendering check out. However, the plan under-specifies how
the operation composes with three call sites whose tail is NOT plain catalogue-
backed defaulting: `config get` applies no catalogue default at all,
`get`/`path` honour a caller-supplied `--default` that outranks the catalogue,
and `--explain` must report both levels. Routing these through
`effective(...).rendered()` unconditionally would change observable output and
break the byte-identical guarantee.

**Strengths**:
- The `effective` None-branch correctly preserves fail-loud-on-either-level;
  read count unchanged.
- The empty-collapse policy is faithfully modelled; `render_value(Scalar::Null)`
  verifiably yields `""`.
- The agent-drift fix is correct: both paths obtain the prefixed default from
  `catalogue::default_for` via the domain.
- `review::resolve` correctly left out of Phase 2's collapse; the hard-coded
  literals genuinely equal their catalogue values.

**Findings**:
- 🔴 MAJOR (high): `resolve_get` applies NO catalogue default; routing through
  `effective` changes output — Phase 3. `config get review.max_lenses` would
  return `8` where it returns empty today.
- 🟡 MAJOR (high): explicit `--default` precedence dropped for `path`/`get` —
  Phase 3. `config path tmp --default X` (absent) would return `.accelerator/tmp`
  instead of `X`. Branch on `from_config()`.
- 🟡 MAJOR (high): `--explain` provenance can't be built from `source()` alone —
  Phase 3. Keep the per-level probe for building explain lines.
- 🟡 MAJOR (medium): `template::scalar` `Option<String>` contract broken by
  `effective_nonempty` — Phase 2. For `templates.<name>` keys with no catalogue
  default, `effective_nonempty` yields `Unset`/`""`, not `None`; map non-
  `from_config()` back to `None`.
- 🔵 MINOR (medium): four-variant taxonomy incomplete (`get` no-catalogue, `path
  --default`) — Current State. Root of the Phase 3 routing risks.

### Test Coverage

**Summary**: The plan is unusually test-conscious: a concrete regression
contract (byte-identical goldens), TDD-first framing, and per-phase pinning of
subtleties. The existing CLI suite is a strong byte-exact gate covering most of
the surface. The main gap is Phase 5's move of review verdict/revise prose from
core to render: those branches are exercised by no fixture (baseline goldens use
only default severities → empty-output branch), so a behaviour-neutral move has
effectively zero regression protection. A secondary weakness is Phase 7's umask
"resolved once" criterion, which is not observable by any test.

**Strengths**:
- Establishes an explicit, falsifiable regression contract, reasserted per phase.
- TDD-first framing is concrete: Phase 1 specifies a failing test before the fix.
- The three preserved scalar subtleties in Phase 3 are each pinned by a named
  test; the existing suite already covers fail-closed and `--explain`.
- `effective`/`effective_nonempty` unit coverage is well-specified.

**Findings**:
- 🟡 MAJOR (high): review verdict/revise prose branches are unexercised, so the
  core→render move has no regression protection — Phase 5. Add non-default-
  verdict fixtures before the move.
- 🔵 MINOR (high): the umask "resolved once" guarantee is not observable by any
  test — Phase 7. `PreserveOr` mode tests can't distinguish once-vs-per-call.
- 🔵 MINOR (high): no committed golden actually moves in Phase 1; the fixed
  scalar path has no golden — Phase 1. Verification step misdescribes the
  evidence.

### Compatibility

**Summary**: The plan is unusually disciplined about its compatibility
contract: it defines the gate as byte-identical committed goldens, scopes the
single deliberate change to Phase 1, and documents it in Migration Notes. The
trait-level additions (`effective`/`effective_nonempty`, `Source`) are additive
and dyn-compatible with a single in-tree implementor, so no external-consumer
risk. The material risk is that the taxonomy omits `config get`, and generalises
scalar resolution onto `effective` — which unconditionally injects the catalogue
default. Applied naively to `get` (and the `--default` precedence of
`path`/`work`), that silently changes observable output.

**Strengths**:
- The regression contract is stated precisely and pinned to a concrete gate.
- The one deliberate behaviour change is isolated to Phase 1 and recorded in
  Migration Notes.
- The new trait methods and `Source` enum are additive; only `ConfigService`
  implements the trait; signatures are object-safe.
- Phase 4's catalogue-sourced review defaults were checked against the catalogue
  literals and agree; the mode-specific work-item `min_lenses "3"` is correctly
  kept as a launcher literal.

**Findings**:
- 🔴 MAJOR (medium): `config get` has no catalogue fallback; routing through
  `effective` changes output — Phase 3. Keep `get` on raw `get()` +
  explicit-or-empty.
- 🟡 MAJOR (medium): `path_fallback`/`work_fallback` give explicit `--default`
  precedence over the catalogue; `effective` drops it — Phase 3.
- 🔵 MINOR (medium): `DEFAULT_CORE` derived via `render_value` gains brackets
  `[a, b]` and moves the review golden — Phase 4. Join scalar items with `", "`.
- 🔵 MINOR (high): the `ConfigError::Io` reword is a second observable output
  change, contradicting the "one phase" framing — Phase 8.

### Standards

**Summary**: The plan is strongly convention-aware: it reasons about pup import
boundaries per phase, honours the no-comments and no-staleness-in-docs policies
(min_lenses left uncommented; Phase 8 strips the exit-code and reference cruft),
applies `#[must_use]`, and re-exports the new identifier helper from `lib.rs` in
the established style. Two naming/placement decisions are worth flagging: the new
`Source`/`Resolution` types omit the derive set every sibling domain enum carries
(and `const fn source()` needs `Copy`), and folding all four scalar subcommands
into a single `core/scalar.rs` breaks the one-module-per-subcommand pattern while
colliding with the existing `config::Scalar` type.

**Strengths**:
- Phase-by-phase pup boundary reasoning is explicit and correct.
- Strong adherence to the low-comment / no-staleness convention (Phase 8 removes
  the banned exit-code assertion and ADR-style framing).
- The new `validate_identifier` helper follows the module + `lib.rs` re-export
  convention; `#[must_use]` applied consistently.
- Byte-identical committed goldens keep observable-output conventions frozen.

**Findings**:
- 🔵 MINOR (medium): new `Source`/`Resolution` types omit the standard domain
  derive set — Phase 1. `const fn source()` needs `Copy`; tests need `PartialEq`/
  `Debug`. Specify `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`.
- 🔵 MINOR (medium): `core/scalar.rs` breaks the one-module-per-subcommand
  pattern and collides with `config::Scalar` — Phase 3. Co-locate with block
  siblings or rename (`resolve.rs`).
- 🔵 SUGGESTION (low): `read_within` reuses write-centric `WriteError`/
  `WriteBounds` naming — Phase 7. Consider a neutral containment name or note the
  intentional retention.

### Safety

**Summary**: A behaviour-neutral refactoring of a developer-facing CLI whose
write paths (config files, corpus JSONL, binary cache) are all recoverable via
VCS and already carry strong protective mechanisms — atomic whole-file
replacement, symlink-escape containment refusal, and caller-supplied file modes.
The regression contract and the explicit requirement that `store::read_within`
retain the symlink-escape refusal are sound. The one real hazard is Phase 7's
consolidation of the containment check and the tolerant read into a single
`store::read_within` call in corpus-adapters: the two operations must sit on
opposite sides of the mkdir/lock in `append_record`.

**Strengths**:
- The regression contract fails the gate on accidental drift rather than
  shipping silently.
- Phase 1's `effective`/`effective_nonempty` preserves fail-loud-on-either-
  malformed-level, pinned by a unit test.
- Phase 7 requires `store::read_within`'s tests to include a symlink-escaping
  refusal mirroring `atomic_write`.
- Phase 7's umask read-once alignment closes the documented per-write race; the
  on-disk mode is unchanged.
- Phase 8's `cache.rs` change shares only `TEMP_PREFIX`, leaving write-then-
  rename and 0600-before-publish semantics untouched.

**Findings**:
- 🟡 MAJOR (high): `read_within` bundles containment + read, but they must
  straddle the mkdir/lock in `append_record` — Phase 7. A naive merge either
  weakens the symlink-containment invariant or introduces a lost-append race.
  Keep a standalone `ensure_contained` before `create_dir_all`; use
  `read_within` only for the post-lock read.
- 🔵 MINOR (medium): the `Vec<u8>` read half must keep erroring on non-UTF-8, not
  decode lossily — Phase 7. Use `String::from_utf8`, not `from_utf8_lossy`.
- 🔵 MINOR (medium): replacing hard-coded path defaults with catalogue lookups
  removes the last-resort literal — Phase 2. `Unset`/`""` + `absolutise("")`
  collapses to the project root (fail-open). Retain the literal as an `Unset`
  fallback or assert the catalogue default is non-empty.

## Re-Review (Pass 2) — 2026-07-22

**Verdict:** REVISE

The revision resolved the dominant Phase-3 scalar-routing theme cleanly (all
three lenses that raised it confirm `config get` now correctly stays off
`effective`, `path`/`work` branch on `from_config()`, and `--explain` keeps its
per-level probe) and the Phase-7 containment/lock ordering hazard is now
correctly specified (safety confirms containment-before-mkdir / read-after-lock).
The plan's spine is sound. However, the deeper second-pass scrutiny surfaced a
cluster of narrower edge-case regressions the first pass did not reach — an
unknown agent name now rendering empty, an empty `paths.templates` losing its
collapse, and an unspecified corpus UTF-8 re-decode — plus structural coverage
gaps (stderr warnings, the `(default: …)` core-lenses line, and several verdict
branches) that the stdout-only golden contract cannot see. Five new majors keep
the verdict at REVISE.

### Previously Identified Issues

- 🟡 **Architecture / Correctness / Compatibility**: `config get` routed through
  `effective` injects a catalogue default — **Resolved**. `get` stays on raw
  `get()`; confirmed by three lenses.
- 🟡 **Correctness / Compatibility**: explicit `--default` precedence dropped for
  `get`/`path` — **Resolved**. `path`/`work` branch on `from_config()`. (A finer
  empty-`--default` predicate distinction between `get` and `path` is newly noted
  as a minor.)
- 🟡 **Architecture / Correctness**: `--explain` provenance not reconstructible
  from `source()` — **Resolved**. The per-level probe is retained.
- 🟡 **Correctness**: `template::scalar`'s `Option<String>` contract — **Partially
  resolved**. `scalar` now maps non-`from_config()` back to `None`, but its
  sibling `templates_dir` is still routed through the non-collapsing `effective`
  (new major below).
- 🟡 **Safety**: `store::read_within` containment/lock ordering — **Resolved**.
  Standalone `ensure_contained` before `create_dir_all`, `read_within` post-lock;
  safety confirms the ordering is preserved.
- 🟡 **Test Coverage**: Phase 5 review prose exercised by no fixture — **Partially
  resolved**. The golden-pin-first criterion was added, but the branch
  enumeration is incomplete and Phase 4's `(default: …)` line remains uncovered
  (new majors below).
- 🟡 **Architecture**: launcher→store dependency contradicts the store-boundary
  invariant — **Resolved**. The invariant was amended to permit the const-only
  edge (architecture now only queries whether the coupling is spurious — a new
  minor).
- 🔵 **Compatibility**: `ConfigError::Io` reword framing — **Resolved**. The
  Overview now enumerates it as a second deliberate change.
- 🔵 **Compatibility**: `DEFAULT_CORE` bracket form — **Resolved as specified**
  (join with `", "`), though the line is untested (new major below).
- 🔵 **Test Coverage**: umask "resolved once" not observable — **Addressed**
  (reclassified as structural); test-coverage still recommends landing the
  counted-port seam now rather than deferring.
- 🔵 **Test Coverage**: no golden moves in Phase 1 — **Resolved**.
- 🔵 **Standards / Code Quality**: `Source`/`Resolution` derive set — **Resolved**.
- 🔵 **Standards**: `core/scalar.rs` naming/placement — **Partially resolved**.
  Change #1 co-locates the assemblers, but stray `core/scalar.rs` references
  remain in Change #2 and Manual Verification (new minor below).
- 🔵 **Safety**: non-UTF-8 decode (config-adapters) — **Resolved** (`from_utf8`);
  but corpus `remove_by_key` is not specified (new major below).
- 🔵 **Safety**: catalogue lookup removes last-resort literal — **Addressed** via a
  guard test; safety adds a destructive-call-site fail-safe note.
- 🔵 **Architecture**: Phase 3→4 ordering dependency — **Resolved**.
- 🔵 **Code Quality**: `review::resolve` parallel helper — **Resolved** (noted as a
  deliberate exception).
- 🔵 **Code Quality**: assemblers inject `ConfigStack` — **Resolved** (take only
  the ports they use).

### New Issues Introduced

- 🟡 **Correctness** (major, high): `config agent <name>` for a name outside
  `AGENT_KEYS` now renders empty (was `accelerator:<name>`) — Phase 1.
  `catalogue::default_for` returns `None` for unknown names → `Source::Unset` →
  `""`, but today `resolve_agent` prefixes any name unconditionally on absence.
  The agreement test only exercises `reviewer` (a known key), so the gate misses
  it. Fall back to `format!("{}{name}", catalogue::AGENT_PREFIX)` when the
  resolution is not `from_config()`.
- 🟡 **Correctness** (major, high): `templates_dir` routed through
  non-collapsing `effective` drops the empty-`paths.templates` collapse — Phase 2.
  `templates_dir` delegates to `scalar` today (which collapses `Found("")` →
  `None` → default); `effective` renders `""`, so `paths.templates: ""` would
  newly resolve to an empty directory. Use `effective_nonempty` for
  `templates_dir`.
- 🟡 **Test Coverage / Compatibility** (major, high): relocated stderr warnings
  (`legacy_alias_warning`, `unknown_path_key_warning`, `--explain`) are uncovered
  by the stdout-only golden contract — Phase 3. A behaviour-neutral relocation
  could drop/reword a user-facing migration warning with the golden gate green.
  Add stderr byte-assert characterization tests and state the golden contract
  covers stdout only.
- 🟡 **Compatibility** (major, high): the `Core lenses` / `(default: …)` line is
  emitted by no baseline golden, so Phase 4's `DEFAULT_CORE` rewrite and Phase 5's
  `core_lenses_note` relocation are asserted-but-untested — Phase 4/5. Add a
  fixture that sets `review.core_lenses` with a committed golden pinning the exact
  bytes, landing in/before Phase 4.
- 🟡 **Safety** (major, medium): corpus `remove_by_key`'s UTF-8 re-decode is
  unspecified — Phase 7. It reads with `read_to_string` today (fail-loud); a lossy
  `from_utf8_lossy` on the `read_within` bytes would silently corrupt the store on
  the subsequent atomic rewrite. Specify `String::from_utf8` and add an
  invalid-UTF-8 test asserting the file is left byte-identical.
- 🔵 **Code Quality / Standards** (minor): stray `core/scalar.rs` references in
  Phase 3 Change #2 and Manual Verification contradict the co-located Change #1
  (`get.rs`/`work.rs`/`agents.rs`/`paths.rs`).
- 🔵 **Architecture / Code Quality** (minor): `effective`/`effective_nonempty` are
  added as required `ConfigAccess` port methods though they are pure derivations
  of `get` + `catalogue`; a second implementor/test-double must re-derive them.
  Prefer default trait methods or free functions over `&dyn ConfigAccess`.
- 🔵 **Correctness** (minor): empty `--default` differs between `get`
  (`unwrap_or_default` → `""`) and `path` (`filter(!is_empty)` → falls through);
  preserve each predicate verbatim on extraction and pin with a test.
- 🔵 **Architecture** (minor): fail-safe policy is increasingly coupled to error-
  variant classification (`Invalid ⇒ Refusal`) as refusals migrate into `core/`.
- 🔵 **Suggestions**: `from_config()` → `is_`-prefixed to match the crate
  convention; soften Phase 7's umask "closes the race" to "narrows" (the global
  set/restore window remains); reserve "failing-test-first" for Phase 1 and label
  behaviour-neutral pins as characterization tests; grep skills/hooks for the old
  `ConfigError::Io` wording before rewording; note that any destructive consumer
  of a resolved path must reject an empty/`Unset` resolution.

### Assessment

The plan is materially stronger: the structural theme that drove the first
REVISE is resolved, and the safety-critical Phase-7 ordering is now correct. What
remains is a tighter, more mechanical set — three genuine edge-case regressions
(unknown agent name, empty templates path, corpus UTF-8), two output-surface
coverage gaps the golden contract structurally cannot see (stderr warnings, the
core-lenses default line), and a scatter of naming/design polish. None require
rethinking the approach; each is a localised specification fix. One more revision
pass addressing the five new majors should reach APPROVE.

---
*Re-review generated by /accelerator:review-plan*

## Re-Review (Pass 3) — 2026-07-22

**Verdict:** REVISE

All five pass-2 majors are resolved: the agent unknown-name fallback is
preserved, `templates_dir` uses `effective_nonempty`, corpus `remove_by_key`
decodes fail-loud, the `DEFAULT_CORE`/core-lenses golden is added, and the stderr
warning surface is acknowledged. `effective`/`effective_nonempty` are now provided
default methods, and the `is_from_config()` rename landed. The four residual
majors are qualitatively different from earlier rounds — **none is a logic defect
in the plan**; three are test-coverage gaps and one is an API-safety hardening.
The trajectory (7 → 5 → 4 majors, each round narrower and less structural)
indicates a plan essentially ready pending a final round of test-hardening.

### Previously Identified Issues

- 🟡 **Correctness / Compatibility**: `config agent` unknown-name regression —
  **Resolved**. The prefixed fallback is preserved for non-`is_from_config()`
  results; a finer explicit-empty-unknown-name edge is newly noted as a minor.
- 🟡 **Correctness**: `templates_dir` empty-collapse — **Resolved**
  (`effective_nonempty`); test-coverage adds a minor to also test a config-empty
  `templates.<name>` fall-through.
- 🟡 **Test Coverage / Compatibility**: relocated stderr warnings uncovered —
  **Partially resolved**. A per-helper byte-assert criterion was added; compat
  now escalates to pinning warning *order* exhaustively (new major below).
- 🟡 **Compatibility**: `DEFAULT_CORE`/core-lenses line untested — **Resolved**
  (fixture + golden added in Phase 4).
- 🟡 **Safety**: corpus `remove_by_key` UTF-8 re-decode — **Resolved**
  (`String::from_utf8` + invalid-UTF-8 test); the *config-adapters* twin lacks the
  same test (new major below).
- 🔵 Minors from pass 2 (stray `core/scalar.rs` ref, default trait methods, empty
  `--default` predicate, `from_config`→`is_from_config`, umask wording,
  characterization-test labelling, Phase 8 grep, destructive-path note) — **all
  Resolved**.

### New Issues Introduced

- 🟡 **Test Coverage** (major, high): config-adapters `read_within` fail-loud
  UTF-8 decode is asserted-but-untested — Phase 7. The corpus twin gets an
  invalid-UTF-8 test; the config path (the *safer* of the two) does not. Add a
  symmetric test feeding `config_body`/`read_skill_file` invalid UTF-8 and
  asserting it errors rather than lossily decoding.
- 🟡 **Compatibility** (major, medium): the observable contract includes stderr,
  but committed goldens gate stdout only and the existing stderr assertions are
  `contains(...)`/non-empty, not byte-exact — Phase 3. Relocating the warning
  helpers could reword or reorder the legacy-alias → fallback → explain sequence
  undetected. Make the per-helper byte-assert tests exhaustive and pin warning
  *order* for the multi-warning path.
- 🟡 **Safety** (major, medium): the containment-before-`create_dir_all` ordering
  has no regression test — Phase 7. The only symlink test is leaf-only (caught by
  `atomic_write` regardless); nothing pins the pre-mkdir guard. Add a test that
  `append_record`/`remove_by_key` refuse a symlinked *intermediate* component and
  create no directory outside the root.
- 🟡 **Code Quality** (major, medium; echoed by architecture + safety as minors):
  `Resolution::rendered()` collapses `Unset`, `Catalogue`, and config-empty all to
  `""`, re-enabling the empty-vs-absent footgun class the refactor exists to kill.
  Add a type-level accessor (e.g. `configured_value() -> Option<&str>`, `None` for
  non-config sources) so `rendered().is_empty()` cannot silently reintroduce the
  bug.
- 🔵 **Suggestions**: name the empty-collapse policy more explicitly (`effective`
  vs `effective_nonempty`); `Resolution` vs the existing `Resolved` are
  near-homonyms (doc the distinction); commit characterization tests in a separate
  green-first commit within each PR; pin the co-located scalar-assembler function
  names; state the intended linear order of Phases 2→3→4→5.

### Assessment

The plan is in strong shape and the approach is settled. Every remaining major is
mechanical: three add tests (config-adapters UTF-8, stderr order/byte-exactness,
containment-before-mkdir) and one adds a safer accessor to the `Resolution` type.
None requires reworking the design or revisiting a decision. One more pass
applying these should reach APPROVE.

---
*Re-review generated by /accelerator:review-plan*

## Re-Review (Pass 4) — 2026-07-22

**Verdict:** COMMENT

All four pass-3 majors are resolved (the `configured_value()` accessor, the
config-adapters UTF-8 test, the containment-before-`create_dir_all` test, and the
byte-exact/ordered stderr criterion). The major count has fallen 7 → 5 → 4 → 1.
The single remaining major is a test-coverage gap (an unexercised dump branch),
below the REVISE threshold; everything else is a minor or a suggestion. The plan
is **acceptable as-is** — the items below would improve it but do not block
implementation. The one item that should be actioned regardless is the missing
`# Errors` rustdoc sections, since `mise run cli:check` (clippy pedantic,
`warnings = deny`) will fail without them.

### Previously Identified Issues

- 🟡 **Code Quality / Architecture / Safety**: `Resolution::rendered()` empty-vs-
  absent footgun — **Resolved**. `configured_value() -> Option<String>` added and
  the agent/template consumers routed through it; residual concern is now a
  suggestion (a doc-warning on `rendered()` itself).
- 🟡 **Test Coverage**: config-adapters UTF-8 fail-loud untested — **Resolved**
  (symmetric invalid-UTF-8 test added).
- 🟡 **Safety / Test Coverage**: containment-before-mkdir untested — **Resolved**
  (intermediate-symlink refusal test added; safety confirms the ordering is
  preserved).
- 🟡 **Compatibility**: stderr byte-identity/order not gated — **Resolved**
  (byte-exact, exhaustive, order-pinning criterion added).
- 🔵 Pass-3 suggestions (empty-collapse policy doc, `Resolution`/`Resolved`
  distinction, green-first commit, scalar-assembler names, phase ordering) — **all
  Resolved**.

### New / Residual Issues

- 🟡 **Test Coverage** (major, medium): dump's invalid-`work.integration`
  annotation branch (`"{value} (invalid: must be …)"`) is refactored by Phase 4
  (validator swap) and Phase 5 (string relocation) but exercised by no committed
  golden — the dump fixture uses a valid integration. Add a dump fixture with an
  invalid value and a golden pinning the exact cell bytes, mirroring the
  branch-audit discipline already applied to the review verdict branches.
- 🔵 **Standards** (minor): the new fallible public items (`effective`/
  `effective_nonempty`, `validate_identifier`, `store::read_within`) omit the
  `# Errors` rustdoc sections that clippy pedantic (`missing_errors_doc`, warnings
  = deny) enforces — they will fail `cli:check` without them. Add `# Errors`
  sections matching the adjacent `get`/`set` docs.
- 🔵 **Correctness** (minor): Phase 2 says `config_get` "calls `effective`", but
  that shared helper also feeds `optional_row`/`extra_row`, whose present-vs-absent
  contract must survive. State that `config_get` preserves its raw present/absent
  signal (keep it on raw `get()`/`configured_value()`; switch only
  `defaulted_row`/`work_row` to `effective`) so the invariant is in the code, not
  implicit in which keys happen to lack a catalogue default.
- 🔵 **Architecture** (minor): Phase 5 moves `push_line` (which emits the final
  `+`/`-`/space prefix + newline) into `core/template.rs`, so core would emit
  formatted bytes — inverting the very core/render rule the phase establishes.
  Have core return a structured diff (kind + text per line) and let render format
  the prefix/newline.
- 🔵 **Test Coverage** (minor): apply the same pre-move branch audit to the
  summary core↔render inversion (the largest Phase 5 restructuring) — enumerate its
  body states and byte-pin any unexercised combination before inverting.
- 🔵 **Compatibility** (minor): enumerate the `subagent_type` consumers of `config
  agent` (three shipped review skills) in Migration Notes and note the change is
  safe (an empty `subagent_type` was already broken), making the downstream
  assessment explicit.
- 🔵 **Suggestions**: add a one-location "resolution idiom chooser" doc table;
  a byte-exact assertion on the blank-path note before relocating it; a narrow pup
  rule enforcing the const-only launcher→`store` edge; consider `Resolution` →
  `Effective` to remove the `Resolved` near-homonym; a CHANGELOG/version note for
  the two deliberate output changes; a doc-warning on `rendered()` steering
  destructive callers to `configured_value()`.

### Assessment

The plan is ready to implement. The approach has been stable across the last two
passes; the remaining items are localised test-additions, doc sections, and small
layer-purity/clarity refinements — none touches a decision or the phase structure.
Recommended before starting: add the `# Errors` rustdoc (else `cli:check` fails),
the dump invalid-integration golden, and the `config_get` raw-contract note; the
rest are polish that can be folded in during implementation.

---
*Re-review generated by /accelerator:review-plan*

## Verdict Override — 2026-07-22

**Verdict:** APPROVE (author decision)

The pass-4 verdict was COMMENT (one below-threshold test-coverage major, the rest
minors/suggestions). The three highest-value items were then applied to the plan —
the `# Errors` rustdoc requirement, the dump invalid-integration golden, and the
`config_get` raw-contract clarification — clearing the single major and the
CI-blocking standards minor. The author has accepted the plan and marked it
**ready for implementation**; the remaining pass-4 minors and suggestions (LCS
structured-diff in core, the summary branch audit, the `subagent_type` Migration
Notes enumeration, the resolution-idiom chooser table, a pup rule for the
launcher→`store` edge, the `Resolution`→`Effective` rename, a CHANGELOG note, and
a `rendered()` doc-warning) are recorded above and may be folded in during
implementation.

---
*Verdict override recorded by /accelerator:review-plan*
