---
type: plan-review
id: "2026-07-03-0164-launcher-and-git-style-dispatch-review-1"
title: "Plan Review: Launcher and Git-Style Dispatch"
date: "2026-07-03T19:23:13+00:00"
author: Toby Clemson
producer: review-plan
status: complete
target: "plan:2026-07-03-0164-launcher-and-git-style-dispatch"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, security, safety, portability, compatibility]
review_number: 1
review_pass: 2
tags: [rust, launcher, dispatch, cli, fetch-verify-cache-exec, minisign, security]
last_updated: "2026-07-04T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Launcher and Git-Style Dispatch

**Verdict:** REVISE

This is an unusually rigorous, well-researched plan: it faithfully extends the
existing hexagonal ports-and-adapters pattern, internalises the correct trust
model (minisign as the security boundary, sha256 as corruption-detection,
re-verify-before-every-exec against a user-writable cache), and specifies
resilience (layered timeouts, bounded retry, atomic-rename-or-nothing,
fail-closed) rather than assuming it. It earns strong marks across all eight
lenses. It is being sent back for REVISE not because it is weak but because the
lenses converged on a small number of high-consequence gaps — chiefly the
under-enforced fixture→production key handoff (a latent RCE root of trust), an
under-pinned cross-story contract with 0165, and a cache-eviction/locking model
that races concurrent exec — plus a set of precise, cheap-to-fix correctness and
portability sharpenings that are far easier to close now than after
implementation.

### Cross-Cutting Themes

- **The 0165 contract is pinned only by prose fixtures, not a shared artifact**
  (flagged by: architecture, compatibility, test-coverage, security) — the
  `manifest.json` schema, the asset-naming/URL template, the sha256 wire format,
  the platform-alias map, and the embedded pubkey are all shared with 0165 and
  its parallel development, but exist only as inline examples in each plan. The
  research itself names this the single highest-risk coordination item. Both
  stories can pass their own tests and still fail the first real end-to-end
  fetch.
- **The fixture→production minisign key handoff is under-enforced**
  (flagged by: security, compatibility) — the fixture keypair is embedded in a
  verify-any-of set with only a *documented* swap to 0165's production key. If
  the fixture public key survives into a release build, the repo-visible fixture
  private key becomes a permanent forging root of trust on real user machines.
  No automated check prevents this, and 0164's own tests *want* the fixture key
  to verify, so nothing here would catch it. This is the review's one critical
  finding.
- **Cross-language single-sourcing is a test, not a mechanism**
  (flagged by: code-quality, test-coverage, portability, compatibility) — the
  triple→platform-alias map is hand-maintained in Python (`targets.py`), Rust
  (launcher), and bash (`bin/accelerator`), guarded only by a coherence test
  whose oracle and completeness are unspecified. A test that checks only the
  four canonical aliases misses a bootstrap `case` arm that fails to normalise
  `amd64`/`aarch64`.
- **Cache eviction, locking, and exec handoff race under concurrency**
  (flagged by: correctness, safety) — cap-eviction operates across the whole
  cache root while the advisory lock is per-key; the lock is released before
  exec; and the cache-hit self-heal evicts a working entry before a verified
  replacement exists. Together these open windows where a concurrent invocation
  can unlink a binary another process is about to exec, or destroy the only copy
  when offline.
- **Manifest verification ordering & sanitisation need to be exact**
  (flagged by: correctness, code-quality, architecture, test-coverage) — the
  schema-version gate is specified *after* reading the version field (which
  requires parsing under a possibly-unrecognised schema); control-char stripping
  is attached only to the help path though it applies wherever manifest strings
  reach a terminal; and the manifest is verified by two independent code paths
  (resolution and help) with no shared owner.
- **Host-target-only validation defers musl-specific guarantees**
  (flagged by: portability, compatibility) — the properties most likely to
  break (musl static-link, hickory-dns bypassing getaddrinfo, bundled
  webpki-roots sufficing) are exactly the ones a darwin/linux-gnu host build
  cannot exercise; they go unproven until 0165.

### Tradeoff Analysis

- **Security vs Usability — the `ACCELERATOR_<SUB>_BIN` / `ACCELERATOR_CACHE_DIR`
  escape hatches**: these intentionally bypass verification (the offline/air-gapped
  path, AC6), but an environment-injection then becomes an arbitrary-code-exec
  primitive. Recommendation: keep the hatches (they are a real requirement) but
  document the trust boundary explicitly, ensure `ACCELERATOR_CACHE_DIR` changes
  only the *location* and never disables content re-verification, and log when
  either override is active.
- **Architecture vs Safety/Portability — the XDG fallback**: the 0136 research
  makes the `${CLAUDE_PLUGIN_ROOT}` cache a hard invariant (an XDG-resident
  binary breaks the `allowed-tools` glob match), yet the noexec/read-only
  fallback and luminosity parity argue for an XDG path. These genuinely conflict.
  Recommendation: resolve the open question explicitly — either drop the XDG
  fallback (plugin-root-or-named-error) to preserve the permission invariant, or
  document precisely how permission matching still holds for an XDG-resident
  binary before shipping that branch.

### Findings

#### Critical

- 🔴 **Security + Compatibility**: Fixture keypair in the verify-any-of set can
  ship to production as a valid forging root of trust
  **Location**: Decisions settled §5; Phase 2 §1 (embed pubkey); Migration Notes
  The fixture→production key swap is a *documented* handoff, not an enforced one.
  Because the fixture private key lives in the repo/CI to sign test fixtures, if
  the fixture public key remains in the trusted set at release, any holder of
  that private key can forge a binary the production launcher will accept —
  full RCE via the fetch/verify/exec path. Gate the fixture key behind
  `#[cfg(test)]`/a test-only feature so it cannot compile into a release binary,
  or add a 0164 (not deferred) release-time assertion that no fixture/test key is
  present in the embedded set.

#### Major

- 🟡 **Compatibility + Architecture**: The 0165 manifest/asset contract is pinned
  only by prose fixtures
  **Location**: Decisions §4; Phase 2 §2; What We're NOT Doing
  Extract the `manifest.json` schema (fields, types, `schema_version` semantics,
  signature encoding) and the asset-URL/filename template into a single committed
  contract artifact (schema file + golden fixture, or shared Rust type + Python
  constant) that both 0164 and 0165 consume and test against, so drift fails a
  shared test rather than the first production release.
- 🟡 **Compatibility**: New `manifest.json` is a second, divergent schema
  alongside the shipping `checksums.json` with no stated coexistence/migration
  **Location**: Decisions §4; Phase 2 §2
  State whether `manifest.json` supersedes, extends, or coexists with
  `checksums.json` (still written by `build.py`, read by `launch-server.sh`), and
  how `validate_version_coherence` and the all-zeros sentinel-digest contract
  carry forward.
