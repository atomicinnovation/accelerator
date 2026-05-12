---
date: "2026-04-27T09:44:53+01:00"
researcher: Toby Clemson
git_commit: 6947ac9f1b3d2429623df1d008cc38578bbde52f
branch: ticket-management
repository: accelerator
topic: "Extending create-work-item to accept an existing work item as a starting point"
tags: [research, create-work-item, work-item, skills, argument-handling, enrichment]
status: complete
last_updated: "2026-04-27"
last_updated_by: Toby Clemson
---

# Research: Extending `create-work-item` to Accept an Existing Work Item

**Date**: 2026-04-27T09:44:53+01:00
**Researcher**: Toby Clemson
**Git Commit**: 6947ac9f1b3d2429623df1d008cc38578bbde52f
**Branch**: ticket-management
**Repository**: accelerator

## Research Question

Extend the `create-work-item` skill so that it can be called with an existing
work item (by name or relative path). It should read that work item, treat it
as a shell that needs more work, and perform the same interactive tasks as
normal creation while taking into account any existing context in the document.

## Summary

The extension is well-supported by the existing codebase and requires changes
to only one file: `skills/work/create-work-item/SKILL.md`. No new scripts are
needed — `work-item-read-field.sh` (already in `allowed-tools`) can extract
any frontmatter field from an existing file, and the two-form argument
resolution pattern (path-like vs. numeric) is already established across four
peer skills. The main structural change is introducing a **branch point at Step
0**: when the argument resolves to an existing file, the skill enters an
"enrich existing" path that runs a gap analysis instead of broad discovery
questions, preserves identity fields, and overwrites the existing file at write
time rather than allocating a new number and path.

The extension is a clean augmentation of the creation flow — the five-step
structure stays intact, every step simply has an "existing file" variant that
is either abbreviated (Step 1) or redirected (Step 5).

---

## Detailed Findings

### 1. Current Argument Handling

`skills/work/create-work-item/SKILL.md:1-8` declares:

```yaml
argument-hint: "[topic or description]"
```

Step 0 (`SKILL.md:60-87`) has **two branches**:

- **Argument provided** (lines 64–71): Evaluate vagueness; if vague ask a
  clarifying question, otherwise proceed to Step 1.
- **No argument** (lines 73–87): Emit a fixed invitation prompt and wait.

There is no file-path or numeric branch. The skill treats every argument as a
topic string. No file resolution logic exists anywhere in the skill body.

The `allowed-tools` frontmatter (lines 6–7) permits:
```
Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
Bash(${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/*)
```

This already covers `work-item-read-field.sh`, `work-item-read-status.sh`, and
every other script in `skills/work/scripts/`. No `allowed-tools` change is
needed for the extension.

---

### 2. The Canonical Two-Form Resolution Pattern

Four peer skills (`review-work-item`, `update-work-item`,
`stress-test-work-item`, `refine-work-item`) implement the same two-form
argument resolution pattern. The most complete specification is in
`review-work-item/SKILL.md:35-50`:

- **Path-like** — contains `/` or ends in `.md`: treat as a literal file path.
  If not found: print `"No work item at <path>."` and offer `/list-work-items`.
- **Numeric** — zero-pad to 4 digits, glob `{work_dir}/NNNN-*.md`:
  - 0 matches: print error and offer `/list-work-items`.
  - 1 match: use it.
  - Multiple matches: list them and ask user to select.

`update-work-item/SKILL.md:43-55` extends this with a numbered-option
interactive selection when multiple files match a numeric glob.

`refine-work-item/SKILL.md:61-75` additionally handles the case where the
frontmatter is unparseable (missing closing `---` or syntax error) — exiting
with a diagnostic rather than attempting to read partial content.

The `create-work-item` extension should adopt the `review-work-item`/
`update-work-item` specification as the authoritative pattern: path-like +
numeric with the three-way numeric match (0/1/N), plus the `refine-work-item`
unparseable-frontmatter guard.

---

### 3. The Branch Point: Three-Way Step 0

After the extension, Step 0 becomes a **three-way branch**:

| Argument form | Current behaviour | Extended behaviour |
|---|---|---|
| Absent | Emit invitation prompt, wait | Unchanged |
| Topic string | Vagueness check, possibly clarify | Unchanged |
| Path-like or numeric | (not handled) | Resolve → read file → enter "enrich existing" path |

