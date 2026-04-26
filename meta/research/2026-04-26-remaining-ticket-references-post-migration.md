---
date: "2026-04-26T19:01:48+01:00"
researcher: Toby Clemson
git_commit: f03a7dfe
branch: ticket-management
repository: accelerator
topic: "Remaining 'ticket' references after rename migration"
tags: [ research, migration, ticket, work-item, terminology, cleanup ]
status: complete
last_updated: "2026-04-26"
last_updated_by: Toby Clemson
last_updated_note: "Added follow-up research for agents/ directory"
---

# Research: Remaining 'ticket' References After Rename Migration

**Date**: 2026-04-26T19:01:48+01:00
**Researcher**: Toby Clemson
**Git Commit**: f03a7dfe (zryszqxl)
**Branch**: ticket-management
**Repository**: accelerator

## Research Question

We have just renamed everything related to "tickets" to instead use the term
"work-item" (in the category of skill "work"). However, there remain a number
of references to ticket throughout the codebase. Perform thorough research of
the codebase to identify any areas where tickets are still mentioned and
determine whether they were missed during the migration or are contextually
correct and should remain. This will be used to educate a plan to finalise
the work.

## Summary

The codebase contains **32 missed migration references** across **9 files**,
plus **29 borderline references** (generic English noun usage in pre-rename
work items) that are a judgment call on scope.

The vast majority of the codebase is clean. The missed references cluster
into five categories:

1. **Test helper scripts** (`test-work-item-scripts.sh`) — function names,
   fixture content, and test descriptions still use `ticket` terminology.
2. **Lens structure lint script** (`test-lens-structure.sh`) — variable name,
   function name, and several comments still say `TICKET_LENSES` /
   `ticket lens`.
3. **Templates** (`plan.md`, `pr-description.md`) — two templates have
   `ticket` in a frontmatter key, an example path, and placeholder text.
4. **Agent definition** (`agents/documents-locator.md`) — directory tree,
   categorisation list, example output paths, and a naming-hint all reference
   `tickets/` and "Ticket files".
5. **Scattered single-site misses** — one SKILL.md prose reference, one eval
   test-case name, one comment in a test script, and one body text in a
   work item document.

Everything in `skills/config/migrate/`, all historical meta documents
(`meta/research/`, `meta/plans/`, `meta/decisions/`, `meta/reviews/`,
`meta/validations/`), and all `CHANGELOG.md` entries are intentionally correct.

---

## Detailed Findings

### Category 1: Missed — `scripts/test-lens-structure.sh`

**9 locations** across comments, a variable name, a function definition, a
call site, and test output messages.

| Line  | Type              | Current text                                                                                     | Should be                                                                         |
|-------|-------------------|--------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------|
| 4     | comment           | `# Lint every ticket review lens SKILL.md`                                                       | `work-item review lens`                                                           |
| 9–12  | comment block     | `# The peer-ticket-lens reference check … TICKET_LENSES … ticket lenses … ticket-specific peers` | `work-item-lens … WORK_ITEM_LENSES … work-item lenses … work-item-specific peers` |
| 24    | comment           | `# Built-in ticket lens identifiers`                                                             | `work-item lens identifiers`                                                      |
| 25    | **variable**      | `TICKET_LENSES=(clarity completeness dependency scope testability)`                              | `WORK_ITEM_LENSES=(...)`                                                          |
| 27    | **function name** | `_is_ticket_lens()`                                                                              | `_is_work_item_lens()`                                                            |
| 29–30 | function body     | `for tl in "${TICKET_LENSES[@]}"`                                                                | `for tl in "${WORK_ITEM_LENSES[@]}"`                                              |
| 124   | comment           | `# For ticket lenses this should follow …`                                                       | `work-item lenses`                                                                |
| 136   | **call site**     | `if _is_ticket_lens "$LENS_ID"`                                                                  | `_is_work_item_lens "$LENS_ID"`                                                   |
| 146   | PASS message      | `peer ticket lenses ($PEER_COUNT found)`                                                         | `peer work-item lenses`                                                           |
| 149   | FAIL message      | `peer ticket lenses (need >= 3)`                                                                 | `peer work-item lenses`                                                           |

The functional variable `TICKET_LENSES` and function `_is_ticket_lens` are
the most important (lines 25, 27, 136) — the comments are secondary.

---

### Category 2: Missed — `scripts/test-hierarchy-format.sh`

**1 location** — comment only, not functional.

| Line | Type    | Current text                                                                                                             | Should be                                                  |
|------|---------|--------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------|
| 3–4  | comment | `# Check that the canonical tree fence in list-tickets/SKILL.md and refine-ticket/SKILL.md are byte-for-byte identical.` | `list-work-items/SKILL.md` and `refine-work-item/SKILL.md` |

