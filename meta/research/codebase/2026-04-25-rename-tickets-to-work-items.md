---
date: "2026-04-25T21:03:16+01:00"
researcher: Toby Clemson
git_commit: 6947ac9f1b3d2429623df1d008cc38578bbde52f
branch: ticket-management
repository: accelerator
topic: "Rename `tickets` category to `work` and `ticket` to `work-item`, and consider an `/accelerator:migrate` skill"
tags: [research, rename, terminology, work-item, migration, meta-directory, configuration]
status: complete
last_updated: "2026-04-25"
last_updated_by: Toby Clemson
---

# Research: Rename `tickets` → `work`, `ticket` → `work-item`, and consider `/accelerator:migrate`

**Date**: 2026-04-25T21:03:16+01:00
**Researcher**: Toby Clemson
**Git Commit**: 6947ac9f1b3d2429623df1d008cc38578bbde52f
**Branch**: ticket-management
**Repository**: accelerator

## Research Question

Replace the loaded service-desk term "ticket" with the more neutral
product-development term "work-item" throughout the codebase, with the skill
category renamed from `tickets` → `work`. The slash-command surface becomes
`/accelerator:create-work-item`, `/accelerator:review-work-item`, etc. The
default storage directory becomes `meta/work/`.

Identify every place the change needs to be applied, propose a phased plan,
and evaluate whether to add an `/accelerator:migrate` skill that applies an
ordered, idempotent set of migrations to a consumer codebase to bring its
`meta/` directory into line with the latest plugin expectations.

## Summary

The rename is large but mechanically tractable. It touches:

- **9 files** in the plugin manifest, scripts, and config layer
  (`.claude-plugin/plugin.json`, `scripts/config-read-path.sh`,
  `scripts/config-read-review.sh`, `scripts/config-dump.sh`,
  `scripts/test-hierarchy-format.sh`, README.md, CHANGELOG.md, configure
  SKILL.md, init SKILL.md)
- **The whole of `skills/tickets/`** — 7 SKILL.md files, 6 helper scripts, all
  eval fixtures (over 200 small `.md` files plus `evals.json`/`benchmark.json`
  per skill)
- **The 5 `*-lens/SKILL.md` files** under `skills/review/lenses/` and the
  `ticket-review-output-format` skill — each carries `applies_to: [ticket]` in
  frontmatter and uses "ticket" prose extensively
- **The template** at `templates/ticket.md`, including a frontmatter key
  `ticket_id:` baked into every existing work-item file
- **The review subsystem** (`config-read-review.sh`) — review mode literal,
  `BUILTIN_TICKET_LENSES` array, three default constants, three config keys
- **Cross-skill references** in `create-plan/SKILL.md` and the cross-skill
  hierarchy assertion in `scripts/test-hierarchy-format.sh`

There is **no migration framework** today. The project's precedent for
handling user-facing breaks is a manual instruction in CHANGELOG (per v1.8.0
ephemeral-file move) or a clean break with "no backwards-compatibility shim"
when the surface was unreleased (per v1.9.0 `initialise` → `init` rename).
The ticket category is currently in the `Unreleased` section of CHANGELOG, so
the v1.9.0 precedent technically permits a clean break. However, a small set
of pre-release users (including this repo itself, which holds 29 work-item
files under `meta/tickets/`) will need their data migrated.

I recommend introducing a lightweight `/accelerator:migrate` skill — narrowly
scoped to applying ordered, idempotent shell-based migrations driven by a
state file (`meta/.migrations-applied`) — and seeding it with a single
migration that effects this rename. The infrastructure is small (one skill,
one state file, a `migrations/` directory), the rename's user-side
consequences are non-trivial enough to justify automation, and the
infrastructure pays back any time a future restructure (path move, frontmatter
key rename, template restructure) ships.

The phased plan below has six phases. Phases 1–3 are independent of the
migration question (the rename can land without it). Phase 4 introduces the
migration framework and the rename's migration. Phase 5 self-applies the
migration to this repo. Phase 6 releases the change.

## Detailed Findings

### 1. Skill category surface (the most visible rename)

#### Skills under `skills/tickets/`

7 skills exist, each declared as `name: <skill>` in SKILL.md frontmatter. All
become `/accelerator:<new-name>`:

