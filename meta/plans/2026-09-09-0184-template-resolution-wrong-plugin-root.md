---
type: "plan"
id: "2026-09-09-0184-template-resolution-wrong-plugin-root"
title: "Template Resolution Wrong-Root Refusal Implementation Plan"
date: "2026-09-09T07:15:11+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0184"
parent: "work-item:0184"
derived_from: ["codebase-research:2026-09-08-0184-template-resolution-wrong-plugin-root"]
tags: ["cli", "config", "templates", "plugin-root", "visualiser"]
revision: "48a9f5347c558bb678beff1be719c6d99d661b82"
repository: "accelerator"
last_updated: "2026-09-09T16:29:30+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Template Resolution Wrong-Root Refusal Implementation Plan

## Overview

Extend 0182's *absent*-root refusal to the *wrong*-root case — a present plugin
root that carries no `templates/` directory, so it is not an Accelerator
installation — across the three plugin-root template accessors in
`FileConfigStore`. Today those accessors pass the `require_plugin_root` gate on a
wrong root (it is `Some(path)`) and fall through their own filesystem checks into
a silent empty answer or a false template-not-found. After this change every
structural wrong-root shape — no `templates/`, a `templates` that is a file, or a
root that is itself a file — makes all six template commands and the visualiser
compose path refuse with a dedicated `ConfigError::PluginRootNotAnInstallation`
diagnostic that names the offending root and the invalid `templates/` layout (and
`ACCELERATOR_PLUGIN_ROOT`), while a genuine missing template inside a valid
installation, resolving user overrides, and the root-independent config families
all keep working unchanged. A distinct
variant — rather than reusing `PluginRootUnavailable`, whose message says the root
is *unknown* and to *set* the variable — is what lets the wrong-root diagnostic
tell a developer who already set the variable that their root simply is not an
installation, and name which path.

## Current State Analysis

`require_plugin_root` (`cli/config-adapters/src/store.rs:73`) is the only gate
raising `ConfigError::PluginRootUnavailable`, and it fires solely on an *absent*
root. A present-but-wrong root reaches each accessor's own check:

| Accessor | Site | Wrong-root behaviour today |
|----------|------|----------------------------|
| `template_names` | `store.rs:412` | `let Ok(entries) = read_dir(..) else { Ok(Vec::new()) }` swallows every error |
| `resolve_template` plugin-default tier | `store.rs:400` | `<name>.md` not a file → `Ok(None)` (names no template) |
| `plugin_template_path` | `store.rs:466` | builds the path, tests nothing |
| `plugin_default` | `store.rs:436` | routes through `plugin_template_path`, then `is_file()` → `Ok(None)` |
| `eject` | `store.rs:483` | routes through `plugin_template_path`, then `is_file()` → `EjectOutcome::NoDefault` |

Each maps to an observable command, all exiting **1** today (a refusal becomes
`kernel::Error::Failed` → `ExitCode::FAILURE`; a wrong root instead degrades the
diagnosis). `PluginRootNotAnInstallation` maps through the same `From<ConfigError>`
arm, so the new refusal also exits **1**:

| Command | Store site | Wrong-root outcome today |
|---------|------------|--------------------------|
| `config templates list` | `template_names` | empty table, exit 0 |
| `config templates eject --all` | `eject_all` → `template_names` | ejects nothing, exit 0 |
| `config template <name>` | `resolve_template` tier | `Ok(None)` → not-found |
| `config templates eject <name>` | `eject` → `plugin_template_path` | `EjectOutcome::NoDefault` |
| `config templates diff` / `reset <name>` | `plugin_default` → `plugin_template_path` | `Ok(None)` → unknown-template |
| visualiser compose | `resolve_templates` → `template_names` | empty template set |

`template_names` is also consumed by the visualiser server compose path:
`resolve_templates` loops `store.template_names()?` (`cli/visualiser/server/src/compose.rs:157`)
and `ComposeError::Config(#[from] ConfigError)` (`compose.rs:20`) re-emits the
wrapped message verbatim through `#[error("configuration error: {0}")]`.

## Desired End State

On a structural wrong root (no `templates/`, `templates` a file, or a root that is
a file), every template command and the compose path refuses:

- `config templates list`, `template <name>` (no resolving override), `eject`,
  `eject --all`, `diff`, `reset` each exit non-zero with the
  `PluginRootNotAnInstallation` diagnostic — naming the offending root, the
  invalid `templates/` layout, and `ACCELERATOR_PLUGIN_ROOT` — even under
  `--fail-safe`.
- `compose::load` returns `Err(ComposeError)` whose message names the offending
  root and `ACCELERATOR_PLUGIN_ROOT` rather than composing an empty template set.

The distinctions that must survive:

- The wrong-root message **differs** from the absent-root message: it names the
  offending path and states the root is not an installation, rather than saying
  the root is *unknown* and to *set* the variable.
- A valid installation missing one `<name>.md` still reports a template-not-found
  naming `<name>`, **not** the plugin root.
- A resolving user override for `<name>` still renders at exit 0 under a wrong
  root (the override tiers precede the plugin-default check).
- Root-independent families (`config paths`, `config summary`, …) still exit 0
  with their normal non-empty output, **including under `--fail-safe`**.
- A genuine *unreadable* fault (permission denied, filesystem loop) yields
  `ConfigError::Io` naming the path, stays degradable, and does **not** name
  `ACCELERATOR_PLUGIN_ROOT` — the one wrong-root shape that does not fail closed,
  because the root's validity is genuinely undeterminable.

This refines the work item's **AC8**. As written, AC8 treats a `templates` path
that is a file as the representative *genuine I/O fault* (→ `Io`); under this plan
that shape is a structural refusal, and `Io` is reserved for a genuinely
unreadable fault (permission, filesystem loop). AC8 and its trigger example should
be updated in `meta/work/0184-…` to match, and a criterion added for the two extra
structural shapes (`templates` a file, root a file) refusing — a work-item
follow-up, not done in this plan.

### Key Discoveries

- `plugin_template_path` (`store.rs:466`) already returns `Result<PathBuf,
  ConfigError>` and is the single choke both `plugin_default` (`:436`) and `eject`
  (`:483`) route through — fixing it there fixes both callers at once.
- The plugin-default check must sit **inside** each plugin-default step, after the
  override tiers miss: `resolve_template` checks the config-path and user-override
  tiers first (`store.rs:374-399`), and hoisting the root check above them would
  refuse a resolving override. Pinned by `a_user_override_still_resolves_with_no_plugin_root`
  (`cli/launcher/tests/config_read.rs:1356`).
- The `NotFound`-vs-`Io` split to mirror already exists in `custom_lenses`
  (`store.rs:296`) and `skill_names` (`store.rs:322`), constructing `Io` via the
  `io_error` helper (`store.rs:729`). This item's divergence: `NotFound` maps to
  `PluginRootNotAnInstallation`, not an empty list — resting on the
  installation-ships-`templates/` invariant, not sibling parity.
- `PluginRootUnavailable.is_refusal()` is `true` (`cli/config/src/error.rs:78`),
  so `Failure::from` tags it `Failure::Refusal` (`cli.rs:424`) and `finish`'s
  degrade arm — which matches `Failure::Read` only (`cli.rs:463`) — never absorbs
  it. Behaviour is byte-identical with and without `--fail-safe`.
  `PluginRootNotAnInstallation` is classified `is_refusal() == true` alongside it,
  so it fails closed identically. `is_refusal` is exhaustive inside the config
  crate (no wildcard, `error.rs:76-88`), so adding the variant does not compile
  until it is classified — the classification cannot be defaulted by omission.
- R3 needs **no production change**: the `?` at `compose.rs:157` already
  propagates the refusal with its message intact. AC6 is a test, and it passes
  only once R1 lands (today a wrong root composes successfully with an empty map).
- The packaging invariant holds for the git-tag distribution model: 13 templates
  are tracked under `templates/` and ride along with the checkout; nothing filters
  them out of the release artifact.
- Test scaffolding to reuse (`config_read.rs`): `run_with_plugin_root` (`:1226`,
  sets the var to any root — the wrong-root invoker), `run_with_plugin` (`:1219`,
  the committed valid fixture at `tests/fixtures/plugin/templates/{demo,other}.md`),
  `run_in` (`:71`, strips the var), `assert_refuses_without_a_plugin_root`
  (`:1280`), `assert_names_the_plugin_root` (`:1270`). The characterisation test
  to replace is `a_root_without_a_templates_directory_still_renders_an_empty_table`
  (`:1373`).

