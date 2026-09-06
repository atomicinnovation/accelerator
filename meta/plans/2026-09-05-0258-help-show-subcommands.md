---
type: "plan"
id: "2026-09-05-0258-help-show-subcommands"
title: "Help Should Show Subcommands Implementation Plan"
date: "2026-09-05T17:08:36+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "ready"
work_item_id: "work-item:0258"
parent: "work-item:0258"
derived_from: ["codebase-research:2026-09-05-0258-help-show-subcommands"]
tags: ["cli", "help", "launcher", "discoverability"]
revision: "7cac7010e86cde524f6e3be1470c0f8d52469e50"
repository: "accelerator"
last_updated: "2026-09-06T08:57:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Help Should Show Subcommands Implementation Plan

## Overview

Converge `accelerator help`, bare `accelerator`, and `accelerator --help` on one
merged, user-facing command listing that shows built-ins and dispatched
sub-binaries in a single unheadinged "Commands:" section. The listing is driven
by the signed manifest, so a newly registered sub-binary appears automatically.
Bare invocation keeps its non-zero exit.

## Current State Analysis

Three entry points diverge in `cli/launcher/src/main.rs`. clap models help,
version, and missing-subcommand as `Err(clap::Error)`, and only `--help` reaches
the augmentation path:

```text
accelerator --help   → Err(DisplayHelp)        → is_root_help TRUE  → render_augmented_help()  [built-ins + "External subcommands:" block, stdout, exit 0]
accelerator help     → Err(DisplayHelp)        → is_root_help FALSE → print!("{error}")        [built-ins only, stdout, exit 0]
accelerator (bare)   → Err(MissingSubcommand)  → is_root_help FALSE → error.print()            [usage error, stderr, exit 1]
```

`is_root_help()` (`main.rs:106-117`) returns `true` only when the kind is
`DisplayHelp` **and** `args_os().nth(1)` is not one of `version|config|cache|
help` — the word-list excludes `help`. `handle_parse_error()` (`main.rs:123-143`)
routes `DisplayHelp if is_root_help` to `render_augmented_help()`, other display
kinds to a plain `print!`, and everything else (including bare's
`MissingSubcommand`) to the exit-1 usage-error arm.

`render_augmented_help()` (`main.rs:93-101`) attaches the sub-binary block via
clap `after_help`, sourced from `external_subcommands_section()`
(`help.rs:14-31`), which hard-codes the `"External subcommands:"` heading at
`help.rs:24`. Built-ins live in the clap `Command` enum
(`cli.rs:16-35`); sub-binaries live only in the signed manifest — two disjoint
sources clap cannot unify on its own.

The user-facing descriptions are crate `Cargo.toml` `package.description`,
flowing through `tasks/manifest.py` `_read_description()` (`manifest.py:106-112`)
into the signed manifest and out via `BinaryEntry.description` (`manifest.rs:36-42`).
Seven of nine carry launcher-internal phrasing; `migrate` matches AC-5 in
wording but carries a stray trailing period; `visualiser` changes verb
(Launch→Serve) and drops "interactive".

## Desired End State

All three entry points print an identical command listing — built-ins and
sub-binaries in one "Commands:" section, no origin heading, each carrying a
concise user-facing description. `--help` and `help` exit 0; bare exits 1. A new
sub-binary appears with no help-code change. Verify by running the three commands
and diffing their command lists, and by the automated tests below.

The sub-binary rows are fail-open: each entry point loads the manifest
independently through `Fetcher::for_help()`, so the three listings are identical
only when the manifest-load result is stable across the three invocations — the
normal case (host reachable, or unreachable for all three). Near the 3s help
timeout on a marginal link, back-to-back invocations can legitimately differ
(sub-binaries present vs. failed open to built-ins); the diff check assumes a
stable load, not a hard cross-invocation invariant.

### Key Discoveries

- Divergence is two gates feeding one path in `main.rs`: the `is_root_help()`
  word-list (`main.rs:106-117`) and the `handle_parse_error()` routing
  (`main.rs:123-143`).
- Bare invocation is `ErrorKind::MissingSubcommand`, **not**
  `DisplayHelpOnMissingArgumentOrSubcommand` (which is bare `config`/`cache`,
  exits 0). The work item Technical Notes (`0258:141-144`) are wrong here; AC-3's
  non-zero exit is satisfiable because `MissingSubcommand` is the mechanism.
- `help` is clap's synthesised subcommand, not a declared `Command` variant
  (`cli.rs:16-35` has only `Version`, `Config`, `Cache`, `External`).