| Current directory & SKILL name | New directory & SKILL name | File |
|---|---|---|
| `create-ticket` | `create-work-item` | `skills/tickets/create-ticket/SKILL.md:2` |
| `extract-tickets` | `extract-work-items` | `skills/tickets/extract-tickets/SKILL.md:2` |
| `list-tickets` | `list-work-items` | `skills/tickets/list-tickets/SKILL.md:2` |
| `refine-ticket` | `refine-work-item` | `skills/tickets/refine-ticket/SKILL.md:2` |
| `review-ticket` | `review-work-item` | `skills/tickets/review-ticket/SKILL.md:2` |
| `stress-test-ticket` | `stress-test-work-item` | `skills/tickets/stress-test-ticket/SKILL.md:2` |
| `update-ticket` | `update-work-item` | `skills/tickets/update-ticket/SKILL.md:2` |

Each SKILL.md additionally has:
- A `description:` line with "ticket"/"tickets" prose
- An `allowed-tools:` line with literal path fragments
  `${CLAUDE_PLUGIN_ROOT}/skills/tickets/scripts/*` (e.g.
  `create-ticket/SKILL.md:7`, `extract-tickets/SKILL.md:10`,
  `list-tickets/SKILL.md:8`, `refine-ticket/SKILL.md:9`,
  `review-ticket/SKILL.md:8`, `update-ticket/SKILL.md:9`). The
  `stress-test-ticket` skill does **not** reference the scripts directory
- Body prose using "ticket" / "the ticket file" / "this ticket" pervasively
  (see Section 6 of this doc)
- Cross-references to other slash commands (e.g.
  `refine-ticket/SKILL.md:419-426` enumerates `/create-ticket`,
  `/extract-tickets`, `/refine-ticket`, `/review-ticket`,
  `/stress-test-ticket`, `/update-ticket`, `/create-plan`)

#### Helper scripts under `skills/tickets/scripts/`

| Current | New |
|---|---|
| `ticket-next-number.sh` | `work-item-next-number.sh` |
| `ticket-read-field.sh` | `work-item-read-field.sh` |
| `ticket-read-status.sh` | `work-item-read-status.sh` |
| `ticket-update-tags.sh` | `work-item-update-tags.sh` |
| `ticket-template-field-hints.sh` | `work-item-template-field-hints.sh` |
| `test-ticket-scripts.sh` | `test-work-item-scripts.sh` |

Each script also contains internal references to the word "ticket":
`ticket-next-number.sh:4-9, :35-47, :55-67`,
`ticket-read-field.sh:4, :8, :15, :20-44, :70`,
`ticket-update-tags.sh:4-12, :14, :19, :42-73`,
`ticket-template-field-hints.sh:4-7, :11, :15, :19, :53` (this last one
literally calls the resolver with template key `ticket` at line 53).

#### Plugin manifest

`.claude-plugin/plugin.json:16` lists `"./skills/tickets/"`. After the rename:
`"./skills/work/"`.

### 2. Configuration & path resolution

The configuration system (analysed in detail at
`scripts/config-common.sh:22-29`, `scripts/config-read-path.sh:21-24`,
`scripts/config-read-value.sh:43-110`) reads YAML frontmatter from
`.claude/accelerator.md` (team) and `.claude/accelerator.local.md` (local),
with the local file overriding the team file. There is no `meta/config.toml`,
no `~/.accelerator/config`, no schema version, and no central defaults map —
**defaults are passed inline at every call site** as the second argument to
`config-read-path.sh <key> <default>`.

#### Path keys to rename

| Current key | Current default | New key | New default |
|---|---|---|---|
| `paths.tickets` | `meta/tickets` | `paths.work` | `meta/work` |
| `paths.review_tickets` | `meta/reviews/tickets` | `paths.review_work` | `meta/reviews/work` |

(Decision required: prefer `paths.work` matching the category name — the user
explicitly chose "work" as both the category and the default directory name.
The item type is "work-item" but storage and config keys follow the category.
This matches today's pattern where the category is `tickets` and the path key
is also `tickets`.)

#### Files where `paths.tickets` / `meta/tickets` defaults appear (must edit)

1. `scripts/config-read-path.sh:17` — doc comment listing recognised keys
2. `scripts/config-dump.sh:184` — `PATH_KEYS` array (`"paths.tickets"`)
3. `scripts/config-dump.sh:198` — `PATH_DEFAULTS` array (`"meta/tickets"`)
4. `scripts/config-dump.sh:182, :196` — same for `review_tickets` /
   `meta/reviews/tickets`
