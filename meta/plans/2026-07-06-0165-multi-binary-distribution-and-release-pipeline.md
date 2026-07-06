---
type: plan
id: "2026-07-06-0165-multi-binary-distribution-and-release-pipeline"
title: "Multi-Binary Static Distribution and Release Pipeline with minisign Implementation Plan"
date: "2026-07-06T00:32:26+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0165"
parent: "work-item:0165"
derived_from: ["codebase-research:2026-07-06-0165-multi-binary-distribution-release-pipeline"]
relates_to: ["work-item:0164", "work-item:0168"]
tags: [rust, distribution, release, cross-compile, minisign]
revision: "d8e9c6eb30fa112f43efaa9c288a6245f76e7613"
repository: accelerator
last_updated: "2026-07-06T11:01:16+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Multi-Binary Static Distribution and Release Pipeline with minisign Implementation Plan

## Overview

Build the **producer half** of on-demand static-binary distribution: extend the
hand-rolled invoke release pipeline so it cross-compiles the `accelerator`
launcher (plus every dispatched `accelerator-<sub>` sub-binary — none exist at
HEAD), signs each binary and a `manifest.json` with minisign, retires the flat
`checksums.json` as the launcher's integrity artifact in favour of the manifest,
vendors the per-platform verify shims the bootstrap needs, and enforces
version coherence across `plugin.json`, every crate's effective version, and
`manifest.version`.

The **consumer half is frozen by 0164** and must not change: the launcher
(`cli/launcher/src/launch/outbound/resolve/`) and the bootstrap
(`bin/accelerator`) already fetch, sha256-verify, and minisign-verify against a
build-embedded public key. This plan makes the pipeline emit artifacts those
consumers accept.

## Current State Analysis

Two independent fetch/verify paths already exist on the consumer side, and the
producer must feed both:

| Consumer | Fetches | Verifies with | Integrity source |
|---|---|---|---|
| Bootstrap `bin/accelerator:161-175` | `accelerator-{platform}` + `.minisig` | vendored shim + committed key; whole-file minisig, **no sha256, no manifest** | detached `.minisig` |
| Launcher `resolve/mod.rs:137-172` | `manifest.json` + `manifest.minisig`, then `{name}-{platform}` | embedded key: manifest signature first, then per-binary sha256 + inline signature | `manifest.json` |

The producer today (`tasks/build.py`, `tasks/github.py`, `tasks/version.py`,
`tasks/release.py`) does none of this for the launcher:

- `build.server_cross_compile` (`build.py:201-216`) cross-compiles a **single
  hardcoded** `accelerator-visualiser` from
  `skills/visualisation/visualise/server/`; the cli workspace launcher is never
  built.
- `build.create_checksums` (`build.py:228-239`) emits the flat
  `checksums.json` (`{version, binaries: {platform: "sha256:hex"}}`). There is
  **zero minisign code** anywhere in `tasks/` or the workflows.
- `validate_version_coherence` (`build.py:131-151`) spans five sources
  including `checksums.json` (`build.py:58-60`) but not `manifest.version`.
- `github.upload_and_verify` (`github.py:136-178`) uploads the four visualiser
  binaries + debug archives, re-downloads and sha256-verifies each binary
  against `checksums.json`, publishes on success, and **preserves the draft** on
  `AssetVerificationError` (`github.py:165-171`).
- `keys/accelerator-release.pub` is committed and embedded by
  `cli/launcher/build.rs:28-45`; the matching secret does not exist.

### Key Discoveries

- **The manifest `binaries` map is legitimately empty at HEAD.** The only cli
  workspace binaries are `accelerator` (bootstrap-fetched via detached minisig,
  never a manifest entry) and `accelerator-verify` (the vendored trust shim). No
  *externally-dispatched* sub-binary exists; the schema explicitly allows
  `binaries: {}` (`manifest.schema.json:20`). The first real sub-binary (the
  visualiser) arrives in 0168.
- **`minisign 0.12` (mise-pinned, `mise.toml:32`) prehashes by default and works
  non-interactively.** Empirically verified: `minisign -G -W` keygen and plain
  `minisign -S` (no `-H`) both run against `/dev/null` stdin (no TTY), and the
  emitted signature's trusted comment ends in `hashed` (algorithm bytes `ED`).
  This is exactly what the launcher's `allow_legacy = false`
  (`keys.rs:68`) requires — so "sign whole-file, not `-H`" means *don't pass the
  flag*, not "produce a legacy signature". `cli/verify/tests/verify.rs:44-58`
  already round-trips this invocation through the shim.
- **The producer must sign the exact bytes it uploads.** The launcher verifies
  the raw `manifest.json` bytes against `manifest.minisig` *before parsing*
  (`mod.rs:129-130`), so any re-serialisation after signing breaks the
  signature. Serialise once to disk, sign that file, upload the same file.
- **Release binaries are gitignored; the bootstrap's shim must be committed.**
  `.gitignore:22` ignores `skills/visualisation/visualise/bin/accelerator-visualiser-*`
  (uploaded, never committed), while `checksums.json`, `bin/accelerator`, and
  `keys/*.pub` are tracked. The bootstrap reads the vendored shim from
  `${CLAUDE_PLUGIN_ROOT}/bin/accelerator-verify-${platform}`
  (`bin/accelerator:64-66`) — so those shims ship *inside the plugin package*
  and must be tracked/committed, a different lifecycle from the uploaded
  binaries. The shim takes the public key as an argument (`verify.rs`,
  `bin/accelerator:153`), so it is key-agnostic: vendor once, refresh on demand,
  never in the release hot path (cross-compiled binaries are not byte-reproducible,
  and `git.commit_version` runs `git add .` at `git.py:73`).
