---
date: "2026-05-08T22:30:00+01:00"
type: plan
skill: create-plan
work-item: "0046"
status: done
---

# 0046 Work Management System Configuration — Implementation Plan

## Overview

Add a new `work.integration` config key (allowed values
`jira | linear | trello | github-issues`), centralise the `work.*` config
family in `scripts/config-defaults.sh` mirroring the existing `paths.*` and
`templates.*` registries, route every consumer through a new
`scripts/config-read-work.sh` thin wrapper, and surface all three `work.*`
keys in `/accelerator:configure view` (with non-fatal annotation of
invalid integration values). Validation hard-fails on unrecognised values;
the integration-agnostic helper `work_resolve_default_project` in a new
`scripts/work-common.sh` warns when `work.default_project_code` is empty
while `work.integration` is set, so future Linear / Trello / GitHub-Issues
consumers share one implementation. Implemented test-first across six phases.

## Current State Analysis

The plugin's configuration system splits responsibilities cleanly: a generic
key reader (`scripts/config-read-value.sh`), a centralised registry of
keys+defaults (`scripts/config-defaults.sh`, sourced via
`scripts/config-common.sh`), and per-family thin wrappers
(`scripts/config-read-path.sh`, `scripts/config-read-review.sh`).

The `work.*` family does not yet follow this shape:

- `config-defaults.sh` holds `PATH_KEYS`, `PATH_DEFAULTS`, `TEMPLATE_KEYS`
  but **no** `WORK_KEYS` / `WORK_DEFAULTS`.
- There is **no** `config-read-work.sh` wrapper. Every consumer calls
  `config-read-value.sh` directly with an inline default literal:
  - `skills/work/scripts/work-item-next-number.sh:58-59`
  - `skills/work/scripts/work-item-resolve-id.sh:45-46`
  - `skills/visualisation/visualise/scripts/write-visualiser-config.sh:95-96`
  - `skills/integrations/jira/scripts/jira-create-flow.sh:179`
  - `skills/integrations/jira/scripts/jira-search-flow.sh:207`
  - `skills/integrations/jira/scripts/jira-init-flow.sh:170`
  - `skills/integrations/jira/create-jira-issue/SKILL.md:54`
  - `skills/work/extract-work-items/SKILL.md:349-350`
  - `skills/work/list-work-items/SKILL.md:24-25`
- `config-dump.sh` iterates `REVIEW_KEYS`, `AGENT_KEYS`, `PATH_KEYS`,
  `TEMPLATE_KEYS` (lines 159-192) — no `WORK_KEYS` loop, so neither
  `work.id_pattern` nor `work.default_project_code` appears in
  `/accelerator:configure view` output today.

The acceptance criteria for 0046 are partially satisfied by the codebase's
current shape:

- **AC1** ("no external API calls when `work.integration` unset"):
  trivially met — none of the seven local work skills (`create`, `update`,
  `list`, `extract`, `refine`, `review`, `stress-test`) make any HTTP /
  Jira-helper calls.
- **AC2** ("auto-scope to `default_project_code`"): already met
  unconditionally by `jira-search-flow.sh:207` and `jira-create-flow.sh:179`.
  Both fall back whenever `--project` is omitted, regardless of whether
  `work.integration` is set. The conservative reading of AC2 leaves these
  fallbacks unconditional; gating them on `work.integration: jira` is out
  of scope.
- **AC3** ("local-first writes"): met — every work skill writes to
  `meta/work/`; no skill bypasses local storage.
- **AC4** ("informative error on unrecognised value"): **needs new code**.
- **AC5** ("warn when `default_project_code` empty + `work.integration`
  set, on integration-skill invocation"): **needs new code**.

### Key Discoveries

- Centralisation pattern (post-0030): `config-defaults.sh` defines the
  registry arrays; `config-read-path.sh:17-42` sources `config-defaults.sh`
  directly, looks up the default by index, then delegates to
  `config-read-value.sh`. The new `config-read-work.sh` will mirror this
  shape exactly.
- Validation idioms in the plugin: `config-read-review.sh:129-138`
  (`validate_severity` — warn + default fallback) is too lenient for AC4
  ("informative error" implies hard fail). The closer model is
  `wip_validate_pattern` in
  `skills/work/scripts/work-item-common.sh:17-23`, which writes to stderr
  and exits non-zero. `scripts/log-common.sh` provides `log_die` /
  `log_warn` helpers; new code should use these.
- Single-definition-site invariant: `scripts/test-config.sh:2476-2483` runs
  a regex-grep that asserts `PATH_KEYS|PATH_DEFAULTS|TEMPLATE_KEYS` are
  defined exactly once (in `config-defaults.sh`). Adding `WORK_KEYS` /
  `WORK_DEFAULTS` requires extending that regex.
- No-inline-default invariant: `scripts/test-config.sh:2883` asserts no
  consumer passes a hardcoded inline default to `config-read-path.sh`.
  Phase 4 mirrors this for `config-read-work.sh`.
- Empty-vs-missing semantics: `config-read-value.sh` treats empty and
  missing identically. Cross-key checks (AC5) must `[ -z ]`-test.
- Multiline call pattern in Jira flows: the read sites at
  `jira-create-flow.sh:179`, `jira-search-flow.sh:207`, and
  `jira-init-flow.sh:170` use line-continuation calls of the form
  `default_project=$("$PLUGIN_ROOT/scripts/config-read-value.sh" \` (line
  break) `"work.default_project_code" "" 2>/dev/null) || default_project=""`.
  Migration must preserve the surrounding `2>/dev/null` and `||` clauses.
- The `migrations/` directory referenced in the research does not exist on
  this branch. The plan does not touch it.

## Desired End State

After the plan completes, all of the following are true and verifiable:

1. `scripts/config-defaults.sh` exposes
   `WORK_KEYS=("work.integration" "work.id_pattern" "work.default_project_code")`
   (fully-dotted, matching the existing `PATH_KEYS` / `TEMPLATE_KEYS` storage
   form), parallel `WORK_DEFAULTS=("" "{number:04d}" "")`, and
   `WORK_INTEGRATION_VALUES=("jira" "linear" "trello" "github-issues")`
   as the single source of truth for the integration enum (consumed by
   both the wrapper's hard-fail validation and the dump's non-fatal
   annotation; empty value is implicit-valid in both consumers).
2. `scripts/config-read-work.sh <key>` is a thin wrapper that mirrors
   `config-read-path.sh`:
   - Looks up the key in `WORK_KEYS`; returns the corresponding
     `WORK_DEFAULTS` entry when not configured.
   - Warns to stderr when called with an unknown `work.*` key, then still
     delegates to `config-read-value.sh` with empty default — matching
     `config-read-path.sh:37-42` behaviour exactly so user-set values for
     keys outside the registry are still readable.
   - When called with `integration`, validates the value against
     `"" | jira | linear | trello | github-issues`. Any other value
     causes an `Error: …` message to stderr (naming all four valid values
     plus a pointer to `.accelerator/config.md` and
     `/accelerator:configure view`) and exit code 1 via `log_die`.
3. `scripts/work-common.sh` (new) exposes `work_resolve_default_project()`,
   integration-agnostic so future Linear / Trello / GitHub-Issues consumers
   can share it. It reads both `work.integration` and
   `work.default_project_code` via `config-read-work.sh`; when integration
   is set and the project code is empty it emits a warning via `log_warn`
   naming whichever integration is configured; it echoes the project code
   (which may be empty) for the caller. Both reads deliberately omit the
   `2>/dev/null) || …=""` defensive idiom: the integration read needs
   AC4 enum-validation hard-fails to propagate, and the
   `default_project_code` read has no validation but its empty value
   already comes from the centralised `WORK_DEFAULTS` — so any non-zero
   exit there is a real error worth surfacing rather than masking. The
   two duplicate fallback blocks at `jira-search-flow.sh:207` and
   `jira-create-flow.sh:179` are replaced by a single call to this helper.
4. Every remaining inline `work.*` read site uses `config-read-work.sh`:
   - Bash scripts: `work-item-next-number.sh`, `work-item-resolve-id.sh`,
     `write-visualiser-config.sh`, `jira-init-flow.sh`.
   - SKILL.md bash snippets: `create-jira-issue/SKILL.md`,
     `extract-work-items/SKILL.md`, `list-work-items/SKILL.md`.
   No file outside `scripts/test-config.sh` and the
   `scripts/config-*.sh` family contains a literal
   `config-read-value.sh work.<...>` invocation.
5. `scripts/config-dump.sh` iterates `WORK_KEYS` after the
   `TEMPLATE_KEYS` block, emitting each `work.*` key in
   `/accelerator:configure view` output with team / local / default source
   attribution.
6. `skills/config/configure/SKILL.md` documents `work.integration` (table
   row + recognised-keys allow-list), updates the section lead-in from
   "Two keys are recognised" to acknowledge the third key, and contains a
   one-paragraph "local-first storage" note that explains AC3 (naming the
   skill directories the invariant applies to).
7. `README.md` mentions `work.integration` and the local-first invariant
   alongside its existing `work.id_pattern` / `work.default_project_code`
   references at lines 122, 280, 342.
8. `skills/integrations/jira/init-jira/SKILL.md`,
   `skills/integrations/jira/create-jira-issue/SKILL.md`, and
   `skills/integrations/jira/search-jira-issues/SKILL.md` each contain a
   short Configuration / Prerequisites note linking to the `### work`
   section of `configure/SKILL.md`, so the new key is discoverable from
   integration-skill docs (not only the canonical config doc).