- 🟡 **Compatibility**: Bare-hex sha256 in the manifest contradicts the
  established `sha256:`-prefixed wire format
  **Location**: Phase 2 §2
  The Python pipeline emits `"sha256:<hex>"` (`build.py:127`) and the shell
  launcher strips it; a strict bare-hex Rust reader silently fails on a prefixed
  digest. Either align on the prefixed format or require the reader to strip a
  `sha256:` prefix if present (be liberal in what you accept).
- 🟡 **Correctness**: Schema-version gate is ordered *after* reading the
  `version` field, which requires parsing under a possibly-unrecognised schema
  **Location**: Phase 2 §1
  Reorder to: verify signature over raw bytes → parse a minimal stable envelope
  containing only `schema_version` → gate schema → then parse the rest and apply
  the version-equality anti-rollback check. Keep `schema_version` in a
  version-stable outer envelope.
- 🟡 **Correctness**: `ACCELERATOR_<SUB>_BIN` env-var name derivation is undefined
  for hyphenated/non-alphanumeric subcommand names
  **Location**: Phase 1 §5
  Git-style subcommands contain hyphens (`frobnicate-thing`), but env var names
  permit only `[A-Za-z0-9_]`. Specify a total normalisation (uppercase, then
  replace every non-`[A-Z0-9_]` char with `_`) applied identically in the
  launcher and `bin/accelerator`, with a hyphenated-name fixture test.
- 🟡 **Correctness + Safety**: Cap-eviction spans multiple cache keys while the
  advisory lock is per-key, racing concurrent fetches/execs
  **Location**: Phase 2 §3 (CacheStore / CacheRootResolver)
  "Oldest-by-mtime eviction under the per-key lock, skipping in-flight entries"
  cannot serialise against another process fetching a *different* key. Specify a
  cache-root-wide lock for the scan+evict, or a non-blocking try-lock per
  candidate so a held entry is provably skipped, and define lock ordering to
  avoid deadlock.
- 🟡 **Safety**: Lock is released before exec, so a concurrent eviction can unlink
  a binary another process is about to exec
  **Location**: Phase 2 §3
  The lock is scoped to fetch/verify/rename and the fd closed before exec, so
  between path resolution and the exec syscall an evicting process can remove the
  file (spurious ENOENT under load). Hold an fd on the resolved binary across the
  resolve→exec handoff (exec via that fd), or touch mtime on resolution and
  document the inode-retention safety argument.
- 🟡 **Safety**: Cache-hit self-heal evicts the working copy before a verified
  replacement exists
  **Location**: Phase 2 §3; Desired End State
  "Evict + re-fetch once" removes the existing (bad) entry first; if the re-fetch
  fails (offline), a previously-working offline invocation now hard-fails,
  contradicting the "pre-existing verified entry left intact" invariant.
  Fetch-and-verify the replacement into a temp file first and unlink the corrupt
  entry only as part of the atomic rename (replace-in-place).
- 🟡 **Security**: Unsigned `cli/verify/` shim is the bootstrap root of trust but
  ships in 0164 with its only integrity guard (byte-identity) deferred to 0165
  **Location**: Phase 4 §1
  Between 0164 landing and 0165's guard, an attacker who can write the shim
  substitutes the entire root of trust. State the shim's trust dependency on the
  plugin package's own distribution integrity as an explicit 0164 assumption, and
  ensure the writable-fallback copy cannot be pre-planted (own the fallback dir
  with restrictive perms; run the shim only from a path the bootstrap controls).
- 🟡 **Security**: Host-suffix redirect allowlist is vulnerable to suffix
  confusion
  **Location**: Phase 2 §3 (Fetcher)
  A naive `ends_with("githubusercontent.com")` matches `evil-githubusercontent.com`
  and `githubusercontent.com.attacker.net`. Match on a dotted-label boundary
  (`host == … || host.ends_with(".githubusercontent.com")`), require https
  post-redirect, and add explicit rejection tests for the confusion cases.
- 🟡 **Architecture**: `CacheRootResolver` XDG fallback conflates the
  `allowed-tools` plugin-root invariant with a general fallback
  **Location**: Phase 2 §3
  See the Security-vs-Portability tradeoff above — resolve whether XDG applies at
  all given the permission-match constraint, rather than importing luminosity's
  fallback unreconciled.
- 🟡 **Architecture**: The built-in/external split point is a load-bearing
  invariant co-owned by 0167/0169 but defended only by a local test
  **Location**: Implementation Approach; Phase 1 §5
  Design the boundary as an extensible registry of built-in commands (so a future
  built-in `vcs guard` is an open-closed extension) and mark the boundary
  provisional pending 0167/0169 rather than a settled invariant.
- 🟡 **Code Quality**: The `launch` module concentrates a large,
  high-complexity orchestration surface
  **Location**: Phase 2 §1, §3
  Name the orchestrator's decision states explicitly (override → cache-hit-verify
  → miss-fetch-verify-cache → evict-refetch-once) and give `launch` a submodule
  layout (orchestrator vs collaborators) so the orchestrator stays a thin,
  readable sequence of guard clauses.
- 🟡 **Test Coverage**: SIGTERM propagation test needs a specified readiness
  handshake, not a sentinel-print race
  **Location**: Phase 1 Success Criteria; Testing Strategy
  Pin the ordering: fixture installs its signal disposition, then writes+flushes
  the sentinel, then blocks; the test reads until the sentinel is observed, only
  then signals, and asserts 128+SIGTERM — with no `sleep` sequencing.
- 🟡 **Test Coverage**: Coverage exclusion may hide the exec code path, not just
  the fixture stub
  **Location**: Phase 1 §6; Phase 2 §3
  Scope the exclusion to the fixture `[[bin]]` source only; keep all
  argv-marshalling and path-selection logic (non-UTF-8 forwarding,
  `ACCELERATOR_<SUB>_BIN` selection) in a covered module with pure unit tests
  that build the argv without calling `exec`.
- 🟡 **Test Coverage**: Concurrency and timeout collaborator tests are described
  in prose but absent from the phase's Automated Verification checklist
  **Location**: Phase 2 Testing Strategy
  Promote CacheStore concurrency (held-lock wait/reuse, crash-reclaim) and
  Fetcher timeout (stalled → named error; slow-but-progressing not aborted;
  404 not retried) into the gating checkboxes, and specify how each is made
  deterministic (controllable byte-stall server; injected lock barrier, not a
  sleep).
- 🟡 **Portability**: Host-target-only build leaves musl-static, DNS, and
  cert-store guarantees unvalidated on three of four release triples until 0165
  **Location**: Overview; Phase 2 Manual Verification
  Add at least one `cargo build --target x86_64-unknown-linux-musl` + `ldd`
  smoke check to 0164 (cheap under a container), or state explicitly that the
  musl runtime guarantees are formally unvalidated until 0165 with a 0165 AC that
  gates on them.
