---
date: 2026-05-05T00:46:40+01:00
researcher: Toby Clemson
git_commit: 3e96e9c5e616a93f42a7b1496c8c6e0b72092b9b
branch: HEAD (work item drafting)
repository: accelerator
topic: "Consolidating Accelerator-owned files under .accelerator/ (work item 0031)"
tags: [research, codebase, configuration, migration, init, init-jira, visualiser, paths, integrations]
status: complete
last_updated: 2026-05-05
last_updated_by: Toby Clemson
last_updated_note: "Resolved all seven open questions via Q1-Q7 review."
---

# Research: Consolidating Accelerator-Owned Files Under `.accelerator/` (Work Item 0031)

**Date**: 2026-05-05 00:46:40 BST
**Researcher**: Toby Clemson
**Git Commit**: 3e96e9c5e616a93f42a7b1496c8c6e0b72092b9b
**Branch**: HEAD (detached, work item drafting branch)
**Repository**: accelerator

## Research Question

What is the current state of every code surface and document touched by work
item 0031 (`Consolidate Accelerator-Owned Files Under .accelerator/`), so that
implementation planning can proceed with concrete file:line references and any
hidden complexity surfaced before drafting?

## Summary

The work item is largely accurate but there are five material findings that
should shape the plan:

1. **`config-read-path.sh` is a pure pass-through.** Adding the `integrations`
   key requires only a one-line update to the doc-comment header — no executable
   code change. The runtime behaviour (`paths.integrations` lookup with
   default-fallback) already works because the script forwards `paths.${1}` and
   `${2:-}` directly to `config-read-value.sh`. (`scripts/config-read-path.sh:21-24`)

2. **Jira integration's path resolution is already centralised.** All seven
   Jira skills route through `jira_state_dir()` in `jira-common.sh:54-71`,
   which already calls `config-read-path.sh integrations meta/integrations`.
   The runtime change for the `meta/integrations/` → `.accelerator/state/integrations/`
   move is **a single line** (`jira-common.sh:62`). No flow script hardcodes
   the path.

3. **The init skill restructure is bigger than the work item suggests.**
   The init skill creates 14 directories via a single iteration loop
   (`skills/config/init/SKILL.md:42`, `<!-- DIR_COUNT:14 -->`), of which only
   2 (`templates`, `tmp`) move under `.accelerator/`. The other 12
   (plans, research, decisions, prs, validations, review_plans, review_prs,
   review_work, work, notes, design_inventories, design_gaps) STAY in `meta/`.
   The init skill needs a hybrid structure: a separate step creating the
   `.accelerator/` core scaffold (config-localignore, state/, skills/, lenses/),
   plus a modified iteration that handles the split path destinations.

4. **`init-jira` requires three behaviour changes — not just a path change.**
   The work item describes ownership semantics (`init-jira` owns its state
   directory) that are partially in place but materially differ from the
   target:
   - Today `.gitignore` is written at the **project root**, ignoring
     `<state>/.lock` and `<state>/.refresh-meta.json`. Target: write
     `.gitignore` **inside** the state dir, ignoring `site.json`.
   - Today `fields.json` and `projects.json` are **overwritten on every run**.
     Acceptance criterion at work item:229-232 requires they not be overwritten
     when the directory already exists. This is a discover-flow behaviour
     change, not just a path change.
   - Today there is no `.gitkeep` written and no warning if `.accelerator/`
     is absent. Both are new requirements.

5. **The migration framework has a self-referential bootstrap problem.**
   `run-migrations.sh:13-14` captures `STATE_FILE="$PROJECT_ROOT/meta/.migrations-applied"`
   **before** invoking the migration. Migration 0003 cannot relocate the state
   files on its own — the driver must be updated atomically with the migration
   so that `STATE_FILE` already points at `.accelerator/state/migrations-applied`
   when 0003 runs. Same applies to `migrate-discoverability.sh:22`. This
   matches the work item's explicit "atomic delivery" clause (work item:318-321)
   and ADR-0023's pinned-path-preservation contract.

The work item omits one non-trivial visualiser surface
(`scripts/write-visualiser-config.sh:48`, `meta/templates` default arg), and the
`integrations` row in `skills/config/configure/SKILL.md:402` already documents
the key with default `meta/integrations` (so this needs updating too — the
configure-skill table is currently divergent from the `config-read-path.sh`
header comment, which omits `integrations`).

## Detailed Findings

### Config-resolution scripts (`scripts/`)

All readers source `scripts/config-common.sh`, which is the single source of
truth for hardcoded `.claude/accelerator*.md` paths and the tier-2 templates
default. Adding a new path key requires **no executable change** to
`config-read-path.sh` (the script is a 4-line forwarder).