- **`checksums.json` is not a release asset today.** `upload_and_verify` uploads
  binaries + archives, reading `checksums.json` only as a local hash source
  (`github.py:140-144`). "No `checksums.json` asset" is already true.

## Desired End State

A push to `main` produces a GitHub Release whose assets are:

- `accelerator-{platform}` (× 4) + `accelerator-{platform}.minisig` (× 4) — the
  launcher, fetched and shim-verified by the bootstrap.
- `manifest.json` + `manifest.minisig` — `schema_version: 1`,
  `version == plugin.json version`, `binaries: {}` at HEAD (populated by 0168).
- `{name}-{platform}` + `.minisig` for each manifest `binaries` entry (none at
  HEAD).
- The existing visualiser binaries + `*.debug.tar.gz` — **untouched** (this plan
  is additive; 0168 folds the visualiser into the manifest and removes
  `checksums.json`).

A launcher built from that HEAD (embedding the real committed key) bootstraps,
fetches `manifest.json`, verifies its signature, and accepts the version. Every
binary the manifest lists verifies sha256 + minisign end-to-end. Version
coherence fails the build on any `plugin.json` / crate / `manifest.version`
disagreement. A corrupted asset or swapped `.minisig` leaves the release in
draft. `RELEASING.md` documents the key lifecycle and no longer advertises the
non-existent runtime provenance hook.

Verify with `mise run test:unit:tasks`, `mise run test:unit:cli`, `mise run
check`, and the manual release dry-run in Testing Strategy.

## What We're NOT Doing

- **Not touching the frozen consumer contract**: `resolve/{manifest,verifier,
  keys,fetcher,mod}.rs`, `help.rs`, the manifest fixtures, `cli/verify`, or
  `bin/accelerator`. Read-only references only.
- **Not designing the manifest schema** — it is frozen (`schema_version: 1`); we
  conform to `manifest.schema.json`.
- **Not cross-compiling or re-homing the visualiser** — 0168 folds it into the
  workspace and into the manifest. Its `server_cross_compile`, `checksums.json`,
  and `launch-server.sh` stay exactly as they are.
- **Not removing the physical `checksums.json`** — 0168 owns that. This plan
  only stops the *launcher track* from depending on it.
- **Not generating or committing the real production keypair** — this plan
  delivers the tooling + runbook; a repo admin generates the `-W` key at
  rollout, commits its public half, and provisions the secret as a GHA secret.
  Tests sign with ephemeral keys.
- **Not signing or manifest-listing debug archives** — they remain unsigned
  convenience symbolication assets (never trusted-executed).
- **Not adding in-process SLSA** — CI-side `actions/attest-build-provenance@v2`
  stays as out-of-band provenance (ADR-0046).
- **Not running the foreign-arch binaries on their target platform** — all four
  targets are cross-compiled on the single `macos-latest` runner, so CI re-verify
  executes only the host-arch (darwin-arm64) shim; the darwin-x64 / linux-arm64 /
  linux-x64 shims and launchers are gated by magic-byte + static assertions + the
  drift guard, not by an on-target run. Per-platform runtime validation is a
  conscious deferral to consumer-side testing (a future qemu/Rosetta smoke could
  close it). This is the highest-consequence residual coverage gap and is called
  out so it is a decision, not an oversight.

## Implementation Approach

Six independently mergeable phases, TDD throughout, strictly **additive** so the
visualiser keeps releasing at every commit. Phases 1–4 add new tasks and modules
wired into nothing live; Phase 5 wires the new track into the release workflow
(gated on the admin provisioning the secret); Phase 6 is a standalone docs
correction that can merge at any time. The launcher→sub-binary path is exercised
against a fixture crate because no real sub-binary exists until 0168; the
bootstrap→launcher path is exercisable end-to-end at HEAD once the shims are
vendored.

---

## Phase 1: Signing primitives + key-generation tooling + key lifecycle runbook

### Overview

Introduce a minisign signing helper and a `-W` key-generation task, plus the
`RELEASING.md` key lifecycle procedure. Wired into no release step yet.

### Changes Required

#### 1. Signing helper

**File**: `tasks/signing.py` (new)
**Changes**: `sign_file(secret_key, target, signature)` shells `minisign -S -s
<secret> -x <signature> -m <target>` (default prehash, no `-H`). The signature
output path is an **explicit argument**, not derived from `target` — binaries
sign to `<name>-<platform>.minisig` and the manifest signs to `manifest.minisig`
(not the `manifest.json.minisig` a suffix-append would produce, which the
launcher would 404 on — it fetches the hardcoded `manifest.minisig`,
`resolve/mod.rs:123`). It runs with `check=False` and, on a non-zero exit,
raises a new `SigningError` (added to `tasks/shared/errors.py`, following the
`InvalidVersionError` precedent, since it is imported across signing/release)
whose message includes `result.stderr.strip()`, so a signing failure surfaces
minisign's real diagnostic and aborts the `*_sign` task cleanly rather than
raising an opaque `CalledProcessError`.

