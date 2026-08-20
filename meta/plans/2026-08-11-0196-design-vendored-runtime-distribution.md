---
type: plan
id: "2026-08-11-0196-design-vendored-runtime-distribution"
title: "accelerator-design: Vendored Runtime Distribution Implementation Plan"
date: "2026-08-11T21:49:36+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0196"
parent: "work-item:0196"
blocked_by: ["work-item:0205"]
derived_from: ["codebase-research:2026-08-11-0196-design-cli-implementation-surface"]
relates_to: ["plan:2026-08-11-0196-design-cli-migration", "work-item:0208", "plan-review:2026-08-11-0196-design-vendored-runtime-distribution-review-1"]
supersedes: ["plan:2026-08-11-0196-accelerator-design-inventory-gap-tooling-cli"]
tags: [rust, design, playwright, launcher, release-pipeline, tree-artifacts, distribution]
revision: "2cd521542aea64abb6cd81dc104505d8c7609519"
repository: "accelerator"
last_updated: "2026-08-19T00:00:00+00:00"
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

**Phase 0 was lifted out into work-item:0214, which is now closed.** It carried four
questions that were answered wrongly on paper twice across three review passes, and its
own deliverable was a spike rather than shipped behaviour, so it became a spike work item
following the precedent work-item:0205 set for the warm-dispatch measurement method. All
four are settled against prototypes on real hosts, and the sections they govern now
specify a mechanism rather than a candidate. Every one of the four candidates the plan
previously carried was falsified: the libc probe could not have worked at all
(a static-musl binary has no `PT_INTERP` to read), the reaper's pid gate repeated a
failure already recorded in `meta/notes/`, the seal was not the discriminator it was
described as, and the trust-root shape had an in-repo precedent the plan had not found.

**Two eight-lens review passes reshaped three mechanisms; the ones below are the
survivors.** Pass 2's finding was that the fix for pass 1's attestation defect had itself
put `release_version` and `layout_version` into a producer-signed document without asking
who knows those values and when — which made cross-version adoption impossible, gave the
assembling job a field it cannot compute, and turned any future layout bump into an
unbreakable re-materialisation loop. The attestation now binds identity and content only;
rollback is anchored on a compiled-in expected digest; the pointer is digest-keyed; and
the `.files` table ships inside the archive so the archive signature covers it.

**Pass 1's ten critical defects, for the record.** The design survived again; three mechanisms did not. The
attestation could not have been verified as specified — the manifest's signature covers
archive bytes the launcher deletes — and would not have bound artifact identity, platform
or release version if it could, leaving an unsigned pointer free to roll back to an older
release's generation. Phase 3 never repointed the executor at the vendored Node (`const
NODE` is a second threading site the plan asserted did not exist) and would have broken
module resolution outright, since Node's ESM resolver ignores `NODE_PATH`. And Phase 2's
functional gate was inexpressible: a `permissions: {}` job cannot gate a step inside the
job producing its inputs. Assembly therefore moves upstream, the attestation becomes a
producer-side signed document binding the full tuple, and the `flock` lease — which no
algorithm in the previous draft actually acquired — is placed per ADR-0061 and taken on
both the warm and `ensure` paths.

**Phases are renumbered.** They previously kept the numbers they carried in the
superseded plan (4, 5, 7) so that cross-references to the sibling stayed valid; the
sibling has merged, so the gaps now cost more than they save. Old Phase 4 is Phase 1,
old Phase 5 is Phase 2, old Phase 7 is Phase 3, and `Step 4a`/`4b`/`4c` are `Step
1a`/`1b`/`1c`. Anything in the merged sibling plan referring to "the sibling plan's
Phase 7 §6" means Phase 3 §6 here.

## Implementation Progress

Updated 2026-08-20. Criteria ticked: **Phase 1 44/68**, Phase 2 0/67, Phase 3
18/50, Removal 9/14. **Phase 2 is now structurally complete** — the whole
verification, assembly, publish-path, fetch-orchestration and CI-workflow code
is committed and green; its criteria stay unticked only because they assert
release-lane behaviour that needs the human-gated trust anchors and a live
release to exercise (see the Phase 2 progress notes below). **Phase 3's
self-contained domain layer is committed and green** (the downgrade vocabulary,
platform classification, spawn-failure classification, availability ordering,
the config precedence helper, `design.browser_path` registration, and `design
notices` — see the Phase 3 progress note); the remaining Phase 3 work is the
executor/launcher/`daemon.js` integration wiring and the container harness. Two
amendments were made to the plan during implementation,
recorded inline at the sections they touch rather than here: the `-S -H`
prehash distinction does not exist (pinned `minisign 0.12` prehashes under a
plain `-S`), and reqwest's async-builder `read_timeout` panics on the blocking
path so the per-request timeout supplies the idle bound directly. `minisign-verify`
also exposes no caller-supplied-prehash entry point, so the streamed body carries
sha256 alone and BLAKE2b state lives in a per-attempt sink.

**Phase 1 (launcher tree artifacts) — code-complete.** The whole resolver, the
`accelerator cache` built-in, and their adversarial/concurrency/crash coverage
are implemented and green (`cli:check`, `deny:check`, `pup:check`, and the
build-system suite). Built under `cli/launcher/src/launch/`: `core/tree.rs` (the
self-classifying `TreeError`, the three ports, `Clock`), `core/tree_entry.rs`
(the pure admission allowlist), the nine-module `outbound/resolve/tree/` adapter
(layout, table, attestation, extract via `openat`+`O_NOFOLLOW`, seal,
lease+single-flight `flock`, reap, pins, claims, download with `Range` resume,
and `resolver.rs` orchestrating `acquire`/`query`/`materialise`/`verify`), and
`launch/cache.rs` wired through `dispatch`'s `run_cache` closure. The compiled-in
digest map is generated by `build.rs` from a committed `pins.toml`. Coverage
includes the crash-injection seam across all seven publish steps, the bounded
single-flight waiter, two concurrent cold resolutions issuing exactly one fetch,
the retention-claim window, the `trees/` ownership guard, `verify`'s detection
shapes, and cross-process `Range` resume.

Two Phase 1 deviations, both recorded in their commits. The **pup import
allowlist for the tree adapter was not added** — cargo-pup cannot resolve
imports inside the adapter's `#[cfg(unix)] mod unix {}` inline submodules
(reports empty module paths), so the architectural guarantee rests on the
composition-root type (`acquire_trees` accepts only `AcquireSealedTree`, making
`MaterialiseTree`-in-dispatch a compile error), which the plan already names as
the primary enforcement. A `ExpectedDigests::{Compiled,Fixed}` test seam lets the
end-to-end suite pin a digest; production is always `Compiled`.

**Phase 1's 24 unticked criteria are not missing code.** The **two measurement
gates** — the warm-path bound (the plan marks it BLOCKED, pending 0205's SQ-4
being instantiated) and the binary-size delta ceiling (needs a measured per-MB
verify slope) — require a controlled reference-host benchmark run and were
deliberately **not** fabricated. The **manual/special-environment** checks (RSS
ceiling under a container memory limit, the `flock`-unavailable fallback needing
an NFS/FUSE mount, end-to-end materialisation timing) exercise mechanisms that
exist but assert wall-clock or environment behaviour. A few remaining items are
Phase-3-adjacent (e.g. an `ensure`-resolved tree spared while its *consumer* is
alive depends on the design binary holding the lease).

**Phase 2 (release pipeline) — the artifact contract and GPG check are in; the
rest is not started.** Done and green: the exact playwright version pin (§1);
`tasks/vendor/gpg.py` — the `classify_status_lines` pure predicate plus an
injected-runner wrapper (§2 Node bullet); `tasks/vendor/archive.py` — the
deterministic `.tar.gz` with the `.files` table as its first member, each
normalisation asserted with a negative case (§3, §8); `tasks/vendor/attestation.py`
— the signed document's body, matching the Rust reader's fields (§5 arm 0); the
`manifest.schema.json` artifact `$defs` and the contract test (§6); and — the
load-bearing cross-phase check — a Rust test (`cli/launcher/tests/cross_language_archive.rs`)
that extracts a **real Python-built** archive and deserialises a Python-emitted
attestation, so a table-format or attestation-shape disagreement fails at
`cargo test` rather than in Phase 3's container fixture. Building it surfaced and
fixed a trailing-slash mismatch (Python `tarfile` names directories `lib/`, the
table keys them `lib`).

**Phase 2 progress, 2026-08-19.** The **assembly core and all upstream
verification logic are now built and green** in `test:unit:tasks`, added as
tested units since the artifact contract landed:

- `tasks/vendor/assemble.py` — `extract_zip` (the `external_attr` mode +
  `S_IFLNK` symlink reconstruction, with in-root containment §3 warns about),
  the version guards (§4), `NoticeSource`/`TreeSpec`/`stage_tree` composition and
  `write_notices` (§9), `structural_check`/`smoke_check` predicates, and
  `assemble_specs` + `assert_matches_pin` producing flat, deterministically-named
  archives gated against `ASSEMBLED_SHA256` (§3, §8).
- `tasks/vendor/chromium.py` — the pinned-revision cross-check and per-platform
  byte-hash (§2), with `pins.toml` gaining `[chromium]`/`[node]` sections and
  `pins.py` the accessors.
- `tasks/vendor/nodejs.py` — the exact-filename digest match wired onto
  `gpg.verify_detached` with an injected runner (§2 Node bullet).
- `tasks/vendor/npm.py` — **the npm ECDSA decision is settled: `cryptography`
  was added** (exact-pinned, with `requests`, to the build group). Registry
  ECDSA-P256 signature verify, the sha512-integrity binding, and SLSA via an
  injected argv-pinned `gh attestation verify` runner (§2 npm bullet).

**The publish path (§5, §6, §7) is now built and green too.** Added since the
verification core: `manifest.collect_artifact_entries` + additive `artifacts`
key (§5.2); `signing.sign_tree_artifacts` re-deriving each `.sealed` against the
archive before signing archive→`.minisig` and attestation→`.sealed.sig` (§5.0/1);
`archive.read_archive_stats`; `github._tree_artifact_uploads`/`_tree_artifact_reverifies`
threaded through `_release_uploads`/`_release_reverifies`, tree tokens derived
from the manifest's own `artifacts` map so the skip escape works (§5.3/4, §6.6);
`release._sign` signing + collecting trees only when staged, the
`_assert_assembled_matches_pins` `ASSEMBLED_SHA256` gate wired between
`cli_cross_compile` and `create_debug_archives` in both prepares (§3), and the
`_assert_staged_manifest_is_current` full-cross-product arm (§6.1). 🔴 **The
`except Exception` `--cleanup-tag` delete arm is removed outright** — no path
deletes a published tag (§7). The attest globs are asserted to cover the flat
tree archives (§6.2). 2691 tasks tests green; `build-system:check` clean.

**The fetch orchestration and CI workflow are now built and green too.**
- `tasks/vendor/fetch.py` — the streamed `download` + `get_json`, injected everywhere.
- `tasks/vendor/upstream.py` — `verify_upstream_inputs` wiring npm/nodejs/chromium
  verification over the fetch layer, with the URL builders; the SLSA signer
  workflow, Chromium CDN base and per-platform names are marked release-lane
  validated. `npm.packument_dist` parses the registry packument.
- `tasks/vendor/assemble.py` — `extract_tar` + `assemble_tree_artifacts`
  (extract → compose → attest → structural/smoke gate, `run_smoke` off for
  cross-platform assembly) + `default_spec_builder` (the real driver/browser
  layout, glob-based, fails loudly on an unexpected layout) +
  `smoke_downloaded_archives`. The miniature-fixture-triple end-to-end tests run
  in `test:unit:tasks` (§8's predicates).
- `tasks/vendor/commands.py` + `tasks/__init__.py` + `mise.toml` — the
  `vendor:verify-upstream-inputs` / `vendor:assemble-tree-artifacts` /
  `vendor:smoke-runtime` invoke tasks.
- `.github/workflows/main.yml` — the `assemble-runtime` (`permissions: {}`,
  GH_TOKEN only on the SLSA verify step) and matrix `smoke-runtime`
  (`permissions: {}`, native per target) jobs; `prerelease`/`release` gain both
  in `needs:` and a `download-artifact` step feeding `dist/release/` before the
  pin gate. Workflow-shape tests pin the new jobs' permissions, secret
  isolation, `needs:` wiring and matrix coverage. ⚠️ The two artifact actions
  (`upload-artifact`/`download-artifact`) use `@v4` tags and are flagged
  **SHA-pin before merge**; `timeout-minutes` from a measured double-pass is
  still to add (§7).

**Phase 2's only remaining work — human-gated trust anchors:**
`keys/nodejs-release.asc`, `keys/npm-registry.pem`, the real `pins.toml`
Chromium/Node/`ASSEMBLED_SHA256` values, the RELEASING.md refresh procedure and
the trust-anchor CI guard job. Plus the release-lane manual validations the plan
already lists (real-input layout confirmation, the SHA-pins, `timeout-minutes`).
Left as placeholders in code.

**Phase 3 (executor swap) — the self-contained domain layer is built and green;
the integration wiring is not started.** Committed since Phase 2, all tested at
unit level (`cli/design` lib now 112 tests, `public-api:check` green):

- **§6 the downgrade vocabulary** — `runtime/downgrade.rs` rewritten to the
  vendored-runtime reasons (dropped `node-missing`/`node-too-old`/
  `bootstrap-failed`; added `unsupported-platform`, `loader-unresolvable`,
  `glibc-too-old`, `runtime-libraries-missing`, `artifact-unavailable`,
  `materialisation-in-progress`; kept the three retained reasons). Goldens
  regenerated, the `DowngradeReasonArg` clap mirror updated, `public-api.txt`
  regenerated, and `notify-downgrade-messages.json` plus its `include_str!`
  drift test deleted as §6 directs.
- **§4 platform classification** — `runtime/platform.rs` (new): the musl-first
  `classify(Observations) -> Support` over the two observations, unit-tested for
  all seven shapes (macOS, Debian ±musl-tools, Alpine ±gcompat, NixOS,
  distroless). The domain function is not `cfg`-gated so it tests on macOS; only
  the adapter that gathers observations will be Linux-gated.
- **§4 spawn-failure classification** — `runtime/bootstrap.rs` (new):
  `classify_bootstrap_log` hand-parsed (the domain crate's pup rule bans
  `regex`), extracting the glibc version and the missing soname with validated
  tokens and whole-line matching so untrusted output cannot force a downgrade.
- **§4 availability ordering** — `runtime/availability.rs` (new): the ADR-0062
  platform → runtime → browser order over lazy thunks, proving zero-network on a
  refusal and that the hatch substitutes the browser while the driver is still
  ensured.
- **§5 config precedence** — `config::env_beats_config` (new pure function in
  `cli/config`); the visualiser's `resolve_optional` retargeted at it so the
  environment read stays in the composition root and `cli/config` keeps no
  `std::env` read.
- **§5 `design.browser_path`** registered in `EXTRA_KEYS` with its
  `config-defaults.sh` mirror, dump-golden row and a catalogue test. Its
  personal-level-only enforcement and repo-inside refusal are deferred to the
  executor's browser-path read (§2/§3), and its docs to the Removal sweep's
  docs batch.
- **§7 `design notices`** — `design-cli/src/notices.rs` (new): the seventh
  subcommand, reading each `ACCELERATOR_TREE_<NAME>` and listing its `NOTICES/`
  components, with a pure core and CLI success/failure tests.

**Phase 3's remaining work is the integration wiring, much of it exercised only
by the container harness:** §3 tree resolution (the launcher exporting
`ACCELERATOR_TREE_<NAME>`, `design-cli/src/executor.rs` calling `cache ensure`
before `launcher.lock`, retargeting `const NODE` at both spawn sites, launcher
discovery); §4's remainder (`design-adapters/src/platform.rs`'s Linux-gated
observations, the sticky-marker policy, and the `BootstrapDiagnostics` port +
`Spawner` errno wired into `launch.rs`); §2's `design.browser_path` read and
`daemon.js`'s loader narrowing, explicit `executablePath` and `ping` fix (§1);
§8's deletions; the container harness; and the `PROTOCOL.md`/`evals.json`/
`benchmark.json` cleanup plus the standing conformance guard.

**Phase 3 integration wiring, 2026-08-19 — two foundational leaves landed.**
Both are unit-green (`cli:check`, `pup:check`) on darwin; the criteria they
serve that assert integrated or container behaviour stay unticked until the
executor consumes them:

- **The host platform observation adapter** — `design-adapters/src/platform.rs`
  gathers the two observations the committed `classify` consumes (`/bin/sh`'s
  `PT_INTERP` basename and the psABI interpreter's presence), Linux-gated with
  the ELF parser and basename classifier exercised on the build host, and joins
  the `design_adapters_read_in_process` pup rule.
- **The launcher tree-variable export** — a tree-consuming dispatch clears every
  `ACCELERATOR_TREE_<name>` (from the compiled-in set) ahead of the resolve
  path's override short-circuit, then sets each acquired tree's path plus
  `ACCELERATOR_LAUNCHER_PATH`, holding the leases until the consumer takes over
  (`export_consumed_trees`/`acquire_consumed_trees` in `main.rs`, `tree_var` /
  `consumes_trees` / `LAUNCHER_PATH_VAR` in `launch::core`). The collision
  criterion is ticked; the clear-on-override and acquire-only-on-consumer
  criteria wait on the executor/container coverage. **Deviation:** the warm
  export builds a `TreeResolver` (and so a `Fetcher`) whose network side
  `acquire` never uses; a `Fetcher`-free acquire type is the follow-up if the
  0205-gated warm-path measurement demands it.

Two sequencing findings recorded here rather than re-derived later: **§6's
consumer cleanup is entangled with §3/§8** — the eval prompts describe the
Step-4 bootstrap the executor swap removes, so §6 lands *with* §8 after the
executor swap, not before — and the **`cache ensure` structured-envelope
contract needs a cause taxonomy `TreeError` does not yet carry** (a transport
failure is wrapped as `Extraction`, an unwritable cache root as `Lease`), so the
executor-facing envelope is a Phase-1-adjacent enrichment, not a pure add.

**Phase 3 integration wiring, 2026-08-19 (continued) — two more sections
landed, both unit-green on darwin:**

- **The `cache ensure` structured envelope + `TreeError` cause taxonomy** —
  `TreeError` gained `Unreachable` (transport failures, previously a tampering
  `Extraction`) and `CacheRootUnwritable` (previously a `Lease`), each degradable
  rather than a refusal; a new `EnsureCause` enum + `TreeError::cause()` maps
  every variant to a token; `cache ensure` returns a JSON envelope
  (`{"error":"ensure-failed","cause":…,"artifact":…,"message":…}`) on failure,
  rendered to stderr, with `disk-shortfall`/`cache-unwritable`/
  `materialisation-in-progress` distinct and the finer fetch/verification causes
  named for diagnostics. Golden + a 404-driven integration test.
- **The `Spawner` errno + `BootstrapDiagnostics` port + launch classification** —
  `Spawner::spawn` now returns a `SpawnError` distinguishing `NotFound` (an
  `execve` `ENOENT` on a program that exists → `loader-unresolvable`); the launch
  state machine reads the bootstrap log through a `BootstrapDiagnostics` port on a
  readiness failure and classifies it into `glibc-too-old` /
  `runtime-libraries-missing` (else it keeps the timeout envelope, never
  guessing); `LaunchFailure::Downgrade(reason)` carries the verdict and the
  executor renders the reason token for the caller to decide on. `design`'s
  public-API snapshot regenerated. **Deviation:** `BootstrapDiagnostics` lives in
  `design::executor::ports` beside the other launch ports (where it is consumed),
  not in a new `runtime/ports.rs` as the plan's file list anticipated.

**Phase 3, 2026-08-19 (continued) — §1/§2 done and three more domain leaves
landed, all unit-green on darwin:**

- **§1/§2 the automation retarget** — the Playwright loader is narrowed to import
  the driver tree's `playwright-core` ESM entry (`index.mjs`) by absolute path,
  dropping the `exports`-map selection, the CJS-shim branch and the
  `playwright`-vs-`playwright-core` distinction; `daemon.js` launches Chromium
  with the executor-resolved `ACCELERATOR_DESIGN_BROWSER_EXECUTABLE` and its
  `ping` probe checks that launch path rather than `executablePath()`, naming
  `accelerator cache repair` in the diagnostic. The three `fake-playwright*`
  fixtures collapse to one `playwright-core` tree. `test:unit:design-automation`
  is green at 76 cases (the floor needed no numeric edit — see the §1 amendment).
- **§5 the `design.browser_path` hatch policy** — `runtime/browser_path.rs`
  (new): a pure `vet` applying the two security barriers to an already-chosen
  value — a team-level value ignored with a warning naming the personal route,
  and a value resolving inside the inventoried repository refused, including a
  symlink committed inside the repo that points out (caught on the containing
  directory). The env-beats-personal precedence stays in the composition root.
- **§3 the ensure-cause mapping** — `runtime/ensure.rs` (new): `classify_cause`
  maps the launcher's cache-ensure cause token onto a downgrade reason and its
  stickiness, `materialisation-in-progress` the one non-sticky cause, every
  transport/integrity failure and any unrecognised token to
  `artifact-unavailable`.
- **§3/§4 the sticky-marker policy** — `runtime/marker.rs` (new): `suppresses`,
  `cleared_by_successful_ensure` and `is_host_condition`. Fetch/environment
  markers are session-scoped and TTL-bound; host-condition markers are
  digest-keyed and survive a successful materialisation. Every marker is
  session-scoped for suppression (the committed-marker defence). **Deviation:**
  `loader-unresolvable` is treated as a host condition alongside the plan's named
  `glibc-too-old`/`runtime-libraries-missing`, since its post-fetch (spawn-errno)
  arm is the same class of host property and equally expensive to re-attempt.

These are the leaves the executor `run` composes; each is committed and
`public-api`/`pup`/`clippy`/`rustfmt`-clean. A pre-existing rustfmt drift in
earlier Phase-3 sources (the platform adapter, the spawner, the tree resolver
and two tests) was reformatted in the same batch so `format:cli:check` is clean.

**Phase 3, 2026-08-19 (continued) — the §3/§4 executor integration and its
adapters landed, `cli:check` green.** The remaining wiring the earlier note
listed is done:

- **The `cache ensure` adapter** — `design-adapters/src/ensure.rs`: launcher
  discovery (exported path → plugin-root `bin` → `PATH`, pure candidate order),
  the `cache ensure` subprocess, the tab-separated success parse and the JSON
  envelope's cause extraction, tested against a fake launcher.
- **The `MarkerStore`** — `design-adapters/src/marker.rs`: the session key
  (`getsid` + its start time, since the plan left the source unspecified and
  Claude Code exposes none), the symlink/uid path check, and the JSON round-trip
  of the domain marker; joins the `design_adapters` no-spawn pup rule.
- **The executor `run`** — `design-cli/src/executor.rs` rewritten to the
  ADR-0062 ordering over lazy thunks: the platform probe, then the runtime thunk
  (warm exported-variable path, else the cold `cache ensure` guarded by the
  sticky marker — fetch failures TTL-keyed before the fetch, host conditions
  digest-keyed before the spawn), then the browser thunk (hatch, else the bundled
  shell). A `ResolvedRuntime` retargets both spawn sites at the driver tree's own
  `node` and threads `ACCELERATOR_DESIGN_BROWSER_EXECUTABLE`. `design.browser_path`
  is read in `config.rs` through the shared precedence helper and vetted; warnings
  reach stderr.
- **The lockhash namespace is gone** — `HostPaths` no longer computes one,
  `PathResolution` drops `namespace_root`, `lockhash_golden.rs` is deleted, and
  the `playwright-not-installed` envelope is removed (the layout precondition is
  now an `artifact-unavailable` downgrade). The `executor_preflight` tests were
  repointed at the new downgrade behaviour.

**Two Phase-3 deviations recorded here:**

1. **The `flock` lease-hold is deferred to a follow-up.** The executor does not
   yet re-open and `LOCK_SH` the tree's `<generation>.lease` sidecar before the
   spawn. Doing it correctly needs the lease fd inherited by the long-lived
   *daemon* (`FD_CLOEXEC` cleared), and it is only end-to-end verifiable with a
   real daemon and a concurrent `cache prune` — both gated on Phase 2's pins. It
   is a Phase-1-adjacent enrichment, not on the common path, and its absence only
   loses reap/prune protection under concurrency.
2. **`loader-unresolvable` is a sticky host condition**, beyond the plan's named
   two (recorded at the marker note above).

**Phase 3, 2026-08-20 — the §6/§8 cleanup landed, unit-green on darwin.** Two
commits: the design skill's downgrade contract retargeted at the vendored
runtime (PROTOCOL.md's condition→reason mapping and env-var table rewritten to
the nine live reasons, dropping `node-missing`/`node-too-old`/`bootstrap-failed`
and the lockhash env vars; SKILL.md Step 4/5 rewritten so the reason is read
from `executor ping`'s `{"error":"downgrade","reason":…}` envelope rather than
from `ensure-playwright.sh`; the `scripts/*` `allowed-tools` grant and its
conformance assertion dropped; evals 20/21 retargeted off the retired reasons
and deleted scripts; the twenty-one benchmark strings corrected; and a **standing
conformance guard** that fails on a stale script or retired-reason reference in
`evals.json`/`benchmark.json`/`PROTOCOL.md`). Then the §8 deletions
(`ensure-playwright.sh`, `test-ensure-playwright.sh`, `package-lock.json`,
`test-design.sh`) with the `integration.py` preflight repointed at the driver
tree (`ACCELERATOR_TREE_DRIVER` or `cache ensure driver`), still refusing rather
than skipping. Removal §1 (config floor → 14) and §4 (the no-`.sh`-under-
`skills/design/` final-state assertion) landed in the same change, since deleting
`test-design.sh` forces the floor move.

Still ahead in Phase 3: the deferred lease-hold and the container harness
(miniature lane + AC11 run locally on the arm64 Docker engine; AC6/AC12's
real-tree fixtures wait on Phase 2's pins).

**The Removal sweep — the non-blocked work (§1–§6) is done.** §1 (config floor →
14) and §4 (the no-`.sh`-under-`skills/design/` assertion) landed with the §8
cutover. §2 (documentation: the design CLI docs page's runtime-and-cache section,
the nine live `notify-downgrade` reasons, `plugin.json` dropping `Node >= 20`,
the changelog entry — the skill reference pages regenerate from SKILL.md and the
lockhash env vars are gone). §3 (ADR-0064 supersedes ADR-0061's attestation and
pointer; ADR-0061 → `superseded`; work-item:0196's addressing/prerequisite text
corrected). §5 (four follow-up work items raised: 0220 advisory-feed monitoring,
0221 config-key executable-path audit, 0222 offline `cache ensure --from`, 0223
default-cache-root bounding). §6 (work-item:0208 records the container lane as the
CI-job owner, with its stale `mise.toml` citations fixed). What remains in Removal
is gated: `docs:check` (network + Chromium), the clean-`git-status`-after-
materialisation check, a full `mise run`, and the fresh-install manual run.

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

`_EXPECTED_CONFIG_SUITES` is **15** (`tasks/test/integration.py:45`) and `scripts/`
discovers **exactly 15** suites. This plan's earlier text said deleting
`test-design.sh` "takes `scripts/` from 16 discovered suites to 15 against a floor
already at 15" and therefore needs no edit. That is stale by one — it predates the
sibling's Phase 3 retiring `test-metadata-helpers.sh`. **Deleting `test-design.sh`
lands discovery at 14 and fails `_require_suite_floor` unless the floor moves to 14 in
the same change.** The sibling plan's own Removal sweep already recorded this handoff
as "14-against-15, not 15-against-15"; the current numbers confirm it.

`test-design.sh` is not in `_REQUIRED_CONFIG_SUITES` (`:67`), so only the count holds
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
  explicit anti-memoisation test. The tree hit path runs on every **tree-consuming**
  dispatch, so it must probe **zero** times, and `materialise`'s probe must be accounted
  for in those counts rather than discovered by a red test. Note also that
  `PROBE_ATTEMPTS` is a `thread_local!` (`cache_root.rs:74-75`), so a `probes_during`
  assertion is blind to any thread a concurrency test spawns.
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

⚠️ **The environment vector is not the only threading site — the Node executable is
separate, and it is a bare `PATH` lookup.** `const NODE: &str = "node"`
(`executor.rs:28`) is used verbatim as `program: PathBuf::from(NODE)` in **both**
`DaemonSpawner` (`:163`) and `ExecClient` (`:174`). Retargeting only the environment
vector leaves the executor shelling out to a system `node`, which is the prerequisite
this whole plan exists to remove: the spawn would fail with `ENOENT` on an AC6 host
after ~294MB had already been fetched, verified and sealed. Phase 3 §3 therefore
retargets the program fields as well.

### The automation is ESM, so `NODE_PATH` cannot resolve the vendored driver

`skills/design/inventory-design/scripts/playwright/package.json` declares
`"type": "module"` and `lib/daemon.js` is ESM (top-level `import`, `import.meta.url`).
**Node's ESM resolver ignores `NODE_PATH` entirely** — it is a CommonJS-only mechanism.
That is precisely why `playwright-loader.js` exists in the shape it does: it builds an
absolute `pathToFileURL(resolve(nsRoot, 'node_modules/playwright', entryFile))` and
imports that URL (`:63-66`) rather than a bare specifier.

So a bare `import 'playwright-core'` from `daemon.js` would resolve by walking
`node_modules` upward from the plugin tree, never into the sealed driver tree, and every
crawl would fail with `ERR_MODULE_NOT_FOUND` on exactly the machines the vendored
runtime exists to serve. Phase 3 §1 keeps an absolute-path import mechanism rather than
deleting the resolution capability along with the 0072 CJS-shim logic.

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
  (work-item:0214 SQ-4). `cli/vcs-adapters/Cargo.toml:27-33` is the precedent and states
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
- `tasks/signing.py:24-43` signs with `minisign -S`, one file per invocation, under a
  120-second per-file timeout sized for an 8MB binary. The absent `-H` is **not** an
  absent prehash: at the pinned `minisign 0.12` the two forms produce byte-identical
  signatures, and `cli/verify`'s `allow_legacy = false` proves the output is prehashed
  already.
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

Three phases plus a removal sweep. Work-item:0214 is closed, so nothing blocks Phase 1
or Phase 2 from starting.

```
Phase 1 ──┐
Phase 2 ──┴──> Phase 3 ──> Removal sweep
```

Phase 1 (launcher) and Phase 2 (pipeline) are independent of each other and each
leaves the tree green on its own; Phase 3 needs both, because it consumes artifacts
Phase 2 produces through the resolver Phase 1 builds.

**Two things both halves must agree on before either merges**, since either may land
first:

1. **The asset-name convention**, pinned in a fixture both sides read
   (`manifest.example.json`, Step 1a §1).
2. **The attestation document's shape**, pinned in the same fixture — Phase 1 verifies it
   on the hit path and Phase 2 emits it (§5 arm 0), so a field disagreement would surface
   only in Phase 3's container fixture, after both halves had merged.

**One value genuinely crosses the boundary in the other direction**, and it is handled
by handoff rather than by pretending it does not exist: Step 1a's fetch deadline is
derived from measured archive sizes that only Phase 2 produces. Phase 1 therefore ships
an interim value derived from its own ~120MB estimate at a stated throughput floor — a
real bound, not a placeholder — and **Phase 3 carries an explicit criterion that it has
been re-derived**, so the handoff is checked rather than assumed.

Decisions taken during planning, so no phase carries an open question:

- **Tree addressing** is by content digest, platform, layout version and generation, with
  a **digest-keyed** pointer — not a per-release one. The launcher embeds its expected
  `(artifact, platform) → digest` map at build time from the reviewed `ASSEMBLED_SHA256`
  anchor, and resolves only that digest; rollback is refused because a superseded digest is
  never looked for, cross-version adoption works because an unchanged pin yields the same
  digest, and both hold offline. This supersedes ADR-0061's per-release pointer for the
  reasons the Removal sweep records.
- **The signed attestation binds identity and content, never the plugin release version or
  the launcher's layout version** — the first is unknowable in the job that assembles, the
  second is consumer-owned policy that a signed copy would freeze. The `.files` table
  ships inside the archive so the archive signature covers it.
- **The launcher owns materialisation, the design binary owns the decision.** ADR-0061
  puts the embedded key in one holder; **ADR-0062** puts the ordering and the downgrade
  vocabulary in `accelerator-design`. Phase 3 §3 splits them by cost.
- **`disk-floor-not-met` and `cache-unwritable` are retained** in the downgrade
  vocabulary, not dropped. Both still arise and both are now *more* likely.
- **The archive format is `tar.gz`, flat in `dist/release/`**, because the attest globs
  do not cross `/`.
- **Every release signature is already prehashed** at the pinned `minisign 0.12`, where
  `-S` prehashes by default and `-H` is a no-op, so the launcher never buffers ~120MB to
  verify and no signing-side change is needed to make that true.
- **Assembly runs in an upstream job with `permissions: {}`**, not as a step inside
  `release`, which is what makes the functional smoke gate expressible at all.
- **`flock` is the only cross-process liveness mechanism** — for the in-use lease and for
  single-flight alike. No pid gates anywhere.

### A note on comments

Several sections below ask for a derivation, a threshold or a rationale to be recorded
next to the code it governs. Where that is genuinely warranted — an externally imposed
constraint, a non-obvious invariant, a derived bound — the comment must be a
**self-contained statement of the constraint**: no ADR number, no work-item number, no
plan-phase reference, and no host-specific measurement, all of which go stale faster than
the code they annotate. A named constant carrying the bound is preferred to a comment
explaining it. Everything the plan expresses through citations here is scaffolding for
the reader of the plan, not text to transcribe into the source.

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
(`:26-32`), `#[serde(default)]`. `SUPPORTED_SCHEMA_VERSION` stays `1` (`:13`).

**A deliberately-absent platform is represented by omitting the platform key, not by
the all-zeros sentinel digest.** The sentinel (`:16-17`) cannot carry over here: it is
handled by `bare_sha256`, an *inherent method on `PlatformEntry`* (`:48-66`) rather than
a trait or free function, so `ArtifactPlatformEntry` cannot reuse it without an
extraction — and more importantly a sentinel entry has no archive, so its three required
sizes would have no honest value but `0`, which is exactly the value the consumer must
treat as a defect (below). Omission is unambiguous in both directions: an absent
platform key yields `None` at lookup, and the launcher emits `unsupported-platform`
rather than attempting a zero-byte download. `binaries` keeps the sentinel unchanged.

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
(`:146-150`) — the body buffered, transiently twice. Add a streaming path that copies
from the response reader, leaving `get` as a thin wrapper for the existing small-asset
callers.

**The fetcher must own per-attempt reset, so the signature cannot be
`&mut impl Write`.** `get` (`:109-127`) retries up to `MAX_ATTEMPTS` (3), and today each
attempt is safe only because `try_get` returns a fresh `Vec<u8>` — a failed attempt
leaves nothing behind. Writing into a caller-provided sink breaks that invariant: an
attempt that fails partway has already written bytes, and the next appends the full body
after them. The sha256 would catch the result, so nothing unverified is extracted, but
the retry loop could never succeed — a transient blip on a large transfer would become a
permanent, unrecoverable failure presenting as a checksum mismatch.

A `&mut impl Write` cannot express the fix: truncation needs `File`/`Seek`, and the
incremental digest is caller state the fetcher has no way to reset. So the contract puts
both inside the fetcher:

```rust
pub(super) fn get_streaming(
    &self,
    url: &str,
    timeout: Duration,
    open_sink: &mut dyn FnMut() -> io::Result<File>,
) -> Result<StreamedBody, FetchError>;
```

`open_sink` is invoked once at the start of *each* attempt and returns a freshly created
and truncated file; the fetcher owns the incremental digests and returns them in
`StreamedBody` alongside the byte count, so a retried attempt cannot inherit either the
previous attempt's bytes or its digest state. An implementer who follows the signature
gets the correct behaviour rather than having to remember a rule stated in prose.

⚠️ **Two digests are computed over the one stream, but only one of them belongs to the
fetcher.** Prehashed minisign hashes with BLAKE2b-512 while `archive_sha256` and the
manifest comparison need sha256, so both must be computed concurrently or the ~60-120MB
archive is read a second time from disk immediately before ~294MB of extraction writes —
discarding the "no second pass" property prehashing exists for.

The fetcher owns sha256, because that is what bounds and identifies the transfer:

```rust
pub(super) struct StreamedBody {
    pub bytes: u64,
    pub sha256: [u8; 32],
}
```

The BLAKE2b state lives in the sink, because `minisign-verify` at the pinned version
offers no caller-supplied-prehash entry point — only the incremental
`verify_stream` → `update`/`finalize` (see the signature discussion below). Since
`open_sink` yields a fresh sink per attempt, the signature state resets in lockstep with
the file rather than in a second place the implementer must remember.

**The deadline is a throughput floor, not a number picked once.** `TOTAL_TIMEOUT`'s
300s per attempt was sized for a multi-MB binary. It governs the *compressed archive*,
whereas the ~294MB figure is the uncompressed tree — so the value is expressed as "sized
for X MB at ≥N KB/s sustained" rather than as a bare duration.

**Phase 1 ships an interim value and Phase 3 re-derives it, because Phase 2 owns the
measurement and the two phases are independent.** The interim is derived from this
plan's own ~120MB compressed estimate at a 200 KB/s sustained floor — 600s per attempt —
which is a real bound rather than a placeholder, and it is deliberately conservative in
the direction that fails a stalled link slowly rather than failing a slow-but-healthy
one early (the idle bound below is what makes that safe). Phase 3 carries an
explicit criterion that the constant has been re-derived from Phase 2's measured archive
sizes, so the cross-phase handoff is checked rather than assumed.

⏱️ **Bound the total, not only each attempt.** `TOTAL_TIMEOUT` is per attempt and
`MAX_ATTEMPTS` is 3, so an enlarged per-attempt deadline triples: 600s becomes a 30-minute
worst case inside a tool call with no progress output and no cancel. The retry loop
therefore carries its own wall-clock bound across all attempts, and it is the bound the
caller's failure envelope reports.

Make the per-attempt value a per-request override via `RequestBuilder::timeout()` rather than a
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
therefore runs under an idle bound, so the large deadline bounds legitimate slow
transfers while stalls fail quickly. Both numbers are named constants beside the
deadline.

The mechanism needs naming, because a plain byte-counting check between reads cannot fire
while a read is blocked — which is the stall case.

⚠️ **`reqwest`'s `read_timeout` is not available on the blocking builder at the pinned
version.** Verified against the vendored source for `reqwest = "=0.12.28"`:
`pub fn read_timeout` exists only on the **async** `ClientBuilder`
(`src/async_impl/client.rs:1453`); `src/blocking/client.rs` exposes `timeout`,
`connect_timeout` and `pool_idle_timeout` and no idle bound, which is why
`fetcher.rs:12-14` records that blocking reqwest has none. `RequestBuilder::timeout` *is*
present on the blocking path, so the per-request total override above is unaffected.

🔴 **Reaching `read_timeout` through the async builder does not work, and this was
established by running it rather than by reading the source.** An earlier revision
specified exactly that: a blocking client built from
`impl From<async_impl::ClientBuilder> for blocking::ClientBuilder`
(`blocking/client.rs:1190-1197`) with `read_timeout` set on the async side, on the
reasoning that the bound would reach blocking body reads through `async_impl::body`'s
read-timeout wrapper. The conversion compiles and the `From` impl is where the plan said,
but **every streamed body read then panics**:

```text
there is no reactor running, must be called from the context of a Tokio 1.x runtime
  at reqwest-0.12.28/src/async_impl/body.rs:346
```

The read-timeout wrapper arms a Tokio timer on poll, and the blocking `wait` path drives
the future on the calling thread with no runtime context. That is presumably *why*
`read_timeout` is absent from the blocking builder — it is unsupported there, not merely
unexposed. The plan's two conversion caveats below were both correct and both moot.

**The mechanism is instead the per-request timeout, set to the idle bound rather than to
the attempt deadline.** That follows from the second caveat this section already recorded,
read as a capability rather than as a hazard:

- **A per-request timeout becomes a per-*read* bound once the body is streamed.**
  `impl Read for blocking::Response` (`blocking/response.rs:435-444`) wraps each `read` in
  `wait::timeout(..., self.timeout)`. So `RequestBuilder::timeout` is a whole-attempt
  deadline only for the buffered `bytes()` path the existing `get` uses; on the streaming
  path it bounds one read. A bound on one read *is* an idle bound: a slow-but-progressing
  transfer resets it on every chunk, while a connection that stops sending fails within it.
- **`From`'s other semantics stop mattering**, because there is no conversion. The client
  keeps its existing `Client::builder()` construction, so the manifest, sidecar and
  attestation GETs keep their 300s client-level deadline with nothing to re-apply.

So the streaming request passes the **idle** bound as its per-request timeout, and the
attempt and whole-loop deadlines are enforced by the copy loop between reads. That
ordering is what makes the loop's checks reachable at all: they can only run between
reads, so they depend on the per-read bound to guarantee a read returns. Measured against
the scripted stall fixture, a stalled transfer fails in one idle bound rather than waiting
out 600s × 3.

The watchdog fallback is therefore **not taken**, and its cost is recorded only so a
future reader knows what was avoided: the reading thread owns the `Response` by value
while blocked inside `Read::read`, so no other thread can drop it, and the realistic shape
is running the transfer on a spawned thread and abandoning it, leaking a thread, a
connection and a partially-written sink per stalled attempt. The **enlarged per-attempt
deadline was conditional on a working idle bound**, and it has one.

`SO_RCVTIMEO` over an extracted socket is **rejected**, not offered as an alternative:
it is not reachable through blocking `reqwest` over rustls without unsafe socket
extraction, and its partial-read and `EAGAIN`-versus-`ETIMEDOUT` semantics differ across
glibc, musl and Darwin — so it would behave differently on each of the four targets.

The test fixture must **stop sending** rather than trickle, or it exercises the slow
path and passes without ever testing a stall. The mechanism for that already exists
and is currently unused: `cli/launcher/tests/common/mod.rs:30-32` declares
`Route::Stall(Duration)` and `:163-172` serves it, and no test in `resolution.rs`
reaches it today.

🔴 **Every release signature is already prehashed, so the `-H` distinction this section
was built on does not exist.** An earlier revision specified tree archives signed
`minisign -S -H` against single-file binaries signed `-S`, reasoning from the true
observation that `tasks/signing.py:24-43` passes no `-H`. The inference from that
observation was wrong. Measured against the pinned `minisign 0.12` (`mise.toml:35`):

```console
$ minisign -S    -s sec.key -m payload.bin
$ minisign -S -H -s sec.key -m payload.bin
```

produce **byte-identical** `.minisig` files, both carrying algorithm `ED` and a trusted
comment ending `hashed`. At this version `-S` prehashes by default and `-H` is a no-op.

Two things follow, and the second is load-bearing. The repository has been emitting
prehashed signatures all along — which is why `cli/verify/src/main.rs` and
`TrustedKeys::verifies` (`keys.rs:62-69`) both pass `allow_legacy = false` and still work,
since that argument makes the verifier accept *only* prehashed signatures. And the
streaming verification this section wants needs no signing change whatsoever: the archives
are already in the form it requires.

The resource argument the prehash decision rested on stands unchanged and is the reason
the streaming path exists at all: buffering means reading a ~120MB archive into a
`Vec<u8>`, a peak RSS one to two orders of magnitude above anything the launcher does
today, doubled when both trees materialise in one session, in exactly the memory-limited
containers AC6 and AC11 use — where the outcome is an OOM kill mid-materialisation rather
than a diagnosable downgrade.

Cross-process **resume** re-hashes the prefix already on disk rather than restoring a
serialised digest state: neither `sha2` nor `minisign-verify` exposes a resumable
mid-stream state, so "pre-seeded digest state" is not obtainable in a fresh process. That
is a local re-read of at most ~120MB (tens of milliseconds with hardware sha256), not a
second network pass, so the no-second-pass property is about the *network* transfer and
holds. The sink also **retains the longest verified prefix across attempts** rather than
truncating on every one — otherwise an attempt that reaches 100MB followed by one that
dies at 1MB would leave 1MB, and a link failing early on the last attempt could oscillate
without ever converging.

So **Phase 2 §5's signing arm needs no per-target flag at all**, and
`_subbinary_signing_targets()`'s form is untouched because there is nothing to
distinguish it from. The launcher verifies the archive from the streamed digest with no
second pass and no buffer.

Both preconditions were checked as the first step of this change, before any other work in
Step 1a, and both hold — the second more strongly than expected. `cli/verify` accepts a
prehashed signature; being an `allow_legacy = false` caller it accepts *nothing else*.

⚠️ **`minisign-verify` exposes prehashed verification incrementally, not as a
caller-supplied prehash.** At `=0.2.5` the only prehashed entry points are
`PublicKey::verify` (which hashes the contiguous slice itself) and
`verify_stream` → `StreamVerifier::update`/`finalize`. Nothing public accepts a
precomputed BLAKE2b-512 digest. So the earlier revision's `StreamedBody` carrying a
`prehash: [u8; 64]` alongside sha256 is not implementable and is not needed: the BLAKE2b
state lives in a `StreamVerifier` held by the caller's sink, obtained fresh from
`open_sink` on each attempt. `StreamedBody` carries the byte count and sha256 alone.

That placement **strengthens** the per-attempt reset rather than weakening it. The
signature state and the file are now created by the same call at the same moment, so they
cannot get out of step; the fetcher still owns when a reset happens, which is the property
the signature was shaped to guarantee.

A test asserts the release signatures are prehashed, so a future minisign that defaults
back to the legacy form fails loudly on both sides rather than silently producing
signatures the launcher and the bootstrap shim would both reject.

The download is capped at `archive_size` from the artifact's platform entry;
`uncompressed_size` and `entry_count` bound the extraction in Step 1b §2.

### Step 1b: Extraction, sealing, atomic rename, attestation, pointer

#### 1. Archive dependency

**Files**: `cli/Cargo.toml`, `cli/launcher/Cargo.toml`, `cli/deny.toml`,
`tests/integration/deny/test_launcher_feature_graph.py`
**Changes**: Add `tar`, `flate2`, `rustix` and a CSPRNG as workspace-pinned dependencies
with justification comments.

⚠️ **Four dependencies, not two — an earlier revision named only the archive pair and
calibrated the whole size gate against it.** The tree mechanisms specified later need
syscalls and randomness that `std` does not reach: `flock` with `LOCK_SH`/`LOCK_EX|LOCK_NB`
and clearing `FD_CLOEXEC` (Step 1b §2), `openat` with `O_NOFOLLOW` per path component under
a directory fd and `fstatat` for the `(st_dev, st_ino)` recheck (§4, `acquire` step 6),
`utimensat` for the claim refresh (Step 1c), and 16 hex characters from the platform CSPRNG
for the generation suffix (`materialise` step 0). `cli/launcher/Cargo.toml:23-35` declares
no `libc`, no `rustix` and no `rand`; `rustix` is present only *transitively* via `store`,
and depending on a transitive edge is exactly the coupling the workspace's pin discipline
exists to prevent.

`rustix` is the right choice — already workspace-pinned, and `features = ["fs", "process"]`
covers `flock`, `openat`, `statat` and `utimensat` — plus `getrandom` (or
`rustix::rand`) for the suffix. All four are declared as direct launcher dependencies, and
the measured size delta, the `cli/deny.toml` edit and the four-triple feature-graph
assertion are derived against the **full** set rather than the archive pair alone. `tar` is pinned **exactly**, not caret-bound: it is pre-1.0,
and its entry classification, PAX/GNU long-name handling and symlink semantics are
precisely what the extraction allowlist sits on top of, so a patch bump could shift
the trust boundary without a pin-edit review. `cli/Cargo.toml`'s stated discipline is
to exact-pin crates whose behaviour the workspace depends on — `clap` at `:47`,
`reqwest`/`rustls`/`minisign-verify` at `:61-67`, `serde-saphyr` at `:77-79` — and to
caret-bind only those documented as behaviour-stable (`regex` at `:71-73`,
`tempfile`/`rand`/`libc`/`rustix` at `:86-91`). `tar` also gets
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

**That guard must be parametrised over the target triples, not left on the host.**
`_feature_tree()` runs a bare `cargo tree -e features -p accelerator`, resolved for the
host triple only — as the module's own docstring concedes when it says the four-triple
build is the authority for static linking. A C-backend edge introduced under a
target-specific table (`[target.'cfg(target_env = "gnu")'.dependencies]`) would not
appear in the host tree on either a macOS developer machine or the Linux CI runner, so
the guard protecting the static-musl cross-build would pass while the musl target
acquired a C dependency, deferring detection to the release lane. `_feature_tree()`
therefore takes a `--target` and the assertion runs over all four `TARGETS`.

**The binary-size budget, re-derived.** This plan previously quoted a
`cli/Cargo.toml` comment saying launcher size should be reconsidered "if it exceeds a
few hundred KB". **No such text exists.** What the file actually says, at `:183-185`,
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

So the slope is obtained directly, with the measurement method work-item:0205 owes (its
SQ-4 is still open — see the warm-path criterion):
verify two padded launchers of known differing size on the same host and take the
difference. A 1ms budget then converts to a real ceiling.

Three notes on the gate. `tar` plus `flate2`/`miniz_oxide` realistically add a few
hundred KB, so a multi-MB ceiling is a weak tripwire — the enforced assertion is on the
measured delta plus a small margin, not on the headroom.

**The gate runs in both lanes, because a release-only gate cannot fail before merge.**
The absolute per-target ceiling is checked against the cross-compiled artefacts, which
exist only after `build.cli_cross_compile` — and `main.yml` runs that in the
`prerelease` and `release` jobs, not in any job gating a PR. On its own that means the
ceiling cannot fail on the change that adds `tar` and `flate2`, nor on any later
dependency addition: it fails at release time, blocking a cut rather than a change. So a
**host-target size assertion joins the PR lane**, where the `check-cli` job already
builds the launcher, and the per-target ceiling in the release lane then catches only
genuinely cross-target-specific growth.

The absolute per-target numbers are recorded beside the other pins in
`tasks/shared/paths.py` as figures with their derivation, and the *delta* is the
enforced tripwire — an absolute ceiling drifts, and once unrelated launcher growth trips
it and it is bumped reflexively it stops encoding the 1ms budget it came from.

The backend's other direction is less consequential than an earlier draft assumed.
`miniz_oxide` inflates materially slower than a zlib-ng build, but ~294MB through it is
on the order of one to two seconds, against a ~120MB download and ~294MB of filesystem
writes that plausibly dominate. So the recorded figure is **end-to-end materialisation
excluding download** — verify, inflate, write, seal — with the inflate term reported as
a share of it, and the ceiling is set on that total rather than on inflate in isolation.
If inflate turns out to be under ~20% of the total on the reference host, the backend
question is closed and the escalation is dropped rather than carried forward; if
`rust_backend` genuinely misses the ceiling, the resolution is a faster pure-Rust
backend (`zlib-rs`, if it can be shown to need no C toolchain), never a `*-sys` crate.

#### 2. Tree materialisation

**Files**: `cli/launcher/src/launch/outbound/resolve/tree/` (new — module set in §3)
**Changes**:

Layout — a dedicated subdirectory so `cache::find`'s prefix scan (`cache.rs:51-73`)
never sees a tree, content-addressed so an unchanged artifact is one tree however many
plugin versions want it, per-platform so a shared cache root cannot mix incompatible
trees, and generation-suffixed so a rename target is always fresh:

```
<cache_root>/trees/<gen-name>/               the sealed tree (contains .files at its root)
<cache_root>/trees/<gen-name>.sealed         the attestation
<cache_root>/trees/<gen-name>.sealed.sig     its detached release-key signature
<cache_root>/trees/<gen-name>.lease          the in-use lease
<cache_root>/trees/<name>-<platform>-<sha256>.ref      the pointer
<cache_root>/trees/claims/<sha256>.<launcher-id>       one install's retention claim

  where <gen-name> = <name>-<platform>-<sha256>-<layout>-<gen>
```

The **pointer is keyed by digest, not by release version**, so an unchanged pin resolves
across plugin upgrades with no manifest load and nothing accumulates per release. The
**table lives inside the sealed tree** rather than beside it, so the signed archive digest
covers it.

All names are ASCII — `cache.rs:56` aborts the *entire* scan on one non-UTF-8 entry,
so a stray name here would turn every single-file resolution into a miss — and none is
named `*.minisig` (hence `.sealed.sig`, which also keeps the tree's signature visibly
distinct from the single-file convention).

The **attestation** is small and fixed-size and is the only sidecar the hit path opens.
The **table** carries one `(path, mode, size, sha256)` row per entry, or a link target
for a symlink; it is produced by assembly, shipped inside the archive, and read only by
`verify` and `repair`, so the hit path's cost never scales with the driver tree's ~490
files. The **pointer** names a generation directory rather than a digest, which is what
lets a repair swap one generation for another atomically while the digest keying stays
stable.

**The lease is a sidecar beside the generation, not a file inside it**, per ADR-0061.
Inside, the `0555`/`0444` seal would make it read-only for the very dispatches that must
open it for writing to take a lock, and it would be an entry absent from the `.files`
table — so `verify` would report every healthy tree as carrying an unexpected extra
entry and `repair` would re-materialise trees that are fine, turning recovery into a
loop. As a sidecar it is outside the sealed directory, outside the table, and outside
`verify`'s walk, which covers only the generation directory itself.

The attestation and the pointer each carry a `format_version`, and the tree directory
name carries a layout version alongside the generation. Extraction and sealing policy
— the entry-type allowlist, mode masking, the `0444`/`0555` seal, the table's own
shape — is launcher-version-specific and is *not* covered by the archive digest, yet
content addressing means a newer launcher routinely adopts an older launcher's tree
from a shared cache root. Without a layout version a policy fix would be silently
inherited rather than applied, and `verify` would pass because it checks against the
older table. The same "unknown additive fields ignored, higher version refused"
discipline `manifest.rs:1-3` documents applies.

**The version is enforced in both consumers, or it is decoration.** It is part of the
directory-name grammar `locate` validates (below), so a generation whose layout the
running launcher does not recognise is a miss rather than an adoption; and the reuse
scan compares it too, so a matching digest at an older layout is re-materialised rather
than adopted. Criteria cover both directions plus the additive case, mirroring the
manifest's own `"future_field": 42` test (`manifest.rs:223-231`).

The generation is the load-bearing addition. Because every materialisation picks a
fresh one, `rename(2)` never lands on an existing target — so there is no
already-present branch to get right, no need to distinguish a concurrent winner from a
crash leftover at rename time, and a repair can build a complete replacement beside a
tree a live daemon is still reading.

**The ownership boundary is `trees/`, which the launcher creates itself `0700`** — not
the inherited cache root. `trees/`, every generation directory and every sidecar must be
owned by the effective uid and be neither group- nor world-writable, and anything
failing that is treated as absent rather than trusted. This is a check the existing
resolver does not make: `cache_root.rs` performs **no ownership check, no directory-mode
check and no symlink guard** — its only permission signal is
`probe_writable_and_executable` (`:122-142`) succeeding, and the single-file path
tolerates that because it re-verifies on every hit. Trees do not, and ADR-0060's threat
model assumes the cache lives under the user's own home directory while
`ACCELERATOR_CACHE_DIR` — which this plan actively recommends — can break that
assumption.

Rooting it at `trees/` rather than at the cache root is what keeps the check enforceable
without excluding legitimate hosts. The cache root's mode is inherited from whoever
created it and is umask-dependent: RHEL/Fedora-family hosts default to `umask 002` with
user-private groups, so a hand-created `ACCELERATOR_CACHE_DIR` is `0775`; Docker Desktop
and devcontainer bind mounts present host files under a mapped uid; NFS-squashed homes
and shared CI caches likewise. Refusing on the inherited root would make every tree
resolution a permanent miss on those hosts — re-materialising ~294MB per attempt or
downgrading with `cache-unwritable` — on exactly the relocation the plan recommends.
`trees/` is ours: the launcher creates it `0700` and `chmod`s it into compliance on
every materialisation, so the strict check is one it can always satisfy. Group-writability
on an ancestor is refused only when the owning group has members other than the effective
user, and every refusal names the exact `chmod`/`chown` remediation so the condition is
fixable rather than fatal.

🔒 **The checks use `symlink_metadata`, never `stat`.** `stat` follows symlinks, so a
symlink placed at a generation path and pointing at any user-owned, non-group-writable
directory — a freshly cloned untrusted repository checkout, say — satisfies an
ownership-and-mode test perfectly. Generation paths are opened with `O_NOFOLLOW` under a
directory fd for `trees/`, a symlink at that position is refused outright rather than
resolved, and the **pointer file's own uid and mode are checked** before its contents
are used as a path. On the hit path this is a handful of extra `lstat`s, which the
0.17% attestation accounting below shows is affordable; the resulting count is stated in
the criteria so the work-item:0189 probe expectations stay derived rather than
discovered.

**The attestation is signed**, settled by work-item:0214 SQ-2. Without it the checks
below are all *local and self-referential* — an attestation whose digest matches the
digest in its own directory name proves nothing about provenance. That is the hit path's
only cryptographic anchor, and two things about it have to be right.

**It is a producer-side artifact, not the manifest's archive signature reused.** The
manifest's inline signature is over the archive *file's bytes* — `tasks/signing.py:24-43`
runs `minisign -S -m <file>` and `tasks/manifest.py:81-108` slurps the resulting
`.minisig` verbatim — and the launcher deletes the archive after extraction, so there is
nothing left on disk for that signature to verify against. A signature the consumer
cannot check is not a control. Phase 2 §5 therefore emits, signs, uploads and re-verifies
a small per-artifact-per-platform attestation document alongside each archive, and the
launcher stores the document and its detached signature as `.sealed` and `.sealed.sig`.

**It binds artifact identity and content — deliberately not the plugin release version.**
Artifact identity and platform would otherwise live only in the attestation body and the
pointer filename, both unsigned local state, so any process able to write `trees/` could
repoint a `.ref` at another artifact's or another platform's generation whose signature
is entirely valid. The signed document is therefore:

```json
{
  "attestation_format_version": 1,
  "artifact": "browser",
  "platform": "linux-x64",
  "archive_sha256": "<64 hex>",
  "uncompressed_size": 185790464,
  "entry_count": 14,
  "table_sha256": "<64 hex>"
}
```

**Two fields an earlier revision put here are deliberately absent**, because each
belonged to a party that does not know its value at signing time:

- **`release_version`.** The plugin release version is not knowable in `assemble-runtime`,
  which runs upstream of `version.bump`; one archive set serves both the stable and pre.0
  cuts, so it would need two different values; and binding it would make cross-version
  adoption impossible, contradicting the criterion that two releases sharing a digest use
  one generation and fetch nothing. Rollback is defended below by a mechanism that does
  not have those problems.
- **`layout_version`.** Extraction and sealing policy is *launcher*-owned, so a signed
  copy of it can never be rewritten by the launcher that owns it: a policy bump would
  miss, re-materialise, download the same producer document still carrying the old value,
  and miss again — an unbreakable loop. The layout version lives only in the
  directory-name grammar and the reuse scan, where a mismatch means "re-materialise under
  my policy" rather than "refuse".
⚠️ **`table_sha256` is present, and an intermediate revision was wrong to remove it.**
That revision moved the `.files` table inside the archive and dropped the field, reasoning
that "the archive signature covers it". The archive signature does cover it — but only
while the archive exists, and `materialise` step 5 discards the archive after verifying
it. From then on the table would be an ordinary `0444` file inside a tree the owning uid
can `chmod`, with no digest recorded anywhere — so `cache verify`, the *only* mechanism
that ever hashes tree contents, would be walking against an oracle any local process could
rewrite to match a substituted member. That is the same "a signature the consumer cannot
check is not a control" error this section identifies two paragraphs above, made a second
time.

The cross-language objection that motivated the removal does not actually arise: the
producer hashes the table **file as it places it in the archive**, and the consumer hashes
**the same bytes as they are extracted**. That is one byte stream both sides already
agree on, so no agreement about the table's internal shape is needed.

**Rollback is prevented by a compiled-in expected digest, not by a signed version.** The
launcher embeds, at build time, the `(artifact, platform) → archive_sha256` map from the
same reviewed `ASSEMBLED_SHA256` anchor the release job asserts against, and `acquire`
resolves **only** its own compiled-in digest. That is strictly stronger than a version
field and has none of its problems:

| Property | How it follows |
|---|---|
| Rollback refused | a superseded artifact's digest is simply not the one this launcher looks for |
| Cross-version adoption | an unchanged pin yields the same digest, so a newer launcher finds the existing generation |
| Works offline | the digest is in the binary; no manifest, no network |
| No new trust root | the launcher binary is already content-addressed and signature-verified by the bootstrap |

**The pointer is therefore keyed by digest, not by release version** —
`<name>-<platform>-<sha256>.ref` — which also removes the pointer-accumulation problem
that made `prune` unable to reclaim anything, and removes the manifest load that an
earlier revision needed before the reuse scan could discover an already-present tree.

`ASSEMBLED_SHA256` now has three consumers: the release job's pre-signing gate, the
launcher's compiled-in map, and the attestation's content. It therefore moves from
`tasks/vendor/pins.py` into a language-neutral `pins.toml` that both `tasks/` and a
`cli/launcher` build step read, pinned by a drift test in the same shape as the
`TREE_ARTIFACTS` mirror.

**The `.files` table ships inside the archive** — written by assembly as the archive's
**first member** — and its digest is carried in the signed attestation. Both halves are
load-bearing and they do different jobs:

- **Inside the archive, first**, so extraction can verify each member's sha256 against its
  row *as the member is written*, in one pass over ~294MB. First rather than merely
  present, because a `tar.gz` is a stream: a table sorting last would force either
  buffering every digest or a second inflate-and-read. Assembly asserts the position and
  the launcher refuses an archive whose first member is not the table (`TableMissing`).
- **Digested in the attestation**, so the table stays anchored *after* the archive is
  discarded. `verify` checks `table_sha256` before trusting a single row, which is what
  makes every other `cache verify` assertion non-vacuous.

Modes agree because §8's deterministic assembly already masks to the same `0755`/`0644`
the launcher enforces, and the seal is a deterministic function of the executable bit, so
`verify` computes the expected sealed mode from the recorded one. The table is excluded
from `verify`'s expected-entry set — it is an archive member with no row describing
itself, and treating it as unexpected would reproduce the extra-entry repair loop the
lease-sidecar placement was chosen to avoid.

So `acquire` checks the document's `artifact` and `platform` against what it was asked
for and against `HOST_PLATFORM`, `archive_sha256` against both the directory name **and**
the compiled-in expected digest, and `attestation_format_version` for equality — treating
any mismatch as a miss.

⏱️ Measured cost is 51.7µs median cold-process (43.5µs warm in-loop, p99 58.5µs) for one
Ed25519 verify over a 244-byte attestation in the shipped release profile: **0.17%** of
work-item:0186's 29.92ms warm bootstrap, or 0.35% for both trees. The added fields do not
move that — the document stays a few hundred bytes.

The `0444`/`0555` seal is **not** an additional discriminator, and must not be described
as one. `tar` and `unzip` both preserve read-only modes exactly, and these artifacts are
`tar.gz`, so `tar xzf` into the cache root reproduces the seal perfectly; only a git
checkout cannot, because git records `100644` for a `0444` file. The seal check is
retained because the `stat` in step 3 already happens and it therefore costs no extra
syscall, but it detects inconsistency rather than establishing trust.

**`acquire`** (the hit path, on every dispatch that will use a tree):

1. `lstat` the pointer `<name>-<platform>-<D>.ref`, where `D` is the launcher's
   **compiled-in expected digest** for this `(artifact, platform)` — so a superseded
   artifact's pointer is never even named — and check its own uid and mode before reading
   it. Absent, a symlink, wrongly owned, group/world-writable, or unparseable → miss.
2. Reject its contents unless they match
   `<name>-<platform>-<64 lowercase hex>-<layout>-<gen>` exactly, with `<name>` equal to
   the artifact being resolved, `<platform>` equal to `HOST_PLATFORM`, the hex equal to
   `D`, and `<layout>` equal to the running launcher's layout version — and unless the
   result is a direct child of `trees/`. The pointer is unsigned local state whose
   contents become a path, so it is validated before it is joined.
3. Open the generation directory with `O_NOFOLLOW` under a directory fd for `trees/`,
   and `symlink_metadata` it: present, a real directory rather than a symlink, correctly
   owned, not group/world-writable. Otherwise miss — a tree removed by a partial
   `rm -rf` or an interrupted prune leaves its tiny sidecars behind, and returning a dead
   path would surface as an opaque Node error instead of a re-materialisation.
4. Read the attestation and its detached signature, and **verify the signature under the
   embedded release key**. A signature that does not verify is a miss.
5. Check every attestation field against what is being resolved: `artifact`, `platform`,
   `attestation_format_version`, and `archive_sha256` against both the digest in the
   directory name and the compiled-in expected digest `D`. Any mismatch is a miss.
   `table_sha256` is **not** checked here — it is read by `verify` and `repair`, which is
   what keeps the hit path's cost independent of the driver tree's ~490 files.
6. Open `<gen-name>.lease` and take `LOCK_SH`, clearing `FD_CLOEXEC` so the open file
   description survives the `exec` into the design binary and on into the detached
   daemon. **Then re-`fstatat` the generation directory and confirm the lease fd's
   `(st_dev, st_ino)` still matches its path**, treating any change as a miss and
   retrying once — otherwise a `prune` reclaiming between steps 3 and 6 hands back a path
   that no longer exists, and leaves a lock held on an unlinked inode that no later
   liveness probe can observe. Return the path together with the held lease.

Three small reads, a handful of `lstat`s, one Ed25519 verify and one `flock`. No network,
no manifest, and the table untouched.

⏱️ **That sequence, not the verify alone, is what the warm-path budget must cover.** The
measured 51.7µs is one Ed25519 verify; `acquire` additionally performs roughly a dozen
syscalls, three file opens and a lock per tree, doubled across both trees. The Phase 1
gate is an absolute 1.0ms, so the budget is stated over the **whole** `acquire` sequence
and measured as such — an earlier revision quoted the 0.17% figure for work the
measurement did not include.

**Step 6 is why this port is `AcquireSealedTree` rather than a pure `locate`.** The lease
is the whole basis of the reaper's liveness oracle, and the only process positioned to
take it is the one that is about to `exec` a consumer — so leaving it unowned would mean
it is bolted into whichever function an implementer reaches first, most likely one the
plan had declared side-effect-free. Naming the effect in the port keeps the contract
honest. The corresponding **query without the effect** — used by `cache verify`, `prune`
and diagnostics, which must not pin a generation merely by inspecting it — is steps 1-5
alone, and it is a separate operation on the same port.

`acquire` is called only on dispatches that will actually use a tree (Phase 3 §3), not
on every external dispatch, so `accelerator vcs guard` neither pays for it nor inherits
lease descriptors.

🔒 **`acquire` must not probe the cache root.** `verify_writable` is `pub(super)` with a
thread-local attempt counter as its first statement (`cache_root.rs:101-113`), and
`resolution.rs:590-654` pins exact per-dispatch probe counts with an explicit
anti-memoisation test. `acquire` writes nothing outside the lease's lock state, so it
calls `candidate` (`:56-72`, selection only, no filesystem access) and never
`verify_writable`. A criterion asserts the probe count is unchanged across a dispatch
that resolves a tree.

⚠️ **The hit path must be bounded, because a cache root can stop responding.** Every
read and `lstat` above runs against a path the user may have relocated with
`ACCELERATOR_CACHE_DIR`, and a `stat` against a hard-mounted NFS or otherwise
unresponsive volume blocks uninterruptibly in the kernel — no Rust-side check can
recover it. Confining `acquire` to tree-consuming dispatches (Phase 3 §3) is what keeps
that from wedging every Claude Code tool call through the `vcs guard` PreToolUse hook.
Within design, any I/O error is treated as a miss rather than propagated, and
`ACCELERATOR_CACHE_DIR` is documented as requiring a local filesystem.

**`materialise`** (the cold path — reached only from `cache ensure` and `repair`),
under the per-`(name, platform)` single-flight lock:

0. Pick the generation. The suffix is **16 hex characters from the platform CSPRNG**,
   not a pid, a timestamp or a counter — the "fresh by construction" property that
   removes the rename-collision branch at step 10 is load-bearing, and this repository
   has already been bitten by pid reuse in cache paths (`cli` test fixtures, fixed with
   self-cleaning `TempDir`s). Step 10 still carries an explicit branch for a rename that
   unexpectedly finds an existing target, treated as an internal error rather than
   assumed impossible.
1. **Reuse scan, before any network access**: any
   `trees/<name>-<platform>-D-<layout>-*` at the **running launcher's layout version**
   and the compiled-in expected digest `D`, whose attestation signature verifies, whose
   fields match, and whose directory passes the step-3 checks → publish the pointer at it
   and return, **with no download and no manifest load**. Ordering this first is what
   makes an unchanged artifact across two plugin versions a genuine hit rather than a
   refetch, and — because `D` comes from the binary rather than the network — what keeps
   that hit working offline. A matching digest at a *different* layout version is not a
   candidate: adopting it would silently inherit the extraction and sealing policy the
   version exists to distinguish.
2. Load the manifest, which supplies `archive_size` and the asset URLs. Its
   `sha256` **must equal** the compiled-in `D`; a manifest naming a different digest for
   this artifact means the launcher and the release disagree, which is a refusal rather
   than an instruction to fetch something else.
3. Free-space precheck against `archive_size + uncompressed_size` plus a margin, summed
   over every tree about to be materialised. A shortfall emits `disk-floor-not-met`
   before a single byte is fetched.
4. Stream the archive to `trees/.tmp-<name>-<platform>-<D>.archive` via `get_streaming`,
   under Step 1a's deadline, total bound and idle bound. **The temp archive is named
   by artifact, platform and digest — not by generation** — for two reasons: it is what
   makes the `Range` resume in the next paragraph reachable at all (a generation-keyed
   name would be unique per attempt, so a later `ensure` could never find the partial),
   and it is what lets the reaper derive which single-flight lock guards a residue.
5. Fetch the attestation document and its signature (two small assets), verify the
   signature under the embedded release key, and check its fields against the host and
   against `D`. Then verify the archive: its streamed sha256 against `archive_sha256`,
   and its prehashed minisign signature. On any failure, remove the temp archive and
   return the cause — nothing has been extracted.
6. Extract into `trees/.tmp-<gen>/` under the entry rules in §4 below, **computing each
   entry's sha256 inline as it is written** and checking it against the corresponding
   row of the `.files` table the archive carries, so verification costs no second pass
   over ~294MB and a tampered member is caught during extraction rather than afterwards.
7. Seal bottom-up: `0444` for files, `0555` for files the archive marks executable,
   directories left owner-writable. Symlinks are walked with `symlink_metadata` and
   their permissions left alone — `set_permissions` follows a link and would re-mode
   the target.
8. Write `.tmp-<gen>.sealed` and `.tmp-<gen>.sealed.sig` from the verified document and
   signature — they are the release's bytes, not locally synthesised. The table needs no
   write: it arrived inside the archive and is already inside the sealed tree.
9. Create `<gen-name>.lease` and take `LOCK_SH` on it **before** the rename, holding it
   until after step 10. Without this the generation spends the 9→10 window looking
   exactly like crash residue to a concurrent `prune`, which would delete ~294MB of
   freshly verified work — or, worse, unlink it under a pointer published a moment later.
10. `rename(2)` the temp directory into place, then the sidecars. Fresh by construction,
    so no collision is expected; an existing target is an internal error, not a merge.
11. Publish the pointer atomically, last. Until then the generation is invisible to
    `acquire`, so a crash at any earlier step leaves only garbage rather than a
    half-trusted tree.

`materialise` **does** probe the cache root, once — and it does so at the very top, before
`trees/` is created and before the single-flight lock file is opened, not "before step 4"
as an earlier revision said. Both of those are themselves writes to the cache root, so a
probe placed later would let an unwritable or full root surface as an opaque lock-file
creation error rather than as the `cache-unwritable`/`disk-floor-not-met` downgrade the
probe exists to produce, and the free-space precheck would never run. That probe is
accounted for in the 0189 counter's expectations rather than left to surface as a red
test: the criteria below state the expected count explicitly, as "exactly one, before any
cache-root write".

**Single-flight**: an exclusive `flock` on one lock file per `(name, platform)` under
`trees/` — **not** the PID-owner staleness discipline `bin/accelerator` uses for its own
bootstrap lock. Using a pid gate here would contradict the lease three paragraphs below,
which exists precisely because a pid gate "is a repeat of a documented failure", and it
would put the weaker mechanism on the more critical path: a crashed winner would leave a
sentinel that every subsequent cold materialisation waits out behind a staleness
heuristic. `flock` gives the same mutual exclusion with no sentinel protocol, no waiter
budget and no second discipline for a reader to keep straight, a crashed holder releases
with no cleanup code, and it composes with the lease rather than sitting beside it.

The loser waits on the **lock**, never on the pointer: a winner that fails writes no
pointer, so a pointer-waiter would hang forever. On acquiring the lock the loser re-runs
the `acquire` query and materialises only if still needed.

⚠️ **The waiter's bound is short, and its expiry is a distinct cause.** Deriving it from
the fetch deadline plus an extraction allowance would make it minutes — against a crawl
bounded at five minutes and 100–200 executor invocations — so on a cold cache over a
slow-but-healthy link the second invocation would time out *while the winner was
legitimately downloading*, and, through Phase 3's sticky marker, degrade the entire
remaining crawl to code-only. That is the outcome the artifacts exist to prevent,
triggered by the artifacts working correctly. So the waiter's bound is seconds, its
expiry emits `materialisation-in-progress` rather than `artifact-unavailable`, and that
cause is explicitly **not sticky**: the next invocation waits again, and the crawl
converges as soon as the winner finishes. The lock is released by a `Drop` guard on
every path.

⏱️ **The archive download is resumable, and the resumable file is not an orphan.**
Without resume, a link slow enough that one crawl's materialisation does not finish
restarts from byte zero on the next crawl and can never converge. The temp archive is
named by artifact, platform and digest (step 4), so a subsequent `ensure` for the same
digest issues a `Range` request from the bytes already on disk; the streamed digest is
recomputed over the whole file before verification, so a resumed transfer is verified
exactly as a fresh one is.

That requires a retention rule the reaper must respect, or the two mechanisms cancel
out: an age backstop sized to the fetch deadline would reclaim the partial archive before
the next crawl could resume it, and resume would never fire. So a partial archive whose
digest is the launcher's current expected `D` is **exempt from the generation reaper's
backstop**; it is reclaimed when that artifact materialises successfully, or when `D`
moves, or under a separate, much longer window. `get_streaming`'s per-attempt truncation
governs retries *within* one call; cross-process resumption passes an explicit offset and
a pre-seeded digest state, so the two are distinct paths rather than a contradiction.

Without this, two cold invocations each stream ~294MB, hash it, verify it, extract it
and seal it — ~588MB of transfer and ~1.2GB of transient disk, one copy of which is
then discarded. `cache::store` needs no such guard at ~8MB; at this size the
duplication is the dominant cost of a first run.

**The in-use signal is a shared `flock` lease**, settled by work-item:0214 SQ-3. The
lease sidecar `<gen-name>.lease` is opened and held `LOCK_SH` with `FD_CLOEXEC` cleared,
so the open file description is inherited through the `exec` into the design binary and
on into the detached daemon. The reaper and `prune` probe with `LOCK_EX | LOCK_NB`;
`EWOULDBLOCK` means a live holder. The kernel is the liveness oracle, so there is no pid,
no start time and no sentinel protocol to get wrong, and a crashed holder releases with
no cleanup code and leaves no stale state. A shared lease admits concurrent crawls and
stays held until the last holder dies, which is exactly the "any generation a live
process still holds" property `repair` needs.

**Both entry paths must take it, not just the warm one.** The inheritance chain above
exists only on the dispatch path. On the cold path the design binary invokes
`accelerator cache ensure`, a separate short-lived process whose descriptor closes when
it exits — and the `ACCELERATOR_DESIGN_BIN` dev-override path routes through `ensure`
too. So on exactly the first-run and development flows a daemon would be running against
a tree nothing holds, and a concurrent `prune` would `rm -rf` files a live Chromium is
still opening lazily (locale packs, `.pak` resources, `icudtl.dat`). `ensure` therefore
prints the lease path alongside the resolved tree path, and `accelerator-design` opens
and `LOCK_SH`es it itself before spawning the daemon, so the holder is the process that
outlives the resolution. A criterion covers `prune` sparing an `ensure`-resolved tree
while its consumer is alive.

**Where `flock` is unavailable, fall back rather than trust the probe.** `flock(2)` is
not uniform across the filesystems a relocated `ACCELERATOR_CACHE_DIR` can land on: NFS
emulates it via POSIX locks and can return `ENOLCK`, SMB and several FUSE backends make
it a no-op or fail outright, and overlayfs has its own history. A spuriously successful
`LOCK_EX | LOCK_NB` probe would reclaim a live daemon's tree — the failure generations
exist to prevent. So `ENOLCK`/`EOPNOTSUPP` from either side is treated as "liveness
unknown", which falls through to the age backstop below rather than to reclamation, and
a criterion covers a cache root on a filesystem without working `flock`.

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
the reason generations exist — or retains for ever. But the backstop applies to **every**
unreferenced generation rather than only to lease-less ones, so nothing can leak
permanently: an abandoned generation whose lease file exists but is held by nobody is
reclaimable immediately, and one whose liveness is unknown (no working `flock`) is
reclaimable once the backstop passes.

Orphan reaping: `cache.rs:130` removes a single temp file on a failed rename. Here the
residues are larger and more varied — a partial temp archive, a partial temp tree, and
a fully-materialised generation no pointer references (left by a crash between steps 10
and 11, or superseded by a repair). `reap_orphans` reclaims all three, with an age
backstop beyond the fetch-plus-extract deadline. It runs from `materialise` and from
`cache prune`, never from `acquire`.

**Reaping runs under the per-`(name, platform)` single-flight lock, held by its caller.**
Otherwise the reaper and a concurrent materialisation of a *different* artifact race
through `trees/` with different notions of what is referenced, and the 10→11 window plus
the reuse scan's validate-then-publish window both become removable. The lease closes
those windows for the process holding it; the lock is what keeps two reclaimers and a
materialiser from disagreeing about the same directory.

Two mechanics that make it work. `reap_orphans` takes a **lock token** proving the caller
already holds the lock rather than acquiring one itself — `materialise` acquires it once
at the top and `prune` once per artifact, and a second `flock` from a process already
holding one through a different open file description does not re-enter, so a
self-acquiring reaper would deadlock or need a hidden already-held flag. And because temp
residues are named `.tmp-<name>-<platform>-<D>.archive` and `.tmp-<name>-<platform>-<gen>/`
(step 4), the reaper can derive which lock guards each one; a whole-root `prune` takes
them in sorted `(name, platform)` order, non-blocking, skipping any artifact whose lock is
held.

#### 3. Ports, errors, and the documented divergence

**Files**: `cli/launcher/src/launch/outbound/resolve/mod.rs`,
`cli/launcher/src/launch/outbound/resolve/tree/` (the module set below),
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

- `AcquireSealedTree` — local only, no network. Two operations: `query` (steps 1-5,
  side-effect-free, for `verify`/`prune`/diagnostics) and `acquire` (steps 1-6, returning
  the path together with a held `LOCK_SH` lease). This is the only port the dispatch path
  may call, and the effect is in its name rather than hidden behind a "pure lookup"
  label — because the lease *must* be taken by the process about to `exec` a consumer.
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

**The split is enforced by the composition root's types, not by convention.** The plan's
own observation that no pup rule constrains `launch::outbound` cuts both ways: nothing
structurally stops a later contributor threading `MaterialiseTree` into the dispatch
path, and the probe-count criterion would not catch a materialisation that happened to
hit a warm cache in the test. So the dispatch composition path accepts only
`&impl AcquireSealedTree`, making the wrong wiring a compile error, and a pup rule is
added covering `accelerator::launch::outbound::resolve::tree` alongside the existing
`launch::core` rule (`cli/pup.ron:25-39`).

**A `Clock` port covers the three time-dependent behaviours.** The reaper's age backstop,
the single-flight waiter's bound and Phase 3's sticky-marker TTL are all decisions with
observable behaviour, and without injected time each can only be tested by sleeping or
by back-dating mtimes — in a repository with documented CI flake history around
lock timing. The pattern already exists (`cli/design/src/executor/ports.rs:16-18` with
`MonotonicClock` in `design-adapters`, and `corpus::metadata::Clock`), so this reuses it
rather than inventing one.

**Tree errors are a nested `ResolutionError::Tree(TreeError)`**, not five more variants
flattened into the existing fifteen (`core.rs:38-91`). Flattening would take a single
enum spanning two unrelated resolution paths to twenty variants, forcing every consumer
to reason about variants that cannot arise on its path; nesting keeps the tree taxonomy
cohesive and gives it one place to be classified.

That classification is the decision, and it is taken here rather than left to the
implementer. `From<ResolutionError> for kernel::Error` (`core.rs:167-193`) maps exactly
five integrity-class variants to `Refusal` and everything else to `Failed`, and
`swallow_under_fail_safe` (`core.rs:219-224`) swallows only `Failed` — so the mapping
silently decides whether a crawl degrades or hard-fails under `--fail-safe`:

| `TreeError` variant | Class | Why |
|---|---|---|
| `Attestation` | `Refusal` | a signature or field mismatch is evidence of tampering |
| `UnexpectedDigest` | `Refusal` | the manifest names a digest the launcher does not expect — the two disagree about what this release ships |
| `PathEscape` | `Refusal` | an archive attempting to write outside the root is hostile |
| `Extraction` | `Refusal` | a rejected entry type, a breached size bound, or a member disagreeing with the table |
| `Seal` | `Refusal` | a tree that cannot be sealed cannot be trusted unverified later |
| `TableMissing` | `Refusal` | an archive with no `.files` table cannot be verified at all |
| `LayoutUnsupported` | `Failed` | a higher layout version than this launcher knows — re-materialisation is the answer |
| `Pointer` | `Failed` | local state, recoverable by re-materialisation |
| `Lease` | `Failed` | an environmental limitation, not an integrity signal |
| `DiskShortfall` | `Failed` | remediable by the user; degrade rather than abort a crawl |
| `MaterialisationInProgress` | `Failed` | another process is succeeding; this one should yield |

So a hostile archive hard-fails a crawl even under `--fail-safe`, while local-state
damage degrades and re-materialises — and a criterion asserts that `--fail-safe`
behaviour directly rather than leaving it to follow from the mapping.

**The remaining `ensure` causes reuse the existing enum rather than growing `TreeError`.**
Phase 3 §3 enumerates ten causes, and six of them are not tree-specific: an unreachable
host is `Fetch`, an artifact absent from the manifest is `AssetNotFound`, a mismatched
archive digest or signature is `ChecksumMismatch`/`SignatureMismatch`, an unwritable cache
root is `CacheRootUnavailable`, and an unsupported platform is decided in the design binary
before the launcher is invoked at all. Their classes are already fixed by
`core.rs:167-193`. Stating that explicitly is what stops an implementer inventing parallel
tree variants for conditions the enum already models — and it is why the table above is
eleven rows rather than the six an earlier revision carried against a ten-cause list.

**The classification is exhaustive by construction.** The two tests pinning the existing
mapping (`resolution.rs:420-447` and `:450-495`) are hand-maintained `vec![]` literals
with no link to the enum, so a new variant omitted from both compiles, passes and ships
unclassified. `TreeError` therefore carries a `const fn class(&self) -> ErrorClass` whose
`match` the compiler forces to cover every variant, and both `From` and the tests derive
from it rather than from parallel lists.

Since the pup rule pins `launch::core` to std, `kernel::Error` and self, the discrepancy
report and the attestation are plain core-owned types with serde living in the adapter.

**The adapter is a directory of named modules, not one file.** `resolve/tree.rs` owning
layout naming, pointer validation, attestation read/write, the per-entry table, streaming
download, the entry allowlist, mode masking, the seal walk, single-flight locking, the
lease and reaping is the god-module this section says it wants to avoid — and `repair`,
which needs several of these independently, would have nothing clean to reuse. So:

| Module | Responsibility |
|---|---|
| `resolve/tree/mod.rs` | `acquire`/`materialise` orchestration over the others |
| `resolve/tree/layout.rs` | name construction and grammar validation, pointer read/write |
| `resolve/tree/attestation.rs` | document parse, signature verification, field checks |
| `resolve/tree/table.rs` | the per-entry table's shape, write and read |
| `resolve/tree/download.rs` | verified streaming fetch of archive and attestation |
| `resolve/tree/extract.rs` | entry admission and writing |
| `resolve/tree/seal.rs` | the bottom-up seal walk and permission helpers |
| `resolve/tree/lease.rs` | lease acquisition and the `LOCK_EX \| LOCK_NB` liveness probe |
| `resolve/tree/reap.rs` | orphan and unreferenced-generation reclamation |

`repair` consumes `attestation`, `table`, `download`, `extract`, `seal` and `reap`
directly.

Following `cache.rs`'s convention (`:135-164`), the sealing, permission and lease helpers
carry `#[cfg(not(unix))]` no-op arms so the launcher still type-checks off Unix. That is
the settled choice rather than one of two options: both neighbouring modules (`cache.rs`,
`cache_root.rs`) keep the marker arms and document why, and a half-portable resolver is
worse than either consistent answer. Windows is outside ADR-0062's matrix, so this is
about keeping the neighbours' discipline rather than about supporting Windows.

#### 4. Extraction entry rules

**Files**: `cli/launcher/src/launch/core.rs`,
`cli/launcher/src/launch/outbound/resolve/tree/extract.rs`,
`tests/fixtures/adversarial-archives/` (new, shared with Phase 2)
**Changes**: An allowlist, not a denylist. Regular files and directories are admitted;
symlinks are admitted only if they resolve inside the root. Everything else — hardlinks,
FIFOs, devices, sockets, absolute paths, any component equal to `..` — is rejected, and
rejection fails the whole materialisation rather than skipping the entry.

**The classification is a pure function in `launch::core`**, over a described entry
(`kind`, `path`, `mode`, `link_target`, and the running byte and entry totals) returning
admit or reject-with-reason. This is the highest-consequence logic in Phase 1 — the trust
boundary between an untrusted archive and the filesystem — and leaving it inside the
adapter means its rejection matrix can only be exercised by constructing tarballs. As a
core function it is a table-driven unit test, the adapter reduces to read-entry,
classify, write, and it falls under the `launch::core` pup rule rather than outside every
rule.

**The containment mechanism is `openat` with `O_NOFOLLOW` per path component**, under a
directory fd for the temp root, not a lexical prefix check and not a `canonicalize` after
the fact. A lexical check is defeated by a symlink-then-traverse chain, and a
check-then-create pair is a TOCTOU window; resolving each component through a fd chain
that refuses to traverse a symlink closes both, which is what "resolved against the real
root as it is created" has to mean in practice.

Three further rules that tar CVE history turns on, stated rather than left to
implementer discretion:

- **Archive-supplied ownership and timestamps are discarded.** uid, gid, mtime and
  extended attributes are never applied. (`tar`'s `xattr` feature is disabled in §1 for
  dependency-graph reasons; this is the integrity reason, and it is independent.)
- **Duplicate paths and long-name override records are rejected.** A second entry for an
  already-written path, and PAX/GNU long-name or long-link records that disagree with the
  header they extend, are both rejected rather than last-one-wins.
- **Entry names have a charset and length policy.** Non-UTF-8 names, control characters,
  Windows-style drive or UNC prefixes, and names beyond a stated component and total
  length are rejected — the same ASCII discipline the cache directory names follow, for
  the same `cache.rs:56` reason.

Modes are masked to `0755`/`0644` before the seal, so setuid, setgid and sticky bits
cannot survive extraction. The running totals of uncompressed bytes and entry count are
checked against `uncompressed_size` and `entry_count` from the signed attestation as
extraction proceeds, so a decompression bomb aborts partway rather than after filling the
disk.

**One fixture corpus serves both extractors.** Phase 2 §3 applies the same rules CI-side
in Python, and two independent implementations of one security-critical allowlist drift —
a rule tightened on one side silently would not apply on the other. So the adversarial
archives (a `../` entry, an escaping symlink, an escaping hardlink, an absolute path, a
symlink-then-traverse chain, a FIFO, a setuid member, a duplicate path, a PAX long-name
override, an over-size tree, an over-count tree) are committed once and iterated by both
the Rust suite and the Python suite, so adding a case exercises both.

### Step 1c: `accelerator cache` built-in

#### 1. Command surface

**Files**: `cli/launcher/src/launch/inbound/cli.rs`, `cli/launcher/src/launch/core.rs`,
`cli/launcher/src/main.rs`, `tasks/shared/dispatch_coherence.py`,
`tests/unit/tasks/shared/test_dispatch_coherence.py`
**Changes**:

```
accelerator cache verify [<name>]   walk sealed trees against their file tables
accelerator cache repair [<name>]   re-materialise any tree that fails verify
accelerator cache ensure <name>...  materialise trees if not already, concurrently
accelerator cache prune             reclaim unreferenced generations and orphans
```

`verify` walks each pointed-at generation against its `.files` table using
`symlink_metadata`, and **hashes every regular file**. There is deliberately no
stat-and-escalate shortcut: a substitution that preserves size and mode is exactly the
case the table exists to catch, and an escalation predicate keyed on size or mode never
fires for it — the digests would never be read on the only path that reads them.
⏱️ The cost is **~118ms for the set this plan actually ships** — ADR-0060's
`chromium-headless-shell` row (~71ms) plus the driver bundle (~47ms). It is *not* the
~120ms an earlier draft quoted: that is ADR-0060's *full Chromium* row (297MB across 327
files), which this plan explicitly does not ship, and the coincidence of the two totals
is not a derivation. Both figures assume an Apple Silicon host with hardware-accelerated
sha256 at ~2.5GB/s and a warm page cache; on an x86_64 host without SHA extensions, or
reading ~294MB cold off disk, `verify` is several times that. It stays affordable on a
command a user runs deliberately and never runs on the hit path — but it is also on the
**recovery** path, since `repair` verifies first and the failure envelopes name `repair`
as the remediation, so the stat pre-check short-circuits `repair`'s common case where an
entry is simply missing rather than hashing the whole set to discover it.

The stat pass otherwise survives only as a cheap pre-check for missing and unexpected
entries. `verify` reports per-entry discrepancies — missing, extra, size, mode, digest,
link target — rather than a bare pass/fail, so the output diagnoses as well as detects.
**`verify` checks the table's own digest against the attestation's `table_sha256` first**,
before trusting a single row. The table is the oracle every other check reads, so a table
edited after materialisation to match a substituted member would make every tree-side
detection vacuous — and the attestation is signed under the embedded release key, so the
oracle's trust does not originate on the user's disk even though the file does. A
`table_sha256` mismatch is itself a detection, reported as tampering rather than as a
missing entry. `verify` computes each entry's expected sealed mode from the mode the table
records, since the seal is a deterministic function of the executable bit.

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

**`repair` then reaps the superseded generation itself**, which is what makes it the
`verify → materialise → repoint → reap` composition §3 describes rather than three
quarters of it. Leaving disposal entirely to `prune` would make the criterion that
matters most here — that a repair run while a process holds files open in the old
generation does not unlink them — pass vacuously, since nothing would be attempting to
unlink anything. Reaping under the lease is what actually exercises the liveness oracle:
a superseded generation whose lease is still held is skipped and left for a later
`prune`, and one nobody holds goes immediately.

`repair --force` skips verification and re-materialises unconditionally. It is the only
recovery for a tree that is internally consistent but *wrong* — assembled for the wrong
architecture, or missing a component — which `verify` cannot detect by construction,
since such a tree matches its own table perfectly. Without it, a user following the
remediation string in a failure envelope gets a successful no-op and no diagnosis.

`ensure` is the cold-path entry point `accelerator-design` calls when the launcher
exported no path for a tree it needs. It materialises and prints the resolved tree path
**and its lease path** (so the caller can hold the lease itself — see the lease
discussion in Step 1b §2), or fails with a structured cause the caller maps to a
downgrade reason. It exists so the launcher never has to know which design subcommands
need a runtime (see Phase 3).

⏱️ **It takes one or more names and materialises them concurrently.** A first run needs
both trees, and serialising them means two full download-verify-inflate sequences back to
back, each in its own launcher process building its own `Fetcher` — a fresh rustls
provider install, a background runtime thread and a fresh TLS handshake per artifact
(`fetcher.rs:81-101`). The two are entirely independent, hold different single-flight
locks and write different temp generations, so `ensure driver browser` overlaps the
latency- and bandwidth-bound download phases and the user-visible first-run cost becomes
the max rather than the sum. The stated first-run ceiling is derived on that basis.

`prune` reclaims generations and orphan temps, and it is what bounds growth for anyone
who takes the documented `ACCELERATOR_CACHE_DIR` escape, since that location sits outside
the plugin tree and so outside the only eviction this design otherwise has: content
addressing means an unchanged artifact is reused rather than duplicated, but each pin bump
still materialises a fresh tree and nothing else would ever remove the old one.

**Digest-keyed pointers make the predicate tractable, where version-keyed ones did not.**
Because a pointer is `<name>-<platform>-<sha256>.ref`, a shared root accumulates one per
*distinct artifact version* rather than one per plugin release — so an unchanged pin adds
nothing however many plugin upgrades pass over it, and a superseded pointer is one whose
digest no installed launcher looks for.

`prune` therefore reclaims:

- Any generation no pointer names, and on which no lease is held.
- Any pointer, and the generation it names, that **no installed launcher has claimed
  recently** — where "claimed" is a fact on disk, not an inference.

⚠️ **The claim must be written down, because a compiled-in digest is private to the binary
that carries it.** An intermediate revision had `prune` reclaim "any pointer whose digest
is not the expected `D` of any launcher installed under the roots it can see" — which is
not implementable: `D` lives inside each launcher's binary, and the only ways to learn a
sibling's value are executing an unverified binary found by walking plugin roots, or
scanning it for strings. Neither is a mechanism this plan would accept anywhere else, and
the failure mode of getting it wrong is deleting a live install's ~294MB.

So each launcher **declares its claim**: on every successful `acquire` or `materialise` it
writes `trees/claims/<digest>.<launcher-id>` — an empty, `0600`, uid-checked file whose
mtime is refreshed. `prune` reads that directory and needs no knowledge of any other
install. A generation is reclaimable when no claim file for its digest has been refreshed
inside the window, `--older-than` overrides the window, and a claim file failing its
ownership check is ignored rather than trusted. There is no `--all-versions` flag: digest
keying removed the case it was invented for.

⏱️ The refresh is **best-effort and never fails a resolution.** `bin/accelerator:13-15`
documents that a populated cache directory may be read-only on a warm start, and the hit
path must keep working there — so `EROFS`/`EACCES`/`EPERM` are ignored, and the write is
skipped entirely when the recorded mtime is newer than a fraction of the prune window,
which turns 200-400 writes per crawl into at most one. The `utimensat` is counted in the
hit-path syscall figures rather than left out of them.

Criteria cover a root holding two installed versions' claims where only one is running
(both spared), a claim nobody has refreshed inside the window (reclaimed), a sibling
install's tree used yesterday (spared), a read-only cache root still resolving on a hit,
and a simulated pin bump leaving total footprint bounded rather than growing by ~294MB per
bump for ever.

`prune` also reports total tree footprint across sibling plugin-version roots, so the
default root's growth is discoverable even though ADR-0063 delegates its eviction to
Claude Code's orphan sweep — a user tracking prereleases otherwise accumulates hundreds of
megabytes per upgrade with no signal until it surfaces as a `disk-floor-not-met` downgrade
in unrelated work.

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

- [x] Failing test first: *a tarball entry named `../escape` is rejected and nothing is
      written* — red before `resolve/tree/extract.rs` exists, and it is the entry
      classification pure function in `launch::core`, so it needs no archive plumbing to
      go green
- [x] `skip_if_no_minisign!` (`resolution.rs:255-265`) **fails closed under `CI`** rather
      than returning `Ok(())` with an `eprintln!`. The extraction, sealing, pointer and
      reaper tests exercise the tree modules directly with no signing step and do not
      take the guard at all; but the signature, attestation and end-to-end cases do, and
      those are precisely the tests covering the hit path's only cryptographic anchor —
      the ones that must never be able to vanish silently. `minisign` is pinned in
      `mise.toml:35`, so a hard failure costs CI nothing and closes the local false-green
- [x] A crash injected at each of steps 4 through 11 is driven by a **named test seam** —
      an injectable `after_step` hook on the materialisation adapter, test-only and *not*
      a cargo feature (`tasks/lint/cli.py:7` and `tasks/test/cli.py:13` pass
      `--all-features`, so a feature would be on during every check) — rather than by
      hand-constructing the seven post-crash on-disk states, which would test the reaper
      and not the publish sequencing
- [x] The concurrency and lease tests synchronise on a rendezvous file or pipe handshake,
      never a sleep; and the probe assertions account for `PROBE_ATTEMPTS` being a
      `thread_local!` (`cache_root.rs:74-75`), so a `probes_during` expectation is blind
      to any thread the test spawns
- [ ] A corrupt archive is rejected **before** anything is extracted — the test asserts
      the trees directory is empty after the failure
- [x] 🔒 An attestation whose signature does not verify under the embedded key is a
      **miss**, not a hit; and one signed by a *different, untrusted* keypair is also a
      miss — the `MockServer` harness already generates two keypairs for exactly this
      shape. Deleting the verification call from `acquire` must turn at least one test red
- [x] 🔒 An attestation whose `artifact`, `platform`, `attestation_format_version` or
      `archive_sha256` differs from what is being resolved is a miss, asserted per field
- [x] 🔒 A generation whose attestation is entirely valid but whose digest is **not** the
      launcher's compiled-in expected digest is refused — the rollback defence, asserted
      by pointing a `.ref` at a superseded artifact version's intact generation
- [ ] Two launchers whose compiled-in expected digest is the **same** share one generation
      directory: the second resolves it with **zero** HTTP requests **and no manifest
      load**, including with the release host unreachable — the cross-version adoption the
      digest-keyed pointer exists for
- [ ] A manifest naming a different `sha256` for an artifact than the launcher's
      compiled-in digest is a refusal, not an instruction to fetch it
- [x] The compiled-in digest map, `TREE_ARTIFACTS` and the shared `pins` data file agree,
      pinned by one drift test
- [ ] 🔒 A `.files` table rewritten to match a substituted file is still detected, because
      its digest no longer matches the attestation's signed `table_sha256` — the case that
      makes every other `cache verify` assertion non-vacuous. Asserted **after** the
      archive has been discarded, since that is when the table has no other anchor
- [ ] An archive whose `.files` table disagrees with a member's actual content is rejected
      **during extraction**, before the tree is sealed or the pointer published
- [x] An archive whose first member is not the `.files` table is refused (`TableMissing`),
      so single-pass verification cannot silently degrade to a second inflate
- [x] `cache verify` does not report the `.files` table itself as an unexpected entry
- [ ] A generation directory replaced by a **symlink** pointing at an otherwise-compliant
      user-owned directory is refused rather than resolved, and the pointer file's own
      ownership and mode are checked before its contents are used as a path
- [x] A generation at an unrecognised **higher** layout version is refused rather than
      parsed; one at a **lower** layout version is re-materialised rather than adopted by
      the reuse scan; and an attestation carrying an unknown additive field still parses,
      mirroring `manifest.rs:223-231`
- [x] A tarball is rejected for each of: a `../` entry, an escaping symlink, a hardlink
      whose target escapes, an absolute path, a symlink-then-traverse chain, a FIFO or
      device entry, a tree exceeding `uncompressed_size`, and an entry count over
      `entry_count`
- [x] A setuid archive member is materialised without its setuid bit, and an archive
      member marked executable keeps only its executable bit
- [x] A streaming fetch whose first attempt fails after N bytes succeeds on retry,
      rather than producing a concatenated archive that can never verify
- [ ] A stalled transfer fails fast rather than waiting out the full deadline three
      times, driven by `Route::Stall` (`tests/common/mod.rs:30-32`) which stops sending
      rather than trickling
- [x] The retry loop's **total** wall clock is bounded across all three attempts, not
      only per attempt, and the reported failure names that bound
- [x] An interrupted download resumes: a second `ensure` for the same digest issues a
      `Range` request rather than restarting from byte zero, and the resumed archive
      verifies exactly as a fresh one does
- [ ] ⏱️ Peak RSS during a cold materialisation of the larger tree stays within a stated
      ceiling, asserted under a container memory limit set to it — the check that keeps
      the prehashed-signature decision from silently regressing into a full buffer
- [ ] Release signatures are **prehashed** for both artifact classes, so a future
      minisign defaulting back to the legacy form fails loudly rather than silently
      producing signatures the launcher and the bootstrap shim would both reject
- [x] A second resolution of the same tree issues **zero** HTTP requests, asserted
      against the `MockServer`'s request count
- [ ] A resolution with the release host unreachable still succeeds on a populated
      cache
- [x] Two concurrent cold resolutions of the same tree issue **exactly one** archive
      fetch, and neither observes a partial tree
- [x] A winner that fails mid-materialisation releases the lock, and the loser makes
      progress rather than waiting on a pointer that will never appear
- [x] ⚠️ A loser whose wait bound expires **while the winner goes on to succeed** emits
      `materialisation-in-progress`, and that cause does **not** suppress subsequent
      attempts — so a slow-but-healthy first fetch cannot degrade the rest of a crawl to
      code-only
- [x] A crash at each of steps 4 through 11 leaves only reclaimable garbage: no pointer
      is published, `acquire` reports a miss, and the reaper removes the residue
- [ ] A `cache prune` racing a `materialise` never removes the generation being
      published, including in the window between the rename and the pointer write, and
      never removes the generation an in-flight reuse scan is about to point at
- [x] A pointer naming a directory that does not exist, is not a direct child of
      `trees/`, does not match the full
      `<name>-<platform>-<64 hex>-<layout>-<gen>` grammar, names a different artifact or
      platform, or is not owned by the effective uid is treated as a miss rather than
      exported
- [x] A sealed tree is removable by `remove_dir_all` without an intervening chmod; an
      archive member marked executable is still executable after sealing; and a
      symlink's target is not re-moded by the seal walk
- [x] `cache verify` detects each of a deleted file, a truncated file, a **same-size
      same-mode** content substitution, a mode change, a changed symlink target, and an
      unexpected extra entry
- [ ] `cache verify` succeeds with the release host unreachable
- [x] A truncated tree and a corrupted tree are each returned to a working state by
      `accelerator cache repair`, which materialises a **new generation** and swaps the
      pointer rather than removing the old tree first
- [ ] A repair whose refetch fails leaves the previous tree in place and still
      resolvable
- [ ] A repair run while a process holds files open in the old generation does not
      unlink them, and that process can still open further files from it
- [x] `repair --force` re-materialises a tree that passes `verify`
- [x] Every `cache` verb refuses an unrecognised `<name>` without touching the
      filesystem
- [x] Two release versions naming the same digest share **one** generation directory
      and two pointers, and the second version issues **zero** archive fetches
- [ ] Two platforms sharing one cache root each resolve their own tree
- [x] A `trees/` directory that is group- or world-writable, or not owned by the
      effective uid, is refused rather than trusted — and a cache root inherited at
      `0775` under a user-private group (the RHEL/Fedora `umask 002` default) still
      resolves, since the launcher creates and `chmod`s `trees/` itself. Every refusal
      names the exact `chmod`/`chown` remediation
- [x] The reaper removes a temp archive, a temp tree, and an unreferenced generation;
      spares any generation whose `flock` lease is still held — asserted with the lease
      inherited by a detached child while every ancestor has exited — and spares nothing
      indefinitely once the age backstop passes, including for a generation whose lease
      file exists but is held by nobody
- [ ] A tree resolved through `cache ensure` (not through a dispatch) is spared by
      `prune` while its consuming process is alive, proving the lease is held by the
      process that outlives the resolution rather than by the exited `ensure` child
- [ ] On a cache root whose filesystem does not support `flock` (`ENOLCK`/`EOPNOTSUPP`),
      liveness is treated as unknown and reclamation falls through to the age backstop
      rather than proceeding on a spuriously successful probe
- [x] `cache prune` reclaims an unreferenced generation and leaves the pointed-at one
- [x] `cache prune` on a root holding two **installed** launchers' refreshed claim files
      spares both, and reclaims a generation whose claims have all gone stale — so a
      simulated pin bump leaves total footprint bounded rather than growing by ~294MB per
      bump indefinitely
- [x] ⚠️ A sibling install's tree **used yesterday** is spared, because its claim file's
      mtime is inside the window — the case where an age test on creation time would
      silently destroy another installed version's ~294MB on a shared cache root
- [x] `prune` reads only `trees/claims/`, never another launcher's binary: no sibling
      executable is spawned and no binary is scanned for constants
- [x] A claim file that is a symlink, or not owned by the effective uid, is ignored rather
      than treated as a live claim
- [ ] The claim refresh is best-effort: a **read-only** populated cache root still resolves
      a tree on a hit, and the refresh is skipped when the recorded mtime is already fresh
- [x] `--older-than` overrides the window; there is no flag that drops every pointer but
      the running launcher's
- [x] 🔒 A dispatch that resolves a tree via `acquire` leaves `probe_attempts()`
      unchanged, and a cold `materialise` adds exactly one — asserted with the
      `probes_during` harness (`resolution.rs:199-213`) so work-item:0189's
      once-per-dispatch guarantee is extended rather than broken. The hit path's exact
      read, `lstat`, verify and `flock` counts are asserted as stated numbers, so the
      added ownership and symlink checks are accounted for rather than discovered
- [x] The dispatch composition root accepts only `AcquireSealedTree`, so wiring
      `MaterialiseTree` into it is a compile error rather than a test failure
- [x] `TreeError::class()` is exhaustive by construction, and the `Refusal`/`Failed`
      tests derive from it rather than from hand-maintained lists — a new variant added
      without a classification must not compile
- [x] Under `--fail-safe`, a hostile archive (path escape, failed attestation) hard-fails
      while pointer damage degrades and re-materialises
- [x] `manifest.example.json` with an added `artifacts` key still parses, and a manifest
      *without* `artifacts` still resolves single-file binaries
- [x] `BUILTIN_SUBCOMMANDS` and the clap `Command` enum agree, with
      `test_dispatch_coherence.py:606-611` and `:628-635` updated in the same change
- [x] `mise run cli:check` exits 0
- [x] `mise run deny:check` exits 0, and `libz-sys`/`zlib-ng-sys`/`zlib-sys` are absent
      from the launcher feature graph **for every one of the four target triples**, not
      only the host — `_feature_tree()` takes a `--target` and the assertion is
      parametrised over `TARGETS`
- [ ] The launcher binary size delta is within a ceiling derived from a **measured**
      per-MB verify slope and a 1ms budget. The **delta** is the enforced assertion; it
      runs on the **host target in the PR lane** (where `check-cli` already builds the
      launcher) as well as per-target in the release lane, so the regression fails on the
      change that causes it rather than on the next release cut
- [ ] ⚠️ **BLOCKED — the warm-path gate cannot be instantiated from anything this plan or
      its references currently contain.** A warm executor invocation must show no
      regression against a pre-Phase-1 launcher on the same host, and the gate must be a
      threshold rather than a recorded observation. But the statistical design does not
      exist yet: work-item 0205's **SQ-4, "Close the statistical design", still lists the
      gate statistic, its interval flavour, the resample count and `n` as open questions**,
      and the only interleaved measurement committed anywhere in `meta/` is
      `meta/migrations/0196-warm-path-measurement.md` (50 samples, darwin-arm64, ratio
      0.406). An earlier revision of this criterion asserted a specific instantiation —
      n = 300, a Hodges–Lehmann shift estimate, an upper 95% bound, and a "recorded 1.28
      ratio" — and attributed it to 0205. **None of that appears in 0205 or anywhere else
      in the repository; it was not sourced.** It has been removed rather than corrected,
      because there is no correct version of it to write here.

      So this criterion is an explicit **dependency, not a specification**: 0205 must close
      SQ-4 before Phase 1 can state its gate, and Phase 1 must then restate it
      self-containedly with a derivation a reviewer can check against 0205's answer. What
      the plan can say without inventing anything: the bound is **absolute rather than a
      ratio** (a ratio gate at this scale is the shape 0205 exists because three prose
      specifications of it failed review), the baseline binary is built at the merge base,
      and **a regression blocks Phase 1 and reopens the budget in ADR-0061 rather than
      being recorded and waved through**. Step 1b §1's size ceiling derives from the same
      budget, so both gates must be instantiated together once 0205 lands
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] End-to-end materialisation of the browser tree **excluding download** — verify,
      inflate, write, seal — completes within **20s** on the reference host, with the
      inflate term reported as a share of it. A threshold, not a recorded observation. If
      inflate is under ~20% of the total the backend question is closed and the `zlib-rs`
      escalation is dropped; if the total is missed and inflate dominates it, the
      escalation is a faster **pure-Rust** backend (`zlib-rs`, if it can be shown to need
      no C toolchain), never a `*-sys` crate
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