- 🟡 **Portability**: The reqwest `hickory-dns` feature name/composition is
  unverified and may silently not activate the musl DNS bypass
  **Location**: Phase 1 §1
  Pin the exact reqwest version, confirm the feature spelling against that
  version's Cargo.toml, and assert `cargo tree -e features -p launcher` shows the
  hickory resolver crate present (alongside the existing ring-present /
  aws-lc-rs-absent checks).
- 🟡 **Compatibility**: Cross-language platform-alias single-sourcing has a test
  but no single-source mechanism, and the test's completeness is unspecified
  **Location**: Phase 2 §2; Phase 4 §2
  Name the single source (generate Rust/bash tables from `targets.py`, or make
  `targets.py` the fixture the test loads) and require the coherence test to
  assert the full uname-input→alias mapping in every language, not just the four
  canonical aliases.
- 🟡 **Compatibility**: `manifest.json` version binding must be reconciled with
  version-coherence and the all-zeros sentinel contract
  **Location**: Decisions §4; Migration Notes
  Add `manifest.json.version` to `validate_version_coherence`, and decide the
  sentinel's fate for the new schema (carry forward with a named "no binary for
  this version" error, or document its removal) so the anti-rollback equality
  check and the coherence check cannot disagree.

#### Minor

- 🔵 **Correctness**: Retry idempotence requires resetting the temp file per
  attempt — not stated (partial-body-then-retry could concatenate corrupt bytes).
  Add a partial-body-then-success fixture case.
  **Location**: Phase 2 §3 (Fetcher)
- 🔵 **Correctness**: Backoff sleep can race the aggregate deadline; clamp backoff
  to the remaining budget and assert the terminal error class (deadline vs
  attempts-exhausted) deterministically.
  **Location**: Phase 2 §3 (Fetcher)
- 🔵 **Correctness / Compatibility**: Version-equality anti-rollback reference
  point is implicit; state it compares exactly against the launcher's
  `CARGO_PKG_VERSION`, derive the fixture version from that constant, and document
  that a newer manifest is also refused by design.
  **Location**: Decisions §4; Phase 2 §2
- 🔵 **Correctness / Test Coverage**: Control/escape stripping must define its
  character class over decoded Unicode scalars (C0/C1 + ESC/CSI), preserving
  printable/whitespace; test with a multi-byte UTF-8 description and several
  distinct control characters, asserting exact sanitised-string equality.
  **Location**: Phase 3 §1
- 🔵 **Correctness**: Guard the empty-`External`-vector boundary (named error, not
  an index panic) and assert a fetched sub-binary named `version`/`config` cannot
  shadow a built-in.
  **Location**: Phase 1 §4
- 🔵 **Security**: Document the trust boundary of the unauthenticated
  `ACCELERATOR_<SUB>_BIN` / `ACCELERATOR_CACHE_DIR` overrides; ensure
  `ACCELERATOR_CACHE_DIR` never disables content re-verification; log when active.
  **Location**: Phase 1 §5; Phase 2 §3
- 🔵 **Security**: Verify→exec TOCTOU — verify by an open fd and exec that same fd,
  or hold the per-key lock across verify→exec, and note the required cache-root
  ownership/permissions.
  **Location**: Phase 2 §3
- 🔵 **Security / Compatibility**: Define the verify-any-of retirement discipline —
  at most two keys (current + previous), previous dropped on the next release,
  revocation = forced plugin-version bump; assert the set at release.
  **Location**: Decisions §4, §5; Phase 2 §1
- 🔵 **Safety**: Specify idempotent recursive creation of the cache dir (EEXIST =
  success) and cover it in the cache-root branch tests (fresh-install first-use
  path).
  **Location**: Phase 2 §3
- 🔵 **Safety**: Note the atomic-rename-over-a-busy-binary (ETXTBSY) reasoning and
  add a replace-while-busy test or defensive retry.
  **Location**: Phase 2 §3
- 🔵 **Safety**: The bash bootstrap lacks the launcher's flock concurrency control;
  add a bash-3.2-safe per-target lock (mkdir/flock), unique per-process mktemp
  names, verify-before-evict ordering, and a concurrent-bootstrap test.
  **Location**: Phase 4 §2
- 🔵 **Safety**: Make the plugin-root cache disk-growth bound explicit — confirm
  version-scoped old caches are removed with the old plugin install, or apply the
  retained-versions cap to the plugin-root path too.
  **Location**: Phase 2 §3
- 🔵 **Architecture**: Route both the resolution path and the lazy help path
  through a single `Manifest` loader that owns fetch + signature-verify +
  anti-rollback + schema gate, so trusted-manifest access has one implementation.
  **Location**: Phase 2 §3; Phase 3 §1
- 🔵 **Architecture / Code Quality**: Specify the launcher-local error → kernel
  boundary mapping concretely (e.g. a single `kernel::Error::Launcher(#[from] …)`
  carrying the typed launcher error) so category + payload survive to the
  composition root while `version` stays free of network variants.
  **Location**: Phase 1 §3
- 🔵 **Architecture**: Acknowledge the bash↔Rust integrity-contract duplication
  (cache-key format, freshness/anti-rollback rule) as an explicit tradeoff and
  pin the shared semantics both sides honour.
  **Location**: Phase 4 §2; Phase 2 §3
- 🔵 **Code Quality**: Pin the dispatch module name (`launch`) and its internal
  split (`launch::core` pup-constrained vs the imperative shell) up front — the
  plan currently uses both `<dispatch module>` and `launch`.
  **Location**: Phase 1 §3, §4
- 🔵 **Code Quality**: Extract the `ACCELERATOR_<SUB>_BIN` override check into one
  shared pure helper in `launch::core` so the fake and real adapters cannot
  diverge on the escape-hatch semantics.
  **Location**: Phase 1 §5; Phase 2
- 🔵 **Code Quality**: Reword "deny-level" restriction lints to "warn-level,
  promoted to hard errors via `warnings = deny` / `-D warnings`" — the mechanism
  the fixture-bin exclusion works around.
  **Location**: Current State Analysis; Key Discoveries
- 🔵 **Code Quality / Correctness**: Sanitise manifest strings once at the trust
  boundary (Verifier returns already-sanitised descriptions, or a
  constructed-sanitised newtype) so every downstream printer is safe by
  construction.
  **Location**: Phase 3 §1
- 🔵 **Test Coverage**: Make the built-in/external boundary test assert against the
  actual built-in registry the dispatcher consults (plus a negative case that an
  arbitrary name *does* route to External), so adding/removing a built-in without
  updating the guard fails.
  **Location**: Phase 1 §4
