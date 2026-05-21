---
date: 2026-05-21T18:47:09+01:00
author: Toby Clemson
git_commit: 64eca1bf99c3b311862da9df1baf1095b43ca4a7
branch: main
repository: accelerator
work_item_id: "0064"
topic: "Canonicalise `work_item_id` and `author` frontmatter field names"
tags: [research, codebase, frontmatter, schema, migration, visualiser, plan, templates]
status: complete
last_updated: 2026-05-21
last_updated_by: Toby Clemson
---

# Research: Canonicalise `work_item_id` and `author` Frontmatter Field Names

**Date**: 2026-05-21T18:47:09+01:00
**Researcher**: Toby Clemson
**Git Commit**: 64eca1bf99c3b311862da9df1baf1095b43ca4a7
**Branch**: main
**Repository**: accelerator

## Research Question

For story 0064, map out the complete consumer surface, migration precedent,
and inherited context needed to plan the bundled rename of plan frontmatter
`work-item:` → `work_item_id:` and research / RCA frontmatter `researcher:` →
`author:`. The story bundles producers, consumers (visualiser server + frontend,
helper scripts), and a corpus migration into a single releasable change.

## Summary

Outside `meta/`, the rename surface is small and well-bounded:

- **`work-item:` (frontmatter key) producers** — exactly one template
  (`templates/plan.md:5`). The producing skill (`skills/planning/create-plan/SKILL.md`)
  does **not** emit `work-item:` inline; it relies on the template.
- **`work-item:` (frontmatter key) consumers** — exactly one string-literal read
  site in the visualiser server (`server/src/frontmatter.rs:326`). Everything
  else (`indexer.rs`, `patcher.rs`, `api/related.rs`) flows from the Rust
  identifier `work_item_refs` populated by that read, plus inline test fixtures
  that contain the YAML key.
- **`work-item:` in the frontend** — *no* string-literal reads of `"work-item"`.
  The three files named in the story (`api/types.ts`, `api/work-item.ts`,
  `routes/kanban/WorkItemCard.tsx`) do not reference the frontmatter key at all;
  they either expose Rust-mirror struct types (camelCase `workItemId` /
  `workItemRefs`) or use filename-derived IDs. **This is a notable
  story-vs-reality discrepancy** — the story over-scopes the frontend surface.
- **`researcher:` producers** — two templates (`templates/codebase-research.md:3`,
  `templates/rca.md:3`). Producer skills (`skills/research/research-codebase/SKILL.md`,
  `skills/research/research-issue/SKILL.md`) do not emit `researcher:` inline.
- **`researcher:` consumers** — **none**. Zero references in the visualiser
  server, the frontend, helper scripts, or any other skill. The migration must
  still rewrite the corpus, but no production code reads this field today.

The migration precedent (`0005-rename-work-item-type-to-kind.sh`) is a clean fit,
with two important deltas: (a) story 0064 walks **three** corpora (plans,
codebase-research, RCA — where RCA lives under `paths.research_issues`, not a
separate `paths.rca`), not one, and (b) it also rewrites `.accelerator/templates/*.md`
content, which **no existing migration has done before** — 0004 only *renames*
template files. The story's claim that 0005's `paths-override` fixture is the
precedent for template-content rewriting is a mischaracterisation; the precedent
0005 actually demonstrates is `config-read-path.sh`-based corpus resolution.

Next available migration number is **0006**. The pattern is to mirror 0005 line
by line, but with multi-path iteration and an additional template-content rewrite
pass.

## Detailed Findings

### Producer surface (templates + producing skills)

**Templates** — the only on-disk YAML emitters. Each affected template has
both a frontmatter key edit and (for research/RCA) a body-label edit:

- `templates/plan.md:5` — frontmatter `work-item: "{work-item reference, if any}"` → `work_item_id: "{work-item reference, if any}"`. No body label to update.
- `templates/codebase-research.md:3` — frontmatter `researcher: [Git author]` → `author: [Git author]`.
- `templates/codebase-research.md:17` — body label `**Researcher**: [Researcher name from thoughts status]` → `**Author**: [Author name from thoughts status]` (also normalises the placeholder text from "Researcher name" → "Author name").
- `templates/codebase-research.md:11` — placeholder value `last_updated_by: [Researcher name]` → `last_updated_by: [Author name]` (value text only; the field name `last_updated_by` is unchanged).
- `templates/rca.md:3` — frontmatter `researcher: [Git author]` → `author: [Git author]`.
- `templates/rca.md:17` — body label `**Researcher**: [Researcher name]` → `**Author**: [Author name]`.
- `templates/rca.md:11` — placeholder value `last_updated_by: [Researcher name]` → `last_updated_by: [Author name]`.

