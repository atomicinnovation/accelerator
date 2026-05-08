---
date: "2026-05-08T09:02:26Z"
type: plan
skill: create-plan
work-item: "0030"
status: draft
---

# Centralise PATH and TEMPLATE Config Arrays Implementation Plan

## Overview

Extract the `PATH_KEYS`, `PATH_DEFAULTS`, and `TEMPLATE_KEYS` array
definitions from `scripts/config-dump.sh` into a new
`scripts/config-defaults.sh`, sourced transitively via
`scripts/config-common.sh`. This is a pure refactor — no observable
behaviour changes — driven by TDD: a new structural test asserts the
single-definition-site invariant before the inline definitions are
removed.

## Current State Analysis

`scripts/config-dump.sh:175-219` defines all three arrays inline. Any
future rename of a path key (e.g. the recent `paths.tickets` →
`paths.work` migration) requires editing both the definition and its
consumers, but at least the definition lives in one place per array.
That definition site is currently embedded in a script whose primary
job is rendering a configuration dump table — the arrays are not
discoverable for other config scripts that might want them.

`scripts/config-common.sh` is the existing "shared bash module" sourced
by `config-dump.sh:12`. It already exposes shared constants
(`AGENT_PREFIX` at line 11), parsers (`config_extract_frontmatter`,
`config_parse_array`), and resolvers (`config_resolve_template`).
Adding a sibling `config-defaults.sh` and sourcing it from
`config-common.sh:8` (after `vcs-common.sh`) fits the existing pattern
and propagates the arrays to any current or future caller transitively.

The relevant tests live in `scripts/test-config.sh`:
- `=== config-dump.sh ===` block at lines 2426-2555 exercises the
  rendering end-to-end and is the regression suite for this refactor.
- `=== config-read-path.sh ===` block at lines 2606-2761 exercises
  every `paths.*` resolution with both default-when-unset and
  override-honoured cases.

The harness is wired to `mise run test:integration:config`, which globs
`scripts/test-*.sh` via `tasks/test/integration.py:21-24`.

### Key Discoveries:

- Single definition site confirmed at `scripts/config-dump.sh:175-187`,
  `:189-201`, `:212-219`. All three arrays use bare `=` literal form
  (no `declare -a`), so the AC2 grep
  (`'PATH_KEYS=\|PATH_DEFAULTS=\|TEMPLATE_KEYS='`) catches them.
- `scripts/config-common.sh:7-8` already establishes `SCRIPT_DIR` and
  sources `vcs-common.sh` — line 8 is the natural insertion point for
  `source "$SCRIPT_DIR/config-defaults.sh"`.
- `scripts/config-dump.sh:12` already sources `config-common.sh`, so no
  edit beyond deletion of the inline blocks is needed in `config-dump.sh`.
- `scripts/test-config.sh:20` already sources `config-common.sh`, so
  the new arrays are available in test scope as a side effect once the
  source line is added — no new wiring needed for the structural tests.
- `TEMPLATE_DEFAULTS` does not exist (template fallback is the
  three-tier `config_resolve_template()` at `config-common.sh:188-227`);
  it is not part of this migration.
- `workspaces/` is jj workspace checkouts, not source duplicates. Only
  one `config-dump.sh` exists.

## Desired End State

After the plan completes:

- `scripts/config-defaults.sh` exists and is the only file in the repo
  (excluding `workspaces/`) where `PATH_KEYS=`, `PATH_DEFAULTS=`, or
  `TEMPLATE_KEYS=` are assigned.
- `scripts/config-common.sh:8` sources `config-defaults.sh` immediately
  after `vcs-common.sh`.
- `scripts/config-dump.sh` no longer contains the array literals at
  lines 175-219; the arrays are inherited via the existing
  `source "$SCRIPT_DIR/config-common.sh"` at line 12.
- `mise run test:integration:config` passes, including the
  pre-existing `config-dump.sh` and `config-read-path.sh` test blocks
  and a new `=== config-defaults.sh ===` block added in Phase 1.

### Verification

```bash
mise run test:integration:config
grep -rn --include='*.sh' \
  'PATH_KEYS=\|PATH_DEFAULTS=\|TEMPLATE_KEYS=' \
  --exclude-dir=workspaces .
# Expected: only scripts/config-defaults.sh matches.
```

