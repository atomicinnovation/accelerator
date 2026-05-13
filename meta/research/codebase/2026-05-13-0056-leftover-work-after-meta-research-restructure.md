---
date: "2026-05-13T09:29:26+01:00"
researcher: Toby Clemson
git_commit: 99c29ca35498adbe72fafec97a68216d6ebf6fde
branch: main
repository: accelerator
topic: "Leftover work / issues after meta directory restructure (0056)"
tags: [research, codebase, migration, 0056, meta-restructure]
status: complete
last_updated: 2026-05-13
last_updated_by: Toby Clemson
---

# Research: Leftover work / issues after meta directory restructure (0056)

**Date**: 2026-05-13 09:29:26 BST
**Researcher**: Toby Clemson
**Git Commit**: 99c29ca35498adbe72fafec97a68216d6ebf6fde
**Branch**: main
**Repository**: accelerator

## Research Question

Are there any leftover work items or unresolved issues after the recent
restructuring of the `meta/` directory as described in
`meta/plans/2026-05-12-0056-restructure-meta-research-into-subject-subcategories.md`?

## Summary

All eight phases of the plan landed cleanly across commits `99b785f27` →
`99c29ca35`. Both test suites pass (`test-config.sh`: 535 assertions;
`test-migrate.sh`: 277 assertions). The plugin's own `meta/` was migrated
in `cca787aee`.

**One real leftover issue exists**, plus two minor known deviations from
the literal plan spec:

1. **STRAY FILE — needs fixing.** A glyph-component research file slipped
   past the migration and still sits at the bare `meta/research/` level
   (rather than under `meta/research/codebase/`), along with 9 broken
   inbound references to other research files that the migration *did*
   move.
2. **AWK-not-Perl deviation** in the migration's Step 3 — functionally
   equivalent but does not match the plan's literal regex strategy.
   Not a defect.
3. **Documents-locator output structure** uses a nested `### Research` /
   `#### <subcategory>` heading hierarchy (per commit `99c29ca35`)
   instead of four sibling `### Research (codebase)` etc. groups.
   Intentional post-plan design change. Not a defect.

## Detailed Findings

### 1. Stray glyph-component research file (real leftover work)

**File**: `meta/research/2026-05-12-0037-glyph-component.md`

This file should have been moved by migration 0004 to
`meta/research/codebase/2026-05-12-0037-glyph-component.md` along with
the other ~37 files. It is the only file still living directly under
`meta/research/`. Verified via:

```
$ git ls-tree --name-only cca787aee:meta/research
.gitkeep
2026-05-12-0037-glyph-component.md      ← stayed put
codebase
design-gaps
design-inventories
issues
```

Migration commit `cca787aee` shows **R100/R099/R098** rename entries
for every other dated `meta/research/<file>.md` → `meta/research/codebase/<file>.md`
but **no** rename for the glyph component file.

**Likely cause**: jj branching/rebasing. The glyph file was committed
at `2026-05-12 10:46:59` (commit `582731c44`); the migration was
applied at `2026-05-12 20:18:57` (commit `cca787aee`). Most other 0056
commits author-date between these two timestamps. The migration was
likely run on a jj working-copy state that did not yet include the
glyph commit, and the file was reintroduced at the bare path on a
subsequent rebase/merge.

The migration script's `_plan_research_moves` helper
([0004-restructure-meta-research-into-subject-subcategories.sh:213-225](skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh))
is itself correct — re-running the same glob today picks up the
glyph file:

```
$ cd meta/research && shopt -s nullglob dotglob; for f in *; do [ -f "$f" ] && echo "$f"; done
2026-05-12-0037-glyph-component.md
```

But the migration is gated by `.accelerator/state/migrations-applied`
which lists `0004-restructure-meta-research-into-subject-subcategories`,
so it will not re-run.

#### Stale inbound references in the same file

