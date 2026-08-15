---
type: work-item
id: "0210"
title: "Settle the vendored-runtime tree-artifact mechanisms"
date: "2026-08-15T13:24:07+00:00"
author: Toby Clemson
producer: create-work-item
status: ready
kind: spike
priority: high
parent: "work-item:0196"
relates_to: ["work-item:0189", "work-item:0186", "work-item:0205"]
derived_from: ["plan:2026-08-11-0196-design-vendored-runtime-distribution"]
tags: [rust, design, launcher, distribution, tree-artifacts, playwright]
last_updated: "2026-08-17T10:37:32+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0210: Settle the vendored-runtime tree-artifact mechanisms

**Kind**: Spike
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Four mechanisms in `plan:2026-08-11-0196-design-vendored-runtime-distribution`
have no settled answer, and each was answered wrongly on paper at least once
across three review passes of the plan they came from. Each changes the shape of
the code that follows it, and each is cheap to check against the real system.
This spike answers them against prototypes, so the plan can specify a mechanism
rather than a candidate.

The distinguishing fact is the same one work-item:0205 recorded for the
warm-dispatch gate: every failure was a *design* failure authored ahead of the
evidence that would have settled it. A libc probe was specified from a
filesystem convention that classifies macOS as unsupported; a reaper was gated
on a signal with no data source; a test trust root was put behind a cargo
feature in a repository that passes `--all-features` to both its lint and its
test tasks. The remedy is to measure first and specify second.

## Context

Work-item:0196 splits into two plans. The first —
`plan:2026-08-11-0196-design-cli-migration` — is implemented and merged (PR #64,
validated partial on 2026-08-13). The second vendors the Playwright runtime so
the system Node prerequisite goes away: the launcher learns to resolve
directory-tree artifacts, the release pipeline assembles them from verified
upstream inputs, and the executor swaps onto them.

That second plan cannot be scheduled while these four questions are open. Three
of its four phases contain sections written against a candidate mechanism, and
in each case the plan records why the candidate is doubted rather than pretending
it is settled.

The questions are not equally weighted. SQ-1 and SQ-4 are correctness questions
whose wrong answers ship — a probe that downgrades every Mac, or a launcher that
trusts a test key. SQ-2 and SQ-3 are design questions whose wrong answers cost a
rewrite of the tree resolver's hot path and its reaper respectively.

## Requirements

### SQ-1: What identifies the host's libc, and what does the answer say on macOS?

ADR-0057 scopes browser automation to Playwright's supported platforms, which
excludes musl. `HOST_PLATFORM` (`cli/launcher/src/launch/outbound/resolve/mod.rs`)
is a compile-time constant reading `linux-x64` on Alpine and Debian alike —
`TARGETS` builds Linux against `*-unknown-linux-musl` precisely so one binary
runs on every libc — and the manifest's platform axis carries no libc dimension.
Without a probe, an Alpine host fetches ~294MB of glibc-linked artifacts, seals
them, and dies at `execve` with a bare `ENOENT` from the absent dynamic loader.

An earlier draft classified from the set of loader paths present, globbing
`/lib/ld-musl-*.so.1` and `/lib64/ld-linux-*.so.2`, with "neither present"
meaning `unsupported-platform`. **On macOS neither exists**, so that logic
downgrades every Mac — a supported platform and the primary development platform
— before it touches a tree. NixOS places its glibc loader in the Nix store
rather than `/lib64`; Debian with `musl-tools` and Alpine with `gcompat` have
both.

Settle:

- Does the probe short-circuit on non-Linux targets at compile time?
- Is there a positive glibc signal that does not depend on a filesystem
  convention? Reading the ELF interpreter of `/proc/self/exe` is the leading
  candidate.
- Which way does an ambiguous host fail — and is failing *open* (attempt the
  glibc runtime and let `execve` fail) better than failing closed, for a
  capability that already has a graceful downgrade?

**Deliverable**: a probe that returns the right answer on macOS, Debian, Alpine
and one non-standard-layout Linux, with the classification a pure function over
an injected observation.

