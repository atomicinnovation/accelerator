---
type: work-item
id: "0196"
title: "accelerator-design: Design Inventory and Gap Tooling CLI"
date: "2026-08-05T19:03:35+00:00"
author: Toby Clemson
producer: review-work-item
status: ready
kind: story
priority: medium
parent: "work-item:0136"
derived_from: ["work-item:0173"]
tags: [rust, design, cli, playwright, distribution]
last_updated: "2026-08-09T23:37:39+00:00"
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
  below). The Playwright executor
  (`run.sh`, currently under `inventory-design/scripts/playwright/`) remains
  a thin wrapper that launches the bundled driver as a subprocess daemon —
  together with the `lib/*.js` modules it depends on (see Technical Notes)
  — per the decision recorded in Context and Drafting Notes (2026-08-08)
  and the bundling requirements that follow.
- Rewrite the call sites and `allowed-tools` of every skill under
  `skills/design/**` to call the new `accelerator design` subcommands,
  following the invocation contract established in work-item:0167.
- The release pipeline that publishes `manifest.json` gains a new step to
  fetch Microsoft's per-platform driver bundle and re-sign it under the
  project's own minisign key before publication — preserving the same
  manifest/signature trust story as existing single-file binaries (resolved
  2026-08-09; see Drafting Notes) rather than referencing Microsoft's CDN
  directly.
- Extend the CLI's fetch-verify-cache mechanism
  (`cli/launcher/src/launch/outbound/resolve/{mod,manifest,fetcher,verifier,keys,cache,cache_root}.rs`,
  currently `manifest.json` + minisign signing for single-file
  `accelerator-<token>-<platform>` binaries) to support fetching, verifying,
  and caching a runtime-plus-package-tree artifact — Microsoft's official
  per-platform Playwright driver bundle (a prebuilt Node.js binary +
  `playwright-core`) — reusing the same manifest/signature-verification
  trust story rather than introducing a parallel distribution mechanism.
- `accelerator-design`'s Playwright executor launches browser automation via
  the fetched bundled driver (its Node binary + `playwright-core`'s CLI/driver
  entrypoint), removing the system Node.js ≥20 prerequisite currently
  enforced by `ensure-playwright.sh`
  (`skills/design/inventory-design/scripts/ensure-playwright.sh`).
- Chromium browser binary installation (currently `npx playwright install
  chromium`, cached under
  `${ACCELERATOR_PLAYWRIGHT_CACHE:-$HOME/.cache/accelerator/playwright}/<lockfile-hash>`)
  is driven through the bundled driver's own CLI entrypoint rather than
  `npx`, since `npm`/`npx` are no longer assumed present on the host either.

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
      launches the bundled driver and exits 0, producing a report artefact
      that is byte-identical to the one the current shell invocation produces
      for a fixed fixture input. Restructuring the report format is out of
      scope for this item; if a future need to restructure it arises, it is
      tracked as a separate follow-up item.
- [ ] **AC3.** All skills previously invoking `skills/design/**/scripts/*` now call the
      corresponding `accelerator design` subcommand, with `allowed-tools`
      updated to match, per the work-item:0167 contract.
- [ ] **AC4.** The migrated `skills/design/**` scripts are removed (excepting `run.sh`
      and the `lib/*.js` modules it depends on, retained as the thin-wrapper
      executor per Technical Notes), with the affected suite floors (the
      minimum
      test-count thresholds enforced by CI per
      `tasks/README.md#registering-a-dispatched-sub-binary`) decremented in
      lockstep (see work-item:0174).
- [ ] **AC5.** `accelerator-design` passes every item of the sub-binary registration
      checklist at `tasks/README.md#registering-a-dispatched-sub-binary`.
- [ ] **AC6.** On a machine with no system Node.js installed (verified in CI or a
      container fixture with Node absent from `PATH`), the
      `inventory-design` subcommand's Playwright-driven path fetches the
      bundled driver on first run, launches Chromium, and produces a report
      artefact byte-identical to the fixed-fixture output required by AC2.
- [ ] **AC7.** The bundled driver artifact is sha256- and minisign-verified and cached
      following the same trust model as existing sub-binary fetches
      (`manifest.json` + `.minisig`, embedded public key) — no unverified
      binary is executed.
