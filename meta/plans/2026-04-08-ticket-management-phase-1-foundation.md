---
date: "2026-04-08T12:47:11+00:00"
type: plan
skill: create-plan
ticket: ""
status: draft
---

# Ticket Management Phase 1: Foundation and Configuration

## Overview

Create the foundational infrastructure for the ticket management skill
category: companion scripts for numbering and field extraction, a ticket
template, plugin registration, and configuration updates. This phase delivers
no user-facing skills but establishes everything that Phases 2-6 depend on.

## Current State Analysis

The Accelerator plugin has an established pattern for lifecycle-managed
filesystem artifacts, best exemplified by the `decisions/` category:

- **Companion scripts** in `skills/decisions/scripts/`:
  - `adr-next-number.sh` — scans `meta/decisions/` for `ADR-NNNN` files,
    returns the next sequential number in `NNNN` format
  - `adr-read-status.sh` — extracts `status:` from YAML frontmatter
- **Test harness** in `skills/decisions/scripts/test-adr-scripts.sh` — custom
  bash test runner with temp repo setup and pass/fail summary
- **Shared test helpers** in `scripts/test-helpers.sh` — provides
  `assert_eq`, `assert_exit_code`, `assert_file_executable`,
  `assert_stderr_empty`, `test_summary`, and the `PASS`/`FAIL`
  counters. Any new `test-*.sh` file sources this rather than
  redefining the assertions locally.
- **Template** in `templates/adr.md` — YAML frontmatter plus structured
  sections
- **Plugin registration** in `.claude-plugin/plugin.json` — skill directories
  listed in the `skills` array
- **Path configuration** in `scripts/config-read-path.sh` — `tickets` is
  already registered; `review_tickets` is not
- **Init skill** in `skills/config/init/SKILL.md` — creates output directories
  with `.gitkeep`
- **Configure skill** in `skills/config/configure/SKILL.md` — documents paths,
  templates, and their management

### Key Discoveries:

- `meta/tickets/` contains 25 existing tickets numbered `0001` through `0025`,
  all with type `adr-creation-task` and the filename pattern
  `NNNN-description.md` (no `ADR-` prefix)
- The `adr-next-number.sh` script uses a glob pattern `ADR-[0-9][0-9][0-9][0-9]*`
  — the ticket equivalent must use `[0-9][0-9][0-9][0-9]-*` to match the
  existing convention
- The `adr-read-status.sh` script does pure bash frontmatter parsing (no
  external YAML tools) — the ticket scripts must follow the same approach
- The test harness uses `setup_repo` to create temp directories with `.git/`
  markers for `find_repo_root` — ticket tests must follow the same pattern
- `config-read-template.sh` validates template names dynamically via
  `config_enumerate_templates` (lists `templates/*.md` files) — adding
  `templates/ticket.md` automatically makes `ticket` a valid key
- The configure skill lists available template keys on line 425-426:
  `` `plan`, `research`, `adr`, `validation`, `pr-description` `` — must add
  `ticket`

## Desired End State

After this plan is complete:

1. `skills/tickets/scripts/ticket-next-number.sh` exists and correctly returns
   the next sequential ticket number by scanning the configured tickets
   directory for `NNNN-*.md` files
2. `skills/tickets/scripts/ticket-read-status.sh` exists and extracts the
   `status` field from a ticket file's YAML frontmatter (delegates to
   `ticket-read-field.sh` to avoid duplicating the frontmatter parser)
3. `skills/tickets/scripts/ticket-read-field.sh` exists and extracts any named
   field from a ticket file's YAML frontmatter
4. `skills/tickets/scripts/test-ticket-scripts.sh` exists and passes, covering
   all three scripts above
5. `templates/ticket.md` exists with the template structure from the research
   document
6. `.claude-plugin/plugin.json` includes `"./skills/tickets/"` in the skills
   array
7. `scripts/config-read-path.sh` documents `review_tickets` as a path key
8. `skills/config/init/SKILL.md` creates the `review_tickets` directory
9. `skills/config/configure/SKILL.md` documents `review_tickets` in the paths
   table, adds `ticket` to the template key lists, and includes `ticket` in
   the template management example

### Verification:

```bash
# All tests pass
bash skills/tickets/scripts/test-ticket-scripts.sh

# Scripts are executable
test -x skills/tickets/scripts/ticket-next-number.sh
test -x skills/tickets/scripts/ticket-read-status.sh
test -x skills/tickets/scripts/ticket-read-field.sh

# Template is resolvable
bash scripts/config-read-template.sh ticket

# Plugin registration includes tickets
grep -q '"./skills/tickets/"' .claude-plugin/plugin.json

# review_tickets path is documented
grep -q 'review_tickets' scripts/config-read-path.sh

# Init skill includes review_tickets
grep -q 'review_tickets' skills/config/init/SKILL.md

# Configure skill includes ticket template key
grep -q 'ticket' skills/config/configure/SKILL.md
```

## What We're NOT Doing

- No user-facing skills (create-ticket, extract-tickets, etc.) — those are
  Phase 2+
