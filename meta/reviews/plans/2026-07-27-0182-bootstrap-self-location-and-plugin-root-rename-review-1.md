---
type: "plan-review"
id: "2026-07-27-0182-bootstrap-self-location-and-plugin-root-rename-review-1"
title: "Plan Review: Bootstrap Self-Location and the ACCELERATOR_PLUGIN_ROOT Rename"
date: "2026-07-27T10:43:33+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-07-27-0182-bootstrap-self-location-and-plugin-root-rename"
relates_to: ["work-item:0182"]
reviewer: "Toby Clemson"
verdict: "REVISE"
lenses: ["correctness", "portability", "test-coverage", "architecture", "compatibility", "security", "code-quality", "standards"]
review_number: 1
review_pass: 4
tags: ["plan-review", "cli", "launcher", "bootstrap", "plugin-root", "hooks", "lint-guards"]
last_updated: "2026-07-27T21:33:30+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Bootstrap Self-Location and the `ACCELERATOR_PLUGIN_ROOT` Rename

**Verdict:** REVISE

This is an exceptionally well-grounded plan — it measures rather than assumes,
reuses a previously reviewed symlink-chase specification, restates three
acceptance criteria that measurement showed to be unsatisfiable, names the
falsifying assertion for nearly every change, and correctly identifies both the
vacuous tests it inherits and the two tests that assert the bug. Phases 0–3 need
only sharpening. Phase 5, however, rests on a mechanism that does not exist:
a compose-time plugin-root error is propagated by `dispatch` before any
`--fail-safe` policy is consulted, so the phase's own success criterion is
unsatisfiable, three of its four claimed degradation removals do not follow from
the change, and it would redden the launcher's largest test suite. Separately,
neither Phase 1's headline local verification command nor Phase 4's 204-command
harness states how the bootstrap they invoke obtains a launcher — as written both
fall through to a real `curl` against an unpublished GitHub release and write an
untracked binary into the shipped `bin/`, which is precisely the hazard the plan
cites when deleting two entrypoint tests.

### Cross-Cutting Themes

- **Phase 5's error path is specified against a code path that isn't there**
  (flagged by: correctness, architecture, code-quality) — three lenses
  independently traced `compose_config()?` in `cli/launcher/src/launch/mod.rs:189`
  and found that `--fail-safe` is applied only by `finish` in
  `config_command/inbound/cli.rs:452-471`, after a `ConfigStack` exists. Add to
  that: no `ConfigError` variant can carry "missing installation root" with the
  promised `Failure::Read` classification, and the requirement is placed at the
  one composition point shared by all nine `config` families when the plan's own
  measurements show only the template family needs it. The phase needs
  re-designing around per-capability requirement, not re-wording.
- **No phase states how the bootstrap under test gets a launcher** (flagged by:
  correctness, test-coverage, portability, compatibility) — four lenses reached
  the same conclusion from different directions. `.claude-plugin/plugin.json` is
  at `1.24.0-pre.16` and the closure sequence publishes *after* merge, so the
  fetch 404s; a version that did exist would supply the pre-rename launcher,
  which cannot read the new variable. The dev override is unusable (its
  containment gate needs the repo root, and the marker there is forbidden by an
  existing assertion). This affects a Phase 1 automated criterion, the whole of
  Phase 4, and the plan's claim that the version-keyed cache makes lockstep safe.
- **The symlink hop counter is unreachable, so its test cannot pass** (flagged
  by: correctness, portability, test-coverage) — the cited prior-art conclusion
  that `bash "$CYCLE_A"` reaches the in-script counter is wrong: `bash` must
  `open()` that path and gets `ELOOP` too. More fundamentally, any path bash can
  open has already had its chain resolved by the kernel within `SYMLOOP_MAX`, so
  a bound of 32 can only fire on Linux for a *non-cyclic* 33–40-hop chain — it
  rejects chains the host kernel accepts and never detects a cycle.
- **`--fail-safe` makes integrity failures silent** (flagged by: security,
  correctness) — consuming the flag inside `fail()` covers all 14 gates including
  missing verify shim, missing public key, and `fetch_and_verify` failure. At the
  204 `!` sites stderr is invisible, and ~86 of those commands legitimately emit
  nothing, so signature-verification failure becomes observationally identical to
  benign empty output. This also contradicts the plan's own "the 20
  `config template` sites stay loud" claim, which holds only for launcher-layer
  refusals.
- **The promoted `sources()` walk loses caller-specific pruning** (flagged by:
  correctness, code-quality, architecture, test-coverage, standards) — the
  proposed `(root, suffixes, subtree)` signature cannot express `.venv` (pruned
  explicitly because it is *not* in `.gitignore`) or `_keep`'s `workspaces/`
  rule, and `_ignore_spec()` honours only the root `.gitignore`, so the new
  guard will descend into `cli/visualiser/frontend/dist/` — ignored only by a
  nested file. No existing assertion would catch either loss.
- **Phase 2's guard is scoped where the bug wasn't** (flagged by: architecture,
  code-quality, standards, correctness) — the docstring is to state "no plugin
  entry point may require `CLAUDE_PLUGIN_ROOT` from its process environment", but
  the entry point that violated it is `bin/accelerator`, a shell script outside
  `cli/` with no extension, and the suffix tuple is deny-by-omission against an
  acceptance criterion phrased as "no tracked source file under `cli/`".
- **Symlink-writing mechanics are unspecified in both the bootstrap and the
  hook** (flagged by: security, portability, correctness, test-coverage) — `ln`
  flags, non-symlink destinations, atomicity, `mkdir -p`, and directory modes are
  all absent; `ln -sf` onto a symlink-to-a-directory writes *inside* it, which
  would defeat the containment property the snapshot-diff test is meant to
  establish.
- **Phase independence is overstated** (flagged by: architecture) — the plan
  claims six independently mergeable phases and then admits Phase 2 must follow
  Phase 1; in fact Phases 2–5 all depend on Phase 1, a ~25-file hub spanning four
  toolchains. This bears directly on the project constraint that phases be
  independently integratable.

### Tradeoff Analysis

- **Fail-safe availability vs integrity visibility**: the plan's blanket
  degradation is right for configuration gates (a `!` site must not discard the
  prompt) and wrong for integrity gates (a tampering attempt must not be
  invisible). Security proposes mirroring the launcher's own two-class model
  (`Failure::Read` degrades, `Failure::Refusal` never does). Recommendation:
  adopt the split — it costs one extra variable and directly preserves R8's
  purpose while keeping the file's stated fail-closed contract honest.
- **Hard requirement at the boundary vs per-capability requirement**:
  architecture, code-quality and correctness all argue Phase 5's placement at
  `compose_stack` *widens* the blast radius from one family to eight, contrary to
  its stated goal. Usability of a single choke point loses to the plan's own
  measurement here. Recommendation: require the root in the four template/
  plugin-content consumers.
- **Conservative hop bound (32) vs never rejecting a host-legal chain (40)**:
  the plan picks the lower bound as "conservative", but the only reachable
  behaviour is rejecting Linux-legal 33–40-hop chains. Recommendation: either
  lower it well below both kernels (e.g. 16, making it testable with a
  non-cyclic chain) or raise it to 40 and label it unreachable-for-cycles
  termination insurance.
- **Real-launcher fidelity vs fast, direct assertions**: six of the eight new
  Phase 1 tests only need to know *which root was derived*; routing them through
  a signed multi-tens-of-MB debug binary makes each failure surface as a missing
  sentinel in rendered table output. Recommendation: keep `real_launcher` for the
  two rendering tests and extend the existing stub to dump its environment for
  the rest — which also asserts R2's export directly rather than transitively.
- **`meta/`-wide purge criterion vs a discriminating check**: standards and
  compatibility both note the purge grep cannot pass. Recommendation: scope it to
  non-`meta/` tracked files and add the entrypoint suite to the permitted set
  with its reason, so a genuine miss cannot be waved through as known residue.

### Findings

#### Critical

- 🔴 **Correctness + Architecture**: Phase 5's `--fail-safe` degradation path does
  not exist
  **Location**: Phase 5, §1 Make the boundary require a root
  `dispatch` calls `let stack = compose_config()?;`
  (`cli/launcher/src/launch/mod.rs:189`) and converts `ConfigError` straight into
  `kernel::Error`; `--fail-safe` is a per-action flag applied only by `finish`
  (`config_command/inbound/cli.rs:452-471`) once a stack exists. A missing root
  would exit non-zero for every `config` command with *or* without the flag, so
  the phase's criterion ("the same command **with** `--fail-safe` exits 0 with
  empty stdout") is unsatisfiable — and the change reinstates at the launcher
  layer exactly the prompt-discarding mode Phase 1 removes at the bootstrap
  layer.

- 🔴 **Correctness**: Phase 5 misreads `config_read.rs:62` as one test case; it is
  the suite-wide runner
  **Location**: Phase 5, §3 Update the tests
  `config_read.rs:62` sits inside `run_in` (`:58-67`), the shared runner used by
  ~150 invocation sites across the file's 133 tests, and it does
  `env_remove("CLAUDE_PLUGIN_ROOT")` unconditionally. Only the template tests use
  `run_with_plugin` (`:1196-1208`). With the root mandatory at compose time,
  every `get`/`path`/`agents`/`paths`/`work`/`review`/`summary`/`context`/
  `instructions`/`dump` test fails before its handler runs, so
  `mise run test:unit:cli` — the phase's own criterion — cannot pass.

- 🔴 **Correctness + Test Coverage + Portability + Compatibility**: no phase states
  how the bootstrap under test obtains a launcher, so Phase 1's verification
  command and Phase 4's harness fetch over the network
  **Location**: Phase 1 Success Criteria (`env -u … ./bin/accelerator config
  templates list`); Phase 4, §2 The harness
  Run from the repo, the self-locating bootstrap resolves `<repo>` as the root,
  satisfies the `plugin.json` / shim / key / cache gates (all four
  `bin/accelerator-verify-*` triples are committed), finds no cached launcher for
  `1.24.0-pre.16`, and `curl`s the real GitHub release — which does not exist for
  an unpublished version, and which if it did exist would supply the *pre-rename*
  launcher that cannot read `ACCELERATOR_PLUGIN_ROOT`. It also writes an
  untracked `bin/accelerator-launcher-*` plus `.minisig` into the shipped `bin/`
  (not gitignored). The dev override cannot substitute: its containment gate
  requires the plugin root to *be* the repo root, and creating
  `.accelerator-dev-launcher` there is forbidden by
  `test_accelerator_entrypoint.py:826` (and would flake it under parallel
  `mise run`).

#### Major

- 🟡 **Correctness + Test Coverage + Portability**: the symlink-cycle test cannot
  reach the in-script hop counter, and the counter is unreachable by construction
  **Location**: Phase 1 §1 (`test_symlink_cycle_fails_with_a_named_error`); Phase
  1 Manual Verification; Testing Strategy → Key Edge Cases
  `bash "$CYCLE_A"` must `open()` that path and the kernel returns `ELOOP` there
  exactly as for `execve()`, so bash aborts with its own error and
  `accelerator: symlink loop resolving …` is never printed. Any path bash *can*
  open has had its whole chain resolved within `SYMLOOP_MAX`, so a bound of 32 can
  only fire on Linux for a non-cyclic 33–40-hop chain.

- 🟡 **Security + Correctness**: `--fail-safe` turns signature-verification failure
  into a silent exit 0, indistinguishable from benign empty output
  **Location**: Phase 1 §2 (the fail-safe-aware `fail()`); Phase 4 assertion table
  Five of the 14 remaining gates are integrity gates — missing verify shim
  (`:72`), missing public key (`:74`), unhashable/un-stageable/un-chmod-able shim
  (`:163,:168,:170`), refused dev launcher (`:134`), and `fetch_and_verify`
  failure (`:254`), where a bad minisign signature lands. Nothing unverified is
  ever exec'd, so this is not an integrity bypass — it is a detection and audit
  failure plus an availability lever: all 43 skills silently inject *empty*
  context and the model proceeds believing it has configuration it does not. It
  also falsifies the plan's "the 20 `config template` sites stay loud" claim,
  which holds only for launcher-layer refusals.

- 🟡 **Security + Correctness**: unchecked `readlink`/`cd` and an external `dirname`
  can silently resolve the cwd — or the wrong level — as the trust root
  **Location**: Phase 1 §2 (the symlink chase)
  The loop runs under `set -uo pipefail` *without* `-e`, so a failed `readlink`
  (a real TOCTOU window, since the Phase 3 hook rewrites the middle hop every
  SessionStart) leaves `self` as `"<dir>/"` and `dirname` then returns the
  grandparent. Worse, `dirname` receiving an argument beginning with `-` (taken
  verbatim from `readlink` output) errors and prints nothing, whereupon `cd ""`
  **succeeds as a no-op in bash** and `pwd -P` yields the current working
  directory — for a skill-invoked bootstrap, the user's project. A cloned repo
  carrying `.claude-plugin/plugin.json`, `keys/accelerator-release.pub` and
  `bin/accelerator-verify-<platform>` would become the trust anchor. `CDPATH` is
  a second route to the same outcome, and can also inject a printed directory
  into the command substitution, corrupting `plugin_root` into two lines.

- 🟡 **Security**: the release public key and verify shim are read from the
  runtime-derived root, so relocating the entry point relocates the trust anchor
  **Location**: Phase 1 §2 (self-location); Phase 3 (documented symlink chain)
  One derived path supplies the key, the verifier that uses it, and the cache the
  launcher is exec'd from. A bootstrap whose chase lands in a prepared tree with
  its own `plugin.json`, key and shim passes every gate and execs a launcher the
  attacker signed — minisign succeeds against the attacker's key. This is a
  pre-existing property of the env-var design (and the write-only rule is a real
  improvement, since an ambient variable was easier to inject), but Phase 3 makes
  the symlink chain the documented, hook-refreshed default. The launcher already
  solves this: `cli/launcher/build.rs` embeds the key at build time.

- 🟡 **Security + Portability + Correctness + Test Coverage**: the SessionStart
  symlink writer specifies no clobber, atomicity or mode semantics
  **Location**: Phase 3 §2 The hook; Phase 3 §1 The hook test suite
  `ln -sf TARGET LINK` where `LINK` is already a symlink to a *directory* creates
  the new link inside it on both BSD and GNU (`-n`/`-h` is the fix), so a
  pre-planted link makes the hook write outside `${CLAUDE_PLUGIN_DATA}` — the
  containment property the snapshot-diff test is meant to establish. A regular
  file at that path is neither preserved nor refused, the same clobber risk the
  work item cites as a reason not to write `~/.local/bin`. `ln -sf` is also
  unlink-then-symlink, so concurrent sessions and in-flight terminal invocations
  can observe `ENOENT`, and `mkdir -p` with no explicit mode can leave a
  group-writable `bin/` under a permissive umask.

- 🟡 **Architecture + Code Quality**: Phase 5's requirement at `compose_stack`
  breaks seven measured root-independent command families
  **Location**: Phase 5 Overview and §2
  The plan measures that `config agents`, `paths`, `path <k>`,
  `instructions <skill>`, `context --skill`, `work <k>` and `review <k>` are
  byte-identical with and without a root, then imposes the requirement on the one
  composition point they all share. For the "future caller who forgets the
  export" the phase exists to protect, the change *widens* the empty-answer
  surface from one family to eight.

- 🟡 **Correctness + Test Coverage**: three of Phase 5's four claimed degradation
  removals do not follow, and none is individually tested
  **Location**: Phase 5 §2 Drop the silent fallbacks; Testing Strategy → Unit
  `template_names` (`store.rs:356-376`) has a *second* silent fallback
  (`let Ok(entries) = fs::read_dir(plugin.join("templates")) else { return
  Vec::new() }`) and a port signature returning `Vec<String>`, not `Result`;
  `known_skill_names` is still suppressed downstream by `if !known.is_empty()` in
  `config_command/core/summary.rs:144`; and the plugin-default tier's
  `if default.is_file()` still yields the identical `Failure::Refusal`
  "not found". So "a missing template file is distinguishable from a skipped
  tier" is not delivered. `known_skill_names`' removal — the widest behavioural
  change, since rootless skill-name validation currently passes silently — has no
  specified test at all.

- 🟡 **Code Quality**: non-optional `plugin_root` forces a constructor and
  `compose()` signature change the phase does not scope
  **Location**: Phase 5 §2
  The field is defaulted to `None` by `FileConfigStore::at()` (`store.rs:42-49`)
  and set later by `with_plugin_root(Option<PathBuf>)` (`:58`). Making it
  non-optional means `config_adapters::compose()` (`compose.rs:41`) — which does
  not know the plugin root — plus 17 `FileConfigStore::at(&root)` calls in
  `store.rs`'s own tests and one in `tests/parity.rs:53` must all supply one.
  Presented as a four-site change, it is a public-API change rippling to ~20
  sites; the likely shortcut is an internal `Option` plus `expect`, reintroducing
  the shape the phase removes.

- 🟡 **Code Quality**: no `ConfigError` variant can carry "missing installation
  root" with the promised classification
  **Location**: Phase 5 §1
  `From<ConfigError> for Failure` (`cli.rs:422-429`) maps only
  `ConfigError::Invalid` to `Refusal`; the taxonomy in `cli/config/src/error.rs:38-66`
  has no variant for an absent root. `Invalid` renders the best message but
  fail-closes; `Io` degrades correctly but renders "I/O error on
  'ACCELERATOR_PLUGIN_ROOT'", which is not an I/O error.

- 🟡 **Architecture**: phase independence is overstated — Phases 2–5 all depend on
  a ~25-file Phase 1 hub
  **Location**: Implementation Approach; Phase 1 and Phase 2 Overviews
  Phase 4's suite is red before Phase 1; Phase 5's criteria are all expressed
  against a variable only Phase 1 introduces; Phase 3 ships documentation for an
  `ln -s` recipe that cannot work until the chase exists. Only Phase 0 is
  order-free. Under the constraint that phases be independently integratable this
  offers one very large first merge and no way to land the urgent repair
  separately from the rename — the work item itself sanctions a transitional
  dual export for exactly this purpose.