## What We're NOT Doing

- **Consumer-site refactoring.** The 13 bash consumer scripts and ~25
  SKILL.md exec blocks that pass `<key> <default>` pairs to
  `config-read-path.sh`/`config-read-value.sh` are untouched. Their
  defaults remain inline at the call site.
- **`DIR_KEYS`/`DIR_DEFAULTS` in `skills/config/init/scripts/init.sh`.**
  These use a different vocabulary (bare keys, not `paths.`-prefixed)
  and include `design_inventories`/`design_gaps` which `PATH_KEYS`
  does not. Unification is deferred to a follow-on work item per
  the work item's Open Questions.
- **`TEMPLATE_DEFAULTS`.** Does not exist as an array; template
  fallback is `config_resolve_template()`. No extraction needed.
- **`scripts/config-read-path.sh`.** Lines 7-21 are a comment-only
  enumeration; no executable definitions to migrate.
- **Adding `skills/config/init/scripts/test-init.sh` to the harness.**
  AC3 in the work item is scoped to `mise run test:integration:config`;
  test-init.sh is a pre-existing harness gap and `init.sh` is not
  modified here.
- **Editing the work item or research document.** Both are already
  consistent with the corrected scope.

## Implementation Approach

Two TDD phases. Each phase introduces a failing test that captures the
invariant being established, then makes the test pass with the minimum
required edit. Existing tests serve as the regression suite — a refactor
is observable behaviourally only via the existing tests, so they must
continue to pass at the end of each phase.

The phases are sequenced so that the AC2 grep invariant (only
`config-defaults.sh` defines the arrays) becomes the test that drives
the file-deletion step in Phase 2 — which is the actual point of this
work item.

---

## Phase 1: Create `scripts/config-defaults.sh` and add structural tests

### Overview

Add a new `=== config-defaults.sh ===` test block to `test-config.sh`
that asserts the file exists and defines the three arrays with the
expected entries. Then create the file. At the end of this phase the
arrays are defined in two places (the new file plus the inline block in
`config-dump.sh`) — Phase 2 collapses that.

### Changes Required:

#### 1. New test block in `scripts/test-config.sh`

**File**: `scripts/test-config.sh`
**Changes**: Insert a new `=== config-defaults.sh ===` block immediately
before the existing `=== config-dump.sh ===` block at line 2426. The
block must:
- Source `scripts/config-defaults.sh` in a subshell.
- Assert each array is set with the expected number of entries (11, 11, 6).
- Assert the contents match the expected values in the expected order
  (using a single string-comparison per array via `IFS=` join, mirroring
  how existing tests use `assert_eq`).

The expected values (from `scripts/config-dump.sh:175-219`):
- `PATH_KEYS`: `paths.plans`, `paths.research`, `paths.decisions`,
  `paths.prs`, `paths.validations`, `paths.review_plans`,
  `paths.review_prs`, `paths.review_work`, `paths.templates`,
  `paths.work`, `paths.notes`.
- `PATH_DEFAULTS`: `meta/plans`, `meta/research`, `meta/decisions`,
  `meta/prs`, `meta/validations`, `meta/reviews/plans`,
  `meta/reviews/prs`, `meta/reviews/work`, `.accelerator/templates`,
  `meta/work`, `meta/notes`.
- `TEMPLATE_KEYS`: `templates.plan`, `templates.research`,
  `templates.adr`, `templates.validation`, `templates.pr-description`,
  `templates.work-item`.

Test pattern (mirrors lines 2518-2535 of test-config.sh):

