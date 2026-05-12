---
date: "2026-04-25T22:30:00+01:00"
type: plan
skill: create-plan
ticket: ""
status: draft
---

# Rename `tickets` → `work` and `ticket` → `work-item` (with Migration Skill) Implementation Plan

## Overview

Replace the loaded service-desk term "ticket" with the neutral
product-development term "work-item" throughout the plugin. The skill
category becomes `work`, individual skills become
`/accelerator:create-work-item`, `/accelerator:extract-work-items`, etc.,
the default storage directory becomes `meta/work/`, and the canonical
template becomes `templates/work-item.md` with `work_item_id:` in the
frontmatter.

A new `/accelerator:migrate` skill is introduced alongside the rename to
apply ordered, idempotent migrations to consumer codebases. Its first
registered migration performs the rename in a user's `meta/` directory
and `.claude/accelerator*.md` config files. The skill is destructive by
default — it operates on version-controlled repositories and the user
can revert via VCS if needed.

This plan executes the rename via test-driven development: each rename
phase begins by updating tests and golden fixtures to assert the new
target names (turning the suite red), then performs the rename to turn
the suite green. The migration skill (Phase 4) is built test-first
against isolated fixtures.

## Current State Analysis

The plugin's `tickets` category exists across nine layers:

1. **Plugin manifest** (`.claude-plugin/plugin.json:16`) lists
   `./skills/tickets/` as a category directory.
2. **Skills directory** `skills/tickets/` contains 7 SKILL.md files,
   6 helper scripts under `skills/tickets/scripts/`, and per-skill
   eval suites under `skills/tickets/<skill>/evals/`.
3. **Configuration system** (`scripts/config-read-path.sh`,
   `scripts/config-dump.sh`) registers `paths.tickets` (default
   `meta/tickets`) and `paths.review_tickets` (default
   `meta/reviews/tickets`). Defaults are not centralised — each call
   site repeats the default literal.
4. **Review subsystem** (`scripts/config-read-review.sh`) accepts
   `ticket` as a mode literal, defines `BUILTIN_TICKET_LENSES`,
   `DEFAULT_TICKET_REVISE_SEVERITY`, `DEFAULT_TICKET_REVISE_MAJOR_COUNT`,
   `DEFAULT_MIN_LENSES_TICKET`, and reads config keys
   `review.ticket_revise_severity`, `review.ticket_revise_major_count`,
   `review.min_lenses_ticket`.
5. **Review lenses** (`skills/review/lenses/{clarity,completeness,
   dependency,scope,testability}-lens/SKILL.md`) declare
   `applies_to: [ticket]` and use "ticket" prose. Lens directory names
   themselves do not contain "ticket".
6. **Review output format** at
   `skills/review/output-formats/ticket-review-output-format/SKILL.md`,
   with `review-ticket/SKILL.md` stamping artifacts as
   `type: ticket-review`.
7. **Template** at `templates/ticket.md` with frontmatter field
   `ticket_id:` (referenced as a literal lookup key `ticket` in
   `skills/tickets/scripts/ticket-template-field-hints.sh:53` via
   `scripts/config-read-template.sh`).
8. **Test infrastructure** — `mise run test` aggregates format checks
   (hierarchy, toc-link, evals, skill, frontmatter, allowed-tools,
   cross-references), the per-script test
   `skills/tickets/scripts/test-ticket-scripts.sh`, and per-skill
   evals (`skills/tickets/<skill>/evals/{evals,benchmark}.json`). Eval
   JSON files reference fixture filenames like `ticket.md` as literal
   strings in `prompt:` and `files:` arrays.
9. **Documentation** — `README.md` (lines 86, 184, 244-273, 315, 337,
   350), `CHANGELOG.md` Unreleased section (lines 5-98),
   `skills/config/configure/SKILL.md` (~12 reference points),
   `skills/config/init/SKILL.md`, `skills/planning/create-plan/SKILL.md`.

There is no migration framework today. Schema/state versioning is
absent. Prior breaking changes used either a clean rename (v1.9.0
`initialise` → `init`, no shim, since the surface was unreleased) or a
manual instruction in CHANGELOG (v1.8.0 ephemeral-file move with an
`rm -rf` command).

This repository itself holds 29 work-item files under `meta/tickets/`.
Per the v1.9.0 precedent, since the ticket category sits in
CHANGELOG's Unreleased section, the rename can ship as a clean break.
The migration skill exists to automate the user-side rewrite that the
clean break would otherwise leave to manual `mv` and `sed`.

### Key Discoveries:

- `scripts/config-read-review.sh:15` — mode literal `"ticket"`; rename
  to `"work-item"`. Same file: `:28-31` constants, `:53-60` lens array,
  `:65-66`/`:73-74` mode branches, `:85-86` config-key reads,
  `:144` valid_modes string, `:206-209` validate calls, `:414-418`
  default core lenses block, `:450-473` lens-subset note,
  `:501-503` output labels, `:556-571` verdict block.
- `scripts/config-dump.sh` — `PATH_KEYS` and `PATH_DEFAULTS` arrays
  span `:174-200` (research's `:182-198` was the ticket-specific
  subset). Two ticket entries: `paths.review_tickets`/`meta/reviews/
  tickets` at `:182`/`:196`; `paths.tickets`/`meta/tickets` at
  `:184`/`:198`.
- `templates/ticket.md` frontmatter has nine fields. Only `ticket_id`
  needs renaming. The `parent` field's comment ("ticket number of the
  parent epic/story") needs prose update. All other field names stay
  (none contain "ticket").
- `scripts/test-hierarchy-format.sh:21-22` — defaults to
  `skills/tickets/list-tickets/SKILL.md` and
  `skills/tickets/refine-ticket/SKILL.md`. Asserts byte-exact equality
  between `<!-- canonical-tree-fence -->` blocks in the two files.
- Eval fixture filenames are bound as literal strings in `evals.json`
  (verified in `skills/tickets/refine-ticket/evals/evals.json` —
  `ticket.md` appears in `prompt` strings and `files:` arrays).
  Renaming a fixture filename mandates JSON edits in the same commit.
- `.claude-plugin/plugin.json` is a 21-line file with a single
  ticket-related path at `:16`. No other category-path entries.
- No migration framework exists. No `meta/.state`,
  `meta/.migrations`, or schema-version key anywhere.
- `skills/tickets/scripts/test-ticket-scripts.sh` exists as the
  per-script test suite — pattern to follow when adding a migration
  skill test.
- Slash-command pluralisation precedent is mixed: `extract-tickets`
  (plural — bulk op), `create-ticket`/`refine-ticket`/`update-ticket`/
  `review-ticket`/`stress-test-ticket` (singular — per-item op). New
  names follow the same rule:
  `/accelerator:extract-work-items`, `/accelerator:list-work-items`
  (plural); `/accelerator:create-work-item`,
  `/accelerator:refine-work-item`, `/accelerator:update-work-item`,
  `/accelerator:review-work-item`,
  `/accelerator:stress-test-work-item` (singular).

## Desired End State

After this plan completes:

- `mise run test` is fully green with all assertions referring to the
  new names.
- `.claude-plugin/plugin.json:16` reads `./skills/work/`.
- `skills/work/` contains 7 renamed skills and 6 renamed helper
  scripts. No "ticket" literal appears in any SKILL.md, helper script,
  or eval fixture under `skills/work/`.
- `scripts/config-read-path.sh` and `scripts/config-dump.sh` recognise
  `paths.work` (default `meta/work`) and `paths.review_work` (default
  `meta/reviews/work`); `paths.tickets`/`paths.review_tickets` are
  removed entirely (no shim).
- `scripts/config-read-review.sh` accepts mode `work-item` (not
  `ticket`); constants are `BUILTIN_WORK_ITEM_LENSES`,
  `DEFAULT_WORK_ITEM_REVISE_SEVERITY`, etc.; config keys are
  `review.work_item_revise_severity`,
  `review.work_item_revise_major_count`, `review.min_lenses_work_item`.
- `templates/work-item.md` exists with frontmatter `work_item_id:`.
- `skills/review/output-formats/work-item-review-output-format/SKILL.md`
  exists. Lens `applies_to:` lists `[work-item]`.
- `skills/config/migrate/SKILL.md` exists with one registered
  migration `0001-rename-tickets-to-work.sh`. Running
  `/accelerator:migrate` against a user's repo applies pending
  migrations idempotently, refuses to run on a dirty working tree
  (with `ACCELERATOR_MIGRATE_FORCE` override), prints a one-line
  preview per pending migration, and records applied IDs in
  `meta/.migrations-applied`. Each migration script begins with a
  `# DESCRIPTION:` comment consumed by the driver.
- A SessionStart hook (`hooks/migrate-discoverability.sh`) compares
  the highest applied migration ID against the highest available
  ID in the bundled registry and emits a one-line warning to
  stderr when the repo is behind, pointing at
  `/accelerator:migrate`.
- This repository's own work-item directory has been migrated:
  `meta/tickets/` → `meta/work/` with `ticket_id:` rewritten to
  `work_item_id:` in all ~29 files (count re-verified during
  Phase 5 §5.1); `meta/reviews/tickets/` → `meta/reviews/work/`;
  `meta/.migrations-applied` contains `0001-rename-tickets-to-work`.
  Body content of every migrated file is byte-identical to its
  pre-migration state (verified via Phase 5's content-hash
  regression check).
- Two new ADRs are recorded: terminology choice (`work` /
  `work-item`, including typing-friction tradeoff) and migration
  framework design (clean-tree pre-flight, preview, pinned-path
  preservation, downgrade tolerance).
- `CHANGELOG.md` Unreleased section reflects the rename with two
  bullets (breaking-change announcement + upgrade procedure).
  Version is bumped to `1.19.0` on release.
- A follow-up work-item under `meta/work/` tracks the
  centralisation of `PATH_DEFAULTS` to
  `scripts/config-defaults.sh`.
- The `jj log` history is structured as roughly: ADRs change → Phase
  1 change → Phase 2 change → Phase 3 change → Phase 4 change →
  **separate** Phase 5 self-apply change → release change. Each
  phase lands as a single atomic transition (RED+GREEN squashed).

## What We're NOT Doing

- **No backwards-compatibility shim** for old config keys, old slash
  commands, old template name, or old `ticket_id:` frontmatter field.
  This is a clean break (v1.9.0 precedent applies — surface is
  unreleased per CHANGELOG).
- **No retroactive edits to historical research/plan/decision/review
  documents** under `meta/research/codebase/`, `meta/plans/`, `meta/decisions/`,
  `meta/reviews/`, except where their cross-references are now broken
  paths and only to fix those broken paths. Prose stays as-written.
- **No retroactive renames in CHANGELOG entries for already-released
  versions**. The Unreleased section is rewritten; v1.18.0 and prior
  entries are immutable.
- **No centralisation of the configuration default-map** despite this
  rename touching 15 default literals. That is a separate cleanup.
- **No type-enum changes** in the template (`story | epic | task |
  bug | spike` stays — none are "ticket").
- **No lens directory renames** (`clarity-lens` etc. — names don't
  contain "ticket").
- **No filename renames** within `meta/tickets/` for files that match
  `NNNN-kebab-slug.md` (no filename literally contains "ticket"). The
  migration only renames the directory.