- The description source of truth is crate `Cargo.toml`, bridged only by
  `tasks/manifest.py`. No `build.rs`, no `CARGO_PKG_DESCRIPTION` reader, no cargo
  publish — the rewrite is safe (`manifest.py:106-112`).
- ⚠️ `config --help` is also `ErrorKind::DisplayHelp`. The `is_root_help` guard
  must **narrow**, not vanish — removing it wholesale would route `config --help`
  to top-level help.
- Only **one** Python test breaks: `test_manifest.py:152-160` reads the real
  `visualiser` crate description. `test_github.py:_SUBBINARY_DESCRIPTIONS`
  (`:40-63`) is a self-contained upload-flow fixture — its comment (`:36-39`)
  states the strings are arbitrary, and they are never compared to crate text, so
  the rewrite does not break it. This corrects the research, which listed both.
- A black-box test cannot forge a signed manifest (`tests/help.rs:1-3`), so the
  populated-listing assertion stays at the unit level; black-box covers only the
  manifest-unavailable path.

## What We're NOT Doing

- Not adding a manifest-injection or signature-bypass seam. The populated-listing
  assertion is a unit test over a hand-built `Manifest`; routing is unit-tested
  separately; black-box covers only the unavailable path.
- Not styling help beyond the single merged list — colour, section ordering, and
  wording polish are 0259's remit. This plan owns layout (one unheadinged list)
  only. Three consequences of that boundary are known and deliberately deferred,
  not oversights:
  - **Mixed description register.** After Phase 1 §5 strips their trailing
    periods, the built-in descriptions are still full sentences (`Read or write
    Accelerator configuration`) while the AC-5 sub-binary descriptions are terse
    imperative fragments. Punctuation is now uniform; the sentence-vs-fragment
    register split remains until 0259, whose scope must cover restyling the
    built-in doc comments to match.
  - **Ordering.** clap renders built-ins in declaration order (`version`,
    `config`, `cache`) followed by the manifest binaries alphabetically
    (`BTreeMap`). The intended order for this plan is that deliberate
    core-then-domain grouping. It is not obviously the most discoverable — it
    surfaces the infrastructure built-ins above the primary domain verbs (`work`,
    `jira`, …) a user most often wants — so 0259 should consciously choose between
    this grouping and a fully-alphabetical sort rather than inherit the
    clap-declaration order by default.
  - **AC-5 fixed wording.** `Serve the meta-directory visualiser` (accuracy over
    the more discoverable "Launch"/"Open"), `Validate and process design sources`
    (broad over enumerating design's command surface), and the `corpus`
    parenthetical that lists three of four subcommands (omitting `linkage`) are
    the verbatim strings AC-5 enumerates. This plan renders them as specified;
    revisiting the wording is 0259's remit.
- Not moving the ADR surface — that is 0260. This plan renders whatever the
  command surface is; if 0260 lands first the merged renderer inherits `adr`.
- Not touching dispatch, resolution, exec, caching, or signature verification —
  save one behaviour-preserving change in `fetcher.rs` (Phase 2 §6): parameterise
  `build()`'s timeouts/attempts and add a `for_help()` constructor for the
  fail-open help load. `new()`/`with_backoff()` pass today's values, so dispatch's
  fetch behaviour is unchanged.
- Not rewording `migrate`'s description — only trimming its trailing period so
  the merged listing is punctuation-uniform.

## Implementation Approach

Two independently mergeable phases, each leaving `mise run check` green alone.

Phase 1 rewrites the descriptions at their source of truth and fixes the one
Python assertion that reads a real crate description. It is valuable and complete
on its own: `--help`'s existing external block immediately shows clean wording.

Phase 2 collapses the two lists and converges the three entry points. It replaces
`after_help` with clap-native subcommand injection — the render-only clap
`Command` gains one child per manifest binary, so clap renders a single
"Commands:" section natively. Routing narrows `is_root_help` to the true root
forms and folds the whole error-kind decision into a pure `classify` function
over `(ErrorKind, &[OsString])`; `main` collects the args once and threads them
in, so the routing decision — and its exit-code contract — reads no process
global and is unit-testable end to end. Bare's `MissingSubcommand` routes to the
same renderer with exit 1 preserved.

Phase 2 is correct on top of Phase 1's descriptions but does not depend on them —
either order leaves the tree green.

## Phase 1: User-Facing Descriptions

### Overview

Rewrite the seven launcher-internal crate descriptions and `visualiser`'s verb to
the AC-5 text, trim the stray trailing periods (`migrate`'s description and the
three built-in doc comments) so the merged listing is punctuation-uniform, add a
regression guard over the descriptions, note the description's dual purpose in the
registration checklist, and update the one Python assertion that reads the real
crate description.

### Changes Required

#### 1. Sub-binary crate descriptions

**Files**: `cli/<token>-cli/Cargo.toml` (seven), `cli/visualiser/server/Cargo.toml`
**Changes**: rewrite `package.description` to the AC-5 user-facing text.

```diff
# cli/work-cli/Cargo.toml
-description = "The work create|show|resolve|diff|update sub-binary."
+description = "Manage work items"
```

```diff
# cli/vcs-cli/Cargo.toml
-description = "The vcs detect|status|log|guard sub-binary."
+description = "Detect and inspect version-control state"
```

```diff
# cli/jira-cli/Cargo.toml
-description = "The jira create|update|show|search|comment|transition|attach|init|fields|resolve-fields sub-binary."
+description = "Manage Jira issues"
```

```diff
# cli/linear-cli/Cargo.toml
-description = "The linear create|update|show|search|comment|transition|attach|init sub-binary."
+description = "Manage Linear issues"
```

```diff
# cli/corpus-cli/Cargo.toml
-description = "The corpus adr|metadata|linkage|frontmatter sub-binary."
+description = "Manage the meta-document corpus (ADRs, metadata, frontmatter)"
```

```diff
# cli/collaboration-cli/Cargo.toml
-description = "The collaboration pr base-repo|update-body sub-binary."
+description = "Manage pull-request collaboration"
```

```diff
# cli/design-cli/Cargo.toml
-description = "The design validate-source|resolve-auth|scrub-secrets|notify-downgrade|audit-cue-phrases sub-binary."
+description = "Validate and process design sources"
```

```diff
# cli/visualiser/server/Cargo.toml
-description = "Launch the interactive meta-directory visualiser"
+description = "Serve the meta-directory visualiser"
```

```diff
# cli/migrate-cli/Cargo.toml
-description = "Apply pending meta-directory schema migrations."
+description = "Apply pending meta-directory schema migrations"
```

`migrate`'s wording already matches AC-5, but its trailing full stop does not —
the AC-5 text and all eight rewrites above carry none. Left as-is, `migrate`
would be the only entry in the merged listing ending in a period. Drop the stop
so all nine descriptions share one no-trailing-period convention.

#### 2. The one breaking Python assertion

**File**: `tests/unit/tasks/test_manifest.py`
**Changes**: update the expected string in
`test_default_manifest_maps_visualiser_to_the_server_crate` (`:152-160`).

```diff
-        expected = "Launch the interactive meta-directory visualiser"
+        expected = "Serve the meta-directory visualiser"
```

`tests/integration/tasks/test_github.py:_SUBBINARY_DESCRIPTIONS` needs no change:
its own comment (`:36-39`) already states the strings are arbitrary upload-flow
placeholders, never compared to crate text, so the retired `"… sub-binary."`
strings there are documented as placeholders rather than a stale mirror of the
contract. Leave them; aligning them is cosmetic and not required for green.

#### 3. A regression guard that descriptions stay user-facing

**File**: `tests/unit/tasks/test_manifest.py`
**Changes**: add a test that collects every sub-binary crate `description` through
the same `manifest.py` path and asserts none contains the substring `sub-binary`
and none ends with a `.`.

AC-5 is a standing behavioural requirement — "no listed command carries
launcher-internal phrasing such as '… sub-binary.'" — and today's only checks are
a one-time visualiser assertion and manual review. Without a guard, the next
sub-binary registered with `"The … sub-binary."` phrasing silently reintroduces
the defect this work item fixes. The two assertions track the source of truth
(the crate descriptions) rather than a hand-maintained expected list: the
substring check guards the phrasing, and the trailing-period check guards the
no-trailing-period convention `migrate`'s edit establishes.

#### 4. Make the description's dual purpose discoverable

**File**: `tasks/README.md` (the sub-binary registration checklist)
**Changes**: extend the existing `package.description` checklist item (point 3)
to note that the description is user-facing CLI help text (rendered in the merged
listing), not only package metadata. Extending point 3 rather than adding a new
point keeps the list at thirteen, so the "thirteen-point" references in `CLAUDE.md`
and `tasks/CLAUDE.md` stay accurate.

This is a recorded, deliberate overload — the work item's Dependencies section
already documents it — but the coupling is invisible at the `Cargo.toml` edit
site, so a maintainer tuning a description for packaging could silently reword the
CLI help. The checklist line makes the constraint discoverable where a new
sub-binary is registered, and the phrasing guard in #3 is the enforcing tripwire.

#### 5. Strip the built-in descriptions' trailing periods

**File**: `cli/launcher/src/launch/inbound/cli.rs`
**Changes**: drop the trailing period from the three built-in `///` doc comments
that render as top-level command descriptions.

```diff
-    /// Print the version, commit SHA, build date, and target triple.
+    /// Print the version, commit SHA, build date, and target triple
     Version,
-    /// Read or write Accelerator configuration.
+    /// Read or write Accelerator configuration
     Config {
-    /// Inspect and repair the cached directory-tree artifacts.
+    /// Inspect and repair the cached directory-tree artifacts
     Cache {
```

This is the punctuation half of the mixed-register consequence, and it is
mechanical: with `migrate` and all eight sub-binary descriptions period-free, the
built-ins were the only entries left ending in a stop, so the merged listing is
now uniformly period-free. The substantive register split — the built-ins are
sentences, the sub-binaries terse fragments — is untouched and remains 0259's
remit.

### Success Criteria

#### Automated Verification

- [x] Rust workspace check passes: `mise run cli:check`
- [x] Python build-system check passes: `mise run build-system:check`
- [x] `test_manifest.py` visualiser assertion passes: `uv run pytest
      tests/unit/tasks/test_manifest.py -k visualiser`
- [x] Sub-binary phrasing guard passes (no description contains `sub-binary` or
      ends with `.`): `uv run pytest tests/unit/tasks/test_manifest.py -k phrasing`
- [x] Read-only CI mirror passes: `mise run check`

#### Manual Verification

- [ ] `accelerator --help` shows the new descriptions in the "External
      subcommands:" block (pre-Phase-2 layout).
- [ ] No description reads "… sub-binary."; `visualiser` reads "Serve the
      meta-directory visualiser"; no description ends in a trailing period.

---

## Phase 2: Merge the List and Converge the Entry Points

### Overview

Replace the `after_help` external block with clap-native subcommand injection so
built-ins and sub-binaries render in one "Commands:" section, and route all three
entry points through it, preserving exit codes.

### Changes Required

#### 1. Manifest → clap subcommand augmentation

**File**: `cli/launcher/src/launch/help.rs`
**Changes**: replace `external_subcommands_section()` (which returned an
`after_help` block) with a function that adds each manifest binary to a clap
`Command` as a child carrying its sanitised description as `about`. Keep
`sanitize()`. Test-drive with a hand-built `Manifest`.

```rust
pub fn augment_with_subbinaries(
    mut command: clap::Command,
    manifest: &Manifest,
) -> clap::Command {
    for (name, entry) in &manifest.binaries {
        let name = sanitize(name);
        if name.is_empty()
            || name == "help"
            || command.find_subcommand(&name).is_some()
        {
            continue;
        }
        command = command.subcommand(
            clap::Command::new(name).about(sanitize(&entry.description)),
        );
    }
    command
}
```

The guard is load-bearing, not defensive dressing: clap `debug_assert`s on a
duplicate subcommand, so a manifest binary colliding with a built-in would panic
the help path under `cargo test` and silently double-list in release. `sanitize()`
strips control characters but not collisions or empties, so the guard skips any
sanitised name that is empty, `help`, or already present. Two distinct sources
must be covered: the declared built-ins (`version`, `config`, `cache`) are present
on the un-built `Cli::command()` and caught by `find_subcommand`; clap's `help`
subcommand is **not** — it is appended only during `_build` (triggered at render
time) with no dedup, so `find_subcommand("help")` returns `None` at augmentation
time. The explicit `name == "help"` skip is what actually prevents the `help`
collision; `find_subcommand` alone would miss it. A built-in or `help` always
wins, and such a name could never dispatch anyway (it is shadowed), so the skipped
entry loses nothing real.

Unit tests (the AC-6 renderer test plus the collision guard): build a `Manifest`
with a `zzfixture` binary, augment `Cli::command()`, render via
`command.render_help().to_string()`, assert the output contains `zzfixture` and
its description **and** a built-in (`version`), and does **not** contain `External
subcommands`. Adding the fixture is manifest data — the renderer is untouched,
satisfying "no change to help code". A second case builds a manifest whose
binaries include names colliding with a declared built-in (`version`) and with
clap's synthesised `help`, plus an empty name; it augments, renders, and asserts
the render neither panics nor lists `version` or `help` twice. The empty-name
half is asserted distinctly — `command.get_subcommands()` on the augmented command
yields no subcommand with an empty name — so dropping the `is_empty()` guard (which
clap accepts without panicking) reddens this test rather than passing silently.

#### 2. One renderer for all root-help forms

**File**: `cli/launcher/src/main.rs`
**Changes**: fold `render_augmented_help()` and `help_section()` into one path
that builds `Cli::command()`, augments it with the manifest when available
(fail-open to built-ins), prints to stdout, and returns the caller's exit code.
Split the pure composition — command plus optional manifest to the listing
command — out of the I/O so it is unit-testable without a signed manifest.

```rust
fn build_listing(
    command: clap::Command,
    manifest: Option<&Manifest>,
) -> clap::Command {
    match manifest {
        Some(manifest) => augment_with_subbinaries(command, manifest),
        None => command,
    }
}

fn render_full_listing(exit: ExitCode) -> ExitCode {
    let command = build_listing(Cli::command(), load_help_manifest().as_ref());
    let _ = command.print_help();
    println!();
    exit
}
```

`load_help_manifest()` is today's `help_section()` body up to the verified
`Manifest` (signature-verified, `None` on any failure).

Splitting `build_listing` closes the composition gap: `render_full_listing`'s own
branch (augment when `Some`, built-ins when `None`) is otherwise never exercised,
because every black-box test forces the manifest unavailable, so deleting the
augmentation — the whole point of the work item — would leave the suite green.
Unit-test `build_listing` directly: `Some(&fixture_manifest)` yields a render
containing the `zzfixture` sub-binary; `None` yields built-ins only, with
`zzfixture` absent. This proves the wired path populates the listing without
forging a signed manifest.

`load_help_manifest` bounds its fetch to a single short attempt via the
`Fetcher::for_help()` constructor from §6 (`max_attempts = 1`, 3s connect, 5s
total), distinct from dispatch's retry-heavy fetcher — so a slow or blackhole host
fails open to the built-ins within ~3s rather than inheriting the 3×-retry,
10s-connect, 300s-per-attempt archive budget. A single short attempt is the right
shape for a fail-open help render: unlike dispatch, help never needs the manifest,
so it should give up fast rather than retry.

Carry the two non-obvious rationales from the folded functions into the new doc
comments rather than re-deriving them later: `load_help_manifest` installs the
crypto provider lazily so a `version`/`config`/`cache` built-in never pays for
TLS it does not use, and it returns `None` on any failure so the help path fails
open to the built-in listing. Both are exactly the genuinely-non-obvious "why"
the project's comment policy keeps.

#### 3. Narrow the root-help predicate to a pure function

**File**: `cli/launcher/src/main.rs`
**Changes**: replace `is_root_help()`'s `args_os()` read with a pure function over
the parsed args, so `config --help` and `help <sub>` stay on clap's per-command
help while bare `help` and top-level `--help` reach the full listing.

```rust
fn is_root_help_args(args: &[OsString]) -> bool {
    match args {
        [first, ..] if first == "--help" || first == "-h" => true,
        [only] if only == "help" => true,
        _ => false,
    }
}
```

The leading-token match (not an exact single-element slice) matches the old
predicate's behaviour for a stray trailing token: `accelerator --help extra` is
still `DisplayHelp` with args `["--help", "extra"]`, and clap ignores the extra,
so it stays root and keeps its sub-binaries — an exact `[a]` match would have
silently dropped them. The bare word `help`, by contrast, is root only as the
sole token, so `help config` remains per-command help. `OsString` compares
against `&str` directly, so no allocation or `OsStr` conversion is needed.

`main` collects `std::env::args_os().skip(1).collect::<Vec<_>>()` once and threads
it into `handle_parse_error` (Section 4), so no function re-reads the global. This
treats `["--help"]`, `["-h"]`, `["help"]`, and `["--help","extra"]` as root;
`["config","--help"]`, `["help","config"]`, `["version","--help"]` are not root.

#### 4. Route the three kinds through one pure classifier

**File**: `cli/launcher/src/main.rs`
**Changes**: split the error-kind-to-outcome decision into a pure `classify`
function over `(ErrorKind, &[OsString])`, and pass the parsed args into
`handle_parse_error` so the routing decision reads no process global. `classify`
returns a `HelpRoute` whose `FullListing` carries a `ListingCause` naming *why*
the listing renders — the two reasons differ only in exit code and whether the
missing-subcommand cue prints, so modelling the cause keeps those derived rather
than stored as loose, independently-settable fields. The enum is `PartialEq`, so
the whole truth table is unit-testable; `handle_parse_error` maps the route to its
side effect: the full listing, clap's own per-command help/version, or the
usage-error arm.

```rust
#[derive(Debug, PartialEq, Eq)]
enum HelpRoute {
    FullListing(ListingCause),
    PerCommand,
    UsageError,
}

#[derive(Debug, PartialEq, Eq)]
enum ListingCause {
    RootHelp,
    MissingSubcommand,
}

fn classify(kind: ErrorKind, args: &[OsString]) -> HelpRoute {
    match kind {
        ErrorKind::DisplayHelp if is_root_help_args(args) => {
            HelpRoute::FullListing(ListingCause::RootHelp)
        }
        ErrorKind::MissingSubcommand if args.is_empty() => {
            HelpRoute::FullListing(ListingCause::MissingSubcommand)
        }
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayVersion
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            HelpRoute::PerCommand
        }
        _ => HelpRoute::UsageError,
    }
}

fn handle_parse_error(error: &clap::Error, args: &[OsString]) -> ExitCode {
    match classify(error.kind(), args) {
        HelpRoute::FullListing(cause) => {
            let exit = match cause {
                ListingCause::RootHelp => ExitCode::SUCCESS,
                ListingCause::MissingSubcommand => {
                    eprintln!(
                        "error: a subcommand is required; \
                         run 'accelerator --help' to see available commands"
                    );
                    ExitCode::from(1)
                }
            };
            render_full_listing(exit)
        }
        HelpRoute::PerCommand => {
            print!("{error}");
            ExitCode::SUCCESS
        }
        HelpRoute::UsageError => {
            let _ = error.print();
            ExitCode::from(1)
        }
    }
}
```

Carry the existing stdout-forcing rationale onto the `PerCommand` arm during the
fold: `print!("{error}")` forces stdout because clap routes
`DisplayHelpOnMissingArgumentOrSubcommand` to stderr, which would otherwise make
bare `config` print its help on a different stream than `config --help`. This is
the third non-obvious "why" to preserve, alongside the two in §2.

`main`'s call site becomes `handle_parse_error(&error, &root_args)`, where
`root_args` is the collected `Vec<OsString>` from Section 3.

The `if args.is_empty()` guard on the `MissingSubcommand` arm is load-bearing:
`MissingSubcommand` is not exclusive to bare `accelerator`. A nested required
group without `arg_required_else_help` — `accelerator config templates`, whose
`TemplatesAction` is required (`cli.rs:301-360`) — raises the same kind with args
`["config","templates"]`. Only the empty-args case is the root bare invocation;
a non-empty `MissingSubcommand` falls through to `UsageError`, so clap prints its
contextual per-group usage error exactly as today rather than dumping the
top-level listing.

The `MissingSubcommand` cause keeps bare invocation honest. Today bare
`accelerator` prints clap's usage error to stderr; routing it to the listing
would otherwise leave a success-looking help body on stdout with an exit-1 and
empty stderr — no signal that anything went wrong. Emitting `error: a subcommand
is required; run 'accelerator --help' …` to **stderr** while the listing goes to
**stdout** restores the diagnostic — with a next step — on the stream a caller
inspects, so `--help`/`help` (clean stdout, exit 0) and bare (listing on stdout,
cue on stderr, exit 1) stay distinguishable even under a stderr-only capture.

Routing tests (all pure, no process spawn): `classify` maps
`(DisplayHelp, ["--help"])`, `(DisplayHelp, ["-h"])`, `(DisplayHelp, ["help"])`,
and `(DisplayHelp, ["--help","extra"])` to `FullListing(ListingCause::RootHelp)`,
and `(MissingSubcommand, [])` to `FullListing(ListingCause::MissingSubcommand)`;
`(DisplayHelp, ["config","--help"])`, `(DisplayVersion, ["version","--help"])`,
and `(DisplayHelpOnMissingArgumentOrSubcommand, ["config"])` (bare `config`) map
to `PerCommand`; `(MissingSubcommand, ["config","templates"])` and a usage kind
map to `UsageError`. Because `render_full_listing` is the sole augmenting renderer
and both listing causes resolve to the one `FullListing` variant, this
structurally guarantees AC-6's "all three outputs" convergence without a
signed-manifest end-to-end test.

#### 5. Extend the black-box tests to all three entry points

**File**: `cli/launcher/tests/help.rs`
**Changes**: alongside the existing `--help` unavailable-path test, assert
`accelerator help` (exit 0) and bare `accelerator` (exit 1) both print a built-in
(`version`) with the manifest forced unavailable via `DEAD_RELEASE_URL`. This
proves all three entry points render the built-in listing end-to-end; the
populated listing stays a unit assertion (`tests/help.rs:1-3` constraint). The
bare case additionally asserts the stream separation the cue exists for: the
listing (`version`) is on **stdout** and `subcommand` appears on **stderr**, so a
regression that dropped the cue or emitted it to the wrong stream reddens the
test.

Add one further black-box case guarding the documented `config --help` hazard:
with the manifest forced unavailable, `accelerator config --help` prints config's
own help surface (a config-specific token) and **not** the top-level listing
(assert the output omits a sub-binary origin cue and a non-config built-in such
as `cache`). This is the end-to-end counterpart to the pure `classify` table: the
table proves `["config","--help"]` maps to `PerCommand`, and this test proves the
wired path renders config's help rather than the merged list.

#### 6. A help-scoped fetcher constructor

**File**: `cli/launcher/src/launch/outbound/resolve/fetcher.rs`
**Changes**: the tight bound §2 relies on is not achievable by setting
`max_attempts = 1` alone — `CONNECT_TIMEOUT` (10s) and `TOTAL_TIMEOUT` (300s) are
module constants baked into the reqwest client inside `build()`, so a single
attempt still waits out a 10s connect. Parameterise the per-instance limits and
add a help-scoped constructor.

```rust
const HELP_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const HELP_TOTAL_TIMEOUT: Duration = Duration::from_secs(5);

pub fn for_help() -> Result<Self, String> {
    Self::build(
        Limits {
            connect: HELP_CONNECT_TIMEOUT,
            total: HELP_TOTAL_TIMEOUT,
            max_attempts: 1,
            backoff: Duration::ZERO,
        },
        true,
    )
}
```

`build()` gains a `Limits` parameter feeding `connect_timeout`/`timeout` on the
client and `max_attempts` on the `Fetcher`; `new()` and `with_backoff()` pass a
`Limits` carrying today's `CONNECT_TIMEOUT`/`TOTAL_TIMEOUT`/`MAX_ATTEMPTS`, so
dispatch's fetch behaviour is unchanged. `for_help()` is what `load_help_manifest`
constructs.

Test-drive the bound: point `for_help()` at a blackhole host (a non-routable IP
that drops the SYN) and assert the fetch returns an error within a ceiling a few
seconds above `HELP_CONNECT_TIMEOUT` — one attempt, no retry, no 10s inheritance.
This pins the corrected worst case rather than narrating it.

### Success Criteria

#### Automated Verification

- [ ] Rust workspace check passes: `mise run cli:check`
- [ ] Launcher unit + black-box tests pass: `cargo test -p accelerator --manifest-path cli/Cargo.toml`
- [ ] Read-only CI mirror passes: `mise run check`
- [ ] Full local CI mirror passes: `mise run`

#### Manual Verification

- [ ] `accelerator --help`, `accelerator help`, and `accelerator` print an
      identical "Commands:" listing — built-ins and sub-binaries together, no
      "External subcommands" heading. Diff:
      `diff <(accelerator --help) <(accelerator help)` is empty. Run against a
      reachable release host so the manifest load is stable across the three (the
      sub-binary rows are fail-open, per Desired End State).
- [ ] Bare `accelerator` prints the full listing and exits non-zero
      (`accelerator; echo $?` prints `1`); `--help` and `help` exit 0.
- [ ] `accelerator config --help` still shows config's own help, not the
      top-level listing.
- [ ] `accelerator config templates` (a nested required group) still prints
      clap's contextual usage error, not the top-level listing.
- [ ] With the release host unreachable, all three still print the built-ins and
      keep their exit codes.

---

## Testing Strategy

### Unit Tests

- `augment_with_subbinaries` over a fixture `Manifest`: fixture binary and its
  description appear in the same "Commands:" section as a built-in; no "External
  subcommands" heading (`help.rs`). This is the AC-6 "registers a fixture
  sub-binary" test.
- `augment_with_subbinaries` collision guard (`help.rs`): a manifest whose names
  collide with a declared built-in (`version`), with clap's synthesised `help`,
  and include an empty name augments without panicking the renderer and lists
  neither `version` nor `help` twice.
- `build_listing` composition (`main.rs`): `Some(&fixture_manifest)` yields a
  render containing the fixture sub-binary; `None` yields built-ins only with the
  fixture absent. Guards the wired augmentation against deletion.
- The Python phrasing guard (`test_manifest.py`): no sub-binary crate
  `description` contains the substring `sub-binary`.
- `is_root_help_args` truth table (`main.rs`): `["--help"]`, `["-h"]`,
  `["help"]`, and `["--help","extra"]` true; `["config","--help"]`,
  `["help","config"]`, `["version","--help"]`, and `[]` false.
- `classify` truth table (`main.rs`): the root-help forms (`--help`, `-h`,
  `help`, `--help extra`) map to `FullListing(RootHelp)` and bare (`[]`) to
  `FullListing(MissingSubcommand)`; `config --help`, `version --help`, and bare
  `config` (`DisplayHelpOnMissingArgumentOrSubcommand`) map to `PerCommand`;
  `config templates` (a non-root `MissingSubcommand`) and a usage kind map to
  `UsageError`. This proves both listing causes converge on the single augmenting
  renderer, pins the exit-code contract, pins the bare-only stderr cue, pins the
  nested-group guard, and pins the preserved bare-`config` per-command routing —
  all without a process spawn.
- `sanitize` behaviour is unchanged and already covered (`help.rs:77-82`).

### Integration Tests

- Black-box, manifest-unavailable: `--help`, `help`, and bare `accelerator` each
  print built-ins; `--help`/`help` exit 0, bare exits 1 (`tests/help.rs`).
- Black-box, manifest-unavailable: `accelerator config --help` renders config's
  own help, not the top-level listing — the wired counterpart to the `classify`
  table's `config --help → PerCommand` case (`tests/help.rs`).
- Existing dispatch fixtures (`tests/dispatch.rs`) are unaffected — dispatch
  routing is untouched.
- `Fetcher::for_help()` bound (`fetcher.rs`): a fetch against a SYN-dropping
  blackhole host returns an error within a ceiling a few seconds above the 3s
  help connect timeout — one attempt, no retry, confirming the help path does not
  inherit dispatch's 10s-connect / 300s-total / 3×-retry budget.

### Manual Testing Steps

1. Run the three commands and diff their command lists — identical.
2. Confirm bare exits 1, `--help`/`help` exit 0.
3. Confirm `accelerator config --help` still renders config's own help.
4. Force the release host unreachable (`ACCELERATOR_RELEASE_BASE_URL=https://127.0.0.1:1`)
   and confirm all three fall open to built-ins with correct exit codes.
5. Point at a blackhole host that stalls the connect
   (`ACCELERATOR_RELEASE_BASE_URL=https://10.255.255.1`) and confirm bare
   `accelerator` returns the built-in listing within ~3s (the help connect
   timeout) rather than the multi-second retry budget dispatch would spend.

## Performance Considerations

Bare `accelerator` moves from an instant clap usage error (no I/O) to the same
manifest load `--help` already performs, because AC-1 requires the merged listing
on the bare path too — augmentation cannot be skipped there. The added cost is a
network round-trip on a path that previously did none.

Left on dispatch's shared `Fetcher`, that cost is loosely bounded: `get` retries
up to `MAX_ATTEMPTS` (3) with the reqwest client's 10s connect and 300s total
per-attempt timeouts (`fetcher.rs`), so a SYN-dropping host stalls bare
invocation ~30s and a host that connects but stalls the body stalls it minutes —
unacceptable for the most common mistyped invocation. The help path therefore
uses the help-scoped `Fetcher::for_help()` from Phase 2 §6 (`max_attempts = 1`, 3s
connect, 5s total): one short attempt, then fail open to the built-ins. A retry
budget is right for dispatch (it needs the binary) and wrong for help (it never
does), so bare and `help` degrade within ~3s against a dead host instead of
inheriting the archive-sized budget.

Built-in dispatch (`version`, `config`, `cache`) never constructs the resolver,
and that stays true — the manifest load lives only in the help path.

A manual verification below forces a blackhole host and confirms all three entry
points return within the connect bound.

## Migration Notes

None. No data or on-disk format changes. The crate `description` fields are
consumed only by `tasks/manifest.py`; no cargo publish or `build.rs` reads them.

## References

- Original work item: `meta/work/0258-help-show-subcommands.md`
- Related research: `meta/research/codebase/2026-09-05-0258-help-show-subcommands.md`
- Divergence locus: `cli/launcher/src/main.rs:93-143`
- External block: `cli/launcher/src/launch/help.rs:14-31`
- Built-in command enum: `cli/launcher/src/launch/inbound/cli.rs:16-35`
- Description bridge: `tasks/manifest.py:106-112`, `tasks/shared/paths.py:29-39`
- Test mirror to update: `tests/unit/tasks/test_manifest.py:152-160`
- Self-contained fixture (no change): `tests/integration/tasks/test_github.py:36-63`
