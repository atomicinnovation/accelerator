---
type: "plan-validation"
id: "2026-09-05-0258-help-show-subcommands-validation"
title: "Validation Report: Help Should Show Subcommands Implementation Plan"
date: "2026-09-06T11:27:00+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
parent: "plan:2026-09-05-0258-help-show-subcommands"
target: "plan:2026-09-05-0258-help-show-subcommands"
tags: ["cli", "help", "launcher", "discoverability"]
last_updated: "2026-09-06T11:27:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Help Should Show Subcommands Implementation Plan

Both phases are fully implemented and every success criterion is verified.
The three entry points converge on one identical command listing; all four
components pass read-only CI. Three files outside the plan's declared set were
touched — each a necessary ripple of the planned change, not scope creep.

### Implementation Status

- ✅ Phase 1: User-Facing Descriptions — fully implemented
- ✅ Phase 2: Merge the List and Converge the Entry Points — fully implemented

Both phases are committed: Phase 1 as `Rewrite sub-binary descriptions as
user-facing help text (0258 phase 1)`, Phase 2 as the merge commit `Merge help
into one command listing across all three entry points (0258 phase 2)`.

### Automated Verification Results

- ✅ Rust workspace check: `mise run cli:check` (exit 0)
- ✅ Launcher unit + black-box tests: `cargo test -p accelerator` (all suites green, 0 failed across 12 binaries)
- ✅ Visualiser + phrasing guard: `pytest test_manifest.py -k "visualiser or phrasing"` (2 passed)
- ✅ Dispatch-coherence suite after the scraper deletion: `pytest test_dispatch_coherence.py` (74 passed)
- ✅ Read-only CI mirror: `mise run check` (exit 0, all four components)

The full local CI mirror (`mise run`) was not re-run here; the plan records it
passing with a single known unrelated flake in the visualiser-readiness lane.
`mise run check` — the read-only lane CI actually gates on — is green.

### Code Review Findings

#### Matches Plan

- **All nine descriptions** rewritten to the AC-5 text at their `Cargo.toml`
  source of truth; `migrate`'s trailing period trimmed; `visualiser` reads
  `Serve the meta-directory visualiser`.
- **Three built-in doc comments** (`version`, `config`, `cache`) stripped of
  trailing periods in `cli.rs`, so the merged listing is punctuation-uniform.
- **Phrasing guard** added in `test_manifest.py`: iterates
  `DISPATCHED_SUBBINARIES` through `_read_description`, asserting no description
  contains `sub-binary` or ends with `.`.
- **README checklist** point 3 extended to note the description is user-facing
  help text; the list stays at thirteen points, preserving the `CLAUDE.md`
  references.
- **`augment_with_subbinaries`** matches the plan verbatim, including the
  load-bearing `is_empty() || == "help" || find_subcommand` skip guard and
  `sanitize()`.
- **`classify` / `HelpRoute` / `ListingCause`** implemented as a pure function
  over `(ErrorKind, &[OsString])`; `main` collects `root_args` once and threads
  it in, so routing reads no process global.
- **`Fetcher::for_help()`** parameterises `build()` via a `Limits` struct
  (3s connect, 5s total, 1 attempt); `new()`/`with_backoff()` retain today's
  values, so dispatch is unchanged.
- **Black-box coverage** extended to all three entry points plus the
  `config --help` isolation case, with the bare case asserting stream
  separation (listing on stdout, cue on stderr).

#### Deviations from Plan

- **`classify` bare-invocation arm folds in `DisplayHelpOnMissingArgumentOrSubcommand`.**
  The plan's pseudocode keyed the bare case on `ErrorKind::MissingSubcommand`.
  This clap version surfaces bare `accelerator` as
  `DisplayHelpOnMissingArgumentOrSubcommand` with empty args, so the arm matches
  both kinds under `args.is_empty()` and keeps `MissingSubcommand` as a
  defensive fallback. The plan itself flagged this in its Phase 2 manual-check
  note (line 684); the implementation is the correct adaptation.
- **`cli/Cargo.toml` adds clap's `"string"` feature.** Required, not optional:
  `augment_with_subbinaries` builds subcommands from owned `String` names
  (`sanitize()` output), which `clap::Command::new` accepts only under `string`.
  Not mentioned in the plan.
- **`test_dispatch_coherence.py` deletes `test_is_root_help_agrees_as_a_secondary_check`.**
  That test scraped the old `is_root_help` word-list `Some("version" | …)` from
  `main.rs`, which §3 removed in favour of the pure `is_root_help_args`. The
  scraped pattern no longer exists, so the test had to go. A necessary ripple
  the plan did not enumerate.
- **`crypto_provider.rs` doc comment** updated `help_section()` →
  `load_help_manifest()` — a rename ripple from the §2 fold.

#### Potential Issues

- **`classify` doc comment overstates the derive.** Line 157 says "the derive
  marks the required root subcommand `arg_required_else_help`", but the root
  `Cli` struct carries no such attribute — the behaviour comes from clap's
  default handling of a required subcommand field. Cosmetic; the actual kind is
  pinned by the black-box tests, so behaviour is unaffected.
- **No live-manifest end-to-end test**, by design. The populated listing is a
  unit assertion over a hand-built `Manifest` (`build_listing` /
  `augment_with_subbinaries`); black-box covers only the manifest-unavailable
  path, since a test cannot forge a signed manifest. The convergence guarantee
  rests on `render_full_listing` being the sole augmenting renderer — a
  structural argument, verified here manually against a dead host.

### Manual Testing Performed

All manual criteria were exercised against the built `debug/accelerator` with
the release host forced unreachable (`ACCELERATOR_RELEASE_BASE_URL=https://127.0.0.1:1`):

1. Merged listing:
  - ✅ `--help` renders one "Commands:" section, no "External subcommands"
    heading, period-free descriptions.
  - ✅ `diff <(--help) <(help)` and `diff <(--help) <(bare stdout)` both empty
    (IDENTICAL).
2. Exit codes and streams:
  - ✅ Bare exits 1 with the listing on stdout and
    `error: a subcommand is required; run 'accelerator --help' …` on stderr.
  - ✅ `--help` and `help` exit 0.
3. Isolation:
  - ✅ `config --help` renders `Usage: accelerator config <COMMAND>`, not the
    top-level listing.

Not performed here (covered by the plan's own recorded runs): the
reachable-host three-way diff with sub-binaries present, and the blackhole-host
~3s fail-open timing (unit-tested by `the_help_fetcher_gives_up_within_its_connect_bound`).

### Recommendations

- **Tidy the `classify` doc comment** to describe clap's default required-
  subcommand behaviour rather than a non-existent `arg_required_else_help` on
  the root. Non-blocking.
- **Align the `test_github.py` placeholder strings** at leisure — the plan
  correctly leaves them as documented arbitrary fixtures; no action required
  for correctness.
- **Carry the deferred 0259 items forward**: the sentence-vs-fragment register
  split, the core-then-domain ordering decision, and the AC-5 fixed wording are
  all consciously out of scope here and belong to 0259.
