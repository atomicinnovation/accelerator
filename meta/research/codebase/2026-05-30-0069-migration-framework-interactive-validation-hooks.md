---
date: 2026-05-30T11:01:43+01:00
researcher: Toby Clemson
git_commit: 4c801749a9b8cb2d5c7f995d65914f7dc4411dbc
branch: HEAD
repository: accelerator
topic: "Migration framework interactive validation hooks — implementation surface for story 0069"
tags: [research, codebase, migration-framework, interactive-hooks, accelerator-plugin, adr-0037, adr-0038]
status: complete
last_updated: 2026-05-30
last_updated_by: Toby Clemson
---

# Research: Migration framework interactive validation hooks — implementation surface for story 0069

**Date**: 2026-05-30 11:01:43 BST
**Researcher**: Toby Clemson
**Git Commit**: 4c801749a9b8cb2d5c7f995d65914f7dc4411dbc
**Branch**: HEAD
**Repository**: accelerator

## Research Question

What is the current state of the Accelerator meta-directory migration framework, and what concrete implementation surface does story 0069 need to extend to implement ADR-0037's opt-in interactive contract (with ADR-0038's parameterisation as the first consumer)?

The story (`meta/work/0069-migration-framework-interactive-validation-hooks.md`) names: the trigger-predicate plumbing, runner-surfaced display, accept/edit/skip controls, the resumability mechanism (session log + resume state), the transformation ordering invariant, the transformation key schema, source-drift behaviour, and SKILL.md documentation updates. This research maps each of those into specific lines, helpers, and conventions in the live codebase.

## Summary

**The runner today is a 220-line bash script** (`skills/config/migrate/scripts/run-migrations.sh`) implementing the ADR-0023 mechanical contract: discover, preview, apply, ledger-append. It has no internal function boundaries — extension means inserting code into one of three natural splice points or sourcing a new helper.

**Two existing migration-author conventions provide the obvious extension precedent**: (1) the `# DESCRIPTION:` line-2 comment scraped for preview text, and (2) the `MIGRATION_RESULT: no_op_pending` stdout sentinel. Story 0069's hook declaration should slot in symmetrically — either as a new sentinel namespace, additional header comments, or (likelier) by sourcing a per-migration declaration file written by the migration author.

**Per-migration invocation today** is `bash "$f" >"$STDOUT_FILE" 2>&1` (run-migrations.sh:184) with `PROJECT_ROOT`, `CLAUDE_PLUGIN_ROOT`, `ACCELERATOR_MIGRATION_MODE=1` env vars and no positional args. The output is buffered to a tempfile, scanned for the sentinel, then forwarded to stderr. This is the boundary the interactive contract has to traverse: the runner currently has no way to converse with the running migration mid-execution. **Story 0069 will need to change the process model** — either by inverting it (runner drives, migration is a sourced library declaring the predicate, transformations, validator, and display fields) or by introducing a structured stdin/stdout protocol.

**Sourced helpers already exist** for atomic file writes (`scripts/atomic-common.sh`: `atomic_write`, `atomic_append_unique`, `atomic_remove_line`), which is exactly the primitive the session-log incremental writes need (ADR-0037 §3 guarantee 1). The session-log path convention `migrations-<migration-id>-session.jsonl` (ADR-0038) is consistent with the existing `migrations-applied` / `migrations-skipped` placement under `.accelerator/state/`.

**Test infrastructure is a single bash harness** (`scripts/test-migrate.sh`) that copies fixtures from `scripts/test-fixtures/<NNNN>/` into a `mktemp -d` repo, fakes VCS via `mkdir .git`, seeds the applied-ledger to gate earlier migrations, runs the driver, and asserts inline. There is **no per-migration golden-output tree** — assertions are inline `assert_contains` / `assert_file_exists` calls. Story 0069's mechanical-path snapshot test (AC-1) will need new infrastructure: either inline diff assertions per migration, or a new golden-output convention.

**The mechanical path can be preserved verbatim** by gating all new behaviour on a migration-declared opt-in signal — the existing apply loop (run-migrations.sh:179-208) only needs an `if migration_declares_hook` branch added. The runner is confidence-agnostic (ADR-0037 §1, reaffirmed by story Technical Notes) — bands, thresholds, predicates are entirely migration concerns. The runner only evaluates whatever predicate the migration declares.

## Detailed Findings

### 1. Runner anatomy and splice points

