---
date: "2026-03-28T12:27:27+0000"
type: plan
skill: create-plan
status: draft
---

# Per-Skill Userspace Customisation Implementation Plan

## Overview

Add per-skill customisation to the Accelerator plugin, allowing users to
provide skill-specific context and additional instructions by placing files in
`.claude/accelerator/skills/<skill-name>/` directories. This extends the
existing custom lenses precedent (`.claude/accelerator/lenses/`) to all 13
user-facing skills, while keeping all configuration parameters at the global
level.

## Current State Analysis

The plugin currently supports:

- **Global context injection**: All 13 user-facing skills include
  `config-read-context.sh` via preprocessor, injecting the markdown body from
  `.claude/accelerator.md` / `.claude/accelerator.local.md`
- **Global config**: Structured YAML frontmatter for agents, review settings,
  paths, and templates — all globally scoped
- **Custom lenses**: The only existing per-skill extension point, using
  `.claude/accelerator/lenses/<lens-name>/SKILL.md` with auto-discovery in
  `config-read-review.sh`

There is no mechanism for users to provide context or instructions targeted at
a specific skill. For example, a user cannot say "when creating plans, always
include a security section" without that instruction bleeding into every other
skill via global context.

### Key Discoveries:

- Custom lens discovery (`config-read-review.sh:124-202`) provides a tested
  pattern for scanning `.claude/accelerator/` subdirectories
- `config-read-context.sh` already handles multi-source context concatenation
  with trimming — the same pattern extends to per-skill context
- The `config-summary.sh` script detects and reports config state at session
  start — it should also report per-skill customisations
- All 13 user-facing skills already have `config-read-context.sh` preprocessor
  lines, making it straightforward to add adjacent per-skill lines
- The `configure` skill (`skills/config/configure/SKILL.md`) documents all
  config features and needs updating

## Desired End State

After this plan is complete:

1. Users can create `.claude/accelerator/skills/<skill-name>/context.md` to
   provide skill-specific context that is injected after global context
2. Users can create `.claude/accelerator/skills/<skill-name>/instructions.md`
   to provide additional instructions that are appended to a skill's prompt
3. A new script `config-read-skill-context.sh` reads skill-specific context
4. A new script `config-read-skill-instructions.sh` reads skill-specific
   instructions
5. All 13 user-facing skills include preprocessor calls for both scripts
6. The SessionStart hook reports detected per-skill customisations
7. The `/accelerator:configure` skill documents the feature with examples
8. The test suite covers all new scripts and edge cases

### Verification:

- `bash scripts/test-config.sh` passes with new tests
- Creating `.claude/accelerator/skills/review-pr/context.md` causes its
  content to appear in the review-pr skill output after global context
- Creating `.claude/accelerator/skills/create-plan/instructions.md` causes its
  content to appear at the end of the create-plan skill
- The SessionStart hook reports per-skill customisations when files are present
- `/accelerator:configure help` documents the new feature

## What We're NOT Doing

