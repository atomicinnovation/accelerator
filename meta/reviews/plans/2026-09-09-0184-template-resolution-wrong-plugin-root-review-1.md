---
type: "plan-review"
id: "2026-09-09-0184-template-resolution-wrong-plugin-root-review-1"
title: "Plan Review: Template Resolution Wrong-Root Refusal Implementation Plan"
date: "2026-09-09T08:03:18+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-09-0184-template-resolution-wrong-plugin-root"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "correctness", "code-quality", "test-coverage", "usability", "standards"]
review_number: 1
review_pass: 2
tags: ["cli", "config", "templates", "plugin-root", "visualiser"]
last_updated: "2026-09-09T16:29:30+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Template Resolution Wrong-Root Refusal Implementation Plan

**Verdict:** COMMENT

The plan is well-grounded and implementable: the correctness lens verified its
central branching against the live `store.rs` and found the `require_templates_dir`
match arms exhaustive, the tier ordering preserved, and the genuine-not-found
`Ok(None)` intact across all three plugin-default sites; the test-coverage lens
confirmed complete AC1–AC12 traceability across a well-balanced three-layer
pyramid. The one finding that recurs across two lenses is that the fix routes the
*wrong-root* case through the existing `PluginRootUnavailable` variant, whose
message tells a developer who has already set `ACCELERATOR_PLUGIN_ROOT` to "set
`ACCELERATOR_PLUGIN_ROOT`" and never names the offending path — undercutting the
diagnostic-quality goal that is the entire point of the work item. It is
acceptable as a scoped tradeoff, but the plan does not currently acknowledge it as
one. Plan is acceptable but could be improved — see the major finding below.

### Cross-Cutting Themes

- **The reused `PluginRootUnavailable` diagnostic mis-describes a set-but-wrong
  root** (flagged by: architecture, usability) — both lenses independently land on
  the same defect from different angles. Architecture sees a taxonomy that
  conflates two distinct domain conditions (absent root vs present-but-not-an-
  installation) behind one variant whose `Display` says the root is "unknown";
  usability sees a self-contradictory remediation ("set a variable you have set")
  that omits the path a developer needs to diagnose. The ACs pass regardless
  because they only assert the message *contains* the variable name.
- **The `Io` "not a directory" branch is under-served** (flagged by:
  test-coverage, usability, correctness) — the `templates`-is-a-file case attracts
  three separate concerns: its arm and the `read_dir` `map_err` line are never
  exercised (both AC8 tests hit the not-a-directory arm, not the genuine-fault
  arm); its bare "I/O error" message gives the developer no thread back to the
  plugin root; and a TOCTOU race between the helper's `stat` and the later
  `read_dir` could misclassify a `NotFound` as `Io`.
- **Detection is centralised but adjacent duplication is left in place** (flagged
  by: architecture, code-quality) — the plan consolidates the *validity check* into
  one helper, but the `<plugin-root>/templates` *layout fact* is still
  reconstructed in `compose.rs`, and the `require_templates_dir()?.join("{name}.md")`
  path-building plus is-file-then-resolve shape is duplicated between
  `resolve_template`'s tier and `plugin_default`.

### Tradeoff Analysis

- **Diagnostic precision vs minimal surface**: The plan's minimal-surface instinct
  — reuse `PluginRootUnavailable`, touch the single `plugin_template_path` choke,
  add no new variant — is what keeps it small and low-risk, but it is also the
  direct cause of the major finding. A dedicated `PluginRootNotAnInstallation
  { path }` variant would resolve the mis-diagnosis at the cost of a new error
  variant, a new message, and touching `error.rs`. Recommendation: take the
  precision, because the work item exists specifically to improve this
  diagnostic; if minimal surface wins instead, record the message wording as a
  conscious, documented tradeoff and pin an AC that the wrong-root message differs
  from the absent-root message.

### Findings

#### Critical

None.

#### Major

- 🟡 **Architecture + Usability**: Wrong-root refusal reuses the absent-root
  message, which mis-describes a set-but-wrong root
  **Location**: Desired End State / Implementation Approach (`require_templates_dir`
  returning `PluginRootUnavailable`)
  The plan maps a present-but-invalid root onto `ConfigError::PluginRootUnavailable`,
  whose message reads "the plugin installation root is unknown: set
  `ACCELERATOR_PLUGIN_ROOT`, or invoke accelerator through `bin/accelerator`"
  (`error.rs:131`). On a wrong root the root is *known* — the developer set it —
  so the remediation is a no-op they have already done, the message names neither
  the offending path nor the real cause (a missing `templates/`), and a developer
  could reasonably read it as an env-passing bug. The ACs and
  `assert_names_the_plugin_root` only assert the message *contains* the variable, a
  weak proxy for actionability that this misleading text passes.

