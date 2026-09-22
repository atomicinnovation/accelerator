---
type: "plan-validation"
id: "2026-09-20-0286-eradicate-direct-git-calls-from-skills-validation"
title: "Validation Report: Eradicate Direct Git Calls From Skills Implementation Plan"
date: "2026-09-22T14:10:11+00:00"
author: "Toby Clemson"
producer: "validate-plan"
status: "complete"
result: "pass"
target: "plan:2026-09-20-0286-eradicate-direct-git-calls-from-skills"
tags: ["vcs", "skills", "cli"]
last_updated: "2026-09-22T14:10:11+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Eradicate Direct Git Calls From Skills Implementation Plan

All six phases and the Work Item Reconciliation are fully implemented; every
runnable automated check is green, including the read-only CI mirror (`mise run
check`, exit 0). The change surface matches the plan — one new CLI subcommand,
two reference bullets per backend, four skill rewrites, and two registered lint
guards — with no drift into the guard blocklist, `repository_root`, or the
skills the plan ruled out. The behavioural diff-equality and identity-resolution
criteria are the plan's deliberate model-driven carve-out (no CLI backstop, by
the "What We're NOT Doing" decision), so they are listed for attended
verification rather than failed.

### Implementation Status

Each phase landed as a single-concern commit, in dependency order (foundations
first).

- ✓ **Phase 1** — `accelerator vcs root` — fully implemented. Handler over
  `RepoRoot::discover`, `Root` clap variant (no `--fail-safe`), dispatch wiring,
  and the four-topology parity test all present.
- ✓ **Phase 2** — SessionStart diff-range + identity idioms — fully implemented
  in both `JJ_COMMAND_REFERENCE` and `GIT_REFERENCE`, render tests extended, three
  descriptive goldens hand-edited.
- ✓ **Phase 3** — `validate-plan` rewrite — fully implemented. Raw-git evidence
  block replaced, soft "git" prose neutralised, `allowed-tools` extended.
- ✓ **Phase 4** — `config/migrate` status reword — fully implemented.
- ✓ **Phase 5** — `refine-work-item` identity chain + discard prose — fully
  implemented across all three abort sites, eval spec updated.
- ✓ **Phase 6** — git-token lint + SKILL↔CLI reference check — fully implemented,
  both guards wired into `build-system:check` **and** `lint:check`, clap-enum pin
  in place.
- ✓ **Work Item Reconciliation** — all six points written back into work item
  0286.

### Automated Verification Results

Run from the `build-system` secondary workspace (jj mode); `make`/`git` in the
skill body substituted by the session's `mise`/`jj` equivalents.

| Check | Command | Status |
|---|---|---|
| Plugin-wide git-token sweep (`SKILL.md`) | `grep -rE …` over `skills/` | ✅ clean |
| Per-phase grep criteria (P3–P5) | targeted `grep` set | ✅ all clean |
| git-token lint (real tree) | `mise run lint:git-tokens:check` | ✅ exit 0 |
| SKILL↔CLI reference check (real tree) | `mise run lint:skill-cli-refs:check` | ✅ exit 0 |
| Phase 6 guard + gate-pin unit tests | `pytest test_git_tokens/​skill_cli_refs/​mise` | ✅ 66 passed |
| `accelerator-vcs` suite (bash-parity) | `cargo test -p accelerator-vcs --features bash-parity` | ✅ exit 0 |
| Read-only CI mirror | `mise run check` | ✅ exit 0 |

The Rust suite breakdown: **24** lib unittests (incl. `root::tests::*` and the
`detect::tests::*` render assertions), **5** `detect_goldens`, **4**
`root_goldens` — including
`a_jj_secondary_workspace_prints_the_workspace_root_not_the_main_repo`, the
discriminating assertion for the `discover`-not-`repository_root` decision — plus
the pre-existing status/log goldens and parity tests. No failures.

Live behavioural checks I ran against the built `accelerator-vcs` binary:

- ✅ `vcs root` from this secondary workspace printed the workspace path, not
  `…/accelerator`.
- ✅ `vcs root` from `/tmp` exited 1 with `not inside a repository (searched from
  /private/tmp)`.
- ✅ `vcs root --help` describes the working-copy root and the
  secondary-workspace behaviour; no "repository-root lookup" phrasing anywhere in
  `cli/vcs-cli/src/`.
- ✅ `vcs detect --descriptive` (jj mode) carried both new jj bullets plus the
  `git config user.name` fallback clause, emitted through the SessionStart JSON
  envelope.

