---
type: "plan-validation"
id: "2026-08-31-0203-third-party-attribution-artefact-validation"
title: "Validation Report: Third-Party Attribution Artefact Implementation Plan"
date: "2026-09-04T13:24:16+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-08-31-0203-third-party-attribution-artefact"
tags: ["rust", "frontend", "licensing", "release", "vcs"]
last_updated: "2026-09-04T13:24:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Third-Party Attribution Artefact Implementation Plan

Result: **pass**. All three phases are fully implemented and every automated
criterion is green, including the round-trip idempotence AC and the full
read-only CI mirror (`mise run check`, exit 0). Three deviations from the plan
text are recorded below; each is a sound improvement or a user-approved
substitution, not a gap.

### Implementation Status

- ✅ Phase 1: Generators, artefact, drift check — fully implemented (with a
  render-mechanism deviation; see Deviations)
- ✅ Phase 2: Release staging, upload, coverage guards — fully implemented
- ✅ Phase 3: Rationale on work item 0203 — fully implemented

Evidence is four commits on the current history — `nwmwuptt` (phase 1),
`wllrtzvy` (phase 2), `msuqsrtk` (phase 3), and `tsmstryv` (a header
correction). The working copy is empty; nothing is left uncommitted.

### Automated Verification Results

| Check | Command | Status |
| --- | --- | --- |
| Notices update writes the file | `mise run notices:update` | 🟢 passing |
| Round-trip idempotent (clean tree) | `notices:update && notices:check` | 🟢 passing |
| Gate placement | `pytest test_mise.py` | 🟢 passing |
| Fold/render + §3.2 regression | `pytest test_notices.py` | 🟢 passing |
| Upload presence (name-set) | `pytest test_build.py` | 🟢 passing |
| Upload presence (attest-glob) | `pytest test_workflows.py` | 🟢 passing |
| Staging wiring (both lanes) | `pytest test_release.py` | 🟢 passing |
| Frontend bundled-import guard | `pytest tests/unit/frontend` | 🟢 passing |
| cargo-deny after comment edit | `mise run deny:check` | 🟢 passing |
| Aggregate read-only gate | `mise run check` | 🟢 passing |

- ✅ `notices:update` then `notices:check` leaves `jj status` clean — the
  artefact is byte-reproducible, the plan's central generator-reproducibility
  criterion.
- ✅ 144 passed across `test_notices.py`/`test_mise.py`/`test_build.py`/
  `test_workflows.py`; 39 passed in `test_release.py` (integration); 4 passed in
  the frontend guard.
- ✅ `mise run check` exits 0 in ~714s (full four-component read-only mirror).

### Code Review Findings

#### Matches Plan

- **Two hermetic generators, one folded file.** `tasks/notices.py` carries the
  `update()`/`check()` pair modelled on `public_api.py`, with the impure
  `_run_cargo_about`/`_run_license_checker` runners separated from the pure
  `_render_rust`/`_render_frontend`/`_fold`. `check()` verifies the file exists
  before paying the dual-generator cost and raises distinct `Exit`s per failure
  path.
- **§3.2 corresponding-source discharge.** The committed artefact carries
  `uluru 3.1.0` under `MPL-2.0` with a §3.2 block resolving to
  `github.com/servo/uluru` plus the immutable crates.io download endpoint. A
  `test_notices.py` regression asserts every MPL-2.0 block carries a source URL,
  so a future copyleft entry cannot ship without one.
- **Upload wiring.** `ATTRIBUTION_ARTEFACT_STAGED` is appended in
  `_release_uploads()` unsigned; `paths.py` carries both the committed and
  staged constants; `stage_notices` copies into `dist/release/`; both prepare
  lanes invoke it (asserted in the integration suite).
- **Coverage guards.** Positive presence assertions land in `test_build.py`
  (name-set) and `test_workflows.py` (attest-glob), beside the existing negative
  guard.
- **Determinism controls.** `cli/about.toml` pins `targets` to the four shipped
  release triples, excludes the fifth dev triple, ignores build/dev deps and
  the private workspace roots; `_fold` normalises to LF with one trailing
  newline; `.gitattributes` pins the artefact to `eol=lf`.
- **CI gate.** `check-attribution` exists in `main.yml` with `RUSTUP_HOME`
  routing and its own cache-key prefix, and is listed in the `prerelease`
  `needs:`. `deny.toml`'s discharge comment is rewritten to reference the
  artefact without over-claiming signing.
