# Task tree

The repo's dev tasks are declared in `mise.toml` (run them with
`mise run <task>`) and implemented as [invoke](https://www.pyinvoke.org/) tasks
in this package. `mise tasks` lists every leaf with its description; this file
documents the *shape* of the tree so it only has to be learned once, and
carries the checklist for registering a dispatched sub-binary.

## Per-component checks

Each component has a `<component>:check` roll-up that folds that component's
format + lint (+ type-check where applicable):

| Component      | Roll-up              | Folds                                        |
| -------------- | -------------------- | -------------------------------------------- |
| Frontend       | `frontend:check`     | format + lint + types (Biome, tsc)           |
| Rust server    | `server:check`       | format + lint (rustfmt, clippy)              |
| Rust cli       | `cli:check`          | format + lint (rustfmt, workspace-wide clippy) |
| Python tooling | `build-system:check` | format + lint + types (ruff, pyrefly), plus workflow lint and the dispatch-coherence guard |
| Shell          | `scripts:check`      | format + lint (shfmt, ShellCheck + bashisms) |

`build-system` is the repo-root Python automation toolchain (this `tasks/`
package + its tests) — unrelated to the `build:*` artifact namespace. Its task
descriptions name Python/ruff/pyrefly so `mise tasks | grep -i python` finds it.

`cli:check` runs **one workspace-wide** `cargo clippy --workspace` pass that
covers every member declared in `cli/Cargo.toml` `[workspace].members`, so new
members join enforcement with no per-member wiring. `format:cli:*` and
`lint:cli:*` each depend on `deps:install:rust-components` — mise's `[tools]`
rust `components` field is silently skipped for an already-present toolchain, so
rustfmt/clippy are provisioned explicitly. `lint:cli:fix` applies only clippy's
machine-rewritable subset (lints such as `unwrap_used` cannot be auto-fixed), so
`cli:check` must still be run for the remainder. It also folds three Python
guards over the same tree — `lint:vendor-shims:check`,
`lint:store-duplication:check` and `lint:claude-coupling:check`. Those three are
wired into `lint:check` as well: `cli:check` is what CI runs, but the bare
`default` task depends on `lint:check` and not on `check`, so a `cli:check`-only
guard stays green in a full local run however badly its invariant is broken.
`tests/unit/tasks/test_mise.py` pins both placements. `build-system:check`
carries `lint:dispatch-coherence:check` under the same reasoning: it is a
skills-tree guard rather than a Python one, but `build-system:check` is what CI
runs, and it is wired into `lint:check` as well so a bare `mise run` reaches it
too. Beyond `cli:check`, Rust
enforcement also spans standalone entity tasks wired directly into the top-level
`check` (they sit outside the `cli:` roll-up, mirroring `version:*` /
`github:*`): `deny:check` (cargo-deny supply-chain) and `pup:check` (cargo-pup
architecture, on an isolated nightly lane). `pup.ron` carries one rule per
domain boundary plus `vcs_adapters_library_reads_in_process`, which scopes the
library-backed VCS adapter's imports to a permit list and denies `std::process`
— see "Library-backed VCS dependency pins" below.

## Family aggregates

`format:check`, `lint:check`, and `types:check` run the corresponding family
across every component; `check` runs all of them (this is what CI runs). `fix`
applies `format:fix` + `lint:fix` (mechanical changes only).

## Conventions (learn once)

- A component name **leads** its roll-up (`server:check`, entity-first, like
  `version:*` / `github:*`) but **trails** in the families
  (`format:server:check`).
- `scripts` and `server` have no `types:*` — only `frontend` and `build-system`
  type-check. `lint:scripts:check` nests one level deeper (shellcheck +
  bashisms) because shell has two linters.
- There are `<component>:check` roll-ups but **no** `<component>:fix`. Fix one
  component via `format:<component>:fix` + `lint:<component>:fix`, or run the
  top-level `fix`. Shell has no autofixer, so `scripts` is absent from
  `lint:fix` — run `mise run scripts:check` for remaining shell findings.

The `lint:<language>` task naming requested by work item 0098 is satisfied by
these `<component>:check` roll-ups, and its aggregate `lint` / `format`
acceptance criteria by the family aggregates above.

### Executable-bit invariant

A tracked `.sh` is executable (`0755`) **iff it is _not_ a sourced-only
library**. The `lint:scripts:exec-bits:check` guard
(`exec_bits` in `tasks/lint/scripts.py`) enforces this over every shell source
and fails — naming each offending file with the exact `chmod` to run — when an
off-list entrypoint lacks `+x`, a library carries `+x`, or a library-list path
is no longer enumerated.

- **Default: new `.sh` files are entrypoints.** `chmod +x` and commit them. You
  only touch `SHELL_LIBRARIES` (the manifest in `tasks/lint/scripts.py`) for a
  sourced-only library.
- **The classification rule is two-part: sourced AND never invoked by path.**
  "Sourced" alone is not enough. `jira-fields.sh` is `source`d by
  `jira-init-flow.sh` *and* invoked `bash …/jira-fields.sh refresh` in
  production, so it is an **entrypoint** that stays OFF the list at `0755`.
  Dual-use ⇒ entrypoint.
- **Maintenance:** a new sourced-only library must be **added** to
  `SHELL_LIBRARIES` (or the guard demands `+x`); a removed/renamed library must
  be **deleted/updated** there (or the stale-entry check fails).
- **Runner vs helper:** `test-interactive-protocol.sh` is a test *runner* →
  entrypoint → `0755`; `test-helpers.sh` is a sourced *helper* → on the list →
  `0644`.
- **Fixtures are a third category.** Scripts under `test-fixtures/**` are
  bash-run migration fixtures (executed via `bash "$f"`, never sourced, never
  path-invoked): the guard exempts them in both directions — they need neither
  `+x` nor a list entry.
- **Working-copy mode.** The guard reads the *working-copy* mode (matching
  `tasks/test/helpers.py`), so the `chmod` must be **committed** to satisfy CI
  on a fresh checkout. It intentionally enforces working-copy (not VCS-recorded)
  mode and assumes an exec-bit-preserving filesystem — acceptable given the
  macOS + Linux target matrix (CI runs `check-scripts` on `ubuntu-latest`; local
  dev is macOS via jj workspaces).

### Library-backed VCS dependency pins

`cli/vcs-adapters` reads git through `gix` and jj through `jj-lib` in-process.
Those two pins are **not independent**, and neither is bumped alone.

**The coupling is six-way**: `jj-lib` (exact, `=0.43.0`), `gix`
(tilde, `~0.85.0`), `prost` and `pollster` (jj-lib's own, adopted as direct
edges), the Rust toolchain, and `mise.toml`'s `jj` CLI pin. The CLI
writes the repository format the library reads, so a skew between them surfaces
as an apparently wrong detection answer rather than as a version mismatch;
`jj-lib`'s MSRV moved 1.85 → 1.88 → 1.89 across eight releases, so a bump drags
the toolchain too. The asymmetry is deliberate: `jj-lib` is exact because the
design leans on its declared-unstable loader internals, while `gix` takes a
tilde so a RustSec fix is a lock update rather than a pin edit. `gix` also sets
`default-features = false`, which is what keeps `gix-credentials` — whose
helpers spawn `git credential-*` programs — out of a module that exists to avoid
spawning. Widening that feature list is how a consumer adds a capability.

`prost` and `pollster` are the newest two, and they are **jj-lib's, not ours**.
They exist so the jj working-copy commit id can be read without constructing a
settings value: `prost` decodes `jj_lib::protos::local_working_copy::Checkout` (a
public module, so this is API rather than a private wire format) and `pollster`
drives the `OpStore` trait's async reads, which jj-lib itself drives with
pollster. Both were already in the lock through jj-lib, so adopting them added
two edges and no packages. They are pinned to the majors jj-lib requires, because
the decoded type comes *from* jj-lib — a major mismatch would put two `prost`
graphs in the lock and the generated code would stop matching the decoder. Move
them when `jj-lib` moves, never on their own.

Two committed checks hold this together:
`tests/unit/tasks/test_vcs_pin_lockstep.py` (the declarations agree and keep
their inline rationale) and `tests/integration/deny/test_vcs_library_graph.py`
(versions, single gix graph, a single `prost`/`pollster` version each, the
enabled feature set, no TLS in the subtree on any of deny.toml's five targets,
MSRV, and a build-script/proc-macro snapshot).
That the *binary* building fixtures matches the pin is asserted by the fixture
harness, not by these.

**Two enforcement mechanisms, with a clean division of labour.** cargo-pup owns
**import** prohibitions; the `tasks/` source guards own **usage** prohibitions
imports cannot express — `RestrictImports` resolves `use` paths, so a
fully-qualified `jj_lib::settings::UserSettings::from_config(…)` or a
`Workspace::load` method call is invisible to it. That is the whole
justification for the extra Python machinery over a one-line `denied` clause.

`lint:vcs-settings:check` (`tasks/lint/vcs_settings.py`) is that guard: no code
in `cli/vcs-adapters` may construct a `UserSettings` or call `Workspace::load`,
whose defaults are private to jj-lib and were discovered one panic at a time.
It **strips comments before matching**, so the crate can document why it avoids
them without flagging itself. It rides both `cli:check` and `lint:check`, for
the same bare-`default` reason as the other `cli/`-scoped guards.

**Break-glass for a supply-chain failure.** Both transitive trees enter
cargo-deny's `advisories` scope under `unmaintained = "all"` with
`yanked = "deny"`, over a ~60-crate closure no repo code calls, and the advisory
DB is fetched fresh every run. One upstream advisory there turns
`check-supply-chain` red for every unrelated PR — and that job is in
`prerelease.needs`, so it also stops releases. Recovery is a scoped, dated
`[advisories].ignore` entry following the existing `RUSTSEC-2026-0118/0119`
precedent, with a `review-by: YYYY-MM-DD` in its `reason`;
`tests/integration/deny/test_advisory_ignores.py` asserts every entry carries
one and that none has lapsed.

**The break-glass is scoped to the `unmaintained`, `yanked` and `notice`
classes only.** A `vulnerability`-class advisory takes the escalation path —
upgrade, patch or vendor — never an ignore, regardless of release pressure,
because this closure reaches the publicly distributed signed
`accelerator-visualiser` binary. cargo-deny's `ignore` list is flat with no
class distinction, so the scoping has to be written down or the pre-authorised
action silently covers every class.

**The licence side has no `ignore` mechanism at all.** `[licenses].allow` is
pruned to exactly the licences the current closure carries, so a transitive
crate acquiring or replacing one is a hard failure needing either an `allow`
addition (permissive) or a justified `[[licenses.exceptions]]` (copyleft), with
the `uluru` MPL-2.0 entry as the template.

### Zero-spawn strong form

The library-backed VCS adapter reads git and jj **in-process**. Two mechanisms
prove it, and they prove different things.

`test:integration:zero-spawn` puts marker-writing `git`/`jj` stubs first on a
synthetic `PATH`, drops every directory that could resolve a real one, runs the
whole fixture matrix, and asserts **both** that no stub recorded a spawn **and**
that every value matches an unrestricted run — an adapter degrading to absence
also writes no marker. It is scoped to `git`/`jj` specifically, not "no
subprocess at all": the clock spawns `date` unconditionally.

That is the **weak** form: a caller reaching `/usr/bin/git` by absolute path
never consults `PATH`. The **strong** form additionally shadows those absolute
paths. It is `test:integration:zero-spawn:strong`, and it owns the whole
sequence: compile the artefacts and build the fixture matrix while the real
binaries are still reachable, shadow them, run the prebuilt suite, restore in a
`finally`. Only the `check-zero-spawn` CI job invokes it, and that job is in
`prerelease.needs`.

Targets are resolved at run time from **three** sources: every `PATH` hit (macOS
ships `git` in two directories), `mise which git`/`mise which jj`, and the known
system paths. `mise which` is the load-bearing one on CI — there is no system
`jj` on the runner, the real binary lives under the mise install tree, and what
sits on `PATH` may be a shim pointing at it. Shadowing only a shim would leave
the real binary reachable by absolute path while the harness agreed the run was
strong, because it is only told about the paths we shadowed.

The harness and the task have an explicit contract —
`ACCELERATOR_ZERO_SPAWN_MODE` and `ACCELERATOR_ZERO_SPAWN_SHADOWED` — and the
harness **fails closed** on a malformed mode or a path that is still executable,
so a runner image that relocates `git` cannot turn the `sudo mv` into a silent
no-op.

`test:integration:zero-spawn` is deliberately **out of the `test:integration`
roll-up**: membership would rebuild the ~34-fixture matrix a second time per
run, on both OS legs and on every bare `mise run`, in a code path with a
documented flake history under parallel CI load. It stays runnable on demand.

**The Rust harness never writes outside its own temp directories.** It resolves
and *reports* absolute paths; it never moves, chmods or `sudo`s anything. All
privileged mutation lives in the strong-form task, which is **gated behind
`ACCELERATOR_ZERO_SPAWN_SHADOW=yes`** and refuses to start without it.
`/opt/homebrew/bin` is user-writable, so an ungated task would succeed there and
could leave a developer's machine without `git` or `jj`. The gate is an
environment variable rather than a flag precisely because a task name can be
tab-completed by accident and an `env:` block cannot.

**Containment assumes ephemeral runners.** Shadow, run and restore live in one
task, with the restore in a `finally` and a step-level `timeout-minutes` shorter
than the job's, so the process rather than the scheduler guarantees the restore.
A trailing `if: always()` step then asserts `git --version` and `jj --version`
both succeed — deliberately as bare commands, not `mise run`, because mise would
reinstall the missing tool and turn the one check that catches a failed restore
into one that quietly repairs it. For the same reason the task invokes cargo
directly inside the window: mise is entered before it and never within. The job sets `cache: false` on `mise-action` because the jj shadow
target sits inside the tree the action saves on its post step, so a failed
restore would otherwise persist a `jj`-less tool tree into the cache that every
later run restores. A move to self-hosted, containerised or reusable runners
turns a contained hazard into a persistently broken runner.

`build:cli:fixture-size` is the third guard: the linked reference artefact must
be at least 3× the stubbed twin, so a future edit that stops printing a query
result — letting the linker drop `gix`/`jj-lib` — is caught on the PR path
rather than first firing during a release. The cross-compile applies the same
ratio on every triple plus an absolute byte floor on **musl only**; the darwin
stripped delta clears that floor by ~9%, and every triple is stripped, so gating
darwin would put a 9%-margin heuristic on `prerelease:prepare`'s critical path.
When it fires: re-measure, then adjust the constants in `tasks/build.py` only if
the drop is understood.

### Rust nightly lane (cargo-pup)

Architecture enforcement (the ADR-0053 inward-dependency rule) runs via
**cargo-pup**, a compiler plugin that needs a **second, pinned nightly**
toolchain (`PUP_NIGHTLY` / `PUP_VERSION` in `tasks/shared/rust.py`, a matched
pair). Everything else — the product build and every other check — stays on the
mise-pinned stable `1.90.0`.

- **Isolated by construction.** The nightly is provisioned only by
  `deps:install:pup` (rustup-managed, deliberately *not* a mise `[tool]`: mise
  cannot pin two rust toolchains, and a `cargo:` backend would build cargo-pup
  against stable and fail to load). Only `pup:check` and `test:integration:pup`
  consume it, and only the `check-architecture` CI job runs them. A nightly
  break (or a GC'd pinned nightly) therefore reddens `check-architecture` alone;
  every stable-lane check and the product build stay green. `pup:check` is
  wired into the top-level `check`, so a local `mise run check` still exercises
  it. The isolation is guarded by `tests/unit/tasks/test_workflows.py`.
- **First run is slow.** `deps:install:pup` builds cargo-pup from source
  (multi-minute) the first time; a presence probe skips the rebuild in steady
  state, so subsequent `pup:check` runs are fast.
- **Bumping the pin.** `PUP_NIGHTLY` and `PUP_VERSION` are a matched pair —
  cargo-pup's `rustc_private` driver only loads under the nightly it was built
  against — so bump them **together**. Dated nightlies are GC'd from
  `static.rust-lang.org` after a window; when the pinned one disappears,
  `deps:install:pup` fails with an actionable message naming the pin. Before
  committing a bump, verify the upstream release's published
  checksum/attestation (mirroring the SHA-256/SLSA discipline the visualiser
  binary gets via the signed `manifest.json`).
- **`mise.lock` refresh.** The committed `mise.lock` hash-pins the aqua-backed
  tools. On **any** `[tools]` edit (or aqua pin bump), regenerate it — `mise
  lock --platform linux-x64,macos-arm64,macos-x64` (all matrix platforms) — and
  commit the result, so a lock authored on one arch does not force a fetch or
  dirty the tree on another. It does **not** cover the from-source cargo-pup
  build or the rustup nightly (an accepted unverified surface for the isolated
  lane).

### File-descriptor limit on the cross-compiles

`cargo zigbuild` links through zig, which opens **every** object file of a link
at once, and a release link of the `cli/` workspace passes it several hundred
(~640 for `accelerator`). macOS's launchd default soft limit is **256**
descriptors, so from a stock shell the link dies partway through with
`ProcessFdQuotaExceeded` on an object it cannot open — a failure that reads like
a broken build but is purely an environment limit.

Both cross-compile tasks therefore call `raise_descriptor_limit()`
(`tasks/shared/limits.py`) before their first `cargo zigbuild`. `setrlimit` is
inherited across `fork`/`exec`, so raising it once in the task process covers
cargo, rustc and the zig wrapper beneath them, and no contributor needs a
`ulimit -n` in their shell profile. The raise is best-effort: it never lowers an
adequate limit, clamps to the hard limit (finite on Linux, `RLIM_INFINITY` on
macOS), and warns rather than aborting if `setrlimit` is refused — the link
itself fails loudly if the limit really was the constraint.

### Contributor environment variables

Local-only toolchain escape hatches. **CI ignores both** (it runs the
fail-closed defaults), so the fix for a red job is the underlying finding, not
the env var.

| Variable                 | Default | Effect                                    |
| ------------------------ | ------- | ----------------------------------------- |
| `ACCELERATOR_PUP_MODE`   | `deny`  | `warn` downgrades a cargo-pup findings failure to advisory (log only). Unrecognised values fail closed to `deny`. |
| `ACCELERATOR_COVERAGE`   | `on`    | `off`/`false`/`0`/`no` drops `test:unit:cli` from instrumented `cargo llvm-cov nextest` to plain `cargo nextest run` (faster inner loop). |

## Registering a dispatched sub-binary

A **dispatched sub-binary** is a separate static binary the launcher fetches on
demand from the signed release manifest (ADR-0054). Its **token** is the
subcommand name in `accelerator <token>`, and the same string is also the
`DISPATCHED_SUBBINARIES` entry, the manifest key, and the launcher's cache
filename prefix — while the crate, its `[[bin]]` and the published asset all
carry an `accelerator-` prefix. The registries are spelled `*_SUBBINARIES` for
history. Each point below is tagged with where a mistake surfaces: **[PR]** a
test or a per-PR CI gate catches it, **[release]** it fails the release job,
**[author]** nothing catches it.

1. **Add** the token to `DISPATCHED_SUBBINARIES` (`tasks/shared/paths.py`), then
   update three things in `tests/integration/tasks/test_github.py`: the registry
   pin (a deliberate anti-vacuity anchor, not a count to bump blindly), the
   `assert len(uploads) == 22` count, and `_setup_release`'s staged fixture and
   in-test manifest, which are single-token today. Converting that count to a
   derived expression is expected of the first sibling to land. **[PR]**
2. **Add** an entry to `_SUBBINARY_MANIFESTS` (`tasks/manifest.py`) when the
   crate is not at `cli/<token>/`. The visualiser is the worked example:
   `"visualiser": CLI_DIR / "visualiser/server/Cargo.toml"`. **No action when**
   the crate is at `cli/<token>/`. **[release]**
3. **Add** the crate's `Cargo.toml`: `[[bin]] name = "accelerator-<token>"` (the
   asset name the manifest and signing expect), a mandatory
   `package.description` (the manifest sources the description from it), and the
   inherited `version.workspace`, `edition.workspace`, `rust-version.workspace`,
   `license.workspace` and `publish.workspace`. Inherit the version so the next
   workspace bump cannot desynchronise the member — the version-coherence check
   in `tasks/build.py` only reports a *mismatch*, so a hardcoded current version
   passes today and breaks at the next bump. For lints, either inherit with
   `[lints] workspace = true` or declare a crate-local `[lints.clippy]` table if
   you need allows, as `cli/visualiser/server/Cargo.toml` does; either way `-D
   warnings` from `lint:cli:check` still promotes warnings to errors, and
   without the workspace table you opt out of the shared pedantic/nursery/
   `unwrap_used` opt-ins rather than out of lint enforcement. **[release]**
4. **Register** the crate in `[workspace].members` in `cli/Cargo.toml`
   (**[release]** — nothing pins the members list, so an omission surfaces as a
   missing cross-compiled binary during signing), and commit the regenerated
   `cli/Cargo.lock` (`lint:cli:check` runs `--locked`, so a stale lock reddens
   `cli:check` as an apparent clippy failure).
5. **Add** `bin/<token>-*` to `.gitignore`. The launcher caches every fetched
   sub-binary as `bin/<token>-<version>-<sha256>` in the plugin root, so this is
   needed whether or not anything is staged there at release time.
   `bin/*.minisig` and `**/bin/*.debug.tar.gz` are already token-generic.
   **[author]** — nothing catches it, and the local-dev release path's `git add
   .` would sweep the cached binary into the version-bump commit.
6. **Add** an entry to `cli/launcher/tests/fixtures/manifest.example.json` only
   if you want the golden contract to stay representative of a multi-binary
   manifest. **No action when** you are adding a new key: both co-readers —
   `tests/unit/tasks/test_manifest_contract.py`, which iterates
   `binaries.values()` generically, and the `include_str!` in
   `cli/launcher/src/launch/outbound/resolve/manifest.rs`, which reads the
   existing entry — are key-agnostic and break only if an existing entry is
   renamed or removed. **[author]**
7. **Add** the skill binding: a skill invoking `accelerator <token>` through the
   `!` preprocessor, plus a `Bash(...)` rule whose subcommand segment is exactly
   `<token>` and which covers that invocation. A rule scoped tighter than the
   token (`Bash(…/accelerator <token> start)`) binds provided it covers the
   invocation; a wildcarded segment (`Bash(…/accelerator <tok>*)`) does not. A
   bare `Bash` tool, a rule authorising the bare launcher, or a rule with a
   wildcarded token segment **anywhere** in that skill's frontmatter
   disqualifies the whole skill as a witness, so pick or write a skill that has
   none of them. The guard sees only `!`-preprocessor commands in
   `skills/**/SKILL.md` naming `${CLAUDE_PLUGIN_ROOT}/bin/accelerator` —
   invocations from `hooks/`, `scripts/` and model-driven Bash are outside its
   reach. **[PR]**
   - Also check: if the binding is satisfied by a *new* skill that injects
     config context or instructions, bump `EXPECTED_INJECTION_SKILLS`
     (`tasks/lint/skill_permissions.py`) in the same change, and keep the `!`
     command free of shell metacharacters — both are separate guards. Authoring
     a new skill is its own registration surface; this checklist covers only the
     binding.
   - Alternatively **add** an entry to `SKILL_EXEMPT_SUBBINARIES` when *no*
     SKILL.md invokes the token. An exemption whose token is invoked is
     rejected, as is one naming an undispatched token, and at least one
     dispatched token must remain non-exempt.
8. **Add** `accelerator-<token>` — the cargo `[[bin]]` name from point 3, not
   the bare token — to `_CLI_RELEASE_BINARIES` (`tasks/build.py`).
   `cli_cross_compile` stages via `cli_binary_path(name, platform)`, i.e.
   `dist/release/<name>-<platform>`, which equals `subbinary_asset_path(token,
   platform)` **only** because of that prefix; a bare token stages
   `dist/release/<token>-<platform>` and signing then fails on a missing
   `accelerator-<token>-<platform>`. It also gives you `_assert_magic_bytes`
   and, for musl, `_assert_static_elf`, and `build:cli:cross-compile` is already
   called from `prerelease_prepare` and `release_prepare` (`tasks/release.py`).
   **No action when** you take that route — no new task and no `mise.toml` leaf
   are needed. A crate that cannot ride that loop needs its own staging task
   wired into **both** prepare tasks *and* a `mise.toml` leaf, and owes
   `_assert_static_elf` explicitly for musl. **[release]**
9. **Update** all three `attest-build-provenance` blocks in
   `.github/workflows/main.yml`, identically, **only when** the release
   publishes an artefact that no existing `subject-path` glob matches — today, a
   symbolication archive written into a committed `bin/` tree (point 12). The
   condition is about the published *artefact*, not the sub-binary: the
   sub-binary is always staged in `dist/release/`, which
   `dist/release/accelerator-*` covers — but `manifest.json` and
   `manifest.minisig` live there too and were matched by nothing until they were
   added explicitly, which is why the rule is stated this way. **[PR]** — a test
   derives the expected coverage from `_release_uploads()`.
10. **Update** `BUILTIN_SUBCOMMANDS` (`tasks/shared/dispatch_coherence.py`) in
    lockstep whenever the launcher's built-in set changes in either direction.
    **No action when** you are only adding a dispatch token — but note a name in
    that set is unavailable as one. A test pins the set against the clap
    `Command` enum, so the two cannot drift. **[PR]**
11. **Document** the sub-binary for users: its own page under
    `docs-site/src/content/docs/`, an entry in the **Concepts** list under
    `## Documentation` in the root `README.md`, and an
    `ACCELERATOR_<TOKEN>_BIN` override row wherever that sub-binary's overrides
    are documented (`docs-site/src/content/docs/visualiser.md` is the
    visualiser's). Starlight derives the sidebar and the prev/next chain from
    `docs-site/astro.config.mjs`, so a new page means adding it there too.
    `docs-site/src/content/docs/internals.md`'s env-var table holds only
    launcher-wide inputs and is already token-generic. **No action when** the
    sub-binary is not user-facing. **[author]**
12. **Add** an entry to `DEBUG_ARCHIVE_DIRS` (`tasks/shared/paths.py`) when the
    sub-binary ships a symbolication archive, and update the registry pin in
    `tests/unit/tasks/shared/test_paths.py`. The value must be a `bin/`
    directory — `.gitignore`'s rule is `**/bin/*.debug.tar.gz`, so an archive
    written elsewhere would be committed by the release path's `git add .`. This
    is what triggers point 9's obligation. **No action when** the sub-binary
    ships no symbolication archive — omitting an entry silently ships no archive
    and nothing catches it, though the shape of an entry you *do* add is checked
    by `_debug_archive_targets`. **[author]**
13. **Extend** `cli/deny.toml` when the new crate's dependency graph needs a
    licence or advisory exception, with a comment giving the justification. **No
    action when** `mise run deny:check` is already green. **[PR]**

Points 1, 2, 3, 4, 7 and 8 must land in the **same change**. The release path
resolves them together, and only the 1↔7 pair is caught before the release job —
by the dispatch guard, which runs from `tasks/manifest.py` on every release
*and* as `lint:dispatch-coherence:check` in `build-system:check`.

The Cargo **package** is `accelerator-<token>`; where a domain crate already
owns `cli/<token>/`, the binary crate lives elsewhere with a
`_SUBBINARY_MANIFESTS` entry, because `tasks/manifest.py` defaults the manifest
path to `cli/<token>/Cargo.toml` and cargo-pup rules match on whole crate names.
A crate carrying domain modules may also owe a `cli/pup.ron` rule; that is the
generic add-a-Rust-crate surface, not part of this checklist.

The **token** must match `^[a-z][a-z0-9-]*$`. Underscores are rejected because
the token derives `ACCELERATOR_<TOKEN>_BIN`
(`cli/launcher/src/launch/core.rs`), which the launcher refuses to build from a
name outside that set — so an underscore token can never resolve an override.

`verify` and `launcher` are **reserved**, and a name in `BUILTIN_SUBCOMMANDS` is
unavailable as a token. `verify` collides on the staged asset name:
`cli_binary_path("accelerator-verify", …)` and `subbinary_asset_path("verify",
…)` both yield `dist/release/accelerator-verify-<platform>`, so registering it
would sign the vendored verify shim and advertise it in the manifest. Both
`verify` and `launcher` additionally shadow real `cli/<name>/` crates through
`_SUBBINARY_MANIFESTS`' default. A built-in-shadowed token would be signed and
listed in the manifest but never dispatched. All three constraints are enforced
by the dispatch guard.

## CI job → local command

Each CI check job mirrors a single `mise run` task, so a red job is reproducible
locally with the mapped command:

| CI job (`.github/workflows/main.yml`) | Local command                             |
| ------------------------------------- | ----------------------------------------- |
| `check-cli`                           | `mise run cli:check`                      |
| `check-supply-chain`                  | `mise run deny:check`                     |
| `check-architecture`                  | `mise run pup:check` (+ `test:integration:pup`) |
| `check-zero-spawn`                    | `mise run test:integration:zero-spawn` (PATH-only; the CI job runs `test:integration:zero-spawn:strong`, which shadows absolute paths and needs `ACCELERATOR_ZERO_SPAWN_SHADOW=yes`) |
