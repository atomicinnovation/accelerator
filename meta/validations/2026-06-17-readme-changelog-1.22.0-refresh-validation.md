---
type: plan-validation
id: "2026-06-17-readme-changelog-1.22.0-refresh-validation"
title: "Validation Report: README and CHANGELOG 1.22.0 Refresh"
date: "2026-06-17T17:24:12+00:00"
author: "Toby Clemson"
producer: validate-plan
status: complete
result: pass
target: "plan:2026-06-17-readme-changelog-1.22.0-refresh"
relates_to: ["codebase-research:2026-06-17-readme-changelog-1.22.0-refresh"]
tags: [changelog, readme, release, docs]
last_updated: "2026-06-17T17:24:12+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: README and CHANGELOG 1.22.0 Refresh

Validated in-session, against the four implementation commits (`nrpn` planning
artifacts → `rtmp` CHANGELOG → `vqxm` Jira/Linear → `lyqk` accuracy pass →
`vput` Getting Started + hero). The plan was amended mid-implementation (the
`TEMPLATE_KEYS` discovery, below); the validation is against the **amended**
plan, and the implementation matches it.

### Implementation Status

- ✓ **Phase 1: CHANGELOG `[Unreleased]` refresh** — Fully implemented (`rtmp`).
  Added group (Linear, create-note, 5 visualiser items, `rejected` ADR status,
  sync ergonomics), 2 Changed items (typed-linkage clustering, reader polish),
  and a Migrations subsection for 0007. Keep-a-Changelog structure preserved.
- ✓ **Phase 2: Getting Started + hero screenshot** — Fully implemented (`vput`).
  Light/dark plan hero captured fresh against the real
  `create-note-skill` plan at 1280×800, embedded via a `<picture>` block;
  Getting Started block added; `### Getting Started` renamed to
  `### Managing Configuration`; "Jump to installation" line removed;
  Installation cross-reference added.
- ✓ **Phase 3: Remote Work Item Management umbrella** — Fully implemented
  (`vqxm`). Umbrella section added; Jira demoted to `### Jira` with the
  `jira-integration` anchor preserved; Linear subsection added with an 8-row
  skill table. Sub-headings prefixed with the tracker name (`#### Jira
  Configuration`, `#### Linear Configuration`, …) per the user's in-session
  decision, which also resolved a pre-existing `Configuration` slug collision.
- ✓ **Phase 4: Accuracy pass** — Fully implemented (`lyqk`), with an
  in-session correction (see Deviations). Integration-availability clause,
  `notes/` row → `create-note`, visualiser editor-key rows, Views table
  (tickets → work items, typed-linkage clustering), the browsable
  templates/RCAs/search/recovery note, and the template-keys list.

### Automated Verification Results

All 17 plan success-criteria checks re-run against the final committed state:

- ✓ CHANGELOG parses under `keepachangelog`; single `[Unreleased]`; precedes
  `[1.21.0]`.
- ✓ Both hero assets present; no "Jump to installation"; `#installation`
  target intact; exactly one "Getting Started" heading; `Managing
  Configuration` present.
- ✓ Umbrella section present; `<a id="jira-integration">` preserved; **no
  duplicate heading slugs** (`uniq -d` empty); all 8 Linear skill names
  referenced.
- ✓ `notes/` row shows `create-note`; editor keys documented; `create-note`
  mentioned; template-keys list includes `codebase-research`,
  `` `design-inventory` ``, and `` `note` `` (all-13 filename-driven surface);
  no "tickets" terminology remains.
- ✓ `mise run check` exits 0 (non-regression gate).

> Note on the gate: `mise run check` flaked red twice on the first attempt of
> each run via the **known pyrefly / `node_modules` race**
> (`types:build-system:check` globbing `frontend/node_modules` while node deps
> were mid-install — hit `@types/pngjs` then `punycode`). Confirmed a false
> negative by running `build-system:check` in isolation (0 errors) and then
> re-running the full `mise run check` to a clean `EXIT=0`.

### Code Review Findings

#### Matches Plan

- Every phase's specified edit is present and matches the plan's prescribed
  content (CHANGELOG entries, hero `<picture>` block, umbrella framing, Linear
  skill table, editor-key rows, Views table).
- The Linear Usage cells match each skill's `argument-hint` frontmatter (the
  plan's binding pre-merge gate): `create-linear-issue <work-item-file>`
  required positional, `<IDENTIFIER>` (not `<ID>`), `<STATE-NAME>` transition
  arg, and `attach`'s `--url`/`--file`.
- Sidebar vocabulary asserted in the CHANGELOG/README ("Operate" category,
  "META" section, "Templates") verified against the running dev visualiser and
  `Sidebar.tsx`.
- 80-column prose wrapping held for all new prose (tables, code fences, and the
  hero `<picture>` HTML exempt, per the plan's precedent).

#### Deviations from Plan

- **`TEMPLATE_KEYS` premise corrected mid-flight (intentional, plan amended).**
  The plan's original Phase 4 §5 (and §2 note, and a Key Discovery) assumed
  `TEMPLATE_KEYS` gates template customisation, so it instructed trimming the
  keys list to 6 and warning that `note` was uncustomisable. Investigation
  showed the `configure templates` subcommands are filename-driven over
  `templates/*.md` (all 13 templates are ejectable); `TEMPLATE_KEYS` only feeds
  the config dump. The implementation now lists **all 13** ejectable keys and
  omits the false `note` caveat. The plan, its §5 success criterion, and work
  item `0113` (repurposed from "register note in TEMPLATE_KEYS" to "derive the
  config dump from the templates directory") were all updated to match. Plan
  and implementation are in sync.
- **Tracker-prefixed sub-headings (Phase 3).** Plan §2/§3 said to demote Jira's
  sub-headings to `####` keeping their names; instead they were prefixed
  (`#### Jira Configuration`, etc.) per the user's decision, to satisfy the
  plan's own "no duplicate heading slugs" gate (which a pre-existing
  `## Configuration` vs `### Configuration` collision had already been failing).
  Improvement, not a regression.
- **Minor plan inaccuracy (cosmetic).** Phase 2 mechanics reference
  `mise run dev:up`; the actual task is `mise run dev`. The hero was captured
  with the real task; no impact.

#### Potential Issues

- None material. README/CHANGELOG are not linted, so doc correctness rests on
  the grep gates + manual review (as the plan acknowledges) rather than tooling.
- The repurposed work item `0113` is now the only follow-up: the config dump
  still under-reports customisable templates until it derives rows from the
  directory. Out of scope for this docs plan by design.

### Manual Testing Required

1. GitHub render (the one outstanding manual item, `[~]` in the plan):
  - [ ] On a GitHub branch preview, confirm the hero `<picture>` switches
        correctly between light and dark, and the plan content is legible.
  - [ ] Sanity-check the bare `<img>` light fallback in a renderer that ignores
        `prefers-color-scheme`. (Captures verified locally: both 1280×800,
        identical dimensions, legible in both themes; fallback → light capture.)
2. Flow read:
  - [ ] Read the README top-to-bottom for flow after the top-of-file and
        Remote Work Item Management restructures.

### Recommendations

- Push a branch and eyeball the hero on GitHub before merge (only remaining
  unverified criterion).
- Keep work item `0113` (derive the config dump's template rows from the
  `templates/` directory) on the backlog so the config dump stops
  under-reporting — it is the structural fix behind this pass's keys-list
  correction.
- This refresh leaves `[Unreleased]` ready for `release:prepare` to promote at
  1.22.0 cut time (no version bump performed here, per plan scope).
