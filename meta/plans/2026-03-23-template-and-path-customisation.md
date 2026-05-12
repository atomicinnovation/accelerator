# Template & Path Customisation Implementation Plan

## Overview

Add configuration support for overriding output paths (where skills write
documents) and document templates (the structure of generated plans, research
documents, ADRs, and review output). These are the least frequently changed
configuration options but provide important flexibility for teams with
different conventions.

**Depends on**: Plan 1 (Configuration Infrastructure) must be complete first.

## Current State Analysis

Skills write documents to hardcoded paths under `meta/`:

| Skill | Output Path | Template |
|-------|-------------|----------|
| `create-plan` | `meta/plans/YYYY-MM-DD-*.md` | Embedded (lines 209-310) |
| `research-codebase` | `meta/research/codebase/YYYY-MM-DD-*.md` | Embedded (lines 110-166) |
| `create-adr` | `meta/decisions/ADR-NNNN-*.md` | Embedded (lines 161-215) |
| `extract-adrs` | `meta/decisions/ADR-NNNN-*.md` | Delegates to `create-adr` template |
| `review-adr` | Edits existing ADRs | N/A |
| `describe-pr` | `meta/prs/{number}-description.md` | User-provided at `meta/templates/pr-description.md` |
| `validate-plan` | `meta/validations/YYYY-MM-DD-*.md` | Embedded (lines 102-150 report template, 170-181 frontmatter wrapper) |
| `review-plan` | `meta/reviews/plans/*-review-*.md` | Embedded (lines 378-408) |
| `review-pr` | `meta/reviews/prs/pr-review-{number}/` | Temp working dir (not an output template) |
| `create-plan` | References `meta/tickets/` | N/A (context source, not output) |
| `adr-next-number.sh` | Scans `meta/decisions/` | N/A |

Additional `meta/` paths referenced but not as skill outputs:
- `meta/templates/pr-description.md` — user-provided PR description template
- `meta/tickets/` — ticket files referenced by `create-plan`
- `meta/notes/` — notes directory

### Key Discoveries:

- Templates are deeply embedded in skill instructions as inline markdown code
  blocks. They can't be extracted without restructuring the skills.
- The `describe-pr` skill already has a user-override pattern: it reads
  `meta/templates/pr-description.md` from the user's repo. We should follow
  this pattern.
- `adr-next-number.sh:32` hardcodes `$REPO_ROOT/meta/decisions` — this must
  also be updated for path overrides.
- Several skills cross-reference each other's paths:
  - `create-plan:308` references `meta/research/codebase/` in its template
  - `validate-plan:160` derives its output path from the plan's path
  - `review-plan:362-364` derives review path from plan stem
  - `extract-adrs:105` uses `research-metadata.sh` for metadata
- The `meta/tmp/` path in `review-pr` is a working directory, not a document
  output. It's less important to override but included for completeness.
- Output format skills (`pr-review-output-format`, `plan-review-output-format`)
  are JSON schemas read by reviewer agents — overriding them is an advanced
  use case that is deferred.

## Desired End State

After this plan:
1. Users can override the base paths where skills write documents via config.
2. Users can provide custom templates for plans, research documents, and ADRs
   by placing template files in conventional locations.
3. Path overrides propagate to all scripts and skills that reference those paths.
4. Template overrides are full-file replacements — no section-level merging.
5. When no overrides are configured, behaviour is identical to today.

## What We're NOT Doing

- Section-level template overrides (replace entire file only)
- Output format schema overrides (pr-review-output-format,
  plan-review-output-format) — these are structural contracts between the
  reviewer agent and orchestrator
- File naming convention overrides (e.g., changing `ADR-NNNN` to
  `DEC-NNNN`) — naming patterns are deeply coupled to scripts
- PR description template content or structure — it remains a user-provided
  template. However, its location is now controlled by `paths.templates`
  (defaults to `meta/templates`), consolidating all templates in one directory.

