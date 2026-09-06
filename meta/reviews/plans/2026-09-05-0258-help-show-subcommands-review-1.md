---
type: "plan-review"
id: "2026-09-05-0258-help-show-subcommands-review-1"
title: "Plan Review: Help Should Show Subcommands Implementation Plan"
date: "2026-09-05T22:16:59+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-05-0258-help-show-subcommands"
parent: "plan:2026-09-05-0258-help-show-subcommands"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "code-quality", "test-coverage", "correctness", "usability", "compatibility", "documentation"]
review_number: 1
review_pass: 3
tags: ["cli", "help", "launcher", "discoverability"]
last_updated: "2026-09-06T08:57:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Help Should Show Subcommands Implementation Plan

**Verdict:** REVISE

The plan is unusually well-researched and structurally sound: it replaces a
hand-rolled `after_help` text block with clap-native subcommand injection,
converges three divergent help entry points on one manifest-driven listing,
and correctly corrects the work item's wrong error-kind assumption
(`MissingSubcommand`, not `DisplayHelpOnMissingArgumentOrSubcommand`). No
critical defects surfaced, and exit-code preservation, fail-open behaviour, and
deterministic ordering all hold up against the source. The revise verdict rests
on five major findings, all edge cases and completeness gaps rather than
fundamental design faults — the headline testability gain is undercut by a
retained global read, three lenses independently flag an unguarded name
collision, and the merged listing has a punctuation inconsistency that violates
AC-5.

### Cross-Cutting Themes

- **Global `args_os()` read keeps routing untestable** (flagged by: code-quality,
  test-coverage) — the plan sells extracting `is_root_help_args` as dropping the
  untestable global read, but `handle_parse_error` still calls
  `is_root_help_args(&root_args())`, where `root_args()` re-reads
  `std::env::args_os()`. The pure predicate becomes testable; the actual
  error-kind-plus-args-to-exit-code decision stays as unverifiable as before.
- **Name collision on the merged clap namespace** (flagged by: architecture,
  compatibility, correctness) — `augment_with_subbinaries` injects one clap
  subcommand per manifest binary into a tree that already declares `version`,
  `config`, `cache`, and clap's synthesised `help`. A manifest name colliding
  with a built-in (or an empty name) hits clap's duplicate-subcommand
  `debug_assert` — panicking the help path under `cargo test` — and silently
  double-lists in release. `sanitize()` guards control characters, not
  collisions.
- **Bare invocation: stream flip and lost error cue** (flagged by: usability,
  compatibility, correctness) — routing `MissingSubcommand` to
  `render_full_listing(ExitCode::from(1))` moves output from clap's stderr usage
  error to a full help body on stdout while keeping exit 1. The result is a
  success-looking listing paired with a failure exit, no "a subcommand is
  required" cue, and an empty stderr.
- **Merged listing style inconsistency** (flagged by: usability, documentation)
  — built-in descriptions are full sentences ending in a period; the new
  sub-binary descriptions are terse imperative fragments; `migrate` keeps a
  trailing period the AC-5 text omits. The one list the feature exists to unify
  ends up visibly half-merged.

### Tradeoff Analysis

- **Testability vs plan scope**: threading the parsed args into
  `handle_parse_error` (or splitting a pure `route(kind, &args) -> ExitCode`)
  makes the exit-code mapping unit-testable but widens the Phase 2 change beyond
  the predicate extraction. Recommendation: take the wider change — the routing
  decision is the most bug-prone logic and AC-3's exit contract deserves a fast
  deterministic guard, not only a spawned-process assertion.
- **Immediate render vs network fetch on the bare/error path** (architecture):
  routing bare `accelerator` through `load_help_manifest` puts a
  signature-verified network fetch on a previously-instant error path.
  Recommendation: confirm the fetch timeout is tight and bounded, or render
  built-ins immediately and attach sub-binaries only when the manifest is
  already resolvable — keeping the most common mistyped invocation off the
  network's critical latency path.

### Findings

#### Critical

None.

#### Major

- 🟡 **Documentation**: `migrate` keeps a trailing period the AC-5 text omits
  **Location**: Phase 1, Section 1 (migrate note, plan lines 191-192)
  The plan leaves `cli/migrate-cli/Cargo.toml` as "already correct", but its
  value ends in a full stop while the AC-5 text and all eight rewrites have
  none. In the merged listing `migrate` becomes the only entry ending in a
  period — off-spec against AC-5 and visibly inconsistent.

- 🟡 **Architecture**: Bare invocation becomes network-bound
  **Location**: Phase 2, Section 2; Performance Considerations
  Routing `MissingSubcommand` through `render_full_listing` calls
  `load_help_manifest()`, a signature-verified network fetch. The most common
  mistyped invocation, previously an instant clap usage error with no I/O, now
  blocks on a network round-trip before printing. The Performance section treats
  `help` and bare as equivalent one-shots but does not bound the timeout or the
  error-path latency.

- 🟡 **Code Quality**: Routing decision still reads global process state
  **Location**: Phase 2, Sections 3-4
  The extracted `is_root_help_args` is pure, but `handle_parse_error` calls it
  via `root_args()`, which re-reads `std::env::args_os()`. The mapping of error
  kind plus args to renderer and exit code — the most bug-prone logic — stays
  untestable, so the stated testability gain is largely cosmetic.

