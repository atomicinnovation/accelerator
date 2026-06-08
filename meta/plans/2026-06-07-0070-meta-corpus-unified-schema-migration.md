---
type: plan
id: "2026-06-07-0070-meta-corpus-unified-schema-migration"
title: "Ship meta/ Corpus Unified-Schema Migration Implementation Plan"
date: "2026-06-07T09:05:13+00:00"
author: "Toby Clemson"
producer: create-plan
status: ready
work_item_id: "work-item:0070"
parent: "work-item:0070"
derived_from: ["codebase-research:2026-06-07-0070-meta-corpus-unified-schema-migration"]
tags: [migration, frontmatter, schema, interactive, visualiser, linkage, dogfood]
revision: "fd03c62147244115d0440905f91b5524f3ee5715"
repository: "ticket-management"
last_updated: "2026-06-08T09:33:11+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Ship `meta/` Corpus Unified-Schema Migration Implementation Plan

## Overview

Author and ship the numbered Accelerator migration (`0007`) that rewrites every
existing artifact under `meta/` to the unified frontmatter schema, populates
typed linkage frontmatter from the corpus's de-facto body sections (resolved
inferences mechanically, ambiguous ones through the 0069 interactive hook),
backfills baseline frontmatter onto the frontmatter-less files (13 fence-less
notes + 1 partial-fence note + 18 pre-convention plans), adds the unified `id:`
read path and deprecation warnings to the visualiser server's transitional
fallbacks (their **removal deferred to a follow-on release** per an
expand/migrate/contract split — see Phase 5), and dogfoods the whole thing
end-to-end against this repo's own corpus. This is the closing integration item
of epic 0057.

A new corpus-frontmatter validator is built as part of this work: AC-1 demands
that every migrated file "validates against the unified-schema validator", and
no such validator exists today (enforcement currently lives only at the
producer-surface contract-test level). The validator reuses
`scripts/templates-schema.tsv` as its per-type schema source.

## Current State Analysis

- **Migration framework is ready.** The runner
  (`skills/config/migrate/scripts/run-migrations.sh`) discovers
  `[0-9][0-9][0-9][0-9]-*.sh` by `sort -z`, records applied IDs in
  `.accelerator/state/migrations-applied`, and runs a clean-tree pre-flight.
  Head is **`0006`** (confirmed present on disk and as the last applied-ledger
  entry) → the migration to author is **`0007`**.
- **0005 (`type:`→`kind:`) and 0006 (`work_item_id`/`author` canonicalisation)
  are applied.** So work-items already carry `kind:`, plans already carry a
  quoted foreign `work_item_id:`, and research/RCA already carry `author:`. This
  migration must **not** duplicate those rewrites, and its awk transforms may
  assume `kind:` present and foreign `work_item_id:` already quoted.
- **The awk precedent is `0006-canonicalise-work-item-id-and-author.sh`**: an
  embedded awk state machine separating the frontmatter-fence region from the
  pre-first-`## H2` body region, writing via `atomic_write` only when `cmp -s`
  shows a change, with a `grep -qE` fast-skip early-return and
  `DIVERGE`/`REFUSE`/`MALFORMED` stderr diagnostics.
- **The interactive contract (0069) is ~588 lines of reusable harness.**
  `scripts/interactive-harness.sh` (plugin-root) + `interactive-protocol.sh`
  (plugin-root) + `skills/config/migrate/scripts/interactive-lib.sh`
  (runner-side). A migration opts in with `# INTERACTIVE: yes` in its first 5
  lines and implements `migration_emit_transformations` /
  `migration_apply_decision` (+ optional predicate/validate/verify hooks), then
  calls `harness_run`. Worked example:
  `…/test-fixtures/interactive/doc-example/migrations/0099-doc-example.sh`,
  driven by `scripts/test-migrate-interactive.sh`.
- **`scripts/templates-schema.tsv` is the de-facto per-type schema table** (**13
  data rows** — `work-item`, `plan`, `plan-validation`, `pr-description`, `adr`,
  `codebase-research`, `issue-research`, `design-inventory`, `design-gap`,
  `plan-review`, `work-item-review`, `pr-review`, `note`; `talk`/`spec` are not
  rows): per row it gives `type`, `code_state_anchored`, `extras`,
  `status_vocab`, `forbidden_own_id_key`, and `typed_linkage_keys`. The `note`
  row's status vocab is **`captured` only**, and `note` is
  `code_state_anchored=yes` (it carries `revision`/`repository`). **All consumers
  derive `code_state_anchored` from this column, never from a hard-coded type
  list.**
- **The corpus is 487 `.md` files** under `meta/`: work 100, plans 102,
  decisions 40, research 87 (across four subdirs — `codebase/`, `issues/`,
  `design-gaps/`, `design-inventories/`), reviews 136 (`plans/` 88, `work/` 48,
  `prs/` 0), validations 11, notes 14, specs 1, talks 1 (counts approximate;
  reconcile against the live corpus at implementation time). **31 files have no
  frontmatter fence at all** (13 notes + 18 pre-convention plans), plus **1
  note** (`2026-04-17-security-lens-owasp-ai-top-10.md`) carries a *partial*
  fence whose opening `---` line has **trailing whitespace** — a strict `/^---$/`
  detector treats it as fence-less, a loose `/^---[[:space:]]*$/` detector treats
  it as fenced. Notes are therefore routed to the note-backfill **by location**,
  never by absence-of-fence (see Phase 3 §3).
- **The visualiser-server transitional fallbacks** are at four sites:
  `frontmatter.rs:330-341` (the `work-item:` arm in `read_ref_keys`, pinned by a
  test at `:469-477`), `cluster_key.rs:119-131` (`parent_or_legacy_id`, sole call
  site `:75`), and `indexer.rs:~1233` (filename-id fallback). The
  `work-item-review` template carries a transitional `work_item_id:` alias at
  `templates/work-item-review.md:13`.
- **Crucially, `indexer.rs` work-item identity reads only `work_item_id:`**
  (`:1216-1233`), not the unified `id:`. After migration a work-item carries
  `id:` and no `work_item_id:`, so `fm_id` resolves to `None` and identity falls
  through to filename extraction. For conventionally-named files (`0070-foo.md`)
  the filename fallback still resolves; but a **synced work-item whose filename
  does not encode its id** (the very case the existing doc-comment cites) would
  lose its identity post-migration. The reader must therefore **gain the `id:`
  key this release** (additively, alongside the retained legacy fallbacks) so
  migrated corpora resolve while the cross-repo rollout is mid-flight.

### Key Discoveries:

- **Pre-existing frontmatter linkage is in legacy shapes (net scope).** Producer
  updates 0065/0066 already populated linkage frontmatter, but not in the typed
  form ADR-0034 / the validator require: **139 `target:` values are path-shape**
  (`"meta/work/0030-foo.md"`), **21** linkage values are bare-number (`"0030"`),
  and only **48** are already typed (`"work-item:0030"`). The ~49 `reviews/work/`
  artifacts carry both a path-shape `target:` *and* the redundant `work_item_id:`
  alias. Normalising these (path→typed, deterministic; strip the review alias;
  derive `parent:` from foreign `work_item_id:`; bare-number → hook) is a
  first-class mechanical concern of the awk rewrite (Phase 3 §2) — **not** just
  the body-section parser, which only sees `## …` sections.
- **No corpus validator exists.** `scripts/test-template-frontmatter.sh`
  validates `templates/*.md` against `templates-schema.tsv`;
  `scripts/test-skill-frontmatter-population.sh` validates SKILL.md guidance.
  ADR-0040 explicitly records "No test inspects a generated artifact." → **build
  one** (Phase 1), reusing `templates-schema.tsv`.
- **The fence-less files split into notes and 18 pre-convention plans**, not
  "design-inventory captures" as the research first framed it. **Each of the 18
  must be inspected and confirmed to be a genuine plan before assigning
  `type: plan`** — any that are prose-only design notes get a different type or
  are excluded, not silently mis-typed. The confirmed plans get **full plan
  frontmatter backfilled**, reusing the notes-backfill identity/VCS machinery
  extended for the plan type's code-state-anchored provenance bundle.
- **`type:` (ADR-0033 artifact discriminator) is inferred from corpus
  location** *only where absent* — files that already carry an explicit `type:`
  (e.g. `design-gap`/`design-inventory` artifacts, which also carry
  `git_commit`/`branch` to drop) keep their declared type; location inference
  never overrides a present `type:`. The location map, **enumerated exhaustively
  (no "research subdirs" shorthand)**:
  `meta/work/`→`work-item`, `meta/plans/`→`plan`, `meta/decisions/`→`adr`,
  `meta/research/codebase/`→`codebase-research`,
  `meta/research/issues/`→`issue-research`,
  `meta/research/design-gaps/`→`design-gap`,
  `meta/research/design-inventories/`→`design-inventory`,
  `meta/reviews/plans/`→`plan-review`, `meta/reviews/work/`→`work-item-review`,
  `meta/reviews/prs/`→`pr-review`, `meta/validations/`→`plan-validation`,
  `meta/notes/`→`note`. **Reviews are discriminated by subdirectory, not by
  filename** (filenames carry only a `-review-N` suffix that cannot separate
  plan-review from work-item-review).
