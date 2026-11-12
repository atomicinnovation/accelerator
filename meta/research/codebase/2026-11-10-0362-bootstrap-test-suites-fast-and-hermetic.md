---
type: "codebase-research"
id: "2026-11-10-0362-bootstrap-test-suites-fast-and-hermetic"
title: "Making the bootstrap test suites fast and hermetic (0362)"
date: "2026-11-10T16:24:28+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0362"
parent: "work-item:0362"
relates_to: ["codebase-research:2026-10-28-full-build-duration-contention-and-slowness"]
topic: "Making the bootstrap test suites fast and hermetic (0362)"
tags: ["research", "codebase", "build-system", "testing", "bootstrap", "minisign", "hooks", "cargo-profile"]
revision: "b677d0f904ee4a9affea5807cd97caf41f4fe179"
repository: "accelerator"
last_updated: "2026-11-10T16:24:28+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Making the bootstrap test suites fast and hermetic (0362)

**Date**: 2026-11-10T16:24:28+00:00
**Author**: Toby Clemson
**Git Commit**: b677d0f904ee4a9affea5807cd97caf41f4fe179
**Branch**: (jj working copy, unpushed)
**Repository**: accelerator

## Research Question

What does the codebase look like where work item 0362 touches it? Specifically:

- the `minisign-verify` opt-level override (Requirement 1);
- rewriting the hooks launcher-dispatch smoke test onto `make_installation`
  (Requirement 2);
- the new attacker-key refusal case under `--fail-safe` (Requirement 3).

Which facts, constraints and gaps should the plan account for?

## Summary

- **Requirement 1 is a one-table change.** It carries one placement
  constraint, set by the vendored-shim drift marker. The table also speeds
  up more than the shim: the launcher depends on `minisign-verify` as well.
- **Requirements 2 and 3 are feasible with the existing harness.**
  - `make_installation`, `run_bootstrap`, `sign`, `download_log` and
    `LAUNCHER_STUB_SRC` already give everything the acceptance criteria
    need.
  - No new harness options are required.
  - `run_bootstrap` builds a minimal environment from scratch. The tests
    must therefore pass `ACCELERATOR_VCS_BIN`, `ACCELERATOR_LOG=error` and
    (optionally) `ACCELERATOR_CACHE_DIR` through `extra_env`.
  - `Installation` does not carry the keys directory, so the control step
    needs the `keys` path kept separately.
- **The `--fail-safe` refusal path is fully determined by `bin/accelerator`.**
  - The exit status is 0.
  - stderr carries
    `accelerator: could not fetch and verify the accelerator launcher for <platform>`.
  - One line is appended to `<cache>/.accelerator-unverified.log`.
  - The temporary files are removed and nothing reaches the cache path.
  - The refused launcher never reaches the cache, so the re-sign control
    fetches again with no extra cache manipulation.