```python
def sign_file(secret_key: Path, target: Path, signature: Path) -> Path:
    result = subprocess.run(
        ["minisign", "-S", "-s", str(secret_key),
         "-x", str(signature), "-m", str(target)],
        check=False, capture_output=True, text=True, timeout=120,
    )
    if result.returncode != 0:
        raise SigningError(f"{target.name}: {result.stderr.strip()}")
    return signature
```

`resolve_secret_key` is a `@contextmanager`, not a plain function: it yields the
key `Path` and unlinks the temp material in its own `finally`, so the caller
brackets the *whole* signing batch with `with resolve_secret_key(...) as key:`
and the key survives every `sign_file` call before cleanup. When
`ACCELERATOR_RELEASE_SECRET_KEY` is set it materialises the GHA secret to a
mode-`0600` file inside a `TemporaryDirectory` (atomic directory cleanup even on
an abrupt exit path); otherwise it yields the local dev key path unchanged. It
never falls through to a non-existent path silently — an unset env var with no
dev key raises.

#### 2. Key-generation task

**File**: `tasks/signing.py` (new), registered in `mise.toml`
**Changes**: `keys.generate(pub_path=..., sec_path=...)` runs `minisign -G -W -f
-p <pub> -s <sec>` non-interactively, defaulting to `keys/accelerator-release.pub`
and `keys/accelerator-release.sec` (matched by `.gitignore:34` `/keys/*.sec`) but
accepting explicit output paths so tests direct it at `tmp_path` and never
clobber the tracked public key. It does **not** print the secret: it directs the
admin to provision it straight from the written `.sec` (e.g. piped into `gh
secret set` without echoing), avoiding an extra exposure channel in terminal
scrollback / screen-share / shell history.

#### 3. Key lifecycle documentation

**File**: `RELEASING.md`
**Changes**: New "Release signing key lifecycle" section. The currently-committed
`keys/accelerator-release.pub` is a structurally-valid key of **unknown secret
provenance** and must be treated as untrusted: the runbook mandates generating a
fresh `-W` keypair whose secret has never left the admin's control, and replacing
the committed public half, before any real (non-empty) sub-binary ships. It
documents: generate the `-W` keypair; commit the new public half (launcher
rebuilt from that HEAD so `build.rs` re-embeds it); provision the secret as the
`ACCELERATOR_RELEASE_SECRET_KEY` GHA encrypted secret reachable only from the
approved `release` environment; the strict rollout sequence (commit key → cut and
distribute the launcher built from that HEAD → only then sign any release with
the matching secret); and compromise detection/response — who can access the
secret, how compromise is detected, and expected time-to-remediate given
version-pinned launchers (rotate on compromise only, by embedding a new key in
the next launcher release; the launcher's latent verify-any-of keyring leaves
headroom for an overlap window if ever needed).

### Success Criteria

#### Automated Verification

- [x] `uv run pytest tests/unit/tasks/test_signing.py -v` passes: an ephemeral
      `-W` key signs a temp file and the `.minisig` verifies through the built
      `accelerator-verify` shim. The round-trip **fails closed in CI** — when a
      CI env marker is set it fails (or `xfail(strict=True)`) rather than skipping
      if `minisign` is absent, so this security-critical assertion cannot silently
      no-op; only a local dev run skips (mirroring `verify.rs:60-70`).
- [x] A single `with resolve_secret_key(...) as key:` block signs **multiple**
      files, and the temp key is gone (and mode `0600` while live) after the
      block, including on the exception path.
- [x] `sign_file` writes to the exact signature path it is given (a manifest signs
      to `manifest.minisig`), and a forced non-zero minisign exit raises
      `SigningError` carrying the captured stderr.
- [x] The signature file has the four-line untrusted/base64/trusted/base64 shape.
- [x] `keys.generate` directed at `tmp_path` produces a public key that parses
      (round-tripped via the built `accelerator-verify` shim against a file it
      signed) without touching the tracked `keys/accelerator-release.pub`.
- [x] `mise run check` passes (ruff ALL + pyrefly strict on the new module).

#### Manual Verification

- [ ] The `RELEASING.md` key section reads as an executable runbook.

---

## Phase 2: Multi-binary cross-compile + static assertions + vendored shims

### Overview

Cross-compile the cli workspace launcher (and the verify shim) for all four
targets with a fully-static assertion for musl, and vendor the per-platform
shims the bootstrap requires.

### Changes Required

#### 1. Path + target helpers

**File**: `tasks/shared/paths.py`
**Changes**: Add `RELEASE_STAGING = REPO_ROOT / "dist" / "release"` (gitignored),
`cli_binary_path(name, platform, dir=RELEASE_STAGING)` →
`{dir}/{name}-{platform}`, `VENDORED_SHIM_DIR = REPO_ROOT / "bin"`,
`vendored_shim_path(platform)` → `bin/accelerator-verify-{platform}`, and
`DISPATCHED_SUBBINARIES: tuple[str, ...] = ()` (the crates whose binaries the
manifest lists; empty at HEAD, 0168 appends the visualiser).

**File**: `.gitignore`
**Changes**: Ignore `/dist/`. The vendored `bin/accelerator-verify-*` are **not**
ignored (they are committed).

#### 2. Static-linking assertion + ELF-reader provisioning

**File**: `tasks/build.py`, `mise.toml`
**Changes**: `_assert_static_elf(path)` runs `llvm-readelf -dl <path>` and raises
if a `PT_INTERP` program header or any `DT_NEEDED` (`NEEDED`) dynamic entry is
present. Does **not** assert ELF type `EXEC` (musl static-PIE is `ET_DYN`).
Called only for the two musl triples; darwin keeps the existing
`_assert_magic_bytes`.