5. `skills/tickets/create-ticket/SKILL.md:22`
6. `skills/tickets/list-tickets/SKILL.md:23`
7. `skills/tickets/extract-tickets/SKILL.md:25-27`
8. `skills/tickets/update-ticket/SKILL.md:24`
9. `skills/tickets/refine-ticket/SKILL.md:24`
10. `skills/tickets/review-ticket/SKILL.md:25-26`
11. `skills/tickets/stress-test-ticket/SKILL.md:24`
12. `skills/tickets/scripts/ticket-next-number.sh:35`
13. `skills/planning/create-plan/SKILL.md:23`
14. `skills/config/init/SKILL.md:27, :29`
15. `skills/config/configure/SKILL.md:397, :415` (help-doc table + example
    YAML)
16. `CHANGELOG.md` (Unreleased section)
17. `README.md:86`

### 3. Review subsystem

`scripts/config-read-review.sh` is the most heavily ticket-dependent file
outside `skills/tickets/`. It carries:

- Mode literal `"ticket"` accepted as the first CLI argument
  (`scripts/config-read-review.sh:15` and validation at `:65-77`, `:140-159`)
- `DEFAULT_TICKET_REVISE_SEVERITY`, `DEFAULT_TICKET_REVISE_MAJOR_COUNT`,
  `DEFAULT_MIN_LENSES_TICKET` constants (`:28-31`)
- `BUILTIN_TICKET_LENSES` array of 5 lens names — the lens names themselves
  (`clarity`, `completeness`, `dependency`, `scope`, `testability`) do NOT
  contain "ticket" (`:53-60`)
- Config keys `review.ticket_revise_severity`,
  `review.ticket_revise_major_count`, `review.min_lenses_ticket` (`:85-86`,
  `:209-215`, `:414-419`, `:450-473`, `:501-507`, `:556-571`)
- A golden-output test fixture
  `scripts/test-fixtures/config-read-review/ticket-mode-golden.txt`

Lens SKILL.md files at `skills/review/lenses/{clarity,completeness,dependency,scope,testability}-lens/SKILL.md` declare:
- `description:` "Ticket review lens for…" (each `*-lens/SKILL.md:3`)
- `applies_to: [ticket]` in frontmatter
- Body prose using "ticket"/"tickets" densely (e.g. `clarity-lens/SKILL.md`
  matches at `:13, :32, :41, :63, :66, :102, :109, :113, :138, :144`;
  `scope-lens/SKILL.md` matches at `:12, :19, :22, :27, :32, :38, :40, :74,
  :75, :77, :80, :81, :85, :103, :105, :106, :114, :121, :133, :138`)

The review-output-format skill at
`skills/review/output-formats/ticket-review-output-format/SKILL.md:2-10`
becomes `work-item-review-output-format` — directory rename, `name:`
frontmatter rename, JSON-schema example update.

Review artifacts stamped with `type: ticket-review`
(`review-ticket/SKILL.md:347`) become `type: work-item-review`. The artifact
filename pattern remains `<stem>-review-<N>.md` (no ticket literal).

### 4. Template and schema

`templates/ticket.md` defines the canonical frontmatter shape. The
load-bearing fields:

- `ticket_id:` (`templates/ticket.md:2`) — embedded in every existing work-item
  file. Rename to `work_item_id:`. This is a breaking schema change.
- `type` enum `story | epic | task | bug | spike` (no rename — none of these
  values are "ticket")
- Body H1 `# NNNN: Title` (no rename)
- Comment "ticket number of the parent epic/story" at `templates/ticket.md:9`

The hardcoded fallback lists in
`skills/tickets/scripts/ticket-template-field-hints.sh:24-49` mirror the
template comment values. If any enum changes, both the template comments and
these fallbacks must be updated.

The template lookup key `ticket` is referenced at:
- `scripts/config-read-template.sh:6` (comment)
- `skills/tickets/scripts/ticket-template-field-hints.sh:53` (literal call:
  `"$PLUGIN_ROOT/scripts/config-read-template.sh" ticket`)
- `skills/config/configure/SKILL.md:189-197, :294-301, :485` (template enum)
- `README.md:184` (list of template keys)

