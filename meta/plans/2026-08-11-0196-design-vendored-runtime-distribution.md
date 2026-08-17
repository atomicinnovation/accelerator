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
blocked_by: []
derived_from: ["codebase-research:2026-08-11-0196-design-cli-implementation-surface"]
relates_to: ["plan:2026-08-11-0196-design-cli-migration", "work-item:0208"]
supersedes: ["plan:2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli"]
tags: [rust, design, playwright, launcher, release-pipeline, tree-artifacts, distribution]
revision: "9d8b9d862daf092e14986e91130c607ed3d06d7d"
repository: "accelerator"
last_updated: "2026-08-17T10:37:32+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# accelerator-design: Vendored Runtime Distribution Implementation Plan

## Overview

Vendor the Playwright runtime so the design tooling stops depending on a system
Node.js. The launcher gains the ability to resolve directory-tree artifacts alongside
the single-file sub-binaries it already fetches; the release pipeline gains a
build-time assembly step that constructs the driver bundle and the browser from
verified upstream inputs and publishes them under the project's own signing key; and
the executor swaps onto them.

This is the second of two plans against work-item:0196. The first —
`plan:2026-08-11-0196-design-cli-migration` — **is implemented and merged** (thirteen
commits, PR #64, validated *partial* on 2026-08-13). It created the
`accelerator-design` sub-binary and ported `run.sh` to Rust, deliberately leaving
`ensure-playwright.sh`, the lockhash namespace and the system Node prerequisite in
place. This plan removes them.

### What this rewrite changed, and why

The plan was written before its sibling was implemented, and the implementation moved
the ground under it. This revision rebuilds every factual claim against the merged
tree. Nine of the plan's assertions were false and three collisions were new; the
design survives, its edit set did not. The corrections are recorded inline rather
than in a changelog, but the classes were: `scripts/test-design.sh`'s contents (moved,
not left behind), the suite-floor arithmetic (off by one, in the direction that
reddens CI), a `cli/Cargo.toml` comment quoted verbatim that does not exist, and three
guards added after the plan was written that this plan's deletions now trip.

**Phase 0 was lifted out into work-item:0210, which is now closed.** It carried four
questions that were answered wrongly on paper twice across three review passes, and its
own deliverable was a spike rather than shipped behaviour, so it became a spike work item
following the precedent work-item:0205 set for the warm-dispatch measurement method. All
four are settled against prototypes on real hosts, and the sections they govern now
specify a mechanism rather than a candidate. Every one of the four candidates the plan
previously carried was falsified: the libc probe could not have worked at all
(a static-musl binary has no `PT_INTERP` to read), the reaper's pid gate repeated a
failure already recorded in `meta/notes/`, the seal was not the discriminator it was
described as, and the trust-root shape had an in-repo precedent the plan had not found.

**Phases are renumbered.** They previously kept the numbers they carried in the
superseded plan (4, 5, 7) so that cross-references to the sibling stayed valid; the
sibling has merged, so the gaps now cost more than they save. Old Phase 4 is Phase 1,
old Phase 5 is Phase 2, old Phase 7 is Phase 3, and `Step 4a`/`4b`/`4c` are `Step
1a`/`1b`/`1c`. Anything in the merged sibling plan referring to "the sibling plan's
Phase 7 §6" means Phase 3 §6 here.

## Current State Analysis

### What the migration left behind

Two scripts survive, and both are this plan's to remove:

| Path | Lines | Disposition |
|---|---|---|
| `skills/design/inventory-design/scripts/ensure-playwright.sh` | 367 | deleted, no replacement |
| `skills/design/inventory-design/scripts/test-ensure-playwright.sh` | 171 | deleted with it |

They are the only `.sh` files remaining under `skills/design/`. Note the location:
`ensure-playwright.sh` sits directly under `scripts/`, **not** under
`scripts/playwright/`, and reads its manifests from the `playwright/` subdirectory
(`:48-49`). With them go the lockhash namespace under
`${ACCELERATOR_PLAYWRIGHT_CACHE:-$HOME/.cache/accelerator/playwright}`, the sentinel
idempotency contract, the disk floor, the node-version floor, the sweep, and
`package-lock.json`.

`regenerate-notify-downgrade-fixtures.sh` **no longer exists** — the sibling deleted
it, and its data moved to `cli/design/tests/fixtures/notify-downgrade-messages.json`,
where `cli/design/tests/downgrade_goldens.rs:74` `include_str!`s it as a two-sided
drift test.

### Three coupled edits, enforced by guards added during validation

`scripts/test-design.sh` is now a **12-line delegation shim** that asserts nothing of
its own: it sources `test-helpers.sh`, runs `test-ensure-playwright.sh`, and calls
`test_summary`. Everything it once asserted was re-homed into
`scripts/test-skill-frontmatter-conformance.sh:407-699` — including the `SKILL=`
assignment and the `# shellcheck disable=SC2016` comment this plan previously reserved
as adjacency traps to inherit. They are not in `test-design.sh` to inherit.

The consequence is that deleting `ensure-playwright.sh` is a **three-part lockstep
edit**, and any two of the three without the third reddens CI:

| Edit | Location | Guard that fires if omitted |
|---|---|---|
| The Step 4 call site | `inventory-design/SKILL.md:126-143` | *Design script references resolve* (`conformance:619-664`) |
| The `allowed-tools` grant | `inventory-design/SKILL.md:15` | *Design script grants have call sites* (`conformance:666-699`) |
| The assertion that the grant exists | `conformance:551-553` | that assertion itself |

Both guards were added during the sibling's validation, after this plan was written.
`analyse-design-gaps/SKILL.md` has no `scripts/*` grant and no `ensure-playwright.sh`
call site, so only the one skill is involved.

### The floor arithmetic, corrected

`_EXPECTED_CONFIG_SUITES` is **15** (`tasks/test/integration.py:44`) and `scripts/`
discovers **exactly 15** suites. This plan's earlier text said deleting
`test-design.sh` "takes `scripts/` from 16 discovered suites to 15 against a floor
already at 15" and therefore needs no edit. That is stale by one — it predates the
sibling's Phase 3 retiring `test-metadata-helpers.sh`. **Deleting `test-design.sh`
lands discovery at 14 and fails `_require_suite_floor` unless the floor moves to 14 in
the same change.** The sibling plan's own Removal sweep already recorded this handoff
as "14-against-15, not 15-against-15"; the current numbers confirm it.

`test-design.sh` is not in `_REQUIRED_CONFIG_SUITES` (`:66`), so only the count holds
it.

### Four resolver properties that do not carry across to trees

- `fetcher.rs:129-151` buffers the whole body — `response.bytes().map(|body|
  body.to_vec())` — and `cache::store` (`cache.rs:81-88`) takes `&[u8]`.
  `TOTAL_TIMEOUT` is 300s *per attempt* (`fetcher.rs:12-15`), sized in its own comment
  for "a multi-MB release binary over a slow link".
- No archive crate exists in the workspace. Neither `cli/Cargo.toml`'s
  `[workspace.dependencies]` nor `cli/launcher/Cargo.toml:23-35` declares `tar`,
  `flate2` or a zip crate; the only compression crates in `Cargo.lock` arrive
  transitively through the `gix`/`jj-lib` and `octocrab` trees, not the launcher's
  graph.
- `cache.rs:118-133` renames files only. `cache::store` writes exactly two files and
  performs two renames; there is no directory staging, no recursive rename, and no
  eviction.
- Nothing seals, reaps orphan temp trees, or writes an attestation.

### Three collisions with work that landed after this plan was written

- 🔒 **The cache-root probe is now guarded against exactly what a tree path would do.**
  Work-item:0189 narrowed `verify_writable` to `pub(super)`, made a thread-local
  `PROBE_ATTEMPTS` counter its first statement, and added
  `cli/launcher/tests/resolution.rs:590-654` — four `probes_during` tests pinning exact
  per-dispatch counts plus `each_of_two_cold_misses_probes_the_cache_root_once`, an
  explicit anti-memoisation test. `locate` runs on **every** dispatch, so it must probe
  **zero** times, and `materialise`'s probe must be accounted for in those counts
  rather than discovered by a red test.
- **`BUILTIN_SUBCOMMANDS` is pinned to the clap variants themselves.**
  `tests/unit/tasks/shared/test_dispatch_coherence.py:606-611` asserts the extracted
  variant set is exactly `{"Version", "Config", "External"}`, so adding a `Cache`
  variant is a two-sided edit, not a frozenset entry.
- **A `≤ 1.1 ×` warm-path gate is not a criterion one can simply write down.**
  Work-item:0205 exists because that gate shape was specified three times in prose for
  the 0189/0169 lane and found unsound each time — an instrumented launcher that could
  not reach the verified path, a budget whose terms closed by construction, a sample
  count inherited from a four-fold effect and applied to a ten-percent margin. This
  plan's warm-path criterion is the same shape and must use 0205's settled method.

### What the design crates actually look like

The sibling's delivered layout differs from what this plan assumed in one place that
matters. `cli/design/src/executor/` is a **top-level module of the domain crate**;
`cli/design/src/runtime/` survives holding only `downgrade.rs`. So `platform.rs` still
has the home this plan gives it — `cli/design/src/runtime/platform.rs` — but the
sub-domain it joins has one sibling, not two.

Also delivered, and load-bearing here: `runtime_is_installed`
(`cli/design-cli/src/executor.rs:118-122`) is the single lockhash-namespace
precondition Phase 3 replaces; the child's environment is assembled in one vector at
`executor.rs:139-156`, shared by spawner and exec client, which is the single place a
resolved browser path is threaded; and `cli/design/tests/fixtures/public-api.txt` is a
cargo-public-api snapshot that moves whenever `cli/design` gains a public item.

## Desired End State

On a machine with no system Node.js, the inventory skill's Playwright path fetches a
driver bundle and a `chromium-headless-shell` tree from the project's own release
host, verifies both before extraction, seals them, and drives a headless crawl. The
release pipeline assembles both artifacts in CI from inputs verified against their
publishers' own signatures. No `.sh` file remains under `skills/design/`.

Verified by: `mise run` exits 0; `manifest.json` carries an `artifacts` map beside
`binaries`; `skills/design/` contains no `.sh` file; a container fixture with Node
absent from `PATH` completes a Playwright-driven inventory.

### Acceptance criteria in scope

Fully: **AC6**, **AC7**, **AC8**, **AC9**, **AC10**, **AC11**, **AC12**, **AC13**,
**AC14**, **AC16**.

Completing what the sibling started: **AC1** (the `notices` subcommand — the one
member of the recorded seven-subcommand set that does not exist), **AC2** (the bundled
path's envelopes), **AC3** (the bootstrap step's call site), **AC4**
(`ensure-playwright.sh` and the last floor movement).

### Key Discoveries

- The manifest extension is additive by construction. `manifest.rs:1-3` states
  "Unknown additive fields are ignored"; there is no `deny_unknown_fields` anywhere;
  `manifest.rs:223-231` is a dedicated test feeding `"future_field": 42`; and the gate
  rejects only strictly-greater versions (`manifest.rs:89-94`). No `SCHEMA_VERSION`
  bump, and no flag day.
- `cache::find`'s prefix scan (`cache.rs:51-73`) will *see* a directory in the same
  root and rejects it only because no `.minisig` sidecar exists. Tree entries need a
  distinct subdirectory, and a tree's sidecars must never be named `*.minisig`.
  `cache.rs:56` aborts the **whole scan** on one non-UTF-8 entry — the `?` is on an
  `Option` inside `find` — so new on-disk names stay ASCII or a stray filename turns
  every single-file resolution into a miss.
- `cache.rs:1-6` records that "the checksum in the name lets a hit resolve offline".
  The single-file warm path never loads the manifest, and `load_manifest`
  (`resolve/mod.rs:116-135`) is two HTTPS GETs plus a signature verification, called
  only on a miss. A tree hit must hold the same property: each executor invocation is
  a fresh launcher process and a crawl makes 100–200 of them.
- `@actions/glob`'s `*` does not cross `/` — `tests/unit/tasks/test_workflows.py:161-168`
  expands it to `[^/]*` explicitly, with a comment saying so — and
  `test_attest_globs_cover_every_published_asset` (`:207-221`) derives its expectation
  from `tasks.github._release_uploads`. Tree archives must stay flat in
  `dist/release/`.
- The launcher's redirect allowlist is `github.com` plus `*.githubusercontent.com`
  only (`fetcher.rs:17-18, 31-33`), matched at a dotted-label boundary.
- `tasks/lint/cli.py:7` passes `--workspace --all-targets --all-features` and
  `tasks/test/cli.py:13` passes `--all-features`, the latter deliberately to enable
  `bash-parity` (documented at `tasks/test/cli.py:26`). **Any non-default cargo feature
  added to a `cli/` crate is on during `mise run cli:check` and `mise run
  test:unit:cli`** — so the test trust root is a second `[[bin]]`, not a feature
  (work-item:0210 SQ-4). `cli/vcs-adapters/Cargo.toml:27-33` is the precedent and states
  the reason; `cli/launcher/Cargo.toml:17-21` already carries the fixture-bin convention.
  A build-time environment variable was rejected: without
  `cargo:rerun-if-env-changed` — which appears nowhere in this repository — a binary once
  built with a substituted key keeps it after the variable is unset, silently and
  durably.
- `cli/launcher/src/launch/outbound/mod.rs:21-47` reads `ACCELERATOR_<SUB>_BIN` as a
  dev-override input and returns the path **unverified**. `ACCELERATOR_LAUNCHER_BIN` is
  not a free name to export.
- `tasks/git.py:35-52` runs a bare `git push --atomic` with no credential helper and
  no authenticated remote URL, so the release push depends entirely on the credentials
  `actions/checkout` persists — and those are the **GitHub App token**
  (`main.yml:475-478`, `:585-588`), not `GH_TOKEN`. Neither checkout sets
  `persist-credentials`, so it defaults to `true`.
- `tasks/` has no HTTP client at all: `pyproject.toml:10-26` declares no
  `requests`/`httpx`/`urllib` dependency, and every network operation shells out to
  `gh`. `gpg` is not pinned in `mise.toml`, whose `[settings] lockfile = true` (`:46`)
  hash-pins aqua/ubi artifacts only.
- `tasks/signing.py:24-43` signs with `minisign -S`, **no `-H` prehash**, one file per
  invocation, under a 120-second per-file timeout sized for an 8MB binary.
- `_release_reverifies` is built at `tasks/github.py:344`, *before* the `try` — so a
  manifest `KeyError` raises outside the delete envelope. The `except Exception` arm
  that runs `gh release delete --cleanup-tag` (`:359-364`) is reachable only from
  inside the upload/re-verify envelope, which is exactly where a large-asset timeout
  lands.

## What We're NOT Doing

- Anything the sibling plan owns and delivered: the `design` sub-binary, the five
  ported subcommands, the `run.sh` port, the metadata-script retirement, the
  registration checklist.
- Shipping full Chromium. `chromium-headless-shell` is 177MB across 14 files against
  297MB across 327, and the daemon launches headless (`lib/daemon.js:132-140`).
- Bundling `ffmpeg`. `browsers.json` marks it install-by-default but it serves video
  recording.
- A musl driver bundle. Playwright publishes none, and its Chromium builds are
  glibc-linked.
- Cross-plugin-version artifact sharing beyond what content addressing gives for free,
  or bespoke cache eviction beyond a `prune` verb.
- A formal legal review gate on the release.
- Automated removal of the abandoned legacy Playwright cache on user machines.
- Fixing work-item:0208. Wiring `test:integration:design-automation` into a CI job is
  that item's, not this plan's — but this plan's container lane is the approach 0208
  names as a candidate, so the two are related explicitly rather than proposing the
  same job twice. See the Removal sweep.
- Closing the sibling validation's D4 (`dup2(read_fd, IDENTITY_FD)` is a POSIX no-op
  when `read_fd` is already 3, leaving `FD_CLOEXEC` set). It is unowned and real, but
  it is executor-adapter work with no bearing on the runtime's provenance.

