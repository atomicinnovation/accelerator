---
type: codebase-research
id: "2026-06-07-0070-meta-corpus-unified-schema-migration"
title: "Research: Implementing the meta/ corpus unified-schema migration (story 0070)"
date: "2026-06-07T08:36:10+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0070"
parent: "work-item:0070"
relates_to: ["codebase-research:2026-05-24-0068-related-documents-inference-accuracy"]
topic: "Implementing the meta/ corpus unified-schema migration (story 0070)"
tags: [research, codebase, migration, frontmatter, schema, interactive, visualiser, linkage]
revision: "ae318e09c04bb2d7f7b78f4031f73696212e1062"
repository: "ticket-management"
last_updated: "2026-06-07T08:36:10+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Implementing the meta/ corpus unified-schema migration (story 0070)

**Date**: 2026-06-07T08:36:10+00:00
**Author**: Toby Clemson
**Git Commit**: ae318e09c04bb2d7f7b78f4031f73696212e1062
**Branch**: ticket-management workspace (change `qttmrsrwyvpz`, no bookmark)
**Repository**: ticket-management

## Research Question

What does the codebase tell us about how to implement story 0070 — shipping the
numbered Accelerator migration that rewrites every `meta/` artifact to the
unified frontmatter schema, populates typed linkage frontmatter via the
interactive hook, removes the transitional visualiser-server read-path
fallbacks, and backfills `meta/notes/` baseline frontmatter? Where do all the
anchors live, what are the contracts each must obey, and what is the actual
scope of the dogfood corpus?

## Summary

Story 0070 is the closing integration item of epic 0057. Everything it depends
on has shipped: migrations 0005 (`type:`→`kind:`) and 0006
(`work_item_id`/`author` canonicalisation) exist, all templates are unified
(0065/0066/0067), and the interactive migration runner (0069) is in place. The
migration to author is **`0007-*.sh`** — 0006 is the current head in
`skills/config/migrate/migrations/` and the discovery glob
(`[0-9][0-9][0-9][0-9]-*.sh` then `sort -z`) makes 0007 sort next. It must be
**interactive** (declare `# INTERACTIVE: yes` in its first five lines) because
spike 0068 measured the body-section linkage parser at **11.3% wrong** — above
the ≤5% threshold — and mandated the interactive-hook path (ADR-0038).

Five workstreams, bound by one end-to-end dogfood gate:

1. **A deterministic awk frontmatter rewrite** modelled on `0006-…sh` — base
   fields + defaults, `skill:`→`producer:`, own-identity `work_item_id:`/`adr_id:`
   → quoted `id:`, provenance bundle (`revision`/`repository`, drop
   `git_commit`/`branch`), per-type extras, `schema_version: 1`, omit-when-empty.
2. **A net-new body-section linkage parser** (the 0068 prototype was throwaway)
   with the three spike fixes encoded *before* band classification, feeding the
   interactive hook for ambiguous-band inferences.
3. **The interactive migration** built on the 0069 contract (FIFO transport,
   TSV frames, JSONL session log keyed on `(artifact_path, source_anchor)`).
4. **Rust visualiser-server removal** at four sites — the `work-item:` fallback
   in `frontmatter.rs:read_ref_keys`, its pinning test, the `parent_or_legacy_id`
   path in `cluster_key.rs`, and the filename-id fallback in `indexer.rs`.
5. **Notes backfill** of the 14 `meta/notes/` files to the `note` baseline shape
   that `create-note`/`templates/note.md` produce.

The dogfood corpus is **~486 `.md` files** under `meta/` (not the 381 the spike
measured — the corpus has grown). Legacy-key prevalence: `skill:` in 217 files,
`work_item_id:` in 167, `git_commit:`/`branch:` in 73 each, `adr_id:` in 43,
`researcher:` in 9, hyphenated `work-item:` in 8, and **32 files with no
frontmatter fence** (the 14 notes plus design-inventory captures and prose-only
artifacts). Body-section headers to parse: `## References` (240 files),
`## Dependencies` (74), `## Historical Context` (63), `## Related Research` (60),
`## Source References` (29); `## Related Documents` confirmed absent.

