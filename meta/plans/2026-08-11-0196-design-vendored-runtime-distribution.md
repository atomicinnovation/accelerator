---
type: plan
id: "2026-08-11-0196-design-vendored-runtime-distribution"
title: "accelerator-design: Vendored Runtime Distribution Implementation Plan"
date: "2026-08-11T21:49:36+00:00"
author: Toby Clemson
producer: create-plan
status: draft
work_item_id: "work-item:0196"
parent: "work-item:0196"
derived_from: ["codebase-research:2026-08-11-0196-design-cli-implementation-surface"]
relates_to: ["plan:2026-08-11-0196-design-cli-migration"]
supersedes: ["plan:2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli"]
tags: [rust, design, playwright, launcher, release-pipeline, tree-artifacts, distribution]
revision: "8117629cd5dc64027b0174a21ddb33c72ef0468d"
repository: "accelerator"
last_updated: "2026-08-11T21:49:36+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# accelerator-design: Vendored Runtime Distribution Implementation Plan

## Overview

Vendor the Playwright runtime so the design tooling stops depending on a system Node.js.
The launcher gains the ability to resolve directory-tree artifacts alongside the
single-file sub-binaries it already fetches; the release pipeline gains a build-time
assembly step that constructs the driver bundle and the browser from verified upstream
inputs and publishes them under the project's own signing key; and the executor swaps
onto them.

This is the second of two plans against work-item:0196. It **depends on**
`plan:2026-08-11-0196-design-cli-migration`, which creates the `accelerator-design`
sub-binary and ports `run.sh` to Rust. That plan leaves `ensure-playwright.sh`, the
lockhash namespace and the system Node prerequisite in place; this one removes them.

## Read this first: four questions to settle empirically

**This plan is not ready to implement.** It carries four questions that were answered
wrongly on paper twice, across three review passes, and that a day of throwaway code
would settle definitively. Phase 0 exists to answer them, and Phases 4, 5 and 7 should
not be scheduled until it has.

The pattern that produced this recommendation is worth stating, because it is the reason
this plan is shaped differently from its sibling. The two halves of work-item:0196 were
planned as one eight-phase document and reviewed three times. Each pass closed the
previous pass's findings and introduced new criticals *in the fix material* — 7 after
pass 1, 8 after pass 2 — and **every one landed in the three phases collected here**. The
defects were not random: they clustered in mechanisms invented to close a finding, and
several were contradictions between a paragraph and one written minutes earlier in
another section. Four of them were assertions about what the filesystem, the dynamic
loader, a JavaScript API or the build system actually does. Those are cheap to check and
expensive to guess.

See
`meta/reviews/plans/2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli-review-1.md`
for the full three-pass record.

### Phase numbering is inherited

Phases keep the numbers they carried in the superseded plan — 4, 5, 7 here; 1, 2, 3, 6 in
the sibling — so the gaps are expected. Renumbering would have meant rewriting every one
of the dozens of internal `Phase N §M` and `Step 4b` cross-references, and a single missed
reference is precisely the defect class three review passes kept finding in this material.

## Current State Analysis

Two scripts survive the sibling plan and are this plan's to remove:

| Script | Lines | Disposition |
|---|---|---|
| `inventory-design/scripts/ensure-playwright.sh` | 367 | deleted, no replacement |
| `inventory-design/scripts/test-ensure-playwright.sh` | — | deleted with it |

With them go the lockhash namespace under
`${ACCELERATOR_PLAYWRIGHT_CACHE:-$HOME/.cache/accelerator/playwright}`, the sentinel
idempotency contract, the disk floor, the node-version floor, the sweep, and
`package-lock.json`.

`scripts/test-design.sh` also outlives the sibling plan, reduced to two blocks this plan
owns: `:486-490` (the `test-ensure-playwright.sh` delegation) and `:154-155` (the
`inventory-design` `allowed-tools` `scripts/*` glob, whose rule this plan drops). Deleting
those takes `scripts/` from 16 discovered suites to 15 against a floor already at 15.

**Four resolver properties do not carry across to trees.** `fetcher.rs:147-150` buffers
the whole body and `cache::store` takes `&[u8]`; `TOTAL_TIMEOUT` is 300s *per attempt*,
sized in its own comment for "a multi-MB release binary". No archive crate exists in the
workspace (`cli/Cargo.toml:13-131`). `cache.rs:118-133` renames files only. And nothing
seals, reaps orphan temp trees, or writes an attestation.

## Desired End State

On a machine with no system Node.js, the inventory skill's Playwright path fetches a
driver bundle and a `chromium-headless-shell` tree from the project's own release host,
verifies both before extraction, seals them, and drives a headless crawl. The release
pipeline assembles both artifacts in CI from inputs verified against their publishers' own
signatures. No `.sh` file remains under `skills/design/`.

Verified by: `mise run` exits 0; `manifest.json` carries an `artifacts` map beside
`binaries`; `skills/design/` contains no `.sh` file; a container fixture with Node absent
from `PATH` completes a Playwright-driven inventory.

### Acceptance criteria in scope

Fully: **AC6**, **AC7**, **AC8**, **AC9**, **AC10**, **AC11**, **AC12**, **AC13**,
**AC14**, **AC16**.

Completing what the sibling plan starts: **AC1** (the `notices` subcommand), **AC2** (the
bundled path's envelopes), **AC3** (the bootstrap step's call sites), **AC4**
(`ensure-playwright.sh` and the last floor movement).

### Key Discoveries

- The manifest extension is additive by construction — `manifest.rs:1-3` states "Unknown
  additive fields are ignored", the schema has no top-level `additionalProperties: false`,
  and `manifest.rs:223-231` is a dedicated test feeding `"future_field": 42`. The gate
  rejects only strictly-greater versions (`manifest.rs:82-89`). No `SCHEMA_VERSION` bump,
  and no flag day.
- `cache::find`'s prefix scan (`cache.rs:51-73`) will *see* a directory in the same root
  and rejects it only because no `.minisig` sidecar exists. Tree entries need a distinct
  subdirectory, and a tree's sidecars must never be named `*.minisig`. `cache.rs:56` also
  aborts the whole scan on one non-UTF-8 entry, so new on-disk names stay ASCII.
- `cache.rs:1-6` records that "the checksum in the name lets a hit resolve offline". The
  single-file warm path never loads the manifest, and `load_manifest`
  (`resolve/mod.rs:116-135`) is two HTTPS GETs plus a signature verification, called only
  on a miss. A tree hit must hold the same property: each executor invocation is a fresh
  launcher process and a crawl makes 100–200 of them.
- `@actions/glob`'s `*` does not cross `/`, so tree archives must stay flat in
  `dist/release/` for `dist/release/accelerator-*` to keep matching.
- The launcher's redirect allowlist is `github.com` plus `*.githubusercontent.com` only
  (`fetcher.rs:17-18,31-33`).
- `tasks/lint/cli.py:7` and `tasks/test/cli.py:13` both pass `--all-features`, the latter
  deliberately to enable `bash-parity`. **Any non-default cargo feature added to a `cli/`
  crate is therefore on during `mise run cli:check` and `mise run test:unit:cli`** — which
  is why Phase 0 Q4 exists.
- `bin/accelerator:239-251` already reads `ACCELERATOR_LAUNCHER_BIN` as a *dev-override
  input*, gated on `ACCELERATOR_ALLOW_UNVERIFIED_LAUNCHER=1` and refused unless the path
  is inside `${plugin_root}/cli/target/`. It is not a free name to export.
- `tasks/git.py:50-52` runs a bare `git push --atomic` with no credential helper, so the
  release job depends on `actions/checkout` persisting its token. `persist-credentials:
  false` cannot be added without an explicit replacement.

## What We're NOT Doing

- Anything the sibling plan owns: the `design` sub-binary, the five ported subcommands,
  the `run.sh` port, the metadata-script retirement, the registration checklist.
- Shipping full Chromium. `chromium-headless-shell` is 177MB across 14 files against 297MB
  across 327, and the daemon launches headless (`lib/daemon.js:106`).
- Bundling `ffmpeg`. `browsers.json` marks it install-by-default but it serves video
  recording.
- A musl driver bundle. Playwright publishes none, and its Chromium builds are
  glibc-linked.
- Cross-plugin-version artifact sharing beyond what content addressing gives for free, or
  bespoke cache eviction beyond a `prune` verb.
- A formal legal review gate on the release.
- Automated removal of the abandoned legacy Playwright cache on user machines.

---

## Phase 0: Settle the four empirical questions

### Overview

A time-boxed spike, not an implementation phase. Each question below has been answered
wrongly at least once in a review pass, each answer changes the shape of the code that
follows, and each is cheap to check against the real system. Record the answers on
work-item:0196 and edit the affected sections of this plan before scheduling Phase 4.

Use `/accelerator:conduct-spike` — this is exactly its shape: throwaway prototypes
answering empirical questions that block planning.

### Q1 — What actually identifies the host's libc, and what does the answer say on macOS?

An earlier draft classified a `LibcFlavour` from the set of loader paths present, globbing
`/lib/ld-musl-*.so.1` and `/lib64/ld-linux-*.so.2`, with "neither present" meaning
`unsupported-platform`. **On macOS neither exists**, so that logic downgrades every Mac —
a supported platform per ADR-0057 and the primary development platform — before it touches
a tree. NixOS places its glibc loader in the Nix store rather than `/lib64`, and Debian
with `musl-tools` or Alpine with `gcompat` has both.

Settle: does the probe short-circuit on non-Linux targets at compile time? Is there a
positive glibc signal that does not depend on a filesystem convention (reading the ELF
interpreter of `/proc/self/exe` is the obvious candidate)? Which way does an ambiguous host
fail — and is failing *open* (attempt the glibc runtime, let `execve` fail) better than
failing closed for a capability that already has a graceful downgrade?

**Deliverable**: a probe that returns the right answer on macOS, Debian, Alpine and one
non-standard-layout Linux, with the classification a pure function over an injected
listing.

### Q2 — Can a tree hit be bound to the release key without a manifest fetch?

The warm path must be local and offline (a crawl makes 100–200 invocations), which is why
the design avoids `load_manifest` on a hit. But that leaves the hit path with **no
cryptographic check at all**: an attestation whose digest matches the digest in its own
directory name is self-referential, and the cache root is `ACCELERATOR_CACHE_DIR`, which
this plan recommends relocating and which per-project config can set.

Settle: does storing the manifest's minisign signature over the archive digest *inside*
the attestation, and verifying it in `locate`, cost what it looks like it costs (one
Ed25519 verify over ~100 bytes)? Measure it on the hit path against the ~30ms warm
bootstrap budget. If it is free, the hit path gains a real trust anchor and the
`ACCELERATOR_CACHE_DIR` hazard mostly closes. Also settle whether requiring the seal
itself (`0444`/`0555`) as an acceptance condition is a cheap additional discriminator — a
git checkout or an unzip cannot produce read-only files.

**Deliverable**: a measured figure for signature verification on the hit path, and a
decision on whether the attestation is signed.

### Q3 — What can actually be known about who holds a materialised tree?

The reaper and `prune` were specified to gate on "the owning pid and its start time" plus
"a skip for any generation a live process still holds". Neither has a data source: temp
names carry only a generation, nothing records a pid after the publish rename, and there is
no portable way to ask which process holds a directory. Yet that gate is what makes
`repair` safe against a live daemon — the whole reason the generation design exists.

Settle: what does a lease look like? An `flock`-held file inside the generation, written by
the executor and observable by the launcher, is the obvious shape — prototype it and check
that the lock survives the daemon's lifetime, is visible cross-process, and is released on
kill. Then decide whether `prune` needs it at all, or whether a minimum retention window
(keep the previous generation until the next successful materialisation plus a grace
period) is sufficient and simpler.

**Deliverable**: a working in-use signal, or a reasoned decision that retention windows
replace it.

### Q4 — How is a test trust root introduced without `--all-features` turning it on?

The container fixtures for AC6 and AC12 must verify artifacts they signed themselves, but
`cli/launcher/build.rs:32` embeds `keys/accelerator-release.pub` unconditionally with no
override. A non-default cargo feature was proposed — and `tasks/lint/cli.py:7` and
`tasks/test/cli.py:13` both pass `--all-features`, deliberately, so the feature would be
**on** during `mise run cli:check`.

Settle: is a build-time env var read by `build.rs` with no feature flag the right shape
(unreachable by `--all-features`, but then what stops a stray variable in a release
build)? Or should the fixtures verify against the *real* key by having the spike produce a
signed synthetic artifact once? Or should the trust root be *substituted* rather than
widened, so a leaked build fails closed and loudly instead of silently trusting an extra
key forever?

**Deliverable**: a mechanism plus a positive guard — an assertion that a shipped launcher
embeds exactly the committed production key, rather than a negative marker-string scan.

### Success Criteria

#### Automated Verification

- [ ] Nothing. This phase produces throwaway code and recorded answers, not shipped
      behaviour.

#### Manual Verification

- [ ] Each of Q1–Q4 has a recorded answer on work-item:0196, with the prototype that
      produced it referenced
- [ ] The affected sections of this plan are edited to match, and the sections that were
      written against a wrong answer are corrected rather than annotated
- [ ] Any answer that changes an accepted ADR (most likely ADR-0060, on tree addressing
      and adoption) has its amendment raised via `/accelerator:review-adr`

---

## Phase 4: Launcher tree artifacts

### Overview

Teach the resolver to fetch, verify, extract and seal directory-tree artifacts,
and add the repair path that replaces the self-healing trees are exempt from.
Tested against a synthetic tarball — no design consumer exists yet.

Three internally staged steps, each of which should compile and test green on
its own.

### Step 4a: Manifest `artifacts` map and streaming fetch

#### 1. Manifest shape

**File**: `cli/launcher/src/launch/outbound/resolve/manifest.rs`
**Changes**: A new `artifacts: BTreeMap<String, ArtifactEntry>` beside
`binaries`, `#[serde(default)]`. `SUPPORTED_SCHEMA_VERSION` stays `1`. The
all-zeros sentinel digest carries over for platforms where an artifact is
deliberately absent, reusing `bare_sha256`'s existing handling.

