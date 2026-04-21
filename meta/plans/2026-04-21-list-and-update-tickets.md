---
date: "2026-04-21T00:00:00+01:00"
type: plan
skill: create-plan
status: draft
---

# Ticket Listing and Updating Skills (Phase 3)

## Overview

Implement two new skills — `list-tickets` (discovery and filtering) and
`update-ticket` (frontmatter field edits) — following the same structural
conventions established in Phase 2. Both skills are developed TDD-style:
approach evals are defined first as the specification, then
`/skill-creator:skill-creator` is used to author and validate each
SKILL.md against them.

## Current State Analysis

Phases 1 and 2 are complete. The following artifacts are confirmed:

- `templates/ticket.md` — template with 8 frontmatter fields (a
  `title:` field is added as a prerequisite to this phase — see
  Prerequisites). Comments on `type`, `status`, and `priority`
  enumerate the shipping defaults but the template is user-overridable
  via `meta/templates/ticket.md`
- `skills/tickets/scripts/ticket-next-number.sh` — sequential numbering
- `skills/tickets/scripts/ticket-read-status.sh` — status extraction
- `skills/tickets/scripts/ticket-read-field.sh` — generic field extraction
- `skills/tickets/scripts/test-ticket-scripts.sh` — 44-test regression
  suite (uses shared `scripts/test-helpers.sh`)
- `skills/tickets/create-ticket/SKILL.md` — interactive ticket creation
- `skills/tickets/extract-tickets/SKILL.md` — batch extraction from
  documents
- `.claude-plugin/plugin.json` — `"./skills/tickets/"` registered; all
  subdirectories scanned automatically

The 29 existing tickets in `meta/tickets/` use a minimal schema
(`title`, `type: adr-creation-task`, `status: todo|done`) that predates
the richer `templates/ticket.md` schema. New tickets follow the
template schema. Both must coexist; neither skill may assume every
ticket carries every field.

After the Prerequisites below are applied, both schemas uniformly
carry a `title:` frontmatter field — legacy tickets already have it,
and the template gains it. `title:` is the authoritative source for
each ticket's title; the body H1 is display text kept in sync by
`create-ticket`, `extract-tickets`, and `update-ticket`.

### Key Discoveries

- `skills/decisions/review-adr/SKILL.md` is the closest structural model
  for `update-ticket`: reads status via a companion script, edits YAML
  frontmatter in place, and also updates the matching body `**Status**:`
  line for consistency (SKILL.md lines 180–214). It does not use a
  dedicated frontmatter-write script — the LLM uses the `Edit` tool
- No existing skill is a direct model for `list-tickets`. The
  discovery step in `review-adr` (SKILL.md lines 32–70) is the closest
  precedent: scans the decisions directory, parses frontmatter, and
  presents the result for selection
- **Status, type, and priority values are user-defined** via the
  ticket template (comments on each field list the shipping defaults,
  but a user can replace the template and introduce custom values).
  Consequently, neither skill may hardcode these values — the skill
  must read from the template at runtime when displaying options as
  hints, and must not reject values that fall outside those hints
- **Status transitions are NOT enforced in this phase.** A future
  feature will allow users to declare valid transitions in
  configuration. Until then, `update-ticket` treats status changes as
  arbitrary field updates. The user decides what is valid
- The frontmatter parser in `ticket-read-field.sh` is first-match-wins
  and preserves raw YAML array values verbatim (`tags: [a, b]` is
  returned as the literal string `[a, b]`). Consumers that need typed
  arrays parse them via `config_parse_array` in `scripts/config-common.sh`

## Desired End State

After this plan is complete:

- `skills/tickets/list-tickets/SKILL.md` exists and is callable as
  `/list-tickets`
- `skills/tickets/update-ticket/SKILL.md` exists and is callable as
  `/update-ticket`
- `bash skills/tickets/scripts/test-ticket-scripts.sh` exits 0 with all
  44 tests passing (no regressions in Phase 1 scripts)
- Both SKILL.md files pass all approach evals via
  `/skill-creator:skill-creator`
- Both SKILL.md files follow the structural conventions of Phase 2:
  `disable-model-invocation: true`, fully-qualified `accelerator:`
  agent prefixes in fallback list, `allowed-tools` restricted to
  `config-*` and `tickets/scripts/*` patterns

## What We're NOT Doing

- **No status transition enforcement.** Arbitrary transitions are
  allowed. A future feature will add user-configurable transition
  graphs
- **No batch updates.** `/update-ticket all stories under 0001 mark
  ready` is out of scope. Single-ticket updates only. Callers compose
  batches by invoking the skill multiple times
- **No ticket renaming.** Changing `ticket_id` is hard-blocked;
  the filename prefix is the authoritative ticket number. To
  renumber, rename the file (`jj mv`) and update `ticket_id` to
  match. `update-ticket` does not orchestrate this
- **No audit trail / history section inside tickets.** Git history
  (via `jj`) is the system of record for changes
- **No new bash helper scripts except `ticket-update-tags.sh` and
  `ticket-template-field-hints.sh`.** The LLM edits frontmatter
  directly via the `Edit` tool, matching `review-adr`. The two
  exceptions are narrow helpers: `ticket-update-tags.sh` owns the
  canonical tag array round-trip (P.4) and
  `ticket-template-field-hints.sh` owns template hint extraction
  with a hardcoded fallback (P.5)
- **No migration of legacy `todo`/`done` statuses.** Legacy tickets
  coexist as-is. `update-ticket` recognises any existing status
  without requiring it to be in the template's enumerated defaults