The runner is `skills/config/migrate/scripts/run-migrations.sh` (one file, 220 lines, no internal functions, `set -euo pipefail`). It executes in nine numbered sections:

1. Bootstrap and source helpers (run-migrations.sh:4-14)
2. Subcommand dispatch (`--skip` / `--unskip`) (run-migrations.sh:17-39)
3. Pre-flight clean-tree check (run-migrations.sh:42-70)
4. Read both ledgers into bash arrays (run-migrations.sh:73-85)
5. Discover migration files (run-migrations.sh:88-93)
6. Cross-validate state, warn on anomalies (run-migrations.sh:96-131)
7. Compute pending set (run-migrations.sh:134-148)
8. Print preview banner (run-migrations.sh:162-175)
9. Apply loop (run-migrations.sh:179-208) + summary line (run-migrations.sh:211-220)

**Splice points for the interactive contract**:

- **Per-migration invocation (run-migrations.sh:181-208)** — the natural home for "evaluate predicate, route to prompt or mechanical path". The current `bash "$f"` is the boundary that must change.
- **Per-migration result classification (run-migrations.sh:192)** — currently parses one sentinel (`MIGRATION_RESULT: no_op_pending`). The `MIGRATION_RESULT:` namespace is the obvious extension surface.
- **Subcommand dispatch (run-migrations.sh:17-39)** — if hook-related verbs are needed (e.g. `--resume <id>`, `--inspect-session-log <id>`), they slot into the existing `case "$1" in` block.

**Extension caveats**:

- No internal functions means new logic must either be inline or factored into a new sourced library file (no precedent for `skills/config/migrate/lib.sh` today — shared logic lives in top-level `scripts/`).
- `STDOUT_FILE` is `rm -f`'d eagerly on both success and failure paths (run-migrations.sh:186, 197); preserving captured output for inspection needs care.
- `set -e` regime means new helpers must explicitly wrap non-zero returns (`if !` pattern is the existing idiom).

### 2. Existing ledger and atomic-write primitives

The runner already has all the primitives the session-log incremental-write guarantee needs:

- `STATE_FILE="$PROJECT_ROOT/.accelerator/state/migrations-applied"` (run-migrations.sh:13)
- `SKIP_FILE="$PROJECT_ROOT/.accelerator/state/migrations-skipped"` (run-migrations.sh:14)
- Format: newline-delimited migration IDs, one per line; absent file = empty set
- Writes go through `atomic_write` (`scripts/atomic-common.sh:16-32`) — temp-file-then-rename in same directory, with EXIT-trap temp-file cleanup
- `atomic_append_unique` (`scripts/atomic-common.sh:38-61`) — short-circuits on `grep -Fxq`; perfect for "append migration ID once on full completion"
- `atomic_remove_line` (`scripts/atomic-common.sh:66-80`) — `grep -Fxv` then `atomic_write`; relevant if source-drift behaviour requires removing stale records (note: story specifies *replace*, not remove-then-append, so this may not be needed)

ADR-0038 specifies the session-log path convention as `.accelerator/state/migrations-<migration-id>-session.jsonl` and JSONL format with per-record schema `{artifact_path, source_anchor, inferred_key, inferred_target, band, decision, edited_key?, edited_target?}` appended atomically. Story 0069 generalises the schema (`{transformation_key, outcome, proposed_value, [user_value], timestamp}`) — the migration-declared form takes precedence at the framework level; ADR-0038's schema is what the unified-schema migration actually emits.

The runner currently uses `mkdir -p "$(dirname "$STATE_FILE")"` belt-and-braces before ledger writes (run-migrations.sh:24, 204). Session-log writes should follow the same convention.

### 3. Migration discovery and ID conventions

Migration discovery at run-migrations.sh:88-93:

```bash
MIGRATIONS_DIR="${ACCELERATOR_MIGRATIONS_DIR:-$PLUGIN_ROOT/skills/config/migrate/migrations}"
find "$MIGRATIONS_DIR" -maxdepth 1 -name '[0-9][0-9][0-9][0-9]-*.sh' -print0 | sort -z
```

Conventions:

- Location: `skills/config/migrate/migrations/` (overridable via `ACCELERATOR_MIGRATIONS_DIR` — used by tests)
- Filename: `NNNN-<slug>.sh` (four digits + dash + name + `.sh`)
- ID = basename without `.sh`
- Sort = lexical NUL-delimited, gives deterministic numeric ordering
- Line 2 must be `# DESCRIPTION: <short imperative description>` — consumed by the preview banner

