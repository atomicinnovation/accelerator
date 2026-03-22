# Review System Customisation Implementation Plan

## Overview

Add configuration support for the review system: lens management (disable,
reorder, add custom lenses), numeric review limits, and verdict thresholds.
This allows users to tailor the review process to their project's needs.

**Depends on**: Plan 1 (Configuration Infrastructure) must be complete first.
Plan 2 (Context & Agent Customisation) is recommended but not required.

## Current State Analysis

The review system consists of two orchestrator skills (`review-pr` and
`review-plan`) that select from 13 built-in lenses, spawn `reviewer` agents
in parallel, and aggregate results. All configuration is hardcoded:

- **Lens catalogue**: 13 lenses hardcoded in tables at `review-pr/SKILL.md:48-63`
  and `review-plan/SKILL.md:42-57`
- **Core lenses**: Architecture, Code Quality, Test Coverage, Correctness —
  hardcoded at `review-pr/SKILL.md:171-172` and `review-plan/SKILL.md:146-148`
- **Lens count**: 6-8 target, 4 minimum — `review-pr/SKILL.md:167-184` and
  `review-plan/SKILL.md:142-160`
- **Inline comment cap**: 10 — `review-pr/SKILL.md:326-327`
- **Dedup proximity**: 3 lines — `review-pr/SKILL.md:309`
- **PR verdict rules**: Any critical → REQUEST_CHANGES, major or lower → COMMENT,
  no findings → APPROVE — `review-pr/SKILL.md:331-334`
- **Plan verdict rules**: Any critical or 3+ major → REVISE, 1-2 major or
  minor → COMMENT, no findings → APPROVE — `review-plan/SKILL.md:281-285`
- **Lens paths**: `${CLAUDE_PLUGIN_ROOT}/skills/review/lenses/[lens]-lens/SKILL.md`

### Key Discoveries:

- Both `review-pr` and `review-plan` share the same lens catalogue structure
  and auto-detect criteria. Changes must be applied to both consistently.
- The lens path template uses `[lens]-lens` naming convention — custom lenses
  would need to follow this or use absolute paths.
- Lens selection criteria are detailed prose instructions — they can't be
  cleanly parameterised without restructuring. Instead, we can override the
  core lenses list and add a disabled lenses list.
- The `reviewer` agent reads lens and output format files from paths provided
  in its task prompt. Custom lens paths can be injected here.
- Auto-detect criteria for each lens are separate from the catalogue table.
  Disabling a lens means removing it from the table AND skipping its auto-detect
  criteria.

## Desired End State

After this plan:
1. Users can disable specific built-in lenses via config.
2. Users can override which lenses are considered "core four."
3. Users can add custom lenses from `.claude/accelerator/lenses/` directories.
4. Users can tune numeric limits (inline comment cap, lens count range,
   dedup proximity).
5. Users can adjust verdict thresholds for both PR and plan reviews using
   simple numeric and boolean config keys.
6. All changes apply consistently to both `review-pr` and `review-plan`.

## What We're NOT Doing

- Modifying individual lens SKILL.md files — those remain read-only
- Allowing per-review overrides (all config is project-level)
- Supporting lens composition or inheritance
- Changing the output format schemas
- Modifying the `reviewer` agent itself
- Adding new built-in lenses (users can add their own via custom lenses)

## Implementation Approach

Create a `config-read-review.sh` script that reads review-related config and
outputs a structured configuration block. This block is injected into both
orchestrator skills via the `!`command`` preprocessor and replaces or augments
the hardcoded values in the skill instructions.

For custom lenses, the script also discovers lens directories in
`.claude/accelerator/lenses/` and includes them in the catalogue.

## Phase 1: Review Config Reader Script

### Overview

Create a script that reads review configuration and outputs a markdown block
with the effective review settings.

### Changes Required:

#### 1. Review Config Reader

**File**: `scripts/config-read-review.sh`
**Changes**: New file. Reads review config section and outputs a markdown block.