The release jobs run on `macos-latest`, which ships **no** `readelf` and no
`llvm-readelf` on PATH (Rust's `llvm-tools-preview` provides `llvm-readobj`, a
*different* tool). So the ELF reader must be provisioned explicitly, not assumed:
pin a **concrete** ELF-reader package (exact backend + version, the way
`minisign` / `actionlint` are pinned in `mise.toml`) that is confirmed to ship
`llvm-readelf` and resolve on `macos-latest` arm64 — not an abstract "llvm entry"
— and make `prerelease:prepare` / `release:prepare` depend on its install task
(mirroring `deps:install:rust-targets`). A preflight asserts `llvm-readelf
--version` resolves before the assertion is relied on. `_assert_static_elf`
**fails closed** — if no ELF reader is found it raises rather than skipping, so a
broken cross-compile can never slip through by silently disabling the check. The
parser is pinned to `llvm-readelf` output (not `llvm-readobj`) and its test
fixtures are captured from real static and real dynamic binaries.

#### 3. cli cross-compile + shim vendoring

**File**: `tasks/build.py`, `mise.toml`
**Changes**: `build.cli_cross_compile` iterates `TARGETS`, runs `cargo zigbuild
--release --target {triple} --manifest-path cli/Cargo.toml`, and for each of
`accelerator` and `accelerator-verify` stages `cli/target/{triple}/release/{bin}`
to `cli_binary_path(bin, platform)` after magic-byte + (musl) static assertions.
`build.vendor_verify_shims` copies the cross-compiled `accelerator-verify` into
`vendored_shim_path(platform)` (`0755`), committed once here and refreshed on
demand. New `mise` tasks: `build:cli:cross-compile`,
`build:vendor-verify-shims`.

```python
@task
def cli_cross_compile(context: Context) -> None:
    for triple, platform in TARGETS:
        context.run(f"cargo zigbuild --release --target {triple} "
                    f"--manifest-path {CLI_WORKSPACE_CARGO_TOML}", pty=True)
        for name in ("accelerator", "accelerator-verify"):
            src = CLI_DIR / "target" / triple / "release" / name
            _assert_magic_bytes(src, triple)
            if "musl" in triple:
                _assert_static_elf(src)
            shutil.copy2(src, cli_binary_path(name, platform))
```

#### 4. Commit the initial vendored shims + drift guard