- 🟡 **Test Coverage**: AC-6 "appears in all three help outputs" proven for one
  renderer call only
  **Location**: Phase 2, Sections 1 & 5; Testing Strategy
  The populated listing is tested once via a single `augment_with_subbinaries`
  call; the three-way convergence is proven only on the fail-open path, which by
  construction cannot assert any sub-binary. A regression where
  `MissingSubcommand` or root `help` reached a non-augmented `Cli::command()`
  would pass every proposed test.

- 🟡 **Test Coverage**: The called-out `config --help` regression has no
  end-to-end guard
  **Location**: Key Discoveries (config --help); Phase 2, Sections 3-4
  The plan flags that narrowing (not removing) `is_root_help` is essential to
  keep `config --help` on config's own help, but guards it only with the pure
  predicate table plus manual verification. No automated test asserts
  `accelerator config --help` renders config's help rather than the merged
  listing.

#### Minor

- 🔵 **Correctness / Compatibility / Architecture**: Merged clap namespace has no
  collision or empty-name reconciliation
  **Location**: Phase 2, Section 1 (`augment_with_subbinaries`)
  A manifest binary named like a built-in (`version`, `config`, `cache`, `help`)
  or with an empty name creates a duplicate/malformed subcommand: clap
  `debug_assert`s (panic in debug/test builds) and silently double-lists in
  release. Three lenses flagged this independently. Skip or defensively handle
  names that collide with an existing subcommand (clap exposes
  `find_subcommand`) or are empty, and add a fixture test asserting a built-in
  wins and the renderer does not panic.

- 🔵 **Correctness**: Multi-token root-help forms lose sub-binary augmentation
  **Location**: Phase 2, Section 3 (`is_root_help_args`)
  The new predicate matches exactly `["--help"]`, `["-h"]`, `["help"]`. The
  current `is_root_help` routes any `DisplayHelp` whose second token is not a
  built-in word, so `accelerator --help extra` currently augments but under the
  new predicate falls through to built-ins-only. Either accept the narrowing
  explicitly or match a leading-token pattern.

- 🔵 **Usability**: Merged list mixes two description styles
  **Location**: Phase 1, Section 1 & Phase 2, Section 1
  Built-in doc comments are full sentences with trailing periods; new
  sub-binary descriptions are terse fragments without. Once both appear in one
  unheadinged list, the tone and punctuation mismatch is visible on every
  invocation. Align both on one convention (imperative fragment, no trailing
  period) or call out the built-in restyle as an explicit 0259 dependency.

- 🔵 **Usability**: Bare invocation loses its actionable error cue
  **Location**: Phase 2, Sections 2 & 4
  `MissingSubcommand` → `render_full_listing` prints an ordinary-looking help
  screen to stdout with exit 1 and empty stderr — no line explaining what went
  wrong. The "you asked for help" vs "you made a mistake" distinction is erased.
  Prepend a one-line usage cue (ideally to stderr) before the listing.

- 🔵 **Compatibility**: Bare invocation output moves from stderr to stdout
  **Location**: Phase 2, Sections 2 & 4
  A caller capturing stderr to detect the missing-subcommand error, or treating
  non-empty stdout as success, would observe a silent contract change. AC-3
  constrains only the exit code. The move is defensible (it matches the existing
  force-stdout-for-help intent) but should be a recorded decision.

- 🔵 **Code Quality**: Needless allocation in `is_root_help_args`
  **Location**: Phase 2, Section 3
  The body collects a `Vec<&OsStr>` purely to match a single-element slice.
  `matches!(args, [a] if a == "--help" || a == "-h" || a == "help")` compares
  `OsString` against `&str` directly and expresses the intent at a glance.

- 🔵 **Code Quality**: Fail-open and lazy-crypto-init rationale must move with the
  refactor
  **Location**: Phase 2, Section 2
  `help_section()` and `render_augmented_help()` carry non-obvious doc comments
  explaining why the crypto provider installs lazily and why the manifest load
  fails open. The fold into `load_help_manifest`/`render_full_listing` must carry
  that rationale — exactly the genuinely-non-obvious "why" the project's comment
  policy preserves.

- 🔵 **Documentation**: `design-cli` description under-describes its surface
  **Location**: Phase 1, Section 1 (design-cli)
  "Validate and process design sources" covers `validate-source` but "process"
  is vague and evokes none of `resolve-auth`, `scrub-secrets`,
  `notify-downgrade`, or `audit-cue-phrases`. Confirm this is the deliberate
  AC-5 wording or note that broader wording is deferred to 0259.

#### Suggestions

- 🔵 **Usability**: Merged listing ordering is inconsistent
  **Location**: Phase 2, Section 1
  Built-ins render in declaration order (`version`, `config`, `cache`); manifest
  binaries follow alphabetically (`BTreeMap`), with clap's `help` slotted in.
  Decide and document an intended order (fully alphabetical scans best) since
  the merge is when ordering becomes user-visible.

- 🔵 **Usability**: "Serve the meta-directory visualiser" is less discoverable
  than the intent
  **Location**: Phase 1, Section 1 (visualiser)
  "Serve" is accurate but a user scanning for "how do I view my meta directory"
  may not recognise it. If AC-5's wording is fixed, note this as a deliberate
  accuracy-over-discoverability trade for 0259 to revisit.

