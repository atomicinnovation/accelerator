---
type: plan-review
id: "2026-08-02-0187-generalise-sub-binary-registration-surface-review-1"
title: "Plan Review: Generalise the Sub-Binary Registration Surface"
date: "2026-08-03T07:05:20+00:00"
author: "Toby Clemson"
producer: review-plan
status: complete
parent: "plan:2026-08-02-0187-generalise-sub-binary-registration-surface"
target: "plan:2026-08-02-0187-generalise-sub-binary-registration-surface"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [correctness, test-coverage, architecture, code-quality, safety, security, documentation, standards]
review_number: 1
review_pass: 4
tags: [build-system, distribution, rust]
last_updated: "2026-08-03T12:40:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Generalise the Sub-Binary Registration Surface

**Verdict:** APPROVE

This is a strong, unusually well-grounded plan: it reproduced three
specification defects empirically before designing around them (the broken
`covered_by` probe, the hard circular import via `tasks/lint/__init__.py` →
`vendor_shims` → `tasks.build`, three SLSA blocks rather than one), it treats
anti-vacuity as a first-class property, and it enumerates its deviations from
the work item rather than leaving validation to discover them as drift. The
findings below are concentrated in two places: the guard's permission
mechanism, which is wrong in both directions and can be silenced entirely
through the new exemption set; and Phase 3, which changes a helper's arity in a
way that breaks the very regression pin the phase nominates as its evidence.
Phase 5's checklist — the deliverable four sibling work items already link to —
carries several factual errors and, as drafted, fails the mechanical predicate
Phase 5 itself specifies.

### Cross-Cutting Themes

- **The permission mechanism is wrong in both directions** (flagged by:
  correctness, security, safety) — the sentinel probe
  `<launcher> <token> zz-subcommand-probe-zz` rejects rules scoped *tighter*
  than the token and accepts rules whose wildcard sits *inside* the token
  segment. Verified against the tree: `Bash(…/accelerator visualiser start)`
  and `Bash(…/accelerator visualiser --owner-pid *)` both return `False`
  (a correctly-scoped rule reported as unbound, failing the release), while
  `Bash(…/accelerator v*)` and `Bash(…/accelerator [a-y]*)` both return `True`
  (an over-broad rule certified as correctly scoped). The visualiser's real
  rule happens to be `visualiser *`, which is why the drafting-time check
  passed.

- **The escape hatch is unbounded** (flagged by: safety, security,
  test-coverage) — `SKILL_EXEMPT_SUBBINARIES` is consumed only as a filter.
  Setting it equal to `DISPATCHED_SUBBINARIES` leaves the collection non-empty,
  so the anti-vacuity check passes and the guard checks nothing at all. The
  same edit silences the one test covering the guard's only real binding. The
  work item forbids this route in prose; nothing enforces it.

- **Phase 3 cannot land green as written** (flagged by: correctness,
  test-coverage, architecture, safety, standards — five lenses) —
  `debug_archive_path` gains a `token` parameter, but
  `tests/integration/tasks/test_github.py:253-258` patches it with a
  **one-argument** lambda. Every test through `_setup_release` raises
  `TypeError`, including the `assert len(uploads) == 22` pin the phase's own
  Success Criteria names as its regression evidence. That file is not in the
  phase's change list.

- **Seams are added where behaviour is unchanged and withheld where it changed**
  (flagged by: test-coverage, code-quality, architecture, safety) — Phase 4
  parameterises three builders that already iterated a token collection, while
  the one loop this plan actually rewrites (the debug-archive loop in
  `create_debug_archives` and `_release_uploads`) gets no seam. Phase 3
  nonetheless promises to "test `create_debug_archives` against an injected
  two-token registry", which its own code sketch makes impossible.

- **The checklist's enforcement is pinned in the wrong direction** (flagged by:
  architecture, documentation) — the literal-string test reads only
  `tasks/README.md`. Renaming `DISPATCHED_SUBBINARIES` in code leaves the README
  stale and the test green; *correcting* the README to the new name makes the
  test fail. The guard resists the maintenance it exists to force.

- **A new registration point is added without being registered** (flagged by:
  safety, correctness) — `DEBUG_ARCHIVE_DIRS` now decides which sub-binaries
  ship symbolication archives and into which committed tree, and it is the
  trigger for checklist point 9's provenance obligation. It appears in neither
  the eleven points nor the pinned literal-string set.

- **Cross-language and cross-artefact facts are duplicated with prose-only
  lockstep** (flagged by: security, architecture, code-quality, test-coverage) —
  `BUILTIN_SUBCOMMANDS` is a third hand-maintained copy of the launcher's
  built-in set, enforced only by checklist point 10, in a plan that mechanises
  every other obligation it documents. The house idiom for exactly this
  (`tests/unit/tasks/test_manifest_contract.py` regex-extracting Rust
  constants) is already in the repo.

### Tradeoff Analysis

