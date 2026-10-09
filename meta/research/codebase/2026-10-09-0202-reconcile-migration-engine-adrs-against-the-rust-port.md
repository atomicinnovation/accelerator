---
type: "codebase-research"
id: "2026-10-09-0202-reconcile-migration-engine-adrs-against-the-rust-port"
title: "Superseding the migration-engine ADRs (0023, 0037, 0038) against the Rust port"
date: "2026-10-09T08:42:53+00:00"
author: "Toby Clemson"
producer: "research-codebase"
status: "complete"
work_item_id: "0202"
parent: "work-item:0202"
topic: "Superseding the migration-engine ADRs (0023, 0037, 0038) against the Rust port"
tags: ["research", "codebase", "migrate", "migration-engine", "adr", "interactive-contract", "m0007"]
revision: "e76a8a9bb860cad08c208932ae74c917a8ca88cf"
repository: "accelerator"
last_updated: "2026-10-09T08:55:04+00:00"
last_updated_by: "Toby Clemson"
last_updated_note: "Added follow-up research on whether emit_transformations can be made side-effect-free"
schema_version: 1
---

# Superseding the migration-engine ADRs (0023, 0037, 0038) against the Rust port

**Date**: 2026-10-09T08:42:53+00:00
**Author**: Toby Clemson
**Git Commit**: e76a8a9bb860cad08c208932ae74c917a8ca88cf
**Branch**: detached (jj change `mpzsnvwyopyr`)
**Repository**: accelerator

## Research Question

What does work-item:0202 need, from the live code and the three
predecessor ADRs, to draft the ADR-0023, ADR-0037 and ADR-0038 successors?
Do the gate checks pass, and which requirements does the shipped system
not satisfy?

## Summary

The gate passes for all three ADRs: each one's triggering text names
deleted machinery or a stale path. All three Confirmed cells can be set to
`yes`. The successors will be ADR-0070, ADR-0071 and ADR-0072 in drafting
order. `accelerator corpus adr next-number` takes max+1 and the current
highest is ADR-0069.

The research also found six places where 0202's requirements disagree with
the shipped code or the tooling. Each needs a call from Toby before
drafting:

1. ❌ **Repeatable-callback purity**: 0202 asks the ADR-0037 successor to
   state that `emit_transformations` is side-effect-free. m0007 runs its
   whole mechanical rewrite inside `emit_transformations`. The engine
   documents this (`cli/migrate/src/engine.rs:112-115`), and `--list` and
   the decisions-file dry-apply both call it.
2. ❌ **ADR-0038 features that did not ship**: these are lost behaviour,
   not just renames.
   - Display extras (prose line, heading, alternative keys, on-disk
     resolution, the v1 key menu on edit).
   - Path-form edit targets.
   - Accept-degraded.
   - Type-pair checking of edits.
3. ⚠️ **Predecessor body `**Status**:` line**: `create-adr` step 4.4
   rewrites it (`skills/decisions/create-adr/SKILL.md:263-267`). The
   ADR-0057 precedent shows the change. 0202's AC allows only frontmatter
   `status`, `superseded_by` and timestamp changes.
4. ⚠️ **work-item:0136's only citation is about the superseded design**:
   L98 reads "ADR-0038's shape drifted". Under 0202's own rule it should
   become `ADR-0038 (superseded by ADR-0072)`. 0202 says 0136 is
   "replaced rather than annotated".
5. ⚠️ **Collision guard reversed**: ADR-0023 aborts when both directories
   exist. m0001 now merges them and the source file wins.
6. 🟡 **Skip has no ADR origin**: ADR-0037 §4 credits `--skip` to
   ADR-0023, but ADR-0023 contains no skip mechanism. The ADR-0023
   successor records it for the first time; it does not restate it.

## Gate check: Dispositions

