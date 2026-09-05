---
type: "codebase-research"
id: "2026-08-31-0203-third-party-attribution-artefact"
title: "Research: Ship a Third-Party Attribution Artefact with the Release Uploads"
date: "2026-08-31T20:26:46+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0203"
parent: "work-item:0203"
relates_to: ["codebase-research:2026-08-10-0185-converge-corpus-adapters-library-backed-vcs", "codebase-research:2026-08-02-0188-library-backed-vcs-adapter", "codebase-research:2026-07-06-0165-multi-binary-distribution-release-pipeline"]
topic: "Ship a Third-Party Attribution Artefact with the Release Uploads"
tags: ["research", "codebase", "licensing", "release", "cargo-about", "attribution", "cli", "frontend"]
revision: "bd77d85f303712cf22fb881b51fb0e6c63dc5317"
repository: "accelerator"
last_updated: "2026-08-31T20:26:46+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Ship a Third-Party Attribution Artefact with the Release Uploads

**Date**: 2026-08-31T20:26:46+00:00
**Author**: Toby Clemson
**Git Commit**: bd77d85f303712cf22fb881b51fb0e6c63dc5317
**Branch**: HEAD (detached; jj working copy)
**Repository**: accelerator (build-system workspace)

## Research Question

What are the live integration surfaces for work item 0203 — shipping a single
generated third-party attribution artefact covering both distributed closures
(the Rust `cli/` workspace and the React frontend embedded in the visualiser
binary), staged into the release upload set and guarded by the two coverage
tests, with the `uluru` MPL-2.0 exception comment updated?

## Summary

Every integration point the work item names exists and was confirmed. Three
findings sharpen the plan beyond what the item states:

- **The `TREE_ARTIFACTS` route is the wrong mechanism.** Technical Notes say
  register the artefact via `TREE_ARTIFACTS`, but that path fans one token out
  to 16 per-platform assets (archive + three sidecars × four platforms) and
  drags in executable smoke-checks, a `pins.toml` digest per platform, a
  `TreeSpec` in the spec builder, an `Artifact` literal widening, and an
  exact-set manifest-coherence gate. A single platform-independent text file
  fits none of that. **Model it on `RELEASE_MANIFEST` instead** — a standalone
  path constant staged flat into `dist/release/` and appended in
  `_release_uploads()`.
- **Naming decides whether the workflow changes.** The attest-glob test
  (`test_workflows.py:207`) uses `@actions/glob` semantics (`*` does not cross
  `/`). A name like `THIRD-PARTY-NOTICES.txt` matches no existing glob and would
  force edits to all three attest blocks in `main.yml`. **Naming it
  `accelerator-third-party-notices.txt`** makes `dist/release/accelerator-*`
  cover it with zero workflow edits.
- **The name-set test needs no change; a positive assertion must be added.**
  `test_build.py:498` is a negative membership check (no fixture binary is an
  upload). Nothing there asserts the attribution file is present, so the item's
  "name-set assertion" is a *new* positive check to write, not an existing one
  to extend.

The Rust generator (`cargo-about`) and JS licence pass are both greenfield: no
`about.toml`/`.hbs`/Handlebars usage exists, and no JS licence tooling is in
the frontend package. The repo carries strong precedents for every piece — the
`public_api` generate-and-byte-compare drift check, the aqua-pinned cargo
subcommand model, and an existing verbatim-licence `NOTICES` generator in the
vendor-assembly path.

## Detailed Findings

### Release upload-set assembly (`tasks/github.py`, `tasks/shared/paths.py`)

`_release_uploads(tokens, debug_dirs, tree_tokens)`
(`tasks/github.py:258-277`) returns a flat list of `Path`s, each of which must
resolve inside `dist/release/` (`RELEASE_STAGING`, `paths.py:22`) — except the
debug archives, which live under each sub-binary's committed `bin/` tree. It
only *references* already-staged paths; it never stages. Categories, in order:
debug archives, launcher binary + `.minisig`, `RELEASE_MANIFEST` +
`RELEASE_MANIFEST_SIG`, sub-binary assets via `_subbinary_uploads()`
(`github.py:224-233`), and tree artifacts via `_tree_artifact_uploads()`
(`github.py:236-255`).

**`RELEASE_MANIFEST` is the template for a single-file upload.** It is a path
constant (`paths.py:23-24`) pointing at `dist/release/manifest.json`, staged by
the manifest lane, and appended directly in `_release_uploads()`
(`github.py:273-274`). `upload_and_verify_release()` (`github.py:404-456`)
asserts every referenced path exists before upload (`github.py:427-431`) and
uploads with `--clobber`. Whether the attribution file also needs a detached
signature and a `_release_reverifies()` entry (`github.py:280-315`) is a
policy choice — the manifest is tamper-evident; a notices file need not be.

