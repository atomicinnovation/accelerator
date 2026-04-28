---
name: update-work-item
description: Update fields (status, priority, tags, parent, etc.) of an
  existing work item. Use to transition status, change priority, manage tags,
  or edit any frontmatter field. No transition enforcement — arbitrary
  changes are allowed.
argument-hint: "[work-item-ref] [field-op...]"
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/*)
---

# Update Work Item

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh update-work-item`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

**Work items directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh work meta/work`

## Work Item Template

The following template defines the work item schema and field defaults.
Hint values are extracted at runtime via `work-item-template-field-hints.sh`.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh work-item`

You are tasked with updating frontmatter fields on an existing work item.
This skill supports status transitions, priority changes, tag management,
parent assignment, title changes, and any other frontmatter field edit.
No status transition logic is enforced — the user decides what's valid.
A future feature may add user-configurable transition graphs; this skill
predates that feature.

## Step 1: Identify Target Work Item

Parse the first argument and resolve via the configured pattern's
resolver:

```
${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-resolve-id.sh <argument>
```

The resolver respects `work.id_pattern` and accepts paths, full IDs
(`PROJ-0042`), and bare numbers (legacy or pattern-shape).

- **Exit 0**: stdout is the absolute path; use it.
- **Exit 1**: input was unrecognised. Print
  `"Unrecognised input '<argument>' — pass a path, a full ID, or a
  bare number."` and exit.
- **Exit 2**: ambiguous match. The resolver lists every candidate with
  a source-category tag. Show the list and ask the user to
  disambiguate by re-running with a full ID or path.
- **Exit 3**: no match. Print `"No work item matching <argument>."` and
  exit.
- **No argument**: ask the user which work item to update. Accept the
  response and run the resolver against it.

## Step 2: Read Current Frontmatter

Read the target file. Extract frontmatter between `---` delimiters.

- If the first line is not `---`: print `"Error: <filename> has no YAML
  frontmatter. Add a '---' line as the first line of the file, then
  re-run."` and exit. Use the resolved filename, not a glob pattern.
- If `---` opens but never closes: print `"Error: <filename> has
  unclosed YAML frontmatter. Add a '---' line after the last
  frontmatter key, then re-run."` and exit.

Parse field/value pairs from the frontmatter. Preserve the original key
order for diff rendering in Step 4.

## Step 3: Interpret Operation

Parse the remaining arguments (after the work item reference) as one or
more field operations. If no operation arguments were provided, show the
current frontmatter and ask which field(s) to change.

Arguments are parsed left-to-right using these rules:

### 3.1 Tag operations
`add tag <value>` / `remove tag <value>` — delegate to
`work-item-update-tags.sh`:

```
${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-update-tags.sh <path> add|remove <value>
```

The script handles parsing, mutation, canonical re-serialisation, and
no-op detection. If it exits 1 (e.g. block-style tags), print the
stderr message and exit without writing. If it prints `no-change`,
report "No change needed" and exit cleanly.

### 3.2 Structured field operations
`<field> <value>` where `<field>` is a known frontmatter field name
from the template (e.g. `status ready`, `priority high`, `parent 0001`,
`title "New title"`). The next token after the field name is consumed
as the value. Quoted strings are treated as a single value.

### 3.3 Field-only hint elicitation
A known field name as the **last token** with no following value
triggers hint elicitation. Call `work-item-template-field-hints.sh`:

```
${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-template-field-hints.sh <field>
```

Present the returned values as examples: "Common statuses: draft,
ready, in-progress, review, done, blocked, abandoned. What would you
like to set?" Accept any value the user provides, including values not
in the hint list. If the script returns no hints, skip the hint list
and simply ask for a value.

### 3.4 Natural language
Anything that does not match rules 3.1–3.3 (e.g. "mark as done",
"set priority to high", "add backend tag") — interpret into one of the
structured shapes above and echo the interpretation explicitly:
`"Interpreted as: status → done"` so the user can correct before
confirming.

### Disambiguation
If the token sequence is ambiguous, ask the user for clarification
rather than guessing. Present the possible interpretations and let the
user choose.

### Special field rules

**`work_item_id` — hard-blocked**: print `"Error: work_item_id cannot be
changed — the filename prefix is the authoritative work item ID. To
renumber a work item, rename the file (e.g. jj mv) and update
work_item_id to match. The work_item_id field is always a quoted
string."` No diff, no write, no confirmation prompt.

**`date` — warned**: print `"date records the work item's creation time and
is typically not edited. Proceed anyway? (y/n)"`. If the user confirms,
proceed through the normal diff-and-confirm flow. If declined, print
"No changes applied." and exit.

**`parent` — canonicalised**: normalise the value via
`work-item-common.sh:wip_canonicalise_id` before writing. The
canonicaliser produces the full ID under the configured pattern,
quoted as a string. Examples:

- Default `{number:04d}`: `1` → `"0001"`, `42` → `"0042"`.
- `{project}-{number:04d}` with `default_project_code: PROJ`:
  `1` → `"PROJ-0001"`, `PROJ-0042` → `"PROJ-0042"`,
  `0042` (legacy) → `"PROJ-0042"` (canonical form).

### No-op detection
If the computed new value equals the current value:
- **Single-op**: print `"No change needed: <field> is already
  '<value>'."` and exit. No diff, no write.
- **Multi-op**: note the no-op field informally (`"status is already
  'ready' — skipping"`), exclude it from the diff, and continue with
  the remaining changes. If ALL operations are no-ops, print the
  combined "no change needed" message and exit.

### Non-template values
If a value is not one of the template's default hints, add a
single-line informational note above the diff: `"Note: '<value>' is not
one of the template's default statuses; proceeding anyway."` No
enforcement — the user confirms, the skill writes.

## Step 4: Preview and Confirm

Produce a diff showing only the lines that change, before and after:
```
-status: draft
+status: ready
-priority: medium
+priority: high
```

### Body label sync

When a field that has a corresponding body label changes, scan the body
for the first line outside any code fence (between ` ``` ` delimiters)
whose text starts with the matching label. If found, include the body
line change in the diff preview. If no such line exists, do not inject
one.

Field-to-label mapping:
- `status` ↔ `**Status**: `
- `type` ↔ `**Type**: `
- `priority` ↔ `**Priority**: `
- `author` ↔ `**Author**: `

Convert the frontmatter value to display text using this deterministic
rule: replace hyphens with spaces and apply title case (capitalise each
word, keeping small words lowercase unless they open the phrase). Small
words: `a, an, and, as, at, but, by, for, in, nor, of, on, or, so,
the, to, up, vs, yet`. Single words are simply capitalised
(`draft` → `Draft`). Multi-word hyphenated values become spaced title
case (`waiting-on-legal` → `Waiting on Legal`,
`in-progress` → `In Progress`).

### Title sync

If the title is changing, include the body H1 change in the diff too.
The H1 is the first `# ` line in the body after the frontmatter.
Preserve any prefix before the first `: ` in the existing H1 (e.g.
`# 0042: Old title` or `# ADR Work item: Old title`) and substitute
the new title after the prefix:
```
-# 0042: Old title
+# 0042: New title
```
If the existing H1 has no `: ` separator, replace the full heading text
after `# ` with the new title. Only the first H1 is touched.

### Confirmation prompt

Print the diff and `"Apply these changes? (y/n)"`. Wait for user
response. Accepted affirmatives: case-insensitive `y` or `yes`. On `n`
or `no`: print "No changes applied." and exit without writing. On any
other input: re-prompt once. If the second response is still
unrecognised, treat as decline.

### Field insertion preview

When a field does not exist in the target work item's frontmatter (e.g.
adding `priority:` to a legacy work item), the diff preview shows a pure
addition: `+priority: high`.

## Step 5: Write

Apply edits using the Edit tool:

1. **Frontmatter changes first** — each field change is a separate Edit
   call. For field insertion (field absent from frontmatter), insert
   the new field as the last line before the closing `---` delimiter.
2. **Body label syncs second** — update the matching body lines.
3. **Title H1 sync** — if the title changed, update the body H1.

If a body sync Edit fails after frontmatter was already written, print:
`"Warning: frontmatter updated but body sync failed for **<Label>**: —
check the file manually. To revert, run: jj restore <filename>"`. Do
not attempt to revert the frontmatter change.

After writing, print a confirmation:
```
Updated <filename>:
  status: draft → ready
```

## Quality Guidelines

- **Confirmation required**: never write without explicit user
  confirmation via the y/n prompt.
- **No transition enforcement**: any status value the user supplies is
  acceptable. Arbitrary transitions (draft → done, skipping
  intermediate states) are allowed without warning.
- **Hint values are suggestions, not constraints**: when surfacing
  field hints via `work-item-template-field-hints.sh`, present them as
  examples. Accept any value the user provides.
- **`work_item_id` is immutable**: hard-block edits with an error pointing
  to file rename (`jj mv`) as the correct approach.
- **`date` is guarded**: warn before editing the creation timestamp;
  allow if the user confirms.
- **Tags via script**: delegate all tag operations to
  `work-item-update-tags.sh`. The script owns parsing, mutation, and
  canonical re-serialisation. If the script exits 1 (block-style),
  print stderr and exit. If it prints `no-change`, report "No change
  needed" and exit.
- **Body label sync scope**: only update the first non-code-fence
  occurrence of `**Status**:`, `**Type**:`, `**Priority**:`, or
  `**Author**:`. Do not inject labels into work items that lack them.
  Do not update occurrences inside code fences.
- **Resilient to malformed frontmatter**: abort cleanly on missing or
  unclosed frontmatter. Error messages use the resolved filename and
  match `work-item-read-field.sh` phrasing.
- **Legacy work items supported**: work items with unusual type or status
  values (e.g. `type: adr-creation-task`, `status: todo`) are fully
  updatable. No migration is offered or required.
- **Ambiguous globs**: if multiple work items match a number glob, list
  them and ask the user to choose. Never silently pick one.
- **No file renaming**: `work_item_id` edits are hard-blocked. Point the
  user to `jj mv` + manual frontmatter edit for renumbering.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh update-work-item`
