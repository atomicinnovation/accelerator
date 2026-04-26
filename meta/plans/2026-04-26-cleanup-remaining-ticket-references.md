---
date: "2026-04-26T00:00:00+01:00"
type: plan
skill: create-plan
work-item: ""
status: draft
---

# Cleanup Remaining Ticket References Implementation Plan

## Overview

Following the rename of "tickets" to "work items", 32 references were missed
across 9 files. This plan eliminates them in four phases ordered by user-facing
impact. Production code is entirely clean; all misses are confined to test
scripts, templates, and documentation.

## Current State Analysis

Research conducted at commit `f03a7dfe` identified every remaining reference.
The missed sites cluster into five structural areas:

- **Templates** — `templates/plan.md` still carries a `ticket:` frontmatter
  key and a `meta/tickets/` example path that are injected into every new plan
- **Agent definition** — `agents/documents-locator.md` example output block
  causes the agent to emit `### Tickets` and `meta/tickets/…` paths on every
  invocation
- **`test-lens-structure.sh`** — functional variable (`TICKET_LENSES`) and
  function (`_is_ticket_lens`) plus comments still use old terminology
- **`test-work-item-scripts.sh`** — internal helpers (`make_ticket`,
  `make_tagged_ticket`), two inline `printf` fixtures using `ticket_id:`, and
  four H1 fixture headings reading "Test Ticket"
- **Scattered single-site misses** — one comment, one prose reference, one eval
  assertion name, one body text in a work item document

## Desired End State

All `ticket` references that are not intentionally historical have been renamed
to use `work-item`/`work_item` terminology. The following grep should return
**zero** results after the plan is complete:

```bash
grep -rn --include="*.sh" --include="*.md" --include="*.json" \
  -i "ticket" \
  scripts/ templates/ agents/ \
  skills/work/review-work-item/SKILL.md \
  skills/work/update-work-item/evals/ \
  skills/work/scripts/ \
  meta/work/0026-init-skill-for-repo-bootstrap.md
```

All existing test suites continue to pass: `mise run test`.

### Key Discoveries

- `templates/plan.md:5` — the `ticket:` frontmatter key is injected into every
  plan created by `/accelerator:create-plan`
- `agents/documents-locator.md:74–76` — the example output block is reproduced
  verbatim by the agent, so callers receive wrong paths until fixed
- `scripts/test-lens-structure.sh:25,27,136` — `TICKET_LENSES` and
  `_is_ticket_lens` are functional (not just cosmetic); both the variable and
  every call site must be renamed together
- `skills/work/scripts/test-work-item-scripts.sh:218,268` — `ticket_id:` in
  inline `printf` fixtures (separate from the `make_ticket` helper, whose
  fixture body already uses `work_item_id:`)
- `skills/work/scripts/test-work-item-scripts.sh:284,445` — `make_ticket` and
  `make_tagged_ticket` have many call sites scattered through the file; all
  must be renamed atomically

## What We're NOT Doing

- **`# ADR Ticket:` headings in `meta/work/0001–0029`** — generic English noun
  in pre-rename human-authored body text; out of scope per the original rename
  plan scope (machine-readable identifiers only)
- **`skills/config/migrate/`** — references there are intentionally correct
  (the migration script targets old names as its input)
- **`CHANGELOG.md`** — historical record of the breaking change; must not be
  altered
- **`.claude/settings.local.json:59–65`** — stale `mv` allow-entries; correct
  to prune as housekeeping but separate from this cleanup

---

## Phase 1: Templates

### Overview

Fix the two template files that are consumed every time a plan or PR description
is created. These carry the highest ongoing cost while stale.

### Changes Required

#### 1. `templates/plan.md`

**File**: `templates/plan.md`

Line 5 — rename frontmatter key:

```diff
-ticket: "{ticket reference, if any}"
+work-item: "{work-item reference, if any}"
```

Line 107 — update example reference path:

```diff
-- Original ticket: `meta/tickets/eng-XXXX.md`
+- Original work item: `meta/work/NNNN-title.md`
```

#### 2. `templates/pr-description.md`

**File**: `templates/pr-description.md`

Line 24 — update placeholder text:

```diff
-[Link to relevant ticket, plan, or research document if applicable]
+[Link to relevant work item, plan, or research document if applicable]
```

### Success Criteria

#### Automated Verification

- [ ] Grep confirms no `ticket` references remain in `templates/`:
  `grep -rni ticket templates/`
- [ ] Tests pass: `mise run test`

#### Manual Verification

- [ ] A new plan created via `/accelerator:create-plan` carries `work-item:` in
  its frontmatter (not `ticket:`)

---

## Phase 2: Agent Definition

### Overview

Fix `agents/documents-locator.md` so the agent stops emitting stale `### Tickets`
headings and `meta/tickets/` paths in its output.

### Changes Required

#### 1. `agents/documents-locator.md`

**File**: `agents/documents-locator.md`

Line 25 — update categorisation label:

```diff
-- Tickets (usually in tickets/ subdirectory)
+- Work items (usually in work/ subdirectory)
```

Lines 54–55 — update directory tree entry:

```diff
-├── tickets/      # Ticket documentation
+├── work/         # Work item documentation
```

Line 74 — update example output heading:

```diff
-### Tickets
+### Work Items
```

Lines 75–76 — update example output paths (two occurrences):

```diff
-- `meta/tickets/eng-1234.md` - Implement rate limiting for API
-- `meta/tickets/eng-1235.md` - Rate limit configuration design
+- `meta/work/0001-implement-rate-limiting.md` - Implement rate limiting for API
+- `meta/work/0002-rate-limit-configuration-design.md` - Rate limit configuration design
```

Line 121 — update naming hint:

```diff
-- Ticket files often named `eng-XXXX.md`
+- Work item files often named `NNNN-title.md`
```

### Success Criteria

#### Automated Verification

- [ ] Grep confirms no `ticket` references remain in `agents/`:
  `grep -rni ticket agents/`
- [ ] Tests pass: `mise run test`

---

## Phase 3: Functional Script Renames

### Overview

Rename the variable, function, and all call sites in `test-lens-structure.sh`,
then rename the two helper functions and fix fixture content in
`test-work-item-scripts.sh`. These are coordinated renames — variable/function
name and all usages must move together.

### Changes Required

#### 1. `scripts/test-lens-structure.sh`

**File**: `scripts/test-lens-structure.sh`

Lines 4 and 9–12 — update header comment block:

```diff
-# Lint every ticket review lens SKILL.md for structural conformance.
+# Lint every work-item review lens SKILL.md for structural conformance.
 # When a single lens directory name is given (e.g. "scope-lens") only that
 # lens is checked; otherwise all *-lens directories under the lenses base are
 # checked.
 #
-# Structural checks apply to every lens.  The peer-ticket-lens reference check
-# applies only to lenses whose identifier appears in TICKET_LENSES (the five
-# built-in ticket lenses) because code-review lenses are not expected to
-# reference ticket-specific peers.
+# Structural checks apply to every lens.  The peer-work-item-lens reference check
+# applies only to lenses whose identifier appears in WORK_ITEM_LENSES (the five
+# built-in work-item lenses) because code-review lenses are not expected to
+# reference work-item-specific peers.
```

Line 24 — update inline comment:

```diff
-# Built-in ticket lens identifiers — peer-reference check applies only to these.
+# Built-in work-item lens identifiers — peer-reference check applies only to these.
```

Line 25 — rename variable:

```diff
-TICKET_LENSES=(clarity completeness dependency scope testability)
+WORK_ITEM_LENSES=(clarity completeness dependency scope testability)
```

Lines 27–33 — rename function and update body:

```diff
-_is_ticket_lens() {
+_is_work_item_lens() {
   local id="$1"
-  for tl in "${TICKET_LENSES[@]}"; do
+  for tl in "${WORK_ITEM_LENSES[@]}"; do
     [ "$id" = "$tl" ] && return 0
   done
   return 1
 }
```

Line 124 — update inline comment:

```diff
-  # For ticket lenses this should follow the "Review as a[n] ... specialist ..."
+  # For work-item lenses this should follow the "Review as a[n] ... specialist ..."
```

Line 136 — update call site:

```diff
-  if _is_ticket_lens "$LENS_ID"; then
+  if _is_work_item_lens "$LENS_ID"; then
```

Lines 139–140 — update TICKET_LENSES reference in loop body:

```diff
-    for PEER in "${TICKET_LENSES[@]}"; do
+    for PEER in "${WORK_ITEM_LENSES[@]}"; do
```

Lines 146 and 149 — update PASS/FAIL messages:

```diff
-      echo "  PASS: $LENS_NAME 'What NOT to Do' names at least 3 peer ticket lenses ($PEER_COUNT found)"
+      echo "  PASS: $LENS_NAME 'What NOT to Do' names at least 3 peer work-item lenses ($PEER_COUNT found)"
 ...
-      echo "  FAIL: $LENS_NAME 'What NOT to Do' names only $PEER_COUNT peer ticket lenses (need >= 3)"
+      echo "  FAIL: $LENS_NAME 'What NOT to Do' names only $PEER_COUNT peer work-item lenses (need >= 3)"
```

#### 2. `skills/work/scripts/test-work-item-scripts.sh`

**File**: `skills/work/scripts/test-work-item-scripts.sh`

Line 100 — fix test comment misquoting the current error message:

```diff
-# Test 10: Highest 9999 → exits 1 with "ticket number space exhausted" error
+# Test 10: Highest 9999 → exits 1 with "work item number space exhausted" error
```

Lines 66–67 — update test comment and echo:

```diff
-# Test 5: Directory with non-ticket files only (README.md) → outputs 0001
-echo "Test: Directory with non-ticket files only"
+# Test 5: Directory with non-work-item files only (README.md) → outputs 0001
+echo "Test: Directory with non-work-item files only"
```

Lines 74–75 — update test comment and echo:

```diff
-# Test 6: Mixed ticket and non-ticket files → outputs next after highest ticket
-echo "Test: Mixed ticket and non-ticket files"
+# Test 6: Mixed work-item and non-work-item files → outputs next after highest work item
+echo "Test: Mixed work-item and non-work-item files"
```

Line 218 — fix inline printf fixture frontmatter key:

```diff
-printf -- '---\nticket_id: 0001\nstatus: draft  \n---\n\n# 0001: Test\n' \
+printf -- '---\nwork_item_id: 0001\nstatus: draft  \n---\n\n# 0001: Test\n' \
```

Line 268 — fix inline printf fixture frontmatter key:

```diff
-printf -- '---\nticket_id: 0001\nstatus: \n---\n\n# 0001: Test\n' \
+printf -- '---\nwork_item_id: 0001\nstatus: \n---\n\n# 0001: Test\n' \
```

Lines 283–284 — rename helper function and update comment:

```diff
-# Helper: create a standard ticket fixture in a temp repo
-make_ticket() {
+# Helper: create a standard work-item fixture in a temp repo
+make_work_item() {
```

Line 298 — update fixture H1 heading:

```diff
-# 0001: Test Ticket
+# 0001: Test Work Item
```

Lines 307, 314, 321, 328, 335, 341, 348, 387, 407, 413
— rename all `make_ticket` call sites to `make_work_item` (replace all occurrences).

Lines 444–445 — rename tagged helper function and update comment:

```diff
-# Helper: create a ticket with specific tags content
-make_tagged_ticket() {
+# Helper: create a work item with specific tags content
+make_tagged_work_item() {
```

Line 456 — update fixture H1 heading:

```diff
-# 0001: Test Ticket
+# 0001: Test Work Item
```