- **No multi-migration framework features** beyond what the rename
  needs: the registry is a sorted glob, the state file is
  newline-delimited, no rollback support. Per-migration metadata
  is limited to a single `# DESCRIPTION:` comment line consumed by
  the driver for preview output.
- **No dry-run flag** for the migrate skill. The clean-tree
  pre-flight check plus the preview-before-apply step provide the
  same user protection without an extra flag, matching the
  `init` skill's report-then-act idiom.
- **No new release infrastructure** — version bump and CHANGELOG
  follow existing v1.x precedents.
- **No centralisation of `PATH_DEFAULTS`** in this plan, but a
  follow-up work-item (filed as part of Phase 6) tracks the
  extraction of `scripts/config-defaults.sh` so future migrations
  don't re-pay the same 15-site coordination cost.

## Implementation Approach

Six phases plus an ADR phase.

- **Phase 0** writes two ADRs documenting decisions made during
  research (terminology, migration framework). No code changes.
- **Phases 1–3** rename the existing surface. Each phase follows a
  strict TDD red→green sequence: update tests/fixtures to expect the
  new names → confirm suite is red (with named expected-failing
  tests) → perform the rename → confirm suite is green. Each phase
  ends with an explicit enumeration of which `mise run test:*`
  subsets must be green and which may remain red until later
  phases.
- **Phase 4** builds the migration skill test-first against isolated
  shell fixtures. The first migration (`0001-rename-tickets-to-work`)
  is included. Phase 4 also adds a SessionStart hook that warns
  when the repo's `meta/.migrations-applied` lags the plugin's
  bundled migrations.
- **Phase 5** self-applies that migration to this repository in a
  separate jj change so it can be reverted independently of the
  framework build. A pre-migration jj bookmark and content-hash
  snapshot guard against any regression Phase 4's tests missed.
- **Phase 6** updates CHANGELOG, bumps the version, and files a
  follow-up work-item to centralise `PATH_DEFAULTS`.

The order is critical for one reason: Phase 5 depends on Phase 4 (the
skill must exist before self-apply). All other phase orderings are
independent in principle, but the listed order minimises rebase pain
because Phase 1 renames touch the largest surface (helper script
paths, plugin manifest) that later phases reference. Phase 1
deliberately stops short of touching the template and its lookup
wiring; those move atomically in Phase 3 to avoid an intermediate
state where the helper script and the recognised template-key list
disagree.

---

## Phase 0: ADRs

### Overview

Capture two terminology and design decisions in ADR form. No code
changes; this phase exists so that subsequent phases reference
authoritative decisions.

### Changes Required:

#### 1. ADR — Use `work`/`work-item` terminology

**File**: `meta/decisions/ADR-XXXX-work-item-terminology.md` (next
sequential number; check `meta/decisions/` to determine)

**Content**:
- Context: original ticket research
  (`meta/research/codebase/2026-04-08-ticket-management-skills.md`) introduced
  `ticket` without a documented alternatives review.
- Decision: rename to `work` (category) and `work-item` (item type).
- Consequences: removes service-desk connotation; aligns with
  product-development vocabulary; storage directory and config keys
  match the category name (`paths.work`).
- Alternatives considered (and rejected): `task` (collides with
  template enum value `task`), `story` (same — it's an enum value),
  `backlog-item` (verbose), `issue` (collides with GitHub issues
  vocabulary), `item` / `card` (too generic — would collide with
  template `type` enum or kanban-tool vocabulary), retain `ticket`
  (rejected — service-desk slant).
- Tradeoff acknowledged: `work-item` adds five characters and a
  hyphen to every slash command versus `ticket`. The neutrality
  gain (vocabulary independent of Jira/Linear/GitHub Issues
  service-desk framing, plus alignment of category name `work`
  with config keys `paths.work`) is judged to outweigh the typing
  cost given the slash commands are auto-completed and read more
  often than typed.

#### 2. ADR — Migration framework for the meta directory

**File**: `meta/decisions/ADR-XXXX-meta-directory-migration-framework.md`

**Content**:
- Context: pre-release breaking changes (v1.9.0 `initialise`→`init`,
  this rename) accumulate manual upgrade instructions in CHANGELOG.
- Decision: introduce `/accelerator:migrate` with an ordered,
  idempotent shell migration registry under
  `skills/config/migrate/migrations/` and a state file at
  `meta/.migrations-applied`.
- Design contract:
  - Migration ID = filename without extension, prefixed
    `[0-9][0-9][0-9][0-9]-`.
  - Each migration script begins with a `# DESCRIPTION:` comment on
    line 2 that the driver `grep`s once for preview output and
    end-of-run summaries.
  - State file is newline-delimited migration IDs that have been
    applied. Unknown IDs in the state file (e.g. from a downgrade)
    are preserved on rewrite and warned about, never deleted.
  - Each migration script is independently idempotent (state file is
    a belt; per-script idempotency is the suspenders). Each migration
    must self-detect no-op conditions and exit 0 on already-applied
    state.
  - Migrations must rewrite plugin-level expectations, NOT user
    intent: a user-pinned config value (e.g. `paths.tickets:
    meta/custom-tix`) has its KEY rewritten (`paths.work:
    meta/custom-tix`) and its directory left where the user put it.
    Directory renames apply only when the resolved path is the
    plugin default.
  - Default behaviour is destructive but guarded: before mutating,
    the driver verifies a clean working tree (jj/git status reports
    no uncommitted changes in `meta/` or `.claude/accelerator*.md`)
    and prints a one-line preview per pending migration. A
    `--force` env var (`ACCELERATOR_MIGRATE_FORCE=1`) overrides the
    clean-tree check for advanced users.
  - On per-migration failure the skill aborts the entire run; the
    state file does NOT receive an entry for the failed migration.
    Already-applied entries from prior successful migrations are
    preserved.
  - File-mutating operations (config rewrites, frontmatter sed)
    write to a temp file and rename, so partial writes cannot
    corrupt the original.
- Discoverability: a SessionStart hook compares the highest applied
  migration ID in `meta/.migrations-applied` against the highest
  available migration ID in the plugin's bundled
  `migrations/` directory and emits a one-line warning when the
  repo is behind, pointing the user at `/accelerator:migrate`.
- Consequences: future restructures land as small migration scripts
  rather than CHANGELOG instructions; users get a clear breadcrumb
  when their repo's schema lags behind the installed plugin.
- Alternatives considered (and rejected):
  - No migration (manual CHANGELOG step — error-prone for users
    with pinned config).
  - Python/JS migration runner (out of step with the plugin's
    shell conventions).
  - Schema-version field in `meta/.config` (bigger surface than
    needed for the actual migration cadence).
  - Dry-run-by-default with `--apply` (the research's original
    proposal) — rejected in favour of clean-tree pre-flight +
    preview, which gives the same user-protection without the
    extra flag and matches the `init` skill's report-then-act
    idiom.
  - Rollback support (per-migration undo scripts) — rejected as
    redundant with VCS revert and a maintenance multiplier.

### Success Criteria:

#### Automated Verification:

- [x] ADR files match the plugin's ADR format:
      `mise run test:format` passes (it includes ADR frontmatter and
      cross-reference checks)
- [x] ADR numbering is contiguous: `ls meta/decisions/ADR-*` shows no
      gaps
- [x] Both ADRs are listed in `README.md`'s ADR index (if one
      exists) — verify with `grep -c "ADR-XXXX" README.md`

#### Manual Verification:

- [x] Rationale reads coherently and matches research findings
- [x] Alternatives sections are non-trivial (multiple rejected
      candidates each)

---

## Phase 1: Plugin manifest, config defaults, helper scripts

### Overview

Rename the `tickets` category to `work`, the seven skills inside it,
the six helper scripts, the plugin-manifest entry, the path-config
keys and defaults (`paths.tickets`/`paths.review_tickets` →
`paths.work`/`paths.review_work`), and update the 15 call sites that
duplicate the path defaults. This is the largest single rename phase
by file count.

This phase does NOT touch:
- The review subsystem (`config-read-review.sh`, lenses,
  output-format) — Phase 2.
- The template, the template lookup wiring (including
  `scripts/config-read-template.sh`'s recognised-keys list and
  `work-item-template-field-hints.sh:53`), eval fixtures, or
  prose inside SKILL.md bodies — Phase 3. These are deliberately
  deferred so the template rename and every reference to it move
  in lockstep within a single phase, with no intermediate state
  where the helper script and the recognised-keys list disagree.
- This repository's own `meta/tickets/` data — Phase 5.

### Changes Required (Step 1 — RED: update tests and golden fixtures first):

#### 1.1 Update `scripts/test-hierarchy-format.sh:21-22`

**Change**: defaults reference the new paths.

```bash
FILE_A="${1:-$REPO/skills/work/list-work-items/SKILL.md}"
FILE_B="${2:-$REPO/skills/work/refine-work-item/SKILL.md}"
```

Run `mise run test:format` — expected to fail because
`skills/work/...` doesn't exist yet.

#### 1.2 Update `skills/tickets/scripts/test-ticket-scripts.sh`

This file becomes `skills/work/scripts/test-work-item-scripts.sh` in
Step 2. For Step 1 (still under the old path), update its assertions
and helper-script invocations to expect the new script names
(`work-item-next-number.sh`, `work-item-read-field.sh`,
`work-item-read-status.sh`, `work-item-update-tags.sh`,
`work-item-template-field-hints.sh`).

Run `mise run test:tickets` — expected to fail because the new
scripts don't exist yet.

#### 1.3 Update `scripts/test-cross-references.sh`

If it asserts on paths like `skills/tickets/...`, update them to
`skills/work/...`. (Read the file in Step 1 to identify exact
references.)

#### 1.4 Update format-test fixtures

Any golden file under `scripts/test-fixtures/` referencing
`skills/tickets/` or `tickets:` config keys is updated to the new
names. Identify by:

```bash
rg -l "tickets|ticket_id|paths\.tickets" scripts/test-fixtures/
```

Apply the same rename to each match.

#### 1.5 Confirm RED state

Run `mise run test:format` and `mise run test:tickets`. Expected
failures (record the exact failing test names and error substrings
in the jj change description so a later bisect can confirm RED for
the right reason):

- `scripts/test-hierarchy-format.sh` — fails with file-not-found
  on `skills/work/list-work-items/SKILL.md` and
  `skills/work/refine-work-item/SKILL.md`.
- The test under `skills/tickets/scripts/test-ticket-scripts.sh`
  invoked by `mise run test:tickets` — fails because the renamed
  helper script names (`work-item-next-number.sh`, etc.) do not
  resolve.
- Any format-test fixture under `scripts/test-fixtures/` updated
  in §1.4 — fails byte-equality against the live source.

`mise run test:skills`, `mise run test:cross-references`,
`mise run test:evals` are NOT expected to be green at this
checkpoint and should be skipped during RED verification (they
will be addressed at the close of Phase 1's GREEN step or in
later phases — see §1.16).

**Commit this state as a single jj change** — the failing test
suite is the spec for the next step. Squash this RED change into
the GREEN change at the end of §1.16 so the phase lands in jj log
as one atomic transition.

