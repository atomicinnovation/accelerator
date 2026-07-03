---
type: plan-review
id: "2026-07-02-0163-scaffold-cli-workspace-version-subcommand-review-1"
title: "Plan Review: Scaffold the cli/ Hexagonal Workspace with a version Subcommand"
date: "2026-07-02T23:47:36+00:00"
author: Toby Clemson
producer: review-plan
status: complete
parent: "work-item:0163"
target: "plan:2026-07-02-0163-scaffold-cli-workspace-version-subcommand"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, standards, compatibility, portability]
review_number: 1
review_pass: 2
tags: [rust, cli, hexagonal, scaffold, version, kernel]
last_updated: "2026-07-03T00:11:47+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Scaffold the cli/ Hexagonal Workspace with a version Subcommand

**Verdict:** REVISE

The plan is architecturally excellent and genuinely test-first: it faithfully
realises ADR-0053/0054's hexagon (pure domain core behind inbound/outbound ports,
composition root, cargo-pup enforcement), factors behaviour into pure directly-testable
seams, and decomposes into three independently-green phases. It is not far from ready.
The reason for REVISE is a cluster of major findings that converge on two concrete
themes: the AC-2 git-metadata testing strategy is internally contradictory (the
integration test's `assert_ne!(field, "unknown")` assumes a git tree, directly
contradicting AC-2's own git-less degradation contract), and the vergen-gitcl build
coupling has an unpinned version edge plus an undocumented git-CLI prerequisite. A
secondary standards conflict — carrying comments verbatim against the plan's own
no-comments policy — and a reachability question on the AC-5 error arm round out the
must-fix set. None are structural; all are addressable with targeted edits.

### Cross-Cutting Themes

- **AC-2 git-fact degradation is proven fragilely and self-contradictorily**
  (flagged by: test-coverage, correctness, portability) — Three distinct gaps in
  the same AC: `or_unknown` is unit-tested in isolation but nothing verifies the
  three accessors actually route through it; the plan's description of the
  integration test drops the load-bearing `assert_eq!(field, option_env!(...))`
  reconciliation; and the `assert_ne!(field, "unknown")` guard bakes in a
  "git tree always present" assumption that contradicts the very git-less build
  AC-2 declares supported. Together these mean the one behaviour AC-2 exists to
  protect is not deterministically guarded, and a correct degraded build reports
  as a test failure.

- **vergen-gitcl build coupling is under-pinned and under-documented**
  (flagged by: compatibility, portability) — `vergen` is pinned exactly `=9.0.6`
  because a transitive `vergen-lib` bump breaks the `Emitter`, but the sibling
  `vergen-gitcl = "1"` is an open caret sharing that same `vergen-lib` — a
  `cargo update` could float it and recreate the break. Separately, `vergen-gitcl`
  shells out to the `git` CLI binary at build time, an undocumented build-host
  prerequisite distinct from merely having a `.git` tree.

- **AC-5 error-arm reachability is unverified**
  (flagged by: correctness, test-coverage) — The whole "real (not uninhabited)
  kernel error taxonomy" justification rests on `EnvFilter::parse()` returning
  `Err` for the chosen malformed strings, but the parser is lenient, this is
  net-new code with no luminosity precedent, and the unit test (`"=not a filter="`)
  and integration test (`":::"`) use different, cross-unchecked inputs.

- **Comment carry-over vs the plan's own comment policy**
  (flagged by: standards, code-quality) — The plan directs luminosity comments
  (build.rs doc-comment, pup.ron ADR-0053 header) be carried verbatim, in direct
  tension with the create-plan instructions governing this plan that ban
  code-restating comments and ADR/work-item references in comments.

### Tradeoff Analysis

- **Dependency-light kernel vs error-chain fidelity**: code-quality notes
  `kernel::Error::LogFilter(String)` flattens the underlying parse error, losing
  the `std::error::Error` source chain the server precedent preserves via
  `#[source]`/`#[from]`; architecture wants `kernel` kept dependency-light.
  Recommendation: `#[from] ParseError` adds no new dependency (the type already
  comes with `tracing-subscriber`) and preserves the chain — take it, it satisfies
  both.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Test Coverage**: AC-2 tests the `or_unknown` helper in isolation, not the accessor wiring
  **Location**: Phase 2 §5 (or_unknown); Testing Strategy — Unit Tests
  Nothing verifies each `VergenBuildMetadata` accessor actually routes its
  `option_env!("VERGEN_*")` through `or_unknown` or reads the correct key; a
  bypass/wrong-key regression passes every test in a git tree.

- 🟡 **Correctness**: Plan omits the `assert_eq!`-against-`option_env!` reconciliation that makes git-fact asserts deterministic
  **Location**: Phase 3 §2 (black-box integration test)
  The luminosity source the plan replicates first does
  `assert_eq!(field, option_env!("VERGEN_*").unwrap_or("unknown"))` — the
  load-bearing line — which the plan's prose drops, so an implementer following
  the text could omit it and misjudge the git-less failure mode.

- 🟡 **Portability**: Integration test assumes a git tree + git binary on every build host, contradicting AC-2
  **Location**: Phase 3 §2 (`assert_ne!(field, "unknown")`)
  Green on current GitHub runners but fails on shallow/git-less/sandboxed
  builders (`cargo install` from tarball, Nix/Bazel, minimal containers) — the
  exact environments AC-2 declares supported.

- 🟡 **Correctness**: Malformed-filter error path reachability depends on unverified `EnvFilter` rejection
  **Location**: Phase 1 §4 (filter_from_env tests) & Phase 3 §2 (malformed-filter case)
  `tracing-subscriber`'s parser is deliberately lenient; if `":::"` /
  `"=not a filter="` parse without error, the `kernel::Error::LogFilter` arm is
  unreachable and AC-5 collapses to the never-arm it claims to have eliminated.

- 🟡 **Compatibility**: `vergen-gitcl = "1"` caret can float a `vergen-lib` bump that breaks the pinned `vergen =9.0.6`
  **Location**: Phase 1 §1 ([workspace.dependencies])
  The two crates share `vergen-lib`; a `cargo update` on `vergen-gitcl` alone
  could recreate the exact Emitter break the exact `vergen` pin exists to prevent
  (vetted pair is 9.0.6 / gitcl 1.0.8).

- 🟡 **Portability**: `vergen-gitcl` shells out to the `git` CLI at build time — undocumented build-host prerequisite
  **Location**: Phase 2 §2 (build script) / Overview ("git-less/shallow build")
  The plan conflates "no `.git` dir" with "no `git` binary"; both degrade to
  `unknown`, but the `git`-CLI-on-PATH requirement for a real SHA is never stated
  as a prerequisite and no manual step exercises the git-absent path.

- 🟡 **Standards**: Verbatim comment carry-over conflicts with the plan's own no-comments / no-ADR-ref policy
  **Location**: Phase 1 §1, Phase 2 §2 (build.rs), Phase 3 §3 (pup.ron header naming ADR-0053)
  The create-plan instructions governing this plan ban code-restating comments and
  ADR/work-item references in comments; the plan directs several be carried verbatim.

- 🟡 **Test Coverage**: `render` unit test asserts only "four values present," not exact prefixes/ordering
  **Location**: Phase 2 §6 (render test)
  The fixed per-field prefixes and line ordering are only checked in the black-box
  integration test; a swapped prefix or reordered line passes the unit test.

#### Minor

- 🔵 **Architecture**: pup rule grants the domain core import access to the *entire* kernel crate (`^kernel(::|$)`), not just the error taxonomy
  **Location**: Phase 3 §3 (pup.ron)
  As kernel grows (config-access, dispatch contracts in later stories), `core`
  could import infrastructure-flavoured kernel modules and the rule would not
  catch it.

- 🔵 **Architecture**: Launcher crate named `launcher` diverges from ADR-0054's `cli`; deviation is in the work item but not the plan
  **Location**: Overview / Current State Analysis
  A reader working from the plan alone sees an unexplained mismatch with the
  accepted ADR.

- 🔵 **Architecture**: Outbound `BuildMetadata` port returns `&'static str` while `VersionReport` holds owned `String`s
  **Location**: Phase 2 §4/§5
  The `'static` is a compile-time-source artefact leaking into the port; a
  runtime-sourced metadata adapter later would force a port signature change.

- 🔵 **Code Quality / Correctness**: `init` swallows *all* `try_init()` failures via `let _ =`, not just already-initialised
  **Location**: Phase 1 §4 (logging init)
  Since `main` inits exactly once, the benign case can't occur for the only real
  caller; a genuine install failure is silently discarded and `init` still returns
  `Ok`, and the bare `let _ =` reads as an accidental ignore.

- 🔵 **Code Quality**: `dispatch` returns `Result<(), kernel::Error>` but can never `Err` for the only compiled command
  **Location**: Phase 2 §6 / Phase 3 §1
  Forward-looking scaffolding; invites a search for a failing path in `dispatch`
  that doesn't exist (the only reachable error is `init` in `run`).

- 🔵 **Code Quality**: `render` field-to-prefix-to-line mapping is implicit and positional
  **Location**: Phase 2 §6 (render)
  Reordering/adding a field silently shifts line indices; the coupling between
  `render` and index-based consumers is fragile.

- 🔵 **Code Quality**: `kernel::Error::LogFilter(String)` discards the `ParseError` source chain
  **Location**: Phase 1 §3 (kernel error taxonomy)
  Diverges from the server precedent (`#[source]`/named fields); `#[from]` would
  preserve the chain with no new dependency.

- 🔵 **Test Coverage**: Unit and integration malformed-filter tests use different, cross-unchecked inputs
  **Location**: Phase 1 §4 vs Phase 3 §2
  Use one verified-rejected string at both levels and assert the specific
  `Error::LogFilter` variant, not just `is_err()`.

- 🔵 **Test Coverage**: `kernel::Error` "satisfies std::error::Error (compile-time)" is listed as a test but none is specified
  **Location**: Testing Strategy — Unit Tests
  Overstates coverage; the `Display` message contract the integration test greps
  (`invalid log filter`) is pinned only by that one integration assertion.

- 🔵 **Test Coverage**: No automated negative for a quiet default-level run (only in Manual Verification)
  **Location**: Phase 3 §2 (log-line case)
  A regression emitting the version log at `info!` (or dropping the filter) would
  pass the positive assertion, silently making the CLI noisy on every invocation.

- 🔵 **Correctness**: Phase 2 leaves `dispatch`'s Ok path unexercised until Phase 3
  **Location**: Phase 2 / Testing Strategy
  The `Version`-arm match, debug line, and print are compiled but never executed
  by a Phase 2 test, weakening the "each phase independently green" guarantee for
  the behaviour Phase 2 introduces.

- 🔵 **Correctness / Portability**: Log-line assertion couples to default subscriber formatting and stderr capture
  **Location**: Phase 3 §2 (ACCELERATOR_LOG=debug case)
  A `tracing-subscriber` upgrade or sink change breaks the test for reasons
  unrelated to the behaviour under test; assert only the stable message substring.

- 🔵 **Standards**: Crate-manifest metadata (`publish`/`license`) is inconsistent across members
  **Location**: Phase 1 §2 (kernel) vs Phase 2 §1 (launcher)
  kernel gets `publish = false`; launcher (edited but not updated) does not,
  though both are equally unpublishable.

- 🔵 **Compatibility**: New closure must build under the pinned `nightly-2026-01-22` pup lane, not just stable 1.90.0
  **Location**: Phase 1 Success Criteria (pup:check)
  Caret ranges could resolve a crate whose MSRV/nightly usage the pinned nightly
  rejects; failure would look like a pup problem.

- 🔵 **Compatibility**: Unicode-3.0/DFS-2016 + TLS-ban coverage of the new closure is asserted, not verified
  **Location**: Key Discoveries / Phase 1 Manual Verification
  `time`/`clap`/`vergen` closures are first introduced here; a transitive
  un-allowed license would fail `deny:check` despite the "no allow-list edits"
  claim.

- 🔵 **Portability**: musl-static guarantee is graph-checked in deny.toml but not build-checked in this story
  **Location**: cli/deny.toml targets vs "What We're NOT Doing"
  A new dep could compile on host yet fail to statically link under musl; not
  surfaced until the cross-compile story.

#### Suggestions

- 🔵 **Code Quality**: Single-variant `Command` enum + match is marginal indirection; a one-line comment tying it to 0164's subcommand growth removes "why is this here" friction.
  **Location**: Phase 2 §6

- 🔵 **Architecture**: State the global-subscriber first-wins contract for `init` explicitly (correct for a CLI; document so callers know).
  **Location**: Phase 1 §4

- 🔵 **Standards**: Hoist `license`/`publish` into `[workspace.package]` and inherit via `.workspace = true`, matching the existing version/edition inheritance.
  **Location**: Phase 1 §1

- 🔵 **Test Coverage**: AC-7 case should assert clap's unrecognised-subcommand message, not just a non-zero exit, so it pins the clap-rejection path.
  **Location**: Phase 3 §2

- 🔵 **Compatibility**: clap 4.6 AC-7 exit behaviour is a parse-time behavioural contract; the integration assertion (not a compile check) is the right guard — keep it.
  **Location**: Phase 2 §6

- 🔵 **Portability**: `ACCELERATOR_LOG` diverges from the server's `RUST_LOG`; make the namespacing decision explicit and document it in help text.
  **Location**: Phase 1 §4

### Strengths

- ✅ Exemplary functional-core / imperative-shell split: reachable behaviour is
  factored into pure seams (`filter_from_env(Option<&str>)`, `or_unknown`,
  `VersionReporter<M>` against a fake) while global/IO wiring stays thin and is
  proven by the black-box test — precisely the testability ADR-0053 targets.
- ✅ Ports-and-adapters boundaries are correctly shaped and the domain core imports
  nothing, satisfying the inward rule structurally, not just via pup.
- ✅ AC-7 ordering is verifiably correct: `Cli::parse()` exits via clap before any
  kernel work is reached.
- ✅ The AC-5 divergence from the luminosity mirror (real thiserror `kernel::Error`
  + tracing facility) is explicitly surfaced as a Resolved Decision with rationale.
- ✅ Blessed versions match the server precedent exactly (thiserror 1, tracing 0.1,
  tracing-subscriber 0.3, clap 4); the exact `vergen =9.0.6` pin is documented as a
  deliberate, load-bearing deviation.
- ✅ `version.workspace = true` for kernel keeps version-coherence clean; the
  member-agnostic task tree needs no task edits.
- ✅ Phase decomposition is clean — each phase leaves the workspace green and is
  independently mergeable, with an inert `main.rs` in Phase 2 letting the hexagon
  land and be unit-tested before the composition root goes live.
- ✅ Strong integration-test symmetry-breaking guards (non-empty, HashSet
  distinctness, RFC-3339 not-in-future) stop the field assertions degenerating into
  tautologies.
- ✅ Choosing a stderr `fmt` subscriber over the server's JSON-to-file writer is the
  portable choice for a CLI; the native-tls/openssl ban consciously protects the
  musl-static target.

### Recommended Changes

1. **Fix the AC-2 git-fact testing strategy end-to-end** (addresses: "AC-2 tests
   or_unknown in isolation", "Plan omits assert_eq! reconciliation", "Integration
   test assumes a git tree"). Add a unit test driving `VergenBuildMetadata`
   directly, asserting each accessor equals `option_env!("VERGEN_*").unwrap_or("unknown")`
   (proves the accessor→helper wiring). In the plan's integration-test description,
   restore the load-bearing `assert_eq!(field, option_env!(...).unwrap_or("unknown"))`
   lines. Make the `assert_ne!(field, "unknown")` guards conditional on a `.git`
   directory being present (or gate behind a documented env flag), and document the
   git-working-tree assumption as a CI precondition — so a correct git-less build
   is not a test failure.

2. **Pin the vergen pair and document the git-CLI prerequisite** (addresses:
   "vergen-gitcl caret can float vergen-lib", "vergen-gitcl shells out to git").
   Pin `vergen-gitcl = "=1.0.8"` (the vetted pair) and add a comment noting the two
   are a matched pair like PUP_NIGHTLY/PUP_VERSION. Document "a `git` CLI on the
   build host" (distinct from a `.git` tree) as the prerequisite for a non-`unknown`
   SHA, and add one manual step running the build with `git` off PATH to confirm it
   degrades cleanly.

3. **Verify the AC-5 error arm is actually reachable** (addresses: "Malformed-filter
   reachability unverified", "Unit/integration use different strings"). Before
   implementation, confirm against pinned tracing-subscriber 0.3 that the chosen
   malformed string returns `Err`; if `EnvFilter` is too lenient, switch to a
   provably-rejected directive (e.g. `foo=notalevel`) and use the *same* string in
   both the unit and integration tests. Assert the specific `Error::LogFilter`
   variant, not just `is_err()`.

4. **Reconcile the comment policy** (addresses: "Verbatim comment carry-over").
   Either drop the code-restating comments and the ADR-0053 reference in pup.ron,
   keeping only the genuinely non-obvious rationale (vergen pin, `fail_on_error()`
   omission) phrased without ADR/work-item citations — or record a scoped, justified
   exception in the plan rather than a blanket "carry verbatim".

5. **Strengthen the render unit test** (addresses: "render test asserts only values
   present"). Assert the exact four-line output (or per-line prefix + value at each
   index) against a hand-built `VersionReport`, so a prefix/order mutation fails at
   the unit level.

6. **Address the minor consistency items opportunistically**: narrow the pup rule's
   kernel allowance (or record the whole-kernel allowance as deliberate); carry the
   `launcher`-crate-name deviation note into the plan; standardise manifest
   `publish`/`license` (ideally via `[workspace.package]`); use `#[from] ParseError`
   for `kernel::Error::LogFilter`; add a Phase 2 `dispatch` Ok-path test; and add an
   automated quiet-default-stderr negative.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally sound and faithfully realises the
hexagonal pattern of ADR-0053 and the composition model of ADR-0054: a thin clap
inbound adapter, a pure domain core behind inbound/outbound ports, a vergen
outbound adapter bound at a composition root, and cargo-pup enforcing inward
dependencies at module granularity. Functional-core/imperative-shell separation is
exemplary. The main structural questions are the breadth of the kernel import
allowance granted to the domain core and the acknowledged-but-worth-restating
divergences from the ADRs (crate name, kernel depth).

**Strengths**:
- Clean functional-core/imperative-shell split (pure seams vs I/O wiring proven by
  the black-box test).
- Ports-and-adapters boundaries correctly shaped; core imports nothing.
- AC-5 divergence from the mirror surfaced as a Resolved Decision with rationale.
- kernel kept deliberately dependency-light; config/dispatch deferred.
- Phase decomposition architecturally clean; inert Phase 2 main.rs.

**Findings**:
- 🔵 minor (high): pup rule grants core import access to the entire kernel crate,
  not just the error taxonomy (Phase 3 §3). As kernel grows, core could import
  non-taxonomy modules and the rule would not catch it. Narrow to `^kernel::Error`
  or record the whole-kernel allowance as deliberate.
- 🔵 minor (high): launcher crate named `launcher` diverges from ADR-0054's `cli`;
  deviation is in the work item but not the plan (Overview). Carry a one-line note
  into the plan or amend the ADR.
- 🔵 minor (medium): outbound `BuildMetadata` returns `&'static str` while
  `VersionReport` holds owned `String`s (Phase 2 §4/§5); the `'static` is a
  compile-time-source artefact that would force a port change for a runtime-sourced
  adapter later.
- 🔵 suggestion (medium): `init` swallows the already-initialised error to be
  idempotent — reasonable, but state the first-wins global-subscriber contract
  (Phase 1 §4).

### Code Quality

**Summary**: A well-structured, test-first scaffold with clear module boundaries,
pure/testable seams, and self-documenting naming that will be pleasant to maintain.
Design is proportional to a deliberately-trivial feature; the main concerns are a
few vestigial/forward-looking abstractions (dispatch Result, single-variant enum)
and a couple of error-handling patterns (swallowed try_init, the render/prefix
contract, the stringly error) worth an explicit note so intent is clear.

**Strengths**:
- Behaviour-carrying logic factored into pure, directly-testable seams.
- Clean dependency inversion; core imports no adapter/I/O.
- Self-documenting naming and module layout.
- Complexity proportional to requirements (KISS/YAGNI honoured).
- Non-obvious decisions captured in load-bearing comments.

**Findings**:
- 🔵 minor (high): `init` discards `try_init()` via bare `let _ =`, reading as an
  accidental ignore; name/comment the intent (Phase 1 §4).
- 🔵 minor (medium): `dispatch` Result is vestigial for the only compiled command;
  the reachable error is in `run`/`init` (Phase 2 §6 / Phase 3 §1). Keep the
  fallible signature but comment that `version` cannot fail.
- 🔵 minor (medium): `render` field-to-prefix-to-line mapping is implicit and
  positional (Phase 2 §6); make the mapping data, iterated by both render and
  consumers.
- 🔵 suggestion (medium): single-variant `Command` enum is marginal indirection;
  a comment tying it to 0164 removes friction (Phase 2 §6).
- 🔵 minor (low): `kernel::Error::LogFilter(String)` discards the `ParseError`
  source chain, diverging from the server's `#[source]` precedent (Phase 1 §3);
  `#[from]` preserves it with no new dependency.

### Test Coverage

**Summary**: Genuinely test-first with thoughtful seam design — behaviour-carrying
logic is factored into pure functions and global/IO wiring is covered by a
black-box integration test; every AC has at least one nominated test. The main
gaps are that a few pure-seam unit tests are decoupled from the production call
sites they claim to protect (AC-2's or_unknown, render prefixes/ordering), so a
mutation at the wiring layer slips through, and a couple of AC clauses are only
implicitly covered.

**Strengths**:
- Concrete test-first specification per phase; each phase leaves a runnable suite.
- Good pyramid balance; pure seams avoid process-global mutation.
- AC-5's runtime-observable clauses covered at the right (integration) level.
- Strong symmetry-breaking guards (non-empty, distinctness, RFC-3339 not-future).
- pup probe fixture retargeted with a retained positive control.

**Findings**:
- 🟡 major (high): AC-2 discharged solely by unit-testing `or_unknown`; nothing
  verifies the accessors route through it or read the right key (Phase 2 §5). Add a
  direct `VergenBuildMetadata` accessor test.
- 🟡 major (medium): `render` unit test specified only as "four values present";
  prefixes/ordering only checked in integration (Phase 2 §6). Assert the exact
  four-line output at the unit level.
- 🔵 minor (medium): unit (`"=not a filter="`) and integration (`":::"`) use
  different, cross-unchecked malformed inputs (Phase 1 §4 vs Phase 3 §2); use one
  verified string and assert the `Error::LogFilter` variant.
- 🔵 minor (medium): `kernel::Error` std::error::Error "test" is a compile
  assertion, not a specified test (Testing Strategy); add a `Display`-contract unit
  test or drop the phantom entry.
- 🔵 minor (medium): no automated negative that a default-level run keeps stderr
  quiet (Phase 3 §2); convert the manual check to an assertion.
- 🔵 suggestion (medium): AC-7 case asserts only non-zero exit; assert clap's
  unrecognised-subcommand message (Phase 3 §2).

### Correctness

**Summary**: Logically sound core control flow — `Cli::parse()` genuinely runs
before kernel work (AC-7 correct), and the seams are pure and directly testable.
The most significant risks are in net-new code with no luminosity precedent: the
malformed-filter error path's reachability hinges on unverified assumptions about
what `EnvFilter` rejects, and `try_init()` error-swallowing conflates
already-initialised with any install failure. Separately, the plan's description of
the integration test omits the load-bearing `assert_eq!`-against-`option_env!`
reconciliation, leaving a fragile git-tree assumption.

**Strengths**:
- AC-7 ordering verifiably correct (parse exits before kernel work).
- Degradation and domain-service logic factored into pure side-effect-free seams.
- AC-6 single-source-of-truth logically sound (env!("CARGO_PKG_VERSION")).
- RFC-3339 not-in-future bound correctly reasoned (no flaky lower bound).

**Findings**:
- 🔴/🟡 major (medium): malformed-filter error path reachability depends on
  unverified `EnvFilter` rejection (Phase 1 §4 & Phase 3 §2); if the strings parse
  Ok, AC-5 collapses to the never-arm. Verify against pinned tracing-subscriber and
  use a provably-rejected string at both levels.
- 🟡 major (high): plan omits the `assert_eq!(field, option_env!(...).unwrap_or("unknown"))`
  reconciliation that makes the git-fact asserts deterministic (Phase 3 §2); the
  "always a git tree" assumption is an environmental precondition, not a guarantee.
- 🔵 minor (high): `try_init()` swallows all install failures, not just
  already-initialised; the benign case can't occur for the single caller (Phase 1 §4).
- 🔵 minor (medium): log-line assertion couples to default event formatting and
  level enablement (Phase 3 §2).
- 🔵 minor (medium): Phase 2 leaves `dispatch`'s Ok path unexercised until Phase 3
  (Phase 2 / Testing Strategy); add a Phase 2 dispatch test.

### Standards

**Summary**: Strongly convention-aware — matches the server's blessed dependency
versions, preserves the exact vergen pin as a justified deviation, keeps
`version.workspace = true` for coherence, and renames the pup rule in step with the
module path. The one material conflict is carrying luminosity comments verbatim
(and the pup.ron ADR-0053 reference) against the plan's own governing instructions
banning code-restating comments and ADR references in comments. A secondary
inconsistency is the crate-manifest `license`/`publish` metadata diverging between
members.

**Strengths**:
- Blessed versions matched to server precedent exactly.
- Exact vergen pin called out as a deliberate, documented deviation.
- `version.workspace = true` for kernel keeps version-coherence clean.
- pup rule renamed in lockstep with the retargeted path.
- `ACCELERATOR_LOG` follows the established `ACCELERATOR_*` prefix convention.
- `[workspace.dependencies]` centralises versions via workspace inheritance.

**Findings**:
- 🟡 major (high): verbatim comment carry-over conflicts with the plan's own
  no-comments / no-ADR-ref policy (Phase 1 §1, Phase 2 §2, Phase 3 §3). Drop or
  scope-except the offending comments.
- 🔵 minor (high): crate-manifest `publish`/`license` inconsistent across members
  (Phase 1 §2 vs Phase 2 §1); standardise (ideally via `[workspace.package]`).
- 🔵 suggestion (medium): hoist `license`/`publish` to `[workspace.package]` and
  inherit, matching version/edition inheritance (Phase 1 §1).

### Compatibility

**Summary**: A greenfield workspace-growth scaffold with no external API consumers,
so contract compatibility is largely moot; the meaningful surface is dependency
version-constraint safety and toolchain interoperability. Pinning is mostly sound
(the exact vergen pin is correctly reasoned), but there is a genuine version-safety
gap: the caret-ranged `vergen-gitcl "1"` can float independently of the exactly-
pinned `vergen =9.0.6` and drag a transitive `vergen-lib` bump that breaks the
Emitter. Secondary risks are the new closure building under the pinned pup nightly
and the license/ban-coverage claims for the new closure.

**Strengths**:
- The load-bearing exact vergen pin is correctly identified and documented inline.
- build.rs omits fail_on_error() + adapter degrades to "unknown" (git-less
  compatible).
- Dependency versions aligned with the in-repo Rust precedent.
- Explicit lockfile audit for the native-tls/openssl bans.
- edition 2021 inherited via `edition.workspace = true`.

**Findings**:
- 🟡 major (medium): `vergen-gitcl = "1"` caret can float a `vergen-lib` bump that
  breaks the pinned `vergen =9.0.6` Emitter (Phase 1 §1); pin the vetted pair
  (`=1.0.8`).
- 🔵 minor (medium): new closure must build under `nightly-2026-01-22` (pup lane),
  not just stable 1.90.0 (Phase 1 Success Criteria); add an explicit criterion.
- 🔵 minor (medium): Unicode-3.0/DFS-2016 + TLS-ban coverage asserted, not verified
  against the resolved graph (Key Discoveries); make the deny audit hard and
  enumerate the new closures' licenses.
- 🔵 minor (low): clap 4.6 AC-7 exit is a parse-time behavioural contract; the
  integration test (not a compile check) is the right guard — keep it (Phase 2 §6).

### Portability

**Summary**: Well-grounded in the luminosity blueprint with several deliberately
portable choices (stderr subscriber, rustls-only bans, vergen degradation,
member-agnostic task tree). The main risks are around build-time git/vergen
coupling: `vergen-gitcl` shells out to the `git` CLI at build time (an undocumented
build-host prerequisite), and the integration test's `assert_ne!(field, "unknown")`
assumes every build environment is a git tree with git installed — true for current
runners but broken for shallow/git-less/sandboxed builders, directly contradicting
AC-2's git-less degradation contract.

**Strengths**:
- stderr fmt subscriber is the portable CLI choice (no filesystem assumptions).
- build.rs omits fail_on_error(); adapter degrades via option_env!.unwrap_or.
- Exact vergen pin + native-tls/openssl ban protect the musl-static target.
- CARGO_TARGET_TRIPLE / ACCELERATOR_LOG portable across shipped triples.
- cargo-pup nightly coupling isolated to one self-provisioning lane.

**Findings**:
- 🟡 major (high): integration test assumes a git tree + git binary on every build
  host, contradicting AC-2 (Phase 3 §2). Gate the `assert_ne!` guards on a `.git`
  presence and document the precondition.
- 🟡 major (medium): `vergen-gitcl` shells out to the `git` CLI at build time — an
  undocumented build-host prerequisite distinct from a `.git` tree (Phase 2 §2);
  document it and add a git-absent manual step.
- 🔵 minor (medium): log-line integration test couples to stderr capture + default
  formatting (Phase 3 §2); assert the stable substring and document the
  stdout/stderr split as a contract.
- 🔵 minor (medium): musl-static guarantee graph-checked in deny.toml but not
  build-checked in this story (deny.toml targets vs "What We're NOT Doing"); note
  it remains unverified-by-build until the cross-compile story.
- 🔵 minor (low): `ACCELERATOR_LOG` diverges from the server's `RUST_LOG` with no
  fallback (Phase 1 §4); make the namespacing decision explicit and document it.

## Re-Review (Pass 2) — 2026-07-03

**Verdict:** APPROVE

Re-ran all 7 lenses against the revised plan. **All 8 original major findings are
resolved.** The re-review surfaced 5 new majors — most a direct consequence of the
round-1 edits (the accessor-test asymmetry and the pup-rule narrowing created new,
narrower test gaps) or refinements the tighter plan made visible. All 5 have been
addressed in a follow-up round of edits, along with the notable new minors. No
critical findings in either pass; no unresolved majors remain. Remaining open items
are minors/suggestions consciously accepted or deferred to follow-up stories.

### Previously Identified Issues (round-1 majors)

- 🟡 **Test Coverage**: AC-2 tests `or_unknown` in isolation — **Resolved** (per-accessor
  wiring tests assert each accessor routes its own `VERGEN_*` key through the helper).
- 🟡 **Correctness**: `assert_eq!` reconciliation omitted — **Resolved** (restored, with
  the version line also reconciled against `env!("CARGO_PKG_VERSION")`).
- 🟡 **Portability**: integration test assumes a git tree — **Resolved** (documented as a
  hard CI invariant; per user, CI is always a git checkout).
- 🟡 **Correctness**: malformed-filter reachability unverified — **Resolved** (verified
  string, `Error::LogFilter` variant + Display-substring assertions, same string in
  unit and integration).
- 🟡 **Compatibility**: `vergen-gitcl` caret can float `vergen-lib` — **Resolved** (pinned
  `=1.0.8` matched pair).
- 🟡 **Portability**: `vergen-gitcl` git-CLI prerequisite undocumented — **Resolved**
  (recorded in Migration Notes; distinguishes `.git` tree from `git` binary).
- 🟡 **Standards**: verbatim comment carry-over vs no-comments policy — **Resolved** (only
  genuinely non-obvious comments retained; ADR-0053 reference dropped from pup.ron).
- 🟡 **Test Coverage**: `render` test asserts only "values present" — **Resolved** (exact
  four-line string, prefix + value per index).

### New Issues Introduced (round-2 review) — all now addressed

- 🟡 **Test Coverage**: AC-6 had no automated test; `crate_version` accessor lacked the
  wiring test the three git facts got — **Addressed** (added the `crate_version`
  accessor unit test + integration version-line reconciliation against
  `env!("CARGO_PKG_VERSION")`; AC-6 now automated + manual belt-and-braces).
- 🟡 **Test Coverage**: the pup-rule narrowing (`^kernel::Error` only) was proven only by
  a manual step — **Addressed** (probe fixture grows a two-module `kernel`; a
  `kernel::Error` import passes and a `kernel::logging` import is rejected, proving the
  narrowing discriminates automatically).
- 🟡 **Correctness**: `assert_ne!(field, "unknown")` wrongly attributed `build_date` and
  `target_triple` to the git invariant — **Addressed** (invariant scoped to
  `commit_sha` only; the two non-git vergen outputs hold unconditionally, in
  Phase 3 §2, Testing Strategy, and Migration Notes).
- 🟡 **Compatibility**: caret-ranged `tracing-subscriber 0.3` could drift the
  `ParseError`/Display contract the reachability check depends on — **Addressed**
  (Migration Notes frames the committed `Cargo.lock` as the compatibility anchor both
  toolchains build; committed reachability/message tests gate any `cargo update`).
- 🟡 **Compatibility**: the frozen pup nightly must compile the closure at whatever
  `cargo update` resolves — **Addressed** (same lockfile-anchor framing; pin the
  member rather than loosen the gate if an MSRV outruns the nightly).

Also addressed: the shown pup.ron snippet now carries the retained (ADR-free) header
comment (standards); the ADR-0054 kernel dependency-tail tradeoff is made explicit in
the Resolved-decision section (architecture); the imprecise "server caret" phrasing is
corrected (standards).

### Assessment

The plan is in strong shape and ready for implementation. Both review passes'
major findings are resolved, and the remaining items are minor/suggestion-level and
consciously accepted or deferred: the `dispatch` `Result` and single-variant `Command`
enum are intentional 0164 extension points (now exercised by tests); the ADR-0054
crate-name amendment and the `core`-vs-`domain` vocabulary reconciliation are
appropriate post-landing follow-ups; `git`-on-build-host documentation beyond the plan
and an actual `--target *-musl` smoke build belong to the distribution story (0165).
The one implementation-time gate to honour is the plan's own instruction: confirm the
chosen malformed filter string is genuinely rejected by the pinned `tracing-subscriber`
before relying on the AC-5 error arm.