- **Literal work-item conformance vs sound layering**: the plan declines to
  extract the pure parsing half of `tasks/lint/skill_permissions.py` because
  "the work item requires importing from `tasks.lint.skill_permissions`". The
  architecture lens notes this is a requirements-conformance argument, not an
  architectural one, and the plan already overrides the work item in seven
  other places. **Recommendation**: the cheaper resolution is to move the guard
  to `tasks/dispatch.py` (per `tasks/manifest.py`'s task-less precedent), which
  keeps `tasks/shared/` a true leaf without a new phase. The full parsing
  extraction is a legitimate deferral if recorded as an accepted tradeoff with
  a trigger stated in terms of 0169–0173 rather than a hypothetical third
  consumer.

- **False positives vs false negatives in the probe**: the two suggested
  remedies fix opposite halves. Probing with the *actual invoked command* found
  in the SKILL.md (safety lens) fixes the false-negative and makes the guard
  agree with `skill_permissions` by construction, but still accepts `v*`. A
  structural check on the rule's token segment (security lens) rejects `v*` but
  needs its own parsing. **Recommendation**: take both — they compose, and the
  combined check is still shorter than the sentinel design it replaces.

- **Release-gate strictness vs release-path reliability**: making the guard
  strictly generic means any skill edit anywhere can now redden a release, and
  it fires *after* four cross-compiles and a use of the signing secret. The
  architecture and safety lenses both want it wired into `lint:check` as well.
  **Recommendation**: wire it into `lint:check` and keep the argument-free
  release call (an acceptance criterion pins the latter); this is additive and
  costs one mise task.

- **Mechanising prose vs test maintenance**: the code-quality lens argues the
  closed verb-set predicate is high-maintenance and low-signal (a rewording
  fails the build over a verb whitelist), while correctness and documentation
  argue the current draft fails it. **Recommendation**: keep the item count and
  the literal registration-point strings, drop the verb-set half, and redraft
  the eleven items in the imperative anyway — the phrasing obligation is better
  enforced at review than by a whitelist.

### Findings

#### Critical

- 🔴 **Safety + Security**: `SKILL_EXEMPT_SUBBINARIES` can restore full vacuity
  **Location**: Phase 2 §1 (the exemption registry) and §2 (the entry point)
  `exempt` is consumed only as a filter, with no assertion that
  `exempt ⊆ tokens`, that a non-exempt token remains, or that the stated
  "hook-only consumer" bar holds. `SKILL_EXEMPT_SUBBINARIES =
  DISPATCHED_SUBBINARIES` leaves `tokens` non-empty, so the anti-vacuity check
  passes while `unbound` and `unregistered` are both empty — the guard checks
  nothing. Exempt tokens also skip condition 2 entirely, so a skill may invoke
  one under an ancestor glob unchallenged. The same gap hides token *loss*: an
  exempt token deleted from `DISPATCHED_SUBBINARIES` produces no complaint, no
  manifest entry, no signature and no upload. 0169's hook-invoked `vcs` is the
  archetypal exemption case, so this hatch will be used in the very next change.

#### Major

- 🟡 **Correctness + Safety**: the sentinel probe rejects rules scoped tighter
  than the token
  **Location**: Phase 2 §2 (`_authorises`, `_PROBE_SENTINEL`)
  Verified against the tree: `Bash(…/accelerator visualiser start)` → `False`,
  `Bash(…/accelerator visualiser --owner-pid *)` → `False`. Both are
  contract-conformant rules that `skill_permissions` accepts and whose
  violation message at `:184-185` explicitly invites ("name 'config' (or the
  specific subcommand)"). A sibling story narrowing its rule gets a red release
  telling it to add the rule it already has; the only workaround is to broaden
  the grant.

- 🟡 **Security**: condition 2 accepts over-broad rules whose wildcard sits in
  the token segment
  **Location**: Phase 2 §2 (`_authorises`)
  Verified: `Bash(…/accelerator v*)` and `Bash(…/accelerator [a-y]*)` both
  satisfy the probe and both fail to cover the `zz-`-prefixed `BARE_LAUNCHER`,
  so the guard certifies them as correctly scoped. `v*` pre-authorises
  `visualiser`, `vcs` (which 0169 ships), `version` and every future
  `v`-prefixed token; `[a-y]*` authorises everything not starting with `z`. The
  sentinel's leading `z` is the only thing standing between the two checks.

- 🟡 **Security + Correctness**: bare `Bash` breaks the permission model in both
  directions
  **Location**: Decisions locked during planning (bare-`Bash` row); Phase 5 §1
  point 7
  A skill declaring both `- Bash` and a scoped rule yields a rule satisfying
  both conditions, so the guard reports the token bound and correctly scoped —
  while bare `Bash` grants unrestricted Bash, strictly broader than the
  ancestor glob condition 2 rejects. Conversely a bare-`Bash`-only skill counts
  as unbound (`frontmatter_bash_rules` returns `[]`), diverging from
  `skill_permissions`, which treats bare `Bash` as authorising everything
  (`:167`). Eleven skills declare bare `Bash` today — `skills/config/migrate`
  plus all ten Linear/Jira integration skills, which is exactly where 0170's
  `work-item` token would be consumed.

- 🟡 **Correctness + Test Coverage + Architecture + Safety + Standards**:
  Phase 3's arity change breaks the pin the phase nominates as its evidence
  **Location**: Phase 3 §3 (the upload) and Phase 3 Success Criteria
  `tests/integration/tasks/test_github.py:253-258` patches
  `debug_archive_path` with `side_effect=lambda p: …` — one parameter. The new
  two-argument call site raises `TypeError` for every test through
  `_setup_release`, including `assert len(uploads) == 22` (`:326`), the
  missing-asset test and both preserve-draft tests. `test_github.py` is not in
  Phase 3's change list, so the phase fails the "independently mergeable and
  landing green" constraint, and the failure invites a `lambda *a` patch rather
  than a deliberate fixture update.

- 🟡 **Test Coverage + Code Quality + Architecture + Safety**: the one loop that
  actually changes gets no seam
  **Location**: Phase 3 §2 and §5; Phase 4
  `create_debug_archives` stays an `@task` reading the module-global
  `DEBUG_ARCHIVE_DIRS`, and `debug_archive_path`'s `dirs` default binds at
  def-time — so the promised "injected two-token registry" test can only work
  by `mocker.patch.dict` on the shared dict, the module-state patching Phase 4
  exists to retire. `_release_uploads` likewise threads its new `tokens`
  parameter only to `_subbinary_uploads`, leaving the debug loop on the global.
  The generalisation this plan delivers is therefore discharged by inspection.

- 🟡 **Safety + Correctness**: `DEBUG_ARCHIVE_DIRS` is a new registration point
  the checklist never names
  **Location**: Phase 3 §1; Phase 5 §1 and §2
  The registry decides whether a sub-binary ships symbolication archives and
  which committed tree they land in — and it is the condition that triggers
  checklist point 9's sibling `subject-path` obligation. It appears in neither
  the eleven points nor the fifteen pinned literals. A future author following
  the checklist ships without archives (silent — the loop skips the token) and
  with no pointer to the provenance consequence. Nothing relates the registry
  to `DISPATCHED_SUBBINARIES` either: a key in one and not the other surfaces
  as a bare `FileNotFoundError` from `tarfile.add`.

- 🟡 **Safety + Security + Test Coverage + Correctness**: the SLSA test asserts
  symmetry, not coverage — and near-duplicates a committed test
  **Location**: Phase 3 §4 (the SLSA guard)
  `> 1` does not pin the count, so deleting the attest step from the `release`
  job leaves two symmetric blocks and stays green — the stable track then ships
  with no provenance. Symmetry is not coverage: all three blocks can agree on a
  set that omits a newly-staged tree, which is precisely what point 9 leaves to
  diligence. Verified: `tests/unit/tasks/test_workflows.py:147-159`
  (`test_attest_globs_include_the_launcher_binaries`) already iterates every
  attest step and asserts both `dist/release/accelerator-*` and
  `accelerator-visualiser-*`. The plan does not mention it, and its hardcoded
  `visualiser` literal is exactly what the "no literal `visualiser` on the
  release path" goal would tempt an implementer to relax. The genuinely new
  power is the identical-set assertion.

- 🟡 **Correctness + Documentation**: the drafted checklist items fail Phase 5's
  own predicate
  **Location**: Phase 5 §1 vs §2
  §2 defines the predicate as "first line begins with a capitalised verb from a
  small closed set, **or** contains `No action when`". Applied to §1's own
  drafted text, items 1, 3, 4, 6, 7 and 10 satisfy neither branch (item 10 is a
  gerund; items 1 and 4 are bare noun phrases), and item 8 passes only if the
  verb set is stretched. The phase cannot land green without either weakening
  the test to nothing or rewriting six items during implementation — the kind of
  unplanned rewording that drops one of the fifteen pinned literals.

- 🟡 **Architecture + Documentation**: the README test pins docs against docs,
  not docs against code
  **Location**: Phase 5 §2; Desired End State
  The test reads only `tasks/README.md`. Renaming `DISPATCHED_SUBBINARIES` or
  deleting `_assert_static_elf` leaves the README stale and green; correcting
  the README to match makes it fail. The plan's core justification for choosing
  documentation-as-enforcement rests on a guard that cannot detect the drift it
  exists to catch — the eleven points will age exactly as research §8's
  superseded list did.

- 🟡 **Architecture + Standards**: `tasks/shared/dispatch.py` inverts the
  shared-package layering
  **Location**: Decisions locked during planning (circular-import remedy);
  Phase 2 §2
  All twenty existing `tasks/shared/` modules import only stdlib, third-party
  packages, or other `tasks.shared.*`. This would be the first to reach up into
  a task package — and because importing the submodule executes
  `tasks/lint/__init__.py`, anything importing it transitively pulls in
  `invoke`, all of `tasks.lint`, and `tasks.build`. The stated rationale
  ("because the module declares no invoke tasks") is refuted by
  `tasks/manifest.py`, which declares none and lives at `tasks/`.

- 🟡 **Architecture**: the circular import is relocated, not removed, and
  nothing pins the new invariant
  **Location**: Current State Analysis (verified empirically); Phase 2 §2
  The root cause — `tasks/lint/vendor_shims.py:3` importing `tasks.build`,
  eagerly loaded by `tasks/lint/__init__.py` — is untouched. The remedy works
  only while nothing in `tasks.build`'s closure imports the dispatch module.
  That edit is likely: the guard used to live in `build.py`, every acceptance
  criterion still anchors it there, and `BUILTIN_SUBCOMMANDS` is the kind of
  constant a build task reaches for. The plan adds four source scans but none
  for the import-direction invariant it now depends on.

- 🟡 **Architecture + Safety**: a repo-wide skills scan runs only on the release
  hot path
  **Location**: Desired End State; Phase 2 §3
  The guard is invoked only from `emit_manifest`, after four cross-compiles,
  after `sign_staged_binaries` has used the release secret, and after the
  manifest bytes are on disk — while `lint:skill-permissions:check` already
  walks the same tree on every PR and already rejects the ancestor-glob shape
  (`skill_permissions.py:183-188`), so most of the fifteen-case matrix tests
  states unreachable in a green repo. On failure it leaves a freshly written
  unsigned `manifest.json` beside a stale `manifest.minisig`.

- 🟡 **Architecture + Code Quality**: three injection idioms whose divergence
  fails silently
  **Location**: Decisions locked during planning; Phase 3 §2; Phase 4
  Direct def-time defaults (the guard, `subbinary_signing_targets`,
  `debug_archive_path`), `None` sentinels resolved against the module global
  (`_subbinary_uploads`, `_release_uploads`, `_subbinary_reverifies`), and a
  bare global read with no seam (`create_debug_archives`). The two semantics
  differ observably, and the split exists to preserve the two
  `mocker.patch.object` sites the seam work is meant to retire. The failure mode
  is silent: a future author who "tidies" `github.py` to match `signing.py`
  disables both injections with no test turning red.

- 🟡 **Code Quality**: `_scan`'s `dict[str, bool]` makes keys and values mean
  different things
  **Location**: Phase 2 §2 (`_scan`, `validate_dispatch_coherence`)
  Keys mean "invoked by some skill", values mean "authorised by some skill" —
  and the variable is called `bound` in both readings, so `token in bound`
  naturally misreads as "is bound". The two directions the work item is careful
  to name distinctly collapse into one ambiguous structure at exactly the point
  a third rule would be added. Two sets (`invoked`, `bound`) make each direction
  one line.

- 🟡 **Code Quality + Documentation**: the unbound message conflates three
  causes and recommends the escape hatch
  **Location**: Phase 2 §2 (error composition)
  A token lands in `unbound` because no skill invokes it, because its only
  matching rule is an ancestor glob, or because the invoking skill declares bare
  `Bash` — needing three different fixes. The message names only the first and
  offers `SKILL_EXEMPT_SUBBINARIES` as an alternative remedy, which for the
  ancestor-glob case permanently vacates the check the work item forbids
  vacating. `_scan` holds the offending path and discards it.

- 🟡 **Security + Safety**: upload and re-verify token sets become independently
  parameterised with no invariant tying them
  **Location**: Phase 4 §2 and §3
  Each leaf resolves its own default, `_subbinary_reverifies` gains an
  independent `manifest_path`, and `_release_reverifies` — the intermediate
  `upload_and_verify_release` actually calls — gains no parameter, so the seam
  cannot be threaded consistently. Nothing checks that every uploaded
  `accelerator-<token>-<platform>` has a corresponding `_Reverify`, nor that
  `set(manifest["binaries"]) == set(names)`. Because `()` is not `None`, an
  explicitly empty `tokens` now empties uploads and re-verifications while the
  manifest still advertises them — a signed manifest promising an asset that
  does not exist, which is the one outcome the pipeline cannot recover from.

- 🟡 **Test Coverage**: `_seed` cannot express the prose case
  **Location**: Phase 2 §4 (the `_seed` helper and the matrix)
  `_seed` takes only `rules` and `commands` and wraps every command in
  `` !`…` ``. The "mentions the token only in prose / a backticked reference"
  case has no parameter to express it and degenerates into a duplicate of the
  no-invoking-skill case; the bare-`Bash` case needs a frontmatter shape `_seed`
  cannot emit. The prose case is the regression test for the exact defect this
  task exists to remove — today's guard reports the visualiser bound on the
  strength of `skills/visualisation/visualise/SKILL.md:46`.

- 🟡 **Test Coverage**: the real-repo case can be silenced by an unrelated
  constant
  **Location**: Phase 2 §4 ("Real repo, no arguments | passes")
  It resolves the live `SKILL_EXEMPT_SUBBINARIES`, and nothing asserts that
  constant is empty or excludes `visualiser`. A one-line addition makes the only
  check on the guard's sole production binding pass vacuously with everything
  else green — the route the work item explicitly forbids.

- 🟡 **Test Coverage**: the anti-reimplementation scan has a straightforward
  bypass
  **Location**: Phase 2 §4 (source-scan tests)
  Forbidding `Bash(` and `re.compile` does not stop an inline re-implementation
  using `fnmatch.fnmatchcase` and `str.startswith`/`split` — the actual
  mechanics of `covered_by` and `is_plugin_invocation` — and nothing asserts the
  six imported names are ever *used*. The shadow regex is anchored at column 0,
  so a nested `def covered_by` evades it. The work item states this is the sole
  verification of the reuse requirement.

- 🟡 **Test Coverage**: the `emit_manifest` spy test needs signing
  infrastructure the plan does not schedule
  **Location**: Phase 2 §4 (release-call-site tests); Testing Strategy
  `emit_manifest` also runs `validate_version_coherence` against the real root
  and then `sign_file`, which shells out to `minisign`. The house precedent
  (`tests/unit/tasks/test_manifest.py`) handles this by *skipping* when the tool
  is absent — so copying it makes the criterion's guard silently non-executing
  locally, and a skipped guard is indistinguishable from a passing one.

- 🟡 **Security + Architecture + Code Quality + Test Coverage**:
  `BUILTIN_SUBCOMMANDS` is a third unpinned copy of a Rust-side fact
  **Location**: Phase 2 §2; Phase 5 §1 point 10
  Three hand-maintained copies now exist: clap's `Command` enum
  (`cli/launcher/src/launch/inbound/cli.rs:17-29`, which lists only `Version`
  and `Config`), `is_root_help`'s literal (`main.rs:107`), and the Python set.
  ADR-0054 names only `version` and `config`. Point 10 documents *additions*
  only; the removal direction is the dangerous one — a name left in the Python
  set after leaving the launcher silently exempts every skill invocation of a
  subcommand that no longer exists. `tests/unit/tasks/test_manifest_contract.py`
  already regex-extracts Rust constants for exactly this purpose.

- 🟡 **Documentation**: several checklist points state things the code
  contradicts
  **Location**: Phase 5 §1 (points 5, 6, 8, 11)
  Point 5's exemption ("No action when nothing is staged under a committed
  `bin/` tree") misattributes the mechanism — verified: `.gitignore:44`'s
  `bin/visualiser-*` sits in the launcher-cache block, whose own comment says
  "and the sub-binary cache"; the launcher caches every fetched sub-binary as
  `bin/<token>-<version>-<sha256>` in the plugin root, so the entry is needed
  unconditionally. Point 6's emphatic "**both** co-readers in the same change"
  is false for *adding* a token: `test_manifest_contract.py` iterates
  `binaries.values()` generically and the Rust `include_str!` test reads the
  existing `visualiser` entry. Point 8 omits the two `tasks/release.py` call
  sites (`:91`, `:122`) and the `mise.toml` leaf without which a new staging
  task is never invoked in CI. Point 11's `docs/internals.md` env-var table
  holds only launcher-wide, already-generic variables, while the obligation an
  author actually has — a per-sub-binary doc page and its index entry — is
  unstated.

- 🟡 **Architecture + Standards + Code Quality + Test Coverage**: test homes
  break the mirror and are internally inconsistent
  **Location**: Decisions locked during planning (test homes); Phase 2 §4;
  Phase 3 §5; Phase 5 §2
  The repo mirrors `tasks/shared/*.py` into `tests/unit/tasks/shared/test_*.py`
  strictly. Phase 2 puts `tasks/shared/dispatch.py`'s tests outside that mirror
  while Phase 3 puts `paths.py` tests inside it — opposite rules in adjacent
  phases. Phase 3 names a destination directory but no file (none exists), and
  `paths.py` coverage then splits from `TestCliPathHelpers` in `test_build.py`,
  which the decision table says it is avoiding. Phase 5 then loads
  `test_dispatch.py` with README prose assertions, and homes the `emit_manifest`
  spy away from `test_manifest.py`'s existing `emit_manifest` tests.

- 🟡 **Safety**: a debug archive staged outside a `bin/` directory is swept into
  the release commit
  **Location**: Phase 3 §1; Phase 5 §1 point 5
  `DEBUG_ARCHIVE_DIRS` accepts any directory. The release path runs `git add .`
  then commits, tags and pushes, and `_assert_no_leaked_artifacts` matches only
  `.sec`, `dist/release/` and `dist/`. Protection today is `.gitignore:54`'s
  `bin/*.debug.tar.gz`, which matches only directories literally named `bin`. A
  token mapped to, say, `skills/vcs/artifacts` commits multi-megabyte binaries
  onto `main`, into the tag, and into the shipped plugin package.

- 🟡 **Architecture**: deferring the parsing extraction preserves the cause of
  the coupling
  **Location**: What We're NOT Doing
  `skill_permissions.py` mixes a ~60-line pure parsing core (depending only on
  `re` and `fnmatch`) with an imperative `@task` shell — which is why the
  parsing cannot be consumed without dragging in `invoke`, `tasks.lint` and
  `tasks.build`. Extracting the pure half to a `tasks/shared/` leaf removes the
  inverted edge entirely and turns Phase 1's rename into a move. See the
  tradeoff analysis: the cheaper partial fix is relocating the guard.

#### Minor

- 🔵 **Correctness**: the SLSA test cannot detect the fourth-block case it
  claims to catch
  **Location**: Phase 3 §4
  A fourth `attest-build-provenance` block carrying the same `subject-path` set
  satisfies both `> 1` and set-identity, so it passes silently. Only the
  asymmetric-edit case is caught, yet the plan records the count as pinned.

- 🔵 **Correctness**: two manual-verification greps are unsatisfiable as stated
  **Location**: Phase 3 Manual Verification; Manual Testing Steps
  `rg 'visualiser' tasks/github.py` still matches `:150`
  (`# ── unified launcher + manifest + visualiser publish ──`), which the plan
  never lists. `rg "visualiser" tasks/shared/paths.py` matches `SERVER` (`:14`),
  `FRONTEND` (`:16`) and `subbinary_asset_path`'s docstring (`:67-68`) in
  addition to the two registries — roughly eight matches, not four.

- 🔵 **Correctness + Test Coverage + Standards**: three mechanical follow-through
  edits are omitted, each reddening a phase
  **Location**: Phase 2 §3/§4; Phase 3 §1/§5
  (a) Deleting `TestValidateDispatchCoherence` leaves `DispatchCoherenceError`
  unused at `test_build.py:18` — verified: its only uses are `:136` and `:142`,
  both inside the deleted class; `F401` is not in the `tests/**` per-file
  ignores, so `mise run check` reddens. (b) `Mapping` is not imported in
  `tasks/shared/paths.py` (it imports only `tomllib`, `Path`, `Any`). (c) The
  new `tests/unit/tasks/shared/` file is unnamed.

- 🔵 **Correctness**: the error-ordering rationale is misattributed and one
  criterion is unpinned
  **Location**: Phase 2 §2
  "Sorting `unbound` is what satisfies the 'names the second token' criterion"
  is wrong — the filter does; `unbound` has one element in that fixture, and the
  sort discards registry order. Separately, "fires because unbound, **not**
  because invocation→registration caught it" is guaranteed only by fixture
  construction, never asserted.

- 🔵 **Correctness + Documentation**: the `launcher` reservation is justified by
  a collision that does not occur
  **Location**: Phase 5 §1 (reserved tokens)
  `verify` genuinely collides (`cli_binary_path("accelerator-verify", p)` and
  `subbinary_asset_path("verify", p)` both yield `accelerator-verify-<platform>`)
  but the launcher stages as `accelerator-<platform>`, so
  `accelerator-launcher-*` does not collide in `dist/release/`. The real hazards
  are `_default_subbinary_manifest` resolving onto the existing `cli/launcher/`
  crate, and the bootstrap's `bin/accelerator-launcher-*` cache name. A
  normative claim an author can disprove invites treating the rest as folklore.

- 🔵 **Safety**: flag-shaped first arguments are treated as unregistered tokens
  **Location**: Phase 2 §2 (`_invoked_token`, `BUILTIN_SUBCOMMANDS`)
  `!`…/bin/accelerator --version`` tokenises to `--version`, failing the release
  with "needs an entry in DISPATCHED_SUBBINARIES" — nonsense for a flag, and
  wrong to follow. `is_root_help` already treats bare flags as root help.

- 🔵 **Code Quality**: the `is_plugin_invocation` condition exists only to
  satisfy an import assertion
  **Location**: Phase 2 §2 (`_invoked_token`)
  `startswith(LAUNCHER)` already implies `startswith(PLUGIN_PREFIX)`, so the
  first clause cannot fire. Its only function is satisfying the six-name import
  scan — and deleting it breaks a test whose message talks about imports.

- 🔵 **Code Quality**: the comment budget is spent on the wrong facts
  **Location**: Phase 1 §1; Phase 2 §2
  The two facts a maintainer cannot recover from the code — why the probe needs
  a sentinel argument, and why `tail.startswith(" ")` is load-bearing against
  `bin/accelerator-verify-*` — appear only in the plan. Meanwhile Phase 1's
  snippet *replaces* the existing semantic comment on `BARE_LAUNCHER` ("A
  launcher command naming no subcommand — any rule matching it is too broad")
  with a visibility justification. In a repo with very low comment tolerance,
  these two are exactly the cases that earn one.

- 🔵 **Code Quality**: `debug_archive_path` re-looks-up a registry both callers
  already iterate
  **Location**: Phase 3 §1
  `create_debug_archives` holds `directory` from `.items()` and discards it so
  the helper can look it up again. The mapping parameter also diverges from
  `cli_binary_path`/`subbinary_asset_path`/`vendored_shim_path`, which all take
  a directory, and the planned `KeyError` test pins behaviour neither production
  call site can reach.

- 🔵 **Code Quality**: `LAUNCHER` sits beside the imported `BARE_LAUNCHER` and
  invites misreading
  **Location**: Phase 2 §2 (module constants)
  One is a path prefix, the other a complete probe command ending in
  `zz-external-subcommand-zz`; two lines apart in `_authorises`, the natural
  reading of `BARE_LAUNCHER` is "`LAUNCHER` with no arguments", which it is not.
  `LAUNCHER` also has no external consumer.

- 🔵 **Code Quality**: the closed verb-set predicate is high-maintenance and
  low-signal
  **Location**: Phase 5 §2
  It couples a hand-maintained verb whitelist in test code to English prose. An
  innocuous rewording fails the build with a message about a verb list, while
  catching no defect the item count and the fifteen literals do not.

- 🔵 **Code Quality + Standards**: two snippets exceed 80 columns
  **Location**: Phase 2 §4 (`_seed`); Phase 3 §2
  `with tarfile.open(debug_archive_path(token, platform), "w:gz") as tar:` is 82
  columns at that indent, and `_seed`'s `write_text` line is 81. `ruff format`
  will split the `with` header across three lines — worse than today's shape,
  which names `archive_path` first.

- 🔵 **Test Coverage**: two mutations survive the guard matrix
  **Location**: Phase 2 §4
  No case has the skill invoke the target token while carrying a scoped rule for
  a *different* subcommand, so dropping condition 1 (or omitting the token from
  the probe) is undetected — the ancestor-glob case exercises condition 2 only.
  And of three `BUILTIN_SUBCOMMANDS` members only `config` and `help` are
  tested; `version` is unpinned.

- 🔵 **Test Coverage**: the default-count assertions are brittle against the
  five consumers this task unblocks
  **Location**: Phase 4 §4
  "exactly 4 / 8 / 4" plus `assert len(uploads) == 22` means each of 0169–0173
  hand-edits four literal counts across two test files — each edit a chance to
  bump a number rather than ask why it changed. Deriving from
  `len(DISPATCHED_SUBBINARIES) * len(TARGETS)` and keeping one literal pin of
  the registry itself fixes this.

- 🔵 **Test Coverage**: point 10's lockstep is enforced by prose alone
  **Location**: Phase 5 §1 point 10
  The plan mechanises every other obligation it documents. The removal direction
  fails silently (a stale entry exempts invocations of a subcommand that no
  longer exists), which is the failure class the anti-vacuity design elsewhere
  works hard to close.

- 🔵 **Safety**: the gate runs after signing and after the manifest is written
  **Location**: Phase 2 §3 (the `tasks/manifest.py:138` call site)
  The guard reads nothing from the manifest — it is a pure static scan — yet
  runs after `atomic_write_text` and after `sign_staged_binaries` has used the
  release secret on eight binaries. On failure it leaves an unsigned
  `manifest.json` beside a stale `manifest.minisig`. Keeping the argument-free
  call but moving it to the first statement costs nothing.

- 🔵 **Security**: the release gate now depends on `covered_by`'s unverified
  model of Claude Code's matcher
  **Location**: Phase 1; Phase 2 reuse of `covered_by`
  `covered_by` delegates to `fnmatch`, so it honours `?`, `[seq]` and `[!seq]`
  — which is why `[a-y]*` passes above — and silently widens a rule not ending
  in `*`. Nothing in the repo tests that model against the real matcher (the
  `tests/integration/skill-invocation` suite executes commands; it does not
  exercise permissions). Phase 1 promotes this to a public contract and Phase 2
  makes a release gate its second consumer.

- 🔵 **Security + Documentation**: the guard's scope is narrower than point 7
  implies
  **Location**: Phase 2 §2; Phase 5 §1 point 7
  `_scan` walks only `skills/**/SKILL.md` and requires the command to *begin*
  with `${CLAUDE_PLUGIN_ROOT}/`. Invocations from `hooks/` and `scripts/`
  (`hooks/config-detect.sh` already invokes `bin/accelerator`), commands led by
  `env FOO=1 …`, and model-driven Bash are all invisible. Point 7 presents "the
  skill binding" without stating this, which is the same reasoning gap that
  makes the exemption's "consumer is a hook" claim unverifiable.

- 🔵 **Documentation**: the `validate_dispatch_coherence` docstring's
  "first/second" is unresolvable
  **Location**: Phase 2 §2
  Within the docstring the two enumerated things are the invocation and the
  rule; "an ancestor glob satisfies the first condition and fails the second"
  silently refers to the work item's *permission* conditions, which the
  docstring never enumerates. This is the module's only prose explanation of its
  subtlest rule.

- 🔵 **Documentation**: the insertion point is ambiguous and reparents three
  subsections
  **Location**: Phase 5 §1
  "After `## Conventions (learn once)` (`:50`)" read literally inserts the new
  `##` between that heading and its own body (`:51-147`), reparenting
  Executable-bit invariant (`:68`), Rust nightly lane (`:101`) and Contributor
  environment variables (`:137`) under it.

- 🔵 **Documentation + Standards**: Phase 5 claims a markdown check that does
  not exist
  **Location**: Phase 5 Success Criteria
  Verified: `format:build-system:check` runs `ruff format --check` — its mise
  description is "Check Python formatting with ruff". The repo has no markdown
  formatter or linter, so the new section's 80-column wrapping is unenforced.

- 🔵 **Standards**: two criteria bypass sanctioned `mise run` entry points
  **Location**: Phase 1 and Phase 3 Success Criteria
  `uv run invoke lint.skill-permissions.check` and
  `uv run invoke lint.workflows.actionlint` have first-class equivalents
  (`mise run lint:skill-permissions:check`, `mise run lint:workflows:check`),
  which also carry the `deps:install:python` dependency the bare call skips.
  Phase 3 changes no workflow YAML, so the actionlint criterion verifies an
  unmodified file.

- 🔵 **Standards**: registries split across two modules with three collection
  types
  **Location**: Phase 2 §2; Phase 5 §1 point 10
  `DISPATCHED_SUBBINARIES` (tuple), `SKILL_EXEMPT_SUBBINARIES` (tuple) and
  `DEBUG_ARCHIVE_DIRS` (mutable dict) in `paths.py`; `BUILTIN_SUBCOMMANDS`
  (frozenset) in the guard. Point 10 then sends an author to a different module
  for one of four related registries.

- 🔵 **Standards**: public/private constant discipline is applied inconsistently
  **Location**: Phase 1 §1; Phase 2 §2
  `skill_permissions.py` makes constants private by default and marks a public
  one with `# Public: …` stating why. The new module declares `LAUNCHER` and
  `BUILTIN_SUBCOMMANDS` public with no justification while `_PROBE_SENTINEL`
  beside them is private, and `LAUNCHER` has no external consumer.

- 🔵 **Standards**: two structurally identical seams, one public and one private
  **Location**: Phase 4 §1 and §2
  `subbinary_signing_targets` is public, `_subbinary_uploads` private, with the
  same role and the same consumers (one internal caller plus a test). Both
  modules' existing style is underscore-private, and tests here import private
  helpers routinely (`SLF001` is per-file-ignored).

- 🔵 **Standards**: new helper names diverge from close siblings
  **Location**: Phase 2 §2 and §4
  `_seed` is a near-duplicate of `_skill` in
  `tests/unit/tasks/test_skill_permissions.py:22-25` under a different name and
  without that file's annotations. `_scan` is far terser than the house naming
  (`_pinned_member_versions`, `_command_violations`, `_frontmatter_lines`) and
  says nothing about its return.

- 🔵 **Documentation**: checklist point 2 omits the one worked example that
  exists
  **Location**: Phase 5 §1 point 2
  The `_SUBBINARY_MANIFESTS` rule is stated abstractly without
  `"visualiser": cli/visualiser/server/Cargo.toml`. 0168's Dependencies bullet
  expects exactly that worked example, and 0169 is the same case — `cli/vcs/`
  already holds a `vcs` domain crate.

- 🔵 **Documentation**: checklist point 3 understates the inherited-key set
  **Location**: Phase 5 §1 point 3
  Existing members carry `edition/rust-version/license/publish.workspace = true`
  and `[lints] workspace = true`, the latter applying `warnings = "deny"` and
  the clippy opt-ins. A crate created from point 3 alone silently opts out of
  workspace lint enforcement. `_pinned_member_versions` also only reports a
  *mismatch*, so a hardcoded current version passes today and breaks at the next
  bump — which is the real reason to inherit.

#### Suggestions

- 🔵 **Architecture**: home Phase 5's docs test in its own file
  **Location**: Phase 5 §2
  Putting it in `test_dispatch.py` makes Phase 5 non-mergeable without Phase 2
  and mislocates a whole-surface documentation guard inside one module's
  coverage — only points 1, 7 and 10 concern dispatch at all.

- 🔵 **Standards**: injected collections annotated `tuple[str, ...]` where the
  cited precedent uses `Iterable[str]`
  **Location**: Phase 2 §2; Phase 4 §1
  `collect_entries` — named as "the house precedent to copy" — uses
  `Iterable[str]`. The identity pin works identically under either.

- 🔵 **Correctness**: give `DEBUG_ARCHIVE_DIRS` a coherence link
  **Location**: Phase 3 §1; Desired End State
  Extend the SLSA test to assert every registry value has a matching
  `subject-path` glob in all three blocks, and assert
  `set(DEBUG_ARCHIVE_DIRS) <= set(DISPATCHED_SUBBINARIES)`.

- 🔵 **Code Quality**: pin `BUILTIN_SUBCOMMANDS` against the Rust source, or at
  minimum name the file it tracks at the constant
  **Location**: Phase 2 §2
  So the obligation travels with the code rather than only with the README.

### Strengths

- ✅ Three specification defects were reproduced empirically at a named revision
  before any design was committed — the broken `covered_by` probe, the hard
  circular import via `tasks/lint/__init__.py` → `vendor_shims` →
  `tasks.build`, and three SLSA blocks rather than one. All three reproduce.
- ✅ The `None`-sentinel decision for `tasks/github.py` is correct and
  non-obvious: `mocker.patch.object(gh, "DISPATCHED_SUBBINARIES", ("foo",))`
  works only because the name resolves from module globals at call time, and no
  test patches `signing.DISPATCHED_SUBBINARIES` — so the plan drew the direct
  default / sentinel line in exactly the right place.
- ✅ `_authorises` applies both permission conditions to a *single* rule, which
  is the semantics the work item demands; the naive two-`any()` formulation
  would wrongly reject a skill carrying both a scoped rule and an unrelated
  broad one.
- ✅ Anti-vacuity is treated as a first-class design property, and the one
  behavioural reversal (`test_both_absent_is_coherent`) is named in Key
  Discoveries, in the Phase 2 deletion and again in Migration Notes rather than
  quietly deleted.
- ✅ The plan records its own limitations honestly — CPython interning `()` makes
  the `SKILL_EXEMPT_SUBBINARIES` identity assertion degenerate, and "an
  imperative action line" is not mechanically decidable — with compensating
  measures rather than overclaiming.
- ✅ The built-in set was corrected to include `help` after reading
  `is_root_help` rather than trusting the work item's two-name set; `help` is a
  clap-generated built-in that takes precedence over the external subcommand.
- ✅ Phase 3 preserves the archive filename and the exact upload iteration
  order, so the 22-upload census keeps its meaning as a check that no asset was
  dropped.
- ✅ Tokenisation is fail-closed in the interesting cases: a dynamically
  computed token or a flag-first invocation resolves to a non-registered string
  and fails rather than escaping the check; `has_metacharacter` already bans
  `$(` and backticks, so the `!`-site token space stays statically decidable.
- ✅ The refusal to collapse registration into one derived allowlist is
  well-costed — Cargo names, a gitignore pattern, a golden fixture with a Rust
  and a Python co-reader, a workflow glob and a skill permission grammar
  genuinely share no derivation.
- ✅ House idioms are reused rather than reinvented: regex-over-`read_text`
  source scans per `test_bootstrap_coverage.py`, yaml parsing per
  `test_workflows.py`, and `collect_entries`' parameter-with-default shape.
- ✅ Phase ordering is deliberate about churn — Phase 3 lands
  `_release_uploads` in its final shape before Phase 4 adds a seam.
- ✅ Deviations from the work item are enumerated in a dedicated section, so a
  validator can distinguish decision from drift.
- ✅ Every one of the fifteen literal strings the checklist test asserts is
  genuinely present in the drafted section text, and the resulting GitHub
  anchor matches the links already committed in 0170–0173.
- ✅ Documentation updates ride with the code they describe rather than being
  deferred — the `DEBUG_ARCHIVE_DIRS` comment, the generic
  `create_debug_archives` docstring, the `BARE_LAUNCHER` rationale, and the
  explicit rewrite of the stale `tasks/github.py:220-222` comment.

### Recommended Changes

1. **Replace the sentinel probe with a two-part check** (addresses: sentinel
   probe rejects narrower rules; condition 2 accepts token-segment wildcards;
   bare `Bash` breaks the model). Probe with the *actual invoked command* found
   in the SKILL.md — `covered_by(command, rule) and not covered_by(BARE_LAUNCHER,
   rule)` — which makes the guard agree with `skill_permissions` by construction
   and passes the visualiser's real `--owner-pid $PPID ${ARGUMENTS:-start}`
   invocation against its `visualiser *` rule. Add a structural assertion that
   the rule's token segment equals the token exactly, rejecting any segment
   containing `*`, `?` or `[`. Treat `has_bare_bash(text)` as never satisfying
   condition 2 regardless of other rules. Document the accepted rule shapes in
   checklist point 7.

2. **Bound the exemption set** (addresses: `SKILL_EXEMPT_SUBBINARIES` can
   restore full vacuity; the real-repo case can be silenced). Raise when
   `set(exempt) - set(tokens)` is non-empty (a stale exemption becomes the loud
   symptom of a dropped token) and when `set(tokens) <= set(exempt)`. Have the
   real-repo test pass `exempt=()` explicitly. Consider making the exemption a
   `dict[str, str]` mapping token to its declared non-skill consumer path,
   asserted to exist and contain the token, so an exemption carries evidence.

3. **Add `tests/integration/tasks/test_github.py` to Phase 3's change list**
   (addresses: the arity change breaks the pin the phase nominates as evidence).
   Update `_setup_release`'s double to
   `side_effect=lambda token, p: tmp_path / f"accelerator-{token}-{p}.debug.tar.gz"`,
   and state that the 22-upload pin holds *after* that fixture edit rather than
   through it.

4. **Give the debug-archive stage a real seam** (addresses: the one loop that
   changes gets no seam; three injection idioms). Extract
   `debug_archive_targets(dirs=DEBUG_ARCHIVE_DIRS, staging_dir=RELEASE_STAGING)`
   with the `@task` as a thin loop over it, thread a `debug_dirs` parameter
   through `_release_uploads`, and keep `debug_archive_path`'s signature
   parallel to its siblings by passing the directory rather than the mapping.
   Pick one injection idiom repo-wide and migrate the two `test_github.py` patch
   sites to argument passing.

5. **Redraft the eleven checklist items in the imperative and fix their factual
   errors** (addresses: items fail Phase 5's own predicate; points 5, 6, 8, 11,
   3, 2 and the reserved-token rationale). Lead each with the action; name the
   closed verb set in the plan so section and test are drafted against each
   other. State point 5 unconditionally (the launcher caches every fetched
   sub-binary into the plugin `bin/`); correct point 6 to "**No action when** you
   add only a new key — both co-readers are key-agnostic"; extend point 8 with
   `subbinary_asset_path`, the two `tasks/release.py` prepare tasks, the
   `mise.toml` leaf and the `_CLI_RELEASE_BINARIES` route; replace point 11's
   `docs/internals.md` reference with the per-sub-binary page plus docs-index
   entry; add `[lints] workspace = true` to point 3; show the existing
   `_SUBBINARY_MANIFESTS` entry as point 2's worked example; and give `verify`
   and `launcher` their actual, separate reasons.

6. **Make the checklist test cross-reference code rather than itself**
   (addresses: the README test pins docs against docs; `DEBUG_ARCHIVE_DIRS` is
   unenumerated). For each named registration point, assert the symbol resolves
   in the file the checklist attributes it to. Add a twelfth point covering
   `DEBUG_ARCHIVE_DIRS` (with "No action when the sub-binary ships no
   symbolication archive"), add it to the pinned literals, and cross-reference
   it from point 9. Drop the verb-set half of the predicate; keep the count and
   the literals.

7. **Strengthen the SLSA test into a coverage assertion** (addresses: symmetry
   is not coverage; the near-duplicate committed test). Pin the attest-step
   count per publishing job rather than `> 1`, assert set-equality of
   `subject-path` across blocks, and *derive* the expectation from the producer:
   every `DEBUG_ARCHIVE_DIRS` value must appear as a glob in each block. Extend
   the existing `test_attest_globs_include_the_launcher_binaries` rather than
   adding a second, weaker test beside it, and say what happens to its
   hardcoded `accelerator-visualiser-*`.

8. **Relocate the guard to `tasks/dispatch.py` and pin the import direction**
   (addresses: `tasks/shared/` layering inversion; the relocated circular
   import; test-home inconsistency). It follows `tasks/manifest.py`'s task-less
   precedent, keeps every benefit the plan cites, leaves `tasks/shared/` a leaf,
   and makes `tests/unit/tasks/test_dispatch.py` correct rather than an
   exception. Add a source-scan test asserting nothing in `tasks.build`'s import
   closure imports it, and a one-line comment at the import site naming
   `vendor_shims` as the reason.

9. **Wire the guard into `lint:check` as well as the release path** (addresses:
   release-only hot path; runs after signing). Follow the house pattern
   `tasks/README.md:32-38` documents for `lint:vendor-shims` and
   `lint:claude-coupling`, and move the `emit_manifest` call to the function's
   first statement. Keep the argument-free call — an acceptance criterion pins it.

10. **Split the guard's result and its error messages by cause** (addresses:
    `dict[str, bool]` conflates meanings; the message conflates three causes and
    recommends the escape hatch). Return `invoked: set[str]` and `bound:
    set[str]`; carry the invoking path through and emit distinct messages for
    "no skill invokes `accelerator <token>`", "<path> invokes it but its only
    matching rule also authorises the bare launcher", and the bare-`Bash` case.
    Reserve the exemption hint for the no-invocation case only.

11. **Close the upload/re-verify divergence while Phase 4 is in the file**
    (addresses: independently parameterised token sets). Resolve the token tuple
    once in `upload_and_verify_release` and thread it into both `_release_uploads`
    and `_release_reverifies`. Assert `set(manifest["binaries"]) == set(names)`
    before building the re-verify list, and raise on an explicitly empty
    `tokens` rather than returning `[]`.

12. **Fix the Phase 2 test design gaps** (addresses: `_seed` cannot express the
    prose case; the anti-reimplementation scan is bypassable; the spy test needs
    signing). Extend `_seed` with `prose: str = ""` and `bare_bash: bool =
    False`, and specify the prose case's literal body as both a backticked and a
    plain-text mention. Assert each of the six imported names is *used* outside
    the import block, and add `fnmatch` to the forbidden tokens. Patch
    `tasks.manifest.sign_file` (and `validate_version_coherence`) in the spy
    test so it needs no key and no `minisign`.

13. **Pin `BUILTIN_SUBCOMMANDS` against the launcher** (addresses: third
    unpinned copy; point 10's prose-only lockstep). Regex-extract
    `is_root_help`'s `Some("…" | "…" | "…")` arm and assert set-equality,
    following `tests/unit/tasks/test_manifest_contract.py`. Extend point 10 to
    cover removals, and note that a name in the set is unavailable as a token.

14. **Constrain the debug-archive directory** (addresses: archives swept into
    the release commit). Validate in the producer that each registry value's
    final component is `bin` so `.gitignore:54` always covers it, or extend
    `_ARTIFACT_MARKERS` with `.debug.tar.gz` so a non-ignored archive aborts
    before the commit.

15. **Fix the mechanical follow-through omissions and the unsatisfiable
    criteria** (addresses: three omitted edits; two greps stated too strictly;
    the markdown-check claim; the `uv run invoke` criteria). Add the
    `test_build.py:18` import trim, `from collections.abc import Mapping` in
    `paths.py`, and the new `tests/unit/tasks/shared/test_paths.py` filename.
    Add `tasks/github.py:150` to the comment-rewrite list and restate the
    `paths.py` grep as an enumerated allow-list. Drop the markdown-formatting
    criterion (no markdown checker exists) and the Phase 3 actionlint criterion,
    and replace both `uv run invoke` calls with their `mise run` equivalents.

## Per-Lens Results

### Correctness

**Summary**: The plan's core guard logic is largely sound: both directions are
genuinely both-direction, the two permission conditions are correctly conjoined
per-rule inside the `any(...)` rather than as two independent `any(...)` calls,
the anti-vacuity check is placed before the scan, and `tail.startswith(" ")`
correctly prevents `accelerator-verify-*` mis-tokenisation. The dominant
correctness defect is the sentinel probe: it silently requires the consuming
skill to authorise the *whole* subcommand, so a legitimately narrower rule is
reported as unbound and fails the release. Two secondary defects are provable
against committed code: Phase 3's `debug_archive_path(token, platform)` breaks
the one-argument stub at `tests/integration/tasks/test_github.py:252-258` that
the same phase asserts stays green, and Phase 5's drafted checklist items
contradict the mechanical predicate Phase 5 itself specifies.

**Strengths**:
- All three "verified empirically during planning" claims reproduce against the
  tree, including three `attest-build-provenance` blocks at `:421`, `:532`,
  `:554` all carrying identical `subject-path` sets.
- The `None`-sentinel decision for `tasks/github.py` is correct and non-obvious;
  conversely no test patches `signing.DISPATCHED_SUBBINARIES`, so the direct
  default there is safe — the plan drew the line in exactly the right place.
- `_authorises` applies both conditions to a single rule, the semantics the work
  item demands.
- Phase 3's upload loop preserves iteration order exactly.
- The built-in set was corrected to include `help` after reading `is_root_help`
  rather than trusting the work item.

**Findings**:

- **major / high** — Sentinel probe rejects legitimately narrower Bash rules,
  failing the release for a correctly-registered token.
  *Location*: Phase 2 §2 — `_authorises` / the sentinel probe.
  `covered_by` treats the rule as a literal prefix, so condition 1 passes only
  for rules at or above token level. `Bash(…/accelerator vcs status)` → glob
  `…vcs status*` → probe `…vcs zz-subcommand-probe-zz` → `False`; likewise
  `Bash(…/accelerator visualiser --owner-pid *)`. Such rules are fully
  contract-conformant, and the linter's own message at `:184-185` explicitly
  invites them. The plan verifies the probe only against the rule shapes present
  today (all `<token> *`). One of the five sibling stories writing a tightly
  scoped rule gets a red release path with a message instructing it to add what
  it already did; the only workaround broadens the grant. Accept a rule in
  either direction, and state in point 7 which shapes satisfy the binding.

- **major / high** — Phase 3's two-argument `debug_archive_path` breaks the
  committed one-argument stub the phase asserts stays green.
  *Location*: Phase 3 §3 / §5.
  `test_github.py:252-258` stubs with a one-parameter lambda; called with two
  positional arguments it raises `TypeError`, so nine tests through
  `_setup_release` fail, including the 22-upload pin the phase's own criteria
  require. `test_github.py` is not in the change list. Add it, changing the stub
  to `side_effect=lambda token, p: tmp_path / f"accelerator-{token}-{p}.debug.tar.gz"`.

- **major / high** — The drafted checklist items fail the mechanical predicate
  the same phase specifies.
  *Location*: Phase 5 §1 vs §2.
  Items 1, 3, 4, 6, 7 and 10 satisfy neither branch of the predicate (item 10 is
  a gerund); item 8 passes only if the verb set is stretched. Either the test is
  weakened until it asserts nothing, or six items are rewritten during
  implementation — the kind of unplanned rewording that drops a pinned literal.
  Redraft in the imperative and name the closed verb set in the plan.

- **minor / high** — The SLSA test as described cannot detect the fourth-block
  case it claims to catch.
  *Location*: Phase 3 §4. A fourth block with the same `subject-path` set
  satisfies both `> 1` and set-identity. Only asymmetric edits are caught, yet
  the plan records the count as pinned. Assert the count equals 3 explicitly.

- **minor / high** — Two manual-verification greps are stated more strictly than
  the tree permits.
  *Location*: Phase 3 Manual Verification; Manual Testing Steps.
  `tasks/github.py:150` still matches `visualiser` and is never mentioned;
  `paths.py` still matches at `SERVER` (`:14`), `FRONTEND` (`:16`) and
  `subbinary_asset_path`'s docstring (`:67-68`) — roughly eight matches, not
  four. Restate as an enumerated allow-list.

- **minor / high** — Three mechanical follow-through edits are omitted and each
  turns a phase red.
  *Location*: Phase 2 §3/§4; Phase 3 §1/§5.
  (a) `DispatchCoherenceError` becomes unused at `test_build.py:18` — ruff F401.
  (b) `Mapping` is not imported in `paths.py`. (c) The new
  `tests/unit/tasks/shared/` file is unnamed.

- **minor / high** — The error-ordering rationale is misattributed, and the
  "fires because unbound, not unregistered" property is unpinned.
  *Location*: Phase 2 §2. The filter, not `sorted()`, satisfies the
  second-token criterion; `unbound` has one element in that fixture, and the
  sort discards registry order. Add a negative assertion on the absence of the
  "invoked by a skill but not dispatched" substring.

- **minor / medium** — Bare-`Bash` skills count as unbound, but the checklist
  authors will read never says so.
  *Location*: Decisions table; Phase 5 §1 point 7.
  This diverges from `skill_permissions`, which treats bare `Bash` as
  authorising everything (`:167`). Eleven shipped skills declare bare `- Bash`,
  including every Linear and Jira integration skill — plausible first consumers
  of a new token. Point 7 as drafted reads as already satisfied for them.

- **minor / high** — The `launcher` reservation is justified by a collision that
  does not occur.
  *Location*: Phase 5 §1, reserved-token bullet. Holds for `verify`, not for
  `launcher` (`accelerator-<platform>` vs `accelerator-launcher-<platform>`).
  The real hazard is `_default_subbinary_manifest` resolving
  `cli/launcher/Cargo.toml`, an existing non-sub-binary crate.

- **suggestion / medium** — `DEBUG_ARCHIVE_DIRS` has no coherence link to either
  the provenance globs or `DISPATCHED_SUBBINARIES`.
  *Location*: Phase 3 §1; Desired End State. A second entry pointing at a
  different tree silently ships an unattested archive; a key absent from
  `DISPATCHED_SUBBINARIES` surfaces as a bare `FileNotFoundError` from
  `tarfile.add`. The one point converted from prose into a data structure gains
  no mechanical check.

### Test Coverage

**Summary**: The plan is unusually strong on criterion→test traceability: a
fifteen-case guard matrix maps cleanly onto every acceptance criterion, the
non-vacuity cases are named explicitly, and the plan honestly records where an
assertion is degenerate rather than overclaiming. Three concrete problems
undercut it: Phase 3 changes `debug_archive_path`'s arity while asserting the
committed 22-upload pin stays green — the pin's own fixture patches that helper
with a one-argument lambda; the newly generalised debug-archive loop is the one
thing this task actually changes on the release path and it gets no injection
seam; and the seeding helper cannot express the prose/backticked-reference case,
which is precisely the regression test for the bare-substring defect the task
exists to fix. Several source-scan and default-count assertions are also weaker
than the plan claims.

**Strengths**:
- Criterion-to-test traceability is close to complete, with the non-vacuity
  cases identified explicitly rather than assuming a passing test proves a
  working guard.
- The interned-empty-tuple degeneracy is recorded and paired with a source-scan
  companion as "the non-degenerate half".
- The `None`-sentinel decision is correctly grounded in the two committed patch
  sites.
- House test idioms are reused rather than reinvented.
- The probe shape was validated empirically before the matrix was designed.
- The one behavioural reversal is called out three times rather than quietly
  deleted.

**Findings**:

- **major / high** — Phase 3's arity change breaks the fixture behind the
  regression pin it cites.
  *Location*: Phase 3 §3 and Success Criteria. Eight tests through
  `_setup_release` raise `TypeError`, including the 22-upload pin, the
  missing-asset test and both preserve-draft tests. The plan's stated regression
  guard is the very thing that breaks — inviting a loosened assertion or a
  reverted signature rather than a fixture update.

- **major / high** — The debug-archive producer has no injection point, so its
  promised test is unimplementable.
  *Location*: Phase 3 §1/§2/§5. `create_debug_archives` is an `@task` taking
  only `context`, and `debug_archive_path`'s `dirs` default binds at def-time —
  so patching `tasks.build.DEBUG_ARCHIVE_DIRS` leaves the helper resolving
  against the real one-entry registry and a second fixture token raises
  `KeyError`. The test also needs staged binaries for `tar.add`, unmentioned.

- **major / high** — `_seed` cannot express two matrix cases.
  *Location*: Phase 2 §4. No parameter for free-text body content, so the prose
  case degenerates into a duplicate of the no-invoking-skill case; the
  bare-`Bash` case needs a `- Bash` frontmatter line. The prose case is the
  regression test for the exact defect the task exists to remove — today's guard
  reports the visualiser bound on the strength of SKILL.md `:46`'s backticked
  prose. If it degenerates, a reintroduced substring match passes the matrix.

- **major / high** — The real-repo case resolves the live exemption set.
  *Location*: Phase 2 §4. Nothing asserts `SKILL_EXEMPT_SUBBINARIES` is empty or
  excludes `visualiser`, so a future one-line addition makes the only check on
  the guard's one production binding pass vacuously. Pass `exempt=()`
  explicitly.

- **major / medium** — The anti-reimplementation scan has a straightforward
  bypass.
  *Location*: Phase 2 §4. An inline re-implementation using
  `fnmatch.fnmatchcase` or `str.startswith`/`split` satisfies all four clauses,
  and nothing asserts the imported names are *used*. The shadow regex is
  anchored at column 0, so an indented nested `def covered_by` evades it. The
  work item states this is the sole verification of the reuse requirement.

- **major / medium** — The spy test needs signing infrastructure the plan does
  not schedule.
  *Location*: Phase 2 §4; Testing Strategy. `emit_manifest` also runs
  `validate_version_coherence` against the real root and then `sign_file`, which
  shells out to `minisign`. The house precedent skips when the tool is absent, so
  copying it makes the criterion's guard silently non-executing locally. The
  plan also classifies this as an integration test while homing it in a unit
  file.

- **major / medium** — Seams are added where behaviour is unchanged and withheld
  where it changed.
  *Location*: Phase 3 vs Phase 4. All three Phase 4 targets already iterated a
  token collection; the debug-archive loop inside `_release_uploads` is the one
  genuine behavioural change and gets no seam, verified only by a count against
  a single-entry registry.

- **minor / high** — The SLSA test adds little over the committed one.
  *Location*: Phase 3 §4. `> 1` does not pin the count, and
  `test_attest_globs_include_the_launcher_binaries` (`:147-159`) already
  iterates every attest step asserting both globs. The genuinely new power is
  the identical-set assertion.

- **minor / high** — Deleting `TestValidateDispatchCoherence` leaves an unused
  import.
  *Location*: Phase 2 §4. `DispatchCoherenceError` appears only at `:136` and
  `:142`, both inside the deleted class; `F401` is not in the `tests/**`
  per-file ignores under `select = ["ALL"]`.

- **minor / medium** — Two mutations survive the matrix.
  *Location*: Phase 2 §4. No case where the skill invokes the target token while
  carrying a scoped rule for a *different* subcommand, so dropping condition 1
  is undetected. And of three `BUILTIN_SUBCOMMANDS` members only `config` and
  `help` are tested.

- **minor / high** — Default-count assertions are brittle against the five
  consumers this task unblocks.
  *Location*: Phase 4 §4. Four literal counts across two test files, hand-edited
  by each of 0169–0173. Derive from `len(DISPATCHED_SUBBINARIES) * len(TARGETS)`
  and keep one literal pin of the registry.

- **minor / medium** — Point 10's lockstep is enforced by prose alone, and the
  dangerous direction is the silent one.
  *Location*: Phase 5 §1 point 10. A name removed from the launcher but left in
  `BUILTIN_SUBCOMMANDS` silently exempts every invocation of a subcommand that
  no longer exists. A regex over `is_root_help` in the house idiom fixes it.

- **minor / medium** — Test homes contradict the locked decision.
  *Location*: Decisions table; Phase 3 §5. `debug_archive_path` tests go to
  `tests/unit/tasks/shared/` while its siblings are tested in
  `test_build.py::TestCliPathHelpers`, and no `test_paths.py` exists there
  today.

### Architecture

**Summary**: The plan is unusually well-grounded — it reproduced the hard
circular import, the broken `covered_by` probe and the triple SLSA block
empirically before designing, and it correctly locates the registration surface
on the producer side where ADR-0054's constraints actually live. Its central
structural weakness is the chosen circular-import remedy: putting the guard in
`tasks/shared/dispatch.py` makes it the first module in the repo's foundation
package to depend *upward* on `tasks/lint`, which relocates the cycle rather
than removing it and leaves a live footgun that nothing pins. Secondary
concerns are the release-path placement of a repo-wide skills scan that a
PR-time lint guard already traverses, three different injection idioms across
four sibling seams, and a documentation-as-enforcement mechanism pinned in the
wrong direction.

**Strengths**:
- Three specification defects were reproduced empirically at a named revision
  before any design was committed.
- The reuse requirement is defended structurally, not just behaviourally.
- Anti-vacuity is treated as a first-class design property, matching the
  codebase's established guard tradition (`EXPECTED_INJECTION_SKILLS`,
  `TestFailClosed`, exact-set coverage pins).
- The sentinel probe mirrors the existing `BARE_LAUNCHER` design rather than
  inventing a trailing-space literal.
- Phase ordering avoids editing the same code twice.
- The refusal to collapse registration into a single derived allowlist is
  well-costed and correct.
- The yaml test pinning the three attest blocks closes a real drift hole.
- Deviations are enumerated explicitly, and the non-decidability of "an
  imperative action line" is recorded honestly.

**Findings**:

- **major / high** — `tasks/shared/dispatch.py` inverts the foundation-layer
  dependency direction.
  *Location*: Decisions table; Phase 2 §2. Every existing `tasks/shared/`
  module depends only on stdlib, third-party packages or other `tasks.shared`
  modules; this would be the first to import upward into a task package, and
  importing the submodule executes `tasks/lint/__init__.py`, transitively
  pulling in `invoke`, all of `tasks.lint` and `tasks.build`. The stated
  criterion is contradicted by `tasks/manifest.py`, which declares no invoke
  tasks either and lives at `tasks/` — and is the guard's only caller. Home the
  guard at `tasks/dispatch.py` (the research's Remedy B).

- **major / high** — The circular import is relocated, not removed, and nothing
  pins the new invariant.
  *Location*: Current State Analysis; Phase 2 §2. The root-cause edge
  (`vendor_shims.py:3`) is untouched. The moment `tasks/build.py` — or anything
  it imports, including any other `tasks/shared/*` module — imports
  `tasks.shared.dispatch`, the same `ImportError` returns and breaks four entry
  points. That edit is likely: the guard used to live in `build.py` and every
  acceptance criterion still anchors it there. Four source scans are added; none
  covers the import direction.

- **major / medium** — Deferring the parsing extraction preserves the only real
  cause of the coupling problem.
  *Location*: What We're NOT Doing. `skill_permissions.py` mixes a ~60-line pure
  parsing core with an imperative shell, which is why the parsing cannot be
  consumed without dragging in `invoke` and `tasks.build`. The stated reason is
  requirements conformance, not architecture — and the plan already overrides
  the work item in seven other places. The follow-up trigger ("if a third
  consumer appears") means the inversion is load-bearing for all five sibling
  stories.

- **major / high** — A repo-wide skills-tree scan is wired only into the release
  hot path, duplicating a PR-time traversal.
  *Location*: Desired End State; Phase 2 §3. `lint:skill-permissions:check`
  already walks the same tree on every PR and already enforces condition 2
  (`:183-188`), so an ancestor-glob rule cannot exist in a green tree — most of
  the matrix tests unreachable states. A pure sub-second text check gives its
  feedback at the slowest, most expensive and least recoverable point in the
  pipeline. The house pattern (`tasks/README.md:32-38`) is to wire such guards
  into `lint:check`.

- **major / high** — The checklist's enforcement is pinned in the wrong
  direction.
  *Location*: Phase 5 §2. The test only reads the README, so renaming
  `DISPATCHED_SUBBINARIES` or deleting `_assert_static_elf` leaves it green; the
  test fires only when someone edits the README. Make it a cross-reference: for
  each named point, assert the symbol resolves in the attributed source file.

- **major / medium** — Three different injection idioms across four sibling
  seams, chosen to preserve the tests they replace.
  *Location*: Decisions table; Phase 3 §2; Phase 4. Direct defaults, `None`
  sentinels, and a plain global read with no seam. The two semantics differ
  observably, and the split is driven entirely by preserving the very test idiom
  the seam work exists to retire. Phase 3 also contradicts itself: no registry
  parameter in the sketch, an injected registry in the test plan.

- **major / high** — Phase 3's signature change ripples into an unlisted test
  double, so the phase lands red.
  *Location*: Phase 3 §1 / Success Criteria. A positional-arity change to a
  helper exported from the repo's most widely imported module, with
  `test_github.py`'s single-argument double omitted from the change list.
  Consider whether the token argument is needed at all, since both call sites
  already hold it.

- **minor / high** — `tasks/shared/paths.py` accretes a fifth responsibility,
  and the invariant pinning it is unsatisfiable.
  *Location*: Desired End State; Phase 2 §1; Phase 3 Manual Verification. The
  module already carries four concerns; the plan adds guard policy
  (`SKILL_EXEMPT_SUBBINARIES`, whose sibling `BUILTIN_SUBCOMMANDS` lives
  elsewhere) and a mutable `dict` global where its sibling is an immutable
  tuple. The grep criterion cannot pass. Consider a dedicated registry module
  and `MappingProxyType`.

- **minor / medium** — A cross-language registry is duplicated with only a
  documented lockstep obligation, where a house idiom exists.
  *Location*: Phase 2 §2; Phase 5 point 10.
  `tests/unit/tasks/test_manifest_contract.py:51-74` already regex-extracts
  `SUPPORTED_SCHEMA_VERSION` and `HOST_PLATFORM` literals from Rust and asserts
  equality against Python constants.

- **suggestion / medium** — Phase 5's docs test is homed in the dispatch
  module's test file, coupling it to Phase 2.
  *Location*: Implementation Approach; Decisions table; Phase 5 §2. Only points
  1, 7 and 10 concern dispatch; the stated rationale does not apply since the
  README is not that module's artefact. Give it
  `tests/unit/tasks/test_registration_docs.py`.

### Code Quality

**Summary**: The plan is unusually rigorous for a build-system change: it
extracts the guard into a single-responsibility module, reuses the existing
SKILL.md parsing rather than re-implementing it, and reduces most new code to
pure list-derivations that are testable without keys, network or a release. The
main quality risks are in the guard's internal data shape, an error message that
cannot distinguish the two failure modes it already knows apart, and a
proliferation of three different dependency-injection idioms across four sibling
helpers whose divergence is explained only in the plan and not in the code.
Several snippets also spend their comment budget on the less non-obvious facts
while leaving the two genuinely load-bearing subtleties documented only in the
plan.

**Strengths**:
- Extracting the guard to its own module gives it a single responsibility and
  makes the no-literal-`visualiser` assertion a decidable whole-file scan — the
  design choice does double duty.
- `_invoked_token` returning `""` matches the established idiom in
  `skill_permissions.py` (`frontmatter_name`, `_name_after`).
- The new helpers are pure derivations with no I/O, so the release-stage
  assertions need no key, network or staged artefacts.
- Phase ordering is deliberate about churn.
- The `None`-sentinel divergence is empirically justified rather than
  accidental.
- The plan records the interned-empty-tuple limitation rather than overclaiming.

**Findings**:

- **major / high** — `_scan`'s `dict[str, bool]` makes keys and values mean
  different things.
  *Location*: Phase 2 §2. Keys mean "invoked", values mean "authorised", and the
  variable is called `bound` in both readings, so `token in bound` misreads as
  "is bound". The two directions the work item names distinctly collapse into
  one ambiguous structure at exactly the point a third rule would be added.
  Return `invoked: set[str]` and `bound: set[str]` from a function named for its
  result.

- **major / high** — The unbound message conflates two failure modes the code
  already distinguishes.
  *Location*: Phase 2 §2. A token lands in `unbound` because no skill invokes it
  or because its only matching rule is an ancestor glob — opposite fixes, and
  the ancestor-glob case is the subtle one condition 2 exists to catch. Neither
  message names the offending SKILL.md, though `_scan` holds the path and
  discards it. The vacuity message also asserts its conclusion confusingly and
  names no constant.

- **major / high** — The generalised producer gets no seam, so its test must
  patch module state.
  *Location*: Phase 3 §2/§5. Follow the existing `vendor_verify_shims` /
  `vendor_shim_marker_digest` split in the same file — extract
  `debug_archive_targets(...)` and let the `@task` be a thin loop.

- **major / high** — `debug_archive_path` re-looks-up a registry its only two
  callers already iterate.
  *Location*: Phase 3 §1. The mapping parameter diverges from `cli_binary_path`,
  `subbinary_asset_path` and `vendored_shim_path`, which all take a directory,
  and the planned `KeyError` test pins behaviour neither production call site
  can reach. (`Mapping` is also not imported.)

- **major / high** — Divergent injection idioms whose failure mode is silent
  test rot.
  *Location*: Phase 4. The resolution expression is repeated three times, once
  inlined awkwardly into a for-statement, and the loop variable reverts to
  `name` in a change whose purpose is to establish "token" as the domain term. A
  future author who tidies `github.py` to match `signing.py` disables both
  patch-based injections with no test turning red. Add one `_resolve_tokens`
  helper with the single comment that earns its place.

- **minor / high** — Redundant `is_plugin_invocation` condition exists only to
  satisfy an import assertion.
  *Location*: Phase 2 §2. `startswith(LAUNCHER)` already implies it. Deleting it
  breaks a source-scan test whose message talks about imports — a confusing
  coupling to debug. Make the reuse load-bearing or drop the clause.

- **minor / medium** — The comment budget is spent on visibility rationale, not
  the load-bearing subtleties.
  *Location*: Phase 1 §1; Phase 2 §2. The sentinel's necessity and
  `tail.startswith(" ")`'s purpose appear only in the plan; Phase 1's snippet
  replaces the existing semantic comment on `BARE_LAUNCHER` with a visibility
  note. Both undocumented facts look like accidental complexity and will be
  "simplified".

- **minor / high** — Test homes break the source-tree mirror and make one module
  a grab bag.
  *Location*: Decisions table; Phase 5 §2. A fifteen-case matrix, three source
  scans, two signature pins and a docs-structure checker in one module, outside
  the `tests/unit/tasks/shared/` mirror the repo maintains.

- **minor / medium** — The closed verb-set predicate is high-maintenance and
  low-signal.
  *Location*: Phase 5 §2. An innocuous rewording fails the build with a message
  about a verb list, while catching no defect the count and literals do not.

- **minor / medium** — `LAUNCHER` sits next to the imported `BARE_LAUNCHER` and
  invites misreading.
  *Location*: Phase 2 §2. A path prefix beside a complete probe command, two
  lines apart in the one function whose two-condition logic is easy to get
  wrong. Rename to `_LAUNCHER_PREFIX`.

- **minor / high** — Two snippets exceed 80 columns and will be reflowed into a
  worse shape.
  *Location*: Phase 3 §2; Phase 2 §4. The `with tarfile.open(...)` header is 82
  columns and will split across three lines, losing readability relative to
  today's named intermediate.

- **suggestion / medium** — `BUILTIN_SUBCOMMANDS` duplicates a Rust list with
  prose-only enforcement.
  *Location*: Phase 2 §2; Phase 5 point 10. In a plan that otherwise mechanises
  every obligation it documents, including a yaml test for three workflow
  blocks.

### Safety

**Summary**: The plan is unusually safety-conscious for a release-producer
change: it identifies the anti-vacuity requirement explicitly, pins builder
defaults by identity and by census count, preserves the 22-upload equality pin,
and keeps every behavioural default byte-identical to today's asset set. The
residual risks are all in the same shape — the new escape hatches and registries
it introduces are themselves unguarded, so the guard the plan hardens can be
made vacuous again by a one-line edit no test catches, and the new SLSA test
asserts internal symmetry rather than coverage of the artefacts actually
produced. Sequencing is sound: no phase leaves a published partial release
possible, because the whole gate sits inside the existing pre-publish draft
envelope and before any commit, tag or push.

**Strengths**:
- Anti-vacuity is a first-class safety property, implemented as a hard failure
  and deliberately reversing a committed test.
- The default-pinning strategy is the right defence against the failure that
  matters, and the plan names the interned-empty-tuple degeneracy rather than
  pretending identity is sufficient.
- Phase 3 preserves the archive filename and upload order, so the 22-upload pin
  keeps working as a census.
- The guard fails closed before `git.commit_version`/`tag_version`/`push` and
  before `create_release`, so a coherence failure leaves no tag, draft or
  published asset; the existing draft-preserve envelope still contains later
  failures.
- The real-repo case runs in `test-unit`, which runs on `pull_request` and which
  `prerelease` depends on — so guard drift surfaces as a red PR.
- Risky assumptions were tested rather than assumed.

**Findings**:

- **critical / high** — `SKILL_EXEMPT_SUBBINARIES` can restore full vacuity,
  defeating the anti-vacuity guard it ships beside.
  *Location*: Phase 2 §1 and the entry point. With
  `SKILL_EXEMPT_SUBBINARIES = ("visualiser",)` the collection is non-empty,
  `unbound` and `unregistered` are empty, and the gate passes having checked
  nothing. The work item forbids this in prose only; the matrix covers "exempt
  passes" and "exemption removed raises" but never bounds the set. The same gap
  hides token *loss*: an exempt token deleted from `DISPATCHED_SUBBINARIES`
  produces no complaint, no manifest entry, no signature and no upload — the
  launcher just 404s for every hook user. 0169's hook-invoked `vcs` is the
  archetypal exemption case, so the hatch will be used and normalised in the
  very next change. Raise on `set(exempt) - set(tokens)` and on
  `set(tokens) <= set(exempt)`; consider a `dict[str, str]` carrying the
  declared non-skill consumer path.

- **major / high** — The SLSA test asserts symmetry between blocks, not
  attestation coverage of the artefacts produced.
  *Location*: Phase 3 §4. With `> 1`, deleting one of three blocks leaves two
  identical blocks and a green test — an entire release track then publishes
  signed binaries with no provenance. Symmetry is not coverage: all three can
  agree on a set omitting a newly-staged tree, exactly what point 9 leaves to
  diligence. The suite already has
  `test_attest_globs_include_the_launcher_binaries`, which must not be
  displaced. Pin the count, or assert one attest step per `Sign*` step per job
  (the suite already enumerates `Sign*` steps at `:127`), and derive the
  expected `subject-path` set from `DEBUG_ARCHIVE_DIRS`.

- **major / high** — Phase 3 creates a new registration point that the
  eleven-point checklist never names.
  *Location*: Phase 3 §1; Phase 5 §1/§2. `DEBUG_ARCHIVE_DIRS` decides whether a
  sub-binary ships symbolication archives and into which committed tree, and is
  the trigger for point 9's obligation — yet it is in neither the eleven points
  nor the fourteen pinned identifiers. An author following the checklist ships
  without archives, silently. It is also the one release-stage collection Phase 4
  leaves without a seam.

- **major / medium** — A debug archive staged outside a `bin/` directory is swept
  into the release commit by `git add .`.
  *Location*: Phase 3 §1; Phase 5 point 5. The only leak guard,
  `_assert_no_leaked_artifacts`, matches just `.sec`, `dist/release/` and
  `dist/`; protection is `.gitignore:54`'s `bin/*.debug.tar.gz`, which matches
  only directories literally named `bin`. Point 5's "already token-generic"
  claim is true of the token but not the directory. Multi-megabyte binaries land
  on `main`, in the tag, and in the shipped package.

- **major / medium** — Nothing checks that every binary the signed manifest
  promises was actually uploaded and re-verified.
  *Location*: Phase 4 §3. A manifest entry with no corresponding token produces
  no upload, no re-verification and no complaint — a published, signed manifest
  advertising an asset that was never uploaded. Because `()` is not `None`, an
  explicitly empty `tokens` now takes the `if not names: return []` path and
  empties uploads and re-verifications while the manifest still lists them.
  Assert `set(manifest["binaries"]) == set(names)`; raise on an explicitly empty
  collection.

- **major / medium** — The sentinel probe rejects rules scoped tighter than the
  token, turning a security improvement into a red gate.
  *Location*: Phase 2 §2. A maintainer narrowing a skill's `allowed-tools` — a
  strict improvement, and the direction `skill_permissions` pushes — makes the
  token report as unbound, with a message that will not point at the rule that
  caused it. Probe with the *actual invoked command* instead: byte-for-byte the
  semantics `skill_permissions` already enforces, still rejecting the ancestor
  glob via condition 2, and it removes the probe literal from the design
  entirely.

- **minor / high** — The gate runs after signing and after the manifest has
  already been written to disk.
  *Location*: Phase 2 §3. A pure static scan costs a full four-target
  cross-compile and a use of the release signing secret before it is detected,
  and leaves an unsigned `manifest.json` beside a stale `manifest.minisig`. Call
  it as the first statement of `emit_manifest`, and additionally from
  `release_prepare`/`prerelease_prepare` or `lint:check`.

- **minor / medium** — Flag-shaped first arguments are treated as unregistered
  tokens and fail the release gate.
  *Location*: Phase 2 §2. `--version` yields "needs an entry in
  DISPATCHED_SUBBINARIES" — nonsense for a flag. Return an empty token when the
  first argument starts with `-`; `is_root_help` already treats bare flags as
  root help.

- **minor / high** — The committed 22-upload regression pin cannot run
  unchanged.
  *Location*: Phase 3 §3 and Success Criteria. The plan nominates it as the
  evidence that Phase 3 changes no behaviour, while breaking it — inviting a
  hasty `lambda *a` patch. `create_debug_archives` is meanwhile mocked out
  everywhere (`test_release.py:75`, `:114`) and has no direct test today, so the
  new archive tests are the rewrite's only coverage. Declare `dirs` as a
  read-only `Mapping` (or `MappingProxyType`) so a test cannot mutate the
  release registry for the session.

### Security

**Summary**: The plan strengthens the release path in real ways — it replaces a
bare-substring binding check with a structured two-directional guard, adds a
genuine anti-vacuity failure for a lost `DISPATCHED_SUBBINARIES`, keeps fixture
tokens out of the real signing/manifest/upload paths, and pins the guard's
defaults by identity. But it also promotes `skill_permissions.py`'s permission
model from an advisory lint to a release gate without hardening it: the
two-condition probe accepts over-broad rules whose wildcard sits in the token
segment, ignores bare `Bash` declared alongside a scoped rule, and depends on
`fnmatch` semantics nothing tests against Claude Code's actual matcher.
Separately, Phase 3 generalises `DEBUG_ARCHIVE_DIRS` into an unsigned,
un-re-verified publish loop whose only integrity control is a token-specific
SLSA subject-path glob, and the replacement SLSA test asserts symmetry rather
than coverage.

**Strengths**:
- The anti-vacuity failure is a genuine defence: today an emptied registry would
  silently produce a manifest with no `binaries`, empty the signing expected set
  and make `_subbinary_reverifies` early-return `[]`.
- The guard runs after `atomic_write_text` but before `sign_file`, so it fails
  closed with no attestation or upload having run.
- Tokenisation is fail-closed for dynamically-computed tokens and flag-first
  invocations; combined with `has_metacharacter`'s ban on `$(` and backticks,
  the `!`-site token space stays statically decidable.
- Keeping fixture tokens out of `DISPATCHED_SUBBINARIES` means verification
  never perturbs real derivation.
- The identity pins close a hole the spy test cannot see.
- The positive import assertions are the right mechanism for a reuse requirement
  every behavioural test would pass equally against a re-implementation.
- `tail.startswith(" ")` correctly prevents `bin/accelerator-verify-*` — a
  distinct trust artefact — from being mis-tokenised into the dispatch
  namespace.
- Phase 1's rename is the right call: a release gate depending on a lint
  module's private probe literal is exactly the coupling to remove.

**Findings**:

- **major / high** — `SKILL_EXEMPT_SUBBINARIES` can silence the guard completely
  and the anti-vacuity check does not cover it.
  *Location*: Phase 2 §1/§2. No assertion that `exempt ⊆ tokens`, that a
  non-exempt token remains, or that the stated bar holds. Exempt tokens also
  skip the *permission* direction, so a skill may invoke one under
  `Bash(…/accelerator *)` unchallenged. The easiest way to make a red release
  green is to add the token to the exemption set — disabling both the binding
  and the ancestor-glob rejection that is this guard's whole reason to exist.
  Pin the constant's literal contents the way `EXPECTED_INJECTION_SKILLS` is
  pinned, and require a `hooks/**` or `scripts/**` reference for each exemption.

- **major / high** — The SLSA symmetry test proves consistency, not coverage — a
  new `DEBUG_ARCHIVE_DIRS` entry publishes an unattested, unsigned,
  un-re-verified asset.
  *Location*: Phase 3 §1–§4. All three blocks carry the token-specific
  `skills/visualisation/visualise/bin/accelerator-visualiser-*` glob. Adding one
  dict entry immediately publishes assets that are (a) not in
  `sign_staged_binaries`' expected set, so unsigned; (b) not in
  `_release_reverifies`, so never re-verified before `--draft=false`; and (c) not
  matched by any subject-path — even a second token in the same `BIN_DIR`
  produces a name `accelerator-visualiser-*` does not match. Provenance is
  therefore the *only* integrity control on debug archives. Derive the
  expectation from the registry, pin the count per publishing job, and extend
  the existing test rather than adding a weaker one beside it. Note its
  hardcoded `visualiser` literal is what the no-literal-`visualiser` goal would
  tempt relaxing.

- **major / high** — Condition 2 accepts over-broad rules whose wildcard sits in
  the token segment.
  *Location*: Phase 2 §2. `Bash(…/accelerator v*)` covers the probe and not the
  `zz-`-prefixed bare-launcher probe, so the guard certifies it as
  correctly-scoped — while it pre-authorises `visualiser`, `vcs`, `version` and
  every future `v`-prefixed token. `[a-y]*` authorises everything not beginning
  with `z`, and still passes, because the sentinel's sole job is to be
  `z`-prefixed. The existing lint has the same blind spot. Require the rule's
  extracted token segment to equal `token` exactly, rejecting segments
  containing `*`, `?` or `[`; keep the `BARE_LAUNCHER` probe as a redundant
  layer.

- **major / high** — A bare `Bash` declaration alongside a scoped rule passes
  both the guard and the lint while authorising every sub-binary.
  *Location*: Decisions table. The "counts as unbound" reasoning holds only
  when there is *no* scoped rule. With both, `_authorises` returns `True` and the
  guard reports the token bound and correctly scoped — while bare `Bash` grants
  unrestricted Bash, strictly broader than the ancestor glob condition 2
  rejects. `has_bare_bash` is never itself a violation; it only *suppresses* the
  coverage check. Eleven skills declare bare `Bash` today — `skills/config/migrate`
  and all ten Linear/Jira skills, precisely where 0170's `work-item` token would
  be invoked. Have `_scan` treat a bare-`Bash` skill as never satisfying
  condition 2.

- **major / medium** — The built-in allowlist is a third unverified copy of a
  Rust-side fact and only drifts safely in one direction.
  *Location*: Phase 2 §2; Phase 5 point 10. ADR-0054 names only `version` and
  `config`. Point 10 documents additions but not removals, and nothing is
  compile- or test-enforced either way. If `help` ever stops being a clap
  built-in, `accelerator help` routes to `External(["help"])` and the allowlist
  means a skill invoking it needs no registration and no permission check —
  while `override_path` honours `ACCELERATOR_HELP_BIN` "before any fetch,
  returning the path unverified". Pin the three copies against each other.

- **major / medium** — Upload and re-verify token sets become independently
  parameterised with no invariant tying them.
  *Location*: Phase 4 §2/§3. `_release_reverifies` — the intermediate
  `upload_and_verify_release` actually calls — gains no `tokens` parameter, so
  the seam cannot be threaded consistently, and `_subbinary_reverifies` gains an
  independent `manifest_path`. Today the two halves agree only because both read
  the same global. Resolve once in `upload_and_verify_release` and assert every
  uploaded `accelerator-<token>-<platform>` appears in the re-verify set.

- **minor / medium** — The release gate now depends on `covered_by`'s unverified
  model of Claude Code's permission matcher.
  *Location*: Phase 1; Phase 2. `fnmatch` honours `?`, `[seq]` and `[!seq]`,
  which a plain prefix/`*` matcher would treat as literals, and a rule not
  ending in `*` is silently widened. Neither is covered by any test (the
  `tests/integration/skill-invocation` suite executes commands; it does not
  exercise the matcher). Add a shared matcher-contract table referenced from
  both consumers, or narrow `covered_by` to translate only `*` so the guard
  cannot credit wildcard classes the real matcher does not implement.

- **minor / high** — The guard's scope is `!`-preprocessor sites in
  `skills/**/SKILL.md` only, which the checklist should state explicitly.
  *Location*: Phase 2 §2; Phase 5 point 7. Invisible: invocations from `hooks/`
  and `scripts/` (`hooks/config-detect.sh` already invokes `bin/accelerator`);
  commands not leading with the prefix, e.g. `env FOO=1 …` (skipped by both this
  guard and `skill_permissions`' coverage check); and model-driven Bash. In
  practice (2) yields a permission prompt rather than silent execution, so
  exposure is limited — but point 7 presents "the skill binding" without stating
  the scope, the same reasoning gap that makes the exemption's "consumer is a
  hook" claim unchecked.

### Documentation

**Summary**: The plan's non-documentation phases are unusually well-anchored —
nearly every file:line citation verifies against the tree, and the new
docstrings and comments are drafted inline rather than deferred. Phase 5,
however, is the weakest part of the plan measured against its own stated
purpose: the eleven points are drafted as noun-phrase pointers rather than
actions, three points (5, 6, 11) make claims the code contradicts, and point 8
omits the two `tasks/release.py` call sites without which a new sub-binary is
never built in CI. The mechanical literal-string test also does not protect
against the failure mode it claims.

**Strengths**:
- All fifteen pinned literal strings are genuinely present in the drafted
  section text, verified string by string.
- The heading is pinned implicitly by the test's slice, and the resulting GitHub
  anchor matches the links already committed in 0170–0173.
- The plan is honest about the limits of its own mechanisation.
- Citations verify: `_VISUALISE_SKILL_RELATIVE` at `build.py:35`, the guard at
  `:189-208`, `DISPATCHED_SUBBINARIES` at `paths.py:25`, `debug_archive_path` at
  `:79-80`, `create_debug_archives` at `:498-510`, `_assert_static_elf` at
  `:132-159`, `_SUBBINARY_MANIFESTS` at `manifest.py:51-53`, three attest
  blocks, `is_root_help`'s three-name list, `.gitignore:44`/`:54`, and both
  `manifest.example.json` co-readers.
- The plan corrects the work item where the work item was wrong and records each
  correction in a dedicated Deviations section.
- Documentation updates ride with the code they describe.

**Findings**:

- **major / high** — The literal-string test freezes doc text instead of pinning
  doc↔code correspondence.
  *Location*: Phase 5 §2; Desired End State. Renaming a symbol leaves the README
  stale and green, while an author who correctly updates the README makes the
  test **fail** — so the guard actively resists the maintenance it claims to
  force, which is how such tests get deleted. Pair each literal with a
  source-existence assertion.

- **major / high** — The drafted eleven points are noun phrases naming
  locations, not actions.
  *Location*: Phase 5 §1. Points 1, 3, 4, 6 and 7 fail the plan's own predicate,
  and point 8 is borderline — so the Phase 5 test would reject the Phase 5
  content as drafted. A list of file names leaves each action implicit (add?
  edit? check?) for an audience whose whole purpose is to follow it
  top-to-bottom.

- **major / high** — Checklist point 5 misattributes the `.gitignore`
  mechanism.
  *Location*: Phase 5 §1 point 5. `bin/visualiser-*` (`.gitignore:44`) exists
  because the launcher caches the *fetched sub-binary* as
  `{token}-{version}-{sha256}` (`resolve/cache.rs:30`) into
  `${ACCELERATOR_PLUGIN_ROOT}/bin` (`cache_root.rs:65`) — which happens for
  **every** dispatched sub-binary, whether or not anything is staged there at
  release time. The gitignore comment at `:35-41` says exactly this. An author
  taking the exemption finds a multi-megabyte untracked binary in their working
  copy, with a real risk of committing it. State the point unconditionally.

- **major / high** — Checklist point 8 omits where the staging task must be
  invoked.
  *Location*: Phase 5 §1 point 8. `build.server_cross_compile(context)` appears
  in `tasks/release.py:91` (`prerelease_prepare`) and `:122`
  (`release_prepare`), plus the mise leaf `build:server:cross-compile`
  (`mise.toml:120-123`). It also omits that for a `cli/` workspace member the
  simplest route is adding the binary name to `_CLI_RELEASE_BINARIES`
  (`build.py:37`), which already gives `_assert_magic_bytes` + musl
  `_assert_static_elf` and stages to the name `subbinary_asset_path` expects. An
  author can write a correct staging task CI never calls; the failure surfaces
  late as a missing file in `collect_entries`.

- **major / medium** — Checklist point 6's "both co-readers in the same change"
  is false for the case an author will be in.
  *Location*: Phase 5 §1 point 6. Neither co-reader is affected by *adding* a
  token: `test_manifest_contract.py` iterates `binaries.values()` generically
  (`:30-41`), and the Rust `include_str!` test reads the existing `visualiser`
  entry (`resolve/manifest.rs:146-148`). The point also states no action, so it
  is unclear whether an entry is needed at all. The most emphatic instruction in
  the list being false undermines trust in the rest.

- **major / medium** — Checklist point 11's claims about the user-facing docs are
  partly wrong and omit the real obligation.
  *Location*: Phase 5 §1 point 11. `docs/visualiser.md` checks out (Customisation
  `:59-69`, First-run download `:42-55`), but `docs/internals.md`'s env-var table
  (`:202-215`) holds only `ACCELERATOR_CACHE_DIR` and
  `ACCELERATOR_RELEASE_BASE_URL` — launcher-wide, already token-generic.
  Conversely `docs/visualiser.md` is visualiser-scoped: a `vcs` sub-binary needs
  its own page and a docs-index entry, not extra rows. No action is stated
  either way.

- **major / high** — The unbound diagnostic names one of three causes and offers
  the forbidden remedy.
  *Location*: Phase 2 §2. An author whose real problem is an over-broad rule is
  told a consuming skill is missing, and the suggested
  `SKILL_EXEMPT_SUBBINARIES` escape would permanently vacate the check the work
  item forbids vacating.

- **minor / high** — The `validate_dispatch_coherence` docstring's
  "first/second" is unresolvable.
  *Location*: Phase 2 §2. Within the docstring the two enumerated things are the
  invocation and the rule, and an ancestor glob does not satisfy the invocation
  at all; "first" and "second" silently refer to the work item's permission
  conditions, which the docstring never enumerates. This is the module's only
  prose explanation of its subtlest rule.

- **minor / high** — The insertion point is ambiguous and would reparent three
  subsections.
  *Location*: Phase 5 §1. "After `:50`" inserts the new `##` between the
  Conventions heading and its own body (`:51-147`), reparenting Executable-bit
  invariant (`:68`), Rust nightly lane (`:101`) and Contributor environment
  variables (`:137`). Say "immediately before `## CI job → local command`". The
  README's opening self-description also stops being accurate.

- **minor / high** — Phase 5 records automated markdown assurance that does not
  exist.
  *Location*: Phase 5 Success Criteria. `format:build-system:check` is ruff over
  Python (`mise.toml:314-317`); nothing in the repo formats or lints Markdown,
  including the 80-column convention `.editorconfig` states.

- **minor / medium** — The `_SUBBINARY_MANIFESTS` rule is stated abstractly
  without the one worked example that exists.
  *Location*: Phase 5 §1 point 2 / body. 0168's Dependencies bullet
  (`:183-193`) expects the landed nested placement as the worked example, and
  0169 is exactly that case — `cli/vcs/` already holds a `vcs` domain crate. The
  first author to use the checklist hits the non-default case immediately.

- **minor / medium** — The reserved-token rationale does not hold for
  `launcher`.
  *Location*: Phase 5 §1 body. Nothing stages `accelerator-launcher-*` into
  `dist/release/`; the real clash is the bootstrap's cache name
  `bin/accelerator-launcher-${version}-${platform}` (`bin/accelerator:305`,
  ignored by `.gitignore:42`). A reader who checks the stated reason and finds it
  false will treat neighbouring claims as folklore.

- **minor / medium** — Checklist point 3 understates the inherited-key set and
  mis-states the version rationale.
  *Location*: Phase 5 §1 point 3. `_pinned_member_versions` (`build.py:74-101`)
  only reports a *mismatch*, so a hardcoded current version passes today and
  breaks at the next bump — the real reason to inherit. Every existing member
  also carries `edition/rust-version/license/publish.workspace = true` and
  `[lints] workspace = true` (e.g. `cli/vcs/Cargo.toml:3-10`), the latter
  applying `warnings = "deny"` and the clippy opt-ins. A crate created from
  point 3 alone silently opts out of workspace lint enforcement.

### Standards

**Summary**: The plan is unusually convention-aware — it cites the right house
precedents, matches the repo's British spelling and error-message style, and
enumerates its deviations from the work item explicitly. The main standards
problems are structural rather than stylistic: the new module is placed in
`tasks/shared/`, the one package whose every existing member imports nothing
outside `tasks.shared.*`, and the test-file placement then contradicts both that
package's strict mirror convention and the plan's own stated "follow each
module's existing home" rule. A handful of Success Criteria commands are either
not the sanctioned `mise run` entry point or verify something they do not check,
and two snippets plus one missing import would fail `mise run
build-system:check` as written.

**Strengths**:
- Deviations from the work item are enumerated in a dedicated section, including
  naming and heading-level choices.
- Cites the correct house precedents for each new test idiom.
- `SKILL_EXEMPT_SUBBINARIES` reads as an explicit sibling of
  `DISPATCHED_SUBBINARIES`, with the one-line orienting comment that file's
  constants carry.
- Error messages and identifiers use the repo's British spelling and its
  multi-clause, actionable message style.
- Docstrings follow the house idiom exactly, which is what ruff's enabled
  D2xx/D4xx rules require given D100-D107 are ignored.
- The checklist references paths without line numbers, matching how
  `tasks/README.md` already refers to source files, rather than copying the work
  item's line-anchored references that would age immediately.
- Correctly distinguishes the `None`-sentinel seam from the direct default and
  states the reason at each site.

**Findings**:

- **major / high** — `tasks/shared/dispatch.py` inverts the shared-package
  layering, and the stated rationale is contradicted by `tasks/manifest.py`.
  *Location*: Decisions table; Phase 2 §2. All twenty existing `tasks/shared/`
  modules import only from `tasks.shared.*` (verified: `shared/locking.py`,
  `shared/playwright.py`, `shared/dev/*` are the only intra-`tasks` importers
  and all stay inside `shared`). `tasks/manifest.py` declares no `@task` and
  lives at `tasks/`, and is not imported by `tasks/__init__.py` — the same would
  be true of `tasks/dispatch.py`, which also makes
  `tests/unit/tasks/test_dispatch.py` the correct home rather than an exception.

- **major / high** — Test-file placement breaks the `tests/unit/tasks/shared/`
  mirror and is internally inconsistent.
  *Location*: Decisions table; Phase 2 §4; Phase 3 §5. The repo mirrors
  `tasks/shared/**` one-to-one; the plan puts the module in `tasks/shared/` and
  its tests at the top level while placing `paths.py` tests inside the mirror in
  the next phase. It also calls `tests/unit/tasks/shared/` an "existing package"
  though the tree deliberately has no `__init__.py`, never names the new file,
  and does not say what happens to `TestCliPathHelpers`
  (`test_build.py:174-186`), which already covers `paths.py` helpers — so
  `paths.py` coverage splits across two files, which the Test-homes decision
  says it is avoiding.

- **minor / high** — Deleting `TestValidateDispatchCoherence` leaves an unused
  import that fails ruff.
  *Location*: Phase 2 §4. `DispatchCoherenceError` is used only at `:136` and
  `:142`, both inside the deleted class; `F401` is not in the `tests/**`
  per-file ignores (`S`, `ANN`, `D`, `PLR2004`, `SLF001`, `PT`, `INP001`) under
  `select = ["ALL"]`.

- **minor / high** — The new `debug_archive_path` signature uses `Mapping`
  without adding the import.
  *Location*: Phase 3 §1. `paths.py` imports only `tomllib`, `Path` and `Any`.
  As written the snippet fails ruff (`F821`) and pyrefly. Match
  `tasks/manifest.py:2`, which imports `Mapping` from `collections.abc` for the
  same purpose.

- **minor / high** — Two code snippets exceed the 80-column limit.
  *Location*: Phase 2 §4; Phase 3 §2. The `with tarfile.open(...)` line is 82
  columns; `_seed`'s `write_text` is 81 (the equivalent at
  `test_skill_permissions.py:25` sits at exactly 80 only because its
  interpolations are shorter). Pre-wrap both — binding `archive =
  debug_archive_path(token, platform)` also matches today's `build.py:508`.

- **minor / high** — `format:build-system:check` is cited as verifying markdown
  formatting, which it does not.
  *Location*: Phase 5 Success Criteria. It runs `uv run ruff format --check`
  (`tasks/format/build_system.py:13`), and the repo has no markdown formatter or
  linter at all (no markdownlint, mdformat or prettier in any config).

- **minor / high** — Two criteria call `uv run invoke` where sanctioned `mise
  run` tasks exist.
  *Location*: Phase 1 and Phase 3 Success Criteria.
  `mise run lint:skill-permissions:check` (`mise.toml:407`) and
  `mise run lint:workflows:check` (`:339`) also carry the
  `deps:install:python` dependency the bare call skips. Phase 3 changes no
  workflow YAML, so the actionlint criterion verifies an unmodified file.

- **minor / medium** — Token registries split across two modules with three
  collection types.
  *Location*: Phase 2 §2; Phase 5 point 10. `paths.py` is already the de-facto
  registry home (`DISPATCHED_SUBBINARIES` is not a path either). Mixed
  `tuple` / `dict` / `frozenset` typing makes the group read as unrelated rather
  than as one registry family.

- **minor / medium** — Public/private constant discipline is applied
  inconsistently in the new module.
  *Location*: Phase 1 §1; Phase 2 §2. `skill_permissions.py` makes constants
  private by default and marks a public one with `# Public: …` stating why. The
  new module declares `LAUNCHER` and `BUILTIN_SUBCOMMANDS` public without
  justification while `_PROBE_SENTINEL` beside them is private, and `LAUNCHER`
  has no external consumer. Phase 1's snippet also replaces, rather than keeps,
  the existing semantic comment — the only line explaining what the value *is*.

- **minor / medium** — Two structurally identical seams introduced in one phase,
  one public and one private.
  *Location*: Phase 4 §1/§2. Both modules' existing style is underscore-private
  (`_sig`, `_release_uploads`, `_subbinary_reverifies`, `_signature_path`), and
  tests here import private helpers routinely with `SLF001` per-file-ignored. The
  asymmetry reads as meaningful when nothing outside the module calls it.

- **minor / medium** — The `emit_manifest` call-site test is placed away from
  `emit_manifest`'s existing test home, and classified inconsistently.
  *Location*: Phase 2 §4; Testing Strategy. Every existing `emit_manifest` test
  lives in `tests/unit/tasks/test_manifest.py:194-229`, and the plan's own
  Test-homes rule says not to split a module's coverage. Testing Strategy lists
  it under Integration Tests while Phase 2 places it in `tests/unit/`.

- **minor / medium** — `tasks/README.md` structural assertions hidden in a test
  named after the dispatch guard.
  *Location*: Phase 5 §2. The repo already has the convention of a dedicated
  file for cross-cutting guards over non-Python artefacts (`test_mise.py`,
  `test_workflows.py`, `test_bootstrap_coverage.py`). A contributor editing the
  README and getting a failure from `test_dispatch.py` will not connect the two.

- **minor / medium** — New helper names diverge from close siblings that already
  do the same job.
  *Location*: Phase 2 §2/§4. `_seed` is a near-duplicate of `_skill`
  (`test_skill_permissions.py:22-25`) under a different name and without that
  file's annotations; there is precedent for lifting a shared seeder
  (`tests/unit/tasks/shared/doubles.py`,
  `tests/integration/support/skill_corpus.py`). `_scan` is far terser than the
  house naming and does not say what it returns.

- **minor / high** — `debug_archive_path`'s arity change breaks an existing test
  double the plan does not enumerate.
  *Location*: Phase 3 §3 and Success Criteria. The plan carefully enumerates
  co-readers elsewhere (point 6's two co-readers; Phase 4's `:271`/`:427`
  patches) but omits this one while asserting the 22-upload pin stays green.
  Mirror the `subbinary_asset_path` double directly below it at `:264-268`.

- **suggestion / medium** — Injected token collections annotated
  `tuple[str, ...]` where the cited precedent uses `Iterable[str]`.
  *Location*: Phase 2 §2; Phase 4 §1. `collect_entries` — named as "the house
  precedent to copy" — uses `Iterable[str]`. The identity pin works identically
  under either annotation, and the narrower type blocks injecting a plain list.

---

## Re-Review (Pass 2) — 2026-08-03

**Verdict:** REVISE

All eight lenses re-ran against the revised plan. The redesigned permission
check is **verified correct** — the false negatives and false positives from
pass 1 are genuinely fixed, confirmed independently against the tree — and the
layering, error-reporting, test-home and checklist-accuracy findings are
resolved. But the revision introduced roughly a dozen new defects of its own,
three of them blocking, and it encoded one **false invariant** into code, docs
and tests. Two statements in pass 1 were themselves wrong and are corrected
below.

### Corrections to pass 1

- **The `.gitignore` claim was wrong.** Pass 1 (and the plan's Key Discoveries)
  treated `bin/*.debug.tar.gz` as matching a `bin` directory at any depth.
  Verified in a scratch repo: a pattern with a mid-string separator is anchored
  to its `.gitignore`'s own directory, so it matches only
  `<repo-root>/bin/*.debug.tar.gz` and **not**
  `skills/visualisation/visualise/bin/`. `**/bin/*.debug.tar.gz` does match.
  (`git check-ignore` run from inside this jj workspace is useless here — it
  reports a match on the parent repo's `workspaces/` rule regardless.) This
  makes the `bin/`-suffix constraint added in the revision a false invariant,
  and it also means the skill-tree archives are currently
  untracked-and-unignored: `.gitignore:28`'s comment still claims "the four
  `bin/*.debug.tar.gz` archives under this tree are tracked", which the recent
  untrack commit made stale. `tasks/git.py` stages with a bare `git add .` and
  `_assert_no_leaked_artifacts` screens only `.sec`, `dist/release/`, `dist/` —
  so this is a live pre-existing hazard, not merely a plan defect.
- **`lint:check` is not the CI gate pass 1 assumed.** Recommendation 9 in pass 1
  said wiring the guard into `lint:check` would give PR-time feedback. Verified:
  `check` depends on `frontend/server/cli/deny/pup/build-system/scripts:check`
  and **not** on `lint:check`, and no CI job runs `lint:check` or bare `check`.
  `lint:skill-permissions:check` — the pattern cited as house precedent — is
  itself in that blind spot.

### Previously Identified Issues

**Resolved**

- 🔴 **Safety + Security**: exemption vacuity — Resolved for cardinality. The
  stale-exemption and `tokens ⊆ exempt` raises close the one-line-to-vacuous
  path. (The *bar* is still unenforced — see below.)
- 🟡 **Correctness + Safety**: sentinel probe rejects narrower rules — Resolved.
  Verified: `visualiser start` and `visualiser --owner-pid *` now bind, and the
  visualiser's real `--owner-pid $PPID ${ARGUMENTS:-start}` invocation binds
  against its real `visualiser *` rule.
- 🟡 **Security**: token-segment wildcards accepted — Resolved. Verified that
  `v*`, `[a-y]*`, `?isualiser`, `[v]isualiser`, `"visualiser"`, `visualiser*`
  and whitespace variants all now fail segment equality.
- 🟡 **Correctness + 4 others**: Phase 3's arity change breaks `_setup_release` —
  Resolved; the fixture is now in the change list with the two-argument lambda.
- 🟡 **Architecture + Standards**: `tasks/shared/` layering inversion — Resolved
  by Phase 1. Verified that `tasks/lint/dispatch.py` → `tasks.shared.dispatch`
  introduces no cycle.
- 🟡 **Architecture**: circular import relocated not removed — Resolved; fixed at
  the `vendor_shims` cause, with the leaf invariant asserted.
- 🟡 **Code Quality**: `dict[str, bool]` conflation — Resolved via `_bindings`
  returning a bound set plus a token→path map.
- 🟡 **Code Quality + Documentation**: error messages conflate causes — Resolved;
  split by cause and naming the SKILL.md.
- 🟡 **Architecture + Documentation**: README test pins docs against docs —
  Resolved via the source-existence half.
- 🟡 **Documentation**: checklist points 5, 6, 8, 11, 2, 3, reserved tokens —
  Mostly resolved. Point 5's launcher-cache correction is verified right; points
  6 and 8 are accurate; point 2's worked example is byte-accurate. Point 3 and
  the `launcher` reservation introduced **new** errors (below).
- 🟡 **Architecture + Standards + Code Quality**: test homes — Resolved.
- 🟡 **Test Coverage**: `_seed` cannot express prose; real-repo case silenceable;
  anti-reimplementation bypass; spy test needs `minisign` — All resolved.
- 🟡 **Safety + Security**: upload/re-verify divergence — Resolved for `tokens`;
  **reopened** for `manifest_path` (below).
- 🔵 Most pass-1 minors (80 columns, `Mapping`/`Iterable` imports in
  `paths.py`/`github.py`/`signing.py`, the `F401` trim, `_LAUNCHER`, docstring
  numbering, insertion point, `mise run` commands, the false markdown criterion)
  — Resolved.

**Partially resolved**

- 🟡 **Security**: the exemption's *stated bar* ("invoked only by hooks, never
  from a SKILL.md") is still unenforced — `if token in exemptions or token in
  bound: continue` means a token whose SKILL.md consumer exists but carries only
  a bare `Bash` or an ancestor glob can be exempted. The plan itself names the
  ten bare-`Bash` Linear/Jira skills as 0170's likely consumers, so the guard
  will fail on the story it exists to serve and the exemption is the path of
  least resistance out. One clause fixes it: raise when `token in exemptions and
  token in invoked`.
- 🟡 **Security**: `BUILTIN_SUBCOMMANDS` is pinned against `is_root_help` — a
  help-*routing* heuristic — rather than the clap `Command` enum, which is the
  dispatch authority. A name added to `is_root_help` alone would be adopted into
  the allowlist and silently exempt invocations the launcher routes to
  `External`. Three lenses independently recommend pinning the enum with
  `is_root_help` as a secondary consistency assertion.
- 🟡 **Safety + Security**: SLSA coverage now derives from `DEBUG_ARCHIVE_DIRS`,
  which cannot see the publish set. Verified: `_release_uploads` uploads
  `manifest.json` and `manifest.minisig`, and no `subject-path` glob covers
  them — the signed manifest, the highest-value artefact in the distribution,
  ships with **no** provenance attestation today, and the new derivation will
  never notice. Deriving from `_release_uploads()` instead would.
- 🔵 **Security**: guard scope. Two evasions remain and both fail *open* for the
  invocation→registration direction: a metacharacter-chained command (`… config
  get x && … vcs status`) yields only the first token, and a path-aliased or
  quoted launcher path (`…/bin/../bin/accelerator`, `"…/bin/accelerator"`)
  yields none — bypassing the lint rule's condition 2 as well.

### New Issues Introduced

Blocking:

- 🔴 **Test Coverage + Correctness + Safety + Security**: `manifest_path: Path =
  RELEASE_MANIFEST` binds at def-time, so migrating `test_github.py:271`'s
  `mocker.patch.object(gh, "RELEASE_MANIFEST", …)` to an argument makes
  `_release_uploads` list the real `dist/release/manifest.json`, the `missing`
  check raises `FileNotFoundError`, and **every** test in
  `TestUploadAndVerifyRelease` fails — including the `assert len(uploads) == 22`
  pin the plan nominates as its own regression evidence. `RELEASE_MANIFEST_SIG`
  (`:272`) has no seam at all, so the migration cannot even be completed. The
  same split leaves the re-verified manifest able to diverge from the published
  one — the exact hole the `tokens` single-resolution rule was added to close.
- 🔴 **Test Coverage + Correctness + Standards**: `lint:dispatch:check` folded
  only into `lint:check` reaches neither `mise run check` nor CI (verified). The
  stated benefit — PR-time failure rather than mid-release — is not delivered,
  Phase 5's "Read-only CI mirror passes" criterion would not exercise it, and
  the checklist would ship the false claim that a missing binding "leaves both
  the release path and CI red". `build-system:check` is the roll-up CI actually
  runs. The plan also names 2 of the 4 files a lint leaf needs
  (`tasks/lint/__init__.py` and `tasks/__init__.py` are missing), and
  "add to whatever aggregate-membership expectation `test_mise.py` pins" has no
  referent — that file pins `_CHECK_GATES` and `_CLI_CHECK_GATES` only.
- 🔴 **Correctness + Test Coverage**: `debug_archive_targets` is specified both
  to validate that every key is in `DISPATCHED_SUBBINARIES` **and** to be tested
  against an injected two-token fixture registry, while *What We're NOT Doing*
  forbids registering fixture tokens. The three requirements are mutually
  exclusive. Giving it `tokens: Iterable[str] = DISPATCHED_SUBBINARIES` resolves
  it and matches the declared one-idiom rule, which this seam currently breaks.

Major:

- 🟡 **Correctness + Safety + Security**: the `bin/`-suffix constraint rests on
  the false gitignore reading corrected above. It blocks a correct author who
  stages elsewhere with a working ignore rule, and passes an author who stages
  into a nested `bin/` whose archives are *not* ignored — encoding a false
  invariant into code, `tasks/README.md` point 12, and a test. Fix the cause
  (`**/bin/*.debug.tar.gz`), assert the property that matters (the path is
  actually ignored), and add `.debug.tar.gz` to `_ARTIFACT_MARKERS`.
- 🟡 **Correctness + Code Quality + Architecture + Safety**: the
  `AssetVerificationError` rationale is wrong. Verified: `_release_reverifies`
  is called at `tasks/github.py:319`, one line **above** the `try` at `:320`, so
  the raise escapes both handlers — no forensic alert, no draft-preserve, no
  cleanup — and by then `_publish` has already committed, tagged and pushed. A
  producer-configuration error determined at sign time leaves a pushed tag and
  an orphaned draft. Hoist the check into the sign stage and use `ManifestError`
  or a dedicated type.
- 🟡 **Test Coverage + Architecture + Code Quality**: Phase 1's "move the
  parsing-primitive cases" is fictional — verified that no direct test of
  `covered_by`, `frontmatter_bash_rules`, `has_bare_bash`, `frontmatter_name`,
  `preprocessor_commands`, `is_plugin_invocation` or `has_metacharacter` exists
  anywhere. Seven functions would be promoted to a shared release-gating
  contract with no direct coverage, while the phase's success criterion passes
  against a near-empty file.
- 🟡 **Test Coverage + Architecture + Code Quality**: Phase 1's "no other
  consumer moves" is false — verified that
  `tests/integration/support/skill_corpus.py:21-26` imports `PLUGIN_PREFIX`,
  `frontmatter_name`, `is_plugin_invocation` and `preprocessor_commands` from
  `tasks.lint.skill_permissions`. After the move `PLUGIN_PREFIX` has no
  remaining in-module use, so re-exporting it is an `F401` failure under
  `select = ["ALL"]`.
- 🟡 **Documentation**: point 3's `[lints] workspace = true` claim is wrong —
  verified that `cli/visualiser/server/Cargo.toml`, the crate point 2 holds up
  as the worked example, has no such key; it declares `[lints.clippy]` with a
  comment noting `-D warnings` from `lint:cli:check` is what promotes warnings.
  The mandatory key is mutually exclusive with what a crate needing local allows
  requires, and the stated failure mode does not exist.
- 🟡 **Architecture + Standards**: injecting `tokens`/`manifest_path` through
  `upload_and_verify_release` — verified an `@task` — turns test seams into
  operator-facing CLI flags on the one task that publishes signed assets, and
  inverts the pure-helper convention Phase 3 itself introduces.
- 🟡 **Architecture + Test Coverage**: Phase 5's claim to be "mergeable
  independently of Phase 2" is false — its cross-reference test asserts
  `SKILL_EXEMPT_SUBBINARIES`, `DEBUG_ARCHIVE_DIRS` and `BUILTIN_SUBCOMMANDS`
  resolve, all created by Phases 2 and 3. The real graph is 1→2, 3→4, {2,3}→5.
- 🟡 **Documentation + Safety + Security**: the same-change rule still names only
  points 1 and 7, but points 3, 4 and 8 are equally release-breaking and caught
  by nothing before the release job. Reserved-token and built-in-collision
  constraints also remain prose-only, at a guard that already resolves all three
  registries together — `verify` would cause the **verify shim** to be signed and
  advertised under a dispatch token.

Minor:

- 🔵 **Correctness + Test Coverage**: under segment equality the
  `not covered_by(BARE_LAUNCHER, rule)` conjunct is unreachable except where the
  token is a prefix of `zz-external-subcommand-zz` — where it is a false
  negative. No matrix case discriminates it, so the acceptance criterion for
  condition 2 is verified by inspection only, and the decision table's "both
  close opposite halves" is not established.
- 🔵 **Security**: a skill carrying both a scoped rule and an ancestor glob still
  binds — the bare-`Bash` veto is skill-level but the ancestor-glob veto is
  per-rule.
- 🔵 **Code Quality + Architecture**: the launcher prefix is now duplicated —
  `BARE_LAUNCHER` hardcodes it in `skill_parsing.py` while `dispatch.py`
  re-derives `_LAUNCHER` from `PLUGIN_PREFIX`, which is exactly the
  desynchronisation the constant's own comment warns against. `_launcher_token`
  is a parsing primitive and belongs in the leaf.
- 🔵 **Code Quality**: `debug_archive_path`'s `bin_dir: Path = BIN_DIR` default is
  only correct for the visualiser; `debug_archive_path("vcs", platform)` silently
  files under the visualiser's tree and still passes the `bin/` check.
  `cli_member_manifests` is the closer precedent (required, no default).
- 🔵 **Code Quality**: the entry point is now ~70 lines mixing three validation
  raises, both directions and message construction; the house shape is a pure
  `violations(...) -> list[str]` plus a thin raising wrapper, and `"; ".join`
  produces a run-on line where the house idiom is `"\n  ".join`.
- 🔵 **Standards**: `Mapping` is used in `tasks/build.py` without an import (named
  for three other files but not this one); `MappingProxyType` is a third registry
  idiom beside `dict` and `frozenset`; `debug_archive_targets` is public while its
  three sibling seams are private; `collect_entries`' parameter is still
  `subbinaries` against the new `tokens`; `tasks/lint/dispatch.py` is terser than
  every sibling lint leaf and collides in name with `tasks/shared/dispatch.py`.
- 🔵 **Documentation**: no section intro, so `token` and "dispatched sub-binary"
  are undefined at the anchor readers arrive by; the cross-reference test covers
  fewer points than claimed; the `launcher` cache-collision reason is wrong (the
  crate-shadowing reason is sound); point 8 mandates three wirings then says the
  simplest route needs none; no `cli/deny.toml` point despite `deny:check` being
  a gate CI *does* run; no pointer from `tasks/CLAUDE.md`; point 7 omits the
  `EXPECTED_INJECTION_SKILLS` bump a new consuming skill triggers.
- 🔵 **Safety**: `DEBUG_ARCHIVE_DIRS` has no anti-vacuity pin, and the SLSA
  derivation iterating it becomes a silent no-op if it is emptied.

### Assessment

The core mechanism is now right, and that was the hardest part — the permission
check was wrong in both directions before, and is verified correct in both now.
The layering fix is also genuinely better than the workaround it replaced.

But the revision moved too fast on its periphery. Three defects block a phase
from landing green, and one encodes a false invariant into shipped code and
docs — worse than the prose it replaced, because a test will now assert it. The
common thread is that the additive hardening was written from reasoning rather
than from checking: the CI wiring, the gitignore semantics, the `try`-block
placement, the existence of parsing tests, the third `skill_permissions`
consumer, and the visualiser crate's `[lints]` table are all facts that a single
command each would have settled, and all six went the other way.

The plan needs a third pass focused narrowly on those verifiable facts, plus the
four partially-resolved items (exemption bar, built-in pin authority, SLSA
coverage source, guard scope). It should not need another redesign.

---

## Re-Review (Pass 3) — 2026-08-03

**Verdict:** REVISE

All eight lenses re-ran. The **guard is now right** — the permission check, the
layering, the registry bounds, the CI wiring and the checklist's factual claims
all verified correct, and pass 2's three blockers are resolved. But pass 3
introduced a fresh set of blockers of its own, concentrated in the two
mechanisms it *added* rather than the ones it fixed: the `emit_manifest`
set-equality check and the `create_debug_archives` task seam. Both are
self-contradictory against the plan's own stated rules.

### Verified this pass

Seven claims checked with commands, all confirming the reviewers:

- `tasks/release.py:_sign` calls `emit_manifest(…, manifest.collect_entries(), key)`
  — both sides default to `DISPATCHED_SUBBINARIES`, so the new set-equality
  check **compares a constant with itself** in production.
- `tests/unit/tasks/test_manifest.py` passes `{}`, `{}` and
  `collect_entries(["foo"], …)` to `emit_manifest` — all three committed
  `TestEmitManifest` tests would raise `ManifestError`.
- `build:debug-archives` is a real mise leaf (`mise.toml:135-138`) running
  `invoke build.create-debug-archives`, so `@task` parameters leak `--dirs` and
  `--staging-dir` onto a release-path task.
- `tasks/build.py` imports **no** `collections.abc` names, so both `Mapping`
  *and* `Iterable` are undefined (the plan names only `Mapping`).
- `cli_cross_compile` stages via `cli_binary_path(name, platform)` =
  `dist/release/<name>-<platform>`, so a `_CLI_RELEASE_BINARIES` entry must be
  literally `accelerator-<token>`.
- `test_github.py:403-433` asserts on **upload** strings from `_release_uploads`
  through the `@task`, so migrating its `DISPATCHED_SUBBINARIES` patch to an
  argument on `_release_reverifies` cannot preserve it.
- `covered_by(BARE_LAUNCHER, "…/accelerator [a-y]*")` is **`False`** (and `v*`
  likewise), so the new skill-level ancestor-glob veto cannot see them.

### Previously Identified Issues

**Resolved**

- 🔴 `manifest_path` def-time default breaking `TestUploadAndVerifyRelease` —
  Resolved; the parameter is gone and the manifest globals stay call-time.
- 🔴 Lint wiring unreachable from CI — Resolved. Verified `build-system:check` is
  run by `main.yml:181` and is a `needs` of the `prerelease` job; all four
  registration files are named; `test_mise.py` gets a concrete gate list.
- 🔴 `debug_archive_targets` membership-vs-fixture contradiction — Resolved by
  the `tokens` parameter (but see the new `@task` finding, which reintroduces it
  one level up).
- 🟡 The false `.gitignore` invariant — Resolved. The plan now fixes the pattern
  (`**/bin/*.debug.tar.gz`), corrects `:28`'s stale comment, and adds the
  `_ARTIFACT_MARKERS` backstop.
- 🟡 Phase 1's fictional test move; the missing `skill_corpus.py` consumer;
  point 3's `[lints]` claim; the `launcher` cache-collision claim; Phase 5's
  false independence; `upload_and_verify_release` CLI surface; the 4-file lint
  registration — all Resolved.
- 🟡 SLSA symmetry-not-coverage — Resolved, and better than asked: coverage now
  derives from `_release_uploads()`, and the workflow gains the manifest paths.

**Partially resolved**

- 🟡 **Security**: the exempt-but-invoked raise closes the exemption bar, but
  both fail-closed directions are prefix-anchored, so a launcher call chained
  mid-command (`cd . && ${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs status`)
  escapes `is_plugin_invocation` and is invisible to the guard *and* to
  `skill_permissions`. The `has_metacharacter` → `continue` also drops the token
  from `invoked`, not just from `bound`.
- 🟡 **Security**: `RESERVED_SUBBINARIES` is a hardcoded literal where
  `BUILTIN_SUBCOMMANDS` is derived. `verify` is unsafe *because*
  `_CLI_RELEASE_BINARIES` stages `accelerator-verify-*`; a future entry there
  creates a new unsafe token silently.

### New Issues Introduced

Blocking:

- 🔴 **`emit_manifest`'s set-equality check is a tautology that reddens three
  committed tests.** In production both operands derive from the same constant,
  so it can never fire; in tests all three `TestEmitManifest` cases now raise.
  Flagged by five lenses. The divergence actually worth guarding — manifest keys
  vs the set `_release_uploads` publishes — remains unguarded, and Phase 3
  already has the machinery (`_release_uploads()`) to check it.
- 🔴 **`create_debug_archives`' `@task` parameters contradict Phase 4's own
  rule** — the rule the plan states as load-bearing and manually verifies for
  `upload_and_verify_release`. They also leak `--dirs` (typed `Mapping[str, Path]`,
  unexpressible on a CLI) onto a release-path leaf, and because `tokens` is *not*
  forwarded, `_debug_archive_targets`' own validation rejects every fixture
  registry the phase's end-to-end test must inject.
- 🔴 **`Iterable` is undefined in `tasks/build.py`** — `F821` plus pyrefly, so
  Phase 3 fails its own `build-system:check` criterion. `DISPATCHED_SUBBINARIES`
  is also order-dependent between Phases 2 and 3, which the declared graph makes
  freely orderable.

Major:

- 🟡 **Phase 3's `_release_uploads` snippet uses `debug_dirs`, which Phase 4
  introduces** — a `NameError` at import if Phase 3 lands alone, contradicting
  the "each phase lands green" contract for two phases the graph says are
  freely orderable.
- 🟡 **The `test_github.py:427` migration breaks the test it migrates.** That
  test asserts on upload strings produced through the `@task`; with the token
  seam deliberately kept off the `@task`, the module patch is the *correct*
  mechanism there. Migrating it either reddens the suite or silently deletes the
  only end-to-end evidence that a registered token reaches a real
  `gh release upload … --clobber`.
- 🟡 **The skill-level ancestor-glob veto is blind to `v*` and `[a-y]*`.**
  Verified. A skill carrying a correctly scoped rule *alongside*
  `Bash(…/accelerator [a-y]*)` has `broad == False`, so the guard certifies the
  token cleanly bound while the skill pre-authorises every token not starting
  with `z`. The veto should derive from `launcher_token(rule)` failing the token
  charset, not from the `BARE_LAUNCHER` sentinel.
- 🟡 **`_subbinary_reverifies`' new empty-collection raise sits at the exact
  call site the plan condemns** (`tasks/github.py:319`, outside the `try` at
  `:320`, after commit/tag/push/`create_release`) and uses
  `AssetVerificationError`, whose meaning the plan itself defines two sections
  later as "a published-candidate asset failed its check → preserve the draft".
  It also restates the anti-vacuity message `_registry_problems` already owns.
- 🟡 **No test proves the new lint leaf fails when the invariant is broken.**
  Only the green path is asserted. The house precedent exists
  (`test_claude_coupling.py:144`, `test_lint.py:43` — `pytest.raises(Exit)`).
  This is the gate the plan argues for at length.
- 🟡 **The `.gitignore` fix and `_ARTIFACT_MARKERS` backstop land with no test**,
  and the backstop is defeated by `git status --porcelain`'s default collapsing
  of a wholly-untracked directory to one line — so `.debug.tar.gz` never appears
  in the scanned text in exactly the scenario it is written for. The repo already
  has the idiom (`test_bootstrap_coverage.py:79-92` uses
  `pathspec.GitIgnoreSpec`).
- 🟡 **No matrix case discriminates the `covered_by(command, rule)` conjunct**,
  and none exercises the acceptance criterion's "both conditions on a *single*
  rule" quantifier — a mutant that drops coverage entirely, and one that splits
  the conjunction across rules, both survive all 30 rows. One case fixes both: a
  skill whose only rule is `… <token> start` invoking `… <token> status`.
- 🟡 **The chained-command matrix row does not discriminate the metacharacter
  skip** — it puts `config` first, so `launcher_token` yields a built-in either
  way and the message is identical with and without the guard.
- 🟡 **Checklist point 8 never says `accelerator-<token>`.** Verified: the
  `_CLI_RELEASE_BINARIES` route stages `dist/release/<name>-<platform>`, so a
  bare token produces the wrong asset name and a `SigningError` deep in the
  release job. Point 3 also omits `[[bin]] name = "accelerator-<token>"`.
- 🟡 **Points 9 and 12 give contradictory triggers for the same action.** Point 9
  says "only when the sub-binary is staged outside `dist/release/`" — which never
  happens; it is the *debug archive* that lands in a committed tree, as point 12
  correctly states.
- 🟡 **The *enforced* markers do not match the definition the lead-in gives
  them.** Points 2, 3, 4 and 8 all fail the release when missed and are unmarked;
  point 13 states its enforcement in prose instead of using the marker; point 12
  is marked though omitting the entry fails nothing (what is enforced is the
  *shape* of an entry you do add).
- 🟡 **The checklist omits the two literal registry pins Phases 3 and 4 add** —
  so the first sibling author gets a red unit run from a test naming neither
  their token nor any registration point, on a checklist claiming to be complete.

Minor (selected):

- 🔵 `_authorises`' third conjunct is now **fully** dead, not just near-dead: the
  skill-level veto guarantees no rule covers `BARE_LAUNCHER` whenever
  `_authorises` is evaluated. The plan's "three conditions" prose and three
  defending paragraphs describe a branch that cannot fire.
- 🔵 The Overview still says **"twelve-point"** where everything else says
  thirteen — an edit that silently failed to apply. The Testing Strategy says
  "29-case" for a 30-row table.
- 🔵 The verb predicate does not match any drafted item: all thirteen begin
  `**Add**`/`**Register**`/… with bold emphasis, which
  `startswith(verb)` rejects. `Stage` is in the closed set and unused.
- 🔵 The cross-reference list pins `is_root_help`, which the section no longer
  names, and omits `cli/launcher/src/launch/inbound/cli.rs` — the one source
  point 10 actually attributes authority to.
- 🔵 `dispatch_violations` diverges from the house `violations(root)` name and
  required-root signature; the module/leaf/test names are `dispatch` /
  `dispatch-coherence` / `dispatch` where every sibling is single-named.
- 🔵 `ValueError` is the only one in `tasks/build.py`, which uses `RuntimeError`
  uniformly for the same class; three error idioms now cover "a release registry
  constant is malformed".
- 🔵 `build-system:check` gains a fourth non-Python member with no update to its
  description or the `tasks/README.md` row that names what it folds.
- 🔵 Point 4 omits the regenerated `cli/Cargo.lock` (`lint:cli:check` runs
  `--locked`, and CI runs it per PR).
- 🔵 Attestation-before-publication ordering is still unpinned; the SLSA test
  asserts coverage but not step order.
- 🔵 Six absence-style source scans have no positive control, so a
  non-matching regex reads as cleanliness.

### Assessment

The guard is finished. Everything I verified about it this pass — the probe, the
segment check, the layering, the registry bounds, the CI reachability, the real
tree passing — holds. Pass 3 also fixed every pass-2 blocker.

The problem is that pass 3 added two mechanisms that do not pay their way and
contradict the plan's own rules: an `emit_manifest` check that cannot fire where
it matters and breaks three tests where it does, and a `@task` seam that violates
the injection rule stated one phase later. Both were added on reasoning about
what *ought* to be guarded rather than by checking what the production call sites
actually do — the same failure mode as pass 2's six false facts, one level up.

Pass 4 should be **subtractive**, not additive. Concretely: drop the
`emit_manifest` equality (or re-aim it at `_release_uploads()`), drop the
`create_debug_archives` parameters, drop the `_subbinary_reverifies` raise, drop
the dead `BARE_LAUNCHER` conjunct, and leave `test_github.py:427` alone. Then fix
the mechanical omissions (`Iterable`, `debug_dirs` phase attribution, the verb
predicate, the Overview count, point 8's binary name, points 9/12, the *enforced*
markers) and add the four cheap tests the reviewers identified (lint-task failure
path, gitignore semantics, the coverage-conjunct matrix case, the chained-command
row reordering). The veto blind spot and the chained-invocation hole are the only
two design changes worth making.

---

## Re-Review (Pass 4) — 2026-08-03

**Verdict:** REVISE — and the loop should stop here.

All eight lenses reported. Every finding
below was verified against the tree or by tracing the plan's own snippets before
being recorded. Two lenses reached the `RESERVED_TOKENS` deadlock independently,
and three reached the `_every_token` branch defect independently.

The subtractive pass did what it set out to do: all five removals are verified
**safe**, and two verified as improvements. Dropping the `BARE_LAUNCHER`
conjunct admits nothing (every shape escaping the veto yields an empty token
segment, so it fails closed as *unbound*); keeping `if not names: return []`
avoids a raise outside the draft-preserve envelope; removing the `emit_manifest`
check removes a tautology. The two-part veto is verified complete over the
canonical rule space.

But the pass introduced **three criticals and eight majors of its own**, several
of them direct self-contradictions between things written in the same document.

### Self-contradictions introduced this pass

- **The `Bash(` source scan reddens against the guard it guards.** The scan
  (`:944`) forbids the literal `Bash(` in `dispatch_coherence.py`; the guard's
  own error messages contain `Bash(...)` (`:719`, `:731`), and the case matrix
  asserts on that exact substring (`:913`).
- **`:427` is unfixable as specified.** The plan keeps the module patch
  (`:1569`), uses def-time `tokens=DISPATCHED_SUBBINARIES` defaults, and bans
  call-time sentinels (`:1622`). Def-time defaults cannot be reached by a module
  patch — the mechanism the plan itself documents for `RELEASE_MANIFEST` one
  paragraph earlier. Pass 3 said migrating the patch breaks the test; pass 4 said
  keeping it works. Both are wrong: it breaks either way.
- **`RESERVED_TOKENS` deadlocks the registration path.** Checklist point 8 tells
  every author to add `accelerator-<token>` to `_CLI_RELEASE_BINARIES`; the
  derivation then reserves that very token, so `_registry_problems` rejects it.
  The first sibling story cannot register. Found independently by the security and
  documentation lenses. The fix both propose is the same — reserve only staged
  names that are *not* dispatched — which is a one-line change, but it is the
  third consecutive pass in which a newly-added mechanical check turned out to
  contradict a mechanism elsewhere in the same document.
- **`_every_token` broke the diagnostic it was added beside.** It populates
  `invoked` before the metacharacter guard, so a chained command takes the wrong
  branch and the matrix row asserting `"no skill invokes"` is unwritable.
- **`violations` checks `bound` before `exempt`**, so an exempt + invoked + bound
  token reports nothing — contradicting the docstring and decisions table in the
  same file.
- **Point 12 carries two audience tags** where the test asserts exactly one;
  **`Stage`** is in the README lead-in (`:1668`) but not the test's verb set
  (`:1818`).

### Mechanical defects that fail on first run

- `_publish` references `json` and `DISPATCHED_SUBBINARIES`, neither in scope in
  `tasks/release.py` — verified the imports.
- `_launcher_occurrences` needs `LAUNCHER`; the import block lists eight names
  and the scan pins "all eight".
- The `_publish` read breaks three committed tests
  (`test_release.py:176`, `:182`, `:215`), none listed — the same audit the plan
  performs for the *rejected* `emit_manifest` placement.
- New tests filed at `tests/unit/tasks/test_release.py`, which does not exist.
- The `_SUBBINARY_MANIFESTS` → `MappingProxyType` conversion fails
  `types:build-system:check`: it is declared `dict[str, Path]` and
  `MappingProxyType` is not a `dict` subclass; `tasks/manifest.py` imports
  nothing from `types`.
- `DEBUG_ARCHIVE_DIRS` is used in `tasks/github.py` without being imported there.
- Checklist point 1 names one `test_github.py` pin, but an added token also
  breaks `assert len(uploads) == 22` (`:326`, which reads the real registry) and
  `_setup_release`'s single-token fixture and manifest (`:251-307`).

### Reasoning defects

- **The `_publish` check cannot detect the stale cut it is justified by.**
  Verified `dist/release/` is never cleaned, so a stale tree survives — but it
  carries the *same* token set and a different version, and the check compares
  only key sets.
- **The `emit_manifest` fail-fast rationale is wrong.** `_sign` calls
  `sign_staged_binaries(key)` **before** `emit_manifest`, so the guard still
  fires after the signing secret is used.
- **Phase 5 forward-references a pin Phase 4 creates**, so the graph is
  `{2,3,4}→5`, not `{2,3}→5`.
- **The `RuntimeError` rationale miscounts its own evidence** — the plan says
  `tasks/build.py` has "thirteen non-typed precondition sites"; it has nine. The
  conclusion holds, but a locked decision cites a figure that was not checked.

### Assessment

The guard's core design has been correct since pass 3 and is verified so again.
Everything since has been peripheral, and the peripheral work is not converging:

| Pass | Blockers found | Introduced by the previous pass |
|---|---|---|
| 1 | 1 critical, 20 major | — |
| 2 | 3 blockers | 3 of 3 |
| 3 | 3 blockers | 3 of 3 |
| 4 | 3 criticals, 8 majors | ~all |

Four consecutive passes have fixed the prior pass's defects and introduced a
comparable number. The plan is now 2182 lines — larger than the code it
describes — and the defect density is concentrated in exactly the parts that
grew: cross-referenced test specifications, a thirteen-point checklist with a
per-item tag assertion, and source scans that constrain the very code they sit
beside. Each addition creates new opportunities for the document to contradict
itself, and it now does so in six places.

This is a process failure, not a plan failure. Continuing to revise-and-review
is very unlikely to converge, because the mechanism generating the defects is the
revision rate itself.

**Recommendation.** Stop revising. Two viable routes:

1. **Implement Phases 1–2 now.** The guard design is verified correct and its
   defects are all mechanical and locally fixable at the keyboard, where a type
   checker and a test run give feedback in seconds rather than a 200k-token
   review cycle. Let the implementation settle Phases 3–5.
2. **Cut the plan back to its verified core** — the parsing extraction, the
   guard, the lint wiring — and drop the checklist, the SLSA work and the
   release-path checks to sibling work items. Those three are where every
   self-contradiction landed.

Either way, do not run a fifth revise pass against this document.

---

## Verdict: APPROVE — set by the reviewer, 2026-08-03

The reviewer set this verdict over the pass-4 recommendation of REVISE. The
findings and pass history above are the record of what the lenses found and are
left unchanged; this note records what was fixed afterwards and what was
knowingly accepted.

### Fixed after pass 4

All mechanical defects and every self-contradiction pass 4 identified:

- Missing imports named with their reason — `json` + `DISPATCHED_SUBBINARIES`
  (`tasks/release.py`), `LAUNCHER` (guard, scan corrected to nine names),
  `DEBUG_ARCHIVE_DIRS` (`tasks/github.py`), `MappingProxyType` **and** the
  `Mapping[str, Path]` re-annotation (`tasks/manifest.py`).
- The `Bash(` source scan scoped to `re.compile` arguments, so it no longer
  reddens against the guard's own error messages.
- `RESERVED_TOKENS` derived from `_CLI_RELEASE_BINARIES` **minus** the dispatched
  set, removing the deadlock with checklist point 8.
- `violations` checks the exemption before the `bound` short-circuit.
- The metacharacter matrix row expects the message the code actually produces.
- `_publish` gained the **version** comparison (the token-set check alone cannot
  see a stale cut), an existence check, and extraction to a named helper; the
  three committed tests it breaks are listed with their fix.
- `:427` resolved by having the `@task` resolve the token collection once and
  thread it — this preserves the module patch, gives resolve-once, and keeps one
  injection idiom with no sentinel.
- Phase graph `{2,3,4}→5`; test paths corrected to
  `tests/integration/tasks/test_release.py`; the lint-leaf test file renamed;
  `validate_dispatch_coherence` made keyword-only; `Stage` dropped; checklist
  points 1, 4, 9 and 12 corrected; every item audited to exactly one tag; the
  `emit_manifest` fail-fast claim withdrawn (`_sign` signs first).

### Knowingly accepted, not fixed

These are carried into implementation rather than resolved on paper:

- **`_TOKEN` vs `derive_override_var`** genuinely disagree on `Vcs` — the Rust
  rule accepts uppercase. The pin as specified asserts agreement and will fail;
  it needs restating as one-directional containment.
- **A wildcarded token segment disqualifies a witness but is never reported.**
  `Bash(…/accelerator v*)` in a skill that only invokes `config` is caught by
  nothing. The fix belongs in `skill_permissions`, outside this plan's scope.
- **`_every_token` can register a launcher path inside a quoted argument**,
  which fails closed but can block a release for a non-defect.
- **`_launcher_occurrences` placement** — arguably belongs in the parsing leaf
  as one `launcher_tokens()`.
- **Architecture's structural items** — hoisting the guard out of
  `emit_manifest`, a stated admission rule for `tasks/shared/`, registry
  co-location.

The first two are the ones most likely to bite: one fails a test as written, the
other is a real permission-scoping gap. Both are cheap, and both are better
settled against a type checker and a test run than against another review pass.

---
*Review generated by /accelerator:review-plan*