- No review lenses or output formats — those are Phase 4-5
- No migration of existing 25 tickets — they coexist as-is. The 25 legacy
  tickets use a minimal schema (`title`, `type: adr-creation-task`, `status`);
  the new template introduces a richer schema (`ticket_id`, `date`, `author`,
  `type`, `status`, `priority`, `parent`, `tags`). The contract for
  ticket-consuming skills is: `type` and `status` are guaranteed present on
  every ticket; all other fields are new-schema-only and consumers must
  handle missing fields gracefully. The template is user-overridable, so
  teams can replace the shipping schema if they prefer the legacy shape.
- No configurable filename patterns (e.g., `XXX-NNNN-description.md`) — future
  enhancement per research resolved question 1
- No `review_tickets` directory creation — the init skill update documents it
  but the directory is created when `/accelerator:init` is run
- No consolidation of template/path key enumerations. Adding a single
  template key currently requires updating six files (`config-read-template.sh`,
  `config-dump.sh`, `configure/SKILL.md`, `README.md`, `test-config.sh`, and
  the template file itself) because `config-dump.sh` holds hardcoded
  `TEMPLATE_KEYS` / `PATH_KEYS` arrays that parallel the dynamic
  `config_enumerate_templates` helper. Consolidating onto dynamic enumeration
  is a future enhancement; this phase accepts the duplication as known debt.
- No allocation reservation / locking for `ticket-next-number.sh`. Two
  concurrent invocations receive the same number; callers are responsible
  for serialising ticket creation. Acceptable for the current human-driven
  workflow; revisit if multi-agent batch extraction becomes common.
- No migration of `adr-read-status.sh` onto the shared `config_extract_frontmatter`
  helper. The new `ticket-read-field.sh` delegates to the helper; the ADR
  equivalent keeps its own parser for now. Convergence is a future cleanup.

## Implementation Approach

Follow a TDD approach: write failing tests first, then implement the scripts
to make them pass. The three subphases are ordered by dependency — the
numbering script is standalone, the field reading scripts are standalone, and
the config/template changes depend on the scripts directory existing.

---

## Subphase 1A: Ticket Numbering Script

### Overview

Create `ticket-next-number.sh` with its tests. This script scans the
configured tickets directory for the highest `NNNN-*.md` file and returns the
next number. Follows the same structure as `adr-next-number.sh` but with a
different glob pattern (no `ADR-` prefix).

### Changes Required:

#### 1. Test harness (test-first)

**File**: `skills/tickets/scripts/test-ticket-scripts.sh` (new)

Create the test harness with the same structure as
`skills/decisions/scripts/test-adr-scripts.sh`. Source the shared
assertion helpers from `scripts/test-helpers.sh` (this exposes
`assert_eq`, `assert_exit_code`, `assert_file_executable`,
`assert_stderr_empty`, `test_summary`, and the `PASS`/`FAIL` counters)
instead of redefining them locally. The `setup_repo` helper and the
`TMPDIR_BASE` cleanup trap remain local to this file because they
encode a test-harness policy specific to scripts that use
`find_repo_root`; they are not in `test-helpers.sh`. Write
`ticket-next-number.sh` tests only in this subphase (the other script
tests are added in Subphase 1B).

**Tests to include for `ticket-next-number.sh`:**

```bash
# Test 1: No meta/tickets/ directory → outputs 0001
# Test 2: Empty meta/tickets/ directory → outputs 0001
# Test 3: Directory with 0003-foo.md → outputs 0004
# Test 4: Directory with gaps (0001, 0005) → outputs 0006 (uses highest)
# Test 5: Directory with non-ticket files only (README.md) → outputs 0001
# Test 6: Mixed ticket and non-ticket files → outputs next after highest ticket
# Test 7: --count 3 with highest 0002 → outputs 0003, 0004, 0005
# Test 8: --count 0 (invalid) → exits 1
# Test 9: --count abc (invalid) → exits 1
# Test 10: Highest 9999 → exits 1 with "ticket number space exhausted" error
# Test 11: Files with 5-digit prefix (00003-foo.md) → glob does not match
#           (character 5 is '0', not '-'; the four-digit-then-hyphen pattern
#           rejects it), file ignored, outputs 0001
# Test 12: Existing ADR-style files mixed in (shouldn't match) → ignored
# Test 13: --count with no value → exits 1 with error
# Test 14: Highest 9998 with --count 2 → outputs 9999 only and exits 1
#           (second number would overflow the 4-digit space)
# Test 15: Filename without hyphen (0001.md) → glob does not match, outputs 0001
```

The test file structure:

```bash
#!/usr/bin/env bash
set -euo pipefail

# Test harness for ticket management companion scripts
# Run: bash skills/tickets/scripts/test-ticket-scripts.sh

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Shared assertion helpers (assert_eq, assert_exit_code,
# assert_file_executable, assert_stderr_empty, test_summary) plus the
# PASS/FAIL counters. See scripts/test-helpers.sh for the exposed
# surface.
source "$PLUGIN_ROOT/scripts/test-helpers.sh"

NEXT_NUMBER="$SCRIPT_DIR/ticket-next-number.sh"

# Temporary-directory scaffolding is local to this harness because
# setup_repo encodes the .git-marker requirement of find_repo_root; it is
# not in test-helpers.sh.
TMPDIR_BASE=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BASE"' EXIT

setup_repo() {
  local repo_dir
  repo_dir=$(mktemp -d "$TMPDIR_BASE/repo-XXXXXX")
  mkdir -p "$repo_dir/.git"
  echo "$repo_dir"
}

# === ticket-next-number.sh tests ===
# (as enumerated above)

test_summary
```

#### 2. Implementation

**File**: `skills/tickets/scripts/ticket-next-number.sh` (new)

Parallel to `skills/decisions/scripts/adr-next-number.sh` with these
differences:

| Aspect            | ADR script                                       | Ticket script                                                                                                                  |
|-------------------|--------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------|
| Path key          | `decisions`                                      | `tickets`                                                                                                                      |
| Default path      | `meta/decisions`                                 | `meta/tickets`                                                                                                                 |
| Glob pattern      | `ADR-[0-9][0-9][0-9][0-9]*`                      | `[0-9][0-9][0-9][0-9]-*`                                                                                                       |
| Number extraction | `sed 's/^ADR-//'` then `grep -oE '^[0-9]+'`      | `grep -oE '^[0-9]+'`                                                                                                           |
| Warning message   | references "decisions directory"                 | references "tickets directory"                                                                                                 |
| 9999 boundary     | Emits 10000 (glob permits 5+ digit ADR prefixes) | Exits 1 with "number space exhausted" — glob requires exactly 4 digits so 5-digit files would be invisible on subsequent scans |

```bash
#!/usr/bin/env bash
set -euo pipefail

# Outputs the next sequential ticket number in NNNN format.
# Scans the configured tickets directory for the highest existing NNNN number
# and increments by one. Outputs "0001" if no tickets exist.
#
# Usage: ticket-next-number.sh [--count N]
#   --count N  Output N sequential numbers, one per line (default: 1)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/vcs-common.sh"

COUNT=1
while [ $# -gt 0 ]; do
  case "$1" in
    --count)
      if [ $# -lt 2 ]; then
        echo "Error: --count requires a value" >&2; exit 1
      fi
      COUNT="$2"; shift 2 ;;
    *) echo "Usage: ticket-next-number.sh [--count N]" >&2; exit 1 ;;
  esac
done

if ! [[ "$COUNT" =~ ^[1-9][0-9]*$ ]]; then
  echo "Error: --count requires a positive integer, got '$COUNT'" >&2
  exit 1
fi

REPO_ROOT=$(find_repo_root) || REPO_ROOT="$PWD"

TICKETS_PATH=$("$PLUGIN_ROOT/scripts/config-read-path.sh" tickets meta/tickets)

if [[ "$TICKETS_PATH" == /* ]]; then
  TICKETS_DIR="$TICKETS_PATH"
else
  TICKETS_DIR="$REPO_ROOT/$TICKETS_PATH"
fi

HIGHEST=0
if [ ! -d "$TICKETS_DIR" ]; then
  echo "Warning: tickets directory '$TICKETS_DIR' does not exist — defaulting to next number 0001. Run /accelerator:init or create the directory to persist tickets." >&2
else
  for f in "$TICKETS_DIR"/[0-9][0-9][0-9][0-9]-*; do
    [ -e "$f" ] || continue
    BASE=$(basename "$f")
    NUM=$(echo "$BASE" | grep -oE '^[0-9]+')
    if [ -n "$NUM" ] && [ "$((10#$NUM))" -gt "$HIGHEST" ]; then
      HIGHEST=$((10#$NUM))
    fi
  done
fi

# Clamp to the 4-digit number space. The scanning glob above requires
# exactly 4 digits followed by '-', so numbers beyond 9999 would be
# invisible on subsequent runs and cause collisions. Apply the clamp
# uniformly — both for directories we've scanned AND for the
# missing-directory case, because --count could request more than 9999
# numbers even against a fresh repo.
if [ "$((HIGHEST + COUNT))" -gt 9999 ]; then
  echo "Error: ticket number space exhausted (9999 reached); archive completed tickets to free numbers below 9999, or file an enhancement ticket requesting a 5-digit pattern" >&2
  # Still emit any numbers that fit before the boundary, so batch callers
  # can consume what they can before handling the failure.
  for ((i = 1; i <= COUNT; i++)); do
    NEXT=$((HIGHEST + i))
    [ "$NEXT" -gt 9999 ] && break
    printf "%04d\n" "$NEXT"
  done
  exit 1
fi

for ((i = 1; i <= COUNT; i++)); do
  printf "%04d\n" "$((HIGHEST + i))"
done
```

### Success Criteria:

#### Automated Verification:

- [ ] Tests pass: `bash skills/tickets/scripts/test-ticket-scripts.sh`
- [ ] Script is executable: `test -x skills/tickets/scripts/ticket-next-number.sh`
- [ ] Output matches `^[0-9]{4}$` and is greater than the existing highest
      ticket number: `bash skills/tickets/scripts/ticket-next-number.sh`
      (point-in-time sanity check; exact value drifts as tickets are added)

