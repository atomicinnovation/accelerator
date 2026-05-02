---
date: "2026-04-30T22:00:00+01:00"
type: plan
skill: create-plan
ticket: null
status: draft
---

# Meta Visualiser — Phase 12: Packaging, docs, and release

## Overview

Phase 12 is the release-discipline phase. The visualiser implementation
is complete through Phase 11 — server, frontend, watcher, kanban write
path, cross-references, error handling, and tests are all green. What's
missing is the machinery to produce per-arch release binaries, attach
them to **every** GitHub Release the existing CI pipeline cuts (both
`*-pre.N` pre-releases on every merge to `main` and stable `X.Y.Z`
releases), and verify a clean install of the plugin in a fresh project
boots end-to-end.

Approach: test-driven where possible. **The release process is
implemented entirely in Python invoke tasks — no new bash scripts.**
Pure helper logic (`compute_sha256`, `update_checksums_json`,
`validate_version_coherence`, `is_prerelease_version`) lives in a new
`tasks/release_helpers.py` Python module; the network-bound
`verify_uploaded_asset` lives next to the orchestration in
`tasks/release_binaries.py`. Every helper is exercised by pytest unit
tests before any orchestration is written. The orchestration itself
extends the existing Python invoke task layer (`tasks/release.py`,
`tasks/github.py`), so the existing CI jobs (`mise run prerelease`,
`mise run release`) inherit binary builds with minimal new wiring.
Tests use `pytest-mock`'s `mocker.patch` against `invoke.Context.run`
and `subprocess.run` to suppress side effects — there is no dry-run
flag, in keeping with the project's convention against dry-run UX on
destructive ops. A binary-acquisition smoke test deferred from Phase
11 lands here, exercising the existing
`launch-server.sh` (the end-user launcher, which stays bash because
it must run on user machines without a Python toolchain) against a
local HTTP mirror. The actual first CI-driven release is one-shot
and only verifiable manually on each target platform — but everything
in the release-task graph is exercised by pytest before the first
real CI cut.

## Current State Analysis

### Already wired (do not duplicate)

- **Embedded frontend pipeline**: `server/build.rs` panics if
  `frontend/dist/index.html` is missing when `embed-dist` is enabled
  (`server/build.rs:7-23`). `rust-embed` with brotli compression embeds
  `dist/` into each release binary (`Cargo.toml:14, 33`).
- **Cargo features**: `default = ["embed-dist"]`, `dev-frontend`
  alternative for disk-based serving (`Cargo.toml:13-16`).
- **launch-server.sh fetch flow**: tri-precedence binary resolution
  (env override → config override → cached + checksum-verified or
  download from GitHub Releases). Manifest version-drift check, dev
  override via `ACCELERATOR_VISUALISER_BIN`, mirror override via
  `ACCELERATOR_VISUALISER_RELEASES_URL`
  (`scripts/launch-server.sh:101-164`). Sentinel rejection of
  uninitialised projects (`launch-server.sh:43-50`).
- **Plugin .gitignore entries**: `accelerator-visualiser-*` and
  `frontend/dist/` already gitignored (`.gitignore:13-16`).
- **Placeholder checksums.json**: `bin/checksums.json` already in
  place with `0…0` sentinels and a "fail any build that sees it" note.
- **CLI wrapper**: `cli/accelerator-visualiser` already in place,
  delegates to `launch-server.sh` with full POSIX-symlink resolution
  (`cli/accelerator-visualiser:1-30`).
- **CI pipeline** (`.github/workflows/main.yml`): three jobs on every
  push to `main` — `test` (mise run test), `prerelease` (mise run
  prerelease, every push), `release` (mise run release, every push,
  needs the `release` GitHub Environment for approval gating). All
  three run on `ubuntu-latest`.
- **Invoke task layer** (`tasks/`): the orchestration substrate that
  Phase 12 extends rather than replaces.
  - `tasks/release.py` — `prerelease()` bumps + commits + tags +
    pushes; `release()` finalises + updates marketplace + commits
    changelog + tags + pushes + creates GitHub Release + bumps to
    next minor pre.0.
  - `tasks/github.py` — `create_release()` runs
    `gh release create "<ver>" --generate-notes --title "<ver>"`. **No
    `--prerelease` flag today; only invoked from the stable
    `release()` path.**
  - `tasks/build.py` — `frontend()`, `server_dev()`, `server_release()`
    already in place. None cross-compile.
  - `tasks/version.py` — semver-aware bump logic.
- **Mise toolchain** (`mise.toml`): pins `rust = "1.90.0"`, `node = "22"`,
  `gh = "2.89.0"`. **Does not pin `zig` or `cargo-zigbuild`** — needs
  adding for the cross-compile flow.
- **CHANGELOG / README**: visualiser referenced once in CHANGELOG (the
  kanban-write entry under "Unreleased") but no README section exists.

### Still missing

- `zig` and `cargo-zigbuild` in `mise.toml` so CI runners (and dev
  hosts) can cross-compile to the four target triples.
- A binary build + upload invoke task — the Python wrapper around the
  cross-compile, hash, upload, and verify flow.