### SQ-2: Can a tree cache hit be bound to the release key without a manifest fetch?

The warm path must be local and offline — a crawl makes 100–200 launcher
invocations — which is why the design avoids `load_manifest` on a hit. But that
leaves the hit path with no cryptographic check at all: an attestation whose
digest matches the digest in its own directory name is self-referential, and the
cache root is selectable via `ACCELERATOR_CACHE_DIR`, which per-project config
can set and which this plan actively recommends relocating.

Work-item:0205 recorded the adjacent finding that a cache-hit sha256 is a
corruption check rather than a trust check, which is the same distinction this
question turns on.

Settle:

- Does storing the manifest's minisign signature over the archive digest *inside*
  the attestation, and verifying it during `locate`, cost what it looks like it
  costs — one Ed25519 verify over ~100 bytes? Measure it on the hit path against
  the ~30ms warm bootstrap budget work-item:0186 established.
- Is requiring the seal itself (`0444`/`0555`) as an acceptance condition a cheap
  additional discriminator? A git checkout or an unzip cannot produce read-only
  files.

**Deliverable**: a measured figure for signature verification on the hit path,
and a decision on whether the attestation is signed.

### SQ-3: What can be known about who holds a materialised tree?

The reaper and the `prune` verb were specified to gate on "the owning pid and its
start time" plus "a skip for any generation a live process still holds". Neither
has a data source: temp names carry only a generation, nothing records a pid
after the publish rename, and there is no portable way to ask which process holds
a directory. Yet that gate is what makes `repair` safe against a live daemon —
the whole reason the generation design exists.

Settle:

- What does a lease look like? An `flock`-held file inside the generation,
  written by the executor and observable by the launcher, is the obvious shape.
  Prototype it and check that the lock survives the daemon's lifetime, is visible
  cross-process, and is released on kill.
- Does `prune` need it at all, or is a minimum retention window — keep the
  previous generation until the next successful materialisation plus a grace
  period — sufficient and simpler?

**Deliverable**: a working in-use signal, or a reasoned decision that retention
windows replace it.

### SQ-4: How is a test trust root introduced without `--all-features` turning it on?

The container fixtures for AC6 and AC12 must verify artifacts they signed
themselves, but `cli/launcher/build.rs` embeds `keys/accelerator-release.pub`
unconditionally with no override, and `keys.rs` `include_str!`s it. A non-default
cargo feature was proposed — and `tasks/lint/cli.py:7` passes
`--workspace --all-targets --all-features` while `tasks/test/cli.py:13` passes
`--all-features` deliberately, to enable `bash-parity`. The feature would
therefore be **on** during `mise run cli:check` and `mise run test:unit:cli`:
either `build.rs` fails on the unset key path, making the plan's own
"`mise run` exits 0" criterion unsatisfiable, or every launcher in `cli/target/`
silently trusts the test key, reachable through the documented
`ACCELERATOR_LAUNCHER_BIN` dev override.

Settle:

- Is a build-time environment variable read by `build.rs` with no feature flag
  the right shape — unreachable by `--all-features`, but then what stops a stray
  variable in a release build?
- Or should the fixtures verify against the *real* key, by having the spike
  produce a signed synthetic artifact once?
- Or should the trust root be **substituted** rather than widened, so a leaked
  build fails closed and loudly instead of silently trusting an extra key
  forever?

**Deliverable**: a mechanism plus a **positive** guard — an assertion that a
shipped launcher embeds exactly the committed production key, rather than a
negative marker-string scan.

## Acceptance Criteria

- [x] **AC1.** SQ-1 is answered by a prototype run on macOS, Debian, Alpine and
      one non-standard-layout Linux, with the observed classification recorded
      per host and the fail-open-versus-fail-closed choice stated with its
      reasoning. *Six hosts, not four: the two ambiguous shapes the brief named
      were run as well. Fail open, with the reasoning and the prior art.*