`meta/research/2026-05-12-0037-glyph-component.md` contains 7 stale
references to peer research files that *did* move into `codebase/`:

- Line 220: `meta/research/2026-05-06-0033-design-token-system.md`
- Line 235: `meta/research/2026-05-08-0034-theme-and-font-mode-toggles.md`, `meta/research/2026-05-07-0035-topbar-component.md`
- Line 241: `meta/research/2026-04-17-meta-visualiser-implementation-context.md`
- Lines 247-251: five bullet-list references with the same legacy prefix

All targets now live at `meta/research/codebase/<file>.md`. The links
are broken at the markdown-renderer level.

#### Stale inbound references in sibling plan

`meta/plans/2026-05-12-0037-glyph-component.md` contains 2 stale
references at lines 895 and 900:

- `Research: meta/research/2026-05-12-0037-glyph-component.md` (line 895) — points to the file *itself* at its current bare-research location; if/when the file moves, this will need updating to `meta/research/codebase/…`
- `Sibling component plans: meta/research/2026-05-07-0035-topbar-component.md, meta/research/2026-05-08-0034-theme-and-font-mode-toggles.md` (line 900) — both targets live under `codebase/` now

#### Recommended fix

Three options, in order of preference:

1. **Manual `jj mv` + sed** — fastest and preserves rename history:
   ```bash
   jj mv meta/research/2026-05-12-0037-glyph-component.md \
         meta/research/codebase/2026-05-12-0037-glyph-component.md
   # Then sed-rewrite the 9 stale references across both files.
   ```
2. **Re-run migration 0004** — remove the line from
   `.accelerator/state/migrations-applied` and re-run `accelerator:migrate`.
   The migration is idempotent (negative-lookbehind / boundary-anchor
   prevents double-substitution) and will sweep both the file move and
   the inbound references. Trade-off: any *other* drift in the corpus
   would also be touched.
3. Leave as-is — the broken links cost the visualiser correctness and
   future readers will hit dead links when navigating from the glyph
   research note.

### 2. Migration Step 3 uses awk instead of Perl (deviation, not defect)

The plan (Phase 5) specified Perl with `\Q…\E` literal mode and
`(?<!\Q$new\E/)` negative lookbehind to prevent double-substitution.
The shipped implementation
([0004-restructure-meta-research-into-subject-subcategories.sh:501-554](skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh))
uses awk with an explicit sibling-subcategory exclusion list
(`codebase|issues|design-inventories|design-gaps`) inside
`rewrite_line` for the same effect.

Functional consequences:

- Idempotency: equivalent. Tests in `test-migrate.sh` exercise the
  double-substitution recovery path and pass.
- Boundary-anchor: implemented via `is_boundary()` (lines 514-518)
  covering `/ " ' space tab ) ] #` and end-of-line. Equivalent.
- Step 0d Perl version check is absent (no longer needed) — also
  consistent with the awk implementation.

This is a tactical implementation choice; the user (or a future
follow-up work item) may want to align the implementation back to the
plan if the awk regex proves to drift from the boundary-anchor spec
under future changes. No action needed today.

### 3. Documents-locator output structure (deviation, intentional)

`agents/documents-locator.md` lines 80-92 use a nested heading
hierarchy:

```
### Research
#### Codebase
- `{research_codebase}/<file>.md` - …
#### Issues
- `{research_issues}/<file>.md` - …
#### Design inventories
…
#### Design gaps
…
```