**Files**: `tasks/vendor/npm.py` (new), `tasks/vendor/nodejs.py` (new),
`tasks/vendor/chromium.py` (new), `tasks/vendor/gpg.py` (new), `tasks/vendor/pins.py`
(new), `tasks/__init__.py`, `keys/nodejs-release.asc` (new), `keys/npm-registry.pem`
(new), `pyproject.toml`, `mise.toml`, `RELEASING.md`
**Changes**: Three verifications, each failing the release rather than the user's run.

**One module per upstream source, not one `verify.py` holding three trust protocols.**
The npm registry signature plus SLSA attestation, the GPG-signed `SHASUMS256.txt`, and
the pinned Chromium hash share nothing but the fact that they all gate the same release;
each runs to a substantial amount of code with its own fixtures. `pins.py` is the shared
anchor data they all read. `tasks/__init__.py` gains the `vendor` collection — without it
the tasks are unreachable from `mise`, and it appears in no other Files list.
Each needs a trust anchor that does not arrive over the channel it is verifying, and
that is the part ADR-0059 leaves open: it establishes that the sha512 integrity is
fixity rather than provenance "because it comes from registry metadata fetched over
TLS", but never says where the key validating the *signature* comes from. Fetching that
key from the registry too would reproduce the same problem one level up, so both key
sets are committed.