- 🔵 **Test Coverage**: Add a bootstrap suite case for a validly-signed-but-stale
  launcher (valid minisig, wrong version/sha256) refused fail-closed, mirroring
  the Rust anti-rollback test.
  **Location**: Phase 4 §2, Success Criteria
- 🔵 **Test Coverage**: When relocating `an_unknown_subcommand_exits_non_zero`,
  preserve its intent — assert an unresolvable unknown subcommand exits non-zero
  with stderr naming the subcommand and the failed resolution step.
  **Location**: Migration Notes; Phase 1 §7
- 🔵 **Portability**: Register `bin/accelerator` in *both* shell-discovery
  mechanisms — `_EXTRA_SHELL_SOURCES` in `sources.py` (shfmt/ShellCheck) *and*
  `lint-bashisms.sh`'s own `git ls-files '*.sh'` discovery (which never matches an
  extensionless file) — and assert coverage in all three tools independently.
  **Location**: Phase 4 §3
- 🔵 **Portability**: State the darwin cache-root precedence explicitly
  (`ACCELERATOR_CACHE_DIR` → `~/Library/Caches` → XDG only if set) and add a
  darwin-specific cache-root test arm.
  **Location**: Phase 2 §3
- 🔵 **Portability**: Drive `bin/accelerator` triple-detection with injected
  `uname -m`/`-s` values covering all four arch combinations (arm64, aarch64,
  x86_64, amd64 × darwin, linux) so the `case` normalisation is validated without
  the hardware.
  **Location**: Phase 4 §2
- 🔵 **Portability**: Confirm the plan states the committed per-triple verify shims
  are a 0165 deliverable (0164 shim coverage is host-arch-only), with a 0165 AC
  that all four shims are built, reproducible, and byte-verified.
  **Location**: Phase 4 §1
- 🔵 **Compatibility**: Re-run `deny:check` after the `resolver = "2"→"3"` bump
  (MSRV-aware selection can pick older deps, perturbing the pinned vergen closure
  and the pruned licence allow-list) and state kernel's `rust-version` explicitly.
  **Location**: Decisions §3; Phase 1 §2
- 🔵 **Compatibility**: Confirm `external_subcommand` + `try_parse`/`DisplayHelp`
  behaviour against the resolved clap version; consider tightening `clap = "4.6"`
  to an exact pin since the help-interception ordering is load-bearing.
  **Location**: Phase 1 §4

### Strengths

- ✅ Faithfully extends the established hexagon: `ResolveBinary` driven port with a
  fake adapter (Phase 1) before the real fetch/verify/cache adapter (Phase 2),
  mirroring the `version` hexagon's port+hand-written-fake pattern (no mocking
  framework), letting dispatch/exec merge without the network stack.
- ✅ Correct dependency direction: a rich launcher-local resolution error maps into
  a small `kernel::Error` at the boundary, keeping `version` from compiling
  against fetch/signature variants.
- ✅ Layering enforcement is not left to convention — a new
  `launch_core_imports_only_permitted` pup rule mirrors the `version::core` rule,
  and dispatch glue is kept out of `version::core`.
- ✅ Correct trust model: minisign as the security boundary and sha256 as
  corruption-detection; manifest signature verified before any field is trusted;
  re-verify before *every* exec including cache hits; a valid-sha256/non-release-key
  signature is refused; control/escape chars stripped from manifest strings.
- ✅ Honestly solves the bootstrap root-of-trust problem — a tampered launcher
  cannot verify itself, so a vendored per-triple shim verifies it against the
  plugin-committed key, fail-closed, with no silent TLS-only downgrade.
- ✅ Resilience is designed, not assumed: layered timeouts (connect + idle-stall +
  aggregate deadline), bounded retry-with-backoff justified by idempotence,
  atomic-rename-or-nothing writes, download-size + free-space caps, per-cache-key
  advisory locking with `FD_CLOEXEC`.
- ✅ Strongly portability-aware: rustls-only enforced at the deny layer across all
  four triples, bundled webpki-roots (no host cert store), hickory-dns for musl,
  ring over aws-lc-rs for cross-build cleanliness, bash-3.2 floor with portable
  sha256 and curl-or-wget fallback.
- ✅ Unusually test-conscious: exactly-one-fetch cache-reuse assertion,
  per-refusal-check naming, mutated-on-disk refusal, offline cache-hit,
  built-ins-work-with-no-manifest, and the fixture `[[bin]]` located via
  `CARGO_BIN_EXE_<name>` with a reasoned coverage/lint exclusion.
- ✅ Phases are genuinely independently mergeable, each leaving `mise run` green,
  and the built-in/external boundary is pinned by a test.

### Recommended Changes

1. **Enforce the fixture→production key handoff mechanically** (addresses: the
   critical fixture-key finding, verify-any-of retirement). Gate the fixture key
   behind `#[cfg(test)]`/a test-only feature so it cannot compile into a release
   build, and add a 0164 release-time assertion that no fixture/test key is in the
   embedded set. Define the two-key rotation discipline (current + previous only).

2. **Elevate the 0165 contract to a shared, tested artifact** (addresses: manifest
   contract pinned by prose, coexistence with `checksums.json`, bare-hex vs
   prefixed sha256, version-coherence + sentinel, cross-language alias
   single-sourcing). Commit a manifest JSON schema + golden fixture + asset-URL
   template that both 0164 and 0165 consume; state coexistence/migration with
   `checksums.json`; reconcile the sha256 wire format (strip-prefix-if-present);
   add `manifest.json.version` to `validate_version_coherence`; and make
   `targets.py` the single source the coherence test derives Rust/bash from,
   asserting the full uname→alias mapping.

3. **Specify the cache locking/eviction/exec concurrency model** (addresses:
   cap-eviction vs per-key lock, lock released before exec, self-heal evicts the
   working copy, mkdir race, ETXTBSY, bootstrap concurrency). Define a
   cache-root-wide or try-lock-per-candidate eviction scheme with lock ordering;
   fetch-and-verify a replacement before unlinking a corrupt entry
   (replace-in-place); hold/exec an fd across resolve→exec; specify idempotent
   `mkdir -p`; and add a bash-3.2-safe bootstrap lock.

4. **Fix the manifest verification ordering and sanitisation** (addresses:
   schema-gate ordering, sanitise-once, two verify paths). Read `schema_version`
   from a stable outer envelope and gate *before* reading `version`; route both
   resolution and help through one `Manifest` loader; sanitise descriptions once
   at the trust boundary over decoded scalars.

5. **Close the two high-confidence correctness gaps** (addresses:
   `ACCELERATOR_<SUB>_BIN` name derivation, empty-`External` guard). Specify a
   total env-var-name normalisation applied identically in Rust and bash with a
   hyphenated-name test; guard the empty-vector boundary with a named error.