After rename, the template file is `templates/work-item.md` and the lookup
key is `work-item`.

### 5. Filesystem structure today

```
meta/tickets/                  → meta/work/
meta/reviews/tickets/          → meta/reviews/work/
templates/ticket.md            → templates/work-item.md
skills/tickets/                → skills/work/
skills/tickets/scripts/        → skills/work/scripts/
skills/review/output-formats/ticket-review-output-format/
                               → skills/review/output-formats/work-item-review-output-format/
```

Filenames within `meta/tickets/` follow the pattern `NNNN-kebab-slug.md`
(verified at `ticket-next-number.sh:47`, `list-tickets/SKILL.md:109, :127`).
**No filename contains "ticket"**, so existing data files do not need
renaming when the directory moves — only the directory itself (and the
`ticket_id:` frontmatter field inside each file).

### 6. Test fixtures and evals

Eval files include:
- `skills/tickets/<skill>/evals/{benchmark,evals}.json`
- `skills/tickets/<skill>/evals/files/<scenario>/ticket.md` — many of these
  exist (e.g. all of `refine-ticket/evals/files/scenario-2/ticket.md`,
  `scenario-3/ticket.md`, ...)
- `skills/tickets/refine-ticket/evals/files/scenario-11{b,c}/tickets/0001-stub-ticket-1.md`
  through `…-35.md` — directory `tickets/` and filenames
  `*-stub-ticket-N.md`
- `skills/tickets/stress-test-ticket/evals/files/<scenario>/ticket.md`
- `skills/review/lenses/<lens>/evals/files/<scenario>/ticket.md`
- `skills/review/lenses/clarity-lens/evals/files/clean-ticket/ticket.md` (a
  scenario directory named `clean-ticket`)

Each `evals.json` / `benchmark.json` references those fixture paths; the
references must be updated when fixtures are renamed.

### 7. Documentation

`README.md` references "ticket" at lines 86, 184, 244-273 (the entire
**Ticket Management** H2 section), 315, 337, 350. The structural pattern
(skill table, ASCII flow diagram, lens table) is identical to other
sections — the rename is mostly find-and-replace.

`CHANGELOG.md` has the entire ticket-management feature set under the
`Unreleased` section (lines 5-98). Per the v1.9.0 `initialise` → `init`
precedent (`CHANGELOG.md:198-204`), a rename of an unreleased surface ships
as a single `### Changed` bullet without a backwards-compatibility shim.
That precedent is a good fit here: most users are still on a release that
predates the ticket category. Pre-release adopters need migration help, but
that's what the migration skill is for.

`skills/config/configure/SKILL.md` and `skills/config/init/SKILL.md` are
substantial — configure especially, because it is the user-facing
documentation of every config key (`configure/SKILL.md:132, :161, :189-197,
:213-214, :225-227, :294-301, :316-318, :393-397, :415, :436, :485` —
roughly 12 reference points).

`skills/planning/create-plan/SKILL.md:22-23, :52-53` references
`paths.tickets` and example `@tickets/eng-1234.md`.

`scripts/test-hierarchy-format.sh:21-22` enforces a byte-exact match between
two canonical-tree fences in `list-tickets/SKILL.md:209-214` and
`refine-ticket/SKILL.md:353-365`. Both fences need updating in lockstep.

### 8. Naming-alternatives history

The original research that introduced the term "ticket" is
`meta/research/codebase/2026-04-08-ticket-management-skills.md`. A grep through
research/plans/decisions for `work-item|work item|backlog|alternative.*name`
returned only three docs — and in all three, "work item" appears as a
descriptive synonym, not as a considered alternative term. The naming
choice was made by the author without an explicit comparison against
alternatives. There is no ADR for the term.

This means the rename is uncovering a terminology decision that was never
formally debated, and we should write an ADR for it now.

### 9. Files I did not exhaustively read

- All 7 plan documents covering the ticket initiative
  (`meta/plans/2026-04-{08,19,21,22,24,24}-…`) — these will contain "ticket"
  prose but are historical artifacts; per project convention historical
  research/plan/decision documents are NOT retroactively edited
- `meta/reviews/plans/*ticket*` — review artifacts of plans, also historical
- The 29 work-item files in `meta/tickets/` — these will need the
  `ticket_id` → `work_item_id` frontmatter migration (handled by the
  migration script, not by hand) and the directory rename