**File**: `bin/accelerator-verify-{darwin-arm64,darwin-x64,linux-arm64,linux-x64}`
(new, tracked binaries), `tasks/build.py`, `mise.toml`
**Changes**: Produced by `build.vendor_verify_shims` and committed so the
bootstrap's `[[ -x "${shim_source}" ]]` check (`bin/accelerator:64-66`) passes at
HEAD. The shims are the bootstrap's root of trust and are non-reproducible, so a
`lint:vendor-shims:check` task records a marker over the shims' **full build
inputs** — `cli/verify/**`, the `minisign-verify` pin in `cli/Cargo.toml` (the
crate that defines the shim's verification behaviour), and the verify crate's
resolved dependency closure in `cli/Cargo.lock` — and fails CI when any of those
have changed since the shims were last vendored. This catches a `minisign-verify`
bump or lockfile change (neither of which touches `cli/verify/**`) that would
otherwise ship a stale verifier with the guard green. The regeneration
environment (zig + `rustup target add` + `cargo-zigbuild`) is documented in
`RELEASING.md`.

### Success Criteria

#### Automated Verification

- [x] `uv run pytest tests/unit/tasks/test_build.py -v` passes: `_assert_static_elf`
      accepts `file` output for a real static musl binary (the committed
      linux-x64 shim) and rejects a real non-static binary (the committed darwin
      Mach-O shim), and **raises** when `file` is not on PATH (fail-closed)
      rather than skipping. (Uses `file`, not `llvm-readelf` — see the Phase 2
      static-check decision: no light mise `llvm-readelf` exists and the
      luminosity reference uses `file`.)
- [x] A native-host smoke test runs the real `_assert_static_elf` against the
      committed real static musl shim, anchoring the parser to real `file`
      output rather than to itself.
- [x] Path-helper tests for `cli_binary_path` / `vendored_shim_path` pass.
- [x] `mise run build:cli:cross-compile` stages four `accelerator-{platform}`
      binaries into `dist/release/`, each passing its magic-byte and (musl)
      static assertion.
- [x] Each committed `bin/accelerator-verify-{platform}` is executable and passes
      its platform magic-byte check; `lint:vendor-shims:check` passes (shims match
      the current `cli/verify` source revision).
- [x] `mise run check` passes.

#### Manual Verification

- [x] A bootstrap run on darwin (real `CLAUDE_PLUGIN_ROOT`) finds and execs the
      vendored shim without the "verify shim missing" failure.
- [x] `file` on a musl binary reports "statically linked" with no interpreter
      (already on PATH on macOS; no `llvm` install needed).

---

## Phase 3: manifest.json emitter + version-coherence extension

### Overview

Sign the staged binaries, then emit and sign a `manifest.json` conforming to the
frozen schema, sourcing each sub-binary's description from its crate
`Cargo.toml`, and add `manifest.version` to version coherence.

### Changes Required

#### 1. Sign the staged binaries

**File**: `tasks/signing.py`
**Changes**: `sign_staged_binaries(secret_key)` signs an **explicit expected set**
— `TARGETS` × (`accelerator` + `DISPATCHED_SUBBINARIES`) — not a directory scan,
asserting each expected binary is present first (mirroring `upload_and_verify`'s
`missing` guard at `github.py:149-157`) so a partial cross-compile fails closed
rather than silently signing 3 of 4 platforms. It deliberately excludes the
`accelerator-verify-{platform}` shims that Phase 2 also stages under
`dist/release/` (they ship committed in `bin/`, never as release assets). Each
signed binary gets a detached `<name>.minisig`. This is a distinct concern from
manifest emission: the launcher binaries are consumed by the **bootstrap** via
their detached `.minisig` (never manifest entries), and the sub-binary `.minisig`
contents become the manifest's inline `signature` field. Called under one
`with resolve_secret_key(...) as key:` so the whole batch shares the materialised
key. At HEAD `DISPATCHED_SUBBINARIES` is empty, so this signs only the four
launcher binaries.

#### 2. Entry assembly + manifest emitter

**File**: `tasks/manifest.py` (new)
**Changes**: `BinaryEntry` / `PlatformAsset` are defined here.
`collect_entries(version, subbinaries)` builds the typed
`Mapping[str, BinaryEntry]` — per sub-binary and platform it reads the crate's
`Cargo.toml` `package.description` (raising if a listed sub-binary lacks one),
computes `sha256` via `compute_sha256`, and slurps the pre-produced `.minisig`
contents as the inline `signature` — keeping this assembly out of `release.py`
so it is unit-testable in isolation. `build_manifest(version, entries)` assembles
the frozen shape as a `TypedDict` (load-bearing field names checked by pyrefly,
not just at the launcher). `emit_manifest(path, version, entries, secret_key)`
writes the serialised bytes with `atomic_write_text`, then signs *that file* via
`signing.sign_file` with the explicit signature path `manifest.minisig` (the name
the launcher fetches — not `manifest.json.minisig`). The `secret_key` is threaded
in from the caller's `resolve_secret_key` context (the `*_sign` task), since the
materialised key only lives inside that `with` block. At HEAD
`DISPATCHED_SUBBINARIES` is empty, so `binaries` is `{}`.

```python
class PlatformAsset(TypedDict):
    sha256: str
    signature: str

def build_manifest(version: str, entries: Mapping[str, BinaryEntry]) -> Manifest:
    return {
        "schema_version": 1,
        "version": version,
        "binaries": {
            name: {
                "description": entry.description,
                "platforms": {
                    plat: {"sha256": asset["sha256"], "signature": asset["signature"]}
                    for plat, asset in entry.platforms.items()
                },
            }
            for name, entry in entries.items()
        },
    }
```

#### 3. Version coherence extension

**File**: `tasks/build.py`
**Changes**: `validate_version_coherence(require_manifest=False)` gains an explicit
parameter for whether `manifest.version` participates. `create_checksums` (which
runs in *prepare*, before any `manifest.json` is staged) keeps calling it with the
default `require_manifest=False`, so it is unaffected. `emit_manifest` calls it
**once, after writing** the manifest, with `require_manifest=True` — which reads
`manifest.version` and raises if the staged file is missing. This avoids the
create_checksums-style before-and-after bracketing (a "before" call has no manifest
to read on the gitignored staging tree). The `checksums.json` reader stays (the
visualiser still emits it). Because `build_manifest` sets `version` from the same
resolved value fed as `expected_version`, the manifest reader's real job is
guarding against a *stale* on-disk manifest; the load-bearing anti-rollback
enforcement is the workspace-`Cargo.toml` entry checked **before** the
cross-compile (see Phase 5 ordering).

### Success Criteria

#### Automated Verification

- [x] `jsonschema` is added and version-pinned in the dev dependency group (it is
      not currently a dependency; note the ruff-ALL / pyrefly implications).
- [x] `uv run pytest tests/unit/tasks/test_manifest.py -v` passes: an emitted
      manifest validates against `cli/launcher/tests/fixtures/manifest.schema.json`
      (jsonschema, as a shape check complementing — not replacing — the real serde
      parser); `schema_version == 1`, `version` matches, empty `binaries` is
      accepted; a fixture sub-binary sources its `description` from a Cargo.toml
      and a missing description raises.
- [x] **End-to-end fixture test** (adapted): a fixture sub-binary is staged,
      signed, and assembled into a *non-empty* manifest via the real producer
      (`collect_entries` + `emit_manifest`). Rather than an in-process
      `FetchVerifyCacheResolver` call (which needs a fragile cross-language
      artifact handoff), the producer-emitted bytes are proven against the same
      `minisign-verify` the launcher embeds — the built `accelerator-verify` shim
      verifies the raw manifest signature AND each inline per-binary signature —
      plus a jsonschema shape check and a sha256 cross-check. The existing Rust
      `resolution.rs` / `manifest.rs` tests parse the identical shape, so the two
      together cover parse → verify → resolve against producer output at HEAD.
- [x] Round-trip: `emit_manifest` output + its `manifest.minisig` (exact asset
      name) verify through the built `accelerator-verify` shim; fails closed in CI
      when `minisign` is absent.
- [x] Coherence test: a manifest whose `version` disagrees with `plugin.json`
      raises `VersionCoherenceError`; agreement passes; a missing staged manifest
      in the emit flow raises rather than skipping.
- [x] `mise run test:unit:cli` still green (contract fixtures untouched).
- [x] `mise run check` passes.

#### Manual Verification

- [x] The emitted `manifest.json` byte-for-byte matches what a `cat` of the
      signed file shows (no re-serialisation between sign and inspect), and the
      signature asset is named exactly `manifest.minisig`.

---

## Phase 4: Unified upload + re-verify + single-gate publish

### Overview

Extend the upload/verify/publish flow to the launcher binaries, the manifest, and
every `.minisig`. The draft→published transition becomes a **single gate** shared
across the visualiser and launcher tracks, so a verification failure on either
track leaves the whole release in draft and a delete never runs against an
already-published release.

### Changes Required

#### 1. Single-gate upload/re-verify envelope

**File**: `tasks/github.py`
**Changes**: To preserve the additive invariant, Phase 4 **adds**
`upload_and_verify_release` alongside a fully-intact, still-publishing
`upload_and_verify` — it does not mutate the live function. `release._publish` is
repointed to the new function and the old `upload_and_verify` is **removed** in
Phase 5 (the same change that wires the launcher track), so there is never a
window where the visualiser release is stranded in draft and exactly one publish
path exists once both land. The new `upload_and_verify_release(context, version)`
orchestrates the whole release, owning the single `--draft=false` transition:
1. upload **all** assets across both tracks — the visualiser binaries + debug
   archives (unchanged), each `accelerator-{platform}` + its detached `.minisig`,
   `manifest.json` + `manifest.minisig`, and each sub-binary asset + `.minisig`;
2. re-download and re-verify **all** assets;
3. flip `--draft=false` **exactly once**, only after every re-verify has passed.

The visualiser and launcher re-verification bodies are factored into one shared
envelope parameterised by asset set + per-asset verify strategy (sha256 vs
shim-minisig), so the draft-preserve-vs-delete policy has a single implementation.
On `AssetVerificationError` the forensic-annotate-and-preserve seam
(`github.py:165-171`) fires and the release stays draft; any other exception
deletes the release + tag (`:172-177`) — and because the delete path is inside
the pre-publish envelope, it can never run once the release is published.

Uploads use `gh release upload --clobber` so a preserved-draft release can be
re-driven to green after fixing the cause, without manual asset deletion.
`_emit_forensic_alert` is parameterised with the failing track's label (visualiser
vs launcher/manifest) instead of the hardcoded `Visualiser release` (`github.py:25`).