Lines 463, 470, 477, 484, 491, 498, 520, 577, 584, 591 — rename all
`make_tagged_ticket` call sites to `make_tagged_work_item` (replace all
occurrences).

Lines 512 and 536 — update remaining fixture H1 headings:

```diff
-# 0001: Test Ticket
+# 0001: Test Work Item
```

### Success Criteria

#### Automated Verification

- [ ] Grep confirms no `ticket` references remain in `scripts/` or
  `skills/work/scripts/`:
  `grep -rni ticket scripts/ skills/work/scripts/`
- [ ] Tests pass: `mise run test`

---

## Phase 4: Minor Fixes

### Overview

Four isolated single-site changes: one stale comment, one prose reference to
a variable name, one eval assertion name, and one body text in a work item
document.

### Changes Required

#### 1. `scripts/test-hierarchy-format.sh`

**File**: `scripts/test-hierarchy-format.sh`

Lines 2–3 — update header comment (functional path variables on lines 21–22 are
already correct):

```diff
-# Check that the canonical tree fence in list-tickets/SKILL.md and
-# refine-ticket/SKILL.md are byte-for-byte identical.
+# Check that the canonical tree fence in list-work-items/SKILL.md and
+# refine-work-item/SKILL.md are byte-for-byte identical.
```

#### 2. `skills/work/review-work-item/SKILL.md`

**File**: `skills/work/review-work-item/SKILL.md`

Line 104 — update prose reference to the variable name (the actual variable in
`scripts/config-read-review.sh:54` is already `BUILTIN_WORK_ITEM_LENSES`):

```diff
-By default, run every lens registered in `BUILTIN_TICKET_LENSES` unless the
+By default, run every lens registered in `BUILTIN_WORK_ITEM_LENSES` unless the
```

Lines 208 and 246 — rename configuration key references in prose:

```diff
-   REVISE verdict when `ticket_revise_severity` is `major` or higher.
+   REVISE verdict when `work_item_revise_severity` is `major` or higher.
```

```diff
-   - If `ticket_revise_severity` is `none`, skip the severity-based REVISE
+   - If `work_item_revise_severity` is `none`, skip the severity-based REVISE
```

#### 3. `skills/work/update-work-item/evals/evals.json`

**File**: `skills/work/update-work-item/evals/evals.json`

Line 63 — rename eval assertion:

```diff
-          "name": "handles_legacy_ticket",
+          "name": "handles_legacy_work_item",
```

#### 4. `meta/work/0026-init-skill-for-repo-bootstrap.md`

**File**: `meta/work/0026-init-skill-for-repo-bootstrap.md`

Line 107 — update directory name in body text:

```diff
-  (`tmp/`, `templates/`, `tickets/`, `notes/`) — skill-output directories get
+  (`tmp/`, `templates/`, `work/`, `notes/`) — skill-output directories get
```

### Success Criteria

#### Automated Verification

- [ ] Grep confirms no remaining missed `ticket` references across all target
  files:
  ```bash
  grep -rni ticket \
    scripts/ templates/ agents/ \
    skills/work/review-work-item/SKILL.md \
    skills/work/update-work-item/evals/ \
    skills/work/scripts/ \
    meta/work/0026-init-skill-for-repo-bootstrap.md
  ```
- [ ] Tests pass: `mise run test`

---

## Testing Strategy

Each phase is independently testable. After every phase run `mise run test` to
confirm nothing regressed. The full-corpus grep above is the definitive
completeness check after Phase 4.

The changes in Phase 3 involve coordinated multi-site renames. Verify the
rename is atomic before running tests — a partial rename (function renamed but
some call sites missed) will cause a bash error, which the test suite will
surface immediately.

## References

- Research: `meta/research/2026-04-26-remaining-ticket-references-post-migration.md`
- Original rename plan: `meta/plans/2026-04-25-rename-tickets-to-work-items.md`
- Canonical term decision: `meta/decisions/ADR-0022-work-item-terminology.md`