```rust
#[derive(Debug, Deserialize)]
pub struct ArtifactEntry {
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub platforms: BTreeMap<String, ArtifactPlatformEntry>,
}

#[derive(Debug, Deserialize)]
pub struct ArtifactPlatformEntry {
    pub sha256: String,
    pub signature: String,
    pub archive_size: u64,
    pub uncompressed_size: u64,
    pub entry_count: u64,
}
```

A tree needs three sizes that a single-file binary does not, and they are three
different magnitudes: `archive_size` bounds the download (~120MB compressed),
`uncompressed_size` and `entry_count` bound the extraction (~294MB across hundreds of
files), and the free-space precheck needs `archive_size + uncompressed_size` because
both exist on disk at once. One field serving all three would be wrong by 2–3× for at
least one consumer whichever quantity it held.

So `binaries` keeps `PlatformEntry` genuinely untouched, and artifacts get their own
entry type. The three sizes are **required, not `#[serde(default)]`** — a defaulted 0
would silently disable the download cap and the decompression-bomb ceiling, which is
the failure mode a default exists to avoid. Additivity is unaffected: an older
launcher never reads `artifacts` at all, and a newer one reading a manifest without
the key gets an empty map.

The asset-name convention is `accelerator-{key}-{platform}.tar.gz`, mirroring the
single-file `accelerator-{token}-{platform}` rule pinned in one commented place at
`resolve/mod.rs:144-147`. Phase 4 builds the consumer and Phase 5 the producer, in
separate changes, so the convention is pinned in one artefact both sides read:
`tests/fixtures/manifest.example.json` gains an `artifacts` block in this phase,
asserted from `manifest.rs`'s golden test here and from
`tests/unit/tasks/test_manifest_contract.py` in Phase 5. Without that, a key-name
or extension disagreement surfaces only in Phase 7's container fixture, after both
halves have merged.

#### 2. Streaming download

**File**: `cli/launcher/src/launch/outbound/resolve/fetcher.rs`
**Changes**: `try_get` currently ends in `response.bytes().map(|body| body.to_vec())`
(`:147-150`) — the body buffered, transiently twice. Add a
`get_to_writer(&self, url: &str, sink: &mut impl Write)` that copies from the
response reader, leaving `get` as a thin wrapper for the existing small-asset
callers.

**The sink must be owned inside the retry loop.** `get` retries up to
`MAX_ATTEMPTS` (3), and today each attempt is safe only because `try_get` returns a
fresh `Vec<u8>` — a failed attempt leaves nothing behind. Writing into a
caller-provided sink breaks that invariant: an attempt that fails partway has
already written bytes, and the next appends the full body after them. The sha256
would catch the result, so nothing unverified is extracted, but the retry loop could
never succeed — a transient blip on a 294MB transfer would become a permanent,
unrecoverable failure presenting as a checksum mismatch. So the streaming path
creates and truncates the temp file at the start of *each* attempt (or `set_len(0)`
plus seek to 0) and resets the incremental digest state with it.

**The deadline is a throughput floor, not a number picked once.** `TOTAL_TIMEOUT`'s
300s per attempt was sized for a multi-MB binary. It governs the *compressed
archive*, whereas the ~294MB figure is the uncompressed tree — so the value is
derived from Phase 5's measured archive sizes, expressed as "sized for X MB at ≥N
KB/s sustained", and recorded in the constant's doc comment with its reasoning as
the existing one does. Make it a per-request override via
`RequestBuilder::timeout()` rather than a second `Fetcher`: each `Fetcher` builds a
`reqwest::blocking::Client` (installing the rustls provider and a background runtime
thread), and `FetchVerifyCacheResolver::new` already constructs one on *every*
invocation including warm hits — so it is also constructed lazily, and a warm
resolution builds none at all.

**A stalled transfer must fail fast.** `fetcher.rs:12-14` records that blocking
reqwest has no idle timeout and that the total deadline is "the only bound on a
slow-but-progressing transfer". Enlarging that deadline widens the window in which a
connection stalled at byte one is indistinguishable from a slow one — three times
over, inside a tool call with no progress output and no cancel. The copy loop
therefore enforces a progress floor (abort if fewer than N bytes arrive in M seconds),
so the large deadline bounds legitimate slow transfers while stalls fail quickly. Both
numbers go in the doc comment.

The **mechanism** must be named, because a plain byte-counting check between reads
cannot fire while a read is blocked — which is the stall case. Blocking reqwest exposes
neither an idle timeout nor the socket, so the floor needs either a watchdog thread
that drops the response to interrupt the blocked read, or a custom `Read` wrapper over
a socket with `SO_RCVTIMEO` set. State which. And the test fixture must **stop
sending** rather than trickle, or it exercises the slow path and passes without ever
testing a stall.

**Signature verification needs a named streaming mechanism.** sha256 streams
trivially, but `TrustedKeys::verifies(&self, data: &[u8], signature: &str)`
(`keys.rs:62`) is a contiguous-slice API, and incremental Ed25519 verification is
only possible in minisign's *prehashed* mode. `tasks/signing.py:24-43` signs with a
plain `minisign -S` and no `-H`, so the form must be established before this step
starts rather than assumed: confirm what `minisign -S` emits, and if it is not
prehashed either add `-H` for tree artifacts — checking the vendored `cli/verify`
shim and `minisign-verify` both accept it — or state plainly that the archive is
buffered for verification and bound the peak. Left unstated, an implementer reads a
294MB temp file back into a `Vec<u8>`, giving the launcher a peak RSS an order of
magnitude above anything it does today, in exactly the memory-limited containers
AC6 and AC11 use. A test asserts the release pipeline's signatures are in the
expected form, so a signing-flag change fails loudly rather than degrading to a
full buffer.

The download is capped at `archive_size` from the artifact's platform entry;
`uncompressed_size` and `entry_count` bound the extraction in Step 4b step 4.

### Step 4b: Extraction, sealing, atomic rename, attestation, pointer

#### 1. Archive dependency

**Files**: `cli/Cargo.toml`, `cli/launcher/Cargo.toml`, `cli/deny.toml`,
`tests/integration/deny/test_launcher_feature_graph.py`
**Changes**: Add `tar` and `flate2` as workspace-pinned dependencies with
justification comments. `tar` is pinned **exactly**, not caret-bound: it is pre-1.0,
and its entry classification, PAX/GNU long-name handling and symlink semantics are
precisely what the extraction allowlist sits on top of, so a patch bump could shift
the trust boundary without a pin-edit review. `cli/Cargo.toml`'s stated discipline is
to exact-pin crates whose behaviour the workspace depends on (`clap`, `reqwest`,
`rustls`, `minisign-verify`, `serde-saphyr`, `jj-lib`) and caret-bound only those
documented as behaviour-stable. It also gets `default-features = false`, since the
default `xattr` feature adds a transitive edge that mode masking makes pointless.

`flate2` is pinned explicitly to its pure-Rust backend:

```toml
flate2 = { version = "1", default-features = false, features = ["rust_backend"] }
```

That pin is load-bearing, not stylistic. `flate2`'s alternative backends (`zlib`,
`zlib-ng`, `zlib-rs`) pull `libz-sys`/`zlib-ng-sys`, which need a C toolchain and
would break the fully-static musl cross-build ADR-0046 depends on. Because Cargo
unifies features across the workspace, a *future* crate enabling a C backend would
pull it into the launcher silently — so `libz-sys`, `zlib-ng-sys` and `zlib-sys`
join `_ABSENT` in `tests/integration/deny/test_launcher_feature_graph.py:24-31`,
which already parametrises `test_banned_or_native_crate_is_absent` over that tuple
for exactly this class of regression.

`cli/Cargo.toml:149-151` documents launcher binary size as a per-invocation latency
term, because `bin/accelerator:352-354` minisign-verifies the whole launcher on
every warm start. "Reconsider if it exceeds a few hundred KB" is not a gate — it has
no number and no outcome — and it also weights the wrong axis. Work-item:0186
measured shim exec plus verify of a 7.6MB launcher at ~6.8ms, with minisign alone at
~2.3ms for 8MB: roughly **0.3ms/MB**, so a few hundred KB is ~0.1ms, plausibly below
the measurement noise floor. So the budget is expressed in time and converted to a
ceiling — but the slope is **measured, not back-derived**. The 2.3ms figure comes from
0186's pre-change Context table, a 20-run bash loop that 0186's own Validation Results
declare "not method-comparable" to its interleaved medians; and the post-change
composition table's ~6.8ms bundles shim process startup with the read, so attributing
all of it to size gives 0.3ms/MB while attributing half gives ~0.45ms/MB and a ceiling
nearer 2MB. Deriving a marginal per-MB cost from one point that includes fixed costs is
not sound. So the slope is obtained directly, with 0186's method: verify two padded
launchers of known differing size on the same host and take the difference. The 1ms
budget then converts to a real ceiling.

Two notes on that gate. `tar` plus `flate2`/`miniz_oxide` realistically add a few
hundred KB, so a multi-MB ceiling is a weak tripwire — the assertion is on the measured
delta plus a small margin, not on the headroom. And the ceiling is an **absolute
per-target size** checked against the cross-compiled artefacts in the release lane,
recorded beside the other pins in `tasks/shared/paths.py` with its derivation in the
comment, because a ratio gate would need a stored pre-Phase-4 baseline that
`mise run test:*` has nowhere to keep and cross-compiled binaries that only exist after
`build.cli_cross_compile`.

The backend's real consequence is the other direction: `miniz_oxide` (the pure-Rust
default) inflates materially slower than a zlib-ng build, and the cold path inflates
~294MB. So record decompression throughput over a real archive alongside the size
figure, and if `rust_backend` proves unacceptably slow the resolution is a faster
pure-Rust backend (`zlib-rs`, if it can be shown to need no C toolchain), never a
`*-sys` crate.

#### 2. Tree materialisation

**File**: `cli/launcher/src/launch/outbound/resolve/tree.rs` (new)
**Changes**:

Layout — a dedicated subdirectory so `cache::find`'s prefix scan
(`cache.rs:51-73`) never sees a tree, content-addressed so an unchanged artifact is
one tree however many plugin versions want it, per-platform so a shared cache root
cannot mix incompatible trees, and generation-suffixed so a rename target is always
fresh:

```
<cache_root>/trees/<name>-<platform>-<sha256>-<gen>/        the sealed tree
<cache_root>/trees/<name>-<platform>-<sha256>-<gen>.sealed  the attestation
<cache_root>/trees/<name>-<platform>-<sha256>-<gen>.files   the per-entry table
<cache_root>/trees/<name>-<platform>-<version>.ref          the pointer
```

All names are ASCII (`cache.rs:56` aborts the scan on one non-UTF-8 entry) and none
is named `*.minisig`. The **attestation** is small and fixed-size — archive digest,
platform, release version, entry count, and a digest of the table — and is the only
sidecar the hit path opens. The **table** carries one
`(path, mode, size, sha256)` row per entry (or a link target, for a symlink) and is
read only by `verify` and `repair`; keeping it out of the attestation is what stops
the hit path's cost scaling with the driver tree's ~490 files. The **pointer** names
a directory rather than a digest, which is what lets a repair swap one generation for
another atomically.

The attestation and the pointer each carry a `format_version`, and the tree directory
name carries a layout version alongside the generation. Extraction and sealing policy
— the entry-type allowlist, mode masking, the `0444`/`0555` seal, the table's own shape
— is launcher-version-specific and is *not* covered by the archive digest, yet content
addressing means a newer launcher routinely adopts an older launcher's tree from a
shared cache root. Without a layout version a policy fix would be silently inherited
rather than applied, and `verify` would pass because it checks against the older
table. The same "unknown additive fields ignored, higher version refused" discipline
`manifest.rs` already documents applies, and the ADR-0060 amendment records that
cross-version tree *adoption* — not just digest addressing — is the real deviation.

The generation is the load-bearing addition. Because every materialisation picks a
fresh one, `rename(2)` never lands on an existing target — so there is no
already-present branch to get right, no need to distinguish a concurrent winner from
a crash leftover at rename time, and a repair can build a complete replacement
beside a tree a live daemon is still reading.

`trees/` is created `0700`. The cache root, every generation directory and every
sidecar must be owned by the effective uid and be neither group- nor
world-writable; anything failing that is treated as absent rather than trusted.
ADR-0060's threat model assumes the cache lives under the user's own home directory,
and `ACCELERATOR_CACHE_DIR` — which this plan actively recommends — can break that
assumption, so it is enforced rather than assumed and documented as requiring a
private, user-owned path.

> **⚠ Blocked on Phase 0 Q2.** The four checks below are all *local and self-referential* —
> an attestation whose digest matches the digest in its own directory name proves nothing
> about provenance, and nothing on this path is bound to the release key. That is acceptable
> only if ADR-0060's premise holds that the cache lives under the user's own home
> directory, and this plan actively recommends relocating it via `ACCELERATOR_CACHE_DIR`,
> which per-project config can set. Q2 measures whether verifying a stored minisign
> signature over the archive digest costs anything meaningful on this path (one Ed25519
> verify over ~100 bytes); if it does not, the attestation is signed and this list gains a
> real trust anchor.

**`locate`** (the hit path, on every dispatch):

1. Read `<name>-<platform>-<version>.ref`. Absent or unparseable → miss.
2. Reject the name unless it matches `<name>-<platform>-<64 lowercase hex>-<gen>`
   exactly and resolves to a direct child of `trees/`. The pointer is unsigned local
   state whose contents become a path, so it is validated before it is joined.
3. `stat` the directory: present, a directory, correctly owned, not
   group/world-writable. Otherwise miss — a tree removed by a partial `rm -rf` or an
   interrupted prune leaves its tiny sidecars behind, and returning a dead path would
   surface as an opaque Node error instead of a re-materialisation.
4. Read the attestation; its digest must equal the digest in the directory name.

Two small reads and two stats. No network, no manifest, and the table untouched.

**`materialise`** (the cold path — reached only from `cache ensure` and `repair`),
under the per-`(name, platform)` single-flight lock:

1. Load the manifest; the entry names digest `D`, `archive_size` and
   `uncompressed_size`.
2. **Reuse scan**: any `trees/<name>-<platform>-D-*` whose attestation is valid and
   whose directory passes the step-3 checks → publish the pointer at it and return,
   **with no download**. This is what makes an unchanged artifact across two plugin
   versions a genuine hit rather than a refetch, and it is asserted by a zero-fetch
   criterion rather than by a directory-layout one.
3. Free-space precheck against `archive_size + uncompressed_size` plus a margin, for
   every tree about to be materialised. A shortfall emits `disk-floor-not-met` before
   a single byte is fetched.
4. Stream the archive to `trees/.tmp-<gen>.archive`, truncating the file and
   resetting the incremental digest at the start of each attempt, under Step 4a's
   deadline and progress floor.
5. Verify sha256 and minisign over the archive. On failure, remove the temp archive
   and return the cause — nothing has been extracted.
6. Extract into `trees/.tmp-<gen>/` under the entry rules in step 4 below,
   **computing each entry's sha256 inline as it is written**, so the table costs no
   second pass over ~294MB.
7. Seal bottom-up: `0444` for files, `0555` for files the archive marks executable,
   directories left owner-writable. Symlinks are walked with `symlink_metadata` and
   their permissions left alone — `set_permissions` follows a link and would re-mode
   the target — and recorded in the table by link target rather than by digest.
8. Write `.tmp-<gen>.files`, then `.tmp-<gen>.sealed` carrying its digest.
9. `rename(2)` the temp directory into place, then the two sidecars. Fresh by
   construction, so no collision case arises.
10. Publish the pointer atomically, last. Until then the generation is invisible to
    `locate` and reclaimable by the reaper, so a crash at any earlier step leaves
    only garbage rather than a half-trusted tree.

**Single-flight**: one lock directory per `(name, platform)` under `trees/`, reusing
the PID-owner staleness discipline `bin/accelerator:317-345` implements — but not its
waiter budget, which resets on every live-owner observation and so waits unbounded.
Here the wait carries an explicit deadline derived from the fetch deadline plus an
extraction allowance, and the loser waits on the **lock**, never on the pointer: a
winner that fails writes no pointer, so a pointer-waiter would hang forever. On
acquiring the lock the loser re-runs `locate` and materialises only if still needed;
on deadline expiry it emits `artifact-unavailable` rather than blocking a crawl. The
lock is released by a `Drop` guard on every path.

Without this, two cold invocations each stream ~294MB, hash it, verify it, extract it
and seal it — ~588MB of transfer and ~1.2GB of transient disk, one copy of which is
then discarded. `cache::store` needs no such guard at ~8MB; at this size the
duplication is the dominant cost of a first run.

> **⚠ Blocked on Phase 0 Q3.** The gate below — "the owning pid **and** its start time"
> plus a skip for any generation a live process holds — has **no data source**. Temp names
> carry only a generation, nothing records a pid after the publish rename, and there is no
> portable way to ask which process holds a directory. That gate is what makes `repair` safe
> against a live daemon, so it is load-bearing rather than defensive. Q3 produces a working
> in-use signal (a lease file whose lock the launcher can observe is the leading shape) or
> concludes that a minimum retention window replaces it.

Orphan reaping: `cache.rs:130` removes a single temp file on a failed rename. Here
the residues are larger and more varied — a partial temp archive, a partial temp
tree, and a fully-materialised generation no pointer references (left by a crash
between steps 9 and 10, or superseded by a repair). `reap_orphans` reclaims all
three. Liveness is gated on the owning pid **and its start time** — the executor
already needs that probe, and a bare pid check would spare an orphan forever once the
pid recycled — with an age backstop beyond the fetch-plus-extract deadline so nothing
leaks permanently, and a skip for any generation a live process still holds. It runs
from `materialise` and from `cache prune`, never from `locate`, which stays a query
with no side effects.

#### 3. Documented divergence

**Files**: `cli/launcher/src/launch/outbound/resolve/mod.rs`,
`cli/launcher/src/launch/core.rs`, `cli/pup.ron`
**Changes**: ADR-0060 calls the two integrity models "a documented difference
rather than an oversight", which means it must actually be documented in
`resolve/`. Extend the module doc comment to state both models and which applies
where.

Trees are **not** routed through `ResolveBinary::resolve` (`mod.rs:180-233`) —
that method's per-exec re-verify is precisely what they are exempt from, and its
contract is name → executable path for `exec`. But refusing that port leaves the
second artifact class with no port at all: `resolve/tree.rs` would be an *outbound
adapter* module called directly from `main.rs` and from the `cache` built-in, while
`launch::core` holds both existing driven ports (`ResolveBinary`, `ExecBinary`), the
error taxonomy and the `run_external` use case — and `cli/pup.ron` pins
`accelerator::launch::core` to std/kernel/self imports. The launcher's one enforced
architectural rule would then cover one of its two resolution paths.

So `launch::core` declares **three narrow ports**, not two broad ones:

- `LocateSealedTree` — pure lookup, no network, returns `Option<TreePath>`. This is
  the only one the dispatch path may call.
- `MaterialiseTree` — network plus filesystem, called only by `ensure` and `repair`.
- `VerifyTree` — a read-only walk returning a per-entry discrepancy report.

A single `ResolveArtifactTree` meaning "find-or-materialise" would put the forbidden
behaviour one argument away from the warm path, when the whole design rests on
dispatch never fetching; and a `VerifyArtifactTree` meaning "walk, and repair" would
put a query and a destructive mutation behind one abstraction, blunting the very seam
the ports exist to provide. With the split, `repair = verify → materialise → repoint →
reap` is a **use case in `launch::core`** over the three ports, mirroring how
`run_external` sits over `ResolveBinary` + `ExecBinary` — so the interesting decision
(what to do when verification fails) sits in front of the adapter rather than inside
it.

Tree-specific `ResolutionError` variants — extraction, path-escape, seal, attestation,
pointer — replace folding everything into `Cache { path, detail }`. Each states its
`Refusal`/`Failed` mapping explicitly, because `swallow_under_fail_safe`
(`launch/core.rs:218-224`) swallows only `Failed`, so the choice silently decides
whether a crawl degrades or hard-fails under `--fail-safe`. Since the pup rule pins
`launch::core` to std, `kernel::Error` and self, the discrepancy report and the
attestation are plain core-owned types with serde living in the adapter.

Which variant each maps to is not cosmetic: `swallow_under_fail_safe`
(`launch/core.rs:218-224`) swallows only `Failed`, so the choice silently decides
whether a crawl degrades or hard-fails under `--fail-safe`. Every new variant states
its mapping explicitly.

`tree.rs` is split along its natural seams — layout and attestation, verified download,
safe extraction, sealing — rather than being one module owning seven
responsibilities, because `cache repair` needs several of them independently.
Following `cache.rs`'s convention, the sealing and permission helpers carry
`#[cfg(not(unix))]` no-op arms so the launcher still type-checks off Unix, or the
module doc states that tree materialisation is Unix-only by design; Windows is
outside ADR-0057's matrix either way, so this is about keeping the neighbouring
module's discipline rather than about supporting Windows.

#### 4. A test trust root, so the container fixtures can verify anything

**Files**: `cli/launcher/build.rs`, `cli/launcher/Cargo.toml`,
`cli/launcher/src/launch/outbound/resolve/keys.rs`, `keys/accelerator-test.pub`
(new), `tasks/build.py`
**Changes**: AC6 and AC12 rest on container fixtures that build artifacts in the same
run and serve them from a `MockServer` — but nothing can sign those artifacts.
`build.rs:28-45` copies `keys/accelerator-release.pub` into `OUT_DIR`
unconditionally and `keys.rs:12` `include_str!`s it, with no env override, no feature
and no path indirection, so a compiled launcher accepts only artifacts signed with
the real release secret, which no test can hold. `ACCELERATOR_RELEASE_BASE_URL`
answers *where* the manifest comes from, never *who signed it*.
`cli/launcher/tests/resolution.rs` sidesteps this by constructing `TrustedKeys`
in-process, which a container running the real binary cannot do.

> **⚠ Blocked on Phase 0 Q4.** The mechanism below — a non-default `test-trust-root` cargo
> feature — **does not work in this repository**. `tasks/lint/cli.py:7` and
> `tasks/test/cli.py:13` both pass `--all-features`, the latter deliberately to enable
> `bash-parity`, so the feature would be on during `mise run cli:check` and
> `mise run test:unit:cli`: either `build.rs` fails on the unset key path (making this
> phase's own "`mise run` exits 0" criterion unsatisfiable) or every launcher in
> `cli/target/` silently trusts the test key, reachable through the documented dev override.
> Guard 1 below is therefore void, and guards 2 and 3 are negative marker scans that do not
> cover the per-run key the container task actually signs with. Q4 produces a mechanism plus
> a **positive** guard — an assertion that a shipped launcher embeds exactly the committed
> production key — and decides whether the trust root is substituted rather than widened, so
> a leaked build fails closed and loudly instead of trusting an extra key forever.

`keys.rs` already documents itself as "verify-any-of over a small key set, so
rotation has an overlap window", so the seam exists. A non-default
`test-trust-root` cargo feature makes `build.rs` embed a **second** key from
`ACCELERATOR_TEST_PUBLIC_KEY_FILE` alongside the production one, with
`rerun-if-env-changed` on both and a `cargo:warning` when it fires. The production
key is always embedded, so the feature widens the trusted set for a test build rather
than substituting it, and the production verification path is still the one under
test.

Three guards keep it out of a release, because a feature that weakens the trust root
is only acceptable if it cannot ship:

1. The feature is absent from `[features] default`, and a build-system test asserts
   `build.cli_cross_compile` passes no `--features` flag.
2. `keys/accelerator-test.pub`'s minisign comment line is the fixed marker
   `ACCELERATOR TEST KEY — NEVER SHIP`. Because an embedded key is a string constant
   in the binary, the release pipeline asserts that byte sequence appears in **none**
   of the four cross-compiled launchers — a mechanical check, not a convention.
3. The same assertion runs over the committed `bin/` shims, so a locally-built
   launcher with the feature on cannot be committed either.

The container task builds its launcher with `--features test-trust-root` and signs
its synthetic artifacts with the matching throwaway secret, which is generated per
run and never committed.

### Step 4c: `accelerator cache` built-in

#### 1. Command surface

**Files**: `cli/launcher/src/launch/inbound/cli.rs`,
`cli/launcher/src/launch/core.rs`, `cli/launcher/src/main.rs`,
`tasks/shared/dispatch_coherence.py`
**Changes**:

```
accelerator cache verify [<name>]   walk sealed trees against their file tables
accelerator cache repair [<name>]   re-materialise any tree that fails verify
accelerator cache ensure <name>     materialise a tree if it is not already
accelerator cache prune             reclaim unreferenced generations and orphans
```

`verify` walks each pointed-at generation against its `.files` table using
`symlink_metadata`, and **hashes every regular file**. There is deliberately no
stat-and-escalate shortcut: a substitution that preserves size and mode is exactly
the case the table exists to catch, and an escalation predicate keyed on size or mode
never fires for it — the digests would never be read on the only path that reads
them. ADR-0060 measures a full hash of the whole set at roughly 120ms on the
reference host, which is affordable on a command a user runs deliberately and never
runs on the hit path. The stat pass survives only as a cheap pre-check for missing
and unexpected entries. `verify` reports per-entry discrepancies — missing, extra,
size, mode, digest, link target — rather than a bare pass/fail, so the output
diagnoses as well as detects.

`verify` is **offline by construction**: `<name>` is validated against a compiled-in
artifact-name set (the Rust mirror of `TREE_ARTIFACTS`, held to it by a drift test),
not against the manifest. Validating against the manifest would make a diagnostic
that inspects local disk require two HTTPS GETs and a signature verification, so it
would be unavailable exactly when a user reaches for it — offline, air-gapped, or
with the release host down. Default-deny still holds, and no path is ever constructed
from an unrecognised token.

`repair` verifies, then **materialises a new generation** for each failing artifact
and swaps the pointer to it. Because generations are distinct directories, the
replacement is built alongside the tree in use: a live daemon keeps every inode it has
already opened *and* every file it opens later — locale packs, `.pak` resources,
`icudtl.dat`, lazily-`require`d modules — which the single-file `exec` inode argument
does not cover. Nothing is unlinked before a verified replacement exists, so a repair
whose refetch fails leaves the working tree exactly as it was rather than destroying
the only copy. The superseded generation is left for `prune`.

`repair --force` skips verification and re-materialises unconditionally. It is the
only recovery for a tree that is internally consistent but *wrong* — assembled for the
wrong architecture, or missing a component — which `verify` cannot detect by
construction, since such a tree matches its own table perfectly. Without it, a user
following the remediation string in a failure envelope gets a successful no-op and no
diagnosis.

`ensure` is the cold-path entry point `accelerator-design` calls when the launcher
exported no path for a tree it needs. It materialises and prints the resolved path, or
fails with a structured cause the caller maps to a downgrade reason. It exists so the
launcher never has to know which design subcommands need a runtime (see Phase 7).

`prune` reclaims every generation no pointer references and no live process holds,
plus orphan temps. It is what bounds growth for anyone who takes the documented
`ACCELERATOR_CACHE_DIR` escape, since that location sits outside the plugin tree and
so outside the only eviction this design otherwise has: content addressing means an
unchanged artifact is reused rather than duplicated, but each pin bump still
materialises a fresh tree and nothing else would ever remove the old one.

`<name>` is validated against the compiled-in artifact set for every verb, and the
canonicalised target must be a direct child of `trees/` before any removal.

`BUILTIN_SUBCOMMANDS` (`dispatch_coherence.py:41`) gains `"cache"` (registration
point 10). A test pins that set against the clap `Command` enum, so the two
cannot drift. `cache` becomes permanently unavailable as a dispatch token.

### Success Criteria

#### Automated Verification

- [ ] Failing tests first. The signature and end-to-end resolution cases follow
      `tests/resolution.rs:41-199` with its `MockServer` and real keypair — but the
      extraction, sealing, attestation, pointer and reaper tests exercise
      `resolve/tree.rs` **directly with no signing step**, so they cannot inherit that
      file's `skip_if_no_minisign!` guard (`:189-199`), which returns `Ok(())` with
      only an `eprintln!` and would report green on any machine without `minisign` on
      `PATH`
- [ ] A corrupt archive is rejected **before** anything is extracted — the test
      asserts the trees directory is empty after the failure
- [ ] A tarball is rejected for each of: a `../` entry, an escaping symlink, a
      hardlink whose target escapes, an absolute path, a symlink-then-traverse
      chain, a FIFO or device entry, a tree exceeding `uncompressed_size`, and an
      entry count over `entry_count`
- [ ] A setuid archive member is materialised without its setuid bit, and an
      archive member marked executable keeps only its executable bit
- [ ] A streaming fetch whose first attempt fails after N bytes succeeds on retry,
      rather than producing a concatenated archive that can never verify
- [ ] A stalled transfer (no bytes for longer than the progress floor) fails fast
      rather than waiting out the full deadline three times
- [ ] Exactly **one** archive fetch occurs when two cold resolutions of the same
      tree race, asserted against the `MockServer`'s request count
- [ ] The launcher binary size delta is within the stated per-target ceiling
      (~3.3MB, derived from work-item:0186's ~0.3ms/MB verify rate and a 1ms
      budget), asserted per target rather than recorded
- [ ] A second resolution of the same tree issues **zero** HTTP requests,
      asserted against the `MockServer`'s request count
- [ ] A resolution with the release host unreachable still succeeds on a populated
      cache
- [ ] Two concurrent cold resolutions of the same tree issue **exactly one** archive
      fetch, asserted against the `MockServer`'s request count, and neither observes
      a partial tree
- [ ] A winner that fails mid-materialisation releases the lock, and the loser makes
      progress rather than waiting on a pointer that will never appear
- [ ] A crash at each of steps 4 through 10 leaves only reclaimable garbage: no
      pointer is published, `locate` reports a miss, and the reaper removes the
      residue
- [ ] A pointer naming a directory that does not exist, is not a direct child of
      `trees/`, is not 64-hex, or is not owned by the effective uid is treated as a
      miss rather than exported
- [ ] A sealed tree is removable by `remove_dir_all` without an intervening chmod;
      an archive member marked executable is still executable after sealing; and a
      symlink's target is not re-moded by the seal walk
- [ ] `cache verify` detects each of a deleted file, a truncated file, a **same-size
      same-mode** content substitution, a mode change, a changed symlink target, and
      an unexpected extra entry
- [ ] `cache verify` succeeds with the release host unreachable
- [ ] A truncated tree and a corrupted tree are each returned to a working state by
      `accelerator cache repair`, which materialises a **new generation** and swaps
      the pointer rather than removing the old tree first
- [ ] A repair whose refetch fails leaves the previous tree in place and still
      resolvable
- [ ] A repair run while a process holds files open in the old generation does not
      unlink them, and that process can still open further files from it
- [ ] `repair --force` re-materialises a tree that passes `verify`
- [ ] Every `cache` verb refuses an unrecognised `<name>` without touching the
      filesystem
- [ ] Two release versions naming the same digest share **one** generation
      directory and two pointers, and the second version issues **zero** archive
      fetches
- [ ] Two platforms sharing one cache root each resolve their own tree
- [ ] A cache root that is group- or world-writable, or not owned by the effective
      uid, is refused rather than trusted
- [ ] The reaper removes a temp archive, a temp tree, and an unreferenced
      generation whose owning pid is dead; spares all three while it is live; and
      spares nothing indefinitely once the age backstop passes, including after pid
      reuse
- [ ] `cache prune` reclaims an unreferenced generation and leaves the pointed-at
      one
- [ ] `manifest.example.json` with an added `artifacts` key still parses, and a
      manifest *without* `artifacts` still resolves single-file binaries
- [ ] `mise run cli:check` exits 0
- [ ] `mise run deny:check` exits 0, and `libz-sys`/`zlib-ng-sys`/`zlib-sys` are
      absent from the launcher feature graph
- [ ] `test-trust-root` is absent from the launcher's default features, and
      `build.cli_cross_compile` passes no `--features` flag
- [ ] The `ACCELERATOR TEST KEY` marker appears in none of the four
      cross-compiled launchers, nor in any committed `bin/` shim
- [ ] A launcher built with `--features test-trust-root` verifies an artifact
      signed with the test key **and** one signed with the production key
- [ ] A warm executor invocation satisfies `after ≤ 1.1 × before` against a
      pre-Phase-4 launcher on the same host, measured with work-item:0186's method
      (50 interleaved samples in one process, order alternated) rather than a
      bash loop, which 0186 records as not method-comparable
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] Inflating the browser archive completes within a stated ceiling on the
      reference host — a threshold, not a recorded observation; if `rust_backend`
      misses it the escalation is a faster **pure-Rust** backend (`zlib-rs`, if it can
      be shown to need no C toolchain), never a `*-sys` crate
