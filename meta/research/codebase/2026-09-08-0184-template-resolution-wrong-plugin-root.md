---
type: "codebase-research"
id: "2026-09-08-0184-template-resolution-wrong-plugin-root"
title: "Research: Template resolution succeeds silently on a plugin root that is not an installation"
date: "2026-09-08T22:44:28+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0184"
parent: "work-item:0184"
topic: "Template resolution succeeds silently on a plugin root that is not an installation"
tags: ["research", "codebase", "config", "templates", "plugin-root", "cli", "visualiser"]
revision: "8da94dbf0ba9aa3ddaffb5823633a16cdec2189d"
repository: "accelerator"
last_updated: "2026-09-08T22:49:34+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Added follow-up research settling the templates.rs scope gap (no gap: it is a downstream renderer of the composed map, not an independent store consumer; R3 correctly scoped to compose.rs)."
schema_version: 1
---

# Research: Template resolution succeeds silently on a plugin root that is not an installation

**Date**: 2026-09-08 22:44 UTC
**Author**: Toby Clemson
**Git Commit**: 8da94dbf0ba9aa3ddaffb5823633a16cdec2189d
**Branch**: anonymous jj change `vnqrqyrt` (no bookmark, atop `main`)
**Repository**: accelerator

## Research Question

Ground work item 0184 against live code: verify that the three plugin-root
template accessors in `FileConfigStore` still succeed silently on a *wrong* root
(present directory, no `templates/`), map every affected command and consumer to
its store site, confirm the fix's precondition, and surface the test scaffolding
a fix would build on.

## Summary

Every behavioural claim in 0184 holds against the current tree. The wrong-root
case is real at all three accessors, the `require_plugin_root` gate fires only on
an *absent* root, and the `ErrorKind::NotFound` split the fix must mirror already
exists in the sibling accessors. The packaging precondition — a real installation
always ships `templates/` — is **supported** inside this repo.

Three findings sharpen or extend the item:

- **Exit code is 1, not 2, for all six commands.** `PluginRootUnavailable`
  converts to `kernel::Error::Failed` → `ExitCode::FAILURE` (exit 1). The ACs say
  "non-zero", which exit 1 satisfies; exit 2 is reserved for the write commands'
  own domain refusals (`EjectOutcome::Exists`, no-override diff/reset).
- ✅ **No second visualiser consumer — R3 is correctly scoped.** Traced in the
  follow-up below: `cli/visualiser/server/src/templates.rs` is a downstream
  renderer of the already-composed `Config.templates` map, not a store consumer.
  Its `plugin_root` is display-only; the single server enforcement point is
  `store.template_names()?` at `compose.rs:157`, which R3/AC6 already cover.
- ⚠️ **AC6's wrong-root compose test only works after R1 lands.** Today a
  non-empty wrong root composes *successfully* with an empty template map; it
  never errors. The existing `an_empty_plugin_root_refuses_to_compose` test
  exercises the *absent* root via `PathBuf::new()`. A present-but-no-`templates/`
  root reaches the refusal only once `template_names` maps `NotFound` →
  `PluginRootUnavailable`, which the `?` at `compose.rs:157` then propagates with
  no production change.

## Detailed Findings

### The three accessors and their current wrong-root behaviour (`cli/config-adapters/src/store.rs`)

`require_plugin_root` (`store.rs:73-77`) is the sole gate raising
`PluginRootUnavailable`, and it fires only when `self.plugin_root` is `None`. A
present-but-wrong root is `Some(path)`; `as_deref()` yields `Some(&path)` and it
returns `Ok(&path)` with no filesystem validation. `with_plugin_root`
(`store.rs:64-69`) normalises an empty-string root to `None` via
`.filter(|root| !root.as_os_str().is_empty())`, so `Some("")` collapses to the
absent case.

| Accessor | Site | Wrong-root behaviour today | Fix target |
|----------|------|----------------------------|------------|
| `template_names` | `store.rs:412` | `let Ok(entries) = read_dir(...) else { Ok(Vec::new()) }` swallows every error | R1 |
| `resolve_template` plugin-default tier | `store.rs:400-408` | `<name>.md` not a file → `Ok(None)` (names no template) | R2 |
| `plugin_template_path` | `store.rs:466-471` | tests nothing; only builds the path | R2 |
| `plugin_default` | `store.rs:432-445` | `is_file()` at `:437` → `Ok(None)` at `:438` | R2 |
| `eject` | `store.rs:475-532` | `is_file()` at `:484` → `EjectOutcome::NoDefault` | R2 |

The `template_names` swallow is exactly as the item quotes, at `store.rs:414-416`:

```rust
let Ok(entries) = fs::read_dir(plugin.join("templates")) else {
    return Ok(Vec::new());
};
```

It calls `require_plugin_root()?` first (`store.rs:413`), so an absent root
short-circuits; once the root is present, a missing `templates/` (`NotFound`), a
permission error, and a not-a-directory error all collapse identically to
`Ok(Vec::new())`. The plugin-default tier of `resolve_template` builds its path
**inline** (`store.rs:401`), not through `plugin_template_path`; `plugin_default`
routes through `plugin_template_path` (`store.rs:436`). Both fall to `Ok(None)`
via a `path.is_file()` check.

Two distinct sites, near-identical names, as the item warns: `resolve_template`'s
plugin-default tier (reached by `config template <name>`) and the `plugin_default`
method (reached only by `diff`/`reset`).

### The error machinery and the pattern to mirror (`cli/config/src/error.rs`)

`ConfigError` is hand-written, not `thiserror` — a manual `Display`
(`error.rs:90-139`) and a bare `impl std::error::Error`. `PluginRootUnavailable`
is a unit variant (`error.rs:66`) whose message already names the variable and
the bootstrap (`error.rs:131-136`):

```text
the plugin installation root is unknown: set ACCELERATOR_PLUGIN_ROOT, or invoke
accelerator through bin/accelerator, which derives it
```

It is classified as a refusal (`error.rs:78`), so `--fail-safe` cannot absorb it.
`ConfigError::Io { path, detail }` (`error.rs:52-55`, message `error.rs:112-114`)
is **not** a refusal, so it can be degraded. The `Io` half of the fix is built via
the `io_error` helper (`store.rs:729-734`).

The three-way `ErrorKind::NotFound` split the R1 `Io` half must mirror already
exists in `custom_lenses` (`store.rs:296-302`) and `skill_names`
(`store.rs:322-328`), and in `read`/`write`/`ensure_line`/`ensure_inner_gitignore`:

```rust
let entries = match fs::read_dir(&dir) {
    Ok(entries) => entries,
    Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
    Err(error) => return Err(io_error(&dir, &error)),
};
```

⚠️ **The R1 divergence.** The siblings map `NotFound` → empty list; R1 maps it →
`PluginRootUnavailable`. That is a deliberate divergence resting on the
Assumption, not parity — only the `Io` (non-`NotFound`) half matches the siblings.

### CLI command surface, exit codes, and `--fail-safe`

The command-to-site mapping in the item is verified exactly. Handlers live in
`cli/launcher/src/config_command/inbound/cli.rs`; a two-layer dispatch maps clap
`ConfigAction` → the hexagon's `Action` (`launch/mod.rs:33-163`) before reaching
them.

| Command | Handler | Store site | Confirmed |
|---------|---------|------------|-----------|
| `config templates list` | `cli.rs:260` | `template_names` (`store.rs:412`) | yes |
| `config template <name>` | `cli.rs:238` | `resolve_template` tier (`store.rs:400`) — **not** `plugin_default` | yes |
| `config templates eject <name>` | `cli.rs:507` | `eject` → `plugin_template_path` | yes |
| `config templates eject --all` | `cli.rs:533,542` | `eject_all` → `template_names` | yes |
| `config templates diff <name>` | `cli.rs:579` | `plugin_default` → `plugin_template_path` | yes |
| `config templates reset <name>` | `cli.rs:602` | `plugin_default` → `plugin_template_path` | yes |
| `config paths` | `cli.rs:311` | none — never reads the plugin root | yes |

**Exit-code precision.** Every `ConfigError` becomes
`kernel::Error::Failed(msg)` via `From<ConfigError>` (`error.rs:143-147`), never
`kernel::Error::Refusal`. `main::report` (`main.rs:417-426`) maps `Refusal` →
exit 2 and everything else → exit 1. So a `PluginRootUnavailable` from any of the
six commands prints to stderr and exits **1**. The `assert_refuses_without_a_plugin_root`
helper (`config_read.rs:1278-1288`) asserts non-zero + empty stdout + stderr names
the variable — satisfied by exit 1.

**`--fail-safe` (relevant to AC11).** The flag maps to `OnFailure::Degrade`
(`launch/mod.rs:23-29`). In `finish` (`cli.rs:453-472`) the degrade arm matches
**only** `Failure::Read`; a `Failure::Refusal` (which `PluginRootUnavailable`
becomes, `cli.rs:422-430`) is never degraded. So under `--fail-safe` a genuinely
failing degradable read *also* exits 0 — with empty stdout (scalar) or a
`## … Unavailable` notice (block). That is why AC11 must assert exit 0 **and**
non-empty real output; `the_root_independent_families_still_succeed_with_no_plugin_root`
(`config_read.rs:1391-1401`) already does exactly this. `eject`/`diff`/`reset`
carry no `--fail-safe` flag at all.