## Implementation Approach

**Path overrides**: A `config-read-path.sh` script reads path config and
outputs the configured value or default. Skills and scripts use this to
determine output directories. Since paths are referenced in both skill text and
shell scripts, the override must work in both contexts.

**Template overrides**: A `config-read-template.sh` script checks for a user
template at a conventional location, falls back to an extracted default template
in the plugin's `templates/` directory, and outputs the template content. Skills
use the `!`command`` preprocessor to inject the template instead of embedding
it.

## Phase 1: Path Override Script

### Overview

Create a script that reads path overrides from config and outputs the
configured path or default.

### Changes Required:

#### 1. Path Reader Script

**File**: `scripts/config-read-path.sh`
**Changes**: New file.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Reads a path configuration value.
# Usage: config-read-path.sh <path_key> [default]
#
# Path keys:
#   plans         → where plans are written (default: meta/plans)
#   research      → where research docs are written (default: meta/research/codebase)
#   decisions     → where ADRs are written (default: meta/decisions)
#   prs           → where PR descriptions are written (default: meta/prs)
#   validations   → where validation reports are written (default: meta/validations)
#   review_plans  → where plan reviews are written (default: meta/reviews/plans)
#   review_prs    → where PR review working dirs go (default: meta/reviews/prs)
#   templates     → where user templates are found (default: meta/templates)
#   tickets       → where ticket files are stored (default: meta/tickets)
#   notes         → where notes are stored (default: meta/notes)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Delegate to config-read-value.sh with paths. prefix
exec "$SCRIPT_DIR/config-read-value.sh" "paths.${1:-}" "${2:-}"
```

A thin wrapper around `config-read-value.sh` with the `paths.` prefix. This keeps
the interface clean for callers.

#### 2. Test Updates

**File**: `scripts/test-config.sh`
**Changes**: Add test cases for `config-read-path.sh`:
- No paths config → outputs default
- `paths.plans: docs/plans` → outputs `docs/plans`
- `paths.decisions: docs/adrs` → outputs `docs/adrs`
- `paths.review_plans: docs/reviews/plans` → outputs `docs/reviews/plans`
- `paths.review_prs: docs/reviews/prs` → outputs `docs/reviews/prs`
- `paths.templates: docs/templates` → outputs `docs/templates`
- `paths.tickets: docs/tickets` → outputs `docs/tickets`
- `paths.notes: docs/notes` → outputs `docs/notes`
- Absolute path `/opt/docs/plans` → outputs `/opt/docs/plans`
- Local overrides team for paths

### Success Criteria:

#### Automated Verification:

- [x] `scripts/config-read-path.sh` exists and is executable
- [x] `bash scripts/test-config.sh` passes all tests

---

## Phase 2: Extract Default Templates

### Overview

Extract the hardcoded templates from skills into standalone template files in
the plugin's `templates/` directory. The original skills will use the
`!`command`` preprocessor to inject templates instead of embedding them.

### Changes Required:

#### 1. Plan Template

**File**: `templates/plan.md`
**Changes**: New file. Extract the plan template from
`skills/planning/create-plan/SKILL.md:209-310`.

This is the template structure currently embedded in the create-plan skill,
starting with `# [Feature/Task Name] Implementation Plan` and ending with
the References section.

#### 2. Research Template

**File**: `templates/research.md`
**Changes**: New file. Extract the research document template from
`skills/research/research-codebase/SKILL.md:110-166`.

This is the template structure with YAML frontmatter fields and body sections
(Research Question through Open Questions).

#### 3. ADR Template

**File**: `templates/adr.md`
**Changes**: New file. Extract the ADR template from
`skills/decisions/create-adr/SKILL.md:161-215`.

This is the template with YAML frontmatter (adr_id, date, author, status,
etc.) and body sections (Context through References).

#### 4. Validation Template

**File**: `templates/validation.md`
**Changes**: New file. Extract the validation report template from
`skills/planning/validate-plan/SKILL.md:102-150`.

