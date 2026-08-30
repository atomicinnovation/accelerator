---
type: "plan-validation"
id: "2026-08-30-0201-in-process-section-diff-validation"
title: "Validation Report: In-Process Section Diff Implementation Plan"
date: "2026-08-30T20:17:14+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
parent: "plan:2026-08-30-0201-in-process-section-diff"
target: "plan:2026-08-30-0201-in-process-section-diff"
tags: ["rust", "work-adapters", "diff", "tech-debt"]
last_updated: "2026-08-30T20:17:14+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: In-Process Section Diff Implementation Plan

All three phases are fully implemented and committed; every automated check
run passes; all three manual verifications confirm the intended behaviour.
`work-adapters` now diffs sections in-process via `similar`, spawns no
subprocess, and needs no `diff` binary on `PATH`, with the frozen framing
byte-identical to before.

### Implementation Status

- ✅ Phase 1: Retire the bash-oracle diff parity suites — fully implemented (commit `ntnuxzqm`)
- ✅ Phase 2: In-process `similar` renderer, infallible — fully implemented (commit `npozospw`)
- ✅ Phase 3: Crate-wide zero-spawn pup rule — fully implemented (commit `kykpvuvm`)

### Automated Verification Results

- ✅ CLI unit lane (`--all-features`): `mise run test:unit:cli` — 2662 passed, 1 skipped, 0 failed
- ✅ pup integration probes: `mise run test:integration:pup` — 63 passed, both `work_adapters_is_zero_spawn` probes green
- ✅ cargo-pup enforcement: `mise run pup:check` — no violations
- ✅ Format + clippy: `mise run cli:check` — clean, no warnings
- ✅ Licence check: `cd cli && cargo deny check licenses` — `licenses ok` (`similar` Apache-2.0)
- ✅ Source spawn grep: `grep -rn "std::process|Command::new|Stdio|DiffUnavailable" work-adapters/src` — nothing (the only `std::process` hits are `ExitCode` in `work-cli`, the CLI return type)
- ✅ Corpus reference grep: `grep -rn "work-item-section-diff" work-adapters/tests work-cli/tests` — nothing

Not run: the full aggregate `mise run check` and the full default `mise run`.
The change surface is confined to the `cli/` Rust workspace plus one Python
pup test; the lanes exercising exactly that surface were run directly and are
green. The plan records both aggregates as passing end-to-end at
implementation time (with one confirmed load-induced `api_smoke` flake in
`test:integration:visualiser`, unrelated to this change).

### Code Review Findings

#### Matches Plan:

- `work_adapters::diff::render(&SectionDiff) -> String` is infallible, feeds `diff.local`/`diff.remote` to `TextDiff::from_lines` verbatim with `context_radius(3)`, and emits the `=== name (- LOCAL / + REMOTE) ===` header plus a single trailing blank line (`cli/work-adapters/src/diff.rs`).
- `DiffUnavailable`, `DEFAULT_CAP`, `POLL_INTERVAL`, `run_capped`, `render_with`, and all subprocess/temp-file imports are gone; the module is renamed `diff_shellout.rs` → `diff.rs` and re-registered in `lib.rs`.
- All four consumers ripple to the infallible signature: `work-cli/src/diff.rs` (no `DiffUnavailable` outcome), `work-cli/src/main.rs` (arm removed), `work-adapters/src/sync/run.rs:237` and `work-cli/src/sync.rs:284,317` (`&dyn Fn(&SectionDiff) -> String`), production injection at `sync.rs:479` (`&work_adapters::diff::render`).
- Parity teardown complete: both `_parity` suites and the `work-item-section-diff` fixture corpus deleted; `cli_diff.rs` (non-parity) retained; baseline manifest realigned and the guard count dropped 10 → 8 with the corpus removed from `RELOCATED_CORPORA` and `corpus_home()` (`bash_parity_baseline.rs:194`).
- Phase 3 three-guard structure present: crate-wide `work_adapters_is_zero_spawn` deny alongside the retained `work_adapters_filesystem_reads_in_process` allow-list (`cli/pup.ron:289,308`); the `test_import_rule.py` probe pair; the runtime `zero_spawn.rs` harness (`the_render_paths_reach_no_external_binary`, green).
- New tests present and passing: four in-process unit tests, the AC3 exact-match `one_differing_section_renders_a_unified_body_with_the_frozen_framing`, and the `a_last_updated_only_frontmatter_change_renders_into_the_body` case covering the detect→render pipeline.

#### Deviations from Plan:

- The `diff.rs` module docstring is a single line (`//! In-process section diffing for the work domain crate.`) rather than the plan's two-line phrasing. Cosmetic; the substance (header contract, `similar` body) moved to the `render` docstring. No functional impact.

#### Potential Issues:

- None found. The documented boundary stands: the runtime harness guards only the two render-composing paths; an inline spawn added later to `author.rs` or `filesystem.rs` would escape both the use-path pup rule and the harness, relying on the pup rule and review. The plan calls this out as a deliberate, acceptable boundary with `author` tripwire coverage as a reasonable follow-up.

### Manual Testing Required:

All performed and passing against a fresh `target/debug/accelerator-work` build:

1. Rendering:
  - [x] `work diff a.md b.md` on two items differing in one section prints the unchanged framing with a `@@ -1 +1 @@` hunk and `-`/`+` body
  - [x] `env PATH= work diff a.md b.md` (AC8) renders identically, exit 0 — no external binary reached
  - [x] `work diff a.md a.md` prints `(no differing sections after normalisation)` (AC5)

### Recommendations:

- Add a probe pair for the sibling `work_adapters_filesystem_reads_in_process` rule (currently unguarded) so the crate's pup surface is fully covered — noted in-plan as out-of-scope follow-up.
- If the 0213 conflict-resolution flow lands and parses the diff body, pin the `similar` output format deliberately rather than inheriting it (the `=2.7.0` pin protects the goldens but not a downstream parser).
