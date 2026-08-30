---
type: "plan"
id: "2026-08-30-0201-in-process-section-diff"
title: "In-Process Section Diff Implementation Plan"
date: "2026-08-30T13:34:43+00:00"
author: "Toby Clemson"
producer: "create-plan"
status: "done"
work_item_id: "work-item:0201"
parent: "work-item:0201"
derived_from: ["codebase-research:2026-08-30-0201-in-process-section-diff"]
relates_to: ["work-item:0170", "work-item:0174", "work-item:0188", "work-item:0198"]
tags: ["rust", "work-adapters", "diff", "tech-debt"]
revision: "6e0cf31e384d6995030af9bf79a7b14b5ff5afc5"
repository: "accelerator"
last_updated: "2026-08-30T17:08:29+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# In-Process Section Diff Implementation Plan

## Overview

Replace `work-adapters`' subprocess `diff -u` section renderer with an
in-process `similar` implementation, make the renderer infallible, retire the
spent bash-oracle parity suites, and add a crate-wide `cargo-pup` zero-spawn
guard alongside the existing filesystem allow-list. The result: `work-adapters` spawns no
subprocess and needs no `diff` binary on `PATH`, while the frozen
`=== name (- LOCAL / + REMOTE) ===` framing callers depend on is unchanged.

## Current State Analysis

`work_adapters::diff_shellout::render` writes `LOCAL`/`REMOTE` temp files and
spawns `diff -u` under a 10s cap with a 10ms poll loop, collapsing three
failure modes into one `DiffUnavailable` unit struct
(`cli/work-adapters/src/diff_shellout.rs:24,33-102`). The header and
blank-line framing is the frozen contract (`:58-63`); the body between is GNU
diffutils' hunk output, which this item un-freezes.

`render`'s `Result<String, DiffUnavailable>` is threaded through **four edited
files** across two commands (`work diff`, `sync`):

| Consumer | Site | Handling |
| --- | --- | --- |
| `work diff` | `cli/work-cli/src/diff.rs:41-44` | `Err` → `RunOutcome::DiffUnavailable` → `main.rs:167` |
| Sync/dossier engine | `cli/work-adapters/src/sync/run.rs:258-277` | `Err` → `DossierRender::Unrenderable` fallback |
| Sync CLI wiring | `cli/work-cli/src/sync.rs:285,318,480` | injects `render` as `&dyn Fn(...) -> Result<_, DiffUnavailable>` |

The parity teardown spans six artefacts, and its guard runs in the
**default** test lane — not behind the `bash-parity` feature — so a partial
edit reds the ordinary `cargo nextest` run:

- `cli/work-adapters/tests/diff_shellout_parity.rs` — `bash-parity`-gated golden suite.
- `cli/work-cli/tests/cli_diff_parity.rs` — `bash-parity`-gated CLI golden suite.
- `cli/work-adapters/tests/fixtures/work-item-section-diff/` — the five case fixtures.
- `cli/work-adapters/tests/fixtures/bash-parity-baseline.txt` — 2 `test` rows, 5 `case` rows, 5 `hash` rows for this corpus.
- `cli/work-adapters/tests/bash_parity_baseline.rs` — asserts `recorded.len() == 10`, names `work-item-section-diff` in `RELOCATED_CORPORA` and `corpus_home()`.
- `cli/work-cli/Cargo.toml` — work-cli's own `bash-parity` feature (independent of work-adapters'), whose sole consumer is `cli_diff_parity.rs`; deleting that suite orphans it.

### Key Discoveries

- The `pup.ron` rule guards `filesystem`, **not** the spawner. `work_adapters_filesystem_reads_in_process` matches `^work_adapters::filesystem` and denies `std::process` imports *there*; `diff_shellout` is deliberately left outside it (`cli/pup.ron:283-304`). Deleting the rule drops `filesystem.rs`'s purity guard, not a quarantine on the spawner.
- `std::process` appears in `src/` **only** in `diff_shellout.rs`; every other module is `use std::process`-free, so the crate is genuinely zero-spawn once this module is replaced.
- `tempfile` **stays** a normal dependency: `filesystem.rs:48` and the `sync/*` test modules use it. Only the diff module's use goes.
- `cargo-pup` `RestrictImports` catches `use`-path imports, not inline fully-qualified calls — `filesystem.rs:48`'s inline `tempfile::tempdir()` is invisible to the rule, and so is any inline `std::process::Command::new()` (`cli/pup.ron:306-314`).
- `similar`'s `unified_diff()` **without** `.header()` emits hunks only (`@@ … @@` plus `-`/`+`/context lines) — no `--- /+++` file lines — preserving the custom framing while giving AC3's `@@` header and `-`/`+` prefixes. `context_radius(3)` matches `diff -u`'s default.
- `similar` is `Apache-2.0` (in the pruned allow-list); its `text` default feature is dependency-free — the three optional deps (`bstr`/`unicode-segmentation`/`serde`) are non-default — so `cli/deny.toml` needs no edit.
- The `bash-parity` feature flag **survives**: `sync_working_copy_status.rs` gates its VCS real-repo tests on it, and `tasks/test/cli.py` enables it via `--all-features`.
- `DossierRender::Unrenderable` has a second trigger — `dossier.local_unreadable` (`sync/run.rs:262`) — so the variant survives; only its `Err(DiffUnavailable)` entry path is removed.
- No live consumer parses the diff body; the only structural reader is the unimplemented 0213 flow. The body format is free to change.
- `SectionDiff.local`/`.remote` from `differing_sections` are **already newline-terminated** for non-empty content (`trim_lines` → `join_with_trailing_newlines`, `cli/work/src/normalise.rs:34-41`), and `""` (no newline) for an absent/one-sided section. The renderer must feed these to `similar` verbatim — re-appending `\n` doubles the terminator, injects a phantom trailing blank context line, and widens `@@ -1 +1 @@` to `@@ -1,2 +1,2 @@`.