## Implementation Approach

Three phases plus a removal sweep. Work-item:0210 is closed, so nothing blocks Phase 1
or Phase 2 from starting.

```
Phase 1 ──┐
Phase 2 ──┴──> Phase 3 ──> Removal sweep
```

Phase 1 (launcher) and Phase 2 (pipeline) are independent of each other and each
leaves the tree green on its own; Phase 3 needs both, because it consumes artifacts
Phase 2 produces through the resolver Phase 1 builds. The asset-name convention is the
one thing both halves must agree on before either merges, which is why Phase 1 pins it
in a fixture both sides read (Step 1a §1).

Decisions taken during planning, so no phase carries an open question beyond the four
the spike owns:

- **Tree addressing** is by content digest, platform and generation, with the release
  version in a separate pointer file, per **ADR-0061** — which supersedes ADR-0060
  precisely because ADR-0060 said "addressed by release version and digest".
- **The launcher owns materialisation, the design binary owns the decision.** ADR-0061
  puts the embedded key in one holder; **ADR-0062** puts the ordering and the downgrade
  vocabulary in `accelerator-design`. Phase 3 §3 splits them by cost.
- **`disk-floor-not-met` and `cache-unwritable` are retained** in the downgrade
  vocabulary, not dropped. Both still arise and both are now *more* likely.
- **The archive format is `tar.gz`, flat in `dist/release/`**, because the attest globs
  do not cross `/`.

---

## Phase 1: Launcher tree artifacts

### Overview

Teach the resolver to fetch, verify, extract and seal directory-tree artifacts, and
add the repair path that replaces the self-healing trees are exempt from. Tested
against a synthetic tarball — no design consumer exists yet.

Three internally staged steps, each of which should compile and test green on its own.

### Step 1a: Manifest `artifacts` map and streaming fetch

#### 1. Manifest shape

**File**: `cli/launcher/src/launch/outbound/resolve/manifest.rs`
**Changes**: A new `artifacts: BTreeMap<String, ArtifactEntry>` beside `binaries`
(`:26-32`), `#[serde(default)]`. `SUPPORTED_SCHEMA_VERSION` stays `1` (`:13`). The
all-zeros sentinel digest (`:16-17`) carries over for platforms where an artifact is
deliberately absent, reusing `bare_sha256`'s existing handling (`:55-65`).

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

So `binaries` keeps `PlatformEntry` (`:42-46`) genuinely untouched, and artifacts get
their own entry type. The three sizes are **required, not `#[serde(default)]`** — a
defaulted 0 would silently disable the download cap and the decompression-bomb
ceiling, which is the failure mode a default exists to avoid. Additivity is
unaffected: an older launcher never reads `artifacts` at all, and a newer one reading
a manifest without the key gets an empty map.

The asset-name convention is `accelerator-{key}-{platform}.tar.gz`, mirroring the
single-file `accelerator-{token}-{platform}` rule pinned in one commented place at
`resolve/mod.rs:144-147`. Phase 1 builds the consumer and Phase 2 the producer, in
separate changes, so the convention is pinned in one artefact both sides read:
`cli/launcher/tests/fixtures/manifest.example.json` — today 27 lines carrying a single
`visualiser` entry — gains an `artifacts` block in this phase, asserted from
`manifest.rs`'s golden test (`:137-150`) here and from
`tests/unit/tasks/test_manifest_contract.py` in Phase 2. Without that, a key-name or
extension disagreement surfaces only in Phase 3's container fixture, after both halves
have merged.

#### 2. Streaming download

**File**: `cli/launcher/src/launch/outbound/resolve/fetcher.rs`
**Changes**: `try_get` currently ends in `response.bytes().map(|body| body.to_vec())`
(`:146-150`) — the body buffered, transiently twice. Add a
`get_to_writer(&self, url: &str, sink: &mut impl Write)` that copies from the response
reader, leaving `get` as a thin wrapper for the existing small-asset callers.

**The sink must be owned inside the retry loop.** `get` (`:109-127`) retries up to
`MAX_ATTEMPTS` (3), and today each attempt is safe only because `try_get` returns a
fresh `Vec<u8>` — a failed attempt leaves nothing behind. Writing into a
caller-provided sink breaks that invariant: an attempt that fails partway has already
written bytes, and the next appends the full body after them. The sha256 would catch
the result, so nothing unverified is extracted, but the retry loop could never
succeed — a transient blip on a large transfer would become a permanent,
unrecoverable failure presenting as a checksum mismatch. So the streaming path creates
and truncates the temp file at the start of *each* attempt (or `set_len(0)` plus seek
to 0) and resets the incremental digest state with it.

**The deadline is a throughput floor, not a number picked once.** `TOTAL_TIMEOUT`'s
300s per attempt was sized for a multi-MB binary. It governs the *compressed archive*,
whereas the ~294MB figure is the uncompressed tree — so the value is derived from
Phase 2's measured archive sizes, expressed as "sized for X MB at ≥N KB/s sustained",
and recorded in the constant's doc comment with its reasoning as the existing one
does. Make it a per-request override via `RequestBuilder::timeout()` rather than a
second `Fetcher`: each `Fetcher` builds a `reqwest::blocking::Client` (installing the
rustls provider and a background runtime thread) at `fetcher.rs:81-101`, and the
production resolver is already constructed lazily per `resolve()` call
(`main.rs:56-71`) — so a warm resolution builds none at all, and a second client per
dispatch would undo that.

**A stalled transfer must fail fast.** `fetcher.rs:12-14` records that blocking
reqwest has no idle timeout and that the total deadline is "the only bound on a
slow-but-progressing transfer". Enlarging that deadline widens the window in which a
connection stalled at byte one is indistinguishable from a slow one — three times
over, inside a tool call with no progress output and no cancel. The copy loop
therefore enforces a progress floor (abort if fewer than N bytes arrive in M seconds),
so the large deadline bounds legitimate slow transfers while stalls fail quickly. Both
numbers go in the doc comment.

The **mechanism** must be named, because a plain byte-counting check between reads
cannot fire while a read is blocked — which is the stall case. Blocking reqwest
exposes neither an idle timeout nor the socket, so the floor needs either a watchdog
thread that drops the response to interrupt the blocked read, or a custom `Read`
wrapper over a socket with `SO_RCVTIMEO` set. State which.

The test fixture must **stop sending** rather than trickle, or it exercises the slow
path and passes without ever testing a stall. The mechanism for that already exists
and is currently unused: `cli/launcher/tests/common/mod.rs:30-32` declares
`Route::Stall(Duration)` and `:163-172` serves it, and no test in `resolution.rs`
reaches it today.

**Signature verification needs a named streaming mechanism.** sha256 streams
trivially, but `TrustedKeys::verifies(&self, data: &[u8], signature: &str)`
(`keys.rs:62-69`) is a contiguous-slice API, and incremental Ed25519 verification is
only possible in minisign's *prehashed* mode. `tasks/signing.py:24-43` signs with a
plain `minisign -S` and no `-H` — confirmed against the current source, not assumed —
so the form must be established before this step starts: either add `-H` for tree
artifacts, checking that the vendored `cli/verify` shim and `minisign-verify` both
accept it, or state plainly that the archive is buffered for verification and bound
the peak. Left unstated, an implementer reads a 294MB temp file back into a
`Vec<u8>`, giving the launcher a peak RSS an order of magnitude above anything it does
today, in exactly the memory-limited containers AC6 and AC11 use. A test asserts the
release pipeline's signatures are in the expected form, so a signing-flag change fails
loudly rather than degrading to a full buffer.

The download is capped at `archive_size` from the artifact's platform entry;
`uncompressed_size` and `entry_count` bound the extraction in Step 1b §2.

### Step 1b: Extraction, sealing, atomic rename, attestation, pointer

#### 1. Archive dependency

**Files**: `cli/Cargo.toml`, `cli/launcher/Cargo.toml`, `cli/deny.toml`,
`tests/integration/deny/test_launcher_feature_graph.py`
**Changes**: Add `tar` and `flate2` as workspace-pinned dependencies with
justification comments. `tar` is pinned **exactly**, not caret-bound: it is pre-1.0,
and its entry classification, PAX/GNU long-name handling and symlink semantics are
precisely what the extraction allowlist sits on top of, so a patch bump could shift
the trust boundary without a pin-edit review. `cli/Cargo.toml`'s stated discipline is
to exact-pin crates whose behaviour the workspace depends on — `clap` at `:44-45`,
`reqwest`/`rustls`/`minisign-verify` at `:56-64`, `serde-saphyr` at `:76-78` — and to
caret-bind only those documented as behaviour-stable (`regex` at `:70-72`,
`tempfile`/`rand`/`libc`/`rustix` at `:82-84`). `tar` also gets
`default-features = false`, since the default `xattr` feature adds a transitive edge
that mode masking makes pointless.

`flate2` is pinned explicitly to its pure-Rust backend:

```toml
flate2 = { version = "1", default-features = false, features = ["rust_backend"] }
```

That pin is load-bearing, not stylistic. `flate2`'s alternative backends (`zlib`,
`zlib-ng`, `zlib-rs`) pull `libz-sys`/`zlib-ng-sys`, which need a C toolchain and would
break the fully-static musl cross-build ADR-0046 depends on. Because Cargo unifies
features across the workspace, a *future* crate enabling a C backend would pull it
into the launcher silently — so `libz-sys`, `zlib-ng-sys` and `zlib-sys` join `_ABSENT`
in `tests/integration/deny/test_launcher_feature_graph.py:24-31`, which already
parametrises its absence assertion over that tuple for exactly this regression class.

**The binary-size budget, re-derived.** This plan previously quoted a
`cli/Cargo.toml` comment saying launcher size should be reconsidered "if it exceeds a
few hundred KB". **No such text exists.** What the file actually says, at `:182-184`,
is:

```toml
# The bootstrap hashes the whole launcher on every invocation, so binary size is
# a per-call latency term and the cold-fetch payload. `strip` trades symbol names
# for size; `lto = "thin"` trades release build time for size.
```

That is a rationale with no threshold, which makes the budget this plan must set a
genuinely new decision rather than a restatement. The budget is expressed in time and
converted to a size ceiling, and the per-MB slope is **measured, not back-derived**.
Work-item:0186's figures cannot supply it: its ~2.3ms-for-8MB minisign number comes
from a pre-change 20-run bash loop that 0186's own Validation Results declare "not
method-comparable" to its interleaved medians, and its ~6.8ms composition figure
bundles shim process startup with the read. Attributing all of it to size gives
0.3ms/MB; attributing half gives ~0.45ms/MB and a ceiling nearer 2MB. Deriving a
marginal cost from one point that includes fixed costs is not sound.

So the slope is obtained directly, with work-item:0205's settled measurement method:
verify two padded launchers of known differing size on the same host and take the
difference. A 1ms budget then converts to a real ceiling.

Two notes on the gate. `tar` plus `flate2`/`miniz_oxide` realistically add a few
hundred KB, so a multi-MB ceiling is a weak tripwire — the assertion is on the
measured delta plus a small margin, not on the headroom. And the ceiling is an
**absolute per-target size** checked against the cross-compiled artefacts in the
release lane, recorded beside the other pins in `tasks/shared/paths.py` with its
derivation in the comment, because a ratio gate would need a stored pre-Phase-1
baseline that `mise run test:*` has nowhere to keep and cross-compiled binaries that
only exist after `build.cli_cross_compile`.

The backend's real consequence is the other direction: `miniz_oxide` (the pure-Rust
default) inflates materially slower than a zlib-ng build, and the cold path inflates
~294MB. So record decompression throughput over a real archive alongside the size
figure, and if `rust_backend` proves unacceptably slow the resolution is a faster
pure-Rust backend (`zlib-rs`, if it can be shown to need no C toolchain), never a
`*-sys` crate.

#### 2. Tree materialisation

**File**: `cli/launcher/src/launch/outbound/resolve/tree.rs` (new)
**Changes**:

Layout — a dedicated subdirectory so `cache::find`'s prefix scan (`cache.rs:51-73`)
never sees a tree, content-addressed so an unchanged artifact is one tree however many
plugin versions want it, per-platform so a shared cache root cannot mix incompatible
trees, and generation-suffixed so a rename target is always fresh:

```
<cache_root>/trees/<name>-<platform>-<sha256>-<gen>/        the sealed tree
<cache_root>/trees/<name>-<platform>-<sha256>-<gen>.sealed  the attestation
<cache_root>/trees/<name>-<platform>-<sha256>-<gen>.files   the per-entry table
<cache_root>/trees/<name>-<platform>-<version>.ref          the pointer
```

All names are ASCII — `cache.rs:56` aborts the *entire* scan on one non-UTF-8 entry,
so a stray name here would turn every single-file resolution into a miss — and none is
named `*.minisig`. The **attestation** is small and fixed-size (archive digest,
platform, release version, entry count, and a digest of the table) and is the only
sidecar the hit path opens. The **table** carries one `(path, mode, size, sha256)` row
per entry, or a link target for a symlink, and is read only by `verify` and `repair`;
keeping it out of the attestation is what stops the hit path's cost scaling with the
driver tree's ~490 files. The **pointer** names a directory rather than a digest,
which is what lets a repair swap one generation for another atomically.

The attestation and the pointer each carry a `format_version`, and the tree directory
name carries a layout version alongside the generation. Extraction and sealing policy
— the entry-type allowlist, mode masking, the `0444`/`0555` seal, the table's own
shape — is launcher-version-specific and is *not* covered by the archive digest, yet
content addressing means a newer launcher routinely adopts an older launcher's tree
from a shared cache root. Without a layout version a policy fix would be silently
inherited rather than applied, and `verify` would pass because it checks against the
older table. The same "unknown additive fields ignored, higher version refused"
discipline `manifest.rs:1-3` documents applies.

The generation is the load-bearing addition. Because every materialisation picks a
fresh one, `rename(2)` never lands on an existing target — so there is no
already-present branch to get right, no need to distinguish a concurrent winner from a
crash leftover at rename time, and a repair can build a complete replacement beside a
tree a live daemon is still reading.

`trees/` is created `0700`. The cache root, every generation directory and every
sidecar must be owned by the effective uid and be neither group- nor world-writable;
anything failing that is treated as absent rather than trusted. This is a check the
existing resolver does not make: `cache_root.rs` performs **no ownership check, no
directory-mode check and no symlink guard** — its only permission signal is
`probe_writable_and_executable` (`:122-142`) succeeding, and the single-file path
tolerates that because it re-verifies on every hit. Trees do not, and ADR-0060's threat
model assumes the cache lives under the user's own home directory while
`ACCELERATOR_CACHE_DIR` — which this plan actively recommends — can break that
assumption. So it is enforced here rather than assumed, and documented as requiring a
private, user-owned path.

**The attestation is signed**, settled by work-item:0210 SQ-2. Without it the checks
below are all *local and self-referential* — an attestation whose digest matches the
digest in its own directory name proves nothing about provenance — so the attestation
carries the manifest's minisign signature over the archive digest, and `locate` verifies
it under the embedded release key. Measured cost is 51.7µs median cold-process
(43.5µs warm in-loop, p99 58.5µs) for one Ed25519 verify over a 244-byte attestation in
the shipped release profile: **0.17%** of work-item:0186's 29.92ms warm bootstrap, or
0.35% for both trees. That is the hit path's only cryptographic anchor.

The `0444`/`0555` seal is **not** an additional discriminator, and must not be described
as one. `tar` and `unzip` both preserve read-only modes exactly, and these artifacts are
`tar.gz`, so `tar xzf` into the cache root reproduces the seal perfectly; only a git
checkout cannot, because git records `100644` for a `0444` file. The seal check is
retained because the `stat` in step 3 already happens and it therefore costs no extra
syscall, but it detects inconsistency rather than establishing trust.

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