**Implications for story 0069**:

- The transformation ordering invariant (story requirement) aligns with the existing emission-order convention — the runner already processes in deterministic order.
- The session-log path convention `migrations-<migration-id>-session.jsonl` interpolates the migration ID = basename — already trivially derivable from the file path the runner is processing.
- New header comments (e.g. `# REQUIRES_INTERACTIVE: yes`) could be scraped the same way `# DESCRIPTION:` is.

### 4. Existing migration-author interface (the "corpus")

Six migrations exist today, all under `skills/config/migrate/migrations/`:

- `0001-rename-tickets-to-work.sh`
- `0002-rename-work-items-with-project-prefix.sh`
- `0003-relocate-accelerator-state.sh`
- `0004-restructure-meta-research-into-subject-subcategories.sh`
- `0005-rename-work-item-type-to-kind.sh`
- `0006-canonicalise-work-item-id-and-author.sh`

All honour this contract:

| Surface | Provided by |
|---|---|
| **Filename** | `NNNN-<slug>.sh` |
| **Metadata** | `# DESCRIPTION: <one-line>` on line 2 |
| **Shebang/opts** | `#!/usr/bin/env bash` + `set -euo pipefail` |
| **Inputs (env)** | `PROJECT_ROOT`, `CLAUDE_PLUGIN_ROOT`, `ACCELERATOR_MIGRATION_MODE=1` |
| **Inputs (cli)** | None |
| **Mutation style** | Read-modify-write to `.tmp` + `mv` (0001) or `atomic_write` (0005) |
| **Outputs** | stdout = user-visible summary; stderr = warnings/errors |
| **Exit codes** | `0` = applied; `0` + `MIGRATION_RESULT: no_op_pending` = stays pending; non-zero = abort |
| **Idempotency** | Self-detect already-migrated state before mutating |

**The `MIGRATION_RESULT: no_op_pending` sentinel** (visible in `0002-rename-work-items-with-project-prefix.sh:if-block`) is the existing precedent for migration→runner structured communication. Story 0069's hook declaration is a natural extension of this surface — but unlike `no_op_pending` which is one-shot at process end, interactive hooks need a bidirectional channel per transformation, which `MIGRATION_RESULT:` lines on stdout don't provide.

**The plausible options for the interactive contract's process model**:

1. **Sourced library / inverted control**: the migration script becomes a library declaring functions (`migration_emit_transformations`, `migration_evaluate_predicate`, `migration_validate_edit`, etc.) that the runner sources and calls. Pros: clean, bash-idiomatic, lets the runner drive the prompt loop. Cons: breaks the existing "child bash process" isolation; existing mechanical migrations would either need a thin shim or the runner would dual-mode based on whether the migration declares the hook.
2. **Coprocess / structured stdin-stdout protocol**: the runner spawns the migration with bidirectional pipes; migration emits `EMIT_TRANSFORMATION: <json>`, reads `DECIDE: <accept|edit|skip>` on stdin. Pros: preserves child-process isolation. Cons: more code; bash coprocesses are fiddly.
3. **Sidecar declaration file**: alongside `NNNN-<slug>.sh`, a sibling `NNNN-<slug>.interactive.sh` (or `.toml`/`.json`) declaring predicate + display fields + validator entry points. The main script remains the mutation driver. Pros: cleanly opt-in. Cons: a new file convention to maintain.

This research does not pick — that's an implementation-planning concern. But Option 1 (sourced library) maps most naturally to bash idioms and would minimise the diff against the existing runner.

### 5. SKILL.md current shape and update surface

`skills/config/migrate/SKILL.md` is 104 lines, organised:

- Frontmatter (lines 1-9): `name`, `description`, `allowed-tools: Read, Write, Edit, Bash`. No `model-invocation` key.
- Preamble warning block (line 11): destructive-by-default + VCS-revert framing
- `## When to invoke` (lines 13-25)
- `## How it works` (lines 27-37) — 7-step lifecycle of the runner
- `## Per-migration contract` (lines 39-48) — MUST/MAY bullet list
- `## State file format` (lines 50-65) — both ledgers documented
- `## Skip-tracking` (lines 67-81) — `--skip` / `--unskip` invocations
- `## MIGRATION_RESULT contract` (lines 83-87) — `no_op_pending` sentinel
- `## Executing the migration` (lines 89-97)
- `## Cross-references` (lines 99-103)