**Producing skills** — none of them emit the renamed keys inline. The
following SKILL.md files render the template via the template-resolution
mechanism (`config-read-template.sh`) and do not duplicate the key in their
own prose:

- `skills/planning/create-plan/SKILL.md` — no `work-item:` matches.
- `skills/research/research-codebase/SKILL.md` — no `researcher:` matches.
- `skills/research/research-issue/SKILL.md` — no `researcher:` matches.

This means the producer rename is fully captured by the three template edits
above. Cross-check ripgrep over `skills/**/SKILL.md` confirms no other
producer surface.

### Visualiser consumer surface

#### Rust server (`skills/visualisation/visualise/server/src/`)

| File | Line(s) | What |
|---|---|---|
| `frontmatter.rs` | 297-302 | Doc comment describing `work-item:` semantics |
| `frontmatter.rs` | **326** | **Only production read of `"work-item"` (HashMap key lookup)** |
| `frontmatter.rs` | 325 | Inline comment `// work-item: wins over legacy ticket:` |
| `frontmatter.rs` | 441, 455, 523 | Inline YAML test fixtures containing `work-item: "0042"` |
| `indexer.rs` | 174 | Struct field `pub work_item_refs: Vec<String>` (Rust identifier — not the frontmatter key) |
| `indexer.rs` | 212-216 | Doc comment + struct field `work_item_refs_by_target` reverse index |
| `indexer.rs` | 269, 300-309 | `rescan()` populates the reverse index from `entry.work_item_refs` |
| `indexer.rs` | 596-608 | `pub async fn work_item_refs_by_id(&self, id: &str)` reader |
| `indexer.rs` | 610-643 | `declared_outbound()` uses `entry.work_item_refs` |
| `indexer.rs` | 934-983 | Incremental update helpers (`update_/remove_…work_item_refs_by_target`) |
| `indexer.rs` | 1025 | Call site of `frontmatter::read_ref_keys` from `build_entry` |
| `indexer.rs` | 2585, 2641, 2660, 2687 | Inline YAML test fixtures containing `work-item:` |
| `patcher.rs` | 259-267, 407-415 | Test fixtures + assertions that `work-item:` passes through `patch_status` unchanged. **Not** a production consumer. |
| `api/related.rs` | 78 | Doc comment only; the endpoint queries the prebuilt reverse index via `indexer.work_item_refs_by_id(id)` — no string-literal read of `"work-item"` |

**Key insight**: the only Rust source line that needs to change for behavioural
correctness is `frontmatter.rs:326`. Every other Rust mention is either a
comment (cosmetic), a struct-field name (identifier — must not be auto-renamed
without conscious decision), or an inline YAML test fixture (must be updated to
keep tests realistic).

Sanity-check ripgrep confirmed **zero references to `researcher:`** anywhere
under `server/src/`.

#### Frontend (`skills/visualisation/visualise/frontend/src/`)

| File | What it references |
|---|---|
| `api/types.ts:71` | `workItemId: string \| null` — camelCase wire mirror of the Rust struct field `IndexEntry.work_item_id`. **This is the filename-derived ID**, *not* the frontmatter cross-ref key. |
| `api/types.ts:75` | `workItemRefs: string[]` — camelCase wire mirror of `IndexEntry.work_item_refs`. |
| `api/work-item.ts` | Reads filename via regex (`/^(\d+)-/`) and `entry.frontmatter['status']` only. **Does not read `work-item:` or `researcher:`**. |
| `routes/kanban/WorkItemCard.tsx` | Reads `entry.relPath` and `entry.frontmatter['kind']` only. **Does not read `work-item:` or `researcher:`**. |
| `routes/library/template-tier.ts:37` | Has `'work-item': 'work-items'` as a template-stem → doc-type-key map entry. Unrelated to the frontmatter field; this maps the template filename stem `work-item.md` to the doc-type key `'work-items'`. |
| `e2e/legacy-schema.spec.ts:4` | A doc-comment mentioning `"no work_item_id:"` — forward-looking; becomes correct after 0064 lands. |

