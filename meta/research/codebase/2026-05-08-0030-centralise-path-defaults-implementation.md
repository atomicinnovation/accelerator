---
date: "2026-05-08T01:08:36+01:00"
researcher: Toby Clemson
git_commit: 7dab2df48e6016dd4fe9b3a807193000432ea002
branch: HEAD
repository: accelerator
topic: "Implementation of work item 0030: centralise PATH and TEMPLATE config arrays"
tags: [research, codebase, config, refactoring, work-0030, path-defaults, config-dump]
status: complete
last_updated: 2026-05-08
last_updated_by: Toby Clemson
last_updated_note: "Corrected: workspaces/ is jj workspace checkouts, not source duplicates. There is only one config-dump.sh."
---

# Research: Implementation of work item 0030 — centralise PATH and TEMPLATE config arrays

**Date**: 2026-05-08 01:08 BST
**Researcher**: Toby Clemson
**Git Commit**: 7dab2df48e6016dd4fe9b3a807193000432ea002
**Branch**: HEAD
**Repository**: accelerator

## Research Question

What does the codebase currently look like for the changes proposed in
`meta/work/0030-centralise-path-defaults.md`, and what implementation approach
fits the existing source-graph and test harness?

## Important Premise: workspaces/ is not source code

The `workspaces/` directory at the repo root contains **jj workspace
checkouts** (`build-system`, `ticket-management`, `visualisation-system`),
confirmed by `jj workspace list`. Files inside `workspaces/*/scripts/...`
are not distinct source files — they are parallel working copies of the
same repository at different revisions. They must be ignored for the
purposes of this research.

This invalidates a key premise of the work item: the Summary, Context,
Requirements, Acceptance Criteria (AC1, AC2), Assumptions, and Technical
Notes all reference "4 `config-dump.sh` files" / "the four files to
migrate". **In reality there is only one `config-dump.sh` file**, at
`scripts/config-dump.sh`. The migration is a 1-file extraction, not 4.

The work item itself needs an update before implementation; the
re-counted scope is captured below.

## Summary

With `workspaces/` excluded, work item 0030 reduces to a small,
self-contained refactor:

1. Create `scripts/config-defaults.sh` with three array definitions
   (`PATH_KEYS`, `PATH_DEFAULTS`, `TEMPLATE_KEYS`).
2. Replace the inline definitions in `scripts/config-dump.sh:175-219` with
   `source "$SCRIPT_DIR/config-defaults.sh"` (or source it from
   `config-common.sh:8` so all current and future config scripts pick it
   up automatically).
3. Run `mise run test:integration:config` for the config-dump and
   path-resolution tests.

Three points still merit attention:

- **`${CLAUDE_PLUGIN_ROOT}` is the wrong sourcing mechanism** — none of
  the relevant scripts use it, and it isn't reliably set under
  `mise run test` or CLI invocations. The existing
  `$SCRIPT_DIR` self-resolution pattern is the correct fit.
- **The "11 consumer scripts" figure is stale** — actual is 13 bash
  consumers. Doesn't block 0030 (consumers are explicitly out of scope)
  but the count drift means the work item's Summary needs a tweak.
- **AC3's "config-init tests" claim is not exercised by `mise run test`**.
  `tasks/test/integration.py:21-24` runs only `scripts/test-*.sh`, not
  `skills/config/init/scripts/test-init.sh`. Pre-existing gap, not
  introduced by 0030, but AC3's wording either needs tightening or the
  test wiring needs a small extension.

## Detailed Findings

### 1. The single `config-dump.sh` file — array contents

Defining file: `scripts/config-dump.sh` (lines 175-219). Array contents:

**`PATH_KEYS`** (`scripts/config-dump.sh:175-187`, 11 entries): `paths.plans`,
`paths.research`, `paths.decisions`, `paths.prs`, `paths.validations`,
`paths.review_plans`, `paths.review_prs`, `paths.review_work`,
`paths.templates`, `paths.work`, `paths.notes`.

**`PATH_DEFAULTS`** (`scripts/config-dump.sh:189-201`, paired by index):
`meta/plans`, `meta/research`, `meta/decisions`, `meta/prs`,
`meta/validations`, `meta/reviews/plans`, `meta/reviews/prs`,
`meta/reviews/work`, `.accelerator/templates`, `meta/work`, `meta/notes`.