```bash
echo "=== config-defaults.sh ==="
echo ""

DEFAULTS_FILE="$SCRIPT_DIR/config-defaults.sh"

echo "Test: config-defaults.sh exists"
if [ -f "$DEFAULTS_FILE" ]; then
  echo "  PASS: file exists"
  PASS=$((PASS + 1))
else
  echo "  FAIL: file exists"
  echo "    Expected: $DEFAULTS_FILE"
  FAIL=$((FAIL + 1))
fi

echo "Test: PATH_KEYS has expected length and order"
EXPECTED_PATH_KEYS="paths.plans paths.research paths.decisions paths.prs paths.validations paths.review_plans paths.review_prs paths.review_work paths.templates paths.work paths.notes"
ACTUAL_PATH_KEYS_LEN=$( source "$DEFAULTS_FILE" && echo "${#PATH_KEYS[@]}" )
assert_eq "PATH_KEYS length" "11" "$ACTUAL_PATH_KEYS_LEN"
ACTUAL_PATH_KEYS=$( source "$DEFAULTS_FILE" && echo "${PATH_KEYS[*]}" )
assert_eq "PATH_KEYS contents" "$EXPECTED_PATH_KEYS" "$ACTUAL_PATH_KEYS"

echo "Test: PATH_DEFAULTS has expected length and order"
EXPECTED_PATH_DEFAULTS="meta/plans meta/research meta/decisions meta/prs meta/validations meta/reviews/plans meta/reviews/prs meta/reviews/work .accelerator/templates meta/work meta/notes"
ACTUAL_PATH_DEFAULTS_LEN=$( source "$DEFAULTS_FILE" && echo "${#PATH_DEFAULTS[@]}" )
assert_eq "PATH_DEFAULTS length" "11" "$ACTUAL_PATH_DEFAULTS_LEN"
ACTUAL_PATH_DEFAULTS=$( source "$DEFAULTS_FILE" && echo "${PATH_DEFAULTS[*]}" )
assert_eq "PATH_DEFAULTS contents" "$EXPECTED_PATH_DEFAULTS" "$ACTUAL_PATH_DEFAULTS"

echo "Test: TEMPLATE_KEYS has expected length and order"
EXPECTED_TEMPLATE_KEYS="templates.plan templates.research templates.adr templates.validation templates.pr-description templates.work-item"
ACTUAL_TEMPLATE_KEYS_LEN=$( source "$DEFAULTS_FILE" && echo "${#TEMPLATE_KEYS[@]}" )
assert_eq "TEMPLATE_KEYS length" "6" "$ACTUAL_TEMPLATE_KEYS_LEN"
ACTUAL_TEMPLATE_KEYS=$( source "$DEFAULTS_FILE" && echo "${TEMPLATE_KEYS[*]}" )
assert_eq "TEMPLATE_KEYS contents" "$EXPECTED_TEMPLATE_KEYS" "$ACTUAL_TEMPLATE_KEYS"

echo "Test: config-dump.sh renders at least one paths.* and one templates.* row"
REPO=$(setup_repo)
mkdir -p "$REPO/.accelerator"
printf -- '---\nreview:\n  max_inline_comments: 15\n---\n' > "$REPO/.accelerator/config.md"
OUTPUT=$(cd "$REPO" && bash "$CONFIG_DUMP")
if echo "$OUTPUT" | grep -qF '| `paths.plans` |' && echo "$OUTPUT" | grep -qF '| `templates.plan` |'; then
  echo "  PASS: paths.* and templates.* rows present in config-dump output"
  PASS=$((PASS + 1))
else
  echo "  FAIL: paths.* and templates.* rows present in config-dump output"
  echo "    Output: $(printf '%q' "$OUTPUT")"
  FAIL=$((FAIL + 1))
fi

echo ""
```

The `paths.*` / `templates.*` row-presence test is the regression
check for the transitive sourcing chain. In Phase 1 it passes
trivially because `config-dump.sh` still has inline definitions; the
value lands in Phase 2, where deleting the inline definitions without
correctly wiring `config-common.sh` to source `config-defaults.sh`
would cause `config-dump.sh` to render no `paths.*` / `templates.*`
rows and this assertion to fail. Without this test the regression
suite has a gap — the existing `=== config-dump.sh ===` block only
asserts `review.*` and one `agents.*` row, so a sourcing-chain
breakage could pass `mise run test:integration:config` and only be
caught by the manual fixture smoke check. Mirrors the completeness
pattern already used at `test-config.sh:2518-2535`.

The `( source ... && echo ... )` subshell isolates each assertion from
the parent shell's already-sourced environment, matches the existing
test-config.sh idiom (no nested `bash -c` quoting), and inherits the
default IFS so `${ARRAY[*]}` joins on a single space.

Run the suite — these tests fail (the file does not exist).

#### 2. Create `scripts/config-defaults.sh`