**Bottom line**: the frontend has **no string-literal frontmatter-key reads** to
update for either rename. The story over-scopes the frontend surface. The wire
contract (camelCase struct mirrors) is decoupled from the YAML key name —
unless the planning session deliberately chooses to also rename the Rust
struct field `IndexEntry.work_item_id` (which would propagate to `workItemId`
in TypeScript), no frontend code change is required.

#### Visualiser test fixtures with the literal YAML key

Beyond the inline fixtures in `frontmatter.rs`/`indexer.rs`/`patcher.rs`, the
on-disk fixture corpus contains live `work-item:` lines:

- `server/tests/fixtures/meta/work/0001-first-work-item.md:4`
- `server/tests/fixtures/meta/work/0002-second-work-item.md:4`
- `server/tests/fixtures/meta/work/0003-third-work-item.md:3`
- `server/tests/fixtures/meta/work/0005-sse-test-work-item.md:4`
- `server/tests/fixtures/meta/work/0006-conflict-test-work-item.md:4`

These must be rewritten alongside the production-code change to keep tests
realistic. They are also fair game for the `rg -n 'work-item:' …` AC grep —
the AC's search roots in the story already include `skills/`, so these fixtures
fall in scope.

### Migration framework + 0005 precedent

#### Migration runner (`skills/config/migrate/scripts/run-migrations.sh`)

- Discovery glob: `[0-9][0-9][0-9][0-9]-*.sh` at `MIGRATIONS_DIR`, sorted
  lexicographically (`run-migrations.sh:87-93`).
- Driver exports `PROJECT_ROOT`, `CLAUDE_PLUGIN_ROOT`, `ACCELERATOR_MIGRATION_MODE=1`
  per migration (`run-migrations.sh:177-208`).
- State tracking at `.accelerator/state/migrations-applied` and
  `.accelerator/state/migrations-skipped` (`run-migrations.sh:13-14, 204-205`).
- Clean-tree guard scoped to `meta/`, `.claude/accelerator*.md`, `.accelerator/`
  (`run-migrations.sh:42-70`); bypassed by `ACCELERATOR_MIGRATE_FORCE=1` in tests.
- `# DESCRIPTION:` second-line convention consumed by preview banner
  (`run-migrations.sh:165-167`).

#### Next available migration number

Current bundled migrations under `skills/config/migrate/migrations/`:

- `0001-rename-tickets-to-work.sh`
- `0002-rename-work-items-with-project-prefix.sh`
- `0003-relocate-accelerator-state.sh`
- `0004-restructure-meta-research-into-subject-subcategories.sh`
- `0005-rename-work-item-type-to-kind.sh`

**Next available number: `0006`.**

#### 0005's shape — what 0064 should mirror

`skills/config/migrate/migrations/0005-rename-work-item-type-to-kind.sh`:

1. **Header** (lines 1-10): shebang, `# DESCRIPTION:`, `set -euo pipefail`,
   `PLUGIN_ROOT` resolution, source `config-common.sh`, `atomic-common.sh`,
   `log-common.sh`.
2. **Path discovery** (lines 12-29): `bash "$PLUGIN_ROOT/scripts/config-read-path.sh" work`
   to honour `paths.work` overrides; defensive empty-string and dangerous-path
   guards.
3. **Missing-directory handling** (lines 31-35): warn via `log_warn`, exit 0 so
   the migration is still recorded as applied.
4. **Frontmatter key rewrite** (lines 41-54): anchor `^type:`; if `^kind:` also
   present (partial prior run), drop `^type:` and keep `kind:`; otherwise
   `sed 's/^type:/kind:/'`. Value bytes (quoting, lists, etc.) preserved
   verbatim because substitution stops at the colon.
5. **Body label rewrite** (lines 57-69): same shape for `**Type**:` →
   `**Kind**:`. **0064 needs this for the `researcher:` rename only**.
   Confirmed: `templates/codebase-research.md:17` and `templates/rca.md:17` both
   emit `**Researcher**: …` body labels (and these propagate into every active
   research / RCA artefact). The migration must twin-pass these: rewrite
   `**Researcher**:` → `**Author**:` across the same corpora as the
   frontmatter `researcher:` → `author:` pass. `templates/plan.md` has no
   `**Work-Item**:` body label, so the `work-item:` → `work_item_id:` rename
   is frontmatter-only.
6. **Per-file atomic write** via `atomic_write` (`scripts/atomic-common.sh:16-32`).
7. **Single summary line on stdout** + warnings to stderr via `log_warn`.

