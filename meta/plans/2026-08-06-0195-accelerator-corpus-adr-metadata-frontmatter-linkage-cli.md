---
type: plan
id: "2026-08-06-0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli"
title: "accelerator-corpus: ADR, Metadata, Frontmatter Validation, and Linkage CLI Implementation Plan"
date: "2026-08-06T08:41:17+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0195"
parent: "work-item:0195"
derived_from: ["codebase-research:2026-08-06-0195-accelerator-corpus-cli-implementation-surface"]
tags: [rust, corpus, cli, adr, frontmatter, linkage, sub-binary]
revision: "bb58376ea87e91b416b640f9e010671ab9ccd65b"
repository: "accelerator"
last_updated: "2026-08-07T13:33:11+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# accelerator-corpus: ADR, Metadata, Frontmatter Validation, and Linkage CLI Implementation Plan

## Overview

Build `accelerator-corpus`, a new dispatched sub-binary exposing `accelerator
corpus <noun> <verb>`, over four noun groups — `adr`, `metadata`,
`linkage`, `frontmatter` — replacing five bash entry points
(`adr-next-number.sh`, `adr-read-status.sh`, `artifact-derive-metadata.sh`,
`validate-corpus-frontmatter.sh`, `linkage-parser.sh`) and every caller of
them. Delivered as five independently mergeable phases, smallest/most
self-contained group first.

## Current State Analysis

Five bash scripts, most of their behaviour already characterised in
`meta/research/codebase/2026-08-06-0195-accelerator-corpus-cli-implementation-surface.md`.
Two of the four noun groups' domain logic already exists in Rust and is
differentially parity-tested: `corpus::linkage::parse_document` (linkage
extraction) and `corpus_adapters::metadata::{derive_at,render}` (metadata
derivation). ADR numbering/status has no Rust equivalent anywhere. Frontmatter
validation (16 violation codes, a 13-row per-type schema table, whole-corpus
referential integrity) has no Rust equivalent beyond the generic untyped
frontmatter parser (`corpus_adapters::document::parse`). No CLI-facing crate
reads `work.id_pattern`/doc-type directories from `.accelerator/config.md`
today — only the visualiser server does, and only for its own long-running
process, not a one-shot CLI invocation.

### Key Discoveries

- `corpus::linkage::extract_bare_ids` (`cli/corpus/src/linkage.rs:498-514`)
  already hardcodes the exact same 4-digit bare-number scan as bash's
  `lp_extract_tokens` (`grep -oE '[0-9]{4}'`,
  `scripts/linkage-parser.sh:279-291`) — confirmed byte-for-byte equivalent, so
  the "preserve or fix the gap" open question from the research document is
  resolved: preserve, no change needed, the Rust port already does.
- `corpus::typed_ref::parse_typed_ref` (`cli/corpus/src/typed_ref.rs`) is **not**
  reusable for `frontmatter validate`'s `BAD-LINKAGE-SHAPE` check — it
  recognises only four prefixes (`work-item`, `plan`, `adr`, `pr`) and
  special-cases path-shaped values, whereas the bash validator's
  `FM_TYPED_REF_RE` (`scripts/frontmatter-emission-rules.sh:88`) is
  `^(${FM_SOURCE_TYPE_RE}):[A-Za-z0-9.-]+$` over all 14 vocabulary types
  (`scripts/frontmatter-emission-rules.sh:41`) with no path carve-out. A new,
  purpose-built shape check is needed.
- The config→corpus wiring precedent is
  `cli/visualiser/server/src/config.rs`'s `WorkItemConfig` (composes
  `corpus::WorkItemIdScheme` + `corpus_adapters::RegexScanner`) and the
  launcher's own composition root, `compose_stack`
  (`cli/launcher/src/main.rs:149-171`), which calls
  `config_adapters::compose(&start, policy)` to get a `ConfigAccess` +
  `FileConfigStore`. `accelerator-corpus` should link `config`/`config-adapters`
  directly as libraries and compose the same way — not shell out to itself.
  `FileConfigStore::discover_root` (`cli/config-adapters/src/store.rs:113-125`)
  already walks up for `.accelerator`/`.git`/`.jj`, so no Rust port of
  `vcs-common.sh`'s `find_repo_root` is needed.
- `config::paths::doc_type_dirs` (`cli/config/src/paths.rs:30-67`) resolves
  each doc-type's directory as `&'static str` names sourced from
  `catalogue::DOC_TYPES`; these names match `corpus::DocTypeKey`'s own
  `linkage_type_name()` vocabulary, so building the `Vec<(DocTypeKey,
  PathBuf)>` table `corpus::linkage`/the new frontmatter validator need is a
  small lookup, not new design.
- The registration checklist's edit points are concrete and already located:
  `tasks/shared/paths.py:29` (`DISPATCHED_SUBBINARIES`), `tasks/manifest.py:52-57`
  (`_SUBBINARY_MANIFESTS`), `tasks/build.py:35` (`_CLI_RELEASE_BINARIES`),
  `tests/integration/tasks/test_github.py:507-510` (registry pin, currently
  `("visualiser", "vcs")`), `:338` (`assert len(uploads) == 30`, not 22 as
  `tasks/README.md`'s worked example says — the doc is stale), and
  `:252-320` (`_setup_release`'s two-token hardcoded fixture/manifest, which
  `tasks/README.md#registering-a-dispatched-sub-binary` point 1 explicitly
  expects the first additional sibling to generalise into a derived
  expression).
- `scripts/test-metadata-helpers.sh` loops over **three** sibling scripts
  (`artifact-derive-metadata.sh` plus two out-of-scope design-domain helpers,
  `skills/design/inventory-design/scripts/inventory-metadata.sh` and
  `skills/design/analyse-design-gaps/scripts/gap-metadata.sh`) — it cannot be
  deleted wholesale; only the one HELPERS entry for
  `artifact-derive-metadata.sh` is removed.
- `scripts/test-validate-corpus-frontmatter.sh` is a **required-by-name** CI
  gate (`tasks/test/integration.py:63-66`,
  `_REQUIRED_CONFIG_SUITES`) serving as the migration-completion signal for a
  *different*, already-shipped migration (work item 0102). Its floor/registry
  entry must be removed in the same change that removes the file, and the
  Rust-native `frontmatter validate` test suite takes over as the
  fail-closed guarantee going forward (a plain `cargo test` failure already
  fails CI unconditionally — no bash-style "required suite" registry is
  needed on the Rust side, matching how no such registry exists for
  `vcs-cli`'s or `corpus-adapters`' own tests today).
