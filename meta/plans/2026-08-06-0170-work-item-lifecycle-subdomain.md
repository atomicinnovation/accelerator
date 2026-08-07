---
type: plan
id: "2026-08-06-0170-work-item-lifecycle-subdomain"
title: "Work-Item Lifecycle Subdomain Implementation Plan"
date: "2026-08-06T08:39:45+00:00"
author: Toby Clemson
producer: create-plan
status: ready
work_item_id: "work-item:0170"
parent: "work-item:0170"
derived_from: ["codebase-research:2026-08-06-0170-work-item-lifecycle-subdomain"]
tags: [rust, work-items, cli, corpus, config]
revision: "702b6426ab80a1b6868f8921bb592360b9919a8d"
repository: "accelerator"
last_updated: "2026-08-07T13:12:10+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Work-Item Lifecycle Subdomain Implementation Plan

## Overview

Build `accelerator-work` — a new dispatched sub-binary implementing
`work create|show|resolve|diff|update` — over the shared `corpus`/
`corpus-adapters`/`config`/`config-adapters` crates, absorbing ID allocation,
resolution, section-diffing, and tag mutation out of 11 bash scripts in
`skills/work/scripts/`. Every command is local-only: no network calls, no
`RemoteTracker` dependency. `--push` support and the sync engine are 0194's
scope.

## Current State Analysis

The shared hexagonal crates already provide every low-level primitive this
story needs — atomic whole-file writes (`store::atomic_write`, wrapped by
`corpus_adapters::FileCorpusStore::write`,
`cli/corpus-adapters/src/store.rs:101-105`), an ID-pattern domain model
(`corpus::WorkItemIdScheme`, `cli/corpus/src/work_item_id.rs:20-169`), a
partially-ported pattern-DSL compiler (`corpus_adapters::work_item_pattern::
compile_scan_regex`, `cli/corpus-adapters/src/work_item_pattern.rs:125-229`),
and a whole-frontmatter parse/render layer (`document::{parse, render}`,
preserving an existing document's body verbatim). Nothing composes them into
work-item lifecycle logic yet, and `external_id` has no Rust representation
anywhere — purely a frontmatter-key convention today (ADR-0044).

The closest architectural precedent is `cli/vcs-cli/` (package
`accelerator-vcs`), the only other sub-binary built on the current
domain/adapter/binary hexagon, and its sibling work item 0169
(`meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md`), whose
10-phase structure this plan mirrors, scaled down: 0170 needs none of 0169's
launcher-level fail-safe/cache-probe fixes or hook-envelope work, since none
of its commands are hooks and none call the launcher's external-dispatch
resolver in a new way.

### Key Discoveries

- **The work item's own AC premise is wrong on one point, verified against
  the actual repository state**: the AC states "none has a dedicated
  `test-work-item-*.sh` suite today" for the 10 characterization-target
  scripts. In fact `skills/work/scripts/test-work-item-scripts.sh` (64.6K, one
  suite covering `next-number`, `resolve-id`, `read-status`, `read-field`,
  `update-tags`, `template-field-hints`, `normalise`, `file-dirty`,
  `project-remote`, and `section-diff` — confirmed by its own `=== ... ===`
  section headers) already exercises all 10 with real, non-stub assertions,
  and is glob-discovered and run by `mise run test:integration:work`
  (`run_shell_suites(context, "skills/work", ...)`,
  `tasks/test/helpers.py:58-83`) as one of the `_EXPECTED_WORK_SUITES = 6`
  floor (`tasks/test/integration.py:49,383`). This does not remove any of
  0170's work — the Rust port still needs its own characterization tests and
  the bash suites still need deleting per the AC — but Phase 1 does not need
  to invent fixture states from nothing: it ports the already-pinned
  assertions in `test-work-item-scripts.sh` (and `test-work-item-pattern.sh`,
  already correctly identified as the parity gate) into goldens, the same
  role the existing `skills/work/scripts/test-fixtures/work-item-next-number.golden`
  already plays for one case. Because `test-work-item-scripts.sh` also covers
  four 0194-owned sync-side scripts (`sync-label`, `sync-baseline`,
  `sync-classify`, `sync-decide`, `sync-apply`) in the same file, Phase 9
  deletes only the lifecycle-side sections of that file, not the file itself.
- **`work-item-project-remote.sh` genuinely is in 0170's scope**, contrary to
  the research document's own per-script deep-dive (which claims it is "not
  named in 0170's Requirements or Technical Notes" and should stay with
  0194) — that specific paragraph is simply wrong; the work item's Requirements
  section lists `project-remote` explicitly among the internal helpers, the
  characterization AC lists `work-item-project-remote.sh` explicitly among the
  10 scripts, and the research's own summary count of "11 lifecycle scripts"
  already includes it. It takes already-fetched JSON on stdin — no network
  call of its own — so it is fully compatible with this story's local-only
  scope.
- **`work-item-template-field-hints.sh` cannot be "internal-only, no CLI
  subcommand" as the work item's Technical Notes state**, because it has real
  production callers that are not Rust and never will be: `update-work-item`
  and `list-work-items` (`skills/work/update-work-item/SKILL.md:107-110`,
  `skills/work/list-work-items/SKILL.md:52-59`) invoke it directly as a
  standalone process via the `!` preprocessor/Bash tool. A Claude-driven skill
  can shell out to a CLI subcommand; it cannot link a Rust crate. This plan
  therefore exposes it as a small `accelerator work template-hints <field>`
  subcommand (Phase 8) — a correction to the Technical Notes, not a violation
  of it: `work-item-pattern.sh` has **zero** production callers anywhere in
  `skills/` (grep-confirmed) — its only consumer is its own test suite —
  which is why "no subcommand" is exactly right for it, and demonstrates why
  the same framing does not fit `template-field-hints`. The remaining three
  scripts in the "internal-only" group (`work-item-file-dirty.sh`,
  `work-item-project-remote.sh`, `work-item-normalise.sh`) turn out **not**
  to fit the "consumed only by other scripts" framing either — grep confirms
  `sync-work-items/SKILL.md` shells out to all three directly
  (`SKILL.md:120,136,311`), the same category of direct-skill-caller that
  makes `template-field-hints` need a subcommand. They stay internal-only
  here anyway, but for a narrower reason: this story has no CLI-consuming
  command of its own to attach them to (`create`/`update`/`resolve`/`show`/
  `diff` don't call them), and `sync-work-items`' own orchestration logic is
  0194's scope to rewrite, not 0170's — so exposing a subcommand now would be
  built for a caller (0194's rewritten skill) that doesn't exist yet, in a
  shape 0170 can't validate. Their bash originals are therefore **not**
  deleted by this story (see Phase 9) — deleting them while
  `sync-work-items` still shells out to them would silently disable the
  script's fail-safe dirty-check guard (command-not-found reads as "false"
  in the guarding `if`), a data-loss risk worse than leaving three already
  characterization-tested scripts in place a little longer. This is a
  deliberate, scoped deviation from the work item's AC (which asks for all
  11 scripts to be removed "in the same change") — the AC's
  characterize-then-port intent is fully met for these three (Phase 1
  goldens, Phase 3/8 ports), only their bash-deletion date moves to 0194,
  when it rewires `sync-work-items` onto its own `sync` command and can
  repoint these three call sites to a real replacement.
- **`update-work-item`'s SKILL.md does today's actual write** via the `Edit`
  tool (line-by-line), not via any script — only tag mutations delegate to
  `work-item-update-tags.sh`, which computes the new canonical array but does
  not write the file itself (`WORK_ITEM_SCRIPT_DIR/work-item-update-tags.sh:
  1-8`). `accelerator work update` is the first thing to actually perform an
  atomic whole-file frontmatter rewrite for arbitrary field/tag mutations; the
  skill's own interactive interpretation (natural-language parsing, diff
  preview, confirmation, and body-label/H1 prose sync) stays exactly as it is
  today and is out of this story's scope — `update` only needs to accept
  already-decided field/tag operations and perform one atomic write.
- **`document::render` (`cli/document/src/render.rs:19-28`) is the
  already-built tool for `create`'s and `update`'s writes** — it re-parses an
  existing document's frontmatter (erroring rather than silently overwriting a
  YAML-invalid file) and re-serialises a whole `Yaml` tree while preserving
  the body verbatim. The AC's "same whole-file replace contract as
  `work-item-update-tags.sh`... with all other fields left unchanged" refers
  to atomicity (`store::atomic_write`) and content-preservation, not
  byte-identical formatting — `work-item-update-tags.sh` itself never writes a
  file at all, so there is no byte-format precedent to match. Extending
  `patch_status`'s (`cli/corpus-adapters/src/patcher.rs`) line-surgical,
  single-key approach to N keys was considered and rejected: it would need to
  re-derive multi-key-aware comment/quote-preservation logic `document::render`
  already solves generically, for no behavioural benefit the AC requires.
- **`work-item-section-diff.sh`'s `--stdin` normalisation mode only trims
  whitespace and blank lines** (`work-item-normalise.sh:82-94`'s `--stdin`
  branch calls `_win_trim` only) — it does **not** strip the `IGNORE_KEYS`
  provenance fields (`last_updated`, `revision`, etc.), because that filtering
  only exists in the `<file>` mode's `_win_filter_frontmatter`, which
  `--stdin` never reaches. So `work diff`'s frontmatter section faithfully
  shows a diff even when only `last_updated`/`revision` changed — this is
  the real, unsurprising-once-verified bash behaviour and is reproduced
  as-is, not "fixed." The full `IGNORE_KEYS`-stripping `<file>` mode is
  0194's sync-engine change-detection concern (not used by any of 0170's own
  five commands) but is still ported and characterization-tested here per the
  AC's script list, exactly like `file-dirty` (see "What We're NOT Doing").
- **`work-item-update-tags.sh`'s tag parser splits on every comma, not just
  separator commas** (`tr ',' '\n'` at `work-item-update-tags.sh:81-87`) —
  an existing tag written as `"c,d"` (a shape the same script's own
  `format_tag` quote-on-comma logic can produce) is mis-split into `c` and
  `d` the next time the array is re-parsed for an add/remove. This is a real
  asymmetry between write-side quoting and read-side splitting, not a
  hypothetical: the Rust port (`work::tags::mutate_tags`, Phase 3)
  reproduces it exactly, the same "characterize the real behaviour, don't
  silently correct it" treatment already applied to the `IGNORE_KEYS` quirk
  below.
- **Comparing normalised section content in Rust needs no hashing** — the
  bash hashes via `sha256` only to sidestep `diff`'s exit-status portability
  trap when deciding whether to print a section at all; a Rust `String`
  equality check after normalisation is a direct, simpler equivalent with
  identical results, not a declared behavioural departure.
- **`skills/work/scripts/EXIT_CODES.md` covers none of these 12 scripts** —
  it documents only the `E_DISPATCH_*` bridge taxonomy for the four
  0194-owned remote-facing bridges (`work-item-create-remote.sh`,
  `work-item-fetch-remote.sh`, `work-item-update-remote.sh`,
  `work-item-push-decide.sh`). No changes to this file are needed.

## Desired End State

`accelerator-work` is a registered dispatched sub-binary implementing `work
create|show|resolve|diff|update`, plus the `work template-hints` and `work
canonicalise-id` utility subcommands, reproducing the 11 lifecycle scripts'
behaviour exactly except the one declared simplification (hash-free section
comparison, behaviourally identical) and the one preserved quirk (the
naive-comma-split tag re-parse, reproduced not fixed — see Key Discoveries).
`EXIT_CODES.md`'s lifecycle-irrelevant status is confirmed unaffected. Of
the 11 scripts, 8 are removed along with `work-item-common.sh` and
`work-item-pattern.sh`'s test suite, and the corresponding sections of
`test-work-item-scripts.sh`; the work suite floor is decremented; every
skill that shelled out to one of those 8 now invokes `accelerator work
<verb>` instead. The remaining 3 (`work-item-file-dirty.sh`,
`work-item-project-remote.sh`, `work-item-normalise.sh`) are ported and
characterization-tested per the AC but their bash originals stay in place —
`sync-work-items` still shells out to them directly and 0194 removes them
once it rewires that skill (see Key Discoveries and "What We're NOT Doing"
for why). `mise run` is green end to end.

**Verification**: every phase below states its own automated and manual
success criteria; the story-level acceptance criteria in
`meta/work/0170-work-item-subdomain-and-sync-engine.md` are the composite
target and are cross-referenced per phase.

**0194's dependency surface**: three things this plan produces are
contracts 0194 builds against, not just this story's internal
implementation detail — `work-cli`'s CLI flags (frozen by the
whole-command-set snapshot test in Phase 8), the `cli/work` domain crate's
own `pub fn` signatures (`resolve`, `section_diff`, `tags`, `normalise`,
`template_hints`, `file_dirty`, `own_identity`, `update`), and
`cli/work-adapters::project_remote::project`, since 0194's `tracker`/`sync`
crate depends on both crates as libraries — either for the three scripts
with no CLI subcommand at all (`file_dirty`, `project_remote`,
`normalise`), or, for the rest, because in-process library reuse from
0194's own Rust code is available as an alternative to shelling out to the
CLI subcommand that also exists for each of them (`update`/`template_hints`
both have subcommands *and* are usable as a library; `resolve`/
`own_identity` have no CLI-exposed reason to be called directly but are
still public so 0194 can use them the same way). There's no snapshot test
for either library surface the way there is for the CLI flags — reviewers
changing any of these signatures in a follow-up to this story should treat
them with the same care as the CLI contract.

## What We're NOT Doing

- Not implementing `--push` on `create`/`update`, the `tracker` crate, or the
  `sync` command — all 0194's scope.
- Not touching `work-item-create-remote.sh`, `work-item-update-remote.sh`,
  `work-item-push-decide.sh`, `work-item-fetch-remote.sh`, or the four
  sync-stage scripts (`sync-label`, `sync-baseline`, `sync-classify`,
  `sync-decide`, `sync-apply`) — 0194 removes these.
- Not deleting `work-item-file-dirty.sh`, `work-item-project-remote.sh`, or
  `work-item-normalise.sh` — ported and characterization-tested per the AC
  (Phase 1 goldens, Phase 3/8 modules), but their bash originals stay in
  place because `sync-work-items` still shells out to all three directly and
  this story adds no CLI surface for that skill to repoint to (see Key
  Discoveries). This deviates from the AC's "removed... in the same change"
  language for these three scripts specifically; 0194 deletes them once it
  rewires `sync-work-items` onto its own `sync` command.
- Not wiring `work-item-file-dirty.sh`'s dirtiness guard into `update`'s
  control flow — ported and characterization-tested as a private function
  only; `update` performs an unconditional atomic write, matching the
  behaviour of every other write path in this story.
