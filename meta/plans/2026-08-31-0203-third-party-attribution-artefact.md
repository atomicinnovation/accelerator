---
type: "plan"
id: "2026-08-31-0203-third-party-attribution-artefact"
title: "Third-Party Attribution Artefact Implementation Plan"
date: "2026-08-31T20:44:29+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0203"
parent: "work-item:0203"
derived_from: ["codebase-research:2026-08-31-0203-third-party-attribution-artefact"]
tags: ["rust", "frontend", "licensing", "release", "vcs"]
revision: "64587f966a0afa3b44eeb8184aa42cb3a43958b5"
repository: "accelerator"
last_updated: "2026-08-31T22:52:15+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Third-Party Attribution Artefact Implementation Plan

## Overview

Ship a single generated third-party attribution file covering both distributed
closures — the Rust `cli/` workspace linked into the signed binaries, and the
Vite-bundled React frontend embedded in the visualiser binary — and stage it
flat into the release upload set. The file is produced by two hermetic
generators folded into one text file, guarded by a `public_api`-style
byte-compare drift check, and its presence in the upload set is pinned by the
name-set test and the attest-glob test. The `uluru` MPL-2.0 exception comment
is updated to point at the shipped artefact.

## Current State Analysis

The signed release distributes binaries carrying third-party components under
notice-and-attribution licences, and no upload discharges them. Five of nine
dispatched sub-binaries link `uluru` (MPL-2.0), which adds a §3.2
source-disclosure duty; the permissive closure (MIT/Apache/BSD/ISC/Unicode/CDLA)
adds pervasive binary-attribution duties. `cli/deny.toml`'s allow-list
enumerates the Rust side; nothing enumerates the frontend npm closure, which
ships inside the visualiser binary via `rust-embed`.

Verified integration surfaces:

- **Upload set is one derivation.** `_release_uploads()`
  (`tasks/github.py:258-277`) returns a flat `list[Path]`, each resolving inside
  `dist/release/` (`RELEASE_STAGING`, `paths.py:22`). `RELEASE_MANIFEST` is
  appended directly (`github.py:273`) — the template for a single-file upload.
  `upload_and_verify_release()` asserts every path exists before upload
  (`github.py:427-431`).
- **`TREE_ARTEFACTS` is the wrong route.** It fans one token to 16 assets and
  trips an exact-set manifest gate (`release.py:93-111`), smoke-checks that run
  `[binary, "--version"]`, and a per-platform `pins.toml` digest. A text file
  fits none of it.
- **Two guards read the derivation.** `test_no_fixture_binary_is_a_release_upload`
  (`test_build.py:498-509`) computes `{path.name for path in _release_uploads()}`
  — a negative membership check; a positive presence assertion is new.
  `test_attest_globs_cover_every_published_asset` (`test_workflows.py:207-228`)
  asserts every published path is matched by an `@actions/glob` `subject-path`
  from the three attest blocks in `main.yml`; `*` spans dots but not slashes.
- **Naming keeps the workflow untouched.** `dist/release/accelerator-*` already
  covers `accelerator-third-party-notices.txt` — no edit to the three attest
  blocks.
- **Generators are greenfield.** No `about.toml`/`.hbs`, no JS licence tooling.
  `public_api.py:113-159` is the generate-and-byte-compare precedent;
  `write_notices()` (`assemble.py:203-218`) is the fail-on-missing-licence
  precedent; `licenses/chromium.LICENSE` is the committed-licence-directory
  precedent.
- **Tool pinning.** `mise.toml [tools]` pins aqua tools with per-platform
  `sha256` hashes in `mise.lock`; `ubi:` backends (`minisign`, `cosign`) are
  pinned by **version only** — `mise.lock` carries no checksum for them, so a
  `ubi:` tool is not hash-verified. `test_mise.py:19` (`_CHECK_GATES`) pins each
  drift gate into `check.depends`.

### Key Discoveries:

- `cargo-about` is **not** in the aqua registry (`EmbarkStudios/cargo-about`
  returns 404; only `cargo-deny` is there). Pin it as
  `ubi:EmbarkStudios/cargo-about` at `0.9.2`, matching the `minisign`/`cosign`
  `ubi:` precedent (`mise.toml:35,39`). This is **version, not hash, pinning**:
  `mise.lock` records only `version`/`backend`/`options` for `ubi:` backends
  (no per-platform `checksum`/`url`, unlike the hashed aqua entries), so the
  binary is fetched from a mutable GitHub-release tag with no digest
  verification. Confirm cargo-about `0.9.2` publishes GitHub-release assets ubi
  can resolve for `linux-x64`, `macos-arm64` and `macos-x64` before committing.
- `license-checker` (davglass) is abandoned since 2019; the maintained fork is
  `license-checker-rseidelsohn` (`5.0.1`). It emits verbatim `licenseText` and
  `copyright` per package via a `--customPath` JSON template.
- `cargo about generate --frozen` reads per-crate licence files and copyright
  lines from the local cargo cache and embeds standard SPDX texts with no
  network round-trip, which is what lets the drift check live in the fast
  `check` lane, not `default`-only. Two provisos the plan pins below: (1)
  confirm `--frozen` is a recognised `generate` flag in `0.9.2` and implies both
  `--locked` and `--offline`; if not, pass `--locked --offline` explicitly. (2)
  Offline generation still needs the `cli/` registry populated — so the
  dedicated CI job warms it with `cargo fetch --locked` before the generate,
  and the task provisioning is declared, not assumed.