A notable finding: **there is no standalone "unified-schema validator"** that
inspects generated artifacts. AC-1's "validates against the unified-schema
validator" has no implementing program today — enforcement lives only at the
producer-surface contract-test level (`scripts/test-template-frontmatter.sh`,
`scripts/test-skill-frontmatter-population.sh`) and ADR-0040 explicitly records
"No test inspects a generated artifact." This is a gap the story must reconcile
(build a corpus validator, or scope the AC to the existing checks).

## Detailed Findings

### 1. Migration runner, ledger, and numbering

**Runner**: `skills/config/migrate/scripts/run-migrations.sh`.

- **Discovery** (`run-migrations.sh:161-164`): NUL-delimited `find -maxdepth 1
  -name '[0-9][0-9][0-9][0-9]-*.sh' -print0 | sort -z`. Ordering is purely
  string-lexical on the basename; the four-digit prefix is the sole sort key.
  The migration **ID** is the basename minus `.sh` (`:169`).
- **Existing migrations** in `skills/config/migrate/migrations/`: `0001` (rename
  tickets→work), `0002` (project-prefix), `0003` (relocate accelerator-state),
  `0004` (research subcategories), `0005` (`type:`→`kind:`), `0006`
  (`work_item_id`/`author`). **Head is 0006 → author `0007-*.sh`.** Verify
  against the applied ledger rather than hard-coding (story's own caution).
- **Applied ledger** (`run-migrations.sh:39-40`):
  `$PROJECT_ROOT/.accelerator/state/migrations-applied` (and `-skipped`). The
  directory **has already moved** to `.accelerator/state/`; the legacy
  `meta/.migrations-applied` is bridged by migration `0003` (`_merge_state_file`,
  `0003-…sh:189-233`), not by the runner. This resolves the ADR-0023 vs
  ADR-0037/0038 path discrepancy: **use `.accelerator/state/`.**
- **Pending** = ID in neither applied nor skipped (`:204-219`). Recording is
  `atomic_append_unique` after success (`:293-294`); idempotent.
- **`MIGRATION_RESULT: no_op_pending` sentinel** (`:279-291`): a full-line match
  on stdout makes the runner skip the ledger append so the migration **stays
  pending** and re-runs next invocation — distinct from idempotent re-runs
  (which are excluded via the ledger). In the interactive path it is only valid
  *before* `READY` (`interactive-lib.sh:380-389`).
- **Clean-tree pre-flight** (`:67-141`, ADR-0023): VCS auto-detect (`.jj`→jj,
  else `.git`→git), dirty check scoped to `meta/|.claude/accelerator|.accelerator/`,
  bypassed by `ACCELERATOR_MIGRATE_FORCE`. Detects in-flight interactive session
  logs and prints a resume/discard message.
- **Diagnostics** (`DIVERGE`/`REFUSE`/`MALFORMED`) are **migration-authored**,
  not a runner primitive. The runner captures stdout+stderr and relays them
  (`:285`); emitting them does **not** fail the run — only a non-zero exit fails
  (aborts the whole run, ID not recorded). A `0007-` migration may adopt the
  same `0007-DIVERGE/REFUSE/MALFORMED` prefix convention freely.

### 2. The awk rewrite precedent — migration 0006

`migrations/0006-canonicalise-work-item-id-and-author.sh` is the model to copy.

- **Embedded awk state machine** (`0006:80-242`): two independent flags seeded in
  `BEGIN`. `in_frontmatter` + a `seen_frontmatter_open` latch bound the fence
  region (`:132-140`, strict `/^---$/`); `saw_first_h2` flips on the first
  `/^## /` (`:141`). Frontmatter rules guard on `in_frontmatter && /…/`; body-label
  rules guard on `!saw_first_h2 && /…/`. The key idiom: **non-terminating
  detector rules** at the top (`:128-130`, set `saw_*_anywhere`, no `next`)
  layered over terminating transform rules below, with a catch-all `{ print }`
  (`:225`).
- **Change-detection gate** (`0006:270-280`): transform to a temp file, then
  `if ! cmp -s "$file" "$tmp_out"; then atomic_write "$file" <"$tmp_out"; fi`.
  This is filesystem-level idempotency — already-canonical files are byte-identical
  and never rewritten.
- **Fast-skip early return** (`rewrite_file`, `0006:252-257`): `grep -qE` for any
  target key (legacy *and* canonical); returns untouched if none match.
- **Diagnostics** emitted via `print … > "/dev/stderr"`, surfaced through
  `log_warn`: `REFUSE` (unsafe value shape — `refuses()` at `:104-109` flags
  unquoted `#` or bare `"`), `DIVERGE` (two sources for one field disagree —
  `:227-236`), `MALFORMED` (legacy key seen anywhere but no `---` fence —
  `:237-239`).
- **Quoting** (`normalise_value`, `0006:89-98`): already-double-quoted passes
  through; single-quoted is unwrapped and re-escaped to double; bare values are
  double-quoted. `semantic_inner` (`:99-103`) strips one quote layer for
  equality comparisons. **Note the asymmetry**: only `work_item_id` values are
  quote-normalised; `author`/`**Author**` are renamed but not re-quoted. 0070
  must quote **every** `id:` value (ADR-0033 identity contract).
- **Shared libraries** (`0006:8-10`): `scripts/config-common.sh` (corpus path
  resolution; also offers `config_extract_frontmatter`/`config_extract_body`
  with a *looser* `/^---[[:space:]]*$/` fence), `scripts/atomic-common.sh`
  (`atomic_write` at `:16-32`, `atomic_append_unique`, `atomic_jsonl_*`),
  `scripts/log-common.sh` (`log_warn`/`log_die`).
- **Corpus walking**: 0006 walks multiple corpora (`plans research_codebase
  research_issues`, `:333`) plus userspace template overrides, with bash-3.2-safe
  dedup via parallel indexed arrays and path-safety guards (`assert_safe_relpath`,
  `resolve_corpus_path`). 0070 walks the **whole corpus** — extend this pattern
  across work, plans, decisions, research/*, reviews/*, validations, notes.

### 3. The interactive contract (0069)

Protocol/harness live at the **plugin root**: `scripts/interactive-protocol.sh`
and `scripts/interactive-harness.sh`; the runner-side library is
`skills/config/migrate/scripts/interactive-lib.sh`. (The story's reference to
`skills/config/migrate/scripts/interactive-protocol.sh` is a wrong path — it is
at the plugin-root `scripts/`.)

- **Transport** (`interactive-lib.sh:338-360`): two named FIFOs under
  `.accelerator/state/` — `migrations-<id>-r2m.fifo` and `-m2r.fifo` — wired
  bash-3.2-safe (fd 7 read-write on r2m, fd 8 read-only on m2r). The runner
  sends an `INIT` frame carrying `resume_state_path` + `decisions_path`
  (`:364-366`).
- **Frame protocol** (`interactive-protocol.sh:14-32`), TAB-separated,
  line-delimited, JSON never on the wire. Migration→runner: `READY`,
  `MECHANICAL_APPLIED`, `RESUMED_APPLIED`, `RESUMED_SKIPPED`, `PROMPT`,
  `VALIDATE_ERR`, `RECORDED`, `APPLIED_CONFIRM`, `DRIFT`, `DONE`, `FAIL`.
  Runner→migration: `INIT`, `DECIDE`, `APPLY`, `DRIFT_CLEARED`, `ABORT`.
  Field escaping (`:48-93`): backslash→`\\`, TAB→`\t`, newline→`\n`. Multi-line
  content rides as `display_b64`; extras ride as `0x1F`-joined `key=value` pairs.
- **State machine** (`interactive-protocol.sh:34-40`):
  `PROMPT → DECIDE → (VALIDATE_ERR loop) → RECORDED → APPLY → APPLIED_CONFIRM`.
  **Write-ahead-log invariant**: the runner persists the JSONL record on
  `RECORDED` *before* replying `APPLY` (`interactive-lib.sh:495-508`), so a
  recorded accept always persisted before mutation. `VALIDATE_ERR` fires only on
  an `edit` whose `migration_validate_edit` returns non-zero, and loops back to
  a re-prompt without re-emitting `PROMPT`. `DRIFT` fires on resume when live
  `proposed` differs from the recorded value, or when `migration_verify_applied`
  reports the recorded mutation absent → `DRIFT_CLEARED` → fresh `PROMPT`.
- **Terminal states**: success is `APPLIED_CONFIRM` (or `RESUMED_APPLIED`/
  `RESUMED_SKIPPED` on resume); the only non-success terminal is `FAIL` (aborts
  the migration). **Important for AC-7** ("every reference reaches
  `APPLIED_CONFIRM`"): a *skipped* reference reaches `APPLIED_CONFIRM` but
  performs **no mutation** — so an AC demanding *applied* linkage must drive a
  non-skip decision, not merely terminal arrival.
- **Session log** (`interactive-lib.sh:151-187`):
  `.accelerator/state/migrations-<id>-session.jsonl`, atomic append
  (`atomic_jsonl_append`, `atomic-common.sh:177-210`, temp-then-rename +
  `.lockdir` mutex). Canonical field order `transformation_key`, `schema_version:
  1`, `outcome ∈ {accepted,edited,skipped}`, `proposed_value`, `user_value`
  (edited only), `timestamp`, then author extras. `transformation_key` **must be
  the first field** (the remove-by-key matcher anchors on it) — set it to the
  `(artifact_path, source_anchor)` pair.
- **Migration-side API** (hooks): required `migration_emit_transformations`
  (calls `harness_emit_transformation key= path= anchor= proposed=
  predicate_value= display=` once per reference) and `migration_apply_decision
  key path anchor decision value` (accept/edit only, never skip). Optional
  `migration_evaluate_predicate` (exit 0=prompt, 1=mechanical, else FAIL),
  `migration_validate_edit`, `migration_verify_applied`,
  `migration_session_log_path`. Last line calls `harness_run`.
- **Worked example to copy**:
  `skills/config/migrate/scripts/test-fixtures/interactive/doc-example/migrations/0099-doc-example.sh`
  — three transformations (ambiguous→prompt, resolved→mechanical via a `band`
  extra, ambiguous-with-validator). Driven by `scripts/test-migrate-interactive.sh`.

### 4. Body-section linkage parser (net-new)

The 0068 prototype (`/tmp/spike-0068/parser.py`, ~280 lines) was throwaway and
never committed — **build fresh**, encoding the spike's failure-pattern
catalogue. Per ADR-0038, the **three fixes run before the band classifier**:

1. **`template-path` blocklist** — literal placeholders (`ADR-NNNN.md`,
   `YYYY-MM-DD-topic.md`, `{number}-description.md`). 7 of the spike's 17 wrong
   cases, all in `## References` of skill-design artifacts.
2. **Tightened `\bblocks?\b`** — so "code-block" prose produces no `blocks`
   linkage (hyphen is a word boundary). 1 case.
3. **`\bsibling\b` → `relates_to`** — instead of falling through to the
   `(plan, codebase-research) → derived_from` default. 1 case.

**Band rule** (ADR-0038): two bands. `resolved` = parser maps to exactly one
`(source-type, key, target-type)` tuple in ADR-0034's table with no competing
candidate → **apply mechanically**. `ambiguous` = zero or >1 tuples match (e.g.
a bare number resolvable to either a work-item or an ADR) → **route to the hook**
(trigger predicate `band == 'ambiguous'`). Deterministic renames/shape
normalisation are not inferences and always apply mechanically. The cheap fixes
do not change the verdict (~5.3% still over threshold) but reduce prompt count
~6% and ensure ambiguity is *genuine*, not parser bugs.

**Prose disambiguation onto ADR-0034's table**: a plan's `"Source:"` line →
`parent` for a work-item target, `derived_from` for a research target; non-meta
targets → `source`. Emit the typed `"doc-type:id"` form (never bare `"NNNN"`),
write only the canonical bidirectional side (`blocks`, `supersedes`). Tolerate
`pr:` references (not yet in ADR-0034's vocabulary; supplementary ADR pending).

### 5. Visualiser-server removal (Rust)

Crate: `skills/visualisation/visualise/server/` (`accelerator-visualiser`).
Tests run via `mise run test:unit:visualiser` (`tasks/test/unit.py:19-24`, runs
cargo `--lib` twice — with and without `dev-frontend`). Four sites:

| Site | File:lines | Removal |
|---|---|---|
| `work-item:` fallback in `read_ref_keys` | `frontmatter.rs:334-341` (doc 299-300/335-338) | drop the `else if let Some(v)=m.get("work-item")` arm; `ticket:` arm must follow `work_item_id` directly |
| Pinning test | `frontmatter.rs:469-477` | delete `read_ref_keys_reads_legacy_work_item_key_via_transitional_fallback`; review companion `…prefers_work_item_id_over_transitional_work_item` (493-498) |
| `parent_or_legacy_id` legacy branch | `cluster_key.rs:119-131` (legacy arm 125-129), sole call site `:75` | drop the `work_item_id` branch, keep `id_from_value` + the `parent` branch |
| Filename id fallback | `indexer.rs:1233` (block 1212-1233) | drop `.or_else(|| work_item_cfg.extract_id(filename))`; keep `extract_id` (used elsewhere) |

Tests that **break and must be removed**: `frontmatter.rs:469-477`,
`cluster_key.rs:251-265` (`plan_with_work_item_id_frontmatter_resolves`),
`cluster_key.rs:267-281` (`plan_with_path_shape_work_item_id_resolves`). Tests
that **survive but lose their premise** (review):
`frontmatter.rs:493-498`, `cluster_key.rs:283-297`. A grep for
`parent_or_legacy_id` must return nothing after removal (AC-12). The crate is
otherwise field-name-agnostic; `read_ref_keys` feeds `IndexEntry.work_item_refs`
→ reverse cross-ref index → `related.rs:60`.

### 6. Notes backfill

`create-note` (`skills/notes/create-note/SKILL.md`) loads `templates/note.md`
via `config-read-template.sh note` — that template (`templates/note.md:1-19`) is
the authoritative `note` baseline shape the migration must match:

```yaml
type: note
id: "{filename-stem}"
title: "{Note title}"
date: "{ISO timestamp}"
author: "{author from VCS}"
producer: create-note
status: captured
parent: ""        # omit-when-empty
relates_to: []    # omit-when-empty
topic: "{Note topic}"
tags: []
revision: "{commit}"
repository: "{repo}"
last_updated: "{ISO}"
last_updated_by: "{author}"
schema_version: 1
```

`meta/notes/` holds **14 notes**: 13 are bare-H1, no frontmatter; **1**
(`2026-04-17-security-lens-owasp-ai-top-10.md:1-8`) has a partial hand-written
block (`date`, `author`, `tags`, `status: draft` — note: not `captured`). The
migration backfills `type: note`, `schema_version`, `id`/`title`/`date`/`topic`
inferred from filename + H1, and `author` from VCS history with literal
`Unknown` fallback. AC-11 wants both author branches verified (history present →
original committer; absent → `Unknown`). Reconcile the lone `status: draft`
against the canonical `captured`.

### 7. The `work-item-review` alias

`templates/work-item-review.md:13` carries the transitional
`work_item_id: ""` alias (comment: "consumed by visualiser frontmatter.rs:330
until Phase 7 … Removed by the visualiser consumer-update"). Owned by
`review-work-item` (loads via `config-read-template.sh work-item-review`; 0066
moved the frontmatter into the template). **Remove this line.** The other review
templates are clean: `plan-review.md` and `pr-review.md` carry no alias
(`pr-review` uses `pr_number`). Distinct from the **foreign** `work_item_id: ""`
on `codebase-research.md:9`, `rca.md:9`, `plan.md:9`, `pr-description.md:9` —
those are retained (omit-when-empty foreign refs), **not** removed.

### 8. The validator gap (AC-1)

There is **no standalone unified-schema validator** for generated artifacts.
The two contract tests validate *producer surfaces*, not corpus output:

- `scripts/test-template-frontmatter.sh` validates `templates/*.md` against
  `scripts/templates-schema.tsv` (per-type required base fields, `type:` literal,
  `schema_version: 1` bare int, quoted `id:`, forbidden legacy own-id keys,
  provenance bundle on `code_state_anchored=yes` rows, per-type extras,
  typed-linkage slot shapes, status vocab). The TSV is the de-facto **per-type
  schema table** — it enumerates every artifact type, its extras, its
  typed-linkage keys, its `code_state_anchored` flag, and its forbidden own-id
  key.
- `scripts/test-skill-frontmatter-population.sh` validates SKILL.md *guidance*
  against `scripts/skills-schema.tsv` (does the skill instruct population/omission).
- ADR-0040 (§Consequences) explicitly states "No test inspects a *generated
  artifact*." So AC-1's "validates against the unified-schema validator" must
  either be implemented (a new corpus validator — likely awk/Python reusing
  `templates-schema.tsv` as the schema source) or rescoped. **This is the
  largest unstated implementation task in the story.**

## Code References

- `skills/config/migrate/scripts/run-migrations.sh:161-164` — discovery glob + `sort -z`
- `skills/config/migrate/scripts/run-migrations.sh:39-40` — applied/skip ledger paths
- `skills/config/migrate/scripts/run-migrations.sh:279-291` — `no_op_pending` sentinel
- `skills/config/migrate/scripts/run-migrations.sh:67-141` — clean-tree pre-flight
- `skills/config/migrate/migrations/0006-canonicalise-work-item-id-and-author.sh:80-242` — awk state machine
- `skills/config/migrate/migrations/0006-…sh:270-280` — `cmp -s` + `atomic_write` gate
- `skills/config/migrate/migrations/0006-…sh:89-109` — `normalise_value`/`semantic_inner`/`refuses`
- `skills/config/migrate/migrations/0006-…sh:227-239` — DIVERGE/MALFORMED checks
- `scripts/atomic-common.sh:16-32` — `atomic_write`; `:177-210` — `atomic_jsonl_append`
- `scripts/interactive-protocol.sh:14-46` — frame catalogue + state machine
- `scripts/interactive-harness.sh:114-176,274-439` — emit/predicate/apply/resume hooks
- `skills/config/migrate/scripts/interactive-lib.sh:289-588` — `run_interactive_migration`
- `skills/config/migrate/scripts/interactive-lib.sh:151-187` — `write_session_record`
- `skills/config/migrate/scripts/test-fixtures/interactive/doc-example/migrations/0099-doc-example.sh` — worked example
- `skills/visualisation/visualise/server/src/frontmatter.rs:305-368` — `read_ref_keys`; `:334-341` fallback; `:469-477` test
- `skills/visualisation/visualise/server/src/cluster_key.rs:119-131` — `parent_or_legacy_id`; `:75` call site
- `skills/visualisation/visualise/server/src/indexer.rs:1212-1233` — filename id fallback
- `templates/note.md:1-19` — `note` baseline shape
- `templates/work-item-review.md:13` — transitional `work_item_id:` alias to remove
- `scripts/templates-schema.tsv` — de-facto per-type schema table
- `scripts/test-template-frontmatter.sh:32-252` — template frontmatter contract test
- `meta/notes/` — 14 files (13 bare-H1, 1 partial); `2026-04-17-security-lens-owasp-ai-top-10.md:1-8`

## Architecture Insights

- **Idempotency is layered**: ledger (won't re-run an applied ID) + per-file
  `cmp -s` gate (won't rewrite unchanged bytes) + interactive resume (won't
  re-prompt recorded `(artifact_path, source_anchor)` keys). A 0070 re-run is a
  no-op on all three planes — AC-15 is achievable by following the precedent.
- **The mechanical/inferential split maps cleanly onto the contract**: ADR-0037
  routes predicate-false transformations to the mechanical path. So 0070's awk
  rewrite (deterministic) and resolved-band linkages run mechanically
  (`MECHANICAL_APPLIED`), while only ambiguous-band linkages prompt. The session
  log is therefore **deliberately incomplete** — resolved-band inferences are not
  recorded; the migrated frontmatter is their only record (ADR-0038 §Neutral).
- **Atomic-everything**: every mutation is temp-then-rename; the session log uses
  a `.lockdir` mutex. VCS revert is the only rollback (ADR-0023; no inverse
  migration).
- **Cross-repo coupling drives the indivisible XL scope**: the visualiser
  fallbacks exist to tolerate un-migrated userspace repos, so their removal must
  land in the *same release that closes 0070*, after the combined rewrite leaves
  no legacy keys to fall back to. This is why the Rust removal cannot be a
  follow-on story.
- **Runtime migration ordering ≠ story blockers**: 0007 assumes 0005 (`kind:`)
  and 0006 (`work_item_id`/`author`) have already been *applied* against the
  corpus; its awk transforms assume `kind:` present and foreign `work_item_id:`
  already quoted. The ordered ledger replay enforces this.

## Historical Context

- `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`
  — the spike: 1,231 candidate linkages over 267/381 files; stratified sample of
  150 (50/band, seed=42); **84% correct, 11.3% wrong, 4.7% uncertain**; rubric
  was "deterministic+report if wrong ≤5% AND uncertain ≤15%, else interactive
  hooks". Verdict: **interactive hooks** (not moot). Recommends the three cheap
  fixes anyway to cut prompt volume.
- `meta/work/0057-…md` — parent epic (in-progress). Establishes the identity-key
  convention (own `id` vs foreign `<type>_id`), the typed linkage vocabulary, the
  provenance bundle, per-type `schema_version`. Defers `specs/`+`global/`; defers
  the notes decision to 0070; names 0070 as the corpus-migration integration item
  every sibling `Blocks`.
- Sibling stories (all `done`): **0063** shipped `0005` (`type:`→`kind:`) and
  removed that rewrite from 0070's scope; **0064** shipped `0006`
  (`work-item:`→`work_item_id:`, `researcher:`→`author:`) and the visualiser
  `work-item:` transitional fallback; **0065** unified the nine templates and
  flagged ADR-0033's own `adr_id:` as 0070's job; **0066** moved review/validation
  frontmatter into templates and surfaced the `pr:` prefix + `work_item_id:` alias
  follow-ups; **0067** created `create-note` + `note.md` and flagged the notes
  decision; **0069** built the interactive runner (ADR-0037), with 0070 its named
  first consumer via ADR-0038.
- ADRs (all accepted): **0023** (migration framework), **0033** (unified base
  schema + identity + provenance + `schema_version` per-type bump duty),
  **0034** (typed linkage vocabulary + full type-pair table; `pr:` **not** in
  vocabulary), **0037** (interactive contract primitives), **0038** (this
  migration's parameters: `band=='ambiguous'`, three fixes, JSONL path,
  `(artifact_path, source_anchor)` key, `"Source:"`→`parent` resolution),
  **0040** (omit-when-empty: base fields + `tags: []` + always-valued extras
  always present; typed-linkage/foreign-ref/optional-extra keys omitted when
  empty; "absent = no value" reader rule binds 0070).

## Related Research

- `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`
  (the driving spike — `relates_to`).

## Open Questions

1. **AC-1's "unified-schema validator" has no implementation.** Build a corpus
   validator (reusing `scripts/templates-schema.tsv` as the schema source) or
   rescope the AC to the existing producer-surface contract tests? This is the
   biggest unscoped task.
2. **Skipped-reference semantics vs AC-7.** A `skipped` ambiguous linkage reaches
   `APPLIED_CONFIRM` with no mutation. Does AC-7's "every reference reaches
   `APPLIED_CONFIRM`" accept skips, or must each ambiguous reference resolve to an
   applied link? The contract permits skip as a legitimate terminal.
3. **Band-classification fixture set (AC-6/AC-8).** Where do the
   ≥150-resolved-band stratified sample and the known-ambiguous fixture set live?
   No such fixtures exist yet — they must be authored alongside the parser.
4. **The lone partial note** (`2026-04-17-security-lens-owasp-ai-top-10.md`) has
   `status: draft`, not the canonical `captured`. Preserve, or normalise?
5. **Corpus drift since the spike.** The spike measured 381 files / 1,231
   linkages; the live corpus is ~486 files with header counts higher across the
   board (`## References` 240 vs 207). The dogfood wrong-rate sample (AC-6) must
   be re-drawn against the current corpus, not the spike's.
6. **The 32 no-fence files.** 14 are notes (handled); the rest are
   design-inventory captures and prose-only artifacts. Does the migration backfill
   all of them, or only `meta/notes/`? The story scopes notes explicitly but the
   other ~18 no-fence files need a decision (likely skip — they are not standard
   producer artifacts).
