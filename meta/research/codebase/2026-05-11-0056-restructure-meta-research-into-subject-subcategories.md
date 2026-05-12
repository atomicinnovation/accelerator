---
date: 2026-05-11T23:54:00+01:00
researcher: Toby Clemson
git_commit: 81461f75df95cb56e25bda8a6503c4d873691b16
branch: main
repository: accelerator
topic: "Restructure meta/research/codebase/ into subject subcategories (work item 0056)"
tags: [research, codebase, migration, paths, documents-locator, configure, research-codebase, research-issue, design-convergence, work-item-0056]
status: complete
last_updated: 2026-05-11
last_updated_by: Toby Clemson
---

# Research: Restructure `meta/research/codebase/` into Subject Subcategories — Work Item 0056

**Date**: 2026-05-11T23:54:00+01:00
**Researcher**: Toby Clemson
**Git Commit**: 81461f75df95cb56e25bda8a6503c4d873691b16
**Branch**: main
**Repository**: accelerator

## Research Question

For work item 0056 (`meta/work/0056-restructure-meta-research-into-subject-subcategories.md`),
ground the existing Technical Notes against current code. Verify every cited
file:line; inventory every reference site that the migration must rewrite;
characterise the migration framework, the three exemplar migrations whose
patterns will be combined, the path-config layer, the skill consumers, and the
`documents-locator` / `configure` surgery sites. Surface anything missing from
the work item's already-detailed plan that an implementer would hit.

## Summary

The work item's Technical Notes match the codebase very closely. All cited
line numbers verify. The codebase analysis adds **eleven concrete findings the
work item does not call out**, summarised at the head of *Architecture
Insights* below. The most important are:

1. **Fixture location convention mismatch.** AC #5 requires `fixtures/` sibling
   to the migration script, but the existing convention is
   `skills/config/migrate/scripts/test-fixtures/<id>/`. Adopting the AC's
   layout is **new infrastructure**, not a follow-on pattern.
2. **Wider `config-read-path.sh research` callers** beyond the two research
   skills: `visualise`, `extract-adrs`, `init`, `extract-work-items` — the work
   item only flags the two research-skill sites.
3. **Stale text in `skills/config/migrate/SKILL.md`** referencing
   `meta/.migrations-applied` (state file was relocated to
   `.accelerator/state/migrations-applied` by migration 0003); the rewrite
   should either fix this in 0056 or be tracked as a separate cleanup.
4. **`init.sh:33-39` passes an explicit non-empty default to
   `config-read-path.sh`**, which means the centralised `PATH_DEFAULTS` is
   **shadowed** for init scaffolding. `DIR_DEFAULTS` is the sole source of
   truth for what `accelerator:init` creates on a fresh repo.
5. **`config-read-template.sh research`** at `research-codebase/SKILL.md:128` —
   a *template* key with the same bare name as the path key. Work item
   touches only path keys; template-key handling needs an explicit decision.
6. The visualiser prebuilt JS bundle (`skills/visualisation/visualise/frontend/dist/assets/index-DSMqnoSb.js`)
   contains the legacy strings and **regenerates** from `frontend/src/`; do
   not rewrite by hand.
7. **A stray `.DS_Store` under `meta/research/design-inventories/`** that should be
   excluded from the move (or removed pre-migration).
8. The configure-skill paths-table example YAML **already drifted** — it
   omits `design_inventories`, `design_gaps`, `integrations`. 0056 should
   either also fix this drift or note it as deferred.
9. There is **no first-class structured-notification mechanism** in the
   migration framework. The AC #3 "Renamed paths.X → paths.Y" line is plain
   echo; the driver merges stdout/stderr and replays after success.
10. **Ordering between Phase B (config rewrite) and old-value resolution
    matters.** Old `paths.research` etc. values must be captured *before*
    Phase B rewrites them; new values may be derived from old values + the
    fixed subcategory suffix rather than re-reading the post-rewrite config.
11. **`documents-locator.md` has two layers of hardcoding**: a duplicated
    legend at lines 33-41 (claimed to live in the preloaded paths block,
    but actually inline) and flat output group labels at lines 71-97. The
    "subcategory output groups" AC #9 requires structural changes to both.

The rest of this document gives the supporting evidence with full file:line
citations.

## Detailed Findings

### 1. Legacy directory inventory (the move scope)

**`meta/research/codebase/` — flat, single-file shape, 37 `*.md` + `.gitkeep` (38 entries)**.
Locator pass confirms every entry is a flat `*.md`; no subdirectories. Sample
entries span 2026-02-22 → 2026-05-08, mixing codebase research, idea/strategy
notes, and research-codebase outputs (
`meta/research/codebase/2026-02-22-pr-review-agents-design.md` …
`meta/research/codebase/2026-05-08-0052-documents-locator-config-driven-paths.md`).

**`meta/research/design-inventories/` — directory-per-investigation, 2 inventories**:
- `meta/research/design-inventories/.gitkeep`
- `meta/research/design-inventories/.DS_Store` — **stray macOS metadata, FLAG: exclude from move**
- `meta/research/design-inventories/2026-05-06-135214-current-app/` — `inventory.md` +
  11 PNG screenshots under `screenshots/`
- `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/` —
  `inventory.md` + 13 PNG screenshots under `screenshots/`

**`meta/research/design-gaps/` — flat, single-file, 1 `*.md` + `.gitkeep`**:
- `meta/research/design-gaps/.gitkeep`
- `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`