- [x] **AC2.** SQ-2 records a measured hit-path verification cost against the
      ~30ms warm bootstrap budget, taken with work-item:0205's settled method
      rather than a shell loop, and states whether the attestation is signed.
      *51.7µs cold-process, 0.17% of 29.92ms; in-process against the pinned
      crate in the release profile; the attestation is signed.*
- [x] **AC3.** SQ-3 produces either a working cross-process in-use signal,
      demonstrated to survive a daemon's lifetime and to release on kill, or a
      recorded decision that a retention window replaces it, with the window and
      grace period stated. *A shared `flock` lease, demonstrated across `exec`
      into a detached daemon with every ancestor exited, and on `SIGKILL`.*
- [x] **AC4.** SQ-4's mechanism is demonstrated to be unreachable from
      `mise run cli:check` and `mise run test:unit:cli`, and its positive guard
      is written and shown to fail against a launcher embedding any key other
      than the committed production one. *Unreachable by construction — a second
      `[[bin]]` involves no feature and no env var, so neither invocation can
      alter the shipped binary's trust root. Guard verified in both directions,
      and its positive half against a real built launcher.*
- [x] **AC5.** Every answer is recorded on this work item, with the prototype
      that produced it referenced, and
      `plan:2026-08-11-0196-design-vendored-runtime-distribution` is edited to
      match — the affected sections corrected rather than annotated. *All four
      marked sections rewritten to specify mechanisms; the eight downstream
      cross-references, the phase diagram and `blocked_by` updated with them.*
- [x] **AC6.** Any answer that changes an accepted ADR — most likely ADR-0060, on
      tree addressing and adoption — has its amendment raised via
      `/accelerator:review-adr` rather than edited in place. *Both raised as
      supersessions, since an accepted ADR is immutable: ADR-0061 supersedes
      ADR-0060, ADR-0062 supersedes ADR-0057. Neither superseded document was
      edited beyond its status fields. Both successors are `proposed`.*
- [x] **AC7.** Every throwaway artefact is positively asserted absent: no
      prototype crate or `examples/` target remains in `cli/`, no dev-override
      input is set, and `sha256(keys/accelerator-release.pub)` matches its
      committed value. *Verified: no cargo `examples/` target and no prototype
      crate under `cli/`, no dev-launcher marker, `ACCELERATOR_LAUNCHER_BIN`
      unset, key digest `0f3fe9a9…f6fb2e` unchanged. Every prototype was built
      in the session scratchpad.*

## Dependencies

- **Blocks**: `plan:2026-08-11-0196-design-vendored-runtime-distribution`, whose
  Phases 1, 2 and 3 each contain a section written against an unsettled
  mechanism and which should not be scheduled until this closes.
- **Blocked by**: nothing. Every question is answerable against the tree as it
  stands.
- **Parent**: work-item:0196.

## Assumptions

- A prototype is cheaper than a review pass. Three passes over the superseded
  plan closed the previous pass's findings and introduced new criticals in the
  fix material, and every one landed in the phases these four questions govern.
- The four are independent and can be answered in any order or in parallel.
  SQ-2's measurement and SQ-4's mechanism both touch `cli/launcher/`, so they
  are the pair most likely to want doing together.

## Technical Notes

- SQ-1's classification belongs in `cli/design/src/runtime/platform.rs` as a pure
  function over an injected observation, with the adapter half in
  `cli/design-adapters/src/platform.rs`. The `design` domain crate may import
  only `std`/`core`/`alloc`, `kernel::Error` and `crate` (`cli/pup.ron:231-245`),
  so anything reading the filesystem or `/proc` arrives through a port.
- SQ-2's measurement must not use a bash loop. Work-item:0186 records its own
  20-run loop figures as "not method-comparable" to its interleaved medians, and
  work-item:0205 exists because three prose specifications of a warm-path
  measurement failed review.
- SQ-3's leading candidate is constrained by an existing contract: the
  `owner.<nonce>` / `reclaiming.<pid>.<nonce>` mkdir-lock sentinel protocol is
  shared on-disk state between `cli/corpus-adapters` and `scripts/atomic-common.sh`,
  and `cli/design-adapters/src/lock.rs` already implements a single `flock`
  backend with a `Drop` guard. Prefer extending what exists to inventing a third
  lock scheme.