| ADR | Triggering text (line) | Live reality | Confirmed |
|---|---|---|---|
| ADR-0023 | `meta/.migrations-applied` L69/98/114/160; `run-migrations.sh` L118; `hooks/migrate-discoverability.sh` L139/194; glob L106 | ledger at `.accelerator/state/migrations-applied` (`cli/migrate-adapters/src/ledger_store.rs:34-36`); compiled `registry()` (`cli/migrate/src/registry.rs:71-84`); hook is `accelerator migrate --discoverability-hook` (`hooks/hooks.json:22-30`) | yes |
| ADR-0037 | `skills/config/migrate/scripts/run-migrations.sh` L40, L145 | runner is `cli/migrate/src/engine.rs::run_interactive`; script deleted at 0172 Phase 10 | yes |
| ADR-0038 | L122 "`NNNN-<slug>` form of its file in `skills/config/migrate/migrations/` … migration script" | id is `MigrationMeta::id()` = `"0007-unify-meta-corpus-frontmatter"` (`cli/migrate/src/migrations/m0007/mod.rs:48`) | yes |

ADR-0038 L126 is also stale, though the Dispositions table does not list
it. It says the migration reads the log itself and resumes on
`(artifact_path, source_anchor)`. The engine now matches on
`transformation_key` (`engine.rs:39-42`).

## Detailed Findings

### Base framework (ADR-0023 successor)

**Registry.**
- `MigrationMeta { id, description }` is at `registry.rs:22-25`.
- `Migration::apply` returns `ApplyOutcome { Applied, NoOpPending }` (`registry.rs:27-39`).
- `MigrationEntry { Mechanical, Interactive }` is at `registry.rs:46-49`. m0007 is the only interactive migration.
- Migrations run in the order of the `vec!` literal. Nothing checks for sortedness or duplicate IDs.
- IDs are full slugs, for example `"0001-rename-tickets-to-work"`.

**Ledgers.**
- **Applied.** `.accelerator/state/migrations-applied` holds newline-delimited IDs, trimmed and blank-skipped on read (`ledger_store.rs:51-68`).
  - After each success the engine re-reads the file, appends with `append_unique` and writes it back atomically (`cli/migrate/src/lifecycle.rs:111-113`). The re-read exists because m0003 writes to this file itself.
  - Unknown IDs are preserved and produce a warning (`ledger.rs:19-45`, `render.rs:151-156`; test `tests/lifecycle.rs:280-309`).
- **Skipped.** `.accelerator/state/migrations-skipped`, written by `--skip`, `--unskip` and `--unapply` (`cli/migrate-cli/src/cli.rs:19-28`, `main.rs:164-181`).
  - `ledger::pending` excludes IDs that are applied or skipped and keeps registry order, so a skipped migration blocks nothing (`ledger.rs:47-67`).
  - When an ID is in both ledgers, applied wins and the engine warns (`render.rs:165-170`; tests `tests/ledger.rs:60,101`, `tests/lifecycle.rs:337-368`).
  - Force never touches the ledgers (`preflight.rs:79-82`). Test `force_bypasses_only_the_dirty_check_not_a_skipped_migration` (`cli/migrate-cli/tests/dirty_tree_preflight.rs:250-293`) covers this; it is gated behind `cfg(feature = "bash-parity")`.

**Pre-flight** (`cli/migrate/src/preflight.rs:72-142`).

```text
lock ─► observe dirty paths in SCOPES ─► force? ──yes──► fresh run (Clean)
                                          │no
                    drop runner-managed ──┤
                          none left? ─yes─► fresh run (Clean)
                                          │no
     run base matches recorded? ──no──► every path foreign ─► refuse (UnownedChanges)
                                          │yes
     every dirty path in manifest/session artefact? ──yes──► Resumed
                                                     └─no──► refuse
```

- **Scopes.** `["meta/", ".claude/accelerator", ".accelerator/"]` (`preflight.rs:17`).
- **Path manifest.** `.accelerator/state/migrations-run-paths.txt`, appended by `MigrationContext::write` (`cli/migrate-adapters/src/context.rs:111-143`). `merge_move` is not manifest-tracked.
- **Run base.** Stored in `.accelerator/state/migrations-run.id` (`run_base.rs:15-22`). Both files are cleared on success (`main.rs:328-330`).
- ⚠️ **Force value.** `ACCELERATOR_MIGRATE_FORCE` accepts any non-empty value, not only `1` (`main.rs:100-103`). The successor can still state `=1`, which is true.