6. **Harden the transport and validate musl** (addresses: redirect suffix
   confusion, host-target-only build, hickory-dns feature). Match the redirect
   allowlist on a dotted-label boundary with rejection tests; add a musl
   cross-build + `ldd` smoke check (or an explicit 0165 gating AC); pin the
   reqwest version and assert the hickory resolver is present in the feature tree.

7. **Promote the prose-only tests into gating criteria and sharpen the fragile
   ones** (addresses: concurrency/timeout tests, SIGTERM handshake, coverage
   scope, boundary-test tautology, escape-char class, freshness-replay,
   unknown-subcommand intent). Move the collaborator concurrency/timeout tests
   into Phase 2's checklist; pin the SIGTERM readiness handshake; scope the
   coverage exclusion to the fixture source; assert the built-in registry;
   broaden the sanitisation and add the stale-launcher and unknown-subcommand
   cases.

8. **Resolve the XDG-vs-`allowed-tools` tradeoff and the built-in/external
   registry** (addresses: CacheRootResolver conflation, provisional boundary).
   Decide explicitly whether XDG fallback is permissible under the permission
   invariant, and model built-ins as an extensible registry marked provisional
   pending 0167/0169.

9. **Tidy the plan's descriptive accuracy and naming** (addresses: module-name
   placeholder, "deny-level" wording, override-helper duplication, error-boundary
   mapping, bash↔Rust duplication tradeoff, resolver/clap confirmations). These
   are low-effort edits that keep the next implementer oriented.

## Per-Lens Results

### Architecture

**Summary**: Architecturally strong — faithful hexagon extension (driven
`ResolveBinary` port with fake-then-real adapters), launcher-local error mapped
to a slim `kernel::Error`, dispatch glue moved out of `version::core` into a
dedicated `launch` module with its own pup rule, and explicit functional-core /
imperative-shell separation. Well-sequenced four-phase decomposition with
designed resilience. Main risks: unowned contract drift with 0165/0167/0169
across three seams, and under-specified `launch` internal structure.

**Strengths**:
- Faithful hexagon extension; fake-then-real `ResolveBinary` lets dispatch/exec
  merge without the network stack.
- Correct dependency direction; rich launcher-local error → 1-2 `kernel::Error`
  variants keeps light subcommands clean.
- Layering enforcement via a new pup rule, not convention.
- Resilience is architecturally explicit (layered timeouts, bounded retry,
  atomic-rename-or-nothing, self-healing, fail-closed re-verify).
- Clean core/shell separation with inward-only dependency direction.
- Coherent trust boundary reasoning (minisign vs sha256; the vendored shim).

**Findings**:
- 🟡 (high) Three cross-story contract seams lack a single owning artifact and can
  drift silently (manifest schema/asset template, alias map, pubkey) — Phase 2 §2,
  Phase 4 §4, Testing Strategy.
- 🟡 (medium) `CacheRootResolver` conflates the `allowed-tools` plugin-root
  invariant with a general XDG fallback — Phase 2 §3.
- 🟡 (high) The built-in/external split point is a load-bearing invariant co-owned
  by three stories with only a local test to defend it — Implementation Approach,
  Phase 1 §5.
- 🔵 (medium) Manifest is fetched/verified by both the resolution path and the
  help path with no shared owner — Phase 2 §3, Phase 3 §1.
- 🔵 (medium) Cache root, atomic-write, and freshness logic are implemented once in
  Rust and re-expressed in the bootstrap — Phase 4 §2, Phase 2 §3.
- 🔵 (medium) Boundary mapping into one or two generic `kernel::Error` variants
  risks lossy diagnostics or a leaky abstraction — Phase 1 §3.

### Code Quality

**Summary**: Exceptionally thorough and convention-respecting (const-fn services,
hand-written fakes, launcher-local error, no unwrap/expect against the deny-level
restriction lints, fake-then-real adapter split). Main maintainability concern is
that Phase 2 concentrates a large behaviour surface into one `launch` module with
four collaborators, and several cross-cutting concerns are single-sourced by
convention rather than an enforced mechanism.

**Strengths**:
- Reuses the hexagon pattern; launcher-local error mapped into a small
  `kernel::Error` (good SRP + dependency direction).
- Honours the restriction lints by routing IO/HTTP/verify through `Result` and
  treating crypto-provider install as a fallible mapped call.
- Decomposes resolution into four separately-testable collaborators.
- Fake-then-real sequencing keeps phases independently mergeable.
- Error variants carry the payload their diagnostics need.
- Built-in/external boundary pinned by a test.

**Findings**:
- 🟡 (medium) The `launch` module concentrates a large, high-complexity
  orchestration surface — Phase 2 §1, §3.
- 🔵 (high) The dispatch/launch module name is left as an unresolved placeholder —
  Phase 1 §3, §4.
- 🔵 (medium) Triple→alias map and cache-root resolution are single-sourced by
  convention, not an enforced seam — Phase 2 §2, Phase 4 §2, Testing Strategy.
- 🔵 (medium) The env-override short-circuit is duplicated across fake and real
  adapters — Phase 1 §5, Phase 2.
- 🔵 (low) Plan describes restriction lints as "deny-level"; config sets them to
  "warn" promoted under `warnings = deny` — Current State Analysis, Key
  Discoveries.
- 🔵 (medium) Control-character stripping of manifest strings is an
  easily-forgotten cross-cutting rule — Phase 3 §1.

### Test Coverage

**Summary**: Unusually test-conscious — exactly-one-fetch, SIGTERM
readiness-handshake, per-refusal-check naming, self-healing, cache-root branches,
mirroring the black-box `CARGO_BIN_EXE` idiom and hand-written-fake pattern.
Coverage is broad and proportional to risk. Gaps are at the seams: an
under-specified handshake, a coverage exclusion that could hide the exec path,
concurrency/timeout tests only in prose, and a couple of assertions whose strength
is left implicit.

**Strengths**:
- The exactly-one-fetch assertion is strong behaviour-not-implementation testing.
- Refusal paths enumerated per-check with the naming assertion.
- Reuses the established test architecture (black-box spawns, hand-written fakes,
  isolated collaborators).
- Negative-space assertions present and specific.
- Manifest-independence of built-ins pinned by a dedicated test.
- The fixture bin's exclusion is reasoned and bounded.

**Findings**:
- 🟡 (medium) SIGTERM propagation test needs a specified readiness handshake, not a
  sentinel-print race — Phase 1 Success Criteria, Testing Strategy.
- 🟡 (medium) Coverage exclusion may hide the exec code path, not just the fixture
  stub — Phase 1 §6, Phase 2 §3.
