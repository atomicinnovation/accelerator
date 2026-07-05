---
type: plan-validation
id: "2026-07-03-0164-launcher-and-git-style-dispatch-validation"
title: "Validation Report: Launcher and Git-Style Dispatch Implementation Plan"
date: "2026-07-05T14:18:42+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: "pass"
parent: "plan:2026-07-03-0164-launcher-and-git-style-dispatch"
target: "plan:2026-07-03-0164-launcher-and-git-style-dispatch"
tags: [rust, launcher, dispatch, cli, fetch-verify-cache-exec, minisign, reqwest, bootstrap]
last_updated: "2026-07-05T14:18:42+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Launcher and Git-Style Dispatch Implementation Plan

### Implementation Status

- ✓ **Phase 1** — Dependency stack, error taxonomy, external dispatch + exec —
  fully implemented
- ✓ **Phase 2** — Real resolution: fetch → verify → cache (hermetic) — fully
  implemented
- ✓ **Phase 3** — Discoverable surface: manifest-driven help + `--help`
  delegation — fully implemented
- ✓ **Phase 4** — `bin/accelerator` bootstrap + `cli/verify/` shim crate — fully
  implemented

All four phases are implemented and committed. Every automated success criterion
in the plan is green. The remaining `[~]`-marked items are explicit,
plan-documented deferrals to 0165 or tracked follow-ons — not incomplete 0164
work.

### Automated Verification Results

Run fresh from a clean tree during validation:

- ✓ `mise run cli:check` — rustfmt + clippy pedantic/nursery/restriction,
  `-D warnings` (exit 0)
- ✓ `mise run test:unit:cli` — **81 tests, 81 passed, 0 skipped** (unit +
  hermetic integration + shim black-box)
- ✓ `mise run deny:check` — the rustls trap is not sprung (exit 0)
- ✓ `mise run pup:check` — layering constraints hold; `launch::core` has its own
  inward-only rule (exit 0)
- ✓ `mise run scripts:check` — shfmt + ShellCheck + bashisms + exec-bits
  (exit 0); `bin/accelerator` is registered in all discovery mechanisms
- ✓ Python suites — **37 passed**: `test_accelerator_entrypoint.py` (hermetic
  bootstrap), `test_manifest_contract.py` (shared contract + cross-language
  alias coherence), `test_msrv_coherence.py`, `test_bootstrap_coverage.py`
  (three-tool coverage), `test_launcher_feature_graph.py` (deny feature graph:
  ring/hickory-resolver/rustls present; aws-lc-rs/native-tls/openssl/host-cert
  crates absent)

The aggregate `mise run` (full CI mirror) was **not** re-run end-to-end during
validation; the plan phases themselves marked it `[~]` on the grounds that the
change surface is confined to `cli/`, `bin/`, `tasks/`, `tests/`, and CI YAML,
which do not touch frontend/server runtime code. The component-level equivalents
that CI would run were all executed above and are green.

### Code Review Findings

#### Matches Plan

- **External dispatch + exec** (`cli/launcher/src/launch/`): the
  `External(Vec<OsString>)` arm routes through a dedicated `launch` module;
  `ExternalCommand::from_raw` rejects the empty vector as a named error (no
  index panic); `OsString` preserves non-UTF-8 args to exec; `UnixExec` is
  process-replacing via `CommandExt::exec`.
- **`ACCELERATOR_<SUB>_BIN` override** (`launch/outbound/mod.rs` +
  `launch/core.rs`): checked before any resolution in the composed
  `LazyProductionResolver`; the single shared `derive_override_var` helper
  performs the documented total normalisation and rejects colliding-underscore,
  leading-digit, and empty names.
- **fetch → verify → cache** (`launch/outbound/resolve/`): collaborators are
  cleanly separated (`Fetcher`, `verifier`, `cache`, `cache_root`, `manifest`,
  `keys`); the orchestrator is a readable guard-clause sequence
  (override → cache-hit-verify → miss-fetch-verify-cache → self-heal).
- **Verification ordering** (`resolve/mod.rs` + `manifest.rs`): manifest
  signature over raw bytes → `schema_version` gate via a minimal envelope →
  version-equality anti-rollback → per-binary sha256 + minisign; a shared
  `load_manifest` serves both the resolve path and the help path so the gates
  cannot diverge. Re-verify runs on every cache hit before exec.