- 🔵 **Documentation**: `corpus` parenthetical drops the `linkage` subcommand
  **Location**: Phase 1, Section 1 (corpus-cli)
  "Manage the meta-document corpus (ADRs, metadata, frontmatter)" reads as an
  exhaustive list but omits the live `linkage` subcommand. Include it or frame
  the parenthetical as illustrative.

- 🔵 **Test Coverage**: No regression guard that descriptions stay user-facing
  (AC-5)
  **Location**: Phase 1, Sections 1-2; Success Criteria
  Nothing prevents a future sub-binary from being registered with "… sub-binary."
  phrasing. Add a lightweight test asserting no collected description contains
  the substring "sub-binary".

- 🔵 **Compatibility**: `test_github` fixture left as a stale mirror of the old
  contract
  **Location**: Phase 1, Section 2
  `_SUBBINARY_DESCRIPTIONS` keeps the retired "The … sub-binary." strings.
  Harmless for green, but a stale example diverging from the source of truth.
  Take the plan's own optional step and align them, or reaffirm they are
  placeholders.

- 🔵 **Code Quality / Architecture**: `package.description` overloaded as help
  copy without a visible seam
  **Location**: Phase 1, Section 1
  Each crate's `description` now serves both packaging metadata and CLI help
  text, an implicit coupling invisible at the edit site. Record it as an
  accepted tradeoff and consider a one-line note in `tasks/README.md`'s
  registration checklist that `description` is user-facing help.

### Strengths

- ✅ Replacing the hand-rolled `after_help` block (manual width calc, `write!`
  formatting) with clap-native `.subcommand()` injection removes bespoke layout
  code, an entire function, and delivers the open-closed property AC-6 demands.
- ✅ Correctly identifies bare `accelerator` as `ErrorKind::MissingSubcommand`
  (the top-level `Cli` sets no `arg_required_else_help`), routes it to exit 1,
  and corrects the work item's mistaken Technical Notes.
- ✅ Exit-code preservation is sound across all three paths; `Manifest.binaries`
  is a `BTreeMap`, so the merged listing is deterministically ordered and the
  `diff <(--help) <(help)>`-empty acceptance is well-founded.
- ✅ Refuses to add a manifest-injection or signature-bypass seam, decomposing
  the AC-6 assertion into a unit test over a hand-built `Manifest` rather than
  weakening the signed-manifest trust boundary.
- ✅ Verifies rather than assumes blast radius: narrows the breaking Python
  assertions to the single `test_manifest.py` visualiser test and confirms no
  docs-site page or README references the old heading or descriptions.
- ✅ Two phases are independently mergeable with the tree green after either.

### Recommended Changes

1. **Drop the trailing period from `cli/migrate-cli/Cargo.toml`** (addresses:
   `migrate` keeps a trailing period). Reconcile the research table that marks
   it "already matches", so all nine descriptions follow one no-period
   convention and the merged listing is uniform.

2. **Thread the parsed root args into `handle_parse_error`** (addresses: routing
   decision still reads global; exit-code routing hard to unit-test). Have
   `main` collect `std::env::args_os().skip(1)` once and pass it in, or split a
   pure `route(kind, &args) -> ExitCode`, so the error-kind-to-exit-code mapping
   is table-testable without process globals.

3. **Guard `augment_with_subbinaries` against collisions and empty names**
   (addresses: merged clap namespace has no reconciliation). Skip any manifest
   name already present as a subcommand (via `find_subcommand`) or empty, and
   add a fixture test asserting a colliding/empty name neither panics the
   renderer nor double-lists.

4. **Add routing-level and `config --help` tests** (addresses: AC-6 proven for
   one call only; `config --help` has no e2e guard). Assert each of the three
   parse kinds maps to the single augmenting `render_full_listing`, and add a
   black-box test that `accelerator config --help` (manifest unavailable) prints
   config's own help, not the top-level listing.

5. **Bound or relocate the bare-path manifest fetch** (addresses: bare
   invocation becomes network-bound). State the fetch timeout is tight, or
   render built-ins immediately and attach sub-binaries only when the manifest
   is already resolvable, keeping the error path off the network critical path.

6. **Add a one-line usage cue to the bare path and record the stream move**
   (addresses: lost error cue; stderr→stdout change). Prepend "error: a
   subcommand is required" (ideally to stderr) before the listing, and note the
   intentional stderr→stdout move as a recorded decision.

7. **Unify the description style and fix `is_root_help_args` details**
   (addresses: mixed styles; needless allocation; multi-token forms; lost
   rationale). Align built-in and sub-binary descriptions on one convention (or
   flag the built-in restyle as a 0259 dependency); match the slice directly
   without the `Vec` allocation; decide whether a leading-token pattern is
   wanted; carry the fail-open/lazy-init rationale into the new functions.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is architecturally sound and improves structural
integrity: it replaces an ad-hoc `after_help` text block with clap-native
subcommand injection, unifying two disjoint command sources (compile-time enum +
runtime manifest) into a single render path, and it extracts the root-help
predicate into a pure function that drops a global `args_os()` read. It
preserves the fail-open trust boundary and, commendably, declines to add a
signature-bypass seam for testability. The main architectural forces worth
scrutiny are that bare invocation — an error path — now incurs a network
manifest fetch, and that merging two independently-governed command namespaces
into one clap tree introduces a latent collision mode with no reconciliation
rule.