- **Several line references in the work item are stale** (see
  [Discrepancies with the work item](#discrepancies-with-the-work-item)).
  One premise is also slightly off: the sibling
  `test_research_guard_registration.py` uses the `build:cli:dev` launcher
  but copies it with `shutil.copy2`, not `claim_artefact`.

## Detailed Findings

### Requirement 1: the `minisign-verify` opt-level override

**Today's profiles**

- `cli/Cargo.toml:286-291` holds the only profile table, `[profile.release]`
  (`strip = true`, `lto = "thin"`), with a comment explaining why size
  matters.
- There is no `[profile.dev]` or `[profile.dev.package.*]`.

**The dependency**

- `minisign-verify` is pinned at `cli/Cargo.toml:99` (`=0.2.5`).
- Both `cli/verify/Cargo.toml:22` and `cli/launcher/Cargo.toml:47` take it
  from the workspace.
- `cli/Cargo.lock:3428-3431` lists no dependencies for it: the BLAKE2b and
  Ed25519 code is built into the crate. So
  `[profile.dev.package.minisign-verify] opt-level = 3` covers all of the
  hot code, with no transitive crates to name.

**How the shim verifies** (`cli/verify/src/main.rs:36-42`)

- It reads the whole target with `std::fs::read`.
- It calls `public_key.verify(&target, &signature, false)`, which accepts
  prehashed signatures only and hashes over an in-memory buffer.
- The hash over the 76 MB debug launcher is the hot loop the research
  profiled.

**The launcher**

- The launcher verifies by streaming
  (`cli/launcher/src/launch/outbound/resolve/keys.rs:84-113`, 64 KiB
  chunks).
- The override speeds up the debug launcher's own sub-binary verification
  as well. That is a side benefit; no test depends on it.

**Checks that read `cli/Cargo.toml` as text**

- `tasks/build.py:456-460` (`_minisign_verify_pin`):
  - The vendored-shim drift marker hashes the **first** line whose stripped
    text starts with `minisign-verify` (`tasks/build.py:547-590`). It is
    checked by `tasks/lint/vendor_shims.py:34` and
    `tests/unit/tasks/test_build.py:239-245`.
  - A `[profile.dev.package.minisign-verify]` header starts with `[`, so it
    does not match.
  - The table must still sit **after** line 99, and no line inside it may
    start with `minisign-verify`.
  - Next to `[profile.release]` is the natural place for it.
- `cli/vcs-cli/src/report.rs:244-265`:
  - It slices the `[profile.release]` section up to the next `\n[` and
    asserts it contains no `panic`/`abort`.
  - A new table placed after it does not change that slice.
- Everything else reads specific keys through `tomllib` or `tomlkit`, so an
  extra table is ignored. Cargo-deny (`cli/deny.toml`), cargo-pup
  (`cli/pup.ron`) and clippy configuration do not mention profiles.
- The drift marker does not hash profile settings at all, so the dev
  override cannot invalidate the committed release shims.

**Evidence**

The research measured the effect; this document did not re-measure it. See
`meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md:254-282`.

| Verifier | Time to verify the 76 MB debug launcher |
|---|---|
| Debug shim | 1.15 s |
| Debug shim with opt-level 3 | 0.06 s |
| Committed release shim | 0.06 s |

The research also sampled the suite: the shim was running in 299 of 328
samples. It measured the rebuild cost of the override at 1.8 s.

**Stale docstring**

`tests/integration/skill-invocation/test_skill_invocation_conformance.py:50`
says a duplicate case costs "~0.1s a go". It is about 1.1 s today (1.75 s
under load), and about that after the fix. Correcting the docstring belongs
in the Requirement 1 pull request.

### The bootstrap's verification and refusal path (`bin/accelerator`)

**`--fail-safe`** (`:35-46`)

- `abort_status=1` by default.
- The first `--fail-safe` before any `--` sets it to 0.
- The token stays in argv, so the launcher also sees it.

**`fail`** (`:51-54`) prints `accelerator: <msg>` to stderr and exits with
`abort_status`.

**`fail_integrity`** (`:61-71`)

- Appends `<UTC ISO timestamp> pid=<pid> <message with newlines replaced by spaces>`
  to `unverified_log`, then calls `fail`.
- `unverified_log` is provisional at `:120`. It becomes
  `${cache_dir}/.accelerator-unverified.log` at `:219`.

**Cache directory** (`resolve_cache_dir`, `:198-207`)

- `ACCELERATOR_CACHE_DIR` if set, otherwise `${plugin_root}/bin`.
- There is no XDG fallback.

**The shim**

- It is always `${plugin_root}/bin/accelerator-verify-${platform}`
  (`:165-167`); no environment variable overrides it.
- It is staged into the cache as
  `accelerator-verify-${platform}-${sha256}` and run from there
  (`:300-352`).
- `make_installation` places the debug shim at the source path, which is
  how the debug shim enters the chain.

**The public key** is `${plugin_root}/keys/accelerator-release.pub`
(`:168-170`).

**Release URLs**

- `base_url` defaults to `https://github.com/.../download/v${version}`
  (`:406`); `ACCELERATOR_RELEASE_BASE_URL` overrides it.
- The assets are `accelerator-${platform}` and
  `accelerator-${platform}.minisig` (`:418`, `:422`).
- The downloader is overridden by `ACCELERATOR_BOOTSTRAP_DOWNLOADER`
  (`:148-163`).

**Re-verification on every call**

- `ensure_launcher` (`:435-455`) returns early only if
  `[[ -x launcher ]] && [[ -f launcher_sig ]] && verify_launcher`.
- So every cache hit runs the shim over the whole launcher. This is the
  cost Requirement 1 removes.

**`fetch_and_verify`** (`:415-433`)

1. Fetches into `${cache_dir}/.tmp-launcher-$$` and its `.minisig`.
2. On any fetch or verify failure, `rm -f`s both and returns 1.
3. Only on success does it `mv` the signature, then the binary, into
   `accelerator-launcher-${version}-${platform}`.

There is no trap-based cleanup; the explicit `rm -f` calls are the only
cleanup. That is why Requirement 3's assertion that the cache holds no
launcher and no `.tmp-launcher-*` file holds.

**The call site** (`:457-459`):

```
ensure_launcher || fail_integrity "could not fetch and verify the accelerator launcher for ${platform}"
```

**`ACCELERATOR_BIN`** is not read anywhere in `bin/`.

**The dev-launcher override** (`:221-254`)

- It needs all three of:
  - `ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER=1`;
  - the `${plugin_root}/.accelerator-dev-launcher` marker;
  - an `ACCELERATOR_LAUNCHER_BIN` contained under
    `${plugin_root}/cli/target/`.
- It skips fetch, cache and verification, which is why 0362 rejects it for
  the smoke test.

### The test harness (`tests/integration/support/installation.py`)

**`make_installation(root, server, *, keys, shim, bootstrap, alias, label="self", secret="release", launcher_source=None, templates=None)`**
(`:200-249`) writes:

- `plugin.json` with version `9.9.9-test` (`:47`, `:234-236`);
- `keys/accelerator-release.pub`, always copied from `keys/release.pub`
  (`:239`);
- `bin/accelerator`, copied from `REPO_BOOTSTRAP` (`:240`);
- the shim, at `bin/accelerator-verify-<alias>` (`:241-243`);
- the served launcher, through
  `serve_launcher(server, alias, keys/f"{secret}.key", source=launcher_source)`
  (`:245-248`).

The `bootstrap` parameter is accepted but **never used** (`:240` calls
`copy_bootstrap` on the repository's bootstrap directly). That is harmless
for 0362, but worth knowing.

**`serve_launcher`** (`:174-190`) writes the stub, or `shutil.copy(source)`,
to `server/accelerator-<alias>`, sets mode 0755, and signs it.

**`sign(secret_key, target)`** (`:156-171`)

- Runs `minisign -S -s <key> -m <target> -x <target>.minisig`.
- Checked in this research: re-signing an already signed file with a
  different key overwrites the `.minisig`, and the result verifies against
  the new key. So
  `sign(keys/"release.key", installation.server / f"accelerator-{alias}")`
  is enough for Requirement 3's control.

**`Installation`** (`:193-197`) is frozen, with `root`, `server` and
`sentinel`. It has no keys, cache or log fields:

- the keys directory must be kept by the test itself;
- the cache directory is `root/bin` unless `ACCELERATOR_CACHE_DIR` is
  passed;
- the download log is `server/dl.log`, read through `download_log(server)`
  (`:344-346`).

**`LAUNCHER_STUB_SRC`** (`:57-73`)

- Writes argv to `$LAUNCHER_ARGS_OUT` and the environment to
  `$LAUNCHER_ENV_OUT`.
- Exits with `$LAUNCHER_EXIT` (default 0).
- Emits nothing on stdout, which is why Requirement 2's hook-JSON assertion
  rules out the stub.

**`DOWNLOADER_SRC`** (`:78-92`) appends each URL to `$DL_LOG` and serves
`$SERVER_DIR/<basename>`. It exits 22 if the file is missing.

**`run_bootstrap(root, server, downloader, *, args, xtrace, extra_env, path, entry, cwd, timeout)`**
(`:278-337`)

- Its base environment is only:
  - `PATH` and `HOME`;
  - `ACCELERATOR_BOOTSTRAP_DOWNLOADER`;
  - `ACCELERATOR_RELEASE_BASE_URL=https://example.invalid/v9.9.9-test`;
  - `SERVER_DIR` and `DL_LOG`.
- `extra_env` is merged on top before `assert_hermetic` runs.
- `cwd` defaults to a fresh empty temporary directory outside any checkout,
  which suits the "outside any checkout" requirement.
- It runs under `/bin/bash`, so the bash 3.2 floor applies on macOS.

**`assert_hermetic`** (`:252-275`) requires:

- the downloader variable is present;
- the release host ends in `.invalid` or is `http://127.0.0.1`;
- the entry is not, and does not resolve to, the repository's own
  `bin/accelerator`.

The launcher reads the same `ACCELERATOR_RELEASE_BASE_URL`
(`cli/launcher/src/main.rs:158-169`), so it is confined too.

**`tests/support/artefacts.py`**

- `claim_artefact` (`:37-53`) copies a built binary into a process-lifetime
  temporary directory. It retries `FileNotFoundError` for up to 5 s.
- `_cargo_build` (`:56-77`) memoises per process.
- `build_shim()` (`:80-82`) runs `cargo build -p accelerator-verify`. No
  mise task builds the shim.
- `build_launcher()` (`:85-95`) runs
  `--bin accelerator --features accelerator/test-loopback`. Its docstring
  forbids pairing it with a `build:cli:dev` edge (cargo target-lock
  contention).

**Launcher variant**

`build:cli:dev` builds the launcher with the same feature:
`tasks/shared/dev_builds.py:48`,
`LAUNCHER = DevBinary("accelerator", ..., loopback_seam=True)`, which adds
`accelerator/test-loopback`. So `cli/target/debug/accelerator` is the same
variant whichever suite last built it. A claimed copy taken during a full
run is therefore never the "wrong" launcher, only one possibly re-linked
under concurrent builds, and `claim_artefact` exists for exactly that case.

**Fixture scoping**

There is no `conftest.py` under `tests/integration`; each suite defines its
fixtures module-locally.

- The entrypoint suite's `make_harness` factory
  (`tests/integration/entrypoint/test_accelerator_entrypoint.py:126-160`)
  gives every test a fresh root, server and cache.
- Skill-invocation shares one module-scoped installation and cache
  (`test_skill_invocation_conformance.py:68-82`).
- Both duplicate a session-scoped autouse `repo_bin_is_untouched` backstop
  (entrypoint `:103-113`, skill-invocation `:60-65`).

### The hooks suite today

**`tests/integration/hooks/test_vcs_detect_registration.py`**

`_dispatch_env` (`:30-45`):

- copies `os.environ`;
- sets `ACCELERATOR_VCS_BIN` and `ACCELERATOR_PLUGIN_ROOT` by `setdefault`;
- forces `ACCELERATOR_LOG=error`, because the sub-binary override logs an
  INFO line that would break the empty-stderr assertion;
- forces `ACCELERATOR_CACHE_DIR`, so the bootstrap does not stage into the
  repository's `bin/`.

`test_launcher_dispatch_smoke_on_a_plain_non_repo_dir` (`:48-82`):

- runs the **repository's** `bin/accelerator vcs detect --format=hook --fail-safe --descriptive`
  with `cwd=tmp_path`;
- sets no dev-override variables, so the bootstrap curls the published
  launcher.

Its assertions are weaker than 0362 requires:

- rc 0;
- empty stderr;
- stdout parses as JSON *only if non-empty*;
- no prohibition phrases;
- no `WORKSPACE BOUNDARY DETECTED`.

Under `--fail-safe` a fetch failure exits 0, so today only the
empty-stderr check catches a broken fetch.

The registration test (`:90-104`) pins `_DETECT_COMMAND` to the
`hooks/hooks.json:9` `SessionStart` command.

**`tests/integration/hooks/test_research_guard_registration.py`**

- Uses the `build:cli:dev` launcher through the dev override, with a plain
  `shutil.copy2` into `root/cli/target/debug/accelerator` (`:88-90`).
- Does not call `claim_artefact`.
- Bypasses `run_bootstrap`.
- Its docstring (`:6-8`) says a real signature failure cannot be produced
  offline. The harness's `secret="attacker"` already does that, and
  Requirement 3 adds such a test right next to it.

**`tests/integration/hooks/test_launcher_link_refresh.py`** needs no
compiled binaries and is unaffected.

**How the suite is built and run**

- `mise.toml:462-465` declares `test:integration:hooks` with
  `depends = ["build:cli:dev", "build:cli:vcs:dev", "build:cli:research:dev"]`.
- `tests/unit/tasks/test_mise.py:64-74` (`_CLI_DEV_BUILDS`) pins those
  edges.
- `tasks/test/integration.py:120-133` runs
  `uv run pytest tests/integration/hooks -v` with
  `accelerator_env(vcs_bin=True)`.
- `tasks/test/helpers.py:9-63` sets `ACCELERATOR_BIN`,
  `ACCELERATOR_PLUGIN_ROOT` and `ACCELERATOR_VCS_BIN`, defaulting to
  `cli/target/debug/...`.
- The rewritten smoke test still needs `ACCELERATOR_VCS_BIN`. Because
  `run_bootstrap` does not inherit the environment, it must be read from
  `os.environ` (or computed) and passed through `extra_env`.
- Adding `build_shim()` to the hooks suite creates one in-pytest cargo
  build. It does not conflict with the `build_launcher()` docstring rule,
  because that rule concerns the launcher only. 0366 already expects this
  build (its Dependencies section).

### What the launcher and `accelerator-vcs` do for the smoke test

**The override short-circuit**

- `LazyProductionResolver::resolve` (`cli/launcher/src/main.rs:171-204`)
  checks `override_path` first
  (`cli/launcher/src/launch/outbound/mod.rs:21-47`).
- The override name is derived as `ACCELERATOR_VCS_BIN`
  (`cli/launcher/src/launch/core.rs:412-437`).
- When set, `release_access()` never runs: no key parse, no fetch, no
  signature check.
- It logs `tracing::info!`, hence `ACCELERATOR_LOG=error`.
- `re_resolve` declines for an override.

**The embedded key** comes from `cli/launcher/build.rs:29-39` and
`keys.rs:11-12`. It is the committed key, which no per-run key can satisfy.
That is why the override stays.

**Launcher `--fail-safe`** (`main.rs:613-630`, `core.rs:280-290`)

- It swallows `Failed` dispatch errors (exit 0, `warn!`).
- It does not swallow `Refusal`s.
- This is irrelevant to the happy path, but it explains why stderr stays
  empty when dispatch succeeds.

**Mode outside a checkout**

- `cli/vcs/src/mode.rs:48-61` returns `Mode::Git` when no `.jj` is found
  walking up.
- `cli/vcs/src/classify.rs:115-117` gives `Classification::None` when
  neither VCS is present, so there is no boundary block.

**Output** (`cli/vcs-cli/src/detect.rs`)

- `GIT_REFERENCE` (`:51-74`) begins exactly
  `This repository uses git as its version control system.`
- `render` (`:176-192`) with `--descriptive` emits the reference, and
  appends `\n\n<boundary>\n` only if a boundary exists.
- The boundary block (`:149-171`) starts with `WORKSPACE BOUNDARY DETECTED`.

**Envelope** (`cli/kernel/src/hooks.rs:9-23`)

```
{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"…"}}
```

**vcs-cli `--fail-safe` on adapter errors** (`detect.rs:206-229`,
`hooks.rs:52-58`)

- It emits a bare `{"systemMessage": …}` with no `hookSpecificOutput`.
- The `hookEventName == "SessionStart"` assertion therefore also catches a
  swallowed adapter failure. That is a useful property for the criterion to
  rely on.

### Patterns to copy for Requirement 3

**`test_trust_chain_failure_records_durably_under_fail_safe`**
(`tests/integration/entrypoint/test_accelerator_entrypoint.py:1217-1237`)
is the closest model:

```python
assert result.returncode == 0
assert "<message>" in log.read_text().splitlines()[-1]
```

**`test_non_release_key_signature_is_refused`** (`:254-262`)

- Uses `secret="attacker"` and asserts `returncode != 0` plus `"verify"` in
  the output.
- It does not check the download log, the cache or the unverified log;
  Requirement 3 goes further.

**`test_tampered_cached_launcher_is_refused_and_healed`** (`:237-251`)
re-fetches on a broken cache entry. Healing involves no `fail_integrity`,
so it writes no log entry.

**`LAUNCHER_ARGS_OUT` usage**: `test_happy_path_forwards_args_and_exit_code`
(`:206-222`).

**Expected observable state for the attacker case under `--fail-safe`**,
read off the code:

- `download_log(server)` holds two URLs, the launcher and `.minisig`;
- rc 0;
- stderr holds `accelerator: could not fetch and verify the accelerator launcher for <alias>`;
- `.accelerator-unverified.log` has exactly one line containing that
  message;
- no `LAUNCHER_ARGS_OUT` file;
- no `accelerator-launcher-*` or `.tmp-launcher-*` in the cache;
- a staged `accelerator-verify-<alias>-<digest>` and its lock directory may
  exist, so the empty-cache assertion must target the launcher and
  temporary-file names rather than "directory is empty".

**The control step**

- After `sign(keys/"release.key", server/f"accelerator-{alias}")`, a second
  call fetches again, because nothing was cached.
- The download log grows to four lines.
- The stub writes its `LAUNCHER_ARGS_OUT` record.

### Release-time re-verification (what the removed check relies on)

- `tasks/release.py:172-180` (`_publish`) ends with
  `github.upload_and_verify_release`.
- `tasks/github.py:176-189` (`_run_shim`) runs the vendored host shim
  against the committed `keys/accelerator-release.pub`. Failure raises
  `AssetVerificationError`, and `upload_and_verify_release` (`:442-448`)
  keeps the release in draft.
- `tests/integration/tasks/test_github.py:466-483` covers that path with
  mocks.

### CI timing visibility

- `.github/workflows/main.yml:63-110`: the `test-integration` matrix job's
  final step is `mise run test:integration`. All suites run in parallel in
  that one step, so their output is interleaved.
- No `--durations` is configured. The per-suite duration for the CI
  criterion comes from pytest's `N passed in X.XXs` summary under the
  `[test:integration:skill-invocation]` prefix.
- That figure excludes mise scheduling, but includes the in-fixture cargo
  builds, which run inside pytest.

## Discrepancies with the work item

| Work item says | Codebase at b677d0f9 |
|---|---|
| Smoke test at `test_vcs_detect_registration.py:30-81` | Test function is `:48-82`; `_dispatch_env` is `:30-45` |
| `build:cli:dev` edge at `mise.toml:457-460` | `test:integration:hooks` is `mise.toml:462-465`; `build:cli:dev` itself is `:214-216` |
| Hooks task at `tasks/test/integration.py:114-127` | `:120-133` |
| `release_base_url()` at `cli/launcher/src/main.rs:55-59` | `:158-169` |
| Resolver at `cli/launcher/src/main.rs:72-92` | `LazyProductionResolver` `:171-204` |
| `test_research_guard_registration.py` "already uses that build" with `claim_artefact` (implied) | It uses the build, but through `shutil.copy2` and the dev override, not `claim_artefact` |
| Research cites `[profile.release]` at `cli/Cargo.toml:260-262` | Now `:286-291` |
| Skill-invocation runs 143 cases | The module docstring says 122 distinct of 204 occurrences; the count was not re-collected here |

The `bin/accelerator` line references in the work item (`:51-54`, `:61-71`,
`:409-435`, `:415-433`) are still accurate.

## Code References

- `cli/Cargo.toml:99` - `minisign-verify = "=0.2.5"` pin (the table must
  sit after it)
- `cli/Cargo.toml:286-291` - `[profile.release]`, the only profile today
- `cli/verify/src/main.rs:36-42` - whole-file read and prehashed verify
- `cli/launcher/Cargo.toml:47` - the launcher also depends on
  `minisign-verify`
- `tasks/build.py:456-460`, `:547-590` - vendored-shim drift marker that
  hashes the first `minisign-verify` line
- `cli/vcs-cli/src/report.rs:244-265` - test that slices
  `[profile.release]` as text
- `bin/accelerator:35-54` - `--fail-safe` parsing and `fail`
- `bin/accelerator:61-71`, `:120`, `:219` - `fail_integrity` and the
  unverified-log path
- `bin/accelerator:165-170` - shim and public-key paths under the plugin
  root
- `bin/accelerator:404-459` - cache names, `verify_launcher`,
  `fetch_and_verify`, `ensure_launcher`, the refusal message
- `tests/integration/support/installation.py:57-92` - stub launcher and
  stub downloader sources
- `tests/integration/support/installation.py:156-190` - `sign`,
  `serve_launcher`
- `tests/integration/support/installation.py:200-249` - `make_installation`
- `tests/integration/support/installation.py:252-337` - `assert_hermetic`,
  `run_bootstrap`
- `tests/integration/support/installation.py:344-346` - `download_log`
- `tests/support/artefacts.py:37-95` - `claim_artefact`, `_cargo_build`,
  `build_shim`, `build_launcher`
- `tasks/shared/dev_builds.py:48` - the `build:cli:dev` launcher carries
  `test-loopback`
- `tests/integration/hooks/test_vcs_detect_registration.py:30-82` - today's
  smoke test
- `tests/integration/hooks/test_research_guard_registration.py:65-135` -
  sibling test using the dev override
- `tests/integration/entrypoint/test_accelerator_entrypoint.py:206-262`,
  `:1217-1237` - patterns for the refusal case
- `tests/integration/skill-invocation/test_skill_invocation_conformance.py:50`
  - stale "~0.1s a go" docstring
- `tests/integration/skill-invocation/test_skill_invocation_conformance.py:68-113`
  - shared installation and invocation
- `mise.toml:462-465`; `tasks/test/integration.py:120-133`;
  `tasks/test/helpers.py:9-63` - hooks lane wiring
- `tests/unit/tasks/test_mise.py:64-80` - pinned build edges per suite
- `cli/launcher/src/main.rs:158-204` - `release_base_url`, sub-binary
  resolver with override short-circuit
- `cli/vcs/src/mode.rs:48-61`; `cli/vcs-cli/src/detect.rs:51-74`,
  `:149-229`; `cli/kernel/src/hooks.rs:9-58` - `vcs detect` output
- `tasks/release.py:172-180`; `tasks/github.py:176-189`, `:442-448`;
  `tests/integration/tasks/test_github.py:466-483` - release-time
  re-verification
- `.github/workflows/main.yml:63-110` - CI integration job

## Architecture Insights

- **One trust chain under test.** The harness exercises the real
  fetch → verify → cache → exec chain and substitutes only:
  - the transport (stub downloader);
  - the trust root (a per-run keypair placed at the committed key's path);
  - the payload (stub or real launcher).

  Optimising one crate keeps that fidelity at release speed. The shim and
  launcher remain exactly what ships, except for the optimisation level.
- **`run_bootstrap` is a funnel, not a fixture.** It composes the
  environment explicitly, so ambient `ACCELERATOR_*` variables never leak
  in. A hooks test moving onto it must opt in to every variable it needs.
  This is the reverse of today's `_dispatch_env`, which copies
  `os.environ`.
- **`--fail-safe` deliberately hides refusals from the exit status.**
  Integrity failures are made observable instead through the durable
  `.accelerator-unverified.log` record. Requirement 3 tests exactly that
  contract for the hook invocation shape.
- **Feature-unified dev launcher.** `build:cli:dev` and `build_launcher()`
  produce the same `test-loopback` binary, so the concurrent builds can
  contend but cannot diverge.

## Historical Context

- `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`
  - The source of A1 and A7 (table rows at lines 457 and 463).
  - Shim evidence at 254-282.
  - Smoke-test fetch evidence at 341-353: 42 s fetch, 0.32 s on a cache
    hit.
  - CI per-case cost at 716-739: run 40601848159, ubuntu skill-invocation
    1,266 s, median 8.0 s per case, first case 154 s because it builds the
    fixture.
- `meta/work/0182-*` and `meta/plans/2026-07-27-0182-*`
  - Removed earlier tests that ran the repository's own `bin/accelerator`,
    for the same hazard as today's smoke test.
  - Established serving the compiled launcher through the stub downloader
    as the only workable route, and extracted `installation.py`.
- `meta/work/0165-*` and `meta/plans/2026-07-06-0165-*:566-606` -
  release-time re-verification against the committed key, the check that
  "moves rather than disappears".
- `meta/validations/2026-08-03-0188-*:217-221` - the concurrent-relink
  failures that motivated `claim_artefact`.
- `meta/plans/2026-10-19-0218-bound-cache-root-growth.md` - the
  `.invalid`/loopback rule in `assert_hermetic` and its funnel test.
- `meta/work/0366-cut-rust-compile-cost.md:100-109` - expects 0362 to add a
  `build_shim()` to the hooks suite and to own the `minisign-verify`
  override.
- `meta/work/0368-*:116-121` - the baseline commit's hooks test depends on
  the published release staying available.
- `meta/measurements/build_2026-11-01T143523Z_engywook_warm-no-op_devtools.json`
  - `default` median 315.1 s.
  - Task medians: skill-invocation 221.9 s, entrypoint 22.6 s, hooks 2.7 s.
  - `fetch_failures` empty.

## Related Research

- `meta/research/codebase/2026-10-28-full-build-duration-contention-and-slowness.md`

## Open Questions

- **The hooks baseline is small.**
  - The 0368 baseline records `test:integration:hooks` at 2.7 s median.
    That is far below the research's 18–58 s and implies the 17 MB fetch
    was fast that day.
  - Requirement 2's wall-clock effect against 0361's baseline may be small.
    Its value is hermeticity and offline safety, which matches the work
    item's decision not to give the smoke test a speed bar.
  - The new suite adds a `build_shim()` cargo build (warm: a no-op check)
    and two bootstrap runs with the debug shim. After Requirement 1 these
    should cost well under a second each.
- **Where the hooks tests keep their fixtures.**
  - Requirements 2 and 3 each need keys, a downloader and a shim.
  - The entrypoint suite uses a function-scoped factory, and the hooks
    directory has no conftest.
  - The plan should decide whether to define module-local fixtures in
    `test_vcs_detect_registration.py`, or to add a hooks `conftest.py`. No
    integration directory has one today.
  - Whether to copy the `repo_bin_is_untouched` backstop is also open. The
    rewritten test no longer runs the repository's bootstrap, but the
    backstop costs nothing.
- **Skill-invocation case count.** The work item says 143 cases, while the
  module docstring says 122 distinct of 204. Collecting from the command
  line returned "no tests collected" in this session, so the live count was
  not confirmed. It does not affect any acceptance criterion, which all
  use ratios or durations.
- **The `bootstrap` parameter.** `make_installation`'s `bootstrap`
  parameter is unused. The plan can ignore it, or tidy it separately; it is
  out of 0362's scope.
