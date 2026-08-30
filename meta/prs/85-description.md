---
type: "pr-description"
id: "85"
title: "[0201] In-process section diff"
date: "2026-08-30T21:46:27+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0201"
parent: "work-item:0201"
relates_to: ["work-item:0170", "work-item:0174", "work-item:0188", "work-item:0198"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/85"
pr_number: 85
tags: ["rust", "work-adapters", "diff", "tech-debt"]
revision: "a5d276abeb2fea458174694a46b3cda944008ed8"
repository: "accelerator"
last_updated: "2026-08-30T21:46:27+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0201] In-process section diff

## Summary

Replaces `work-adapters`' subprocess `diff -u` section renderer with an in-process `similar` implementation, makes the renderer infallible, retires the spent bash-oracle parity suites, and adds a crate-wide zero-spawn guard. `work-adapters` now spawns no subprocess and needs no `diff` binary on `PATH`, while the frozen `=== name (- LOCAL / + REMOTE) ===` framing callers depend on is unchanged.

## Changes

- **In-process renderer**: `work_adapters::diff::render(&SectionDiff) -> String` computes a unified-diff body via `similar` (exact-pinned `=2.7.0`), with no `std::process`, no `Command`, and no temp files. The old `diff_shellout.rs` (temp-file writes, `diff -u` under a 10s poll-loop cap) is gone.
- **Infallible signature**: `DiffUnavailable` and `RunOutcome::DiffUnavailable` are removed; both consumers (`work diff`, `sync`) call an infallible `render`. The sync dossier's only remaining unrenderable trigger is an unreadable local file, and its message is reworded accordingly.
- **Parity teardown**: the `bash-parity`-gated `diff_shellout_parity.rs` / `cli_diff_parity.rs` suites and the `work-item-section-diff` fixture corpus are retired; the baseline manifest and its default-lane guard are realigned (10 → 8 recorded tests); `work-cli`'s orphaned `bash-parity` feature is removed.
- **Zero-spawn invariant**: a new crate-wide `work_adapters_is_zero_spawn` `cargo-pup` rule sits alongside the retained `work_adapters_filesystem_reads_in_process` allow-list; a `test_import_rule.py` probe pair registers it; a runtime `zero_spawn.rs` harness drives the render paths under a spawn-tripwire `PATH` to close the rule's inline-`Command::new()` blind spot.
- **New tests**: four in-process unit tests, an exact-match CLI golden for the frozen framing plus a `last_updated`-only case covering the detect→render pipeline.
- **Planning artefacts**: the work item, plan, research, plan/work reviews, and the validation report, with the work item and plan marked done.

## Context

Implements work item `0201` (`meta/work/0201-in-process-section-diff.md`), following the plan at `meta/plans/2026-08-30-0201-in-process-section-diff.md` and its validation at `meta/validations/2026-08-30-0201-in-process-section-diff-validation.md`. The work item itself was added separately in merged PR #56; this PR is the implementation. Relates to the shell-tooling-retirement effort (0174), the crate where this code originated (0170), and the library-backed VCS adapter work (0188, 0198).

## Testing

- [x] CLI unit lane (`--all-features`, runs the zero-spawn harness): `mise run test:unit:cli` — 2662 passed
- [x] pup probe pair against the real config: `mise run test:integration:pup` — both `work_adapters_is_zero_spawn` probes green
- [x] cargo-pup enforcement: `mise run pup:check` — no violations
- [x] Format + clippy: `mise run cli:check` — clean
- [x] Licence check: `cargo deny check licenses` — `similar` Apache-2.0 ok
- [x] Empty-`PATH` render: `env PATH= accelerator work diff a.md b.md` renders identically, exit 0 — no external binary reached
- [x] Frontmatter conformance: `accelerator corpus frontmatter validate` — exit 0

## Notes for Reviewers

- The diff **body** format changes (hunks without `---/+++` file lines); no live consumer parses it. The one future consumer (the unimplemented 0213 conflict flow) should pick up the new `work_adapters::diff::render(&SectionDiff) -> String` path.
- The exact `=2.7.0` pin makes the CLI golden's `assert_eq!` against `similar`'s hunk bytes safe — a formatting change in a new release must arrive as a deliberate re-bless.
- The runtime zero-spawn harness guards only the two render-composing paths; an inline spawn added later to a non-rendering module (`author.rs`, `filesystem.rs`) relies on the pup rule and review. This is a documented boundary. Probe-pairing the sibling `work_adapters_filesystem_reads_in_process` rule is a noted follow-up.
