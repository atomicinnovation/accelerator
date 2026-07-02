---
type: plan
id: "2026-07-02-0162-rust-toolchain-guard-rails"
title: "Rust Toolchain Guard Rails in mise + CI Implementation Plan"
date: "2026-07-02T01:23:10+00:00"
author: Toby Clemson
producer: create-plan
status: draft
work_item_id: "work-item:0162"
parent: "work-item:0162"
derived_from: ["codebase-research:2026-06-29-0162-rust-toolchain-guard-rails-wiring"]
relates_to: ["work-item:0163", "work-item:0166"]
tags: [rust, tooling, ci, guard-rails, architecture-enforcement]
revision: "76856fe0ae0a5e746df3bf86f238bb1404ca2f13"
repository: "accelerator"
last_updated: "2026-07-02T15:05:42+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Rust Toolchain Guard Rails in mise + CI Implementation Plan

## Overview

Extend the repo's four-toolchain enforcement to a new multi-crate `cli/` Rust
workspace: format, lint, tests-with-coverage, supply-chain (cargo-deny), and
intra-crate architecture enforcement (cargo-pup on an isolated pinned-nightly
lane). Because there is no Rust code outside the visualiser yet, this plan first
stands up a **minimal `cli/` workspace** as the substrate the gates run against,
so every phase leaves `mise run check` and the bare `mise run` green and is
independently mergeable. The paired scaffold story (0163) later builds the real
hexagonal launcher + `version` subcommand on top; the config split (0166) later
makes the infra-out-of-domain ban bite.

The implementation follows luminosity's proven wiring (work item 0006 → its
`cli/` workspace, `deny.toml`, `pup.ron`, `tasks/deny.py`, `tasks/pup.py`,
`tasks/deps.py`, `tasks/shared/rust.py`, and the `check-supply-chain` /
`check-architecture` CI jobs), adapted to accelerator's brownfield state
(top-level `cli/` under a plugin repo, an existing standalone visualiser-server
crate that stays put, and plugin.json/checksums.json version coherence).

## Current State Analysis

- **One stable toolchain, one crate.** `mise.toml:8` pins `rust = { version =
  "1.90.0", components = "rustfmt,clippy" }`. The only crate is the visualiser
  server at `skills/visualisation/visualise/server/`. No nightly, no
  cargo-nextest / cargo-llvm-cov / cargo-deny / cargo-pup, no `deny.toml` /
  `pup.ron` / `clippy.toml`.
- **Roll-ups are pure mise `depends`; Python tasks are leaves.** `mise.toml`
  aggregates (`server:check` at `mise.toml:305-307`, `check` at `354-356`) only
  `depends`; the work lives in `tasks/*` invoke leaves that raise `Exit(code=1)`.
- **`check` and `test` are disjoint.** No `test:*` task is reachable from
  `check`; `cargo test` appears only under `test:` (`tasks/test/unit.py:19`,
  `tasks/test/integration.py:47`, each re-deriving the server manifest literal).
- **Cargo scoping is `--manifest-path`, never `cd`** (`tasks/format/server.py:11`,
  `tasks/lint/server.py:15`); `-p <crate>` is not used anywhere today.
- **Version coherence assumes a single `[package]`.** `tasks/shared/paths.py:8`
  hard-codes one `CARGO_TOML`; `tasks/build.py:49-52,87-103` reads
  `data["package"]["version"]` into a single `"Cargo.toml"` key;
  `tasks/version.py:32-35` writes one manifest. No `[workspace].members` parsing
  exists.
- **CI fans `check` into per-component jobs; there is no aggregate-check job.**
  `check-visualiser-server` (`.github/workflows/main.yml:135-164`) carries a
  load-bearing `RUSTUP_HOME`-routing + `cache_key_prefix` workaround for the
  toolchain-component cache/parallel-race hazard. Test jobs run on a
  `[ubuntu-latest, macos-latest]` matrix. `tests/unit/tasks/test_workflows.py:87`
  asserts exactly two `accelerator-release` concurrency-lock members.

## Desired End State

`mise run check` runs, in addition to today's checks, a workspace-wide
`cli:check` (rustfmt `--check` + `cargo clippy --workspace -D warnings`),
`deny:check` (cargo-deny advisories/licenses/bans/sources), and `pup:check`
(cargo-pup on the pinned nightly), and exits 0 while staying read-only and
test-free. The bare `mise run` additionally runs cli tests with coverage
(collected, not gated). CI gains `check-cli`, `check-supply-chain`, and
`check-architecture` jobs (the last self-provisioning the isolated nightly lane)
plus cli coverage folded into the existing test-unit matrix. `tasks/shared/paths.py`
resolves every workspace member manifest, and version coherence spans
plugin.json, the standalone server manifest, the `cli/` workspace, and
checksums.json. A native-tls/OpenSSL dependency fails `deny:check`; a
domain-imports-adapter module fails `pup:check`; both are proven by automated
fixture regressions. With the nightly unavailable, only `check-architecture`
fails — every stable-lane gate and the product build stay green.

Because the workspace is single-crate at 0162's close, enforcement against real
shipped code is limited: cargo-deny's cross-crate bans are inert until the 0166
config split, and cargo-pup runs a rule with no real domain/adapter modules to
constrain until 0163/0166 land. In the interim the inward-dependency rule is
proven only by the fixture regressions, not by the shipped `launcher` crate — an
accepted co-verification boundary with no stable-lane floor (the grep tripwire was
deliberately rejected).

### Key Discoveries:

- Luminosity runs **one workspace-wide** `cargo clippy --workspace --all-targets
  --all-features -- -D warnings` as `lint:cli:check`
  (`../luminosity/tasks/lint/cli.py:10-12`); per-crate tasks (`kernel:check`)
  exist only ad-hoc and are excluded from the aggregate. **This plan adopts the
  workspace-wide approach**, relaxing 0162's per-crate AC1 (see What We're NOT
  Doing).