**Why not `TREE_ARTIFACTS`.** `TREE_ARTIFACTS`
(`paths.py:45-48`, currently `("driver", "browser")`) is a tuple of bare
tokens, each keyed into satellite tables the whole assembly path asserts over:
`ARTIFACT_EXECUTABLES` + `smoke_check` (runs `[binary, "--version"]` — a text
file has no executable), `TREE_ARTIFACT_DESCRIPTIONS`, `default_spec_builder`'s
hardcoded `TreeSpec`s, the `Artifact = Literal["driver", "browser"]` alias
(`assemble.py:44`), a reviewed `pins.toml` digest per (artifact, platform), and
`_assert_staged_manifest_is_current` (`release.py:93-111`), which requires the
staged manifest's artifact set to *exactly equal*
`{(name, platform) for name in TREE_ARTIFACTS for platform in TARGETS}`. One new
token would trip every one of these.

### The two coverage guards (`test_build.py`, `test_workflows.py`)

Both tests read the single derivation `_release_uploads()`; neither hardcodes an
expected name set.

**Guard 1 — name-set (`tests/unit/tasks/test_build.py:498-509`).** The test is
`test_no_fixture_binary_is_a_release_upload`. It computes
`names = {path.name for path in _release_uploads()}` and asserts no
`_CLI_FIXTURE_BINARIES` substring appears in it — a *negative* check. Adding an
attribution upload passes trivially (no fixture substring in the name). The
item's "name-set assertion covering the artefact" is therefore a **new positive
assertion to add** (e.g. `assert "accelerator-third-party-notices.txt" in
names`), not an edit to this test.

**Guard 2 — attest glob (`tests/unit/tasks/test_workflows.py:207-228`).**
`test_attest_globs_cover_every_published_asset` computes `published` from
`_release_uploads(tree_tokens=TREE_ARTIFACTS)` (tree artifacts *included* here,
unlike guard 1's default `()`), makes each path repo-relative posix, and asserts
every one is matched by some `subject-path` glob drawn from the three
`attest-build-provenance` steps in `.github/workflows/main.yml` (lines
660-664, 788-792, 812-816). Matching is `@actions/glob` via `_glob_matches`
(`test_workflows.py:161-168`): each `*` compiles to `[^/]*`, so `*` spans dots
but not slashes. The three blocks must stay identical
(`test_every_attest_block_declares_the_same_subjects`, lines 198-204).

The current globs are `skills/visualisation/visualise/bin/accelerator-visualiser-*`,
`dist/release/accelerator-*`, `dist/release/manifest.json`,
`dist/release/manifest.minisig`. **`dist/release/accelerator-third-party-notices.txt`
is covered by `dist/release/accelerator-*` with no workflow edit.** A name not
starting with `accelerator-`, or one nested in a subdirectory, would need a new
`subject-path` line added to all three blocks together.

### Rust closure: licence config (`cli/deny.toml`)

The `[licenses]` section (lines 44-102) sets `version = 2` and an `allow` list
pruned to exactly the eleven SPDX identifiers the closure carries — `MIT`,
`Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `BSD-2-Clause`, `BSD-3-Clause`,
`ISC`, `Zlib`, `Unicode-3.0`, `CDLA-Permissive-2.0`, `CC0-1.0`. The header
comment (lines 46-51) records the pruning policy: cargo-deny warns on an unused
allowance, and any copyleft/MPL/*GPL must go through
`[[licenses.exceptions]]`, never the blanket allow.

The **`uluru` exception** is the sole exception block (lines 100-102:
`crate = "uluru"`, `allow = ["MPL-2.0"]`), preceded by the rationale comment at
lines 66-99. The clause the item targets is lines 69-71 — "§3.2's notice
obligation binds wherever the closure is actually linked … which the release
upload set carries no artefact for." The comment already anticipates the fix
(lines 88-89: "a third-party attribution artefact is owed by the release upload
set"). The `[graph]` targets (lines 11-17) define the closure the artefact must
cover; the shipped triples are the four `TARGETS` plus an x86_64-linux-gnu CI
dev graph. Five sub-binaries link `uluru` (`accelerator-vcs`, `-work`,
`-collaboration`, `-migrate`, `-corpus`); four do not (`-visualiser`, `-design`,
`-linear`, `-jira`).

cargo-deny runs via `mise run deny:check` → `invoke deny.check`
(`tasks/deny.py:6-20`), which runs `cargo deny check advisories licenses bans
sources` from `cli/` (config resolves relative to cwd). It sits outside the
`cli:` roll-up, wired directly into top-level `check` (`mise.toml:640`) and the
bare `default` (line 644). The binary is aqua-pinned:
`"aqua:EmbarkStudios/cargo-deny" = "0.19.8"` (`mise.toml:22-25`).

### Frontend closure: build and embed (`cli/visualiser/`)

The frontend is built by **Vite 6** into `cli/visualiser/frontend/dist/`
(default `outDir`, no override in `vite.config.ts`), via `tasks/build.py:268-271`
(`npm --prefix {FRONTEND} run build` → `tsc -b && vite build`). That directory
is baked into the release binary by **`rust-embed`** under the `embed-dist`
feature — the embed lives in `cli/visualiser/server/src/assets.rs:69-72`
(`#[folder = "../frontend/dist"]`), not in `server.rs` (which only wires the SPA
fallback). `build.rs:7-23` asserts `../frontend/dist/index.html` exists at
compile time when `embed-dist` is set. The release build path uses default
features (embed on); `dev-frontend` serves from disk instead.