- **`playwright-core`** — fetch from `registry.npmjs.org`, verify the registry
  signature against `keys/npm-registry.pem`, and verify the SLSA provenance
  attestation.

  **The registry signature must be bound to the bytes, or it is decorative.** npm's
  signature covers a metadata string from the packument — `<name>@<version>:<integrity>`
  — not the tarball. So verifying the signature proves only that the registry asserted an
  integrity value; the check is completed by recomputing the downloaded tarball's sha512
  and comparing it against the `integrity` value **inside the signed message**. Without
  that second step the entire npm-side guarantee collapses onto SLSA alone, defeating the
  defence-in-depth the two checks exist to provide, and a criterion covers the negative.

  The SLSA check is likewise only as strong as its predicate: `gh attestation verify`
  without `--owner`/`--repo` accepts an attestation from any builder, so the expected
  source repository, the expected workflow identity, and a subject digest bound to the
  fetched tarball are all asserted explicitly, and any mismatch fails the release. Because
  the check's runner is injected for testing, a test also **pins the exact argv** passed
  to `gh attestation verify` — otherwise dropping `--repo` would leave every SLSA
  criterion green while removing the check itself. `gh attestation verify` appears today
  only as a manual step in `RELEASING.md:271-281`, which also states plainly that "the
  launcher's runtime trust root is the signed manifest, not SLSA provenance"; this makes
  it a pipeline step without changing that.
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

  **The predicate is a pure function over status lines, separate from invoking `gpg`.**
  All the subtlety in this phase lives in that classification, and testing it through the
  subprocess would mean crafting revoked and expired keyrings and depending on a
  particular host GnuPG — the same shape as the `skip_if_no_minisign!` trap Phase 1
  rejects. So `tasks/vendor/gpg.py` exposes `classify_status_lines(lines) -> Verdict`,
  fed by a thin wrapper, and every combination is a table-driven unit test over recorded
  fixture output: `VALIDSIG` with `REVKEYSIG`, `VALIDSIG` with `EXPKEYSIG`, a subkey
  fingerprint whose primary differs from the allowlist, `NO_PUBKEY`, and the good case.
  An absent `gpg` **fails** rather than skipping.

  The digest is matched by **exact filename** against a `SHASUMS256.txt` line, never by
  searching the file for a digest — a search would accept a line describing a different
  artifact that happens to be listed in the same signed manifest.

  ⚠️ **A frozen keyring cannot observe upstream revocation, and the criterion must say
  so.** GnuPG emits `REVKEYSIG` only when the *local* keyring carries the revocation
  certificate, so a Node release key revoked upstream *after* our snapshot was taken
  yields a plain `GOODSIG` against our committed copy. The revoked-key test therefore
  verifies that a revocation **present in the committed keyring** is honoured — which is
  what it actually checks — and closing the real gap is a freshness obligation, not a
  predicate: the refresh procedure below carries a **maximum keyring age**, with a
  scheduled guard that opens an issue when it is exceeded.

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