🔒 **`locate` must not probe the cache root.** `verify_writable` is `pub(super)` with a
thread-local attempt counter as its first statement (`cache_root.rs:88-102`), and
`resolution.rs:590-654` pins exact per-dispatch probe counts with an explicit
anti-memoisation test. `locate` is a pure query that runs on every dispatch — including
`accelerator vcs guard`, a PreToolUse hook — so it calls `candidate` (`:56-72`, selection
only, no filesystem access) and never `verify_writable`. A criterion asserts the probe
count is unchanged across a dispatch that resolves a tree.

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
   every tree about to be materialised. A shortfall emits `disk-floor-not-met` before a
   single byte is fetched.
4. Stream the archive to `trees/.tmp-<gen>.archive`, truncating the file and resetting
   the incremental digest at the start of each attempt, under Step 1a's deadline and
   progress floor.
5. Verify sha256 and minisign over the archive. On failure, remove the temp archive and
   return the cause — nothing has been extracted.
6. Extract into `trees/.tmp-<gen>/` under the entry rules in §4 below, **computing each
   entry's sha256 inline as it is written**, so the table costs no second pass over
   ~294MB.
7. Seal bottom-up: `0444` for files, `0555` for files the archive marks executable,
   directories left owner-writable. Symlinks are walked with `symlink_metadata` and
   their permissions left alone — `set_permissions` follows a link and would re-mode
   the target — and recorded in the table by link target rather than by digest.
8. Write `.tmp-<gen>.files`, then `.tmp-<gen>.sealed` carrying its digest.
9. `rename(2)` the temp directory into place, then the two sidecars. Fresh by
   construction, so no collision case arises.
10. Publish the pointer atomically, last. Until then the generation is invisible to
    `locate` and reclaimable by the reaper, so a crash at any earlier step leaves only
    garbage rather than a half-trusted tree.

`materialise` **does** probe the cache root, once, before step 4 — it is about to write
hundreds of megabytes there. That probe is accounted for in the 0189 counter's
expectations rather than left to surface as a red test: the criteria below state the
expected count for a cold tree materialisation explicitly.

**Single-flight**: one lock directory per `(name, platform)` under `trees/`, reusing
the PID-owner staleness discipline `bin/accelerator` implements for its own bootstrap
lock — but not its waiter budget, which resets on every live-owner observation and so
waits unbounded. Here the wait carries an explicit deadline derived from the fetch
deadline plus an extraction allowance, and the loser waits on the **lock**, never on
the pointer: a winner that fails writes no pointer, so a pointer-waiter would hang
forever. On acquiring the lock the loser re-runs `locate` and materialises only if
still needed; on deadline expiry it emits `artifact-unavailable` rather than blocking a
crawl. The lock is released by a `Drop` guard on every path.

Without this, two cold invocations each stream ~294MB, hash it, verify it, extract it
and seal it — ~588MB of transfer and ~1.2GB of transient disk, one copy of which is
then discarded. `cache::store` needs no such guard at ~8MB; at this size the
duplication is the dominant cost of a first run.

**The in-use signal is a shared `flock` lease**, settled by work-item:0210 SQ-3. A lease
file inside each generation is opened by the launcher, held `LOCK_SH`, and has its
`FD_CLOEXEC` cleared so the open file description is inherited through the `exec` into
the design binary and on into the detached daemon. The reaper and `prune` probe with
`LOCK_EX | LOCK_NB`; `EWOULDBLOCK` means a live holder. The kernel is the liveness
oracle, so there is no pid, no start time and no sentinel protocol to get wrong, and a
crashed holder releases with no cleanup code and leaves no stale state. A shared lease
admits concurrent crawls and stays held until the last holder dies, which is exactly the
"any generation a live process still holds" property `repair` needs.

The pid-and-start-time gate this section previously specified is not merely
data-source-less but a **repeat of a documented failure**:
`meta/notes/2026-05-19-playwright-daemon-owner-pid-ephemeral-shell.md` records the daemon
shutting down with `owner-exited` seconds after every bootstrap because `--owner-pid $$`
bound it to an ephemeral shell, and its resolution was to stop using the pid.

The lease is a **second, distinct lock** from `cli/design-adapters/src/lock.rs`, not an
extension of it. That module's descriptor deliberately does *not* leak into the daemon
(`:1-10`), because holding it for the daemon's lifetime would falsely report
`another-launcher-running` — precisely the property a lease must invert.

A minimum retention window does **not** replace the lease: it cannot distinguish "old
but in use" from "old and abandoned", so it either reaps a live daemon's tree — defeating
the reason generations exist — or retains for ever. An age backstop is retained only for
generations carrying no lease file, i.e. those left by a launcher predating this
mechanism.

Orphan reaping: `cache.rs:130` removes a single temp file on a failed rename. Here the
residues are larger and more varied — a partial temp archive, a partial temp tree, and
a fully-materialised generation no pointer references (left by a crash between steps 9
and 10, or superseded by a repair). `reap_orphans` reclaims all three, with an age
backstop beyond the fetch-plus-extract deadline so nothing leaks permanently. It runs
from `materialise` and from `cache prune`, never from `locate`, which stays a query
with no side effects.

#### 3. Ports, errors, and the documented divergence

**Files**: `cli/launcher/src/launch/outbound/resolve/mod.rs`,
`cli/launcher/src/launch/core.rs`, `cli/pup.ron`
**Changes**: ADR-0061 carries forward ADR-0060's framing of the two integrity models as
"a documented difference rather than an oversight", which means it must actually be
documented in `resolve/`. Extend
the module doc comment to state both models and which applies where.

Trees are **not** routed through `ResolveBinary::resolve` (`core.rs:227-235`) — that
method's per-exec re-verify is precisely what they are exempt from, and its contract is
name → executable path for `exec` (`core.rs:238-242`, handed straight to
`Command::new(...).exec()`). But refusing that port leaves the second artifact class
with no port at all: `resolve/tree.rs` would be an *outbound adapter* module called
directly from `main.rs` and from the `cache` built-in, while `launch::core` holds both
existing driven ports, the error taxonomy and the `run_external` use case — and
`cli/pup.ron:25-39` pins `accelerator::launch::core` to std, `kernel::Error` and self
imports. **No pup rule constrains `launch::outbound` at all**, so the launcher's one
enforced architectural rule would cover one of its two resolution paths.

So `launch::core` declares **three narrow ports**, not two broad ones:

- `LocateSealedTree` — pure lookup, no network, returns `Option<TreePath>`. This is the
  only one the dispatch path may call.
- `MaterialiseTree` — network plus filesystem, called only by `ensure` and `repair`.
- `VerifyTree` — a read-only walk returning a per-entry discrepancy report.

A single `ResolveArtifactTree` meaning "find-or-materialise" would put the forbidden
behaviour one argument away from the warm path, when the whole design rests on dispatch
never fetching; and a `VerifyArtifactTree` meaning "walk, and repair" would put a query
and a destructive mutation behind one abstraction, blunting the very seam the ports
exist to provide. With the split, `repair = verify → materialise → repoint → reap` is a
**use case in `launch::core`** over the three ports, mirroring how `run_external`
(`core.rs:246-255`) sits over `ResolveBinary` + `ExecBinary` — so the interesting
decision (what to do when verification fails) sits in front of the adapter rather than
inside it.

Tree-specific `ResolutionError` variants — extraction, path-escape, seal, attestation,
pointer — join the existing sixteen (`core.rs:38-91`) rather than folding everything
into `Cache { path, detail }`. Each states its `Refusal`/`Failed` mapping explicitly,
because that choice is not cosmetic: `From<ResolutionError> for kernel::Error`
(`core.rs:167-193`) maps exactly five integrity-class variants to `Refusal` and
everything else to `Failed`, and `swallow_under_fail_safe` (`core.rs:219-224`) swallows
only `Failed` — so the mapping silently decides whether a crawl degrades or hard-fails
under `--fail-safe`. Since the pup rule pins `launch::core` to std, `kernel::Error` and
self, the discrepancy report and the attestation are plain core-owned types with serde
living in the adapter.

`tree.rs` is split along its natural seams — layout and attestation, verified download,
safe extraction, sealing — rather than being one module owning seven responsibilities,
because `cache repair` needs several of them independently. Following `cache.rs`'s
convention (`:135-164`), the sealing and permission helpers carry `#[cfg(not(unix))]`
no-op arms so the launcher still type-checks off Unix, or the module doc states that
tree materialisation is Unix-only by design; Windows is outside ADR-0062's matrix
either way, so this is about keeping the neighbouring module's discipline rather than
about supporting Windows.

#### 4. Extraction entry rules

**File**: `cli/launcher/src/launch/outbound/resolve/tree.rs`
**Changes**: An allowlist, not a denylist. Regular files and directories are admitted;
symlinks are admitted only if they resolve inside the root, and Phase 2 §3 decides
whether they are admitted at all. Everything else — hardlinks, FIFOs, devices, sockets,
absolute paths, any component equal to `..` — is rejected, and rejection fails the whole
materialisation rather than skipping the entry.

Each entry is resolved against the **real** root as it is created, not against a
lexical prefix, because a symlink-then-traverse chain defeats a purely lexical check.
Modes are masked to `0755`/`0644` before the seal, so setuid, setgid and sticky bits
cannot survive extraction. The running totals of uncompressed bytes and entry count are
checked against `uncompressed_size` and `entry_count` from the manifest as extraction
proceeds, so a decompression bomb aborts partway rather than after filling the disk.

### Step 1c: `accelerator cache` built-in

#### 1. Command surface

**Files**: `cli/launcher/src/launch/inbound/cli.rs`, `cli/launcher/src/launch/core.rs`,
`cli/launcher/src/main.rs`, `tasks/shared/dispatch_coherence.py`,
`tests/unit/tasks/shared/test_dispatch_coherence.py`
**Changes**:

```
accelerator cache verify [<name>]   walk sealed trees against their file tables
accelerator cache repair [<name>]   re-materialise any tree that fails verify
accelerator cache ensure <name>     materialise a tree if it is not already
accelerator cache prune             reclaim unreferenced generations and orphans
```

`verify` walks each pointed-at generation against its `.files` table using
`symlink_metadata`, and **hashes every regular file**. There is deliberately no
stat-and-escalate shortcut: a substitution that preserves size and mode is exactly the
case the table exists to catch, and an escalation predicate keyed on size or mode never
fires for it — the digests would never be read on the only path that reads them.
ADR-0060 measures a full hash of the whole set at roughly 120ms on the reference host,
which is affordable on a command a user runs deliberately and never runs on the hit
path. The stat pass survives only as a cheap pre-check for missing and unexpected
entries. `verify` reports per-entry discrepancies — missing, extra, size, mode, digest,
link target — rather than a bare pass/fail, so the output diagnoses as well as detects.

`verify` is **offline by construction**: `<name>` is validated against a compiled-in
artifact-name set (the Rust mirror of `TREE_ARTIFACTS`, held to it by a drift test),
not against the manifest. Validating against the manifest would make a diagnostic that
inspects local disk require two HTTPS GETs and a signature verification, so it would be
unavailable exactly when a user reaches for it — offline, air-gapped, or with the
release host down. Default-deny still holds, and no path is ever constructed from an
unrecognised token.

`repair` verifies, then **materialises a new generation** for each failing artifact and
swaps the pointer to it. Because generations are distinct directories, the replacement
is built alongside the tree in use: a live daemon keeps every inode it has already
opened *and* every file it opens later — locale packs, `.pak` resources, `icudtl.dat`,
lazily-`require`d modules — which the single-file `exec` inode argument does not cover.
Nothing is unlinked before a verified replacement exists, so a repair whose refetch
fails leaves the working tree exactly as it was rather than destroying the only copy.
The superseded generation is left for `prune`.

`repair --force` skips verification and re-materialises unconditionally. It is the only
recovery for a tree that is internally consistent but *wrong* — assembled for the wrong
architecture, or missing a component — which `verify` cannot detect by construction,
since such a tree matches its own table perfectly. Without it, a user following the
remediation string in a failure envelope gets a successful no-op and no diagnosis.

`ensure` is the cold-path entry point `accelerator-design` calls when the launcher
exported no path for a tree it needs. It materialises and prints the resolved path, or
fails with a structured cause the caller maps to a downgrade reason. It exists so the
launcher never has to know which design subcommands need a runtime (see Phase 3).

`prune` reclaims every generation no pointer references and no live process holds, plus
orphan temps. It is what bounds growth for anyone who takes the documented
`ACCELERATOR_CACHE_DIR` escape, since that location sits outside the plugin tree and so
outside the only eviction this design otherwise has: content addressing means an
unchanged artifact is reused rather than duplicated, but each pin bump still
materialises a fresh tree and nothing else would ever remove the old one.

`<name>` is validated against the compiled-in artifact set for every verb, and the
canonicalised target must be a direct child of `trees/` before any removal.

**Registration point 10 is a two-sided edit, not a frozenset entry.**
`BUILTIN_SUBCOMMANDS` (`dispatch_coherence.py:39-46`) gains `"cache"`, *and*
`tests/unit/tasks/shared/test_dispatch_coherence.py:606-611` — which asserts the
extracted clap variants are exactly `{"Version", "Config", "External"}` and that
`dispatchable | {"help"} == set(BUILTIN_SUBCOMMANDS)` — must move with it. The
secondary check at `:628-635` (`is_root_help` agreement, `main.rs:104-110`) moves too.
`cache` becomes permanently unavailable as a dispatch token.

### Success Criteria

#### Automated Verification

- [ ] Failing tests first. The signature and end-to-end resolution cases follow
      `cli/launcher/tests/resolution.rs:267-725` with its `MockServer` and real
      keypair — but the extraction, sealing, attestation, pointer and reaper tests
      exercise `resolve/tree.rs` **directly with no signing step**, so they must not
      inherit that file's `skip_if_no_minisign!` guard (`:255-265`), which returns
      `Ok(())` with only an `eprintln!` and would report green on any machine without
      `minisign` on `PATH`
- [ ] A corrupt archive is rejected **before** anything is extracted — the test asserts
      the trees directory is empty after the failure
- [ ] A tarball is rejected for each of: a `../` entry, an escaping symlink, a hardlink
      whose target escapes, an absolute path, a symlink-then-traverse chain, a FIFO or
      device entry, a tree exceeding `uncompressed_size`, and an entry count over
      `entry_count`
- [ ] A setuid archive member is materialised without its setuid bit, and an archive
      member marked executable keeps only its executable bit
- [ ] A streaming fetch whose first attempt fails after N bytes succeeds on retry,
      rather than producing a concatenated archive that can never verify
- [ ] A stalled transfer fails fast rather than waiting out the full deadline three
      times, driven by `Route::Stall` (`tests/common/mod.rs:30-32`) which stops sending
      rather than trickling
- [ ] A second resolution of the same tree issues **zero** HTTP requests, asserted
      against the `MockServer`'s request count
- [ ] A resolution with the release host unreachable still succeeds on a populated
      cache
- [ ] Two concurrent cold resolutions of the same tree issue **exactly one** archive
      fetch, and neither observes a partial tree
- [ ] A winner that fails mid-materialisation releases the lock, and the loser makes
      progress rather than waiting on a pointer that will never appear
- [ ] A crash at each of steps 4 through 10 leaves only reclaimable garbage: no pointer
      is published, `locate` reports a miss, and the reaper removes the residue
- [ ] A pointer naming a directory that does not exist, is not a direct child of
      `trees/`, is not 64-hex, or is not owned by the effective uid is treated as a miss
      rather than exported
- [ ] A sealed tree is removable by `remove_dir_all` without an intervening chmod; an
      archive member marked executable is still executable after sealing; and a
      symlink's target is not re-moded by the seal walk
- [ ] `cache verify` detects each of a deleted file, a truncated file, a **same-size
      same-mode** content substitution, a mode change, a changed symlink target, and an
      unexpected extra entry
