---
type: "codebase-research"
id: "2026-09-05-0258-help-show-subcommands"
title: "Research: Help Should Show Subcommands (0258)"
date: "2026-09-05T12:45:31+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0258"
parent: "work-item:0258"
topic: "Launcher help divergence: help, bare, and --help entry points"
tags: ["research", "codebase", "launcher", "cli", "help", "dispatch", "manifest"]
revision: "4a18db85a476d4250e294bf895458344ef90bfaf"
repository: "accelerator"
last_updated: "2026-09-05T12:45:31+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Help Should Show Subcommands (0258)

**Date**: 2026-09-05T12:45:31+00:00
**Author**: Toby Clemson
**Git Commit**: 4a18db85a476d4250e294bf895458344ef90bfaf
**Branch**: HEAD (detached)
**Repository**: accelerator

## Research Question

For the bug at `meta/work/0258-help-show-subcommands.md`: why do `accelerator
help`, bare `accelerator`, and `accelerator --help` diverge in the command list
they print, where does the divergence originate, what drives the sub-binary
listing and its descriptions, and what must change — across launcher source,
manifest data, crate metadata, and tests — to converge all three on one merged,
user-facing listing.

## Summary

The divergence is entirely in `cli/launcher/src/main.rs`, at two gates that feed
a single augmentation path. Only `accelerator --help` reaches
`render_augmented_help()`, which appends the manifest-driven "External
subcommands:" block via clap `after_help`; `help` and bare invocation take
routes that print clap's built-in-only help. Three distinct clap error kinds —
`DisplayHelp` (both `--help` and `help`), and `MissingSubcommand` (bare) — are
routed differently by `handle_parse_error()`, and an arg-word guard in
`is_root_help()` further excludes the `help` subcommand.

The fix has three moving parts, in order of surface area. First, **collapse the
two lists into one** by abandoning `after_help` and composing the full command
list from clap built-ins plus manifest sub-binaries, then route all three entry
points to it. Second, **rewrite seven of the nine crate `Cargo.toml`
descriptions** — the source of truth that flows through the build system into
the signed manifest — noting `visualiser` and `migrate` already carry clean
user-facing text. Third, **extend tests**, which is where the work item
under-scopes: two Python test mirrors pin the descriptions verbatim, and no
black-box harness can assert a populated listing because the manifest is
signature-verified.

Three corrections to the work item surfaced. The bare-invocation error kind is
**`MissingSubcommand`**, not `DisplayHelpOnMissingArgumentOrSubcommand` as the
Technical Notes state. `help` is **clap's synthesised subcommand**, not a
declared built-in — the `Command` enum has only `Version`, `Config`, `Cache`,
`External`. And the "Tests to extend" list omits the two Python mirrors that
will break on a description rewrite.

## Detailed Findings

### The three-entry-point divergence (`cli/launcher/src/main.rs`)

All help output is produced in the error-handling path, because clap surfaces
help, version, and missing-subcommand as `Err(clap::Error)`. `main()` calls
`Cli::try_parse()` and delegates any `Err` to `handle_parse_error(&error)`
(`main.rs:370,376`). Each invocation raises a different clap `ErrorKind`, and
routing plus the `is_root_help()` guard send only `--help` through augmentation.

```text
accelerator --help   → Err(DisplayHelp)        → is_root_help TRUE  → render_augmented_help()  [built-ins + External block, stdout, exit 0]
accelerator help     → Err(DisplayHelp)        → is_root_help FALSE → print!("{error}")        [built-ins only, stdout, exit 0]
accelerator (bare)   → Err(MissingSubcommand)  → is_root_help FALSE → error.print()            [usage error, built-ins in brackets, stderr, exit 1]
```

`is_root_help()` (`main.rs:106-117`) returns `true` only when the kind is
`DisplayHelp` **and** `args_os().nth(1)` is not one of `"version" | "config" |
"cache" | "help"`. For `--help`, arg 1 is `--help` (not in the set) → true. For
`help`, clap's implicit help subcommand raises `DisplayHelp` but arg 1 is
`"help"` (in the set) → false. This word-list is the guard that makes `help`
diverge from `--help`.