- 🟡 **Architecture**: the two-hop shim reintroduces the staleness that justified
  unconditional self-location, as channel-global filesystem state
  **Location**: Phase 3 Overview and §2
  `${CLAUDE_PLUGIN_DATA}/bin/accelerator` is one symlink shared by every session
  of a channel, written last-writer-wins with no version comparison, and the
  bootstrap now self-locates *through* it. Because hook commands keep the
  previous version's `${CLAUDE_PLUGIN_ROOT}` until `/reload-plugins`, a session
  started before an upgrade re-points the shim **backwards** for every terminal
  user and every other session — so the documented caveat ("for the rest of that
  session") understates the scope, and the link dangles once the old root is
  garbage-collected.

- 🟡 **Compatibility**: the lockstep guarantee holds only for released,
  version-bumped artifacts
  **Location**: Migration Notes
  Three reachable paths defeat it silently: the dev override execs whatever
  `ACCELERATOR_LAUNCHER_BIN` names inside `cli/target/`, so a pre-rename build
  left by a branch switch is used; during development `plugin.json` does not
  change, so a warm same-version cached launcher is a *hit* (`bin/accelerator:246`
  verifies and reuses it) with nothing invalidating it; and a same-version
  re-publish leaves the old binary cached, since the key carries no content hash.
  Every case yields an empty table at exit 0 with no `accelerator:` diagnostic,
  and Phase 5 — which would make it loud — is last.

- 🟡 **Compatibility**: `ACCELERATOR_MIGRATION_MODE` is not dead, and the
  assignment slated for removal is a live cross-layer contract test
  **Location**: Phase 1 §4 (opportunistic cleanup)
  `skills/config/migrate/scripts/run-migrations.sh:643` and
  `interactive-lib.sh:434,745` export it into every migration child; migrations
  `0001`, `0002`, `0004`, `0005`, `0006` invoke the Rust CLI; and
  `scripts/config-common.sh:41,56` plus `scripts/doc-type-table.sh:43` read it.
  `cli/config-adapters/tests/config_reader.rs:34` feeds
  `a_legacy_layout_fails_closed_under_migration_mode` (`:67-77`), which pins
  0178's decision not to port the bash bypass — an acceptance criterion 0178's own
  review demanded. Deleting the assignment collapses that test into a duplicate.

- 🟡 **Compatibility**: Phase 3 adds a third documented terminal recipe without
  reconciling the two that exist
  **Location**: Phase 3 §5 Documentation
  `skills/visualisation/visualise/SKILL.md:162` prints, live at every skill load,
  `ln -s "${CLAUDE_PLUGIN_ROOT}/bin/accelerator" "$HOME/.local/bin/accelerator"` —
  a **version-pinned one-hop** link, exactly what the two-hop shim exists to
  avoid; after Phase 1 it silently runs an older installation's bootstrap and
  cached launcher. `docs/visualiser.md:24,38` still documents an
  `accelerator-visualiser` wrapper that has not existed since the 0168 fold.

- 🟡 **Compatibility**: the `${CLAUDE_PLUGIN_DATA}` hedge protects the plugin but
  not the user-facing recipe
  **Location**: Phase 0 §2; Phase 3 §5
  "The hook is inert when unset" means that on any Claude Code lacking the
  variable the hook creates nothing and the documented `ln -s` therefore produces
  a **dangling** `accelerator` on the user's general `PATH`, with no diagnostic
  from either the hook (silent by design) or the link. The floor is prose-only
  with nothing enforcing or detecting a sub-floor install.

- 🟡 **Test Coverage**: 86 of Phase 4's per-command assertions reduce to "stdout is
  empty", which is byte-identical to a degraded failure
  **Location**: Phase 4 §2 (per-family table)
  The fixture configures instructions for one skill and context for one skill, so
  ~84 of those expectations are `stdout == ""` — satisfied equally by a launcher
  that failed every one of those reads at exit 0 with no `accelerator:` line. 41%
  of the suite cannot distinguish success from silent degradation, while the
  corpus size makes it read as comprehensive.

- 🟡 **Test Coverage + Security**: deleting the two root-gate tests leaves the
  "self-location lands outside a plugin root" path uncovered and installs no guard
  against the hazard recurring
  **Location**: Phase 1 §1 (the two deletions)
  No test exercises the `plugin.json` or `public_key` gates — every existing test
  builds a complete fixture root, and the new fail-safe test covers only the
  missing shim. Meanwhile the module still exposes
  `_BOOTSTRAP = _REPO_ROOT / "bin/accelerator"`, so any future test running it by
  repo path is now implicitly production-rooted, and omitting the injected
  downloader or the `.invalid` base URL is a one-line mistake. `.gitignore` covers
  neither `bin/accelerator-launcher-*` nor the digest-suffixed staged shims, the
  lock dir, or `.accelerator-unverified.log`.

- 🟡 **Test Coverage**: the empty-string `ACCELERATOR_PLUGIN_ROOT` edge case is
  fixed but never tested
  **Location**: Testing Strategy → Key Edge Cases; Phase 1 §3; Phase 5 §3
  Phase 1 adds the filter `cache_root.rs:27` already has, but no assertion covers
  it: bootstrap-level tests always export a non-empty derived root, and Phase 5
  §3 names only the `env_remove` cases. After Phase 5 the difference is a named
  error versus a `PathBuf::from("")` resolving templates relative to the cwd.

- 🟡 **Test Coverage + Architecture**: the `hooks.json` registration is only an
  ad-hoc `jq` line in the checklist, not a committed regression test
  **Location**: Phase 3 §3; Phase 3 Success Criteria
  The work item flags that without the entry the hook never fires while every
  other hook criterion still passes — yet all new suite cases invoke the script
  directly. The registration is checked once by hand. The fix should also assert
  by *content* rather than position: pinning index 3 doubles an existing
  positional coupling that carries no semantic meaning.

- 🟡 **Test Coverage + Correctness**: "fails closed on an empty match set" is
  ambiguous and untested, and the template has no such test either
  **Location**: Phase 2 §2 The guard; Phase 2 §4 The paired test
  The cited convention (`tasks/lint/scripts.py:8-11`) is about *file discovery*
  returning nothing, not violations; read as "no violations ⇒ fail" the guard
  inverts and can never pass. Neither the enumerated cases nor
  `violations(REPO_ROOT) == []` covers the branch — and that assertion is
  *also* satisfied when the walk yields zero files, so a pruning or
  suffix-filter regression reads as cleanliness.

- 🟡 **Correctness + Code Quality + Architecture + Test Coverage + Standards**: the
  promoted `sources()` signature cannot express either caller's pruning, and
  inherits root-only gitignore blindness
  **Location**: Phase 2 §1 Promote the gitignore-honouring walk
  `_py_files` (`test_python_coverage.py:68-94`) prunes `.venv` explicitly
  *because it is not in `.gitignore`*; `shell_sources()` needs `_keep`'s
  `workspaces/` rule applied to the extras too; and the two callers return
  different shapes (`set[str]` vs sorted `list[str]`). Separately `_ignore_spec()`
  honours only the root `.gitignore` (documented at `sources.py:13-16`, justified
  there for `.sh` only), so scanning `.js`/`.json`/`.ts` descends into
  `cli/visualiser/frontend/dist/` — ignored only by a nested file, and built
  before `cli:check` under `mise run`, which also defeats the plan's sub-second
  manual criterion.

- 🟡 **Architecture**: Phase 2's guard cannot see the entry point whose bug it
  documents
  **Location**: Phase 2 §2 The guard
  The docstring is to state "no plugin entry point may require
  `CLAUDE_PLUGIN_ROOT` from its process environment", but the scope is `cli/` with
  eight code suffixes. `bin/accelerator` is outside it, extensionless, and is
  where the invariant was violated — checked only by a one-off
  `! grep -q 'CLAUDE_' bin/accelerator` line in Phase 1's criteria. The four
  renamed writers outside `cli/` are likewise unguarded.

- 🟡 **Standards + Compatibility**: Phase 2's repo-wide purge criterion is
  unsatisfiable as written
  **Location**: Phase 2 Success Criteria
  `meta/**` is neither gitignored nor listed, yet dozens of tracked files there
  name the variable (including this plan and work item), and
  `tests/integration/entrypoint/test_accelerator_entrypoint.py` is omitted even
  though Phase 1 deliberately keeps the string there — the ambient-root test is
  parametrised over both names. A criterion that cannot pass has no
  discriminating power, so a genuine miss can be waved through as known residue.

- 🟡 **Standards**: wiring the guard into both `cli:check` and `lint:check`
  contradicts the precedent it names
  **Location**: Phase 2 §3 Registration
  `lint:store-duplication:check` is *not* in `lint:check.depends` — nor is
  `lint:vendor-shims:check`. The established split is `cli/`-scoped guards in
  `cli:check` only. Because the bare `default` task depends on `lint:check` (not
  `check`), dual-wiring would make this the sole `cli/`-scoped guard reachable
  from the default task — a third pattern rather than either existing one.

- 🟡 **Standards**: the new `_EXPECTED_HOOKS_SUITES` floor omits the paired guard
  test every other floor has
  **Location**: Phase 3 §4 Suite discovery floor
  All four existing floor constants have a guard class in
  `tests/unit/tasks/test_integration.py` (`TestConfigSuiteGuard`,
  `TestMigrateSuiteGuard`, `TestWorkSuiteGuard`, `TestIntegrationsSuiteGuard`),
  each asserting `pytest.raises(Exit)` below baseline. The plan adds neither a
  `TestHooksSuiteGuard` nor that file to Phase 3's criteria, so an off-by-one in
  the new `Exit` block would ship silently.

- 🟡 **Standards**: rewriting `shell_sources()` skips its own dedicated regression
  suite
  **Location**: Phase 2 §1; Phase 2 Success Criteria
  `tests/unit/tasks/shared/test_sources.py` — nine cases, importing the private
  `_keep` — is named nowhere, and the plan never says where `sources()`'s own
  tests live, though the `tests/unit/tasks/shared/test_<module>.py` mirroring
  convention determines the answer.

- 🟡 **Standards**: `tests/fixtures/skill-conformance-project/` breaks the fixture
  co-location convention
  **Location**: Phase 4 §1 The fixture project
  No top-level `tests/fixtures/` exists; every Python-test fixture tree is
  co-located (`tests/unit/tasks/fixtures/`,
  `tests/integration/deny/fixtures/{banned,clean,…}/`), and the shell side uses a
  parallel `<subtree>/scripts/test-fixtures/` convention pinned by
  `tasks/lint/scripts.py:57`.

- 🟡 **Standards**: a SKILL.md conformance suite under `tests/integration/tasks/`
  contradicts that directory's and the mise task's stated scope
  **Location**: Phase 4 §2, §3
  That directory holds integration tests of invoke task *modules*
  (`test_github.py` → `tasks/github.py`, `test_release.py` → `tasks/release.py`),
  and `test:integration:tasks`' own description is "Run pytest integration tests
  for invoke tasks" (`mise.toml:161`). Widening it gives the task a Rust build
  dependency and a scope its description contradicts.

- 🟡 **Standards + Code Quality**: "shim" already means the minisign verify binary
  **Location**: Phase 3 §§1–3; Phase 3 §5
  `bin/accelerator`'s `shim_source`/`shim_digest`/`shim`, the committed
  `bin/accelerator-verify-<platform>` triples, `tasks/lint/vendor_shims.py`,
  `lint:vendor-shims:check`, `build:vendor-verify-shims` and the entrypoint
  suite's `shim_bin` fixture all use it for the verifier — and Phase 1 of this
  plan does too. Reusing it for a convenience symlink means `grep -ri shim` stops
  discriminating, and a "shim refresh" failure at SessionStart reads as the trust
  root breaking.

#### Minor

- 🔵 **Correctness + Security + Compatibility**: the argv scan honours `--fail-safe`
  anywhere in argv, including as an option value or after `--`, so a
  caller-supplied value can degrade integrity gates the launcher would have
  rejected as malformed. **Location**: Phase 1 §2 (argv scan)
- 🔵 **Correctness**: "external-subcommand dispatch still succeeds with no root"
  contradicts `cli/launcher/tests/version.rs:166-183`, in the file the phase
  edits — the lazy closure only skips the *config* compose; an external
  subcommand still fails at the cache-root step. **Location**: Phase 5 §3
- 🔵 **Correctness**: two falsification claims describe the wrong failure mode —
  the two-hop test's "a single dereference yields an empty table" actually aborts
  at the `plugin.json` gate, and removing `hardcoded_fallback`'s `status)` arm
  also reddens Test 7 and both tripwires, so it does not isolate the seam (Test 5
  already supplies the positive direction). **Location**: Phase 1 §1, §6
- 🔵 **Correctness + Standards**: five line/file citations have drifted —
  `test:integration:entrypoint` is `mise.toml:179-182` (`:184-187` is
  `test:integration:config`, which *already* has the edge);
  `preprocessor_commands`/`is_plugin_invocation` are at `skill_permissions.py:94-96,99-101`
  (`:104-111` is `covered_by`); `scripts.py:130-133` is the `chmod +x` branch, not
  `chmod -x`; `test_bootstrap_coverage.py:39-43` pins only the key, not
  `.accelerator-dev-launcher`; and "11 remaining reads" should be 8. Also
  "`check` depends on the seven `<component>:check` roll-ups" is five roll-ups plus
  two entity tasks per `tasks/README.md:32-36`. **Location**: multiple phases
- 🔵 **Portability + Standards**: the bash 3.2 floor is only incidentally verified —
  `scripts/lint-bashisms.sh` is self-documented KNOWN-INCOMPLETE and bans none of
  the snippet's constructs, `_run_bootstrap` spawns PATH-resolved `bash` (so
  Homebrew bash 5 on macOS runs the suite off-floor and green), and
  `for arg in ${1+"$@"}` has no precedent in the ~175-file shell corpus (the
  precedented guard is an explicit `$#` test) and carries no justified
  `# shellcheck disable=` under `enable=all`. **Location**: Phase 1 §2; Phase 1
  Success Criteria
- 🔵 **Code Quality + Standards**: `_SUFFIXES` is deny-by-omission — `.md`, `.sh`,
  `.py`, `.yml`/`.yaml` and the `.mts`/`.cts` that `.editorconfig:14-17`
  anticipates are all unguarded under `cli/`, while the acceptance criterion says
  "no tracked source file under `cli/`". **Location**: Phase 2 §2
- 🔵 **Code Quality**: an empty `ALLOWLIST` "with per-entry reasons" is YAGNI, and
  the failure message advertising it invites the coarse whole-path exemption
  (`cli/launcher/src/main.rs`) that would blind the guard to the very site that
  caused this bug. **Location**: Phase 2 §2
- 🔵 **Code Quality**: `fail()` deciding its exit status from a global flag makes
  all 14 call sites read dishonestly; resolving the policy once into a named
  `abort_status` keeps the contract visible as data. **Location**: Phase 1 §2
- 🔵 **Code Quality**: the hop bound is duplicated in the comparison and the
  message, `${BASH_SOURCE[0]}` is re-expanded three times, and `plugin_root` —
  whose whole design rests on being written once — is a plain mutable local rather
  than `readonly`. **Location**: Phase 1 §2
- 🔵 **Code Quality + Test Coverage**: `_ADR_SENTINEL.format(tag=…)` couples
  assertions to `make_harness` call order, and `_run_bootstrap` has no "invoke via
  this path" parameter, which two of the eight tests require. **Location**: Phase
  1 §1
- 🔵 **Code Quality**: `test_rootless_templates_list_carries_the_plugin_default_row`
  pins the renderer's exact column order, backtick quoting and padding in a test
  about root resolution — the plan elsewhere rejects path-shaped assertions for
  being indirect. **Location**: Phase 1 §1
- 🔵 **Code Quality**: three prescribed snippet comments restate what the code
  says or reference the reported failure (which goes stale); only the `ELOOP` note
  and the chain diagram earn their place under this repo's comment policy.
  **Location**: Phase 1 §1, §6
- 🔵 **Test Coverage**: `launcher_bin` duplicates `build:cli:dev` verbatim while
  Phase 1 also adds the mise edge and pins it in `_LAUNCHER_DEPENDENTS` —
  contending on cargo's target lock and making the asserted edge inert. Pick one.
  **Location**: Phase 1 §1, §7
- 🔵 **Test Coverage**: routing six of eight new tests through a signed real
  launcher is slow and asserts R2's export only transitively; an env-dumping stub
  would assert the derived root directly. **Location**: Phase 1 §1
- 🔵 **Test Coverage**: `is_plugin_invocation` is
  `command.startswith("${CLAUDE_PLUGIN_ROOT}/")` — true for every plugin *script*
  invocation, not just `bin/accelerator config`. As literally specified the
  harness would execute arbitrary plugin scripts, some of which write `meta/` or
  call remote trackers. **Location**: Phase 4 §2
- 🔵 **Test Coverage**: the `>= 200 commands` floor fires on benign skill
  consolidation and stays quiet on losing a whole family; a structural invariant
  (every SKILL.md with a `config` `!` site contributes ≥1; every family
  non-empty) is self-maintaining. **Location**: Phase 4 Success Criteria
- 🔵 **Test Coverage**: two hook branches are uncovered — `CLAUDE_PLUGIN_ROOT`
  unset (the self-location fallback, which is the mid-session-upgrade path), and a
  pre-existing regular file or directory symlink at the shim path.
  **Location**: Phase 3 §1
- 🔵 **Security**: Phase 4 does not say whether extracted commands are executed
  through a shell; `!`-block content is arbitrary shell text, so `shell=True`
  makes any SKILL.md an execution vector. The plan's own measurement means
  `shlex.split` with `shell=False` loses nothing. **Location**: Phase 4 §2
- 🔵 **Security**: `${CLAUDE_PLUGIN_DATA}` is validated only for non-emptiness, so
  a relative value composes against the hook's cwd — the user's project directory
  — and the snapshot-diff test would not catch it (it always supplies an absolute
  temp path). **Location**: Phase 3 §2
- 🔵 **Portability**: the inertness test's "no `/bin` entry" assertion is vacuous
  wherever `/` is unwritable (both CI legs, and SIP on macOS), so it passes
  whether or not the guard exists; the environment where it matters — Claude Code
  as root in a container — is the one the test cannot reach. **Location**: Phase 3
  §1
- 🔵 **Portability**: the hook suite's snapshot diff is specified by intent, which
  is where GNU-only behaviour creeps in — `find -printf` and `stat -c` do not
  exist on macOS, and an unset `LC_ALL` makes `sort` order tree listings
  differently under a UTF-8 locale. Pin `find … -print | LC_ALL=C sort`,
  `diff -u`, plain `readlink`, no `stat`. **Location**: Phase 3 §1
- 🔵 **Portability**: the documented recipe needs `mkdir -p ~/.local/bin` (the
  directory often does not exist, so the bare `ln -s` fails), and on Debian/Ubuntu
  the "when present" `PATH` entry is evaluated at *login shell* start, so a user
  who creates the directory to follow the recipe still has no `accelerator` in the
  current session. **Location**: Phase 3 §5
- 🔵 **Compatibility**: "consumers who exported `CLAUDE_PLUGIN_ROOT` are
  unaffected" is wrong in one direction — `hooks/config-detect.sh:10`,
  `migrate-discoverability.sh:23` and `run-migrations.sh:6` read it as a
  *higher-precedence override*, and `scripts/interactive-harness.sh:29` reads it
  with a hard `:?`, so a stale version-pinned export silently pins the migration
  runner to a previous plugin version with no signal. **Location**: Migration
  Notes
- 🔵 **Compatibility**: there are 205 `!`-site launcher invocations, not 204 — the
  excluded one (`skills/visualisation/visualise/SKILL.md:30`) is the least safe:
  no `--fail-safe`, genuine argument interpolation, and the only `!` site
  exercising the renamed `cache_root.rs` read, whose failure is a hard error with
  no degradation. The root-sensitivity table also omits external-subcommand
  dispatch, which hard-requires the root. **Location**: Phase 4 Overview
- 🔵 **Compatibility**: the channel-switch procedure in
  `docs/releases-and-compatibility.md:21-29` becomes incomplete — the shim is
  per-plugin-id, so the user's first hop must be re-pointed — and the manual
  criterion "the dangling link stays in plugin-owned space" is true only of the
  second hop. **Location**: Phase 3 §5; Phase 3 Manual Verification
- 🔵 **Standards**: `## Terminal invocation` breaks `docs/internals.md`'s Title
  Case convention (`## The meta/ Directory`, `## Agents`, `## VCS Detection`), and
  the grep criterion makes the inconsistency the tested contract.
  **Location**: Phase 3 §5
- 🔵 **Standards**: the Phase 1 §6 shell snippet uses a tab where
  `.editorconfig`'s `[*.sh]` mandates 2 spaces (shell has no autofixer), and the
  Python snippets are not `ruff format` output (compressed argument list,
  backslash continuation). The `bin/accelerator` snippet's tabs *are* correct —
  the extensionless file does not match `[*.sh]` — but the plan does not say the
  two deliberately differ. **Location**: Phase 1 §1, §6
- 🔵 **Standards**: "the `hooks` subtree is the only test-bearing subtree with no
  floor count" is false — `decisions` (1 suite) and `github` (3 suites) also lack
  one. **Location**: Phase 3 §4
- 🔵 **Standards**: the 43-per-family count conflicts with
  `tasks/lint/skill_permissions.py:36-39`'s `EXPECTED_INJECTION_SKILLS = 42`,
  which is an exact equality over the same corpus the harness reuses. One is
  stale, and Phase 4's fail-closed threshold inherits the discrepancy.
  **Location**: Phase 4 Overview and table
- 🔵 **Standards**: three registration conventions are unaddressed —
  `tasks/lint/__init__.py` keeps its import tuple and `__all__` alphabetical,
  every `ns_lint.add_collection` call carries a trailing comment naming the task
  path, and `tasks/README.md` (the designated "shape of the tree" reference)
  already under-describes `cli:check` and is in neither phase's file list.
  **Location**: Phase 2 §3; Phase 3 §5
- 🔵 **Architecture**: after this change only `bin/accelerator` can determine the
  root, and the plan rejects launcher self-location outright — so any path
  bypassing the bootstrap (`ACCELERATOR_BIN`, used by 28 files and the test
  overlay; the dev harness; a future second entry point) has no recovery, which
  Phase 5's hard requirement turns from "degraded" into "fails entirely".
  **Location**: Desired End State; Phase 1 §3
- 🔵 **Architecture + Standards**: the guard's `cli:check` reachability criterion is
  vacuous — `test_mise.py`'s `_CHECK_GATES` asserts only that
  `cli:check`/`deny:check`/`pup:check` appear in `check.depends`, and Phase 2 adds
  nothing to it, so the wiring that gives the guard its only CI reach can be
  dropped silently. `_LAUNCHER_DEPENDENTS` likewise pins only a positive list, so
  a new suite added to neither it nor `mise.toml` still ships green.
  **Location**: Phase 2 §3; Phase 1 §7
- 🔵 **Architecture**: the closing step has a precondition outside the plan (a
  published prerelease), duplicates the closure sequence already in Migration
  Notes, and makes a work-item acceptance criterion satisfiable only after a
  release the same item gates. **Location**: Closing step
- 🔵 **Standards**: `mise.local.toml` is already at `.gitignore:26`, which is the
  repo's convention for machine-local mise overrides — so the per-push
  `git cat-file -e` ritual is largely redundant, while the genuinely unusual fact
  (a gitignored file present in the jj working-copy commit) goes unexplained.
  **Location**: Migration Notes; Closing step

#### Suggestions

- 🔵 **Architecture + Compatibility**: record the `${CLAUDE_PLUGIN_DATA}` reversal
  durably (a short ADR, or a forward-pointing note on
  `meta/plans/2026-05-06-…:125`) — the prior decision's stated rationale
  ("availability across Claude Code versions is uncertain") is exactly what Phase
  0 §2 now resolves, and it becomes a load-bearing dependency of a supported user
  surface. **Location**: Phase 3 Overview
- 🔵 **Security**: document the two integrity preconditions the recipe depends on —
  a user-owned, non-group-writable `PATH` directory (Homebrew's `/usr/local/bin`
  frequently is not), and deleting the link on uninstall, since the hook that
  repairs the second hop stops running. **Location**: Phase 3 §5
- 🔵 **Security**: add a Phase 5 assertion that the rootless failure under
  `--fail-safe` emits **nothing on stdout**, since anything on stdout at a `!`
  site is spliced into the prompt and leaves the machine. Removing the
  `known_skill_names` fallback is a security improvement worth calling out as
  such. **Location**: Phase 5
- 🔵 **Portability**: name `ACCELERATOR_RELEASE_BASE_URL` and
  `ACCELERATOR_CACHE_DIR` in the new docs section as the supported overrides for
  mirrored/offline and read-only installs, state the supported platforms (the
  bootstrap gates on `uname -s` for `Darwin`/`Linux`), and state that
  `ACCELERATOR_PLUGIN_ROOT` is exported by the bootstrap and never read by it.
  **Location**: Phase 3 §5
- 🔵 **Portability + Architecture**: have the Phase 2 docstring name the
  adapter-layer exemption explicitly, so the boundary reads as "nothing under
  `cli/`" rather than an unqualified absolute that the very next phase's hook
  breaks. **Location**: Phase 2 §2
- 🔵 **Code Quality**: promote `_PLUGIN_PREFIX` to a public `PLUGIN_PREFIX`
  alongside the two functions the harness already imports, rather than making a
  third literal copy of the substitution prefix. **Location**: Phase 4 §2
- 🔵 **Code Quality**: state Phase 1's commit sequence (tests → bootstrap →
  readers → writers → seam → build edge) and move the `ACCELERATOR_MIGRATION_MODE`
  cleanup out, so the shell change — where the risk actually is — is not reviewed
  inside a ~25-file diff. **Location**: Implementation Approach; Phase 1
- 🔵 **Test Coverage**: extend `tests/unit/tasks/test_bootstrap_coverage.py` with an
  assertion that `ACCELERATOR_PLUGIN_ROOT` appears as an `export` in
  `bin/accelerator` and as the `var_os` key in both `main.rs` and `cache_root.rs`,
  so a one-sided rename is a fast unit failure rather than an integration symptom.
  **Location**: Migration Notes
- 🔵 **Standards**: `meta/decisions/ADR-0051-skills-as-the-product.md:117` also
  states the v2.1.144 floor as a live fact ("currently v2.1.144"), so it is a
  third site if the floor rises. **Location**: Phase 0 §2

### Strengths

- ✅ Measurement over assumption throughout: the per-family root-sensitivity
  table, `display_path`'s `<plugin>/` shortening, and the 204-command corpus shape
  each removed a wrong assumption *before* implementation, and three acceptance
  criteria were restated on that basis rather than carried forward broken.
- ✅ The Deviations table replaces path-shaped assertions the renderer provably
  never emits with fixture sentinels, and the two-sentinel form in criterion 5 is
  falsifiable in both directions with the negative assertion paired to a positive
  one, so it cannot pass vacuously.
- ✅ Test-first is not merely asserted — the plan includes a manual step running
  the new regression tests against the stashed pre-change bootstrap to prove they
  are red first.
- ✅ The exact `templates list` row string is byte-valid against `render::list`,
  and the real-launcher fixture is mechanically sound: the bootstrap fetches a raw
  minisign-signed binary, so no archive shape, `checksums.json`, manifest or SLSA
  gate is bypassed.
- ✅ Strict directionality (bootstrap writes `ACCELERATOR_PLUGIN_ROOT` and never
  reads it) is a genuine security and correctness improvement, eliminating the
  stale/ambient-root class by construction, and it is pinned by a test
  parametrised over both names individually and together.
- ✅ The symlink chase reuses a previously reviewed specification rather than being
  re-derived, correctly requires a loop rather than one dereference, and correctly
  places the `${CLAUDE_PLUGIN_DATA}` inertness guard *before* any path
  composition.
- ✅ The two deleted tests are removed on a correct and non-obvious mechanism
  analysis — post-self-location they would satisfy every gate and perform a live
  network fetch plus a working-tree write — rather than as tidiness.
