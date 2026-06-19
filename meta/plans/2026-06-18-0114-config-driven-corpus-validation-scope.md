---
type: plan
id: "2026-06-18-0114-config-driven-corpus-validation-scope"
title: "Config-Driven Corpus Validation Scope Implementation Plan"
date: "2026-06-18T17:22:07+00:00"
author: "Toby Clemson"
producer: create-plan
status: ready
work_item_id: "work-item:0114"
parent: "work-item:0114"
relates_to: ["plan:2026-06-17-0114-fix-migration-0007-incomplete-mechanical-normalisation"]
tags: [validator, frontmatter, config, doc-type-inference, unified-schema, allowlist]
revision: "3a696f4cddb00fce6fbad76b41e32e1cf32e292b"
repository: "accelerator"
last_updated: "2026-06-19T00:26:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Config-Driven Corpus Validation Scope Implementation Plan

## Overview

Replace the corpus frontmatter validator's hardcoded `out_of_scope` **denylist**
with a configured-path **allowlist**: a `meta/` file is in scope for schema
validation if and only if it lives under a directory configured for a known
schema doc-type. Doc-type inference becomes config-aware (honouring
`.accelerator/config.md` path overrides) in the same move. This fixes the
class of bug that currently surfaces as the failing `meta/announcements/` case
and retires the band-aid skip-set entry added for it.

## Current State Analysis

The validator walks the entire `meta/` tree and validates *every* `*.md`
except a hardcoded skip set. This model **fails open**: any subtree a
downstream plugin consumer creates that the plugin authors did not anticipate
is treated as a malformed schema artifact and rejected.

**The validator** (`scripts/validate-corpus-frontmatter.sh`):
- Sources the shared classifier at `:33,35`
  (`DOC_TYPE_INFERENCE="${DOC_TYPE_INFERENCE:-$SCRIPT_DIR/doc-type-inference.sh}"`).
- `build_index` (`:195-213`) walks `find "$root" -name '*.md'`, calls
  `out_of_scope "$f" && continue` (`:198`) then `infer_type_from_path`
  (`:202`) to build the referential-integrity index.
- `main` (`:393-420`) walks the same tree, `out_of_scope "$f" && continue`
  (`:404`), then `validate_file "$f" yes`.

**The shared classifier** (`scripts/doc-type-inference.sh`):
- `infer_type_from_path` (`:17-34`) maps **hardcoded path suffixes**
  (`*/work/*` → `work-item`, `*/reviews/prs/*` → `pr-review`, etc.) to types.
  Because it matches literal default suffixes, it already **ignores configured
  path overrides** — a consumer who sets `paths.work: custom/work-items` gets
  no type inference. This is a second, latent bug.
- `out_of_scope` (`:41-46`) is the denylist:
  `*/specs/* | */talks/* | */global/* | */meta/docs/* | */meta/announcements/*`.
  The `*/meta/announcements/*` arm is the band-aid added in the current working
  copy to make `test:integration:config` pass.

**Path configuration machinery** (already exists, fully reusable):
- `scripts/config-defaults.sh:26-64` — `PATH_KEYS` (17) / `PATH_DEFAULTS`
  parallel arrays: the single registry of path keys and their defaults.
- `scripts/config-read-path.sh` / `config-read-value.sh` — resolve one key,
  honouring `.accelerator/config.md` then `config.local.md` (last-writer-wins),
  falling back to the registry default.
- `scripts/config-read-all-paths.sh` — already loops `PATH_KEYS`, resolves each,
  and emits a markdown legend; excludes `tmp`/`templates`/`integrations` via
  `EXCLUDED_KEYS` (`:14`) but **not** `global`.

**The schema type table** (`scripts/templates-schema.tsv`) defines exactly **13
doc-types**: `work-item, plan, plan-validation, pr-description, adr,
codebase-research, issue-research, design-inventory, design-gap, plan-review,
work-item-review, pr-review, note`.

**The other consumer** — the 0007 migration
(`skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`)
sources the *same* `doc-type-inference.sh` (`:15`) and calls `out_of_scope` /
`infer_type_from_path` in **ten** sites (`:259,262,319,395,404,522,534,551,
554,591`). A separate awk copy
(`skills/config/migrate/scripts/0007-frontmatter-rewrite.awk:78-98`,
`path_to_typed`) encodes the same directory→type fact for a *different* input
(a meta-path referenced *inside* a linkage value, not the current file). The
two are deliberately kept aligned, pinned by a test.

**Tests pinning current behaviour:**
- `scripts/test-validate-corpus-frontmatter.sh:302-313` — "migrated corpus
  validates clean" (the test currently failing without the band-aid).