## Desired End State

- `work_adapters::diff::render(&SectionDiff) -> String` computes a
  unified-diff-style body entirely in-process via `similar`, with no
  `std::process`, no `Command`, and no temp files anywhere in `work-adapters`.
- The `=== name (- LOCAL / + REMOTE) ===` header and blank-line framing are
  byte-identical to today; the body carries per-line `-`/`+` prefixes and at
  least one `@@`-style hunk header.
- `DiffUnavailable` and `RunOutcome::DiffUnavailable` are gone; both commands
  (`work diff`, `sync`) call an infallible `render`.
- The `diff_shellout`/`cli_diff` parity suites and the `work-item-section-diff`
  fixture corpus are retired; the baseline manifest and its guard are
  realigned; a new in-process golden test asserts the renderer's own output
  contract.
- `cli/pup.ron` carries a new crate-wide `work_adapters_is_zero_spawn` rule
  **alongside** the retained `work_adapters_filesystem_reads_in_process`
  allow-list; the cargo-pup check passes. The rule is registered with a probe
  pair in `tests/integration/pup/test_import_rule.py`, and a runtime
  `work-adapters/tests/zero_spawn.rs` harness exercises the crate's real
  render paths under a spawn-tripwire `PATH` — together they guard the invariant
  against both a mistyped rule and a future inline spawn the import rule cannot
  see.
- `mise run` is green with no `diff` binary required anywhere in the
  `work`/`work-adapters`/`work-cli` build or test process.

Verify: `accelerator work diff <local> <remote>` on two work items differing in
one section prints the unchanged framing with a `@@`/`-`/`+` body, and the same
command succeeds with `diff` absent from `PATH`.

## What We're NOT Doing

- Not changing `work::section_diff::differing_sections`, `SectionName`, or the
  trim-only normalisation — the section-extraction and comparison logic is
  untouched.
- Not removing the `bash-parity` feature flag — `sync_working_copy_status.rs`
  still gates on it.
- Not removing `DossierRender::Unrenderable` — its `local_unreadable` trigger
  remains; only the diff-unavailable trigger goes.
- Not matching GNU diffutils' hunk output byte-for-byte — the body is
  un-frozen; only the framing is preserved.
- Not implementing the planned 0213 conflict-resolution consumer.

## Implementation Approach

Three phases, dependency-ordered 1 → 2 → 3, each independently mergeable and
each keeping `mise run` green on its own. Phase 1 retires the parity suites
first so Phase 2's signature change touches no dead test surface. Phase 2 is
the migration proper — one atomic compiling change across the module and its
four edited files. Phase 3 locks in the zero-spawn invariant once the crate is
genuinely zero-spawn: a crate-wide isolation rule, its `test_import_rule.py`
probe pair, and a runtime harness that closes the rule's inline-spawn blind spot.

The `cli/` workspace is nested; run `cargo` from `cli/` for single-crate loops,
and the `mise run` tasks from the repo root for the CI mirror.

---

## Phase 1: Retire the bash-oracle diff parity suites

### Overview

Delete the two `bash-parity`-gated diff suites and the `work-item-section-diff`
fixture corpus, and realign the baseline manifest and its default-lane guard in
lockstep. This does the *retire* half of AC6. It stands alone: `render` still
spawns `diff` (Phase 2 replaces it), but the deleted suites were the frozen
bash oracle whose byte-identity purpose is spent, and the guard stays honest.

### Changes Required

#### 1. Confirm the fixture corpus has no other reader

Before deleting, grep the test surface for any other reference to the corpus:

```bash
cd cli && grep -rn "work-item-section-diff" work-adapters/tests work-cli/tests
```

Expect matches only in the two parity suites, the baseline manifest, and the
default-lane guard `bash_parity_baseline.rs` (edited in step 4). Any other hit
widens this phase. The string also appears in production at
`cli/work-cli/src/main.rs:164` (the `NonFileArgument` error prefix — unrelated
to the fixture corpus, left untouched), which is why the grep is scoped to
`tests/` rather than the whole crate tree.

#### 2. Delete the two parity suites and the fixtures

**Files**: delete outright.

- `cli/work-adapters/tests/diff_shellout_parity.rs`
- `cli/work-cli/tests/cli_diff_parity.rs`
- `cli/work-adapters/tests/fixtures/work-item-section-diff/` (all five case directories)

⚠️ Delete only the `_parity` suite. `cli/work-cli/tests/cli_diff.rs` (no
`_parity` suffix) is the non-`bash-parity` CLI-boundary suite and is **retained**
— Phase 2 extends it.

#### 3. Realign the baseline manifest

**File**: `cli/work-adapters/tests/fixtures/bash-parity-baseline.txt`
**Changes**: remove the `work-item-section-diff` rows — 2 `test`, 5 `case`, 5 `hash`.

