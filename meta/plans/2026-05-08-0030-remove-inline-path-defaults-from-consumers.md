---
date: "2026-05-08T00:00:00Z"
type: plan
skill: create-plan
work-item: "0030"
status: draft
---

# Remove Inline Path Defaults from Consumer Call Sites

## Overview

Make the `[default]` argument to `scripts/config-read-path.sh` truly optional by
looking it up from the centralized `scripts/config-defaults.sh` when not supplied.
Then strip the redundant hardcoded default string from every consumer call site —
61 SKILL.md bang-command lines across 23 files and 13 bash consumer scripts.

This is the consumer-site half of work item 0030. Work item 0030 centralised
the array *definitions* into `config-defaults.sh`; this plan eliminates the
parallel duplication at call sites, so a future path-default rename requires only
a one-line edit to `config-defaults.sh`.

## Current State Analysis

`scripts/config-defaults.sh` (created by 0030) defines:

- `PATH_KEYS` — 11 entries (`paths.plans` … `paths.notes`)
- `PATH_DEFAULTS` — 11 paired defaults (`meta/plans` … `meta/notes`)
- `TEMPLATE_KEYS` — 6 entries

`scripts/config-read-path.sh` takes `<key> [default]`, prepends `paths.`, then
delegates to `config-read-value.sh`:

```bash
exec "$SCRIPT_DIR/config-read-value.sh" "paths.${1:-}" "${2:-}"
```

When `$2` is absent, `${2:-}` expands to empty string, which `config-read-value.sh`
outputs verbatim when the key is missing from config — i.e. callers currently *must*
supply the default or they get empty output for unconfigured keys.

Four keys used in SKILL.md and bash consumers are not yet in `config-defaults.sh`:
- `tmp` → `.accelerator/tmp`
- `integrations` → `.accelerator/state/integrations`
- `design_inventories` → `meta/design-inventories`
- `design_gaps` → `meta/design-gaps`

### Key Discoveries

- `scripts/config-read-path.sh` does **not** source `config-common.sh`; it only
  sets `SCRIPT_DIR`. Sourcing `config-defaults.sh` directly is lightweight and
  correct — `config-defaults.sh` has no dependencies of its own.
- `scripts/config-read-value.sh:22` — `DEFAULT="${2:-}"`. When `$2` is empty
  (or absent), `DEFAULT` is empty and the script outputs nothing for unset keys.
  The fix lives entirely in `config-read-path.sh`, not `config-read-value.sh`.
- `skills/visualisation/visualise/scripts/write-visualiser-config.sh:34-36`
  defines a two-argument `resolve_path()` wrapper and a two-argument `abs_path()`
  wrapper; 12 call sites use `abs_path <key> <literal>`. Both wrappers and all
  call sites require updating (Phase 3).
- `skills/config/init/scripts/init.sh:34` calls
  `bash "$CONFIG_READ_PATH" "$key" "$default"` where `$default` comes from
  `DIR_DEFAULTS` — a variable, not a literal. Only line 52
  (`bash "$CONFIG_READ_PATH" tmp .accelerator/tmp`) carries a hardcoded literal
  and is in scope for Phase 3.
- `skills/planning/create-plan/SKILL.md:211` contains a live bang-command
  reference (`!`...config-read-path.sh plans meta/plans``) inside the plan
  filename instruction — it IS preprocessed at invocation time and must be
  updated.
- `scripts/test-config.sh` legitimately passes explicit defaults in regression
  tests (lines 2675, 2687, 2699, …) to verify the backward-compatible
  explicit-override path. These tests are *not* changed; the structural test in
  Phase 3 excludes `test-config.sh`.

## Desired End State

After the plan completes:

1. `scripts/config-defaults.sh` defines 15 path key/default pairs (11 existing
   plus `tmp`, `integrations`, `design_inventories`, `design_gaps`).
2. `scripts/config-read-path.sh` makes `[default]` optional: if `$2` is absent
   or empty, it looks up the default from `PATH_DEFAULTS` via `config-defaults.sh`.
   Explicit `$2` still takes precedence (backward compatible).
3. No SKILL.md or bash script (outside `test-config.sh` and migration scripts)
   passes a hardcoded literal second argument to `config-read-path.sh`.

### Verification