The procedure is documentation, so it is backed by mechanical guards, because a committed
anchor is only as strong as the review that gates it and this repository has no CODEOWNERS
file — a change to `keys/**` or `tasks/vendor/pins.py` is reviewed exactly like a version
bump today.

1. **A required CI job fails on any diff to the anchor set unless a second person
   approved it.** The anchor set is `keys/**`, the shared `pins` data file, and the
   fingerprint allowlist module. "Carries an explicit trust-anchor approval" is not a
   control if the author can grant it — a label, a body string or a commit trailer are all
   self-applicable — so the job queries the PR's reviews through the GitHub API and fails
   unless there is an **approving review from a named reviewer team whose author is not the
   PR author**. That is checkable in CI, which a repository setting is not, and it is the
   property that actually matters. A CODEOWNERS entry over the same paths plus branch
   protection requiring code-owner review is the belt to that braces; CODEOWNERS alone
   enforces nothing, since without protection requiring the review the file is advisory.
2. **The gate covers the anchors and their self-checks together, because file separation
   is not the control.** An earlier revision put the expected fingerprints "in a separate
   test module with its own change gate", which is circular — the separation only helps if
   something stops one PR editing both, and that something is the approval in point 1. So
   the fingerprint module is inside the guarded set rather than outside it, and the
   build-system test that compares it against `keys/nodejs-release.asc` is honest about
   what it detects: an *inconsistent* edit, not a coordinated substitution. The coordinated
   case is what the second approver is for.
3. **Fingerprints are additionally published in `RELEASING.md`**, so a reviewer verifying a
   rotation has a value to compare against that did not arrive in the same diff — the
   nearest thing to genuinely out-of-band this repository can offer without a second
   repository.
4. **Key expiry and pin age are scheduled guards, not unit tests.** Asserting "each key is
   unexpired" in `test:unit:build-system` means that on the day the first key expires, CI
   reddens on every branch for every contributor with no code change — a staleness alarm
   implemented as a hard failure blocks unrelated work instead of notifying an owner. So
   expiry, the keyring's maximum age, and the **pinned runtime's** maximum age all live in
   one scheduled job that opens an issue. The runtime pins (`playwright-core`, Node,
   `CHROMIUM_REVISION`) are recorded in `RELEASING.md` as security-relevant dependencies
   with a named owner and a maximum age, so a browser engine shipping to every user cannot
   age indefinitely with nothing watching. Advisory-feed integration stays a follow-up; the
   stale-pin tripwire lands here, because it is cheap and the exposure is a full browser
   engine.

**The refresh triggers include our own assembly code, not just upstream movement.**
`ASSEMBLED_SHA256` (§8) depends on the compression implementation and level, the tar
writer's record choices, the normalisation code itself, and the pinned `requests`/Python
versions — so a routine dependency bump or a normalisation refactor makes every archive
mismatch a committed digest, and the plan's response to a mismatch is to fail the release.
Left unstated, an unrelated change silently makes the project unreleasable, which is the
failure mode §8 exists to remove. The procedure therefore names assembly-implementation
and build-dependency changes as digest-refresh triggers, and §8 adds a default-branch CI
check so a determinism break surfaces on the PR that causes it.

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

**Files**: `tasks/vendor/assemble.py` (new), `tasks/vendor/archive.py` (new),
`tasks/build.py`, `tasks/release.py`, `.github/workflows/main.yml`,
`tests/unit/tasks/test_workflows.py`
**Changes**: **Assembly moves out of the `release` job entirely, into an upstream job
holding no credentials.** An earlier draft made it a *step* inside `release` with
`GH_TOKEN` removed from that step's `env`; that is both weaker than it looks and
incompatible with the functional gate §8 requires.

```
jobs:
  assemble-runtime:            # permissions: {}
    - vendor.verify_upstream_inputs      # needs GH_TOKEN for gh attestation verify
    - build.assemble_tree_artifacts      # extracts; no GH_TOKEN
    - upload-artifact: dist/release/accelerator-{driver,browser}-*.tar.gz

  smoke-runtime:               # permissions: {}, matrix over the four targets
    needs: assemble-runtime
    - download-artifact: accelerator-runtime-<platform>   # this leg's ~300MB only
    - execute node --version and the headless shell; assert NOTICES/

  release:                     # unchanged permissions and step sequence
    needs: [assemble-runtime, smoke-runtime]
    - download-artifact into dist/release/    # all four targets
    - assert every archive matches pins ASSEMBLED_SHA256
    - prepare -> sign -> attest -> finalise      # still one job

  prerelease:                  # runs on every push to main — same gating
    needs: [assemble-runtime, smoke-runtime]
    - download-artifact into dist/release/
    - assert every archive matches pins ASSEMBLED_SHA256
    - prepare -> sign -> attest -> finalise
```

**`prerelease` is wired identically to `release`, not left out.** It runs on every push to
main (`main.yml:429-446`), it gains the same `ASSEMBLED_SHA256` assertion in
`prerelease_prepare`, and prereleases are the channel this plugin's users actually track —
so leaving it unwired would either fail that assertion on every cut or ship tree artifacts
no smoke matrix ever executed, on the most-consumed lane. The workflow-shape criterion
covers both publishing jobs, and §7's capacity figures account for `assemble-runtime` and
the smoke matrix running per push rather than only per release.

⚠️ **That makes the runtime lane a release-blocking dependency for unrelated fixes.** A
retired runner label, an upstream outage or an assembly regression would hold up a
security fix to something entirely unrelated — the single-point-of-failure hazard §8 works
to remove on the third-party side, reintroduced in-house. So a `skip_tree_artifacts`
workflow input omits the `artifacts` map entirely and is asserted to produce a manifest
that older launchers and the design binary both handle as "not materialised" (the design
path degrades to the code-only crawler), giving an auditable escape that cannot silently
become the default.

Three things this buys that a step boundary cannot.

**The gate becomes expressible.** §8 requires the vendored Node and headless shell to be
*executed* before publication, in a job with `permissions: {}`, because executing them is
a stronger form of handling untrusted input than extracting them and Chromium is the one
input ADR-0059 accepts on TLS trust alone. As a step inside `release`, that is a `needs:`
cycle — a step cannot wait on a job that consumes the artifacts its own job produces, and
GitHub Actions has no construct for it. Upstream, it is a plain dependency. Without this
the only reachable outcomes were dropping the strongest gate in the phase or executing
upstream binaries in the job whose later step holds `ACCELERATOR_RELEASE_SECRET_KEY`.

**The credential removal becomes real.** A step-level `env` cannot scope away job-wide
values: `id-token: write` and `attestations: write` (`main.yml:572-575`) mean
`ACTIONS_ID_TOKEN_REQUEST_URL`/`_TOKEN` and `ACTIONS_RUNTIME_TOKEN` are present in every
step of the job, so an extraction escape could still mint an OIDC token for a fraudulent
attestation. Nor does a step boundary remove the GitHub App token `actions/checkout`
persists into `.git/config`. A separate job with `permissions: {}` removes all of it.

**The `ASSEMBLED_SHA256` argument stops being circular.** An earlier draft justified the
residual token exposure by saying the committed digest "means tampered bytes cannot reach
the signing step at all — the attacker's path to a *signed* artifact is closed
independently of the token question". That does not hold when assembly and the check share
a checkout: `tasks/vendor/pins.py` and the code enforcing it are exactly what a
path-traversal escape targets — the plan itself names "a `tasks/*.py` module that the later
Sign step imports" as the hazard motivating out-of-checkout staging — so an escape defeats
the digest gate in the same run, before Sign. With assembly upstream, the `release` job
reads the pin from its own clean checkout and compares it against bytes that arrived as an
opaque artifact, which is a real boundary rather than a self-referential one.

Version monotonicity is unaffected: the `prepare → sign → attest → finalise` sequence the
job's own comment (`main.yml:607-612`) requires to stay together does stay together. The
~1.2GB inter-job transfer is the price, and it is one §8's smoke gate was already paying.

Extraction still lands in a staging directory **outside the checkout**, with only the
finished archives copied into `dist/release/`, and the same entry rules the launcher
applies (Step 1b §4) apply CI-side over the same shared fixture corpus. `_publish` commits
with `git add .` (`tasks/git.py:73`) and the only backstop is the marker list at
`tasks/release.py:22` checked against `git status --porcelain -uall`; `/dist/` is
gitignored and root-anchored (`.gitignore:23`), so archives there are invisible to both,
but a staging tree anywhere else inside the checkout would be swept into the version-bump
commit.

**`persist-credentials: false` is still not adopted, and the reason is unchanged.**
Neither release checkout sets it (`main.yml:475-478`, `:585-588`), so it defaults to
`true`, and `tasks/git.py:35-52` runs a bare `git push --atomic` with no credential
helper, no authenticated remote URL and no `gh auth setup-git`. That persisted app token
is the only credential the release push has — `GH_TOKEN` is `secrets.GITHUB_TOKEN`, set
for the `gh` CLI and not what authenticates the push. Adding the flag without a
replacement wedges every cut after the version bump has been pushed. If the hardening is
wanted it must land together with an explicit credential scoped to the finalise step, and
the test must assert both. Moving assembly upstream reduces what that token is exposed to
without depending on the flag.

**Unix modes and symlinks must survive the zip-to-tar round trip, and they do not by
default.** Assembly extracts an npm tarball *and* Chromium's **zip** and repacks both as
`tar.gz`. Python's `zipfile` does not apply the Unix permission bits stored in
`external_attr`, and it materialises symlink entries as regular files containing the target
path; `tarfile` preserves modes only under a deliberately chosen extraction filter. A
browser tree whose `chrome-headless-shell` lost its executable bit would pass the
structural check, sha256, minisign and `ASSEMBLED_SHA256`, be sealed `0444` on the user's
machine, and fail at `execve` with `EACCES` — unrecoverable without a new release, and
invisible on any platform the smoke matrix does not execute. So `tasks/vendor/archive.py`
reconstructs modes from `external_attr` and symlinks from the `S_IFLNK` marker explicitly,
and a CI-side assertion checks the executable bit on every expected binary in **every**
produced archive, not only the runner's.

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

**The symlink branch is settled empirically per platform, before Phase 1 fixes the
allowlist — and it is expected to stay.** An earlier draft proposed retiring it: since we
produce the archives, if assembly emits no symlink then a CI-side assertion pins that and
Step 1b §4 narrows to regular files and directories only, dropping the hardest-to-review
code in the extractor. That reasoning holds for the Linux trees and is very likely wrong
for darwin: Chromium's macOS build uses the standard framework layout, with
`Versions/Current` and top-level symlinks into `Versions/A`, and flattening them to copies
both duplicates a substantial share of ~177MB and changes the bundle layout that the
upstream code signature's `CodeResources` records — which on arm64 macOS, where a valid
signature is required for execution, is an execution failure rather than a cosmetic
difference. So the darwin layout is confirmed against the pinned `playwright-core` as the
first step of this section; if it contains symlinks or is signed as a bundle, the in-root
symlink branch and its real-root containment checks stay, and the CI-side assertion pins
the expected *set* of symlinks per platform rather than their absence.

The `tar` `default-features = false` justification in Step 1b §1 reasons from mode masking
and does not cover extended attributes, which are a distinct concern on macOS artifacts;
§4's explicit rule that archive-supplied xattrs are never applied is what actually settles
it on both sides.

**Neither task is wired into `release_prepare` or `prerelease_prepare`.** They run in
`assemble-runtime` as their own mise tasks — which is what makes the credential scoping
assertable, since the existing attest-block tests inspect workflow shape and cannot see
inside an invoke call graph. `release_prepare` (`tasks/release.py:144-160`) and
`prerelease_prepare` (`:117-129`) instead gain a step that **consumes** the downloaded
archives: assert each against `pins.ASSEMBLED_SHA256`, **after**
`build.cli_cross_compile` and **before** `build.create_debug_archives`. Nothing fetches
from npm, nodejs.org or the CDN anywhere in the `release` job, and nothing extracts an
upstream archive there either — `_sign` (`:86-100`) remains the only function holding the
secret, and `main.yml` scopes `ACCELERATOR_RELEASE_SECRET_KEY` to Sign steps deliberately
(`:505-508`, `:618-621`, `:642-645`).

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

Five arms follow, and none is optional:

