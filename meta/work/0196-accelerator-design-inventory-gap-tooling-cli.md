---
type: work-item
id: "0196"
title: "accelerator-design: Design Inventory and Gap Tooling CLI"
date: "2026-08-05T19:03:35+00:00"
author: Toby Clemson
producer: review-work-item
status: in-progress
kind: story
priority: medium
parent: "work-item:0136"
derived_from: ["work-item:0173"]
tags: [rust, design, cli, playwright, distribution]
last_updated: "2026-08-20T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0196: accelerator-design: Design Inventory and Gap Tooling CLI

**Kind**: Story
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

Migrate the design inventory/gap tooling (`inventory-design`,
`analyse-design-gaps`) into an `accelerator-design` sub-binary, bundling
Microsoft's official per-platform Playwright driver (Node.js +
`playwright-core`) as a distributed artifact so the tooling no longer
requires a system-installed Node.js runtime. The bundled driver is fetched,
verified, and cached via an extension of the CLI's existing fetch-verify-cache
mechanism (`manifest.json` + minisign), generalised to support
runtime-plus-package-tree artifacts rather than single-file binaries.

## Context

Split out of work-item:0173 (now abandoned) on 2026-08-05, per that item's
review-1 scope finding: bundling `accelerator-corpus`, `accelerator-design`, and
`accelerator-collaboration` into a single story risked partial-completion
ambiguity and an oversized PR. 0173 also stated the Playwright-executor's fate
inconsistently — as a hedged either/or in Requirements, and separately as an
unresolved Open Question — a clarity finding this item originally resolved by
stating the choice once, in Open Questions. The 2026-08-08 re-scope below
superseded that placement: the settled decision now lives in Requirements
(see Drafting Notes), and Open Questions no longer addresses the
executor's disposition at all.

Research into Playwright's driver architecture (2026-08-08) established that
Playwright's non-JS bindings (Python, Java, .NET) avoid a system Node.js
dependency by bundling Microsoft's own per-platform driver bundle — a zip
containing a prebuilt Node.js binary and `playwright-core` — directly inside
their distributed package. True in-process (no subprocess) Playwright
automation from Rust does not exist anywhere: every unofficial
`playwright-rust` crate still spawns the bundled Node driver as a child
process, architecturally identical to today's `run.sh` → `node run.js`
daemon. This item adopts the proven bundled-driver approach: the Playwright
executor remains a subprocess launch, but the Node+driver dependency itself
is now vendored and fetched via the CLI's own distribution pipeline instead
of assumed present on the host.

## Requirements

- `accelerator-design` — design inventory/gap tooling for `skills/design/**`
  maintainers and consumers, per the sub-binary consistency established by
  parent epic work-item:0136 (`inventory-design/scripts/*`,
  `analyse-design-gaps/scripts/*`; the subcommand set is whatever these two
  script directories resolve to — the concrete mapping must be recorded in
  Drafting Notes before implementation begins, per AC1's precondition
  below). The Playwright executor's **bash launcher** (`run.sh`, currently
  under `inventory-design/scripts/playwright/`) is **reproduced in Rust** and
  removed: `accelerator design` spawns the bundled Node runtime directly, so
  the delegation chain is CLI → Node with no shell in between (ADR-0058). The
  JavaScript automation it drives (`run.js` and the `lib/*.js` modules — see
  Technical Notes) is retained as-is, since `playwright-core` is a JavaScript
  library and must run in Node.
- Rewrite the call sites and `allowed-tools` of every skill under
  `skills/design/**` to call the new `accelerator design` subcommands,
  following the invocation contract established in work-item:0167.
- Per ADR-0059 the release pipeline **assembles** the driver bundle rather than
  re-signing Microsoft's prebuilt one, which carries no upstream signature. It
  fetches `playwright-core` from npm and the paired Node runtime from
  `nodejs.org/dist`, verifies each against its publisher's own primitive at
  build time (npm registry signature plus SLSA attestation; the GPG signature
  on `SHASUMS256.txt`), assembles the per-platform tree, and publishes it under
  the project's own minisign key as a build artifact of ours.
- Also per ADR-0059 the Chromium build is fetched and pinned at build time, at
  the revision named by the vendored `playwright-core`'s `browsers.json`, and
  published as a **separate** per-platform manifest artifact rather than fused
  into the driver bundle. Its bytes are pinned by hash but its chain of custody
  is TLS-only — upstream publishes no signature for it. `ffmpeg` is excluded:
  `browsers.json` marks it install-by-default but it serves video recording.
- The vendored `playwright-core` version is the exact version the retained
  automation declares. `package.json`'s `~1.55.1` range is tightened to a single
  version so the fetched package, the API `lib/*.js` was written against, and
  the derived Chromium revision are one choice.
- Extend the CLI's fetch-verify-cache mechanism
  (`cli/launcher/src/launch/outbound/resolve/{mod,manifest,fetcher,verifier,keys,cache,cache_root}.rs`,
  currently `manifest.json` + minisign signing for single-file
  `accelerator-<token>-<platform>` binaries) to fetch, verify, and cache
  directory-tree artifacts — the assembled driver bundle and the browser —
  reusing the same manifest/signature-verification trust story rather than
  introducing a parallel distribution mechanism. The CLI gains no new
  verification primitive; all upstream-signature checking lives in the
  pipeline.