```diff
-test  cli/work-adapters/tests/diff_shellout_parity.rs     3   work-item-section-diff
 test  cli/work-adapters/tests/sync_baseline_corpus.rs     1   work-item-sync-baseline
 test  cli/work-adapters/tests/sync_working_copy_status.rs 7   (no committed fixture)
-test  cli/work-cli/tests/cli_diff_parity.rs               3   work-item-section-diff
 test  cli/remote-projection/tests/parity.rs               4   work-item-project-remote
```

```diff
-case  work-item-section-diff     case-frontmatter-only
-case  work-item-section-diff     case-frontmatter-only-last-updated
-case  work-item-section-diff     case-named-heading-and-one-sided
-case  work-item-section-diff     case-no-diff-after-normalisation
-case  work-item-section-diff     case-preamble
```

Remove the five `hash` rows for `work-item-section-diff/case-*/expected.txt`.

#### 4. Realign the guard

**File**: `cli/work-adapters/tests/bash_parity_baseline.rs`
**Changes**: drop the two retired test rows from the count, and drop the corpus
from both the `RELOCATED_CORPORA` const and the `corpus_home()` map.

```diff
     assert_eq!(
         recorded.len(),
-        10,
-        "the baseline names the ten pure-Rust parity tests 0212 left behind"
+        8,
+        "the baseline names the eight pure-Rust parity tests left behind"
     );
```

```diff
 const RELOCATED_CORPORA: &[&str] = &[
     "work-item-normalise",
     "work-item-project-remote",
-    "work-item-section-diff",
     "work-item-sync-baseline",
 ];
```

```diff
-        "work-item-section-diff" | "work-item-sync-baseline" => {
-            "cli/work-adapters/tests/fixtures"
-        }
+        "work-item-sync-baseline" => "cli/work-adapters/tests/fixtures",
```

#### 5. Reword the surviving `bash-parity` feature comment

**File**: `cli/work-adapters/Cargo.toml`
**Changes**: the `bash-parity` comment (`:13-15`) describes the now-deleted
`diff_shellout` golden test. Reword it to its surviving purpose — gating
`sync_working_copy_status.rs`'s real-repo VCS tests, which need `jj`/`git` on
`PATH`. The feature itself stays.

#### 6. Remove work-cli's orphaned `bash-parity` feature