**`scripts/config-common.sh`** (the load-bearing file):
- `config-common.sh:25` — `team="$root/.claude/accelerator.md"` (hardcoded)
- `config-common.sh:26` — `local_="$root/.claude/accelerator.local.md"`
  (hardcoded)
- `config-common.sh:153-193` — `config_resolve_template()`: implements 3-tier
  resolution. **Tier 2 default `meta/templates` lives at line 176**, passed as
  the second arg to `config-read-path.sh templates meta/templates`. A single
  edit at this line propagates the new default to every template caller.

**`scripts/config-read-path.sh`** (4 lines of code, 25 with comments):
- Line 24 — `exec "$SCRIPT_DIR/config-read-value.sh" "paths.${1:-}" "${2:-}"`.
  Pure pass-through. `${2:-}` is forwarded as `DEFAULT`.
- Lines 7-19 — header comment listing 11 documented keys. **`integrations`
  is missing from this list.** Adding it is a one-line doc edit.

**`scripts/config-read-skill-context.sh:22`** —
`CONTEXT_FILE="$PROJECT_ROOT/.claude/accelerator/skills/$SKILL_NAME/context.md"`
(hardcoded).

**`scripts/config-read-skill-instructions.sh:23`** —
`INSTRUCTIONS_FILE="$PROJECT_ROOT/.claude/accelerator/skills/$SKILL_NAME/instructions.md"`
(hardcoded).

**`scripts/config-summary.sh`**:
- Line 19 — `config-read-path.sh tmp meta/tmp`
- Line 91 — `SKILL_CUSTOM_DIR="$ROOT/.claude/accelerator/skills"` (hardcoded)
- Line 114 — warning string mentions `.claude/accelerator/skills/$skill_name/`

**`scripts/config-read-review.sh:220`** —
`CUSTOM_LENSES_DIR="$PROJECT_ROOT/.claude/accelerator/lenses"` (hardcoded).

**`scripts/config-dump.sh`** — display strings hardcoded:
- Lines 23-24, 98-99 — `.claude/accelerator.md`, `.claude/accelerator.local.md`
- Lines 174-186, 188-200 — `PATH_KEYS` and `PATH_DEFAULTS` arrays. `tmp` and
  `integrations` are NOT in the list (the dump does not iterate them).

**`scripts/config-read-template.sh`**:
- Comments at lines 14, 17-18 mention `meta/templates` and `.claude/accelerator/`.
  Resolution itself lives in `config-common.sh:153-193`, so no executable
  change is needed in this file beyond the header comment.

There is **no helper function** that constructs `.claude/accelerator/...`
paths centrally. Each consumer hardcodes its own literal — which is why the
edits are scattered across 5 scripts.

### `init` skill (`skills/config/init/`)

- `skills/config/init/SKILL.md` is the only file (no helper scripts).
- Lines 20-33 — resolves 14 directories via `config-read-path.sh <key> meta/<dir>`.
- Line 42 — `<!-- DIR_COUNT:14 -->` marker drives the Step 1 loop.
- Lines 40-60 — Step 1: iterate, `mkdir -p`, `touch .gitkeep`.
- Lines 62-81 — Step 2: write inner `.gitignore` for tmp dir (exact body at
  lines 74-79: `*`, `!.gitkeep`, `!.gitignore` — ADR-0019 pattern).
- Lines 83-99 — Step 3: append `.claude/accelerator.local.md` to root
  `.gitignore` (idempotent).

**Implication for the reorg.** Of the 14 paths, only `templates` and `tmp` move
to `.accelerator/`. The remaining 12 stay in `meta/`. The init skill therefore
cannot collapse its Step 1 to "create everything under `.accelerator/`". It
needs:
- A new step (or extension of Step 3) creating the `.accelerator/` core
  scaffold: top-level `.gitignore` (covering `config.local.md`), `state/`,
  `skills/`, `lenses/`, `templates/`, `tmp/` (with its inner `.gitignore`).
- Modified path-default args throughout — but defaults are already supplied
  via `config-read-path.sh <key> <default>`, so this is just a string change
  per resolved line.
- Step 3's root `.gitignore` rule changes from `.claude/accelerator.local.md`
  to `config.local.md` (or `.accelerator/config.local.md` — the work item is
  ambiguous here; see Open Questions).