- **No modification of Phase 1 scripts**
- **No consolidated frontmatter parser.** Both new skills
  re-implement multi-field frontmatter parsing in their prompts
  (matching `ticket-read-field.sh`'s single-field rules). A
  `ticket-read-frontmatter.sh` that emits all fields could reduce
  this duplication; deferred to a future phase if drift becomes a
  problem
- **No structural refactor of Phase 2 skills.** The only Phase 2
  touchup permitted is the minimal change needed to populate
  `title:` in frontmatter (see Prerequisites). No other changes.
- **No modification of the ticket template beyond adding `title:`.**
  The template gains a `title:` frontmatter field as a prerequisite
  to this phase (see Prerequisites); no other template changes are
  in scope.
- **No implementation of `review-ticket`, `stress-test-ticket`, or
  `refine-ticket`** — those are Phases 4–6

## Implementation Approach

Each subphase follows TDD order:

1. Approach evals in this plan are the specification (written first)
2. Invoke `/skill-creator:skill-creator` with the spec and evals
3. Run the evals built into the skill-creator flow
4. Iterate on the SKILL.md until all evals pass
5. Run the Phase 1 regression suite before marking the subphase done

---

## Prerequisites

Before the subphases below can proceed, two small precursor changes
align the template and Phase 2 skills with this phase's reliance on
frontmatter `title:` as the authoritative title source.

### P.1 Add `title:` to the ticket template

**File**: `templates/ticket.md`

Add a `title:` frontmatter field. Suggested placement: immediately
after `ticket_id:` (title is part of identity; placing it adjacent
keeps identity fields grouped).

```yaml
ticket_id: NNNN                              # from ticket-next-number.sh
title: "Title as Short Noun Phrase"           # human-readable title; kept in sync with body H1
date: "YYYY-MM-DDTHH:MM:SS+00:00"            # date -u +%Y-%m-%dT%H:%M:%S+00:00
# ...remaining fields unchanged
```

The body H1 `# NNNN: Title as Short Noun Phrase` remains as-is; it is
display text kept in sync with frontmatter `title:`.

Note for changelog / release notes: users with custom templates
(overridden via `meta/templates/ticket.md`) should add `title:` to
their override to ensure tickets created from their template include
the field. Without it, tickets created from the custom template will
lack `title:` frontmatter.

### P.2 Update `create-ticket` to populate frontmatter `title:`

**File**: `skills/tickets/create-ticket/SKILL.md`

Minimal touchup: when the skill writes a new ticket, it must set both
the frontmatter `title:` field and the body H1 to the user-supplied
title. The body H1 format remains `# NNNN: <title>`. Additionally,
update the quality-guidelines field enumeration to include `title:`
alongside the other listed frontmatter fields, so the LLM does not
omit it when following the checklist strictly. No other changes to
`create-ticket` are in scope.

### P.3 Update `extract-tickets` to populate frontmatter `title:`

**File**: `skills/tickets/extract-tickets/SKILL.md`

Minimal touchup matching P.2: each extracted ticket must have `title:`
set in frontmatter and rendered in the body H1. Additionally, update
the quality-guidelines field enumeration to include `title:` alongside
the other listed frontmatter fields. No other changes to
`extract-tickets` are in scope.

### P.4 Create `ticket-update-tags.sh`

**File**: `skills/tickets/scripts/ticket-update-tags.sh`

A narrow bash helper that owns the canonical tag array round-trip:
parse the current value, apply an add or remove mutation, and emit
the result in canonical flow-style format.

**Interface**:
```
ticket-update-tags.sh <ticket-path> add <tag>
ticket-update-tags.sh <ticket-path> remove <tag>
```

**Behaviour**:

Pre-checks (before any field reading):
1. Validate `<ticket-path>` exists. If not, exit 1 with stderr:
   `"Error: file not found: <ticket-path>"`
2. Validate the file has YAML frontmatter (opens with `---` and has
   a closing `---`). If not, exit 1 with stderr matching
   `ticket-read-field.sh` phrasing (`"no frontmatter"` /
   `"unclosed frontmatter"`).
3. Block-style detection: read the raw `tags:` line in the
   frontmatter. If the line is `tags:` with no inline value (or
   only whitespace after the colon) AND the next line matches
   `^[[:space:]]+- `, the field is block-style. Exit 1 with stderr:
   `"Error: tags field is in block format — convert to
   tags: [...] first. Example: tags: [api, search]"`

After pre-checks pass, read the `tags` field value via
`ticket-read-field.sh`. If the field is absent
(`ticket-read-field.sh` exits 1 after pre-checks already confirmed
the file and frontmatter are valid), treat as "field not present."

Mutation rules:
- Canonical format: flow-style inline array with single space after
  comma, no quotes unless the value contains `,`, `:`, or `#`
  (e.g. `[api, search, backend]`)
- `add`: appends the tag at the end; preserves existing order; if
  the tag is already present, exits 0 and prints `no-change` to
  stdout
- `remove`: removes the named tag; preserves order of remaining
  tags; if the tag is not present, exits 0 and prints `no-change`
  to stdout
- Removing the last tag yields `[]` (empty array), never deletes
  the field
- If the `tags` field is absent from frontmatter: `add` creates
  `tags: [<tag>]`; `remove` exits 0 and prints `no-change`
- If `tags: []` (empty): `add` appends; `remove` prints `no-change`
- If bare `tags:` with no value (raw value from `ticket-read-field.sh`
  is an empty string): treat identically to `tags: []`
- Output on success (when a change is made): the new canonical
  array string, e.g. `[api, search, backend]`. The caller (the
  `update-ticket` SKILL.md) uses this value with the `Edit` tool
  to rewrite the frontmatter line.

**Tests**: Add tests to `test-ticket-scripts.sh` covering:
- Add to existing flow-style array
- Add duplicate (no-change)
- Remove existing tag
- Remove absent tag (no-change)
- Remove last tag → `[]`
- Remove from empty `[]` → no-change
- Add to absent field → `[new-tag]`
- Add to empty `[]` → `[new-tag]`
- Block-style detection (raw file check) → exit 1 with error
- Non-existent file → exit 1 with file-not-found error
- Missing frontmatter → exit 1 with no-frontmatter error
- Unclosed frontmatter → exit 1 with unclosed-frontmatter error
- Tag containing comma is quoted in output
- Tag containing colon is quoted in output
- Tag containing hash is quoted in output

### P.5 Create `ticket-template-field-hints.sh`

**File**: `skills/tickets/scripts/ticket-template-field-hints.sh`

A narrow bash helper that extracts hint values for a given
frontmatter field from the ticket template, with a hardcoded
fallback when the template has no parseable comment.

**Interface**:
```
ticket-template-field-hints.sh <field>
```

**Behaviour**:
- Reads the ticket template via `config-read-template.sh ticket`
- Finds the frontmatter line matching `^<field>:` in the template
- Parses the trailing comment: everything after the first `#` on
  the line is split on `|`, each token whitespace-trimmed. The
  resulting values are printed one per line to stdout.
  For example, `status: draft  # draft | ready | in-progress | ...`
  produces:
  ```
  draft
  ready
  in-progress
  review
  done
  blocked
  abandoned
  ```
- If the template line has no `#` comment, or the field is not
  found in the template, fall back to a hardcoded default list
  matching the shipping template values:
  - `type`: story, epic, task, bug, spike
  - `status`: draft, ready, in-progress, review, done, blocked,
    abandoned
  - `priority`: high, medium, low
  - Any other field: exit 0 with no output (no hints available)
- Exit code: always 0. Empty output means no hints.

**Tests**: Add tests to `test-ticket-scripts.sh` covering:
- Field with trailing comment → parsed values
- Field with no trailing comment → hardcoded fallback
- Field not in template → hardcoded fallback for known fields
- Unknown field with no comment → empty output
- User-overridden template with custom values → custom values
  returned (not hardcoded)
- `config-read-template.sh` failure (template missing) → hardcoded
  fallback for known fields, exit 0
- Hardcoded fallback values match the shipping template's trailing
  comments (tripwire test: fails if either side changes without
  the other)

### Prerequisite Success Criteria

- [x] `grep -n "^title:" templates/ticket.md` returns a line
- [ ] A ticket created by `/create-ticket` after P.2 contains a
  non-empty `title:` frontmatter field whose value matches the body H1
  suffix after the `NNNN: ` prefix
- [ ] A ticket extracted by `/extract-tickets` after P.3 contains a
  non-empty `title:` frontmatter field matching the body H1 suffix
- [x] File exists: `skills/tickets/scripts/ticket-update-tags.sh`
- [x] `ticket-update-tags.sh` tests pass within `test-ticket-scripts.sh`
  (add, remove, duplicate, absent, empty, last-removal, block-style,
  file-not-found, malformed-frontmatter, special characters per
  delimiter type)
- [x] File exists: `skills/tickets/scripts/ticket-template-field-hints.sh`
- [x] `ticket-template-field-hints.sh status` returns the 7 shipping
  default statuses (one per line)
- [x] `ticket-template-field-hints.sh` tests pass within
  `test-ticket-scripts.sh` (comment parsing, fallback, unknown field,
  custom template, config-read-template failure, fallback-vs-template
  tripwire)
- [x] The Phase 1 regression suite still passes
  (`bash skills/tickets/scripts/test-ticket-scripts.sh`)

---

## Subphase 3.1: `list-tickets` Skill

### Overview

Author `skills/tickets/list-tickets/SKILL.md` — a read-only skill that
scans the configured tickets directory, parses frontmatter from each
ticket, optionally filters by user-specified criteria (natural language
or structured), and presents the result as a markdown table. No
sub-agents are spawned; no files are written.

### Approach Evals

These 20 scenarios are the "test" half of TDD. Define them before
authoring the SKILL.md and pass them as the eval specification to
`/skill-creator:skill-creator`.

**Scenario 1 — Bare invocation lists all tickets**
Input: `/list-tickets` (no arguments)
Expected: Skill scans `{tickets_dir}`, parses frontmatter from every
`NNNN-*.md` file, and presents a markdown table with columns for
number (from filename), title (from frontmatter `title:`), type,
status, and priority. Rows are sorted by ticket number ascending. No
sub-agents are spawned.

**Scenario 2 — Empty tickets directory**
Input: `/list-tickets` when `{tickets_dir}` exists but contains no
`NNNN-*.md` files
Expected: Skill prints "No tickets found in `{tickets_dir}`." and
exits cleanly. No table is rendered.

**Scenario 3 — Missing tickets directory**
Input: `/list-tickets` when `{tickets_dir}` does not exist at all
Expected: Skill prints "Tickets directory `{tickets_dir}` not found.
Check the `paths.tickets` configuration or run `/create-ticket` to
create the first ticket." and exits cleanly. No table is rendered.

**Scenario 4 — Filter by status (natural language)**
Input: `/list-tickets only drafts`
Expected: Skill parses the filter as `status: draft` and returns only
matching tickets. The filter expression used is echoed back to the
user above the table for transparency.

**Scenario 5 — Filter by type**
Input: `/list-tickets epics`
Expected: Skill parses the filter as `type: epic` and returns only
tickets where the frontmatter `type` equals `epic`.

**Scenario 6 — Combined filter (two fields)**
Input: `/list-tickets bugs in review`
Expected: Skill parses the filter as `type: bug AND status: review`
and returns tickets where both conditions hold.

**Scenario 7 — Filter by priority**
Input: `/list-tickets high priority`
Expected: Skill parses as `priority: high` and filters accordingly.

**Scenario 8 — Filter by tag**
Input: `/list-tickets tagged backend`
Expected: Skill filters to tickets whose `tags` frontmatter array
contains the string `backend`. Raw YAML array values from
`ticket-read-field.sh` (`[backend, api]`) are parsed before matching.
Tickets with `tags: []` or absent `tags` field are excluded from the
match (not errors). Block-style tag arrays and quoted tag values are
parsed correctly.

**Scenario 9 — Filter by parent (normalised matching)**
Input: `/list-tickets under 0042`
Expected: Skill filters to tickets whose `parent` frontmatter field
equals `0042`. Both the filter value and the stored `parent` value
are normalised before comparison: stripped of quotes, zero-padded to
4 digits. So `parent: "0042"`, `parent: 0042`, `parent: 42`, and
`parent: "42"` all match the filter `under 0042`. Likewise,
`/list-tickets under 42` matches the same tickets. Result title:
"Children of 0042".

**Scenario 10 — No matches**
Input: A filter that matches zero tickets
Expected: Skill prints "No tickets matched: `{filter description}`."
and exits cleanly. No empty table rendered.

**Scenario 11 — Legacy tickets are included**
Input: `/list-tickets` against a directory containing both new-schema
tickets and the 29 legacy `adr-creation-task` tickets
Expected: Legacy tickets appear in the table. Both schemas carry
`title:` uniformly, so the Title column is populated for every row.
Fields absent from legacy frontmatter (`priority`, `tags`, `parent`)
are rendered as `—`. No legacy ticket is silently dropped because of
its schema.

**Scenario 12 — Malformed frontmatter does not crash the listing**
Input: `/list-tickets` when one ticket has unclosed frontmatter (first
line is `---` but no closing `---`)
Expected: Skill warns `"<filename>: skipped — unclosed frontmatter"`
and continues listing the remaining tickets. The table includes every
well-formed ticket.

**Scenario 13 — Missing frontmatter does not crash the listing**
Input: `/list-tickets` when one file in `{tickets_dir}` has no
frontmatter block at all (body-only markdown)
Expected: Skill warns `"<filename>: skipped — no frontmatter"` and
continues. The table includes every well-formed ticket.

**Scenario 14 — Hierarchy mode groups children under parents**
Input: `/list-tickets hierarchy` (or "show hierarchy", "as a tree")
Expected: Skill produces a nested listing where tickets with a
non-empty `parent` field appear indented under their parent.
Orphan children (parent field points to a ticket that does not exist)
appear at the top level with a note `"(parent 0099 not found)"`.
Tickets with no parent appear at the top level.

**Scenario 15 — Free-text search in title**
Input: `/list-tickets about authentication`
Expected: Skill performs a case-insensitive substring match against
the `title:` frontmatter field for tickets containing "authentication".
Both new-schema and legacy tickets carry `title:` uniformly, so no
fallback to body parsing is needed. A ticket with no `title:`
frontmatter is excluded from the match (and a warning is emitted, per
Scenarios 12–13). This filter is distinguished from a field-name
filter because "authentication" is not a value of any enumerated
field the template defines.

**Scenario 16 — Ambiguous single-token filter asks for disambiguation**
Input: `/list-tickets backend` where `backend` is not a template
status, type, or priority value but could be a tag or a title
substring
Expected: Skill applies rule 5 (free-text title search) since
`backend` does not match any template-comment value. The interpreted
filter is echoed: "Filter: title contains 'backend'". If the user
wanted a tag filter, they would use `tagged backend` (rule 2).

**Scenario 17 — Explicit form matches legacy status values**
Input: `/list-tickets status todo`
Expected: Skill applies rule 2 (explicit structured form) and
filters to tickets with `status: todo`. Legacy tickets with
`status: todo` are returned. The shorthand `todo` alone would fall
through to rule 5 (title search) since `todo` is not in the
template's default status comment — the explicit form is needed.

**Scenario 18 — Unpadded parent filter matches padded values**
Input: `/list-tickets under 1`
Expected: Skill normalises the filter value `1` to `0001` and
matches tickets whose `parent` field normalises to the same value.
A ticket with `parent: "0001"` or `parent: 1` is included.

**Scenario 19 — Parent cycle in hierarchy mode**
Input: `/list-tickets hierarchy` against a tickets directory where
ticket A has `parent: "B"` and ticket B has `parent: "A"` (a
two-node cycle)
Expected: The skill detects the cycle and terminates in bounded
time. Both cyclic tickets are rendered at the top level with a
`(cycle)` marker. Non-cyclic tickets render normally. No infinite
loop, no crash.

**Scenario 20 — Non-ticket files in directory are excluded**
Input: `/list-tickets` when `{tickets_dir}` contains valid tickets
(`0001-foo.md`, `0002-bar.md`) alongside non-matching files
(`README.md`, `notes.txt`, `000-missing-digit.md`, `archive/`)
Expected: Only `0001-foo.md` and `0002-bar.md` appear in the table.
Non-matching files are silently excluded — no warnings, no errors,
no table rows. The glob matches exactly 4-digit prefixed markdown
files (`[0-9][0-9][0-9][0-9]-*.md`).

### Changes Required

#### 1. Create `skills/tickets/list-tickets/SKILL.md`

**File**: `skills/tickets/list-tickets/SKILL.md`

Invoke `/skill-creator:skill-creator` with this specification:

```
Skill name: list-tickets
Category: tickets
Model after: The discovery step of skills/decisions/review-adr/SKILL.md
(lines 32–70). There is no direct precedent for a full listing skill,
so the SKILL.md is authored from this specification.

Frontmatter:
  name: list-tickets
  description: >
    List and filter tickets from the configured tickets directory.
    Use when discovering what tickets exist, filtering by
    status/type/priority/parent/tag, or viewing the ticket hierarchy.
  (Plain multi-line scalar with 2-space continuation indent; no `>`
  block scalar in the actual SKILL.md — the `>` above is spec
  notation only.)
  argument-hint: "[filter description]"
  disable-model-invocation: true
  allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/tickets/scripts/*)
  (Note: `tickets/scripts/*` is needed for
  `ticket-template-field-hints.sh`.)

H1 heading (first line after frontmatter):
  # List Tickets

Configuration preamble (bang-executed in this order):
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh list-tickets`
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

Agent fallback (canonical phrasing required):
  If no "Agent Names" section appears above, use these defaults:
  accelerator:reviewer, accelerator:codebase-locator,
  accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
  accelerator:documents-locator, accelerator:documents-analyser,
  accelerator:web-search-researcher

Path injection:
  **Tickets directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tickets meta/tickets`

Template section (under a `## Ticket Template` heading with framing
paragraph: "The following template defines the ticket schema and
field defaults. Hint values for filter parsing are extracted at
runtime via `ticket-template-field-hints.sh`."):
  ## Ticket Template
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh ticket`

Instructions injection (end of file):
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh list-tickets`

Skill flow:

  Step 1 — Resolve filter
    If an argument was provided: parse it as a filter expression
    using the following precedence rules, applied in order. The first
    rule that matches wins.

    1. **Presentation keywords**: `hierarchy`, `as a tree`,
       `show hierarchy`. These change rendering, not filtering, and
       may combine with a filter (e.g. `hierarchy under 0042`).
    2. **Explicit structured forms**: `tagged <value>`,
       `with tag <value>`, `under <value>`, `children of <value>`,
       `status <value>`, `type <value>`, `priority <value>`,
       `about <text>`. The keyword identifies the field or mode
       unambiguously.
    3. **Multi-token template-value shorthand**: two or more tokens
       that each match a known template-comment value in different
       fields (e.g. `bugs in review` → `type: bug AND
       status: review`). Both tokens must match values from distinct
       fields. If either token is ambiguous, fall through to rule 5.
    4. **Single-token template-value shorthand**: one token matching
       a known type, status, or priority value from the template
       comments. If the token matches values in more than one field,
       ask the user for disambiguation rather than guessing
       (e.g. if a custom template had `review` as both a status and
       a type, the skill would ask "Did you mean status: review or
       type: review?").
    5. **Free-text title search**: anything that does not match
       rules 1–4 is treated as a case-insensitive substring search
       against the `title:` frontmatter field.

    To populate the known values for rules 3–4, call
    `ticket-template-field-hints.sh` for `type`, `status`, and
    `priority`. The script parses the template's trailing comments
    (or falls back to hardcoded shipping defaults if comments are
    absent). These hints inform rules 3–4 but do not restrict what
    values may appear on tickets. Legacy values like `todo`, `done`,
    or `adr-creation-task` are matchable via the explicit form
    (rule 2, e.g. `status todo`, `type adr-creation-task`) but are
    not template-value shorthands since they do not appear in the
    hint output.
    Always echo the interpreted filter before showing results so the
    user can rephrase if the parse was wrong.
    If no argument: filter is "all, no filter".

  Step 2 — Scan tickets directory
    Read every file in {tickets_dir} matching the glob `NNNN-*.md`.
    For each file, extract frontmatter (between `---` delimiters).
    - If the file has no frontmatter: warn "<filename>: skipped — no
      frontmatter" and continue. The file is excluded from the table.
    - If frontmatter is unclosed: warn "<filename>: skipped — unclosed
      frontmatter" and continue. The file is excluded.
    - Otherwise: parse fields and keep the ticket.
    For each kept ticket, derive a number from the filename prefix
    (`0042-foo.md` → `0042`). This is authoritative even if
    frontmatter contains a different `ticket_id`.

  Step 3 — Apply filter
    Apply the parsed filter from Step 1. For legacy tickets missing
    fields that the filter references (e.g. priority), exclude them
    from matching — they will only match filters on fields they carry.
    If the filter is "all, no filter": keep every ticket.
    **Parent normalisation**: when filtering by parent (`under X`),
    normalise both the filter value and each ticket's `parent` field
    to a zero-padded 4-digit string before comparison. Strip quotes
    and leading zeros, then re-pad to 4 digits. This ensures
    `parent: "0042"`, `parent: 42`, and `parent: 0042` all match
    `under 0042` or `under 42`.

  Step 4 — Render
    Default rendering: a markdown table with columns ID, Title, Type,
    Status, Priority. Rows sorted by number ascending. Individual
    missing fields render as `—`. If a column would be `—` for every
    row in the current result set, suppress the column entirely to
    reduce noise (e.g. a listing of only legacy tickets would omit
    the Priority column since none carry it).
    Hierarchy rendering (if requested): tickets with no `parent` at
    the top level; tickets with a `parent` pointing to a known ticket
    indented beneath it; tickets whose `parent` points to a
    nonexistent ID appear at the top level with "(parent NNNN not
    found)" suffix.
    Always echo the interpreted filter above the table, e.g. "Filter:
    status=draft AND type=story (2 matches)".
    If zero tickets match: print "No tickets matched: <filter>." with
    no table. If the active filter was rule 5 (free-text title search),
    append a discoverability hint: "Tip: to filter by field value, use
    `status <value>`, `type <value>`, or `tagged <value>`."
    If the directory is empty or missing: print "No tickets found in
    {tickets_dir}."

Quality guidelines:
  - Never write any files. This is a read-only skill.
  - Never spawn sub-agents. Filesystem reads only.
  - Never assume a specific set of status, type, or priority values —
    the template's comments list shipping defaults, not a closed set.
    Users may override the template and introduce custom values.
  - Explicit structured filters (`status <value>`, `type <value>`)
    match any value present on any ticket, not just template defaults.
    This is how legacy values like `todo` or `adr-creation-task` are
    reachable.
  - Malformed or missing frontmatter must not crash the listing —
    warn and continue. Warning messages should use the resolved
    filename and phrasing consistent with `ticket-read-field.sh`
    ("no frontmatter" / "unclosed frontmatter").
  - The filename NNNN prefix is the authoritative ticket number.
  - Hierarchy rendering must not loop infinitely if a `parent` cycle
    exists (detect and break; render cyclic entries flat with a note).

Eval scenarios: [the 20 scenarios listed above in Subphase 3.1]
```

### Success Criteria

#### Automated Verification

- [ ] Phase 1 regression suite passes: `bash skills/tickets/scripts/test-ticket-scripts.sh`
- [ ] File exists: `skills/tickets/list-tickets/SKILL.md`
- [ ] `grep "disable-model-invocation: true" skills/tickets/list-tickets/SKILL.md` matches
- [ ] `grep "accelerator:reviewer" skills/tickets/list-tickets/SKILL.md` matches
- [ ] `grep -E "allowed-tools:.*tickets/scripts/\*" skills/tickets/list-tickets/SKILL.md` matches

#### Manual Verification (via `/skill-creator` evals)

- [ ] All 20 approach evals pass
- [ ] Bare `/list-tickets` invocation produces a full table sorted by
  number
- [ ] Legacy `adr-creation-task` tickets appear in the listing
  alongside new-schema tickets
- [ ] `/list-tickets status todo` returns legacy tickets (explicit
  form for non-template values)
- [ ] Malformed/missing frontmatter warns but does not crash the
  listing
- [ ] Natural language filters ("drafts", "epics", "bugs in review")
  are interpreted correctly
- [ ] Hierarchy rendering groups children under parents; orphan
  children are flagged
- [ ] No sub-agents are spawned
- [ ] No files are written

---

## Subphase 3.2: `update-ticket` Skill

### Overview

Author `skills/tickets/update-ticket/SKILL.md` — an interactive skill
that edits frontmatter fields of an existing ticket. The user
identifies the target ticket (by number or file path) and describes
what to change. The skill shows a diff preview and waits for
confirmation before editing. No status transition logic is enforced —
the user decides what's valid.

### Approach Evals

These 36 scenarios are the "test" half of TDD. Define them before
authoring the SKILL.md and pass them as the eval specification to
`/skill-creator:skill-creator`.

**Scenario 1 — Bare invocation prompts for ticket**
Input: `/update-ticket` (no arguments)
Expected: Skill asks which ticket to update, accepting either a
number or a path. It does not read any file or write anything yet.

**Scenario 2 — Identifies ticket by number**
Input: `/update-ticket 0042`
Expected: Skill globs `{tickets_dir}/0042-*.md`, finds exactly one
match, and reads it.

**Scenario 3 — Identifies ticket by path**
Input: `/update-ticket meta/tickets/0042-add-search.md`
Expected: Skill reads the specified file directly, without globbing.

**Scenario 4 — Number does not exist**
Input: `/update-ticket 9999` when no `9999-*.md` file exists
Expected: Skill prints "No ticket numbered 9999 found in
`{tickets_dir}`." and exits. No write.

**Scenario 5 — Ambiguous number match**
Input: `/update-ticket 0042` when both `0042-foo.md` and
`0042-bar.md` exist in `{tickets_dir}` (edge case; should not occur
in practice but may if tickets are added manually)
Expected: Skill reports the ambiguity, lists the matching paths as
numbered options (e.g. `1. 0042-foo.md`, `2. 0042-bar.md`), and
asks the user to select by number or specify the full path.

**Scenario 6 — Asks what to change when no op given**
Input: `/update-ticket 0042` (after successful identification)
Expected: Skill reads the current frontmatter, shows it to the user,
and asks which field(s) to change. Valid field examples are surfaced
as hints using the ticket template's frontmatter field names.

**Scenario 7 — Structured op: status change**
Input: `/update-ticket 0042 status ready` (ticket currently has
`status: draft`)
Expected: Skill plans a single edit: frontmatter `status: draft` →
`status: ready`. Shows a diff preview and asks for confirmation.

**Scenario 8 — Structured op: priority change**
Input: `/update-ticket 0042 priority high`
Expected: Skill plans `priority: medium` → `priority: high`. Preview
and confirmation.

**Scenario 9 — Structured op: add tag preserves existing**
Input: `/update-ticket 0042 add tag backend` (ticket has
`tags: [api, search]`)
Expected: Skill calls `ticket-update-tags.sh <path> add backend` and
receives `[api, search, backend]`. Plans
`tags: [api, search]` → `tags: [api, search, backend]`. Existing
tags preserved in original order; new tag appended. No duplicates —
if `backend` is already present, the script prints `no-change` and
the skill reports "no change needed" (Scenario 20).

**Scenario 10 — Structured op: remove tag preserves others**
Input: `/update-ticket 0042 remove tag backend` (ticket has
`tags: [api, backend, search]`)
Expected: Skill calls `ticket-update-tags.sh <path> remove backend`
and receives `[api, search]`. Plans
`tags: [api, backend, search]` → `tags: [api, search]`. Only the
named tag is removed; others keep their order. If the tag is not
present, the script prints `no-change` and the skill reports "no
change needed".

**Scenario 11 — Structured op: set parent (canonicalised)**
Input: `/update-ticket 0042 parent 0001`
Expected: Skill plans `parent: ""` → `parent: "0001"`. Preview and
confirmation. The parent value is always written as a zero-padded
4-digit quoted string. If the user supplies an unpadded value
(e.g. `parent 1`), the skill normalises it to `"0001"` before
writing.

**Scenario 12 — Natural language op interpreted**
Input: `/update-ticket 0042 mark it as done`
Expected: Skill interprets as `status: done` and plans the edit.
Shows the interpretation explicitly above the diff ("Interpreted as:
status → done") so the user can correct misinterpretations before
confirming.

**Scenario 13 — Confirmation preview always shown**
Input: Any op that changes at least one field
Expected: Skill prints a unified diff of the frontmatter block
(before and after) and asks `"Apply these changes? (y/n)"`. No edit
happens until the user types `y` (or equivalent affirmation).

**Scenario 14 — User declines confirmation → no write**
Input: User types `n` at the confirmation prompt
Expected: Skill prints "No changes applied." and exits. The ticket
file is byte-for-byte unchanged (verifiable via `diff` against a
pre-image).

**Scenario 15 — No transition enforcement**
Input: `/update-ticket 0042 status done` when current status is
`draft` (skipping intermediate states like `ready`, `in-progress`,
`review`)
Expected: Skill proceeds with the change through the normal
confirmation flow. It does NOT warn about skipped states or reject
the transition. Arbitrary transitions are allowed.

**Scenario 16 — Body `**Status**:` line synced when present**
Input: A status change on a ticket whose body contains a line
`**Status**: Draft`
Expected: The Edit in Step 4 updates both the frontmatter `status:
draft` → `status: ready` AND the body `**Status**: Draft` →
`**Status**: Ready`. The first non-code-fence occurrence of
`**Status**: ` in the body is matched; additional occurrences are
left untouched. If no matching body line exists, only the frontmatter
is touched.

**Scenario 17 — Template-enumerated values surfaced as hint**
Input: `/update-ticket 0042 status` (no value specified)
Expected: Skill calls `ticket-template-field-hints.sh status` and
receives the hint values (e.g. `draft`, `ready`, `in-progress`, …).
It presents these as a hint list: "Common statuses: draft, ready,
in-progress, review, done, blocked, abandoned. What would you like
to set?". The skill accepts any value the user provides, including
values not in the hint list. If the script returns no hints (e.g.
user-overridden template with no comment for this field and an
unrecognised field name), the skill skips the hint list and simply
asks for a value.

**Scenario 18 — Arbitrary status value allowed**
Input: `/update-ticket 0042 status waiting-on-legal`
Expected: Skill proceeds with the change. Adds a single-line
informational note above the diff: "Note: 'waiting-on-legal' is not
one of the template's default statuses; proceeding anyway." No
enforcement — user confirms, skill writes.

**Scenario 19 — Legacy ticket update proceeds normally**
Input: `/update-ticket 0011` where `0011-*.md` is a legacy ticket
with `status: todo` and `type: adr-creation-task`
Expected: Skill reads the ticket, offers normal update operations,
and proceeds through the same flow as a new-schema ticket. No
migration is required or offered. The legacy `todo` status is
treated as the current value; any transition from it is allowed.

**Scenario 20 — No-op detection**
Input: `/update-ticket 0042 status draft` when current status is
already `draft`
Expected: Skill prints "No change needed: status is already 'draft'."
and exits cleanly. No diff shown. No write.

**Scenario 21 — `ticket_id` is hard-blocked; `date` is warned**
Input: `/update-ticket 0042 ticket_id 9999`
Expected: Skill prints: "Error: `ticket_id` cannot be changed — the
filename prefix is the authoritative ticket number. To renumber a
ticket, rename the file (e.g. `jj mv`) and update `ticket_id` to
match." No diff, no write, no confirmation prompt.
Input: `/update-ticket 0042 date 2027-01-01`
Expected: Skill warns: "`date` records the ticket's creation time
and is typically not edited. Proceed anyway? (y/n)". If the user
confirms, the edit proceeds through the normal diff-and-confirm flow.
If declined, "No changes applied." and exit.

**Scenario 22 — Malformed frontmatter aborts with clear error**
Input: `/update-ticket 0042` when the ticket has unclosed
frontmatter (first line is `---` but no closing `---`)
Expected: Skill prints "Error: `0042-add-search.md` has unclosed
YAML frontmatter. Add a `---` line after the last frontmatter key,
then re-run." — using the resolved filename, not the glob pattern.
Exits cleanly. No write is attempted.

**Scenario 23 — Title change syncs body H1 (new-schema prefix)**
Input: `/update-ticket 0042 title "Add search to dashboard"` on a
ticket whose body H1 is `# 0042: Old title`
Expected: The diff preview shows both the frontmatter change and the
body H1 change:
```
-title: "Old title"
+title: "Add search to dashboard"
-# 0042: Old title
+# 0042: Add search to dashboard
```
On confirmation, both lines are updated in a single coherent edit.
The `# 0042: ` prefix is preserved.

**Scenario 24 — Title change syncs body H1 (legacy prefix)**
Input: `/update-ticket 0011 title "New title"` on a legacy ticket
whose body H1 is `# ADR Ticket: Old title`
Expected: The diff preview shows the body H1 change with the
`# ADR Ticket: ` prefix preserved:
```
-# ADR Ticket: Old title
+# ADR Ticket: New title
```
No normalisation of the prefix occurs; the legacy H1 format is
respected.

**Scenario 25 — Body sync covers all four template labels**
Input: `/update-ticket 0042 type epic` on a ticket whose body
contains `**Type**: Story`
Expected: The diff preview shows both the frontmatter change
(`type: story` → `type: epic`) and the body label change
(`**Type**: Story` → `**Type**: Epic`). The same sync pattern
applies to `**Priority**:` on a priority change and `**Author**:`
on an author change.

**Scenario 26 — Body sync skips code-fence content**
Input: `/update-ticket 0042 status ready` on a ticket whose body
contains a fenced code block with `**Status**: Draft` inside it,
followed by a real `**Status**: Draft` label line outside the fence.
Expected: The sync updates only the first non-fenced `**Status**:`
occurrence. The occurrence inside the code block is not touched.

**Scenario 27 — Hyphenated status: display text conversion**
Input: `/update-ticket 0042 status waiting-on-legal`
Expected: The body sync writes `**Status**: Waiting on Legal`
(hyphens become spaces, title case applied with "on" lowercase as
a preposition). `in-progress` would produce `**Status**: In Progress`;
`in-flight` would produce `**Status**: In Flight`. The rule is
deterministic: always replace hyphens with spaces and apply title
case. Predictability across invocations is prioritised over
linguistic perfection for compound adjectives.

**Scenario 28 — Add tag to ticket with absent `tags` field**
Input: `/update-ticket 0011 add tag backend` on a legacy ticket that
has no `tags` field in frontmatter
Expected: `ticket-update-tags.sh` detects the absent field and
returns `[backend]`. The diff preview shows the new field being
added: `+tags: [backend]`. On confirmation, the field is inserted
into the frontmatter.

**Scenario 29 — Add tag to empty `tags: []`**
Input: `/update-ticket 0042 add tag backend` on a ticket with
`tags: []`
Expected: `ticket-update-tags.sh` returns `[backend]`. The diff
preview shows `tags: []` → `tags: [backend]`.

**Scenario 30 — Remove last tag yields empty array**
Input: `/update-ticket 0042 remove tag backend` on a ticket with
`tags: [backend]`
Expected: `ticket-update-tags.sh` returns `[]`. The diff preview
shows `tags: [backend]` → `tags: []`. The field is not deleted from
frontmatter.

**Scenario 31 — Block-style tags array rejected**
Input: `/update-ticket 0042 add tag backend` on a ticket whose
`tags` field uses block-style YAML:
```yaml
tags:
  - api
  - search
```
Expected: `ticket-update-tags.sh` exits 1. The skill prints:
"Error: tags field is in block format — convert to `tags: [...]`
first. Example: tags: [api, search]" No diff, no write.

**Scenario 32 — Unpadded parent value normalised on write**
Input: `/update-ticket 0042 parent 1`
Expected: Skill normalises the value to `"0001"` (zero-padded,
quoted). The diff preview shows `parent: ""` → `parent: "0001"`.
On confirmation, `parent: "0001"` is written.

**Scenario 33 — Multi-op combined diff and single confirmation**
Input: `/update-ticket 0042 status ready priority high`
Expected: Skill parses two structured ops (`status: ready` and
`priority: high`). A single combined diff is shown:
```
-status: draft
+status: ready
-priority: medium
+priority: high
```
Plus any body label syncs for both `**Status**:` and `**Priority**:`.
A single `"Apply these changes? (y/n)"` prompt is shown. On `y`,
all changes are applied together. On `n`, none are applied.

**Scenario 34 — Set absent scalar field on legacy ticket**
Input: `/update-ticket 0011 priority high` on a legacy ticket that
has no `priority:` line in frontmatter
Expected: Skill detects the field is absent, plans a field insertion
(not a replacement). The diff preview shows a pure addition:
`+priority: high`. The new field is inserted as the last line before
the closing `---` delimiter. On confirmation, the Edit inserts the
line at that position. The frontmatter structure remains valid (both
`---` delimiters intact, new field inside them).

**Scenario 35 — `date` warning declined → no further prompt**
Input: `/update-ticket 0042 date 2027-01-01` — user types `n` at
the warning prompt
Expected: Skill prints "No changes applied." and exits immediately.
No confirmation diff is shown, no further prompt is issued, no write
occurs. The ticket file is byte-for-byte unchanged.

**Scenario 36 — Partial no-op in multi-op command**
Input: `/update-ticket 0042 status review priority high` on a ticket
where `status` is already `review` but `priority` is `medium`.
Expected: Skill notes that status is already `review` (informational,
not an error). Diff preview shows only the priority change
(`-priority: medium` / `+priority: high` plus body label sync). No
status line appears in the diff. User confirms, write applies only the
priority change. The ticket's `status` field is byte-for-byte
unchanged.

### Changes Required

#### 1. Create `skills/tickets/update-ticket/SKILL.md`

**File**: `skills/tickets/update-ticket/SKILL.md`

Invoke `/skill-creator:skill-creator` with this specification:

```
Skill name: update-ticket
Category: tickets
Model after: skills/decisions/review-adr/SKILL.md — specifically its
frontmatter-edit pattern (lines 180–214). Adapt for ticket fields
rather than ADR status, and omit transition enforcement entirely.

Frontmatter:
  name: update-ticket
  description: >
    Update fields (status, priority, tags, parent, etc.) of an
    existing ticket. Use to transition status, change priority,
    manage tags, or edit any frontmatter field. No transition
    enforcement — arbitrary changes are allowed.
  (Plain multi-line scalar with 2-space continuation indent; no `>`
  block scalar in the actual SKILL.md — the `>` above is spec
  notation only.)
  argument-hint: "[ticket-ref] [field-op...]"
  disable-model-invocation: true
  allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/tickets/scripts/*)

H1 heading (first line after frontmatter):
  # Update Ticket

Configuration preamble (bang-executed in this order):
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh update-ticket`
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

Agent fallback (canonical phrasing required):
  If no "Agent Names" section appears above, use these defaults:
  accelerator:reviewer, accelerator:codebase-locator,
  accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
  accelerator:documents-locator, accelerator:documents-analyser,
  accelerator:web-search-researcher

Path injection:
  **Tickets directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tickets meta/tickets`

Template section (under a `## Ticket Template` heading with framing
paragraph: "The following template defines the ticket schema and
field defaults. Hint values are extracted at runtime via
`ticket-template-field-hints.sh`."):
  ## Ticket Template
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh ticket`

Instructions injection (end of file):
  !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh update-ticket`

Skill flow:

  Step 1 — Identify target ticket
    If a ticket reference was provided as the first argument:
      - Path-like (contains `/` or ends in `.md`): treat as a file path.
        If the file does not exist, abort with "No ticket at <path>".
      - Otherwise: treat as a ticket number. Glob
        `{tickets_dir}/NNNN-*.md` where NNNN is the argument
        zero-padded to four digits.
        - Zero matches: abort with "No ticket numbered N found in
          {tickets_dir}."
        - One match: use it.
        - Multiple matches: list them as numbered options and ask
          the user to select by number or specify the full path.
    If no argument: ask the user which ticket to update. Accept the
    response in the same two forms (number or path).

  Step 2 — Read current frontmatter
    Read the target file. Extract frontmatter between `---`
    delimiters using the same rules as ticket-read-field.sh:
      - No leading `---` line → abort with "Error: <filename> has no
        YAML frontmatter. Add a `---` line as the first line of the
        file, then re-run."
      - Unclosed frontmatter → abort with "Error: <filename> has
        unclosed YAML frontmatter. Add a `---` line after the last
        frontmatter key, then re-run."
    Parse field/value pairs. Preserve original key order for later
    diff rendering.

  Step 3 — Interpret operation
    Parse the remaining arguments (or ask the user) as one or more
    field operations. Arguments are parsed left-to-right using
    the following precedence:

    1. **Tag ops**: `add tag <value>` / `remove tag <value>` —
       delegate to `ticket-update-tags.sh <path> add|remove <value>`.
       The script handles parsing, mutation, canonical
       re-serialisation, and no-op detection.
    2. **Structured field ops**: `<field> <value>` where `<field>`
       is a known frontmatter field name from the template
       (e.g. `status ready`, `priority high`, `parent 0001`,
       `title "New title"`). The next token after the field name is
       consumed as the value. Quoted strings are treated as a single
       value.
    3. **Field-only (hint elicitation)**: a known field name as the
       **last token** with no following value triggers hint
       elicitation — call `ticket-template-field-hints.sh <field>`
       and present the returned values as examples, then wait for
       the user to provide a value.
    4. **Natural language**: anything that does not match rules 1–3
       (e.g. "mark as done", "set priority to high", "add backend
       tag") — interpret into one of the structured shapes above and
       echo the interpretation explicitly ("Interpreted as:
       <field> → <value>") for user verification before proceeding.

    **Disambiguation**: if the token sequence is ambiguous
    (e.g. `status priority high` could be `status: priority` +
    stray `high`, or `status: <missing>` + `priority: high`), ask
    the user for clarification rather than guessing. Present the
    possible interpretations and let the user choose.

    **Parent canonicalisation**: if the target field is `parent`,
    normalise the value to a zero-padded 4-digit quoted string
    before writing (e.g. `1` → `"0001"`, `42` → `"0042"`). This
    ensures consistent matching in `list-tickets` hierarchy and
    `under` filters.

    If the target field is `ticket_id`: hard-block with an error
    message pointing to file rename as the correct approach (see
    Quality Guidelines). No diff, no write.
    If the target field is `date`: surface a warning (see Quality
    Guidelines). If the user confirms, the edit proceeds normally.
    If declined, exit with "No changes applied."
    If the computed new value equals the current value for a
    single-op command: print "No change needed: <field> is already
    <value>." and exit cleanly. No diff, no write.
    If multiple ops are provided at once (e.g. "status ready
    priority high"): compute all edits together. No-op detection
    is per-field: fields already at the target value are excluded
    from the diff with an informational note ("status is already
    'ready' — skipping"). The "no change needed" exit only triggers
    when ALL requested operations are no-ops. Otherwise, present a
    combined diff of the changed fields in Step 4.

  Step 4 — Preview and confirm
    Produce a unified diff showing only the frontmatter lines that
    change, before and after. Format:
      -status: draft
      +status: ready
      -priority: medium
      +priority: high
    Body label sync: when a field that has a corresponding body label
    changes (`status` ↔ `**Status**:`, `type` ↔ `**Type**:`,
    `priority` ↔ `**Priority**:`, `author` ↔ `**Author**:`), scan
    the body for the first line outside any code fence (between
    ` ``` ` delimiters) whose text starts with `**<Label>**: ` (label
    matching is case-insensitive). If found, include the body line
    change in the diff preview. If no such line exists, skip the body
    sync for that label — do NOT inject one.
    Convert the frontmatter value to display text using this
    deterministic rule: replace hyphens with spaces and apply title
    case (capitalise each word, keeping small words lowercase unless
    they open the phrase). Small words:
    `a, an, and, as, at, but, by, for, in, nor, of, on, or, so, the, to, up, vs, yet`.
    Single words are simply capitalised (`draft` → `Draft`).
    Multi-word hyphenated values become spaced title case
    (`waiting-on-legal` → `Waiting on Legal`,
    `in-progress` → `In Progress`, `in-flight` → `In Flight`).
    Predictability across invocations is prioritised over
    linguistic perfection for compound adjectives.
    If the title is changing: include the body H1 change in the diff
    too. The H1 is the first `# ` line in the body after the
    frontmatter. Preserve any prefix before the first `: ` in the
    existing H1 (e.g. `# 0042: Old title` or `# ADR Ticket: Old title`)
    and substitute the new title after the prefix:
      -# 0042: Old title
      +# 0042: New title
    If the existing H1 has no `: ` separator, replace the full heading
    text after `# ` with the new title. Only the first H1 is touched;
    later headings are never modified.
    Print the diff and the prompt "Apply these changes? (y/n)". Wait
    for user response. Accepted affirmatives: case-insensitive
    exact `y` or `yes`. On `n` or `no`: print "No changes applied."
    and exit without writing. On any other input: re-prompt once
    ("Please confirm with 'y' or 'n'."). If the second response is
    still unrecognised: treat as decline and exit with "No changes
    applied."

  Step 5 — Write
    Apply edits in order: frontmatter changes first, then body label
    syncs. Each is a separate Edit call.
    Field insertion: when a field does not exist in the target
    ticket's frontmatter (e.g. adding `priority:` to a legacy ticket
    that has never had it), insert the new field as the last line
    before the closing `---` delimiter. Never insert outside the
    frontmatter delimiters.
    If a body sync Edit fails after the frontmatter was already
    written, print: "Warning: frontmatter updated but body sync
    failed for `**<Label>**:` — check the file manually. To revert,
    run: `jj restore <filename>`". Do not attempt to revert the
    frontmatter change.
    After writing, print a confirmation: "Updated <filename>:"
    followed by a compact summary of the changes (e.g.
    "status: draft → ready").

Quality guidelines:
  - Never write without explicit user confirmation.
  - Never enforce status transitions. Any value the user supplies is
    acceptable. A future feature may add user-configurable transition
    graphs; this skill predates that feature.
  - When surfacing field hints (e.g. for "what status?"): call
    `ticket-template-field-hints.sh <field>` (see Prerequisites P.5).
    The script returns one value per line, parsed from the template's
    trailing comment or falling back to hardcoded shipping defaults.
    Present those as examples, not as a closed list. Accept any value.
  - Hard-block `ticket_id` edits — print an error pointing to file
    rename (`jj mv`) as the correct approach. No diff, no write.
  - Warn before editing `date` (creation timestamp); allow if the
    user confirms.
  - For `tags` operations: delegate to `ticket-update-tags.sh`
    (see Prerequisites P.4). The script owns parsing, mutation, and
    canonical re-serialisation. The skill calls it, reads its stdout,
    and uses the returned value with the Edit tool. If the script
    exits 1 (block-style tags), the skill prints the stderr message
    and aborts. If the script prints `no-change`, the skill reports
    "No change needed" and exits cleanly.
  - Body label sync applies to `**Status**:`, `**Type**:`,
    `**Priority**:`, and `**Author**:` body lines. Only update the
    first non-code-fence occurrence of a matching label line — do
    NOT inject a label line into tickets that do not have one, and
    do NOT update occurrences inside code fences.
  - Abort cleanly on missing or malformed frontmatter (no `---` line,
    unclosed frontmatter). Do not attempt to repair. Error messages
    should be consistent with `ticket-read-field.sh` output: use the
    resolved filename (not a glob) and the same phrasing
    ("no YAML frontmatter" / "unclosed YAML frontmatter").
  - Legacy tickets with unusual `type` or `status` values (e.g.
    `type: adr-creation-task`, `status: todo`) are fully supported;
    no migration is offered or required.
  - If multiple tickets match a number glob: abort with guidance, do
    not silently pick one.
  - Never rename the file. `ticket_id` edits are hard-blocked; point
    the user to `jj mv` + manual frontmatter edit for renumbering.

Eval scenarios: [the 36 scenarios listed above in Subphase 3.2]
```

### Success Criteria

#### Automated Verification

- [ ] Phase 1 regression suite passes: `bash skills/tickets/scripts/test-ticket-scripts.sh`
- [ ] File exists: `skills/tickets/update-ticket/SKILL.md`
- [ ] `grep "disable-model-invocation: true" skills/tickets/update-ticket/SKILL.md` matches
- [ ] `grep "accelerator:reviewer" skills/tickets/update-ticket/SKILL.md` matches
- [ ] `grep -E "allowed-tools:.*tickets/scripts/\*" skills/tickets/update-ticket/SKILL.md` matches
- [ ] No transition-graph assertion anywhere in the SKILL.md:
  `grep -iE "transition|draft.*ready|invalid.*status" skills/tickets/update-ticket/SKILL.md`
  returns nothing suggesting enforcement (manual verification against
  remaining matches)

#### Manual Verification (via `/skill-creator` evals)

- [ ] All 35 approach evals pass
- [ ] Target identification works by number, by path, and handles
  ambiguity and missing files cleanly
- [ ] Confirmation preview is always shown before writing; declining
  results in an unchanged file
- [ ] Body `**Status**:` line is updated in sync with frontmatter
  when present; not injected when absent
- [ ] Arbitrary status values (including custom strings like
  `waiting-on-legal`) are accepted without enforcement
- [ ] Legacy tickets (status `todo`, type `adr-creation-task`) are
  updatable without migration
- [ ] `ticket_id` edits are hard-blocked with error message
- [ ] `date` edits warn but allow on confirmation; decline exits
  cleanly
- [ ] No-op updates (same value as current) exit cleanly without
  writing
- [ ] Tag add/remove preserves existing tags and order; delegated
  to `ticket-update-tags.sh`
- [ ] Tag edge cases handled: absent field, empty `[]`,
  last-removal → `[]`, block-style rejection
- [ ] Natural language ops ("mark as done") are echoed as
  "Interpreted as: <field> → <value>" before the diff
- [ ] Multiple simultaneous ops ("status ready priority high")
  render as a combined diff

---

## Subphase 3.3: Integration Verification

### Overview

Confirm both skills integrate cleanly with the Phase 1 foundation and
Phase 2 skills, load correctly via the plugin, and are callable
without errors.

### Changes Required

None — verification only.

### Verification Steps

1. Run Phase 1 regression suite: `bash skills/tickets/scripts/test-ticket-scripts.sh`
2. Confirm `allowed-tools` is inline (no block scalar) in both new
   SKILL.md files
3. Confirm both SKILL.md files contain `disable-model-invocation: true`
4. Confirm agent fallback block uses the `accelerator:` prefix in both
   files
5. Confirm plugin registration already covers both — `"./skills/tickets/"`
   in `plugin.json` scans all subdirectories, so no further changes
   are needed
6. Invoke each skill bare in a Claude Code session and confirm it
   prompts correctly without errors
7. Run a happy-path flow: `/list-tickets`, then `/update-ticket
   0001 priority high` on one of the 29 legacy tickets, to confirm
   legacy compatibility end-to-end

### Success Criteria

#### Automated Verification

- [ ] `bash skills/tickets/scripts/test-ticket-scripts.sh` exits 0, "All tests passed!"
- [ ] `grep "disable-model-invocation: true" skills/tickets/create-ticket/SKILL.md` matches
- [ ] `grep "disable-model-invocation: true" skills/tickets/extract-tickets/SKILL.md` matches
- [ ] `grep "disable-model-invocation: true" skills/tickets/list-tickets/SKILL.md` matches
- [ ] `grep "disable-model-invocation: true" skills/tickets/update-ticket/SKILL.md` matches
- [ ] `grep "allowed-tools" skills/tickets/list-tickets/SKILL.md` shows inline `config-*` and `tickets/scripts/*` (no block scalar)
- [ ] `grep "allowed-tools" skills/tickets/update-ticket/SKILL.md` shows inline `config-*` and `tickets/scripts/*` (no block scalar)
- [ ] `grep "accelerator:reviewer" skills/tickets/list-tickets/SKILL.md` matches
- [ ] `grep "accelerator:reviewer" skills/tickets/update-ticket/SKILL.md` matches

#### Manual Verification

- [ ] `/list-tickets` is available in a Claude Code session and
  produces a full table when invoked with no arguments
- [ ] `/update-ticket` is available and prompts for a ticket
  reference when invoked with no arguments
- [ ] No unresolved `{agent placeholder}` tokens visible in either
  skill
- [ ] End-to-end: `/list-tickets only drafts` returns zero rows
  against the current 29 legacy tickets (all are `todo` or `done`);
  `/list-tickets status todo` returns the legacy tickets with
  `status: todo` (explicit form for non-template values);
  `/update-ticket 0011 priority high` on a legacy ticket succeeds and
  a subsequent `/list-tickets high priority` includes it

---

## Testing Strategy

### TDD Order Per Subphase

1. Approach evals in this plan are the specification — written
   before implementation
2. `/skill-creator:skill-creator` authors the SKILL.md from the
   specification and evals
3. Built-in evals run against the authored SKILL.md
4. Iterate on the SKILL.md until all evals pass
5. Run `bash skills/tickets/scripts/test-ticket-scripts.sh` as a
   regression guard before marking the subphase done

### What Cannot Be Automated

SKILL.md files are markdown prompts, not executable code. The
`/skill-creator:skill-creator` eval runner is the verification
mechanism. There is no CI-runnable test for SKILL.md prompt
correctness. The `allowed-tools` boundary (whether the skill actually
avoids calling scripts outside its declared scope) must be verified
manually during the integration step by running each skill through a
happy-path flow and confirming no "tool not permitted" errors occur.

## References

- Research: `meta/research/2026-04-08-ticket-management-skills.md`
  (§5.6 list-tickets, §5.7 update-ticket, §6 template, §10 phasing)
- Phase 1 foundation plan:
  `meta/plans/2026-04-08-ticket-management-phase-1-foundation.md`
- Phase 2 plan (TDD + skill-creator pattern):
  `meta/plans/2026-04-19-ticket-creation-skills.md`
- Pattern for frontmatter edit:
  `skills/decisions/review-adr/SKILL.md` (lines 180–214)
- Pattern for discovery scan:
  `skills/decisions/review-adr/SKILL.md` (lines 32–70)
- Phase 1 scripts (consumed read-only):
  `skills/tickets/scripts/ticket-read-field.sh`,
  `skills/tickets/scripts/ticket-read-status.sh`
- Phase 1 regression suite:
  `skills/tickets/scripts/test-ticket-scripts.sh`
- Ticket template (source of shipping default field values):
  `templates/ticket.md`