## Migration Skill: Should we build it?

### Existing precedents in this codebase

- `skills/config/init/SKILL.md` is idempotent: `mkdir -p`,
  `[ -d {dir} ]` checks, `.gitignore` line-presence checks, paired
  `created | already exists` reporting (`init/SKILL.md:91-95, :105-125`).
  This is the canonical "safe to re-run" pattern but does *not* perform
  any state transformations.
- The CHANGELOG carries one prior breaking-change with an `rm -rf`
  instruction (v1.8.0, `CHANGELOG.md:222-234`) and one pre-release rename
  with no shim (v1.9.0, `CHANGELOG.md:198-204`).
- `ticket-next-number.sh:43-55` is the project's idiom for reasoning about
  filesystem state (existence check, glob over numbered files, error to
  stderr, continue on missing).
- Pattern-finder confirmed there is no migration framework, no schema
  versioning, no `meta_version` / `schema_version` field anywhere.

### Why a migration skill is justified for this rename

1. **Pre-release users have data**. This repo itself has 29 work-item files
   under `meta/tickets/`. Any pre-release adopter (e.g. an Atomic Innovation
   internal user trialling v1.19.0-pre.x) will be in the same position. A
   manual `mv meta/tickets meta/work` is shallow but **`ticket_id:` →
   `work_item_id:` requires editing every file**, and updating
   `.claude/accelerator.md` to rename a pinned `paths.tickets` is more
   error-prone than `mv`.
2. **The rename is the second pre-release breaking change in three releases**
   (initialise→init was the first). The pattern is recurring; the next
   restructure will benefit from the same scaffolding.
3. **Idempotency is non-negotiable**. Without a state file, users can't tell
   whether a migration has already run. With a state file it becomes a safe,
   re-runnable command.
4. **The infrastructure is small**. One skill, one state file, a
   `migrations/` directory, and a registry. The first migration script is
   the rename itself; subsequent migrations are added one at a time as
   needed.

### Why we might NOT build it

- The project's prior precedent is manual changelog instructions, not
  automation. Adding infrastructure for a single rename is borderline
  speculative generalisation.
- The number of pre-release users with data may be small (one — this repo).
- A migration skill introduces a new state file (`meta/.migrations-applied`),
  which itself becomes part of the meta-directory contract.

### Proposed shape (if building)

**Skill**: `skills/config/migrate/SKILL.md` — sits in the config category
alongside `init` and `configure`. Slash command `/accelerator:migrate`.

**State file**: `meta/.migrations-applied` — a plain newline-delimited list
of migration IDs that have been applied. Each ID is the migration filename
without extension. Example:

```
0001-rename-tickets-to-work
```

**Migration registry**: `skills/config/migrate/migrations/` — each migration
is a discrete shell script with a leading 4-digit ID, like
`0001-rename-tickets-to-work.sh`. The script must be idempotent on its own
(state file is a belt; idempotency is the suspenders).

**Migration contract** (each script):
- Reads the project root from a `$PROJECT_ROOT` env var the skill exports
- Resolves any path through `config-read-path.sh` (so user-pinned overrides
  are respected)
- Reports each step with paired `applied | already applied` labels
- Exits 0 on success, non-zero on any failure (skill aborts the run)
- Supports a `DRY_RUN=1` env var that prints what would change without
  modifying anything

**Skill behaviour**:
1. Read `meta/.migrations-applied` (create empty if missing)
2. Glob `skills/config/migrate/migrations/[0-9][0-9][0-9][0-9]-*.sh` in
   sorted order
3. Filter out IDs already in the state file
4. For each remaining migration, run it and append its ID to the state
   file on success
5. Report a per-migration status table at the end
6. Default: dry-run. User passes `--apply` to actually mutate.

**First migration** (`0001-rename-tickets-to-work.sh`) does the following,
each step idempotent:
- If `meta/tickets` exists and `meta/work` does not, `mv meta/tickets
  meta/work` (or honour `paths.tickets` override if set)
- If `meta/reviews/tickets` exists and `meta/reviews/work` does not, move it
- For every `meta/work/*.md`, if frontmatter contains `ticket_id:`, sed it
  to `work_item_id:`
- For `.claude/accelerator.md` and `.claude/accelerator.local.md`, if
  `paths.tickets:` is present, rename to `paths.work:`. Same for
  `paths.review_tickets:` → `paths.review_work:` and
  `review.ticket_revise_severity:` → `review.work_item_revise_severity:`
  (etc.)

