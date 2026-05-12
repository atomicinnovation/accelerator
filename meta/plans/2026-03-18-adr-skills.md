# ADR Skills Implementation Plan

## Overview

Add architecture decision record (ADR) management to the accelerator plugin via
three new skills (`create-adr`, `extract-adrs`, `review-adr`) under
`skills/decisions/`, with two companion scripts and supporting registration and
documentation changes. ADRs are stored in `meta/decisions/` using sequential
`ADR-NNNN` numbering with YAML frontmatter, and follow an append-only lifecycle
where only `proposed` ADRs may have their content edited.

Based on: `meta/research/codebase/2026-03-18-adr-support-strategy.md`

## Current State Analysis

- The plugin has five skill categories: `vcs/`, `github/`, `planning/`,
  `research/`, `review/`
- The `documents-locator` agent already references `meta/decisions/` in its
  directory structure listing (line 50) but no files or skills exist for it
- Skills follow a consistent pattern: YAML frontmatter in `SKILL.md`, optional
  companion scripts, interactive workflows with user checkpoints
- Companion scripts use `set -euo pipefail`, resolve paths via
  `BASH_SOURCE[0]`, and source shared utilities from `scripts/vcs-common.sh`
- The `research-metadata.sh` script demonstrates the companion script pattern
  for gathering context at a specific workflow step

### Key Discoveries:

- `agents/documents-locator.md:17-18` — Already lists `meta/decisions/`
- `.claude-plugin/plugin.json:9-16` — Skill registration array needs new entry
- `skills/research/research-codebase/scripts/research-metadata.sh` — Companion
  script pattern to follow
- `scripts/vcs-common.sh` — Shared VCS utility to source for repo detection
- `README.md:73-79` — Meta directory table to update

## Desired End State

After implementation:

1. Three new user-invocable skills exist under `skills/decisions/`:
   - `/accelerator:create-adr` — Interactively creates ADRs in
     `meta/decisions/`
   - `/accelerator:extract-adrs` — Extracts decisions from existing meta
     documents into ADRs
   - `/accelerator:review-adr` — Reviews proposed ADRs and transitions their
     status
2. Two companion scripts exist under `skills/decisions/scripts/`:
   - `adr-next-number.sh` — Outputs the next sequential ADR number
   - `adr-read-status.sh` — Reads an ADR's status from YAML frontmatter
3. ADRs use the template defined in the research document with YAML frontmatter
4. Append-only enforcement: only `proposed` ADRs can have content modified;
   other statuses permit only status transitions
5. Plugin registration and README are updated

### Verification:

- All three skills appear in `/accelerator:` skill list
- `create-adr` produces correctly formatted ADRs in `meta/decisions/`
- `extract-adrs` identifies decisions in meta documents and creates ADRs
- `review-adr` enforces immutability for non-proposed ADRs
- Companion scripts execute correctly and handle edge cases
- `documents-locator` discovers ADRs in `meta/decisions/`

## What We're NOT Doing

- No hook-based immutability enforcement (skill-level enforcement is sufficient)
- No ADR index/table of contents file (discovery via `documents-locator`)
- No multiple ADR template variants (single template, can add variants later)
- No code comment scanning in `extract-adrs` (meta documents only)
- No review lens for ADR quality (direct review logic in `review-adr` skill)
- No changes to existing skills or agents beyond registration

## Implementation Approach

Follow established plugin patterns exactly: YAML frontmatter in SKILL.md files,
companion scripts with defensive error handling, interactive workflows with user
checkpoints, and sub-agent delegation for context gathering. Build scripts first
since all three skills depend on them, then build `create-adr` first since
`extract-adrs` reuses its template and `review-adr` needs ADRs to exist.

---

## Phase 1: Companion Scripts

### Overview

Create the two shell scripts that all three ADR skills will use for number
assignment and status checking.

### Changes Required:

#### 1. ADR Next Number Script

**File**: `skills/decisions/scripts/adr-next-number.sh` (new)