- [ ] **AC8.** The release pipeline's driver-bundle re-signing step (`tasks/release.py`;
      see Requirements) runs successfully for every platform
      `accelerator-design` supports and produces a `.minisig` for each that
      passes the same CLI-side sha256/minisign verification exercised by the
      criterion above.
- [ ] **AC9.** Bundle download and browser-binary download each happen at most once
      per platform per version (cache hit on subsequent runs), matching the
      idempotency of today's `ensure-playwright.sh` sentinel behaviour.
- [ ] **AC10.** The bundled driver's Playwright version is verified compatible with
      the version pinned in
      `skills/design/inventory-design/scripts/playwright/package.json`
      (currently `~1.55.1`) that the retained `lib/*.js` automation code
      depends on — checked automatically at build or CI time, per whichever
      synchronisation mechanism is recorded resolving the versioning Open
      Question below.

## Open Questions

- Manifest schema shape for a runtime-plus-package-tree artifact — does the
  existing `Manifest.binaries: BTreeMap<String, BinaryEntry>` shape need a
  new `BinaryEntry` variant, or a parallel map alongside it? A design
  decision for implementation time.
- Versioning: how is the bundled driver's Playwright version kept in sync
  with the `playwright` version pinned in
  `skills/design/inventory-design/scripts/playwright/package.json`
  (currently `~1.55.1`) so the driver and the retained `lib/*.js`
  automation code (see Technical Notes) stay compatible?

## Dependencies

- Blocked by: confirmation that the release-artifact hosting
  infrastructure serving `manifest.json` and its binaries can accommodate
  the ~117-118MB-per-platform driver bundle (see Technical Notes precedent
  sizes) across every platform `accelerator-design` supports — several
  hundred MB in aggregate, a substantial addition to whatever currently
  hosts single-file binaries. Must be confirmed before the release-pipeline
  requirement ships. Prior blockers are resolved: work-item:0166 (shared
  crates, done), work-item:0167 (invocation-contract pattern, done —
  subsumes the earlier launcher/dispatch scaffold), work-item:0187
  (sub-binary registration surface, merged via PR #42).
- Blocks: work-item:0174 (shell/CI-guard retirement — floor decrements from
  this item's script removals feed its lockstep requirement).
- External: Chromium download is still required at first run (~150MB,
  cached); the Node.js + Playwright driver dependency is now vendored via
  the CLI's fetch-verify-cache mechanism rather than assumed present on the
  host — resolution of the manifest schema shape is tracked in Open
  Questions above; the release-pipeline publishing design is resolved (see
  Requirements and Drafting Notes).
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
- The report artefact format is fully deterministic — free of timestamps,
  absolute paths, or non-deterministic ordering — across the legacy shell
  invocation and the new Rust-launched subprocess invocation, so the
  byte-identical comparisons required by AC2 and AC6 are meaningful
  pass/fail tests rather than a source of false-negative failures.

## Technical Notes

- Source bash: `skills/design/**/scripts/*` (`inventory-design`,
  `analyse-design-gaps`), including `run.sh` (confirmed at
  `inventory-design/scripts/playwright/run.sh` — the Playwright executor
  belongs to the `inventory-design` subcommand, not `analyse-design-gaps`).
- The Playwright executor's implementation is not limited to `run.sh`:
  `inventory-design/scripts/playwright/lib/*.js` (`auth-header.js`,
  `client.js`, `daemon.js`, `errors.js`, `identity.js`, `lock.js`,
  `mask.js`, `path-guard.js`, `playwright-loader.js`, `state.js`) are the
  JS modules `run.js` (invoked by `run.sh`) depends on for daemon
  lifecycle, request auth, secret masking, path guarding, and Playwright
  loading. These carry the same thin-wrapper/subprocess-daemon disposition
  as `run.sh` itself — not migrated into the Rust binary, and excepted
  from the script removal in Acceptance Criteria alongside `run.sh`.
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

## References

- Split from: `meta/work/0173-remaining-subdomains-corpus-design-collaboration.md`
  (abandoned)
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0048, ADR-0053
- Playwright license (Apache-2.0):
  https://github.com/microsoft/playwright/blob/main/LICENSE
- playwright-python driver-bundling precedent (`setup.py` /
  `scripts/build_driver.py`):
  https://github.com/microsoft/playwright-python
- playwright-java driver-bundle artifact size discussion:
  https://github.com/microsoft/playwright-java/issues/1196
- Existing fetch-verify-cache implementation:
  `cli/launcher/src/launch/outbound/resolve/`