- **Determinism needs pinned `targets` and features.** cargo-about's resolved
  closure and feature unification vary by host triple, so an unpinned
  `about.toml` renders different bytes on macOS vs Linux from an identical
  `Cargo.lock`. Pin `targets` to the **four shipped release triples**
  (`aarch64`/`x86_64-apple-darwin`, `aarch64`/`x86_64-unknown-linux-musl`) and
  fix the feature set so the byte-compare is platform-independent. Note
  `deny.toml`'s `[graph].targets` lists **five** — those four plus
  `x86_64-unknown-linux-gnu`, its CI dev graph — so pin the four shipped, not
  deny's full set, since only the shipped closure is distributed.
- `license-checker-rseidelsohn --production` walks `node_modules` (no network),
  over-approximating the bundled set — the accepted safe direction.
- The upload derivation defaults `tree_tokens=()` (`github.py:261`), so an
  unconditional `uploads.append(ATTRIBUTION_ARTEFACT_STAGED)` is seen by both guards
  (guard 1 at default, guard 2 with `tree_tokens=TREE_ARTEFACTS`).

## Desired End State

`licenses/accelerator-third-party-notices.txt` is a checked-in, generator-
reproducible superset of both distributed closures, carrying each component's
verbatim licence text and copyright, and for every MPL-2.0 component an explicit
§3.2 corresponding-source statement resolving to the crate's repository and
crates.io source. `mise run notices:update` regenerates it byte-for-byte from
`cargo-about` over `cli/` (targets pinned to the four shipped release triples)
and `license-checker-rseidelsohn` over the frontend production tree.
`mise run notices:check` fails on drift; it is wired into `check` and `default`
and, because CI runs no aggregate `check`, executed by a dedicated
`check-attribution` job that provisions both closures. The file is staged into
`dist/release/accelerator-third-party-notices.txt` by the release-prepare lanes,
appended to `_release_uploads()` unsigned, and its presence is asserted by both
coverage guards. `deny.toml`'s `uluru` comment references the shipped artefact.
`mise run` (bare default) exits 0.

Verification: `mise run notices:check` is green after `mise run notices:update`
with no working-tree change; `mise run` exits 0; the release-prepare lane stages
the file and `upload_and_verify_release` finds it.

## What We're NOT Doing

- No detached signature (`.minisig`) or `_release_reverifies()` entry for the
  notices file. It is not a trust anchor — the launcher resolves nothing against
  it — and it still gets SLSA provenance via the `accelerator-*` attest glob.
- No `TREE_ARTEFACTS` registration, no `TreeSpec`, no `pins.toml` digest, no
  `Artifact` literal widening.
- No workflow (`main.yml`) edits — the `accelerator-` prefix keeps the file
  inside the existing provenance glob.
- No manifest (`manifest.json`) entry — the file is neither a binary nor a tree
  artifact, so no manifest-coherence gate touches it.
- No bundle-exact JS licence plugin (Vite/Rollup) rendering the notices from the
  built bundle. The production `node_modules` superset stays the render source —
  a licence plugin would force `build:frontend` before the check and push it out
  of the fast `check` lane. But the superset is *guarded* against the unsafe
  omission direction: a `default`-lane test asserts every runtime import in the
  built `dist/` bundle resolves to a package in the `--production` closure, so a
  runtime dependency mis-declared under `devDependencies` (which the bundle would
  ship but `--production` would omit) fails a check rather than shipping
  unattributed. See Phase 1 §7.
- No hand-curation of the closure. Over-inclusion (listing components a binary
  dead-code-eliminates) is the safe, accepted direction.

## Implementation Approach

Two hermetic generators, one folded file, one byte-compare drift check —
modelled exactly on the `public_api` update/check pair. `cargo-about` renders
the Rust section from an `about.hbs` template; `license-checker-rseidelsohn`
emits JSON that a Python renderer turns into a matching frontend section;
`tasks/notices.py` folds both, sorted, into the committed file and byte-compares
on `check`. Staging copies the committed file into `dist/release/` and
`_release_uploads()` appends it, so both existing guards see it through the one
derivation.

Test-driven throughout: the fold/render logic in `tasks/notices.py` is pure and
fixture-tested; `test_mise.py`'s parametrised gate test drives the `check`
wiring red-first; the positive upload-presence assertions in `test_build.py` and
`test_workflows.py` are written before the `_release_uploads()` append.

Three phases, mergeable in sequence (1 → 2 → 3), each leaving `mise run` green.
Phase 2 references the path constants and committed artefact introduced in
Phase 1, so it must not land ahead of it; only Phase 3 (docs) is order-free.

---

## Phase 1: Generators, Artefact, and Drift Check

### Overview

Pin both generators, add their config, write `tasks/notices.py` (fold + byte-
compare), generate and commit the artefact, and wire `notices:check` into the
CI-mirror lanes. Ends with a generated, guarded file that is not yet in the
release.

### Changes Required:

#### 1. Tool pins

**File**: `mise.toml`
**Changes**: Add `cargo-about` under `[tools]` (ubi backend, exact pin with a
justifying comment matching the `minisign`/`cosign` style). Regenerate
`mise.lock` via `mise lock --platform linux-x64,macos-arm64,macos-x64` and
commit the result. The regenerated entry records `version`/`backend`/`options`
only — `ubi:` backends carry no per-platform checksum — so this is
version-not-hash pinning; do not expect a `sha256` line as the aqua tools have.

```toml
"ubi:EmbarkStudios/cargo-about" = "0.9.2"
```

**File**: `cli/visualiser/frontend/package.json`
**Changes**: Add `license-checker-rseidelsohn` to `devDependencies` and a script
that emits production-only JSON with verbatim text and copyright.

```jsonc
"devDependencies": {
  "license-checker-rseidelsohn": "5.0.1"
},
"scripts": {
  "licenses:generate": "license-checker-rseidelsohn --production --json --relativeLicensePath --customPath license-format.json"
}
```