**Strengths**:
- Extracting `is_root_help_args` as a pure function over `&[OsString]` cleanly
  separates the functional core from the imperative shell (`args_os()` read),
  improving testability and honouring the functional-core/imperative-shell
  boundary.
- Moving from a hand-formatted `after_help` string to clap-native `.subcommand()`
  injection collapses two rendering paths into one and delivers the open-closed
  property the AC demands.
- Explicitly refusing to add a manifest-injection or signature-bypass seam
  preserves the signed-manifest trust boundary.
- Fail-open behaviour is preserved, and the two phases are independently
  mergeable with the tree green after either.

**Findings**:
- major (medium): Bare invocation becomes network-bound. Routing bare
  `accelerator` (`MissingSubcommand`) through `render_full_listing` calls
  `load_help_manifest()`, a signature-verified network fetch. Under a slow or
  unreachable host, bare invocation — an error path — blocks for the fetch
  timeout before emitting its exit-1 listing. Suggestion: confirm the timeout is
  tight and bounded, or render built-ins immediately and attach sub-binaries only
  when the manifest is already resolvable.
- minor (medium): Two command sources share one clap namespace with no
  reconciliation rule. A future (or malicious-but-signed) manifest binary named
  `config` would render a duplicate/conflicting entry. Suggestion: define a
  precedence/dedup rule and test a built-in wins over a colliding sub-binary.
- minor (low): User-facing help text sourced from `package.description` overloads
  a cargo-packaging field to serve as help copy. Inert today; note it explicitly
  as a recorded pragmatic tradeoff.

### Code Quality

**Summary**: A small, well-scoped, generally clean plan. Its strongest quality
move is replacing the hand-rolled `after_help` string-formatting with
clap-native subcommand rendering, deleting bespoke column-width logic. The main
concern is that the headline testability improvement — extracting a pure
`is_root_help_args` — is undercut by the routing snippet still reaching for a
global env read, leaving the actual routing decision as untestable as before.

**Strengths**:
- Replacing the hand-rolled `after_help` block with clap-native injection removes
  bespoke layout code and an entire function.
- Extracting the root-help predicate into a pure function is the right instinct.
- `augment_with_subbinaries` is a clean pure function preserving the `sanitize()`
  boundary on both name and description.
- Folding three near-identical entry paths onto one renderer reduces the
  divergence that is the root cause of the bug.
- Deterministic ordering is free because `Manifest.binaries` is a `BTreeMap`.

**Findings**:
- major (medium): Routing snippet calls `is_root_help_args(&root_args())`, where
  `root_args()` re-reads `std::env::args_os()`. The pure predicate is testable
  but `handle_parse_error` stays untestable. Suggestion: thread args into
  `handle_parse_error(&error, &root_args)` so routing becomes table-testable.
- minor (high): `is_root_help_args` allocates a `Vec<&OsStr>` to match a
  single-element slice. Suggestion: `matches!(args, [a] if a == "--help" || a ==
  "-h" || a == "help")`.
- minor (medium): The existing fail-open and lazy-crypto-init doc-comment
  rationale must move with the refactor rather than being lost.
- suggestion (low): `package.description` serves both packaging metadata and help
  text with no visible seam; keep the `test_manifest.py` tripwire green and
  consider a note in the registration checklist.

### Test Coverage

**Summary**: The plan is unusually test-aware: it identifies the signing
constraint forcing the populated-listing assertion to the unit level, adds a
strong mutation-guarding renderer test, a truth-table for `is_root_help_args`,
and extends the black-box suite to cover exit codes fail-open. The main gaps are
that AC-6's "appears in all three help outputs" is verified for one renderer call
rather than across the three routes, the `config --help` regression risk has no
end-to-end guard, and the exit-code routing remains awkward to unit-test because
it still reads global args.