- [ ] `cache verify` succeeds with the release host unreachable
- [ ] A truncated tree and a corrupted tree are each returned to a working state by
      `accelerator cache repair`, which materialises a **new generation** and swaps the
      pointer rather than removing the old tree first
- [ ] A repair whose refetch fails leaves the previous tree in place and still
      resolvable
- [ ] A repair run while a process holds files open in the old generation does not
      unlink them, and that process can still open further files from it
- [ ] `repair --force` re-materialises a tree that passes `verify`
- [ ] Every `cache` verb refuses an unrecognised `<name>` without touching the
      filesystem
- [ ] Two release versions naming the same digest share **one** generation directory
      and two pointers, and the second version issues **zero** archive fetches
- [ ] Two platforms sharing one cache root each resolve their own tree
- [ ] A cache root that is group- or world-writable, or not owned by the effective uid,
      is refused rather than trusted
- [ ] The reaper removes a temp archive, a temp tree, and an unreferenced generation;
      spares any generation whose `flock` lease is still held — asserted with the lease
      inherited by a detached child while every ancestor has exited — and spares nothing
      indefinitely once the age backstop passes for a lease-less generation
- [ ] `cache prune` reclaims an unreferenced generation and leaves the pointed-at one
- [ ] 🔒 A dispatch that resolves a tree via `locate` leaves `probe_attempts()`
      unchanged, and a cold `materialise` adds exactly one — asserted with the
      `probes_during` harness (`resolution.rs:199-213`) so work-item:0189's
      once-per-dispatch guarantee is extended rather than broken
- [ ] `manifest.example.json` with an added `artifacts` key still parses, and a manifest
      *without* `artifacts` still resolves single-file binaries
- [ ] `BUILTIN_SUBCOMMANDS` and the clap `Command` enum agree, with
      `test_dispatch_coherence.py:606-611` and `:628-635` updated in the same change
- [ ] `mise run cli:check` exits 0
- [ ] `mise run deny:check` exits 0, and `libz-sys`/`zlib-ng-sys`/`zlib-sys` are absent
      from the launcher feature graph
- [ ] The launcher binary size delta is within a ceiling derived from a **measured**
      per-MB verify slope and a 1ms budget, asserted per target rather than recorded
- [ ] A warm executor invocation shows no regression against a pre-Phase-1 launcher on
      the same host, measured with work-item:0205's settled method — not a bash loop,
      which work-item:0186 records as not method-comparable, and not a bare `≤ 1.1 ×`
      assertion, which is the gate shape 0205 exists because three prose specifications
      of it failed review
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] Inflating the browser archive completes within a stated ceiling on the reference
      host — a threshold, not a recorded observation; if `rust_backend` misses it the
      escalation is a faster **pure-Rust** backend (`zlib-rs`, if it can be shown to
      need no C toolchain), never a `*-sys` crate
- [ ] Files in a materialised tree are not writable by the owning user without an
      explicit chmod, and the tree as a whole is still removable
- [ ] `accelerator cache verify` on a clean cache reports every tree as sealed and
      matching

---

## Phase 2: Release-pipeline assembly

### Overview

Assemble the driver bundle and the browser in CI from verified upstream inputs, and
publish them on the existing manifest and minisign path. Nothing in `tasks/` exists to
reuse for the *inputs*: there is no HTTP client (`pyproject.toml:10-26` declares none,
and every fetch shells out to `gh`), no GPG code, and no npm signature or SLSA
verification. AC13 is three new implementations.

The *output* side reuses the existing path but is not free either. Every list on that
path is derived from `DISPATCHED_SUBBINARIES` — now seven tokens
(`tasks/shared/paths.py:29-37`) — by design rather than from a directory scan, so tree
artifacts are invisible to signing, upload and pre-publish re-verification until each
is given an explicit arm (§5). Getting that wrong publishes a signed manifest promising
assets that do not exist, which is unrecallable.

### Changes Required

#### 1. Pin the vendored version

**File**: `skills/design/inventory-design/scripts/playwright/package.json`
**Changes**: `~1.55.1` becomes the exact version. This makes the fetched package, the
API `lib/*.js` was written against, and the derived Chromium revision one choice rather
than three that can drift. AC10's guard reads this file.

#### 2. Upstream input verification

**Files**: `tasks/vendor/verify.py` (new), `tasks/vendor/pins.py` (new),
`keys/nodejs-release.asc` (new), `keys/npm-registry.pem` (new), `pyproject.toml`,
`mise.toml`, `RELEASING.md`
**Changes**: Three verifications, each failing the release rather than the user's run.
Each needs a trust anchor that does not arrive over the channel it is verifying, and
that is the part ADR-0059 leaves open: it establishes that the sha512 integrity is
fixity rather than provenance "because it comes from registry metadata fetched over
TLS", but never says where the key validating the *signature* comes from. Fetching that
key from the registry too would reproduce the same problem one level up, so both key
sets are committed.

- **`playwright-core`** — fetch from `registry.npmjs.org`, verify the registry
  signature against `keys/npm-registry.pem`, and verify the SLSA provenance
  attestation. That check is only as strong as its predicate: `gh attestation verify`
  without `--owner`/`--repo` accepts an attestation from any builder, so the expected
  source repository, the expected workflow identity, and a subject digest bound to the
  fetched tarball are all asserted explicitly, and any mismatch fails the release. `gh
  attestation verify` appears today only as a manual step in `RELEASING.md:271-281`,
  which also states plainly that "the launcher's runtime trust root is the signed
  manifest, not SLSA provenance"; this makes it a pipeline step without changing that.
- **Node runtime** — fetch `SHASUMS256.txt` and its `.asc` from `nodejs.org/dist`,
  verify the GPG signature, then verify the tarball's digest against the signed
  manifest. The version is not chosen independently: ADR-0059 has it mirror the pairing
  upstream ships, so it is derived from the vendored driver's pairing and guarded like
  the Chromium revision (§4).

  The verification must not trust `gpg`'s exit code, which is **0** for a well-formed
  signature from a key merely present in the keyring and carrying no trust — it prints
  only `WARNING: This key is not certified` to stderr. So: `gpg --no-default-keyring
  --keyring` against the committed key, with `--status-fd` parsed.

  `VALIDSIG` alone is not the predicate, and this is the adjacent trap: GnuPG emits
  `VALIDSIG` for cryptographically valid signatures made by **expired** and **revoked**
  keys too — those cases replace `GOODSIG` with `EXPKEYSIG` or `REVKEYSIG` rather than
  suppressing `VALIDSIG`. A `VALIDSIG`-plus-fingerprint check would therefore accept a
  `SHASUMS256.txt` signed by a Node release key that has since been revoked, which is
  the single case where rotation matters most. So the check requires `GOODSIG` **and**
  explicitly rejects `EXPKEYSIG`, `REVKEYSIG`, `EXPSIG` and `NO_PUBKEY`, and compares
  the allowlist against `VALIDSIG`'s **primary-key** fingerprint field rather than only
  the signing subkey's.

  `gpg` joins the pinned tooling, since its presence and version on the
  `macos-latest` runner are otherwise incidental and its absence must fail the release
  loudly rather than skip the check. The route needs checking rather than assuming:
  `mise.toml`'s `[settings] lockfile = true` (`:46`) hash-pins aqua and ubi artifacts,
  `minisign` is pinned as `ubi:jedisct1/minisign` (`:35`), and GnuPG is not distributed
  that way. If no satisfactory pin exists, pin the *behaviour* instead with a preflight
  that asserts a known-good signature verifies and a known-bad one does not, so a host
  `gpg` that cannot be pinned is at least proven functional before the release depends
  on it.
- **Chromium** — pinned, not verified, per ADR-0059. The revision is read from the
  vendored `playwright-core`'s `browsers.json` and cross-checked against
  `pins.CHROMIUM_REVISION`; the bytes are checked against a committed
  `pins.CHROMIUM_SHA256` per platform. That committed constant is what makes ADR-0059's
  "makes the bytes reviewable" true — a digest derived from whatever the CDN served
  this release attests our own output rather than the input, and is trust-on-first-use
  on every cut. Committing it converts that into one reviewed moment. It bounds blast
  radius; it does not establish provenance, and the module's docstring says so plainly.

**One refresh procedure** covers both key sets and both pins, documented in
`RELEASING.md`, because they fail the same way — stale blocks releases, and carelessly
refreshed is the verification's weakest point, which is ADR-0059's own recorded
consequence. It requires that a new key or hash be obtained from a channel independent
of the one it will verify, landed in the same PR as the `playwright-core` pin bump that
motivated it, and reviewed as a change to a trust anchor rather than a routine version
bump. A Playwright upgrade is therefore one PR touching the pin, four Chromium hashes,
the eight `ASSEMBLED_SHA256` entries §8 introduces (two artifacts × four platforms),
and any key that rotated with it.

The procedure is documentation, so it is backed by two mechanical guards, because a
committed anchor is only as strong as the review that gates it and this repository has
no CODEOWNERS file — a change to `keys/**` or `tasks/vendor/pins.py` is reviewed
exactly like a version bump today. First, a build-system test asserts the keys present
in `keys/nodejs-release.asc` are exactly the fingerprints in the committed allowlist
and that each is unexpired. Second, a CODEOWNERS entry (or an equivalent CI guard)
covers `keys/**` and `tasks/vendor/pins.py`.

The assembled digests are the one anchor whose value cannot be obtained independently —
they are computed from our own deterministic assembly of inputs the other anchors have
already verified, so they attest reproducibility rather than provenance. The procedure
records that distinction, and requires them to be regenerated by a clean assembly on a
machine that fetched the upstream inputs fresh, never copied from a reuse path.

`requests` is added to `pyproject.toml`'s `build` dependency group — a genuinely new
dependency, since `tasks/` has no HTTP client and `[settings] lockfile` does not cover
Python packages. It is pinned exactly, matching the group's existing discipline for
`pyyaml==6.0.3` and `ruff==0.15.16`.

#### 3. Assembly

**Files**: `tasks/vendor/assemble.py` (new), `tasks/build.py`,
`.github/workflows/main.yml`
**Changes**: Two tasks, not one, and **two workflow steps**, not one:

- `vendor.verify_upstream_inputs` downloads and verifies, and **never extracts**. It
  needs `GH_TOKEN` for `gh attestation verify`.
- `build.assemble_tree_artifacts` extracts and assembles, and runs with **no**
  `GH_TOKEN`.

Wiring both into `release_prepare` would make the split imaginary: `Prepare stable
release` (`main.yml:613-616`) is a single step running `mise run release:prepare` with
`GH_TOKEN` in its `env` (`:615`), so two invoke tasks inside it share one environment.
Assembly therefore gets its own mise task and its own workflow step, invoked outside
`release:prepare` — which is also what makes the scoping assertable, since the existing
attest-block tests inspect workflow shape and cannot see inside an invoke call graph.

The split matters because assembly extracts an npm tarball and the Chromium zip, and
ADR-0059 records Chromium's custody as TLS-only with no signature. Today the `Prepare`
steps carry `GH_TOKEN` in a job holding `contents: write` and `attestations: write`
(`main.yml:572-575`), upstream of the step holding `ACCELERATOR_RELEASE_SECRET_KEY`
(`:618-621`) — so a path-traversal entry could overwrite a `tasks/*.py` module that the
later Sign step imports. Extraction therefore lands in a staging directory **outside
the checkout**, only the finished archives are copied into `dist/release/`, and the
same entry rules the launcher applies (Step 1b §4) apply CI-side too. This extends the
rule the plan already follows for the signing secret: the step that handles untrusted
input holds no credential.

A second reason to stage outside the checkout: `_publish` commits with `git add .`
(`tasks/git.py:73`), and the only backstop is the marker list at `tasks/release.py:22`
checked against `git status --porcelain -uall`. `/dist/` is gitignored and root-anchored
(`.gitignore:23`), so archives there are invisible to both; a staging tree anywhere else
inside the checkout would be swept into the version-bump commit.

**What a step boundary does and does not buy**, stated plainly rather than overclaimed.
It removes `GH_TOKEN` from the extracting step.

It does **not** remove the GitHub App token `actions/checkout` writes into
`.git/config`, and `persist-credentials: false` cannot simply be added: neither release
checkout sets it (`main.yml:475-478`, `:585-588`), so it defaults to `true`, and
`tasks/git.py:35-52` runs a bare `git push --atomic` with no credential helper, no
authenticated remote URL and no `gh auth setup-git`. That persisted app token is the
only credential the release push has — `GH_TOKEN` is `secrets.GITHUB_TOKEN`, set for
the `gh` CLI and not what authenticates the push. Adding the flag without a replacement
wedges every cut after the version bump has been pushed. If the hardening is wanted it
must land together with an explicit credential scoped to the finalise step, and the
test must assert both.

It does **not** remove the job-wide values: `id-token: write` and `attestations: write`
mean `ACTIONS_ID_TOKEN_REQUEST_URL`/`_TOKEN` and `ACTIONS_RUNTIME_TOKEN` are present in
every step of the job regardless of its `env`. So an extraction escape still reaches
enough to mint an OIDC token for a fraudulent attestation.

Two things bound that residue. §8's committed `ASSEMBLED_SHA256` means tampered bytes
cannot reach the signing step at all — the attacker's path to a *signed* artifact is
closed independently of the token question — so what remains is token theft rather than
artifact substitution. And the extraction rules above are what stop the escape
happening in the first place.

Full isolation would mean a separate job with `permissions: {}`, passing ~1.2GB of
archives between jobs as workflow artifacts. That is deliberately not taken here: the
release job's own comment requires the prepare/sign/finalise sequence to stay in one
job for version monotonicity, the transfer cost is substantial, and the digest pin
already closes the outcome that matters. It is recorded as the escalation if the
residual is later judged unacceptable.

`build.assemble_tree_artifacts` produces, per platform:

```
dist/release/accelerator-driver-<platform>.tar.gz
dist/release/accelerator-browser-<platform>.tar.gz
```

Flat in `dist/release/` — `tests/unit/tasks/test_workflows.py:161-168` expands
`@actions/glob`'s `*` to `[^/]*` explicitly, so a nested staging tree would silently
miss `dist/release/accelerator-*` and fail
`test_attest_globs_cover_every_published_asset` (`:207-221`).

The driver tree contains the Node binary and `playwright-core`. The browser tree
contains `chromium-headless-shell` only; `ffmpeg` is excluded.

Assembly also **decides whether the trees contain symlinks at all**, and records the
answer, because the launcher's extraction allowlist admits in-root symlinks — the
trickiest branch in the extractor, since defeating a symlink-then-traverse chain needs
each entry resolved against the real root as it is created. Since we produce the
archives, that branch may be unnecessary: if assembly emits no symlink, a CI-side
assertion pins that and Step 1b §4 narrows its allowlist to regular files and
directories only, retiring the hardest-to-review code in the extractor rather than
maintaining it for a capability nothing exercises.

Both tasks are wired into `prerelease_prepare` (`tasks/release.py:117-129`) and
`release_prepare` (`:144-160`), verification then assembly, **after**
`build.cli_cross_compile` and **before** `build.create_debug_archives`. They go in
`prepare`, never `sign` — `_sign` (`:86-100`) is the only function holding the secret,
and `main.yml` scopes `ACCELERATOR_RELEASE_SECRET_KEY` to Sign steps deliberately
(`:505-508`, `:618-621`, `:642-645`). No npm, nodejs.org or CDN fetch ever happens
inside `_sign`.

#### 4. Version guards

**File**: `tasks/vendor/assemble.py`
**Changes**: The assembly fails the release if the fetched `playwright-core` is not the
exact version `package.json` declares, or if the fetched Chromium revision is not the
one that package's `browsers.json` names. Per ADR-0059 the pairing is structural, so
this guards the construction rather than testing compatibility after the fact.

#### 5. The publish path: registry, signing, manifest, upload, re-verify