After editing `package.json`, run `npm --prefix cli/visualiser/frontend install`
and commit the regenerated `cli/visualiser/frontend/package-lock.json`.
`deps:install:node` runs `npm ci`, which aborts if the lock is out of sync with
`package.json`; the lock is also the only integrity pin for the new tool's
transitive install surface.

**File**: `cli/visualiser/frontend/license-format.json`
**Changes**: The `--customPath` field template selecting the fields the fold
renders. `repository` is included so the frontend section can name each
component's source location alongside its licence text.

```json
{
  "name": "",
  "version": "",
  "licenses": "",
  "repository": "",
  "copyright": "",
  "licenseText": ""
}
```

#### 2. cargo-about config and template

**File**: `cli/about.toml`
**Changes**: Config co-located with `deny.toml`/`clippy.toml`/`rustfmt.toml`/
`pup.ron`. Three things beyond the accepted list:

- **`targets`** pinned to the four **shipped** release triples
  (`aarch64`/`x86_64-apple-darwin`, `aarch64`/`x86_64-unknown-linux-musl`), so
  the resolved closure — and therefore the rendered bytes — is host-independent
  (the byte-compare runs on macOS locally and Linux in CI). `deny.toml` lists a
  fifth (`x86_64-unknown-linux-gnu`, its CI dev graph); exclude it — only the
  shipped closure is distributed.
- **`accepted`** = every SPDX id `deny.toml`'s allow-list carries plus the
  `uluru` MPL-2.0 exception, so the Rust section supersets the linked closure.