### Changes Required (Step 2 — GREEN: perform the rename):

#### 1.6 Rename the skills directory and subdirectories

```bash
jj mv skills/tickets skills/work
jj mv skills/work/create-ticket skills/work/create-work-item
jj mv skills/work/extract-tickets skills/work/extract-work-items
jj mv skills/work/list-tickets skills/work/list-work-items
jj mv skills/work/refine-ticket skills/work/refine-work-item
jj mv skills/work/review-ticket skills/work/review-work-item
jj mv skills/work/stress-test-ticket skills/work/stress-test-work-item
jj mv skills/work/update-ticket skills/work/update-work-item
```

(If `jj mv` is unavailable, plain `mv` works — jj tracks the rename
on next `jj status`.)

#### 1.7 Rename the helper scripts

```bash
cd skills/work/scripts
jj mv ticket-next-number.sh work-item-next-number.sh
jj mv ticket-read-field.sh work-item-read-field.sh
jj mv ticket-read-status.sh work-item-read-status.sh
jj mv ticket-update-tags.sh work-item-update-tags.sh
jj mv ticket-template-field-hints.sh work-item-template-field-hints.sh
jj mv test-ticket-scripts.sh test-work-item-scripts.sh
```

#### 1.8 Update each renamed SKILL.md

For each of the 7 skills, edit:
- `name:` frontmatter line — replace ticket → work-item.
- `description:` line — rewrite prose using "work item" / "work
  items".
- `allowed-tools:` line — replace
  `${CLAUDE_PLUGIN_ROOT}/skills/tickets/scripts/*` with
  `${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/*`.

(Body prose is updated in Phase 3; this phase touches only
machine-readable frontmatter and `allowed-tools` paths.)

#### 1.9 Update each renamed helper script

Inside each helper script, replace internal "ticket" string
references and error messages with "work-item". Specific edits per
research:
- `work-item-next-number.sh:4-9, :35-47, :55-67`
- `work-item-read-field.sh:4, :8, :15, :20-44, :70`
- `work-item-update-tags.sh:4-12, :14, :19, :42-73`
- `work-item-template-field-hints.sh:4-7, :11, :15, :19` —
  internal strings only. **Leave line 53 (`config-read-template.sh
  ticket`) unchanged in Phase 1**: that line and the template
  itself move together in Phase 3 as one atomic edit. The script
  remains fully functional in this phase because Phase 1 does not
  modify `config-read-template.sh`'s recognised-keys behaviour
  (see §1.14 — that doc-comment update is also deferred to
  Phase 3).

#### 1.10 Update the plugin manifest

**File**: `.claude-plugin/plugin.json:16`

```json
"./skills/work/"
```

#### 1.11 Update `scripts/config-read-path.sh:17`

The doc-comment block listing recognised keys gains `paths.work` /
`paths.review_work` and loses `paths.tickets` / `paths.review_tickets`.

#### 1.12 Update `scripts/config-dump.sh:174-200`

`PATH_KEYS` array: `"paths.tickets"` → `"paths.work"`;
`"paths.review_tickets"` → `"paths.review_work"`.
`PATH_DEFAULTS` array: `"meta/tickets"` → `"meta/work"`;
`"meta/reviews/tickets"` → `"meta/reviews/work"`.

#### 1.13 Update every `config-read-path.sh tickets meta/tickets` call site

15 locations enumerated in research Section 2:

1. `scripts/config-read-path.sh:17` — already covered (1.11)
2-4. `scripts/config-dump.sh:182,184,196,198` — already covered
   (1.12)
5. `skills/work/create-work-item/SKILL.md:22`
6. `skills/work/list-work-items/SKILL.md:23`
7. `skills/work/extract-work-items/SKILL.md:25-27`
8. `skills/work/update-work-item/SKILL.md:24`
9. `skills/work/refine-work-item/SKILL.md:24`
10. `skills/work/review-work-item/SKILL.md:25-26`
11. `skills/work/stress-test-work-item/SKILL.md:24`
12. `skills/work/scripts/work-item-next-number.sh:35`
13. `skills/planning/create-plan/SKILL.md:23`
14. `skills/config/init/SKILL.md:27, :29`
15. `skills/config/configure/SKILL.md:397, :415`

(Plus README and CHANGELOG — Phase 3.)

In each: replace `config-read-path.sh tickets meta/tickets` with
`config-read-path.sh work meta/work` and the `review_tickets` /
`meta/reviews/tickets` equivalent with `review_work` /
`meta/reviews/work`.

#### 1.14 (Deferred to Phase 3)

`scripts/config-read-template.sh:6` doc-comment update moves to
Phase 3 alongside the template file rename and the
`work-item-template-field-hints.sh:53` call site update — see
Phase 3 §3.4. Keeping these together avoids an intermediate state
where the doc-comment, the recognition logic, and the helper
script's call all disagree.

#### 1.15 Update mise task wiring

The mise task that wires `mise run test:tickets` references the
old helper-test path. Read the actual mise task definition (`mise
tasks` to list, then `cat mise/tasks/<file>` or
`mise task info test:tickets` to find the source) and update it
to:

- Rename the task: `test:tickets` → `test:work-items`.
- Re-point the script path:
  `skills/tickets/scripts/test-ticket-scripts.sh` →
  `skills/work/scripts/test-work-item-scripts.sh`.
- Update any aggregate task (e.g. `test`) that references
  `test:tickets` to use the new task name.

Verify with `mise tasks | grep work-items` after the edit.

#### 1.16 Confirm GREEN and squash

Run `mise run test:format`, `mise run test:work-items`, and
`mise run test:skills`. Expected: all green. Squash the §1.5 RED
jj change into this GREEN change so the phase lands as a single
atomic transition in jj log.

### Success Criteria:

#### Phase boundary — test-subset green/red expectations:

| Subset | End of Phase 1 |
|---|---|
| `test:format` | GREEN |
| `test:work-items` (formerly `test:tickets`) | GREEN |
| `test:skills` | GREEN |
| `test:cross-references` | MAY BE RED — Phase 2/3 surfaces still hold ticket strings |
| `test:evals` | MAY BE RED — fixture rename is Phase 3 |
| `test` (aggregate) | MAY BE RED for the above reasons |

#### Automated Verification:

- [x] `mise run test:format` passes
- [x] `mise run test:work-items` passes (new task name; tests
      renamed helper scripts)
- [x] `mise run test:skills` passes (no SKILL.md frontmatter
      regressions)
- [x] `bash -n` is clean for every renamed script:
      `find skills/work/scripts -name '*.sh' -exec bash -n {} +`
- [x] No "ticket" literal remains in `.claude-plugin/plugin.json`,
      `scripts/config-read-path.sh`, `scripts/config-dump.sh`, or
      `skills/work/`'s SKILL.md frontmatter / allowed-tools
      lines: `rg -l '\bticket' .claude-plugin/
      scripts/config-read-path.sh scripts/config-dump.sh
      skills/work/*/SKILL.md` returns nothing.
      (`scripts/config-read-template.sh` still mentions `ticket`
      in its doc-comment until Phase 3 §3.4 — this is by design,
      see §1.14. `config-dump.sh` still has `templates.ticket` which
      is the template key — deferred to Phase 3.)
- [x] `mise tasks | grep -E '^test:work-items'` shows the renamed
      task; no `test:tickets` task remains. (N/A — test runner is
      `invoke test.integration`; reference in tasks/test.py updated
      to skills/work/scripts/test-work-item-scripts.sh.)

#### Manual Verification:

- [ ] Run `/accelerator:list-work-items` in a sandbox repo with
      `meta/work/` containing one work item; output looks correct
- [ ] Run `/accelerator:create-work-item` and verify it creates a
      file under `meta/work/` (not `meta/tickets/`)
- [ ] Confirm renamed skills appear under their new slash commands
      (e.g. via the Claude Code skill picker)

---

## Phase 2: Review subsystem

### Overview

Update `scripts/config-read-review.sh` to use `work-item` mode and
rename its constants/keys; rename the output-format skill from
`ticket-review-output-format` to `work-item-review-output-format`;
update the five review-lens SKILL.md files (`applies_to:`,
`description:`, body prose); update the artifact `type:` stamp in
`review-work-item/SKILL.md`.

Lens directories themselves (`clarity-lens`, etc.) are NOT renamed —
the names contain no "ticket" literal.

### Changes Required (Step 1 — RED):

#### 2.1 Rename the golden test fixture

**File**: `scripts/test-fixtures/config-read-review/ticket-mode-golden.txt`
→ `work-item-mode-golden.txt`

Author the expected file by hand based on the existing fixture's
structure, with the new mode name, lens-array variable, and renamed
config keys / output labels. **Do NOT regenerate the golden file
from the script's actual output** — that turns the test into a
self-confirming snapshot. If GREEN-state output disagrees with the
hand-authored file, fix the script, not the fixture.

#### 2.2 Update the test runner that consumes the fixture

If `scripts/test-config.sh` (or wherever the review-mode test lives)
references `ticket-mode-golden.txt` or runs
`config-read-review.sh ticket`, update those literals to
`work-item-mode-golden.txt` and `config-read-review.sh work-item`.

#### 2.3 Confirm RED

Run the targeted review-mode test (whichever `mise run test:*`
subset wires `scripts/test-config.sh`, or the test runner directly
if no per-subset task exists). Expected failure: the new fixture
file refers to a mode (`work-item`) that `config-read-review.sh`
does not yet recognise; the script exits with the "unknown mode"
error and the byte-equality assertion against the golden file
fails. Record the failing assertion's line number in the jj change
description.

`test:format`, `test:work-items`, `test:skills` MUST stay green at
this checkpoint (they did at end of Phase 1; no Phase 2 RED step
should regress them). Commit this state. Squash the RED change
into the GREEN change at the end of §2.7 so the phase lands
atomically.

### Changes Required (Step 2 — GREEN):

#### 2.4 Rename `scripts/config-read-review.sh` symbols

| Current (line) | New |
|---|---|
| Mode literal `"ticket"` (`:15, :65, :73, :144, :414, :556`) | `"work-item"` |
| `DEFAULT_TICKET_REVISE_SEVERITY` (`:28`) | `DEFAULT_WORK_ITEM_REVISE_SEVERITY` |
| `DEFAULT_TICKET_REVISE_MAJOR_COUNT` (`:29`) | `DEFAULT_WORK_ITEM_REVISE_MAJOR_COUNT` |
| `DEFAULT_MIN_LENSES_TICKET` (`:31, :74`) | `DEFAULT_MIN_LENSES_WORK_ITEM` |
| `BUILTIN_TICKET_LENSES` (`:53, :66, :450, :454`) | `BUILTIN_WORK_ITEM_LENSES` |
| Config key `review.ticket_revise_severity` (`:85, :209, :414, :501`) | `review.work_item_revise_severity` |
| Config key `review.ticket_revise_major_count` (`:86, :206, :502`) | `review.work_item_revise_major_count` |
| Config key `review.min_lenses_ticket` | `review.min_lenses_work_item` |
| Var `ticket_verdict_changed` (`:556`) | `work_item_verdict_changed` |
| Var `ticket_revise_severity` (in verdict block) | `work_item_revise_severity` |
| Var `ticket_revise_major_count` (in verdict block) | `work_item_revise_major_count` |
| Output label "ticket revise severity" / "ticket revise major count" (`:501-503`) | "work-item revise severity" / "work-item revise major count" |
| Note "built-in ticket lens(es)…" (`:454`) | "built-in work-item lens(es)…" |
| Verdict text "REVISE when…" (`:556-571` ticket-specific phrasing) | Same intent, replace "ticket" → "work-item" |
| Usage string `"<pr|plan|ticket>"` (`:16`) | `"<pr|plan|work-item>"` |