**Strengths**:
- The AC-6 renderer test is a genuine mutation guard (asserts "External
  subcommands" gone, plus a built-in and the fixture present).
- Extending `tests/help.rs` to assert bare exits 1 and `help`/`--help` exit 0
  gives real end-to-end coverage of AC-3's exit-code contract.
- `is_root_help_args` refactored into a pure function with an explicit truth
  table.
- The plan verifies rather than assumes the Python blast radius.

**Findings**:
- major (medium): AC-6 "appears in all three help outputs" is only proven for one
  `augment_with_subbinaries` call; three-way convergence is proven only on the
  fail-open path, which cannot assert any sub-binary. Suggestion: add a
  routing-level test asserting all three parse kinds map to the augmenting
  renderer.
- major (medium): The called-out `config --help` regression has no end-to-end
  guard beyond the predicate table and manual verification. Suggestion: add a
  black-box test that `accelerator config --help` prints config's own help.
- minor (medium): Exit-code routing stays hard to unit-test because
  `handle_parse_error` reads the global args. Suggestion: pass args in or split a
  pure `route(kind, &args) -> ExitCode`.
- suggestion (medium): No regression guard that descriptions stay user-facing
  (AC-5). Suggestion: assert no collected description contains "sub-binary".

### Correctness

**Summary**: The plan is well-researched and its core correctness claims hold up
against the source: bare invocation is genuinely `MissingSubcommand`, exit codes
are preserved across all three entry points, and the `is_root_help` narrowing
correctly keeps `config --help`, `help <sub>`, and `version --help` on clap's
per-command help. `Manifest.binaries` is a `BTreeMap`, so the merged listing is
deterministically ordered and the diff-empty acceptance is sound. The only gaps
are edge cases: a manifest binary sharing a built-in name would collide when
augmenting, and the narrowed single-token predicate silently drops augmentation
for multi-token root-help forms the old predicate handled.

**Strengths**:
- Correctly identifies bare `accelerator` as `MissingSubcommand` and routes it to
  exit 1, correcting the work item's Technical Notes.
- Exit-code preservation is sound across all three paths.
- `Manifest.binaries` is a `BTreeMap`, so injection is deterministically sorted.
- The narrowed predicate correctly excludes `config --help`, `help config`, and
  `version --help`.
- Fail-open manifest loading is preserved.

**Findings**:
- minor (medium): No guard against a manifest binary name colliding with a
  built-in or clap's auto `help` — clap's duplicate-subcommand `debug_assert`
  panics in debug/test builds and double-lists in release. Suggestion: skip any
  manifest name that already exists as a subcommand or is reserved.
- minor (medium): Multi-token root-help forms (`accelerator --help extra`) lose
  sub-binary augmentation under the exact single-element predicate; the current
  predicate handles them. Suggestion: accept explicitly or match a leading-token
  pattern.
- suggestion (low): Bare invocation now emits a full help body on stdout with
  empty stderr while exiting non-zero. Suggestion: confirm no hook/script depends
  on the bare-invocation stderr text and that the stream split is intended.

### Usability

**Summary**: From a developer-experience standpoint this plan is a clear net
win: it converges three divergent help entry points on one identical,
manifest-driven listing, replaces launcher-internal jargon with user-facing verb
phrases, and preserves per-command `--help` so progressive disclosure stays
intact. The main wrinkles are stylistic/behavioural: the merged list mixes two
description styles, and bare invocation now prints a help-looking listing to
stdout while exiting non-zero with no cue explaining the failure.

**Strengths**:
- Converging all three entry points on an identical listing is a strong
  least-surprise improvement, proven by a diff in manual verification.
- Rewriting seven launcher-internal descriptions into user-facing verb phrases is
  a significant first-run DX gain.
- Progressive disclosure preserved: per-command help still routes to the specific
  command.
- Driving the listing from the signed manifest means new sub-binaries appear
  automatically.
- clap-native injection lets clap own alignment and terminal-width wrapping.
- The `-h` short flag is explicitly handled alongside `--help`/`help`.

**Findings**:
- minor (high): The merged list mixes full-sentence built-in descriptions (with
  trailing periods) and terse imperative sub-binary fragments (without).
  Suggestion: align both on one convention, or call out the built-in restyle as
  an explicit dependency.
- minor (medium): Bare invocation loses its actionable error cue while still
  exiting non-zero, and the diagnostic moves from stderr to stdout. Suggestion:
  prepend a one-line usage cue, ideally to stderr.
- suggestion (medium): The merged listing ordering is inconsistent (declaration
  order then alphabetical). Suggestion: decide and document an intended order.
- suggestion (low): "Serve the meta-directory visualiser" is less discoverable
  than the intent; consider "Launch"/"Open" or note the accuracy trade for 0259.

### Compatibility

**Summary**: This plan reworks three CLI entry points onto one merged listing
and rewrites seven crate `description` fields. The behaviour-contract surface is
well understood and mostly safe: no external consumers of the `description`
fields beyond `tasks/manifest.py`, no scripts/docs parse the `External
subcommands:` heading, the manifest schema and signing contract are untouched,
and exit codes are explicitly preserved. The residual risks are narrow — a
stderr→stdout stream change for bare invocation, and the merge coupling
signature-verified manifest names to clap's command-builder naming constraints.

**Strengths**:
- The merged listing is driven by the signed manifest — additive and
  forward-compatible.
- Research verified the `description` field has no other consumers (no cargo
  publish, no `build.rs`, no completion/man-page generation).
- Exit-code contract preserved explicitly; the plan corrects the work item's
  wrong error-kind assumption.
- clap is pinned exactly (`=4.6.1`) and tests assert on substrings, not exact
  layout.
- The signed manifest is version-pinned, so no cross-version description skew.

**Findings**:
- minor (medium): Bare invocation output moves from stderr to stdout while
  keeping exit 1. Suggestion: make the stream choice explicit in the plan and
  confirm no hook/script consumes bare `accelerator` on stderr.
- minor (medium): The merge feeds manifest names into `clap::Command::new`,
  coupling manifest content to clap's naming rules; a name colliding with a
  built-in or an empty name panics (`debug_assert`) or double-lists.
  `sanitize()` handles neither. Suggestion: skip/handle collisions and empties,
  add a fixture test.
- suggestion (low): `test_github.py:_SUBBINARY_DESCRIPTIONS` is left as a stale
  mirror of the retired "The … sub-binary." strings. Suggestion: align the
  fixture or reaffirm the strings are placeholders.

### Documentation

**Summary**: Phase 1 replaces launcher-internal crate descriptions with concise,
audience-appropriate one-liners that surface as CLI help — a clear improvement,
and the plan correctly traces the description source of truth and the single
Python test mirror that must move with it. However, the plan asserts `migrate`
"already matches" the AC-5 text when it actually carries a trailing full stop
that the intended text and all eight rewritten descriptions lack, leaving the
merged listing punctuation-inconsistent and off-spec against AC-5.
External-docs currency is sound: no docs-site page or README documents the
"External subcommands:" heading or the old descriptions.

**Strengths**:
- The rewrites replace internal jargon with clear end-user-facing descriptions in
  a consistent imperative mood.
- The plan correctly identifies the description source of truth and updates the
  one Python assertion that pins a real crate description.
- Verified: no docs-site page or README references the old heading, behaviour, or
  descriptions, so no cross-references go stale.

**Findings**:
- major (high): `migrate` keeps a trailing period the AC-5 text omits, breaking
  listing consistency; it will be the only entry ending in a period. Suggestion:
  drop the period from `cli/migrate-cli/Cargo.toml` and reconcile the research
  table that marks it "already matches".
- minor (medium): "Validate and process design sources" under-describes the
  design sub-binary's surface (resolve-auth, scrub-secrets, notify-downgrade,
  audit-cue-phrases). Suggestion: confirm the AC-5 wording or note broader wording
  is deferred to 0259.