Idempotency is well-documented in the existing skill (line 4 description "Safe
to run repeatedly"; lines 56-59, 71-72, 93-97) — the new steps must preserve
this.

### `init-jira` skill (`skills/integrations/jira/init-jira/`)

State-directory resolution is **already centralised** through
`jira_state_dir()` (`skills/integrations/jira/scripts/jira-common.sh:54-71`),
which calls `config-read-path.sh integrations meta/integrations` (line 62) and
suffixes `/jira`. **All seven Jira skills route through this helper.**

State-write call sites (each goes through `jira_state_dir`):
- `jira-init-flow.sh:45-46, 108, 130, 179, 200`
- `jira-fields.sh:44-45, 90, 121-122`
- `jira-search-flow.sh:73-75`
- `jira-create-flow.sh:90-93, 244-245`
- `jira-update-flow.sh:99-100, 253-254`

**Behaviour deltas vs. work item target:**

| Aspect | Current | Work item target |
|---|---|---|
| `.gitignore` location | Project root (`jira-init-flow.sh:56`) | Inside state dir (work item:108-111) |
| `.gitignore` rules | `<state>/.lock`, `<state>/.refresh-meta.json` (`jira-init-flow.sh:62-66`) | `site.json` (work item:108) |
| `.gitkeep` | Not written | Required (work item:112-113) |
| `fields.json` overwrite | Always overwritten (`jira-fields.sh:75`) | Not overwritten if present (work item:229-232) |
| `projects.json` overwrite | Always overwritten (`jira-init-flow.sh:146`) | Not overwritten if present (work item:229-232) |
| `.accelerator/` absence warning | None | Required to stderr (work item:119-121) |
| `issuetypes/` directory | Not created or referenced anywhere | Stated in target structure (work item:85), but not in current AC for `init-jira` — likely future work |

The current `.refresh-meta.json` sidecar is written every refresh with a fresh
timestamp (`jira-fields.sh:77-80`), so the SKILL prose claim of "byte
idempotency" only applies to `fields.json` content, not the sidecar.

User-facing prose in `skills/integrations/jira/init-jira/SKILL.md` mentions
`meta/integrations/jira/` literally at lines 8, 106, 148-149, 153 — these
need updating.

### Migration framework

**Driver**: `skills/config/migrate/scripts/run-migrations.sh`
- Lines 13-14 — `STATE_FILE="$PROJECT_ROOT/meta/.migrations-applied"`,
  `SKIP_FILE="$PROJECT_ROOT/meta/.migrations-skipped"`. **Captured at script
  start** — the driver itself cannot read post-relocated state.
- Lines 41-69 — clean-tree pre-flight. Currently checks `meta/` and
  `.claude/accelerator.md`/`.claude/accelerator.local.md`. Must be extended
  to cover `.accelerator/`.
- Lines 86-92 — discovers migrations via
  `[0-9][0-9][0-9][0-9]-*.sh` glob.
- Lines 178-207 — invokes each migration with `PROJECT_ROOT` and
  `CLAUDE_PLUGIN_ROOT` env vars, captures stdout/stderr, writes ID to
  `STATE_FILE` only on success.
- Line 204 — `atomic_append_unique "$STATE_FILE" "$id"` records applied
  migration. Because `STATE_FILE` is captured at line 13, **a migration that
  moves the state file does not change where the driver writes the ID after
  the migration finishes**. Migration 0003 must be released atomically with
  driver updates so `STATE_FILE` already points to `.accelerator/state/migrations-applied`
  when 0003 begins.

**Discoverability hook**: `hooks/migrate-discoverability.sh`
- Lines 14-18 — sentinel detection: `.claude/accelerator.md` OR `meta/`. Must
  add `.accelerator/`.
- Line 22 — `STATE_FILE="$PROJECT_ROOT/meta/.migrations-applied"` (hardcoded).
- Line 51 — warning string mentions `meta/.migrations-applied`.

**Existing migrations**: `skills/config/migrate/migrations/`
- `0001-rename-tickets-to-work.sh` — confirmed.
- `0002-rename-work-items-with-project-prefix.sh` — confirmed.
- Next free number is `0003` (verified against `test-migrate.sh:494, 497, 502, 520`).

**Migration script conventions to follow:**
- Shebang `#!/usr/bin/env bash` + `# DESCRIPTION:` line (parsed by driver at
  `run-migrations.sh:164` for the preview banner) + `set -euo pipefail`.
- Bootstrap: source `$PLUGIN_ROOT/scripts/config-common.sh` and (for atomic
  rewrites) `atomic-common.sh`.
- Idempotency: `[ -d SOURCE ]` / `[ -f SOURCE ]` guards; collision check
  `[ -d SOURCE ] && [ -d DESTINATION ] → abort`.
- `MIGRATION_RESULT: no_op_pending` sentinel for soft-defer (see 0002:24-27).
- Driver appends to state file only after the migration script returns 0.

**Atomic delivery requirement.** Three files must change in the same
commit/PR as `0003-relocate-accelerator-state.sh`:
1. `scripts/config/migrate/scripts/run-migrations.sh` — extend clean-tree
   regex to cover `.accelerator/`; relocate `STATE_FILE`/`SKIP_FILE` paths to
   `.accelerator/state/migrations-{applied,skipped}`.
2. `hooks/migrate-discoverability.sh` — extend sentinel detection; relocate
   `STATE_FILE` path; update warning string.
3. The migration script itself.

This is consistent with the work item:165-181 and 318-321.

### Visualiser (`skills/visualisation/visualise/`)

All scripts route through `config-read-path.sh`; only default args and prose
strings need updating. Confirmed line numbers from work item are accurate.

- `scripts/launch-server.sh:16` — `tmp meta/tmp` ✅
- `scripts/launch-server.sh:127` — error hint string with
  `.claude/accelerator.local.md` ✅
- `scripts/stop-server.sh:13` — `tmp meta/tmp` ✅
- `scripts/status-server.sh:13` — `tmp meta/tmp` ✅
- `SKILL.md:21` — `templates meta/templates` ✅
- `SKILL.md:24` — `tmp meta/tmp` ✅
- `SKILL.md:96-98` — config docs prose ✅
- `server/tests/fixtures/config.valid.json` — lines 5, 9, 24, 29, 34, 39,
  44 contain `meta/tmp` and `meta/templates` strings ✅

**Surface omitted from the work item:**

`skills/visualisation/visualise/scripts/write-visualiser-config.sh:48` —
`TEMPLATES_USER_ROOT="$(abs_path templates meta/templates)"`. This file is
invoked from `launch-server.sh:189` and embeds the same `meta/templates`
default. To stay consistent with `SKILL.md:21`, this default arg should also
update to `.accelerator/templates`.

### Jira integration runtime surface

Jira state path is centralised through `jira_state_dir`
(`jira-common.sh:54-71`); this is the **only** runtime line that needs to
change to relocate the integration state directory:

```bash
# jira-common.sh:62 — change `meta/integrations` to `.accelerator/state/integrations`
integrations_path=$(cd "$root" && "$_JIRA_PLUGIN_ROOT/scripts/config-read-path.sh" \
  integrations meta/integrations)
```

No flow script in any of the seven Jira skills hardcodes the integration
state path. State files referenced are: `site.json`, `fields.json`,
`projects.json`, `.refresh-meta.json`, `.lock/`. **No `issuetypes/`
subdirectory exists or is referenced** in the runtime — the work item:85
target structure lists it but the current implementation does not create it.

### Configure skill (`skills/config/configure/SKILL.md`)

21 references to the affected paths. Notable lines:
- Lines 19-20 — config-file table.
- Lines 31-32 — config-file existence checks.
- Lines 47, 51 — section headers.
- Line 59 — `.claude/accelerator/skills/` enumeration.
- Lines 96, 239, 242, 306, 309, 336 — references to skills/lenses/global
  context paths.
- Line 396 — `templates` row in path table (default `meta/templates`).
- Line 399 — `tmp` row in path table (default `meta/tmp`).
- **Line 402 — `integrations` row in path table (default `meta/integrations`).
  This row already exists (added during Jira Phase 1) and its default needs
  updating to `.accelerator/state/integrations`.** This is divergent from
  `config-read-path.sh`'s header comment, which doesn't list `integrations`.
- Lines 621, 624 — templates directory prose.
- Lines 830-831 — config-file paths in editing instructions.

### README.md

Old-path references at lines 90, 107, 115, 122, 128, 160-161, 228, 246, 254,
257, 324-325, 352, 387, 483. All require updating.

### CHANGELOG.md

References at lines 109, 112, 143-145, 248-249, 255, 339, 426, 441, 444, 447,
456, 572, 575, 600, 619, 632. CHANGELOG.md is historical — these should NOT
be updated retroactively. The work item:158-161 implies adding a new entry
documenting the reorg, not editing prior entries.

### Root `.gitignore`

Three relevant entries today:
- Line 13 — `.claude/accelerator.local.md`
- `meta/integrations/jira/.lock`
- `meta/integrations/jira/.refresh-meta.json`

After migration 0003, the second and third lines become obsolete (the
inner `.gitignore` inside `.accelerator/state/integrations/jira/` covers
their replacements). The first line should be replaced with `.accelerator/config.local.md`
(or whatever the work item finalises).

### ADR / note context

Synthesis from the seven referenced documents (see Historical Context below):

- **ADR-0016**: Config file *names* (`accelerator.md`, `accelerator.local.md`)
  are load-bearing. Their parent `.claude/` is convention, not constraint —
  selected for `plugin-dev` ecosystem alignment. Renaming to `config.md` is
  a mild break with this rationale that the work item does not address.
- **ADR-0017**: Custom lenses at `.claude/accelerator/lenses/*/SKILL.md`,
  three-tier templates with tier-2 at `meta/templates/`. Work item moves the
  former under `.accelerator/lenses/` (consistent with the unification
  intent) and the latter under `.accelerator/templates/`.
- **ADR-0019**: `paths.tmp` default `meta/tmp`, inner-gitignore pattern
  `*`/`!.gitkeep`/`!.gitignore` (load-bearing — never use root-level rule).
  Work item preserves the inner-gitignore mechanism intact.
- **ADR-0020**: Per-skill `context.md`/`instructions.md` at
  `.claude/accelerator/skills/<name>/`. Work item moves under
  `.accelerator/skills/<name>/` — must keep the file-name and no-frontmatter
  contract.
- **ADR-0023**: Migration framework with clean-tree pre-flight, pinned-path
  preservation, atomic per-step semantics, `meta/.migrations-applied` state.
  Work item leverages this for migration 0003; the driver itself must
  upgrade because the state files relocate.
- **2026-04-29 reorg note**: Source proposal. Work item explicitly extends
  scope to include lenses and per-skill customisation directories (not in
  the original note); explicitly rejects the note's backward-compat read
  window in favour of a hard cut.
- **2026-04-29 Jira research**: Defers the reorg to a separate work item
  (this one); proposes pre-adding `paths.integrations` to limit damage —
  partially done (key works via delegate, not yet documented in
  `config-read-path.sh`).

## Code References

- `scripts/config-read-path.sh:21-24` — pure pass-through; no per-key logic.
- `scripts/config-common.sh:25-26, 176` — hardcoded config-file paths and
  tier-2 templates default.
- `scripts/config-read-skill-context.sh:22` — hardcoded skills directory.
- `scripts/config-read-skill-instructions.sh:23` — hardcoded skills directory.
- `scripts/config-summary.sh:19, 91, 114` — `tmp` default, skills dir, warning.
- `scripts/config-read-review.sh:220` — hardcoded lenses directory.
- `scripts/config-dump.sh:23-24, 98-99` — display strings.
- `skills/config/init/SKILL.md:20-33, 40-99` — 14-directory iteration plus
  3-step bootstrap.
- `skills/config/migrate/scripts/run-migrations.sh:13-14, 41-69, 178-207` —
  state file paths, clean-tree check, migration invocation.
- `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh:1-100` —
  reference convention for 0003.
- `skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh:1-30`
  — `MIGRATION_RESULT: no_op_pending` sentinel pattern.
- `skills/integrations/jira/scripts/jira-common.sh:54-71` — `jira_state_dir`,
  the single point of state-path resolution. Line 62 is the runtime change.
- `skills/integrations/jira/scripts/jira-init-flow.sh:40-67, 108-114, 130-146`
  — `.gitignore` writing, `site.json`/`projects.json` writes.
- `skills/integrations/jira/scripts/jira-fields.sh:44-80` — `fields.json` and
  `.refresh-meta.json` writes.
- `skills/integrations/jira/init-jira/SKILL.md:8, 106, 148-149, 153` — prose
  paths.
- `skills/visualisation/visualise/scripts/launch-server.sh:16, 127` — tmp
  default, error hint.
- `skills/visualisation/visualise/scripts/stop-server.sh:13` — tmp default.
- `skills/visualisation/visualise/scripts/status-server.sh:13` — tmp default.
- `skills/visualisation/visualise/scripts/write-visualiser-config.sh:48` —
  templates default (omitted from work item).
- `skills/visualisation/visualise/SKILL.md:21, 24, 96-98` — defaults and
  prose.
- `skills/visualisation/visualise/server/tests/fixtures/config.valid.json:5, 9, 24, 29, 34, 39, 44`
  — fixture paths.
- `skills/config/configure/SKILL.md:19-20, 31-32, 47, 51, 59, 96, 239, 242, 306, 309, 336, 396, 399, 402, 621, 624, 830-831`
  — pervasive prose references; line 402 holds the existing `integrations`
  table row.
- `hooks/migrate-discoverability.sh:14-18, 22, 51` — sentinel detection,
  state file path, warning string.
- `.gitignore:13` — `.claude/accelerator.local.md` rule. Plus
  `meta/integrations/jira/.lock` and `meta/integrations/jira/.refresh-meta.json`
  entries that become obsolete post-migration.

## Architecture Insights

- **Pass-through path resolution.** `config-read-path.sh` is intentionally a
  4-line forwarder. New keys do not need code changes — only documentation
  changes. This makes `paths.integrations` essentially "already wired up"; the
  only work is updating the doc comment, the configure SKILL.md table row's
  default, and the runtime default arg in `jira-common.sh:62`.
- **Defaults supplied by callers, not by readers.** Every `config-read-path.sh`
  call site supplies its own default as the second arg. There is no central
  defaults registry. Renaming a default therefore touches every call site
  that supplies the old default — but each one is a single string change.
- **Single load-bearing helper for Jira state.** `jira_state_dir` is the only
  place in the seven Jira skills that resolves the integration state path.
  This is a clean abstraction the reorg can leverage.
- **Self-referential migration framework.** State files and clean-tree-check
  paths are themselves part of what's being migrated. Atomic delivery of the
  migration script with the driver and hook updates is mandatory, not optional.
- **Init skill's mixed destinations.** Of 14 created directories, 12 stay
  under `meta/` (project content) and 2 move under `.accelerator/` (plugin
  state). The init skill's iteration is no longer uniform after this work —
  the loop body needs a per-key destination override or the iteration must
  split into two passes.
- **Inner-gitignore pattern is load-bearing.** ADR-0019 explicitly rejects
  root-level `meta/tmp/` ignore rules because they prevent git from
  descending. The same pattern applies to the new `.accelerator/tmp/` and
  to `.accelerator/state/integrations/jira/`.

## Historical Context

- `meta/decisions/ADR-0016-userspace-configuration-model.md` — establishes
  two-tier `accelerator.md`/`accelerator.local.md` config files. File names
  are load-bearing; `.claude/` parent is convention.
- `meta/decisions/ADR-0017-configuration-extension-points.md` — three-tier
  templates with tier-2 at `meta/templates/`; custom lens auto-discovery from
  `.claude/accelerator/lenses/*/SKILL.md`.
- `meta/decisions/ADR-0019-ephemeral-file-separation-via-paths-tmp.md` —
  `paths.tmp` key with default `meta/tmp` and the inner-gitignore pattern
  (`*`, `!.gitkeep`, `!.gitignore`). Inner-gitignore is the *only* sanctioned
  mechanism.
- `meta/decisions/ADR-0020-per-skill-customisation-directory.md` — per-skill
  `context.md`/`instructions.md` at `.claude/accelerator/skills/<name>/`. Two
  reader scripts; raw markdown only (no frontmatter).
- `meta/decisions/ADR-0023-meta-directory-migration-framework.md` — migration
  framework: ordered `[0-9]{4}-*.sh` scripts, `meta/.migrations-applied`
  state, clean-tree pre-flight covering `meta/` and `.claude/accelerator*.md`,
  pinned-path preservation, SessionStart discoverability hook.
- `meta/notes/2026-04-29-accelerator-config-state-reorg.md` — source proposal
  for this work item. Originally planned a backward-compat read window with
  one-minor-version removal target. Work item explicitly rejects the read
  window in favour of a hard cut (work item:51-54).
- `meta/research/2026-04-29-jira-cloud-integration-skills.md` — Jira research
  that deferred this reorg (lines 1246-1283) and recommended pre-adding
  `paths.integrations` (research:1153-1159). Latter recommendation is
  partially executed: the key works via the generic delegate, the configure
  SKILL.md table documents it, but `config-read-path.sh`'s header comment
  doesn't list it yet.
- `meta/decisions/ADR-0018-init-skill-for-repository-bootstrap.md` —
  prior init-skill ADR; relevant only for `.claude/accelerator.local.md`
  gitignore rule (line 94).
- Several `meta/plans/2026-03-*` and `2026-04-*` files contain historical
  references to the old paths; these are completed-work artefacts and should
  not be retroactively rewritten.

## Related Research

- `meta/research/2026-04-29-jira-cloud-integration-skills.md` — Jira research
  that produced the state-path centralisation pattern this work item builds
  on.
- `meta/notes/2026-04-29-accelerator-config-state-reorg.md` — source note
  this work item promotes to story.

## Open Questions

1. **Root `.gitignore` rule format for the relocated config.** The work item
   says init must add a rule "covering `config.local.md`" (work item:96-98).
   If the user's project root `.gitignore` has the entry literally as
   `config.local.md` (no path prefix), it would also ignore an unrelated
   `config.local.md` anywhere in the project. The safer pattern is
   `/.accelerator/config.local.md` (anchored). Migration 0003 must use the
   same form when rewriting the existing `.claude/accelerator.local.md` rule.
   Recommend confirming the literal rule string before drafting the plan.

2. **`config.md` rename motivation.** The reorg renames `accelerator.md` →
   `config.md` (work item:68-69). Inside a `.accelerator/` tree this makes
   sense — `<plugin>/<plugin>.md` is redundant. But ADR-0016:170-172 cited
   `plugin-dev` ecosystem alignment as a rationale for the original name.
   The work item does not explicitly address this rationale. Worth a brief
   note in the plan or an ADR amendment confirming the rename is deliberate.

3. **`issuetypes/` directory.** Work item:85 lists `issuetypes/` in the
   target tree, and work item:287-290 references it in an acceptance
   criterion. But no current Jira-skill code creates this subdirectory.
   Either it's a forward-looking placeholder (in which case the AC should
   be removed or qualified), or it's expected to be created by a different
   skill outside the seven listed (which should be identified).