- Not reproducing `update-work-item`'s natural-language interpretation, diff
  preview, confirmation prompt, or body-label/H1 prose sync in Rust — these
  stay exactly as they are today in the skill; `accelerator work update`
  supplies only the atomic frontmatter write.
- Not adding transition enforcement to `status` or any other field —
  `update` accepts any value, matching the skill's existing "no transition
  enforcement" stance.
- Not building a generic multi-key line-surgical patcher extending
  `patch_status` — `document::render`'s whole-tree rewrite is used instead
  (see Key Discoveries).
- Not modifying `skills/work/scripts/EXIT_CODES.md` — it covers none of the
  12 scripts this story touches.

## Implementation Approach

Nine phases, each independently mergeable and each leaving `mise run` green.
Characterization fixtures come first, reusing `test-work-item-scripts.sh`'s
already-pinned assertions rather than re-deriving expected behaviour by eye.
The pattern-DSL completion (a small, mechanical extension of an existing
shared module) is isolated from the new `work` domain crate's own logic,
which in turn is built and unit-tested against hand-rolled doubles before any
binary exists — mirroring 0169 Phases 2-3. The five user-facing commands are
built as five separate vertical slices so each phase's diff is a complete,
working increment; the first slice (`resolve`) bundles the sub-binary
registration checklist's same-change points (1, 2, 3, 4, 7, 8), exactly as
0169's Phase 5 did for `vcs detect`. Skill repoint and shell deletion land
last, once every command exists.

---

## Phase 1: Capture Characterization Fixtures

### Overview

Pins the 11 scripts' current behaviour as goldens before any of them is
deleted, reusing `test-work-item-scripts.sh`'s and `test-work-item-pattern.sh`'s
already-verified assertions as the source of truth (see Key Discoveries)
rather than re-deriving expected values from reading source alone.

### Changes Required

#### 1. New golden fixture files

**Directory**: `skills/work/scripts/test-fixtures/` (existing directory,
already holds `work-item-next-number.golden`)

**New files**, one golden per script, each row produced by running the real
script and capturing output, cross-checked against the matching assertion(s)
already in `test-work-item-scripts.sh`. Two formats, not one — the existing
`work-item-next-number.golden` reader
(`test-work-item-scripts.sh:419-459`) parses `<setup>|<args>|<expected>`
where `<setup>` is a comma-separated list of bare filenames to `touch`;
that's sufficient only for goldens whose fixture state is "which filenames
exist" (`resolve-id`, `update-tags`, `canonicalise-id`, `file-dirty`'s
non-content cases). The remaining goldens need real file/stdin *content*,
not just filenames, so they use a second, richer format instead: one
fixture file per case under a per-script subdirectory (e.g.
`test-fixtures/work-item-section-diff/case-frontmatter-only/{local.md,
remote.md,expected.txt}`), read directly rather than reusing the
`touch`-only reader. This split applies to `section-diff`, `project-remote`
(stdin JSON), `normalise` (file content), and `template-field-hints`
(template content) — reusing the existing reader for these would require
inventing an escaping convention for embedding multi-line content in a
single pipe-delimited line, which the richer per-case-directory format
avoids entirely:

- `work-item-resolve-id.golden` — one row per classification arm (path,
  full_id, bare_number × 4 candidate sources, invalid) and per exit code
  (0/1/2/3). The AC's parity requirement for `resolve` is exit-code parity
  only ("the same exit codes for unrecognised/ambiguous/no-match input"),
  not stderr-text parity — the golden's ambiguous-match rows exist to pin
  the exit code and the *set* of candidates found, not bash's exact listing
  format. This matters because bash itself is inconsistent here: `bare_number`
  ambiguity lists `path [tag]` pairs with a source-category tag, while
  `full_id` ambiguity lists plain untagged paths
  (`work-item-resolve-id.sh:129-135` vs. `228-234`) — `ResolveOutcome::
  Ambiguous(Vec<TaggedCandidate>)` (Phase 3) applies the tagged shape
  uniformly to both for implementation simplicity, which is a permitted
  format difference from bash's `full_id` listing, not a parity gap.
- `work-item-read-field.golden` — present field, own-identity alias
  (`id`↔`work_item_id`), quoted/unquoted values, array-verbatim value,
  missing field, missing/unclosed frontmatter.
- `work-item-section-diff.golden` — frontmatter-only diff, preamble diff,
  named-heading diff, heading present on one side only, byte-identical
  (omitted) section, `(no differing sections after normalisation)` case,
  missing-argument usage error (exit 2), and non-file-argument error (exit
  2, `"both arguments must be files"`) — this pair is the script's only
  error paths (`work-item-section-diff.sh:73-82`).
- `work-item-update-tags.golden` — add (new/duplicate), remove
  (present/absent), block-style-tags rejection, quoting-needed tag values, and
  the naive-comma-split round-trip: adding/removing a tag when an existing
  tag is `"c,d"` (comma-quoted per `format_tag`), asserting the bash's actual
  (bug-preserving) re-split behaviour, not a "corrected" one.
- `work-item-file-dirty.golden` — every case already in
  `test-work-item-scripts.sh:1573-1640` (git dirty/clean/untracked, jj
  dirty/clean, jj-colocated, indeterminate, and the two real-worktree
  end-to-end cases), transcribed rather than re-derived.
- `work-item-normalise.golden` — `<file>` mode (IGNORE_KEYS stripped) and
  `--stdin` mode (trim-only), both from `test-work-item-scripts.sh:1218-1305`,
  plus missing-file (exit 1, `"no such file: ..."`) and bad-flag/no-argument
  usage errors (exit 1) — `work-item-normalise.sh:82-112`'s full error set —
  and a row with non-ASCII whitespace (e.g. U+00A0 no-break space) in a
  trimmed line, pinning the ASCII-only trim semantics from Phase 3, item 6.
