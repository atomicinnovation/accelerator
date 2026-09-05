---
type: "pr-description"
id: "98"
title: "[0203] Ship a third-party attribution artefact with the release"
date: "2026-09-05T11:08:43+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "work-item:0203"
parent: "work-item:0203"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/98"
pr_number: 98
tags: ["licensing", "release", "rust", "frontend", "vcs"]
revision: "87092f9e6991fe7e4eec1ad453d0590068ab985b"
repository: "accelerator"
last_updated: "2026-09-05T11:08:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0203] Ship a third-party attribution artefact with the release

## Summary

Ships one generated third-party attribution file covering both distributed
closures — the Rust `cli/` workspace linked into the signed binaries and the
Vite-bundled React frontend embedded in the visualiser — and stages it flat into
the release upload set. The signed release distributes components under
notice-and-attribution licences (five sub-binaries link `uluru`, MPL-2.0) that
no upload currently discharges; this closes that gap with a byte-reproducible,
drift-checked artefact that names verbatim licence text, copyright, and — for
every MPL-2.0 component — a §3.2 corresponding-source statement.

## Changes

- **Dual-generator notices pipeline** (`tasks/notices.py`). `cargo about
  generate --format json` renders the Rust closure and
  `license-checker-rseidelsohn --production` renders the frontend closure; a pure
  functional core (`_render_rust`/`_render_frontend`/`_fold`) folds both into
  `licenses/accelerator-third-party-notices.txt`, sorted by `name@version` and
  normalised to LF. Modelled on the `public_api` update/check pair.
- **Drift check wired into the CI mirror.** `mise run notices:check` byte-
  compares the committed file against a fresh render and is in `check.depends` +
  `default.depends`. A dedicated `check-attribution` CI job provisions both
  closures (source-built `cargo-about`, warm `cli/` registry, `npm ci`) and is
  added to the `prerelease` `needs:` so it gates the release.
- **Release staging and upload.** `stage_notices` copies the committed file into
  `dist/release/`; both prepare lanes call it; `_release_uploads()` appends it
  unsigned. Positive presence guards land in `test_build.py` (name-set) and
  `test_workflows.py` (attest-glob); staging-wiring is asserted in the
  integration suite.
- **Determinism controls.** `cli/about.toml` pins the four shipped release
  triples, ignores build/dev deps and private workspace roots; `.gitattributes`
  pins the artefact to `eol=lf`.
- **Frontend omission guard** (`tests/unit/frontend/test_frontend_licenses.py`).
  A `default`-lane test asserts every runtime module in the built bundle resolves
  to the `--production` closure, catching a runtime dep mis-declared under
  `devDependencies` before it ships unattributed.
- **`deny.toml` reconciliation** and a British-spelling rename of
  `TREE_ARTIFACTS` → `TREE_ARTEFACTS` across its call sites.
- **Documentation.** A Design Rationale section on work item 0203, `tasks/README.md`
  entries, and a validation report recording a `pass`.

## Context

- Work item: `meta/work/0203-ship-a-third-party-attribution-artefact-with-the-release.md`
- Plan: `meta/plans/2026-08-31-0203-third-party-attribution-artefact.md` (status `done`)
- Research: `meta/research/codebase/2026-08-31-0203-third-party-attribution-artefact.md`
- Validation: `meta/validations/2026-08-31-0203-third-party-attribution-artefact-validation.md`

## Testing

- [x] Round-trip idempotent: `mise run notices:update && mise run notices:check`
      leaves the working tree clean (byte-reproducible).
- [x] Unit + integration suites pass: `test_notices.py`, `test_mise.py`,
      `test_build.py`, `test_workflows.py` (144), `test_release.py` (39),
      `tests/unit/frontend` (4).
- [x] `mise run deny:check` green after the exception-comment edit.
- [x] Aggregate read-only gate: `mise run check` exits 0 (full four-component
      mirror; code unchanged since that run — the two commits on top are
      docs-only).
- [ ] Cross-platform determinism: a macOS `notices:update` and a Linux
      `check-attribution` agree byte-for-byte — deferred to CI.
- [ ] Manual: `nm -a` confirms the `uluru` MPL sub-closure in an unstripped
      build; sample one component per licence family for verbatim text.

## Notes for Reviewers

Three deviations from the plan text, all sound:

- **JSON render, not `about.hbs`.** The Rust section is built in Python from
  cargo-about JSON rather than a Handlebars template — one fixture-testable
  render path, matching the frontend renderer's block shape.
- **`cargo-about` source-built, not a `[tools]` ubi pin.** Its 0.9.x release
  binaries omit `x86_64-apple-darwin`, which would break `mise install` on the
  Intel-mac leg; provisioned via `deps:install:cargo-about` instead. No
  `mise.lock` change; noted as an accepted-unverified surface in `mise.toml
  [settings]`.
- **Header disclaims the vendored-runtime archives.** Node/playwright-core and
  Chromium ship their own `NOTICES/` in the `accelerator-driver-*` /
  `accelerator-browser-*` archives, so the file states it does not cover them.

Two caveats worth tracking: `license-checker-rseidelsohn@5.0.1` declares
`engines.node >= 24` while the repo pins node v22 (works today, `EBADENGINE`
warning only); and branch protection's required-status-check list must name
`check-attribution` for the new gate to block merges.