4. **`fields.json` / `projects.json` idempotency.** Work item:229-232
   requires that re-running `init-jira` on an existing state directory not
   overwrite `fields.json` or `projects.json`. Today they're overwritten on
   every discover/refresh run via `jira_atomic_write_json`. Implementing
   the AC requires changing the discover flow's contract — perhaps by
   gating the writes on a new flag, or by treating "discover" as
   non-idempotent and "verify-only" as idempotent. This is a separable
   concern from the path move.

5. **Backwards-compat with un-migrated repos.** The work item is a hard cut
   (work item:51-54). The discoverability hook should still detect
   un-migrated repos so they can be told to run `/accelerator:migrate`. A
   sensible detection rule: `.accelerator/` OR `.claude/accelerator.md` OR
   `meta/.migrations-applied` exist (covers post-migrate, pre-migrate, and
   partial-migrate states). The work item:165-171 says "test for
   `.accelerator/` instead" — this is too narrow if pre-migrate repos must
   still trigger.

6. **Visualiser `write-visualiser-config.sh:48`.** Default arg
   `meta/templates` should change to `.accelerator/templates` to stay
   consistent with the `SKILL.md:21` change. Confirm this is in scope.

7. **`paths.tmp` migration vs. user override.** The work item:202 says the
   move only happens "if `paths.tmp` is at the default value; if overridden,
   the custom path is left untouched". Confirm migration 0003 reads the
   effective value via `config-read-path.sh tmp` (no default) and compares
   against the new default `.accelerator/tmp` AND the old default `meta/tmp`
   to decide. Equivalent logic needed for `templates` (per ADR-0023's
   pinned-path preservation rule).

