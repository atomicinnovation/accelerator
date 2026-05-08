---
name: list-work-items
description: List and filter work items from the configured work directory.
  Use when discovering what work items exist, filtering by
  status/type/priority/parent/tag, or viewing the work item hierarchy.
argument-hint: "[filter description]"
disable-model-invocation: false
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/*)
---

# List Work Items

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh list-work-items`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

**Work items directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh work`
**Work item ID pattern**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-value.sh work.id_pattern "{number:04d}"`
**Default project code**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-value.sh work.default_project_code ""`

## Work Item Template

The following template defines the work item schema and field defaults.
Hint values for filter parsing are extracted at runtime via
`work-item-template-field-hints.sh`.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh work-item`

You are tasked with listing and filtering work items from the configured
work items directory. This is a **read-only** skill — never write any files
and never spawn sub-agents. The entire flow uses filesystem reads and the
companion scripts listed in `allowed-tools`.

## Step 1: Resolve Filter

If an argument was provided, parse it as a filter expression using the
following precedence rules. The first rule that matches wins.

Before applying rules 3–4, call `work-item-template-field-hints.sh` for
each of `type`, `status`, and `priority` to populate the known
template-comment values:

```
${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-template-field-hints.sh type
${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-template-field-hints.sh status
${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-template-field-hints.sh priority
```

Each call outputs one value per line. Collect these into three sets:
known types, known statuses, and known priorities. These hints inform
the shorthand rules below but do not restrict what values may appear on
work items — legacy values like `todo`, `done`, or `adr-creation-task` are
matchable only via the explicit structured form (rule 2).

### Filter Precedence Rules

**Rule 1 — Presentation keywords** (changes rendering, not filtering):
`hierarchy`, `as a tree`, `show hierarchy`. These may combine with a
filter (e.g. `hierarchy under 0042`). Strip the keyword from the
argument and continue parsing the remainder, if any, through rules 2–5.

**Rule 2 — Explicit structured forms** (keyword identifies the field):
- `tagged <value>` or `with tag <value>` → filter by tag
- `under <value>` or `children of <value>` → filter by parent
- `status <value>` → filter by status (matches any value on any work item)
- `type <value>` → filter by type (matches any value on any work item)
- `priority <value>` → filter by priority
- `about <text>` → free-text title search (case-insensitive substring)

**Rule 3 — Multi-token template-value shorthand**: two or more tokens
that each match a known template-comment value in different fields.
For example, `bugs in review` → `type: bug AND status: review` (after
singularising `bugs` to `bug` and recognising `in-progress`, `review`,
etc. as status values with filler words like `in`, `only`, `all`
stripped). Both tokens must match values from distinct fields. If either
token is ambiguous across fields, fall through to rule 5.

**Rule 4 — Single-token template-value shorthand**: one token matching a
known type, status, or priority value from the template comments. Map
common plurals (`bugs`→`bug`, `epics`→`epic`, `stories`→`story`,
`tasks`→`task`, `spikes`→`spike`) and common synonyms
(`drafts`→`draft`). If the token matches values in more than one field,
ask the user for disambiguation rather than guessing.

**Rule 5 — Free-text title search**: anything that does not match rules
1–4 is treated as a case-insensitive substring search against the
`title:` frontmatter field.

**Always echo the interpreted filter** before showing results so the
user can rephrase if the parse was wrong. Example: `Filter: status=draft
(3 matches)`.

If no argument was provided: filter is "all work items, no filter".

## Step 2: Scan Work Items Directory

1. Check that `{work_dir}` exists. If not, print:
   ```
   Work items directory `{work_dir}` not found.
   Check the `paths.work` configuration or run `/create-work-item` to
   create the first work item.
   ```
   and exit cleanly.

2. List all `*.md` files in `{work_dir}`. The discovery glob is broadened
   from a literal numeric prefix to `*.md` because the work-item ID
   pattern is configurable (`work.id_pattern`) and legacy `NNNN-*.md`
   files may coexist with project-coded files (`PROJ-NNNN-*.md`) during
   a pattern transition. A file is treated as a work item iff
   `work-item-common.sh:wip_is_work_item_file` returns success — that is,
   the file has YAML frontmatter and a non-empty `work_item_id` field.
   Files lacking either are silently excluded; files with malformed
   frontmatter emit a one-line warning to stderr and are skipped.

3. If no work-item files exist after filtering, print:
   ```
   No work items found in `{work_dir}`.
   ```
   and exit cleanly.

4. **Extract frontmatter from all work items in a single pass.** Work item
   directories can contain dozens of files, so reading each one
   individually would be too slow. Instead, use a single Bash command
   to extract the frontmatter fields from every matching file at once.

   Example approach — run one `awk` command across all `*.md` files:
   ```bash
   for f in {work_dir}/*.md; do
     # Filter to true work items via wip_is_work_item_file
     if ! bash -c "source ${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-common.sh && wip_is_work_item_file '$f'"; then
       continue
     fi
     awk -v file="$f" '
       NR==1 && /^---[[:space:]]*$/ { in_fm=1; next }
       NR==1 { print file "\tERROR\tno frontmatter"; exit }
       in_fm && /^---[[:space:]]*$/ { closed=1; exit }
       in_fm { print file "\t" $0 }
       END { if (in_fm && !closed) print file "\tERROR\tunclosed frontmatter" }
     ' "$f"
   done
   ```

   This outputs one line per frontmatter field per file (tab-delimited:
   filepath, field line) plus ERROR lines for malformed files. Parse the
   output to build the work item list in memory.

   - Lines containing `ERROR	no frontmatter`: warn
     `"<filename>: skipped — no frontmatter"` and exclude the file.
   - Lines containing `ERROR	unclosed frontmatter`: warn
     `"<filename>: skipped — unclosed frontmatter"` and exclude.
   - For each valid file, derive the work item ID from the filename via
     `wip_extract_id_from_filename`. The compiled scan regex respects
     the configured `work.id_pattern`; a legacy `0042-foo.md` file
     under a `{project}-{number:04d}` pattern is matched by a
     legacy-fallback path so the file remains visible in the listing.
     The filename prefix remains the authoritative work item ID, even
     if `work_item_id` in frontmatter differs.

5. **Mixed-prefix discoverability hint**: when the listing detects both
   files matching the legacy `[0-9]{4}-` shape AND files matching the
   configured `{project}` pattern in the same corpus, prepend a single
   informational line to the output before the table:

   ```
   note: mixed prefix corpus detected — N legacy items, M project-prefixed
   items. Run /accelerator:migrate to normalise.
   ```

   The note appears once per invocation (not per file) and is suppressed
   when the configured pattern lacks `{project}`.

5. From the extracted frontmatter lines, parse these fields for each
   work item (all optional — missing fields are recorded as absent, not
   as errors):
   - `title` — the human-readable title
   - `type` — the work item type
   - `status` — the current status
   - `priority` — the priority level
   - `tags` — a YAML inline array (e.g. `[backend, api]`)
   - `parent` — the parent work item number

## Step 3: Apply Filter

Apply the parsed filter from Step 1 to the scanned work items.

- **"All, no filter"**: keep every work item.
- **Status/type/priority filter**: match the field value exactly
  (case-sensitive, matching the raw frontmatter value). Work items missing
  the filtered field are excluded from the result (not errors).
- **Tag filter**: parse the raw `tags` value (e.g. `[backend, api]`)
  into individual tag strings. A work item matches if any tag equals the
  filter value. Work items with `tags: []`, empty `tags:`, or absent `tags`
  field do not match (and are not errors).
- **Parent filter** (`under X`): normalise both the filter value and
  each work item's `parent` field via
  `work-item-common.sh:wip_canonicalise_id` before comparison. The
  canonicaliser strips quotes, accepts short and long forms, and
  zero-pads to the configured pattern's width (or pre-pends the
  default project code when the pattern requires `{project}` and the
  input is a bare number). So under default config `parent: "0042"`,
  `parent: 0042`, `parent: 42`, and `parent: "42"` all match
  `under 0042` or `under 42`. Under `{project}-{number:04d}` config
  with `default_project_code: PROJ`, `parent: "PROJ-0042"`,
  `parent: "0042"` (legacy), and `parent: 42` all canonicalise to
  `PROJ-0042` and match `under PROJ-0042` or `under 42`.
- **Free-text title search** (`about X` or rule 5): case-insensitive
  substring match against the `title:` frontmatter value. Work items
  without a `title` field are excluded.
- **Combined filters** (rule 3): all conditions must hold (AND).

## Step 4: Render

### Default Rendering (table)

Present the filtered work items as a markdown table with these columns:

| ID | Title | Type | Status | Priority |

- Sort rows by work item number ascending.
- Render missing fields as `—`.
- If a column would be `—` for every row in the current result set,
  suppress that column entirely to reduce noise. For example, a listing
  of only legacy work items (which lack `priority`) would omit the Priority
  column.

### Hierarchy Rendering

If a hierarchy presentation keyword was detected in Step 1:

- Work items with no `parent` (or empty `parent`) appear at the top level.
- Work items whose `parent` points to a work item in the current result
  set are rendered as children. Each parent→children group prints
  as a tree using Unicode box-drawing characters. Children use
  `├── ` for all entries except the last in the group, which uses
  `└── `. Indent two spaces per depth level. Example:

<!-- canonical-tree-fence -->
NNNN — parent title (type: <type>, status: <status>)
  ├── NNNN — child 1 title (type: <type>, status: <status>)
  ├── NNNN — child 2 title (type: <type>, status: <status>)
  └── NNNN — last child title (type: <type>, status: <status>)
<!-- /canonical-tree-fence -->

  No ASCII fallback is attempted; terminals without Unicode
  support will render mojibake. Users on such terminals can
  re-display the hierarchy via /list-work-items.
- Work items whose `parent` points to a work item number that does not exist
  in the result set appear at the top level with a suffix:
  `(parent NNNN not found)`.
- **Cycle detection**: before rendering, walk the parent chain for each
  work item. If a work item is visited twice during a walk, it is part of a
  cycle. Render all cyclic work items at the top level with a `(cycle)`
  marker. This ensures bounded execution — no infinite loops.

### Empty Results

- If zero work items match the filter, print:
  ```
  No work items matched: <filter description>.
  ```
  Do not render an empty table. If the active filter was a free-text
  title search (rule 5), append:
  ```
  Tip: to filter by field value, use `status <value>`, `type <value>`,
  or `tagged <value>`.
  ```

### Filter Echo

Always print the interpreted filter and match count above the table:
```
Filter: status=draft (3 matches)
```
or for a parent filter:
```
Children of 0042 (2 matches)
```
or for no filter:
```
All work items (29 total)
```

## Quality Guidelines

- **Read-only**: never write any files. This skill only reads and
  displays.
- **No sub-agents**: never spawn sub-agents. All work is done via
  filesystem reads and companion scripts.
- **No hardcoded field values**: never assume a specific set of status,
  type, or priority values. The template's comments list shipping
  defaults, not a closed set. Users may override the template with
  custom values.
- **Explicit structured filters are universal**: `status <value>`,
  `type <value>`, etc. match any value present on any work item, not just
  template defaults. This is how legacy values like `todo` or
  `adr-creation-task` are reachable.
- **Resilient to malformed work items**: missing or unclosed frontmatter
  must not crash the listing — warn using the resolved filename and
  continue. Warning phrasing should match `work-item-read-field.sh`:
  "no frontmatter" / "unclosed frontmatter".
- **Filename is authoritative**: the ID extracted from the filename
  (via `wip_extract_id_from_filename`) is the work item ID, even if
  `work_item_id` in frontmatter differs. This applies to both legacy
  bare-number filenames and project-coded filenames.
- **Hierarchy safety**: hierarchy rendering must terminate in bounded
  time even if parent cycles exist. Detect cycles and render affected
  work items flat with a marker.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh list-work-items`