The skill is read-only at the directories it does not own (e.g. it never
touches files outside `meta/` and `.claude/accelerator*.md`).

### Decision

I recommend **building the migration skill as Phase 4 of the rename**, with
`0001-rename-tickets-to-work.sh` as the first registered migration. This
keeps the skill's scope sharp (a real migration motivates its first version)
and lets future restructures land as additional small migration scripts
rather than ad-hoc instructions in CHANGELOG.

A separate ADR should record the design (state file, registry layout,
idempotency contract, dry-run default).

## Phased Plan

### Phase 0 — Decisions (ADRs)

- **ADR**: "Use 'work' / 'work-item' terminology" — record the rename and
  why "ticket" is being retired (service-desk connotations vs neutral
  product-development term).
- **ADR**: "Migration framework for the meta directory" — record the
  state-file design, the migration ID convention, idempotency contract,
  dry-run default. Only write this ADR if Phase 4 is approved.

### Phase 1 — Rename plugin/manifest, config defaults, scripts

- `.claude-plugin/plugin.json:16` `./skills/tickets/` → `./skills/work/`
- Rename directory `skills/tickets/` → `skills/work/`
- Rename each skill subdirectory and update each `SKILL.md`'s `name:`
  frontmatter and `description:` line
- Rename helper scripts under `skills/work/scripts/` and update their
  internal references and error messages
- `scripts/config-read-path.sh:17` (comment), `:6` of
  `scripts/config-read-template.sh`, and `scripts/config-dump.sh:184, :198,
  :182, :196` to use new keys/defaults
- Update every `config-read-path.sh tickets meta/tickets` call site to
  `config-read-path.sh work meta/work` (15 occurrences listed in
  Section 2)
- Update `scripts/config-read-review.sh`: mode literal `ticket` →
  `work-item`, rename `BUILTIN_TICKET_LENSES`, three `DEFAULT_TICKET_*`
  constants, three config keys
- Rename `scripts/test-fixtures/config-read-review/ticket-mode-golden.txt`
  → `work-item-mode-golden.txt` and regenerate
- Update `scripts/test-hierarchy-format.sh:21-22` paths

### Phase 2 — Update review subsystem

- Lens SKILL.md files (`{clarity,completeness,dependency,scope,testability}-lens/SKILL.md`):
  update `description:`, `applies_to:`, body prose
- Output-format skill: rename
  `skills/review/output-formats/ticket-review-output-format/` →
  `work-item-review-output-format/` and update its SKILL.md `name:`,
  schema example, prose
- Update `review-work-item/SKILL.md` (formerly `review-ticket/SKILL.md`):
  artifact `type: ticket-review` → `type: work-item-review`

### Phase 3 — Update template, evals, fixtures, prose, docs

- `templates/ticket.md` → `templates/work-item.md`; rename `ticket_id:` →
  `work_item_id:`; update template comments
- Update `ticket-template-field-hints.sh` (now
  `work-item-template-field-hints.sh`) hardcoded fallbacks to mirror the new
  template, and change template lookup key `ticket` → `work-item`
- Rename eval fixture files `ticket.md` → `work-item.md` (or keep as
  `ticket.md` since fixture filenames are private to evals — decide based on
  whether eval runners care; lowest-risk choice is to rename for consistency)
- Update `evals.json` / `benchmark.json` references to the renamed fixtures
- Rename scenario directories like `clean-ticket/` → `clean-work-item/`,
  `scenario-11b/tickets/` → `scenario-11b/work-items/`, etc.
- Update prose in all SKILL.md files (descriptions, body, examples)
  including planning/create-plan and config/configure, config/init
- Rename `meta/tickets/` → `meta/work/` *in this repo* (handled by Phase 4
  migration if the framework is built; otherwise manual)
- Rename `meta/reviews/tickets/` → `meta/reviews/work/`
- Update `README.md` (lines 86, 184, 244-273, 315, 337, 350)
- Update `CHANGELOG.md` Unreleased section to reflect the renamed surface
  (the user-facing skills appear under their new names; do NOT retroactively
  rename historical released entries)

### Phase 4 — Migration framework (recommended)

- Create `skills/config/migrate/SKILL.md` per the design in the previous
  section