The discriminator is applied **before** the vagueness check:
1. If the argument contains `/` or ends in `.md` → file-path form.
2. If the argument matches `^[0-9]{1,4}$` → numeric form.
3. Otherwise → topic string (existing logic).

Ambiguous cases (e.g. a topic that happens to be a 4-digit number like `1234`)
should be treated as numeric by default; if the resolved file does not exist
the skill should fall through and ask the user whether they meant a topic or a
work item number.

---

### 4. Reading an Existing File

Once a file is resolved, the skill should read it fully (all frontmatter and
body). `work-item-read-field.sh` can extract individual frontmatter fields:

```bash
"${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-read-field.sh" work_item_id <path>
"${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-read-field.sh" title <path>
"${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-read-field.sh" type <path>
"${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-read-field.sh" status <path>
"${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-read-field.sh" author <path>
"${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-read-field.sh" date <path>
"${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-read-field.sh" priority <path>
"${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-read-field.sh" parent <path>
"${CLAUDE_PLUGIN_ROOT}/skills/work/scripts/work-item-read-field.sh" tags <path>
```

The script exits 1 on missing file, missing/unclosed frontmatter, or field not
found (`work-item-read-field.sh:29-70`). Unparseable frontmatter should cause
an early abort, as established by `refine-work-item/SKILL.md:68-75`.

In practice, the body content is also needed for the gap analysis (Step 1
adapted), so the model should read the full file — not just the frontmatter
fields extracted via script. The script calls are most useful for extracting
the identity fields that must be preserved exactly (see §5).

---

### 5. Identity Fields: Preserve vs Regenerate

The key design constraint is that an existing work item has **identity fields**
that must be preserved, and **content fields** that the skill is enriching.

**Must be preserved from the existing file:**

| Field | Why |
|---|---|
| `work_item_id` | The permanent identity of this work item; `work-item-next-number.sh` would return a *higher* number because the existing file is already on disk and counted |
| `date` | Original creation timestamp; calling `date -u` would replace it with the current time |
| `author` | Original author; may differ from the current invoking user |
| `status` | May have advanced beyond `draft`; the quality guidelines default `status: draft` at creation which would be a regression |

**May be updated by the enrichment process:**

`title`, `type`, `priority`, `parent`, `tags`, and all body sections — these
are the content the skill is enriching.

The skill's quality guidelines currently state (`SKILL.md:281-296`): "status:
always `draft` at creation". This needs a carve-out: when enriching an
existing file, preserve the existing status unless the user explicitly requests
a change.

---

### 6. Changes to Each Step

#### Step 0 (Argument handling) — `SKILL.md:60-87`

Add a third branch **before** the existing vagueness check:
1. Detect path-like or numeric argument.
2. Resolve to a file path using the two-form pattern.
3. Read the file fully; extract identity fields via `work-item-read-field.sh`.
4. Set an internal flag (e.g. `{enriching_existing}`) and the resolved path
   (e.g. `{existing_work_item_path}`).
5. Skip to an adapted Step 1 — do not ask the usual topic-gathering questions.

#### Step 1 (Business context) — `SKILL.md:89-105`

**Normal path**: unchanged — ask 3–5 broad discovery questions.

