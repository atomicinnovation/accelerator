# Future Work: Restructure `meta/research/codebase/` into subcategories

## Problem

`meta/research/codebase/` is currently a flat directory holding research documents of
heterogeneous types — codebase investigations (the bulk, produced by
`research-codebase`), technical-spike write-ups, idea explorations, and now
prospectively design-convergence research (`design-inventories`,
`design-gaps` — see `meta/research/codebase/2026-05-02-design-convergence-workflow.md`).

As the research corpus grows, the flat layout starts to obscure rather than
help — finding "the codebase research about X" requires scanning past unrelated
exploratory notes; future tooling that wants to operate on a specific kind of
research (e.g. enumerate all codebase investigations) has no structural
affordance to do so.

A nested layout — e.g. `meta/research/codebase/`, `meta/research/codebase/ideas/`,
`meta/research/design-inventories/`, `meta/research/design-gaps/` — would
group research by kind, mirror the `documents-locator` agent's existing
mental model of "categories of research", and create a natural home for new
research types as they emerge.

## Why this is not done now

It is a bigger structural change than its surface suggests:

1. **Migration of existing files.** Every existing flat-laid file in
   `meta/research/codebase/` needs a target subdirectory. The categorisation is mostly
   obvious for the codebase-research files but ambiguous for one-off notes
   that don't fit a clean bucket.
2. **`research-codebase` skill path resolution.** The skill's filename
   convention (`YYYY-MM-DD-ENG-XXXX-description.md` in
   `<paths.research>`) presumes a single flat directory. If the layout
   nests, either the skill writes to `<paths.research>/codebase/...` (new
   subpath logic) or a new path key like `paths.research_codebase` is
   introduced. Either way the skill, the README path table, and the
   configure-help table all need updating.
3. **`documents-locator` agent.** Its inline directory map (the "Research
   Documents", "Implementation Plans", etc. block in
   `agents/documents-locator.md`) and its output format would need
   restructuring to surface the subcategories.
4. **Backward compatibility.** Existing inbound links from plans, ADRs,
   work items, and READMEs that reference research documents by path
   would break on file moves unless the moves are done as renames that
   git can track.
5. **Decision criteria.** It is not obvious whether *every* research type
   should nest from the start, or only those with multiple files
   (e.g. codebase has many; ideas may have few). A consistent rule —
   "every research type lives under a named subdirectory" — is simpler
   than "nest on threshold N", but commits to subdirectories that may
   only ever hold one file.

This is therefore not a bundle-with-something-else change. It deserves its own
ADR, its own implementation plan, and its own migration commit.

## Trigger / forcing function

The introduction of the design-convergence workflow (`design-inventories` and
`design-gaps`, captured in
`meta/research/codebase/2026-05-02-design-convergence-workflow.md`) was the first
moment the flat layout felt actively wrong — design inventories are
distinctly not codebase research, and pluralising a flat directory with
artifacts of a different shape would be the worst of both worlds.

The choice taken in that work was to keep the inventories at top-level
(`meta/research/design-inventories/`, `meta/research/design-gaps/`) rather than nest them under
research. That keeps the design-convergence work clean but defers — does
not resolve — the question of how research itself should be organised.

If a second case emerges where a new research-shaped artifact type is being
introduced, or if the existing flat directory crosses some readability
threshold, the restructure should land. The design-convergence workflow may
also surface useful prior art: its `paths.design_inventories` /
`paths.design_gaps` registration pattern is the same shape a future
`paths.research_codebase` / `paths.research_ideas` split would use.

## Suggested path forward

1. Write an ADR (`Research directory subcategorisation`) capturing the
   subcategories chosen, the rule for when a new subcategory is added, and
   the migration approach (rename existing files; preserve git history).
2. Survey all existing inbound references to `meta/research/codebase/*.md` from plans,
   ADRs, work items, and READMEs; plan the rename so git tracks moves
   cleanly.
3. Update `research-codebase` to write into the appropriate subdirectory
   (likely `meta/research/codebase/` by default, configurable).
4. Update `documents-locator` agent inline directory map and output format.
5. Update README's `meta/` directory table and the configure-help paths
   table.

## References

- `meta/research/codebase/2026-05-02-design-convergence-workflow.md` — the work that
  surfaced this; see §9.8 (rejected alternative: nested layout under
  `meta/research/codebase/`) for the discussion at the time.
- `agents/documents-locator.md` — agent whose directory map would need
  restructuring.
- `skills/research/research-codebase/SKILL.md` — skill whose path
  resolution and filename convention would change.
- `meta/notes/2026-04-26-agents-hardcode-default-directory-locations.md` —
  related concern about agent path-awareness; may interact with this work
  if agents are made config-aware first.