Note: The `--count` case branch includes a guard (`[ $# -lt 2 ]`) to produce
a clear error message when `--count` is provided without a value, rather than
relying on `set -u` to emit an opaque bash diagnostic.

---

## Subphase 1B: Ticket Field Reading Scripts

### Overview

Create `ticket-read-status.sh` and `ticket-read-field.sh` with their tests.
`ticket-read-status.sh` is a direct parallel of `adr-read-status.sh`.
`ticket-read-field.sh` is a new generic version that extracts any named
frontmatter field — it subsumes the status reader but the dedicated status
script is kept for convenience and consistency with the ADR pattern.

### Changes Required:

#### 1. Tests (test-first)

**File**: `skills/tickets/scripts/test-ticket-scripts.sh` (append)

Add two new test sections to the existing test harness.

**Tests for `ticket-read-status.sh`:**

```bash
# Test 1: Valid frontmatter status: draft → outputs "draft"
# Test 2: Valid frontmatter status: ready → outputs "ready"
# Test 3: Quoted value status: "draft" → outputs "draft" (strips quotes)
# Test 4: No space after colon status:draft → outputs "draft"
# Test 5: Trailing whitespace → outputs "draft" (stripped)
# Test 6: Missing file → exits 1
# Test 7: File with no frontmatter → exits 1
# Test 8: Unclosed frontmatter → exits 1
# Test 9: Status in body ignored, frontmatter value returned
# Test 10: Empty status value → outputs empty string
# Test 11: No arguments → exits 1
```

**Tests for `ticket-read-field.sh`:**

```bash
# Test 1: Read type field → outputs "story"
# Test 2: Read priority field → outputs "high"
# Test 3: Read status field → outputs "draft" (works same as read-status)
# Test 4: Read parent field → outputs "0001"
# Test 5: Read missing field → exits 1 with error
# Test 6: Quoted field value → strips quotes
# Test 7: Field with array value tags: [a, b] (unquoted YAML) → outputs
#           "[a, b]" verbatim (the raw value is returned; callers parse
#           arrays via config_parse_array if needed)
# Test 8: Missing file → exits 1
# Test 9: No frontmatter (first line is not `---`) → exits 1 with
#           "No YAML frontmatter... Add a '---' line as the first line" error
# Test 10: Unclosed frontmatter (first line is `---` but no closing `---`)
#           → exits 1 with "opened but not closed... Add a '---' line after
#           the last frontmatter key" error
# Test 11: No arguments → exits 1
# Test 12: One argument (file only, no field name) → exits 1
# Test 13: Field name in body ignored, frontmatter value returned
# Test 14: Duplicate key (two `status:` lines with different values) →
#           first-match-wins (returns the first occurrence; pins the
#           contract and catches a regression to last-match-wins)
# Test 15: Prefix-collision (query `tag` when frontmatter has only `tags: [...]`)
#           → exits 1 (must not match as a substring, prefix includes ':')
# Test 16a: Literal-match — fixture has `sub.type: foo`, query `sub.type`
#           → outputs "foo" (proves dots are matched literally)
# Test 16b: Negative-match — fixture has `subXtype: foo` (no `sub.type:` key),
#           query `sub.type` → exits 1 (proves `.` is NOT interpreted as a
#           regex wildcard; the pair of tests together verifies the
#           injection-defence contract)
# Test 17: Value with trailing whitespace after closing quote (e.g.,
#           `status: "draft"  `) → outputs "draft" (no orphan quote;
#           pins the sed ordering invariant)
```

#### 2. ticket-read-status.sh

**File**: `skills/tickets/scripts/ticket-read-status.sh` (new)

Convenience wrapper around `ticket-read-field.sh` that reads the `status`
field. Unlike `adr-read-status.sh` which has its own parsing logic,
`ticket-read-status.sh` delegates to the generic field reader to avoid
duplicating the frontmatter-parsing state machine. The script retains its
own argument validation and usage message for ergonomic consistency with
the ADR pattern.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Reads the status field from a ticket file's YAML frontmatter.
# Usage: ticket-read-status.sh <path-to-ticket-file>
# Outputs the status value (e.g., "draft", "ready", "in-progress").
# Exits with code 1 if file not found or no valid frontmatter.
#
# Convenience wrapper around ticket-read-field.sh.