- ✅ Recognising that the four existing R4 seam assertions pass vacuously, and
  re-pointing the seam onto the script's own documented `ACCELERATOR_BIN` override
  (the repo's convention in 28 files) with a distinguishable-values pair, states
  the seam's actual intent instead of relying on a root gate's side effect.
- ✅ Phase 2 refuses in-tree sentinel violations by design, citing the documented
  `test:unit:tasks` / `cli:check` concurrency — a flake avoided rather than
  discovered.
- ✅ The exec-bit treatment is exactly right (0755, mode committed, deliberately
  *not* in `SHELL_LIBRARIES`), the `hooks.json` append-at-index-3 constraint
  verifies against a repo-wide search for hard-coded SessionStart indices, the
  `ACCELERATOR_*` naming fits the existing namespace, and the `bin/accelerator`
  snippet's tab indentation is correct for an extensionless file.
- ✅ The Phase 1 fixture keeps test key material out of the shipped tree (keypairs
  generated into `tmp_path`, `*.key` gitignored, `ACCELERATOR_RELEASE_BASE_URL`
  pinned to `example.invalid`, downloader injected), and the plan preserves the
  inherited hardening it does not touch (content-addressed shim staging, shim
  invoked by absolute path, no eviction before a verified successor, PID-owner
  lock reclaim).
- ✅ The `cli/` rename inventory matches a live grep exactly — all 21 `CLAUDE_`
  occurrences across 11 files, including the three message-text assertion sites
  called out as observable contract — and the majority of the plan's ~40 line
  citations verify correct.
- ✅ Resequencing `mise.local.toml` to last on the correct reasoning (it is what
  keeps the tools needed to do the work functioning) reverses two source documents
  on evidence rather than preference.
- ✅ Every new mechanism is modelled on an existing precedent
  (`store_duplication.py`, `migrate-discoverability.sh`, `_EXPECTED_WORK_SUITES`,
  the reviewed chase), so the change adds capability without adding architectural
  vocabulary.
- ✅ Phase 3's two-hop split gives each hop a distinct owner and stability
  guarantee, and the four recorded reasons for not writing `~/.local/bin` from an
  unprompted hook are sound.

### Recommended Changes

1. **Re-design Phase 5 around a per-capability requirement** (addresses: the
   `--fail-safe` path that does not exist; `config_read.rs:62` is the suite-wide
   runner; the requirement breaks seven root-independent families; three of four
   degradation removals do not follow; the missing `ConfigError` variant; the
   constructor ripple; four untested degradation paths). Keep `plugin_root`
   optional at `compose_stack`; add a `ConfigError::PluginRootUnavailable
   { detail }` variant that lands in the `Read` arm of
   `From<ConfigError> for Failure`; return it from the four template/
   plugin-content consumers instead of `Ok(vec![])`/`Ok(None)`/`vec![]`. Then
   state honestly what the phase buys (a missing *root* becomes a named error) and
   scope separately what it does not (`template_names`' second `read_dir`
   fallback and its `Vec<String>` port signature; `summary.rs:144`'s
   `!known.is_empty()` guard). Add one test per former degradation site asserting
   both halves of the split.

2. **State the launcher-supply mechanism for every phase that runs the
   bootstrap** (addresses: the network-fetch critical; the non-hermetic Phase 4
   suite; the lockstep-guarantee gaps). Reuse the Phase 1 fixture shape — a
   fixture installation root plus the stub release server serving the freshly
   built launcher — and substitute the extracted prefix to *that* root, for both
   Phase 4 and Phase 1's `templates list` criterion. Add an autouse
   `conftest.py` fixture for the entrypoint suite forcing an unroutable
   `ACCELERATOR_RELEASE_BASE_URL` and requiring the injected downloader, add
   `bin/accelerator-launcher-*`, `bin/accelerator-verify-*-*`,
   `bin/.accelerator-lock-*` and `bin/.accelerator-unverified.log` to
   `.gitignore`, and record in Migration Notes that the dev override and a warm
   same-version cache sit outside the lockstep guarantee.

3. **Split `fail()` into two classes** (addresses: silent integrity failures; the
   "template sites stay loud" contradiction; the argv scan's breadth). Mirror the
   launcher's model: configuration gates (arch/OS, cache dir, downloader, lock
   timeout) degrade under `--fail-safe`; integrity gates (verify shim, public key,
   shim staging, refused dev launcher, fetch-and-verify) stay closed regardless —
   or degrade *loudly*, appending to the existing
   `${cache_dir}/.accelerator-unverified.log`. Scan only tokens before the first
   `--`. Update `bin/accelerator:7`'s "Fail-closed throughout" line to match, and
   add a test that the verification-failure case is distinguishable from the
   legitimately-empty case.

4. **Harden the chase's path handling and make the bound testable** (addresses:
   unchecked `readlink`/`cd`; `cd ""` yielding the cwd; `CDPATH`; the unreachable
   hop counter). Use `cd -- "${self%/*}"` (parameter expansion, no external
   `dirname`, immune to a leading `-`), add `|| fail …` to the `readlink` and the
   relative-target resolution, assert the derived root is absolute, set `CDPATH=`
   alongside `set -uo pipefail`, and mark the result `readonly`. Lower the hop
   bound below both kernels' `SYMLOOP_MAX` (e.g. 16) so a non-cyclic 17-link chain
   makes the named error observable on both platforms — or keep 40 and label it
   unreachable-for-cycles insurance, replacing the cycle test with a
   terminates-non-zero assertion. Add cases for a target beginning with `-`, a
   target containing a space, and a derived root lacking `.claude-plugin/plugin.json`.

5. **Pin the trust anchor into the shipped bootstrap** (addresses:
   trust-anchor relocation). Follow the launcher's own precedent
   (`cli/launcher/build.rs` embeds the key; `resolve/keys.rs` verifies against the
   embedded copy): add a literal expected SHA-256 of
   `keys/accelerator-release.pub` to `bin/accelerator`, fail closed on mismatch,
   and pin the constant against the committed key in
   `test_bootstrap_coverage.py`. Add a test that a relocated bootstrap facing a
   *different* key is refused.

6. **Pin the hook's symlink mechanics and its uncovered branches** (addresses:
   clobber/atomicity/mode; relative `${CLAUDE_PLUGIN_DATA}`; backwards
   re-pointing; the two uncovered branches; the untested registration).
   `mkdir -p` + explicit `0755`, `ln -sfn` into a temporary name then `mv -f`
   (rename is atomic), refuse a non-symlink destination with a stderr diagnostic,
   require an absolute `${CLAUDE_PLUGIN_DATA}` in the inertness guard, and refuse
   to move the link backwards by comparing the target's `plugin.json` version. Add
   suite cases for: destination as a regular file, destination as a
   symlink-to-directory, `CLAUDE_PLUGIN_ROOT` unset, a relative
   `${CLAUDE_PLUGIN_DATA}`, and a content-based (not index-based) `hooks.json`
   registration assertion. Make the inertness assertion non-vacuous by asserting
   the hook's *decision* rather than the absence of a write the OS blocks anyway.

7. **Fix the `sources()` promotion or narrow it to the walk** (addresses: the
   signature that cannot express either caller's pruning; nested-gitignore
   blindness; the untested `.venv` prune; the skipped regression suite). Prefer
   promoting the *walk* (`walk_files(root, spec, prune=())`) and keeping suffix
   filtering, subtree scoping and per-caller pruning in the three callers; either
   way add per-directory `.gitignore` layering or an explicit build-output prune
   (`dist/`, `.venv/`), preserve each caller's return shape, add
   `assert not any(p.startswith(".venv/") …)` to `TestInScopeSet` *before* the
   refactor, and put `sources()`'s own cases in
   `tests/unit/tasks/shared/test_sources.py` (naming it in Phase 2's criteria,
   since it imports `_keep` privately).

8. **Correct Phase 2's scope, wiring, and fail-closed semantics** (addresses: the
   guard cannot see `bin/accelerator`; deny-by-omission suffixes; the empty
   `ALLOWLIST`; the ambiguous and untested fail-closed rule; the `lint:check`
   precedent; the unsatisfiable purge criterion; the vacuous reachability
   criterion). Scope the guard to the rename set (`cli/` + `bin/accelerator` +
   the four renamed writers) with the adapter layer as named `ALLOWLIST` entries
   carrying reasons; invert the suffix filter to scan-by-default and skip a named
   binary set; state explicitly that the fail-closed check is on the *discovered
   file set* being empty and test that branch plus a file-count floor; wire the
   leaf into `cli:check` only; add an explicit `_CLI_CHECK_GATES` assertion for
   it; and rescope the purge grep to exclude `meta/` and permit
   `test_accelerator_entrypoint.py` with its reason.

9. **Restate phase independence and decompose Phase 1** (addresses: the
   overstated claim; the ~25-file hub; commit granularity). Say plainly that the
   phases are *sequentially* mergeable with Phase 1 as the hub, then split it: 1a
   — bootstrap self-locates, `--fail-safe`-aware `fail()`, exporting **both**
   names transitionally (green on its own, and the shippable urgent fix); 1b — the
   reader/writer rename and dropping the old export. Note that two pieces are
   green *today* and can land independently: the `ACCELERATOR_BIN` seam re-point,
   and the `build:cli:dev` edge with its `test_mise.py` assertion. Move the
   `ACCELERATOR_MIGRATION_MODE` cleanup out of Phase 1 (and, per finding, drop it
   — it is not dead) and state the intended commit sequence.

10. **Reconcile the documentation surfaces** (addresses: three competing `ln -s`
    recipes; the dangling-link hedge gap; channel switching; the `mkdir -p` and
    login-shell gaps; heading case; the "shim" collision). Re-point
    `skills/visualisation/visualise/SKILL.md:159-162` at
    `${CLAUDE_PLUGIN_DATA}/bin/accelerator` (or at the new docs section), fix
    `docs/visualiser.md:24,38` to `accelerator visualiser`, add both to Phase 3's
    file list and criteria; make the recipe
    `mkdir -p ~/.local/bin && ln -sfn …` with a verify step and the
    Debian-next-login note; add a re-point/remove step to the channel-switch and
    uninstall procedures; use `## Terminal Invocation`; and rename the hook away
    from "shim" (e.g. `hooks/launcher-link-refresh.sh`) so the term keeps meaning
    the minisign verifier.

11. **Close the remaining test gaps and fix the citations** (addresses: 86 empty
    assertions; the empty-string root; the deleted tests' lost coverage; the
    `>= 200` floor; `is_plugin_invocation` over-collecting; `shell=True`; the
    hooks floor's missing guard test; the fixture and suite placement; the 43-vs-42
    discrepancy; the drifted references). Give the Phase 4 fixture a distinct
    non-empty instructions *and* context override for every skill in the corpus so
    all 86 become content matches; add a launcher-level empty-string root case;
    add a "derived root is not a plugin root" test; replace the numeric floor with
    a structural invariant and derive family sizes from
    `EXPECTED_INJECTION_SKILLS`; compose the extraction filter explicitly
    (`is_plugin_invocation(cmd) and "/bin/accelerator config " in cmd`) and state
    `shlex.split` with `shell=False`; add `TestHooksSuiteGuard`; co-locate the
    fixture and give the conformance suite its own topic directory and mise leaf;
    and correct the five drifted citations plus the "11 reads"/"seven roll-ups"/
    "only subtree with no floor" claims.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Correctness

**Summary**: The plan's Phase 1 logic (self-location loop, argv scan, fail-safe
funnel) is broadly sound and correctly reuses a reviewed prior art, but three of
its load-bearing correctness claims do not survive checking against the tree.
Phase 5 rests on a false premise — a compose-time plugin-root error propagates
through `dispatch`'s `compose_config()?` before any `--fail-safe` policy is
consulted, so its own success criterion is unsatisfiable, its "silent degradation
removed" bullets are mostly unachieved without an unscoped port change, and it
would break the majority of the 133 tests in `config_read.rs` (whose shared
runner strips the root for every test, not one). Phase 4's conformance harness
has no stated mechanism to supply a launcher and would run the real bootstrap
against the real release URL — the exact hazard the plan cites when deleting two
entrypoint tests — and the prescribed symlink-cycle test cannot fire the
in-script hop counter at all because `open()` returns ELOOP before bash starts.

**Strengths**:

- Test-first sequencing with an explicit red-against-pre-change verification
  step.
- The two deleted entrypoint tests are removed on a correct mechanism analysis.
- The restated acceptance criteria replace unsatisfiable `display_path`
  assertions with fixture sentinels; the two-sentinel form is falsifiable in both
  directions.
- Correctly identifies that the chase must be a loop; relative-target resolution
  handles the practical shapes, including a bare filename where `dirname` yields
  `.`.
- Correctly requires the `${CLAUDE_PLUGIN_DATA}` inertness guard before any path
  composition.
- Correctly spots the missing empty-string filter at `main.rs:176` relative to
  `cache_root.rs:27`.
- The directionality invariant structurally eliminates the stale/ambient-root
  class; the `hooks.json` append-at-index-3 constraint checks out.

**Findings**:

- **critical / high — A compose-time plugin-root error cannot degrade under
  `--fail-safe`; Phase 5's own criterion is unsatisfiable** (Phase 5 §1).
  `dispatch` calls `let stack = compose_config()?;`
  (`cli/launcher/src/launch/mod.rs:189`) and converts the `ConfigError` straight
  into `kernel::Error` — `--fail-safe` lives inside `config_cli::run` →
  `run_read` → `finish` (`config_command/inbound/cli.rs:452-471`), reached only
  once a `ConfigStack` exists. A missing root exits non-zero with or without the
  flag. Impact: the criterion is unsatisfiable and the change reintroduces the
  prompt-discarding mode for any `!` site whose launcher runs without the root.
  Suggestion: thread the parsed action's `on_failure` into the dispatch site, or
  keep the boundary infallible and move the requirement inward.
- **critical / high — Phase 5 misreads `config_read.rs:62` as one test case; it
  is the suite-wide runner** (Phase 5 §3). `run_in` (`:58-67`) is used by ~150
  invocation sites across 133 tests and calls
  `env_remove("CLAUDE_PLUGIN_ROOT")` unconditionally; only template tests use
  `run_with_plugin` (`:1196-1208`). Impact: Phase 5 lands red across the
  launcher's largest suite; `mise run test:unit:cli` cannot pass. Suggestion:
  have `run_in` inject a root and keep a few deliberately rootless cases; also
  reconcile with the measured root-*insensitivity* of the seven non-template
  families.
- **major / high — Phase 4 has no mechanism to supply a launcher** (Phase 4 §2).
  Substituting to the repo root runs the real `bin/accelerator`, which passes
  every gate and falls through to real `curl` against
  `v<plugin.json version>` — 404 for an unpublished version at the time the phase
  lands. The dev override cannot substitute (containment gate needs the repo
  root; the marker there is forbidden by `test_accelerator_entrypoint.py:826`),
  and a locally signed launcher cannot be served because the repo's key is the
  real release key. Suggestion: reuse the fixture-installation + stub-server +
  test-key shape and substitute to the fixture root, noting that the 20
  `template <name>` non-empty assertions then need the fixture to carry the real
  `templates/` tree.
- **major / high — The symlink-cycle test cannot reach the in-script hop
  counter** (Phase 1 §1 and Manual Verification). `bash "$CYCLE_A"` must `open()`
  that path and the kernel returns `ELOOP` there too; bash aborts with exit
  126/127 and the named message never prints. The counter is unreachable for any
  true cycle, and on Darwin the 32-hop bound is unreachable; on Linux the only
  reachable case is a 33–40-hop chain the kernel would have resolved.
  Suggestion: assert termination non-zero, or extract the chase into a sourced
  helper with an injected start path.
- **major / high — Three of the four claimed degradation removals do not follow**
  (Phase 5 §2). `template_names` (`:356-376`) has a second silent fallback
  (`let Ok(entries) = fs::read_dir(…) else { return Vec::new() }`) and a
  `Vec<String>` port signature; `known_skill_names` (`:281`) is still suppressed
  by `if !known.is_empty()` in `config_command/core/summary.rs:144` and
  internally by `let Ok(content) = … else { continue }`; the plugin-default
  tier's `if default.is_file()` (`:344-351`) still yields `Ok(None)` producing
  the same `Failure::Refusal`. Suggestion: narrow the claims or scope the port
  change explicitly.
- **major / high — The promoted `sources()` drops the `.venv` prune that
  `_py_files` needs** (Phase 2 §1). `_py_files` (`:68-94`) prunes `.venv`
  explicitly *and separately from gitignore*, with a comment stating why (`.venv`
  is not in `.gitignore` — confirmed). Impact: thousands of `.venv/**/*.py` paths
  enter the discovered set, reddening the scope-coherence assertions. Suggestion:
  add an explicit extra-prune argument and preserve each caller's return shape
  (`set[str]` vs sorted `list[str]`).
- **major / medium — Unchecked `readlink` and `cd` failures can silently resolve a
  wrong root** (Phase 1 §2). Under `set -uo pipefail` without `-e`, a failed
  `readlink` (a real TOCTOU window given the Phase 3 hook's per-session rewrite)
  leaves `self` as `"<dir>/"` and `dirname` returns the grandparent; a failed
  inner `cd` makes `self` become `/<target>`. Suggestion: add `|| fail …` to both,
  assigning the directory to a temporary first so the failure is detectable.
- **minor / high — "External-subcommand dispatch still succeeds with no root"
  contradicts an existing test in the same file** (Phase 5 §3). An external
  subcommand still calls `cache_root::resolve(&CacheRootConfig::from_env())`,
  which fails closed with no root and no `ACCELERATOR_CACHE_DIR` — asserted by
  `cli/launcher/tests/version.rs:166-183`. Suggestion: restate as "fails at the
  *cache-root* step, not the plugin-root step".
- **minor / high — Blanket exit-0 in `fail()` contradicts the "template `<name>`
  stays loud" claim** (Phase 1 §2 vs Phase 4 table). The fail-closed guarantee
  holds only for launcher-layer `Failure::Refusal`; a missing verify shim,
  unusable cache dir, lock timeout or failed fetch now produces empty stdout at
  exit 0 for those same 20 sites. Suggestion: scope the claim, and consider
  keeping broken-installation gates loud.
- **minor / medium — The argv scan matches `--fail-safe` anywhere, including as an
  option value or after `--`** (Phase 1 §2). The bash-3.2/`set -u` form is right
  (bash before 4.4 errors on a bare `"$@"` with no positionals, and quoting is
  preserved inside the alternate word), but the scope is not. Suggestion: scan
  only leading option-shaped arguments and stop at the first `--` or non-flag
  token.
- **minor / medium — `cd "$(dirname …)"` is CDPATH-sensitive** (Phase 1 §2).
  Invoked as `bash bin/accelerator`, `dirname` yields `bin`; with `CDPATH`
  exported, `cd` searches it first and *prints* the resolved directory to stdout
  inside the command substitution, so `plugin_root` becomes two lines.
  Suggestion: `CDPATH=` alongside `set -uo pipefail`, and/or normalise to `./…`.
- **minor / medium — `ln -sf` re-point is not atomic** (Phase 3 §2). Unlink-then-
  symlink; two windows starting at once, or a terminal invocation resolving the
  user's hop at that instant, can hit the gap. `ln -sf` also lacks `-n`.
  Suggestion: create under a temporary name and `mv -f`; pass `-n`; add a
  re-point-twice case.
- **minor / medium — "Fails closed on an empty match set" is ambiguous and would
  invert the guard; the suffix list is narrower than the criterion** (Phase 2 §2).
  The cited convention (`tasks/lint/scripts.py:8-11`,
  `_EMPTY_SCOPE = "no shell sources matched — scope discovery is broken"`) is
  about file discovery, not violations. `_SUFFIXES` misses `.md`, `.sh`,
  `.yaml`/`.yml`, snapshots and extension-less scripts. Suggestion: state the
  check is on the discovered file set, and align the suffix set with the
  criterion.
- **minor / high — Two falsification claims are inaccurate about the mode they
  would fail in** (Phase 1 §1, §6, Manual Verification). (a) With a single
  dereference the root becomes `<tmp>/plugin-data`, which fails the `plugin.json`
  gate — the run aborts at exit 1 before any table renders. (b) Removing
  `hardcoded_fallback`'s `status)` arm also reddens Test 7 (`:1071`) and both
  tripwire comparisons, and the positive half of the new pair duplicates existing
  Test 5 (`:1043-1045`). Suggestion: correct the comment, add an exit-0 assertion
  to the two-hop test, and note Test 5 already supplies the positive direction.
- **minor / high — Measured counts and line references that have drifted** (Phase
  1 §2, §7; Phase 5 §2). (a) "the remaining 11 reads" — there are 11 including
  the three inside the two deleted gates (`:25,26,27`); the remaining set is the
  **8** enumerated. (b) `test:integration:entrypoint` is `mise.toml:179-182`;
  `:186-189` is `test:integration:config`. (c) `display_path` is `store.rs:74-84`
  (doc from `:72`), `known_skill_names` `:281`, `template_names` `:356`,
  `plugin_default` `:378`; `:343` and `:413-417` are correct.

### Portability

**Summary**: The plan's core self-location design is portability-sound: it
deliberately rejects `readlink -f`, uses the pure-bash `while [[ -L ]]` chase plus
`pwd -P` (genuinely BSD/GNU-equivalent), keeps the bash-3.2 floor's `${1+"$@"}`
iteration form, matches the surrounding file's `[[ ]]`/tab style, and lands both
new suites under `test:integration`, which CI already runs on the ubuntu and
macos legs. The weaker areas are the phases that add *new* host-environment
surface: the Phase 3 hook's symlink mechanics and its shell suite are specified
only by intent, the Phase 4 conformance harness does not say how it supplies a
launcher and as described would fall through to a real network fetch, and the
newly *documented* terminal surface deepens coupling to Claude Code-owned paths
and to GitHub Releases without stating the platform scope or the mirror/cache
override knobs. None of the findings are blockers; most are one-line pins that
stop a darwin-green/linux-red (or worse, root-in-container) divergence appearing
later.

**Strengths**:

- The chase reuses an already-reviewed shape and explicitly rejects `readlink -f`,
  a capability probe, and a `perl` fallback; `readlink` and `dirname` exist on
  every supported host (Darwin, glibc/musl Linux, WSL).
- `for arg in ${1+"$@"}` is the correct pre-bash-4.4 form, and the plan records
  why.
- Both new suites sit under `test:integration`, which CI runs on
  `ubuntu-latest` *and* `macos-latest`, so no workflow edit is needed.
- The Deviations table's root-independent assertions incidentally sidestep the
  macOS `/private/var` vs `/var` comparison trap.
- The hook is confined to `${CLAUDE_PLUGIN_DATA}` with the inertness guard placed
  before any path composition.
- `mise.local.toml` (a machine-specific `/Users/…` path) is already gitignored,
  plus a belt-and-braces pre-push check.
- Phase 5's split keeps the diagnostic contract portable across the two very
  different invocation environments rather than picking one.

**Findings**:

- **major / medium — Phase 4's harness never says which binary answers the call**
  (Phase 4 §2). In a checkout the resolved root is the repo, so each of ~204
  commands runs the repo bootstrap, which self-locates, satisfies every gate,
  finds no cached launcher and `curl`s
  `https://github.com/atomicinnovation/accelerator/releases/download/v<version>/…`
  — the exact hazard cited for deleting `test_unset_plugin_root_is_a_named_error`.
  The added `build:cli:dev` edge hints at `ACCELERATOR_BIN`, but the dev override
  is unusable and `ACCELERATOR_BIN` contradicts "precisely the production shape".
  Suggestion: state the mechanism; add an assertion that the suite performs no
  network fetch (e.g. `ACCELERATOR_RELEASE_BASE_URL=https://example.invalid/`).
- **minor / medium — The hook's `ln` invocation, flags and non-symlink cases are
  unspecified** (Phase 3 §2). Three host-dependent traps: `ln -sf` onto a
  symlink-to-a-directory creates the link *inside* it on both BSD and GNU
  (`ln -sfn` is the portable fix — BSD documents `-n` as an alias for `-h`);
  `ln -sf` cannot replace a real directory; and `${CLAUDE_PLUGIN_DATA}/bin` may
  not exist. Suggestion: `mkdir -p` then `ln -sfn`, with a
  `[ -e "$dest" ] && [ ! -L "$dest" ]` branch; add tests for a pre-existing
  regular file and a pre-existing directory.
- **minor / medium — `lint:scripts:check` is the only automated floor gate, and it
  proves little** (Phase 1 Success Criteria). `scripts/lint-bashisms.sh` is
  self-documented KNOWN-INCOMPLETE and bans none of `[[ ]]`, `${arg}`, `$((…))`,
  `BASH_SOURCE[0]` or `${1+"$@"}` — it would pass equally if `${1+"$@"}` were
  "simplified" to `"$@"`, which errors under `set -u` on bash 3.2 (fixed in 4.4).
  `_run_bootstrap` spawns PATH-resolved `bash`, so a contributor with Homebrew
  bash 5 on macOS runs the suite off-floor and green. Suggestion: invoke through
  `/bin/bash` explicitly (or parametrise a second leg), or assert `BASH_VERSINFO`
  on Darwin; reword the criterion.
- **minor / medium — The cycle test cannot go green, and 32 makes the one reachable
  case platform-divergent the wrong way** (Phase 1 §2; Key Edge Cases). `bash
  FILE` also `open()`s FILE and gets `ELOOP`; any path bash can open has had its
  whole chain resolved within `SYMLOOP_MAX`. A 33–40-hop chain opens fine on
  Linux but the bootstrap rejects it, while on Darwin it never opens. The bare
  `32` also appears twice with no in-code note. Suggestion: verify empirically;
  assert exit status rather than message text; use 40 so the script never rejects
  a chain the kernel accepts; add a one-line `SYMLOOP_MAX` comment.
- **minor / medium — The hook suite's snapshot diff is specified by intent, not by
  tool** (Phase 3 §1). `find -printf` and `stat -c` do not exist on macOS (BSD
  `stat` uses `-f`), `readlink -f` is a BSD no-op, and an unset `LC_ALL` makes
  `sort` order listings differently under a UTF-8 locale. Suggestion: pin
  `find "$tree" -print | LC_ALL=C sort`, `diff -u`, plain `readlink`, no `stat`,
  and rule those out explicitly as the plan already does for `readlink -f`.
- **minor / medium — The inertness test's "no `/bin` entry" assertion is vacuous**
  (Phase 3 §1). Both CI legs run unprivileged and macOS `/bin` is SIP-protected,
  so it passes whether or not the guard exists; the environment where the guard
  matters — Claude Code as root in a container — is unreachable by the test.
  Suggestion: assert the hook's *decision* (composed path via a debug flag, or a
  refusal on stderr) rather than the absence of a write the OS blocked anyway.