- SQ-4's guard has a natural home: `tests/integration/deny/test_launcher_feature_graph.py`
  already parametrises absence assertions over a tuple of crate names for exactly
  this regression class, and `cli/launcher/tests/resolution.rs` constructs
  `TrustedKeys` in-process via `from_public_key_files`, which is the only
  injection seam that exists today.

## Spike Outcome

**Date**: 2026-08-17. **Time spent**: one session; no box was stated in the
brief. **Verdict**: all four questions answered against prototypes. Every one of
the four candidate mechanisms the brief carried was wrong or incomplete, and in
three cases the brief's own leading candidate was falsified by measurement.

Each answer below was produced by a throwaway prototype in the session
scratchpad, discarded per AC7. Each is described precisely enough to rebuild.

## Findings

### SQ-1: the host's libc, and macOS

**Mechanism**: a compile-time `#[cfg(target_os)]` short-circuit, then two
observations on Linux, classified musl-first.

1. The **basename** of `/bin/sh`'s `PT_INTERP`. `ld-musl-*` is positive musl
   evidence and refuses immediately.
2. Whether the psABI interpreter the artifact demands — `/lib/ld-linux-aarch64.so.1`
   on aarch64, `/lib64/ld-linux-x86-64.so.2` on x86_64 — is present and
   executable.

Ordering is load-bearing: `gcompat` installs a glibc loader on a musl host, so
signal 2 passes there and signal 1 must win.

**Prototype**: a static-musl Rust binary (built in `rust:alpine`, run unmodified
in each container) parsing ELF program headers for `PT_INTERP`, plus a natively
built darwin copy. Seven unit tests over injected observations, all passing.

| Host | `/bin/sh` interp | Required loader | Verdict | Correct |
|---|---|---|---|---|
| macOS arm64 | compile-time short-circuit | — | Supported | yes |
| Debian 12 | `ld-linux-aarch64.so.1` | present | Supported | yes |
| Debian 12 + `musl-tools` | `ld-linux-aarch64.so.1` | present | Supported | yes |
| Alpine 3.20 | `ld-musl-aarch64.so.1` | absent | UnsupportedLibc | yes |
| Alpine 3.20 + `gcompat` | `ld-musl-aarch64.so.1` | **present** | UnsupportedLibc | yes |
| NixOS (`nixos/nix`) | `ld-linux-aarch64.so.1` | **absent** | UnsupportedLoaderAbsent | yes |

Three claims in the brief and the plan are falsified:

- **`PT_INTERP` of `/proc/self/exe` cannot work.** A static musl binary has no
  `PT_INTERP`; the probe returned "no interpreter" on Debian, Alpine and NixOS
  alike. It is a constant, not a signal. Corroborated upstream: `detect-libc`'s
  equivalent step works only because `node` is itself dynamically linked.
- **The loader's directory carries no information; its basename does.** NixOS
  keeps glibc's loader at
  `/nix/store/jp8avbmpfcdnm0axwrzyk072nmq9cr0d-glibc-2.42-67/lib/ld-linux-aarch64.so.1`.
  Classifying on location misclassifies NixOS; classifying on basename does not.
- **Both named ambiguous hosts really do carry both loaders**, so the
  filesystem-glob approach is genuinely undecidable on each. Confirmed by
  observation on Debian + `musl-tools` and Alpine + `gcompat`.

**"Is the host glibc?" is the wrong question.** A real glibc-linked aarch64
binary demanding `/lib/ld-linux-aarch64.so.1` exits 0 on Debian and fails on
NixOS with `cannot execute: required file not found` (exit 127) — the bare
`ENOENT` this probe exists to prevent. NixOS is a fully supported libc whose
loader is not where the artifact demands it. The right question is the one the
kernel asks, which is why signal 2 exists.