```bash
#!/usr/bin/env bash
set -euo pipefail

# Outputs the next sequential ADR number in NNNN format.
# Scans meta/decisions/ for the highest existing ADR-NNNN number
# and increments by one. Outputs "0001" if no ADRs exist.
#
# Usage: adr-next-number.sh [--count N]
#   --count N  Output N sequential numbers, one per line (default: 1)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Source VCS common for repo root detection
source "$PLUGIN_ROOT/scripts/vcs-common.sh"

COUNT=1
while [ $# -gt 0 ]; do
  case "$1" in
    --count) COUNT="$2"; shift 2 ;;
    *) echo "Usage: adr-next-number.sh [--count N]" >&2; exit 1 ;;
  esac
done

# Validate --count is a positive integer
if ! [[ "$COUNT" =~ ^[1-9][0-9]*$ ]]; then
  echo "Error: --count requires a positive integer, got '$COUNT'" >&2
  exit 1
fi

REPO_ROOT=$(find_repo_root) || REPO_ROOT="$PWD"
DECISIONS_DIR="$REPO_ROOT/meta/decisions"

if [ ! -d "$DECISIONS_DIR" ]; then
  for ((i = 1; i <= COUNT; i++)); do
    printf "%04d\n" "$i"
  done
  exit 0
fi

# Find highest ADR number using glob (avoids fragile ls parsing)
HIGHEST=0
for f in "$DECISIONS_DIR"/ADR-[0-9][0-9][0-9][0-9]*; do
  [ -e "$f" ] || continue
  BASE=$(basename "$f")
  NUM=$(echo "$BASE" | sed 's/^ADR-//' | grep -oE '^[0-9]+')
  if [ -n "$NUM" ] && [ "$((10#$NUM))" -gt "$HIGHEST" ]; then
    HIGHEST=$((10#$NUM))
  fi
done

for ((i = 1; i <= COUNT; i++)); do
  printf "%04d\n" "$((HIGHEST + i))"
done
```

#### 2. ADR Read Status Script

**File**: `skills/decisions/scripts/adr-read-status.sh` (new)

```bash
#!/usr/bin/env bash
set -euo pipefail

# Reads the status field from an ADR file's YAML frontmatter.
# Usage: adr-read-status.sh <path-to-adr-file>
# Outputs the status value (e.g., "proposed", "accepted").
# Exits with code 1 if file not found or no valid frontmatter.

if [ $# -lt 1 ]; then
  echo "Usage: adr-read-status.sh <adr-file-path>" >&2
  exit 1
fi

ADR_FILE="$1"

if [ ! -f "$ADR_FILE" ]; then
  echo "Error: File not found: $ADR_FILE" >&2
  exit 1
fi

# Extract YAML frontmatter (between first two --- lines)
# and find the status field
IN_FRONTMATTER=false
while IFS= read -r line; do
  if [ "$line" = "---" ]; then
    if [ "$IN_FRONTMATTER" = true ]; then
      break
    fi
    IN_FRONTMATTER=true
    continue
  fi
  if [ "$IN_FRONTMATTER" = true ]; then
    if echo "$line" | grep -qE '^status:'; then
      # Strip key, whitespace, and optional quotes
      VALUE=$(echo "$line" | sed 's/^status:[[:space:]]*//' | sed 's/^["'"'"']//; s/["'"'"']$//' | sed 's/[[:space:]]*$//')
      echo "$VALUE"
      exit 0
    fi
  fi
done < "$ADR_FILE"

echo "Error: No status field found in frontmatter of $(basename "$ADR_FILE")." >&2
echo "Expected YAML frontmatter with 'status: proposed|accepted|rejected|superseded|deprecated' between --- delimiters." >&2
exit 1
```

#### 3. Test Script

**File**: `skills/decisions/scripts/test-adr-scripts.sh` (new)

A test harness that creates temporary fixtures and asserts expected behaviour.
Run via `bash skills/decisions/scripts/test-adr-scripts.sh`.

Test cases for `adr-next-number.sh`:

- No `meta/decisions/` directory → outputs `0001`
- Empty `meta/decisions/` directory → outputs `0001`
- Directory with `ADR-0003-foo.md` → outputs `0004`
- Directory with gaps (`ADR-0001`, `ADR-0005`) → outputs `0006` (uses highest)
- Directory with non-ADR files (`README.md`, `DRAFT-notes.md`) → ignores them
- Directory with mixed ADR and non-ADR files → outputs correct next number
- `--count 3` with highest `ADR-0002` → outputs `0003`, `0004`, `0005`
- `--count 0` → exits 1 with error (not a positive integer)
- `--count abc` → exits 1 with error (not a positive integer)
- ADR-9999 overflow → outputs `10000` (5 digits, no error)

Test cases for `adr-read-status.sh`:

- Valid frontmatter `status: proposed` → outputs `proposed`
- Valid frontmatter `status: accepted` → outputs `accepted`
- Quoted value `status: "proposed"` → outputs `proposed` (strips quotes)
- No space `status:proposed` → outputs `proposed`
- Trailing whitespace `status: proposed  ` → outputs `proposed` (stripped)
- Missing file → exits 1 with error
- File with no frontmatter → exits 1 with helpful error message
- File with unclosed frontmatter (single `---`) → exits 1
- Status-like line in body (after frontmatter) → returns frontmatter value,
  ignores body
- Empty status value `status: ` → outputs empty string (caller handles)
- No arguments → exits 1 with usage message

### Success Criteria:

#### Automated Verification:

- [x] Test script passes: `bash skills/decisions/scripts/test-adr-scripts.sh`
- [x] Both scripts are executable: `ls -l skills/decisions/scripts/*.sh`

#### Manual Verification:

- [x] Review test script output for any unexpected failures

---

## Phase 2: `create-adr` Skill

### Overview

Create the interactive ADR generation skill that guides users through creating
architecture decision records.

### Changes Required:

#### 1. Skill Definition

**File**: `skills/decisions/create-adr/SKILL.md` (new)

```markdown
---
name: create-adr
description: Interactively create an architecture decision record (ADR). Use
  when the user wants to document an architectural decision, technology choice,
  or significant design decision. Guides through context gathering, options
  analysis, and consequence documentation.
argument-hint: "[topic or description] [--supersedes ADR-NNNN]"
disable-model-invocation: true
---

# Create Architecture Decision Record

You are tasked with guiding the user through creating an architecture decision
record (ADR) — a concise document capturing a significant architectural
decision, its context, the options considered, and the consequences.

## Initial Setup

When this command is invoked:

1. **Check if parameters were provided**:

- If a topic/description was provided, proceed directly to context gathering
- If `--supersedes ADR-NNNN` was provided, note the supersession target
- If no parameters provided, respond with:

```
I'll help you create an architecture decision record. Please provide:
1. The topic or decision to document (e.g., "use PostgreSQL for user data")
2. Any relevant context or constraints

You can also specify if this supersedes an existing ADR:
`/accelerator:create-adr use Redis for caching --supersedes ADR-0003`
```

Then wait for the user's input.

## Process Steps

### Step 1: Determine ADR Number

1. Run the companion script to get the next ADR number:

```
${CLAUDE_PLUGIN_ROOT}/skills/decisions/scripts/adr-next-number.sh
```

2. If `--supersedes ADR-NNNN` was specified:
   - Find the target ADR file by matching `meta/decisions/ADR-NNNN-*.md`
   - Verify exactly one file matches the glob pattern (error if zero or
     multiple matches)
   - Read the target ADR's status using the companion script:
     ```
     ${CLAUDE_PLUGIN_ROOT}/skills/decisions/scripts/adr-read-status.sh <resolved-path>
     ```
   - Verify the target ADR is in `accepted` status (only accepted ADRs can be
     superseded). This is an early-fail check to avoid wasted effort — the
     status will be re-verified before writing in Step 4.
   - If not `accepted`, inform the user and ask how to proceed

### Step 2: Gather Context

1. **Spawn agents to gather relevant context** (in parallel):

- Use **documents-locator** to find related research, plans, and existing ADRs
  in `meta/`
- Use **codebase-locator** to find relevant code related to the decision topic

2. **Read any directly mentioned files** fully

3. **Present gathered context and ask clarifying questions**:

```
Based on my research, here's what I found relevant to this decision:

**Related documents:**
- [list of relevant meta documents]

**Related code:**
- [list of relevant code areas]

**Existing ADRs on related topics:**
- [list of related ADRs, if any]

Before I draft the ADR, I'd like to understand:
1. What forces or constraints are driving this decision?
2. What alternatives have you considered?
3. Are there specific tradeoffs you want to highlight?
```

If context gathering finds nothing relevant (e.g., first ADR in a new project),
skip the context sections and present:

```
No existing documents or ADRs found related to this topic. I'll draft the ADR
based on the information you provide.