9. `scripts/test-config.sh` exercises every new behaviour: enum
   validation (hard-fail and scoped), cross-key warning (positive and
   negative cases), consumer migration (no-inline-default invariant
   covering line-continuation calls + no stale `config-read-value.sh
   work.*` references), dump output for all three `work.*` keys
   (including ordering and mixed-source attribution), and a structural
   regression guard asserting the seven local work skills source no
   `skills/integrations/` paths and no `*-api.sh` / `*-auth.sh` files.

## What We're NOT Doing

- **Not** gating the existing unconditional fallbacks in
  `jira-search-flow.sh` / `jira-create-flow.sh` on `work.integration: jira`.
  Acceptance criteria are satisfied under both readings; conservative
  interpretation leaves existing behaviour intact.
- **Not** scaffolding any non-Jira integration. `linear`, `trello`, and
  `github-issues` become valid enum values but no skills consume them yet —
  those are stories 0048+.
- **Not** adding sync-status branching to `/list-work-items` or
  `/create-work-item`. Story 0047 owns that.
- **Not** introducing stable error codes (`E_WORK_INTEGRATION_INVALID`).
  Plain `Error: …` text via `log_die` is sufficient; no external consumer
  parses these codes.
- **Not** changing the empty-vs-missing semantics of
  `config-read-value.sh`. All cross-key logic stays at the wrapper /
  consumer level.
- **Not** touching `skills/integrations/jira/scripts/jira-init-flow.sh`'s
  prompt logic — only its read-side call site is migrated. The init flow
  must NOT call `work_resolve_default_project`, because its job is to
  *set* the default project code; the warning would always fire pre-init.
  It reads `default_project_code` directly via `config-read-work.sh`.

## Implementation Approach

Strict TDD throughout. Each phase writes failing tests against the
intended behaviour first, then implements the minimum code to pass.
`bash scripts/test-config.sh` is green at the end of every phase.

Phase ordering follows the natural dependency graph:

- Phase 1 (registry + reader) is the foundation everything else depends on.
- Phase 2 (validation) needs the reader.
- Phase 3 (cross-key warning helper) needs validation.
- Phase 4 (consumer migration) lands once helpers and validation exist.
- Phase 5 (dump wiring) only needs the registry — could land in parallel
  with 4, but sequenced after for review-size reasons.
- Phase 6 (docs + regression guard) wraps up.

Each phase produces a coherent, independently-reviewable change.

---

## Phase 1: Reader skeleton + `WORK_KEYS` centralisation

### Overview

Establish `config-read-work.sh` as a thin wrapper backed by a centralised
`WORK_KEYS` / `WORK_DEFAULTS` registry. Read paths only — no validation
yet. Existing inline read sites continue to work unchanged.

### Changes Required

#### 1. Centralise work.* defaults

**File**: `scripts/config-defaults.sh`
**Changes**: Append `WORK_KEYS` and `WORK_DEFAULTS` arrays after the
existing `TEMPLATE_KEYS` block. Update the file's header scope-note
comment (currently at lines 11-17) to acknowledge that `WORK_KEYS` is now
centralised here too.

```bash
# After TEMPLATE_KEYS (~line 70):

WORK_KEYS=(
  "work.integration"
  "work.id_pattern"
  "work.default_project_code"
)

WORK_DEFAULTS=(
  ""
  "{number:04d}"
  ""
)

# Allowed non-empty values for work.integration. Empty value is
# additionally permitted by both consumers (unset is the default state).
# Consumers: config-read-work.sh (Phase 2 hard-fail validation) and
# config-dump.sh (Phase 5 non-fatal annotation).
WORK_INTEGRATION_VALUES=(
  "jira"
  "linear"
  "trello"
  "github-issues"
)
```

Note: `WORK_KEYS` entries are fully dotted, matching `PATH_KEYS` and
`TEMPLATE_KEYS` in the same file. `config-read-work.sh` looks up the key
by comparing against `"work.${key}"`, mirroring the path wrapper's
`"paths.${key}"` lookup at `config-read-path.sh:32`. The Phase 5 dump
loop iterates `WORK_KEYS` directly without re-prefixing.

`WORK_INTEGRATION_VALUES` lists only the non-empty allowed values; both
consumers treat empty separately as the unset/default state. Adding a
fifth integration is therefore a single-line registry edit. Update the
single-definition-site invariant regex to cover this array.

#### 2. Create config-read-work.sh wrapper

**File**: `scripts/config-read-work.sh` (new)
**Changes**: Mirror `config-read-path.sh:1-42` shape. Source
`config-defaults.sh` directly (matching `config-read-path.sh`'s
performance-driven choice to avoid `config-common.sh`).