## Resolutions (2026-05-05 review)

The seven Open Questions above were worked through in a follow-up review on
2026-05-05. Resolutions:

**Q1 — Root `.gitignore` rule format.** **Resolved: use the anchored form
`.accelerator/config.local.md`.** Eliminates the unanchored-rule footgun where
an unrelated `config.local.md` elsewhere in the repo would be silently ignored.
Both `init` Step 3 and migration 0003 write this exact string. Migration 0003's
idempotency check defensively looks for either anchored or unanchored forms but
writes only the anchored form.

**Q2 — `config.md` rename motivation.** **Resolved: keep the rename.**
`.accelerator/config.md` is self-documenting; `.accelerator/accelerator.md`
would be redundant; the plugin-dev ecosystem-alignment rationale from
ADR-0016:170-172 no longer applies once we leave `.claude/`. **A full
superseding ADR will be created separately** to document the rename and the
new userspace configuration model.

**Q3 — `issuetypes/` directory.** **Resolved: drop from work item 0031.**
- Remove `issuetypes/` from the target-structure listing at work item:85.
- Remove `issuetypes/` from the AC at work item:287-290 (the AC then reads
  "`fields.json` and `projects.json` are present").
- Optionally add a note in "Drafting Notes" or "Assumptions" stating future
  Jira phases may add `issuetypes/` and other subdirectories under
  `.accelerator/state/integrations/jira/`; this work item makes no provision
  for them beyond the directory itself existing.

