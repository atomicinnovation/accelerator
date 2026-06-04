---
name: extract-adrs
description: Extract architecture decision records from existing meta documents
  (research, plans). Scans documents for implicit or explicit architectural
  decisions and converts selected ones into formal ADRs. Use when decisions are
  buried in research or planning documents and need to be captured formally.
argument-hint: "[research doc paths...] or leave empty to scan all"
allowed-tools:
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/scripts/artifact-*)
  - Bash(${CLAUDE_PLUGIN_ROOT}/skills/decisions/scripts/*)
---

# Extract ADRs from Meta Documents

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh extract-adrs`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

If no "Agent Names" section appears above, use these defaults:
accelerator:reviewer, accelerator:codebase-locator,
accelerator:codebase-analyser, accelerator:codebase-pattern-finder,
accelerator:documents-locator, accelerator:documents-analyser,
accelerator:web-search-researcher.

**Decisions directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh decisions`
**Research directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh research_codebase`
**Plans directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans`

You are tasked with identifying architectural decisions within existing meta
documents and helping the user capture them as formal ADRs.

## Initial Setup

When this command is invoked:

1. **Check if parameters were provided**:

- If one or more file paths were provided, note them as the target documents
- If no parameters provided, respond with:

```
I'll help you extract architectural decisions from existing documents.

You can:
1. Specify documents to scan: `/accelerator:extract-adrs @{research directory}/2026-03-18-auth-flow.md`
2. Let me scan all documents in the configured directories for decisions (this may take a moment)

Which would you prefer?
```

Wait for user input.

## Process Steps

### Step 1: Identify Source Documents

1. If specific files were provided, read them FULLY
2. If scanning all meta documents:
   - Spawn a **{documents locator agent}** agent to find all documents in the
     configured research, plans, and decisions directories (shown above)
   - Present the discovered documents and let the user select which to scan:
     ```
     I found the following documents:

     **Research:**
     - `{research directory}/2026-03-18-auth-flow.md` — Authentication flow research
     - ...

     **Plans:**
     - `{plans directory}/2026-03-18-api-redesign.md` — API redesign plan
     - ...

     Which documents should I scan for decisions? (enter numbers, "all", or
     specific paths)
     ```
   - Wait for user selection

### Step 2: Analyse Documents for Decisions

1. **Spawn {documents analyser agent} agents** (one per document, in parallel) with
   instructions to identify architectural decisions. Look for:

   - Explicit decision statements ("We decided...", "We will use...",
     "The approach is...")
   - Option comparisons and tradeoffs ("Option A vs Option B",
     "We considered...")
   - Technology selections ("We chose X", "Using Y because...")
   - Pattern/approach choices ("The pattern for this is...",
     "We'll follow the X approach")
   - Constraint acknowledgements ("Due to X, we must...",
     "Given the constraint of...")
   - Recommendations with rationale ("Recommendation: Use X because...")

2. **Wait for all agents to complete**

3. **Present discovered decisions** as Y-statement summaries:

```
I found the following architectural decisions in the scanned documents:

1. **[Short title]** — In the context of [X], facing [Y], we decided for [Z]
   to achieve [Q], accepting [D].
   Source: `{research directory}/2026-03-18-topic.md`

2. **[Short title]** — In the context of [X], facing [Y], we decided for [Z]
   to achieve [Q], accepting [D].
   Source: `{plans directory}/2026-03-18-topic.md`

3. ...

Which decisions would you like to capture as ADRs? (enter numbers, "all",
or "none")
```

Wait for user selection.

### Step 3: Generate ADRs

1. **Gather metadata** by running:
   ```
   ${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh
   ```

2. **For each selected decision**, generate a draft ADR using the create-adr
   template with:
   - Context pre-filled from the source document
   - Decision drivers extracted from the document's analysis
   - Considered options populated if the source discusses alternatives
   - Consequences derived from the document's findings
   - References linking back to the source document
   - Use a placeholder number (e.g., `ADR-XXXX`) in the draft — final numbers
     are assigned after approval to avoid gaps from skipped ADRs

3. **Present each generated ADR** for user review:
   ```
   Here's draft ADR #N of M:

   [ADR content with placeholder number]

   Does this look good? (yes / revise / skip / approve all remaining)
   ```

   Wait for approval before proceeding. If the user selects "approve all
   remaining", accept all subsequent drafts without further prompts.

4. **Assign final ADR numbers** to approved ADRs only, by running:
   ```
   ${CLAUDE_PLUGIN_ROOT}/skills/decisions/scripts/adr-next-number.sh --count N
   ```
   where N is the number of approved (not skipped) ADRs. Replace placeholder
   numbers with the assigned sequential numbers. This ensures no gaps from
   skipped decisions.

### Step 4: Write ADRs

1. Create the configured decisions directory if it doesn't exist

2. **Populate frontmatter** for each approved ADR. Before writing the
   file, capture metadata and substitute the unified base fields into
   the template's frontmatter block:

   1. Invoke `${CLAUDE_PLUGIN_ROOT}/scripts/artifact-derive-metadata.sh`
      once for the batch to obtain `Current Date/Time (UTC):`.
   2. For each approved ADR, **substitute** every field below with the
      indicated value:
      - `type:` ← `adr`
      - `id:` ← the ADR identifier `ADR-NNNN`, always quoted as a
        YAML string
      - `title:` ← the ADR title (without the `ADR-NNNN:` prefix)
      - `date:` ← the `Current Date/Time (UTC):` value
      - `author:` ← the author resolved per the standard chain
        (config → VCS user → prompt)
      - `producer:` ← `extract-adrs`
      - `status:` ← `proposed`
      - `last_updated:` ← the same `Current Date/Time (UTC):` value
      - `last_updated_by:` ← the same value resolved for `author`
      - `schema_version:` ← `1` (bare integer)

      Optional linkage / decision-maker keys are omit-by-default:
      the template shows each as `""`/`[]`, but write a key
      into the artifact **only** when it has a value, and omit it
      entirely otherwise (do not carry the empty placeholder through).

      - `parent:` ← the owning work item as a typed-linkage ref
        (`"work-item:NNNN"`). Fill when the source names an owning work
        item; otherwise omit the key.
      - `supersedes:` ← a YAML list of typed-linkage refs of the form
        `"adr:ADR-NNNN"` to the ADR(s) this one replaces. Fill when the
        source records a supersession; otherwise omit the key.
      - `relates_to:` ← list of typed-linkage refs to loosely related
        ADRs (`["adr:ADR-NNNN", ...]`). Fill when related decisions are
        explicit in the source; otherwise omit the key.
      - `decision_makers:` ← a YAML list of the people who agreed. Fill
        when the source names them; otherwise omit the key.

3. Write each approved ADR to:
   `{decisions directory}/ADR-NNNN-description.md`

4. Present summary:
   ```
   Created the following ADRs:
   - `{decisions directory}/ADR-0001-description.md` — [title]
   - `{decisions directory}/ADR-0002-description.md` — [title]
   - ...

   All ADRs are in "proposed" status. Use `/accelerator:review-adr` to
   review and accept them.
   ```

## ADR Template

The ADR template is loaded directly via the template loader so the
shape stays in sync with `create-adr`:

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh adr`

When populating the template from extracted decisions:

- **Context**: Extracted from the source document's forces and constraints
- **Decision Drivers**: Extracted from the document's analysis
- **Considered Options**: Populated if the source discusses alternatives
- **Decision**: Stated or implied decision, rewritten in active voice
- **Consequences**: Derived from the document's findings
- **References**: Always link back to the source document

## Important Notes

- Decisions should be **architecturally significant** — not every choice is
  worth an ADR. Help the user distinguish between significant decisions and
  routine implementation choices.
- Extracted ADRs always start with status `proposed` — extraction is discovery,
  not acceptance.
- Preserve the source document's context faithfully — don't invent rationale
  that wasn't in the original.
- Cross-reference bidirectionally: the ADR references the source, and the user
  may wish to note the ADR in the source document.
- When extracting from a single document, multiple decisions may be found —
  each becomes a separate ADR.
- Use sequential numbering: determine the starting number once, then increment
  for batch creation.

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh extract-adrs`