Re-verification after re-download:
- launcher binaries → shim-minisig against the committed key (no sha256 recorded
  for the launcher);
- `manifest.json` → verify `manifest.minisig` via the shim;
- sub-binary assets → sha256 + the **inline** manifest `signature` (the load-bearing
  artifact the launcher actually consumes; any detached sub-binary `.minisig` is
  redundant convenience, not the tested contract).

```python
def _reverify_via_shim(context, tag, asset, sig_asset) -> None:
    shim = vendored_shim_path(host_platform())  # host-arch shim; macos-latest is arm64
    pub = REPO_ROOT / "keys" / "accelerator-release.pub"  # committed/embedded key
    bin_tmp, sig_tmp = _download_pair(context, tag, asset, sig_asset)
    result = subprocess.run(
        [str(shim), str(pub), str(sig_tmp), str(bin_tmp)],
        check=False, capture_output=True, text=True)
    if result.returncode != 0:
        raise AssetVerificationError(f"{asset}: minisign verification failed")
```

`_reverify_via_shim` verifies against the **committed** `keys/accelerator-release.pub`
(the same file `build.rs` embeds), not a key derived from the signing secret — so
the check genuinely guards "signed by the key launchers embed" and cannot pass
tautologically. The shim is resolved to the **runner host arch** via the
`uname`→alias mapping in `tasks/shared/targets.py`. Invocations use argv lists
(not f-string shell commands), matching `download_release_asset`.

### Success Criteria

#### Automated Verification

- [ ] `uv run pytest tests/integration/tasks/test_github.py -v` passes (extending
      the existing draft-preserve fixtures): the upload set equals visualiser
      assets + launcher binaries + their `.minisig` + `manifest.json` +
      `manifest.minisig` (+ sub-binary assets + `.minisig` when present); a
      corrupted re-download or swapped `.minisig` on **either** track raises
      `AssetVerificationError`, the release is **not** published and **not**
      deleted; an unrelated error deletes it; `--draft=false` is flipped exactly
      once and only after all re-verifies pass.
- [ ] A binary/manifest signed by a non-committed key fails re-verify and
      preserves the draft (guards the committed-key pinning).
- [ ] A re-run after a preserved draft succeeds (uploads are `--clobber`
      idempotent); the forensic alert names the failing track.
- [ ] The existing visualiser upload/verify tests remain green (additive).
- [ ] `mise run check` passes.

#### Manual Verification

- [ ] A local dry-run against a fork release shows the launcher + manifest +
      minisig assets present and the draft published only after every re-verify.

---

## Phase 5: Wire the new track into the release workflow + CI secret

### Overview

Call the new build/sign/upload steps from the release orchestration tasks and add
the signing secret + launcher attestation to the workflow. The signing secret is
scoped to a dedicated **sign** step so it is never in the environment during
compilation. Merge is gated on the admin provisioning
`ACCELERATOR_RELEASE_SECRET_KEY`, and signing fails **closed** without it.

### Changes Required

#### 1. Orchestration + ordering