- `work-item-template-field-hints.golden` — template-comment-present,
  template-comment-absent (hardcoded fallback), template-unreadable
  (hardcoded fallback), for `kind`/`status`/`priority`, plus the
  no-argument usage error (exit 1, `work-item-template-field-hints.sh:14-16`
  — the script's only non-zero-exit path).
- `work-item-project-remote.golden` — jira/linear `updated`, jira/linear
  `body` (including ADF key-order-independence), transcribed from
  `test-work-item-scripts.sh:1641-1661`, plus the unsupported-integration
  error (exit 2, `"unsupported integration: ..."`,
  `work-item-project-remote.sh:88-91`).
- `work-item-canonicalise-id.golden` — already-full-ID normalisation (bare,
  zero-padded, quoted), bare-number-under-`{project}`-pattern prepend (with
  and without a configured default project), parent-equality-via-canonicalise
  (`PROJ-0042` ≡ `42`), transcribed from
  `test-work-item-scripts.sh:501-541`.

`work-item-read-status.sh` needs no separate golden: it is a pure `exec`
delegate to `work-item-read-field.sh status`, covered by the `status` rows of
that golden.

#### 2. Confirm the existing parity gate is unaffected

**File**: `skills/work/scripts/test-work-item-pattern.sh` — unchanged in this
phase; remains the designated parity gate the AC names, repointed in Phase 9.

### Success Criteria

#### Automated Verification

- [x] `mise run test:integration:work` still passes unchanged: `_EXPECTED_WORK_SUITES`
      stays 6, `test-work-item-scripts.sh`'s existing assertions are untouched
- [x] Every new golden file parses under a stable, documented format —
      deviation: the simple goldens use the existing pipe-delimited shape
      but are consumed by new Rust table-driven tests (Phase 2/3), not
      the bash `<setup>|<args>|<expected>` reader, since that reader is
      itself deleted in Phase 9; the richer goldens use per-case
      directories exactly as this phase's Overview describes
- [x] Each golden row is cross-checked against a real invocation of the
      corresponding script at plan-implementation time (not merely transcribed
      from this plan's prose) — done by direct script execution against
      constructed fixtures (see fixture files' header comments); the two
      cases that need to prove they follow the same code path as a live
      failure mode without a safe way to force it live (`file-dirty`'s
      real-worktree end-to-end cases, `template-field-hints`'
      template-unreadable arm) are called out explicitly in the golden
      files' own comments rather than silently presented as live-checked

#### Manual Verification

- [x] Spot-check 2-3 rows per golden by eye against the corresponding section
      of `test-work-item-scripts.sh` to confirm no transcription drift —
      `file-dirty.golden`'s override cases transcribed verbatim from
      `test-work-item-scripts.sh:1573-1640`; `read-field`/`update-tags`
      cases cross-checked against `test-work-item-scripts.sh:675-1000`

---

## Phase 2: Complete the Pattern-DSL Port

### Overview

Extends the existing, already-parity-tested `corpus_adapters::work_item_pattern`
module and `corpus::WorkItemIdScheme` with the pieces `_wip_compile`'s other
modes and the legacy-ID helpers still need, so Phase 3's domain logic has a
complete Rust pattern-DSL to build on.

### Changes Required

#### 1. Format-mode compilation, refactored to share the token walker

**File**: `cli/corpus-adapters/src/work_item_pattern.rs`
**Changes**: introduce a private `Mode { Scan, Format }` and refactor
`compile_scan_regex`'s token-walking loop into a shared `compile(pattern,
project_value, mode) -> Result<String, PatternError>`, mirroring
`_wip_compile`'s own single-function, mode-driven design
(`work-item-common.sh:43-207`) rather than duplicating the loop. Add:

```rust
pub fn compile_format_string(
    pattern: &str,
    project_value: &str,
) -> Result<String, PatternError> { .. }
```

Port of `wip_compile_format` (`work-item-common.sh:231-237`, the `mode =
"format"` branch of `_wip_compile`): literal `%` escaped to `%%`, `{number:0Nd}`
→ `%0Nd`, `{project}` substituted verbatim (already escape-validated).
`compile_scan_regex` becomes a thin wrapper over `compile(.., Mode::Scan)`;
existing tests and the existing parity test (`cli/corpus-adapters/tests/
work_item_pattern_parity.rs`) must still pass unchanged.

#### 2. Pattern cap

**File**: `cli/corpus-adapters/src/work_item_pattern.rs`
**Changes**:

```rust
pub fn pattern_max_number(pattern: &str) -> Result<u64, PatternError> { .. }
```

Port of `wip_pattern_max_number` (`work-item-common.sh:239-265`): finds the
`{number:0Nd}` width (default 4 when bare `{number}`), returns `10^N - 1`.
Uses `10u64.checked_pow(n)`, returning `PatternError` on overflow (widths
around 20+ overflow `u64`) rather than panicking or silently wrapping —
pattern validation constrains the width's *shape* (`^0[1-9][0-9]*d$`) but
not its upper bound, so a misconfigured `work.id_pattern` with an
unreasonably large width must fail loudly, not compute a wrong cap.

#### 3. Full-ID parsing

**File**: `cli/corpus-adapters/src/work_item_pattern.rs`
**Changes**:

```rust
pub struct ParsedId {
    pub project: Option<String>,
    pub number: String,
}

pub fn parse_full_id(
    id: &str,
    pattern: &str,
) -> Result<ParsedId, PatternError> { .. }
```

Port of `wip_parse_full_id` (`work-item-common.sh:289-347`): builds a
capturing regex from the pattern (`{project}` → `([A-Za-z][A-Za-z0-9]*)`,
`{number...}` → `([0-9]+)`, literals escaped), matches `id` against it. Needed
by `work::resolve`'s classification (Phase 3).

#### 4. Legacy-ID helpers on the domain type

**File**: `cli/corpus/src/work_item_id.rs`
**Changes**: add to `WorkItemIdScheme`, pure and dependency-free (no regex
needed — these are digit-count/zero-pad predicates):

```rust
impl WorkItemIdScheme {
    #[must_use]
    pub fn is_legacy_id(id: &str) -> bool { .. }   // ^[0-9]{1,4}$ and >= one non-zero digit

    /// # Errors
    /// `None` when `input` is not all-ASCII-digit.
    #[must_use]
    pub fn pad_legacy_number(input: &str) -> Option<String> { .. } // zero-pad to 4
}
```

Ports of `wip_is_legacy_id`/`wip_pad_legacy_number`
(`work-item-common.sh:267-287`).

#### 5. ID canonicalisation

**File**: `cli/corpus-adapters/src/work_item_pattern.rs`
**Changes**:

```rust
pub fn canonicalise_id(
    input: &str,
    pattern: &str,
    project_value: &str,
) -> Result<String, PatternError> { .. }
```

Port of `wip_canonicalise_id` (`work-item-common.sh:356-425`): a thin
composition of `parse_full_id` (item 3, re-emitting zero-padded on a
successful parse) and `compile_format_string` (item 1, for the bare-number
fallback, requiring `project_value` only when the pattern has `{project}`) —
no new parsing logic of its own. This is the function `update-work-item` and
`list-work-items` shell out to directly today
(`skills/work/update-work-item/SKILL.md:149-151`,
`skills/work/list-work-items/SKILL.md:240-248`) to normalise `parent` values
before writing/comparing; exposed as `work canonicalise-id` in Phase 8 so
both skills have a CLI subcommand to repoint to once
`work-item-common.sh` is deleted in Phase 9.

### Success Criteria

#### Automated Verification

- [x] `cargo test -p corpus -p corpus-adapters --locked` passes, including new
      unit tests for every function above, table-driven against the golden
      rows captured in Phase 1 plus the inline examples read directly from
      `work-item-common.sh` (e.g. `compile_format_string("{number:04d}", "")
      == "%04d"`, `pattern_max_number("{number:05d}") == 99999`,
      `parse_full_id("PROJ-0042", "{project}-{number:04d}")` →
      `project: Some("PROJ"), number: "0042"`)
- [x] `canonicalise_id` matches every row in the Phase 1
      `work-item-canonicalise-id.golden`, plus the missing-project and
      unrecognised-shape error arms not covered by that golden
- [x] `pattern_max_number("{number:020d}")` (a width that overflows `u64`)
      returns `PatternError`, not a panic or a silently wrong value
- [x] The existing `compile_scan_regex` test suite and
      `work_item_pattern_parity.rs` still pass unchanged after the `Mode`
      refactor
- [x] `mise run cli:check` passes (rustfmt, clippy); `mise run pup:check`
      (cargo-pup) also passes — no new pup.ron rule was needed since this
      phase adds no new module boundary

#### Manual Verification

- [ ] None — this phase is pure, dependency-free logic fully covered by unit
      tests

---

## Phase 3: Core `work` Domain Logic

### Overview

Adds the pure `work` domain crate: resolve's candidate-search cascade,
next-number's allocation, section-diff's extraction/comparison, tag-array
mutation, template-hint-comment parsing, and the own-identity predicate — all
test-driven against hand-rolled doubles, no filesystem or regex dependency.

**Implementation-time correction to items 2 and 3 below**: the prose as
originally drafted said `resolve`'s classification and `allocate`'s
formatting/cap logic would reuse `corpus_adapters::work_item_pattern`'s
`compile_format_string`/`pattern_max_number`/`parse_full_id`. That is not
possible: `corpus_adapters` depends on the `regex` crate, and `work`'s own
`work_domain_imports_only_permitted` pup.ron rule (item 1) permits only
`std`/`kernel::Error`/`corpus`/`crate` — `corpus_adapters` is not on that
list, deliberately, and adding it would defeat the point of the rule this
same phase introduces. `resolve.rs` and `next_number.rs` therefore each
carry a small, regex-free, dependency-free equivalent (a greedy
literal/token walker for full-ID matching, and a local `{number:0Nd}`-width
parser for the numeric cap), and reuse the existing
`corpus::WorkItemIdScheme::canonicalise_id`/`pad_legacy_number` for
formatting wherever that already covers the need. This duplicates a small
amount of logic across the `corpus-adapters`/`work` boundary but keeps both
crates' own import-restriction rules intact rather than punching a hole in
either.

### Changes Required

#### 1. Crate scaffold

**File**: `cli/work/Cargo.toml` (new)
**Changes**: package `work`, `[dependencies] corpus = { path = "../corpus" }
kernel = { path = "../kernel" }`, `[lints] workspace = true`. No `[[bin]]` —
this is a library crate; `accelerator-work` (the binary) lives at
`cli/work-cli/` from Phase 4, exactly mirroring `vcs`/`vcs-cli`'s split
(avoiding the domain-crate-name-vs-dispatch-token trap the research flags:
`_SUBBINARY_MANIFESTS["work"]` will point at `cli/work-cli/Cargo.toml`, not
default-resolve to `cli/work/Cargo.toml`).

**File**: `cli/Cargo.toml`
**Changes**: add `"work"` to `[workspace].members` (ordinary crate
registration — no dispatch ceremony; that is Phase 4's concern for the
binary crate).

**File**: `cli/pup.ron`
**Changes**: add a `work_domain_imports_only_permitted` rule restricting
`work`'s imports to `std`/`kernel::Error`/`corpus`/`crate`, mirroring
`vcs_domain_imports_only_permitted` (`cli/pup.ron:75-89`).

#### 2. Resolve's candidate-search cascade

**File**: `cli/work/src/resolve.rs` (new)
**Changes**: port of `work-item-resolve-id.sh:65-241`.

```rust
pub enum InputClass { Path, FullId, BareNumber, Invalid }

pub fn classify_input(
    input: &str,
    scheme: &WorkItemIdScheme,
) -> InputClass { .. }

pub enum SearchClass { FullId, BareNumber }

pub struct TaggedCandidate { pub path: String, pub tag: String }

pub enum ResolveOutcome {
    Single(String),
    Ambiguous(Vec<TaggedCandidate>),
    NotFound,
}

pub trait DirectoryLister {
    fn filenames(&self) -> Vec<String>;
}

pub fn resolve(
    input: &str,
    class: SearchClass,
    scheme: &WorkItemIdScheme,
    lister: &dyn DirectoryLister,
) -> ResolveOutcome { .. }
```

Reproduces the exact four-source priority-ordered candidate build
(project-prepended, legacy ≤4-digit, no-project pattern-shape, cross-project
scan) and first-tag-wins de-duplication from
`work-item-resolve-id.sh:143-217`. `resolve` never touches a filesystem
directly — `DirectoryLister` is the injected port; `work-cli` (Phase 4)
implements it over `std::fs::read_dir`, tests here use a hand-rolled
in-memory double.

`resolve`'s `class` parameter is `SearchClass`, not `InputClass` — a
separate two-variant type covering only the two classes that actually need
the candidate-search cascade. `resolve`'s own pub fn signature is part of
0194's frozen library-dependency surface (see Desired End State), so its
precondition ("only `FullId`/`BareNumber`, `Path`/`Invalid` never reach
here") is enforced by the type system, not by a comment a future caller
could misread: there is no `SearchClass` value that maps to `Path` or
`Invalid`, so passing either is a compile error, not a documented-but-
unchecked runtime contract. `work-cli` (Phase 4) converts `classify_input`'s
`InputClass` into `SearchClass` only on the two arms that call `resolve`;
`Path`/`Invalid` are handled entirely before that conversion, as already
described below.

`resolve` takes `class` as a precondition, not something it derives itself,
and only handles `FullId`/`BareNumber` — its two callers, `Path` and
`Invalid`, need no candidate-search logic at all and are handled entirely by
`work-cli` (Phase 4) *before* `resolve` is ever called: `Path` is a plain
filesystem existence check plus canonicalisation (`std::fs::metadata` +
absolute-path construction, no search, no ambiguity possible), and `Invalid`
is an immediate exit-1 with no work to do. Forcing both through `resolve`'s
signature — as an earlier version of this plan did, returning
`Result<ResolveOutcome, InvalidInput>` — would have required either
smuggling a filesystem-existence check into `DirectoryLister` (a port
designed for listing `WORK_DIR`'s contents, not checking arbitrary paths) or
adding dead branches to `resolve` for input the CLI has already excluded by
construction. `resolve` is therefore infallible (no error type) since its
caller guarantees `class` is one of the two variants it actually handles;
passing `Path`/`Invalid` is a caller bug, not a runtime error to report.

#### 3. Next-number allocation

**File**: `cli/work/src/next_number.rs` (new)
**Changes**: port of `work-item-next-number.sh:94-140`.

```rust
pub enum AllocationError {
    MissingProject,
    ProjectUnused,
    Overflow {
        partial: Vec<String>,
        highest: u64,
        highest_file: Option<String>,
        cap: u64,
    },
}

pub fn allocate(
    scheme: &WorkItemIdScheme,
    project: Option<&str>,
    count: u32,
    filenames: &[String],
    scanner: &dyn corpus::IdScanner,
) -> Result<Vec<String>, AllocationError> { .. }
```

Scans `filenames` via the injected `IdScanner` (Phase 2's
`RegexScanner`/`compile_scan_regex` compose this at the `work-cli` boundary),
tracks the highest existing number, applies the overflow guard via
`pattern_max_number` (Phase 2), formats via `compile_format_string` (Phase 2).
`Overflow` carries `highest`/`cap` (not just `highest_file`) because the
bash's two distinct overflow messages both interpolate the numeric highest
value, and the choice between them turns on `highest > cap` vs. `highest <=
cap` (`work-item-next-number.sh:120-135`) — without these fields the CLI
layer (Phase 4) can't reproduce either message or pick the right one. The
domain function still only *carries* the data; formatting the exact bash
wording (`"out-of-width file '{highest_file}' has number {highest}
exceeding..."` vs. `"number space exhausted (highest={highest},
cap={cap})..."`) is the adapter's job, consistent with every other error
message in this plan.

#### 4. Section-diff extraction and comparison

**File**: `cli/work/src/section_diff.rs` (new)
**Changes**: port of `work-item-section-diff.sh:39-115`, minus the sha256
step (see Key Discoveries — direct string equality after normalisation is
behaviourally identical).

```rust
pub enum SectionName { Frontmatter, Preamble, Heading(String) }

pub fn heading_union(local: &[String], remote: &[String]) -> Vec<String> { .. }

pub fn extract_section(content: &str, name: &SectionName) -> String { .. }

pub struct SectionDiff { pub name: String, pub local: String, pub remote: String }

pub fn differing_sections(
    local_content: &str,
    remote_content: &str,
) -> Vec<SectionDiff> { .. }
```

`differing_sections` calls `extract_section` for each unioned section name on
both sides, trims via the `--stdin`-mode normalisation (item 6 below), and
keeps only sections whose trimmed content differs — reproducing the
`--stdin`-only-trims behaviour from Key Discoveries exactly (no `IGNORE_KEYS`
stripping on the frontmatter section).

#### 5. Tag-array mutation

**File**: `cli/work/src/tags.rs` (new)
**Changes**: port of `work-item-update-tags.sh:76-163`.

```rust
pub enum TagAction { Add, Remove }
pub enum TagMutation { Changed(String), NoChange }
pub enum TagError { BlockStyleTags }

pub fn mutate_tags(
    frontmatter_raw: &str,
    action: TagAction,
    tag: &str,
) -> Result<TagMutation, TagError> { .. }
```

Takes the *whole* raw frontmatter text, not just the already-extracted tags
value — this makes the block-style-before-parse ordering a structural
property of the function rather than a caller convention: `mutate_tags`
internally scans `frontmatter_raw` for a `tags:` line followed by an
indented `- ` continuation (block-style tags are precisely the shape a
`Yaml`-typed parse would represent differently, so this scan must run before
any parse is attempted) and returns `Err(BlockStyleTags)` immediately if
found, before ever extracting or parsing a tags value — so
`TagError::BlockStyleTags` is now reachable from within the function that
owns it, not just from an external pre-check a future caller could skip.
Only once that check passes does it extract the current tags value (via
`show::read_field_raw`, item 9 below) and reproduce the parse (`[a, b, "c,d"]` →
items via a **naive comma split**, matching
`work-item-update-tags.sh:76-88`'s `tr ',' '\n'` exactly), duplicate/absent
no-op detection, and quote-if-needed (comma/colon/hash) rebuild.
`TagMutation::Changed`'s `String` payload is the fully rebuilt canonical
array literal (`build_canonical`'s output shape, e.g. `[a, b, "c,d"]`) —
already flow-style formatted, ready to parse as a `Yaml` sequence and
splice into the mapping `--set`'s writes are applied to (Phase 8, item 3),
not a raw fragment needing further assembly.

**Preserved quirk, not a bug to fix**: the naive comma split does not
respect quoting, so an existing tag written as `"c,d"` (a shape
`format_tag`'s own quote-on-comma logic can produce) is mis-split into two
tokens (`c`, `d`) on the next add/remove — this is bash's actual behaviour
and is reproduced as-is, the same treatment Key Discoveries already gives
the un-stripped `IGNORE_KEYS` quirk in `section_diff`. `work-cli` supplies
the raw frontmatter text.

#### 6. Normalisation (trim-only, `--stdin`-equivalent)

**File**: `cli/work/src/normalise.rs` (new)
**Changes**: port of `_win_trim` (`work-item-normalise.sh:49-57`) —
per-line trim plus trailing-blank-line strip. Used by `section_diff` above.

```rust
pub fn trim_lines(content: &str) -> String { .. }
```

Must trim ASCII whitespace only (`char::is_ascii_whitespace`), not Rust's
default `str::trim()` (Unicode-aware, a broader class). The bash runs this
whole script under `LANG=C LC_ALL=C` — its own header comment calls this
"load-bearing for the committed, cross-machine baseline" — so
`[[:space:]]` under the C locale means ASCII whitespace exactly; a
Unicode-aware trim would normalise non-ASCII-whitespace content
differently across machines than the original script did, silently
undermining the byte-identical cross-machine guarantee 0194's sync baseline
depends on.

The full `<file>`-mode `IGNORE_KEYS`-stripping normaliser
(`work-item-normalise.sh:59-80`) is ported too, characterization-tested per
the AC, but consumed by none of this story's five commands (0194's own
sync-engine change-detection is its only future caller — see Key
Discoveries):

```rust
pub const IGNORE_KEYS: &[&str] =
    &["last_updated", "last_updated_by", "id", "external_id", "updated_at", "revision"];

pub fn filter_frontmatter_keys(frontmatter_raw: &str) -> String { .. }
```

#### 7. Template-hint-comment parsing

**File**: `cli/work/src/template_hints.rs` (new)
**Changes**: port of `work-item-template-field-hints.sh:52-88`.

```rust
pub fn hardcoded_fallback(field: &str) -> Vec<String> { .. } // kind/status/priority only

pub fn extract_hints(template_frontmatter: &str, field: &str) -> Vec<String> { .. }
```

`extract_hints` scans the raw template frontmatter text for the field's line
and its trailing `# a | b | c` comment, falling back to
`hardcoded_fallback` when the line or comment is absent — matching the bash's
own line-scan (not a `Yaml` parse, which would drop the comment).

#### 8. Own-identity predicate and field alias

**File**: `cli/work/src/own_identity.rs` (new)
**Changes**: port of `wip_is_work_item_file`
(`work-item-common.sh:448-477`) and the `id`↔`work_item_id` alias fallback
from `work-item-read-field.sh:86-100`, simplified to take already-extracted
values (pushing frontmatter parsing to the adapter boundary, not duplicating
it here):

```rust
#[must_use]
pub fn is_work_item_file(id: Option<&str>, work_item_id: Option<&str>) -> bool {
    id.is_some_and(|v| !v.is_empty()) || work_item_id.is_some_and(|v| !v.is_empty())
}

#[must_use]
pub fn own_identity_alias(field: &str) -> Option<&'static str> {
    match field {
        "id" => Some("work_item_id"),
        "work_item_id" => Some("id"),
        _ => None,
    }
}
```

Consumed by `show`'s alias fallback (item 9 below, formalised in Phase 5's
CLI wiring). `is_work_item_file` has no consumer among this story's five
commands — ported for the same future-0194-consumer reason as `file_dirty`
(Phase 8, item 7) and `project_remote` (Phase 6, item 3 — relocated to
`work-adapters` since its real logic is JSON extraction, not pure decision
logic; see that phase), not because any command in this story calls it yet.

#### 9. Raw field read (domain)

**File**: `cli/work/src/show.rs` (new)
**Changes**: port of `work-item-read-field.sh:53-79` as a pure string
function operating on already-extracted frontmatter text:

```rust
#[must_use]
pub fn read_field_raw(frontmatter: &str, field: &str) -> Option<String> { .. }
```

First-match-wins line scan for `<field>:` prefix, trims both ends, strips
one layer of surrounding `"`/`'`. Defined here rather than in Phase 5 (where
`work show`'s CLI wiring is built) because `work::tags::mutate_tags` (item
5, above) also needs it to extract the current tags value — putting it in
Phase 5 would make Phase 3 depend on a module two phases later, breaking
this plan's own "each phase independently mergeable" claim. Phase 5 adds
only the CLI wiring around this already-complete domain function; no new
domain logic is added there.

### Success Criteria

#### Automated Verification

- [x] `cargo test -p work --locked` passes unconditionally — every module's
      unit tests, table-driven against the Phase 1 goldens, using hand-rolled
      `DirectoryLister`/`IdScanner` doubles (no fixtures, no filesystem)
- [x] `resolve`'s four-candidate-source priority ordering and de-duplication
      is exercised with an in-memory filename list covering all four sources
      simultaneously, asserting the exact `Ambiguous` tag list and order
- [x] `allocate`'s overflow guard is exercised with a filename list at/over
      the cap, asserting the exact partial-emission-then-error behaviour from
      `work-item-next-number.sh:120-135`, covering **both** overflow arms
      distinctly: a stray out-of-width file (`highest > cap`) and ordinary
      number-space exhaustion (`highest <= cap`), asserting `Overflow`
      carries the right `highest`/`cap`/`highest_file` combination for each
- [x] `read_field_raw` is table-driven against the Phase 1
      `work-item-read-field.golden` (own-identity alias, quoted/unquoted
      values, array-verbatim value, missing field) — this is Phase 5's
      parity oracle, exercised here since the function now lives in this
      phase
- [x] `differing_sections` is exercised with a frontmatter-only change limited
      to `last_updated`, asserting the section is still reported as differing
      (confirms the Key Discoveries finding is reproduced, not silently
      "fixed")
- [x] `mutate_tags` given a block-style frontmatter returns
      `Err(TagError::BlockStyleTags)` without needing a separate pre-check
      call — asserting the check is now internal to the function, not a
      caller-side convention; a comma-containing existing tag (`"c,d"`) is
      re-split into two tokens on the next add/remove, matching the
      preserved-quirk decision in Key Discoveries exactly, not a "corrected"
      single-token result
- [x] `mise run cli:check` passes (rustfmt, clippy, cargo-pup); confirms
      `work` imports only `std`/`kernel::Error`/`corpus`/`crate`

#### Manual Verification

- [ ] None — pure logic, fully covered by unit tests

---

## Phase 4: First Vertical Slice — Crate Scaffold, Registration, and `work resolve`

### Overview

Scaffolds the `accelerator-work` binary crate, completes the sub-binary
registration checklist's same-change points, and implements `work resolve`
end to end — the command every other skill and every later phase needs first.

### Changes Required

#### 1. Adapters crate scaffold

**File**: `cli/work-adapters/Cargo.toml` (new)
**Changes**: package `work-adapters`, `version.workspace = true` (+ other
inherited fields), `[lints] workspace = true`. `[dependencies] work = {
path = "../work" }`, `kernel = { path = "../kernel" }` — a library crate (no
`[[bin]]`), mirroring `vcs-adapters`' shape exactly.

**File**: `cli/work-adapters/src/filesystem.rs` (new)
**Changes**: a `std::fs::read_dir`-backed implementation of
`work::resolve::DirectoryLister`, moved here rather than living inline in
`work-cli` — this is the crate's pure-read adapter module, mirroring
`vcs-adapters::library`'s role (no subprocess spawning).

**File**: `cli/pup.ron`
**Changes**: add a `work_adapters_filesystem_reads_in_process` rule
restricting `work_adapters::filesystem`'s imports to
`std`/`core`/`alloc`/`work`/`crate` and denying `std::process`, mirroring
`vcs_adapters_library_reads_in_process` — keeps the pure-read adapter module
isolated from the subprocess-shelling module Phase 6 adds
(`work_adapters::diff_shellout`).

**File**: `cli/Cargo.toml`
**Changes**: add `"work-adapters"` to `[workspace].members`.

#### 2. Binary crate scaffold

**File**: `cli/work-cli/Cargo.toml` (new)
**Changes**: package `accelerator-work`, mandatory `description`,
`version.workspace = true` (+ the other inherited fields), `[lints] workspace
= true`, `[[bin]] name = "accelerator-work" path = "src/main.rs"`. Depends on
`work`, `work-adapters`, `corpus`, `corpus-adapters`, `config`,
`config-adapters`, `document`, `kernel`, `clap` — thin wiring only, no
adapter implementation of its own, mirroring `vcs-cli`'s relationship to
`vcs-adapters`. `document` is needed directly here (not just via
`corpus-adapters`) because `create`/`update` (Phase 7/8) call
`document::parse`/`document::render`/`document::Yaml` from `work-cli`
itself — the whole-frontmatter rewrite is adapter-level work with no
existing crate performing it on `work-cli`'s behalf.

**File**: `cli/work-cli/src/cli.rs`, `src/main.rs` (new)
**Changes**: clap `Cli { command: Command }`; `Command::Resolve { input:
String }` for this phase (subsequent phases each add one variant). `main`
maps outcomes to the bash-documented exit codes: `ResolveOutcome::Single` →
0, path on stdout; `Ambiguous` → 2, candidates on stderr; `NotFound` → 3.
The two pre-`resolve()` branches (handled before `resolve` is ever called,
see below) get their own bash-matched codes, not a shared one: a
`Path`-class miss also exits **3** (`work-item-resolve-id.sh:99-112`'s
`E_RESOLVE_NOT_FOUND` — the same not-found signal as the search-cascade's
`NotFound`, not the invalid-input code), while `Invalid` class exits **1**
(`E_RESOLVE_INVALID`). `Cli` and every `Command` variant/field carry a
clap `#[command(about = "...")]`/`#[arg(help = "...")]` doc string from this
phase on — `accelerator work <verb>` is directly human-invocable (this
plan's own Manual Verification steps run it by hand), so `--help` at every
level (`work --help`, `work resolve --help`, etc.) must be a genuine entry
point, not an afterthought added once the CLI is otherwise complete. Every
later phase's `Command` variant follows the same convention.

**File**: `cli/work-cli/src/resolve.rs` (new)
**Changes**: `run(start: &Path, config: &dyn ConfigAccess) -> Result<..>`
wiring: repo-root discovery via `config_adapters::FileConfigStore::discover_root`,
`work.id_pattern`/`work.default_project_code` via
`ConfigAccess::effective_nonempty`, `paths.work` via
`config::paths::doc_type_dirs`, then calls `work::resolve::classify_input`
and branches on its result: `InputClass::Path` is resolved directly here
(`std::fs::metadata` existence check + absolute-path construction, exit 3 on
a miss — no domain call); `InputClass::Invalid` exits 1 immediately;
`InputClass::FullId`/`BareNumber` are converted to the corresponding
`SearchClass` variant and passed to `work::resolve::resolve` along with
`work_adapters::filesystem`'s `DirectoryLister` impl.

#### 3. Registration (checklist points 1, 2, 3, 4, 7, 8 — same change)

**File**: `tasks/shared/paths.py:29`
**Changes**: `DISPATCHED_SUBBINARIES = ("visualiser", "vcs", "work")`.

**File**: `tasks/manifest.py:52-61`
**Changes**: add `"work": CLI_DIR / "work-cli/Cargo.toml"` to
`_SUBBINARY_MANIFESTS` (the binary crate is not at `cli/work/` — that's the
domain crate).

**File**: `cli/Cargo.toml`
**Changes**: add `"work-cli"` to `[workspace].members` (alongside
`work-adapters` from item 1).

**File**: `tasks/build.py:35`
**Changes**: add `"accelerator-work"` to `_CLI_RELEASE_BINARIES`.

**File**: `tests/integration/tasks/test_github.py`
**Changes**: update the `DISPATCHED_SUBBINARIES` registry pin and
`_setup_release`'s staged fixture/in-test manifest per checklist point 1; the
`len(uploads)` assertion is already derived from `len(DISPATCHED_SUBBINARIES)`
(confirmed by 0169's own Key Discoveries) so needs no edit.

**File**: `skills/work/update-work-item/SKILL.md`
**Changes**: repoint Step 1's resolve invocation from
`work-item-resolve-id.sh` to `${CLAUDE_PLUGIN_ROOT}/bin/accelerator work
resolve <argument>`; add `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator work *)`
to `allowed-tools` (broad enough to cover every verb this plan adds through
Phase 8, added once here). This is the one skill binding the checklist's
point 7 requires; no new injection context is added, so
`EXPECTED_INJECTION_SKILLS` needs no update.

**File**: `.gitignore`
**Changes**: `bin/work-*` already matched by the existing token-generic
`bin/*.minisig`/`**/bin/*.debug.tar.gz` entries — confirm no new entry is
needed (checklist point 5's "no action" case, since the cached-binary glob is
already token-generic).

**Checklist point 11 (user-facing documentation)**: no action, matching the
precedent this plan otherwise mirrors closely — `accelerator-vcs` (the
`vcs` sub-binary) is directly human-invocable (this plan's own manual
verification steps show `accelerator work resolve`/`create` run by hand
too) but has no docs-site page, README Concepts-list entry, or
`ACCELERATOR_VCS_BIN` override row either. Primary usage for both
sub-binaries is skill-driven, not direct human discovery, so this plan
treats `work` the same tolerated-gap way rather than introducing new
documentation scope this story's sibling doesn't carry.

### Success Criteria

#### Automated Verification

- [ ] `accelerator work --help` and `accelerator work resolve --help` print
      a non-empty description for the top-level command and for `resolve`'s
      `input` argument — a smoke check that clap doc strings are present,
      not placeholder-empty
- [ ] `cargo test -p accelerator-work --locked` passes: exit-code mapping for
      all four `InputClass` outcomes, exercised against a real temp directory
      (not domain-level doubles — this is the adapter/binary boundary test):
      `Path` resolved directly by `resolve.rs` without calling
      `work::resolve::resolve`, exiting **3** (not 1) on a miss; `Invalid`
      rejected immediately with exit 1; `FullId`/`BareNumber` routed through
      `work::resolve::resolve`
- [ ] `cargo test -p work-adapters --locked` passes: the `DirectoryLister`
      impl in `filesystem.rs` is exercised against a real temp directory
- [ ] `mise run cli:check` passes; `cargo build --locked` produces
      `accelerator-work`; cargo-pup confirms `work_adapters::filesystem`
      imports only `std`/`core`/`alloc`/`work`/`crate` and never
      `std::process`
- [ ] `mise run lint:dispatch-coherence:check` (or the task it rolls into)
      passes — confirms the `work` token is valid, unreserved, and not
      builtin-shadowed
- [ ] `tests/integration/tasks/test_github.py` passes with the updated
      registry pin
- [ ] `mise run lint:skills:check` (or equivalent skill-permissions gate)
      passes for the repointed `update-work-item` skill

#### Manual Verification

- [ ] `ACCELERATOR_WORK_BIN=$(pwd)/cli/target/debug/accelerator-work
      ${CLAUDE_PLUGIN_ROOT}/bin/accelerator work resolve 1` resolves correctly
      in a scratch work directory, dispatched through the real launcher
      override path

---

## Phase 5: Second Vertical Slice — `work show`

### Overview

Implements `work show <path> [--field NAME]`, reproducing
`work-item-read-field.sh`/`work-item-read-status.sh`'s raw first-match-wins
line-scan byte-for-byte (not a `Yaml`-typed parse, which would lose the
array-verbatim/quote-stripped-only output contract the AC requires parity
with).

### Changes Required

#### 1. CLI wiring

**File**: `cli/work-cli/src/cli.rs`
**Changes**: add `Command::Show { path: PathBuf, field: Option<String> }`.

**File**: `cli/work-cli/src/show.rs` (new)
**Changes**: without `--field`, print the file verbatim (matching "prints the
full rendered item"). With `--field NAME`, distinguish missing-file /
no-frontmatter / unclosed-frontmatter (matching
`work-item-read-field.sh:35-51`'s exact error phrasing) before calling
`work::show::read_field_raw` (Phase 3, item 9 — no new domain logic added in
this phase, only the adapter/binary wiring around an already-complete and
already-tested function); on a miss, try `own_identity_alias(NAME)` (Phase
3, item 8) then `read_field_raw` again; exit 1 with the exact bash error
message on a final miss.

**Exit-code contract**: 0 on success (either the full-file print or a
successful `--field` lookup); 1 for missing-file, no-frontmatter,
unclosed-frontmatter, or a field miss after the alias fallback — matching
`work-item-read-field.sh`'s own single-failure-code behaviour (it has no
usage-error or ambiguity code distinct from a plain miss).

### Success Criteria

#### Automated Verification

- [ ] `read_field_raw`'s own unit tests are already covered by Phase 3's
      success criteria (it's defined there); this phase adds no new domain
      tests, only adapter/binary-boundary ones below
- [ ] `cargo test -p accelerator-work --locked` — `work show <path>` (no
      flag) prints the file byte-for-byte; `work show <path> --field status`
      matches `work-item-read-status.sh`'s output for the same fixture;
      missing-file/no-frontmatter/unclosed-frontmatter error messages match
      the bash originals verbatim; every exit code matches the contract
      stated above (0 success, 1 every failure)
- [ ] `mise run cli:check` passes

#### Manual Verification

- [ ] None — fully covered by golden-driven automated tests

---

## Phase 6: Third Vertical Slice — `work diff`

### Overview

Implements `work diff <local> <remote>`, using Phase 3's `section_diff`
module for extraction/comparison and shelling the real `diff -u` per
differing section for rendering — the same "shell the real tool for
byte-identical output" adapter pattern 0169 established for `vcs status`/`vcs
log`.

### Changes Required

#### 1. Diff-shellout adapter

**File**: `cli/work-adapters/Cargo.toml`
**Changes**: add `tempfile = { workspace = true }` to `[dependencies]`; add a
`bash-parity = []` feature gating the real-`diff`-binary test below, mirroring
`vcs-adapters`' own `bash-parity` feature (not `vcs-cli`'s — the feature
lives on the crate that owns the subprocess dependency).

**File**: `cli/work-adapters/src/diff_shellout.rs` (new)
**Changes**: for a given `SectionDiff`, writes `LOCAL`/`REMOTE` into a
`tempfile::TempDir` (RAII-cleaned, replacing the bash's `mktemp -d` + `EXIT`
trap) and shells `diff -u LOCAL REMOTE`, reproducing
`work-item-section-diff.sh:104-109`'s exact header format (`=== %s (- LOCAL
/ + REMOTE) ===`). This is the crate's one subprocess-spawning module,
isolated from `filesystem.rs`'s pure reads by the
`work_adapters_filesystem_reads_in_process` pup.ron rule from Phase 4 (which
restricts the *other* module, not this one — this module is where
`std::process` is expected). If spawning `diff` fails (`std::io::Error`,
e.g. `NotFound` when the binary isn't on `PATH`), `render` returns that
error rather than panicking; the CLI layer (item 2) maps it to a clear
"`diff` is required on PATH for `work diff`; install it or check PATH"
message on exit 1, rather than surfacing a raw I/O error to the user. Bounds
the spawned process with the same capped-wait shape `vcs-adapters::
subprocess::run_capped` already provides for `vcs status`/`vcs log`
(default 10s, poll-and-kill on timeout) — reusing that helper if it's
exposed generically enough, or a local equivalent otherwise — so a wedged
`diff` (or a non-standard shim earlier on `PATH`) can't hang `work diff`
indefinitely, matching the precedent's full robustness, not just its
happy-path shellout shape.

**File**: `cli/pup.ron`
**Changes**: none. `work_adapters_filesystem_reads_in_process`'s `Module`
matcher already targets only `work_adapters::filesystem`, so it has no
effect on this new `diff_shellout` module — that scoping is exactly what
keeps the subprocess-spawning code here from tripping the filesystem
module's `std::process` restriction.

#### 2. CLI wiring

**File**: `cli/work-cli/Cargo.toml`
**Changes**: add a bare `bash-parity = []` feature (independent of
`work-adapters`' own flag of the same name — `vcs-cli`/`vcs-adapters` follow
the identical two-independent-features shape, coordinated only by CI's
`--all-features` run, not a Cargo feature dependency).

**File**: `cli/work-cli/src/cli.rs`
**Changes**: add `Command::Diff { local: PathBuf, remote: PathBuf }`.

**File**: `cli/work-cli/src/diff.rs` (new)
**Changes**: reads both files, calls
`work::section_diff::differing_sections`, and for each result calls
`work_adapters::diff_shellout::render`, printing its output, with the
`(no differing sections after normalisation)` fallback when nothing differs.
Exit-code contract, matching `work-item-section-diff.sh:73-82`'s own arms
plus one case bash never needed to handle: 0 on success (whether or not any
sections differ — a non-empty diff is not a failure), 2 on missing-argument
usage error or a non-file argument (matching the bash exit code for both
arms exactly), 1 if the `diff` binary itself cannot be spawned (item 1's
`diff_shellout::render` failure) — a runtime-environment failure, not a
usage error, so it gets its own code rather than overloading 2.

#### 3. Remote-body projection (relocated from the domain crate, characterization only)

**File**: `cli/work-adapters/Cargo.toml`
**Changes**: add `serde_json = { workspace = true }` to `[dependencies]`.

**File**: `cli/work-adapters/src/project_remote.rs` (new)
**Changes**: port of `work-item-project-remote.sh:39-106`. This module lives
in `work-adapters`, not the `work` domain crate — an earlier version of this
plan put it in `work` typed directly against `serde_json::Value`, which
neither `work`'s declared dependencies nor its
`work_domain_imports_only_permitted` pup.ron rule permit, and which
contradicts the crate-local-type treatment (`FieldValue`, Phase 7) the plan
already applies to keep third-party value types out of the domain crate.
`project_remote`'s real work — JSON field extraction plus a `jq -cS`-
equivalent canonicalisation of the Jira ADF description field (`jq -cS`:
compact, key-sorted; `serde_json::to_string` over a value built from a
`BTreeMap`-backed `serde_json::Map` gives the same key-sorted, whitespace-free
output) — is adapter-shaped I/O-adjacent logic, not a domain decision, so
relocating it here (rather than inventing an artificial pure/impure split
purely to keep it in `work`) matches this plan's Phase 8 precedent of not
manufacturing a domain abstraction where the underlying work is mechanical:

```rust
pub enum Integration { Jira, Linear }
pub enum Op { Updated, Body }

pub fn project(
    integration: Integration,
    op: Op,
    remote_json: &serde_json::Value,
) -> String { .. }
```

Reproduces the bash's field paths exactly (`jira`/`updated`:
`.fields.updated // ""`; `jira`/`body`: `.fields.summary // ""` plus
`.fields.description // null` canonicalised via the `jq -cS` equivalent
above; `linear`/`updated`: `.data.issue.updatedAt // ""`; `linear`/`body`:
`.data.issue.title // ""` plus `.data.issue.description // ""` verbatim,
no canonicalisation — matching bash's own asymmetry between the two
tracker's body handling) — always infallible, since bash's own `// ""`/
`// null` defaults mean a missing field is never an error, only an empty
result; there is no `MissingField` case to model (an earlier version of
this plan invented one that doesn't correspond to real bash behaviour).
`--integration <string>` parsing into `Integration` (erroring with the
bash's exact `"unsupported integration: ..."` message, exit 2, for an
unrecognised value) happens at the CLI/adapter boundary, not in this
function — matching how every other bash-message-formatting responsibility
in this plan stays at the adapter layer.

Characterization-tested against the Phase 1 `work-item-project-remote.golden`
but not called from any of this story's five commands — its only consumer
is `sync-work-items`' bidirectional diff/apply flow, which today shells out
to the bash script directly (see "What We're NOT Doing") and stays that way
until 0194 owns the sync engine and can depend on this crate as a library.

### Success Criteria

#### Automated Verification

- [ ] `cargo test -p work-adapters --locked` — `project` matches every row
      in the Phase 1 `work-item-project-remote.golden`, including the
      ADF-canonicalisation-independent-of-key-order case; the
      unsupported-integration parse error (tested at the adapter boundary
      where `--integration` is parsed, not in `project` itself, which is
      infallible)
- [ ] `cargo test -p work-adapters --locked --features bash-parity` (gating
      this test on the real `diff` binary being on `PATH`, mirroring
      `vcs-adapters`' own `bash-parity` convention) — `diff_shellout::render`
      compares against the Phase 1 `work-item-section-diff.golden` fixtures
      byte-for-byte
- [ ] `cargo test -p accelerator-work --locked --features bash-parity` — the
      full `work diff` CLI path (section selection via `work::section_diff`,
      output assembly, the no-differing-sections fallback) is exercised
      end-to-end against the same golden fixtures
- [ ] The frontmatter-only-`last_updated`-change fixture (Phase 3's
      characterization test) is re-exercised at the CLI boundary, confirming
      the un-stripped-`IGNORE_KEYS` behaviour survives end to end
- [ ] `work diff`'s exit codes match the contract stated above: 0 for both
      the differences-found and no-differences cases, 2 for a missing
      argument and for a non-file argument, 1 for a simulated missing-`diff`
      environment (e.g. `PATH` overridden in the test), with the clear
      "`diff` is required on PATH..." message on stderr rather than a raw
      I/O error
- [ ] The capped-wait behaviour is exercised directly, not just documented
      in prose: substitute a deliberately slow/blocking script in place of
      `diff` on `PATH` in the test and confirm `diff_shellout::render`
      still terminates (with a clear timeout error, not a hang) within a
      bounded time — proving the reused/local capped-wait wrapper is
      actually wired around this spawn, not just described as intended
- [ ] `mise run cli:check` passes

#### Manual Verification

- [ ] None — `diff -u`'s output is deterministic and covered by the golden
      comparison

---

## Phase 7: Fourth Vertical Slice — `work create`

### Overview

Implements `work create`, composing next-number allocation, artifact-metadata
derivation, and a fresh whole-frontmatter write via `document::render` — the
most complex command, since no bash script currently performs this write (the
`create-work-item` skill does it today via direct frontmatter assembly, not a
dedicated script).

### Changes Required

#### 1. CLI surface

**File**: `cli/work-cli/src/cli.rs`
**Changes**: add

```rust
Command::Create {
    title: String,
    kind: String,
    priority: String,
    #[arg(long, default_value = "draft")]
    status: String,
    #[arg(long)]
    parent: Option<String>,
    #[arg(long = "tag")]
    tags: Vec<String>,
    #[arg(long = "block")]
    blocks: Vec<String>,
    #[arg(long = "blocked-by")]
    blocked_by: Vec<String>,
    #[arg(long = "derived-from")]
    derived_from: Vec<String>,
    #[arg(long = "relates-to")]
    relates_to: Vec<String>,
    #[arg(long)]
    source: Option<String>,
    #[arg(long)]
    project: Option<String>,
    #[arg(long)]
    author: Option<String>,
    #[arg(long, default_value = "accelerator-work")]
    producer: String,
    #[arg(long = "body-file")]
    body_file: Option<PathBuf>,
},
```

Every optional typed-linkage slot the template declares (`parent`, `blocks`,
`blocked_by`, `derived_from`, `relates_to`, `source`) now has a flag — an
earlier version of this plan covered only `parent`/`tags`, which would have
silently dropped the other five whenever a caller supplied them.
`--producer` defaults to `accelerator-work` for direct CLI use but lets a
calling skill pass its own name through (`create-work-item`,
`extract-work-items`, ...), preserving today's per-item provenance
convention (`producer: create-work-item`) instead of overwriting it with
the low-level tool's name. `--body-file` is new: without it, the body is
the template's H1 + section skeleton with the literal placeholder `NNNN`
substituted with the allocated ID; with it, the file's content is used as
the body **after the same `NNNN` → allocated-ID substitution is applied to
it too** — this is how a calling skill passes a fully-drafted body
(Summary, Requirements, Acceptance Criteria, etc.) through, since no other
channel on this command can carry that content.

**Why the caller passes a placeholder, not a pre-resolved ID**: `work
create` always self-allocates its own ID internally (item 4, lock-protected
via the Collision guard below) — there is no `--id` override. If the
calling skill instead pre-substituted a *previewed* ID (from an earlier,
separate `work next-number` call) into its drafted body before passing it
via `--body-file`, a race between that preview and `work create`'s own
allocation could leave the written file internally inconsistent — the
frontmatter's `id:` (from `work create`'s allocation) disagreeing with the
body's H1 line (from the skill's stale preview) — a silent identity defect
in the persisted artifact, not just a display glitch. Requiring the literal
token `NNNN` in the draft (matching `templates/work-item.md`'s own
placeholder convention) and having `work create` perform the substitution
itself, using the same ID it just allocated under its own lock, makes the
ID's origin a single source of truth by construction: there is no
allocate-elsewhere-then-reconcile step for a race to land in.

The `create-work-item` skill continues to drive interactive elicitation
(title/kind/priority/etc.) and passes the gathered values as flags — this
command is the low-level atomic-creation primitive, matching
`work-item-next-number.sh`'s own role (the skill orchestrates, the script/
command executes).

**Skill repoint, scoped narrowly**: `create-work-item/SKILL.md`'s Step 5
today calls `work-item-next-number.sh` directly (`SKILL.md:414-418`) to
preview the ID for display and to substitute into the drafted body's H1
line before writing. Under this repoint, the skill keeps the preview call
(repointed generically to `work next-number` per Phase 9's pass, used only
to *show* the user a likely ID during confirmation) but stops substituting
that previewed value into the body it drafts — the draft keeps the literal
`NNNN` placeholder throughout (H1 and any other ID-bearing text), matching
`templates/work-item.md`'s own placeholder convention, so the previewed ID
is cosmetic only and never becomes the ID actually written. The skill does
its *frontmatter population and Write* by hand today (`SKILL.md:444-500`),
not via a single create-style call, producing a fully-drafted body
(Summary, Requirements, Acceptance Criteria, Assumptions, Open Questions,
Drafting Notes, References — everything the user approved across Steps
1-4), not a bare template skeleton. Its "no integration configured" branch
(`SKILL.md:499`, "Write the file now with the substituted frontmatter
block") repoints to `accelerator work create ... --body-file <tmp>` — the
skill writes its already-rendered, `NNNN`-still-placeholder draft body to a
scratch file and passes it through `--body-file`; `work create` performs
the one real `NNNN` → allocated-ID substitution as part of the same atomic
write that decides the ID, so the approved content survives the repoint
with no possibility of an id/H1 mismatch (see the "Why the caller passes a
placeholder" note above). Its integration-configured branch (`SKILL.md:497-498`,
`564-566`), though, holds the write until a remote push-state machine
resolves (`external_id` substitution, retryable-transport fallback states)
— logic this story explicitly does not implement (`create` has no
`--push`, per "What We're NOT Doing"). Repointing *only* the no-integration
branch to `accelerator work create` here, while leaving the
integration-configured branch's manual `Write` untouched until 0194 adds
`--push`, is the safe boundary; forcing the integration branch onto `work
create` now would either drop the retry-state handling or require
inventing scope this story doesn't own. Confirm this split holds by
re-reading `create-work-item/SKILL.md`'s full Step 5-6 flow at
implementation time — it is described here from the skill's current prose,
not verified against a scratch run.

#### 2. Frontmatter composition (domain)

**File**: `cli/work/src/create.rs` (new)
**Changes**: a pure function deciding *what* the new frontmatter contains,
taking every already-resolved input as a plain value (no filesystem, no
`document`, no VCS access — consistent with every other Phase 3 module):

```rust
pub struct TypedLinkage<'a> {
    pub parent: Option<&'a str>,
    pub blocks: &'a [String],
    pub blocked_by: &'a [String],
    pub derived_from: &'a [String],
    pub relates_to: &'a [String],
    pub source: Option<&'a str>,
}

pub struct CreateInputs<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub kind: &'a str,
    pub priority: &'a str,
    pub status: &'a str,
    pub linkage: TypedLinkage<'a>,
    pub tags: &'a [String],
    pub author: &'a str,
    pub producer: &'a str,
    pub date: &'a str,
}

pub enum FieldValue { Scalar(String), Sequence(Vec<String>) }

pub fn compose_frontmatter(
    inputs: &CreateInputs<'_>,
) -> Vec<(String, FieldValue)> { .. }
```

Builds the field list against `templates/work-item.md`'s real schema —
`type`, `id`, `title`, `date`, `author`, `producer`, `status`, `kind`,
`priority`, the six typed-linkage slots (`parent`/`source` only when given,
`blocks`/`blocked_by`/`derived_from`/`relates_to`/`tags` only when
non-empty, all omitted otherwise per the template's own omit-when-empty
convention), then `last_updated`/`last_updated_by` (identical to
`date`/`author` at creation time — a freshly created item's last-updated
timestamp and editor are its creation timestamp and author, the same
convention `create-work-item`'s own Step 5.2 already follows) and
`schema_version: 1` unconditionally, since the template declares those
three without an omit-when-empty comment. `CreateInputs` deliberately has
no `revision`/`repository` fields — an earlier version of this plan
included them (copied from `ArtifactMetadata`'s shape for *plan*/*research*
producers), but `templates/work-item.md`'s frontmatter has no `revision`/
`repository` keys at all (confirmed: zero matches for either key across
every file in `meta/work/`), so item 4's `derive_at` call only needs its
`datetime_utc` output, not `revision`/`repository_name`. This field list is
built as an ordered key/value list using `FieldValue`, a small crate-local
type — not `document::Yaml` directly, since `work` stays within its
existing `std`/`kernel::Error`/`corpus`/`crate` import restriction (Phase
3's `work_domain_imports_only_permitted` rule); converting `FieldValue`
entries into a `document::Yaml` mapping is the adapter's job (item 4).
Unit-tested with every combination of optional-field presence/absence,
table-driven, no filesystem involved.

#### 3. Template-schema drift guard (domain)

**File**: `cli/work/src/create.rs`
**Changes**:

```rust
pub const KNOWN_FRONTMATTER_KEYS: &[&str] = &[
    "type", "id", "title", "date", "author", "producer", "status", "kind",
    "priority", "parent", "blocks", "blocked_by", "derived_from",
    "relates_to", "source", "external_id", "tags", "last_updated",
    "last_updated_by", "schema_version",
];

pub fn assert_matches_template_schema(
    template_frontmatter_keys: &[String],
) -> Result<(), SchemaDriftError> { .. }
```

A structural cross-check, not a behavioural port, and deliberately
**invocation-independent**: `KNOWN_FRONTMATTER_KEYS` is the fixed, complete
set of keys `compose_frontmatter` knows how to produce across every
optional-field combination (not the subset any one `CreateInputs` value
happens to populate — the earlier version of this plan compared against a
single invocation's `composed_keys`, which would spuriously fail for the
common no-`--parent`/no-`--tag` case since those keys are legitimately
absent from *that* invocation's output without the schema having drifted
at all). `assert_matches_template_schema` compares this constant against
`templates/work-item.md`'s own declared frontmatter keys (parsed by the
adapter, see item 4) as two complete sets — so a future template edit
(a renamed, added, or removed key) is caught by CI regardless of which
fields any particular `work create` call happens to supply. `external_id`
is in `KNOWN_FRONTMATTER_KEYS` for schema completeness even though `create`
never writes it (see below).

#### 4. Creation flow (adapter/binary)

**File**: `cli/work-adapters/src/author.rs` (new)
**Changes**:

```rust
#[must_use]
pub fn current_vcs_user() -> Option<String> { .. }
```

Shells `jj config get user.name` (if the repo is jj-colocated, matching
`vcs`'s own jj-present-WINS mode resolution) or `git config user.name`
otherwise, trimming the result; `None` on any failure (command not found,
non-zero exit, empty output) rather than erroring — there is no existing
Rust module to reuse here (grep-confirmed: no production code in
`cli/vcs`/`cli/vcs-adapters` reads `user.name`; `create-work-item`'s own
"config, then VCS identity, then ask the user" chain is a skill-level,
conversational flow with no Rust equivalent), so this is new, self-contained
work — small enough that inventing a shared abstraction with `vcs`/
`vcs-adapters` isn't warranted. Lives in `work-adapters` (not `work`) since
it shells a subprocess, matching `diff_shellout`'s precedent for where
subprocess-spawning code belongs.

**File**: `cli/work-cli/src/create.rs` (new)
**Changes**: allocates the next ID (`work::next_number::allocate`, count 1),
derives `datetime_utc` (`corpus_adapters::metadata::derive_at`, using only
this one field of `ArtifactMetadata` — `revision`/`repository_name` don't
apply to work items, see item 2), resolves `author` from `--author` if
given, else `work_adapters::author::current_vcs_user()`, else a hard error
("author could not be resolved: pass --author or run inside a git/jj
repository with user.name configured") — `work create` is a non-interactive
CLI, so unlike the interactive skill's chain it cannot fall back to asking
the user; callers that need the skill's full config-then-VCS-then-ask chain
(e.g. `create-work-item`) resolve `author` themselves and pass it via
`--author` explicitly, exactly as they already resolve every other flag
value before calling this command — resolves the `work-item` template via
`config::ReadTemplate::resolve_template` and parses its frontmatter keys via
`document::parse` (feeding `work::create::assert_matches_template_schema`
once, independent of any particular invocation), calls
`work::create::compose_frontmatter` with the assembled `CreateInputs`,
converts the returned key/value list into a `document::Yaml` mapping, and
calls `document::render(None, &yaml)` — which produces the frontmatter
block *only*; `render`'s `existing` parameter has no way to carry a fresh
body (`None` yields an empty body per `cli/document/src/render.rs:19-28`),
so the adapter concatenates the frontmatter block it returns with the
separately-resolved body text (below) before the single
`corpus_adapters::FileCorpusStore::write` call — after confirming the
target path does not already exist (see the collision-guard note below;
`create`, unlike `update`, must never silently overwrite).

**Body text**: takes `--body-file`'s content when given, or the resolved
`work-item` template's body verbatim otherwise — either way, the resulting
text still contains the literal placeholder `NNNN` (in the fallback case,
from the template itself; in the `--body-file` case, because the caller is
required to leave it there — see the CLI surface note above on why). The
adapter then substitutes every `NNNN` occurrence with the ID `allocate`
just returned, and the title placeholder with `--title`'s value, as one
pass over whichever body text was selected — not two separate substitution
paths for the two sources, so the same code performs the ID hand-off
whether the caller supplied a body or not.

`external_id` is never written by `create` (nothing to sync yet) — omitted
entirely, matching the template's "omit when not linked" comment.

**Collision guard**: `work create`'s allocate-then-write sequence has no lock
between the scan (`work::next_number::allocate`) and the write, and
`FileCorpusStore::write` itself provides no mutual exclusion — it calls
`store::atomic_write`, whose `persist()` step is an unconditional
`fs::rename` with no `O_EXCL`/no-clobber semantics (confirmed by the
existing `a_write_through_the_port_replaces_existing_content` test in
`cli/corpus-adapters/src/store.rs`, which asserts a pre-existing target is
silently overwritten). A bare pre-write existence re-check would only
narrow the race window, not close it, and would risk being read as a
correctness guarantee it isn't. Instead, acquire the codebase's existing
mkdir-lock primitive (`corpus_adapters::lock::acquire`, the same one
`FileCorpusStore::append_record`/`remove_by_key` already use, over a
work-dir-scoped sentinel — e.g. `<paths.work>/.accelerator-work-create.lockdir`,
since the contested resource is "the next ID in this directory," not any
one target file that doesn't exist yet) around the whole scan-allocate-write
sequence, releasing it (via `LockGuard`'s `Drop`) only after the write
completes. This gives genuine mutual exclusion between concurrent `work
create` invocations, not a best-effort narrowing — matching (and reusing,
not reinventing) the codebase's own established primitive for this exact
class of problem, and matching `sync-work-items`' existing requirement
(`SKILL.md:313`) to "abort the batch on an unexpected collision rather than
overwriting" in spirit, with a stronger mechanism than that skill's own
best-effort pre-check. This guarantee is scoped to callers that actually go
through `work create`: `create-work-item`'s integration-configured branch
(left on its manual `Write` path per the Skill repoint note above) doesn't
participate in this lock and remains subject to the pre-existing
unlocked-allocation race until 0194 migrates that branch too — "genuine
mutual exclusion" describes `work create`'s own callers, not every path
that can still allocate an ID in this codebase.

**Exit-code contract**: `create` has no bash original to inherit exit codes
from (Overview), so this plan defines one explicitly rather than leaving it
implicit: 0 on success (path of the new file on stdout); 1 for every
domain-level failure — `AllocationError` (any variant), the collision guard
above, an unresolvable `author` (item 4's hard error above), or a
template-resolution failure. No richer exit-code space is
needed (unlike `resolve`'s four-way split): every failure here is a single
"could not create" outcome from the caller's point of view, and 0194's
`--push` wiring (the one thing depending on this command's contract) only
needs to distinguish success from failure, not failure *kind*, at the exit
code level — failure kind is conveyed by the stderr message.

### Success Criteria

#### Automated Verification

- [ ] `cargo test -p work-adapters --locked` — `current_vcs_user` returns
      `Some(name)` in a fixture jj/git repo with `user.name` configured,
      and `None` (not an error) outside a repo or with no `user.name` set
- [ ] `cargo test -p work --locked` — `compose_frontmatter` is table-driven
      against every combination of optional-field presence/absence (parent
      given/omitted, tags empty/non-empty), asserting the exact key set and
      omit-when-empty behaviour, with no filesystem or `document` dependency
- [ ] `cargo test -p work --locked` — `assert_matches_template_schema` fails
      when given a template key list `compose_frontmatter` doesn't produce,
      and passes against the real `templates/work-item.md`'s current key set
      (this second case is the drift guard: it fails the moment the template
      and `compose_frontmatter` diverge)
- [ ] `cargo test -p accelerator-work --locked` — a fresh `work create`
      invocation produces a file whose frontmatter parses back via
      `document::parse` into exactly the fields supplied plus the derived
      ones, with every typed-linkage slot the AC's "fully populated
      frontmatter" language requires present; a second invocation in the same
      directory allocates the next sequential ID, not a duplicate
- [ ] `--block`/`--blocked-by`/`--derived-from`/`--relates-to`/`--source`
      each populate their corresponding frontmatter key when given and are
      omitted when not, matching `parent`/`tags`' existing omit-when-empty
      treatment
- [ ] `--body-file <path>` writes the file's content as the body, including
      multi-section content (Summary, Requirements, Acceptance Criteria,
      ...), with every literal `NNNN` occurrence in that content replaced
      by the actual allocated ID — not verbatim, and not just a title/H1
      substitution; without `--body-file`, the template-skeleton fallback
      gets the identical substitution treatment via the same code path
- [ ] The frontmatter's `id:` and every `NNNN`-derived occurrence in the
      body are the *same* value in every case, including when a second
      `work create` is allocated concurrently (see the concurrent-process
      lock test below) — there is no code path where the two could diverge,
      since both come from the one `allocate` call this invocation made
- [ ] `--producer custom-skill-name` writes that value as `producer:`,
      confirming the default (`accelerator-work`, for direct CLI use) is
      overridable, not hardcoded
- [ ] Two `work create` invocations launched concurrently against the same
      `WORK_DIR` (spawned as real separate processes, not sequential calls
      in one test) never both succeed with the same allocated ID — the
      second either blocks until the first's lock releases and then
      allocates the next number, or observes the first's write and
      allocates past it; genuine mutual exclusion, not a narrowed race
      window
- [ ] The overflow-guard path (Phase 3's `AllocationError::Overflow`) surfaces
      as a clear CLI error, not a partial write, with both the
      out-of-width-file and number-space-exhausted message wordings matching
      the bash originals verbatim, interpolating `highest`/`cap`/`highest_file`
      from the error variant's fields
- [ ] `work create` exits 0 on every success path and 1 on every failure
      path (`AllocationError`, a lock-acquisition timeout, template
      resolution), matching the exit-code contract stated above
- [ ] `mise run cli:check` passes; cargo-pup confirms `work::create` stays
      within the existing `work_domain_imports_only_permitted` rule
      (`std`/`kernel::Error`/`corpus`/`crate` — no new import needed, since
      `compose_frontmatter` returns the crate-local `FieldValue`, not
      `document::Yaml`)

#### Manual Verification

- [ ] `accelerator work create --title "Test item" --kind task --priority
      medium` in a scratch `meta/work/` directory produces a file that opens
      cleanly and whose frontmatter matches the template schema field-for-field
- [ ] Drive `create-work-item` end to end with no integration configured,
      confirming its Step 5 write now goes through `accelerator work create`
      rather than a manual `Write` call, and that the integration-configured
      path (still manual, untouched) continues to work unchanged

---

## Phase 8: Fifth Vertical Slice — `work update` and `work template-hints`

### Overview

Implements `work update` (arbitrary field sets plus tag mutations, one atomic
write) and the small `work template-hints` utility subcommand the correction
in Key Discoveries requires.

### Changes Required

#### 1. `work update` CLI surface

**File**: `cli/work-cli/src/cli.rs`
**Changes**: add

```rust
Command::Update {
    path: PathBuf,
    #[arg(long = "set", value_parser = parse_key_value)]
    sets: Vec<(String, String)>,
    #[arg(long = "add-tag")]
    add_tags: Vec<String>,
    #[arg(long = "remove-tag")]
    remove_tags: Vec<String>,
    #[arg(long = "append", value_parser = parse_key_value)]
    appends: Vec<(String, String)>,
    #[arg(long = "remove", value_parser = parse_key_value)]
    removes: Vec<(String, String)>,
},
```

`--set KEY=VALUE` (repeatable) generalises `work-item-update-tags.sh`'s
add/remove verb pattern to arbitrary **scalar** fields — no bash script
exists to port for this half (today's skill does it via ad-hoc `Edit`
calls, see Key Discoveries), so this flag shape is this plan's own design,
chosen to mirror the tag flags' own repeatable-flag convention already
established. `--append`/`--remove KEY=VALUE` cover the other list-typed
typed-linkage fields (`blocks`, `blocked_by`, `derived_from`, `relates_to`
— `tags` keeps its own dedicated `--add-tag`/`--remove-tag` flags rather
than folding into these, since `mutate_tags`' bash-parity behaviour,
including the preserved naive-comma-split quirk, is specific to how
`tags:` was historically written and must not leak into fields that never
had that bash history). Without `--append`/`--remove`, `update-work-item`'s
repointed write (item 3 below) would have no way to edit
`blocks`/`blocked_by`/`derived_from`/`relates_to` — the skill's own
"any... frontmatter field edit" promise currently covers them, and dropping
that silently would be an unannounced regression.

#### 2. Set-key validation (domain)

**File**: `cli/work/src/update.rs` (new)
**Changes**:

```rust
pub enum UpdateError {
    IdImmutable,
    ScalarSetOnListField { key: String },
}

pub fn validate_set_key(key: &str) -> Result<(), UpdateError> { .. }

pub const LIST_FIELDS: &[&str] =
    &["blocks", "blocked_by", "derived_from", "relates_to"];

pub enum ListAction { Append, Remove }

pub enum ListMutation { Changed(Vec<String>), NoChange }

pub fn mutate_list(
    current: &[String],
    action: ListAction,
    value: &str,
) -> ListMutation { .. }
```

`validate_set_key` rejects **two** classes of key, not just
`id`/`work_item_id`: the own-identity hard-block (unchanged), and — new in
this revision — `tags` and every `LIST_FIELDS` member, returning
`ScalarSetOnListField` with the message `"'{key}' is not a valid --set
target — it is a list field; use --append/--remove (or --add-tag/
--remove-tag for tags) instead"`. Without this second check, `--set
blocks=work-item:0099` would reach the generic "apply as a `Yaml` mapping
key write" path (item 3 below) unfiltered, silently overwriting a `Yaml`
sequence (`blocks: []`) with a bare scalar string — corrupting the field's
type with no error, exactly the failure `--append`/`--remove` exist to
prevent. Both checks run over every `--set` key before any mutation is
applied (same all-keys-validated-before-any-write discipline the
`IdImmutable` check already has), so a `--set blocks=...` mixed with valid
`--set` flags in the same invocation still fails the whole call rather than
partially applying.

`ListAction` (not the tag-specific `TagAction`, Phase 3 item 5) is
`mutate_list`'s action parameter — a separate, identically-shaped type
rather than reusing `TagAction`, so a reader of `update.rs` doesn't have to
know a tag-domain type secretly governs non-tag list fields; the two-line
duplication is cheaper than the cross-module naming confusion the shared
type would otherwise cause.

**Missing-key semantics**: `mutate_list` operates on `current: &[String]`,
the field's already-parsed `Yaml` sequence — but `compose_frontmatter`
(Phase 7) omits every `LIST_FIELDS` member from a freshly-created item
whenever it's empty (the template's own omit-when-empty convention), so
the *first* `--append` against a typical item hits a frontmatter with no
`blocks:` key at all, not an empty sequence. The adapter (item 3) treats a
missing `LIST_FIELDS` key as `current: &[]` when reading it (mirroring
`--set`'s already-stated "creating the key if absent" behaviour), and
inserts the key into the `Yaml` mapping when `mutate_list` returns
`Changed` with a non-empty result — so the very first `--append` on a
freshly-created item creates the key, exactly matching `--set`'s existing
insertion semantics rather than erroring or silently no-op-ing.

Both checks are pure, unit-tested predicates — the piece of `update`'s
decision logic that doesn't inherently need `document::Yaml` to express.
`mutate_list` itself has no bash quirk to preserve (these fields were only
ever edited via the skill's generic `Edit` flow, never a script), so it's a
plain, non-quirky add-if-absent/remove-if-present operation over an
already-parsed `Vec<String>` — no raw-text block-style detection needed,
unlike `mutate_tags`. The rest of `update`'s work (applying validated
`--set` as mapping-key writes, invoking `mutate_tags`/`mutate_list`,
serialising) is mechanical `Yaml`-tree manipulation with no further
branching logic of its own, so it stays in the adapter (item 3) rather than
introducing an artificial domain abstraction over what is already a thin
pass-through to `document`.

#### 3. Update flow (adapter/binary)

**File**: `cli/work-cli/src/update.rs` (new)
**Changes**: acquires `corpus_adapters::lock::acquire` over the target
file's own `<path>.lockdir` (the same per-file sentinel convention
`FileCorpusStore::append_record`/`remove_by_key` already use — see the
Lost-update guard note below) before doing anything else, reads the target
file's frontmatter via `document::parse`, calls `work::update::
validate_set_key` for every `--set` key before applying any of them
(failing the whole invocation on the first `IdImmutable` or
`ScalarSetOnListField` hit, touching nothing), applies each validated
`--set` as a `Yaml` mapping key write (creating the key if absent, matching
the skill's "field insertion" behaviour), applies `--add-tag`/
`--remove-tag` via `work::tags::mutate_tags(&raw_frontmatter, action, tag)`
(the block-style check is now internal to `mutate_tags` itself — see Phase
3, item 5 — so no separate pre-check call is needed here), validates each
`--append`/`--remove` key against `work::update::LIST_FIELDS` before
touching any field — rejecting `tags` specifically with `"'tags' is not a
valid --append/--remove key; use --add-tag/--remove-tag instead"`, and any
other unrecognised key with `"'{key}' is not a valid --append/--remove
key; supported: blocks, blocked_by, derived_from, relates_to"` — and
applies each validated one via `work::update::mutate_list` on the field's
already-parsed `Yaml`
sequence — treating a missing key as an empty sequence and inserting it on
a non-empty result, matching `--set`'s own key-creation behaviour (see item
2) — then writes once via `document::render(Some(&existing_content), &yaml)` +
`FileCorpusStore::write`, releasing the lock (via `LockGuard`'s `Drop`)
once the write completes. `work-item-file-dirty.sh`'s port (`work::
file_dirty` — not yet added; add alongside this phase since it has no
earlier natural home) is exercised by its own characterization test but is
**not** called from this flow, per the decision recorded in "What We're NOT
Doing".

**Lost-update guard**: the read-then-write sequence above has no lock of
its own, and `FileCorpusStore::write` provides none either (see `create`'s
Collision guard note for why a bare pre-write re-check would only narrow,
not close, the race). Holding `corpus_adapters::lock::acquire`'s mkdir-lock
over `<path>.lockdir` for the whole read-modify-write sequence gives
genuine mutual exclusion between concurrent `work update` invocations
against the same file, reusing the exact primitive `append_record`/
`remove_by_key` already rely on rather than inventing a new optimistic
check. This is a pre-existing gap in today's `Edit`-tool-based skill flow
too (not a regression this story introduces), but `work update` is a
faster, more scriptable primitive than the interactive skill it replaces
and is more likely to see genuine concurrent use.

**Exit-code contract**: like `create`, `update` has no bash original to
inherit exit codes from (`work-item-update-tags.sh` only ever computed a new
tag array, it never wrote a file — see Key Discoveries), so this plan
defines one explicitly: 0 on success; 1 for every failure — `UpdateError::
IdImmutable`, `UpdateError::ScalarSetOnListField`, `TagError::
BlockStyleTags`, an unrecognised `--append`/`--remove` key (not in
`LIST_FIELDS`), a lock-acquisition timeout, or a `document::parse`/
file-access failure. Same one-failure-outcome rationale as
`create`: 0194's `--push` wiring only needs success/failure, not failure
kind, at the exit-code level.

**`IdImmutable` message text**: `validate_set_key` returns the error data;
`work-cli` renders it as the skill's own existing wording verbatim —
`"Error: own-identity (id) cannot be changed — the filename prefix is the
authoritative work item ID. To renumber a work item, rename the file (e.g.
jj mv) and update the id field to match. The id field is always a quoted
string."` (`update-work-item/SKILL.md:133-137`) — so there is exactly one
place this message is written, not two independently-maintained copies.

**File**: `skills/work/update-work-item/SKILL.md`
**Changes**: repoint the actual field/tag/list write step (today: direct
`Edit` tool calls for scalar and list fields, `work-item-update-tags.sh`
plus a manual write for tags) to `${CLAUDE_PLUGIN_ROOT}/bin/accelerator
work update <path> --set KEY=VALUE ... --add-tag ... --remove-tag ...
--append KEY=VALUE ... --remove KEY=VALUE ...`, once the skill's own
natural-language interpretation, diff preview, and confirmation prompt have
decided the concrete operations (per "What We're NOT Doing" — that
interactive layer is unchanged). Remove the Special Field Rules' own
hard-coded `id`-immutability error text and instead let the CLI's exit-1
failure (with the message above) surface directly — the skill's
natural-language layer still recognises an attempted `id`/`work_item_id`
edit early enough to avoid gathering a diff preview for it, but the error
text itself now has one source of truth, not two.

#### 4. `work template-hints` utility subcommand

**File**: `cli/work-cli/src/cli.rs`
**Changes**: add `Command::TemplateHints { field: String }`.

**File**: `cli/work-cli/src/template_hints.rs` (new)
**Changes**: resolves the `work-item` template via
`config::ReadTemplate::resolve_template` (config-adapters), calls
`work::template_hints::extract_hints`, prints one value per line, always
exits 0 — exact behavioural match for `work-item-template-field-hints.sh`.

**Files**: `skills/work/update-work-item/SKILL.md`,
`skills/work/list-work-items/SKILL.md`
**Changes**: repoint every `work-item-template-field-hints.sh <field>`
invocation to `${CLAUDE_PLUGIN_ROOT}/bin/accelerator work template-hints
<field>`. `list-work-items` is repointed to a `work` subcommand for the
first time in this phase — add `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator
work *)` to its `allowed-tools`, mirroring Phase 4's grant to
`update-work-item` (which already has this permission and needs no change).

#### 5. `work canonicalise-id` utility subcommand

**File**: `cli/work-cli/src/cli.rs`
**Changes**: add `Command::CanonicaliseId { input: String }`.

**File**: `cli/work-cli/src/canonicalise_id.rs` (new)
**Changes**: resolves `work.id_pattern`/`work.default_project_code` via
`ConfigAccess::effective_nonempty` (the same config wiring `resolve`
already does in Phase 4), calls
`corpus_adapters::work_item_pattern::canonicalise_id`, prints the result on
stdout, exits 1 with the bash's exact `E_PATTERN_*` message on error — exact
behavioural match for `wip_canonicalise_id`.

**Files**: `skills/work/update-work-item/SKILL.md`,
`skills/work/list-work-items/SKILL.md`
**Changes**: repoint every `wip_canonicalise_id` invocation (via
`bash -c "source work-item-common.sh && wip_canonicalise_id ..."`) to
`${CLAUDE_PLUGIN_ROOT}/bin/accelerator work canonicalise-id <input>`.

#### 6. `work next-number` utility subcommand

**File**: `cli/work-cli/src/cli.rs`
**Changes**: add
`Command::NextNumber { #[arg(long)] project: Option<String>, #[arg(long, default_value = "1")] count: u32 }`.

**File**: `cli/work-cli/src/next_number.rs` (new)
**Changes**: the same config wiring `resolve`/`create` already do
(`work.id_pattern`/`work.default_project_code`, `paths.work`), then calls
`work::next_number::allocate` (already built in Phase 3 — this subcommand
is a thin display wrapper, not new domain logic) and prints each ID on its
own line — display-only, matching `work-item-next-number.sh --project
<code> --count <count>`'s own behaviour exactly: it never writes a file and
never commits a number, it just reports what the next N would currently be.
No lock is needed here for that reason (unlike `create`'s own internal
allocation, which does write and therefore does lock — see Phase 7's
Collision guard).

**Exit-code contract**: 0 on success, IDs on stdout; 1 for any
`AllocationError` — matching `work-item-next-number.sh`'s own single
failure code (`return 1`/`exit 1` on every error arm; it has no distinct
usage-error code). On `AllocationError::Overflow`, reproduces bash's
partial-emission behaviour exactly (`work-item-next-number.sh:127-134`):
prints every ID that fits before the boundary to stdout, then exits 1 —
this is the primary consumer where `count > 1` makes `Overflow`'s
`partial` field actually non-empty (unlike `create`'s always-`count-1`
allocation, where overflow always yields an empty `partial`).

**Why this exists**: `extract-work-items/SKILL.md` calls `work-item-
next-number.sh --project <code> --count <count>` twice — a display-only
projected-ID computation re-issued after every draft amendment, and a
final batch commit per distinct project (the commit path still uses `work
create`'s own per-item allocation, one call per item, not this
subcommand). `sync-work-items/SKILL.md:304` makes an equivalent batch
display call during pull. Neither skill had anywhere to repoint that
`--count N` call without this subcommand — `work create` only ever
allocates and writes one ID per invocation. The actual skill repoints
happen in Phase 9 (`extract-work-items` via its generic grep-repoint pass,
now that a target exists for this call; `sync-work-items` explicitly, per
that phase's item 1) — this item only builds the subcommand.

#### 7. `work-item-file-dirty.sh` port (characterization only)

**File**: `cli/work/src/file_dirty.rs` (new)
**Changes**: port of `work-item-file-dirty.sh:39-106`'s pure decision logic
(mode dispatch + dirty/clean/indeterminate-fail-safe), taking already-fetched
VCS status text as input (the real `jj diff`/`git status` shellout is the
adapter's job, not exercised here beyond a characterization test using the
Phase 1 golden's injected-override cases):

```rust
pub enum VcsMode { Jj, Git, Indeterminate }

#[must_use]
pub fn is_dirty(mode: VcsMode, path_relative: &str, status_text: &str) -> bool { .. }
```

### Success Criteria

#### Automated Verification

- [ ] `cargo test -p work --locked` — `validate_set_key` rejects `id` and
      `work_item_id` and rejects `tags`/every `LIST_FIELDS` member
      (`ScalarSetOnListField`), accepting every other key, table-driven, no
      filesystem or `document` dependency; `mutate_list` covers add
      (new/duplicate) and remove (present/absent) for each of the four
      `LIST_FIELDS`, including the missing-key-treated-as-empty case,
      table-driven, no filesystem dependency
- [ ] `cargo test -p accelerator-work --locked` — `work update --set
      status=ready --set priority=high` on a fixture file changes exactly
      those two frontmatter *values* (parsed equality, not byte-identity —
      `document::render` re-serialises the whole tree, so this does not
      assert byte-identical bytes for untouched fields; see Key Discoveries);
      `--add-tag`/`--remove-tag` matches the Phase 1
      `work-item-update-tags.golden` rows including the no-change and
      block-style-rejection cases; `--append blocks=work-item:0099` on a
      fixture with **no** `blocks:` key inserts the key with a one-element
      sequence; `--remove blocks=work-item:0099` on an existing sequence
      changes only the `blocks:` sequence; `--set blocks=work-item:0099`
      is rejected (`ScalarSetOnListField`, not applied as a scalar
      overwrite) with a message pointing at `--append`/`--remove`;
      `--append tags=...` is rejected (`tags` must use `--add-tag`, not
      `--append`); `--set id=...` is hard-blocked with the exact bash error
      message via `validate_set_key`, failing before any `--set` is applied
      even when other valid `--set` flags are present in the same
      invocation
- [ ] `work update` exits 0 on every success path and 1 on every failure
      path (`IdImmutable`, `BlockStyleTags`, an unrecognised `--append`/
      `--remove` key, missing/unparseable file), matching the exit-code
      contract stated above
- [ ] Two `work update` invocations launched concurrently against the same
      target file (real separate processes) never both succeed silently
      overwriting each other — the second either blocks on the first's lock
      and applies its own change afterward against the now-current content,
      or the test otherwise confirms neither change is silently lost;
      genuine mutual exclusion via `<path>.lockdir`, not a narrowed race
      window
- [ ] A CLI-surface snapshot test asserts **every** `Command` variant's
      clap-generated argument list (flag names, arity, `--set`'s repeatable
      `KEY=VALUE` shape) against a single committed golden, not just
      `Create`/`Update` — `Resolve`, `Show`, `Diff`, `TemplateHints`,
      `CanonicaliseId`, and `NextNumber` are equally part of 0194's
      dependency surface (`next-number`'s flags specifically, per Phase 8
      item 6's "Why this exists") and an earlier version of this plan only
      snapshotted two of the eight variants while the Desired End State
      claimed the whole CLI surface was frozen by this test — this is the
      fix that makes that claim actually true. `Create`/`Update`'s own
      signatures are additionally the frozen contract 0194 depends on (its
      own Assumptions section states these signatures are stable once
      implemented); the snapshot makes a future accidental change to any
      command's flags a visible, deliberate diff instead of a silent break
      for 0194 to discover later
- [ ] A dedicated formatting-preservation test runs `work update` against a
      fixture frontmatter block containing an inline comment, a CRLF line
      ending, and a flow-style array (`tags: [a, b]`), asserting only which
      properties survive the round trip (parsed values) and pinning, not
      hiding, whichever of those three do not (comments are expected to be
      dropped — `document::Yaml` has no comment representation — so this test
      documents the actual behaviour rather than leaving it to be discovered
      by a surprised user)
- [ ] `work template-hints kind` matches the Phase 1
      `work-item-template-field-hints.golden` rows, including the
      hardcoded-fallback path
- [ ] `work canonicalise-id` matches every row in the Phase 1
      `work-item-canonicalise-id.golden` at the CLI boundary, plus the
      missing-project and unrecognised-shape error messages verbatim
- [ ] `cargo test -p work` — `is_dirty` matches every case in the Phase 1
      `work-item-file-dirty.golden`
- [ ] `cargo test -p accelerator-work --locked` — a **behavioural**
      (not source-grep) confirmation that `update`'s dirtiness guard is
      unwired: `work update` against a target file with simulated
      uncommitted VCS changes (a dirty `jj`/`git` working copy in the test
      fixture) still succeeds and writes, proving the write path is
      unconditional rather than merely asserting `file_dirty` is absent from
      the source text — a behavioural check survives a refactor that makes
      `file_dirty` reachable indirectly (a re-export, a trait object) in a
      way a grep would miss
- [ ] `work next-number --project PROJ --count 3` prints exactly 3
      sequential IDs matching `work-item-next-number.sh --project PROJ
      --count 3`'s output for the same fixture directory, and a repeated
      invocation prints the *same* starting number again (display-only —
      confirms no file is written and no number is committed)
- [ ] `work next-number --count N` against a fixture directory pre-seeded
      at/over the cap prints the partial IDs that fit before exiting 1,
      matching `work-item-next-number.sh:127-134`'s partial-emission
      behaviour exactly — this is the case Phase 7's `create` (always
      `count=1`) cannot exercise, since its `partial` field is always empty
- [ ] `mise run cli:check` passes
- [ ] `mise run lint:skills:check` passes for both repointed skills, plus
      `extract-work-items` and `sync-work-items` (repointed to `work
      next-number` in Phase 9)

#### Manual Verification

- [ ] `accelerator work update <path> --set priority=high` on a scratch work
      item under `meta/work/` (real files, which carry no comments or
      non-default formatting today) shows only a single-line `jj diff`/`git
      diff` change in practice; this is an expected outcome given today's
      corpus, not a guaranteed one — the automated formatting-preservation
      test above is the actual contract
- [ ] Drive `update-work-item` end to end on a scratch work item, confirming
      its Step 3 write now goes through `accelerator work update` and that
      the id-immutability error (attempted `--set id=...`) surfaces the
      CLI's message, not a separate skill-side check

---

## Phase 9: Skill Repoint and Shell Deletion

### Overview

Repoints every remaining skill invocation of the 11 lifecycle scripts, deletes
the scripts and their characterization coverage (splitting, not deleting,
`test-work-item-scripts.sh`), and decrements the work suite floor — landed
together per the AC.

### Changes Required

#### 1. Remaining skill repoints

**File**: `skills/work/sync-work-items/SKILL.md`
**Changes**: this file references five scripts total, not three — an
earlier version of this plan checked only the three deferred scripts and
wrongly claimed "Changes: none" for the whole file. Two of the five *are*
deleted by this story and must be repointed:
- `SKILL.md:199` (`work-item-section-diff.sh`, the conflict-resolution
  diff shown to the user immediately before the typed-token prompt that
  decides whether to overwrite local edits with the remote version — the
  most safety-sensitive call site in this skill) repoints to
  `${CLAUDE_PLUGIN_ROOT}/bin/accelerator work diff <local> <remote>`
  (already built in Phase 6).
- `SKILL.md:304` (`work-item-next-number.sh --count N`, batch ID
  allocation during pull) repoints to `${CLAUDE_PLUGIN_ROOT}/bin/accelerator
  work next-number --count N` (Phase 8, item 6).

`work-item-project-remote.sh`/`work-item-file-dirty.sh`/
`work-item-normalise.sh` invocations (`SKILL.md:120,136,311`) are left
exactly as they are — per the deferred-deletion decision recorded in "What
We're NOT Doing" and Key Discoveries, these three scripts are not deleted by
this story, so the skill's existing shell-outs to them keep working
unmodified. `sync-work-items`'s own remaining logic (the sync engine itself)
stays 0194's scope per the work item's Drafting Notes; 0194 repoints these
three call sites when it rewires the skill onto its own `sync` command and
removes the three scripts.
**File**: `skills/work/create-work-item/SKILL.md`,
`skills/work/refine-work-item/SKILL.md`,
`skills/work/stress-test-work-item/SKILL.md`,
`skills/work/review-work-item/SKILL.md`,
`skills/work/extract-work-items/SKILL.md`
**Changes**: grep each for any remaining `work-item-*.sh` invocation among the
8 scripts this story deletes (see item 2 below) and repoint to the matching
`accelerator work <verb>` — confirmed empty or non-empty at implementation
time, per 0169's lesson to re-verify script partitions from source rather
than trusting prior prose. `extract-work-items` is explicitly included here
because it invokes `work-item-next-number.sh` at least eight times, including
batch allocation (`--project <code> --count <count>`); if `work create`'s
single-ID-per-invocation signature (Phase 7) can't serve that batch shape,
call this out during this phase rather than silently leaving a stale
reference for the automated check below to catch.

**File**: skill frontmatter for every skill repointed in this phase
(`update-work-item` already covered in Phase 4, `list-work-items` already
covered in Phase 8; add `create-work-item`, `refine-work-item`,
`stress-test-work-item`, `review-work-item`, `extract-work-items`, and
`sync-work-items` — this last one now genuinely needs the grant too, since
it gets real repoints above, not the "no changes" this phase originally
assumed)
**Changes**: add `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator work *)` to each
newly-repointed skill's `allowed-tools`, matching the pattern Phase 4
establishes for `update-work-item` — without this, a repointed invocation
triggers an interactive tool-approval prompt instead of running silently.

#### 2. Script and test-suite deletion

**Files removed**: `work-item-common.sh`, `work-item-next-number.sh`,
`work-item-pattern.sh`, `work-item-read-field.sh`, `work-item-read-status.sh`,
`work-item-resolve-id.sh`, `work-item-section-diff.sh`,
`work-item-update-tags.sh`, `work-item-template-field-hints.sh` (9 files: 8 of
the 11 named in Requirements, plus `work-item-common.sh`, the sourced-only
library every one of them depends on and which has no other consumer once
they are gone — confirm this at implementation time via grep, since
`vcs-common.sh`'s `find_repo_root`/`vcs_mode` precedent shows a shared
library can outlive its originally-obvious callers; also confirm none of the
three deferred scripts below source `work-item-common.sh`, since their
survival would block its deletion too — verified not to as of this plan's
writing).

**Not removed** (see "What We're NOT Doing" and Key Discoveries):
`work-item-file-dirty.sh`, `work-item-project-remote.sh`,
`work-item-normalise.sh` — `sync-work-items` still shells out to all three
directly and this story adds no CLI surface to repoint that skill to; 0194
removes them once it rewires the skill.

**File**: `skills/work/scripts/test-work-item-pattern.sh` — removed (the
parity gate; superseded by `cargo test -p corpus-adapters --locked`'s parity
suite from Phase 2).

**File**: `skills/work/scripts/test-work-item-scripts.sh` — **sections**
removed (not the whole file). Verify each `=== ... ===` marker against the
file's real contents at implementation time rather than trusting this list —
the sections are interleaved, not contiguous, so no single line range covers
them. As of this plan's writing, the sections to remove are:

- `=== work-item-next-number.sh ===`
- `=== work-item-next-number.sh (configured pattern) ===`
- `=== work-item-resolve-id.sh ===`
- `=== work-item-next-number.sh default-pattern golden file ===`
- `=== Frontmatter consumer integration (quoted work_item_id) ===` (covers
  `read-field`'s quoted-value handling, `wip_is_work_item_file`, and
  `wip_canonicalise_id` — all now ported per Phase 3/8)
- `=== work-item-read-status.sh ===`
- `=== work-item-read-field.sh ===`
- `=== work-item-update-tags.sh ===`
- `=== work-item-template-field-hints.sh ===`
- `=== work-item-section-diff.sh — section-grouped conflict diff ===`

The following sections stay, since their scripts are not removed by this
phase: `=== work-item-sync-label.sh ===`, `=== work-item-normalise.sh ===`,
`=== work-item-sync-baseline.sh ===`, `=== work-item-sync-label.sh —
baseline-dependent label arms ===`, `=== work-item-sync-classify.sh —
change-detection engine ===`, `=== work-item-sync-decide.sh — (mode × state)
decision table ===`, `=== work-item-file-dirty.sh — VCS-mode-aware overwrite
guard ===`, `=== work-item-project-remote.sh — per-tracker projection seam
===`, `=== work-item-sync-apply.sh — pull + finalise + resumability ===`, and
the shared `setup_repo` harness.

**File**: `skills/work/scripts/test-fixtures/` — the new goldens from Phase 1
stay (they document the Rust port's characterization basis); the directory's
existing `work-item-next-number.golden` stays too, now read only by the
deleted golden-reader section... **decide at implementation time** whether to
keep the goldens as documentation or delete them alongside the bash reader
that consumed them — recommend keeping, since they remain a legible
characterization record even after the bash reader is gone.

#### 3. Suite floor decrement

**File**: `tasks/test/integration.py:49`
**Changes**: `_EXPECTED_WORK_SUITES` decreases by 1 (from 6 to 5) — deleting
`test-work-item-pattern.sh` removes one whole suite;
`test-work-item-scripts.sh` stays (edited, not removed), so it still counts
as one suite.

#### 4. Changelog entry

**File**: `CHANGELOG.md`
**Changes**: add an entry under `## [Unreleased]` → `### Added`, following
the existing entries' style: a one-sentence bolded summary of the new
`accelerator work create|show|resolve|diff|update` command family plus the
`template-hints`/`canonicalise-id`/`next-number` utility subcommands,
noting they replace the 8 bash scripts this phase deletes.

### Success Criteria

#### Automated Verification

- [ ] `mise run test:integration:work` passes with the new floor of 5
- [ ] `grep -rn "work-item-\(common\|next-number\|pattern\|read-field\|read-status\|resolve-id\|section-diff\|update-tags\|template-field-hints\)\.sh" skills/`
      returns zero matches outside `meta/` history/plan documents (the three
      deferred scripts — `file-dirty`, `project-remote`, `normalise` —
      deliberately excluded from this pattern; confirm their only remaining
      references are the intact `sync-work-items/SKILL.md:120,136,311` calls)
- [ ] Before this phase's script deletion lands, run `accelerator work
      show`/`resolve`/`diff` against every real file in `meta/work/` (not
      just the Phase 1 curated goldens) and diff the output against the
      bash originals invoked the same way — a one-off, full-corpus parity
      sweep gated as the last check before the bash fallback is gone. A
      divergence found here is caught in CI, not in production against
      irreplaceable work-item data. Extend the same sweep to the three
      deferred ports (`file_dirty`, `project_remote`, `normalise`) even
      though their bash originals aren't deleted this phase — they sit
      unexercised by any live path until 0194 wires them in, so a drift
      between the Rust port and its still-live bash original is otherwise
      invisible until 0194 depends on it
- [ ] `mise run` (the full bare default task) passes end to end
- [ ] `mise run lint:scripts:check` passes (no dangling references, exec-bit
      invariant intact for whatever remains)
- [ ] `mise run lint:skills:check` passes for every skill repointed in this
      phase, confirming the `allowed-tools` updates in item 1 are complete

#### Manual Verification

- [ ] Manually drive `update-work-item`, `list-work-items`, `create-work-item`,
      and `extract-work-items` against a scratch work item end to end,
      confirming no residual shell-out to a deleted script and no behavioural
      regression versus pre-migration usage
- [ ] Manually confirm `sync-work-items`' dirty-check guard
      (`work-item-file-dirty.sh`) still functions unmodified — invoke a
      pull against a deliberately-dirtied local file and confirm it is still
      refused, since this guard is exactly what Phase 9 must not silently
      break

---

## Testing Strategy

### Unit Tests

- Every `work` domain function (Phase 3) is table-driven against the Phase 1
  goldens, using hand-rolled `DirectoryLister`/`IdScanner` doubles — no
  filesystem, no subprocess.
- Pattern-DSL extensions (Phase 2) get direct unit tests plus continued parity
  coverage via the existing `work_item_pattern_parity.rs` harness.

### Integration Tests

- `cli/work-adapters/tests/` — adapter-boundary tests for `filesystem.rs`
  (real temp directories) and `diff_shellout.rs` (the real `diff` binary,
  gated behind `bash-parity`), mirroring `vcs-adapters`' split.
- `cli/work-cli/tests/` — CLI-boundary tests spawning the compiled
  `accelerator-work` binary against real temp directories, covering exit
  codes, stdout/stderr shape, and atomic-write behaviour (parsed field
  values preserved for untouched fields — not byte-identical formatting,
  since `document::render` re-serialises the whole tree; see Key
  Discoveries and Phase 8's dedicated formatting-preservation test).
- `--features bash-parity` (Phase 6) gates the tests needing the real
  `diff` binary on `PATH` in both `work-adapters` and `work-cli`, mirroring
  `vcs-adapters`/`vcs-cli`'s convention; CI enables it, local dev does not
  need it by default.

### Manual Testing Steps

1. Create a work item via `accelerator work create`, confirm frontmatter and
   body match the template schema.
2. Resolve it by bare number, full ID, and path via `accelerator work
   resolve`, confirming identical results to the bash original on the same
   fixture corpus.
3. Update a field and a tag via `accelerator work update`, confirming only
   the targeted lines change (`jj diff`).
4. Diff two divergent copies via `accelerator work diff`, confirming section
   boundaries and the `(no differing sections after normalisation)` fallback.
5. Drive the `update-work-item` and `list-work-items` skills end to end,
   confirming hint elicitation still works via `work template-hints`.

## Performance Considerations

None expected to be material — every command is a single-pass scan over a
work-item directory (typically low hundreds of files) and one atomic write;
no new network or subprocess cost beyond the existing `diff -u` shellout
(Phase 6), which 0169 already established as an acceptable pattern for
byte-parity rendering.

## Migration Notes

No data migration — work-item files themselves are untouched by this story;
only the tooling that reads/writes them changes. `external_id` remains a
frontmatter-key convention (ADR-0044); this story adds `create`'s
"never write it" behaviour and `update`'s ordinary `--set external_id=...`
support, but no new domain type or validation beyond presence.

## References

- Work item: `meta/work/0170-work-item-subdomain-and-sync-engine.md`
- Research: `meta/research/codebase/2026-08-06-0170-work-item-lifecycle-subdomain.md`
- Architectural precedent: `meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md`
- `cli/vcs-cli/`, `cli/vcs/`, `cli/vcs-adapters/` — the three-crate split this
  plan mirrors
- ADR-0044 (`external_id` convention), ADR-0045, ADR-0052, ADR-0053
- Split sibling: `meta/work/0194-tracker-crate-and-remote-sync-engine.md`