#### Minor

- 🔵 **Test Coverage**: Genuine-I/O-fault arm of `require_templates_dir` (and the
  `read_dir` `map_err` in `template_names`) is never exercised
  **Location**: Implementation Approach; Phase 1 §1–§3
  The helper has four outcomes but the plan tests three. Both AC8 tests trigger the
  fault by making `templates` a *file*, so `fs::metadata` succeeds and the
  not-a-directory `Ok(_)` arm fires — the `Err(other) => io_error(..)` arm and the
  new `read_dir(..).map_err(..)?` line (unreachable once metadata confirms a
  directory) both stay green under any mutation.

- 🔵 **Standards + Code Quality**: The `require_templates_dir` doc comment breaks
  the 80-column limit and partly restates its own match arms
  **Location**: Implementation Approach (the helper snippet)
  The second doc line is 81 chars as written, ~85 once indented into the impl
  block, breaking the repo-wide 80-col rule; rustfmt does not reflow doc comments
  (`wrap_comments` off), so `mise run cli:check` will not catch it — the identical
  silent gap recorded before. Separately, the comment's opening restates the name
  and signature and its closing narrates the two `Io` arms verbatim; only the
  invariant clause (why `NotFound` → `PluginRootUnavailable`) earns its place under
  the project's comment rule. Trimming to the invariant clause fixes both.

- 🔵 **Usability**: A `templates` path that is a file surfaces as a bare "I/O
  error" with no pointer back to the plugin root
  **Location**: Implementation Approach (`require_templates_dir` `Io` branch) /
  Phase 1 AC8
  Two near-identical misconfigurations — `templates/` missing vs `templates` being
  a file — produce wildly divergent diagnostic classes (an installation refusal vs
  "I/O error on '<root>/templates': not a directory"), and AC8 asserts the file
  variant does *not* name the plugin root, leaving the developer no thread back to
  their `ACCELERATOR_PLUGIN_ROOT` setting for what is a plugin-root problem.

- 🔵 **Usability**: The visualiser compose error contradicts the server's own
  "var is set" check and offers launcher-only remediation
  **Location**: Phase 3: Compose path surfaces the refusal (R3)
  `run_serve` first confirms `ACCELERATOR_PLUGIN_ROOT` *is* set (`main.rs:68`), then
  the composed failure prints "…the plugin installation root is unknown: set
  `ACCELERATOR_PLUGIN_ROOT`, or invoke accelerator through `bin/accelerator`". Within
  one binary the developer is told the variable is set and then that it is unknown,
  plus remediation ("`bin/accelerator`") that describes the launcher, not the server
  they are running.

- 🔵 **Test Coverage**: AC11 asserts non-empty output but never runs the
  `--fail-safe` path its rationale targets
  **Location**: Phase 1 §3 (`a_root_independent_family_still_succeeds_against_a_wrong_root`)
  AC11's stated reason for demanding non-empty output is that the `config` family
  carries `--fail-safe`, which degrades a failing read to exit-0 empty output — yet
  the planned test runs `config paths` *without* the flag, so it never exercises the
  degrade path the assertion is designed to distinguish from real success.

- 🔵 **Architecture**: The `templates/` layout fact remains reconstructed in
  `compose.rs`
  **Location**: Phase 3: Compose path (relationship to `require_templates_dir`)
  The helper centralises the `<plugin-root>/templates` path and its validity check
  inside `FileConfigStore`, but the compose path independently rebuilds the same
  layout with `plugin_root.join("templates")` (`compose.rs:155`) to populate each
  `TemplateTiers.plugin_default`. The validity gate is centralised; the *location*
  of `templates/` now lives in two crates, only one of which carries the invariant.

- 🔵 **Standards**: Store unit-test snippets drop the root `TempDir` immediately
  **Location**: Phase 1 §2 / Phase 2 §2 (store unit tests)
  The snippets build `FileConfigStore::at(tempdir()?.path())`, letting the root
  `TempDir` drop (and delete its directory) at the end of that statement — every
  existing `store.rs` unit test binds the handle first (`let root = tempdir()?;`),
  and the plan itself already does this for the *plugin* handle. It roots the store
  at an already-deleted path and breaks the module's uniform idiom.

- 🔵 **Correctness**: TOCTOU between the helper's `stat` and `template_names`'
  `read_dir` can misclassify `NotFound` as `Io`
  **Location**: Phase 1 §1 (the shared helper and `template_names`)
  `template_names` stats `templates/` through the helper (mapping `NotFound` →
  `PluginRootUnavailable`), then does a separate `read_dir` whose error arm maps
  *every* failure — including `NotFound` — to `Io`. If `templates/` is removed
  between the two calls, the racing `NotFound` becomes the degradable `Io` and,
  under `--fail-safe`, exit-0 empty output — the exact mode this item closes.
  Negligible window on a cold path, but inconsistent with the helper's own
  classification.