## What We're NOT Doing

- Detecting a mis-pointed root that happens to retain an unrelated `templates/`
  directory — indistinguishable from a valid installation by this signal, out of
  scope.
- Touching `known_skill_names` (`store.rs:338`) — deliberately left tolerant by
  0182; not a template accessor.
- Changing the root-independent config families — they never read the plugin root
  and must stay unaffected (R4).
- The visualiser frontend's surface-level handling of the propagated
  `ComposeError` — downstream of the server boundary, out of scope.
- `cli/visualiser/server/src/templates.rs` (`TemplateResolver`) — a downstream
  renderer of the already-composed map, its `plugin_root` display-only; not a
  store consumer.
- Consolidating the `<plugin-root>/templates` **layout** fact, which
  `compose.rs:155` still reconstructs with `plugin_root.join("templates")` to build
  each tier's absolute `plugin_default` path. The validity *gate* is centralised
  (compose refuses via `store.template_names()?`); relocating the per-name path
  construction behind a store accessor is a wider refactor of the compose tier
  build, deliberately out of scope here. The residual duplication is a conscious
  choice, not an oversight.
- Generalising `require_templates_dir` into a resource-agnostic
  `require_installation_root()` predicate. Only `templates/` presence is the
  installation signal today and there is no second consumer; the helper stays
  templates-specific (YAGNI) rather than speculatively abstracted.
- Renaming the helper away from the `require_*` gate family. `require_templates_dir`
  reads as a path accessor but can refuse; the name is kept for consistency with
  `require_plugin_root` / `require_secure_personal_file`, and the doc comment
  states the refusal.

## Implementation Approach

A dedicated error variant, `ConfigError::PluginRootNotAnInstallation { path }`,
carries the wrong-root refusal, and one shared installation-detection helper —
`require_templates_dir` — validates the `templates/` directory, raises that
variant on any structurally-invalid `templates/`, and returns the directory's
path; all three accessors call it. The helper probes with `fs::metadata` (a
single `stat`), so the plugin-default sites pay one `stat` and `template_names`
pays `stat` + `read_dir`; the structural shapes (missing, a file, root-is-a-file)
are each explicit. The three phases are strictly ordered but each is
independently mergeable and leaves the tree green. Every phase drives the store
change from a failing test first: unit tests on `FileConfigStore` for the tight
red-green loop, CLI integration tests in `config_read.rs` to pin the observable
acceptance criteria, and a compose contract test for the server boundary.

The new variant is added to `ConfigError` in `cli/config/src/error.rs` — a
struct-like variant `PluginRootNotAnInstallation { path: String }`, a `Display`
arm naming the path and the missing-`templates/` cause, and a `true` arm in
`is_refusal` (which is exhaustive, so the classification is forced):

```rust
Self::PluginRootNotAnInstallation { path } => write!(
    formatter,
    "the plugin root '{path}' is not an Accelerator \
     installation (no templates/ directory); check \
     ACCELERATOR_PLUGIN_ROOT"
),
```

The shared helper, added to a `FileConfigStore` impl block near
`plugin_template_path`:

```rust
/// The installation's `templates/` directory. A root whose `templates/`
/// is absent or not a directory — or a root that is itself a file — is
/// not an installation and refuses with `PluginRootNotAnInstallation`,
/// resting on the invariant that every installation ships `templates/`.
/// A genuine unreadable fault surfaces as the degradable `Io`.
fn require_templates_dir(&self) -> Result<PathBuf, ConfigError> {
    let root = self.require_plugin_root()?;
    let templates = root.join("templates");
    let not_an_installation = || ConfigError::PluginRootNotAnInstallation {
        path: display(root),
    };
    match fs::metadata(&templates) {
        Ok(metadata) if metadata.is_dir() => Ok(templates),
        Ok(_) => Err(not_an_installation()),
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::NotFound | ErrorKind::NotADirectory
            ) =>
        {
            Err(not_an_installation())
        }
        Err(error) => Err(io_error(&templates, &error)),
    }
}
```

