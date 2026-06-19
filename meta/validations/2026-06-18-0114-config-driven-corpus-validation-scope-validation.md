---
type: plan-validation
id: "2026-06-18-0114-config-driven-corpus-validation-scope-validation"
title: "Validation Report: Config-Driven Corpus Validation Scope Implementation Plan"
date: "2026-06-19T15:04:16+00:00"
author: "Toby Clemson"
producer: validate-plan
status: complete
result: "pass"
parent: "plan:2026-06-18-0114-config-driven-corpus-validation-scope"
target: "plan:2026-06-18-0114-config-driven-corpus-validation-scope"
tags: [validator, frontmatter, config, doc-type-inference, unified-schema, allowlist]
last_updated: "2026-06-19T15:04:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Validation Report: Config-Driven Corpus Validation Scope Implementation Plan

### Implementation Status

✓ Phase 1: Canonical type→path-key registry + resolver — Fully implemented
✓ Phase 2: Config-injected allowlist in shared classifier (backward-compatible) — Fully implemented
✓ Phase 3: Wire the validator to the allowlist — Fully implemented
✓ Phase 4: Wire the 0007 migration to the allowlist — Fully implemented
✓ Phase 5: Remove the transitional fallback and dead denylist — Fully implemented
✓ Phase 6: Config-aware awk `path_to_typed` — Fully implemented

Each phase maps to a discrete commit:

- `yrmvyukr` Add type→path-key registry and doc-type directory resolver
- `mklmrvln` Add config-injected allowlist mode to doc-type classifier
- `tymootmk` Wire the corpus validator to the config-driven allowlist
- `roslrtrl` Wire the 0007 migration to the config-driven allowlist
- `lpuruksn` Remove the transitional fallback and dead denylist from the classifier
- `oonklmpo` Make the 0007 awk path_to_typed classifier config-aware

Working copy is clean; all six phases are committed.

### Automated Verification Results

✓ `bash scripts/test-config-read-doc-type-paths.sh` — all tests passed (Phase 1)
✓ `bash scripts/test-doc-type-inference.sh` — all tests passed (Phases 2, 5)
✓ `bash scripts/test-validate-corpus-frontmatter.sh` — all tests passed (Phase 3)
✓ `bash skills/config/migrate/scripts/test-migrate-0007.sh` — all tests passed (Phases 4, 6); includes "doc-type table resolved exactly once per run" trace check
✓ `mise run test:integration:config` — PASS (the originally-failing task, now green without the band-aid)
✓ `mise run lint:scripts:check` — exit 0 (shfmt + ShellCheck + bashisms, bash 3.2 floor)
✓ `scripts/validate-corpus-frontmatter.sh "$(pwd)/meta"` — exit 0 on the real corpus

### Code Review Findings

#### Matches Plan:

- **Type→path-key registry** (`scripts/config-defaults.sh:74-86`): `DOC_TYPE_NAMES`
  (13 entries) and `DOC_TYPE_PATH_KEYS` parallel arrays, index-coupled, exactly
  as specified. The schema TSV carries 13 types (14 lines incl. header), matching
  the registry; coherence is pinned by a drift test.
- **Resolver** (`scripts/config-read-doc-type-paths.sh`): emits `type\tdir` TSV;
  accepts an optional project-root and resolves config inside a
  `( cd "$root" && … )` subshell (Phase 1 §2 mechanism); coerces blank values to
  the registry default with a stderr note; rejects `..`/leading-`/`/`.`/empty and
  tab/newline values; normalises each dir (`tr -s '/'`, strip `./` and trailing
  `/`) with no `${var//}` (bash-3.2 safe).
- **Shared `load_doc_type_table`** (`scripts/doc-type-table.sh`): owns the parse,
  the resolve-once invariant, and the `DOC_TYPE_TABLE_INJECTED` sentinel in one
  place; returns non-zero on resolver failure or zero rows so callers fail closed.
- **Injected-table matching** (`scripts/doc-type-inference.sh:45-74`): prefix-
  agnostic `*/"$d"/* | "$d"/*` with quoted `$d` and trailing-`/` segment boundary;
  longest-match-wins by integer `${#d}`; deterministic first-in-array tiebreak;
  distinct `DOC_TYPE_INJECTED_*` array names; explicit `DOC_TYPE_TABLE_INJECTED`
  sentinel. Header rewritten to document the injected-dependency precondition and
  fail-closed contract, replacing the stale "pure functions" framing.
- **Validator wiring** (`scripts/validate-corpus-frontmatter.sh:40-58`): resolves
  + injects once at top-level scope before `build_index`; aborts loudly if
  resolution fails; preserves the `DOC_TYPE_TABLE`/`DOC_TYPE_INFERENCE` override
  seams.
- **Migration wiring** (`0007-...sh:34-48`): canonicalises `PROJECT_ROOT`
  (`pwd -P`), injects via `load_doc_type_table "$PROJECT_ROOT"`; fail-closed guard
  at `:757-760` sits right after `fm_assert_schema_columns` and before
  `precondition_prepass`; self-validators run the spawned validator under
  `( cd "$PROJECT_ROOT" && … )` (`:565,571`) so the self-check observes the same
  scope that drove the mutation.
- **Phase 5 cleanup**: `grep 'specs\|talks\|announcements'` over
  `doc-type-inference.sh` returns nothing — the denylist and the
  `*/meta/announcements/*` band-aid are fully retired; the only surviving
  `announcements` token is an explanatory comment in the validator listing
  allowlist examples, not a skip-set entry.
- **Phase 6 awk** (`0007-frontmatter-rewrite.awk`): `path_to_typed` now reads a
  `-v doc_type_table` channel parsed in `BEGIN`; records joined by `0x1E` (not
  newline — one-true-awk rejects a newline in a `-v` value) and split on a single
  literal separator (POSIX/BWK-safe, no `gensub`/`asort`); id-derivation halves
  preserved.

#### Deviations from Plan:

- The shared populate helper lives in its own file `scripts/doc-type-table.sh`
  rather than being inlined "alongside the resolver." This is a cleaner
  single-source factoring than the plan's loose wording implied and is sourced by
  both consumers — a faithful realisation of the Phase 1 §2 intent, not a
  functional deviation.

No behavioural deviations from the plan were found.

#### Potential Issues:

- None blocking. The accepted, documented limitation stands: migration
  self-validation shares the injected scope, so it cannot independently catch a
  *wrong-but-non-empty* scope; this is mitigated at source by the resolver's
  path-safety rejection and the non-empty-table guard, with VCS revert as the
  recovery path (consistent with the repo's destructive-op safety convention).

### Manual Testing Required:

The plan's manual-verification items are all covered by automated tests
(custom-path typing, default-layout byte-equivalence, CWD≠root, constant
resolver-spawn count, fail-closed guard). No additional manual testing is
required. Optional spot-check if desired:

1. Config override behaviour:
  - [ ] In a scratch repo with `paths.work: custom/work`, confirm a work item
        under `custom/work/` is validated and one left at `meta/work/` is ignored.
2. Unknown subtree:
  - [ ] Add `meta/whatever/x.md` with junk frontmatter; confirm the validator
        exits 0 (silently skipped).

### Recommendations:

- None required before merge — `mise run test:integration:config`,
  `lint:scripts:check`, and all four targeted suites are green, and the real
  corpus validates clean.
- As a pre-push courtesy, run the full `mise run check` once to confirm the
  other three component toolchains are unaffected (this change is shell-only, so
  no impact is expected).
- The Performance Considerations note already flags the 13-fork startup cost and
  a future single-dump optimisation; no action needed now.