- Per user direction: **all** bash test suites this item touches are retired
  by the end of the work, not kept-and-repointed indefinitely. Any
  `bash-parity`-feature-gated Rust test that shells out to a script this item
  removes (`corpus-adapters/tests/metadata.rs`'s
  `derive_at_agrees_with_the_live_metadata_helper`,
  `corpus-adapters/tests/parity.rs`'s `linkage_extraction_matches_the_bash_parser`)
  is deleted in the same phase that removes its oracle script, after being
  used once, transiently, during development to confirm parity.

## What We're NOT Doing

- Not touching `doc-type-inference.sh`, `doc-type-table.sh`,
  `templates-schema.tsv`, `linkage-type-pairs.tsv`, `vcs-common.sh`, or
  `frontmatter-emission-rules.sh` as bash files — they remain (used by the
  0007 migration script and other things outside 0195's scope), even though
  their *data* is mirrored into new Rust constants.
- Not changing `artifact-derive-metadata.sh`'s output contract/shape.
- Not rewriting the 0007 migration script's own doc-type-table sourcing —
  only its two call-out lines to `validate-corpus-frontmatter.sh` and
  `linkage-parser.sh`.
- Not building a typed-deserialization layer over frontmatter — following the
  established untyped `Mapping`/`FrontmatterValue` navigation convention.
- Not touching `accelerator-vcs`, `accelerator-visualiser`, or any other
  sub-binary beyond the shared registration-machinery edits every new token
  requires.
- Clap's own argument-parsing usage errors (missing required argument,
  unrecognised flag) are **not** matched byte-for-byte against bash's literal
  `Usage: ...` text — that is CLI-framework presentation, not domain logic.
  Byte-for-byte characterization applies to domain-level success and failure
  output: file content, count arithmetic, status parsing, violation
  messages, and exit codes for domain-level failures (a bad `--count` value,
  a missing ADR file, a schema violation). Exit *codes* are not exempted by
  this carve-out, including for usage errors — bash's ADR scripts exit 1 for
  a bad invocation, so where clap's own built-in validation would otherwise
  produce its default exit code 2 for an equivalent case (e.g. `adr
  read-status`'s missing `file` argument), the CLI hand-validates instead
  (mirroring the `--count` pattern already scoped for `adr next-number`) to
  preserve exit 1. See Phase 1's `adr read-status` command-layer notes.
- Not adding a new skill. Skill edits in this plan are call-site rewrites
  inside existing `SKILL.md` files.

## Implementation Approach

Bottom-up by noun group, smallest and most self-contained first. Each phase:
writes failing Rust tests against bash-captured goldens first (TDD), reuses
existing `corpus`/`corpus-adapters` primitives wherever they already exist,
builds new domain logic only where genuinely absent, rewrites that group's
callers, removes the migrated bash (source script and, where retirable, its
dedicated test harness), and adjusts CI floors. Phase 1 also carries the
one-time binary scaffold and sub-binary registration, since registration
checklist points 1/2/3/4/7/8 must land together and point 7 (skill binding)
needs a real subcommand to bind to.

## Phase 1: Binary scaffold, sub-binary registration, and the `adr` noun group

### Overview

Creates `cli/corpus-cli/` (package `accelerator-corpus`), registers it as a
dispatched sub-binary end to end, and implements `adr next-number` / `adr
read-status` — the smallest group (`read-status` needs no config or VCS
access at all; `next-number` needs only a `paths.decisions` lookup).

### Changes Required

#### 1. Crate scaffold

**File**: `cli/corpus-cli/Cargo.toml`
**Changes**: New crate, mirroring `cli/vcs-cli/Cargo.toml`'s shape:
`[package] name = "accelerator-corpus"`, workspace-inherited
version/edition/rust-version/license/publish, mandatory `description`,
`[lints] workspace = true`, `[[bin]] name = "accelerator-corpus"`, `path =
"src/main.rs"`. Dependencies: `corpus`, `corpus-adapters`, `config`,
`config-adapters`, `kernel`, `clap` (workspace). No `serde_json`/`regex`
direct edge — neither is needed at this layer (regex compilation stays
inside `corpus_adapters::RegexScanner`; the frontmatter shape checks in
Phase 4 stay hand-rolled in `corpus`, matching how `corpus::linkage` avoids
`regex`).

**File**: `cli/Cargo.toml`
**Changes**: Add `"corpus-cli"` to `[workspace].members`. Regenerate
`cli/Cargo.lock` (`lint:cli:check` runs `--locked`).

**File**: `cli/corpus-cli/src/cli.rs`
**Changes**: New, mirroring `cli/vcs-cli/src/cli.rs`'s `Cli`/`Command` shape:

```rust
#[derive(Parser)]
#[command(name = "accelerator-corpus", disable_version_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Adr {
        #[command(subcommand)]
        action: AdrAction,
    },
}

#[derive(Subcommand)]
pub enum AdrAction {
    NextNumber {
        #[arg(long, default_value = "1")]
        count: String,
    },
    ReadStatus {
        file: Option<PathBuf>,
    },
}
```

`count` is a raw `String`, hand-validated against `^[1-9][0-9]*$` inside the
handler (not a clap-parsed `u32`) so the exact bash error text and exit code
1 are reproducible for an invalid value. **This raw-`String`-plus-hand-
validation shape is a bash-parity-driven exception, not this CLI's general
argument-validation convention** — it exists solely to reproduce bash's
exact error text/exit code, and should not be copied for a future argument
with no such parity requirement (where a clap-parsed type is preferable).
`ReadStatus`'s `file` is likewise a bare optional positional (not a
clap-required argument) so a missing argument reaches the handler as `None`
rather than triggering clap's own required-argument error, which defaults
to exit code 2 — the handler hand-validates presence and produces bash's
exit-1 message instead. See the "What We're NOT Doing" scoping note on
clap-vs-domain error/exit-code parity. `Metadata`/`Linkage`/`Frontmatter`
variants are added in their own phases, not stubbed here.

Every `Command`/`*Action` variant and field, in this snippet and in every
later phase's `cli.rs` additions, carries a doc comment (`///`) describing
what it does — clap surfaces these as `--help` text, and `cli/vcs-cli/src/
cli.rs` already does this for every variant/flag. The illustrative snippets
in this plan omit them for brevity; the implementation does not.

**File**: `cli/corpus-cli/src/main.rs`
**Changes**: Dispatcher mirroring `cli/vcs-cli/src/main.rs`'s shape: parse
`Cli`, match on `Command`, map the result to `ExitCode` the same way
(`Refusal` → 2, anything else → `FAILURE`). Unlike `vcs-cli`'s dispatcher,
this one is also the sole composition root — it composes config and the
real filesystem adapter and resolves per-command inputs before calling into
the generic, port-injected command functions; see Section 3 below for the
full `Adr` branch (Phase 3 and 4 extend this same dispatcher with their own
branches when their commands are added).

#### 2. ADR domain logic (pure)

**File**: `cli/corpus/src/adr.rs`
**Changes**: New module.

```rust
pub fn next_numbers(existing: &[&str], count: u32) -> Vec<String>
pub fn read_status(content: &str) -> Option<String>
```

`next_numbers` takes existing ADR-directory basenames already filtered to
`ADR-[0-9][0-9][0-9][0-9]*`, extracts each match's full leading digit run,
tracks the max, and formats `HIGHEST+1..HIGHEST+count` zero-padded to a
*minimum* of 4 digits (`format!("{:04}", n)` — Rust's `{:04}` is already
minimum-width, so `10000` renders untruncated exactly like bash's `%04d`).
Bash's own script needs an explicit `10#` base-10 prefix on its `$((...))`
arithmetic to avoid misparsing a leading-zero numeral like `0008` as octal
— this is a bash-specific hazard with **no Rust equivalent**:
`"0008".parse::<u32>()` already yields `8` with no special-casing. Do not
carry this guard over as defensive code in the Rust port; it would be
accidental complexity fixing a problem that was never present in Rust.

`read_status` replicates the line-by-line state machine exactly: the first
bare `---` opens the fence, the second closes it and the scan stops
immediately (a body `status:` line after the closing fence is never seen);
inside the fence every `status:`-prefixed line overwrites the captured value
(last one wins). Value extraction replicates bash's exact three-stage
pipeline, **in this order** — the stages are not equivalent to trim-then-
strip or to matched-pair stripping:
1. Strip the `status:` prefix and any inline leading whitespace.
2. Strip **one** leading quote char (`"` or `'`) if present, and **one**
   trailing quote char if present — both checked against the string as it
   stands after step 1, independently of each other (a leading quote can be
   stripped even if there's no matching trailing quote, and vice versa).
3. Strip trailing whitespace.

Because quote-stripping (step 2) runs *before* whitespace-trimming (step
3), a value like `status: "foo" ` (trailing space *inside* the fence, after
the closing quote) does **not** come out as `foo`: step 2 sees the string
still ending in a space (not a quote), so only the *leading* quote is
stripped (`foo" `); step 3 then trims the trailing space, leaving `foo"` —
a stray trailing quote survives. This is bash's actual behaviour, not a
bug to "fix" during the port; a unit test pins it (see Section 4 below).

Success requires both a closed fence and at least one `status:` line — an
empty status value still counts as success. Returns `None` uniformly for
"no fence", "unclosed fence", or "no
status key" — bash gives one combined failure message regardless of which,
so the caller (CLI layer) forms that message.

Add `pub mod adr;` to `cli/corpus/src/lib.rs`.

**File**: `cli/corpus/src/scan.rs`
**Changes**: New. Two small port traits, mirroring the existing
`corpus::metadata::Clock`/`corpus::work_item_id::IdScanner` idiom (a narrow
trait in `corpus`, a real filesystem-backed implementation in
`corpus-adapters`, injected at the composition root in `corpus-cli`'s
`main.rs` — the same shape as `vcs::classify::CheckoutProbe`/
`vcs::mode::ModeProbe`, which `cli/vcs-cli/src/detect.rs::run<P>` is already
generic over):

```rust
pub trait DirReader {
    /// Immediate entries' file names, or `None` if `dir` doesn't exist.
    fn list(&self, dir: &Path) -> Result<Option<Vec<String>>, kernel::Error>;
}

pub trait FileReader {
    /// A file's contents, or `None` if it doesn't exist or isn't a regular
    /// file.
    fn read(&self, path: &Path) -> Result<Option<String>, kernel::Error>;
}
```

`corpus::adr`'s two commands need exactly these: `DirReader` for `adr
next-number`'s decisions-directory listing, `FileReader` for `adr
read-status`'s file read. Phase 3 (`linkage extract`) reuses `FileReader`.
Phase 4 (`frontmatter validate`) adds a third port, `CorpusWalker`, in its
own section, since its recursive multi-root `*.md` walk has no equivalent
need in Phase 1 or 3.

Add `pub mod scan;` to `cli/corpus/src/lib.rs`.

**File**: `cli/corpus-adapters/src/fs.rs`
**Changes**: New. `pub struct RealFs;` implementing `corpus::scan::DirReader`
and `corpus::scan::FileReader` via direct `std::fs` calls (`read_dir`,
`read_to_string`), mapping "not found" to `Ok(None)` and any other I/O error
to `kernel::Error::Failed`. This is the concrete adapter composed once, at
`corpus-cli`'s `main.rs`, and injected into every command; test code injects
a hand-written stub instead (see Section 4).

#### 3. Command layer (decision logic, generic over injected ports)

**File**: `cli/corpus-cli/src/adr.rs`
**Changes**: New. Both functions are generic over the ports they need and
contain *only* decision logic — no `config_adapters::compose` call and no
direct `std::fs` access lives here; `main.rs` (Section 1) composes the real
config service and `corpus_adapters::fs::RealFs`, resolves `project_root`
and `paths.decisions` once, and passes already-resolved values in. This
mirrors `cli/vcs-cli/src/detect.rs::run<P: ModeProbe + CheckoutProbe>`:
the branching logic is unit-testable against a stub without spawning a
process or touching a real filesystem; only `main.rs` ever sees the
concrete adapter.

```rust
pub fn run_next_number<D: DirReader>(
    count: &str,
    decisions_dir: &Path,
    dir_reader: &D,
) -> Result<Outcome, kernel::Error>

pub fn run_read_status<F: FileReader>(
    file: Option<&Path>,
    file_reader: &F,
) -> Result<Outcome, kernel::Error>
```

`Outcome { stdout: String, stderr: String }` (defined once, shared across
all four noun groups' command modules) — `main.rs` prints `stdout`/`stderr`
verbatim and maps `Ok` to exit 0, `Err(kernel::Error::Failed(_))` to exit 1;
callers never `println!`/`eprintln!` directly, keeping the whole function
pure with respect to I/O once its ports are given.

`run_next_number`:
- Hand-validate `count` against `^[1-9][0-9]*$`; on failure, exact bash
  message (`Error: --count requires a positive integer, got '{count}'`) in
  `Outcome.stderr`, return `Err(kernel::Error::Failed(..))`. The regex
  accepts arbitrarily long digit strings, but `corpus::adr::next_numbers`
  takes `count: u32` — route a regex-valid value that overflows `u32` (e.g.
  `--count 5000000000`) through this same error path (same message, same
  exit code), not an unchecked `.parse().unwrap()`, so no input matching the
  documented regex can panic the binary.
- Call `dir_reader.list(decisions_dir)`. `Ok(None)` (directory doesn't
  exist): bash warning in `Outcome.stderr`, `0001..000N` in
  `Outcome.stdout`, return `Ok(..)` — a success path, not an error, matches
  bash's `exit 0`. `Ok(Some(entries))`: filter to `ADR-[0-9]{4,}*`
  basenames, call `corpus::adr::next_numbers`, join into `Outcome.stdout`.
  `Err(..)`: propagate.

`run_read_status`:
- If `file` is `None` (no argument given): bash's own no-args usage message
  in `Outcome.stderr`, return `Err(kernel::Error::Failed(..))` (exit 1) —
  the hand-validated substitute for clap's built-in required-argument
  error, which would otherwise exit 2 and break bash parity (see "What
  We're NOT Doing").
- Call `file_reader.read(file)`. `Ok(None)` (doesn't exist / not a regular
  file): the bash message in `Outcome.stderr`, return
  `Err(kernel::Error::Failed(..))`. `Ok(Some(content))`: call
  `corpus::adr::read_status`; on `None`, the two-line bash message (using
  the file's basename) in `Outcome.stderr` and fail; on `Some`, the value
  in `Outcome.stdout`.

**File**: `cli/corpus-cli/src/outcome.rs`
**Changes**: New. Defines the shared `Outcome` struct used by every command
module across all four phases — a flat, single-purpose module alongside
`adr.rs`/`metadata.rs`/`linkage.rs`/`frontmatter.rs`/`config.rs`/`cli.rs`/
`main.rs`, matching `cli/vcs-cli/src/`'s flat per-file layout (no
`commands/` subdirectory — that had no precedent in `vcs-cli`, which this
crate otherwise mirrors closely).

**File**: `cli/corpus-cli/src/config.rs`
**Changes**: New (created here in Phase 1 since it's needed from this
phase onward; Phase 3 extends it with `table_from_config`, per that
phase's own section). Defines the single composition-root helper every
dispatch branch calls, rather than each phase re-deriving the
`config_adapters::compose` call and its error mapping independently:

```rust
pub fn compose(cwd: &Path) -> Result<Composed, kernel::Error>
```

wrapping `config_adapters::compose(cwd, LegacyPolicy::Reject)` and mapping
any `ConfigError` to `kernel::Error::Failed` (exit 1) — matching bash's
uniform treatment of config/schema-resolution failures as ordinary
failures, not usage errors. `Composed` carries the `ConfigAccess` service
and the discovered project root. Every command that needs config
(`Adr::NextNumber`, `Linkage::Extract`, `Frontmatter::Validate`) calls this
one function; `Adr::ReadStatus` doesn't, since `adr read-status` needs
neither config nor a project root.

**File**: `cli/config/src/paths.rs`
**Changes**: New function alongside the existing `doc_type_dirs`:

```rust
pub fn resolve_with_fallback(
    config: &dyn ConfigAccess,
    path_key: &str,
) -> Result<String, ConfigError>
```

extracting the core `Key::parse` → `config.get` → `catalogue::default_for`-
on-absent fallback sequence that
`cli/launcher/src/config_command/core/paths.rs:60-78`'s `resolve` currently
inlines for itself (its own legacy-alias-warning and `--explain`/`--default`
CLI-flag layering stay in `launcher`, built on top of this). This is the
one piece of `paths.<key>` resolution genuinely shared between `accelerator
config path` and `accelerator corpus adr next-number`; extracting it means
a future change to the fallback rule updates both call sites instead of
needing two synchronised edits.

**File**: `cli/launcher/src/config_command/core/paths.rs`
**Changes**: `resolve` (lines 60-78) calls
`config::paths::resolve_with_fallback` for its fallback-value computation
instead of inlining it, preserving its own additional behaviour
(`legacy_alias_warning`, `--explain`, an explicit `--default` override)
unchanged. Existing `launcher` tests continue to assert the same outward
behaviour; only the internal implementation is deduplicated.

**File**: `cli/corpus-cli/src/main.rs` (Section 1's dispatcher, extended)
**Changes**: Beyond the thin `Cli`-parse-and-match dispatch already
described in Section 1: for the `Adr::NextNumber` branch, call
`config::compose(&cwd)` (above), resolve `paths.decisions` via
`config::paths::resolve_with_fallback` (above), joined against the composed
store's discovered root when relative, and construct a
`corpus_adapters::fs::RealFs`. Pass the resolved `decisions_dir`/`&RealFs`
(or file path, for `read-status`, which needs neither `compose` nor
`RealFs`'s `DirReader` half) into `run_next_number`/`run_read_status`, then
print the returned `Outcome` and map the result to `ExitCode`. Phase 3 and
Phase 4's dispatch branches call the same `config::compose` helper.

#### 4. Tests (TDD — written before the handlers above)

**File**: `cli/corpus/src/adr.rs` (inline `#[cfg(test)]`)
**Changes**: Unit tests for `next_numbers` (empty existing → `0001..`;
`ADR-9999` present → next is `10000`, untruncated; non-4-digit-leading names
ignored; `ADR-0008` parses as 8; `count = 0` at the domain-function boundary
— the CLI layer's regex validation currently prevents `0` from reaching
this function, but the function's own contract should be independently
pinned rather than relying solely on the caller's guard) and `read_status`
(fence-close breaks immediately so a body `status:` is never read; last
`status:` line inside the fence wins; an empty status value still succeeds;
missing fence and missing status key both return `None`; the three-stage
quote/whitespace pipeline's ordering — a double-quoted value with a
trailing space unwraps to a value with a stray trailing quote, per the
worked example above; a single-quoted value unwraps the same way; an
unquoted value is unaffected by the quote-stripping stage).

**File**: `cli/corpus-cli/src/adr.rs` (inline `#[cfg(test)]`)
**Changes**: New. Fast, in-process unit tests for `run_next_number`/
`run_read_status`'s own decision logic, against hand-written
`StubDirReader`/`StubFileReader` fakes (mirroring `vcs-cli/src/detect.rs`'s
`StubProbe` pattern) — no process spawn, no real filesystem. Covers the
branches the golden tests below cover only coarsely: `--count` validation
(including the `u32`-overflow case), the missing-decisions-directory
success-with-warning path, `dir_reader`/`file_reader` returning an error
(propagates as `Err`), and `file: None` (no argument). These are the primary
regression net for this module's branching; the black-box golden tests
below remain for true end-to-end/process-boundary coverage (real `main.rs`
composition, real exit codes) rather than branch enumeration.

**File**: `cli/corpus-cli/tests/adr_goldens.rs`
**Changes**: New. Spawns the compiled binary
(`env!("CARGO_BIN_EXE_accelerator-corpus")`) in a tempdir, one `#[test]` per
case transliterated from `skills/decisions/scripts/test-adr-scripts.sh`'s 21
cases (10 for next-number, 11 for read-status), asserting literal expected
stdout/stderr/exit code. The expected values are captured once, during
development, by running the current bash scripts against the same fixtures —
this is the "characterization against the golden baseline" AC1 requires; it
is not committed as a live bash-parity comparison (there is no bash left to
compare against once this phase deletes it). Also includes at least one test
writing a non-default `.accelerator/config.md` into the tempdir (a
configured `paths.decisions` value other than the catalogue default) and
asserting `adr next-number` honours it, plus one test exercising a legacy-
format config file and asserting the command fails closed via
`LegacyPolicy::Reject` rather than silently proceeding — this is the
riskiest genuinely-new wiring in this phase (per the research document) and
would otherwise have no direct test coverage of its own. Phase 3 and Phase 4
add the equivalent non-default-config coverage for their own config-
dependent commands (`linkage extract`'s and `frontmatter validate`'s
doc-type table resolution).

#### 5. Sub-binary registration

**File**: `tasks/shared/paths.py`
**Changes**: `DISPATCHED_SUBBINARIES: tuple[str, ...] = ("visualiser", "vcs",
"corpus")` (line 29).

**File**: `tasks/manifest.py`
**Changes**: Add `"corpus": CLI_DIR / "corpus-cli/Cargo.toml"` to
`_SUBBINARY_MANIFESTS` (line 52-57) — required because `cli/corpus/` is
already the domain crate.

**File**: `tasks/build.py`
**Changes**: Add `"accelerator-corpus"` to `_CLI_RELEASE_BINARIES` (line 35).

**File**: `.gitignore`
**Changes**: Confirm `bin/<token>-*` already covers `bin/corpus-*` generically
(per the checklist, it should — no new entry expected, but verify at
implementation time).

**File**: `tests/integration/tasks/test_github.py`
**Changes**:
- Line 507-510: rename/update
  `test_the_dispatched_registry_holds_visualiser_and_vcs` to assert
  `DISPATCHED_SUBBINARIES == ("visualiser", "vcs", "corpus")`.
- Line 252-320 (`_setup_release`): generalise the hardcoded two-token
  per-platform fixture/manifest construction into a loop over
  `DISPATCHED_SUBBINARIES` (or an explicit token→description mapping), adding
  `corpus`'s staged binary/minisig pair and manifest entry. This is the
  "first sibling" generalisation `tasks/README.md` point 1 calls out as
  expected.
- Line 338: `assert len(uploads) == 30` becomes a derived expression rather
  than a bumped literal, per the same checklist note. Per-platform, three
  launcher-related items are uploaded (debug archive + launcher binary +
  launcher `.minisig`, confirmed against `_setup_release`'s own fixture
  writes at lines 273-277) — **not** a flat constant of `4` as an earlier
  draft of this plan miscounted (a `4` would silently fail to reproduce
  even the current, pre-`corpus` value of 30). Two items are uploaded once,
  not per-platform (the manifest + its signature). Each dispatched
  sub-binary contributes two items per platform (the sub-binary + its
  `.minisig`). The correct derived expression is:
  `len(uploads) == len(_PLATFORMS) * 3 + 2 + len(DISPATCHED_SUBBINARIES) *
  len(_PLATFORMS) * 2`. Sanity-checked against the pre-`corpus` baseline:
  `4 * 3 + 2 + 2 * 4 * 2 = 12 + 2 + 16 = 30`, matching today's literal
  exactly; for the post-`corpus` case (`DISPATCHED_SUBBINARIES` now 3
  tokens): `4 * 3 + 2 + 3 * 4 * 2 = 12 + 2 + 24 = 38`.

**File**: `tests/unit/tasks/test_signing.py`, `tests/integration/tasks/test_release.py`
**Changes**: Confirm these already derive from `DISPATCHED_SUBBINARIES`
generically (they appeared to at research time) — no edit expected, verify.

**File**: `cli/deny.toml`
**Changes**: None expected — `corpus`, `corpus-adapters`, `config`,
`config-adapters`, `kernel`, `clap` are already vetted workspace
dependencies. Confirm `mise run deny:check` stays green.

**File**: `cli/pup.ron`
**Changes**: None — `corpus-cli` carries only command/composition-root code
per ADR-0053 (mirrors `vcs-cli`, which has no `pup.ron` entry either).

#### 6. Skill call-site rewrites

**Grant-scoping principle** (applies here and in every later phase's skill
rewrites): scope each skill's `allowed-tools` grant to what its own call
sites actually invoke — wildcard the noun (`corpus <noun> *`) when a skill
invokes, or plausibly will invoke, more than one verb under that noun;
exact-match a single `corpus <noun> <verb>` when a skill only ever calls
that one verb. This is a per-skill judgement call, not a blanket rule for
the phase — different skills legitimately land on different scoping.

**File**: `skills/decisions/create-adr/SKILL.md`
**Changes**: Invokes both `next-number` and `read-status` — replace the
`Bash(${CLAUDE_PLUGIN_ROOT}/skills/decisions/scripts/*)` allowed-tools rule
with `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator corpus adr *)` (wildcarded:
two verbs); rewrite the two `!`-preprocessor invocations to `accelerator
corpus adr next-number`/`accelerator corpus adr read-status`.

**File**: `skills/decisions/extract-adrs/SKILL.md`
**Changes**: Invokes only `next-number` — grant
`Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator corpus adr next-number)`
(exact-match: one verb) and rewrite its invocation accordingly.

**File**: `skills/decisions/review-adr/SKILL.md`
**Changes**: Invokes only `read-status` — grant
`Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator corpus adr read-status)`
(exact-match: one verb) and rewrite its invocation accordingly.

#### 7. Removal

**Files removed**: `skills/decisions/scripts/adr-next-number.sh`,
`skills/decisions/scripts/adr-read-status.sh`,
`skills/decisions/scripts/test-adr-scripts.sh`.

**File**: `tasks/test/integration.py`
**Changes**: `_EXPECTED_DECISIONS_SUITES` line 73: `1` → `0`.

### Success Criteria

#### Automated Verification:

- [x] `cargo test --manifest-path cli/Cargo.toml -p corpus -p corpus-cli`
      passes (package name is `accelerator-corpus`, not `corpus-cli`; `-p
      accelerator-corpus` used instead — 35 tests pass)
- [x] `mise run cli:check` passes (clippy/rustfmt across the workspace incl.
      the new crate)
- [x] `mise run lint:dispatch-coherence:check` passes
- [x] `mise run check` passes
- [x] `mise run test:integration:decisions` passes with the decisions floor
      at 0. `mise run test:integration:config` has one pre-existing,
      unrelated failure (`meta/reviews/plans/2026-08-06-0195-...-review-1.md`
      carries two `EMPTY-PLACEHOLDER` violations already present in the
      parent commit, before this phase's changes) — not caused by this phase
      and out of scope to fix here
- [x] `uv run pytest tests/integration/tasks/test_github.py
      tests/unit/tasks/shared/test_dispatch_coherence.py
      tests/unit/tasks/test_signing.py` passes

#### Manual Verification:

- [x] `accelerator corpus adr next-number --count 3` and `accelerator corpus
      adr read-status <file>` run correctly against a real ADR file in this
      repo
- [ ] `/accelerator:create-adr`, `/accelerator:extract-adrs`,
      `/accelerator:review-adr` invoked interactively and confirmed to call
      the new subcommand without a permission prompt for the old script path
      — not exercised (requires an interactive Claude Code session with the
      locally-built binary on the dispatch path)

---

## Phase 2: `metadata derive`

### Overview

Almost pure CLI glue: `corpus_adapters::metadata::{derive_at, render,
SystemClock}` already implements the full behaviour, differentially
parity-tested. This phase wires it into the CLI, migrates its (many) callers,
and closes a pre-existing `allowed-tools` gap the research flagged.

### Changes Required

#### 1. CLI + command layer

**File**: `cli/corpus-cli/src/cli.rs`
**Changes**: Add `Command::Metadata { #[command(subcommand)] action:
MetadataAction }`, `MetadataAction::Derive`.

**File**: `cli/corpus-adapters/src/metadata.rs`
**Changes**: Add a small public constructor to the existing `ClockError`
type: `#[must_use] pub fn new(reason: impl Into<String>) -> Self { Self(reason.into()) }`.
`ClockError`'s single field is private, so nothing outside this module can
currently construct one — including test code in `corpus-cli`, which needs
to synthesize a `ClockError` to test `run_derive`'s failure branch (below)
without forcing a real host tzdata failure. This is the only edit to
already-shipped `corpus-adapters` code in this phase.

**File**: `cli/corpus-cli/src/metadata.rs`
**Changes**: New.

```rust
pub fn run_derive(
    metadata: Result<ArtifactMetadata, ClockError>,
    format: FilenameTimestampFormat,
) -> Result<Outcome, kernel::Error>
```

`Ok(metadata)`: render via `corpus_adapters::metadata::render(&metadata,
format)`, the block in `Outcome.stdout`. `Err(clock_error)`:
`Err(kernel::Error::Failed(clock_error.to_string()))`. Unlike the other
three noun groups' command layers (Phases 1, 3, 4), this one is **not**
made generic over an injected port — the underlying `derive`/`render` pair
are themselves already generic over `corpus::metadata::Clock` (see Phase
2's Tests section), and `run_derive`'s only remaining decision (render on
success, map the error on failure) is already directly testable once given
an already-resolved `Result` rather than calling `derive_at` itself. This
mirrors the composition-root pattern used elsewhere: fallible, environment-
dependent construction (`SystemClock::try_new()`, wrapped by `derive_at`)
stays in `main.rs`; the command layer takes an already-resolved value.

**File**: `cli/corpus-cli/src/main.rs` (`Metadata` dispatch branch)
**Changes**: Call `corpus_adapters::metadata::derive_at(&cwd,
FilenameTimestampFormat::DateTimeUnderscored)`, pass its `Result` straight
into `run_derive`, print the returned `Outcome`/map the error to `ExitCode`
as elsewhere.

#### 2. Tests (TDD)

**File**: `cli/corpus-cli/src/metadata.rs` (inline `#[cfg(test)]`)
**Changes**: New. Two fast, in-process unit tests for `run_derive` — no
process spawn, no real filesystem, no host-environment dependency: `Ok(some
ArtifactMetadata)` renders the expected block into `Outcome.stdout`;
`Err(ClockError::new("test"))` (using the constructor added above) maps to
`Err(kernel::Error::Failed(..))` whose message contains the `ClockError`'s
text. This is AC1's required failure-path test for this subcommand — bash's
`artifact-derive-metadata.sh` has **no explicit `exit` statement anywhere**
(confirmed: only `set -euo pipefail`, no defined non-zero exit behaviour to
characterize), so there is no bash golden-baseline failure text to match
here, unlike the other four subcommands. The one concrete Rust-side failure
condition (`SystemClock::try_new()`'s `ClockError`, driven by real host
tzdata availability) can't be forced hermetically through the compiled
binary, so it's exercised at the unit level instead via the synthetic
`ClockError` rather than attempted in `metadata_goldens.rs` below.

**File**: `cli/corpus-cli/tests/metadata_goldens.rs`
**Changes**: New. Spawns the compiled binary inside a hermetically isolated
git and (when available) jj tempdir — reusing the same isolation pattern as
`scripts/test-metadata-helpers.sh`'s `run_helper_in_clean_repo` (unset
`GIT_DIR`/`JJ_CONFIG`, scoped `HOME`/`XDG_CONFIG_HOME`) — and asserts the
rendered block satisfies the same contract
`corpus-adapters/tests/metadata.rs::assert_satisfies_the_helper_contract`
already checks (label presence, ISO `+00:00`, no legacy labels). Covers the
success path only, per the note above; the failure path is covered at the
unit level.

**File**: `cli/corpus-adapters/tests/metadata.rs`
**Changes**: Delete `derive_at_agrees_with_the_live_metadata_helper` (its
oracle, `scripts/artifact-derive-metadata.sh`, is removed this phase). Keep
every `FakeClock`-based deterministic test — they exercise `derive`/`render`
directly and need no bash oracle.

**File**: `scripts/test-metadata-helpers.sh`
**Changes**: Drop `"$ROOT/scripts/artifact-derive-metadata.sh"` from the
`HELPERS` array (line 22), leaving `inventory-metadata.sh` and
`gap-metadata.sh` — both out of scope for 0195. File stays; no floor change.

#### 3. Skill call-site rewrites

All grants below are exact-match (`corpus metadata derive`, not `corpus
metadata *`), per Phase 1 §6's grant-scoping principle — `metadata` has
only one verb in this item's scope, so exact-match and noun-wildcard would
grant identically today; exact-match is chosen because none of these
skills has a stated need for a second `metadata` verb.

**Files**: `skills/decisions/create-adr/SKILL.md`,
`skills/decisions/extract-adrs/SKILL.md`,
`skills/research/conduct-spike/SKILL.md`,
`skills/research/research-issue/SKILL.md`,
`skills/research/research-codebase/SKILL.md`,
`skills/notes/create-note/SKILL.md`
**Changes**: Replace the `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/artifact-*)`
allowed-tools rule and its invocation with
`Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator corpus metadata derive)` /
`accelerator corpus metadata derive`.

**Files**: `skills/work/create-work-item/SKILL.md`,
`skills/work/review-work-item/SKILL.md`,
`skills/work/extract-work-items/SKILL.md`,
`skills/github/describe-pr/SKILL.md`, `skills/github/review-pr/SKILL.md`,
`skills/planning/create-plan/SKILL.md`,
`skills/planning/review-plan/SKILL.md`,
`skills/planning/validate-plan/SKILL.md`
**Changes**: These currently invoke the bash helper via prose with **no**
matching `allowed-tools` grant (a pre-existing gap, not introduced by 0195).
Add the `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator corpus metadata derive)`
grant and rewrite the invocation to the new subcommand, closing the gap as
part of AC2's "matching `allowed-tools`" requirement.

#### 4. Removal

**File removed**: `scripts/artifact-derive-metadata.sh`. No floor change
(`scripts/test-metadata-helpers.sh` stays, edited).

**Known, pre-existing, accepted divergence**: `SystemClock::try_new`'s own
doc comment (existing code, not modified by this plan) already states it
is a deliberate divergence from the bash helpers — which degrade silently
when the host's UTC offset can't be resolved (e.g. missing `tzdata`/`TZ`) —
whereas `SystemClock` fails hard on the same condition, and `run_derive`
maps that `ClockError` to `kernel::Error::Failed`. A host environment where
bash would still print output (with degraded/blank timezone data) makes
this CLI invocation fail instead. This is not introduced by this item and
is out of scope for AC1's byte-for-byte parity requirement — noted here so
it isn't mistaken for an oversight during review.

### Success Criteria

#### Automated Verification:

- [x] `cargo test --manifest-path cli/Cargo.toml -p corpus-adapters
      -p corpus-cli` passes (`-p accelerator-corpus` — 74 corpus-adapters
      tests incl. metadata.rs, 40 accelerator-corpus tests)
- [x] `mise run check` passes
- [x] `mise run test:integration:config` passes (floor unchanged at 18);
      `bash scripts/test-metadata-helpers.sh` also passes standalone with
      the two remaining design-domain helpers
- [x] `mise run lint:dispatch-coherence:check` passes — `corpus` stays bound
      via Phase 1's `adr next-number --fail-safe` eager line; none of this
      phase's rewrites are eager (`metadata derive`'s call sites are all
      mid-flow prose, same as bash), which is fine since only one binding is
      required per token

#### Manual Verification:

- [x] `accelerator corpus metadata derive` run directly against this repo —
      renders the ISO UTC timestamp, filename timestamp, revision, and
      repository name correctly
- [ ] `/accelerator:create-plan`, `/accelerator:create-adr` invoked
      interactively and the metadata block at the top of a freshly created
      artifact still renders correctly — not exercised (requires an
      interactive Claude Code session)

---

## Phase 3: `linkage extract`

### Overview

`corpus::linkage::parse_document` already implements the full extraction
pipeline, differentially parity-tested over 8 fixtures. This phase adds the
config wiring (doc-type table only — the bare-number scan needs no config,
confirmed already faithful in Key Discoveries above), the CLI/TSV rendering,
and migrates the sole non-skill caller.

### Changes Required

#### 1. Config → doc-type table adapter

**File**: `cli/corpus-cli/src/config.rs` (extends the module Phase 1 created
for the shared `compose` helper; lives in `corpus-cli`'s own composition
code, **not** `corpus-adapters` — `corpus-adapters` has no `config`
dependency today and is also consumed by `accelerator-visualiser`, which
composes its own config wiring in its own crate rather than the shared
adapters crate; see the precedent below)
**Changes**: New function

```rust
pub fn table_from_config(
    config: &dyn config::ConfigAccess,
    project_root: &Path,
) -> Result<Vec<(corpus::DocTypeKey, PathBuf)>, config::ConfigError>
```

mirroring `cli/visualiser/server/src/compose.rs:124-136`'s
`resolve_doc_paths` exactly: calls `config::paths::doc_type_dirs(config)`,
builds a `HashMap<String, PathBuf>` keyed by each `DocTypeDir::path_key`
with `project_root.join(&resolved.dir)` as the value, then delegates to the
**existing** `corpus_adapters::doc_type::table_from_paths` to produce the
`(DocTypeKey, PathBuf)` table — reusing the sibling helper that already
performs this exact mapping (keyed by `config_path_key()`) rather than
reimplementing it keyed by `linkage_type_name()`.

#### 2. CLI + command layer

**File**: `cli/corpus-cli/src/cli.rs`
**Changes**: Add `Command::Linkage { #[command(subcommand)] action:
LinkageAction }`,

```rust
LinkageAction::Extract {
    file: PathBuf,
    #[arg(long)]
    source_type: Option<String>,
}
```

a named `--source-type` flag rather than a second bare positional (bash's
`lp_parse_file <file> [source_type_override]` shape) — the "What We're NOT
Doing" scoping note already exempts CLI argument shape from bash parity, so
there's no reason to carry over a less-discoverable positional when a named
flag is clearer in `--help` and less error-prone to invoke correctly.

**File**: `cli/corpus-cli/src/linkage.rs`
**Changes**: New, generic over the injected `corpus::scan::FileReader`
(Phase 1) — no `config_adapters::compose` or direct `std::fs` call in this
module, matching Phase 1's port-injection shape:

```rust
pub fn run_extract<F: FileReader>(
    file: &Path,
    source_type_override: Option<&str>,
    table: &[(DocTypeKey, PathBuf)],
    file_reader: &F,
) -> Result<Outcome, kernel::Error>
```

`file_reader.read(file)`; `Ok(None)` (doesn't exist / not a regular file):
bash's message in `Outcome.stderr`, `Err(kernel::Error::Failed(..))`.
`Ok(Some(content))`: resolve `source_type` (given override, or
`corpus::linkage::type_from_path`, or `"unknown"` — mirrors bash), call
`corpus::linkage::parse_document`, join each record as
`{source_type}\t{key}\t{target_ref}\t{anchor}\t{band}` into `Outcome.stdout`.

**File**: `cli/corpus-cli/src/main.rs` (`Linkage` dispatch branch)
**Changes**: Call `config::compose(&cwd)` (Phase 1's shared helper) and
resolve the doc-type table via `table_from_config` (Section 1), construct
the `corpus_adapters::fs::RealFs` already introduced in Phase 1, and call
`run_extract` with the resolved table and `&RealFs`.

#### 3. Tests (TDD)

**File**: `cli/corpus-cli/src/config.rs` (inline `#[cfg(test)]`)
**Changes**: New. A focused unit test for `table_from_config` against a
hand-built fake `ConfigAccess`, including the case where a configured
doc-type name has no matching `DocTypeKey` — direct coverage for this
function rather than relying solely on the end-to-end golden test below.

**File**: `cli/corpus-cli/src/linkage.rs` (inline `#[cfg(test)]`)
**Changes**: New. Fast, in-process unit tests for `run_extract` against a
`StubFileReader` (reusing the same stub shape as Phase 1's
`StubFileReader`, or importing the same one if it's defined once and shared
— see Phase 1 Section 4) — covers the missing-file failure path and the
`source_type` resolution precedence (override, then path-inferred, then
`"unknown"`) without a process spawn.

**File**: `cli/corpus-cli/tests/linkage_goldens.rs`
**Changes**: New. Black-box test spawning the compiled binary over the same
8-fixture corpus already in `cli/corpus-adapters/tests/parity.rs`'s
`FIXTURES` constant, proving the CLI wiring (config composition → doc-type
table → `parse_document` → TSV rendering) end to end. Includes at least one
test writing a non-default `.accelerator/config.md` (a configured doc-type
directory other than the catalogue default) into the tempdir and asserting
`linkage extract` resolves types against the configured directory, not the
catalogue default — the same non-default-config coverage Phase 1 adds for
`adr next-number`.

**File**: `cli/corpus-adapters/tests/parity.rs`
**Changes**: Delete `linkage_extraction_matches_the_bash_parser` (its oracle,
`scripts/linkage-parser.sh`, is removed this phase) after using it once,
transiently, to confirm the new binary agrees. Keep
`doc_type_inference_matches_the_bash_matcher` and
`the_compiled_scan_regex_drives_slug_and_id_extraction` — both exercise
`doc-type-inference.sh`/`work-item-pattern.sh`, neither in scope for removal.

**File removed**: `scripts/test-linkage-parser.sh` (dedicated, 33 cases, no
by-name CI requirement; ground fully covered by `corpus::linkage`'s existing
unit tests plus the new black-box CLI test).

#### 4. Call-site rewrite

**File**: `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`
**Changes**: Line 660's `PARSER="$PLUGIN_ROOT/scripts/linkage-parser.sh"` and
its invocation are replaced with a call to the compiled `accelerator corpus
linkage extract` (resolved via `${ACCELERATOR_BIN:-$PLUGIN_ROOT/bin/accelerator}`,
matching the invocation-contract pattern from 0167). Not a `SKILL.md`, so no
`allowed-tools` change — this is a plain script-to-script call.

#### 5. Removal

**File removed**: `scripts/linkage-parser.sh`.

**File**: `tasks/test/integration.py`
**Changes**: `_EXPECTED_CONFIG_SUITES` line 43: `18` → `17`.

### Success Criteria

#### Automated Verification:

- [x] `cargo test --manifest-path cli/Cargo.toml -p corpus-cli` passes
      (`-p accelerator-corpus` — 52 tests total across corpus, corpus-adapters,
      accelerator-corpus)
- [x] `mise run check` passes
- [x] `mise run test:integration:config` passes with the config floor at 17
      (only the pre-existing, unrelated review-doc failure remains — see
      Phase 1's note)
- [ ] The 0007 migration's own test suite (`skills/config/migrate/scripts/test-migrate-0007.sh`)
      — not exercised: it is `INTERACTIVE: yes` and not wired into any
      `mise run test:*` task (confirmed via grep), and it hung waiting on
      stdin in this session rather than completing non-interactively

#### Manual Verification:

- [x] `accelerator corpus linkage extract meta/work/0195-....md` run locally
      against this repo's own work item — produces correctly-typed records
      (`work-item\tblocked_by\twork-item:0166\t...`, etc.)
- [ ] The 0007 migration script's linkage call site runs without error against
      a real corpus — not separately exercised beyond `bash -n` syntax
      validation and the test-migrate-0007.sh suite above

**Design correction found during implementation**: the plan's `table_from_config`
spec (joining `project_root` onto every doc-type directory, mirroring the
visualiser) silently breaks `corpus::linkage::path_roots`, which derives its
scan roots from each directory's leading path segment
(`dir.split('/').next()`) — empty for any absolute path. The golden tests
caught this (`every_fixture_extracts_correctly` returned zero records for
every fixture). Fixed by keeping the table project-root-relative, matching
`config::paths::doc_type_dirs`'s own return shape; `corpus::doc_type::infer`
tolerates either absolute or relative paths via its embedded-match fallback,
so nothing else needed to change. See `table_from_config`'s doc comment.
Phase 4's `CorpusWalker` will need to join the project root itself, at the
point it actually walks the filesystem — `table_from_config` must stay
relative for Phase 3's callers.

**Also fixed** (surfaced by the full `test:integration:config` run, not caught
in Phase 2's own narrower verification): Phase 2's metadata skill rewrites
dropped `skills/research/conduct-spike/SKILL.md` out of
`test-skill-frontmatter-conformance.sh`'s discovery regex (it matched solely
via its `artifact-derive-metadata.sh` reference, which Phase 2 rewrote).
Removed it from that suite's `EXCLUDED` list and updated the discovery count
18 → 17.

---

## Phase 4: `frontmatter validate`

### Overview

The largest phase — genuinely new domain logic. Ports the 16-violation-code
structural/referential validator (plus one new 17th code, `DuplicateId`,
closing a referential-integrity blind spot bash's associative-array index
had no way to detect — see Section 3's `Index` design), its 13-row per-type
schema table, and the cross-cutting emission rules into
`corpus`/`corpus-adapters`, then retires the
required-by-name bash CI gate in favour of an unconditional Rust test suite.

### Changes Required

#### 1. Domain module — schema table and pure per-file validation

**File**: `cli/corpus/src/frontmatter_validation/mod.rs`
**Changes**: New. Pure logic (no regex, no filesystem — mirrors
`corpus::linkage`'s dependency discipline), split into cohesive sub-modules
rather than one large file — more surface than `corpus::linkage` (a
13-row schema table, cross-cutting emission constants, a 16-variant
`Violation` enum, and a shape checker are each their own concern):

- `schema.rs`: `SCHEMA: [SchemaRow; 13]` const mirroring
  `scripts/templates-schema.tsv` row for row (type, `code_state_anchored`,
  extras, `status_vocab`, `forbidden_own_id_key`, `typed_linkage_keys`),
  parity-tested against the TSV the same way `corpus::linkage::TYPE_PAIRS`
  mirrors `linkage-type-pairs.tsv`; plus the cross-cutting constants
  mirroring `scripts/frontmatter-emission-rules.sh`: `FM_BASE_FIELDS` (line
  31), `FM_PROVENANCE_FIELDS`/`FM_FORBIDDEN_PROVENANCE_FIELDS` (34-35), the
  14-type source-type vocabulary (`FM_SOURCE_TYPE_RE`, line 41),
  `FM_OPTIONAL_EXTRAS` (line 74), the obsolete-legacy-key list
  (`ticket`/`ticket_id`, `scripts/validate-corpus-frontmatter.sh:75`).
- `violation.rs`: the `Violation` enum with the 16 bash-mirrored variants
  (`NoFence`, `InvalidType`, `UnquotedId`, `BadSchemaVersion`,
  `BadTimestamp`, `BadStatus`, `MissingProvenance`,
  `ProvenanceOnNonAnchored`, `ForbiddenProvenance`, `ForbiddenOwnId`,
  `ObsoleteLegacyKey`, `MissingExtra`, `EmptyPlaceholder`,
  `BadLinkageShape`, `DanglingRef`, plus the file-level `NoFence`/
  `InvalidType` already listed) plus the one new 17th variant,
  `DuplicateId` (Section 3 below), whose `Display`
  renders the exact `<CODE> — <message>` text (em dash `—`, U+2014, not a
  hyphen — byte-significant per the bash script's own `violation()`
  formatter). Include a unit test asserting the separator is literally
  `'\u{2014}'` as its own character-level check, independent of any
  golden-string comparison elsewhere — a hand-transcribed golden fixture
  could silently normalise U+2014 to a plain hyphen (visually near-
  identical in most editors) without a string-equality test catching it.
- `shape.rs`: a hand-rolled typed-linkage shape checker (no `regex`,
  matching `corpus::linkage`'s discipline) validating the double-quoted-only
  rule: `^"(known-type|pr):[A-Za-z0-9.-]+"$` against the full 14-type
  vocabulary — confirmed in Key Discoveries as *not* the same grammar as
  `corpus::typed_ref::parse_typed_ref`.
- `mod.rs`: the public facade — `pub fn validate_file(mapping: &Mapping,
  resolved_type: Option<&str>) -> Vec<Violation>` (the per-file structural
  checks: base fields, id quoting, `schema_version`, timestamps, status
  vocabulary, provenance bundle presence/absence by `code_state_anchored`,
  forbidden own-id key, obsolete legacy keys, required extras,
  omit-when-empty, linkage shape — deliberately preserving the bash quirks
  the research flagged: the `UNQUOTED-ID`/`EMPTY-PLACEHOLDER` gap on a
  truly-empty `id:` value, and the double-quote-only `BAD-LINKAGE-SHAPE`
  asymmetry) and `pub fn dangling_refs(mapping: &Mapping, index: &Index) ->
  Vec<Violation>` (the referential-integrity check, `pr:` exempted, given an
  externally built `type:id` index built by the adapter layer, not here) —
  re-exporting `Violation`, `SchemaRow`, and whatever `schema.rs`/
  `shape.rs` need to expose, so callers `use
  corpus::frontmatter_validation::{validate_file, dangling_refs,
  Violation}` exactly as if it were one file.

Add `pub mod frontmatter_validation;` to `cli/corpus/src/lib.rs`.

#### 2. Domain-crate walk port

**File**: `cli/corpus/src/scan.rs` (extends Phase 1's `DirReader`/`FileReader`)
**Changes**:

```rust
pub trait CorpusWalker {
    /// Recursively finds every `*.md` file under each of `roots`.
    fn walk_markdown(&self, roots: &[PathBuf]) -> Result<Vec<PathBuf>, kernel::Error>;
}
```

`corpus_adapters::fs::RealFs` (Phase 1) also implements this. Every
filesystem-touching function in Section 3 below is generic over `W:
CorpusWalker + FileReader` (or takes `&dyn` equivalents) rather than calling
`std::fs`/`corpus_adapters::document::parse`'s own read step directly — the
same shape as `vcs::classify`/`vcs::mode`'s probes, extended one layer
further than Phases 1/3 needed since this phase's walk is recursive and
multi-root rather than a single `read_dir`/`read_to_string`.

#### 3. Adapter module — whole-corpus walk and index

**File**: `cli/corpus-adapters/src/frontmatter_validation.rs`
**Changes**: New.

Structural and referential-integrity checking are no longer two mutually
exclusive CLI "modes" — every invocation runs both check categories by
default over a file selection that defaults to the whole corpus (see
Section 4). This section covers the always-full-corpus index and the
per-file check orchestration that section 4's command layer subsets and
filters. Every function below is generic over `W: CorpusWalker +
FileReader`, taking the port as a parameter rather than calling `std::fs`
internally — `main.rs` supplies the real `RealFs`; tests supply a stub (see
Section 5).

- `pub fn corpus_files<W: CorpusWalker>(table: &[(DocTypeKey, PathBuf)],
  walker: &W) -> Result<Vec<PathBuf>, kernel::Error>` — calls
  `walker.walk_markdown(...)` over every directory in the doc-type table
  (i.e. the whole configured corpus), returning every in-scope `*.md` file.
  This is the single walk primitive; both `Index`-building and the command
  layer's "no `--dir`/`--file` given" default target set use it.
- `pub struct Index(HashMap<(String, String), Vec<PathBuf>>)` or similar,
  built by calling `corpus_files` and, for each file, parsing via
  `corpus_adapters::document::parse` and **indexing only files whose
  `FrontmatterState` is `Parsed`** — a file that is `Absent` (no fence) or
  `Malformed` (fence present but unparseable content, e.g. a tagged YAML
  node or a non-mapping root) is excluded from the index entirely, not
  given a type/id via fallback. This is a deliberate correctness rule, not
  a parity port: bash's own `has_fence` gate (`scripts/
  validate-corpus-frontmatter.sh:129-134`) only checks that the *first
  line* is `---`, so a fenced-but-unparseable file would still pass bash's
  gate and get a fallback-derived index entry — but a file's type/id should
  only be trusted as a reference target when the file itself has
  successfully declared them, and `Malformed` is exactly the same
  epistemic situation as `Absent` (no reliable, self-declared type/id
  exists) — not the situation a `Parsed` file that merely omits `type:`/
  `id:` keys is in, where path-inference/filename-stem fallback augments
  an otherwise-valid document. This also keeps the `Index` consistent with
  `validate_path`'s own treatment of `Malformed` as `NoFence` (below): a
  file already reported as broken when validated directly should not
  simultaneously be usable as a valid reference target for other files.
  For a `Parsed` file: type from a parsed `type:` field, else path-
  inferred; id from `id:`/`work_item_id:`/`adr_id:`, else the filename
  stem — **appending** to the `(type, id)` key's `Vec` rather than
  overwriting, so a genuine collision (two files resolving to the same
  type/id) is visible rather than silently losing one entry, unlike the
  bash predecessor's associative-array `build_index`, which this
  deliberately improves on now that the index is being rebuilt in a
  language where tracking collisions is cheap. **Always built from the
  whole corpus**, regardless of which files the command layer selects for
  validation — referential-integrity checking needs full-corpus context
  even when only a handful of files are being validated/reported.
- A new `Violation::DuplicateId` variant (17th — the one addition beyond
  the 16 bash-mirrored codes, since bash's own associative-array index has
  no way to detect this) added to `violation.rs`, with its own `<CODE> —
  <message>` `Display` form. `validate_targets` (below) checks each
  target file's own resolved `(type, id)` against `Index`; if that key's
  `Vec` has more than one entry, emit `DuplicateId` — gated the same way as
  `dangling_refs`: only when `checks.references` is enabled and the file
  didn't already fail `NoFence`/`InvalidType`, since it's the same
  referential-integrity category, not a structural check.
- `pub fn validate_path<F: FileReader>(path: &Path, table: &[(DocTypeKey,
  PathBuf)], file_reader: &F) -> Result<Vec<Violation>, kernel::Error>` —
  calls `file_reader.read(path)`. `Ok(None)` (the file doesn't exist or
  isn't a regular file — the same condition every other command layer
  function branches on explicitly) is treated identically to
  `FrontmatterState::Absent` below, i.e. reported as `Violation::NoFence`,
  **not** a distinct "file not found" error and not an early return —
  verified against bash's actual behaviour (`has_fence`,
  `scripts/validate-corpus-frontmatter.sh:129-134`, does `read -r first
  &lt;"$file" 2&gt;/dev/null || return 1`: for a nonexistent file the redirect
  fails, `has_fence` returns false, and `validate_file` reports `NO-FENCE —
  no frontmatter fence at file head` — bash treats "doesn't exist"
  identically to "exists but has no fence," never surfacing a distinct
  missing-file message). `Ok(Some(content))` parses via
  `corpus_adapters::document::parse`, which returns one of three
  `FrontmatterState`s: `Absent` (no fence found) and `Malformed` (a fence
  was found but the content doesn't parse to a mapping — e.g. tagged YAML
  nodes, which the untyped parser fails closed on by design, or a sequence/
  scalar root) both map to `Violation::NoFence` — bash's naive line-based
  scanner has no equivalent "fence found but unparseable" concept, so
  `Malformed` is folded into the same violation as `Absent` rather than
  given a new code, since in both cases there's no `Mapping` to validate
  further. `Parsed(mapping)` resolves its type **strictly from the
  frontmatter `type:` field — no path-inference fallback**, matching bash's
  `validate_file` exactly (a file missing `type:` fails `INVALID-TYPE`
  regardless of which directory it lives in; see the `boundary-untyped.md`
  bash fixture). Path-inference is reserved for `Index`-building only
  (above), which resolves referential-integrity *targets*, not a file's own
  declared type. Calls `corpus::frontmatter_validation::validate_file`.
- `pub struct Checks { pub structure: bool, pub references: bool }` — which
  check categories the command layer has enabled (see Section 4's
  `--checks` flag).
- `pub enum TargetOutcome { Violations(Vec<Violation>), Skipped }` or
  similar — `validate_targets` (below) reports each ineligible-for-
  references file distinctly rather than folding it into silent success.
- `pub fn validate_targets<F: FileReader>(files: &[PathBuf], table:
  &[(DocTypeKey, PathBuf)], index: &Index, checks: &Checks, file_reader: &F)
  -> Result<Vec<(String, TargetOutcome)>, kernel::Error>` — for each file:
  always internally runs `validate_path` (needed regardless of
  `checks.structure` to determine short-circuit eligibility below), but only
  *emits* its violations when `checks.structure` is set. Runs
  `dangling_refs` **and** the `DuplicateId` check (both referential-
  integrity checks, driven by `Index`) for that file, emitting their
  violations, only when `checks.references` is set **and** `validate_path`
  returned no `NoFence`/`InvalidType` violation for it — matching bash's
  early-return-on-`NO-FENCE`/`INVALID-TYPE` exactly (the linkage-shape/
  dangling-ref loop is never reached for a file that failed those two
  checks). **When a file fails `NoFence`/`InvalidType` and `checks.structure`
  is disabled** (so its structural violation isn't emitted) **and
  `checks.references` is enabled** (so the caller is relying on this run
  for referential coverage): emit `TargetOutcome::Skipped` for that file
  rather than nothing — this is the one case where the file's eligibility
  for the enabled check category couldn't be determined without the
  disabled one. `Skipped` is **not** a violation (doesn't affect the exit
  code) but **is** distinct, visible output, so a `--checks references`-
  only run can't be mistaken for "0 violations, fully clean" when a file
  was actually never eligible for the check that ran — closing exactly the
  false-clean-result risk this design would otherwise carry, since
  `--checks` (unlike bash, which never offered a references-only mode) is
  a new, user-selectable way to skip structural checking.

#### 4. CLI + command layer

**File**: `cli/corpus-cli/src/cli.rs`
**Changes**: Add `Command::Frontmatter { #[command(subcommand)] action:
FrontmatterAction }`,

```rust
FrontmatterAction::Validate {
    #[arg(long)]
    dir: Vec<PathBuf>,
    #[arg(long)]
    file: Vec<PathBuf>,
    #[arg(long, value_delimiter = ',', default_value = "structure,references")]
    checks: Vec<CheckKind>,
}
```

with `CheckKind` a `clap::ValueEnum` (`Structure`, `References`) — an
extensible list rather than a pair of boolean flags, so a future check
category is a new variant, not a new flag. `checks` deduplicates repeats;
an unrecognised value is rejected by clap's own `ValueEnum` parsing (new
CLI surface with no bash predecessor, so no byte-for-byte exit-code
obligation applies here — see the "What We're NOT Doing" scoping note).

**File**: `cli/corpus-cli/src/frontmatter.rs`
**Changes**: New, generic over `W: CorpusWalker + FileReader` — no
`config_adapters::compose` or direct `std::fs` call in this module, matching
Phases 1 and 3's port-injection shape:

```rust
pub fn run_validate<W: CorpusWalker + FileReader>(
    dirs: &[PathBuf],
    files: &[PathBuf],
    checks: &Checks,
    table: &[(DocTypeKey, PathBuf)],
    walker_and_reader: &W,
) -> Result<Outcome, kernel::Error>
```

Always compute `corpus_files(&table, walker_and_reader)` plus the `Index`
built from it (full corpus, unconditionally). Resolve the target file set:
- If both `dirs` and `files` are empty: target = every file from
  `corpus_files` (whole-corpus default).
- Otherwise: target = the union of every in-scope file found by walking
  each directory in `dirs` (via `walker_and_reader.walk_markdown`), plus
  every path in `files`, deduplicated.

Call `validate_targets(&target_files, &table, &index, checks,
walker_and_reader)`. Join one `{file}: {CODE} — {message}` line per
violation, and one `{file}: SKIPPED — structural checks disabled and this
file is ineligible for the referential-integrity check that ran (would
fail NO-FENCE/INVALID-TYPE)` line per `TargetOutcome::Skipped` entry, into
`Outcome.stderr`. `Skipped` entries don't affect the exit code: return
`Err(kernel::Error::Failed(..))` if any *violations* exist (exit 1),
`Ok(..)` otherwise (exit 0) — including when the only output is `Skipped`
lines, since the user explicitly chose to disable structural checking and
shouldn't have the run fail because of it, just be told what wasn't
checked.

**File**: `cli/corpus-cli/src/main.rs` (`Frontmatter` dispatch branch)
**Changes**: Call `config::compose(&cwd)` (Phase 1's shared helper) and
resolve the doc-type table via `table_from_config`, resolve `checks:
Vec<CheckKind>` into `Checks { structure, references }` (a category present
anywhere in the list enables it — `default_value` already supplies both
when the flag is omitted entirely), construct the
`corpus_adapters::fs::RealFs` already introduced in Phase 1, and call
`run_validate` with the resolved table, `Checks`, and `&RealFs`.

#### 5. Tests (TDD)

**File**: `cli/corpus/src/frontmatter_validation/{schema,violation,shape}.rs`
(inline `#[cfg(test)]` in each)
**Changes**: Unit tests transliterated from
`scripts/test-validate-corpus-frontmatter.sh`'s 49 cases — not merely "one
test per violation code" as a floor, since a meaningful fraction of the
original 49 exercise combinations (a single file triggering multiple
violations, and the order violations are reported in) and per-type
schema-row edge cases across all 13 rows, neither of which a strict
one-per-code mapping would cover. Include the preserved quirks above, the
schema-table parity assertion against `scripts/templates-schema.tsv`
(`schema.rs`), and the em-dash character-level assertion (`violation.rs`,
noted in Section 1 above). Where a bash case is redundant with another
(exercises the identical code path with different but equivalent input),
it's fine to consolidate — but the *coverage* (combinations, per-type rows,
ordering) must carry forward before
`scripts/test-validate-corpus-frontmatter.sh` is deleted in Section 7
below, not just the violation-code names (16 bash-mirrored plus the new
`DuplicateId`).

**File**: `cli/corpus-adapters/tests/frontmatter_validation.rs`
**Changes**: New. Against a hand-written `StubWalker`/`StubFileReader`
(no real filesystem) unless noted: `DanglingRef` fires for a file selected
via `files` when the index (always full-corpus) contains a dangling target;
`pr:` exempted; own-index id-fallback chain (`id:` → `work_item_id:`/
`adr_id:` → filename stem); a file failing `NoFence`/`InvalidType` never
reaches `dangling_refs` regardless of which `Checks` are enabled; a file
whose `FileReader` content is unparseable YAML (a tagged node, or a
sequence/scalar root) resolves to `FrontmatterState::Malformed` and is
reported as `NoFence`, same as a file with no fence at all; a `Malformed`
file is **excluded from the `Index`** — a reference targeting it is
reported as `DanglingRef`, not silently resolved via path-inferred type/
filename-stem fallback, distinguishing this from a `Parsed` file that
merely omits `type:`/`id:` keys (which *does* get indexed via that same
fallback chain); `DuplicateId`
fires when two corpus fixtures resolve to the same `(type, id)` (via
distinct `id:` values, and separately via the filename-stem fallback for
two files sharing a stem in different directories), and does **not** fire
when a file's own `(type, id)` key has exactly one entry in the `Index`;
`validate_targets` returns `TargetOutcome::Skipped` (not silently nothing)
for a file that fails `NoFence`/`InvalidType` under `checks == { structure:
false, references: true }`.

**File**: `cli/corpus-cli/src/frontmatter.rs` (inline
`#[cfg(test)]`)
**Changes**: New. Fast, in-process unit tests for `run_validate`'s target-
set resolution against a `StubWalker`/`StubFileReader` — the branches the
implicit-mode design (finding 5, since resolved) risked leaving untested:
empty `dirs`+`files` → whole corpus; `dirs`-only; `files`-only; both
together with overlapping entries → deduplicated union; each `--checks`
combination's effect on which violations `Outcome.stderr` contains for a
fixed fixture set. No process spawn, no real filesystem — the black-box
golden tests below remain for true end-to-end coverage.

**File**: `cli/corpus-cli/tests/frontmatter_goldens.rs`
**Changes**: New. Black-box CLI tests covering: default (no `--dir`/`--file`,
whole corpus, both checks), `--dir`-only, `--file`-only, `--dir`+`--file`
together, `--checks structure`-only, `--checks references`-only, and both
`--checks` values together given `structure,references` explicitly — at
least one success and one failure path per AC1, asserting exit codes 0/1.
Includes a case asserting the `Skipped` design specifically: `--checks
references` against a fixture with no frontmatter fence exits 0 (not a
violation) but its stderr contains a `SKIPPED` line for that file — proving
the run is distinguishable from a genuinely clean corpus, not just that it
doesn't crash. Includes a case for `--file` naming a nonexistent path,
asserting it reports `NO-FENCE` (matching bash's verified behaviour) and
exit 1 — not a panic and not a distinct "file not found" message.

Includes one **unconditional** (not `bash-parity`-gated) test running
`validate` with no arguments against this repository's own `meta/` tree and
asserting zero violations — this becomes the fail-closed migration-
completion signal that replaces `scripts/test-validate-corpus-frontmatter.sh`'s
`_REQUIRED_CONFIG_SUITES` role (a `cargo test` failure already fails CI
unconditionally; no bash-style "required suite" registry is needed on the
Rust side). Also includes the same non-default-config coverage Phase 1 and
Phase 3 add for their own commands: a test writing a non-default
`.accelerator/config.md` (a configured doc-type directory other than the
catalogue default) into the tempdir and asserting `frontmatter validate`
resolves its doc-type table from the configured value.

**File**: `scripts/test-validate-corpus-frontmatter.sh`
**Changes**: Used once, transiently, during development to confirm the new
binary agrees with current bash output across its 49 cases, then deleted —
per the user's confirmed direction that all bash test suites this item
touches are retired, not kept-and-repointed.

#### 6. Call-site rewrite

**File**: `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`
**Changes**: Line 611's `VALIDATOR="$PLUGIN_ROOT/scripts/validate-corpus-frontmatter.sh"`
is replaced with the compiled `accelerator corpus frontmatter validate` call.
Two distinct edits are needed — inside the two `self_validate_*` function
bodies, and separately at their two call sites — since this script's
existing structure separates "how the validator is invoked" from "how a
non-zero exit is handled," and only the first of those is addressed by
simply swapping the bash invocation for the compiled one:

- **Inside `self_validate_structural`** (function body, currently line 622:
  `bash "$VALIDATOR" "${files[@]}"`, bash's file-list mode): becomes a
  **single** `accelerator corpus frontmatter validate` invocation carrying
  one `--file "$f"` flag per entry in `files` (i.e. build the flag list,
  then invoke once) — **not** a shell loop spawning the binary once per
  file. The script runs under `set -euo pipefail`; a per-file loop would
  make `set -e` abort at the *first* file with a violation, silently
  skipping validation of every subsequent file — a diagnostic-completeness
  regression on an operation that has already mutated the whole corpus.
  Bash's original single `bash "$VALIDATOR" "${files[@]}"` call already
  validates every file in one pass and reports all violations together;
  the rewrite must preserve that, matching the CLI's own `Vec<PathBuf>`
  design for repeated `--file` flags.
- **Inside `self_validate_referential`** (function body, currently line
  628: `bash "$VALIDATOR" "$META_ABS"`, bash's whole-corpus mode): becomes
  `--dir "$META_ABS"`.
- **At both call sites** (`self_validate_structural`'s call at line 834,
  inside the `{ ... } >&2` orchestration block, and
  `self_validate_referential`'s call immediately after `harness_run`):
  wrap the call in the same `if ! self_validate_structural; then log_warn
  "..."; exit 1; fi` guard pattern every other failure branch in this
  script already uses (e.g. the `DOC_TYPE_TABLE_OK`/`precondition_prepass`/
  `REFUSE_COUNT` guards immediately above). Currently both `self_validate_*`
  calls are bare statements — a non-zero exit is caught only by `set -e`'s
  implicit abort, with no `log_warn` message and no "revert meta/ via your
  VCS, then re-run" guidance, unlike every sibling guard in the same file.
  This gap becomes materially more likely to be hit after this migration:
  `self_validate_structural` now runs referential-integrity checks by
  default (see below) and `self_validate_referential` now also runs the
  new `DuplicateId` check, both potentially surfacing real, pre-existing
  corpus issues for the first time, at a point where files have already
  been rewritten on disk.

Both call sites now get full structural **and** referential-integrity
checking by default (the new CLI's default `--checks structure,references`),
whereas bash's file-list mode never ran referential-integrity checks, and
`self_validate_referential`'s whole-corpus pass now also runs the new
`DuplicateId` check, which has no bash predecessor at all. These are
deliberate, positive behavioural changes — flag them explicitly in the PR
description as intentional widenings, not silent migration side effects,
and exercise the newly-reachable failure path deliberately (not just the
happy path) in this phase's manual verification against a real corpus,
given both call sites are now materially more likely to fail than before.

#### 7. Removal

**Files removed**: `scripts/validate-corpus-frontmatter.sh`,
`scripts/test-validate-corpus-frontmatter.sh`.

**File**: `tasks/test/integration.py`
**Changes**: `_REQUIRED_CONFIG_SUITES` (line 63-66): drop
`"scripts/test-validate-corpus-frontmatter.sh"`, keeping
`"scripts/test-skill-frontmatter-conformance.sh"`. `_EXPECTED_CONFIG_SUITES`
(line 43): `17` → `16` (continuing from Phase 3's decrement).

### Success Criteria

#### Automated Verification:

- [ ] `cargo test --manifest-path cli/Cargo.toml -p corpus -p corpus-adapters
      -p corpus-cli` passes, including the whole-corpus self-check against
      this repo's own `meta/` tree
- [ ] `mise run check` passes
- [ ] `mise run test:integration:config` passes with the config floor at 16
      and `_REQUIRED_CONFIG_SUITES` down to the one remaining entry
- [ ] `corpus-cli` is confirmed **not** present in any `--exclude` list or
      feature-gate that would skip it in the default `cargo test`/`mise run
      cli:check` invocation — the whole-corpus self-check is the sole
      fail-closed replacement for the retired `_REQUIRED_CONFIG_SUITES`
      entry and must actually run unconditionally

#### Manual Verification:

- [ ] `accelerator corpus frontmatter validate` (no arguments, whole-corpus
      default) run locally against the real corpus and its violation
      count/shape compared against what bash used to report for the same
      tree; `--dir meta/` and `--checks structure` also exercised manually
- [ ] The 0007 migration script's frontmatter call site runs without error
      against a real corpus
- [ ] The 0007 migration script's failure path is exercised deliberately:
      run it against a scratch corpus seeded with a dangling reference (or a
      duplicate id), and confirm `self_validate_structural`/
      `self_validate_referential`'s new guard prints the `log_warn`/VCS-revert
      message and exits 1, matching every other failure branch in the
      script — not a bare `set -e` abort with no guidance

---

## Phase 5: Documentation and final acceptance sweep

### Overview

User-facing documentation for the whole `accelerator-corpus` surface (written
once, now that all four noun groups exist), plus a closing repo-wide sweep
confirming AC2's closed set is fully migrated.

### Changes Required

#### 1. Documentation

**File**: `docs-site/src/content/docs/corpus.md` (new)
**Changes**: New page modelled on `docs-site/src/content/docs/visualiser.md`'s
structure: covers all four noun groups (`adr`, `metadata`, `linkage`,
`frontmatter`), documents the `ACCELERATOR_CORPUS_BIN` local-dev override row.

**File**: `docs-site/astro.config.mjs`
**Changes**: Add the new page to the sidebar / prev-next chain.

**File**: `README.md`
**Changes**: Add `accelerator-corpus` to the Concepts list under
`## Documentation`.

#### 2. Closing AC2 sweep

Repo-wide search for any remaining *executable* invocation (`Bash(...)`
calls, `allowed-tools` entries, shell `source`/exec sites — not bare
documentation/ADR mentions, which AC2 explicitly excludes) of
`adr-next-number.sh`, `adr-read-status.sh`, `artifact-derive-metadata.sh`,
`validate-corpus-frontmatter.sh`, or `linkage-parser.sh`. Confirm zero
remain.

#### 3. Final checklist closure

Confirm `mise run deny:check` and the `cli:check` cargo-pup lane are both
green with the accumulated `corpus-cli` crate. Update work-item:0195's
status; note that the floor decrements applied across Phases 1/3/4 are the
concrete instances feeding work-item:0174's lockstep requirement.

### Success Criteria

#### Automated Verification:

- [ ] `mise run` (the bare default task, full local CI mirror) passes end to
      end
- [ ] A repo-wide grep for the five removed script names' executable
      invocation forms returns nothing outside documentation/ADR prose

#### Manual Verification:

- [ ] All five subcommands (`adr next-number`, `adr read-status`, `metadata
      derive`, `linkage extract`, `frontmatter validate`) run back to back
      in a real checkout without error
- [ ] The new `docs-site` page renders correctly via `mise run docs:build`

## Testing Strategy

### Unit Tests

- Pure domain logic in `corpus` (adr numbering/status, frontmatter schema
  validation, linkage extraction — the latter already existing) is tested
  in-crate with no I/O, no bash oracle.
- Edge cases specifically preserved from bash: ADR number-overflow
  untruncated width, octal-safe zero-padded parsing, frontmatter-fence
  early-break-on-second-fence, empty-status-still-succeeds,
  double-quote-only linkage shape, the hardcoded 4-digit bare-number scan.

### Integration Tests

- Per-phase black-box tests spawning the compiled `accelerator-corpus`
  binary (`cli/corpus-cli/tests/*_goldens.rs`), following the
  `cli/vcs-cli/tests/detect_goldens.rs` precedent. **Unlike** the existing
  `bash-parity`-feature-gated tests this codebase already has (e.g.
  `vcs-cli/tests/detect_goldens.rs`'s own live-bash comparisons,
  `corpus-adapters/tests/parity.rs`), these `*_goldens.rs` files compare
  against pre-captured expected values, not a live bash process — they run
  **unconditionally**, in the default `cargo test` invocation with no
  `#![cfg(feature = "bash-parity")]` gate. State this explicitly at the top
  of each `*_goldens.rs` file (a comment is fine) given the strong local
  convention of feature-gating this style of binary-spawning test; the
  risk is the gate being copy-pasted by habit and silently disabling the
  primary regression-protection layer for these four newly-ported
  subcommands.
- One unconditional whole-corpus self-check (`frontmatter validate` against
  this repo's own `meta/`) as the permanent fail-closed gate replacing the
  retired bash requirement — confirm at implementation time that
  `corpus-cli` is not excluded from the default `cargo test`/`mise run
  cli:check` invocation the way `accelerator-visualiser` is (`tasks/test/
  cli.py`'s `--exclude accelerator-visualiser`), since an accidental future
  exclusion would silently reintroduce exactly the failure mode the retired
  `_REQUIRED_CONFIG_SUITES` registry existed to prevent.

### Manual Testing Steps

1. Run each new subcommand directly against real files in this repository
   and compare output to what the bash predecessor produced for the same
   input (captured before that phase's bash removal).
2. Invoke each rewritten skill (`/accelerator:create-adr`,
   `/accelerator:create-plan`, etc.) interactively and confirm no permission
   prompt fires for a stale `allowed-tools` rule and the artifact this
   session produces carries a correctly-rendered metadata block / linkage
   section.
3. Run the 0007 migration script end to end against a scratch corpus after
   Phases 3 and 4 land, confirming its two rewritten call-outs behave
   identically to their bash predecessors.

## Performance Considerations

None expected — every subcommand is a short-lived, single-invocation CLI
call replacing an equally short-lived bash script; no hot path or
long-running process is introduced.

## Migration Notes

Noun groups ship independently across the five phases; a mixed bash/Rust
state between phases is expected and accepted per the work item's own
Technical Notes. Each phase's floor/registry edits are scoped to exactly what
that phase's bash removal affects — no phase decrements a floor its own
removal doesn't touch.

## References

- Original work item: `meta/work/0195-accelerator-corpus-adr-metadata-frontmatter-linkage-cli.md`
- Related research: `meta/research/codebase/2026-08-06-0195-accelerator-corpus-cli-implementation-surface.md`
- Sub-binary precedent: `cli/vcs-cli/` (`Cargo.toml`, `src/main.rs`, `src/cli.rs`, `tests/detect_goldens.rs`)
- Registration checklist: `tasks/README.md#registering-a-dispatched-sub-binary`
- Config wiring precedent: `cli/visualiser/server/src/config.rs`, `cli/launcher/src/main.rs:149-171`
