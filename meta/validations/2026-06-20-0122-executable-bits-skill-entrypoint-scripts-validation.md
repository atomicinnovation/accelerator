---
type: plan-validation
id: "2026-06-20-0122-executable-bits-skill-entrypoint-scripts-validation"
title: "Validation Report: Audit and Correct Missing Executable Bits on Skill Entrypoint Scripts"
date: "2026-06-20T21:10:43+00:00"
author: "Toby Clemson"
producer: validate-plan
status: complete
result: "pass"
target: "plan:2026-06-20-0122-executable-bits-skill-entrypoint-scripts"
tags: [scripts, permissions, ci, lint, executable-bit]
last_updated: "2026-06-20T21:10:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Audit and Correct Missing Executable Bits on Skill Entrypoint Scripts

Validated in-session immediately after implementation. All evidence below
was re-gathered fresh (not from memory) against the committed tree.

### Implementation Status

✓ **Phase 1: Correct the Executable Bits** — Fully implemented (commit
  `lxqnzqxl` "Correct executable bits on shell entrypoints and libraries").
  23 mode-only corrections committed: 18 entrypoints flipped `644 → 755`, 5
  sourced-only libraries flipped `755 → 644`.
✓ **Phase 2: Library-List Constant, Bidirectional Guard, Tests, Docs** — Fully
  implemented (commit `krorxosm` "Add exec-bit invariant guard and
  sourced-library manifest"). `SHELL_LIBRARIES` (30 members), the `exec_bits`
  guard, 15 tests, and docs in `tasks/README.md` + `CLAUDE.md`, left unwired.
✓ **Phase 3: Wire the Guard into `mise run check`** — Fully implemented (commit
  `qkozpoxk` "Wire the exec-bit guard into the shell lint check graph").
  `lint:scripts:exec-bits:check` registered and added to the
  `lint:scripts:check` dependency chain; real-tree + AC5 tests added (17 total).

### Automated Verification Results

✓ `mise run check` exits 0 end-to-end (full CI mirror) — re-run during
  validation.
✓ `mise run lint:scripts:exec-bits:check` passes over the real tree, and the
  guard is in the `lint:scripts:check` dependency chain (`mise.toml:240`).
✓ Guard unit + integration tests pass: `tests/unit/tasks/test_exec_bits.py`
  (17 passed).
✓ Invariant holds over the committed tree: a programmatic sweep using the real
  `SHELL_LIBRARIES` and `shell_sources()` reports **zero** violations (no
  off-list non-executable, no on-list executable, no stale manifest entry);
  `test-fixtures/` exempted in both directions.
✓ All 16 named packaged entrypoints (6 jira `*-flow.sh` + 10 `work-item-*`)
  plus `test-interactive-protocol.sh` and the `0004` migration confirmed `755`
  in the committed tree; 5 libraries confirmed `644`.
⚠️ `mise run test` does **not** exit 0 — but for two reasons entirely unrelated
  to this plan (see Potential Issues). The exec-bit-sensitive consequence of
  this work (the newly-executable `test-interactive-protocol.sh` now discovered
  by the suite runner) passes 24/0/0.

### Code Review Findings

#### Matches Plan:

- **Phase 1** corrected exactly the 23 files the plan enumerated, as mode-only
  changes (`jj diff` showed executable-bit flips, no content edits). The
  plan's blocking **AC1 re-derivation gate** was executed: a repo-wide
  source-vs-path classification sweep over the shell + SKILL.md/agent/hook
  surface confirmed the proposed 30-member manifest exactly — no additions or
  removals — and every library-list member (not just the 4 named suspects) was
  statted, finding no other library hiding at `755`.
- **Phase 2** `SHELL_LIBRARIES` is a module-level `frozenset[str]` beside the
  guard (Decision 2); the guard mirrors the existing fail-closed idiom
  (`_EMPTY_SCOPE`, single `Exit(..., code=1)` listing offenders), reads the
  working-copy mode via `os.access` (matching `tasks/test/helpers.py`), exempts
  `test-fixtures/` by path-segment match (Decision 1), and includes the
  stale-entry guard keyed on enumeration (`in_scope`) rather than mere
  existence. Tests include all the anti-vacuous-pass guards the plan mandated
  (materialise-on-disk; patched one-element `SHELL_LIBRARIES` in the stale
  test; near-miss at a violating mode for the fixture-scope test; extensionless
  entry materialised). The exact-membership and dual-use integrity tests pin
  the manifest and the three dual-use entrypoints.
- **Phase 3** wired exactly as specified (`mise.toml` task block + `depends`
  append); no `lint:fix` change (preserves "shell has no autofixer"); no
  `__init__.py` edit (auto-registers via `Collection.from_module`). Real-tree
  integration assertion and AC5 `node_modules`-segment exclusion both present.
- Docs: `### Executable-bit invariant` subsection added to `tasks/README.md`
  covering all six required points (default, two-part rule with the
  `jira-fields.sh` counter-example, maintenance, runner-vs-helper, fixtures,
  working-copy stance); `CLAUDE.md` shell bullet points to it.

#### Deviations from Plan:

- Tests placed in a new sibling `tests/unit/tasks/test_exec_bits.py` rather than
  appended to `test_lint.py` — explicitly permitted by the plan ("or a sibling
  `test_exec_bits.py` following the same conventions").
- The "bogus `SHELL_LIBRARIES` path" negative-path probe was verified via the
  unit test `test_flags_stale_library_entry` (identical real-tree logic) rather
  than a transient live edit to the constant; the two `chmod` probe directions
  were exercised live and reverted. Functionally equivalent coverage.
- Stale-entry construction uses `offenders.extend(<generator>)` instead of a
  `for`/`append` loop — a ruff `PERF401`-driven refinement; behaviour identical.

#### Potential Issues:

- **Working-copy-mode gap (known, documented).** The guard reads working-copy
  mode, not the VCS-recorded mode, so a local uncommitted `chmod` would not be
  caught until commit. This is the deliberate, documented stance (matches
  `tasks/test/helpers.py`); the Phase 1 `jj diff` check confirmed the modes are
  actually committed as `100755`/`100644`. Not a defect.
- **Two pre-existing `mise run test` failures, unrelated to this plan** (mode
  flips cannot affect either):
  1. `meta/reviews/work/0118-reconcile-0007-backfill-sentinel-with-validator-review-1.md`
     carries an empty `relates_to: []`, tripping the corpus-frontmatter
     validator (`test:integration:config`) — the same class of issue commit
     `d7f1df88f` fixed on a sibling 0118 file.
  2. `test:e2e:visualiser` resolved-styles specs fail (a separate frontend/
     design-token concern).
  Neither is a regression from this work and neither is in scope; `mise run
  check` (the gate the plan's Desired End State leads with) is green.

### Manual Testing Required:

1. CI on the branch:
  - [ ] Confirm `mise run scripts:check` (`.github/workflows/main.yml:99-115`)
        is green once pushed — verified locally; the guard is in the chain CI
        runs.
2. Release timing (from Migration Notes, de-jargonned with the user — there is
   no external owner; the scripts are owned by this repo):
  - [ ] Decide whether to cut a patch release now so users on already-released
        versions (which shipped the entrypoints at `0644`) get working
        bare-path invocation, or let the fix ride the next normal release. The
        corrected modes are tracked VCS attributes and propagate automatically
        on fresh checkout/install.

### Recommendations:

- **Out of scope but adjacent:** fix the pre-existing empty `relates_to: []` in
  the 0118 review file (one-line frontmatter fix, direct precedent in
  `d7f1df88f`) in a separate commit, so `test:integration:config` goes green
  again. Offered to the user during implementation.
- Investigate the `test:e2e:visualiser` resolved-styles failures separately —
  unrelated to this work.
- No changes needed to the exec-bit implementation itself; it is complete,
  green under `mise run check`, and well-covered by tests.