- suggestion (low): The corpus parenthetical omits the live `linkage` subcommand
  while reading as an exhaustive list. Suggestion: include it or frame it as
  illustrative.

## Re-Review (Pass 2) — 2026-09-05

**Verdict:** REVISE

All nineteen findings from the initial review are resolved. The revision threaded
the parsed args into a pure `classify`/`is_root_help_args` pair, added the
collision guard, the `config --help` and phrasing-guard tests, the migrate-period
diff, the stderr cue, and the 0259 boundary callouts — every one verified against
the source. The verdict stays REVISE because the now-concrete plan drew concrete
new scrutiny: four new major findings, one of them a genuine regression the
`classify` design introduced. None are structural; all are localised fixes.

### Previously Identified Issues

All resolved; two carry a follow-on caveat now tracked as a new issue.

- ✅ **Documentation** — migrate trailing period — Resolved (diff added, both stale
  notes reconciled; verified all nine descriptions are period-free).
- 🟡 **Architecture** — bare invocation network-bound — Resolved in intent, but the
  documented bound is inaccurate (see new issues).
- ✅ **Code Quality** — routing read the `args_os()` global — Resolved (pure
  `classify` threaded args from a single collect).
- ✅ **Test Coverage** — AC-6 all-three convergence — Resolved (`classify` truth
  table proves all three kinds reach the one full-listing route).
- ✅ **Test Coverage** — `config --help` e2e guard — Resolved (black-box test with a
  strong `cache`-absence discriminator).
- 🟡 **Correctness/Compatibility/Architecture** — clap namespace collision —
  Resolved for the declared built-ins, but the guard misses clap's synthesised
  `help` (see new issues).
- ✅ **Correctness** — multi-token `--help extra` — Resolved (leading-token match).
- ✅ **Usability** — mixed description register — Resolved (deferred to 0259 with an
  explicit hand-off note).
- ✅ **Usability/Compatibility/Correctness** — bare error cue and stderr→stdout —
  Resolved (stderr cue restores the diagnostic stream).
- ✅ **Code Quality** — needless `Vec` allocation — Resolved (slice match).
- ✅ **Code Quality** — fail-open/lazy-init rationale — Resolved (carry-across note).
- ✅ **Documentation** — `design-cli` under-describes — Resolved (verbatim AC-5,
  deferred).
- ✅ **Usability/Documentation** — ordering, `Serve`, `corpus` linkage — Resolved
  (documented as AC-5/0259 decisions).
- ✅ **Test Coverage** — AC-5 phrasing regression guard — Resolved (substring test).
- ✅ **Compatibility** — `test_github` fixture — Resolved (existing comment
  documents the placeholders; verified).
- ✅ **Code Quality/Architecture** — `package.description` overload — Resolved
  (checklist note + recorded tradeoff).

### New Issues Introduced

#### Major

- 🔴 **Correctness**: Blanket `MissingSubcommand → FullListing` hijacks nested
  subcommand-group errors. `accelerator config templates` (the required
  `TemplatesAction` group, no `arg_required_else_help`) also raises
  `MissingSubcommand`; the new arm would print the generic cue plus the top-level
  listing instead of clap's contextual error — a regression from today's
  `error.print()`. Fix: gate on `args.is_empty()` and let non-root
  `MissingSubcommand` fall through to `UsageError`; pin with a `(MissingSubcommand,
  ["config","templates"])` case. *High confidence.*
- 🟡 **Test Coverage**: The wired `augment_with_subbinaries` call in
  `render_full_listing` is never exercised — every black-box test forces the
  manifest unavailable, so the augment branch (the core deliverable) survives
  deletion. Fix: extract a pure `build_listing(command, Option<&Manifest>)` and
  unit-test that `Some` augments and `None` does not. *High confidence.*
