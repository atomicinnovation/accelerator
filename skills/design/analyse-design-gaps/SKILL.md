---
name: analyse-design-gaps
description: Compare two design inventories produced by inventory-design and emit
  a structured gap artifact whose prose paragraphs satisfy the extract-work-items
  cue-phrase contract. Use after running inventory-design for both a current and
  target design surface. The resulting gap artifact under meta/design-gaps/ feeds
  directly into /accelerator:extract-work-items to produce actionable work items.
argument-hint: "[current-source-id] [target-source-id]"
disable-model-invocation: true
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/analyse-design-gaps/scripts/*)
---

# Analyse Design Gaps

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh analyse-design-gaps`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

**Design inventories directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh design_inventories`
**Design gaps directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh design_gaps`

You are tasked with comparing two design inventories and producing a structured
`design-gap` artifact. The artifact's prose paragraphs must satisfy the
`extract-work-items` cue-phrase contract so they flow into the work-item
lifecycle without manual editing.

## Steps

### 1. Resolve Each Source-Id to Its Inventory

For each of `[current-source-id]` and `[target-source-id]`:

**a. Validate format**: must match `^[a-z0-9][a-z0-9-]*$`. Report a clear error
naming offending characters if not.

**b. Glob** `<design_inventories>/*-{source-id}/` (excluding leading-dot directories).

**c. Zero matches**: exit non-zero with:
> `error: source-id '<id>' did not match any inventory under <root>. Available
> source-ids: <comma-separated list>. Run /accelerator:inventory-design <id>
> <location> first.`

**d. Read frontmatter** for each match. If an `inventory.md` is missing,
unparseable, or lacks a `status` field, treat it as `superseded` and log a
one-line warning naming the directory.

**e. Filter to `status != superseded`** (also exclude leading-dot directories).
If multiple remain (crashed prior supersede step), apply the resolver tiebreaker:
1. Highest `sequence` number in frontmatter (primary — robust against clock skew)
2. Directory mtime, newest first (secondary — for equal-sequence concurrent writes)
3. `YYYY-MM-DD-HHMMSS` directory-name prefix, newest first (final)

Emit a one-line warning naming any non-selected directories when more than one
remained after filtering.

**f. Record** the resolved absolute paths in the gap artifact frontmatter as
`current_inventory` and `target_inventory`.

### 2. Read Both Inventories

Read each resolved `inventory.md` in full into context.

### 3. Compute Structural Diff

Compare the two inventories across five categories:

1. **Token Drift** — colour, typography, spacing tokens present in current but
   missing from target, or vice-versa; tokens with changed values
2. **Component Drift** — components present in both but with structural changes
   (variant count, prop changes, composition changes)
3. **Screen Drift** — screens/routes that have changed navigation structure,
   state matrix, or layout
4. **Net-New Features** — features present in target but not in current
5. **Removed Features** — features present in current but not in target

If a category has no differences, omit its H2 section from the artifact rather
than writing an empty section.

### 4. Write Gap Prose

For each non-empty category, write at least one paragraph whose prose satisfies
the cue-phrase contract used by `extract-work-items`. Each paragraph must contain
one or more of:
- "we need to …" / "we need a …"
- "users need …"
- "the system must …"
- "Implement <ProperNoun> …" (capital letter on the named feature)

**Guidance per category**:
- **Token Drift**: describe specific tokens to migrate, add, or remove. Example:
  "We need to migrate the 14-colour primary scale to the 8-token system defined
  in the target."
- **Component Drift**: describe the structural changes needed. Example:
  "Users need a five-variant Button — the current implementation has two variants
  while the target design specifies five."
- **Screen Drift**: describe changes to navigation structure or states. Example:
  "The system must support a redesigned Settings navigation pattern matching the
  target's two-level sidebar."
- **Net-New Features**: introduce the feature and its scope. Example:
  "Implement Search to expose Cmd+K activation, query input, and recent-history
  preview panels."
- **Removed Features**: flag for confirmation before removal. Example:
  "The legacy Analytics tab is present in the current implementation but absent
  from the target. We need to confirm with stakeholders whether it should be
  removed before proceeding."

Avoid bare structural headings without actionable content. Every non-empty H2
must contain at least one paragraph meeting the cue-phrase contract.

### 5. Run Cue-Phrase Audit

After generating the gap body, run:
```bash
${CLAUDE_PLUGIN_ROOT}/skills/design/analyse-design-gaps/scripts/audit-cue-phrases.sh \
  "<draft-body-path>"
```

**On failure**: the script names the offending H2 sections. Revise only those
sections while keeping passing sections byte-identical. Re-run the audit. After
three consecutive failures, write the rejected body to:
```
<design_gaps>/.YYYY-MM-DD-{current-source-id}-vs-{target-source-id}.draft.md
```
with a note at the top naming the failed sections, then report the failure and
the draft path to the user. Do not write the artifact to its final path.

The frontmatter timestamp is generated once before the first write attempt and
reused across retries.

### 6. Generate Metadata

Run:
```bash
${CLAUDE_PLUGIN_ROOT}/skills/design/analyse-design-gaps/scripts/gap-metadata.sh
```

### 7. Write Artifact

Use the `design-gap` template:
```
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh design-gap`
```

Write to:
```
<design_gaps>/YYYY-MM-DD-{current-source-id}-vs-{target-source-id}.md
```

The `References` section must record:
- The resolved inventory directory path for each source-id
- Any resolver warnings emitted in Step 1

### 8. Present Summary

Report:
- The artifact path
- Count of non-empty drift categories
- Any resolver warnings (multi-match or corrupt-frontmatter)
- Whether the cue-phrase audit required revisions

Suggest next steps:
- Run `/accelerator:extract-work-items <gap-file>` to generate work items

## Important Guidelines

- Empty drift categories are omitted — do not write placeholder sections.
- The `sequence` tiebreaker is authoritative; do not rely on directory-name date
  alone.
- Never fabricate observations. Write only what differs between the two inventories.
- The cue-phrase audit is programmatic enforcement, not a style suggestion. Revise
  failing sections until the audit passes or the three-attempt limit is reached.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh analyse-design-gaps`