**File**: `scripts/config-defaults.sh` (new)
**Changes**: Define the three arrays verbatim from
`scripts/config-dump.sh:175-187`, `:189-201`, `:212-219`. No
`declare -a` and no rearrangement. Include a `#!/usr/bin/env bash`
shebang to match the sibling `*-common.sh` modules
(`config-common.sh`, `vcs-common.sh`, `atomic-common.sh`,
`log-common.sh`) — harmless on a sourced file and removes a
stand-out inconsistency. Include a structured banner comment in the
sibling-module style (one-line purpose, blank comment line,
rationale + scope note) and apply a single file-scope `# shellcheck
disable=SC2034` rather than per-array directives.

```bash
#!/usr/bin/env bash

# Shared path and template key arrays.
#
# Sourced transitively via config-common.sh so the arrays are available
# to config-dump.sh and any future config script that sources
# config-common.sh. Centralising the definitions here means the next
# default rename is a one-line edit at this site rather than a
# grep-and-replace across the consumer surface.
#
# Scope note: this file currently centralises only PATH and TEMPLATE
# *keys* (and the path defaults). Review-key DEFAULTS, AGENT_KEYS, and
# AGENT_DEFAULTS remain inline in config-dump.sh because they have no
# external consumers. DIR_KEYS/DIR_DEFAULTS in
# skills/config/init/scripts/init.sh use a different vocabulary
# (bare keys vs paths.*-prefixed) and are tracked for unification in a
# follow-on work item.
#
# Do not source this file directly — source config-common.sh instead.

# shellcheck disable=SC2034
# (variables are exported-by-sourcing; consumers are invisible to a
# per-file lint)

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
)

TEMPLATE_KEYS=(
  "templates.plan"
  "templates.research"
  "templates.adr"
  "templates.validation"
  "templates.pr-description"
  "templates.work-item"
)
```

ShellCheck honours file-scope `disable` directives placed before the
first command, so a single directive after the banner suppresses
SC2034 for every array in the module — future arrays inherit the
suppression without requiring a new directive line.

Run the suite — Phase 1 tests now pass. Existing config-dump and
path-resolution tests continue to pass (they test the rendering, not
the array definition site).

### Success Criteria:

#### Automated Verification:

- [ ] `scripts/config-defaults.sh` exists.
- [ ] New `=== config-defaults.sh ===` test block passes
      (`mise run test:integration:config`), including the file-existence
      assertion, the length + ordered-contents assertions for each of
      the three arrays, and the row-presence assertion that
      `config-dump.sh` renders at least one `paths.*` and one
      `templates.*` row.
- [ ] Existing `=== config-dump.sh ===` (lines 2426-2555) tests pass.
- [ ] Existing `=== config-read-path.sh ===` (lines 2606-2761) tests pass.
- [ ] No shell-lint regressions: `mise run lint` (or repo equivalent).

#### Manual Verification:

- [ ] Diff of `scripts/config-defaults.sh` against the inline blocks at
      `scripts/config-dump.sh:175-187,189-201,212-219` shows identical
      array contents in identical order.

---

## Phase 2: Source `config-defaults.sh` from `config-common.sh`, remove inline definitions, enforce single-definition-site invariant

### Overview

Add a structural test that captures AC2 — only `config-defaults.sh`
should match the array-definition grep. Then add the source line to
`config-common.sh` and delete the inline definitions in `config-dump.sh`.

At the end of this phase the arrays are defined in exactly one place
and propagate via the existing `config-dump.sh:12 → config-common.sh →
config-defaults.sh` source chain.

### Changes Required:

#### 1. Add the AC2 single-definition-site test

**File**: `scripts/test-config.sh`
**Changes**: Append a new test to the `=== config-defaults.sh ===` block
added in Phase 1. The test runs the AC2 grep and asserts the only
matching file is `scripts/config-defaults.sh`.

