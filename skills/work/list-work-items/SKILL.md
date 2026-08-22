---
name: list-work-items
description: List and filter work items from the configured work directory.
  Use when discovering what work items exist, filtering by
  status/kind/priority/parent/tag, or viewing the work item hierarchy.
argument-hint: "[filter description]"
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator config *)
  - Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator work *)
---

# List Work Items

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config context --skill list-work-items --fail-safe`
!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config agents --fail-safe`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

**Work items directory**: !`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config path work --fail-safe`
**Work item ID pattern**: !`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config work id_pattern --fail-safe`
**Default project code**: !`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config work default_project_code --fail-safe`
**Active integration**: !`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config work integration --fail-safe`

The reads above inform how you parse the filter argument. `accelerator work
list` owns the sync-status rendering itself: it adds the Sync column only when
`work.integration` names a tracker and a `last-sync.json` baseline exists, and
degrades to presence-only when the remote is unreachable — you do not branch on
the integration here.

## Work Item Template

The following template defines the work item schema and field defaults.

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config template work-item --fail-safe`

**Known kind values**: !`${CLAUDE_PLUGIN_ROOT}/bin/accelerator work template-hints kind`
**Known status values**: !`${CLAUDE_PLUGIN_ROOT}/bin/accelerator work template-hints status`
**Known priority values**: !`${CLAUDE_PLUGIN_ROOT}/bin/accelerator work template-hints priority`

Each line above lists one value per line. Collect these into three sets:
known kinds, known statuses, and known priorities. These hints inform
the filter shorthand rules below but do not restrict what values may
appear on work items — legacy values like `todo`, `done`, or
`adr-creation-task` are matchable only via the explicit structured form
(rule 2).

You are tasked with listing and filtering work items from the configured
work items directory. This is a **read-only** skill — never write any files
and never spawn sub-agents. Your job is to parse the user's natural-language
filter into `accelerator work list`'s flags and present its output; the CLI
owns the scan, filter, sync classification, and table/hierarchy rendering.

## Step 1: Resolve Filter

If an argument was provided, parse it as a filter expression using the
following precedence rules (rules 3–4 use the known kind/status/priority
sets collected above). The first rule that matches wins.

### Filter Precedence Rules

**Rule 1 — Presentation keywords** (changes rendering, not filtering):
`hierarchy`, `as a tree`, `show hierarchy`. These may combine with a
filter (e.g. `hierarchy under 0042`). Strip the keyword from the
argument and continue parsing the remainder, if any, through rules 2–5.

**Rule 2 — Explicit structured forms** (keyword identifies the field):
- `tagged <value>` or `with tag <value>` → filter by tag
- `under <value>` or `children of <value>` → filter by parent
- `status <value>` → filter by status (matches any value on any work item)
- `kind <value>` → filter by kind (matches any value on any work item)
- `priority <value>` → filter by priority
- `about <text>` → free-text title search (case-insensitive substring)

**Rule 3 — Multi-token template-value shorthand**: two or more tokens
that each match a known template-comment value in different fields.
For example, `bugs in review` → `kind: bug AND status: review` (after
singularising `bugs` to `bug` and recognising `in-progress`, `review`,
etc. as status values with filler words like `in`, `only`, `all`
stripped). Both tokens must match values from distinct fields. If either
token is ambiguous across fields, fall through to rule 5.

**Rule 4 — Single-token template-value shorthand**: one token matching a
known kind, status, or priority value from the template comments. Map
common plurals (`bugs`→`bug`, `epics`→`epic`, `stories`→`story`,
`tasks`→`task`, `spikes`→`spike`) and common synonyms
(`drafts`→`draft`). If the token matches values in more than one field,
ask the user for disambiguation rather than guessing.

**Rule 5 — Free-text title search**: anything that does not match rules
1–4 is treated as a case-insensitive substring search against the
`title:` frontmatter field.

If no argument was provided: no filter — list every work item.

### Translate the parsed filter into `work list` flags

Once the argument is parsed, map each recognised clause onto a concrete
flag. This translation is the only work the skill does; the CLI applies the
filters and echoes the interpreted filter and match count itself, so do not
echo it separately.

| Parsed clause | Flag |
|---|---|
| presentation keyword (`hierarchy`, `as a tree`) | `--hierarchy` |
| `tagged X` / `with tag X` (repeatable) | `--tag X` |
| `under X` / `children of X` | `--parent X` |
| `status X`, or a status shorthand token | `--status X` |
| `kind X`, or a kind shorthand token | `--kind X` |
| `priority X` | `--priority X` |
| `about X`, or free-text (rule 5) | positional `X` (last argument) |

A multi-token shorthand (rule 3, e.g. `bugs in review`) becomes several
flags (`--kind bug --status review`). Pass the canonical value the parse
resolved (singularised, synonym-mapped); `--parent` is canonicalised by the
CLI, so pass the bare form the user gave. Every flag is a conjunct — an item
must satisfy all of them.

## Step 2: Run `work list` and present its output

Invoke the CLI with the translated flags:

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator work list \
  [--status <s>] [--kind <k>] [--priority <p>] [--parent <ref>] \
  [--tag <t>]... [--hierarchy] [<title-substring>]
```

