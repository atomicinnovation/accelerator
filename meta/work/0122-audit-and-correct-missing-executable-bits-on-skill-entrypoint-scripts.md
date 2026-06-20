---
type: work-item
id: "0122"
title: "Audit and Correct Missing Executable Bits on Skill Entrypoint Scripts"
date: "2026-06-20T13:29:42+00:00"
author: Toby Clemson
producer: create-work-item
status: ready
kind: task
priority: medium
relates_to: ["work-item:0106", "work-item:0098", "work-item:0107"]
tags: [scripts, permissions, ci, lint, plugin, executable-bit]
last_updated: "2026-06-20T13:29:42+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-143
---

# 0122: Audit and Correct Missing Executable Bits on Skill Entrypoint Scripts

**Kind**: Task
**Status**: Ready
**Priority**: Medium
**Author**: Toby Clemson

## Summary

A number of `.sh` scripts in the repo that are meant to run directly as
executables (invoked by bare path from skill bodies, hooks, migrations, mise
tasks, and test runners) do not have the executable bit set. Under the bare-path
invocation convention established by work item 0106 — skills run scripts
directly, never prefixed with `bash`/`sh`/`env` — a direct-invocation entrypoint
that lacks `+x` is a latent break: it fails with "permission denied" or escapes
its `allowed-tools` permission. Maintain a checked-in **library-list** — an
explicit allowlist of the sourced-only scripts — then enforce a single
invariant: a tracked `.sh` is executable (`100755`) **if and only if** it is not
on the list. That means setting `+x` on every off-list entrypoint that lacks it
and clearing `+x` from every on-list library that wrongly carries it, backed by
a CI guard, driven by that same list, so the condition cannot recur.

## Context

Work item 0106 ("Invoke Plugin Scripts by Bare Path in Skill Bodies", done)
states the contract plainly: scripts "self-execute (shebang `#!/usr/bin/env
bash` + execute bit)" and skill bodies invoke them by bare path. The execute bit
is therefore load-bearing, not cosmetic — the bare-path form only works if the
file is executable.

A read-only scan of the working tree (excluding `node_modules/`, `workspaces/`,
and `target/`) shows two distinct populations among the scripts missing `+x`:

- **Sourced libraries** where the absent bit is *correct*: `fs-common.sh`,
  `hash-common.sh`, `jsonl-common.sh`, `log-common.sh`, `work-common.sh`,
  `config-defaults.sh`, `doc-type-table.sh`, `frontmatter-emission-rules.sh`,
  `frontmatter-fixtures.sh`, `interactive-harness.sh`, `interactive-protocol.sh`,
  `test-helpers.sh`, `skills/config/migrate/scripts/interactive-lib.sh`, and —
  confirmed during this review — `scripts/accelerator-scaffold.sh` (sourced per
  the 0031 plan) and `scripts/doc-type-inference.sh` (its own header states it is
  "single-sourced by the 0007 migration and the corpus validator").
- **Genuine bugs** — entrypoints missing the bit:
  `skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh`
  (its siblings `0001`–`0007` are all executable) and
  `scripts/test-interactive-protocol.sh` (a test runner; its sibling test
  suites are all executable).

A first-pass scan initially misjudged `accelerator-scaffold.sh` and
`doc-type-inference.sh` as entrypoint bugs precisely because both carry a
`#!/usr/bin/env bash` shebang despite being sourced — the exact failure mode a
shebang-presence heuristic would inherit, and the reason this work item uses an
explicit library-list audited against real call sites instead.

So the task is not "chmod +x everything" — it is to distinguish entrypoints from
sourced libraries and correct only the entrypoints, then prevent regression.
Rather than infer the distinction with a heuristic (shebang presence + a
path-reference search that excludes `source`/`.`), this work item maintains an
explicit, checked-in **library-list** of the sourced-only scripts. The list is
the authoritative classification: every tracked `.sh` *not* on it is treated as
an entrypoint and must be executable. This makes classification deterministic
and auditable (the list itself is the artefact a reviewer inspects), keeps the
maintained set small (libraries are the minority and rarely added), and fails
safe — a newly added script that nobody classifies defaults to "must be
executable", which is the correct default since entrypoints are the common case.