- Create `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh`
- Add to `.claude-plugin/plugin.json` skill list (already covered if `config`
  category is listed)
- Add documentation: README has a small "Migrations" subsection;
  `configure/SKILL.md` mentions `/accelerator:migrate` in the same context
  as `/accelerator:init`

### Phase 5 — Self-apply migration

- Run `/accelerator:migrate --apply` on this repo
- Verify `meta/tickets/` → `meta/work/`, `meta/reviews/tickets/` →
  `meta/reviews/work/`, `ticket_id:` → `work_item_id:` in all 29 work-item
  files, `meta/.migrations-applied` contains `0001-rename-tickets-to-work`
- Update any historical research/plan/decision/review documents that
  reference `meta/tickets/` paths *only if those paths are now broken
  cross-references* — do NOT rewrite historical prose

### Phase 6 — Release

- CHANGELOG entry under `### Changed`: a single bullet noting the rename and
  pointing to `/accelerator:migrate` for upgraders
- If Phase 4 not built, CHANGELOG includes manual `mv` and `sed` commands as
  per the v1.8.0 precedent
- Bump version per project convention. The v1.9.0 precedent set
  unreleased-rename = minor; the same applies here. Bump to `1.19.0` (or
  next minor)

## Risks

1. **Eval fixture renames cascade**. If eval fixture filenames are bound to
   strings inside `evals.json`, rename mistakes will surface as eval
   failures only at run-time. Mitigation: run `mise run test` after each
   phase.
2. **The `BUILTIN_TICKET_LENSES` array change is brittle**. The lens NAMES
   themselves (clarity, completeness, etc.) don't change, but every
   reference to the array variable must be updated. Mitigation: rename the
   variable in one commit and run `bash -n` plus the existing
   `scripts/test-config.sh` and `scripts/test-fixtures` golden tests.
3. **User-pinned `.claude/accelerator.md` config**. If a user has
   `paths.tickets:` pinned and we change the default key to `paths.work`,
   their pin silently stops working — `paths.tickets` becomes an unread
   key, and the new code reads `paths.work` (default `meta/work`).
   Mitigation: the migration script renames the YAML key inside their
   config file. Without the migration: a manual instruction in CHANGELOG.
4. **`ticket_id:` field inside historical work-items (29 files in this
   repo)**. Mitigation: the migration script `sed`s the field name. Without
   it: a one-line `find ... -exec sed` instruction in CHANGELOG.
5. **External tooling that reads `ticket_id:`**. Unlikely but worth
   noting — third-party scripts grepping for `ticket_id:` would break.
   Mitigation: changelog flag, no shim.
6. **Test golden files**. Several golden output files
   (`scripts/test-fixtures/config-read-review/ticket-mode-golden.txt`)
   embed the literal "ticket" string. Mitigation: regenerate after the
   rename.

## Code References

- `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/ticket-management/.claude-plugin/plugin.json:16` — plugin manifest skill listing
- `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/ticket-management/scripts/config-read-path.sh:17` — recognised path keys (doc comment)
- `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/ticket-management/scripts/config-read-review.sh:15, :28-31, :53-60, :85-86, :140-159, :209-215, :414-419` — review-mode literals and constants
- `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/ticket-management/scripts/config-read-template.sh:6` — template key list (doc comment)
- `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/ticket-management/scripts/config-dump.sh:182, :184, :196, :198` — `PATH_KEYS`/`PATH_DEFAULTS` arrays
- `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/ticket-management/scripts/test-hierarchy-format.sh:21-22` — hierarchy assertion paths
- `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/ticket-management/skills/tickets/` — entire directory
- `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/ticket-management/skills/review/lenses/{clarity,completeness,dependency,scope,testability}-lens/SKILL.md` — lens descriptions and `applies_to`
- `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/ticket-management/skills/review/output-formats/ticket-review-output-format/SKILL.md` — output-format skill
- `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/ticket-management/skills/planning/create-plan/SKILL.md:22-23, :52-53` — cross-skill ticket references
- `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/ticket-management/skills/config/init/SKILL.md:27, :29` — path defaults
- `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/ticket-management/skills/config/configure/SKILL.md:132, :161, :189-197, :213-214, :225-227, :294-301, :316-318, :393-397, :415, :436, :485` — config docs
- `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/ticket-management/templates/ticket.md` — canonical template
- `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/ticket-management/README.md:86, :184, :244-273, :315, :337, :350` — user docs
- `/Users/tobyclemson/Code/organisations/atomic/company/accelerator/workspaces/ticket-management/CHANGELOG.md:5-98, :198-204, :222-234` — Unreleased and prior breaking-change precedents