#### Suggestions

- 🔵 **Code Quality**: Residual path-building duplication between
  `resolve_template`'s tier and `plugin_template_path`
  **Location**: Phase 2 §1
  After Phase 2, `require_templates_dir()?.join(format!("{name}.md"))` and the
  is-file-then-resolve shape exist in both `resolve_template`'s inline tier and
  `plugin_default`/`plugin_template_path`, differing only in the `warning` argument.
  Having the tier build its candidate via `self.plugin_template_path(name)?` would
  keep the plugin-default path concept in one place; if the differing `warning`
  makes full unification awkward, note the residual duplication as deliberate.

- 🔵 **Test Coverage**: No fast store-unit test for the override-precedes-plugin-
  default crux
  **Location**: Phase 2 §2
  The hoisting hazard (the root check must sit *inside* each plugin-default step) is
  pinned only at the slow CLI layer (AC10). A `config-adapters` unit test that seeds
  a user override under a wrong root and asserts `resolve_template` returns
  `Ok(Some(UserOverride))` would catch a hoisting regression in seconds.

- 🔵 **Test Coverage**: The AC9 characterisation-test swap can skip an observable
  red
  **Location**: Phase 1 §3
  Replacing the passing characterisation test with its inverse in the same change as
  the production edit means the new test's red state is only observed if the author
  deliberately stages it first. Note in the phase that the inverse test is written
  and seen failing against the unmodified `template_names` before the swallow is
  replaced.

- 🔵 **Architecture**: The helper is templates-specific despite the "is this an
  installation?" framing
  **Location**: Decisions / Implementation Approach
  `require_templates_dir` hard-codes `join("templates")`, coupling the installation-
  validity concept to one resource; a future plugin-root accessor for a different
  family could not reuse it cleanly. YAGNI-bounded — factor a general
  `require_installation_root()` only if a second consumer is anticipated; otherwise
  reconcile the Decisions wording with the templates-specific name.

- 🔵 **Standards**: The helper name reads as a path accessor but encodes an
  installation-validity refusal
  **Location**: Implementation Approach
  `require_templates_dir` looks like a plain path accessor, yet a missing
  `templates/` surfaces `PluginRootUnavailable` (a refusal naming
  `ACCELERATOR_PLUGIN_ROOT`). Consistent with the `require_*` gate family, but a
  name like `require_installed_templates_dir` would signal the refusal at the call
  site. A judgement call — hence low confidence.

### Strengths

- ✅ The central branching is verified sound against live code: `require_templates_dir`'s
  match arms are exhaustive (`Ok(is_dir)`→path, `Ok(_)`→`Io "not a directory"`,
  `Err(NotFound)`→`PluginRootUnavailable`, `Err(other)`→`io_error`), and the
  referenced free functions and imports already exist.
- ✅ The root check is correctly placed *inside* each plugin-default tier, after the
  config-path and user-override tiers, so a resolving override still renders at exit
  0 under a wrong root (AC10) — the R4 invariant is preserved, not hoisted away.
- ✅ Genuine template-not-found (`Ok(None)`) survives across all three plugin-default
  paths, and routing the fix through the single `plugin_template_path` choke fixes
  `template`, `eject`, `diff`, and `reset` from one edit without duplicating the
  branch.
- ✅ Complete AC1–AC12 traceability across a well-balanced test pyramid: fast
  `config-adapters` store unit tests for the red-green loop, black-box `config_read.rs`
  pins for the observable surface, and one compose contract test at the server
  boundary — with the not-found/refusal split double-covered and mutation-resistant.
- ✅ The behavioural change lands in the shared `config-adapters` library, so both
  binaries and the compose consumer inherit it through existing `?` / `#[from]`
  abstractions with no consumer modification, in lockstep via 0182's version-keyed
  cache.
- ✅ The change conforms closely to existing conventions: it reuses the
  `ErrorKind::NotFound` split idiom and the `io_error` helper, names the gate in the
  established `require_*` family, and encodes the careful absent-vs-wrong-root
  distinction in its test names.
- ✅ The refusal classification is load-bearing and correct: `PluginRootUnavailable.is_refusal()`
  is `true`, so wrong-root refusals fail closed byte-identically with and without
  `--fail-safe`, while the `Io` arm stays degradable — matching AC8.

### Recommended Changes

1. **Resolve the wrong-root diagnostic** (addresses: the major finding; the compose
   contradiction and the bare-I/O usability minors). Either introduce a dedicated
   `PluginRootNotAnInstallation { path }` variant whose message names the offending
   directory and the missing-`templates/` cause (dropping the launcher-only
   `bin/accelerator` remediation for the server path), or consciously keep
   `PluginRootUnavailable` and record the message reuse as a documented tradeoff in
   the plan. Whichever path is taken, add an AC asserting the wrong-root message
   differs from the absent-root message and names the path.