- `skills/config/migrate/scripts/test-migrate-0007.sh:414-494` — `meta/docs/`
  byte-unchanged + whole-corpus-validates-clean (relies on denylist
  membership); `:502-510` — single-source guard (neither consumer defines the
  functions locally; both source the helper); `:512-530` — `path_to_typed`
  id-derivation alignment.

## Desired End State

A `meta/` file is validated **iff** it resolves to a known schema doc-type by
location, using the project's *configured* directories. Unknown subtrees
(`specs/`, `talks/`, `global/`, `docs/`, `announcements/`, and anything a
consumer invents) are silently skipped — no denylist. Doc-type inference
honours `.accelerator/config.md` path overrides. The `meta/announcements/`
band-aid is gone, and `test:integration:config` is green because the file is
out of scope structurally, not by name.

Verification: `mise run test:integration:config` passes; the validator run on a
corpus with a custom-configured doc-type dir validates files in that dir and
skips an arbitrary unknown dir; `mise run check` is green.

### Key Discoveries

- The type→path-key link is the **one mapping that exists nowhere
  canonically** today; it must be introduced (`config-defaults.sh` arrays).
- The allowlist is exactly `PATH_KEYS` minus `{templates, tmp, integrations,
  global}` — i.e. the 13 schema-typed dirs. `config-read-all-paths.sh` is *not*
  a drop-in source because it leaks `global`.
- `doc-type-inference.sh` is deliberately **pure** (no config reads, bash-3.2
  safe, sourced under `set -euo pipefail`). The refactor must keep it pure:
  consumers resolve config once and **inject** the resolved table; the helper
  only matches against injected arrays.
- Both in-tree consumers source the same helper, so a hard signature change
  must land in both simultaneously. A small backward-compatible fallback
  (hardcoded behaviour when no table is injected) decouples the consumer
  migrations into independently-mergeable phases.
- The awk `path_to_typed` classifies *historical pre-migration* linkage-value
  paths; under default config its alignment test stays green even if it is left
  default-pathed. Making it config-aware is a correctness nicety, not required
  to fix the bug — isolated to an optional final phase.

## What We're NOT Doing

- **Not** validating files that carry explicit `type:` frontmatter but live
  *outside* every configured doc-type dir. Scope is purely location-based: the
  validator polices configured corpus locations, not stray typed files
  elsewhere. (Decision — keeps the model simple and matches intent.)
- **Not** restricting the `find` walk to only configured dirs. We keep
  walk-all-then-filter for uniform handling of nested dirs; narrowing the walk
  is a possible future optimization, not this change.
- **Not** changing the schema TSV column structure. The type→path-key mapping
  lives in `config-defaults.sh`, leaving `fm_assert_schema_columns` and every
  positional `IFS=$'\t' read` untouched.
- **Not** supporting absolute paths in `paths.*` config values (project-relative
  only, as today).
- **Not** touching the stale `workspaces/*/` copies (jj workspace checkouts, not
  source).

## Implementation Approach

Introduce a canonical type→path-key registry and a resolver that emits the
configured doc-type directories. Refactor the shared classifier to match
against an *injected* resolved table (with a temporary fallback to today's
hardcoded behaviour so consumers can migrate one at a time). Wire the validator,
then the migration, to resolve-and-inject. Remove the fallback and the denylist
once both inject. Optionally make the awk surface config-aware last.

Each phase is independently mergeable: Phase 1 is purely additive; Phase 2
preserves existing behaviour via fallback; Phases 3–4 flip one consumer each;
Phase 5 is dead-code removal after both are flipped. **Phase 5 is mandatory and
must land in the same change-set series as Phases 2–4** — it retires the
two-headed classifier (the dual allowlist + dead denylist); leaving it undone
would strand a permanently confusing fallback. Only Phase 6 (config-aware awk)
is genuinely deferrable.

TDD throughout: tests for new/changed behaviour are written before the
implementation in each phase.

---

## Phase 1: Canonical type→path-key registry + resolver

### Overview

Add the single source of truth linking each schema doc-type to its path key,
and a script that resolves those keys into concrete directories honouring
config. Purely additive — no existing consumer changes.

### Changes Required

#### 1. Type→path-key registry

**File**: `scripts/config-defaults.sh`
**Changes**: Add two parallel arrays (bash 3.2 — no associative arrays),
covering all 13 schema doc-types, alongside the existing `PATH_KEYS`.

