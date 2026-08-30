---
name: update-work-item
description: Update fields (status, priority, tags, parent, etc.) of an
  existing work item. Use to transition status, change priority, manage tags,
  or edit any frontmatter field. No transition enforcement — arbitrary
  changes are allowed.
argument-hint: "[work-item-ref] [field-op...]"
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)
  - Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator work *)
  - Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator corpus frontmatter validate *)
---

# Update Work Item

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config context --skill update-work-item --fail-safe`
!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config agents --fail-safe`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

**Work items directory**: !`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config path work --fail-safe`

## Work Item Template

The following template defines the work item schema and field defaults.
Hint values are extracted at runtime via `work template-hints`.

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config template work-item --fail-safe`

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
${CLAUDE_PLUGIN_ROOT}/bin/accelerator work resolve <argument>
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

Each recognised operation is recorded as a pending `work update` flag
(`--set`, `--add-tag`, `--remove-tag`, `--append`, or `--remove`) rather
than applied immediately — Step 4 previews the combined effect, and
Step 5 issues the single `work update` call that actually writes.

Arguments are parsed left-to-right using these rules:

### 3.1 Tag operations
`add tag <value>` / `remove tag <value>` — record a pending
`--add-tag <value>` / `--remove-tag <value>` flag. Preview the effect
against the current `tags` list already read in Step 2 (add: append if
absent; remove: drop if present). The "No-op detection" rule below
applies the same way as for any other field.

### 3.2 List-field operations
`add <field> <value>` / `remove <field> <value>` where `<field>` is one
of `blocks`, `blocked_by`, `derived_from`, `relates_to` — record a
pending `--append <field>=<value>` / `--remove <field>=<value>` flag.
Preview against the current list already read in Step 2, the same way
as tag operations.

### 3.3 Structured field operations
`<field> <value>` where `<field>` is a known **scalar** frontmatter
field name from the template (e.g. `status ready`, `priority high`,
`parent 0001`, `title "New title"`) — record a pending
`--set <field>=<value>` flag. The next token after the field name is
consumed as the value. Quoted strings are treated as a single value.

### 3.4 Field-only hint elicitation
A known field name as the **last token** with no following value
triggers hint elicitation. Call `work template-hints`:

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator work template-hints <field>
```

Present the returned values as examples: "Common statuses: draft,
ready, in-progress, review, done, blocked, abandoned. What would you
like to set?" Accept any value the user provides, including values not
in the hint list. If the command returns no hints, skip the hint list
and simply ask for a value.

### 3.5 Natural language
Anything that does not match rules 3.1–3.4 (e.g. "mark as done",
"set priority to high", "add backend tag") — interpret into one of the
structured shapes above and echo the interpretation explicitly:
`"Interpreted as: status → done"` so the user can correct before
confirming.

### Disambiguation
If the token sequence is ambiguous, ask the user for clarification
rather than guessing. Present the possible interpretations and let the
user choose.

### Special field rules

**`id` (own-identity) — hard-blocked**: recognise an attempted edit to
`id` (or the legacy `work_item_id` alias) before gathering a diff
preview. Run `accelerator work update <path> --set id=<value>`
directly and print its stderr verbatim — the CLI rejects it with the
own-identity error, pointing to file rename (`jj mv`) as the correct
approach. No diff, no confirmation prompt.

**`date` — warned**: inform the user that `date` records the work item's
creation time and is typically not edited, then use the `AskUserQuestion` tool
with two options:

1. **Yes, proceed anyway** — proceed through the normal diff-and-confirm flow
2. **No, cancel** — print "No changes applied." and exit

**`parent` — canonicalised**: normalise the value via

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator work canonicalise-id <input>
```

before writing. The canonicaliser produces the full ID under the
configured pattern, quoted as a string. Examples:

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
- `kind` ↔ `**Kind**: `
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

Print the diff, then use the `AskUserQuestion` tool with two options:

1. **Yes, apply changes** — write the changes to disk
2. **No, cancel** — print "No changes applied." and exit without writing

### Field insertion preview

When a field does not exist in the target work item's frontmatter (e.g.
adding `priority:` to a legacy work item), the diff preview shows a pure
addition: `+priority: high`.

## Step 5: Write

1. **Frontmatter changes first** — issue one call assembling every
   pending operation accumulated in Step 3:

   ```
   ${CLAUDE_PLUGIN_ROOT}/bin/accelerator work update <path> \
     --set <field>=<value> ... \
     --add-tag <value> ... --remove-tag <value> ... \
     --append <field>=<value> ... --remove <field>=<value> ...
   ```

   Omit any flag category with no pending operations. A non-zero exit
   means nothing was written (the CLI validates every key before
   applying any of them) — print the stderr message and exit.
2. **Body label syncs second** — update the matching body lines using
   the Edit tool. `work update` only ever touches frontmatter.
3. **Title H1 sync** — if the title changed, update the body H1 using
   the Edit tool.

If a body sync Edit fails after frontmatter was already written, print:
`"Warning: frontmatter updated but body sync failed for **<Label>**: —
check the file manually. To revert, run: jj restore <filename>"`. Do
not attempt to revert the frontmatter change.

After writing, print a confirmation:
```
Updated <filename>:
  status: draft → ready
```

**Validate the frontmatter**: after writing, run

```bash
${CLAUDE_PLUGIN_ROOT}/bin/accelerator corpus frontmatter validate --file <filename>
```

If it exits non-zero, the document violates the canonical frontmatter
standard; report the emitted violation and fix the frontmatter before
completing.

## Quality Guidelines

- **Confirmation required**: never write without explicit user
  confirmation via the y/n prompt.
- **No transition enforcement**: any status value the user supplies is
  acceptable. Arbitrary transitions (draft → done, skipping
  intermediate states) are allowed without warning.
- **Hint values are suggestions, not constraints**: when surfacing
  field hints via `work template-hints`, present them as examples.
  Accept any value the user provides.
- **Own-identity is immutable**: hard-block edits to `id` (or to
  `work_item_id` on legacy files) before gathering a diff preview, and
  let `work update`'s own exit-1 error surface directly.
- **`date` is guarded**: warn before editing the creation timestamp;
  allow if the user confirms.
- **All frontmatter mutation via `work update`**: scalar fields,
  tags, and typed-linkage lists are all applied in the single Step 5
  `work update` call, which owns parsing, mutation, and canonical
  re-serialisation. A non-zero exit means nothing was written — print
  stderr and exit.
- **Body label sync scope**: only update the first non-code-fence
  occurrence of `**Status**:`, `**Kind**:`, `**Priority**:`, or
  `**Author**:`. Do not inject labels into work items that lack them.
  Do not update occurrences inside code fences.
- **Resilient to malformed frontmatter**: abort cleanly on missing or
  unclosed frontmatter. Error messages use the resolved filename and
  match `work show`'s phrasing.
- **Legacy work items supported**: work items with unusual kind or status
  values (e.g. `kind: adr-creation-task`, `status: todo`) are fully
  updatable. No migration is offered or required.
- **Ambiguous globs**: if multiple work items match a number glob, list
  them and ask the user to choose. Never silently pick one.
- **No file renaming**: own-identity edits (`id`, or `work_item_id` on
  legacy files) are hard-blocked. Point the user to `jj mv` + manual
  frontmatter edit for renumbering.

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config instructions update-work-item --fail-safe`