2. **Close the `Io`-arm coverage gap** (addresses: the genuine-I/O-fault minor).
   Add one store-unit test that induces a real fault on a *present* `templates/`
   directory (`chmod 0o000` with the non-root guard AC8 already prescribes) and
   asserts `Err(ConfigError::Io { .. })`, pinning both the stat-error arm and the
   `read_dir` `map_err`.
3. **Strengthen AC11 to exercise the flagged path** (addresses: the AC11 minor).
   Run `config paths --fail-safe` against a wrong root and assert exit 0 *and*
   non-empty real output, so the degrade path AC11 reasons about is actually run.
4. **Trim the helper doc comment to the invariant clause** (addresses: the merged
   doc-comment minor). Keep only the "why `NotFound` → `PluginRootUnavailable`"
   rationale; drop the signature restatement and the `Io`-arm narration, which also
   brings every line under 80 columns.
5. **Fix the store-unit-test `TempDir` idiom** (addresses: the `TempDir` minor).
   Bind the root handle first (`let root = tempdir()?;`) before passing `.path()` to
   `FileConfigStore::at`, matching the surrounding tests.
6. **Decide the residual duplication explicitly** (addresses: the compose-layout and
   path-building findings). Either derive the compose plugin-default paths and the
   `resolve_template` tier from single-owner accessors, or note both as deliberate,
   out-of-scope for this fix.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan is structurally sound: it consolidates installation-detection
into a single shared helper (`require_templates_dir`) called by all three
plugin-root template accessors, places the check inside each plugin-default step so
the tiered resolution architecture and resolving-override property survive, and
lands the change in the shared `config-adapters` library so both binaries and the
compose consumer inherit it through existing abstractions (`?`/`#[from]`) with no
consumer modification. The one meaningful architectural concern is that it overloads
the existing `PluginRootUnavailable` error variant — whose message asserts the root
is 'unknown' — to model a semantically distinct condition (root present but not an
installation), producing a diagnostic that mis-describes the very case this work
exists to diagnose. Two lesser observations concern incomplete centralisation of the
templates-dir location and the templates-specific coupling of a helper the design
framed as a general 'is-this-an-installation?' predicate.

**Strengths**:
- Detection consolidated into one shared helper called by all three accessors —
  good cohesion and a single source of truth for both the templates-dir location and
  the installation-validity check, which is also the justification for keeping the
  three accessors bundled.
- The check is correctly placed *inside* each plugin-default step, preserving the
  tiered resolution architecture and the resolving-override-under-wrong-root property
  (AC10).
- The behavioural change lives in the shared `config-adapters` library, so launcher,
  the six CLI commands, and the visualiser server inherit it in lockstep; the server
  picks it up through existing `?` propagation and `ComposeError::Config(#[from])`
  with no production change — a clean open-closed extension.
- Respects the pre-existing fail-closed-for-answers vs degrade-for-hints boundary,
  and correctly leaves `known_skill_names` tolerant rather than sweeping it in.
- Introducing unit-level `FileConfigStore`-with-plugin-root tests pushes plugin-root
  behaviour below the slow black-box CLI layer.
- Choosing to hard-refuse at server compose rather than serve an empty template set
  is an appropriate fail-fast posture.

**Findings**:
- **major** (confidence: medium) — *Wrong-root case overloads a variant whose
  message says the root is 'unknown'* — Implementation Approach / Current State
  Analysis. Maps a present-but-invalid root onto `PluginRootUnavailable`, whose
  `Display` (`error.rs:131`) says the root is "unknown" and tells the user to set a
  variable they set; the server surfaces this even though `run_serve` already
  confirmed the variable is set. The taxonomy conflates absent-root and
  present-but-not-an-installation, and the ACs pass because they only require the
  message to *name* the variable. Suggestion: a dedicated
  `PluginRootNotAnInstallation { path }` variant, or an explicit documented tradeoff.
- **minor** (confidence: medium) — *Templates-dir location remains split between the
  store helper and `compose.rs`* — Phase 3. The validity gate is centralised, but
  `compose.rs:155` independently rebuilds `plugin_root.join("templates")`; the layout
  fact now lives in two crates. Suggestion: derive compose's plugin-default paths
  from a store-exposed accessor, or note the residual duplication.
- **suggestion** (confidence: low) — *Helper is templates-specific despite being
  framed as a general installation predicate* — Decisions / Implementation Approach.
  `require_templates_dir` hard-codes `join("templates")`; a future plugin-root
  accessor for another family could not reuse it cleanly. YAGNI-bounded.

### Correctness