Before I draft, I'd like to understand:
1. What forces or constraints are driving this decision?
2. What alternatives have you considered?
3. Are there specific tradeoffs you want to highlight?
```

Wait for user input before proceeding.

### Step 3: Draft the ADR

1. **Gather metadata** by running:

```
${CLAUDE_PLUGIN_ROOT}/skills/research/research-codebase/scripts/research-metadata.sh
```

2. **Draft the ADR** using the template below and present it to the user for
   review:

```
Here's my draft ADR. Please review and let me know if you'd like any changes
before I write it to disk:

[draft content]
```

Wait for user approval or revision requests before writing.

3. **Iterate** on the draft based on user feedback. Only proceed to writing
   when the user approves.

### Step 4: Write the ADR

1. Create the `meta/decisions/` directory if it doesn't exist

2. Write the ADR to:
   `meta/decisions/ADR-NNNN-description.md`
   where NNNN is the number from Step 1 and description is a kebab-case summary

3. If this supersedes an existing ADR:
   - Read the superseded ADR's current status to confirm it's `accepted`
   - Update ONLY the superseded ADR's frontmatter:
     - Change `status: accepted` to `status: superseded`
     - Add `superseded_by: ADR-MMMM` (where MMMM is the new ADR number)
   - Do NOT modify any other content in the superseded ADR

4. Present the result:

```
ADR created: `meta/decisions/ADR-NNNN-description.md`
Status: proposed

[If supersession]: Updated ADR-XXXX status to "superseded"

Next steps:
- Review and refine while in "proposed" status
- When ready, use `/accelerator:review-adr` to accept or reject
```

## ADR Template

Use this exact template structure when generating ADRs:

```markdown
---
adr_id: ADR-NNNN
date: "YYYY-MM-DDTHH:MM:SS+00:00"
author: Author Name
status: proposed
supersedes: ADR-NNNN     # only include if this ADR replaces another
tags: [tag1, tag2]
---

# ADR-NNNN: Title as Short Noun Phrase

**Date**: YYYY-MM-DD
**Status**: Proposed
**Author**: Author Name

## Context

[Forces at play — technological, political, social, project-specific.
Value-neutral language describing facts, not advocating.]

## Decision Drivers

- [Driver 1]
- [Driver 2]

## Considered Options

1. **Option A** — Brief description
2. **Option B** — Brief description
3. **Option C** — Brief description

## Decision

[The chosen option and why, stated in active voice: "We will..."]

## Consequences

### Positive

- [Consequence 1]

### Negative

- [Consequence 1]

### Neutral

- [Consequence 1]

## References

- `meta/research/codebase/YYYY-MM-DD-topic.md` — Related research
- `meta/decisions/ADR-NNNN.md` — Related/superseded ADR
```

## Quality Guidelines

When drafting ADRs, follow these principles:

- **Concise**: One to two pages maximum. Match length to problem complexity.
- **Assertive**: Use active voice ("We will...", "We chose...")
- **Balanced**: Include genuine pros AND cons. Avoid the "Fairy Tale" pattern
  (only pros, no cons)
- **Honest options**: Only include options that were genuinely considered. Avoid
  "Dummy Alternatives" (non-viable options to make preferred choice look good)
- **Focused**: Each ADR captures ONE decision. If you find multiple decisions,
  suggest creating separate ADRs.
- **Context-rich**: Explain WHY, not just WHAT. Future readers need to
  understand the forces at play.

## Anti-Patterns to Avoid

- **Fairy Tale**: Only listing positive consequences
- **Dummy Alternative**: Including obviously non-viable options
- **Mega-ADR**: Multi-page documents crammed with implementation detail
- **Blueprint in Disguise**: Reads like a cookbook, not a decision journal
- **Missing context**: Decision without the forces that drove it

## Important Notes

- New ADRs ALWAYS start with status `proposed`
- ADR numbers are NEVER reused — always increment from the highest existing
- File naming is `ADR-NNNN-description.md` (e.g., `ADR-0001-use-jujutsu.md`)
- Only modify existing ADRs to update status fields during supersession
- Cross-reference related documents in the References section
- Use `documents-locator` and `codebase-locator` agents for context, not deep
  file reads in the main context
- **Dual status fields**: The template includes status in both YAML frontmatter
  (`status: proposed`) and the body (`**Status**: Proposed`). The frontmatter
  is the authoritative source of truth — `adr-read-status.sh` reads only
  frontmatter. The body line is for human readability. When updating status,
  ALWAYS update both locations.
- Before writing a new ADR file, verify the target path does not already exist
  to prevent accidental overwrites from concurrent invocations
```

### Success Criteria:

#### Automated Verification:

- [x] `skills/decisions/create-adr/SKILL.md` exists with correct frontmatter
- [x] Frontmatter includes `name`, `description`, `argument-hint`,
  `disable-model-invocation`
- [x] Template includes all required sections: Context, Decision Drivers,
  Considered Options, Decision, Consequences, References
- [x] Supersession workflow is documented

#### Manual Verification:

- [ ] Invoke `/accelerator:create-adr "use PostgreSQL for user data"` and
  verify the interactive flow works
- [ ] Verify generated ADR matches the template structure
- [ ] Verify ADR number is correctly assigned
- [ ] Test supersession flow with `--supersedes` argument
- [ ] Verify quality guidelines produce balanced ADRs

---

## Phase 3: `extract-adrs` Skill

### Overview

Create the skill that identifies architectural decisions within existing meta
documents and converts them into formal ADRs.

### Changes Required:

#### 1. Skill Definition

**File**: `skills/decisions/extract-adrs/SKILL.md` (new)

```markdown
---
name: extract-adrs
description: Extract architecture decision records from existing meta documents
  (research, plans). Scans documents for implicit or explicit architectural
  decisions and converts selected ones into formal ADRs. Use when decisions are
  buried in research or planning documents and need to be captured formally.
argument-hint: "[@meta/research/codebase/doc.md ...] or leave empty to scan all"
disable-model-invocation: true
---

# Extract ADRs from Meta Documents

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
1. Specify documents to scan: `/accelerator:extract-adrs @meta/research/codebase/2026-03-18-auth-flow.md`
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
     - `meta/research/codebase/2026-03-18-auth-flow.md` — Authentication flow research
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
   Source: `meta/research/codebase/2026-03-18-topic.md`

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
```

### Success Criteria:

#### Automated Verification:

- [x] `skills/decisions/extract-adrs/SKILL.md` exists with correct frontmatter
- [x] Frontmatter includes `name`, `description`, `argument-hint`,
  `disable-model-invocation`

#### Manual Verification:

- [ ] Invoke `/accelerator:extract-adrs @meta/research/codebase/2026-03-18-adr-support-strategy.md`
  and verify decisions are identified
- [ ] Verify Y-statement summaries are presented for selection
- [ ] Verify generated ADRs contain context from source documents
- [ ] Verify sequential numbering works for batch creation
- [ ] Verify cross-references to source documents are included

---

## Phase 4: `review-adr` Skill

### Overview

Create the skill that reviews ADR quality, enforces the immutability model, and
manages status transitions.

### Changes Required:

#### 1. Skill Definition

**File**: `skills/decisions/review-adr/SKILL.md` (new)

```markdown
---
name: review-adr
description: Review an architecture decision record for quality and
  completeness, then accept, reject, or suggest revisions. Enforces ADR
  immutability — only proposed ADRs can be modified, accepted ADRs can only
  transition to superseded or deprecated. Use when a proposed ADR is ready for
  review, or when an accepted ADR needs to be deprecated.
argument-hint: "[@meta/decisions/ADR-NNNN.md] [--deprecate reason]"
disable-model-invocation: true
---

# Review Architecture Decision Record

You are tasked with reviewing ADRs for quality and managing their lifecycle
status transitions, enforcing the append-only immutability model.

## Initial Setup

When this command is invoked:

1. **Check if parameters were provided**:

- If an ADR path was provided, read it fully and proceed to review
- If `--deprecate` was specified with a path, proceed to deprecation workflow
- If `--deprecate` was specified without a path:
  - Scan `meta/decisions/` for ADRs in `accepted` status
  - Present them for selection:
    ```
    I found the following accepted ADRs available for deprecation:

    1. `meta/decisions/ADR-0001-use-jujutsu.md` — Use Jujutsu for Version
       Control
    2. `meta/decisions/ADR-0002-filesystem-chaining.md` — Filesystem-Mediated
       Skill Chaining

    Which ADR would you like to deprecate? (enter number or path)
    ```
  - After selection, ask for the deprecation reason and proceed to deprecation
    workflow
- If no parameters provided:
  - Scan `meta/decisions/` for ADRs in `proposed` status
  - Present them for selection:

```
I found the following proposed ADRs ready for review:

1. `meta/decisions/ADR-0001-use-jujutsu.md` — Use Jujutsu for Version Control
2. `meta/decisions/ADR-0003-append-only-lifecycle.md` — Append-Only ADR
   Lifecycle