**Files**: `tasks/shared/paths.py`, `tasks/signing.py`, `tasks/manifest.py`,
`tasks/github.py`
**Changes**: Every list on the publish path is derived from an explicit registry rather
than from a directory scan, and each is derived from the *same* one —
`upload_and_verify_release` records why: the "every asset uploaded" and "every asset
re-verified before `--draft=false`" lists cannot derive from two values. Tree artifacts
need the same treatment, so they start with a registry.

`tasks/shared/paths.py` gains `TREE_ARTIFACTS: tuple[str, ...] = ("driver", "browser")`
beside `DISPATCHED_SUBBINARIES` (`:29-37`), plus a
`tree_artifact_asset_path(name, platform)` mirroring `subbinary_asset_path` (`:80-90`).
Assembly, signing, manifest emission, upload and re-verification all derive from it, so
adding or retiring an artifact is one edit rather than a hunt across five files.

The single source has to cross the language boundary, or it stops at `tasks/`. The Rust
side encodes the same names in two places — the launcher's compiled-in set that
validates `cache` verbs offline, and `accelerator-design`'s `ensure` call sites — so a
**drift test** pins the Rust set against `TREE_ARTIFACTS` and both against the
`artifacts` keys in `manifest.example.json`, in the same shape as the
`BUILTIN_SUBCOMMANDS` ↔ clap pin. Without it, retiring an artifact yields a launcher
exporting a variable nothing publishes, or a design binary requesting a name the
manifest no longer carries — failures that surface at runtime on a user's machine,
since trees are exempt from the per-exec re-verification that would otherwise catch a
mismatch.

Four arms follow, and none is optional:

1. **Signing** — `sign_staged_binaries` (`tasks/signing.py:60-79`) builds an explicit
   expected list from the launcher plus `_subbinary_signing_targets()` and raises on
   any missing member, deliberately never scanning a directory. A
   `_tree_artifact_signing_targets()` arm joins it, so a partial assembly fails closed
   exactly as a partial cross-compile does. ⏱️ `sign_file` (`:24-43`) runs one
   `minisign -S` per file under a **120-second timeout sized for an 8MB binary**;
   eight ~120MB archives join the existing 32 invocations, so the timeout is re-derived
   and stated rather than inherited.
2. **Manifest** — `collect_artifact_entries()` mirrors `collect_entries`
   (`tasks/manifest.py:81-108`) and a second key joins `build_manifest` (`:111-130`). It
   emits more than `collect_entries` does: alongside `sha256` and the inline signature
   it records `archive_size`, `uncompressed_size` and `entry_count`, all three measured
   during assembly rather than restated, so producer and consumer cannot disagree about
   the bounds the launcher enforces. **Do not bump `SCHEMA_VERSION`** (`:23`). Ordering
   is not free here: `collect_entries` slurps the pre-produced `.minisig` contents as
   the inline signature, so collection must follow signing — which `_sign`
   (`tasks/release.py:86-100`) already sequences correctly inside one
   `resolve_secret_key` block, and the artifact arms slot into those same two calls.
3. **Upload** — `_release_uploads` (`tasks/github.py:231-248`) assembles launcher,
   manifest, debug archives and `_subbinary_uploads`; a `_tree_artifact_uploads` arm
   joins it, each archive with its `.minisig` sidecar. Today's 70 uploads become 86. The
   existing `missing` check (`:339-343`) then fails loudly on an unassembled artifact
   before a single upload starts.
4. **Re-verification** — `_subbinary_reverifies` (`:287-315`) reads
   `manifest["binaries"][name]` and re-downloads each asset to check its sha256 and
   inline signature. A `_tree_artifact_reverifies` arm reads `manifest["artifacts"][name]`
   and does the same, so the `--draft=false` transition (`:356`) waits on the tree
   archives too.

Without all four, the release publishes a *signed* manifest naming artifacts that were
never signed, never uploaded and never re-verified —
`_assert_staged_manifest_is_current`'s own docstring names that outcome as one that
cannot be recalled. Every user on that version would 404 on their first design run, and
the fix would be a whole new release.

#### 6. The guards that will trip

**Files**: `tasks/release.py`, `tests/unit/tasks/test_manifest_contract.py`,
`tests/integration/tasks/test_github.py`,
`cli/launcher/tests/fixtures/manifest.schema.json`, `.github/workflows/main.yml`
**Changes**:

1. `_assert_staged_manifest_is_current` (`tasks/release.py:57-83`) compares only
   `set(staged["binaries"])` against `DISPATCHED_SUBBINARIES`. Without an artifact
   equivalent a stale artifact manifest passes silently — add the parallel arm against
   `TREE_ARTIFACTS`.
2. `test_attest_globs_cover_every_published_asset` (`test_workflows.py:207-221`) —
   satisfied by the flat naming above, but assert it rather than assume it.
3. `test_every_attest_block_declares_the_same_subjects` (`:198-204`) — all three blocks
   (`main.yml:513-517`, `:626-630`, `:650-654`) must stay byte-identical.
4. `tests/unit/tasks/test_manifest_contract.py:30-48` iterates `binaries` only; add a
   parallel arm for `artifacts`, asserting the same
   `accelerator-{key}-{platform}.tar.gz` convention Phase 1 pinned in
   `manifest.example.json`, so producer and consumer are held to one fixture.
5. `cli/launcher/tests/fixtures/manifest.schema.json` describes itself as the signed
   distribution contract between the release signer and the launcher/bootstrap readers,
   and its top-level `required` is `["schema_version", "version", "binaries"]` (`:7`)
   with no `artifacts` property and no artifact `$defs` (`:24-58`). It gains both — an
   `artifactEntry` and an `artifactPlatformEntry` carrying the three required sizes — or
   the one document a third party would read to understand the wire format describes a
   shape the producer no longer emits. `test_schema_platform_enum_matches_the_alias_set`
   (`test_manifest_contract.py:43-48`) reads only `$defs.binaryEntry` today, so it is
   extended to assert the artifact side's platform-alias enum equals `ALIASES` too.
6. `tests/integration/tasks/test_github.py:356-373` asserts an exact upload count from
   three derived terms; a fourth term for tree artifacts joins it, and `_setup_release`
   (`:275-347`) stages the archives and their sidecars so the count is reachable.
   `_SUBBINARY_DESCRIPTIONS` (`:35-50`) is keyed by dispatched token and does **not**
   need an artifact entry — tree descriptions come from the assembly, not from a
   `Cargo.toml` — but the fixture's manifest writer does.

A guard that turns out **not** to trip: `_assert_no_leaked_artifacts`
(`tasks/release.py:40-54`) matches its markers against `git status --porcelain -uall`,
and `/dist/` is gitignored, so the archives are invisible to it. Worth recording so
nobody spends time on it.

#### 7. Release-job capacity and the failure envelope

**Files**: `.github/workflows/main.yml`, `tasks/github.py`
**Changes**: The `release` job (`main.yml:554-659`) runs the whole pipeline **twice** —
stable, then the post-stable pre.0 cut — so roughly 2.4GB of assembly and upload per
stable release, on a `macos-latest` runner with **no `timeout-minutes`** (the only ones
in the file are at `:125`, `:335` and a step-level `:372`) and no disk guard.
`dist/release/` is never cleaned between the two passes, and `--clobber` on retry
(`:318-319`) re-uploads the lot.

Add a `timeout-minutes` to both publishing jobs and a disk-space assertion before
assembly. Hosting capacity itself is confirmed and assumed.

⚠️ **One failure path changes character at this payload size, and it burns a version
number.** `download_and_verify` (`tasks/github.py:132-148`) converts a
`subprocess.TimeoutExpired` into an `AssetVerificationError` — but it is unused in the
production flow. The two re-verify helpers tree artifacts actually reach do not:
`_reverify_via_shim` (`:186-197`) and `_reverify_subbinary` (`:200-216`) call
`download_release_asset` bare, and its `timeout=120` (`:111`) is sized for a 7.6MB
launcher. A 177MB archive plausibly exceeds it, raising `TimeoutExpired`, which is not
an `AssetVerificationError` and so lands in `upload_and_verify_release`'s
`except Exception` arm — running `gh release delete <tag> --cleanup-tag --yes`
(`:359-364`) *after* `_publish` (`tasks/release.py:103-111`) has already committed,
tagged and pushed the version bump. A transient download hiccup would burn a version
number and leave the repository and the release host inconsistent, under the
`accelerator-release` concurrency lock.

So: size `download_release_asset`'s timeout to the expected asset rather than a flat
120s, and wrap both re-verify helpers so a transport failure becomes an
`AssetVerificationError`. That routes it to the draft-preserving arm with the forensic
alert that already exists (`:37-40`, `:153`), and `--clobber` means a preserved draft
can be re-driven to green.

`TimeoutExpired` is not the only newly-reachable path, so the narrowing is by
**default** rather than by enumeration: every failure inside the upload/re-verify
envelope preserves the draft, and the delete arm is reserved for an explicit,
enumerated set of pre-upload failures. At ~2.4GB per stable cut, `OSError: No space
left on device` from a re-verify download or from `compute_sha256`, a hung
`gh release upload` (`_upload_clobber` has neither timeout nor retry, `:318-319`), a
hung shim verification (`_run_shim` likewise, `:170-183`), and a `CalledProcessError`
from a transport blip are all now plausible. Bounded retry with backoff wraps
`_upload_clobber`, the disk assertion covers the whole job rather than only
pre-assembly, and the newly-added `timeout-minutes` is itself recorded as an abort
cause that runs no cleanup arm, so it is sized with headroom and `--clobber` is
documented as its recovery.

Worth recording as already-safe: `_release_reverifies` is built at `:344`, *before* the
`try`, so a manifest `KeyError` — a token in the registry but absent from the staged
manifest — raises outside the delete envelope entirely.

#### 8. Reuse across cuts, and a functional gate

**Files**: `tasks/vendor/assemble.py`, `.github/workflows/main.yml`
**Changes**: Two problems that only appear once assembly is in the pipeline.

**Every release becomes dependent on three third-party hosts.** Assembly is wired into
both `prerelease_prepare` and `release_prepare`, so every cut fetches from
`registry.npmjs.org`, `nodejs.org/dist` and `cdn.playwright.dev` — yet all three inputs
are pinned by exact version and hash, so the produced bytes are identical release after
release. As written, an npm outage, a key rotation or a yanked version makes the
pipeline unreleasable, including for an urgent fix to something entirely unrelated: a
large new single point of failure in front of the one mechanism that ships fixes to
users.

So assembly becomes **deterministic and digest-pinned**, and reuse is authenticated
rather than merely cached.

**Deterministic assembly.** `assemble_tree_artifacts` normalises everything that would
otherwise vary between runs: entries emitted in sorted order, mtimes, uid, gid and owner
names fixed to constants, modes masked to the same `0755`/`0644` the launcher enforces,
and gzip written without an embedded timestamp. Assembling the same pin triple twice
must produce byte-identical archives, asserted by a test that assembles twice and
compares digests. This is worth doing on its own merits — it makes a release auditable
by anyone who can run the same pins — but it is also the precondition for everything
below, because an unreproducible archive cannot be pinned.

**A committed expected digest.** `pins.py` gains `ASSEMBLED_SHA256`, one digest per
artifact per platform, committed and reviewed under the same trust-anchor refresh
procedure as the keys and the upstream pins (§2). Every archive that reaches the signing
step — freshly assembled or reused — is checked against it, and a mismatch fails the
release. Without this the digest check is self-referential: a matching digest computed
from whatever is on disk proves only that the bytes are the bytes.

**Reuse is our own signed asset, not a cache blob.** When the pin triple is unchanged,
the reuse source is the **previous release's published artifact**, re-downloaded and
verified with sha256 plus minisign against the embedded public key — the identical check
the launcher performs on a user's machine — and then against `ASSEMBLED_SHA256`. That
keeps the chain of custody inside our own signature rather than extending trust to a
mutable store: a CI cache is writable from other workflows on the default branch, is
evictable after a quiet week, and shares a per-repository budget with the toolchain
caches this repo already depends on, so a poisoned or partially-restored entry would be
signed with `ACCELERATOR_RELEASE_SECRET_KEY` and published with none of §2's npm, SLSA,
GPG or Chromium-hash gates re-running — the plan's own unrecallable outcome, reached by
accident rather than by attack. Any mismatch, any absent asset, and any pin movement
falls back to a full cold assembly, so the reuse path can only ever be an optimisation.

The same mechanism removes the duplicated work in the release job's double pass (§7):
the post-stable pre.0 cut reuses the stable pass's archives by digest instead of
re-assembling identical bytes.

**Nothing ever executes what was built.** Every other gate in this phase is about
provenance and shape: upstream signatures, version and hash guards, glob coverage,
manifest arms, and a `.minisig` the CLI-side verifier accepts. A brand-new step
composing four platforms from three upstreams can produce a correctly-signed,
correctly-hashed, structurally-wrong tree — wrong architecture, missing `NOTICES/`, a
layout `playwright-core` cannot resolve — and it would pass everything, reach every user
of that release, never self-heal (trees are exempt from per-exec re-verification), and
be faithfully re-fetched by `cache repair`, which trusts the same manifest. Recovery
would be a new release for every affected user.

So assembly ends with a functional gate — but **not inside the release job**. Executing
the vendored Node binary and `chromium-headless-shell` is a stronger form of handling
untrusted input than extracting them, and Chromium is the one input ADR-0059 records as
accepted on TLS trust alone. The `release` and `prerelease` jobs carry
`ACCELERATOR_RELEASE_SECRET_KEY` in a later step, plus job-wide `id-token: write` and
`ACTIONS_RUNTIME_TOKEN` that no step-level `env` can scope away — so a compromised CDN
build would gain code execution one step before the signing key enters the environment.
§3's own rule was written for extraction and applies here with more force.

The smoke check therefore runs in a **separate job with `permissions: {}`**, consuming
the archives as workflow artifacts: unpack the driver and browser for that runner's
platform, execute the Node binary and the headless shell with `--version`, and assert
`NOTICES/` is populated. It runs on **reused** archives as well as freshly assembled
ones — a reuse path that skipped it would be the one route by which an unexecuted
artifact reaches a release. The publish step gates on that job.

If the artifact transfer between jobs proves disproportionate, the fallback is to keep
only the structural check in-job and accept the reduced assurance explicitly — never to
execute upstream binaries beside the signing key.

The smoke check cannot cover the cross-compiled platforms, so the three non-host
artifacts get a structural check instead: the expected file set, plus the ELF/Mach-O
header and architecture of the Node binary and the headless shell for the target they
claim to be. That catches a wrong-architecture or truncated assembly without executing
it. Between them the two checks are the only gates distinguishing "signed" from "works",
and both are nearly free on the macOS runner.

#### 9. Redistribution notices

**File**: `tasks/vendor/assemble.py`
**Changes**: Each artifact carries the notices for what it contains — Node and its
bundled dependencies, `playwright-core`, and Chromium's credits — assembled into a
`NOTICES/` directory at the tree root. Phase 3 adds the subcommand that surfaces them,
so a user reaches them without unpacking the artifact by hand.

An automated assertion covers it here rather than only the manual check: the produced
tree contains `NOTICES/` with an entry per expected component, driven from the same
component list the assembly uses. AC16's notices are the plan's stated substitute for a
legal review gate, so an assembly refactor that silently drops a component must fail
rather than ship.

### Success Criteria

#### Automated Verification

- [ ] Failing tests first for each verification, using recorded upstream fixtures rather
      than live network calls. Committing the keys makes Node/GPG fully
      offline-verifiable, so it is tested for real rather than mocked; the SLSA check
      contacts a transparency log, so its runner is injected and both branches asserted
      — and the plan records that the attestation's *content* is not verified in tests
- [ ] A tampered `SHASUMS256.txt` signature fails the release
- [ ] A `SHASUMS256.txt` signed by a well-formed key absent from the committed
      fingerprint allowlist fails the release, **even though `gpg` exits 0**
