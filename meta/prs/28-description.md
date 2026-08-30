---
type: "pr-description"
id: "28"
title: "[0182] Bootstrap self-location and --fail-safe"
date: "2026-07-28T22:04:16+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0182"
parent: "work-item:0182"
relates_to: ["work-item:0183", "work-item:0164", "work-item:0167", "work-item:0136"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/28"
pr_number: 28
tags: ["bug", "cli", "launcher", "bootstrap", "plugin-root", "symlinks", "fail-safe", "test-flakes"]
revision: "d246af08e787a6bf59acea8bbef647f4896395b0"
repository: "accelerator"
last_updated: "2026-07-28T22:04:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0182] Bootstrap self-location and --fail-safe

## Summary

`bin/accelerator` aborted unless `CLAUDE_PLUGIN_ROOT` was present in its process environment, but Claude Code only ever *substitutes* that token into skill content — it exports it to hooks and MCP/LSP subprocesses, never to the Bash tool or a `!` preprocessor shell. Every skill invocation therefore arrived with a correct absolute path and an empty environment, and all 45 CLI-invoking skills failed at load. The bootstrap now derives its installation root from its own location, symlink-aware.

This is **Phases 0 and 1a** of the 0182 plan, deliberately stopping short of the `ACCELERATOR_PLUGIN_ROOT` rename. The derived root is exported under **both** names, so the already-shipped `1.24.0-pre.16` launcher reads the one it knows and works unchanged. No Rust changes, and the fix is independently releasable.

## Changes

**Self-location (`bin/accelerator`)**

- Chases `BASH_SOURCE[0]` through up to 16 symlinks, honouring relative targets, then resolves the root with `cd -P`. The repo-wide `cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P` idiom is insufficient here: `pwd -P` resolves symlinks among the *directory* components but not a final symlink to the script itself, so an `accelerator` linked onto `PATH` would derive the link's parent. The loop rather than a single dereference is required because the terminal-invocation chain Phase 3 documents is two hops deep.
- `cd -P`, not bare `cd`, for the `..` step. Logical mode collapses `..` textually, so a directory symlink as the final component yields the link's parent rather than the target's — a prepared tree carrying its own `plugin.json` in the symlink's parent would otherwise become the trust anchor.
- The hop bound is **16**, not the 32/40 of `SYMLOOP_MAX`. Any path bash can `open(2)` has already had its whole chain resolved by the kernel, so a bound at or above either kernel's limit is unreachable — it can neither be tested nor detect a cycle. 16 sits below both, is generous against a two-hop chain, and is testable with a 17-link non-cyclic chain on both platforms.
- `dir_of` replaces `dirname` (pure parameter expansion, and with `cd --` immune to a target beginning with `-`, which `dirname` reads as an option before printing nothing — whereupon `cd ""` succeeds as a no-op and `pwd -P` yields the caller's project). `CDPATH=` stops `cd` printing a matched directory into the command substitution. Every `readlink`/`cd` failure is a named abort.

**`--fail-safe` awareness**

- The flag is now recognised by the bootstrap, resolved once into a named `abort_status` rather than branched on at 14 gates. A bootstrap-layer abort exits 0 with a stderr diagnostic, so a `!` site degrades to empty injected context instead of discarding the whole prompt.
- The scan stops at the first `--` and the first match: a token appearing as an option value, after a separator, or in a future sub-binary's arguments must not silently switch every gate to exit 0.
- The six trust-chain gates route through a distinct `fail_integrity`, which appends a sanitised single-line record to `.accelerator-unverified.log` before degrading. Nothing unverified is ever exec'd either way, so what this buys is *detectability* — without it a bad signature is byte-identical to the ~86 commands that legitimately emit nothing. A named second entry point rather than a positional tag on `fail()`: a misspelling is `command not found` instead of a silently disabled record.

**Test harness (`tests/integration/entrypoint/`)**

- The suite injected `CLAUDE_PLUGIN_ROOT` into every invocation, so the one configuration that matters in production — correct path, empty environment — was never exercised, and two tests actively asserted the faulty behaviour. The injection is gone and those two tests are deleted: with self-location they resolve the real repo root, satisfy every gate, and `curl` the real GitHub release into the working tree's `bin/`.
- `_run_bootstrap` is now the single funnel, and enforces at that funnel that a real fetch is inexpressible: a stubbed downloader, a release host under the reserved `.invalid` TLD, and an entry path that is never the repo's own bootstrap. A session-scoped guard backstops anything bypassing it.
- 16 new cases: rootless render, two-hop and directory symlinks, relative and dash-leading link targets, ambient roots (old name, new name, both), a not-an-installation root, the 16/17-hop boundary, a cycle, the `--fail-safe` scan window, and the two durable-record cases. Verified red against the pre-change bootstrap — 18 failures, every one `CLAUDE_PLUGIN_ROOT is not set`.
- **The suite had been running on Homebrew bash 5.3, not the 3.2 floor it exists to guard.** `_BASH` now pins `/bin/bash`, with an assertion that it is major version 3 on Darwin.

**Three pre-existing test flakes (out of scope, own commit)**

Surfaced but not caused by this work — three consecutive full runs each failed exactly one *different* wall-clock assertion, each passing in the other two runs and in isolation.

- **`test:integration:config` was a real daemon defect.** The Playwright executor's `shutdown()` closed the browser *before* removing `server-info.json` and `server.pid`, so for as long as Chromium took to exit, `run.sh`'s reuse check still passed — live pid, info file present — and the next launcher dispatched onto a dead page. Callers saw `Target page, context or browser has been closed` instead of a clean respawn. Reachable by any client; load only widened the window. State files now go first, and a request landing mid-shutdown gets a typed retryable `daemon-stopping` envelope.
- **`test:unit:frontend`** — Vitest's 5s default is a hang detector, not a latency budget, but it sizes its worker pool to the CPU count while `mise run` has cargo and the Python suites alongside; ~100ms renders blew past it. Timeouts to 30s, where a real hang still fails.
- **`test:integration:visualiser`** — the indexer scan measures **713ms idle** against a 5s ceiling, under a name claiming one second. It is a tripwire for an algorithmic blowup, not an SLO, so it gets 30s (~40x idle) and a name that says so: `scan_2000_files_does_not_blow_up`.

**Determinations (Phase 0)**

- `${CLAUDE_PLUGIN_DATA}` landed in Claude Code **v2.1.78**, 66 patches below the declared v2.1.144 floor, so **the floor does not move** and no prose site or ADR is touched. It is exported to hook processes and MCP/LSP subprocesses only, and resolves to `~/.claude/plugins/data/{id}/` — not version-scoped, which is what makes the terminal-link recipe a one-time action.
- Hook output channels: `systemMessage` is a *universal* top-level field; `SessionStart` stdout becomes Claude's context, not user output; stderr at exit 0 has no documented destination. That last point means `hooks/migrate-discoverability.sh`'s advisory reaches nobody — raised as **0183**.

## Context

- Implements work item **0182** (this PR's parent), Phases 0 and 1a of `meta/plans/2026-07-27-0182-bootstrap-self-location-and-plugin-root-rename.md`. The plan is included in the diff along with its review, the codebase research and the issue research.
- Raises **0183** for the discarded `SessionStart` stderr advisory.
- Related: **0164** (introduced the bootstrap), **0167** (routed the skills through it), **0136** (the parent Rust CLI migration epic).
- **Blocks the next prerelease** — the plugin is substantially unusable as shipped in `1.24.0-pre.16` for any consumer who has not manually exported the variable.

## Testing

- [x] `mise run` (bare default — the full local CI mirror) exits 0 end-to-end, **three consecutive times**, with identical pass counts each run.
- [x] `mise run check` (read-only CI mirror across all five components) exits 0.
- [x] Entrypoint suite 46/46, on `/bin/bash` 3.2 — which is what proves the new bootstrap constructs (`${1//$'\n'/ }`, `$'\n'`, `[[ -L ]]`, `cd -P --`) are on the floor.
- [x] Build-system unit tests 341/341; visualiser 334/334; frontend 122/122; Playwright executor 33/33.
- [x] The new regression tests fail against the pre-change bootstrap — confirmed at the intermediate commit, 18 red for the right reason. The three that pass pre-change are the ones the plan predicts: the cycle case (characterises the kernel's `ELOOP` at `open(2)`, not the in-script counter) and the two scan-window cases (a gate fires either way).
- [ ] **Manual, deferred to the release candidate**: rootless invocation by absolute path against a published artifact. `1.24.0-pre.16`'s assets are not published, so running it now would 404 against the real GitHub release *and* write into the shipped `bin/`. The hermetic equivalent, and the actual CI gate, is the entrypoint suite.

## Notes for Reviewers

- **Start with `bin/accelerator`.** Each element of the chase is load-bearing and the plan records the measurement behind it — the `-P`, the `--`, the `${dir:-/}` arm, the 16-hop bound, `CDPATH=`.
- **The dual export is transitional.** Phase 1b removes `CLAUDE_PLUGIN_ROOT`, and Phase 2's boundary guard turns a leftover into a lint failure rather than a silent carry-over. It carries no explanatory comment deliberately: a comment naming a plan phase would outlive the plan.
- **Two acceptance criteria were restated**, both recorded under *Deviations from the work item* in the plan. `display_path` shortens plugin-root paths to a `<plugin>/` token, so the `templates list` Path column can never carry an absolute path — the assertions use a per-root `templates/adr.md` sentinel or the launcher's own dumped environment instead. The work item's open question about the dev-override precondition is answered rather than restated: the launcher is supplied by serving the real compiled binary through the stub release server, giving the genuine fetch → verify → cache → exec chain with no network and no override.
- **One hazard worth knowing about, not fixed here.** `mise run` builds the frontend, and Vite empties `dist/` before rewriting it. The built SPA is *tracked* but also matches the nested `cli/visualiser/frontend/.gitignore`, so the rebuild silently untracks it and a subsequent commit records three deletions. It happened on this branch and was corrected before review (the bytes were identical to `main`; the branch diff is clean). Nothing guards against it recurring for the next contributor who commits after a full run — worth its own item.
- Follow-up sequence is **1c → 1b → 2/3/4/5**; 1c must precede the rename or its seam becomes vacuous. The plan's mergeability table carries the reasoning.