Every *structural* wrong-root shape refuses and fails closed under `--fail-safe`:
`templates/` missing (`NotFound`), `templates` present but a file (`Ok(!is_dir)`),
and a root that is itself a file (`NotADirectory` on the join). `Io` is reserved
for a genuine *unreadable* fault — a permission error or a filesystem loop — where
the root's validity is genuinely undeterminable; only that case stays degradable.
`ErrorKind::NotADirectory` is stable on the pinned toolchain (Rust 1.90);
`NotFound` and `NotADirectory` collapse to one refusal arm.

## Phase 1: `template_names` refuses on a wrong root (R1)

### Overview

Introduce the `PluginRootNotAnInstallation` variant and `require_templates_dir`,
and route `template_names` through the helper, so enumeration refuses on a wrong
root instead of swallowing the failed `read_dir`. This alone fixes `config
templates list` and `config templates eject --all`.

### Changes Required

#### 1. The error variant

**File**: `cli/config/src/error.rs`
**Changes**: Add the struct-like variant `PluginRootNotAnInstallation { path:
String }` to `ConfigError`, its `Display` arm (above), and a `true` arm in
`is_refusal` alongside `PluginRootUnavailable`. `is_refusal` is exhaustive, so the
crate does not compile until the variant is classified.

#### 2. The shared helper and `template_names`

**File**: `cli/config-adapters/src/store.rs`
**Changes**: Add `require_templates_dir` (above). Replace `template_names`' swallow
with a call to it, then map any enumeration error to `Io`:

```rust
fn template_names(&self) -> Result<Vec<String>, ConfigError> {
    let templates = self.require_templates_dir()?;
    let entries = fs::read_dir(&templates)
        .map_err(|e| io_error(&templates, &e))?;
    let mut files: Vec<String> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            Path::new(name).extension().and_then(|e| e.to_str()) == Some("md")
        })
        .collect();
    files.sort();
    Ok(files
        .into_iter()
        .filter_map(|name| name.strip_suffix(".md").map(str::to_owned))
        .collect())
}
```

The `stat`-then-`read_dir` pair is a deliberate double probe. The helper
establishes not-an-installation — a persistent condition — via `metadata`; a
`read_dir` failure *after* a confirmed directory is a transient concurrent-mutation
fault, so mapping every such error (including a racing `NotFound`) to `Io`
(degradable) is correct: a directory that vanishes mid-enumeration degrades under
`--fail-safe` rather than being misreported as not-an-installation.

#### 3. Error-variant unit tests (red first)

**File**: `cli/config/src/error.rs` (test module)
**Changes**: Mirror `plugin_root_unavailable_names_the_variable_and_the_bootstrap`
and the refusal-classification test for the new variant:

- `plugin_root_not_an_installation_names_the_path_and_variable`: assert the
  rendered message contains the path passed in and `ACCELERATOR_PLUGIN_ROOT`, and
  does **not** contain "unknown".
- Extend `a_missing_plugin_root_is_a_refusal_and_a_read_failure_is_not` to assert
  `PluginRootNotAnInstallation { .. }.is_refusal()` is `true`.

#### 4. Store unit tests (red first)

**File**: `cli/config-adapters/src/store.rs` (test module; add `ReadTemplate` to
the test imports)
**Changes**: Drive the change from failing unit tests that build a store with a
plugin root — a pattern not yet present in this crate. Bind the root `TempDir` to
a local first (matching the module idiom), so it outlives the store:

```rust
#[test]
fn template_names_refuses_a_root_without_a_templates_dir(
) -> Result<(), TestError> {
    let root = tempdir()?;
    let plugin = tempdir()?;
    let store = FileConfigStore::at(root.path())
        .with_plugin_root(Some(plugin.path().to_path_buf()));
    assert!(matches!(
        store.template_names(),
        Err(ConfigError::PluginRootNotAnInstallation { .. })
    ));
    Ok(())
}

#[test]
fn template_names_refuses_when_templates_is_a_file(
) -> Result<(), TestError> {
    let root = tempdir()?;
    let plugin = tempdir()?;
    fs::write(plugin.path().join("templates"), "not a dir")?;
    let store = FileConfigStore::at(root.path())
        .with_plugin_root(Some(plugin.path().to_path_buf()));
    assert!(matches!(
        store.template_names(),
        Err(ConfigError::PluginRootNotAnInstallation { .. })
    ));
    Ok(())
}

#[test]
fn template_names_refuses_when_the_root_is_a_file(
) -> Result<(), TestError> {
    let root = tempdir()?;
    let holder = tempdir()?;
    let plugin_file = holder.path().join("not-a-dir");
    fs::write(&plugin_file, "")?;
    let store =
        FileConfigStore::at(root.path()).with_plugin_root(Some(plugin_file));
    assert!(matches!(
        store.template_names(),
        Err(ConfigError::PluginRootNotAnInstallation { .. })
    ));
    Ok(())
}

#[cfg(unix)]
#[test]
fn template_names_reports_io_on_an_unreadable_templates(
) -> Result<(), TestError> {
    use std::os::unix::fs::symlink;
    let root = tempdir()?;
    let plugin = tempdir()?;
    symlink("templates", plugin.path().join("templates"))?;
    let store = FileConfigStore::at(root.path())
        .with_plugin_root(Some(plugin.path().to_path_buf()));
    assert!(matches!(
        store.template_names(),
        Err(ConfigError::Io { .. })
    ));
    Ok(())
}
```

The two file shapes refuse: `templates` present as a file hits the `Ok(!is_dir)`
arm, and a root that is itself a file makes `fs::metadata` fail `NotADirectory` —
both structural, both `PluginRootNotAnInstallation`. The last test pins the
genuine-fault `Io` arm with a self-referential `templates` symlink, whose
`fs::metadata` fails `FilesystemLoop` (neither `NotFound` nor `NotADirectory`) —
Unix-only, but with no permission fault and no root guard.

#### 5. CLI integration tests (the acceptance pins)

**File**: `cli/launcher/tests/config_read.rs`
**Changes**:

Add an assertion helper `assert_refuses_as_not_an_installation(&output, root)`
next to `assert_refuses_without_a_plugin_root`: non-zero exit, empty stdout,
stderr contains `ACCELERATOR_PLUGIN_ROOT`, contains the wrong root's path, and
contains "not an Accelerator installation". The wrong-root command tests below use
it, pinning the new message rather than only the presence of the variable name.

- **Replace** `a_root_without_a_templates_directory_still_renders_an_empty_table`
  (`:1373`) with its inverse (AC9 = inverse of AC1). Write the inverse test and
  observe it fail against the unmodified `template_names` **before** replacing the
  swallow, keeping the red step verifiable:

```rust
#[test]
fn a_root_without_a_templates_directory_refuses_to_list() -> TestResult {
    let fixture = Fixture::new()?.team("---\npaths:\n  work: x\n---\n")?;
    let bare = tempfile::Builder::new().prefix("config-read-").tempdir()?;
    let output = run_with_plugin_root(
        &fixture.root,
        bare.path().as_os_str(),
        &["config", "templates", "list"],
    )?;
    assert_refuses_as_not_an_installation(&output, bare.path());
    Ok(())
}
```

- **Add** `a_wrong_root_diagnostic_differs_from_the_absent_root_diagnostic`:
  capture `config templates list` stderr under a wrong root (via
  `run_with_plugin_root`) and under an absent root (via `run_in`), assert both
  refuse, and assert the two stderr strings differ and only the wrong-root one
  contains the bare root's path. Pins the distinction the dedicated variant buys.
- **Add** `templates_eject_all_against_a_wrong_root_refuses` (AC4): a wrong root,
  `config templates eject --all` refuses and creates no override directory.
- **Add** `a_wrong_root_with_a_templates_file_refuses` (AC8-revised): seed
  `templates` as a file, `config templates list` refuses via
  `assert_refuses_as_not_an_installation` — a structural shape now fails closed.
- **Add** `a_root_that_is_a_file_refuses`: point `ACCELERATOR_PLUGIN_ROOT` at a
  regular file, `config templates list` refuses naming the root.
  Environment-independent, no root guard.