**`TEMPLATE_KEYS`** (`scripts/config-dump.sh:212-219`, 6 entries):
`templates.plan`, `templates.research`, `templates.adr`,
`templates.validation`, `templates.pr-description`, `templates.work-item`.

All three definitions use the bare `=` array-literal form (no
`declare -a`, no associative arrays), so the work item's AC2 grep pattern
(`'PATH_KEYS=\|PATH_DEFAULTS=\|TEMPLATE_KEYS='`) catches all definition
forms.

### 2. Source-graph and the `${CLAUDE_PLUGIN_ROOT}` question

`scripts/config-dump.sh:11-13` uses the standard self-location pattern:

```
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"
config_assert_no_legacy_layout
```

`scripts/config-common.sh:1-11` already establishes `SCRIPT_DIR` from
`BASH_SOURCE[0]` and houses shared constants (`AGENT_PREFIX`,
`CONFIG_TEMPLATE_SOURCE_*`). It is intentionally not `set -e`-armed so
callers retain control.

**Two equivalent sourcing options**:

- **(a) Source from `config-common.sh:8`** (after `vcs-common.sh`).
  Propagates the arrays to `config-dump.sh` and any future caller of
  `config-common.sh` automatically. `config-dump.sh` itself just deletes
  the inline definitions. Smallest diff to the working file.
- **(b) Source directly from `config-dump.sh:14`** (alongside the
  existing `source "$SCRIPT_DIR/config-common.sh"`). More explicit, but
  doesn't help any other config script that might want the arrays.

Option (a) is cleaner given `config-common.sh` is already the "shared
bash module" (already exports `AGENT_PREFIX`, parsers, resolvers).

**`${CLAUDE_PLUGIN_ROOT}` is wrong for this case**:

- Not referenced in `scripts/config-dump.sh` or `scripts/config-common.sh`.
- Set by Claude Code only for hooks and `allowed-tools` SKILL frontmatter
  — not guaranteed under `mise run test` or direct CLI invocation.
- The `$SCRIPT_DIR` pattern is the existing convention; deviating would
  introduce a fragile dependency on harness-set environment variables.

**Recommended sourcing**: `source "$SCRIPT_DIR/config-defaults.sh"` from
`scripts/config-common.sh:8`. The work item's Requirements should be
updated to specify this rather than `${CLAUDE_PLUGIN_ROOT}`.

### 3. Consumer-site count discrepancy

The work item asserts "11 consumer scripts that reference the same key
names inline". Excluding `workspaces/`, current state:

**Bash consumer scripts that pass `<key> <default>` pairs to
`config-read-path.sh` or `config-read-value.sh` (13 files)**:

1. `scripts/config-common.sh:211` (templates)
2. `scripts/config-eject-template.sh:69` (templates)
3. `scripts/config-summary.sh:20` (tmp)
4. `skills/visualisation/visualise/scripts/launch-server.sh:16` (tmp)
5. `skills/visualisation/visualise/scripts/status-server.sh:13` (tmp)
6. `skills/visualisation/visualise/scripts/stop-server.sh:13` (tmp)
7. `skills/visualisation/visualise/scripts/write-visualiser-config.sh:34,43-44`
8. `skills/config/init/scripts/init.sh:8,34,52`
9. `skills/decisions/scripts/adr-next-number.sh:34` (decisions)
10. `skills/work/scripts/work-item-resolve-id.sh:38` (work)
11. `skills/work/scripts/work-item-next-number.sh:51` (work)
12. `skills/integrations/jira/scripts/jira-common.sh:73` (integrations)
13. `skills/design/inventory-design/scripts/playwright/run.sh:21` (tmp)

**SKILL.md files with inline `config-read-path.sh <key> <default>` exec
blocks (~25 files)** — see Code References below for the full list.

**Migration scripts that intentionally hardcode keys (special-case)**:
`skills/config/migrate/migrations/0001-rename-tickets-to-work.sh`,
`skills/config/migrate/migrations/0003-relocate-accelerator-state.sh`.