```bash
mise run test:integration:config

# AC: no hardcoded defaults in SKILL.md files
grep -rn --include='SKILL.md' --exclude-dir=workspaces \
  -E 'config-read-path\.sh [a-z_]+ [^$\n]' .
# Expected: no output

# AC: no hardcoded defaults in bash scripts (excluding test and migration files)
# Pattern uses "? to match both bare (SKILL.md) and quoted-path ("$VAR/config-read-path.sh")
# invocation styles used in bash consumers.
grep -rn --include='*.sh' --exclude-dir=workspaces \
  --exclude='test-config.sh' \
  -E 'config-read-path\.sh"?[[:space:]]+[a-zA-Z_][a-zA-Z0-9_]*[[:space:]]+[^$"\n[:space:]]' . | grep -v '/migrations/'
# Expected: no output
```

## What We're NOT Doing

- **Changing `init.sh` line 34** (`bash "$CONFIG_READ_PATH" "$key" "$default"`).
  This iterates DIR_KEYS/DIR_DEFAULTS — variable references, not inline literals.
  The DIR_KEYS/DIR_DEFAULTS unification question is deferred (0030 Open Questions).
- **Changing `config-read-value.sh`.** All changes are confined to
  `config-read-path.sh` and its callers.
- **Supporting `""` as a "suppress lookup" sentinel.** The proposed guard
  `[ -n "${2:-}" ]` treats an explicit empty-string `$2` identically to an
  absent `$2` — both trigger the centralized lookup. No current caller passes
  `""` as a second argument, and the usage comment documents this contract.
  If future callers need to force empty output, they should supply a
  non-matching explicit default rather than `""`.
- **Stripping explicit defaults from `test-config.sh`.** Regression tests
  legitimately exercise the explicit-override path; they are not consumers.
- **Changing migration scripts** that hardcode path defaults for safety.
- **Adding `design_inventories`/`design_gaps` to the comment block in
  `config-read-path.sh`.** The comment was already a partial enumeration; this
  plan leaves comment maintenance to the implementer's discretion.

## Implementation Approach

Three TDD phases. Each introduces a failing test that captures the invariant
being established, then makes the minimum changes to pass it. Existing
`config-read-path.sh` tests (explicit-default path) serve as the regression
suite throughout.

---

## Phase 1: Extend `config-defaults.sh` to cover all path keys

### Overview

Add `tmp`, `integrations`, `design_inventories`, and `design_gaps` to
`PATH_KEYS`/`PATH_DEFAULTS` in `scripts/config-defaults.sh`. A new test block
extension asserts the four additions before they are added.

### Changes Required

#### 1. Extend the test block in `scripts/test-config.sh`

**File**: `scripts/test-config.sh`
**Changes**: In the existing `=== config-defaults.sh ===` block, update the
length assertions from 11 → 15 and the content assertions to include the four
new entries. The entries must appear after the existing 11 in the order shown
below, so that `PATH_KEYS[i]` and `PATH_DEFAULTS[i]` remain paired by index.

Updated expected values:
- `PATH_KEYS` (15): append `paths.tmp`, `paths.integrations`,
  `paths.design_inventories`, `paths.design_gaps`
- `PATH_DEFAULTS` (15): append `.accelerator/tmp`,
  `.accelerator/state/integrations`, `meta/design-inventories`,
  `meta/design-gaps`

Run the suite — the length and contents assertions fail.

#### 2. Extend `scripts/config-defaults.sh`

**File**: `scripts/config-defaults.sh`
**Changes**: Append four entries to `PATH_KEYS` and four to `PATH_DEFAULTS`:

```bash
PATH_KEYS=(
  "paths.plans"
  "paths.research"
  "paths.decisions"
  "paths.prs"
  "paths.validations"
  "paths.review_plans"
  "paths.review_prs"
  "paths.review_work"
  "paths.templates"
  "paths.work"
  "paths.notes"
  "paths.tmp"
  "paths.integrations"
  "paths.design_inventories"
  "paths.design_gaps"
)

PATH_DEFAULTS=(
  "meta/plans"
  "meta/research"
  "meta/decisions"
  "meta/prs"
  "meta/validations"
  "meta/reviews/plans"
  "meta/reviews/prs"
  "meta/reviews/work"
  ".accelerator/templates"
  "meta/work"
  "meta/notes"
  ".accelerator/tmp"
  ".accelerator/state/integrations"
  "meta/design-inventories"
  "meta/design-gaps"
)
```

Run the suite — Phase 1 tests now pass. The AC2 single-definition-site test
also continues to pass (only `config-defaults.sh` defines the arrays).

### Success Criteria

#### Automated Verification