- [ ] Files in a materialised tree are not writable by the owning user without an
      explicit chmod, and the tree as a whole is still removable
- [ ] `accelerator cache verify` on a clean cache reports every tree as sealed
      and matching

---
---

## Phase 5: Release-pipeline assembly

### Overview

Assemble the driver bundle and the browser in CI from verified upstream inputs,
and publish them on the existing manifest and minisign path. Nothing in `tasks/`
exists to reuse for the *inputs*: there is no HTTP helper, no GPG code, and no npm
signature or SLSA verification. AC13 is three new implementations.

The *output* side reuses the existing path but is not free either. Every list on
that path is derived from `DISPATCHED_SUBBINARIES` by design rather than from a
directory scan, so tree artifacts are invisible to signing, upload and
pre-publish re-verification until each is given an explicit arm (§5). Getting that
wrong publishes a signed manifest promising assets that do not exist, which is
unrecallable.

### Changes Required

#### 1. Pin the vendored version

**File**: `skills/design/inventory-design/scripts/playwright/package.json`
**Changes**: `~1.55.1` becomes the exact version. This makes the fetched package,
the API `lib/*.js` was written against, and the derived Chromium revision one
choice rather than three that can drift. AC10's guard reads this file.

#### 2. Upstream input verification

**Files**: `tasks/vendor/verify.py` (new), `tasks/vendor/pins.py` (new),
`keys/nodejs-release.asc` (new), `keys/npm-registry.pem` (new), `mise.toml`,
`RELEASING.md`
**Changes**: Three verifications, each failing the release rather than the user's
run. Each needs a trust anchor that does not arrive over the channel it is
verifying, and that is the part ADR-0059 leaves open: it establishes that the
sha512 integrity is fixity rather than provenance "because it comes from registry
metadata fetched over TLS", but never says where the key validating the *signature*
comes from. Fetching that key from the registry too would reproduce the same
problem one level up, so both key sets are committed.

- **`playwright-core`** — fetch from `registry.npmjs.org`, verify the registry
  signature against `keys/npm-registry.pem`, and verify the SLSA provenance
  attestation. That check is only as strong as its predicate:
  `gh attestation verify` without `--owner`/`--repo` accepts an attestation from
  any builder, so the expected source repository, the expected workflow identity,
  and a subject digest bound to the fetched tarball are all asserted explicitly,
  and any mismatch fails the release. `gh attestation verify` appears today only
  as a manual step in `RELEASING.md:271-281`; this makes it a pipeline step.
- **Node runtime** — fetch `SHASUMS256.txt` and its `.asc` from `nodejs.org/dist`,
  verify the GPG signature, then verify the tarball's digest against the signed
  manifest. The version is not chosen independently: ADR-0059 has it mirror the
  pairing upstream ships, so it is derived from the vendored driver's pairing and
  guarded like the Chromium revision (§4).

  The verification must not trust `gpg`'s exit code, which is **0** for a
  well-formed signature from a key merely present in the keyring and carrying no
  trust — it prints only `WARNING: This key is not certified` to stderr. So:
  `gpg --no-default-keyring --keyring` against the committed key, with `--status-fd`
  parsed.

  `VALIDSIG` alone is not the predicate, though, and this is the adjacent trap:
  GnuPG emits `VALIDSIG` for cryptographically valid signatures made by **expired**
  and **revoked** keys too — those cases replace `GOODSIG` with `EXPKEYSIG` or
  `REVKEYSIG` rather than suppressing `VALIDSIG`. A `VALIDSIG`-plus-fingerprint check
  would therefore accept a `SHASUMS256.txt` signed by a Node release key that has since
  been revoked, which is the single case where rotation matters most. So the check
  requires `GOODSIG` **and** explicitly rejects `EXPKEYSIG`, `REVKEYSIG`, `EXPSIG` and
  `NO_PUBKEY`, and compares the allowlist against `VALIDSIG`'s **primary-key**
  fingerprint field rather than only the signing subkey's. `gpg` joins the pinned tooling in `mise.toml`, since its
  presence and version on the `macos-latest` runner are otherwise incidental, and
  its absence must fail the release loudly rather than skip the check. The pinning
  route needs checking rather than assuming: `minisign` is pinned as a direct
  GitHub-release binary (`mise.toml:32-35`, `ubi:jedisct1/minisign`), and GnuPG is
  not distributed that way — if no satisfactory pin exists, pin the *behaviour*
  instead with a preflight that asserts a known-good signature verifies and a
  known-bad one does not, so a host `gpg` that cannot be pinned is at least
  proven functional before the release depends on it.
- **Chromium** — pinned, not verified, per ADR-0059. The revision is read from the
  vendored `playwright-core`'s `browsers.json` and cross-checked against
  `pins.CHROMIUM_REVISION`; the bytes are checked against a committed
  `pins.CHROMIUM_SHA256` per platform. That committed constant is what makes
  ADR-0059's "makes the bytes reviewable" true — a digest derived from whatever the
  CDN served this release attests our own output rather than the input, and is
  trust-on-first-use on every cut. Committing it converts that into one reviewed
  moment. It bounds blast radius; it does not establish provenance, and the
  module's docstring says so plainly rather than implying otherwise.

**One refresh procedure** covers both key sets and both pins, documented in
`RELEASING.md`, because they fail the same way — stale blocks releases, and
carelessly refreshed is the verification's weakest point, which is ADR-0059's own
recorded consequence. It requires that a new key or hash be obtained from a channel
independent of the one it will verify, landed in the same PR as the
`playwright-core` pin bump that motivated it, and reviewed as a change to a trust
anchor rather than a routine version bump. A Playwright upgrade is therefore one PR
touching the pin, four Chromium hashes, the eight `ASSEMBLED_SHA256` entries §8
introduces (two artifacts × four platforms), and any key that rotated with it.

The procedure is documentation, so it is backed by two mechanical guards, because a
committed anchor is only as strong as the review that gates it and this repository has
no CODEOWNERS file — a change to `keys/**` or `tasks/vendor/pins.py` is reviewed
exactly like a version bump today. First, a build-system test asserts the keys present
in `keys/nodejs-release.asc` are exactly the fingerprints in the committed allowlist and
that each is unexpired, so the two halves of the Node anchor cannot diverge silently.
Second, a CODEOWNERS entry (or an equivalent CI guard) covers `keys/**` and
`tasks/vendor/pins.py`, so a trust-anchor change cannot merge on a routine path.

The assembled digests are the one anchor whose value cannot be obtained
independently — they are computed from our own deterministic assembly of inputs the
other anchors have already verified, so they attest reproducibility rather than
provenance. The procedure records that distinction, and requires them to be
regenerated by a clean assembly on a machine that fetched the upstream inputs fresh,
never copied from a reuse path.

`requests` is added to the build-system dependency group, since `tasks/` has no
HTTP client and every existing fetch delegates to `npm`/`cargo`/`rustup`/`gh`.

#### 3. Assembly

**Files**: `tasks/vendor/assemble.py` (new), `tasks/build.py`,
`.github/workflows/main.yml`
**Changes**: Two tasks, not one, and — this is the part an earlier draft got wrong —
**two workflow steps**, not one:

- `vendor.verify_upstream_inputs` downloads and verifies, and **never extracts**. It
  needs `GH_TOKEN` for `gh attestation verify`.
- `build.assemble_tree_artifacts` extracts and assembles, and runs with **no**
  `GH_TOKEN`.

Wiring both into `release_prepare` would have made the split imaginary: `Prepare
stable release` (`.github/workflows/main.yml:604-607`) is a single step running
`mise run release:prepare` with `GH_TOKEN` in its `env`, so two invoke tasks inside
it share one environment. Assembly therefore gets its own mise task and its own
workflow step, invoked outside `release:prepare` — which is also what makes the
scoping assertable, since the existing attest-block tests inspect workflow shape and
cannot see inside an invoke call graph.

The split matters because assembly extracts an npm tarball and the Chromium zip,
and ADR-0059 records Chromium's custody as TLS-only with no signature. Today the
`Prepare` steps carry `GH_TOKEN` in a job holding `contents: write` and
`attestations: write`, upstream of the step holding
`ACCELERATOR_RELEASE_SECRET_KEY` — so a path-traversal entry could overwrite a
`tasks/*.py` module that the later Sign step imports. Extraction therefore lands in
a staging directory **outside the checkout**, only the finished archives are copied
into `dist/release/`, and the same entry rules the launcher applies (Step 4b step 4,
plus the entry-type allowlist, absolute-path and hardlink rejection, mode masking,
and size and count caps) apply CI-side too. This extends the rule the plan already
follows for the signing secret: the step that handles untrusted input holds no
credential.

**What a step boundary does and does not buy**, stated plainly rather than
overclaimed. It removes `GH_TOKEN` from the extracting step.

