---
type: "plan-validation"
id: "2026-08-11-0204-remote-tracker-port-validation"
title: "Validation Report: RemoteTracker Port Implementation Plan"
date: "2026-08-12T11:11:35+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-08-11-0204-remote-tracker-port"
tags: ["rust", "tracker", "sync", "port", "cargo-pup"]
last_updated: "2026-08-12T11:11:35+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: RemoteTracker Port Implementation Plan

### Implementation Status

✓ Phase 1: The Crate, Its Vocabulary, and Its Enforcement Rule - Fully implemented
✓ Phase 2: The Port - Fully implemented
✓ Phase 3: Pup Probe Backfill - Fully implemented

`jj log` shows the plan's phases landed as seven discrete, well-scoped
commits (`ad420e74` vocabulary, `fa5f181b` port trait, `f5b80683` pup
backfill), preceded by a frozen-surface reopening (`a222634e`) and followed
by the dangling-reference fix and the 0204/0194/0171 handoff commit
(`6a72a2367a`, `21fcadadc2`). The working copy is clean and empty on top of
the final commit — nothing outstanding, nothing uncommitted.

### Automated Verification Results

✓ `mise run cli:check` passes (format, clippy, `--locked` lockfile sync)
✓ `mise run pup:check` passes — `tracker` satisfies its own domain rule
✓ `mise run public-api:check` passes against the committed
  `cli/tracker/tests/fixtures/public-api.txt`
✓ `cd cli && cargo nextest run -p tracker` — 27/27 tests pass across
  `vocabulary.rs`, `errors.rs`, `port.rs`, `structure.rs`
✓ `mise run test:integration:pup` — 39/39 pass (16 pre-existing + 23
  Phase-3 backfill cases for `corpus`, `vcs`, `work`, `migrate`)
✓ `mise run test:unit:tasks` — 746/746 pass
✓ `mise run deny:check` — advisories ok, bans ok, licenses ok, sources ok
✓ `mise run build-system:check` — ruff, pyrefly, actionlint all clean
✓ `mise run check` (the full read-only CI mirror, including frontend and
  server lanes) — exits 0 end to end
✓ `cargo nextest run -p accelerator-corpus -E
  'test(this_repositorys_own_corpus_is_clean)'` — passes, confirming the
  post-implementation dangling-`relates_to` fix recorded in Phase 2's notes

No automated check failed. This is a stronger result than the plan's own
Phase 2 notes recorded (one unrelated corpus-goldens failure, since fixed) —
the corpus goldens test now passes cleanly, and the bare `mise run` claim in
those notes is corroborated by `mise run check` here.

### Code Review Findings

#### Matches Plan:

- `cli/tracker/` contains exactly the files the plan specifies:
  `Cargo.toml`, `src/lib.rs`, and four test binaries plus three fixtures
  under `tests/`. No extra files, none missing.
- `cli/Cargo.toml` lists `tracker` as a workspace member; `cli/pup.ron`
  carries `tracker_domain_imports_only_permitted` with the exact
  `matches`/`allowed_only` anchors the plan specifies, omitting the
  `kernel::Error` allowance as deviation 3 requires.
- `src/lib.rs` declares exactly six public items (`ExternalId`,
  `RemoteTimestamp`, `RemoteIssue`, `TrackerError`, `FetchOutcome`,
  `RemoteTracker`), matching AC 10. Both `new` constructors are `const fn`
  (deviation 7, confirmed empirically per the plan's own notes). No
  `#[non_exhaustive]` on `TrackerError`, per the plan's explicit exclusion.
- The public-API snapshot lane (`tasks/public_api.py`, `deps:install:public-api`,
  `PUBLIC_API_VERSION`, the `check-architecture` CI step) is wired exactly as
  deviation 8 and §10 describe, including the `--include
  function-parameter-names` flag the implementation notes record as a
  necessary addition over the plan's original invocation.
- All three documentation pointer edits from §9 are present and match the
  plan's prescribed wording: root `CLAUDE.md`'s "thirteen-point checklist …
  plus a shorter one for registering a plain library crate", the
  `tasks/README.md` index line, and `tasks/CLAUDE.md`'s additive anchor.
- The Phase 3 backfill covers all four domain rules (`corpus`, `vcs`, `work`,
  `migrate`) with the violation/control/kernel-error/extra-allowance/
  cross-rejection family the plan specifies — 23 new cases, 39 total,
  confirmed by test count.
- The 0204/0194/0171 handoff edits are all present: 0204's frontmatter shows
  `status: done` and `blocks: ["work-item:0171", "work-item:0194"]`; 0171's
  `blocked_by` no longer names `work-item:0204`; 0194 carries no
  `blocked_by` field at all (cleared per the plan's notes); the review
  document's `relates_to` now points at the correctly-named
  `work-item-review:0194-tracker-crate-and-remote-sync-engine-review-2`
  rather than the dangling bare `:0194` reference.

#### Deviations from Plan:

- None beyond the ones the plan itself already documents and reconciles
  (deviations 1–8, all recorded in the plan body with their rationale). No
  undocumented drift was found between the frozen block, the plan, and the
  shipped code.

#### Potential Issues:

- Five doc-comment lines in `cli/tracker/src/lib.rs` (116, 147, 226, 246,
  270) run to 81 characters, one over the repo's stated 80-column
  convention. `rustfmt` does not reflow doc comments (`wrap_comments` is
  off), so `mise run cli:check` does not and cannot catch this — it is a
  cosmetic nit with no automated guard anywhere in the repo, not a defect
  introduced by this plan's own tooling.

### Manual Testing Required

The plan's Manual Verification sections are one-shot mutation checks by
design — each establishes that a guard discriminates, then is reverted and
never re-run. The plan's own implementation notes (dated 2026-08-12) record
every one of these as performed, including the one genuine gap the process
surfaced and chose to record rather than paper over:

1. [x] AC 2's default-body clause is **confirmed unguarded** — a fifth
   required method given a default body is caught by neither
   `public-api:check` nor `port.rs`. This is handed forward to 0171/0194 as
   a known gap, exactly as the plan instructs, and is not a defect in this
   implementation.
2. [x] All other one-shot mutations (rule deletion, anchor corruption,
   variant rename, parameter swap, derive addition, dependency-table
   addition) are recorded as performed with the expected red observed.

No further manual testing is required to close this plan — the one
outstanding item (AC 2's gap) is explicitly a handoff, not an omission.

### Recommendations

- None blocking. The plan is fully implemented, its own deviations are
  reconciled and recorded, its handoffs to 0171 and 0194 are complete, and
  every automated gate — including the full `mise run check` mirror — is
  green.
- Optional, non-blocking: wrap the five 81-character doc-comment lines in
  `cli/tracker/src/lib.rs` to 80 columns for consistency with the stated
  house convention, next time that file is touched.
