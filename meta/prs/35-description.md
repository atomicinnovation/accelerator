---
type: "pr-description"
id: "35"
title: "Bump the jj pin to 0.43.0 to match the jj-lib crate"
date: "2026-08-02T18:55:44+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
relates_to: ["work-item:0136", "work-item:0188"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/35"
pr_number: 35
tags: ["toolchain", "mise", "jj", "vcs", "dependencies"]
revision: "e6ebc18aab5bc277914b7b096e21d3e030042f94"
repository: "accelerator"
last_updated: "2026-08-02T18:55:44+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Bump the jj pin to 0.43.0 to match the jj-lib crate

## Summary

0188 will add a library-backed VCS adapter built on `jj-lib` 0.43. Its test fixtures are **written by the installed `jj` CLI and read by `jj-lib`**, so the two become a lockstep pair — and `mise.toml` pinned the CLI at 0.36.0, a seven-version gap. This bumps the CLI pin to 0.43.0 ahead of that work, with an inline comment tying the two pins together.

## Changes

- `mise.toml`: `jj = "0.36.0"` → `jj = "0.43.0"`.
- A four-line comment above the pin recording *why* it is not free to float independently — that the library-backed adapter reads repositories the CLI writes, that a skew between them is a format-coherence risk, and that the two must be bumped together. It points at `meta/work/0188-library-backed-vcs-adapter.md`.

## Context

- Prepares `meta/work/0188-library-backed-vcs-adapter.md`, under epic 0136.
- The coupling is really a **three**-pin one: `jj-lib` 0.43 requires Rust 1.89 and `gix` 0.85 requires 1.85, against the repo's pinned 1.90.0. `jj-lib`'s MSRV has moved 1.85 → 1.88 → 1.89 across eight releases, so a future `jj-lib` bump will likely drag both `gix` and the Rust toolchain with it.
- Landing this separately from 0188 is deliberate. A skew between the CLI and the library fails in a way that reads as an adapter defect rather than a pin mismatch, so it is worth discovering on a one-line PR rather than inside the story that adds two dependency trees, a licence exception and a pre-1.0 API bet.

## Testing

- [ ] **The jj-fixture shell suites have not been re-run on 0.43.** `mise ls jj` reports `0.43.0 (missing)` — the pin is committed but not provisioned on this machine, and the active binary is still `~/.local/share/mise/installs/jj/0.36.0/jj`. Verifying locally means `mise install`, which also makes 0.43 the active `jj` for this repo. CI provisions the pinned version from scratch, so **CI on this PR is the real verification** for the bump.
- [ ] CI on this PR.

The suites that build or read jj fixtures, and so carry the risk:

- `hooks/test-vcs-detect.sh` (and its `hooks/test-fixtures/vcs-detect/regenerate.sh`)
- `scripts/test-metadata-helpers.sh`
- `skills/work/scripts/test-work-item-scripts.sh`
- `skills/config/migrate/scripts/test-migrate.sh` and `test-migrate-interactive.sh`
- the Python task tests that shell out to jj, under `tests/unit/tasks/`

## Notes for Reviewers

**The blast radius is wider than 0188 recorded.** That work item names `hooks/test-vcs-detect.sh` and the `skills/work/scripts/` suite. Grepping for jj invocations across the shell and Python suites turns up four more sites (listed above), including the migration-framework suites and several `tests/unit/tasks/` modules. Nothing here changes those files — but if CI goes red on this PR, that list is where to look, and 0188's Dependencies section should be corrected to match.

**This bumps the toolchain ahead of the consumer.** Nothing in the tree uses `jj-lib` yet, so for now the only effect is that every contributor and every CI lane runs `jj` 0.43 instead of 0.36. That is the point: it surfaces any fixture-format fallout on a one-line, trivially revertable change instead of inside 0188.

**Contributors will need `mise install`** after this merges. Their local `jj` does not move on its own.

**Stack position.** Second of three: #34 (the 0169 split) → **#35** → #36 (close out 0167/0168/0182). This PR's content is independent of #34 — it sits in the stack only because it shares an ancestry, and it can be reviewed on its own.