- 🟡 (medium) Concurrency and timeout collaborator tests are described in prose but
  absent from success criteria — Phase 2 Testing Strategy.
- 🔵 (medium) Built-in/external boundary test may be a tautology unless it asserts
  the enumeration source — Phase 1 §4, Implementation Approach.
- 🔵 (medium) Cross-language triple→alias coherence test lacks a defined oracle and
  drift-direction guarantee — Phase 2 §2, Testing Strategy.
- 🔵 (medium) Escape-stripping test asserts absence/presence but not the specific
  dangerous sequences — Phase 3 §1.
- 🔵 (medium) No test named for the launcher-freshness / replay-of-older-signed-
  launcher path — Phase 4 Success Criteria, §2.
- 🔵 (low) Regression intent of the changed unknown-subcommand test should be
  preserved, not just relocated — Migration Notes, Phase 1 §7.

### Correctness

**Summary**: Unusually rigorous — verification ordering, atomic-rename-or-nothing,
per-key locking, self-heal, lazy help path mostly sound. Strongest concerns: a
genuine ordering inconsistency between the schema gate and reading `version`, the
`ACCELERATOR_<SUB>_BIN` name derivation for hyphenated names, and cap-eviction
spanning multiple keys under a per-key lock.

**Strengths**:
- Atomic-rename-or-nothing correctly satisfies the no-partial-entry invariant.
- Re-verify-before-every-exec correctly treats the cache as untrusted.
- Lazy help path correctly decouples offline built-ins from manifest availability.
- minisign-boundary / sha256-corruption ordering correct; non-release-key refusal
  tested.
- verify-any-of gives a correct rotation overlap.
- `FD_CLOEXEC`/closing the lock fd before exec is correct.

**Findings**:
- 🔴→ (high, reported as major) Schema-gate ordering: version-equality reads a field
  parsed under a possibly-unrecognised schema — Phase 2 §1.
- 🟡 (high) `ACCELERATOR_<SUB>_BIN` name derivation undefined for
  hyphenated/non-alphanumeric subcommand names — Phase 1 §5.
- 🟡 (medium) Cap-eviction spans multiple cache keys but the advisory lock is
  per-cache-key — Phase 2 §3.
- 🔵 (medium) Retry idempotence requires resetting the temp file per attempt; not
  stated — Phase 2 §3.
- 🔵 (medium) Backoff sleep can race the aggregate deadline; interaction
  unspecified — Phase 2 §3.
- 🔵 (medium) Version-equality anti-rollback reference point and fixture version
  coupling underspecified — Decisions §4, Phase 2 §2.
- 🔵 (low) Cache-hit self-heal "evict then re-fetch once" can destroy the only copy
  when offline — Phase 2 §3.
- 🔵 (medium) Control/escape stripping must define its character class precisely —
  Phase 3 §1.
- 🔵 (low) `External(Vec<OsString>)` with an empty vector / built-in name collision
  path unspecified — Phase 1 §4.

### Security

**Summary**: Exceptionally security-conscious and internalises the correct threat
model. Dominant residual risks are all in the root-of-trust seams 0164 owns but
defers guarding: the unsigned shim, the fixture keypair in a verify-any-of set,
and the two unauthenticated exec escape hatches. The suffix-based redirect
allowlist and the verify→exec TOCTOU also warrant tightening.

**Strengths**:
- minisign as the boundary, sha256 as corruption-only; poisoned-but-well-formed
  cache entry refused.
- Re-verify before every exec including cache hits (defence-in-depth, tested).
- Manifest signature verified before any field; anti-rollback + schema gate close
  substitution/downgrade.
- Bootstrap root-of-trust solved honestly via the vendored shim, fail-closed.
- Control/escape stripping closes terminal-injection from signed-but-influenced
  manifest content.
- Atomic-rename-or-nothing + size caps + per-key locking guard poisoning/races.
- https pinned, redirect allowlist, no curl `-k`, native-tls/openssl banned,
  bundled roots.
- Shim invoked by absolute path (PATH-decoy test); `"$@"` forwarded as argv.

**Findings**:
- 🔴 (high, reported as major) Fixture keypair embedded in verify-any-of set risks
  shipping to production as a valid signing root — Decisions §5, Phase 2 §1,
  Migration Notes. **[Curated to Critical in the aggregate summary.]**
- 🟡 (medium) Unsigned shim is the bootstrap root of trust but ships in 0164 with
  its only integrity guard deferred to 0165 — Phase 4 §1.
- 🟡 (medium) Host-suffix redirect allowlist is vulnerable to suffix-confusion and
  open-redirect abuse — Phase 2 §3.
- 🔵 (high) Unauthenticated exec/cache-location overrides are documented but their
  trust implications are not scoped — Phase 1 §5, Phase 2 §3.
- 🔵 (medium) TOCTOU window between final verification and exec of the cached
  binary — Phase 2 §3.
- 🔵 (medium) verify-any-of key set widens the trust anchor with no shrink/expiry
  discipline — Phase 2 §1, Decisions §4.
- 🔵 (low) Bootstrap anti-replay of an old signed launcher relies on manifest
  version-equality only — Phase 4 §2.

### Safety

**Summary**: Exceptionally strong — atomic-rename-or-nothing, pre-exec re-verify,
fail-closed, advisory locking, size/free-space caps, timeouts, bounded retry all
explicit and gated. Residual concerns are edge cases around the self-heal eviction
path and the eviction-vs-exec window; contained (recovery is a re-fetch) but worth
pinning.

**Strengths**:
- Atomic-rename-or-nothing precisely specified (temp inside cache dir, intra-fs
  rename, verify-then-rename).
- Fail-closed pervasive and explicit (named errors, no silent TLS-only downgrade).
- Re-verify-before-every-exec gated by a dedicated tamper test.
- Runaway-resource protections present (size cap, free-space, layered timeouts,
  bounded retry, https redirect allowlist).
- Concurrency considered (per-key lock, `FD_CLOEXEC`, crash-reclaim test).
- Anti-rollback + schema gate prevent stale/future manifest trust.

**Findings**:
- 🟡 (medium) Cache-hit self-heal evicts the working copy before a clean
  replacement is fetched/verified — Phase 2 §3, Desired End State.
- 🟡 (medium) Advisory lock released before exec; concurrent eviction can unlink a
  binary a process is about to exec — Phase 2 §3-4.
- 🔵 (high) Cache directory creation (mkdir -p) and racing-mkdir idempotence not
  specified — Phase 2 §3, Decisions §2.
- 🔵 (medium) Atomic rename over a busy binary (ETXTBSY) not addressed — Phase 2 §3.
- 🔵 (medium) Bash bootstrap re-implements fetch/verify/cache/self-heal without the
  launcher's flock concurrency control — Phase 4 §2.