- **Cache safety** (`cache.rs`): 0600 temp-in-dir, exec-bit set before rename,
  atomic rename-by-inode keyed by name+version+checksum; replace-in-place
  self-heal fetches a verified successor before renaming over a corrupt entry;
  distinct `CorruptCacheAndRefetchFailed` diagnostic offline.
- **Cache root** (`cache_root.rs`): `${CLAUDE_PLUGIN_ROOT}/bin` probed for
  write+exec, `ACCELERATOR_CACHE_DIR` override, no XDG fallback; unset root and
  read-only root are named errors.
- **Fetcher** (`fetcher.rs`): https pin (incl. post-redirect via `https_only`),
  dotted-label redirect allowlist refusing `evil-githubusercontent.com` /
  `githubusercontent.com.attacker.net`, bounded retry with 404 not retried.
- **Help** (`help.rs`, `main.rs`): lazy `try_parse` → `ErrorKind::DisplayHelp`
  → `after_help`; built-ins never read the manifest; descriptions sanitised at
  the boundary over Unicode scalars.
- **Bootstrap** (`bin/accelerator`): bash-3.2 (`set -uo pipefail`, no bash-4
  constructs); named `CLAUDE_PLUGIN_ROOT` diagnostics; hardened curl/wget
  (`--proto '=https'`, bounded redirects/size/time, cert check never disabled);
  shim run by absolute path from the resolved cache root; `mkdir`-based
  per-target lock with PID-owner stale-lock reclaim; replace-in-place healing;
  `exec "$@"` as an argv list.
- **Verify shim** (`cli/verify/`): third workspace member, MSRV-pinned, restriction
  lints, `Result`-propagating, statically links pure `minisign-verify`.
- **Contract artifact** (`manifest.example.json` + `manifest.schema.json`):
  present and cross-tested by Rust and Python; aliases single-sourced from
  `tasks/shared/targets.py`, which now carries the `UNAME_TO_ALIAS` table.
- **Version drift resolved**: `cli/Cargo.toml` is `1.24.0-pre.7`, matching
  `plugin.json`, and the cli workspace is in `validate_version_coherence`.

#### Deviations from Plan

All are reasonable; most are documented in the plan itself.

1. **`kernel::Error::Failed(String)` instead of
   `kernel::Error::Launcher(#[from] ResolutionError)`.** The plan (Phase 1 §3)
   wanted a typed variant carrying the resolution payload to the composition
   root. The implementation keeps the rich typed `launch::core::ResolutionError`
   enum (14 variants, structurally asserted in launcher-local tests) but
   flattens it to a string via `From<ResolutionError> for kernel::Error` at the
   boundary. The plan's *intent* — diagnostics that name the failed check
   (sha256 vs minisign) and the sub-binary, structurally assertable — is met at
   the launcher level; the typed payload simply does not survive into `kernel`.
   Acceptable: keeps `kernel` genuinely minimal.
2. **Fixture-key handling** (documented in the plan's own Phase 4 deviation note,
   lines 1013–1021). No test-only cargo feature and no separate "release-time
   no-fixture-key assertion"; instead the launcher always `include_str!`s the
   single committed `keys/accelerator-release.pub` (a placeholder 0165 replaces)
   and tests inject freshly-generated keys via config, so a fixture key **cannot**
   reach a release build by construction. This supersedes Decisions §5 and the
   Phase 4 §4 mention. The byte-identity guard (launcher-embedded key ≡
   bootstrap-shipped key) holds because `build.rs` single-sources that one file
   and the bootstrap reads the same file at `${CLAUDE_PLUGIN_ROOT}/keys/…`.
3. **Entrypoint test ported to Python.** The plan's Phase 4 criterion names
   `bash scripts/test-accelerator-entrypoint.sh`; that shell script was removed
   and reimplemented as `tests/integration/entrypoint/test_accelerator_entrypoint.py`
   (the most recent commit). It passes and is auto-discovered by the Python
   integration suite. A post-plan improvement, not a regression.