- **Add** `a_genuine_io_fault_is_not_the_refusal` (AC8-revised, the `Io` arm):
  Unix-only, create a self-referential `templates` symlink so `fs::metadata` fails
  `FilesystemLoop`, run `config templates list`, assert non-zero exit and stderr
  contains `I/O error` but **not** `ACCELERATOR_PLUGIN_ROOT`. Pins that a genuine
  unreadable fault stays `Io` (degradable), distinct from the structural refusal.
- **Add** `a_root_independent_family_still_succeeds_against_a_wrong_root` (AC11):
  a wrong root, `config paths --fail-safe` exits 0 **and** prints non-empty
  stdout. The `--fail-safe` flag is the point: it forces the assertion to prove
  the family genuinely succeeded rather than degraded to exit-0 empty output.

### Success Criteria

#### Automated Verification

- [x] Error-variant unit tests pass: `cargo test -p config`
- [x] Store unit tests pass: `cargo test -p config-adapters`
- [x] Launcher integration tests pass: `cargo test -p accelerator --test config_read`
- [x] Format + lint clean: `mise run cli:check`

#### Manual Verification

- [x] `ACCELERATOR_PLUGIN_ROOT=$(mktemp -d) accelerator config templates list`
      exits non-zero and prints a diagnostic naming the root path, stating it is
      not an Accelerator installation, and naming `ACCELERATOR_PLUGIN_ROOT`.
- [x] The same against `config templates eject --all` refuses and writes nothing.

---

## Phase 2: Plugin-default resolution refuses on a wrong root (R2)

### Overview

Route `plugin_template_path` through the helper and have `resolve_template`'s
plugin-default tier build its candidate via `plugin_template_path` rather than
rejoining the path inline. `plugin_template_path` then becomes the single choke
for all four of `config template <name>`, `eject <name>`, `diff <name>`, and
`reset <name>` (the last two via `plugin_default`, which already routes through
it), eliminating the duplicated path-building while preserving the genuine
template-not-found for a valid installation missing one file.

### Changes Required

#### 1. `resolve_template` plugin-default tier and `plugin_template_path`

**File**: `cli/config-adapters/src/store.rs`
**Changes**: Route `plugin_template_path` (`:466`) through the helper:

```rust
fn plugin_template_path(&self, name: &str) -> Result<PathBuf, ConfigError> {
    Ok(self.require_templates_dir()?.join(format!("{name}.md")))
}
```

In `resolve_template`, replace the inline tier (`:400-401`) so it builds its
candidate via `plugin_template_path` — inheriting the root check ahead of the
`is_file()` fall-through and sharing the one path-building site:

```rust
let default = self.plugin_template_path(name)?;
if default.is_file() {
    return Ok(Some(self.resolved(
        TemplateSource::PluginDefault,
        &default,
        warning,
    )?));
}
Ok(None)
```

The eject handlers pre-compute `template_view::available_or_none(..)`, which
swallows `template_names()` via `unwrap_or_default()`; that tolerance stays safe
only because every command is dominated by a `?` on `plugin_default`,
`template_names`, or `plugin_template_path` that surfaces the refusal before the
swallowed value is observable. A future caller of `available()` not preceded by
such a `?` would re-mask a wrong-root refusal — the invariant to preserve.

#### 2. Store unit tests (red first)

**File**: `cli/config-adapters/src/store.rs` (test module; add `TemplateOverride`
to the test imports)
**Changes**: Cover, with a plugin root (binding each root `TempDir` to a local
first, per the module idiom):

- `resolve_template` under a wrong root, no override →
  `Err(PluginRootNotAnInstallation { .. })`.
- `resolve_template` under a wrong root **with** a user override for `<name>`,
  seeding `<store-root>/.accelerator/templates/<name>.md` first (so the
  `UserOverride` tier reads real content) → `Ok(Some(..))` with
  `TemplateSource::UserOverride`. This pins the hoisting crux — the root check sits
  *inside* the plugin-default tier, after the override tiers — at the fast store
  layer, not only at the CLI layer (AC10).
- `resolve_template` under a valid fixture root missing `<name>.md`, no override →
  `Ok(None)` (the genuine not-found preserved).