- **minor / high — The documented recipe omits `mkdir -p` and the login-shell
  caveat** (Phase 3 §5). `~/.local/bin` frequently does not exist, so the bare
  `ln -s` fails; and on Debian/Ubuntu the "when present" test in `~/.profile`
  runs at *login shell* start, so a user who creates the directory to follow the
  recipe still has no `accelerator` in the current session. Suggestion:
  `mkdir -p ~/.local/bin && ln -sfn "${CLAUDE_PLUGIN_DATA}/bin/accelerator"
  ~/.local/bin/accelerator`, plus one sentence about the current shell.
- **suggestion / high — The only documented entry point is a Claude Code-owned
  path, and the Phase 2 docstring reads as absolute while the adapter layer is the
  exception** (Phase 3 Overview and §5; Phase 2 §2). The next phase's hook itself
  reads `CLAUDE_PLUGIN_ROOT`. Suggestion: name the adapter-layer exemption in the
  docstring; state in `docs/internals.md` that the escape hatch, if ever needed,
  is an `install-shim` subcommand.
- **suggestion / high — The two knobs that relieve mirrored/offline and read-only
  installs are undocumented** (Phase 1 §2; Phase 5; Phase 3 §5).
  `ACCELERATOR_RELEASE_BASE_URL` and `ACCELERATOR_CACHE_DIR` exist in
  `bin/accelerator` but appear only in `meta/`. Suggestion: name them in the new
  section, and state that `ACCELERATOR_PLUGIN_ROOT` is exported by the bootstrap
  and never read by it.
- **suggestion / high — No platform statement, and an inert hook leaves the user's
  link dangling silently** (Phase 3 §5; Phase 0 §2). `bin/accelerator`'s
  `case "${host_os}"` accepts only `Darwin` and `Linux`, so Git Bash/MSYS fails
  the gate. Suggestion: add a one-line platform statement and the
  `${CLAUDE_PLUGIN_DATA}` floor to the new section; have the hook emit one
  `[accelerator]` stderr line when inert.

### Test Coverage

**Summary**: This is an unusually rigorous test-first plan: it names the
falsifying assertion for almost every change, restates three acceptance criteria
specifically to make them falsifiable in both directions, and correctly
identifies existing vacuous tests (the four R4 seam assertions) and existing
tests that assert the bug (the two deletions). The strongest idea — serving the
real compiled launcher through the existing stubbed release server — is
mechanically sound (the bootstrap fetches a raw minisign-signed binary, so no
archive/manifest/SLSA gate is involved) and eliminates the stub that would
otherwise hide the rename. Three areas nonetheless carry real coverage risk:
Phase 4 never states how the bootstrap it invokes obtains a launcher (as written
it would fetch an unreleased version over the network from CI), the
symlink-cycle test cannot reach the code it claims to test because the kernel
returns ELOOP first, and several risks the plan itself enumerates (empty-string
root, the plugin-root-missing gate left uncovered by the two deletions,
hooks.json registration) have no committed assertion.

**Strengths**:

- Test-first is stated as a rule and backed by an explicit falsifiability check
  (stash, confirm red, restore).
- The real-compiled-launcher fixture is mechanically sound — `_serve_launcher`
  signs whatever file is placed in the stub server and the bootstrap fetches a
  raw binary, so no release-pipeline gate is bypassed.
- The three restated criteria move assertions off a Path the renderer provably
  never emits and onto fixture sentinel content — strictly more falsifiable.
- The two-sentinel ambient-root test is falsifiable in both directions and is
  parametrised over each variable alone and both together.
- The exact `templates list` row assertion is byte-valid against `render::list`.
- Recognising the four seam assertions as vacuous and adding a
  distinguishable-values pair is the right instinct, and the pair is genuinely
  two-directional.
- `_LAUNCHER_DEPENDENTS` closes a real hole.
- Phase 2 explicitly refuses in-tree sentinels because `test:unit:tasks` races
  `cli:check`, citing the documented precedent.
- The rationale for deleting the two root-gate tests is correct and non-obvious.

**Findings**:

- **critical / high — Phase 4 never states how the bootstrap obtains a launcher**
  (Phase 4 §2, §3). With the root at the repo, the bootstrap `curl`s
  `v<dev-version>`, which does not exist, so every command degrades and prints
  `accelerator:` on stderr, failing the suite's own stderr assertion; a version
  that *did* exist would supply the pre-rename launcher — a chicken-and-egg. The
  `build:cli:dev` edge does not help: the bootstrap has no `ACCELERATOR_BIN`
  seam, and reaching `cli/target/debug/accelerator` needs the
  `.accelerator-dev-launcher` marker at the repo root, which
  `test_accelerator_entrypoint.py:826` asserts must never exist. Suggestion:
  reuse the Phase 1 `real_launcher` shape with
  `ACCELERATOR_RELEASE_BASE_URL` pointed at the stub server and substitute the
  prefix to that fixture root.
- **major / high — The symlink-cycle test cannot reach the in-script hop counter**
  (Phase 1 §1). `bash <path>` calls `open(2)` on the same path and the kernel
  returns `ELOOP` before bash reads a byte. The bound is unreachable by
  construction: if the path is openable, the chain has no cycle and fewer hops
  than `SYMLOOP_MAX` (32 Darwin, 40 Linux), so 32 can only fire on Linux for a
  33–40-hop non-cyclic chain. Suggestion: lower the bound *below* both kernels
  (e.g. 16 — the documented chain is two hops) and test with a 17-link non-cyclic
  chain, or drop the test and label the bound explicitly defensive.
- **major / high — Deleting the two root-gate tests leaves the "self-location
  lands outside a plugin root" path with no coverage** (Phase 1 §1). No existing
  test exercises the `plugin.json` or `public_key` gates — every test builds a
  complete fixture root — and the new fail-safe test covers only the missing
  shim. Suggestion: add a test copying `bin/accelerator` into a bare `<tmp>/bin/`
  with no `.claude-plugin/`, asserting a named `plugin.json not found` naming the
  *derived* path (which also pins that self-location happened), plus the
  `--fail-safe` variant.
- **major / high — 86 of 204 Phase 4 assertions reduce to "stdout is empty"**
  (Phase 4 §2). All 204 carry `--fail-safe`; a `Failure::Read` under it yields
  exit 0, empty stdout, no `accelerator:` prefix. The fixture configures one
  skill each, so ~84 expectations are `stdout == ""` — satisfied by a launcher
  that failed every one of those reads. Suggestion: give the fixture a distinct
  non-empty instructions *and* context override for every skill name in the
  corpus, deriving expectations from the same fixture data.
- **major / high — The empty-string `ACCELERATOR_PLUGIN_ROOT` edge case is fixed
  but never tested** (Key Edge Cases; Phase 1 §3; Phase 5 §3). Bootstrap tests
  always export a non-empty derived root, Phase 1 §4 only renames, and Phase 5 §3
  names only `env_remove` cases. Suggestion: add a launcher-level case setting
  `ACCELERATOR_PLUGIN_ROOT=""` asserting parity with unset, parametrised
  alongside the existing case.
- **major / high — The `hooks.json` index-3 registration is only an ad-hoc `jq`
  line** (Phase 3 §3, Success Criteria). All new suite cases invoke the script
  directly; the registration is run once by hand. Suggestion: add a structural
  block mirroring `hooks/test-vcs-detect.sh:615-634`, searching the array for the
  command rather than hard-coding index 3.
- **major / high — "Fails closed on an empty match set" has no test case, and the
  template has none either** (Phase 2 §2, §4). The enumerated cases do not cover
  it, `test_store_duplication.py` has no such test, and
  `violations(REPO_ROOT) == []` is *also* satisfied when the walk returns nothing.
  Suggestion: raise when the scanned-file set is empty; add
  `violations(<empty tmp tree>)` fails-closed plus a file-count floor on the real
  tree.
- **major / medium — Phase 5's four former degradation paths are not given a test
  each; `known_skill_names` is the riskiest and is unnamed** (Phase 5 §3). With
  no root, skill-name validation currently passes silently, so after the change
  `config instructions <bogus-skill>` must start erroring — asserted nowhere.
  Suggestion: enumerate one test per site, each asserting both halves of the
  split.
- **minor / high — `launcher_bin` duplicates `build:cli:dev` verbatim** (Phase 1
  §1, §7). Character-for-character what `tasks/build.py:257-260` runs; under
  `mise run` the two contend on cargo's target lock, and `_LAUNCHER_DEPENDENTS`
  then pins an edge the suite does not need. Suggestion: pick one.
- **minor / high — Most new bootstrap tests need only the derived root** (Phase 1
  §1). Six of eight route a multi-tens-of-MB debug binary through sign/copy/
  rename per test and per parametrisation, and leave R2's export asserted only
  transitively. Suggestion: extend `_LAUNCHER_SRC` to dump its environment and
  assert `ACCELERATOR_PLUGIN_ROOT == <expected>` directly; reserve
  `real_launcher` for the rendering tests.
- **minor / high — Harness spec gaps** (Phase 1 §1). `_run_bootstrap` hard-codes
  `bash str(root / "bin/accelerator")` with no "invoke via this path" parameter,
  which the symlink-chain and cycle tests require; and
  `_ADR_SENTINEL.format(tag=counter['n'])` ties each root's sentinel to
  `make_harness` call order (and `tag=1` would substring `tag=10`). Suggestion:
  add `entry: Path | None = None`; give `make_harness` a named
  `sentinel="self"`/`"injected"` argument.
- **minor / high — `is_plugin_invocation` over-collects** (Phase 4 §2). It is
  `command.startswith("${CLAUDE_PLUGIN_ROOT}/")` (`skill_permissions.py:99-101`)
  — true for every plugin *script* invocation. As specified the harness would
  execute arbitrary plugin shell scripts, some of which write `meta/` or call
  remote trackers. Suggestion: state the composed filter and log
  extracted-but-excluded counts.
- **minor / medium — The `>= 200` floor is a magic number** (Phase 4 Success
  Criteria). Removing five `!` sites turns it red for a legitimate change, while
  losing a whole skill's two commands (or the 20 `template` sites) still clears
  200 if skills are added elsewhere. Suggestion: assert every SKILL.md with a
  `config` `!` site contributes ≥1 command and every family is non-empty.
- **minor / medium — Repointing `test_python_coverage.py` drops an untested
  `.venv` prune** (Phase 2 §1). `_py_files` (`:76-94`) prunes it because it is
  not in `.gitignore`, and returns a `set`; no existing assertion would fail if
  the prune vanished. Suggestion: add
  `assert not any(p.startswith(".venv/") …)` before the refactor, and give
  `sources()` its own unit test.
- **minor / medium — Two hook branches uncovered** (Phase 3 §1). The
  `CLAUDE_PLUGIN_ROOT`-unset self-location fallback (the mid-session-upgrade
  path), and a pre-existing regular file or directory symlink at the shim path
  where `ln -sf` writes *inside* the target while still exiting 0. Suggestion:
  add both cases asserting the final `readlink` target is exactly
  `<root>/bin/accelerator`.
- **suggestion / medium — No durable guard against a half-rename** (Migration
  Notes). Nothing textually pins that the name the bootstrap exports is the name
  the launcher reads. Suggestion: extend
  `tests/unit/tasks/test_bootstrap_coverage.py` — which already pins shared
  literals across the trust boundary — with that assertion.

### Architecture

**Summary**: The plan's central structural move — transferring ownership of the
installation-root variable from a Claude-Code-owned name to a plugin-owned one,
with the bootstrap as sole writer and the Rust layers as sole readers — is a
genuine improvement in boundary integrity, and the layer vocabulary (bootstrap /
launcher / server / adapter) gives the rename a principled membership rule. Two
structural problems stand out: Phase 5 places the non-optional plugin-root
requirement at `compose_stack`, which sits *above* the `--fail-safe` boundary and
outside the only capability that is measurably root-sensitive, so its stated
mechanism does not exist and its scope over-constrains six ports; and Phase 2's
guard is scoped to `cli/` while the invariant it documents was violated by
`bin/accelerator`, which the guard cannot see. Phase independence is also
overstated — Phases 2–5 all depend on Phase 1, which is a ~25-file hub that the
work item itself sanctions decomposing via a transitional dual export.

**Strengths**:

- The ownership transfer is a real boundary improvement consistent with
  ADR-0045, and strict directionality eliminates a class of stale-override bugs
  by construction rather than convention.
- The layer vocabulary gives the rename a membership rule based on participation
  rather than directory — which is why the four out-of-tree writers were found.
- The plan measures instead of assuming, and restates three criteria where
  measurement contradicted them.
- Phase 4 turns the skill↔CLI integration surface (a cost ADR-0048 names) into an
  executable contract, with a fixture project as the oracle so it does not drift.
- Phase 3's two-hop split assigns each hop a distinct owner and stability
  guarantee; the four rejection reasons for writing `~/.local/bin` are sound.
- Every new mechanism is modelled on an existing precedent, so the change adds
  capability without adding architectural vocabulary.
- The reasoning for resequencing `mise.local.toml` to last is correct and
  reverses two source documents on evidence.

**Findings**:

- **critical / high — A compose-time plugin-root error cannot degrade under
  `--fail-safe`** (Phase 5 §1). `dispatch` calls
  `let stack = compose_config()?;` (`launch/mod.rs:189`) and propagates to
  `kernel::Error`; `--fail-safe` is a per-action flag translated into
  `OnFailure` in `to_action` and applied only by `finish`
  (`config_command/inbound/cli.rs:452-471`). Impact: a hard non-zero exit for
  every `config` command including all 204 `!` sites, reinstating at the launcher
  layer the mode Phase 1 removes at the bootstrap layer; making it pass requires
  duplicating the degradation decision outside the single `finish` funnel.
  Suggestion: move the requirement below the fail-safe boundary — have the
  template ports return a named `ConfigError`.
- **major / high — Phase 5's boundary placement imposes a one-capability
  requirement on nine** (Phase 5 Overview, §2). Seven families that work rootless
  today stop working, so for the "future caller who forgets the export" the change
  *widens* the silent-wrong-answer surface from one family to eight, and couples
  five ports to an input none consumes. Suggestion: keep `plugin_root` optional
  and make the four plugin-content consumers return
  `ConfigError::PluginRootUnavailable`.
- **major / high — Phase 2's guard enforces a strictly weaker invariant than it
  documents** (Phase 2 §2). The entry point that violated it is
  `bin/accelerator` — outside `cli/`, no extension — and the four renamed writers
  are outside too. The bootstrap regression is checked only by a one-off
  `! grep -q` in Phase 1's criteria. Suggestion: define the scope as the rename
  set (`cli/` + `bin/accelerator` via `_EXTRA_SHELL_SOURCES` + the four writers)
  with the adapter layer as named `ALLOWLIST` entries.
- **major / high — Phase independence is overstated** (Implementation Approach;
  Phase 1 and 2 Overviews). Phase 4 is red before Phase 1; Phase 5's criteria are
  expressed against a variable only Phase 1 introduces; Phase 3 ships docs for a
  recipe that cannot work yet. Only Phase 0 is order-free. Suggestion: restate as
  sequential mergeability with Phase 1 as the hub, and split it into 1a
  (self-location + fail-safe + dual export — the shippable urgent fix) and 1b
  (the rename). Note the seam re-point and the build edge are green *today*.
- **major / medium — The two-hop chain reintroduces staleness as shared filesystem
  state on the critical path** (Phase 3 Overview, §2). One symlink per channel,
  last-writer-wins, no version comparison, and the bootstrap self-locates
  *through* it — so a session started before an upgrade re-points the shim
  *backwards* for every terminal user and every other session, and the link
  dangles once the old root is collected. Suggestion: refuse to move backwards by
  comparing the target's `plugin.json` version; document the channel-global
  last-session-wins semantics.
- **minor / high — Promote the walk, not the policy** (Phase 2 §1). The proposed
  signature cannot express `_py_files`' `.venv` prune (documented as necessary
  because `.venv` is not in `.gitignore`), and `_keep`'s `workspaces/` exclusion
  is in fact already redundant with `.gitignore:24` — so the only genuinely
  caller-specific policy is the one the signature drops. Suggestion: expose
  `walk_files(root, spec, prune=())` and keep filtering/scoping/pruning in each
  caller.
- **minor / high — Two wiring guards provide little** (Phase 2 §3; Phase 1 §7).
  The "gate is reachable from `cli:check`" criterion is vacuous —
  `test_mise.py`'s `_CHECK_GATES` asserts only `check.depends` membership and
  Phase 2 adds nothing to it — and `_LAUNCHER_DEPENDENTS` pins seven names by
  hand while the entrypoint fixture already builds the launcher itself.
  Suggestion: add an explicit `_CLI_CHECK_GATES` assertion; derive
  `_LAUNCHER_DEPENDENTS` or state it is a review prompt, not enforcement.
- **minor / medium — After this change only `bin/accelerator` can determine the
  root** (Desired End State; Phase 1 §3). The plan rejects launcher
  self-location outright — a valid argument against making it primary, not
  against a last-resort fallback — so any bypass path (`ACCELERATOR_BIN` in 28
  files and the test overlay, the dev harness, a second entry point) has no
  recovery, which Phase 5 escalates from "degraded" to "fails entirely".
  Suggestion: note a possible `current_exe()`-walked-up fallback, or state
  explicitly that non-bootstrap paths must supply the variable.