The CLI does everything the previous scan/filter/render steps did, so there
is nothing to reconstruct in prose:

- **Scan and validity.** It reads every `*.md` file in the work directory,
  treats a file as a work item only when it has closed frontmatter and a
  non-empty `id` (or legacy `work_item_id`), silently excludes non-items,
  and warns `"<filename>: skipped — no frontmatter"` /
  `"… unclosed frontmatter"` on malformed files. The filename prefix stays
  the authoritative displayed ID.
- **Filter.** Each flag is applied as a conjunct; `--parent` canonicalises
  both sides (short and long ID forms compare equal); the title term is a
  case-insensitive substring. The CLI prints the interpreted filter and
  match count itself (`Filter: status=draft (3 matches)`, `Children of 0042
  (2 matches)`, or `All work items (29 total)`).
- **Sync column.** When `work.integration` names a tracker and a
  `last-sync.json` baseline exists, it appends a **Sync** column carrying
  the five-state label vocabulary (`🟢 synced`, `⚪ unsynced`,
  `🔵 locally modified`, `🟣 remotely modified`, `🔴 conflict`), driven by one
  bulk remote read through the shared classifier. With no integration or no
  baseline it renders presence-only and omits the column; if the remote is
  unreachable it degrades to presence-only and still exits 0 — never
  retrying or hanging.
- **Hierarchy.** `--hierarchy` renders the parent/child tree with Unicode
  box-drawing characters, appends each line's sync label, marks an
  out-of-set parent `(parent … not found)`, and detects cycles so rendering
  always terminates.

  The parent/child tree renders as, for example:

<!-- canonical-tree-fence -->
NNNN — parent title (kind: <kind>, status: <status>)
  ├── NNNN — child 1 title (kind: <kind>, status: <status>)
  ├── NNNN — child 2 title (kind: <kind>, status: <status>)
  └── NNNN — last child title (kind: <kind>, status: <status>)
<!-- /canonical-tree-fence -->

- **Empty and missing.** A directory that does not exist prints the
  `paths.work` guidance and exits 0; an empty directory prints `No work
  items found in …`; a filter that matches nothing prints `No work items
  matched: …` rather than an empty table.

Print the CLI's stdout to the user as-is (it is markdown-native — a table or
tree the conversation renders directly), and surface any stderr warnings it
emitted. Do not re-echo the filter or re-render the table.

## Quality Guidelines

- **Read-only**: never write any files. This skill only reads and
  displays.
- **No sub-agents**: never spawn sub-agents. All work is a natural-language
  filter parse plus one `accelerator work list` invocation.
- **No hardcoded field values**: never assume a specific set of status,
  kind, or priority values. The template's comments list shipping
  defaults, not a closed set. Users may override the template with
  custom values.
- **Explicit structured filters are universal**: `status <value>`,
  `kind <value>`, etc. match any value present on any work item, not just
  template defaults. This is how legacy values like `todo` or
  `adr-creation-task` are reachable.
- **Resilient to malformed work items**: missing or unclosed frontmatter
  must not crash the listing — warn using the resolved filename and
  continue. Warning phrasing should match `work show`'s own errors:
  "no frontmatter" / "unclosed frontmatter".
- **Filename is authoritative**: the ID extracted from the filename is
  the work item ID, even if the `id` field (or `work_item_id` on
  legacy files) in frontmatter differs. This applies to both legacy
  bare-number filenames and project-coded filenames.
- **Hierarchy safety**: hierarchy rendering must terminate in bounded
  time even if parent cycles exist. Detect cycles and render affected
  work items flat with a marker.

!`${CLAUDE_PLUGIN_ROOT}/bin/accelerator config instructions list-work-items --fail-safe`