- Per ADR-0060 that extension takes a specific shape. Tree artifacts
  are carried in a **new top-level `artifacts` map** in `manifest.json`,
  additive under `schema_version: 1` so no other sub-binary sees a flag day,
  and `binaries` keeps meaning one key, one executable. The **launcher**
  resolves them, so the embedded signing key keeps a single holder. Digest and
  signature are checked over the fetched archive **before extraction**;
  extraction lands in a temp directory that is **renamed into place in one
  syscall**; and the tree is then **sealed read-only** with a sentinel
  recording the verified digest and release version, placed beside the tree
  rather than inside it. Tree entries are addressed by release version and
  digest, so an upgrade materialises a new tree rather than mutating one.
- Also per ADR-0060, tree artifacts are **exempt from per-exec
  re-verification**. Single-file sub-binaries keep theirs unchanged; a warm
  browser command checks the sentinel instead of re-hashing hundreds of
  megabytes, which is what keeps a crawl inside its budget. Because automatic
  self-healing goes with it, a **user-invocable repair path** (re-verify and
  refetch) is required to restore recovery from a corrupt or partial tree.
- `accelerator-design`'s Playwright executor launches browser automation via
  the assembled driver (its Node binary + `playwright-core`'s CLI/driver
  entrypoint), removing the system Node.js ≥20 prerequisite that
  `ensure-playwright.sh` formerly enforced — that script is now deleted, and
  `plugin.json` no longer declares the requirement.