**Driver.**
- **Preview.** Two lines per migration (description, then a `To skip:` hint) plus a preamble (`render.rs:131-138`). ADR-0023 describes one line.
- **Abort.** The run stops at the first failure. The ledger keeps earlier successes and gets nothing for the failed migration (`tests/lifecycle.rs:407-441`). ADR-0023's "no partial state write" holds per migration.
- **Summary.** One line, `Migration complete. applied: N[; skipped: …][; pending (no-op): M].` (`render.rs:252-272`), where ADR-0023 describes a table.
- **Dry-run.** There is no `--dry-run` (`cli.rs:18-52`).

**Pinned paths.** These are handled per migration through `configured_path_override` (`context.rs:156-161`):
- m0001 only moves default-path directories, and keeps pinned values when it renames keys (`m0001.rs:53-90,132-180`; test `migration_0001.rs:255-295`).
- m0003, m0004 and m0009 do the same.

**Collision.** m0001 merges with source-wins (`merge_move.rs:43-73`; test `migration_0001.rs:151-196`). It no longer aborts. m0002 and m0004 have their own refusals (`m0002.rs:112-137`, `m0004.rs:531`).

**Discoverability** (`cli/migrate-cli/src/discoverability.rs`).
- **Lag rule.** Lexicographic max applied < max registry ID (`:48-69`). An unknown high ID in the ledger would silence the advisory; this is untested.
- **Repo guard.** The advisory fires only if `.accelerator/`, `.claude/accelerator.md` or `meta/` exists (`:9-13`).
- **Ledger fallback.** It falls back to the legacy `meta/.migrations-applied` (`:20-27`).
- **Output.** A JSON `systemMessage`, not stderr.

**First migration.** `Migration0001` (`m0001.rs:20-102`) refuses malformed config, rewrites `ticket_id:` → `work_item_id:`, merge-moves the default directories, and renames five config keys in both nested and flat forms.

### Interactive contract (ADR-0037 successor)

The contract primitives map to code as follows.

| Primitive | Rust mapping | Location |
|---|---|---|
| Opt-in | `MigrationEntry::Interactive(Box<dyn InteractiveMigration>)` | `registry.rs:46-49` |
| Transformation enumeration | `InteractiveMigration::emit_transformations` → `Vec<Transformation>` | `interactive.rs:48-51`, `7-16` |
| Trigger predicate | `evaluate_predicate` → `PredicateOutcome { Prompt, Mechanical, Fail }` | `interactive.rs:34-38,53-56` |
| Display elements | `Transformation { proposed, path, anchor, predicate_value, display }`, rendered by `TtyDecisionSource::render_prompt` | `tty_decision_source.rs:42-73` |
| Controls | `Decision { Accept, Edit(String), Skip }` via `DecisionSource::next_decision` | `interactive.rs:40-45`, `ports.rs:482-491` |
| Edit validity | `validate_edit` → `Err` prints `[interactive] …` and re-prompts | `interactive.rs:58-64`, `engine.rs:224-240` |
| Application | `apply_decision`, never for `Skip` | `interactive.rs:66-76`, `engine.rs:204-206` |
| Resumability artefact | engine-owned `SessionLog` port; `corpus::Record` | `ports.rs:436-467`, `corpus/src/record.rs:22-34` |
| Resume check | `verify_applied` (default `true`) | `interactive.rs:78-87`, `engine.rs:44-52` |
| Post-apply | `finalise` (default no-op), once after the loop | `interactive.rs:89-102`, `engine.rs:92-95` |