- `plugin_default` under a valid fixture root missing `<name>.md` → `Ok(None)`.
  This pins the genuine-not-found for the diff/reset site specifically, distinct
  from `resolve_template`'s tier — so a mutation conflating not-an-installation
  with a missing file at `plugin_default` cannot pass while AC5/AC7 stay green.
- `plugin_default` and `eject` under a wrong root →
  `Err(PluginRootNotAnInstallation { .. })`.

#### 3. CLI integration tests (the acceptance pins)

**File**: `cli/launcher/tests/config_read.rs`
**Changes**:

- **Add** `template_against_a_wrong_root_refuses` (AC2): wrong root, no override,
  `config template demo` refuses via `assert_refuses_as_not_an_installation`.
- **Add** `template_not_found_names_the_template_not_the_plugin_root` (AC7):
  `run_with_plugin` (valid fixture), `config template nonesuch`, assert non-zero,
  stderr contains `not found` and `nonesuch`, and does **not** contain
  `ACCELERATOR_PLUGIN_ROOT`.
- **Add** `eject_against_a_wrong_root_refuses` (AC3): wrong root,
  `config templates eject demo` refuses as not-an-installation (not `NoDefault`).
- **Add** `diff_and_reset_against_a_wrong_root_refuse` (AC5): wrong root,
  `config templates diff demo` and `reset demo` each refuse as not-an-installation.
- **Add** `a_user_override_resolves_against_a_wrong_root` (AC10): seed
  `.accelerator/templates/demo.md`, invoke via `run_with_plugin_root` with a bare
  root, assert exit 0 and the override's rendered content.

### Success Criteria

#### Automated Verification

- [x] Store unit tests pass: `cargo test -p config-adapters`
- [x] Launcher integration tests pass: `cargo test -p accelerator --test config_read`
- [x] Format + lint clean: `mise run cli:check`

#### Manual Verification

- [x] `ACCELERATOR_PLUGIN_ROOT=$(mktemp -d) accelerator config template plan`
      refuses naming the root path and `ACCELERATOR_PLUGIN_ROOT`, distinct from the
      absent-root message.
- [x] Against the real plugin root, `config template not-a-real-template` still
      reports a template-not-found naming the template, with no mention of the
      plugin root.

---

## Phase 3: Compose path surfaces the refusal (R3)

### Overview

Confirm the wrong-root refusal reaches the visualiser server boundary intact and
pin it with a compose contract test. No production change: the `?` at
`compose.rs:157` already propagates `PluginRootNotAnInstallation` into
`ComposeError`, and this test passes only because Phase 1 made `template_names`
refuse. Because the new message drops the launcher-only `bin/accelerator`
remediation, the server-side error no longer contradicts `run_serve`'s own prior
`ACCELERATOR_PLUGIN_ROOT`-is-set check.

### Changes Required

#### 1. Compose contract test

**File**: `cli/visualiser/server/tests/compose_contract.rs`
**Changes**: Mirror `an_empty_plugin_root_refuses_to_compose` (`:187`), but pass a
real temp directory without `templates/` as the plugin root:

```rust
#[test]
fn a_wrong_plugin_root_refuses_to_compose() {
    let tmp = tempfile::tempdir().unwrap();
    seed_project(tmp.path());
    let bare = tempfile::tempdir().unwrap();
    let error = load(Params {
        cwd: tmp.path().to_path_buf(),
        plugin_root: bare.path().to_path_buf(),
        owner_pid: 0,
        owner_start_time: None,
        host: "127.0.0.1".to_string(),
    })
    .expect_err("a wrong plugin root composed a config");
    let message = error.to_string();
    assert!(
        message.contains("ACCELERATOR_PLUGIN_ROOT"),
        "the error does not name the variable: {error}"
    );
    assert!(
        message.contains("not an Accelerator installation"),
        "the error does not name the wrong-root cause: {error}"
    );
    assert!(
        message.contains(&bare.path().display().to_string()),
        "the error does not name the offending root: {error}"
    );
}
```

### Success Criteria

#### Automated Verification

- [ ] Compose contract tests pass: `cargo test -p accelerator-visualiser --test compose_contract`
- [ ] Format + lint clean: `mise run cli:check`
- [ ] Full local CI mirror is green: `mise run` (AC12)