It does **not** currently remove the app token `actions/checkout` writes into
`.git/config`, and `persist-credentials: false` cannot simply be added:
`tasks/git.py:50-52` runs a bare `git push --atomic` with no credential helper, no
authenticated remote URL and no `gh auth setup-git`, so that persisted token is the only
credential the release push has. Adding the flag without a replacement wedges every cut
after the version bump has been pushed. If the hardening is wanted it must land together
with an explicit credential scoped to the finalise step, and the test must assert both. It does **not** remove the job-wide values: `id-token: write` and
`attestations: write` mean `ACTIONS_ID_TOKEN_REQUEST_URL`/`_TOKEN` and
`ACTIONS_RUNTIME_TOKEN` are present in every step of the job regardless of its `env`.
So an extraction escape still reaches enough to mint an OIDC token for a fraudulent
attestation.

Two things bound that residue. §8's committed `ASSEMBLED_SHA256` means tampered bytes
cannot reach the signing step at all — the attacker's path to a *signed* artifact is
closed independently of the token question — so what remains is token theft rather
than artifact substitution. And the extraction rules above are what stop the escape
happening in the first place.

Full isolation would mean a separate job with `permissions: {}`, passing ~1.2GB of
archives between jobs as workflow artifacts. That is deliberately not taken here: the
release job's own comment (`main.yml:600-603`) requires the prepare/sign/finalise
sequence to stay in one job for version monotonicity, the transfer cost is
substantial, and the digest pin already closes the outcome that matters. It is
recorded as the escalation if the residual is later judged unacceptable.

`build.assemble_tree_artifacts` produces, per platform:

```
dist/release/accelerator-driver-<platform>.tar.gz
dist/release/accelerator-browser-<platform>.tar.gz
```

Flat in `dist/release/` — `@actions/glob`'s `*` does not cross `/`, so a nested
staging tree would silently miss `dist/release/accelerator-*` and fail
`test_attest_globs_cover_every_published_asset`
(`tests/unit/tasks/test_workflows.py:207-221`).

The driver tree contains the Node binary and `playwright-core`. The browser tree
contains `chromium-headless-shell` only; `ffmpeg` is excluded.

Assembly also **decides whether the trees contain symlinks at all**, and records the
answer, because the launcher's extraction allowlist admits in-root symlinks — the
trickiest branch in the extractor, since defeating a symlink-then-traverse chain needs
each entry resolved against the real root as it is created. Since we produce the
archives, that branch may be unnecessary: if assembly emits no symlink, a CI-side
assertion pins that and Step 4b narrows its allowlist to regular files and directories
only, retiring the hardest-to-review code in the extractor rather than maintaining it
for a capability nothing exercises.

Both tasks are wired into `prerelease_prepare` (`tasks/release.py:117-129`) and
`release_prepare` (`:144-160`), verification then assembly, **after**
`build.cli_cross_compile` and **before** `build.create_debug_archives`. They go in
`prepare`, never `sign` — `_sign` (`:86-100`) is the only function holding the
secret, and `.github/workflows/main.yml:494-499` scopes
`ACCELERATOR_RELEASE_SECRET_KEY` to Sign steps deliberately. No npm, nodejs.org or
CDN fetch ever happens inside `_sign`.

#### 4. Version guards

**File**: `tasks/vendor/assemble.py`
**Changes**: The assembly fails the release if the fetched `playwright-core` is
not the exact version `package.json` declares, or if the fetched Chromium
revision is not the one that package's `browsers.json` names. Per ADR-0059 the
pairing is structural, so this guards the construction rather than testing
compatibility after the fact.

#### 5. The publish path: registry, signing, manifest, upload, re-verify

**Files**: `tasks/shared/paths.py`, `tasks/signing.py`, `tasks/manifest.py`,
`tasks/github.py`
**Changes**: Every list on the publish path is derived from an explicit registry
rather than from a directory scan, and each is derived from the *same* one —
`upload_and_verify_release` (`tasks/github.py:335-337`) records why: the "every
asset uploaded" and "every asset re-verified before `--draft=false`" lists "cannot
derive from two values". Tree artifacts need the same treatment, so they start with
a registry.

`tasks/shared/paths.py` gains `TREE_ARTIFACTS: tuple[str, ...] = ("driver",
"browser")` beside `DISPATCHED_SUBBINARIES` (`:29`), plus a
`tree_artifact_asset_path(name, platform)` mirroring `subbinary_asset_path`
(`:79`). Assembly, signing, manifest emission, upload and re-verification all
derive from it, so adding or retiring an artifact is one edit rather than a hunt
across five files.

The single source has to cross the language boundary, though, or it stops at `tasks/`.
The Rust side encodes the same names in two places — the launcher's compiled-in set
that validates `cache` verbs offline, and `accelerator-design`'s `ensure` call sites —
so a **drift test** pins the Rust set against `TREE_ARTIFACTS` and both against the
`artifacts` keys in `manifest.example.json`, in the same shape as the
`BUILTIN_SUBCOMMANDS` ↔ clap `Command` pin registration point 10 already requires.
Without it, retiring an artifact yields a launcher exporting a variable nothing
publishes, or a design binary requesting a name the manifest no longer carries —
failures that surface at runtime on a user's machine, since trees are exempt from the
per-exec re-verification that would otherwise catch a mismatch.

Four arms follow, and none is optional:

1. **Signing** — `sign_staged_binaries` (`tasks/signing.py:60-79`) builds an
   explicit expected list from the launcher plus `_subbinary_signing_targets()`
   and raises on any missing member, deliberately never scanning a directory. A
   `_tree_artifact_signing_targets()` arm joins it, so a partial assembly fails
   closed exactly as a partial cross-compile does.
2. **Manifest** — `collect_artifact_entries()` mirrors `collect_entries`
   (`tasks/manifest.py:80-107`) and a second key joins `build_manifest`
   (`:110-129`). It emits more than `collect_entries` does: alongside `sha256` and
   the inline signature it records `archive_size`, `uncompressed_size` and
   `entry_count`, all three measured during assembly rather than restated, so
   producer and consumer cannot disagree about the bounds the launcher enforces.
   **Do not bump `SCHEMA_VERSION`** (`:23`). Ordering is not free
   here: `collect_entries` slurps the pre-produced `.minisig` contents as the
   inline signature, so collection must follow signing — which `_sign`
   (`tasks/release.py:84-99`) already sequences correctly, and the artifact arms
   slot into those same two calls.
3. **Upload** — `_release_uploads` (`tasks/github.py:231-248`) assembles launcher,
   manifest, debug archives and `_subbinary_uploads`; a `_tree_artifact_uploads`
   arm joins it, each archive with its `.minisig` sidecar. The existing `missing`
   check (`:339-343`) then fails loudly on an unassembled artifact before a single
   upload starts.
4. **Re-verification** — `_subbinary_reverifies` (`:287-315`) reads
   `manifest["binaries"][name]` and re-downloads each asset to check its sha256
   and inline signature. A `_tree_artifact_reverifies` arm reads
   `manifest["artifacts"][name]` and does the same, so the `--draft=false`
   transition (`:356`) waits on the tree archives too.

Without all four, the release publishes a *signed* manifest naming artifacts that
were never signed, never uploaded and never re-verified —
`_assert_staged_manifest_is_current`'s own docstring names that outcome as one that
"cannot be recalled". Every user on that version would 404 on their first design
run, and the fix would be a whole new release.

#### 6. The five guards that will trip

**Files**: `tasks/release.py`, `tests/unit/tasks/test_manifest_contract.py`,
`cli/launcher/tests/fixtures/manifest.schema.json`, `.github/workflows/main.yml`
**Changes**:

1. `_assert_staged_manifest_is_current` (`tasks/release.py:57-83`) compares only
   `set(staged["binaries"])` against `DISPATCHED_SUBBINARIES`. Without an
   artifact equivalent a stale artifact manifest passes silently — add the
   parallel arm against `TREE_ARTIFACTS`.
2. `test_attest_globs_cover_every_published_asset` — satisfied by the flat
   naming above, but assert it rather than assume it.
3. `test_every_attest_block_declares_the_same_subjects` (`:198-204`) — all three
   blocks (`main.yml:502-508`, `:615-621`, `:639-645`) must stay identical.
4. `tests/unit/tasks/test_manifest_contract.py:30-48` iterates `binaries` only;
   add a parallel arm for `artifacts`, asserting the same
   `accelerator-{key}-{platform}.tar.gz` convention Phase 4 pinned in
   `manifest.example.json`, so producer and consumer are held to one fixture.
5. `cli/launcher/tests/fixtures/manifest.schema.json` describes itself as "the
   signed distribution contract between the release signer and the
   launcher/bootstrap readers", and its top-level `required` is
   `["schema_version", "version", "binaries"]` with no `artifacts` property and no
   artifact `$defs`. It gains both — an `artifactEntry` and an
   `artifactPlatformEntry` carrying the three required sizes — or the one document a
   third party would read to understand the wire format describes a shape the
   producer no longer emits. `test_schema_platform_enum_matches_the_alias_set` reads
   only `$defs.binaryEntry` today, so it is extended to assert the artifact side's
   platform-alias enum equals `ALIASES` too; otherwise the guard that exists to stop
   platform tables drifting would not cover the new axis.

A fifth guard turns out **not** to trip: `_assert_no_leaked_artifacts`
(`tasks/release.py:40-54`) matches markers against `git status --porcelain -uall`,
and `/dist/` is gitignored (`.gitignore:23`), so the archives are invisible to it.
Worth recording so nobody spends time on it.

#### 7. Release-job capacity

**File**: `.github/workflows/main.yml`
**Changes**: The `release` job runs the whole pipeline **twice** in one job —
stable, then the post-stable pre.0 cut (`:604-650`) — so roughly 2.4GB of
assembly and upload per stable release, on a `macos-latest` runner with no
`timeout-minutes` and no disk guard. `dist/release/` is never cleaned between the
two (`tasks/release.py:60-62`), and `--clobber` on retry
(`tasks/github.py:318-319`) re-uploads the lot.

Add a `timeout-minutes` to the release job and a disk-space assertion before
assembly. Hosting capacity itself is confirmed and assumed.

