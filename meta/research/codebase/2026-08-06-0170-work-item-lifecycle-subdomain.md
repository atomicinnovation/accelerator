---
type: "codebase-research"
id: "2026-08-06-0170-work-item-lifecycle-subdomain"
title: "Research: Work-Item Lifecycle Subdomain (accelerator-work)"
date: "2026-08-06T08:12:12+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0170"
parent: "work-item:0170"
topic: "Work-Item Lifecycle Subdomain (accelerator-work)"
tags: ["research", "codebase", "rust", "work-items", "cli", "corpus", "config", "store"]
revision: "0008d64b6ae5772c53e58c99fc2e968ec8f56d93"
repository: "accelerator"
last_updated: "2026-08-06T08:12:12+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Work-Item Lifecycle Subdomain (accelerator-work)

**Date**: 2026-08-06T08:12:12+00:00
**Author**: Toby Clemson
**Git Commit**: 0008d64b6ae5772c53e58c99fc2e968ec8f56d93
**Branch**: (jj workspace `visualisation-system`, working-copy change `omvsuxzo`)
**Repository**: accelerator

## Research Question

What does the codebase already provide, and what does it still lack, in order
to implement work item [0170: Work-Item Lifecycle Subdomain](../../work/0170-work-item-subdomain-and-sync-engine.md)
— an `accelerator-work` Rust CLI sub-binary implementing local-only work-item
lifecycle CRUD (`create`, `show`, `update`, `resolve`, `diff`, plus internal
helpers) over the shared `corpus`/`config`/`store` crates, replacing 11 bash
scripts in `skills/work/scripts/`?

## Summary

The shared hexagonal crates (`corpus`, `corpus-adapters`, `config`,
`config-adapters`, `store`, `document`) already provide every low-level
primitive `accelerator-work` needs — atomic whole-file writes, an ID-pattern
domain model and scan-regex compiler, config keys for `work.id_pattern`/
`work.default_project_code`, and a frontmatter parse/render layer — but
**nothing composes them into work-item lifecycle logic yet**. 0170 will be the
first crate to wire config → `WorkItemIdScheme` → `RegexScanner` →
ID-allocation into an end-to-end flow, and the first place `external_id` gets
any Rust representation at all (today it's a pure frontmatter-key convention
per ADR-0044, read/written only by bash).

The most direct implementation template is `cli/vcs-cli/` (package
`accelerator-vcs`), the only other dispatched sub-binary built on this
generation of the architecture (the visualiser predates it and doesn't follow
the domain/adapter/binary three-crate split). Its sibling work item, 0169
(VCS Subdomain and Hooks Migration), is the closest process precedent:
same "characterization-then-port" methodology, same registration checklist,
same hexagon/pup-rule/parity-test-feature-flag conventions. 0169's 10-phase
plan structure is a strong starting point for 0170's own plan.

The 11 bash scripts in scope are all **read/compute helpers — none of them
writes to a work-item file**. Actual file mutation (the atomic whole-file
replace) is something 0170 must wire up itself using `FileCorpusStore::write`
(`corpus-adapters`) and `document::render` (whole-frontmatter rewrite) or the
existing single-key `patch_status` pattern (surgical single-line rewrite) —
neither of which currently generalises to multi-field/tag mutations.

The scope boundary between 0170 and its split sibling 0194 is already
precisely reconciled by review: **11 lifecycle-side scripts belong to 0170,
4 sync-side scripts belong to 0194**, with one correction worth double-checking
against the current work-item text — `work-item-fetch-remote.sh` was
originally miscounted into 0170 but actually belongs to 0194.

## Detailed Findings

### The 11 bash scripts in scope, and what each does

All in `skills/work/scripts/`. `work-item-common.sh` is a sourced-only
function library (not a script itself, but foundational — see below). None of
these twelve files writes to a work-item's content; they are exclusively
read/query/compute helpers. Actual mutation is left to callers (the `/`
skills), which currently do the write themselves via `config_set_frontmatter_field`
et al. — a responsibility 0170 must absorb into `accelerator work
create`/`update`.

#### `work-item-common.sh` — pattern-DSL compiler and ID helpers (sourced only)

A pure-bash library with a stable `E_PATTERN_*` error-prefix taxonomy
(`work-item-common.sh:16-24`). Core internal function `_wip_compile`
(`:43-207`) is the pattern-DSL compiler, driven by a `mode` arg (`scan` |
`format` | `validate`) — this is already ported to Rust as
`corpus-adapters::work_item_pattern::compile_scan_regex`
(`cli/corpus-adapters/src/work_item_pattern.rs:125-229`), parity-tested
against the bash original. Public API surfaces `accelerator-work` needs to
either reuse (where already ported) or port fresh:

- `wip_validate_pattern` — **partially ported**, only via
  `compile_scan_regex`'s validation path (scan mode); the bash original also
  validates independently of `format`/`validate` mode, so parity coverage
  for those modes should be checked at implementation time.
- `wip_compile_scan` — **already ported** (`compile_scan_regex`).
- `wip_compile_format` — not yet ported; needed for `create`'s
  zero-padded-ID formatting.
- `wip_pattern_max_number` — not yet ported; needed for `create`'s overflow
  guard.
- `wip_is_legacy_id` / `wip_pad_legacy_number` — legacy 4-digit ID handling;
  not yet ported.
- `wip_parse_full_id` / `wip_canonicalise_id` / `wip_extract_id_from_filename`
  — id parsing/canonicalisation; `corpus::WorkItemIdScheme::normalise_id`/
  `canonicalise_id`/`extract_id` (`cli/corpus/src/work_item_id.rs:87-168`)
  cover similar ground but should be checked field-by-field for parity before
  assuming equivalence.
- `wip_is_work_item_file` — own-identity (`id`/`work_item_id`) predicate; not
  yet ported as a standalone function, though `corpus_adapters::assemble`
  does related work.

#### `work-item-next-number.sh` — ID allocation

`--project CODE` / `--count N` flags. Reads `work.id_pattern`/
`work.default_project_code` via the `accelerator config` binary (shells out —
0170's Rust version should call `config::ConfigAccess` directly instead).
Scans `$WORK_DIR/*.md` for the highest existing number matching the compiled
scan regex, applies an overflow guard against the pattern's numeric cap, and
prints `COUNT` sequential formatted IDs. Read-only; no writes.
**Exit codes**: 0 success; 1 on usage/pattern/overflow errors.

#### `work-item-normalise.sh` — content normalisation for diffing

`<file>` or `--stdin` modes. Forces `LANG=C LC_ALL=C` for cross-machine byte
stability. Strips a fixed set of provenance keys
(`last_updated last_updated_by id external_id updated_at revision`) before
hashing, so a bare re-save isn't flagged as a change — this is the mechanism
`work-item-section-diff.sh` relies on for its normalised-hash comparison.
Read-only.

#### `work-item-pattern.sh` — pattern-compiler CLI wrapper

Thin CLI exposing `--validate`/`--compile-scan`/`--compile-format` over
`work-item-common.sh`'s functions. **This is the internal helper — not a
subcommand** per 0170's Technical Notes; becomes a private function inside
`accelerator-work`. Its test suite, `test-work-item-pattern.sh`, is the
designated **parity gate** other new characterization suites get repointed
against (see below).

#### `work-item-read-field.sh` / `work-item-read-status.sh`

`<field-name> <file>` reads a frontmatter field value (first-match-wins line
scan, quote-stripped, arrays returned verbatim/unparsed). Has an
own-identity alias: asking for `id` falls back to `work_item_id` and vice
versa. `read-status` is a pure delegate (`exec`s the field reader with
`status`). These map directly to `accelerator work show <path> --field NAME`
(including the `--field status` shorthand named in 0170's AC).

#### `work-item-resolve-id.sh` — ID/path → file resolver

Single positional `<input>` arg. **Documented exit-code contract**: 0 =
single match; 1 = `E_RESOLVE_INVALID`; 2 = `E_RESOLVE_AMBIGUOUS`; 3 =
`E_RESOLVE_NOT_FOUND`. Classifies input as `path` / `full_id` / `bare_number`
/ `invalid`, and for `bare_number` builds a **tagged candidate set** across
four priority-ordered sources (project-prepended, legacy 4-digit, no-project
pattern-shape, cross-project scan) before resolving ambiguity. This is
meaningfully more logic than a thin wrapper over `WorkItemIdScheme` — 0170's
`resolve` command needs to reproduce this exact candidate-search/ambiguity
logic, not just call `normalise_id`.

#### `work-item-section-diff.sh` — per-section diff for sync conflicts

`<local-file> <remote-file>`, direction fixed (local = `-`, remote = `+`).
Extracts `frontmatter`, `(preamble)`, and each unioned `## heading` section
from both files, skips sections whose **normalised hashes** match (via
`work-item-normalise.sh --stdin` + sha256, deliberately not relying on
`diff`'s exit status), and runs `diff -u` per differing section into a
`mktemp -d` scratch dir (cleaned via `EXIT` trap). Read-only on the real
files. Maps directly to `accelerator work diff <local> <remote>`.

#### `work-item-update-tags.sh` — tag-array mutator (compute only)

`<path> add|remove <tag>`. Detects and refuses block-style YAML tags
(`tags:` followed by an indented `- ` list) rather than attempting to rewrite
them. Parses/rebuilds the canonical inline-array form (`[a, b, "c,d"]`),
quoting tags containing `,`/`:`/`#`. **Prints the new canonical array to
stdout — does not write the file itself.** `accelerator work update` must
both compute this (or equivalent) and perform the actual atomic write, which
the bash version's caller currently does separately.

#### `work-item-file-dirty.sh` — VCS dirtiness predicate

`<path>`, exit 0 = dirty, exit 1 = clean. Resolves jj-vs-git mode (jj wins if
`.jj` present, never falls back to git even when colocated), fails safe to
DIRTY on any indeterminate state (no repo root, or an untracked file under
git counts as dirty too — deliberately not consistent with the `run-migrations`
convention). Not itself part of 0170's five user-facing subcommands, but
listed as an internal helper — likely relevant to `update`'s local-overwrite
safety semantics if any exist for a purely local command (worth confirming
whether local-only `update` even needs a dirty check, since there's no remote
overwrite risk in this story's scope).

#### `work-item-project-remote.sh` — remote payload projection

Reads raw tracker `show` JSON on stdin, `--integration jira|linear
updated|body`. **This is out of 0170's scope** — it's a remote-facing
projection seam feeding the sync pipeline, not named in 0170's Requirements
or Technical Notes list, and should stay with 0194 alongside the other
remote-facing scripts (confirm this script isn't accidentally in 0170's
"internal helpers" list — it currently isn't).

#### `work-item-template-field-hints.sh` — template hint extractor

`<field>`, always exits 0. Extracts trailing-comment `#a|b|c` hint lists from
the `create-work-item` template's frontmatter field lines, falling back to a
hardcoded list (`kind`/`status`/`priority`) if the template read fails or the
field has no hint comment. No equivalent exists in `config`/`config-adapters`
today (see Gaps below) — this is new logic to write.

#### `EXIT_CODES.md` — does not cover these 12 scripts

`skills/work/scripts/EXIT_CODES.md` documents a *different*, narrower
contract: the `E_DISPATCH_*` taxonomy (70/71/72/73) for the remote-facing
bridge scripts (`work-item-create-remote.sh`, `work-item-fetch-remote.sh`,
`work-item-update-remote.sh`, `work-item-push-decide.sh`) — all 0194's scope.
Exit codes for the 12 lifecycle-side files must be read from each script's own
header comments/inline `E_*` echoes, not from this file.

#### `test-work-item-pattern.sh` — the designated parity gate

Sources `scripts/test-helpers.sh`, exercises `work-item-pattern.sh` at both
the CLI (subprocess, black-box) and library (`source`d function-call) levels
across six groups: `--validate` (all five `E_PATTERN_*` rules),
`--compile-scan`, `--compile-format`, a round-trip property test (format →
scan → recover the original number across boundary widths), direct library
calls, and project-value validation (rule 5) at use time. 0170's AC names
this as the gate the Rust `accelerator work` parity suite repoints against —
each `assert_eq`/`assert_exit_code`/`assert_contains`/`assert_matches_regex`
call maps close to mechanically onto an equivalent Rust CLI-output assertion.

### Shared crates: what's already built

#### `cli/corpus/` — pure domain (no serde/YAML/regex/fs)

- `WorkItemIdScheme` (`cli/corpus/src/work_item_id.rs:20-169`) — the domain
  model of `{id_pattern, default_project_code}`, with
  `normalise_id`/`canonicalise_id`/`extract_id`/`canonical_digit_width`
  methods. `extract_id` takes an injected `IdScanner` port
  (`cli/corpus/src/work_item_id.rs:7-17`).
- `DocTypeKey::WorkItems` (`cli/corpus/src/doc_type.rs:9-193`) — one of 14
  doc-type variants; `config_path_key()` → `"work"` (matches `paths.work`),
  `linkage_type_name()` → `"work-item"`.
- `record::Record`/`store::{AtomicWrite, RecordStore, StoreError}`
  (`cli/corpus/src/record.rs`, `store.rs:14-79`) — the two driven ports
  `accelerator-work` will consume for persistence.
- `metadata::{ArtifactMetadata, Clock}` — the artifact-metadata block
  `create` will need to stamp (`Current Date/Time (UTC)`, filename
  timestamp, repo/revision).
- **No `external_id` anywhere** — confirmed by grep across all of `cli/`.
  Purely a frontmatter-key convention today (ADR-0044), read/written only by
  bash. 0170 must add its own minimal handling (likely just a `Mapping`/
  `Yaml` entry read/write, no bespoke domain type, since ADR-0044's rule is
  presence-only).

#### `cli/corpus-adapters/` — outbound adapters

- `FileCorpusStore::write` (`cli/corpus-adapters/src/store.rs:101-105`) —
  the ready-made whole-file-replace call: `FileCorpusStore::new(root)
  .write(&path, &bytes)`, internally using `store::atomic_write` with
  `NewFileMode::PreserveOr`. This is the primitive `create`/`update` should
  call directly.
- `work_item_pattern::compile_scan_regex`
  (`cli/corpus-adapters/src/work_item_pattern.rs:125-229`) — the existing
  Rust port of `_wip_compile`'s scan mode, parity-tested against bash. This
  is exactly `work-item-pattern.sh`'s scan-compile function; format-compile
  and max-number are not yet ported.
- `RegexScanner` (`cli/corpus-adapters/src/scanner.rs:10-40`) — implements
  `corpus::IdScanner` over a compiled regex.
- `patcher::patch_status` (`cli/corpus-adapters/src/patcher.rs:48-94`) — a
  line-preserving, **single-key-only** (`status:`) frontmatter rewrite,
  byte-for-byte preserving everything else. This is the existing template
  for surgical rewrites, but doesn't generalise to multi-field/tag mutation
  — see Gaps.
- `metadata::derive_at` — one-call metadata derivation `create` can use.
- `document::{parse, FrontmatterState}` — translates `document::Yaml` into
  `corpus::FrontmatterValue`; the boundary `accelerator-work` should route
  through rather than touching `serde_saphyr` directly.
- `lock::{acquire, LockGuard}` — the mkdir-based advisory lock backing
  `FileCorpusStore`.

#### `cli/config/` and `cli/config-adapters/`

- `catalogue::WORK_KEYS` (`cli/config/src/catalogue.rs:98-114`) — exactly
  the three keys needed: `work.integration` (0194's concern),
  `work.id_pattern` (default `"{number:04d}"`), `work.default_project_code`.
- `service::ConfigAccess::effective_nonempty`
  (`cli/config/src/service.rs:425-438`) — the read path 0170 needs (resolves
  personal-over-team, collapses explicit-empty to catalogue default).
- `FileConfigStore::discover_root` (`cli/config-adapters/src/store.rs:113-125`)
  — walks up for `.accelerator/`/`.git`/`.jj`; reusable for locating
  `meta/work/`.
- `paths::doc_type_dirs` (`cli/config/src/paths.rs:30-67`) — resolves
  `paths.work` (default `"meta/work"`) with the same safety checks as every
  other doc-type path.
- No template-field-hints equivalent exists — `FileConfigStore::resolve_template`
  (`cli/config-adapters/src/store.rs:332-410`) resolves template *content*
  (three-tier: config path → user override → plugin default) but doesn't
  parse hint comments out of it.

#### `cli/store/` — atomic-write primitive (infra-only, no `kernel` dep)

- `atomic_write(path, bytes, bounds, mode)` (`cli/store/src/lib.rs:85-97`) —
  stage-then-rename-then-fsync, refusing symlinked/out-of-bounds paths via
  `ensure_contained`. This **is** "the same whole-file replace contract as
  `work-item-update-tags.sh`" 0170's AC names — already implemented, not
  something to reimplement.
- `WriteBounds`/`NewFileMode::{Set, PreserveOr}` — the mode-selection
  contract `FileCorpusStore` already wires up.

#### `cli/document/` — markdown+YAML-frontmatter protocol

- `parse`/`render` (`cli/document/src/parse.rs:16-19`,
  `render.rs:19-28`) — `render` re-serialises a **whole** frontmatter tree
  via `serde_saphyr`, preserving the body verbatim. This whole-tree
  round-trip path (not `patch_status`'s line-surgical approach) is likely
  what `create` (fresh file) and multi-field `update`/tag mutation need,
  unless a comparably surgical multi-key patcher gets built instead.
- `tags::reject_tagged` — fails closed on any explicit YAML tag, scanning
  the parser's event stream rather than raw text.

### Gaps `accelerator-work` (0170) must fill itself

1. **`external_id` has no Rust representation anywhere** — a frontmatter-key
   convention only, per ADR-0044 (presence-based sync signal, independent of
   `id`'s value/format). 0170 needs to read/write it but likely doesn't need
   a bespoke domain type.
2. **No composed "work-item lifecycle" service exists.**
   `WorkItemIdScheme` (domain), `compile_scan_regex`/`RegexScanner`
   (adapter), and `config::WORK_KEYS` (config) are three separate, unwired
   pieces today. 0170 is the first place that composes
   config → scheme → scanner → ID-allocation end to end.
3. **No generic multi-field frontmatter patcher.** `patch_status` only ever
   touches one non-indented `status:` line; `document::render` (whole-tree
   re-serialise) is the alternative for tag add/remove and multi-field
   `update`, but nothing today does a targeted, line-preserving *multi-key*
   rewrite the way `patch_status` does for one key. Deciding between
   "extend `patch_status`-style surgery to N keys" vs. "use `document::render`'s
   whole-tree rewrite" is an open implementation decision.
4. **No next-number/ID-allocation logic.** `WorkItemIdScheme` validates and
   canonicalises IDs but has no "scan existing corpus, return next free ID"
   method — this is new logic porting `work-item-next-number.sh`.
5. **No section-diff logic** exists in any shared crate — new logic porting
   `work-item-section-diff.sh`.
6. **`wip_compile_format`, `wip_pattern_max_number`, legacy-ID helpers, and
   `wip_is_work_item_file`** are not yet ported to Rust (only scan-mode
   compilation is).
7. **Template-field-hints extraction** — no counterpart exists;
   `resolve_template` gets content only, not parsed hints.

### The 0170/0194 split: authoritative script boundary

Per the review that drove the split
(`meta/reviews/work/0170-work-item-subdomain-and-sync-engine-review-1.md`):
**11 lifecycle-side scripts belong to 0170, 4 sync-side scripts belong to
0194.** One correction worth re-checking against 0170's current text:
`work-item-fetch-remote.sh` was originally miscounted into 0170's parity/
removal criteria but is actually a dependency of `work-item-sync-apply.sh`
(0194's scope), not of any lifecycle command. The work item as currently
written (read in full at the start of this research) does *not* list
`work-item-fetch-remote.sh` among its Requirements/Technical Notes, so this
correction already appears to have landed — but it's flagged here since the
review explicitly called out this exact miscount as something to watch for.

Other review outcomes already reflected in the current 0170 text (confirmed
by reading it fully): `show`/`resolve`/`diff` have explicit Given/When/Then
ACs anchored to their respective bash scripts; the characterization-test AC
requires every flag/argument combination plus at least one error path per
script; `create`'s "fully populated frontmatter" AC references the
`create-work-item` template schema explicitly. One item from the review not
yet visible as resolved in 0170's text: the review names "a cargo-nextest
filter excluded from the default `cargo test`/`cargo nextest run` invocation"
as the mechanism for gating the (in this story's case, absent — no network
calls) contract/integration suite; 0170's own AC instead states this crate
needs *no* separate contract/integration suite at all, which is consistent
(this story makes no network calls) — no discrepancy, just worth noting the
mechanism named in the review (nextest filter / `bash-parity`-style feature
gate) is the established idiom if any gated suite turns out to be needed
later (e.g. for characterization tests that shell out to real VCS state, per
`work-item-file-dirty.sh`).

### Architectural precedent: `cli/vcs-cli/` and work item 0169

`cli/vcs-cli/` (package `accelerator-vcs`) is the only existing dispatched
sub-binary built on the current domain/adapter/binary three-crate hexagonal
pattern (the visualiser predates this generation of the architecture). Its
shape:

- **`Cargo.toml`** — package `accelerator-vcs`, mandatory `description`,
  inherited `version.workspace`/`edition.workspace`/etc., `[lints] workspace
  = true`, `[[bin]] name = "accelerator-vcs" path = "src/main.rs"`. A
  `bash-parity` feature flag gates golden-fixture tests needing real jj/git
  binaries (off by default, on in CI).
- **`src/cli.rs`** — clap `derive` API: `Cli { command: Command }`,
  `Command` an enum with one arm per subcommand (`Detect`, `Status`, `Log`,
  `Guard`).
- **`src/main.rs`** — parses via `Cli::parse()`, matches on `cli.command`,
  each arm calls a thin `run_*` function wiring an adapter implementation
  into a `mod <subcommand>` that delegates to the domain crate. Errors are
  `kernel::Error`, mapped to exit codes via a `report()` helper.
- **Tests**: `cli/vcs-cli/tests/{detect_goldens,status_log_goldens,
  guard_decision_table}.rs` — golden-fixture/table-driven, spawning the
  *compiled* binary via `env!("CARGO_BIN_EXE_accelerator-vcs")`, gated
  behind `bash-parity`.
- **`cli/vcs-test-support/`** — a dev-dependency-only fixture crate
  (`hermetic.rs`, `masks.rs`, `stubs.rs`) shared between `vcs-adapters` and
  `vcs-cli` tests — a pattern `accelerator-work` could mirror if it needs
  fixtures shared across its own crate + `corpus-adapters` tests.

**Naming collision to avoid**: 0169's research explicitly flags that the
domain crate's directory name and the dispatch token are two separate
naming systems that silently assume they're the same string.
`_SUBBINARY_MANIFESTS`' default path resolution
(`tasks/manifest.py:52-61`) assumes `cli/<token>/Cargo.toml`, and
cargo-pup's domain-import restriction rule matches on the crate's module
path. If 0170 needs a `work` domain crate *and* an `accelerator-work` binary
crate as separate directories (mirroring `vcs`/`vcs-cli`), the binary crate
must live at a different directory name (e.g. `work-cli`) with an explicit
`_SUBBINARY_MANIFESTS["work"] = CLI_DIR / "work-cli/Cargo.toml"` entry — this
exact collision bit 0169 once already.

0169's plan (`meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md`)
used a 10-phase structure directly transferable to 0170:

1. Capture shell behaviour as fixtures (masks.toml, goldens, decision
   tables, hand-authored departure fixtures) — **before** any Rust domain
   code, and before any shell deletion.
2. Move/relocate shared types into the domain crate (mechanical crate-
   boundary move).
3. Core domain logic, test-first against a hand-rolled test-double port; a
   separate fixture-matrix integration test lives in the adapter crate to
   avoid tripping the domain crate's own pup import-restriction rule.
4. Shared infrastructure module (N/A for 0170 unless an analogous piece
   exists).
5. First vertical slice: launcher-level blockers plus scaffolding the new
   binary crate and one complete subcommand end-to-end.
6. Second vertical slice (read-only subcommands).
7. Third vertical slice (most complex/stateful subcommand) as pure
   `decide(input) -> Decision` domain logic, no port trait, kept separate
   from I/O composition.
8. Sub-binary registration + skill repoint, landed together (checklist
   points 1, 2, 3, 4, 7, 8 "must land in the same change").
9. Hook/entry-point rewrite and legacy shell deletion, repointing the
   parity suite per a re-verified script partition.
10. Hand-offs, documentation, validation.

Key process lessons flagged as having bitten 0169 twice: the exact
script/case partition between "ported" and "shell-only" must be
**re-verified from source at plan time**, not trusted from prior prose —
0169's own review history recorded this partition drifting across editing
passes. 0170's plan should independently re-count the 11 lifecycle scripts
against the current `skills/work/scripts/` directory contents rather than
trusting this research (or the 0170 review) as final, since further edits
may land between now and plan time.

### Sub-binary registration: the 13-point checklist

Full checklist at `tasks/README.md:304-456` (verbatim quoted in the
`0187 sub-binary registration and 0164 launcher/dispatch` sub-research, not
reproduced in full here for brevity — see that section's complete capture).
Points **1, 2, 3, 4, 7, 8 must land in the same change**. Key facts for a
`work` token:

- Token `work` matches the required `^[a-z][a-z0-9-]*$` pattern, is not in
  `RESERVED_TOKENS` (`verify`, `launcher`), and is not in
  `BUILTIN_SUBCOMMANDS` (`version`, `config`, `help`) — no naming conflict.
- Override env var: `ACCELERATOR_WORK_BIN` (derived by
  `derive_override_var("work")` in `cli/launcher/src/launch/core.rs:257-293`).
- Registries to update: `DISPATCHED_SUBBINARIES`
  (`tasks/shared/paths.py:29`), `_SUBBINARY_MANIFESTS`
  (`tasks/manifest.py:52-61`, only if the binary crate isn't at
  `cli/work/`), `[workspace].members` in `cli/Cargo.toml:4`,
  `_CLI_RELEASE_BINARIES` (`tasks/build.py:35`, add
  `"accelerator-work"` — the `[[bin]]` name, not the bare token).
- Skill binding: a `SKILL.md` invoking `accelerator work ...` via the `!`
  preprocessor plus a scoped `Bash(...)` rule (not a wildcarded token
  segment, not a bare `Bash` tool) — `skills/work/*` is the obvious
  candidate to repoint once the binary exists.
- Dispatch itself needs **zero launcher Rust code changes** — clap's
  `#[command(external_subcommand)]` on `Command::External` already captures
  any unrecognised subcommand generically; all the registration work is in
  Python build-system registries plus the crate's own `Cargo.toml`.

### Related meta documents (not deep-read this pass, but located)

Beyond the four analysed above, `meta/documents-locator` surfaced ~70
relevant documents. Most directly useful if deeper reading is wanted before
planning:

- `meta/decisions/ADR-0044-remote-work-item-identity-in-external-id.md` —
  the `external_id` convention's canonical source (read by the corpus/config
  analysis agent, not independently deep-read here).
- `meta/decisions/ADR-0045-skills-vs-cli-division-of-labour.md`,
  `ADR-0052-filesystem-as-message-bus-and-knowledge-corpus.md`,
  `ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md` — the
  three ADRs 0170 itself cites in References.
- `meta/decisions/ADR-0054-git-style-modular-cli-of-on-demand-static-binaries.md`
  — underpins the dispatch/registration mechanism (0164/0187).
- `meta/research/codebase/2026-07-11-0179-corpus-crates-parsing-conventions.md`,
  `2026-07-19-0180-atomic-store-primitives-corpus-adapters.md`,
  `2026-07-07-0178-config-crates-native-yaml-reader.md` — the research
  behind the three crates 0170 builds directly on.
- `meta/research/codebase/2026-08-02-0187-generalise-sub-binary-registration-surface.md`
  — full research behind the registration checklist.
- `meta/reviews/work/0194-tracker-crate-and-remote-sync-engine-review-1.md`
  — the sibling's own review, useful for confirming the 0170/0194 boundary
  from the other side.
- No plan, PR, or validation document exists yet for 0170 or 0194
  themselves — both are still pre-plan.

## Code References

- `cli/corpus/src/work_item_id.rs:20-169` — `WorkItemIdScheme` domain type
- `cli/corpus/src/doc_type.rs:9-193` — `DocTypeKey::WorkItems` and doc-type
  table
- `cli/corpus/src/store.rs:14-79` — `AtomicWrite`/`RecordStore` ports
- `cli/corpus-adapters/src/store.rs:101-105` — `FileCorpusStore::write`
  (the whole-file-replace call to use directly)
- `cli/corpus-adapters/src/work_item_pattern.rs:125-229` —
  `compile_scan_regex`, the existing Rust port of `_wip_compile` scan mode
- `cli/corpus-adapters/src/scanner.rs:10-40` — `RegexScanner`
- `cli/corpus-adapters/src/patcher.rs:48-94` — `patch_status`, the
  single-key surgical rewrite template
- `cli/config/src/catalogue.rs:98-114` — `WORK_KEYS`
  (`work.integration`/`work.id_pattern`/`work.default_project_code`)
- `cli/config/src/service.rs:425-438` — `ConfigAccess::effective_nonempty`
- `cli/config-adapters/src/store.rs:113-125` — `FileConfigStore::discover_root`
- `cli/store/src/lib.rs:85-97` — `atomic_write` (stage-rename-fsync)
- `cli/document/src/render.rs:19-28` — `render`, whole-frontmatter rewrite
- `cli/vcs-cli/Cargo.toml`, `src/cli.rs`, `src/main.rs` — the dispatched
  sub-binary template to model `accelerator-work` after
- `cli/launcher/src/launch/inbound/cli.rs:16-29` — `Command::External`,
  the generic-dispatch clap surface (zero launcher changes needed)
- `cli/launcher/src/launch/core.rs:257-293` — `derive_override_var`
  (`ACCELERATOR_WORK_BIN` derivation and token validation)
- `tasks/README.md:304-456` — the 13-point sub-binary registration
  checklist
- `tasks/shared/paths.py:29` — `DISPATCHED_SUBBINARIES`
- `tasks/manifest.py:52-61` — `_SUBBINARY_MANIFESTS`
- `tasks/build.py:35` — `_CLI_RELEASE_BINARIES`
- `skills/work/scripts/work-item-common.sh:16-425` — pattern-DSL compiler
  and ID helpers (sourced library)
- `skills/work/scripts/work-item-next-number.sh` — ID allocation
- `skills/work/scripts/work-item-resolve-id.sh` — ID/path resolver,
  documented exit codes 0/1/2/3
- `skills/work/scripts/work-item-section-diff.sh` — per-section diff
- `skills/work/scripts/work-item-update-tags.sh` — tag-array compute (no
  write)
- `skills/work/scripts/test-work-item-pattern.sh` — the parity gate

## Architecture Insights

- **Hexagonal layering is consistent across all shared crates**: pure
  domain (`corpus`, `config`) with zero serde/YAML/regex/fs dependencies,
  enforced by cargo-pup import-restriction rules matching the crate's
  module path; adapter crates (`corpus-adapters`, `config-adapters`) own
  every infra concern. `accelerator-work` should follow the same split —
  likely a pure `work` domain crate plus reuse of `corpus-adapters`/
  `config-adapters` rather than a new adapters crate, unless work-item-
  specific adapter logic (e.g. the candidate-search resolve logic) is
  substantial enough to warrant one.
- **The domain-crate-name vs. dispatch-token trap** is a recurring,
  explicitly-documented gotcha (bit 0169 twice) — a domain crate directory
  name and the CLI dispatch token are two separate naming systems that
  default-path resolution and pup rules both silently assume are identical.
- **Two independent enforcement mechanisms** gate crate boundaries:
  `cargo-deny` (crate-level ban-lists) and `cargo-pup` (intra-crate module-
  import restrictions). A new subdomain needs its own pup rule(s) in
  `cli/pup.ron`.
- **Registration is almost entirely a Python build-system concern**, not a
  Rust one — the launcher's `external_subcommand` clap mechanism already
  handles arbitrary new tokens generically; all the "ceremony" is
  registries in `tasks/`.
- **Characterization-then-port** is the established migration methodology:
  capture golden fixtures from the *real* running shell script before any
  deletion; hand-author (never auto-capture) fixtures for deliberate
  behavioural departures; gate fixture/subprocess-dependent tests behind a
  `bash-parity`-style cargo feature so they don't run by default.
- **Whole-file atomic replace is a solved, shared primitive**
  (`store::atomic_write`, wrapped by `FileCorpusStore::write`) — 0170's
  Requirements/AC language about "atomic write" and "whole-file replace
  contract" maps directly onto reusing this existing call, not
  reimplementing it.

## Historical Context

- `meta/reviews/work/0170-work-item-subdomain-and-sync-engine-review-1.md`
  — the review (two-pass, REVISE → APPROVE) that split the original
  combined "work-item subdomain + sync engine" story into 0170 (this item)
  and 0194, on scope grounds (epic-scale work filed as a story) and a
  missing 0171 dependency-relationship flag. Reconciled the script
  inventory as 11 (0170) + 4 (0194), reworded ACs to Given/When/Then, and
  named the `create-work-item` template schema explicitly for `create`'s AC.
- `meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md` — the
  closest sibling implementation plan; its 10-phase structure and process
  lessons (sequencing constraints, hand-authored departure fixtures,
  domain-crate/dispatch-token naming trap) are directly transferable
  precedent for a 0170 plan.
- `meta/decisions/ADR-0044-remote-work-item-identity-in-external-id.md` —
  the source of the `external_id`-as-remote-key convention 0170's
  Requirements explicitly say to preserve.

## Related Research

- `meta/research/codebase/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md`
  — closest architectural precedent (VCS subdomain migration).
- `meta/research/codebase/2026-08-02-0187-generalise-sub-binary-registration-surface.md`
  — full research behind the registration checklist referenced above.
- `meta/research/codebase/2026-07-11-0179-corpus-crates-parsing-conventions.md`,
  `2026-07-19-0180-atomic-store-primitives-corpus-adapters.md`,
  `2026-07-07-0178-config-crates-native-yaml-reader.md` — research behind
  the three foundation crates.
- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
  — the parent epic's original scope/architecture research, source of 0170
  itself (`derived_from` in frontmatter).

## Open Questions

- **Domain/adapter/binary crate split**: should `accelerator-work` follow
  the `vcs`/`vcs-adapters`/`vcs-cli` three-crate split, or a simpler
  two-crate (`work` + binary-in-same-directory) shape? The registration
  checklist behaves differently depending on this choice (point 2's
  `_SUBBINARY_MANIFESTS` entry is only needed for the three-crate shape).
  This is also 0170's own first Open Question (the internal-vs-subcommand
  boundary), not yet resolved against an implementation spike.
- **Multi-field frontmatter rewrite strategy**: extend `patch_status`'s
  line-surgical, single-key approach to N keys, or use `document::render`'s
  whole-tree re-serialise for `update`/tag mutation? Each has different
  byte-preservation guarantees the existing `patch_status` docstring cares
  about (comment style, quote style, CRLF) — worth resolving before
  `update`'s implementation, since 0170's AC requires "all other fields
  left unchanged."
- **Does `work-item-file-dirty.sh`'s dirtiness check matter to this
  story's `update` at all?** It's listed as an internal helper in 0170's
  Technical Notes, but the local-overwrite risk it guards against (losing
  uncommitted changes to a sync-driven remote overwrite) doesn't obviously
  apply to a purely local `update` command with no remote round-trip in
  this story's scope — confirm whether it's needed here or genuinely
  belongs entirely to 0194's sync-apply path despite being named in 0170's
  Requirements.
- **Confirm `work-item-fetch-remote.sh` is fully absent from 0170's
  current scope** — the review flagged an earlier miscount of this exact
  script into 0170; the work item as currently written doesn't name it,
  which appears consistent, but worth an explicit final check against
  `skills/work/scripts/` at plan time per 0169's lesson about re-verifying
  script partitions from source rather than trusting prior prose.
- **`wip_compile_format`/`wip_pattern_max_number` and the legacy-ID
  helpers are unported** — need Rust equivalents (either as new
  `corpus`/`corpus-adapters` additions, since they're closely related to
  the existing `compile_scan_regex`, or as `accelerator-work`-local code)
  before `create`'s ID-allocation logic can be written.