- [ ] `mise run test:integration:config` passes with updated length assertions
      (11 → 15) and content assertions for `PATH_KEYS` and `PATH_DEFAULTS`.
- [ ] AC2 grep still returns only `./scripts/config-defaults.sh`.

#### Manual Verification

- [ ] Four new entries at the end of each array in `config-defaults.sh`; order
      and content match the table above.

---

## Phase 2: Make `[default]` optional in `config-read-path.sh`

### Overview

Update `scripts/config-read-path.sh` to look up the default from
`config-defaults.sh` when `$2` is absent or empty. Add tests that call the
script without a second argument and assert the centralized default is returned.

### Changes Required

#### 1. Add "no-default" tests to `scripts/test-config.sh`

**File**: `scripts/test-config.sh`
**Changes**: Append a new `=== config-read-path.sh (no-default lookup) ===`
block after the existing `=== config-read-path.sh ===` block. Each test calls
`bash "$READ_PATH" "<key>"` (no `$2`) against a repo with no config and asserts
the centralized default is returned. Cover a representative sample:

```bash
echo "=== config-read-path.sh (no-default lookup) ==="
echo ""

echo "Test: plans key → meta/plans with no $2"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" plans)
assert_eq "plans default" "meta/plans" "$OUTPUT"

echo "Test: tmp key → .accelerator/tmp with no $2"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" tmp)
assert_eq "tmp default" ".accelerator/tmp" "$OUTPUT"

echo "Test: integrations key → .accelerator/state/integrations with no $2"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" integrations)
assert_eq "integrations default" ".accelerator/state/integrations" "$OUTPUT"

echo "Test: design_inventories key → meta/design-inventories with no $2"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" design_inventories)
assert_eq "design_inventories default" "meta/design-inventories" "$OUTPUT"

echo "Test: design_gaps key → meta/design-gaps with no $2"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" design_gaps)
assert_eq "design_gaps default" "meta/design-gaps" "$OUTPUT"

echo "Test: templates key → .accelerator/templates with no \$2"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" templates)
assert_eq "templates default" ".accelerator/templates" "$OUTPUT"

echo "Test: no-\$2 returns configured value when key is set in config"
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator"
cat > "$REPO/.accelerator/config.md" << 'FIXTURE'
---
paths:
  work: docs/work-items
---
FIXTURE
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" work)
assert_eq "config-set value with no \$2" "docs/work-items" "$OUTPUT"

echo "Test: unknown key returns empty output with no \$2"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" unknown_key 2>/dev/null || true)
assert_eq "unknown key returns empty" "" "$OUTPUT"

echo "Test: explicit \$2 still overrides centralized default"
REPO=$(setup_repo)
OUTPUT=$(cd "$REPO" && bash "$READ_PATH" plans custom/plans)
assert_eq "explicit override" "custom/plans" "$OUTPUT"

echo ""
```

Run the suite — "no $2" tests fail (all return empty output).

#### 2. Update `scripts/config-read-path.sh`

**File**: `scripts/config-read-path.sh`
**Changes**: Source `config-defaults.sh` directly; resolve the default from
`PATH_DEFAULTS` when `$2` is absent or empty; retain explicit `$2` as override.

Replace the entire file body (keeping the shebang and comment block, updating
the `[default]` note):

```bash
#!/usr/bin/env bash
set -euo pipefail

# Reads a path configuration value.
# Usage: config-read-path.sh <path_key> [default]
#
# When [default] is omitted or empty (both treated identically), the
# plugin-standard default for the key is looked up from config-defaults.sh.
# An explicit non-empty [default] takes precedence (backward compatible).
#
# If the key is not found in PATH_KEYS and no explicit [default] is provided,
# a warning is printed to stderr and the output is empty. Callers that need
# a non-empty fallback for unknown keys should supply an explicit [default].
#
# Path keys and their plugin-standard defaults are defined in config-defaults.sh.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=config-defaults.sh
source "$SCRIPT_DIR/config-defaults.sh"

key="${1:-}"
if [ -z "$key" ]; then
  echo "Usage: config-read-path.sh <path_key> [default]" >&2
  exit 1
fi

if [ -n "${2:-}" ]; then
  default="${2}"
else
  default=""
  for i in "${!PATH_KEYS[@]}"; do
    if [ "${PATH_KEYS[$i]}" = "paths.${key}" ]; then
      default="${PATH_DEFAULTS[$i]}"
      break
    fi
  done
  if [ -z "$default" ]; then
    echo "config-read-path.sh: warning: unknown key '${key}' — no centralized default" >&2
  fi
fi

exec "$SCRIPT_DIR/config-read-value.sh" "paths.${key}" "${default}"
```

