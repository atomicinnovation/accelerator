---
type: "work-item"
id: "0203"
title: "Ship a Third-Party Attribution Artefact with the Release Uploads"
date: "2026-08-10T18:40:00+00:00"
author: "Toby Clemson"
producer: "implement-plan"
status: "done"
kind: "story"
priority: "medium"
parent: "work-item:0136"
relates_to: ["work-item:0185", "work-item:0188", "work-item:0165"]
tags: ["rust", "frontend", "licensing", "release", "vcs"]
last_updated: "2026-08-31T15:40:48+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-733"
---

# 0203: Ship a Third-Party Attribution Artefact with the Release Uploads

**Kind**: Story
**Status**: Done
**Priority**: Medium
**Author**: Toby Clemson

## Summary

The signed binaries we distribute carry third-party components under licences
that impose notice and attribution obligations, and no release upload
discharges them. MPL-2.0 (`uluru`, linked into five of six sub-binaries) adds a
§3.2 source-disclosure duty on top. Build a single third-party attribution
artefact covering both distributed closures — the Rust `cli/` workspace and the
React frontend embedded in the visualiser binary — and stage it alongside the
signed binaries.

## Context

`cli/deny.toml`'s `uluru` exception was recorded on the basis that dead-code
elimination removed the whole `gix`/`jj-lib` closure from every shipped
binary, so §3.2 did not bind. 0185 re-ran that check across all six
`DISPATCHED_SUBBINARIES` rather than the visualiser alone, and the premise
turned out to hold only for the visualiser.

Measured on unstripped `--release` builds for `aarch64-apple-darwin`, by
counting `gix_`/`jj_lib`/`uluru` symbols:

- `accelerator-vcs`, `accelerator-work`, `accelerator-collaboration` and
  `accelerator-migrate` already linked `uluru` **before** 0185's switch. Each
  constructs `InProcessProbe` directly, through call sites unrelated to the
  metadata-read path, so directly-called code cannot be eliminated. This is a
  pre-existing finding 0185 surfaced, not one it caused.
- `accelerator-corpus` linked none of the three before and all of them after.
  This one is caused by 0185 repointing `vcs_adapters::facts` onto the
  library-backed probe.
- `accelerator-visualiser` links none of the three, before or after, so its
  original exception rationale is intact.

`uluru` is `gix-pack`'s LRU pack cache, reached through `gix-odb`'s default
features, so it cannot be feature-gated out of the closure.

Beyond MPL-2.0's disclosure duty, the permissive closure carries pervasive
attribution obligations. MIT, Apache-2.0, BSD-2/3-Clause, ISC, Unicode-3.0 and
CDLA-Permissive-2.0 each require their copyright notice and licence text be
reproduced in a binary distribution; only Zlib (source-dist only) and CC0-1.0
(public domain) impose nothing. `cli/deny.toml`'s allow-list enumerates the
Rust side, pruned to exactly the closure it carries.

The obligation spans two distributed closures, not one. The Rust `cli/`
workspace links into the signed binaries. The visualiser binary additionally
embeds the Vite-bundled React frontend (`embed-dist`), so the frontend's npm
runtime deps — react, react-dom, `@tanstack/*`, `@dnd-kit/*`, react-markdown,
remark/rehype, and `highlight.js` (BSD-3-Clause) — are distributed inside a
signed binary too. `cli/deny.toml` covers only the Rust closure.

Note for whoever picks this up: the two string literals the original
verification procedure used (`extensions.objectFormat` for gix, `There is no
Jujutsu repo` for jj-lib) are unreliable as an absence test — both are missing
from binaries that demonstrably link the closure. Count symbols with `nm -a`
instead. Plain `grep` over a Mach-O binary also reports false positives here;
`strings -a | grep` or `nm -a | grep` are the sound forms.

## Requirements

- Produce a single third-party attribution artefact covering both distributed
  closures — the Rust `cli/` workspace and the React frontend embedded in the
  visualiser binary — carrying each component's notice and licence text, and
  for MPL-2.0 the §3.2 means of obtaining the corresponding source.
- Stage it into the release upload set (`_release_uploads()` in
  `tasks/github.py`) so it ships with the signed manifest rather than
  existing only in the repository.
- Extend the upload-set coverage assertions — the direct name-set check in
  `test_build.py` and the attest-glob coverage in `test_workflows.py` — so a
  future change to the upload set cannot silently drop the artefact.
