---
date: "2026-05-21T20:30:00+01:00"
type: plan
skill: create-plan
work-item: "meta/work/0064-canonicalise-work-item-id-and-author-fields.md"
status: accepted
---

# Canonicalise `work_item_id` and `author` Frontmatter Field Names — Implementation Plan

## Overview

Rename two frontmatter field names so the canonical spelling decided
in ADR-0033 is used uniformly across the plugin and the meta/ corpus:

1. Plan frontmatter `work-item:` → `work_item_id:` (cross-ref key on
   plans pointing at a work item).
2. Codebase-research and RCA frontmatter `researcher:` → `author:`
   (artifact authorship). The companion body label
   `**Researcher**:` → `**Author**:` rewrites alongside.

Ship a new migration (0006) that rewrites the plan, codebase-research,
and issue/RCA research corpora — honouring `paths.*` overrides — and
also rewrites userspace template overrides (tier 1
`templates.<name>:` paths and tier 2 `<paths.templates>/<name>.md`
files). Apply the migration to this repo's corpus inside the same
delivery, update the producer templates and the single visualiser
read site, and update the downstream story 0070 so it stops
duplicating these renames. Bundle the lot into a single releasable
PR so the repo and any userspace consumer stay coherent across the
version bump.

The rename unblocks story 0065 (full template-wide unified-schema
adoption) and shrinks the surface 0070 has to cover.

## Current State Analysis

- **Producers — templates.** The two renames have a small,
  template-bound producer surface (research §"Producer surface"):
  - `templates/plan.md:5` — `work-item: "{work-item reference, if any}"`.
  - `templates/codebase-research.md:3,11,17` — `researcher: [Git author]`
    frontmatter; `last_updated_by: [Researcher name]` placeholder;
    `**Researcher**: [Researcher name from thoughts status]` body label.
  - `templates/rca.md:3,11,17` — same shape as codebase-research.
  - Producer SKILL.md files
    (`skills/planning/create-plan/SKILL.md`,
    `skills/research/research-codebase/SKILL.md`,
    `skills/research/research-issue/SKILL.md`) do **not** emit either
    field name inline; they render via `config-read-template.sh`.
- **Consumers — visualiser server.** Only **one** behavioural read of
  the old key exists anywhere in the codebase:
  `skills/visualisation/visualise/server/src/frontmatter.rs:326`
  (string-literal HashMap lookup `m.get("work-item")`). Doc comments at
  `frontmatter.rs:297-302,325` and `api/related.rs:78` reference the
  old key in prose only. The Rust identifier `work_item_refs` and all
  reverse-index plumbing in `indexer.rs` operate on the
  `Vec<String>` populated by that one read site and stay as they are.
- **Consumers — visualiser frontend.** **No** string-literal reads of
  `"work-item"` or `"researcher"` anywhere under
  `skills/visualisation/visualise/frontend/src/`. The TS wire types
  (`api/types.ts:71,75` — `workItemId`, `workItemRefs`) are camelCase
  mirrors of Rust struct fields, decoupled from YAML key spelling.
  The story over-scopes the frontend; no frontend code change is
  required.
- **Consumers — `researcher:`.** Zero references in `skills/`,
  `scripts/`, or anywhere outside templates and the `meta/` corpus.
  The migration must still rewrite the corpus, but no code reads this
  field today.
- **Visualiser on-disk test fixtures** at
  `skills/visualisation/visualise/server/tests/fixtures/meta/work/0001-…`
  through `0006-…` contain literal `work-item:` lines (5 files), as
  do inline YAML fixtures inside `frontmatter.rs:441,455,523`,
  `indexer.rs:2585,2641,2660,2687`, and pass-through tests in
  `patcher.rs:259,407`. These must be rewritten alongside the
  production read-site change to keep tests realistic and to satisfy
  the AC `rg` over `skills/`.
- **Migration framework** (ADR-0023) and **0005 precedent**
  (`skills/config/migrate/migrations/0005-rename-work-item-type-to-kind.sh`):
  outer `grep -q` guard, dual-presence sub-check for partial-prior-run
  idempotence, `atomic_write` per file, `config-read-path.sh` for
  corpus discovery, `log_warn` for divergences. 0064 mirrors this
  shape but walks three corpora and adds an unprecedented
  template-content rewrite pass.
- **`paths.*` resolution.** Configured paths for the three corpora
  (research `config-defaults.sh:26-64`):
  - `paths.plans` (default `meta/plans`).
  - `paths.research_codebase` (default `meta/research/codebase`).
  - `paths.research_issues` (default `meta/research/issues`) — **RCA
    artefacts share this path with issue research**; there is no
    separate `paths.rca`.
  - `paths.templates` (default `.accelerator/templates`).
- **Template override tiers** (`config-read-template.sh:11-15`): tier
  1 `templates.<name>:` config key pointing at an arbitrary path; tier
  2 `<paths.templates>/<name>.md` file presence. Both must be handled
  by the migration's template-rewrite pass. `templates.rca` is not in
  `TEMPLATE_KEYS` but may still be defined as a free-form config key
  (resolved via `config-read-value.sh`).
- **Corpus statistics at HEAD.**
  - `meta/plans/`: 86 files carrying `work-item:` (all quoted strings).
  - `meta/research/codebase/`: 52 files carrying `researcher:`.
  - `meta/research/issues/`: directory exists with only `.gitkeep`;
    zero RCAs to migrate today, but the migration must still resolve
    and skip it cleanly.
- **Frontmatter shape contract** (ADR-0033): `work_item_id:` values
  are always quoted YAML strings. A naive `sed 's/^work-item:/work_item_id:/'`
  preserves value bytes verbatim — fine for the (current) 100%-quoted
  corpus at HEAD but unsafe for userspace repos that may contain
  unquoted numeric values. The migration must defensively
  quote-normalise during the rename. `author:` is free-form text; no
  shape contract applies.
- **Sibling — 0063 just landed.** Plan
  `meta/plans/2026-05-20-0063-rename-work-item-type-to-kind.md`
  shipped migration 0005 plus this TDD-per-phase ordering. 0064
  follows the same pattern and picks the next migration number 0006.

## Desired End State

After this plan is complete:

1. `skills/config/migrate/migrations/0006-canonicalise-work-item-id-and-author.sh`
   exists, is idempotent, honours `paths.plans`,
   `paths.research_codebase`, `paths.research_issues`, and
   `paths.templates` overrides, and handles both template-override
   tiers (`templates.<name>:` and `<paths.templates>/<name>.md`).
2. Migration 0006 has been applied to this repo. All 86 plan files
   carry `^work_item_id:` (quoted string values); all 52
   codebase-research files carry `^author:` and `^\*\*Author\*\*:`
   (where a body label was present). No file carries `^work-item:`,
   `^researcher:`, or `^\*\*Researcher\*\*:`.
3. The three templates (`templates/plan.md`,
   `templates/codebase-research.md`, `templates/rca.md`) emit the
   canonical names.
4. Visualiser server reads the new key. `frontmatter.rs:326` calls
   `m.get("work_item_id")`. Inline and on-disk YAML test fixtures
   under `server/src/` and `server/tests/fixtures/` use
   `work_item_id:`. `cargo test` exits 0.
5. `meta/work/0070-ship-meta-corpus-unified-schema-migration.md`
   stops listing these two renames in Requirements/AC (0064 owns them
   now; 0070 must not duplicate).