**Summary**: The plan's central branching logic is sound and complete:
`require_templates_dir` exhaustively distinguishes `ErrorKind::NotFound`
(→`PluginRootUnavailable`), a present-but-non-directory `templates` entry (→`Io`),
and other I/O faults (→`Io`), and every plugin-default site preserves the `Ok(None)`
genuine-not-found while routing the wrong-root refusal ahead of the `is_file()`
fall-through. The tier ordering is correctly preserved — the root check sits inside
each plugin-default step, after the config-path and user-override tiers — so a
resolving override under a wrong root still renders. The one correctness wrinkle is a
non-atomic double filesystem probe in `template_names` whose `read_dir` error arm
maps every error to `Io`, diverging from the helper's `NotFound`→`PluginRootUnavailable`
classification in a narrow TOCTOU window; practically negligible on this cold path.

**Strengths**:
- `require_templates_dir`'s match arms are exhaustive and each maps to the intended
  variant; verified `display()`, `io_error()`, and the `ErrorKind` import already
  exist.
- The plugin-default root check is correctly placed inside the plugin-default tier
  (after config-path and user-override tiers), so AC10 / the R4 invariant is
  preserved.
- Genuine template-not-found (`Ok(None)`) is preserved across all three
  plugin-default paths.
- Routing the fix through `plugin_template_path` (the single choke both
  `plugin_default` and `eject` call) is a correct minimal-surface choice.
- The refusal-vs-degradable classification is load-bearing and correct:
  `is_refusal()` is `true`, so wrong-root refusals fail closed identically with and
  without `--fail-safe`, while the `Io` arm stays degradable (AC8).
- No concurrency correctness concern introduced: `FileConfigStore` is a
  stateless-over-filesystem `Clone`; the watcher rebuilds from a cloned map without
  re-enumerating.

**Findings**:
- **minor** (confidence: low) — *Non-atomic double probe in `template_names`* —
  Phase 1 §1. The helper stats `templates/` (mapping `NotFound` →
  `PluginRootUnavailable`), then a separate `read_dir` maps every failure — including
  `NotFound` — to `Io`. A TOCTOU removal between the two calls degrades to exit-0
  empty under `--fail-safe`. Suggestion: map the `read_dir` `NotFound` arm to
  `PluginRootUnavailable` for consistency, or note the arm is helper-guaranteed
  impossible.

### Code Quality

**Summary**: A clean, well-factored quality change: a single `require_templates_dir`
helper consolidates installation detection across all three accessors, mirroring the
existing `require_plugin_root` naming and error-raising convention, and it replaces a
swallow-everything `let Ok(...) else { Ok(Vec::new()) }` with precise three-way error
categorisation. Testability is strong — it introduces store-level unit tests that
inject a plugin root through the existing `with_plugin_root` builder, and the phased
structure keeps the tree green at each step. The only quality nits are a doc comment
that partly restates its own match arms and a small residual path-building
duplication left in place.

**Strengths**:
- The shared helper is a genuine DRY consolidation: one detection point called by all
  three accessors, mirroring `require_plugin_root`'s `require_`-prefix convention and
  returning the resolved path so callers never re-join the segment.
- Error handling is a clear improvement — the previous swallow (collapsing `NotFound`,
  permission, and not-a-directory faults identically) is replaced with distinct,
  diagnosable outcomes.
- `fs::metadata` is a well-chosen single detection primitive serving both the
  enumerating and the path-building callers, avoiding an inline presence check
  duplicated per site.
- Testability is well-considered: injects a plugin root via the existing builder,
  backs it with black-box CLI pins and a compose contract test, drives every store
  change red-first.
- The three-phase split is cohesive; each phase is independently mergeable and leaves
  the tree green.

**Findings**:
- **minor** (confidence: high) — *Doc comment partly restates its own match arms* —
  Implementation Approach. The invariant clause (why `NotFound` →
  `PluginRootUnavailable`) is load-bearing and permitted, but the opening restates the
  name and signature and the closing narrates the two `Io` arms verbatim; the file's
  surrounding comments are terse and why-focused. Suggestion: trim to the invariant
  clause.
- **suggestion** (confidence: medium) — *Residual path-building duplication between
  `resolve_template`'s tier and `plugin_template_path`* — Phase 2 §1. The
  `require_templates_dir()?.join("{name}.md")` and is-file-then-resolve shape live in
  two near-identical spots, differing only in `warning`. Suggestion: build the tier's
  candidate via `plugin_template_path`, or note the duplication as deliberate.

### Test Coverage