The work item's "11 consumer scripts" figure is stale by 2 (predates
`skills/design/inventory-design/scripts/playwright/run.sh` and does not
count `scripts/config-common.sh` / `scripts/config-eject-template.sh`).
Doesn't block 0030 — consumer-site refactoring is explicitly out of
scope per Assumptions — but the Summary's "15 sites in total" needs the
counts updating.

### 4. `config_resolve_template` and `TEMPLATE_DEFAULTS` non-existence

The work item asserts (correctly) that `TEMPLATE_DEFAULTS` does not
exist. `scripts/config-common.sh:188-227` defines
`config_resolve_template()` which uses a three-tier lookup:

1. Config-path tier: `"$SCRIPT_DIR/config-read-value.sh" "templates.${key}" ""`.
2. Templates-dir tier:
   `"$SCRIPT_DIR/config-read-path.sh" templates .accelerator/templates`
   (one of the 13 inline duplications listed above).
3. Plugin-default tier: `$plugin_root/templates/${key}.md`.

Confirmed by grep: `TEMPLATE_DEFAULTS` matches zero files outside the
work item itself and its review.

### 5. `config-read-path.sh` — comment-only enumeration confirmed

`scripts/config-read-path.sh` is 26 lines total. Lines 7-21 are a comment
block listing supported keys. The implementation (lines 22-25) prepends
`paths.` and execs `config-read-value.sh`:

```
exec "$SCRIPT_DIR/config-read-value.sh" "paths.${1:-}" "${2:-}"
```

No executable array definitions; all call-site defaults are inline string
arguments at the consumer scripts.

### 6. `init.sh`'s `DIR_KEYS`/`DIR_DEFAULTS` — out of scope and shape-different

`skills/config/init/scripts/init.sh:18-29` defines:

- `DIR_KEYS` (12 entries, **bare keys**: `plans`, `research`, …, plus
  `design_inventories`, `design_gaps` — not present in `PATH_KEYS`).
- `DIR_DEFAULTS` (paired).

Three shape differences from `config-dump.sh`'s arrays:

1. `init.sh` uses **bare keys** (`plans`); `config-dump.sh` uses
   **prefixed keys** (`paths.plans`). `config-read-path.sh` prepends
   `paths.` itself, so each side is consistent with its callsite.
2. `init.sh` includes `design_inventories` and `design_gaps`; `PATH_KEYS`
   includes neither.
3. `init.sh` excludes `templates` (handled separately at lines 46-49) and
   handles `tmp` in a separate step (lines 51-62).

`init.sh` does **not** source `config-common.sh`, so even if
`config-defaults.sh` is sourced from `config-common.sh`, the arrays do
not become available to `init.sh` without an explicit additional source
line. Sharing the arrays would also need an adapter or a different
canonical shape. The work item's Open Questions correctly defers this
as out of scope for 0030.

### 7. Test infrastructure — partial coverage

**`test:integration:config` task**: `tasks/test/integration.py:21-24`
calls `run_shell_suites(context, "scripts")`. `tasks/test/helpers.py:13-34`
globs `**/test-*.sh` under the given subtree. So the config integration
suite runs every `scripts/test-*.sh` file (9 files, including the
relevant `scripts/test-config.sh`).

**`scripts/test-config.sh` coverage** (~4400 lines):
- `=== config-dump.sh ===` block at lines 2426-2555 — covers AC3's
  "config-dump tests".
- `=== config-read-path.sh ===` block at lines 2606-2761 — covers AC3's
  "path-resolution tests" with both default-when-unset and
  override-honoured cases for all path keys.
- Lines 3039-3194 verify SKILL.md files reference `config-read-path.sh`
  correctly.

**`skills/config/init/scripts/test-init.sh`** (config-init tests):
- Tests `init.sh` end-to-end (12 dirs, .gitkeep, scaffold, gitignore).
- **NOT picked up by any `mise run test` task.**
  `tasks/test/integration.py` globs only `scripts/`,
  `skills/visualisation/visualise/`, and `skills/decisions/`.

**Implication for AC3**: As written, "all config-dump, path-resolution,
and config-init tests pass" needs:
- `mise run test:integration:config` for config-dump and path-resolution.
- A separate manual invocation `bash skills/config/init/scripts/test-init.sh`
  for config-init.