### Code Review Findings

#### Matches Plan:

- `root.rs` mirrors `detect.rs` (direct probe port over `RepoRoot::discover`),
  errors naming the searched-from path, and the module doc/`--help`/`cli.rs`
  variant carry the working-copy-root and secondary-workspace language the design
  decision mandates.
- The two reference consts carry the load-bearing semantics the model acts on:
  `fork_point(trunk() | @)` (merge-base of both endpoints, not the buggy
  single-revision form), the `root()`-fallback caveat, `<trunk>...HEAD` with the
  `git symbolic-ref` resolution hint, and the jj→git identity fallback clause.
- `validate-plan` defers the diff to the reference by name with no inlined
  backend command, and its checks step uses the `root="$(accelerator vcs root)"
  && cd … && make check test` abort chain exactly as specified.
- `refine-work-item` collapses the author legs into one reference-deferring
  identity step and words all three recovery sites "delete the newly-written
  child file(s)"; the "Parent not updated" site retains its "add their links
  manually" alternative.
- Phase 6 guards build on `tasks/shared/skill_parsing.py`, scope the sweep to
  `SKILL.md` only (eval fixtures untouched), and pin `VCS_SUBCOMMANDS` against the
  clap `Command` enum by test.

#### Deviations from Plan:

- None material. Two cosmetic variances against the plan's illustrative anchors —
  a line-wrapped status sentence in `config/migrate` and a sentence-initial
  "Delete" at one `refine-work-item` recovery site — are wording, not behaviour,
  and match the plan's intent.

#### Potential Issues:

- ⚠️ The Phase 3 diff-range and Phase 5 colocated identity fallback are
  model-driven with **no CLI backstop** — an accepted consequence of the
  no-`vcs diff` decision. Their only detection mechanism is the release-time
  gate below; drift in a consumer repo whose topology differs from the fixtures
  surfaces only at the next attended run, not continuously. This is documented in
  the plan, not a defect.
- The jj-side neutralisations (`jj restore`/`jj config`) are guarded once by the
  Phase 5 greps, not durably — the git-token sweep is git-only by design (a
  blanket jj sweep would false-positive on `update-work-item`'s retained `jj
  restore`). Intentional asymmetry.

### Manual Testing Required

These are the plan's declared release-time gates — model-driven prose that no
unit test can exercise. Run against fixtures carrying ≥1 implementation commit
ahead of an `origin` trunk bookmark (so `trunk()` resolves).

1. `validate-plan` diff evidence (Phase 3):
  - [ ] Pure-jj fixture: non-empty recent-commit list and non-empty
    implementation diff, no `fatal: not a git repository`.
  - [ ] Diff equals the committed cumulative change since the trunk divergence
    point across git-only, colocated, and pure-jj — with trunk advanced past the
    divergence point in ≥1 fixture, so a single-endpoint revset regression is
    caught.
  - [ ] Checks step runs from the checkout `validate-plan` was invoked in.

2. `refine-work-item` author resolution (Phase 5):
  - [ ] jj-only fixture (`jj config user.name` set, no git identity) → child
    resolves that value.
  - [ ] git-only fixture → child resolves the git value via the idiom, no
    hard-coded `git config` in the skill.
  - [ ] Colocated fixture (jj identity unset, `git config user.name` set) → child
    resolves the git value via the reference's fallback (the previously-regressing
    case).
  - [ ] The refine-work-item eval suite passes via the eval harness (not
    CI-gated; `benchmark.json`/`.md` regenerated by the harness, not hand-edited).

3. `research-issue` regression (no code change, coupled to the Phase 2 reference
   edit):
  - [ ] Pure-jj fixture gathers a non-empty log and diff with no `fatal: not a
    git repository`.

### Recommendations

- Execute the release-time gates above before closing work item 0286 at the
  release level; they carry the load-bearing AC5 diff-equality and AC6 colocated
  guarantees.
- Regenerate `refine-work-item`'s `benchmark.json`/`.md` through the eval harness
  as part of that pass — do not hand-edit, to keep the recorded evidence faithful
  to an actual run.
- Treat "Phase 6 merged" as satisfied: the SKILL↔CLI subcommand binding now makes
  the Phase 3/5 → Phase 1/2 ordering a CI failure rather than discipline, so the
  documented ordering constraint is enforced.
- Consider running the full bare `mise run` (adds `docs:check` + the whole test
  suite + in-place formatting) once more at release time; this validation used
  the non-mutating `mise run check` CI mirror plus targeted suites.