**Conventions**:

- ADRs referenced by full filename path (`meta/decisions/ADR-0023-migration-framework.md — framework design rationale`), not bare ID, in `## Cross-references` only
- Inline file references in backticks without markdown link syntax
- Three fenced text blocks (SessionStart hook example, ledger examples); three fenced bash blocks (commands)
- No worked end-to-end examples or transcripts today
- Tone: specification-heavy, MUST-style bullets, terse paragraphs

**Where new documentation slots in**:

- Story 0069 requires: hook declaration mechanics, three runner guarantees from ADR-0037 §§1-4, source-drift behaviour, transformation ordering invariant, transformation key schema, worked example with sample prompt transcript and session-log excerpt.
- Most natural new section: **between `## MIGRATION_RESULT contract` (ends line 87) and `## Executing the migration` (starts line 89)** — or expand `## MIGRATION_RESULT contract` into a sibling family. The current "## Per-migration contract" section's MAY bullet at line 48 (`MAY emit MIGRATION_RESULT`) is exactly the kind of opt-in protocol hooks would resemble, so an additional bullet there pointing to a new dedicated section is consistent.
- Worked example needs new infrastructure — there's no precedent in the file. The CI test that asserts the example doesn't drift (story AC-13) is novel for this skill.

### 6. Test infrastructure and the snapshot-test challenge (AC-1)

Single test harness: `skills/config/migrate/scripts/test-migrate.sh`. Pattern (from the harness, lines ~506-520 per pattern finder):

- Copies fixtures from `scripts/test-fixtures/<NNNN>/` into a `mktemp -d` repo
- Fakes VCS state via `mkdir .git`
- Seeds `.accelerator/state/migrations-applied` to gate earlier migrations
- Runs `bash run-migrations.sh`
- Asserts inline using `assert_contains` / `assert_file_exists`-style helpers
- No per-migration golden-output trees

Fixture inventory:

- `test-fixtures/0002/` — single fixture
- `test-fixtures/0003/` — single fixture
- `test-fixtures/0004/` — 7 scenarios (`all-overridden`, `default-layout`, `inbound-corpus`, `local-config-only`, `mixed-config`, `partial-state`, `research-override-only`)
- `test-fixtures/0005/` — 10 scenarios (`body-label-only` through `paths-override-missing`)
- `test-fixtures/0006/` — 37 scenarios (the most extensive set — `body-label-anchored-no-h2`, `body-label-multiple`, etc.)
- No fixtures for migration `0001`

**Implications for story 0069's AC-1 "Mechanical path unchanged" snapshot test**:

- AC-1 says: "running the framework against every migration present at HEAD under `skills/config/migrate/scripts/` that does not declare a hook produces byte-identical migrated artefacts and the same exit code as the pre-change runner, verified by a snapshot test that diffs both runs against the same input fixture set (the per-migration golden inputs under each migration's existing test directory)."
- **The "golden inputs" referenced don't exist as separate trees today** — the fixtures are inputs, and assertions are inline (not golden outputs on disk). The snapshot test will need to either (a) capture outputs from a baseline run of the pre-change runner against each fixture, save those as golden trees, then diff post-change runs against them — i.e. introduce a new golden-tree convention; or (b) keep the test inline-assertion-based but run the full existing `test-migrate.sh` against both code paths and diff intermediate states.
- Option (a) is cleaner and survives future migrations being added.

### 7. ADR contract surfaces (binding requirements)

**ADR-0037 (framework contract; status accepted 2026-05-26)**:

- §1 trigger predicate — boolean over migration-declared field set, must admit one confidence-valued field; runner supports uniform and hybrid application.
- §2 runner-surfaced display — mandatory three: proposed transformation, source location, predicate's evaluated value; migrations may declare extras.
- §3 resumability — three guarantees: incremental write per transformation, re-entry reconstructs resume state before prompting, full completion appends migration ID to `.accelerator/state/migrations-applied`. Persistence mechanism implementer-chooses.
- §4 accept/edit/skip — three controls each defined by (i) artifact effect and (ii) session-log effect; accept-degraded is a special case of edit (not a separate verb).
- §5 recursive supplement clause — future extensions must be new supplementary ADRs.