The grep must run from the plugin root (resolvable as
`PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"`, already established at
`test-config.sh:22`) and exclude the `workspaces/` directory because it
contains jj workspace checkouts. The pattern must match alternative
declaration forms a future contributor might reasonably use
(`declare -a`, `readonly`, `export`, `local`, `+=`) — narrowing to bare
`=` would let the invariant be silently broken by a slightly different
form. The grep output is piped through `sort -u` so the assertion
contract is independent of filesystem traversal order (matching the
verification command in Phase 2's automated checks).

```bash
echo "Test: config-defaults.sh is the only definition site for the arrays"
DEFINITION_PATTERN='^[[:space:]]*((declare|typeset)[[:space:]]+(-[a-zA-Z]+[[:space:]]+)?|readonly[[:space:]]+|export[[:space:]]+|local[[:space:]]+)?(PATH_KEYS|PATH_DEFAULTS|TEMPLATE_KEYS)(\+)?='
MATCHES=$(cd "$PLUGIN_ROOT" && grep -rlnE --include='*.sh' \
  --exclude-dir=workspaces \
  "$DEFINITION_PATTERN" . | sort -u)
EXPECTED="./scripts/config-defaults.sh"
assert_eq "only config-defaults.sh defines PATH_KEYS/PATH_DEFAULTS/TEMPLATE_KEYS" \
  "$EXPECTED" "$MATCHES"
```

The leading `^[[:space:]]*` anchors the match to the start of a line
(plus optional indentation), so commented references like
`# Defined in PATH_KEYS=` and inline references inside string literals
no longer trigger false positives. `grep -E` is required for the
extended regex; the alternation block matches bare assignments and the
`declare`/`typeset`/`readonly`/`export`/`local` prefixes (with any
combination of flags via `-[a-zA-Z]+`, including `g`/`i` for `declare
-ga` or `declare -gA`); the optional `(\+)?` catches `+=` append forms.

Run the suite — this test fails (`config-dump.sh` still defines the
arrays inline, so two files match).

#### 2. Source `config-defaults.sh` from `config-common.sh`

**File**: `scripts/config-common.sh`
**Changes**: Add `source "$SCRIPT_DIR/config-defaults.sh"` immediately
after the existing `source "$SCRIPT_DIR/vcs-common.sh"` at line 8.

```bash
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/vcs-common.sh"
source "$SCRIPT_DIR/config-defaults.sh"
```

This makes the arrays available transitively to any caller of
`config-common.sh`, including `config-dump.sh:12` and `test-config.sh:20`.

#### 3. Remove inline array definitions from `config-dump.sh`

**File**: `scripts/config-dump.sh`
**Changes**: Delete the three array-literal blocks at lines 175-187
(`PATH_KEYS`), 189-201 (`PATH_DEFAULTS`), and 212-219 (`TEMPLATE_KEYS`).
Preserve the surrounding loop at lines 203-209 (which iterates over
`PATH_KEYS`/`PATH_DEFAULTS`) and lines 221-229 (which iterates over
`TEMPLATE_KEYS`) — those continue to work unchanged because the arrays
are now provided by the transitive source.

Replace the existing one-word section markers with one-line breadcrumbs
that point a reader at the new definition site instead of just labelling
the section:
- Line 174 (`# Path keys`) → `# Path keys (defined in config-defaults.sh)`
- Line 211 (`# Template keys`) → `# Template keys (defined in config-defaults.sh)`

After the deletions, `config-dump.sh` shrinks by ~35 lines and the
`for` loops sit directly under the existing agent-key loop at line 172,
each preceded by its breadcrumb comment so a reader scanning the file
in isolation can locate the array contents without grepping.

Run the suite — the AC2 test now passes; the existing config-dump and
path-resolution tests continue to pass.

### Success Criteria:

#### Automated Verification:

- [ ] AC2 grep returns only `scripts/config-defaults.sh`:
      `grep -rlnE --include='*.sh' --exclude-dir=workspaces '^[[:space:]]*((declare|typeset)[[:space:]]+(-[a-zA-Z]+[[:space:]]+)?|readonly[[:space:]]+|export[[:space:]]+|local[[:space:]]+)?(PATH_KEYS|PATH_DEFAULTS|TEMPLATE_KEYS)(\+)?=' . | sort -u`
      outputs exactly `./scripts/config-defaults.sh`.
- [ ] All `=== config-defaults.sh ===` tests pass (including the new
      single-definition-site test).
- [ ] All `=== config-dump.sh ===` tests pass:
      `mise run test:integration:config`.
- [ ] All `=== config-read-path.sh ===` tests pass.
- [ ] Full integration suite passes: `mise run test:integration`.
- [ ] Phase 1's row-presence assertion (added to the `=== config-defaults.sh ===`
      block) still passes after the inline definitions are removed —
      confirming the transitive sourcing chain delivers `paths.*` and
      `templates.*` rows from `config-defaults.sh` rather than from
      inline definitions in `config-dump.sh`.