Runtime `dependencies` (`package.json:24-36`), the direct set for the licence
pass: `@dnd-kit/core`, `@dnd-kit/sortable`, `@dnd-kit/utilities`,
`@tanstack/react-query`, `@tanstack/react-router`, `highlight.js` (BSD-3-Clause),
`react`, `react-dom`, `react-markdown`, `rehype-highlight`, `remark-gfm`. The
markdown/remark/rehype trio pulls a large transitive tree (micromark, mdast,
hast, unist); **enumerate transitives against the built bundle, not just these
eleven**. No JS licence tooling exists yet — no `license-checker`, no
`rollup-plugin-license`, no vite licence plugin in devDependencies or config.

### Generator and drift-check precedents (`tasks/`)

The **`public_api` pair is the closest template** for a checked-in generated
artefact validated by re-running the generator: `update()`
(`tasks/public_api.py:146-159`) renders and writes the snapshot; `check()`
(lines 113-143) re-renders and byte-compares, raising `Exit` that names
`mise run public-api:update`. Two other variants exist — `docs.py`'s
generate-then-set-coverage check (lines 55-100) and `build.py`'s SHA-256
digest-marker guard (`build.py:536-616` + `tasks/lint/vendor_shims.py`), the
latter cheaper than re-running the generator in CI. All raise
`invoke.Exit(msg, code=1)` naming the exact regeneration command.

An **existing verbatim-licence generator** already lives in the vendor path:
`write_notices()` (`tasks/shared/vendor/assemble.py:203-218`) copies each
component's licence files into `NOTICES/<component>/` and **fails the release if
a component contributes no licence file** — the exact "omitting a shipped
component is the violation" invariant 0203 wants. `licenses/chromium.LICENSE`
(wired via `CHROMIUM_LICENSE`, `paths.py:53-55`) is the committed-licence-text
precedent and sets the precedent for a top-level `licenses/` directory.

**Tool pinning.** A cargo subcommand pins via aqua at an exact version with an
inline justifying comment; `cargo-about` is an EmbarkStudios project like
`cargo-deny`, so the natural pin is
`"aqua:EmbarkStudios/cargo-about" = "<version>"` in `mise.toml [tools]`. Any
`[tools]` edit forces `mise lock --platform linux-x64,macos-arm64,macos-x64`
and a `test_mise.py` placement guard. `license-checker` fits the JS model
better as a frontend `devDependency` invoked through an npm script, not a
`[tools]` entry. cargo-about consumes an `.hbs` Handlebars template — none
exists in the repo yet, so co-locate config + template under `cli/` beside
`deny.toml`, `rustfmt.toml`, `clippy.toml`, `pup.ron`.

## Code References

- `tasks/github.py:258-277` — `_release_uploads()`, the single upload-set derivation both tests read.
- `tasks/github.py:236-255` — `_tree_artifact_uploads()`, 16-asset fan-out per token (the mechanism to avoid).
- `tasks/github.py:404-456` — `upload_and_verify_release()`, existence assertion + `--clobber` upload.
- `tasks/shared/paths.py:22-24` — `RELEASE_STAGING`, `RELEASE_MANIFEST`, `RELEASE_MANIFEST_SIG` (the single-file template).
- `tasks/shared/paths.py:45-48` — `TREE_ARTIFACTS` and its keyed-satellite contract.
- `tasks/shared/paths.py:53-55` — `CHROMIUM_LICENSE`, the committed-licence precedent.
- `tasks/shared/vendor/assemble.py:203-218` — `write_notices()`, verbatim-licence copy with fail-on-missing.
- `tests/unit/tasks/test_build.py:498-509` — negative name-set guard; add a positive presence assertion here.
- `tests/unit/tasks/test_workflows.py:161-228` — attest-glob coverage with `@actions/glob` matching.
- `.github/workflows/main.yml:660-664,788-792,812-816` — the three identical attest `subject-path` blocks.
- `cli/deny.toml:66-102` — `uluru` MPL-2.0 exception comment (lines 69-71 to update) and the pruned allow-list.
- `tasks/deny.py:6-20` — cargo subcommand run from `cli/` idiom.
- `cli/visualiser/server/src/assets.rs:69-72` — `rust_embed` `#[folder = "../frontend/dist"]`.
- `cli/visualiser/frontend/package.json:24-36` — frontend runtime dependencies.
- `tasks/build.py:268-271` — `build.frontend` (Vite build).
- `tasks/public_api.py:113-159` — generate-and-byte-compare drift-check template.
- `mise.toml:22-25` — aqua cargo-subcommand pin pattern.