**Fail direction**: fail **open**. Decided by the author on the cost asymmetry,
and matching unanimous prior art — `rustup-init.sh` defaults `_clibtype="gnu"`,
`nodejs/unofficial-builds`' `is_musl()` returns false on any `ldd` failure,
`detect-targets` returns both targets glibc-first, and Playwright performs no
libc detection at all. The two-signal design shrinks the ambiguous case to one
host shape: musl **and** a static `/bin/sh` **and** `gcompat`.

**Consequence beyond the brief's scope**: NixOS needs a **third downgrade
reason**, distinct from `unsupported-platform`, because its remediation is
`nix-ld` or `design.browser_path` rather than "use a different distribution".

### SQ-2: binding a tree cache hit to the release key

**Sign the attestation.** One Ed25519 verify over a 244-byte attestation, using
the launcher's own pinned `minisign-verify =0.2.5` in the shipped release
profile (`strip = true`, `lto = "thin"`), aarch64 darwin:

| Measurement | Median | p99 / p90 | Share of 29.92ms |
|---|---|---|---|
| Warm in-loop, n=5000 | 43.5µs | 58.5µs (p99) | 0.15% |
| Cold-process, n=40 | 51.7µs | 58.8µs (p90) | **0.17%** |

Predicted band before measuring was 50–200µs; the warm figure came in just
below it, so the operation is marginally cheaper than predicted. Two trees cost
0.35% of the warm budget. Measured in-process against the real crate rather
than through a shell loop, per the discipline work-item:0205 established.

**The `0444`/`0555` seal is not a discriminator.** The brief's claim that "a git
checkout or an unzip cannot produce read-only files" is half wrong, and the
wrong half is the one that matters:

| Route | Reproduces the seal? |
|---|---|
| `tar` extract | yes — modes preserved exactly |
| `unzip` | yes — `0444` preserved |
| git checkout | no — records `100644` for a `0444` file |

The artifacts are `tar.gz`, so `tar xzf` into the cache root reproduces the seal
perfectly. Keep the seal check — the `stat` already happens in `locate` step 3,
so it costs no extra syscall — but as a consistency signal only. The signature
is the sole trust anchor on the hit path.

### SQ-3: who holds a materialised tree

**A shared `flock` lease inside the generation directory.** The launcher opens
it, takes `LOCK_SH`, clears `FD_CLOEXEC`, and the open file description is
inherited through `exec` into the detached daemon. The reaper and `prune` probe
with `LOCK_EX | LOCK_NB`; `EWOULDBLOCK` means a live holder. The kernel is the
liveness oracle — no pid, no start time, no sentinel protocol.

Demonstrated against the real process topology (open → `exec` → spawn detached
daemon → every ancestor exits):

- Survives the ephemeral holder's lifetime: with all ancestors gone, the lease
  is still held by the daemon alone. `F_GETFD` returned 0 after the `exec`,
  confirming the descriptor and its lock survived.
- Visible cross-process: a separate probe process observes it held.
- Released on kill: `SIGKILL` the daemon and the next probe is free. No cleanup
  code, so no stale state is reachable.
- Multiple concurrent holders are admitted and the lease stays held until the
  **last** one dies — which is exactly "any generation a live process still
  holds", for concurrent crawls.

**This is a second, distinct lock — not an extension of the existing one.**
`cli/design-adapters/src/lock.rs:1-10` documents that its descriptor
deliberately does *not* leak into the daemon, because holding it for the
daemon's lifetime would falsely report `another-launcher-running`. A lease must
invert precisely that property. The technical note's instruction to prefer
extending what exists does not apply here.

**A pid-based gate would repeat a documented failure.**
`meta/notes/2026-05-19-playwright-daemon-owner-pid-ephemeral-shell.md` records
the daemon shutting down with `owner-exited` seconds after every bootstrap
because `--owner-pid $$` bound it to an ephemeral Bash-tool shell; the note's
own resolution was to stop using the pid. The reaper's specified gate — "the
owning pid **and** its start time" — is the same primitive, in the same process
environment, for a daemon with the same lifetime.