#### Test fixture taxonomy (`skills/config/migrate/scripts/test-fixtures/0005/`)

Each scenario is a self-contained pseudo-repo:

- `default-layout/` — pre-migration canonical: both frontmatter and body label
  present.
- `legacy-adr-task/` — frontmatter only, no body label.
- `partial-prior-run/` — both old + new frontmatter keys, matching values.
- `partial-prior-run-divergent/` — both keys, different values → expect
  warning, keep new, drop old.
- `partial-prior-run-body-label/` + `partial-prior-run-body-label-divergent/` —
  analogous for body labels.
- `body-label-only/` — frontmatter already migrated, body still legacy.
- `paths-override/` — userspace `paths.work: docs/work` override; asserts work
  dir is honoured and `meta/work/` is **not** created.
- `paths-override-missing/` — override points to non-existent dir; warns + exit 0.
- `empty-work-dir/` — override to empty dir with only `.gitkeep`.

**Important note on the story's "paths-override" precedent claim**: 0005's
`paths-override` fixture validates that the migration honours `paths.work` via
`config-read-path.sh`, not that it rewrites template content under
`.accelerator/templates/`. **No existing migration rewrites template content.**
0004 has a `_rename_user_template_file_if_present` helper
(`0004-restructure-meta-research-into-subject-subcategories.sh:419-430`) that
renames `<paths.templates>/research.md` → `<paths.templates>/codebase-research.md`
via `mv` — file path only, not contents. 0064 will be the **first** migration
to rewrite content inside user-overridden templates.

#### Test runner (`skills/config/migrate/scripts/test-migrate.sh`)

- Single 1454-line script covering all migrations; 0005 block at lines
  1255-1454.
- Per-scenario pattern: `setup_NNNN_repo`, `run_NNNN_driver` (invokes the
  driver with `ACCELERATOR_MIGRATIONS_DIR=$ONLY_NNNN_DIR` and
  `ACCELERATOR_MIGRATE_FORCE=1`), then `assert_contains` /
  `assert_not_contains` / `assert_dir_not_exists` / `tree_hash`
  idempotency checks.
- No expected-output golden files — assertions are inline string literals.
- 0064 should append a new `=== Migration 0006: ... ===` block following the
  0005 layout.

### Config-resolver coverage for the three corpora

From `scripts/config-defaults.sh` lines 26-64, the relevant `paths.*` keys are:

| Key | Default | Used for |
|---|---|---|
| `paths.plans` | `meta/plans` | Plan artefacts (the `work-item:` rewrite target corpus) |
| `paths.research_codebase` | `meta/research/codebase` | Codebase research (the `researcher:` rewrite target corpus) |
| `paths.research_issues` | `meta/research/issues` | **Issue/RCA research** (the second `researcher:` corpus — RCA artefacts live here, not under a separate `paths.rca`) |
| `paths.templates` | `.accelerator/templates` | Userspace template overrides |