- 🟡 **Test Coverage**: The deliberately-added bare stderr cue has no assertion;
  the black-box bare test checks only stdout and exit 1. Removing the cue passes
  every test. Fix: assert the cue lands on stderr.
- 🟡 **Architecture**: Performance Considerations states a "10s connect bound", but
  the shared `Fetcher` retries up to 3× and carries a 300s per-attempt total
  timeout, so a stalling host bounds bare invocation far more loosely (~30s on
  SYN-drop; minutes on a stalled read). Fix: correct the worst-case text, or give
  the help-path fetch a tighter single-attempt deadline distinct from dispatch.

#### Minor

- 🔵 **Correctness/Compatibility/Architecture**: The collision guard misses clap's
  synthesised `help`. `Cli::command()` is un-built at augmentation time and clap
  appends `help` unconditionally during `_build` with no dedup, so
  `find_subcommand("help")` returns `None` and a manifest binary named `help`
  slips past — the exact panic/double-list the guard claims to close. The stated
  rationale ("already carries clap's synthesised `help`") is factually wrong. Fix:
  also skip the literal `help` (or `disable_help_subcommand`), and test the `help`
  token. Flagged by three lenses.
- 🔵 **Code Quality**: `HelpRoute::FullListing { exit: u8, missing_subcommand:
  bool }` permits contradictory states and spreads bare-path logic across three
  sites. Fix: model `RootHelp` and `MissingSubcommand` as distinct variants and
  derive exit/cue from the variant.
- 🔵 **Code Quality**: The existing `main.rs:131-134` rationale — forcing stdout
  because clap routes `DisplayHelpOnMissingArgumentOrSubcommand` to stderr — is not
  in the carry-across list, so the `PerCommand` arm risks losing it.
- 🔵 **Test Coverage**: The `classify` truth table omits the
  `DisplayHelpOnMissingArgumentOrSubcommand → PerCommand` case (bare
  `config`/`cache`), a pre-existing behaviour the refactor must preserve.
- 🔵 **Test Coverage**: The collision test bundles the empty-name and `version`
  cases into one assertion; clap accepts `Command::new("")` without panicking, so
  the `is_empty()` guard's mutation survives. Fix: assert no blank subcommand
  entry distinctly.
- 🔵 **Documentation**: Current State Analysis says `visualiser` "differs only in
  verb", but the edit also drops "interactive".
- 🔵 **Documentation**: The `tasks/README.md` checklist note has no placement;
  adding a fourteenth point would stale the "thirteen-point" references in two
  CLAUDE.md files. Fix: extend the existing point 3.

#### Suggestions

- 🔵 **Usability**: Pull the trivial trailing-period strip on the three built-in
  `///` doc comments into Phase 1 alongside migrate's, delivering the
  punctuation-uniform merged list the plan already targets (leaving the
  sentence-vs-fragment register to 0259).
- 🔵 **Usability**: The stderr cue points nowhere; append "run `accelerator --help`
  to see available commands".
- 🔵 **Usability**: Flag to 0259 that core-then-domain ordering surfaces infra
  commands above the primary domain verbs — the restyle should choose
  consciously.
- 🔵 **Test Coverage**: Extend the phrasing guard to also assert no description ends
  with `.`, guarding the no-trailing-period convention the plan establishes.
- 🔵 **Compatibility**: Add a black-box assertion that the cue lands on stderr while
  the listing lands on stdout (the stream-separation contract).

### Assessment

The plan is close. Every prior finding is genuinely resolved, and the structural
design — clap-native injection, a pure classifier, fail-open loading — is sound
and unchanged. One new major is a real regression (`config templates`), and the
others are test-composition and accuracy gaps rather than design faults; all have
localised fixes named above. One more revision pass addressing the four majors —
the `args.is_empty()` gate, the `build_listing` extraction, the stderr assertion,
and the corrected timeout text — plus the `help`-collision minor should bring it
to APPROVE.

---
*Re-review generated by /accelerator:review-plan*

## Re-Review (Pass 3) — 2026-09-05

**Verdict:** COMMENT

Every pass-2 finding — the four majors and the seven minors/suggestions — is
resolved and verified against source across all seven lenses. No critical or
major findings remain, so the plan is acceptable as-is. The pass surfaced only
minor polish and refinements, most of them prose accuracy, test realism, or
implementation guidance rather than design or behaviour defects.

### Previously Identified Issues

All resolved and verified.

- ✅ **Correctness** — nested-group hijack (`config templates`) — `args.is_empty()`
  gate confirmed correct: bare has empty argv, any nested `MissingSubcommand` is
  non-empty and falls to `UsageError`.
- ✅ **Test Coverage** — wired augment untested — `build_listing` extracted, both
  arms unit-tested.
- ✅ **Test Coverage** — bare stderr cue unasserted — black-box now asserts the
  stream split.
- ✅ **Architecture** — timeout bound understated — help-scoped single-attempt
  fetcher; Performance text corrected (one residual refinement below).
- ✅ **Correctness/Compatibility/Architecture** — `help` collision — explicit
  `name == "help"` skip, correctly justified against clap's lazy `_build`.