**Summary**: The plan achieves complete AC-to-test traceability: all twelve criteria
map onto a well-balanced three-layer pyramid — fast `config-adapters` store unit
tests, black-box `config_read.rs` integration tests, and a single compose contract
test. Coverage is strong on the crux distinctions (genuine-not-found vs refusal at
both unit and CLI level, override-resolves-under-wrong-root, `NotFound` vs
not-a-directory) and reuses established scaffolding faithfully. The residual gaps are
narrow: one branch of the helper (a genuine I/O fault on a real `templates/`
directory) is never exercised, and AC11's fail-safe rationale is asserted without
running the flagged path.

**Strengths**:
- Complete AC1–AC12 mapping, with genuine-not-found (AC7) and IO-fault (AC8)
  double-covered at both store-unit and CLI layers.
- Balanced pyramid fitting the risk: red-green loop in fast unit tests, durable
  acceptance pins as black-box runs, exactly one server contract test.
- Mutation-resistant assertions on the crux split (stderr contains the template name
  AND not the variable; "I/O error" present AND variable absent).
- AC10 correctly recognised as a new composition distinct from the absent-root
  property, asserting rendered override content, not merely exit 0.
- The characterisation test is replaced in place by its inverse (AC9) rather than
  deleted, preserving the site's visibility.

**Findings**:
- **minor** (confidence: high) — *Genuine-I/O-fault arm never exercised* —
  Implementation Approach; Phase 1. Both AC8 tests make `templates` a file, so
  `metadata` succeeds and the not-a-directory arm fires; the `Err(other) => io_error`
  arm and the `read_dir` `map_err` line stay green under mutation. Suggestion: add a
  `chmod 0o000` store-unit test on a present `templates/` with the non-root guard.
- **minor** (confidence: medium) — *AC11 never runs the `--fail-safe` path* — Phase 1
  §3. The test runs `config paths` without the flag, so the degrade path its
  non-empty assertion targets is unverified. Suggestion: run `config paths
  --fail-safe` and assert exit 0 AND non-empty output.
- **suggestion** (confidence: low) — *No fast store-unit test for
  override-precedes-plugin-default* — Phase 2 §2. The hoisting crux is pinned only at
  the CLI layer (AC10). Suggestion: a `config-adapters` unit test seeding an override
  under a wrong root asserting `Ok(Some(UserOverride))`.
- **suggestion** (confidence: low) — *AC9 swap can skip an observable red* — Phase 1
  §3. Replacing a passing test with its inverse alongside the production edit can be
  authored to pass by construction. Suggestion: note the inverse test is seen failing
  first.

### Usability

**Summary**: The change delivers a real DX win in shape — replacing a silent empty
table / misleading template-not-found with a loud, non-zero refusal across all six
commands and the compose path, while preserving the genuine-missing-template message
and resolving overrides. But the entire stated value is diagnostic quality, and the
plan reuses the absent-root message verbatim for the wrong-root case: it tells a
developer who has *set* `ACCELERATOR_PLUGIN_ROOT` to a wrong path that the root 'is
unknown: set `ACCELERATOR_PLUGIN_ROOT`' — advice that is a no-op and can be read as
an env-passing bug, and it never names the offending path. The distinction the plan
nails is wrong-root-vs-missing-template; the distinction it collapses is
absent-root-vs-wrong-root, precisely the case being fixed.

**Strengths**:
- Turns a silent wrong answer into a loud, non-zero refusal — strictly more
  discoverable, the right least-surprise default.
- Preserves and message-distinguishes the genuine-missing-template case (AC7).
- User overrides still resolve at exit 0 under a wrong root (AC10).
- Root-independent families keep working with normal output (AC11) — no collateral
  regression.
- Behaviour is uniform across all six commands and the compose path, byte-identical
  with/without `--fail-safe`.

**Findings**:
- **major** (confidence: high) — *Wrong-root refusal reuses the absent-root message*
  — Desired End State / Implementation Approach. The `PluginRootUnavailable` message
  ("unknown: set `ACCELERATOR_PLUGIN_ROOT`") is self-contradictory for a set-but-wrong
  root, echoes no path, and could read as an env-passing bug; the ACs only assert the
  message *contains* the variable. Suggestion: a distinct diagnostic naming the path
  and the real cause, plus an AC that the wrong-root message differs and names the
  path.
- **minor** (confidence: medium) — *Compose error contradicts `run_serve`'s own
  check and offers launcher-only remediation* — Phase 3. Within one binary the server
  is told the variable is set (`main.rs:68`) then that it is unknown, with
  `bin/accelerator` remediation that describes the launcher. Suggestion: a
  server-standalone message without the launcher remediation.
- **minor** (confidence: medium) — *A `templates` file is reported as a bare "I/O
  error" with no pointer to the plugin root* — Implementation Approach `Io` branch /
  Phase 1 AC8. Two near-identical misconfigurations diverge into an installation
  refusal vs a bare "I/O error"; AC8 asserts the file variant does not name the root.
  Suggestion: word the detail to name the plugin-root expectation.