Run the suite — all Phase 2 tests pass. Existing explicit-default tests
(`bash "$READ_PATH" plans meta/plans`) continue to pass unchanged.

#### 3. Update the governance comment in `scripts/config-defaults.sh`

**File**: `scripts/config-defaults.sh`
**Changes**: Replace line 19 (`# Do not source this file directly — source config-common.sh instead.`)
with a comment that reflects the new reality:

```bash
# config-read-path.sh sources this file directly (it cannot afford the VCS
# detection overhead pulled in via config-common.sh). All other consumers
# should source config-common.sh, which sources this file transitively.
```

### Success Criteria

#### Automated Verification

- [ ] New `=== config-read-path.sh (no-default lookup) ===` block passes for
      all nine tests (plans, tmp, integrations, design_inventories, design_gaps,
      templates, config-set value with no `$2`, unknown key returns empty,
      explicit override).
- [ ] All existing `=== config-read-path.sh ===` tests continue to pass
      (explicit `$2` path is unaffected).
- [ ] `mise run test:integration:config` passes.

#### Manual Verification

- [ ] `bash scripts/config-read-path.sh plans` (no second arg, no config)
      outputs `meta/plans`.
- [ ] `bash scripts/config-read-path.sh tmp` outputs `.accelerator/tmp`.
- [ ] `bash scripts/config-read-path.sh unknown_key` prints a warning to
      stderr and produces no stdout output.
- [ ] `scripts/config-defaults.sh` line 19 no longer contains "Do not source
      this file directly"; the updated comment explains the sourcing split.

---

## Phase 3: Strip inline defaults from all consumers; add structural invariant test

### Overview

Remove the hardcoded second argument from every bash consumer script and
every SKILL.md bang-command line. Add a structural grep test to
`test-config.sh` that asserts no consumer passes a literal inline default.

### Changes Required

#### 1. Add structural invariant test

**File**: `scripts/test-config.sh`
**Changes**: Append a final test to the
`=== config-read-path.sh (no-default lookup) ===` block (or as a new block
if preferred) that greps for literal second arguments to `config-read-path.sh`
across the codebase. This test fails while inline defaults still exist and
passes once they are all stripped.

```bash
echo "Test: no consumer passes a hardcoded inline default to config-read-path.sh"
# "? matches both bare invocations (SKILL.md backtick style: config-read-path.sh key default)
# and quoted-path bash style ("$VAR/config-read-path.sh" key default).
# Note: backtick-enclosed defaults (e.g. `meta/plans`) are not matched — not used in practice.
INLINE_DEFAULT_PATTERN='config-read-path\.sh"?[[:space:]]+[a-zA-Z_][a-zA-Z0-9_]*[[:space:]]+[^$"\n[:space:]]'
SKILL_MATCHES=$(cd "$PLUGIN_ROOT" && grep -rn --include='SKILL.md' \
  --exclude-dir=workspaces \
  -E "$INLINE_DEFAULT_PATTERN" . | sort -u)
BASH_MATCHES=$(cd "$PLUGIN_ROOT" && grep -rn --include='*.sh' \
  --exclude-dir=workspaces \
  --exclude='test-config.sh' \
  -E "$INLINE_DEFAULT_PATTERN" . | grep -v '/migrations/' | sort -u)
# jira-common.sh uses a multiline invocation — check the default token separately
# (line continuation means key and default don't appear on the same line).
JIRA_FILE="$PLUGIN_ROOT/skills/integrations/jira/scripts/jira-common.sh"
if [ ! -f "$JIRA_FILE" ]; then
  echo "  FAIL: $JIRA_FILE not found — cannot verify multiline call site"
  FAIL=$((FAIL + 1))
fi
JIRA_MATCHES=$(grep -n '\.accelerator/state/integrations' "$JIRA_FILE" 2>/dev/null | \
  grep -v '#' | sort -u || true)
ALL_MATCHES="${SKILL_MATCHES}${BASH_MATCHES}${JIRA_MATCHES}"
if [ -z "$ALL_MATCHES" ]; then
  echo "  PASS: no inline defaults found"
  PASS=$((PASS + 1))
else
  echo "  FAIL: inline defaults remain at:"
  echo "$ALL_MATCHES" | sed 's/^/    /'
  FAIL=$((FAIL + 1))
fi
```