**Enrich-existing path**: Replace broad discovery with a **gap analysis**:
- Identify which sections are **substantive** (contain real content beyond the
  template's instructional `[...]` placeholder text) and which are **gaps**
  (empty, contain only placeholder text, or are clearly stubs).
- Present the gap analysis to the user: "I've read the existing work item.
  Here's what looks complete and what still needs work: [summary]. I'll ask
  targeted questions about the gaps."
- Ask only questions that address the identified gaps (missing acceptance
  criteria, empty dependencies section, unstated type, etc.).
- If the existing content is rich enough that no gaps are obvious, briefly
  confirm what the user wants to add or improve and proceed.

A section qualifies as a gap if it:
- Is empty or contains only template placeholder text (`[...]` blocks)
- Consists solely of the template's instructional prose carried over verbatim
- Is missing entirely from the file

#### Step 2 (Investigation) — `SKILL.md:109-142`

Largely unchanged. The duplicate-detection branch (lines 130–142) should
exclude the work item being enriched from the similarity scan — the
`{documents locator agent}` search of `{work_dir}` will find the existing
file; the skill must not surface it as a "potential duplicate" of itself.

The web-search-researcher spawn criteria (lines 117–123) apply equally —
if the existing item's topic is well-understood, skip it; otherwise spawn.

#### Step 3 (Proposal and refinement) — `SKILL.md:144-200`

**Normal path**: unchanged — lead with a structured proposal.

**Enrich-existing path**: Replace the proposal with a **review and augmentation
presentation**:
- Show the existing content section by section, annotating each with one of:
  `[complete]`, `[needs improvement: <brief reason>]`, or `[missing]`
- Propose specific additions for the gaps identified in Step 1
- Retain the refinement loop (lines 194–200) — challenge untestable criteria,
  vague requirements, etc. — applied to both existing and proposed content

#### Step 4 (Draft production) — `SKILL.md:202-239`

**Normal path**: unchanged — draft with `XXXX` placeholder.

**Enrich-existing path**:
- Produce a complete updated work item incorporating existing content +
  approved additions from Step 3
- Use the preserved identity fields: `NNNN` (from `work_item_id`), existing
  `date`, existing `author`, existing `status` (unless user changed it)
- The H1 heading remains `# NNNN: <title>` using the preserved `NNNN`
- No `XXXX` placeholder is needed — the real number is already known
- `work-item-next-number.sh` must NOT be called at this step (as stated in
  `SKILL.md:238-239`) and is not called at all in the enrich-existing path

#### Step 5 (File write) — `SKILL.md:241-271`

**Normal path**: unchanged — call `work-item-next-number.sh`, check path does
not exist, write.

**Enrich-existing path**:
- Do NOT call `work-item-next-number.sh`
- Target path is `{existing_work_item_path}` (already known from Step 0)
- The existence-check guard (`SKILL.md:255-260`) becomes a **confirmation
  step** rather than an abort: "I'm about to overwrite `{path}`. Confirm?"
  or, if using an interactive approval loop that already happened in Step 3,
  a silent overwrite is acceptable.
- Show a brief summary of what changed (fields updated, sections added/
  modified) before writing.
- Confirmation message changes from "Work item created" to
  "Work item updated: `{existing_work_item_path}`".

---

### 7. Frontmatter and Quality Guideline Changes

**`argument-hint`** (line 5): change from:
```
[topic or description]
```
to:
```
[topic, description, or existing work item number/path]
```

**Quality guidelines** (`SKILL.md:273-323`) require additions:

- When enriching an existing file: preserve `work_item_id`, `date`, `author`,
  and `status` from the source file. Do not regenerate these fields.
- The `status: draft` default (line 284) applies only to new work items, not
  to enrichment of existing ones.
- The H1 heading must use the existing `NNNN` when enriching, not `XXXX`.

---

### 8. What Requires No Changes

- `templates/work-item.md` — no change needed; used for gap analysis
  reference, not as an output template in the enrich path
- `work-item-next-number.sh` — not called in the enrich path; no change needed
- `work-item-read-field.sh` — already handles every required field; no change
  needed
- `work-item-read-status.sh` — no change needed
- `work-item-update-tags.sh` — no change needed
- `work-item-template-field-hints.sh` — no change needed
- `plugin.json` — no change needed
- Any config scripts — no change needed
- Any other skill — no change needed (this is a self-contained addition to
  `create-work-item/SKILL.md`)

---

### 9. Design Constraints from Prior Documents

The Phase 2 plan (`meta/plans/2026-04-19-ticket-creation-skills.md`) drew an
explicit boundary: `create-work-item` must not offer to modify an existing item
inline when a near-duplicate is found during the duplicate-detection branch
(plan line 262):

> "Do not offer to modify it inline. Wait for the user's choice."

The extension respects this constraint because the user is explicitly invoking
the skill **with** an existing file path — that is an explicit instruction to
enrich, not an accidental collision. The duplicate-detection branch (Step 2)
applies to *other* similar items discovered during investigation, not to the
explicitly provided input file.

The plan also notes that `refine-work-item` (Phase 6, not yet planned) was
designed for enrichment. The extension proposed here is narrower than
`refine-work-item` — it does not decompose epics, link dependencies in bulk,
or spawn codebase agents for technical enrichment. It reuses the existing
creation conversation to fill out a sparse draft, which is a different
(lighter) operation. Both can coexist: `create-work-item <path>` fills content
gaps through dialogue; `refine-work-item <path>` does structural decomposition
and technical enrichment.

---

## Code References

- `skills/work/create-work-item/SKILL.md:1-8` — frontmatter, argument-hint, allowed-tools
- `skills/work/create-work-item/SKILL.md:60-87` — Step 0 argument handling (both branches)
- `skills/work/create-work-item/SKILL.md:89-105` — Step 1 business context questions
- `skills/work/create-work-item/SKILL.md:109-142` — Step 2 investigation and duplicate detection
- `skills/work/create-work-item/SKILL.md:144-200` — Step 3 proposal and refinement loop
- `skills/work/create-work-item/SKILL.md:202-239` — Step 4 draft production (XXXX placeholder rule)
- `skills/work/create-work-item/SKILL.md:241-271` — Step 5 file write (work-item-next-number.sh, path check)
- `skills/work/create-work-item/SKILL.md:273-323` — quality guidelines (field defaults, status: draft)
- `skills/work/review-work-item/SKILL.md:35-50` — canonical two-form argument resolution pattern
- `skills/work/update-work-item/SKILL.md:43-55` — numeric multi-match handling (numbered options)
- `skills/work/refine-work-item/SKILL.md:61-75` — unparseable-frontmatter guard
- `skills/work/scripts/work-item-read-field.sh` — field extraction from existing file
- `skills/work/scripts/work-item-next-number.sh:43-73` — filesystem scan algorithm (why existing file causes number conflict)
- `templates/work-item.md:1-11` — identity fields and their defaults

## Architecture Insights

The extension is a **clean branching addition to an existing skill** rather
than a separate skill. The five-step structure is preserved; each step gains a
parallel "enrich-existing" variant that runs when `{existing_work_item_path}`
is set. The new variant is lighter in every step: Step 1 is targeted not
broad, Step 3 augments rather than proposes from scratch, Step 5 overwrites
rather than allocates.

The two-form argument resolution pattern (path-like vs. numeric) is already
a codebase convention established across four skills. Adding it to
`create-work-item` is an alignment with the existing norm, not an invention.

The only genuine novelty is the **gap analysis** concept — determining which
sections of an existing document are substantive vs. placeholder. This is
entirely model-driven (no script required): the model compares section content
against the template's instructional text and applies judgment. This is
analogous to the vagueness-check logic already in Step 0 (line 66-69), which
is also model-driven with no scripted validation.

## Historical Context

- `meta/plans/2026-04-19-ticket-creation-skills.md` — Phase 2 plan for
  `create-work-item` and `extract-work-items`; establishes the "no inline
  editing" constraint (line 262) and the deferred-numbering rule (lines 59-61)
- `meta/research/codebase/2026-04-08-ticket-management-skills.md` — original design
  for `create-ticket`; defines `refine-ticket` (§5.5) as the Phase 6 skill
  for enrichment/decomposition — a distinct and heavier operation than the
  "fill in the gaps" enrichment proposed here
- `skills/work/review-work-item/SKILL.md:35-50` — canonical two-form
  resolution pattern that all work item skills targeting existing files follow

## Related Research

- `meta/research/codebase/2026-04-08-ticket-management-skills.md` — original skill design
- `meta/research/codebase/2026-04-25-rename-tickets-to-work-items.md` — rename research
  (historical context only)

## Open Questions

1. **Interactive confirmation before overwrite**: Should the skill show a
   before/after diff of changed fields, or just a summary of sections
   modified? The `update-work-item` precedent shows a diff preview before
   writing; mirroring that would be consistent but adds complexity to the
   enrichment step.

2. **Status upgrade on approval**: If the user's existing item is `draft` and
   the enrichment session results in a fully completed item, should the skill
   offer to transition `status: draft → ready` as part of the write step?
   This would be a useful quality-of-life feature but is not strictly required
   for the core extension.

3. **Scope boundary with `refine-work-item`**: The extension covers "fill in
   content gaps through dialogue". Once `refine-work-item` is implemented, the
   skill descriptions will need to clearly distinguish the two: `create-work-item
   <path>` = collaborative content enrichment; `refine-work-item <path>` =
   structural decomposition and technical enrichment via codebase agents.

4. **Evals**: The existing create-work-item eval suite covers 12 scenarios
   (`meta/plans/2026-04-19-ticket-creation-skills.md:107-183`), all of which
   use topic-string inputs. New eval scenarios would be needed for the
   enrich-existing path: a sparse file with missing sections, a file with a
   wrong type, a file with non-stub content that should be preserved, and a
   numeric argument that resolves correctly.