The functional path variables at lines 21–22 already correctly reference
`list-work-items` and `refine-work-item`. Only the header comment is stale.

---

### Category 3: Missed — `skills/work/scripts/test-work-item-scripts.sh`

**11 locations** across function names, fixture frontmatter, fixture headings,
and test descriptions. This is the highest-density miss in the codebase.

| Line(s)                                                                   | Type                        | Current text                                                | Should be                                   |
|---------------------------------------------------------------------------|-----------------------------|-------------------------------------------------------------|---------------------------------------------|
| 66–67                                                                     | test name + comment         | `"non-ticket files only"`                                   | `"non-work-item files only"`                |
| 74–75                                                                     | test name + comment         | `"Mixed ticket and non-ticket files"`                       | `"Mixed work-item and non-work-item files"` |
| 218                                                                       | **fixture frontmatter**     | `ticket_id: 0001`                                           | `work_item_id: 0001`                        |
| 268                                                                       | **fixture frontmatter**     | `ticket_id: 0001`                                           | `work_item_id: 0001`                        |
| 283–284                                                                   | comment + **function name** | `make_ticket` (helper for creating work-item test fixtures) | `make_work_item`                            |
| 298                                                                       | fixture H1 heading          | `# 0001: Test Ticket`                                       | `# 0001: Test Work Item`                    |
| 307, 314, 321, 328, 335, 342, 350, 358, 364, 374, 380, 386, 392, 407, 413 | call sites                  | `make_ticket`                                               | `make_work_item`                            |
| 444–445                                                                   | comment + **function name** | `make_tagged_ticket`                                        | `make_tagged_work_item`                     |
| 456                                                                       | fixture H1 heading          | `# 0001: Test Ticket`                                       | `# 0001: Test Work Item`                    |
| 463, 470, 477, 484, 491, 498, 577, 584, 591                               | call sites                  | `make_tagged_ticket`                                        | `make_tagged_work_item`                     |
| 512                                                                       | fixture H1 heading          | `# 0001: Test Ticket` (Test 7)                              | `# 0001: Test Work Item`                    |
| 536                                                                       | fixture H1 heading          | `# 0001: Test Ticket` (Test 9)                              | `# 0001: Test Work Item`                    |

**Important:** Lines 218 and 268 use `ticket_id:` in fixture YAML frontmatter
for tests of `work-item-read-field.sh` and friends. These are NOT migration
test fixtures — they are work-item helper script tests. Since work-item files
should now carry `work_item_id:`, the fixtures should match.

---

### Category 4: Missed — `skills/work/review-work-item/SKILL.md`

**1 location** — prose reference to an internal variable name.

| Line | Type  | Current text            | Should be                  |
|------|-------|-------------------------|----------------------------|
| 104  | prose | `BUILTIN_TICKET_LENSES` | `BUILTIN_WORK_ITEM_LENSES` |

The actual variable in `scripts/config-read-review.sh:54` is already named
`BUILTIN_WORK_ITEM_LENSES` (correctly updated during migration). The SKILL.md
prose refers to it by the old name. The fix is purely cosmetic — the
behaviour is already correct.

---

### Category 5: Missed — `skills/work/update-work-item/evals/evals.json`

**1 location** — eval test-case name.

| Line | Type           | Current text              | Should be                    |
|------|----------------|---------------------------|------------------------------|
| 63   | eval test name | `"handles_legacy_ticket"` | `"handles_legacy_work_item"` |

---

### Category 6: Missed — `templates/plan.md`

**2 locations** — frontmatter key and example reference path.

| Line | Type                | Current text                                      | Should be                                           |
|------|---------------------|---------------------------------------------------|-----------------------------------------------------|
| 5    | **frontmatter key** | `ticket: "{ticket reference, if any}"`            | `work-item: "{work-item reference, if any}"`        |
| 107  | example path        | `- Original ticket: \`meta/tickets/eng-XXXX.md\`` | `- Original work item: \`meta/work/NNNN-title.md\`` |

Line 5 is the most impactful: every plan created by `/accelerator:create-plan`
will carry the stale `ticket:` frontmatter key until this is fixed.

---

### Category 7: Missed — `templates/pr-description.md`

**1 location** — instructional body text.

| Line | Type             | Current text                                                          | Should be                                                                |
|------|------------------|-----------------------------------------------------------------------|--------------------------------------------------------------------------|
| 24   | body placeholder | `[Link to relevant ticket, plan, or research document if applicable]` | `[Link to relevant work item, plan, or research document if applicable]` |

---

### Category 8: Missed — `meta/work/0026-init-skill-for-repo-bootstrap.md`

**1 location** — body text listing directory names.