- **Build/dev-dependency scope.** `deny.toml`'s allow-list is pruned to exactly
  the *linked* closure; cargo-about scans a broader manifest set (build scripts,
  proc-macros, and — unless filtered — dev deps), any of which may carry an SPDX
  id outside the pruned list and make `cargo about generate` hard-error. Scope
  cargo-about to the shipped closure (exclude dev/build-only deps via its config,
  matching deny's traversal) so an unrelated build-dep licence neither aborts
  generation nor pads the notice. Reconcile the final `accepted` list against
  cargo-about's actual manifest output during implementation.

```toml
accepted = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Zlib",
    "Unicode-3.0",
    "CDLA-Permissive-2.0",
    "CC0-1.0",
    "MPL-2.0",
]
```

**File**: `cli/about.hbs`
**Changes**: A Handlebars template rendering one deterministic block per crate,
sorted by `name@version` (the same key the frontend renderer sorts by, so the
two halves are uniform). Each block carries: name, version, SPDX id, the crate's
**repository/source URL** (cargo-about exposes crate metadata including the
repository), copyright, and verbatim licence text. For any crate whose licence
is **MPL-2.0**, the block additionally emits an explicit **§3.2
corresponding-source statement** naming where source is obtained — the crate's
repository URL plus the immutable crates.io tarball
(`https://crates.io/api/v1/crates/<name>/<version>/download`, the direct source
endpoint rather than the JS-rendered info page) — so reproducing the licence
body is not mistaken for discharge. The block shape (and
this ordering of fields) is the shared contract with the Python frontend
renderer. Document it on the two artefacts that actually emit blocks (the
`about.hbs` header comment and the `_render_frontend` docstring), not on `_fold`
(which only concatenates). Enforce it not only by hand-authored fixtures — which
pin the mimic, not the template — but by the structural assertions over the
**committed artefact** (the real `about.hbs` output): every block has a source
URL and a non-empty licence body, and MPL blocks carry §3.2. `about.hbs` already
emits finished block text, so keep `_render_rust` genuinely thin (a validate/trim
pass) or fold it into `_run_cargo_about`; do not invent a vacuous seam whose only
test is a fixture mirroring the template.

#### 3. Fold and drift check

**File**: `tasks/notices.py`
**Changes**: An `update()`/`check()` pair after the `public_api.py` model, with
the impure generator invocations kept separate from the pure rendering so the
pure parts are fixture-testable without a live toolchain. `check()` verifies the
committed file exists **before** paying the dual-generator cost (matching
`public_api.check`'s ordering), then byte-compares, raising
`invoke.Exit(..., code=1)` naming `mise run notices:update`. Each generator's
exit status is checked and raises a *distinct* Exit before any byte comparison,
so a generator/config failure is not misreported as drift.

```python
from invoke import Context, Exit, task

from tasks.shared.paths import CLI_DIR, FRONTEND, ATTRIBUTION_ARTEFACT


def _render(context: Context) -> str:
    rust = _render_rust(_run_cargo_about(context))
    frontend = _render_frontend(_run_license_checker(context))
    return _fold(rust, frontend)


@task
def check(context: Context) -> None:
    """Pin the third-party notices file against a fresh dual-generator render."""
    if not ATTRIBUTION_ARTEFACT.exists():
        raise Exit(
            f"{ATTRIBUTION_ARTEFACT} is missing — run "
            "`mise run notices:update`",
            code=1,
        )
    if _render(context) != _read(ATTRIBUTION_ARTEFACT):
        raise Exit(
            f"{ATTRIBUTION_ARTEFACT} has drifted from the dependency "
            "graphs — run `mise run notices:update`",
            code=1,
        )


@task
def update(context: Context) -> None:
    """Regenerate the third-party notices file from both dependency graphs."""
    ATTRIBUTION_ARTEFACT.parent.mkdir(parents=True, exist_ok=True)
    _write(ATTRIBUTION_ARTEFACT, _render(context))
```

The functional-core / imperative-shell split:

- **`_run_cargo_about(context) -> str`** (impure) runs
  `cargo about generate --frozen about.hbs` with `context.cd(CLI_DIR)` (config
  resolves relative to cwd, the `deny.py:13` idiom), checks `result.exited`, and
  raises `Exit("cargo-about failed to render the Rust closure …", code=1)` on a
  non-zero exit. Returns raw stdout.
- **`_run_license_checker(context) -> str`** (impure) runs
  `npm --prefix {FRONTEND} run licenses:generate`, checks exit status the same
  way, and returns raw stdout JSON.
- **`_render_rust(raw) -> str`** and **`_render_frontend(payload) -> str`**
  (pure) take the raw generator output and produce the section text — the
  frontend one JSON-decodes, selects fields, and sorts by `name@version`. These
  are what `test_notices.py` fixture-drives; neither shells out.
- **`_fold(rust, frontend) -> str`** (pure) concatenates a fixed header, the
  Rust section, and the frontend section, and **normalises line endings to LF
  with a single trailing newline** so verbatim upstream `licenseText` carrying
  CRLF or a stray trailing newline cannot cause spurious byte-compare drift. Its
  docstring documents the shared per-block contract (see §2).
- **`_read`/`_write`** read and write the artefact in a newline-stable way
  (`newline=""`, explicit UTF-8) so `read_text()`'s universal-newline
  translation cannot diverge the committed bytes from the freshly rendered
  string.

The header is a fixed preamble naming the file, stating it is the third-party
attribution notice for the distributed Rust binaries and the embedded frontend
bundle, and that MPL-2.0 components carry a §3.2 corresponding-source statement
below their entry. The verbatim string is chosen at implementation and pinned by
the committed artefact and the `test_notices.py` fixtures (the plan fixes its
content, not its literal wording). The §3.2 clause of the preamble is accurate
only if the frontend section also emits the statement for any MPL-2.0 npm
package: the current production closure is MIT/BSD (no copyleft), so none arises
today — but specify that `_render_frontend` mirrors the Rust §3.2 emission (or
that the closure is asserted copyleft-free) so the blanket header claim stays
true if a future MPL-2.0 npm dep enters.

**File**: `.gitattributes`
**Changes**: Pin the artefact to LF
(`licenses/accelerator-third-party-notices.txt text eol=lf`) so git never
normalises the committed bytes away from what `_fold()` renders.

**File**: `tasks/shared/paths.py`
**Changes**: The committed-artefact path constant (the staged path lands in
Phase 2).

```python
ATTRIBUTION_ARTEFACT = REPO_ROOT / "licenses" / (
    "accelerator-third-party-notices.txt"
)
```

**File**: `licenses/accelerator-third-party-notices.txt`
**Changes**: Generated by `mise run notices:update` and committed.

#### 4. Task wiring

**File**: `mise.toml`
**Changes**: Register `notices:check` and `notices:update` with explicit
`depends` on **both** closures they read — `deps:install:node` (for
`license-checker` over `node_modules`) and a `deps:install:cargo-sources` warmer
(for the offline `cargo about` read of the `cli/` registry) — so provisioning is
declared on the task, symmetric across both closures, never left to sibling-gate
ordering (the frontend-task convention at `mise.toml:595-615`). Add
`deps:install:cargo-sources` (runs `cargo fetch --locked` from `CLI_DIR`). Add
`notices:check` to `check.depends` and `default.depends` beside `public-api:check`.

```toml
[tasks."deps:install:cargo-sources"]
description = "Populate the cli/ cargo registry so offline cargo-about can read licence files"
run = "invoke deps.install-cargo-sources"

[tasks."notices:check"]
description = "Pin the third-party notices artefact against a fresh dual-generator render (cargo-about + license-checker)"
depends = ["deps:install:node", "deps:install:cargo-sources"]
run = "invoke notices.check"

[tasks."notices:update"]
description = "Regenerate the third-party notices artefact from the cli/ closure and the frontend production tree"
depends = ["deps:install:node", "deps:install:cargo-sources"]
run = "invoke notices.update"
```

`check.depends` (`mise.toml:640`) and `default.depends` (`mise.toml:644`) each
gain `"notices:check"`. Both closures are now declared task edges, so a
fresh-clone `mise run check` provisions them rather than racing sibling gates.

⚠️ **Verify `cargo fetch` extracts sources.** `cargo fetch` populates the
registry index and the `.crate` tarball cache, but extraction into
`registry/src/` is normally lazy (build-time). cargo-about reads per-crate
LICENSE/COPYRIGHT files from the *extracted* source for crates whose text is not
synthesised from the SPDX store. During implementation, confirm `cargo fetch`
alone satisfies `cargo about generate --frozen`; if not, either force extraction
in `deps:install:cargo-sources`, or drop `--offline` (use `--locked` alone) so
cargo-about extracts from the local `.crate` cache without network —
determinism is preserved by `--locked`, and the registry stays local.

**File**: `.github/workflows/main.yml`
**Changes**: CI runs no aggregate `mise run check` — each `*:check` is its own
job, and none provisions both closures. Add a dedicated `check-attribution` job
(a new job, orthogonal to the three attest/provenance blocks — no attest-block
edits) that provisions both and runs the gate, mirroring `check-architecture`'s
rust-job scaffolding **completely**:

- **Route the rust toolchain** — set `RUSTUP_HOME=$HOME/.local/share/mise/rustup`
  before `mise install`, exactly as every cargo-running job does. Without it, on
  a cache-hit run `mise install` no-ops while the toolchain lives outside the
  cache and cargo is absent (or parallel passes race: `detected conflict:
  bin/cargo`). Not optional.
- `mise-action` install with its **own** `cache_key_prefix: mise-attribution-v1`
  so it never shares a toolchain-cache namespace with a non-rust job.
- `Swatinem/rust-cache` with `workspaces: cli`.
- `cargo fetch --locked` from `cli/` (see the extraction caveat above).
- `mise run notices:check` (its `deps:install:node` edge runs `npm ci`, so the
  guard reads the locked tree).

**Add `check-attribution` to the `prerelease` job's `needs:` list**
(`main.yml:565-580`, which already enumerates every other `check-*`/`test-*`
job), so the drift gate **blocks the release pipeline** — a parallel job alone
does not gate a release. Branch protection's required-status-check list must
also name the new job to gate merges; call that GitHub-settings change out
explicitly, since a required check absent from the list is silently skipped.

**File**: `tasks/README.md`
**Changes**: Three edits, each against text verified to exist:

- Extend the standalone-gate enumeration (`deny:check`, `pup:check`,
  `public-api:check`) to include `notices:check`/`notices:update`.
- Extend the licence-mechanism paragraph (the `deny.toml` allow/exceptions
  discussion, ~lines 165-169) to note that a licence bump now also requires
  regenerating the notices artefact via `mise run notices:update`. (Do **not**
  phrase this as removing a "no discharge exists" statement — that assertion
  lives in the `deny.toml` comment, not the README.)
- Add a `check-attribution` → `mise run notices:check` row to the CI-job→command
  table (~lines 706-717), matching how `check-supply-chain`/`check-architecture`
  are listed, so a red job has a documented local reproduction command.

#### 5. Gate placement guard

**File**: `tests/unit/tasks/test_mise.py`
**Changes**: Add `"notices:check"` to `_CHECK_GATES` (line 19), so
`test_gate_wired_into_check` proves it stays in `check.depends`.

#### 6. Fold/render unit tests

**File**: `tests/unit/tasks/test_notices.py`
**Changes**: New. Because the pure renderers are now separated from the impure
runners, every case below drives pure functions over fixtures — no live
toolchain, no `node_modules`.

- **`_fold()` and `_render_frontend()`** over fixtures with a **deliberately
  out-of-order** crate/package set and a licence body containing **multiple
  lines and quotes**, asserting the output is sorted by `name@version`,
  header-prefixed, and preserves the licence **content** (assert the LF-form
  substring, since `_fold` normalises line endings — the guarantee is
  content-preserved, endings-normalised, not byte-for-byte "unmodified").
- **`_fold()` LF normalisation**: a fixture whose `licenseText` carries CRLF and
  a trailing blank line, asserting the folded output contains only LF and
  exactly one trailing newline (the cross-platform byte-stability mechanism —
  otherwise untested by anything but a single same-platform round-trip).
- **`_render_rust()`** over a fake cargo-about-rendered fixture: assert the block
  shape matches the frontend block shape (the shared contract), an MPL entry
  carries the §3.2 source statement and a resolvable source URL, and a
  non-MPL entry does not.
- **Generator-failure branches**: over a `MagicMock(spec=Context)` whose `run`
  returns `exited=1` (the `test_deps.py` runner idiom), assert `_run_cargo_about`
  and `_run_license_checker` each raise an `Exit` **naming the failed generator**
  — distinct from the drift `Exit`'s `notices:update` message — so a tool failure
  is never misreported as drift.
- **`_render_frontend()` edge cases**: missing `licenseText`, missing
  `copyright`, an empty payload, and the same package name at two versions —
  asserting the renderer's defined behaviour (not a `KeyError` or a blank
  block) for each.
- **`check()` isolation and branches**: monkeypatch `_render` to a fixed string
  and point `ATTRIBUTION_ARTEFACT` at `tmp_path`; assert (a) match → no raise,
  (b) drift → `Exit` naming `notices:update`, (c) **missing file** → `Exit`
  naming `notices:update`. No real generator runs.
- **§3.2 regression over the committed artefact**: parse the committed
  `licenses/accelerator-third-party-notices.txt` and assert **every** block whose
  SPDX id is `MPL-2.0` carries a corresponding-source URL (not just `uluru` by
  name) — so a template edit that drops §3.2 for the known crate *or* a future
  MPL entry fails CI without invoking the generator.

#### 7. Frontend bundled-import guard

**Changes**: A guard asserting the built `dist/` bundle ships no module that
`--production` would omit — so a runtime dependency mis-declared under
`devDependencies`, which Vite bundles but `--production` excludes from the
notices, fails a check rather than shipping unattributed. This is the sole guard
against the unsafe omission direction, so its **lane and enumeration source both
matter**:

- **Host it where the bundle is built and CI already gates it.** It needs
  `build:frontend`, so make it a task with `depends = ["build:frontend"]`
  aggregated into `test:unit` (via the `test:unit:visualiser`/`test:unit:frontend`
  group that already depends on `build:frontend`, `mise.toml:259-261`). That
  runs it in the existing `test-unit` CI job, which is already in the
  `prerelease` `needs:` list — so the guard actually executes and gates a
  release. Do **not** leave it under `tests/unit/tasks/`, which
  `test:unit:tasks` collects with only `deps:install:python` and no built bundle.
- **Enumerate from a structured source, not bundled text.** Read the third-party
  module set from Rollup/Vite's module graph or the build manifest/sourcemap —
  a regex over minified output can silently under-enumerate and false-pass.
- **Fail loud on a missing `dist/`**, never skip: an absent bundle must error,
  not vacuously pass.
- **Include a negative fixture** proving the guard fails when a package present
  in the bundle graph is absent from the `--production` closure — otherwise the
  guard's own correctness is untested.

### Success Criteria:

#### Automated Verification:

- [ ] Notices task registered: `mise run notices:update` writes
      `licenses/accelerator-third-party-notices.txt`
- [ ] Drift check is idempotent: `mise run notices:update && mise run notices:check`
      exits 0 with no working-tree change (`git status --porcelain` empty)
- [ ] Gate placement guard passes: `uv run pytest tests/unit/tasks/test_mise.py`
- [ ] Fold/render unit tests pass: `uv run pytest tests/unit/tasks/test_notices.py`
- [ ] §3.2 regression passes: the committed artefact carries an `uluru`/`MPL-2.0`
      entry with a corresponding-source URL (asserted in `test_notices.py`)
- [ ] Bundled-import guard passes: `uv run pytest tests/unit/tasks/test_frontend_licenses.py`
      (every bundled module resolves to the `--production` closure)
- [ ] Build-system checks pass: `mise run build-system:check`
- [ ] Aggregate read-only gate passes: `mise run check`
- [ ] `mise.lock` regenerated and committed (no drift on re-run of `mise lock`)
- [ ] `cli/visualiser/frontend/package-lock.json` regenerated and committed;
      `npm ci` succeeds against it
- [ ] Cross-platform determinism: `notices:update` on macOS and a Linux
      `notices:check` (or CI's `check-attribution` job) agree byte-for-byte
      (targets pinned in `about.toml`, LF-normalised)

#### Manual Verification:

- [ ] The Rust section supersets `deny.toml`'s allow-list reconciled with
      `cargo-about`'s manifest output; `nm -a` symbol counts confirm the `uluru`
      MPL sub-closure appears
- [ ] Sampling one component per licence family shows verbatim licence text and
      the copyright line, not merely the SPDX id
- [ ] Each MPL-2.0 component carries a §3.2 source-availability statement
      resolving to an obtainable source
- [ ] The frontend section includes `highlight.js` (BSD-3-Clause) and the
      react/tanstack/dnd-kit/remark/rehype transitive set

---

## Phase 2: Release Staging, Upload, and Coverage Guards

### Overview

Stage the committed artefact into `dist/release/`, append it to the upload set
unsigned, add the two positive presence assertions, and update the `uluru`
exception comment. Ends with the artefact shipping and both guards pinning it.

### Changes Required:

#### 1. Staged-path constant

**File**: `tasks/shared/paths.py`
**Changes**: The staged upload path (flat in `dist/release/`, `accelerator-`
prefix), modelled on `RELEASE_MANIFEST`.

```python
ATTRIBUTION_ARTEFACT_STAGED = RELEASE_STAGING / "accelerator-third-party-notices.txt"
```

#### 2. Staging step

**File**: `tasks/build.py`
**Changes**: A task copying the committed source into the staging tree, mirroring
the `server_cross_compile` staging idiom (`build.py:382,392`).

```python
@task
def stage_notices(context: Context) -> None:
    """Stage the committed third-party notices artefact into dist/release/."""
    RELEASE_STAGING.mkdir(parents=True, exist_ok=True)
    shutil.copy2(ATTRIBUTION_ARTEFACT, ATTRIBUTION_ARTEFACT_STAGED)
```

**File**: `tasks/release.py`
**Changes**: Call `build.stage_notices(context)` in both `prerelease_prepare`
and `release_prepare`, alongside the existing `build.frontend`/cross-compile
staging.

#### 3. Upload-set append

**File**: `tasks/github.py`
**Changes**: Append the staged path in `_release_uploads()` after
`RELEASE_MANIFEST_SIG` (`github.py:274`). Unsigned — no `_sig`, no
`_release_reverifies()` entry.

```python
uploads.append(RELEASE_MANIFEST)
uploads.append(RELEASE_MANIFEST_SIG)
uploads.append(ATTRIBUTION_ARTEFACT_STAGED)
```

#### 4. Coverage guards

**File**: `tests/unit/tasks/test_build.py`
**Changes**: New positive presence test beside the existing negative guard.

```python
def test_the_attribution_artefact_is_a_release_upload(self) -> None:
    from tasks.github import _release_uploads

    names = {path.name for path in _release_uploads()}
    assert "accelerator-third-party-notices.txt" in names
```

**File**: `tests/unit/tasks/test_workflows.py`
**Changes**: Assert the artefact is among the published set (so the existing
per-path glob loop then proves the `accelerator-*` glob covers it).

```python
def test_the_attribution_artefact_is_published_and_attested(wf):
    from tasks.github import _release_uploads
    from tasks.shared.paths import TREE_ARTEFACTS

    published = [
        path.name
        for path in _release_uploads(tree_tokens=TREE_ARTEFACTS)
    ]
    assert "accelerator-third-party-notices.txt" in published
```

**File**: `tests/integration/tasks/test_release.py`
**Changes**: Extend the **existing** `TestPrereleasePrepare` harness (and a
`release_prepare` analogue) to assert each prepare lane invokes `stage_notices`.
This must live in the integration suite, not a new unit test: both prepare lanes
orchestrate a long chain of real side-effecting collaborators (`version.bump`
which mutates `plugin.json`/`Cargo.toml`/`Cargo.lock`, `git.pull`,
`changelog.release`, `marketplace.update_*`, `build.frontend`,
`build.*_cross_compile`, `assert_staged_launcher_versions`,
`_assert_assembled_matches_pins`), and `TestPrereleasePrepare._setup` already
mocks every one of them to make the lane callable. Add `stage_notices` to that
mock set and assert it is called, mirroring `test_asserts_staged_launcher_versions`.
A lane driven with only `stage_notices` stubbed (as a naïve unit test would) runs
a real version bump and cross-compile — it will not run green.

```python
def test_prerelease_prepare_stages_the_attribution_artefact(self, mocker):
    self._setup(mocker)
    staged = mocker.patch.object(build, "stage_notices")
    release.prerelease_prepare(self.context)
    staged.assert_called_once_with(self.context)
```

Add the matching `release_prepare` case (its `_setup` additionally mocks
`changelog.release`/`marketplace.update_*`). Both prove the file is *staged*, not
merely *listed* — closing the gap where a dropped `stage_notices` passes every
enumeration guard and fails only at release time.

#### 5. Exception comment

**File**: `cli/deny.toml`
**Changes**: Rewrite the two clauses that assert no discharge artefact exists,
precisely — not over-claiming the file is signed (it is not) and not calling it
a discharge unless it names obtainable source (it does, via per-crate URLs).

Replace the clause at lines 70-71 ("… which the release upload set carries no
artefact for.") with:

```text
# to obtain the source. The release upload set carries
# accelerator-third-party-notices.txt, which lists each MPL-2.0 component with a
# §3.2 corresponding-source statement naming its repository and crates.io
# source; that discharges the obligation.
```

Replace the sentence at lines 88-89 ("… and a third-party attribution artefact
is owed by the release upload set.") with:

```text
# So the notice obligation is live today for five shipped binaries, discharged
# by accelerator-third-party-notices.txt in the release upload set. That file
# ships unsigned — it is not a trust anchor — but rides the dist/release/
# accelerator-* SLSA provenance glob like every other published asset.
```

#### 6. British-spelling rename of `TREE_ARTIFACTS`

**Files**: `tasks/shared/paths.py` (definition) and every reference —
`tasks/github.py` (`_tree_artifact_uploads`, `_release_uploads`),
`tasks/release.py` (`_assert_staged_manifest_is_current`),
`tests/unit/tasks/test_workflows.py`, `tests/unit/tasks/test_build.py`.
**Changes**: Rename the constant `TREE_ARTIFACTS` → `TREE_ARTEFACTS` so the new
`ATTRIBUTION_ARTEFACT`/`ATTRIBUTION_ARTEFACT_STAGED` constants do not sit beside
an American-spelled sibling in `paths.py`. A pure identifier rename — no
behavioural change; the existing `test_workflows.py`/`test_build.py` guards prove
it out. The tuple's string tokens (`"driver"`, `"browser"`) and the wider
tree-artifact identifier family (`_tree_artifact_uploads`,
`tree_artifact_asset_path`, `TREE_ARTIFACT_DESCRIPTIONS`, `ARTIFACT_EXECUTABLES`,
the `Artifact` literal) are **out of scope** here — renaming only the constant
this plan references keeps the change bounded; a full spelling sweep is a
separate task.

### Success Criteria:

#### Automated Verification:

- [ ] Name-set presence guard passes: `uv run pytest tests/unit/tasks/test_build.py`
- [ ] Attest-glob presence guard passes: `uv run pytest tests/unit/tasks/test_workflows.py`
- [ ] Staging-wiring guard passes: both prepare lanes invoke `stage_notices`
      (`uv run pytest tests/integration/tasks/test_release.py`)
- [ ] cargo-deny still green after the comment edit: `mise run deny:check`
- [ ] Build-system checks pass: `mise run build-system:check`
- [ ] Aggregate read-only gate passes: `mise run check`

#### Manual Verification:

- [ ] A local `mise run prerelease` (outside CI) stages
      `dist/release/accelerator-third-party-notices.txt` and
      `upload_and_verify_release`'s existence assertion finds it
- [ ] The three attest blocks in `main.yml` are unchanged (the `accelerator-*`
      glob already covers the file)
- [ ] The `uluru` comment no longer asserts the upload set carries no MPL
      artefact

---

## Phase 3: Rationale on the Work Item

### Overview

Record the chosen generated, dual-generator rationale on 0203 (AC bullet 5).
Pure documentation.

### Changes Required:

#### 1. Rationale note

**File**: `meta/work/0203-ship-a-third-party-attribution-artefact-with-the-release.md`
**Changes**: Add a section (or extend Drafting Notes) recording:

- **Two generators** because the Cargo and npm graphs never meet.
- **Generated over hand-maintained** because the manifest over-approximates the
  linked closure — the safe direction (over-inclusion harmless, omission the
  violation).
- **`ubi:` rather than aqua** for `cargo-about` because it is absent from the
  aqua registry, accepting version-not-hash pinning (like `minisign`/`cosign`).
  Containment is **job isolation**, not the drift check: a substituted binary
  could emit byte-identical notices yet run arbitrary code, so what bounds the
  blast radius is that `check-attribution` is read-only and carries no signing
  keys or write-scoped secrets — confirm that when adding the job. Treat
  cargo-about as a fourth accepted unverified surface and note it beside the
  cargo-pup/cargo-public-api carve-out in `mise.toml [settings]`.
- **Tamper-detection for the unsigned artefact is SLSA provenance**, not a
  `_release_reverifies()` entry — recorded here so a future reader does not read
  the missing re-verify as an oversight. The durable crates.io/repository source
  anchors make a swapped §3.2 pointer implausible to weaponise.
- **`license-checker-rseidelsohn`** over the abandoned original.
- **Hermetic generation** (`--frozen` / `--production`) so the drift check stays
  in the fast `check` lane — with the tradeoff that the byte-compare couples the
  committed file to each tool's exact output, so a deliberate tool-version or
  `package-lock.json` bump is expected to require a `notices:update`.
- **§3.2 discharged by per-crate source URLs** (repository + crates.io) rather
  than a hosted mirror or written offer: cheapest form that resolves to
  obtainable source, byte-stable, no infrastructure to maintain.
- **`--production` node_modules guarded, not bundle-rendered**: keeps the render
  in the fast lane while a `default`-lane bundled-import guard closes the unsafe
  omission direction (a runtime dep mis-declared as a devDependency).
- **Unsigned** because the file is not a trust anchor and rides the
  `accelerator-*` provenance glob.
- **Dedicated `check-attribution` CI job** because CI runs no aggregate `check`
  and no existing job provisions both the cargo registry and `node_modules`.

### Success Criteria:

#### Automated Verification:

- [ ] Work-item frontmatter still validates:
      `bin/accelerator corpus frontmatter validate --file meta/work/0203-ship-a-third-party-attribution-artefact-with-the-release.md`

#### Manual Verification:

- [ ] AC bullet 5 ("rationale for the chosen generated, dual-generator approach
      is documented on this item") reads as satisfied

---

## Testing Strategy

### Unit Tests:

- `test_notices.py`: pure `_render_rust()`/`_render_frontend()`/`_fold()` over
  fixtures (out-of-order input, multi-line verbatim text, missing
  `licenseText`/`copyright`, empty payload, duplicate name@version) → expected
  sorted, verbatim, header-prefixed text; the MPL §3.2 source statement present
  on MPL entries only; `check()` isolated via monkeypatched `_render` + `tmp_path`
  raises `Exit` on drift and on a missing file, passes on a match; and a
  committed-artefact substring guard for the `uluru`/`MPL-2.0` source URL.
- `test_frontend_licenses.py`: every module in the built `dist/` bundle resolves
  to the `--production` closure (bundled-import guard, `default` lane).
- `test_mise.py`: `notices:check` in `_CHECK_GATES` proves `check.depends`
  membership.
- `test_build.py`: positive presence of the artefact in `_release_uploads()`.
- `test_workflows.py`: artefact in the published set and matched by the
  `accelerator-*` glob.
- `test_release.py` (integration): both prepare lanes invoke `stage_notices`,
  extending the existing `TestPrereleasePrepare` harness (staging-wiring spy).

### Integration Tests:

- `mise run notices:update && mise run notices:check` round-trips with no
  working-tree change (byte reproducibility, the AC's generator-reproducibility
  criterion).
- `mise run check` and the bare `mise run` exit 0 end-to-end.

### Manual Testing Steps:

1. Run `mise run notices:update`; inspect the file for verbatim licence text and
   copyright per component, the §3.2 statement on MPL components, and the
   frontend section (react/tanstack/dnd-kit/remark/rehype + `highlight.js`).
2. Confirm the Rust section supersets `deny.toml`'s allow-list and that
   `nm -a | grep uluru` on an unstripped `accelerator-vcs --release` build shows
   the MPL sub-closure present.
3. Run `mise run prerelease` locally and confirm the staged file appears in
   `dist/release/` and passes the pre-upload existence assertion.

## Performance Considerations

`cargo about generate --frozen` reads the local cargo registry with no network
round-trip. Locally that registry is warm from ordinary `cli/` work; in CI the
`check-attribution` job populates it with `cargo fetch --locked` before the
generate, so the offline read never fails on a cold cache (the plan does not
rely on a sibling gate having warmed it). `license-checker-rseidelsohn
--production` walks the `node_modules` provisioned by the task's
`deps:install:node` edge. Neither generator needs the built `dist/` bundle, so
the drift check stays out of `build:frontend` and in the fast lane. The one
bundle-dependent piece — the frontend bundled-import guard (Phase 1 §7) — does
need `build:frontend`, and is therefore placed in the `test:unit` group that
already builds the bundle, not in fast `check`.

The frontend byte-compare assumes the `--production` `node_modules` tree
resolves identically on the macOS generate host and the Linux check host. The
current production set (react, react-dom, `@dnd-kit/*`, `@tanstack/*`,
remark/rehype, `highlight.js`) carries no platform-specific native
`optionalDependencies`, so this holds. Record the invariant — the production
closure must stay free of platform-gated optional deps — so a future dependency
that breaks it is caught by expectation, not by a mysterious CI-only byte
mismatch.

## Migration Notes

No data migration. Two toolchain-coupling risks, both covered in Phase 1:

- The `[tools]` edit adds `cargo-about`, forcing `mise lock` regeneration across
  three platforms and a `test_mise.py` gate registration; and the new
  devDependency forces a `package-lock.json` regeneration for `npm ci`.
- CI runs no aggregate `mise run check` — each `*:check` is a separate job — so
  the drift gate does **not** ride an existing lane. The new `check-attribution`
  job is what provisions `cargo-about` (ubi) plus a warm `cli/` registry
  (`cargo fetch --locked`) plus the frontend `node_modules` (`npm ci` via the
  task's `deps:install:node` edge) and runs `notices:check`. Without that job,
  the gate never executes at pipeline time even though it is in `check.depends`.

## References

- Original work item: `meta/work/0203-ship-a-third-party-attribution-artefact-with-the-release.md`
- Related research: `meta/research/codebase/2026-08-31-0203-third-party-attribution-artefact.md`
- Upload-set derivation: `tasks/github.py:258-277`
- Single-file upload template: `tasks/shared/paths.py:22-24` (`RELEASE_MANIFEST`)
- Generate-and-byte-compare precedent: `tasks/public_api.py:113-159`
- Fail-on-missing-licence precedent: `tasks/shared/vendor/assemble.py:203-218`
- Name-set guard: `tests/unit/tasks/test_build.py:498-509`
- Attest-glob guard: `tests/unit/tasks/test_workflows.py:161-228`
- Gate placement guard: `tests/unit/tasks/test_mise.py:19,106-111`
- `uluru` exception comment: `cli/deny.toml:66-102`
- cargo-deny run idiom: `tasks/deny.py:6-20`
- Frontend embed: `cli/visualiser/server/src/assets.rs:69-72`
- ubi tool-pin precedent: `mise.toml:35,39`