- Generate the artefact from both dependency graphs rather than
  hand-maintaining it, so it tracks closure changes automatically:
  `cargo-about` (or equivalent) over the `cli/` workspace, and a JS-side pass
  (e.g. `license-checker` or a Rollup licence plugin) over the frontend
  bundle, folded into one file.
- Update `cli/deny.toml`'s `uluru` exception comment to point at the shipped
  artefact once it exists, replacing the current statement that the release
  upload set carries none.

## Acceptance Criteria

- [ ] The attribution artefact exists as a manifest-derived superset of both
      distributed closures — the Rust binaries and the embedded frontend
      bundle — naming each component with its licence and, for MPL-2.0, the
      source-availability notice. Completeness is verified against the actual
      output, over-inclusion accepted: the artefact's Rust component set
      supersets the chosen Rust generator's manifest output reconciled with
      `cli/deny.toml`'s allow-list, `nm -a` symbol counts confirm the MPL
      sub-closure is present, and the chosen JS licence pass over the built
      `dist/` bundle confirms the frontend set. Any generator permitted by the
      Requirements satisfies this, `cargo-about` and `license-checker` being the
      reference tools.
- [ ] Each named component carries its verbatim licence text and copyright
      notice, and each MPL-2.0 component a §3.2 statement resolving to an
      obtainable source — not merely the SPDX identifier. Verified by sampling
      one component per licence family and confirming its full text and
      copyright line are present.
- [ ] It is present in the release upload set and covered by both the
      name-set assertion in `test_build.py` and the attest-glob assertion in
      `test_workflows.py`.
- [ ] The artefact is produced by a checked-in generator config/command over
      both graphs; re-running it reproduces the shipped file with no hand edits.
- [ ] The rationale for the chosen generated, dual-generator approach is
      documented on this item.
- [ ] `cli/deny.toml`'s `uluru` exception comment references the shipped
      attribution artefact and no longer asserts the release upload set carries
      no MPL component.
- [ ] `mise run` (bare default task) exits 0 end-to-end.

## Dependencies

- Relates to: work-item:0185 — surfaced the finding and made
  `accelerator-corpus` reach the closure.
- Relates to: work-item:0188 — delivered the library-backed adapter whose
  closure carries `uluru`.
- Relates to: work-item:0165 — owns the release manifest and upload set this
  artefact has to join. Its `_release_uploads()`/`TREE_ARTIFACTS` staging has
  shipped, so the upload-set-presence and generator-reproducibility criteria
  have a stable integration surface; this item is not blocked on it.
- Build-toolchain coupling: `cargo-about` and the chosen JS licence tool
  (`license-checker` or a Rollup/Vite plugin) must be pinned in `mise.toml` and
  available to CI, or the "`mise run` exits 0" criterion fails at pipeline time
  even with correct artefact logic.

## Technical Notes

- Register the artefact as a release tree artifact: add it to `TREE_ARTIFACTS`
  (`tasks/shared/paths.py`) so `_tree_artifact_uploads()` stages it into
  `_release_uploads()` (`tasks/github.py:258`) flat in `dist/release/`
  alongside the signed manifest.
- Two coverage guards must see it: the direct name-set assertion in
  `tests/unit/tasks/test_build.py:499`, and
  `test_attest_globs_cover_every_published_asset` in
  `tests/unit/tasks/test_workflows.py:207`, which proves the `accelerator-*`
  attest glob covers every published asset.
- Two generators, one file. `cargo-about` (config plus a Handlebars template)
  covers the `cli/` workspace closure; the frontend needs a separate JS-side
  pass (`license-checker`, or a Rollup/Vite licence plugin run over the
  bundle) since `cli/deny.toml` and `cargo-about` never see the npm tree.
- The frontend is a distributed closure via `embed-dist`
  (`cli/visualiser/server/src/server.rs`), which bakes `dist/` into the
  visualiser binary. Its bundled npm licences are MIT-heavy plus `highlight.js`
  (BSD-3-Clause); enumerate against the built bundle, not just direct
  `dependencies`.
- Generation reflects the manifest graph, not the linked closure. A generated
  artefact over-approximates — it lists components even for binaries that
  dead-code-eliminate them (the visualiser). That is the safe direction:
  over-inclusion in a notice is harmless, omitting a shipped component is the
  violation. This is why generated beats hand-maintained despite the
  imprecision.