- Wiring of that task into `tasks/release.py` (both `prerelease()` and
  `release()` paths) and `.github/workflows/main.yml` (so CI does the
  upload, not a maintainer's machine).
- `--prerelease` flag in `tasks/github.py:create_release()` — derived
  from the version string via a new `tasks.release_helpers`
  Python function.
- Pure-Python helper functions for SHA-256 hashing, checksums.json
  mutation, version coherence checks, and pre-release detection
  (`tasks/release_helpers.py`); plus an asset-verification helper
  next to the orchestration in `tasks/release_binaries.py`.
- A `pytest` dev dependency group plus a `tests/tasks/` directory
  for invoke-task unit tests (the existing `tasks/test/` is an
  invoke sub-collection, not a pytest layout).
- Binary-acquisition smoke test — deferred from Phase 11.5 explicitly,
  awaiting real release infrastructure.
- README section for the `/accelerator:visualise` skill (only
  user-facing reference per the research doc).
- CHANGELOG entry for the version bump that ships the binaries.
- End-to-end verification that `/accelerator:init` →
  `/accelerator:visualise` works on a fresh clone on all four
  supported platforms after a CI-published release.

### Pre-existing concerns to honour

- `Cargo.toml` version is `0.1.0`; the plugin manifest is
  `1.19.0-pre.4`. The plan must reconcile these (the Rust binary's
  internal `X-Accelerator-Visualiser` version header and `--version`
  output should match the plugin version it ships with). This becomes
  routine once `tasks/version.py` learns to write Cargo.toml in
  addition to plugin.json.
- The existing `scripts/launch-server.sh` short-circuits on a `0…0`
  sentinel checksum with a friendly error pointing at
  `ACCELERATOR_VISUALISER_BIN`. After Phase 12 ships, this sentinel
  path is reached only in working-copy / dev mode — every released
  version (pre or stable) gets real hashes committed in the same
  commit as the version bump.
- The CI `release` job uses a GitHub Environment named `release` —
  giving it a manual approval gate. The new `prerelease` binary
  upload runs in the existing `prerelease` job (no environment, no
  approval gate) — every push to `main` produces uploaded binaries
  immediately. This is intentional: pre-release binaries are how
  internal users dogfood unreleased changes.

## Desired End State

After Phase 12 ships:

1. Every push to `main` triggers (via the existing CI workflow):
   - The `prerelease` job, which bumps to the next `*-pre.N`, builds
     four cross-compiled binaries, commits the version bump and
     `bin/checksums.json` update, tags, pushes, and creates a GitHub
     Release marked `--prerelease` with the four binaries attached.
   - The `release` job (gated by the `release` environment), which on
     manual approval finalises the version, builds four binaries,
     commits + tags + pushes, and creates a stable GitHub Release with
     binaries attached.
2. A user with a clean checkout of the plugin runs `/accelerator:init`
   then `/accelerator:visualise` in a fresh project: the binary
   downloads from the relevant Release (pre-release or stable), verifies
   against the committed manifest, launches, and the URL loads in
   their browser.
3. The smoke test that exercises this download-verify-launch flow is
   automated (against a local HTTP mirror), runs in CI alongside the
   existing test suite, and is wired into the existing
   `mise run test` task graph.
4. The README has a `/accelerator:visualise` section explaining what
   the visualiser is, how to launch it, and what a first-run download
   looks like, including a note that pre-release plugin versions get
   their own binaries (no `ACCELERATOR_VISUALISER_BIN` required for
   pre-release dogfooding).
5. The CHANGELOG has an entry for the released version describing the
   visualiser as a user-facing feature.
6. `mise.toml` pins `zig` and `cargo-zigbuild`; CI installs both via
   `jdx/mise-action` and the cross-compile step succeeds on
   `ubuntu-latest`.

### Verification

```bash
# Helpers covered by pytest unit tests (TDD seed)
mise run test:unit:tasks

# Orchestration covered by mocker.patch-based pytest integration tests
mise run test:integration:tasks

# Binary acquisition smoke test (deferred from Phase 11.5)
mise run test:integration:binary-acquisition

# Full test suite
mise run test

# After a CI prerelease publishes:
gh release view v1.20.0-pre.5 --json assets,isPrerelease
# Asserts isPrerelease=true and four assets attached.

# After a CI stable release publishes:
gh release view v1.20.0 --json assets,isPrerelease
# Asserts isPrerelease=false and four assets attached.

# In a fresh project on each of the four supported platforms:
/accelerator:init
/accelerator:visualise
# (manual: open the URL in a browser, verify the library loads)
```

## What We're NOT Doing

- **No separate maintainer-machine release path.** All releases (pre
  and stable) flow through CI. Maintainers do not run release scripts
  locally; the manual step is approving the `release` environment in
  the GitHub UI for a stable cut.
- **No Windows binaries.** Out of scope per the spec.
- **No homebrew tap, deb/rpm packaging, or `cargo install` story.** The
  binary is a private implementation detail of the plugin, not a
  general-purpose tool — discoverability happens via the plugin
  marketplace, not the system package manager.
- **No code-signing or notarisation** for the macOS binaries. The
  binaries are run by `launch-server.sh` from a path inside
  `${CLAUDE_PLUGIN_ROOT}` after SHA-256 verification, never opened by
  Finder or Gatekeeper. Notarisation cost (Apple Developer ID,
  altool/notarytool round trip per release) outweighs the benefit
  given the SHA-256 manifest already gates execution.
- **No bisecting tooling for the four-platform smoke matrix.** Each
  platform is smoke-tested manually post-release.
- **No retry / partial-resume for failed CI release jobs.** A failed
  job leaves the working tree at whatever state CI reached; the next
  push to `main` cuts a fresh `*-pre.N+1` and re-attempts. (This is
  consistent with the existing `mise run prerelease` flow today.)
- **No deletion of older pre-release binaries.** GitHub Releases
  accumulate; pruning policy can come later.

## Implementation Approach

The phase decomposes into seven sub-phases, ordered so each has a
testable deliverable and the irreversible first-CI-release is the
very last step:

1. **Phase 12.1 — Pre-release binary policy + toolchain** (decision +
   `mise.toml`): commit the policy ("every CI-cut version gets
   binaries"), add `zig` and `cargo-zigbuild` to `mise.toml`, verify
   they install on `ubuntu-latest`.
2. **Phase 12.2 — Release helper functions (TDD)**: pure-Python
   helpers for SHA-256, checksums.json mutation, version coherence,
   pre-release detection, and uploaded-asset verification — all in
   a new `tasks/release_helpers.py` module. Each lands with a
   failing pytest test first.
3. **Phase 12.3 — Binary build + upload invoke task (TDD via
   `pytest-mock`)**: a new Python invoke task in
   `tasks/release_binaries.py` that imports the helpers directly
   (no shelling out to bash) and wraps them in the strict atomic
   order D8 prescribes, callable from both prerelease and release
   flows.
4. **Phase 12.4 — Wire into existing release tasks + CI workflow**:
   call the new task from `tasks/release.py:prerelease()` and
   `tasks/release.py:release()`; add `--prerelease` flag handling to
   `tasks/github.py:create_release()`; ensure `GH_TOKEN` is available
   in the CI `prerelease` job.
5. **Phase 12.5 — Binary acquisition smoke test**: cargo integration
   test that stands up a tiny HTTP mirror, points
   `ACCELERATOR_VISUALISER_RELEASES_URL` at it, runs
   `launch-server.sh`, asserts the binary downloads, verifies,
   launches, and serves `/api/types`.
6. **Phase 12.6 — Documentation**: README section + CHANGELOG entry.
7. **Phase 12.7 — First CI release**: merge the work, observe the
   first CI prerelease cut, smoke-test on all four platforms,
   approve the first stable release.

---

## Phase 12.1: Pre-release binary policy + toolchain

### Overview

Commit the policy decision (resolving Gap 7 from the research
follow-up) and add the cross-compile toolchain to `mise.toml` so CI
runners — and any dev host running `mise install` — can build all
four target triples.

### Decision

**Every CI-cut release gets the full binary build pipeline.** The
existing CI pipeline already cuts `*-pre.N` on every push to `main` and
optionally cuts `X.Y.Z` stables; both flows now produce four
cross-compiled binaries and upload them to a GitHub Release.

The only difference between the two flows on the GitHub side is the
`--prerelease` flag passed to `gh release create`, which:

- Marks the Release as pre-release in the GitHub UI.
- Excludes it from the "Latest release" calculation.
- Does **not** prevent users with the right URL or
  `ACCELERATOR_VISUALISER_RELEASES_URL` mirror from downloading the
  assets.

Pre-release detection runs entirely off the version string (semver
core followed by a `-` suffix), via the
`tasks.release_helpers.is_prerelease_version` Python function added
in Phase 12.2.

The `0…0` sentinel in `bin/checksums.json` becomes a
working-copy-only state — it lives only between version bumps, on dev
machines, before the binaries for the new version exist. Every commit
that bumps the plugin version also lands real SHAs in the same commit
(the binary build runs before the version-bump commit; see Phase
12.3's atomic flow ordering).

### Changes Required

#### 1. Plan-level commitment

This subsection. Documented for downstream phases to consume.

#### 2. `mise.toml` toolchain additions

**File**: `mise.toml`

Add to the `[tools]` section:

```toml
[tools]
uv = "0.11.6"
python = "3.14.4"
gh = "2.89.0"
rust = "1.90.0"
node = "22"
zig = "0.13.0"
cargo-zigbuild = "0.19.5"
```

Pin both versions; the precise pins can be the latest stable at the
time of merge. The mise registry exposes both `zig` and
`cargo-zigbuild` as plugins.

#### 3. Add `pytest` to the build dependency group

**File**: `pyproject.toml`

The release helpers and orchestration need a Python-native test
runner. Add to the `build` group:

```toml
[dependency-groups]
build = [
    "invoke>=2.2.1",
    "keepachangelog>=2.0.0",
    "rich>=14.2.0",
    "semver>=3.0.4,<4",
    "tomlkit>=0.13,<0.14",
    "pytest>=8",
    "pytest-mock>=3.14",
]
```

The `semver` major-version cap is deliberate: `Version.prerelease` returns
`None` for absent prereleases in semver 3.x but returned an empty string in
2.x. The helpers below use a truthy check (`if parsed.prerelease:`) so
either shape behaves correctly, but pinning the major version is the
defensive choice.

`pytest-mock` provides the `mocker` fixture used in the orchestration
tests for stubbing `context.run` calls. Both deps install via `uv sync
--only-group build` (the existing `mise.toml:hooks.postinstall` step).

#### 4. Add Rust target installations

`cargo-zigbuild` consumes `rustup target add` outputs. The mise hook
that already runs `uv sync` post-install needs a sibling step to
install the four targets:

**File**: `mise.toml`

```toml
[hooks]
postinstall = [
  "uv sync --only-group build --frozen",
  "rustup target add aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-musl x86_64-unknown-linux-musl"
]
```

(If mise's `postinstall` doesn't accept a list on the pinned mise
version, fall back to a single chained shell command:
`"uv sync --only-group build --frozen && rustup target add aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-musl x86_64-unknown-linux-musl"`.
Validate this in Phase 12.1 against the version in `mise.toml` —
list-syntax support has shifted across mise releases and the
chained form is more portable.)

#### 4a. Align existing files to a coherent starting state

Phase 12.4 §1 will extend `version.write` to advance all three
coherence-tracked files in lockstep (plugin.json + Cargo.toml +
checksums.json's `version` field). For that lockstep to be valid
on the very first invocation, the three files must already agree
*before* the first `version.write` runs. As of writing:

- `.claude-plugin/plugin.json`: at the current plugin version
  (e.g. `1.21.0-pre.1`).
- `skills/visualisation/visualise/server/Cargo.toml`: at `0.1.0`.
- `skills/visualisation/visualise/bin/checksums.json`: at a
  previous plugin pre-release (e.g. `1.19.0-pre.2`) with sentinel
  `0…0` hashes.

In a single Phase 12.1 commit:
- Update `Cargo.toml` `[package].version` to match `plugin.json`.
- Update `bin/checksums.json` `version` field to match `plugin.json`
  (keep the sentinel hashes — those mean "no real binary published
  yet for this version", which remains true until Phase 12.4
  activates).

This makes the working tree pass `validate_version_coherence` from
the moment Phase 12.2 lands, rather than failing until the first
prerelease cut. It also makes the Cargo.toml jump from `0.1.0` a
single deliberate commit rather than a side effect of the first
release.

#### 5. Smoke build to validate the toolchain

Before any orchestration code, validate that the toolchain works
end-to-end:

```bash
mise install
cd skills/visualisation/visualise/frontend && npm ci && npm run build
cd ../server
cargo zigbuild --release --target aarch64-apple-darwin
cargo zigbuild --release --target x86_64-apple-darwin
cargo zigbuild --release --target aarch64-unknown-linux-musl
cargo zigbuild --release --target x86_64-unknown-linux-musl
file target/aarch64-apple-darwin/release/accelerator-visualiser
file target/x86_64-apple-darwin/release/accelerator-visualiser
file target/aarch64-unknown-linux-musl/release/accelerator-visualiser
file target/x86_64-unknown-linux-musl/release/accelerator-visualiser
```

Each `file` output should confirm the right arch (Mach-O for darwin,
ELF for linux). This is a one-shot manual validation, not an
automated test — its purpose is to catch any environment-specific
zig/musl problems before CI hits them.

### Success Criteria

#### Automated Verification:

- [ ] `mise install` on a clean checkout pulls `zig` and
      `cargo-zigbuild` and exits 0.
- [ ] `mise list` shows both pinned versions.
- [ ] On `ubuntu-latest` (CI runner): `mise install` plus the
      smoke-build commands above complete in <10 minutes (validated
      via a throwaway PR adding a `validate-toolchain` workflow that
      runs the smoke build, then deletes the workflow).

#### Manual Verification:

- [ ] On the maintainer's macOS dev host, the four `cargo zigbuild`
      commands complete and `file` reports the right arch for each.
- [ ] The pre-release policy is referenced in both the binary-build
      task's `--help` output (Phase 12.3) and the README's pre-release
      subsection (Phase 12.6).

---

## Phase 12.2: Release helper functions (TDD)

### Overview

The release orchestrator decomposes cleanly into pure functions. The
pure helpers live in **a single Python module** —
`tasks/release_helpers.py` — and are exercised by pytest. **No bash
scripts are introduced**; the release process consumes the helpers
via direct Python imports. Each helper lands with a failing pytest
test first.

The asset-verification helper (`verify_uploaded_asset`) is the one
helper that performs network I/O and shells out to `gh`. It does not
belong in the pure-helper module; it is defined as a private helper
in `tasks/release_binaries.py` (Phase 12.3) where the rest of the
network/subprocess machinery already lives. Phase 12.2 covers it
under "asset verification" but the code lands next to the
orchestration.

The Python module layout:

```
tasks/
└── release_helpers.py    # five pure functions + custom exceptions
```

The pytest test layout (a new `tests/` directory at the repo root —
`tasks/test/` is already taken by the invoke sub-collection):

```
tests/
├── __init__.py
├── conftest.py                       # shared pytest fixtures
└── tasks/
    ├── __init__.py
    ├── test_release_helpers.py
    └── fixtures/
        ├── checksums.example.json
        ├── checksums.with_sentinels.json
        └── tiny_binary.bin           # 4 bytes
```

A new mise task `test:unit:tasks` runs `uv run pytest tests/tasks/`
and is added to the `test:unit` depends list so `mise run test`
picks it up.

### Module signature

```python
# tasks/release_helpers.py
"""Pure functions consumed by the release orchestration tasks.

Helpers in this module read and write local files but do not invoke
subprocesses, make network calls, or shell out. Asset-verification
(which does shell out to `gh`) lives next to the orchestration in
`tasks/release_binaries.py`.
"""
from __future__ import annotations

import hashlib
import json
import tomllib
from pathlib import Path
from typing import Mapping

import semver

_REPO_ROOT = Path(__file__).resolve().parent.parent

# Public helpers ────────────────────────────────────────────────────

def compute_sha256(path: Path) -> str: ...
def update_checksums_json(
    manifest_path: Path,
    version: str,
    platform_hashes: Mapping[str, str] | None = None,
) -> None: ...
def validate_version_coherence(
    expected_version: str, repo_root: Path | None = None,
) -> None: ...
def is_prerelease_version(version: str) -> bool: ...

# Custom exceptions ────────────────────────────────────────────────

class ReleaseHelperError(Exception): ...
class VersionCoherenceError(ReleaseHelperError): ...
class InvalidVersionError(ReleaseHelperError): ...
```

The `AssetVerificationError` exception type is defined in
`tasks/release_binaries.py` next to `verify_uploaded_asset`.

Helpers raise typed exceptions on failure (not silent return values
or string error tokens) so the orchestration task can handle each
failure shape explicitly and pytest can `pytest.raises(...)` with
precision.

### Changes Required

#### 0a. `_atomic_write_text(path, content)` — shared atomic-write primitive

The atomic write-to-tmp + os.replace pattern is needed by both
`update_checksums_json` here and the multi-file lockstep writer
in `tasks/version.py:write` (Phase 12.4 §1). Defining it once in
the helper layer (rather than duplicating it in two modules)
keeps the dependency direction correct: helpers are imported by
the orchestration layer (`tasks/version.py`), never the reverse.

```python
# tasks/release_helpers.py
def _atomic_write_text(path: Path, content: str) -> None:
    """Write *content* to *path* atomically.

    Writes to a sibling `.tmp` file then os.replaces onto the
    target.  On any failure during the write/replace, unlinks the
    `.tmp` rather than leaving orphan state.

    Catches `BaseException` deliberately: the cleanup is a single
    local unlink (O(1), no network), so running it under
    KeyboardInterrupt / SystemExit doesn't interfere with shutdown
    the way a network call would. Contrast with `upload_and_verify`
    in Phase 12.3 §1, which catches `Exception` only because its
    cleanup is a network call.
    """
    tmp = path.with_suffix(path.suffix + ".tmp")
    try:
        tmp.write_text(content)
        tmp.replace(path)
    except BaseException:
        tmp.unlink(missing_ok=True)
        raise
```

**Pytest cases**:
- Successful write: target contains exactly *content* afterwards;
  no `.tmp` sibling exists on disk.
- Pre-existing target: contents are replaced atomically (the test
  patches `os.replace` to log its call sequence and asserts only
  one rename to *path* happens).
- Mid-write failure: the test patches `Path.write_text` to raise
  `OSError("disk full")` after writing some bytes; asserts the
  original target is byte-identical to its pre-call state and the
  `.tmp` sibling does not remain on disk.
- KeyboardInterrupt mid-write: the test patches `Path.write_text`
  to raise `KeyboardInterrupt`; asserts the same cleanup invariants
  and that the exception propagates (the bare `raise` re-raises).

#### 1. `compute_sha256(path)`

```python
def compute_sha256(path: Path) -> str:
    """Return the lowercase hex SHA-256 of the file at *path*.

    Reads in 64-KiB chunks to keep memory bounded for the ~8 MB
    release binaries.
    """
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(64 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()
```

**Pytest cases (RED first)**:
- Empty file → `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.
- `b"hello\n"` → known constant.
- Missing path → `FileNotFoundError` (Python's built-in; we
  intentionally don't wrap it — the OS-level signal is informative
  enough).
- Output is always lowercase (asserts `result == result.lower()`).
- Idempotency: calling twice on the same file returns the same digest.

#### 2. `update_checksums_json(manifest_path, version, platform_hashes=None)`

```python
def update_checksums_json(
    manifest_path: Path,
    version: str,
    platform_hashes: Mapping[str, str] | None = None,
) -> None:
    """Update the checksums manifest in place.

    Sets `.version` to *version*.  For each (platform, hex) pair in
    *platform_hashes*, sets `.binaries[platform]` to
    `f"sha256:{hex}"`.  Platforms not passed in keep their existing
    value.

    Atomic: delegates to `_atomic_write_text` (defined just above
    in this module — see Phase 12.2 §0a).  Single source of truth
    for atomic-write discipline across the helper layer.
    """
    data = json.loads(manifest_path.read_text())
    data["version"] = version
    if platform_hashes:
        for platform, hex_digest in platform_hashes.items():
            data.setdefault("binaries", {})[platform] = f"sha256:{hex_digest}"
    _atomic_write_text(manifest_path, json.dumps(data, indent=2) + "\n")
```

**Pytest cases**:
- All four platforms updated → resulting JSON matches fixture
  byte-for-byte.
- Single platform updated → other three preserved as-is.
- `platform_hashes=None` → only `version` changes; existing
  `binaries` map preserved (including any sentinel `0…0` values).
- Missing manifest → `FileNotFoundError`.
- Atomic write under a partial failure: the test patches
  `Path.write_text` to raise after writing the `.tmp` file, then
  asserts (a) the original manifest is byte-identical to its
  pre-call state, and (b) the `.tmp` sibling does not remain on
  disk. The helper code shown above already includes the
  try/except cleanup; this test exercises that path.

#### 3. `validate_version_coherence(expected_version, repo_root=None)`

```python
def validate_version_coherence(
    expected_version: str,
    repo_root: Path | None = None,
) -> None:
    """Raise VersionCoherenceError unless plugin.json, Cargo.toml,
    and checksums.json all carry *expected_version*.
    """
    root = repo_root or _REPO_ROOT
    found = {
        "plugin.json": _read_plugin_json_version(root),
        "Cargo.toml":  _read_cargo_toml_version(root),
        "checksums.json": _read_checksums_json_version(root),
    }
    mismatches = {k: v for k, v in found.items() if v != expected_version}
    if mismatches:
        raise VersionCoherenceError(
            f"expected {expected_version!r}, found mismatches: {mismatches}"
        )
```

The three private readers (`_read_plugin_json_version`,
`_read_cargo_toml_version`, `_read_checksums_json_version`) live in
the same module. The Cargo.toml reader parses with `tomllib`
(stdlib in Python 3.11+; the repo pins 3.14.4) and reads
`data["package"]["version"]` directly — no regex, no ambiguity
about which `version =` line is targeted, and no risk of drift
between reader and writer regexes.

**Pytest cases**:
- All three match → returns `None`.
- One mismatch → `VersionCoherenceError`; the error message names
  exactly which file is wrong and what value it has.
- Missing file → `FileNotFoundError` (don't wrap — the path is in
  the exception message already).
- `expected_version=""` → `InvalidVersionError`.

This helper is what makes the release task idempotent: a CI job
that crashed mid-flight can be re-run; the helper signals exactly
which file is out of sync.

**Cargo.toml integration with `tasks/version.py`**: Phase 12.4 adds
a new `version.write_cargo_toml(version)` Python helper called
from `version.write()`, so `mise run version:bump` keeps Cargo.toml
in sync. The reader here and the writer in 12.4 both go through
`tomllib` / `tomlkit` (see Phase 12.4 §1) — there is no shared
regex, so reader/writer drift is impossible by construction.

#### 4. `is_prerelease_version(version)`

```python
def is_prerelease_version(version: str) -> bool:
    """True iff *version* parses as semver and has a non-empty
    prerelease component.

    Reuses the existing `semver` build dep (already used by
    `tasks/version.py`) so the parse semantics match what
    `version.bump()` produces.
    """
    try:
        parsed = semver.Version.parse(version)
    except (ValueError, TypeError) as exc:
        raise InvalidVersionError(f"not a valid semver: {version!r}") from exc
    return bool(parsed.prerelease)
```

The truthy check (`bool(parsed.prerelease)`) is deliberately
permissive: in `python-semver` 3.x, `parsed.prerelease` is `None`
when the version has no prerelease component; in older 2.x releases
it could be `""`. Both are falsy, so the helper behaves correctly
across the version range we pin (`semver>=3.0.4,<4`) and any future
minor revisions.

**Pytest cases**:
- `"1.20.0"` → `False`.
- `"1.20.0-pre.1"` → `True`.
- `"1.20.0-rc.1"` → `True`.
- `"1.20.0-pre.2+build.42"` → `True`.
- `"1.20"` → `InvalidVersionError`.
- `""` → `InvalidVersionError`.
- `None` → `InvalidVersionError`.

This helper is consumed by `tasks/github.py:create_release()` to
decide whether to pass `--prerelease` to `gh release create`.

#### 5. Pytest fixtures and conftest

**File**: `tests/conftest.py`

```python
import shutil
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
TASKS_FIXTURES = REPO_ROOT / "tests/tasks/fixtures"


@pytest.fixture
def fake_repo_tree(tmp_path: Path) -> Path:
    """Build a stub repo tree with plugin.json, Cargo.toml, and
    bin/checksums.json at the same paths the helpers expect.
    """
    (tmp_path / ".claude-plugin").mkdir()
    (tmp_path / ".claude-plugin/plugin.json").write_text(
        '{"name":"accelerator","version":"1.20.0"}'
    )
    cargo_dir = tmp_path / "skills/visualisation/visualise/server"
    cargo_dir.mkdir(parents=True)
    (cargo_dir / "Cargo.toml").write_text(
        '[package]\nname = "x"\nversion = "1.20.0"\n'
    )
    bin_dir = tmp_path / "skills/visualisation/visualise/bin"
    bin_dir.mkdir(parents=True)
    shutil.copy(TASKS_FIXTURES / "checksums.example.json",
                bin_dir / "checksums.json")
    return tmp_path
```

The `fake_repo_tree` fixture is shared between Phase 12.2 helper
tests and Phase 12.3 orchestration tests.

#### 6. Mise task wrapper

**File**: `mise.toml`

Add:

```toml
[tasks."test:unit:tasks"]
description = "Run pytest unit tests for invoke tasks"
run = "uv run pytest tests/tasks/ -v"
```

Update the `test:unit` depends list to include `test:unit:tasks` so
`mise run test` covers the new tests.

### Success Criteria

#### Automated Verification:

- [ ] All four helper test classes (one per function) written first
      (RED — `pytest tests/tasks/test_release_helpers.py` exits
      non-zero).
- [ ] Helpers implemented; all pytest cases pass (GREEN).
- [ ] `mise run test:unit:tasks` exits 0.
- [ ] `mise run test` includes the new tests (verified by inspecting
      the depends graph — `test:unit` should list `test:unit:tasks`).
- [ ] `tasks/release_helpers.py` has zero subprocess invocations
      (verified by `grep -n "subprocess\|os.system\|os.popen" tasks/release_helpers.py`
      returning empty).
- [ ] Type annotations pass `uv run python -m py_compile
      tasks/release_helpers.py`.

#### Manual Verification:

- [ ] Reading the module top-to-bottom: each function's docstring
      makes its purpose obvious; no surprises.
- [ ] Custom exceptions (`VersionCoherenceError`,
      `InvalidVersionError`) are imported cleanly by Phase 12.3's
      orchestration task.

---

## Phase 12.3: Binary build + upload invoke task

### Overview

> **Implementation deviation**: The planned `tasks/release_binaries.py` module
> and `BuildArtefacts` dataclass were not created. Instead the concerns were
> split between existing modules: cross-compile/stage/checksum tasks live in
> `tasks/build.py` (as `server_cross_compile`, `create_debug_archives`,
> `create_checksums`); upload/verify/publish tasks live in `tasks/github.py`
> (as `upload_release_asset`, `download_release_asset`, `verify_release_asset`,
> `download_and_verify`, `upload_and_verify`). `upload_and_verify` takes a plain
> `version: str` parameter and reads hashes from `checksums.json` directly,
> rather than from a `BuildArtefacts` dataclass. Helper functions remain in
> `tasks/shared/releases.py` (Phase 12.2's module, renamed from
> `release_helpers.py`). Path constants live in `tasks/shared/paths.py` and
> platform targets in `tasks/shared/targets.py`.

The orchestration lives as a Python invoke task in a new
`tasks/release_binaries.py` module, wired into the collection in
`tasks/__init__.py`. The task is callable directly via
`mise run release:binaries:build` and from the existing
`prerelease()` / `release()` task functions. **It imports the
Phase 12.2 helpers directly — no bash, no `context.run` to scripts
in this repo.** External CLIs (`cargo`, `gh`, `npm`) are invoked
through `context.run` exactly as the existing `tasks/build.py` and
`tasks/github.py` do.

`tasks/release_binaries.py` also owns the `verify_uploaded_asset`
helper and its `AssetVerificationError` exception type — these
perform network I/O and shell out to `gh`, so they belong next to
the rest of the orchestration's subprocess machinery rather than
inside the pure-helpers module.

Test discipline: pytest tests patch `invoke.Context.run` and
`subprocess.run` via `pytest-mock`'s `mocker.patch` and assert on
the recorded `call_args_list`. No real network calls, no real disk
writes outside the per-test tempdir. There is **no `dry_run` flag**
on the production task surface — adding one would mix test
scaffolding into the operator-facing API and creates the kind of
ambiguous "is this side-effect-free?" question that the project's
convention against dry-run UX on destructive ops exists to avoid.

### Atomic release flow ordering (committed)

The order locks in here (resolves Gap 3 from the research follow-up).
The invoke task runs **after** version bumping and **before** version
commit, so the binary build picks up the new version and the manifest
update lands in the same commit:

1. **Pre-flight checks** — fail fast on these before any side effects:
   1.1. Required tools present: `cargo`, `cargo-zigbuild`, `gh`,
        `npm`, `node` ≥ 20. On CI, mise has installed them; on a
        dev host, this catches missing local toolchain early. (No
        `jq` or `sha256sum`/`shasum` check — the helpers use
        Python's `json` and `hashlib` modules directly.)
   1.2. `gh auth status` succeeds (CI has `GH_TOKEN`; dev host has
        `gh auth login`).
   1.3. `--version <X.Y.Z>` flag provided (the task does not infer
        the version from `plugin.json`; the caller passes it
        explicitly to keep the task pure).
   1.4. Pre-update version coherence — call
        `release_helpers.validate_version_coherence(version)`
        before any side effects so a typoed or stale `--version`
        argument is caught before disk mutations begin. This works
        because `version.write` (Phase 12.4 §1) advances all three
        coherence-tracked files (`plugin.json`, `Cargo.toml`,
        `bin/checksums.json`'s `version` field) in lockstep, so by
        the time `release_binaries.build` runs the three are
        already at the new version. Step 6 then overwrites
        `checksums.json`'s `binaries` map with real hashes (the
        `version` field is already correct from `version.write`).
2. **Frontend build** —
   `cd frontend && npm ci && npm run build`. Must produce
   `frontend/dist/index.html`. Failure aborts before any binary
   build.
3. **Cross-compile four binaries**:
   - `aarch64-apple-darwin`
   - `x86_64-apple-darwin`
   - `aarch64-unknown-linux-musl`
   - `x86_64-unknown-linux-musl`

   Each via `cargo zigbuild --release --target <quadruple>`. Two
   artefacts per target are kept: an unstripped copy preserved at
   `target/<triple>/release/accelerator-visualiser` (used for
   debug-symbol archival) and a stripped copy staged for release.
   Stripping uses `cargo-zigbuild`'s built-in `--strip` flag — this
   delegates to zig's strip support, which handles both Mach-O and
   ELF correctly. **Do not** call the host's GNU `strip(1)` as a
   follow-up; on a Linux CI runner it does not understand Mach-O
   and will corrupt the macOS outputs.

   After build, assert the magic bytes of each staged binary match
   the expected ELF / Mach-O signature for the target — pure
   Python (`open(path, 'rb').read(4)`), no `file(1)` dependency.

   **Debug-symbol archives**: alongside each stripped release
   binary, upload an `accelerator-visualiser-<os>-<arch>.debug.tar.gz`
   asset containing the unstripped binary (and, on darwin targets,
   the `.dSYM` bundle if cargo-zigbuild produced one). These debug
   assets are **not** in the SHA-256 manifest (`launch-server.sh`
   never fetches them), so they don't enter the launcher's trust
   path — they exist purely so a maintainer triaging a customer
   crash report can symbolicate a backtrace from the released
   binary's address space. Storage cost is ~10-15 MB per asset
   per release; well inside the GitHub free-tier ceiling for
   public repos. The verify step (step 14) skips debug archives —
   only the four stripped release binaries are SHA-verified.
4. **Stage binaries** — copy each binary into
   `skills/visualisation/visualise/bin/accelerator-visualiser-<os>-<arch>`.
   These are gitignored; CI uploads from this path.
5. **Compute hashes** — one `release_helpers.compute_sha256(path)`
   call per binary; collect results into a Python dict.
6. **Update `checksums.json`** in-place via
   `release_helpers.update_checksums_json(...)`, with the version +
   all four platforms set.
7. **Verify post-update coherence** —
   `release_helpers.validate_version_coherence(version)` raises if
   plugin.json, Cargo.toml, or checksums.json disagree.
8. **Return a `BuildArtefacts(version, binaries, hashes)` value to
   the caller.** This is a small frozen dataclass (defined in
   `tasks/release_binaries.py`) carrying the version string, a
   mapping of platform → staged `Path`, and a mapping of platform →
   lowercase hex SHA-256. Returning a dataclass instead of a tuple
   of dicts keeps the call site type-checkable and removes the
   primitive-obsession smell. The caller (`tasks/release.py:prerelease()`
   or `release()`) runs the subsequent commit/tag/push/upload steps
   because they vary between pre-release and stable flows (changelog,
   marketplace) and need access to the existing helpers in
   `tasks/git.py`.

The caller-driven post-task sequence (covered in Phase 12.4):

9.  Commit the version bump + Cargo.toml + checksums.json in one
    atomic commit.
10. Tag the commit.
11. Push the commit + tag to the remote.
12. **Create the GitHub Release as a draft** on the pushed tag —
    `gh release create "v<ver>" --draft --generate-notes --title "v<ver>" [--prerelease]`.
    Drafts are not visible on the public Releases page, are not
    linked from the Releases atom feed, and are not served from
    `/releases/download/v<ver>/...` until published. The tag is
    `v`-prefixed to match the tag pushed in step 11 and the URL
    `launch-server.sh` constructs. The `--prerelease` flag is added
    by `tasks/github.py:create_release()` when
    `release_helpers.is_prerelease_version(version)` returns `True`.
13. Upload the four binaries as Release assets to the draft.
14. Verify uploaded asset hashes byte-for-byte via
    `release_binaries.verify_uploaded_asset(...)` for each. (See
    Phase 12.3 §1 for the helper definition.) Verification re-downloads
    each asset; while the Release is still a draft, this requires the
    `GH_TOKEN` already in scope (drafts are not anonymously fetchable).
15. **Publish the draft** —
    `gh release edit "v<ver>" --draft=false`. After this call, the
    Release is publicly visible at `/releases/tag/v<ver>` and
    `/releases/download/v<ver>/...` resolves to the verified assets.
    This is the atomic publish point: nothing user-visible exists
    until every asset has been hash-verified.
16. **On any failure** in steps 12-15: delete the draft and exit
    non-zero — `gh release delete "v<ver>" --cleanup-tag --yes`. The
    `--cleanup-tag` flag also removes the pushed tag from the remote
    so the next push can re-bump cleanly. (`gh release delete` on a
    draft doesn't surface the deletion in any audit-visible way; the
    tag deletion does, but a tag-without-release is the only artefact
    that needs cleaning up.) See "Failure modes by step" below for
    the per-step state diagram. **On success** (after step 15 publishes):
    print version, four asset URLs, checksum manifest path, tag URL.

### Failure modes by step

The 16-step flow has many intermediate states. This subsection
enumerates each failure boundary so an operator triaging a failed
CI job knows whether to ignore it (next push supersedes) or
intervene manually. **Key property of the draft-release flow**: no
public release exists until step 15 (`--draft=false`); a failure
anywhere in steps 12-14 can be fully cleaned up by deleting the
draft, with no user-visible side effect.

| Failure point                              | Remote state after failure                           | Auto-recoverable on next push? | Operator action                                                                                                                               |
|--------------------------------------------|------------------------------------------------------|--------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------|
| Pre-flight (steps 1.1-1.4)                 | None — no side effects yet                           | Yes                            | None.                                                                                                                                         |
| Frontend / cross-compile (steps 2-3)       | None on remote; CI runner discarded                  | Yes                            | None.                                                                                                                                         |
| Stage / hash / manifest update (steps 4-7) | None on remote; CI runner discarded                  | Yes                            | None on CI. On a dev host, revert the `bin/checksums.json` change before retrying.                                                            |
| Commit (step 9)                            | None on remote                                       | Yes                            | None.                                                                                                                                         |
| Tag (step 10)                              | None on remote (tag is local-only)                   | Yes                            | None.                                                                                                                                         |
| Push (step 11)                             | Tag pushed; no draft exists                          | No                             | `git push --delete origin v<ver>` so the next prerelease can re-bump cleanly. (Or let the operator pick up from `gh release create --draft`.) |
| `gh release create --draft` (step 12)      | Tag pushed; empty draft exists                       | No                             | `gh release delete v<ver> --cleanup-tag --yes` (handles both draft and tag in one call).                                                      |
| Upload (step 13) — transient                | Tag pushed; draft with partial assets                | No                             | `gh release delete v<ver> --cleanup-tag --yes`. The orchestration does this automatically per step 16.                                        |
| Verify (step 14) — `AssetVerificationError` | Tag pushed; draft with all assets but bad SHA(s)     | **No — preserved for triage**  | **Do NOT auto-clean.** Orchestration emits a `::error` workflow annotation and PRESERVES draft + tag. See "AssetVerificationError triage" below for the procedure (inspect asset SHAs, compare against `bin/checksums.json` on the tagged commit, only delete after triage closes). |
| Verify (step 14) — transient                | Tag pushed; draft with all assets, transient gh failure | No                          | `gh release delete v<ver> --cleanup-tag --yes`. The orchestration does this automatically per step 16 (Exception path, not AssetVerificationError). |
| Publish (step 15)                           | Tag pushed; draft still draft (failed `--draft=false`) | No                           | Re-run `gh release edit v<ver> --draft=false` manually, OR delete the draft+tag.                                                              |

**No public-facing artefacts** are produced by any failure in steps
1-14. A failure at step 15 is the only one that could leave a
non-public-but-recoverable state on the remote — the draft itself.
The operator action for every other failure is "cleanup is automatic
or trivial; let the next push to `main` supersede with `*-pre.N+1`."

**AssetVerificationError triage**: when the verify step raises
`AssetVerificationError` (a SHA-256 mismatch on a re-downloaded
asset), the orchestration deliberately does NOT auto-clean — a SHA
mismatch is the supply-chain-tampering signal that should be
preserved for forensic review, not silently deleted. The recovery
procedure is:

1. The workflow run shows a `::error title=Visualiser release v<ver>::AssetVerificationError — draft + tag PRESERVED for triage` annotation. Capture the raw `gh release download` stderr from the run log to identify which asset failed and what the expected/actual SHAs were.
2. Download the suspect asset out-of-band: `gh release download v<ver> <asset-name> --output /tmp/triage-<asset>`.
3. Compare against the canonical hash in `bin/checksums.json` on the pushed tag: `git show v<ver>:skills/visualisation/visualise/bin/checksums.json | jq -r .binaries.<platform>` matches `sha256:<sha256sum-of-triage-asset>`?
4. If the tampering is confirmed: open a security incident, audit the CI runner / dependencies, and only after triage closes run `gh release delete v<ver> --cleanup-tag --yes` to clear the preserved draft.
5. If the verify failure was a false positive (e.g., flaky network during the verify step caused a partial download): also run `gh release delete v<ver> --cleanup-tag --yes` and let the next prerelease push supersede.

In neither case is the preserved draft user-visible (it never
became `--draft=false`), but the tag is pushed and binds the
namespace, so the next prerelease for `v<ver>` would collide
unless the tag is cleaned up first.

The orchestration's automatic cleanup (`gh release delete --cleanup-tag --yes`)
runs from a `try / except Exception` wrapper around the publish-flow
steps in `tasks/release_binaries.py:upload_and_verify` (see §1 below).
Two failure modes that bypass the cleanup wrapper:

- **`KeyboardInterrupt` / `SystemExit`**: an operator Ctrl-C or a
  CI runner SIGTERM during steps 13-15 propagates immediately
  without running cleanup, so the orphan draft must be cleaned up
  manually: `gh release delete v<ver> --cleanup-tag --yes`. This
  is intentional — running another network call on the
  interrupt-handler path makes Ctrl-C take longer to respond and
  can race with the runner's grace-period timeout.
- **Runner preemption between `*_prepare` and `*_publish`**: if
  the GitHub Actions runner is killed between the prepare half
  and the corresponding publish half (e.g., spot-instance reclaim,
  OOM kill), the staged binaries on disk are lost. The transparency-
  log attestation that the workflow's attest step published refers
  to subjects (binaries) that no public asset matches; this is
  benign (no public release was cut) but pollutes the attestation
  log. The next push to `main` cuts a fresh `*-pre.N+1` and
  attempts again. Operator action: none — the orphan attestation
  is a known no-op.

### Changes Required

#### 1. The Python invoke task

**File**: `tasks/release.py` (extended) or `tasks/release_binaries.py`
(new). Recommendation: new module, to keep the existing two-function
`release.py` readable.

```python
# tasks/release_binaries.py
from __future__ import annotations

import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping

from invoke import Context, task

from . import release_helpers
from .release_helpers import _REPO_ROOT, compute_sha256

_BIN_DIR = _REPO_ROOT / "skills/visualisation/visualise/bin"
_CHECKSUMS = _BIN_DIR / "checksums.json"
_FRONTEND = _REPO_ROOT / "skills/visualisation/visualise/frontend"
_SERVER = _REPO_ROOT / "skills/visualisation/visualise/server"

_TARGETS = (
    ("aarch64-apple-darwin",         "darwin-arm64"),
    ("x86_64-apple-darwin",          "darwin-x64"),
    ("aarch64-unknown-linux-musl",   "linux-arm64"),
    ("x86_64-unknown-linux-musl",    "linux-x64"),
)


@dataclass(frozen=True)
class BuildArtefacts:
    """Output of `build()`, consumed by `upload_and_verify()`."""
    version: str
    binaries: Mapping[str, Path]    # platform → staged binary path
    hashes: Mapping[str, str]       # platform → lowercase hex SHA-256


class AssetVerificationError(Exception):
    """Raised when an uploaded asset's SHA-256 does not match the
    manifest, or when `gh release download` fails."""


def build(context: Context, version: str) -> BuildArtefacts:
    """Build cross-compiled binaries and stage them for release.

    Steps 1-7 of the atomic release flow.  Returns a BuildArtefacts
    value carrying the staged binary paths and their SHA-256 hashes.

    The caller is responsible for committing, tagging, pushing,
    creating the GH Release, uploading assets, and verifying.

    Plain function, not @task: the BuildArtefacts return value is
    consumed by the Python orchestration in tasks/release.py;
    invoke's CLI cannot serialise a dataclass return.
    """
    _preflight(context, version)
    _build_frontend(context)
    binaries = _cross_compile_all(context)
    hashes = {
        platform: compute_sha256(path)
        for platform, path in binaries.items()
    }
    release_helpers.update_checksums_json(_CHECKSUMS, version, hashes)
    release_helpers.validate_version_coherence(version, repo_root=_REPO_ROOT)
    return BuildArtefacts(version=version, binaries=binaries, hashes=hashes)


def upload_and_verify(context: Context, artefacts: BuildArtefacts) -> None:
    """Steps 13-15: upload assets to the draft Release, verify each,
    then publish the draft.

    Step 12 (creating the Release as a draft) is owned by
    tasks.github.create_release; the caller wires the two together.
    Whether the Release is marked `--prerelease` is decided inside
    `create_release` from the version string, so this function does
    not take a `prerelease` parameter.

    On any exception during upload or verify, deletes the draft and
    its tag so no public artefact is left behind.  On success,
    publishes the draft (`--draft=false`) — the publish call is the
    atomic point at which the Release becomes user-visible.

    Plain function, not @task: invoke's CLI cannot construct a
    BuildArtefacts from argv strings.
    """
    # Defensive: validate the version string parses through the
    # same semver check that gates create_release, so a tampered
    # or synthetic BuildArtefacts can't cause us to delete the
    # wrong release via f-string interpolation. The `_ =` makes
    # the validate-only intent explicit at the call site.
    _ = release_helpers.is_prerelease_version(artefacts.version)

    tag = f"v{artefacts.version}"
    try:
        _upload_assets(context, artefacts.version, artefacts.binaries)
        for platform, asset_path in artefacts.binaries.items():
            verify_uploaded_asset(
                release_tag=tag,
                asset_name=asset_path.name,
                expected_hex=artefacts.hashes[platform],
            )
        # Atomic publish: nothing user-visible until this line returns.
        context.run(f"gh release edit {tag} --draft=false", pty=True)
    except AssetVerificationError:
        # SHA-mismatch or download failure during verify is exactly
        # the supply-chain-tampering signal that should be PRESERVED
        # for forensic triage, not silently cleaned up. Leave the
        # draft + tag in place; emit a structured CI annotation so
        # an operator's review queue picks it up; re-raise so the
        # workflow fails loudly. Manual cleanup via
        # `gh release delete <tag> --cleanup-tag --yes` once the
        # incident is closed.
        _emit_forensic_alert(context, tag,
                             "AssetVerificationError — draft + tag PRESERVED for triage")
        raise
    except Exception:
        # Transient failures (network, timeout, gh non-zero on
        # upload, etc.) — clean up the draft so the next push can
        # produce a fresh release without an orphan blocking the
        # tag namespace. We catch Exception (not BaseException) so
        # KeyboardInterrupt / SystemExit propagate immediately
        # without running another network call; the failure-modes
        # table documents that an interrupted publish leaves an
        # orphan draft that must be cleaned up manually with
        # `gh release delete <tag> --cleanup-tag --yes`.
        # warn=True turns a cleanup-side non-zero exit into a
        # logged warning rather than a masking exception. timeout=120
        # bounds the cleanup so a hung gh call doesn't consume the
        # runner's job timeout.
        context.run(
            f"gh release delete {tag} --cleanup-tag --yes",
            warn=True, timeout=120,
        )
        raise


def _emit_forensic_alert(context: Context, tag: str, message: str) -> None:
    """Emit a high-severity workflow annotation describing a
    supply-chain-relevant failure that has been left in place for
    triage rather than auto-cleaned. Visible in the GitHub Actions
    UI as a workflow error annotation.
    """
    # GitHub Actions workflow-command syntax for an error-level
    # annotation. Outside CI this is just stderr noise — harmless.
    print(f"::error title=Visualiser release {tag}::{message}",
          flush=True)


def verify_uploaded_asset(
    release_tag: str, asset_name: str, expected_hex: str,
) -> None:
    """Re-download an uploaded asset via `gh` and assert its SHA-256
    matches *expected_hex*.

    Lives next to the orchestration (rather than in
    `tasks/release_helpers.py`) because it shells out to `gh` and
    performs network I/O — neither of which fit the pure-helper
    contract.

    Raises AssetVerificationError on any mismatch or download
    failure (including timeout).
    """
    with tempfile.NamedTemporaryFile(delete=False) as tmp:
        tmp_path = Path(tmp.name)
    try:
        # Use the positional asset-name form rather than `--pattern`
        # (which is a glob and writes to a directory, not a single
        # file) plus `--output` to a single path. The positional
        # form is the documented single-file shape and avoids the
        # implementation-defined behaviour when --pattern matches
        # more than one asset.
        result = subprocess.run(
            ["gh", "release", "download", release_tag,
             asset_name,
             "--output", str(tmp_path),
             "--clobber"],
            capture_output=True, text=True, timeout=120,
        )
        if result.returncode != 0:
            raise AssetVerificationError(
                f"gh release download failed: {result.stderr.strip()}"
            )
        actual = compute_sha256(tmp_path)
        if actual != expected_hex:
            raise AssetVerificationError(
                f"{asset_name}: expected sha256:{expected_hex}, "
                f"got sha256:{actual}"
            )
    except subprocess.TimeoutExpired as exc:
        raise AssetVerificationError(
            f"gh release download timed out for {asset_name}"
        ) from exc
    finally:
        tmp_path.unlink(missing_ok=True)
```

Each private function (`_preflight`, `_build_frontend`,
`_cross_compile_all`, `_upload_assets`) is a thin wrapper around
`context.run("cargo …")`, `context.run("npm …")`, or
`context.run("gh release upload …")`. **No private function
shells out to a bash script in this repo.** All in-repo logic
(hashing, manifest mutation, version checks) goes through
`release_helpers` Python imports; asset verification is the
private `verify_uploaded_asset` defined above.

`_preflight(context, version)` enforces the pre-flight gates from
the atomic flow: required tools present, `gh auth status`,
`--version` argument provided, pre-update version coherence via
`validate_version_coherence(version)`, **and (when running outside
CI) a working-tree cleanliness check**. The pre-update coherence
check is what makes a typoed `--version` argument fail before any
disk mutation.

The working-tree cleanliness check guards against the local-dev
diagnostic path (`mise run prerelease`/`release`, both of which
pass through `_refuse_under_ci` then would otherwise proceed) being
invoked against a dev host with uncommitted edits to `plugin.json`,
`Cargo.toml`, or `bin/checksums.json` — which `version.write` would
silently overwrite. The check uses `jj status --no-pager` (the
project's VCS) and aborts if any of the three coherence-tracked
files appear in the dirty list. CI runs always start from a fresh
checkout so the check is a no-op there; when `CI`/`GITHUB_ACTIONS`
is set, the helper short-circuits to `return` before invoking jj.

#### 2. Mise task wrappers

`build()` and `upload_and_verify()` in `tasks/release_binaries.py`
are plain Python functions (not `@task`) — they take or return a
`BuildArtefacts` dataclass that invoke's CLI cannot serialise. They
are invoked **only** from the Python orchestration in
`tasks/release.py`. The CI workflow exercises them via the
prepare/finalize wrappers defined in Phase 12.4 §3a / §3b, which
*are* `@task`-decorated and CLI-callable.

No `release:binaries:*` mise tasks exist for direct invocation —
debugging happens by reading the orchestration code or by calling
the prepare/finalize tasks against a fork.

#### 3. Orchestration tests

**File**: `tests/tasks/test_release_binaries.py` (new) — pytest, in the
same `tests/tasks/` directory as Phase 12.2's helper tests.

All test cases use `pytest-mock`'s `mocker.patch` against
`invoke.Context.run` and `subprocess.run` to capture external command
invocations and suppress side effects. Tests run against the
`fake_repo_tree` fixture so the real `bin/checksums.json` is never
touched. No `dry_run` flag, no shell stubs, no `PATH` manipulation —
the test seam is uniformly Python-level mocking.

- **`test_stable_full_flow`**: version `1.20.0`. Asserts:
  - exactly four `cargo zigbuild --release --target …` calls (one
    per quadruple);
  - exactly **eight** `gh release upload v1.20.0 …` calls — four for
    the stripped release binaries plus four for the `*.debug.tar.gz`
    debug-symbol archives (Phase 12.3 atomic flow step 3);
  - `release_helpers.update_checksums_json` called once with all
    four platform keys (and **only** those four — debug archives
    are NOT in the SHA-256 manifest);
  - `release_helpers.validate_version_coherence` called twice (once
    pre-flight before any disk mutation, once after the manifest
    update);
  - `verify_uploaded_asset` called exactly four times (one per
    stripped release binary; debug archives are not SHA-verified).
- **`test_prerelease_full_flow`**: version `1.20.0-pre.5`. Asserts
  the same shape as stable, plus that
  `release_helpers.is_prerelease_version("1.20.0-pre.5")` returns
  `True` (the orchestration test invokes
  `tasks.github.create_release` and asserts the eventual
  `gh release create` invocation includes `--prerelease` and uses
  the `v`-prefixed tag).
- **`test_create_release_uses_v_prefixed_draft_tag`**: stable
  version `1.20.0`. Asserts `context.run` was called with a command
  string containing `gh release create v1.20.0 --draft` (and the
  title `v1.20.0`), not `gh release create 1.20.0` and not without
  `--draft`. Locks in two contracts at once: the `v`-prefix that
  the launcher's URL construction depends on, and the draft-flow
  atomic-abort property.
- **`test_publish_runs_after_verify`**: full happy-path flow.
  Asserts the call ordering ends with
  `gh release edit v1.20.0 --draft=false` (the publish), and that
  `gh release delete` is never called.
- **`test_asset_verification_error_preserves_draft_and_emits_alert`**:
  arrange `verify_uploaded_asset` to raise `AssetVerificationError`.
  Asserts (a) the task re-raises the original exception unchanged,
  (b) `gh release delete` is **never** called (forensic preservation
  is the load-bearing security property — the supply-chain-tampering
  signal must survive for triage), (c) `_emit_forensic_alert` is
  called exactly once with the offending tag, and (d) the publish
  call (`gh release edit ... --draft=false`) does not appear in the
  call list.
- **`test_generic_exception_deletes_draft`**: arrange `_upload_assets`
  to raise a transient `subprocess.CalledProcessError` (not an
  AssetVerificationError). Asserts the task re-raises AND
  `context.run` was called with `gh release delete v1.20.0
  --cleanup-tag --yes` exactly once with `warn=True, timeout=120`.
  Locks in the contract that transient failures auto-clean while
  AssetVerificationError preserves.
- **`test_debug_archives_not_in_checksums_manifest`**: assert
  `release_helpers.update_checksums_json` is invoked with platform
  keys exactly equal to `{darwin-arm64, darwin-x64, linux-arm64,
  linux-x64}` — never with `*.debug.tar.gz` suffixes. Locks in
  the launcher-trust-path boundary that debug archives must not
  enter.
- **`test_verify_skips_debug_archives`**: assert
  `verify_uploaded_asset` is called exactly four times (one per
  stripped binary), never on a `.debug.tar.gz` asset name.
- **`test_missing_tool_aborts_at_preflight`**: `mocker.patch`
  `shutil.which` to return `None` for `cargo`; task raises
  `RuntimeError("cargo not found")` before any other helper or
  `context.run` call (verified via `mock.call_count == 0` on the
  patched `context.run`).
- **`test_pre_update_version_drift_aborts_before_disk_writes`**:
  fixture writes `Cargo.toml` with `0.1.0` while `plugin.json` says
  `1.20.0`; task raises `VersionCoherenceError` from the pre-flight
  call (step 1.4) before any `cargo zigbuild` or
  `update_checksums_json` runs. The post-update coherence call
  (step 7) is therefore never reached. Asserted via
  `mock_update_checksums_json.call_count == 0`.
- **`test_verify_short_circuits_on_first_mismatch`**:
  `verify_uploaded_asset` is called once per platform; the test
  arranges for the first call to fail and asserts subsequent
  `subprocess.run` invocations do not happen (locks in the
  fail-fast semantic vs. an aggregate-failure approach).
- **`test_no_real_filesystem_writes`**: the real
  `bin/checksums.json` (under `_REPO_ROOT`, not the fake tree) is
  byte-identical before and after a full task invocation under
  test mocks. Snapshots the file via `Path.read_bytes()` rather
  than relying on `jj status` so the assertion is VCS-agnostic.

`BuildArtefacts.from_disk` (Phase 12.4 §3a) gets its own focused
test file `tests/tasks/test_build_artefacts.py`:

- **`test_from_disk_happy_path`**: populated `bin/checksums.json`
  with valid `sha256:<hex>` entries plus four staged binaries on
  disk → returns `BuildArtefacts` with the `sha256:` prefix
  stripped from every hash (asserts `hashes["darwin-arm64"] ==
  "abc...123"` not `"sha256:abc...123"`), correct paths derived
  from `_TARGETS`, and the version field matching what was passed.
- **`test_from_disk_version_mismatch_raises`**: `checksums.json`
  has `"version": "1.20.0"` but `from_disk` is called with
  `"1.21.0-pre.0"` → `ValueError` whose message contains both
  versions.
- **`test_from_disk_missing_binary_raises`**: manifest valid but
  one of the four expected staged binaries is absent → `ValueError`
  whose message names the missing path. Aligned with the rest of
  `from_disk`'s exception contract (every defensive guard raises
  `ValueError` with a precise message).
- **`test_from_disk_missing_platform_entry_raises`**: manifest
  valid for three platforms but missing the entry for the fourth
  → `ValueError` naming the missing platform (locks in the
  defensive guard for the bare-`KeyError` minor finding from the
  pass-3 review).
- **`test_from_disk_malformed_prefix_raises`**: a manifest entry
  is the sentinel `"0000…0000"` (no `sha256:` prefix) → `ValueError`
  naming the offending platform and showing the malformed value.
  This exercises the prefix-validation guard against a partially-
  initialised manifest.

The cleanup wrapper's interrupt and cleanup-failure paths get
three focused tests. The cleanup branch catches `Exception`
(transient failures) but **not** `BaseException` (so Ctrl-C /
SIGTERM propagate immediately), and bounds the cleanup `gh release
delete` with `warn=True, timeout=120` to prevent a hung gh call
from consuming the runner's job timeout:

- **`test_keyboard_interrupt_skips_cleanup`**: arrange the
  `_upload_assets` step to raise `KeyboardInterrupt`. Assert
  `gh release delete` was **never** invoked (the `Exception`
  catch deliberately lets `KeyboardInterrupt` propagate without
  running the cleanup network call) and the exception propagates
  unchanged.
- **`test_cleanup_failure_does_not_mask_original_error`**:
  arrange `_upload_assets` to raise a transient
  `subprocess.CalledProcessError` (a non-AssetVerificationError
  exception that DOES go through the cleanup branch) AND the
  cleanup `gh release delete` to fail with non-zero exit (mocked
  via `mocker.patch("invoke.Context.run")` returning a failing
  result for the delete invocation only). Assert that the exception
  that escapes the task is the original `CalledProcessError`, not
  a new error from the cleanup. Verifies the `warn=True` semantic.
  (Note: AssetVerificationError is not a useful trigger here —
  per the forensic-preservation split it bypasses cleanup entirely
  via the dedicated `except AssetVerificationError` branch.)
- **`test_cleanup_invocation_uses_warn_and_timeout`**: arrange a
  transient exception in steps 13-15. Assert `context.run` was
  called with the cleanup command AND with `warn=True, timeout=120`
  keyword arguments. Locks in both the warn semantic and the
  timeout bound against future refactors that drop either.

The four CI-only release halves (Phase 12.4 §3a) and the
`_refuse_under_ci` guard (Phase 12.4 §3) get tests in
`tests/tasks/test_release.py`:

- **`test_refuse_under_ci_raises`** (parametrised over the four
  truthy env-var shapes the helper accepts):
  `(GITHUB_ACTIONS, "true")`, `(CI, "1")`, `(CI, "yes")`,
  `(CI, "true")`. For each shape, monkeypatch the environment,
  call `prerelease()` and `release()`, and assert each raises
  `RuntimeError` whose message names the offending task and
  points at the prepare/finalize replacement. Locks in cross-CI-
  system coverage rather than only the GitHub-Actions case.
- **`test_refuse_under_ci_silent_outside_ci`**: with both env vars
  unset, `prerelease()` and `release()` invoke their respective
  halves without raising. Plus a parametrised case asserting empty
  string (`GITHUB_ACTIONS=""`) is treated as unset.
- **`test_prerelease_prepare_writes_state_for_finalize`**: invoke
  `prerelease_prepare` against a fake repo tree; assert
  `version.bump` ran, `release_binaries.build` ran, and on exit
  the working tree has uncommitted changes plus four staged
  binaries — i.e., the state `prerelease_finalize` expects.
- **`test_prerelease_finalize_reads_state_from_prepare`**: arrange
  the post-prepare state on disk; invoke `prerelease_finalize`;
  assert `BuildArtefacts.from_disk` is called with the correct
  version, and the commit/tag/push/draft/upload/verify/publish
  sequence runs.
- **`test_stable_publish_runs_after_prepare`**: paired prepare +
  publish for the stable cut; assert ordering invariant via
  `mock_calls.index()` — `release_binaries.build` runs before
  `git.commit_version`, `gh release create --draft` runs before
  any `gh release upload`, and `gh release edit --draft=false`
  runs after every `verify_uploaded_asset`.
- **`test_post_stable_runs_after_stable`**: end-to-end mocked
  `release()` (via the thin wrapper); assert the four halves run
  in order `stable_prepare → stable_publish → post_stable_prepare
  → post_stable_publish` with attestation gates conceptually
  between each prepare/publish pair (since the wrapper composes
  without the workflow's attest step, the assertion is purely on
  call ordering, not on attestation having run).

Tests reuse the `fake_repo_tree` fixture from `tests/conftest.py`
(Phase 12.2). No bash stubs are needed: every external command is
mocked at the Python level via `mocker.patch`.

#### 4. Mise task wrapper for tests

**File**: `mise.toml`

```toml
[tasks."test:integration:tasks"]
description = "Run pytest integration tests for invoke tasks"
run = "uv run pytest tests/tasks/test_release_binaries.py -v"
```

Add `test:integration:tasks` to the `test:integration` depends
list. (`test:unit:tasks` from Phase 12.2 covers the pure-helper
tests; the orchestration test is integration-tier because it
exercises multiple modules.)

### Success Criteria

#### Automated Verification:

- [ ] All ten pytest test cases written first (RED) and passing once
      the invoke task lands (GREEN).
- [ ] `mise run test:integration:tasks` exits 0.
- [ ] Pre-release test (`test_prerelease_full_flow`) records
      `--prerelease` exactly once in the `gh release create` call;
      stable test (`test_stable_full_flow`) records zero
      `--prerelease` invocations.
- [ ] Stable test (`test_create_release_uses_v_prefixed_draft_tag`)
      asserts the `gh release create` argument is `v1.20.0 --draft`,
      not `1.20.0` and not without `--draft`.
- [ ] `test_no_real_filesystem_writes` passes — `bin/checksums.json`
      is byte-identical pre/post task invocation under test mocks.
- [ ] `mise run test` includes the new test target.
- [ ] `tasks/release_binaries.py` contains zero references to
      bash scripts in this repo (verified by
      `grep -E "\\.sh|bash " tasks/release_binaries.py` returning
      empty).

#### Manual Verification:

- [ ] Reading the invoke task module top-to-bottom matches the 16-step
      atomic flow ordering above with no surprises.
- [ ] The "Failure modes by step" table is consistent with the
      flow as implemented (each step's failure leaves the state
      described in the table).

---

## Phase 12.4: Wire into existing release tasks + CI workflow

### Overview

> **Implementation deviation**: Because invoke does not allow a task and a
> sub-collection to share the same name, the split tasks are registered as
> flat top-level invoke tasks (`invoke prerelease-prepare`) rather than under
> a `release` collection (`invoke release.prerelease-prepare`). The mise task
> keys still use the `release:*` prefix (e.g. `mise run release:prerelease-prepare`)
> so the external interface is unchanged. `BuildArtefacts.from_disk` was not
> implemented; `upload_and_verify` reads hashes from `checksums.json` on disk
> directly, which eliminates the need for the reconstruct helper.

Connect the new binary-build / upload-and-verify task to the existing
`tasks/release.py:prerelease()` and `tasks/release.py:release()`
flows, and update `.github/workflows/main.yml` so the `prerelease` job
has the `GH_TOKEN` it needs to upload assets.

### Changes Required

#### 1. Extend `tasks/version.py` to write Cargo.toml

**File**: `tasks/version.py`

The existing `version.write()` function only updates `plugin.json`.
Add a sibling that updates `server/Cargo.toml`'s `[package].version`
field, and call it from `write()`:

```python
import json
from pathlib import Path

import tomlkit  # tomli-w-plus-comments writer; new build dep

from .release_helpers import _atomic_write_text

_PLUGIN_JSON = Path(".claude-plugin/plugin.json")
_CARGO_TOML = Path("skills/visualisation/visualise/server/Cargo.toml")
_CHECKSUMS_JSON = Path("skills/visualisation/visualise/bin/checksums.json")


def read_cargo_toml_version() -> str:
    """Return the [package].version string from server/Cargo.toml."""
    data = tomlkit.parse(_CARGO_TOML.read_text())
    return str(data["package"]["version"])


def _render_cargo_toml(version: str) -> str:
    """Pure function: parse Cargo.toml, set [package].version, return
    the new contents.  No I/O.

    Uses tomlkit (a structural TOML round-tripper) rather than a
    regex so the writer is anchored to the [package] table and
    cannot drift onto a dependency table's `version = "..."` key.
    """
    data = tomlkit.parse(_CARGO_TOML.read_text())
    data["package"]["version"] = version
    return tomlkit.dumps(data)


def _render_checksums_with_version(version: str) -> str:
    """Pure function: read checksums.json, set `.version`, return the
    new contents.  Preserves the `binaries` map exactly.

    This is what keeps `validate_version_coherence` consistent at
    every commit: when `version.bump` advances the plugin version,
    checksums.json's `version` field advances in lockstep so the
    three-file invariant holds even before `release_binaries.build`
    has had a chance to overwrite the `binaries` hashes.
    """
    data = json.loads(_CHECKSUMS_JSON.read_text())
    data["version"] = version
    return json.dumps(data, indent=2) + "\n"


def _render_plugin_json(version: str) -> str:
    """Pure function: read plugin.json, set `.version`, return the
    new contents.
    """
    plugin_metadata = read_plugin_metadata()
    plugin_metadata["version"] = version
    return json.dumps(plugin_metadata, indent=2) + "\n"


# Public single-file writers (kept for callers that want to update
# only one of the three files, e.g. tests).
def write_cargo_toml(version: str) -> None:
    """Set [package].version atomically."""
    _atomic_write_text(_CARGO_TOML, _render_cargo_toml(version))


def write_checksums_version(version: str) -> None:
    """Set checksums.json's `version` field atomically, preserving
    the binaries map.
    """
    _atomic_write_text(_CHECKSUMS_JSON, _render_checksums_with_version(version))


@task
def write(_context: Context, version: str):
    """Write plugin version (plugin.json + Cargo.toml + checksums.json)
    in a way that minimises the inconsistent-state window.

    All three target contents are *rendered first* (pure functions,
    no disk writes), then *committed in sequence* via atomic
    write-to-tmp + replace.  If rendering fails (parse error,
    invalid input) the disk is untouched.  If a write fails
    mid-sequence (disk full, SIGTERM), the failed file's .tmp is
    cleaned up by `_atomic_write_text` and a partial-advance state
    can occur — but the next call to `write(version)` will redo
    the lockstep advance idempotently because each renderer reads
    the current on-disk state and produces a known-good output.
    Recovery from a partial-advance is therefore: re-run the same
    bump, OR `jj revert` to the pre-bump state.
    """
    rendered_plugin_json = _render_plugin_json(version)
    rendered_cargo_toml = _render_cargo_toml(version)
    rendered_checksums = _render_checksums_with_version(version)

    _atomic_write_text(_PLUGIN_JSON, rendered_plugin_json)
    _atomic_write_text(_CARGO_TOML, rendered_cargo_toml)
    _atomic_write_text(_CHECKSUMS_JSON, rendered_checksums)
```

The "render first, write second" pattern bounds the inconsistent-
state window to the gap between three sequential `os.replace` calls
— microseconds in normal operation. A SIGTERM landing exactly between
two replaces leaves a partial advance, but the renderers are pure and
idempotent, so re-running `write(version)` recovers cleanly. This is
not full transactionality (true two-phase commit across three files
is not feasible without a journal), but it eliminates the truncation
window that the previous direct-`write_text` shape exposed.

**Accepted technical debt — single-skill coupling**: `tasks/version.py`
is a generic plugin-version layer (its peers handle changelog,
marketplace, github releases) but `_CARGO_TOML` and `_CHECKSUMS_JSON`
hard-code the visualisation skill's filesystem layout. This is
deliberate scope-narrowing for Phase 12 — the visualiser is the only
skill that ships native binaries, so a registry pattern would be
speculative generality. **When (not if) a second binary-bearing skill
arrives**, the right refactor is to introduce a `VersionedFile`
registry (each skill registers its own coherence-tracked files +
render functions; `version.write` iterates the registry) rather than
adding a parallel pair of constants. This is documented in
`RELEASING.md` under "Future extension points" so the next contributor
hits the right answer rather than duplicating the pattern.

`update_checksums_json` (Phase 12.2 §2) is updated to use the same
`_atomic_write_text` helper rather than its inline try/except — single
source of truth for atomic-write discipline.

`tomlkit` is added to the `build` dependency-group in `pyproject.toml`
alongside `pytest`/`pytest-mock` from Phase 12.1. `release_helpers.py`'s
`_read_cargo_toml_version` imports `read_cargo_toml_version` from
`tasks.version` rather than reimplementing the parse — single source
of truth for Cargo.toml version handling.

**Tests** (`tests/tasks/test_version.py`, new — moved out of the
existing `tasks/test/` invoke sub-collection to align with the
pytest layout from Phase 12.2):
- `write` updates **all three** of `plugin.json`, `Cargo.toml`, and
  `bin/checksums.json` (the `version` field) in lockstep.
- `write` is idempotent: running twice with the same version yields
  the same content byte-for-byte across all three files.
- After `write(version)`, `release_helpers.validate_version_coherence(version)`
  passes immediately (no intervening helper calls needed). This is
  the test that locks in the pre-flight-coherence invariant.
- `write_checksums_version` preserves the `binaries` map exactly,
  including any pre-existing platform hashes. Running it on a
  manifest with real hashes leaves those hashes intact — only the
  top-level `version` field changes.
- `write_cargo_toml` preserves comments, blank lines, and unrelated
  fields (fixture: a Cargo.toml with `# top-of-file comment`,
  `[package]`, `[dependencies] foo = { version = "1.2.3" }`,
  `[build-dependencies]`).
- `write_cargo_toml` does **not** mutate a `[dependencies]` entry
  named `version = "..."` — fixture asserts the dependency's
  version is byte-identical after the call.
- `write_cargo_toml` correctly mutates `[package].version` even
  when `[workspace.package]` precedes `[package]` in the file.
- `read_cargo_toml_version` returns the package version, not a
  dependency or workspace version, against the same fixtures.

#### 2. Add `--prerelease` handling to `tasks/github.py:create_release()`

**File**: `tasks/github.py`

```python
import shlex

from invoke import task, Context

from . import release_helpers, version


@task
def create_release(context: Context, target_version: str | None = None):
    """Create a draft release on GitHub.

    The Release is created with `--draft` so it is not user-visible
    until the orchestration's verify step has confirmed every asset's
    SHA-256.  `tasks.release_binaries.upload_and_verify` publishes
    the draft via `gh release edit --draft=false` after verification.

    For pre-release versions (X.Y.Z-suffix), passes --prerelease so
    GitHub marks the release accordingly when published.

    The release tag is `v`-prefixed to match the tag pushed by
    `tasks/git.py:tag_version` and the URL constructed by
    `launch-server.sh` (`/download/v${PLUGIN_VERSION}/...`).
    """
    resolved_version = str(
        target_version or version.read(context, print_to_stdout=False)
    )
    # Validate the version string round-trips through the semver parser
    # before formatting it into the gh command — defensive against any
    # future caller that bypasses version.read's validation.
    is_prerelease = release_helpers.is_prerelease_version(resolved_version)

    tag = f"v{resolved_version}"
    cmd = ["gh", "release", "create", tag,
           "--draft", "--generate-notes", "--title", tag]
    if is_prerelease:
        cmd.append("--prerelease")
    context.run(shlex.join(cmd), pty=True)
```

The `--prerelease` decision is a pure-Python call into
`release_helpers.is_prerelease_version` — no bash, no
`subprocess.run` of an in-repo script. The argv-then-`shlex.join`
form removes the f-string-into-shell injection surface that the
previous shape had (latent today, fragile under future changes).

The `--draft` flag is unconditional. The atomic-abort property of
the release flow depends on it: nothing exists at
`/releases/download/v<ver>/...` until `upload_and_verify` has
verified every asset and run `gh release edit --draft=false`. If
the orchestration crashes between `create_release` and the publish
step, automatic cleanup deletes the draft so no broken public
artefact is left behind.

**Tests** (`tests/tasks/test_github.py`, new):
- `create_release` with stable version `1.20.0` → `context.run`
  called with the exact command string
  `gh release create v1.20.0 --draft --generate-notes --title v1.20.0`,
  no `--prerelease` substring.
- `create_release` with `1.20.0-pre.5` → `context.run` called with
  `gh release create v1.20.0-pre.5 --draft --generate-notes --title v1.20.0-pre.5 --prerelease`.
- `create_release` with malformed `1.20` → `InvalidVersionError`
  raised before any `context.run` call.
- All verified by `mocker.patch("invoke.Context.run")` and asserting
  on `call_args_list` with exact-string equality (not `in`).

#### 3. Wire binary build + upload into `prerelease()` and `release()`

**File**: `tasks/release.py`

```python
import os

from invoke import Context, task

from . import changelog, git, github, marketplace, release_binaries, version


def _refuse_under_ci(task_name: str) -> None:
    """Guard against the local-dev convenience tasks running under
    CI, where the prepare/finalize halves are the sanctioned path
    (they interleave SLSA attestation; the single-call shape skips
    it).  See Phase 12.4 §3a.

    Recognises both `GITHUB_ACTIONS` (set by GitHub Actions
    runners) and the generic `CI` convention (set by most other
    CI systems — GitLab, CircleCI, Buildkite, Jenkins via plugins).
    Catches any non-empty value, not just literal `"true"`, since
    historically some CI systems set `CI=1` or `CI=yes`.
    """
    if os.environ.get("GITHUB_ACTIONS") or os.environ.get("CI"):
        raise RuntimeError(
            f"{task_name} is the local-dev diagnostic task; CI must use "
            f"the prepare/finalize split (see RELEASING.md). Bypassing "
            f"the split skips SLSA attestation, producing releases "
            f"that fail downstream `gh attestation verify` calls."
        )


@task
def prerelease(context: Context):
    """Local-dev diagnostic only. Composes prerelease_prepare +
    prerelease_finalize without the workflow's attestation step in
    between, so the resulting release has no SLSA provenance.

    Useful when debugging the release pipeline against a fork. CI
    must use the split tasks directly (see Phase 12.4 §3a).
    """
    _refuse_under_ci("prerelease")
    prerelease_prepare(context)
    prerelease_finalize(context)


@task
def release(context: Context):
    """Local-dev diagnostic only. Composes the four stable halves
    without the workflow's attestation steps in between. CI must use
    the split tasks directly.
    """
    _refuse_under_ci("release")
    stable_prepare(context)
    stable_publish(context)
    post_stable_prepare(context)
    post_stable_publish(context)
```

The single-call wrappers are now ten lines each — no logic
duplication with the split halves. The `_refuse_under_ci` guard
prevents accidentally bypassing attestation in CI: an operator who
configures the workflow to call `mise run prerelease` (instead of
the prepare/finalize sequence) sees an immediate error rather than
a silent skip of attestation.

**Note on commit ordering and the post-release bump**:
`release_binaries.build()` writes `bin/checksums.json` before
`git.commit_version` runs (in each `*_publish` halve), so the
checksums update lands in the same commit as the version bump.
`version.write` separately advances `checksums.json`'s `version`
field (Phase 12.4 §1) so the pre-flight coherence check passes
before `build` runs and re-populates the `binaries` map with
real hashes.

The post-release `*-pre.0` bump deliberately runs the full release
pipeline a second time (post_stable_prepare + attest + post_stable_publish).
This is a deliberate cost-vs-coherence tradeoff:

- **Cost**: roughly doubles the wallclock time of a stable release
  cut (~10-15 min → ~20-30 min), since the cross-compile is the
  hot path. It also creates a `*-pre.0` GitHub Release that nobody
  explicitly asks for.
- **Benefit**: `validate_version_coherence` holds at every tagged
  commit — there is no "intermediate-version tag points at a commit
  with stale checksums" hole. The launcher's manifest-version-drift
  check works at any tag without exceptions.

The wallclock cost only applies to stable releases (which are
already gated by the `release` GitHub Environment approval and
happen on a manual cadence), not to per-merge pre-releases. So
the user-visible impact is "stable cuts take longer," which is
acceptable given how rarely they happen.

Distinguishing auto-cut `*-pre.0` releases from genuine `*-pre.N`
cuts (the architecture lens flagged this concern): the auto-cut
release inherits its title and body from `gh release create
--generate-notes`, which compares against the previous tag — for
`*-pre.0` that's the just-published stable, so the auto-generated
notes will read "no changes since vX.Y.Z" and clearly identify it
as a scaffolding cut. No additional marker is needed beyond what
GitHub's auto-notes already produce.

**Tests** (`tests/tasks/test_release.py`, new):
- `prerelease` exercises the full sequence with all dependencies
  mocked via `mocker.patch`: bump → build → commit → tag → push →
  create release → upload → verify.
- `release` exercises the full sequence including marketplace and
  changelog updates.
- The mocks confirm `release_binaries.build` is called before
  `git.commit_version` so the manifest update is staged.
- All assertions are `mock.call_args_list` based — pure pytest, no
  shell stubs or `PATH` manipulation.

#### 3a. Split for SLSA build provenance

The `prerelease()` and `release()` tasks shown above are the
single-call shape used for **local diagnostics only** (a maintainer
running `mise run prerelease` against a fork to debug a CI issue).
They do not generate SLSA provenance attestations because that
requires a workflow-injected OIDC token, which is unavailable
outside GitHub Actions.

For CI, the same logical sequence is split at the build/commit
boundary so the workflow can interleave
`actions/attest-build-provenance@v2.4.0` between the build and the
publish. The stable-release flow splits into **four** halves
because attestation must run twice — once for the stable cut, once
for the post-release `*-pre.0` rebuild:

```python
# tasks/release.py — additional CI-only halves
#
# The prerelease flow has two halves; the release flow has four
# (stable cut + post-stable pre.0 cut, each with its own attest gate).
# Workflow YAML interleaves actions/attest-build-provenance@v2.4.0 between
# each prepare and its sibling publish.

def _finalize_and_publish(context: Context) -> None:
    """Shared body of every `*_publish` halve: reconstruct
    BuildArtefacts from disk, commit + tag + push, create draft
    Release, upload assets, verify each, publish (or auto-cleanup
    on failure via upload_and_verify's wrapper).

    Single source of truth so changes to the publish sequence land
    in one place rather than four.  Reads the current version from
    plugin.json (which `*_prepare` advanced via version.write's
    lockstep call).
    """
    current_version = str(version.read(context, print_to_stdout=False))
    artefacts = release_binaries.BuildArtefacts.from_disk(
        version=current_version, repo_root=_REPO_ROOT,
    )
    git.commit_version(context)
    git.tag_version(context)
    git.push(context)
    github.create_release(context)
    release_binaries.upload_and_verify(context, artefacts=artefacts)


@task
def prerelease_prepare(context: Context):
    """CI prerelease, halve 1 of 2.  Bump version + build binaries.

    Side effects on exit: plugin.json / Cargo.toml / bin/checksums.json
    have the new version (via version.write's lockstep advance), four
    binaries are staged in skills/visualisation/visualise/bin/, no
    commit yet.  The workflow's attest step then signs the staged
    binaries before prerelease_finalize is invoked.
    """
    git.configure(context)
    git.pull(context)
    version.bump(context, bump_type=[version.BumpType.PRE])
    new_version = str(version.read(context, print_to_stdout=False))
    release_binaries.build(context, version=new_version)


@task
def prerelease_finalize(context: Context):
    """CI prerelease, halve 2 of 2.  Commit + tag + push + create
    draft Release + upload + verify + publish, all via
    `_finalize_and_publish`.
    """
    _finalize_and_publish(context)


@task
def stable_prepare(context: Context):
    """CI stable, halve 1 of 4.  Finalise version + marketplace +
    changelog + build binaries.  No commit yet.
    """
    git.configure(context)
    git.pull(context)
    version.bump(context, bump_type=[version.BumpType.FINALISE])
    new_version = str(version.read(context, print_to_stdout=False))
    marketplace.update_version(context, plugin="accelerator")
    changelog.release(context)
    release_binaries.build(context, version=new_version)


@task
def stable_publish(context: Context):
    """CI stable, halve 2 of 4.  Publish the stable release via
    `_finalize_and_publish`.
    """
    _finalize_and_publish(context)


@task
def post_stable_prepare(context: Context):
    """CI stable, halve 3 of 4.  Bump to next minor pre.0 + build
    binaries for that version.  No commit yet.

    The rebuild is what preserves the validate_version_coherence
    invariant at every tagged commit (see Phase 12.4 §3 note).
    """
    version.bump(
        context, bump_type=[version.BumpType.MINOR, version.BumpType.PRE]
    )
    next_version = str(version.read(context, print_to_stdout=False))
    release_binaries.build(context, version=next_version)


@task
def post_stable_publish(context: Context):
    """CI stable, halve 4 of 4.  Publish the post-stable pre.0
    release via `_finalize_and_publish`.
    """
    _finalize_and_publish(context)
```

Each `*_prepare` ends with `release_binaries.build` writing real
hashes into `bin/checksums.json` (overwriting whatever was there —
the previous publish's hashes for prerelease_prepare and
post_stable_prepare; the stable_prepare's just-built hashes for
post_stable_prepare). The corresponding `*_publish` halve always
reads back the freshest hashes via `BuildArtefacts.from_disk`, so
each prepare→publish pair is self-contained.

`BuildArtefacts.from_disk(version, repo_root)` contract:

```python
@classmethod
def from_disk(cls, version: str, repo_root: Path) -> "BuildArtefacts":
    """Reconstruct from the on-disk state that *_prepare leaves
    behind.  Reads bin/checksums.json for the binaries map and
    derives staged-binary paths from _TARGETS.

    Raises FileNotFoundError if any expected binary is missing.
    Raises ValueError if checksums.json's version doesn't match
    the requested version (defensive guard against running
    finalize against a stale prepare-side state).
    """
    bin_dir = repo_root / "skills/visualisation/visualise/bin"
    manifest = json.loads((bin_dir / "checksums.json").read_text())
    if manifest.get("version") != version:
        raise ValueError(
            f"checksums.json version {manifest.get('version')!r} does "
            f"not match expected {version!r}"
        )
    manifest_binaries = manifest.get("binaries", {})
    binaries: dict[str, Path] = {}
    hashes: dict[str, str] = {}
    for _triple, platform in _TARGETS:
        if platform not in manifest_binaries:
            raise ValueError(
                f"checksums.json missing entry for platform {platform!r}"
            )
        path = bin_dir / f"accelerator-visualiser-{platform}"
        if not path.exists():
            raise ValueError(f"staged binary missing: {path}")
        raw = manifest_binaries[platform]   # e.g. "sha256:abc123…"
        prefix, _, hex_digest = raw.partition(":")
        if prefix != "sha256" or not hex_digest:
            raise ValueError(
                f"unexpected manifest entry for {platform}: {raw!r}; "
                f"expected 'sha256:<hex>'"
            )
        binaries[platform] = path
        hashes[platform] = hex_digest
    return cls(version=version, binaries=binaries, hashes=hashes)
```

Three subtle points called out:

- The `sha256:` prefix is **stripped** before populating
  `hashes[platform]` — the rest of the orchestration (the
  `verify_uploaded_asset` call in `upload_and_verify`) compares
  against the raw hex without prefix.
- The version-mismatch guard catches the case where
  `prerelease_prepare` ran for `1.20.0-pre.5` but `prerelease_finalize`
  was somehow invoked with `1.20.0-pre.6` — fail fast rather than
  publish a release tagged for the wrong version.
- Path derivation depends on `_TARGETS` being importable from the
  same module; the helper, the writer (`_cross_compile_all`), and
  the reader (`from_disk`) all share that one tuple.

The single-call `prerelease()` and `release()` retain their
existing shape for local-dev diagnostics — they do not call
prepare/finalize (otherwise the local flow would silently skip
attestation and produce a release without provenance, which would
confuse downstream `gh attestation verify` invocations). Document
this clearly in `RELEASING.md`: **CI is the only sanctioned
release path**, the local-dev tasks exist for debugging only.

#### 3b. Mise task wrappers for the prepare/publish halves

**File**: `mise.toml`

Each Python invoke task gets a corresponding mise task so the
workflow YAML can invoke them by name:

```toml
[tasks."release:prerelease-prepare"]
description = "CI prerelease halve 1: bump + build (no commit)"
run = "invoke release.prerelease-prepare"

[tasks."release:prerelease-finalize"]
description = "CI prerelease halve 2: commit + tag + push + draft + upload + verify + publish"
run = "invoke release.prerelease-finalize"

[tasks."release:stable-prepare"]
description = "CI stable halve 1 of 4: finalise version + marketplace + changelog + build"
run = "invoke release.stable-prepare"

[tasks."release:stable-publish"]
description = "CI stable halve 2 of 4: commit + tag + push + draft + upload + verify + publish stable"
run = "invoke release.stable-publish"

[tasks."release:post-stable-prepare"]
description = "CI stable halve 3 of 4: bump to next minor pre.0 + build"
run = "invoke release.post-stable-prepare"

[tasks."release:post-stable-publish"]
description = "CI stable halve 4 of 4: commit + tag + push + draft + upload + verify + publish pre.0"
run = "invoke release.post-stable-publish"
```

`invoke` translates underscores to hyphens in task names by default,
so `release.prerelease_prepare` (Python) is invoked as
`invoke release.prerelease-prepare` (CLI). The mise wrappers exist
to give the workflow YAML a stable, namespaced task name surface
without coupling it directly to invoke's CLI conventions.

#### 4. Update `.github/workflows/main.yml`

**File**: `.github/workflows/main.yml`

The `prerelease` job needs `GH_TOKEN` and write permission. Two
additional concerns are folded in alongside: a concurrency group so
two pushes to `main` cannot race, and per-job permissions so the
`test` job does not silently inherit `contents: write`.

Workflow-level default — least privilege:

```yaml
permissions:
  contents: read
```

The `test` job inherits `contents: read`; only `prerelease` and
`release` are scoped wider, per-job. Both also need
`id-token: write` and `attestations: write` so
`actions/attest-build-provenance@v2.4.0` can sign with the workflow's
OIDC token and publish to GitHub's attestation transparency log:

```yaml
prerelease:
  name: Create prerelease
  runs-on: ubuntu-latest
  needs: test
  if: github.event_name == 'push'
  environment: prerelease   # no approvers — exists as a kill switch
  permissions:
    contents: write
    id-token: write
    attestations: write
  concurrency:
    group: prerelease-${{ github.ref }}
    cancel-in-progress: false

  steps:
    - name: Checkout code
      uses: actions/checkout@v4

    - name: Install dependencies
      uses: jdx/mise-action@v4
      with:
        install: true
        cache: true
        experimental: true

    - name: Bump version + build binaries
      env:
        GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      run: mise run release:prerelease-prepare

    - name: Attest binary provenance
      uses: actions/attest-build-provenance@v2.4.0
      with:
        subject-path: 'skills/visualisation/visualise/bin/accelerator-visualiser-*'

    - name: Commit, tag, push, publish
      env:
        GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      run: mise run release:prerelease-finalize
```

`actions/attest-build-provenance@v2.4.0` publishes the attestation to
GitHub's attestation API (transparency-log-backed, sigstore-signed
with the workflow's OIDC token). It does **not** write a local
attestation file by default; verification later is via
`gh attestation verify <binary> --repo atomic-innovation/accelerator`
(repo-scoped, not org-scoped — see Phase 12.4 §6 for why), which
fetches the attestation from the API.

The action is pinned to a specific patch version (`@v2.4.0`) rather
than a floating major tag (`@v2`), matching the rest of the
toolchain's exact-version pinning convention. SLSA tooling is
moving fast; an unguarded floating tag would amount to "we don't
pin our security infrastructure." `RELEASING.md` lists the
attest-build-provenance pin as one of the dependencies a
maintainer must explicitly review when bumping.

The `environment: prerelease` directive is intentionally configured
**without approvers** — it is a kill switch, not an approval gate.
Per-merge prereleases are how internal users dogfood unreleased
changes, so adding human approval would defeat the cadence. But
the Environment provides a single-toggle disable mechanism in the
GitHub UI: a repo admin can pause prereleases by setting a
`deployment branch policy` that excludes `main`, or by adding a
required reviewer that never approves, without modifying any code.

**Important caveat — the kill switch only halts *future* runs.**
GitHub Environments evaluate the deployment-branch policy at
job-start; an in-flight `prerelease` job that is mid-publish when
an admin flips the toggle will run to completion. For an in-flight
halt, the operator must additionally cancel the running job via
`gh run cancel <id>` (or the Actions UI's Cancel button). See
`RELEASING.md`'s incident-response section for the full procedure
including the cancel step.

The `release` job mirrors this shape, with two attestation steps —
one for the stable cut and one for the post-release `*-pre.0` bump
(per Phase 12.4 §3's coherence-invariant decision):

```yaml
release:
  name: Release
  runs-on: ubuntu-latest
  needs: test
  if: github.event_name == 'push'
  environment: release
  permissions:
    contents: write
    id-token: write
    attestations: write
  concurrency:
    group: stable-release-${{ github.ref }}
    cancel-in-progress: false

  steps:
    - name: Checkout code
      uses: actions/checkout@v4

    - name: Install dependencies
      uses: jdx/mise-action@v4
      with:
        install: true
        cache: true
        experimental: true

    - name: Stable: bump + build
      env:
        GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      run: mise run release:stable-prepare

    - name: Stable: attest
      uses: actions/attest-build-provenance@v2.4.0
      with:
        subject-path: 'skills/visualisation/visualise/bin/accelerator-visualiser-*'

    - name: Stable: publish
      env:
        GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      run: mise run release:stable-publish

    - name: Post-release pre.0: bump + build
      env:
        GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      run: mise run release:post-stable-prepare

    - name: Post-release pre.0: attest
      uses: actions/attest-build-provenance@v2.4.0
      with:
        subject-path: 'skills/visualisation/visualise/bin/accelerator-visualiser-*'

    - name: Post-release pre.0: publish
      env:
        GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      run: mise run release:post-stable-publish
```

`cancel-in-progress: false` is deliberate: cancelling a release
job mid-flight after `gh release create` has run is worse than
queuing the next job, since cancellation produces exactly the
mid-flight failure modes documented in the "Failure modes by step"
table above.

The `prerelease` and `release` jobs use **distinct** concurrency
groups (`prerelease-${{ github.ref }}` and
`stable-release-${{ github.ref }}`). They do not contend on shared
mutable state (each pushes its own version-bump commit and tag
with non-overlapping versions), so independent groups are safe
and preserve the dogfooding cadence: a stable release awaiting
environment approval no longer blocks subsequent prereleases on
`main`. Within each group, `cancel-in-progress: false` still
serialises operations of the same type so two prereleases (or
two stable cuts) cannot race on the tag-push window.

#### 5. (covered by §4 above)

Permissions and concurrency are folded into the per-job
configuration in §4 rather than declared at workflow level. This
removes the over-privileging of the `test` job that an unconditional
`permissions: contents: write` would create.

#### 6. Opt-in launcher provenance verification

**File**: `skills/visualisation/visualise/scripts/launch-server.sh`

After SHA-256 verification succeeds (and only then), if the user
has set `ACCELERATOR_VISUALISER_VERIFY_PROVENANCE=1`, the launcher
runs `gh attestation verify` against the cached binary and refuses
to launch on failure. The check lives in a helper function in
`launcher-helpers.sh`; `launch-server.sh` invokes it after SHA-256
verifies and before exec.

**File**: `skills/visualisation/visualise/scripts/launcher-helpers.sh`

```bash
# Compare two dotted version strings numerically. Returns 0 (true)
# if $1 >= $2, 1 (false) otherwise. Pure-awk so it works on BSD
# sort (older macOS) and GNU sort identically.
#
# Usage: _gh_version_at_least 2.89.0 2.49.0  # exit 0
#        _gh_version_at_least 2.48.99 2.49.0 # exit 1
_gh_version_at_least() {
  local current="${1:-}" required="$2"
  # Empty current-version → fail closed (treat as too old).
  [[ -z "${current}" ]] && return 1
  awk -v cur="${current}" -v req="${required}" '
    function tonum(v,   parts, n, i, total) {
      n = split(v, parts, ".")
      total = 0
      for (i = 1; i <= 3; i++) total = total * 1000 + (parts[i] + 0)
      return total
    }
    BEGIN { exit (tonum(cur) >= tonum(req)) ? 0 : 1 }
  '
}

# Provenance verification helper. Wrapped in a function so `local`
# and the `trap ... RETURN` cleanup are valid (they require a
# function scope in bash). Called from launch-server.sh after the
# SHA-256 check passes.
#
# Reads:
#   $1 - path to the cached binary
# Env:
#   ACCELERATOR_VISUALISER_PROVENANCE_REPO (default: atomic-innovation/accelerator)
# Exits via die_json on failure; returns 0 on success.
_verify_provenance() {
  local bin_path="$1"
  if ! command -v gh >/dev/null 2>&1; then
    die_json "ACCELERATOR_VISUALISER_VERIFY_PROVENANCE is set but \`gh\` is not installed. Install gh >= 2.49.0 (https://cli.github.com/) and re-run, or unset the variable."
  fi
  # gh attestation verify was added in gh 2.49.0; surface the
  # minimum-version requirement explicitly rather than letting the
  # user see "unknown command 'attestation'" on older builds.
  local gh_version
  gh_version="$(gh --version 2>/dev/null | awk '/^gh version/{print $3; exit}')"
  if ! _gh_version_at_least "${gh_version}" 2.49.0; then
    die_json "ACCELERATOR_VISUALISER_VERIFY_PROVENANCE requires gh >= 2.49.0 (found ${gh_version:-unknown}). Upgrade gh or unset the variable."
  fi
  # Repo-scoped (--repo) rather than owner-scoped (--owner): pins
  # the attestation subject to a specific repository, defeating the
  # byte-equal-fork attack where a malicious fork ships a binary
  # identical to an upstream one and benefits from the upstream
  # attestation.
  local repo="${ACCELERATOR_VISUALISER_PROVENANCE_REPO:-atomic-innovation/accelerator}"
  local err_file
  err_file="$(mktemp)"
  # The RETURN trap fires when this function returns (success or
  # failure path). die_json calls exit, so we also clean up via an
  # EXIT trap inside die_json's own cleanup chain — but the RETURN
  # trap is sufficient for the success path.
  trap 'rm -f "${err_file}"' RETURN
  if ! gh attestation verify "${bin_path}" \
       --repo "${repo}" \
       --quiet 2>"${err_file}"; then
    die_json "Provenance verification failed for ${bin_path}: $(cat "${err_file}"). The binary's SHA-256 matches the manifest, but the matching attestation could not be located or did not verify. Recovery: (a) delete ${bin_path} to force a fresh download from a release with provenance; (b) set ACCELERATOR_VISUALISER_PROVENANCE_REPO if your install came from a fork; or (c) unset ACCELERATOR_VISUALISER_VERIFY_PROVENANCE if your environment cannot reach GitHub's attestation API."
  fi
}
```

**File**: `skills/visualisation/visualise/scripts/launch-server.sh`

```bash
# Inserted after the existing SHA-256 check passes, before the
# binary is exec'd. The opt-in check is one bash if; the body is
# the helper function defined in launcher-helpers.sh.
if [[ -n "${ACCELERATOR_VISUALISER_VERIFY_PROVENANCE:-}" ]]; then
  _verify_provenance "${BIN_CACHE}"
fi
```

`die_json` is the existing helper in `launcher-helpers.sh` that
emits a structured error and exits non-zero. The `--quiet` flag on
`gh attestation verify` suppresses success output; failure output
goes to a `mktemp`-allocated file (avoiding the predictable-path
symlink-attack surface) and is read into the `die_json` payload
only on failure.

The `_gh_version_at_least` body uses awk's numeric comparison
rather than `sort -V` so the helper works identically on BSD sort
(older macOS) and GNU sort. The implementation handles a missing
`gh --version` (empty `current`) as fail-closed: an unparseable
gh version is treated as "too old", surfacing the version error
rather than silently allowing the call.

Three new env vars / behaviours introduced here, all opt-in:

- `ACCELERATOR_VISUALISER_VERIFY_PROVENANCE=1` — enable the check.
- `ACCELERATOR_VISUALISER_PROVENANCE_REPO` — override the repo the
  attestation must be signed for (default
  `atomic-innovation/accelerator`). Forks set this to their own
  `<owner>/<repo>` so verification works against attestations
  signed by their fork's CI.
- The `--repo` form (vs `--owner`) closes the byte-equal-fork
  attack noted in the security re-review.

**Default behaviour is unchanged** — users who do not set the
flag see no behaviour change, the launcher does not require `gh`,
and the SHA-256 manifest check is the same defence it has always
been. The opt-in flag is targeted at security-conscious operators
who want defence-in-depth against a compromised CI runner.

**Tests** (Phase 12.5 smoke-test extensions):
- `provenance_verify_skipped_when_unset`: launcher succeeds when
  `ACCELERATOR_VISUALISER_VERIFY_PROVENANCE` is unset, even if the
  test mirror serves a binary that has no published attestation.
  This confirms zero-impact-by-default.
- `provenance_verify_required_when_set`: launcher fails with a
  clear `die_json` error when the flag is set but `gh attestation
  verify` cannot locate or verify an attestation. The test arranges
  for `gh` to be a stub that always exits non-zero from
  `attestation verify`.
- `provenance_verify_missing_gh_errors_clearly`: launcher fails
  with a specific error message when the flag is set but `gh` is
  not on PATH. Test simulates this by pointing PATH at a directory
  that does not contain `gh`.

### Success Criteria

#### Automated Verification:

- [ ] `tests/tasks/test_version.py` — `write` updates Cargo.toml;
      all tests pass.
- [ ] `tests/tasks/test_github.py` — `--prerelease` flag flows
      correctly based on version; all tests pass.
- [ ] `tests/tasks/test_release.py` — split prepare/finalize tasks
      and the existing single-call wrappers all pass mocked
      end-to-end sequences.
- [ ] `mise run test` includes all three new test files and exits 0.
- [ ] Workflow YAML lints clean (`actionlint .github/workflows/`).
- [ ] No new bash scripts introduced anywhere under `tasks/` or
      `tests/` (verified by
      `find tasks tests -name '*.sh' -newer .gitignore` returning
      empty).
- [ ] `actions/attest-build-provenance@v2.4.0` is referenced exactly
      once in each of `prerelease` (1×) and `release` (2×) jobs in
      the workflow YAML.

#### Manual Verification:

- [ ] On a throwaway PR branch, manually trigger `mise run release:prerelease-prepare`
      (against a fork): the binaries appear in `bin/`. Then on a
      copy of the workflow run with attestation enabled: the
      attestation is published to GitHub's attestation API.
- [ ] On the same fork, the GitHub Release UI shows the pre-release
      banner and the four assets.
- [ ] `gh release view <tag> --json isPrerelease` shows `true` for
      pre-releases and `false` for stable.
- [ ] `gh attestation verify <downloaded-binary> --owner <fork-owner>`
      succeeds for a binary downloaded from the just-published
      release.
- [ ] Setting `ACCELERATOR_VISUALISER_VERIFY_PROVENANCE=1` in a
      fresh project successfully launches against an
      attestation-bearing release; setting the same flag against
      an artificially-tampered cached binary triggers the
      `die_json` provenance error.

---

## Phase 12.5: Binary acquisition smoke test

### Overview

Phase 11.5's deferred test: from a clean cache (no
`bin/accelerator-visualiser-*`), invoke `launch-server.sh` against a
local HTTP "release mirror" (via `ACCELERATOR_VISUALISER_RELEASES_URL`),
assert it downloads the right binary, verifies the SHA against the
manifest, launches the server, and serves `/api/types`.

This test cannot use `cargo run` because the test verifies the
download flow; it has to consume a real, on-disk binary that
`launch-server.sh` fetches from an HTTP server the test stands up.

### Approach

The test stands up a tiny `axum` HTTP server (in the test process)
serving a single route: `/v<version>/accelerator-visualiser-<os>-<arch>`
returning a real release-build binary the test built ahead of time
(via a `cargo build --release` invocation in the test setup).

Manifest manipulation: the test writes a temporary `checksums.json`
into a temp tree, computes the SHA-256 of the actual binary, and
points `launch-server.sh` at the temp manifest by overriding the
skill root via `ACCELERATOR_VISUALISER_SKILL_ROOT`. This env var is
treated as a **test-only seam** — add a comment to its consumer in
`launch-server.sh` flagging that it exists for the binary-acquisition
smoke test and should not be promoted to a public hook without first
documenting it in the README.

Because the in-process mirror serves plain HTTP on `127.0.0.1`, the
test sets `ACCELERATOR_VISUALISER_INSECURE_DOWNLOAD=1` explicitly
so the launcher's HTTPS-enforcement is satisfied. The launcher
**enforces** that the insecure-download flag is paired with a
loopback URL — see Phase 12.5 §3 for the helper that gates this.
The contract: README's mirror-override instructions require HTTPS
for any real-world mirror; the insecure-download flag is a
localhost-only escape hatch backed by code, not just documentation.

#### 3. Loopback-only enforcement for `ACCELERATOR_VISUALISER_INSECURE_DOWNLOAD`

**File**: `skills/visualisation/visualise/scripts/launcher-helpers.sh`

Add a guard helper that the existing `download_to` (or the
`launch-server.sh` precedence resolver) calls before honouring
`ACCELERATOR_VISUALISER_INSECURE_DOWNLOAD=1`:

```bash
# Reject INSECURE_DOWNLOAD against a non-loopback mirror URL. The
# flag is documented as a localhost-only escape hatch; this guard
# turns the documentation into a runtime contract, eliminating the
# combination "INSECURE_DOWNLOAD=1 + remote http://...mirror" that
# would otherwise allow a network MITM to substitute the binary.
#
# Returns 0 (true) if either:
#   - the flag is unset, OR
#   - the URL host parses as 127.0.0.1, ::1, or localhost.
# Otherwise calls die_json and exits non-zero.
#
# Accepted URL forms:
#   http://127.0.0.1[:port]/path
#   http://localhost[:port]/path
#   http://[::1][:port]/path     (IPv6 loopback in canonical bracketed form)
# Rejected URL forms include http://user@host/, http://host.evil.com/,
# any non-loopback host, and any malformed URL.
_assert_insecure_download_loopback_only() {
  local mirror_url="$1"
  if [[ -z "${ACCELERATOR_VISUALISER_INSECURE_DOWNLOAD:-}" ]]; then
    return 0
  fi
  # Extract host using bash parameter expansion. Handles IPv6
  # brackets explicitly so [::1] is not mangled by ":" / "/" splitting.
  local rest="${mirror_url}"
  rest="${rest#*://}"          # strip scheme
  rest="${rest##*@}"           # strip optional userinfo (everything up to and incl. last @)
  local host
  if [[ "${rest}" == \[* ]]; then
    # Bracketed IPv6: [host]:port/path
    host="${rest#[}"
    host="${host%%]*}"
  else
    # Hostname or IPv4: host[:port]/path
    host="${rest%%[:/]*}"
  fi
  case "${host}" in
    127.0.0.1|::1|localhost)
      return 0
      ;;
    *)
      die_json "ACCELERATOR_VISUALISER_INSECURE_DOWNLOAD=1 is only valid against a loopback mirror (127.0.0.1, ::1, or localhost). Got host: ${host:-<unparseable>} from URL ${mirror_url}. Either remove the flag and use HTTPS, or point the mirror at a loopback address."
      ;;
  esac
}
```

The bash-parameter-expansion form handles three URL shapes
explicitly: bracketed IPv6 (`http://[::1]:port/...`), userinfo-prefixed
(`http://user@evil.com/...` — userinfo is stripped, so `evil.com`
hits the case fall-through and is rejected), and the canonical
`scheme://host[:port]/path` form. Sibling smoke tests cover each
accepted host (127.0.0.1, ::1 via bracketed form, localhost) and
the major rejection categories (`http://user@127.0.0.1/`,
`http://127.0.0.1.evil.com/`, schemeless URLs).

Sibling smoke tests covering every branch of the helper:

- `insecure_download_unset_does_not_check`: flag absent + remote
  URL → launcher proceeds (the helper short-circuits before any
  URL inspection).
- `insecure_download_with_127_0_0_1_allowed`: flag set + `http://127.0.0.1:port/`
  → launcher proceeds.
- `insecure_download_with_ipv6_loopback_allowed`: flag set +
  `http://[::1]:port/` → launcher proceeds (locks in the IPv6
  bracketed form support).
- `insecure_download_with_localhost_allowed`: flag set +
  `http://localhost:port/` → launcher proceeds.
- `insecure_download_with_remote_url_rejected`: flag set +
  `http://10.0.0.1/` → launcher fails with the documented
  `die_json` payload.
- `insecure_download_with_userinfo_rejected`: flag set +
  `http://user@evil.com/` → launcher fails (userinfo is stripped
  to expose the real host, which doesn't match the allow-list).
- `insecure_download_with_lookalike_host_rejected`: flag set +
  `http://127.0.0.1.evil.com/` → launcher fails (whole-host
  match against the allow-list, not prefix match).
- `insecure_download_with_schemeless_url_rejected`: flag set +
  `127.0.0.1:8080/path` (no scheme) → launcher fails (the
  parameter expansion can't reliably strip a missing scheme,
  so this falls into the reject branch).

### Changes Required

#### 0. Declare the `smoke` Cargo feature

**File**: `skills/visualisation/visualise/server/Cargo.toml`

The smoke test below is gated by `#[cfg(feature = "smoke")]` and the
mise task in §3 invokes it with `--features smoke`. Cargo requires
every feature passed via `--features` (and every feature referenced
in `cfg(feature = ...)`) to be declared in the consuming crate's
`[features]` table. The existing `[features]` block declares
`default = ["embed-dist"]` and `dev-frontend`; add the smoke gate
alongside them:

```toml
[features]
default = ["embed-dist"]
embed-dist = ["dep:rust-embed"]
dev-frontend = []
# Test-only feature: enables the binary-acquisition smoke test
# (#[cfg(feature = "smoke")] in tests/binary_acquisition_smoke.rs).
# Deliberately empty — gates only #[cfg], does NOT pull additional
# dependencies into the production binary build. Test-time deps
# (axum, reqwest) live under [dev-dependencies] and are never
# linked into a release build regardless of feature selection.
smoke = []

[dev-dependencies]
# ... existing dev-deps ...
axum = "0.8"           # in-process HTTP mirror for the smoke test
reqwest = "0.12"       # smoke test exercises /api/types via reqwest
```

The `smoke` feature is empty — it carries no `dep:` references —
so a release build invoked with `--features smoke` does NOT pull
test-only dependencies into the production binary. The smoke
test's runtime deps (`axum`, `reqwest`) are scoped to
`[dev-dependencies]`, which Cargo includes only when building
test/example targets.

**Success Criterion**: add an automated check that the feature is
recognised:

```bash
cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml \
    --features smoke -- --list 2>&1 \
  | grep -q '^binary_acquisition_smoke::fresh_install_downloads_and_launches:'
```

This grep fails if the feature is undeclared (cargo errors out
before listing tests) or if the test name drifts.

#### 1. Test fixture: a minimal "fake plugin" tree

**File**: `skills/visualisation/visualise/server/tests/fixtures/fake-plugin-tree/`

```
fake-plugin-tree/
├── .claude-plugin/plugin.json   # version: 1.99.0-test
├── scripts/
│   ├── config-read-path.sh      # symlink to real script (or stubbed)
│   └── vcs-common.sh            # symlink to real script
└── skills/visualisation/visualise/
    ├── bin/checksums.json       # populated dynamically by the test
    └── scripts/
        ├── launch-server.sh     # symlink to real launch-server.sh
        ├── launcher-helpers.sh
        ├── stop-server.sh
        └── write-visualiser-config.sh
```

The fake tree is the smallest set of files `launch-server.sh` needs
to reach the binary-fetch branch, with a deterministic version and a
manipulable checksums.json.

#### 2. The test

**File**: `skills/visualisation/visualise/server/tests/binary_acquisition_smoke.rs`

```rust
//! Smoke test deferred from Phase 11.5: ensures a fresh plugin
//! checkout downloads the right binary, verifies it, and launches.

use std::process::Command;

// Gated on a Cargo feature rather than #[ignore]: a feature gate
// fails compilation when invoked without `--features smoke`,
// which is loud and fails CI fast — versus #[ignore] which
// silently skips when `--ignored` is dropped from the test
// invocation. Phase 12.5's mise task always passes
// `--features smoke`, so the test runs in CI; `cargo test` alone
// without the feature simply doesn't see the test, with no
// silent-skip risk.
#[cfg(feature = "smoke")]
#[tokio::test(flavor = "multi_thread")]
async fn fresh_install_downloads_and_launches() {
    // 1. Build the release binary if missing.
    let binary = build_release_binary();

    // 2. Compute its SHA-256.
    let sha = sha256_of(&binary);

    // 3. Stand up an axum mirror at 127.0.0.1:<random>.
    let mirror_url = spawn_release_mirror(&binary, &sha).await;

    // 4. Lay out a fake plugin tree in a tempdir.
    let plugin_root = lay_out_fake_plugin_tree(&sha);

    // 5. Lay out a project with /accelerator:init already done.
    let project_root = lay_out_initialised_project();

    // 6. Invoke launch-server.sh with the mirror URL.
    let output = Command::new("bash")
        .arg(plugin_root.join("skills/visualisation/visualise/scripts/launch-server.sh"))
        .env("ACCELERATOR_VISUALISER_RELEASES_URL", &mirror_url)
        .env("ACCELERATOR_VISUALISER_SKILL_ROOT",
             plugin_root.join("skills/visualisation/visualise"))
        .env("CLAUDE_PLUGIN_ROOT", &plugin_root)
        .current_dir(&project_root)
        .output().unwrap();

    assert!(output.status.success(),
            "launch-server.sh failed: {}", String::from_utf8_lossy(&output.stderr));

    // 7. Extract URL from stdout.
    let url = parse_visualiser_url(&output.stdout);

    // 8. Hit /api/types — must return 200.
    let res = reqwest::get(format!("{url}/api/types")).await.unwrap();
    assert_eq!(res.status(), 200);

    // 9. Confirm the binary landed in the cache and is identical
    //    to the source binary (byte-for-byte).
    let cached = plugin_root.join(
        format!("skills/visualisation/visualise/bin/accelerator-visualiser-{os}-{arch}",
                os = current_os(), arch = current_arch()));
    assert_eq!(sha, sha256_of(&cached));

    // 10. Stop the server.
    Command::new("bash")
        .arg(plugin_root.join("skills/visualisation/visualise/scripts/stop-server.sh"))
        .env("CLAUDE_PLUGIN_ROOT", &plugin_root)
        .current_dir(&project_root)
        .status().unwrap();
}
```

#### 3. Mise integration

**File**: `mise.toml`

```toml
[tasks."test:integration:binary-acquisition"]
description = "Smoke-test launch-server.sh download/verify/launch flow"
depends = ["build:server:release"]
run = "cargo test --manifest-path skills/visualisation/visualise/server/Cargo.toml --test binary_acquisition_smoke --features smoke"
```

Add to `test:integration` depends. The dependency on
`build:server:release` ensures a fresh release binary is on disk
before the test starts.

### Success Criteria

#### Automated Verification:

- [ ] Test fails (RED) on a tree where `bin/checksums.json` has
      sentinel `0…0` values (validates the sentinel-rejection path).
- [ ] Test passes (GREEN) once the test populates a real
      `checksums.json` matching the released binary.
- [ ] `mise run test:integration:binary-acquisition` exits 0.
- [ ] On test exit, the cached binary remains valid (test doesn't
      leave the plugin tree in a broken state for follow-up
      iterations).
- [ ] Sibling failure-path tests pass:
      `sentinel_checksum_rejected_with_actionable_error` (set the
      manifest to `0…0`, expect the launcher to exit non-zero with
      a JSON error pointing at `ACCELERATOR_VISUALISER_BIN`),
      `sha_mismatch_aborts` (mirror serves a binary whose SHA
      differs from the manifest, expect the launcher to fail and
      remove the cache file), and `mirror_404_handled` (mirror
      returns 404, expect a network-error JSON). Each reuses the
      `spawn_release_mirror` infrastructure with minor variants.

#### Manual Verification:

- [ ] Running the test with `RUST_LOG=debug` prints the download URL,
      received SHA, expected SHA, and final URL — all consistent.
- [ ] Removing `bin/accelerator-visualiser-<os>-<arch>` between test
      runs causes a fresh download every time (re-validates the
      cache-miss path).

---

## Phase 12.6: Documentation (README + CHANGELOG + RELEASING.md)

### Overview

Add a `/accelerator:visualise` section to the README, a CHANGELOG
entry for the version bump that ships the visualiser binaries, and
a maintainer-facing `RELEASING.md` covering the release pipeline.

### Changes Required

#### 1. README section

**File**: `README.md`

Insert under the existing skills documentation (after the
`/accelerator:review-pr` section or wherever most-recently-added
skills land), a new `### Visualiser` (or `### /accelerator:visualise`)
section covering:

- **What it is**: a local browser-based visualiser of the project's
  `meta/` directory. Library, lifecycle, and kanban views.
- **How to launch**: `/accelerator:visualise` (slash command) or
  `accelerator-visualiser` (CLI wrapper, optionally symlinked onto
  `$PATH`).
- **First-run download**: ~8 MB binary fetched from GitHub Releases
  over HTTPS, verified against a committed SHA-256 manifest, cached
  under the plugin root. Requires outbound network access to
  `github.com` on first run.
- **Pre-release versions ship binaries too**: every plugin version
  the CI pipeline cuts (both `*-pre.N` and stable `X.Y.Z`) has its
  own four-platform release. Users on a pre-release plugin version
  get the matching pre-release binary automatically.
- **Customisation hooks**: `ACCELERATOR_VISUALISER_BIN` (one-shot
  override for local development against a hand-built binary),
  `visualiser.binary` config key (persistent override), and
  `ACCELERATOR_VISUALISER_RELEASES_URL` (alternative HTTPS mirror
  for air-gapped or self-hosted environments — must be HTTPS;
  plaintext is only supported via the localhost-only
  `ACCELERATOR_VISUALISER_INSECURE_DOWNLOAD` escape hatch).
  **The `INSECURE_DOWNLOAD` flag is enforced by code, not just
  documentation**: the launcher rejects the flag combined with
  any non-loopback mirror URL. Accepted hosts are `127.0.0.1`,
  `::1` (in canonical bracketed form `http://[::1]:port/`), and
  `localhost`; any other host produces a structured error.
- **Privacy & security**: localhost-only, dynamic port, no auth, no
  telemetry.
- **Opt-in provenance verification**: every released binary is
  signed with a SLSA-equivalent build provenance attestation
  (sigstore-keyless, transparency-log-backed). Default-on
  verification is the SHA-256 manifest, which proves the binary
  matches what the build runner produced. Provenance verification
  adds a second layer that proves the binary was produced by a
  specific GitHub Actions workflow on a specific commit — defence
  in depth against a compromised CI runner that could otherwise
  publish a tampered binary with a matching tampered manifest.
  - To enable: set `ACCELERATOR_VISUALISER_VERIFY_PROVENANCE=1`.
    The launcher runs `gh attestation verify --repo
    atomic-innovation/accelerator` after the SHA-256 check and
    refuses to launch if the attestation is missing or invalid.
  - Requires `gh >= 2.49.0` installed and authenticated, plus
    network reachability to GitHub's attestation API
    (`api.github.com`) — separate from the binary mirror at
    `github.com/atomic-innovation/accelerator/releases`. Air-gapped
    setups that mirror Releases but cannot reach `api.github.com`
    cannot use this flag.
  - Forks and private mirrors override the verifying repo via
    `ACCELERATOR_VISUALISER_PROVENANCE_REPO=<owner>/<repo>`. The
    `--repo`-scoped check (rather than `--owner`) prevents the
    byte-equal-fork attack where an attacker ships a binary
    identical to an upstream one in the hope of inheriting the
    upstream attestation.

Length target: **~50–80 lines** to match the existing skill
sections in this README (which average ~30 lines each). Detailed
mirror setup and customisation reference belong in the
`/accelerator:visualise` SKILL.md, not the top-level README.

#### 1a. RELEASING.md (maintainer audience)

**File**: `RELEASING.md` (new, repo root)

The release pipeline introduced in this phase needs a documented
home outside the archived plan, so a new maintainer can self-serve.
Cover:

- The CI pipeline shape: `test` → `prerelease` (every push) and
  `test` → `release` (gated by `release` Environment approval).
- The 16-step atomic flow with a brief one-line description per
  step (link to this plan's Phase 12.3 for the full detail).
- The prepare → attest → publish split that lets
  `actions/attest-build-provenance@v2.4.0` interleave between
  `release_binaries.build()` and the upload phase. Note that
  the local-dev `prerelease()` / `release()` tasks deliberately
  bypass attestation — CI is the only sanctioned release path.
- The "Failure modes by step" table (copy from Phase 12.3 above).
- How to run the full release pipeline locally for diagnostics
  (`mise run prerelease` against a fork — never against the main
  remote).
- Recovery procedures: `gh release delete <tag> --cleanup-tag`
  for cleaning up a bad publish; pushing a fresh `*-pre.N+1`
  through CI for forward recovery.
- **Incident response — halting prereleases**: the prerelease job
  is gated by a `prerelease` GitHub Environment configured without
  approvers. To halt all *future* prereleases without modifying
  code, a repo admin opens `Settings → Environments → prerelease`
  and either (a) sets a deployment branch policy that excludes
  `main` or (b) adds a required reviewer that does not approve.
  Re-enabling is the reverse.
  - **In-flight halt**: the Environment toggle does not stop a
    `prerelease` job that has already started; for that the
    operator must additionally run `gh run cancel <run-id>` (or
    use the Cancel button in the Actions UI). The full incident-
    response sequence is therefore: (1) flip the Environment
    toggle, (2) `gh run list --workflow=main.yml --status=in_progress`,
    (3) `gh run cancel <id>` for any in-flight prerelease.
  - **Cleanup of a published-but-bad release**: if a bad release
    made it past the verify gate before the kill switch was flipped,
    follow the recovery procedure in Phase 12.7 §1 of the plan
    (`gh release delete <tag> --cleanup-tag --yes` plus a jj
    revert of the version-bump commit).
  - This is the canonical kill switch; do not use code reverts as
    a substitute for incident response.
- **Out-of-band provenance verification**: any user can verify a
  downloaded binary's SLSA provenance independently of the launcher's
  opt-in flag:

  ```bash
  gh attestation verify accelerator-visualiser-<os>-<arch> \
      --repo atomic-innovation/accelerator
  ```

  Requires `gh >= 2.49.0` (when `attestation verify` was added).
  The same command runs inside `launch-server.sh` when
  `ACCELERATOR_VISUALISER_VERIFY_PROVENANCE=1` is set; documenting
  it here lets security auditors and pre-deployment reviewers run
  the check without launching the visualiser.
- **Workflow-step → mise task → Python function map**: during
  incident response the operator needs to trace from a failing CI
  step name to the line of code that ran. Mirrored in the workflow
  YAML, `mise.toml`, and `tasks/release.py`:

  | Workflow step (Actions UI label)     | mise task                      | Python invoke task                        |
  |--------------------------------------|--------------------------------|-------------------------------------------|
  | `Bump version + build binaries`      | `release:prerelease-prepare`   | `tasks.release.prerelease_prepare`        |
  | `Attest binary provenance`           | (none — workflow action)       | `actions/attest-build-provenance@v2.4.0`  |
  | `Commit, tag, push, publish`         | `release:prerelease-finalize`  | `tasks.release.prerelease_finalize`       |
  | `Stable: bump + build`               | `release:stable-prepare`       | `tasks.release.stable_prepare`            |
  | `Stable: attest`                     | (none — workflow action)       | `actions/attest-build-provenance@v2.4.0`  |
  | `Stable: publish`                    | `release:stable-publish`       | `tasks.release.stable_publish`            |
  | `Post-release pre.0: bump + build`   | `release:post-stable-prepare`  | `tasks.release.post_stable_prepare`       |
  | `Post-release pre.0: attest`         | (none — workflow action)       | `actions/attest-build-provenance@v2.4.0`  |
  | `Post-release pre.0: publish`        | `release:post-stable-publish`  | `tasks.release.post_stable_publish`       |

  The four `*_publish` tasks all delegate to a shared
  `_finalize_and_publish(context)` helper inside `tasks/release.py`,
  so a regression in the commit/tag/push/upload/verify/publish
  sequence shows up at one location regardless of which workflow
  step triggered it.

- **Why `mise run prerelease` / `mise run release` cannot be
  called from CI**: the single-call wrappers compose the prepare
  and finalize halves directly, **without** the workflow's
  attestation step between them. A release produced via the
  single-call path therefore has no SLSA build-provenance
  attestation, and any `gh attestation verify` invocation against
  it (whether via `ACCELERATOR_VISUALISER_VERIFY_PROVENANCE=1` or
  out-of-band) will fail. The `_refuse_under_ci` guard at the top
  of each wrapper raises `RuntimeError` if `GITHUB_ACTIONS` or
  `CI` is set, so the misconfiguration fails fast rather than
  silently producing an unprovenanced release.

- **Existing `release` GitHub Environment configuration**: the
  upstream repository's `release` Environment (used by the stable
  release job) is configured with: required reviewers — the
  release-owner team; deployment branch policy — `main` only;
  wait timer — 0 minutes. Forks setting up their own pipeline
  should mirror this configuration (or relax the required-reviewers
  constraint as appropriate) so the stable-cut approval gate
  matches upstream behaviour.
- **AssetVerificationError triage** (procedure for the forensic-
  preservation path): when a workflow run shows a `::error
  title=Visualiser release v<ver>::AssetVerificationError — draft
  + tag PRESERVED for triage` annotation, follow the procedure in
  Phase 12.3's "AssetVerificationError triage" section: download
  the suspect asset out-of-band, compare against the canonical hash
  in the tagged commit's `bin/checksums.json`, escalate as a
  security incident if confirmed, and only run `gh release delete
  v<ver> --cleanup-tag --yes` after triage closes. **Do not** treat
  a preserved draft as an orphan-cleanup case.
- **Symbolicating customer-reported crashes**: every release uploads
  `accelerator-visualiser-<os>-<arch>.debug.tar.gz` assets containing
  the unstripped binary (and `.dSYM` on darwin) for crash-symbolication.
  Download with `gh release download v<ver> --pattern '*.debug.tar.gz'`,
  extract, and use the platform-appropriate symbolicator (`addr2line`
  for ELF, `atos` or `lldb` for Mach-O / dSYM). Debug archives are
  **not** in `bin/checksums.json` and are never fetched by
  `launch-server.sh`, so SHA verification is not part of this flow.
- **Future extension points**:
  - `tasks/version.py` currently hard-codes the visualisation skill's
    `Cargo.toml` and `bin/checksums.json` paths. When a second
    binary-bearing skill arrives, introduce a `VersionedFile` registry
    (each skill registers its own coherence-tracked files + render
    functions; `tasks.version.write` iterates the registry) rather
    than adding a parallel pair of `_CARGO_TOML_2` constants. The
    `_atomic_write_text` primitive in `release_helpers.py` is already
    skill-agnostic.
- **Dependency pin maintenance**: the SLSA pipeline has two pinned
  dependencies that the rest of `mise.toml`'s pins don't cover:
  `actions/attest-build-provenance@v2.4.0` (workflow YAML) and
  the `gh >= 2.49.0` floor enforced by `launch-server.sh` for
  provenance verification. Bump these explicitly when pinning sweeps
  happen — they are not visible to `mise outdated` or similar.
- The four-platform manual smoke matrix (Phase 12.7 §3) as the
  acceptance gate before stable approval.
- Pointers to `tasks/release_helpers.py`, `tasks/release_binaries.py`,
  and `tasks/release.py` as the source of truth.

#### 2. CHANGELOG entry

**File**: `CHANGELOG.md`

Move all current "Unreleased" visualiser-related entries (from the
Phase 5 → Phase 11 plan deliverables) under the new version heading.
Add a single top-level summary line for the visualiser launch:

```markdown
## <next-version> — <release-date>

### Added

- **Meta visualiser** — a new browser-based companion view for the
  `meta/` directory. Launches via `/accelerator:visualise` or
  `accelerator-visualiser` CLI. Three views: library (markdown reader
  for every doc type), lifecycle (slug-clustered timelines), kanban
  (drag-drop ticket status updates). Distributed as per-arch native
  binaries via GitHub Releases (every plugin version, pre-release or
  stable, ships its own four-platform binaries); first run downloads
  ~8 MB over HTTPS and verifies against a committed SHA-256 manifest.
  See README for details.

  - Library reader for all 11 doc types with cross-reference rendering
    (Phase 5)
  - Lifecycle clusters and timeline view (Phase 6)
  - Read-only kanban (Phase 7)
  - Kanban write path with optimistic concurrency (Phase 8)
  - Wiki-link resolution `[[ADR-NNNN]]`, `[[TICKET-NNNN]]` (Phase 9)
  - Error handling, accessibility, and observability polish (Phase 10)

### Changed

- (any non-visualiser changes that landed in this release)

### Notes

- Air-gapped installs: point `ACCELERATOR_VISUALISER_RELEASES_URL`
  at an internal HTTPS mirror, or set `ACCELERATOR_VISUALISER_BIN`
  to a locally-built binary for offline use.
- Pre-release plugin versions ship matching pre-release binaries —
  dogfooding the visualiser does not require a local cargo build.
- The visualiser respects `paths.*` configuration: changing
  `paths.tickets` (etc.) routes the visualiser at the new location.
```

The version header and date are placeholders because this CHANGELOG
entry is authored at release-cut time, not pre-merge — by the time
Phase 12.4 activates and the first stable release is approved, the
actual version will be whatever the auto-bumper has produced.

### Success Criteria

#### Automated Verification:

- [ ] README, CHANGELOG, and `RELEASING.md` render correctly in
      `markdown-lint` / GFM-aware tooling (no broken links, no
      malformed lists).
- [ ] README new section is between 50 and 80 lines (verified via
      `awk '/^### .*[Vv]isualis/{flag=1} flag{print} /^### / && flag && NR>l{exit} {l=NR}' README.md | wc -l`
      or equivalent). Disproportionate length is a documentation
      smell against the existing terse style.
- [ ] README new section mentions each public customisation env var
      at least once: `ACCELERATOR_VISUALISER_BIN`,
      `visualiser.binary`, `ACCELERATOR_VISUALISER_RELEASES_URL`,
      `ACCELERATOR_VISUALISER_VERIFY_PROVENANCE` (verified via
      `grep` for each).
- [ ] `RELEASING.md` exists at the repo root and references
      `tasks/release_helpers.py`, `tasks/release_binaries.py`, and
      `tasks/release.py` by name.

#### Manual Verification:

- [ ] Reading the README's new section without prior context, a user
      understands what the visualiser is, how to launch it, and what
      first-run behaviour to expect.
- [ ] Reading the CHANGELOG entry, a user understands enough to
      decide whether to upgrade.

---

## Phase 12.7: First CI release

### Overview

The terminal phase. Merge Phases 12.1-12.6, observe the first CI-cut
pre-release with binaries, smoke-test on all four platforms, then
approve the first stable cut.

This phase is **not TDD-able** — it's a one-shot ceremonial action.
Confidence comes from the mocked integration tests in 12.3, the
mocked invoke-task tests in 12.4, and the binary acquisition smoke
test in 12.5 all having passed in CI.

### Changes Required

#### 0. GitHub UI prerequisites (one-time setup)

Before Phase 12.4 merges, a repo admin creates the `prerelease`
GitHub Environment that the workflow gates on:

1. `Settings → Environments → New environment`.
2. Name: `prerelease`.
3. Required reviewers: leave empty.
4. Wait timer: leave at 0.
5. Deployment branch policy: "Selected branches", add `main`.

The Environment exists purely as a kill switch — adding `main` to
the deployment branch policy means the prerelease job runs
unconditionally on every push to `main` (current behaviour); a repo
admin can later remove `main` from the policy to halt all
prereleases without modifying any code.

If the Environment is not created before 12.4 merges, the first
push to `main` will fail with "Environment 'prerelease' does not
exist". This is a one-time setup, easily reversible, and
audit-logged in the repo's settings history.

The existing `release` Environment (used by the stable release
job) is unchanged.

#### 1. Merge sequencing

The Phase 12 work lands across multiple PRs (ideally one per sub-phase)
that all merge to `main` before the first CI release attempts the
binary build. **Important**: each interim merge to `main` between
Phase 12.1 and Phase 12.4 will trigger the existing `prerelease` job,
which today does not build binaries — that's fine until 12.4 wires
the binary-build into the prerelease task. After 12.4 lands and CI
runs, the next push triggers the first ever CI prerelease with
binaries.

To minimise risk:

- Merge Phase 12.1 (`mise.toml` toolchain) first.
- Merge Phase 12.2 (helpers + tests) — no behavioural change yet.
- Merge Phase 12.3 (invoke task + tests) — no behavioural change yet
  (task exists but isn't called).
- Merge Phase 12.4 (wiring) — **this is the activation merge**. The
  next push to `main` after this merge triggers the first
  binary-cutting prerelease.
- Merge Phase 12.5 (smoke test) — adds the integration test gate.
- Merge Phase 12.6 (docs) — purely additive.

If the activation merge (12.4) produces a broken first prerelease,
the recovery sequence is:

1. **Delete the bad release on GitHub**:
   ```bash
   gh release delete v<bad-version> --cleanup-tag --yes
   ```
   `--cleanup-tag` deletes both the GitHub Release and the underlying
   git tag in one call.
2. **Revert the version-bump commit on `main`** (the prerelease job
   pushed it). Use `jj` (this repo's VCS) to revert it cleanly, then
   `jj git push` to publish the revert. This puts `plugin.json`,
   `Cargo.toml`, and `bin/checksums.json` back to the pre-failure
   state.
3. **Revert the Phase 12.4 PR** through the GitHub UI (or
   equivalent), so the next push doesn't immediately re-trigger
   the broken pipeline.
4. **Iterate locally** on a branch — re-merge once green.

Skipping any of these steps leaves the repo in an inconsistent
intermediate state (zombie tag, half-applied version bump, etc.).

#### 2. Watch the first prerelease cut

After the 12.4 merge, the next push to `main` triggers:
- `test` job — must pass.
- `prerelease` job — bumps to next `*-pre.N`, runs binary build,
  uploads four binaries, verifies, creates GH Release marked
  `--prerelease`.

Expected duration: ~10-15 minutes total (test ~3min, prerelease
~10min for cross-compile + upload + verify).

If the `prerelease` job fails:
- Inspect the CI log for the failing step.
- If the failure is in the orchestration layer: revert 12.4, fix
  locally, re-merge.
- If the failure is in the toolchain (e.g., zig version drift): pin
  a different version in `mise.toml`, re-merge.
- If the failure is in the upload (e.g., `GH_TOKEN` permissions):
  fix the workflow YAML, re-merge.

#### 3. Smoke test the first prerelease on all four platforms

For each of `darwin-arm64`, `darwin-x64`, `linux-arm64`, `linux-x64`:

1. On a clean machine for that platform (or a Docker container for
   linux-musl variants), install the plugin from the
   just-published pre-release.
2. In a fresh project, run `/accelerator:init`.
3. Run `/accelerator:visualise`.
4. Confirm: binary downloads, SHA verifies, server starts, browser
   loads `/library`.
5. Drag a ticket card on `/kanban` to confirm the write path works.
6. Run `bash skills/visualisation/visualise/scripts/stop-server.sh`.

If any platform fails:
- Open an issue describing the platform + error.
- The next CI push automatically supersedes the bad prerelease with
  a new one — the bad prerelease stays available for forensics but
  doesn't block forward progress (users on a pre-release plugin
  version pick up the new one on the next plugin update).

#### 4. Approve the first stable release

Once a prerelease has been smoke-tested green on all four platforms:

1. Create the version-bump PR that finalises (e.g., `1.20.0-pre.5`
   → `1.20.0`).
2. Merge that PR.
3. The `release` CI job runs and pauses for environment approval.
4. Approve the `release` environment in the GitHub UI.
5. The job: bumps to stable, builds four binaries, commits + tags +
   pushes, creates GH Release without `--prerelease`, uploads
   binaries, verifies, then bumps to next `*-pre.0`.

#### 5. Smoke-test the first stable release

Same four-platform sweep as in step 3, against the stable Release.

#### 6. Confirm CLI wrapper works

On any of the four platforms:

```bash
ln -s "$(claude plugin path accelerator)/skills/visualisation/visualise/cli/accelerator-visualiser" \
   ~/.local/bin/accelerator-visualiser
cd /tmp/fresh-project
accelerator-visualiser
```

Expected: same URL output as the slash command, server starts, URL
loads.

### Success Criteria

#### Automated Verification:

- [ ] CI `prerelease` job exits 0 on the first run after 12.4 merges.
- [ ] `gh release view <prerelease-tag> --json assets,isPrerelease`
      shows four assets and `isPrerelease=true`.
- [ ] CI `release` job (after the version-finalise PR merges and
      approval is granted) exits 0.
- [ ] `gh release view <stable-tag> --json assets,isPrerelease` shows
      four assets and `isPrerelease=false`.
- [ ] All four binaries' SHA-256 digests, computed via
      `curl -fsSL <asset-url> | sha256sum`, match
      `bin/checksums.json` on the tagged commit byte-for-byte.

#### Manual Verification:

- [ ] On `darwin-arm64`: clean install → `/accelerator:init` →
      `/accelerator:visualise` → URL loads → kanban drag works →
      stop-server cleans up. Repeat for both pre-release and stable.
- [ ] Same on `darwin-x64`.
- [ ] Same on `linux-arm64` (Docker container running
      `arm64v8/ubuntu:22.04` is acceptable — the binary is musl-static
      so distro-portable). On a non-arm64 host, this requires
      `qemu-user-static` registered with binfmt:
      `docker run --privileged --rm tonistiigi/binfmt --install all`.
- [ ] Same on `linux-x64`.
- [ ] CLI wrapper symlinked onto `$PATH` works identically to the
      slash command.
- [ ] The Release page on GitHub shows the four assets, the
      auto-generated release notes, the `Pre-release` banner (for
      pre-releases) or `Latest` banner (for stable), and the version
      tag.

---

## Testing Strategy

### Unit Tests (helper level)

Pytest cases in `tests/tasks/test_release_helpers.py`, one class per
helper function (four classes, ~16 cases total):
- Happy path with realistic inputs.
- Empty / missing input.
- Pre-existing manifest preserved on partial updates.
- Invalid input rejected via typed exceptions.
- Idempotency (running twice produces the same final state).
- Atomic-write recovery (`.tmp` cleanup on partial failure).
- Edge cases: BOM-prefixed JSON, `[workspace.package]` Cargo.toml,
  dependency tables containing `version = "..."`.

(Asset verification helpers live with the orchestration in
`tests/tasks/test_release_binaries.py`, not the helpers tests, since
`verify_uploaded_asset` itself moved out of `release_helpers.py`.)

### Integration Tests (invoke-task level)

Ten pytest cases in `tests/tasks/test_release_binaries.py` covering:
- Stable end-to-end (no `--prerelease` flag, `v`-prefixed draft tag,
  draft published only after verify).
- Pre-release end-to-end (`--prerelease` flag exactly once).
- `gh release create` uses `v`-prefixed tag with `--draft`.
- Publish (`gh release edit --draft=false`) runs after verify on
  the happy path; never runs if verify fails.
- Verify failure deletes the draft and re-raises.
- Upload failure (third of four uploads) deletes the draft and
  re-raises.
- Verify short-circuits on first mismatch (no further `subprocess.run`
  to `gh release download`).
- Missing tool aborts at preflight.
- Pre-update version drift aborts before any disk mutation.
- Real filesystem (`bin/checksums.json`) is byte-identical pre/post
  task invocation under test mocks.

Plus three pytest files for the existing-task extensions:
- `tests/tasks/test_version.py` — `write` updates Cargo.toml; the
  writer is anchored to `[package]` and ignores dependency tables.
- `tests/tasks/test_github.py` — `--prerelease` flag derived from
  version; `gh release create` argument exact-string-equality.
- `tests/tasks/test_release.py` — `prerelease()` and `release()`
  end-to-end mocked sequences.

All assertions use `pytest-mock`'s `mocker.patch` against
`invoke.Context.run` and `subprocess.run`. **No bash test runners,
no shell stubs, no `PATH` manipulation.**

### End-to-end Test (cargo `--features smoke`)

The binary acquisition smoke test from Phase 12.5 lives behind a
Cargo feature flag (`#[cfg(feature = "smoke")]`) rather than
`#[ignore]`. The mise task `test:integration:binary-acquisition`
invokes cargo with `--features smoke`, compiling and running the
test. `cargo test` without the feature flag does not see the test
at all — there is no silent-skip risk because there is no test to
skip. If the mise task were ever rewired in a way that dropped
`--features smoke`, the workflow would still fail loudly: the
test simply wouldn't run, and any CI assertion that depends on
its execution (e.g., a downstream check that grep's `cargo test`
output for the test name) would fail.

### Manual Test Matrix

The four-platform smoke matrix (Phase 12.7 success criteria) is the
final acceptance gate. Without machines for all four, partial coverage
is acceptable for an initial cut as long as `darwin-arm64` (the
maintainer's host) and one Linux variant pass.

## Performance Considerations

- **Cross-compile time**: `cargo zigbuild` for four targets on a
  GitHub-hosted `ubuntu-latest` runner: ~6-10 minutes (zig's linker
  is fast; the Rust frontend is the hot path; CI's mise cache speeds
  follow-up runs). Acceptable for the per-merge prerelease cadence.
- **Frontend bundle size**: with brotli compression in `rust-embed`,
  each binary embeds ~200-400 KB of frontend on top of the ~6-8 MB
  Rust binary. Net: ~7-9 MB per binary, well within GitHub Release
  asset limits.
- **First-run download**: single `curl` call, ~8 MB. Visible to the
  user via the `Downloading visualiser server (first run, ~8 MB)…`
  banner already in `launch-server.sh`.
- **Storage cost on GitHub**: with prereleases on every merge, the
  Releases storage grows by ~36 MB per merge (4 binaries × 9 MB).
  Over a year of active development (~200 merges) that's ~7 GB —
  inside GitHub's free tier for public repos. Pruning policy can
  come later.

## Migration Notes

For users on the last pre-Phase-12 plugin version or earlier:

1. The visualiser was previously available only via direct
   `cargo run` (no released binary). The pre-release sentinel-checksum
   path in `launch-server.sh` printed an error pointing at
   `ACCELERATOR_VISUALISER_BIN`.
2. Upgrading to the first version published by the new CI binary
   pipeline enables the binary download flow. No project-side
   migration is required — existing `meta/` directories work
   unchanged.
3. **Override precedence is unchanged**, but worth restating now
   that the cached-binary path is the default:
   `ACCELERATOR_VISUALISER_BIN` (env var, one-shot) >
   `visualiser.binary` (config, persistent) > cached binary >
   download. Users who previously needed `visualiser.binary` to
   point at a hand-built binary can now remove that config entry
   and let the auto-download flow handle distribution.
4. **Optional provenance verification**: security-conscious users
   can set `ACCELERATOR_VISUALISER_VERIFY_PROVENANCE=1` to require
   sigstore-signed build-provenance attestation on top of the
   default SHA-256 manifest check. Requires `gh` installed and
   authenticated against GitHub. Default behaviour (the variable
   unset) is unchanged from previous versions.
5. **Optional CLI wrapper installation**: a `cli/accelerator-visualiser`
   script in the plugin tree can be symlinked onto `$PATH` for
   non-Claude-Code launches:

   ```bash
   ln -s "$(claude plugin path accelerator)/skills/visualisation/visualise/cli/accelerator-visualiser" \
      ~/.local/bin/accelerator-visualiser
   ```

   This is documented as a one-time setup step in the README
   (Phase 12.6 §1) and is not required for the slash-command flow.

## References

- Spec: `meta/specs/2026-04-17-meta-visualisation-design.md` —
  Distribution section (lines 651-680), GitHub-Releases mechanics.
- Research: `meta/research/2026-04-17-meta-visualiser-implementation-context.md`
  — D8 (binary distribution), D10 (rust-embed frontend), Phase 12
  description (lines 1230-1273), Gap 7 (pre-release burden, line 1552).
- Existing infrastructure to honour:
  - `.github/workflows/main.yml` — CI pipeline (test → prerelease →
    release).
  - `tasks/release.py:6-41` — invoke-task release flows.
  - `tasks/github.py:6-13` — current `gh release create`.
  - `tasks/version.py` — semver bumping.
  - `tasks/build.py` — frontend + server build helpers.
  - `mise.toml` — toolchain pins + task graph.
  - `skills/visualisation/visualise/scripts/launch-server.sh:1-220`
    (binary-fetch flow).
  - `skills/visualisation/visualise/scripts/launcher-helpers.sh`
    (`sha256_of`, `download_to`, `die_json` patterns).
  - `skills/visualisation/visualise/server/build.rs:1-26`
    (frontend-dist freshness check).
  - `skills/visualisation/visualise/bin/checksums.json` (current
    placeholder).
  - `.gitignore:13-16` (binary + dist gitignore entries).
- Test patterns to mirror:
  - `scripts/test-config.sh` (bash-native test runner).
  - `tasks/test/` (pytest layout for invoke tasks).
- Previous phase plans:
  - Phase 11 (`meta/plans/2026-04-29-meta-visualiser-phase-11-testing.md`)
    — established `mise run test` as the top-level harness; Phase 12
    extends it with release-binary tests.