0. **The signed attestation document** — the one artifact the launcher's hit path actually
   verifies (Step 1b §2), and the one this plan previously assumed came for free from the
   manifest's archive signature. It does not: `sign_file` signs the archive *file's bytes*
   and the launcher deletes the archive after extraction, so that signature has nothing
   left on disk to verify against. So assembly emits, per artifact per platform, a small
   JSON document carrying `attestation_format_version`, `artifact`, `platform`,
   `archive_sha256`, `uncompressed_size`, `entry_count` and `table_sha256` — so a
   repointed pointer cannot substitute another artifact or another platform, and the
   `.files` table stays anchored after the archive is discarded.

   ⚠️ **`assemble-runtime` emits the document; the publishing job signs it.** §8's upload
   list must therefore name `<archive>.sealed` and **not** `.sealed.sig` — an earlier
   revision listed both, which is impossible: `assemble-runtime` declares
   `permissions: {}` and is asserted by criterion never to reference
   `ACCELERATOR_RELEASE_SECRET_KEY`, so it cannot produce a signature. Resolving that
   contradiction the other way would put the release signing key into the job that extracts
   three third-party archives, which is precisely what §3's topology exists to prevent. So:
   the upstream job uploads archive + `.sealed`; `_sign` produces `.sealed.sig` alongside
   the archive's `.minisig`; all four assets are uploaded and re-verified.

   **The publishing job re-derives the two size fields rather than signing what arrived.**
   `archive_sha256` is independently anchored (the archive is checked against
   `ASSEMBLED_SHA256` from the release job's own clean checkout), but `uncompressed_size`,
   `entry_count` and `table_sha256` are measured upstream and travel the same unpinned
   workflow-artifact channel — and those are exactly the launcher's decompression-bomb
   ceiling and table anchor. Signing them unverified would hand a release-key signature to
   whatever tampered with the artifact. Since the archive is pin-verified by then, the
   publishing job walks it to recompute all three and refuses on any disagreement with the
   emitted document.

   **Everything in the document is knowable in `assemble-runtime`.** That is the
   constraint that shapes it: the job runs upstream of `version.bump`, so it cannot know
   the plugin release version — and one archive set serves both the stable and pre.0
   cuts, so a version field would need two values for one set of bytes. Step 1b §2
   therefore keeps `release_version` and the launcher-owned `layout_version` out of the
   signed body entirely and anchors rollback on the launcher's compiled-in expected digest
   instead. Signing happens in the publishing job as for every other asset; the document's
   *content* is fixed upstream and carried forward with the archives, so signing adds no
   knowledge and needs none.

   **The `.files` table is emitted inside the archive, not hashed into the document.**
   Assembly writes it at the tree root, so the signed archive digest covers it; the
   launcher reads it rather than writing it. That removes the requirement — which an
   earlier revision created — that a Python producer and a Rust consumer serialise an
   identical table byte-for-byte in order for a digest comparison to hold.
1. **Signing** — `sign_staged_binaries` (`tasks/signing.py:60-79`) builds an explicit
   expected list from the launcher plus `_subbinary_signing_targets()` and raises on
   any missing member, deliberately never scanning a directory. A
   `_tree_artifact_signing_targets()` arm joins it, so a partial assembly fails closed
   exactly as a partial cross-compile does. `sign_file` needs **no per-target flag**: the pinned
   `minisign 0.12` prehashes under a plain `-S`, so tree archives are already in the form
   the launcher's streamed verification requires — see Step 1a §2. ⏱️ `sign_file`
   (`:24-43`) runs one invocation per file under a **120-second timeout sized for an 8MB
   binary**; eight ~120MB archives plus eight documents join the existing 32 invocations,
   so the timeout is **re-derived from a measured signing run over a real archive and
   stated as a number**, not inherited.
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
   joins it, each archive with **three** sidecars: its `.minisig`, its `.sealed`
   attestation and that document's `.sealed.sig`. Today's 70 uploads become 102 (8
   archives × 4 files). The existing `missing` check (`:339-343`) then fails loudly on an
   unassembled artifact before a single upload starts.
4. **Re-verification** — `_subbinary_reverifies` (`:287-315`) reads
   `manifest["binaries"][name]` and re-downloads each asset to check its sha256 and
   inline signature. A `_tree_artifact_reverifies` arm reads `manifest["artifacts"][name]`
   and does the same for the archive **and its attestation document**, so the
   `--draft=false` transition (`:356`) waits on both — an artifact whose attestation
   failed to upload would otherwise publish a tree no launcher could ever resolve, which
   is a 404 on the hit path rather than a recoverable miss.

Without all five, the release publishes a *signed* manifest naming artifacts that were
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
   `TREE_ARTIFACTS`. It asserts the full **`(artifact, platform)` cross-product** against
   `TREE_ARTIFACTS × TARGETS`, not key-set equality: a key-set check passes a manifest
   whose `artifacts.browser` carries three of four platform entries, and the `missing`
   file check in `upload_and_verify_release` sees only files the registry enumerates
   rather than the manifest's own platform map — so a partially-assembled artifact set
   could reach the signed, published manifest for one platform's users, through the one
   guard added to prevent exactly that.
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
   three derived terms; a fourth term for tree artifacts joins it — derived as
   `len(TREE_ARTIFACTS) × len(TARGETS) × 4` rather than hard-coded — and `_setup_release`
   (`:275-347`) stages the archives and all three sidecars so the count is reachable.
   `_SUBBINARY_DESCRIPTIONS` (`:35-50`) is keyed by dispatched token and does **not**
   need an artifact entry — tree descriptions come from the assembly, not from a
   `Cargo.toml` — but the fixture's manifest writer does.

A guard that turns out **not** to trip: `_assert_no_leaked_artifacts`
(`tasks/release.py:40-54`) matches its markers against `git status --porcelain -uall`,
and `/dist/` is gitignored, so the archives are invisible to it. Worth recording so
nobody spends time on it.

#### 7. Release-job capacity and the failure envelope

**Files**: `.github/workflows/main.yml`, `tasks/github.py`, `RELEASING.md`
**Changes**: The `release` job (`main.yml:554-659`) runs the whole pipeline **twice** —
stable, then the post-stable pre.0 cut — so roughly 2.4GB of upload per stable release, on
a `macos-latest` runner with **no `timeout-minutes`** (the only ones in the file are at
`:125`, `:335` and a step-level `:372`) and no disk guard. `dist/release/` is never
cleaned between the two passes, and `--clobber` on retry (`:318-319`) re-uploads the lot.

⏱️ **The dominant new cost is serial transfer, and §8's reuse does not touch it.**
`upload_and_verify_release` (`:322-365`) uploads with a serial `for path in uploads: gh
release upload` loop and then re-verifies with a serial loop spawning a fresh `gh release
download` per asset — so the tree artifacts add roughly 480MB up **and** 480MB back down
per pass, doubled by the two passes, on a 3-vCPU runner. §8's deterministic-reuse
mechanism removes the re-assembly CPU, which is the smaller term; claiming it "removes the
duplication itself" would credit it with the wrong one. Three consequences, all of which
must land here rather than being discovered at the first slow cut:

- `timeout-minutes` is sized from a **measured** double-pass dry run, not a guess, because
  the abort runs no cleanup arm.
- The in-job pre.0 pass reuses the local `dist/release/` copies rather than re-downloading
  the previous release's assets — the bytes are already on disk from the stable pass.
- Upload and re-verification are **bounded-parallel** rather than serial. The assets are
  independent, and the serial loops are what turn ~2GB into a wall-clock problem.

A whole-job disk-space assertion joins them, stated as a number against the staging tree
(four platforms' extracted Chromium alone is ~700MB) rather than only asserted at runtime.
Hosting capacity itself is confirmed and assumed.

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
tagged and pushed the version bump, under the `accelerator-release` concurrency lock.

🔴 **And it does more than burn a version number.** The pushed commit carries
`marketplace.update_version`'s edit setting `.claude-plugin/marketplace.json`'s
`source.ref` to `vX`. Deleting the tag with `--cleanup-tag` therefore leaves `main`
advertising a plugin ref pointing at a git tag that no longer exists — breaking fresh
installs and `/plugin update` for **every** user of the marketplace until someone pushes
a correction. A transient download hiccup during a ~480MB re-verify would cause an
outage, not an inconvenience.

**So the `except Exception` delete arm is removed outright, not narrowed.** An earlier
draft proposed reserving it "for an explicit, enumerated set of pre-upload failures" —
but the `try` block begins with the upload loop and the `missing` check sits outside it
(`:339-344`), so after that narrowing there would be **no pre-upload failures inside the
envelope at all**. The arm would be dead code that still fires on anything unanticipated.
Once `_publish` has pushed, a preserved draft plus the forensic alert that already exists
(`:37-40`, `:153`) is strictly safer than any automatic tag deletion, and `--clobber`
means the draft can be re-driven. A criterion asserts no code path deletes a tag that
`git.push` has published.

The supporting changes stand on their own merits regardless: size
`download_release_asset`'s timeout to the expected asset rather than a flat 120s, and
wrap both re-verify helpers so a transport failure surfaces as an
`AssetVerificationError` with a diagnosable cause. At ~2.4GB per stable cut, `OSError: No
space left on device` from a re-verify download or from `compute_sha256`, a hung `gh
release upload` (`_upload_clobber` has neither timeout nor retry, `:318-319`), a hung shim
verification (`_run_shim` likewise, `:170-183`), and a `CalledProcessError` from a
transport blip are all now plausible; bounded retry with backoff wraps `_upload_clobber`.

⚠️ **A job timeout needs a recovery path that exists.** The plan previously recorded
`--clobber` as the recovery for the newly-added `timeout-minutes`, but that recovery is
unreachable through the workflow: `release_prepare` begins with `git.pull` then
`version.bump`, so re-running the job after a timeout bumps to the *next* version against
the already-pushed commit, and `--clobber` only helps if `release:finalise` is re-invoked
against the same staged `dist/release/` — which the runner no longer has and no workflow
entry point offers. The realistic consequence is a pushed bump, a pushed tag, a
marketplace ref and a partial draft, recoverable only by a manual local re-sign against
the production secret, which puts that secret on a laptop. So a **re-drivable finalise
entry point** lands with the timeout: a `workflow_dispatch` taking an explicit version
that skips the bump and re-runs sign/upload/finalise against a re-downloaded draft. The
procedure is documented in `RELEASING.md` and walked once before the phase closes.

Worth recording as already-safe: `_release_reverifies` is built at `:344`, *before* the
`try`, so a manifest `KeyError` — a token in the registry but absent from the staged
manifest — raises outside the delete envelope entirely.

#### 8. Reuse across cuts, and a functional gate

**Files**: `tasks/vendor/assemble.py`, `.github/workflows/main.yml`
**Changes**: Two problems that only appear once assembly is in the pipeline.

**Every release becomes dependent on three third-party hosts.** `assemble-runtime` runs
on every cut, fetching from
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
and gzip written without an embedded timestamp. This is worth doing on its own merits —
it makes a release auditable by anyone who can run the same pins — but it is also the
precondition for everything below, because an unreproducible archive cannot be pinned.

**The test asserts the mechanisms, not just the outcome.** "Assemble twice and compare
digests" is the obvious test and it is nearly worthless on its own: both assemblies run in
the same process, on the same host, within the same second, so the result is invariant to
every factor that actually threatens reproducibility — a different DEFLATE encoder or
level, different tar PAX/GNU record choices, a different `umask`, filesystem readdir
order, locale-dependent sorting, or APFS filename normalisation when Linux trees are
staged on macOS. So each normalisation is asserted independently, with a negative case:
sorted order against a **shuffled** input directory; mtime/uid/gid/uname against the
constants read back out of the emitted tar headers; gzip bytes 4-7 zero; modes masked.
The double assembly additionally runs under a different `TZ`, `LANG` and `umask` than the
first.

**And the compression implementation is fixed, not only its inputs.** Byte-identity
depends on the encoder as much as on what is fed to it, so the gzip member is written with
an explicitly pinned encoder at a pinned level rather than whatever the ambient
zlib happens to be. Without that, a `macos-latest` runner image bump changes every digest
and presents as a supply-chain alarm rather than an environment difference.

**A committed expected digest.** `pins.py` gains `ASSEMBLED_SHA256`, one digest per
artifact per platform, committed and reviewed under the same trust-anchor refresh
procedure as the keys and the upstream pins (§2). Every archive that reaches the signing
step — freshly assembled or reused — is checked against it **from the `release` job's own
clean checkout**, against bytes that arrived as an opaque workflow artifact, and a
mismatch fails the release. Without this the digest check is self-referential: a matching
digest computed from whatever is on disk proves only that the bytes are the bytes.

Two things make the anchor itself trustworthy. Its refresh triggers include our own
assembly and build-dependency changes, not only upstream pin movement (§2) — otherwise an
unrelated refactor silently makes the project unreleasable. And the value is **reproduced
by a second independent environment before merge**: a PR-triggered job with
`permissions: {}` re-runs the deterministic assembly on any change to the pins, the
assembly code or the build dependency group, and annotates the digests it produces. The
reviewer then compares two machine-produced values rather than trusting one produced on a
maintainer's unaudited laptop, and a determinism break surfaces on the PR that causes it
rather than at the next cut. That job runs on the same runner OS and architecture as
`assemble-runtime`, and the refresh procedure records that requirement, because a digest
regenerated on a different host is not comparable.

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

The smoke check therefore runs as `smoke-runtime`, a **separate job with
`permissions: {}`** that `needs: assemble-runtime` and consumes the archives as workflow
artifacts: unpack the driver and browser, execute the Node binary and the headless shell
with `--version`, and assert `NOTICES/` is populated. `release` then `needs:` both jobs.

**This is why §3 moves assembly upstream.** With assembly inside `release`, gating a
publish *step* on a downstream job is a `needs:` cycle — GitHub Actions has no construct
for it — so the gate as previously written was unimplementable, and its stated fallback
(keep only the structural check in-job) silently dropped the one control distinguishing
"signed" from "works". Upstream, it is an ordinary job dependency, and the ~1.2GB
inter-job transfer that §3 previously counted as a cost against isolation was already
being paid here.

⚠️ **The smoke job is a matrix over all four targets, not the release runner's platform.**
The `release` job runs on `macos-latest`, which is arm64 — so a host-only smoke check
executes exactly `darwin-arm64` and ships the other three unexecuted, including both
Linux targets, which are the ones the entire vendoring exercise exists for and the only
ones carrying the libc probe and the loader question. Since `smoke-runtime` is already a
separate artifact-consuming job, a matrix costs nothing architecturally. `TARGETS`
(`tasks/shared/targets.py:4-8`) publishes four platforms and each needs a host that can
execute its binaries:

| Artifact | Runner | Execution |
|---|---|---|
| darwin-arm64 | `macos-latest` | native |
| darwin-x64 | **`macos-15-intel`** | native |
| linux-x64 | `ubuntu-latest` | native |
| linux-arm64 | `ubuntu-24.04-arm` | native |

**Every leg executes natively.** `macos-13` — the obvious darwin-x64 label, and the one an
earlier revision named — is a **retired** image: a workflow requesting it does not fall
back, it fails to find a runner, which would block every release. `macos-15-intel` is the
surviving Intel image.

Native Intel execution is chosen over running the darwin-x64 archive on the arm64 runner
under Rosetta 2, and the reason is that Rosetta cannot discriminate architecture. An arm64
binary mistakenly assembled into the *darwin-x64* archive runs **natively** on an arm64
host with Rosetta never invoked, so a Rosetta leg would pass a wrong-architecture
artifact; a native Intel host refuses it outright. Under Rosetta the gate would depend
entirely on the structural check being correct about architecture, whereas native
execution makes the two checks independent — which is what you want from a gate whose
whole purpose is catching an assembly that is signed, hashed and structurally wrong.

⚠️ **`macos-15-intel` is announced as the last Intel image, so this leg has a known end of
life.** The plan records the fallback order rather than leaving it to be rediscovered:
first Rosetta on the arm64 runner, accepting the reduced assurance and leaning on the
structural check for architecture; failing that, the structural check alone for darwin-x64
with the loss of execution coverage recorded. The cleanest answer at that point is
probably to retire the `x86_64-apple-darwin` target altogether, which is a
`tasks/shared/targets.py` decision spanning the whole distribution rather than this plan's
to take.

`ubuntu-24.04-arm` is free only for public repositories, so confirm this repository's
entitlement before wiring, falling back to a QEMU-based leg if absent.

The assemble and reproduction jobs pin explicit image labels rather than `-latest`, since
§8 requires the reproduction job to match `assemble-runtime`'s host for
`ASSEMBLED_SHA256` comparability and a floating label defeats that.

⏱️ **Artifacts are uploaded per target, not as one set.** A single `upload-artifact` of
all eight archives means each of the four matrix legs downloads ~1.2GB to execute the
~300MB it needs, turning the stated one-time transfer into roughly 6GB per cut and
invalidating §7's measured `timeout-minutes`. So `assemble-runtime` uploads
`accelerator-runtime-<platform>` per target — **including each archive's `.sealed`
document**, which the naive `dist/release/accelerator-*.tar.gz` glob would silently omit,
leaving the release job unable to publish the one artifact the hit path verifies. The
`.sealed.sig` is *not* in that upload: it does not exist yet, because the signing key lives
only in the publishing job (§5 arm 0). `release` downloads all four targets; each smoke leg
downloads one.

It runs on **reused** archives as well as freshly assembled ones — a reuse path that
skipped it would be the one route by which an unexecuted artifact reaches a release.

A **structural check** runs in `assemble-runtime` for every platform regardless, since it
needs no matching runner: the expected file set, the executable bit on every expected
binary, and the ELF/Mach-O header and architecture of the Node binary and the headless
shell for the target they claim to be. That catches a wrong-architecture, mode-stripped
or truncated assembly cheaply and covers any target the matrix cannot reach. Between them
the two checks are the only gates distinguishing "signed" from "works".

**Both are exercised in `test:unit:build-system`, not only in the release lane.** As
release-only checks they would first run inside the concurrency lock after `_publish` has
pushed a version bump — the plan's own unrecallable position — and several of Phase 2's
criteria would be assertions about YAML shape rather than about the gate firing. So the
assembly path is built around a **miniature fixture triple**: a few-KB fake "node", a fake
headless shell and a synthetic `browsers.json`. Determinism, the `ASSEMBLED_SHA256` match,
the reuse fallback, `NOTICES/` population, the smoke predicate and the structural
predicate then all run on every CI run, each with a paired **negative** case asserting the
release fails — a tree whose shell will not execute, an archive whose digest does not
match, an artifact whose `NOTICES/` is empty, a cross-compiled artifact with the wrong
architecture.

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

- [ ] Failing test first: *a `SHASUMS256.txt` signed by a key absent from the committed
      allowlist fails the release* — red before `tasks/vendor/nodejs.py` exists, and it
      exercises `classify_status_lines` over recorded fixture output rather than needing a
      crafted keyring
- [ ] Verifications use recorded upstream fixtures rather than live network calls.
      Committing the keys makes Node/GPG fully offline-verifiable, so it is tested for
      real; the SLSA check contacts a transparency log, so its runner is injected and both
      branches asserted — and the plan records that the attestation's *content* is not
      verified in tests
- [ ] `classify_status_lines` is a pure function with a table-driven test over every
      status-line combination, so the negatives below need no host GnuPG
- [ ] A tampered `SHASUMS256.txt` signature fails the release
- [ ] A `SHASUMS256.txt` signed by a well-formed key absent from the committed
      fingerprint allowlist fails the release, **even though `gpg` exits 0**
- [ ] A signature whose `VALIDSIG` primary-key fingerprint differs from its signing
      subkey's is compared against the **primary**, not the subkey
- [ ] A `SHASUMS256.txt` whose revocation is **present in the committed keyring** fails
      the release, and an expired key fails the release, even though both yield
      `VALIDSIG` — the criterion says what is actually verified, since a key revoked
      upstream after our snapshot yields a plain `GOODSIG` against a frozen keyring
- [ ] The digest is matched by exact filename, so a line describing a different artifact
      in the same signed manifest is not accepted
- [ ] An absent `gpg` fails the release rather than silently skipping the check
- [ ] The npm/SLSA path fails closed in each degraded mode — attestation bundle absent,
      transparency log unreachable, `gh attestation verify` unavailable
- [ ] The exact argv passed to `gh attestation verify` is pinned, including `--owner`,
      `--repo` and the workflow identity, so a dropped predicate flag fails locally rather
      than leaving every SLSA criterion green
- [ ] The committed Node keyring and the committed fingerprint allowlist describe the same
      key set — an inconsistent-edit check, and the criterion says so rather than claiming
      to detect a coordinated substitution. **Expiry and maximum keyring age are a
      scheduled guard that opens an issue**, not a unit test — a clock-dependent assertion
      in `test:unit:build-system` reddens every contributor's branch on the day a key
      expires, with no code change
- [ ] 🔒 The trust-anchor guard job exists and **fires on a synthetic diff** to `keys/**`,
      the shared `pins` file and the fingerprint module, failing unless an approving review
      exists from the named reviewer team by someone other than the PR author — so the
      anchor gate is proven to have teeth rather than assumed. Without this the whole
      committed-anchor design rests on an unverified repository setting
- [ ] A scheduled guard opens an issue when `playwright-core`, the Node pin or
      `CHROMIUM_REVISION` exceeds its stated maximum age
- [ ] A `playwright-core` tarball failing its registry signature fails the release
- [ ] A `playwright-core` tarball whose sha512 differs from the **registry-signed
      `integrity` value** fails the release — the binding without which the registry
      signature proves only that the registry asserted something
- [ ] An attestation whose source repository or workflow identity differs from the
      pinned predicate fails the release
- [ ] An attestation whose subject digest does not match the fetched tarball fails the
      release
- [ ] A `playwright-core` version other than `package.json`'s pin fails the release
- [ ] A Chromium revision other than `browsers.json`'s fails the release
- [ ] Chromium bytes whose sha256 differs from `pins.CHROMIUM_SHA256` fail the release
- [ ] A Node version other than the vendored driver's pairing fails the release
- [ ] Assembly runs in `assemble-runtime`, a job declaring `permissions: {}`, and neither
      it nor `smoke-runtime` can reference `ACCELERATOR_RELEASE_SECRET_KEY`; `release`
      `needs:` both. Asserted by a workflow test alongside the existing attest-block
      assertions — and the new invariants join
      `test_invariants_reject_known_bad_shapes` (`test_workflows.py:342`) and
      `test_isolation_rejects_known_bad_shapes` (`:530`) with mutations that grant the
      smoke job a permission, put `GH_TOKEN` into the assembly step's env, and remove the
      timeout, so a renamed job cannot make the guards vacuous
- [ ] `release` extracts no upstream archive and fetches from no upstream host —
      it consumes workflow artifacts and checks them against `pins.ASSEMBLED_SHA256`
- [ ] If `persist-credentials: false` is adopted, the finalise step has an explicit
      credential and the push still authenticates — asserted together, since the flag
      alone breaks every release
- [ ] Extraction happens outside the checkout, and every archive in the shared
      `tests/fixtures/adversarial-archives/` corpus is rejected CI-side by the same rules
      the launcher applies — the same corpus the Rust suite iterates, so a rule tightened
      on one side cannot silently skip the other
- [ ] Unix modes and symlinks survive the zip-to-tar round trip: every expected binary in
      **every** produced archive carries its executable bit, and each platform's symlink
      set matches what assembly recorded for it
- [ ] The assembled, signed, manifest-listed, uploaded and re-verified sets are pinned
      against each other by one test, so an artifact cannot appear in some and not others
- [ ] An unassembled artifact fails the **signing** step, not the upload step
- [ ] A tree archive with no `.minisig` fails `collect_artifact_entries`
- [ ] Each artifact platform emits a `.sealed` attestation, and the publishing job
      **re-derives** `uncompressed_size`, `entry_count` and `table_sha256` by walking the
      pin-verified archive, refusing on any disagreement with the emitted document — so a
      tampered inter-job artifact cannot obtain a release-key signature over inflated
      extraction bounds or a forged table anchor
- [ ] `.sealed.sig` is produced by the publishing job, never by `assemble-runtime`;
      asserted by a workflow test that mutating `assemble-runtime` to reference the signing
      secret is rejected, and that its upload list names `.sealed` without `.sealed.sig`
- [ ] An artifact missing its `.sealed` or its `.sealed.sig` fails the **signing** step
- [ ] Every produced archive contains a `.files` table at its tree root whose rows match
      the members alongside it, so the launcher can verify each entry during extraction
- [ ] The attestation carries no `release_version` and no `layout_version`, asserted
      explicitly — both are values `assemble-runtime` cannot know or must not freeze, and a
      future contributor adding either would reintroduce the two-values-for-one-archive
      and unbreakable-layout-loop failures
- [ ] Every signed asset's `.minisig` is prehashed, asserted per artifact class, so the
      launcher's streamed verification cannot silently lose its precondition
- [ ] `_assert_staged_manifest_is_current` rejects a manifest whose `artifacts` keys
      differ from `TREE_ARTIFACTS`, **and** one missing exactly one `(artifact, platform)`
      entry from the full `TREE_ARTIFACTS × TARGETS` cross-product
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
- [ ] 🔴 **No code path deletes a tag that `git.push` has already published** — the
      `except Exception` delete arm is gone, not narrowed, so no failure can leave `main`
      advertising a `marketplace.json` `source.ref` pointing at a deleted tag
- [ ] The re-drivable finalise entry point re-signs and re-uploads against an explicit
      version without bumping, exercised once end-to-end
- [ ] Every produced archive matches `dist/release/accelerator-*`
- [ ] Assembling the same pin triple twice produces **byte-identical** archives — and
      each normalisation is independently asserted with a negative: sorted order against a
      shuffled input directory, mtime/uid/gid/uname read back out of the tar headers,
      gzip header bytes 4-7 zero, modes masked. The double assembly runs under a different
      `TZ`, `LANG` and `umask` than the first
- [ ] A PR touching the pins, the assembly code or the build dependency group runs the
      reproduction job and annotates the digests it produced, so a determinism break fails
      on that PR rather than at the next cut
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
      secret, asserted by a workflow test, and as a **matrix covering all four targets**
      rather than only the release runner's platform — so no published artifact ships
      unexecuted. Every leg executes natively — darwin-x64 on `macos-15-intel` — and no
      leg requests a retired image label
- [ ] An **arm64** Node binary planted in the **darwin-x64** archive fails the release,
      caught independently by the native Intel leg refusing to execute it *and* by the
      structural check's architecture read, so neither check is the sole discriminator
- [ ] The smoke check runs on reused archives as well as freshly assembled ones, and
      fails the release on a tree whose Node binary or headless shell will not execute,
      or whose `NOTICES/` is empty
- [ ] The structural check fails a cross-compiled artifact whose Node binary or headless
      shell has the wrong architecture or object format for its target
- [ ] The smoke predicate, the structural predicate, the determinism check, the
      `ASSEMBLED_SHA256` match, the reuse fallback and the `NOTICES/` assertion all run in
      `test:unit:build-system` against a **miniature fixture triple**, each with a paired
      negative asserting the release fails — so none is first exercised inside the release
      job after `_publish` has pushed
- [ ] The produced tree contains a `NOTICES/` entry per expected component, driven from
      the assembly's own component list
- [ ] An end-to-end round trip: a synthetic tree assembled through the real assembly
      path, a manifest emitted through the real `build_manifest`, signed with a test key,
      resolved by the launcher's tree resolver
- [ ] `mise run test:unit:build-system` and `mise run build-system:check` exit 0
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] A full local dry-run assembly produces both artifacts for one platform, and their
      measured sizes are recorded for Phase 3's re-derivation of Step 1a's fetch deadline
- [ ] Each produced `.tar.gz` has a prehashed `.minisig` that the CLI-side verifier and
      `minisign-verify` both accept, and each `.sealed` a `-S` signature the launcher's
      embedded-key path accepts
- [ ] `sign_file`'s timeout is re-derived from a measured signing run over a real ~120MB
      archive and recorded as a number
- [ ] `timeout-minutes` for the publishing jobs is set from a measured double-pass dry run
- [ ] The upload list and the re-verify list, printed for one platform, each contain both
      tree archives and all three of each one's sidecars
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
`chromium.executablePath()` (`:152`), both present in `playwright-core`.

⚠️ **But the loader cannot simply be deleted in favour of a bare specifier.** The
`exports`-map interpretation the 0072 fix added is retired; the *absolute-path resolution*
is not, because `NODE_PATH` does not reach ESM (see Current State Analysis). A bare
`import 'playwright-core'` from `daemon.js` would walk `node_modules` upward from the
plugin tree and never enter the sealed driver tree, failing with `ERR_MODULE_NOT_FOUND` on
precisely the machines the vendored runtime serves.

So the loader is **narrowed, not removed**: it keeps `pathToFileURL(...)` over a root
supplied by the executor — now the driver tree rather than the lockhash namespace — and
loses the `exports`-map selection, the CJS-shim branch and the `playwright`-versus-
`playwright-core` package distinction. Its three `fake-playwright*` fixture trees collapse
to one, since the shapes they distinguished no longer have branches to exercise, and
`playwright-loader.test.js` shrinks accordingly rather than being deleted.

The 0072 regression does not recur: the bug was the loader selecting a CJS shim entry from
an `exports` map it misinterpreted, and no such selection remains. A test asserts
`chromium` is a defined export of the resolved module, which is the property 0072 actually
cared about.

**So only one unit-lane floor moves, not two.** `_EXPECTED_DESIGN_AUTOMATION_SUITES` stays
**9** (`tasks/test/unit.py:62`) because `playwright-loader.test.js` survives; the case
floor `_EXPECTED_DESIGN_AUTOMATION_CASES` (**76**, `:66`) moves down by the number of
cases the retired branches carried, read off the runner's TAP summary rather than guessed.
An earlier draft moved both, on the assumption the loader was deleted outright.

**Amendment (implementation): the case floor needed no numeric edit.** The pre-change TAP
total was **78** against a floor of **76** — the floor already carried two cases of slack.
The narrowing retired exactly the two loader cases (`exports["."].import` selection and the
non-string-import throw), so the actual total landed at **76**, equal to the existing floor.
The floor is now tightly calibrated at the live total with no change to `tasks/test/unit.py`,
and the suite floor stays at 9.

Leaving the case floor unmoved fails `test:unit:design-automation`, which is in the
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

- **Warm, on a tree-consuming dispatch**: for each tree `acquire` resolves, the launcher
  exports `ACCELERATOR_TREE_<NAME>` — a generic name, not a `DESIGN`-prefixed one, so a
  second tree consumer inherits the convention rather than a design-shaped variable.
  That is `acquire`'s handful of local reads and `lstat`s per tree (Step 1b §2), issues no
  network request, probes no cache root, and has no failure mode: a tree that is absent,
  unpointed, unparseable, or failing its ownership check simply yields no variable.

  **The names come from the compiled-in artifact set, not from a directory scan.** An
  earlier draft said the launcher "enumerates rather than knows" by reading whatever
  pointer files were present, while the Performance section said the opposite; the
  compiled-in set is correct for three reasons. It is bounded — pointers are version-keyed
  and accumulate, so a scan of a shared cache root grows with every plugin version ever
  installed, turning a fixed-cost path into O(releases). It keeps unvalidated on-disk
  filenames out of the environment of every dispatched sub-binary. And it is the only
  reading under which the clearing invariant below is achievable at all: a tree with no
  pointer yields no *name* to clear under enumeration, so an injected variable for an
  unmaterialised tree would survive precisely in the cold-cache case the clearing exists
  to defend. The set is already pinned by the `TREE_ARTIFACTS` drift test and is what
  makes `cache verify`'s offline name validation coherent.

  **The export is confined to dispatches that consume trees.** Exporting on every external
  dispatch would charge `accelerator vcs guard` — a PreToolUse hook firing on every tool
  call — and every SessionStart hook with tree resolution for a subsystem only
  `accelerator design` uses, so a degraded cache root would surface as a failure in
  unrelated work rather than in design work. It also means a hard-mounted unresponsive
  `ACCELERATOR_CACHE_DIR` cannot wedge the whole session.

  The variables are **always set or explicitly cleared**, never merely left alone, so an
  inherited or injected value from the surrounding environment can never be mistaken for
  one the launcher resolved.
- **Cold, only when needed**: `accelerator-design` calls `accelerator cache ensure
  <name>...` at the point in its own ordering where it has established that it needs the
  runtime, naming both trees in one invocation so they materialise concurrently. That is
  the only place a ~294MB fetch can be triggered, so `validate-source`, `resolve-auth`,
  `scrub-secrets`, `notify-downgrade` and `audit-cue-phrases` never touch the network, and
  `notices` reads whatever is already materialised.

  `ensure` runs **before** the executor takes its `FileLock` on `launcher.lock`
  (`design-cli/src/executor.rs:160`), alongside the platform probe. Inside that lock, a
  first-run download would block every concurrent design invocation for minutes behind a
  lock whose contention is reported as `another-launcher-running` — a misleading diagnosis
  for a download in progress.

An absent variable is therefore the normal state rather than an error: it means "not
materialised yet", and the executor decides whether to `ensure`, downgrade, or proceed.

⚠️ **The clearing must happen before the dev-override short-circuit, not on the resolve
path.** `ACCELERATOR_DESIGN_BIN` is read at
`cli/launcher/src/launch/outbound/mod.rs:21-47` and returns early from
`LazyProductionResolver::resolve` (`main.rs:61-63`), so clearing code placed on the resolve
path never runs — and "never set" is not "explicitly cleared". With the override in use
(the standard development and container-fixture configuration) an ambient or injected
`ACCELERATOR_TREE_DRIVER` would pass straight through and be treated as launcher-resolved,
with `NODE_PATH` and the browser executable derived from it. So the clear-then-set of the
tree variables happens on the dispatch path ahead of `resolve`, and the override case
clears without setting — after which the executor reaches `ensure` exactly as it would on
a cold cache.

**The `ensure` contract**, since this is a machine-consumed interface between two
separately-built executables:

- **Discovery.** `accelerator-design` must locate the launcher to invoke it, and
  `argv[0]` is its own content-addressed cache path. The launcher exports its own resolved
  shim path as **`ACCELERATOR_LAUNCHER_PATH`** — deliberately *not*
  `ACCELERATOR_LAUNCHER_BIN`, which this plan's own Key Discoveries identifies as unsafe:
  `derive_override_var("launcher")` (`core.rs:268-293`) produces exactly that string,
  `launcher` is in `RESERVED_TOKENS`, and the `ACCELERATOR_<SUB>_BIN` namespace means "exec
  this path unverified". Exporting into it would give one name two incompatible meanings,
  inherited by every descendant of every dispatch, and would turn a future promotion of
  `launcher` to a dispatchable token into a silent resolution bypass. A guard test asserts
  no exported variable collides with `derive_override_var`'s output for any reserved or
  dispatched token.

  **Discovery falls back rather than failing.** The variable is the first choice, but the
  dev-override path deliberately does not set it, and that is the configuration developers
  and the container fixtures use — so treating its absence as terminal would make tree
  materialisation unreachable without a full release cycle. The order is
  `ACCELERATOR_LAUNCHER_PATH` → `${CLAUDE_PLUGIN_ROOT}/bin/accelerator` → `PATH`, and only
  exhausting all three yields `artifact-unavailable` with a cause naming why. (An earlier
  draft said both that the override path reaches `ensure` normally and that it reports
  `artifact-unavailable`; this is the reconciliation, and the criteria pin the first.)
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

**A failed materialisation is sticky for the session — but only for *persistent* causes.**
A crawl makes 100–200 executor invocations, and with no negative caching a persistent
failure — a full disk, a read-only plugin root, a flapping link, a 404 for one platform —
would produce a fresh full-size attempt, times three fetch retries, on *every one* of
them. A single crawl on a failing machine could attempt tens of gigabytes and repeatedly
fill the user's disk with partial archives. This risk did not exist for megabyte-scale
single-file sub-binaries.

⚠️ **Which causes are sticky is the load-bearing distinction, not the stickiness itself.**
The enumerated persistent causes — disk shortfall, unwritable cache root, signature or
digest mismatch, platform unsupported, artifact absent from the manifest, host
unreachable — suppress re-attempts, and the remaining invocations take the code-only path
immediately. But `materialisation-in-progress` (Step 1b §2's single-flight waiter
expiring) is **not** sticky: it means another invocation is actively downloading and will
succeed shortly. Treating it as persistent would mean that on a cold cache over a
slow-but-healthy link, invocation 2 loses the race, writes a marker, and invocations 3-200
degrade to code-only for the rest of the crawl *because materialisation was working* —
the exact outcome the artifacts exist to prevent, on exactly the machines they exist to
serve.

The marker lives in the executor's own state directory — the `0700` directory the
sibling established at `<repo>/<paths.tmp>/inventory-design-playwright`
(`design-adapters/src/paths.rs:51-56`, created at `design-cli/src/executor.rs:103-116`)
— **not** beside `trees/`. Two of the failure causes it exists to damp are a full disk
and an unwritable cache root, so a marker written into the cache root could not be
created in exactly the cases that recur. It records the artifact name, the cause and a
timestamp.

🔒 **That directory is inside the repository being inventoried**, which for this skill is
routinely an unfamiliar project. So the marker path is validated before use — refused if
it is a symlink or not owned by the effective uid — and its validity is keyed to the
current session, so a pre-planted marker cannot satisfy it. Without that, an untrusted
repository could suppress its own design-inventory findings for the marker's TTL simply by
committing a file, and the suppression is silent by construction.

**Clearing is the direction that must not fail.** Suppression is the safe error; a marker
that never clears means a user who freed disk space, reconnected, or ran the documented
`accelerator cache repair` remediation still gets the code-only crawler indefinitely — with
no message, because the marker's whole purpose is to suppress the attempt that would
produce one. So it is cleared by any successful `ensure` **and** by `cache repair`, making
the documented remediation also the reset, and its TTL is derived from the crawl bound:
a crawl is bounded at five minutes, so a TTL of that order suppresses within-crawl retries
without stranding the next crawl. All three paths carry criteria; the TTL is exercised over
the injected `Clock` rather than by sleeping.

Tree-related failure envelopes also carry a remediation string naming `accelerator cache
repair <name>`. ADR-0060 accepts as a known negative that a truncated tree "surfaces as
a confusing runtime failure until the repair path is run" — but self-healing needed no
discovery, whereas this needs the user to already know a command exists that the failure
never mentions. Naming it in the failure is what makes AC14's recovery reachable in
practice rather than only documented.

🔴 **The Node executable is retargeted too — it is a second threading site, and without
it nothing else in this plan works.** `const NODE: &str = "node"` (`executor.rs:28`) is
used verbatim as `program: PathBuf::from(NODE)` in **both** `DaemonSpawner` (`:163`) and
`ExecClient` (`:174`): a bare name resolved through `PATH`. An earlier draft asserted the
environment vector was "the single place a resolved browser path is threaded" and edited
only that, which would leave the executor shelling out to a system `node` — the very
prerequisite this plan exists to remove — and fail AC6's Node-absent fixture with `ENOENT`
after ~294MB had been fetched, verified and sealed.

So both `program` fields derive from the resolved driver tree, and the `NODE` constant is
retired so a bare-name spawn is unreachable. Program and environment are resolved together
into one `ResolvedRuntime` value threaded to both call sites, so there is genuinely one
site rather than three — the shape the earlier draft assumed already existed.

The executor also holds the lease: it opens and `LOCK_SH`es the lease path `ensure` prints
(or that `acquire` returned) before spawning the daemon, so the holder is a process that
outlives the resolution rather than an `ensure` child that has already exited.

The rest of the environment follows. The one vector at
`design-cli/src/executor.rs:139-156` — shared by `DaemonSpawner` and `ExecClient` — stops
deriving `NODE_PATH` and `ACCELERATOR_PLAYWRIGHT_NS_ROOT` from
`namespace_root.join("node_modules")` and derives them from the driver tree instead,
gaining `ACCELERATOR_DESIGN_BROWSER_EXECUTABLE` from the browser tree. (`NODE_PATH` is
retained for any CommonJS the driver bundle loads internally; it is *not* what resolves
`playwright-core` for `daemon.js`, which is ESM — see §1.) The layout
precondition `runtime_is_installed` (`:118-122`) enforces — today exit 3
`playwright-not-installed`, envelope at `cli/design/src/executor/envelope.rs:43,55,66,76-79`
— becomes an `artifact-unavailable` downgrade rather than a hard failure, since the
artifacts are now fetchable. `design-adapters/src/paths.rs`'s `lockhash`, `cache_root`
and `namespace_root` (`:64-129`) go with it, along with
`cli/design-adapters/tests/lockhash_golden.rs`.

#### 4. Failure ordering and the platform probe

**Files**: `cli/design/src/runtime/platform.rs` (new),
`cli/design/src/runtime/availability.rs` (new),
`cli/design/src/runtime/ports.rs` (new — the availability ports, incl.
`BootstrapDiagnostics`), `cli/design/src/executor/ports.rs` (the `Spawner` error),
`cli/design/src/executor/launch.rs` (invoke diagnostics on readiness failure),
`cli/design-adapters/src/platform.rs` (new), `cli/design-adapters/src/process.rs`,
`cli/design-cli/src/executor.rs`, `cli/pup.ron`
**Changes**: ADR-0062 requires the runtime check to come **before**
`design.browser_path` is consulted, because the hatch substitutes the browser and never
the runtime. A musl host must reach the code-only downgrade, not a browser-path error.
Nothing enforces any such ordering today because neither check exists.

Order: platform supported? → runtime available? → browser resolvable (bundled, then
`design.browser_path`)? Each failure emits its downgrade reason, and the default and
hybrid crawler modes fall back to the code-only crawler. An explicit `--crawler runtime`
request hard-fails.

**The ordering itself lives in `design::runtime::availability`**, a domain module
returning either a resolved runtime or a `DowngradeReason`, with the platform probe,
`ensure` and browser-path resolution injected as ports. Without a named owner the sequence
accretes as a chain of `if`s in `design-cli/src/executor.rs`, whose own doc comment states
that "Nothing here decides anything. The launcher's sequence, its verdicts and its envelope
taxonomy all live in the domain" — and it would then be testable only through process
invocation, while this phase's own criterion asks for it at unit level over injected
inputs. The sticky-marker policy is a decision too, so it lives there as well, with the
marker store behind a port.

**`design.browser_path` short-circuits browser *materialisation*, not the runtime check.**
"Browser resolvable (bundled, then `design.browser_path`)" is ambiguous between "bundled
tree not materialised" and "bundled tree materialised but broken", and the two readings
give opposite behaviour: one downloads ~177MB on every cache-cold run before reaching a
hatch the user set precisely because the bundled browser does not work on their host. So
the predicate is explicit — an explicitly configured `design.browser_path` skips browser
materialisation entirely, while the driver `ensure` still runs, and the bundled browser is
preferred only when the hatch is unset. A criterion asserts **zero** browser fetches when
`design.browser_path` is set.

The platform check needs a mechanism that exists nowhere in the codebase today.
`HOST_PLATFORM` (`resolve/mod.rs:21-28`) is a compile-time constant reading `linux-x64`
on Alpine and Debian alike — `TARGETS` builds Linux against `*-unknown-linux-musl`
precisely so one binary runs on every libc — and the manifest's platform axis carries no
libc dimension. Nothing in the existing resolution path can tell the two apart, so
without a probe an Alpine host fetches ~294MB of glibc-linked artifacts, seals them, and
dies at `execve` with a bare `ENOENT` from the absent dynamic loader: the hard failure
AC11 exists to prevent, at maximum cost.

The mechanism, settled by work-item:0214 SQ-1 against prototypes on six hosts, is a
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

**Observation 2 is three-valued, not a boolean.** `/bin/sh` is not guaranteed to exist,
be readable, or carry a `PT_INTERP` at all: distroless and scratch-based images have no
shell — and that is exactly what the AC6 and AC11 container fixtures are — while a
busybox-static `/bin/sh` on a *glibc* host has no interpreter to read. So the port yields
`MuslLoader`, `OtherLoader(basename)` or `Unobservable(reason)`, and `Unobservable`
classifies as **not-musl** under the fail-open policy below rather than being conflated
with either answer by an adapter forced to invent a representation. The enumerated test
shapes gain a distroless case alongside the six existing ones.

⚠️ **Two further conditions stop the artifacts running on hosts that pass both
observations, and they dominate in practice.** The boundary as stated is "glibc *and* a
resolvable psABI interpreter", but a real glibc host still fails if:

- **The glibc version is too old.** Chromium and modern Node need roughly glibc ≥ 2.28-2.31,
  so CentOS 7, Debian 10 and Ubuntu 18.04 pass both observations and then die with
  `GLIBC_2.xx not found` — a symbol-version error.
- **Chromium's shared libraries are absent.** `libnss3`, `libatk`, `libgbm`, `libasound2`
  and siblings are why upstream ships `playwright install --with-deps`, and ADR-0057
  records the gap explicitly as a negative consequence.

**Neither is decided by the pre-fetch probe. Both are classified from the spawn failure
instead** — and that is a deliberate reversal of an earlier revision, which tried to probe
for them and specified mechanisms that cannot work from this binary:

- `confstr(_CS_GNU_LIBC_VERSION)` reports the **calling** binary's libc. `TARGETS` builds
  Linux static-musl, so it reports musl on every host, including the glibc ones whose
  version it was meant to read.
- Reading the shared-library set needs `dlopen`, which is a stub that always fails in a
  static musl binary; the fallback — searching the filesystem — has to reproduce
  per-distribution search paths (Debian multiarch, Fedora `/usr/lib64`, NixOS's store,
  `/etc/ld.so.cache`'s glibc-private format), which is precisely the axis the probe is
  trying to classify. A `DT_NEEDED` snapshot also lists only *direct* dependencies, while
  much of the `--with-deps` set is `dlopen`ed lazily, so a passing check would prove
  little and a failing one would misfire on NixOS by construction.

So the split is: **the pre-fetch probe answers what a filesystem observation can answer;
the loader answers the rest, because only the loader actually knows.** A failing spawn
carries precise signals — `version \`GLIBC_2.34' not found` for a version shortfall and
`error while loading shared libraries: libnss3.so` naming the *actual* missing soname — so
classifying them yields a better diagnostic than any probe could, including the exact
library to install rather than a guessed package list.

⚠️ **But the executor has no observation point for them today, and this is the edit set
that creates one.** The daemon is spawned with `setsid` and **both stdout and stderr
redirected into `bootstrap_log`** (`design-adapters/src/process.rs:113-145`); the parent
never `waitpid`s it; `Spawner::spawn` returns `Result<Identity, kernel::Error>`
(`design/src/executor/ports.rs:72-80`), carrying neither exit status nor stderr; and the
only failure path is `await_readiness`, which polls for 30 seconds and returns
`DaemonStartTimeout { bootstrap_log }` — it *names* the log and never reads it
(`design/src/executor/launch.rs:100-143`). `ExecClient` uses `command.exec()`
(`process.rs:262-278`), replacing the process image, so on that path there is no Rust
process left to classify anything.

Two additions, both in Phase 3 §4's edit set:

- **A `BootstrapDiagnostics` port** that reads the bootstrap log — already `0600` and
  already truncated per attempt — invoked by the launch state machine when readiness
  fails, with a pure `classify(lines) -> Option<DowngradeReason>` in
  `design::runtime::availability` over recorded fixtures. Classification runs on the
  *spawn* path only; the `exec` path keeps `executor-ping-failed`.
- **A distinguishable `Spawner` error.** `loader-unresolvable`'s post-fetch arm is **not**
  "exit 127 + stderr": an absent `PT_INTERP` makes `execve` fail with `ENOENT`, so
  `spawn()` errors with no child, no stderr and no exit status at all. An earlier revision
  cited `cannot execute: required file not found`, which is a **bash** message and
  unreachable here since the executor spawns with no shell. So `Spawner::spawn` carries the
  raw `io::ErrorKind`, and "program exists but `execve` says `ENOENT`" is what distinguishes
  a missing interpreter from a missing program.

🔒 **Classification treats the log as untrusted input.** It matches whole lines against
fixed patterns within the first N lines of a *failed* start — never a substring search over
arbitrary output — and validates every extracted token before it leaves the classifier
(`[A-Za-z0-9._+-]{1,64}` for a soname, `\d+\.\d+` for a glibc version), falling back to
`executor-ping-failed` on anything else. Without that, anything able to write that stream —
a `design.browser_path` wrapper, an ambient `NODE_OPTIONS`, renderer output while crawling
an untrusted project — could emit a marker substring and force a code-only downgrade,
silently suppressing the inventory findings about that project; and the extracted token is
interpolated into a remediation string an agent reads.

⏱️ These two reasons are **added to the sticky cause set** (§3). They are host properties,
not fetch failures, so a successful `ensure` must not clear them: without stickiness every
one of a crawl's 100–200 invocations pays a full spawn plus the 30s readiness timeout. The
marker for them is keyed to the resolved tree's digest and cleared by `cache repair` or a
digest change, not by a successful materialisation.

That also removes a field from the wire format. An earlier revision carried the
`DT_NEEDED` list in the manifest's artifact entry, which was doubly wrong: the warm path
never loads the manifest (so the check would be unreachable exactly on a populated or
offline cache), and the probe is required to run before any resolution at zero network
cost. `ArtifactPlatformEntry` therefore keeps its three sizes and nothing more.

**The accepted cost, stated plainly:** on a too-old-glibc or missing-library host the
~294MB is fetched, verified and sealed *before* the failure is known, where a working
probe would have refused up front. Three things bound it. The sticky marker makes it once
per session rather than once per executor invocation. The tree is content-addressed, so a
user who then installs the missing packages needs no refetch. And the envelope names the
missing soname or the required glibc version, so the remediation is actionable — which the
opaque failure this section originally set out to prevent was not. The case the probe
*does* still decide cheaply is the common one: Alpine and other musl hosts, AC11's
scenario, refused at zero network cost.

Both conditions keep their own **remediable** downgrade reasons (§6) — they are now
emitted from classification rather than from prediction. The AC6 container fixture's base
image and package set are stated rather than left as "Node absent from `PATH`", so the
lane proves the positive path rather than accidentally exercising one of these failures.

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
domain crate's pup rule (`cli/pup.ron:253`) permits only `std`/`core`/`alloc`,
`kernel::Error` and `crate`, so every observation arrives through a port implemented in
`design-adapters`. The Alpine container fixture confirms wiring but cannot on its own
distinguish "detected musl" from "failed for some other reason", which is why the unit
test carries the property. And the probe runs **before** any artifact resolution, so an
unsupported host downgrades at zero network cost.

The new adapter joins the enforced set rather than slipping outside it:
`design_adapters_read_in_process` (`cli/pup.ron:274`) matches only
`^design_adapters::(filesystem|environment)($|::)` today, so a `platform` module
performing filesystem reads would silently not be held to the discipline its siblings are.
It performs no spawn, so it is added to that match.

#### 5. `design.browser_path`

**Files**: `cli/config/src/catalogue.rs`, `cli/config/src/level.rs`,
`scripts/config-defaults.sh`, `cli/launcher/tests/fixtures/dump/dump.golden`,
`docs-site/…/design.md`

🔒 **The key is readable from the Personal level only.** `.accelerator/config.md` is
repo-tracked — `cli/config/src/level.rs:19` shows only `config.local.md` is the personal,
gitignored level — and §2 passes the resolved value straight into
`chromium.launch({ executablePath })`. Left at team level, opening an untrusted repository
and running the inventory skill executes a binary that repository chose. `visualiser.editor`
sets that precedent, but it extends it to a path executed *automatically* by a skill whose
whole purpose is being pointed at unfamiliar projects, so inheriting the precedent is not
a reason to ship the hazard. A team-level `design.browser_path` is therefore ignored with a
warning naming the personal-level route, and as a second barrier a value canonicalising
inside the repository being inventoried is refused outright. A precedence test covers
team-set × personal-set × neither. Auditing the `visualiser.*` keys against the same
standard stays a follow-up (Removal sweep §5), since those are a pre-existing hazard rather
than one this plan introduces.

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
env-beats-config shape, whitespace collapse included, and copying logic while leaving its
tests at the original site is how two copies drift — this precedence is the mechanism AC12
rests on.

But it cannot be lifted as-is. `resolve_optional` returns `Result<_, ComposeError>`, and
`ComposeError` (`compose.rs:20-25`) wraps `ConfigError` **and**
`work_item_pattern::PatternError`, a visualiser-only concern; meanwhile the natural home is
ruled out two sentences earlier by `config-adapters`' one-env-var rule.

**So it splits rather than moves, because the whole helper cannot live in either crate.**
The function's first statement is `env_nonempty(env_var)` — an environment read — and
`cli/config` is declared pure domain by its own architecture rule (`cli/pup.ron:40-44`:
"The whole config crate is domain (no adapter modules live in it)") with no production
`std::env` read anywhere in it today. Moving the helper wholesale would make it the first
domain crate in the workspace to read the process environment, and the pup rule would not
catch it, since `std` is permitted — so the erosion would land silently and the next
contributor would reasonably conclude env reads are fine in domain crates.

The split:

- **The precedence itself** — an `Option<&str>` from the environment and an
  `Option<&str>` from configuration in, one `Option<String>` out, whitespace-only treated
  as absent — is a pure function in `cli/config`, named for what it encodes rather than
  `resolve_optional`, which says nothing about environment-beats-configuration.
- **The environment read** stays in each composition root: `design-cli` and the
  visualiser's `compose.rs`, mirroring how `plugin_root_from_env` is already called
  explicitly by the roots that need it rather than reached for from inside the domain.

The visualiser's callers are retargeted at the shared function and its tests move with it,
so there is one implementation and one test site. Tests cover env set/unset × config
set/unset × whitespace-only against the pure function, where they need no environment
manipulation and cannot race other tests.

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
`bootstrap-failed`; add:

| Reason | Decided by | Arises when | Remediable |
|---|---|---|---|
| `unsupported-platform` | pre-fetch probe | musl libc (AC11) | no — code-only is the answer |
| `loader-unresolvable` | probe **or** spawn errno | psABI loader absent or relocated (NixOS) | `nix-ld` or `design.browser_path` |
| `glibc-too-old` | bootstrap log | ``version `GLIBC_2.34' not found`` | upgrade the distribution |
| `runtime-libraries-missing` | bootstrap log | `error while loading shared libraries: <soname>` | install the package providing that soname |
| `artifact-unavailable` | materialisation | failed, cause unmapped or persistent | `accelerator cache repair` |
| `materialisation-in-progress` | single-flight waiter | expired; another process is fetching | none — retries on the next invocation |

The **Decided by** column is load-bearing, not documentation. Only the first two are
reachable before a fetch; the middle two are classified from the loader's own output in the
bootstrap log, read through the `BootstrapDiagnostics` port §4 adds, because no observation
this binary can make predicts them. `loader-unresolvable` appears twice because the probe
catches the absent-loader case cheaply while the **spawn errno** — `execve` yielding
`ENOENT` for a program that exists — catches the relocated one the probe's fail-open policy
deliberately lets through. It is an errno rather than a shell message: the executor spawns
with no shell, so `cannot execute: required file not found` can never appear.

`glibc-too-old` and `runtime-libraries-missing` are **sticky**, keyed to the resolved tree's
digest rather than to a materialisation attempt — they are host properties, and a successful
`ensure` must not clear them (§4).

The last four are the ones an earlier draft did not carry. `loader-unresolvable` cannot
share `unsupported-platform`'s entry because its remediation is not "use a different
distribution"; `glibc-too-old` and `runtime-libraries-missing` are the two conditions that
otherwise fetch ~294MB and then fail opaquely (§4); and `materialisation-in-progress` must
be distinct because it is the one cause that must **not** be sticky (§3).

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
- **`evals/benchmark.json`** — ⚠️ **twenty-one** stale references, not the fifteen an
  earlier draft totalled (its individual citations were right; the arithmetic was not),
  and **fifteen** of them are already stale *today* rather than being made stale by this
  phase. Six name `validate-source.sh` (`:1738`, `:1743`, `:1781`, `:1786`, `:1824`,
  `:1829`) and nine name `run.sh` (`:1877`, `:1915`, `:1953`, `:1986`, `:1991`, `:2024`,
  `:2029`, `:2062`, `:2067`) — both deleted by the sibling, which updated `evals.json` and
  missed this file. Three more name `ensure-playwright.sh` (`:1981`, `:2019`, `:2057`) and
  three name `bootstrap-failed` (`:1867`, `:1905`, `:1943`), which this phase retires.

  **A standing guard lands with the correction, not just the correction.** Nothing in CI
  catches these: `scripts/test-skill-frontmatter-conformance.sh:569-574` asserts only that
  the file exists and is valid JSON, never that its content names a live command — which
  is how fifteen references rotted through the immediately preceding plan unnoticed.
  Fixing twenty-one strings without a guard simply resets the counter. So the conformance
  suite gains an assertion that every script path and every downgrade-reason token named
  in `evals.json`, `benchmark.json` and `PROTOCOL.md` resolves to a file that exists or a
  live vocabulary entry — the same shape as the existing "Design script references
  resolve" guard (`conformance:619-664`) — and the criterion is phrased against that guard
  rather than against a count.

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

- [x] Failing test first: *a musl-classified host downgrades to code-only rather than
      resolving a browser path* — red before `design::runtime::availability` exists, over
      injected platform, runtime and browser resolution, so the ADR-0062 ordering is
      pinned in a fast test rather than only in a container
- [x] The platform classification returns the right answer for every injected shape —
      **macOS**, Debian, Debian + `musl-tools`, Alpine, Alpine + `gcompat` (which must
      refuse despite a present glibc loader), a relocated-loader host such as NixOS (which
      must emit `loader-unresolvable`, not the libc reason), and a **distroless image with
      no `/bin/sh` at all** (the `Unobservable` case, which must fail open rather than
      panic or be conflated with either answer)
- [x] Spawn-failure classification is a **pure function over recorded bootstrap-log
      fixtures**, not a live host: `version \`GLIBC_2.34' not found` → `glibc-too-old` with
      the version extracted; `error while loading shared libraries: <soname>` →
      `runtime-libraries-missing` **naming that soname**; an unrecognised failure →
      `executor-ping-failed` rather than a guess
- [x] `loader-unresolvable`'s post-fetch arm is driven by the `Spawner`'s raw `io::ErrorKind`
      — program present but `execve` yielding `ENOENT` — **not** by a shell message the
      executor can never produce, since it spawns with no shell
- [x] 🔒 A bootstrap log containing a marker substring embedded in unrelated output does
      **not** classify, and an over-long or metacharacter-bearing soname is rejected rather
      than reaching the remediation string — so an untrusted project cannot force a
      code-only downgrade or inject text into an agent-facing envelope
- [ ] `glibc-too-old` and `runtime-libraries-missing` are **sticky**, keyed to the resolved
      tree's digest and cleared only by `cache repair` or a digest change — a successful
      `ensure` must not clear them, since materialisation succeeded and the host is what
      failed
- [ ] The probe issues no HTTP request and reads no manifest, so an Alpine host refuses at
      zero network cost — and `ArtifactPlatformEntry` carries no shared-library list, since
      the warm path never loads the manifest that would have delivered it
- [x] An unsupported platform downgrades without issuing any HTTP request
- [ ] A non-executor design subcommand performs no tree resolution and no fetch on an
      empty cache
- [ ] `acquire` runs only on tree-consuming dispatches: `accelerator vcs guard` and a
      SessionStart hook touch no tree state and export no tree variable
- [ ] The tree variables are set from the **compiled-in artifact set**, and an injected
      `ACCELERATOR_TREE_<NAME>` for an unmaterialised tree is **cleared** — including on
      the `ACCELERATOR_DESIGN_BIN` override path, where the resolve path short-circuits
      before any clearing code on it could run
- [x] With no tree variables set (the `ACCELERATOR_DESIGN_BIN` override path), the
      executor reaches `cache ensure` rather than failing, discovering the launcher through
      the documented fallback order when `ACCELERATOR_LAUNCHER_PATH` is unset
- [x] No exported variable collides with `derive_override_var`'s output for any reserved
      or dispatched token
- [ ] `ensure` runs **before** the executor takes `launcher.lock`, so a first-run download
      does not block concurrent invocations behind a lock reporting
      `another-launcher-running`
- [x] `ensure`'s distinct failure causes map to distinct downgrade reasons, and
      `materialisation-in-progress` is **not** among the sticky ones
- [ ] The container harness is a named deliverable with its own Changes Required section —
      image definitions, an invoke task, a `main.yml` job and a workflow-shape test pinning
      it — serving trees from a container-reachable HTTP fixture, not the production release
      host, so the criteria run pre-merge on the change that introduces them rather than
      only after a signed release exists. The lane follows the
      `test:e2e:visualiser:docker` precedent: its own task, excluded from the default
      roll-up with the exclusion asserted in `tests/unit/tasks/test_mise.py`, preflight
      **failing rather than skipping** in CI, and per-architecture image tags
- [ ] ⚠️ **Two fixture classes, and each criterion states which it uses.** A few-KB fake
      `node` cannot run `daemon.js` and a fake headless shell cannot be launched by
      `playwright-core`, so a single miniature fixture cannot serve this lane:
      - **Miniature test-key-signed trees** for everything that does not execute the
        runtime — resolution, attestation, field and layout checks, pointer validation,
        the reaper, the downgrade ordering, and AC11 (which refuses before any resolution).
        Fast, and the bulk of the lane.
      - **One real tree per lane platform**, assembled by Phase 2's own
        `build.assemble_tree_artifacts` on the CI host and signed with the test key, for
        AC6, AC12, the `ping` regression and the relocated `lib/*.test.js` suites — every
        criterion that requires a runtime to actually start. It is one platform, not four,
        because the lane runs on its own host; it reuses the assembly path rather than
        adding machinery; and it is cached between runs, with the ~294MB build cost stated
        in the lane's time budget rather than discovered
- [ ] A container fixture with Node absent from `PATH` fetches both artifacts, launches
      the headless shell, and emits the envelopes the sibling pinned (AC6) — with the
      image's base and package set stated, since Chromium's shared-library set is a
      precondition rather than an incidental
- [ ] The executor spawns the **driver tree's** Node binary, not a `PATH` lookup: with no
      `node` on `PATH` the daemon still starts, and the `NODE` constant no longer appears
      as a spawn program
- [ ] `daemon.js` resolves `playwright-core` from the driver tree by absolute path — a
      bare-specifier import fails on a host whose ancestor `node_modules` lacks it, proving
      the resolution does not rely on `NODE_PATH`, which ESM ignores
- [ ] A musl/Alpine container fixture emits `unsupported-platform` and completes via the
      code-only crawler with a non-error exit — and does so with `design.browser_path`
      both set and unset (AC11)
- [ ] On a glibc host with the bundled browser unavailable and `design.browser_path`
      pointing at a system Chromium, the runtime crawler runs against that executable
      (AC12)
- [ ] `--crawler runtime` hard-fails on an unsupported platform
- [ ] Each artifact downloads at most once per platform per version (AC9)
- [x] `chromium` is a defined export of the module `daemon.js` resolves
- [ ] `daemon.js` launches with an explicit `executablePath`, and the value it receives
      is the one Rust resolved — asserted for both the bundled tree and the
      `design.browser_path` hatch, since AC12 depends on it
- [ ] `ping` succeeds when `playwright-core`'s registry path does not exist, proving the
      handler checks the launch path rather than `executablePath()` — the regression that
      would silently degrade every crawl to code-only
- [ ] A launch succeeds against a read-only browsers root, proving an explicit
      `executablePath` bypasses registry validation and writes
- [x] The precedence helper is a **pure function** over two `Option<&str>` values in
      `cli/config`, tested over env set/unset × config set/unset × whitespace-only with no
      environment manipulation, with the visualiser's callers retargeted at it and its
      tests moved with it
- [x] `cli/config` contains no production `std::env` read — the environment read stays in
      each composition root, so the domain crate's purity (`cli/pup.ron:40-44`) is not
      eroded by a change the pup rule cannot catch
- [ ] 🔒 A **team-level** `design.browser_path` is ignored with a warning, and a value
      canonicalising inside the repository being inventoried is refused — so an untrusted
      repository cannot name the binary the inventory skill executes
- [x] `design notices` has a success path and a failure path, including `--artifact`,
      over a fixture tree
- [ ] With `design.browser_path` set, **zero** browser archive fetches occur — the hatch
      short-circuits browser materialisation while the driver `ensure` still runs
- [ ] A persistent materialisation failure produces **one** fetch attempt per session,
      not one per executor invocation
- [ ] A successful `ensure` clears the sticky marker, `cache repair` clears it (so the
      remediation named in the envelope is actually effective), and a marker past its TTL
      does not suppress the next crawl — the TTL exercised over the injected `Clock`
      rather than by sleeping
- [ ] 🔒 A pre-planted marker file, a marker path that is a symlink, and a marker not owned
      by the effective uid are all refused rather than honoured
- [ ] A free-space shortfall emits `disk-floor-not-met` before any fetch starts, and an
      unwritable cache root emits `cache-unwritable`
- [ ] Tree-failure envelopes name `accelerator cache repair <name>`
- [x] The downgrade goldens stay exhaustive by construction across the vocabulary change
      — a variant with no golden fails, and an orphan golden fails
- [x] A **standing conformance guard** asserts that every script path and downgrade-reason
      token named in `evals.json`, `benchmark.json` and `PROTOCOL.md` resolves to an
      existing file or a live vocabulary entry — so the twenty-one stale references are
      not merely corrected but cannot recur, which is what happened to fifteen of them
      during the preceding plan. Eval 20 passes against `artifact-unavailable`
      (**amendment**: the reason half is a retired-reason denylist derived against the
      live `key()` arms, since a prose reference — "the literal `<reason>` message" —
      cannot be anchored positively; `REASONS_EVER` is append-only so a future retirement
      is caught without editing the guard)
- [x] `cli/design/tests/fixtures/public-api.txt` is regenerated and committed
- [x] `mise run test:unit:design-automation` passes with the **case** floor moved to the
      new TAP-reported total; the suite floor stays at **9**, since the loader survives in
      narrowed form rather than being deleted
- [x] `mise run test:integration:design-automation` still fails rather than skips when no
      runtime is available, with its preflight resolving the driver tree rather than the
      deleted namespace
- [ ] Step 1a's fetch deadline has been **re-derived** from Phase 2's measured archive
      sizes and no longer carries Phase 1's interim value — the cross-phase handoff is
      asserted rather than assumed
- [ ] `mise run cli:check` exits 0
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] A full inventory crawl on a machine with no system Node produces the same artefacts
      as one on a machine with Node installed
- [ ] First-run download of **both** trees completes within a stated wall-clock ceiling at
      the stated minimum throughput (the same floor Step 1a's deadline encodes), with host
      and connection recorded — a pass/fail bound, not an observation. The ceiling is
      derived from the concurrent `ensure driver browser` shape, so it is the max of the
      two transfers rather than their sum
- [ ] `accelerator design notices` reaches all three licence sets
- [ ] Deleting one file from a sealed tree, then running `accelerator cache repair`,
      restores a working crawl

---

## Removal sweep

### Overview

The residue this plan owns: the floor arithmetic, the acceptance of the two ADR
supersessions work-item:0214 raised (ADR-0061 and ADR-0062), the documentation of the
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

`test-design.sh` is not in `_REQUIRED_CONFIG_SUITES` (`:67`), so no by-name gate moves.

#### 2. Documentation

**Files**: the `docs-site/src/content/docs/` pages describing the Playwright
prerequisite, `README.md`, `CHANGELOG.md`, `.claude-plugin/plugin.json`
**Changes**: `plugin.json:11` declares the `Node >= 20` requirement this plan removes —
it goes. Every page describing the bootstrap step, the lockhash namespace or the disk and
node-version floors is repointed at the vendored artifacts, the `cache` verbs and the
`design.browser_path` hatch. The `design` docs page the sibling created gains the
artifact and cache sections; `ACCELERATOR_CACHE_DIR` is documented as
**trust-relevant** — it must be a private, user-owned path — not merely as a
longer-lived location, and as requiring a **local filesystem**: `flock` is unreliable on
NFS/SMB/FUSE (Step 1b §2) and an unresponsive network mount can block the hit path in the
kernel.

Three further documentation items this plan's mechanisms create:

- **`design.browser_path` is a personal-level key** and why (Phase 3 §5), so nobody adds
  it to a shared `.accelerator/config.md` expecting it to work.
- **`ACCELERATOR_RELEASE_BASE_URL` is the mirror hatch**, with its limitation stated: the
  redirect allowlist is `github.com` plus `*.githubusercontent.com`
  (`fetcher.rs:17-18, 31-33`), matched at a dotted-label boundary, so a mirror must serve
  bytes inline rather than 3xx to another host. That mattered little for 8MB single-file
  binaries; at ~1.2GB per release it is the difference between a locked-down network being
  able to use the design tooling and not. The allowlist is therefore **derived from the
  configured base URL's host** when an override is set, defaulting to the current GitHub
  pair otherwise, with a criterion covering a tree resolved from an overridden base URL
  that redirects within its own host.
- **Ephemeral environments** — CI agents, devcontainers and Codespaces have no persistent
  cache, so the first design command of every session fetches ~294MB or fails. What this
  plan offers them is documentation only: the recommended `ACCELERATOR_CACHE_DIR`
  placement and cache-mount key so the cost is paid once per cache generation rather than
  once per session.

  An earlier revision proposed `accelerator cache ensure --from <path-or-url>` here as an
  offline populate route. **It is cut and raised as a follow-up instead** (§5), because it
  is a second ingestion path into a trust boundary the rest of Phase 1 spends a whole step
  establishing, and specifying it in a documentation bullet would get it implemented
  without the extraction rules, size bounds, cause mapping and adversarial coverage the
  fetched path receives. Its verification contract also needs real design rather than
  "the same checks as a fetched one": verifying against the *manifest* costs two HTTPS
  GETs, which is unavailable in exactly the disconnected case the flag exists for, so an
  offline route has to be anchored on the attestation alone and the operator has to stage
  three files rather than one.

#### 3. ADR and work-item amendments

**Files**: `meta/work/0196-accelerator-design-inventory-gap-tooling-cli.md`
**Changes**: The ADR work this plan owed is **done** — two supersessions, one new
decision (all three accepted), and now the superseding attestation ADR: **ADR-0064**
(producer-signed tree attestation over a compiled-in digest) supersedes ADR-0061 on the
attestation shape and the pointer key, ADR-0061 has transitioned to `superseded`, and
work-item:0196's addressing/prerequisite text is corrected.

- **ADR-0061** (signed content-addressed tree generations) supersedes ADR-0060. It
  records content-based addressing with a per-release pointer (which this plan supersedes
  in favour of a digest-keyed one — see below), the cross-version tree
  **adoption** that follows on a shared root and the layout version that makes it safe,
  the signed attestation under the embedded release key (0.17% of the warm budget), and
  the shared `flock` lease that ADR-0060's repair story assumed without naming — as a
  **sidecar beside** each generation, which this plan follows: inside the sealed
  directory the lease would be read-only for the dispatches that must take it and would
  appear to `verify` as an entry absent from the `.files` table.

  ⚠️ **A superseding ADR is a deliverable of this sweep, on two counts.** First, ADR-0061
  describes the attestation as carrying the manifest's signature over the archive digest.
  That is not implementable — `tasks/signing.py` signs the archive *file's bytes* and the
  launcher deletes the archive after extraction, so nothing remains for that signature to
  verify against — and a signature over a bare digest would not bind artifact identity or
  platform anyway. Second, and more substantially, ADR-0061 records addressing "with a
  per-release pointer", which this plan replaces with a **digest-keyed** pointer plus a
  **compiled-in expected digest** as the rollback defence.

  That second change is the load-bearing one, so the superseding ADR should record why:
  a per-release pointer accumulates one entry per plugin version on a shared root (making
  `prune` unable to reclaim anything), forces a manifest load before the reuse scan can
  discover an already-present tree (so a zero-byte upgrade fails offline), and — if the
  release version is *signed*, as an intermediate revision of this plan proposed — makes
  cross-version adoption impossible, since the older generation's document names the older
  version. Digest keying gives rollback refusal, cross-version adoption and offline
  resolution from one mechanism, and it needs no field the producer cannot know. The ADR
  also records the `.files` table moving inside the archive, so its integrity follows from
  the archive signature rather than from a locally-written digest.
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

**Changes**: Three this plan surfaces and does not fix. None exists yet — 0206, 0207,
0208 and 0209 cover other things — so raising them is a deliverable of this sweep rather
than a note:

- **Advisory-feed monitoring for the pinned runtime.** The stale-pin half of this is
  **no longer a follow-up**: Phase 2 §2 records `playwright-core`, Node and
  `CHROMIUM_REVISION` in `RELEASING.md` as security-relevant dependencies with an owner
  and a maximum age, and adds the scheduled guard that opens an issue when one is
  exceeded. That much is cheap and the exposure — a full browser engine shipped to every
  user, exempt from per-exec re-verification, with §8's reuse path skipping
  fetch-and-verify while pins hold — is too large to leave entirely unwatched. What
  remains for the follow-up is the harder part: cross-referencing the pinned revisions
  against an advisory feed, since `cargo-deny` covers Rust crates only.
- 🔒 **The `visualiser.*` config keys have the hazard `design.browser_path` no longer
  has.** Phase 3 §5 now restricts `design.browser_path` to the personal (gitignored)
  level and refuses a value resolving inside the inventoried repository, so this plan
  does not ship the repo-supplied-executable path. But `visualiser.editor` — the
  precedent it was modelled on — remains settable from repo-tracked
  `.accelerator/config.md`, and the follow-up audits it and its siblings against the same
  standard.
- **An offline populate route.** Cut from §2 above rather than shipped under-specified.
  The follow-up designs `cache ensure --from` properly: local path only or the same host
  allowlist, the `.sealed` and `.sealed.sig` staged alongside the archive so verification
  is anchored without the manifest, the same entry rules and size bounds read from the
  attestation, a cause mapped into the downgrade vocabulary, and criteria for a mismatched
  digest, a wrong-artifact or wrong-platform attestation and an unsigned archive.
- **Bounding the default cache root.** ADR-0063 delegates eviction there to Claude Code's
  ~14-day orphan sweep, and `cache prune` now reports the footprint across sibling
  plugin-version roots so the growth is at least visible — but a user tracking
  prereleases still accumulates ~294MB per platform per upgrade for up to a fortnight
  with no ceiling. The follow-up decides whether `prune` should reclaim there by default
  and whether an upgrade should adopt the previous version's trees rather than
  re-materialising.

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

- [x] Failing test first for the final-state assertion
- [x] `mise run test:integration:config` passes with `_EXPECTED_CONFIG_SUITES` moved to
      **14** and `test-design.sh` absent from the discovered suites
- [x] Both design-script conformance guards still pass with both design skills carrying
      no `scripts/*` grant and no script-shaped call site
- [x] `mise run test:unit:build-system` passes (the task is `test:unit:tasks`; the plan's
      name does not exist)
- [x] `mise run lint:scripts:exec-bits:check` exits 0
- [ ] `mise run docs:check` exits 0
- [x] **No `.sh` file remains under `skills/design/`**
- [ ] `git status --porcelain -uall` is clean after a tree materialisation in a dev
      checkout, so the trees directory under the cache root is genuinely ignored
- [ ] `mise run` exits 0

#### Manual Verification

- [ ] The docs site builds and every design page's links resolve
- [ ] A fresh plugin install with no system Node completes an inventory run
- [x] work-item:0196 no longer describes a scheme the code does not implement (ADR-0061,
      ADR-0062 and ADR-0063 are already accepted; the attestation/pointer correction is
      ADR-0064)
- [x] A superseding ADR records the attestation document's shape and the tuple it binds,
      replacing ADR-0061's "manifest signature over the archive digest" — **ADR-0064**,
      binding `{artifact, platform, archive_sha256, uncompressed_size, entry_count,
      table_sha256}`, digest-keyed pointer, compiled-in expected digest
- [x] Work-item:0208 records which of the two lanes owns the CI job

---

## Testing Strategy

### Unit Tests

- **Entry classification** as a pure function in `launch::core`, table-driven over the
  committed adversarial corpus — the full rejection set including PAX/GNU long-name
  records and duplicate-path entries, which is where tar CVEs live. The same corpus drives
  the Python CI-side extractor, so the two implementations of one allowlist cannot drift.
- Tree materialisation in `cli/launcher/` against synthetic tarballs: rejection before
  extraction, attestation signature and field checks (including an untrusted keypair and
  each field mismatch), layout-version refusal and re-materialisation, table-digest
  binding, a crash injected at each step of the publish sequence through the `after_step`
  seam, single-flight with a failing winner, pointer validation, `verify`'s detection of
  each corruption shape including a rewritten `.files` row, and repair's new-generation
  swap against a live reader. The unsigned cases exercise the tree modules directly with
  **no signing step**; the signature and attestation cases do take
  `skip_if_no_minisign!`, which is why `resolution.rs:255-265` is changed to fail closed
  under `CI` rather than returning `Ok(())` with an `eprintln!`.
- Time-dependent behaviour — the reaper's age backstop, the waiter's bound, the sticky
  marker's TTL — over the injected `Clock`, never by sleeping or back-dating mtimes.
- Platform classification in `cli/design/src/runtime/platform.rs` over injected
  observations — exactly two: the shell interpreter's basename (three-valued, including
  `Unobservable`) and whether the demanded psABI interpreter is executable. Seven shapes
  are pinned without a container: macOS, Debian, Debian + `musl-tools`, Alpine, Alpine +
  `gcompat` (musl must win over a present glibc loader), a relocated-loader host, and a
  distroless image with no `/bin/sh`. So AC11's musl case, the Mac case and the gcompat
  ordering all hold in a fast test, and the spike's own prototype tests transfer directly.
- **Spawn-failure classification** over recorded loader stderr fixtures, which is where
  `glibc-too-old` and `runtime-libraries-missing` are decided — they are not probe
  observations, because neither is answerable from a static-musl binary (§4). Each fixture
  pins its reason and, for the library case, that the *actual* missing soname reaches the
  envelope rather than a guessed package list.
- The **failure-ordering state machine** in `design::runtime::availability` over injected
  platform, runtime and browser resolution, so ADR-0062's ordering and the
  `design.browser_path` short-circuit are pinned without a process.
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
  (`resolution.rs:199-213`) gains a tree-`acquire` case asserting zero and a tree-
  `materialise` case asserting one, so work-item:0189's guarantee still has teeth on the
  new path. Concurrency tests must not rely on it across threads — `PROBE_ATTEMPTS` is a
  `thread_local!` (`cache_root.rs:74-75`).
- The assembly path against a **miniature fixture triple** (a few-KB fake `node`, a fake
  headless shell, a synthetic `browsers.json`) in `test:unit:build-system`: determinism
  with each normalisation independently asserted, the `ASSEMBLED_SHA256` match, the reuse
  fallback, `NOTICES/` population, and the smoke and structural predicates — each with a
  paired negative asserting the release fails. This is what keeps Phase 2's gates from
  being first exercised inside the release job after `_publish` has pushed.
- An assembly round trip: a synthetic tree through the real assembly path, a manifest and
  a signed attestation through the real emission path, signed with a test key, resolved by
  the launcher's tree resolver — so the two halves of the artifact contract are verified
  together rather than only by hand.
- Container fixtures: Node-absent glibc (AC6), musl/Alpine (AC11), and
  bundled-browser-unavailable with `design.browser_path` set (AC12). The harness is a
  **named Phase 3 deliverable** — image definitions, invoke task, `main.yml` job and a
  workflow-shape test — because the launcher's `MockServer` is a `#[cfg(test)]` type bound
  to loopback, reachable neither from a container nor from an invoke task. It serves both
  fixture classes over a container-reachable HTTP fixture rather than the production
  release host, so the lane runs pre-merge on the change that introduces it rather than
  only after a signed release exists: **miniature test-key-signed trees** for the
  non-executing majority, and **one real locally-assembled tree** for the criteria that
  must actually start a runtime (AC6, AC12, `ping`, and the relocated `lib/*.test.js`
  suites below). AC11 needs neither, since the platform probe refuses before any
  resolution. It follows the `test:e2e:visualiser:docker` precedent for roll-up membership
  (own task, excluded with the exclusion asserted in `tests/unit/tasks/test_mise.py`,
  `docker info`-style preflight that fails rather than skips in CI), and names
  per-architecture image tags.
- The retained `lib/*.test.js` suites plus `test-run.js` and `daemon-runtime.test.js`,
  moved into that container lane where a runtime exists and zero skips can be asserted
  across the whole set — subject to the 0208 coordination the Removal sweep §6 records.

### Manual Testing Steps

1. Time a warm executor invocation before and after Phase 1 against a launcher built at
   the merge base, using work-item:0205's interleaved-medians method at n = 300, and
   confirm the Hodges–Lehmann shift's upper 95% bound is below 1.0ms.
2. Corrupt a file in a sealed tree, run `accelerator cache verify`, then `repair`, and
   confirm a working crawl.
3. Run a full inventory crawl on a machine with no system Node and compare artefacts
   against one with Node installed.
4. Point `ACCELERATOR_CACHE_DIR` at a group-writable location and confirm `trees/` is
   refused with a message naming the exact `chmod`; then at a `0775` user-private-group
   directory and confirm it **resolves**, since the launcher owns `trees/`'s mode.
5. Walk the re-drivable finalise procedure once against a preserved draft, so the
   documented recovery from a job timeout is known to work before it is needed.

## Performance Considerations

Three budgets are load-bearing.

**The warm path.** Work-item:0186 took warm bootstrap from 125ms to ~30ms. Per-exec
re-verification of a 294MB artifact set would spend 16–33 seconds per crawl re-hashing
immutable bytes, which is why ADR-0060 exempts trees. The hit path is therefore local
reads plus `lstat`s, one Ed25519 verify and one `flock` per tree; it loads no manifest and
**probes no cache root**, which is what keeps a populated cache working offline, and the
per-entry file table is deliberately not on it, so its cost does not scale with an
artifact's file count.

⏱️ **The budget is stated over the whole `acquire` sequence, not over the verify alone.**
The measured 51.7µs (0.17% of the warm bootstrap) covers one Ed25519 verify over a
few-hundred-byte document; `acquire` also performs roughly a dozen syscalls, three file
opens and a lock per tree, doubled across both trees, none of which that figure includes.
Since the Phase 1 gate is an absolute 1.0ms and a regression blocks the phase, the full
sequence is what gets measured — and the same 1.0ms is not double-counted against the
binary-size ceiling, which is derived from the bootstrap's own O(size) hash pass.

Two properties keep that bounded. The variable names come from the **compiled-in artifact
set**, never a `readdir` of `trees/` — pointers are version-keyed and accumulate, so a
scan against a long-lived shared cache root would grow with every plugin version ever
installed, turning a fixed-cost path into O(releases). And `acquire` runs only on
**tree-consuming dispatches**, so `accelerator vcs guard` (a PreToolUse hook firing on
every tool call) and every SessionStart hook pay nothing and inherit no tree-subsystem
failure surface — which also means an unresponsive relocated cache root cannot wedge the
session.

**Launcher binary size.** `cli/Cargo.toml:183-185` records that the bootstrap hashes the
whole launcher on every invocation, so binary size is a per-call latency term and the
cold-fetch payload. It sets no threshold, so Step 1b §1 derives one — from a **measured**
per-MB slope, not from work-item:0186's non-method-comparable figure — and enforces the
*delta* in the PR lane rather than only an absolute per-target ceiling at release time.

**First run.** ~294MB per platform, and resumable so a link slow enough to outlast one
crawl still converges.

⏱️ **Two `ensure` operations run concurrently, but that buys the latency term only — not
the bytes.** Overlapping converts sum into max for connection setup, TLS handshake and
TTFB; the ~240MB of compressed payload is bandwidth-bound, so two streams share the link
and the wall clock is close to the sum. The stated first-run ceiling is derived on that
basis, and the per-attempt deadline is sized for the **per-stream** share of the
throughput floor rather than the whole link — otherwise a healthy 200 KB/s connection
times out at precisely the floor the deadline was written to accommodate, three times, and
lands in a sticky `artifact-unavailable`. A single shared `Fetcher` serves both
materialisations so the rustls install, background runtime thread and connection pool are
paid once.

On the default cache root — inside the versioned plugin tree — a plugin upgrade discards
it, and this plugin pre-releases often. `ACCELERATOR_CACHE_DIR` is the escape, and
digest-keyed addressing is what makes it work: the driver and browser change only when the
pinned `playwright-core` changes, so an upgrade that leaves the pin alone looks for the
same digest, finds the same pointer, and hits — with no manifest load, so it also hits
offline.

**The release job.** It runs the whole pipeline twice per stable release on a
`macos-latest` runner with no `timeout-minutes` today. The dominant new cost is **serial
transfer**, not assembly: `upload_and_verify_release` uploads and re-verifies through
per-asset subprocess loops, so the tree artifacts add ~480MB up and ~480MB back down per
pass. §8's reuse removes the re-assembly CPU — the smaller term — so Phase 2 also makes
upload and re-verification bounded-parallel, has the pre.0 pass reuse the stable pass's
local copies, adds a `timeout-minutes` sized from a measured double-pass run, and adds a
whole-job disk assertion against a staging tree whose extracted Chromium alone is ~700MB
across four platforms.

## Migration Notes

Existing installs carry a populated
`${ACCELERATOR_PLAYWRIGHT_CACHE:-$HOME/.cache/accelerator/playwright}/<sha8>` namespace
that nothing will read after Phase 3. It lives outside the plugin tree so plugin pruning
will not reclaim it. Phase 3's documentation names the path and states it is safe to
delete; no automated removal is added, consistent with not building destructive-op UX
where the filesystem makes recovery trivial. `accelerator cache prune` reports it with
its measured size and the exact command, so the reclamation is discoverable at the moment
a user is already thinking about cache space.

⚠️ **That report is guarded, because the tool is authoring a destructive command for a
path outside version control.** `ACCELERATOR_PLAYWRIGHT_CACHE` is an environment variable
nothing else reads or validates after Phase 3, so a stale or over-broad value — pointing
at `$HOME/.cache`, say — would yield a ready-to-paste `rm -rf` for the wrong directory.
`prune` therefore emits the path and command **only** when the directory actually matches
the legacy `<sha8>` namespace layout, refuses to name any path that is not a leaf under a
recognised accelerator cache directory, and otherwise reports the size alone and points at
the documentation.

`SKILL.md` Step 4's `ensure-playwright.sh` bootstrap and its
`ACCELERATOR_DOWNGRADE_REASON=` stderr protocol are replaced in Phase 3 §6, together with
the `allowed-tools` grant that keeps them reachable and the conformance assertion that
pins the grant. All three edits land in one change, because either of the two guards the
sibling's validation added fires on any subset.

## References

- Work item: `meta/work/0196-accelerator-design-inventory-gap-tooling-cli.md`
- **Blocking spike**: `meta/work/0214-settle-the-vendored-runtime-tree-artifact-mechanisms.md`
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