### Standards

**Summary**: The plan conforms closely to the `config-adapters` error conventions: it
reuses the `ErrorKind::NotFound` split idiom and the `io_error` helper, constructs
`ConfigError::Io` inline (matching `to_config_error`/`read_within`) for the
no-`std::io::Error`-in-hand 'not a directory' case, names the new gate in the existing
`require_*` family, and correctly uses per-crate test-result aliases and
absent-vs-wrong-root test vocabulary. The one genuine convention break is a doc-comment
line that exceeds the repo-wide 80-column limit with no rustfmt safety net; a
test-idiom deviation and a helper-naming nuance are minor. No API, web, or
accessibility standards apply.

**Strengths**:
- Mirrors the `ErrorKind::NotFound` split idiom of `custom_lenses`/`skill_names`,
  routing the non-`NotFound` branch through the shared `io_error` helper.
- Constructs `ConfigError::Io { path, detail }` inline for the file case (no
  `std::io::Error` in hand), consistent with `to_config_error`/`read_within`; the
  terse lowercase 'not a directory' detail matches the 'not writable' style.
- Names the gate `require_templates_dir`, following the `require_*` family.
- Test names encode the absent-vs-wrong-root distinction, keeping the suites
  discoverable and non-conflated.
- The doc comment documents a genuinely non-obvious invariant, within the codebase's
  comment carve-out.
- Reuses the exact existing scaffolding and mirrors the
  `an_empty_plugin_root_refuses_to_compose` contract-test shape.

**Findings**:
- **minor** (confidence: high) — *Doc-comment line breaks the 80-column convention
  with no rustfmt net* — Implementation Approach / Phase 1 §1. The second doc line is
  81 chars, ~85 indented; rustfmt does not reflow doc comments, so `cli:check` will
  not catch it. The `read_dir` chain also reaches ~83 cols but rustfmt will reformat
  that code line. Suggestion: rewrap the doc comment ≤80 and pre-break the `read_dir`
  chain as rustfmt will.
- **minor** (confidence: medium) — *Store unit-test snippets drop the root `TempDir`
  immediately* — Phase 1 §2 / Phase 2 §2. `FileConfigStore::at(tempdir()?.path())`
  lets the root `TempDir` drop; every existing test binds the handle first, as the
  plan already does for the plugin handle. Suggestion: bind the root to a local first.
- **suggestion** (confidence: low) — *Helper name reads as a path accessor but
  encodes an installation-validity refusal* — Implementation Approach. `self.require_templates_dir()?`
  does not signal that an `ACCELERATOR_PLUGIN_ROOT` refusal can surface. Suggestion:
  `require_installed_templates_dir`, or lean on the doc comment. A judgement call.

## Re-Review (Pass 2) — 2026-09-09

**Verdict:** APPROVE

All six lenses re-ran on the revised plan. The sole major and the great majority
of minors are resolved; the re-review caught one new major (a genuine-fault test
that could not build as described), now fixed in the plan along with two further
test-coverage gaps. The remaining open items are conscious, documented tradeoffs.

**Approved (2026-09-09):** after the fail-closed-on-all-structural-shapes decision
was folded into the plan and the work item (0184) reconciled to match, the author
marked this review APPROVE and moved the plan to `ready`. No open findings remain.
The fail-closed refinement and the work-item edits were applied after this pass and
were not themselves re-run through the lenses.

### Previously Identified Issues

- 🟡 **Architecture + Usability**: Wrong-root refusal reuses the absent-root
  message — **Resolved**. The dedicated `PluginRootNotAnInstallation { path }`
  variant is assessed as the correct taxonomy; the exhaustive `is_refusal` +
  `#[non_exhaustive]` combination makes it fail-closed by construction, and the
  message is verified actionable and self-contained.
- 🔵 **Test Coverage**: Genuine-I/O-fault arm never exercised — **Resolved** (via
  the new-major fix below): a root-that-is-a-file trigger pins the `io_error` arm
  environment-independently at both store and CLI layers.
- 🔵 **Standards + Code Quality**: Doc comment over-wide and restates its arms —
  **Resolved**. Widths recomputed ≤80 at real indent; the comment is now judged
  justified, and its opening clause was trimmed further this pass.
- 🔵 **Usability**: Bare "I/O error" with no pointer to the plugin root —
  **Partially resolved**. The `Io` detail now names the plugin-root expectation; the
  file case still omits the env var by AC8's design (accepted asymmetry).
- 🔵 **Usability**: Compose error contradicts `run_serve`'s own check —
  **Resolved**. Verified: the message drops the launcher-only remediation, and
  `bin/accelerator` exports the derived root into `ACCELERATOR_PLUGIN_ROOT`, so
  "check `ACCELERATOR_PLUGIN_ROOT`" is the correct single lever on both paths.