The shell lint toolchain from work item 0098 ("Repo-Wide Linting, Formatting,
And Static Analysis Guardrails", done) — `lint:shell` (ShellCheck + `shfmt -d`),
the custom bashisms linter, and the `scripts` check component — is the natural
home for the new guard, which slots alongside the existing `lint:shell` task in
`tasks/lint/scripts.py`. The guard must enumerate scripts the same way the
existing shell tooling does: via the `shell_sources()` helper
(`tasks/shared/sources.py:60`), which walks the tree honouring `.gitignore`,
never via `git ls-files`, which is blind inside jj workspaces.

## Requirements

- **Establish the library-list.** Add a checked-in manifest enumerating, by
  repo-relative path, every `.sh` script that is sourced-only (loaded via
  `source`/`.` and never invoked by path). It is produced by auditing the
  current tree — see Technical Notes for the seed set the audit must start from.
  The list is the authoritative classifier: a tracked `.sh` is an **entrypoint**
  if and only if it is *not* on the list. The list lives alongside the shell-lint
  configuration (exact location an implementation detail) and is committed.
  (Throughout this work item, scripts are named by bare basename in prose as
  shorthand for the unique tracked file of that name; the manifest itself stores
  repo-relative paths.)
- **Correct both directions.** Set the executable bit (`chmod +x`, committed so
  the version-control system (VCS) records mode `100755`) on every tracked `.sh`
  not on the library-list that currently lacks it, **and** clear it
  (`chmod -x` → `100644`) on every library-list member that currently carries it
  (e.g. `atomic-common.sh`, `config-common.sh`, `vcs-common.sh`,
  `work-item-common.sh`). This establishes the invariant *executable ⟺ off the
  library-list*. This is safe with no false positives: AC1 guarantees every list
  member is sourced-only and never invoked by path, so none ever needs `+x`.
- Resolve the known suspects explicitly: `migrations/0004-…sh`,
  `accelerator-scaffold.sh`, `doc-type-inference.sh`, and
  `test-interactive-protocol.sh` are each either placed on the library-list or
  corrected to `100755`. Their expected resolutions are recorded in the
  Acceptance Criteria.
- **Regression guard (bidirectional).** Add an automated check in the
  `scripts`/`lint:shell` family (alongside `tasks/lint/scripts.py`) that enforces
  the *executable ⟺ off the library-list* invariant in both directions: it fails,
  naming each offender, when a tracked `.sh` not on the library-list lacks `+x`,
  **and** when a library-list member carries `+x`. It enumerates files via
  `shell_sources()` (`tasks/shared/sources.py:60`) — the existing
  `.gitignore`-honouring walk — never `git ls-files`, and so already excludes
  `node_modules/`/`workspaces/`/`target/`. Wire it into `mise run check` so CI
  enforces it. The guard reports violations only — correction stays a manual
  `chmod` (the check names the file); it is deliberately **not** wired into
  `mise run fix`, preserving the repo's "shell has no autofixer" convention.
- **Guard the list itself.** The check must also fail if a path on the
  library-list no longer exists (a stale entry), so the manifest cannot silently
  rot and start exempting a since-renamed entrypoint.
- Document the library-list mechanism alongside the shell-lint conventions: what
  the list is for, that any new sourced-only library must be registered on it
  (otherwise the guard will demand `+x`), and the test-runner-versus-test-helper
  discriminating example.

## Acceptance Criteria

- [ ] A checked-in library-list manifest exists, naming each sourced-only `.sh`
      by repo-relative path. Every path on it resolves to an existing tracked
      file, and each listed file is in fact loaded via `source`/`.` and never
      invoked by path — so the list is auditable entry-by-entry against real call
      sites, not by re-deriving a heuristic. The "never invoked by path" half is
      verified by a defined procedure: for each listed path, a repo-wide search
      over the `shell_sources()` corpus plus the skill/agent/hook surface
      (`SKILL.md` bodies, `agents/`, `hooks/`, mise/invoke tasks) finds **≥ 1**
      `source`/`.` reference and **zero** bare-path or `bash`/`sh`/`env`-prefixed
      invocations. An entry failing this (a referenced-by-path file, or one with
      no references at all) is not a library and must come off the list.
- [ ] Given the committed tree, the *executable ⟺ off the library-list*
      invariant holds for every tracked `.sh` (committed mode recorded by the
      VCS, as seen on a fresh git clone): each file not on the library-list has
      mode `100755`, and each library-list member has mode `100644` — including
      the libraries that previously carried `+x` (`atomic-common.sh`,
      `config-common.sh`, `vcs-common.sh`, `work-item-common.sh`, and any others
      the audit finds).
- [ ] The four named suspects resolve to these exact outcomes, recorded as the
      audit's expected answers:
      - `migrations/0004-restructure-meta-research-into-subject-subcategories.sh`
        → entrypoint → `100755`.
      - `scripts/test-interactive-protocol.sh` → entrypoint → `100755`.
      - `scripts/accelerator-scaffold.sh` → library-list entry (sourced) → stays
        `100644`.
      - `scripts/doc-type-inference.sh` → library-list entry (sourced) → stays
        `100644`.
- [ ] Given an entrypoint whose exec bit is removed, when `mise run check` runs,
      then it fails and names the offending file; given a library-list member
      that carries `+x`, the check also fails naming it; given a library-list
      path that no longer exists, the check also fails naming the stale entry;
      given the tree and list both correct, the check passes.
- [ ] Given the guard enumerates files via `shell_sources()`, then a known
      vendored script — `skills/visualisation/visualise/frontend/node_modules/playwright-core/bin/reinstall_chrome_stable_linux.sh`
      — does not appear in its input set, demonstrating `node_modules/` (and
      likewise `workspaces/`/`target/`) are excluded.
- [ ] The library-list mechanism is documented where the shell-lint conventions
      live, and the documentation states (a) that a new sourced-only library must
      be added to the list or the guard will demand `+x`, and (b) the
      test-runner-versus-test-helper discriminating example, so a future
      contributor can classify a new script unaided.

## Open Questions

None remaining. The two questions that were open at review time are now
resolved; the decisions and their rationale are recorded in Drafting Notes and
reflected in Requirements and Acceptance Criteria:

- **Bidirectional guard** → adopted. The guard enforces *executable ⟺ off the
  library-list* in both directions.
- **`mise run fix` wiring** → declined. The guard reports only; correction stays
  a manual `chmod`, preserving the "shell has no autofixer" convention.

## Dependencies

- Blocked by: none.
- Relates to: 0106 (bare-path invocation convention — the reason the exec bit
  matters), 0098 (shell lint toolchain — where the guard lives), 0107 (companion
  skill-body invocation lint).
- **Shared extension point with 0107**: both this work item and 0107 add a new
  check into the same `scripts`/`lint:shell` task family (`tasks/lint/scripts.py`,
  wired into `mise run check`). They are independent in *intent* but touch the
  same task-registration surface; whichever lands second should expect a small
  rebase against the other's task wiring. No hard ordering — neither blocks the
  other.
- **Realisation surface — plugin packaging/release**: the missing bit is latent
  inside the repo but manifests when the plugin is packaged and a skill invokes
  the entrypoint by bare path. If step-1 of this work finds a *currently-shipped*
  entrypoint broken — a file not on the library-list, missing `+x`, that is part
  of the packaged plugin artefact and invoked by bare path at runtime (the strict
  0106 case, not merely a repo-internal test runner) — escalate to the
  release/packaging owner and bump priority to high per the Drafting Notes — this
  is the named trigger and escalation target for that event. If that conditional
  fires, add a Blocks-style entry (or a follow-up release item) so the
  release/packaging coupling is tracked rather than left as inline narrative.
- **`mise run fix` autofixer convention**: the decision (see Drafting Notes) is
  to keep correction manual — the guard reports violations and `mise run fix`
  gains no auto-`chmod`, preserving the established "shell has no autofixer"
  convention. No coupling to the broader fix-task design is introduced.

## Assumptions

- "Intended to be executable as part of claude skills" is read broadly to
  include not just scripts named in `SKILL.md` bodies but also hooks,
  migrations, mise-invoked scripts, and test runners — all of which are invoked
  by path. If the intent was narrower (only `SKILL.md`-referenced scripts), the
  scope shrinks accordingly.
- The exec bit is tracked by the VCS (mode `100755` vs `100644`), so a single
  committed `chmod +x` propagates the fix to every checkout and CI run.

## Technical Notes

- A script's mode is checked with `test -x` / `[ -x … ]`. Classification is a
  membership test against the library-list, not a heuristic — but building the
  list initially requires auditing each candidate's real call sites (does
  anything `source`/`.` it, or only invoke it by path?). A shebang alone proves
  nothing: sourced libraries here routinely carry `#!/usr/bin/env bash` for
  editor/ShellCheck/`set -euo pipefail` ergonomics (e.g. `doc-type-inference.sh`
  is explicitly "safe to source" yet shebang-led).