**Write-ahead log.** `record_then_apply` (`engine.rs:177-208`) appends the
record first, returns early on `Skip`, and only then calls `apply`. The
append goes through `FileCorpusStore::append_record`
(`corpus-adapters/src/store.rs:154-179`), which takes a lock and rewrites
the file with `atomic_write`. That fsyncs the temp file and the parent
directory (`store/src/lib.rs:246-284`), so "durably persisted" holds.

**Record shape.** Fields are written in a fixed order:
`transformation_key`, `schema_version` (=1), `outcome`, `proposed_value`,
`user_value`, `timestamp` (`corpus-adapters/src/jsonl.rs:97-130`).
- `user_value` is written only when present.
- An `edited` outcome must carry `user_value`, and `accepted` and `skipped` must not (`jsonl.rs:71-91`).
- The adapter sets the timestamp (`session_log_factory.rs:65-84`).
- Records with an unknown `schema_version` are refused.
- Mechanical-path transformations are not logged (`engine.rs:62-71`).

**Resume.** A decided record is replayed only when its `proposed_value`
matches. Beyond that:
- a `Skipped` record is then decided without further checks (sticky skip);
- an `Accepted` or `Edited` record must also pass `verify_applied`.

A drifted or unverified record is removed from the log and the
transformation is prompted again (`engine.rs:44-59`).

**Timeout and stall.**
- **Decision timeout.** `DECISION_TIMEOUT = 30s` (`migrate-cli/src/main.rs:54`). `Timeout` and `Eof` both fail the run with `timed out waiting for a decision` (`engine.rs:262-267`).
- **Stall.** `NoInputDecisionSource` never arms the timeout (`ports.rs:493-512`). The stall reads `MIGRATION STALLED: no decision input available` (`engine.rs:252-261`, `render.rs:200-246`).
- **Unrecognised TTY input.** Any other line becomes `Edit(line)` (`tty_decision_source.rs:117-128`). This is an implementation detail, not part of the contract.

**Determinism.**
- **Source code.** Determinism is not stated anywhere in the code: no doc comment requires `emit_transformations`, `evaluate_predicate`, `validate_edit` or `verify_applied` to be deterministic or side-effect-free.
- **SKILL.md.** `skills/config/migrate/SKILL.md:143` states only that callbacks repeat and that `validate_edit` must be pure.
- **Repeat count per run.**
  - Without a decisions file, `emit_transformations` runs once per run.
  - With `--decisions-file`, it runs twice in one process: once for the dry-apply (`lifecycle.rs:77-91` → `engine::pending_transformations`) and again in `run_interactive`. The second run sees the corpus the first one mutated.
  - `verify_applied` is called from `engine.rs:49` and `engine.rs:141`.

**Band ownership.** `Transformation` has no band field. Band exists only as
m0007's `predicate_value` and an unread `extras` entry, so band design is
migration-owned in the code.

**Invocation surfaces.** `--list` (`list.rs:55`), `--decisions-file` /
`ACCELERATOR_MIGRATE_DECISIONS_FILE` (`main.rs:113-120`), and source
selection (`main.rs:304-317`) all live in `migrate-cli`. None are in the
`InteractiveMigration` trait.

### Migration 0007 parameters (ADR-0038 successor)

- **Identity.** `MigrationMeta::id()` = `"0007-unify-meta-corpus-frontmatter"` (`m0007/mod.rs:46-56`).
- **Two bands.** `corpus::linkage::Band { Resolved, Ambiguous }` (`corpus/src/linkage.rs:22-35`), assigned by `classify_band` (`:374-395`). The predicate returns `Prompt` when it is `"ambiguous"`, otherwise `Mechanical` (`mod.rs:276-288`).
- **Resolved targets.** A resolved target that is missing from the corpus index is dropped with `0007-DIVERGE[reverse-orphan]` (`mod.rs:218-237`).
- **Field filling.**
  - `transformation_key` = `{relpath}#body:{section-slug}#{seq}` (`mod.rs:240`, `linkage.rs:620`).
  - `proposed_value` = `{key}={target_ref}` (`mod.rs:243`).
  - `user_value` = the edited `{key}={doc-type:id}`.
