# documents-locator does not surface design_inventories or design_gaps paths

## Observation

Plan 0052 ("documents-locator config-driven paths") wired the
`documents-locator` agent to a preloaded `paths` skill that emits all
configured document-discovery paths. The skill is sourced from
`scripts/config-read-all-paths.sh`, which iterates `PATH_KEYS` from
`scripts/config-defaults.sh` and excludes a hardcoded set of non-document
keys:

```bash
EXCLUDED_KEYS=(tmp templates integrations design_inventories design_gaps)
```

`tmp`, `templates`, and `integrations` are correctly excluded — they hold
working state, not searchable documents.

`design_inventories` (default `meta/design-inventories`) and `design_gaps`
(default `meta/design-gaps`) hold **design artifacts** — outputs of the
`design/inventory-design` and `design/analyse-design-gaps` skills. They
are documents in the same sense as `research/`, `plans/`, etc. and should
be discoverable by `documents-locator`.

## Why this matters

A user asking the agent "what design work has been done on X?" or
"do we have a design inventory for Y?" should get a hit if the relevant
file exists under `meta/research/design-inventories/` or `meta/research/design-gaps/`. With
the current exclusion list, the agent does not even know those directories
exist — they are absent from the preloaded **Configured Paths** block and
from the legend in `skills/config/paths/SKILL.md`.

## Fix outline

1. Remove `design_inventories` and `design_gaps` from the
   `EXCLUDED_KEYS` array in `scripts/config-read-all-paths.sh`.
2. Add legend entries to `skills/config/paths/SKILL.md`:
   - `design_inventories` — design inventories produced by
     `inventory-design` (default: `meta/design-inventories`)
   - `design_gaps` — design-gap analyses produced by
     `analyse-design-gaps` (default: `meta/design-gaps`)
3. Add categorisation entries in `agents/documents-locator.md` Core
   Responsibilities #2:
   - `design_inventories` — design inventories
   - `design_gaps` — design gap analyses
4. Add the same keys under Search Tips → "Check multiple locations" in
   the appropriate routing category (likely "Historic intent and context"
   or a new "Design artefacts" category).
5. Update Phase 2 tests in `scripts/test-config.sh` to expect 13 keys in
   the all-paths output rather than 11, including the two design keys.

## Why this was not caught

Plan 0052 explicitly listed `design_inventories` and `design_gaps` as
keys to exclude from the document-discovery output (see plan §"What
We're NOT Doing"). The original rationale was likely that they were
treated as state/working directories rather than documents, but the
distinction is wrong: design inventories and gap analyses are markdown
artefacts the agent should be able to discover.

## References

- `scripts/config-read-all-paths.sh:14` — `EXCLUDED_KEYS` definition site
- `skills/config/paths/SKILL.md` — Path legend (needs design entries)
- `agents/documents-locator.md` — categorisation list and search-routing
  tips (need design entries)
- `meta/plans/2026-05-08-0052-documents-locator-config-driven-paths.md`
  — landing plan that introduced the gap
- `skills/design/inventory-design/SKILL.md`,
  `skills/design/analyse-design-gaps/SKILL.md` — producers of the
  artefacts in question