`handle_parse_error()` (`main.rs:123-143`) has three branches: `DisplayHelp if
is_root_help` → `render_augmented_help()` (augmented, exit 0); `DisplayHelp |
DisplayVersion | DisplayHelpOnMissingArgumentOrSubcommand` → `print!` to stdout,
exit 0 (plain); `_` → `error.print()` to stderr, `ExitCode::from(1)`. Bare
invocation's `MissingSubcommand` lands in the `_` arm.

`render_augmented_help()` (`main.rs:93-101`) rebuilds the clap command via
`Cli::command()`, and if `help_section()` yields a block, attaches it with
`command.after_help(section)` before `print_help()`. `help_section()`
(`main.rs:82-91`) loads and signature-verifies the release manifest and calls
`external_subcommands_section()`; any failure returns `None`, so built-in help
still prints (fail-open).

### Bare-invocation exit status — correction to the work item

Bare `accelerator` produces **`ErrorKind::MissingSubcommand`**, not
`DisplayHelpOnMissingArgumentOrSubcommand`. The top-level `Cli`
(`cli/launcher/src/launch/inbound/cli.rs:9-14`) has a required subcommand and no
`arg_required_else_help`, so a missing subcommand is a usage error. It re-maps
clap's native exit 2 to **exit 1** in the `_` arm (`main.rs:138-141`), because
exit 2 is reserved for subcommand refusals (`main.rs:119-122`, `report()` at
`342-351`).

⚠️ The work item Technical Notes (`0258:141-144`) attribute bare invocation to
`DisplayHelpOnMissingArgumentOrSubcommand`. That kind exits **0** via branch 2
and would contradict AC-3's required non-zero exit. That kind actually applies
to bare `config`/`cache`, which set `arg_required_else_help = true`
(`cli.rs:21,27`; confirmed by `config_read.rs:493` `a_bare_config_prints_help_
and_exits_zero`). AC-3 is satisfiable as written — the mechanism is
`MissingSubcommand`, and the fix must preserve the `_`-arm exit-1 mapping while
adding the full listing to that path.

### The external-subcommands block (`cli/launcher/src/launch/help.rs`)

`external_subcommands_section()` (`help.rs:14-31`) builds the block. It returns
`None` when `manifest.binaries` is empty (`help.rs:15-17`). The heading text
`"External subcommands:"` is a hard-coded literal at **`help.rs:24`**. It
iterates `&manifest.binaries` — a `BTreeMap<String, BinaryEntry>`, so ordering
is lexicographic by name — and writes `\n  {name:<width$}  {description}` per
entry, padding to the widest sanitised name. `sanitize()` (`help.rs:36-38`)
strips C0/C1 control chars from these signature-verified but terminal-rendered
strings. This block is attached solely through `after_help`, so it appears only
on the augmented path.

Merging into one list means abandoning `after_help` and composing the full
command list, because built-ins live in the clap `Command` tree and sub-binaries
live in the manifest — two disjoint sources clap cannot unify on its own.

### The two lists to merge

The built-ins are the clap `Command` enum (`cli/launcher/src/launch/inbound/
cli.rs:16-35`), with exactly three explicit variants plus the catch-all:
`Version`, `Config`, `Cache`, `External(Vec<OsString>)`. Each carries a `///`
doc comment that clap turns into its `about` text. The only `#[command(...)]`
attributes are behavioural (`arg_required_else_help`, `external_subcommand`).

❌ There is **no `Help` variant**. `accelerator help` is clap's auto-generated
help subcommand, recognised at runtime as a token in `is_root_help()` but never
declared. The work item Summary and AC treat `help` as one of "four built-in
commands"; precisely, there are three declared built-ins plus clap's synthesised
`help`, plus the `External` catch-all.

