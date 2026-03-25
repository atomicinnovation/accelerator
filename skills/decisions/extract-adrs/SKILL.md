---
name: extract-adrs
description: Extract architecture decision records from existing meta documents
  (research, plans). Scans documents for implicit or explicit architectural
  decisions and converts selected ones into formal ADRs. Use when decisions are
  buried in research or planning documents and need to be captured formally.
argument-hint: "[@meta/research/doc.md ...] or leave empty to scan all"
disable-model-invocation: true
---

# Extract ADRs from Meta Documents

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`

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
1. Specify documents to scan: `/accelerator:extract-adrs @meta/research/2026-03-18-auth-flow.md`
2. Let me scan all meta documents for decisions (this may take a moment)

Which would you prefer?
```

Wait for user input.

## Process Steps

### Step 1: Identify Source Documents

1. If specific files were provided, read them FULLY
2. If scanning all meta documents:
   - Spawn a **documents-locator** agent to find all documents in `meta/`
     (research, plans, decisions)
   - Present the discovered documents and let the user select which to scan:
     ```
     I found the following documents in meta/:

     **Research:**
     - `meta/research/2026-03-18-auth-flow.md` — Authentication flow research
     - ...

     **Plans:**
     - `meta/plans/2026-03-18-api-redesign.md` — API redesign plan
     - ...

     Which documents should I scan for decisions? (enter numbers, "all", or
     specific paths)
     ```
   - Wait for user selection

### Step 2: Analyse Documents for Decisions

1. **Spawn documents-analyser agents** (one per document, in parallel) with
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
   Source: `meta/research/2026-03-18-topic.md`

2. **[Short title]** — In the context of [X], facing [Y], we decided for [Z]
   to achieve [Q], accepting [D].
   Source: `meta/plans/2026-03-18-topic.md`

3. ...

Which decisions would you like to capture as ADRs? (enter numbers, "all",
or "none")
```

Wait for user selection.

### Step 3: Generate ADRs

1. **Gather metadata** by running:
   ```
   ${CLAUDE_PLUGIN_ROOT}/skills/research/research-codebase/scripts/research-metadata.sh
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

1. Create `meta/decisions/` directory if it doesn't exist

2. Write each approved ADR to:
   `meta/decisions/ADR-NNNN-description.md`

3. Present summary:
   ```
   Created the following ADRs:
   - `meta/decisions/ADR-0001-description.md` — [title]
   - `meta/decisions/ADR-0002-description.md` — [title]
   - ...

   All ADRs are in "proposed" status. Use `/accelerator:review-adr` to
   review and accept them.
   ```

## ADR Template

Use the template exactly as defined in the `create-adr` skill
(`skills/decisions/create-adr/SKILL.md`, under the "ADR Template" section).
The single authoritative template lives there to avoid duplication and drift.

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