Which ADR would you like to review? (enter number or path)

Tip: To deprecate an accepted ADR, use:
`/accelerator:review-adr --deprecate`
```

Wait for user selection.

## Mutability Rules

Before making ANY changes to an ADR, check its status using:

```
${CLAUDE_PLUGIN_ROOT}/skills/decisions/scripts/adr-read-status.sh <adr-path>
```

Then apply these rules:

| Current Status | Content Editable? | Permitted Transitions                          |
|----------------|-------------------|------------------------------------------------|
| `proposed`     | Yes               | → `accepted`, → `rejected`                     |
| `accepted`     | No                | → `superseded` (via create-adr), → `deprecated`|
| `rejected`     | No                | None (terminal)                                |
| `superseded`   | No                | None (terminal)                                |
| `deprecated`   | No                | None (terminal)                                |

**CRITICAL**: If the ADR status is anything other than `proposed`, you MUST NOT
modify its content. You may only update the status field and related metadata
fields (`superseded_by`, `deprecated_reason`) for permitted transitions.

If the user asks to modify a non-proposed ADR's content, explain:

```
This ADR has status "[status]" and is immutable. Its content cannot be changed.

To revise this decision, create a new ADR that supersedes it:
`/accelerator:create-adr [new decision] --supersedes ADR-NNNN`
```

## Process Steps

### For Proposed ADRs — Quality Review

#### Step 1: Read and Analyse

1. Read the ADR fully
2. Check status is `proposed` (if not, apply mutability rules above)
3. Spawn agents for context (in parallel):
   - **documents-locator** to find related ADRs and source documents
   - **codebase-locator** to verify technical claims in the ADR

#### Step 2: Quality Review

Evaluate the ADR against these quality criteria:

**Context Completeness:**
- Are the forces and constraints clearly described?
- Is the context value-neutral (describing facts, not advocating)?
- Would a new team member understand WHY this decision matters?

**Decision Drivers:**
- Are the key drivers listed?
- Do they connect logically to the decision?

**Options Analysis:**
- Are the considered options genuinely viable? (No "Dummy Alternatives")
- Are there obvious options missing that should be considered?
- Is each option described clearly enough to understand?

**Decision Statement:**
- Is the decision clear and assertive? ("We will..." not "We might...")
- Does it follow from the context and drivers?
- Is it specific enough to be actionable?

**Consequences:**
- Are there both positive AND negative consequences? (No "Fairy Tale")
- Are consequences honest and realistic?
- Are neutral consequences included where relevant?

**Overall Quality:**
- Is the length appropriate for the complexity of the decision?
- Is it a decision record, not a blueprint or cookbook?
- Are references to related documents included?

#### Step 3: Present Review

```
## ADR Review: ADR-NNNN — [Title]

**Overall Assessment**: [Strong / Adequate / Needs Revision]