- [ ] A `SHASUMS256.txt` signed by a revoked key fails the release, and one signed by an
      expired key fails the release, even though both yield `VALIDSIG`
- [ ] An absent `gpg` fails the release rather than silently skipping the check
- [ ] The npm/SLSA path fails closed in each degraded mode — attestation bundle absent,
      transparency log unreachable, `gh attestation verify` unavailable
- [ ] The committed Node keyring and the committed fingerprint allowlist describe the
      same key set, each unexpired
- [ ] A `playwright-core` tarball failing its registry signature fails the release
- [ ] An attestation whose source repository or workflow identity differs from the
      pinned predicate fails the release
- [ ] An attestation whose subject digest does not match the fetched tarball fails the
      release
- [ ] A `playwright-core` version other than `package.json`'s pin fails the release
- [ ] A Chromium revision other than `browsers.json`'s fails the release
- [ ] Chromium bytes whose sha256 differs from `pins.CHROMIUM_SHA256` fail the release
- [ ] A Node version other than the vendored driver's pairing fails the release
- [ ] Assembly is its own workflow step, and that step's `env` contains no `GH_TOKEN` —
      asserted by a workflow test alongside the existing attest-block assertions, which
      is only possible because it is a step rather than a task nested inside
      `release:prepare`
- [ ] If `persist-credentials: false` is adopted, the finalise step has an explicit
      credential and the push still authenticates — asserted together, since the flag
      alone breaks every release
- [ ] Extraction happens outside the checkout, and a tarball with a `../` entry, an
      escaping symlink, a hardlink, an absolute path or a setuid bit is rejected CI-side
      by the same rules the launcher applies
- [ ] The assembled, signed, manifest-listed, uploaded and re-verified sets are pinned
      against each other by one test, so an artifact cannot appear in some and not others
- [ ] An unassembled artifact fails the **signing** step, not the upload step
- [ ] A tree archive with no `.minisig` fails `collect_artifact_entries`
- [ ] `_assert_staged_manifest_is_current` rejects a manifest whose `artifacts` keys
      differ from `TREE_ARTIFACTS`
- [ ] An artifact platform entry missing any of `archive_size`, `uncompressed_size` or
      `entry_count` fails to parse, rather than defaulting to 0 and disabling the cap it
      feeds
- [ ] The emitted sizes match the assembled archive and its extracted tree
- [ ] `manifest.schema.json` validates a manifest carrying `artifacts`, and its artifact
      platform-alias enum equals `ALIASES`
- [ ] `test_github.py`'s upload-count assertion covers the tree archives and their
      sidecars, derived rather than hard-coded
- [ ] A simulated download timeout during tree re-verification preserves the draft and
      emits the forensic alert, rather than deleting the release and its tag
- [ ] Every produced archive matches `dist/release/accelerator-*`
- [ ] Assembling the same pin triple twice produces **byte-identical** archives
- [ ] Every archive reaching the signing step matches `pins.ASSEMBLED_SHA256`, whether
      freshly assembled or reused; a mismatch fails the release
- [ ] An unchanged pin triple reuses the previous release's published artifact and
      performs **no** upstream fetch; moving any one pin re-runs the fetch-and-verify
      path
- [ ] A reused artifact failing its minisign check, its sha256, or the committed digest
      falls back to a full cold assembly rather than being signed
- [ ] The second (pre.0) pass reuses the stable pass's archives rather than re-assembling
      them
- [ ] The smoke check runs in a job with `permissions: {}` and no access to the signing
      secret, asserted by a workflow test
- [ ] The smoke check runs on reused archives as well as freshly assembled ones, and
      fails the release on a tree whose Node binary or headless shell will not execute,
      or whose `NOTICES/` is empty
- [ ] The structural check fails a cross-compiled artifact whose Node binary or headless
      shell has the wrong architecture or object format for its target
- [ ] The produced tree contains a `NOTICES/` entry per expected component, driven from
      the assembly's own component list
- [ ] An end-to-end round trip: a synthetic tree assembled through the real assembly
      path, a manifest emitted through the real `build_manifest`, signed with a test key,
      resolved by the launcher's tree resolver
- [ ] `mise run test:unit:build-system` and `mise run build-system:check` exit 0
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] A full local dry-run assembly produces both artifacts for one platform, and their
      measured sizes are recorded and fed back into Step 1a's fetch deadline
- [ ] Each produced `.tar.gz` has a `.minisig` that the CLI-side verifier accepts, with
      the signing form (`-S` versus `-S -H`) recorded
- [ ] The upload list and the re-verify list, printed for one platform, each contain both
      tree archives and their sidecars
- [ ] `manifest.json` renders `artifacts` beside `binaries` with a launcher built before
      this phase still resolving single-file binaries from it
- [ ] The `NOTICES/` directory in each artifact contains all three licence sets

---

## Phase 3: Swap onto the bundled driver and browser

### Overview

Point the executor at launcher-resolved tree artifacts, retarget the automation at
`playwright-core`, and delete the on-machine install. Depends on Phases 1 and 2.

### Changes Required

#### 1. Retarget the automation

**Files**: `lib/daemon.js`, `lib/playwright-loader.js` and its three
`fake-playwright*` fixture trees, `lib/playwright-loader.test.js`, `tasks/test/unit.py`
**Changes**: The assembled bundle ships `playwright-core`, not `playwright`.
`playwright-loader.js:23-25` resolves `<nsRoot>/node_modules/playwright/package.json`
and `:53-56` deliberately throws when `exports['.']` is an object whose `.import` is not
a string — the fix for the 0072 CJS-shim bug.

`daemon.js` uses only `chromium.launch({ headless: true })` (`:137`) and
`chromium.executablePath()` (`:152`), both present in `playwright-core`. So `daemon.js`
imports `playwright-core` directly, matching what Microsoft's own bindings do, and
`playwright-loader.js`, its test and its three fixture trees are deleted.

The 0072 regression it guarded does not recur: the bug was the loader selecting a CJS
shim entry from a `playwright` package whose `exports` map it misinterpreted, and there
is no longer a loader making that selection. A test asserts `chromium` is a defined
export of the resolved module, which is the property 0072 actually cared about.

**Both unit-lane floors move in this change.** `_EXPECTED_DESIGN_AUTOMATION_SUITES` is
**9** (`tasks/test/unit.py:62`) against exactly nine `lib/*.test.js` files, and
`_EXPECTED_DESIGN_AUTOMATION_CASES` is **76** (`:66`), an at-least floor over the
runner's own TAP summary. Deleting `playwright-loader.test.js` takes the suite floor to
8 and the case floor down by that file's executed count, read off the TAP summary rather
than guessed. Leaving either behind fails `test:unit:design-automation`, which is in the
default roll-up (`mise.toml:266-268`) and therefore in CI.

#### 2. Passing the browser path, and the `chromium-not-found` diagnostic

**File**: `lib/daemon.js`
**Changes**: `daemon.js:137` calls `chromium.launch({ headless: true })` with **no
`executablePath`**, so `playwright-core` resolves from its own browser registry —
exactly the mechanism both the bundled tree and the `design.browser_path` hatch must
override. Without an explicit argument the path would be resolved in Rust and then
ignored in JS, and **AC12 could not pass**. So `daemon.js` reads the resolved path from
`ACCELERATOR_DESIGN_BROWSER_EXECUTABLE` and passes it:
`chromium.launch({ headless: true, executablePath })`.

**The `ping` handler must read the same variable, not `executablePath()`.**
`daemon.js:149-157` currently calls `cr.executablePath()` and `promises.access(execPath)`
on the result. `BrowserType.executablePath()` is computed from `playwright-core`'s
**browser registry** — the `PLAYWRIGHT_BROWSERS_PATH` layout or its default — and
neither takes nor reflects a per-launch `executablePath` option. With the bundled sealed
tree, and a fortiori under the hatch pointing at a distro Chromium, that registry path
does not exist, so the `access` throws and `ping` returns `chromium-not-found`.

That is not cosmetic. `ping` is the readiness probe `SKILL.md:145-156` runs, and its
failure is the `executor-ping-failed` downgrade — so **every crawl would degrade to the
code-only crawler on exactly the machines the bundled artifacts exist to serve**, and
AC6 and AC12 would both fail, after Phases 1 and 2 have shipped ~1.2GB per release to
support them. The handler therefore `access`es and reports the launch path. If the
registry path is wanted as a secondary diagnostic it may be reported alongside, but it
never decides the outcome.

The diagnostic's own text changes too: `:154` reports against the **full Chromium** path
while this ships `chromium-headless-shell`, and its message says "Run
ensure-playwright.sh to reinstall", naming a script this phase deletes. It is rewritten
to name `accelerator cache repair`.

Passing `executablePath` explicitly also resolves the sealed-tree layout risk rather
than merely mitigating it: supplying the path is what makes `playwright-core` skip
registry resolution and its validation entirely, so the browsers root of a `0444`/`0555`
tree is never consulted or written. Confirm that empirically against the pinned
`playwright-core` version — a test asserting a launch succeeds against a read-only
browsers root is the cheapest form — and if any path still writes there, place the
marker outside the tree rather than unsealing it.

#### 3. Tree resolution

**Files**: `cli/design-cli/src/executor.rs`, `cli/launcher/src/main.rs`,
`cli/launcher/src/launch/core.rs`
**Changes**: The embedded signing key keeps exactly one holder (ADR-0061), so the
launcher owns materialisation — but it must not own the *decision*, because ADR-0062
puts the ordering and the downgrade vocabulary in the design binary. The split is by
cost:

- **Warm, on every dispatch**: for each tree `locate` resolves, the launcher exports
  `ACCELERATOR_TREE_<NAME>` — a generic name derived from the pointer files present on
  disk, not a `DESIGN`-prefixed one, so the launcher enumerates rather than knows, and a
  second tree consumer inherits the convention rather than a design-shaped variable.
  That is `locate`'s two small reads and two stats per tree (Step 1b §2), issues no
  network request, probes no cache root, and has no failure mode: a tree that is absent,
  unpointed, unparseable, or failing its ownership check simply yields no variable.

  The variables are **always set or explicitly cleared**, never merely left alone, so an
  inherited or injected value from the surrounding environment can never be mistaken for
  one the launcher resolved.
- **Cold, only when needed**: `accelerator-design` calls `accelerator cache ensure
  <name>` at the point in its own ordering where it has established that it needs the
  runtime. That is the only place a ~294MB fetch can be triggered, so `validate-source`,
  `resolve-auth`, `scrub-secrets`, `notify-downgrade` and `audit-cue-phrases` never
  touch the network, and `notices` reads whatever is already materialised.

An absent variable is therefore the normal state rather than an error: it means "not
materialised yet", and the executor decides whether to `ensure`, downgrade, or proceed.
That is also what makes the `ACCELERATOR_DESIGN_BIN` dev override work — it is read at
`cli/launcher/src/launch/outbound/mod.rs:21-47` and bypasses the resolve path entirely,
so the variables are never set and the executor reaches `ensure` exactly as it would on
a cold cache.

**The `ensure` contract**, since this is a machine-consumed interface between two
separately-built executables:

- **Discovery.** `accelerator-design` must locate the launcher to invoke it, and
  `argv[0]` is its own content-addressed cache path. The launcher exports
  `ACCELERATOR_LAUNCHER_BIN` (its own resolved shim path) alongside the tree variables;
  its absence is itself a diagnosable cause, not a panic. This closes the dev-override
  case too: `ACCELERATOR_DESIGN_BIN` bypasses the resolve path, so the variable is unset
  and the executor reports `artifact-unavailable` with a cause naming why.
- **Envelope.** `ensure` emits a golden-pinned structured envelope with an enumerated
  cause set mapped 1:1 onto downgrade reasons — unreachable host, signature mismatch,
  digest mismatch, disk shortfall, unwritable cache root, platform unsupported, artifact
  absent from the manifest. The executor maps causes, never parses prose.
- **Version skew.** Against a launcher predating Phase 1, `cache` is not a built-in and
  is treated as a dispatch token, producing an `AssetNotFound` for
  `accelerator-cache-<platform>` — a distribution error that would surface instead of a
  downgrade. So an unrecognised cause, a non-zero exit with no parseable envelope, and a
  resolution error all map to `artifact-unavailable`.

Collapsing every cause into `artifact-unavailable` unconditionally would leave a 3am
failure with no diagnosis, which is why the cause set exists; mapping *unknown* causes
there is the fallback, not the default.

**A failed materialisation is sticky for the session.** A crawl makes 100–200 executor
invocations, and with no negative caching a persistent failure — a full disk, a
read-only plugin root, a flapping link, a 404 for one platform — would produce a fresh
full-size attempt, times three fetch retries, on *every one* of them. A single crawl on
a failing machine could attempt tens of gigabytes and repeatedly fill the user's disk
with partial archives. This risk did not exist for megabyte-scale single-file
sub-binaries. So the first `artifact-unavailable` downgrade suppresses re-attempts and
the remaining invocations take the code-only path immediately.

The marker lives in the executor's own state directory — the `0700` directory the
sibling established at `<repo>/<paths.tmp>/inventory-design-playwright`
(`design-adapters/src/paths.rs:51-56`, created at `design-cli/src/executor.rs:103-116`)
— **not** beside `trees/`. Two of the failure causes it exists to damp are a full disk
and an unwritable cache root, so a marker written into the cache root could not be
created in exactly the cases that recur. It records the artifact name, the cause and a
timestamp, and it is cleared by any successful `ensure` and by `cache repair`, so the
documented remediation is also the reset. Its TTL is derived from the crawl bound: a
crawl is bounded at five minutes, so a TTL of that order suppresses within-crawl retries
without stranding the next crawl after a user frees disk space or reconnects.

Tree-related failure envelopes also carry a remediation string naming `accelerator cache
repair <name>`. ADR-0060 accepts as a known negative that a truncated tree "surfaces as
a confusing runtime failure until the repair path is run" — but self-healing needed no
discovery, whereas this needs the user to already know a command exists that the failure
never mentions. Naming it in the failure is what makes AC14's recovery reachable in
practice rather than only documented.

The executor sets `NODE_PATH` and the browser executable path from the resolved trees,
replacing the lockhash namespace. Concretely, the one environment vector at
`design-cli/src/executor.rs:139-156` — shared by `DaemonSpawner` and `ExecClient` — stops
deriving `NODE_PATH` and `ACCELERATOR_PLAYWRIGHT_NS_ROOT` from
`namespace_root.join("node_modules")` and derives them from the driver tree instead,
gaining `ACCELERATOR_DESIGN_BROWSER_EXECUTABLE` from the browser tree. The layout
precondition `runtime_is_installed` (`:118-122`) enforces — today exit 3
`playwright-not-installed`, envelope at `cli/design/src/executor/envelope.rs:43,55,66,76-79`
— becomes an `artifact-unavailable` downgrade rather than a hard failure, since the
artifacts are now fetchable. `design-adapters/src/paths.rs`'s `lockhash`, `cache_root`
and `namespace_root` (`:64-129`) go with it, along with
`cli/design-adapters/tests/lockhash_golden.rs`.

#### 4. Failure ordering and the platform probe

**Files**: `cli/design/src/runtime/platform.rs` (new),
`cli/design-adapters/src/platform.rs` (new), `cli/design-cli/src/executor.rs`
**Changes**: ADR-0062 requires the runtime check to come **before**
`design.browser_path` is consulted, because the hatch substitutes the browser and never
the runtime. A musl host must reach the code-only downgrade, not a browser-path error.
Nothing enforces any such ordering today because neither check exists.

Order: platform supported? → runtime available? → browser resolvable (bundled, then
`design.browser_path`)? Each failure emits its downgrade reason, and the default and
hybrid crawler modes fall back to the code-only crawler. An explicit `--crawler runtime`
request hard-fails.