- **Edit validation.** `validate_edit` (`mod.rs:290-307`) checks:
  - the value contains `=`;
  - the key is in the nine-key vocabulary (`schema.rs:34-43`);
  - the target passes `shape::is_well_formed`.
- **`finalise`.** Runs the whole-corpus `validate_frontmatter(&[])` (`mod.rs:332-335`).
- **`verify_applied`.** Not overridden. Its signature has no `ctx`, so it cannot re-read the file.

ADR-0038 L124 lists eight session-log fields. Each one now lives in the
Rust record as follows.

| ADR-0038 field | Class | Rust destination | Evidence |
|---|---|---|---|
| `artifact_path` | folded | `transformation_key`, before `#` | `mod.rs:240` |
| `source_anchor` | folded | `transformation_key`, after `#` (`body:<slug>#<seq>`) | `mod.rs:240`, `linkage.rs:620` |
| `inferred_key` | folded | `proposed_value`, left of `=` | `mod.rs:243` |
| `inferred_target` | folded | `proposed_value`, right of `=` | `mod.rs:243` |
| `band` | dropped | only `ambiguous` decisions are logged, so it is implied; the `extras` entry is discarded | `engine.rs:62-87`, `session_log_factory.rs:79` |
| `decision` | renamed | `outcome` (same three values) | `record.rs:5-20` |
| `edited_key` | folded | `user_value`, left of `=` | `engine.rs:195`, `mod.rs:295` |
| `edited_target` | folded | `user_value`, right of `=` | same |

The ADR-0038 Decision subsections fall into three groups against the live
code.

- **Holds:**
  - two-band design;
  - hybrid application;
  - predicate `band == 'ambiguous'`;
  - all three parser fixes: `is_template_path` (`linkage.rs:84-91`), the `contains_word` boundary set (`:93-136`), and `has_sibling_keyword` → `relates_to` (`:148-152, 343-345`);
  - `Source:` → `parent` for a work-item target (`linkage.rs:355-362`);
  - broader-workstream left untouched, since `extract_tokens` produces no record (`linkage.rs:519-527`).
- **Changed:**
  - `Source:` mapping is keyed on the target type and ignores the source document's type. For a plan source it resolves mechanically; for other source types it is ambiguous, so it is prompted.
  - The predicate field set is now just `band`, carried in `predicate_value`.