This is a pre-existing test-wiring gap, not introduced by 0030. Either
AC3's wording should match what `mise run test` actually runs, or a
small additional change in `tasks/test/integration.py` should add
`skills/config/init/` to the test discovery. Since 0030 does not touch
`init.sh`, the simpler fix is tightening AC3's wording.

### 8. Implementation approach — recommendation

With `workspaces/` correctly excluded, the implementation collapses to:

1. **Create `scripts/config-defaults.sh`** with the three array literals
   verbatim (no `declare -a`, no rearrangement) plus a banner comment
   documenting that these are shared between `config-dump.sh` and any
   future caller of `config-common.sh`.

2. **Edit `scripts/config-common.sh`** to add
   `source "$SCRIPT_DIR/config-defaults.sh"` near line 8 (after the
   existing `source "$SCRIPT_DIR/vcs-common.sh"` line). Since
   `config-common.sh` is sourced by `config-dump.sh`, the arrays become
   transitively available without any change to `config-dump.sh`'s
   source statements.

3. **Edit `scripts/config-dump.sh`** to delete lines 175-187, 189-201,
   and 212-219 (the array definitions). The arrays are then provided by
   the transitive source.

4. **Update the work item** (`meta/work/0030-centralise-path-defaults.md`)
   to:
   - Replace "4 files" → "1 file" throughout (Summary, Context,
     Requirements, AC1, AC2, Assumptions, Technical Notes).
   - Replace `${CLAUDE_PLUGIN_ROOT}` → `$SCRIPT_DIR` in the Requirements
     example.
   - Update the consumer-script count from 11 → 13 (Summary, Context).
   - Tighten AC3 to name `mise run test:integration:config` and
     acknowledge that `test-init.sh` is not currently in the harness
     (or, alternatively, scope a small `tasks/test/integration.py` edit
     to include it).

5. **Run AC3**: `mise run test:integration:config`. Optionally
   `bash skills/config/init/scripts/test-init.sh` for completeness, but
   `init.sh` is not touched by this change.

## Code References

### Definition site (the single file to migrate)

- `scripts/config-dump.sh:175-187` — `PATH_KEYS`.
- `scripts/config-dump.sh:189-201` — `PATH_DEFAULTS`.
- `scripts/config-dump.sh:212-219` — `TEMPLATE_KEYS`.

### Source-graph anchors

- `scripts/config-common.sh:7` — `SCRIPT_DIR` set from `BASH_SOURCE[0]`.
- `scripts/config-common.sh:8` — sources `vcs-common.sh` (recommended
  insertion point for `source "$SCRIPT_DIR/config-defaults.sh"`).
- `scripts/config-common.sh:11` — `AGENT_PREFIX="accelerator:"` (existing
  shared constant — establishes the "shared bash constants" pattern).
- `scripts/config-common.sh:188-227` — `config_resolve_template()`
  three-tier template fallback.
- `scripts/config-dump.sh:11-13` — `SCRIPT_DIR` + `source config-common.sh`
  + `config_assert_no_legacy_layout`.

### Test infrastructure

- `mise.toml:104-112` — `test:integration` task chain.
- `tasks/test/integration.py:21-24` — `config()` task globs `scripts/`.
- `tasks/test/helpers.py:13-34` — `run_shell_suites` glob.
- `scripts/test-config.sh:2426-2555` — `config-dump.sh` test block.
- `scripts/test-config.sh:2606-2761` — `config-read-path.sh` test block.
- `skills/config/init/scripts/test-init.sh` — config-init tests (NOT
  picked up by `mise run test`).

### Consumer-script call sites (representative — full list above)

- `scripts/config-common.sh:211` — `templates .accelerator/templates`.
- `scripts/config-summary.sh:20` — `tmp .accelerator/tmp`.
- `skills/config/init/scripts/init.sh:18-29` — `DIR_KEYS`/`DIR_DEFAULTS`
  (independent shape; out of scope).
- `skills/work/scripts/work-item-next-number.sh:51` — `work meta/work`.

### Confirmed non-existence

- `scripts/config-defaults.sh` — does not exist (glob returned 0 matches).
- `TEMPLATE_DEFAULTS` — does not exist as a variable anywhere.
- `config_resolve_path` — does not exist as a function.

## Architecture Insights