- Zlib (source-dist only) and CC0-1.0 (public domain) impose nothing on the
  binary; include them harmlessly rather than special-casing exclusion.

## Drafting Notes

- Widened scope from MPL-only to a full third-party attribution artefact across
  both distributed closures, per direction, after finding the permissive
  closure (MIT/Apache/BSD/ISC/Unicode/CDLA) carries binary attribution duties
  and the visualiser binary embeds the npm frontend bundle.
- Recorded the artefact as generated per direction; the manifest-over-
  approximates-closure trade-off in Technical Notes means the output is a safe
  superset of the actual closure, not an exact match.
- The frontend transitive licence set is not yet enumerated — only direct
  runtime deps were checked; the JS-side generator run produces the
  authoritative list.

## Design Rationale

Recorded against AC bullet 5, on the generated, dual-generator approach as built.

- **Two generators, one file.** The Cargo and npm dependency graphs never meet:
  `cargo-about` reads the `cli/` workspace closure, `license-checker-rseidelsohn`
  reads the frontend production tree. `tasks/notices.py` folds both into
  `licenses/accelerator-third-party-notices.txt`.
- **Generated, not hand-maintained.** The manifest over-approximates the linked
  closure — it lists components a binary may dead-code-eliminate. That is the
  safe direction: over-inclusion in a notice is harmless, omitting a shipped
  component is the violation.
- **`license-checker-rseidelsohn`** over the abandoned original `license-checker`
  (unmaintained since 2019).
- **cargo-about is source-built, not a `mise [tools]` ubi pin.** Its 0.9.x
  release binaries omit `x86_64-apple-darwin`, so a `[tools]` ubi pin would fail
  `mise install` on the Intel-mac `smoke-runtime` CI leg. `deps:install:cargo-about`
  runs `cargo install cargo-about --locked --features cli`, the `cargo-public-api`
  pattern — resolvable on every host, and checksum-verified against crates.io by
  cargo (more integrity than a mutable ubi release tag would have carried, not
  less). It is a fourth cargo-installed tool outside `mise.lock`'s hash-pinning,
  alongside cargo-pup and cargo-public-api.
- **Both sections render in Python from `cargo about --format json`.** cargo-about's
  Handlebars model is licence-grouped and cannot join crate-to-text or globally
  sort per-crate blocks with their verbatim text, so `about.hbs` was dropped;
  `_render_rust` and `_render_frontend` share one `_block` builder, making the two
  halves structurally identical by construction.
- **Hermetic generation** (`cargo about --frozen`, `license-checker --production`)
  keeps the drift check in the fast `check` lane. `cargo fetch --locked` extracts
  the registry sources `--frozen` reads, verified byte-identical across a cold and
  a warm cache. The tradeoff: the byte-compare couples the committed file to each
  tool's exact output, so a deliberate tool-version or `package-lock.json` bump is
  expected to require a `notices:update`.
- **§3.2 discharged by per-crate source URLs** (repository + the immutable
  crates.io download endpoint) rather than a hosted mirror or written offer —
  the cheapest form that resolves to obtainable source, byte-stable, no
  infrastructure to maintain. The frontend renderer mirrors the emission against
  the npm tarball, so the header's blanket §3.2 claim holds if a copyleft npm
  dependency ever enters the closure.
- **`--production` node_modules guarded, not bundle-rendered.** Rendering the
  production superset keeps the check off `build:frontend`; a `default`-lane guard
  (`test:unit:frontend-licenses`) closes the unsafe omission direction by
  asserting every module in a throwaway sourcemap build resolves to that closure
  — catching a runtime dep mis-declared under `devDependencies`.
- **Unsigned artefact, tamper-detection via SLSA provenance.** The notice is not a
  trust anchor — the launcher resolves nothing against it — so it carries no
  `.minisig` and no `_release_reverifies()` entry, but rides the `dist/release/
  accelerator-*` provenance glob. A swapped §3.2 pointer is implausible to
  weaponise against durable crates.io/repository source anchors.
- **Dedicated `check-attribution` CI job.** CI runs no aggregate `check`, and no
  existing job provisions both the cargo registry and `node_modules`. The job is
  read-only and holds no signing keys, so a substituted generator's blast radius
  is bounded by job isolation rather than by the drift check.
