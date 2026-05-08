---
name: refine-work-item
description: Interactively refine a work item by decomposing it into children,
  enriching it with codebase context, sharpening its acceptance criteria,
  sizing it, or linking it to dependencies. Use after a work item has been
  drafted and before planning begins.
argument-hint: "[work item number or path]"
disable-model-invocation: false
allowed-tools: Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*), Bash(${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/*)
---

# Refine Work Item

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh refine-work-item`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

**Work items directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh work`

## Work Item Template

The template below defines the sections and frontmatter fields that every
work item must contain. Read it now — use it to know valid types, statuses,
priorities, and section names without re-reading the file at runtime.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh work-item`

You are tasked with refining a work item through one or more of five operations:
decompose it into child work items, enrich it with codebase context, sharpen
vague acceptance criteria, add a t-shirt size indicator, or populate its
dependencies. Every operation is interactive and targeted — you propose, the
user approves, and the Edit tool makes the minimum change needed.

## Step 0 — Parameter Check

**If no work item path or number was provided**, respond with:

```
I'll help you refine a work item. Please provide the path or work item number.

Example: `/refine-work-item {work_dir}/0042-user-auth.md`
Or by number: `/refine-work-item 42`

Run `/list-work-items` to see available work items.
```

Then wait for the user's input.

**If a path or number was provided**: proceed to Step 1.

Accepted forms:
- A path (e.g. `meta/work/0042-user-auth.md`)
- A bare work item number (e.g. `0042` or `42`, resolved against `{work_dir}`)

If the resolved path does not exist, report:
```
No work item file at <path> — run `/list-work-items` to see available work items.
```
and exit without reading any other file, spawning any agent, or writing anything.

If the YAML frontmatter cannot be parsed (missing closing `---` or syntax
error), report:
```
Could not parse frontmatter in <path> — the file may be corrupted. Re-open it
and check that the YAML frontmatter is bracketed by two `---` lines and
contains all nine required fields, or run `/update-work-item <path>` which
surfaces the same diagnostic with field-level detail.
```
and exit without editing the file or spawning agents.

## Step 1 — Read Target and Context

Read the target work item fully. If the `parent` field is non-empty, also read
the parent work item. The work item template (loaded above) tells you the valid
types, statuses, and priorities — do not re-read it at runtime.

## Step 2 — Analyse (mandatory parallel agents)

Spawn BOTH agents in the same tool-use turn (parallel, not sequential):

- **{codebase locator agent}**: find files relevant to the work item's
  Requirements and Summary
- **{codebase analyser agent}**: analyse how the relevant components
  currently work

Wait for both before presenting the menu. Even if the work item seems
straightforward, always spawn these agents — the menu previews depend
on their findings.

## Step 3 — Present Refinement Menu

Present the five operations with one-line descriptions. Reference agent
findings in the previews so the user can pick informed:

1. **decompose** — split into child work items (epic→stories, story→tasks);
   e.g. "Requirements suggest 3 child stories (R1, R2, R3)"
2. **enrich** — add Technical Notes from codebase analysis;
   e.g. "4 relevant files identified (src/auth/session.ts, …)"
3. **sharpen** — tighten vague acceptance criteria;
   e.g. "2 vague criteria detected (AC3, AC5)" or "all criteria already testable"
4. **size** — append a t-shirt size indicator with rationale;
   e.g. "estimate M based on files in auth/ and session/"
5. **link** — populate Dependencies from related work items;
   e.g. "1 potential blocker found (0031)" or "no related work items found"

Operations that have nothing to do should be marked as such (e.g.
"sharpen — all criteria already testable").

User can select one, multiple, or "all relevant". Regardless of selection
order, always execute in canonical order: **decompose → enrich → sharpen
→ size → link**. This ensures Technical Notes content is in place before
size's `**Size**:` line is prepended, and decompose's children exist before
any link operation references them.

## Step 4 — Execute Operations

### 4a. Decompose

Propose 2–5 candidate children (2–4 for story decomposing to tasks) with
draft titles and one-line Summaries derived from the Requirements section.

**Bug/spike challenge**: if the work item type is `bug` or `spike`, first ask:
```
bug/spike work items don't typically decompose — are you sure? (y/n)
```
Only proceed on explicit `y`. On `n`, exit decompose and return to the menu.

**Existing children**: if the Requirements section already contains a
`### Child work items` subsection, offer:
```
append (add new children to the existing list) / skip (do not decompose further) / cancel
```
Never replace the existing list silently. For the append path, use the
anchor described below under "append to existing `### Child work items`".

**Approval grammar**: each proposal MUST include a one-line grammar legend
immediately under the numbered child list:
```
Commands: approve all | edit N: <title> | drop N | add: <title> | regenerate | cancel
```

The legend appears on every proposal turn — the first proposal and after
every grammar iteration.

Process user input:
- `approve all`, `yes`, `lgtm` → proceed to the pre-write warning
- `edit N: <new title>` → update child N's title, re-show updated proposal
  with legend restated. Do NOT write.
- `drop N` → remove child N, renumber remaining children 1…M, re-show
  with legend. Do NOT write.
- `add: <title>` → append a new child with that title, re-show with legend.
  Do NOT write.
- `regenerate` → discard current proposal, generate a fresh set of 2–5
  candidates from the same Requirements. Do NOT write.
- `cancel`, `abort` → print "decompose cancelled — no children written"
  and return to the menu. No numbers allocated.
- Any other input → print "unrecognised command", restate the legend, and
  re-show the unchanged proposal unchanged. Do NOT treat as approval.

**Pre-write warning**: before writing, emit:
```
This will allocate N work item numbers and write N files; aborting mid-write
leaves partial state — use `jj restore <file>` to discard any children
written. Proceed? (y/n)
```
Wait for explicit `y`. On `n`, cancel without writing.

**On approval**:

1. Call `work-item-next-number.sh --count N` exactly once to allocate N
   consecutive numbers.

2. For each child, write `NNNN-kebab-slug.md` with all nine frontmatter
   fields:
   - `work_item_id` — from the script, zero-padded four-digit string
   - `title` — per-child proposal title; body H1 matches exactly
   - `date` — current UTC timestamp via `date -u +%Y-%m-%dT%H:%M:%S+00:00`
   - `author` — first match in chain: parent work item's `author` field → configured
     `author` value (from context config) → `jj config get user.name` →
     `git config user.name` → ask the user once and apply to all children
   - `type` — derived: `epic → story`, `story → task`, `bug`/`spike` → ask
     user to confirm before proceeding (already done in the challenge step),
     any other type → `story` with a one-line notice
   - `status` — literal `draft`
   - `priority` — inherit from parent; if parent has none, ask once and
     apply to every child written in this session
   - `parent` — the target work item's `work_item_id`, canonicalised to a
     zero-padded four-digit string (e.g. `"1"` → `"0001"`)
   - `tags` — verbatim copy of the parent's `tags` array (empty array `[]`
     if the parent has none)

   Immediately before writing each child, verify the computed filename does
   not already exist. If it does, abort with:
   ```
   Collision: <path> already exists (concurrent session?). Aborting.
   Allocated: NNNN, NNNN, …; Wrote: N-1 files (list them).
   Use `jj restore <file>` to discard any children written.
   ```

   Child body includes: Summary (from proposal), Context (linking to
   parent with "Child of NNNN — <parent title>"), Requirements (minimal
   but substantive; no `[bracketed placeholder]` text), Acceptance Criteria
   (minimal but substantive; sharpen can tighten these later), Dependencies
   (blank), and remaining template sections.

3. **Append `### Child work items` to the parent's Requirements section**:

   After all children are written successfully:
   - Read the parent file
   - Locate `\n## Requirements\n` and `\n## Acceptance Criteria\n`
   - Extract the last non-empty line of Requirements (the line immediately
     before the blank-line transition to Acceptance Criteria); call this
     `<req_tail>`
   - Build: `old_string` = `<req_tail>\n\n## Acceptance Criteria\n`
   - Build: `new_string` = `<req_tail>\n\n### Child work items\n\n- NNNN — title\n…\n\n## Acceptance Criteria\n`
   - Edge case — empty Requirements: use `## Requirements\n\n## Acceptance Criteria\n`
     as `old_string` and insert `### Child work items` between them.
   - Pre-Edit, count occurrences of `old_string` in the parent file. If not
     exactly 1, abort the parent update with:
     ```
     Could not locate a unique '## Acceptance Criteria' anchor in <path>
     (matches found: <N>). Parent not updated.
     Children NNNN, NNNN, NNNN remain on disk; add their links manually
     or run `jj restore <parent-path>` and re-run decompose.
     ```
     Children already written remain on disk.

   **Append to existing `### Child work items`** (re-decompose path):
   - Locate `### Child work items` subsection, find its last `- NNNN — title`
     line; call this `<last_link>`
   - Build: `old_string` = `<last_link>\n\n## Acceptance Criteria\n`
   - Build: `new_string` = `<last_link>\n- NNNN — title\n…\n\n## Acceptance Criteria\n`
   - Same uniqueness pre-check; same abort diagnostic if not exactly 1.

4. Print a completion ledger, one line per child:
   ```
   Wrote NNNN — title
   Wrote NNNN — title
   Allocated 3 numbers, wrote 3 files.
   ```
   On aborted or partial write, show allocated vs. written counts and
   list any written filenames.

After a successful decompose, proceed to Step 5 (hierarchy display) before
running any remaining operations.

### 4b. Enrich

Read the target work item's existing Technical Notes content.

If the codebase agents returned nothing concrete (no specific files or
components identified), report:
```
no enrichment could be grounded in code — skipping enrich
```
and make no Edit.

Otherwise propose Technical Notes content with specific `path:line`
references drawn from agent results. Do not invent references.

If non-trivial Technical Notes content already exists (anything beyond a
`**Size**:` line), ask:
```
replace (deletes existing Technical Notes) / append (add after existing content) / skip?
```
- `replace` → show a unified diff (old struck, new added) and require an
  explicit second `y/n` confirmation before invoking Edit
- `append` → add new content after existing content. Preserve any leading
  `**Size**:` line placed by a prior size operation — never overwrite it
- `skip` → make no Edit

On approval via Edit: modify only the Technical Notes section. Do not touch
Requirements, Acceptance Criteria, Summary, or any frontmatter field.

### 4c. Sharpen

Read the target work item's Acceptance Criteria. Identify criteria that are
vague or untestable (non-measurable phrases like "should be fast", "handles
errors gracefully", "works correctly").

If every criterion is already specific and testable, report:
```
all acceptance criteria already testable — nothing to sharpen
```
and make no Edit.

Otherwise, for each vague criterion propose a specific, measurable rewrite
(e.g. "p95 latency under 200ms under default benchmark dataset"). Skip
criteria that are already testable — only propose rewrites for vague ones.
Iterate with the user until each proposed rewrite is agreed.

On approval via Edit: modify only the Acceptance Criteria section with the
agreed rewrites. Preserve criteria that were not sharpened byte-for-byte.

### 4d. Size

Read the target work item's Technical Notes. Check whether a `**Size**:` line
already exists as the first line.

Propose a t-shirt size (`XS`, `S`, `M`, `L`, `XL`) with a rationale
referencing specific files or subsystems from agent results. Iterate with
the user.

On approval:
- **No existing `**Size**:` line**: insert `**Size**: <value> — <rationale>`
  as the FIRST line of Technical Notes, followed by a blank line separating
  it from any existing content. Do NOT add as a frontmatter key.
- **Existing `**Size**:` line, proposed value AND rationale match byte-for-byte**
  (ignoring leading/trailing whitespace): report "size unchanged — <value>"
  and make no Edit.
- **Existing `**Size**:` line with a different value or rationale**: show a
  unified diff of the proposed change (existing line struck, new line added)
  and require an explicit second `y/n` confirmation before invoking Edit.
  On `y`, replace the line in place. On `n`, make no Edit.

### 4e. Link

Count `NNNN-*.md` files in `{work_dir}` using a single Glob invocation.

- **Count ≤ 30**: read them directly via batched Read
- **Count > 30**: spawn **{documents locator agent}** scoped to `{work_dir}`

Propose `Blocked by:` and/or `Blocks:` entries in Dependencies, referencing
only real work item numbers (verify each proposed number exists before
including it). If no related work items are found, print:
```
no related work items found — link skipped
```
and make no Edit.

If Dependencies already has non-empty content, ask:
```
replace (overwrites existing entries) / append (add new entries after existing) / skip?
```
- `replace` → show a unified diff and require a second `y/n` confirmation
- `append` → add only net-new entries, skipping any that duplicate existing
  entries
- `skip` → make no Edit

On approval via Edit: modify only the Dependencies section.

## Step 5 — Display Hierarchy

Run immediately after decompose writes at least one child (skip if decompose
was cancelled, declined by user, or the parent Edit failed).

Render the parent → children tree using the format pinned in `/list-work-items`:
Unicode box-drawing characters, two-space indent per depth level, `├── ` for
all children except the last, `└── ` for the last child. The canonical fence
below MUST appear verbatim in this step's prose so
`scripts/test-hierarchy-format.sh` can verify byte-equality with the matching
fence in `list-work-items/SKILL.md`:

<!-- canonical-tree-fence -->
NNNN — parent title (type: <type>, status: <status>)
  ├── NNNN — child 1 title (type: <type>, status: <status>)
  ├── NNNN — child 2 title (type: <type>, status: <status>)
  └── NNNN — last child title (type: <type>, status: <status>)
<!-- /canonical-tree-fence -->

Concrete work item IDs and titles replace the placeholders in the actual
output (e.g. `0042 — User Auth Rework (type: epic)`). The status field
of the parent is omitted only if blank; all present fields are shown.

Step 5 runs inline after decompose completes, before enrich, sharpen, size,
and link operate on the parent. The hierarchy is not re-rendered after
subsequent operations.

## Step 6 — Offer Review

After the entire selected operation set completes, offer:
```
Would you like to run `/review-work-item` on this work item now?
```
If decompose was in the selection, also offer to run review on each child
written. Do NOT invoke `/review-work-item` automatically — only make the offer.
The skill exits after this offer regardless of the user's response.

## Important Guidelines

- **Mandatory agents at Step 2**: always spawn both codebase agents in
  parallel before presenting the menu, even for simple work items.
- **Canonical operation order**: decompose → enrich → sharpen → size → link,
  regardless of the order the user selected them.
- **Each operation owns its sections only**:
  - decompose: writes new child files and appends `### Child work items` to
    the parent's Requirements (never touching existing Requirements prose)
  - enrich: owns Technical Notes prose content
  - sharpen: owns Acceptance Criteria
  - size: owns the single `**Size**: <value> — <rationale>` line, always as
    the FIRST line of Technical Notes; replace in place on re-run
  - link: owns Dependencies
- **Never modify any frontmatter field** of the target work item (`work_item_id`,
  `title`, `date`, `author`, `type`, `status`, `priority`, `parent`,
  `tags`) — those transitions are `/update-work-item`'s concern. Children of
  decompose are new work items getting their initial frontmatter.
- **Destructive paths require two-step confirmation**: replace mode for
  enrich, replace mode for link, clobbering an existing `**Size**:` line —
  each must show a unified diff and require a second `y/n` confirmation.
- **Edit failure path**: if an Edit's target string cannot be matched (file
  changed between read and edit), abort that specific edit with a clear
  diagnostic and continue with remaining approved edits. For decompose: write
  all children first, update the parent last; if the parent Edit fails,
  children stay on disk and work item numbers are printed for manual linking.
- **Numbering is consumed eagerly** on decompose with no rollback; the skill
  assumes a single-session invocation.
- **Canonicalise parent field** to zero-padded four-digit string (same as
  `/update-work-item`), e.g. `"1"` → `"0001"`.
- **Idempotent on re-run**: every operation checks existing content before
  proposing additions.

## Relationship to Other Commands

1. `/create-work-item` or `/extract-work-items` — create the work item
2. `/refine-work-item` — decompose and enrich (this command)
3. `/review-work-item` — automated multi-lens quality review
4. `/stress-test-work-item` — interactive adversarial examination
5. `/update-work-item` — status/metadata transitions (not this skill's concern)
6. `/create-plan` — plan implementation from an approved work item

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh refine-work-item`