- 🔵 (low) Plugin-root cache disk-growth bound across plugin versions unclear —
  Phase 2 §3.

### Portability

**Summary**: Unusually portability-conscious — four-Unix-target model, rustls-only,
bundled webpki-roots, hickory-dns for musl, ring over aws-lc-rs, bash-3.2 floor
with portable sha256 and curl-or-wget fallback, noexec/read-only writable+exec
probe with XDG/darwin fallback. Chief residual risk is structural: 0164 validates
only the host target, so both musl triples go unexercised until 0165.

**Strengths**:
- rustls-only made the load-bearing constraint and defended in the dependency
  graph via deny across all four triples.
- Bundled webpki-roots; never reads the host cert store.
- hickory-dns bypasses musl getaddrinfo/nsswitch.
- ring selected explicitly for cross-build cleanliness.
- bash-3.2 floor, portable sha256, curl-or-wget fallback.
- noexec/read-only fallback with a writable+exec probe; shim by absolute path.
- Single-sourced alias map with a coherence test.
- otool -L / ldd verification split per-OS.

**Findings**:
- 🟡 (high) Host-target-only build leaves musl-static, DNS, and cert-store
  portability unvalidated on three of four triples until 0165 — Overview, Phase 2
  Manual Verification.
- 🟡 (medium) reqwest `hickory-dns` feature name likely incorrect for 0.12 — the
  musl DNS bypass may silently not activate — Phase 1 §1.
- 🔵 (high) `bin/accelerator` must be registered in two independent shell-discovery
  mechanisms, not one — Phase 4 §3.
- 🔵 (medium) Darwin XDG fallback ordering assumes `XDG_CACHE_HOME` precedence over
  `~/Library/Caches` — Phase 2 §3.
- 🔵 (medium) uname-based host-triple detection not exercised across the four
  arches in 0164 — Phase 4 §2.
- 🔵 (low) Vendored per-triple verify shim reintroduces a four-target build surface
  0164's host-only scope does not cover — Phase 4 §1.

### Compatibility

**Summary**: Rigorous on the launcher's internal contracts and correctly frames
0165 as a producer/consumer coupling pinned via fixtures. Dominant risk is that
0164 introduces a second, divergent manifest schema alongside `checksums.json`
without specifying coexistence, sha256-format reconciliation, the sentinel's fate,
or version-coherence — and the fixture-vs-production schema is not pinned in a
shared artifact, so 0164 and 0165 can silently drift.

**Strengths**:
- Treats 0165 as producer/consumer, pinned via fixtures; verify-any-of rotation
  overlap for the key handoff.
- Spec-grade verification ordering.
- rustls-only as a hard dependency-compatibility constraint with regression tests.
- schema_version forward-compat with the correct asymmetry.
- Cross-language coherence test proposed for the alias map.

**Findings**:
- 🟡 (high) New `manifest.json` is a second, divergent schema alongside
  `checksums.json` with no stated coexistence or migration — Decisions §4,
  Phase 2 §2.
- 🟡 (high) Bare-hex sha256 in `manifest.json` contradicts the established
  `sha256:`-prefixed on-wire format — Phase 2 §2.
- 🟡 (medium) Fixture-vs-production manifest/asset schema is not pinned in a shared
  contract artifact — Decisions §4, What We're NOT Doing.
- 🟡 (medium) Cross-language platform-alias single-sourcing has a test but no
  single-source mechanism — Phase 2 §2, Phase 4 §2.
- 🟡 (medium) `manifest.json` version binding must be reconciled with
  version-coherence and the sentinel contract — Decisions §4, Migration Notes.
- 🔵 (medium) resolver 2→3 bump and `rust-version = 1.90.0` change dependency
  selection workspace-wide — Decisions §3, Phase 1 §2.
- 🔵 (medium) clap `external_subcommand` behaviour depends on the floating `4.6`
  constraint, not a firm pin — Phase 1 §4, Assumptions.
- 🔵 (medium) Fixture→production key rotation overlap needs a defined verify-any-of
  retirement path — Decisions §5, Phase 2 §1, Migration Notes.

---

## Re-Review (Pass 2) — 2026-07-04

**Verdict:** REVISE

The revision resolved the critical finding and the large majority of pass-1
majors. All eight lenses re-ran against the edited plan. No critical findings
remain; the plan is now structurally sound. The residual is a **bounded cluster**
dominated by (a) bringing the bash bootstrap up to the parity the Rust launcher
and the repo's *existing* hardened shell precedent already set, (b) a handful of
precision defects introduced by the pass-1 edits, and (c) coordinating the
checksums.json→manifest.json cutover plus a real pre-existing version drift. This
is a focused second pass, not a rework.

### Previously Identified Issues

- 🔴→✅ **Security/Compatibility**: Fixture key ships to production — **Resolved**
  (test-only cargo feature + release-time no-fixture-key assertion + two-key
  rotation; security lens now lists it as a strength).
- 🟡→✅ **Architecture/Compatibility**: 0165 contract pinned only by prose —
  **Resolved** (Decisions §6 shared, tested contract artifact).
- 🟡→✅ **Compatibility**: bare-hex vs `sha256:`-prefix — **Resolved**
  (strip-if-present tolerance).
- 🟡→✅ **Correctness**: schema-gate ordering — **Resolved** (raw-bytes signature
  → minimal envelope → gate → version-equality).
- 🟡→✅ **Correctness/Safety**: cap-eviction spans keys / eviction-vs-exec race —
  **Resolved** (XDG dropped → cap machinery removed entirely; version-scoped
  bound).
- 🟡→✅ **Safety**: self-heal evicts working copy — **Resolved** (replace-in-place;
  verified successor before unlink).
- 🟡→✅ **Security**: unsigned shim guard deferred / redirect suffix confusion /
  verify→exec TOCTOU — **Resolved** (distribution-integrity assumption +
  controlled-path; dotted-label allowlist; lock held across verify→exec).
- 🟡→✅ **Architecture**: CacheRootResolver XDG conflation — **Resolved** (XDG
  dropped, plugin-root-or-named-error).
- 🟡→✅ **Test Coverage**: SIGTERM handshake / coverage exclusion scope /
  concurrency+timeout tests as prose — **Resolved** (deterministic handshake;
  exclusion scoped to the fixture source; collaborator tests promoted to gating).
- 🟡→✅ **Correctness**: `ACCELERATOR_<SUB>_BIN` name derivation — **Resolved** (total
  normalisation defined) — but see the new collision sub-issue below.
- 🟡→◐ **Compatibility/Architecture**: manifest.json vs checksums.json coexistence —
  **Partially resolved** (supersede decided; the *cutover ownership and overlap
  window* are now the open edge).