- **minor / high — An existing positional coupling is worked around and doubled**
  (Phase 3 §3, Success Criteria). Suggestion: assert by content in both places
  (`jq -e '[.hooks.SessionStart[].hooks[].command] | any(endswith("…"))')` and
  relax the `vcs-detect.sh` assertion while it is being read anyway.
- **minor / medium — The closing step cannot be completed by implementation work**
  (Closing step; Migration Notes). Its precondition is outside the plan, it
  duplicates the closure sequence, and it makes an acceptance criterion
  satisfiable only after a release the same item gates. Suggestion: carry it as a
  post-release line in the work item and the `meta/validations/` note.
- **suggestion / medium — The `${CLAUDE_PLUGIN_DATA}` reversal is not recorded
  durably** (Phase 3 Overview). The prior decision was narrower than the framing
  suggests (it chose `~/.cache/accelerator/` for a Playwright *binary cache*, on
  the grounds that availability across versions is uncertain — precisely what
  Phase 0 §2 resolves), and it still reads as current guidance. Suggestion: a
  short ADR, or a forward-pointing note on the 2026-05-06 plan.

### Compatibility

**Summary**: The plan's core compatibility move — renaming the launcher/server
root variable and landing the bootstrap export in the same phase — is correctly
sequenced, and the write-only bootstrap plus the parametrised ambient-root test
close the redirect hazard cleanly. However the Migration Notes' central claim,
that the version-keyed cache guarantees no pre-rename launcher stays reachable,
holds only for released, version-bumped artifacts: the plan's own Phase 1
verification command will exec the *released* 1.24.0-pre.16 launcher and
reproduce the exact silent-empty mode the work item exists to remove, and the
dev-launcher override gives the same result with no diagnostic. Secondary gaps
are three now-competing documented terminal-invocation recipes, an unenforceable
prose-only `${CLAUDE_PLUGIN_DATA}` floor whose hedge protects the plugin but not
the user-facing recipe, and one cross-layer contract test
(`ACCELERATOR_MIGRATION_MODE`) being removed as dead when it is not.

**Strengths**:

- Phase 1 lands the export and the readers as one indivisible unit, discharging
  the work item's dual-export contingency rather than deferring it.
- The bootstrap is write-only, pinned by a test parametrised over the old name,
  the new name, and both.
- The empty-string filter is added to `main.rs:176`, removing a divergence where
  two readers of the same variable disagreed on whether `""` means "set".
- Root-sensitivity was measured, and three criteria restated against the
  renderer's actual `<plugin>/` shortening.
- The chase reuses a reviewed design and stays inside ADR-0049's bash 3.2 floor.
- Phase 2 converts the boundary into a machine check wired into `cli:check` (the
  roll-up CI runs) *and* backed by `violations(REPO_ROOT) == []`.

**Findings**:

- **critical / high — Phase 1's `./bin/accelerator config templates list`
  criterion cannot pass before the fix is released** (Phase 1 Success Criteria).
  The cache is keyed on `plugin.json`'s version, which the change does not bump —
  still `1.24.0-pre.16` — so the bootstrap fetches the **published pre-rename**
  launcher, which reads `CLAUDE_PLUGIN_ROOT` that `env -u` just removed, and
  renders a header-only empty table at exit 0: the exact mode the criterion
  forbids. It also makes a real network call and writes an untracked launcher plus
  `.minisig` into the tracked `bin/` (no `.gitignore` entry for
  `bin/accelerator-launcher-*`). Suggestion: use the Phase 1 fixture harness or
  the documented dev override, and gitignore `bin/accelerator-launcher-*`,
  `bin/accelerator-verify-*-*`, `bin/.accelerator-lock-*` and
  `bin/.accelerator-unverified.log`.
- **major / high — The lockstep guarantee holds only for released, version-bumped
  artifacts** (Migration Notes). Three silent defeats: the dev override execs
  whatever `ACCELERATOR_LAUNCHER_BIN` names inside `cli/target/`; during
  development `plugin.json` does not change, so a warm same-version cached
  launcher is a *hit* (`bin/accelerator:246`) with nothing invalidating it; and a
  same-version re-publish leaves the old binary cached, since the key carries no
  content hash. Phase 5, which would make this loud, is last and "additive".
  Suggestion: export both names transitionally, or pull Phase 5's `Result` change
  into Phase 1; at minimum state the exclusions and require
  `mise run build:cli:dev` before dev-override validation.
- **major / high — `ACCELERATOR_MIGRATION_MODE` is dead only inside `cli/`**
  (Phase 1 §4). `run-migrations.sh:643` and `interactive-lib.sh:434,745` export it
  into every migration child; migrations `0001`, `0002`, `0004`, `0005`, `0006`
  invoke the Rust CLI; `scripts/config-common.sh:41,56` and
  `scripts/doc-type-table.sh:43` read it. `config_reader.rs:34` feeds
  `a_legacy_layout_fails_closed_under_migration_mode` (`:67-77`), pinning 0178's
  decision not to port the bash bypass. Suggestion: drop the removal and instead
  reword `store.rs:20` to say the variable is *received* and deliberately
  ignored, cross-referencing `run-migrations.sh:643`.
- **major / high — Phase 3 adds a third documented terminal recipe without
  reconciling the two that exist** (Phase 3 §5).
  `skills/visualisation/visualise/SKILL.md:162` prints a **version-pinned
  one-hop** `ln -s` live at every skill load — exactly what the two-hop shim
  avoids — so after Phase 1 an existing link silently runs an older
  installation's bootstrap and cached launcher. `docs/visualiser.md:24,38` still
  documents an `accelerator-visualiser` wrapper that has not existed since the
  0168 fold. Suggestion: re-point the SKILL.md, fix `docs/visualiser.md`, and add
  both to the phase's files and criteria.
- **major / medium — The `${CLAUDE_PLUGIN_DATA}` hedge does not cover the surface
  at risk** (Phase 0 §2). Inertness protects the plugin; the user-facing artefact
  is a hand-run recipe, so on a Claude Code lacking the variable the documented
  `ln -s` yields a **dangling** command on the user's general `PATH` with no
  diagnostic from either side. The floor is prose-only with nothing enforcing or
  detecting a sub-floor install, and unlike the subagent skill-preload soft floor
  there is no equivalent guard. Suggestion: emit one `[accelerator]` stderr line
  naming the variable and the required version; state the version inline at the
  recipe and open it with `ls -l "${CLAUDE_PLUGIN_DATA}/bin/accelerator"`.
- **minor / high — "Consumers who exported `CLAUDE_PLUGIN_ROOT` are unaffected" is
  wrong for the adapter layer** (Migration Notes). `hooks/config-detect.sh:10`,
  `migrate-discoverability.sh:23` and `run-migrations.sh:6` read it as a
  *higher-precedence* override, and `scripts/interactive-harness.sh:29` with a
  hard `:?`. A version-pinned workaround export therefore makes the migration
  runner source `config-common.sh`, `atomic-common.sh` and the whole
  `migrations/` set from a previous plugin version — with no signal, because the
  CLI now works regardless. Suggestion: tell such consumers to remove it, and
  carry that into the release notes.
- **minor / high — There are 205 `!`-site launcher invocations, not 204** (Phase 4
  Overview, §2). The excluded one,
  `skills/visualisation/visualise/SKILL.md:30`, is the least safe: no
  `--fail-safe`, genuine argument interpolation, and the only `!` site exercising
  the renamed `cache_root.rs` read (external-subcommand dispatch resolves
  `<plugin_root>/bin` and returns `CacheRootUnavailable` without a root). The
  root-sensitivity table also covers only `config` subcommands. Suggestion: note
  the corpus is the `config` subset of 205, add one assertion for the visualiser
  site (`visualiser status` needs no daemon) or record the gap, and amend the
  table.
- **minor / medium — The purge criterion is unsatisfiable** (Phase 2 Success
  Criteria). `meta/**` carries well over two hundred tracked matches, and
  `tests/integration/entrypoint/test_accelerator_entrypoint.py` is omitted even
  though Phase 1 keeps the string there deliberately. The guard's suffix set also
  excludes `.md`, `.sh` and `.py`, so a `cli/**` README could reintroduce a
  documented dependency without tripping it. Suggestion: scope to non-`meta/`,
  add the entrypoint suite with its reason, and either extend the suffix set or
  state it is deliberately code-only.
- **minor / medium — Channel switching and uninstall leave the first hop dangling**
  (Phase 3 §5, Manual Verification). `docs/releases-and-compatibility.md:21-29`
  documents a channel switch that becomes incomplete, since the shim is
  per-plugin-id; and the "dangling link stays in plugin-owned space" criterion is
  true only of the second hop. Suggestion: add a re-point/remove step to both
  procedures and soften the criterion to what is actually verified.
- **suggestion / medium — The argv scan widens `--fail-safe` into a global
  reserved token** (Phase 1 §2). A value or a post-`--` occurrence enables
  degradation; a future sub-binary is barred from different semantics; and
  because the flag is consumed in `fail()`, "could not fetch and verify" now
  exits 0 — contradicting `bin/accelerator:7`'s "Fail-closed throughout".
  Suggestion: stop at the first `--`, record the token in the header, and update
  that line.
- **suggestion / low — The `${CLAUDE_PLUGIN_DATA}` reversal deserves an ADR; the
  rename does not** (Phase 3 Overview). No ADR pins the CLI's variable name
  (`ADR-0052:149` names `${CLAUDE_PLUGIN_ROOT}` only as the skill-content
  substitution token, which this work preserves). The reversal adds the plugin's
  first dependency on a host facility whose availability at the declared floor is
  unknown, against ADR-0046's zero-setup framing, and the decision it reverses
  lives only in a plan's Key Discoveries. Suggestion: record it as a short ADR or
  an ADR-0046 supplement, stating the floor it assumes and the inert-when-unset
  fallback.

### Security

**Summary**: The plan is unusually security-aware for a bootstrap change: it makes
the bootstrap write-only for `ACCELERATOR_PLUGIN_ROOT` (so no ambient value can
redirect the trust root), keeps the fetch/verify/exec chain fail-closed, tests
ambient-root rejection in both directions, and deliberately confines the new shim
to plugin-owned space rather than writing `~/.local/bin`. Two structural
concerns remain. First, the derived root supplies *both* the release public key
and the verify shim *and* the cache the launcher is exec'd from, with no
integrity binding back to the shipped bootstrap — so relocating the entry point
relocates the entire trust anchor, and Phase 3 makes a symlink chain the
documented invocation path. Second, Phase 1's `--fail-safe` change funnels every
gate — including signature-verification failure, a missing public key and a
refused dev launcher — into a silent `exit 0` at 204 `!` sites where stderr is
invisible, giving a tampering or verification-suppression attempt exactly the
same observable signature as the ~86 commands that legitimately emit nothing.

**Strengths**:

- Strict directionality is a genuine security improvement over the env-var
  design; an ambient or stale variable can no longer redirect the key, shim,
  cache or exec'd launcher, and the plan tests this explicitly.
- The chase reuses a previously reviewed specification rather than being
  re-derived.
- The Phase 1 fixture keeps test key material out of the shipped tree (keypairs
  in `tmp_path`, `*.key` gitignored, fixture root carries the *test* public key,
  `ACCELERATOR_RELEASE_BASE_URL` pinned to `example.invalid`, downloader
  injected), so the production trust path is not exercisable.
- The plan identifies, rather than discovers late, that the two deleted tests
  would perform a real `curl` and a working-tree write.
- Phase 3 refuses to write `~/.local/bin` from an unprompted hook, for the right
  reasons, and asserts containment via a snapshot diff.
- Phase 2 turns a convention into an enforced invariant, wired into the roll-up
  CI runs, backed by a unit assertion, and fails closed on an empty match set.
- Phase 5 removes a real control suppression: `known_skill_names` degrading to
  `Ok(vec![])` silently disables skill-name validation.
- The plan preserves the hardening it inherits: content-addressed shim staging,
  shim invoked by absolute path, no eviction before a verified successor,
  PID-owner lock reclaim.

**Findings**:

- **major / high — The key and verify shim are read from the runtime-derived root,
  so relocating the entry point relocates the trust anchor** (Phase 1 §2; Phase
  3). One derived path supplies the key, the verifier and the cache. A chase
  landing in a prepared tree with its own `plugin.json`, key and shim passes every
  gate and execs a launcher the attacker signed — minisign succeeds against the
  attacker's key. This is pre-existing rather than a regression (and write-only is
  an improvement, since an ambient variable was easier to inject), but Phase 3
  makes the symlink shape the documented, hook-refreshed default and adds no
  integrity check. Suggestion: follow the launcher's own precedent
  (`cli/launcher/build.rs` embeds the key; `resolve/keys.rs` verifies against the
  embedded copy) — add a literal expected SHA-256 to `bin/accelerator`, pin it in
  `test_bootstrap_coverage.py`, and test that a relocated bootstrap facing a
  different key is refused.
- **major / high — `--fail-safe` turns verification failure into a silent exit 0**
  (Phase 1 §2). Five of 14 gates are integrity gates: missing verify shim
  (`:72`), missing public key (`:74`), unhashable/un-stageable/un-chmod-able shim
  (`:163,:168,:170`), refused dev launcher (`:134`), and `fetch_and_verify`
  failure (`:254`). All 204 `!` commands carry the flag and stderr is invisible
  there (`:116`). Nothing unverified is exec'd, so this is a detection/audit
  failure plus an availability lever: all 43 skills inject *empty* context and the
  model proceeds on missing configuration believing it has it. Suggestion: split
  `fail()` into two classes mirroring `Failure::Read`/`Failure::Refusal`
  (`cli.rs:414-418`); keep degradation for environment gates and either close
  integrity gates or degrade loudly via
  `${cache_dir}/.accelerator-unverified.log` plus a stdout marker; add a test
  distinguishing the two cases.
- **major / medium — The SessionStart symlink writer specifies no clobber
  semantics** (Phase 3 §2). `ln -sf TARGET LINKPATH` where `LINKPATH` is already
  a symlink to a *directory* creates the link **inside** it, so a pre-planted
  link makes the hook write outside plugin space — defeating the containment the
  snapshot test establishes (the test only creates that shape if written to). A
  regular file there is neither preserved nor refused, the same clobber risk the
  work item cites against `~/.local/bin`. The shim is also a predictable
  well-known path the user's `PATH` points at indefinitely. Suggestion: `ln -sfn`
  (or `rm -f` only after confirming symlink-or-absent), refuse a non-symlink with
  a stderr diagnostic, `mkdir -p` then explicit `chmod 0755`, and add both
  hostile-precondition cases.
- **major / medium — Deleting the two tests removes today's hazard but installs no
  guard** (Phase 1 §1). The module still exposes
  `_BOOTSTRAP = _REPO_ROOT / "bin/accelerator"`, so any future test running it by
  repo path is implicitly production-rooted, and omitting the injected downloader
  or the `.invalid` base URL is a one-line mistake. `.gitignore` covers
  `mise.local.toml`, `cli/target/` and `*.key` but not
  `bin/accelerator-launcher-*`, the digest-suffixed staged shims,
  `bin/.accelerator-lock-*` or `bin/.accelerator-unverified.log` — so a downloaded
  binary sits untracked in the directory the plugin ships from. Suggestion: an
  autouse `conftest.py` fixture forcing an unroutable base URL and requiring the
  injected downloader, a session-scoped assertion that `bin/` gained nothing, the
  `.gitignore` entries, and replacing `_BOOTSTRAP` with a copy-into-`tmp_path`
  fixture.
- **major / medium — `cd "$(dirname "${self}")"` can silently resolve to the cwd**
  (Phase 1 §2). Neither `cd` uses `--` and neither guards an empty inner result.
  `dirname` receiving an argument beginning with `-` (taken verbatim from
  `readlink`) errors and prints nothing; `cd ""` **succeeds as a no-op in bash**
  and `pwd -P` yields the cwd — for a skill-invoked bootstrap, the user's
  project, which a merely-cloned repo could turn into the trust anchor. Also:
  `$(readlink …)` strips trailing newlines, and root derivation now depends on
  PATH-resolved `readlink`/`dirname`, oddly beside the file's own hardening at
  `:156-158` (run the shim by absolute path so a PATH-planted decoy cannot stand
  in). Suggestion: `cd -- "${self%/*}"` (parameter expansion, no `dirname`),
  reject empty/unchanged results with a named `fail`, assert absoluteness, and add
  cases for a leading `-` target, a spaced target, and a root lacking
  `.claude-plugin/plugin.json`.
- **minor / medium — `${CLAUDE_PLUGIN_DATA}` is validated only for non-emptiness**
  (Phase 3 §2). A relative or `.`-prefixed value composes against the hook's cwd
  — the user's project directory — creating `<project>/…/bin/accelerator` outside
  plugin space, in a tree that may be committed. The snapshot test always supplies
  an absolute temp path. Suggestion: require absolute
  (`case "${CLAUDE_PLUGIN_DATA}" in /*) ;; *) exit 0 ;; esac`) and add a relative
  case.
- **minor / medium — `--fail-safe` is honoured anywhere in argv, wider than the
  launcher's own parse** (Phase 1 §2). Positions the launcher would reject still
  switch the integrity gates to exit 0. The 204 static literals bound the
  skill-injection route; the residual route is the ~20 helper scripts and external
  subcommands passing caller- or model-supplied values through. Suggestion: stop
  at the first `--` and add a test pinning where the flag is and is not honoured —
  largely dissolved by the two-class `fail()` split.
- **minor / medium — Phase 4 does not say whether extracted commands go through a
  shell** (Phase 4 §2). `!`-block content is arbitrary shell text, so a
  `shell=True` harness turns any SKILL.md into an execution vector inside the test
  process with the working tree and CI credentials in scope; the plan's own
  measurement means refusing shell semantics loses nothing. Suggestion: state
  `shlex.split` with `shell=False`, and report any command containing shell
  metacharacters via the existing decline channel.
- **suggestion / medium — The documented recipe should name its two integrity
  preconditions** (Phase 3 §5). The chosen `PATH` directory becomes part of the
  trust chain for a command that fetches and execs signed binaries (a group- or
  world-writable `/usr/local/bin` is common), and the link is not removed on
  uninstall, leaving an `accelerator` command on `PATH` pointing at a predictable,
  no-longer-refreshed path. Suggestion: recommend a user-owned,
  non-group-writable directory and add a "removing it" line.
- **suggestion / high — Confirm the degradation path keeps absolute paths out of
  the prompt** (Phase 5). An absolute path in a diagnostic is proportionate here,
  but anything reaching stdout at a `!` site is spliced into the prompt and
  forwarded to the model API. Suggestion: assert the rootless `--fail-safe`
  failure emits nothing on stdout. Removing the `known_skill_names` fallback is a
  security improvement worth stating as such.

### Code Quality

**Summary**: The plan is unusually well-grounded — measured behaviour, reused
prior art, root-injected pure functions for the new guard, and a bootstrap
snippet that carries no comments at all (the notes live in the plan, which is
exactly right for this codebase). The maintainability risks cluster in three
places: Phase 5 understates a constructor-signature ripple and has no error
variant that can carry the failure it promises, Phase 2's scan scope is wrong in
both directions (blind to the nested `.gitignore` that hides the built SPA, and
deny-by-omission on file suffixes), and the promoted `sources()` signature cannot
express either existing caller's extra pruning. Phase 1's bootstrap shape is
sound but leans on a global flag inside `fail()` and duplicates a couple of
literals that will drift.

**Strengths**:

- The bootstrap snippet contains zero comments — the load-bearing rationale lives
  in the plan, matching this repo's very low comment tolerance.
- The chase is not designed from scratch; the plan cites the prior specification
  and the four decisions it already settled.
- Phase 2 keeps the guard testable by design (pure `violations(root)`, thin
  `@task check`, no in-tree sentinels because `test:unit:tasks` races
  `cli:check`).
- Phase 2 §1 refuses to write the walk a third time — the right DRY instinct on a
  helper with a documented failure history.
- Phase 5 goes after the underlying design flaw and correctly identifies all four
  silent-degradation sites.
- The seam re-pointing states the seam's actual intent and adds a
  distinguishable-values pair so the four assertions can no longer pass
  vacuously.
- The Deviations table replaces three criteria with mechanisms matching measured
  renderer behaviour, keeping a falsifiable discriminator each time.
- Test-first is not just asserted — there is a manual step proving the new tests
  are red first.

**Findings**:

- **major / high — Non-optional `plugin_root` forces a constructor and `compose()`
  signature change the plan does not scope** (Phase 5 §2). The field is defaulted
  to `None` by `FileConfigStore::at()` (`store.rs:42-49`) and set by
  `with_plugin_root(Option<PathBuf>)` (`:58`). `config_adapters::compose()`
  (`compose.rs:41`) does not know the root (the launcher applies it at
  `main.rs:161`), the server wraps its already-non-optional root in `Some(...)`
  (`server/src/compose.rs:44`), and there are 17 `FileConfigStore::at(&root)`
  calls in `store.rs`'s tests plus one in `tests/parity.rs:53`. Suggestion: record
  the constructor shape (e.g. `at(root, plugin_root)`,
  `compose(root, policy, plugin_root)`) and note the ~20 mechanical edits.
- **major / high — No `ConfigError` variant can carry "missing installation root"
  with the promised classification** (Phase 5 §1).
  `From<ConfigError> for Failure` (`cli.rs:422-429`) maps only `Invalid` to
  `Refusal`; `cli/config/src/error.rs:38-66` has no variant for an absent root.
  `Invalid` gives the best message but fail-closes; `Io` degrades but renders "I/O
  error on 'ACCELERATOR_PLUGIN_ROOT'". Suggestion: add
  `ConfigError::PluginRootUnavailable { detail }` to the `#[non_exhaustive]` enum
  with its own `Display` arm, and state it stays in the `Read` arm.
- **major / medium — Requiring the root at `compose_stack` makes seven measured
  root-independent families fail** (Phase 5 §1). `compose_stack`
  (`main.rs:149-171`) is the single composition point for all six ports; at a `!`
  site each of the seven degrades to an "unavailable" notice instead of the correct
  answer it can still compute. Suggestion: attach the requirement to the
  consuming port, or record why the measured granularity is discarded.