The platform check needs a mechanism that exists nowhere in the codebase today.
`HOST_PLATFORM` (`resolve/mod.rs:21-28`) is a compile-time constant reading `linux-x64`
on Alpine and Debian alike — `TARGETS` builds Linux against `*-unknown-linux-musl`
precisely so one binary runs on every libc — and the manifest's platform axis carries no
libc dimension. Nothing in the existing resolution path can tell the two apart, so
without a probe an Alpine host fetches ~294MB of glibc-linked artifacts, seals them, and
dies at `execve` with a bare `ENOENT` from the absent dynamic loader: the hard failure
AC11 exists to prevent, at maximum cost.

The mechanism, settled by work-item:0210 SQ-1 against prototypes on six hosts, is a
compile-time short-circuit plus **two** filesystem observations, classified musl-first:

1. Non-Linux targets never reach the probe at all — the module is gated on
   `#[cfg(target_os = "linux")]` rather than branching at runtime, so macOS cannot be
   misclassified by construction.
2. The **basename** of `/bin/sh`'s `PT_INTERP`. `ld-musl-*` is positive musl evidence
   and refuses immediately. The basename rather than the path is load-bearing: NixOS
   keeps glibc's loader at `/nix/store/<hash>-glibc-<version>/lib/ld-linux-aarch64.so.1`,
   so any location-based test misclassifies it.
3. Whether the psABI interpreter the artifact demands — `/lib/ld-linux-aarch64.so.1` on
   aarch64, `/lib64/ld-linux-x86-64.so.2` on x86_64, noting the `x86-64`/`x86_64`
   spelling asymmetry against musl — is present and executable.

Order is load-bearing: `gcompat` puts a glibc loader on a musl host, so observation 3
passes there and observation 2 must win. Neither observation alone is sufficient —
observation 3 alone accepts Alpine + `gcompat`, observation 2 alone accepts NixOS.

Reading `PT_INTERP` of `/proc/self/exe` — which an earlier draft of this section
proposed — **cannot work**: `TARGETS` builds Linux static-musl, and a static binary has
no `PT_INTERP`, so the read returns the same "no interpreter" on Alpine, Debian and
NixOS alike. Nor can a loader-path glob: both Debian + `musl-tools` and Alpine +
`gcompat` carry both loaders, so the glob is undecidable on each.

An **ambiguous host fails open** — it attempts the glibc runtime and lets `execve` fail
— matching every installer surveyed (`rustup-init.sh` defaults to `gnu`,
`nodejs/unofficial-builds` treats any `ldd` failure as glibc, `detect-targets` returns
glibc first). With two observations the ambiguous case reduces to one host shape: musl
**and** a static `/bin/sh` **and** `gcompat`.

**A third downgrade reason is required.** NixOS is a fully supported libc whose loader
is not where the artifact demands it — a real glibc-linked binary exits 127 there with
`cannot execute: required file not found`. Its remediation is `nix-ld` or
`design.browser_path`, not "use a different distribution", so it cannot share
`unsupported-platform`'s vocabulary entry. §6 carries the added reason.

The classification is a **pure function** in
`cli/design/src/runtime/platform.rs` over observations the adapter supplies, unit-tested
over injected inputs for every shape **including macOS**. That is the sub-domain
`downgrade.rs` already occupies — `runtime/` survives the sibling's delivered layout
holding exactly that one module, while `executor/` sits at the crate root — and the
domain crate's pup rule (`cli/pup.ron:231-245`) permits only `std`/`core`/`alloc`,
`kernel::Error` and `crate`, so every observation arrives through a port implemented in
`design-adapters`. The Alpine container fixture confirms wiring but cannot on its own
distinguish "detected musl" from "failed for some other reason", which is why the unit
test carries the property. And the probe runs **before** any artifact resolution, so an
unsupported host downgrades at zero network cost.

#### 5. `design.browser_path`

**Files**: `cli/config/src/catalogue.rs`, `scripts/config-defaults.sh`,
`cli/launcher/tests/fixtures/dump/dump.golden`, `docs-site/…/design.md`
**Changes**: Add to `EXTRA_KEYS` (`catalogue.rs:121-133`, today eleven entries) — no
default, presence-only, exactly like `visualiser.editor`. That costs the catalogue
entry, a mirror at `scripts/config-defaults.sh:208-220`, a row in the dump golden, and
docs. It does **not** touch `assert_eq!(count, 55)` (`catalogue.rs:259-269`) or the
Rust↔bash drift test: `EXTRA_KEYS` is deliberately excluded from that count because keys
with no catalogue default do not participate. A catalogue *default* would cost a new
group, an entry in `default_for`'s hardcoded group loop, a `dump::assemble` arm, two
extra drift-test loops, the count bump **and** the test's own name, which encodes the
number — so `EXTRA_KEYS` is the route.

The `ACCELERATOR_DESIGN_BROWSER_PATH` env override is **not** a config-layer concern:
`config-adapters` reads exactly one env var and `store.rs:195-205` documents that as the
rule. `cli/visualiser/server/src/compose.rs:216-252` (`resolve_optional`) is the exact
env-beats-config shape, whitespace collapse included — but it is **extracted into a
shared crate with its tests** rather than copied verbatim. Copying logic while leaving
its tests at the original site is how two copies drift, and this precedence is the
mechanism AC12 rests on. If extraction proves disproportionate, the fallback is explicit
precedence tests at the new site over env set/unset × config set/unset ×
whitespace-only, so a mutation in either copy fails locally.

#### 6. Downgrade vocabulary, PROTOCOL.md and the skill

**Files**: `cli/design/src/runtime/downgrade.rs`, `cli/design-cli/src/cli.rs`,
`cli/design/tests/fixtures/notify-downgrade/*`,
`cli/design/tests/fixtures/notify-downgrade-messages.json`,
`cli/design/tests/downgrade_goldens.rs`, `cli/design/tests/fixtures/public-api.txt`,
`skills/design/inventory-design/evals/evals.json`,
`skills/design/inventory-design/evals/benchmark.json`,
`skills/design/inventory-design/PROTOCOL.md`,
`skills/design/inventory-design/SKILL.md`
**Changes**: Keep `executor-ping-failed`; drop `node-missing`, `node-too-old` and
`bootstrap-failed`; add `unsupported-platform` (AC11's musl case) and
`artifact-unavailable`.

**`disk-floor-not-met` and `cache-unwritable` are retained.** Both still arise and are
now *more* likely: a first run needs headroom for a ~294MB archive **plus** its
extracted copy — ~600MB peak, more with both trees — and the cache root's unwritability
is already modelled as `CacheRootUnavailable` in the launcher. Today
`ensure-playwright.sh` refuses up front with a named reason; dropping these would mean a
disk-full condition surfaces mid-extraction as a generic `artifact-unavailable`, having
already consumed the remaining free space. So free space is checked *before* a fetch
starts against `archive_size + uncompressed_size` summed over every tree about to be
materialised — not against the archive size alone, which would under-reserve roughly
threefold. A partial temp tree is removed eagerly on failure rather than left to the
reaper.

The vocabulary lives in three coupled places the sibling delivered, and all three move
together: the `DowngradeReason` enum and its `ALL` slice
(`runtime/downgrade.rs:17-36`), the `const fn message` match (`:53-62`) that makes
exhaustiveness a compile error, and the clap `ValueEnum` mirror `DowngradeReasonArg`
(`design-cli/src/cli.rs:84-105`), which the domain crate cannot derive under its pup
rule. The goldens at `cli/design/tests/fixtures/notify-downgrade/<key>.expected.txt` are
exhaustive by construction — `downgrade_goldens.rs:18-22` iterates the enum and
`:49-65` fails on an orphan — so a variant without a golden fails and a golden without a
variant fails.

`notify-downgrade-messages.json` is **deleted** here, with the `include_str!` drift test
that reads it (`downgrade_goldens.rs:71-94`). The sibling relocated it into
`cli/design/tests/fixtures/` precisely so the on-disk contract survived the shell
script's deletion; once the vocabulary is rewritten there is no on-disk file left to
drift against, and keeping one would mean maintaining a second copy of a table the
compiler already makes exhaustive. Its three retained messages also tell the user to
"Run `ensure-playwright.sh` manually", which this phase makes unfollowable.

Changing the enum's variant count changes `cli/design`'s public API, so
`cli/design/tests/fixtures/public-api.txt` — a cargo-public-api snapshot — moves with
it.

Three consumers beyond the enum and its goldens name retired reasons by string:

- **`PROTOCOL.md:584-600`** — the "Detected-Condition → `notify-downgrade` Enum Mapping"
  table, mapping every retired reason to an `ensure-playwright.sh` exit code. It is the
  executor's published contract, so leaving it describing a vocabulary nothing emits is
  worse than the drift the sibling already fixed. Its env-var table at `:630-637` also
  documents `ACCELERATOR_PLAYWRIGHT_CACHE` and `ACCELERATOR_PLAYWRIGHT_NS_ROOT`, both
  removed here. (The table's undocumented exit-2 path — `ensure-playwright.sh:5` and
  `:37` — dies with the script rather than needing documenting.)
- **`evals/evals.json`** — eval 20, `executor-bootstrap-failure-fallback` (`:196-200`),
  expects the literal `bootstrap-failed` downgrade message and drives it with
  `ACCELERATOR_PLAYWRIGHT_MOCK_NPM_EXIT=1`. Retargeted onto `artifact-unavailable`,
  which now covers its scenario, rather than deleted. Eval 21 (`:210-215`) names
  `ensure-playwright.sh` in its prompt and needs the same treatment.
- **`evals/benchmark.json`** — ⚠️ **fifteen** stale references, not the six an earlier
  draft claimed, and eleven of them are already stale *today* rather than being made
  stale by this phase. Six name `validate-source.sh` (`:1738`, `:1743`, `:1781`,
  `:1786`, `:1824`, `:1829`) and nine name `run.sh` (`:1877`, `:1915`, `:1953`, `:1986`,
  `:1991`, `:2024`, `:2029`, `:2062`, `:2067`) — both deleted by the sibling, which
  updated `evals.json` and missed this file. Three more name `ensure-playwright.sh`
  (`:1981`, `:2019`, `:2057`) and three name `bootstrap-failed` (`:1867`, `:1905`,
  `:1943`), which this phase retires. All fifteen are corrected here. Nothing in CI
  catches them: `scripts/test-skill-frontmatter-conformance.sh:569-574` asserts only that
  the file exists and is valid JSON, never that its content names a live command.

`SKILL.md` Step 4 (`:126-143`) also changes here. It invokes `ensure-playwright.sh` and
parses its `ACCELERATOR_DOWNGRADE_REASON=` stderr line (`:136`). With bootstrapping moved
to build time there is no bootstrap step to run, so Step 4 is replaced by the executor's
own ordering and the reason is read from the executor's envelope. The
`allowed-tools` grant at `:15` — the last `scripts/*` grant across both design skills —
is dropped in the same edit, and the conformance assertion that pins it
(`scripts/test-skill-frontmatter-conformance.sh:551-553`, with its `SC2016` disable at
`:551`) is deleted with it. Any two of those three without the third reddens
`test:integration:config`; see Current State Analysis.

#### 7. `design notices`

**File**: `cli/design-cli/src/notices.rs` (new), `cli/design-cli/src/cli.rs`
**Changes**: `accelerator design notices [--artifact driver|browser]` prints the paths of
the `NOTICES/` directories Phase 2 assembles into each tree, and lists the components
covered. This is what makes AC16's "reachable by a user without unpacking the artifact by
hand" true; it lands here rather than in Phase 2 because the trees it reads do not exist
on a user's machine until this phase.

It is the seventh of the seven subcommands work-item:0196's Drafting Notes record, and
the only one the sibling did not deliver — `cli/design-cli/src/cli.rs:19-79` exposes six
today. AC1's "at least one success and one failure path per subcommand" applies to it.

#### 8. Deletion

**Files**: `skills/design/inventory-design/scripts/ensure-playwright.sh`,
`skills/design/inventory-design/scripts/test-ensure-playwright.sh`,
`skills/design/inventory-design/scripts/playwright/package-lock.json`,
`tasks/test/integration.py`
**Changes**: Deleted. With them go the lockhash namespace under
`${ACCELERATOR_PLAYWRIGHT_CACHE:-$HOME/.cache/accelerator/playwright}`, the sentinel
idempotency contract, the disk floor, the node-version floor and the sweep.

`scripts/test-design.sh`'s only remaining statement is the `test-ensure-playwright.sh`
delegation, so the file dies here rather than in the Removal sweep — there is nothing
left in it to sweep.

**The opt-in runtime lane's preflight resolves the namespace this phase deletes.**
`tasks/test/integration.py:414-421` computes the lockhash namespace from
`package-lock.json` and `:434-444` refuses the lane when
`node_modules/playwright/index.js` is absent, with the refusal message at `:438-443`
telling the user to run `{_PLAYWRIGHT_DIR}/ensure-playwright.sh`. That message is
**already wrong** — it names `scripts/playwright/ensure-playwright.sh` while the script
lives one directory up — and after this phase there is no script to name at all. The
preflight is repointed at the materialised driver tree, resolved through
`accelerator cache ensure driver` or its exported variable, so an absent runtime remains
a visible refusal rather than a silent pass. `_DESIGN_AUTOMATION_RUNTIME_SUITES`
(`:406-409`) is unchanged.

### Success Criteria

#### Automated Verification

- [ ] Failing tests first for the failure-ordering state machine, at unit level over
      injected platform, runtime and browser resolution, so the ADR-0062 ordering is
      pinned in a fast test rather than only in a container
- [ ] The platform classification returns the right answer for every injected shape —
      **macOS**, Debian, Debian + `musl-tools`, Alpine, Alpine + `gcompat` (which must
      refuse despite a present glibc loader), and a relocated-loader host such as NixOS
      (which must refuse with the loader-absent reason, not the libc one)
- [ ] An unsupported platform downgrades without issuing any HTTP request
- [ ] A non-executor design subcommand performs no tree resolution and no fetch on an
      empty cache
- [ ] With no tree variables set (the `ACCELERATOR_DESIGN_BIN` override path), the
      executor reaches `cache ensure` rather than failing
- [ ] `ensure`'s distinct failure causes map to distinct downgrade reasons
- [ ] A container fixture with Node absent from `PATH` fetches both artifacts, launches
      the headless shell, and emits the envelopes the sibling pinned (AC6)
- [ ] A musl/Alpine container fixture emits `unsupported-platform` and completes via the
      code-only crawler with a non-error exit — and does so with `design.browser_path`
      both set and unset (AC11)
- [ ] On a glibc host with the bundled browser unavailable and `design.browser_path`
      pointing at a system Chromium, the runtime crawler runs against that executable
      (AC12)
- [ ] `--crawler runtime` hard-fails on an unsupported platform
- [ ] Each artifact downloads at most once per platform per version (AC9)
- [ ] `chromium` is a defined export of the module `daemon.js` resolves
- [ ] `daemon.js` launches with an explicit `executablePath`, and the value it receives
      is the one Rust resolved — asserted for both the bundled tree and the
      `design.browser_path` hatch, since AC12 depends on it
- [ ] `ping` succeeds when `playwright-core`'s registry path does not exist, proving the
      handler checks the launch path rather than `executablePath()` — the regression that
      would silently degrade every crawl to code-only
- [ ] A launch succeeds against a read-only browsers root, proving an explicit
      `executablePath` bypasses registry validation and writes
- [ ] `resolve_optional`'s precedence is tested over env set/unset × config set/unset ×
      whitespace-only, at whichever site owns it
- [ ] `design notices` has a success path and a failure path, including `--artifact`,
      over a fixture tree
- [ ] A persistent materialisation failure produces **one** fetch attempt per session,
      not one per executor invocation
- [ ] A free-space shortfall emits `disk-floor-not-met` before any fetch starts, and an
      unwritable cache root emits `cache-unwritable`
- [ ] Tree-failure envelopes name `accelerator cache repair <name>`
- [ ] The downgrade goldens stay exhaustive by construction across the vocabulary change
      — a variant with no golden fails, and an orphan golden fails
- [ ] The retired reasons and the deleted script names appear nowhere in `evals.json`,
      `benchmark.json` or `PROTOCOL.md`, and eval 20 passes against
      `artifact-unavailable`
- [ ] `cli/design/tests/fixtures/public-api.txt` is regenerated and committed
- [ ] `mise run test:unit:design-automation` passes with the loader suite removed and
      **both** floors moved — suites 9 → 8 and cases 76 → the new TAP-reported total
- [ ] `mise run test:integration:design-automation` still fails rather than skips when no
      runtime is available, with its preflight resolving the driver tree rather than the
      deleted namespace
- [ ] `mise run cli:check` exits 0
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] A full inventory crawl on a machine with no system Node produces the same artefacts
      as one on a machine with Node installed