- 🟡→◐ **Portability**: host-target-only musl validation — **Partially resolved**
  (deferred to a 0165 gating AC, but the AC lives only as prose in this plan, not
  yet written into the 0165 work item).
- 🟡→◐ **Portability**: hickory-dns feature activation — **Partially resolved**
  (feature-tree assertion added, but the dependency snippet still shows the caret
  `0.12`, not the exact pin the prose mandates).
- 🟡→◐ **Compatibility**: manifest version binding vs coherence/sentinel —
  **Partially resolved** (coherence membership stated; the cli-workspace version
  drift and the exact sentinel byte-form are newly surfaced — see below).
- 🟡→◐ **Code Quality/Architecture**: `launch` concentrates complexity —
  **Partially resolved** (state machine + core/shell split; CacheStore now flagged
  as concentrating invariants).
- 🔵→✅ Most minors (deny-level wording, error boundary mapping, sanitise-once
  newtype, empty-`External` guard, registry boundary test, resolver deny re-run,
  MSRV on all members, two shell-discovery mechanisms) — **Resolved**.

### New Issues Introduced

- 🟡 **Safety/Security/Correctness**: **The bash bootstrap is under-specified vs the
  repo's existing hardened precedent.** The Rust `Fetcher` has connect/stall/
  aggregate timeouts and the existing `launcher-helpers.sh` `download_to` already
  carries `--proto '=https' --tlsv1.2 --max-redirs --max-filesize`; the bootstrap
  fetch (Phase 4 §2) restates only "cert-verified, never `-k`". And the
  `mkdir`-based advisory lock has no stale-lock recovery, though `launch-server.sh`
  already uses a `trap … EXIT` + stale hint. Risks a non-https redirect, an
  unbounded hang, or a first-use deadlock. **[Dominant new theme.]**
- 🟡 **Compatibility**: **Exact-equality anti-rollback collides with an existing
  version drift** — `cli/Cargo.toml` is `1.24.0-pre.2` while
  `plugin.json`/`checksums.json` are `1.24.0-pre.7`. If the launcher's
  `CARGO_PKG_VERSION` and the emitted `manifest.version` don't share one coherent
  source, the check refuses *every* production manifest while the fixture (which
  derives version from `CARGO_PKG_VERSION`) passes — the fixture-passes/production-
  fails trap. The cli workspace must join the coherence set.
- 🔴/🟡 **Correctness**: **FD_CLOEXEC contradicts "lock held across verify→exec."**
  `FD_CLOEXEC` releases the lock *at* `execve`, the exact verify→use boundary. The
  real load-time guarantee is rename-by-inode (the kernel holds the verified inode
  open across exec); the wording should attribute safety there and make
  rename-by-inode a tested invariant.
- 🟡 **Test Coverage**: **The `targets.py` oracle lacks the uname spellings.**
  `targets.py` holds only Rust-triple→alias pairs, not the `uname -m`/`-s`
  spellings the coherence test asserts against — so the "full uname-mapping"
  assertion has no real oracle to load. Needs a canonical uname table (or a
  reworded source).
- 🟡 **Compatibility/Architecture**: **checksums.json→manifest.json cutover has
  split ownership** — which story edits `validate_version_coherence`, and when, is
  ambiguous; keep checksums.json in the coherence set until 0165 physically
  retires it, with a both-agree test during the overlap.
- 🔵 **Correctness**: `ACCELERATOR_<SUB>_BIN` normalisation is lossy —
  `frobnicate-thing` and `frobnicate_thing` collide onto one variable; document as
  bounded by curated names or reject colliding names.
- 🔵 **Architecture/Code Quality**: the Rust↔bash duplication of cache-root /
  self-heal / lock logic is real (they can't share code) and still lacks an
  explicit shared-behavioural-contract acknowledgement.
- 🔵 Assorted: pull the key-coherence cross-check forward into 0164; specify temp-
  file perms (0600, exec bit only post-verify); pin the sentinel's exact byte-form
  in the shared contract; pin clap `=4.6.x` unconditionally; mark the verify crate's
  pup/deny exclusion explicitly (survives 0167 activating the ban-lists); add a
  target module layout for `launch::`; consider dropping the unused `args` param
  from `ResolveBinary::resolve`; exclude built-in names from the synthesised help
  listing.

### Assessment

The plan is in good shape and no longer has any critical or structural defects —
pass-1's critical and every high-impact major are resolved. What remains is a
tight, well-defined cluster: **harden the bash bootstrap to the parity the Rust
launcher and the existing `launch-server.sh`/`launcher-helpers.sh` already set**
(timeouts, curl `--proto`/TLS/redirect/size flags, stale-lock recovery), **fix
four precision defects the pass-1 edits introduced** (FD_CLOEXEC wording →
rename-by-inode, the `targets.py` uname oracle, the caret-vs-exact reqwest pin,
the normalisation collision), and **coordinate the checksums→manifest cutover and
the real cli-workspace version drift**. These are all addressable in a focused
second revision pass rather than a rework; the plan is close to ready.

---

## Approval — 2026-07-04

**Verdict:** APPROVE

Approved after the pass-2 findings were addressed by a subsequent targeted edit
pass (no new lens run):

- **Bootstrap parity** — `bin/accelerator` now carries forward the existing
  `launcher-helpers.sh` `download_to` hardening (`--proto '=https' --tlsv1.2
  --max-redirs --max-filesize --connect-timeout --max-time`) and the
  `launch-server.sh` stale-lock recovery (`trap` release + bounded acquisition
  timeout + PID-owner reclaim), with tests for a stalled fetch, a non-https
  redirect, and an orphaned-lock reacquire.
- **Precision defects fixed** — FD_CLOEXEC wording now attributes the load-time
  guarantee to rename-by-inode; the canonical `uname`-input→alias table is added
  to `targets.py` as the coherence-test oracle; reqwest/rustls/clap are
  exact-pinned with a crate-name-keyed hickory assertion; the override
  normalisation rejects colliding/non-identifier names.
- **Version coherence + cutover** — the cli workspace joins
  `validate_version_coherence` (resolving the `pre.2`/`pre.7` drift) with the
  anti-rollback asserted against the real workspace version; the
  checksums.json→manifest.json cutover is a monotonic, tested migration; the
  in-repo key-coherence cross-check is pulled into 0164.

Remaining open items are low-stakes 🔵 minors accepted for handling at
implementation: the explicit Rust↔bash duplication tradeoff note, marking the
`cli/verify/` pup/deny exclusion, a `launch::` module layout, dropping the unused
`args` param from `ResolveBinary::resolve`, excluding built-in names from the
synthesised help listing, and dedicated tests for lock-non-leak-across-exec and
the sanitisation newtype. The plan is **ready for implementation**.

---
*Review generated by /accelerator:review-plan*