The sub-binaries are unknown at compile time (fetched on demand), swallowed by
`external_subcommand` (`cli.rs:33-34`); their names and descriptions exist only
in the signed manifest. The merge currently happens at `main.rs:93-101` and is
`--help`-only.

### Description source of truth — build system, not launcher

The description flow is a chain from crate metadata to rendered help:

```text
crate Cargo.toml package.description
  → tasks/manifest.py _read_description()      [packaging time]
  → BinaryEntry.description (Python)
  → signed manifest.json binaries.<token>.description
  → (runtime) launcher fetch + verify
  → Manifest::parse_and_validate → BinaryEntry.description (Rust)
  → external_subcommands_section() → help output
```

The token list is a hardcoded tuple `DISPATCHED_SUBBINARIES` in
`tasks/shared/paths.py:29-39`. `tasks/manifest.py` `_read_description()`
(`106-112`) reads `package.description` from each crate's `Cargo.toml` and
**raises `ManifestError` if empty** — description is required. `_SUBBINARY_
MANIFESTS` (`manifest.py:87-99`) maps `visualiser` → `cli/visualiser/server/
Cargo.toml`, most tokens → `cli/<name>-cli/Cargo.toml`. The Rust `BinaryEntry`
(`manifest.rs:36-42`) is only a signed **read contract** — it deserialises the
manifest, it does not build it. There is **no `build.rs`** in the workspace and
no code embeds `CARGO_PKG_DESCRIPTION`; `tasks/manifest.py` is the only bridge.

This confirms AC "driven by the same source of truth as dispatch": a
newly-registered sub-binary appears automatically because both dispatch and the
listing read the manifest.

### Current crate descriptions and the rewrite scope

Seven of nine crates carry launcher-internal phrasing; **`visualiser` and
`migrate` already carry clean user-facing text**. The rewrite is narrower than
the work item's nine-item list implies.

| Token | `Cargo.toml` current description | Intended (0258) | Change |
|---|---|---|---|
| `work` | `The work create\|show\|resolve\|diff\|update sub-binary.` | Manage work items | 🔴 rewrite |
| `vcs` | `The vcs detect\|status\|log\|guard sub-binary.` | Detect and inspect version-control state | 🔴 rewrite |
| `jira` | `The jira create\|update\|... sub-binary.` | Manage Jira issues | 🔴 rewrite |
| `linear` | `The linear create\|update\|... sub-binary.` | Manage Linear issues | 🔴 rewrite |
| `corpus` | `The corpus adr\|metadata\|linkage\|frontmatter sub-binary.` | Manage the meta-document corpus (ADRs, metadata, frontmatter) | 🔴 rewrite |
| `collaboration` | `The collaboration pr base-repo\|update-body sub-binary.` | Manage pull-request collaboration | 🔴 rewrite |
| `design` | `The design validate-source\|... sub-binary.` | Validate and process design sources | 🔴 rewrite |
| `migrate` | `Apply pending meta-directory schema migrations.` | Apply pending meta-directory schema migrations | 🟢 already matches |
| `visualiser` | `Launch the interactive meta-directory visualiser` | Serve the meta-directory visualiser | 🟡 near-match, verb differs |

Crate paths: `cli/<token>-cli/Cargo.toml` for all except `visualiser`
(`cli/visualiser/server/Cargo.toml`). The domain crates `cli/vcs/`, `cli/work/`,
etc. do not carry these descriptions.

### Description dual-use and non-consumers

Rewriting the descriptions affects exactly two functional surfaces — packaging
(`tasks/manifest.py`) and launcher help (`help.rs:27-28`) — which is the
intended effect. Confirmed non-consumers, so the rewrite is safe:

- **Cargo packaging**: nothing publishes these crates; `description` is read only
  by `tasks/manifest.py`. No `build.rs`, no `CARGO_PKG_DESCRIPTION` reader.
- **Visualiser**: `NoResultsPanel.tsx:19` `description` is an unrelated CSS
  class; the visualiser does not read manifest binary descriptions.