The plan (Phase 6) called for four sibling `### Research (codebase)`,
`### Research (issues)`, etc. groups. Commit `99c29ca35` ("Nest research
subcategories under a single Research heading") deliberately replaced
the flat structure with the nested one. The omit-empty instruction
applies at both the parent `Research` level (omit the whole block
when nothing matches) and the subcategory `#### …` level.

Treat as a post-plan UX refinement. No action needed.

### 4. Test suites and grep residues (all green)

- `bash scripts/test-config.sh` — exit 0, 535 assertions pass.
- `bash skills/config/migrate/scripts/test-migrate.sh` — exit 0,
  277 assertions pass (including ~34 0004-specific cases across
  filesystem moves, config rewrites, and inbound rewriting).
- `bash skills/config/migrate/scripts/run-migrations.sh` — exit 0,
  no pending 0004; only 0002 pre-existing no-op.
- README/CHANGELOG/templates/visualiser code: zero legacy-path
  residue across the broadened residue grep.
- `meta/.migrations-*` references: zero matches in
  `skills/config/migrate/SKILL.md`.

### 5. Phase-by-phase completion

All eight phases have corresponding commits and all success criteria
verified PASS aside from the leftover above:

| Phase | Commit       | Description |
|-------|--------------|-------------|
| 1     | `d9bb50f84`  | Introduce research subject subcategory path keys |
| 2     | `b4d478da1`  | Update skill consumers to use renamed research path keys |
| 3     | `c7fb1fe42`  | Add migration 0004 — filesystem moves |
| 4     | `5e3556b88`  | Add migration 0004 — config-key rewrites |
| 5     | `6b4013ba5`  | Add migration 0004 — inbound-link rewriting |
| 6     | `9252ea12f`  | Update documents-locator and configure UX |
| 7     | `cca787aee`  | Apply migration 0004 to plugin meta/ |
| 8     | `7afa0667d`, `99c29ca35` | Narrative surfaces + documents-locator nesting refinement |

## Code References

- `skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh:213-225` — `_plan_research_moves` (correct logic; should pick up the stray glyph file)
- `skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh:501-554` — Step 3 awk implementation (deviation from Perl plan)
- `agents/documents-locator.md:80-92` — nested Research heading structure (deviation from flat plan)
- `meta/research/2026-05-12-0037-glyph-component.md` — the stray file (rows 220, 235, 241, 247-251 contain stale references)
- `meta/plans/2026-05-12-0037-glyph-component.md:895,900` — sibling stale references
- `.accelerator/state/migrations-applied` — line 3 records 0004 as applied

## Architecture Insights

- The migration script's gated re-execution semantics (state file)
  protect against double-application but also lock out fixing
  post-migration drift. To re-sweep, the user must manually delete the
  state-file entry. This is documented in CHANGELOG and
  `migrate/SKILL.md`.
- jj's working-copy model (no staging area; the working copy is a
  commit) makes it possible for files created in parallel changes
  to escape a one-shot migration that runs on a single jj revision.
  Future migrations may want to call `jj log -r 'all()'` or use a
  fresh commit baseline rather than just the working copy.

## Historical Context

- `meta/plans/2026-05-12-0056-restructure-meta-research-into-subject-subcategories.md` — the plan being verified
- `meta/work/0056-restructure-meta-research-into-subject-subcategories.md` — the source work item
- `meta/research/codebase/2026-05-11-0056-restructure-meta-research-into-subject-subcategories.md` — source research
- `meta/reviews/plans/2026-05-12-0056-restructure-meta-research-into-subject-subcategories-review-1.md` — plan review 1 (D1–D4 design decisions)

## Related Research

- `meta/research/codebase/2026-04-25-rename-tickets-to-work-items.md` — 0002 migration precedent
- `meta/research/codebase/2026-05-08-0052-documents-locator-config-driven-paths.md` — paths.* surface that 0056 extended

## Open Questions

1. Should the glyph-component file be moved via direct `jj mv` (one
   commit), via re-running migration 0004 (re-sweeps the whole repo),
   or left as-is with the broken links accepted as a known papercut?
2. Is the awk-not-Perl Step 3 implementation worth retroactively
   aligning to the plan, or is the as-shipped version the new
   reference?
3. The documents-locator nested-heading structure is a clean post-plan
   refinement — should the plan document be amended to reflect the
   ship state, or left as the original spec with the commit being the
   record of truth?