Run the suite — the structural test fails (many matches).

#### 2. Update bash consumer scripts (13 files)

Strip the second argument from each call site. `write-visualiser-config.sh`
also requires updating its wrappers.

| File | Line(s) | Before | After |
|------|---------|--------|-------|
| `scripts/config-eject-template.sh` | 69 | `config-read-path.sh templates .accelerator/templates` | `config-read-path.sh templates` |
| `scripts/config-common.sh` | 212 | `config-read-path.sh templates .accelerator/templates` | `config-read-path.sh templates` |
| `scripts/config-summary.sh` | 20 | `config-read-path.sh tmp .accelerator/tmp` | `config-read-path.sh tmp` |
| `skills/visualisation/visualise/scripts/launch-server.sh` | 16 | `config-read-path.sh" tmp .accelerator/tmp` | `config-read-path.sh" tmp` |
| `skills/visualisation/visualise/scripts/status-server.sh` | 13 | `config-read-path.sh" tmp .accelerator/tmp` | `config-read-path.sh" tmp` |
| `skills/visualisation/visualise/scripts/stop-server.sh` | 13 | `config-read-path.sh" tmp .accelerator/tmp` | `config-read-path.sh" tmp` |
| `skills/decisions/scripts/adr-next-number.sh` | 34 | `config-read-path.sh" decisions meta/decisions` | `config-read-path.sh" decisions` |
| `skills/work/scripts/work-item-resolve-id.sh` | 38 | `config-read-path.sh" work meta/work` | `config-read-path.sh" work` |
| `skills/work/scripts/work-item-next-number.sh` | 51 | `config-read-path.sh" work meta/work` | `config-read-path.sh" work` |
| `skills/integrations/jira/scripts/jira-common.sh` | 73-74 | `config-read-path.sh" \ integrations .accelerator/state/integrations` | `config-read-path.sh" \ integrations` |
| `skills/design/inventory-design/scripts/playwright/run.sh` | 21 | `TMP_REL="$("$PLUGIN_ROOT/scripts/config-read-path.sh" tmp .accelerator/tmp 2>/dev/null \|\| echo '.accelerator/tmp')"` | `TMP_REL="$("$PLUGIN_ROOT/scripts/config-read-path.sh" tmp)"` (drop the explicit default and the now-unreachable `\|\| echo` fallback) |
| `skills/config/init/scripts/init.sh` | 52 | `bash "$CONFIG_READ_PATH" tmp .accelerator/tmp` | `bash "$CONFIG_READ_PATH" tmp` |

**`skills/visualisation/visualise/scripts/write-visualiser-config.sh`** (special case):

The `resolve_path` and `abs_path` wrappers both forward `"$2"`. Drop `"$2"` from
both wrappers, then drop the second argument from all 12 `abs_path` call sites:

```bash
# Before
resolve_path() { "$PLUGIN_ROOT/scripts/config-read-path.sh" "$1" "$2"; }
abs_path() {
  echo "$PROJECT_ROOT/$(resolve_path "$1" "$2")"
}
DECISIONS="$(abs_path decisions meta/decisions)"
WORK="$(abs_path work meta/work)"
# ... (12 call sites total, lines 50-63)

# After
resolve_path() { "$PLUGIN_ROOT/scripts/config-read-path.sh" "$1"; }
abs_path() {
  echo "$PROJECT_ROOT/$(resolve_path "$1")"
}
DECISIONS="$(abs_path decisions)"
WORK="$(abs_path work)"
# ... (all 12 call sites drop the second argument)
```

#### 3. Update SKILL.md bang-command lines (61 lines across 23 files)

Strip the trailing `<default>` token from each bang-command invocation. All
calls follow the pattern:

```
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh <key> <default>`
```

Replace with:

```
!`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh <key>`
```

Files to update (grouped by skill area):

**config/init** (`skills/config/init/SKILL.md`, 12 lines: lines 20-31):
`plans`, `research`, `decisions`, `prs`, `validations`, `review_plans`,
`review_prs`, `review_work`, `work`, `notes`, `design_inventories`, `design_gaps`

**planning** (5 files):
- `create-plan/SKILL.md` lines 22-23, and line 211 (live bang in filename template)
- `implement-plan/SKILL.md` line 23
- `review-plan/SKILL.md` lines 25-26
- `stress-test-plan/SKILL.md` line 23
- `validate-plan/SKILL.md` lines 23-24

**decisions** (3 files):
- `create-adr/SKILL.md` line 24
- `extract-adrs/SKILL.md` lines 24-26
- `review-adr/SKILL.md` line 25

**github** (3 files):
- `describe-pr/SKILL.md` lines 16-17
- `respond-to-pr/SKILL.md` line 16
- `review-pr/SKILL.md` lines 25-26

**work** (6 files):
- `create-work-item/SKILL.md` line 22
- `extract-work-items/SKILL.md` lines 25-26
- `list-work-items/SKILL.md` line 23
- `refine-work-item/SKILL.md` line 24
- `review-work-item/SKILL.md` lines 25-26
- `stress-test-work-item/SKILL.md` line 24
- `update-work-item/SKILL.md` line 24

**research** (1 file):
- `research-codebase/SKILL.md` lines 23-25

**design** (2 files):
- `analyse-design-gaps/SKILL.md` lines 27-28
- `inventory-design/SKILL.md` line 32

**visualisation** (1 file):
- `visualise/SKILL.md` lines 14-27 (14 keys total)

Run the suite — structural test passes; all `=== config-read-path.sh ===` tests
continue to pass.

### Success Criteria

#### Automated Verification

- [ ] Structural grep test passes (0 inline-default matches in SKILL.md and
      bash consumer files, excluding `test-config.sh` and migrations).
- [ ] `mise run test:integration:config` passes with 0 failures.
- [ ] `mise run test:integration` passes (full suite).

#### Manual Verification

- [ ] `bash scripts/config-read-path.sh work` (no config, no second arg)
      outputs `meta/work`.
- [ ] `bash scripts/config-eject-template.sh` does not fail or produce an empty
      templates path (smoke test the template ejection flow).
- [ ] Launch the visualiser in a project with no `.accelerator/config.md` —
      verify all paths resolve to their plugin defaults.
- [ ] `grep -rn 'config-read-path\.sh' skills/ --include='SKILL.md' | grep -v 'test-' | head -5`
      shows calls with only one argument after the script name.

---

## Testing Strategy

### Structural Tests (added in this plan)

- **Phase 1**: Updated array-length assertions (11 → 15) and content assertions
  for `PATH_KEYS`/`PATH_DEFAULTS` in `=== config-defaults.sh ===` block.
- **Phase 2**: New `=== config-read-path.sh (no-default lookup) ===` block with
  six assertions covering representative keys and the explicit-override path.
- **Phase 3**: Structural grep asserting 0 inline-default matches across
  SKILL.md and bash consumer files.

### Regression Tests (must continue to pass throughout)

- `=== config-defaults.sh ===` — array definition site invariant.
- `=== config-read-path.sh ===` — explicit `$2` override path (unchanged).
- `=== config-dump.sh ===` — config-dump rendering end-to-end.
- Full `mise run test:integration` suite.

### Manual Testing Steps

1. From the repo root, call `bash scripts/config-read-path.sh plans` — expect
   `meta/plans`.
2. Call `bash scripts/config-read-path.sh plans custom/plans` — expect
   `custom/plans` (explicit override still works).
3. Call `bash scripts/config-read-path.sh unknown_key` — expect empty output
   (no match in PATH_KEYS, no default found, config-read-value returns nothing).
4. In a project with `paths.work: docs/work-items` in `.accelerator/config.md`,
   call `bash scripts/config-read-path.sh work` — expect `docs/work-items`.

## Performance Considerations

`config-read-path.sh` is called once per path resolution. Adding a loop over a
15-element `PATH_KEYS` array adds negligible overhead (single-digit milliseconds
at most). The loop exits on first match, so typical invocations visit 1-14
entries.

## Migration Notes

No data migration. The `[default]` argument to `config-read-path.sh` remains
accepted for backward compatibility — existing callers not updated in this plan
(e.g. `test-config.sh` regression tests, migration scripts) continue to work
unchanged.

## References

- Originating work item: `meta/work/0030-centralise-path-defaults.md`
- Implementation research:
  `meta/research/2026-05-08-0030-centralise-path-defaults-implementation.md`
- Dependent work item: `meta/work/0052-make-documents-locator-paths-config-driven.md`
  (will source `config-defaults.sh` once this plan lands)
- Definition site: `scripts/config-defaults.sh`
- Consumer entry point: `scripts/config-read-path.sh`
- Test suite: `scripts/test-config.sh`