#### Manual Verification:

- [ ] `scripts/config-dump.sh` no longer contains `PATH_KEYS=`,
      `PATH_DEFAULTS=`, or `TEMPLATE_KEYS=`.
- [ ] `scripts/config-common.sh` sources `config-defaults.sh`
      immediately after the existing `vcs-common.sh` source line
      (line-number-agnostic — the existing blank separator may or may
      not be preserved at the implementer's discretion).
- [ ] No `${CLAUDE_PLUGIN_ROOT}` references introduced anywhere
      (sourcing uses the existing `$SCRIPT_DIR` pattern).

---

## Testing Strategy

### Unit / Structural Tests (added in this plan):

- File-existence assertion for `scripts/config-defaults.sh`.
- Array-content assertions for `PATH_KEYS`, `PATH_DEFAULTS`,
  `TEMPLATE_KEYS` (length and ordered contents).
- Single-definition-site invariant via AC2 grep.

These tests live in the new `=== config-defaults.sh ===` block in
`scripts/test-config.sh`, picked up automatically by
`mise run test:integration:config` via the existing
`tasks/test/integration.py:21-24` glob.

### Regression Tests (existing, must continue to pass):

- `scripts/test-config.sh:2426-2555` — `config-dump.sh` end-to-end
  (no-config, team-only, local-only, merged, defaults-shown,
  agent-defaults).
- `scripts/test-config.sh:2606-2761` — `config-read-path.sh` for every
  `paths.*` key with both default and override paths.
- `scripts/test-config.sh:3034-3194` — SKILL.md `config-read-path.sh`
  reference checks (these don't touch the arrays directly but exercise
  the path-key vocabulary).

### Manual Testing Steps:

1. From the repo root, run
   `bash scripts/config-dump.sh` in a clean repo with no
   `.accelerator/config.md` — output should be empty (matches existing
   behaviour at `config-dump.sh:18-21`).
2. Create a fixture `.accelerator/config.md` with
   `paths.plans: docs/plans` and re-run — the rendered table should show
   `paths.plans | docs/plans | team`.
3. Add a `templates.plan: custom-plan.md` line and re-run — the table
   should show `templates.plan | custom-plan.md | team`.

These three steps verify that the transitive sourcing chain delivers
the arrays at runtime under the same conditions the test fixtures use.

## Performance Considerations

None. The change adds one `source` line to a script already sourced
once per `config-dump.sh` invocation. The new file is ~30 lines of
array literals — sub-millisecond load time.

## Migration Notes

No data migration. No behavioural change. Pure refactor. Rollback is
`jj abandon` of the change — no state lives outside the working tree.

## References

- Work item: `meta/work/0030-centralise-path-defaults.md`
- Implementation research:
  `meta/research/2026-05-08-0030-centralise-path-defaults-implementation.md`
- Related ADR:
  `meta/decisions/ADR-0023-meta-directory-migration-framework.md`
- Originating migration:
  `meta/plans/2026-04-25-rename-tickets-to-work-items.md`
- Downstream consumer:
  `meta/work/0052-make-documents-locator-paths-config-driven.md`
- Definition site (current):
  - `scripts/config-dump.sh:175-187` (`PATH_KEYS`)
  - `scripts/config-dump.sh:189-201` (`PATH_DEFAULTS`)
  - `scripts/config-dump.sh:212-219` (`TEMPLATE_KEYS`)
- Recommended insertion point:
  `scripts/config-common.sh:8` (after `source "$SCRIPT_DIR/vcs-common.sh"`)
- Existing test-suite anchors:
  - `scripts/test-config.sh:2426-2555` (`=== config-dump.sh ===` block)
  - `scripts/test-config.sh:2606-2761` (`=== config-read-path.sh ===` block)
  - `tasks/test/integration.py:21-24` (`config()` task glob)