```bash
#!/usr/bin/env bash
set -euo pipefail

# Reads a work-management configuration value with centralised defaults.
# Usage: config-read-work.sh <work_key>
#
# Recognised keys:
#   integration            → active remote tracker
#                            (allowed: jira, linear, trello, github-issues; default empty)
#   id_pattern             → DSL controlling work-item ID shape
#                            (default {number:04d})
#   default_project_code   → project code substituted into {project}
#                            (default empty)
#
# When called for an unknown work.* key, prints a warning to stderr and
# exits 0 with empty stdout (mirrors config-read-path.sh).

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=config-defaults.sh
source "$SCRIPT_DIR/config-defaults.sh"

key="${1:-}"
if [ -z "$key" ]; then
  echo "Usage: config-read-work.sh <work_key>" >&2
  exit 1
fi

default=""
found=false
for i in "${!WORK_KEYS[@]}"; do
  if [ "${WORK_KEYS[$i]}" = "work.${key}" ]; then
    default="${WORK_DEFAULTS[$i]}"
    found=true
    break
  fi
done

if [ "$found" = false ]; then
  echo "config-read-work.sh: warning: unknown key 'work.${key}' — no centralized default" >&2
fi

exec "$SCRIPT_DIR/config-read-value.sh" "work.${key}" "${default}"
```

This delegates with empty default on unknown keys, matching
`config-read-path.sh:37-42` exactly so user-set values for keys outside
the registry are still readable. Mark the file executable.

#### 3. Tests

**File**: `scripts/test-config.sh`
**Changes**: Add a new `=== config-read-work.sh ===` section near the
existing `=== config-read-path.sh ===` section. Test cases (write red
first; then implement above to green):

- `Test: No argument -> exits with error` (mirrors path test at line 2673)
- `Test: integration key -> empty when unset`
- `Test: id_pattern key -> {number:04d} when unset`
- `Test: default_project_code key -> empty when unset`
- `Test: integration -> reads team config value (jira)`
- `Test: id_pattern -> reads team config value`
- `Test: default_project_code -> reads team config value`
- `Test: local override of work.integration wins over team`
- `Test: local override of work.id_pattern wins over team`
- `Test: local override of work.default_project_code wins over team`
- `Test: work.integration explicitly set to empty string -> empty value, no error, no warning`
- `Test: work.default_project_code set to empty string in team config -> empty value (matches missing semantics)`
- `Test: unknown work.* key -> warning to stderr, then delegates to config-read-value.sh with empty default`
- `Test: unknown work.* key with value set in config -> warning + value returned (matches config-read-path.sh contract)`
- `Test: WORK_KEYS[i] / WORK_DEFAULTS[i] index alignment — for each declared key, the wrapper returns the documented default when unset`

Extend the single-definition-site invariant test at lines 2476-2483:

```bash
DEFINITION_PATTERN='^[[:space:]]*((declare|typeset)[[:space:]]+(-[a-zA-Z]+[[:space:]]+)?|readonly[[:space:]]+|export[[:space:]]+|local[[:space:]]+)?(PATH_KEYS|PATH_DEFAULTS|TEMPLATE_KEYS|WORK_KEYS|WORK_DEFAULTS|WORK_INTEGRATION_VALUES)(\+)?='
```

Add structural assertions about the new arrays (mirroring
`test-config.sh:2441-2455` for `PATH_KEYS`):

- `Test: WORK_KEYS has expected length and order`
- `Test: WORK_DEFAULTS has expected length and matches WORK_KEYS`
- `Test: WORK_INTEGRATION_VALUES has expected length and contains exactly jira, linear, trello, github-issues`
- `Test: no scripts/skills file outside config-defaults.sh contains a literal 'jira | linear | trello | github-issues' alternation` *(single-source-of-truth invariant — catches future regressions where someone re-hardcodes the enum in a third location)*

### Success Criteria

#### Automated Verification:

- [x] `bash scripts/test-config.sh` passes all new and existing tests
- [x] `bash scripts/config-read-work.sh integration` outputs empty
- [x] `bash scripts/config-read-work.sh id_pattern` outputs `{number:04d}`
- [x] `bash scripts/config-read-work.sh default_project_code` outputs empty
- [x] `bash scripts/config-read-work.sh nonexistent` writes a warning to
      stderr and delegates with empty default (exit code reflects
      `config-read-value.sh`'s normal behaviour, not a hard exit)
- [x] Single-definition-site test rejects a duplicate definition of any
      of the six registry array names (`PATH_KEYS`, `PATH_DEFAULTS`,
      `TEMPLATE_KEYS`, `WORK_KEYS`, `WORK_DEFAULTS`,
      `WORK_INTEGRATION_VALUES`)

#### Manual Verification:

- [ ] In a repo with `work.id_pattern: "{project}-{number:04d}"` and
      `work.default_project_code: "PROJ"` configured,
      `config-read-work.sh id_pattern` returns `{project}-{number:04d}`
      and `config-read-work.sh default_project_code` returns `PROJ`

---

## Phase 2: Enum validation for `work.integration`

### Overview

Hard-fail validation when `config-read-work.sh integration` is called and
the configured value is outside the allowed enum. Validation is **scoped
to the `integration` key** — reading other `work.*` keys does not trigger
validation. This is deliberate: a typo in `work.integration` should not
break local-only skills (`/list-work-items`, `/create-work-item`) that
have nothing to do with integrations.

### Changes Required

#### 1. Add validation to config-read-work.sh

**File**: `scripts/config-read-work.sh`
**Changes**: Replace the trailing `exec` with a value-capture + validation
+ echo block, sourcing `log-common.sh` for `log_die`. The shift from
`exec` to capture+echo is a deliberate semantic change (validation needs
the value before the script returns), so add a Phase 2 test that asserts
the script's exit code propagates correctly when `config-read-value.sh`
itself fails (e.g. when no config file exists). Validation iterates
`WORK_INTEGRATION_VALUES` (sourced in Phase 1 from `config-defaults.sh`)
rather than embedding an inline enum literal — single source of truth
for the allowed values.

```bash
# Replace the final `exec ... config-read-value.sh ...` line with:

# shellcheck source=log-common.sh
source "$SCRIPT_DIR/log-common.sh"

value=$("$SCRIPT_DIR/config-read-value.sh" "work.${key}" "${default}")

if [ "$key" = "integration" ] && [ -n "$value" ]; then
  valid=false
  for allowed in "${WORK_INTEGRATION_VALUES[@]}"; do
    if [ "$value" = "$allowed" ]; then valid=true; break; fi
  done
  if [ "$valid" = false ]; then
    allowed_list="${WORK_INTEGRATION_VALUES[*]}"
    log_die "Error: work.integration must be one of: ${allowed_list// /, } (got '${value}'). Update work.integration in .accelerator/config.md or run '/accelerator:configure view' to inspect the current value."
  fi
fi

echo "$value"
```

The validation intentionally treats empty as valid (`[ -n "$value" ]`
short-circuits — an unset integration is the supported default state).
The error message renders the allowed-values list at runtime from
`WORK_INTEGRATION_VALUES` so adding a fifth integration in
`config-defaults.sh` automatically updates the user-facing error text
without further edits. The message names both the file users edit and
the diagnostic command — pointing the user to the fix path rather than
only listing valid values.

#### 2. Tests

**File**: `scripts/test-config.sh`
**Changes**: Append to the `=== config-read-work.sh ===` section.

- `Test: work.integration: jira -> reads jira`
- `Test: work.integration: linear -> reads linear`
- `Test: work.integration: trello -> reads trello`
- `Test: work.integration: github-issues -> reads github-issues`
- `Test: work.integration: garbage -> exits non-zero`
- `Test: work.integration: garbage -> stderr contains 'jira' AND 'linear' AND 'trello' AND 'github-issues' (each asserted independently so the test survives format refactors)`
- `Test: work.integration: garbage -> stderr contains the input garbage value (so users can see what they typed)`
- `Test: work.integration: garbage -> stderr names .accelerator/config.md and /accelerator:configure view as remediation pointers`
- `Test: work.integration unset -> empty value, no error`
- `Test: work.integration: garbage but reading id_pattern -> id_pattern read succeeds`
  *(critical scoping regression guard — without this, a future refactor
  could over-broaden validation and break local skills)*
- `Test: work.integration: garbage but reading default_project_code -> read succeeds`
- `Test: capture+echo form propagates exit code when config-read-value.sh itself fails (no config files present in a temp dir)`

### Success Criteria

#### Automated Verification:

- [ ] `bash scripts/test-config.sh` passes
- [ ] In a repo with `work.integration: foo`,
      `bash scripts/config-read-work.sh integration` exits 1 with stderr
      containing `jira, linear, trello, github-issues`
- [ ] In the same repo, `bash scripts/config-read-work.sh id_pattern`
      succeeds (validation does not bleed across keys)

#### Manual Verification:

- [ ] In a repo with `work.integration: jira` configured,
      `config-read-work.sh integration` outputs `jira` and exits 0
- [ ] In a repo with `work.integration: jura` (typo) configured,
      `config-read-work.sh integration` prints a clear, actionable error
      and exits 1

---

## Phase 3: `work_resolve_default_project` helper with cross-key warning

### Overview

Factor the duplicate fallback at `jira-search-flow.sh:207` and
`jira-create-flow.sh:179` into a single integration-agnostic helper in a
new `scripts/work-common.sh`, and use that helper to emit the AC5
warning. The helper lives in `scripts/` (not under
`skills/integrations/jira/`) so future Linear / Trello / GitHub-Issues
consumers (stories 0048+) share one implementation rather than each
re-inventing it.

Restricting the warning to integration consumers (rather than firing it
inside `config-read-work.sh` on every read) keeps the warning targeted:
it appears when an integration skill needs the project code, not on
every `/list-work-items` invocation. The integration read inside the
helper deliberately omits the `2>/dev/null) || …=""` defensive idiom so
that Phase 2's enum validation can surface as an AC4 hard-fail at the
caller — preserving the defensive style there would silently swallow
the most common real-world failure mode (a typo'd `work.integration`).

### Changes Required

#### 1. Create scripts/work-common.sh with work_resolve_default_project

**File**: `scripts/work-common.sh` (new)
**Changes**: New shared helper file for work-management consumers.

```bash
#!/usr/bin/env bash
# Shared helpers for work-management consumers (integration skills).
# Sourced by jira-common.sh (and by future linear-common.sh / etc.).

WORK_COMMON_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=log-common.sh
source "$WORK_COMMON_SCRIPT_DIR/log-common.sh"

# Resolve the default project code, warning when work.integration is set
# but work.default_project_code is empty. Echoes the project code
# (possibly empty) for the caller to use as its default. The warning text
# names whichever integration is configured, so this helper is shared by
# all integration consumers.
#
# Both reads intentionally omit `2>/dev/null` and `||` fallback. For the
# integration read this allows Phase 2's enum validation hard-fail
# (log_die) to propagate via set -euo pipefail. For the default_project_code
# read, WORK_DEFAULTS already supplies "" as the centralised default —
# an empty value is the *successful* outcome of an unset key, so the only
# way the read can exit non-zero is a true script error (malformed config,
# I/O error, etc.), which should reach the caller rather than be masked
# into a silent empty value.
work_resolve_default_project() {
  local read_work integration project
  read_work="$WORK_COMMON_SCRIPT_DIR/config-read-work.sh"
  integration=$("$read_work" integration)
  project=$("$read_work" default_project_code)
  if [ -n "$integration" ] && [ -z "$project" ]; then
    log_warn "work.default_project_code is empty but work.integration is set ($integration) — pass --project explicitly or set default_project_code in .accelerator/config.md"
  fi
  echo "$project"
}
```

Mark the file readable (no execute bit — it's sourced, not run directly).

#### 2. Source scripts/work-common.sh from jira-common.sh

**File**: `skills/integrations/jira/scripts/jira-common.sh`
**Changes**: Add a single `source` line near the existing helper sources
(after `vcs-common.sh`/`log-common.sh` etc.), using the existing
`_JIRA_PLUGIN_ROOT` variable already defined at jira-common.sh:42:

```bash
# shellcheck source=../../../../scripts/work-common.sh
source "$_JIRA_PLUGIN_ROOT/scripts/work-common.sh"
```

This makes `work_resolve_default_project` available to every script that
already sources `jira-common.sh` — including both flows we are migrating
— without introducing a parallel path-resolution helper. (No new
`jira_plugin_scripts_dir` is needed.)

#### 3. Refactor jira-search-flow.sh and jira-create-flow.sh

**File**: `skills/integrations/jira/scripts/jira-search-flow.sh:207`
**Changes**: Replace the multiline fallback block with a single call:

```bash
# Before (sketch):
if [ -z "$project" ]; then
  default_project=$("$PLUGIN_ROOT/scripts/config-read-value.sh" \
    "work.default_project_code" "" 2>/dev/null) || default_project=""
  project="$default_project"
fi

# After:
if [ -z "$project" ]; then
  project=$(work_resolve_default_project)
fi
```

**File**: `skills/integrations/jira/scripts/jira-create-flow.sh:179`
**Changes**: Same pattern, preserving the surrounding
`E_CREATE_NO_PROJECT` exit logic (the helper echoes empty when no project
is configured; the existing `[ -z "$project" ]` check downstream catches
that).

Both flows already source `jira-common.sh`, which (after Section 2) in
turn sources `scripts/work-common.sh`, so the new helper is available
without further wiring.

#### 4. Update SKILL.md `allowed-tools` if needed

**Files**:
- `skills/integrations/jira/search-jira-issues/SKILL.md`
- `skills/integrations/jira/create-jira-issue/SKILL.md`

**Changes**: This is now subsumed by the structural assertion added in
Phase 4 Section 4 ("every SKILL.md whose script invokes
`config-read-work.sh` has an `allowed-tools` entry that permits it"). No
per-file audit required at this phase.

#### 5. Tests

**File**: `scripts/test-config.sh`
**Changes**: Add a new `=== work_resolve_default_project ===` section.
Each test sources `scripts/work-common.sh` in a controlled subshell with
the working directory set to a temp config root. Use this fixture
pattern (mirrors the existing source-in-subshell pattern in
`test-jira-common.sh`):

```bash
( set -e; cd "$TEST_REPO"
  stderr=$( { stdout=$(source "$PLUGIN_ROOT/scripts/work-common.sh" \
    && work_resolve_default_project); } 2>&1 1>&3 ) 3>&1
  # assert on $stdout (project code) and $stderr (warning) independently
)
```

Behaviour tests:

- `Test: integration unset, project unset -> no warning, returns empty`
- `Test: integration unset, project = PROJ -> no warning, returns PROJ`
- `Test: integration = jira, project = PROJ -> no warning, returns PROJ`
- `Test: integration = jira, project unset -> warning to stderr, returns empty`
- `Test: integration = jira, project unset -> warning names "jira"`
- `Test: integration = linear, project unset -> warning names "linear"`
- `Test: integration = trello, project unset -> warning names "trello"`
- `Test: integration = github-issues, project unset -> warning names "github-issues"`
- `Test: warning includes the phrase "pass --project" and references default_project_code`
- `Test: warning includes the phrase ".accelerator/config.md"`

Critical: surface AC4 through the helper:

- `Test: integration = "jura" (invalid), project unset -> exits non-zero and stderr names valid values`
  *(regression guard: defensive `2>/dev/null) || …=""` would silently
  swallow this, defeating AC4 at the consumer boundary)*
- `Test: integration = "jura" (invalid), project = PROJ -> still exits non-zero (validation fires before project check)`

Structural assertions to lock in the refactor:

- `Test: jira-search-flow.sh contains no inline 'config-read-value.sh
  work.default_project_code'`
- `Test: jira-create-flow.sh contains no inline 'config-read-value.sh
  work.default_project_code'`
- `Test: jira-search-flow.sh contains 'work_resolve_default_project'`
- `Test: jira-create-flow.sh contains 'work_resolve_default_project'`
- `Test: jira-common.sh sources scripts/work-common.sh`

### Success Criteria

#### Automated Verification:

- [ ] `bash scripts/test-config.sh` passes the new helper tests
- [ ] `grep -E 'config-read-value\.sh.*work\.default_project_code'
      skills/integrations/jira/scripts/jira-{search,create}-flow.sh`
      returns no matches
- [ ] Existing Jira flow tests still pass, specifically
      `bash skills/integrations/jira/scripts/test-jira-create.sh` Case 2
      ("no --project, no work.default_project_code exits 100"), and an
      analogous positive AC2 case is added covering integration set +
      project set → helper returns project, no warning, JQL/REST call
      uses the project. Mirror in `test-jira-search.sh`.
- [ ] In a temp repo with `work.integration: jura` (typo) and no
      `default_project_code` configured, `work_resolve_default_project`
      exits non-zero with stderr containing `jira, linear, trello,
      github-issues` (i.e. AC4 reaches the consumer through the helper)

#### Manual Verification:

- [ ] In a repo with `work.integration: jira` and no
      `work.default_project_code`, running `/accelerator:create-jira-issue
      --summary "test"` (without `--project`) prints the warning and
      proceeds to the existing `E_CREATE_NO_PROJECT` exit path
- [ ] In a repo with `work.integration: jira` and
      `work.default_project_code: PROJ`, the same command does not print
      the warning and uses `PROJ` as the project key

---

## Phase 4: Migrate inline `work.*` read sites to `config-read-work.sh`

### Overview

Replace every direct `config-read-value.sh work.<key>` call with
`config-read-work.sh <key>`. Centralisation only delivers value if every
read goes through the wrapper. Add the no-inline-default invariant
(mirroring `test-config.sh:2883` for paths) and a no-stale-references
invariant.

### Changes Required

#### 1. Migrate bash script call sites

**File**: `skills/work/scripts/work-item-next-number.sh:58-59`
```bash
# Before:
PATTERN=$("$PLUGIN_ROOT/scripts/config-read-value.sh" work.id_pattern "{number:04d}")
DEFAULT_PROJECT=$("$PLUGIN_ROOT/scripts/config-read-value.sh" work.default_project_code "")
# After:
PATTERN=$("$PLUGIN_ROOT/scripts/config-read-work.sh" id_pattern)
DEFAULT_PROJECT=$("$PLUGIN_ROOT/scripts/config-read-work.sh" default_project_code)
```

Apply the same shape change to:

- `skills/work/scripts/work-item-resolve-id.sh:45-46`
- `skills/visualisation/visualise/scripts/write-visualiser-config.sh:95-96`
  (drop the existing `2>/dev/null || echo "{number:04d}"` and
  `2>/dev/null || true` defensive tails on these two reads. They guard
  against `id_pattern` and `default_project_code` reads, neither of
  which can trigger Phase 2 validation — the tails are functionally dead
  and mask what would otherwise be a genuine script error worth
  surfacing. Validation only fires on the `integration` key.)
- `skills/integrations/jira/scripts/jira-init-flow.sh:170` (use
  `config-read-work.sh default_project_code` directly — **not** via
  `work_resolve_default_project`. The init flow's job is to set
  `default_project_code` when missing, so the AC5 warning would
  always-fire pre-init and would be noise.)

The two Jira flow scripts (`jira-search-flow.sh`,
`jira-create-flow.sh`) were already migrated in Phase 3 via
`work_resolve_default_project`.

#### 2. Migrate SKILL.md inline bash snippets

**File**: `skills/work/extract-work-items/SKILL.md:349-350`
```markdown
<!-- Before -->
PATTERN=$(${CLAUDE_PLUGIN_ROOT}/scripts/config-read-value.sh work.id_pattern "{number:04d}")
DEFAULT_PROJECT=$(${CLAUDE_PLUGIN_ROOT}/scripts/config-read-value.sh work.default_project_code "")
<!-- After -->
PATTERN=$(${CLAUDE_PLUGIN_ROOT}/scripts/config-read-work.sh id_pattern)
DEFAULT_PROJECT=$(${CLAUDE_PLUGIN_ROOT}/scripts/config-read-work.sh default_project_code)
```

Apply the same pattern to:

- `skills/work/list-work-items/SKILL.md:24-25`
- `skills/integrations/jira/create-jira-issue/SKILL.md:54`

#### 3. SKILL.md `allowed-tools` — single structural assertion

Rather than per-file audits, codify the rule once. Every SKILL.md whose
scripts (or inline snippets) invoke `config-read-work.sh` must have an
`allowed-tools` entry that permits it — either an unrestricted `Bash`,
or a `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/*)` glob, or a more specific
`Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)` glob. The structural
assertion in Section 4 (below) enforces this; no manual checklist
needed.

#### 4. Tests

**File**: `scripts/test-config.sh`
**Changes**: Append a new `=== consumer migration: config-read-work.sh ===`
section, modelled on the existing path consumer assertions at
`test-config.sh:2883+`. Each regex is preceded by a comment block giving
(a) what shape it matches, (b) what it rejects, (c) one positive and
one negative example so the assertion is auditable.

- `Test: no consumer passes a hardcoded inline default to
  config-read-work.sh`. The regex must match both same-line and
  line-continuation invocation forms — the path-equivalent test had to
  special-case `jira-common.sh`'s multiline call, and the work-side call
  sites already use the multiline shape (`jira-create-flow.sh:179`,
  `jira-init-flow.sh:170`). Implement as a tiny awk pre-pass that joins
  trailing `\` continuations before grepping:

  ```bash
  # awk pass: join lines ending in `\` so multiline invocations
  # appear on a single logical line for grep.
  joined=$(awk 'BEGIN{p=""} { if (sub(/\\$/, "")) { p=p$0; next }
                              print p$0; p="" }' "$file")
  INLINE_DEFAULT_PATTERN='config-read-work\.sh"?[[:space:]]+[a-zA-Z_][a-zA-Z0-9_]*[[:space:]]+[^$"\n[:space:]]'
  # MATCHES:    config-read-work.sh id_pattern "{number:04d}"
  # REJECTS:    config-read-work.sh id_pattern        (no default)
  # REJECTS:    config-read-work.sh id_pattern "$VAR" (variable, not literal)
  ```
  Search both `*.sh` and `SKILL.md`, exclude `workspaces/` and
  `test-config.sh`, expect no matches.

- `Test: no scripts/skills file outside the config-* family contains
  a literal 'config-read-value.sh work.' invocation` (catches future
  regressions where a new consumer bypasses the wrapper):

  ```bash
  STALE_PATTERN='config-read-value\.sh"?[[:space:]]+\\?\\?[[:space:]]*"?work\.'
  # MATCHES:    "$X/config-read-value.sh" work.id_pattern
  # MATCHES:    "$X/config-read-value.sh" \    (line-continuation form,
  #               "work.foo"                    via the awk pre-pass)
  # REJECTS:    config-read-value.sh paths.plans
  # excluding scripts/test-config.sh and the config-read-* family
  ```

- `Test: every known work.* consumer file references
  config-read-work.sh` (positive assertion — count the expected migrated
  call sites and assert all appear in the grep output).

- `Test: every SKILL.md that invokes config-read-work.sh has an
  allowed-tools entry that permits it` (replaces the per-file audit in
  Section 3 — enumerate SKILL.md files containing
  `config-read-work.sh`, parse their `allowed-tools:` frontmatter, and
  assert each contains either bare `Bash`, a `${CLAUDE_PLUGIN_ROOT}/scripts/*`
  glob, or a `${CLAUDE_PLUGIN_ROOT}/scripts/config-*` glob).

- `Test: SKILL.md inline bash snippets that invoke config-read-work.sh
  parse cleanly under bash -n` — extract each fenced bash block from
  the migrated SKILL.md files, pipe through `bash -n`, assert exit 0.
  Catches typos in pasted snippets that structural greps miss.

### Success Criteria

#### Automated Verification:

- [ ] `bash scripts/test-config.sh` passes all migration assertions
- [ ] `grep -RE 'config-read-value\.sh[^|]*\bwork\.' skills/` returns no
      matches (excluding documentation lines / comments)
- [ ] Each migrated bash script is executable and exits 0 in a freshly
      initialised repo:
      `bash skills/work/scripts/work-item-next-number.sh`,
      `bash skills/work/scripts/work-item-resolve-id.sh`,
      `bash skills/visualisation/visualise/scripts/write-visualiser-config.sh`

#### Manual Verification:

- [ ] `/accelerator:create-work-item` runs end-to-end and writes a
      correctly-named file using the configured `id_pattern`
- [ ] `/accelerator:list-work-items` displays existing items unchanged
- [ ] `/accelerator:extract-work-items` produces unchanged output on a
      sample document
- [ ] `/accelerator:visualisation:visualise` (or whatever the
      visualisation entry point is) renders work items correctly
- [ ] `/accelerator:init-jira` (interactive) prompts for the default
      project code identically to pre-migration when none is set
- [ ] `/accelerator:configure view` is unchanged at this phase (Phase 5
      adds the `work.*` rows)

---

## Phase 5: Wire `work.*` into `config-dump.sh`

### Overview

Surface all three `work.*` keys in `/accelerator:configure view` output by
adding a `WORK_KEYS` iteration to `config-dump.sh`, mirroring the existing
loops for paths, templates, review, and agent keys.

### Changes Required

#### 1. Extend config-dump.sh

**File**: `scripts/config-dump.sh`
**Changes**: After the existing `TEMPLATE_KEYS` loop at lines 184-192, add
a `WORK_KEYS` loop. The dump uses `config-read-value.sh` directly (not
`config-read-work.sh`) so a misconfigured `work.integration` value still
appears in the output instead of crashing the dump — diagnostics are the
dump's job, not validation. However, the dump *does* perform a cheap
non-fatal validation against the same enum and annotates invalid values
inline, so users running `/accelerator:configure view` to debug see the
typo immediately rather than waiting for an integration skill to
`log_die`.

`WORK_KEYS` is now stored fully-dotted (matching `PATH_KEYS`/`TEMPLATE_KEYS`),
so the loop iterates entries directly without re-prefixing.

```bash
# After TEMPLATE_KEYS loop:

# Work keys (defined in config-defaults.sh)
for i in "${!WORK_KEYS[@]}"; do
  key="${WORK_KEYS[$i]}"
  default="${WORK_DEFAULTS[$i]}"
  value=$("$READ_VALUE" "$key" "$default")
  source=$(get_source "$key")
  display="$value"
  # Non-fatal enum validation for work.integration: surface typos at
  # the diagnostic surface rather than waiting for an integration skill
  # to hard-fail. Empty value is valid (unset is the default state).
  # Iterates WORK_INTEGRATION_VALUES (defined in config-defaults.sh) so
  # the dump and the wrapper share one source of truth for allowed values.
  if [ "$key" = "work.integration" ] && [ -n "$value" ]; then
    valid=false
    for allowed in "${WORK_INTEGRATION_VALUES[@]}"; do
      if [ "$value" = "$allowed" ]; then valid=true; break; fi
    done
    if [ "$valid" = false ]; then
      allowed_list="${WORK_INTEGRATION_VALUES[*]}"
      display="$value (invalid: must be ${allowed_list// /, })"
    fi
  fi
  if [ -n "$value" ]; then
    echo "| \`$key\` | \`$display\` | $source |"
  else
    echo "| \`$key\` | *(not set)* | $source |"
  fi
done
```

#### 2. Tests

**File**: `scripts/test-config.sh`
**Changes**: In the `=== config-dump.sh ===` section, append:

- `Test: work.integration appears in dump as *(not set)* with default source when unconfigured`
- `Test: work.id_pattern appears in dump with default {number:04d}`
- `Test: work.default_project_code appears in dump as *(not set)* by default`
- `Test: configured work.integration: jira shows 'jira' with team source (no invalid annotation)`
- `Test: local override of work.integration shows 'local' source`
- `Test: invalid work.integration value (e.g. 'jura') appears in dump
  with '(invalid: must be jira, linear, trello, github-issues)'
  annotation` *(replaces the previous "displays as-is" expectation; the
  dump now surfaces typos non-fatally)*
- `Test: invalid work.integration value does not cause dump to exit
  non-zero` *(regression guard for the design decision to validate
  non-fatally in the dump path)*
- `Test: completeness check — all three work.* keys appear in dump output`
  (mirrors the review completeness check at line 2580)
- `Test: work.* rows appear in WORK_KEYS declaration order in dump
  output (work.integration before work.id_pattern before
  work.default_project_code)`
- `Test: mixed source attribution — integration local, id_pattern team,
  default_project_code default — each row's source column reflects its
  own provenance independently`

### Success Criteria

#### Automated Verification:

- [ ] `bash scripts/test-config.sh` passes the new dump tests
- [ ] In a repo with `work.integration: jira`,
      `work.id_pattern: "{project}-{number:04d}"`, and
      `work.default_project_code: PROJ` configured,
      `bash scripts/config-dump.sh` output contains three rows starting
      with `` `work.integration` ``, `` `work.id_pattern` ``,
      `` `work.default_project_code` `` (in that order)
- [ ] `bash scripts/config-dump.sh` does not exit non-zero in a repo
      with an invalid `work.integration` value

#### Manual Verification:

- [ ] `/accelerator:configure view` shows all three `work.*` keys with
      correct source attribution (team / local / default)
- [ ] In a repo with no `.accelerator/config.md`,
      `/accelerator:configure view` is unchanged from current behaviour
      (dump exits early when no config files exist — line 19-21 of
      `config-dump.sh`)

---

## Phase 6: Documentation + regression guard

### Overview

Document the new `work.integration` key in
`skills/config/configure/SKILL.md`, extend the recognised-keys allow-list,
add the local-first storage note (AC3), update README and integration-skill
docs so the new key is discoverable from where users actually look, and
add a structural regression guard that defends AC1 against future drift.

### Changes Required

#### 1. Update skills/config/configure/SKILL.md

**File**: `skills/config/configure/SKILL.md` (around lines 425-520)
**Changes**: Four edits in the `### work` section.

**A.** Update the section lead-in (line 429): the existing prose reads
"Customise work-item identifier filenames. Two keys are recognised:".
Once a third row is added the count is wrong and the framing is too
narrow (the section is no longer just about identifier filenames).
Rewrite to:

```markdown
Configure work-item identifiers and the active remote tracker. Three
keys are recognised:
```

**B.** Update the table at lines 431-434 — add a new top row for
`integration` (so the table reads `integration`, `id_pattern`,
`default_project_code` in declaration order matching `WORK_KEYS`):

```markdown
| Key                          | Default          | Description                                |
|------------------------------|------------------|--------------------------------------------|
| `integration`                | (empty)          | Active remote tracker. Allowed values: `jira`, `linear`, `trello`, `github-issues`. When set, integration skills auto-scope to `default_project_code`. Team→local override precedence applies; use `/accelerator:configure view` to confirm which source is active. |
| `id_pattern`                 | `{number:04d}`   | DSL controlling work-item ID shape         |
| `default_project_code`       | (empty)          | Project code substituted into `{project}`  |
```

**C.** Add a new subsection between the table and the existing
`#### Pattern DSL Reference` heading (after the existing example
configuration block at lines 436-449):

```markdown
#### Local-first storage

Work items are always written to `meta/work/` as local files, regardless
of whether `work.integration` is configured. The remote integration is an
additional layer on top of local storage, not a replacement. A skill that
pushes a work item to a remote tracker must still write the work item
to `meta/work/` first.

This invariant applies to every skill under `skills/work/`
(`create-work-item`, `update-work-item`, `list-work-items`,
`extract-work-items`, `refine-work-item`, `review-work-item`,
`stress-test-work-item`). Integration skills under `skills/integrations/`
add remote behaviour on top — they read from and write to the same
local store. When `work.integration` is unset, every work-management
skill operates purely against `meta/work/` with no external API calls.
```

**D.** Update the "Recognised keys" paragraph at lines 517-520:

```markdown
#### Recognised keys

Only `work.integration`, `work.id_pattern`, and
`work.default_project_code` are recognised. Other `work.*` keys are not
consumed by any plugin script.
```

#### 2. Update README.md

**File**: `README.md`
**Changes**: The README references `work.id_pattern` and
`work.default_project_code` in three places (around lines 122, 280, 342)
but mentions nothing about `work.integration` or the local-first
invariant. Add a short note alongside the existing `work.*` mention near
line 280 (the most prose-heavy of the three) introducing the third key
and pointing to the configure SKILL.md `### work` section for the full
table. The exact insertion shape:

```markdown
The `work.integration` key (allowed values `jira`, `linear`, `trello`,
`github-issues`; empty by default) selects the active remote tracker.
When unset, all work-management skills operate purely against
`meta/work/` with no external API calls. See
[`skills/config/configure/SKILL.md`](skills/config/configure/SKILL.md#work)
for the full reference.
```

If the existing line 122/342 references are list bullets rather than
prose, update them in line (single-line tweak each) to mention the
third key without a full prose insertion.

#### 3. Add integration-skill SKILL.md notes

So the new key is discoverable from where users actually look (the
integration skills), not only from the canonical config doc, add a short
"Configuration" or "Prerequisites" note to:

- `skills/integrations/jira/init-jira/SKILL.md`
- `skills/integrations/jira/create-jira-issue/SKILL.md`
- `skills/integrations/jira/search-jira-issues/SKILL.md`

Each note is one-or-two lines stating that integration auto-scoping
requires `work.integration: jira` and a configured `work.default_project_code`,
with a link to the `### work` section of `configure/SKILL.md`. Keep
content centralised in `configure/SKILL.md`; the per-skill notes are
discoverability pointers, not duplication.

#### 4. Replace AC1 regression guard with structural inverse assertion

**File**: `scripts/test-config.sh`
**Changes**: The previous draft used a hardcoded grep pattern
(`\b(curl|wget)\b|jira-(auth|api)\.sh|jira_(curl|api|auth)`) which would
silently rot as new integrations land (`linear-api.sh`, etc.). Replace
with a structural inverse assertion that grows automatically.

```bash
echo "=== regression guard: local work skills don't reach into integrations ==="

LOCAL_WORK_SKILLS=(
  skills/work/create-work-item
  skills/work/update-work-item
  skills/work/list-work-items
  skills/work/extract-work-items
  skills/work/refine-work-item
  skills/work/review-work-item
  skills/work/stress-test-work-item
)

# Inverse assertion: local skills must not source any path under
# skills/integrations/ and must not source any *-api.sh / *-auth.sh
# helper. This grows automatically as new integrations land — adding
# linear-api.sh, trello-auth.sh, etc., does not require updating this
# pattern. The test exhaustively defends AC1 by asserting the absence
# of integration coupling rather than enumerating known external-call
# entry points.
INTEGRATION_REF_PATTERN='skills/integrations/|/[a-z][a-z-]*-(api|auth)\.sh\b'

for skill in "${LOCAL_WORK_SKILLS[@]}"; do
  echo "Test: $skill does not depend on any integrations/ path"
  hits=$(cd "$PLUGIN_ROOT" && grep -RIEn \
    --include='*.sh' --include='*.md' \
    --exclude-dir=workspaces \
    "$INTEGRATION_REF_PATTERN" "$skill" 2>/dev/null || true)
  assert_eq "no integration references in $skill" "" "$hits"
done

# Belt-and-braces: also reject curl/wget in local skills (cheap, and
# catches the rare case where a script bypasses the helper layer).
HTTP_TOOL_PATTERN='\b(curl|wget)\b'
for skill in "${LOCAL_WORK_SKILLS[@]}"; do
  echo "Test: $skill makes no direct HTTP calls"
  hits=$(cd "$PLUGIN_ROOT" && grep -RIEn \
    --include='*.sh' --include='*.md' \
    --exclude-dir=workspaces \
    "$HTTP_TOOL_PATTERN" "$skill" 2>/dev/null || true)
  assert_eq "no curl/wget in $skill" "" "$hits"
done
```

The structural assertion's pattern names what is *forbidden* by AC1 —
sourcing `skills/integrations/` or any `*-api.sh` / `*-auth.sh` helper —
which is fixed by the architecture. It does not need tuning when new
integrations are added.

### Success Criteria

#### Automated Verification:

- [ ] `bash scripts/test-config.sh` passes the regression guard tests
      (one per local skill × two assertions, all green)
- [ ] `grep -n "work.integration" skills/config/configure/SKILL.md`
      returns at least three lines (table row, local-first paragraph,
      recognised-keys paragraph)
- [ ] `grep -n "Three keys are recognised" skills/config/configure/SKILL.md`
      returns one match (the lead-in fix)
- [ ] `grep -n "Local-first storage" skills/config/configure/SKILL.md`
      returns one match
- [ ] `grep -n "work.integration" README.md` returns at least one match
- [ ] `grep -l "work.integration" skills/integrations/jira/init-jira/SKILL.md
      skills/integrations/jira/create-jira-issue/SKILL.md
      skills/integrations/jira/search-jira-issues/SKILL.md` lists all
      three files
- [ ] No new test in `test-config.sh` relies on the existence of
      `migrations/` (which does not exist on this branch)

#### Manual Verification:

- [ ] `/accelerator:configure` (the SKILL itself) renders the `### work`
      section with the new row, the local-first paragraph, and the
      updated recognised-keys allow-list
- [ ] A search across `meta/` for `work.integration` returns only the
      authoritative locations (work item, research, this plan, the
      `configure` SKILL.md) — no other docs accidentally reference the
      key with stale guidance

---

## Testing Strategy

### Unit Tests

All assertions live in `scripts/test-config.sh`. Each phase adds tests in
its own `=== … ===` section so failures localise cleanly. Approximate test
count distribution:

- `=== config-read-work.sh ===` (Phases 1–2): ~24 tests (covering
  defaults, precedence, explicit-empty, index alignment, scoped
  validation, error-message content, capture-form exit propagation)
- `=== work_resolve_default_project ===` (Phase 3): ~16 tests
  (behaviour matrix across four integrations × project set/unset, plus
  AC4 surfacing through the helper, plus structural refactor
  assertions)
- `=== consumer migration: config-read-work.sh ===` (Phase 4): ~5 tests
  (no-inline-default with multiline-aware regex, no stale
  `config-read-value.sh work.*`, positive presence assertion,
  allowed-tools coverage, SKILL.md `bash -n` parse check)
- `=== config-dump.sh: work.* keys ===` (Phase 5): ~10 tests
  (per-key presence, defaults, source attribution, invalid-annotation,
  non-fatal-on-invalid, ordering, mixed-source attribution,
  completeness)
- `=== regression guard: local work skills don't reach into integrations ===`
  (Phase 6): 14 tests (two assertions per local skill — no
  integrations/ refs, no curl/wget)

The total is approximately 69 new test cases. The headline figure
matters less than the per-phase enumeration, which is what implementers
should verify against.

Existing tests at lines 244-293 (the previous `work.id_pattern` /
`work.default_project_code` reads via `config-read-value.sh`) remain
unchanged — they continue to verify the underlying `config-read-value.sh`
behaviour, which `config-read-work.sh` builds on.

### Integration Tests

The structural assertions in Phases 4 and 6 act as cross-file integration
tests for the codebase. The existing `bash scripts/test-config.sh` runner
is the integration entry point; no new harness is required.

The Jira flow tests (`skills/integrations/jira/scripts/test-jira-*.sh`)
must continue to pass after Phase 3's refactor — they exercise the
`jira-create-flow.sh` and `jira-search-flow.sh` end-to-end behaviour.

### Manual Testing Steps

1. Configure a repo with `work.integration: jira` and
   `work.default_project_code: PROJ`. Run `/accelerator:configure view` —
   confirm all three `work.*` rows appear with team source.
2. Override locally with `.accelerator/config.local.md` containing
   `work.integration: linear`. Re-run `/accelerator:configure view` —
   confirm `integration` row shows `linear` with local source.
3. Set `work.integration: jura` (typo). Run `/accelerator:create-jira-issue
   --summary "test"` (or any flow that calls
   `work_resolve_default_project`). Confirm an error is surfaced from
   `config-read-work.sh integration`.
4. Unset `work.default_project_code`, keep `work.integration: jira`. Run
   `/accelerator:create-jira-issue --summary "test"` (without
   `--project`). Confirm the warning is printed and the existing
   `E_CREATE_NO_PROJECT` exit path triggers.
5. Remove the entire `work:` section from `.accelerator/config.md`. Run
   `/accelerator:list-work-items` and `/accelerator:create-work-item` —
   confirm both succeed without printing any work-integration-related
   warnings or errors.
6. With `work.integration: jira` set but `work.default_project_code`
   unset, run `/accelerator:init-jira` — confirm the prompt-default flow
   triggers normally (no AC5 warning, since init does not call
   `work_resolve_default_project`).

## Performance Considerations

The validation in `config-read-work.sh` is a single string-match enum
check on a four-element set — negligible. The new `WORK_KEYS` loop in
`config-dump.sh` adds three additional value reads per dump invocation,
each parsing the (already-cached) frontmatter block — also negligible.
`work_resolve_default_project` invokes `config-read-work.sh` twice per
call (once for `integration`, once for `default_project_code`); this is
unchanged in net cost from the existing single-read because the previous
code only read `default_project_code`. The doubled read is the price of
the AC5 warning.

## Migration Notes

No data migration is required. Existing repos either:

- Have no `work.integration` configured → behave identically to today
  (empty value, no validation triggered, no warning).
- Have a valid `work.integration` configured → behaviour unchanged.
- Have an invalid `work.integration` configured → surfaced as a clear
  error the next time `config-read-work.sh integration` is called from an
  integration consumer.

The structural changes (`WORK_KEYS` centralisation, consumer migration,
dump wiring) are purely refactors; no user-visible behaviour changes for
the two existing keys.

## References

- Source: `meta/work/0046-work-management-system-configuration.md`
- Research: `meta/research/2026-05-08-0046-work-management-system-configuration.md`
- Parent epic: `meta/work/0045-work-management-integration.md`
- Centralisation precedent: `scripts/config-defaults.sh` (post-0030 state),
  `scripts/config-read-path.sh:17-42` (thin-wrapper pattern)
- Validation precedent (rejected — too lenient for AC4):
  `scripts/config-read-review.sh:129-138` (`validate_severity`)
- Validation precedent (adopted — hard-fail):
  `skills/work/scripts/work-item-common.sh:17-23` (`wip_validate_pattern`),
  `scripts/log-common.sh` (`log_die` / `log_warn`)
- Single-definition-site invariant: `scripts/test-config.sh:2476-2483`
- No-inline-default invariant: `scripts/test-config.sh:2883`
- ADR-0016 (userspace configuration model) — establishes the team /
  local / default override layering that `work.integration` inherits.
- ADR-0017 (configuration extension points) — sanctions adding a new
  top-level config family (`work.*`) with its own thin wrapper and
  registry, and is the precedent this plan applies.

## Adoption

Process bookkeeping (kept separate from the documentation phase):
update the work item at
`meta/work/0046-work-management-system-configuration.md` from
`status: ready` to `status: in-progress` on plan adoption, then to
`status: done` on completion. No content change.