### Strengths:
- [What's done well]

### Suggestions:
- [Specific improvements, if any]

### Issues:
- [Problems that should be fixed before acceptance, if any]

---

**Actions available:**
1. **Accept** — Mark as accepted (ADR becomes immutable)
2. **Reject** — Mark as rejected with reason (ADR becomes immutable)
3. **Revise** — Make suggested improvements (stays proposed)

What would you like to do?
```

Wait for user decision.

#### Step 4: Execute Action

**If Accept:**
1. Update the ADR's frontmatter:
   - Change `status: proposed` to `status: accepted`
2. Update the in-body status line:
   - Change `**Status**: Proposed` to `**Status**: Accepted`
3. Confirm:
   ```
   ADR-NNNN has been accepted. It is now immutable — content can no longer be
   changed. If this decision needs to be revised in the future, create a new
   ADR that supersedes it.
   ```

**If Reject:**
1. Ask for a rejection reason
2. Update the ADR's frontmatter:
   - Change `status: proposed` to `status: rejected`
   - Add `rejected_reason: "[reason]"`
3. Update the in-body status line:
   - Change `**Status**: Proposed` to `**Status**: Rejected`
4. Confirm:
   ```
   ADR-NNNN has been rejected. Reason: [reason]
   ```

**If Revise:**
1. Make the suggested improvements to the ADR content
2. Present the updated ADR for another round of review
3. Return to Step 2

### For Accepted ADRs — Deprecation Only

If the ADR is in `accepted` status and `--deprecate` was specified:

1. Ask for or confirm the deprecation reason
2. Update the ADR's frontmatter:
   - Change `status: accepted` to `status: deprecated`
   - Add `deprecated_reason: "[reason]"`
3. Update the in-body status line:
   - Change `**Status**: Accepted` to `**Status**: Deprecated`
4. Confirm:
   ```
   ADR-NNNN has been deprecated. Reason: [reason]

   Note: The ADR content remains unchanged. If a replacement decision is
   needed, use `/accelerator:create-adr` to create a new ADR.
   ```

### For Terminal Status ADRs

If the ADR is in `rejected`, `superseded`, or `deprecated` status:

```
ADR-NNNN is in "[status]" status, which is terminal. No further changes are
permitted.

[If superseded]: This ADR was superseded by ADR-MMMM. See
`meta/decisions/ADR-MMMM-description.md`.
[If deprecated]: Reason: [deprecated_reason from frontmatter]
[If rejected]: Reason: [rejected_reason from frontmatter]
```

## Important Notes

- **ALWAYS** check status before any modification — this is the core
  enforcement mechanism
- Never modify content of non-proposed ADRs, only status metadata
- Supersession is handled by `create-adr` with `--supersedes`, not by this
  skill
- Be constructive in reviews — suggest specific improvements, not vague
  criticism
- Quality review is a service, not a gate — the user decides whether to accept
- **Enforcement boundary**: Immutability is enforced at the skill level, not
  the filesystem level. Direct file edits outside of ADR skills bypass this
  protection. This is an accepted tradeoff — skill-level enforcement is
  simpler and sufficient for the intended workflow. If an ADR appears to have
  been modified outside the skills, note this to the user during review.
```

### Success Criteria:

#### Automated Verification:

- [x] `skills/decisions/review-adr/SKILL.md` exists with correct frontmatter
- [x] Frontmatter includes `name`, `description`, `argument-hint`,
  `disable-model-invocation`
- [x] Mutability rules table is present and complete

#### Manual Verification:

- [ ] Invoke `/accelerator:review-adr` and verify it lists proposed ADRs
- [ ] Review a proposed ADR and verify quality criteria are applied
- [ ] Accept an ADR and verify status changes correctly
- [ ] Reject an ADR and verify status and reason are recorded
- [ ] Attempt to modify an accepted ADR and verify immutability is enforced
- [ ] Deprecate an accepted ADR and verify status transition
- [ ] Attempt to modify a rejected/superseded/deprecated ADR and verify
  terminal status is enforced

---

## Phase 5: Plugin Registration & Documentation

### Overview

Register the new skill category and update documentation.

### Changes Required:

#### 1. Plugin Registration

**File**: `.claude-plugin/plugin.json`
**Changes**: Add `./skills/decisions/` to the skills array

```json
{
  "name": "accelerator",
  "version": "1.3.0",
  "description": "Development acceleration toolkit with multi-lens code review, implementation planning, codebase research, architecture decision records, and git workflow automation.",
  "author": {
    "name": "Toby Clemson",
    "email": "toby@go-atomic.io"
  },
  "skills": [
    "./skills/vcs/",
    "./skills/github/",
    "./skills/planning/",
    "./skills/research/",
    "./skills/decisions/",
    "./skills/review/lenses/",
    "./skills/review/output-formats/"
  ]
}
```

#### 2. CHANGELOG Update

**File**: `CHANGELOG.md`
**Changes**: Add a new version entry

```markdown
## 1.3.0 — YYYY-MM-DD

_Versions 1.1.0–1.2.1 added VCS detection, jujutsu support, and bug fixes
but were not recorded in the changelog._

### Added

- **Architecture Decision Records (ADRs)**: New `decisions/` skill category
  with three skills for managing architectural decisions
  - `create-adr` — Interactively create ADRs with context gathering and
    quality guidelines
  - `extract-adrs` — Extract decisions from existing research and planning
    documents into formal ADRs
  - `review-adr` — Review proposed ADRs for quality; accept, reject, or
    deprecate with append-only lifecycle enforcement
- Companion scripts `adr-next-number.sh` and `adr-read-status.sh` for ADR
  number assignment and status checking
- `meta/decisions/` directory for storing ADRs with sequential `ADR-NNNN`
  numbering
```

#### 3. README Update

**File**: `README.md`
**Changes**: Add decisions directory to meta directory table and add ADR
workflow section

Add to the meta directory table (after `plans/` row):

```markdown
| `decisions/` | Architecture decision records (ADRs)    | `create-adr`, `extract-adrs`, `review-adr` |
```

Add a new section after "The Development Loop" and before "PR Workflow Skills":

```markdown
## Architecture Decision Records

ADR skills capture architectural decisions that emerge from research and
planning:

```
research-codebase → create-plan → implement-plan
       ↓                ↓
  meta/research/codebase/    meta/plans/
       ↓                ↓
  extract-adrs ←────────┘
       ↓
  meta/decisions/
       ↓
  review-adr → accepted ADRs inform future research & planning
```

| Skill            | Usage                                          | Description                                                    |
|------------------|------------------------------------------------|----------------------------------------------------------------|
| **create-adr**   | `/accelerator:create-adr [topic]`              | Interactively create an ADR with context gathering             |
| **extract-adrs** | `/accelerator:extract-adrs [@meta/doc.md ...]` | Extract decisions from existing meta documents into ADRs       |
| **review-adr**   | `/accelerator:review-adr [@meta/decisions/ADR-NNNN.md]` | Review proposed ADRs; accept, reject, or suggest revisions |

ADRs follow an append-only lifecycle: once accepted, an ADR's content becomes
immutable. To revise a decision, create a new ADR that supersedes the original.
```

### Success Criteria:

#### Automated Verification:

- [x] `plugin.json` contains `"./skills/decisions/"` in skills array
- [x] `README.md` contains `decisions/` in meta directory table
- [x] `README.md` contains ADR workflow section

#### Manual Verification:

- [ ] All three skills appear when listing available accelerator skills
- [ ] `documents-locator` discovers ADRs placed in `meta/decisions/`

---

## Testing Strategy

### End-to-End Workflow Tests:

1. **Create**: `/accelerator:create-adr "use jujutsu for version control"`
   → verify ADR-0001 is created with proposed status
2. **Review + Accept**: `/accelerator:review-adr @meta/decisions/ADR-0001-*.md`
   → review, accept → verify immutability
3. **Supersede**: `/accelerator:create-adr "use git instead" --supersedes ADR-0001`
   → verify ADR-0002 created, ADR-0001 status changed to superseded
4. **Extract**: `/accelerator:extract-adrs @meta/research/codebase/2026-03-18-adr-support-strategy.md`
   → verify decisions are identified and ADRs generated
5. **Deprecate**: `/accelerator:review-adr @meta/decisions/ADR-0002.md --deprecate "no longer relevant"`
   → verify deprecation
6. **Immutability**: Attempt to modify accepted/rejected/superseded/deprecated
   ADR content → verify refusal

### Negative Path Tests:

1. `create-adr` when `adr-next-number.sh` fails (permissions) → clear error
2. Superseding a proposed ADR (not yet accepted) → rejected with guidance
3. Superseding an already-superseded ADR → rejected with guidance
4. `extract-adrs` on a document with no identifiable decisions → informative
   message, no empty ADRs created
5. `review-adr` when no proposed ADRs exist → informative message
6. `review-adr` attempting content modification on accepted ADR → refused with
   supersession guidance

### Edge Cases (assigned to phases):

- First ADR ever, no `meta/decisions/` directory (Phase 1: tested by
  `test-adr-scripts.sh`; Phase 2: create-adr creates directory)
- Non-sequential ADR numbers / gaps from deleted files (Phase 1: tested by
  `test-adr-scripts.sh`)
- Multiple ADRs extracted in batch (Phase 3: manual verification)
- Superseding a non-accepted ADR (Phase 2: manual verification)
- ADR with missing or malformed frontmatter (Phase 1: tested by
  `test-adr-scripts.sh`)
- Concurrent invocations assigning same number (Phase 2: accepted risk for
  single-user Claude Code usage; mitigated by file-existence check before
  writing)

## References

- `meta/research/codebase/2026-03-18-adr-support-strategy.md` — Full research on ADR
  strategy
- `skills/research/research-codebase/SKILL.md` — Pattern for skill with
  companion scripts and interactive workflow
- `skills/research/research-codebase/scripts/research-metadata.sh` — Companion
  script pattern
- `.claude-plugin/plugin.json:9-16` — Skill registration
- `agents/documents-locator.md:17-18` — Already references `meta/decisions/`
- `README.md:73-79` — Meta directory documentation