**There is no `paths.rca` key.** RCA artefacts share the path with issue
research under `paths.research_issues`. This is confirmed by
`agents/documents-locator.md:35` ("`research_issues` — issue / RCA research
documents") and `skills/config/configure/SKILL.md:390`. There is also no
`templates.rca` key in `TEMPLATE_KEYS` (`config-defaults.sh:66-73`) — though
the `templates/rca.md` template file does exist and is loaded by name. RCA
templates appear to be hand-installable under `.accelerator/templates/rca.md`
without a dedicated `templates.rca:` key (mentioned in `CHANGELOG.md:193`).

**Implication for the migration**: the 0006 migration must iterate over (at
least) three configured paths: `paths.plans`, `paths.research_codebase`,
`paths.research_issues`. Each pass should resolve its directory via
`config-read-path.sh <bare-key>` so userspace overrides flow through.

For the template-override pass, the migration must:

1. Resolve `paths.templates` via `config-read-path.sh templates` (default
   `.accelerator/templates`).
2. Skip if the directory does not exist (`log_warn` + count as 0 rewrites).
3. Otherwise iterate over the three template filenames (`plan.md`,
   `codebase-research.md`, `rca.md`) — only those, since other templates are
   out of scope — and rewrite each one's frontmatter key with the same dual-
   presence guard as the corpus pass.

### Userspace template override mechanism

From `scripts/config-read-template.sh:11-15` — three-tier resolution:

1. `templates.<name>:` explicit per-template path in config.
2. `<paths.templates>/<name>.md` file presence in the templates directory.
3. Plugin default at `<plugin_root>/templates/<name>.md`.

**Decision (resolved)**: the migration must handle both tier 1
(`templates.<name>:` paths pointing at arbitrary non-`paths.templates`
locations) and tier 2 (`<paths.templates>/*.md` file presence) overrides for
the three affected templates (`plan`, `codebase-research`, `rca`). This means
the migration cannot simply iterate the `<paths.templates>/` directory; for
each affected template name it must:

1. Probe the config for `templates.<name>:` (via `config-read-value.sh templates.<name>`).
2. If set and the file exists, rewrite at that path.
3. Otherwise probe `<paths.templates>/<name>.md`; if it exists, rewrite there.
4. Otherwise skip (the user is using the plugin default, which the producer
   side of this same release already covers).

Note that `templates.rca` is **not** in `TEMPLATE_KEYS` (`config-defaults.sh:66-73`),
but a userspace `.accelerator/config.md` may still define it as a free-form
key — the resolver supports arbitrary `templates.<name>` lookups via
`config-read-value.sh`, so this works without registering the key.

### Corpus statistics

- Plans (`meta/plans/*.md`): every plan file uses `work-item:` (line 5 typically)
  — 76 plan files at HEAD. **All must be rewritten by the migration.**
- Codebase research (`meta/research/codebase/*.md`): ~52 files using
  `researcher:` (line 3).
- Issue / RCA research (`meta/research/issues/*.md`): unknown count — verify
  during planning whether any active files contain `researcher:` (template
  emits it, so any RCA written from the template will).
- Work items, ADRs, reviews: do not use `work-item:` or `researcher:` (those use
  `work_item_id:` and `author:` respectively already).

### Frontmatter shape contract

ADR-0033 (`meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`,
accepted 2026-05-19) mandates:

- All identity values are **quoted YAML strings**: `work_item_id: "0042"`,
  not `work_item_id: 42`.
- `author` is the canonical name for the human authoring an artefact;
  `producer` is reserved separately for the skill identifier.

A naive `sed 's/^work-item:/work_item_id:/'` preserves the value bytes
verbatim and would leave any unquoted values unquoted after the rename. AC #6
("All `work_item_id:` values remain quoted YAML strings per the identity-value
shape contract") requires the new key to always carry a quoted value.

**Decision (resolved)**: the migration must defensively quote-normalise as
part of the rename, even though a ripgrep over `meta/plans/` at HEAD finds
zero unquoted `work-item:` values today. The corpus may legitimately contain
unquoted values in userspace repos, and template overrides written by hand
could also omit quoting. The migration should therefore handle three input
shapes in one pass:

- `work-item: "0042"` → `work_item_id: "0042"` (already quoted, identity)
- `work-item: 0042` → `work_item_id: "0042"` (numeric / unquoted scalar → quoted string)
- `work-item:` *empty value* → `work_item_id:` *empty value* (preserve empty as-is; no quoting added)

This is more than a pure key-prefix `sed` — it needs to inspect (and possibly
rewrite) the value portion. A small awk/sed pipeline anchored on the line
shape (`^work-item:[[:space:]]*` + capture group + optional comment) is the
natural fit; the dual-presence (partial-prior-run) guard from 0005 still
applies to the key portion. A new fixture scenario should cover the unquoted
input case explicitly. The `researcher:` → `author:` rename does **not**
require quote-normalisation: `author` is a free-text scalar, not an identity
value, and the AC does not impose a shape contract on it.

### Sibling pattern — 0063 (just landed)

- Work item: `meta/work/0063-rename-work-item-type-to-kind.md`
- Plan: `meta/plans/2026-05-20-0063-rename-work-item-type-to-kind.md`
- All phases marked complete in commits `64eca1bf` (frontmatter refinement),
  `9cea44c7` (phase completion), `e843e225` (eval/grader rename), `b9266ae7`
  (SKILL.md rename).
- Shipped migration 0005 (the precedent).
- 0064 sequences immediately after 0063 and picks migration 0006.

### Blocked successor — 0065 (drafted)

`meta/work/0065-update-artifact-templates-to-unified-schema.md` is blocked on
0064 and will rewrite every template under `templates/` to the full unified
schema (adding `schema_version`, provenance bundle, `last_updated`, etc.).
**0064 stays minimal**: only the two key renames, no extra fields. 0065 will
overwrite the same files with the broader schema later.

### Broader workstream — 0070 (drafted)

`meta/work/0070-ship-meta-corpus-unified-schema-migration.md` currently lists
the same `work-item:` → `work_item_id:` and `researcher:` → `author:` renames
in its Requirements. **0064 owns them now**; 0070's Requirements and AC will
need updating once 0064 lands (same pattern as 0063 forcing 0070 to drop the
`type:` → `kind:` step).

## Code References

### Producer surface
- `templates/plan.md:5` — `work-item: "{work-item reference, if any}"`
- `templates/codebase-research.md:3` — `researcher: [Git author]`
- `templates/rca.md:3` — `researcher: [Git author]`

### Visualiser server (the only behavioural read site)
- `skills/visualisation/visualise/server/src/frontmatter.rs:326` — `if let Some(v) = m.get("work-item")` (string-literal lookup)
- `skills/visualisation/visualise/server/src/frontmatter.rs:297-302` — doc comment
- `skills/visualisation/visualise/server/src/frontmatter.rs:441,455,523` — inline YAML test fixtures
- `skills/visualisation/visualise/server/src/indexer.rs:1025` — call site of `read_ref_keys` in `build_entry`
- `skills/visualisation/visualise/server/src/indexer.rs:2585,2641,2660,2687` — inline YAML test fixtures
- `skills/visualisation/visualise/server/src/patcher.rs:259,407` — pass-through test fixtures
- `skills/visualisation/visualise/server/src/api/related.rs:78` — doc comment only

### Visualiser frontend (no string-literal frontmatter-key reads)
- `skills/visualisation/visualise/frontend/src/api/types.ts:71,75` — `workItemId`, `workItemRefs` (camelCase mirrors of Rust struct fields)
- `skills/visualisation/visualise/frontend/src/api/work-item.ts:4-39` — filename-derived ID; reads `frontmatter['status']` only
- `skills/visualisation/visualise/frontend/src/routes/kanban/WorkItemCard.tsx:26-27` — reads `relPath` and `frontmatter['kind']`

### Server on-disk test fixtures
- `skills/visualisation/visualise/server/tests/fixtures/meta/work/0001-first-work-item.md:4` (and 0002, 0003, 0005, 0006)

### Migrate skill — 0005 precedent
- `skills/config/migrate/migrations/0005-rename-work-item-type-to-kind.sh:1-75`
- `skills/config/migrate/scripts/run-migrations.sh:87-93,165-167,177-208`
- `skills/config/migrate/scripts/test-migrate.sh:1255-1454`
- `skills/config/migrate/scripts/test-fixtures/0005/` (10 scenarios)
- `skills/config/migrate/scripts/test-fixtures/0005/paths-override/.accelerator/config.md` — `paths.work` override fixture (model for corpus path resolution, not template content rewrite)

### Migrate skill — template-rename precedent (file path only, not content)
- `skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh:419-430` — `_rename_user_template_file_if_present`

### Config-resolver
- `scripts/config-read-path.sh:1-76` — bare-key CLI; supports all `paths.*` keys
- `scripts/config-defaults.sh:26-73` — `PATH_KEYS` / `PATH_DEFAULTS` / `TEMPLATE_KEYS`
- `scripts/config-read-template.sh:1-60` — three-tier template resolution
- `skills/config/configure/SKILL.md:381-427` (paths table), `685-731` (template overrides), `429-430` (no YAML comments)

### Shared helpers
- `scripts/atomic-common.sh:16-32` — `atomic_write`
- `scripts/log-common.sh:16-18` — `log_warn`, `log_die`

### Spurious matches to ignore (NOT rename targets)
- `scripts/test-config.sh:5133` — `templates: work-item: custom/work-item.md` is a config-schema YAML key under `templates:`, not the plan frontmatter key.
- `scripts/test-config.sh:968`, `scripts/config-read-agents.sh:41`, `scripts/config-dump.sh:141,151`, `skills/config/configure/SKILL.md:149` — all `web-search-researcher:` (agent name), not the YAML field.
- `skills/work/create-work-item/SKILL.md:461` — slash-command name in prose.
- `skills/config/migrate/scripts/test-migrate.sh:1230` — filename `researchers.md`.
- `skills/work/create-work-item/evals/evals.json:98` — agent-name mention.
- `frontend/src/routes/library/template-tier.ts:37` — template-stem map entry.
- All `'work-items'` (plural) hits — `DocTypeKey` literal, unrelated.

### Corpus (in-scope for migration, out of scope for the AC grep)
- `meta/plans/*.md` — 76 files with `work-item:` (rewrite target)
- `meta/research/codebase/*.md` — ~52 files with `researcher:` (rewrite target)
- `meta/research/issues/*.md` — RCA / issue research (rewrite target; count to verify)

## Architecture Insights

1. **Template-first emission, near-zero inline emission in SKILL.md**: the
   accelerator's producing skills uniformly delegate to template files via
   `config-read-template.sh`. This makes the rename surface much smaller than
   the story implies — no inline frontmatter emitters in `skills/planning/` or
   `skills/research/` need rewriting.

2. **String-literal key reads are rare and centralised**: in the Rust
   visualiser, frontmatter YAML keys are looked up by string literal only in
   `frontmatter.rs` (read_ref_keys + a few other field-specific helpers).
   Downstream code operates on Rust struct fields whose names are
   identifier-bound and decoupled from YAML key names. This isolation means a
   frontmatter rename is a one-line behavioural change plus test-fixture
   updates.

3. **Wire contract decouples from YAML keys**: the frontend's `workItemId` and
   `workItemRefs` are camelCase mirrors of Rust struct fields, not direct
   echoes of frontmatter keys. The TS API contract therefore does **not** need
   to change unless the Rust struct fields are themselves renamed — which is
   *not* required by the rename (the existing identifier `work_item_refs`
   semantically still describes "refs to work items" regardless of YAML key
   spelling).

4. **Migration scripts use line-anchored `sed` over key prefix**: this preserves
   YAML value byte-for-byte (including quoting, lists, comments after the
   value, whitespace) and makes idempotence trivial — a second run finds no
   `^old-key:` line and is a no-op. **This breaks if a value spans multiple
   lines** (block scalars, multiline lists) — verify no plans use multiline
   `work-item:` values during planning.

5. **`paths.*` resolution is uniform across migrations**: every migration that
   walks a corpus uses `bash "$PLUGIN_ROOT/scripts/config-read-path.sh" <bare-key>`.
   This is the entire userspace-override story for migration path resolution.

6. **Template-content rewriting is a new pattern**: no existing migration
   rewrites content inside `.accelerator/templates/*.md`. 0064 introduces this
   pattern. The closest precedent is 0004's template file rename (`mv` only).
   The planning session should agree on the new shape — including how to
   handle "tier 1" template overrides (explicit `templates.<name>:` paths
   pointing outside `paths.templates`).

## Historical Context

### Parent epic and base schema decision
- `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md` —
  the parent epic. Explicitly enumerates the `work-item:` ↔ `work_item_id:`
  and `researcher:` ↔ `author:` conflicts as renames; identity values must be
  quoted YAML strings.
- `meta/work/0060-adr-unified-base-frontmatter-schema.md` — the ADR-creation
  task; status **done**.
- `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md` — accepted
  2026-05-19. Canonicalises `author` and reserves `producer` for skill
  identifier. **Latent tension**: ADR-0033 introduces a unified `id` field for
  artefact identity, but the per-artefact-extras still names cross-ref keys
  like `work_item_id`. The two are different concepts (own-identity vs typed
  reference); 0064 canonicalises only the cross-ref key (`work_item_id` on
  plans pointing at work items), not the work item's own ID, which is already
  named correctly.
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md` — accepted 2026-05-20.
  Reference shape contract `"doc-type:id"` quoted strings; relevant only if
  the planning session decides to also reshape plan cross-refs (out of scope
  for 0064).

### Sibling — just landed
- `meta/work/0063-rename-work-item-type-to-kind.md` and its plan at
  `meta/plans/2026-05-20-0063-rename-work-item-type-to-kind.md`. All phases
  complete; migration 0005 shipped. **Direct template for 0064's planning,
  TDD ordering, and eval-baseline approach.**

### Blocked successor
- `meta/work/0065-update-artifact-templates-to-unified-schema.md` — blocked on
  0064. Will rewrite every template to the full unified schema later. 0064
  must leave templates in a coherent intermediate state (the two renames, no
  other shape changes).

### Broader workstream
- `meta/work/0070-ship-meta-corpus-unified-schema-migration.md` — currently
  duplicates the two renames in its Requirements. Will need updating after
  0064 lands.

### Migration framework
- `meta/decisions/ADR-0023-meta-directory-migration-framework.md` — governs
  all migration scripts (driver-owned dirty-tree guard, state-file append,
  preview banner). 0006 must conform.

## Related Research

No prior research document exists for 0064. Most relevant adjacent research:

- `meta/research/codebase/2026-05-03-update-visualiser-for-work-item-terminology.md`
  — earlier work on `ticket:` → `work-item:` in the visualiser. Useful for
  understanding the Rust read paths.
- `meta/research/codebase/2026-04-26-remaining-ticket-references-post-migration.md`
  — methodology for inventorying post-rename leftover references; the AC grep
  pattern in 0064 follows the same shape.

## Resolved Decisions

The original open questions have been answered by the user; resolutions are
inlined above (in "Producer surface", "0005's shape", "Frontmatter shape
contract", and "Userspace template override mechanism") and summarised here:

1. **Body labels for `researcher:` → `author:` rename**: **In scope.** Both
   `templates/codebase-research.md:17` and `templates/rca.md:17` emit
   `**Researcher**:` body labels, and these propagate into the corpus. The
   migration twin-passes the body label exactly like 0005's `**Type**:` →
   `**Kind**:`. (The placeholder text `[Researcher name]` in `last_updated_by`
   values at line 11 of each template is also touched as part of the template
   edits, though it is a placeholder example and does not require corpus
   rewriting.) `templates/plan.md` has no `**Work-Item**:` body label, so the
   `work-item:` → `work_item_id:` rename is frontmatter-only.

2. **Quote-normalise `work_item_id` values defensively**: **Yes.** Ripgrep
   shows zero unquoted `work-item:` values in `meta/plans/` at HEAD, but the
   migration must handle unquoted values gracefully and emit quoted strings
   regardless of input shape. See "Frontmatter shape contract" above for the
   three input cases and the implication that the rewrite is not a pure
   key-prefix `sed`. A new fixture scenario covers the unquoted input case.

3. **Multiline values**: not present at HEAD; the awk/sed approach is line-
   anchored and would not corrupt a multiline value (it touches only the key
   on the anchored line). No special handling needed.

4. **Tier-1 template overrides (`templates.<name>:`)**: **In scope.** Migration
   probes the config for each affected template name and rewrites at the
   override path if set; otherwise falls back to `<paths.templates>/<name>.md`.
   See "Userspace template override mechanism" above.

5. **`templates.rca` key absence**: handled by the same resolution logic in
   resolution #4 — the migration looks up `templates.rca` via the value
   resolver (which supports arbitrary keys), not the registered `TEMPLATE_KEYS`
   array.

6. **Rust struct field `IndexEntry.work_item_id` rename**: **Not in scope.**
   The struct field is filename-derived (a different concept from the
   frontmatter cross-ref key being renamed) and decoupled from the YAML key
   name via the existing `m.get("work-item")` indirection in `frontmatter.rs`.
   The only behavioural change in Rust source is updating the string literal
   at `frontmatter.rs:326` from `"work-item"` to `"work_item_id"`, plus the
   inline YAML test fixtures. Identifier names (`work_item_refs`,
   `work_item_refs_by_target`, etc.) stay as they are.

7. **Visualiser bundle staleness**: flagged for the planning session; the
   plan's manual-verification phase should include a bundle rebuild and
   browser cache clear before running the visualiser smoke test from AC #7.

8. **AC grep root coverage**: the AC's grep roots are correct (redundant
   listing of visualiser paths after `skills/` is harmless; the exclusions of
   `meta/` and `skills/config/migrate/migrations/` are correct).

## References

- Source work item: `meta/work/0064-canonicalise-work-item-id-and-author-fields.md`
- Parent epic: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Base schema ADR: `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`
- Sibling rename: `meta/work/0063-rename-work-item-type-to-kind.md` (plan: `meta/plans/2026-05-20-0063-rename-work-item-type-to-kind.md`)
- Blocked successor: `meta/work/0065-update-artifact-templates-to-unified-schema.md`
- Broader workstream: `meta/work/0070-ship-meta-corpus-unified-schema-migration.md`
- Migration framework: `meta/decisions/ADR-0023-meta-directory-migration-framework.md`
- Migration precedent: `skills/config/migrate/migrations/0005-rename-work-item-type-to-kind.sh`
- Template-rename precedent: `skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh:419-430`