The script takes a required argument specifying the review mode: `pr` or
`plan`. This controls which settings and verdict lines are included in the
output:
- `pr` mode: includes `max_inline_comments`, `dedup_proximity`, and
  `pr_request_changes_severity` verdict line. Omits `plan_revise_severity`
  and `plan_revise_major_count`.
- `plan` mode: includes `plan_revise_severity` and `plan_revise_major_count`
  verdict lines. Omits `max_inline_comments`, `dedup_proximity`, and
  `pr_request_changes_severity`.
- Shared settings (`min_lenses`, `max_lenses`, `core_lenses`,
  `disabled_lenses`) and the lens catalogue are included in both modes.

The script reads these config keys (all under the `review` section):

| Key | Default | Applies to | Description |
|-----|---------|------------|-------------|
| `max_inline_comments` | `10` | PR only | Max inline comments on PR reviews |
| `dedup_proximity` | `3` | PR only | Line proximity for merging findings |
| `pr_request_changes_severity` | `critical` | PR only | Min severity to trigger REQUEST_CHANGES |
| `plan_revise_severity` | `critical` | Plan only | Min severity to trigger REVISE |
| `plan_revise_major_count` | `3` | Plan only | Number of major findings to trigger REVISE |
| `min_lenses` | `4` | Both | Minimum lenses to run |
| `max_lenses` | `8` | Both | Maximum lenses to run |
| `core_lenses` | `[architecture, code-quality, test-coverage, correctness]` | Both | Lenses considered "core four" |
| `disabled_lenses` | `[]` | Both | Lenses to never use |

Valid values for severity keys: `critical`, `major`, `none`. These are a
closed enum validated by the script — unrecognised values emit a warning to
stderr and fall back to the default. `none` disables the severity-based
verdict escalation entirely (e.g., `pr_request_changes_severity: none` means
the PR verdict will never be REQUEST_CHANGES based on severity alone).
Numeric keys (`plan_revise_major_count`) are validated as positive integers.

The script outputs nothing if no review config exists AND no custom lenses
are found. When config is present or custom lenses exist, it outputs a
markdown block with two sections: settings overrides and a unified lens
catalogue.

```markdown
## Review Configuration

The following review settings are configured for this project. These override
the defaults specified later in this skill:

- **Max inline comments**: 15 (default: 10)
- **Lens count range**: 3 to 10 (default: 4 to 8)
- **Dedup proximity**: 5 lines (default: 3)
- **Core lenses**: architecture, security, test-coverage, correctness
  (default: architecture, code-quality, test-coverage, correctness)
- **Disabled lenses**: portability, compatibility
  (these lenses should be skipped regardless of auto-detect)
- **PR verdict**: REQUEST_CHANGES when any `major` or higher
  (default: any `critical`)

### Lens Catalogue

Use the paths below when constructing agent prompts. Always use the path
from this table rather than constructing paths from the lens name.

| Lens | Path | Source |
|------|------|--------|
| architecture | /path/to/plugin/skills/review/lenses/architecture-lens/SKILL.md | built-in |
| code-quality | /path/to/plugin/skills/review/lenses/code-quality-lens/SKILL.md | built-in |
| ... | ... | ... |
| compliance | /path/to/project/.claude/accelerator/lenses/compliance-lens/SKILL.md | custom |
| accessibility | /path/to/project/.claude/accelerator/lenses/accessibility-lens/SKILL.md | custom (always include) |
```

Custom lenses that provide an `auto_detect` field in their SKILL.md
frontmatter are treated like any non-core built-in lens: they participate in
auto-detect selection and are subject to the `max_lenses` cap. Custom lenses
that omit `auto_detect` default to always-included (bypassing auto-detect)
for backward compatibility. The source column notes "custom (always include)"
for lenses without auto-detect criteria.

Built-in lens paths are derived from `SCRIPT_DIR` (i.e.,
`$SCRIPT_DIR/../skills/review/lenses/[lens]-lens/SKILL.md`) rather than
relying on `CLAUDE_PLUGIN_ROOT` as an environment variable, since the plugin
system does not guarantee env var export to preprocessor command processes.
Custom lens paths are absolute paths constructed from the project root
returned by `config_project_root`.