**Q4 — `fields.json` / `projects.json` idempotency.** **Resolved: AC was
misworded; `init-jira` keeps current refresh semantics.** No behaviour change
to `_jira_do_discover` or `_fields_do_refresh` — they continue to atomically
rewrite from the live tenant on every run. AC line 229-232 should be reworded:

> Given `init-jira` runs on a repo that already has
> `.accelerator/state/integrations/jira/`, then the `.gitignore` continues to
> ignore `site.json` and `.refresh-meta.json`; `fields.json` and
> `projects.json` are refreshed from the live tenant; and re-running against
> an unchanged tenant produces no `git status` diff for `fields.json` or
> `projects.json`.

Plus a new AC pinning `.lock/` lifecycle:

> Given `init-jira` completes successfully, then no `.lock/` directory remains
> under `.accelerator/state/integrations/jira/`.

**Update work item:108-111** (`init-jira` `.gitignore` requirement) to ignore
`site.json`, `.refresh-meta.json`, and `.lock/` — not just `site.json`. The
inner `.gitignore` written by both `init-jira` and migration 0003 carries
these three rules.

The "Recommended Plan Shape" section's unit 4 must be updated: drop the
"switch to skip-if-exists" item; the only `init-jira` behaviour changes are
relocating the `.gitignore` from project root to inside the state dir,
expanding the rule list, adding `.gitkeep`, and adding the absent-`.accelerator/`
warning.