### Visualiser compose path (`cli/visualiser/server/src/compose.rs`)

`compose::load` calls `resolve_templates` (`compose.rs:55-61`), which loops
`store.template_names()?` at `compose.rs:157`; the store there carries the plugin
root (`compose.rs:42-44`). `ComposeError::Config(#[from] ConfigError)`
(`compose.rs:19-25`) stores the whole `ConfigError` value and re-emits its message
through `#[error("configuration error: {0}")]`, so a wrapped
`PluginRootUnavailable` renders with `ACCELERATOR_PLUGIN_ROOT` **intact** — no
flattening. R3's claim that the `?` propagation needs no production change is
correct.

The only production caller is `run_serve` (`main.rs:80-92`), which exits 2 on a
compose error. It guards the env var *before* compose (`main.rs:68-71`), so the
absent-var path prints a different message and never reaches `template_names`;
the compose diagnostic is reachable through `main` only via the empty-string edge.
AC6's clean boundary is therefore calling `compose::load(Params { plugin_root, .. })`
directly, as the existing contract test does.

### Second server template site, traced (`cli/visualiser/server/src/templates.rs`)

✅ Not an independent consumer — see the follow-up section for the full trace.
`TemplateResolver::build` (`templates.rs:117-122`) takes the already-composed
`Config.templates: HashMap<String, TemplateTiers>` and renders it; it never calls
the store's plugin-root accessors. Its `plugin_root` argument feeds only
`display_path` (`templates.rs:90-104`) for `<plugin-root>`-prefix stripping. Per-tier
file reads go through `load_via_driver` (`templates.rs:254-268`), which swallows a
read failure into `present: false` — a correct rendering of a genuinely-absent
tier file, not a root-validity signal. R3 stays scoped to `compose.rs`.

### Test scaffolding

All plugin-root behaviour is tested at the **CLI integration layer** in
`cli/launcher/tests/config_read.rs` (black-box `Command` runs of the compiled
binary via `env!("CARGO_BIN_EXE_accelerator")`), plus two unit tests on the error
variant in `error.rs:248-268`. There are **no** `config-adapters` unit tests that
build a `FileConfigStore` with a plugin root — every store unit test uses bare
`FileConfigStore::at(dir)` with `plugin_root == None`.

- **Characterisation test to replace (AC9)**:
  `a_root_without_a_templates_directory_still_renders_an_empty_table`
  (`config_read.rs:1373-1389`) — builds a `bare` temp dir with no `templates/`,
  runs `templates list` via `run_with_plugin_root`, asserts the byte-exact header
  table and exit 0. Its doc comment says "tracked as its own work item"; the
  0182 plan is what names that item as **0184**.
- **Property to preserve (R4)**:
  `a_user_override_still_resolves_with_no_plugin_root` (`config_read.rs:1356-1368`)
  — writes `.accelerator/templates/demo.md`, runs `config template demo` via
  `run_in` (which strips `ACCELERATOR_PLUGIN_ROOT`), asserts the override resolves
  at exit 0. Pins the plugin-default check at the *third* tier.
- **The 0182 absent-root suite** (`config_read.rs:1244-1401`): the reusable
  helpers `run_with_plugin_root` (`:1226`, sets the var to any `&OsStr` — the
  invoker for a wrong root), `run_with_plugin` (`:1219`, the committed valid
  fixture), `run_in` (`:71`, strips the var), `assert_names_the_plugin_root`
  (`:1270`), and `assert_refuses_without_a_plugin_root` (`:1278`).
- **Valid-installation fixture (AC7, AC10)**: committed at
  `cli/launcher/tests/fixtures/plugin/templates/{demo,other}.md`, pointed at by
  `run_with_plugin`. AC7 (present `templates/`, absent `<name>.md`) is already
  the shape of `template_not_found_fails_closed_even_with_fail_safe`
  (`config_read.rs:1446`). AC10 (wrong root + override) is a *new* composition of
  the override setup with `run_with_plugin_root`.
- **IO-fault triggers (AC8)**: the established idiom is `fs::create_dir(x)` where
  a file is expected (`config_read.rs:648-703, 1639-1725`); AC8's inverse —
  `fs::write(plugin.join("templates"), ...)` so `read_dir` fails with
  `NotADirectory` (not `NotFound`, so it maps to `Io`) — is the mirror. The
  `chmod` variant uses `PermissionsExt::from_mode` (`config_read.rs:1853`).