**Left to story 0069 (runner-level, not framework primitives)**:
- Persistence mechanism format choice (JSON checkpoint vs append-only line-delimited; choose one and document in SKILL.md).
- Process/invocation model (in-process callback vs subprocess vs coprocess) — researcher's recommendation in section 4 above is sourced library / inverted control.
- Prompt sequencing and rendering.
- Source-drift behaviour (story already decides: re-prompt with new proposed value; discard old record).
- Transformation ordering invariant (story already decides: emission order).
- Transformation key schema (story already decides: migration-supplied ID, or `artifact_path + structural_anchor`).

**ADR-0038 (first consumer's parameterisation; status accepted 2026-05-26)**:

- Two-band predicate: `band == 'ambiguous'`.
- Hybrid application — `resolved` band applies mechanically.
- Declared display fields (beyond the framework three): prose line, section heading, alternative linkage keys, on-disk-resolution status; on `edit` invocation, full ADR-0034 v1 linkage-key menu surfaced.
- Edit mutation forms: linkage-key correction and/or target-identifier correction; migration validates against ADR-0034 vocabulary and ADR-0033 identity-value contract.
- Session log: `.accelerator/state/migrations-<migration-id>-session.jsonl`, JSONL format, schema `{artifact_path, source_anchor, inferred_key, inferred_target, band, decision, edited_key?, edited_target?}`.
- Three parser fixes incorporated before band classifier runs.

**Story 0069's runner is the contract-implementation layer**; ADR-0038's values are the consumer's parameterisation and live in the migration script (0070), not the runner. The runner only needs to support the *shape* (named field set including confidence-valued field; declared extras; declared validator; declared session-log path/format).

### 8. Discoverability hook (adjacent, out-of-scope but worth noting)

`hooks/migrate-discoverability.sh` (registered in `hooks/hooks.json`) is a SessionStart hook that prompts about pending migrations. Tested by `hooks/test-migrate-discoverability.sh`. This is the hook that fires the warning text shown verbatim in `SKILL.md` lines 17-21 (the fenced block under `## When to invoke`). Story 0069 does **not** need to modify this — the discoverability hook fires *before* the runner is invoked, and the interactive flow happens inside the runner.

### 9. Dependent and consuming work items

- **0068 (spike, done)**: measured 11.3% wrong-rate, 5.3% cheap-fix counterfactual — both above the 5% deterministic threshold. Verdict: interactive hooks. Removes any conditionality from 0069's existence.
- **0070 (consumer, draft)**: ships the unified-schema corpus migration using this runner extension. Notable additional needs not in 0069's ACs:
  - **File-level skipping** for `meta/notes/` (0070 OQ1) — 0069's ACs only enumerate per-transformation accept/edit/skip; file-level skipping isn't explicitly covered. Possibly resolvable as a migration-side concern (the migration declares zero transformations for those files) without runner change.
  - **`last_updated` policy on first-time set** (0070 OQ2) — migration concern, not runner.
  - **Visualiser fallback removal** — out of 0069 scope.
- **0092 (done)**: produced ADR-0037; 0069 implements, not re-decides.
- **0062 (done)**: produced ADR-0038; 0069 receives the parameterised values.
- **0057 (parent epic, in-progress)**: 0069 is one of its remaining stories.

## Code References

### The runner (target of extension)

- `skills/config/migrate/scripts/run-migrations.sh:2` — `set -euo pipefail` strict mode
- `skills/config/migrate/scripts/run-migrations.sh:6-8` — sources `scripts/config-common.sh` and `scripts/atomic-common.sh`
- `skills/config/migrate/scripts/run-migrations.sh:13-14` — `STATE_FILE` / `SKIP_FILE` ledger paths
- `skills/config/migrate/scripts/run-migrations.sh:17-39` — subcommand dispatch (`--skip` / `--unskip`)
- `skills/config/migrate/scripts/run-migrations.sh:42-70` — clean-tree pre-flight (jj + git detection)
- `skills/config/migrate/scripts/run-migrations.sh:73-85` — ledger reads into bash arrays
- `skills/config/migrate/scripts/run-migrations.sh:88-93` — migration discovery
- `skills/config/migrate/scripts/run-migrations.sh:96-131` — state cross-validation, warnings
- `skills/config/migrate/scripts/run-migrations.sh:134-148` — pending set computation
- `skills/config/migrate/scripts/run-migrations.sh:162-175` — preview banner construction
- `skills/config/migrate/scripts/run-migrations.sh:179-208` — **apply loop (primary splice point)**
- `skills/config/migrate/scripts/run-migrations.sh:184` — per-migration `bash "$f"` invocation
- `skills/config/migrate/scripts/run-migrations.sh:192-202` — `MIGRATION_RESULT: no_op_pending` sentinel handling
- `skills/config/migrate/scripts/run-migrations.sh:204-207` — atomic ledger append on success
- `skills/config/migrate/scripts/run-migrations.sh:211-220` — summary line

### Sourced helpers (already provide session-log primitives)

- `scripts/atomic-common.sh:16-32` — `atomic_write` (temp-then-rename)
- `scripts/atomic-common.sh:38-61` — `atomic_append_unique` (idempotent line append)
- `scripts/atomic-common.sh:66-80` — `atomic_remove_line`
- `scripts/config-common.sh:40,55` — `config_find_files` and `config_assert_no_legacy_layout` (use `ACCELERATOR_MIGRATION_MODE`)

### Existing corpus (fixture targets for AC-1 snapshot)

- `skills/config/migrate/migrations/0001-rename-tickets-to-work.sh`
- `skills/config/migrate/migrations/0002-rename-work-items-with-project-prefix.sh` (uses `MIGRATION_RESULT: no_op_pending` sentinel)
- `skills/config/migrate/migrations/0003-relocate-accelerator-state.sh`
- `skills/config/migrate/migrations/0004-restructure-meta-research-into-subject-subcategories.sh`
- `skills/config/migrate/migrations/0005-rename-work-item-type-to-kind.sh` (canonical `atomic_write` user)
- `skills/config/migrate/migrations/0006-canonicalise-work-item-id-and-author.sh` (most extensive fixture set)

### Test infrastructure

- `skills/config/migrate/scripts/test-migrate.sh` — single harness
- `skills/config/migrate/scripts/test-fixtures/0002/` through `0006/` — per-migration fixture trees (37 scenarios alone under 0006)

### Documentation

- `skills/config/migrate/SKILL.md` — 104 lines, lines 39-48 (`## Per-migration contract`) and 83-87 (`## MIGRATION_RESULT contract`) are the natural neighbours for new hook documentation

### ADRs (binding contracts)

- `meta/decisions/ADR-0037-optional-interactive-contract-supplement-to-adr-0023.md` — primitives this story implements
- `meta/decisions/ADR-0038-interactive-validation-parameters-for-unified-schema-linkage-migration.md` — first consumer's parameter values
- `meta/decisions/ADR-0023-meta-directory-migration-framework.md` — mechanical-default contract (supplemented, not edited)
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md` — explains why supplement form

## Architecture Insights

1. **Mechanical-default preservation is structurally cheap**. The runner already has `MIGRATION_RESULT:`-style branching at the per-migration result-classification step. Adding `if migration_declares_hook` at the per-migration invocation step (run-migrations.sh:181) gates all new behaviour behind opt-in without changing the existing path.

2. **Atomic-write helpers are the right primitive for session-log incremental writes**. `atomic_write` (temp-then-rename, same-directory, EXIT-trap cleanup) matches ADR-0037 §3 guarantee 1 (incremental write per transformation) and ADR-0023's atomic-state-write convention exactly. ADR-0038 specifies JSONL append; `atomic_append_unique` over a JSONL file is suspect (the line-uniqueness criterion is the whole record string), so the session-log append likely needs a new helper or a simpler `>> "$session_log_file"` with careful flush semantics.

3. **Process-model change is the deepest architectural decision**. The current "fork bash, capture stdout, scan for sentinel" model can't host bidirectional per-transformation conversation. Either the runner sources migrations as libraries (recommended; minimises diff and matches bash idioms) or introduces a structured stdin-stdout protocol with the migration as coprocess. This is *not* settled by ADR-0037 (which says "implementer chooses invocation model") and should be a planning-time decision in the implementation plan.

4. **The transformation key schema is well-aligned with existing conventions**. `artifact_path + structural_anchor` matches the spike's parser output shape and ADR-0038's `{artifact_path, source_anchor}` resume-match key. Migration-supplied IDs are an optional override. No tension with current code.

5. **`MIGRATION_RESULT:` is a one-shot end-of-process sentinel, not a conversation protocol**. Story 0069's hook declaration needs a different surface — header comments like `# REQUIRES_INTERACTIVE:` are more honest (declared at script load time) than a runtime sentinel.

6. **The runner is confidence-agnostic by design**. Bands, thresholds, predicate shape are migration concerns parameterised in per-migration ADRs (e.g. ADR-0038). The runner only evaluates whatever predicate the migration declares — implementation should resist the temptation to bake in band-related knowledge.

7. **Resume-state reconstruction is a re-read of the session log into memory; no separate index file is needed**. The session log IS the persistence; resume state is a derived in-memory shape (a hash from transformation_key → recorded outcome). Story 0069's terminology ("session log" for on-disk, "resume state" for in-memory) is precise and worth preserving in code and docs.

8. **The single-bash test harness pattern needs extension for interactive testing**. Driving accept/edit/skip from a non-interactive test requires either input redirection (`echo "accept\nedit\nfoo\n" | bash run-migrations.sh ...`) or a `--non-interactive-decisions <file>` runner flag. The harness currently has no precedent for piping stdin.

## Historical Context

- **ADR-0023** (2026-04-25, accepted) established the mechanical-default contract. Story 0069's preservation of "mechanical path unchanged" (AC-1) honours this directly. The state-file schema, clean-tree pre-flight, and one-line preview are all from ADR-0023.
- **ADR-0031** (2026-03-18, accepted) makes accepted ADRs immutable, forcing ADR-0037 to take supplement form. No direct runner impact but explains the three-ADR chain (ADR-0023 → ADR-0037 → per-migration ADRs).
- **ADR-0033 and ADR-0035** are cited as the supplement-form precedent (supplements to ADR-0028 and ADR-0026 respectively).
- **Spike 0068** (`meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`, done) measured the 11.3% wrong-rate and 5.3% cheap-fix counterfactual — both above the 5% threshold. Cited as the verdict that activated 0069.
- **Research 2026-05-26-0092** (`meta/research/codebase/2026-05-26-0092-adr-optional-interactive-contract-for-migration-framework.md`) — the research backing ADR-0037's design.
- **Research 2026-05-26-0062** (`meta/research/codebase/2026-05-26-0062-adr-interactive-validation-for-corpus-migration.md`) — the research backing ADR-0038's parameter choices.
- **Review document** (`meta/reviews/work/0069-migration-framework-interactive-validation-hooks-review-1.md`) — prior pass-1 review notes on 0069 itself.

## Related Research

- `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md` — spike measurements
- `meta/research/codebase/2026-05-26-0092-adr-optional-interactive-contract-for-migration-framework.md` — ADR-0037 design research
- `meta/research/codebase/2026-05-26-0062-adr-interactive-validation-for-corpus-migration.md` — ADR-0038 design research
- `meta/research/codebase/2026-05-05-0031-consolidate-accelerator-owned-files.md` — earlier research on the `.accelerator/state/` consolidation that produced migration 0003

## Open Questions

These are not blockers — they are choices the implementation plan needs to make, not contract gaps:

1. **Process / invocation model**: sourced library (recommended) vs coprocess vs sidecar declaration file. ADR-0037 explicitly leaves this to the implementer. Recommend deciding this in the implementation plan rather than during implementation.

2. **Snapshot-test approach for AC-1**: capture golden-output trees from a baseline runner run (new infrastructure) vs diff the inline-assertion behaviour of `test-migrate.sh` against pre- and post-change runners. Story AC-1 implies the former ("diffs both runs against the same input fixture set") but the existing test infrastructure assumes the latter.

3. **Session-log write helper**: extend `atomic-common.sh` with a JSONL-aware `atomic_jsonl_append` or use plain `>>` with explicit `sync`? The ADR-0037 §3 guarantee 1 ("incremental write per transformation") demands that the durability semantics be explicit. A SIGKILL-durability test is in the ACs, so the write helper must survive process death after the line is committed.

4. **Stdin protocol for testing**: how does the test harness simulate accept/edit/skip decisions non-interactively? Either a `--decisions <file>` runner flag or stdin redirection. The choice affects the public CLI surface.

5. **Worked-example placement and drift test (AC-13)**: the SKILL.md worked example must be CI-tested for drift. Need a small fixture migration that lives under `scripts/test-fixtures/` plus a doc-extraction step that asserts the transcript in SKILL.md matches a recorded run. No precedent in the repo for this pattern.

6. **File-level skipping** (0070 dependency): 0070's `meta/notes/` treatment may need file-level skip support that 0069 doesn't currently provide. Resolvable as a migration-side concern (declare zero transformations for those files) but worth confirming with 0070 before implementing.
