---
date: "2026-05-03T00:00:00+01:00"
type: plan
skill: create-plan
work-item: ""
status: approved
---

# Design Convergence Workflow Implementation Plan

## Overview

Implement the inventory-and-diff design convergence workflow described in
`meta/research/2026-05-02-design-convergence-workflow.md`: two new skills
(`inventory-design`, `analyse-design-gaps`) under a new `skills/design/`
category, two new agents (`browser-locator`, `browser-analyser`), two new
artifact templates, a new MCP server dependency on Playwright, and the
supporting init/configure/README/manifest wiring.

The work follows test-driven development at every phase: assertions are
added to the existing `scripts/test-*.sh` harness (or new
`test-design-*.sh` files following the same convention) before the
implementation that makes them pass. Skill development runs through the
`skill-creator:skill-creator` skill; each new skill ships with
`evals/evals.json` and `evals/benchmark.json` that meet the existing 0.9
pass-rate floor enforced by `scripts/test-evals-structure.sh:14-15`.

## Current State Analysis

The repository already contains every piece of infrastructure the new
artifacts and skills need, but the wiring is closed-list in three places
that must be extended.

**Auto-discovery vs. closed-list inventory of touch points:**

- Templates are auto-discovered. `scripts/config-common.sh:103-113`
  globs `templates/*.md`; the three-tier resolver at
  `scripts/config-common.sh:153-193` checks user override and plugin
  default. Adding `templates/design-inventory.md` and
  `templates/design-gap.md` makes them valid keys with no script edit.
- Path keys are arbitrary lookups. `scripts/config-read-path.sh:23-24`
  is a passthrough to `config-read-value.sh` with a `paths.` prefix;
  the second argument is the caller-supplied default. New keys need no
  code change.
- Init directory list is closed. `skills/config/init/SKILL.md:20-31`
  hardcodes 12 path-resolution lines, `skills/config/init/SKILL.md:40`
  hardcodes the count "12", and lines 105-117 hardcode the summary
  block. Adding `design_inventories` and `design_gaps` requires three
  coordinated edits.
- Configure paths table is closed. `skills/config/configure/SKILL.md:386-399`
  is documentation only (no validator), but mismatch hurts user trust.
- README is closed. `README.md:77-88` (meta/ table), `README.md:213-214`
  (template keys list), `README.md:466-475` (agents table) all need
  rows added.
- Plugin manifest skills array is closed. `.claude-plugin/plugin.json:10-21`
  enumerates skill directories; the new `./skills/design/` entry must
  be added.
- No `.mcp.json` exists today. Adding one is precedent-setting per
  research §6.3.

**Test infrastructure (TDD targets):**

- `mise run test` is the CI gate (`mise.toml`, executed in
  `.github/workflows/main.yml:30-31`).
- Bash test scripts live in `scripts/test-*.sh` and source
  `scripts/test-helpers.sh` for the shared assertion helpers
  (`assert_eq`, `assert_contains`, `assert_exit_code`,
  `assert_stderr_contains`, `assert_dir_absent`, `test_summary`).
  **Note** (resolved by Phase 0 below): `assert_file_exists`,
  `assert_file_not_exists`, `assert_empty`, and `assert_file_content_eq`
  live only in `scripts/test-config.sh` today; the same file also
  shadows `assert_contains` with the inverted signature
  `(name, needle, haystack)`. Phase 0 lifts the file/empty helpers
  into `test-helpers.sh` and removes the shadow so all subsequent
  phases can rely on the canonical signature.
- `scripts/test-config.sh` is the canonical bash integration test
  (the largest file in the suite). Tests for path/template
  resolution belong here.
- `scripts/test-evals-structure.sh` enforces a 0.9 pass-rate floor on
  `benchmark.json` files; new skills must ship benchmarks meeting
  this gate.
- The pytest harness under `tests/tasks/` covers `invoke` tasks and
  is not directly relevant to this work.

**Cue-phrase contract (load-bearing):**