**Retention windows are demoted to a backstop**, not a replacement: they cannot
distinguish "old but in use" from "old and abandoned", so they either reap a live
daemon's tree — defeating the reason generations exist — or retain for ever. Keep
an age backstop only for generations carrying no lease file, i.e. those left by a
launcher predating this mechanism.

### SQ-4: a test trust root that `--all-features` cannot reach

**A second `[[bin]]` in `cli/launcher`** — none of the brief's three options.
The repository has already solved this exact problem and documented why, at
`cli/vcs-adapters/Cargo.toml:27-33`: *"A second `[[bin]]` rather than a `stub`
feature: the crate must gain no `[features]` entry beyond `bash-parity`, and
CI's `--all-features` would turn a fixture feature on workspace-wide."*
`cli/launcher/Cargo.toml:17-21` already carries the in-package fixture-bin
convention for `accelerator-fixture`.

This needs no cargo feature, no build-time environment variable and no
`build.rs` change, and the shipped `accelerator` binary keeps
`TrustedKeys::embedded()` unconditionally — so it cannot be made to trust a test
key at all, rather than merely being unlikely to.

**The injection seam already exists and is public.**
`FetchVerifyCacheResolver::new` and `with_fetcher` both take `TrustedKeys` by
value; the resolver is key-agnostic and `main.rs:68` is the only place the
embedded key enters. `cli/launcher/tests/resolution.rs:135-142` already injects a
freshly generated keypair through that seam, with no feature, cfg or env var.

**The build-time environment variable is rejected on evidence.** Built once with
a substituted key and then rebuilt with the variable **unset**, the binary still
embedded the test key: without `cargo:rerun-if-env-changed` cargo never re-runs
`build.rs`. The failure is silent and persists in any build or CI cache that ever
produced the fixture — worse than the stray-variable case the brief anticipated,
because the variable need not still be set. No `cargo:rerun-if-env-changed`
appears anywhere in this repository, so it would not be added by habit.

**The positive guard**, following the existing template at
`tasks/build.py:560-576` (`assert_staged_launcher_versions`, which byte-greps a
staged cross-compiled launcher for the release version, with unit tests at
`tests/unit/tasks/test_build.py:85-114`): assert the committed key's base64 line
appears in the staged artifact exactly once. Verified both directions —

```
built-with=production  committed=1  test=0  -> PASS
built-with=test        committed=0  test=1  -> FAIL
```

— and the positive half re-confirmed against a real built launcher in
`cli/target`, which yielded exactly one occurrence. `include_str!` places the key
in `.rodata`, which `strip = true` leaves intact.

**SQ-2 and SQ-4 are coupled**, which is why the brief sensed they belonged
together: if the attestation is signed, a container fixture cannot pre-place a
sealed tree without a test trust root, so SQ-4's mechanism becomes mandatory. SQ-2
signs, so it is.

## Recommendation

Adopt all four mechanisms above and edit the blocked plan's four marked sections
to specify them. Specifically:

- Phase 3 §4: replace the guessed loader-glob probe with the two-signal
  mechanism, and add the third downgrade reason for a present-but-relocated
  glibc loader.
- Phase 1 Step 1b §2: sign the attestation; restate the seal as a consistency
  check rather than a trust discriminator.
- Phase 1 Step 1b §2 and Step 1c: replace the pid-and-start-time reaper gate
  with the shared `flock` lease; keep an age backstop only for lease-less
  generations.
- Phase 2 / Testing Strategy: introduce the test trust root as a second
  `[[bin]]`, with the positive key guard in `tasks/build.py`.

## Residual Risks & Open Questions

- **Probe false positive on one host shape.** musl **and** a static `/bin/sh`
  **and** `gcompat` classifies as supported and would fetch before failing at
  `execve`. Accepted under the fail-open decision. Revisit if such a host is
  reported.
- **The x86_64 psABI interpreter path is confirmed only under emulation.**
  `/lib64/ld-linux-x86-64.so.2` was read from an emulated `linux/amd64` Debian,
  not from a real Playwright artifact. Phase 1 should assert both per-arch
  constants against the actual assembled trees; note the spelling asymmetry
  (`x86-64` for glibc, `x86_64` for musl).