| Line | Type      | Current text                                           | Should be                                           |
|------|-----------|--------------------------------------------------------|-----------------------------------------------------|
| 108  | body text | `(\`tmp/\`, \`templates/\`, \`tickets/\`, \`notes/\`)` | `(\`tmp/\`, \`templates/\`, \`work/\`, \`notes/\`)` |

This work item describes the `/accelerator:init` skill's `.gitkeep` logic. The
directory it names is now `meta/work/` not `meta/tickets/`.

---

### Category 9: Missed — `agents/documents-locator.md`

**6 locations** across the directory tree diagram, the document categorisation
list, example output paths, and a file-naming hint. All are MISSED — the agent
was not covered in the original migration passes.

| Line | Type | Current text | Should be |
|------|------|-------------|-----------|
| 25 | categorisation label + path | `- Tickets (usually in tickets/ subdirectory)` | `Work items (usually in work/ subdirectory)` |
| 54 | **directory tree** | `├── tickets/      # Ticket documentation` | `├── work/      # Work item files` |
| 74 | **example output heading** | `### Tickets` | `### Work Items` |
| 75–76 | example output paths | `` `meta/tickets/eng-1234.md` `` × 2 | `` `meta/work/eng-1234.md` `` |
| 121 | naming hint | `- Ticket files often named \`eng-XXXX.md\`` | `Work item files often named` |

The example output block (lines 74–76) is particularly important: the agent
will literally reproduce this section heading and path format when reporting
findings to the orchestrator, causing incorrect paths in every document-
location result until fixed.

The six other agent files (`codebase-locator.md`, `codebase-analyser.md`,
`codebase-pattern-finder.md`, `documents-analyser.md`, `reviewer.md`,
`web-search-researcher.md`) contain zero ticket references and are clean.

---

### Borderline — `meta/work/0001–0029` H1 headings

**29 locations** — generic English noun "Ticket" used as a synonym for
"work item" in the H1 heading of 29 pre-rename ADR drafting tasks.

All 29 follow this pattern:

```markdown
# ADR Ticket: <title>
```

These were created before the formal work-item system existed. The word
"Ticket" here is generic English (as in "Jira ticket" / "support ticket"),
not a reference to `meta/tickets/` or the renamed skill category.

Whether to update these depends on scope: if the rename is intended to
eliminate "ticket" as a generic synonym for "work item" across the entire
repository (including human-readable prose), they should become
`# ADR Work Item:`. If the rename was scoped to machine-readable references
(paths, skill names, frontmatter keys, variable names), they can remain.

The plan document (`meta/plans/2026-04-25-rename-tickets-to-work-items.md`)
scoped the rename to skill names, directory paths, config keys, template
field names, and script identifiers — not generic prose. On that basis these
headings are **out of scope and should remain unchanged**.

---

### Confirmed Clean

The following were exhaustively searched and contain no missed ticket
references:

**agents/**: `codebase-locator.md`, `codebase-analyser.md`,
`codebase-pattern-finder.md`, `documents-analyser.md`, `reviewer.md`,
`web-search-researcher.md` — clean; only `documents-locator.md` has misses
(covered in Category 9 above)

**`.github/workflows/main.yml`**: Clean — calls `mise run test` only, no
ticket-specific paths

**`meta/notes/`**, **`meta/specs/`**, **`meta/templates/`**: Empty (`.gitkeep`
only) — nothing to search

**`.gitignore`**, **`LICENSE`**, **`uv.lock`**: Clean

**`assets/`**: Image files only — not applicable

**scripts/**: `test-config.sh`, `test-boundary-evals.sh`,
`test-evals-structure.sh`, `test-evals-structure-self.sh`,
`test-format.sh`, `config-common.sh`, `config-dump.sh`,
`config-read-path.sh`, `config-read-review.sh`, `config-summary.sh`

**skills/work/**: All SKILL.md files (create-work-item, list-work-items,
extract-work-items, refine-work-item, update-work-item, stress-test-work-item,
review-work-item except line 104), all production scripts
(work-item-next-number.sh, work-item-read-field.sh, work-item-read-status.sh,
work-item-update-tags.sh, work-item-template-field-hints.sh), all evals
except update-work-item/evals/evals.json:63

**skills/**: All of planning/, github/, decisions/, research/, vcs/, config/
(except the correct migration script and tests), review/

**hooks/**: All hook scripts (vcs-detect.sh, config-detect.sh,
migrate-discoverability.sh, vcs-guard.sh)

**tasks/**: All Python task modules

**.claude-plugin/**: plugin.json, marketplace.json

**mise.toml**, **pyproject.toml**, **README.md**: Clean

**templates/**: work-item.md, adr.md, research.md, validation.md — clean

**meta/reviews/plans/**: All 14 plan review files — intentionally historical

**meta/validations/**: Stress-test validation — intentionally historical

---

### Contextually Correct — Should Not Be Changed

**`skills/config/migrate/migrations/0001-rename-tickets-to-work.sh`**
The migration script itself must reference old names (`paths.tickets`,
`meta/tickets`, `ticket_id`) as its input targets. These are correct.

**`skills/config/migrate/scripts/test-migrate.sh`**
Test fixtures deliberately set up the pre-migration state (`meta/tickets/`,
`ticket_id:` frontmatter, `paths.tickets` config) to verify the migration
transforms them correctly.

**`skills/config/migrate/SKILL.md`**
References the migration ID `0001-rename-tickets-to-work` — this is the
actual filename and must not be renamed.

**`CHANGELOG.md`**
All "ticket" references in the Unreleased section document the breaking
change and upgrade procedure. These name the old identifiers as the
*source* side of the rename arrow and are correct historical record.

**`.claude/settings.local.json:59–65`**
`allow` entries for the `mv` commands executed during implementation.
Stale (those moves will never be needed again) but not semantically wrong.
Safe to prune as a housekeeping item, separately from the migration cleanup.

---

## Code References

- `scripts/test-lens-structure.sh:25` — `TICKET_LENSES` variable
- `scripts/test-lens-structure.sh:27` — `_is_ticket_lens()` function
- `scripts/test-lens-structure.sh:136` — `_is_ticket_lens` call site
- `scripts/test-hierarchy-format.sh:3-4` — stale comment
- `skills/work/review-work-item/SKILL.md:104` — `BUILTIN_TICKET_LENSES` prose
- `skills/work/scripts/test-work-item-scripts.sh:218,268` — fixture `ticket_id:`
- `skills/work/scripts/test-work-item-scripts.sh:283-284` — `make_ticket`
  function
- `skills/work/scripts/test-work-item-scripts.sh:444-445` — `make_tagged_ticket`
  function
- `skills/work/scripts/test-work-item-scripts.sh:298,456,512,536` — fixture
  headings
- `skills/work/update-work-item/evals/evals.json:63` — `handles_legacy_ticket`
  test name
- `templates/plan.md:5` — `ticket:` frontmatter key
- `templates/plan.md:107` — `meta/tickets/` example path
- `templates/pr-description.md:24` — "relevant ticket" placeholder
- `meta/work/0026-init-skill-for-repo-bootstrap.md:108` — `tickets/` directory
  name
- `agents/documents-locator.md:25` — categorisation label `Tickets (usually in tickets/)`
- `agents/documents-locator.md:54` — directory tree entry `├── tickets/`
- `agents/documents-locator.md:74–76` — example output heading `### Tickets` and paths
- `agents/documents-locator.md:121` — naming hint "Ticket files often named"

---

## Architecture Insights

The missed references cluster around **test infrastructure** and
**templates** — two areas that Phase 1–3 of the plan touched but where some
internal test helper details were overlooked:

1. **`test-lens-structure.sh`** predates the work-item rename and was not
   part of any work-item–specific test phase; its `TICKET_LENSES` variable
   was introduced for the work-item lens review system but uses the old
   naming convention throughout.

2. **`test-work-item-scripts.sh`** was renamed and updated for production
   script names but the internal helper functions (`make_ticket`,
   `make_tagged_ticket`) and test fixture content (`ticket_id:`) were left
   using old terminology.

3. **Templates** (`plan.md`, `pr-description.md`) are consumed by humans and
   LLMs every time a plan or PR description is created, making the `ticket:`
   frontmatter key in `plan.md` particularly impactful.

The production code path is entirely clean. Every user-facing skill, helper
script, and config script uses correct `work-item` terminology. The missed
references are confined to test scripts, templates, and prose.

---

## Historical Context

- `meta/plans/2026-04-25-rename-tickets-to-work-items.md` — the six-phase
  plan that drove the rename; Phases 1–6 now complete
- `meta/decisions/ADR-0022-work-item-as-canonical-term.md` — documents the
  decision to use "work item" as the canonical term
- `meta/decisions/ADR-0023-meta-directory-migration-framework.md` — the
  migration framework design

---

## Open Questions

1. **`# ADR Ticket:` headings in `meta/work/0001–0029`**: Treat as
   out-of-scope generic prose or update to `# ADR Work Item:`? Recommendation:
   leave them — the rename scope was machine-readable identifiers, not generic
   English nouns in human-authored body text.

2. **`.claude/settings.local.json:59–65` stale allow entries**: These should
   be removed as housekeeping (they permit `mv` commands that will never be
   run again), but they are not incorrect. Treat as a separate cleanup item
   outside the rename plan scope.