- **Phase 3 docs.** A `## Design Rationale` section on 0203 records the
  dual-generator, generated-over-hand, unsigned/SLSA, and dedicated-job
  rationale; `mise.toml [settings]` notes cargo-about beside the
  cargo-pup/cargo-public-api unverified carve-out.

#### Deviations from Plan

- ⚠️ **Rust section rendered from JSON, not `about.hbs`.** Plan §2 specified a
  Handlebars template at `cli/about.hbs`; the file does not exist. Instead
  `_run_cargo_about` runs `cargo about generate --format json` and
  `_render_rust` builds blocks in Python, matching the frontend renderer's shape
  in one fixture-testable place. This is an improvement — the plan itself warned
  against "a vacuous seam whose only test is a fixture mirroring the template"
  and offered folding as an alternative. No functional loss.
- ⚠️ **cargo-about is source-built, not a `[tools]` ubi pin.** Plan §1
  specified `ubi:EmbarkStudios/cargo-about` in `[tools]` plus a `mise.lock`
  regen. Implementation provisions it via `deps:install:cargo-about` (source
  build on stable) because 0.9.x release binaries omit `x86_64-apple-darwin`,
  which would break `mise install` on the Intel-mac leg. User-approved (marked
  `[~]` in the plan). Consequence: no `mise.lock` change, and cargo-about is a
  fourth accepted-unverified build surface (noted in `mise.toml [settings]`).
- ⚠️ **Header disclaims the vendored-runtime archives.** Commit `tsmstryv`
  extended the artefact header to state it does not cover Node/playwright-core
  (attributed in `accelerator-driver-*`) or Chromium (in
  `accelerator-browser-*`), each carrying its own `NOTICES/`. Not in the
  original plan; a correctness fix that stops the header over-claiming coverage.

#### Potential Issues

- ⚠️ **`license-checker-rseidelsohn@5.0.1` declares `engines.node >= 24`; the
  repo pins node v22.** npm emits `EBADENGINE` and proceeds, so the generator
  works today. A future npm that enforces engines, or a tool version tightening
  the floor, would break `notices:update`/`check-attribution` — the byte-compare
  couples the artefact to this exact tool, so the coupling is intentional but
  the engine mismatch is a latent fragility worth tracking.
- ⚠️ **The frontend guard depends on `node_modules` carrying the
  devDependency, and mise will not reinstall a stale tree.** During validation
  the guard first errored with `license-checker-rseidelsohn` absent from
  `node_modules/.bin`; a fresh `deps:install:node` repaired it and the guard
  passed. mise's idempotence fingerprint can leave a partially-installed
  `node_modules` in place, surfacing as a guard *error* rather than a clean
  reinstall. Environmental, not a plan defect.
- Pre-existing, unrelated: `public-api:check` emits a rustdoc `unclosed HTML
  tag` warning in `corpus/src/frontmatter_validation/template_shape.rs:611`.
  Outside this plan's surface; `mise run check` still exits 0.

### Manual Testing Required

These are the plan's own manual-verification items, deferred to CI or a local
release dry-run; none is a blocker for the pass verdict.

1. Closure fidelity:
   - [ ] `nm -a` on an unstripped `accelerator-vcs --release` build shows the
         `uluru` MPL sub-closure present
   - [ ] Sample one component per licence family for verbatim text + copyright
   - [ ] Confirm the frontend section lists `highlight.js` and the
         react/tanstack/dnd-kit/remark/rehype transitive set

2. Cross-platform determinism:
   - [ ] A macOS `notices:update` and a Linux `check-attribution` agree
         byte-for-byte (targets pinned, LF-normalised)

3. Release integration:
   - [ ] A local `mise run prerelease` stages
         `dist/release/accelerator-third-party-notices.txt` and
         `upload_and_verify_release`'s existence assertion finds it
   - [ ] The three `main.yml` attest blocks are unchanged (the `accelerator-*`
         glob already covers the file)

### Recommendations

- Tick AC bullet 5's checkbox on 0203 — the Design Rationale section satisfies
  it, but the `- [ ]` acceptance-criterion line is still unchecked, leaving the
  work item reading as incomplete against its own AC.
- Record the node-engine gap (v22 vs the tool's `>= 24` floor) somewhere
  durable so a future engine-enforcing npm is caught by expectation rather than
  a CI-only generator failure.
- No code change is required before merge; the pass is unconditional.