- **major / high — The promoted walk inherits root-only gitignore blindness and
  will scan the built SPA** (Phase 2 §1). `_ignore_spec()` honours only the root
  `.gitignore` — documented at `sources.py:13-16`, justified there for `.sh` only
  — and `cli/visualiser/frontend/dist/` is ignored *only* by the nested
  `.gitignore` (the root's `/dist/` is root-anchored). So the walk reads
  `dist/assets/index-*.js` and `dist/index.html`, which exist under `mise run`
  (the frontend builds before `cli:check`). Suggestion: per-directory spec
  layering, or prune an explicit build-output set, plus a test asserting no path
  under `dist/`.
- **major / high — `sources(root, suffixes, subtree)` cannot express either
  caller's pruning** (Phase 2 §1). `shell_sources()` needs `_keep`'s
  `workspaces/` rule on both the walk results *and* the `_EXTRA_SHELL_SOURCES`
  extras; `_py_files` prunes `.venv` because it is not in `.gitignore` (it exists
  in this checkout), and `test_walk_nonempty_and_excludes_only_justified` would
  still pass after the loss. Suggestion: add `prune: tuple[str, ...] = ()` or a
  `keep` predicate, and add a regression test per caller.
- **minor / medium — `_SUFFIXES` is deny-by-omission** (Phase 2 §2). `cli/` also
  holds `README.md`, `index.html`, ~45 `*.module.css` and server test fixtures,
  and could tomorrow hold `.sh`, `.yml` or extensionless files — while the
  docstring and the acceptance criterion both claim broader coverage.
  Suggestion: invert the filter — scan every non-ignored file and skip a named
  binary/lockfile set with a reason each.
- **minor / medium — An empty `ALLOWLIST` invites the wrong escape hatch** (Phase
  2 §2). The failure message advertises a whole-path override, so the first
  exemption added under pressure could be `cli/launcher/src/main.rs` — blinding
  the guard to the site that caused this bug. Suggestion: drop it until a real
  exemption arises (and have the message say the reference must be *removed*), or
  keep it as `dict[str, str]` of path → reason so the round-trip test can assert
  the reason.
- **minor / high — `fail()` deciding its exit status from a global flag makes 14
  call sites read dishonestly** (Phase 1 §2). `fail "verify shim missing…"` may
  exit 0 or 1 depending on a variable set ~50 lines away, with the branch inside
  the function body. Suggestion: resolve the policy once into `abort_status`
  (`1`, set to `0` by the scan) and `exit "${abort_status}"`; consider `abort()`
  or `give_up()` if the name should carry it.
- **minor / high — The hop bound is duplicated; the write-once root is
  unenforced** (Phase 1 §2). `-gt 32` and "(exceeded 32 hops)" will drift;
  `${BASH_SOURCE[0]}` is re-expanded three times; and `plugin_root` — whose design
  rests on being written once — is a plain mutable local. Suggestion:
  `max_hops=32` interpolated into the message, hoist
  `source="${BASH_SOURCE[0]}"`, and `readonly plugin_root`.
- **minor / high — Test assertions couple to a harness call-order counter** (Phase
  1 §1). Nothing in a test body explains that `tag=1` means "the first harness
  this test built"; the factory also grows a `real_launcher` flag while still
  returning a bare `(root, server)` tuple. Suggestion: return a small
  `NamedTuple`/dataclass (`root`, `server`, `sentinel`) so tests assert
  `harness.sentinel in result.stdout`; consider a separate
  `make_real_launcher_harness`.
- **minor / high — The `templates list` test pins the exact rendered markdown row**
  (Phase 1 §1; Deviations criterion 3). Column order, backtick quoting and
  single-space padding are none of them what the test is testing, and the plan
  elsewhere rejects path-shaped assertions for being indirect. Suggestion: find
  the line containing `` `adr` `` and assert `plugin default` on it, or reuse the
  sentinel via `config template adr`.
- **minor / high — Several prescribed snippet comments restate what the code says**
  (Phase 1 §1, §6). `# No CLAUDE_PLUGIN_ROOT, no ACCELERATOR_PLUGIN_ROOT, no
  --fail-safe.`, `# The exact reported failure. …` (a work-item reference that
  goes stale), and the two-fixture-roots/parametrised note (both halves expressed
  by the decorator and the test name) do not clear the bar; the `ELOOP` note and
  the chain diagram do. Suggestion: move them into test and helper names (e.g. a
  `_run_bootstrap_rootless` wrapper) and drop the reported-failure reference.
- **minor / medium — `shim-refresh.sh` collides with the established "shim"
  vocabulary** (Phase 3 §§1–3). In `bin/accelerator` "shim" means the vendored
  per-triple minisign verifier, referenced a dozen times and in the file header.
  A maintainer reading a `shim-refresh` failure will assume the verification chain
  is broken, and the overload reaches published documentation. Suggestion: name
  the hook for what it maintains (e.g. `hooks/bin-link-refresh.sh`), reserving
  "shim" for the verifier.
- **suggestion / medium — The conformance harness re-derives the plugin-root
  prefix** (Phase 4 §2). It reuses two functions from `skill_permissions.py` but
  not `_PLUGIN_PREFIX` (`:55`), so a third literal copy is the likely outcome —
  desynchronising the guard from the suite on a future prefix change. Suggestion:
  promote it to a public `PLUGIN_PREFIX`.
- **suggestion / medium — `_LAUNCHER_DEPENDENTS` pins only the positive list**
  (Phase 1 §7). A new `test:integration:*` task needing a launcher and added to
  neither `mise.toml` nor the list ships green — exactly today's failure.
  Suggestion: enumerate every `test:integration:*` task and require each to be in
  `_LAUNCHER_DEPENDENTS` or an explicit `_NO_LAUNCHER_NEEDED` set with a reason.
- **suggestion / medium — Phase 1 folds unrelated cleanup into the largest diff**
  (Implementation Approach; Phase 1). Seven change-sets across ~25 files,
  including an explicitly "opportunistic" `ACCELERATOR_MIGRATION_MODE` removal,
  with no commit structure proposed — so the shell change, where the risk is, gets
  skimmed. Suggestion: state the commit sequence (tests → bootstrap → readers →
  writers → seam → build edge) and move the cleanup out.

### Standards

**Summary**: The plan is unusually disciplined about convention compliance — its
exec-bit treatment, `ACCELERATOR_*` variable naming, hooks.json
append-at-index-3, lint-guard shape (root-injected `violations()` +
`violations(REPO_ROOT) == []`), and gitignore-pruning-not-rglob discovery all
match what the tree actually requires, and the large majority of its ~40 line
citations verify correct. The residual problems are concentrated in four places:
two new files are placed against the repo's established file-organisation
conventions, three paired-test/registration conventions are silently skipped, one
Phase 2 success criterion is unsatisfiable as written, and `hooks/shim-refresh.sh`
collides with the repo's already-established meaning of "shim". A handful of line
citations have drifted enough to misdirect an implementer.

**Strengths**:

- Exec-bit invariant is exactly correct: both new `.sh` files declared 0755 with
  the mode committed and explicitly *not* in `SHELL_LIBRARIES`, matching
  `tasks/lint/scripts.py`'s entrypoint-by-default rule, so no update to
  `_RECONCILED_LIBRARIES` in `test_exec_bits.py` is needed.
- No test-suite registry update is needed for the new hooks suite, and the plan
  says so: `run_shell_suites` (`tasks/test/helpers.py:62-68`) globs
  `<subtree>/**/test-*.sh` filtered on the exec bit, and `EXCLUDED_HELPER_NAMES`
  needs no entry.
- `ACCELERATOR_PLUGIN_ROOT` fits the existing namespace cleanly.
- The hooks.json claim verifies exactly: three SessionStart groups (0–2), house
  style one hook per group with a literal un-expanded command, and a repo-wide
  search confirms `.hooks.SessionStart[0]` in `test-vcs-detect.sh:615-634` is the
  *only* hard-coded SessionStart index anywhere.
- Python tool scoping needs no change: pyrefly's `project-includes` is rooted at
  `tasks/**` (`pyproject.toml:148-151`) and ruff's per-file-ignores cover
  `tests/**` (`:111`), and `test_python_coverage.py` pins both config sets.
- The guard's shape matches `store_duplication.py` precisely, and
  `test_store_duplication.py:21` really does use
  `REPO_ROOT = Path(__file__).resolve().parents[3]` with
  `violations(REPO_ROOT) == []`.
- The tab indentation in the `bin/accelerator` snippet is *correct*:
  `.editorconfig`'s `[*.sh]` does not match the extensionless file, so shfmt
  falls back to tabs and no `-ci`, which is what the file already uses.
- `test:integration:hooks` (`mise.toml:204-206`), `test:integration:tasks`
  (`:160-163`) and `tests/integration/tasks/` all exist.
- The `cli/` inventory matches a live grep exactly — all 21 hits across 11 files,
  including the three assertion sites called out as observable contract.

**Findings**:

- **major / high — Phase 2's `grep -rl` residue list is unsatisfiable** (Phase 2
  Success Criteria). `meta/**` is neither gitignored nor listed, yet dozens of
  tracked files there name the variable (this plan included), and
  `tests/integration/entrypoint/test_accelerator_entrypoint.py` is omitted while
  Phase 1 deliberately keeps the string there. `hooks/test-fixtures/vcs-detect/
  regenerate.sh` and `hooks/test-migrate-discoverability.sh` are covered
  incidentally by `hooks/**`. Impact: the check has no discriminating power.
  Suggestion: scope the grep to exclude `meta/` and add the entrypoint suite with
  its reason.
- **major / high — Adding the guard to both `cli:check` and `lint:check`
  contradicts the precedent it names** (Phase 2 §3). `lint:store-duplication:check`
  is *not* in `lint:check.depends` — nor is `lint:vendor-shims:check`;
  `mise.toml:451` lists exactly the seven component/skill guards. The established
  split is `cli/`-scoped guards in `cli:check` only. Because bare `default`
  depends on `lint:check` (not `check`), dual-wiring would make this the sole
  `cli/`-scoped guard reachable from the default task — a third pattern.
  Suggestion: wire into `cli:check` only; if the default-task blind spot is worth
  closing, add all three `cli/`-scoped guards together as a separate change.
- **major / high — The new `_EXPECTED_HOOKS_SUITES` floor omits the paired guard
  test every other floor has** (Phase 3 §4). All four existing constants have a
  guard class in `tests/unit/tasks/test_integration.py`
  (`TestConfigSuiteGuard`, `TestMigrateSuiteGuard`, `TestWorkSuiteGuard`,
  `TestIntegrationsSuiteGuard`), each asserting `pytest.raises(Exit)` below
  baseline. Suggestion: add `TestHooksSuiteGuard` following the
  `TestWorkSuiteGuard` shape and add that file to Phase 3's criteria.
- **major / high — Rewriting `shell_sources()` skips its own regression suite**
  (Phase 2 §1, Success Criteria). `tests/unit/tasks/shared/test_sources.py` —
  which module-mirrors the file, has nine cases and imports the private `_keep` —
  is named nowhere, and the plan never says where `sources()`'s own tests live
  though the `tests/unit/tasks/shared/test_<module>.py` convention determines it.
  Suggestion: name it as both the home and a required criterion.
- **major / high — `tests/fixtures/skill-conformance-project/` breaks the fixture
  co-location convention** (Phase 4 §1). No top-level `tests/fixtures/` exists;
  every Python-test fixture tree is co-located
  (`tests/unit/tasks/fixtures/tiny_binary.bin`,
  `tests/integration/deny/fixtures/{banned,clean,…}/`), and the shell side uses
  `<subtree>/scripts/test-fixtures/`, pinned as `_FIXTURE_SEGMENT` at
  `tasks/lint/scripts.py:57`. Suggestion: place it beside its suite.
- **major / high — A SKILL.md conformance suite under `tests/integration/tasks/`
  contradicts that directory's and the mise task's scope** (Phase 4 §2, §3). That
  directory holds integration tests of invoke task modules (`test_github.py` →
  `tasks/github.py`, `test_release.py` → `tasks/release.py`), and the task's own
  description is "Run pytest integration tests for invoke tasks"
  (`mise.toml:161`). Suggestion: give it its own topic directory and mise leaf
  (e.g. `tests/integration/skills/` + `test:integration:skills`), or fold it into
  `tests/integration/entrypoint/`.
- **major / high — "shim" already means the minisign verify binary** (Phase 3
  §§1–3, §5). See `bin/accelerator`'s `shim_source`/`shim_digest`/`shim` and its
  `verify shim missing for …` error, the committed
  `bin/accelerator-verify-<platform>` triples, `tasks/lint/vendor_shims.py`,
  `lint:vendor-shims:check`, `build:vendor-verify-shims`, and the entrypoint
  suite's `shim_bin` fixture — and Phase 1 of this plan uses the term that way.
  Suggestion: rename to `<subject>-<action>` naming the thing (e.g.
  `hooks/launcher-link-refresh.sh`) and use "link" in the docs section.
- **minor / high — Five line/file citations have drifted** (multiple phases). Most
  verify correct (`mise.toml:392-395`, `:409`, `:451`, `:465-467`;
  `docs/internals.md` line 88; `README.md:57-58`; `.gitignore:20-21,25`;
  `hooks/test-vcs-detect.sh:615-634`; `tasks/lint/scripts.py:18-49`;
  `test_exec_bits.py:288-292`; the whole `bin/accelerator` map;
  `sources.py:40-48`; `test_python_coverage.py:68-94`;
  `.github/workflows/main.yml:70-71,81-88,257`). Drifted: (1)
  `test:integration:entrypoint` is `mise.toml:179-182`; `:184-187` is
  `test:integration:config`, which *already* has `build:cli:dev`. (2)
  `preprocessor_commands`/`is_plugin_invocation` are at
  `skill_permissions.py:94-96,99-101`; `:104-111` is `covered_by`. (3)
  `scripts.py:130-133` is the `chmod +x` branch — `chmod -x` is `:128-129`. (4)
  `test_bootstrap_coverage.py:39-43` pins only `keys/accelerator-release.pub`, not
  `.accelerator-dev-launcher`. (5) "11 remaining reads" should be the 8
  enumerated. Also "`check` depends on the seven `<component>:check` roll-ups" is
  five roll-ups plus two entity tasks per `tasks/README.md:32-36`. Suggestion:
  correct them, and prefer symbol names over line numbers where unique.
- **minor / medium — `## Terminal invocation` breaks the file's Title Case
  convention** (Phase 3 §5). All three existing `##` headings in
  `docs/internals.md` are Title Case (`## The meta/ Directory` `:3`,
  `## Agents` `:39`, `## VCS Detection` `:66`), as is `docs/configuration.md`
  throughout; the grep criterion makes the deviation the tested contract.
  Suggestion: `## Terminal Invocation`, and update the grep.
- **minor / high — Code snippets do not match enforced formatting** (Phase 1 §1,
  §6). (1) The `test-work-item-scripts.sh` addition uses a **tab** where
  `.editorconfig`'s `[*.sh]` mandates 2 spaces and the surrounding file is
  2-space — and shell has no autofixer. (2) The Python snippets are not
  `ruff format` output: the `cargo build` argument list is compressed where ruff
  explodes it one-per-line (see the real `shim_bin` fixture at
  `test_accelerator_entrypoint.py:136-149`), and a backslash continuation is
  rewritten to parentheses. Suggestion: re-indent the shell snippet, note
  explicitly why `bin/accelerator` uses tabs while `.sh` uses spaces, and run the
  Python through `ruff format`.
- **minor / medium — `for arg in ${1+"$@"}` has no precedent and no ShellCheck
  justification** (Phase 1 §2). Absent from the ~175-file shell corpus outside
  `meta/`; the precedented guard is an explicit count test
  (`scripts/lint-bashisms.sh:23`, `skills/integrations/jira/scripts/jira-jql.sh:55`).
  `.shellcheckrc` sets `enable=all` with `SC2086` not disabled, and the snippet
  carries no justified disable. Suggestion: prefer
  `if [ "$#" -gt 0 ]; then for arg in "$@"; …`, or verify clean first and pair
  with a justified disable naming the bash-3.2 `set -u` reason.
- **minor / medium — The guard's suffix allowlist leaves `.md`, `.sh`, `.py`,
  `.mts`/`.cts` and `.yml` under `cli/` unguarded** (Phase 2 §2). It covers
  today's hits exactly (only `.rs` and `.mjs`), but the stated boundary is
  broader, and `.editorconfig:14-17` adds `mts`/`cts` precisely so a future
  ESM-typed file is covered. Suggestion: widen to the safe text set, or narrow the
  docstring to the scanned suffixes with a reason.
- **minor / high — "the `hooks` subtree is the only test-bearing subtree with no
  floor count" is false** (Phase 3 §4). `decisions` (`skills/decisions`, one
  suite) and `github` (three discoverable suites) also call `run_shell_suites`
  with no guard. Suggestion: correct to "one of three", and either add both floors
  in the same edit (three lines each) or record the gap deliberately.
- **minor / medium — The 43-per-family count conflicts with
  `EXPECTED_INJECTION_SKILLS = 42`** (Phase 4 Overview and table).
  `skill_permissions.py:36-39` pins it as an exact equality with a comment that
  "the equality is what catches an accidental loss", over the same corpus the
  harness reuses. Suggestion: re-measure and derive Phase 4's family sizes from
  that constant.
- **minor / medium — Three registration conventions are unaddressed** (Phase 2 §3;
  Phase 3 §5). (1) `tasks/lint/__init__.py` keeps its import tuple and `__all__`
  alphabetical (`claude_coupling` sorts between `call_site_migration` and `cli`).
  (2) Every `ns_lint.add_collection(...)` in `tasks/__init__.py:84-113` carries a
  trailing comment naming the task path. (3) `tasks/README.md` — the designated
  "learn the shape once" reference — already under-describes `cli:check` as
  "format + lint (rustfmt, workspace-wide clippy)", omitting the two Python
  guards already wired in; Phase 2 adds a third and Phase 3 adds a floor whose
  convention is documented there, and neither touches the file. Suggestion: add
  `tasks/README.md` to Phase 2's file list and state both mechanical conventions.
- **suggestion / medium — A third site names the v2.1.144 floor** (Phase 0 §2).
  `meta/decisions/ADR-0051-skills-as-the-product.md:117` states it as a live fact
  ("currently v2.1.144"). Suggestion: if the floor rises, update the parenthetical
  or reword it to point at `docs/releases-and-compatibility.md` as the single
  source of truth.
- **suggestion / medium — `mise.local.toml` is already gitignored** (Migration
  Notes; Closing step). `.gitignore:26` lists it — the repo's convention for
  machine-local mise overrides — so it cannot be accidentally added under git and
  is not auto-snapshotted under jj unless force-tracked; and `.gitignore` is
  itself an uncommitted modification in the current working copy, so it is unclear
  whether the entry predates this work. Suggestion: note the entry explicitly,
  state whether it is pre-existing, and if so replace the per-push `git cat-file`
  ritual with a one-line statement that the ignore entry is the guard.

---

## Re-Review (Pass 2) — 2026-07-27

**Verdict:** REVISE

All eight lenses re-ran against the revised plan, reviewing it fresh rather than
being shown pass 1, so "did the edits introduce new problems" is an honest signal.

**The pass-1 structural findings are resolved.** Phase 5's re-siting away from
`compose_stack` was independently re-derived and confirmed correct by three
lenses; `run_in` is untouched so the 47 collateral tests are safe; the 16-hop
bound's arithmetic and platform reasoning check out on both kernels; the 1a/1b/1c
split, the widened guard scope, the `walk_files` promotion, the `_EXPECTED_HOOKS_SUITES`
guard, the fixture co-location, the "shim" rename and the honest mergeability
table all survived scrutiny.

**But the revision introduced four new defects and surfaced three deeper ones the
first pass missed.** Three are critical. Two of the new defects are directly
attributable to pass-1 edits: the bootstrap snippet now uses a *logical* `cd ..`
where its own prose specifies `cd -P`, and the 1c-after-1b ordering makes the
work-item seam assertions tautological in the window between them.

### Previously Identified Issues

#### Resolved

- ✅ **Correctness + Architecture**: Phase 5's `--fail-safe` degradation path does
  not exist — **Resolved.** Re-derived independently: "re-siting the requirement at
  the consumers is the right conclusion from that trace."
- ✅ **Correctness**: `config_read.rs:62` is the suite-wide runner — **Resolved.**
  `run_in` untouched; the 47 hygiene cases confirmed safe. (But see the new
  `config summary` finding — the same class, a different family.)
- ✅ **Correctness + Test Coverage + Portability**: the symlink hop counter is
  unreachable — **Resolved.** The 16/17 boundary is confirmed correct on Darwin
  (`SYMLOOP_MAX` 32) and Linux (40), and the loop's increment-before-compare makes
  16 pass and 17 fail with the stated message.
- ✅ **Architecture + Code Quality**: Phase 5 breaks seven root-independent
  families — **Resolved** for those seven. An eighth, unmeasured family surfaced
  instead.
- ✅ **Correctness + Test Coverage**: three of four degradation removals do not
  follow — **Resolved** as a claim (the plan now states its limits), but the
  underlying blocker is worse than recorded: `template_names` cannot carry the
  error at all.
- ✅ **Code Quality**: the constructor/`compose()` ripple — **Resolved.** Keeping
  `Option` on the struct leaves `at()`, `with_plugin_root`, both `compose()`
  functions and ~19 call sites untouched.
- ✅ **Code Quality**: no `ConfigError` variant fits — **Resolved.**
  `PluginRootUnavailable` specified; compatibility confirms it is API-safe
  (`#[non_exhaustive]`, in-crate `Display`, the one cross-crate match already has a
  `_ => Read` arm).
- ✅ **Architecture**: phase independence overstated — **Resolved.** The table is
  now accurate; architecture argues it is if anything over-constrained.
- ✅ **Compatibility**: the lockstep guarantee — **Resolved** for the three
  enumerated escapes. A fourth exists.
- ✅ **Compatibility**: `ACCELERATOR_MIGRATION_MODE` is not dead — **Resolved.**
  Keeping the assignment confirmed as the right call.
- ✅ **Test Coverage**: 86 assertions reduce to empty stdout — **Resolved.** The
  fixture now configures every skill in the corpus.
- ✅ **Test Coverage + Security**: the two deleted tests' lost coverage —
  **Resolved.** The `plugin.json`-gate test covers what they protected.
- ✅ **Test Coverage + Architecture**: `hooks.json` registration not in CI —
  **Resolved.** Content-based assertion moved into the suite.
- ✅ **Test Coverage + Correctness**: "fails closed on an empty match set" —
  **Resolved.** Discovery-versus-violations distinguished, with a file-count floor.
- ✅ **Architecture + Code Quality + Standards**: the guard cannot see
  `bin/accelerator`; `lint:check` wiring; `test_sources.py` skipped; fixture
  placement; `tests/integration/tasks/` topic; "shim" collision — **all Resolved.**
- ✅ **Standards**: `_EXPECTED_HOOKS_SUITES` lacked a paired guard — **Resolved.**

#### Partially resolved

- 🟡 **Security + Correctness**: `--fail-safe` makes integrity failures silent —
  **Partially resolved.** The durable record is specified, but security finds it
  has *no reader anywhere in the repository*, its path is caller-controllable via
  `ACCELERATOR_CACHE_DIR`, the `|| true` swallows append failures, and `$1`
  interpolation allows newline forgery. The empty-stdout consequence is also
  re-raised as a policy-downgrade lever.
- 🟡 **Security + Portability + Correctness + Test Coverage**: hook symlink
  mechanics — **Partially resolved.** `ln -sfn`, the non-symlink refusal and the
  absolute-path guard all landed, but the `mv -f` that was added for atomicity
  reintroduces the symlink-to-directory trap it was meant to close.
- 🟡 **Correctness + Test Coverage + Portability + Compatibility**: the
  launcher-supply mechanism — **Partially resolved.** Phase 1a and Phase 4 now name
  the fixture route, but Phase 4's fixture needs a verify shim `build:cli:dev` does
  not build, the apparatus is module-private with no shared home, and the Phase 1a
  `conftest.py` guard cannot bind to a suite that builds explicit `env=` dicts.
- 🟡 **Compatibility**: three competing `ln -s` recipes — **Partially resolved.**
  The reconciliation is prescribed, but the prescribed target does not expand in
  either context where it is used.
- 🟡 **Compatibility**: the `${CLAUDE_PLUGIN_DATA}` floor hedge — **Partially
  resolved.** The stderr line landed; compatibility now argues the floor raise
  itself is disproportionate for an inert convenience.
- 🟡 **Test Coverage**: the empty-string root case — **Partially resolved.** Now
  specified, but vacuous at Phase 1b (both branches yield `vec![]` in a bare
  tempdir).
- 🟡 **Correctness + Code Quality + Architecture**: `sources()` pruning —
  **Partially resolved.** `.venv`/`dist` covered; `playwright-report/` is a fourth
  nested-ignored tree, the additive `prune` default is undiscoverable from the
  signature, and whether `root` may be a subtree is ambiguous in a way that
  silently defeats the prune.
- 🟡 **Standards + Compatibility**: the purge residue list — **Partially
  resolved.** `meta/` excluded and the entrypoint suite added, but it still omits
  `skills/work/create-work-item/evals/benchmark.json` and the two files Phase 2
  itself creates.

#### Still present

- 🔵 **Architecture**: the two-hop link remains channel-global shared mutable state
  with an explicitly declined downgrade guard. Documented as agreed, but security
  escalates it: what the link selects is the whole trust root, so a stale session
  can silently roll back a security fix.
- 🔵 **Security**: the trust-anchor decision stands as agreed. Security now judges
  the *reasoning* partly overstated — the privilege-gain argument does not cover
  directory-level mis-derivation, which is exactly the new logical-`cd` finding.

### New Issues Introduced

#### Critical

- 🔴 **Correctness + Architecture + Code Quality**: `template_names` cannot carry
  the new error, so Phase 5's headline criterion is unsatisfiable.
  `ReadTemplate::template_names(&self) -> Vec<String>` (`cli/config/src/service.rs:328`)
  is infallible, and the plan says so itself two subsections later while
  simultaneously listing `:356` as a consumer that returns the error. Because
  `template_view::list` derives every row from that call, zero rows means
  `resolve_template` is never reached, so "a rootless `config templates list` is a
  named error rather than an empty table" cannot be satisfied at any site. The
  ripple is wider than the plan records: `available`, `available_or_none` (both
  `-> String`, used to *build* error messages), `eject_all`, and the visualiser
  server's `compose.rs:157`.