- mise's `[tools]` rust `components` is **silently skipped when the toolchain is
  already present** (jdx/mise-action#215); luminosity fixes this with a
  depended-on `deps:install:rust-components` running `rustup component add rustfmt
  clippy llvm-tools-preview` (`../luminosity/tasks/deps.py:25-39`). This plan
  adopts it and pairs new cargo CI jobs with `Swatinem/rust-cache`.
- mise **cannot pin two rust toolchains**; the nightly is rustup-managed via
  `deps:install:pup` (`../luminosity/tasks/deps.py:53-105`) and invoked `cargo
  +<nightly> pup`, not a mise per-task `tools` override.
- cargo-pup is `DataDog/cargo-pup`, config `pup.ron` (RON), rule `RestrictImports
  { denied: [...], severity: Error }`, needs nightly components `rustc-dev
  rust-src llvm-tools-preview`. `PUP_NIGHTLY`/`PUP_VERSION` are a matched pair
  (`nightly-2026-01-22` / `0.1.8`) — bump together.
- cargo-deny's `[bans]` cannot express "X must not be in the domain crate's
  closure" keyed on the dependent; the mechanism is **ban the infra crate
  globally + `wrappers = [permitted dependents]`**, so an unlisted (domain)
  dependent violates. The module-level direction rule is cargo-pup's job.
- `cargo llvm-cov nextest -p <crate> --summary-only` runs tests via nextest and
  reports coverage in one pass, report-only when no `--fail-under-*` flag is set
  (`../luminosity/tasks/test/cli.py:16-20`).

## What We're NOT Doing

- **No per-crate `<crate>:check` in the aggregate.** Following luminosity,
  `cli:check` is one workspace-wide pass; ad-hoc per-crate tasks may be added but
  stay out of the aggregate. 0162's acceptance criteria already carry this
  workspace-wide wording.
- **No grep dependency tripwire.** Deliberately rejected (ADR-0053); cargo-pup is
  the sole intra-crate enforcer.
- **No coverage gating.** Coverage is collected and reported; no threshold fails
  the run.
- **No infra-out-of-domain cross-crate fixture.** The ban is *scaffolded* here
  (empty `skip`/`skip-tree` with the wrapper mechanism documented); it first
  bites and is co-verified at the config/config-adapters split (0166).
- **No real `version` subcommand or hexagonal domain logic** — that is 0163. The
  `launcher` crate here is a minimal compile/lint/test substrate.
- **No visualiser fold-in.** The standalone server crate stays where it is; its
  fold into the workspace is 0168.
- **No branch-protection/required-checks config** — configured out-of-band in
  GitHub settings, not in-repo.

## Implementation Approach

Phase 1 lays the foundation (workspace + generalised version coherence).
Phases 2–5 each add one enforcement family as a self-contained, green,
independently-mergeable increment, wiring the leaf task, the mise aggregate, the
CI job, and (where applicable) an automated fixture regression together. Each of
Phases 2–5 is mergeable on top of the prior merged state (all depend on Phase 1);
they are **not** commutable — every phase edits `mise.toml` and Phases 2/4/5 all
edit `tasks/shared/rust.py`, so each phase's `check.depends` addition must
reference only tasks introduced in that same phase or Phase 1. TDD throughout:
tests/fixtures precede the coherence and enforcement wiring.

Shared constants live in a new `tasks/shared/rust.py` (crate names, `PUP_NIGHTLY`,
`PUP_VERSION`, `pup_mode()`, `coverage_enabled()`), mirroring luminosity. The
80-col width is hand-duplicated into `cli/rustfmt.toml`, `cli/deny.toml`, and
`cli/pup.ron` per the existing convention (further hand-kept copies alongside
`.editorconfig`, `pyproject.toml`, and `server/rustfmt.toml`; no automated sync
check exists). A committed `mise.lock` hash-pins the aqua-backed tool artifacts
(`lockfile = true` added to the **existing** `[settings]` block — a repo-global
setting, so it also pins the pre-existing `aqua:rhysd/actionlint`); each phase
that adds a `[tools]` entry refreshes it. The lock covers aqua backends only — the
from-source cargo-pup build and the rustup-managed nightly are **not** hash-pinned
by it and remain an accepted unverified surface for the isolated lane. Ported
luminosity code keeps only comments that explain a
non-obvious *why* (the GC'd-nightly fatal path, the mise-skips-components gotcha,
the call-time env-read requirement); incidental narration is pruned to match
accelerator's terse Rust leaves.

---

## Phase 1: Minimal `cli/` workspace + version-coherence generalisation

### Overview

Create a minimal virtual workspace at `cli/` and generalise the version-coherence
machinery from a single hard-coded manifest to enumerate workspace members, so a
member that opts out of version inheritance and drifts is detected. No
enforcement gates yet — this phase makes the substrate exist and keeps releases
coherent across it.

### Changes Required:

#### 1. The workspace manifest and launcher crate

**File**: `cli/Cargo.toml` (new)
**Changes**: Virtual workspace root; version held once at workspace level; the
resolved clippy lint set.

```toml
[workspace]
resolver = "2"
members = ["launcher"]

[workspace.package]
version = "1.24.0-pre.7"
edition = "2021"

[workspace.lints.rust]
warnings = "deny"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
unwrap_used = "warn"
expect_used = "warn"
panic = "warn"
dbg_macro = "warn"
todo = "warn"
unimplemented = "warn"
module_name_repetitions = "allow"
must_use_candidate = "allow"
```

**File**: `cli/launcher/Cargo.toml` (new)

```toml
[package]
name = "launcher"
version.workspace = true
edition.workspace = true

[lints]
workspace = true

[[bin]]
name = "accelerator"
path = "src/main.rs"
```

**File**: `cli/launcher/src/main.rs` (new) — minimal, clippy-clean under the
restriction lints.

```rust
fn main() {}
```

**File**: `cli/launcher/src/lib.rs` (new) — a trivial pure function with a unit
test, so Phase 5 coverage has something to instrument.

```rust
pub fn crate_name() -> &'static str {
    "launcher"
}

#[cfg(test)]
mod tests {
    use super::crate_name;

    #[test]
    fn reports_its_name() {
        assert_eq!(crate_name(), "launcher");
    }
}
```

The version literal must equal the current plugin version at implementation
time; read it from `.claude-plugin/plugin.json` rather than copying this draft's
value.

#### 2. Shared paths + member enumeration

**File**: `tasks/shared/paths.py`
**Changes**: Add `import tomllib` (the module imports only `pathlib` today), the
workspace manifest constant, and a member-enumeration helper taking a **required**
manifest path (no default, so the injectable test path is always exercised and a
rootless test can never silently read the real repo manifest) alongside the
existing standalone-server `CARGO_TOML`.

```python
import tomllib

CLI_DIR = REPO_ROOT / "cli"
CLI_WORKSPACE_CARGO_TOML = CLI_DIR / "Cargo.toml"


def load_toml(path: Path) -> dict:
    with path.open("rb") as f:
        return tomllib.load(f)


def cli_member_manifests(workspace_manifest: Path) -> list[Path]:
    members = load_toml(workspace_manifest)["workspace"]["members"]
    return [workspace_manifest.parent / m / "Cargo.toml" for m in members]
```

The shared `load_toml` unifies the open+load idiom that `build.py`'s coherence
helpers below also use; the workspace manifest is parsed at most twice per
coherence pass (once for the version, once for the member list), which is
negligible I/O.

#### 3. Version-coherence read

**File**: `tasks/build.py`
**Changes**: Define `_CLI_WORKSPACE_CARGO_TOML_RELATIVE =
CLI_WORKSPACE_CARGO_TOML.relative_to(REPO_ROOT)` (mirroring the existing
`_CARGO_TOML_RELATIVE`). `validate_version_coherence` gains the workspace version
plus one entry per member that pins its own literal `[package].version`, so the
**single** existing `!= expected_version` comprehension in
`validate_version_coherence` performs all filtering (no second filtering site). A
member that inherits (`version.workspace = true`, so `tomllib` parses `version`
as a `{"workspace": True}` table, not a string) contributes no entry and is never
a mismatch; a member that pins its own drifting version string is named. A
missing `[workspace.package].version` raises the existing `VersionCoherenceError`
with a clear message rather than a bare `KeyError`.

```python
def _read_workspace_version(root: Path) -> str:
    data = load_toml(root / _CLI_WORKSPACE_CARGO_TOML_RELATIVE)
    try:
        return data["workspace"]["package"]["version"]
    except KeyError as exc:
        raise VersionCoherenceError(
            f"{_CLI_WORKSPACE_CARGO_TOML_RELATIVE}: "
            "missing [workspace.package].version"
        ) from exc


def _pinned_member_versions(root: Path) -> dict[str, str]:
    pinned = {}
    for manifest in cli_member_manifests(
        root / _CLI_WORKSPACE_CARGO_TOML_RELATIVE
    ):
        version = load_toml(manifest).get("package", {}).get("version")
        if isinstance(version, str):
            pinned[manifest.relative_to(root).as_posix()] = version
    return pinned
```

`validate_version_coherence`'s `found` dict gains `"cli/Cargo.toml"` (from
`_read_workspace_version`) and every entry from `_pinned_member_versions`; the
existing comprehension then flags any whose value `!= expected_version`, naming
the offending crate. A missing `[workspace].members` key, or a listed member whose
`Cargo.toml` is absent, is likewise translated to `VersionCoherenceError` (naming
the fault) rather than surfacing a bare `KeyError`/`FileNotFoundError` at release
time. The new cli entries are keyed by repo-relative posix path (`cli/Cargo.toml`,
`cli/<member>/Cargo.toml`); the existing bare-basename plugin.json / server
Cargo.toml / checksums.json keys are unchanged, so no source collides.