#### Manual Verification

- [ ] Building the server against a wrong root fails composition rather than
      serving an empty template set (the boundary test is the durable evidence;
      `run_serve` guards the env var before compose, so the running server does
      not reach this diagnostic on the absent-var path).

---

## Testing Strategy

### Unit Tests

- `ConfigError` in `error.rs`: the new variant's `Display` names the path and
  `ACCELERATOR_PLUGIN_ROOT` (and not "unknown"), and its `is_refusal()` is `true`.
- `FileConfigStore` with a plugin root — the new pattern for this crate — covering
  `require_templates_dir`'s outcomes: present dir → path; the three structural
  refusals (no `templates/`; `templates` a file; a root that is a file →
  `NotADirectory`) → `PluginRootNotAnInstallation`; and a genuine unreadable fault
  (a self-referential `templates` symlink → `FilesystemLoop`) → `Io`. Exercised
  through `template_names`, `resolve_template`, `plugin_default`, and `eject`. The
  structural triggers are environment-independent; the `Io` trigger is Unix-only
  but needs no permission fault and no root guard.
- The genuine-not-found path (`resolve_template` → `Ok(None)`) under a valid
  fixture root missing one template, and the hoisting crux (wrong root + override →
  `Ok(Some(UserOverride))`) at the fast store layer.

### Integration Tests

- CLI black-box runs in `config_read.rs` for each of the six commands on a wrong
  root, plus the distinguishing cases (genuine not-found naming the template;
  resolving override at exit 0; wrong-root message differs from absent-root and
  names the path), the structural refusals (`templates` a file; the root a file),
  the genuine-`Io` fault (self-referential symlink), and the
  root-independent-family guard under `--fail-safe`.
- The compose contract boundary test for the server consumer.

### Manual Testing Steps

1. `root=$(mktemp -d)` — a directory with no `templates/`.
2. `ACCELERATOR_PLUGIN_ROOT=$root accelerator config templates list` — refuses,
   names the root path and `ACCELERATOR_PLUGIN_ROOT`, exit non-zero.
3. Repeat for `template <name>`, `templates eject <name>`, `eject --all`,
   `diff <name>`, `reset <name>` — each refuses naming the root path.
4. `ACCELERATOR_PLUGIN_ROOT=$root accelerator config paths --fail-safe` — exits 0
   with normal non-empty output.
5. Against the committed valid root, `config template <missing>` — reports a
   template-not-found naming the template, not the plugin root.

## Performance Considerations

Negligible. The added cost is one `stat` per plugin-default lookup and one `stat`
before `template_names`' existing `read_dir`, on cold, explicitly-invoked config
paths (13 templates at most). No hot path is affected.

## Migration Notes

None. No data or on-disk layout changes. The behavioural change is a wrong root
moving from a silent empty/not-found answer to a hard refusal — the intended
fix — and it ships in the shared `config-adapters` library, so both the launcher
and the visualiser server pick it up at the same plugin version, kept in lockstep
by 0182's version-keyed cache.

## References

- Original work item: `meta/work/0184-template-enumeration-swallows-a-wrong-plugin-root.md`
- Related research: `meta/research/codebase/2026-09-08-0184-template-resolution-wrong-plugin-root.md`
- Builds on: `meta/work/0182-cli-derives-plugin-root-from-own-location.md`,
  `meta/plans/2026-07-27-0182-bootstrap-self-location-and-plugin-root-rename.md` (Phase 5)
- Store sites: `cli/config-adapters/src/store.rs:73,400,412,436,466,483,729`
- Error taxonomy: `cli/config/src/error.rs` — variant list `:38-67`, `is_refusal`
  `:76-88` (exhaustive), `Display` `:90-139`, `From<ConfigError>` `:143-147`, tests
  `:149-283`. The new `PluginRootNotAnInstallation { path }` variant slots into
  each.
- Compose consumer: `cli/visualiser/server/src/compose.rs:20,157`; server env-var
  guard `cli/visualiser/server/src/main.rs:68`
- Test scaffolding: `cli/launcher/tests/config_read.rs:1219-1401`,
  `cli/visualiser/server/tests/compose_contract.rs:187`