6. The two AC verification greps (story 0064 AC #1 and AC #2) return
   zero hits modulo the documented exclusions, and the upgrade-path
   fixture (AC #7) executes cleanly end-to-end.

### Key Discoveries

- Outer `grep -q '^old_key:'` guard + dual-presence sub-check is the
  established migration idempotence pattern
  (`0005-rename-work-item-type-to-kind.sh:42-54`).
- The migration framework exports `PROJECT_ROOT`, `CLAUDE_PLUGIN_ROOT`,
  `ACCELERATOR_MIGRATION_MODE=1` to each migration. Migrations MUST
  NOT touch `.accelerator/state/migrations-applied` — driver appends
  on success (`run-migrations.sh:204-205`).
- `bash "$PLUGIN_ROOT/scripts/config-read-path.sh" <bare-key>` honours
  `paths.*` overrides; returns a path relative to project root and
  empty string if the key is unset and has no default.
- `bash "$PLUGIN_ROOT/scripts/config-read-value.sh templates.<name>`
  resolves tier-1 template overrides; supports arbitrary
  `templates.<name>` keys even if not in `TEMPLATE_KEYS`.
- `atomic_write` from `scripts/atomic-common.sh:16-32` is the
  preferred rewrite primitive.
- Body-label rewrite for `**Researcher**:` mirrors the
  `**Type**:`/`**Kind**:` rewrite pass in 0005:54-69. The plan
  frontmatter has no `**Work-Item**:` body label, so the
  `work-item:` → `work_item_id:` rename is frontmatter-only.
- Quote-normalisation is **not** pure `sed`. An awk pipeline
  anchored on `^work-item:[[:space:]]*` is the natural fit and
  handles three input shapes in one pass (quoted, unquoted scalar,
  empty value).
- **Template-content rewriting is a new migration pattern**. Migration
  0004 has a precedent for renaming template files (`mv`), but no
  existing migration has rewritten content inside
  `.accelerator/templates/*.md`. 0006 introduces this pattern.

## What We're NOT Doing

- **Not** touching the visualiser frontend
  (`skills/visualisation/visualise/frontend/`). No string-literal
  reads of the old keys exist there. The Rust struct field
  `IndexEntry.work_item_id` (filename-derived identity, distinct
  concept) and its TS mirror `workItemId` stay as they are.
- **Not** renaming any Rust identifier
  (`work_item_refs`, `work_item_refs_by_target`, etc.). The single
  behavioural change is the string-literal HashMap lookup at
  `frontmatter.rs:326`.
- **Not** removing the legacy `ticket:` fallback. The plugin still
  supports the ticket → work-item migration path for older user repos
  (migration 0001).
- **Not** changing any field other than the two named in the story.
  Story 0065 will do the broader unified-schema template rewrite next;
  0064 leaves templates in a coherent intermediate state.
- **Not** rewriting unrelated `work-item-…` strings in the codebase
  (slash-command names, agent-name `web-search-researcher`,
  TanStack/HTML `type` attributes, doc-type-key string `'work-items'`)
  — research §"Spurious matches to ignore" enumerates the carve-outs.
- **Not** updating `meta/` historical artefacts that aren't part of
  the three migration target corpora (e.g. older reviews under
  `meta/reviews/`, decisions under `meta/decisions/`). The AC grep
  roots exclude `meta/` for this reason. Phase 5 does sweep
  `meta/work/` and the two ADRs that reference `work-item:` in
  present-tense passages (ADR-0025, ADR-0034) — those are
  documentation that should reflect the post-migration canonical
  schema.
- **Not** updating placeholder copy that aligns to the renamed
  concept but isn't itself a rename target. Specifically: the
  `last_updated_by: [Researcher name]` placeholder hint in
  `templates/codebase-research.md:11` and `templates/rca.md:11`
  is **not** touched in this story. That copy belongs to the
  broader unified-schema template work in story 0065, which has the
  freedom to reconcile placeholder conventions across all templates
  (including `templates/design-inventory.md` which uses a different
  `"{author name}"` shape).

## Implementation Approach

Strict sequencing inside a single atomic delivery (story 0064 AC
bundle):

1. Author migration 0006 first (TDD) so the dogfood corpus can be
   migrated before any producer reads the new key from a file still
   on disk under the old key.
2. Apply the migration to this repo, so `meta/plans/`,
   `meta/research/codebase/`, and (no-op) `meta/research/issues/`
   carry the canonical names.
3. Land producer template updates and visualiser-server consumer
   updates in the same commit/PR as the migration so no consumer
   reads a legacy key from a producer that has already moved.
4. Update the downstream story 0070 to remove the now-redundant
   rename steps from its Requirements/AC.
5. Run the full verification battery (AC greps, eval/test suites,
   upgrade-path fixture run, visualiser manual smoke).

TDD applies to every phase with executable tests:

- **Phase 1** (migration 0006): write
  `test-migrate.sh` cases + test fixtures **first**; see them fail;
  then author the migration script to satisfy them.
- **Phase 4** (visualiser server): update Rust inline YAML fixtures
  to the new key **first**; see `cargo test` fail; then update the
  string literal at `frontmatter.rs:326` and update on-disk
  fixtures; re-run to green.

For phases that rewrite content without executable tests (templates —
Phase 3; downstream-story housekeeping — Phase 5), the story's
acceptance-criteria greps serve as the oracle. Run them before
editing to confirm they hit, perform the rewrites, re-run to confirm
zero hits.

**Phase independence after Phase 2**: Phases 3, 4, and 5 are
independent and may be performed in any order (or in parallel) once
the migration has been applied. Phase 6 depends on all prior phases.

---

## Phase 1: Migration 0006 — author with TDD

### Overview

Write test cases against the migration framework's harness first,
then author migration 0006 to satisfy them. The test cases exercise
every branch the migration must handle: clean rename on each of the
three corpora; quote-normalisation of unquoted plan values; body-label
rewrite alongside the research/RCA frontmatter rename; idempotence
(re-run no-op); partial prior run (both old and new keys present);
`paths.*` overrides; missing/empty corpus directories; userspace
template overrides via both tier-1 and tier-2 mechanisms.

### Changes Required

#### 1. Test fixtures

**Directory**: `skills/config/migrate/scripts/test-fixtures/0006/`

Create the following fixture scenarios, each a self-contained
pseudo-repo with a `.accelerator/config.md` where overrides are
needed and `.md` files placed under the appropriate corpus
directories:

- `default-layout/` — repo carrying:
  - `meta/plans/0001-foo.md` with `work-item: "0042"` and quoted-value
    cross-ref.
  - `meta/research/codebase/2026-01-01-foo.md` with `researcher: Toby Clemson`
    frontmatter and `**Researcher**: Toby Clemson` body label.
  - `meta/research/issues/2026-01-02-bar.md` (RCA shape) with both
    `researcher:` and `**Researcher**:`.

- `unquoted-work-item/` — plan with `work-item: 0042` (no quotes).
  Asserts post-migration shape is `work_item_id: "0042"` (quote-
  normalised). Idempotence: re-run leaves shape as
  `work_item_id: "0042"`.

- `single-quoted-work-item/` — plan with `work-item: '0042'`
  (YAML single-quoted scalar). Asserts post-migration shape is
  `work_item_id: "0042"` (re-wrapped as double-quoted; single quotes
  stripped).

- `no-whitespace-work-item/` — plan with `work-item:0042`
  (no whitespace between colon and value). Asserts post-migration
  shape is `work_item_id: "0042"`. Locks in the awk regex's
  no-whitespace handling.

- `inline-comment-work-item/` — plan with `work-item: 0042 # see TRELLO-91`
  (inline YAML comment on unquoted scalar). Asserts the migration
  REFUSES the rewrite: the original line is preserved, a sentinel
  `# 0006-WARN: refused (inline comment on work-item)` is emitted
  immediately below, and stderr contains a `log_warn` line naming
  the file. Plan idempotence: re-run finds the legacy `work-item:`
  still present + the sentinel and refuses again (no further
  divergence).

- `embedded-quote-work-item/` — plan with `work-item: foo"bar`
  (unquoted with embedded double quote). Asserts REFUSED with
  sentinel and `log_warn` (same shape as inline-comment).

- `trailing-whitespace-work-item/` — plan with `work-item: "0042"   `
  (quoted value followed by trailing spaces). Asserts post-migration
  shape is `work_item_id: "0042"` (trailing whitespace stripped)
  and that the divergence check (when paired with `work_item_id: "0042"`
  with no trailing whitespace) does NOT emit a spurious warning.

- `empty-work-item-value/` — plan with `work-item:` (empty value, no
  whitespace after colon). Asserts post-migration shape is
  `work_item_id:` (empty preserved verbatim; no spurious empty
  quotes added).

- `empty-work-item-value-trailing-ws/` — plan with `work-item:   `
  (empty value with trailing whitespace). Asserts post-migration
  shape is `work_item_id:` (empty preserved, trailing whitespace
  not propagated).

- `mixed-plan-shapes/` — directory with three plans of mixed shape
  (one quoted, one unquoted, one empty-value). Asserts the AC #6
  invariant at the directory level: `rg -c '^work_item_id: [^"]'`
  returns 0 across the directory (the empty-value line, which lacks
  a trailing quote character, is matched by the `^work_item_id: $`
  pattern not the `^work_item_id: [^"]` pattern, so it does not
  count as a violation).

- `partial-prior-run-plan/` — plan carrying both `work-item: "0042"`
  and `work_item_id: "0042"` (matching values). Asserts stale
  `work-item:` is dropped; `work_item_id:` remains; no stderr
  divergence line.

- `partial-prior-run-plan-unquoted/` — plan carrying both
  `work-item: "0042"` and `work_item_id: 0042` (matching but the
  surviving `work_item_id:` is unquoted). Asserts stale `work-item:`
  is dropped AND the surviving `work_item_id:` is run through the
  awk normaliser so the post-migration line is `work_item_id: "0042"`.
  Locks in that the dual-presence path satisfies AC #6.

- `partial-prior-run-plan-divergent/` — plan carrying `work-item: "0042"`
  and `work_item_id: "0099"` (divergent values). Policy: `work_item_id:`
  wins silently in the file; `work-item:` is dropped; stderr emits one
  `log_warn` line naming the file and both values. Asserts
  `assert_contains "$stderr" 'divergent work-item/work_item_id'`.

- `partial-prior-run-research/` — codebase-research file carrying
  both `researcher: A` and `author: A` (matching). Stale `researcher:`
  dropped; no warning.

- `partial-prior-run-research-divergent/` — codebase-research file
  carrying `researcher: A` and `author: B` (divergent). `author:` wins;
  stderr `log_warn`.

- `partial-prior-run-body-label/` — research file carrying both
  `**Researcher**: Toby` and `**Author**: Toby` body labels (matching).
  Stale `**Researcher**:` dropped.

- `partial-prior-run-body-label-divergent/` — research file with
  divergent body labels. `**Author**:` wins; stderr warns.

- `body-label-multiple/` — research file carrying TWO `**Researcher**:`
  lines: one in the canonical header position (between closing
  frontmatter `---` and first `## Summary` heading) AND one inside a
  fenced code block in the body prose (post-first-H2). Asserts that
  only the FIRST (header) occurrence is rewritten to `**Author**:`,
  and the body-prose occurrence is left UNCHANGED (because the
  body-label awk pass is anchored to the pre-first-H2 region).

- `body-label-anchored-no-h2/` — research file with `**Researcher**:`
  in the canonical position AND no `## ` heading anywhere in the
  file. Asserts the rewrite happens (the anchor only suppresses
  rewrites that appear AFTER a `## ` heading; a file without any
  heading still gets the canonical-position rewrite).

- `body-label-quoted-prose-pre-h2/` — research file with
  `**Researcher**:` in the canonical position AND a second
  `**Researcher**:` line in legitimate prose (e.g., a quoted
  example inside a blockquote) that appears BEFORE the first
  `## ` heading. Asserts that BOTH lines are rewritten to
  `**Author**:` — locking in the documented trade-off that the
  pre-H2 anchor is intentionally permissive (matching the
  producer template's emission region) and pre-H2 prose
  occurrences are within the migration's accepted blast radius.
  Cross-referenced from Migration Notes point 4 so users with
  unusual header structures know to spot-check.

- `frontmatter-missing-fence/` — file with `work-item: "0001"` in
  what looks like frontmatter content but with NO leading `---`
  fence. Asserts (1) the file is NOT rewritten (awk's
  `seen_frontmatter_open` never flips, so no rename rule fires),
  (2) stderr contains `0006-MALFORMED: <file> — legacy key seen
  but no frontmatter fence (---) detected`, (3) the rewrite
  counter does NOT increment (the upstream `grep -qE` short-
  circuit and the `cmp -s` byte-equality check together prevent
  a touched-but-unchanged false positive).

- `paths-override-plans/` — `.accelerator/config.md` sets
  `paths.plans: docs/plans`; plan lives at `docs/plans/0001-foo.md`.
  Asserts override is honoured and `meta/plans/` is not created.

- `paths-override-research-codebase/` — `paths.research_codebase: docs/research`
  override; research file under `docs/research/`.

- `paths-override-research-issues/` — `paths.research_issues: docs/rca`
  override; RCA file under `docs/rca/`.

- `paths-missing-plans/` — `paths.plans: docs/typo-plans` pointing at
  a non-existent dir; asserts stderr warning and exit 0; migration
  still recorded as applied.

- `empty-research-issues/` — default layout with
  `meta/research/issues/` containing only `.gitkeep` (mirrors the
  current state of this repo). Asserts migration exits 0, no files
  created or modified, stdout reports `0` rewrites under that
  corpus.

- `template-override-tier2-plan/` — userspace tier-2 override at
  `.accelerator/templates/plan.md` with `work-item:` frontmatter.
  Asserts the file is rewritten in place to `work_item_id:` while the
  rest of the file content (heading, prose, comments) is byte-for-byte
  unchanged.

- `template-override-tier2-research/` — userspace tier-2 override at
  `.accelerator/templates/codebase-research.md` with `researcher:`
  frontmatter and `**Researcher**:` body label. Asserts both are
  rewritten.

- `template-override-tier2-rca/` — userspace tier-2 override at
  `.accelerator/templates/rca.md`. Asserts both frontmatter and body
  label rewritten. Note: `templates.rca` is not in `TEMPLATE_KEYS`
  but tier-2 resolution still works via `paths.templates` directory
  presence.

- `template-override-tier1/` — `.accelerator/config.md` sets
  `templates.plan: custom/templates/my-plan.md` pointing at a file
  outside `paths.templates`. Asserts the file is rewritten at the
  custom path.

- `template-override-both-tiers/` — `templates.plan: custom/templates/my-plan.md`
  AND `.accelerator/templates/plan.md` both present. Asserts only the
  tier-1 target is rewritten (the resolver returns tier-1 first); the
  tier-2 file is left untouched.

- `template-override-tier1-missing-file/` — `templates.plan: custom/templates/my-plan.md`
  configured but the file at the custom path is absent. AND
  `.accelerator/templates/plan.md` IS present. Asserts the migration
  emits a `log_warn` naming the missing tier-1 file and DOES NOT fall
  through to rewrite the tier-2 file (the user pinned a specific
  path; the migration honours their intent rather than masking the
  typo).

- `template-override-missing/` — `.accelerator/templates/` directory
  is absent entirely. Asserts migration exits 0 with no error and no
  warning (this is the default "user is using plugin defaults" state).

- `paths-alias-research/` — `.accelerator/config.md` sets both
  `paths.research_codebase: docs/research` AND
  `paths.research_issues: docs/research` (aliasing the same dir).
  A single research file sits there. Asserts the walker traverses
  the directory exactly once: `log_warn` emitted naming the aliased
  keys; stdout reports `rewrote 1 file(s)` under the first key
  (`research_codebase`) and the second key reports `skipping
  duplicate walk`. File content rewritten exactly once.

- `template-alias/` — `.accelerator/config.md` sets both
  `templates.plan` and `templates.codebase-research` to the same
  file path. Asserts the migration rewrites the file once,
  `log_warn`s the alias, and does NOT double-process.

- `idempotent/` — copy of `default-layout/` after running migration
  once; second run produces zero changes (`tree_hash` byte-identical).
  Implemented as a `setup_then_run_twice` helper that copies
  `default-layout/`, runs the migration, snapshots the tree, runs it
  again, and asserts the snapshot is unchanged. Run a THIRD time as
  well, with the same byte-identity assertion, so the awk's two
  emission shapes (empty-value preservation, quoted-value identity)
  are both pinned across multiple re-runs. This also covers the
  template-override pass: the second run must not abort under
  `set -e` when the templates no longer carry the legacy keys.

Where fixtures share content (e.g. canonical plan body, research
body), use shared helper fixtures or inline minimal content — the
goal is to keep each scenario small and one-concern.

#### 2. Test harness extensions

**File**: `skills/config/migrate/scripts/test-migrate.sh`
**Changes**: Append a new `=== 0006 ===` test block at the end. Add a
setup helper and an `ONLY_0006_DIR` migrations dir analogous to the
0005 block (`test-migrate.sh:1255-1454`).

```bash
ONLY_0006_DIR="$TMPDIR_BASE/only-0006-migrations"
mkdir -p "$ONLY_0006_DIR"
cp "$MIGRATIONS_DIR/0006-canonicalise-work-item-id-and-author.sh" \
  "$ONLY_0006_DIR/"

setup_0006_repo() {
  local scenario="$1"
  local repo_dir
  repo_dir=$(mktemp -d "$TMPDIR_BASE/repo-0006-XXXXXX")
  cp -R "$SCRIPT_DIR/test-fixtures/0006/$scenario/." "$repo_dir/"
  echo "$repo_dir"
}

# Mirrors the 0005 helper at test-migrate.sh:1276-1282 exactly:
# cd into the fixture repo so config_project_root resolves correctly,
# set CLAUDE_PLUGIN_ROOT, and use the canonical ACCELERATOR_MIGRATE_FORCE=1
# bypass (NOT a non-existent ACCELERATOR_MIGRATE_FORCE_NO_VCS).
run_0006_driver() {
  local repo="$1"; shift
  (
    cd "$repo" && \
    CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT" \
    ACCELERATOR_MIGRATIONS_DIR="$ONLY_0006_DIR" \
    ACCELERATOR_MIGRATE_FORCE=1 \
      bash "$DRIVER" "$@" 2>&1
  )
}
```

For each fixture scenario, add an assertion block that invokes
`run_0006_driver` and checks the post-migration shape. **Every
scenario MUST assert the per-corpus `rewrote N file(s) under <path>`
stdout line so the inverted-counter class of bug cannot regress**:

- `default-layout` — assert `^work_item_id: "0042"` in the plan,
  `^work-item:` absent; `^author: Toby Clemson` and
  `^\*\*Author\*\*: Toby Clemson` in research; `^researcher:` and
  `^\*\*Researcher\*\*:` absent in both research and RCA. Stdout
  contains `0006: rewrote 1 file(s) under meta/plans` AND
  `0006: rewrote 1 file(s) under meta/research/codebase` AND
  `0006: rewrote 1 file(s) under meta/research/issues`.

- `unquoted-work-item` — assert exact line `work_item_id: "0042"`.
  Stdout reports `rewrote 1 file(s) under meta/plans`.

- `single-quoted-work-item` — assert exact line `work_item_id: "0042"`
  (single quotes stripped; re-wrapped in double quotes).

- `no-whitespace-work-item` — assert exact line `work_item_id: "0042"`.

- `inline-comment-work-item` — assert the legacy `work-item:` line
  is preserved BYTE-IDENTICALLY (no sentinel is written into the
  file). Stderr contains `0006: 0006-REFUSE: <file> — refused
  work-item (unsafe value shape)`. Exit 0 (the refusal is a
  warning, not a fatal error). Idempotence: run the migration
  THREE times against this fixture and assert `tree_hash` byte-
  identity across all three runs (proves the no-sentinel-in-file
  posture holds — earlier `# 0006-WARN:` sentinel emissions would
  have stacked).

- `embedded-quote-work-item` — analogous: legacy line preserved
  byte-identically, stderr warns, exit 0, three-run idempotent.

- `partial-prior-run-plan-refused-shape/` — plan carrying
  `work-item: 0042 # note` (refused shape) AND
  `work_item_id: "0042"` (canonical). Asserts the migration leaves
  both lines unchanged (no dual-presence cleanup of the refused
  shape — the awk passes the legacy line through and the
  `^work_item_id:` rule normalises the canonical to its
  already-canonical shape). Stderr contains the refusal warning.
  Locks in the policy: refusal-plus-divergence does not silently
  drop the refused legacy line.

- `trailing-whitespace-work-item` — assert exact line
  `work_item_id: "0042"`. When paired with the partial-prior-run
  divergent fixture, assert stderr does NOT contain the
  `divergent work-item/work_item_id` warning (the trim makes the
  comparison whitespace-insensitive).

- `empty-work-item-value` — assert exact line `work_item_id:` (with
  no trailing whitespace, no quotes).

- `empty-work-item-value-trailing-ws` — assert exact line
  `work_item_id:` (trailing whitespace stripped).

- `mixed-plan-shapes` — assert `rg -c '^work_item_id: [^"]'` over
  the post-migration directory returns 0; `rewrote 3 file(s)` is
  reported.

- `partial-prior-run-plan` — frontmatter has exactly one
  `^work_item_id:` line; `^work-item:` absent; no stderr divergence
  line.

- `partial-prior-run-plan-unquoted` — assert exact line
  `work_item_id: "0042"` (the surviving line was unquoted on input;
  the divergent branch quote-normalises it). No stderr warning.

- `partial-prior-run-plan-divergent` — `^work_item_id: "0099"`
  remains; `^work-item:` absent; stderr contains
  `0006-DIVERGE: <file> — work-item="0042" vs work_item_id="0099"
  (kept work_item_id)`.

- `partial-prior-run-research*` — analogous shape assertions for
  the `researcher:` → `author:` rewrite.

- `partial-prior-run-body-label*` — analogous shape assertions for
  the body-label rewrite.

- `body-label-multiple` — assert the file contains EXACTLY one
  `^\*\*Author\*\*:` line (the canonical-position rewrite) AND
  EXACTLY one `^\*\*Researcher\*\*:` line (the prose occurrence
  post-first-H2 was NOT rewritten).

- `body-label-anchored-no-h2` — assert the rewrite happened
  (`^\*\*Author\*\*:` present; `^\*\*Researcher\*\*:` absent).

- `paths-override-*` — assert file at the override path is rewritten;
  default path is **not** created. Stdout reports
  `rewrote 1 file(s) under <override-path>`.

- `paths-missing-plans` — stderr contains
  `'plans directory does not exist'`; exit 0; no files modified.
  Stdout reports `rewrote 0 file(s)`.

- `paths-alias-research` — stdout reports `rewrote 1 file(s)` once
  (for `paths.research_codebase`) and `skipping duplicate walk`
  for `paths.research_issues`. File content rewritten exactly once
  (re-running yields no further change).

- `empty-research-issues` — exit 0; no changes; stdout reports
  `rewrote 0 file(s)` for that corpus.

- `template-override-tier1/2/both/missing` — assert template files
  are rewritten (or not) at the expected paths per the tier-resolution
  rules.

- `template-override-tier1-missing-file` — assert stderr contains
  `templates.plan points at missing file`; the tier-2 file is NOT
  rewritten (no fallthrough); exit 0.

- `template-alias` — file rewritten once; stderr warns about the
  alias; exit 0.

- `idempotent` — `tree_hash` of the repo subtree is byte-identical
  across THREE consecutive driver runs. Each run exits 0 (in
  particular: the second and third runs do not abort under `set -e`
  when the templates and corpora no longer carry the legacy keys).

#### 3. Migration 0006

**File**: `skills/config/migrate/migrations/0006-canonicalise-work-item-id-and-author.sh` (new)
**Changes**: Author the migration to satisfy the tests above.

Structural shape (mirrors 0005, multi-corpus aware). The script
adopts an **inline accumulator** pattern (per the 0005 precedent)
rather than overloading function return codes, so the per-corpus
rewrite counter is unambiguous and helper exits cannot abort the
migration under `set -e`:

```bash
#!/usr/bin/env bash
# DESCRIPTION: Canonicalise plan work-item -> work_item_id and research/RCA researcher -> author. Unconditional within frontmatter; body-label rewrite is anchored to the post-frontmatter / pre-first-H2 region.
set -euo pipefail

MIGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$MIGRATION_DIR/../../../.." && pwd)}"

source "$PLUGIN_ROOT/scripts/config-common.sh"
source "$PLUGIN_ROOT/scripts/atomic-common.sh"
source "$PLUGIN_ROOT/scripts/log-common.sh"

if [ -z "${PROJECT_ROOT:-}" ]; then
  PROJECT_ROOT="$(config_project_root)"
fi

# ---- Path safety ----------------------------------------------------

# Reject empty / traversal / absolute path shapes. Used by both the
# corpus walker (paths.*) and the template-override resolver (tier-1
# templates.<name>:).
assert_safe_relpath() {
  local rel="$1"; local label="$2"
  case "$rel" in
    ''|.|..|/|/*|*/..|../*|*/../*|*/./*)
      log_warn "0006: refusing dangerous $label value: $rel"
      return 1
      ;;
  esac
  # Defence in depth: resolve the path and assert it stays inside
  # PROJECT_ROOT. We use `cd && pwd -P` unconditionally (rather than
  # `realpath -m` which is GNU-coreutils-only and fails on macOS BSD
  # realpath) so the helper works portably across platforms.
  # Resolves the parent directory through symlinks then appends the
  # basename — handles missing leaves correctly while still catching
  # symlinked parents.
  local canonical=""
  local parent leaf abs_parent
  parent="$(dirname "$rel")"
  leaf="$(basename "$rel")"
  if abs_parent="$(cd "$PROJECT_ROOT" && CDPATH= cd -- "$parent" 2>/dev/null && pwd -P)"; then
    canonical="$abs_parent/$leaf"
  fi
  case "$canonical" in
    "$PROJECT_ROOT"|"$PROJECT_ROOT"/*) return 0 ;;
    *) log_warn "0006: $label resolves outside project root: $rel -> ${canonical:-<unresolvable>}"; return 1 ;;
  esac
}

# Resolve a paths.<key> bare value. Returns the relative path on
# stdout (exit 0) for safe values; returns nothing (exit 1) for
# unsafe / empty values. The walker `log_warn`s on rejection but
# continues with the remaining corpora — a single misconfigured key
# does NOT abort the migration (consistent with the missing-directory
# branch in walk_corpus).
resolve_corpus_path() {
  local key="$1"
  local rel
  rel="$(cd "$PROJECT_ROOT" \
    && bash "$PLUGIN_ROOT/scripts/config-read-path.sh" "$key" 2>/dev/null || true)"
  if [ -z "$rel" ]; then
    log_warn "0006: config-read-path.sh returned empty for '$key' — skipping corpus"
    return 1
  fi
  if ! assert_safe_relpath "$rel" "paths.$key"; then
    log_warn "0006: skipping unsafe paths.$key — other corpora will still migrate"
    return 1
  fi
  printf '%s\n' "$rel"
}

# Canonicalise a relative path for dedup-key use. Reduces `./foo`,
# `foo/`, and symlink-equivalent surface forms to the same key. Uses
# `cd && pwd -P` (portable across GNU and BSD systems including macOS)
# rather than `realpath -m` (GNU-coreutils-only). Falls back to the
# raw relative path if the directory doesn't exist (rare — missing
# corpus dirs are handled by walk_corpus, not by this helper).
canonicalise_rel() {
  local rel="$1"
  (cd "$PROJECT_ROOT" && CDPATH= cd -- "$rel" 2>/dev/null && pwd -P) \
    || printf '%s\n' "$PROJECT_ROOT/$rel"
}

# ---- Awk transforms (in-memory, single-write) -----------------------

# Single awk pass that performs all three rename axes in one pipeline,
# anchored to frontmatter where appropriate:
#
#   - `^work-item: ...` (in frontmatter) -> `^work_item_id: "..."`
#       (quote-normalised; see normalise_value)
#   - `^work_item_id: ...` (in frontmatter) -> `^work_item_id: "..."`
#       (idempotent value-shape normalisation; fixes partial-prior-
#        run survivors with unquoted values)
#   - `^researcher:` (in frontmatter) -> `^author:`
#   - `^\*\*Researcher\*\*:` (post-frontmatter, pre-first-`## `)
#       -> `^\*\*Author\*\*:`
#
# Dual-presence cleanup is performed INSIDE awk so it is gated by
# `in_frontmatter` (or by `!saw_first_h2` for the body label) and
# therefore cannot strip body-prose matches outside the canonical
# region.
#
# On refusable shapes (inline `#` comment in unquoted scalar; embedded
# `"` in unquoted scalar): the legacy line is PRESERVED as-is and a
# `log_warn` is emitted to stderr by the caller (no sentinel is
# written to the file, so idempotence holds across re-runs).
awk_transform() {
  # Args:
  #   $1 = file path (for stderr labelling)
  #   $2 = has_wi    (1 if legacy work-item: seen in frontmatter)
  #   $3 = has_id    (1 if canonical work_item_id: seen in frontmatter)
  #   $4 = has_r     (1 if legacy researcher: seen in frontmatter)
  #   $5 = has_a     (1 if canonical author: seen in frontmatter)
  #   $6 = has_rb    (1 if legacy **Researcher**: seen pre-first-H2)
  #   $7 = has_ab    (1 if canonical **Author**: seen pre-first-H2)
  #
  # Dual-presence flags come from the shell pre-scan in rewrite_file,
  # which extracts the frontmatter region (and pre-first-H2 region for
  # the body label) BEFORE grepping — so prose / code-block matches
  # outside the canonical region cannot trigger false dual-presence.
  #
  # The script communicates refusal / divergence / malformed events
  # to the caller via stderr lines tagged `0006-REFUSE:`,
  # `0006-DIVERGE:`, `0006-MALFORMED:` — the caller pipes these into
  # log_warn.
  awk \
    -v file="$1" \
    -v has_wi="${2:-0}" \
    -v has_id="${3:-0}" \
    -v has_r="${4:-0}" \
    -v has_a="${5:-0}" \
    -v has_rb="${6:-0}" \
    -v has_ab="${7:-0}" '
    function normalise_value(line,    inner) {
      # Returns the canonical value-portion (everything after `: `).
      # Identity for double-quoted; strip-and-rewrap for single-quoted;
      # wrap-in-double-quotes for unquoted; empty preserved as "".
      # Callers MUST strip trailing whitespace before calling, so the
      # outer-quote regex matches correctly.
      if (line ~ /^".*"$/) return line
      if (line ~ /^'\''.*'\''$/) {
        inner = substr(line, 2, length(line) - 2)
        gsub(/\\/, "\\\\", inner)
        gsub(/"/, "\\\"", inner)
        return "\"" inner "\""
      }
      return "\"" line "\""
    }
    function semantic_inner(line,    inner) {
      # Strip outer quotes (if any) to expose the inner content for
      # divergence comparison. `"0042"` and `0042` both yield `0042`,
      # so a quoted-vs-unquoted partial-prior-run survivor compares
      # equal semantically. Callers MUST strip trailing whitespace
      # before calling.
      if (line ~ /^".*"$/) return substr(line, 2, length(line) - 2)
      if (line ~ /^'\''.*'\''$/) return substr(line, 2, length(line) - 2)
      return line
    }
    function refuses(line) {
      # Refuses if line is NOT already double-or-single-quoted AND
      # contains an inline-comment `#` (anywhere — conservative; YAML
      # comment rules are nuanced) or an embedded `"`. Callers MUST
      # strip trailing whitespace before calling, so the outer-quote
      # regex matches correctly.
      if (line ~ /^".*"$/ || line ~ /^'\''.*'\''$/) return 0
      if (line ~ /#/) return 1
      if (line ~ /"/) return 1
      return 0
    }

    BEGIN {
      in_frontmatter = 0
      seen_frontmatter_open = 0
      saw_first_h2 = 0
      # Per-axis in-frontmatter tracking (first-seen flag + raw + inner
      # value for divergence comparison in END):
      saw_work_item = 0; saw_work_item_id = 0
      saw_researcher = 0; saw_author = 0
      saw_body_researcher = 0; saw_body_author = 0
      first_wi = ""; first_id = ""
      first_r = ""; first_a = ""
      first_rb = ""; first_ab = ""
      inner_wi = ""; inner_id = ""
      # Ungated `anywhere` flags (set by top-of-file rules regardless
      # of frontmatter state) — used by the END-block MALFORMED check
      # to detect legacy-key sightings in files lacking a `---` fence.
      saw_wi_anywhere = 0
      saw_r_anywhere = 0
      saw_rb_anywhere = 0
      # Body-label dual-presence: drop only the first `**Researcher**:`
      # occurrence to preserve any later prose-anchored occurrences
      # (though those are also outside the pre-first-H2 region and so
      # would not be rewritten anyway).
      dropped_first_rb = 0
    }

    # Ungated detection — sets the `*_anywhere` flags without
    # consuming the line. Order matters: this rule runs BEFORE the
    # in_frontmatter-gated rules, but neither rule body calls `next`
    # in a way that prevents the other from running, so awk evaluates
    # both rules per matching line. The gated rule's `next` ensures
    # the line is not also printed by the catch-all.
    /^work-item:/ { saw_wi_anywhere = 1 }
    /^researcher:/ { saw_r_anywhere = 1 }
    /^\*\*Researcher\*\*:/ { saw_rb_anywhere = 1 }

    # Frontmatter detection: the FIRST `---` line in the file opens
    # frontmatter (NR==1 is too strict — a leading BOM or blank line
    # would silently disable all rules). The next `---` closes it.
    !seen_frontmatter_open && /^---$/ {
      seen_frontmatter_open = 1
      in_frontmatter = 1
      print; next
    }
    in_frontmatter && /^---$/ {
      in_frontmatter = 0
      print; next
    }
    /^## / { saw_first_h2 = 1 }

    # ---- Plan frontmatter: work-item → work_item_id ----
    in_frontmatter && /^work-item:/ {
      line = $0
      sub(/^work-item:[ \t]*/, "", line)
      # Strip trailing whitespace BEFORE refuses() so a quoted value
      # with stray trailing whitespace (e.g., `work-item: "0042"   `)
      # is correctly classified as already-quoted rather than mis-
      # refused via the embedded-quote heuristic.
      sub(/[ \t]+$/, "", line)
      if (refuses(line)) {
        # Refused — pass through unchanged; warn via stderr.
        print $0
        print "0006-REFUSE: " file " — refused work-item (unsafe value shape)" > "/dev/stderr"
        next
      }
      # Capture first value (raw + semantic-inner) for divergence
      # comparison in END.
      if (!saw_work_item) {
        first_wi = line
        inner_wi = semantic_inner(line)
        saw_work_item = 1
      }
      # If the canonical key also exists in this file (dual-presence),
      # drop the legacy line WITHOUT emitting — the work_item_id:
      # branch is authoritative. Otherwise emit the renamed line.
      if (has_id == "1") { next }
      if (line == "") { print "work_item_id:"; next }
      print "work_item_id: " normalise_value(line)
      next
    }

    # ---- Plan frontmatter: idempotent normalisation of work_item_id ----
    in_frontmatter && /^work_item_id:/ {
      line = $0
      sub(/^work_item_id:[ \t]*/, "", line)
      sub(/[ \t]+$/, "", line)
      if (refuses(line)) {
        # Already-canonical key but unsafe value shape: pass through
        # and warn — same posture as the work-item branch.
        print $0
        print "0006-REFUSE: " file " — refused work_item_id (unsafe value shape)" > "/dev/stderr"
        next
      }
      if (!saw_work_item_id) {
        first_id = line
        inner_id = semantic_inner(line)
        saw_work_item_id = 1
      } else {
        # Subsequent `work_item_id:` lines are dropped — but if their
        # value differs semantically from the first, emit a divergence
        # warning so silent data loss is observable.
        if (semantic_inner(line) != inner_id) {
          print "0006-DIVERGE: " file " — multiple work_item_id values: kept first \"" inner_id "\", dropped \"" semantic_inner(line) "\"" > "/dev/stderr"
        }
        next
      }
      if (line == "") { print "work_item_id:"; next }
      print "work_item_id: " normalise_value(line)
      next
    }

    # ---- Research frontmatter: researcher → author ----
    in_frontmatter && /^researcher:/ {
      line = $0
      sub(/^researcher:[ \t]*/, "", line)
      sub(/[ \t]+$/, "", line)
      if (!saw_researcher) { first_r = line; saw_researcher = 1 }
      # Dual-presence: drop legacy if canonical also exists in
      # frontmatter; otherwise rename and emit.
      if (has_a == "1") { next }
      sub(/^researcher:/, "author:")
      print
      next
    }
    in_frontmatter && /^author:/ {
      line = $0
      sub(/^author:[ \t]*/, "", line)
      sub(/[ \t]+$/, "", line)
      if (!saw_author) { first_a = line; saw_author = 1 }
      print
      next
    }

    # ---- Body label: **Researcher**: → **Author**: (pre-first-H2) ----
    !saw_first_h2 && /^\*\*Researcher\*\*:/ {
      line = $0
      sub(/^\*\*Researcher\*\*:[ \t]*/, "", line)
      sub(/[ \t]+$/, "", line)
      if (!saw_body_researcher) { first_rb = line; saw_body_researcher = 1 }
      # Dual-presence: if `**Author**:` also exists in pre-first-H2
      # region, drop the FIRST `**Researcher**:` occurrence only.
      if (has_ab == "1" && !dropped_first_rb) { dropped_first_rb = 1; next }
      sub(/^\*\*Researcher\*\*:/, "**Author**:")
      print
      next
    }
    !saw_first_h2 && /^\*\*Author\*\*:/ {
      line = $0
      sub(/^\*\*Author\*\*:[ \t]*/, "", line)
      sub(/[ \t]+$/, "", line)
      if (!saw_body_author) { first_ab = line; saw_body_author = 1 }
      print
      next
    }

    { print }

    END {
      # Divergence warnings compare SEMANTIC inner values (outer
      # quotes stripped) so quoted-vs-unquoted equivalents like
      # `"0042"` and `0042` do NOT emit a spurious DIVERGE line. The
      # warning message itself shows the raw values for clarity.
      if (saw_work_item && saw_work_item_id && inner_wi != inner_id) {
        print "0006-DIVERGE: " file " — work-item=" first_wi " vs work_item_id=" first_id " (kept work_item_id)" > "/dev/stderr"
      }
      if (saw_researcher && saw_author && first_r != first_a) {
        print "0006-DIVERGE: " file " — researcher=" first_r " vs author=" first_a " (kept author)" > "/dev/stderr"
      }
      if (saw_body_researcher && saw_body_author && first_rb != first_ab) {
        print "0006-DIVERGE: " file " — **Researcher**=" first_rb " vs **Author**=" first_ab " (kept **Author**)" > "/dev/stderr"
      }
      # If we saw a rewrite-target key ANYWHERE in the file but never
      # entered frontmatter mode, warn the user — likely a malformed
      # file the migration silently skipped. Uses the ungated
      # `*_anywhere` flags set by the top-of-file detection rules,
      # because the gated `saw_*` flags require `in_frontmatter` to
      # be true and would never fire for a file without a `---` fence.
      if (!seen_frontmatter_open && (saw_wi_anywhere || saw_r_anywhere || saw_rb_anywhere)) {
        print "0006-MALFORMED: " file " — legacy key seen but no frontmatter fence (---) detected" > "/dev/stderr"
      }
    }
  '
}

# Extract the frontmatter region (lines between the first two `---`
# fences) from a file. Used to scope dual-presence detection to the
# canonical region, so prose / code-block matches don't trigger false
# positives.
extract_frontmatter() {
  awk '/^---$/ { c++; if (c == 1) next; if (c == 2) exit } c == 1 { print }' "$1"
}

# Extract the pre-first-H2 region (everything before the first
# `^## ` line). Used to scope body-label dual-presence detection.
extract_pre_h2() {
  awk '/^## / { exit } { print }' "$1"
}

# Apply awk_transform to a file, surfacing divergence + refusal +
# malformed-file warnings via log_warn. Returns 0 always (no
# return-code overloading); writes the per-file touched flag (0 or 1)
# to stdout for the caller to accumulate.
rewrite_file() {
  local file="$1"
  # Cheap upstream check — avoid the awk + atomic_write round-trip
  # for clean files. Matches any line awk_transform might act on.
  if ! grep -qE '^(work-item:|work_item_id:|researcher:|author:|\*\*Researcher\*\*:|\*\*Author\*\*:)' "$file" 2>/dev/null; then
    printf '0\n'
    return 0
  fi

  # Pre-scan dual-presence flags inside the canonical regions.
  local fm pre_h2
  fm=$(extract_frontmatter "$file")
  pre_h2=$(extract_pre_h2 "$file")
  local has_wi=0 has_id=0 has_r=0 has_a=0 has_rb=0 has_ab=0
  if printf '%s\n' "$fm" | grep -q '^work-item:';     then has_wi=1; fi
  if printf '%s\n' "$fm" | grep -q '^work_item_id:';  then has_id=1; fi
  if printf '%s\n' "$fm" | grep -q '^researcher:';    then has_r=1;  fi
  if printf '%s\n' "$fm" | grep -q '^author:';        then has_a=1;  fi
  if printf '%s\n' "$pre_h2" | grep -q '^\*\*Researcher\*\*:'; then has_rb=1; fi
  if printf '%s\n' "$pre_h2" | grep -q '^\*\*Author\*\*:';     then has_ab=1; fi

  local tmp_out tmp_err
  tmp_out=$(mktemp)
  tmp_err=$(mktemp)
  awk_transform "$file" "$has_wi" "$has_id" "$has_r" "$has_a" "$has_rb" "$has_ab" \
    < "$file" > "$tmp_out" 2> "$tmp_err"

  # Decide whether the rewrite actually changed anything. If awk
  # produced byte-identical output (e.g., the file already canonical
  # but the upstream grep matched a benign shape), skip atomic_write
  # so idempotent re-runs produce zero churn.
  local touched=0
  if ! cmp -s "$file" "$tmp_out"; then
    atomic_write "$file" < "$tmp_out"
    touched=1
  fi

  # Surface awk's stderr signals via log_warn.
  if [ -s "$tmp_err" ]; then
    while IFS= read -r line; do
      log_warn "0006: ${line}"
    done < "$tmp_err"
  fi
  rm -f "$tmp_out" "$tmp_err"

  printf '%s\n' "$touched"
}

# ---- Walker ---------------------------------------------------------

walk_corpus() {
  local key="$1"
  local rel
  if ! rel="$(resolve_corpus_path "$key")"; then
    echo "0006: rewrote 0 file(s) under <unresolved $key>"
    return 0
  fi
  local abs="$PROJECT_ROOT/$rel"
  if [ ! -d "$abs" ]; then
    log_warn "0006: $key directory does not exist: $rel"
    echo "0006: rewrote 0 file(s) under $rel"
    return 0
  fi
  local rewrote=0 touched
  while IFS= read -r -d '' file; do
    touched=$(rewrite_file "$file")
    # Defensive default — if a transient mktemp / awk failure inside
    # the subshell ate the trailing printf, `touched` is empty and
    # bash arithmetic would abort under `set -u`. Treat missing as 0
    # and log the regression.
    if [[ ! "${touched:-}" =~ ^[0-9]+$ ]]; then
      log_warn "0006: rewrite_file '$file' produced non-numeric touched ('${touched:-<empty>}') — treating as 0"
      touched=0
    fi
    rewrote=$((rewrote + touched))
  done < <(find "$abs" -type f -name '*.md' -print0)
  echo "0006: rewrote $rewrote file(s) under $rel"
}

# Collect canonicalised corpus paths and dedupe before walking, so a
# misconfiguration aliasing two paths.* keys (or two surface forms
# resolving to the same dir) does not double-traverse.
declare -A WALKED
for key in plans research_codebase research_issues; do
  raw_rel="$(cd "$PROJECT_ROOT" && bash "$PLUGIN_ROOT/scripts/config-read-path.sh" "$key" 2>/dev/null || true)"
  if [ -z "$raw_rel" ]; then continue; fi
  canon="$(canonicalise_rel "$raw_rel")"
  if [ -n "${WALKED[$canon]:-}" ]; then
    log_warn "0006: paths.$key aliases paths.${WALKED[$canon]} ($raw_rel -> $canon) — skipping duplicate walk"
    continue
  fi
  WALKED[$canon]="$key"
  walk_corpus "$key"
done

# ---- Userspace template overrides -----------------------------------

# Resolve a userspace template path for <name>. Returns the absolute
# path on stdout if a userspace override (tier-1 templates.<name>: OR
# tier-2 <paths.templates>/<name>.md) exists and is safe. Returns
# nothing (and warns) on misconfiguration; the plugin-default template
# (tier-3) is deliberately excluded from rewriting.
resolve_user_template_path() {
  local name="$1"

  # Tier 1: templates.<name>:
  local tier1
  tier1="$(cd "$PROJECT_ROOT" \
    && bash "$PLUGIN_ROOT/scripts/config-read-value.sh" "templates.$name" 2>/dev/null || true)"
  if [ -n "$tier1" ]; then
    if ! assert_safe_relpath "$tier1" "templates.$name"; then
      return 0
    fi
    local tier1_abs="$PROJECT_ROOT/$tier1"
    if [ -f "$tier1_abs" ]; then
      printf '%s\n' "$tier1_abs"
      return 0
    fi
    # Tier-1 set but target missing: warn + return WITHOUT falling
    # through to tier-2. The user pinned a specific path; honour intent.
    log_warn "0006: templates.$name points at missing file: $tier1 (skipping; not falling through to tier-2)"
    return 0
  fi

  # Tier 2: <paths.templates>/<name>.md
  local tdir_rel
  tdir_rel="$(cd "$PROJECT_ROOT" \
    && bash "$PLUGIN_ROOT/scripts/config-read-path.sh" templates 2>/dev/null || true)"
  if [ -n "$tdir_rel" ]; then
    local tier2_abs="$PROJECT_ROOT/$tdir_rel/$name.md"
    if [ -f "$tier2_abs" ]; then
      printf '%s\n' "$tier2_abs"
    fi
  fi
}

# Rewrite a userspace template if one resolves. Helper exits 0 always
# (touched signalled via stdout) so set -e cannot abort the migration
# when the template is already canonical.
declare -A TEMPLATE_PATHS
for name in plan codebase-research rca; do
  path=$(resolve_user_template_path "$name")
  if [ -n "$path" ]; then
    if [ -n "${TEMPLATE_PATHS[$path]:-}" ]; then
      log_warn "0006: templates.$name and templates.${TEMPLATE_PATHS[$path]} resolve to the same file ($path) — skipping duplicate rewrite"
      continue
    fi
    TEMPLATE_PATHS[$path]="$name"
    touched=$(rewrite_file "$path")
    echo "0006: template $name (tier-resolved $path): touched=$touched"
  fi
done
```

Notes on the migration:

- **Single-pass awk transform**: all three rename axes (plan
  frontmatter, research frontmatter, body label) AND dual-presence
  cleanup are performed inside one awk pass that reads from stdin
  and writes to stdout. The caller (`rewrite_file`) feeds the file
  in, captures stdout to a temp, and commits via one `atomic_write`
  — restoring per-file atomicity. Idempotent re-runs are short-
  circuited via an upstream `grep -qE` check on the input file
  AND a `cmp -s` byte-equality check after the awk runs, so a
  clean file produces zero `atomic_write` calls and a refused-shape
  file produces zero churn after the first run.
- **Frontmatter-anchored dual-presence cleanup**: because the
  dual-presence handling runs inside awk and is gated by
  `in_frontmatter` (or `!saw_first_h2` for the body label), legacy
  keys / labels appearing in body prose (quoted snippets, code
  blocks, narrative examples) are **never** stripped. The
  unanchored `grep -v` shell pre-pass that risked corrupting prose
  is gone.
- **`work_item_id:` idempotent normalisation**: in addition to
  rewriting `^work-item:` lines, the awk also normalises the
  value-shape of any `^work_item_id:` line it sees in frontmatter.
  A partial-prior-run plan with a surviving unquoted
  `work_item_id: 0042` is canonicalised to `work_item_id: "0042"`
  in the same pass, so AC #6 holds for partial-prior-run plans
  whether the survivor is quoted, unquoted, or empty.
- **Body-label rewrite is anchored**: `^\*\*Researcher\*\*:` is
  rewritten ONLY before the first `## ` heading, matching the
  producer template's emission position. Prose examples after the
  first H2 (or inside fenced code blocks past the header) are left
  untouched. First-occurrence-drop for the dual-presence case is
  implemented inside awk (portable across awk dialects); the
  previous `sed '0,/pat/{//d;}'` (GNU-only, BSD/macOS-incompatible)
  is gone.
- **Refusal posture without persistent file state**: inline `#`
  comments and embedded `"` in unquoted scalars are REFUSED — the
  legacy line is passed through unchanged and a `0006-REFUSE:`
  stderr line is emitted. The caller pipes that into `log_warn`.
  **No sentinel is written into the file**, so idempotent re-runs
  produce the same warning each time and `cmp -s` reports the file
  as byte-unchanged.
- **Divergence and malformed-file signalling via awk stderr**:
  awk's END block compares the first-seen legacy value to the
  first-seen canonical value (per axis) and emits a `0006-DIVERGE:`
  stderr line on mismatch. A `0006-MALFORMED:` line is emitted at
  EOF if a rewrite-target key was seen but the frontmatter fence
  (`---`) was never detected — addressing the silent-no-op risk
  for files without a leading `---`.
- **`paths.*` alias deduplication via canonicalised keys**: the
  walker tracks paths in `WALKED` keyed on the `realpath`-resolved
  absolute path (or `cd && pwd -P` fallback), so two surface
  variants (`docs/research`, `./docs/research`) resolving to the
  same directory collapse to one walk and emit a `log_warn`. Same
  pattern applied to template paths.
- **`assert_safe_relpath` is shared and structurally honest**: the
  same traversal-guard + canonical-path-inside-project check is
  applied to all four configured paths and to tier-1 template
  overrides. The full-leaf-included `realpath -m` resolution catches
  symlinked basenames (not just symlinked parents); the explicit
  `if abs_parent=...; then` form removes the dead `|| canonical=""`
  fallback. The plugin-default tier-3 template is deliberately not
  in scope.
- **Single misconfigured path warns rather than aborts**:
  `resolve_corpus_path` `log_warn`s and returns 1 for unsafe /
  empty values; `walk_corpus` reports `rewrote 0 file(s) under
  <unresolved $key>` and continues with the other corpora. This
  matches the missing-directory branch's policy and avoids the
  prior `log_die` failure mode that broke the migration on a
  single typo'd config key.
- **Defensive numeric guard in walker**: `walk_corpus` validates
  `touched` is `^[0-9]+$` before arithmetic. A transient `mktemp`
  or awk failure inside `rewrite_file` that ate the trailing
  `printf` is logged and treated as 0 rather than aborting the
  migration mid-walk under `set -u`.
- **`find -type f`** ensures non-regular file matches (directories,
  symlinks, sockets) are skipped.

### Success Criteria

#### Automated Verification

- [ ] All new 0006 tests pass:
  `bash skills/config/migrate/scripts/test-migrate.sh` exits 0 and
  prints each new test as `PASS`.
- [ ] Migration file is bash-syntax-clean:
  `bash -n skills/config/migrate/migrations/0006-canonicalise-work-item-id-and-author.sh`.
- [ ] Driver discovers 0006 in preview banner: from a clean
  repository, the dry-run preview lists 0006 in pending migrations.
- [ ] Migration only rewrites the targeted keys: a fixture containing
  unrelated frontmatter (e.g. `parent: "0042"`, `tags: [foo]`,
  `last_updated_by: Toby`) is byte-identical post-run except for the
  two renamed keys (and their body label, for research).

#### Manual Verification

- [ ] Reading the migration confirms the dual-presence shape from
  0005 is applied per pass (plan frontmatter; research frontmatter;
  research body label) — each gated by its own outer `grep -q` guard.
- [ ] Reading the tests confirms each branch is exercised at least
  once: clean rename across all three corpora; unquoted-value
  normalisation; empty-value preservation; partial-prior-run
  (matching + divergent) for plan/research/body-label; `paths.*`
  overrides for each corpus; missing/empty directories; userspace
  template overrides at tier 1, tier 2, both, and missing;
  idempotence (`tree_hash` byte-identical).

---

## Phase 2: Apply migration 0006 to this repo's corpus

### Overview

Run `/accelerator:migrate` against this repo so all 86 plans carry
`work_item_id:` frontmatter and all 52 codebase-research files carry
`author:` (plus `**Author**:` body labels where present). This step
is mechanical but must occur before any consumer code is rewritten —
otherwise the dogfood corpus becomes unreadable by the (still legacy)
visualiser.

### Changes Required

#### 1. Pre-flight

- Commit Phase 1 (migration script + tests + fixtures) as its own jj
  revision before running `/accelerator:migrate`.
- Confirm `jj status` is clean — both for `meta/` (the framework
  dirty-tree guard covers this) AND for `templates/` and `skills/`
  (the framework guard does NOT cover these, but this particular
  migration mutates `templates/` during Phase 3 and the visualiser
  Rust code during Phase 4; a dirty tree there would entangle
  Phase 2's mechanical migration diff with in-progress edits).
- **Recovery procedure**: if the migration aborts mid-walk (SIGINT,
  disk full, an unexpected `set -e` exit) or produces a diff you
  did not expect, the working copy is recoverable via
  `jj abandon @` (which discards the current change and restores
  the parent revision's state). Phase 1 was committed as a separate
  revision precisely so Phase 2's diff is isolatable. If you only
  want to discard specific files, `jj restore --from @-` against
  the listed paths achieves the same.
- Snapshot the pre-rename corpus shape:

  ```bash
  rg -c '^work-item:' meta/plans/        # expect 86
  rg -c '^researcher:' meta/research/codebase/  # expect 52
  rg -c '^\*\*Researcher\*\*:' meta/research/codebase/  # expect 52
  rg -c '^researcher:' meta/research/issues/    # expect 0
  ```

#### 2. Run the migration

```bash
/accelerator:migrate
```

Confirm stdout reports the per-corpus rewrite count:

```
0006: rewrote 86 file(s) under meta/plans
0006: rewrote 52 file(s) under meta/research/codebase
0006: rewrote 0 file(s) under meta/research/issues
```

#### 3. Verification of corpus state

```bash
# Legacy keys gone from each corpus
rg -n '^work-item:' meta/plans/                       # expect 0
rg -n '^researcher:' meta/research/codebase/          # expect 0
rg -n '^\*\*Researcher\*\*:' meta/research/codebase/  # expect 0
rg -n '^researcher:' meta/research/issues/            # expect 0
rg -n '^\*\*Researcher\*\*:' meta/research/issues/    # expect 0

# Canonical keys present
rg -c '^work_item_id:' meta/plans/   # expect 86
rg -c '^author:' meta/research/codebase/             # expect 52
rg -c '^\*\*Author\*\*:' meta/research/codebase/     # expect 52

# All work_item_id values are quoted YAML strings
rg -n '^work_item_id: [^"]' meta/plans/    # expect 0

# Migration recorded as applied
grep -F '0006-canonicalise-work-item-id-and-author' \
  .accelerator/state/migrations-applied
```

#### 4. Re-run idempotency check

```bash
/accelerator:migrate
# Expect: exit 0, driver reports 0006 as already applied.
# Capture jj's status to a file (so an upstream jj failure shows in
# the artefact rather than being swallowed by grep's "no match" exit
# code) and assert the relevant paths are absent.
jj status > /tmp/0006-phase2-rerun-status.txt
if grep -qE 'meta/(plans|research)/' /tmp/0006-phase2-rerun-status.txt; then
  echo "FAIL: post-rerun changes detected"
  cat /tmp/0006-phase2-rerun-status.txt
  exit 1
else
  echo "PASS: idempotent"
fi
```

### Success Criteria

#### Automated Verification

- [ ] `rg -n '^work-item:' meta/plans/` returns zero hits.
- [ ] `rg -n '^researcher:' meta/research/` returns zero hits.
- [ ] `rg -n '^\*\*Researcher\*\*:' meta/research/` returns zero hits.
- [ ] All 86 plan files contain a `^work_item_id:` line with a quoted
  string value.
- [ ] All 52 codebase-research files contain a `^author:` line and a
  `^\*\*Author\*\*:` body label.
- [ ] `.accelerator/state/migrations-applied` contains
  `0006-canonicalise-work-item-id-and-author`.
- [ ] Re-running `/accelerator:migrate` produces exit 0 and zero
  further changes under `meta/`.

#### Manual Verification

- [ ] Spot-check three representative files:
  `meta/plans/2026-05-20-0063-rename-work-item-type-to-kind.md`,
  `meta/research/codebase/2026-05-20-0063-rename-work-item-type-to-kind.md`,
  `meta/research/codebase/2026-05-21-0064-canonicalise-work-item-id-and-author-fields.md`.
  Confirm canonical key names and that no other content changed.
- [ ] `jj diff meta/` review shows only line-level frontmatter / body
  label rewrites — no other content changes.

---

## Phase 3: Update producer templates

### Overview

Rewrite the three affected templates so newly-created plans and
research artefacts emit the canonical names from now on. No
executable tests gate this phase; the AC #1/#2 greps serve as the
oracle.

### Pre-edit baseline grep

```bash
rg -n '^work-item:|^researcher:|^\*\*Researcher\*\*:' templates/
# Expect: 7 hits across the three templates.

# Defensive sweep of producer SKILL.md files for prose references to
# the legacy field names. These don't emit the keys (the templates do)
# but may discuss them in instructions to the LLM. Stale prose here
# would confuse the agent post-rename.
rg -n 'work-item:|researcher:|\*\*Researcher\*\*:' \
  skills/planning/create-plan/SKILL.md \
  skills/research/research-codebase/SKILL.md \
  skills/research/research-issue/SKILL.md
# Audit any hits; rewrite stale references to the canonical names.
```

### Changes Required

#### 1. `templates/plan.md`

- Line 5: `work-item: "{work-item reference, if any}"` →
  `work_item_id: "{work-item reference, if any}"`.

#### 2. `templates/codebase-research.md`

- Line 3: `researcher: [Git author]` → `author: [Git author]`.
- Line 17: `**Researcher**: [Researcher name from thoughts status]`
  → `**Author**: [Author name from thoughts status]`.
- Line 11 (`last_updated_by: [Researcher name]`) is **NOT touched**
  in this story. The field name `last_updated_by` is unchanged and
  the placeholder copy belongs to the broader template-rewrite work
  in 0065. See §"What We're NOT Doing" for the scope boundary.

#### 3. `templates/rca.md`

- Line 3: `researcher: [Git author]` → `author: [Git author]`.
- Line 17: `**Researcher**: [Researcher name]` → `**Author**: [Author name]`.
- Line 11 (`last_updated_by: [Researcher name]`) is **NOT touched**
  in this story (deferred to 0065, as above).

### Verification grep (post-edit)

```bash
rg -n '^work-item:|^researcher:|^\*\*Researcher\*\*:' templates/
# Expect: zero hits.
rg -n '^work_item_id:|^author:|^\*\*Author\*\*:' templates/
# Expect: at least 5 hits (one work_item_id, two author, two **Author**).
```

### Success Criteria

#### Automated Verification

- [ ] `rg -n 'work-item:' templates/` returns zero hits.
- [ ] `rg -n 'researcher:' templates/` returns zero hits.
- [ ] `rg -n '\*\*Researcher\*\*:' templates/` returns zero hits.
- [ ] Each updated template still has well-formed YAML frontmatter
  delimited by `---` lines (visual inspection or a simple `awk`
  check).

#### Manual Verification

- [ ] Run `/accelerator:create-plan` end-to-end (without applying
  the produced plan); confirm the generated file emits
  `work_item_id:` and no `work-item:`. (Spot-check via `jj diff`
  before discarding.)
- [ ] Read each of the three templates end-to-end. Confirm body
  labels, placeholder text, and trailing comments are consistent.
  No incidental edits.

---

## Phase 4: Visualiser server — TDD

### Overview

Update `frontmatter.rs:326` so the HashMap lookup uses
`"work_item_id"` instead of `"work-item"`. Update all inline YAML
test fixtures and on-disk fixture files alongside, with the YAML
fixture updates landing **first** so `cargo test` fails until the
production read site is updated to match.

### Pre-edit baseline

```bash
cd skills/visualisation/visualise/server
cargo test --no-run 2>/dev/null   # build only; baseline
```

### Changes Required

#### 1. Update inline YAML test fixtures FIRST (expect to fail)

**File**: `skills/visualisation/visualise/server/src/frontmatter.rs`
**Changes**: At lines 441, 455, 523 (inline test YAML), replace
`work-item: "0042"` (and any other literal `work-item:` line) with
`work_item_id: "0042"`. Adjust the assertion expectations only if a
particular test asserts the *legacy* key name in error messages — if
so, change the assertion to reference the new key. The doc comment
at 297-302 and the inline comment at 325 also update to reference
`work_item_id:` instead of `work-item:`.

**File**: `skills/visualisation/visualise/server/src/indexer.rs`
**Changes**: At lines 2585, 2641, 2660, 2687 (inline YAML test
fixtures), replace `work-item:` with `work_item_id:`. The doc
comment block at 212-216 (describing the `work_item_refs_by_target`
reverse index) updates its prose reference from `work-item:` to
`work_item_id:`. The Rust **identifier names** (`work_item_refs`,
`work_item_refs_by_target`, `work_item_refs_by_id`, etc.) do **not**
change.

**File**: `skills/visualisation/visualise/server/src/patcher.rs`
**Changes**: At lines 259-267 and 407-415, the pass-through test
fixtures and their assertions reference `work-item:`. Replace with
`work_item_id:`. The test rationale (verifying that `patch_status`
preserves unrelated frontmatter byte-for-byte) still holds with the
new key name.

**File**: `skills/visualisation/visualise/server/src/api/related.rs`
**Changes**: The doc comment at line 78 references `work-item:`;
update to `work_item_id:`. No code change required.

#### 2. Update on-disk test fixtures

**Files**:
- `skills/visualisation/visualise/server/tests/fixtures/meta/work/0001-first-work-item.md:4`
- `skills/visualisation/visualise/server/tests/fixtures/meta/work/0002-second-work-item.md:4`
- `skills/visualisation/visualise/server/tests/fixtures/meta/work/0003-third-work-item.md:3`
- `skills/visualisation/visualise/server/tests/fixtures/meta/work/0005-sse-test-work-item.md:4`
- `skills/visualisation/visualise/server/tests/fixtures/meta/work/0006-conflict-test-work-item.md:4`

**Current shape**: these fixtures carry **unquoted integer** values
(`work-item: 1`, `work-item: 2`, etc.) — NOT quoted strings.

**Changes**: Replace `work-item:` with `work_item_id:` at each
location, **preserving the unquoted integer value** (e.g.
`work-item: 1` → `work_item_id: 1`). Preserving unquoted form keeps
the existing `read_ref_keys` numeric→string coercion code path
exercised by these tests. If a future cleanup decides to canonicalise
to quoted-string form, add a separate fixture file (or inline YAML
test case) that retains the unquoted-numeric input shape.

#### 3. Run cargo test; expect failure

```bash
cd skills/visualisation/visualise/server && cargo test
# Expect: tests that exercise the read_ref_keys path (or anything
# downstream like work_item_refs_by_id integration tests) FAIL
# because production still reads m.get("work-item").
```

#### 4. Update production read site

**File**: `skills/visualisation/visualise/server/src/frontmatter.rs`
**Changes**:

- Line 326: `if let Some(v) = m.get("work-item") {` →
  `if let Some(v) = m.get("work_item_id") {`.
- Add a **transitional fallback** branch reading the legacy
  `"work-item"` key, between the new `work_item_id:` branch and the
  existing `ticket:` fallback. This gives downstream user repos a
  soft-landing window between plugin upgrade and
  `/accelerator:migrate` execution (the AC #7 step 3 broken-link
  demo would otherwise affect any user who defers the migration).
  The fallback ships for ONE release cycle and is removed in the
  follow-up that closes story 0070:

  ```text
  // Inside read_ref_keys() — pseudocode showing the branch shape.
  if let Some(v) = m.get("work_item_id") {
      // ... existing aggregation logic
  } else if let Some(v) = m.get("work-item") {
      // Transitional fallback: pre-migration repos still emit the
      // legacy `work-item:` key. Remove in the release that closes
      // story 0070 — by then all userspace repos should have run
      // `/accelerator:migrate` at least once.
      // ... same aggregation logic as the new-key branch
  } else if let Some(v) = m.get("ticket") {
      // Legacy fallback for repos that never migrated past
      // `ticket:` — preserved indefinitely per migration 0001.
      // ... existing logic
  }
  ```

- Keep the `else if let Some(v) = m.get("ticket")` legacy-fallback
  branch unchanged.
- Add a new positive Rust test asserting that a fixture carrying
  ONLY the legacy `work-item:` key (no `work_item_id:`) is still
  read correctly via the transitional fallback. This pins the
  fallback against accidental removal until the follow-up.

#### 5. Run cargo test; expect pass

```bash
cd skills/visualisation/visualise/server && cargo test
# Expect: all tests pass.
```

#### 6. Update workspace clippy / fmt as needed

```bash
cd skills/visualisation/visualise/server && cargo clippy --all-targets -- -D warnings
cd skills/visualisation/visualise/server && cargo fmt --check
```

### Success Criteria

#### Automated Verification

- [ ] `cargo test` in
  `skills/visualisation/visualise/server/` exits 0.
- [ ] `cargo clippy --all-targets -- -D warnings` exits 0.
- [ ] `cargo fmt --check` exits 0.
- [ ] `rg -n '"work-item"' skills/visualisation/visualise/server/src/`
  returns zero hits.
- [ ] `rg -n '^work-item:' skills/visualisation/visualise/server/`
  returns zero hits (covers inline + on-disk fixtures).

#### Manual Verification

- [ ] Read the updated `frontmatter.rs:326` and confirm only the
  string literal changed; the surrounding ref-aggregation logic is
  byte-identical.
- [ ] Confirm the Rust identifier `work_item_refs` (and friends)
  remain unchanged — research §"Architecture Insights" point 3
  rationale.

---

## Phase 5: Update downstream story 0070

### Overview

0070 currently lists the two renames done by 0064 in its
Requirements/AC. With 0064 landing, 0070 must stop duplicating those
steps. Mirrors the equivalent housekeeping that 0063 performed on
0070's `type:` → `kind:` line.

### Changes Required

**File**: `meta/work/0070-ship-meta-corpus-unified-schema-migration.md`
**Changes**: Remove from the Requirements section the two lines that
specify renaming `work-item:` → `work_item_id:` and `researcher:` →
`author:`. Remove the corresponding lines from the Acceptance Criteria
section. Add one note in the story's "Dependencies" or "Drafting Notes"
section explaining that these renames are now owned by 0064 (with
migration 0006) and are not within 0070's scope.

Use `Read` to locate the precise lines before editing — the work item
has evolved since the research snapshot and the line numbers may have
drifted. Verify after editing:

```bash
rg -n 'work-item:|researcher:' meta/work/0070-ship-meta-corpus-unified-schema-migration.md
# Expect: zero hits, or only references to "0064 (canonicalise work_item_id and author)".
```

#### 2. Defensive sweep of other open work items

```bash
# Identify any other open work items still referencing the legacy keys
# in their Requirements/AC. Audit each hit; rewrite if it's stale, or
# document why it must remain as-is (historical reference, quoted past
# prose).
rg -n 'work-item:|researcher:|\*\*Researcher\*\*:' meta/work/ \
  | rg -v '0070-ship-meta-corpus-unified-schema-migration.md'
```

Acceptable hits: references in the form `"0064 (canonicalise
work_item_id and author)"`, quoted historical prose in changelog-
style sections, or research-doc citations. Any present-tense
Requirements/AC reference should be rewritten to the canonical
name.

#### 3. ADR sweep

The following ADRs reference `work-item:` (or `researcher:`) in
present-tense canonical-schema descriptions and become stale post-
migration:

- `meta/decisions/ADR-0025-work-item-cross-ref-aggregation.md` —
  rewrite present-tense `work-item:` references to `work_item_id:`.
- `meta/decisions/ADR-0034-typed-linkage-vocabulary.md:71` — same.

For each ADR: update the present-tense schema descriptions; add a
brief editorial note (e.g. "Renamed from `work-item:` to
`work_item_id:` by migration 0006; see ADR-0033 §Field-name
conflicts.") where the rename context is relevant. Verify post-edit:

```bash
rg -n 'work-item:|researcher:' meta/decisions/ \
  | rg -v 'Renamed from|formerly|historically|originally'
# Expect: zero hits — only retrospective/editorial references remain.
```

### Success Criteria

#### Automated Verification

- [ ] `rg -n '^- Renames .*(work-item:|researcher:)' meta/work/0070-*.md`
  returns zero hits.

#### Manual Verification

- [ ] Read the updated 0070 work item; confirm its Requirements and
  AC sections still scope coherently to "the rest of the unified-schema
  migration" without the two now-redundant lines.
- [ ] Confirm the dependency reference to 0064 reads naturally.

---

## Phase 6: Final verification

### Overview

Run the full AC-grep battery across the whole repo. Run the upgrade-
path fixture validation (AC #7). Run the eval and test suites. Run
the visualiser manual smoke test for AC #6.

### AC verification commands

```bash
# AC #1 — work-item: gone from producer + consumer surface
rg -n 'work-item:' \
  templates/ skills/ scripts/ \
  skills/visualisation/visualise/server/src/ \
  skills/visualisation/visualise/frontend/src/ \
  | rg -v 'skills/config/migrate/scripts/test-fixtures/0006/'
# Expect: zero hits.

# AC #2 — researcher: gone from producer + consumer surface
rg -n 'researcher:' \
  templates/ skills/ scripts/ \
  skills/visualisation/visualise/server/src/ \
  skills/visualisation/visualise/frontend/src/ \
  | rg -v 'web-search-researcher|skills/config/migrate/scripts/test-fixtures/0006/'
# Expect: zero hits (the web-search-researcher agent name is unrelated;
# the 0006 test fixtures intentionally contain legacy keys as input).

# Body-label sweep (defence-in-depth)
rg -n '^\*\*Researcher\*\*:' templates/ skills/ meta/research/ meta/work/
# Expect: zero hits outside skills/config/migrate/scripts/test-fixtures/0006/.

# AC #3 — corpus migration applied
rg -n '^work-item:' meta/plans/                       # expect 0
rg -n '^researcher:' meta/research/codebase/          # expect 0
rg -n '^researcher:' meta/research/issues/            # expect 0

# AC #6 — work_item_id values remain quoted
rg -n '^work_item_id: [^"]' meta/plans/  # expect 0

# Migration idempotency
/accelerator:migrate
# Explicit, fail-loud verification — capture jj's status to a file
# (so an upstream jj failure shows in the artefact rather than being
# swallowed by grep's "no match" exit code) and assert it's empty.
jj status > /tmp/0006-post-rerun-status.txt
if grep -qE 'meta/(plans|research)/' /tmp/0006-post-rerun-status.txt; then
  echo "FAIL: post-rerun changes detected"
  cat /tmp/0006-post-rerun-status.txt
  exit 1
else
  echo "PASS: idempotent"
fi
```

### AC #7 — Upgrade-path fixture validation

Create the upgrade-path fixture and exercise the four-step sequence
end-to-end:

**Fixture**: `skills/config/migrate/scripts/test-fixtures/0006/upgrade-path/`
(matches the `0006/<scenario>/` convention used by Phase 1 fixtures
and the 0004/0005 precedent — NOT a per-migration long-form
directory)

Contents:
- `meta/plans/0001-legacy-plan.md` with `work-item: "0001"`.
- `meta/research/codebase/2026-01-01-legacy-research.md` with
  `researcher: Toby Clemson` and `**Researcher**: Toby Clemson`.
- `meta/research/issues/2026-01-02-legacy-rca.md` with the same
  legacy shape.
- `.accelerator/templates/plan.md` — userspace template override
  with the legacy `work-item:` frontmatter.

Steps (encode as a new test harness function or run by hand):

1. Stage the fixture into a `mktemp -d` work repo.
2. Set `CLAUDE_PLUGIN_ROOT` to a pre-rename plugin version (or skip
   this step if the test simulates the upgrade by starting with the
   migration disabled). Confirm the visualiser, when pointed at this
   repo, reads `work-item:` from the legacy plan and renders the
   kanban card with its cross-ref intact.
3. Switch `CLAUDE_PLUGIN_ROOT` to the current (post-rename) version.
   Confirm the kanban card now shows a broken-link badge (the new
   read site looks for `work_item_id:` and finds none).
4. Run `/accelerator:migrate` against the fixture. Confirm the
   migration rewrites plan + research + RCA frontmatter, the
   userspace template override, and the body label, and that the
   kanban card is once again whole.

### AC #6 — Visualiser manual smoke test

After Phases 2 + 3 + 4 have landed and a fresh visualiser bundle is
built:

1. Start the visualiser dev server:
   `cd skills/visualisation/visualise/frontend && npm run dev`.
2. Open `/kanban`. Confirm all previously-visible work items appear
   with their cross-references intact (no broken-link badges).
3. Click into the plan detail view for plan 0063
   (`meta/plans/2026-05-20-0063-rename-work-item-type-to-kind.md`).
   Confirm its parent work-item link points to 0057 and renders
   without a broken-link badge.
4. Sweep all plan detail pages; confirm none shows a broken-link
   badge in place of its parent work-item link.
5. Open the codebase-research listing; confirm each row shows the
   correct `author:` value populated from the renamed field.

### Test suites

```bash
# Migration framework regression
bash skills/config/migrate/scripts/test-migrate.sh
# Expect: all tests (including new 0006 ones) pass.

# Visualiser server
cd skills/visualisation/visualise/server && cargo test
# Expect: all tests pass.

# Visualiser frontend (no production code change but type-check
# because TS may have absorbed shape changes via wire types)
cd skills/visualisation/visualise/frontend && npm test && npm run typecheck
# Expect: pass — no regressions.

# Eval suites (if any of the work-skill or lens evals reference these
# field names — none should, per research, but verify)
rg -n 'work-item|researcher' skills/work/*/evals/ skills/review/lenses/*/evals/
# Expect: any hits are either web-search-researcher agent-name
# mentions or unrelated; no field-name references.
```

### Success Criteria

#### Automated Verification

- [ ] AC #1 grep returns zero hits in the producer + consumer surface
  (modulo the 0006 test-fixture exclusion).
- [ ] AC #2 grep returns zero hits in the producer + consumer surface
  (modulo `web-search-researcher` and the 0006 test-fixture exclusion).
- [ ] Whole-repo `**Researcher**:` body-label sweep returns zero hits
  outside test fixtures.
- [ ] AC #3 corpus grep returns zero hits for legacy keys; canonical
  keys are present in the expected counts (86 plans, 52 research).
- [ ] AC #6 quoted-value check: zero unquoted `work_item_id:` values
  in `meta/plans/`.
- [ ] `bash skills/config/migrate/scripts/test-migrate.sh` exits 0.
- [ ] `cargo test`, `cargo clippy`, `cargo fmt --check` in
  `skills/visualisation/visualise/server/` all exit 0.
- [ ] `npm test` and `npm run typecheck` in
  `skills/visualisation/visualise/frontend/` exit 0.
- [ ] Re-running `/accelerator:migrate` produces exit 0 and no further
  changes.
- [ ] AC #7 upgrade-path fixture sequence runs end-to-end without
  manual intervention; final state matches the post-rename canonical
  shape.

#### Manual Verification

- [ ] AC #6 visualiser smoke test passes per the steps above.
- [ ] Reading `jj diff` for `meta/plans/`, `meta/research/codebase/`,
  `templates/`, and `skills/visualisation/visualise/server/src/`
  confirms only the targeted line-level rewrites — no incidental
  edits.
- [ ] The 0070 work item reads coherently after the two now-redundant
  lines have been removed.

---

## Testing Strategy

### Unit-level tests (driven by TDD in Phases 1 and 4)

- **Migration 0006**: ~20 branch tests in `test-migrate.sh` covering
  clean rename across three corpora, quote-normalisation, partial
  prior runs (matching + divergent) for plan/research/body-label,
  `paths.*` overrides for each corpus, missing/empty directories,
  userspace template overrides at tier 1 / tier 2 / both / missing,
  and idempotence (`tree_hash` byte-identical).
- **Visualiser server**: existing `frontmatter.rs`/`indexer.rs`/
  `patcher.rs` tests, updated to use the new key name in their
  inline + on-disk fixtures, exercise the read path. Adding a new
  test that explicitly asserts the legacy key name `m.get("work-item")`
  returns no ref (i.e. only `work_item_id` is honoured) is optional
  defence-in-depth; the existing tests already verify the new key
  is read correctly.

### Integration tests

- **End-to-end migration**: Phase 2 runs `/accelerator:migrate`
  against this repo (~138 files) exercising the whole driver +
  migration stack including the multi-corpus walk and (no-op) on
  `research_issues`.
- **Upgrade-path fixture (AC #7)**: Phase 6 runs the four-step
  sequence (pre-rename plugin → post-rename plugin → migrate →
  verify) against a fresh fixture, covering all three corpora and a
  userspace template override.

### Manual testing steps

1. Open the visualiser kanban board at `/kanban`. Confirm every
   work-item card and every plan detail page renders without
   broken-link badges.
2. Open `meta/plans/2026-05-20-0063-rename-work-item-type-to-kind.md`
   in the visualiser; confirm `work_item_id` is rendered as a
   cross-ref to 0057.
3. Run `/accelerator:create-plan @meta/work/0072-foo.md` (or any
   draft work-item); confirm the generated plan emits
   `work_item_id:` not `work-item:`.
4. Run `/accelerator:create-codebase-research` or equivalent (if
   such an entry point exists); confirm `author:` is emitted, not
   `researcher:`.

## Performance Considerations

None. The migration is a ~138-file sed/awk pass per corpus —
sub-second on any modern disk. The visualiser server change is a
single string-literal replacement. Template content rewrite is
constant-time per template file (3 files at most). No hot-path
implications.

## Migration Notes

This plan **is** the migration. The downstream story 0070 must drop
its own `work-item:` → `work_item_id:` and `researcher:` → `author:`
steps; Phase 5 of this plan performs that edit.

When migration 0006 ships to user repos, the next
`/accelerator:migrate` run applies it automatically. The driver's
dirty-tree guard and migration idempotency contract make this safe.
User repos with any of the four overrides
(`paths.plans`, `paths.research_codebase`, `paths.research_issues`,
`paths.templates`, or `templates.<plan|codebase-research|rca>:`) have
their custom paths honoured.

### Versioning

This rename is a **breaking schema change**, but it ships during the
plugin's `1.21.0-pre.N` pre-release series. The PR that lands this
plan bumps the version to the next `-pre.N+1` increment (consistent
with the rest of the pre-release stream); the breaking nature is
called out in the CHANGELOG rather than in the version number alone.
When the pre-release series stabilises into a `1.x.y` release, the
CHANGELOG entry tying that release to the `work-item:`/`researcher:`
rename is the canonical migration signal for downstream consumers
pinning the plugin by major/minor version.

The visualiser's **transitional `work-item:` fallback** (Phase 4,
step 4) softens the upgrade-window break: user repos that upgrade the
plugin but defer `/accelerator:migrate` still render their kanban
correctly, at the cost of one extra branch in `read_ref_keys`. The
fallback is scheduled for removal in the release that closes story
0070; the rust code comment names that follow-up explicitly.

### Breaking change for downstream user repos

This rename is a **breaking schema change** to plan and research/RCA
frontmatter. After upgrading the plugin and before running
`/accelerator:migrate`, user repos will experience a window where:

- For ONE release cycle (this release): the visualiser's transitional
  `work-item:` fallback handles unmigrated plans, so the kanban view
  renders correctly even before `/accelerator:migrate` has been run.
  In the follow-up release that closes story 0070, the fallback is
  removed; user repos that have not run the migration by then will
  see broken-link badges on plan cards pointing at work items.
- The visualiser server falls back to `ticket:` if present, otherwise
  records no cross-ref for the plan.
- Userspace template overrides at
  `.accelerator/templates/plan.md`,
  `.accelerator/templates/codebase-research.md`, or
  `.accelerator/templates/rca.md` will continue to emit the legacy
  keys until the migration runs (which rewrites them in place).
- The dirty-tree guard refuses to run the migration if `meta/` has
  uncommitted changes — users must commit or set
  `ACCELERATOR_MIGRATE_FORCE=1` to proceed.
- Users running the visualiser dev server should restart it after
  `/accelerator:migrate` completes.

This story's PR description and the next plugin release's CHANGELOG
must call out:

1. The breaking rename of plan `work-item:` → `work_item_id:` and
   research/RCA `researcher:` → `author:`.
2. The body-label rewrite `**Researcher**:` → `**Author**:` in
   research/RCA artefacts.
3. The need to run `/accelerator:migrate` immediately after upgrade,
   especially for repos with userspace template overrides.
4. The body-label rewrite is **anchored to the pre-first-`## `
   region** in each research/RCA file — matching the producer
   template's emission position between the closing frontmatter
   `---` and the first `## Summary` heading. Body-prose
   `**Researcher**:` occurrences after the first H2 (or inside
   fenced code blocks past the header) are **NOT** rewritten.
   Users with research artefacts whose first H2 is unusually late
   in the file (or absent entirely — see the
   `body-label-anchored-no-h2` fixture) should spot-check the
   migration diff for unintended pre-H2 rewrites.
5. Refused shapes (inline `#` comments or embedded `"` in unquoted
   `work-item:` / `work_item_id:` values) are passed through
   unchanged with a `log_warn` to stderr — the migration deliberately
   does not silently mangle these. The user must hand-fix the
   shape and re-run.

## References

- Source: `meta/work/0064-canonicalise-work-item-id-and-author-fields.md`
- Research: `meta/research/codebase/2026-05-21-0064-canonicalise-work-item-id-and-author-fields.md`
- Parent epic: `meta/work/0057-unified-artifact-frontmatter-and-typed-cross-linking.md`
- Base schema ADR: `meta/decisions/ADR-0033-unified-base-frontmatter-schema.md`
- Migration framework: `meta/decisions/ADR-0023-meta-directory-migration-framework.md`
- Migration precedent: `skills/config/migrate/migrations/0005-rename-work-item-type-to-kind.sh`
- Template-rename precedent (file-path only): `skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh:419-430`
- Sibling rename: `meta/work/0063-rename-work-item-type-to-kind.md` (plan: `meta/plans/2026-05-20-0063-rename-work-item-type-to-kind.md`)
- Blocked successor: `meta/work/0065-update-artifact-templates-to-unified-schema.md`
- Broader workstream (housekeeping target in Phase 5): `meta/work/0070-ship-meta-corpus-unified-schema-migration.md`
- Visualiser read site: `skills/visualisation/visualise/server/src/frontmatter.rs:326`
- Atomic rewrite helper: `scripts/atomic-common.sh:16-32` (`atomic_write`)
- Config-path resolver: `scripts/config-read-path.sh`
- Config-value resolver: `scripts/config-read-value.sh`
- Template resolver: `scripts/config-read-template.sh:11-15`
- Test harness: `skills/config/migrate/scripts/test-migrate.sh:1255-1454` (0005 block as template)