- **Idempotency is layered**: applied-ledger (won't re-run an applied ID) +
  per-file `cmp -s` gate (won't rewrite unchanged bytes) + interactive resume
  (won't re-prompt recorded `(artifact_path, source_anchor)` keys). A re-run is
  a no-op on all three planes.
- **Skip is a legitimate interactive terminal** (`APPLIED_CONFIRM` with no
  mutation). AC-7's terminal-state check accepts skips; the fixture set proves
  the *apply* path via scripted accept/edit decisions.
- **`cluster_key.rs:parent_or_legacy_id`** must keep `id_from_value` + the
  `parent` branch and drop only the `work_item_id` legacy branch (`:125-129`).

## Desired End State

Running `/accelerator:migrate` against this repo's `meta/` corpus applies `0007`
to completion: every artifact (except deliberately out-of-scope `specs/` and
`talks/`) carries unified-schema frontmatter that passes the new corpus
validator; typed linkage frontmatter is populated and **every emitted
`"doc-type:id"` reference resolves to a file that exists** (tolerating only the
`pr:` external prefix); the fence-less files carry baseline frontmatter; the
migration **exits non-zero if any `REFUSE`/`MALFORMED` is emitted** (a refused
file is never silently shipped half-migrated); an immediate re-run reports no
changes; the visualiser server gains the unified `id:`/`target:` read path,
emits a deprecation warning whenever a legacy fallback arm fires, and the
`work-item-review` template alias is removed (**the Rust fallback arms
themselves are retained this release and removed in a follow-on** — see Phase 5);
and a dogfood report records the resolved-band wrong-rate (≤5%) and any annotated
`DIVERGE` lines.

**Verification**: `mise run test:unit:visualiser` green (reader gains `id:`,
fallbacks retained, deprecation-warning paths covered);
`scripts/test-validate-corpus-frontmatter.sh` green (including the
referential-integrity check); the migration's own fixture tests green;
`scripts/test-migrate-interactive.sh` green for the scripted-decision fixtures; a
clean dogfood run on this repo's corpus, followed by a **direct script re-invocation
(ledger bypassed) that produces an empty VCS diff** to prove the `cmp -s`/resume
no-op. (The `grep -r parent_or_legacy_id` returns-nothing check moves to the
follow-on removal story.)

## What We're NOT Doing

- **Not** rewriting `meta/specs/` (deferred per 0057) or `meta/talks/`
  (`type: talk` is a non-producer type with no schema row) — both excluded from
  the migration and validator, annotated in the dogfood gap-fix log as
  deliberately out of scope. The `talk` type is **not** added to the schema.
- **Not** re-doing migration 0005's `type:`→`kind:` rewrite or 0006's
  `work-item:`→`work_item_id:` / `researcher:`→`author:` canonicalisation (those
  ran already; this migration depends on them).
- **Not** building an inverse migration — VCS revert is the safety net
  (ADR-0023).
- **Not** validating `pr:` references — they are tolerated (a supplementary ADR
  to add the `pr:` external-entity prefix to ADR-0034 is pending).
- **Not** touching `specs/` / `global/` (out of scope per 0057).
- **Not** adding dry-run/preview/confirm UX — clean-tree pre-flight + VCS revert
  is the recovery path.
- **Not** removing the visualiser-server fallback *arms* in this release
  (`read_ref_keys` `work-item:` arm, `parent_or_legacy_id`, the `indexer.rs`
  filename fallback). Their removal — and the `grep -r parent_or_legacy_id`
  returns-nothing gate (AC-12/AC-13) — is **deferred to a follow-on contract
  story**, raised as part of this work. This release only *expands* the reader
  and *deprecates* the arms. (Note: the work item's AC-12/AC-13 as written assume
  same-release removal; flag to the work-item owner that those ACs now split
  across two releases.)

## Implementation Approach

Five phases, each independently mergeable and leaving `main` green. The migration
script `0007` is **authored as one complete unit** (Phase 3) — the story argues
its indivisibility (single dogfood gate; the linkage parser, the awk base-field
rewrite, and the backfill all write the *same* files) and, crucially, a
partially-built migration recorded as applied in the ledger would prevent later
additions from re-running. The heavy, separately-shippable *components* it
consumes — the corpus validator (Phase 1) and the body-section linkage parser +
fixtures (Phase 2) — land first as standalone, tested artifacts. The dogfood
(Phase 4) follows.

**Visualiser sequencing — expand/migrate/contract (revised):** the original
plan removed the Rust read-path fallbacks in this same release on the strength
of a migrate-on-use guarantee. That guarantee is *advisory*, not enforced
(`migrate/SKILL.md`: skills do not gate on pending migrations), so a dormant
userspace repo could upgrade the visualiser before applying `0007` and silently
lose cross-references. We therefore split the change: **this release (Phase 5)
expands the reader** (adds the unified `id:`/`target:` path, keeps every legacy
fallback, emits a deprecation warning when one fires) and removes the
`work-item-review` template alias; **a follow-on story contracts** (removes the
fallback arms and their pinning tests, once every consuming repo has migrated).
The `0007` migration and the reader-expand can ship together safely because the
expanded reader accepts both legacy and unified shapes.

Test-driven throughout: each phase writes fixtures/tests before (or alongside)
the implementation, and the new test scripts wire into the `mise` test tasks
**via the integration-test subtree discovery** (`run_shell_suites`), not the
hand-listed `tasks/test/unit.py` — see Phase 1 §3.

---

## Phase 1: Corpus Frontmatter Validator

### Overview

Build a standalone validator that inspects a *generated* `meta/` artifact against
the unified schema, reusing `scripts/templates-schema.tsv` as the per-type schema
source. This is the AC-1 gate consumed by Phase 4 and is independently useful.

### Changes Required:

#### 1. Validator script

**File**: `scripts/validate-corpus-frontmatter.sh` (new)
**Changes**: For one or more `.md` paths, parse frontmatter and validate against
the schema row matched by `type:`:

- `type:` present and a literal in the schema table (else `INVALID-TYPE`); files
  with no fence are `NO-FENCE` (caller decides whether that is in scope).
- Required base fields present: `type`, `id`, `title`, `date`, `author`,
  `tags`, `last_updated`, `last_updated_by`, `schema_version`. `status` required
  only where the type's `status_vocab` is non-empty and the artifact carries one
  (absent `status` is permitted per the defaults rule).
- `id:` is a **quoted** YAML string. **Quoting is enforced on `id:` only** — other
  string base fields (`title`, `author`, `last_updated_by`, `repository`) are
  presence-checked, not quote-checked, so the corpus's existing unquoted-but-present
  `author: Toby Clemson` lines stay conforming and the awk need not re-quote them.
  (A fixture asserts an unquoted-but-present `author:` is accepted, so a future
  tightening of the shared emission-rules helper can't silently open a corpus-wide
  failure.)
- `schema_version:` is the bare integer `1`.
- `status:` (when present) is in the row's `status_vocab`.
- Provenance bundle (`revision`, `repository`) present iff
  `code_state_anchored=yes`; `git_commit`/`branch` **absent**.
- The row's `forbidden_own_id_key` is **absent** (e.g. no `work_item_id` on
  work-items, no `adr_id` on ADRs).
- Per-type `extras` present.
- `tags:` always present (may be `[]`).
- **Omit-when-empty (ADR-0040)**: no empty-placeholder keys — flag any
  `parent: ""`, `blocks: []`, `relates_to: []`, foreign-ref, or optional-extra
  key emitted empty. `tags: []` and always-valued extras are exempt.
- Typed-linkage values (where present) use the `"doc-type:id"` form — never bare
  `"NNNN"` **and never path-shape** (`"meta/work/0030-foo.md"`). Both legacy
  shapes are violations (the migration is responsible for normalising the 139
  path-shape + 21 bare-number values it inherits — see Phase 3 §2).
- **Referential integrity**: every typed-linkage value resolves to an artifact
  that exists in the corpus index (`"doc-type:id"` → a real file). The index
  resolves each artifact's identity by the **same rule the migration uses**
  (`id:` → legacy own-id key → filename), so a mid-flight or legacy-keyed target
  still resolves. Unresolved targets are a violation. The **only** tolerated
  unresolved prefix is `pr:` (the external-entity prefix pending an ADR-0034
  supplement); the validator treats `pr:<n>` as a known-good literal, not a
  resolvable corpus ref. This is the foreign-key constraint that stops the
  migration from silently writing dangling references into the graph the
  visualiser epic consumes. **Referential integrity is a whole-corpus property** —
  it runs over the full corpus index (the migration's in-run *structural*
  self-check may run on a touched-file subset, but referential integrity is
  evaluated only against the complete index; out-of-scope `specs/`/`talks/`
  targets are treated consistently by both the in-run and Phase 4 passes).

**Schema sourcing — be precise about what is single-sourced.** The TSV is the
sole source of the *per-type tabular facts*: the type set, `extras`,
`status_vocab`, `code_state_anchored`, `forbidden_own_id_key`,
`typed_linkage_keys`. The *cross-cutting emission rules* — the required base-field
set, the quoted-`id:` rule, `schema_version: 1` as a bare integer, the
`git_commit`/`branch`-absent rule, omit-when-empty, the `"doc-type:id"` value
shape — have no TSV column and are encoded **once** in a shared sourceable helper
consumed by *both* this validator and the existing
`scripts/test-template-frontmatter.sh`, so the two surfaces cannot drift. (The
prior single-source claim was imprecise; this split is the accurate contract.)

**In scope: extract the shared helper out of `test-template-frontmatter.sh`.**
That script currently encodes the cross-cutting rules inline (base-field set,
provenance/forbidden-provenance sets, source-type vocab, quoted-id/bare-integer
regexes, closed-set check). This Phase **refactors those constructs into the new
shared helper and re-points `test-template-frontmatter.sh` at it**, confirming its
existing self-test still passes — otherwise the validator builds its own copy and
the drift the contract forbids reappears.

**Invocation contract (depended on by Phase 3 self-check + Phase 4):** the
validator accepts either a directory root (walks it) or an explicit file list;
referential integrity is evaluated only in whole-corpus (directory) mode. Exit
non-zero with one diagnostic line per violation; exit 0 when clean. Phase 3's
in-run structural self-check calls it in file-list mode; Phase 3's post-`harness_run`
and Phase 4's gate call it in whole-corpus mode.

#### 2. Validator test

**File**: `scripts/test-validate-corpus-frontmatter.sh` (new) + fixtures under
`scripts/test-fixtures/corpus-validator/`
**Changes**: Fixture files — one valid example per artifact type, plus one
fixture per failure mode (unquoted id, missing base field, `git_commit` present,
forbidden own-id key, empty-placeholder key, bare-number linkage, **path-shape
linkage**, dangling typed reference, bad status, `schema_version` non-integer,
**malformed `date:`/`last_updated:`** paired with a valid ISO-timestamp form).
Assert each is accepted/rejected as expected. Plus a **single-source guard**:
flip one cross-cutting rule in the shared emission-rules helper and assert *both*
this validator and `scripts/test-template-frontmatter.sh` change behaviour
(proves they consult the one helper, not divergent copies). The extraction must
**preserve `test-template-frontmatter.sh`'s gated self-test counts** (`-eq 6` /
`-eq 9`) — confirm they still pass post-refactor, and have the guard flip a rule
that demonstrably feeds *both* the gated self-test path and the validator.

#### 3. Wire into mise

**File**: the relevant **integration**-test task module (not `tasks/test/unit.py`)
**Changes**: Shell `test-*.sh` suites are auto-discovered by `run_shell_suites(subtree)`
in the integration tasks, **not** hand-listed in `unit.py`. A suite placed under
`scripts/` is picked up by `test:integration:config`; one under
`skills/config/migrate/` by `test:integration:migrate`. So:
`scripts/test-validate-corpus-frontmatter.sh` and (Phase 2)
`scripts/test-linkage-parser.sh` land under `scripts/` (run by
`test:integration:config`); ensure each is **executable** (the discovery filter
requires the exec bit). The Phase 3 migration fixture suite under
`skills/config/migrate/` must additionally **bump `_EXPECTED_MIGRATE_SUITES` in
`tasks/test/integration.py`** (currently `3` → `4`) — a new migrate suite without
that bump fails the build. **The `config` task has no analogous suite-count guard
today**: add a `_EXPECTED_CONFIG_SUITES` assertion (mirroring the migrate
pattern) so the two new exec-bit-discovered config suites can't silently vanish
from CI if an exec bit is dropped — these are the AC-1/AC-6/AC-8 gates and must
not fail open. Set its baseline to the **reconciled live count** (the current
discoverable `scripts/test-*.sh` count + 2); since the guard is an at-least
comparison, also add a small test that drops a fixture's exec bit and asserts the
guard fires, so the baseline can't silently drift.

### Success Criteria:

#### Automated Verification:
- [x] Validator test passes via `test:integration:config` (new suite discovered)
- [x] Validator accepts a known-good fixture of every schema-defined type
      (**13 rows in `templates-schema.tsv`**; assert coverage against the TSV row
      count, not a hard-coded number)
- [x] Validator rejects each failure-mode fixture with the expected diagnostic
      (unquoted id, missing base field, `git_commit` present, forbidden own-id
      key, empty-placeholder key, bare-number linkage, bad status,
      `schema_version` non-integer, **dangling typed-linkage reference**)
- [x] Referential-integrity check resolves valid `"doc-type:id"` refs and flags a
      dangling one; `pr:<n>` is accepted as a tolerated literal
- [x] Running the validator over the *current* (pre-migration) corpus reports
      the expected legacy violations (sanity check that it inspects real files)

#### Manual Verification:
- [x] Diagnostics are specific enough to locate the offending file + key
- [x] Per-type tabular facts come only from `templates-schema.tsv`; cross-cutting
      emission rules live in the one shared helper (no duplication with
      `test-template-frontmatter.sh`)

---

## Phase 2: Body-Section Linkage Parser + Band Classifier + Fixtures

### Overview

Build the net-new body-section linkage parser (the 0068 prototype was throwaway)
as a standalone, unit-tested component the migration sources in Phase 3. Encode
the three spike-mandated fixes **before** band classification, and author the
two fixture sets AC-6/AC-8/AC-11 require.

### Changes Required:

#### 1. Parser library

**File**: `scripts/linkage-parser.sh` (new; sourceable + CLI for tests)
**Changes**: Parse the five de-facto linkage-bearing body sections —
`## References`, `## Dependencies`, `## Historical Context`,
`## Related Research`, `## Source References` (confirmed authoritative set;
`## Related Documents` does not appear). For each candidate reference emit a
`(source-type, key, target-type, target-id, anchor, band)` record.

**Three spike fixes, applied before band classification:**

```
# 1. template-path blocklist — literal placeholders produce no linkage
#    e.g. ADR-NNNN.md, YYYY-MM-DD-topic.md, {number}-description.md
# 2. tightened "blocks" match — "code-block" prose produces no `blocks` linkage
#    (hyphen is treated as a boundary)
# 3. "sibling" -> relates_to — instead of falling through to the
#    (plan, codebase-research) -> derived_from default
```

**Portability — do NOT use `\b`.** These three fixes are the spike-mandated
behaviour that must classify identically on every machine, but `\b` word
boundaries are **not portable**: BSD `awk` and BSD `grep -E` (the macOS defaults)
do not honour `\b` — only PCRE does, which stock macOS `grep` lacks. A `\b`
pattern that passes on a Linux/gawk CI runner would silently mis-match on a macOS
contributor and change the resolved-vs-ambiguous band. Emulate boundaries with
explicit POSIX character classes using the **underscore-inclusive** negated class
**verbatim from `scripts/lint-bashisms.sh`**: ERE
`(^|[^[:alnum:]_])blocks?([^[:alnum:]_]|$)` and `(^|[^[:alnum:]_])sibling([^[:alnum:]_]|$)`
(the `_` matters — `code_block`/`sub_blocks` should be treated as boundaried too).
A fixture must prove the hyphen-boundary case (`code-block`) classifies identically
under the macOS toolchain, not only on CI.

**Keep the ADR-0034 type-pair table as explicit data**, not inline regex —
encode the `(source-type, key, target-type)` tuples as a checked-in table the
matcher consults, and name each spike-fix rule as its own documented function, so
the band logic is legible and the eventual `pr:` ADR is a one-row change.

**Band rule (ADR-0038):** map each reference onto ADR-0034's published
`(source-type, key, target-type)` table.
- `resolved` = exactly one tuple matches, no competing candidate → apply
  mechanically.
- `ambiguous` = zero or >1 tuples match (e.g. a bare number resolvable to either
  a work-item or an ADR) → route to the interactive hook.
- Deterministic field renames / shape normalisation are **not** inferences and
  always apply mechanically.

**Prose disambiguation onto ADR-0034's table**: a plan's `"Source:"` line →
`parent` for a work-item target, `derived_from` for a research target; non-meta
targets → `source`. Emit the typed `"doc-type:id"` form (never bare `"NNNN"`),
write only the canonical bidirectional side (`blocks`, `supersedes`). Tolerate
`pr:` references (not yet in ADR-0034's vocabulary).

#### 2. Parser tests + the two fixture sets

**File**: `scripts/test-linkage-parser.sh` (new) + fixtures under
`scripts/test-fixtures/linkage-parser/`
**Changes**:
- **Spike-fix fixtures (AC-11)**: a `ADR-NNNN.md` literal → no linkage; prose
  containing "code-block" → no `blocks` linkage; a "sibling" reference →
  `relates_to`. Include the hyphen-boundary case proven under the macOS toolchain.
- **Band-classification fixture set (AC-6/AC-8)**: a labelled set of references
  with their expected band (resolved vs ambiguous) across all five header types.
- **Resolved-band golden-target set (AC-8 correctness, net-new)**: for resolved
  references, assert the **emitted target value**, not merely the band — drawn
  from real corpus phrasings with hand-verified expected `"doc-type:id"` output.
  This is the automated correctness gate behind the mechanical path that the
  dogfood's manual ≤5% sample alone does not provide.
- **Prose-disambiguation fixtures**: a plan's `"Source:"` line resolves to
  `parent` (work-item target), `derived_from` (research target), and `source`
  (non-meta target) respectively.
- **`pr:` tolerance fixture**: a `pr:<n>` reference is emitted/tolerated and
  never flagged.
- **Known-ambiguous fixture set (AC-9)**: references such as a bare number
  resolvable to either a work-item or an ADR, with the expected linkage after a
  scripted hook decision.

### Success Criteria:

#### Automated Verification:
- [x] Parser tests pass via `test:integration:config`
- [x] Each of the three spike fixes is proven by a dedicated fixture, including
      the hyphen-boundary case under the macOS toolchain (no `\b`)
- [x] Every band-classification fixture classifies to its labelled band
- [x] Every resolved-band golden-target fixture emits its hand-verified
      `"doc-type:id"` value (correctness, not just band)
- [x] `"Source:"` line resolves to `parent` / `derived_from` / `source` for
      work-item / research / non-meta targets
- [x] A `pr:<n>` reference is tolerated, never flagged
- [x] Each known-ambiguous fixture classifies `ambiguous`

#### Manual Verification:
- [x] The five header parsers handle real corpus phrasings (spot-check against
      `meta/research/`, `meta/work/`, `meta/decisions/` examples)

> Implementation note: the plan's prescribed boundary regex
> `(^|[^[:alnum:]_])blocks?(…)` would *match* `code-block` (a hyphen satisfies
> the negated class), contradicting AC-11's "code-block produces no `blocks`
> linkage". The spike's own fix text ("scope it to recognised list-lead
> positions") and the acceptance criterion are unambiguous, so the keyword
> detectors use a whitespace/label boundary that correctly excludes both
> `code-block` and `code_block` (still no `\b`; proven under `/bin/bash` 3.2).

---

## Phase 3: Migration `0007` — Mechanical Rewrite, Backfill, and Interactive Linkage

### Overview

Author `0007` complete, consuming Phase 1's validator (for self-check) and
Phase 2's parser. Three concerns in one indivisible migration: (a) the
deterministic base-field/identity/provenance awk rewrite over the whole corpus,
(b) baseline-frontmatter backfill for the fence-less + partial-fence files
(notes routed by location), (c) interactive linkage population. Modelled
structurally on `0006`.

### Changes Required:

#### 1. Migration skeleton + corpus walker

**File**: `skills/config/migrate/migrations/0007-unify-meta-corpus-frontmatter.sh`
(new)
**Changes**: `# INTERACTIVE: yes` in the first 5 lines. Source
`config-common.sh`, `atomic-common.sh`, `log-common.sh`, and
`$CLAUDE_PLUGIN_ROOT/scripts/interactive-harness.sh`. Reuse 0006's path-safety
(`assert_safe_relpath`, `resolve_corpus_path`), bash-3.2 parallel-array dedup,
and walker. **Export `LC_ALL=C`/`LANG=C`** for the whole text-processing pipeline
(awk/grep/sort/cmp), matching the launcher's locale-safety precedent — byte-class
interpretation must be identical across environments or the `cmp -s`
idempotency gate and `[[:alnum:]]` boundaries flap. Extend the walk across the
**whole** corpus with **`type` inferred from location only where absent**, using
the exhaustive map in Current State Analysis (all four research subdirs; reviews
by subdirectory; an existing `type:` is never overridden); `specs`/`talks`
excluded.

**Per-file dispatch is explicit and mutually exclusive** (resolves the
MALFORMED-vs-backfill hazard): a file with **no leading fence** (using the
strict `/^---$/` test) goes to the backfill pass **only** and is excluded from
the deterministic awk legacy-key rewrite, so a fence-less file containing a
matched token can never trip `0007-MALFORMED`. **Notes are dispatched to the
note-backfill by location (`meta/notes/`), not by absence-of-fence** — this is
how the one partial-fence note (trailing whitespace on its `---`) is caught.

**Pass ordering and protocol hygiene.** After the step-0 precondition pre-pass
(below), the migration body runs, in order: (1) backfill fence-less files to
baseline frontmatter, (2) the deterministic awk normalise/extras rewrite over the
now-fenced corpus, then (3) `harness_run` to drive linkage. Steps 1–2 are
mechanical and unrecorded. **All progress output
from steps 1–2 must go to stderr, never stdout** — for an interactive migration
the runner wires stdout to the `m2r` FIFO and parses every line as a TSV protocol
frame; the first legitimate frame is `READY` inside `harness_run`. 0006's reused
walker echoes a "rewrote N files" line to stdout — redirect that (and all
diagnostics) to fd 2, or the bytes are mis-parsed as frames and corrupt the
handshake. A fixture must assert **no non-frame bytes precede `READY`**.

**Defensive preconditions run as a read-only PRE-PASS, before any write.**
`sort -z` only orders *pending* migrations; it does not guarantee 0005/0006 were
ever applied to this corpus (a hand-edited ledger, a restored snapshot). A fatal
precondition discovered *mid-rewrite* would leave a partially-mutated tree (step-1
backfill already committed). So the precondition scan is **step 0**: walk the whole
corpus read-only and REFUSE+exit-non-zero (zero mutations) if any work-item is
missing `kind:` or any foreign `work_item_id:` is unquoted, surfacing an
out-of-order/skipped predecessor before a single file is touched. The pre-pass
**also guards primary-key integrity**: REFUSE if any work-item's own
`work_item_id:` ≠ its filename-derived id, or if the **post-rewrite `id:` set
would contain a duplicate**. That set is the **union of the awk-derived ids
(fenced files) *and* the backfill-derived full-stem ids of all 32 backfilled
files** — driven off the same by-location backfill dispatch list §3 uses (the 31
strictly fence-less files **plus the partial-fence note**), predicted read-only
during this pass — so a backfilled note/plan whose stem would collide with an
existing or another backfilled id is caught before any write, not mid-rewrite. **Known live instance**: `meta/work/0032-per-test-server-configuration-for-e2e-tests.md`
carries `work_item_id: "0031"` (its H1 also wrongly reads `# 0031:`), colliding
with `0031-…`; the dogfood must correct this stray value before `0007` runs — the
pre-pass turns this latent corpus bug into a loud REFUSE rather than a silent
duplicate `id:`. The full ordering is therefore: **(0) precondition pre-pass →
(1) backfill → (2) awk rewrite → (3) `harness_run`**.

#### 2. Deterministic frontmatter awk rewrite

**File**: same migration
**Changes**: An awk state machine (0006-style: fence-region vs pre-first-`## H2`
body region; non-terminating detector rules over terminating transform rules;
catch-all `{ print }`; `cmp -s` + `atomic_write` gate; `grep -qE` fast-skip):

- Add unified base fields with defaults where absent: `tags: []`,
  `schema_version: 1`, `last_updated`/`last_updated_by` **seeded from the
  artifact's `date`/`author`**, `status` left **unset** when absent. Infer
  `type:` from location (only where absent). The seed must be a **pure function
  of on-disk fields — never wall-clock** — so a re-run is byte-stable and the
  `cmp -s` gate skips it. **Normalise before seeding**: when `date:` is itself
  date-only, the value seeded into an absent `last_updated:` must be the
  *normalised* full-ISO form (apply the date-only→ISO rule below first), so a
  seeded `last_updated:` can never inherit a non-conforming date-only shape.
- **Normalise existing date-only `date:`/`last_updated:` to full ISO** (not just
  seed-when-absent): **71 fenced files** already carry a present date-only
  `last_updated: "2026-02-22"` (and some a date-only `date:`) that the awk would
  otherwise leave untouched — and the validator requires an ISO timestamp, so all
  71 would fail the AC-1 gate. Where a value is bare `YYYY-MM-DD`, string-concat
  the midnight-UTC suffix (`T00:00:00+00:00`, same technique as the §3 backfill);
  where it is already a full ISO timestamp (`Z` or `±HH:MM`), leave it untouched.
  This is a deterministic, byte-stable transform (a fixture pins both the
  date-only→ISO rewrite and the already-ISO no-op).
- **General principle — no present nonconforming base-field value is left for the
  validator.** "Seed/infer only when absent" is *not* sufficient: the corpus has
  accumulated base-field values that predate the unified schema (date-only
  timestamps, legacy status vocabularies), and a *present* value the awk doesn't
  touch reaches the validator and fails AC-1 corpus-wide. So the rule for **every
  base field** is: a present value that does not conform is **deterministically
  normalised to a conforming value, or — where no deterministic normalisation
  exists — emits a counted `0007-DIVERGE` / `0007-REFUSE`**; it is never silently
  passed through. The known nonconforming fields are enumerated below (status,
  dates); the principle guards against the next one too.
- **Status normalisation (legacy → canonical; routed through a new ADR).** The map
  below is derived from an **exhaustive scan of every type's actual `status:`
  values against its `templates-schema.tsv` `status_vocab`** (not just plans) — so
  the enumeration is complete by construction, not discovered field-by-field. The
  decision is: collapse terminal/realised states to each type's canonical value;
  widen a vocab only where a value is a genuine distinct lifecycle state.
  Out-of-vocab values found, and their mapping:
  - **plan** (vocab `draft|ready|in-progress|done`): `accepted` / `complete` /
    `implemented` / `final` / `revised` → `done` (realised); `approved` /
    `reviewed` → `ready` (signed-off but not realised — kept distinct from done);
    `draft`/`ready`/`in-progress`/`done` unchanged.
  - **plan-review** (vocab `complete`): `accepted` → `complete` (an accepted review
    is complete).
  - **design-gap** (vocab `draft`): `accepted` → **widen the design-gap
    `status_vocab` to admit `accepted`** (acknowledged is a genuine gap-record
    state, mirroring design-inventory's `superseded`); `draft` unchanged.
  - **design-inventory** (vocab `draft`): `superseded` → **widen the
    `status_vocab` to admit `superseded`** (a genuine inventory-lifecycle state);
    `draft` unchanged.
  - **note**: the lone `draft` → `captured` (handled by the §3 note backfill).
  - **work-item, adr, codebase-research, work-item-review, plan-validation,
    issue-research, pr-description, pr-review**: all present `status:` values
    already conform — verified, no mapping needed.
  - **Catch-all**: any `status:` value not covered above → `0007-DIVERGE[unmapped-status]`
    (don't guess) — and because the enumeration was derived from a full corpus
    scan, the catch-all should fire on **nothing** in this corpus; the dogfood's
    in-run self-validation re-confirms zero residual out-of-vocab status.
  - This is a **schema-vocabulary decision** (collapsing the plan lifecycle vocab;
    widening design-gap's and design-inventory's) — recorded in **ADR-0042**
    (accepted). **Single-source the map
    concretely**: the legacy→canonical pairs live in one checked-in data file
    (e.g. `scripts/status-legacy-map.tsv`, `type⇥legacy⇥canonical`), consumed by
    the awk; the vocab targets stay in `templates-schema.tsv`. The structural
    invariant — *every map target is a literal in the matched type's
    `status_vocab`* — is fixture-asserted (and caught by the in-run
    self-validation otherwise), so the awk can never map to a value the validator
    rejects. A fixture pins each legacy value → its canonical target and an
    unmapped value → `0007-DIVERGE[unmapped-status]`.
- `skill:` → `producer:`.
- Own-identity `work_item_id:` → quoted `id:` (work-items); own-identity
  `adr_id:` → quoted `id:` (ADRs, **including ADR-0033 itself**). **Foreign**
  `<type>_id` references (e.g. a plan's foreign `work_item_id:`) left in place
  (but see the linkage-normalisation rule below, which derives `parent:` from
  them). **Every** `id:` value is quoted.
- **`id:` derivation for fenced files that have neither an `id:` nor a legacy
  own-id key** (many older research/plan/review/validation files predate the
  identity contract): derive `id:` from the **filename stem** (quoted), the same
  full-stem convention the backfill and the path→typed rule use — so an inbound
  `"plan:<stem>"` / `"…-research:<stem>"` typed ref resolves to it. (Without this,
  these files would emit no `id:` and fail the required-base-field check.) A
  fixture pins a fenced research file with no own-id key emitting `id:` = its stem.
- **Value quoting/refusal is single-sourced via extraction, not copy.** 0006's
  `normalise_value`/`refuses` are *awk-internal* functions, not sourceable
  primitives — "reuse" is impossible without a mechanism. Extract them (and the
  fence-region state-machine scaffold) into a shared awk fragment, e.g.
  `skills/config/migrate/scripts/frontmatter-frag.awk`, concatenated into 0007's
  awk program via `awk -f frontmatter-frag.awk -f body.awk`. A fixture
  asserts 0007 and the extracted fragment quote/REFUSE an identical battery of
  edge-case values (single-quoted, embedded `"`/`\`, unsafe `#`, bare `"`). (0006
  itself is already applied and frozen; the fragment is the forward home — 0006
  need not be retro-refactored, but the equivalence fixture pins parity.) Name it
  `frontmatter-frag.awk` (match the existing `.awk` convention, not `.inc`); it
  exports **functions only** (no top-level rules / no `BEGIN`), ends with a
  trailing newline, and is assembled via `awk -f frag.awk -f body.awk` (the
  `body.awk` owns all `BEGIN`/state/rules). It opens with a **plain comment
  header, not an `awk -f` shebang** (it is include-only, never executed directly),
  and a grep guard asserts the fragment contains zero top-level pattern/action or
  `BEGIN`/`END` blocks. The parity fixture invokes the **genuine two-`-f` form**
  (not a pre-concatenated temp) under **BSD awk (macOS)**, not only CI gawk.
- **The location map and the two shared helpers have one source each, with a
  producer↔validator cross-check.** The directory→doc-type map is needed in
  *both* the shell walker (`type:` inference) and the awk pass (path→typed
  id-extraction); pass it into awk via `-v` pairs generated from the single
  shell-side map (as 0006 passes its `has_*` flags), never hand-copy it. Use a
  **non-escape, non-regex-special single-byte delimiter** to encode the map into
  the scalar (POSIX `awk -v` runs backslash-escape processing on values, and
  `split()`'s separator is an ERE) — split with a one-char literal separator. Two
  distinct shared frontmatter artifacts exist — `frontmatter-frag.awk` *produces*
  valid frontmatter, the Phase 1 emission-rules helper *asserts* what valid
  frontmatter is; add a cross-check fixture that runs awk-produced output through
  the emission-rules helper so **producer and validator** provably agree (not just
  the two validator surfaces).
- **Frontmatter linkage normalisation (deterministic, net scope — see Current
  State).** Pre-existing frontmatter linkage from 0065/0066 is in legacy shapes:
  **139 `target:` values are path-shape** (`"meta/work/0030-….md"`), 21 are
  bare-number, only 48 are already typed. These are *not* body-section inferences
  and are handled in this mechanical awk pass, not the interactive hook:
  Only **fence-region** lines are touched — body-region occurrences (path-shape
  strings in code/example blocks, some pointing at fictional files like
  `2026-01-01-first-plan.md` / `meta/prs/42-…md`) are left untouched by the same
  fence-vs-body separation 0006 uses (fixture-pinned).
  - **Path-shape → typed**, with **id-extraction keyed off the *target*
    doc-type** (this is the load-bearing detail): the directory gives the
    doc-type (location map); the id is then extracted per that doc-type's identity
    convention — **work-item/ADR targets → the bare number / `ADR-NNNN`**
    (`meta/work/0030-foo.md` → `work-item:0030`), but **plan / research /
    review / validation / note targets → the *full filename stem*** (the way
    those types' own `id:` is set), e.g.
    `meta/plans/2026-05-13-0055-sidebar-activity-feed.md` →
    `plan:2026-05-13-0055-sidebar-activity-feed` (NOT `plan:0055`, which no plan's
    `id:` matches). A single bare-number rule would dangle ~83 plan-review targets.
    A path-shape value whose **directory is not in the location map** (`specs/`,
    `talks/`, `global/`, a typo) is **not guessed** — emit `0007-DIVERGE` and
    leave it untouched. Applies to every linkage key (`target`, `parent`,
    `relates_to`, `derived_from`, …). Golden fixture per target doc-type asserts
    the emitted value equals the target file's actual `id:`.
  - **Strip the redundant `work-item-review` `work_item_id:` alias** — gated on
    the **state predicate "`target:` is present and typed"** (not "converted this
    run", so it fires for both already-typed-target and just-converted reviews).
    **Compare before stripping**: if the alias's resolved id disagrees with the
    typed `target:`, do **not** silently drop it — emit a counted `0007-DIVERGE`
    (gap-fix log) and keep it. Done over the ~49 existing review artifacts,
    coordinated with the Phase 5 §4 schema-row edit.
  - **Derive `parent:` from a retained foreign `work_item_id:`** on
    plans/research/pr-description, **normalising the foreign value's three live
    shapes to a bare id first** — bare (`"0079"`), path-shape
    (`"meta/work/0072-….md"`), already-typed (`"work-item:0101"`) — then emit
    `parent: "work-item:<id>"` (never `work-item:work-item:0101` or
    `work-item:meta/...`). **Existence-check** the derived target against the
    corpus index; if it resolves to zero or >1 work-items, emit `0007-DIVERGE` and
    skip the derivation (do not write a wrong-but-resolvable edge). **Precedence /
    idempotency**: replace an empty `parent: ""` placeholder in place; no-op when
    `parent:` already equals the derived value; emit `0007-DIVERGE` and keep the
    existing value when a present non-empty `parent:` disagrees (never duplicate
    the key — that would break the `cmp -s` re-run no-op).
  - **Bare-number `"NNNN"`** linkage: where the **key + source-type pairing is
    deterministic** per ADR-0034's table (e.g. `parent:` on a work-item → a
    work-item; existence-checked), convert **mechanically**. Only genuinely
    **multi-candidate** bare numbers (resolvable to either a work-item or an ADR)
    route to the interactive hook. Routing a *frontmatter* bare number to the hook
    needs plumbing the body-only parser lacks (see §4): the awk pass emits such
    values to a **side-channel** that `migration_emit_transformations` reads
    alongside the parser output. **Side-channel lifecycle** (specified, not
    implicit): a run-scoped file under `.accelerator/state/` (e.g.
    `migrations-0007-frontmatter-linkage.tmp`), **truncated at the start of each
    run**, one record per value mirroring the harness
    `(path, anchor, proposed, band)` shape, with the anchor in a **distinct
    `frontmatter:<key>` namespace** so it cannot collide with a body-section
    `## …` anchor on the same file. For a **list-valued** linkage key
    (`relates_to`/`blocks`) that holds two ambiguous bare numbers, the anchor must
    be **further qualified by the list index** (`frontmatter:relates_to#0`,
    `#1`) — index, not value, so two *identical* bare numbers in one list still get
    distinct `(artifact_path, source_anchor)` resume keys (an unqualified
    `frontmatter:<key>`, or a value-qualified one under a duplicate value, would
    collide and drop the second prompt on resume). A fixture asserts a
    frontmatter `target: "0030"` reaches `PROMPT`, a **list-valued key with two
    ambiguous entries** yields two distinct prompts surviving an interrupt-rerun,
    and the side-channel does not double-emit or drop a prompt.
- Provenance bundle (`revision`, `repository`) added to **every type whose
  `code_state_anchored=yes` in `templates-schema.tsv`** (derive from the TSV
  column, not a hard-coded list — `note` is anchored too); `git_commit`/`branch`
  removed.
- Per-type extras per `templates-schema.tsv`.
- Omit-when-empty (ADR-0040): never emit empty placeholders.
- Diagnostics: `0007-DIVERGE` / `0007-REFUSE` / `0007-MALFORMED` on stderr via
  `log_warn`, same conventions as 0006. Each `0007-DIVERGE` carries a **stable
  sub-reason tag** (`[unmapped-dir]`, `[parent-ambiguous]`, `[parent-conflict]`,
  `[alias-conflict]`, `[reverse-orphan]`, `[author-lookup-failed]`,
  `[unmapped-status]`, and a generic `[nonconforming-base-field]` for the general
  principle's catch-all) so the gap-fix log and any programmatic count group by
  cause. The general principle's enforcement backstop is the **in-run structural
  self-validation** (below): any present value neither normalised nor DIVERGEd is
  caught there and exits non-zero — so the principle is enforced, not aspirational.
  **Unlike 0006, a `REFUSE` or
  `MALFORMED` is fatal**: the migration counts occurrences and **exits non-zero**
  if any fire (the Desired End State demands *zero*, and a refused key otherwise
  leaves a silently half-migrated file that ships looking complete). To shrink the
  partial-mutation window, **step-0 also read-only dry-runs the value-quoting/
  refusal predicate over every fence-region value**, so an unquotable value fails
  before any write; a `REFUSE` that still fires mid-step-2 leaves a
  partially-rewritten tree whose **only** sanctioned recovery is VCS revert to the
  named pre-migration point (this is a recoverable partial-mutation, distinct from
  step-0's zero-mutation guarantee — stated so the asymmetry isn't mistaken for
  full atomicity). The clean-tree pre-flight blocks a naive re-run on the dirty
  tree.
- **Self-validation in two stages** (linkage isn't written until `harness_run`):
  (1) after the mechanical passes, run the validator's **structural checks** over
  touched files (shape, base fields, quoting, omit-when-empty) — *not* referential
  integrity, which is a whole-corpus property not yet fully populated; (2) **after
  `harness_run` completes**, run the **full** validator (incl. referential
  integrity) over the final corpus so the interactive apply path's writes are
  covered in-run. Fail loudly (non-zero) on any violation at either stage. The
  referential-integrity index resolves identity by the same rule the migration
  uses (`id:` → legacy own-id → filename) so a mid-flight/legacy-keyed target
  still resolves.

#### 3. Backfill pass for the fence-less + partial-fence files

**File**: same migration
**Changes**:
- **14 notes** (dispatched by location `meta/notes/`, **not** by absence-of-fence)
  → the `note` baseline shape matching `templates/note.md` / `create-note`
  (0067): `type: note`, `schema_version: 1`, `id`/`title`/`date`/`topic` inferred
  from filename + H1, `producer: create-note`, `status: captured`, `tags: []`,
  provenance bundle, `last_updated`/`last_updated_by`. This includes the **1
  partial-fence note** (`2026-04-17-security-lens-owasp-ai-top-10.md`): its
  existing `---`-delimited block (with a trailing-whitespace fence line) is
  reconciled into the full note shape and its **`status: draft` → `captured`**
  (forced: the `note` status vocab is `captured` only). Routing by location is
  what catches it — an absence-of-fence gate would miss it and leave it
  type-less and out-of-vocab.
- **18 pre-convention plans** → full `plan` baseline, **only after inspecting
  each to confirm it is genuinely a plan** (any prose-only design note is
  excluded or typed correctly, not force-typed `plan`): `type: plan`,
  `id`/`title` (from H1)/`date` (from filename)/`author` (from VCS, `Unknown`
  fallback), `status` left unset, `tags: []`, `schema_version: 1`, the
  code-state-anchored provenance bundle (best-effort `revision`/`repository`),
  `producer` omitted (unknown for hand-written legacy plans).

**Filename dates → ISO timestamp by string concatenation (no `date(1)`).** The
templates (`note.md`, `plan.md`) and the whole fenced corpus use a full ISO
timestamp for `date:`/`last_updated:`; a bare `YYYY-MM-DD` would (a) risk
validator rejection on `last_updated` and (b) break the work item's
"shape-consistent with `create-note`" requirement. So the backfill takes the
filename's `YYYY-MM-DD` prefix and **string-concatenates** a fixed midnight-UTC
suffix → `date: "YYYY-MM-DDT00:00:00+00:00"` (date known, time unknown →
midnight; no `date(1)` round-trip, so GNU-vs-BSD flag differences never bite).
`last_updated`/`last_updated_by` seed from that same value/`author`. `id:` is the
full dated stem. The Phase 1 validator must accept **both** ISO-8601 offset forms
for `date:`/`last_updated:` — the `Z` (Zulu) form and the `±HH:MM` form — because
the untouched corpus already carries both (e.g. `…T21:38:10Z` alongside
`…+00:00`); the backfill emits the `+00:00` form. Good fixtures cover both `Z`
and `+00:00`; one malformed fixture pins rejection.

**VCS author resolution is pinned and single-sourced.** No existing helper
resolves a *per-file historical* author, so this is net-new and must be
deterministic across environments:
- Auto-detect `.jj` then `.git` (reuse the runner's detection), and pin the exact
  query: a `jj log --template …` form and a `git log --format=… -- <path>` form,
  with a stated **minimum version** for each. Force `LANG=C` for the invocation
  (VCS output is locale-sensitive).
- **Distinguish a true empty history from a lookup *failure***: a genuine absence
  → `Unknown`; a *failed* invocation (missing binary, unsupported flag, error
  exit) → `Unknown` **plus a counted `0007-DIVERGE`-style diagnostic** surfaced in
  the dogfood gap-fix log, so every `Unknown` is an inspectable decision, not a
  silent attribution loss.

#### 4. Interactive linkage hooks

**File**: same migration
**Changes**: Implement the 0069 contract:
- `migration_emit_transformations` — run Phase 2's parser over each artifact's
  body sections **and read the §2 side-channel of multi-candidate frontmatter
  bare-number values**; `harness_emit_transformation` once per reference (from
  either source) with a `band` extra and the `(artifact_path, source_anchor)`
  transformation key. (The side-channel is how a frontmatter `target: "0030"`
  reaches the hook, since the parser itself only reads `## …` body sections.)
  **`harness_run` captures this function's stdout and parses every line as a `TX`
  frame**, so the parser invocation and the side-channel read must send **all
  non-`harness_emit_transformation` output to stderr** — a stray parser
  diagnostic or `cat` of the side-channel on stdout corrupts the TX stream and
  aborts the handshake with a misleading "malformed record". The protocol-hygiene
  fixture asserts the emitter produces **only `TX`-prefixed lines** on stdout
  (drive it with a parser that prints a diagnostic).
- `migration_evaluate_predicate` — `[ "$(harness_field band)" = "ambiguous" ]`
  (exit 0 = prompt; non-zero = mechanical `MECHANICAL_APPLIED`).
- `migration_apply_decision` — insert the typed `"doc-type:id"` linkage into the
  **existing frontmatter block** (canonical side only; omit-when-empty), atomic
  temp-then-rename. This is a frontmatter *merge*, harder than the 0099 example's
  body-append: it **must reuse the same fence-aware awk machinery as the §2
  deterministic rewrite** (one shared frontmatter-edit helper, one key-ordering
  rule) so the apply path and the rewrite path produce byte-identical frontmatter
  and stay idempotent. State the insertion point / key order explicitly.
- **Reverse-side reconciliation (committed rule)**: only the canonical side
  (`blocks`, `supersedes`) is written. For any *pre-existing* reverse-side key
  (e.g. a hand-written `blocked_by`), the rule is: **ensure the canonical edge
  exists on the target, then drop the reverse key** — but if the canonical edge
  is *absent* and cannot be derived, emit a counted `0007-DIVERGE` and **keep**
  the reverse key rather than silently losing the relationship. This is a
  deterministic awk rule (covered by the idempotency + self-validation fixtures),
  not a dogfood-time judgement. The canonical-existence check **normalises both
  the reverse value and the target's canonical-side values to the same
  `doc-type:id` shape before comparing** (else a shape mismatch spuriously
  fails-to-find and keeps+DIVERGEs). When the canonical edge must be *written* to
  the target file (cross-file mutation), **write the canonical-edge-on-target
  before dropping the reverse key on the source**, so an abort between the two
  never leaves the relationship wholly absent (it leaves a tolerable transient
  both-sides state, reconciled on re-run). This fires on the real ADR-0026 ↔
  ADR-0036/0039 dual-supersede case — and dropping `superseded_by:` from ADR-0026
  **mutates an accepted, immutable ADR**: that is a conscious, recorded decision
  (a frontmatter-only normalisation, not a content change), noted in the gap-fix
  log. A named fixture covers both branches (canonical present → drop; canonical
  absent/underivable → keep + counted `0007-DIVERGE`) and the shape-mismatch case.
- A **skipped** ambiguous reference writes no linkage and reaches
  `APPLIED_CONFIRM` with no mutation. This is a legitimate terminal, but the
  relationship the prose asserted is then absent from frontmatter with no residual
  marker — so **every skip is recorded in the dogfood gap-fix log** (the session
  log is the audit trail), making it an accountable decision, not a silent loss.
- `migration_validate_edit` — reject malformed edited values.
- `migration_verify_applied` — confirm the recorded linkage is present (drives
  resume `DRIFT` recovery).
- `migration_session_log_path` — `.accelerator/state/migrations-0007-session.jsonl`,
  `transformation_key` first field.
- **`harness_run` is NOT the last line.** It drives the interactive phase and
  returns after emitting `DONE`; the **stage-2 full validation (incl. referential
  integrity) runs *after* `harness_run` returns** and must `exit` non-zero on any
  violation so the runner's clean-exit check withholds the ledger entry. (If
  `harness_run` were literally last, the post-linkage referential-integrity gate
  would never run and a dangling-reference corpus would ship recorded-as-applied.)
  A fixture injects a dangling reference via the apply path and asserts it is
  caught in-run with the ID left pending. **Watchdog budget**: the runner arms a
  ~30s post-`DONE` watchdog that SIGKILLs the migration if it hasn't exited — a
  whole-corpus (~487-file) referential-integrity pass must complete well within
  that window (the Performance section confirms the walk is fast), or stage-2
  runs as a runner-invoked post-step *outside* the watchdog; otherwise a slow-but-
  correct run is killed and mis-reported as "exited unexpectedly". Pin the
  measured stage-2 time in the dogfood record.

#### 5. Migration fixture tests

**File**: a **new standalone discoverable suite**
`skills/config/migrate/scripts/test-migrate-0007.sh` (exec bit set, so
`run_shell_suites` picks it up) + fixtures under
`skills/config/migrate/scripts/test-fixtures/.../0007-*`. Because this adds one
discoverable `test-*.sh`, **bump `_EXPECTED_MIGRATE_SUITES` 3 → 4** in
`tasks/test/integration.py` (the bump is correct *only* because a new suite file
is added — extending `test-migrate-interactive.sh` alone would not warrant it).
The interactive scripted-decision drive can still extend
`test-migrate-interactive.sh`.
**Changes**: A fixture corpus (one of each of the 13 types, plus references
hitting each band). Golden expected output for the mechanical rewrite + backfill.
Scripted hook decisions (accept/edit/skip) driving the known-ambiguous fixtures
to their expected linkage and to `APPLIED_CONFIRM`. Plus, explicitly:
- **Irregular-file fixtures** (where non-idempotency and mis-typing hide): the
  partial-fence note (trailing-whitespace `---`, `status: draft`); a fence-less
  note; a fence-less pre-convention plan with a date-only filename stem;
  ADR-0033's own legacy `adr_id:`; an already-typed `design-gap`/`design-inventory`
  file (assert location inference does **not** override its `type:`); a review in
  each of `reviews/plans/` and `reviews/work/` (assert subdir-derived type).
- **Idempotency / convergence**: re-running the script directly (ledger bypassed)
  over the migrated fixture corpus produces an **empty diff** (all three planes);
  and an **interrupt-then-re-run** over a half-written corpus reaches the same
  fixed point (proves the mechanical pass is independently convergent, not just
  no-op on a clean already-migrated corpus).
- **Author resolution on both branches *and* both VCS**: history present → original
  committer; history absent → `Unknown`; a **git-only** repo and a **jj** repo
  each exercised; a *failed* lookup emits the counted diagnostic.
- **Precondition fixture**: a work-item missing `kind:` (or an unquoted foreign
  `work_item_id:`) → `0007-REFUSE` and non-zero exit (from the read-only pre-pass,
  with **zero files mutated**), not a silent mis-rewrite.
- **Frontmatter linkage normalisation** (covers the pass-3 criticals):
  - path→typed **per target doc-type**: `meta/work/0030-foo.md` → `work-item:0030`
    (bare number) AND `meta/plans/2026-05-13-0055-…md` → `plan:2026-05-13-0055-…`
    (full stem, NOT `plan:0055`) — assert the emitted value equals the target
    file's actual `id:`, one fixture per target doc-type;
  - a path-shape value whose directory is **not in the location map** → `0007-DIVERGE`,
    untouched;
  - **body-region** path-shape/foreign-id occurrence (in a code block) → untouched
    (fence-vs-body separation);
  - parent-derivation from foreign `work_item_id:` in **all three shapes** (bare,
    path-shape, already-typed `work-item:0101`) → correct `parent: "work-item:NNNN"`,
    no double-prefix; existence-check failure → `0007-DIVERGE`+skip; pre-existing
    `parent: ""` placeholder replaced; agreeing `parent:` no-op; disagreeing
    `parent:` kept + `0007-DIVERGE`;
  - alias strip on an **already-typed-target** review AND a just-converted one;
    alias **disagreeing** with `target:` → kept + `0007-DIVERGE`;
  - **duplicate-id precondition**: a work-item whose own `work_item_id:` ≠ filename
    id (the 0032/0031 shape) → step-0 `0007-REFUSE`, zero mutations;
  - a deterministic `parent: "0057"` on a work-item converts mechanically
    (existence-checked); the golden suite asserts the side-channel **file** is
    written for a multi-candidate `target: "0030"`, while the **`^PROMPT\t` frame
    assertion** for that value lives in the interactive-harness drive
    (`test-migrate-interactive.sh`, which stands up the FIFO), not the golden suite;
  - **`id:`-from-stem**: a fenced research/plan file with no own-id key and no
    `id:` emits `id:` = its filename stem;
  - **date-only → ISO**: a fenced file with `last_updated: "2026-02-22"` (and one
    with a date-only `date:`) is rewritten to the full ISO form; an already-ISO
    (`Z` and `+00:00`) value is a no-op; a `last_updated:` *seeded* from a
    date-only `date:` emits the ISO form (normalise-before-seed).
  - **status normalisation**: plan `accepted`/`complete`/`implemented`/`final`/
    `revised` → `done` and `approved`/`reviewed` → `ready`; `plan-review accepted`
    → `complete`; an in-vocab value is a no-op; an unmapped value → `0007-DIVERGE`;
    `design-gap accepted` / `design-inventory superseded` validate against the
    widened vocab (not rewritten).
- **Producer→validator cross-check**: awk-produced frontmatter for each of the 13
  types passes the Phase 1 emission-rules helper (proves the producer emits what
  the validator accepts, distinct from the validator↔test-template single-source
  guard).
- **Reverse-side reconciliation**: the ADR-0026 ↔ ADR-0036/0039 dual-supersede
  case — canonical present → `superseded_by` dropped; canonical absent/underivable
  → kept + counted `0007-DIVERGE`; plus the shape-mismatch normalise-before-compare
  case.
- **Quoting parity**: 0007 and the extracted `frontmatter-frag.awk` fragment
  quote/REFUSE an identical edge-case battery byte-for-byte (via the genuine
  two-`-f` invocation under BSD awk).
- **Negative self-validation**: a deliberately-corrupted rewrite (or injected
  violation) is caught by the in-run self-validation and the migration exits
  non-zero — proving the gate is wired, not just green-path.
- **No-placeholder emission**: assert the migration never *writes* an empty
  `parent: ""`/`blocks: []` placeholder (AC-10 at the emission site, not only via
  the validator over the dogfood corpus).
- **Protocol hygiene**: assert **no non-frame bytes precede `READY`** on stdout.
- **Apply-path proof**: at least one fixture drives an `accepted`/`edited`
  outcome to a written linkage (not exclusively `skipped`), so the apply path is
  genuinely exercised.

### Success Criteria:

#### Automated Verification:
- [ ] Mechanical-rewrite golden test passes (base fields, defaults,
      `skill:`→`producer:`, own-id→quoted `id:`, provenance bundle, extras,
      omit-when-empty, `schema_version: 1`) — AC-3/4/5/6 + defaults
- [ ] Backfill golden test passes for notes (incl. the partial-fence note,
      `status: draft`→`captured`) and confirmed pre-convention plans
- [ ] Author resolution verified on both branches **and both VCS** (jj + git;
      history present → committer, absent → `Unknown`, failed lookup → counted
      diagnostic) — AC-15
- [ ] `last_updated` seeded from `date` where first set, byte-stable on re-run — AC-16
- [ ] Precondition fixture: work-item missing `kind:` / unquoted foreign
      `work_item_id:` → `0007-REFUSE` + non-zero exit — AC-2
- [ ] Migration **exits non-zero** when any `REFUSE`/`MALFORMED` fires
- [ ] No-placeholder emission asserted at the migration's write site — AC-10
- [ ] Location inference does not override an existing `type:`
      (design-gap/design-inventory fixtures); reviews typed by subdirectory
- [ ] Interactive scripted-decision test: known-ambiguous fixtures reach
      `APPLIED_CONFIRM` and produce the expected linkage; at least one
      `accepted`/`edited` outcome writes a linkage (apply path exercised) — AC-9
- [ ] Resolved-band references apply mechanically; ambiguous-band route to the
      hook — AC-7
- [ ] No non-frame bytes precede `READY` on stdout (FIFO protocol hygiene)
- [ ] Every migrated fixture passes the Phase 1 validator (incl. referential
      integrity)
- [ ] Direct script re-invocation (ledger bypassed) over the migrated fixture
      corpus is a byte-identical no-op; an interrupt-then-re-run over a
      half-written corpus converges to the same result — AC-17
- [ ] ADR-0033's `adr_id:` is rewritten to quoted `id:` in the fixture — AC-14

#### Manual Verification:
- [ ] Spot-check that linkage values are typed `"doc-type:id"`, canonical side
      only
- [ ] The interactive prompts render legible prose context for ambiguous refs

---

## Phase 4: End-to-End Dogfood + Gap-Fix

### Overview

Run `0007` against this repo's full `meta/` corpus — the single binding
acceptance gate that exercises all components together — measure the resolved-band
wrong-rate, resolve ambiguous references interactively, validate, and record the
gap-fix log. The deliverable is the migrated corpus + dogfood report + ledger
entry.

### Changes Required:

#### 1. Dogfood run

**Changes**: **Capture a named pre-migration VCS point** (bookmark/commit) first,
so the exact rollback target is unambiguous. On a clean tree, run
`/accelerator:migrate` (or the runner directly). Drive the interactive session
for ambiguous references, **driving a representative subset to an applied
terminal** (not skipping every one) so the apply path runs against real data.
Then run the Phase 1 validator (incl. referential integrity) over the whole
migrated corpus.

**Two distinct re-run checks** (don't conflate them): (a) an immediate
`/accelerator:migrate` re-run **skips 0007 at the ledger** — this only proves the
ledger plane; (b) a **direct script re-invocation with the ledger bypassed**,
asserting an **empty VCS diff**, is what actually proves the `cmp -s`/resume
no-op against the real corpus.

#### 2. Resolved-band wrong-rate measurement (AC-8)

**Changes**: Re-draw a **stratified sample of ≥150 resolved-band linkages**
across the five header types **against the current 487-file corpus** (not the
spike's 381). **Pin the procedure for reproducibility** (the spike used seed=42):
a checked-in script that draws the sample with a **fixed seed and recorded
stratification**, and emits the specific `(file, anchor, emitted-linkage)` rows
to be classified. Classify each correct/wrong by comparing the emitted linkage to
the source prose. Require wrong-rate **≤5%**. **Record the classified sample**
(which rows were judged correct/wrong) in the dogfood report so the tolerance is
auditable and any wrong rows can be located and corrected later. If the corpus
holds <150 resolved-band linkages total, sample the full set.

Note this manual sample is a *backstop*; the per-reference correctness gate is
the Phase 2 resolved-band golden-target fixture set, which catches a parser
regression automatically rather than only when it lands in a sample.

#### 3. Gap-fix log + validation report

**File**: `meta/validations/2026-06-07-0070-*.md` (dogfood report) + a gap-fix log
**Changes**: Record every annotated `DIVERGE` with a one-line rationale (incl. the
deliberately out-of-scope `specs/` + `talks/` files); record the wrong-rate
result; confirm the interactive session log shows every routed reference at
`APPLIED_CONFIRM` (skips counted as valid terminals — decision). Fix any gaps the
pass surfaces.

### Success Criteria:

#### Automated Verification:
- [ ] Runner exits 0; a programmatic grep of the run output confirms **zero
      `0007-REFUSE`/`0007-MALFORMED`** lines (not a manual eyeball) — AC-1
- [ ] Every migrated file passes `scripts/validate-corpus-frontmatter.sh`,
      including referential integrity — AC-1
- [ ] Direct script re-invocation (ledger bypassed) produces an **empty VCS
      diff** over the real corpus — AC-1/AC-17
- [ ] Session log: every routed reference at `APPLIED_CONFIRM`, none in
      `PROMPT`/`VALIDATE_ERR`/`DRIFT`; **at least some `accepted`/`edited`
      outcomes** (apply path exercised on real data, not all skips) — AC-9
- [ ] No empty-placeholder keys in the migrated corpus (validator check) — AC-10

#### Manual Verification:
- [ ] Resolved-band wrong-rate ≤5% on the re-drawn ≥150 sample (fixed-seed,
      reproducible draw; classified sample recorded in the report) — AC-8
- [ ] Zero un-annotated `DIVERGE` lines in the migration report — AC-1
- [ ] Inferred `type:` distribution reconciled against known corpus counts (so an
      off-pattern filename surfaces as a count mismatch, not a silent mis-type)
- [ ] Dogfood report records: the rationale for each accepted `DIVERGE`; the
      out-of-scope `specs/`/`talks/` files; every `Unknown`-author diagnostic;
      every skipped ambiguous reference

---

## Phase 5: Visualiser-Server Reader Expand + Deprecation + Template Alias

### Overview

**Revised from "fallback removal" to "expand + deprecate"** (see Implementation
Approach). This release makes the reader accept *both* legacy and unified shapes,
adds the missing unified `id:` read path, and emits a deprecation warning when a
legacy fallback fires — but **retains** the fallback arms so a userspace repo that
has not yet run `0007` is not broken. The actual arm removal is a **follow-on
contract story** (raised by this work), to ship once every consuming repo has
migrated. The `work-item-review` template alias **is** removed now (new artifacts
use `target:`; the retained reader fallback still serves any legacy review).

**Phase 5 is two merge units, not one** (the ordering constraints make a single
merge impossible — §1 must land before/with Phase 3, §4 must land at/after Phase
4): **Phase 5a** = the reader-expand (§1 `id:` path + `target_path_from_entry`
typed-WorkItem edge) + the §2/§3 deprecation warnings, merged **before or with
`0007`**; **Phase 5b** = the template + schema-row alias drop (§4), merged **at or
after the Phase 4 dogfood** corpus strip. (The Implementation Approach's
"independently mergeable" applies to the partial-order, not free reordering.)

### Changes Required:

#### 1. `indexer.rs` — add the unified `id:` read path (the load-bearing change)

**File**: `skills/visualisation/visualise/server/src/indexer.rs`
**Changes**: Work-item identity (`:1216-1233`) currently reads only
`work_item_id`. **Add `id:` as the primary frontmatter key** (read `id` first,
then legacy `work_item_id`, then the `extract_id(filename)` fallback). The new
`id:` branch must **route through `work_item_cfg.normalise_id` and the empty-trim
shape check exactly like the existing `work_item_id` branch**, so all three
sources yield one canonical identity shape (a raw `id:` must not bypass
normalisation). Without this, a migrated synced work-item whose filename doesn't
encode its id loses its identity. **All three sources are kept this release**;
emit a `tracing::warn!` deprecation line when the legacy `work_item_id` key or the
filename fallback is the resolving source. Place the `id:` read in shared
`build_entry` so **both `rescan` and incremental `refresh_one`** inherit it. No
tests removed.

**Make the `target:` path→typed conversion edge-preserving.** `target_path_from_entry`
(indexer.rs:~869) resolves `TypedRef::Plan`/`TypedRef::Path` but returns `None`
for `TypedRef::WorkItem` — and it is the sole populator of the path-keyed
`reviews_by_target` reverse index. So once a work-item-review's `target:` is typed
to `work-item:NNNN`, that reverse edge would silently vanish, surviving only via a
*different* index (`work_item_refs_by_id`) that itself depends on the `id:` path
above. **Extend `target_path_from_entry` to resolve `TypedRef::WorkItem(id)`** via
`work_item_by_id`, **canonicalising the raw id through `canonicalise_one_id`
against `work_item_cfg` before the lookup** (matching `cluster_key.rs:87`, so a
project-prefix/under-padded id still resolves) — which means threading
`work_item_cfg` (not just the map) into the function. This is a **signature
change rippling to ~5 call sites** — enumerate and update all: rescan Pass B
(`:369`), `update_reviews_by_target` (`:993`), `remove_from_reviews_by_target`
(`:1038`), `declared_outbound` (`:760`), and the `cluster_key.rs:94` delegating
call — each obtaining a `work_item_by_id` snapshot consistent with its existing
lock order (`refresh_one` must snapshot *after* `update_work_item_by_id`, mirroring
the `plans_snapshot` pattern). Pinning tests: a migrated review with only
`target: "work-item:NNNN"` yields its edges on **both `rescan` and `refresh_one`**,
**and** refreshing a *renamed work-item* re-resolves its inbound review edges (or
document that a work-item path change requires a rescan, consistent with the
existing plan-target behaviour). **Tests to *invert*, not keep green**: the
existing `typed_work_item_target_returns_none_resolved_by_cluster_key_resolver`
pins `target_path_from_entry` returning `None` for a typed work-item target — the
exact behaviour this edit reverses; retarget it to assert the new resolution
(`typed_adr_and_pr_targets_return_none` stays green — only WorkItem gains
resolution). The signature change also breaks the **~10 test-module call sites**
that use the current 3-arg form (the `target_path_from_entry_*` / `*_target_*`
tests) — update them to the new arity (empty `work_item_by_id` + a default
`WorkItemConfig` for most). **Ordering**: this reader-expand (Phase 5a) must ship
**before or with** `0007`, never after (recorded in Migration Notes).

#### 2. `read_ref_keys` `work-item:` arm — keep + deprecate

**File**: `skills/visualisation/visualise/server/src/frontmatter.rs`
**Changes**: **Retain** the `else if let Some(v) = m.get("work-item")` arm
(`:334-341`); add a `tracing::warn!` when it fires. **Correct the prior
assumption**: the lookup is a single else-if chain `work_item_id` → `work-item` →
`ticket` → `target`, so `target:` is the *last* fallback, not read ahead of the
legacy arm — precedence is harmless only because migration guarantees `work-item:`
and `target:` never coexist. **Update the stale doc-comment** (`:299-304`,
`:335-338`) that claims this arm is "removed in the release that closes story
0070" to point at the deferred follow-on contract story instead. The pinning tests
(`:469-477`, `:493-498`) **stay green**. (Removal + test deletion → follow-on.)

#### 3. `parent_or_legacy_id` — keep + deprecate

**File**: `skills/visualisation/visualise/server/src/cluster_key.rs`
**Changes**: **Retain** the `work_item_id` legacy branch (`:125-129`) and the
`parent_or_legacy_id` name; add a `tracing::warn!` when the legacy branch
resolves. (Note: `cluster_key.rs` carries **no** stale "closes story 0070"
removal comment — the two stale comments to retarget both live in
`frontmatter.rs` (`:300`, `:336`), already covered by §2.) The `parent:` branch remains
primary. Existing tests stay green. (Removal + rename → follow-on.) **Note**:
because the migration now *mechanically derives `parent:` from the foreign
`work_item_id:`* (Phase 3 §2), the canonical clustering key is populated this
release, and the foreign id is also retained as the fallback — so clustering is
double-covered now, and the follow-on can remove the legacy branch safely.

#### 4. `work-item-review` template + schema alias — remove now

**File**: `templates/work-item-review.md` + `scripts/templates-schema.tsv`
**Changes**: Remove the transitional `work_item_id:` alias line (`:13`) **and, in
the same commit**, drop the `work_item_id` extra from the `work-item-review` row
in `scripts/templates-schema.tsv` so `test-template-frontmatter.sh` and the new
corpus validator agree. (The **foreign** `work_item_id: ""` on
`codebase-research.md`, `rca.md`, `plan.md`, `pr-description.md` are retained.)
**The ~49 existing `reviews/work/` artifacts** that carry the alias are stripped
of it **by the migration** (Phase 3 §2 frontmatter-linkage normalisation, once
their `target:` is typed) — so the template, schema, and corpus all agree after
the dogfood.

#### 5. Follow-on contract story (deferred — not this release)

**Deliverable of this plan**: raise the follow-on work item now (don't leave it as
a note), so the deprecate-then-contract sequence has an owner and a home and can't
ossify in the expanded-but-never-contracted state. It carries: removal of the
three fallback arms + the `parent_or_legacy_id` name; deletion of the pinning
tests (re-derive the exact line refs against the then-current source — note the
`cluster_key.rs` tests near `:251-281` assert *retained* plan `work_item_id`/
path-shape parent resolution, **not** the legacy work-item arm; the genuine
work-item-review legacy/typed tests are `work_item_review_target_path_resolves_to_work_item_id`
and `…_typed_work_item_target_short_circuits`) and retargeting the survivors to
assert canonical resolution; the migrated work-item AC-12/AC-13; and an
**observable migration-completion gate** for arm removal — a corpus-wide grep
returning zero surviving legacy `work-item:`/`work_item_id:` shapes — rather than a
manual "everyone has migrated" judgement. The `parent:`-orphaning risk is already
closed by this release's mechanical `work_item_id:`→`parent:` derivation (Phase 3
§2), so the follow-on inherits a corpus where the canonical side is populated.

### Success Criteria:

#### Automated Verification:
- [x] `mise run test:unit:visualiser` passes (cargo `--lib`, both feature modes),
      including the **retained** fallback pinning tests (411 default / 415 dev-frontend)
- [x] A new test asserts a work-item with `id:` and no `work_item_id:` resolves
      its identity (the unified read path works, via `normalise_id`)
- [x] **Per-arm** deprecation tests assert a `tracing::warn!` fires from *each* of
      the three retained fallbacks (indexer filename/legacy-key, `read_ref_keys`
      `work-item:`, `cluster_key` legacy branch), each driving its resolving call
      **synchronously on the test thread** — captured via a parallel-safe permissive
      global subscriber routing to a per-thread buffer (`log::test_support::capture_logs`),
      since `with_default` thread-local capture is unreliable against tracing's
      global callsite-interest cache when other parallel tests hit the same callsite
- [ ] (Phase 5b — after dogfood) `scripts/test-template-frontmatter.sh` passes with
      the alias removed and the `work_item_id` extra dropped from the
      `work-item-review.md` schema row
- [x] The retained-fallback pinning tests + the per-arm deprecation tests confirm a
      **deliberately un-migrated** corpus still resolves cross-references via the
      retained fallbacks (no silent breakage)

#### Manual Verification:
- [ ] The visualiser clusters work-item reviews via the `target:` typed ref and
      resolves migrated work-items via the new `id:` path
- [ ] Loading this repo's migrated corpus shows no broken cross-references; loading
      a simulated un-migrated corpus degrades gracefully with deprecation warnings,
      not broken edges
- [ ] The follow-on contract work item has been raised (with AC-12/AC-13 and the
      migration-completion gate)

---

## Testing Strategy

All new shell suites are auto-discovered by the **integration** tasks
(`run_shell_suites`) under their subtree — `scripts/` → `test:integration:config`,
`skills/config/migrate/` → `test:integration:migrate` (bump
`_EXPECTED_MIGRATE_SUITES`) — **not** `tasks/test/unit.py`. New scripts must clear
`lint-bashisms.sh` and a bash-3.2 replay, and use no `\b` regexes.

### Shell suites (integration-discovered):
- Corpus validator: per-type good fixtures + per-failure-mode fixtures (incl.
  dangling-reference) + referential integrity (Phase 1).
- Linkage parser: spike-fix fixtures (no `\b`; hyphen-boundary on macOS),
  band-classification set, **resolved-band golden-target set**, `Source:`-line
  disambiguation, `pr:` tolerance, known-ambiguous set (Phase 2).
- Migration: mechanical-rewrite goldens, backfill goldens (incl. partial-fence
  note), author-resolution both branches × both VCS, precondition-REFUSE,
  no-placeholder emission, protocol hygiene, direct-invocation no-op +
  interrupt-rerun convergence (Phase 3).

### Rust unit (`mise run test:unit:visualiser`):
- New: unified `id:` resolution; deprecation-warning fires on a legacy fallback;
  un-migrated corpus still resolves via retained fallbacks. Existing pinning
  tests **retained** (their removal is the follow-on story).

### Integration Tests:
- `scripts/test-migrate-interactive.sh` scripted-decision drive of the
  known-ambiguous fixtures through the full
  `PROMPT → DECIDE → RECORDED → APPLY → APPLIED_CONFIRM` path + resume.
- End-to-end dogfood against the real corpus (Phase 4).

### Manual Testing Steps:
1. Capture a named pre-migration VCS point. On a clean tree, run
   `/accelerator:migrate`; answer the prompts (drive a representative subset to an
   applied terminal); confirm exit 0 and zero `0007-REFUSE`/`0007-MALFORMED`.
   **Run this on a stock macOS toolchain** at least once — the reused interactive
   harness decodes prompt display via `base64 -d` (GNU spelling; older BSD
   `base64` uses `-D`), so verify the ambiguous-ref prompts render their prose
   context (non-empty) rather than blank.
2. Run the corpus validator over `meta/`; confirm zero violations (incl.
   referential integrity).
3. Re-invoke the script directly (ledger bypassed); confirm an empty VCS diff.
4. Draw the fixed-seed ≥150 resolved-band sample; hand-classify; confirm ≤5%
   wrong; record the classified sample in the report.
5. Build + load the visualiser against the migrated corpus; confirm clustering
   and migrated work-item identity; confirm a simulated un-migrated corpus
   degrades gracefully with deprecation warnings.

## Performance Considerations

The migration walks ~487 files with per-file awk + `cmp -s`; well within
interactive-run tolerances. The interactive session only prompts ambiguous-band
references (a minority per spike 0068), so prompt volume is bounded. Atomic
temp-then-rename per mutation; session-log append under a `.lockdir` mutex.

## Migration Notes

- **Status-vocab ADR (prerequisite)**: **ADR-0042** records the legacy→canonical
  status map (plan `accepted`/`complete`/`implemented`/`final`/`revised` → `done`,
  `approved`/`reviewed` → `ready`; `plan-review accepted` → `complete`) and the
  vocab widenings (`design-gap` +`accepted`, `design-inventory` +`superseded`).
  **Accepted** — prerequisite satisfied; the map is single-sourced
  (`scripts/status-legacy-map.tsv`) so the validator and the awk transform agree.
- **Numbering**: `0007`, verified against the applied ledger (head `0006`) — not
  hard-coded.
- **State path**: `.accelerator/state/` (the directory has moved since ADR-0023;
  the legacy `meta/.migrations-applied` is bridged by 0003, not a live target).
- **Runtime ordering**: `0007` assumes `0005` (`kind:`) and `0006`
  (`work_item_id`/`author`) have already been **applied**; the runner's
  `sort -z` discovery enforces this. Its awk assumes `kind:` present and foreign
  `work_item_id:` quoted.
- **Cross-repo coupling (revised)**: migrate-on-use is *advisory*, not enforced
  (`migrate/SKILL.md`: skills don't gate on pending migrations), so the fallback
  arms **cannot** be safely removed in the same release as the migration. This
  release *expands* the reader (unified `id:` path) and *deprecates* the arms;
  arm removal is a **follow-on contract story** to ship once every consuming repo
  has migrated. The expanded reader accepts both legacy and unified shapes, so
  the migration and the expand ship together safely.
- **Intra-release ordering constraints**: (a) the visualiser **reader-expand
  (Phase 5 §1 `id:` path + `target_path_from_entry` typed-WorkItem edge) must ship
  before or with `0007`**, never after — a repo that migrates against an old
  reader loses work-item identity. (b) The **corpus alias-strip (Phase 4 dogfood)
  must land no later than the schema-row alias drop (Phase 5 §4)** — otherwise a
  corpus validated in the window between them carries a `work_item_id` the schema
  no longer accounts for. The validator treats a residual legacy `work_item_id`
  on a review as a *known-legacy* violation (not an unknown-key hard error) to
  keep the transitional window tolerable.
- **Safety**: clean-tree pre-flight + VCS revert (ADR-0023); no inverse
  migration.

## References

- Original work item: `meta/work/0070-ship-meta-corpus-unified-schema-migration.md`
- Related research:
  `meta/research/codebase/2026-06-07-0070-meta-corpus-unified-schema-migration.md`
- Driving spike:
  `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`
- ADRs: ADR-0023 (migration framework), ADR-0033 (unified base schema), ADR-0034
  (typed linkage vocabulary), ADR-0037 (interactive contract), ADR-0038
  (interactive validation parameters), ADR-0040 (omit-when-empty), ADR-0042
  (pre-schema status reconciliation — accepted)
- Awk precedent: `skills/config/migrate/migrations/0006-canonicalise-work-item-id-and-author.sh`
- Interactive worked example:
  `skills/config/migrate/scripts/test-fixtures/interactive/doc-example/migrations/0099-doc-example.sh`
- Schema source: `scripts/templates-schema.tsv`
- Note baseline shape: `templates/note.md`
- Reader sites (expand + deprecate this release; arm removal → follow-on):
  `skills/visualisation/visualise/server/src/frontmatter.rs:330-341,469-477`,
  `cluster_key.rs:119-131`, `indexer.rs:1216-1233` (add `id:` here). Alias removed
  now: `templates/work-item-review.md:13` + the `work-item-review` row in
  `scripts/templates-schema.tsv`.