## Architecture Insights

- **The upload set is a computed derivation, not a manifest of literals.** Both
  coverage guards read `_release_uploads()`; correctness flows from adding the
  artefact there once. Miss that call and the file is neither published nor
  attested, and no test notices.
- **Attest-glob semantics are the hidden naming constraint.** Flat-in-
  `dist/release/` and an `accelerator-` prefix are not stylistic — they are what
  keeps the artefact inside an existing provenance glob without touching three
  workflow blocks. `tree_artifact_asset_path` keeps tree archives flat for the
  same reason (`paths.py:113-117`).
- **Over-approximation is the safe direction and it is already the house
  style.** `write_notices()` fails closed on a missing licence; `deny.toml`
  prunes allow to exactly the closure. A generated attribution superset (listing
  components even for binaries that dead-code-eliminate them) matches this
  posture — over-inclusion is harmless, omission is the violation.
- **Two generators, one file, because the graphs never meet.** cargo-about and
  `deny.toml` never see the npm tree; a JS pass over the built bundle never sees
  the Rust closure. The single artefact is a fold of two independent passes.

## Historical Context

- `meta/work/0188-library-backed-vcs-adapter.md` — introduced the gix/jj-lib/uluru closure and the MPL-2.0 exception.
- `meta/work/0185-converge-corpus-adapters-on-library-backed-vcs.md` — the switch that spread the closure to `accelerator-corpus`; its validation filed 0203.
- `meta/work/0165-multi-binary-distribution-and-release-pipeline.md` — owns `_release_uploads()`/`TREE_ARTIFACTS`; the staging surface 0203 joins.
- `meta/work/0214-settle-the-vendored-runtime-tree-artifact-mechanisms.md` — tree-artifact/release payload mechanics behind the `TREE_ARTIFACTS` route.
- `meta/validations/2026-08-10-0185-converge-corpus-adapters-on-library-backed-vcs-validation.md` — per-binary `gix_`/`jj_lib`/`uluru` symbol table; reproduced the MPL finding against the release.
- `meta/validations/2026-08-03-0188-library-backed-vcs-adapter-validation.md` — flagged that the exception comment asserted a shipped notice artefact that did not yet exist.
- `meta/reviews/work/0185-converge-corpus-adapters-on-library-backed-vcs-review-1.md` — the review that promoted the attribution obligation to its own work item.
- No dedicated ADR covers licensing/attribution; the closest are the distribution/tree-artifact ADRs (ADR-0046, 0053, 0054, 0059–0064).

## Related Research

- `meta/research/codebase/2026-08-10-0185-converge-corpus-adapters-library-backed-vcs.md`
- `meta/research/codebase/2026-08-02-0188-library-backed-vcs-adapter.md`
- `meta/research/codebase/2026-07-06-0165-multi-binary-distribution-release-pipeline.md`
- `meta/research/codebase/2026-08-11-0196-design-cli-implementation-surface.md`

## Open Questions

- **Does the attribution file need a detached signature + re-verify entry?** The
  manifest and binaries are tamper-evident via `.minisig` and
  `_release_reverifies()`. A notices file could ship unsigned (it is not a
  trust anchor), or be signed for uniformity. This is a policy call the plan
  should settle.
- **Byte-compare drift check vs. digest-marker?** `public_api`-style byte
  comparison re-runs both generators in CI (needs cargo-about + node + a built
  bundle present); a digest-marker guard is cheaper but records less. The
  cargo-about pass needs `dist/` built for the JS side regardless.
- **`mise.lock` and `test_mise.py`/placement guards.** Adding `cargo-about` to
  `[tools]` forces a lockfile regen across three platforms and likely a task-
  placement assertion; confirm the exact guard when wiring the generate/check
  task into both `cli:check` and the bare `default` lane.