#### 4. Version write

**File**: `tasks/version.py`
**Changes**: `write` also renders the `cli/` workspace `[workspace.package].version`
(single write site) via a `_render_workspace_cargo_toml` using tomlkit (preserving
the `[workspace.lints.clippy]` table and comments). The standalone server write is
unchanged. The write helper assumes a well-formed `[workspace.package]` table
(mirroring the existing `_render_cargo_toml`'s `[package]` assumption); this
asymmetry with the validating read side is intentional — the read side
(`validate_version_coherence`) is what surfaces a malformed manifest with a clear
`VersionCoherenceError`, and `write` runs against a manifest that side has
validated.

#### 5. Test scaffolding

**File**: `tests/conftest.py`
**Changes**: `fake_repo_tree` also writes a minimal `cli/Cargo.toml` workspace
(`members = ["launcher"]`, `[workspace.package].version`) and a `cli/launcher/
Cargo.toml` inheriting via `version.workspace = true`. At least one existing
coherence test is confirmed still green with the new members present, so the
shared-fixture mutation cannot make a pre-existing test pass for the wrong reason.

**File**: `tests/unit/tasks/test_build.py`
**Changes** (test-first): coherence passes when the workspace version matches and
members inherit; a workspace-version mismatch names `cli/Cargo.toml`; a member
pinning a drifting `[package].version` is named; an inheriting member is not; an
empty `members = []` is a no-op (not a silent pass masking missing coverage); a
`cli/Cargo.toml` missing `[workspace.package].version`, a missing
`[workspace].members` key, and a listed-but-absent member `Cargo.toml` each raise
`VersionCoherenceError` naming the fault.

**File**: `tests/unit/tasks/test_version.py`
**Changes** (test-first): the workspace manifest round-trip preserves
`[workspace.lints.clippy]` and its comments; `write` updates the workspace
version. (The edition-vs-`cli/rustfmt.toml` guard,
`test_cli_cargo_and_rustfmt_editions_match`, lands in Phase 2 alongside
`cli/rustfmt.toml` — where both operands exist — rather than as a trivial
self-comparison here.)

#### 6. Consolidate the re-derived server manifest literals

**File**: `tasks/test/unit.py`, `tasks/test/integration.py`
**Changes**: Replace the two hand-copied
`repo_root() / "skills/visualisation/visualise/server/Cargo.toml"` literals
(`unit.py:19`, `integration.py:47`) with the existing `CARGO_TOML` import from
`tasks/shared/paths.py`, so every manifest reference routes through `paths.py`
and the new `CLI_WORKSPACE_CARGO_TOML` follows one established pattern (a small
in-scope cleanup the 0168 fold-in would otherwise have to chase down). Behaviour
is unchanged and covered by the existing server test leaves.

### Success Criteria:

#### Automated Verification:

- [x] Build-system checks pass: `mise run build-system:check`
- [x] Task unit tests pass: `uv run pytest tests/unit/tasks/test_build.py tests/unit/tasks/test_version.py -v`
- [x] The workspace compiles: `cargo build --manifest-path cli/Cargo.toml`
- [x] Version coherence holds at the current version: `mise run version:read` then `uv run pytest tests/unit/tasks/test_build.py -k coherence -v`
- [x] After the manifest-literal consolidation, the server test leaves still resolve their manifest: `mise run test:unit` and `mise run test:integration` (server portions) pass
- [x] `mise run check` still exits 0 (no new gates yet)

#### Manual Verification:

- [x] `cli/Cargo.toml` version equals `.claude-plugin/plugin.json` version
- [x] Editing `launcher`'s `[package].version` to a different value makes coherence fail and names `cli/launcher/Cargo.toml` (covered by `test_member_pinning_drifting_version_is_named`)

---

## Phase 2: Workspace-wide format + clippy (`cli:check`) into `check` + CI

### Overview

Add rustfmt `--check` and workspace-wide clippy `-D warnings` as `cli:check`,
folded into `check` and the family aggregates, plus the depended-on
`deps:install:rust-components` and a `check-cli` CI job.

### Changes Required:

#### 1. rustfmt config + shared rust constants

**File**: `cli/rustfmt.toml` (new) — `edition = "2021"`, `max_width = 80`, a
hand-duplicated copy of the `.editorconfig` width per the existing no-auto-sync
convention (joining `server/rustfmt.toml` and `pyproject.toml`).

**File**: `tasks/shared/rust.py` (new)

```python
LAUNCHER_CRATE = "launcher"
```

**File**: `tests/unit/tasks/test_version.py`
**Changes** (test-first): add `test_cli_cargo_and_rustfmt_editions_match`
asserting `cli/Cargo.toml`'s `[workspace.package].edition` equals
`cli/rustfmt.toml`'s `edition` (both operands now exist), guarding the two
hand-kept copies against drift.

#### 2. Component provisioning

**File**: `tasks/deps.py`
**Changes**: Add `install_rust_components` running `rustup component add rustfmt
clippy llvm-tools-preview` (explicit, since mise skips components on an
already-present toolchain; `llvm-tools-preview` pre-empts the parallel
coverage-time auto-install race).

**File**: `mise.toml`
**Changes**: `deps:install:rust-components` executor.

#### 3. Format + lint leaves

**File**: `tasks/format/cli.py` (new) — `check`/`fix` running `cargo fmt
--manifest-path cli/Cargo.toml --all [-- --check]`, raising `Exit` on drift
(mirroring `tasks/format/server.py`).

**File**: `tasks/lint/cli.py` (new) — `check`/`fix` running `cargo clippy
--manifest-path cli/Cargo.toml --workspace --all-targets --all-features [-- -D
warnings | --fix --allow-dirty]` (mirroring `../luminosity/tasks/lint/cli.py`).
Its docstring notes, mirroring `lint:server:fix`, that `--fix` applies only the
machine-rewritable subset (lints such as `unwrap_used` cannot be auto-fixed), so
`cli:check` must still be run for the remainder.

**File**: `tasks/format/__init__.py`, `tasks/lint/__init__.py`,
`tasks/__init__.py`
**Changes**: Register the `cli` modules.

#### 4. mise wiring

**File**: `mise.toml`
**Changes**: `format:cli:check`/`fix`, `lint:cli:check`/`fix` (each `depends =
["deps:install:rust-components"]`), a `cli:check` aggregate, and wire `cli:check`
into `check.depends`, `format:check`/`fix`, `lint:check`/`fix`.

**File**: `tests/unit/tasks/test_mise.py` (new)
**Changes** (test-first): a textual-structure guard parsing `mise.toml` (mirroring
`test_workflows.py`'s style) asserting `cli:check` appears in `check.depends`, so
the gate cannot be silently unwired from `check`. Phases 3–4 extend it for
`deny:check` / `pup:check`.

#### 5. CI job

**File**: `.github/workflows/main.yml`
**Changes**: New `check-cli` job (ubuntu) — checkout, the `RUSTUP_HOME` routing
into the mise data dir set before `mise install` plus its own per-job
`cache_key_prefix` (e.g. `mise-cli-v1`), `jdx/mise-action`, `Swatinem/rust-cache`,
`mise run cli:check`. Add `check-cli` to `prerelease.needs`. No
`accelerator-release` concurrency group (respects the `test_workflows.py`
two-member guard).

The two caching mechanisms are complementary and cache disjoint artifacts: the
`RUSTUP_HOME` routing + per-job `cache_key_prefix` isolates the rustup toolchain
(the research flags this as load-bearing for any new cargo job — a mise cache hit
can otherwise leave the toolchain absent while parallel cargo passes race to
auto-install it), while `Swatinem/rust-cache` caches `target/`. Each new cargo job
takes a **distinct** prefix so jobs — and the ubuntu/macos matrix legs — never
share a toolchain-cache namespace. The pre-existing `check-visualiser-server`
keeps its `RUSTUP_HOME` routing + `mise-server-v1` prefix and is knowingly left
without a `Swatinem/rust-cache` target cache in this work (its fold-in is 0168).

#### 6. Docs

**File**: `tasks/README.md`, `CLAUDE.md`
**Changes**: Add the `cli` component row to the per-component table, noting that
Rust enforcement spans `cli:check` (format + workspace-wide clippy) **plus** the
standalone `deny:check` / `pup:check` entity tasks wired directly into top-level
`check` (they sit outside the `cli:` roll-up, mirroring `version:*` / `github:*`),
so the documented task tree stays complete. Note the workspace-wide clippy pass,
the `deps:install:rust-components` dependency, and the `lint:cli:fix`
machine-fixable-subset caveat. Add a CI-job → local-command mapping (`check-cli` →
`mise run cli:check`) so a red job is reproducible locally; the mapping is
extended in Phases 3–4.

### Success Criteria:

#### Automated Verification:

- [x] `mise run cli:check` exits 0
- [x] `mise run check` includes `cli:check` and exits 0
- [x] Format drift is caught: introducing mis-formatting fails `mise run format:cli:check`
- [x] A clippy violation (e.g. an `unwrap()`) fails `mise run lint:cli:check` (proven: the nursery `missing_const_for_fn` finding failed `cli:check` under `-D warnings` until fixed)
- [x] Workflow lint + topology guard pass: `mise run lint:workflows:check` and `uv run pytest tests/unit/tasks/test_workflows.py -v`

#### Manual Verification:

- [ ] `check-cli` runs on a PR and is green
- [x] `cli:check` stays read-only (no files mutated, no tests run)

---

## Phase 3: cargo-deny (bans) into `check` + CI + regression

### Overview

Pin cargo-deny, add `cli/deny.toml` encoding the workspace-wide native-tls/OpenSSL
ban (demonstrable now) and scaffolding the infra-out-of-domain ban (inert until
0166), wire `deny:check` into `check`, add a `check-supply-chain` CI job, and
prove the native-tls ban fires with an automated fixture regression.

### Changes Required:

#### 1. Tool pin + lockfile

**File**: `mise.toml`
**Changes**: `[tools]` gains `"aqua:EmbarkStudios/cargo-deny" = "0.19.8"`
(aligned to the luminosity-validated version; carry a short why-pinned inline
comment mirroring the existing `aqua:rhysd/actionlint` precedent). Add `lockfile =
true` to the **existing** `[settings]` block (not a second block) and commit the
generated `mise.lock`, hash-pinning the aqua tool artifacts locally — this narrows
the aqua-backend trust surface (it does **not** cover the from-source cargo-pup
build or the rustup nightly). `mise.lock` must be regenerated to carry entries for
every target platform (`x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`,
`x86_64-apple-darwin`) so a lock authored on one arch does not force a fetch — or
dirty the tree — on another matrix leg; ensure it is not caught by a broad
`*.lock` gitignore. Phases 4–5 refresh it as they add tools.

#### 2. deny.toml

**File**: `cli/deny.toml` (new), held to ≤80 columns per the hand-duplication
convention. Adapted from `../luminosity/deny.toml`:
- `[graph].targets` — named explicitly as the four shipped release triples from
  `tasks/shared/targets.py` plus `x86_64-unknown-linux-gnu` (the ubuntu CI dev
  graph), rather than the ambiguous "gnu/darwin dev triples", so the ubuntu graph
  a banned edge could hide in is always evaluated (the darwin triples are already
  in the shipped set). The cli/ workspace does not yet cross-compile these triples
  in 0162, so the ban guards a not-yet-built graph — necessary but not sufficient
  for the musl-static guarantee until the cli cross-compile build lands.
- `[advisories]` — `version = 2` (pinned so vulnerability-gating is not left to
  version-sensitive defaults), `unmaintained = "all"`, `yanked = "deny"`, and the
  deny-on-vulnerability policy stated explicitly.
- `[licenses]` — `version = 2` (both sections opt into the stable v2 schema, so
  license evaluation is not left on version-sensitive v1 defaults either), an
  explicit permissive allow-list, `confidence-threshold = 0.8`.
- `[bans]` — deny `native-tls`, `openssl`, `openssl-sys` with empty
  `skip`/`skip-tree` and a comment explaining the global-ban + `wrappers`
  mechanism the currently-empty lists reserve (the comment stays free of
  work-item references per the comment convention).
- `[sources]` — crates.io only.

#### 3. deny leaf + mise

**File**: `tasks/deny.py` (new) — `check` running `cargo deny check advisories
licenses bans sources` from the `cli/` dir, raising `Exit` on findings. Its
docstring notes the deliberate `cd cli/` (cargo-deny resolves `deny.toml`
relative to cwd) versus the `--manifest-path` form the fmt/clippy leaves use.

**File**: `tasks/__init__.py`, `mise.toml`
**Changes**: Register `deny`; add `deny:check` (no `depends` — cargo-deny is a
mise tool) and fold into `check.depends` and the bare `default`.

**File**: `tests/unit/tasks/test_mise.py`
**Changes** (test-first): extend the topology guard to assert `deny:check` appears
in `check.depends`.

#### 4. Automated ban regression (test-first)

**File**: `tests/integration/deny/test_native_tls_ban.py` (new) — exercise the
**real** `cli/deny.toml` against a minimal generated fixture manifest, with a
committed fixture `Cargo.lock` (pinning `native-tls` + its transitive `openssl`)
so `cargo deny check bans` runs offline against a fixed graph rather than
resolving from crates.io in the ubuntu+macos matrix. Assert the violation exits
non-zero **and** names `native-tls`/`openssl` specifically; assert the clean
fixture (no native-tls) exits zero. The test controls cwd/config so it proves the
shipped ban fires, not a copy of it.

**File**: `tasks/test/integration.py`, `mise.toml`
**Changes**: Add `test.integration.deny` leaf (`pytest tests/integration/deny`)
and `test:integration:deny`, wired into `test:integration`.

#### 5. CI job

**File**: `.github/workflows/main.yml`
**Changes**: New `check-supply-chain` job (ubuntu, `mise run deny:check`) with the
`RUSTUP_HOME` routing + its own per-job `cache_key_prefix` (e.g.
`mise-supply-chain-v1`; cargo-deny shells `cargo metadata`, so the toolchain-cache
race applies); the RustSec DB is fetched fresh, never cached stale. Add to
`prerelease.needs`.

#### 6. Docs

**File**: `tasks/README.md`, `CLAUDE.md`
**Changes**: Extend the CI-job → local-command mapping with `check-supply-chain` →
`mise run deny:check`.

### Success Criteria:

#### Automated Verification:

- [x] `mise run deny:check` exits 0 on the clean workspace
- [x] The ban regression passes offline (networking disabled) on both ubuntu and macos: `mise run test:integration:deny` (verified offline locally via `--frozen`; ubuntu confirmed in CI)
- [x] A `native-tls`/`openssl` dependency added to `cli/` fails `mise run deny:check` (proven by the offline fixture regression against the real `cli/deny.toml`)
- [ ] `mise install` on both the ubuntu and macos runners leaves `mise.lock` unmodified (`git status` clean), confirming the committed lock covers all target platforms (populated for linux-x64/macos-arm64/macos-x64 via `mise lock`; cross-runner confirmation is CI-only)
- [x] `mise run check` includes `deny:check` and exits 0

#### Manual Verification:

- [ ] `check-supply-chain` runs on a PR and is green
- [x] `cli/deny.toml` width stays ≤ 80 columns

---

## Phase 4: cargo-pup nightly lane into `check` + CI + regression (isolated)

### Overview

Provision the pinned nightly + cargo-pup rustup-managed, add `pup.ron` and
`pup:check` (with a `warn` escape hatch), wire it into `check`, add an isolated
`check-architecture` CI job self-provisioning the nightly, prove a
domain-imports-adapter module fails with an automated fixture regression, and
assert lane isolation structurally.

### Changes Required:

#### 1. Verify the installed binary first (mandatory prerequisite)

The whole deny-mode model rests on cargo-pup 0.1.8's exit-code contract, so this
is the phase's mandatory first step, not an aside. Confirm against the pinned
nightly: whether a `severity: Error` violation exits non-zero, the exact exit code
and the violation-message/rule-name text it prints, and how config placement
applies across workspace members. Record the confirmed contract in the fixture as
a short note. If the bare CLI does not gate by exit code, adopt the documented
fallback — an `assert_lints`-style wrapper — and shape both the leaf and the
regression around the observed contract before the CI job depends on it.

#### 2. Shared constants (no `[tools]` entry for cargo-pup)

cargo-pup is **not** added to mise `[tools]`: a `cargo:` backend would build it
from source against the mise stable toolchain (1.90.0), which lacks the
`rustc-dev`/`rust-src` nightly components its `rustc_private` ABI requires,
yielding a binary that cannot load against `nightly-2026-01-22` — and it would
duplicate/conflict with `deps:install:pup`. cargo-pup is provisioned solely
through `deps:install:pup` (below), matching luminosity.

**File**: `tasks/shared/rust.py`
**Changes**: Add `PUP_NIGHTLY = "nightly-2026-01-22"`, `PUP_VERSION = "0.1.8"`
(matched pair; the single source of truth for the version), and `pup_mode()`
reading `ACCELERATOR_PUP_MODE` (default `deny`, fail-closed on unrecognised,
normalised) — adapted from `../luminosity/tasks/shared/rust.py:28-49`.

#### 3. Nightly provisioning

**File**: `tasks/deps.py`
**Changes**: Add `install_pup` — `rustup toolchain install {PUP_NIGHTLY}
--profile minimal --component rustc-dev --component rust-src --component
llvm-tools-preview`, then `cargo +{PUP_NIGHTLY} install cargo_pup --version
{PUP_VERSION} --locked` (guarded by a version-token presence probe), then a
`cargo +{PUP_NIGHTLY} --version` preflight. Adapted from
`../luminosity/tasks/deps.py:53-105`. The toolchain-install failure is caught and
re-raised as an `Exit` whose message names the pinned `{PUP_NIGHTLY}` and directs
the reader to bump `PUP_NIGHTLY`/`PUP_VERSION` together in `tasks/shared/rust.py`,
so a GC'd nightly yields an actionable message rather than a raw rustup stack
trace. `cargo install --locked` pins cargo-pup's transitive build deps.

**File**: `mise.toml`
**Changes**: `deps:install:pup` executor.

#### 4. pup.ron + leaf

**File**: `cli/pup.ron` (new), held to ≤80 columns per the hand-duplication
convention — a `RestrictImports` rule authored against the **actual** accelerator
scaffold module names (not luminosity's `version::core`/`kernel`, which do not
exist here until 0163). The rule passes over the minimal scaffold; because the
scaffold has no real domain/adapter modules yet, the pass is near-vacuous, so the
regression (§5) is what proves the rule's discriminating power. Adapted from
`../luminosity/pup.ron`.

**File**: `tasks/pup.py` (new) — `check` running `cargo +{PUP_NIGHTLY} pup` from
`cli/`, honouring `pup_mode()` (`warn` logs and returns; `deny` raises `Exit`).
Its docstring notes the deliberate `cd cli/` (pup resolves `pup.ron` relative to
cwd). Adapted from `../luminosity/tasks/pup.py`.

**File**: `tasks/__init__.py`, `mise.toml`
**Changes**: Register `pup`; add `pup:check` (`depends = ["deps:install:pup"]`)
and fold into `check.depends` and the bare `default`.

**File**: `tests/unit/tasks/test_mise.py`
**Changes** (test-first): extend the topology guard to assert `pup:check` appears
in `check.depends`.

#### 5. Automated architecture regression + mode unit tests (test-first)

**File**: `tests/integration/pup/test_import_rule.py` (new) — a fixture crate
whose `domain`/`adapters` modules are laid out as a member subdir under the
workspace (mirroring the real cli/ shape, since pup config placement applies
per-member), with a `domain` module importing an `adapters` module; assert `cargo
+nightly pup` (with a rule denying that import) fails on the concrete contract
confirmed in §1 — the observed exit code **and** a match on the violation
message/rule name in output — so a tool that logs-but-exits-zero fails the test
rather than passing it. Add a **positive control** — a fixture where a permitted
import is present and explicitly allowed — so a passing run means "evaluated and
allowed", not "evaluated nothing" (guarding against a rule whose module scope
silently matches nothing). Exercise the real `cli/pup.ron` rule shape, controlling
cwd/config, so the rule's discriminating power is proven, not assumed.

**File**: `tests/unit/tasks/test_rust.py` (new) — unit-test `pup_mode()` (default
`deny`; `warn`→`warn`, proving the documented escape hatch is recognised;
`off`/any unrecognised value→`deny` fail-closed — valid modes are `{deny, warn}`,
mirroring luminosity's `_PUP_MODES`) and the leaf's branch (`warn` returns without
raising; `deny` raises `Exit` on a non-zero pup result); also `coverage_enabled()`
(each of off/false/0/no → disabled; default and unrecognised → enabled) and its
leaf branch (llvm-cov vs plain nextest), so both escape-hatch helpers are
regression-protected rather than left to one-shot commands. Also (in
`test_deps.py`) assert `install_pup`'s toolchain-install failure is re-raised as
an `Exit` whose message names `PUP_NIGHTLY` — mocked, so no real nightly is
needed — converting the actionable-error contract from a manual check to an
automated one.

**File**: `tasks/test/integration.py`, `mise.toml`
**Changes**: Add `test.integration.pup` leaf and `test:integration:pup`
(`depends = ["deps:install:python", "deps:install:pup"]`) — **not** wired into
the `test:integration` roll-up (it needs the nightly; it runs only in
`check-architecture`).

#### 6. CI job + isolation guard

**File**: `.github/workflows/main.yml`
**Changes**: New `check-architecture` job (ubuntu) with the `RUSTUP_HOME` routing +
its own per-job `cache_key_prefix` (e.g. `mise-architecture-v1`),
`Swatinem/rust-cache`, running `mise run pup:check` then `mise run
test:integration:pup` (pup rebuilt from source each run — no trustworthy per-OS
prebuilt checksum). It sets no `ACCELERATOR_PUP_MODE`, so CI runs in the
fail-closed `deny` default (`warn` can never be the CI default). Add to
`prerelease.needs`.

**File**: `tests/unit/tasks/test_workflows.py`
**Changes** (test-first): encode lane isolation with a concrete detection rule and
parametrised known-bad mutations (mirroring the existing concurrency guard):
- A job is a nightly consumer iff it runs `pup:check`, `deps:install:pup`, or
  `+nightly`.
- Assert `check-architecture` is the only nightly consumer, and that it invokes
  **both** `pup:check` and `test:integration:pup` (so the sole regression cannot
  be silently dropped from its one host job).
- Assert no stable-lane / product job `needs:` `check-architecture`.
- Known-bad mutations that must each fail the guard: a `needs: check-architecture`
  edge injected into a stable job; a `+nightly` step added to another job.

#### 7. Docs

**File**: `tasks/README.md` (canonical), `CLAUDE.md` (pointer)
**Changes**: `tasks/README.md` is the single canonical home for the toolchain
guard-rail narrative (it already owns the task-tree shape); CLAUDE.md and the
top-level README carry only short pointers to it, not parallel prose. Add there a
nightly-lane orientation subsection (mirroring the "Executable-bit invariant"
style) covering: what the second toolchain is and why it exists; that it is
isolated (a nightly break blocks only `check-architecture`, never the stable lane
or product build); the expected multi-minute first-run `deps:install:pup` build;
the `PUP_NIGHTLY`/`PUP_VERSION` matched-pair bump procedure and GC-window
liability; the `mise.lock` refresh step (regenerate + commit on any `[tools]`
edit); and the `ACCELERATOR_PUP_MODE=warn` escape hatch — noting it is a
**local-only advisory override that CI ignores** (CI always runs `deny`), so the
fix for a red `check-architecture` is the architecture violation, not the env var
(the warn-path log line says the same). These are the repo's first `ACCELERATOR_*`
contributor toggles, so introduce a **new** contributor env-var table in
`tasks/README.md` (not the user-facing visualiser/browser tables in the top-level
README) and add `ACCELERATOR_PUP_MODE` to it. Extend the CI-job → local-command
mapping with `check-architecture` → `mise run pup:check` (+ `test:integration:pup`).
Note in the bump procedure that any tool-version bump — the aqua pins or the
`PUP_NIGHTLY`/`PUP_VERSION` pair — should verify the upstream release's published
checksum/attestation before committing the refreshed `mise.lock`, mirroring the
SHA-256/SLSA discipline the repo already applies to the visualiser binary via
`checksums.json`.

### Success Criteria:

#### Automated Verification:

- [x] `mise run deps:install:pup` provisions the nightly + cargo-pup idempotently (second run skipped the rebuild via the presence probe)
- [x] cargo-pup is absent from mise `[tools]`; `mise run pup:check` uses the `deps:install:pup`-provisioned nightly binary
- [x] `mise run pup:check` exits 0 on the clean workspace
- [x] The architecture regression passes: `mise run test:integration:pup`
- [x] A domain-imports-adapter fixture fails `cargo +nightly pup` on the confirmed exit-code + message contract (exit 101, "is not allowed", rule named)
- [x] Mode + leaf-branch unit tests pass: `uv run pytest tests/unit/tasks/test_rust.py -v`
- [x] `mise run check` includes `pup:check` and exits 0
- [x] Isolation guard passes: `uv run pytest tests/unit/tasks/test_workflows.py -v`

#### Manual Verification:

- [ ] `check-architecture` runs on a PR and is green; it is the only nightly consumer (structurally guarded by `test_workflows.py`; runtime confirmation is CI-only)
- [ ] With the nightly made unavailable, only `check-architecture` fails — every stable job and the product build stay green (CI-only)
- [x] `ACCELERATOR_PUP_MODE=warn mise run pup:check` downgrades a violation to advisory (covered by `test_rust.py::TestPupCheck::test_warn_mode_logs_and_returns_cleanly`)

---

## Phase 5: Tests + coverage (`cargo llvm-cov nextest`) into `test`

### Overview

Pin cargo-nextest + cargo-llvm-cov, run cli tests via nextest with coverage
folded in (collected, not gated), wired into `test:unit` only, and add
`Swatinem/rust-cache` to the cargo-running test jobs.

### Changes Required:

#### 1. Tool pins + coverage toggle

**File**: `mise.toml`
**Changes**: `[tools]` gains `"aqua:nextest-rs/nextest/cargo-nextest" = "0.9.138"`
and `"aqua:taiki-e/cargo-llvm-cov" = "0.8.7"`, each with a short why-pinned inline
comment mirroring the existing `aqua:rhysd/actionlint` precedent — the
`/cargo-nextest` sub-tool-path rationale (the repo publishes its binary as
`cargo-nextest`; without the path the aqua backend cannot resolve the asset)
belongs as an inline comment in `mise.toml`, not only here. Refresh the committed
`mise.lock`.

**File**: `tasks/shared/rust.py`
**Changes**: Add `coverage_enabled()` reading `ACCELERATOR_COVERAGE` (default on;
off/false/0/no disable) — adapted from `../luminosity/tasks/shared/rust.py:14-25`.

#### 2. Coverage leaf

**File**: `tasks/test/cli.py` (new) — `run` executing `cargo llvm-cov nextest
--manifest-path cli/Cargo.toml --workspace --summary-only` when coverage is
enabled, else `cargo nextest run --manifest-path cli/Cargo.toml --workspace`;
report-only, no threshold. Adapted from `../luminosity/tasks/test/cli.py`.

**File**: `tasks/test/__init__.py`, `mise.toml`
**Changes**: Register the module; add `test:unit:cli` (`depends =
["deps:install:rust-components"]`) and wire into `test:unit`. **Not** in `check`.

#### 3. CI build caching

**File**: `.github/workflows/main.yml`
**Changes**: Add `Swatinem/rust-cache` plus the `RUSTUP_HOME` routing + a per-job,
OS-disambiguated `cache_key_prefix` to the `test-unit` (and `test-integration`)
jobs — these now compile Rust on both the ubuntu and macos matrix legs (previously
only ubuntu's server-check compiled Rust), so the toolchain-cache race applies
here too, and the matrix legs must not share a namespace. Components arrive via the
`deps:install:rust-components` mise dependency.

#### 4. Docs

**File**: `tasks/README.md` (canonical), `CLAUDE.md` (pointer)
**Changes**: Add `ACCELERATOR_COVERAGE=off` to the `tasks/README.md` contributor
env-var table introduced in Phase 4, alongside `ACCELERATOR_PUP_MODE`, as one
"toolchain escape hatches" entry.

### Success Criteria:

#### Automated Verification:

- [ ] `mise run test:unit:cli` runs the launcher tests via nextest and emits a coverage summary
- [ ] Coverage is not gated: a low-coverage change still exits 0
- [ ] `ACCELERATOR_COVERAGE=off mise run test:unit:cli` runs plain nextest (no instrumentation)
- [ ] `test:unit:cli` is absent from `check`: `mise run check` does not run cli tests
- [ ] `mise run` (bare default) exits 0 end-to-end including cli coverage

#### Manual Verification:

- [ ] All three cargo tools (cargo-nextest, cargo-llvm-cov, cargo-deny) resolve at their pinned versions on both `ubuntu-latest` and `macos-latest`
- [ ] `test-unit` passes on both ubuntu and macos with cli coverage folded in
- [ ] The coverage summary appears in the test-unit job log

---

## Testing Strategy

### Unit Tests:

- Version coherence: workspace-version match, member inheritance vs drift,
  offending-crate naming, empty-members no-op, and a missing
  `[workspace.package].version` raising `VersionCoherenceError` (`test_build.py`).
- tomlkit round-trip preserving `[workspace.lints.clippy]` + comments; workspace
  version write (`test_version.py`); edition-vs-rustfmt guard, added in Phase 2.
- `pup_mode()` (default deny, warn→warn, off/unrecognised→deny fail-closed) and the
  pup leaf's warn/deny branch; `coverage_enabled()` disable-token parsing and its
  llvm-cov/plain-nextest branch (`test_rust.py`).
- Workflow topology: nightly-lane isolation via a concrete detection rule with
  known-bad mutations, `check-architecture` invoking both `pup:check` and
  `test:integration:pup`, and the existing two-member concurrency-lock invariant
  (`test_workflows.py`).
- mise task topology: `cli:check`, `deny:check`, and `pup:check` are each wired
  into `check.depends` (`test_mise.py`).

### Integration Tests:

- cargo-deny native-tls/OpenSSL ban fires against the real `cli/deny.toml` on an
  offline fixture (committed `Cargo.lock`), asserting the named crates
  (`tests/integration/deny/`).
- cargo-pup domain-imports-adapter rule fires against the real `cli/pup.ron` on the
  confirmed exit-code + message contract (`tests/integration/pup/`, nightly lane
  only).

### Manual Testing Steps:

1. Open a PR touching `cli/` and confirm `check-cli`, `check-supply-chain`, and
   `check-architecture` all run and are green.
2. Add a `native-tls` dependency to a `cli/` member and confirm
   `check-supply-chain` goes red.
3. Add a domain→adapter import to the launcher and confirm only
   `check-architecture` goes red.
4. Temporarily point `PUP_NIGHTLY` at an unavailable nightly and confirm only
   `check-architecture` fails; every stable job and the product build stay green.

## Performance Considerations

- cargo-pup rebuilds from source on `check-architecture` each run (no trustworthy
  prebuilt checksum); `Swatinem/rust-cache` amortises the workspace compile but
  not the pup install — acceptable for an isolated architecture lane.
- `deps:install:pup` is presence-probed so the multi-minute pup build is skipped
  in steady state locally.
- Coverage instrumentation recompiles the workspace; kept in `test`, never
  `check`, so the read-only inner loop stays fast.

## Migration Notes

- No data migration. The `cli/` workspace version must be seeded equal to the
  current plugin version; thereafter `version:write` keeps it coherent, so a
  release after Phase 1 will bump `cli/Cargo.toml` alongside plugin.json /
  checksums.json / the server manifest.
- A committed `mise.lock` is introduced with the first new aqua tool (Phase 3);
  a contributor's first `mise install` after pulling honours the pinned artifact
  hashes.
- 0162's acceptance criteria already carry the workspace-wide `cli:check` wording
  this plan implements (per-crate `<crate>:check` is optional and excluded from the
  aggregate).

## References

- Original work item: `meta/work/0162-rust-toolchain-guard-rails.md`
- Related research: `meta/research/codebase/2026-06-29-0162-rust-toolchain-guard-rails-wiring.md`
- Paired scaffold: `meta/work/0163-scaffold-cli-workspace-version-subcommand.md`
- Infra-ban co-verification: `meta/work/0166-shared-config-corpus-store-crates.md`
- ADRs: ADR-0046, ADR-0048, ADR-0053, ADR-0054
- Reference implementation (luminosity 0006): `../luminosity/` — `mise.toml`,
  `Cargo.toml`, `deny.toml`, `pup.ron`, `tasks/deps.py`, `tasks/deny.py`,
  `tasks/pup.py`, `tasks/shared/rust.py`, `tasks/test/cli.py`,
  `tasks/lint/cli.py`, `.github/workflows/main.yml`