**Q5 — Discoverability hook detection logic.** **Resolved: use a three-clause
OR.** Replace `migrate-discoverability.sh:14-18` with a check that triggers if
ANY of `.accelerator/`, `.claude/accelerator.md`, or `meta/` exists. This
preserves pre-migration detection (the case for which the migration warning
matters most). Update work item:165-171 wording from "test for `.accelerator/`
instead" to "test for `.accelerator/` in addition to existing sentinels".

State-file lookup at `migrate-discoverability.sh:22` becomes a fallback chain:
read `.accelerator/state/migrations-applied` if `.accelerator/` exists, else
fall back to `meta/.migrations-applied`. This fallback is permanent (kept
forever) — it's a discoverability concern, not a runtime concern, and is
compatible with the work item's hard-cut posture for runtime scripts.

**Q6 — Visualiser `write-visualiser-config.sh:48`.** **Resolved: in scope.**
Add this line to the work item's Visualiser section at lines 148-156:

> - `skills/visualisation/visualise/scripts/write-visualiser-config.sh:48` —
>   default arg `meta/templates` → `.accelerator/templates`

Add a note in "Drafting Notes" or "Assumptions" stating that the other nine
`abs_path` calls at `write-visualiser-config.sh:38-46` are deliberately
unchanged because their targets remain in `meta/`.