- No browser download happens on the user's machine. `npx playwright install
  chromium` went with `ensure-playwright.sh`, along with its `<lockfile-hash>`
  namespace under the old `ACCELERATOR_PLAYWRIGHT_CACHE` root — the
  `package-lock.json` it hashed is deleted too.
- Tree artifacts cache under the launcher's existing plugin-root-scoped cache
  root (`resolve/cache_root.rs`, `ACCELERATOR_CACHE_DIR` override, no XDG
  fallback because an XDG-resident binary would break the plugin-root
  `allowed-tools` glob match). Since that root is inside the versioned plugin
  tree, artifacts are scoped per plugin version and are pruned when Claude Code
  prunes old plugin versions — no bespoke eviction logic. The cost is that a
  plugin upgrade discards them: at roughly 117MB of driver plus 177MB of
  headless shell, each upgrade refetches ~294MB per platform, and this plugin
  pre-releases often. `ACCELERATOR_CACHE_DIR` is the escape for anyone who
  wants a longer-lived location (a **trust-relevant** one — private, user-owned,
  on a local filesystem), and because ADR-0061 addresses tree entries by content
  digest — not by release version, superseding ADR-0060's version-and-digest
  scheme — sharing them across plugin versions is what the addressing already
  gives, needing no redesign (ADR-0063 records the per-plugin-version cache
  root).
- The browser artifact is `chromium-headless-shell`, not full Chromium. The
  daemon launches headless (`lib/daemon.js:106`), and the shell is 177MB across
  14 files against 297MB across 327 — materially cheaper to host, fetch,
  extract and seal. Implementation note: `lib/daemon.js:120-125` resolves
  `executablePath()` and reports `chromium-not-found` against the full Chromium
  path, so that diagnostic needs revisiting. Revisit the choice if a rendered
  inventory proves to need the full browser's fidelity.
- Browser automation is scoped to Playwright's own supported platforms per
  ADR-0057; on a host outside that matrix the Playwright path emits the
  existing structured downgrade reason and the default and hybrid crawler
  modes fall back to the code-only crawler rather than failing. An explicit
  `--crawler runtime` request still hard-fails.
- A system-browser escape hatch: a `design.browser_path` configuration key
  (one-shot override `ACCELERATOR_DESIGN_BROWSER_PATH`; precedence env >
  config) names an existing Chromium-family executable to drive instead of
  the bundled browser download, giving air-gapped installs, sandboxed
  networks, and distro-packaged-Chromium users a supported route to the
  runtime crawler. Per ADR-0057 the hatch substitutes the browser only — the
  vendored driver's Node binary stays glibc-linked, so it does not bring the
  runtime crawler to musl hosts.
- Artifact metadata is not reimplemented in the design domain. `accelerator
  corpus metadata derive` already emits the same four lines with the same
  labels in the same order, and `FilenameTimestampFormat::CompactTime` already
  renders `inventory-metadata.sh`'s exact `%Y-%m-%d-%H%M%S` shape. The action
  gains a `--filename-timestamp-format` flag to expose the variant, the
  `inventory-design` skill calls corpus directly for its frontmatter
  provenance, and `inventory-metadata.sh` is deleted with no design-side
  replacement. **Scope note**: this touches `cli/corpus-cli`, outside the
  design subdomain this item otherwise owns.
- Redistribution notices ship with the artifacts: each component's
  `LICENSE`/`NOTICE` (Node and its bundled dependencies, `playwright-core`,
  and Chromium's credits) is assembled into the artifact set and reachable by
  a user. No formal legal review gates the release — see Drafting Notes for the
  position taken and its basis.

## Acceptance Criteria

- [ ] **AC1.** `accelerator design …` reproduces the inventory/gap behaviours, verified
      against repointed suites (existing tests redirected to invoke the new
      binary instead of the legacy shell scripts) and characterization tests
      (tests that pin down current behaviour rather than a separately
      specified behaviour) where none exist — each covering at least the
      primary success path and one failure path per subcommand in the set
      recorded in Drafting Notes (see Requirements). Recording that mapping
      in Drafting Notes is a precondition of this criterion — it must happen
      before implementation begins, not merely "once known."
- [ ] **AC2.** Invoking the `inventory-design` subcommand's Playwright-driven path
      launches the driver and exits 0, and each subcommand's stdout/stderr
      envelope is byte-identical to a golden fixture for a fixed fixture input.
      The volatile inputs (clock, VCS facts, ephemeral port, absolute paths)
      are supplied through injected ports so the output is deterministic by
      construction rather than by normalisation. The model-authored report is
      **not** in scope: no script in `skills/design/**` produces it today, so
      it is not part of the binary's contract. Any screenshot assertion covers
      count, dimensions and non-emptiness rather than bytes, which do not
      reproduce across runs. Restructuring the report format remains out of
      scope for this item.
- [ ] **AC3.** All skills previously invoking `skills/design/**/scripts/*` now call the
      corresponding `accelerator design` subcommand, with `allowed-tools`
      updated to match, per the work-item:0167 contract.
- [ ] **AC4.** The migrated `skills/design/**` scripts are removed — including
      `run.sh`, whose launcher behaviour is reproduced in Rust, and its bash
      suite `test-run.sh`. The JavaScript automation (`run.js`, `lib/*.js`
      and their Node test suites) is retained per Technical Notes. Affected
      suite floors (the minimum
      test-count thresholds enforced by CI per
      `tasks/README.md#registering-a-dispatched-sub-binary`) decremented in
      lockstep (see work-item:0174).
- [ ] **AC5.** `accelerator-design` passes every item of the sub-binary registration
      checklist at `tasks/README.md#registering-a-dispatched-sub-binary`.
- [ ] **AC6.** On a machine with no system Node.js installed (verified in CI or a
      container fixture with Node absent from `PATH`), the
      `inventory-design` subcommand's Playwright-driven path fetches the driver
      bundle and the browser artifact on first run, launches the headless
      shell, and emits the same byte-identical envelopes AC2 pins for that
      fixture.
- [ ] **AC7.** The driver-bundle and browser artifacts are each sha256- and
      minisign-verified against `manifest.json` with the embedded public key
      **before extraction**, extracted to a temp directory, renamed into place
      in a single syscall, and sealed read-only with a sentinel beside the tree
      — so no unverified byte is ever written into a materialised tree and a
      concurrent invocation never observes a partial one. Per ADR-0060 tree
      artifacts are then **exempt** from the per-exec re-verification
      single-file sub-binaries keep; verification happens at materialisation,
      not per invocation.
- [ ] **AC8.** The release pipeline's assembly step (`tasks/release.py`; see
      Requirements) runs successfully for every platform `accelerator-design`
      supports, producing a driver-bundle artifact and a browser artifact per
      platform each with a `.minisig` that passes the same CLI-side
      sha256/minisign verification exercised by the criterion above. Per
      ADR-0059.
- [ ] **AC9.** Driver-bundle download and browser download each happen at most once
      per platform per version (cache hit on subsequent runs), provided by the
      launcher's sealed-tree cache-hit sentinel.
- [ ] **AC10.** The assembly step fails the release if the `playwright-core` it
      fetched is not the exact version declared in
      `skills/design/inventory-design/scripts/playwright/package.json`, or if
      the Chromium revision it fetched is not the one that package's
      `browsers.json` names. Per ADR-0059 the pairing is structural, so this
      criterion guards the construction rather than testing compatibility after
      the fact.
- [ ] **AC11.** On a host outside Playwright's supported platform matrix (verified
      with a musl/Alpine container fixture), `accelerator design`'s inventory
      subcommand emits the structured downgrade reason and completes via the
      code-only crawler with a non-error exit, rather than failing — and does
      so whether or not `design.browser_path` is set, since the hatch cannot
      recover the glibc-linked driver on musl. Per ADR-0057.
- [ ] **AC12.** On a supported (glibc) host with the bundled browser download
      unavailable and `design.browser_path` pointing at a system Chromium, the
      runtime crawler runs against that executable instead of downgrading. Per
      ADR-0057.
- [ ] **AC13.** The assembly step verifies each upstream input before use and fails
      the release otherwise: `playwright-core` against its npm registry
      signature and SLSA provenance attestation, the Node runtime against the
      GPG signature on `SHASUMS256.txt`. Per ADR-0059.
- [ ] **AC14.** A user-invocable repair path re-verifies and refetches a tree
      artifact, restoring the recovery that automatic per-exec self-healing
      provided. Verified against a deliberately corrupted and a deliberately
      truncated tree, each of which the repair returns to a working state. Per
      ADR-0060.
- [ ] **AC15.** `corpus metadata derive` accepts `--filename-timestamp-format`, and
      the `compact-time` variant reproduces `inventory-metadata.sh`'s output
      byte-for-byte for a fixed clock and VCS fixture. The `inventory-design`
      skill calls it for frontmatter provenance and the script is deleted.
- [ ] **AC16.** Each distributed artifact carries the redistribution notices for
      what it contains — Node and its bundled dependencies, `playwright-core`,
      and Chromium's credits — reachable by a user without unpacking the
      artifact by hand.

## Open Questions

None. The layout question — the assembled bundle ships `playwright-core`,
whereas `run.sh` hard-checks for a `node_modules/playwright/` layout and
`playwright-loader.js` throws rather than falling back — was closed during
planning on 2026-08-11 in favour of retargeting `lib/*.js` at `playwright-core`
directly, matching Microsoft's own bindings. See Drafting Notes. (The *version*
half was already closed by ADR-0059: the vendored core is the exact version
`package.json` declares, and AC10 guards it.)

## Dependencies

- Blocked by: nothing. The release-artifact hosting capacity coupling —
  whether the infrastructure serving `manifest.json` and its binaries can
  accommodate **both** per-platform tree artifacts ADR-0059 introduces, the
  ~117-118MB driver bundle (see Technical Notes precedent sizes) plus the
  177MB headless shell, across every platform `accelerator-design` supports,
  roughly 294MB per platform and about 1.2GB per release — was **confirmed on
  2026-08-11**. The real assembled sizes are still measured during the
  release-pipeline work, as a recorded figure rather than a gate. Prior
  blockers are likewise resolved: work-item:0166 (shared
  crates, done), work-item:0167 (invocation-contract pattern, done —
  subsumes the earlier launcher/dispatch scaffold), work-item:0187
  (sub-binary registration surface, merged via PR #42).
- Blocks: work-item:0174 (shell/CI-guard retirement — floor decrements from
  this item's script removals feed its lockstep requirement).
- External: no third-party host is contacted at first run. Per ADR-0059 the
  Node runtime, `playwright-core` and Chromium are all fetched, verified and
  assembled in CI, then served from our own release host through the CLI's
  fetch-verify-cache mechanism. The build-time dependencies are
  `registry.npmjs.org`, `nodejs.org/dist` and `cdn.playwright.dev`, plus a
  maintained set of Node release keys. The manifest schema shape and the
  release-pipeline publishing design are both resolved (ADR-0059, ADR-0060;
  see Requirements and Drafting Notes).
- Coordination: this item now touches shared launcher infrastructure
  (`cli/launcher/src/launch/outbound/resolve/`) and the shared release
  pipeline (`tasks/release.py`), not only `accelerator-design` — flag to
  reviewers since it crosses sub-binary boundaries into code shared by
  every dispatched sub-binary (visualiser, vcs, work, corpus). Siblings
  work-item:0195 (corpus) and work-item:0197 (collaboration) register
  sub-binaries via the same checklist around the same time; coordinate to
  avoid merge contention on the fetch-verify-cache mechanism and the
  release pipeline specifically, not only the registration checklist. No
  fixed merge order is mandated across the three; owners should sync
  before merging any change to `cli/launcher/src/launch/outbound/resolve/`
  or `tasks/release.py` to avoid conflicting extensions to the manifest
  schema or the signing step.
- Parent: work-item:0136 (epic).

## Assumptions

- Repointed suites plus characterization tests where none exist are
  sufficient to establish behavioural parity with the legacy shell scripts.
- Redistributing Microsoft's official per-platform Playwright driver bundle
  (Playwright: Apache-2.0; bundled Node.js binary: MIT) as part of
  `accelerator-design`'s own distribution artifact is permitted. This is
  inferred from the permissive terms of both licenses; no explicit Microsoft
  statement blessing third-party redistribution of the bundle outside its
  own npm/PyPI/Maven/NuGet packages was found.
- ~~The report artefact format is fully deterministic — free of timestamps,
  absolute paths, or non-deterministic ordering — across the legacy shell
  invocation and the new Rust-launched subprocess invocation, so the
  byte-identical comparisons required by AC2 and AC6 are meaningful
  pass/fail tests rather than a source of false-negative failures.~~
  **Falsified 2026-08-10 by codebase research.** The report body is authored
  by the model (SKILL.md Step 9 "Synthesise"), not emitted by any script, and
  its frontmatter carries timestamps, VCS revisions, ephemeral localhost
  ports, absolute paths, and a filesystem-derived `sequence`, under a
  wall-clock-bounded crawl. Two runs of the current shell pipeline are not
  byte-identical to each other. AC2 and AC6 need retargeting — see Open
  Questions.

## Technical Notes

- Source bash: `skills/design/**/scripts/*` (`inventory-design`,
  `analyse-design-gaps`), including `run.sh` (confirmed at
  `inventory-design/scripts/playwright/run.sh` — the Playwright executor
  belongs to the `inventory-design` subcommand, not `analyse-design-gaps`).
- The Playwright executor's implementation is not limited to `run.sh`:
  `inventory-design/scripts/playwright/lib/*.js` (`auth-header.js`,
  `client.js`, `daemon.js`, `errors.js`, `identity.js`, `lock.js`,
  `mask.js`, `path-guard.js`, `playwright-loader.js`, `state.js`) are the
  JS modules `run.js` depends on for daemon lifecycle, request auth, secret
  masking, path guarding, and Playwright loading. These are **retained**
  (~1,726 lines including their Node test suites) and excepted from the
  script removal — they are the Playwright automation itself and must run in
  Node. `run.sh` is **not** in that exception: it is a launcher, and its
  behaviour moves into Rust.
- Scope of the `run.sh` → Rust port (203 lines): process start-time identity
  (`/proc/<pid>/stat` field 20 plus `/proc/stat` `btime` on Linux;
  `ps -p <pid> -o lstart=` parsed under `LANG=C` on Darwin) with the ±1s
  tolerance; the double-checked reuse short-circuit around a `flock`-or-
  `mkdir` lock (honouring `ACCELERATOR_LOCK_FORCE_MKDIR`); `server.pid` /
  `server-info.json` parsing; state-dir creation at mode 0700; the
  Playwright-namespace root and its layout precondition (today exit 3
  `playwright-not-installed`); daemon spawn and the 30s start poll with
  kill-on-timeout; and the `ACCELERATOR_PLAYWRIGHT_STATE_DIR` / `NODE_PATH` /
  `ACCELERATOR_PLAYWRIGHT_NS_ROOT` environment handed to the child.
  The port **removes** runtime dependencies on `jq`, `flock`, `sha256sum`/
  `shasum`, `nohup`, `sed`/`awk`/`tr`/`date`, and the bash 3.2 floor on this
  path, and replaces the re-entrant
  `accelerator config path tmp` shell-out with an in-process call. Prefer the
  repo's existing mkdir-lock sentinel contract over a new lock scheme.
- The Rust side must preserve the start-time identity contract the daemon
  writes (`lib/state.js` `processStartSeconds`, computed under `LANG=C`); a
  mismatch silently respawns the daemon between commands and loses page
  state. Computing the probe natively in Rust avoids the locale hazard on the
  reader side, but the two values must still agree numerically.
- Removing `run.sh` collapses the executor-path coupling: the
  `accelerator:browser-executor` skill, `scripts/config-read-browser-executor.sh`,
  and the `{browser-executor-script}` convention in `agents/browser-locator.md`
  and `agents/browser-analyser.md` exist solely to resolve `run.sh`'s absolute
  path. They should be retired in favour of the browser agents invoking
  `accelerator design` directly, like every other migrated skill.
- Current install/cache flow: `ensure-playwright.sh` hashes
  `playwright/package-lock.json` (sha256, first 8 chars) to a namespace
  `${ACCELERATOR_PLAYWRIGHT_CACHE:-$HOME/.cache/accelerator/playwright}/<sha8>`,
  runs `npm ci --ignore-scripts --no-fund` then `npx playwright install
  chromium` into that namespace, and writes a sentinel
  (`lockhash`/`node_version`/`playwright_version`/`completed_at`) once
  complete. It requires system Node ≥20; `run.sh` refuses to proceed (exit 3,
  `playwright-not-installed`) if that cache is absent.
- Fetch-verify-cache precedent:
  `cli/launcher/src/launch/outbound/resolve/{mod,manifest,fetcher,verifier,keys,cache,cache_root}.rs`.
  `fetch_verify_store(name: &str)` already takes an arbitrary token and looks
  it up in `Manifest.binaries: BTreeMap<String, BinaryEntry>`, verifying
  sha256 + minisign against an embedded public key before atomically storing
  the result — but `BinaryEntry` and the `accelerator-<token>-<platform>`
  asset-name convention assume a single-file binary, not a runtime+package
  tree. Extending it is new design work, not a drop-in reuse.
- Reference bundling approach: `microsoft/playwright-python`'s `setup.py` /
  `scripts/build_driver.py` bakes the platform driver into the wheel at
  build time (~118MB per platform); Java's `driver-bundle` Maven artifact
  does the same (~117MB for v1.29.0). No official binding fetches the bundle
  lazily at first run except the unofficial/community Go binding.
- No ADR or `tasks/README.md` section currently addresses bundling a
  non-Rust runtime as a distributed artifact — ADR-0048 documents Node.js as
  dev-time tooling (frontend build, `actionlint`, markdown/CI) provisioned
  via `mise`, not an end-user-distributed runtime.

## Drafting Notes

- Split out of work-item:0173 on 2026-08-05 following that item's review-1
  (verdict REVISE, scope lens): the three sub-binaries it bundled were
  functionally independent and separately deliverable.
- The Playwright-executor either/or was previously stated twice, inconsistently,
  in 0173 (a review-1 clarity finding); here it appeared once, in Open
  Questions, until 2026-08-08's re-scope resolved it entirely (see below).
- review-1 (2026-08-06, verdict REVISE) found the either/or had recurred
  within this item itself — the Summary stated the decision as settled while
  Requirements/Open Questions/AC still hedged it, and the two outcomes were
  not equivalent effort. Resolved at the time by: hedging the Summary
  consistently, removing the restated either/or from Requirements, and
  adding a default (thin wrapper) plus a pre-implementation confirmation
  gate to Open Questions.
- 2026-08-08: re-scoped following deep research into Playwright's driver
  architecture (see Context). The original either/or (thin wrapper vs. fold
  into Rust) is now resolved as: the executor stays a subprocess launch — no
  in-process alternative to full-fidelity Playwright exists anywhere — but
  the Node+driver dependency itself is now vendored via the CLI's own
  distribution pipeline rather than assumed present on the host. This is a
  materially larger scope than the item originally anticipated.
- The user explicitly chose to keep this work inside 0196 rather than split
  it into a dependent item, despite the scope now touching the CLI's shared
  launcher distribution mechanism (`cli/launcher/...`) rather than being
  confined to `accelerator-design` alone. Flagging for reviewer attention
  given this crosses sub-binary boundaries into infrastructure shared by
  every dispatched sub-binary.
- Rationale for not splitting (review-2, 2026-08-09): the manifest schema
  extension (a new `BinaryEntry` variant vs. a parallel map) is hard to
  validate in the abstract — `accelerator-design` is the first and only
  concrete consumer of the runtime-plus-package-tree shape, so designing
  and validating the schema against a real consumer's requirements in the
  same change reduces the risk of shipping an extension that doesn't fit
  the actual need. This mirrors how the existing single-file `BinaryEntry`
  shape was itself designed against concrete sub-binary consumers rather
  than speculatively.
- 2026-08-09 (review-2): resolved the driver-bundle-publishing Open
  Question — the release pipeline gains a new step to fetch Microsoft's
  per-platform driver bundle and re-sign it under the project's own
  minisign key before publishing it in `manifest.json`, rather than
  referencing Microsoft's CDN directly. This preserves AC7's "same trust
  model" requirement without qualification; see the new Requirements
  bullet.
- Chose Microsoft's official driver bundle over hand-rolling a separate
  Node.js binary fetch + npm install of `playwright` — the proven approach
  already used by playwright-python/java/.NET, avoiding an unproven
  in-house mechanism.
- Chose to extend the existing fetch-verify-cache mechanism over building a
  parallel dedicated one — keeps one consistent distribution/trust story
  across all fetched artifacts, at the cost of new design work on the
  manifest schema (see Open Questions).
- 2026-08-09 (review-2, pass 2): closed three findings surfaced by
  resolving the driver-bundle-publishing Open Question — added an
  Acceptance Criterion verifying the release pipeline's re-signing step
  itself (`tasks/release.py`) rather than only the CLI-side consumption of
  its output; extended the Dependencies Coordination entry to name
  `tasks/release.py` alongside the launcher `resolve/` code as a shared
  surface siblings 0195/0197 should sync on; and defined `lib/*.js`
  (confirmed at `inventory-design/scripts/playwright/lib/*.js`) in
  Technical Notes, stating explicitly that these modules share `run.sh`'s
  thin-wrapper disposition and are excepted from script removal.
- 2026-08-09 (review-2, pass 3): closed two more findings — updated
  Context's closing sentence, which still pointed to Open Questions for
  the Playwright-executor decision after the 2026-08-08 re-scope moved it
  to Requirements; and added an Acceptance Criterion requiring the bundled
  driver's Playwright version to be verified compatible with the
  `package.json` pin, closing the gap where no criterion tested the
  versioning Open Question's outcome.
- 2026-08-09 (review-2, pass 4): closed two more findings — numbered the
  Acceptance Criteria (AC1-AC10) so existing back-references elsewhere in
  the document ("AC2", "AC6", "AC7") resolve against a visible label
  rather than an implicit checkbox position; and added a Dependencies
  entry naming the release-artifact hosting infrastructure's capacity for
  the ~117-118MB-per-platform driver bundle as a coupling to confirm
  before the release-pipeline requirement ships.
- 2026-08-10 (review-2, pass 5): closed the final two findings — fixed a
  latent reference bug the AC numbering exposed but didn't cause: AC6's
  "the criterion above" pointed at AC5 (added between AC2 and AC6 across
  earlier passes) instead of AC2, now stated explicitly; Requirements'
  matching "per the Acceptance Criteria precondition below" now says
  "per AC1's precondition" for the same reason. Also moved the pass-4
  hosting-capacity note into the "Blocked by" entry itself rather than a
  separate untagged bullet, so it no longer sits beside a "Blocked by:
  none currently" claim it contradicts. This closes the review-2 loop;
  remaining findings (licensing-as-assumption, launcher-infrastructure
  bundling, kind-as-story, and a tail of minor/suggestion polish) stay
  open by explicit reviewer choice — see review-2's Pass 5 section.

- 2026-08-10 (codebase research + web research): resolved the platform-support
  conflict surfaced by research against ADR-0046. Playwright does **not**
  support musl/Alpine at all — its supported Linux platforms are Debian 12/13
  and Ubuntu 22.04/24.04/26.04, its Chromium builds are glibc-linked, and the
  driver bundle has no musl variant (published for `mac`, `mac-arm64`,
  `linux`, `linux-arm64`, `win32_x64`, `win32_arm64`; the Linux builds are
  manylinux). The constraint therefore already binds today, since
  `ensure-playwright.sh` runs `npx playwright install chromium` and downloads
  the same glibc Chromium — this item inherits the gap rather than creating
  it. Recorded as ADR-0057 (proposed), which scopes browser automation to
  upstream's matrix, keeps the CLI's own static guarantee intact, relies on
  the existing code-only downgrade, and adds the `design.browser_path`
  escape hatch (new Requirements bullets, AC11 and AC12). Building an in-house
  musl driver bundle was rejected on the record: `nodejs/unofficial-builds`
  musl binaries are experimental and need `libstdc++`, and it would deliver a
  working driver with a non-working browser.
- 2026-08-10 (ADR-0057 review): the escape hatch was over-claimed. It
  substitutes the browser, not the runtime, so a musl host still fails on the
  glibc-linked driver Node binary before the browser path is consulted — the
  hatch serves glibc hosts that cannot download a browser. AC11 asserted the
  runtime crawler would run on Alpine with `design.browser_path` set, which
  cannot pass; it now asserts the downgrade holds regardless of the hatch, and
  the browser-path behaviour moved to AC12 on a glibc host. The Requirements
  bullets were corrected in step, including the downgrade's mode-dependence
  (default and hybrid degrade; explicit `--crawler runtime` hard-fails).
- 2026-08-10: the user directed that the shell layer be eradicated rather
  than retained — `run.sh`'s launcher behaviour is reproduced in Rust so the
  chain is CLI → Node, not CLI → shell → Node. This supersedes the
  2026-08-08 re-scope's "thin wrapper" disposition for `run.sh` specifically;
  the `lib/*.js` automation retention is unchanged. The port is strictly
  subtractive — it deletes runtime dependencies on `jq`, `flock`,
  `sha256sum`, `nohup` and the bash 3.2 floor on this path, and removes a
  re-entrant `accelerator config` shell-out — and aligns with ADR-0048's
  stated direction that shell shrinks as logic migrates into the CLI. It
  also lets the `browser-executor` resolver skill and its two agent call
  sites retire. Scope detail in Technical Notes; recorded as ADR-0058
  (proposed) after the ADR-0057 review found it out of scope there.
- 2026-08-10: the user chose to keep the driver bundling inside this item
  rather than splitting it out, on the grounds that vendoring the runtime is
  what makes the tooling self-contained and removes the system Node.js
  dependency. Two conflicts consequently remain live and need their own
  decisions before implementation: the provenance semantics of re-signing a
  third-party artifact under the project's key (including whether SLSA
  attestation should cover it), and the integrity model for a directory-tree
  artifact whose per-exec re-verification is unaffordable at ~117MB against
  the warm-path budget established by work-item:0186. The determinism basis
  for AC2/AC6 is also still open — see Open Questions.
- 2026-08-10 (ADR-0059): the re-signing conflict is closed, and by dissolution
  rather than by answer. Web research established that Microsoft's prebuilt
  driver bundle carries no upstream signature at all — `build_driver.py`
  assembles it from the `playwright-core` npm tarball and a `nodejs.org` binary
  and verifies neither, and `setup.py` checks only that the downloaded zip
  exists. There is no third-party chain of custody for our key to launder, so
  the question became which inputs we can verify and when. ADR-0059 decides we
  assemble in CI from the original distributors, verifying `playwright-core`
  against its npm registry signature and SLSA attestation and Node against the
  GPG signature on `SHASUMS256.txt`; the result is signed as our own artifact
  on the existing manifest/minisign path, and the CLI gains no new trust
  primitive. Chromium is bundled the same way but is pinned rather than
  verified — upstream publishes no signature for it, so its custody is
  TLS-only, and build-time pinning buys blast-radius containment rather than
  provenance. Consequences for this item: the driver bundle and the browser
  become separate per-platform manifest artifacts (which constrains the
  manifest-schema question); the vendored `playwright-core` version is pinned
  exactly to what `package.json` declares, closing the versioning question and
  turning AC10 into a guard on that construction; AC13 was added for
  input verification; `ffmpeg` is excluded; no third-party host is contacted at
  first run; and the licensing assumption is now a redistribution obligation
  for Node, `playwright-core` and Chromium rather than something to note.
  Per-exec re-verification and the AC2/AC6 determinism basis remain open, and
  the cache-key question gained a pruning half.
- 2026-08-11: the four remaining blockers were worked through and decided, and
  the open questions they left behind were closed with them.
  **Tree integrity** (ADR-0060): permissions are the boundary.
  Measurement drove it — hardware sha256 runs at ~2.5GB/s, `chromium-1193` is
  297MB across 327 files and the driver bundle ~117MB, and a crawl bounded at
  50 routes makes 100–200 executor invocations, so per-exec re-verification
  would burn 16–33s per crawl re-hashing immutable bytes to close a window
  against an attacker who already holds the user's privileges. Verification
  moves to materialisation behind an atomic directory rename, the tree is
  sealed read-only, and a repair path (AC14) replaces the self-healing that
  goes with it.
  **Manifest shape** (ADR-0060): a new top-level `artifacts` map,
  launcher-resolved. The feared flag-day was never real — `manifest.rs` ignores
  unknown fields by design and the schema gate rejects only versions above the
  supported one — so the question was only ever where the entries live.
  **Determinism**: AC2/AC6 retarget onto the binary's own envelopes. AC2 was
  aimed at an artefact no script produces — the report is written by the model,
  and what the scripts emit is envelopes plus four lines of clock and VCS
  output. **Metadata**: rather than reimplement those four lines, the skill
  calls `corpus metadata derive`, which already renders the same labels in the
  same order and already has `CompactTime` matching the shell script's exact
  format; only a flag is missing (AC15). **Licensing**: notices ship with the
  artifacts (AC16) and no formal legal review gates the release, on the basis
  that Electron and Playwright redistribute these same components the same way.
  The reviewer's recommendation was a legal gate; the decision was notices
  only, recorded here so the position reads as taken rather than overlooked.
  **Caching**: artifacts live under the launcher's existing plugin-root-scoped
  cache root, which is already where sub-binaries cache and already has no XDG
  fallback (an XDG-resident binary would break the plugin-root `allowed-tools`
  glob). Being inside the versioned plugin tree, they are pruned when Claude
  Code prunes old plugin versions, so no bespoke eviction is needed; the cost
  is a ~294MB per-platform refetch on every plugin upgrade.
  **Browser choice**: `chromium-headless-shell` for now — the daemon launches
  headless and the shell is 177MB across 14 files against 297MB across 327.
  **Layout** stays open by choice, deferred to planning and implementation.
- 2026-08-11 (planning): **the subcommand mapping AC1 requires is recorded
  here**, resolved against the nine scripts codebase research enumerated. The
  set is seven:

  ```
  accelerator design validate-source <location> [--allow-internal] [--allow-insecure-scheme]
  accelerator design resolve-auth
  accelerator design scrub-secrets <file>
  accelerator design notify-downgrade --reason <enum> [--from <mode>] [--to <mode>]
  accelerator design audit-cue-phrases <file>
  accelerator design executor <command> [json-args]
  accelerator design notices [--artifact driver|browser]
  ```

  Four scripts map to no subcommand: `inventory-metadata.sh` and
  `gap-metadata.sh` are deleted in favour of `corpus metadata derive` (AC15);
  `ensure-playwright.sh` is deleted with no replacement, since ADR-0059 moves
  its whole job to build time and only its downgrade vocabulary survives, into
  the executor path and the platform guard; and
  `regenerate-notify-downgrade-fixtures.sh` — a maintainer dev tool invoked by
  no SKILL.md — is deleted with its fixtures, regeneration becoming a test
  affordance on the Rust goldens. `notices` is new rather than ported, and
  exists to make AC16 reachable. AC14's repair path is **not** a design
  subcommand: it lands as the launcher built-in `accelerator cache
  verify|repair`, per ADR-0060's decision that the launcher owns tree
  resolution and holds the signing key alone.
- 2026-08-11 (planning): the remaining open questions were closed so the plan
  carries none. **Layout** — `lib/*.js` is retargeted at `playwright-core`
  directly (route c), matching what Microsoft's own bindings do;
  `playwright-loader.js` and its three fixture trees retire, and the 0072
  property is re-pinned as "`chromium` is a defined export of the resolved
  module". **Exit codes** — `scrub-secrets` and `audit-cue-phrases` split usage
  error onto exit 2, a deliberate behaviour change aligning them with
  `validate-source`, `notify-downgrade` and the `kernel::Error::Refusal`
  mapping every other sub-binary uses. **Downgrade vocabulary** — replaced, not
  retained: `executor-ping-failed` survives, the four reasons that can no
  longer arise plus `bootstrap-failed` are dropped, and
  `unsupported-platform` and `artifact-unavailable` are added. **Archive
  format** — `tar.gz`, flat in `dist/release/`, because `@actions/glob`'s `*`
  does not cross `/` and a nested staging tree would silently miss the
  provenance globs. **Design test suites** — the four bash suites and
  `scripts/test-design.sh` are deleted rather than wired into CI; the eleven
  retained `node --test` suites gain a `test:unit:design-automation` task, so
  AC1 and AC2 have CI-observable meaning. **Hosting capacity** — confirmed by
  the user on 2026-08-11; the plan assumes it and adds a `timeout-minutes` and
  a disk guard to the release job rather than a capacity gate.

## References

- Split from: `meta/work/0173-remaining-subdomains-corpus-design-collaboration.md`
  (abandoned)
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Research: `meta/research/codebase/2026-08-10-0196-accelerator-design-inventory-gap-tooling-cli.md`
- ADRs: ADR-0048, ADR-0053, ADR-0057 (accepted — browser automation as a
  glibc-only capability), ADR-0058 (accepted — shell-free CLI-to-Node
  delegation), ADR-0059 (accepted — build-time assembly of vendored browser
  artifacts), ADR-0060 (accepted — launcher-resolved tree artifacts)
- Playwright license (Apache-2.0):
  https://github.com/microsoft/playwright/blob/main/LICENSE
- playwright-python driver-bundling precedent (`setup.py` /
  `scripts/build_driver.py`):
  https://github.com/microsoft/playwright-python
- playwright-java driver-bundle artifact size discussion:
  https://github.com/microsoft/playwright-java/issues/1196
- Existing fetch-verify-cache implementation:
  `cli/launcher/src/launch/outbound/resolve/`