- 🔴 **Test Coverage + Correctness + Architecture**: Phase 5's empty-stdout
  assertion contradicts `Degrade::Notice`. `finish` (`cli.rs:462-468`) does
  `eprintln!` **and** `render::emit(&unavailable())`, and every affected action is
  wired `Degrade::Notice` (`:220,:225,:230`); only `Summary` uses
  `Degrade::Suppress`. The existing test
  `templates_list_with_fail_safe_renders_the_unavailable_notice`
  (`config_read.rs:1504-1514`) pins `stdout == "## Template Unavailable\n"`. The
  plan's own Phase 5 rationale cites this same arm as a *cost* of the
  `compose_stack` siting, so the two sections take opposite positions.
- 🔴 **Compatibility + Correctness + Portability**: the documented recipe expands
  `${CLAUDE_PLUGIN_DATA}` in two contexts that never set it. `visualise/SKILL.md:162`
  is a `!` preprocessor site — the founding premise of this work item is that
  plugin variables are not exported there — and the user's terminal has no Claude
  Code process at all. Both would render `ln -s "/bin/accelerator" …`, creating a
  dangling link on `PATH` with `ln` reporting success. Phase 0 §2 determines only
  whether the variable *exists* at the floor, never *where it is visible*. The
  phase's own `grep` criterion passes on the broken variant.

#### Major

- 🟡 **Security**: root derivation uses a logical `cd ..`, not `cd -P` — introduced
  by the pass-1 edit. `cd -- "$(dir_of "${self}")/.."` collapses `..` textually, so
  when the final component is a directory symlink the result is the *link's* parent.
  The plan's own prose specifies `cd -P`; the absoluteness check cannot catch it
  because the mis-derived value is still absolute. This is the directory-level
  mis-derivation the trust-anchor decision's privilege-gain argument does not cover.
- 🟡 **Test Coverage**: landing 1c after 1b makes the seam assertions tautological —
  introduced by the pass-1 split. Once 1b renames `accelerator_env()`, the old
  `CLAUDE_PLUGIN_ROOT="/nonexistent"` is inert, the CLI succeeds, and the four
  assertions plus three Test 8 tripwires compare the shipping template with itself.
  The plan already says 1c "needs neither 1a nor 1b" — it should simply precede 1b.
- 🟡 **Correctness + Compatibility + Architecture**: Phase 5 makes `config summary`
  root-requiring — an unmeasured eighth family. `known_skill_names`' sole caller is
  `summary.rs:141`; `config instructions` never touches it (it validates via
  `validate_identifier` at `core/context.rs:54`), so the stated
  `config instructions <bogus-skill>` criterion is unsatisfiable. Meanwhile
  `config_read.rs:1072` and `:1081` run `summary` rootless through `run_in` and
  assert exit 0 plus a golden — both go red, contradicting "breaks nothing". And
  `config summary --format=hook` is the `SessionStart` contract
  (`hooks/config-detect.sh:13`).
- 🟡 **Portability + Test Coverage + Standards**: `test:integration:skills` is never
  added to `test:integration.depends` (`mise.toml:228-243`), which is what CI runs.
  The list is curated (`test:integration:pup` is deliberately absent) and nothing
  pins membership, so the 204-command suite could ship green and never execute.
- 🟡 **Correctness + Portability**: `mv -f` reintroduces the symlink-to-directory
  trap. Both BSD and GNU `mv` `stat()` the destination and, if it resolves to a
  directory, move the source *inside* it — and GNU's `-T` opt-out does not exist on
  macOS. A symlinked destination passes the `[ -e ] && [ ! -L ]` refusal, so the
  hook writes outside `${CLAUDE_PLUGIN_DATA}` and exits 0 reporting success, failing
  the phase's own test case.
- 🟡 **Security + Standards + Correctness**: `bin/accelerator-verify-*-*` matches
  the four committed vendored shims (`*`=`darwin`, `*`=`arm64`). Tracked files are
  unaffected today, but a re-vendoring becomes silently partial — `git add` stages
  only the marker, while `lint:vendor-shims:check` compares the *marker* against a
  recomputed digest and stays green, so a security-motivated `minisign-verify` bump
  could ship with the old verifier binaries.
- 🟡 **Test Coverage + Security**: the autouse `conftest.py` guard cannot bind.
  `_run_bootstrap` (`:225-252`) builds a complete explicit `env=` dict, so nothing
  the fixture sets reaches the child; the deleted tests called `subprocess.run`
  directly. Only the post-hoc `bin/` snapshot works, and that fires after egress.
  Phase 4's suite is outside the fixture's scope entirely.
- 🟡 **Test Coverage + Compatibility + Standards**: Phase 4's fixture needs a verify
  shim that `build:cli:dev` does not build (it builds `--bin accelerator` only), and
  the whole apparatus (`shim_bin`, the keypairs, `_sign`, `_serve_launcher`,
  `_DOWNLOADER_SRC`) is module-private in the entrypoint suite with no shared home —
  so Phase 4 must duplicate ~150 lines of trust-chain harness or extract it, and the
  plan specifies neither. It also contradicts 1a's own cargo-target-lock reasoning.
- 🟡 **Code Quality**: prescribed comments reference plan artefacts. `# transitional;
  removed in Phase 1b` (added in pass 1), `# Replaces the two deleted tests…`,
  `# see Key Discoveries`, and a `_SKIP_SUFFIXES` block asserting an unmaintained
  `~45 .module.css` count all violate this repo's comment rules.
- 🟡 **Code Quality**: the hook prefers ambient `CLAUDE_PLUGIN_ROOT` while the
  bootstrap deliberately ignores it — opposite precedence for the same variable in
  two adjacent layers of one feature, with the wider-blast-radius layer trusting the
  value the other was rewritten to distrust.
- 🟡 **Security**: unconditional symlink-following `chmod 0755` on
  `${CLAUDE_PLUGIN_DATA}/bin` can relax a user's deliberate `0700`, or apply to a
  symlink target — a hardening step that becomes an exposure primitive.
- 🟡 **Security**: `ACCELERATOR_CACHE_DIR` and `ACCELERATOR_RELEASE_BASE_URL` become
  documented user knobs with no trust requirements attached. The former is where
  `probe_dir` writes and *executes* predictable PID-named files; the latter is a
  rollback lever, since the cache key carries no content hash.
- 🟡 **Security**: the fixed terminal path has no downgrade floor. The declined
  backwards-re-point guard means a stale session can roll the whole trust root back
  ~two weeks; `[ "${target}" -nt … ]` is a bash-3.2-safe monotonicity heuristic
  needing no semver parsing.
- 🟡 **Compatibility**: `accelerator visualiser status` cannot be hermetic *and*
  exercise the renamed `cache_root` read — `ACCELERATOR_VISUALISER_BIN`
  short-circuits before `cache_root::resolve`, and the fetch route verifies against
  the compile-time-embedded real release key.
- 🟡 **Compatibility**: a fourth lockstep escape — `ACCELERATOR_VISUALISER_BIN` /
  `visualiser.binary` (documented in `docs/visualiser.md:61` and
  `configure/SKILL.md:580`) pins a server binary entirely outside the version-keyed
  cache, which after 1b dies with exit 2 on the renamed read.
- 🟡 **Compatibility**: raising the plugin-wide Claude Code floor for an inert
  convenience hook withdraws declared support from the installed base this release
  exists to unbreak.
- 🟡 **Portability + Compatibility**: `playwright-report/` is a fourth
  nested-ignored build output under `cli/`, present after any E2E run (which bare
  `mise run` performs), and `_SKIP_SUFFIXES` has no `.zip`/`.map`/`.webm`.
- 🟡 **Code Quality + Security**: `_FILES` erodes silently — four hard-coded paths
  with no existence assertion, in a guard whose stated purpose is to be
  non-negotiable, and whose empty-*discovery* fail-closed rule covers only the other
  half.
- 🟡 **Test Coverage**: the argv scan's two deliberate boundaries (`--`, first-match)
  have no test, despite a paragraph of rationale; and `CDPATH` is manual-only
  despite `extra_env` making it a one-line automated case.
- 🟡 **Test Coverage**: standalone shell-suite runs become network-touching after
  1a — CLAUDE.md documents `bash scripts/test-config.sh`, where `ACCELERATOR_BIN` is
  unset, and the new `.gitignore` entries would *hide* the fetched artefact.
- 🟡 **Standards**: rewording accepted `ADR-0051` violates the repo's own
  append-only immutability rule (`ADR-0031`, `review-adr/SKILL.md:85-100`).
- 🟡 **Standards**: no `CHANGELOG.md` `[Unreleased]` entry is planned, though
  `tasks/release.py:120` generates release notes from it and Migration Notes
  explicitly says the stale-export warning must reach them.

#### Minor

- 🔵 `dir_of` returns `""` for a root-level path (`/accelerator`), where `dirname`
  returned `/` — a regression feeding the `cd ""` hazard it was added to close, and
  bash-version-dependent.
- 🔵 The absoluteness `case` is unreachable dead code: `pwd -P` always prints a
  non-empty absolute path and `CDPATH=` removes the multi-line route.
- 🔵 The cycle test has no `timeout=`, so a non-terminating chase becomes a hung CI
  job rather than a red test; and there is no at-boundary 16-link case, so `-gt` →
  `-ge` reddens nothing.
- 🔵 Phase 1a's stated commit sequence puts the harness support *after* the tests
  that need it, so commit 1 fails with fixture errors rather than assertion
  failures. Phases 2 and 3 state no sequence despite comparable size.
- 🔵 `fail "msg" integrity` is a positional flag argument; a typo silently disables
  the record. A named `fail_integrity()` puts the intent in the verb.
- 🔵 `unverified_log` duplicates `resolve_cache_dir`'s precedence without its probe,
  and the log filename becomes a third literal in the file.
- 🔵 `_SKIP_SUFFIXES` is a denylist standing in for "readable as text", with no
  `UnicodeDecodeError` path — a `.ttf`/`.wasm` under `cli/` crashes `cli:check`.
- 🔵 The structural corpus invariants are weaker than claimed: "every family
  non-empty" is satisfied by one command per family, so the 86-command family could
  shrink to 2 unnoticed. The decline channel is logged, not asserted, though
  `skill_permissions.py` already guarantees it is empty.
- 🔵 The hook suite's `run_hook()` inherits the ambient environment, so both "unset"
  cases would run with the variable *set* on the maintainer's machine while
  `mise.local.toml` exists.
- 🔵 The `.gitignore`/`bin/` guard sets omit the sub-binary cache shapes
  (`visualiser-<version>-<sha>`, `*.minisig`, `.tmp-*`) and `.gitignore:23` still
  names the pre-0168 `accelerator-visualiser-*` path.
- 🔵 Hook `case` snippets are not shfmt-shaped for `.sh` (`switch_case_indent =
  true`), unlike the correctly-tabbed `bin/accelerator` snippet.
- 🔵 `walk_files` needs a `collections.abc.Iterator` import the plan does not
  mention, and the `Iterator` vs `list` return shape is unstated.
- 🔵 `test:integration:skills` / `tests/integration/skills/` collides with the
  established convention where `test:integration:<name>` denotes a `skills/<name>`
  subtree run through `run_shell_suites`.
- 🔵 Whether `fixtures/installation/` is committed or built per-run is unstated; a
  committed shim would be single-platform and a second unguarded home for a
  trust-root binary.
- 🔵 Two of three cited stale-export surfaces are hook processes, which *do* receive
  Claude Code's own value — only `run-migrations.sh:6` and
  `interactive-harness.sh:29` are genuinely at risk.
- 🔵 The `docs/visualiser.md` fix leaves the "optionally symlink onto `$PATH`"
  trailing comment intact, and its `grep` criterion cannot detect that.
