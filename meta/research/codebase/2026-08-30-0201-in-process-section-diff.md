---
type: codebase-research
id: "2026-08-30-0201-in-process-section-diff"
title: "Research: In-Process Section Diff (0201)"
date: "2026-08-30T13:02:27+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0201"
parent: "work-item:0201"
relates_to: ["work-item:0170", "work-item:0188", "work-item:0198", "work-item:0174"]
topic: "Replacing the subprocess diff -u section renderer with an in-process Rust implementation"
tags: [research, codebase, work-adapters, diff, cargo-pup, cargo-deny]
revision: "804b2a5b972bd17669adf2ab60d150f490333f26"
repository: "accelerator"
last_updated: "2026-08-30T13:02:27+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: In-Process Section Diff (0201)

**Date**: 2026-08-30T13:02:27+00:00
**Author**: Toby Clemson
**Git Commit**: 804b2a5b972bd17669adf2ab60d150f490333f26
**Branch**: unnamed working-copy change off `main` (no bookmark)
**Repository**: accelerator

## Research Question

What does replacing `cli/work-adapters/src/diff_shellout.rs`'s subprocess
`diff -u` with an in-process Rust diff actually touch? Specifically: who
consumes `render`, whether the body format is a frozen contract, what the
`pup.ron` isolation rule really guards, what the `bash-parity` artefacts are,
whether a diff crate can be added without a cargo-deny edit, and what the
0188/0198 VCS migration gives us as a code template.

## Summary

The swap is larger than "edit one file" in one specific way and smaller in
another. **`render` has two consumers, not one** — the `work diff` command
*and* the sync/dossier engine, both threading `Result<String, DiffUnavailable>`
— so removing `DiffUnavailable` ripples through the sync path's error handling.
Against that, **no live consumer parses the diff body**: the only reader of
`work diff` output is a *planned* (unimplemented) 0213 skill flow, so the
work item's central assumption holds and the body format is genuinely free to
change.

Three findings sharpen the plan beyond the work item's text:

- ⚠️ **The `pup.ron` rule does not guard `diff_shellout`.**
  `work_adapters_filesystem_reads_in_process` matches `work_adapters::filesystem`
  and forbids *that* module from importing `std::process`. `diff_shellout` is
  deliberately left outside its scope. Deleting the rule removes `filesystem.rs`'s
  purity guard, not a quarantine on the spawner. Once the crate spawns nothing,
  the right move is a crate-wide zero-spawn guard (the `corpus-adapters`
  `zero_spawn.rs` test is the precedent), not a bare deletion.
- **Dependency choice is a real fork.** `similar` is a genuinely new pinned
  package (license-clean, no deny edit needed); the `imara-diff` algorithm is
  *already* in the lock as `gix-imara-diff` via `gix`, so reaching it adds zero
  packages but binds to a gix-internal surface.
- **The `bash-parity` teardown spans three files**, not one: the feature flag,
  `diff_shellout_parity.rs`, and the `bash_parity_baseline.rs` manifest that
  counts cases and per-test `#[test]` totals — retiring the diff parity suite
  shifts those counts.

The 0188/0198 `vcs-adapters` crate is the direct code template for the error
model (narrow adapter `enum Error` + `From<Error> for kernel::Error`, fallible
vs `Option` by caller need).

## Detailed Findings

### Current implementation (`diff_shellout.rs`)

`render` → `render_with` → `run_capped` writes `LOCAL`/`REMOTE` temp files and
spawns `diff -u LOCAL REMOTE` under a 10s cap with a 10ms poll loop, tolerating
`diff`'s exit-1-means-differ convention
(`cli/work-adapters/src/diff_shellout.rs:33-102`). Output framing:

```text
=== {name} (- LOCAL / + REMOTE) ===
{diff -u body}
{blank line}
```

The header/blank-line framing is the frozen contract (`diff_shellout.rs:58-63`);
the body between them is GNU diffutils' hunk output, which 0201 explicitly
un-freezes. `DiffUnavailable` (`diff_shellout.rs:24`) collapses three distinct
failures — temp-dir/write error, spawn failure, cap timeout — into one unit
struct. `DEFAULT_CAP`/`POLL_INTERVAL` (`:19-20`) and the three subprocess unit
tests (spawn-failure `:135`, cap-timeout `:148`) all disappear with the
subprocess.

### `render` has two consumers, not one

The work item frames `cli/work/src/section_diff.rs` as the consumer, but that
crate only produces `SectionDiff` values (`differing_sections`,
`section_diff.rs:176`) — it never calls `render`. The actual `render` callers:

| Consumer | Site | Handling of `DiffUnavailable` |
| --- | --- | --- |
| `work diff` command | `cli/work-cli/src/diff.rs:41-44` | maps to `RunOutcome::DiffUnavailable` → `main.rs:167` |
| Sync/dossier engine | `cli/work-adapters/src/sync/run.rs:258-277` | maps to `DossierRender::Unrenderable` fallback |
| Sync CLI wiring | `cli/work-cli/src/sync.rs:285,318,480` | injects `render` as `&dyn Fn(...) -> Result<_, DiffUnavailable>` |

⚠️ **The sync path injects `render` by trait object**
(`render: &dyn Fn(&SectionDiff) -> Result<String, DiffUnavailable>`,
`sync/run.rs:260`). Making `render` infallible changes that closure type and
every injection site in `work-cli/src/sync.rs`, plus the `render_dossier` /
`persist_dossiers` / `persist_conflict_dossiers` signatures.

One subtlety for the plan: `DossierRender::Unrenderable` has a **second
trigger** — `dossier.local_unreadable` (`sync/run.rs:262`) — so the variant
survives even after the `Err(DiffUnavailable)` arm (`:269`) is deleted. Removing
`DiffUnavailable` prunes one of its two entry paths, not the variant itself.

### No live consumer parses the diff body

Consumer scan across `skills/`, `agents/`, `templates/`, `hooks/`: no runtime
reader of `work diff` output. The only structural reference is a *planned*
0213 conflict-resolution flow described in
`meta/research/codebase/2026-08-18-0213-...md:147,276-279` — it would call
`accelerator work diff <local> conflicts/<id>.md` and render via
`work_adapters::diff_shellout::render`, but nothing in `skills/` implements it
yet. The work item's Assumption ("the body is an internal rendering contract,
not a byte-for-byte frozen format some downstream skill parses") holds: the body
is free to change. The `work-cli` golden `cli_surface.golden:57,60` fixes only
the command's help/surface text, not body output.

### What `pup.ron` actually guards

⚠️ The rule's name and the work item both suggest it quarantines the
subprocess module. It does not.

```ron
// cli/pup.ron:288-304
Module((
    name: "work_adapters_filesystem_reads_in_process",
    matches: Module("^work_adapters::filesystem($|::)"),
    rules: [RestrictImports(
        allowed_only: Some(["^(std|core|alloc)(::|$)", "^work(::|$)", "^crate(::|$)"]),
        denied: Some(["^std::process(::|$)"]),
        severity: Error,
    )],
)),
```

`matches` targets `work_adapters::filesystem` — the pure-read module — and
forbids *it* from importing `std::process`. The comment (`pup.ron:283-287`)
states the intent: keep `filesystem` pure "so the sibling modules can spawn by
design", explicitly naming `diff_shellout` as a module left *outside* the rule.
So:

- Deleting the rule does not stop `diff_shellout` spawning (it never did — the
  rule doesn't cover it). It removes the guard that keeps `filesystem.rs` pure.
- ⚠️ The rule matches on `use`-path imports only. It does not catch an inline
  fully-qualified `std::process::Command::new()` — the `vcs_adapters` sibling
  comment (`pup.ron:310-314`) records the same caveat and says "the harness is
  what establishes zero-spawn."

**Recommendation for AC7**: once `diff_shellout` no longer spawns, `work-adapters`
has zero subprocess spawns crate-wide (confirmed below). Rather than delete the
rule, replace it with the crate-wide zero-spawn guard pattern — a harness test
like `cli/corpus-adapters/tests/zero_spawn.rs` — which is enforceable against
inline-qualified spawns in a way the import rule is not. Narrowing-vs-deleting
is the work item's Open Question 2; this answers it.

### `work-adapters` spawns nowhere else

Workspace-wide grep for `std::process` / `Command::new` / `.spawn(` matched
**only** `diff_shellout.rs:12,13,52,79`. Every other module — `author.rs`,
`filesystem.rs`, all `sync/*` — is spawn-free. So after this change the crate is
genuinely zero-spawn, and Open Question 2 ("is `diff_shellout` the only spawner")
resolves to **yes**.

Note: `cli/vcs-adapters/src/subprocess.rs` has its own unrelated `run_capped`.
Any plan text must disambiguate — the diff `run_capped` is private to
`diff_shellout.rs`.

### The `bash-parity` teardown spans three files

- **Feature flag**: `cli/work-adapters/Cargo.toml:16` (`bash-parity = []`),
  enabled by `tasks/test/cli.py`. It gates test targets only — no production
  code is `cfg(feature = "bash-parity")`.
- **The parity suite**: `cli/work-adapters/tests/diff_shellout_parity.rs`,
  whole-file gated (`:5`), asserts `render` output against fixtures under
  `tests/fixtures/work-item-section-diff/` and requires the real `diff` binary
  on `PATH`. Companion CLI suite: `cli/work-cli/tests/cli_diff_parity.rs`.
- **The baseline manifest**: `cli/work-adapters/tests/bash_parity_baseline.rs`
  (not itself feature-gated) guards the committed baseline
  `tests/fixtures/bash-parity-baseline.txt` — case-dir sets, per-test `#[test]`
  counts, golden byte-identity hashes (`:36,53`). Retiring the diff parity suite
  changes the enumerated suites (`bash-parity-baseline.txt:36,39`), the five
  cases (`:50-54`), and their hashes (`:78-82`), so this manifest must be
  updated in lockstep.

⚠️ `cli/work-adapters/tests/sync_working_copy_status.rs:32` also has a
`bash-parity` gate, but for VCS real-repo tests — unrelated to diff. Do not
remove the feature flag itself while that gate remains; scope the teardown to
the diff-specific suites.

### Dependency choice: `similar` vs the already-present `imara-diff`

- ✅ **cargo-deny**: `MIT` and `Apache-2.0` are both in the blanket allow-list
  (`cli/deny.toml:52-64`); `similar`/`imara-diff`/`diffy` are not in the `[bans]`
  deny list. A new MIT/Apache-2.0 crate needs **no** allow-list edit. Assumption
  in the work item confirmed.
- **`similar`** — a genuinely new package: a new `[workspace.dependencies]` pin,
  a new lockfile entry, a new line in the pin-review discipline. Ships
  `TextDiff::unified_diff()` producing `@@` headers and `-`/`+` prefixes directly
  — a near-drop-in for the frozen framing.
- **`imara-diff`** — the algorithm is *already compiled in* as `gix-imara-diff`
  v0.2.4 (`cli/Cargo.lock:1781`, pulled by `gix`). Reaching it through `gix`
  adds zero packages (the prost/pollster "adopt a transitive as a direct edge"
  precedent, `cli/Cargo.toml:136-147`) but binds to a gix-internal API and a
  lower-level printer you write yourself. Standalone `imara-diff` would be a new
  package like `similar`.

❓ **Open decision for planning**: `similar` (highest-level, one-call, new
package) vs gix's `imara-diff` (zero new package, build-your-own printer). The
work item defaults to `similar`; the zero-new-package angle is worth weighing
against the pin-review cost.

### 0188/0198 migration template (`vcs-adapters`)

The direct code precedent for the error model and crate shape:

- **Module split by mechanism**: `library` (in-process, denies `std::process`)
  vs `subprocess` (the surviving spawner) — `cli/vcs-adapters/src/lib.rs:13-15`.
- **Narrow adapter error**: hand-written `pub enum Error` with one variant per
  failure site, `Display`/`source` impls, and the bridge
  `impl From<Error> for kernel::Error` (`library.rs:70-187,429-433`). No
  `thiserror`.
- **Fallible vs `Option` by caller need**: probes a caller gates on return
  `Result<_, Error>` (`origin_url`, `library.rs:367`); ambient reads where
  failure and absence are both "no answer" return `Option`, error folded to
  `None` + `warn!` (`git_revision`, `library.rs:570`).
- **Domain/adapter split**: the `vcs` domain crate cannot import the adapter
  `Error`, so shared fallible ports use `kernel::Error` (`cli/vcs/Cargo.toml:12-18`).

For 0201, the simplest resolution of the `DiffUnavailable` question: an
in-process diff **cannot fail** (no spawn, no I/O, no timeout), so `render`
becomes **infallible** (`-> String`). That is cleaner than the `vcs-adapters`
`Result` model and matches the fact that both consumers already have a "just
render text" happy path. The sync path's `Unrenderable` fallback stays for its
`local_unreadable` trigger.

### Adding a workspace dependency (mechanics)

Two-tier: declare once in `cli/Cargo.toml` `[workspace.dependencies]` (with the
pinning-discipline comments as precedent, `:117-147`), reference from
`cli/work-adapters/Cargo.toml` with `{ workspace = true }`. No new crate/member
and no thirteen-point sub-binary checklist applies — this is an added dependency
on an existing crate, not a new binary.

## Code References

- `cli/work-adapters/src/diff_shellout.rs:33-102` — `render`/`render_with`/`run_capped`, the code to replace
- `cli/work-adapters/src/diff_shellout.rs:58-63` — the frozen header/blank-line framing
- `cli/work-adapters/src/diff_shellout.rs:24` — `DiffUnavailable` unit struct
- `cli/work-cli/src/diff.rs:11-47` — `work diff` command consumer (`RunOutcome::DiffUnavailable`)
- `cli/work-adapters/src/sync/run.rs:258-277` — `render_dossier`, second consumer; `Unrenderable` dual-trigger
- `cli/work-cli/src/sync.rs:285,318,480` — sync-path `render` injection sites
- `cli/work/src/section_diff.rs:176-205` — `differing_sections` (produces `SectionDiff`, does not call `render`)
- `cli/pup.ron:283-304` — the `work_adapters_filesystem_reads_in_process` rule (guards `filesystem`, not the spawner)
- `cli/work-adapters/tests/diff_shellout_parity.rs` — the `bash-parity`-gated golden suite to retire
- `cli/work-adapters/tests/bash_parity_baseline.rs` + `tests/fixtures/bash-parity-baseline.txt:36,50-54,78-82` — the counts/hashes manifest to update
- `cli/deny.toml:52-64` — licence allow-list (MIT/Apache-2.0 already allowed)
- `cli/Cargo.lock:1781` — `gix-imara-diff` already transitively present
- `cli/Cargo.toml:117-147` — workspace dependency pinning-discipline precedent
- `cli/vcs-adapters/src/library.rs:70-187,429-433` — narrow error + `From<Error> for kernel::Error` template
- `cli/corpus-adapters/tests/zero_spawn.rs` — crate-wide zero-spawn harness precedent for AC7

## Architecture Insights

- **Ports-and-adapters with mechanism-split modules** (ADR-0053): the domain
  crate (`work`) holds `SectionDiff`/`differing_sections`; the adapter crate
  (`work-adapters`) holds the rendering. Keeping the pure and impure modules
  apart is what lets a per-module import rule exist at all.
- **`cargo-pup` import rules are surface guards, not behaviour guards.** They
  catch `use`-path imports, not inline-qualified calls; zero-spawn is proven by
  a harness test, not the rule. This is why AC7 should prefer a `zero_spawn`
  test over a deleted rule.
- **The `bash-parity` feature is a byte-identity gate for the shell-port era.**
  Its whole reason to exist (prove the Rust port matches the bash original) is
  spent once the subprocess is gone — consistent with the 0174 shell-tooling
  retirement direction.

## Historical Context

- `meta/decisions/ADR-0053-thin-cli-over-a-hexagonal-ports-and-adapters-core.md`
  — the ports/adapters boundary and the subprocess-vs-library adapter split;
  the only ADR touching cargo-deny and this boundary. No dedicated ADR exists
  for the `pup.ron` rules or the deny allow-list (documented in
  `tasks/README.md` per CLAUDE.md).
- `meta/work/0170-work-item-subdomain-and-sync-engine.md` +
  `meta/plans/2026-08-06-0170-...md` — where `diff_shellout` and `work diff`
  were built; the source of the byte-parity requirement now being retired.
- `meta/work/0188-...` / `meta/work/0198-...` — the VCS subprocess→library
  migration precedent (0198 still draft: work item only, no plan).
- `meta/reviews/work/0201-in-process-section-diff-review-1.md` — two-pass review,
  verdict COMMENT, all findings resolved. The material fix: a golden-example AC
  pinning exact expected body text (was self-referential). Residual notes: AC4's
  fallible branch and AC8's "no diff binary" prove-a-negative have no stated
  procedure.

## Related Research

- `meta/research/codebase/2026-08-18-0213-conversational-conflict-resolution-flow.md`
  — describes the (unimplemented) skill flow that would become the first real
  consumer of `work diff` body output.
- `meta/research/codebase/2026-08-02-0188-library-backed-vcs-adapter.md` — the
  library-backed adapter research whose code shape this migration mirrors.

## Open Questions

- ❓ **Diff crate**: `similar` (new package, one-call `unified_diff()`) vs gix's
  already-present `imara-diff` (zero new package, hand-written printer)? Work
  item defaults to `similar`; the plan should weigh pin-review cost against it.
- ❓ **`render` signature**: infallible `-> String` is recommended (an
  in-process diff cannot fail). Confirm no reachable I/O remains that would
  justify keeping a narrow `Result` — if the implementation reads nothing and
  allocates only, infallible is correct and AC4's fallible branch is moot.
- ❓ **AC7 shape**: replace the `pup.ron` rule with a crate-wide `zero_spawn`
  harness test (recommended) rather than deleting it outright, since deletion
  drops `filesystem.rs`'s purity guard. Decide whether to also keep a narrowed
  import rule alongside the harness test.
- ❓ **Module rename** (work item Open Question 1): `diff_shellout` → `diff_render`
  once it no longer shells out. Cosmetic; touches the `lib.rs:5` `pub mod` line
  and every `use` path listed under Code References.