#### 2.5 Update lens SKILL.md files

For each of `skills/review/lenses/{clarity,completeness,dependency,
scope,testability}-lens/SKILL.md`:
- `description:` (line 3) — replace "Ticket review lens for" with
  "Work-item review lens for"
- `applies_to: [ticket]` → `applies_to: [work-item]`
- Body prose: replace "ticket" / "tickets" with "work item" / "work
  items" using the literal-vs-prose rule (see "Prose vs literal" note
  in Phase 3).

#### 2.6 Rename the output-format skill

```bash
jj mv skills/review/output-formats/ticket-review-output-format \
      skills/review/output-formats/work-item-review-output-format
```

Inside `work-item-review-output-format/SKILL.md`:
- `name:` frontmatter → `work-item-review-output-format`
- `description:` prose → updated
- JSON-schema example → update any `type: "ticket-review"` discriminator

#### 2.7 Update artifact type in review-work-item

**File**: `skills/work/review-work-item/SKILL.md:347`

Change artifact stamp from `type: ticket-review` to
`type: work-item-review`.

### Success Criteria:

#### Phase boundary — test-subset green/red expectations:

| Subset | End of Phase 2 |
|---|---|
| `test:format` | GREEN |
| `test:work-items` | GREEN |
| `test:skills` | GREEN |
| `test:config` (or whatever subset wires `test-config.sh`) | GREEN |
| `test:cross-references` | MAY BE RED — Phase 3 absorbs lens-prose and template references |
| `test:evals` | MAY BE RED — fixture rename is Phase 3 |

#### Automated Verification:

- [ ] `bash -n scripts/config-read-review.sh` is clean
- [ ] `scripts/config-read-review.sh work-item` produces output that
      matches `scripts/test-fixtures/config-read-review/work-item-mode-golden.txt`
      byte-for-byte