- ✅ **Code Quality** — `HelpRoute` contradictory states — `FullListing(ListingCause)`.
- ✅ **Code Quality** — stdout-forcing rationale — added to the carry-across list.
- ✅ **Test Coverage** — `classify` table gap, bundled empty-name — both closed.
- ✅ **Documentation** — visualiser prose, checklist placement — both corrected
  and verified (checklist runs to exactly thirteen points).
- ✅ **Usability/Test Coverage** — built-in periods, cue pointer, ordering,
  no-period guard — all applied.

### New Issues Introduced

No critical or major findings.

#### Minor

- 🔵 **Architecture**: The help-scoped `Fetcher` sets `max_attempts = 1`, but the
  10s connect and 300s total timeouts are module constants baked into
  `Fetcher::build()`, so a single attempt still inherits the 10s connect — not the
  "couple of seconds" the plan asserts. Achieving that needs an explicit
  `fetcher.rs` change (parameterised or help-scoped timeout constants) with
  concrete values and a test; add a Changes Required entry, or restate the bound
  as one ~10s connect timeout.
- 🔵 **Compatibility**: The tightened single-attempt fetch makes AC-6's "three
  identical listings" probabilistic near the timeout boundary — each entry point
  fetches independently, so on a marginal link one can show sub-binaries and the
  next fall open to built-ins. Note the `diff`-empty check assumes a stable
  manifest-load result.
- 🔵 **Test Coverage**: The `config templates` regression has only a synthetic
  `classify` case plus manual verification; add a black-box case mirroring the
  `config --help` one.
- 🔵 **Test Coverage**: `render_full_listing`'s manifest-passing call site is still
  unreachable by assertion (the no-forged-manifest constraint), so "guards the
  wired augmentation against deletion" overstates it — acknowledge the limit and
  keep the function a one-line delegation.
- 🔵 **Correctness/Test Coverage**: The `classify` truth-table row `(DisplayVersion,
  ["version","--help"])` is fictional — with `disable_version_flag = true`,
  `version` is a subcommand and `version --help` yields `DisplayHelp`; clap never
  raises `DisplayVersion` for this CLI. Relabel or annotate the arm as an
  unreachable-kind guard.
- 🔵 **Code Quality**: The exit-2→1 usage-error remap rationale (`main.rs:119-122`)
  is not in the carry-across list; add it so the `UsageError` arm keeps its "why".
- 🔵 **Code Quality**: The stderr cue is emitted as a side effect inside the
  exit-code-computing `match cause`; consider emitting it in a separate step so
  render, cue, and exit read distinctly.
- 🔵 **Documentation**: The "full sentences vs terse fragments" register framing is
  undercut by its own data (both are imperative; the built-in exemplar is shorter
  than a shipped sub-binary description) — reframe as verbosity/detail parity.
- 🔵 **Documentation**: The Testing Strategy phrasing-guard bullet describes only
  the `sub-binary` substring assertion, not the newly-added trailing-period one.

#### Suggestions

- 🔵 **Compatibility**: `augment_with_subbinaries` renames the public
  `external_subcommands_section`; add a step to refresh the cargo-public-api
  baseline (or confirm exemption) so `mise run` stays green.
- 🔵 **Test Coverage**: Specify the phrasing guard iterates `DISPATCHED_SUBBINARIES`
  reading descriptions only (not `collect_entries`, which needs staged bytes), so
  it stays hermetic and auto-covers new sub-binaries.
- 🔵 **Test Coverage**: Assert collision dedup via `get_subcommands()` counts rather
  than a substring count of `version` (which appears in its own about text).
- 🔵 **Correctness**: `sanitize()` strips control chars but not whitespace, so a
  name like `foo bar` renders oddly; skip non-token names or record the upstream
  validation invariant.
- 🔵 **Usability**: The cue points to `--help`, which reproduces the listing already
  on stdout in a shared terminal — record as an accepted stderr-only-capture
  tradeoff.
- 🔵 **Usability**: Flag `corpus`'s three-of-four parenthetical (omitting `linkage`)
  in the 0259 hand-off.

### Assessment

The plan is ready. Its structural design — clap-native injection, a pure
classifier over `(ErrorKind, args)`, `build_listing` composition, fail-open
loading with a help-scoped fetch — is sound and well-tested, and no behaviour or
correctness defect remains. The residual items are polish: the architecture and
compatibility minors are the two with real substance (pin the help fetcher's
timeouts concretely, and frame the identical-listing check as assuming a stable
load), and the rest are prose accuracy, test realism, and implementation-note
refinements that can be handled during implementation. Three review passes have
reached clear diminishing returns.

---
*Re-review generated by /accelerator:review-plan*

## Approval — 2026-09-06

**Verdict:** APPROVE

Approved for implementation. The two substantive pass-3 minors were applied
after the pass: the help fetcher's timeouts are now pinned concretely
(`Fetcher::for_help()` — 3s connect, 5s total, `max_attempts = 1` — via a
parameterised `build()`, Phase 2 §6), and the identical-listing check is now
framed as assuming a stable manifest-load result (Desired End State; the manual
`diff` bullet). The remaining pass-3 items — a fictional `DisplayVersion`
truth-table row, an overstated "guards against deletion" claim, two documentation
prose nits, a cargo-public-api baseline note, and assorted test-realism and 0259
hand-off refinements — are non-blocking and folded into implementation. Plan
status set to `ready`.