- [ ] First-run download completes within a stated wall-clock ceiling at the stated
      minimum throughput (the same floor Step 1a's deadline encodes), with host and
      connection recorded — a pass/fail bound, not an observation
- [ ] `accelerator design notices` reaches all three licence sets
- [ ] Deleting one file from a sealed tree, then running `accelerator cache repair`,
      restores a working crawl

---

## Removal sweep

### Overview

The residue this plan owns: the floor arithmetic, the acceptance of the two ADR
supersessions work-item:0210 raised (ADR-0061 and ADR-0062), the documentation of the
removed prerequisite, the final no-`.sh` assertion, and the follow-up items this work
surfaces.

### Changes Required

#### 1. The floors

**File**: `tasks/test/integration.py`
**Changes**: Phase 3 §8 deletes `scripts/test-design.sh` along with the suite it
delegates to. `scripts/` discovers exactly 15 suites today against a floor of exactly 15
(`:44`), so the deletion lands discovery at **14** and
`_require_suite_floor` (`:80-106`) fails unless the floor moves to 14 in the same change.

This corrects the arithmetic this plan previously carried, which said the floor stays at
15 with no edit. That was true when written and stopped being true when the sibling's
Phase 3 retired `test-metadata-helpers.sh`. The sibling plan's own Removal sweep recorded
the corrected handoff — "14-against-15, not 15-against-15" — and the current numbers
confirm it. The floor's docstring (`:32-43`) records each movement and gains a line for
this one.

`test-design.sh` is not in `_REQUIRED_CONFIG_SUITES` (`:66`), so no by-name gate moves.

#### 2. Documentation

**Files**: the `docs-site/src/content/docs/` pages describing the Playwright
prerequisite, `README.md`, `CHANGELOG.md`, `.claude-plugin/plugin.json`
**Changes**: `plugin.json:11` declares the `Node >= 20` requirement this plan removes —
it goes. Every page describing the bootstrap step, the lockhash namespace or the disk and
node-version floors is repointed at the vendored artifacts, the `cache` verbs and the
`design.browser_path` hatch. The `design` docs page the sibling created gains the
artifact and cache sections; `ACCELERATOR_CACHE_DIR` is documented as
**trust-relevant** — it must be a private, user-owned path — not merely as a
longer-lived location.

#### 3. ADR and work-item amendments

**Files**: `meta/work/0196-accelerator-design-inventory-gap-tooling-cli.md`
**Changes**: The ADR work this plan owed is **already done** — two supersessions and
one new decision, all three accepted — so nothing remains here but the work-item
text.

- **ADR-0061** (signed content-addressed tree generations) supersedes ADR-0060. It
  records content-based addressing with a per-release pointer, the cross-version tree
  **adoption** that follows on a shared root and the layout version that makes it safe,
  the signed
  attestation under the embedded release key (0.17% of the warm budget), and the shared
  `flock` lease that ADR-0060's repair story assumed without naming.
- **ADR-0062** (browser automation's platform boundary) supersedes ADR-0057. It restates
  the boundary as a conjunction — glibc libc **and** a resolvable psABI interpreter —
  because ADR-0057's "glibc-only" framing wrongly admits a relocated-loader host.
- **ADR-0063** (plugin-version-scoped artifact cache) is new, not a supersession. It
  decides that trees stay in the per-plugin-version root with eviction delegated to
  Claude Code's ~14-day orphan sweep, and that `accelerator cache prune` owns the two
  roots that sweep never reaches — a relocated `ACCELERATOR_CACHE_DIR` and a symlinked
  development checkout. Each plugin version therefore materialises its own copy and an
  upgrade re-fetches, which is the accepted price of coupling an artifact's lifetime to
  the plugin version's.

All three are accepted. Work-item:0196's Requirements bullet restating version+digest
addressing is corrected in step, and its `ensure-playwright.sh` and `Node >= 20`
references retired.

#### 4. Final state assertion

**File**: `tests/unit/tasks/test_call_site_migration.py`
**Changes**: Assert `skills/design/` contains no `.sh` file, so a future reintroduction is
caught. This is only true once this plan lands — the sibling left `ensure-playwright.sh`
behind by design.

The two conformance guards the sibling added (`test-skill-frontmatter-conformance.sh:619-664`
and `:666-699`) remain and keep working: with no `scripts/*` grant and no script-shaped
call site in either design SKILL.md, both iterate empty sets rather than becoming vacuous
assertions about a directory that no longer exists.

#### 5. Follow-up work items

**Changes**: Two this plan surfaces and does not fix. Neither exists yet — 0206, 0207,
0208 and 0209 cover other things — so raising them is a deliverable of this sweep rather
than a note:

- **The pinned runtime ages silently.** `playwright-core`, Node and Chromium are pinned
  by exact version and hash, and Phase 2 §8's reuse path skips the fetch-and-verify
  entirely while the pins are unchanged — so nothing re-evaluates them for known
  vulnerabilities, and the only route to a newer engine is a human bumping a pin.
  `cargo-deny` covers Rust crates only. The follow-up adds a scheduled guard that fails,
  or opens an issue, when a pinned revision exceeds a stated age or appears in an
  advisory feed, and records the pin in `RELEASING.md` as a security-relevant dependency
  with an owner and a maximum age.
- 🔒 **`design.browser_path` is settable from committed team config.**
  `.accelerator/config.md` is repo-tracked, and Phase 3 §2 passes the value into
  `chromium.launch({ executablePath })` — so opening an untrusted repository and running
  the inventory skill executes a binary that repository named. `visualiser.editor` sets
  the same precedent, so this extends an existing hazard rather than inventing one, but
  it extends it to a path executed automatically by a skill designed to be pointed at
  unfamiliar projects. The follow-up restricts the key to the personal (gitignored) level
  or refuses a value resolving inside the repository, and audits the visualiser keys
  alongside.

#### 6. Relationship to work-item:0208

**Changes**: No code. Work-item:0208 records that
`test:integration:design-automation` runs in no build — it is deliberately absent from
the `test:integration` roll-up (`mise.toml:270-276`), asserted as excluded-with-a-reason
by `tests/unit/tasks/test_mise.py:179-193`, and six defects accumulated behind it — and
names this plan's container lane as one candidate approach.

The two must not each propose the same CI job. This plan's container lane exists for AC6,
AC11 and AC12 and provisions a runtime as a side effect; 0208's requirement is that the
runtime suites run somewhere on every build. If 0208 lands first, this plan's lane
consumes its job rather than adding a second; if this plan lands first, 0208's acceptance
criteria are satisfied by pointing at the container lane. Record the chosen direction on
0208 and cross-reference it here rather than leaving two documents proposing one job.

While in that file, correct 0208's two stale citations: its Context (`:34`) and its
acceptance criterion (`:101`) both cite `mise.toml:258-261` for the exclusion comment,
which now sits at `:270-273`.

### Success Criteria

#### Automated Verification

- [ ] Failing test first for the final-state assertion
- [ ] `mise run test:integration:config` passes with `_EXPECTED_CONFIG_SUITES` moved to
      **14** and `test-design.sh` absent from the discovered suites
- [ ] Both design-script conformance guards still pass with both design skills carrying
      no `scripts/*` grant and no script-shaped call site
- [ ] `mise run test:unit:build-system` passes
- [ ] `mise run lint:scripts:exec-bits:check` exits 0
- [ ] `mise run docs:check` exits 0
- [ ] **No `.sh` file remains under `skills/design/`**
- [ ] `git status --porcelain -uall` is clean after a tree materialisation in a dev
      checkout, so the trees directory under the cache root is genuinely ignored
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] The docs site builds and every design page's links resolve
- [ ] A fresh plugin install with no system Node completes an inventory run
- [ ] work-item:0196 no longer describes a scheme the code does not implement (ADR-0061,
      ADR-0062 and ADR-0063 are already accepted)
- [ ] Work-item:0208 records which of the two lanes owns the CI job

---

## Testing Strategy

### Unit Tests

- Tree materialisation in `cli/launcher/` against synthetic tarballs: rejection before
  extraction, the entry-type allowlist's full rejection set (including PAX/GNU long-name
  records and duplicate-path entries, which is where tar CVEs live), attestation and
  table round-trip, a crash injected at each step of the publish sequence, single-flight
  with a failing winner, pointer validation, `verify`'s detection of each corruption
  shape including a rewritten `.files` row, and repair's new-generation swap against a
  live reader. These exercise `resolve/tree.rs` directly with **no signing step**, so
  they must not inherit `resolution.rs:255-265`'s `skip_if_no_minisign!` guard, which
  returns `Ok(())` with only an `eprintln!`.
- Platform classification in `cli/design/src/runtime/platform.rs` over injected
  observations — the pair being the shell interpreter's basename and whether the demanded
  psABI interpreter is executable. Six shapes are pinned without a container: macOS,
  Debian, Debian + `musl-tools`, Alpine, Alpine + `gcompat` (musl must win over a present
  glibc loader) and a relocated-loader host. So AC11's musl case, the Mac case and the
  gcompat ordering all hold in a fast test, and the spike's own prototype tests
  (seven, over these shapes) transfer directly.
- Upstream verification in `tasks/` against recorded fixtures. Node/GPG is fully
  offline-verifiable against the committed key, so it is tested for real rather than
  mocked, including the revoked-key and expired-key negatives that `VALIDSIG` alone would
  accept. The SLSA check contacts a transparency log, so its runner is injected and both
  branches asserted — and the plan records that the attestation's *content* is not
  verified in tests.

### Integration Tests

- End-to-end resolution against a `MockServer` and a real minisign keypair, following
  `cli/launcher/tests/resolution.rs`, extended with the stall case `Route::Stall`
  (`tests/common/mod.rs:30-32`) makes reachable and currently nothing uses.
- Probe-count coverage extended rather than replaced: the `probes_during` harness
  (`resolution.rs:199-213`) gains a tree-`locate` case asserting zero and a tree-
  `materialise` case asserting one, so work-item:0189's guarantee still has teeth on the
  new path.
- An assembly round trip: a synthetic tree through the real assembly path, a manifest
  through the real `build_manifest`, signed with a test key, resolved by the launcher's
  tree resolver — so the two halves of the artifact contract are verified together rather
  than only by hand.
- Container fixtures: Node-absent glibc (AC6), musl/Alpine (AC11), and
  bundled-browser-unavailable with `design.browser_path` set (AC12), in their own CI job
  with a preflight that **fails rather than skips**. The artifact-serving component and
  its binding must be named — the launcher's `MockServer` is a `#[cfg(test)]` type bound
  to loopback and is not reachable from a container nor callable from an invoke task.
  AC11 needs no artifacts at all, since the platform probe downgrades before any
  resolution.
- The retained `lib/*.test.js` suites plus `test-run.js` and `daemon-runtime.test.js`,
  moved into that container lane where a runtime exists and zero skips can be asserted
  across the whole set — subject to the 0208 coordination the Removal sweep §6 records.

### Manual Testing Steps

1. Time a warm executor invocation before and after Phase 1 with work-item:0205's method
   and confirm no regression against work-item:0186's bootstrap target.
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
immutable bytes, which is why ADR-0060 exempts trees. The hit path is therefore local
reads plus stats, loads no manifest, and — new in this revision — **probes no cache
root**, which also keeps a populated cache working offline; the per-entry file table is
deliberately not on that path, so its cost does not scale with an artifact's file count.
Because the launcher exports tree paths on every dispatch, that cost is charged to
`accelerator vcs guard` (a PreToolUse hook) and every SessionStart hook, not only to
design, so the export is confined to the external-dispatch path and driven from the
compiled-in artifact set rather than a directory scan.

**Launcher binary size.** `cli/Cargo.toml:182-184` records that the bootstrap hashes the
whole launcher on every invocation, so binary size is a per-call latency term and the
cold-fetch payload. It sets no threshold, so Step 1b §1 derives one — from a **measured**
per-MB slope, not from work-item:0186's non-method-comparable figure.

**First run.** ~294MB per platform. On the default cache root — inside the versioned
plugin tree — a plugin upgrade discards it, and this plugin pre-releases often.
`ACCELERATOR_CACHE_DIR` is the escape, and content-addressed naming is what makes it
actually work: the driver and browser change only when the pinned `playwright-core`
changes, so an upgrade that leaves the pin alone resolves the same digest and hits.

**The release job.** It runs the whole pipeline twice per stable release, so roughly
2.4GB of assembly and upload, on a `macos-latest` runner with no `timeout-minutes` today.
Phase 2 adds one and a whole-job disk assertion, and removes the duplication itself: the
second pass reuses the first's archives by digest, and an unchanged pin triple skips the
upstream fetch entirely.

## Migration Notes

Existing installs carry a populated
`${ACCELERATOR_PLAYWRIGHT_CACHE:-$HOME/.cache/accelerator/playwright}/<sha8>` namespace
that nothing will read after Phase 3. It lives outside the plugin tree so plugin pruning
will not reclaim it. Phase 3's documentation names the path and states it is safe to
delete; no automated removal is added, consistent with not building destructive-op UX
where the filesystem makes recovery trivial. `accelerator cache prune` reports it with
its measured size and the exact command, so the reclamation is discoverable at the moment
a user is already thinking about cache space.

`SKILL.md` Step 4's `ensure-playwright.sh` bootstrap and its
`ACCELERATOR_DOWNGRADE_REASON=` stderr protocol are replaced in Phase 3 §6, together with
the `allowed-tools` grant that keeps them reachable and the conformance assertion that
pins the grant. All three edits land in one change, because either of the two guards the
sibling's validation added fires on any subset.

## References

- Work item: `meta/work/0196-accelerator-design-inventory-gap-tooling-cli.md`
- **Blocking spike**: `meta/work/0210-settle-the-vendored-runtime-tree-artifact-mechanisms.md`
- Sibling plan (implemented and merged):
  `meta/plans/2026-08-11-0196-design-cli-migration.md`
- Sibling validation (the source of several corrections here):
  `meta/validations/2026-08-11-0196-design-cli-migration-validation.md`
- Superseded plan and its three-pass review:
  `meta/plans/2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli.md`,
  `meta/reviews/plans/2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli-review-1.md`
- Research: `meta/research/codebase/2026-08-11-0196-design-cli-implementation-surface.md`
- Measurement method: `meta/work/0205-close-the-warm-dispatch-measurement-method.md`,
  `meta/migrations/0196-warm-path-measurement.md`
- Related: `meta/work/0208-runtime-test-lane-absent-from-every-build.md`
- **ADR-0061** (signed content-addressed tree generations), **ADR-0062** (browser
  automation's platform boundary) and **ADR-0063** (plugin-version-scoped artifact
  cache) are the governing decisions, all three accepted; ADR-0059 (build-time assembly
  of vendored browser artifacts) is accepted and unaffected. ADR-0060 and ADR-0057 are
  superseded by 0061 and 0062 respectively, and are cited in this plan only where it
  quotes what they said.
- Release-pipeline template:
  `meta/plans/2026-07-06-0165-multi-binary-distribution-and-release-pipeline.md`