**File**: `tasks/release.py`
**Changes**: The prepare/sign/finalise split keeps the secret out of the compile:
- `prerelease_prepare` / `release_prepare` run `build.cli_cross_compile`
  **after** the version bump (so the launcher embeds the *new* `CARGO_PKG_VERSION`
  it will later be asked to match against `manifest.version` — else it would
  reject its own manifest with `ManifestVersionMismatch`), plus the existing
  `server_cross_compile` / `create_debug_archives` / `create_checksums`. No secret
  is present in this step. A post-cross-compile assertion confirms each staged
  launcher's embedded version equals the release version by **grepping the version
  string out of the binary** (the three foreign-arch targets cannot be executed on
  the host), not by running `--version`.
- A new `*_sign` task runs `signing.sign_staged_binaries` then
  `manifest.emit_manifest` under one `resolve_secret_key` context. This is the
  **only** task that receives the secret. It fails closed: an absent secret
  raises (a clear "secret not provisioned — do not merge Phase 5 yet" preflight
  message), never a silent skip.
- The `finalise` tasks' `_publish` calls `github.upload_and_verify_release` (the
  single-gate envelope), replacing the direct visualiser-only publish.

Every staged binary, `manifest.json`, and all `.minisig` outputs are written under
the gitignored `dist/release/` tree so `git.commit_version`'s `git add .`
(`git.py:73`) can never sweep a non-reproducible binary or materialised secret
into the version-bump commit. A guard asserts `git status --porcelain` is free of
build artifacts before `commit_version` runs.

#### 2. Workflow sign step + attestation

**File**: `.github/workflows/main.yml`
**Changes**: In `prerelease` (`:347-360`), `release` (`:432-445`), and the
post-stable re-cut (`:447-460`), a dedicated `Sign*` step (running `*:sign`) is
inserted between `Prepare*` and `Attest*`, and **only that step** carries
`ACCELERATOR_RELEASE_SECRET_KEY: ${{ secrets.ACCELERATOR_RELEASE_SECRET_KEY }}` —
the `Prepare*` steps (which run `cargo zigbuild` over untrusted transitive crate
build scripts) do not. Each `Attest*` `subject-path` glob is extended to include
`dist/release/accelerator-*` alongside the visualiser glob.

**Secret scope:** the `prerelease` job runs unapproved on every push and cannot
carry `environment: release` (an approval-gated environment would hold the release
concurrency lock and deadlock later prereleases — `main.yml:397-403`). So the
signing secret is a **repository/org secret** readable by the `prerelease` job,
not an environment-scoped one. The accepted consequence is that **push-to-`main`
is prerelease-signing authority** — any merged commit is signed by the production
key and published as a launcher-trusted prerelease with no release-time human
gate. This is bounded by version-pinning (a launcher only trusts its own release's
manifest), and the trust boundary is made explicit: `RELEASING.md` documents that
**required review + branch protection on `main` is the control equivalent to
signing authority**, so `main` must enforce required PR review before merge. The
stable-release path retains its separate `approve-release` human gate.

This phase is the **explicit last merge** of the plan, gated on a checklist item
confirming the GHA secret exists and the freshly-generated public key is committed
and shipped in the launcher (see Phase 1 runbook). Because signing fails closed,
merging before provisioning would fail the whole prepare/sign flow — including the
visualiser — so the sequencing is load-bearing, not advisory.

### Success Criteria

#### Automated Verification

- [ ] `uv run pytest tests/unit/tasks/test_workflows.py -v` passes: the signing
      secret is referenced **only** in the `Sign*` step (not `Prepare*`), and the
      attest globs include the launcher binaries.
- [ ] `uv run pytest tests/integration/tasks/test_release.py -v` passes: `*_prepare`
      runs `cli_cross_compile` after the version bump; `*_sign` runs
      `sign_staged_binaries` + `emit_manifest` under `resolve_secret_key` and
      raises with the preflight message when the secret is absent; `_publish` calls
      `upload_and_verify_release`; the staged-launcher embedded version equals the
      release version; the artifact-cleanliness guard fires if a build artifact is
      outside `dist/release/` before `commit_version`.
- [ ] `mise run lint:workflows:check` (actionlint) passes.
- [ ] `mise run check` passes.

#### Manual Verification

- [ ] With `ACCELERATOR_RELEASE_SECRET_KEY` provisioned, a full CI prerelease
      publishes launcher + manifest + minisig assets and the launcher built from
      that commit bootstraps and loads the manifest end-to-end.
- [ ] Confirm the secret is provisioned and the fresh public key committed +
      shipped in the launcher **before** merging this phase (releases fail closed
      without the secret).

---

## Phase 6: Drop the runtime provenance hook + correct docs

### Overview

Remove the stale runtime `gh attestation verify` claim; keep CI-side SLSA.

### Changes Required

#### 1. Docs correction

**File**: `RELEASING.md`, `README.md`
**Changes**: Remove the "Out-of-band provenance verification"
`ACCELERATOR_VISUALISER_VERIFY_PROVENANCE` runtime claim (`RELEASING.md:153-163`,
`README.md:617,623-631`); the hook does not exist in `launch-server.sh`. Keep the
`gh attestation verify` command as a user-run out-of-band check and keep all CI
`actions/attest-build-provenance@v2` steps.

### Success Criteria

#### Automated Verification

- [ ] `grep -rn ACCELERATOR_VISUALISER_VERIFY_PROVENANCE README.md RELEASING.md`
      returns nothing.
