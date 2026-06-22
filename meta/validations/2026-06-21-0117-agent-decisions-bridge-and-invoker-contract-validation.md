---
type: plan-validation
id: "2026-06-21-0117-agent-decisions-bridge-and-invoker-contract-validation"
title: "Validation Report: Agent-Decisions Bridge and Documented Invoker Contract"
date: "2026-06-22T23:23:41+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: "pass"
parent: "plan:2026-06-21-0117-agent-decisions-bridge-and-invoker-contract"
target: "plan:2026-06-21-0117-agent-decisions-bridge-and-invoker-contract"
tags: [migrate, interactive-migration, agent-invocation, tooling]
last_updated: "2026-06-22T23:23:41+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Agent-Decisions Bridge and Documented Invoker Contract

### Implementation Status

✓ Phase 1: `--list` dry-emit, standalone fixture, flag parser, `--help` — Fully implemented
✓ Phase 2: Fail-closed validation via a no-mutation dry-apply pass (AC6) — Fully implemented
✓ Phase 3: `SKILL.md` invoker contract + ADR-0037 judgment (AC5) — Fully implemented

All in-scope acceptance criteria (AC1–AC6) are satisfied. AC7 (live-0007
integration) was explicitly out of scope and remains gated on 0118.

### Automated Verification Results

✓ Migrate interactive suite passes: `bash skills/config/migrate/scripts/test-migrate-interactive.sh` — **294 passed, 0 skipped, 0 failed**
✓ Shell lint/format/bashisms/exec-bits clean: `mise run scripts:check` (shellcheck, shfmt, bashisms, exec-bits all green)
✓ `--help` routes to **stdout** with `--list` and `ACCELERATOR_MIGRATE_DECISIONS_FILE`; stderr empty (0 bytes) on the help path
✓ Unknown-flag rejection: `run-migrations.sh --frobnicate` → `Unknown argument: --frobnicate`, exit 1
✓ Migrate-suite floor preserved: exactly 4 `test-*.sh` migrate suites (no new file added) — `test-migrate.sh`, `test-migrate-0007.sh`, `test-migrate-interactive.sh`, `test-migrate-snapshot.sh`

Note: the full `mise run check` (all four toolchains) was not re-run as part of
this validation — the change is shell-only and `mise run scripts:check` (the
relevant component gate) is green. The plan records `mise run check` as having
passed during implementation.

### Code Review Findings

#### Matches Plan:

- **Wire protocol** (`scripts/interactive-protocol.sh`): the five new frames
  `LIST_ENTRY`/`LIST_DONE`/`DRY_OK`/`DRY_REJECT`/`DRY_DONE` are documented with
  the same `escape_field` encoding note as `PROMPT`.
- **Child-side modes** (`scripts/interactive-harness.sh`): `_harness_emit_list`,
  `_harness_dry_apply`, and the shared single-source `_harness_classify_tx` are
  all present; all three modes route through the one classifier as designed.
- **Shared fork plumbing** (`interactive-lib.sh`): `_interactive_fork` /
  `_interactive_teardown` extracted and used by the live run, `--list`
  (`enumerate_interactive_transformations`, mode 1), and dry-apply
  (`dry_apply_interactive_migration`, mode 2). The `_FORK_STOP` named sentinel is
  declared re-source-safe (`readonly` guarded on `${_FORK_STOP:-}`).
- **Flag parser**: converted to `while/shift`; `--list` arm, strict `*)`
  unknown-flag rejection, dirty-tree pre-flight gated on `[ -z "$LIST_MODE" ]`.
- **Standalone fixture**: `0006-decisions-bridge.sh` exists with three pinned
  transformations writing real frontmatter via POSIX-portable awk (ENVIRON[]
  value-transparency) plus the `.fixture/applied/log` sentinel oracle.
- **SKILL.md**: AC5 elements (a)–(e) all confirmed by string search — the literal
  `list → decide → write → resume`, `matched by emission order`, verb tokens,
  0116 link + stall marker, env var + `--help` pointer, and fail-closed wording.
- **CHANGELOG**: documents strict argument handling (unrecognised flag or
  positional arg now exits non-zero) and `--help` → stdout.

#### Deviations from Plan (all deliberate, recorded in the plan's "As-built notes"):

- **Dry mode rides on `MIGRATION_HARNESS_MODE` env var, not a third `INIT`
  field** — an empty `decisions_path` middle field would collapse under IFS-tab
  word-splitting; the runner signals mode out-of-band so `INIT` stays
  byte-identical to the live two-field form. (Plan §1 originally sketched a third
  INIT field; the as-built note documents and justifies the change.)
- **Dry-apply mirrors the live `VALIDATE_ERR` re-prompt rather than hard-rejecting
  a bad edit** — preserves "validates == applies"; a terminal bad edit still fails
  closed. Confirmed with the author and documented.
- **Resume drift modelled in dry-apply** — a drifted recorded key is re-prompted
  in the dry pass (consuming a decision as the live run would) without the `DRIFT`
  round-trip, keeping consumption identical.

These deviations are improvements that strengthen the fail-closed invariant and
are explicitly reconciled in the plan body, not silent drift.

#### Potential Issues:

- **No all-or-nothing apply across transformations** — explicitly out of scope
  and documented in both the plan and SKILL.md. Once the live apply loop begins
  (for a validated file), an apply-time failure can leave a partial corpus; VCS
  revert + 0119 guarded resume is the recovery path. Consistent with the project
  convention against dry-run/confirm UX for destructive ops.
- **Mechanical-route applies are not dry-run** — the partial-mutation caveat
  applies to them in full; the reference fixture is all-prompt so this gap is
  unexercised by AC1–AC6. Documented.

### Manual Testing Required:

The suite already automates the substance of every manual item below
(byte-exact `--list` output, clean `.accelerator/state/` after dry runs, frame
ordering, actionable error messages). Remaining genuinely-manual confirmations:

1. Invoker ergonomics:
  - [ ] `--list` output reads naturally — tab-delimited, `path:anchor` join legible
  - [ ] Error messages name the position, key, and expected verbs / reject reason
2. ADR judgment:
  - [ ] The ADR-0037 implementation-detail judgment reads correctly against the
    recursive-supplement clause (captured in the plan + Phase 3 commit message)

### Recommendations:

- None blocking. The implementation is complete, fully tested (294 assertions),
  and the plan is already marked `done` with all automated success criteria
  checked. AC7 follow-up correctly tracked under 0118.
- Consider a future follow-up (already noted as out of scope) for the
  multi-migration decisions-file resume protocol and dry-running the mechanical
  route, should a real corpus need either.