```bash
# Schema doc-type → path key (parallel arrays). The single canonical link
# between a templates-schema.tsv type and the config-defaults.sh path key
# whose directory holds artifacts of that type. Drives the corpus validator's
# allowlist (and, transitively, doc-type inference).
DOC_TYPE_NAMES=(
  "work-item" "plan" "plan-validation" "pr-description" "adr"
  "codebase-research" "issue-research" "design-inventory" "design-gap"
  "plan-review" "work-item-review" "pr-review" "note"
)
DOC_TYPE_PATH_KEYS=(
  "work" "plans" "validations" "prs" "decisions"
  "research_codebase" "research_issues" "research_design_inventories"
  "research_design_gaps" "review_plans" "review_work" "review_prs" "notes"
)
```

#### 2. Resolver script

**File**: `scripts/config-read-doc-type-paths.sh` (new)
**Changes**: Source `config-common.sh` (for the arrays + resolution), resolve
each `DOC_TYPE_PATH_KEYS` entry via `config-read-value.sh`, and emit one
`type<TAB>resolved-dir` line per doc-type with `printf '%s\t%s\n'` (portable tab;
consumers parse with `IFS=$'\t' read -r`). Machine-readable (no markdown legend),
so consumers populate parallel arrays directly.

**Resolution root (explicit mechanism).** `config-read-value.sh` resolves config
strictly from its CWD (`config_find_files` → `config_project_root` →
`find_repo_root`/`$PWD`) — it has **no** root parameter. So the resolver accepts
an optional project-root argument and, when given, runs its `config-read-value.sh`
reads inside a `( cd "$root" && … )` subshell — resolving against the corpus
root, not the caller's CWD, without touching the shared config reader. The
migration passes its canonicalised `PROJECT_ROOT` (Phase 4 §1); the validator
defaults to its own CWD.

**Value hardening** — the matcher consumes these dirs directly, and in the
migration they scope in-place mutation, so degenerate values must be neutralised
at the source:
- **Empty value**: `config-read-value.sh` returns `""` (not the default) for a
  key present but blank (`:126-130`). Treat an empty resolved value as "use the
  registry default" — so all 13 rows stay present and the matcher never sees an
  empty dir (which would degenerate `*/""/*` → `*//*`). Emit a stderr note when
  coercing a present-but-blank key, since blanking a path does **not** disable a
  doc-type (unsupported); document this in the header.
- **Path-safety**: reject a resolved value that fails an `assert_safe_relpath`-
  equivalent check — a `..` segment, a leading `/` (absolute), or `.`/empty —
  aborting non-zero with a diagnostic naming the key. (The migration already
  ships `assert_safe_relpath` at `0007-…sh:48-57` for exactly this; reuse its
  shape.) Stops a traversal/absolute override from widening the in-place
  mutation set.
- **Normalise** each surviving dir: strip a leading `./`, strip a trailing `/`,
  collapse repeated `/` — Phase 2's longest-match and segment anchoring assume
  clean dirs.
- **Reject** a value containing a tab or newline (it would corrupt the TSV
  line): abort non-zero naming the key.

**Shared populate helper.** Factor the read-loop that turns the resolver's TSV
into the injected arrays into one helper (e.g. `load_doc_type_table`, sourced
alongside the resolver) that both the validator and migration call — owning the
parse, the resolve-once invariant, and setting the `DOC_TYPE_TABLE_INJECTED`
sentinel in one place, so the two consumers cannot drift on TSV shape.

### Success Criteria

#### Automated Verification
- [x] New unit test `scripts/test-config-read-doc-type-paths.sh` passes (run:
      `bash scripts/test-config-read-doc-type-paths.sh`): emits 13 `type\tdir`
      lines for a default repo; reflects a `paths.work` override from a fixture
      `.accelerator/config.md`; an *empty* `paths.work:` value falls back to the
      registry default (no empty dir emitted); a trailing-slash override is
      normalised; and the output contains none of `meta/global`,
      `.accelerator/templates`, `.accelerator/tmp`,
      `.accelerator/state/integrations`.