- **Audit seed set** — the sourced-only scripts already identified for the
  initial library-list: `fs-common.sh`, `hash-common.sh`, `jsonl-common.sh`,
  `log-common.sh`, `work-common.sh`, `config-defaults.sh`, `doc-type-table.sh`,
  `frontmatter-emission-rules.sh`, `frontmatter-fixtures.sh`,
  `interactive-harness.sh`, `interactive-protocol.sh`, `test-helpers.sh`,
  `skills/config/migrate/scripts/interactive-lib.sh`, `accelerator-scaffold.sh`,
  and `doc-type-inference.sh`. The audit must confirm this set and look for any
  others before freezing the list.
- The `test-*.sh` family splits across both classes: test *runners* (e.g.
  `test-interactive-protocol.sh`) are entrypoints and stay off the list; test
  *helpers* (e.g. `test-helpers.sh`) are sourced libraries and go on it. Note
  test runners are conventionally invoked via `bash scripts/test-x.sh` (per the
  repo's "run a single test" guidance), so the bit is not strictly load-bearing
  for them — they are treated as entrypoints (`100755`) for consistency with the
  many sibling test suites that are already executable.
- Reuse `shell_sources()` (`tasks/shared/sources.py:60`) for enumeration — it
  walks the tree via pathspec honouring `.gitignore`, correct inside jj
  workspaces where `git ls-files` is blind. The guard belongs next to the
  existing `lint:shell` task in `tasks/lint/scripts.py`.
- The exec bit is a tracked VCS attribute (git mode `100755` vs `100644`); a
  committed `chmod +x` propagates to every checkout and CI run. "Committed mode
  recorded by the VCS" — not the working-copy `stat` — is what the acceptance
  criteria refer to.

## Drafting Notes

- **Decision — bidirectional guard (resolves the first prior Open Question):**
  chose to enforce *executable ⟺ off the library-list* in both directions rather
  than only requiring `+x` on entrypoints. Rationale: the library-list's own
  membership rule (AC1: sourced-only, never invoked by path) means no list member
  can ever legitimately need `+x`, so enforcing `100644` on members has zero
  false-positive risk and turns the file mode into a reliable, self-checking
  signal. The cost — a one-time `chmod -x` on the few sourced libs that currently
  carry the bit — is mechanical and cheap. A reviewer who prefers minimal churn
  could revert to entrypoint-only enforcement, but the invariant is cleaner.
- **Decision — no `mise run fix` autofixer (resolves the second prior Open
  Question):** the guard reports violations only; correction stays a manual
  `chmod`. Rationale: respects the established "shell has no autofixer"
  convention (scripts is deliberately absent from `lint:fix`), the triggering
  event is rare, and the guard already names the offending file. Not worth making
  this the repo's first shell autofixer.
- **Design pivot (this review):** replaced the original shebang-plus-path-
  reference *heuristic* with a maintained, checked-in **library-list** of
  sourced-only scripts. The list is the authoritative classifier (everything not
  on it is an entrypoint that must be executable), which makes classification
  deterministic and auditable and fails safe toward "executable". Adopting it
  also caught two misclassifications: `accelerator-scaffold.sh` and
  `doc-type-inference.sh` look like entrypoints under a shebang heuristic but are
  in fact sourced libraries — they belong on the list, not in the correction set.
- Treated `test-*.sh` runners as entrypoints but `test-helpers.sh` as a sourced
  library; flagged `test-interactive-protocol.sh` (runner → `+x`) versus
  `test-helpers.sh` (helper → on the list) as the discriminating case the
  documentation must teach.
- Read the topic's "scripts … as part of claude skills" broadly (hooks,
  migrations, test runners, mise-invoked scripts), per the bare-path convention
  in 0106 — narrowing to only SKILL.md-referenced scripts is a viable smaller
  scope if preferred.
- Assumed the guard belongs in the existing `scripts`/`lint:shell` check rather
  than as a new standalone task or component.
- Set priority to medium: latent rather than actively breaking inside the repo
  (git tracks the mode), but a real risk for the packaged plugin. Would raise to
  high if step-1 classification finds a currently-shipped entrypoint broken.

## References

- Related: 0106 (`meta/work/0106-invoke-plugin-scripts-by-bare-path.md`),
  0098 (`meta/work/0098-repo-wide-linting-formatting-static-analysis.md`),
  0107 (`meta/work/0107-lint-skill-body-script-invocations.md`)