**Important**: Extracted template files should contain the raw template
content WITHOUT code fence delimiters. The `config-read-template.sh` script
(Phase 3) wraps template output in code fences automatically. This means
user-provided custom templates also don't need code fences — the script
handles it uniformly. If a template file already starts with a code fence
line, the script detects this and outputs it as-is (no double-wrapping).

**Cross-reference limitation**: Default templates contain hardcoded
cross-references to other skills' paths (e.g., the plan template references
`meta/research/codebase/` in its References section, the validation template references
`meta/plans/` in its frontmatter `target` field). These cross-references are
NOT dynamically resolved when paths are overridden. Users who override paths
should also provide custom templates with updated cross-references. This
limitation should be documented in the configure skill's help text (Phase 5).

### Success Criteria:

#### Automated Verification:

- [x] `templates/plan.md` exists
- [x] `templates/research.md` exists
- [x] `templates/adr.md` exists
- [x] `templates/validation.md` exists
- [x] Each template file does NOT include code fence delimiters (the script
  wraps them)
- [x] `templates/plan.md` contains `## Overview` and `## Phase` headings
- [x] `templates/research.md` contains `## Research Question` and YAML
  frontmatter with `topic` field
- [x] `templates/adr.md` contains `adr_id` and `## Context` heading
- [x] `templates/validation.md` contains `### Implementation Status` heading
- [x] Each template's line count is within reasonable range of the original
  embedded block (guards against partial extraction)

---

## Phase 3: Template Reader Script

### Overview

Create a script that checks for user template overrides and falls back to
plugin defaults.

### Changes Required:

#### 1. Template Reader Script

**File**: `scripts/config-read-template.sh`
**Changes**: New file.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Reads a template file, checking user overrides before plugin defaults.
# Usage: config-read-template.sh <template_name>
#
# Template names: plan, research, adr, validation
# (Invalid names produce an error listing available templates.)
#
# Resolution order:
# 1. Path specified in config: templates.<name> (if set and file exists)
# 2. Configured templates directory: <paths.templates>/<name>.md
#    (defaults to meta/templates/<name>.md)
# 3. Plugin default: <plugin_root>/templates/<name>.md
#
# All user-facing templates live in one place (meta/templates/ or whatever
# paths.templates is set to). The .claude/accelerator/ directory is only
# used for custom lenses (Plan 3), not templates.
#
# Outputs the template content to stdout, wrapped in markdown code fences
# (```markdown ... ```) so the LLM interprets the content as a template to
# follow rather than instructions to execute. If the template file already
# starts with a code fence, it is output as-is (no double-wrapping).

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

TEMPLATE_NAME="${1:-}"
if [ -z "$TEMPLATE_NAME" ]; then
  echo "Usage: config-read-template.sh <template_name>" >&2
  exit 1
fi

# Output template content, wrapping in code fences if not already fenced.
_output_template() {
  local file="$1"
  local first_line
  first_line=$(head -1 "$file")
  if [[ "$first_line" == '```'* ]]; then
    # Already fenced — output as-is
    cat "$file"
  else
    # Wrap in code fences
    echo '```markdown'
    cat "$file"
    echo '```'
  fi
}

PROJECT_ROOT=$(config_project_root)