- 🔵 The documented `PATH` guidance enumerates version- and distro-specific facts
  (`/etc/paths` including a Ventura-only entry; Fedora's behaviour stated wrongly)
  that no test can keep honest.
- 🔵 Citation drifts: `cache_dir` is `:106` not `:105` (twice); the lazy closure is
  `main.rs:197-199` not `:196-198`; `display_path` is `:74-84` (cited both ways);
  `shim_bin` spans `:132-153`; `test_sources.py` has eleven cases not nine; seam
  site `:1053` uses `/nonexistent/plugin`, not `/nonexistent`.
- 🔵 Phase 2's purge list omits `benchmark.json` and the two files Phase 2 creates.
- 🔵 `claude_coupling` names a rule broader than it enforces — the docstring has to
  walk the name back in its first paragraph.
- 🔵 Phase 5 leaves three coexisting behaviours, and its follow-ups are recorded
  only as plan prose rather than work items; `summary.rs:144`'s `!known.is_empty()`
  gate becomes misleading dead-looking code.
- 🔵 Forty lines of top-level straight-line script now precede the first host gate;
  two named functions assigning globals would restore the four-named-steps reading.

### Assessment

The revision fixed what it set out to fix — every pass-1 structural finding is
resolved or correctly re-scoped, and the four decisions taken (per-capability
Phase 5, durable-record `fail()`, the 1a/1b/1c split, declining the key pin) all
survived independent re-derivation. The plan is materially better than it was.

It is not ready to implement. Three criticals block: two are Phase 5 assertions
that contradict the launcher's actual `Degrade::Notice` and `ReadTemplate`
contracts, and one is a documentation change that would ship a dangling `PATH`
link. Phase 5 in particular now needs a scope decision rather than a wording fix —
either bring the `ReadTemplate::template_names -> Result` port change in (five call
sites, two of them `-> String` error-message helpers) or drop `templates list`
from the phase and say the empty-table mode survives.

Two further items deserve attention because they were *caused* by the pass-1
edits: the logical `cd ..` in the bootstrap snippet contradicts the plan's own
prose and is the one hazard class the trust-anchor decision does not cover, and the
1c-after-1b ordering makes seven work-item seam assertions tautological in the
window between the two phases — a one-line reordering fix, since the plan already
establishes 1c depends on neither.

The pattern across both passes is worth noting: each round of tightening has
surfaced defects one layer deeper in the same areas (Phase 5's error plumbing,
the hook's link mechanics, the launcher-supply apparatus). That is convergence
rather than churn — the remaining findings are mostly concrete and local — but
Phase 5 and Phase 3 §2 are the two sections that have now been wrong twice, and
both would benefit from tracing the actual code paths before the next revision
rather than after it.

---

## Re-Review (Pass 3) — 2026-07-27

**Scope: Phase 3 and Phase 5 only.** Six lenses — correctness, security,
portability, test-coverage, architecture, code-quality — reviewed the two sections
that had each been revised twice. All other phases were explicitly out of scope.

**Verdict:** REVISE

**Only one lens rated anything critical**, and it is the defect described first
below — security rated the unverified-log path mismatch critical, the other four
lenses that found it rated it major. Otherwise the headline is that after the pass-2
revision both sections are structurally right: Phase 5's re-siting at the consumers,
the `Refusal` classification, the `template_names` port change and the
`known_skill_names` exclusion were each independently re-derived and confirmed —
including the two existing rootless `summary` tests that would have gone red had
the exclusion not been made. Phase 3's clear-before-rename step, `mkdir -p -m`
over an unconditional `chmod`, target validation and refuse-don't-clobber policy
were all confirmed as real, measured protections.

**But the pass-2 edits introduced two new defects, both flagged independently by
four or five lenses.** Neither is subtle in hindsight; both are the kind of thing
that only shows up when someone traces the two halves of a change against each
other.

### Previously Identified Issues

- ✅ **Phase 5's `--fail-safe` degradation path** — **Resolved.** The `Refusal`
  classification removes the degradation case entirely, and `templates_list_with_fail_safe_renders_the_unavailable_notice`
  (`config_read.rs:1504`) is confirmed as the assertion that would catch a
  mistaken `Read` classification.
- ✅ **`config template <name>`'s exit status is unchanged** — **Resolved and
  verified.** `From<ConfigError> for kernel::Error` (`error.rs:115-119`) maps every
  variant to `Failed`, and `report` (`main.rs:207-210`) special-cases only
  `kernel::Error::Refusal`, so swapping the variant leaves exit 1 intact and only
  improves the message.
- ✅ **`template_names` port change** — **Resolved.** Four call sites confirmed
  exact and complete, one implementor, every enclosing signature accepts the `?`,
  and the server's existing tests carry the change.
- ✅ **`known_skill_names` exclusion** — **Resolved and verified on every count**:
  one caller, the `?` precedes the `!known.is_empty()` gate, `assemble` reaches it
  whenever config is present, and `config instructions` validates via
  `validate_identifier` instead.
- ✅ **The `mv -f` symlink-to-directory trap at `dest`** — **Resolved.** The
  clear-before-rename step is confirmed correct and the test case is confirmed
  falsifiable, including that the escaped write survives the `trap`.
- 🟡 **The trust-chain record's reader** — **Regressed into a new defect.** See
  below.
- 🟡 **`${dest}.new.$$` handling** — **Regressed into a new defect.** See below.
- 🔵 **The declined downgrade guard** — **Still present, as accepted.** Security
  and architecture both now note the disposition is closer to
  "guarded vs *unobservable*" than "guarded vs documented", and both suggest the
  same cheap improvement: emit a line only when the target actually changes.

### New Issues Introduced

#### Introduced by the pass-2 edits

- 🟡 **The unverified-log reader watches a path the bootstrap never writes**
  (correctness, security, architecture, code-quality, test-coverage — **five
  lenses**). Phase 3 reads `${CLAUDE_PLUGIN_DATA}/.accelerator-unverified.log`;
  Phase 1a writes `${ACCELERATOR_CACHE_DIR:-${plugin_root}/bin}/…`, which its own
  `.gitignore` addition (`bin/.accelerator-unverified.log`) confirms. The paths
  never coincide, so the `[ -s ]` test is always false. Phase 1a's entire
  detectability rationale — "without it a bad signature is byte-identical to the
  ~86 commands that legitimately emit nothing" — is silently defeated, and no
  enumerated case covers the branch. Architecture adds that correcting the path
  forces the hook to duplicate the bootstrap's cache-dir precedence rule, and
  test-coverage adds that a corrected reader pointed at a self-located repo root
  would pick up logs from local runs and flake.
- 🟡 **`ln -sfn` does not fail when its destination is a real directory**
  (correctness, portability, test-coverage, code-quality — **four lenses**). `-n`
  suppresses following a *symlink* to a directory; with a real directory both BSD
  and GNU `ln` link *into* it. So a pre-existing `${dest}.new.<pid>` directory
  makes `ln` succeed, after which `mv -f` either renames a **directory** onto the
  documented fixed path (with `dest` absent — permanent, and every later session
  then hits the non-symlink refusal and reports success) or fails with `ENOTDIR`
  leaving an orphan `rm -f` cannot remove. The enumerated test case asserts the
  opposite on all three counts *and* is unbuildable: `$$` is the hook child's pid,
  which the suite cannot know in advance.

#### Other new findings

- 🟡 **The hook prefers the ambient `CLAUDE_PLUGIN_ROOT`, inverting Phase 1a's
  central invariant** (security, architecture, portability, correctness). Phase 1a
  spends forty lines establishing that an entry point must never let an ambient
  value redirect its root; Phase 3 then adopts `config-detect.sh`'s env-first
  precedence and *publishes* the result into persistent, channel-global,
  cross-session state. Unlike `config-detect.sh`, which uses the value transiently,
  a wrong value here is durable. The cited idiom also uses logical `cd ..`, which
  Phase 1a measured to be wrong and replaced with `cd -P`, and omits `CDPATH=`.
  Architecture notes the hook is by construction inside the installation it should
  link, so reading the variable adds an attack surface with no benefit.
- 🟡 **Stderr is the channel Phase 1a itself calls invisible** (security,
  architecture). Every Phase 3 signal — including "the documented `PATH` entry has
  been replaced and I will never repair it" — goes to SessionStart stderr, which
  `bin/accelerator:114-116` documents as unseen. Both lenses point at the same
  unused in-repo precedent: `hooks/vcs-detect.sh:14,177-181` emits a
  `{"systemMessage": …}` envelope for exactly this, pinned by
  `test-vcs-detect.sh:666-671`. The plan's claim that "bare plain text on stdout
  has no precedent" is true; the claim that *no* stdout precedent exists is not.
- 🟡 **`Some("")` still resolves plugin templates against the process cwd**
  (security). Phase 5's `plugin()` rejects only `None`, and the empty-string filter
  lives at `main.rs:176` in Phase 1b — but the visualiser server (`main.rs:69`)
  has its own bare `var_os` and no filter. So an empty root makes
  `plugin.join("templates")` a *relative* path, and project files render as
  `plugin default` content spliced into prompts at `!` sites. The phase asserts
  "an empty-string root behaves identically to an absent one" but lists no change
  that achieves it. Fix at one choke point (`with_plugin_root` or `plugin()`), not
  per-composer.
- 🟡 **"With and without `--fail-safe`" is not expressible for the third consumer**
  (test-coverage, correctness). `plugin_template_path` is reachable only via
  `templates eject|diff|reset`, and those clap variants carry no `fail_safe` field
  (`launch/inbound/cli.rs:285-311`) — so the flag is a usage error, exit 1 with
  clap's text, which does not name the variable. Loosening the assertion to "exit
  non-zero" would make it pass on argument parsing instead of the refusal.
- 🟡 **Rootless `templates eject` changes behaviour with no assertion**
  (test-coverage). Today rootless `eject --all` exits 0 having ejected nothing —
  the same silent-empty-answer defect the phase exists to close. After the change
  it errors, and nothing distinguishes them; every existing eject test uses
  `run_with_plugin`. This also hides the phase's one deliberate degradation:
  `.unwrap_or_default()` in `available` would panic if written `.unwrap()` and no
  suite would notice.
- 🟡 **Nothing pins that a user-override template still resolves rootless**
  (test-coverage, code-quality). §3 says "route all three sites through one private
  accessor", which invites hoisting `let plugin = self.plugin()?;` to the top of
  `resolve_template` — turning every rootless template read into a refusal and
  breaking project-local overrides. The only override test injects a root, so the
  mutation reddens nothing. The check must replace the `if let Some(plugin)` *in
  place*.
- 🟡 **`_EXPECTED_HOOKS_SUITES` has no stated value, and a count floor is inert
  here** (test-coverage). `hooks/` holds two suites today, three after the phase.
  `TestHooksSuiteGuard` builds its baseline from the constant, so a floor left at 2
  passes every test while failing to detect the loss of the suite it was added for.
  A `_REQUIRED_HOOKS_SUITES` identity entry (mirroring `_REQUIRED_CONFIG_SUITES`)
  is the only shape that detects it.
- 🟡 **`mkdir -p` precedes the guard meant to protect it** (correctness, security,
  portability). With `data_bin` a regular file or dangling symlink, `mkdir -p`
  fails first, so the `[ -e ] && [ ! -d ]` half is dead and the user gets the wrong
  diagnostic; with a symlink to a non-existent path, `mkdir -p` *follows it* and
  creates directories outside the tree before the refusal fires. Portability adds
  that `-m` applies only to the final component, so `${CLAUDE_PLUGIN_DATA}` itself
  is created umask-dependent (0775 under a 002 umask, common on several distros).
- 🟡 **Six repeated `printf … >&2; exit 0` blocks** (code-quality). The
  `[accelerator]` literal and the exit-0 policy are each restated six times across
  three syntactic shapes, in the same plan whose Phase 1a treats a *named* abort
  funnel as the right structure (`fail`/`fail_integrity`). Several lines also
  exceed 80 columns, which nothing enforces for shell.
- 🟡 **Two unrelated jobs in one hook, with the diagnostic gated behind the other's
  feature guard** (architecture, code-quality). The log check sits *after* the
  `${CLAUDE_PLUGIN_DATA}` inertness guard, so on any Claude Code lacking that
  variable the trust-chain report is suppressed too — coupling a security signal to
  the availability of a convenience feature, and meaning removal of the terminal
  surface silently removes the log's only reader.
- 🟡 **The ADR-0048 divergence is unrecorded** (architecture). ADR-0048 says hook
  logic lives in the CLI with shell as thin wrappers; Phase 3 adds ~50 lines of
  non-trivial bash plus a suite and a CI floor. There *is* a strong justification —
  a link whose purpose is to make the launcher reachable cannot depend on the
  launcher being fetched and verified — but it is nowhere stated, so it reads as
  drift and sets a precedent for the next hook.
- 🟡 **The documented literal path is unverified** (correctness, portability).
  `~/.claude/plugins/data/` does not exist on the maintainer's machine, and the
  installed *cache* uses `cache/<marketplace>/<plugin>/<version>/` — the opposite
  nesting order from the `data/<plugin>-<marketplace>` the plan guesses. It also
  ignores `CLAUDE_CONFIG_DIR`. A wrong literal ships a recipe that creates a
  dangling `PATH` entry, and the grep criterion locks the guess in. Correctness
  adds a deeper question Phase 0 §2 does not ask: **is `${CLAUDE_PLUGIN_DATA}`
  version-scoped?** If it is, the "target that never moves" premise inverts.
- 🟡 **The `Refusal` arm's doc comment now says something the new member is not**
  (architecture, code-quality). The arm reads "a validation refusal"; a missing
  installation root is an environment precondition. Since `ConfigError` is
  `#[non_exhaustive]` across a crate boundary, the `_ => Read` catch-all means the
  compiler can never force classification — so that doc comment is the *only*
  signal to the next person adding a variant, and it now misdirects. Architecture
  adds that the catch-all is fail-open by construction and that the phase's own
  named follow-up would land in it; both suggest a table-driven test pinning every
  variant's class.
- 🟡 **`PluginRootUnavailable { detail: String }` duplicates a literal its `Display`
  arm already owns** (code-quality), at a single construction site with no
  caller-varying payload. `ConfigError::LegacyLayout` is the in-file precedent for
  a unit variant carrying its whole message. The plan's `detail: /* names the
  variable */` placeholder also leaves the payload unspecified.
- 🟡 **`known_skill_names` still reads the raw `Option`, undercutting the
  accessor's stated purpose** (architecture, code-quality). §3 justifies the
  accessor as making the next reader correct by default, then leaves two direct
  readers in the same file — one of them a plugin-content reader — as live
  precedents for the pattern it exists to retire.
- 🔵 **Missing `# Errors` doc is a build failure** (code-quality). The `cli/`
  workspace sets `warnings = "deny"` with `clippy::pedantic`, so promoting a public
  trait method to `Result` without an `# Errors` section fails `cli:check`
  mid-phase.
- 🔵 **The `template_names` bullet contradicts the "does not buy" paragraph**
  (code-quality, correctness). "`templates list` can never render a header-only
  empty table" is false while the `read_dir` fallback survives. Correctness adds
  that the residue is broader than stated: it swallows *any* root that is not an
  installation — including the wrong-root case Phase 1a's self-location makes newly
  likely, and which the directly-invoked launcher never passes a `plugin.json` gate
  to catch.
- 🔵 **The empty-string case is claimed by both Phase 1b and Phase 5**
  (test-coverage, correctness, code-quality). Phase 5 says "move it here"; Phase 1b
  still adds it and still gates a named criterion on it. Testing Strategy also
  still attributes it to 1b.
- 🔵 **Three hook branches would survive deletion of the guard they name**
  (test-coverage): the `[ -e ] && [ ! -d ]` half of the `data_bin` guard, `-n` in
  `ln -sfn` (unreachable once clear-before-rename exists — yet Key Edge Cases still
  cites the trap as covered), and the relative-root case if `-x` refuses first.
- 🔵 **The snapshot-diff case cannot detect a modification or an out-of-tree
  write** (test-coverage). `find -print | sort | diff` is name-only; an in-place
  content change is invisible, as is anything written outside the snapshot root.
- 🔵 **The registration assertion is weaker than the precedent it cites**
  (test-coverage). A suffix match passes for a relative or wrongly-prefixed
  command, and nothing asserts `matcher`, so a `"matcher": "startup"` group would
  silently stop refreshing on resume/clear sessions.
- 🔵 Also: no exit-status capture mechanism for the "exits 0 on every path" case
  (the copied `run_hook` ends in `|| true`); the two mode-dependent suite cases
  (`0555`, `0700`) are uid- and `stat`-portability-dependent; tool-produced stderr
  is neither suppressed nor locale-pinned; the atomicity property the temp-plus-`mv`
  dance exists for is manual-only; `installation-root` knowledge now has three
  homes; `ACCELERATOR_RELEASE_BASE_URL`'s https requirement is unenforced on the
  wget branch; Phase 5 has no commit sequence though it splits cleanly in two; and
  the floor-guard block would become a fifth (or seventh) copy.

### Assessment

The two sections are now structurally sound — one critical (the log-path mismatch,
so rated by security alone), and the decisions that were re-derived independently
all held. What remains is a dense set of concrete, local defects, and two of the
most serious were introduced by the pass-2 edits themselves.

That is the signal worth acting on. Three rounds of review on these two sections
have each fixed the previous round's findings and introduced new ones in the same
two places: Phase 3's link mechanics (three different `ln`/`mv` semantics errors
across three passes, each measured wrong in a different way) and the seam between
Phase 1a's writer and Phase 3's reader. Both are areas where the failure mode is
"two halves of the plan disagree", which review catches only by tracing both — and
which an implementation plus its tests catches immediately and cheaply.

The recommendation is therefore to stop reviewing these sections and start
implementing them, with the pass-3 findings applied as the specification. Phase 3's
hook in particular should be written and run against its suite before any further
plan revision: the `ln`/`mv` behaviours have now been wrong in the plan three
times and correct in a shell session once, which is a strong argument for letting
the shell be the oracle. The findings that genuinely need a decision rather than a
fix are the hook's root precedence (env-first versus self-location), the output
channel (stderr versus `systemMessage`), and whether the log reader belongs in this
hook at all.

---

## Re-Review (Pass 4) — 2026-07-27

**Scope: Phase 1a and Phase 4 only** — the two phases carrying pass-2 edits that had
never been reviewed. Pass 2 found criticals in both (a launcher-supply problem that
would have made each fetch an unpublished release over the network, a network guard
that could not bind, a `.gitignore` pattern matching committed files); those were
revised, and nobody had looked since. Pass 3 was scoped to Phases 3 and 5, so this
closes the coverage gap.

Four lenses were dispatched — correctness, test-coverage, security, portability.
**The correctness agent was interrupted and did not report**, so that lens is
missing from this pass and its findings are unknown. Three returned.

**Verdict:** REVISE — one critical, roughly fifteen major.

### Determinations performed alongside this pass

Two Claude Code facts the plan depended on were settled, and both corrected
something the plan or an earlier review pass had asserted:

- **`${CLAUDE_PLUGIN_DATA}` layout — the plan's original literal was right.**
  `~/.claude/plugins/data/` exists and contains `accelerator-atomic-innovation`,
  `accelerator-atomic-innovation-prerelease` and `accelerator-inline`, i.e.
  `<plugin>-<marketplace>`; the plugins reference confirms `{id}` is
  `name@marketplace` with non-alphanumerics replaced by `-`. An earlier pass claimed
  the directory did not exist and the layout was a guess. It was not. (The
  discovery-first documentation edit stands anyway, since it survives a
  `CLAUDE_CONFIG_DIR` relocation, which the docs do not cover.) Also confirmed: the
  variable is **not** present in a Bash-tool environment, and it is **not**
  version-scoped, so the two-hop premise holds.
- **`systemMessage` is a universal top-level hook field**, documented in the hooks
  reference's "JSON output" section as "Warning message shown to the user", valid for
  every event and not requiring `hookSpecificOutput` nesting. So
  `hooks/vcs-detect.sh:14` is correct precedent, not a bug, and Phase 3's channel
  split is settled rather than contingent. Two related facts: for `SessionStart`,
  plain **stdout becomes Claude's context** ("stdout is added as context that Claude
  can see and act on"), so it must not carry diagnostics; and **stderr at exit 0 is
  undocumented**, which makes `bin/accelerator:114-116`'s "invisible at SessionStart"
  the safe reading and suggests `hooks/migrate-discoverability.sh:68-72`'s advisory
  may reach nobody — pre-existing, worth its own item.

### Critical

- 🔴 **Test Coverage**: Phase 4's per-family assertion table does not reconcile with
  the measured corpus, and specifies a family with zero members.
  **Location**: Phase 4 §2, per-family stdout assertions.
  Measured over the 204 `!`-site `config` commands:
  `agents` 22 + `paths` 1 + `path <k>` 66 + `work <k>` 8 + `review <k>` 3 = **100**
  (table says ~75); `instructions` 42 + `context --skill` 42 = **84** (table, and
  three other places in the phase, say 86); `template <name>` 20 (correct); and
  **`templates list` = 0** — the only `config templates` occurrences in `skills/`
  are nine *documented* commands in `configure/SKILL.md:930-1028`, which are
  model-invoked Bash, not `!` sites. So the table cannot serve as an implementation
  oracle, and nothing asserts the families *partition* the corpus: a future
  `config get` or `config summary` `!` site would match no row and fall through to
  the bare exit-0 pair, which is byte-identical to a `--fail-safe` degradation.

### Major — introduced by the pass-3 edits

- 🟡 **Security + Portability** (independently, both high confidence): the
  newline-rejection guard cannot work. `case "${plugin_root}" in *"$(printf '\n')"*)`
  — command substitution strips trailing newlines, so `$(printf '\n')` is the empty
  string and the pattern degenerates to `**`, **matching every root**. The bootstrap
  would abort on every invocation, and under `--fail-safe` silently at exit 0. The
  correct idiom is already in the same file at `bin/accelerator:48`
  (`${version%%$'\n'*}`). The guard also appears only in prose, not in the code
  block an implementer transcribes.

  **Fixed 2026-07-27.** Not by correcting the idiom — the guard was in the wrong
  place regardless. Tracing which values actually reach a `fail_integrity` message
  showed that gate `:168` interpolates `${cache_dir}`, derived from the
  caller-supplied and otherwise unvalidated `ACCELERATOR_CACHE_DIR`, so a
  `plugin_root` check would have left the log-forgery path open even written
  correctly. The `plugin_root` check is therefore removed entirely and the record is
  sanitised once at the write site (`"${1//$'\n'/ }"`), which covers every input
  reaching the log. Both idioms were measured on stock `/bin/bash` 3.2.57 before the
  edit: the broken form matches a plain path, `*$'\n'*` discriminates correctly in
  both directions, and `${var//$'\n'/ }` behaves as intended. A
  `test_a_record_is_always_one_line` case now drives a newline in through
  `ACCELERATOR_CACHE_DIR` — the reachable path, not the originally imagined one — and
  the plan records why the earlier form was wrong on both counts so it cannot be
  reinstated from the requirement alone.

  The process observation in the Assessment below stands unchanged: the defect was
  introduced by reasoning about shell semantics, and settled in seconds by running
  them.

### Major — Phase 1a

- 🟡 **Test Coverage**: nothing asserts `--fail-safe` still *reaches* the launcher
  after the new argv scan. This is the first argv-reading code in the file; a `shift`
  or a filter would make all 45 `!` sites fail-closed again with every phase test
  green. `test_happy_path_forwards_args_and_exit_code` uses `("alpha", "be ta")` — no
  flag, no `--`.
- 🟡 **Test Coverage**: one `_run_bootstrap` precondition is false against the
  harness's own value and two are tautological. "`ACCELERATOR_RELEASE_BASE_URL` ends
  in `.invalid`" fails against `https://example.invalid/v{_VERSION}`, which ends in
  the version; the downloader-present check cannot fire because `_run_bootstrap` sets
  it unconditionally and `extra_env` can only override, not delete. Check the
  *authority*, evaluate after `env.update(extra_env)`, and keep the one
  non-tautological invariant: the resolved entry lies inside the pytest tmp tree.
- 🟡 **Security**: the durable record misses the two paths where the bootstrap
  *detects tampering and recovers* — a cached launcher failing `verify_launcher` and
  being silently re-fetched (`:246-258`), and a staged shim whose digest mismatches
  being re-staged (`:165-171`). Both are pinned by existing tests as correct
  behaviour, and both are the on-disk shapes a local attacker would produce. The
  audit channel records only the give-up cases.
- 🟡 **Security**: the record is written to an env-redirectable, unvalidated path
  that its Phase 3 reader derives independently. `unverified_log` honours
  `ACCELERATOR_CACHE_DIR` *before* `resolve_cache_dir` validates it, so a relative
  value lands under the user's project; and on a read-only root the pre-cache appends
  fail with only the invisible stderr line. Pin the log to `${plugin_root}/bin` — it
  is a fact about the installation, not the caller's cache — so writer and reader
  agree by construction.
- 🟡 **Security**: the record never clears and shares its file with the dev override,
  which runs unattended at every SessionStart on a contributor machine. After one
  dev-override session the reader's message fires forever with identical text, and
  cannot distinguish a signature failure thirty seconds ago from an override six
  months ago. Separate the streams, or have the reader report the last line plus a
  count and acknowledge it.
- 🟡 **Security + Test Coverage**: `.gitignore` misses the launcher's own temp
  scheme. `store::TEMP_PREFIX = ".tmp-"` plus the sub-binary stem gives
  `.tmp-visualiser-<version>-<sha>-<n>`, which `bin/.tmp-launcher-*` does not match —
  and that file is written **before** verification, so an interrupted fetch leaves an
  untracked *unverified* executable in the shipped `bin/`. `bin/.accelerator-probe-*`
  (`:79`) is likewise uncovered, and `bin/visualiser-*` hardcodes the one current
  sub-binary name. The new assertion also pins only the negative direction (the four
  committed shims are not matched) and not the positive.
- 🟡 **Portability**: the platform gates are the only aborts a genuinely
  non-portable host hits, and they are the only ones excluded from the durable
  record. `unsupported architecture` (`:35`), `unsupported operating system` (`:40`)
  and `need curl or wget` (`:67`) keep plain `fail()`, so under `--fail-safe` a user
  on FreeBSD, illumos, linux-riscv64, or a container without curl/wget sees all 45
  skills render empty context with no diagnostic in any channel. Make the record a
  function of the *degradation* rather than the gate's category.
- 🟡 **Portability**: fixture roots must be realpath-canonical or the
  `ACCELERATOR_PLUGIN_ROOT ==` assertions fail on the Darwin leg only. `pwd -P`
  resolves `/tmp` → `/private/tmp` and `/var/folders` → `/private/var/folders`;
  `tmp_path` happens to be resolved, but the plan introduces several roots that are
  not obviously `tmp_path`-derived (the bootstrap copy, `_run_bootstrap`'s default
  `cwd`, the symlink-chain scratch trees, the injected sentinel root, Phase 4's
  installation root).
- 🟡 **Portability**: the floor-interpreter criterion hardcodes `/bin/bash`, which is
  absent on NixOS and some container bases; its stated alternative (assert
  `BASH_VERSINFO` is 3 on Darwin) reddens the suite for any contributor with Homebrew
  bash on `PATH`. And either way floor coverage exists only where `/bin/bash` happens
  to be 3.2.57, which nothing asserts — so losing it would be silent.
- 🟡 **Test Coverage**: two guard-named tests cannot fail if the guard is removed.
  `CDPATH` is consulted only for an operand that is neither absolute nor
  `.`/`..`-prefixed, and `_run_bootstrap` always invokes an absolute path; the
  dash-target case never produces a dash-leading `cd` operand, because after the
  first hop `self` is always absolute.
- 🟡 **Test Coverage**: the durable record is asserted at one of six gates, and not
  at `:254` — the fetch-and-verify gate the whole rationale is built on.
  `test_non_release_key_signature_is_refused` is not extended, so the bad-signature
  case has no record assertion in either shape. Also unpinned: that a bootstrap-layer
  `fail` leaves *no* record (i.e. that the two channels are distinct), that the record
  honours `ACCELERATOR_CACHE_DIR`, and the "could not record" fallback.

### Major — Phase 4

- 🟡 **Test Coverage + Portability**: the new leaf is on both sides of the exhaustive
  launcher-edge split. Phase 4 gives `test:integration:skill-invocation`
  `build:cli:dev` *and* adds it to `_LAUNCHER_DEPENDENTS`, while inheriting the
  shared helper whose `launcher_bin` builds in-fixture — which is Phase 1a's stated
  reason for keeping `entrypoint` out of the edge. The partition meant to force an
  explicit decision is populated with a contradiction on its first extension.
- 🟡 **Test Coverage**: the shared `tests/integration/support/installation.py` is
  declared a Phase 1a deliverable that Phase 1a neither schedules in its Changes
  Required nor verifies in its criteria — a ~150-line trust-chain harness with no
  owning phase, which is the classic shape for ending up duplicated.
- 🟡 **Test Coverage**: the per-file contribution invariant is either tautological or
  red on landing. It needs a population predicate independent of the corpus scan; the
  obvious one (substring `bin/accelerator config`) is red immediately because
  `configure/SKILL.md:930-1028` carries nine such commands with no `!` site.
- 🟡 **Test Coverage**: the fixture oracle is specified two contradictory ways —
  "the only committed fixture" with a `config.md` configuring every skill, *and*
  "generate the config file and the expectations from one fixture data structure".
  Committed means hand-editing on every skill rename; generated means the manual
  deletion criterion has nothing to delete. The spec also records nothing about what
  makes `agents` (22), `work` (8) and `review` (3) non-empty.
- 🟡 **Portability + Security + Test Coverage**: the decline filter is narrower than
  the shell semantics it replaces, and its stated guarantee does not exist.
  `skill_permissions.py:50`'s `_METACHARACTERS` omits `>`/`<`/bare `&`, tilde, globs
  and plain `$VAR` — and `visualise/SKILL.md:30` already passes rule 4 while carrying
  `$PPID` and `${ARGUMENTS:-start}`. So a future `$VAR`-bearing `config` site would
  be executed with literal semantics where production expands it.
- 🟡 **Portability**: Phase 4 describes its environment **subtractively** ("both
  variables removed") where `_run_bootstrap` derives hermeticity from composing a
  complete explicit `env=`. Every one of the 204 invocations is a real bootstrap run,
  so an ambient `ACCELERATOR_CACHE_DIR`, `ACCELERATOR_BIN`,
  `ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER` or `ACCELERATOR_LAUNCHER_BIN` changes the
  result, and a *missing* downloader/base-URL is what lets it reach the network.
- 🟡 **Security**: the 204 commands run with cwd set to a **tracked** fixture
  directory, with no cleanliness assertion and a substring-based selection filter.
  Today's corpus is read-only (`Scaffold::init`'s writes are reachable only from
  `config init`), but nothing asserts that, and there is no equivalent of Phase 1a's
  `bin/` backstop for the fixture tree.

### Minor

- 🔵 The newline guard, the `--fail-safe` scan's option-value window, `readlink`
  without `--`, and a trailing-slash symlink target all remain unasserted or
  unhardened (security, portability).
- 🔵 The dev-override *refusal* (`:133`) stays on plain `fail()`, so the refused
  unverified exec leaves no trace while the accepted one does — the audit trail is
  inverted with respect to severity (security).
- 🔵 The transitional dual export's *value* is pinned only textually; every
  behavioural assertion reads `ACCELERATOR_PLUGIN_ROOT` only, including the
  `inject="old"` parametrisation where overwriting the ambient old-name value is the
  interesting property (test coverage).
- 🔵 The harness spec omits four mechanisms its own tests use: the env-dump output
  path, `timeout=`, a bare-root builder, and the source of `cwd`'s default (a
  per-call `mkdtemp` would leak a directory per invocation — 200+ under Phase 4)
  (test coverage).
- 🔵 The 16/17-link tests assume a kernel nesting limit above 8; Linux raised it to
  40 only in 4.2, so on an older host kernel the 16-link partner fails with `ELOOP`
  rather than anything pointing at the limit (portability).
- 🔵 Both reused-guard invariants use predicates that differ from the guard's own, so
  they can fail for non-reasons (test coverage).
- 🔵 The one `!` site with no `--fail-safe` (`visualise/SKILL.md:30`) is excluded
  wholesale, though its *bootstrap* half is cheaply coverable with the argv-dumping
  stub Phase 1a already builds (test coverage).
- 🔵 No runtime budget is stated for 204 full bootstrap runs — each re-hashes the
  verify shim and runs `minisign` over a multi-tens-of-MB debug binary — and the
  one-fetch property is unasserted (test coverage).
- 🔵 Phase 4 leaves its interpreter unstated, so the largest exercise of the new
  self-location code runs on an unpinned host bash (portability).
- 🔵 The `$#` guard is justified by corpus precedent rather than by the floor
  requirement that makes it mandatory (bash before 4.4 treats `"$@"` as unset under
  `set -u`), and no static gate would catch its removal (portability).

### Assessment

Checking these two phases was worth it: they were the least-reviewed sections of the
plan and they carried a critical plus roughly fifteen majors, several of which
(`.gitignore` temp coverage, the platform gates' silence, the tracked-fixture cwd,
the false `.invalid` precondition) would have surfaced only as confusing CI or, worse,
as an unverified binary committed into the shipped `bin/`.

The pattern from pass 3 repeated exactly, though: the pass-3 edits introduced a
defect of their own, and it is the sharpest illustration yet of where review is the
wrong instrument. `case "${plugin_root}" in *"$(printf '\n')"*)` is not a subtle
design error — it is a shell-semantics mistake that `bash -n` plus one invocation
would have caught in seconds, that two independent lenses caught by reading, and that
I introduced while reasoning carefully about the very hazard it was meant to close.
Three of the last four defects in this plan have been shell-semantics errors settled
immediately by running a shell.

Verdict stays REVISE. But the remaining confidence should come from **building Phase
1a**, not from a fifth pass: its specification is now the most-reviewed part of the
document, its findings are concrete and local, and the failure modes that keep
recurring are exactly the ones an implementation plus its suite eliminates. Note also
that this pass is missing the correctness lens, so its coverage of these two phases
is incomplete — another reason to prefer execution over a further partial read.