**Q7 — `paths.tmp` migration vs. user override.** **Resolved: simpler
alternative.** Limit pinned-path preservation to `paths.tmp` only. For
`paths.templates` and `paths.integrations`, do the unconditional move during
migration 0003 — these keys have negligible explicit-override population in
practice, and the pinned-path detection logic is non-trivial in pure bash
3.2. Document the trade-off: users who explicitly pinned `paths.templates` or
`paths.integrations` to `meta/<dir>` will need to update their config
post-migration.

This means:
- Work item:201-203 stays as-is (only `paths.tmp` gets the conditional treatment).
- Work item:198 (the unconditional `meta/templates/` → `.accelerator/templates/`
  line) stays as-is.
- The `meta/integrations/jira/` → `.accelerator/state/integrations/jira/` move
  at work item:200 stays as-is (unconditional).
- Add an "Assumptions" note: "Pinned-path preservation per ADR-0023 is applied
  only to `paths.tmp` in this work item. `paths.templates` and
  `paths.integrations` are moved unconditionally; users who have explicitly
  pinned either to a `meta/<dir>` value will need to update their config
  post-migration."

## Recommended Plan Shape

Based on the above, the implementation work decomposes into four roughly
independent units that can land in separate PRs (with the migration unit
landing last):

1. **Path-config additions** — `config-read-path.sh` doc comment update for
   `integrations`; configure SKILL.md table update for `integrations`
   default. Trivial.

2. **Source-of-truth defaults** — update default args at every
   `config-read-path.sh` call site for `templates` and `tmp` (init,
   visualiser scripts, `config-common.sh:176`, `config-summary.sh:19`,
   `write-visualiser-config.sh:48`). Update Jira `jira-common.sh:62` to use
   the new `integrations` default. Update hardcoded `.claude/accelerator/...`
   strings in `config-common.sh:25-26`,
   `config-read-skill-{context,instructions}.sh`, `config-summary.sh:91, 114`,
   `config-read-review.sh:220`, `config-dump.sh:23-24, 98-99`.

3. **Init skill restructure** — split Step 1 into core `.accelerator/`
   scaffold creation plus the existing 12-path iteration for `meta/`
   directories (excluding `templates` and `tmp`); update Step 3's
   `.gitignore` rule.

4. **Init-jira behaviour changes** — relocate `.gitignore` from project
   root to inside the state dir; change the rule list to `site.json`; add
   `.gitkeep`; add `.accelerator/`-absence stderr warning; switch
   `fields.json`/`projects.json` write semantics to skip-if-exists for the
   AC's idempotency requirement.

5. **Migration 0003 + framework updates** — atomic delivery of
   `0003-relocate-accelerator-state.sh`, `run-migrations.sh` updates
   (state-file paths + clean-tree regex), and `migrate-discoverability.sh`
   updates (sentinel detection + state-file path + warning string).

6. **Documentation** — README, configure SKILL.md throughout, init-jira
   SKILL.md prose, visualiser SKILL.md prose. Mostly mechanical.

Units 1-2 are safe to land independently (config-read-path.sh remains a
pass-through, default args are caller-supplied). Unit 5 is gated on the
release of all other units because it executes the move on every
post-update repo. Units 3 and 4 can land before unit 5 as long as their
behaviour is conditional on `.accelerator/` existing (otherwise they would
break un-migrated repos — see Open Question 5).