Implementation notes:
- Use `config-read-value.sh` from Plan 1 for individual key reads
- For array values (`core_lenses`, `disabled_lenses`), use the shared
  `config_parse_array` function (see below) to parse the `[a, b, c]` format
  returned as a raw string by `config-read-value.sh`
- The built-in lens catalogue is hardcoded as a list of 13 names in the script
  (matching the directories under `skills/review/lenses/`). Built-in lens
  paths are derived from `SCRIPT_DIR` (`$SCRIPT_DIR/../skills/review/lenses/
  [lens]-lens/SKILL.md`). This avoids a runtime dependency on
  `CLAUDE_PLUGIN_ROOT` as an environment variable, which is not guaranteed to
  be exported to preprocessor command processes.
- The script always emits a unified lens catalogue table containing both
  built-in and custom lenses with their absolute paths. The orchestrator uses
  this table to construct agent prompts, eliminating the need to distinguish
  built-in from custom lenses when looking up paths.
- Only output changed values (don't repeat defaults)
- The script must execute in this order:
  1. Read all config values (numeric settings, arrays, severity keys)
  2. Validate numeric values. `max_inline_comments` and `dedup_proximity`
     must be non-negative integers (zero is valid). `min_lenses`,
     `max_lenses`, and `plan_revise_major_count` must be positive integers.
     `min_lenses` ≤ `max_lenses`. Emit a warning to stderr and fall back to
     defaults for invalid values.
  3. Discover custom lenses by scanning
     `.claude/accelerator/lenses/*/SKILL.md`. Perform a lightweight structural
     check: verify the file contains YAML frontmatter with a `name` field.
     Emit a warning to stderr and skip lenses that fail this check. Also
     extract the optional `auto_detect` frontmatter field. If present, it is
     included in the output so orchestrators can use it for auto-detect
     selection. If absent, the output notes "always include" so orchestrators
     treat the lens as always-selected.
  4. Validate lens names in `disabled_lenses` and `core_lenses` against the
     combined set of built-in lenses and discovered custom lenses. Warn to
     stderr on unrecognised names (catches typos like `code_quality` instead
     of `code-quality`). Custom lens discovery (step 3) must complete before
     this validation runs, otherwise custom lens names in `core_lenses` would
     produce spurious warnings.
- If a lens appears in both `core_lenses` and `disabled_lenses`, emit a
  warning to stderr. `disabled_lenses` takes precedence (disabling is explicit
  user intent).
- If the number of remaining available lenses (built-in minus disabled, plus
  custom) is less than `min_lenses`, emit a warning to stderr noting the
  conflict. This catches configurations that disable too many lenses.

#### 1a. Shared Array Parsing Utility

**File**: `scripts/config-common.sh`
**Changes**: Add a `config_parse_array` function to the shared utilities.

This function takes a raw string like `[a, b, c]` (as returned by
`config-read-value.sh`) and outputs one element per line. It handles:
- Stripping brackets: `[a, b, c]` → `a, b, c`
- Splitting on commas: `a, b, c` → `a\nb\nc`
- Trimming whitespace around each element
- Empty arrays `[]` → no output
- Single-element arrays `[architecture]` → `architecture`
- Hyphenated names `[code-quality, test-coverage]` → preserved correctly
- Inconsistent spacing `[a,b, c , d]` → `a\nb\nc\nd`

```bash
# Parse a YAML-style inline array string into one element per line.
# Input: "[a, b, c]" (as returned by config-read-value.sh)
# Output: one element per line, whitespace-trimmed
# Empty input or "[]" produces no output.
config_parse_array() {
  local raw="$1"
  # Strip brackets
  raw="${raw#\[}"
  raw="${raw%\]}"
  # Empty after stripping → nothing to output
  [ -z "$raw" ] && return 0
  # Split on commas and trim whitespace
  echo "$raw" | tr ',' '\n' | while IFS= read -r item; do
    # Trim leading/trailing whitespace
    item=$(echo "$item" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
    [ -n "$item" ] && echo "$item"
  done
}
```

#### 1b. Config Dump Script

**File**: `scripts/config-dump.sh`
**Changes**: New file. Outputs all configured review keys with their effective
values and source attribution (team vs local). This plan brings the total
config keys to 9+ (exceeding the 5-key threshold from Plan 1).

The script outputs a formatted table showing each key, its effective value,
and where it came from:

```markdown
## Effective Configuration

| Key | Value | Source |
|-----|-------|--------|
| `review.max_inline_comments` | `15` | team (.claude/accelerator.md) |
| `review.min_lenses` | `4` | default |
| `review.max_lenses` | `10` | local (.claude/accelerator.local.md) |
| `review.dedup_proximity` | `3` | default |
| `review.core_lenses` | `[architecture, security, test-coverage, correctness]` | team (.claude/accelerator.md) |
| `review.disabled_lenses` | `[]` | default |
| `review.pr_request_changes_severity` | `critical` | default |
| `review.plan_revise_severity` | `critical` | default |
| `review.plan_revise_major_count` | `3` | default |
```

Implementation notes:
- Iterate all known review config keys, calling `config-read-value.sh` for
  each, and track which file provided the value (team, local, or default)
- To determine source attribution, read each config file individually before
  falling back to defaults
- Output nothing if no config files exist (matches convention)
- Include agent config keys from Plan 2 as well (iterate `agents.*` keys)

#### 2. Test Updates

**File**: `scripts/test-config.sh`
**Changes**: Add test cases for `config-read-review.sh`:

**Basic behaviour:**
- No review config → outputs nothing
- Partial config (only some keys) → outputs only changed values
- Full config → outputs all overrides

**Array parsing (via `config_parse_array`):**
- `disabled_lenses: [portability, compatibility]` → correctly parsed
- `core_lenses: [architecture, code-quality, test-coverage, correctness]`
  → hyphenated names preserved
- Single-element array `[architecture]` → one element
- Empty array `[]` → no output
- Array with inconsistent spacing `[a,b, c , d]` → all trimmed correctly

**Custom lens discovery:**
- Custom lens directory with valid SKILL.md → listed in output
- Multiple custom lens directories → all listed
- Directory without SKILL.md → skipped (not listed)
- Custom lens with missing `name` in frontmatter → warning, skipped
- Custom lens with same name as built-in lens → warning to stderr
- No `.claude/accelerator/lenses/` directory → no custom lenses listed
- Custom lens with `auto_detect` field → auto-detect criteria included in output
- Custom lens without `auto_detect` field → output shows "always include"

**Validation:**
- Negative `min_lenses` → warning to stderr, falls back to default
- `min_lenses > max_lenses` → warning to stderr, falls back to defaults
- Unrecognised lens name in `disabled_lenses` → warning to stderr
- Lens in both `core_lenses` and `disabled_lenses` → warning to stderr
- Invalid severity value for verdict key → warning to stderr, default used
- Non-integer `plan_revise_major_count` → warning to stderr, default used
- `disabled_lenses` disabling enough lenses to drop available below
  `min_lenses` → warning to stderr

**Verdict threshold config:**
- `pr_request_changes_severity: major` → output shows override
- `pr_request_changes_severity: none` → output shows override (disables
  severity-based REQUEST_CHANGES)
- `plan_revise_severity: none` → output shows override (disables
  severity-based REVISE; major count rule still applies)
- `plan_revise_major_count: 2` → output shows override
- `plan_revise_severity: critical` (same as default) → not listed as override
- Unrecognised severity value → warning to stderr, default used

**Config variable consistency:**
- Every config variable name emitted by `config-read-review.sh pr` output
  appears somewhere in `review-pr/SKILL.md` (catches naming mismatches)
- Every config variable name emitted by `config-read-review.sh plan` output
  appears somewhere in `review-plan/SKILL.md`
- Shared variables (lens settings) appear in both skill files

**config-dump.sh:**
- No config files → outputs nothing
- Team-only config → all keys shown with correct source attribution
- Local-only config → all keys shown with "local" source
- Merged config (team + local override) → overridden key shows "local" source,
  others show "team" or "default"
- Default keys (no config for that key) → shown with "default" source
- All review config keys appear in output (completeness check)

**Default preservation:**
- No config files → empty output (regression guard)

### Success Criteria:

#### Automated Verification:

- [ ] `scripts/config-read-review.sh` exists and is executable
- [ ] `scripts/config-dump.sh` exists and is executable
- [ ] `bash scripts/test-config.sh` passes all tests

---

## Phase 2: Update Review-PR Skill

### Overview

Update `review-pr/SKILL.md` to read review configuration via the preprocessor
and use configured values instead of hardcoded defaults.

### Changes Required:

#### 1. Add Preprocessor Block

**File**: `skills/github/review-pr/SKILL.md`
**Changes**: Add the review config preprocessor line after the context and
agent override preprocessor lines (added by Plan 2), before the
"## Initial Response" section.

```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-review.sh pr`
```

#### 2. Update Lens Selection Instructions

**File**: `skills/github/review-pr/SKILL.md`
**Changes**: Modify the lens selection section (around lines 166-184) to
reference the configuration:

Replace the hardcoded "**6 to 8 most relevant lenses**" text with:

```markdown
**Lens selection cap:** Select the most relevant lenses for the change under
review. If review configuration is provided above, use the configured
`min_lenses` and `max_lenses` values. Otherwise, use the defaults: **6 to 8**
lenses. Apply these prioritisation rules:

Apply this lens selection pipeline in order:

1. **Start with all available lenses**: the 13 built-in lenses plus any
   custom lenses listed in the review configuration above.
2. **Remove disabled lenses**: if review configuration specifies
   `disabled_lenses`, remove those from the available set. They are never
   selected regardless of auto-detect criteria.
3. **Mark core lenses**: if review configuration specifies `core_lenses`,
   use that list. Otherwise, the core lenses are Architecture, Code Quality,
   Test Coverage, and Correctness. Core lenses are included unless the change
   is clearly outside their scope.
4. **Auto-detect remaining lenses**: use the criteria below (for built-in
   lenses) and the auto-detect criteria from review configuration (for custom
   lenses) to identify which non-core lenses are relevant to the change.
   Custom lenses that provide auto-detect criteria participate in selection
   like any other non-core lens. Custom lenses without auto-detect criteria
   (marked "always include" in the configuration) are always selected. Custom
   lenses use absolute paths instead of the `${CLAUDE_PLUGIN_ROOT}` lens
   path template.
5. **Apply focus arguments**: if the user provided focus areas, prioritise
   the corresponding lenses and fill remaining slots with auto-detected ones.
6. **Cap at `max_lenses`**: if more lenses than the configured maximum pass
   selection, rank by relevance and drop the least relevant. Prefer lenses
   whose core responsibilities directly overlap with the change's concerns.
7. **Enforce `min_lenses` floor**: never run fewer than `min_lenses` unless
   the change is trivially scoped.
```

#### 3. Update Inline Comment Cap

**File**: `skills/github/review-pr/SKILL.md`
**Changes**: Modify the inline comment cap reference (around line 326):

Replace:
```
- Select up to 10 comments total for inline posting
```

With:
```
- Select up to the configured `max_inline_comments` (default: 10) comments
  total for inline posting (more if all critical findings push beyond the cap)
```

Also update the guideline at line 519:
```
9. **Cap inline comments** — if agents produce more findings, prioritise
   critical and major severity. Use the configured max (default: 10). Always
   include all critical findings even if that exceeds the cap. Move overflow
   to the summary body. This prevents PR comment spam.
```

#### 4. Update Dedup Proximity

**File**: `skills/github/review-pr/SKILL.md`
**Changes**: Modify the dedup proximity reference (around line 309):

Replace:
```
same side, and overlapping or adjacent line range (same path, lines within
3 of each other)
```

With:
```
same side, and overlapping or adjacent line range (same path, lines within
the configured dedup proximity — default: 3 — of each other)
```

#### 5. Update Verdict Thresholds

**File**: `skills/github/review-pr/SKILL.md`
**Changes**: Modify the verdict determination section (around lines 331-334).

Current:
```markdown
6. **Determine suggested verdict**:
   - If any `"critical"` severity findings exist → suggest `REQUEST_CHANGES`
   - If only `"major"` or lower → suggest `COMMENT`
   - If no findings at all (only strengths) → suggest `APPROVE`
```

Replace with:
```markdown
6. **Determine suggested verdict**:

   If review configuration provides verdict overrides above, apply those
   thresholds instead of the defaults below:
   - If `pr_request_changes_severity` is `none`, skip this rule (never
     suggest REQUEST_CHANGES based on severity)
   - If any findings at or above the configured `pr_request_changes_severity`
     (default: `critical`) exist → suggest `REQUEST_CHANGES`
   - If only findings below that threshold → suggest `COMMENT`
   - If no findings at all (only strengths) → suggest `APPROVE`
```

#### 6. Update Lens Path Handling in Agent Prompt Template

**File**: `skills/github/review-pr/SKILL.md`
**Changes**: In the agent prompt template (around lines 246-251), replace
the hardcoded lens path template with instructions to use the unified lens
catalogue from the review configuration:

Replace:
```markdown
Read the lens skill at: ${CLAUDE_PLUGIN_ROOT}/skills/review/lenses/[lens]-lens/SKILL.md
```

With:
```markdown
Read the lens skill at the path listed in the Lens Catalogue table in the
review configuration above. If no review configuration is present, use:
${CLAUDE_PLUGIN_ROOT}/skills/review/lenses/[lens]-lens/SKILL.md
```

### Success Criteria:

#### Automated Verification:

- [ ] `grep -c 'config-read-review.sh' skills/github/review-pr/SKILL.md` returns 1
- [ ] Skill file contains references to configured values instead of only
  hardcoded numbers

#### Manual Verification:

- [ ] With no review config, review-pr behaves exactly as before
- [ ] With `disabled_lenses: [portability, compatibility]`, those lenses are
  skipped
- [ ] With `max_inline_comments: 5`, only 5 comments are posted (plus critical)
- [ ] With a custom lens directory, the custom lens appears in selection

---

## Phase 3: Update Review-Plan Skill

### Overview

Apply the applicable changes to `review-plan/SKILL.md` from Phase 2. The
review-plan skill shares the same lens system and selection logic but does
**not** have inline comment posting or dedup proximity (those are PR-specific).

Specifically, mirror Phase 2 Steps 1 (preprocessor block), 2 (lens
selection instructions), and 6 (lens path handling in agent prompt template).
Skip Steps 3-4 (inline comment cap and dedup proximity — PR-only). Steps 3
and 4 below handle review-plan's verdict thresholds and the verdict config
output, which differ from review-pr.

### Changes Required:

#### 1. Add Preprocessor Block

**File**: `skills/planning/review-plan/SKILL.md`
**Changes**: Add the review config preprocessor line after the context and
agent override preprocessor lines.

```markdown
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-review.sh plan`
```

#### 2. Update Lens Selection Instructions

**File**: `skills/planning/review-plan/SKILL.md`
**Changes**: Mirror the changes made to `review-pr/SKILL.md` in Phase 2,
Step 2. Update the lens selection section (around lines 142-160) with the
same configurable language.

#### 2a. Update Lens Path Handling in Agent Prompt Template

**File**: `skills/planning/review-plan/SKILL.md`
**Changes**: Mirror Phase 2, Step 6. In the agent prompt template (around
line 216), replace the hardcoded lens path template with instructions to use
the unified lens catalogue from the review configuration:

Replace:
```markdown
Read the lens skill at: ${CLAUDE_PLUGIN_ROOT}/skills/review/lenses/[lens]-lens/SKILL.md
```

With:
```markdown
Read the lens skill at the path listed in the Lens Catalogue table in the
review configuration above. If no review configuration is present, use:
${CLAUDE_PLUGIN_ROOT}/skills/review/lenses/[lens]-lens/SKILL.md
```

#### 3. Update Verdict Thresholds

**File**: `skills/planning/review-plan/SKILL.md`
**Changes**: Modify the verdict determination section (around lines 281-285).

Current:
```markdown
5. **Determine suggested verdict**:
   - If any `"critical"` severity findings exist → suggest `REVISE`
   - If 3 or more `"major"` severity findings exist → suggest `REVISE`
   - If 1-2 `"major"` findings or only minor/suggestion → suggest `COMMENT`
   - If no findings at all (only strengths) → suggest `APPROVE`
```

Replace with:
```markdown
5. **Determine suggested verdict**:

   If review configuration provides verdict overrides above, apply those
   thresholds instead of the defaults below:
   - If `plan_revise_severity` is `none`, skip the severity-based REVISE
     rule (major count rule still applies independently)
   - If any findings at or above `plan_revise_severity` (default: `critical`)
     exist → suggest `REVISE`
   - If `plan_revise_major_count` or more `"major"` findings exist (default: 3)
     → suggest `REVISE`
   - If fewer major findings than the threshold, or only minor/suggestion
     → suggest `COMMENT`
   - If no findings at all (only strengths) → suggest `APPROVE`
```

#### 4. Verdict Config Output

The verdict threshold config keys (`pr_request_changes_severity`,
`plan_revise_severity`, `plan_revise_major_count`) are defined in the Phase 1
config key table alongside all other review keys. When configured, the script
outputs the relevant verdict override in the config block based on the mode
argument:

In `pr` mode:
```markdown
- **Verdict**: REQUEST_CHANGES when any `major` or higher
  (default: any `critical`)
```

In `plan` mode:
```markdown
- **Verdict**: REVISE when any `critical` or 2+ `major`
  (default: any `critical` or 3+ `major`)
```

### Success Criteria:

#### Automated Verification:

- [ ] `grep -c 'config-read-review.sh' skills/planning/review-plan/SKILL.md`
  returns 1
- [ ] Both orchestrator skills reference configurable values for lens selection,
  verdict thresholds, and numeric limits

#### Manual Verification:

- [ ] With no review config, both skills behave exactly as before
- [ ] With modified verdict thresholds, verdicts change accordingly
- [ ] Lens selection respects disabled_lenses and core_lenses overrides
  consistently in both skills

---

## Phase 4: Configure Skill & Documentation Updates

### Overview

Update the configure skill and README with review configuration documentation.

### Changes Required:

#### 1. Configure Skill Update

**File**: `skills/config/configure/SKILL.md`
**Changes**: Add review configuration documentation to the help section.

```markdown
### review

Customise review behaviour for `/accelerator:review-pr` and
`/accelerator:review-plan`. Config keys use underscores (e.g.,
`max_inline_comments`). Lens names within array values use their original
hyphenated form (e.g., `code-quality`, `test-coverage`):

Shared settings (apply to both `review-pr` and `review-plan`):

| Key | Default | Description |
|-----|---------|-------------|
| `min_lenses` | `4` | Minimum lenses to run |
| `max_lenses` | `8` | Maximum lenses to run |
| `core_lenses` | `[architecture, code-quality, test-coverage, correctness]` | Lenses considered "core four" |
| `disabled_lenses` | `[]` | Lenses to never use |

PR review only (`review-pr`):

| Key | Default | Description |
|-----|---------|-------------|
| `max_inline_comments` | `10` | Max inline comments |
| `dedup_proximity` | `3` | Line proximity for merging findings |
| `pr_request_changes_severity` | `critical` | Min severity for REQUEST_CHANGES (`critical`, `major`, or `none`) |

Plan review only (`review-plan`):

| Key | Default | Description |
|-----|---------|-------------|
| `plan_revise_severity` | `critical` | Min severity for REVISE (`critical`, `major`, or `none`) |
| `plan_revise_major_count` | `3` | Major findings count to trigger REVISE |

Example configuration:

\```yaml
---
review:
  min_lenses: 3
  max_lenses: 10
  core_lenses: [architecture, security, test-coverage, correctness]
  disabled_lenses: [portability, compatibility]
  max_inline_comments: 15
  dedup_proximity: 5
  pr_request_changes_severity: major
  plan_revise_severity: critical
  plan_revise_major_count: 2
---
\```

Note: YAML comments (`#`) are not supported by the config parser. Do not
add inline comments to config values.

#### Custom Lenses

Create custom review lenses in `.claude/accelerator/lenses/`:

\```
.claude/accelerator/lenses/
  compliance-lens/
    SKILL.md           # Follow the same structure as built-in lenses
  accessibility-lens/
    SKILL.md
\```

Custom lenses are auto-discovered and added to the available lens catalogue.
They must have YAML frontmatter with a `name` field and follow the same
SKILL.md structure as built-in lenses. Custom lenses that provide an
`auto_detect` field participate in auto-detect selection like built-in
lenses. Those without `auto_detect` are always included. Minimal template:

\```markdown
---
name: compliance
description: Evaluates regulatory and policy compliance
auto_detect: Relevant when changes touch regulatory, compliance, or policy-related code
---

# Compliance Lens

## Core Responsibilities
- [What this lens evaluates]

## Key Questions
1. [Questions the reviewer should ask through this lens]

## Boundary
- [What is NOT in scope for this lens]
\```

See any lens in the plugin's `skills/review/lenses/` directory for full
examples of the expected structure.
```

#### 2. README Update

**File**: `README.md`
**Changes**: Update the Configuration section (added in Plan 1) with review
configuration details.

### Success Criteria:

#### Manual Verification:

- [ ] `/accelerator:configure help` shows review configuration options
- [ ] README documents review configuration

---

## Testing Strategy

### Unit Tests:

- `config-read-review.sh` tests in `test-config.sh`
- `config_parse_array` function tests (direct function tests in
  `test-config.sh` sourcing `config-common.sh`):
  - Empty arrays, single-item arrays, arrays with spaces, arrays with hyphens,
    inconsistent spacing
- Verdict threshold config: valid severity values, invalid values, numeric
  thresholds, default preservation
- Validation warnings: numeric range checks, unrecognised lens names,
  `disabled/core` conflicts, invalid severity values
- Custom lens discovery: valid lenses, missing SKILL.md, missing frontmatter
  `name`, name conflicts with built-in lenses
- `config-dump.sh` tests: no config (empty output), team-only, local-only,
  merged with source attribution, default keys

### Manual Testing Steps:

1. Create config with `review.disabled_lenses: [portability, compatibility]`
2. Run `/accelerator:review-pr` on a PR
3. Verify portability and compatibility are not in the lens selection
4. Create config with `review.max_inline_comments: 3`
5. Review a PR with many findings — verify only 3 inline comments (plus critical)
6. Add a custom lens at `.claude/accelerator/lenses/compliance-lens/SKILL.md`
7. Run `/accelerator:review-pr` — verify the custom lens appears in selection

## Configure Skill Updates (Deferred from Plan 1)

Plan 1 (Configuration Infrastructure) deferred configure skill key
documentation to each plan that introduces keys. This plan must:

1. **Update the configure skill's `help` action**
   (`skills/config/configure/SKILL.md`) to add documentation of the `review`
   section keys:
   - `review.disabled_lenses`, `review.max_inline_comments`,
     `review.min_lenses`, `review.dedup_proximity`, etc. — full list with
     defaults

2. **Update the configure skill's `create` action** to mention that review
   customisation is available and point users to `/accelerator:configure help`
   for the full key reference. Do not walk through each review key
   interactively — the number of keys with non-trivial interactions (enum
   values, cross-key constraints) makes interactive prompting tedious. Users
   can copy the example from `help` and edit it.

3. **`scripts/config-dump.sh`** — included in Phase 1, Step 1b of this plan
   (9+ keys exceed the 5-key threshold from Plan 1).

## References

- Plan 1: `meta/plans/2026-03-23-config-infrastructure.md`
- Plan 2: `meta/plans/2026-03-23-context-and-agent-customisation.md`
- Research: `meta/research/2026-03-22-skill-customisation-and-override-patterns.md`
- Review PR skill: `skills/github/review-pr/SKILL.md`
- Review plan skill: `skills/planning/review-plan/SKILL.md`
- Lens structure analysis: (see research agents output in this session)