- [x] Drift/coherence guard test asserts, in both directions, that
      `DOC_TYPE_NAMES` equals the `type` column of `templates-schema.tsv`; that
      `DOC_TYPE_NAMES` and `DOC_TYPE_PATH_KEYS` are equal length (index-coupled);
      and that every `DOC_TYPE_PATH_KEYS` entry is a member of `PATH_KEYS` (a
      typo'd key would otherwise resolve to an empty dir silently).
- [x] `mise run lint:scripts:check` passes (shfmt + ShellCheck + bashisms;
      bash 3.2 floor).

#### Manual Verification
- [x] `scripts/config-read-doc-type-paths.sh` run by hand in the repo lists the
      expected 13 default dirs and excludes `global`, `templates`, `tmp`,
      `integrations`.

---

## Phase 2: Config-injected allowlist in the shared classifier (backward-compatible)

### Overview

Refactor `doc-type-inference.sh` so `infer_type_from_path` and `out_of_scope`
match against an *injected* resolved table, while preserving today's hardcoded
behaviour when no table is injected. No consumer changes yet, so validator and
migration behaviour is unchanged after this phase.

### Changes Required

#### 1. Injected-table matching

**File**: `scripts/doc-type-inference.sh`
**Changes**: Consume two parallel arrays the caller may pre-populate —
`DOC_TYPE_NAMES[]` and `DOC_TYPE_DIRS[]` (resolved dirs, already normalised by
the resolver per Phase 1 §2: no leading `./`, no trailing `/`, no `//`).

**Path-form contract (explicit).** The injected dirs are project-relative (e.g.
`meta/work`), but consumers feed the classifier whatever `find "$root"` emits —
and in production `$root` is absolute (`validate-corpus-frontmatter.sh
"$(pwd)/meta"`; the migration walks `find "$META_ABS"`). Matching must therefore
be prefix-agnostic. A dir `D` matches path `$1` when:

```bash
case "$1" in
  */"$D"/* | "$D"/*) : ;;  # interior segment (absolute/nested root) OR root-relative start
esac
```

The trailing `/` in both arms enforces a segment boundary (so `meta/prs` never
matches `meta/prs-archive`), and `"$D"` is **quoted** so glob metacharacters in
a config value are matched literally, never re-globbed. Both arms are required:
`*/"$D"/*` covers absolute and nested roots; `"$D"/*` covers a path that begins
exactly with `D`.

**Most-specific (longest) match wins**, computed by plain integer length
comparison `${#D}` in a loop over the arrays — no `sort`, no `${var//}` slash
replacement (bash-3.2 / macOS hazards). This resolves `reviews/prs` vs `prs` and
`research/codebase` vs `research` without relying on case-arm order. On an
exact-length tie (two doc-types configured to the *same* dir), first-in-array
order wins deterministically — document this in the function header.

- `out_of_scope $1` becomes: if a table is injected, `return 0` iff
  `infer_type_from_path $1` is empty (allowlist); else the legacy denylist
  (transitional).

**Injection contract (explicit, not ambient).** The functions now read
caller-populated globals — which the helper header currently *forbids* ("pure
functions only — no top-level side effects"). Make the dependency explicit
rather than hidden: the caller populates `DOC_TYPE_NAMES[]`/`DOC_TYPE_DIRS[]`
exactly once, before first use, and the helper selects fallback-vs-allowlist via
a single sentinel (e.g. `DOC_TYPE_TABLE_INJECTED=1`) rather than inferring it
from array emptiness — so "no doc-types configured" (a real error, caught by the
Phase 4 non-empty guard) stays distinguishable from "table not injected"
(fallback). Name the injected arrays distinctly from the static
`config-defaults.sh` registry (e.g. `DOC_TYPE_INJECTED_NAMES[]` /
`DOC_TYPE_INJECTED_DIRS[]`) so the always-present static `DOC_TYPE_NAMES` cannot
be mistaken for the injected snapshot; the shared `load_doc_type_table` helper
(Phase 1 §2) populates them. Phase 5 rewrites the header to document this as a
required precondition (resolved-once, immutable for the run) and drops the stale
"pure function" language.

### Success Criteria

#### Automated Verification
- [x] New unit test `scripts/test-doc-type-inference.sh` passes (run:
      `bash scripts/test-doc-type-inference.sh`) covering: injected-table type
      match; **absolute-prefixed path** (`/abs/proj/meta/work/x.md` → `work-item`)
      as well as relative; most-specific-match (`…/meta/reviews/prs/x.md` →
      `pr-review`, not `pr-description`); segment-boundary safety
      (`…/meta/prs-archive/x.md` → no match); a configured dir containing a glob
      metacharacter matched **literally**; equal-length tie (two types → same
      dir) resolves deterministically by array order; allowlist skip (unknown
      dir → `out_of_scope` true); **fallback mode**.
- [x] The fallback-mode case is **golden-pinned**: a snapshot of the current
      helper's output (one representative path per arm, including the
      order-sensitive `reviews/prs`-vs-`prs` and `research/codebase`-vs-`research`
      pairs) is captured *before* the refactor and checked in as the expected
      fixture, so a transcription error cannot make implementation and test agree
      on a wrong answer.
- [x] `bash scripts/test-validate-corpus-frontmatter.sh` still passes
      (validator unchanged — still in fallback mode).
- [x] `bash skills/config/migrate/scripts/test-migrate-0007.sh` still passes.
- [x] `mise run lint:scripts:check` passes.

#### Manual Verification
- [x] Sourcing the helper without injecting a table reproduces the exact
      current classifications for the default tree.

---

## Phase 3: Wire the validator to the allowlist

### Overview

Resolve the doc-type table at validator startup, inject it into the classifier,
and flip to allowlist semantics. The `meta/announcements/` band-aid needs no
explicit removal here — it dies structurally the moment the validator stops
consulting the denylist; the shared fallback is left intact until Phase 5.

### Changes Required

#### 1. Resolve + inject at startup

**File**: `scripts/validate-corpus-frontmatter.sh`
**Changes**: At top-level script scope (before `build_index`), populate
`DOC_TYPE_NAMES[]` / `DOC_TYPE_DIRS[]` from a single
`config-read-doc-type-paths.sh` invocation — exactly once, immutable for the run,
so the index-build and validate passes observe identical scope. Keep the
`DOC_TYPE_INFERENCE` override seam. No change needed at the
`out_of_scope`/`infer_type_from_path` call sites (`:198,202,404`) — the functions
now consult the injected table.

#### 2. No fallback-denylist edit (band-aid retired structurally)

**File**: *(none in this phase)*
**Changes**: The validator stops consulting the legacy denylist the instant it
flips to the injected allowlist, so the `*/meta/announcements/*` band-aid is
already dead for the validator without touching `doc-type-inference.sh`. The arm
(and the rest of the denylist) is **left intact** here and removed only in
Phase 5: the 0007 migration is still in fallback mode until Phase 4, so mutating
the shared fallback now would change the still-fallback consumer's behaviour
mid-sequence, breaking per-phase independent mergeability.

#### 3. Tests

**File**: `scripts/test-validate-corpus-frontmatter.sh`
**Changes**: Add allowlist cases — (a) a file in an arbitrary unknown subtree
(e.g. `meta/announcements/x.md`, `meta/random/x.md`) is skipped (corpus
validates clean); (b) with a fixture `.accelerator/config.md` overriding
`paths.work`, assert **both halves** so the test cannot pass for the wrong
reason: a malformed file under the *configured custom* dir IS flagged, AND an
equivalently-malformed file left at the *default* `meta/work/` is skipped
(proving the override actually resolved rather than being silently ignored);
(c) the referential-integrity index includes files under configured dirs, and a
typed ref to a file placed in an out-of-scope subtree is now flagged
`DANGLING-REF` (confirming the index scoping took effect). The existing
"migrated corpus validates clean" assertion (`:302-313`) stays and now passes
without the band-aid.

### Success Criteria

#### Automated Verification
- [ ] `mise run test:integration:config` passes (the originally-failing task).
- [ ] `bash scripts/test-validate-corpus-frontmatter.sh` passes including the
      new allowlist cases.
- [ ] `scripts/validate-corpus-frontmatter.sh "$(pwd)/meta"` exits 0.
- [ ] The doc-type table is resolved once per run regardless of corpus size
      (a counter/trace check that `config-read-doc-type-paths.sh` is spawned a
      constant number of times — not per file or per walk pass).
- [ ] `bash skills/config/migrate/scripts/test-migrate-0007.sh` still passes
      (migration still in fallback mode).
- [ ] `mise run lint:scripts:check` passes.

#### Manual Verification
- [ ] In a scratch repo with `paths.work: custom/work` set, a work item placed
      under `custom/work/` is validated and one under `meta/work/` is ignored.

---

## Phase 4: Wire the 0007 migration to the allowlist

### Overview

Resolve and inject the same table in the migration so its ten file-level
`out_of_scope`/`infer_type_from_path` sites use config-driven classification,
consistent with the validator.

**Scope and type-derivation share one table.** In the migration
`infer_type_from_path` is not merely a scope gate — its return value is the
`type` written into regenerated frontmatter (`backfill_file:319`, the *sole*
type source for fence-less files; `rewrite_file:403-406` as the fallback when no
explicit `type:` is present). Making derivation config-aware is therefore
required, not optional: it is the only way an untyped artifact at a *configured
custom path* (e.g. `custom/work-items/0001.md` under
`paths.work: custom/work-items`) gets typed at all. A config-aware scope gate
paired with default-mapped derivation would be internally inconsistent — the
file passes `out_of_scope` (in scope per config) at `:395` yet derives an empty
type, so `backfill_file` writes `type:` empty (corrupt) or `rewrite_file`
silently `return 0`s an in-scope file (`:406`). Unifying scope and derivation on
the one injected table makes that case impossible by construction: a file is in
scope iff `infer_type_from_path` returns non-empty.

Reproducibility is preserved where it matters: 0007 is applied-once (gated by
`migrations-applied` state) and post-migration files carry an explicit `type:`,
so re-runs never re-derive from path (idempotency holds regardless). A
default/absent config resolves to exactly the default dirs, so default-layout
output is byte-identical to today. The only behaviour delta is for override
repos, where config-aware derivation is a *fix* (their custom-path files were
silently un-typeable under the hardcoded `*/work/*` map).

### Changes Required

#### 1. Resolve + inject in migration setup

**File**: `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`
**Changes**: After sourcing `doc-type-inference.sh` (`:15`), populate the
injected arrays once via the shared `load_doc_type_table` helper (Phase 1 §2)
from `config-read-doc-type-paths.sh`, **passing the migration's canonicalised
`PROJECT_ROOT`** so config resolves against the corpus root, not the migration's
CWD (which is not guaranteed to equal `PROJECT_ROOT`). The ten call sites are
unchanged; both the scope decision and the type derivation read the same
injected table.

**Consistent root for the spawned self-validator.** `self_validate_referential`
(`:742`) runs the validator as a *separate process* (`bash "$VALIDATOR"
"$META_ABS"`), which resolves its **own** table from *its* CWD (Phase 3 §1). To
keep the post-mutation integrity gate on the same scope that drove the mutation,
invoke it with CWD set to `PROJECT_ROOT` (e.g. `( cd "$PROJECT_ROOT" && bash
"$VALIDATOR" "$META_ABS" )`), so a CWD ≠ `PROJECT_ROOT` invocation cannot make
the self-check validate a different file set than was mutated.

#### 2. Pre-mutation guard (resolver-failure / fail-closed net)

**File**: `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`
**Changes**: After resolving the table and before the first mutating phase
(co-located with the existing `fm_assert_schema_columns` guard at `:720`, which
precedes `precondition_prepass` at `:721`), abort loudly with a non-zero exit if
the resolver failed (non-zero exit or zero rows). Because Phase 1 §2 coerces an
empty value to its default, a short/empty table only ever signals a *wholesale*
resolution failure — frame the guard as that fail-closed net, not a per-key
count check. Closes the silent-no-op hazard: a resolution failure would
otherwise classify the whole corpus out-of-scope and exit 0 having migrated
nothing, indistinguishable from a clean idempotent re-run (the clean-tree net
does not catch a no-op).

**Self-validation shares the injected scope (accepted, with an independent
backstop).** `self_validate_structural`/`build_corpus_index` use the same
injected table that scoped the mutation, so they cannot by themselves catch a
*wrong-but-non-empty* scope. The resolver's path-safety rejection (Phase 1 §2)
removes the traversal/absolute mis-scope class at the source; additionally
assert post-mutation that the typed-file count is non-zero for a non-empty
corpus. Document that same-scope self-validation is a deliberate, bounded
limitation, recovered (as for all 0007 faults) via VCS revert.

#### 3. Tests

**File**: `skills/config/migrate/scripts/test-migrate-0007.sh`
**Changes**:
- Update the `meta/docs/` skip assertions (`:414-494`) to assert the skip now
  arises from the allowlist (out-of-scope because `docs/` is not a configured
  doc-type dir).
- **Default-layout byte-equivalence** (the reproducibility guard): capture the
  byte-changed file set from the *current (pre-refactor)* migration over a
  fixture corpus and check it in as the expected golden (mirroring the Phase 2
  fallback golden), then assert the post-change migration reproduces it
  byte-for-byte — so the test cannot agree with a regressed implementation.
- **Prepass parity**: assert `precondition_prepass` produces an identical
  REFUSE/accept verdict on the default-layout fixture (its `out_of_scope`/
  `infer_type_from_path` use at `:259,262` becomes config-aware too, so the
  go/no-go decision — not just emitted bytes — must be unchanged on default
  layout).
- **Custom-path typing**: a fixture with `paths.work` overridden to a
  non-default dir containing an untyped (fence-less) file asserts the file is
  both in scope AND backfilled with the correct `type: work-item`.
- **Pre-mutation guard**: a fixture forcing resolver failure (and one with
  `paths.work: ..` / a leading-`/` value) asserts the migration aborts non-zero
  before mutating (zero files changed) — pins the fail-closed net and the
  path-safety rejection.
- **CWD ≠ PROJECT_ROOT**: run the migration from a CWD other than the corpus
  root (and a symlinked checkout) and assert the resolved scope/types AND the
  spawned self-validator observe the identical file set — pins the explicit-root
  seam end-to-end.
- **Constant resolver-spawn count**: assert `config-read-doc-type-paths.sh` is
  spawned a constant number of times across a migration run regardless of corpus
  size (mirrors the Phase 3 check; the migration is the larger, watchdog-bounded
  consumer).
- Keep the single-source guard (`:502-510`) — both consumers still source the
  helper and define nothing locally.

### Success Criteria

#### Automated Verification
- [ ] `bash skills/config/migrate/scripts/test-migrate-0007.sh` passes,
      including the default-layout byte-equivalence, custom-path typing,
      non-empty-guard, and CWD≠root cases.
- [ ] `bash scripts/test-validate-corpus-frontmatter.sh` still passes.
- [ ] Migration self-validation (`self_validate_structural`, `build_corpus_index`)
      behaves identically on a default-layout corpus.
- [ ] `mise run lint:scripts:check` passes.

#### Manual Verification
- [ ] Dry-run the migration on a copy of a real default-layout corpus: the set
      of files it processes vs skips, and the bytes written, are unchanged from
      before the change.
- [ ] Dry-run on a copy with `paths.work` overridden: an untyped file under the
      configured custom dir is typed `work-item`; an untyped file left at the
      old default `meta/work/` is out of scope and byte-unchanged.

---

## Phase 5: Remove the transitional fallback and dead denylist

### Overview

Both consumers now inject the table. Delete the backward-compatible fallback
branch, the legacy hardcoded suffix table, and the denylist from
`doc-type-inference.sh`, leaving only the injected-table path.

### Changes Required

#### 1. Drop the fallback + denylist

**File**: `scripts/doc-type-inference.sh`
**Changes**: Remove the legacy hardcoded `infer_type_from_path` suffix arms and
the entire `out_of_scope` denylist body (including the `*/meta/announcements/*`,
`*/specs/*`, `*/talks/*`, `*/global/*`, and `*/meta/docs/*` arms — the band-aid
is fully retired here). The functions now require an injected table. Rewrite the
header comment block (lines 1-14, 36-40): **replace the now-false "pure
functions only — no top-level side effects" framing** with the new contract —
the functions read the caller-populated `DOC_TYPE_NAMES[]`/`DOC_TYPE_DIRS[]`
arrays as a **required precondition**, populated exactly once before first use
and immutable for the run, with the `DOC_TYPE_TABLE_INJECTED` sentinel now always
set. Describe the allowlist model and keep the awk-alignment note.

#### 2. Guard tests

**File**: `scripts/test-doc-type-inference.sh`
**Changes**: Remove the fallback-mode cases; add an assertion that an
un-injected table yields "everything out of scope" (fail-closed), guarding
against accidental fail-open regressions.

### Success Criteria

#### Automated Verification
- [ ] `bash scripts/test-doc-type-inference.sh` passes.
- [ ] `bash scripts/test-validate-corpus-frontmatter.sh` passes.
- [ ] `bash skills/config/migrate/scripts/test-migrate-0007.sh` passes
      (single-source guard still green).
- [ ] `mise run test:integration:config` passes.
- [ ] `mise run lint:scripts:check` passes.

#### Manual Verification
- [ ] `grep -n 'specs\|talks\|announcements' scripts/doc-type-inference.sh`
      returns nothing — the denylist is fully gone.

---

## Phase 6 (optional, deferrable): Config-aware awk `path_to_typed`

### Overview

Make the migration's linkage-value classifier honour configured paths too, for
end-to-end config correctness. Independent of the bug fix; can ship later or be
dropped.

### Changes Required

#### 1. Inject the table into awk

**File**: `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk`
**Changes**: Replace the hardcoded `path_to_typed` (`:78-98`) directory tests
with a lookup over a table passed via `-v`, preserving the id-derivation halves
(work-item stem, ADR prefix). **Constrain the `BEGIN` parser to POSIX/BWK awk**
(macOS ships the one-true-awk, not gawk): no `gensub`/`length(array)`/`asort`;
use single-literal-character separators — emit the table as `type<TAB>dir`
records joined by newlines and iterate with `split(tbl, rows, "\n")` then
`split(rows[i], kv, "\t")` (avoid a `:` delimiter, which collides with the
`type:id` payload and would be treated as a regex by a multi-char `split`).

#### 2. Alignment test

**File**: `skills/config/migrate/scripts/test-migrate-0007.sh`
**Changes**: Extend the `path_to_typed` alignment test (`:512-530`) with a
non-default-path probe (a custom configured dir resolves to the right
`type:id`). **Keep the existing literal `type:id` expectations** as the source
of truth for the awk's output — do *not* replace them by driving both the awk
and its assertion from one shared table, or the test degrades into a
pass-through check that can no longer catch a regression in the awk lookup or
its id-derivation halves (work-item stem trimming, ADR prefix extraction).

### Success Criteria

#### Automated Verification
- [ ] `bash skills/config/migrate/scripts/test-migrate-0007.sh` passes,
      including the retained literal `type:id` expectations, the non-default-path
      probe, and the alignment test run under the pinned macOS BWK awk.
- [ ] `mise run lint:scripts:check` passes.

#### Manual Verification
- [ ] On a corpus that used a custom `paths.prs`, a body-section reference to a
      file under that custom dir is rewritten to the correct `pr-description:`
      typed ref.

---

## Testing Strategy

### Unit Tests
- `config-read-doc-type-paths.sh`: default resolution, config override, the
  13-row contract, exclusion of non-schema keys (Phase 1).
- `doc-type-inference.sh`: injected-table match, most-specific-match,
  segment-boundary safety, allowlist skip, fallback equivalence (Phase 2),
  fail-closed when un-injected (Phase 5).

### Integration Tests
- `test-validate-corpus-frontmatter.sh`: unknown subtree skipped; malformed file
  under a custom-configured dir flagged; referential index scoped to configured
  dirs; real migrated corpus validates clean without the band-aid (Phase 3).
- `test-migrate-0007.sh`: allowlist-driven `meta/docs/` skip; single-source
  guard; config-aware alignment (Phases 4, 6).
- `mise run test:integration:config` as the end-to-end gate.

### Manual Testing Steps
1. Scratch repo with `paths.work` overridden: confirm validation follows the
   configured dir, not `meta/work`.
2. Add an arbitrary `meta/whatever/` doc with junk frontmatter: confirm the
   validator ignores it (exit 0).
3. Dry-run 0007 on a default-layout corpus copy: confirm processed/skipped set
   is unchanged.

## Performance Considerations

Resolving the 13 path keys at startup costs 13 short-lived `config-read-value.sh`
forks — each re-sourcing the config library (`config-common.sh` →
`vcs-common.sh`/`config-defaults.sh`/`atomic-common.sh`) and re-parsing the
config file — **not** 13 "VCS detections" (`find_repo_root` is a cheap in-process
ancestor walk). The cost is one-time at startup: the table is resolved once into
the arrays (top-level scope, before `build_index` — see Phase 3 §1), never per
file or per walk pass. The per-file match is O(13) pure-bash `case`/`${#}` work
with no subprocess per array entry, run twice per file (build_index + main); at a
few-hundred-file corpus this is immaterial, well within the migration's post-DONE
watchdog budget. If it ever proves material, a follow-up can collapse the 13
forks into a single in-process config dump; not in scope here.

## Migration Notes

No data migration. The 0007 migration is idempotent and historical — already
migrated users see no diff, and idempotency is unaffected by config-aware
inference because post-migration files carry an explicit `type:` and re-runs
never re-derive from path. Behaviour on default-layout corpora is unchanged
throughout (a default/absent config resolves to exactly the default dirs); only
the *scoping mechanism* changes (denylist → allowlist) and config overrides
begin to be honoured. The one behaviour delta is for override repos, where the
migration now derives types config-aware — a fix that lets untyped artifacts at
configured custom paths be typed at all (the hardcoded `*/work/*` map silently
failed them), guarded by the Phase 4 non-empty-table check and the
default-layout byte-equivalence test. The current working-copy band-aid
(`*/meta/announcements/*` in `out_of_scope`) is retired in Phase 5: the
validator stops consulting it the moment it flips to the allowlist in Phase 3,
but the shared fallback denylist is left intact until both consumers inject, so
the still-fallback migration is unaffected mid-sequence.

## References

- Failing task: `mise run test:integration:config` →
  `scripts/test-validate-corpus-frontmatter.sh:302-313`
- Validator: `scripts/validate-corpus-frontmatter.sh`
- Shared classifier: `scripts/doc-type-inference.sh`
- Path registry: `scripts/config-defaults.sh:26-64`
- Path resolution: `scripts/config-read-path.sh`, `config-read-value.sh`,
  `config-read-all-paths.sh`
- Schema types: `scripts/templates-schema.tsv`
- Migration consumer: `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`
  (sites `:259,262,319,395,404,522,534,551,554,591`)
- Awk linkage classifier: `skills/config/migrate/scripts/0007-frontmatter-rewrite.awk:78-98`
- Migration tests: `skills/config/migrate/scripts/test-migrate-0007.sh:414-494,502-510,512-530`
