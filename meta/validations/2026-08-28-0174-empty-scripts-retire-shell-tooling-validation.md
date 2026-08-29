---
type: plan-validation
id: "2026-08-28-0174-empty-scripts-retire-shell-tooling-validation"
title: "Validation Report: Empty scripts/ and Retire Shell Tooling and CI Guards Implementation Plan"
date: "2026-08-29T00:46:02+00:00"
author: Toby Clemson
producer: validate-plan
status: complete
result: pass
parent: "plan:2026-08-28-0174-empty-scripts-retire-shell-tooling"
target: "plan:2026-08-28-0174-empty-scripts-retire-shell-tooling"
tags: [shell, tooling, ci, cleanup, scripts]
last_updated: "2026-08-29T00:46:02+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Validation Report: Empty scripts/ and Retire Shell Tooling and CI Guards

Result: **pass**. All ten phases are implemented and committed (fifteen commits,
`vtprvlow`→`qvssuwvw`), every automated lane this plan touched is green, and the
sole open box is an out-of-repo GitHub branch-protection setting the
implementer cannot apply from the repository.

### Implementation Status

Each phase maps to one or more commits in the current history; all phase
success-criteria checkboxes are marked `[x]` bar the branch-protection handoff.

| Phase | Subject | Status |
|---|---|---|
| 1 | Jira external-id cutover (`vtprvlow`) | 🟢 done |
| 2 | Config source-chain deletion (`slvsxvln`) | 🟢 done |
| 3 | Data-file relocations (`nopmosyz`) | 🟢 done |
| 4 | doc-type-inference cutover (`lytqrxzr`) | 🟢 done |
| 5 | linkage-type-pairs deletion (`vmkpyzsn`) | 🟢 done |
| 6 | Orphan library deletions (`xooyqsks`) | 🟢 done |
| 7 | Nine-guard Python port (`qsyyomtv`, `uylwysqz`) | 🟢 done |
| 8 | Hooks floor to zero (`vnxplouq`) | 🟢 done |
| 9 | Bashisms denylist → Python (`olszosoy`) | 🟢 done |
| 10 | Final retirement + rescope (`prlnosrw` + follow-ups) | 🟡 done, one manual handoff open |

Three follow-up commits (`xqrqtmum`, `kkqzzzoq`, `nwyovvmy`, `qvssuwvw`)
relocated the VCS goldens out of `hooks/`, moved a cross-check off the hooks
lane, and scrubbed stale shell references — refinements consistent with the
plan's end state, not deviations from it.

### Automated Verification Results

Ran the lanes this plan created or changed, not the full bare `mise run`.

- ✅ `mise run scripts:check` — shfmt + ShellCheck + Python bashisms over the two survivors, green (465ms).
- ✅ `mise run build-system:check` — ruff + pyrefly + actionlint + dispatch-coherence, green.
- ✅ `mise run test:unit:tasks` — 2771 passed (19.1s), covering the eight ported guards and the Python bashisms scanner.
- ✅ `mise run test:integration:conformance` — 27 passed; the validator-driven conformance lane replacing `test:integration:config`.
- ✅ `mise run test:integration:hooks` — 24 passed, including the two ported VCS-detect guards.
- ✅ `cargo test --all-features` over `accelerator-work`, `accelerator-vcs`, `config`, `corpus`, `corpus-adapters`, `design` — all suites `ok`, exit 0, no `warnings = "deny"` breakage.

⚠️ Not run: the full bare `mise run` default (frontend build, docs+Chromium,
repeated Rust compiles). The plan asserts it green per phase; I verified the
lanes it edited rather than re-running the whole heavy mirror.

### Code Review Findings

#### Matches Plan

- `scripts/` is gone entirely — `find scripts -name '*.sh'` and `ls scripts` both empty.
- Both data files relocated: `cli/corpus/src/frontmatter_validation/templates-schema.tsv` and `cli/design/tests/extract-work-items-cue-phrases.txt`.
- `work link-external-id` exists as a `pub` free function in `sync_author.rs:77` with the trait method delegating to it (`:161`), dispatched via `main.rs:471`; the Jira SKILL.md writeback is repointed onto it (`SKILL.md:111`) with the caveat reworded (`:124`).
- Walk machinery retired cleanly: `shell_sources()` has zero callers, `SHELL_LIBRARIES` / `_RECONCILED_LIBRARIES` / `test_exec_bits` fully removed, `SURVIVING_SHELL_SOURCES` (`sources.py:41`) and `walk_files` (`:71`) retained as designed.
- Both survivors are tracked-executable (`0755`).
- `check-scripts` job removed (`grep -c check-scripts main.yml` → 0); the shell lane re-homed as an explicit "Run script checks" step in `check-build-system` (`main.yml:165`).
- `tasks/README.md:78` carries the "Surviving thin shell" section naming both survivors and `SURVIVING_SHELL_SOURCES` as authoritative.

#### Deviations from Plan (all documented, all green)

- **`tasks/measure.py` `RECOVERED_FILES` retains `scripts/vcs-common.sh`** (`:982`). Intentional and user-confirmed in the Phase 2 report: it recovers the file from a pinned `BASELINE_COMMIT` as the recovered guard's runtime dependency, not from the live tree. Removing it would break `recover_baseline`. This is the only surviving `scripts/` reference in production sources.
- **Native config-path-key check homed in `visualiser/server/tests/parity.rs`** rather than a new test, reusing a crate that already depends on both `corpus` and `config` (no new crate edge). Recorded in the Phase 2 report.
- **Hyphenation port named `test_hyphenation_format.py`** to avoid clobbering the pre-existing shfmt-task `test_format.py`. Recorded in Phase 7.

#### Potential Issues

- None found. The `warnings = "deny"` + `--all-features` compile constraint that this plan repeatedly flagged as the per-commit greenness hazard holds: the targeted cargo run compiled all six touched crates with no unused-import errors.

### Manual Testing Required

1. Branch-protection handoff (out-of-repo, at merge):
  - [ ] ⚠️ Drop the "Check scripts" required status check in the GitHub ruleset.
  - [ ] ⚠️ Confirm "Check build system" is required — it now carries the shell lane. If "Check scripts" stays required, PRs hang forever waiting on a status no job reports.

2. Full CI mirror (optional confirmation):
  - [ ] Run the bare `mise run` default once locally, or rely on the branch's CI run, to cover the heavy Rust/frontend/docs lanes left unexercised here.

### Recommendations

- Apply the branch-protection ruleset change in lockstep with the merge — it is the one gating step outside the repository and the only unchecked box in the plan.
- Treat the `RECOVERED_FILES` `vcs-common.sh` entry as load-bearing provenance, not residue; a future sweep for `scripts/` references should skip it.