- **docs-site**: "description" hits are skill frontmatter, unrelated.

The work item's "dual-use" concern (crate `description` is also package metadata)
is real but inert here — no cargo/packaging consumer reads it beyond the manifest
build. 0164's original scoping put the listing on `--help` only, which is the
deliberate choice 0258 reverses.

### Test surface

Dispatch has a mature fixture harness; help-listing does not. The gap is
load-bearing for AC-6.

- **Dispatch fixtures** (`cli/launcher/tests/dispatch.rs`): the
  `accelerator-fixture` bin (`tests/fixtures/accelerator_fixture.rs`, second
  `[[bin]]` in `Cargo.toml:19-21`) is registered via `ACCELERATOR_<SUB>_BIN`
  override vars, bypassing network fetch. Tests cover exit-code propagation,
  hyphen→underscore var normalisation, SIGTERM propagation, per-command
  `--help` delegation to the child, and non-UTF-8 arg passthrough. None assert
  top-level help text.
- **Help unit tests** (`help.rs:40-83`): feed a hand-built `Manifest` to
  `external_subcommands_section()` and assert substring presence (`foo`, `Bar
  tool`) and sanitisation. This is the only place a populated listing is tested.
- **Black-box help** (`cli/launcher/tests/help.rs`): forces the manifest fetch
  to fail via `DEAD_RELEASE_URL` and asserts `--help` still prints built-ins
  (`version`) at exit 0. Only the manifest-**unavailable** path is covered.
