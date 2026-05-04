---
date: "2026-05-05T07:17:16+00:00"
type: plan
skill: create-plan
work-item: "meta/work/0031-consolidate-accelerator-owned-files-under-accelerator.md"
status: ready
---

# 0031: Consolidate Accelerator-Owned Files Under `.accelerator/` — Implementation Plan

## Overview

Relocate every Accelerator-owned configuration, customisation, and state
artefact from its current scattered locations under `.claude/` and `meta/` into
a single `.accelerator/` tree. Deliver this as a hard cut: runtime scripts read
the new paths only, and migration `0003-relocate-accelerator-state.sh` is the
sole recovery path for existing repos. Ship as one PR composed of seven commits
that each leave the test suite green, with TDD applied wherever new logic is
introduced.

## Current State Analysis

Today the Accelerator owns four classes of files in two parent trees:

- `.claude/accelerator.md` and `.claude/accelerator.local.md` — config files
  (`scripts/config-common.sh:25-26`).
- `.claude/accelerator/skills/<name>/{context,instructions}.md` — per-skill
  customisation (`scripts/config-read-skill-context.sh:22`,
  `config-read-skill-instructions.sh:23`, `config-summary.sh:91`).
- `.claude/accelerator/lenses/<name>-lens/SKILL.md` — custom review lenses
  (`scripts/config-read-review.sh:220`).
- `meta/templates/`, `meta/.migrations-applied`, `meta/.migrations-skipped`,
  `meta/integrations/`, `meta/tmp/` — state and template overrides
  (`scripts/config-common.sh:176`, `run-migrations.sh:13-14`,
  `jira-common.sh:61-62`, `config-summary.sh:19`).

Every config-resolution script supplies its own default as the second arg to
`config-read-path.sh` — there is no central defaults registry. The path
delegate `config-read-path.sh` is a single `exec` line wrapped in a long doc
comment, and its header lists 12 keys but does **not** include `integrations`
even though the key is already used at runtime via `jira-common.sh:61-62`.

The init skill (`skills/config/init/SKILL.md`) is markdown-only — the LLM
follows bash blocks inline. It iterates 14 paths via `<!-- DIR_COUNT:14 -->`
at line 42, of which only `templates` and `tmp` move under `.accelerator/`;
the other 12 stay in `meta/` because they are project content.

The migration framework has a self-referential bootstrap problem:
`run-migrations.sh:13-14` captures `STATE_FILE` before invoking any migration,
so migration `0003` cannot relocate the state files unilaterally — the driver
must be updated atomically with the migration. The discoverability hook
(`hooks/migrate-discoverability.sh:14-18, 22, 51`) also reads the state file
and detects un-migrated repos via `.claude/accelerator.md` or `meta/`.

The Jira state-path resolution is centralised through `jira_state_dir()` in
`jira-common.sh:54-71`; all seven Jira skills route through it, so the runtime
move is a single line change at `jira-common.sh:62`. The init-jira skill
(`jira-init-flow.sh:40-67`) currently writes `.gitignore` rules at the project
root rather than inside the state directory and writes neither `.gitkeep` nor
an absent-`.accelerator/` warning.

Tests are plain bash sourcing `scripts/test-helpers.sh`. Established test
suites: `scripts/test-config.sh`, `skills/config/migrate/scripts/test-migrate.sh`
(with fixture-on-disk pattern at `test-fixtures/0002/`),
`skills/integrations/jira/scripts/test-jira-init-flow.sh`,
`skills/visualisation/visualise/scripts/test-launch-server.sh`. Two surfaces
have no test today and need new ones: `skills/config/init/` (currently
markdown-only with no script to test) and `hooks/migrate-discoverability.sh`.

## Desired End State

A specification of the desired state and how to verify it.

After this plan is complete:

- `.accelerator/` is the sole root for Accelerator-owned files. No accelerator
  config, customisation, or state lives under `.claude/` or `meta/`.
- The init skill creates only the `.accelerator/` core scaffold for accelerator
  state, plus the project-content directories that remain under `meta/`. It
  does not create `.accelerator/state/integrations/`.
- The init-jira skill creates and owns
  `.accelerator/state/integrations/jira/` independently, including its inner
  `.gitignore` for `site.json`, `.refresh-meta.json`, and `.lock/`.
- All config-resolution scripts read from `.accelerator/` paths only.
- Migration `0003-relocate-accelerator-state.sh` exists and atomically moves
  every legacy path to its new location with an idempotency guarantee.
- The migration driver and discoverability hook handle the relocated state
  files.
- All test suites pass; new test suites cover the init bootstrap script and
  the discoverability hook.

Verification:

```bash
# Every existing test suite still passes
bash scripts/test-config.sh
bash skills/config/migrate/scripts/test-migrate.sh
bash skills/integrations/jira/scripts/test-jira-init-flow.sh
bash skills/visualisation/visualise/scripts/test-launch-server.sh

# New test suites pass
bash skills/config/init/scripts/test-init.sh
bash hooks/test-migrate-discoverability.sh

# No runtime script references the legacy paths (string match)
! grep -rE '\.claude/accelerator(\.md|\.local\.md|/skills|/lenses)|meta/(\.migrations-|integrations|templates|tmp)' \
  scripts/ skills/ hooks/ \
  --include='*.sh' --include='*.md' \
  | grep -vE '(meta/notes|meta/research|meta/plans|meta/decisions|meta/work|meta/reviews|test-fixtures|CHANGELOG\.md|migrations/000[12]-)'
```

### Key Discoveries

- `config-read-path.sh` is a pure pass-through: one executable line at
  `scripts/config-read-path.sh:24` (`exec ... "paths.${1:-}" "${2:-}"`), 25
  total lines including the header doc comment. The `integrations` key already
  resolves correctly at runtime — the only doc edit needed is one line in the
  header (`config-read-path.sh:8-19`, currently lists 12 keys without
  `integrations`).
- Tier-2 templates default lives at one place: `config-common.sh:176` passes
  `meta/templates` as the default arg. A single edit propagates to every
  template caller.
- `jira_state_dir()` in `jira-common.sh:54-71` is the only runtime line that
  needs to change for the Jira integration state directory move. The literal
  call spans lines 61-62 (`config-read-path.sh integrations meta/integrations`).
- `config-dump.sh` does **not** include `tmp` or `integrations` in its
  `PATH_KEYS`/`PATH_DEFAULTS` arrays (lines 174-200) — the dump enumerates
  only the 11 project-content keys.
- Migration `0001` does not use the `MIGRATION_RESULT: no_op_pending` sentinel,
  but `0002:25` does. Migration `0003` should follow the `0002` pattern when a
  preflight finds nothing to move.
- `write-visualiser-config.sh:39` still references the legacy
  `tickets`/`meta/tickets` path. This is unrelated to this work item and out
  of scope; flag in passing.
- The init skill is markdown-only with no script today. Extracting its
  bootstrap to `skills/config/init/scripts/init.sh` is necessary to apply TDD
  to the new logic and is in scope for this plan.

## What We're NOT Doing

- Not retroactively rewriting CHANGELOG.md historical entries (lines 109-632
  reference legacy paths). A new entry documents this reorg; prior entries
  stay accurate to their releases.
- Not relocating the 12 `meta/` directories that hold project content (plans,
  research, decisions, prs, validations, review_plans, review_prs, review_work,
  work, notes, design_inventories, design_gaps). Only Accelerator-owned files
  move.
- Not preserving pinned-path overrides for `paths.templates` or
  `paths.integrations`. The simpler unconditional move is justified per the
  research Q7 resolution; only `paths.tmp` gets the conditional treatment.
- Not changing `init-jira`'s refresh semantics for `fields.json` and
  `projects.json`. Per research Q4, they continue to atomically rewrite from
  the live tenant on every run; idempotency comes from byte-identical content
  when the tenant is unchanged.
- Not creating `issuetypes/` or any other Jira subdirectory beyond the parent
  `jira/` directory itself (per Q3 resolution).
- Not introducing a backwards-compatibility read window in runtime scripts.
  The discoverability hook is the only place a fallback chain applies, because
  it must continue to detect un-migrated repos.
- Not touching `write-visualiser-config.sh:39`'s legacy `tickets/meta/tickets`
  reference — that's a separate cleanup.
- Not writing the superseding ADR for the `accelerator.md` → `config.md`
  rename. Per work item Assumptions, it is forthcoming as a separate
  document and explicitly out of scope for this implementation plan.
  ADR-0016 receives a minimal status banner (Phase 7 / Section 4b) so
  readers landing on it know its file paths are superseded.
- Not introducing `paths.skills` or `paths.lenses` config keys.
  `.accelerator/skills/` and `.accelerator/lenses/` remain hardcoded
  with no override path, while sibling `templates`/`tmp`/`integrations`
  keys all support overrides. The asymmetry is deliberate: skills and
  lenses are extension points whose ABI is the directory layout
  itself (per ADR-0017 and ADR-0020) and is referenced from skill
  prose; an override key would create three places (config,
  resolution scripts, skill prose) that must agree on the location.
  Document the asymmetry in `skills/config/configure/SKILL.md`'s path
  table (Phase 7) so users do not look for the missing keys.
- Not removing the inner `.accelerator/.gitignore` (the unanchored
  `config.local.md` rule). Both the inner and the anchored root
  `.gitignore` rule are written by `init.sh` and migration 0003.
  Defence-in-depth: the anchored root rule is the load-bearing one,
  but a user who deletes their root `.gitignore` (perhaps regenerating
  it from a template) would lose the protection without the inner
  copy. Keeping both costs one extra write-point per surface; the
  shared helper `accelerator_ensure_inner_gitignore` (Phase 3 / 1b)
  prevents drift between the surfaces. Document the rationale as a
  short comment in the helper.

## Implementation Approach

The work decomposes into seven commits that each leave the suite green. TDD
applies to every commit that introduces logic (extraction, migration, init
bootstrap, init-jira behaviours, hook tests). Mechanical path-string changes
update tests and source together so each commit is internally consistent.

Commit ordering is chosen so that no commit can leave the working tree in a
state where fresh-repo bootstrap or runtime scripts target paths the rest of
the codebase has not yet caught up with:

1. Path-config key documentation (additive; doesn't change runtime).
2. Extract init bootstrap to a script with a baseline test asserting **today's**
   behaviour. Pure refactor.
3. Migration 0003 + driver state-file relocation + discoverability hook
   updates + new hook test. Atomic per work item Dependencies.
4. Init script update to create the `.accelerator/` scaffold; test updated to
   assert new behaviour.
5. init-jira behaviour changes (`.gitignore` location/rules, `.gitkeep`,
   absent-`.accelerator/` warning, root `.gitignore` no longer mutated).
6. Source-of-truth runtime path updates: config scripts, `jira-common.sh:62`,
   visualiser scripts and fixtures. Test fixtures updated in lockstep.
7. Documentation pass: README, configure SKILL.md, init-jira/visualise
   SKILL.md prose, CHANGELOG entry.

Within each commit, write the failing tests first, then make them pass; the
commit landed in the PR is the green state. (If the implementer prefers
red+green commit pairs for review clarity, that's compatible — just label
them clearly.)

---

## Phase 1: Path-config key documentation

### Overview

Make the existing `paths.integrations` key fully documented. The key already
resolves correctly at runtime via the generic delegate; this commit closes the
documentation gap surfaced by the research (`config-read-path.sh` header,
`configure/SKILL.md` table). Independent of every other phase.

### Changes Required

#### 1. Extend `config-read-path.sh` header doc comment

**File**: `scripts/config-read-path.sh`
**Changes**: Add `integrations` to the documented key list at lines 8-19.

```sh
# Documented keys:
#   plans, research, decisions, prs, validations, review_plans, review_prs,
#   review_work, templates, work, notes, tmp, integrations
```

#### 2. Update configure SKILL.md path-table default

**File**: `skills/config/configure/SKILL.md`
**Changes**: At line 402, update the `integrations` row default from
`meta/integrations` to `.accelerator/state/integrations`. Add a
footnote (or short prose paragraph) under the path table noting that
`.accelerator/skills/` and `.accelerator/lenses/` are deliberately
**not** exposed as path-config keys, unlike `templates`/`tmp`/
`integrations`. They are extension-point ABIs whose directory layout
is referenced from skill prose; users cannot relocate them. Move
this note into Phase 7's docs sweep if more table changes are
required there — the line-402 edit is the additive minimum needed
in Phase 1.

#### 3. Add path-resolution test for `integrations` key

**File**: `scripts/test-config.sh`
**Changes**: New test case modeled on the existing `config-read-path.sh`
assertions (line ~174 of test-config.sh). Two assertions:

```bash
echo "Test: config-read-path.sh integrations returns supplied default when paths.integrations is unset"
REPO=$(mktemp -d)
# Match the established test-config.sh pattern (per-test EXIT trap
# reassignment, or explicit cleanup at end-of-test). RETURN traps fire
# only on function return, not in the flat-script test harness.
cleanup() { rm -rf "$REPO"; }
trap cleanup EXIT
cat > "$REPO/.claude/accelerator.md" <<'EOF'
# accelerator
EOF
OUTPUT=$(cd "$REPO" && bash "$PLUGIN_ROOT/scripts/config-read-path.sh" \
  integrations .accelerator/state/integrations)
assert_eq "default returned" ".accelerator/state/integrations" "$OUTPUT"

echo "Test: config-read-path.sh integrations honours paths.integrations override"
cat > "$REPO/.claude/accelerator.md" <<'EOF'
# accelerator

paths:
  integrations: custom/integrations
EOF
OUTPUT=$(cd "$REPO" && bash "$PLUGIN_ROOT/scripts/config-read-path.sh" \
  integrations .accelerator/state/integrations)
assert_eq "override returned" "custom/integrations" "$OUTPUT"
```

(The fixture still uses `.claude/accelerator.md` because Phase 6 is the
commit that switches the source-of-truth path. Test fixtures stay
internally consistent per commit.)

### Success Criteria

#### Automated Verification:

- [ ] All existing config tests pass: `bash scripts/test-config.sh`
- [ ] New `integrations` test case passes within that suite
- [ ] No other test suite changes behaviour: `bash skills/config/migrate/scripts/test-migrate.sh && bash skills/integrations/jira/scripts/test-jira-init-flow.sh`

#### Manual Verification:

- [ ] `scripts/config-read-path.sh` header doc lists 13 keys including
  `integrations`
- [ ] `skills/config/configure/SKILL.md:402` shows the new default

---

## Phase 2: Extract init bootstrap to a script (pure refactor)

### Overview

Move the bash logic currently inline in `skills/config/init/SKILL.md` Steps 1-3
into `skills/config/init/scripts/init.sh`. SKILL.md invokes the script. Add
`skills/config/init/scripts/test-init.sh` asserting **current** behaviour
(creating 14 directories under `meta/`, writing the inner gitignore for tmp,
appending `.claude/accelerator.local.md` to the root `.gitignore`). This
commit changes no observable behaviour — every test asserts existing semantics,
the script and SKILL.md combination produces the same end state as today.

This phase is non-trivial because the existing skill is markdown-driven and
embeds 14-directory iteration with idempotency guards. The translation must
be faithful.

### Changes Required

#### 1. New init script

**File**: `skills/config/init/scripts/init.sh` (new)
**Changes**: Translate SKILL.md Steps 1-3 into a single executable script.

```sh
#!/usr/bin/env bash
# Initialise accelerator scaffold in the current project.
# Idempotent: safe to run repeatedly.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
CONFIG_READ_PATH="$PLUGIN_ROOT/scripts/config-read-path.sh"

PROJECT_ROOT="${PROJECT_ROOT:-$PWD}"
cd "$PROJECT_ROOT"

# Step 1: project-content directories under meta/
DIR_KEYS=(
  plans research decisions prs validations review_plans review_prs review_work
  templates work notes design_inventories design_gaps tmp
)
for key in "${DIR_KEYS[@]}"; do
  default="meta/$key"
  dir=$(bash "$CONFIG_READ_PATH" "$key" "$default")
  mkdir -p "$dir"
  [ -e "$dir/.gitkeep" ] || touch "$dir/.gitkeep"
done

# Step 2: inner gitignore for tmp (ADR-0019 pattern)
TMP_DIR=$(bash "$CONFIG_READ_PATH" tmp meta/tmp)
TMP_GITIGNORE="$TMP_DIR/.gitignore"
if [ ! -f "$TMP_GITIGNORE" ]; then
  cat > "$TMP_GITIGNORE" <<'EOF'
*
!.gitkeep
!.gitignore
EOF
fi

# Step 3: root .gitignore append for .claude/accelerator.local.md
ROOT_GI="$PROJECT_ROOT/.gitignore"
RULE='.claude/accelerator.local.md'
touch "$ROOT_GI"
if ! grep -qFx "$RULE" "$ROOT_GI"; then
  printf '%s\n' "$RULE" >> "$ROOT_GI"
fi
```

#### 2. Update SKILL.md to invoke the script

**File**: `skills/config/init/SKILL.md`
**Changes**: Replace the three bash blocks at lines 40-99 with a single
invocation of `scripts/init.sh`. Preserve all surrounding prose and the
`<!-- DIR_COUNT:14 -->` marker (Phase 4 will revise this marker semantics).

```sh
bash "${CLAUDE_PLUGIN_ROOT}/skills/config/init/scripts/init.sh"
```

#### 3. New test for init script

**File**: `skills/config/init/scripts/test-init.sh` (new, executable)
**Changes**: Test fixtures asserting current behaviour. Patterned on
`test-migrate.sh`.

Test cases (each in a fresh mktemp repo):

1. **Test: fresh repo creates 14 meta directories with `.gitkeep`** — assert
   each of the 14 directory paths exists and `.gitkeep` is present.
2. **Test: fresh repo writes inner tmp `.gitignore` with ADR-0019 pattern** —
   `assert_file_exists "$TMP/meta/tmp/.gitignore"` and content match the
   3-line body.
3. **Test: fresh repo appends `.claude/accelerator.local.md` to root
   `.gitignore`** — `grep -qFx '.claude/accelerator.local.md' .gitignore`.
4. **Test: re-running on already-initialised repo is idempotent** — use
   the existing `tree_hash()` helper at
   `skills/config/migrate/scripts/test-migrate.sh:26-34` to compute a
   deterministic digest of the working tree before and after a second
   `init.sh` run. Assert the two digests are equal. (`tree_hash` uses
   `find -print0` and a deterministic sort, avoiding the
   filesystem-ordering and `xargs`-splitting fragility of an ad-hoc
   `sha256sum` pipeline.)
5. **Test: re-running on repo where root `.gitignore` already contains rule
   does not duplicate it** — assert `grep -c` returns `1`.
6. **Test: respects path overrides via `paths.<key>`** — write a custom
   `.claude/accelerator.md` setting `paths.tmp: custom-tmp/`; assert
   `custom-tmp/.gitignore` exists and `meta/tmp/.gitignore` does not.

### Success Criteria

#### Automated Verification:

- [ ] All test cases in `test-init.sh` pass: `bash skills/config/init/scripts/test-init.sh`
- [ ] Existing test suites pass unchanged
- [ ] `init.sh` is executable: `[ -x skills/config/init/scripts/init.sh ]`

#### Manual Verification:

- [ ] Running `bash skills/config/init/scripts/init.sh` in a fresh repo
  produces the same tree that running the SKILL.md bash blocks did before
  this commit (compare via `find . -type f | sort`)
- [ ] SKILL.md prose still reads coherently with the script invocation in
  place of inline bash

---

## Phase 3: Migration `0003` and framework atomic updates

### Overview

The largest commit. Adds the migration script, updates the driver to relocate
state files, extends the clean-tree regex to cover `.accelerator/`, and updates
the discoverability hook to detect post-migration repos and read state from a
fallback chain. Adds a brand-new test suite for the discoverability hook.

TDD here is non-negotiable: every acceptance criterion in the work item gets
a failing test case before the migration is implemented.

### Changes Required

#### 1. New migration script

**File**: `skills/config/migrate/migrations/0003-relocate-accelerator-state.sh`
(new)
**Changes**: Modeled on `0002-rename-work-items-with-project-prefix.sh`'s
shape. The script is **idempotent**: every step is safe to re-run from
any partial-recovery state. Recovery from a mid-run failure is via VCS
revert (`jj op restore` / `git reset`) followed by a re-run; the
clean-tree refusal at the driver enforces a committed starting state.

**Internal decomposition.** The script body is organised as named
step functions invoked in sequence by a `main` dispatcher at the
bottom of the file (mirroring `0002-...sh`'s shape). This keeps the
flat-script complexity manageable, gives stack traces meaningful
names on failure, and makes per-step audit trivial:

```sh
# (top-level helper definitions: _move_if_pending, probe_paths_key,
# _emit_reconciliation_message, JIRA_INNER_GITIGNORE_RULES, etc.)

_step_preflight()          { ... }   # no-op-pending sentinel
_step_init_scaffold()      { ... }   # accelerator_ensure_* helpers
_step_rewrite_root_gitignore() { ... }
_step_warn_pinned_overrides() { ... }
_step_move_sources()       { ... }   # _move_if_pending loop
_step_relocate_state_files() { ... } # read-write-verify-remove
_step_inner_jira_gitignore() { ... }
_step_done()               { echo "MIGRATION_RESULT: applied"; }

main() {
  _step_preflight && return 0
  _step_init_scaffold
  _step_rewrite_root_gitignore
  _step_warn_pinned_overrides
  _step_move_sources
  _step_relocate_state_files
  _step_inner_jira_gitignore
  _step_done
}

main "$@"
```

The bullets below describe what each step does. The implementer
maps each bullet to the matching `_step_*` function.

Includes:

- Shebang and `# DESCRIPTION:` line for driver banner.
- Source `$PLUGIN_ROOT/scripts/config-common.sh` and `atomic-common.sh`.
- Preflight: if no source paths exist AND no destinations under
  `.accelerator/` other than the minimal scaffold this migration writes
  exist, emit `MIGRATION_RESULT: no_op_pending` and exit 0. Mid-run
  states (some sources moved, others pending) are not no-ops; they
  enter the idempotent step body.
- Move primitive `_move_if_pending <source> <dest>` (helper,
  defined in the script):
  - source absent, dest present → skip (already moved on a prior
    run).
  - source absent, dest absent → skip (nothing to do).
  - source present, dest absent → `mv` source to dest. On `EXDEV`
    (cross-device link, e.g. `.accelerator/` and `meta/` on
    different filesystems), `mv` falls back to copy-then-delete in
    most implementations, but to make behaviour explicit and
    portable the helper detects EXDEV and aborts with: "cross-
    device move not supported; relocate `<source>` to a location
    on the same filesystem as `.accelerator/`, then re-run." This
    is rare; documenting the failure mode is enough.
  - source present, dest present → exit non-zero **before** any
    write with a reconciliation message that:
      1. names both paths.
      2. emits `diff -r <source> <dest>` (or, if either path is a
         file rather than directory, `diff <source> <dest>`) so
         the user can see exactly what differs.
      3. names the prior moves that succeeded in this run (the
         helper accumulates them in a script-local
         `MOVED_THIS_RUN` array) so the user understands the
         working tree is mid-migration, not pre-migration.
      4. recommends `jj op restore` / `git reset` to return to a
         clean baseline before reconciling, or manual reconcile
         followed by re-run (idempotency guarantees re-run
         completes the remaining moves).
    No destructive change is made for *this* pair when the
    conflict branch fires; earlier successful moves in the same
    run remain applied.

  Test 11a's assertion is extended to cover: stderr message
  contains both path names, contains a `diff` section, and lists
  the `MOVED_THIS_RUN` paths. Test 11a's setup includes one prior
  successful move pair (e.g. `.claude/accelerator.md` →
  `.accelerator/config.md`) before seeding the conflict on a
  later pair, to verify the "prior moves" list is correctly
  reported.
- Initialise the **minimal** `.accelerator/` scaffold (only steps with
  no legacy source, so safe to run every time):
  - `.accelerator/.gitignore` (created only if absent), containing the
    unanchored `config.local.md` rule.
  - `.accelerator/state/` directory and `.accelerator/state/.gitkeep`.
  - **Not** pre-created here: `.accelerator/skills/`,
    `.accelerator/lenses/`, `.accelerator/templates/`,
    `.accelerator/tmp/`, `.accelerator/config.md`,
    `.accelerator/config.local.md`, and
    `.accelerator/state/integrations/jira/`. Pre-creating any of these
    breaks the move primitive — POSIX `mv` of a directory onto a
    non-empty directory nests the source instead of replacing.
- Update root `.gitignore` (idempotent, whole-line operations only):
  - Remove any line whose **whole-line** content (after trimming
    trailing whitespace) is exactly `.claude/accelerator.local.md`,
    `/.claude/accelerator.local.md`, `meta/integrations/jira/.lock`,
    `/meta/integrations/jira/.lock`,
    `meta/integrations/jira/.refresh-meta.json`, or
    `/meta/integrations/jira/.refresh-meta.json`.
  - Refuse on customised lines: if a line begins with one of the
    target patterns but has trailing content (e.g. an inline comment
    `meta/integrations/jira/.lock # note`), abort with a message
    naming the line and the file, instructing the user to reconcile
    manually. No destructive change is made on abort.
  - Ensure `.accelerator/config.local.md` is present using `grep -qFx`
    before append (re-running does not duplicate the rule).
- `paths.tmp` probe. The migration parses the legacy config files
  directly — it cannot use `config-read-path.sh`, because Phase 6
  rewires that script to read `.accelerator/config.md`, which may not
  yet exist when the migration runs. Use awk to walk the top-level
  `paths:` block in `.claude/accelerator.md` and
  `.claude/accelerator.local.md` (in that precedence order, local
  overriding team):

  ```awk
  /^paths:[[:space:]]*$/      { in_paths = 1; next }
  /^[^[:space:]]/             { in_paths = 0 }
  in_paths && /^[[:space:]]+tmp:[[:space:]]*/ {
    sub(/^[[:space:]]+tmp:[[:space:]]*/, "")
    print
    exit
  }
  ```

  Anchoring on a column-0 `paths:` key and matching `tmp:` only as
  an indented child within that block avoids false positives from
  unrelated indented `tmp:` keys elsewhere.

  **Bash wrapper for multi-file precedence.** The migration runs
  the awk script per file in team-then-local order; if local has a
  non-empty result, it overrides team. Pseudo-code:

  ```sh
  probe_paths_key() {
    local key="$1" team_file="$2" local_file="$3" raw=""
    [ -f "$team_file" ] && raw=$(awk_probe "$key" < "$team_file")
    if [ -f "$local_file" ]; then
      local local_raw
      local_raw=$(awk_probe "$key" < "$local_file")
      [ -n "$local_raw" ] && raw="$local_raw"
    fi
    # Strip trailing comment and whitespace before returning.
    printf '%s\n' "$raw" | sed 's/[[:space:]]*#.*$//; s/[[:space:]]*$//'
  }
  ```

  Comment-and-whitespace stripping ensures the pinned-override
  warning later in this step displays clean values (e.g.
  `paths.tmp = meta/tmp` instead of `paths.tmp = meta/tmp # legacy`).

  If the cleaned value is non-empty, `paths.tmp` is treated as
  **explicitly set** (any value, including `meta/tmp` literal or
  `meta/tmp/` with trailing slash) and `meta/tmp/` is left
  untouched. If the cleaned value is empty (key absent in both
  files, or comment-only line), `paths.tmp` is unset and
  `_move_if_pending meta/tmp .accelerator/tmp` runs. This aligns
  the migration with the work item's contract that "explicit
  override leaves the source untouched" and supersedes the earlier
  `config-read-path.sh tmp ""` probe (which would have returned an
  empty string in the default case, breaking the move condition).
- Conditional moves via `_move_if_pending` (each pair):
  - `.claude/accelerator.md` → `.accelerator/config.md`
  - `.claude/accelerator.local.md` → `.accelerator/config.local.md`
  - `.claude/accelerator/skills/` → `.accelerator/skills/`
  - `.claude/accelerator/lenses/` → `.accelerator/lenses/`
  - `meta/templates/` → `.accelerator/templates/`
  - `meta/integrations/jira/` → `.accelerator/state/integrations/jira/`
  - `meta/tmp/` → `.accelerator/tmp/` (only if the awk probe above
    indicates `paths.tmp` is unset)
- State-file relocation. The driver captures `STATE_FILE` at startup
  (post-Phase-3 update, the new path), so a `mv` of the state file
  here would race the driver's later `atomic_append_unique`. The
  migration instead reads `meta/.migrations-applied` and
  `meta/.migrations-skipped` if present, writes their contents to
  `.accelerator/state/migrations-applied` and `migrations-skipped`
  using a temp-file-plus-rename (`atomic_write_file` or equivalent
  helper), verifies destination contains every source line, then
  removes the source. If the destination already contains lines (from
  a partial prior run or from the driver having appended an entry
  during failed recovery), write the deduplicated union of source and
  destination lines.
- Inner Jira `.gitignore`. After the Jira move (or if
  `.accelerator/state/integrations/jira/` already exists from a
  partial prior run), ensure the directory's `.gitignore` contains
  the rules `site.json`, `.refresh-meta.json`, `.lock/`. The
  migration declares these as a **local** `JIRA_INNER_GITIGNORE_RULES`
  array with a header comment cross-referencing the canonical array
  in `jira-common.sh` (introduced in Phase 5 / Section 2 — see
  there). Both copies must be kept in sync when adding gitignored
  Jira state files; the migration cannot source `jira-common.sh`
  directly because that would couple the migration to runtime
  config-resolution code the migration is itself rewiring. A test
  in `test-jira-paths.sh` (Phase 6 / 4b) asserts byte-equality
  between the migration's local copy and `jira-common.sh`'s
  canonical array, so drift is caught at CI rather than relying on
  comment-level convention. Per-rule append uses `grep -qFx`
  (exact-line match — `grep -qF` would substring-match
  `!site.json`, `site.json.bak`, or comments). Ensure `.gitkeep`
  exists.
- Pinned-override detect-and-warn for `paths.templates` and
  `paths.integrations`. Reusing the awk probe (extended to read
  `templates:` and `integrations:` keys inside the `paths:` block of
  `.claude/accelerator.md` and `.claude/accelerator.local.md`), if
  either key is explicitly set, emit a `log_warn` notice naming the
  key, the pinned value, and the new default location, instructing
  the user to update their config post-migration. The migration
  proceeds with the unconditional move (per `What We're NOT Doing` /
  Assumptions): the user's `meta/<dir>` source moves to the new
  default location, while their pinned override continues to resolve
  to whatever they set it to — they reconcile manually. The notice
  is the only signal they get; the CHANGELOG entry repeats it.
- Trailing scaffold check. After all moves, do **not** create
  `.gitkeep` files inside `.accelerator/skills/`, `lenses/`,
  `templates/`, or `tmp/` if those directories do not exist —
  scaffolding empty extension-point directories is `init.sh`'s
  responsibility, not the migration's. The migration only relocates
  populated content.
- No explicit post-move source-removal step: `mv` already removes the
  source on success; `_move_if_pending` and the state-file relocation
  step are the only paths that delete content.

#### 1b. Shared scaffold/gitignore helpers (new file)

**File**: `scripts/accelerator-scaffold.sh` (new)
**Changes**: Defines small idempotent helpers shared between
migration 0003 (this commit) and `init.sh` (which is updated in
Phase 4 to source this file). Each helper writes only when a guard
condition fails, so re-running is a safe no-op. Avoids the
previously-planned duplication of scaffold-write logic between
init and migration.

The file carries a header doc comment naming both callers and
stating the idempotency contract:

```sh
#!/usr/bin/env bash
# Shared scaffold/gitignore helpers for the .accelerator/ tree.
#
# Sourced by:
#   - skills/config/init/scripts/init.sh (Phase 4)
#   - skills/config/migrate/migrations/0003-relocate-accelerator-state.sh
#
# Idempotency contract: every helper is a no-op when its
# post-condition already holds. Helpers do not depend on any
# config-resolution layer and may be safely sourced from the
# migration even on legacy-only repos.
#
# Helpers are named accelerator_* for the public surface;
# implementation-only helpers are prefixed _accelerator_.
```

Each helper signature is preceded by a one-line purpose comment.

Helpers:

- `accelerator_ensure_inner_gitignore <project_root>` — writes
  `.accelerator/.gitignore` containing the unanchored
  `config.local.md` rule if the file is absent.
- `accelerator_ensure_root_gitignore_rule <project_root>` — ensures
  the anchored `.accelerator/config.local.md` rule is present in
  `<project_root>/.gitignore` exactly once (`grep -qFx` guard before
  append). Touches `.gitignore` to create it if absent.
- `accelerator_remove_legacy_root_gitignore_rules <project_root>` —
  whole-line removes `.claude/accelerator.local.md` (anchored or
  unanchored), `meta/integrations/jira/.lock`, and
  `meta/integrations/jira/.refresh-meta.json`. Refuses with a
  reconciliation message naming the offending line and file if any
  matching line has trailing content (a comment, `# note`, etc.) —
  used only by migration 0003 and not by `init.sh`.
- `accelerator_ensure_state_dir <project_root>` — creates
  `.accelerator/state/` and `.accelerator/state/.gitkeep` if absent
  (the part of the scaffold that has no legacy source).

Tests (added to `scripts/test-config.sh` or a new
`scripts/test-accelerator-scaffold.sh`):

1. Each helper is a no-op on a repo where its post-condition
   already holds (idempotency).
2. `accelerator_ensure_root_gitignore_rule` does not duplicate
   when re-run.
3. `accelerator_remove_legacy_root_gitignore_rules` exits non-zero
   on `.claude/accelerator.local.md  # note` and leaves the file
   unchanged.
4. Helpers preserve unrelated `.gitignore` lines (comments, blank
   lines, sibling rules).
5. **Scaffold equivalence**: run `init.sh` on a fresh repo, then
   separately run migration 0003 on a fully-seeded legacy fixture.
   Assert the resulting `.accelerator/` subtrees are
   `tree_hash`-equal modulo populated extension-point contents
   (i.e. compare the scaffold-only files: `.gitignore`,
   `state/.gitkeep`, `skills/.gitkeep`, `lenses/.gitkeep`,
   `templates/.gitkeep`, `tmp/.gitignore`, `tmp/.gitkeep`). This
   pins the cross-surface invariant that the prior reviews flagged
   as drift-prone.

Migration 0003 (Step 1) and `init.sh` (Phase 4) both source this
file. The migration's body calls
`accelerator_ensure_inner_gitignore`,
`accelerator_ensure_state_dir`,
`accelerator_remove_legacy_root_gitignore_rules`, and
`accelerator_ensure_root_gitignore_rule` in place of the previously
inlined logic.

#### 2. Driver updates

**File**: `skills/config/migrate/scripts/run-migrations.sh`
**Changes**:

- Lines 13-14: relocate `STATE_FILE` and `SKIP_FILE` to
  `.accelerator/state/migrations-applied` and
  `.accelerator/state/migrations-skipped`.
- Line 24 (`--skip` branch) and line 203: replace `mkdir -p
  "$PROJECT_ROOT/meta"` with `mkdir -p "$(dirname "$STATE_FILE")"` (and
  the parallel call for `SKIP_FILE`). The driver no longer materialises
  a stray `meta/` directory on post-migration repos, and
  `atomic_append_unique` cannot fail because the state-file parent is
  missing.
- Lines 41-69: extend clean-tree regex / path-list to cover
  `.accelerator/` in addition to `meta/` and `.claude/accelerator*.md`
  for both jj and git branches. Path filters are passed unconditionally;
  `git status --porcelain` accepts non-existent path filters (returning
  no entries) so this is safe on pre-migration repos.
- Continue to invoke migrations with `PROJECT_ROOT` and
  `CLAUDE_PLUGIN_ROOT` exported. The `atomic_append_unique` call at
  line 204 writes to the relocated state file by virtue of the line-13
  update; migration 0003's state-file relocation step (Step 1 above)
  writes the merged state file before the driver records `0003`, so
  prior `0001`/`0002` history is preserved.

#### 3. Discoverability hook updates

**File**: `hooks/migrate-discoverability.sh`
**Changes**:

- Lines 14-18: replace the two-clause sentinel check with a three-clause
  OR. The hook continues if **any** of `.accelerator/`,
  `.claude/accelerator.md`, or `meta/` exists.
- Line 22: replace the hardcoded state-file path with an
  **exist-aware** fallback. The hook prefers
  `.accelerator/state/migrations-applied` when that file exists,
  falling back to `meta/.migrations-applied` when the new path is
  missing. The selector is per-file existence, not per-directory: a
  partial-recovery repo where `.accelerator/` exists but its state
  file does not falls through to `meta/.migrations-applied` rather
  than treating the new branch as authoritative-but-empty (which
  would mis-classify `0001`/`0002` as not applied and re-warn).
- Line 51: update the warning string to reference whichever state file
  path was actually used.
- Add a comment marking the fallback chain as a deprecation-track
  shim — the only place runtime code reads legacy paths. A follow-up
  work item (filed at implementation time, referenced from this
  plan's References section) tracks sunset criteria once telemetry or
  audit confirms the legacy branch is no longer hit. Runtime scripts
  outside this hook read the new path exclusively.

#### 4. New discoverability hook test

**File**: `hooks/test-migrate-discoverability.sh` (new, executable)
**Changes**: Patterned on `test-migrate.sh`. Test cases:

1. **Test: silent on a non-Accelerator repo** — fresh mktemp dir, run
   hook, assert **both** stderr and stdout contain no warning text
   (assert against the migrate-warning substring on each stream
   independently, not just one).
2. **Test: triggers on pre-migration repo with `.claude/accelerator.md`**
   — create only that file, run hook, assert warning emitted.
3. **Test: triggers on pre-migration repo with only `meta/`** — create
   only `meta/`, run hook, assert warning emitted.
4. **Test: triggers on post-migration repo with `.accelerator/`** —
   create only `.accelerator/state/migrations-applied`, run hook,
   assert state-file read from `.accelerator/state/migrations-applied`.
5. **Test: state-file fallback uses `meta/.migrations-applied` when
   `.accelerator/` absent** — create only `meta/.migrations-applied`,
   run hook, assert fallback path used and warning string mentions it.
6. **Test: partial-recovery state — `.accelerator/` exists but its
   state file does not** — create `.accelerator/` directory only, plus
   `meta/.migrations-applied` containing `0001\n0002\n`. Run hook,
   assert it reads from `meta/.migrations-applied` (per-file
   fallback, not per-directory) and warns about migration `0003`
   being pending. Verifies the exist-aware fallback in Step 3.
7. **Test: hook exits 0 in every above scenario** — explicit assertion
   per scenario.

#### 5. Migration test extensions

**File**: `skills/config/migrate/scripts/test-migrate.sh`
**Changes**: Add `setup_0003_repo()` and a fixture tree at
`skills/config/migrate/scripts/test-fixtures/0003/` containing seed
files for every move source. Use the existing `tree_hash()` helper
at `test-migrate.sh:26-34` for byte-equal comparisons rather than
ad-hoc `sha256sum` invocations. Add a test case per work-item AC:

1. Dirty-tree refusal across all four sentinel paths — including
   `.accelerator/`. Seed an uncommitted file under
   `.accelerator/state/` (and one variant under each of
   `.claude/accelerator*.md`, `meta/.migrations-*`,
   `meta/integrations/`) and assert non-zero exit with no source
   files moved.
2. End-to-end move from a fully-seeded legacy repo: every source
   reaches its destination; sources are absent post-run; inner Jira
   `.gitignore` and `.gitkeep` are present.
3. Inner Jira `.gitignore` contains the three exact-line rules
   `site.json`, `.refresh-meta.json`, `.lock/`. Assert via
   `grep -qFx`, not substring.
4. `paths.tmp` unset → `meta/tmp/` moves to `.accelerator/tmp/`,
   inner `.gitignore` carried by the move.
5. `paths.tmp` overridden to a custom path (e.g. `custom/tmp`) →
   `meta/tmp/` untouched and the custom path untouched.
6. **`paths.tmp` overridden to `meta/tmp` literal → `meta/tmp/`
   untouched** (explicit override; matches the work-item contract
   "explicit override leaves the source untouched", supersedes the
   earlier draft of this case which had the inverse expectation).
6a. `paths.tmp` overridden to `meta/tmp/` (trailing slash) →
    `meta/tmp/` untouched (explicit override, same as case 6).
6b. `paths.tmp` set under a nested non-`paths` block elsewhere in
    the config (e.g. `some_section:\n  tmp: foo`) → not detected as
    a `paths.tmp` override; `meta/tmp/` is moved. Verifies the awk
    probe's column-0-`paths:` anchoring.
7. Idempotency: re-running migrate after a successful run reports
   `0003` already applied (state-file check at
   `.accelerator/state/migrations-applied`).
8. Root `.gitignore` rewrite: `.claude/accelerator.local.md`
   (anchored OR unanchored, whole-line) removed; anchored
   `.accelerator/config.local.md` present exactly once after run
   (re-run does not duplicate, `grep -qFx`).
8a. Root `.gitignore` rewrite refuses on lines with trailing
    content. Seed `.gitignore` with
    `.claude/accelerator.local.md  # custom note` and assert
    non-zero exit with a reconciliation message naming the line and
    the file. No destructive change made.
9. Root `.gitignore` rewrite: `meta/integrations/jira/.lock` and
   `meta/integrations/jira/.refresh-meta.json` rules removed
   (whole-line match only — a customised line with trailing
   content triggers refusal as in 8a).
10. Already-have-`.accelerator/`-no-sources state →
    `MIGRATION_RESULT: no_op_pending` and clean exit.
11. **Idempotency from any partial state**. For each starting state,
    assert the migration completes cleanly and the resulting tree
    is `tree_hash`-equal to the all-from-scratch outcome:
    - Subset of file-pair sources moved, others pending (e.g.
      `accelerator.md` moved, `skills/` not).
    - All sources moved, state file not yet relocated.
    - State file partially relocated (destination exists with a
      subset of source lines).
    - All sources moved and state file relocated → re-run reports
      no-op and exits 0.
11a. **Conflict detection by `_move_if_pending`**. Seed both
     `.claude/accelerator.md` and `.accelerator/config.md` with
     **differing** content. Assert the migration exits non-zero
     with a reconciliation message naming both paths. Source and
     destination remain present and unchanged (no destructive
     operation took place). Recovery via VCS revert + manual
     reconciliation.
12. State-file relocation: `meta/.migrations-applied` content is
    preserved at `.accelerator/state/migrations-applied`. If the
    destination already contained lines from a partial prior run,
    the resulting file is the deduplicated union of both inputs
    (verifies the merge rule).
13. Trailing scaffold: when no `meta/templates/` source exists, the
    migration leaves `.accelerator/templates/` **uncreated**.
    Scaffolding empty extension-point directories is `init.sh`'s
    job, not the migration's.
14. Pinned-override warning for `paths.templates` and
    `paths.integrations`. Seed `.claude/accelerator.md` with
    `paths:\n  templates: custom/templates\n  integrations: custom/ints\n`.
    Run migration. Assert: stderr contains a `log_warn`-formatted
    notice naming each pinned key and value; the migration still
    moves `meta/templates/` and `meta/integrations/jira/`
    unconditionally; exit status is 0 (warning, not error).
14a. No warning when neither key is pinned. Stderr contains no
     pinned-override notice in the default-config case.

### Success Criteria

#### Automated Verification:

- [ ] All test cases pass: `bash skills/config/migrate/scripts/test-migrate.sh`
- [ ] Hook tests pass: `bash hooks/test-migrate-discoverability.sh`
- [ ] All existing test suites pass unchanged
- [ ] Migration script is executable: `[ -x skills/config/migrate/migrations/0003-relocate-accelerator-state.sh ]`

#### Manual Verification:

- [ ] On a real repo seeded with `.claude/accelerator.md`,
  `meta/.migrations-applied`, `meta/integrations/jira/fields.json`, and
  `meta/tmp/`, running `/accelerator:migrate` moves all files to
  `.accelerator/` and the original sources are gone
- [ ] Driver banner shows `0003` description from the script's
  `# DESCRIPTION:` line
- [ ] Re-running migrate after success reports "No pending migrations"

---

## Phase 4: Init script update for `.accelerator/` scaffold

### Overview

Update `skills/config/init/scripts/init.sh` (extracted in Phase 2) to:

- Drop `templates` and `tmp` from the `meta/` iteration (they move to
  `.accelerator/`).
- Add a new step creating the `.accelerator/` core scaffold matching the work
  item Target directory structure: top-level `.gitignore` with anchored
  `.accelerator/config.local.md` rule; `state/.gitkeep`; `tmp/.gitignore` with
  ADR-0019 body; `tmp/.gitkeep`; `skills/.gitkeep`; `lenses/.gitkeep`;
  `templates/.gitkeep`. Does **not** create `.accelerator/state/integrations/`.
- Update Step 3's root `.gitignore` rule from `.claude/accelerator.local.md` to
  the anchored `.accelerator/config.local.md`.
- Update SKILL.md's `<!-- DIR_COUNT:14 -->` marker to
  `<!-- DIR_COUNT:12 -->` (templates and tmp drop out of the
  meta/ enumeration). Two parser sites exist — `test-config.sh:4283-4289`
  is self-validating (compares grep count of `**<name> directory**:`
  headings against the marker); `test-design.sh:18` asserts the
  literal string and must be updated in lockstep. Both are part of
  this commit. Section 3 below specifies the lockstep edits.

Update test-init.sh test cases to assert **new** behaviour. Tests written
first (this phase is TDD-led).

### Changes Required

#### 1. Update test cases in `test-init.sh`

**File**: `skills/config/init/scripts/test-init.sh`
**Changes**:

Replace/augment the Phase 2 test cases:

1. **Test: fresh repo creates 12 meta directories** — `templates` and `tmp`
   no longer created under `meta/`.
2. **Test: fresh repo creates `.accelerator/` core scaffold** — assert
   `.accelerator/.gitignore` exists and contains anchored
   `.accelerator/config.local.md`; `state/.gitkeep`; `tmp/.gitignore` body
   matches ADR-0019 pattern; `tmp/.gitkeep`; `skills/.gitkeep`;
   `lenses/.gitkeep`; `templates/.gitkeep` all present.
3. **Test: fresh repo does NOT create `.accelerator/state/integrations/`** —
   `assert_dir_not_exists "$REPO/.accelerator/state/integrations"`.
4. **Test: root `.gitignore` contains anchored `.accelerator/config.local.md`
   rule** — exact `grep -qFx '.accelerator/config.local.md' .gitignore`.
5. **Test: re-running is idempotent** — `tree_hash`-equal before
   and after second run, reusing the helper from
   `test-migrate.sh:26-34` (consistent with Phase 2 case 4 and
   Phase 3 case 11).
6. **Test: re-running on repo where anchored rule already present does not
   duplicate** — `grep -c` returns `1`.
7. **Test: re-running on repo with old unanchored rule
   `.claude/accelerator.local.md` leaves the old rule in place** (init does
   not migrate; that's migration 0003's job). Init only ensures the new
   anchored rule is present.
8. **Test: respects `paths.tmp` override** — when override is set,
   `<override>/.gitignore` exists; `.accelerator/tmp/` not created (it is the
   default that's overridden).

   Wait: re-check. The work item says `tmp/` lives at `.accelerator/tmp/`
   under the new default. If the user has overridden `paths.tmp`, the init
   script should respect that override. So the test asserts override path
   gets the inner gitignore and `.accelerator/tmp/` does not.

#### 2. Update `init.sh`

**File**: `skills/config/init/scripts/init.sh`
**Changes**: Source `scripts/accelerator-scaffold.sh` (introduced in
Phase 3) near the top of the file, alongside the existing
`CONFIG_READ_PATH` resolution. Replace the previously inline
scaffold-write logic with helper calls. The result:

```sh
# (top of init.sh, after PLUGIN_ROOT resolution)
# shellcheck source=../../../../scripts/accelerator-scaffold.sh
source "$PLUGIN_ROOT/scripts/accelerator-scaffold.sh"

# Step 1: project-content directories under meta/ (12 items)
DIR_KEYS=(
  plans research decisions prs validations review_plans review_prs review_work
  work notes design_inventories design_gaps
)
for key in "${DIR_KEYS[@]}"; do
  default="meta/$key"
  dir=$(bash "$CONFIG_READ_PATH" "$key" "$default")
  mkdir -p "$dir"
  [ -e "$dir/.gitkeep" ] || touch "$dir/.gitkeep"
done

# Step 2: .accelerator/ core scaffold via shared helpers (introduced
# in Phase 3 / scripts/accelerator-scaffold.sh — sourced once at top
# of init.sh)
accelerator_ensure_inner_gitignore "$PROJECT_ROOT"
accelerator_ensure_state_dir "$PROJECT_ROOT"

# Step 2b: extension-point .gitkeep files (init-only — migration 0003
# does not pre-create these because they receive moves and would
# break the move primitive)
ACC_ROOT="$PROJECT_ROOT/.accelerator"
mkdir -p "$ACC_ROOT/skills" "$ACC_ROOT/lenses" "$ACC_ROOT/templates"
for d in skills lenses templates; do
  [ -e "$ACC_ROOT/$d/.gitkeep" ] || touch "$ACC_ROOT/$d/.gitkeep"
done

# Step 3: tmp directory and inner .gitignore (path may be overridden)
TMP_DIR=$(bash "$CONFIG_READ_PATH" tmp .accelerator/tmp)
mkdir -p "$TMP_DIR"
TMP_GITIGNORE="$TMP_DIR/.gitignore"
if [ ! -f "$TMP_GITIGNORE" ]; then
  cat > "$TMP_GITIGNORE" <<'EOF'
*
!.gitkeep
!.gitignore
EOF
fi
[ -e "$TMP_DIR/.gitkeep" ] || touch "$TMP_DIR/.gitkeep"

# Step 4: anchored root .gitignore rule (shared helper)
accelerator_ensure_root_gitignore_rule "$PROJECT_ROOT"
```

Note `.accelerator/.gitignore` contains the **unanchored** form
`config.local.md` (relative to the directory). The root `.gitignore` uses the
**anchored** form `.accelerator/config.local.md`. Both rules are present and
non-conflicting; the anchored root rule is the load-bearing one (the inner is
a defence in depth).

#### 3. Update SKILL.md prose

**File**: `skills/config/init/SKILL.md`
**Changes**: Update the directory list at lines 20-33 to drop `templates`
and `tmp` from the `meta/` enumeration, document the `.accelerator/`
scaffold created, and update the `<!-- DIR_COUNT:14 -->` marker to
`<!-- DIR_COUNT:12 -->`.

The marker is parsed by two test sites:

- `scripts/test-config.sh:4283-4289` — extracts the count via grep
  and compares it against the count of `**<name> directory**:`
  headings in the SKILL.md Path Resolution list. Self-validating:
  reducing both the heading list and the marker to 12 keeps this
  assertion green automatically.
- `scripts/test-design.sh:18` — asserts the **literal string**
  `<!-- DIR_COUNT:14 -->`. This must be updated in lockstep with
  the marker change to `<!-- DIR_COUNT:12 -->`. Update is part of
  this commit.

Also update the init SKILL.md prose lines that Phase 7 had not
covered:
- Line 83 (Step 3 heading) — replace `Add .claude/accelerator.local.md
  to root .gitignore` with `Add .accelerator/config.local.md to root
  .gitignore`.
- Lines 86-97 (rule example body) — replace the legacy unanchored
  rule with the anchored `.accelerator/config.local.md`.
- Line 129 (Step 4 results template) — replace `.claude/accelerator.local.md
  (added | already present)` with the new anchored rule.
- Line 136 (closing paragraph) — replace `bring meta/ and
  .claude/accelerator*.md in line` with `bring meta/, .claude/, and
  .accelerator/ in line` (or similar phrasing covering all three
  scopes).

### Success Criteria

#### Automated Verification:

- [ ] All updated `test-init.sh` cases pass
- [ ] Existing tests pass unchanged
- [ ] `init.sh` runs cleanly on a fresh mktemp dir

#### Manual Verification:

- [ ] On a fresh repo, `init.sh` creates `.accelerator/` with the exact tree
  specified in work item Target directory structure (excluding
  `state/integrations/`)
- [ ] Re-running `init.sh` is a no-op (no `git status` diff)

---

## Phase 5: `init-jira` behaviour changes

### Overview

Relocate `init-jira`'s `.gitignore` writing from the project root to inside
the integration state directory; expand the rule list to three rules
(`site.json`, `.refresh-meta.json`, `.lock/`); add `.gitkeep` writing; add
absent-`.accelerator/` stderr warning; stop mutating the project root
`.gitignore`. Refresh semantics for `fields.json`/`projects.json` are
unchanged (per Q4 resolution).

This phase is heavily TDD: every behaviour delta is asserted before
implementation.

### Changes Required

#### 1. Update `test-jira-init-flow.sh` to assert new behaviours

**File**: `skills/integrations/jira/scripts/test-jira-init-flow.sh`
**Changes**: Update existing Case 1/Case 2 setup_repo() to seed
`.accelerator/config.md` (post-Phase 6 path, but here we use
`.claude/accelerator.md` for compatibility within this commit; the test
fixture moves with Phase 6). Add assertions:

1. **Test: `.gitignore` written inside state dir** —
   `assert_file_exists ".accelerator/state/integrations/jira/.gitignore"`.
2. **Test: inner `.gitignore` body** — assert exactly three rules:
   `site.json`, `.refresh-meta.json`, `.lock/` (one per line, no extras).
3. **Test: `.gitignore` rules appended idempotently** — pre-write a
   `.gitignore` with only `site.json`, run flow, assert all three present and
   `site.json` not duplicated.
4. **Test: `.gitkeep` written when the directory would otherwise be empty**.
5. **Test: project root `.gitignore` is NOT mutated** — capture root
   `.gitignore` content before and after; assert equal. (No
   `meta/integrations/jira/.lock` or `.refresh-meta.json` rules ever added.)
6. **Test: `.accelerator/`-absent stderr warning** — run init-jira on a repo
   with no `.accelerator/`; assert stderr contains a string like
   `"\.accelerator/ is absent.*accelerator:init"`. Skill exits 0.
7. **Test: refresh idempotency** — already passing; rephrase the assertion to
   match Q4 wording (refresh always runs; byte-equal output if tenant
   unchanged).
8. **Test: `.lock/` is removed on EXIT (success)** — assert
   `assert_dir_not_exists ".accelerator/state/integrations/jira/.lock"`
   after a successful flow run.
8a. **Test: `.lock/` is removed on EXIT (failure)** — inject a
    forced failure mid-flow (e.g. by stubbing the API call to
    `exit 1`) and assert the `.lock/` directory is still cleaned up
    on exit. Verifies the EXIT trap mechanism rather than just the
    happy-path post-condition; a regression that replaces the trap
    with an explicit end-of-script `rm` would fail this test.

#### 2. Update `jira-init-flow.sh`

**File**: `skills/integrations/jira/scripts/jira-init-flow.sh`
**Changes**:

- **New constant** `JIRA_INNER_GITIGNORE_RULES` introduced in
  `jira-common.sh` (this phase — Phase 5 — is where the array is
  first added):

  ```sh
  # Files inside .accelerator/state/integrations/jira/ that must
  # not be committed: per-developer site identity, refresh
  # timestamp sidecar (not byte-idempotent), transient lock dir.
  JIRA_INNER_GITIGNORE_RULES=(
    site.json
    .refresh-meta.json
    .lock/
  )
  ```

  Migration 0003 declares a parallel local copy with the same
  contents and a cross-reference comment (see Phase 3 / Section 1
  inner Jira `.gitignore` step). A regression test in
  `test-jira-paths.sh` (Phase 6 / 4b) pins the two arrays to
  byte-equality so drift is caught at CI.

- `_jira_ensure_inner_gitignore` (renamed from
  `_jira_ensure_gitignore` to reflect the new responsibility)
  rewritten to write `$state_dir/.gitignore` inside the state
  directory with the rules sourced from
  `JIRA_INNER_GITIGNORE_RULES` (defined immediately above in
  `jira-common.sh`). No longer touches project root `.gitignore`.
  Idempotent per-rule with `grep -qFx` (exact-line match, not
  substring — `grep -qF 'site.json'` would false-match on
  `!site.json`, `site.json.bak`, or comments containing the
  substring).
- New helper `_jira_ensure_gitkeep` writing `.gitkeep` if the
  directory would otherwise be empty.
- New helper `_jira_warn_if_accelerator_absent` calling `log_warn`
  (the existing helper from `log-common.sh`, used elsewhere in the
  Jira flow) with a fixed message of the form
  `".accelerator/ is absent — run /accelerator:init to create it."`.
  Called once early in the flow. Does not exit non-zero. The test
  asserts via a substring check on a fixed phrase, not a regex over
  the entire format.

#### 3. Update prose

**File**: `skills/integrations/jira/init-jira/SKILL.md`
**Changes**: Update lines 8, 106, 148-149, 153 from `meta/integrations/jira/`
to `.accelerator/state/integrations/jira/`. (Phase 7 will do the broader
docs sweep; this is the prose immediately adjacent to the behaviour change.)

### Success Criteria

#### Automated Verification:

- [ ] All test cases in `test-jira-init-flow.sh` pass
- [ ] `test-init.sh` and `test-migrate.sh` pass unchanged
- [ ] Other Jira flow tests pass unchanged: `bash skills/integrations/jira/scripts/test-*.sh`

#### Manual Verification:

- [ ] On a fresh repo with `.accelerator/` already initialised, running
  `/accelerator:init-jira` produces the state directory with inner
  `.gitignore` and `.gitkeep`, and root `.gitignore` is untouched
- [ ] On a fresh repo without `.accelerator/`, running init-jira emits the
  warning to stderr and still completes successfully
- [ ] Re-running init-jira against an unchanged tenant produces no diff

---

## Phase 6: Source-of-truth runtime path updates

### Overview

The mechanical bulk: update every config script and visualiser script to
reference the new paths. Test fixtures update in lockstep so each commit is
internally consistent and the suite stays green. After this commit, runtime
scripts read `.accelerator/` exclusively (the hard cut).

No genuinely new logic — string-level changes only. TDD applies in the form
of fixture/assertion updates rather than new test cases.

### Changes Required

#### 1. Config script path constants

**Files and changes**:

- `scripts/config-common.sh:25-26` — `team`/`local_` paths to
  `.accelerator/config.md` / `.accelerator/config.local.md`.
- `scripts/config-common.sh:176` — tier-2 templates default
  `.accelerator/templates`.
- `scripts/config-read-skill-context.sh:22` — `.accelerator/skills/`.
- `scripts/config-read-skill-instructions.sh:23` — `.accelerator/skills/`.
- `scripts/config-summary.sh:19` — tmp default `.accelerator/tmp`.
- `scripts/config-summary.sh:91` — `SKILL_CUSTOM_DIR` to
  `.accelerator/skills`.
- `scripts/config-summary.sh:114` — warning string updated.
- `scripts/config-read-review.sh:220` — `CUSTOM_LENSES_DIR` to
  `.accelerator/lenses`.
- `scripts/config-dump.sh:23-24` — file path strings.
- `scripts/config-dump.sh:97-99` — display labels.

#### 1b. Mid-session legacy-path detector

**File**: `scripts/config-common.sh`
**Changes**: Add a separate `config_assert_no_legacy_layout()`
function and require user-facing entry-point scripts to call it at
top level (not from within a subshell). Calling the assertion from
inside `config_find_files()` would not work: callers invoke that
function via command substitution (`result=$(config_find_files)`),
where `exit 1` only kills the subshell and the parent script
continues with an empty `result`. Putting the assertion at top
level of each entry-point script ensures the exit reliably
terminates the process.

```sh
# Assert the repo is not in a legacy-only layout. Exits non-zero
# with an actionable migrate directive when .claude/accelerator.md
# exists but .accelerator/config.md does not. Must be called from
# top-level script context (not from inside command substitution),
# because exit from a subshell does not propagate.
#
# Migration 0003 sources config-common.sh for utilities but does
# NOT call this function — by design. The function is opt-in at
# entry-point scripts only.
config_assert_no_legacy_layout() {
  local root
  root=$(config_project_root)
  local team="$root/.accelerator/config.md"
  local legacy_team="$root/.claude/accelerator.md"
  if [ ! -f "$team" ] && [ -f "$legacy_team" ]; then
    printf '%s\n' \
      "Accelerator: legacy config detected at .claude/accelerator.md." \
      "Run /accelerator:migrate to update the layout, then retry." >&2
    exit 1
  fi
}
```

`config_find_files()` itself stays a pure path enumerator
(unchanged contract: outputs zero, one, or two existing config
file paths on stdout; returns 0).

**Entry-point scripts** that source `config-common.sh` and need
the legacy guard add a top-level call to
`config_assert_no_legacy_layout` immediately after the source
line. The list:

- `scripts/config-read-path.sh`
- `scripts/config-read-value.sh`
- `scripts/config-read-skill-context.sh`
- `scripts/config-read-skill-instructions.sh`
- `scripts/config-read-review.sh`
- `scripts/config-read-template.sh`
- `scripts/config-summary.sh`
- `scripts/config-dump.sh`

These are all invoked as standalone bash scripts (not sourced from
inside command substitution), so the top-of-script `exit 1`
reliably terminates the parent process. Skills invoke them via
`bash scripts/config-read-*.sh ...`, which executes the assertion
in the called script's own process — the calling skill sees the
non-zero exit status and the stderr directive.

**Migration 0003 / `run-migrations.sh` / `accelerator-scaffold.sh`
do NOT add this call.** They legitimately need to operate on
legacy-only repos. The function is opt-in by design.

Tests in `scripts/test-config.sh`:

1. Seed only `.claude/accelerator.md`, invoke `bash
   scripts/config-read-path.sh tmp default` as a child process
   (capturing exit status from the parent), assert non-zero exit
   and the migrate directive on stderr. Verifies the parent
   process — not a subshell — terminates.
2. Seed both `.claude/accelerator.md` and
   `.accelerator/config.md` (mid-migration partial state), invoke
   the same script, assert exit 0 and normal output (the new
   path takes precedence; no warning).
3. Seed only `.accelerator/config.md`, invoke the script, assert
   exit 0 and normal output.
4. Seed neither file (non-accelerator repo), invoke the script,
   assert exit 0 and the script's empty/default output (the
   assertion is a no-op when no legacy state exists).
5. Source `scripts/config-common.sh` from a process whose CWD is
   a legacy-only fixture but never call
   `config_assert_no_legacy_layout`. Assert source returns 0
   with no output. This pins the no-side-effect-at-source
   contract that the migration relies on.

#### 2. Jira runtime default

**File**: `skills/integrations/jira/scripts/jira-common.sh`
**Changes**: Line 62 — `config-read-path.sh integrations` default arg
becomes `.accelerator/state/integrations`.

#### 3. Visualiser scripts

**Files and changes**:

- `skills/visualisation/visualise/scripts/launch-server.sh:16` — tmp default
  `.accelerator/tmp`.
- `skills/visualisation/visualise/scripts/launch-server.sh:127` — error hint
  `.accelerator/config.local.md`.
- `skills/visualisation/visualise/scripts/stop-server.sh:13` — tmp default.
- `skills/visualisation/visualise/scripts/status-server.sh:13` — tmp default.
- `skills/visualisation/visualise/scripts/write-visualiser-config.sh:48` —
  templates default `.accelerator/templates`.
- `skills/visualisation/visualise/SKILL.md:21` — templates default in docs.
- `skills/visualisation/visualise/SKILL.md:24` — tmp default in docs.
- `skills/visualisation/visualise/SKILL.md:96-98` — config file path docs.
- `skills/visualisation/visualise/server/tests/fixtures/config.valid.json` —
  every legacy `meta/tmp` and `meta/templates` string updated to new defaults
  (lines 5, 9, 24, 29, 34, 39, 44 per research).

#### 4. Test fixture updates

**Files and changes**:

- `scripts/test-config.sh` — every heredoc fixture seeding
  `.claude/accelerator.md` updated to seed `.accelerator/config.md`. Every
  assertion of resolved paths updated to expect `.accelerator/...`.
- `skills/integrations/jira/scripts/test-jira-init-flow.sh` — fixture
  setup_repo() seeds `.accelerator/config.md` instead of
  `.claude/accelerator.md`. State dir assertions expect
  `.accelerator/state/integrations/jira/`.
- `skills/visualisation/visualise/scripts/test-launch-server.sh` —
  `make_project()` seeds `.accelerator/config.md` and `.accelerator/tmp/`
  instead of `.claude/` and `meta/tmp/`. JSON assertions expect
  `.accelerator/...` defaults.
- `skills/config/migrate/scripts/test-migrate.sh` — `setup_old_repo()` and
  similar fixtures keep using the legacy paths (their job is to set up
  legacy state for the migration to consume). No change needed.
- `skills/config/init/scripts/test-init.sh` — already updated in Phase 4.

#### 4b. AC-mapped automated assertions (regression guards)

These tests promote work-item acceptance criteria from one-shot manual
greps into permanent regression guards. Each addresses an AC that the
plan was previously delegating to the cross-cutting clean-room grep in
`Desired End State`.

- `scripts/test-config.sh` — new test cases for AC "config-common.sh,
  config-dump.sh, and config-summary.sh emit no path-valued line with
  a .claude/ or meta/ prefix":

  ```bash
  echo "Test: config-summary.sh / config-dump.sh emit no legacy path prefixes"
  # Seed a fully-migrated repo
  REPO=$(mktemp -d); trap 'rm -rf "$REPO"' EXIT
  mkdir -p "$REPO/.accelerator/state" "$REPO/.accelerator/skills" \
           "$REPO/.accelerator/lenses" "$REPO/.accelerator/templates" \
           "$REPO/.accelerator/tmp"
  cat > "$REPO/.accelerator/config.md" <<'EOF'
  # accelerator
  EOF
  for script in config-summary.sh config-dump.sh; do
    OUTPUT=$(cd "$REPO" && bash "$PLUGIN_ROOT/scripts/$script")
    if echo "$OUTPUT" | grep -E '(^|[[:space:]:=])(/)?\.claude/|(^|[[:space:]:=])(/)?meta/' >/dev/null; then
      fail "$script emitted a legacy-path prefix:"$'\n'"$OUTPUT"
    fi
  done
  ```

  Plus a test that each `config-read-skill-context.sh`,
  `config-read-skill-instructions.sh`, `config-read-review.sh`, and
  `config-read-template.sh` returns a path beginning with
  `.accelerator/` (or empty) for the seeded fixture.

- `skills/integrations/jira/scripts/test-jira-paths.sh` (new,
  executable) — covers AC "no Jira integration skill script
  contains a hardcoded reference to `meta/integrations/jira/`":

  ```bash
  for script in init-jira create-jira-issue update-jira-issue \
                comment-jira-issue attach-jira-issue \
                search-jira-issues show-jira-issue; do
    file="$PLUGIN_ROOT/skills/integrations/jira/$script/SKILL.md"
    # Plus the matching scripts/ files
    if grep -nE 'meta/integrations/jira' "$file" 2>/dev/null; then
      fail "$file contains a legacy-path reference"
    fi
  done
  # Also assert each script resolves the integrations path via
  # config-read-path.sh integrations rather than a literal.
  ```

  The exact iteration list mirrors the work-item AC; add coverage for
  each script's accompanying `scripts/` file when present.

- `skills/visualisation/visualise/scripts/test-launch-server.sh` —
  new test case for AC "given migration applied and no `paths.tmp`
  override, the effective tmp path used by `launch-server.sh` is
  `.accelerator/tmp`":

  ```bash
  echo "Test: launch-server resolves tmp to .accelerator/tmp when paths.tmp unset"
  # make_project() with no paths.tmp override
  ...
  # Assert generated config JSON / effective tmp path contains
  # ".accelerator/tmp" exactly once and no "meta/tmp" anywhere.
  ```

  Distinct from the existing fixture-update assertions (which only
  exercise the override-set path).

#### 5. Root `.gitignore`

**File**: `.gitignore`
**Changes**: Replace line 13 `.claude/accelerator.local.md` with anchored
`.accelerator/config.local.md`. Remove lines 19-20
(`meta/integrations/jira/.lock`, `meta/integrations/jira/.refresh-meta.json`).

### Success Criteria

#### Automated Verification:

- [ ] Every test suite passes: `bash scripts/test-config.sh && bash skills/config/init/scripts/test-init.sh && bash skills/config/migrate/scripts/test-migrate.sh && bash skills/integrations/jira/scripts/test-jira-init-flow.sh && bash skills/visualisation/visualise/scripts/test-launch-server.sh && bash hooks/test-migrate-discoverability.sh`
- [ ] No runtime script under `scripts/`, `skills/`, or `hooks/` references
  the legacy paths (the spec command in Desired End State passes)
- [ ] `config-summary.sh` and `config-dump.sh` produce no `.claude/` or
  `meta/` strings in their path-related output

#### Manual Verification:

- [ ] On a migrated repo, `/accelerator:configure` workflow exercises every
  config surface (skills, lenses, templates, tmp) and resolves them from
  `.accelerator/`
- [ ] Visualiser launches against a migrated repo and renders templates from
  `.accelerator/templates/` correctly

---

## Phase 7: Documentation pass

### Overview

Update every prose reference to the legacy paths. Mostly mechanical. Adds a
CHANGELOG entry. Final commit.

### Changes Required

#### 1. README.md

**File**: `README.md`
**Changes**: Update path references at lines 90, 107, 115, 122, 128,
160-161, 228, 246, 254, 257, 324-325, 352, 387, 483 to use
`.accelerator/...` paths. The Migrations section (lines 105-129)
needs **structural rework** beyond simple substitution: the prose
currently says "apply any pending migrations to your `meta/`
directory and `.claude/accelerator*.md` config files" and "All
mutations are tracked in `meta/.migrations-applied`" / "Skipped IDs
are tracked in `meta/.migrations-skipped`" / "A `SessionStart` hook
fires automatically when `meta/.migrations-applied` lags". After
migration 0003, state files live at
`.accelerator/state/migrations-*` and the discoverability hook reads
via the exist-aware fallback chain (Phase 3). Rewrite the Migrations
section prose to reflect the post-migration layout, with a
parenthetical or footnote describing the legacy fallback that the
hook applies for un-migrated repos.

#### 2. configure SKILL.md

**File**: `skills/config/configure/SKILL.md`
**Changes**: Update path references throughout: lines 19-20, 31-32, 47, 51,
59, 96, 239, 242, 306, 309, 336, 396, 399, 621, 624, 830-831 to
`.accelerator/...` paths. (Line 402 already updated in Phase 1.)

#### 3. init-jira SKILL.md

**File**: `skills/integrations/jira/init-jira/SKILL.md`
**Changes**: Final sweep of any remaining prose references. (Lines 8, 106,
148-149, 153 already updated in Phase 5.)

#### 4. visualise SKILL.md

**File**: `skills/visualisation/visualise/SKILL.md`
**Changes**: Already updated in Phase 6 (prose at lines 21, 24, 96-98). Final
sweep for any other prose references.

#### 4a. config-read-path.sh header parenthetical defaults

**File**: `scripts/config-read-path.sh`
**Changes**: After Phase 6 ships, the runtime defaults for `tmp`,
`templates`, and the new `integrations` key all live under
`.accelerator/`. The header doc comment's parenthetical defaults
(`(default: meta/tmp)` etc.) become stale. Update each parenthetical
to match the runtime default — at minimum `tmp` (now
`.accelerator/tmp`), `templates` (now `.accelerator/templates`), and
`integrations` (`.accelerator/state/integrations`). Remaining keys
(plans, research, decisions, etc.) keep their `meta/<key>` defaults.
The parentheticals are the script's only catalogue documentation;
they must stay aligned with runtime behaviour.

#### 4c. Source proposal note status banner

**File**: `meta/notes/2026-04-29-accelerator-config-state-reorg.md`
**Changes**: Add a one-line status banner at the top of the note
mirroring the ADR-0016 banner edit. Treat the note as historical
record — do not rewrite the body, only add the status:

```markdown
**Status**: Implemented in
`meta/work/0031-consolidate-accelerator-owned-files-under-accelerator.md`.
Retained for historical context.
```

This closes the documentation-graph gap where a reader following
the work item's References link to the note would otherwise read
a proposal that appears unresolved.

#### 4b. ADR-0016 status banner

**File**: `meta/decisions/ADR-0016-userspace-configuration-model.md`
**Changes**: ADR-0016 documents the userspace configuration model with
file paths `.claude/accelerator.md` and `.claude/accelerator.local.md`.
This work item supersedes those file paths (now `.accelerator/config.md`
and `.accelerator/config.local.md`). The full superseding ADR for the
rename is forthcoming as a separate document (per work-item
Assumptions and `What We're NOT Doing`), but ADRs are append-only —
ADR-0016's status header should still be updated so a reader landing
on it does not believe it accurately describes current behaviour.

Add a status note immediately under the ADR's status field:

```markdown
**Status**: Active — file paths superseded in part by
`meta/work/0031-consolidate-accelerator-owned-files-under-accelerator.md`
(`.accelerator/config*.md` replaces `.claude/accelerator*.md`). A
full superseding ADR is forthcoming.
```

This is a minimal, non-rewriting status annotation; it does not
modify the decision text and preserves the historical record.

#### 5. CHANGELOG entry

**File**: `CHANGELOG.md`
**Changes**: Two edits to the `[Unreleased]` section.

**(a) Rewrite the existing Jira state-directory entry at lines 109-113.**
The existing entry shipped under `[Unreleased]` and describes the
state directory as living at `meta/integrations/jira/`, with a note
that "a future top-level `.accelerator/state/` reorg is a one-key
change". That future reorg is now happening in the same release.
Leaving the old wording in place would ship two contradictory
descriptions of where Jira state lives. Replace lines 109-113 with:

```markdown
- **`.accelerator/state/integrations/jira/` state directory**:
  Version-controlled team-shared cache for the field catalogue,
  project list, and site metadata. Owned by `init-jira` (which writes
  the inner `.gitignore` covering `site.json`, `.refresh-meta.json`,
  and `.lock/`). Path honours the `paths.integrations` config key
  (default: `.accelerator/state/integrations`).
```

**(b) Add a new BREAKING entry at the top of `[Unreleased]` documenting
the consolidation in full.** Cover every user-actionable detail —
the work-item Migration Notes treats this as the canonical user-facing
source of upgrade information.

```markdown
### Changed

- **BREAKING**: All Accelerator-owned config, customisation, and
  state files now live under `.accelerator/` rather than `.claude/`
  and `meta/`. Run `/accelerator:migrate` to apply migration
  `0003-relocate-accelerator-state`, which moves existing files into
  the new layout. Specifically:
    - `.claude/accelerator.md` → `.accelerator/config.md`
    - `.claude/accelerator.local.md` → `.accelerator/config.local.md`
    - `.claude/accelerator/skills/` → `.accelerator/skills/`
    - `.claude/accelerator/lenses/` → `.accelerator/lenses/`
    - `meta/templates/` → `.accelerator/templates/`
    - `meta/integrations/` → `.accelerator/state/integrations/`
    - `meta/.migrations-*` → `.accelerator/state/migrations-*`
    - `meta/tmp/` → `.accelerator/tmp/` (only if `paths.tmp` is unset;
      explicit overrides — including `paths.tmp: meta/tmp` literal —
      are preserved untouched)
- **Restart your Claude Code session** after the plugin update before
  invoking any accelerator skill on an un-migrated repo. The
  SessionStart discoverability hook prompts you to run the migration;
  invoking a skill mid-session before migrate exits with the same
  directive thanks to a complementary check in `config-common.sh`,
  but a clean session start gives the cleanest UX.
- **Pinned `paths.templates` or `paths.integrations` overrides are
  not preserved**. If you have either of these keys set to a
  `meta/<dir>` value (or any custom path), the migration moves the
  legacy `meta/<dir>` contents to the new default location
  unconditionally and emits a stderr notice naming the pinned key.
  Update your config to point at the new default
  (`.accelerator/templates` / `.accelerator/state/integrations`) or
  reconcile manually post-migration. `paths.tmp` is the one path key
  whose pinned value the migration leaves untouched.
- **Project root `.gitignore` is rewritten** by the migration:
    - `.claude/accelerator.local.md` (anchored or unanchored
      whole-line forms) is replaced with the anchored
      `.accelerator/config.local.md` rule.
    - Legacy `meta/integrations/jira/.lock` and
      `meta/integrations/jira/.refresh-meta.json` rules are removed
      (their replacements are covered by the inner
      `.accelerator/state/integrations/jira/.gitignore`).
    - The migration refuses to rewrite a line with trailing content
      (e.g. an inline comment) and prints a reconciliation message;
      reconcile manually and re-run.
- **`init-jira` no longer mutates the project root `.gitignore`**.
  Each integration init skill now owns its own state subdirectory
  under `.accelerator/state/integrations/<tool>/` — including the
  inner `.gitignore`. This is the forward convention for all future
  integration init skills.
- **Recovery from a failed migration is via VCS revert**
  (`jj op restore` / `git reset`) followed by a re-run. The migration
  is idempotent — every step is safe to re-run from any partial
  state. The migration refuses to run on a dirty working tree, so a
  committed baseline is always available.
```

### Success Criteria

#### Automated Verification:

- [ ] Every test suite still passes
- [ ] No prose under `README.md`, `skills/`, or `hooks/` (excluding
  `meta/notes/`, `meta/research/`, `meta/plans/`, `meta/decisions/`,
  `meta/work/`, `meta/reviews/`, `CHANGELOG.md`, and migration scripts
  `0001`/`0002`) references the legacy paths
- [ ] CHANGELOG entry renders correctly in markdown preview

#### Manual Verification:

- [ ] README walkthrough is internally consistent
- [ ] configure SKILL.md remains coherent end-to-end
- [ ] CHANGELOG entry accurately describes the breaking change

---

## Testing Strategy

### Unit Tests

**Per-script idempotency and behaviour**:

- `test-init.sh` — bootstrap script idempotency, scaffold creation,
  `paths.<key>` overrides honoured, anchored gitignore rule format.
- `test-jira-init-flow.sh` — inner `.gitignore` location/rules, `.gitkeep`,
  refresh idempotency, lock cleanup, absent-`.accelerator/` warning.
- `test-config.sh` — config script path resolution against new fixtures,
  `paths.integrations` resolution.
- `test-launch-server.sh` — visualiser config defaults.
- `test-migrate-discoverability.sh` (new) — sentinel detection variants and
  state-file fallback chain.

### Integration Tests

**Migration end-to-end** (`test-migrate.sh` extensions):

- Full move scenario from a fully-seeded legacy repo to `.accelerator/`.
- Partial-state recovery (some moves done, others pending).
- Idempotency on re-run.
- Dirty-tree refusal for every sentinel path.
- `paths.tmp` default vs. override branching.
- Root `.gitignore` rewrite (anchored rule replacement, jira rule removal).

### Manual Testing Steps

1. **Fresh-repo init** — In a fresh mktemp directory: `bash scripts/init.sh`,
   then verify `.accelerator/` tree matches the work item target structure
   (excluding `state/integrations/`).
2. **Fresh-repo init-jira** — In a fresh mktemp directory with
   `.accelerator/` initialised: run `/accelerator:init-jira`, verify
   `.accelerator/state/integrations/jira/` is created with inner `.gitignore`
   and three rules.
3. **Fresh-repo init-jira without init** — Same as above but skip the init
   step; verify the stderr warning is emitted and the directory is still
   created.
4. **Migration end-to-end** — Use the test fixture
   `test-fixtures/0003/seeded-legacy-repo/` (created in Phase 3). Run
   `/accelerator:migrate`, verify all paths moved, original sources removed,
   migration recorded in `.accelerator/state/migrations-applied`.
5. **Re-run migrate** — Re-run on the migrated repo, verify "no pending
   migrations" reported.
6. **Visualiser end-to-end** — Launch visualiser on a migrated repo, confirm
   it picks up templates from `.accelerator/templates/` and writes config to
   `.accelerator/tmp/visualiser/config.json`.
7. **Discoverability hook** — On a pre-migration repo (only legacy state),
   trigger SessionStart, confirm warning emitted. On a migrated repo, confirm
   warning suppressed once `0003` is recorded.

## Performance Considerations

None expected. The migration moves files via `mv` (atomic on the same
filesystem) and writes a small number of small files. No hot-path scripts
gain or lose work. Driver state-file location change is one syscall.

## Migration Notes

- The migration is a hard cut: runtime scripts target only
  `.accelerator/` paths after this PR merges. Existing repos must run
  `/accelerator:migrate` before any other accelerator skill works
  correctly.
- **Recovery posture**: the migration script is idempotent — every
  step is safe to re-run from any partial-recovery state. There is no
  preflight, dry-run, or rollback machinery. The recovery path on
  failure is VCS revert (`jj op restore` / `git reset`) followed by a
  re-run; the driver's clean-tree refusal enforces a committed
  starting state, so revert is always available. Users without
  committed VCS state are out of scope — they should commit (to either
  `jj` or `git`) before running the migration. The conflict branch of
  the move primitive (`_move_if_pending`) is the only path that exits
  non-zero during the run, and it does so without making any
  destructive change.
- The discoverability hook (the only fallback retained) ensures
  pre-migration users see a clear warning at SessionStart prompting
  them to run the migration. A complementary legacy-path detector
  inside `config-common.sh` (added in Phase 6) catches the case where
  a user invokes a skill mid-session before running migrate, exiting
  non-zero with the same directive.
- `paths.tmp` explicit-override preservation prevents data loss for
  users who have pinned `paths.tmp` to **any** value, including the
  legacy default `meta/tmp` literal. The probe uses an awk-based
  parse of the legacy config files (column-0-`paths:`-anchored), not
  the runtime config layer that the migration is itself rewiring.
  Users who pinned `paths.templates` or `paths.integrations` to a
  `meta/<dir>` value have their `meta/<dir>` moved unconditionally
  and must update their config post-migration; the CHANGELOG entry
  flags this caveat alongside a detect-and-warn notice the migration
  emits when these keys are explicitly set.
- The driver, discoverability hook, and migration script ship in the
  same commit (Phase 3) — atomic delivery is mandatory per work-item
  Dependencies.

## References

- Original work item: `meta/work/0031-consolidate-accelerator-owned-files-under-accelerator.md`
- Research: `meta/research/2026-05-05-0031-consolidate-accelerator-owned-files.md`
- Source proposal: `meta/notes/2026-04-29-accelerator-config-state-reorg.md`
- Related: ADR-0016 (`meta/decisions/ADR-0016-userspace-configuration-model.md`)
- Related: ADR-0017 (`meta/decisions/ADR-0017-configuration-extension-points.md`)
- Related: ADR-0019 (`meta/decisions/ADR-0019-ephemeral-file-separation-via-paths-tmp.md`)
- Related: ADR-0020 (`meta/decisions/ADR-0020-per-skill-customisation-directory.md`)
- Related: ADR-0023 (`meta/decisions/ADR-0023-meta-directory-migration-framework.md`)
- Test conventions: `scripts/test-helpers.sh`,
  `skills/config/migrate/scripts/test-migrate.sh:7-21` (harness bootstrap),
  `skills/config/migrate/scripts/test-migrate.sh:289-315` (dirty-tree pattern)
- Migration template: `skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh`
  (`MIGRATION_RESULT: no_op_pending` sentinel pattern)