- Per-skill configuration parameter overrides (config stays global-only)
- Skill replacement/override (no replacing entire SKILL.md files)
- Nested directory structures within per-skill directories
- Per-skill template overrides (templates remain globally configured)
- Blocking on unrecognised skill names (advisory warning only; files still
  reported in summary even if the directory name doesn't match a known skill)

## Implementation Approach

Follow the established pattern from custom lenses: shell scripts that scan
convention-based directories, with preprocessor integration into skills.
Two new scripts handle the two file types (context and instructions). Each
skill adds two preprocessor lines. The configure skill and session hook are
updated to document and detect the feature.

## Phase 1: Core Scripts

### Overview

Create the two reader scripts and a discovery utility for the session hook.

### Changes Required:

#### 1. `scripts/config-read-skill-context.sh`

**File**: `scripts/config-read-skill-context.sh` (new)
**Purpose**: Read skill-specific context from
`.claude/accelerator/skills/<skill-name>/context.md`

```bash
#!/usr/bin/env bash
set -euo pipefail

# Reads skill-specific context from the per-skill customisation directory.
# Outputs the content wrapped in a section header, or nothing if no file
# exists.
#
# Usage: config-read-skill-context.sh <skill-name>
#
# Looks for: <project-root>/.claude/accelerator/skills/<skill-name>/context.md

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"

SKILL_NAME="${1:-}"
if [ -z "$SKILL_NAME" ]; then
  echo "Usage: config-read-skill-context.sh <skill-name>" >&2
  exit 1
fi

PROJECT_ROOT=$(config_project_root)
CONTEXT_FILE="$PROJECT_ROOT/.claude/accelerator/skills/$SKILL_NAME/context.md"

[ -f "$CONTEXT_FILE" ] || exit 0

CONTENT=$(config_trim_body < "$CONTEXT_FILE")
[ -z "$CONTENT" ] && exit 0

echo "## Skill-Specific Context"
echo ""
echo "The following context is specific to the $SKILL_NAME skill. Apply this"
echo "context in addition to any project-wide context above."
echo ""
printf '%s\n' "$CONTENT"
```

#### 2. `scripts/config-read-skill-instructions.sh`

**File**: `scripts/config-read-skill-instructions.sh` (new)
**Purpose**: Read skill-specific instructions from
`.claude/accelerator/skills/<skill-name>/instructions.md`

```bash
#!/usr/bin/env bash
set -euo pipefail

# Reads skill-specific instructions from the per-skill customisation
# directory. Outputs the content wrapped in a section header, or nothing
# if no file exists.
#
# Usage: config-read-skill-instructions.sh <skill-name>
#
# Looks for:
#   <project-root>/.claude/accelerator/skills/<skill-name>/instructions.md

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/config-common.sh"

SKILL_NAME="${1:-}"
if [ -z "$SKILL_NAME" ]; then
  echo "Usage: config-read-skill-instructions.sh <skill-name>" >&2
  exit 1
fi

PROJECT_ROOT=$(config_project_root)
INSTRUCTIONS_FILE="$PROJECT_ROOT/.claude/accelerator/skills/$SKILL_NAME/instructions.md"

[ -f "$INSTRUCTIONS_FILE" ] || exit 0

CONTENT=$(config_trim_body < "$INSTRUCTIONS_FILE")
[ -z "$CONTENT" ] && exit 0

echo "## Additional Instructions"
echo ""
echo "The following additional instructions have been provided for the"
echo "$SKILL_NAME skill. Follow these instructions in addition to all"
echo "instructions above."
echo ""
printf '%s\n' "$CONTENT"
```

#### 3. Update `scripts/config-summary.sh`

**File**: `scripts/config-summary.sh`
**Changes**: After the existing context detection block (lines 61-75), add
detection of per-skill customisations.

Add after the `HAS_CONTEXT` block and before the final summary output:

```bash
# Check for per-skill customisations
SKILL_CUSTOM_DIR="$ROOT/.claude/accelerator/skills"
SKILL_CUSTOMISATIONS=""

# Derive known skill names dynamically from plugin skill directories
# (excludes configure, which is not customisable via this mechanism)
PLUGIN_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KNOWN_SKILLS=""
for skill_md in "$PLUGIN_ROOT"/skills/*/SKILL.md "$PLUGIN_ROOT"/skills/*/*/SKILL.md; do
  [ -f "$skill_md" ] || continue
  sname=$(awk '/^name:/{print $2; exit}' "$skill_md")
  [ "$sname" = "configure" ] && continue
  [ -n "$sname" ] && KNOWN_SKILLS="$KNOWN_SKILLS $sname"
done
KNOWN_SKILLS="${KNOWN_SKILLS# }"

if [ -d "$SKILL_CUSTOM_DIR" ]; then
  for skill_dir in "$SKILL_CUSTOM_DIR"/*/; do
    [ -d "$skill_dir" ] || continue
    skill_name=$(basename "$skill_dir")

    # Warn about unrecognised skill names
    case " $KNOWN_SKILLS " in
      *" $skill_name "*) ;;
      *) echo "Warning: .claude/accelerator/skills/$skill_name/ does not match any known skill name. Valid names: $KNOWN_SKILLS" >&2 ;;
    esac

    # Check for non-empty content (matching reader script behaviour)
    has_context=false
    has_instructions=false
    if [ -f "$skill_dir/context.md" ]; then
      trimmed=$(config_trim_body < "$skill_dir/context.md")
      [ -n "$trimmed" ] && has_context=true
    fi
    if [ -f "$skill_dir/instructions.md" ]; then
      trimmed=$(config_trim_body < "$skill_dir/instructions.md")
      [ -n "$trimmed" ] && has_instructions=true
    fi

    if [ "$has_context" = true ] || [ "$has_instructions" = true ]; then
      types=""
      [ "$has_context" = true ] && types="context"
      if [ "$has_instructions" = true ]; then
        [ -n "$types" ] && types="$types + "
        types="${types}instructions"
      fi
      SKILL_CUSTOMISATIONS="$SKILL_CUSTOMISATIONS
    - $skill_name ($types)"
    fi
  done
fi

if [ -n "$SKILL_CUSTOMISATIONS" ]; then
  SUMMARY="$SUMMARY
- Per-skill customisations:$SKILL_CUSTOMISATIONS"
fi
```

### Success Criteria:

#### Automated Verification:

- [ ] `config-read-skill-context.sh create-plan` outputs nothing when no file
      exists
- [ ] `config-read-skill-context.sh create-plan` outputs wrapped content when
      `.claude/accelerator/skills/create-plan/context.md` exists
- [ ] `config-read-skill-instructions.sh review-pr` outputs nothing when no
      file exists
- [ ] `config-read-skill-instructions.sh review-pr` outputs wrapped content
      when `.claude/accelerator/skills/review-pr/instructions.md` exists
- [ ] Both scripts handle empty files gracefully (no output)
- [ ] Both scripts handle files with only whitespace gracefully (no output)
- [ ] Both scripts fail with usage message when no skill name argument
- [ ] `config-summary.sh` reports per-skill customisations when present
- [ ] `config-summary.sh` lists correct file types per skill (context,
      instructions, or both)
- [ ] `config-summary.sh` does not list skills with only empty/whitespace files
- [ ] `config-summary.sh` emits stderr warning for unrecognised skill names
- [ ] `bash scripts/test-config.sh` passes

---

## Phase 2: Skill Integration

### Overview

Add preprocessor lines to all 13 user-facing skills. Context goes immediately
after the global context line. Instructions go at the very end of the skill
file.

### Changes Required:

#### 1. All 13 User-Facing Skills

For each skill, add two preprocessor lines:

**Near the top** (immediately after the existing `config-read-context.sh`
line):

```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh <skill-name>`
```

**At the very end** of the file:

```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh <skill-name>`
```

The 13 skills and their names:

| Skill | Directory | Name for scripts |
|-------|-----------|------------------|
| Create Plan | `skills/planning/create-plan/SKILL.md` | `create-plan` |
| Implement Plan | `skills/planning/implement-plan/SKILL.md` | `implement-plan` |
| Review Plan | `skills/planning/review-plan/SKILL.md` | `review-plan` |
| Validate Plan | `skills/planning/validate-plan/SKILL.md` | `validate-plan` |
| Stress Test Plan | `skills/planning/stress-test-plan/SKILL.md` | `stress-test-plan` |
| Review PR | `skills/github/review-pr/SKILL.md` | `review-pr` |
| Describe PR | `skills/github/describe-pr/SKILL.md` | `describe-pr` |
| Respond to PR | `skills/github/respond-to-pr/SKILL.md` | `respond-to-pr` |
| Research Codebase | `skills/research/research-codebase/SKILL.md` | `research-codebase` |
| Create ADR | `skills/decisions/create-adr/SKILL.md` | `create-adr` |
| Extract ADRs | `skills/decisions/extract-adrs/SKILL.md` | `extract-adrs` |
| Review ADR | `skills/decisions/review-adr/SKILL.md` | `review-adr` |
| Commit | `skills/vcs/commit/SKILL.md` | `commit` |

**Note**: The `configure` skill is excluded — it manages configuration and
should not have user-injected instructions that could interfere with its
operation.

#### Example: `skills/planning/create-plan/SKILL.md`

Current (lines 12-13):

```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`
```

Becomes:

```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-context.sh`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-context.sh create-plan`
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-agents.sh`
```

And at the end of the file, add:

```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-skill-instructions.sh create-plan`
```

### Success Criteria:

#### Automated Verification:

- [ ] All 13 skills contain a `config-read-skill-context.sh` preprocessor line
      with the correct skill name
- [ ] All 13 skills contain a `config-read-skill-instructions.sh` preprocessor
      line with the correct skill name at the end of the file
- [ ] Skill names in preprocessor calls match the `name` field in each skill's
      frontmatter
- [ ] `bash scripts/test-config.sh` passes

#### Manual Verification:

- [ ] Invoking `/accelerator:create-plan` with a
      `.claude/accelerator/skills/create-plan/context.md` present shows the
      skill-specific context in the expanded prompt
- [ ] Invoking `/accelerator:review-pr` with a
      `.claude/accelerator/skills/review-pr/instructions.md` present shows the
      additional instructions at the end of the expanded prompt
- [ ] Skills without per-skill files work identically to before (no empty
      sections, no extra whitespace)

---

## Phase 3: Documentation and Configure Skill

### Overview

Update the `/accelerator:configure` skill to document per-skill customisation
and help users create per-skill files.

### Changes Required:

#### 1. Update `skills/config/configure/SKILL.md`

**File**: `skills/config/configure/SKILL.md`

Add a new section in the `help` action output, after the "Custom Lenses"
section and before the "paths" section. This documents the per-skill
customisation feature:

```markdown
### Per-Skill Customisation

Provide context or additional instructions for specific skills by placing
files in `.claude/accelerator/skills/<skill-name>/`:

\```
.claude/accelerator/skills/
  create-plan/
    context.md          # Context specific to plan creation
    instructions.md     # Additional instructions for plan creation
  review-pr/
    context.md          # Context specific to PR review
    instructions.md     # Additional instructions for PR review
  commit/
    instructions.md     # Additional instructions for commits
\```

**`context.md`** — Skill-specific context injected after global project
context. Use this for information that is only relevant to a particular
skill. For example, review-pr might need to know about specific review
criteria, while create-plan might need architecture context.

**`instructions.md`** — Additional instructions appended to the skill's
prompt. Use this to customise skill behaviour: add extra steps, enforce
conventions, or modify output format.

Both files are optional. If neither exists for a skill, it behaves as
before. Files are read at skill invocation time. Do not add YAML
frontmatter to these files — their entire content is injected as-is.

**When to use which**: Use **global context** (`.claude/accelerator.md`)
for information all skills should know. Use **skill context**
(`context.md`) for information only one skill needs. Use **skill
instructions** (`instructions.md`) to change how a skill behaves — add
steps, enforce formats, or modify output. Per-skill context and
instructions supplement global context (both are visible to the skill);
per-skill instructions appear at the end of the prompt and will typically
take precedence if they conflict with earlier instructions.

**Shared vs personal**: Per-skill files are typically committed to the
repository as team-shared customisations. For personal per-skill
preferences, add the relevant directories to `.gitignore`.

**Troubleshooting**: Directory names must match a known skill name exactly.
The directory name matches the skill name after `/accelerator:` — for
example, `/accelerator:review-pr` uses `review-pr/`,
`/accelerator:create-plan` uses `create-plan/`. Run
`/accelerator:configure view` to see all available skill names and any
active per-skill customisations. The SessionStart hook output also lists
detected per-skill customisations and warns about unrecognised directory
names. To temporarily disable a customisation, rename the file (e.g.,
`context.md.disabled`).

Note: The `configure` skill is not customisable via this mechanism as it
manages configuration itself.

Example `context.md` for review-pr:

\```markdown
## Review Focus Areas

Our team particularly cares about:
- API backward compatibility (we have external consumers)
- Database migration safety (zero-downtime deploys required)
- Test coverage for error paths (we've had incidents from untested error handling)
\```

Example `instructions.md` for create-plan:

\```markdown
- Always include a "Security Considerations" section in plans
- Reference our threat model at docs/security/threat-model.md
- Plans touching the payments service require a rollback strategy
\```
```

Also update the `view` action to enumerate per-skill customisations. After
displaying team and personal config, add a section:

```markdown
### Per-Skill Customisations

[For each directory under .claude/accelerator/skills/ that contains
non-empty context.md or instructions.md, list:]

- `<skill-name>`:
  - context.md: [present / not found]
  - instructions.md: [present / not found]

[If no per-skill customisation directories exist:]

No per-skill customisations found. See `/accelerator:configure help`
for details on per-skill context and instructions.
```

Also update the `create` action guidance (around line 85) to mention per-skill
customisation:

```markdown
5. Mention that additional customisation is available: "You can also
   customise review behaviour (lens selection, verdict thresholds, inline
   comment limits), output paths (where skills write documents), document
   templates (plan, ADR, research, validation formats), and per-skill
   context and instructions (`.claude/accelerator/skills/<skill-name>/`).
   Run `/accelerator:configure help` for the full key reference."
```

### Success Criteria:

#### Automated Verification:

- [ ] The configure skill's help output includes the per-skill customisation
      section
- [ ] The help text directs users to `/accelerator:configure view` for the
      full list of available skill names
- [ ] Examples are syntactically correct markdown

#### Manual Verification:

- [ ] `/accelerator:configure help` displays the per-skill section clearly
- [ ] The documentation is clear enough that a user could set up per-skill
      customisation without further guidance

---

## Phase 4: Testing

### Overview

Add comprehensive tests to `scripts/test-config.sh` covering the new scripts
and integration with existing infrastructure.

### Changes Required:

#### 1. Update `scripts/test-config.sh`

**File**: `scripts/test-config.sh`
**Changes**: Add new test sections for `config-read-skill-context.sh`,
`config-read-skill-instructions.sh`, and updated `config-summary.sh`.

Add script references at the top (after existing script references around
line 14):

```bash
READ_SKILL_CONTEXT="$SCRIPT_DIR/config-read-skill-context.sh"
READ_SKILL_INSTRUCTIONS="$SCRIPT_DIR/config-read-skill-instructions.sh"
```

Add the following test sections:

**config-read-skill-context.sh tests:**

```
=== config-read-skill-context.sh ===

Test: No skill name argument → exits with error
Test: No customisation directory → no output
Test: Skill directory exists but no context.md → no output
Test: context.md exists with content → outputs section with header
Test: context.md exists but is empty → no output
Test: context.md exists with only whitespace/blank lines → no output
Test: context.md with leading/trailing blank lines → trimmed output
Test: Output includes skill name in header text
Test: Multiple skills with context → each reads only its own
```

**config-read-skill-instructions.sh tests:**

```
=== config-read-skill-instructions.sh ===

Test: No skill name argument → exits with error
Test: No customisation directory → no output
Test: Skill directory exists but no instructions.md → no output
Test: instructions.md exists with content → outputs section with header
Test: instructions.md exists but is empty → no output
Test: instructions.md exists with only whitespace/blank lines → no output
Test: instructions.md with leading/trailing blank lines → trimmed output
Test: Output includes skill name in header text
Test: Multiple skills with instructions → each reads only its own
```

**config-summary.sh per-skill tests:**

```
=== config-summary.sh (per-skill customisations) ===

Test: No per-skill directories → no per-skill line in summary
Test: One skill with context.md → summary lists skill with "(context)"
Test: One skill with instructions.md → summary lists skill with "(instructions)"
Test: One skill with both files → summary lists skill with
      "(context + instructions)"
Test: Multiple skills with customisations → all listed
Test: Skill directory with no recognised files → not listed
Test: Empty context.md and instructions.md → skill not listed
Test: Whitespace-only context.md → skill not listed (matches reader behaviour)
Test: Unrecognised skill directory name → stderr warning emitted
Test: Known skill directory name → no stderr warning
```

**Preprocessor placement tests** (mirroring existing placement tests for
`config-read-context.sh`):

```
=== Preprocessor placement (per-skill) ===

Test: config-read-skill-context.sh appears in exactly 13 skills
Test: config-read-skill-instructions.sh appears in exactly 13 skills
Test: config-read-skill-context.sh appears immediately after
      config-read-context.sh in each skill
Test: config-read-skill-instructions.sh is the last preprocessor line
      in each skill
Test: Skill name argument in each preprocessor call matches the skill's
      frontmatter name field
Test: configure skill does NOT contain per-skill preprocessor lines
Test: KNOWN_SKILLS is derived dynamically and includes all user-facing
      skill names from frontmatter (excluding configure)
```

**config-detect.sh hook integration tests:**

```
=== config-detect.sh (per-skill customisations) ===

Test: Per-skill customisations appear in hook additionalContext JSON
Test: Unrecognised skill name warning appears in stderr (not in JSON)
```

Add two new test helpers alongside the existing `assert_eq` and
`assert_exit_code`:

```bash
assert_contains() {
  local test_name="$1" needle="$2" haystack="$3"
  if printf '%s' "$haystack" | grep -qF "$needle"; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected to contain: $needle"
    echo "    Actual: $haystack"
    FAIL=$((FAIL + 1))
  fi
}

assert_empty() {
  local test_name="$1" actual="$2"
  if [ -z "$actual" ]; then
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL: $test_name"
    echo "    Expected empty, got: $actual"
    FAIL=$((FAIL + 1))
  fi
}
```

Each test follows the established pattern in the file: `setup_repo` to create
a temp directory with `.git`, create the necessary files, `cd` into the repo,
run the script, and use the assertion helpers to check output. Error path
tests use `assert_exit_code` to verify exit codes.

### Success Criteria:

#### Automated Verification:

- [ ] `bash scripts/test-config.sh` passes with 0 failures
- [ ] All new test cases are present and passing
- [ ] No existing tests are broken

---

## Testing Strategy

### Unit Tests (in `scripts/test-config.sh`)

All test cases listed in Phase 4 above, covering:

- **Happy path**: Files exist with content, correct output produced
- **Empty/missing**: No directory, no file, empty file, whitespace-only file
- **Isolation**: Multiple skills' files don't interfere with each other
- **Error handling**: Missing arguments produce usage errors with exit code 1
- **Integration**: `config-summary.sh` correctly discovers and reports
  per-skill customisations, skips empty files, warns on unrecognised names
- **Hook integration**: `config-detect.sh` (via `config-summary.sh`) includes
  per-skill information in `additionalContext`
- **Preprocessor placement**: All 13 skills have both new preprocessor lines
  with correct skill names, correct ordering, and frontmatter name matching
- **Exact output format**: At least one test per script verifying complete
  output (header + explanatory text + content) using `assert_eq`

### Manual Testing Steps

1. Create `.claude/accelerator/skills/create-plan/context.md` with test
   content
2. Invoke `/accelerator:create-plan` and verify the skill-specific context
   appears after global context
3. Create `.claude/accelerator/skills/review-pr/instructions.md` with test
   instructions
4. Invoke `/accelerator:review-pr` and verify the additional instructions
   appear at the end of the expanded prompt
5. Start a new Claude Code session and verify the SessionStart hook reports
   the per-skill customisations
6. Invoke `/accelerator:configure help` and verify the documentation is
   present and accurate
7. Remove the per-skill files and verify skills work identically to before

## Performance Considerations

Each skill gains two additional preprocessor commands. When no per-skill files
exist, each script performs one `config_project_root` call (shared with
existing scripts) and one `[ -f ... ]` check, then exits immediately. The
overhead is negligible (~5-10ms per script, ~10-20ms total per skill
invocation).

When files do exist, the scripts read and trim one small file each. This adds
at most ~10ms per file.

## References

- Original customisation research:
  `meta/research/codebase/2026-03-22-skill-customisation-and-override-patterns.md`
- Implementation status research:
  `meta/research/codebase/2026-03-27-skill-customisation-implementation-status.md`
- Custom lens discovery pattern: `scripts/config-read-review.sh:124-202`
- Global context reader: `scripts/config-read-context.sh`
- Session summary: `scripts/config-summary.sh`
- Configure skill: `skills/config/configure/SKILL.md`
- Test harness: `scripts/test-config.sh`