- [ ] `scripts/config-read-review.sh ticket` exits non-zero. The
      error message points the user at `/accelerator:migrate`
      (e.g. "unknown mode 'ticket' — `ticket` mode was removed in
      v1.19.0; run `/accelerator:migrate` to upgrade your repo,
      then use `work-item` mode").
- [ ] No "ticket" literal in `scripts/config-read-review.sh`,
      `scripts/test-fixtures/config-read-review/`, or any
      `applies_to:` line under `skills/review/lenses/`:
      `rg '\bticket' scripts/config-read-review.sh
      scripts/test-fixtures/config-read-review/
      skills/review/lenses/*/SKILL.md` returns nothing in
      frontmatter / `applies_to:` / config-key / mode-literal
      contexts (lens body prose update in Phase 3 if any remains).

#### Manual Verification:

- [ ] Run `/accelerator:review-work-item @meta/work/<one-item>.md` in
      this repo (after Phase 5) and verify the artifact is stamped
      `type: work-item-review`
- [ ] Lens output reads coherently with the renamed prose

---

## Phase 3: Template, evals, fixtures, prose, docs

### Overview

Rename the template file and its frontmatter key; update the template
lookup wiring; rename eval fixtures and the JSON references that bind
them; update body prose in all SKILL.md files (the prose
that Phases 1–2 deferred); update README and CHANGELOG.

This is the longest phase by sheer text volume but the lowest risk —
most edits are find-and-replace inside Markdown.

### Changes Required (Step 1 — RED):

#### 3.1 Update eval JSON files to reference renamed fixtures

For each eval JSON (`skills/work/<skill>/evals/{evals,benchmark}.json`
and `skills/review/lenses/<lens>/evals/{evals,benchmark}.json`):
- Replace fixture path literals: `…/files/<scenario>/ticket.md` →
  `…/files/<scenario>/work-item.md`.
- Special-case fixtures from research:
  - `scenario-11b/tickets/0001-stub-ticket-1.md` →
    `scenario-11b/work-items/0001-stub-work-item-1.md` (and
    sibling -2 through -35)
  - `scenario-11b/target-ticket.md` → `scenario-11b/target-work-item.md`
  - `clean-ticket/ticket.md` → `clean-work-item/work-item.md`
  - `ticket-before-edit.md` → `work-item-before-edit.md`
  - `ticket-during-edit.md` → `work-item-during-edit.md`
- Replace slash-command literals in `prompt:` strings:
  `/list-tickets` → `/list-work-items`, etc.
- Replace `meta/tickets/` directory literals in fixture paths:
  `meta/tickets/9999-does-not-exist.md` →
  `meta/work/9999-does-not-exist.md`.

#### 3.2 Confirm RED

Run the eval-format check (the cheap, deterministic part of the
eval suite that verifies fixture references resolve — see Testing
Strategy section). Expected failure: the JSON references files
that haven't been moved yet, so the format check reports
unresolved fixture paths.

**Do NOT run live LLM evals** at the RED checkpoint — they are
expensive, slow, and non-deterministic. Reserve full eval
execution for Phase 6 sign-off.

`test:format`, `test:work-items`, `test:skills` MUST stay green.
Squash the §3.2 RED change into the GREEN change at the end of
§3.10 so the phase lands atomically.

### Changes Required (Step 2 — GREEN):

#### 3.3 Rename the template

```bash
jj mv templates/ticket.md templates/work-item.md
```

Inside `templates/work-item.md`:
- Frontmatter field `ticket_id:` → `work_item_id:` (line 2)
- Comment on `parent` (line 9): "ticket number of the parent
  epic/story" → "work-item number of the parent epic/story"
- Body H1 placeholder `# NNNN: Title` (no change — no "ticket"
  literal)

#### 3.4 Update template lookup wiring (atomic with §3.3)

All edits in this step move together with the template rename in
§3.3 so there is no intermediate state where the template file,
the recognised-keys list, and the helper-script call site
disagree:

- `scripts/config-read-template.sh:6` — doc-comment listing
  recognised template keys: replace `ticket` with `work-item`.
  Verify whether the script's recognition logic is doc-comment-
  driven (read the file end-to-end before edit). If a hardcoded
  allowlist of keys exists alongside the comment, update both in
  one edit.
- `skills/work/scripts/work-item-template-field-hints.sh:53` —
  the literal `config-read-template.sh ticket` becomes
  `config-read-template.sh work-item`.
- `skills/work/scripts/work-item-template-field-hints.sh:24-49` —
  hardcoded fallback content; mirror the new template (the
  `parent` comment text and the `ticket_id:` → `work_item_id:`
  field need updating; other field names haven't changed).
- `skills/config/configure/SKILL.md:189-197, :294-301, :485` —
  template enum lists; replace `ticket` with `work-item`.
- `README.md:184` — list of recognised template keys.

#### 3.5 Rename eval fixtures (filesystem)

Generate the rename commands programmatically so the entire batch
runs under `set -euo pipefail` and aborts at first failure (no
half-renamed eval suite). Use a one-shot script committed
temporarily, then deleted:

```bash
# scripts/rename-fixtures.sh (committed at start of §3.5,
# deleted at end after verification)
set -euo pipefail

# 1. Rename every fixture file or directory whose basename
# contains "ticket".
find skills -depth -name '*ticket*' \
  -not -path '*/.*' \
  | while IFS= read -r path; do
    parent="$(dirname "$path")"
    new_basename="$(basename "$path" \
      | sed -e 's/tickets/work-items/g' -e 's/ticket/work-item/g')"
    new_path="$parent/$new_basename"
    [ "$path" != "$new_path" ] && jj mv "$path" "$new_path"
  done

# 2. Rewrite ticket_id: → work_item_id: in every renamed fixture
# file with frontmatter (atomic temp-file-then-rename).
find skills -name '*.md' -path '*/evals/files/*' -print0 \
  | while IFS= read -r -d '' file; do
    if grep -q '^ticket_id:' "$file"; then
      sed 's/^ticket_id:/work_item_id:/' "$file" > "$file.tmp" \
        && mv "$file.tmp" "$file"
    fi
  done
```

Run the script, then verify:

```bash
find skills -name '*ticket*' | wc -l    # must be 0
find skills -name '*work-item*' | head  # spot-check naming
grep -rl '^ticket_id:' skills/          # must be empty
```

Then delete the helper script and commit. The verification
commands also appear in the Success Criteria below.

#### 3.6 Update body prose in all SKILL.md files

Files with substantial body prose to update:
- `skills/work/<each>/SKILL.md` body
- `skills/review/lenses/<each>/SKILL.md` body
- `skills/review/output-formats/work-item-review-output-format/SKILL.md`
  body
- `skills/planning/create-plan/SKILL.md:22-23, :52-53`
- `skills/config/configure/SKILL.md:132, :161, :213-214, :225-227,
  :316-318, :393-397, :436` (≈12 reference points)
- `skills/config/init/SKILL.md:27, :29`

**Prose vs literal rule**: in body prose, "ticket" → "work item" (two
words, with space). In identifiers/paths/config keys/slash commands,
"ticket" → "work-item" (hyphenated). Examples:
- "the ticket's frontmatter" → "the work item's frontmatter"
- `ticket_id:` → `work_item_id:`
- `/refine-ticket` → `/refine-work-item`

#### 3.7 Update the cross-skill hierarchy fences

`skills/work/list-work-items/SKILL.md:209-214` and
`skills/work/refine-work-item/SKILL.md:360-365` (the fence
markers `<!-- canonical-tree-fence -->` … `<!-- /canonical-tree-fence -->`)
carry byte-exact canonical-tree fences that
`scripts/test-hierarchy-format.sh` asserts equal. Edit the
content between the markers in both files in a single edit pass
and confirm equality with `diff <(sed -n '/<!-- canonical-tree-fence -->/,/<!-- \/canonical-tree-fence -->/p' skills/work/list-work-items/SKILL.md) <(sed -n '/<!-- canonical-tree-fence -->/,/<!-- \/canonical-tree-fence -->/p' skills/work/refine-work-item/SKILL.md)`.

Re-verify the line ranges against current state before editing —
preceding edits in this phase may have shifted them.

#### 3.8 Update README and CHANGELOG

**README.md** — replace at lines 86, 184, 244-273 (the entire
**Ticket Management** H2 section title and body), 315, 337, 350.
Section title becomes "Work Item Management".

**CHANGELOG.md** — under the Unreleased section (lines 5-98), rewrite
all references to ticket skills/paths/config keys to use the new
names. Add a new bullet under `### Changed`:

```markdown
### Changed
- **BREAKING**: Renamed the `tickets` skill category to `work` and
  individual `ticket` references to `work-item`. Slash commands are
  now `/accelerator:create-work-item`,
  `/accelerator:extract-work-items`, `/accelerator:list-work-items`,
  `/accelerator:refine-work-item`, `/accelerator:review-work-item`,
  `/accelerator:stress-test-work-item`, and
  `/accelerator:update-work-item`. Default storage directory is
  `meta/work/`. Config keys `paths.tickets`/`paths.review_tickets`
  become `paths.work`/`paths.review_work`. Template renamed
  `templates/ticket.md` → `templates/work-item.md`; frontmatter field
  `ticket_id` becomes `work_item_id`.
- **Upgrade procedure**: commit any pending changes to your
  repository, then run `/accelerator:migrate`. The skill renames
  `meta/tickets/` → `meta/work/` (preserving any custom
  `paths.tickets` directory location and rewriting only the
  config key), rewrites `ticket_id:` → `work_item_id:` in every
  file under the resolved work-item directory, and updates
  `.claude/accelerator*.md` config keys. The skill is destructive
  by default but refuses to run on a dirty working tree, and
  prints a one-line preview per pending migration before
  applying. Review the resulting `jj diff` / `git diff` before
  committing.
```

#### 3.9 Fix broken cross-references in historical meta documents (only)

Find historical research/plan/decision/review docs that link to a
path that no longer resolves (e.g. `@meta/tickets/0007-foo.md` →
target file is now at `@meta/work/0007-foo.md` after Phase 5; or
`skills/tickets/...` in code references):

```bash
rg -l 'meta/tickets/|skills/tickets/' meta/ | grep -v 'meta/research/codebase/2026-04-25-rename-tickets-to-work-items.md'
```

For each match, only update broken cross-references — do NOT rewrite
prose. The research document itself is excluded (it's the source of
truth for this plan; its prose remains as-written).

#### 3.10 Add hyphenation guard to format suite

Add a new test under `scripts/test-format.sh` (or add a sourced
sub-script and wire it in) that enforces the prose-vs-literal
hyphenation rule:

```bash
# Disallow `work item` (space) inside identifier contexts:
# - inside backtick spans (likely a code/identifier reference)
# - immediately followed by `_id`, `:`, or `/` (key, path, slash command)
# This is a coarse approximation — flag and review by hand on hits.

if rg -n '`[^`]*work item[^`]*`|work item(_id|:|/)' \
    skills/ scripts/ templates/ README.md CHANGELOG.md; then
  echo "FAIL: 'work item' (space) found in identifier context — use 'work-item'"
  exit 1
fi

# Disallow `work-item` (hyphen) inside obvious prose contexts:
# - flagged on hits where surrounding lowercase letters suggest English prose
# (Heuristic; review by hand. Skip for now if too noisy.)
```

Add this as a new step in the format-test runner so the rule is
enforced on every CI run going forward.

#### 3.11 Confirm GREEN and squash

Run `mise run test:format`, `mise run test:work-items`,
`mise run test:skills`, `mise run test:cross-references`, and the
eval-format check. Expected: all green. Squash the §3.2 RED jj
change into this GREEN change so Phase 3 lands as a single atomic
transition.

### Success Criteria:

#### Phase boundary — test-subset green/red expectations:

| Subset | End of Phase 3 |
|---|---|
| `test:format` (incl. new hyphenation guard) | GREEN |
| `test:work-items` | GREEN |
| `test:skills` | GREEN |
| `test:cross-references` | GREEN |
| Eval-format check (fixtures resolve) | GREEN |
| Live eval execution (`test:evals` full run) | DEFERRED to Phase 6 sign-off |

#### Automated Verification:

- [ ] `mise run test:format` passes including the new hyphenation
      guard
- [ ] `mise run test:cross-references` passes
- [ ] Eval-format check passes (fixtures resolve to existing
      files); live LLM eval execution deferred
- [ ] `rg '\bticket' skills/ templates/ scripts/ .claude-plugin/`
      returns no hits in identifier contexts (frontmatter values,
      config keys, slash commands, paths, allowed-tools, ADR types).
      Body prose hits should be zero in active SKILL.md files (any
      remaining are bugs to fix).
- [ ] `find skills -type d -name '*ticket*'` returns nothing
- [ ] `find skills -type f -name '*ticket*'` returns nothing
- [ ] `templates/ticket.md` no longer exists; `templates/work-item.md`
      exists
- [ ] Hierarchy assertion is green:
      `scripts/test-hierarchy-format.sh` passes with new defaults
- [ ] `scripts/rename-fixtures.sh` deleted (one-shot helper)

#### Manual Verification:

- [ ] README's "Work Item Management" section reads coherently end to
      end
- [ ] CHANGELOG bullet conveys the breaking change clearly to a
      reader who has only seen the v1.18.0 docs
- [ ] One eval scenario from `skills/work/refine-work-item/evals/`
      runs cleanly (e.g. via the eval runner if available; otherwise
      open the fixture and confirm cross-references resolve)

---

## Phase 4: Migration framework

### Overview

Build `/accelerator:migrate` test-first. The skill's job is to apply
ordered, idempotent shell migrations from
`skills/config/migrate/migrations/` to a consumer repository, tracking
applied migrations in `meta/.migrations-applied`. The first registered
migration is `0001-rename-tickets-to-work.sh`, which performs the
user-side rename in their `meta/` and `.claude/accelerator*.md` files.

Default behaviour is destructive but guarded: the driver verifies a
clean working tree before mutating, prints a one-line preview per
pending migration, and aborts with a clear error if collisions are
detected. There is no `--dry-run` flag (the preview + clean-tree
guard provides the same protection without an extra flag) and no
rollback (revert via VCS).

This phase also adds a SessionStart hook so users whose
`meta/.migrations-applied` lags the plugin's bundled migrations
get a one-line warning pointing at `/accelerator:migrate` —
without it, users would only discover they need to migrate when
something breaks.

### Changes Required (Step 1 — RED: write the test suite first):

#### 4.1 Create the test runner

**File**: `skills/config/migrate/scripts/test-migrate.sh`

Mirror the pattern of `skills/decisions/scripts/test-adr-scripts.sh`
and `skills/work/scripts/test-work-item-scripts.sh`:

- Source `scripts/test-helpers.sh` for `assert_eq`, `assert_exit_code`,
  `test_summary`, etc.
- `setup_test_repo` helper creates a temp directory with a
  `.git/` marker (so `find_repo_root` works), seeds it with
  `meta/tickets/0001-foo.md` containing
  `---\nticket_id: 0001\n---\n# 0001: Foo`, plus
  `meta/reviews/tickets/foo-review-1.md`, plus
  `.claude/accelerator.md` with `paths: { tickets: meta/tickets }`.
- `cleanup_test_repo` removes the temp dir.

Test cases:

1. **Apply pending migration succeeds**:
   - Setup temp repo with old structure
   - Run the migrate skill driver (a small wrapper that the SKILL.md
     invokes — implementation per 4.4)
   - Assert: `meta/work/0001-foo.md` exists
   - Assert: `meta/work/0001-foo.md` contains `work_item_id: 0001`,
     no `ticket_id:`
   - Assert: `meta/reviews/work/foo-review-1.md` exists
   - Assert: `.claude/accelerator.md` contains `paths: { work:
     meta/work }`, no `paths.tickets`
   - Assert: `meta/.migrations-applied` exists and contains exactly
     `0001-rename-tickets-to-work\n`

2. **Re-running is idempotent**:
   - Continue from test 1's final state
   - Run the migrate driver again
   - Assert: zero filesystem changes (compare hashes of `meta/`
     contents)
   - Assert: `meta/.migrations-applied` is unchanged

3. **Already-applied migration is skipped on first run if state file
   pre-populated**:
   - Setup temp repo with new structure already (`meta/work/...`),
     pre-write `meta/.migrations-applied` containing
     `0001-rename-tickets-to-work`
   - Run the driver
   - Assert: zero changes; no errors

4. **Failed migration aborts and does NOT update state file**:
   - Setup temp repo with old structure, then pre-create
     `meta/work` as a regular file (not a directory). The
     migration's directory rename will fail deterministically
     because the target path exists as a non-directory — this
     reproduces independent of OS, filesystem, and effective
     UID (root in CI bypasses permission bits, so the
     read-only-parent approach is non-portable).
   - Run the driver
   - Assert: non-zero exit
   - Assert: `meta/.migrations-applied` is empty / does not
     contain `0001-rename-tickets-to-work`
   - Assert: original `meta/tickets/0001-foo.md` is intact (the
     atomic-resequence in §4.6 means frontmatter rewrites have
     already succeeded; the file still has `work_item_id:` in
     `meta/tickets/`, which is the desired retry-friendly state).

5. **Per-migration idempotency** (test the migration script in
   isolation, without the skill driver):
   - Setup temp repo with old structure
   - Run `0001-rename-tickets-to-work.sh` directly with `PROJECT_ROOT`
     env var
   - Assert: rename happened
   - Run again — assert: no further changes, exit 0

6. **Empty repo (no `meta/tickets/`) is a no-op**:
   - Setup a temp dir with only `.git/` and an empty `meta/`
   - Run the driver
   - Assert: exit 0, no errors, state file written with
     `0001-rename-tickets-to-work` (the migration was applied to a
     repo where there was nothing to migrate, but it ran successfully
     and we record that)

7. **`paths.tickets` override is respected — pinned directory
   preserved**:
   - Setup temp repo with `meta/custom-tix/0001-foo.md` and
     `.claude/accelerator.md` containing
     `paths: { tickets: meta/custom-tix }`
   - Run the migration
   - Assert: `meta/custom-tix/` still exists with its files
     (directory NOT moved — the migration rewrites plugin-level
     expectations, not user intent).
   - Assert: `meta/custom-tix/0001-foo.md` contains
     `work_item_id: 0001`, no `ticket_id:` (frontmatter rewrite
     applies regardless of where the directory lives).
   - Assert: config file has `paths: { work: meta/custom-tix }`
     (key renamed, value preserved).
   - Assert: `meta/work/` does NOT exist (no spurious default
     directory created).

   This is the conservative semantics decided in the migration
   framework ADR (§Phase 0): a user's pinned configuration value
   is honoured, only the key is rewritten.

8. **Both default and custom dirs exist — collision aborts
   cleanly**:
   - Setup temp repo with both `meta/tickets/` and `meta/work/`
     populated (e.g. user has partially attempted a manual
     migration).
   - Run the driver.
   - Assert: non-zero exit with an error message naming both
     paths and pointing at the resolution procedure (manual
     merge or removal of one directory).
   - Assert: neither directory is touched.
   - Assert: `meta/.migrations-applied` does not record
     `0001-rename-tickets-to-work`.

9. **Malformed YAML in user config — refuse to migrate**:
   - Setup temp repo with `.claude/accelerator.md` containing
     unterminated frontmatter (e.g. opening `---` with no
     closing).
   - Run the driver.
   - Assert: non-zero exit with a clear error mentioning the
     config file.
   - Assert: `meta/tickets/` is unchanged (no partial migration).

10. **Corrupt state file — preserve unknown IDs, warn, do not
    delete**:
    - Setup temp repo with `meta/.migrations-applied` containing
      `0099-future-migration\n` (a migration ID the installed
      plugin does not know about — simulates a downgrade).
    - Run the driver.
    - Assert: exit 0 (migration 0001 still applies).
    - Assert: state file contains both `0099-future-migration`
      (preserved verbatim) and `0001-rename-tickets-to-work`.
    - Assert: stderr contains a warning about the unknown ID.

11. **Frontmatter with both `ticket_id:` and `work_item_id:` —
    no duplicate key**:
    - Setup temp repo with `meta/tickets/0001-foo.md` whose
      frontmatter contains both `ticket_id: 0001` and
      `work_item_id: 0001` (simulates partial prior rewrite).
    - Run the driver.
    - Assert: post-migration file has exactly one
      `work_item_id:` line and no `ticket_id:` line.

12. **Paths with spaces — handled correctly**:
    - Setup temp repo with `meta/tickets/0001-with space.md`
      (filename contains a space).
    - Run the driver.
    - Assert: `meta/work/0001-with space.md` exists with rewrite
      applied.

13. **Driver — two pending migrations applied in order**:
    - Setup temp repo with old structure plus a stub
      `0002-noop.sh` migration in a fixture migrations directory
      (the test seeds an alternative `MIGRATIONS_DIR` env var so
      the driver picks up both 0001 and 0002 from a fixture
      directory rather than the bundled one).
    - Run the driver.
    - Assert: both migration IDs recorded in
      `meta/.migrations-applied`, in order.
    - Assert: stub migration's marker file (e.g. `touch /tmp/0002-ran`)
      exists, demonstrating it ran after 0001.

14. **Driver — clean-tree pre-flight aborts on dirty repo**:
    - Setup temp repo with old structure. Stage an unrelated
      uncommitted change in `meta/tickets/0001-foo.md` (e.g.
      append a line without committing).
    - Run the driver without
      `ACCELERATOR_MIGRATE_FORCE=1`.
    - Assert: non-zero exit with an error mentioning the dirty
      working tree.
    - Assert: no migration applied; no state-file entry.
    - Re-run with `ACCELERATOR_MIGRATE_FORCE=1` and assert the
      migration applies normally (override path).

#### 4.2 Add a mise task

Before authoring this step, run `mise tasks | grep '^test:'` and
inspect the existing per-skill test task definitions (e.g.
`mise task info test:work-items` after Phase 1, or one of the
other `test:*` entries) to confirm the convention. Wire the new
`test:migrate` task into the same location and pattern.

The new task invokes `skills/config/migrate/scripts/test-migrate.sh`
and is added to whatever aggregate `test` task already runs the
other per-skill test suites.

#### 4.3 Confirm RED

Run `mise run test:migrate`. Expected: every test fails because
neither the driver script nor the migration script exists yet.
Record the failing test names in the jj change description.

Other test subsets MUST stay green — Phase 4 is purely additive
(new files under `skills/config/migrate/`); no existing surface
is modified. Commit this state and squash into the GREEN change
at the end of §4.9 (final GREEN confirm step) so Phase 4 lands
atomically.

### Changes Required (Step 2 — GREEN: build the skill):

#### 4.4 Create the migrate SKILL.md

**File**: `skills/config/migrate/SKILL.md`

Frontmatter:
```yaml
---
name: migrate
description: Apply pending Accelerator meta-directory migrations to bring a repo into line with the latest plugin schema. Destructive by default but guarded — refuses to run on a dirty working tree and prints a one-line preview per pending migration before applying.
allowed-tools: [Read, Write, Edit, Bash]
---
```

Body opens with a prominent warning block describing the
destructive nature (rewrites files in `meta/` and
`.claude/accelerator*.md`), the recovery path (VCS revert), and
the safety guards (clean-tree pre-flight, preview-before-apply).
It then covers:

- When to invoke (after upgrading the plugin to a version that
  ships new migrations — typically signalled by the SessionStart
  hook from §4.10).
- The state file (`meta/.migrations-applied`) format.
- The migration registry layout (`migrations/` glob ordered by
  ID).
- Behaviour:
  1. Verify a clean working tree (`jj status` or
     `git status --porcelain` reports no uncommitted changes
     within `meta/` or `.claude/accelerator*.md`). Abort with a
     clear error otherwise. The
     `ACCELERATOR_MIGRATE_FORCE=1` env var bypasses this check
     for advanced users.
  2. Read state file, glob bundled migrations, filter applied,
     identify pending IDs.
  3. Print a one-line preview per pending migration (using the
     `# DESCRIPTION:` comment from each migration script).
  4. Apply each pending migration in order; on success append
     its ID; on failure abort with no partial state-file write.
  5. Print an end-of-run summary table.
- Per-migration contract:
  - First two lines: `#!/usr/bin/env bash` then
    `# DESCRIPTION: <short imperative description>`.
  - Receives `PROJECT_ROOT` env var.
  - Must self-detect no-op conditions and exit 0 on
    already-applied state.
  - Must use atomic write patterns (temp-file-then-rename) for
    file rewrites.
  - Must NOT honour any `DRY_RUN` env var (this framework
    intentionally has no dry-run mode).
- Cross-reference to the migration framework ADR.

The SKILL.md drives the migration via shell — the skill body
instructs Claude to invoke a driver script:

#### 4.5 Create the driver script

**File**: `skills/config/migrate/scripts/run-migrations.sh`

Logic:

1. **Resolve `PROJECT_ROOT`** by sourcing
   `scripts/config-common.sh` and calling
   `config_project_root` (the canonical repo-root resolver
   already used by `config-read-review.sh`). Do NOT depend on
   `scripts/test-helpers.sh` — that file is test-scoped.

2. **Pre-flight: verify clean working tree**. If
   `ACCELERATOR_MIGRATE_FORCE` is set, skip this check.
   Otherwise:
   - Detect VCS: prefer `jj` if `.jj/` exists, else `git` if
     `.git/` exists; if neither, print a warning and proceed
     (assumes the user has chosen to run without VCS).
   - Run the appropriate status command and parse output for
     uncommitted changes within `meta/` or
     `.claude/accelerator*.md`. Abort with non-zero exit and a
     clear error if any are found, instructing the user to
     commit/discard first or set `ACCELERATOR_MIGRATE_FORCE=1`.

3. **Read state file** at
   `$PROJECT_ROOT/meta/.migrations-applied` into an array (treat
   missing file as empty). Preserve all entries verbatim — even
   IDs the bundled registry doesn't recognise (downgrade
   tolerance).

4. **Glob bundled migrations**:
   `${CLAUDE_PLUGIN_ROOT}/skills/config/migrate/migrations/[0-9][0-9][0-9][0-9]-*.sh`
   in sorted order. The migrations directory may also be
   overridden via `ACCELERATOR_MIGRATIONS_DIR` (used by tests in
   §4.1 case 13 to inject fixture migrations).

5. **Warn about unknown applied IDs**: for each ID in the state
   file that isn't in the bundled registry, print
   `[warning] meta/.migrations-applied references unknown
   migration <ID> — preserved on rewrite` to stderr. Do NOT
   delete the entry.

6. **Compute pending list**: bundled migrations whose IDs are
   NOT in the applied set.

7. **Print preview**: for each pending migration, extract its
   `# DESCRIPTION:` comment and print
   `[<ID>] <description>` to stdout. If there are no pending
   migrations, print `No pending migrations.` and exit 0.

8. **Apply each pending migration in order**:
   - Tag stderr with `[<ID>] running` at start.
   - Export `PROJECT_ROOT`.
   - Run the migration script.
   - On success: append the ID to
     `meta/.migrations-applied` (using atomic temp-file-then-
     rename so a crash mid-write cannot corrupt the state
     file). Tag stderr with `[<ID>] applied`.
   - On non-zero exit: tag stderr with `[<ID>] failed`, print
     the migration's last stdout/stderr, exit 1. Do NOT
     update the state file. Existing applied IDs from prior
     successful migrations remain.

9. **Print end-of-run summary**: a small table to stdout
   listing applied / already applied / failed migrations and
   their descriptions.

The driver itself is small (~80 lines) and focused entirely on
orchestration — all transformation logic lives in the migration
scripts.

#### 4.6 Create the first migration

**File**: `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh`

The script begins:
```bash
#!/usr/bin/env bash
# DESCRIPTION: Rename tickets/work-item terminology in meta/ and config files
set -euo pipefail
```

The migration's logical pipeline. Note the order: **frontmatter
rewrites happen BEFORE directory rename**, so the directory
rename is the atomic transition point. If the script crashes
between any two steps, the user can re-run cleanly because every
step is independently idempotent.

1. **Resolve user paths**.
   - Read `.claude/accelerator.md` (and
     `.claude/accelerator.local.md` if present) to extract any
     `paths.tickets` / `paths.review_tickets` overrides. Use
     `scripts/config-common.sh`'s `config_extract_frontmatter`
     primitive (sourced) to parse YAML structurally rather than
     by regex — handles both nested (`paths:\n  tickets: ...`)
     and flat-dotted (`paths.tickets: ...`) forms uniformly.
     If the config file has malformed frontmatter (unterminated
     `---`, etc.), abort with a clear error before mutating
     anything.
   - Resolved values:
     - `tickets_dir`: pinned value or default `meta/tickets`.
     - `review_tickets_dir`: pinned value or default
       `meta/reviews/tickets`.
   - Track whether each path was pinned (to a non-default value)
     or defaulted, so step 4 can decide whether to rename the
     directory.

2. **Frontmatter rewrites (in-place, idempotent)**.
   - For every `*.md` file under `tickets_dir`, atomically
     rewrite `^ticket_id:` to `^work_item_id:` using a portable
     temp-file-then-rename pattern:
     ```bash
     while IFS= read -r -d '' file; do
       if grep -q '^ticket_id:' "$file"; then
         sed 's/^ticket_id:/work_item_id:/' "$file" > "$file.tmp"
         mv "$file.tmp" "$file"
       fi
     done < <(find "$tickets_dir" -name '*.md' -print0)
     ```
     This pattern works identically on BSD sed (macOS) and GNU
     sed (Linux), unlike `sed -i ''` which is BSD-only. The
     null-delimited `find ... -print0` handles paths with
     spaces.
   - If a file already contains both `ticket_id:` and
     `work_item_id:` (partial prior rewrite), the sed pass
     drops the `ticket_id:` line and leaves `work_item_id:`
     intact — no duplicate keys.

3. **Collision check (before any directory rename)**.
   - If `tickets_dir` is the default (`meta/tickets`) AND
     `meta/work` already exists AND `meta/tickets` also exists,
     abort with: `Both meta/tickets and meta/work exist —
     cannot proceed. Manually merge or remove one of them, then
     re-run /accelerator:migrate.`
   - Same check for `review_tickets_dir` → `meta/reviews/work`.
   - Tracked-pinned paths skip these checks (the user's pinned
     directory is not being renamed).

4. **Directory renames (only when the resolved path is the
   plugin default)**.

   For each of (`tickets_dir`, `meta/work`) and
   (`review_tickets_dir`, `meta/reviews/work`), execute one of
   four explicit branches based on (source-exists × target-exists):

   | Source | Target | Action |
   |---|---|---|
   | exists | absent | Rename (when the source is the default path; pinned paths skip this entirely) |
   | exists | exists | Already aborted in step 3 |
   | absent | exists | No-op (already migrated) |
   | absent | absent | No-op (nothing to migrate, e.g. fresh repo) |

   Pinned paths (where `tickets_dir != meta/tickets`) skip the
   directory rename in step 4 entirely — the user's directory
   stays where they put it. The frontmatter rewrites in step 2
   already covered files in those directories.

5. **Config-key rewrites (atomic write-temp-then-rename)**.

   For each of `.claude/accelerator.md` and
   `.claude/accelerator.local.md` if present:
   - Use `config_extract_frontmatter` to read the YAML
     structurally. Map old keys to new:
     - `paths.tickets` → `paths.work` (preserving the value)
     - `paths.review_tickets` → `paths.review_work` (preserving
       the value)
     - `review.ticket_revise_severity` →
       `review.work_item_revise_severity`
     - `review.ticket_revise_major_count` →
       `review.work_item_revise_major_count`
     - `review.min_lenses_ticket` → `review.min_lenses_work_item`
   - Re-emit the frontmatter with renamed keys (preserving
     comments and ordering of unrelated keys to the extent
     `config_extract_frontmatter`'s emit pass supports). Write
     to `<file>.tmp`, then `mv <file>.tmp <file>`.
   - If `config_extract_frontmatter` doesn't have a structural
     emit pass, document the limitation and use a constrained
     line-anchored sed pattern that only rewrites top-of-line
     occurrences of the target keys (with whitespace tolerance).
     Add fixtures covering both nested and flat-dotted forms,
     plus a comment containing the word "tickets" (must NOT be
     touched).

6. **Exit 0 on full success.** The driver appends the migration
   ID to `meta/.migrations-applied` only after this clean exit.

Each step's idempotency guard makes re-running safe:
- Step 2 is naturally idempotent — already-rewritten files have
  no `^ticket_id:` to match.
- Step 4's four-branch table handles every (source × target)
  state explicitly.
- Step 5's old keys are absent on re-run, so no rewrite occurs.

#### 4.7 Update plugin manifest if needed

`.claude-plugin/plugin.json` already lists `./skills/config/`. The
new migrate skill is auto-discovered. No edit needed unless the
manifest enumerates individual skills (it doesn't, per
`plugin.json:10-20`).

#### 4.8 Documentation

- README: add a "Migrations" subsection (one paragraph + the
  command + the safety guards).
- `skills/config/configure/SKILL.md`: mention
  `/accelerator:migrate` alongside `/accelerator:init` (one
  paragraph).
- `skills/config/init/SKILL.md`: add a cross-reference noting that
  `init` initialises a fresh repo and `migrate` upgrades an existing
  one.

#### 4.9 SessionStart hook for migration discoverability

**Files**:
- `hooks/migrate-discoverability.sh` (new shell script)
- A registration entry under the project's existing hooks
  configuration (read the current hooks layout — likely under
  `.claude/settings.json` or a hook-config file — to identify
  where SessionStart hooks register).

**Logic**:
- Resolve `PROJECT_ROOT` via `config_project_root`.
- Determine the highest applied migration ID in
  `$PROJECT_ROOT/meta/.migrations-applied` (lex-max of
  `[0-9][0-9][0-9][0-9]-` prefix).
- Determine the highest available migration ID in the bundled
  registry under
  `${CLAUDE_PLUGIN_ROOT}/skills/config/migrate/migrations/`.
- If applied < available (or applied is empty AND available is
  non-empty AND the repo has any
  Accelerator-meta state — e.g. `meta/` or
  `.claude/accelerator.md` exists), print one line to stderr:
  ```
  [accelerator] meta/.migrations-applied is behind the plugin
  (highest applied: <X>; highest available: <Y>). Run
  /accelerator:migrate to bring it up to date.
  ```
- Exit 0 in all cases — the hook is informational only and must
  never block session start.

**Why this design**:
- The detection is comparative (applied vs available), so the
  hook self-adjusts as new migrations land — no per-release
  edit needed.
- The detection only fires for repos that are clearly using
  Accelerator (otherwise users with a `meta/` directory unrelated
  to this plugin would see spurious warnings).
- Stderr keeps the warning visible without contaminating
  programmatic stdout consumers.

#### 4.10 Confirm GREEN and squash

Run `mise run test:migrate`, `mise run test:format`, and
`mise run test`. Expected: all green. Squash the §4.3 RED jj
change into this GREEN change.

### Success Criteria:

#### Phase boundary — test-subset green/red expectations:

| Subset | End of Phase 4 |
|---|---|
| `test:migrate` (new) — all 14 cases | GREEN |
| `test:format` | GREEN |
| `test:work-items`, `test:skills`, `test:cross-references` | GREEN |
| Eval-format check | GREEN |
| Live eval execution | DEFERRED to Phase 6 sign-off |

#### Automated Verification:

- [ ] `mise run test:migrate` passes — all 14 test cases green
      (cases 1-7 happy/edge, 8-12 added edge cases, 13-14 driver-
      level)
- [ ] `mise run test` overall is green (no regressions)
- [ ] `bash -n skills/config/migrate/scripts/run-migrations.sh` is
      clean
- [ ] `bash -n skills/config/migrate/migrations/0001-rename-tickets-to-work.sh`
      is clean
- [ ] `bash -n hooks/migrate-discoverability.sh` is clean
- [ ] `mise run test:format` passes (new SKILL.md follows the
      format contract)
- [ ] Running the driver against a temp repo with a pre-populated
      state file results in zero changes and exit 0
- [ ] Running the driver on macOS AND in a Linux container both
      pass `test:migrate` (CI must run at least one of these on
      Linux to catch BSD-vs-GNU portability regressions)

#### Manual Verification:

- [ ] Read `skills/config/migrate/SKILL.md` end-to-end and confirm
      a first-time reader could understand when and how to use it,
      including the destructive-with-guards posture
- [ ] Run the migrate skill against a sandbox copy of a v1.18.x
      repo and verify the rename happens correctly
- [ ] Confirm the state file is human-readable and shows a clear
      audit trail
- [ ] Trigger the SessionStart hook by setting up a sandbox repo
      with empty `meta/.migrations-applied` and confirm the
      one-line warning fires

---

## Phase 5: Self-apply migration to this repository

### Overview

Apply the migration to this repository's own `meta/tickets/` data,
producing `meta/work/` with `work_item_id:` rewritten in every
file (count re-verified at §5.1; was 29 at the time the plan was
authored), plus `meta/reviews/tickets/` → `meta/reviews/work/`,
plus `meta/.migrations-applied` populated.

This phase is a **separate jj change** from Phase 4. The framework
build and the self-apply are decoupled so a revert of Phase 5 does
not touch the framework, and vice versa.

### Changes Required:

#### 5.1 Pre-flight check + safety bookmark

```bash
jj status                          # confirm clean working copy
ls meta/tickets | wc -l            # record file count (e.g. 29)
ls meta/reviews/tickets 2>/dev/null # confirm exists
```

Create a recoverable safety point before any mutation:

```bash
jj bookmark create pre-rename-tickets-to-work
```

Revert from a Phase-4-bug-induced corruption is then a single
command (`jj edit pre-rename-tickets-to-work`) rather than
forensics on jj log.

#### 5.2 Capture content-hash snapshot of work-item bodies

Capture sha256 hashes of every file's body (the content after
the closing `---` of the frontmatter), so we can verify
post-migration that no body content was inadvertently mutated:

```bash
mkdir -p /tmp/migrate-self-apply
for f in meta/tickets/*.md; do
  body_start=$(grep -n '^---$' "$f" \
    | sed -n '2p' | cut -d: -f1)
  if [ -n "$body_start" ]; then
    tail -n +$((body_start + 1)) "$f" \
      | sha256sum \
      | awk -v name="$(basename "$f")" '{print name, $1}'
  fi
done | sort > /tmp/migrate-self-apply/pre.sha
```

Repeat for `meta/reviews/tickets/*.md` if the directory has
content with frontmatter (or capture full-file hashes if those
files don't have frontmatter).

#### 5.3 Create a new jj change for the migration

```bash
jj new -m "Apply 0001-rename-tickets-to-work to this repository"
```

#### 5.4 Run the migrate skill

Invoke the skill via Claude Code or run the driver directly:

```bash
PROJECT_ROOT="$(pwd)" \
  ./skills/config/migrate/scripts/run-migrations.sh
```

Expected output:
- A clean-tree pre-flight pass (the new jj change above is the
  current commit, no uncommitted changes within `meta/`).
- A one-line preview: `[0001-rename-tickets-to-work] Rename
  tickets/work-item terminology in meta/ and config files`.
- One "applied" line for `0001-rename-tickets-to-work`.
- An end-of-run summary table.

#### 5.5 Verify the result

```bash
test -d meta/work
test ! -d meta/tickets
ls meta/work | wc -l                 # should match the prior count
test -d meta/reviews/work
test ! -d meta/reviews/tickets
grep -l '^ticket_id:' meta/work/*.md  # should be empty
grep -c '^work_item_id:' meta/work/*.md | grep -v ':0' | wc -l
                                       # should match the file count
cat meta/.migrations-applied          # should contain exactly:
                                       # 0001-rename-tickets-to-work
```

**Content-hash regression check** (the most important
verification — catches any bug that mutates work-item bodies):

```bash
for f in meta/work/*.md; do
  body_start=$(grep -n '^---$' "$f" \
    | sed -n '2p' | cut -d: -f1)
  if [ -n "$body_start" ]; then
    tail -n +$((body_start + 1)) "$f" \
      | sha256sum \
      | awk -v name="$(basename "$f")" '{print name, $1}'
  fi
done | sort > /tmp/migrate-self-apply/post.sha

diff /tmp/migrate-self-apply/pre.sha \
     /tmp/migrate-self-apply/post.sha
# Expected: no output (every body hash unchanged).
```

If the diff is non-empty, the migration mutated work-item body
content — abort the phase, `jj edit pre-rename-tickets-to-work`
to recover, and file a Phase 4 bug.

#### 5.6 Fix broken cross-references

After the rename, any historical document referring to
`meta/tickets/<filename>.md` is broken (the file lives at
`meta/work/<filename>.md` now). Fix only broken links — do NOT
rewrite prose:

```bash
rg -l 'meta/tickets/' meta/ \
  | grep -v 'meta/research/codebase/2026-04-25-rename-tickets-to-work-items.md' \
  | grep -v 'meta/plans/2026-04-25-rename-tickets-to-work-items.md'
```

For each match, replace `meta/tickets/` with `meta/work/` only in
contexts that are file-path links (e.g.
`@meta/tickets/0007-foo.md` → `@meta/work/0007-foo.md`). The
research and plan documents themselves (the source of truth for this
work) are excluded — their prose stays intact, including their
references to `meta/tickets/` describing the *prior* state.

Any remaining occurrences of `meta/tickets/` in the historical
prose context (e.g. "the original ticket directory was…") should
remain — these describe history, not current state.

#### 5.7 Commit

```bash
jj describe -m "Apply 0001-rename-tickets-to-work to this repository

Migrates this repo's own meta/tickets/ to meta/work/ and rewrites
ticket_id to work_item_id in all 29 work-item files. State file
records the applied migration. Pre-rename safety bookmark
'pre-rename-tickets-to-work' kept for a release window, then
deleted in Phase 6."
```

### Success Criteria:

#### Automated Verification:

- [ ] `mise run test` passes (no test references the old
      `meta/tickets/` path now that everything is renamed)
- [ ] `test -d meta/work && test ! -d meta/tickets` succeeds
- [ ] `grep -r '^ticket_id:' meta/work/` returns no matches
- [ ] `grep -c '^work_item_id:' meta/work/*.md` shows the expected
      count
- [ ] `cat meta/.migrations-applied` contains exactly
      `0001-rename-tickets-to-work` followed by a newline
- [ ] **Content-hash check**: `diff
      /tmp/migrate-self-apply/pre.sha
      /tmp/migrate-self-apply/post.sha` produces no output —
      every work-item body byte survived the migration
- [ ] `jj bookmark list` shows `pre-rename-tickets-to-work` (kept
      until Phase 6 final verification, then deleted)

#### Manual Verification:

- [ ] Open one or two work-item files and confirm frontmatter is
      well-formed
- [ ] Confirm the jj change containing this self-apply is exactly
      one change, separate from Phase 4 (verifiable via `jj log`)
- [ ] Confirm `jj diff` shows ONLY directory renames + frontmatter
      key edits + the new state file — no unrelated changes

---

## Phase 6: Release

### Overview

CHANGELOG entry, version bump, follow-up tracking, safety-bookmark
cleanup. The CHANGELOG bullets were authored in Phase 3.8; here we
move them from `Unreleased` to a new versioned section, bump the
version per project convention, file the deferred-cleanup
follow-up, and delete the Phase 5 safety bookmark.

### Changes Required:

#### 6.1 Move the Unreleased bullets into a new release section

**File**: `CHANGELOG.md`

Per the v1.9.0 precedent (rename of an unreleased surface = minor
bump, no shim, manual instruction OR migration cross-reference),
create a new `## [1.19.0] - 2026-04-25` section above `Unreleased`,
containing the two Phase 3.8 bullets (the breaking-change bullet
and the upgrade-procedure bullet). The Unreleased section becomes
empty (or starts a new feature cycle).

#### 6.2 Bump the version

Identify the exact set of files that need editing by inspecting
the prior version-bump commit — that commit is the source of
truth for which files participate in a release bump:

```bash
jj log -r 'description(glob:"Bump version to *")' --limit 5
# Pick the most recent and inspect:
jj show <commit-id> --summary
```

Update the version in every file the prior bump touched.
Currently this is `.claude-plugin/plugin.json`'s `version` field
(currently `1.19.0-pre.4` per `jj show` of the latest bump
commit — re-confirm by reading the file before editing). Bump
to `1.19.0`.

Search for any leftover prerelease stamps that the prior bump
may not have caught:

```bash
rg '1\.19\.0-pre' .  # must return nothing post-bump
```

#### 6.3 File follow-up work-item

Create a follow-up work-item under `meta/work/` (the new path)
tracking the centralisation of `PATH_DEFAULTS`:

- Title: "Centralise PATH_DEFAULTS to scripts/config-defaults.sh"
- Body: extract `PATH_KEYS` / `PATH_DEFAULTS` arrays (currently
  duplicated across 15 call sites) into a single sourced file so
  the next default rename is a one-line edit.
- Status: `proposed`.
- Reference: links back to this plan and to the migration-
  framework ADR.

#### 6.4 Delete the Phase 5 safety bookmark

After the manual verification in §6.5 confirms the release is
sound:

```bash
jj bookmark delete pre-rename-tickets-to-work
```

The bookmark was a recovery point during Phase 5; it is no
longer needed once the release is committed.

#### 6.5 Verify

```bash
mise run test       # full suite incl. live evals (Phase 3.2's
                    # deferred LLM execution lands here)
jj diff             # CHANGELOG.md, version-bump file(s), follow-up
                    # work-item only
```

### Success Criteria:

#### Automated Verification:

- [ ] `mise run test` is green (including live eval execution
      that Phase 3 deferred)
- [ ] `.claude-plugin/plugin.json`'s `version` field is `1.19.0`
- [ ] `rg '1\.19\.0-pre' .` returns no hits
- [ ] CHANGELOG has a versioned `## [1.19.0]` section above
      `Unreleased`
- [ ] Follow-up work-item exists under `meta/work/` referencing
      `PATH_DEFAULTS` centralisation
- [ ] `jj bookmark list` does not show `pre-rename-tickets-to-work`

#### Manual Verification:

- [ ] The 1.19.0 bullets read as a clear breaking-change
      announcement plus an unambiguous upgrade procedure
- [ ] The "Run `/accelerator:migrate`" sentence is unambiguous to
      a user upgrading from v1.18.x and the precondition (commit
      first) is prominent

---

## Testing Strategy

### Unit Tests:

- **Helper-script tests** (`skills/work/scripts/test-work-item-scripts.sh`,
  formerly `test-ticket-scripts.sh`): cover the renamed helper
  scripts in isolation. No new tests needed beyond updating literals.
- **Migration tests** (`skills/config/migrate/scripts/test-migrate.sh`):
  14 test cases — happy-path apply, idempotency, skipping applied,
  abort on failure (deterministic), per-migration idempotency,
  no-op on empty repo, **pinned-path preservation** (test 7),
  collision abort (8), malformed YAML (9), corrupt state file (10),
  both-keys-present frontmatter (11), paths with spaces (12),
  driver ordering with two pending (13), and clean-tree pre-flight
  with `ACCELERATOR_MIGRATE_FORCE` override (14).

### Distinguishing eval-format from live-eval execution

Per-skill eval JSON files include LLM-driven scenarios that are
expensive, slow, and non-deterministic. The plan distinguishes:

- **Eval-format check** (cheap, deterministic): verifies that
  every fixture path referenced in eval JSONs resolves to an
  existing file on disk. Used at every RED→GREEN gate in
  Phases 1-4.
- **Live eval execution** (`mise run test:evals` full run):
  runs the LLM scenarios. Deferred to Phase 6 final
  verification only — not used for phase-boundary gates.

### Integration Tests:

- **Format suite** (`mise run test:format`): hierarchy, TOC,
  frontmatter, allowed-tools, cross-reference, skill, evals format
  checks, plus the new hyphenation guard (Phase 3.10). The
  hierarchy assertion is updated in Phase 1 to point at the new
  defaults.
- **Skill format** (`mise run test:skills`): SKILL.md frontmatter
  and structure across all renamed skills.
- **Eval format** (subset of `test:format`): fixtures resolve.
- **Live eval execution** (`mise run test:evals` full run):
  per-skill evals against renamed fixtures — Phase 6 only.
- **Cross-references** (`mise run test:cross-references` if
  exists, or part of format suite): ensures inter-skill links
  resolve after the rename.

### Cross-platform CI

The migration uses portable shell patterns
(temp-file-then-rename instead of `sed -i ''`, null-delimited
`find -print0` for paths with spaces, etc.). CI must run
`test:migrate` on **both macOS and Linux** to catch BSD-vs-GNU
shell-tool divergence — verified by Phase 4 success criteria.

### Manual Testing Steps:

1. After Phase 5, run `/accelerator:list-work-items` in this repo and
   verify the listing matches what `/accelerator:list-tickets` would
   have shown before.
2. Run `/accelerator:create-work-item` in a sandbox; confirm the new
   file is at `meta/work/<NNNN>-...md` with `work_item_id:`
   frontmatter.
3. Run `/accelerator:review-work-item @meta/work/<one>.md` and
   verify the artifact is stamped `type: work-item-review`.
4. In a sandbox copy of a v1.18.x repo, run
   `/accelerator:migrate` and confirm the result matches expected
   end state.
5. Run `/accelerator:migrate` a second time on the same repo and
   confirm zero changes occur.

## Performance Considerations

- The migration script `sed`s 29 files in this repo (and however
  many in a user's repo). At ≤ 1000 work items, this is sub-second.
  No optimisation needed.
- Format-test suite runtime grows linearly with file count; the
  rename does not add files (only renames), so suite runtime is
  unchanged.
- No production hot path is affected — all changes are static
  (filesystem layout, configuration metadata).

## Migration Notes

Pre-release adopters (anyone on `1.19.0-pre.x`) and this repository
itself need migration. The path is:

1. Upgrade the plugin to `1.19.0` (new release, no shim).
2. Commit any pending changes — the migration aborts on a dirty
   working tree.
3. The SessionStart hook (Phase 4.9) emits a one-line warning when
   `meta/.migrations-applied` lags the bundled migrations, so
   users discover the upgrade requirement automatically.
4. Run `/accelerator:migrate` once. The skill prints a one-line
   preview per pending migration, then applies them. Idempotent —
   safe to re-run.
5. Verify `meta/work/` exists (or your pinned directory still
   exists if you customised `paths.tickets`), `meta/tickets/` does
   not exist, and `meta/.migrations-applied` lists
   `0001-rename-tickets-to-work`.
6. Review the resulting `jj diff` / `git diff` before committing.

For users with custom config (pinned `paths.tickets:`):
- The directory at the pinned path is **preserved** — the
  migration does NOT rename it to the default. Frontmatter
  rewrites still apply to files in the pinned directory.
- The config key is rewritten: `paths.tickets: <your-custom-path>`
  → `paths.work: <your-custom-path>`. Same for
  `paths.review_tickets` → `paths.review_work`.
- Other keys (`review.ticket_revise_severity`, etc.) are
  similarly key-renamed without value changes.

For users without VCS:
- The migration prints a warning and proceeds (the
  clean-tree pre-flight assumes VCS as the safety net; without it
  the user takes their own risk). Setting
  `ACCELERATOR_MIGRATE_FORCE=1` bypasses the pre-flight if
  needed.

## References

- Original research: `meta/research/codebase/2026-04-25-rename-tickets-to-work-items.md`
- Foundational research: `meta/research/codebase/2026-04-08-ticket-management-skills.md`
- Meta-directory philosophy: `meta/research/codebase/2026-03-18-meta-management-strategy.md`
- Init-skill precedent: `meta/research/codebase/2026-03-28-initialise-skill-requirements.md`
- Userspace config ADR: `meta/decisions/ADR-0016-userspace-configuration-model.md`
- Init-skill ADR: `meta/decisions/ADR-0018-init-skill-for-repository-bootstrap.md`
- v1.9.0 unreleased-rename precedent: `CHANGELOG.md:198-204`
- v1.8.0 manual-instruction precedent: `CHANGELOG.md:222-234`
- Hierarchy assertion: `scripts/test-hierarchy-format.sh:21-22`
- Plugin manifest: `.claude-plugin/plugin.json:16`
- Review subsystem: `scripts/config-read-review.sh:15, :28-31, :53-60, :85-86, :140-159, :206-209, :414-418, :450-473, :501-503, :556-571`
- Path arrays: `scripts/config-dump.sh:174-200`
- Template: `templates/ticket.md:2`
- Template lookup: `skills/tickets/scripts/ticket-template-field-hints.sh:53`
- Eval JSON binding example: `skills/tickets/refine-ticket/evals/evals.json` (multiple lines per fixture)
- Existing skill-test pattern: `skills/decisions/scripts/test-adr-scripts.sh`,
  `skills/tickets/scripts/test-ticket-scripts.sh`
- Test helpers: `scripts/test-helpers.sh`