- 🔵 **Test Coverage**: AC11 never runs `--fail-safe` — **Resolved**. The test now
  runs `config paths --fail-safe`.
- 🔵 **Architecture**: `templates/` layout reconstructed in `compose.rs` —
  **Resolved as accepted**. Recorded as a deliberate deferral; re-review confirms
  reasonable (the gate is centralised; the duplicated path-building is unreachable
  on a wrong root).
- 🔵 **Standards**: Store unit-test snippets drop the root `TempDir` — **Resolved**.
  Snippets bind `let root = tempdir()?;` first.
- 🔵 **Correctness**: TOCTOU in the double probe — **Resolved**. Reasoning assessed
  sound; a deliberate-decision note was added.
- 🔵 **Code Quality**: Path-building duplication — **Resolved**. The tier now builds
  via `plugin_template_path`; one choke for all four commands.
- 🔵 **Test Coverage**: Fast override-hoisting test / AC9 observable red —
  **Resolved**. Both added; this pass made the override test seed its fixture.
- 🔵 **Architecture / Standards**: Helper generality and naming — **Resolved as
  accepted**. Both recorded as deliberate decisions; standards confirms the name is
  not a deviation given the documented refusal.

### New Issues Introduced

- 🟡 **Test Coverage** (major, medium): The `chmod 0o000` genuine-fault test rested
  on a root-guard precedent that does not exist (no root-detection facility in the
  launcher test crate; the existing `chmod` test has no guard) — **Fixed this
  pass**. Replaced with an environment-independent root-is-a-file trigger
  (`metadata` fails `ENOTDIR`, not `NotFound`) at the store and CLI layers.
- 🔵 **Test Coverage** (minor, medium): The genuine-not-found invariant was pinned
  only for the `resolve_template` tier, not the `plugin_default` (diff/reset) site —
  **Fixed this pass**. Added a `plugin_default` valid-root-missing-name → `Ok(None)`
  store test.
- 🔵 **Test Coverage** (suggestion): The override crux store test did not seed the
  override file — **Fixed this pass**. The bullet now seeds
  `.accelerator/templates/<name>.md`.
- 🔵 **Standards / Code Quality** (suggestions): `|error|` vs the dominant `|e|`
  closure idiom, and a trimmable doc-comment opening — **Fixed this pass**.
- 🔵 **Correctness** (suggestion, low): The eject handlers' `available_or_none()`
  swallow is safe only because a `?` on the same helper dominates every caller —
  **Fixed this pass**. Recorded as an explicit invariant in the plan.
- 🔵 **Code Quality** (minor, medium): The structural "templates is a file" fault
  renders under the "I/O error" label — **Accepted**. Consistent with the
  established `Io` reuse (UTF-8 decode, cross-filesystem); the detail already names
  the expectation. Usability raised the same wording nuance.
- 🔵 **Architecture** (minor, medium): The packaging invariant is now encoded in the
  error taxonomy, and the external-installer leg is unverified — **Accepted**.
  Documented in the helper doc comment; a change to the distribution model is the
  trigger to revisit.
- 🔵 **Architecture** (minor, medium) — **Resolved by product decision**: the
  file-not-directory case originally mapped to `Io` (degradable under
  `--fail-safe`). The user chose to fail closed on all structural shapes, so the
  plan now refuses on `templates`-is-a-file (`Ok(!is_dir)`) and root-is-a-file
  (`NotADirectory`) as well as missing `templates/`, reserving `Io` for a genuinely
  unreadable fault (permission, filesystem loop) — tested via a self-referential
  `templates` symlink (`FilesystemLoop`), no root guard. This also resolves the
  "I/O error label on a structural fault" (code-quality) and the "bare I/O error"
  (usability) findings, since those shapes now refuse and name the root. It refines
  the work item's **AC8**, flagged in the plan as a work-item follow-up.

### Assessment

The plan is in strong shape and ready for implementation. The dedicated-variant
decision resolved the headline finding cleanly, and correctness verified — against
live code — that it compiles, forces its own classification, and preserves the
override ordering. The re-review's value was catching a flawed genuine-fault test
before implementation; that and two adjacent coverage gaps are now fixed. The one
item flagged for a product decision — the file-not-directory case degrading under
`--fail-safe` — was decided in favour of failing closed on all structural shapes,
so the plan now refuses on every definitely-wrong root and reserves `Io` for
genuinely unreadable faults. The sole outstanding follow-up is a work-item edit:
AC8 must be revised to match (its file trigger is now a refusal, and criteria added
for the two extra structural shapes).

⚠️ The fixes applied in this pass were not themselves re-run through an agent, so
they are unverified by a further review.

---
*Re-review generated by /accelerator:review-plan*