# 1. Check config-specified path
CONFIG_PATH=$("$SCRIPT_DIR/config-read-value.sh" "templates.${TEMPLATE_NAME}" "")
if [ -n "$CONFIG_PATH" ]; then
  # Resolve relative to project root
  if [[ "$CONFIG_PATH" != /* ]]; then
    CONFIG_PATH="$PROJECT_ROOT/$CONFIG_PATH"
  fi
  if [ -f "$CONFIG_PATH" ]; then
    _output_template "$CONFIG_PATH"
    exit 0
  else
    echo "Warning: configured template path '$CONFIG_PATH' not found, falling back to defaults" >&2
  fi
fi

# 2. Check configured templates directory (paths.templates, default: meta/templates)
TEMPLATES_DIR=$("$SCRIPT_DIR/config-read-path.sh" templates meta/templates)
if [[ "$TEMPLATES_DIR" != /* ]]; then
  TEMPLATES_DIR="$PROJECT_ROOT/$TEMPLATES_DIR"
fi
if [ -f "$TEMPLATES_DIR/${TEMPLATE_NAME}.md" ]; then
  _output_template "$TEMPLATES_DIR/${TEMPLATE_NAME}.md"
  exit 0
fi

# 3. Fall back to plugin default
DEFAULT_PATH="$PLUGIN_ROOT/templates/${TEMPLATE_NAME}.md"
if [ -f "$DEFAULT_PATH" ]; then
  _output_template "$DEFAULT_PATH"
  exit 0
fi

echo "Error: Template '$TEMPLATE_NAME' not found. Available templates: plan, research, adr, validation" >&2
exit 1
```

#### 2. Test Updates

**File**: `scripts/test-config.sh`
**Changes**: Add test cases:
- No user template → outputs plugin default wrapped in code fences
- Template in configured templates directory (`paths.templates` / default
  `meta/templates/`) → outputs user template wrapped in code fences
- `paths.templates` overridden → looks in overridden directory
- Config path specified (`templates.<name>`) and exists → outputs that file
  wrapped in code fences (takes precedence over templates directory)
- Template file already starts with code fence → output as-is (no
  double-wrapping)
- Config path specified but missing → falls back to templates directory,
  then plugin default; warning emitted to stderr
- Config path specified as relative → resolved against project root
- Config path specified as absolute → used as-is
- Unknown template name → error listing available template names

### Success Criteria:

#### Automated Verification:

- [x] `scripts/config-read-template.sh` exists and is executable
- [ ] `bash scripts/test-config.sh` passes all tests

---

## Phase 4: Update Skills to Use Templates and Paths

### Overview

Update skills to use the template reader and path reader instead of hardcoded
values.

**Path injection approach**: Unlike `config-read-context.sh` and
`config-read-agents.sh` (which output structured markdown sections), path
preprocessor calls output bare path strings. To ensure the LLM reliably
associates the path value with the output directory instruction, each skill
should wrap the preprocessor call within the instruction text itself, e.g.:

```markdown
1. **Write the plan** to
   `!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans meta/plans``
   using the filename format `YYYY-MM-DD-ENG-XXXX-description.md`.
```

**Confirmed**: The `!`command`` preprocessor supports inline placement
(mid-sentence). The token is replaced at the token level, not the line level.
This is confirmed by official documentation examples (e.g.,
`- PR diff: !`gh pr diff``) and GitHub issue #12781 which demonstrates
mid-line matching. Use inline placement for path injection as shown above.

### Changes Required:

#### 1. `create-plan/SKILL.md`

**Path changes**: Replace hardcoded `meta/plans/` references with inline
preprocessor calls. Since the preprocessor supports inline placement, embed
the path directly in the instruction text:

Modify the instruction that says "Write the plan to
`meta/plans/YYYY-MM-DD-ENG-XXXX-description.md`" to say:

```markdown
1. **Write the plan** to
   `!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans meta/plans``
   using the filename format `YYYY-MM-DD-ENG-XXXX-description.md`.
```

Apply the same inline replacement to all other `meta/plans/` references in
the skill (line 325, and any other occurrences). For references inside the
template itself (e.g., line 316 `meta/research/codebase/`), these are covered by the
cross-reference limitation — users who override paths should provide custom
templates.

**Template changes**: Replace the embedded template (lines 209-310) with a
preprocessor call:

Replace the entire template code block with:
```markdown
2. **Use this template structure**:

!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh plan`
```

#### 2. `research-codebase/SKILL.md`

**Path changes**: Replace hardcoded `meta/research/codebase/` references for the output
path using inline preprocessor placement. Update the filename instruction to
reference the configured path.

Also replace generic `meta/` scan references (lines 86, 89, 201, 211) with
references to the configured paths. The skill instructs sub-agents to
"explore the entire meta/ directory" — with path overrides, documents may not
be under a single `meta/` parent. Update these instructions to reference the
configured research, plans, and decisions directories individually, matching
the approach used for `extract-adrs` (Step 4).

**Template changes**: Replace the embedded template (lines 110-166) with:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh research`
```

#### 3. `create-adr/SKILL.md`

**Path changes**: Replace hardcoded `meta/decisions/` references.

Add a preprocessor line for the decisions path:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh decisions meta/decisions`
```

Update the output path instruction to reference the configured path.

**Template changes**: Replace the embedded template (lines 161-215) with:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh adr`
```

#### 4. `extract-adrs/SKILL.md`

**Path changes**: Replace hardcoded `meta/decisions/` and `meta/` references.

Add preprocessor lines for all three document paths that the skill scans:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh decisions meta/decisions`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh research meta/research`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh plans meta/plans`
```

**Scan path changes**: The skill currently instructs the `documents-locator`
agent to scan `meta/` as a single parent directory (line 43). When paths are
independently configured (e.g., `paths.plans: docs/plans`,
`paths.research: docs/research`), these may not share a common parent. Update
the scan instruction to reference the three configured paths individually:

Replace the instruction "find all documents in `meta/`" with:
```markdown
find all documents in the configured research, plans, and decisions
directories (shown above, or `meta/research/codebase/`, `meta/plans/`, and
`meta/decisions/` by default)
```

Also update the example output listing (lines 47-55) to use the configured
paths rather than hardcoded `meta/research/codebase/` and `meta/plans/` references.

Update the instruction that says "Write to `meta/decisions/`" to reference the
configured decisions path.

Since `extract-adrs` delegates to `create-adr`'s template, no template
change is needed, but update the reference to note that the template comes
from the `create-adr` skill or the configured template override.

#### 5. `review-adr/SKILL.md`

**Path changes**: Replace hardcoded `meta/decisions/` scan path.

Add a preprocessor line for the decisions path. Update references to
`meta/decisions/` to use the configured path.

#### 6. `validate-plan/SKILL.md`

**Path changes**: The validation output path `meta/validations/` and the
template are both embedded. Add path config for the validations directory:

```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh validations meta/validations`
```

Also update the following hardcoded references in the skill:

- **`mkdir -p meta/validations`** (line 164): Replace with an instruction to
  create the configured validations directory, e.g.:
  ```markdown
  Create the configured validations directory if it doesn't exist.
  ```
- **Output path instruction** (lines 154-160): Replace
  `meta/validations/YYYY-MM-DD-...-validation.md` with a reference to the
  configured validations directory.
- **User notification** (line 195): Replace `meta/validations/{filename}.md`
  with a reference to the configured path.

**Note on `target` frontmatter field**: The frontmatter wrapper (lines 170-181)
includes `target: "meta/plans/{plan-filename}.md"` which references the plans
directory. This is a cross-reference covered by the documented limitation in
Phase 2 — the `target` field in the extracted template will contain the default
`meta/plans/` path. Users who override `paths.plans` should also provide a
custom validation template with the updated `target` path pattern.

**Template changes**: Replace the embedded template with:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh validation`
```

#### 7. `adr-next-number.sh`

**Path changes**: Lines 32-38 hardcode `$REPO_ROOT/meta/decisions` and include
a directory-existence early-exit. Replace the entire block (lines 32-38) with:

```bash
# Read configured decisions path, defaulting to meta/decisions
DECISIONS_PATH=$("$PLUGIN_ROOT/scripts/config-read-path.sh" decisions meta/decisions)

# Resolve: absolute paths used as-is, relative paths resolved against repo root
if [[ "$DECISIONS_PATH" == /* ]]; then
  DECISIONS_DIR="$DECISIONS_PATH"
else
  DECISIONS_DIR="$REPO_ROOT/$DECISIONS_PATH"
fi

# If directory doesn't exist, output sequential numbers starting from 0001
if [ ! -d "$DECISIONS_DIR" ]; then
  echo "Warning: decisions directory '$DECISIONS_DIR' does not exist — defaulting to next number 0001" >&2
  for ((i = 1; i <= COUNT; i++)); do
    printf "%04d\n" "$i"
  done
  exit 0
fi
```

This preserves the original early-exit behaviour (outputting sequential numbers
starting from 0001 when the directory doesn't exist) while adding a stderr
warning to aid diagnosis of misconfigured paths.

#### 8. `describe-pr/SKILL.md`

**Path changes**: Replace hardcoded `meta/prs/` references for the output path.
Also replace hardcoded `meta/templates/` references — the `paths.templates`
key controls where `describe-pr` looks for `pr-description.md`.

Add preprocessor lines:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh prs meta/prs`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh templates meta/templates`
```

Update all `meta/prs/` references in the skill to use the configured path.
The skill references `meta/prs/{number}-description.md` in multiple locations
(output path, existence checks, re-run detection, frontmatter stripping) —
audit all occurrences to ensure none remain hardcoded.

Update `meta/templates/pr-description.md` references (lines 19, 21) to use
the configured templates path.

#### 9. `review-plan/SKILL.md`

**Path changes**: Replace hardcoded `meta/reviews/plans/` references.

Add preprocessor line:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_plans meta/reviews/plans`
```

Update the review output path instruction (around line 362-364) and any other
references to `meta/reviews/plans/` to use the configured path.

#### 10. `review-pr/SKILL.md`

**Path changes**: Replace hardcoded `meta/reviews/prs/` (formerly `meta/tmp/`)
references for the PR review working directory.

Add preprocessor line:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_prs meta/reviews/prs`
```

Update references to the PR review working directory to use the configured
path.

#### 11. `create-plan/SKILL.md` — tickets path

**Path changes**: In addition to the plans path and template changes in Step 1,
replace hardcoded `meta/tickets/` references (lines 38, 39, 50, 315).

Add preprocessor line:
```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh tickets meta/tickets`
```

Update all `meta/tickets/` references to use the configured path. The
template's References section (line 315) contains `meta/tickets/eng-XXXX.md`
— this is a cross-reference covered by the documented template limitation
(users who override paths should provide custom templates with updated
cross-references).

#### 12. Skills referencing `meta/notes/`

**Path changes**: If any skills reference `meta/notes/`, add a preprocessor
line for the notes path. Currently `meta/notes/` is not directly referenced
by any skill (it is a user-created directory), but the path key should be
available for future skills and for `config-dump.sh` completeness.

### Success Criteria:

#### Automated Verification:

- [x] No hardcoded `meta/plans/` in `create-plan/SKILL.md` (except in examples)
- [x] No hardcoded `meta/research/codebase/` in `research-codebase/SKILL.md` (except
  in examples)
- [x] No generic `meta/` scan references in `research-codebase/SKILL.md`
  (replaced with configured path references)
- [x] No hardcoded `meta/decisions/` in ADR skills or `adr-next-number.sh`
  (except in examples)
- [x] No hardcoded `meta/reviews/plans/` in `review-plan/SKILL.md` (except
  in examples)
- [x] No hardcoded `meta/reviews/prs/` in `review-pr/SKILL.md` (except
  in examples)
- [x] No hardcoded `meta/tickets/` in `create-plan/SKILL.md` (except in
  examples)
- [x] No hardcoded `meta/templates/` in `describe-pr/SKILL.md` (except in
  examples)
- [x] All affected skills contain `config-read-path.sh` and/or
  `config-read-template.sh` preprocessor lines
- [x] `bash scripts/test-config.sh` passes all tests, including:
  - `config-read-path.sh plans meta/plans` with no config outputs `meta/plans`
  - `config-read-path.sh plans meta/plans` with `paths.plans: docs/plans`
    outputs `docs/plans`
  - `config-read-template.sh plan` with no user template outputs plugin
    default template content
  - `config-read-template.sh plan` with template in `meta/templates/`
    outputs user template content
  - `config-read-template.sh plan` with `paths.templates: docs/templates`
    and template at `docs/templates/plan.md` outputs that template
- [x] `bash skills/decisions/scripts/test-adr-scripts.sh` still passes
- [x] New `adr-next-number.sh` integration tests pass:
  - Default path behaviour preserved when no config exists
  - With `paths.decisions: custom/adrs` and ADR files in that directory,
    script scans custom directory and returns correct next number
  - With configured directory that does not exist, script warns to stderr
    and returns `0001`

#### Manual Verification:

- [ ] With no config, all skills write to their original default paths
- [ ] With `paths.decisions: docs/adrs`, `create-adr` writes to `docs/adrs/`
- [ ] With a custom plan template at `meta/templates/plan.md`,
  `create-plan` uses it
- [ ] With `paths.templates: docs/templates` and a template at
  `docs/templates/plan.md`, `create-plan` uses it
- [ ] ADR numbering script finds existing ADRs in the configured directory
- [ ] Cross-references between skills still work (e.g., plan template
  referencing research directory)

---

## Phase 5: Configure Skill & Documentation Updates

### Overview

Update the configure skill and README with path and template documentation.

### Changes Required:

#### 1. Configure Skill Update

**File**: `skills/config/configure/SKILL.md`
**Changes**: Add paths and templates documentation:

```markdown
### paths

Override where skills write output documents. Paths are relative to the
project root (absolute paths are also supported):

| Key | Default | Description |
|-----|---------|-------------|
| `plans` | `meta/plans` | Implementation plans |
| `research` | `meta/research` | Research documents |
| `decisions` | `meta/decisions` | Architecture decision records |
| `prs` | `meta/prs` | PR descriptions |
| `validations` | `meta/validations` | Plan validation reports |
| `review_plans` | `meta/reviews/plans` | Plan review artifacts |
| `review_prs` | `meta/reviews/prs` | PR review working directories |
| `templates` | `meta/templates` | User-provided templates (e.g., PR description) |
| `tickets` | `meta/tickets` | Ticket files referenced by create-plan |
| `notes` | `meta/notes` | Notes directory |

Example configuration:

\```yaml
---
paths:
  plans: docs/plans
  research: docs/research
  decisions: docs/adrs
  prs: docs/prs
  validations: docs/validations
  review_plans: docs/reviews/plans
  review_prs: docs/reviews/prs
  templates: docs/templates
  tickets: docs/tickets
  notes: docs/notes
---
\```

Note: YAML comments (`#`) are not supported by the config parser. Do not
add inline comments to config values.

### templates

Override document templates by placing custom template files in the
templates directory (`paths.templates`, defaults to `meta/templates/`):

\```
meta/templates/
  plan.md            # Custom plan template
  research.md        # Custom research template
  adr.md             # Custom ADR template
  validation.md      # Custom validation template
  pr-description.md  # PR description template (used by describe-pr)
\```

All templates — both skill structure templates (plan, ADR, research,
validation) and user content templates (PR description) — live in the same
directory. Override `paths.templates` to move them all:

\```yaml
---
paths:
  templates: docs/templates
---
\```

For advanced use cases, you can also point individual templates to specific
file paths using the `templates` config section:

\```yaml
---
templates:
  plan: docs/templates/our-plan-format.md
  adr: docs/templates/our-adr-format.md
---
\```

Resolution order: `templates.<name>` config path (if set) → templates
directory (`paths.templates`) → plugin default. Use the plugin's
`templates/` directory as a starting point for customisation.

**Note on cross-references**: Default templates contain hardcoded references
to other skills' output paths (e.g., the plan template references
`meta/research/codebase/` in its References section). If you override output paths
(e.g., `paths.research: docs/research`), you should also provide custom
templates with updated cross-references.
```

#### 2. README Update

**File**: `README.md`
**Changes**: Update Configuration section with paths and templates.

### Success Criteria:

#### Manual Verification:

- [ ] `/accelerator:configure help` shows paths and templates documentation
- [ ] README documents all configuration options

---

## Testing Strategy

### Unit Tests:

- `config-read-path.sh` and `config-read-template.sh` tests in `test-config.sh`
- Edge cases: relative vs absolute paths, missing directories, symlinks
- Template extraction fidelity: each extracted template contains expected
  structural markers (headings, frontmatter fields)
- Code fence wrapping: script output starts with ` ```markdown ` and ends
  with ` ``` ` when template file is not already fenced
- No double-wrapping: template file starting with ` ``` ` is output as-is
- Config-specified template path missing: warning emitted to stderr, fallback
  used
- Unknown template name: error message lists available template names

### Integration Tests:

- `adr-next-number.sh` with path overrides in `test-config.sh`:
  - Default path preserved when no config
  - Custom path scanned when configured
  - Missing directory produces warning and returns `0001`
- Preprocessor commands exercised end-to-end:
  - `config-read-path.sh plans meta/plans` with/without config
  - `config-read-template.sh plan` with/without user template

### Manual Testing Steps:

1. Create config: `paths.decisions: docs/adrs`
2. Run `/accelerator:create-adr test-decision`
3. Verify ADR is created in `docs/adrs/` not `meta/decisions/`
4. Create a custom plan template at `meta/templates/plan.md`
5. Run `/accelerator:create-plan`
6. Verify the custom template structure is used
7. Remove the custom template, verify it falls back to the plugin default

## Performance Considerations

Each `!`command`` preprocessor call adds a shell execution at skill load time.
Skills in this plan may have 3-5 preprocessor calls (context, agents, review
config, path, template). Shell scripts should execute quickly (<100ms each)
to avoid noticeable delay. The config reader scripts are lightweight file
operations and should be well within this budget.

## Configure Skill Updates (Deferred from Plan 1)

Plan 1 (Configuration Infrastructure) deferred configure skill key
documentation to each plan that introduces keys. This plan must:

1. **Update the configure skill's `help` action**
   (`skills/config/configure/SKILL.md`) to add documentation of the `paths`
   and `templates` section keys:
   - `paths.plans`, `paths.decisions`, `paths.research`, etc. — full list with
     defaults
   - `templates.plan`, `templates.adr`, `templates.research`, etc. — full list

2. **Update the configure skill's `create` action** to mention that path and
   template customisation is available and point users to
   `/accelerator:configure help` for the full reference. Do not walk through
   each key interactively.

3. **Consider a sentinel value for unsetting team config** (e.g., `~` or
   `null`) if override semantics prove limiting — i.e., when a team config sets
   a path that individual developers need to revert to the default locally.

4. **Add `scripts/config-dump.sh`** if not already added by Plan 3. By this
   plan, the number of config keys should warrant a dump/debug tool.

## References

- Plan 1: `meta/plans/2026-03-23-config-infrastructure.md`
- Research: `meta/research/codebase/2026-03-22-skill-customisation-and-override-patterns.md`
- Existing user template pattern: `skills/github/describe-pr/SKILL.md:18-21`
- Embedded templates:
  - Plan: `skills/planning/create-plan/SKILL.md:209-310`
  - Research: `skills/research/research-codebase/SKILL.md:110-166`
  - ADR: `skills/decisions/create-adr/SKILL.md:161-215`
  - Validation: `skills/planning/validate-plan/SKILL.md:102-150`
- Path references in scripts: `skills/decisions/scripts/adr-next-number.sh:32`