if [ $# -lt 1 ]; then
  echo "Usage: ticket-read-status.sh <ticket-file-path>" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$SCRIPT_DIR/ticket-read-field.sh" status "$1"
```

#### 3. ticket-read-field.sh

**File**: `skills/tickets/scripts/ticket-read-field.sh` (new)

A generic version that takes a field name as its first argument and a file
path as its second. The argument order follows Unix convention (operation
then target), consistent with `sed`, `awk`, and the field-name-as-verb
mental model (e.g., "read status from file").

**Frontmatter extraction delegates to `config_extract_frontmatter`** in
`scripts/config-common.sh`. That helper already implements a closure-aware
YAML-frontmatter extractor (awk, validated against the same malformed-file
cases this script needs to handle). Reusing it keeps the parsing invariant
in one place instead of growing a third copy alongside `adr-read-status.sh`.
This script's own logic is reduced to: (1) validate arguments, (2) ask the
helper for the frontmatter block, (3) find the target field with bash
prefix-matching, (4) strip whitespace and optional surrounding quotes.

Field lookup uses bash string comparison (`==...*`) and parameter expansion
(`${line#...}`) instead of `grep -qE` / `sed` regex interpolation to avoid
metacharacter injection when field names contain dots, brackets, or other
special characters. This follows the defensive pattern established in
`scripts/config-read-value.sh`.

Duplicate-key semantics are **first-match-wins** — the loop `break`s on
the first line whose prefix matches. This matches `config-read-value.sh`
(awk exits on first match) and diverges from `adr-read-status.sh`, which
currently has no `break` and so returns the last occurrence. The
divergence is intentional — the new family aligns with the more recent
convention — and the "What We're NOT Doing" note records that a future
ADR migration onto the shared helper should adopt first-match-wins. A
test pins this behaviour.

Value normalisation strips leading/trailing whitespace first, then
leading/trailing quotes. This order matters: stripping the trailing
quote before trailing whitespace (the naive order) leaves an orphan
quote for values like `"draft"  ` where the closing quote is followed by
spaces.

The caller's `SCRIPT_DIR` variable is saved before sourcing
`config-common.sh`, because that helper sets its own `SCRIPT_DIR` at
source time. The caller's value isn't used after the source today, but
preserving it prevents a latent trap for future edits.

```bash
#!/usr/bin/env bash
set -euo pipefail

# Reads a named field from a ticket file's YAML frontmatter.
# Usage: ticket-read-field.sh <field-name> <path-to-ticket-file>
# Outputs the raw field value (surrounding quotes are stripped).
# Exits with code 1 if the file is missing, frontmatter is missing or
# unclosed, or the field is not present.
#
# Duplicate keys: first-match-wins (consistent with config-read-value.sh;
# diverges from adr-read-status.sh which currently returns last-match).
# Array values (e.g., `tags: [a, b]`) are returned verbatim — callers are
# responsible for parsing them (see config_parse_array in config-common.sh).

TICKET_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$TICKET_SCRIPT_DIR/../../.." && pwd)"

source "$PLUGIN_ROOT/scripts/config-common.sh"

if [ $# -lt 2 ]; then
  echo "Usage: ticket-read-field.sh <field-name> <ticket-file-path>" >&2
  exit 1
fi

FIELD_NAME="$1"
TICKET_FILE="$2"

if [ ! -f "$TICKET_FILE" ]; then
  echo "Error: File not found: $TICKET_FILE" >&2
  exit 1
fi

# Distinguish no-frontmatter from unclosed-frontmatter before calling the
# helper, so the error message can point at the specific problem. The
# helper returns the same non-zero exit for both cases, which makes the
# common unclosed-`---` typo less self-diagnosing without this pre-check.
FIRST_LINE=$(head -n 1 "$TICKET_FILE")
if ! [[ "$FIRST_LINE" =~ ^---[[:space:]]*$ ]]; then
  echo "Error: No YAML frontmatter in $(basename "$TICKET_FILE"). Add a '---' line as the first line of the file." >&2
  exit 1
fi

FRONTMATTER=$(config_extract_frontmatter "$TICKET_FILE") || {
  echo "Error: YAML frontmatter opened but not closed in $(basename "$TICKET_FILE"). Add a '---' line after the last frontmatter key." >&2
  exit 1
}

PREFIX="${FIELD_NAME}:"
FIELD_VALUE=""
FOUND_FIELD=false
while IFS= read -r line; do
  if [[ "$line" == "${PREFIX}"* ]]; then
    FIELD_VALUE="${line#"$PREFIX"}"
    # Order matters: strip both ends of whitespace BEFORE stripping quotes,
    # so trailing whitespace after a closing quote does not leave the quote
    # orphaned. Each command gets its own -e for readability.
    FIELD_VALUE=$(echo "$FIELD_VALUE" \
      | sed -e 's/^[[:space:]]*//' \
            -e 's/[[:space:]]*$//' \
            -e 's/^["'"'"']//' \
            -e 's/["'"'"']$//')
    FOUND_FIELD=true
    break
  fi
done <<< "$FRONTMATTER"

if [ "$FOUND_FIELD" = true ]; then
  echo "$FIELD_VALUE"
  exit 0
fi

echo "Error: No '$FIELD_NAME' field found in frontmatter of $(basename "$TICKET_FILE")." >&2
exit 1
```

### Success Criteria:

#### Automated Verification:

- [ ] All tests pass: `bash skills/tickets/scripts/test-ticket-scripts.sh`
- [ ] Scripts are executable:
      `test -x skills/tickets/scripts/ticket-read-status.sh`
      `test -x skills/tickets/scripts/ticket-read-field.sh`
- [ ] `ticket-read-status.sh` reads status from an existing ticket:
      `bash skills/tickets/scripts/ticket-read-status.sh meta/tickets/0001-three-layer-review-system-architecture.md`
      (should output `todo`)
- [ ] `ticket-read-field.sh` reads type from an existing ticket:
      `bash skills/tickets/scripts/ticket-read-field.sh type meta/tickets/0001-three-layer-review-system-architecture.md`
      (should output `adr-creation-task`)

---

## Subphase 1C: Template, Plugin Registration, and Config Updates

### Overview

Create the ticket template, register the tickets skill directory in
plugin.json, add `review_tickets` to the path configuration documentation,
update the init skill to create the review_tickets directory, and update the
configure skill to document the `ticket` template key and `review_tickets`
path.

### Changes Required:

#### 1. Ticket template

**File**: `templates/ticket.md` (new)

Based on the research document's Section 6 (Ticket Template Design). This is
the default template that skills will use when creating new tickets.

The template is user-overridable via `meta/templates/ticket.md` or a custom
path in `templates.ticket` — teams that want a different schema (or need to
mirror the simpler schema used by the 25 existing ADR-creation tickets) can
replace it without touching plugin code. The schema below is the shipping
default only.

Authoring hints live as inline YAML `#` comments on the frontmatter lines
that carry placeholders or enumerated values. This follows the pattern
established by `templates/adr.md` and keeps the hints from leaking into
rendered template output: YAML comments are valid YAML syntax, so any tool
that parses the frontmatter (or renders the template via
`config-read-template.sh`) preserves them in situ without turning them
into user-visible body content.

```markdown
---
ticket_id: NNNN                              # from ticket-next-number.sh
date: "YYYY-MM-DDTHH:MM:SS+00:00"            # date -u +%Y-%m-%dT%H:%M:%S+00:00
author: Author Name                          # your name or GitHub handle
type: story                                  # story | epic | task | bug | spike
status: draft                                # draft | ready | in-progress | review | done | blocked | abandoned
priority: medium                             # high | medium | low
parent: ""                                   # ticket number of the parent epic/story, or empty
tags: []                                     # YAML array, e.g. [backend, performance]
---

# NNNN: Title as Short Noun Phrase

**Type**: Story | Epic | Task | Bug | Spike
**Status**: Draft
**Priority**: High | Medium | Low
**Author**: Author Name

## Summary

[1-3 sentence description of what this ticket is about and why it matters]

## Context

[Background information, forces at play, relevant constraints.
Link to source documents if extracted.]

## Requirements

[For stories/tasks: specific requirements to be met]
[For epics: high-level goals and themes]
[For bugs: reproduction steps, expected vs actual behaviour]
[For spikes: research questions and time-box]

## Acceptance Criteria

- [ ] [Criterion 1 — specific, testable, measurable]
- [ ] [Criterion 2]
- [ ] [Criterion 3]

[For stories, prefer Given/When/Then format where applicable:

- Given [precondition], when [action], then [expected result]]

## Dependencies

- Blocked by: [ticket references or external dependencies]
- Blocks: [tickets that depend on this one]

## Technical Notes

[Optional: implementation hints, relevant code references,
architectural considerations discovered during refinement]

## References

- Source: `path/to/source-document.md`
- Related: NNNN, NNNN
- Research: `meta/research/codebase/YYYY-MM-DD-topic.md`
```

#### 2. Plugin registration

**File**: `.claude-plugin/plugin.json`

Add `"./skills/tickets/"` to the skills array, after `"./skills/decisions/"`:

```json
"skills": [
  "./skills/vcs/",
  "./skills/github/",
  "./skills/planning/",
  "./skills/research/",
  "./skills/decisions/",
  "./skills/tickets/",
  "./skills/review/lenses/",
  "./skills/review/output-formats/",
  "./skills/config/"
]
```

Note: This directory won't contain any SKILL.md files yet (those come in
Phase 2+), but registering it now avoids a config change later and is
harmless — the plugin simply finds no skills in the directory.

#### 3. Path configuration documentation

**File**: `scripts/config-read-path.sh`

Add `review_tickets` to the path keys comment block:

```bash
#   review_tickets → where ticket reviews are written (default: meta/reviews/tickets)
```

Insert after the existing `review_prs` comment line in the path keys
documentation block.

The script itself needs no code changes — it delegates to
`config-read-value.sh` which handles any key dynamically.

#### 4. Init skill update

**File**: `skills/config/init/SKILL.md`

Three distinct insertion points must all be updated in lockstep; a literal
reading of any one in isolation would leave the file inconsistent.

**4a. Path Resolution section** (top of file, after the `Review PRs
directory` line): add

```markdown
**Review tickets directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh review_tickets meta/reviews/tickets`
```

This is what makes the `{review tickets directory}` placeholder resolvable
elsewhere in the skill.

**4b. Step 1 directory count** (prose): change "11 directories" to "12
directories" so the count matches the number of `directory` entries in the
Path Resolution section. (Consider removing the literal count in a future
enhancement so it cannot drift — see note below.)

**4c. Step 4 report template**: add the review tickets line

```markdown
  ✓ {review tickets directory} (created | already exists)
```

Insert after the `review prs directory` line in the report.

A concrete assertion in `test-config.sh` (specified in Section 7j below)
verifies that the literal count in 4b agrees with the number of
`**... directory**:` lines in the Path Resolution section, so future
path-key additions cannot silently leave the count stale.

#### 5. Configure skill updates

**File**: `skills/config/configure/SKILL.md`

Three changes:

**5a. Paths table**: Add a row after the `review_prs` row:

```markdown
| `review_tickets` | `meta/reviews/tickets` | Ticket review artifacts                        |
```

**5b. Paths example**: After the `review_prs` line in the YAML example
block, add:

```yaml
  review_tickets: docs/reviews/tickets
```

**5c. Template keys**: Locate the sentence `Available template keys:
\`plan\`, \`research\`, \`adr\`, \`validation\`, \`pr-description\`.`
and change it to:

```
Available template keys: `plan`, `research`, `adr`, `validation`,
`pr-description`, `ticket`.
```

**5d. Template directory example**: In the sample directory listing,
after the `validation.md` line, add:

```
  ticket.md          # Custom ticket template
```

#### 6. Template key enumeration updates

Several additional files enumerate template keys explicitly and must be
updated to include `ticket`:

**6a. `scripts/config-read-template.sh`**: Update the header comment
that lists template names. Change:

```
# Template names: plan, research, adr, validation, pr-description
```

To:

```
# Template names: plan, research, adr, validation, pr-description, ticket
```

**6b. `scripts/config-dump.sh`**: Append `"templates.ticket"` to the
`TEMPLATE_KEYS` array immediately after the `"templates.pr-description"`
entry.

Also append `"paths.review_tickets"` to the `PATH_KEYS` array
immediately after the `"paths.review_prs"` entry, and
`"meta/reviews/tickets"` to the `PATH_DEFAULTS` array immediately after
the `"meta/reviews/prs"` entry (keeping the two arrays positionally
aligned). This ensures `config-dump.sh` output includes the new path
key alongside its template key counterpart.

The ordering convention for `PATH_KEYS` / `PATH_DEFAULTS` is "match the
order of `scripts/config-read-path.sh` and `configure/SKILL.md`'s paths
table" — which groups review_* keys adjacently. Inserting after
`review_prs` preserves that grouping.

**6c. `README.md`**: Locate the `Available template keys:` line in the
document body and change it from:

```
Available template keys: `plan`, `research`, `adr`, `validation`,
`pr-description`.
```

To:

```
Available template keys: `plan`, `research`, `adr`, `validation`,
`pr-description`, `ticket`.
```

#### 7. Config test updates

**File**: `scripts/test-config.sh`

Adding `templates/ticket.md` changes the template count from 5 to 6. Update
the following (anchors are content-based; grep for the quoted strings to
locate each site):

**7a.** In the `config-list-template.sh` section, locate the assertion
`assert_eq "5 template rows" "5" "$LINE_COUNT"` and change both `"5"`
occurrences to `"6"` (label and expected value).

**7b.** Immediately after 7a, update the iteration loop from
`for KEY in plan research adr validation pr-description; do` to:

```bash
for KEY in plan research adr validation pr-description ticket; do
```

**7c.** In the `config-eject-template.sh` section, locate the label
`"Test: --all ejects all 5 templates"` and change to
`"Test: --all ejects all 6 templates"`.

**7d.** Immediately after 7c, update the same eject-all key loop
(`for KEY in plan research adr validation pr-description; do`) by
appending ` ticket`.

**7e.** In the `config_enumerate_templates` section, locate the
assertion `assert_eq "outputs 5 keys" "5" "$LINE_COUNT"` and change both
`"5"` occurrences to `"6"`.

**7f.** Immediately before 7e, inside the same test block, add an
assertion that `ticket` appears alongside the other template keys:

```bash
assert_contains "contains ticket" "ticket" "$OUTPUT"
```

**7g.** Locate the `echo "Test: Unknown template still lists all 5
template names including pr-description"` label and update it to
`"Unknown template lists all 6 template names including pr-description
and ticket"`, then append a matching assertion inside that block:

```bash
assert_contains "error lists ticket" "ticket" "$STDERR_OUTPUT"
```

Do the same at the earlier `Unknown template name` test (grep for
`echo "Test: Unknown template name"`): add an assertion that `ticket`
appears in the available-templates error output.

**7h.** Add a new `Test: paths.review_tickets configured` block after
the existing `paths.notes configured` test, mirroring the
`paths.review_prs` test, asserting that a
`review_tickets: docs/reviews/tickets` override resolves correctly
through `config-read-path.sh review_tickets meta/reviews/tickets`.

**7i.** After the existing `Test: Output contains templates.pr-description
row` block, add two analogous blocks:

```bash
echo "Test: Output contains templates.ticket row"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\ntemplates:\n  ticket: custom/ticket.md\n---\n' > "$REPO/.claude/accelerator.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DUMP")
assert_contains "ticket key in dump" "templates.ticket" "$OUTPUT"

echo "Test: Output contains paths.review_tickets row"
REPO=$(setup_repo)
mkdir -p "$REPO/.claude"
printf -- '---\npaths:\n  review_tickets: docs/reviews/tickets\n---\n' > "$REPO/.claude/accelerator.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DUMP")
assert_contains "review_tickets key in dump" "paths.review_tickets" "$OUTPUT"
```

These assert that the new rows the plan adds to `TEMPLATE_KEYS` /
`PATH_KEYS` in `config-dump.sh` are actually emitted — otherwise a
future refactor that accidentally drops either entry would pass
`test-config.sh` silently.

**7j.** Add a new block at the end of the config test file that asserts
the init skill's directory count stays in sync with the Path Resolution
list — so a future path-key addition cannot silently leave the count
stale:

```bash
echo "Test: init SKILL.md directory count matches Path Resolution list"
INIT_SKILL="$PLUGIN_ROOT/skills/config/init/SKILL.md"
# Count lines of the form `**<Name> directory**:` in the Path Resolution
# section. Matches the existing bold-directory-label convention.
EXPECTED=$(grep -cE '^\*\*[A-Za-z][^*]* directory\*\*:' "$INIT_SKILL")
# The Step 1 prose states "For each of the N directories ...". Extract N
# and assert it equals the count above.
ACTUAL=$(grep -oE 'each of the [0-9]+ directories' "$INIT_SKILL" \
  | grep -oE '[0-9]+' | head -1)
assert_eq "directory count agrees with Path Resolution list" \
  "$EXPECTED" "$ACTUAL"
```

This closes the count/list drift invariant mechanically; the Success
Criterion grep for the literal string `12 directories` only catches this
one change, not future ones.

### Success Criteria:

#### Automated Verification:

- [ ] Template resolves: `bash scripts/config-read-template.sh ticket`
      (should output the template wrapped in code fences)
- [ ] Plugin registration:
      `grep -q '"./skills/tickets/"' .claude-plugin/plugin.json`
- [ ] Path key documented:
      `grep -q 'review_tickets' scripts/config-read-path.sh`
- [ ] Init skill updated:
      `grep -q 'review_tickets' skills/config/init/SKILL.md`
- [ ] Configure skill — path table:
      `grep -q 'review_tickets' skills/config/configure/SKILL.md`
- [ ] Configure skill — template key:
      `grep 'Available template keys' skills/config/configure/SKILL.md | grep -q 'ticket'`
- [ ] config-read-template.sh comment updated:
      `grep -q 'ticket' scripts/config-read-template.sh`
- [ ] config-dump.sh TEMPLATE_KEYS updated:
      `grep -q 'templates.ticket' scripts/config-dump.sh`
- [ ] config-dump.sh PATH_KEYS updated:
      `grep -q 'paths.review_tickets' scripts/config-dump.sh`
- [ ] README.md template keys updated:
      `grep 'Available template keys' README.md | grep -q 'ticket'`
- [ ] Config tests still pass:
      `bash scripts/test-config.sh`
- [ ] Init skill Path Resolution includes review_tickets:
      `grep -q 'Review tickets directory' skills/config/init/SKILL.md`
- [ ] Init skill directory count is 12:
      `grep -q '12 directories' skills/config/init/SKILL.md`
- [ ] New paths.review_tickets resolution test exists in test-config.sh:
      `grep -q 'paths.review_tickets configured' scripts/test-config.sh`
- [ ] New config-dump.sh row tests exist for templates.ticket and
      paths.review_tickets

---

## Testing Strategy

### Automated Tests:

All three companion scripts are tested via the unified test harness:

```bash
bash skills/tickets/scripts/test-ticket-scripts.sh
```

The test harness creates temporary directories with `.git/` markers, populates
them with fixture files, and validates script output and exit codes.

**Test count**: ~44 tests across three scripts (15 for next-number, 11 for
read-status, 18 for read-field — Tests 16a/16b count as two, plus Test 17
for the sed-ordering invariant).

### Manual Verification:

- [ ] Run `ticket-next-number.sh` from the repo root — output matches
      `^[0-9]{4}$` and is greater than the highest existing `NNNN-*.md`
      in `meta/tickets/` (exact value drifts as tickets are added)
- [ ] Run `ticket-read-status.sh` against an existing ticket — should output
      `todo`
- [ ] Run `ticket-read-field.sh type` against an existing ticket — should
      output `adr-creation-task`
- [ ] Run `config-read-template.sh ticket` — should output the template
      content

## References

- Research: `meta/research/codebase/2026-04-08-ticket-management-skills.md` — Section
  1 (Conventions), Section 6 (Template Design), Section 10 (Implementation
  Phasing, Phase 1)
- Pattern source: `skills/decisions/scripts/adr-next-number.sh` — numbering
  script
- Pattern source: `skills/decisions/scripts/adr-read-status.sh` — status
  reader
- Test pattern: `skills/decisions/scripts/test-adr-scripts.sh` — test harness
- Template pattern: `templates/adr.md` — template structure