- [ ] `mise run check` passes.

#### Manual Verification

- [ ] `RELEASING.md` still documents CI SLSA attestation and the user-run
      out-of-band verification.

---

## Testing Strategy

### Unit Tests

- `test_signing.py` (`tests/unit/tasks/`) — ephemeral-key sign → shim-verify
  round-trip (fails closed in CI when minisign absent); multi-file signing under
  one `resolve_secret_key` with the temp key gone (`0600` while live) after the
  block incl. exception path; explicit signature path (`manifest.minisig`);
  `SigningError` on non-zero exit; `keys.generate` at `tmp_path` without clobber.
- `test_build.py` (`tests/unit/tasks/`) — `_assert_static_elf` accept/reject on
  real `llvm-readelf` fixtures + fail-closed on missing reader; native-host smoke
  test; `cli_binary_path` / `vendored_shim_path`; magic-byte reuse.
- `test_manifest.py` (`tests/unit/tasks/`) — schema validation via pinned
  `jsonschema` (incl. empty `binaries`); description sourcing + missing-description
  error; manifest sign→verify round-trip; **non-empty fixture-crate end-to-end**
  through the launcher verifier; coherence incl. `manifest.version` + missing-manifest
  raise.
- `test_github.py` (`tests/integration/tasks/`) — full visualiser+launcher+manifest+
  minisig upload set; single-gate publish (draft preserved on `AssetVerificationError`
  on either track, not published, not deleted; delete on other errors); non-committed-key
  re-verify fails; `--clobber` re-run; track-labelled forensic alert.
- `test_workflows.py` (`tests/unit/tasks/`) / `test_release.py`
  (`tests/integration/tasks/`) — secret wired only to `Sign*`; attest globs;
  cross-compile-after-bump; fail-closed sign; artifact-cleanliness guard; prepare/
  sign/publish call graph.

### Integration Tests

- `mise run test:unit:cli` — the frozen contract (`verify.rs`, manifest fixtures)
  stays green: proof the producer conforms to what the consumer parses.
- Manual fork dry-run: `mise run prerelease` (local-dev, no attestation) against a
  fork, then a launcher built from that commit bootstraps + loads the manifest.

### Manual Testing Steps

1. `mise run keys:generate`; confirm a `.pub`/`.sec` pair and a verifying
   round-trip.
2. `mise run build:cli:cross-compile`; `llvm-readelf -dl` a musl binary → no
   `INTERP`/`NEEDED`.
3. `mise run build:vendor-verify-shims`; run `bin/accelerator` on darwin → shim
   found, launcher fetched + verified (against a fork release).
4. Corrupt a staged binary before re-verify → release stays draft.

## Performance Considerations

The cli cross-compile adds four `cargo zigbuild` invocations to each release
(cli workspace is small relative to the visualiser + frontend already built).
Signing is a handful of fast `minisign -S` calls. No runtime cost — verification
already ships in the launcher.

## Migration Notes

Additive through Phase 4: `checksums.json`, `server_cross_compile`, and
`launch-server.sh` are untouched, and Phases 1–4 wire into nothing live, so every
intermediate commit still releases the visualiser. The committed vendored shims
are the only new tracked binaries. The real production key is generated +
provisioned out-of-band by an admin (runbook in Phase 1).

Phase 5 is the load-bearing exception and must merge **last**, only after the
admin has (a) provisioned `ACCELERATOR_RELEASE_SECRET_KEY`, and (b) committed the
freshly-generated public key and shipped a launcher built from that HEAD. Signing
fails **closed** — once Phase 5 is wired, an absent secret aborts the whole
prepare/sign flow (visualiser included), so merging early is a total release
outage, not a graceful degradation. Gate the merge on the Phase 1 checklist.

**Recovery + residual git state:** the single publish gate governs only the
`--draft=false` flip; `_publish` runs `commit_version` / `tag_version` / `push`
*before* upload+re-verify (`release.py:23-29`), so a re-verify failure leaves the
version-bump commit and its pushed tag advanced while the release stays draft. The
`--clobber` idempotency only helps if you re-invoke `upload_and_verify_release`
against the **same preserved tag** — re-running the full workflow re-bumps the
pre.N version and cuts a new release, orphaning the draft + tag. `RELEASING.md`
documents the concrete recovery entrypoint (re-run only the upload/verify against
the preserved tag) and cleanup for orphaned draft+tag, and flags that the
version-bump commit persists after a preserved-draft failure and must be
reconciled. (Re-ordering `push` to after a successful re-verify is a future
tightening, out of scope here.)

## References

- Original work item: `meta/work/0165-multi-binary-distribution-and-release-pipeline.md`
- Research: `meta/research/codebase/2026-07-06-0165-multi-binary-distribution-release-pipeline.md`
- Frozen contract: `cli/launcher/src/launch/outbound/resolve/mod.rs:137-172`,
  `manifest.rs:21-120`, `keys.rs:11-85`, `cli/verify/tests/verify.rs:33-130`,
  `cli/launcher/tests/fixtures/manifest.schema.json`
- Bootstrap: `bin/accelerator:64-194`
- Producer seams: `tasks/build.py:104-239`, `tasks/github.py:136-178`,
  `tasks/version.py:68-84`, `tasks/release.py:35-71`, `tasks/shared/paths.py`
- Workflow: `.github/workflows/main.yml:300-461`
- ADRs: ADR-0046, ADR-0054
- Mirror (luminosity): work item 0008