- **Compose boundary (AC6)**: `an_empty_plugin_root_refuses_to_compose`
  (`compose_contract.rs:187-203`) is the shape — `load(Params { plugin_root, .. })
  .expect_err(..)` asserting the message contains `ACCELERATOR_PLUGIN_ROOT`. AC6
  differs by passing a real temp dir without `templates/` instead of
  `PathBuf::new()`, and depends on R1 landing.

### Packaging precondition — supported

The plugin ships as a **whole git checkout at a tag** (`marketplace.json:13-17`),
not a curated package. The release tasks cross-compile and upload only the Rust
binaries (`tasks/release.py:186-233`, `tasks/manifest.py:115-178`,
`tasks/build.py`); nothing assembles a plugin-content package or selects/omits
files. There is no `files` array in `plugin.json`, no `.claude-pluginignore`, and
`.gitignore` does not exclude `templates/`. The 13 tracked `templates/*.md` ride
along automatically. **Verdict: the in-repo assumption holds** — a real
installation always ships `templates/`, so its absence is a reliable
not-an-installation signal.

## Code References

- `cli/config-adapters/src/store.rs:73-77` — `require_plugin_root`, the absent-root-only gate
- `cli/config-adapters/src/store.rs:412-430` — `template_names`, the `read_dir` swallow (R1)
- `cli/config-adapters/src/store.rs:400-408` — `resolve_template` plugin-default tier (R2)
- `cli/config-adapters/src/store.rs:432-471` — `plugin_default` + `plugin_template_path` (R2)
- `cli/config-adapters/src/store.rs:475-532` — `eject` → `EjectOutcome::NoDefault`
- `cli/config-adapters/src/store.rs:296-302, 322-328, 729-734` — the `NotFound`/`Io` split + `io_error` to mirror
- `cli/config/src/error.rs:66,78,131-136` — `PluginRootUnavailable` variant, refusal classification, message
- `cli/config/src/error.rs:143-147` — `From<ConfigError> for kernel::Error` (→ `Failed`, exit 1)
- `cli/launcher/src/config_command/inbound/cli.rs:238,260,507,533,542,579,602` — the six command handlers
- `cli/launcher/src/config_command/inbound/cli.rs:453-472` — `finish`, the `--fail-safe` degrade boundary
- `cli/launcher/src/main.rs:417-426` — exit-code mapping (Refusal→2, else→1)
- `cli/visualiser/server/src/compose.rs:19-25,55-61,157` — `ComposeError`, `resolve_templates`, the `?` forward
- `cli/visualiser/server/src/templates.rs:90-104,117-122,254-268` — display-only `plugin_root`, driver-swallowing renderer (not a store consumer)
- `cli/visualiser/server/src/server.rs:90-98,140` + `watcher.rs:372-419` + `api/templates.rs:20-31` — how the composed map reaches `TemplateResolver` and the API
- `cli/launcher/tests/config_read.rs:1219-1401` — plugin-root helpers, the characterisation test, the absent-root suite
- `cli/visualiser/server/tests/compose_contract.rs:187-203` — `an_empty_plugin_root_refuses_to_compose`
- `cli/launcher/tests/fixtures/plugin/templates/{demo,other}.md` — committed valid-installation fixture
- `templates/` — 13 tracked plugin-default templates (the shipped source)

## Architecture Insights

- **A single shared installation-detection helper is the natural fix shape.** The
  item's decision is that detection lives in one "is this root an installation?"
  helper validating `templates/` presence, called by all three accessors. It must
  sit *inside* each plugin-default step, not hoisted: for `resolve_template` and
  `plugin_default` the check runs only after the override tiers miss, or a wrong
  root would pre-empt a resolving override (the R4 property). This bundling is why
  the three accessors stay in one item.
- **The genuine-not-found vs root-refusal distinction (AC2/AC5 vs AC7) is the
  crux.** After R2, a plugin-default site must ask "does `templates/` exist?"
  (absent → `PluginRootUnavailable`) *before* "is `<name>.md` a file?" (absent →
  `Ok(None)`), so a valid installation missing one template still reports a
  template-not-found naming `<name>`, not the plugin root.
- **Refusal classification is load-bearing.** `PluginRootUnavailable.is_refusal()`
  keeps behaviour byte-identical with and without `--fail-safe` and forces
  fail-closed — the property 0182 established and 0184 extends to the wrong root.
- **The store is shared across two binaries.** `config-adapters` ships in both the
  launcher and the visualiser server; the fix changes both at once, and the
  version-keyed plugin cache keeps them in lockstep (0182 inheritance).

## Historical Context