**File**: `cli/work-cli/Cargo.toml`
**Changes**: work-cli defines its own `bash-parity` feature (`:17`, independent
of work-adapters'), whose only consumer was the deleted `cli_diff_parity.rs`.
Delete the `bash-parity = []` entry and its comment (`:14-17`) — after this
phase it gates nothing. CI runs `--all-features`, which tolerates the removal.

### Success Criteria

#### Automated Verification

- [x] No stray reference to the corpus in the test surface: `cd cli && grep -rn "work-item-section-diff" work-adapters/tests work-cli/tests` returns nothing.
- [x] Baseline guards pass in the default lane: `cd cli && cargo nextest run -p work-adapters the_baseline_still_describes_the_fixture_corpus every_relocated_corpus_is_covered_by_a_recorded_row every_recorded_parity_test_still_exists_with_its_recorded_count every_committed_golden_matches_its_recorded_hash`.
- [x] The `--all-features` lane is green (the two diff suites are gone, others still shell `diff`/`jj`/`git`): `mise run test:unit:cli`.
- [x] Read-only CI mirror passes: `mise run check`.

#### Manual Verification

- [x] `git`/`jj` status shows only the intended deletions, the manifest + guard edits, the reworded `work-adapters/Cargo.toml` comment, and the removed `work-cli/Cargo.toml` feature — no fixture left orphaned.

---

## Phase 2: In-process `similar` renderer, infallible

### Overview

Add `similar`, rewrite the module as `work_adapters::diff` with an infallible
`render`, and ripple the signature change through all four edited files. Delete
the subprocess unit tests; add in-process unit tests and the AC3 golden. One
atomic compiling change — the signature change forces every edit site to move
together. Satisfies AC1–AC5, AC8, and the *rewrite* half of AC6.

Follow red-green-refactor: write the new in-process unit tests and the CLI
golden test first (they fail to compile against the old `Result` signature),
then make them pass by rewriting the module and its consumers.

### Changes Required

#### 1. Add the `similar` workspace dependency

**File**: `cli/Cargo.toml` (`[workspace.dependencies]`)
**Changes**: add `similar`, default features only.

```toml
# In-process text differ. Default features only: `text` is dependency-free;
# `bytes`/`unicode` pull extra crates and are not needed. Exact-pinned because
# the section-diff goldens assert its unified-diff output byte-for-byte, so a
# formatting change in a new release must arrive as a deliberate re-bless.
similar = "=2.7.0"
```

Exact-pin, matching the workspace discipline for output-sensitive crates
(`clap`, `tar`, `jj-lib`, `octocrab` all carry `=` pins); carets are reserved
for crates whose output is not byte-asserted. `2.7.0` is the version the plan's
`similar` output claims were verified against.

**File**: `cli/work-adapters/Cargo.toml` (`[dependencies]`)

```toml
similar = { workspace = true }
```

Verify the pin adds no packages and no new licence:

```bash
cd cli && cargo tree -p similar --edges normal && cargo deny check licenses
```

#### 2. Rename and rewrite the module

**File**: rename `cli/work-adapters/src/diff_shellout.rs` → `cli/work-adapters/src/diff.rs`, body replaced.

```rust
use similar::TextDiff;
use work::section_diff::SectionDiff;

/// Renders one section's `=== name (- LOCAL / + REMOTE) ===` header plus an
/// in-process unified-diff body. The header and blank-line framing is the
/// contract callers depend on; the body is `similar`'s hunk output.
pub fn render(diff: &SectionDiff) -> String {
    let body = TextDiff::from_lines(&diff.local, &diff.remote)
        .unified_diff()
        .context_radius(3)
        .to_string();

    let mut out = format!("=== {} (- LOCAL / + REMOTE) ===\n", diff.name);
    out.push_str(&body);
    out.push('\n');
    out
}
```

Feed `diff.local`/`diff.remote` to `from_lines` **verbatim** — they arrive
already newline-terminated from `differing_sections` (and `""` for an absent
section). Re-appending `\n` would double the terminator, inject a phantom blank
context line, and widen the hunk header from `@@ -1 +1 @@` to `@@ -1,2 +1,2 @@`.
The one-sided `""` case gives `similar` zero lines on that side, so no
`\ No newline at end of file` marker appears. `from_lines` forces
`newline_terminated`, so `similar`'s body is empty or ends in `\n` for every
input `differing_sections` can produce; the single unconditional `push('\n')`
after it emits the framing's trailing blank line in both cases. No trailing
guard is needed — an earlier draft carried one justified as protecting a
non-terminated field, but such a field would instead leak a `\ No newline`
marker (a body still ending in `\n`), which the guard did not address.

Gone with the subprocess: `DiffUnavailable`, `DEFAULT_CAP`, `POLL_INTERVAL`,
`run_capped`, `render_with`, and the `std::process`/`Stdio`/`io::Read`/
`tempfile`/`Instant` imports.

**File**: `cli/work-adapters/src/lib.rs`
**Changes**: rename the module and reword the crate docstring away from
"subprocess-shelling".

```diff
-//! Outbound adapters for the `work` domain crate: filesystem reads and the
-//! subprocess-shelling modules (`diff`, VCS identity).
+//! Outbound adapters for the `work` domain crate: filesystem reads,
+//! in-process section diffing, and VCS-derived authorship.

 pub mod author;
-pub mod diff_shellout;
+pub mod diff;
 pub mod filesystem;
 pub mod sync;
```

`render` becomes a pure function (`&SectionDiff -> String`, no I/O) yet stays in
`work-adapters` rather than moving to the `work` domain crate. This is a
deliberate tradeoff: keeping it here confines the `similar` dependency to the
adapter crate and leaves the dependency-light domain crate untouched, at the
cost of a pure renderer living beside genuine adapters. The alternative — a
presentation module in `work` — would pull `similar` into the domain and widen
the change for no present consumer benefit.

#### 3. `work diff` command consumer

**File**: `cli/work-cli/src/diff.rs`
**Changes**: reword the module docstring's `work_adapters::diff_shellout::render`
reference to `work_adapters::diff::render`, import the renamed module, drop the
`DiffUnavailable` outcome, and render infallibly.

```diff
-//! `work::section_diff::differing_sections`, and each differing section's
-//! rendering from `work_adapters::diff_shellout::render`.
+//! `work::section_diff::differing_sections`, and each differing section's
+//! rendering from `work_adapters::diff::render`.
```

```diff
-use work_adapters::diff_shellout::render;
+use work_adapters::diff::render;

 pub enum RunOutcome {
     Rendered(String),
     NonFileArgument,
-    DiffUnavailable,
 }
```

```diff
     let mut out = String::new();
     for diff in &diffs {
-        match render(diff) {
-            Ok(rendered) => out.push_str(&rendered),
-            Err(_) => return RunOutcome::DiffUnavailable,
-        }
+        out.push_str(&render(diff));
     }
     RunOutcome::Rendered(out)
```

**File**: `cli/work-cli/src/main.rs`
**Changes**: remove the now-unreachable arm in `run_diff` (`:167-173`).

```diff
         diff::RunOutcome::NonFileArgument => {
             eprintln!("work-item-section-diff: both arguments must be files");
             ExitCode::from(2)
         }
-        diff::RunOutcome::DiffUnavailable => {
-            eprintln!(
-                "`diff` is required on PATH for `work diff`; install it or \
-                 check PATH"
-            );
-            ExitCode::FAILURE
-        }
```

#### 4. Sync/dossier engine consumer

**File**: `cli/work-adapters/src/sync/run.rs`
**Changes**: make the injected renderer infallible, simplify `render_dossier`,
remove the now-dead `raw_sections`, remove the `DiffUnavailable` import, and
reword `unrenderable_header` to describe its only surviving trigger — an
unreadable local file, not a missing binary.

```diff
-use crate::diff_shellout::DiffUnavailable;
```

```diff
 fn unrenderable_header(dossier: &ConflictDossier) -> String {
     format!(
-        "{}This conflict could not be rendered (the `diff` tool was \
-         unavailable). Item {} was left unresolved. Install `diff` and re-run \
-         the sync, or edit the work item by hand.\n\n",
+        "{}This conflict could not be rendered: the local file could not be \
+         read. Item {} was left unresolved. Fix the file and re-run the sync, \
+         or edit the work item by hand.\n\n",
         dossier_header(dossier, "unrenderable"),
         dossier.id,
     )
 }
```

```diff
 pub fn render_dossier(
     dossier: &ConflictDossier,
-    render: &dyn Fn(&SectionDiff) -> Result<String, DiffUnavailable>,
+    render: &dyn Fn(&SectionDiff) -> String,
 ) -> DossierRender {
     if dossier.local_unreadable {
         return DossierRender::Unrenderable(unrenderable_header(dossier));
     }
     let mut sections = String::new();
     for section in &dossier.sections {
-        match render(section) {
-            Ok(body) => sections.push_str(&body),
-            Err(DiffUnavailable) => {
-                return DossierRender::Unrenderable(
-                    unrenderable_header(dossier) + &raw_sections(dossier),
-                );
-            }
-        }
+        sections.push_str(&render(section));
     }
     DossierRender::Renderable(renderable_header(dossier) + &sections)
 }
```

Delete `raw_sections` (its only caller was the removed `Err` arm). Update the
`render_dossier` doc comment to drop the "failing section downgrades" clause —
the only downgrade path is now `local_unreadable`.

Tests in this file: delete
`a_failing_renderer_downgrades_to_unrenderable_with_raw_values` (no failing
renderer is possible); change `ok_renderer` to return `String`; keep
`a_local_unreadable_dossier_is_unrenderable_without_rendering` (its
`must_not_render` closure returns `String`, and it still asserts `left
unresolved`).

Deleting the failing-renderer test removes the only `render_dossier` case that
passes **non-empty** sections through the renderer — the surviving Renderable
tests use `Vec::new()` sections, so the `sections.push_str(&render(section))`
loop body becomes mutation-survivable. Give one surviving Renderable test
(`ok_renderer` now returning `String`) a dossier with at least one section and
assert the rendered body text lands under the header, so the concatenation loop
stays covered.

#### 5. Sync CLI wiring consumer

**File**: `cli/work-cli/src/sync.rs`
**Changes**: import the renamed module, make the two `persist_*` renderer
parameters infallible, drop the `DiffUnavailable` import, and update the test
helper.

```diff
-use work_adapters::diff_shellout::DiffUnavailable;
```

```diff
 fn persist_dossiers(
     dossiers: &[ConflictDossier],
     dir: &Path,
     scheme: &WorkItemIdScheme,
-    render: &dyn Fn(&SectionDiff) -> Result<String, DiffUnavailable>,
+    render: &dyn Fn(&SectionDiff) -> String,
 ) {
```

Apply the same signature change to `persist_conflict_dossiers`. The production
injection site (`:480`) becomes `&work_adapters::diff::render`, which coerces
`fn(&SectionDiff) -> String` to the `&dyn Fn` parameter. Change the test helper
`ok_render` to return `String` and drop `use super::DiffUnavailable`.

#### 6. Module tests: replace the subprocess unit tests

**File**: `cli/work-adapters/src/diff.rs` (`#[cfg(test)] mod tests`)
**Changes**: delete the three subprocess tests (`renders_the_header_and_diff_body`
via the spawn path, `a_missing_diff_binary_is_reported`,
`a_hanging_diff_is_killed_at_the_cap`). Add in-process tests.

```rust
#[test]
fn renders_the_frozen_header_and_a_unified_body() {
    let section = diff("Summary", "local summary\n", "remote summary\n");
    let rendered = render(&section);
    assert!(rendered.starts_with("=== Summary (- LOCAL / + REMOTE) ===\n"));
    assert!(rendered.contains("@@ -1 +1 @@"));
    assert!(rendered.contains("-local summary"));
    assert!(rendered.contains("+remote summary"));
    assert!(!rendered.contains("No newline at end of file"));
    assert!(rendered.ends_with("\n\n"));
}

#[test]
fn identical_sides_render_an_empty_body_under_the_header() {
    let section = diff("Summary", "same text\n", "same text\n");
    let rendered = render(&section);
    assert_eq!(rendered, "=== Summary (- LOCAL / + REMOTE) ===\n\n");
}

#[test]
fn a_section_added_on_the_remote_renders_a_pure_insertion() {
    let section = diff("Summary", "", "remote only\n");
    let rendered = render(&section);
    assert!(rendered.starts_with("=== Summary (- LOCAL / + REMOTE) ===\n"));
    assert!(rendered.contains("@@ -0,0 +1 @@"));
    assert!(rendered.contains("+remote only"));
    assert!(!rendered.contains("No newline at end of file"));
    assert!(rendered.ends_with("\n\n"));
}

#[test]
fn a_section_dropped_on_the_remote_renders_a_pure_deletion() {
    let section = diff("Summary", "local only\n", "");
    let rendered = render(&section);
    assert!(rendered.contains("@@ -1 +0,0 @@"));
    assert!(rendered.contains("-local only"));
    assert!(!rendered.contains("No newline at end of file"));
}
```

`diff(name, local, remote)` is the existing test helper (`SectionDiff`
constructor); keep it, and feed it **newline-terminated** inputs (`""` for an
absent side) so tests exercise the production shape — the helper stores its args
verbatim, so a non-terminated input would emit a `\ No newline` marker that
`differing_sections` never produces.

#### 7. Extend the existing CLI-boundary suite (AC3)

**File**: `cli/work-cli/tests/cli_diff.rs` — the existing non-`bash-parity`
suite, which already spawns `accelerator-work` over inline temp-file inputs and
asserts inline. Its `no_differences_after_normalisation` test already covers AC5,
and its `frontmatter_only_change_is_shown` test asserts structurally
(`starts_with` + `contains("-status: …")`), so both survive the body-format
change unchanged. Add one exact-match test for AC3, following the file's
established inline style (`assert_eq!` against a `const`, as in `cli_show.rs`) —
not a committed `.golden` file, which the project reserves for large stable
outputs (`cli_surface.golden`, `sync-report.golden`).

```rust
#[test]
fn one_differing_section_renders_a_unified_body_with_the_frozen_framing(
) -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    let local = dir.path().join("local.md");
    let remote = dir.path().join("remote.md");
    fs::write(
        &local,
        "---\nstatus: draft\n---\n## Summary\nlocal text\n## Notes\nsame\n",
    )?;
    fs::write(
        &remote,
        "---\nstatus: draft\n---\n## Summary\nremote text\n## Notes\nsame\n",
    )?;
    let output = run(&[
        "diff",
        local.to_str().ok_or("non-utf8")?,
        remote.to_str().ok_or("non-utf8")?,
    ])?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;

    assert_eq!(stdout, EXPECTED);
    Ok(())
}
```

`EXPECTED` is a Rust-output golden held inline — the opposite discipline from
the retired frozen bash oracle. The exact `=2.7.0` pin makes `assert_eq!`
against `similar`'s hunk bytes safe: the output cannot shift under a silent
dependency bump, so an exact-match red always signals a real behaviour change,
never a benign upgrade. With the fields fed verbatim, its confirmed value (one
differing section, single-line hunk, no phantom context) is:

```text
=== Summary (- LOCAL / + REMOTE) ===
@@ -1 +1 @@
-local text
+remote text

```

Add a second inline case for the `last_updated`-only frontmatter change that the
retired `cli_diff_parity` suite covered end-to-end — the `differing_sections`
unit tests assert only that such a change is *detected*, not that it *renders*
into the body, so without this case no surviving test proves the detect→render
pipeline for that `IGNORE_KEYS`-adjacent field. Two documents identical but for
`last_updated:` in the frontmatter should render a `frontmatter` section whose
body carries the `-last_updated: …`/`+last_updated: …` lines.

Before blessing `EXPECTED` and the unit-test hunk headers, run `render` (or
`TextDiff::from_lines`) once on the exact newline-terminated inputs and transcribe
the observed output rather than reconstructing it by hand — the `@@ -1 +1 @@`,
`@@ -0,0 +1 @@`, and `@@ -1 +0,0 @@` shapes were confirmed against `similar`
2.7.0, and a transcribed golden guarantees the assertions match real output on
first run.

The existing `run` helper and `TestError` alias in the file are reused.

### Success Criteria

#### Automated Verification

- [x] `similar` adds no packages and no new licence: `cd cli && cargo tree -p similar --edges normal` and `cargo deny check licenses`.
- [x] No subprocess or temp-file surface in the crate's source: `cd cli/work-adapters/src && grep -rn "std::process\|Command::new\|Stdio\|DiffUnavailable" .` returns nothing.
- [x] Adapter and CLI tests pass without a `diff` binary. The plan's stripped-`PATH` form is infeasible on this host (nextest and the C linker live outside the toolchain dir, and the create/sync suites legitimately spawn `git`/`jj`), so the property is proven instead by the empty-`PATH` binary render below (AC8, stronger — no external binary is reachable) plus the source grep above.
- [x] Formatting and clippy clean: `mise run cli:check`.
- [x] Full `--all-features` lane green: `mise run test:unit:cli`.
- [x] Read-only CI mirror passes: `mise run check`.

#### Manual Verification

- [x] `accelerator work diff <a.md> <b.md>` on two work items differing in one section prints the unchanged `=== … ===` framing with a `@@` hunk and `-`/`+` lines.
- [x] `env PATH= accelerator work diff <a.md> <b.md>` (empty `PATH`) still renders — proving no `diff` binary is reached (AC8).
- [x] `accelerator work diff` on two identical files prints `(no differing sections after normalisation)`.

---

## Phase 3: Crate-wide zero-spawn pup rule

### Overview

Establish the zero-spawn invariant with three complementary guards, added
**alongside** the retained module-scoped `work_adapters_filesystem_reads_in_process`
allow-list: a crate-wide `work_adapters_is_zero_spawn` pup rule, a probe pair in
`test_import_rule.py` that guards the rule against a mistyped or deleted regex,
and a runtime `zero_spawn.rs` harness that exercises the crate's real render
paths under a spawn-tripwire `PATH`. The pup rule catches `use`-path
`std::process` imports at build time; the harness closes its blind spot — an
inline `std::process::Command::new()` the import rule cannot see — by proving no
exercised path spawns at run time. The allow-list keeps `filesystem.rs` confined
to its std/domain import set, and composes with the crate-wide deny without
conflict. Satisfies AC7. Lands only after Phase 2 makes the crate zero-spawn.

Follow red-green-refactor: write the probe pair and the harness first (they fail
until the rule exists and the crate is genuinely zero-spawn), then add the rule.

### Changes Required

#### 1. Add the crate-wide rule

**File**: `cli/pup.ron`
**Changes**: add the crate-wide `Module` block below, **keeping** the existing
`work_adapters_filesystem_reads_in_process` allow-list unchanged. Also update
the stale comment on that existing rule (`pup.ron:284`), which still names
`diff_shellout` as a spawning module.

This is the workspace's first *crate-wide* zero-spawn deny — every existing
sibling (`work_adapters_filesystem_reads_in_process`,
`vcs_adapters_library_reads_in_process`) is deliberately module-scoped so other
modules may spawn by design. The crate-wide scope is a deliberate tightening,
justified because the whole crate is now zero-spawn; it also spans the crate's
in-`src` `#[cfg(test)] mod tests`, so any future in-`src` test helper needing a
subprocess would trip the rule with no module escape hatch. That is acceptable
today (the new unit tests use no `std::process`); `mise run pup:check` confirms
the broadened rule flags no in-`src` test code.

```ron
// work-adapters spawns no subprocess: its section-diff renders in-process and
// its VCS identity reads through vcs-adapters' library path. This crate-wide
// deny catches use-path std::process imports; work-adapters/tests/zero_spawn.rs
// closes the inline-Command::new() blind spot at run time.
Module((
    name: "work_adapters_is_zero_spawn",
    matches: Module("^work_adapters($|::)"),
    rules: [
        RestrictImports(
            allowed_only: None,
            denied: Some([
                "^std::process(::|$)",
            ]),
            severity: Error,
        ),
    ],
)),
```

#### 2. Register the rule with a probe pair

**File**: `tests/integration/pup/test_import_rule.py`
**Changes**: add a violation/compliant probe pair for `work_adapters_is_zero_spawn`,
driven against the real `cli/pup.ron` via `--pup-config`, mirroring the shipped
`test_remote_projection_rule_rejects_spawning` /
`test_remote_projection_rule_permits_json_extraction` pair. `tasks/README.md`
documents this as the required registration surface — there is no coverage guard
for `pup.ron`, so a mistyped or deleted rule is otherwise silent.

Reuse the file's existing `_write_shared_crate_probe(root, crate, lib_body)`
helper with `crate="work-adapters"` and the existing `_SPAWN_VIOLATION`
(`use std::process::Command; … Command::new("jq")`) for the rejection case, and a
`std`-only compliant body for the pass case:

```python
def test_work_adapters_zero_spawn_rule_rejects_spawning(tmp_path: Path) -> None:
    _require_tools()
    _write_shared_crate_probe(tmp_path, "work-adapters", _SPAWN_VIOLATION)
    result = _pup("--pup-config", str(CLI_PUP_RON), cwd=tmp_path)
    output = _ANSI.sub("", result.stdout + result.stderr)
    assert result.returncode != 0, output
    assert "is denied" in output, output
    assert "work_adapters_is_zero_spawn" in output, output


def test_work_adapters_zero_spawn_rule_permits_std_imports(
    tmp_path: Path,
) -> None:
    _require_tools()
    _write_shared_crate_probe(tmp_path, "work-adapters", _PROJECTION_COMPLIANT)
    result = _pup("--pup-config", str(CLI_PUP_RON), cwd=tmp_path)
    assert result.returncode == 0, _ANSI.sub("", result.stdout + result.stderr)
```

The sibling `work_adapters_filesystem_reads_in_process` rule currently has no
probe pair; leaving it unguarded is out of scope here, but note it as a
follow-up so the crate's rule surface is eventually fully covered.

#### 3. Add the runtime zero-spawn harness

**File**: `cli/work-adapters/tests/zero_spawn.rs` (new)
**Changes**: add a `bash-parity`-gated **in-process** marker-only tripwire. It
sets the test process's own `PATH` to a temp directory holding a spawn tripwire
— an executable literally named `diff` (plus `sh`/`bash` for common shell-outs)
that writes a marker file when run — then calls `diff::render` over a two-sided
`SectionDiff` and `sync::run::render_dossier` over a dossier with a section
directly, in-process. Assert the marker was never written. This is the guard the
import rule cannot provide: an inline `std::process::Command::new("diff")` added
to an exercised path later resolves through the tripwire, writes the marker, and
reds the harness, even though `RestrictImports` sees no `use std::process`.

Do **not** follow `corpus-adapters/tests/zero_spawn.rs` literally: that harness
is a black-box design spawning a dedicated `[[bin]]` fixture and applying a stub
`PATH` to that child, because `corpus-adapters` genuinely reads `git`/`jj`
in-process and must disprove degradation to a shell-out. `diff::render` is pure
text diffing with nothing to degrade, so the fixture-binary and
output-comparison halves do not port — the natural shape here is the in-process
tripwire above, with no new `[[bin]]` target and no registration surface. The
process-global `PATH` mutation is safe because nextest runs each test in its own
process. Keep the `bash-parity` gate so the tripwire tooling runs only in the
`--all-features` lane.

Scope and its boundary: this runtime guard covers only the two paths the crate
actually composes for rendering. `render` is pure and consults no `PATH`, so the
harness earns its keep as a *regression* guard for future code, not a check on
today's provably-pure renderer. An inline spawn added later to a **non-rendering**
module (`author.rs`'s identity read, `filesystem.rs`) would escape both the
use-path pup rule and this harness — a deliberate, documented boundary; those
modules rely on the pup rule and code review, and extending the tripwire to
`author`'s path is a reasonable follow-up if that surface ever grows.

Limitation: like the `corpus-adapters` precedent, the tripwire catches only
`PATH`-resolved bare-command spawns; a spawn by absolute path would not fire the
marker. This matches the threat model — a re-introduced `diff` shell-out would
reach for the bare name.

### Success Criteria

#### Automated Verification

- [x] The cargo-pup check passes: `mise run pup:check` (the cargo-pup lane, declared in `mise.toml` and documented in `tasks/README.md`).
- [x] The probe pair passes against the real config: `mise run test:integration:pup` — the rejection case names `work_adapters_is_zero_spawn`, the compliant case exits 0.
- [x] The runtime harness passes in the `--all-features` lane: `mise run test:unit:cli` runs `zero_spawn.rs` and the marker is never written.
- [x] Read-only CI mirror passes: `mise run check`.
- [x] Full local CI mirror is green end-to-end, no `diff` binary required: `mise run`. Every lane passed; the sole failure was a confirmed load-induced flake in `test:integration:visualiser`'s `api_smoke` (the server's 30s startup budget exceeded only under the full run's parallel load), which passes standalone (`mise run test:integration:visualiser`, 3 passed).

#### Manual Verification

- [x] The crate-wide rule has teeth beyond the allow-list: temporarily add `use std::process::Command;` to a **non-`filesystem`** `work-adapters` `src/` module (e.g. `author.rs`), confirm the cargo-pup check errors on `work_adapters_is_zero_spawn`, then revert.
- [x] The harness has teeth beyond the rule: temporarily add an inline `let _ = std::process::Command::new("diff").status();` to `diff::render` (no `use`, so the pup rule stays silent), confirm `zero_spawn.rs` fails on the written marker, then revert.

---

## Testing Strategy

### Unit Tests

- `render` emits the frozen header, a `@@ -1 +1 @@` hunk, and `-`/`+` prefixes
  for a two-sided differing section, with no phantom trailing blank context line
  (`cli/work-adapters/src/diff.rs`).
- A one-sided section (`local == ""` or `remote == ""`) renders a pure
  insertion/deletion with no `\ No newline at end of file` marker — re-covering
  the retired `case-named-heading-and-one-sided` fixture.
- Identical sides render an empty body under the header — the header plus the
  trailing blank line only (AC5 at the renderer level).
- `render_dossier`'s surviving downgrade path: an unreadable local file yields
  `Unrenderable` without invoking the renderer (`sync/run.rs` tests).
- `render_dossier`'s Renderable happy path: a dossier with a non-empty section
  renders that section's body under the header — keeping the concatenation loop
  covered after the failing-renderer test is deleted (`sync/run.rs` tests).

### Integration Tests

All in the existing `cli/work-cli/tests/cli_diff.rs`, extending its inline
temp-file-input style:

- New (AC3): `work diff` over a two-section document where one heading differs —
  full stdout equals an exact inline `EXPECTED`, and carries `@@`.
- New: `work diff` over two documents differing only in the `last_updated`
  frontmatter field renders a `frontmatter` section whose body carries the
  `-last_updated: …`/`+last_updated: …` lines — proving the detect→render
  pipeline for that `IGNORE_KEYS`-adjacent field, which the retired
  `cli_diff_parity` suite covered end-to-end.
- Already present (AC5): `no_differences_after_normalisation` — two identical
  sections report `(no differing sections after normalisation)`.
- Already present, survives unchanged: `frontmatter_only_change_is_shown`
  (structural `-`/`+` assertions), plus the two argument-validation exit-code
  tests.

`differing_sections`' own unit tests (`cli/work/src/section_diff.rs:216-262`)
still cover *detection* of frontmatter-only and `last_updated`-only changes; the
new `last_updated` CLI case above covers their *rendering* end-to-end, which the
unit tests do not.

### Manual Testing Steps

1. `accelerator work diff` on two real work items differing in one section —
   eyeball the unchanged framing and the `@@`/`-`/`+` body.
2. `env PATH= accelerator work diff …` — confirm it still renders with `diff`
   absent from `PATH`.
3. `accelerator work sync --preview` against a repo with a conflicting item —
   confirm a conflict dossier still renders each section, and that an item with
   an unreadable local file reports the reworded "the local file could not be
   read" message.

## Performance Considerations

In-process `similar` replaces a fork+exec, two temp-file writes, and a 10ms
poll loop capped at 10s. The new path allocates two `String`s and runs Myers
diff over a section's lines — microseconds against the former ~milliseconds of
process spawn. No cap is needed; there is no unbounded external process.

## Migration Notes

None for shipped consumers. The change is internal to the CLI; no on-disk
format, config, or public surface changes. The `work diff` body format changes
(hunks without `---/+++` file lines), but no live consumer parses it.

One breadcrumb for the future 0213 conflict-resolution flow (unimplemented, so
nothing breaks today): its research references the old integration point
`work_adapters::diff_shellout::render` returning `Result<String, DiffUnavailable>`
over a body with `---/+++` file lines. All three are superseded — the call site
is now `work_adapters::diff::render(&SectionDiff) -> String` and the body is
`similar`'s hunk-only output. 0213's author should pick up the new path and, if
it parses the body, pin the format deliberately rather than inheriting whatever
`similar` emits.

## References

- Original work item: `meta/work/0201-in-process-section-diff.md`
- Research: `meta/research/codebase/2026-08-30-0201-in-process-section-diff.md`
- Current implementation: `cli/work-adapters/src/diff_shellout.rs:33-102`
- Consumers: `cli/work-cli/src/diff.rs:41-44`, `cli/work-adapters/src/sync/run.rs:258-277`, `cli/work-cli/src/sync.rs:285,318,480`
- Isolation rule: `cli/pup.ron:283-304`
- Parity teardown surface: `cli/work-adapters/tests/bash_parity_baseline.rs`, `cli/work-adapters/tests/fixtures/bash-parity-baseline.txt`
- Error-model precedent: `cli/vcs-adapters/src/library.rs` (0188/0198)
- Architecture: `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`