- ⚠️ **No black-box test registers a sub-binary and asserts it appears in
  top-level help.** The module doc at `tests/help.rs:1-3` states why: a test
  cannot sign a manifest under the embedded key. `help_section()`
  (`main.rs:82-91`) loads and signature-verifies the manifest; the only env seam
  is `ACCELERATOR_RELEASE_BASE_URL`, usable to force failure, not to inject a
  valid signed manifest. AC-6 ("a fixture sub-binary appears in all three help
  outputs") requires a new manifest-injection seam that bypasses or overrides
  signature verification in the help path — it does not exist today.
- ⚠️ **Two Python test mirrors pin descriptions verbatim** and will break on the
  rewrite: `tests/integration/tasks/test_github.py:40-59` (`_SUBBINARY_
  DESCRIPTIONS`, all nine, used at line 350) and `tests/unit/tasks/
  test_manifest.py:158-160` (visualiser only). The work item's "Tests to extend"
  (`0258:156-157`) lists only launcher tests and omits both.

## Code References

- `cli/launcher/src/main.rs:106-117` — `is_root_help()` word-list guard; excludes `help`.
- `cli/launcher/src/main.rs:123-143` — `handle_parse_error()` three-way error-kind routing.
- `cli/launcher/src/main.rs:93-101` — `render_augmented_help()`; `after_help` merge (the only augmented path).
- `cli/launcher/src/main.rs:82-91` — `help_section()`; signature-verifies manifest, fail-open `None`.
- `cli/launcher/src/main.rs:138-141` — `_` arm; bare invocation exits 1 (`MissingSubcommand`).
- `cli/launcher/src/launch/help.rs:14-31` — `external_subcommands_section()`; `"External subcommands:"` literal at `:24`.
- `cli/launcher/src/launch/help.rs:36-38` — `sanitize()`; strips control chars.
- `cli/launcher/src/launch/inbound/cli.rs:16-35` — `Command` enum; three built-ins + `External`, no `Help` variant.
- `cli/launcher/src/launch/outbound/resolve/manifest.rs:36-42` — `BinaryEntry`; read contract.
- `tasks/manifest.py:106-112` — `_read_description()`; reads crate `Cargo.toml`, requires non-empty.
- `tasks/shared/paths.py:29-39` — `DISPATCHED_SUBBINARIES` token tuple.
- `cli/*-cli/Cargo.toml` (+ `cli/visualiser/server/Cargo.toml`) — the nine `description` fields to rewrite (seven need change).
- `cli/launcher/tests/dispatch.rs`, `tests/help.rs`, `tests/fixtures/accelerator_fixture.rs` — dispatch fixtures; no populated-listing black-box test.
- `tests/integration/tasks/test_github.py:40-59`, `tests/unit/tasks/test_manifest.py:158-160` — Python description mirrors to update.

## Architecture Insights

- **Two disjoint command sources.** Built-ins are compile-time clap enum
  variants; sub-binaries are runtime manifest entries. clap cannot enumerate the
  latter (`external_subcommand` is a blind catch-all), so any merged listing must
  be composed by hand, not delegated to clap's help renderer.
- **Help lives in the error path.** clap models help/version/missing-subcommand
  as errors. Converging the three entry points is a routing change in
  `handle_parse_error()` plus retiring the `is_root_help()` word-list, feeding
  all display paths through one full-listing renderer while preserving the `_`
  arm's exit-1 for bare invocation.
- **Signed manifest is the trust and discovery boundary.** Descriptions are
  signature-verified, which is why the runtime sanitises them and why tests
  cannot inject a listing without a new seam. The same signing that secures
  dispatch blocks black-box help-listing coverage.
- **Source of truth is deliberately outside the launcher.** Descriptions live in
  crate `Cargo.toml` and flow through the Python build system, so the
  user-facing rewrite is a cross-language change (Rust help renderer + Python
  packaging + Python test mirrors), not a launcher-only edit.

## Historical Context

- `meta/work/0164-launcher-and-git-style-dispatch.md` (done) — scoped the
  manifest-driven listing to `--help` and unknown-subcommand only; the
  deliberate decision 0258 reverses.
- `meta/work/0187-generalise-sub-binary-registration-surface.md` (done) — the
  generalised registration surface and thirteen-point checklist the listing
  enumerates from.
- `meta/decisions/ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md`
  — governing decision: clap `external_subcommand` + Unix `exec`; the basis for
  the manifest-driven listing.
- `meta/decisions/ADR-0046-zero-setup-static-binary-distribution.md`,
  `ADR-0060-launcher-resolved-tree-artifacts.md`,
  `ADR-0063-plugin-version-scoped-artifact-cache.md` — distribution and caching
  mechanics behind the sub-binaries.
- `meta/notes/2026-06-23-further-ideas-backlog.md:50` — origin note for 0258,
  0259, 0260.
- `meta/reviews/work/0258-help-show-subcommands-review-1.md` — existing review of
  the work item (currently staged).

## Related Research

- `meta/research/codebase/2026-07-03-0164-launcher-and-git-style-dispatch.md` —
  launcher/dispatch design.
- `meta/research/codebase/2026-08-02-0187-generalise-sub-binary-registration-surface.md`
  — registration surface.
- `meta/research/codebase/2026-07-06-0165-multi-binary-distribution-release-pipeline.md`
  — multi-binary distribution.

## Open Questions

- ❓ **AC-6 test seam.** A black-box test asserting a fixture sub-binary in all
  three help outputs needs a manifest-injection seam that bypasses signature
  verification in `help_section()`. None exists. Options: a test-only env
  override feeding a pre-parsed `Manifest`, or restructuring the listing so it is
  unit-testable end-to-end without signing. Which seam is acceptable is a design
  decision for the plan.
- ❓ **`visualiser` description verb.** Intended text is "Serve the
  meta-directory visualiser"; the crate carries "Launch the interactive
  meta-directory visualiser". Rewrite to match exactly, or accept the near-match?
  `migrate` already matches and needs no change.
- ❓ **0260 sequencing.** 0260 moves ADR into a top-level `adr` subcommand,
  changing the command tree the listing renders. If 0260 lands first, the merged
  listing must enumerate `adr`; if 0258 lands first, 0260 inherits the merged
  renderer. The work item defers layout ownership to 0258 and styling to 0259.
