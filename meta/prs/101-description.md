---
type: "pr-description"
id: "101"
title: "[0258] Help should show subcommands"
date: "2026-09-06T15:57:01+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "work-item:0258"
parent: "work-item:0258"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/101"
pr_number: 101
tags: ["cli", "help", "launcher", "discoverability"]
revision: "5efe56a105189b3e4a1fe1268c7c98827c3be34b"
repository: "accelerator"
last_updated: "2026-09-06T15:57:01+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0258] Help should show subcommands

## Summary

`accelerator --help`, bare `accelerator`, and `accelerator help` now converge
on one merged, user-facing command listing that shows built-ins and dispatched
sub-binaries in a single "Commands:" section. The listing is driven by the
signed release manifest, so a newly registered sub-binary appears with no
help-code change. Previously the three entry points diverged — only `--help`
augmented, under an "External subcommands:" heading, while sub-binary
descriptions carried launcher-internal phrasing like "The … sub-binary."

## Changes

Two independently mergeable phases.

**Phase 1 — user-facing descriptions.**
- Rewrote the nine sub-binary crate `package.description` fields to concise,
  period-free imperative phrases (e.g. `work` → "Manage work items";
  `visualiser` → "Serve the meta-directory visualiser").
- Trimmed the trailing periods from the three built-in doc comments
  (`version`, `config`, `cache`) so the merged listing is punctuation-uniform.
- Added a phrasing guard in `test_manifest.py`: no sub-binary description may
  contain `sub-binary` or end in a period.
- Extended the `tasks/README.md` registration checklist to note the
  description is user-facing CLI help text, not only package metadata.

**Phase 2 — merge the list and converge the entry points.**
- Replaced the `after_help` external block with clap-native subcommand
  injection (`augment_with_subbinaries`): each manifest binary becomes a
  render-only clap subcommand, so built-ins and sub-binaries render in one
  native "Commands:" section. A skip guard avoids colliding with a built-in or
  clap's synthesised `help`.
- Folded the three entry points through one renderer (`render_full_listing`)
  and a pure `classify` function over `(ErrorKind, &[OsString])`, so routing
  reads no process global and the whole truth table is unit-testable. `--help`
  and `help` exit 0; bare exits 1 with a required-subcommand cue on stderr
  while the listing goes to stdout.
- Added `Fetcher::for_help()` (3s connect, 5s total, one attempt), so a slow or
  unreachable release host fails open to the built-ins within ~3s rather than
  inheriting dispatch's retry-heavy budget. `new()`/`with_backoff()` keep
  today's values, so dispatch is unchanged.
- Added clap's `string` feature (subcommands are built from owned `String`
  names) and removed a now-obsolete `main.rs` scraper test in
  `test_dispatch_coherence.py`.

## Context

- Work item: `meta/work/0258-help-show-subcommands.md`
- Plan: `meta/plans/2026-09-05-0258-help-show-subcommands.md`
- Research: `meta/research/codebase/2026-09-05-0258-help-show-subcommands.md`
- Validation: `meta/validations/2026-09-05-0258-help-show-subcommands-validation.md`
  (result: pass)

Deliberately out of scope for this PR (0259's remit): colour, section
ordering, the sentence-vs-fragment register split between built-in and
sub-binary descriptions, and the AC-5 fixed wording. The ADR-surface move is
0260.

## Testing

- [x] Rust workspace check: `mise run cli:check` (exit 0)
- [x] Launcher unit + black-box tests: `cargo test -p accelerator` (0 failed)
- [x] Read-only CI mirror: `mise run check` (exit 0, all four components)
- [x] Phrasing guard + visualiser assertion: `pytest test_manifest.py -k "visualiser or phrasing"`
- [x] Three entry points print an identical listing (dead host): `diff <(--help) <(help)` and `diff <(--help) <(bare)` both empty
- [x] Bare exits 1 with the cue on stderr and the listing on stdout; `--help`/`help` exit 0
- [x] `config --help` still renders config's own help, not the top-level listing
- [x] `for_help()` fails open within its connect bound (unit-tested against a SYN-dropping host)

## Notes for Reviewers

- **`classify` routing** is the behavioural core. The bare-invocation arm keys
  on `DisplayHelpOnMissingArgumentOrSubcommand` with empty args (this clap
  version's actual kind for bare `accelerator`), keeping `MissingSubcommand` as
  a defensive fallback; the `args.is_empty()` guard keeps a missing *nested*
  subcommand (`config templates`) on its own per-command help.
- **Convergence is structural**: `render_full_listing` is the sole augmenting
  renderer and both listing causes resolve to it, so a signed-manifest
  end-to-end test is not required — the populated listing is unit-tested over a
  hand-built `Manifest`, and black-box covers the manifest-unavailable path.
- **Minor nit** (non-blocking): the `classify` doc comment references a root
  `arg_required_else_help` that the `Cli` struct does not carry — the behaviour
  is clap's default for a required subcommand field.

https://claude.ai/code/session_01JG4uFKzWEfo8LSdmGzNN1L