- `meta/work/0182-cli-derives-plugin-root-from-own-location.md` — introduced
  `PluginRootUnavailable` and `require_plugin_root` for the *absent* root, and
  Phase 5 §3 explicitly deferred the wrong-root case, raising it as 0184.
- `meta/plans/2026-07-27-0182-bootstrap-self-location-and-plugin-root-rename.md`
  — Phase 5 §2 names `compose.rs:157` forwarding `template_names`' `ConfigError`
  via `?` (the exact link R3 cites); §3 names the characterisation test as the
  deferred residue. Its Implementation notes correct the plan body:
  `plugin_template_path` shipped as `Result<PathBuf, ConfigError>` outright, and
  `known_skill_names` was deliberately left tolerant (do not touch it here).
- `meta/research/issues/2026-07-26-cli-requires-claude-plugin-root-env-var.md` —
  the RCA on the plugin-root env-var contract underpinning the gate.
- `meta/reviews/work/0184-...-review-1.md` — the review of this exact bug.

## Related Research

- `meta/research/codebase/2026-07-27-0182-plugin-root-self-location-implementation-surface.md`
  — where the wrong root originates (bootstrap self-location).
- `meta/research/codebase/2026-07-07-0178-config-crates-native-yaml-reader.md`
  — the `config-adapters` / `FileConfigStore` implementation study.
- `meta/research/codebase/2026-06-11-0096-templates-view-auto-discovery.md`
  — template enumeration internals.
- `meta/research/codebase/2026-03-29-template-management-subcommands.md`
  — `resolve_template` / eject / diff / reset foundations.

## Open Questions

- ✅ **Resolved — is `templates.rs` a second wrong-root consumer?** No. Traced in
  the follow-up section below: it is a downstream renderer of the composed
  `Config.templates` map, its `plugin_root` is display-only, and the single server
  enforcement point is `compose.rs:157`. R3 is correctly scoped.
- ❓ **Does Claude Code's own plugin-install step ever filter files?** The in-repo
  precondition is supported, but the external installer is not asserted anywhere
  in this workspace. The item's Dependencies precondition ("confirm against the
  release artifact's file list") is satisfied for the git-tag model; the residual
  risk sits outside this repo.

## Follow-up Research 2026-09-08 22:49 UTC — the `templates.rs` scope gap, settled

**Verdict: no scope gap. R3/AC6 are correctly scoped to `compose.rs`.**
`cli/visualiser/server/src/templates.rs` (`TemplateResolver`) is a downstream
renderer of the already-composed `Config.templates` map, not an independent
consumer of the store's plugin-root accessors. It needs no requirement of its own.

The data flow, startup and reload:

```text
compose::load (main.rs:80, startup)
  └─ resolve_templates (compose.rs:55-61)
       └─ store.template_names()?   ← compose.rs:157   [single server enforcement point — R3/AC6]
       → Config.templates: HashMap<name, TemplateTiers{ plugin_default: <abs path>, … }>
                                   │
      ┌────────────────────────────┴────────────────────────────┐
 server.rs:90 TemplateResolver::build(&cfg.templates, …)   watcher.rs:412 rebuild from cloned map
                                   │                        (cfg_templates, server.rs:140)
                         api/templates.rs:20 → /api/templates[/:name]
```

Three facts settle it:

- **`plugin_root` is display-only.** `TemplateResolver::build`
  (`templates.rs:117-122`) passes `plugin_root` solely to `display_path`
  (`templates.rs:90-104`), which strips it as a `<plugin-root>` UI prefix. It never
  reads or validates the filesystem with it.
- **Reads are keyed on pre-computed paths and swallow per-tier.** Each tier's
  `plugin_default` is an absolute `PathBuf` resolved upstream by compose;
  `load_via_driver` (`templates.rs:254-268`) reads it and maps `Err(_)` →
  `(present: false, None, None)`. That swallow is the correct rendering of a
  genuinely-absent tier file — the UI analogue of AC7 — and should stay.
- **The watcher does not re-enumerate.** `TemplateChangeHandler::spawn`
  (`watcher.rs:372-419`) captures the cloned `cfg_templates` map
  (`Arc<HashMap<String, TemplateTiers>>`, from `server.rs:140`) and rebuilds
  `TemplateResolver` from that fixed map on every FS change. It never calls
  `store.template_names`.

Consequence: once R1 makes `template_names` refuse on a wrong root,
`compose::load` fails at startup (`main.rs:80` → exit 2) and `TemplateResolver` is
never reached with a wrong-root config. There is no independent server path that
re-enumerates the store and swallows, so R3's single-point coverage at
`compose.rs:157` is complete.