`skills/work/extract-work-items/SKILL.md:130-138` enumerates the prose
cue-phrases the analyser detects ("The system must…", "Users need
to…", "We need to implement…", user stories, etc.).
`skills/work/extract-work-items/SKILL.md:496-499` is the
anti-extraction rule: bare structural headings without actionable
content are skipped. Every gap paragraph in `analyse-design-gaps`
output must satisfy these rules.

## Desired End State

The plugin ships a working design-convergence chain:

```
inventory-design (current)  ─┐
                             ├──▶ analyse-design-gaps ──▶ extract-work-items
inventory-design (target)   ─┘                                  │
                                                                ▼
                                                        meta/work/* …
```

Verification of end state:

- `mise run test` passes (all bash and pytest suites green, eval
  benchmarks ≥ 0.9).
- `/accelerator:init` creates 14 directories including
  `meta/design-inventories/` and `meta/design-gaps/`, each with
  `.gitkeep`.
- `/accelerator:configure paths help` lists `design_inventories` and
  `design_gaps` with their defaults.
- `${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh design-inventory`
  resolves to the bundled template.
- `${CLAUDE_PLUGIN_ROOT}/scripts/config-read-template.sh design-gap`
  resolves to the bundled template.
- `${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh design_inventories
  meta/design-inventories` returns the configured-or-default path.
- `agents/browser-locator.md` and `agents/browser-analyser.md` exist
  with valid frontmatter declaring only explicit
  `mcp__playwright__browser_*` tool names (no wildcards) and no
  filesystem tools.
- `.claude-plugin/.mcp.json` declares the Playwright server with valid
  JSON.
- `/accelerator:inventory-design <source-id> <location>` runs
  end-to-end against a small test fixture (code-static crawler), with
  graceful degradation when Playwright is unavailable.
- `/accelerator:analyse-design-gaps <current-id> <target-id>` produces
  a `design-gap` artifact whose paragraphs are picked up by
  `extract-work-items` (manually verified).

### Key Discoveries:

- Locator/analyser tool allocation is contractual:
  `agents/codebase-locator.md:4` (Grep, Glob, LS only) vs.
  `agents/codebase-analyser.md:4` (Read, Grep, Glob, LS). The new
  `browser-locator` gets navigate + accessibility-tree-snapshot only;
  `browser-analyser` adds screenshot, evaluate, click, type, wait.
- The `{<role> agent}` token convention in
  `skills/research/research-codebase/SKILL.md:69-80` is resolved at
  invocation time by `config-read-agents.sh`. New skills must use the
  same tokens to inherit user-configured agent name overrides.
- `scripts/config-read-skill-instructions.sh` belongs at the **end** of
  every SKILL.md (`skills/research/research-codebase/SKILL.md:189`).
- `scripts/test-evals-structure.sh:14-15` rationale: the 0.9 floor (not
  1.0) accommodates a known historical case in clarity-lens evals.
  New benchmarks should target 1.0 but the gate at 0.9 is the
  enforcement bar.
- `scripts/test-evals-structure.sh` discovers `evals.json` by glob
  under `skills/`, requires sibling `benchmark.json`, and asserts
  every `evals[].id` appears in `benchmark.runs[]` with
  `configuration: "with_skill"`.
- The skill-creator skill (located at
  `~/.claude/plugins/marketplaces/claude-plugins-official/plugins/skill-creator/skills/skill-creator/SKILL.md`)
  bundles `scripts/run_eval.py` and `eval-viewer/generate_review.py`
  for the iterate loop.
- Template style varies: `templates/research.md` uses `[bracket]`
  placeholders; `templates/plan.md` uses `{brace}` placeholders.
  Newer templates (`work-item.md`, `pr-description.md`) favour the
  brace style — new design templates should use brace style.
- Agent frontmatter today never declares `mcp__*` tools; this work
  introduces that pattern.
- `.github/workflows/main.yml:30-31` runs `mise run test` — that
  single command is the CI contract. Anything that should block CI
  must hang off `test:unit` or `test:integration`.

## What We're NOT Doing

The following items are explicitly out of scope:

- **Restructuring `meta/research/` into subcategories** (research §9.8).
  A separate ADR is the right home for that change.
- **Multi-viewport inventories** (research §OQ #5). The first cut treats
  each viewport as a separate `source` (e.g. `prototype-desktop`,
  `prototype-mobile`).
- **Screenshot pruning** (research §OQ #4). Repo will accumulate
  screenshots; mitigation is a future enhancement.
- **Migrating from `.mcp.json` to inline `mcpServers` in `plugin.json`**
  (research §OQ #1). Blocked on Claude Code issue #16143.
- **Framework-specific token discovery profiles** (research §OQ #3).
  Generic codebase-analyser instructions ship in v1; profile graduation
  is deferred to the second or third real use case.
- **Updating `documents-locator` to surface inventory directories.** Out
  of scope; inventories are not currently consumed via that agent.
  This is a known consequence of the directory-per-artifact divergence
  from the flat-file convention used elsewhere in `meta/`. Tracked
  separately as a follow-up: when a downstream consumer needs to
  enumerate inventories via `documents-locator` (e.g. a future
  cross-design-inventory query skill), the locator will need to glob
  for `*/inventory.md` rather than `*.md` directly, and the
  `documents-analyser` instructions will need to know to read the
  directory-name date prefix as the canonical date. The README
  workflow section notes this limitation explicitly so users discover
  it without surprise.
- **Implementing a custom screenshot diff or visual regression tool.**
  Not needed — verification stays at structural-diff layer per
  research §1.
- **A new `tasks/` invoke module.** No new Python orchestration is
  needed; all skill orchestration is markdown-driven.

## Implementation Approach

Six phases, each independently mergeable. Each phase begins with
failing tests (TDD) and ends with `mise run test` green. Skills are
created via the `skill-creator:skill-creator` skill, which means each
skill phase is itself an iterative loop (write → eval → iterate)
rather than a single edit.

Phase ordering rationale:

0. **Test-helper reconciliation.** Lifts shared assertion helpers
   into `scripts/test-helpers.sh` and removes a shadowing
   `assert_contains` from `scripts/test-config.sh`. No functional
   change; pure refactor that unblocks the new test scripts in
   subsequent phases.
1. **Foundation first.** Templates and path-key wiring are pure
   infrastructure; they unblock both new skills without committing to
   any orchestration logic. Failures here are easy to diagnose because
   tests are unit-level shell-script tests.
2. **Browser agents next.** They are pure capability additions —
   self-contained, no skill yet depends on them. Their introduction
   establishes the `mcp__playwright__*` pattern in isolation, before a
   consuming skill complicates the diff.
3. **`inventory-design` skill.** Composes existing `codebase-*`
   agents plus the new `browser-*` agents. Lands first because it
   produces the artifacts the gap analyser consumes.
4. **`analyse-design-gaps` skill.** Consumes inventory artifacts.
   Lands second because end-to-end testing requires real inventories.
5. **Polish.** Final docs, CHANGELOG, version bump. Held to last so
   user-visible documentation reflects the shipped behaviour.

---

## Phase 0: Test-Helper Reconciliation

### Overview

The new test scripts in Phases 1-3 source only `scripts/test-helpers.sh`
and call `assert_file_exists`, `assert_contains`, and (in Phase 2) a
negative-assertion helper. Today:

- `assert_file_exists`, `assert_file_not_exists`, `assert_empty`, and
  `assert_file_content_eq` are defined **only** as local functions in
  `scripts/test-config.sh:50-92`. Any new script that sources only
  `test-helpers.sh` and calls them will exit immediately under
  `set -euo pipefail` with `command not found`.
- `scripts/test-config.sh:25-36` defines a local `assert_contains`
  with the **inverted** signature `(name, needle, haystack)`,
  shadowing `test-helpers.sh:106`'s canonical
  `(name, haystack, needle)`. The new calls in Phases 1-3 are
  written for the canonical signature; without this reconciliation,
  any new assertion appended to `test-config.sh` would silently pass
  or fail for the wrong reason.
- Phase 2's negative assertion (`browser-locator must NOT declare
  browser_take_screenshot`) is implemented inline with manual PASS/FAIL
  counter manipulation. A reusable `assert_not_contains` helper would
  remove that boilerplate and keep the counter coupling private to
  the helper.

This phase ships as a standalone, mergeable refactor before any new
feature work. It contains no functional change to the plugin — only
a tidier test-harness surface.

### Changes Required:

#### 1. Lift local helpers into `scripts/test-helpers.sh`

Move the following four function bodies verbatim from
`scripts/test-config.sh:38-92` into `scripts/test-helpers.sh`
(grouped with the existing assertions in alphabetical order):

- `assert_empty(test_name, actual)`
- `assert_file_exists(test_name, file_path)`
- `assert_file_not_exists(test_name, file_path)`
- `assert_file_content_eq(test_name, file_path, expected)`

Then **delete** the corresponding definitions from `test-config.sh`.

#### 2. Remove the shadowing `assert_contains` in `test-config.sh`

Delete the local `assert_contains` definition at
`scripts/test-config.sh:25-36`. The canonical signature
`(name, haystack, needle)` from `test-helpers.sh:106-117` becomes
the single, project-wide convention.

Audit the existing `test-config.sh` call sites for the inverted
order and flip any that relied on the shadow. (A grep pass plus a
green CI run is sufficient — the shadowing was the ambiguity, not
the call-site intent.)

#### 3. Add `assert_not_contains` to `scripts/test-helpers.sh`

```bash
assert_not_contains() {
  local test_name="$1" haystack="$2" needle="$3"
  if echo "$haystack" | grep -qF "$needle"; then
    echo "  FAIL: $test_name"
    echo "    Expected NOT to contain: $(printf '%q' "$needle")"
    echo "    Actual: $(printf '%q' "$haystack")"
    FAIL=$((FAIL + 1))
  else
    echo "  PASS: $test_name"
    PASS=$((PASS + 1))
  fi
}
```

Update the file header comment ("Exposes: …") to list the new
helpers (`assert_contains`, `assert_empty`, `assert_file_exists`,
`assert_file_not_exists`, `assert_file_content_eq`,
`assert_not_contains`).

#### 4. Update Phase 2 §6 negative assertion to use the helper

The Phase 2 §6 inline negative assertion for
`browser-locator must not declare browser_take_screenshot` becomes
a single `assert_not_contains` call rather than an inline
`if grep -q ... FAIL=$((FAIL + 1))` block. The same applies to
the `@latest` negative assertion in the `.mcp.json` test.

### Success Criteria:

#### Automated Verification:

- [x] `bash scripts/test-config.sh` continues to pass (no behavioural
      change; the lifted helpers are byte-identical and the shadow
      removal does not change outcomes for existing call sites that
      used the canonical order — any that did not are caught now).
- [x] `grep -n '^assert_' scripts/test-helpers.sh` shows the four
      lifted helpers plus `assert_not_contains`.
- [x] `grep -n '^assert_contains\|^assert_file\|^assert_empty' scripts/test-config.sh`
      returns no matches (definitions removed).
- [x] `mise run test` green.

#### Manual Verification:

- [x] Spot-check that no other `scripts/test-*.sh` script defines a
      shadowing assertion helper (search for
      `^assert_[a-z_]*\(\) \{`).

---

## Phase 1: Foundation — Templates and Path Wiring

### Overview

Add the two new templates, register the two new path keys in init and
configure, update README rows, ship the corresponding test
extensions, and commit the shared fixture used by Phase 3-4 evals
and manual verification. No skills, no agents, no MCP yet.

### Changes Required:

#### 1. New templates

**File**: `templates/design-inventory.md` (new)

Mirrors the section structure in research §3.1. Frontmatter uses
brace-style placeholders consistent with `templates/work-item.md`.
Body sections: Overview, Design System (Tokens, Layout primitives),
Component Catalogue, Screen Inventory, Feature Catalogue,
Information Architecture, Crawl Notes, References.

**File**: `templates/design-gap.md` (new)

Mirrors research §3.2. Frontmatter declares `current_inventory` and
`target_inventory` paths. Body sections: Overview, Token Drift,
Component Drift, Screen Drift, Net-New Features, Removed Features,
Suggested Sequencing, References. Section intros include explicit
guidance ("Each entry below is written as actionable prose…") so
authors maintain the cue-phrase contract.

#### 2. Init skill — register new path keys

**File**: `skills/config/init/SKILL.md`

Three coordinated edits:

```markdown
**Design inventories directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh design_inventories meta/design-inventories`
**Design gaps directory**: !`${CLAUDE_PLUGIN_ROOT}/scripts/config-read-path.sh design_gaps meta/design-gaps`
```

Insert these two lines into the existing path block adjacent to the
other content directories (e.g. near `notes`, **not** at the end of
the block) — `tmp` should remain the last entry because Step 2
special-cases it. The init SKILL.md block is not strictly
alphabetical but does group related keys, so co-locating the two
new design keys with the other content directories preserves the
grouping convention. At line 40,
reword the prose count from "the 12 directories resolved above"
to a count-free wording (e.g. "the directories resolved above")
and add an HTML-comment marker on its own line immediately
preceding the prose:

```html
<!-- DIR_COUNT:14 -->
```

The marker is what tests assert on (`assert_contains` against the
literal `<!-- DIR_COUNT:14 -->`), eliminating the
literal-prose-count maintenance liability flagged in the review.
Future directory additions update the marker number alongside the
path block; the prose itself is no longer count-coupled.

Add two corresponding `✓ {design inventories directory}` /
`✓ {design gaps directory}` lines to the summary block at
lines 105-117.

#### 3. Configure skill — paths help table

**File**: `skills/config/configure/SKILL.md`

Append two rows to the paths table at lines 386-399:

```markdown
| `design_inventories` | `meta/design-inventories` | Design-inventory artifacts (one directory per snapshot, with screenshots/) |
| `design_gaps`        | `meta/design-gaps`        | Design-gap analysis artifacts                                              |
```

#### 4. README — meta/ table and template keys list

**File**: `README.md`

Add two rows to the meta/ table at lines 77-88:

```markdown
| `design-inventories/` | Per-source design inventory snapshots (markdown + screenshots) | `inventory-design` |
| `design-gaps/`        | Design-gap analysis artifacts                                  | `analyse-design-gaps` |
```

Update the template keys list at lines 213-214 to include
`design-inventory` and `design-gap`.

#### 4a. Fixture — `examples/design-test-app/`

**Files**: `examples/design-test-app/` (new directory)

A minimal frontend fixture used by Phase 3 evals (eval #1
"code-only crawler against a tiny fixture repo") and Phase 3-4
manual verification scenarios. Commit it as a Phase 1 deliverable
so subsequent phases have a stable, version-controlled target;
both names that previously appeared in earlier plan drafts
(`./examples/test-app`, `examples/design-test-app/`) are
reconciled to **`examples/design-test-app/`** here.

Contents (deliberately tiny — under 200 LoC total):

- `examples/design-test-app/README.md`: scope statement
  ("Minimal frontend fixture for `inventory-design` evals and
  manual end-to-end testing. Deliberately small; do not grow it
  beyond the components/screens/tokens needed by the evals.")
- `examples/design-test-app/tailwind.config.js`: a 4-token colour
  palette (so token-drift evals have a known input)
- `examples/design-test-app/src/components/Button.jsx`: two
  variants (primary, secondary)
- `examples/design-test-app/src/components/Card.jsx`: one
  variant
- `examples/design-test-app/src/pages/Home.jsx`: uses Button
  and Card
- `examples/design-test-app/src/pages/Settings.jsx`: uses
  Button only
- `examples/design-test-app/package.json`: declares React only
  (no Tailwind install needed — the config file is what the
  inventory crawler reads)

Do not run `npm install` — the fixture's value is entirely
static. The `package.json` exists only so framework detection
in `inventory-design --crawler code` has something to read.

#### 5. Tests — add to `scripts/test-config.sh`

**File**: `scripts/test-config.sh` (extend)

Add new test blocks (TDD: write before the templates exist; tests
fail; then create the templates and tests pass):

```bash
echo "=== design templates: auto-discovery ==="
assert_eq "design-inventory listed by enumerator" \
  "$(config_enumerate_templates "$PLUGIN_ROOT" | grep -c '^design-inventory$')" "1"
assert_eq "design-gap listed by enumerator" \
  "$(config_enumerate_templates "$PLUGIN_ROOT" | grep -c '^design-gap$')" "1"

echo "=== design templates: resolver returns plugin default ==="
REPO=$(setup_repo)
RESOLVED=$("$SCRIPT_DIR/config-read-template.sh" design-inventory < /dev/null 2>/dev/null || true)
assert_contains "design-inventory resolves to plugin templates dir" \
  "$RESOLVED" "templates/design-inventory.md"
RESOLVED=$("$SCRIPT_DIR/config-read-template.sh" design-gap < /dev/null 2>/dev/null || true)
assert_contains "design-gap resolves to plugin templates dir" \
  "$RESOLVED" "templates/design-gap.md"

echo "=== design path keys: defaults work ==="
ACTUAL=$("$READ_VALUE" paths.design_inventories meta/design-inventories)
assert_eq "design_inventories default" "meta/design-inventories" "$ACTUAL"
ACTUAL=$("$READ_VALUE" paths.design_gaps meta/design-gaps)
assert_eq "design_gaps default" "meta/design-gaps" "$ACTUAL"
```

Add a structural test file at `scripts/test-design.sh` (new — see
"Test-script granularity" note below). The Phase 1 contributions
to that file go into a `=== Foundation ===` section:

```bash
#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/test-helpers.sh"

INIT="$PLUGIN_ROOT/skills/config/init/SKILL.md"
CONFIGURE="$PLUGIN_ROOT/skills/config/configure/SKILL.md"
README="$PLUGIN_ROOT/README.md"

echo "=== Foundation: init SKILL.md ==="
assert_contains "init lists design_inventories path key" \
  "$(cat "$INIT")" "design_inventories meta/design-inventories"
assert_contains "init lists design_gaps path key" \
  "$(cat "$INIT")" "design_gaps meta/design-gaps"
# Use a structured marker, not a literal count, so the assertion
# does not couple to the directory-count number.
# The init SKILL.md edit also adds:
#   <!-- DIR_COUNT:14 -->
# adjacent to the prose count, and the prose itself is reworded
# from "the 12 directories" to "the directories listed above"
# (eliminating the magic-number maintenance liability flagged in
# the review). The count marker is asserted instead.
assert_contains "init declares directory count via marker" \
  "$(cat "$INIT")" "<!-- DIR_COUNT:14 -->"
assert_contains "init summary lists design inventories directory" \
  "$(cat "$INIT")" "{design inventories directory}"
assert_contains "init summary lists design gaps directory" \
  "$(cat "$INIT")" "{design gaps directory}"

echo "=== Foundation: configure SKILL.md ==="
assert_contains "configure paths table includes design_inventories" \
  "$(cat "$CONFIGURE")" "design_inventories"
assert_contains "configure paths table includes design_gaps" \
  "$(cat "$CONFIGURE")" "design_gaps"

echo "=== Foundation: README ==="
assert_contains "README meta/ table lists design-inventories/" \
  "$(cat "$README")" "design-inventories/"
assert_contains "README meta/ table lists design-gaps/" \
  "$(cat "$README")" "design-gaps/"
assert_contains "README template keys include design-inventory" \
  "$(cat "$README")" "design-inventory"
assert_contains "README template keys include design-gap" \
  "$(cat "$README")" "design-gap"

# Subsequent phases append further sections to this same file
# (=== Browser agents ===, === MCP manifest ===, === Skills ===, etc.).
# `test_summary` runs once at the end of the file in Phase 4.
```

**Test-script granularity (decided)**: a single
`scripts/test-design.sh` with `=== Section ===` headers per
phase, rather than three separate files (`test-design-foundation.sh`,
`test-design-agents.sh`, `test-design-skills.sh`). One file
shares fixture/helper plumbing once, and every subsequent phase
appends its section to the same file. The `=== ... ===` separators
are the existing convention in `scripts/test-config.sh` and provide
adequate visual partitioning.

**CI wiring (decided — no new mise task)**: `tasks/test/helpers.py`
exposes `run_shell_suites(context, subtree)`, which globs all
executable `test-*.sh` files under the given subtree. The existing
`test:integration:config` task already runs
`run_shell_suites(context, "scripts")` (`tasks/test/integration.py:21-24`),
so dropping `scripts/test-design.sh` into place — with the executable
bit set — picks it up automatically as part of `mise run test`. No
edit to `mise.toml` or `tasks/test/integration.py` is required.

The Phase 1 success criteria assert this wiring works: a
`mise tasks ls | grep test:integration:config` confirmation plus a
green `mise run test` is the gate.

### Success Criteria:

#### Automated Verification:

- [x] Templates discoverable: `bash scripts/test-config.sh` includes
      and passes the new design-template assertions.
- [x] Path keys resolvable: `scripts/config-read-path.sh
      design_inventories meta/design-inventories` returns
      `meta/design-inventories`.
- [x] Foundation structural test passes (Foundation section of
      `bash scripts/test-design.sh` — see Phase 1 §5 consolidation
      decision).
- [x] CI gate passes: `mise run test` green (the new test-design.sh
      is auto-discovered by `test:integration:config` via
      `run_shell_suites`).

#### Manual Verification:

- [ ] In a fresh checkout, `/accelerator:init` creates both new
      directories with `.gitkeep`.
- [ ] `/accelerator:configure paths help` shows the two new rows.
- [ ] `/accelerator:configure show-template design-inventory` prints
      the bundled template body.

---

## Phase 2: Browser Agents and Playwright MCP

### Overview

Introduce the two new agents and the `.claude-plugin/.mcp.json`
manifest declaring the Playwright server. No skill consumes them yet;
the goal is to land the new patterns in isolation.

### Changes Required:

#### 1. `.claude-plugin/.mcp.json`

**File**: `.claude-plugin/.mcp.json` (new)

```json
{
  "mcpServers": {
    "playwright": {
      "command": "npx",
      "args": ["@playwright/mcp@<PINNED-SEMVER>"]
    }
  }
}
```

**Pin discipline**: Replace `<PINNED-SEMVER>` with a specific published
version of `@playwright/mcp` at implementation time (e.g. `0.0.41`). Do
not use `latest` — every `npx` resolution would otherwise pick up a
potentially-breaking or potentially-malicious release. The pinned
version becomes part of the plugin's transitive dependency contract;
upgrade requires an explicit PR.

**Filename**: the leading-dot form `.mcp.json` is required by Claude
Code's MCP manifest discovery (sibling to `plugin.json` per the
official MCP docs and research §6.2). The dotted form deviates from
`plugin.json`'s naming on the same level, but the deviation is
upstream-mandated rather than a project choice. Add a one-line
comment in the README MCP install paragraph noting the asymmetry so
contributors do not "fix" it.

#### 2. `agents/browser-locator.md`

**File**: `agents/browser-locator.md` (new)

Mirrors the canonical locator structure in
`agents/codebase-locator.md` (frontmatter, lead-in, Core
Responsibilities, Search Strategy, Output Format, Important
Guidelines, What NOT to Do, Remember closer).

```yaml
---
name: browser-locator
description: Locates routes, screens, and DOM-level component
  presence in a running web application via the Playwright MCP
  server. Call browser-locator when you need to enumerate WHERE
  things appear in the rendered UI, not to extract their detail.
tools: mcp__playwright__browser_navigate, mcp__playwright__browser_snapshot
---
```

Body explicitly forbids screenshots, evaluate, click/type, and any
file-system access. Output Format includes `### Routes`,
`### Components on each screen`, `### State indicators` sections.

#### 3. `agents/browser-analyser.md`

**File**: `agents/browser-analyser.md` (new)

Mirrors `agents/codebase-analyser.md`.

```yaml
---
name: browser-analyser
description: Analyses a focused set of screens in a running web
  application via the Playwright MCP server. Captures detailed
  state, screenshots, and computed values. Call browser-analyser
  when you need to extract HOW a screen behaves, not to
  enumerate WHERE things are.
tools: mcp__playwright__browser_navigate, mcp__playwright__browser_snapshot, mcp__playwright__browser_take_screenshot, mcp__playwright__browser_evaluate, mcp__playwright__browser_click, mcp__playwright__browser_type, mcp__playwright__browser_wait_for
---
```

Body Output Format mandates per-screen blocks with state matrix
(loading | empty | error | success), interaction outcomes, and
screenshot output paths. The "What NOT to Do" list explicitly
forbids reading source files.

**`browser_evaluate` payload allowlist** (security-critical — must
be in the agent body, not just in the consuming skill): the agent
may invoke `browser_evaluate` only with read-only DOM/style
inspection payloads. Permitted:

- `getComputedStyle(element).<property>` reads
- `element.getBoundingClientRect()` and other geometry reads
- Read-only property/attribute reads (`element.value` on
  non-password fields, `element.dataset.*`, `element.tagName`,
  `element.children.length`, etc.)
- Aggregate read-only walks of `document.querySelectorAll(...)`
  results that return primitive/serialisable values

Explicitly **forbidden** payloads (the agent body lists each with a
brief rationale and the "What NOT to Do" section repeats them):

- `fetch(...)`, `XMLHttpRequest`, `WebSocket`, `navigator.sendBeacon`,
  or any other network egress (exfiltration vector)
- `document.cookie` reads/writes (credential exfiltration)
- `localStorage`, `sessionStorage`, `indexedDB` reads/writes
  (credential / PII surface)
- Reads of `[type=password]` or `[autocomplete*=token]` element
  `.value` (would defeat the screenshot mask)
- Any DOM mutation (`appendChild`, `innerHTML =`, `click()`,
  `dispatchEvent`, `setAttribute`, `remove()`, etc. — the
  analyser uses `browser_click` / `browser_type` for intentional
  mutation)
- `eval`, `Function(...)`, dynamic `import(...)`, `new Worker(...)`
- `window.open`, `location =`, `history.pushState` (navigation
  must go through `browser_navigate` so the origin allowlist for
  the auth header applies)

The `What NOT to Do` section closes with: "Treat `browser_evaluate`
as a query language for the rendered page, not a programming
environment. If you cannot express what you need to know as a
read-only expression returning a JSON-serialisable value, do not
use `browser_evaluate`."

#### 4. README — agents table

**File**: `README.md` (lines 466-475)

Append two rows:

```markdown
| **browser-locator**         | Locates routes/screens/components in a running app via Playwright MCP    | mcp__playwright__browser_navigate, mcp__playwright__browser_snapshot                                                                                                                                                              |
| **browser-analyser**        | Analyses screens, captures state and screenshots via Playwright MCP     | mcp__playwright__browser_navigate, mcp__playwright__browser_snapshot, mcp__playwright__browser_take_screenshot, mcp__playwright__browser_evaluate, mcp__playwright__browser_click, mcp__playwright__browser_type, mcp__playwright__browser_wait_for |
```

Add a new short paragraph after the agents table noting that
`browser-*` agents require the Playwright MCP server and that
`mise run deps:install:playwright` (existing task) handles the
browser-binary install.

#### 5. README — install section MCP note

**File**: `README.md` (under `## Installation`, before the
`### Prerelease Versions` subsection)

One-paragraph addition explaining that the plugin declares a
Playwright MCP dependency in `.claude-plugin/.mcp.json` and that
Claude Code will prompt to enable it on first use of any skill that
needs it. Note the graceful-degradation contract:
`inventory-design --crawler code` works without the MCP.

Immediately following, add an `### Authenticated browser crawls`
subsection documenting the four env vars used by `inventory-design`
when crawling auth-walled prototypes. Mirror the format of the
existing `ACCELERATOR_VISUALISER_*` documentation (CHANGELOG Notes
section in v1.20.0):

```markdown
### Authenticated browser crawls

`/accelerator:inventory-design` reads the following environment
variables when the location is a hosted prototype or running app and
authentication is required. They are also read by any future skill
that uses the `browser-*` agents.

| Variable                          | Purpose                                                          |
|-----------------------------------|------------------------------------------------------------------|
| `ACCELERATOR_BROWSER_AUTH_HEADER` | Header injected on navigations to the resolved location's origin |
| `ACCELERATOR_BROWSER_USERNAME`    | Form-login username (with `_PASSWORD` and `_LOGIN_URL`)          |
| `ACCELERATOR_BROWSER_PASSWORD`    | Form-login password                                              |
| `ACCELERATOR_BROWSER_LOGIN_URL`   | Login form URL                                                   |

Precedence: if `AUTH_HEADER` is set it takes precedence and the
form-login vars are ignored (with a warning). If `AUTH_HEADER` is
unset, all three of `USERNAME`, `PASSWORD`, and `LOGIN_URL` must be
set together — partial sets cause the skill to fail with a clear
error. With none set, auth-walled routes are skipped and noted in
the inventory's Crawl Notes.

Security: `AUTH_HEADER` is sent **only** on navigations whose origin
matches the resolved location (or the login URL); cross-origin
navigations strip it. Screenshots mask password and token fields.
The skill refuses to write an inventory if any env-var literal
appears in the generated body.
```

Also add a brief "Security considerations" subsection to the same
Installation area covering: (a) the env vars and where they go,
(b) the screenshot durability concern (committed to the repo),
(c) the recommendation not to point `inventory-design` at
production systems with side-effecting forms (the analyser has
`browser_click` / `browser_type`), and (d) the supply-chain pin
discipline for `@playwright/mcp`.

#### 6. Tests — agent and MCP structural validation

**File**: `scripts/test-design.sh` (extend the file created in
Phase 1 by appending these sections before the final
`test_summary` call). Use a YAML-aware extractor for the
`tools:` field so the assertions don't silently miss tools when
the agent frontmatter wraps onto continuation lines.

```bash
echo "=== Browser agents ==="
LOC="$PLUGIN_ROOT/agents/browser-locator.md"
ANA="$PLUGIN_ROOT/agents/browser-analyser.md"

assert_file_exists "browser-locator.md exists" "$LOC"
assert_file_exists "browser-analyser.md exists" "$ANA"

# YAML-aware extractor: parses the frontmatter block and prints
# the tools field as a sorted, comma-joined string. Robust against
# wrapped values, list-style YAML, and reordering.
extract_tools() {
  python3 -c '
import sys, yaml
content = open(sys.argv[1]).read()
parts = content.split("---", 2)
fm = yaml.safe_load(parts[1]) if len(parts) >= 3 else {}
tools = fm.get("tools", "")
if isinstance(tools, list):
    items = [t.strip() for t in tools]
else:
    items = [t.strip() for t in str(tools).split(",")]
print(",".join(sorted(t for t in items if t)))
' "$1"
}

LOC_TOOLS="$(extract_tools "$LOC")"
ANA_TOOLS="$(extract_tools "$ANA")"

assert_eq "browser-locator declares exactly navigate+snapshot" \
  "mcp__playwright__browser_navigate,mcp__playwright__browser_snapshot" \
  "$LOC_TOOLS"
assert_not_contains "browser-locator does not declare browser_take_screenshot" \
  "$LOC_TOOLS" "browser_take_screenshot"

EXPECTED_ANA_TOOLS="mcp__playwright__browser_click,mcp__playwright__browser_evaluate,mcp__playwright__browser_navigate,mcp__playwright__browser_snapshot,mcp__playwright__browser_take_screenshot,mcp__playwright__browser_type,mcp__playwright__browser_wait_for"
assert_eq "browser-analyser declares exactly the seven Playwright tools" \
  "$EXPECTED_ANA_TOOLS" "$ANA_TOOLS"

echo "=== browser_evaluate payload allowlist ==="
ANA_BODY="$(cat "$ANA")"
# Forbidden patterns must be named in the agent body so the model
# is instructed to avoid them.
for forbidden in "fetch" "XMLHttpRequest" "document.cookie" \
                 "localStorage" "sessionStorage" "indexedDB" \
                 "eval" "innerHTML" "window.open"; do
  assert_contains "browser-analyser body forbids $forbidden in browser_evaluate" \
    "$ANA_BODY" "$forbidden"
done

echo "=== .mcp.json ==="
MCP="$PLUGIN_ROOT/.claude-plugin/.mcp.json"
assert_file_exists ".mcp.json exists" "$MCP"
assert_eq "mcp.json declares playwright server" \
  "$(jq -r '.mcpServers.playwright.command' "$MCP")" "npx"
PLAYWRIGHT_ARG="$(jq -r '.mcpServers.playwright.args[0]' "$MCP")"
assert_contains "mcp.json playwright args pins @playwright/mcp" \
  "$PLAYWRIGHT_ARG" "@playwright/mcp@"
assert_not_contains ".mcp.json pins @playwright/mcp to a specific version (not @latest)" \
  "$PLAYWRIGHT_ARG" "@latest"
assert_eq "mcp.json is valid JSON" "$(jq empty "$MCP" 2>&1)" ""
```

### Success Criteria:

#### Automated Verification:

- [ ] Bash agent/MCP tests pass (Browser agents and MCP manifest
      sections of `bash scripts/test-design.sh`).
- [ ] `.claude-plugin/.mcp.json` is valid JSON: `jq empty
      .claude-plugin/.mcp.json`.
- [ ] CI gate passes: `mise run test` green.

#### Manual Verification:

- [ ] In a fresh Claude Code session opened against this repo,
      Claude Code prompts to enable the Playwright MCP server on
      first use.
- [ ] After enabling, `mcp__playwright__browser_navigate` and
      `mcp__playwright__browser_snapshot` appear in the available
      tool set.
- [ ] Spawning the `browser-locator` agent against a public URL
      returns a structured route enumeration (e.g. against
      `https://example.com` it correctly reports a single page).

---

## Phase 3: `inventory-design` Skill via skill-creator

### Overview

Create `skills/design/inventory-design/SKILL.md` and its bundled
`evals/`, `scripts/`, and supporting files. Run the skill-creator
loop: capture intent → write → evaluate → iterate. The skill produces
one `design-inventory` directory per source.

### Changes Required:

#### 1. Skill scaffolding (use `skill-creator:skill-creator`)

Invoke the skill-creator skill explicitly: it walks through capture
intent, write SKILL.md, write evals, run benchmarks, and iterate.
Pass it the research document and this plan as context. The output is:

**Files**:

- `skills/design/inventory-design/SKILL.md`
- `skills/design/inventory-design/scripts/inventory-metadata.sh`
  (generates frontmatter timestamps and `git_commit`/`branch`,
  mirroring patterns in
  `skills/research/research-codebase/scripts/`)
- `skills/design/inventory-design/scripts/resolve-auth.sh`
  (validates the `ACCELERATOR_BROWSER_*` env-var precedence rules
  from §2 and emits a single canonical mode: `header`, `form`,
  `none`, or exits non-zero with an error pointing at the
  missing/conflicting var)
- `skills/design/inventory-design/scripts/scrub-secrets.sh`
  (pre-write scrubber: greps generated body for env-var literals,
  exits non-zero if any are found)
- `skills/design/inventory-design/evals/evals.json`
- `skills/design/inventory-design/evals/benchmark.json`

**Directory naming**: artifacts land at
`<paths.design_inventories>/YYYY-MM-DD-HHMMSS-{source-id}/inventory.md`.
The `HHMMSS` suffix disambiguates same-day re-runs, removes the
race condition between supersede mutation and new-artifact write
(directory names cannot collide), and makes the directory date
prefix a useful secondary tiebreaker for the resolver
(see Phase 4 step 1).

**Supersede protocol** (load-bearing — specify explicitly in the
SKILL.md, do not leave to model judgement):

1. **Compute the next sequence number**: scan all
   `*-{source-id}/inventory.md` files under the inventory root,
   read each frontmatter `sequence` field, take `max + 1` (start
   at 1 if none exist). The new inventory's frontmatter records
   this sequence number alongside the existing date/timestamp
   fields. **Sequence is the resolver's primary tiebreaker** — see
   below — because `YYYY-MM-DD-HHMMSS` directory prefixes are
   unsafe under NTP correction backwards, restoring from backup,
   or manually renaming a directory.
2. **Write the new directory** (atomically: build under a sibling
   temporary directory `.YYYY-MM-DD-HHMMSS-{source-id}.tmp/`
   containing `inventory.md` plus `screenshots/`, then `mv` to the
   final name). Atomic-rename guarantees Phase 4's resolver never
   sees a half-written directory. Both the directory glob and the
   resolver explicitly skip leading-dot directories so an in-progress
   `.tmp/` is invisible to readers.
3. **Then mutate prior inventories' frontmatter**: glob
   `*-{source-id}/` under the inventory root (excluding leading-dot
   names), exclude the just-written directory, set
   `status: superseded` on each `inventory.md` whose status is
   currently `draft` or `accepted` (idempotent for already-superseded
   files). If the mutation step fails partway through, the new
   directory is already authoritative; the resolver's primary
   tiebreaker (highest `sequence` number) selects it correctly.
4. **Never delete prior screenshots** — the superseded directory
   remains a complete point-in-time record per research §8.

**Frontmatter additions**: the inventory frontmatter (template
`templates/design-inventory.md`) adds two new fields:

```yaml
sequence: 3                    # monotonic per-source-id counter; resolver primary tiebreaker
screenshots_incomplete: false  # true when screenshot byte budget exhausted before crawl complete
```

`screenshots_incomplete` is set to `true` (in addition to the
existing `status: incomplete` for page-cap and wall-clock-timeout
exhaustion) when the screenshot byte-budget bound from §2b fires.
This gives downstream consumers (gap analysis, manual review) a
machine-detectable signal that visual capture is partial even when
text capture is complete.

The SKILL.md body follows the research-codebase structural template
(`skills/research/research-codebase/SKILL.md`):

- Frontmatter: `name`, `description` (multi-line, "pushy" per
  skill-creator guidance — model the tone on
  `skills/work/extract-work-items/SKILL.md`'s description for the
  cue-laden discovery shape, not on shorter skills like `init`),
  `argument-hint: "[source-id] [location] [--crawler code|runtime|hybrid] (default: hybrid for code-repo, runtime otherwise)"`
  (matching the `extract-work-items` precedent of `[]` brackets for
  both required and optional positionals — the conditional default
  is spelled out inline so the user does not have to read the body
  to learn it; required-vs-optional distinction is conveyed by the
  description and Crawler Modes section, not by bracket style. This
  is consistent with every existing skill in the plugin),
  `disable-model-invocation: true`,
  `allowed-tools` covering
  `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)`,
  `Bash(${CLAUDE_PLUGIN_ROOT}/skills/design/inventory-design/scripts/*)`,
  and the **explicit** Playwright MCP tool names:
  `mcp__playwright__browser_navigate`,
  `mcp__playwright__browser_snapshot`,
  `mcp__playwright__browser_take_screenshot`,
  `mcp__playwright__browser_evaluate`,
  `mcp__playwright__browser_click`,
  `mcp__playwright__browser_type`,
  `mcp__playwright__browser_wait_for`. The list matches the union
  of the two `browser-*` agents' frontmatter exactly. **No wildcards**
  (no `mcp__playwright__*` shorthand) — the explicit form prevents
  privilege creep when the upstream Playwright MCP server adds new
  tools, and it keeps the audit surface identical between agent
  frontmatter and skill `allowed-tools`. This is precedent-setting
  for future MCP-using skills in the plugin.
- The three context-loading `!` bash blocks at the top
  (`config-read-context.sh`, `config-read-skill-context.sh
  inventory-design`, `config-read-agents.sh`).
- The "Agent Names" defaults section (extending the canonical list
  to include `accelerator:browser-locator` and
  `accelerator:browser-analyser`).
- Body sections: Crawler Modes (user-facing reference: `code`
  means static analysis only, no MCP needed; `runtime` means
  Playwright-MCP browser inspection only; `hybrid` means
  both, with code-static as ground truth for tokens/components
  and runtime filling in screen states; default is `hybrid`
  for code-repo sources when the MCP is enabled, otherwise
  `runtime` for prototype/running-app URLs, otherwise `code`),
  Resolve Source, Choose Crawl Strategy, Spawn Agents in
  Parallel, Synthesise, Generate Metadata, Write Artifact,
  Present Summary.
- Final `!`config-read-skill-instructions.sh inventory-design`` line.

#### 2a. Source validation (security-critical)

The Resolve Source step validates the `[source-id]` and `[location]`
arguments before any agent is spawned:

**`source-id` format**: must match `^[a-z0-9][a-z0-9-]*$`
(kebab-case, lowercase, no leading hyphen, no spaces). Reject
otherwise with a clear error naming the offending characters.

**`location` URL/path validation** (only when `source_kind` is
`prototype` or `running-app`):

- **Scheme allowlist**: only `https://` accepted by default. `http://`
  is rejected unless the skill is invoked with an explicit
  `--allow-insecure` flag (not in v1; flag is reserved for future
  use). Schemes `file://`, `javascript:`, `data:`, `chrome://`,
  `about:` (other than `about:blank` for internal use) are rejected
  with a hard error.
- **Host allowlist**: hosts that resolve to RFC1918
  (10/8, 172.16/12, 192.168/16), loopback (127/8, ::1),
  link-local (169.254/16, fe80::/10), or `localhost` are rejected
  unless the skill is invoked with an explicit `--allow-internal`
  flag (not in v1). This prevents accidental SSRF reaching cloud
  metadata services or internal admin endpoints, particularly in
  CI contexts.
- **Path locations** (`source_kind: code-repo`): resolved relative
  to the project root and required to remain inside it (no `..`
  escapes). The path must exist and be a directory.

These validations are performed by a small
`skills/design/inventory-design/scripts/validate-source.sh`
script (alongside `resolve-auth.sh` and `scrub-secrets.sh`) so the
contract is testable in isolation and reusable by future browser
skills.

#### 2b. Crawl bounds (DoS / runaway protection)

The skill enforces three bounds on every browser-driven crawl. Each
bound is a documented default with a SKILL.md-level note that v1
does not expose these as user-tunable flags — values are pinned in
the skill body for now and graduate to flags only if a real use
case demands.

- **Page cap**: at most 50 distinct routes navigated per crawl.
  When the cap is reached, the crawl stops, the partial inventory
  is written with frontmatter `status: incomplete`, and the
  `Crawl Notes` section explicitly records that the cap fired and
  lists the routes that were not reached.
- **Wall-clock timeout**: 5 minutes total per crawl. Same handling
  on hit (write `status: incomplete`, note in `Crawl Notes`).
- **Screenshot byte budget**: 50 MB total per crawl. When the
  budget is exhausted, subsequent screenshots are skipped (still
  recorded in `Crawl Notes` so the user knows which screens have
  no visual capture) and the crawl continues until one of the
  other bounds fires.

Rationale: a misconfigured `[location]` pointing at a tarpit page,
infinite-link site, or pathologically large prototype must not
silently consume the user's quota or balloon the repo via the
deferred screenshot-pruning issue. Bounds also protect against
adversarial `[location]` inputs (defence in depth alongside the
URL validation in §2a).

#### 2. Auth handling (per user direction)

The Resolve Source step instructs the skill to read these env vars
when location is a `running-app`/`prototype` URL and authentication
is required:

- `ACCELERATOR_BROWSER_USERNAME`, `ACCELERATOR_BROWSER_PASSWORD`,
  `ACCELERATOR_BROWSER_LOGIN_URL`, `ACCELERATOR_BROWSER_AUTH_HEADER`.

(Namespace is `BROWSER_*`, not `DESIGN_*`, because future browser-using
skills — perf audits, a11y audits, e2e exploration — will need the
same auth contract; the env vars belong to the `browser-*` agent
capability, not to a single consuming skill.)

**Precedence and validation** (load-bearing — must be specified
explicitly in the skill body, not left to model judgement):

1. If `ACCELERATOR_BROWSER_AUTH_HEADER` is set, header injection wins
   (form-login env vars, if also set, are ignored — the skill emits a
   one-line warning naming the env vars it ignored).
2. If `AUTH_HEADER` is unset and **all three** of `USERNAME`,
   `PASSWORD`, `LOGIN_URL` are set, perform form login.
3. If `AUTH_HEADER` is unset and any (but not all) of
   `USERNAME`/`PASSWORD`/`LOGIN_URL` are set, the skill **fails fast**
   with a clear message naming the missing var. Do not attempt a
   partial login.
4. If no auth env vars are set, auth-walled areas are skipped and
   noted in `Crawl Notes` — never fabricated. The skill also emits
   a console message at skip time naming the env vars that would
   have unlocked the route, so users learn about the contract from
   normal use rather than from reading the SKILL.md source. Example:

   > `inventory-design: skipped <url> (appears auth-walled).
   > Set ACCELERATOR_BROWSER_AUTH_HEADER, or
   > ACCELERATOR_BROWSER_USERNAME / _PASSWORD / _LOGIN_URL,
   > to crawl authenticated routes.`

**Origin allowlist for `AUTH_HEADER`** (security-critical): the auth
header is sent **only** on navigations whose origin
(scheme+host+port) matches the resolved `[location]` origin or the
`ACCELERATOR_BROWSER_LOGIN_URL` origin. On any cross-origin
navigation (off-site link, OAuth redirect, attacker-controlled
target reached via crawl), the analyser strips the header before the
request is issued. Document this constraint explicitly in the skill
body and assert it via eval (Phase 3 §4 eval #6).

**Secret-handling protections** (security-critical, all enforced by
the skill body and verified by evals):

- **Screenshot masking**: when calling `mcp__playwright__browser_take_screenshot`,
  pass the Playwright `mask` option for any `[type=password]`,
  `[autocomplete*=token]`, and `[data-secret]` selectors so the
  rendered pixels do not contain credential characters.
- **URL scrubbing**: any URL written into `inventory.md` (e.g. in
  Screen Inventory entries) has its query string stripped. Document
  the policy in `Crawl Notes` so the user knows reduction occurred.
- **Pre-write secret scrubber**: before writing `inventory.md`, the
  skill greps the generated body for the literal values of all set
  `ACCELERATOR_BROWSER_*` env vars. If any literal appears, the skill
  refuses to write the artifact and prints a clear error naming the
  env var (without printing its value). This is a belt-and-braces
  check; the agent is also instructed not to render env-var values
  into any output.
- **LLM-context isolation**: the skill reads auth env vars only
  inside the bash wrapper that drives `browser_type` (and only at the
  moment of typing); the LLM is given an opaque "perform login"
  instruction and never sees the raw values. This narrows the
  prompt-injection exfiltration surface to zero for the credentials
  themselves (the header value, when in use, is unavoidably handed to
  the navigator — see origin allowlist above).

#### 3. Graceful degradation (per user direction)

The Choose Crawl Strategy step:

- `--crawler code`: spawns `codebase-*` agents only. Does not require
  the Playwright MCP.
- `--crawler runtime`: spawns `browser-*` agents only. If the
  Playwright MCP is unavailable in the session, the skill prints a
  clear error message (per the failure-mode taxonomy below) and
  exits cleanly without writing a partial artifact.
- `--crawler hybrid` (default for code-repo sources when MCP
  available, else code-only with a note): code-static is ground
  truth for tokens/components; runtime fills in screen states.

**MCP-availability detection contract** (load-bearing — specify
explicitly in the SKILL.md, do not fall back to side-effecting
probes):

The skill distinguishes three failure modes and emits a different
user-visible message for each. Detection happens during Choose
Crawl Strategy, before any agent is spawned:

1. **MCP server not declared / not enabled in the session**
   (deterministic, safe to fall back). Detected by checking whether
   the literal tool name `mcp__playwright__browser_navigate` appears
   in the session's available tool set — i.e. the model checks its
   own toolbox without invoking anything. If absent under
   `--crawler runtime`, exit non-zero with:

   > `error: Playwright MCP server not enabled in this session.
   > Run `claude mcp` to enable it, or accept the prompt the next
   > time you start Claude Code in this repo.`

   Under `--crawler hybrid`, fall back to `code` and record the
   downgrade in `Crawl Notes` (see "auto-fallback notice" below).

2. **MCP server present but `browser_navigate` errors at first call**
   (likely missing browser binaries from `npx` cache, or a
   first-launch crash). The skill catches the error from the first
   real navigation attempt (not a probe) and re-emits it with a
   diagnostic suffix:

   > `error: Playwright MCP server is enabled but browser_navigate
   > failed: <upstream error>. If this is the first run on this
   > machine, the browser binaries may be missing — run
   > `mise run deps:install:playwright`.`

   Under both `--crawler runtime` and `--crawler hybrid`, this
   surfaces as a hard failure (no silent downgrade — the user
   asked for runtime data and the server is configured; a binary
   problem deserves a fix, not a silent code-only fallback).

3. **MCP server works at start of crawl but a navigation fails
   mid-crawl** (transient: site down, redirect loop, timeout). Per-
   navigation errors are recorded against the affected screen in
   `Crawl Notes` and the crawl continues with the remaining
   screens. The crawl does not downgrade to `code-only` mid-flight.

**No `about:blank` probe.** The previous specification used an
`about:blank` navigation to detect availability; that conflated all
three failure modes and is removed.

**Auto-fallback notice surfacing**: when `--crawler hybrid` falls
back to `code` because of failure mode 1, the skill prints the
notice **before** the crawl starts (not only as a post-hoc Crawl
Note), giving the user the option to abort and enable the MCP if
they want runtime data.

#### 4. Skill-creator evals

`evals/evals.json` covers at minimum:

1. Code-only crawler against a tiny fixture repo with a few
   components and a `tailwind.config.js` produces a valid inventory
   (frontmatter parses; Component Catalogue contains expected
   names; Token table contains expected colours).
2. Source-id supersedes prior inventory: running the skill twice
   with the same source-id (different `HHMMSS` timestamps) leaves
   the older directory in place with frontmatter `status: superseded`
   and the new directory authoritative.
3. Hybrid crawler with MCP unavailable falls back to code-only and
   notes the degradation in `Crawl Notes`.
4. Auth-walled prototype URL with no env vars: skill skips that
   route and lists it in Crawl Notes; does not fabricate.
5. Frontmatter `source` field matches the user-supplied source-id.
6. **Cross-origin auth-header strip**: with
   `ACCELERATOR_BROWSER_AUTH_HEADER` set, navigating to a page that
   redirects to a different host results in the header being
   stripped before the cross-origin request (assert via
   request-log inspection in the Playwright MCP eval harness, or
   by routing the redirect target to a local fixture that asserts
   the absence of the header).
7. **Pre-write secret scrubber**: with
   `ACCELERATOR_BROWSER_PASSWORD=hunter2_uniq` set, an attempt to
   inject the literal `hunter2_uniq` into the inventory body (via
   prompt-injection in a fixture page's text) causes the skill to
   refuse to write the artifact and exit non-zero with a clear
   error.
8. **Same-day re-run**: two runs of the same source-id within the
   same minute produce two distinct directories (different
   `HHMMSS`) — no clobber, no error.
9. **Partial-failure recovery**: if a prior run wrote the new
   directory but never executed the supersede mutation (simulated
   by hand-creating two `status: draft` directories for the same
   source-id), Phase 4's resolver still selects the newer one and
   emits the documented warning.
10. **Auth precedence**: with both `AUTH_HEADER` and
    `USERNAME`/`PASSWORD`/`LOGIN_URL` set, header injection wins
    and the skill emits a one-line warning naming the ignored
    form-login env vars.
11. **Partial credentials fail fast**: with `USERNAME` and
    `PASSWORD` set but `LOGIN_URL` unset, the skill exits non-zero
    naming the missing var rather than attempting a partial login.

**Run-count tiers** (overrides the global "at least 3 times"
default for this skill's benchmark.json):

- Probabilistic evals — those exercising MCP behaviour, network
  navigation, prompt-injection, model-generated prose, or any path
  whose pass/fail depends on a third-party server response — run
  **at least 5 times**. Specifically: evals #1 (code crawl + LLM
  synthesis), #2 (supersede LLM behaviour), #3 (MCP fallback), #4
  (auth-walled fabrication risk), #6 (cross-origin Playwright
  request log), #7 (prompt-injection scrubber), #15 (page-cap LLM
  decisions).
- Deterministic structural/validation evals (purely script-driven
  exits, no LLM stochasticity) may stay at 3 runs. Specifically:
  evals #5, #8, #10, #11, #12, #13.
- Structural-only assertion evals (e.g. eval #14 SKILL.md grep)
  may stay at 1 run.

Document the tiering at the top of `benchmark.json` so future
contributors don't blanket-set runs to 3.

12. **URL scheme rejection**: `inventory-design test-fs file:///etc/passwd`
    exits non-zero before any agent is spawned, with an error
    naming the rejected scheme. No artifact directory is created.
13. **Internal-host rejection**: `inventory-design test-internal http://127.0.0.1:8080`
    and `... http://169.254.169.254/` exit non-zero with an error
    referencing the host allowlist. No artifact directory is
    created.
14. **`browser_evaluate` payload safety**: the skill instructs the
    analyser to avoid forbidden payloads; assert (via the
    structural test, not a probabilistic eval) that the
    `browser-analyser` SKILL/agent body lists each forbidden
    pattern by name (`fetch`, `document.cookie`, `localStorage`,
    `eval`, etc.). A lighter-touch eval can additionally observe
    the analyser's tool-call log on a fixture page and assert no
    `evaluate` payload contains any forbidden token.
15. **Page-cap bound**: a fixture site with 60+ routes triggers the
    50-page cap; the skill writes an inventory with frontmatter
    `status: incomplete` and `Crawl Notes` listing the unreached
    routes.

`benchmark.json` runs each eval at least 3 times to measure
variance; the with_skill mean pass_rate must hit ≥ 0.9.

#### 5. Tests — skill structure and contract

**File**: `scripts/test-design.sh` (extend the same file with new
sections for the inventory-design skill)

```bash
echo "=== inventory-design: skill structure ==="
SKILL="$PLUGIN_ROOT/skills/design/inventory-design/SKILL.md"
assert_file_exists "inventory-design SKILL.md exists" "$SKILL"
assert_contains "name field set" "$(cat "$SKILL")" "name: inventory-design"
assert_contains "argument-hint declares positional source-id and location" \
  "$(cat "$SKILL")" 'argument-hint: "[source-id] [location]'
assert_contains "disable-model-invocation true" \
  "$(cat "$SKILL")" "disable-model-invocation: true"
assert_contains "allowed-tools enumerates browser_navigate" \
  "$(cat "$SKILL")" "mcp__playwright__browser_navigate"
assert_contains "allowed-tools enumerates browser_snapshot" \
  "$(cat "$SKILL")" "mcp__playwright__browser_snapshot"
assert_contains "allowed-tools enumerates browser_take_screenshot" \
  "$(cat "$SKILL")" "mcp__playwright__browser_take_screenshot"
assert_contains "allowed-tools enumerates browser_evaluate" \
  "$(cat "$SKILL")" "mcp__playwright__browser_evaluate"
assert_contains "allowed-tools enumerates browser_click" \
  "$(cat "$SKILL")" "mcp__playwright__browser_click"
assert_contains "allowed-tools enumerates browser_type" \
  "$(cat "$SKILL")" "mcp__playwright__browser_type"
assert_contains "allowed-tools enumerates browser_wait_for" \
  "$(cat "$SKILL")" "mcp__playwright__browser_wait_for"
assert_not_contains "allowed-tools must not use mcp__playwright__* wildcard" \
  "$(cat "$SKILL")" "mcp__playwright__*"
assert_contains "loads config context" \
  "$(cat "$SKILL")" "config-read-context.sh"
assert_contains "loads agent names" \
  "$(cat "$SKILL")" "config-read-agents.sh"
assert_contains "ends with skill-instructions hook" \
  "$(tail -n 5 "$SKILL")" "config-read-skill-instructions.sh inventory-design"
assert_contains "Agent Names defaults include browser-locator" \
  "$(cat "$SKILL")" "accelerator:browser-locator"
assert_contains "Agent Names defaults include browser-analyser" \
  "$(cat "$SKILL")" "accelerator:browser-analyser"

echo "=== inventory-design: validate-source.sh behavioural ==="
VALIDATE="$PLUGIN_ROOT/skills/design/inventory-design/scripts/validate-source.sh"
assert_file_exists "validate-source.sh exists" "$VALIDATE"
assert_file_executable "validate-source.sh is executable" "$VALIDATE"

assert_exit_code "accepts https URL" 0 "$VALIDATE" "https://prototype.example.com"
assert_exit_code "rejects file:// scheme" 1 "$VALIDATE" "file:///etc/passwd"
assert_exit_code "rejects javascript: scheme" 1 "$VALIDATE" "javascript:alert(1)"
assert_exit_code "rejects data: scheme" 1 "$VALIDATE" "data:text/html,<script>"
assert_exit_code "rejects http://localhost without --allow-internal" 1 "$VALIDATE" "http://localhost:8080"
assert_exit_code "rejects http://127.0.0.1 (loopback) without --allow-internal" 1 "$VALIDATE" "http://127.0.0.1:8080"
assert_exit_code "rejects http://169.254.169.254 (link-local AWS metadata)" 1 "$VALIDATE" "http://169.254.169.254/"
assert_exit_code "rejects RFC1918 10.x.x.x" 1 "$VALIDATE" "http://10.0.0.1/"
assert_exit_code "rejects RFC1918 192.168.x.x" 1 "$VALIDATE" "http://192.168.1.1/"
assert_exit_code "accepts code-repo path inside project root" 0 "$VALIDATE" "./examples/design-test-app"
assert_exit_code "rejects path with .. escape" 1 "$VALIDATE" "../../etc/passwd"

echo "=== inventory-design: resolve-auth.sh behavioural ==="
RESOLVE_AUTH="$PLUGIN_ROOT/skills/design/inventory-design/scripts/resolve-auth.sh"
assert_file_exists "resolve-auth.sh exists" "$RESOLVE_AUTH"
assert_file_executable "resolve-auth.sh is executable" "$RESOLVE_AUTH"

# Header wins — emits canonical mode 'header' and a stderr warning
# naming any ignored form-login vars.
ENV_OUT="$(env -i ACCELERATOR_BROWSER_AUTH_HEADER=Bearer-x \
  ACCELERATOR_BROWSER_USERNAME=u ACCELERATOR_BROWSER_PASSWORD=p \
  ACCELERATOR_BROWSER_LOGIN_URL=https://x/login \
  "$RESOLVE_AUTH" 2>/dev/null)"
assert_eq "header takes precedence over form-login vars" "header" "$ENV_OUT"
assert_stderr_contains "warns when form-login vars are ignored" "ignored" \
  env -i ACCELERATOR_BROWSER_AUTH_HEADER=Bearer-x \
  ACCELERATOR_BROWSER_USERNAME=u ACCELERATOR_BROWSER_PASSWORD=p \
  ACCELERATOR_BROWSER_LOGIN_URL=https://x/login \
  "$RESOLVE_AUTH"

# All three form-login vars set — emits 'form'.
ENV_OUT="$(env -i ACCELERATOR_BROWSER_USERNAME=u ACCELERATOR_BROWSER_PASSWORD=p \
  ACCELERATOR_BROWSER_LOGIN_URL=https://x/login \
  "$RESOLVE_AUTH" 2>/dev/null)"
assert_eq "all-three form-login vars resolve to 'form'" "form" "$ENV_OUT"

# Partial credentials — exits non-zero naming the missing var.
assert_exit_code "USERNAME+PASSWORD without LOGIN_URL fails fast" 1 \
  env -i ACCELERATOR_BROWSER_USERNAME=u ACCELERATOR_BROWSER_PASSWORD=p \
  "$RESOLVE_AUTH"
assert_stderr_contains "names the missing LOGIN_URL var" "ACCELERATOR_BROWSER_LOGIN_URL" \
  env -i ACCELERATOR_BROWSER_USERNAME=u ACCELERATOR_BROWSER_PASSWORD=p \
  "$RESOLVE_AUTH"

# No env vars — emits 'none', exits 0.
ENV_OUT="$(env -i "$RESOLVE_AUTH" 2>/dev/null)"
assert_eq "no env vars resolve to 'none'" "none" "$ENV_OUT"

echo "=== inventory-design: scrub-secrets.sh behavioural ==="
SCRUB="$PLUGIN_ROOT/skills/design/inventory-design/scripts/scrub-secrets.sh"
assert_file_exists "scrub-secrets.sh exists" "$SCRUB"
assert_file_executable "scrub-secrets.sh is executable" "$SCRUB"

# Clean body — exits 0.
CLEAN="$(mktemp)"
echo "An ordinary inventory body with no secrets." > "$CLEAN"
assert_exit_code "clean body passes scrubber" 0 \
  env -i ACCELERATOR_BROWSER_PASSWORD=hunter2_uniq "$SCRUB" "$CLEAN"

# Literal leak — exits non-zero, names the env var (not the value).
LEAKY="$(mktemp)"
echo "The reset link contains hunter2_uniq somewhere." > "$LEAKY"
assert_exit_code "literal env-var value triggers scrubber" 1 \
  env -i ACCELERATOR_BROWSER_PASSWORD=hunter2_uniq "$SCRUB" "$LEAKY"
assert_stderr_contains "scrubber names the env var by name (not value)" \
  "ACCELERATOR_BROWSER_PASSWORD" \
  env -i ACCELERATOR_BROWSER_PASSWORD=hunter2_uniq "$SCRUB" "$LEAKY"

rm -f "$CLEAN" "$LEAKY"

echo "=== inventory-design: evals ==="
EVALS="$PLUGIN_ROOT/skills/design/inventory-design/evals/evals.json"
BENCH="$PLUGIN_ROOT/skills/design/inventory-design/evals/benchmark.json"
assert_file_exists "evals.json exists" "$EVALS"
assert_file_exists "benchmark.json exists" "$BENCH"
assert_eq "evals.json is valid JSON" "$(jq empty "$EVALS" 2>&1)" ""
assert_eq "benchmark.json is valid JSON" "$(jq empty "$BENCH" 2>&1)" ""
```

The structural validation in `scripts/test-evals-structure.sh:51-129`
(0.9 pass-rate floor, eval ID coverage) automatically applies — no
new code needed there.

#### 6. Plugin manifest

**File**: `.claude-plugin/plugin.json` (lines 10-21)

Add `"./skills/design/"` to the `skills` array.

### Success Criteria:

#### Automated Verification:

- [ ] Skill structural and helper-script behavioural tests pass
      (inventory-design sections of `bash scripts/test-design.sh`).
- [ ] Eval structure validation passes: `bash
      scripts/test-evals-structure.sh` includes the new skill.
- [ ] Benchmark mean pass_rate ≥ 0.9 across all evals.
- [ ] CI gate passes: `mise run test` green.

#### Manual Verification:

- [ ] `/accelerator:inventory-design design-test-app ./examples/design-test-app
      --crawler code` produces a valid inventory directory
      `<paths.design_inventories>/YYYY-MM-DD-HHMMSS-design-test-app/inventory.md`
      plus `screenshots/` (empty for code-only crawler).
- [ ] Re-running with the same source-id creates a new directory
      and marks the prior `inventory.md` `status: superseded`.
- [ ] Hybrid crawl against a real prototype URL succeeds when MCP
      enabled.
- [ ] With MCP disabled, `--crawler hybrid` degrades to `code` with
      a Crawl Note explaining why.

---

## Phase 4: `analyse-design-gaps` Skill via skill-creator

### Overview

Create `skills/design/analyse-design-gaps/SKILL.md` and its bundled
files. Consumes two inventories produced by Phase 3 and emits a
`design-gap` artifact whose paragraphs satisfy the
`extract-work-items` cue-phrase contract.

### Changes Required:

#### 1. Skill scaffolding (use `skill-creator:skill-creator`)

**Files**:

- `skills/design/analyse-design-gaps/SKILL.md`
- `skills/design/analyse-design-gaps/scripts/gap-metadata.sh`
- `skills/design/analyse-design-gaps/scripts/audit-cue-phrases.sh`
  (per-H2 cue-phrase audit; see step 5)
- `scripts/extract-work-items-cue-phrases.txt` (new shared
  source-of-truth file: one ERE alternative per line, e.g.
  `we need to`, `users? need`, `the system must`, `implement [A-Z]`).
  `audit-cue-phrases.sh` reads this file and joins entries into a
  single `(...|...)` ERE applied with `grep -iE`. The
  `extract-work-items` SKILL.md prose at lines 130-138 must remain
  in agreement with this file; a structural test (added to
  `scripts/test-design.sh`) asserts each non-comment line in
  the file appears as a literal substring in the SKILL.md
  enumeration so drift is caught at CI time.
- `skills/design/analyse-design-gaps/evals/evals.json`
- `skills/design/analyse-design-gaps/evals/benchmark.json`

`argument-hint: "[current-source-id] [target-source-id]"` — both
positional, both required.

Skill body steps:

1. **Resolve each source-id to its current inventory directory**.
   Algorithm (specified explicitly in the SKILL.md — failure modes
   are load-bearing):

   a. Validate the source-id matches `^[a-z0-9][a-z0-9-]*$`. Reject
      with a clear error otherwise (mirrors the `inventory-design`
      validation).
   b. Glob `<paths.design_inventories>/*-{source-id}/` (the directory
      name format is `YYYY-MM-DD-HHMMSS-{source-id}`, so the suffix
      match is unambiguous).
   c. **Zero matches**: exit non-zero with the message
      `error: source-id '<id>' did not match any inventory under
      <root>. Available source-ids: <comma-separated list derived
      from existing directories>.` Suggest running
      `/accelerator:inventory-design <id> <location>` first.
   d. **Read frontmatter for each match**. If a directory's
      `inventory.md` is missing, unparseable, or lacks a `status`
      field, treat it as `superseded` for resolution purposes and
      log a one-line warning naming the directory (do not fail the
      whole run — a corrupt prior inventory should not block gap
      analysis on a healthy newer one).
   e. **Filter to `status != superseded`** (also exclude any
      leading-dot directories — those are in-progress
      `.tmp/` writes). If multiple inventories remain (a state
      that arises when a prior `inventory-design` run crashed
      between the new-write and the supersede-mutation steps,
      or under sub-second concurrent writes), apply the resolver's
      idempotent fallback in this order:

      1. **Primary tiebreaker — highest `sequence` number** in
         frontmatter. This is robust against system clock skew,
         NTP correction backwards, restoring from backup, or
         manual directory renames, because `sequence` is computed
         by reading existing inventories at write time
         (Phase 3 §1 step 1).
      2. **Secondary tiebreaker** (only when sequence numbers
         are equal — concurrent writes that read the same `max`):
         directory mtime, newest first.
      3. **Final tiebreaker** (only when both sequence and mtime
         are equal — extremely unlikely): `YYYY-MM-DD-HHMMSS`
         directory-name prefix, newest first.

      Treat all non-selected directories as superseded for the
      purposes of this run, and emit a one-line warning naming
      the older directories so the user can investigate. The
      gap artifact's References section records which directory
      was resolved.
   f. Record the resolved absolute paths for both source-ids in the
      gap artifact's frontmatter (`current_inventory`,
      `target_inventory`).

2. Read each `inventory.md` in full into context.
3. Compute structural diff across the five categories from research
   §3.2.
4. Convert each diff item to a prose paragraph with cue-phrase
   language.
5. **Post-write cue-phrase audit** (load-bearing — programmatic, not
   exhortation):
   `${CLAUDE_PLUGIN_ROOT}/skills/design/analyse-design-gaps/scripts/audit-cue-phrases.sh
   <generated-body-path>`.

   **Regex source of truth**: the audit regex is derived from the
   canonical cue-phrase list at
   `skills/work/extract-work-items/SKILL.md:130-138`. Rather than
   redeclaring patterns in two places (which silently drift), the
   script either (a) sources the list from a shared constants file
   (`scripts/extract-work-items-cue-phrases.txt`, one regex
   alternative per line) introduced as part of this skill, or (b)
   greps the canonical SKILL.md at runtime for the patterns
   enumerated there. Option (a) is preferred because it is testable
   in isolation; the file becomes the single source of truth, and
   extract-work-items also reads it on startup to assert its own
   in-prose enumeration matches.

   The audit applies the regex with **case-insensitive matching**
   (`grep -iE`) so prose written in sentence case ("We need to…",
   "The system must…", "Users need…") matches alongside the
   lowercase form documented in extract-work-items. The script
   reads the generated body, parses H2 sections, and for each
   non-empty H2 asserts at least one paragraph matches the cue
   regex.

   **Failure handling**: on first audit failure the script exits
   non-zero and names the offending H2(s); the skill catches this,
   instructs the model to revise **only** the failing sections (the
   passing sections must be byte-identical across attempts — the
   skill diff-asserts this between attempts), and re-runs the
   audit. The frontmatter timestamp is generated once at the first
   attempt and reused across retries to avoid drift. After three
   failed audits the skill aborts the write to
   `<paths.design_gaps>/`, removes any tmp/partial state, and
   instead writes the rejected body to a sibling
   `<paths.design_gaps>/.YYYY-MM-DD-{slug}.draft.md` with a Crawl
   Notes-style annotation naming the failed sections. The summary
   surfaces both the failure and the draft path, so the user can
   hand-edit and rerun the audit script themselves rather than
   re-doing the diff. This makes the cross-skill prose contract
   executable rather than aspirational and removes the silent-drop
   failure mode in `extract-work-items`.
6. Generate metadata; write artifact under
   `<paths.design_gaps>/YYYY-MM-DD-{slug}.md`.
7. Present summary; suggest `/accelerator:extract-work-items` next.

The prose-generation step (4) is load-bearing. Skill instructions
firmly direct the model to satisfy
`skills/work/extract-work-items/SKILL.md:130-138` cue-phrases
("we need to…", "users need…", "the system must…") and to avoid
the anti-pattern called out in lines 496-499 (bare structural
headings). Step 5 is the executable enforcement of that contract;
step 4's prose discipline is the first-pass attempt and step 5 is
the gate.

#### 2. Skill-creator evals

`evals/evals.json` covers:

1. Two inventories with a 14-hue → 8-token colour migration: gap
   artifact contains a Token Drift paragraph with explicit
   "we need to migrate" language.
2. Component-only-in-target: gap artifact contains a Net-New
   Features paragraph naming the component and recommending
   implementation.
3. Component-only-in-current (potential removal): gap artifact
   contains a Removed Features paragraph that explicitly asks for
   confirmation before removal (per research §3.2).
4. Empty drift in some categories: H2 sections may be omitted if
   empty (per research §3.2 last paragraph).
5. Round-trip: `analyse-design-gaps` output, when fed to
   `extract-work-items` (eval invokes both skills sequentially),
   produces ≥ 1 work item per non-empty drift category.
6. **Zero-match source-id**: invoking with a source-id that has no
   inventory exits non-zero with the documented error message
   listing the available source-ids.
7. **Multi-match resolver fallback**: with two `status: draft`
   directories for the same source-id (simulating crashed prior
   run) where the older has higher `sequence` (simulating clock
   skew that would mislead a date-only resolver), the resolver
   selects by **highest sequence**, emits the documented warning,
   and produces a valid gap artifact whose References section
   names the resolved directory. Run two variants: (a) sequence
   tiebreaker, (b) equal sequence but different mtime (mtime
   secondary tiebreaker fires), to verify all three resolution
   layers are exercised.
8. **Malformed prior frontmatter**: a prior `inventory.md` whose
   frontmatter cannot be parsed is treated as superseded by the
   resolver and a one-line warning is logged; the run does not
   abort.
9. **Per-paragraph cue-phrase audit**: for each non-empty H2 in
   the generated gap artifact, at least one paragraph matches the
   cue-phrase regex sourced from
   `scripts/extract-work-items-cue-phrases.txt` and applied
   case-insensitively. Implemented as a programmatic post-write
   check inside the skill that fails the run if any non-empty
   category lacks a cue-phrase paragraph (see Phase 4 §1 step 5).

**Run-count tiers** (analogous to Phase 3 §4 tiering):

- Probabilistic evals — #1, #2, #3, #5 (round-trip to
  extract-work-items), #7 (multi-match resolver under
  hand-created fixtures), #8 (malformed frontmatter handling) —
  run **at least 5 times**.
- Deterministic structural evals — #4 (empty-H2 omission), #6
  (zero-match exit), #9 (audit script behavioural — covered
  primarily by the audit's own behavioural tests in Phase 4 §3)
  — may stay at 3 runs.

`benchmark.json` runs each at least 3 times. The with_skill mean
pass_rate must hit ≥ 0.9.

#### 3. Tests — skill structure and contract

**File**: `scripts/test-design.sh` (extend the same file with the
analyse-design-gaps sections; `test_summary` lives at the very
end of the file and runs once after all sections)

```bash
echo "=== analyse-design-gaps: skill structure ==="
SKILL="$PLUGIN_ROOT/skills/design/analyse-design-gaps/SKILL.md"
assert_file_exists "analyse-design-gaps SKILL.md exists" "$SKILL"
assert_contains "name field set" "$(cat "$SKILL")" "name: analyse-design-gaps"
assert_contains "argument-hint two positional ids" \
  "$(cat "$SKILL")" 'argument-hint: "[current-source-id] [target-source-id]"'
assert_contains "instructs cue-phrase prose" \
  "$(cat "$SKILL")" "we need to"
assert_contains "skill body invokes the cue-phrase audit script" \
  "$(cat "$SKILL")" "audit-cue-phrases.sh"
assert_file_exists "audit-cue-phrases.sh exists" \
  "$PLUGIN_ROOT/skills/design/analyse-design-gaps/scripts/audit-cue-phrases.sh"
assert_file_executable "audit-cue-phrases.sh is executable" \
  "$PLUGIN_ROOT/skills/design/analyse-design-gaps/scripts/audit-cue-phrases.sh"

echo "=== analyse-design-gaps: audit-cue-phrases.sh behavioural ==="
AUDIT="$PLUGIN_ROOT/skills/design/analyse-design-gaps/scripts/audit-cue-phrases.sh"

# Compliant fixture: exercises all four cue patterns (capitalised) +
# case-insensitive matching of `grep -iE`.
COMPLIANT="$(mktemp)"
cat > "$COMPLIANT" <<'EOF'
# Gap

## Token Drift
We need to migrate the colour scale.

## Component Drift
Users need a five-variant Button.

## Screen Drift
The system must support a redesigned navigation pattern.

## Net-New Features
Implement Search to expose Cmd+K activation and recent-history previews.
EOF
assert_exit_code "audit passes on compliant fixture (all four cue patterns, capitalised)" 0 "$AUDIT" "$COMPLIANT"

# Non-compliant fixture: H2 with prose but no cue-phrase.
NONCOMPLIANT="$(mktemp)"
cat > "$NONCOMPLIANT" <<'EOF'
# Gap

## Token Drift
The colours are different.

## Component Drift
We need a five-variant Button.
EOF
assert_exit_code "audit fails on non-compliant fixture" 1 "$AUDIT" "$NONCOMPLIANT"

# Negative for `implement [A-Z]` — must not match lowercase implementer name.
LOWER_IMPL="$(mktemp)"
cat > "$LOWER_IMPL" <<'EOF'
# Gap

## Token Drift
implement foo to handle the colour migration.
EOF
assert_exit_code "audit fails when 'implement' is followed by lowercase" 1 "$AUDIT" "$LOWER_IMPL"

# Empty H2: must be allowed (research §3.2 says empty categories may be omitted, but if present should not block).
EMPTY_H2="$(mktemp)"
cat > "$EMPTY_H2" <<'EOF'
# Gap

## Token Drift

## Component Drift
We need a five-variant Button.
EOF
assert_exit_code "audit passes when an H2 is empty" 0 "$AUDIT" "$EMPTY_H2"

# Cue-phrase source-of-truth: the shared regex file exists and is
# loaded by both this script and (eventually) extract-work-items.
assert_file_exists "extract-work-items cue-phrase regex file exists" \
  "$PLUGIN_ROOT/scripts/extract-work-items-cue-phrases.txt"

rm -f "$COMPLIANT" "$NONCOMPLIANT" "$LOWER_IMPL" "$EMPTY_H2"
assert_contains "ends with skill-instructions hook" \
  "$(tail -n 5 "$SKILL")" "config-read-skill-instructions.sh analyse-design-gaps"

echo "=== analyse-design-gaps: evals ==="
EVALS="$PLUGIN_ROOT/skills/design/analyse-design-gaps/evals/evals.json"
BENCH="$PLUGIN_ROOT/skills/design/analyse-design-gaps/evals/benchmark.json"
assert_file_exists "evals.json exists" "$EVALS"
assert_file_exists "benchmark.json exists" "$BENCH"
assert_eq "evals.json is valid JSON" "$(jq empty "$EVALS" 2>&1)" ""
assert_eq "benchmark.json is valid JSON" "$(jq empty "$BENCH" 2>&1)" ""
```

### Success Criteria:

#### Automated Verification:

- [ ] Skill structural and audit-cue-phrases.sh behavioural tests
      pass (analyse-design-gaps sections of `bash scripts/test-design.sh`).
- [ ] Eval structure validation passes for the new skill.
- [ ] Benchmark mean pass_rate ≥ 0.9 across the tiered run-counts
      from §2 (probabilistic evals at 5+ runs, deterministic at 3).
- [ ] CI gate passes: `mise run test` green.

#### Manual Verification:

- [ ] `/accelerator:analyse-design-gaps current target` produces a
      file under `<paths.design_gaps>/`.
- [ ] Each non-empty H2 section contains at least one paragraph
      with a cue-phrase ("we need to…" / "users need…" /
      "the system must…").
- [ ] Feeding the gap artifact to `/accelerator:extract-work-items`
      produces work items in `<paths.work>/`.

---

## Phase 5: Documentation, CHANGELOG, and Version Bump

### Overview

Final user-facing polish. Performed after Phases 1-4 are merged so
the docs reflect shipped behaviour.

### Changes Required:

#### 1. README — `## Design Convergence` section

**File**: `README.md`

Add a new H2 `## Design Convergence` immediately after
`## Work Item Management` (so the chain sits next to the skill it
feeds). Mirror the Work Item Management section's structure:
explainer paragraph, ASCII diagram, skills table, three-step
example invocation, then a pointer to the research doc.

```markdown
## Design Convergence

Design convergence skills capture two design surfaces — a current
frontend and a target prototype — as structured inventory artifacts,
then compute a structured gap between them. The gap artifact's
prose paragraphs satisfy the cue-phrase contract that
`extract-work-items` consumes, so the workflow plugs straight into
the existing work-item lifecycle. Each inventory snapshot is
self-contained (markdown plus screenshots in a dated directory);
re-running for the same source supersedes the prior snapshot
without losing it.

```
inventory-design (current)  ─┐
                             ├─▶ analyse-design-gaps ─▶ extract-work-items ─▶ meta/work/*
inventory-design (target)   ─┘                                                  │
                                                                                ▼
                                                                       create-plan ─▶ implement-plan
```

| Skill                    | Usage                                                                        | Description                                                                              |
|--------------------------|------------------------------------------------------------------------------|------------------------------------------------------------------------------------------|
| **inventory-design**     | `/accelerator:inventory-design [source-id] [location] [--crawler MODE]`      | Generate a design inventory (tokens, components, screens, features) for a frontend source |
| **analyse-design-gaps**  | `/accelerator:analyse-design-gaps [current-source-id] [target-source-id]`    | Compute a structured gap between two inventories as actionable prose                     |

Three-step example:

```
/accelerator:inventory-design current ./apps/webapp
/accelerator:inventory-design prototype https://prototype.example.com
/accelerator:analyse-design-gaps current prototype
```

The resulting gap artifact under `meta/design-gaps/` feeds straight
into `/accelerator:extract-work-items <gap-file>`. See
[`meta/research/2026-05-02-design-convergence-workflow.md`](meta/research/2026-05-02-design-convergence-workflow.md)
for the full design rationale.

`inventory-design` supports three crawler modes — `code` (static
analysis only, no MCP needed), `runtime` (Playwright MCP only),
and `hybrid` (both, default for code-repo sources when the MCP is
available). See the `### Authenticated browser crawls` and
`### Security considerations` subsections under `## Installation`
for the env-var contract and security model.
```

**Visualiser doc-types reconciliation**: the existing `[Unreleased]`
CHANGELOG entry advertises the Visualiser library reader as
covering "11 doc types". This plan adds two new doc types
(`design-inventory` and `design-gap`). Inventories are
directory-style artifacts and so are not surfaced via
`documents-locator` today (see "Out of scope" note added to
"What We're NOT Doing" — followed up separately); they are also
out of scope for the Visualiser library reader in this release.
Update the existing CHANGELOG line from "11 doc types" to remain
accurate: either keep "11 doc types" (the new types are
deliberately not surfaced this round) or rephrase to
"all currently-surfaced doc types" so future additions don't
require a count bump. Recommend the rephrase.

#### 2. CHANGELOG

**File**: `CHANGELOG.md`

Lead with a one-paragraph capability framing (mirroring the existing
`[Unreleased]` Visualiser entry style), then artifact bullets, then a
Notes block listing the env-var contract and MCP setup.

```markdown
- A new design-convergence workflow that compares a current frontend
  to a target prototype via inventory-and-diff, emitting an
  actionable gap artifact that feeds straight into
  `extract-work-items`. Two new skills (`inventory-design`,
  `analyse-design-gaps`) under a new `skills/design/` category, two
  new browser-inspection agents driving the Playwright MCP server,
  and two new artifact templates.
  - Added `inventory-design` skill.
  - Added `analyse-design-gaps` skill.
  - Added `browser-locator` and `browser-analyser` agents.
  - Added `design-inventory` and `design-gap` templates.
  - Added `paths.design_inventories` and `paths.design_gaps`
    config keys.
  - Added Playwright MCP server dependency
    (`.claude-plugin/.mcp.json`, pinned to a specific
    `@playwright/mcp` version).
```

Notes block (under the same release header, sibling to the
Visualiser Notes pattern):

```markdown
**Notes**:

- `inventory-design` reads `ACCELERATOR_BROWSER_AUTH_HEADER` (or
  the trio `ACCELERATOR_BROWSER_USERNAME` /
  `ACCELERATOR_BROWSER_PASSWORD` /
  `ACCELERATOR_BROWSER_LOGIN_URL`) when crawling authenticated
  prototypes. Header takes precedence; partial form-login sets fail
  fast. The header is stripped on cross-origin navigations.
  Screenshots mask password/token fields; the skill refuses to
  write if any env-var literal appears in the generated body.
- The Playwright MCP server is pinned in `.claude-plugin/.mcp.json`
  and Claude Code prompts to enable it on first use of any skill
  that needs it. `inventory-design --crawler code` works without
  the MCP.
- First-time runtime crawls may require a browser-binary install:
  `mise run deps:install:playwright`.
```

#### 3. Version bump

Pre-release semver bump via `mise run version:write` (which
delegates to `invoke version.write`). The single source of truth
for the plugin version is `.claude-plugin/plugin.json`; `mise.toml`
does not declare a plugin-version field. Follow the existing
pre-release convention from the most recent commits
(`Bump version to 1.21.0-pre.7 [skip ci]`).

#### 4. Tests — none needed

This phase is documentation only.

### Success Criteria:

#### Automated Verification:

- [ ] `mise run test` green (no test changes; this is a guard).
- [ ] `jq empty .claude-plugin/plugin.json` succeeds (manifest still
      valid JSON after version bump).

#### Manual Verification:

- [ ] README workflow diagram renders correctly in Markdown
      preview.
- [ ] CHANGELOG entries are accurate and link to the research doc.

---

## Testing Strategy

### Unit / Bash Tests

A single `scripts/test-design.sh` file with `=== Section ===`
headers per phase. The existing `test:integration:config` task
runs `run_shell_suites(context, "scripts")`
(`tasks/test/integration.py:21-24`), which globs every executable
`test-*.sh` under `scripts/`, so dropping the new file in with
the executable bit set picks it up automatically as part of
`mise run test`. **No `mise.toml` or `tasks/test/integration.py`
edits are required**; the wiring is by glob.

(The `test:integration:config` task name is its first user, but
the underlying mechanism is generic — same pattern as
`test:integration:decisions` globbing under `skills/decisions/`.
A future restructure could rename it to
`test:integration:scripts` for accuracy, but that is a separate
cleanup.)

### Eval Tests

`scripts/test-evals-structure.sh` discovers the two new
`evals.json` files automatically. Each new skill ships a
`benchmark.json` with mean pass_rate ≥ 0.9.

### End-to-End

The cross-skill chain (`inventory-design` → `analyse-design-gaps`
→ `extract-work-items`) is exercised non-interactively by:

1. **Eval round-trip** (`analyse-design-gaps` evals.json #5):
   the eval harness runs both skills sequentially and asserts
   ≥ 1 work item per non-empty drift category. This is the
   primary automated end-to-end signal.
2. **Programmatic cue-phrase audit** (`audit-cue-phrases.sh`,
   Phase 4 §1 step 5 / Phase 4 §3 behavioural tests): isolates
   the cross-skill prose contract from `extract-work-items`
   internals so failures are diagnosable without ambiguity.

**End-to-end shell test (deferred)**: a `test:e2e:design` mise
task that drives the full chain through a non-interactive Claude
invocation is *not* added in this plan. The chain depends on a
Claude harness running the skills, and the plugin does not yet
have a precedent for non-interactive skill invocation outside
the eval framework. The eval round-trip + audit script combination
provides the same correctness signal; the recommendation to add
a separate shell-driven e2e graduates only when a non-interactive
skill-invocation pattern is established (track as follow-up).

### Manual scenarios

Using the committed fixture at `examples/design-test-app/`
(Phase 1 §4a):

1. Run `/accelerator:inventory-design design-test-app ./examples/design-test-app
   --crawler code`.
2. Run `/accelerator:inventory-design prototype https://prototype.example.com`
   (or a second local fixture if Playwright MCP is unavailable).
3. Run `/accelerator:analyse-design-gaps design-test-app prototype`.
4. Run `/accelerator:extract-work-items <gap-file>` and confirm
   work items appear in `<paths.work>/`.
5. Confirm graceful degradation: disable Playwright MCP, run
   `/accelerator:inventory-design design-test-app
   ./examples/design-test-app --crawler hybrid` — should fall
   back to code-only and emit the auto-fallback notice before
   the crawl starts.

## Performance Considerations

- Inventory generation is a once-off per snapshot; latency
  dominated by Playwright crawl. Not a hot path.
- Gap analysis reads two markdown files in full; files are bounded
  by inventory size (typically <2 MB). No streaming needed.
- Repo size growth from screenshots is the main long-term concern;
  pruning is deferred per "What We're NOT Doing".

## Migration Notes

No migration required. New skills/agents/templates are additive.
Existing artifacts are unaffected.

For users with existing `.accelerator.json` configs: no schema
change. New `paths.design_inventories` / `paths.design_gaps` keys
fall back to the documented defaults via the standard caller-supplied
default mechanism (`scripts/config-read-path.sh:23-24`).

Power users who customise `paths.*` in their config (e.g. moving
`paths.research` to a sibling directory) can opt the new path keys
into the same layout by adding `paths.design_inventories` and
`paths.design_gaps` overrides. The standard config mechanism
applies; see `/accelerator:configure paths help` for the available
keys.

## References

- Original research:
  `meta/research/2026-05-02-design-convergence-workflow.md`
- Related note (out of scope):
  `meta/notes/2026-05-02-research-directory-subcategory-restructure.md`
- ADR informing agent split:
  `meta/decisions/ADR-0001-context-isolation-principles.md`
- Locator/analyser canonical templates:
  `agents/codebase-locator.md`, `agents/codebase-analyser.md`
- Skill orchestration template:
  `skills/research/research-codebase/SKILL.md`
- Cue-phrase contract:
  `skills/work/extract-work-items/SKILL.md:130-138, 496-499`
- Init/configure touch points:
  `skills/config/init/SKILL.md:20-31, 40, 105-117`,
  `skills/config/configure/SKILL.md:386-399`
- Test infrastructure:
  `scripts/test-helpers.sh`, `scripts/test-config.sh`,
  `scripts/test-evals-structure.sh:14-15`
- Skill-creator skill:
  `~/.claude/plugins/marketplaces/claude-plugins-official/plugins/skill-creator/skills/skill-creator/SKILL.md`
- Claude Code MCP issue informing `.mcp.json` choice:
  https://github.com/anthropics/claude-code/issues/16143