Per-category file-vs-directory shape policy (work item §"Per-category
file-vs-directory shape policy") matches existing reality:
- `codebase/` — flat single-file ✓ (all 37 existing `meta/research/codebase/*.md`)
- `issues/` — flat single-file (no existing instances)
- `design-inventories/` — directory-per-investigation ✓
- `design-gaps/` — flat single-file ✓

### 2. Inbound reference sites the migration must rewrite

Rewrite scope **per legacy target**, with concrete counts:

| Target | Implementation refs | Documentation refs |
|---|---|---|
| `meta/research/codebase/` literal path | scripts: 3 files; skills: ~15 files | README: 4 lines; CHANGELOG: 1 line; meta/: 250+ lines across ~100 files |
| `meta/research/design-inventories/` literal | scripts: 6; skills: ~14 (incl. Rust server + Vite frontend) | README: 1 line; meta/: ~50 lines across ~20 files |
| `meta/research/design-gaps/` literal | scripts: 6; skills: ~12 | README: 5 lines; CHANGELOG: 3 lines; meta/: ~50 lines across ~25 files |
| `paths.research` key | scripts: 2; skills: 1 SKILL.md (`configure`) + 6 SKILL.md sites using `config-read-path.sh research` | meta/: ~12 lines |
| `paths.design_inventories` key | scripts: 3; skills: 2 SKILL.md (`configure`, `init`) + visualiser/design via `config-read-path.sh design_inventories` | CHANGELOG: 1 line; meta/: ~15 lines |
| `paths.design_gaps` key | scripts: 3; skills: 2 SKILL.md (`configure`, `init`) + visualiser/design via `config-read-path.sh design_gaps` | CHANGELOG: 1 line; meta/: ~15 lines |
| `EXCLUDED_KEYS` design tokens | 2 (`config-read-all-paths.sh`, `test-config.sh`) | meta/: 4 files |
| New `paths.research_*` keys | **0 implementation sites** (pre-existence confirmed clean) | only in 0056 planning docs |

**README.md mentions** (every line that hardcodes the legacy paths):
- `README.md:42` — `meta/research/codebase/      meta/plans/     checked-off plan` (ASCII diagram)
- `README.md:47` — `research document in \`meta/research/codebase/\` with findings,`
- `README.md:64` — `` `meta/research/codebase/` ``
- `README.md:84-95` — `meta/` table (rows for `research/`, `design-inventories/`, `design-gaps/`)
- `README.md:409` — `meta/research/codebase/    meta/plans/` (ASCII diagram, ADR section)
- `README.md:413` — `meta/decisions/` (same diagram, anchors `meta/research/codebase/` flow)
- `README.md:579` — `` The resulting gap artifact under `meta/research/design-gaps/` `` **(sixth occurrence — not flagged in work item line 240)**

**CHANGELOG.md mentions** (will likely need synchronisation in the same
commit):
- `CHANGELOG.md:98` — `produces an RCA document in \`meta/research/codebase/\``
- `CHANGELOG.md:155,165,170` — release notes for `analyse-design-gaps` skill
  and `paths.design_gaps` config key

**Implementation surface beyond the work item's enumeration**:
- `skills/visualisation/visualise/SKILL.md:15` — `config-read-path.sh research`
- `skills/decisions/extract-adrs/SKILL.md:25` — `config-read-path.sh research`
- `skills/config/init/SKILL.md:21` — `config-read-path.sh research`
- `skills/work/extract-work-items/SKILL.md:26` — `config-read-path.sh research`
- `scripts/test-config.sh:4361,4379` — assertions on `config-read-path.sh
  research` literals in the two research SKILL.md files
- `scripts/test-config.sh:5789-5792` — assertions on `paths.design_inventories`
  / `paths.design_gaps` defaults
- `skills/visualisation/visualise/frontend/dist/assets/index-DSMqnoSb.js:65` —
  **minified bundle**, regenerated from `frontend/src/`; do not edit

### 3. `accelerator:migrate` framework contract

**Driver**: `skills/config/migrate/scripts/run-migrations.sh` (220 lines)

**Discovery glob** (run-migrations.sh:92):
```sh
find "$MIGRATIONS_DIR" -maxdepth 1 -name '[0-9][0-9][0-9][0-9]-*.sh' -print0 | sort -z
```
Sorted lexicographically on the zero-padded 4-digit prefix. The migration
file's `# DESCRIPTION:` line 2 is harvested by `grep '^# DESCRIPTION:' "$f" |
head -1 | sed 's/^# DESCRIPTION:[[:space:]]*//'` (run-migrations.sh:165-166).

**Dirty-tree guard** (run-migrations.sh:41-70): refuses to proceed when
`jj diff --name-only` (jj-colocated repos) or `git status --porcelain` shows
tracked changes under `meta/`, `.claude/accelerator.md`, `.claude/accelerator.local.md`,
or `.accelerator/`. Untracked files do not make the tree dirty.
`ACCELERATOR_MIGRATE_FORCE=1` bypasses.

**State files** (run-migrations.sh:13-14): `.accelerator/state/migrations-applied`
and `.accelerator/state/migrations-skipped`, newline-delimited migration IDs.
Migration 0003 relocated these from `meta/.migrations-{applied,skipped}`.

**Stale SKILL.md text** (`skills/config/migrate/SKILL.md:11-17, 46-51, 53-57`)
still names `meta/.migrations-*` despite the relocation. **New finding —
implementer should decide whether to fix this in the same 0056 commit or
track as separate cleanup.**

**MUST-NOT policies** (`skills/config/migrate/SKILL.md:41`, verbatim):
> "MUST NOT honour any `DRY_RUN` env var — this framework has no dry-run
> mode"

**Per-migration contract** (`skills/config/migrate/SKILL.md:33-42`):
- Shebang `#!/usr/bin/env bash` on line 1, `# DESCRIPTION: <imperative>` on
  line 2.
- Receive `PROJECT_ROOT`, `CLAUDE_PLUGIN_ROOT`, `ACCELERATOR_MIGRATION_MODE=1`
  (run-migrations.sh:184).
- Idempotent — self-detect already-applied state and exit 0 (SKILL.md:39).
- Use `atomic_write`/`atomic_append_unique`/`atomic_remove_line` from
  `scripts/atomic-common.sh` for any file rewrites (SKILL.md:40).
- Stdout sentinel `MIGRATION_RESULT: no_op_pending` → not recorded as applied,
  retried on future runs (SKILL.md:42, run-migrations.sh:192-202).
- Stdout/stderr merged via `>"$STDOUT_FILE" 2>&1` (run-migrations.sh:184),
  replayed to user stderr after success minus the sentinel line.

**No structured-notification mechanism** — AC #3's `Renamed paths.<old> →
paths.<new> (value preserved: <path>)` is plain `echo`. Closest framework
helper is `scripts/log-common.sh` `log_warn` (lines 16-18) which writes
`Warning: <msg>` to stderr — used by 0003 for pinned-override warnings.

**Fixture location convention** (NEW FINDING):
Existing convention is `skills/config/migrate/scripts/test-fixtures/<id>/`
(see `test-migrate.sh:506,654` and the populated trees under
`skills/config/migrate/scripts/test-fixtures/{0002,0003}/`). **The work item
AC #5 mandates `fixtures/` sibling to the migration script.** The discovery
glob at run-migrations.sh:92 uses `-maxdepth 1` and a `.sh` filter, so a
sibling subdirectory inside `migrations/` would be ignored by discovery —
but it is not a current pattern. Adopting it is **new framework
infrastructure**, not lifted from a prior migration.

**Exit codes**:
- 0 = success (applied or no-op or all-skipped).
- 1 = dirty tree, `--skip`/`--unskip` arg missing, or any migration exited
  non-zero (run-migrations.sh:185-188).

### 4. The three patterns 0056's migration combines

#### Pattern A — Config-key rewrite (from migration 0001)

`skills/config/migrate/migrations/0001-rename-tickets-to-work.sh:111-134`
defines `rewrite_config()`. Four-clause-per-rename sed pipeline:
- Nested-value: `^([[:space:]]+)<old>:[[:space:]]*<old-default>` → indent +
  new key + new default
- Nested-key: `^([[:space:]]+)<old>:` → indent + new key (key-only fallback)
- Flat-value: `^paths\.<old>:[[:space:]]*<old-default>` → flat new key + new
  default
- Flat-key: `^paths\.<old>:` → flat new key (fallback)

Order matters (comment at line 115): value-aware clauses run before key-only
clauses so a default value is rewritten to the new default while overridden
values are preserved verbatim. Output via `"$cfg.tmp" && mv "$cfg.tmp" "$cfg"`
(line 133).

Idempotency: re-runs match no lines (sed silently no-ops); frontmatter
rewriter at lines 57-67 handles partial-prior-run state; directory rename
at lines 91-107 guards via `[ -d "$tickets_dir" ]`.

**Stdout output: 0001 prints nothing on success.** AC #3's notification
line is *new* output not present in 0001.

#### Pattern B — Inbound-link rewriter (from migration 0002)

`skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh:109-313`
defines three rewriters, driven by three separate `find` walks at lines
315-327:

```sh
while IFS= read -r -d '' file; do
  rewrite_frontmatter_in_file "$file"
done < <(find "$PROJECT_ROOT/meta" -name '*.md' -print0 2>/dev/null)
# repeat for markdown_links, prose
```

**Scan corpus is hardcoded to `meta/**/*.md`** (lines 315, 320, 325).
0056's AC #4 requires the corpus to come from the union of paths surfaced
by `accelerator:paths` (i.e. `scripts/config-read-all-paths.sh` output —
the bullet list of `- key: value` pairs at config-read-all-paths.sh:24-33).

**Five reference shapes** (per AC #5):
1. *Frontmatter scalars* — 0002:160-170: `prefix` extracted via bash
   `BASH_REMATCH`, value compared against three quoted variants
   (`"x"`/`'x'`/bare), rewrite always emits double-quoted form.
2. *Frontmatter inline lists* — 0002:147-159: four sed clauses for the
   four lexical positions (quoted/double, quoted/single, bare-first,
   bare-not-first). Sets `in_list=1` if the line opens but doesn't close.
3. *Frontmatter multi-line YAML lists* — 0002:176-189: matches `^[[:space:]]*-
   [[:space:]]`, same value-comparison logic with `- ` prefix.
4. *Markdown links `[label](path)`* — 0002:207-224: one sed expression
   `s|(\[[^]]*\]\([^)]*/)<old>-([^)]+\.md)(#[^)]*)?\)|\1<new>-\2\3)|g`. **For
   0056, the rewrite is path-to-path (not id-to-id), so the regex
   shape changes: no embedded id capture; pure `<old-path>/` → `<new-path>/`
   prefix swap.**
5. *Prose mentions* — 0002:226-313. Split into:
   - Heading-line `#NNNN` refs (lines 235-280) — **irrelevant to 0056**,
     drop.
   - Fenced-code-block path refs (lines 282-308) — keep, parameterise the
     literal `"meta/work/"` (the one hardcoded path literal in 0002's
     rewriter, line 300) over the resolved old/new path pairs.

Tagged code-block branch only fires on languages
`bash|sh|yaml|json|text` (line 290).

Idempotency: each rewriter compares `content` against `original` and only
calls `atomic_write` when something changed (0002:200-202, 221-223, 310-312).
After a successful run the old strings are gone and re-runs no-op.

#### Pattern C — Idempotent filesystem-move helper (from migration 0003)

`skills/config/migrate/migrations/0003-relocate-accelerator-state.sh:32-64`
defines `_move_if_pending(rel_src, rel_dst)` with four explicit cases:

```sh
_move_if_pending() {
  local rel_src="$1" rel_dst="$2"
  local src="$PROJECT_ROOT/$rel_src"
  local dst="$PROJECT_ROOT/$rel_dst"

  # Both absent — nothing to do
  [ ! -e "$src" ] && [ ! -e "$dst" ] && return 0

  # Source absent, dest present — already moved on a prior run
  [ ! -e "$src" ] && [ -e "$dst" ] && return 0

  # Both present — conflict
  if [ -e "$src" ] && [ -e "$dst" ]; then
    printf '%s\n' \
      "accelerator migrate: conflict — both '$rel_src' and '$rel_dst' exist." \
      "Prior moves in this run: ${MOVED_THIS_RUN[*]:-(none)}" >&2
    if [ -d "$src" ] && [ -d "$dst" ]; then
      diff -r "$src" "$dst" >&2 || true
    else
      diff "$src" "$dst" >&2 || true
    fi
    printf '%s\n' \
      "Recover with: jj op restore / git reset, reconcile manually, then re-run." >&2
    exit 1
  fi

  # Source present, dest absent — move it
  local dst_parent
  dst_parent="$(dirname "$dst")"
  mkdir -p "$dst_parent"
  mv "$src" "$dst"
  MOVED_THIS_RUN+=("$rel_src → $rel_dst")
}
```

**VCS rename preservation**: plain `mv`, **not** `jj mv` / `git mv`. Both jj
working-copy snapshot and git's content-similarity rename detection
reconstruct renames from filesystem state — explicit `mv` commands are not
required. The script does not commit; the user's next commit captures the
rename.

`probe_paths_key`/`_awk_probe_paths_key` at 0003:66-102 is a column-0-anchored
awk reader that bypasses `config-read-path.sh` because the migration is
relocating the files that `config-read-path.sh` reads. Useful pattern for
0056 if old key values need to be captured *before* Phase B mutates the
config (see Ordering note in *Architecture Insights* below).

### 5. Path config layer

**`scripts/config-defaults.sh:26-43` — `PATH_KEYS` (16 entries)**:
```bash
PATH_KEYS=(
  "paths.plans"
  "paths.research"             # ← index 1 — rename in-place
  "paths.decisions"
  "paths.prs"
  "paths.validations"
  "paths.review_plans"
  "paths.review_prs"
  "paths.review_work"
  "paths.templates"
  "paths.work"
  "paths.notes"
  "paths.tmp"
  "paths.integrations"
  "paths.design_inventories"   # ← index 13 — rename in-place
  "paths.design_gaps"          # ← index 14 — rename in-place
  "paths.global"
)
```

**`scripts/config-defaults.sh:45-62` — `PATH_DEFAULTS`** index-paired. Entries
for renames:
- index 1: `"meta/research/codebase"` → `"meta/research/codebase"`
- index 13: `"meta/research/design-inventories"` → `"meta/research/design-inventories"`
- index 14: `"meta/research/design-gaps"` → `"meta/research/design-gaps"`

**One new key appended**: `paths.research_issues` → `meta/research/codebase/issues`.

**`skills/config/init/scripts/init.sh:18-31`** carries `DIR_KEYS` (bare,
no `paths.` prefix) and `DIR_DEFAULTS` (13 document-content entries). The
work item flags this as a known duplicate. **NEW FINDING**: init.sh:36 passes
the explicit `DIR_DEFAULTS[i]` to `config-read-path.sh`:

```sh
dir=$(bash "$CONFIG_READ_PATH" "$key" "$default")
```

`config-read-path.sh:27-29` short-circuits the centralised default lookup
when a non-empty `[default]` is passed:

```sh
if [ -n "${2:-}" ]; then
  default="${2}"
```

So `DIR_DEFAULTS` **shadows** the centralised `PATH_DEFAULTS` for the
init-skill scaffolding loop. Bumping only `PATH_DEFAULTS` will leave init
creating the old directory names on fresh repos. Both arrays must be updated
in lockstep — exactly as the work item already states, but the *mechanism*
(explicit default shadowing the centralised one) explains *why*.

**`scripts/config-read-all-paths.sh:14`**:
```bash
EXCLUDED_KEYS=(tmp templates integrations design_inventories design_gaps)
```
After 0056 the design keys are renamed `research_design_*` and must be
**removed** from this exclusion list so `accelerator:paths` surfaces them.
The contract comment at lines 12-13 documents the design:
> "All PATH_KEYS not in this exclusion list are emitted automatically, so new
> document path keys added to config-defaults.sh appear here without editing
> this script."

So adding `paths.research_issues` and `paths.research_codebase` automatically
surfaces them via `config-read-all-paths.sh` once they are in `PATH_KEYS`;
the `EXCLUDED_KEYS` edit is only needed for the design-key renames.

**`skills/config/paths/SKILL.md:25-37` — hand-written legend**:
```markdown
## Path legend

What lives at each path key, with the plugin default if no override is set:

- `plans` — implementation plans for specific work items (default: `meta/plans`)
- `research` — research documents on specific work items (default: `meta/research`)
- `decisions` — architectural decision records for the codebase (default: `meta/decisions`)
- `prs` — PR descriptions for landed changes (default: `meta/prs`)
- `validations` — plan validation reports (default: `meta/validations`)
- `review_plans` — reviews of implementation plans (default: `meta/reviews/plans`)
- `review_prs` — reviews of PR descriptions (default: `meta/reviews/prs`)
- `review_work` — reviews of work items (default: `meta/reviews/work`)
- `work` — work items, often `NNNN-title.md` (default: `meta/work`)
- `notes` — meeting notes, discussions, ad-hoc context (default: `meta/notes`)
- `global` — cross-repo / org-wide information (default: `meta/global`)
```

After 0056 the legend gains four bullets (`research_codebase`,
`research_issues`, `research_design_inventories`, `research_design_gaps`)
and loses the `research` bullet. 11 → 14 entries.

**Sync surface for any path-key change** (5 touchpoints):
1. `scripts/config-defaults.sh:26-43` — `PATH_KEYS`.
2. `scripts/config-defaults.sh:45-62` — `PATH_DEFAULTS`.
3. `scripts/config-read-all-paths.sh:14` — `EXCLUDED_KEYS` (only when key
   moves in/out of document-discovery).
4. `skills/config/paths/SKILL.md:25-37` — hand-written legend.
5. `skills/config/init/scripts/init.sh:18-31` — `DIR_KEYS`/`DIR_DEFAULTS`
   (only for `meta/`-shaped document keys).

### 6. Research skill consumers

**`skills/research/research-codebase/SKILL.md`**:
- Line 23: `**Research directory**: !` (backtick) `${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh research` (backtick) — change bare key `research` → `research_codebase`.
- **Line 128**: `${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh research` — **template key with same name; NEW FINDING — work item does not flag this**. Decision needed: does the template key also rename, or stay as `research`? The work item §"Configuration changes" only discusses `paths.*` keys.
- Frontmatter (lines 1-9) has **no `skills: [accelerator:paths]`** entry. `documents-locator` declares this (per 0052) but `research-codebase` and `research-issue` do not. If 0056 wants the skills to consume `accelerator:paths` output, the frontmatter must be extended too. Not flagged in AC.

**`skills/research/research-issue/SKILL.md`**:
- Line 22: `**Research directory**: !` (backtick) `${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh research` (backtick) — change to `research_issues`.
- Line 97: `${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh rca` — unrelated key, no change.
- Line 93: invokes `skills/research/research-codebase/scripts/research-metadata.sh` — shared helper, no path literal (confirmed below).

**`skills/research/research-codebase/scripts/research-metadata.sh` (26 lines)**:
- **No `meta/research` literal, no `paths.research` lookup, no bare `research` key.** The work item's instruction to "inspect for any hardcoded `meta/research` literal that bypasses the bang call" returns clean. No edits required in this script.

**Wider `config-read-path.sh research` callers (NEW FINDING)** — outside
the two research skills:
- `skills/visualisation/visualise/SKILL.md:15`
- `skills/decisions/extract-adrs/SKILL.md:25`
- `skills/config/init/SKILL.md:21`
- `skills/work/extract-work-items/SKILL.md:26`
- `scripts/test-config.sh:4361,4379` (test assertions on the two research
  SKILL.md sites)

Each of these reads "where research lives" for purposes other than writing.
The work item AC §"Skill updates" only mentions `research-codebase` and
`research-issue`. **Decision needed**: do these callers point at
`research_codebase` (presuming the most common reader is finding existing
research docs), at one of the four new keys depending on intent, or somehow
discover the parent `meta/research/codebase/` directory?

### 7. `documents-locator` surgery

**`agents/documents-locator.md`** frontmatter (lines 1-12):
```yaml
---
name: documents-locator
description: ...
tools: Grep, Glob, LS
skills:
  - accelerator:paths
---
```
Block-sequence form, not inline array (work item says "lines 10-11"; the
content matches but split across two lines).

**Inline legend** (`documents-locator.md:33-41`):
```
- `work` — work items
- `research` — research documents
- `plans` — implementation plans
- `decisions` — architectural decisions
- `validations` — plan validation reports
- `review_plans`, `review_prs`, `review_work` — review artifacts
- `prs` — PR descriptions
- `notes` — discussions, meeting notes
- `global` — cross-repo information
```
**NEW FINDING**: contrary to the work item's framing, this list does **not**
hardcode `meta/`-prefixed paths. It hardcodes the *single-document-type-per-key*
mapping (research → "research documents") with no acknowledgement of
subcategories. AC #9's "Research (codebase) / Research (issues) / Research
(design-inventories) / Research (design-gaps)" requires changing the *legend
shape*, not just substituting paths.

**Inline output template** (`documents-locator.md:71-97`, fenced block):
```
## Documents about [Topic]

### Work Items
- `{work}/0001-implement-rate-limiting.md` - ...

### Research Documents          ← line 77, the flat "Research" group label
- `{research}/2024-01-15_rate_limiting_approaches.md` - ...

### Implementation Plans
- `{plans}/api-rate-limiting.md` - ...

### Related Discussions
- `{notes}/meeting-2024-01-10.md` - ...
- `{decisions}/rate-limit-values.md` - ...

### Reviews
- `{review_plans}/2026-03-22-plan-review.md` - ...

### Validations
- `{validations}/2026-03-22-validation.md` - ...

### PR Descriptions
- `{prs}/pr-456-rate-limiting.md` - ...

Total: 8 relevant documents found
```
Trailing prose at lines 99-100: `` Where `{research}`, `{plans}`, etc. are
the resolved paths from the Configured Paths block. ``

For AC #9 the template must split the single `### Research Documents` group
into four (or render dynamically based on the `accelerator:paths` block keys,
matching them by `research_*` prefix). Decision needed: explicit four-bullet
template vs prefix-grouped dynamic rendering.

**"Historic intent" purpose hints** (`documents-locator.md:110-117`):
```
- Historic intent and context: `research`, `plans`, `decisions`
- Recent activity: `prs`, `review_prs`, `review_plans`, `review_work`
- Quality / risk signals: `validations`, `review_*`
- Active in-flight work: `work`, `plans`
- Team-level knowledge: `notes`, `decisions`
- Cross-repo or org-wide concerns: `global`
```
Line 112 lumps `research` as one key. Post-0056 the bullet must reference
the four `research_*` keys (or use a `research_*` wildcard, which the legend
above does not establish as a convention).

### 8. `configure` skill paths reference

**`skills/config/configure/SKILL.md:386-402` — paths table (verbatim)**:
```
| Key                  | Default                           | Description                                                                      |
|----------------------|-----------------------------------|----------------------------------------------------------------------------------|
| `plans`              | `meta/plans`                      | Implementation plans                                                             |
| `research`           | `meta/research`                   | Research documents                                                               |
| `decisions`          | `meta/decisions`                  | Architecture decision records                                                    |
| `prs`                | `meta/prs`                        | PR descriptions                                                                  |
| `validations`        | `meta/validations`                | Plan validation reports                                                          |
| `review_plans`       | `meta/reviews/plans`              | Plan review artifacts                                                            |
| `review_prs`         | `meta/reviews/prs`                | PR review working directories                                                    |
| `review_work`        | `meta/reviews/work`               | Work item review artifacts                                                       |
| `templates`          | `.accelerator/templates`          | User-provided templates (e.g., PR description)                                   |
| `work`               | `meta/work`                       | Work item files referenced by create-plan                                        |
| `notes`              | `meta/notes`                      | Notes directory                                                                  |
| `tmp`                | `.accelerator/tmp`                | Ephemeral working data (gitignored)                                              |
| `design_inventories` | `meta/design-inventories`         | Design-inventory artifacts (one directory per snapshot, with screenshots/)       |
| `design_gaps`        | `meta/design-gaps`                | Design-gap analysis artifacts                                                    |
| `integrations`       | `.accelerator/state/integrations` | Per-integration cached state (Jira fields/projects, future Linear/Trello caches) |
```

15 rows. The three flagged rows are at lines 389 (`research`), 400
(`design_inventories`), 401 (`design_gaps`). After 0056: replace those three
rows with four — `research_codebase`, `research_issues`,
`research_design_inventories`, `research_design_gaps`.

**`skills/config/configure/SKILL.md:406-422` — example YAML (verbatim)**:
```yaml
---
paths:
  plans: docs/plans
  research: docs/research
  decisions: docs/adrs
  prs: docs/prs
  validations: docs/validations
  review_plans: docs/reviews/plans
  review_prs: docs/reviews/prs
  review_work: docs/reviews/work
  templates: docs/templates
  work: docs/work
  notes: docs/notes
  tmp: docs/tmp
---
```

12 rows. **NEW FINDING — already drifted**: the example YAML omits
`design_inventories`, `design_gaps`, and `integrations`. 0056 either also
fixes this drift (adding the four new research subcategory keys *and*
restoring `integrations`) or leaves the drift in place. AC #14 says
"`accelerator:configure help` is invoked … then it lists all four new keys
with their default values and one-line descriptions, and contains no rows
for the legacy keys" — this targets the table, not the YAML example. Worth
deciding consciously.

### 9. README narrative + diagrams

**`README.md:84-95` — `meta/` table** lists `research/`, `design-inventories/`,
`design-gaps/` as flat top-level rows. After 0056 the three rows are replaced
with the four `research/<sub>/` rows (or a single `research/` row with a
nested-bullet description, depending on preferred shape — AC #11 says "all
four subcategories are listed with their purposes").

**`README.md:42` — ASCII development-loop diagram**:
```
research-codebase  →  create-plan  →  implement-plan
       ↓                   ↓                 ↓
  meta/research/codebase/      meta/plans/     checked-off plan
```
Line 42 (`meta/research/codebase/      meta/plans/     checked-off plan`) needs
`meta/research/codebase/` substituted.

**`README.md:407-415` — ASCII ADR-flow diagram**:
```
research-codebase → create-plan → implement-plan
       ↓                ↓
  meta/research/codebase/    meta/plans/
       ↓                ↓
  extract-adrs ←────────┘
       ↓
  meta/decisions/
       ↓
  review-adr → accepted ADRs inform future research & planning
```
Line 409 (`meta/research/codebase/`) → `meta/research/codebase/`.

**`README.md:579`** (NEW FINDING — not in work item's enumeration):
```
The resulting gap artifact under `meta/research/design-gaps/` feeds straight into
```
This narrative line is in the design-convergence README section; the rewrite
points it at `meta/research/design-gaps/`.

**`README.md:47`** — narrative description of `research-codebase` output
location: `research document in `meta/research/codebase/` with findings,…`

**`README.md:64`** — narrative description of `research-issue` output
location: `produces an RCA document in `meta/research/codebase/`.`

After 0056 these two lines must distinguish destinations:
- `research-codebase` → `meta/research/codebase/`
- `research-issue` → `meta/research/issues/`

## Code References

- `scripts/config-defaults.sh:26-43` — `PATH_KEYS` array (16 entries)
- `scripts/config-defaults.sh:45-62` — `PATH_DEFAULTS` array (index-paired)
- `scripts/config-read-all-paths.sh:14` — `EXCLUDED_KEYS=(tmp templates integrations design_inventories design_gaps)`
- `scripts/config-read-all-paths.sh:24-33` — document-discovery loop emitting `- key: value` bullets
- `scripts/config-read-path.sh:27-29` — explicit-default short-circuit (shadows centralised defaults)
- `scripts/config-read-path.sh:31-40` — centralised default fallback via `PATH_KEYS` index lookup
- `skills/config/init/scripts/init.sh:18-31` — `DIR_KEYS`/`DIR_DEFAULTS` (duplicate of document-content subset)
- `skills/config/init/scripts/init.sh:33-39` — init scaffold loop passing explicit defaults
- `skills/config/paths/SKILL.md:21` — bang preprocessor for `config-read-all-paths.sh`
- `skills/config/paths/SKILL.md:25-37` — hand-written path legend (11 entries)
- `skills/config/migrate/SKILL.md:33-42` — per-migration contract
- `skills/config/migrate/SKILL.md:41` — "MUST NOT honour any `DRY_RUN`"
- `skills/config/migrate/SKILL.md:11-17,46-57` — **stale text** referencing `meta/.migrations-applied`
- `skills/config/migrate/scripts/run-migrations.sh:13-14` — actual state-file paths under `.accelerator/state/`
- `skills/config/migrate/scripts/run-migrations.sh:41-70` — dirty-tree guard
- `skills/config/migrate/scripts/run-migrations.sh:92-93` — discovery glob
- `skills/config/migrate/scripts/run-migrations.sh:184` — env var exports for migrations
- `skills/config/migrate/scripts/run-migrations.sh:192-202` — `no_op_pending` sentinel handling
- `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh:111-134` — `rewrite_config()` (4-clause sed)
- `skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh:113-203` — `rewrite_frontmatter_in_file`
- `skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh:207-224` — `rewrite_markdown_links_in_file`
- `skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh:226-313` — `rewrite_prose_in_file`
- `skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh:300` — only hardcoded `meta/work/` literal in 0002's rewriter
- `skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh:315-327` — three `find $PROJECT_ROOT/meta -name '*.md'` walks (hardcoded scan corpus)
- `skills/config/migrate/migrations/0003-relocate-accelerator-state.sh:32-64` — `_move_if_pending` helper
- `skills/config/migrate/migrations/0003-relocate-accelerator-state.sh:66-102` — `probe_paths_key` for reading pre-rewrite config
- `agents/documents-locator.md:9-11` — `skills: [accelerator:paths]` (block sequence)
- `agents/documents-locator.md:33-41` — inline legend (duplicate of paths SKILL.md legend)
- `agents/documents-locator.md:71-97` — output template (flat "Research Documents" group at line 77)
- `agents/documents-locator.md:110-117` — "Historic intent" purpose hints
- `skills/config/configure/SKILL.md:386-402` — paths table
- `skills/config/configure/SKILL.md:406-422` — example YAML (already drifted)
- `skills/research/research-codebase/SKILL.md:23` — `config-read-path.sh research`
- `skills/research/research-codebase/SKILL.md:128` — `config-read-template.sh research` (template key, decision needed)
- `skills/research/research-issue/SKILL.md:22` — `config-read-path.sh research`
- `skills/research/research-codebase/scripts/research-metadata.sh:1-26` — no path literals; no edits required
- `skills/visualisation/visualise/SKILL.md:15` — wider `config-read-path.sh research` caller
- `skills/decisions/extract-adrs/SKILL.md:25` — wider caller
- `skills/config/init/SKILL.md:21` — wider caller
- `skills/work/extract-work-items/SKILL.md:26` — wider caller
- `README.md:42,47,64,84-95,407-415,579` — narrative + tables + diagrams
- `CHANGELOG.md:98,155,165,170` — release-note references
- `scripts/test-config.sh:4361,4379,5789-5792` — test assertions

## Architecture Insights

### Eleven findings beyond the work item

1. **Fixture location convention mismatch (AC #5).** Existing convention is
   `skills/config/migrate/scripts/test-fixtures/<id>/`. Work item mandates
   `fixtures/` sibling to the migration script. Adopting the AC's layout is
   net-new framework infrastructure — not a pattern lifted from 0001/0002/0003.

2. **Wider `config-read-path.sh research` callers** — four SKILL.md sites
   beyond the two research skills (`visualise`, `extract-adrs`, `init`,
   `extract-work-items`) plus two test-config assertions. Work item's
   "Skill updates" section enumerates only the two research skills.

3. **Stale `migrate/SKILL.md` text** referencing `meta/.migrations-*`.
   Cleanup target (in-scope or deferred).

4. **`init.sh:33-39` shadows centralised defaults** by passing explicit
   `DIR_DEFAULTS[i]` to `config-read-path.sh`. `DIR_DEFAULTS` is the sole
   source of truth for what `accelerator:init` creates. Both arrays must be
   updated in lockstep — the work item notes the duplication but not the
   shadowing mechanism that explains *why* the duplication is load-bearing.

5. **`config-read-template.sh research`** at `research-codebase/SKILL.md:128`
   is a template key bearing the same bare name. Decision: rename in
   lockstep, or leave the template key alone? Work item §"Configuration
   changes" speaks only of path keys.

6. **Visualiser prebuilt JS bundle** at
   `skills/visualisation/visualise/frontend/dist/assets/index-DSMqnoSb.js`
   contains the legacy `design-gaps`/`design-inventories` strings. Bundle
   is regenerated from `frontend/src/` via vite build; do not hand-edit. The
   `frontend/src/api/types.ts:7,17,157,164` and related test files are the
   actual edit targets.

7. **Stray `.DS_Store`** under `meta/research/design-inventories/`. Exclude from move
   (or remove pre-migration).

8. **Configure example YAML already drifted** — misses `design_inventories`,
   `design_gaps`, `integrations`. Decision: also fix the drift in this
   commit, or leave it for a separate cleanup?

9. **No structured-notification mechanism in the framework.** AC #3's
   `Renamed paths.X → paths.Y (value preserved: <path>)` line is plain
   `echo`. Driver merges stdout/stderr and replays after success. No new
   framework infrastructure required.

10. **Ordering between Phase B and old-value capture.** Old `paths.research`
    etc. values must be captured *before* the config sed rewrite mutates
    them (use 0003's `probe_paths_key` pattern, or read first via
    `config-read-value.sh paths.research`). New values are derivable from
    old values + the fixed subcategory suffix (`<old_research>/<file>` →
    `<old_research>/codebase/<file>`), avoiding a second `config-read-*`
    pass against the post-rewrite config.

11. **`documents-locator.md` has two layers of hardcoding**: the inline
    legend (lines 33-41) duplicates the paths SKILL.md legend, and the
    output template (lines 71-97) has a flat "Research Documents" group
    label. AC #9 requires both the legend and the output template to surface
    research subcategories. The legend additionally duplicates content the
    file *claims* lives in the preloaded paths block (line 31), so
    delegating to the preloaded block is also an option.

### Patterns to lift, verbatim where short

The combined 0056 migration script structurally looks like:

```
0056-restructure-meta-research-into-subject-subcategories.sh

  Header + sourcing                           ← all three migrations
  ├─ config-common.sh
  ├─ atomic-common.sh
  └─ log-common.sh

  STEP 0: capture pre-rewrite old paths       ← novel
  ├─ old_research = config-read-value.sh paths.research <legacy-default>
  ├─ old_inv      = config-read-value.sh paths.design_inventories ...
  └─ old_gaps     = config-read-value.sh paths.design_gaps ...
  (or use 0003's probe_paths_key pattern)

  STEP 1: filesystem moves                     ← LIFTED from 0003
  ├─ paste _move_if_pending() verbatim (0003:32-64)
  ├─ MOVED_THIS_RUN=()
  ├─ for f in $old_research/*.md: _move_if_pending $f $old_research/codebase/$(basename $f)
  ├─ for d in $old_inv/*: _move_if_pending $d $new_inv_parent/$(basename $d)
  ├─ for f in $old_gaps/*.md: _move_if_pending $f $new_gaps_parent/$(basename $f)
  └─ rmdir old empty parents (idempotent)

  STEP 2: config-key rewrites                  ← LIFTED from 0001 (4-clause sed)
  ├─ rewrite_config() applied to .accelerator/config.md and config.local.md
  ├─ three rename rules (research → research_codebase, etc.)
  └─ echo "Renamed paths.<old> → paths.<new> (value preserved: <path>)"   ← AC #3, novel stdout

  STEP 3: inbound-link rewriting               ← LIFTED from 0002, generalised
  ├─ scan corpus = union of paths from config-read-all-paths.sh             ← AC #4, novel
  ├─ pairs = [($old_research, $old_research/codebase),
  │            ($old_inv,      $new_inv_parent),
  │            ($old_gaps,     $new_gaps_parent)]
  ├─ rewrite_frontmatter_in_file — path-prefix matching, not id-equality
  ├─ rewrite_markdown_links_in_file — path-to-path regex (no embedded id)
  └─ rewrite_prose_in_file — drop heading branch; keep fenced-block branch
                              parameterised over pairs

  Done — echo "MIGRATION_RESULT: applied" (informational)
```

The novel code surface is:
- **Step 0** (~10 lines reading old values, mirroring 0003's `probe_paths_key`).
- **AC #3 stdout line** in Step 2 (~3 lines).
- **Scan-corpus derivation** in Step 3 (~10-15 lines parsing
  `config-read-all-paths.sh` output into a `SCAN_DIRS[]` array).
- **Frontmatter-rewriter generalisation** — replace id-equality with
  path-prefix matching across any field, not just the
  `parent|related|blocks|…` set 0002 keys on.
- **Markdown-link regex** — strip the `${old_id}-` embedded capture; pure
  prefix swap.

### `accelerator:paths` block as scan-corpus source

`scripts/config-read-all-paths.sh` outputs a Markdown bullet list:
```
## Configured Paths

- plans: meta/plans
- research: meta/research/codebase
- decisions: meta/decisions
- prs: meta/prs
- validations: meta/validations
- review_plans: meta/reviews/plans
- review_prs: meta/reviews/prs
- review_work: meta/reviews/work
- work: meta/work
- notes: meta/notes
- global: meta/global
```

For AC #4 the rewriter parses this into a `SCAN_DIRS[]` array and `find`s
`*.md` files under each — implementing the literal contract of the AC ("union
of markdown files inside every path surfaced by `accelerator:paths`"). After
the design keys are removed from `EXCLUDED_KEYS`, the block also surfaces
`research_design_inventories` and `research_design_gaps`, so the scan corpus
naturally includes the new locations post-migration.

### Why VCS rename history survives plain `mv`

Both jj and git reconstruct renames from snapshot/commit state, not from
explicit move commands. jj's working-copy snapshot captures the disappearance
of `meta/research/codebase/foo.md` and the appearance of `meta/research/codebase/foo.md`
in one snapshot; git's `--follow` and content-similarity rename detection
operate on commit diffs. Plain `mv` is sufficient — `_move_if_pending`
(0003:62) does not use `jj mv` or `git mv`. AC #12's `jj log --follow`
acceptance criterion is therefore automatically satisfied by the move
mechanism as long as the commit captures source-disappear + dest-appear
together.

## Historical Context

The work item's predecessor reasoning lives across six artifacts:

- **`meta/notes/2026-05-02-research-directory-subcategory-restructure.md`** —
  the source note. Frames `meta/research/codebase/` as a flat directory mixing
  heterogeneous content, proposes the four subcategories, identifies the
  design-convergence workflow as the forcing function, lists deferral
  reasons (migration ambiguity, skill path-resolution presumption,
  documents-locator inline maps, backward-compatibility, all-vs-some
  nesting decision). Proposes the path forward 0056 implements: ADR,
  inbound-reference survey, `research-codebase`/`research-issue` skill
  updates, `documents-locator` update, README and configure-help paths
  table updates.
- **`meta/research/codebase/2026-05-02-design-convergence-workflow.md`** — the
  research artifact whose §9.8 ("Nested layout under `meta/research/codebase/`")
  explicitly defers the nested layout. Line 687 (§Future Work item 6):
  > "Future research/ subcategory restructure. … Worth its own ADR. The
  > introduction of `design-inventories` could be the forcing function —
  > but as a separate, clearly-scoped change."
- **`meta/work/0030-centralise-path-defaults.md`** — done. Extracted
  `PATH_KEYS`/`PATH_DEFAULTS` into `scripts/config-defaults.sh` so a path
  rename is a one-line edit. Explicitly flags the `init.sh:18-31` `DIR_KEYS`
  duplication as deferred to a follow-on item.
- **`meta/work/0052-make-documents-locator-paths-config-driven.md`** — done.
  Added `skills/config/paths/SKILL.md`, the `accelerator:paths` skill, and
  the `skills:` frontmatter convention on agent files; preloaded the paths
  block into `documents-locator`. Establishes the `accelerator:paths` →
  agent injection mechanism 0056 builds on.
- **`meta/work/0027-ephemeral-file-separation-via-paths-tmp.md`** — done.
  Precedent for adding a new top-level configurable path key (`paths.tmp`)
  alongside the existing ones. Showed the coordinated edit pattern across
  `config-defaults.sh`, `init.sh`, SKILL.md migrations, and an inner
  `.gitignore` for ephemeral semantics.
- **`meta/decisions/ADR-0023-meta-directory-migration-framework.md`** —
  the ADR documenting the lightweight shell migration framework. Establishes
  the pattern 0056's migration plugs into: idempotent shell migrations,
  preserve user-pinned values while rewriting plugin-level expectations.

Related but tangential:
- **`meta/notes/2026-04-26-agents-hardcode-default-directory-locations.md`** —
  the originating tech-debt note for 0052. Argues for harness-driven path
  injection so agent definitions remain config-aware. Now closed for
  top-level keys; 0056 extends to subcategorisation.
- **`meta/notes/2026-05-09-design-paths-missing-from-documents-locator.md`** —
  post-0052 note arguing that `design_inventories`/`design_gaps` should be
  removed from `EXCLUDED_KEYS` so the documents-locator surfaces them. 0056
  resolves this naturally by renaming them into the `research_design_*` namespace
  and removing them from `EXCLUDED_KEYS`.

## Related Research

- `meta/research/codebase/2026-05-02-design-convergence-workflow.md` — design-convergence
  workflow that introduced `meta/research/design-inventories/` and `meta/research/design-gaps/`
  as top-level directories (deferred subcategory restructure).
- `meta/research/codebase/2026-05-08-0030-centralise-path-defaults-implementation.md` —
  implementation context for the `PATH_KEYS`/`PATH_DEFAULTS` centralisation
  that 0056 extends.
- `meta/research/codebase/2026-05-08-0052-documents-locator-config-driven-paths.md` —
  research underpinning the `accelerator:paths` preload mechanism.

## Open Questions

1. **Template-key parity** — should `config-read-template.sh research` at
   `research-codebase/SKILL.md:128` also rename? The work item is silent on
   template keys.
2. **Wider `config-read-path.sh research` callers** — `visualise`,
   `extract-adrs`, `init`, `extract-work-items` SKILL.md sites. Which new
   key does each point at? Most likely `research_codebase` (since they all
   read "where existing research lives" rather than "where new research
   should go"), but worth confirming intent per call site.
3. **Fixture-location convention** — AC #5 mandates `fixtures/` sibling to
   the migration script. Existing convention is
   `skills/config/migrate/scripts/test-fixtures/<id>/`. Does 0056 also
   migrate the test harness to the new convention, or only introduce it
   for 0056's own fixtures (creating a split convention)?
4. **`migrate/SKILL.md` stale text cleanup** — fix in 0056's commit or
   defer to a separate doc-sync work item?
5. **Configure example YAML drift** — fix as part of 0056 or defer?
6. **`documents-locator` subcategory rendering** — explicit four-bullet
   template vs. prefix-grouped dynamic rendering (e.g. all `research_*` keys
   collapse into one "Research" section with sub-groups)?
7. **`research-codebase` / `research-issue` SKILL.md frontmatter** — do
   these skills also gain `skills: [accelerator:paths]` so they can consume
   the resolved paths block at invocation time? Not flagged in AC.
8. **CHANGELOG.md** — references at lines 98, 155, 165, 170 are *historical*
   release notes describing past releases. Should they be rewritten to
   reflect post-0056 layout, or left as accurate records of what shipped
   when (with 0056 adding its own new entry)?