- **All measurements are darwin/aarch64.** The Ed25519 figure is CPU-bound and
  unlikely to differ materially on Linux, but it was not measured there.
- **The new `[[bin]]` is built by `--all-targets`**, so it is in scope for
  pedantic clippy — the lesson `cli/vcs-adapters` records. Budget for that.
- **`skip_if_no_minisign!` makes 22 tests pass vacuously.** It returns
  `Ok(())` with only an `eprintln!` when `minisign` is off `PATH`, and unlike the
  Python side's `require()` it has no CI-failing branch. `mise.toml:35` pins the
  tool, so the risk is latent rather than live. Pre-existing and out of scope
  here; worth its own follow-up.
- **ADR amendments are raised (AC6 closed).** Both went via supersession rather
  than edit, since an accepted ADR is immutable:
  - **ADR-0061** (signed content-addressed tree generations) supersedes
    ADR-0060. The decisive contradiction was addressing: ADR-0060's Decision
    section asserts trees are "addressed by release version and digest", which
    the generation scheme does not do. Signing the attestation and the lease
    ride along as mechanisms it does not describe.
  - **ADR-0062** (browser automation's platform boundary) supersedes ADR-0057.
    **The justification first recorded here was wrong** and is corrected: the
    reason is not that the third downgrade reason "extends the vocabulary it
    owns" — ADR-0057 line 71 already says "and siblings", so a new sibling is
    anticipated, and enumerating downgrade reasons sits below an ADR's altitude.
    The real defect is that its central claim, the "glibc-only" boundary, is a
    proxy that is wrong on real hosts in both directions: NixOS is glibc and
    cannot execute the artifacts, Alpine + `gcompat` is musl and resolves the
    loader. An implementer gating on libc alone would wrongly enable the
    capability on NixOS. ADR-0062 restates the boundary as a conjunction.
  - **ADR-0063** (plugin-version-scoped artifact cache) is new rather than a
    supersession, and was split out of ADR-0061 during review: the eviction and
    placement decision had no surveyed alternatives inside 0061, so it could not
    be audited there. It decides that trees stay in the per-plugin-version root
    with eviction delegated to Claude Code's ~14-day orphan sweep, and that
    `accelerator cache prune` owns the relocated and symlinked-checkout roots the
    sweep never reaches.
  - All three were reviewed individually and are **accepted**. Review found and
    fixed five defects across them: a rejection reason contradicting its own
    Context, a lease placed inside the sealed tree where the seal and `verify`
    would both trip on it, a missing driver, a fail-open rule that did not cover
    the case its own negatives named, and an absolute guarantee about external
    reclamation that the documented grace window does not give.
- **Not checked**: `mise run` and `mise run cli:check` were not run, because no
  production code was changed; a real Playwright artifact's ELF header was not
  read; nothing was measured on Linux.

## References

- Blocked plan: `meta/plans/2026-08-11-0196-design-vendored-runtime-distribution.md`
- Sibling plan (implemented): `meta/plans/2026-08-11-0196-design-cli-migration.md`
- Validation of the sibling:
  `meta/validations/2026-08-11-0196-design-cli-migration-validation.md`
- Superseded plan and its three-pass review:
  `meta/plans/2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli.md`,
  `meta/reviews/plans/2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli-review-1.md`
- ADR-0059 (build-time assembly of vendored browser artifacts) — accepted,
  unaffected
- Raised by this spike: ADR-0061 (signed content-addressed tree generations,
  superseding ADR-0060) and ADR-0062 (browser automation's platform boundary,
  superseding ADR-0057), both `proposed`
- Superseded inputs, cited above for what they said: ADR-0057 (browser
  automation as a glibc-only capability), ADR-0060 (launcher-resolved tree
  artifacts)
- Measurement-method precedent: `meta/work/0205-close-the-warm-dispatch-measurement-method.md`