4. **MSRV CI leg dropped for a coherence test** (documented in Phase 1 success
   criteria). A `check-cli-msrv` compile leg was added then removed as redundant
   — mise provisions rust 1.90.0 (the exact MSRV) for every cli job — and
   replaced with `test_msrv_coherence.py` (mise ↔ Cargo ↔ clippy agree) plus
   `--locked` on the cli clippy check.
5. **`manifest.json` not added to `validate_version_coherence`.** Decisions §4
   said 0164 owns adding `manifest.json`'s `version` to the coherence set. There
   is no *production* `manifest.json` in 0164 (fixtures only; 0165 emits it), so
   there is nothing to add to coherence yet; instead `test_manifest_contract.py`
   asserts the fixture `schema_version` matches the launcher and the fixture
   derives `version` from `CARGO_PKG_VERSION`. The coherence set does include the
   cli workspace, resolving the pre.2/pre.7 drift the plan flagged.
6. **`help::sanitize` strips whitespace controls (e.g. tab), not just C0/C1/ESC.**
   The plan prose said "preserving … whitespace scalars"; the implementation uses
   `!char::is_control()`, which also removes tab/newline. The plan's actual
   success criterion (C0/C1/ESC removed, multi-byte UTF-8 preserved, asserted by
   exact equality) is satisfied, and for a single-line help entry stripping tabs
   is harmless. Trivial.

#### Potential Issues

- None blocking. The typed-payload flattening (Deviation 1) means a future
  consumer wanting to branch on resolution-error *category* at the composition
  root would need to re-thread the typed error; today only `Display` is consumed
  by `main.rs`, so this is latent, not active.

### Deferred Items (plan-tracked, not 0164 gaps)

Each is marked `[~]` in the plan and scoped to 0165 or a follow-on:

- musl static-link / `hickory-dns`-without-getaddrinfo / bundled-cert-store
  verification — **0165** four-triple gating AC; 0164 closes the graph-level
  half via the deny feature-graph test.
- `otool -L`/`ldd` dynamic-link inspection of a release build — manual/**0165**.
- Fetcher idle/read-stall timeout and explicit deadline-vs-attempts terminal
  error class — deferred; current fetcher uses a per-request total timeout +
  bounded attempt count.
- Explicit per-cache-key `flock`/`FD_CLOEXEC` advisory lock and injected-barrier
  concurrency tests (Rust) — deferred; concurrent first-use is covered by a
  threaded resolution test relying on atomic rename-by-inode + idempotent
  `mkdir`.
- Bootstrap manifest-freshness anti-replay binding (fetched launcher sha256 must
  match a manifest entry) — **follow-on**; needs a launcher manifest entry + JSON
  parsing in bash. The bootstrap does verify the launcher `.minisig` against the
  committed key.
- PATH-planted-decoy, injected-barrier concurrent-bootstraps, and
  stalled-fetch/non-https-redirect explicit stubs — deferred; mitigated by
  construction (absolute-path shim, hardened flags).

### Manual Testing Required

1. Release-build link inspection (0165 territory, but confirmable on host now):
   - [ ] `otool -L` / `ldd` on a release `accelerator` shows rustls-only, no
     dynamic OpenSSL.
2. Live end-to-end (0165, needs real signed assets):
   - [ ] `${CLAUDE_PLUGIN_ROOT}/bin/accelerator version` on a clean machine
     against a live release fetches → verifies → caches → execs.
   - [ ] `accelerator --help` reads coherently with the synthesised external
     subcommands section beneath the built-ins (a test cannot sign under the
     embedded key).

### Recommendations

- Record the 0164 → 0165 handoff explicitly in 0165: swap the placeholder
  `keys/accelerator-release.pub` for the production key (with a two-key rotation
  overlap), emit the production `manifest.json`, add its `version` to
  `validate_version_coherence`, retire `checksums.json`, and cross-compile +
  reproducibly build the four-triple launcher and per-triple verify shims.
- Track the bootstrap manifest-freshness anti-replay binding as a concrete
  follow-on work item so the deferral is not lost.
- If a future caller needs to branch on resolution-error category at the
  composition root, revisit Deviation 1 (typed `kernel::Error` variant).