- ❌ **Not shipped:**
  - all four extra display elements and the v1 key menu (m0007's `display` repeats the mandatory slots, `mod.rs:245-248`);
  - path-form edit targets;
  - accept-degraded;
  - ADR-0033/0034 type-pair validation of edits;
  - validation of accepted raw targets (only `finalise` catches them).

### Citation sweep

Two locations are in scope:

- **`skills/config/migrate/SKILL.md`.** L68 ("matching ADR-0023's
  belt-and-suspenders requirement") and Cross-references L337-339. The
  section is titled "Optional interactive contract" (L103-164), not
  "Interactive contract".
- **work-item:0136.** Status `in-progress`. L98: "0202 — … *(ADR-0038's
  shape drifted when 0172 ported the engine to native Rust)*". This is
  about the superseded design (see Summary item 4).

These are out of scope:

- **Generated docs.** `docs-site/src/content/docs/reference/skills/config/migrate.md` is gitignored (`.gitignore:28`).
- **Done work items.** 0030, 0031, 0057, 0062, 0063, 0069, 0070, 0092, 0115, 0117, 0172 and 0195 are all `done`.
- **Test fixture.** `cli/visualiser/frontend/src/api/wiki-links.test.ts:88` contains the literal `[[ADR-0023]]`. It is a fixture, not a citation.

There are no hits in `agents/`, `templates/`, `hooks/` or the Rust doc
comments.

### Supersession tooling

- **One predecessor per call.** `create-adr --supersedes` takes a single ADR (`create-adr/SKILL.md:7, 63-66`).
- **Accepted only.** A non-accepted predecessor prompts rather than errors (`:67-74`).
- **Flip timing.** The predecessor is flipped at write time, while the successor is still `proposed` (`:193-200`). The skill sets `status: superseded`, adds `superseded_by`, and updates the body `**Status**:` line (`:263-267`).
- **No timestamp bump.** No `last_updated` change is made; ADR-0057 and ADR-0060 confirm this.
- **`superseded_by` format drifts.** The skill shows a bare `ADR-MMMM`, and recent precedents quote it (`"ADR-0062"`). Older ones use `"adr:ADR-0047"`.
- **Reject is terminal.** `review-adr` only offers Accept, Reject and Revise; Revise keeps the ADR `proposed` (`review-adr/SKILL.md:83-89, 170-207`).
- **Successor shape to copy.** ADR-0062: Context restates the predecessor and says what no longer holds, drivers are marked "carried forward from ADR-0057", the Decision ends with a carried-forward bullet, and References adds "superseded by this ADR".

## Proposed checklist dispositions

These are starting positions for the "Restated" / "Superseded design"
split. Toby confirms them during drafting.

| Predecessor item | Proposed home | Citation or reason |
|---|---|---|
| 0023 ordered registration | Restated | `registry()` literal order, `ledger::pending` |
| 0023 per-migration description | Restated | `MigrationMeta::description` |
| 0023 project-root access | Restated | `MigrationContext::root` |
| 0023 idempotency | Restated | SKILL.md "Authoring a migration"; `migration_0001.rs:298-348` |
| 0023 atomic writes | Restated | `store::atomic_write` via `MigrationContext::write` |
| 0023 newline ledger, unknown-ID preservation | Restated | `FileLedgerStore`, `ledger::warnings` |
| 0023 step 1 clean-tree + force | Restated, reshaped | `Preflight::run` (foreign dirt, guarded resume, force) |
| 0023 steps 2, 4 | Restated | `lifecycle::run_pending` |
| 0023 step 3 one-line preview | Superseded design | now two lines plus a preamble |
| 0023 step 5 summary table | Superseded design | now a single summary line |
| 0023 pinned-path preservation | Restated | `configured_path_override`; m0001 |
| 0023 collision guard (abort) | Superseded design | m0001 merges with source-wins; other migrations own their refusals |
| 0023 discoverability lag + guard | Restated | `discoverability.rs`; hook unnamed |
| 0023 first migration | Restated | `Migration0001` |
| 0023 skill-driven, newline ledger, no dry-run, VCS revert, SessionStart | Restated | "shell scripts" half of the skill-driven option goes to Superseded |
| 0037 §1–§4, option 5 | Restated | table above |
| 0037 §3 "persistence declared by migration" | Superseded design | engine owns `SessionLog` |
| 0037 §5 | Restated recursively | applies to the successor |
| 0038 display extras, path-form edits, accept-degraded | Superseded design or follow-up | not shipped; see Open Questions |

## Code References

- `cli/migrate/src/registry.rs:22-84` — `MigrationMeta`, `Migration`, `MigrationEntry`, `registry()`
- `cli/migrate/src/interactive.rs:7-103` — `Transformation`, `PredicateOutcome`, `Decision`, `InteractiveMigration`
- `cli/migrate/src/ports.rs:54-320, 436-512` — `MigrationContext`, `SessionLog`, `DecisionSource`, `NoInputDecisionSource`
- `cli/migrate/src/engine.rs:26-270` — `run_interactive`, `pending_transformations`, `record_then_apply`, `prompt_and_decide`
- `cli/migrate/src/lifecycle.rs:37-130` — driver
- `cli/migrate/src/preflight.rs:17, 72-142` — pre-flight and guarded resume
- `cli/migrate/src/manifest.rs:9-112` — ownership classification
- `cli/migrate/src/ledger.rs:19-103` — pending, warnings, skip/unskip
- `cli/migrate-adapters/src/ledger_store.rs:34-74` — ledger paths and atomic write
- `cli/migrate-adapters/src/tty_decision_source.rs:42-128` — display rendering and timeout read
- `cli/migrate-adapters/src/session_log.rs:7-12` — session log path
- `cli/migrate-cli/src/main.rs:50-54, 100-103, 151-181, 304-330` — constants, force, flags, source selection
- `cli/migrate-cli/src/discoverability.rs:9-69` — lag advisory
- `cli/corpus-adapters/src/jsonl.rs:71-130` — record serialisation and coupling rules
- `cli/migrate/src/migrations/m0007/mod.rs:46-370` — m0007
- `cli/corpus/src/linkage.rs:22-623` — bands, parser fixes, anchors
- `skills/config/migrate/SKILL.md:68, 103-164, 335-342` — citation sites and the contract section

## Architecture Insights

- **Engine owns the contract.** The engine, not the migration, now owns resumability, display rendering and the write-ahead ordering. A migration supplies only values (`key`, `proposed`, `predicate_value`). That is why the ADR-0038 record shape folds into three engine strings.
- **The mechanical/interactive split sits inside a migration.** m0007 is one `Interactive` entry whose `emit_transformations` also does all of its mechanical work. This is the root of the purity conflict. The ADR-0037 successor either states purity as an obligation that m0007 breaks, or narrows it to `evaluate_predicate`, `validate_edit` and `verify_applied` and states that `emit_transformations` must be idempotent.
- **Guarded resume ties three files together.** The path manifest, the run base and the session-artefact ownership pattern are one design, defined in `manifest.rs` and `preflight.rs`.

## Historical Context

- `meta/plans/2026-08-07-0172-migration-engine-subdomain.md`:
  - L120-153 and L1397-1429: the 30 s timeout is deliberately new; the bash watchdog only bounded teardown.
  - L2274-2290: `verify_applied` without `ctx` is a "disclosed narrowing". The plan names no `finalise`; it was added afterwards to replace a `RefCell` workaround.
  - L209-216 and L2712-2718: scopes ADR reconciliation out to 0202.
- `meta/work/0214-*.md`, `meta/work/0157-*.md` are the supersession precedents. 0157:84-88 wrongly says `review-adr` applies the supersede edge.
- ADR-0054 is still `accepted` despite ADR-0069 superseding it. ADR-0026 is `accepted` with `superseded_by` set. Both are signs that supersession flips have been missed before.

## Related Research

- `meta/research/codebase/2026-05-26-0092-adr-optional-interactive-contract-for-migration-framework.md`
- `meta/research/codebase/2026-05-26-0062-adr-interactive-validation-for-corpus-migration.md`
- `meta/research/codebase/2026-05-24-0068-related-documents-inference-accuracy.md`

## Open Questions

- ❓ **Purity.** Should the ADR-0037 successor require side-effect-free
  `emit_transformations`, given that m0007 violates it? The alternative
  narrows the rule to idempotency. Either choice changes 0202's AC wording.
- ❓ **ADR-0038 features that did not ship.** Should the display extras,
  path-form edits, accept-degraded and type-pair edit validation go into
  "Superseded design" with a reason, or each get a follow-up item to
  restore them? 0202 says "records the behaviour that shipped", which
  supports the first option.
- ❓ **Body status line.** Does the AC on predecessor diffs need to allow
  the body `**Status**:` line that `create-adr` rewrites? The alternative
  is to suppress that step.
- ❓ **0136 L98.** Annotate it as `ADR-0038 (superseded by ADR-0072)`,
  following the superseded-design rule, or replace it as 0202 currently
  says?
- ❓ **`superseded_by` format.** Use bare `"ADR-0070"`, matching recent
  precedent and the skill, or typed `"adr:ADR-0070"`?
- Not checked: `cli/migrate-cli/tests/discoverability_hook.rs`, and whether
  the bash-parity-gated force test runs in CI.

## Follow-up Research 2026-10-09T08:55:04+00:00

### Question

Can `emit_transformations` be made side-effect-free (option (a)), and does
m0007 need to mutate inside it?

### Answer

Yes, and no: the workaround isn't needed. It already causes a reproduced bug.

### The workaround's cost: `--list` dirties the tree and blocks the next run

I reproduced this in a scratch jj repo with the binary built at revision
`e76a8a9b`:

1. Seed migrations 0001–0006 as applied.
2. Commit two pre-unification work items.
3. Run `accelerator-migrate --list`.
4. Run `accelerator-migrate`.

```text
== status after --list
.accelerator/state/migrations-run-paths.txt | 2 ++
meta/work/0001-source.md                    | 9 ++++++++-
meta/work/0042-target.md                    | 9 ++++++++-
== real run
Error: dirty working tree — uncommitted changes detected in meta/, ...
Unowned changes (2):
  meta/work/0001-source.md
  meta/work/0042-target.md
```

The cause is a chain of four steps:

1. `run_list` (`cli/migrate-cli/src/main.rs:195-220`) takes no lock and runs no pre-flight.
2. m0007's `emit_transformations` then writes through `ctx.write`, which appends to the path manifest.
3. No run base is recorded, so the next run's `unowned_changes` (`cli/migrate/src/preflight.rs:119-142`) treats every written path as foreign dirt.
4. The real run refuses.

The `--list` tests (`cli/migrate-cli/tests/list_and_decisions_file.rs`)
miss this because they seed corpora whose frontmatter is already unified,
so the rewrite writes nothing.

### Why the mechanical work can be computed in memory

| Step | Mutates? | Evidence |
|---|---|---|
| Pre-pass | no (pure over `(relpath, content)`) | `m0007/prepass.rs:25` |
| Backfill | no (returns new content) | `m0007/mod.rs:146-157`, `backfill.rs:56` |
| Rewrite | no (returns new content; idempotent) | `rewrite.rs:166`; test at `rewrite.rs:842-844` |
| Only mutation | `ctx.write` after each pass | `m0007/mod.rs:155, 180` |
| Corpus index | path-derived, independent of the rewrite | `cli/migrate-adapters/src/corpus_index.rs:27-43` |
| Linkage type and parse | read `content`, not disk | `rewrite.rs:116-130`, `mod.rs:206-216` |
| Structural validation | reads disk by path | `cli/migrate-adapters/src/context.rs:240-255` |

The table leaves one blocker: structural validation reads files from disk.

### Shape of a side-effect-free m0007

```text
emit_transformations (pure)
  prepass ─fail─► sentinel Fail
  backfill+rewrite in memory ─► validate in-memory content ─fail─► sentinel Fail
  emit, in order:
    one Mechanical transformation per changed file   (apply_decision writes it)
    one transformation per linkage record            (Prompt or Mechanical by band)
```

The design rests on five points:

- **The engine already supports it.** `PredicateOutcome::Mechanical` routes a transformation straight to `apply_decision`, without prompting or logging (`engine.rs:62-71`).
- **Emission order is canonical,** so file rewrites land on disk before any linkage `apply_linkage` reads them.
- **The sentinel `Fail` still works** for pre-pass and validation refusals, because it doesn't depend on a mutation.
- **A content-taking validation port is needed.** Structural validation has to accept in-memory content (a variant of `MigrationContext::validate_frontmatter`) to keep the fail-before-any-prompt guarantee. Moving the check into `finalise` would avoid the port change, but the user could then answer every prompt before a structural failure.
- **Resume stays correct.** A re-run recomputes from disk, the rewrite is idempotent, and files already rewritten emit no Mechanical transformation.

A new `prepare` hook would not help. `--list` would either have to run it,
which mutates again, or skip it and enumerate the linkage records against
the pre-rewrite content.

### Consequence for 0202

Option (a) is achievable with a code change to m0007 and one port. 0202 is
documents-only, so that change belongs in a separate work item. That item
would fix the `--list` bug, and with it the doubled emit in a
`--decisions-file` run. The ADR-0037 successor can then state the purity
rule without m0007 breaking it. Not checked: whether bash's 0007 also
mutated during emission (`meta/plans/2026-05-30-0069-migration-framework-interactive-validation-hooks.md:104`).