## Architecture Insights

- The plugin's configuration system has **no central defaults map** — every
  call site passes its own default. This duplicates `meta/tickets` across 15
  files and makes any default change a coordinated edit. A future cleanup
  could centralise defaults; out of scope for this rename, but worth
  flagging.
- The plugin auto-discovers skills from category directories listed in
  `plugin.json`. The slash-command namespace `accelerator:` is implicit
  (derived from `plugin.json:2 name`). Renaming a skill is a SKILL.md `name:`
  edit; renaming a category is a directory rename plus one
  `plugin.json` line.
- Existing breaking-change precedents (v1.8.0 and v1.9.0) prefer "no
  shim, document the migration step in CHANGELOG" over backwards-compatible
  aliases. A migration skill formalises that pattern without contradicting
  it — CHANGELOG still describes the breaking change; the migration script
  is the automation that applies it.
- The review subsystem uses string-based discriminators (`mode=ticket`,
  `applies_to: [ticket]`, artifact `type: ticket-review`) rather than
  type-codes or enums. Renaming requires touching all of them in
  lockstep; there is no compiler to catch mismatches.

## Historical Context

- `meta/research/codebase/2026-04-08-ticket-management-skills.md` — original research
  introducing the term "ticket". Naming alternatives were not explicitly
  considered.
- `meta/research/codebase/2026-03-18-meta-management-strategy.md` — establishes the
  filesystem-as-shared-memory principle that the rename respects.
- `meta/decisions/ADR-0016-userspace-configuration-model.md` — confirms the
  config layer's intentional simplicity (shell-based YAML extraction, no
  central defaults map). The rename does not challenge this; the migration
  framework can extend it without violating its principles.
- `meta/decisions/ADR-0018-init-skill-for-repository-bootstrap.md` —
  precedent for an idempotent setup skill. The proposed migrate skill
  follows the same idiom (paired status reporting, safe to re-run).
- `CHANGELOG.md:198-204` (v1.9.0 `initialise` → `init`) — precedent for
  unreleased renames without a backwards-compatibility shim.
- `CHANGELOG.md:222-234` (v1.8.0 ephemeral-file move) — precedent for
  manual-instruction migrations in CHANGELOG.

## Related Research

- `meta/research/codebase/2026-04-08-ticket-management-skills.md` — original
  ticket-skill design
- `meta/research/codebase/2026-03-18-meta-management-strategy.md` — meta directory
  philosophy
- `meta/research/codebase/2026-03-22-skill-customisation-and-override-patterns.md` —
  user-side override patterns
- `meta/research/codebase/2026-03-28-initialise-skill-requirements.md` — closest
  precedent for the proposed migrate skill's behaviour

## Open Questions

1. **Config key shape**: `paths.work` (matching category) or
   `paths.work_items` (matching item type)? Recommendation: `paths.work` to
   match the precedent (`paths.tickets` matches the category `tickets`).
2. **Should the skill's slash command pluralise or singularise the action?**
   `/accelerator:create-work-item` (singular item) vs
   `/accelerator:create-work-items` (matches `extract-work-items` plural).
   Today the pattern is mixed — `extract-tickets` is plural, `create-ticket`
   singular, dependent on whether the operation is bulk or individual. Keep
   this convention: `/accelerator:create-work-item`,
   `/accelerator:extract-work-items`, `/accelerator:list-work-items`,
   `/accelerator:refine-work-item`, `/accelerator:review-work-item`,
   `/accelerator:stress-test-work-item`, `/accelerator:update-work-item`.
3. **Build the migrate skill, or punt?** Recommendation: build it as Phase 4.
4. **Apply this rename before or after a 1.x release?** Recommendation:
   ship it in `1.19.0` since the ticket category is still in `Unreleased`
   per CHANGELOG. This preserves the v1.9.0 precedent for unreleased
   renames.
5. **Lens directory naming**: `clarity-lens` etc. don't contain "ticket" —
   no rename needed. But `applies_to: [ticket]` does. Confirm: just edit
   the `applies_to:` field in each lens. Yes.