**`config-common.sh` is the existing "shared bash module".** Sourced by
`config-dump.sh`, it provides shared constants (`AGENT_PREFIX`), parsers
(`config_extract_frontmatter`, `config_parse_array`), and resolvers
(`config_resolve_template`). Adding `config-defaults.sh` as a sibling
that `config-common.sh` sources fits this pattern naturally — the arrays
become "shared constants" alongside `AGENT_PREFIX`.

**Two key vocabularies coexist**: prefixed (`paths.plans`, used by
`config-dump.sh` calling `config-read-value.sh` directly) and bare
(`plans`, used by `init.sh` calling `config-read-path.sh` which prepends
`paths.`). Any future unification of `DIR_KEYS` with `PATH_KEYS` must
choose a canonical form and adapt the other side. This is why the work
item's Open Questions defer `DIR_KEYS`.

## Historical Context

- `meta/plans/2026-04-25-rename-tickets-to-work-items.md` (status:
  complete) — the migration that revealed the duplication problem
  (`paths.tickets` → `paths.work` rewrite touching 15 sites) and
  spawned 0030 as a follow-on. The "15 sites" framing originates here.
- `meta/decisions/ADR-0023-meta-directory-migration-framework.md`
  (status: accepted) — establishes the `config-dump.sh` / migration
  framework that 0030 refactors.
- `meta/plans/2026-03-23-config-infrastructure.md` (status: final/complete)
  — originating plan for the shared configuration utilities.
- `meta/plans/2026-03-23-template-and-path-customisation.md` — introduced
  `config-read-path.sh` and the PATH key model.
- `meta/plans/2026-03-29-template-management-subcommands.md` (status:
  complete) — introduced `TEMPLATE_KEYS` semantics.
- `meta/work/0024-configuration-system-architecture.md` (done),
  `meta/work/0025-configuration-extension-points.md` (done),
  `meta/work/0027-ephemeral-file-separation-via-paths-tmp.md` (done),
  `meta/work/0029-template-management-subcommand-surface.md` (done) —
  underlying work items.
- `meta/work/0031-consolidate-accelerator-owned-files-under-accelerator.md`
  (status: done) — sibling refactor that established
  `.accelerator/templates` as the canonical templates path.
- `meta/notes/2026-04-26-agents-hardcode-default-directory-locations.md`
  — tech-debt note on inline path defaults; thematically aligned with
  0030 + 0052.

## Related Work Items and Reviews

- `meta/work/0052-make-documents-locator-paths-config-driven.md` (status:
  draft) — explicit downstream consumer; will source `config-defaults.sh`
  once 0030 lands.
- `meta/reviews/work/0030-centralise-path-defaults-review-1.md` — four
  review passes; pass 4 verdict COMMENT. None of the review passes
  caught the workspaces/jj-checkout misreading (because the work item
  itself frames the duplication as "4 parallel copies").
- `meta/reviews/work/0052-make-documents-locator-paths-config-driven-review-1.md`
  — review of the dependent work item; verdict REVISE.

## Open Questions

1. **The work item itself needs an update before implementation.** Its
   premise of "4 `config-dump.sh` files" is wrong because it counted jj
   workspace checkouts as separate source files. Suggested edits listed
   in §8 above.

2. **Sourcing mechanism for `config-defaults.sh`**: confirm whether to
   source from `config-common.sh:8` (option a, recommended) or directly
   from `config-dump.sh` (option b). Option a is the convention-fit and
   propagates to future callers; option b is more explicit. Trivial
   either way.

3. **AC3's `test-init.sh` claim**: tighten the wording to match what
   `mise run test` actually runs, or add `skills/config/init/` to the
   test discovery. Either is small. The latter is also useful for
   future work that does touch `init.sh`.

4. **`DIR_KEYS`/`DIR_DEFAULTS` follow-on task** is referenced in 0030's
   Open Questions as "work item to be created" but no number is
   assigned. Should be created so the coupling is trackable.

## Related Research

This is the first dedicated research for 0030. Prior context lives in
`meta/plans/2026-04-25-rename-tickets-to-work-items.md` (the migration
that spawned 0030) and the four review passes in
`meta/reviews/work/0030-centralise-path-defaults-review-1.md`.