One failure path also changes character at this payload size. `download_and_verify`
(`tasks/github.py:140-145`) converts a `subprocess.TimeoutExpired` into an
`AssetVerificationError`, which preserves the draft — but the two re-verify helpers
tree artifacts actually reach do not: `_reverify_via_shim` (`:192-193`) and
`_reverify_subbinary` (`:206`) call `download_release_asset` bare, and its
`timeout=120` (`:111`) is sized for a 7.6MB launcher. A 177MB archive plausibly
exceeds it, raising `TimeoutExpired`, which is not an `AssetVerificationError` and
so lands in `upload_and_verify_release`'s `except Exception` arm — running `gh
release delete <tag> --cleanup-tag --yes` (`:359-365`) *after* `_publish` has
already committed, tagged and pushed the version bump. A transient download hiccup
would burn a version number and leave the repository and the release host
inconsistent, under the `accelerator-release` concurrency lock.

So: size `download_release_asset`'s timeout to the expected asset rather than a flat
120s, and wrap both re-verify helpers so a transport failure becomes an
`AssetVerificationError`. That routes it to the draft-preserving arm with the forensic
alert that already exists, and `--clobber` (`:318-319`) means a preserved draft can be
re-driven to green.

`TimeoutExpired` is not the only newly-reachable path, though, so the narrowing is by
**default** rather than by enumeration: every failure inside the upload/re-verify
envelope preserves the draft, and the delete arm is reserved for an explicit,
enumerated set of pre-upload failures. At ~2.4GB per stable cut,
`OSError: No space left on device` from a re-verify download or from `compute_sha256`,
a hung `gh release upload` (which has neither timeout nor retry), and a
`CalledProcessError` from a transport blip are all now plausible — and each would
otherwise delete the tag after `_publish` has already pushed the version bump. Bounded
retry with backoff wraps `_upload_clobber`, the disk assertion covers the whole job
rather than only pre-assembly (assembly plus upload staging plus re-verify temp
downloads, across both passes), and the newly-added `timeout-minutes` is itself
recorded as an abort cause that runs no cleanup arm, so it is sized with headroom and
`--clobber` is documented as its recovery.

#### 8. Reuse across cuts, and a functional gate

**Files**: `tasks/vendor/assemble.py`, `.github/workflows/main.yml`
**Changes**: Two problems that only appear once assembly is in the pipeline.

**Every release becomes dependent on three third-party hosts.** Assembly is wired
into both `prerelease_prepare` and `release_prepare`, so every cut fetches from
`registry.npmjs.org`, `nodejs.org/dist` and `cdn.playwright.dev` — yet all three
inputs are now pinned by exact version and hash, so the produced bytes are identical
release after release. As written, an npm outage, a key rotation or a yanked version
makes the pipeline unreleasable, including for an urgent fix to something entirely
unrelated: a large new single point of failure in front of the one mechanism that
ships fixes to users.

So assembly becomes **deterministic and digest-pinned**, and reuse is authenticated
rather than merely cached.

**Deterministic assembly.** `assemble_tree_artifacts` normalises everything that
would otherwise vary between runs: entries emitted in sorted order, mtimes, uid, gid
and owner names fixed to constants, modes masked to the same `0755`/`0644` the
launcher enforces, and gzip written without an embedded timestamp. Assembling the
same pin triple twice must produce byte-identical archives, asserted by a test that
assembles twice and compares digests. This is worth doing on its own merits — it
makes a release auditable by anyone who can run the same pins — but it is also the
precondition for everything below, because an unreproducible archive cannot be
pinned.

**A committed expected digest.** `pins.py` gains `ASSEMBLED_SHA256`, one digest per
artifact per platform, committed and reviewed under the same trust-anchor refresh
procedure as the keys and the upstream pins (§2). Every archive that reaches the
signing step — freshly assembled or reused — is checked against it, and a mismatch
fails the release. Without this the digest check is self-referential: "matching
digest" computed from whatever is on disk proves only that the bytes are the bytes.

**Reuse is our own signed asset, not a cache blob.** When the pin triple is
unchanged, the reuse source is the **previous release's published artifact**,
re-downloaded and verified with sha256 plus minisign against the embedded public key
— the identical check the launcher performs on a user's machine — and then against
`ASSEMBLED_SHA256`. That keeps the chain of custody inside our own signature rather
than extending trust to a mutable store: a CI cache is writable from other workflows
on the default branch, is evictable after a quiet week, and shares a per-repository
budget with the toolchain caches this repo already depends on, so a poisoned or
partially-restored entry would be signed with `ACCELERATOR_RELEASE_SECRET_KEY` and
published with none of §2's npm, SLSA, GPG or Chromium-hash gates re-running — the
plan's own "cannot be recalled" outcome, reached by accident rather than by attack.
Any mismatch, any absent asset, and any pin movement falls back to a full cold
assembly, so the reuse path can only ever be an optimisation.

The bytes therefore reach a user having been verified against upstream once, at a
reviewed moment recorded in `pins.py`, and authenticated to our own key on every
reuse after that. The same mechanism removes the duplicated work in the release job's
double pass (§7): the post-stable pre.0 cut reuses the stable pass's archives by
digest instead of re-assembling identical bytes.

**Nothing ever executes what was built.** Every other gate in this phase is about
provenance and shape: upstream signatures, version and hash guards, glob coverage,
manifest arms, and a `.minisig` the CLI-side verifier accepts. A brand-new step
composing four platforms from three upstreams can produce a correctly-signed,
correctly-hashed, structurally-wrong tree — wrong architecture, missing `NOTICES/`,
a layout `playwright-core` cannot resolve — and it would pass everything, reach
every user of that release, never self-heal (trees are exempt from per-exec
re-verification), and be faithfully re-fetched by `cache repair`, which trusts the
same manifest. Recovery would be a new release for every affected user.

So assembly ends with a functional gate — but **not inside the release job**. Executing the
vendored Node binary and `chromium-headless-shell` is a stronger form of handling untrusted
input than extracting them, and Chromium is the one input ADR-0059 records as accepted on
TLS trust alone with no signature. The `release` and `prerelease` jobs carry
`ACCELERATOR_RELEASE_SECRET_KEY` in a later step, plus job-wide `id-token: write` and
`ACTIONS_RUNTIME_TOKEN` that no step-level `env` can scope away — so a compromised CDN build
would gain code execution one step before the signing key enters the environment. §3's own
rule ("the step that handles untrusted input holds no credential") was written for
extraction and applies here with more force.

The smoke check therefore runs in a **separate job with `permissions: {}`**, consuming the
archives as workflow artifacts: unpack the driver and browser for that runner's platform,
execute the Node binary and the headless shell with `--version`, and assert `NOTICES/` is
populated. It runs on **reused** archives as well as freshly assembled ones — a reuse path
that skipped it would be the one route by which an unexecuted artifact reaches a release.
The publish step gates on that job.

If the artifact transfer between jobs proves disproportionate, the fallback is to keep only
the structural check in-job (below) and accept the reduced assurance explicitly — never to
execute upstream binaries beside the signing key.

It cannot cover the cross-compiled platforms, so the three non-host artifacts get a
structural check instead: the expected file set, plus the ELF/Mach-O header and
architecture of the Node binary and the headless shell for the target they claim to
be. That catches a wrong-architecture or truncated assembly without executing it,
which is the failure mode that would otherwise reach every Linux user of a release,
never self-heal, and be faithfully re-fetched by `cache repair`. Between them the two
checks are the only gates distinguishing "signed" from "works", and both are nearly
free on the macOS runner.

#### 9. Redistribution notices

**File**: `tasks/vendor/assemble.py`
**Changes**: Each artifact carries the notices for what it contains — Node and
its bundled dependencies, `playwright-core`, and Chromium's credits — assembled
into a `NOTICES/` directory at the tree root. Phase 7 adds the subcommand that
surfaces them, so a user reaches them without unpacking the artifact by hand.

An automated assertion covers it here rather than only the manual check: the produced
tree contains `NOTICES/` with an entry per expected component, driven from the same
component list the assembly uses. AC16's notices are the plan's stated substitute for
a legal review gate, so an assembly refactor that silently drops a component must
fail rather than ship.

### Success Criteria

#### Automated Verification

- [ ] Failing tests first for each verification, using recorded upstream fixtures
      rather than live network calls. Committing the keys makes Node/GPG fully
      offline-verifiable, so it is tested for real rather than mocked; the SLSA
      check contacts a transparency log, so its runner is injected and both
      branches asserted — and the plan records that the attestation's *content* is
      not verified in tests
- [ ] A tampered `SHASUMS256.txt` signature fails the release
- [ ] A `SHASUMS256.txt` signed by a well-formed key absent from the committed
      fingerprint allowlist fails the release, **even though `gpg` exits 0**
- [ ] An absent `gpg` fails the release rather than silently skipping the check
- [ ] A `SHASUMS256.txt` signed by a revoked key fails the release, and one signed by
      an expired key fails the release, even though both yield `VALIDSIG`
- [ ] The npm/SLSA path fails closed in each degraded mode — attestation bundle
      absent, transparency log unreachable, `gh attestation verify` unavailable — since
      only mismatch cases were covered before, and these are the modes most likely to
      be made advisory under release pressure
- [ ] The committed Node keyring and the committed fingerprint allowlist describe the
      same key set, each unexpired
- [ ] A `playwright-core` tarball failing its registry signature fails the release
- [ ] An attestation whose source repository or workflow identity differs from the
      pinned predicate fails the release
- [ ] An attestation whose subject digest does not match the fetched tarball fails
      the release
- [ ] A `playwright-core` version other than `package.json`'s pin fails the
      release
- [ ] A Chromium revision other than `browsers.json`'s fails the release
- [ ] Chromium bytes whose sha256 differs from `pins.CHROMIUM_SHA256` fail the
      release
- [ ] A Node version other than the vendored driver's pairing fails the release
- [ ] Assembly is its own workflow step, and that step's `env` contains no
      `GH_TOKEN` — asserted by a workflow test alongside the existing attest-block
      assertions, which is only possible because it is a step rather than a task
      nested inside `release:prepare`
- [ ] If `persist-credentials: false` is adopted, the finalise step has an explicit
      credential and the push still authenticates — asserted together, since the flag alone
      breaks every release
- [ ] Extraction happens outside the checkout, and a tarball with a `../` entry, an
      escaping symlink, a hardlink, an absolute path or a setuid bit is rejected
      CI-side by the same rules the launcher applies
- [ ] The assembled, signed, manifest-listed, uploaded and re-verified sets are
      pinned against each other by one test, so an artifact cannot appear in some
      and not others
- [ ] An unassembled artifact fails the **signing** step, not the upload step
- [ ] A tree archive with no `.minisig` fails `collect_artifact_entries`
- [ ] `_assert_staged_manifest_is_current` rejects a manifest whose `artifacts`
      keys differ from `TREE_ARTIFACTS`
- [ ] An artifact platform entry missing any of `archive_size`,
      `uncompressed_size` or `entry_count` fails to parse, rather than defaulting
      to 0 and disabling the cap it feeds
- [ ] The emitted sizes match the assembled archive and its extracted tree, so
      producer and consumer agree on the bounds the launcher enforces
- [ ] `manifest.schema.json` validates a manifest carrying `artifacts`, and its
      artifact platform-alias enum equals `ALIASES`
- [ ] A simulated download timeout during tree re-verification preserves the draft
      and emits the forensic alert, rather than deleting the release and its tag
- [ ] `mise run test:unit:build-system` passes, including the new manifest
      contract and attest-glob arms
- [ ] `mise run build-system:check` exits 0
- [ ] Every produced archive matches `dist/release/accelerator-*`
- [ ] Assembling the same pin triple twice produces **byte-identical** archives
- [ ] Every archive reaching the signing step matches `pins.ASSEMBLED_SHA256`,
      whether freshly assembled or reused; a mismatch fails the release
- [ ] An unchanged pin triple reuses the previous release's published artifact and
      performs **no** upstream fetch; moving any one pin re-runs the
      fetch-and-verify path
- [ ] A reused artifact failing its minisign check, its sha256, or the committed
      digest falls back to a full cold assembly rather than being signed
- [ ] The second (pre.0) pass reuses the stable pass's archives rather than
      re-assembling them
- [ ] The smoke check runs in a job with `permissions: {}` and no access to the signing
      secret, asserted by a workflow test — executing upstream binaries beside the release
      key is the failure this separation exists to prevent
- [ ] The smoke check runs on reused archives as well as freshly assembled ones, and fails
      the release on a tree whose Node binary or headless shell will not execute, or whose
      `NOTICES/` is empty
- [ ] The structural check fails a cross-compiled artifact whose Node binary or
      headless shell has the wrong architecture or object format for its target
- [ ] The produced tree contains a `NOTICES/` entry per expected component, driven
      from the assembly's own component list
- [ ] An end-to-end round trip: a synthetic tree assembled through the real
      assembly path, a manifest emitted through the real `build_manifest`, signed
      with a test key, resolved by the launcher's tree resolver — so the two halves
      of the artifact contract are verified together rather than only by hand
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] A full local dry-run assembly produces both artifacts for one platform, and
      their measured sizes are recorded and fed back into Step 4a's fetch deadline
- [ ] Each produced `.tar.gz` has a `.minisig` that the CLI-side verifier accepts
- [ ] The upload list and the re-verify list, printed for one platform, each
      contain both tree archives and their sidecars
- [ ] `manifest.json` renders `artifacts` beside `binaries` with a launcher built
      before this phase still resolving single-file binaries from it
- [ ] The `NOTICES/` directory in each artifact contains all three licence sets

---
---

## Phase 7: Swap onto the bundled driver and browser

### Overview

Point the executor at launcher-resolved tree artifacts, retarget the automation
at `playwright-core`, and delete the on-machine install. Depends on Phases 4, 5
and 6.

### Changes Required

#### 1. Retarget the automation

**Files**: `lib/daemon.js`, `lib/playwright-loader.js` and its three fixture trees,
`lib/playwright-loader.test.js`
**Changes**: The assembled bundle ships `playwright-core`, not `playwright`.
`playwright-loader.js:23-67` requires `<nsRoot>/node_modules/playwright/package.json`
and deliberately throws when `exports['.']` is an object whose `.import` is not a
string (`:53-56`) — the fix for the 0072 CJS-shim bug.

`daemon.js` uses only `chromium.launch({headless:true})` (`:106`) and
`chromium.executablePath()` (`:121`), both present in `playwright-core`. So
`daemon.js` imports `playwright-core` directly, matching what Microsoft's own
bindings do, and `playwright-loader.js`, its test and its three
`fake-playwright*` fixture trees are deleted.

The 0072 regression it guarded does not recur: the bug was the loader selecting
a CJS shim entry from a `playwright` package whose `exports` map it
misinterpreted, and there is no longer a loader making that selection. A test
asserts `chromium` is a defined export of the resolved module, which is the
property 0072 actually cared about.

#### 2. Passing the browser path, and the `chromium-not-found` diagnostic

**File**: `lib/daemon.js`
**Changes**: `daemon.js:106` calls `chromium.launch({ headless: true })` with **no
`executablePath`**, so `playwright-core` resolves from its own browser registry —
exactly the mechanism both the bundled tree and the `design.browser_path` hatch must
override. Without an explicit argument the path would be resolved in Rust and then
ignored in JS, and **AC12 could not pass**. So `daemon.js` reads the resolved path
from `ACCELERATOR_DESIGN_BROWSER_EXECUTABLE` and passes it:
`chromium.launch({ headless: true, executablePath })`.

**The `ping` handler must read the same variable, not `executablePath()`.** An
earlier draft asserted that `cr.executablePath()` at `:121` "reports that same
resolved path". It does not: `BrowserType.executablePath()` is computed from
`playwright-core`'s **browser registry** — the `PLAYWRIGHT_BROWSERS_PATH` layout or
its default — and neither takes nor reflects a per-launch `executablePath` option.
With the bundled sealed tree, and a fortiori under the hatch pointing at a distro
Chromium, that registry path does not exist, so `daemon.js:123`'s
`promises.access(execPath)` throws and `ping` returns `chromium-not-found`.

That is not a cosmetic error. `ping` is the readiness probe `SKILL.md` Step 5 runs,
and its failure is the `executor-ping-failed` downgrade — so **every crawl would
degrade to the code-only crawler on exactly the machines the bundled artifacts exist
to serve**, and AC6 and AC12 would both fail, after Phases 4 and 5 have shipped
~1.2GB per release to support them. The handler therefore `access()`es and reports
the launch path. If the registry path is wanted as a secondary diagnostic it may be
reported alongside, but it never decides the outcome.

The diagnostic's own text changes too: `:120-125` reports against the **full
Chromium** path while this ships `chromium-headless-shell`, and its message says
"Run ensure-playwright.sh to reinstall", naming a script this phase deletes. It is
rewritten to name `accelerator cache repair` — the remediation that now applies.

Passing `executablePath` explicitly also resolves the sealed-tree layout risk rather
than merely mitigating it: supplying the path is what makes `playwright-core` skip
registry resolution and its validation entirely, so the browsers root of a
`0444`/`0555` tree is never consulted or written. Confirm that empirically against
the pinned `playwright-core` version — a test asserting a launch succeeds against a
read-only browsers root is the cheapest form — and if any path still writes there,
place the marker outside the tree rather than unsealing it.

#### 3. Tree resolution

**Files**: `cli/design-cli/src/executor.rs`, `cli/launcher/src/main.rs`,
`cli/launcher/src/launch/core.rs`
**Changes**: The embedded signing key keeps exactly one holder (ADR-0060), so the
launcher owns materialisation — but it must not own the *decision*, because
ADR-0057 puts the ordering and the downgrade vocabulary in the design binary. The
split is by cost:

- **Warm, on every dispatch**: for each tree `locate` resolves, the launcher exports
  `ACCELERATOR_TREE_<NAME>` — a generic name derived from the pointer files present
  on disk, not a `DESIGN`-prefixed one, so the launcher enumerates rather than knows,
  and a second tree consumer inherits the convention rather than a design-shaped
  variable. That is `locate`'s two small reads and two stats per tree (Step 4b),
  issues no network request, and has no failure mode: a tree that is absent,
  unpointed, unparseable, or failing its ownership check simply yields no variable.
  The launcher learns nothing about design's subcommands, and no dispatch path can
  fail because of a tree.

  The variables are **always set or explicitly cleared**, never merely left alone, so
  an inherited or injected value from the surrounding environment can never be
  mistaken for one the launcher resolved.
- **Cold, only when needed**: `accelerator-design` calls
  `accelerator cache ensure <name>` at the point in its own ordering where it has
  established that it needs the runtime. That is the only place a ~294MB fetch can
  be triggered, so `validate-source`, `resolve-auth`, `scrub-secrets`,
  `notify-downgrade` and `audit-cue-phrases` never touch the network, and
  `notices` reads whatever is already materialised.

An absent variable is therefore the normal state rather than an error: it means
"not materialised yet", and the executor decides whether to `ensure`, downgrade,
or proceed. That is also what makes the `ACCELERATOR_DESIGN_BIN` dev override work
— it bypasses the launcher's resolve path entirely, so the variables are never set
and the executor reaches `ensure` exactly as it would on a cold cache.

**The `ensure` contract**, since this is a machine-consumed interface between two
separately-built executables rather than a human-facing command:

- **Discovery.** `accelerator-design` must locate the launcher to invoke it, and
  `argv[0]` is its own content-addressed cache path. The launcher exports
  `ACCELERATOR_LAUNCHER_BIN` (its own resolved shim path) alongside the tree
  variables; its absence is itself a diagnosable cause, not a panic. This closes the
  dev-override case too: `ACCELERATOR_DESIGN_BIN` bypasses the resolve path, so the
  variable is unset and the executor reports `artifact-unavailable` with a cause
  naming why, rather than failing opaquely.
- **Envelope.** `ensure` emits a golden-pinned structured envelope with an enumerated
  cause set mapped 1:1 onto downgrade reasons — unreachable host, signature mismatch,
  digest mismatch, disk shortfall, unwritable cache root, platform unsupported,
  artifact absent from the manifest. The executor maps causes, never parses prose.
- **Version skew.** Against a launcher predating Phase 4, `cache` is not a built-in
  and is treated as a dispatch token, producing an `AssetNotFound` for
  `accelerator-cache-<platform>` — a distribution error that would surface instead of
  a downgrade. So an unrecognised cause, a non-zero exit with no parseable envelope,
  and a resolution error all map to `artifact-unavailable`.

Collapsing every cause into `artifact-unavailable` unconditionally would leave a 3am
failure with no diagnosis, which is why the cause set exists; mapping *unknown* causes
there is the fallback, not the default.

**A failed materialisation is sticky for the session.** A crawl makes 100–200
executor invocations, and with no negative caching a persistent failure — a full
disk, a read-only plugin root, a flapping link, a 404 for one platform — would
produce a fresh full-size attempt, times three fetch retries, on *every one* of them.
A single crawl on a failing machine could attempt tens of gigabytes and repeatedly
fill the user's disk with partial archives. This risk did not exist for
megabyte-scale single-file sub-binaries. So the first `artifact-unavailable` downgrade
suppresses re-attempts and the remaining invocations take the code-only path
immediately.

The marker lives in the executor's own state directory — the `0700` directory under
the repo's config tmp path the sibling plan's Phase 6 already establishes — **not**
beside `trees/`.
Two of the failure causes it exists to damp are a full disk and an unwritable cache
root, so a marker written into the cache root could not be created in exactly the
cases that recur; the state dir is writable when the cache root is not. It records the
artifact name, the cause and a timestamp, and it is cleared by any successful `ensure`
and by `cache repair`, so the documented remediation is also the reset. Its TTL is
stated explicitly and derived from the crawl bound: a crawl is bounded at five
minutes, so a TTL of that order suppresses within-crawl retries without stranding the
next crawl after a user frees disk space or reconnects.

Tree-related failure envelopes also carry a remediation string naming
`accelerator cache repair <name>`. ADR-0060 accepts as a known negative that a
truncated tree "surfaces as a confusing runtime failure until the repair path is
run" — but self-healing needed no discovery, whereas this needs the user to already
know a command exists that the failure never mentions. Naming it in the failure is
what makes AC14's recovery reachable in practice rather than only documented.

The executor sets `NODE_PATH` and passes the browser executable path through to
`daemon.js` from the resolved trees, replacing the lockhash namespace. The layout
precondition that today exits 3 `playwright-not-installed` becomes an
`artifact-unavailable` downgrade rather than a hard failure, since the artifacts
are now fetchable.

#### 4. Failure ordering and the platform probe

**Files**: `cli/design/src/platform.rs` (new),
`cli/design-adapters/src/platform.rs` (new), `cli/design-cli/src/executor.rs`
**Changes**: ADR-0057 requires the runtime check to come **before**
`design.browser_path` is consulted, because the hatch substitutes the browser and
never the runtime. A musl host must reach the code-only downgrade, not a
browser-path error. Nothing enforces any such ordering today because neither
check exists.

Order: platform supported? → runtime available? → browser resolvable (bundled,
then `design.browser_path`)? Each failure emits its downgrade reason, and the
default and hybrid crawler modes fall back to the code-only crawler. An explicit
`--crawler runtime` request hard-fails.

The platform check needs a mechanism that exists nowhere in the codebase today.
`HOST_PLATFORM` (`resolve/mod.rs:21-28`) is a compile-time constant reading
`linux-x64` on Alpine and Debian alike — `TARGETS` builds Linux against
`*-unknown-linux-musl` precisely so one binary runs on every libc — and the
manifest's platform axis carries no libc dimension. Nothing in the existing
resolution path can tell the two apart, so without a probe an Alpine host fetches
~294MB of glibc-linked artifacts, seals them, and dies at `execve` with a bare
ENOENT from the absent dynamic loader: the hard failure AC11 exists to prevent, at
maximum cost.

> **⚠ Blocked on Phase 0 Q1.** The probe's mechanism is not settled, and the version this
> section previously carried was wrong: it globbed for musl and glibc loader paths and
> treated "neither present" as `unsupported-platform`, which classifies **macOS** — a
> supported platform per ADR-0057 and the primary development platform — as unsupported,
> downgrading every Mac before it touches a tree. NixOS and both-loaders-present hosts were
> misclassified too. Do not implement from a guessed mechanism.

What is settled is the shape: the classification is a **pure function** in
`cli/design/src/platform.rs` over observations the adapter supplies, unit-tested over
injected inputs for every shape **including macOS**. The Alpine container fixture confirms
wiring but cannot on its own distinguish "detected musl" from "failed for some other
reason", which is why the unit test carries the property. And the probe runs **before** any
artifact resolution, so an unsupported host downgrades at zero network cost.

What Phase 0 Q1 must decide: whether the probe short-circuits at compile time on non-Linux
targets; what the positive glibc signal is (reading the ELF interpreter of
`/proc/self/exe` is the leading candidate, over a filesystem convention); and which way an
ambiguous host fails — failing *open* and letting `execve` fail may be better than failing
closed, for a capability that already has a graceful downgrade.

#### 5. `design.browser_path`

**Files**: `cli/config/src/catalogue.rs`, `scripts/config-defaults.sh`,
`cli/launcher/tests/fixtures/dump/dump.golden`, `docs-site/…/design.md`
**Changes**: Add to `EXTRA_KEYS` (`catalogue.rs:121-133`) — no default,
presence-only, exactly like `visualiser.editor`. That costs the catalogue entry,
a mirror at `scripts/config-defaults.sh:208-220`, a row in the dump golden, and
docs. It does **not** touch `assert_eq!(count, 55)` at `catalogue.rs:267` or the
Rust↔bash drift test, which does not extract `EXTRA_KEYS`. A catalogue *default*
would cost a new group, an entry in `default_for`'s hardcoded group loop
(`:230`), a `dump::assemble` arm, two extra drift-test loops and the count bump —
so `EXTRA_KEYS` is the route.

The `ACCELERATOR_DESIGN_BROWSER_PATH` env override is **not** a config-layer
concern: `config-adapters` reads exactly one env var and `store.rs:195-205`
documents that as the rule. `cli/visualiser/server/src/compose.rs:216-252`
(`resolve_optional`) is the exact env-beats-config shape, whitespace collapse
included — but it is **extracted into a shared crate with its tests** rather than
copied verbatim. Copying logic while leaving its tests at the original site is how
two copies drift, and this precedence is the mechanism AC12 rests on; verifying it
only through a container fixture would leave the edges untested. If extraction proves
disproportionate, the fallback is explicit precedence tests at the new site over env
set/unset × config set/unset × whitespace-only, so a mutation in either copy fails
locally.

#### 6. Downgrade vocabulary

**Files**: `cli/design/src/downgrade.rs`,
`skills/design/inventory-design/evals/fixtures/notify-downgrade/*`,
`skills/design/inventory-design/evals/evals.json`,
`skills/design/inventory-design/evals/benchmark.json`,
`skills/design/inventory-design/PROTOCOL.md`,
`skills/design/inventory-design/SKILL.md`
**Changes**: Keep `executor-ping-failed`; drop `node-missing`, `node-too-old` and
`bootstrap-failed`; add `unsupported-platform` (AC11's musl case) and
`artifact-unavailable`. The messages and their golden fixtures are rewritten to
match, and the fixtures become Rust goldens beside the subcommand — exhaustive by
construction, iterating the reason enum so a variant without a golden fails, which
replaces the message-key/fixture set-equality check `test-notify-downgrade.sh`
enforced.

**`disk-floor-not-met` and `cache-unwritable` are retained**, contrary to the
previous draft's "conditions that can no longer arise". Both still arise and are now
*more* likely: a first run needs headroom for a ~294MB archive **plus** its extracted
copy — ~600MB peak, more with both trees — and the cache root's unwritability is
already modelled as `CacheRootUnavailable` in the launcher. Today
`ensure-playwright.sh` refuses up front with a named reason; dropping these would
mean a disk-full condition surfaces mid-extraction as a generic
`artifact-unavailable`, having already consumed the remaining free space. So free space is
checked *before* a fetch starts against `archive_size + uncompressed_size` summed
over every tree about to be materialised — not against the archive size alone, which
would under-reserve roughly threefold and let the check pass on a machine that then
fills mid-extraction, which is the exact condition this reason exists to catch. A
partial temp tree is removed eagerly on failure rather than left to the reaper.

Three consumers beyond `downgrade.rs` and the fixtures were missing from the previous
draft's file list, and each names retired reasons by string:

- `evals/evals.json` — eval 20, `executor-bootstrap-failure-fallback`, expects
  "the literal `bootstrap-failed` downgrade message". Retargeted onto
  `artifact-unavailable`, which now covers its scenario, rather than deleted.
- `evals/benchmark.json` — six occurrences, updated in step.
- `PROTOCOL.md:555-566` — a table mapping every retired reason to an exit code, and
  the document the sibling plan defers here because exit 3 is redefined in this phase. It
  is the executor's published contract, so leaving it describing a vocabulary
  nothing emits is worse than the original drift the sibling plan already fixes.

`notify-downgrade-messages.json` is **deleted** here, with the `#[cfg(test)]` drift
test the sibling plan's Phase 2 §3 pins it by. Its content moved into the domain crate's
`const` table
at that point; once the vocabulary is rewritten there is no on-disk file left to
drift against, and keeping one would mean maintaining a second copy of a table the
compiler already makes exhaustive.

`scripts/test-design.sh:154-155` asserts the `inventory-design` `allowed-tools`
entry `Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/*)` — the
residual rule this section drops — so it is rewritten in step, not left to the Removal
sweep.

`SKILL.md` Steps 4–6 (`:117-133`) also change here: they invoke
`ensure-playwright.sh` and parse its `ACCELERATOR_DOWNGRADE_REASON=` stderr line.
With bootstrapping moved to build time there is no bootstrap step to run, so Step 4
is replaced by the executor's own ordering and the reason is read from the executor's
envelope. This is the second of the two places Migration Notes' "rewired in the phase
that deletes what they call" claim was not met. The residual
`Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/**/scripts/*)` `allowed-tools` rules, kept
alive by the sibling plan's Phase 2 §5 while these scripts existed, are dropped here too.

`regenerate-notify-downgrade-fixtures.sh` — a maintainer dev tool invoked by no
SKILL.md — is deleted with them; regeneration is a test affordance on the Rust
goldens.

#### 7. `design notices`

**File**: `cli/design-cli/src/notices.rs`
**Changes**: `accelerator design notices [--artifact driver|browser]` prints the
paths of the `NOTICES/` directories Phase 5 assembles into each tree, and lists
the components covered. This is what makes AC16's "reachable by a user without
unpacking the artifact by hand" true; it lands here rather than in Phase 5
because the trees it reads do not exist on a user's machine until this phase.

#### 8. Deletion

**Files**: `ensure-playwright.sh`, `test-ensure-playwright.sh`, `package-lock.json`,
`regenerate-notify-downgrade-fixtures.sh`
**Changes**: Deleted, along with `scripts/test-design.sh:486-490` (the
`test-ensure-playwright.sh` delegation) — which runs in CI, so omitting the edit
leaves `test:integration:config` red on merge. With them go the lockhash namespace
under
`${ACCELERATOR_PLAYWRIGHT_CACHE:-$HOME/.cache/accelerator/playwright}`, the
sentinel idempotency contract, the disk floor, the node-version floor and the
sweep.

### Success Criteria

#### Automated Verification

- [ ] Failing tests first for the failure-ordering state machine, at unit level
      over injected platform, runtime and browser resolution, so the ADR-0057
      ordering is pinned in a fast test rather than only in a container
- [ ] The libc classification returns musl, glibc and unsupported for the three
      loader-set shapes, over an injected directory listing
- [ ] An unsupported platform downgrades without issuing any HTTP request
- [ ] A non-executor design subcommand performs no tree resolution and no fetch on
      an empty cache
- [ ] With no tree variables set (the `ACCELERATOR_DESIGN_BIN` override path), the
      executor reaches `cache ensure` rather than failing
- [ ] `ensure`'s distinct failure causes map to distinct downgrade reasons
- [ ] A container fixture with Node absent from `PATH` fetches both artifacts,
      launches the headless shell, and emits the envelopes the sibling plan pinned (AC6)
- [ ] A musl/Alpine container fixture emits `unsupported-platform` and completes
      via the code-only crawler with a non-error exit — and does so with
      `design.browser_path` both set and unset (AC11)
- [ ] On a glibc host with the bundled browser unavailable and
      `design.browser_path` pointing at a system Chromium, the runtime crawler
      runs against that executable (AC12)
- [ ] `--crawler runtime` hard-fails on an unsupported platform
- [ ] Each artifact downloads at most once per platform per version (AC9)
- [ ] `chromium` is a defined export of the module `daemon.js` resolves
- [ ] `daemon.js` launches with an explicit `executablePath`, and the value it
      receives is the one Rust resolved — asserted for both the bundled tree and the
      `design.browser_path` hatch, since AC12 depends on it
- [ ] `ping` succeeds when `playwright-core`'s registry path does not exist,
      proving the handler checks the launch path rather than `executablePath()` —
      the regression that would silently degrade every crawl to code-only
- [ ] A launch succeeds against a read-only browsers root, proving an explicit
      `executablePath` bypasses registry validation and writes
- [ ] `resolve_optional`'s precedence is tested over env set/unset × config
      set/unset × whitespace-only, at whichever site owns it
- [ ] `design notices` has a success path and a failure path, including
      `--artifact`, over a fixture tree — it is one of the seven recorded
      subcommands, so AC1 applies to it
- [ ] A persistent materialisation failure produces **one** fetch attempt per
      session, not one per executor invocation
- [ ] A free-space shortfall emits `disk-floor-not-met` before any fetch starts, and
      an unwritable cache root emits `cache-unwritable`
- [ ] Tree-failure envelopes name `accelerator cache repair <name>`
- [ ] The retired reasons appear nowhere in `evals.json`, `benchmark.json` or
      `PROTOCOL.md`, and eval 20 passes against `artifact-unavailable`
- [ ] `mise run test:unit:design-automation` passes with the loader suite removed
- [ ] `mise run cli:check` exits 0
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] A full inventory crawl on a machine with no system Node produces the same
      artefacts as one on a machine with Node installed
- [ ] First-run download completes within a stated wall-clock ceiling at the stated
      minimum throughput (the same floor Step 4a's deadline encodes), with host and
      connection recorded — a pass/fail bound, not an observation
- [ ] `accelerator design notices` reaches all three licence sets
- [ ] Deleting one file from a sealed tree, then running
      `accelerator cache repair`, restores a working crawl

---

---

## Removal sweep

### Overview

The residue this plan owns: the last two scripts, the last two `test-design.sh` blocks and
the file itself, the floor arithmetic, the ADR-0060 amendment, the documentation of the
removed prerequisite, and the final no-`.sh` assertion.

### Changes Required

#### 1. The last scripts, and `test-design.sh`

**Files**: `scripts/test-design.sh`, `tasks/test/integration.py`
**Changes**: Phase 7 §8 deletes `ensure-playwright.sh`, `test-ensure-playwright.sh` and
`package-lock.json`. Two `test-design.sh` blocks go with them:

| Block | Lines | Disposition |
|---|---|---|
| delegated `test-ensure-playwright.sh` | 486-490 | dies with the script |
| `inventory-design` `allowed-tools` `scripts/*` glob | 154-155 | dies with the rule Phase 7 §6 drops |

The sibling plan re-homed the other fifteen blocks and cut its own ranges, so with these
two gone the file has nothing left to assert and is **deleted**.

That takes `scripts/` from 16 discovered suites to 15, and `_EXPECTED_CONFIG_SUITES`
(`tasks/test/integration.py:41`) is **already 15** — so it stays at 15, no edit, which is
the floor doing its job at full strength. Its docstring (`:77-90`) says the floor exists to
catch "an exec bit dropped … or a suite renamed off the `test-*.sh` convention"; every unit
of headroom is one suite that can leave CI silently, so headroom is the blind spot rather
than the safety margin. AC4's last lockstep requirement is satisfied here by leaving one
number alone and asserting it still matches.

#### 2. Documentation

**Files**: the `docs-site/src/content/docs/` pages describing the Playwright prerequisite,
`README.md`, `CHANGELOG.md`, `.claude-plugin/plugin.json`
**Changes**: `plugin.json:11` declares the `Node >= 20` requirement this plan removes — it
goes. Every page describing the bootstrap step, the lockhash namespace or the disk and
node-version floors is repointed at the vendored artifacts, the `cache` verbs and the
`design.browser_path` hatch. The new `design` docs page gains the artifact and cache
sections; `ACCELERATOR_CACHE_DIR` is documented as **trust-relevant** (it must be a
private, user-owned path), not merely as a longer-lived location.

#### 3. ADR and work-item amendments

**Files**: `meta/decisions/ADR-0060-launcher-resolved-tree-artifacts.md`,
`meta/work/0196-accelerator-design-inventory-gap-tooling-cli.md`
**Changes**: ADR-0060 says tree entries are "addressed by release version and digest";
Phase 4 addresses them by digest, platform and generation with the version in a pointer.
The amendment records what the ADR does not contemplate: content-based addressing with a
per-release pointer; that this introduces cross-version tree **adoption**, which is why the
layout carries a format version; that the pointer is a deliberately-unsigned local
indirection (or signed, per Phase 0 Q2); and whatever Phase 0 Q3 concludes about in-use
detection, since ADR-0060's repair story assumes it.

Per ADR immutability an accepted ADR is amended by superseding note rather than edited in
place — `/accelerator:review-adr` is the route. work-item:0196's Requirements bullet
restating version+digest addressing is corrected in step, and its `ensure-playwright.sh`
and `Node >= 20` references retired.

#### 4. Final state assertion

**File**: `tests/unit/tasks/test_call_site_migration.py`
**Changes**: Assert `skills/design/` contains no `.sh` file, so a future reintroduction is
caught. This is only true once this plan lands — the sibling plan leaves
`ensure-playwright.sh` behind by design.

#### 5. Follow-up work items

**Changes**: Two this plan surfaces and does not fix:

- **The pinned runtime ages silently.** `playwright-core`, Node and Chromium are pinned by
  exact version and hash, and Phase 5 §8's reuse path skips the fetch-and-verify entirely
  while the pins are unchanged — so nothing re-evaluates them for known vulnerabilities,
  and the only route to a newer engine is a human bumping a pin. `cargo-deny` covers Rust
  crates only. The follow-up adds a scheduled guard that fails, or opens an issue, when a
  pinned revision exceeds a stated age or appears in an advisory feed, and records the pin
  in `RELEASING.md` as a security-relevant dependency with an owner and a maximum age.
- **`design.browser_path` is settable from committed team config.** `.accelerator/config.md`
  is repo-tracked, and Phase 7 §2 passes the value into
  `chromium.launch({ executablePath })` — so opening an untrusted repository and running the
  inventory skill executes a binary that repository named. `visualiser.editor` sets the same
  precedent, so this extends an existing hazard rather than inventing one, but it extends it
  to a path executed automatically by a skill designed to be pointed at unfamiliar projects.
  The follow-up restricts the key to the personal (gitignored) level or refuses a value
  resolving inside the repository, and audits the visualiser keys alongside.

### Success Criteria

#### Automated Verification

- [ ] Failing test first for the final-state assertion
- [ ] `mise run test:integration:config` passes with `_EXPECTED_CONFIG_SUITES` unchanged at
      15 and `test-design.sh` absent from the discovered suites
- [ ] `mise run test:unit:build-system` passes
- [ ] `mise run lint:scripts:exec-bits:check` exits 0
- [ ] `mise run docs:check` exits 0
- [ ] **No `.sh` file remains under `skills/design/`**
- [ ] `git status --porcelain -uall` is clean after a tree materialisation in a dev
      checkout, so `bin/trees/` is genuinely ignored
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] The docs site builds and every design page's links resolve
- [ ] A fresh plugin install with no system Node completes an inventory run
- [ ] The ADR-0060 amendment is accepted, and work-item:0196 no longer describes a scheme
      the code does not implement

---

## Testing Strategy

### Unit Tests

- Tree materialisation in `cli/launcher/` against synthetic tarballs: rejection before
  extraction, the entry-type allowlist's full rejection set (including PAX/GNU long-name
  records and duplicate-path entries, which is where tar CVEs live), attestation and table
  round-trip, a crash injected at each step of the publish sequence, single-flight with a
  failing winner, pointer validation, `verify`'s detection of each corruption shape
  including a rewritten `.files` row, and repair's new-generation swap against a live
  reader. These exercise `resolve/tree.rs` directly with **no signing step**, so they
  cannot inherit `tests/resolution.rs`'s `skip_if_no_minisign!` guard, which returns
  `Ok(())` with only an `eprintln!`.
- Platform classification in `cli/design/` over injected loader-path sets, per Phase 0 Q1
  — including macOS, so AC11's musl case is pinned without a container and the Mac case
  cannot regress.
- Upstream verification in `tasks/` against recorded fixtures. Node/GPG is fully
  offline-verifiable against the committed key, so it is tested for real rather than
  mocked, including the revoked-key and expired-key negatives that `VALIDSIG` alone would
  accept. The SLSA check contacts a transparency log, so its runner is injected and both
  branches asserted — and the plan records that the attestation's *content* is not verified
  in tests.

### Integration Tests

- End-to-end resolution against a `MockServer` and a real minisign keypair, following
  `cli/launcher/tests/resolution.rs:41-199`.
- An assembly round trip: a synthetic tree through the real assembly path, a manifest
  through the real `build_manifest`, signed with a test key, resolved by the launcher's tree
  resolver — so the two halves of the artifact contract are verified together rather than
  only by hand.
- Container fixtures: Node-absent glibc (AC6), musl/Alpine (AC11), and
  bundled-browser-unavailable with `design.browser_path` set (AC12), in their own CI job
  with a preflight that **fails rather than skips**. The artifact-serving component and its
  binding must be named — the launcher's `MockServer` is a `#[cfg(test)]` type bound to
  loopback and is not reachable from a container nor callable from an invoke task. AC11
  needs no artifacts at all, since the platform probe downgrades before any resolution.
- The retained `lib/*.test.js` suites plus `test-run.js`, moved into the container lane the
  sibling plan could not provide, where a runtime exists and zero skips can be asserted
  across the whole set.

### Manual Testing Steps

1. Time a warm executor invocation before and after Phase 4 and confirm no regression
   against work-item:0186's bootstrap target.
2. Corrupt a file in a sealed tree, run `accelerator cache verify`, then `repair`, and
   confirm a working crawl.
3. Run a full inventory crawl on a machine with no system Node and compare artefacts
   against one with Node installed.
4. Point `ACCELERATOR_CACHE_DIR` at a shared location and confirm the ownership check
   refuses it.

## Performance Considerations

Three budgets are load-bearing.

**The warm path.** Work-item:0186 took warm bootstrap from 125ms to ~30ms. Per-exec
re-verification of a 294MB artifact set would spend 16–33 seconds per crawl re-hashing
immutable bytes, which is why ADR-0060 exempts trees. The hit path is therefore local reads
plus stats and loads no manifest, which also keeps a populated cache working offline; the
per-entry file table is deliberately *not* on that path, so its cost does not scale with an
artifact's file count. Because the launcher exports tree paths on every dispatch, that cost
is charged to `accelerator vcs guard` — a PreToolUse hook — and every SessionStart hook, not
only to design, so the export is confined to the external-dispatch path and driven from the
compiled-in artifact set rather than a directory scan.

**Launcher binary size.** `bin/accelerator:352-354` minisign-verifies the whole launcher on
every warm start, so the `tar` + `flate2` addition is a per-invocation latency term charged
to every sub-binary and every hook. Step 4b §1 derives the budget and the asserted ceiling,
including why the per-MB slope is measured rather than back-derived from work-item:0186's
non-method-comparable figure.

**First run.** ~294MB per platform. On the default cache root — inside the versioned plugin
tree — a plugin upgrade discards it, and this plugin pre-releases often.
`ACCELERATOR_CACHE_DIR` is the escape, and content-addressed naming is what makes it
actually work: the driver and browser change only when the pinned `playwright-core` changes,
so an upgrade that leaves the pin alone resolves the same digest and hits.

**The release job.** It runs the whole pipeline twice per stable release, so roughly 2.4GB
of assembly and upload, on a `macos-latest` runner. Phase 5 adds a `timeout-minutes` and a
whole-job disk assertion, and removes the duplication itself: the second pass reuses the
first's archives by digest, and an unchanged pin triple skips the upstream fetch entirely.

## Migration Notes

Existing installs carry a populated
`${ACCELERATOR_PLAYWRIGHT_CACHE:-$HOME/.cache/accelerator/playwright}/<sha8>` namespace that
nothing will read after Phase 7. It lives outside the plugin tree so plugin pruning will not
reclaim it. Phase 7's documentation names the path and states it is safe to delete; no
automated removal is added, consistent with not building destructive-op UX where the
filesystem makes recovery trivial. `accelerator cache prune` reports it with its measured
size and the exact command, so the reclamation is discoverable at the moment a user is
already thinking about cache space.

`SKILL.md` Step 4's `ensure-playwright.sh` bootstrap and its
`ACCELERATOR_DOWNGRADE_REASON=` stderr protocol are replaced in Phase 7 §6, along with the
residual `Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/**/scripts/*)` `allowed-tools` rules the
sibling plan deliberately left in place.

## References

- Work item: `meta/work/0196-accelerator-design-inventory-gap-tooling-cli.md`
- Sibling plan (prerequisite):
  `meta/plans/2026-08-11-0196-design-cli-migration.md`
- Superseded plan and its three-pass review:
  `meta/plans/2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli.md`,
  `meta/reviews/plans/2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli-review-1.md`
- Research: `meta/research/codebase/2026-08-11-0196-design-cli-implementation-surface.md`
- ADR-0057 (browser automation as a glibc-only capability), ADR-0059 (build-time assembly
  of vendored browser artifacts), ADR-0060 (launcher-resolved tree artifacts — **needs an
  amendment**, see Removal sweep §3)
- Release-pipeline template:
  `meta/plans/2026-07-06-0165-multi-binary-distribution-and-release-pipeline.md`
