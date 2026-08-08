---
type: inventory
id: "0172-suite-audit"
title: "0172 Migrate-Suite Assertion Inventory"
date: "2026-08-08T00:00:00+00:00"
author: Toby Clemson
producer: implement-plan
status: complete
parent: "work-item:0172"
relates_to:
  - "plan:2026-08-07-0172-migration-engine-subdomain"
tags: [rust, migrate, cli, migration, suite-audit]
last_updated: "2026-08-08T00:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0172: Migrate-Suite Assertion Inventory

Pinned revision: **`34d15280adaf`**. `tasks/lint/migrate_suite_inventory.py`
(`inventory()`) discovers, at this revision, **1,010** `assert_*` call sites
across the six retiring suites named in the plan's Technical Notes. No
duplicate site and no raw/inventoried count mismatch
(`tests/unit/tasks/test_migrate_suite_inventory.py::test_the_real_suites_have_no_duplicate_or_gap`).

## Threshold decision

**1,010 > 400.** Per the plan's Phase 9 point 2, the exhaustive
repointable/not-repointable mapping narrows to the three suites named in the
AC — `test-migrate-0007.sh`, `test-migrate-interactive.sh`,
`scripts/test-interactive-protocol.sh` — with 0167's remainder-only pattern
for the rest (`test-migrate.sh`, `test-migrate-snapshot.sh`,
`hooks/test-migrate-discoverability.sh`: repointed at the compiled binary in
place, per Phase 9 point 4, without an exhaustive per-assertion table).

## Per-suite counts

| Suite | Total | Repointable | Not-repointable |
|---|---|---|---|
| `skills/config/migrate/scripts/test-migrate.sh` | 522 | 522 | 0 |
| `skills/config/migrate/scripts/test-migrate-snapshot.sh` | 2 | 2 | 0 |
| `skills/config/migrate/scripts/test-migrate-interactive.sh` | 265 | 265 | 0 |
| `skills/config/migrate/scripts/test-migrate-0007.sh` | 195 | 195 | 0 |
| `scripts/test-interactive-protocol.sh` | 12 | 0 | 12 |
| `hooks/test-migrate-discoverability.sh` | 14 | 14 | 0 |
| **Total** | **1,010** | **998** | **12** |

## Classification method, and its known limitation

`migrate_suite_inventory.py` classifies each assertion by scanning its own
logical statement (backslash-continuation lines joined) for a marker naming
a wire-protocol internal (`$FRAME_TYPE`, `harness_*`, `mkfifo`, ...);
`scripts/test-interactive-protocol.sh` is forced `not-repointable`
categorically, since it drives `interactive-protocol.sh` in-process and has
no CLI surface at all.

**This finds zero not-repointable assertions in `test-migrate-interactive.sh`,
which is a real limitation of the heuristic, not evidence the suite is
trivially repointable.** Reading the suite directly: it is not organised
into `test_*` functions but into sequential `echo "=== Phase N: ... ==="`
sections, each of which typically heredocs a **synthetic bash migration
script** to disk (using `harness_emit_transformation`/`migration_apply_decision`/
`harness_reject` — the author-facing callback contract, not a real numbered
migration) and then drives it through the real `run-migrations.sh`, asserting
on the captured stdout/stderr/session-log file. The *assertions themselves*
mostly reference only captured-output variables (hence "repointable" by this
tool's own literal text-marker rule), but the *test as a whole* has no Rust
equivalent to repoint at: the compiled `accelerator-migrate` registry is
compiled-in (Phase 2), so there is no way to inject a synthetic migration
into it the way bash injects a heredoc'd script. Every one of these tests'
real Rust equivalent already exists as a `FixtureMigration`-driven test in
`cli/migrate/tests/engine.rs`/`list_and_decisions_file.rs` (the established
pattern since Phase 5) — not as a black-box rewrite of the bash assertion
itself.

**Practical implication for Phase 9 point 3 (black-box rewrites):**
`test-migrate-interactive.sh`'s ~265 assertions do not decompose into a
short "not-repointable" list this tool can hand off as a black-box-rewrite
work list. Confirming its behaviour is already covered requires reading each
of its ~13 phase sections against the equivalent `FixtureMigration`
coverage already landed in Phase 5, not running this extractor again with a
smarter marker list — a materially larger, judgement-driven task than the
mechanical classification this tool performs for
`scripts/test-interactive-protocol.sh` (whose 12 sites this tool's
file-level rule classifies correctly and completely). This gap is
disclosed here rather than papered over with a wider marker list that would
only produce a false sense of completeness.

## Coverage audit (2026-08-08, this session)

Five agent-driven read-throughs — one per suite except `test-migrate-snapshot.sh`
(resolved by direct inspection) — cross-referencing every bash test scenario
against the Rust tests already landed (Phases 1-8) or built this session.
**`scripts/test-interactive-protocol.sh` is fully resolved** (see below); the
other four each carry a substantial genuine-gap list, recorded in full here
as the actual work-list deliverable of this audit — not summarised away,
since a future session (or a different contributor) needs to act on this
without re-deriving it.

### `test-migrate-snapshot.sh` — no equivalent needed

A byte-identical snapshot guard against **bash's own** mechanical-path
output (protects bash from silent regression). Superseded outright by
Phase 6's per-migration fixture-golden tests
(`cli/migrate-cli/tests/migration_000{1..6}.rs`), which serve the identical
protective role for the Rust side. Nothing to port.

### `scripts/test-interactive-protocol.sh` — resolved, one real gap found and fixed

All 16 assertions test the FIFO/TSV wire-protocol mechanism itself
(`escape_field`/`unescape_field`/`emit_frame`/`read_frame`) — moot once
everything is an in-process trait call with no wire at all; none needs a
literal port.

**However**, one of the invariants the wire protocol enforced relocates
rather than disappears: bash's `LIST_ENTRY` frame handler
(`interactive-lib.sh:552-566`) explicitly refused to emit a `--list` row
whose `key`/`path`/`anchor`/`proposed` carried an embedded tab or newline
("`--list` output is undefined for such values"), since it joins fields
with tabs. `cli/migrate-cli/src/render.rs`'s `render_list` builds the exact
same shape of tab-delimited line and had **no equivalent guard** — a real,
confirmed safety gap, not a theoretical one. **Fixed this session**:
`migrate::list::list_pending` now refuses (matching bash's message
verbatim, `"[{id}] --list field for key '{key}' contains a tab or newline;
--list output is undefined for such values."`) before ever building a
`ListGroup`, tested in `cli/migrate/tests/list_and_decisions_file.rs`
(`list_pending_refuses_a_proposed_value_carrying_an_embedded_tab`,
`..._a_key_carrying_an_embedded_newline`).

### Confirmed real bugs / design questions found during the audit (not test gaps — need a decision, not just a test)

These surfaced from reading the suites and the Rust source side by side, not
from counting assertions. Ranked by how directly they change observable
behaviour:

1. ✅ **RESOLVED, this session.** `Decision::Skip` reached `apply_decision`
   unconditionally — bash's harness never called `migration_apply_decision`
   for a skip. Fixed: `record_then_apply` now stops after recording a skip,
   before ever calling `apply_decision`; a spy test
   (`a_skip_never_reaches_apply_decision`, `cli/migrate/tests/engine.rs`)
   pins it. Fixing this surfaced a real interaction: migration 0007's own
   whole-corpus `self_validate_referential` trigger relied on
   `apply_decision` being called for every transformation, including skips,
   to detect "this was the last one." Rather than patch around it, added
   `InteractiveMigration::finalise` — called once by the engine after the
   whole interactive loop completes, mirroring bash's actual placement
   (once, after `harness_run`, not per-transformation) — and rewired 0007
   onto it, deleting its old `RefCell`-based last-transformation tracking
   entirely. A more faithful port, not just a fix.
2. ✅ **RESOLVED, this session.** `schema_version` validation on
   session-log resume was unimplemented, despite being named explicitly in
   this plan's own "must reproduce byte-for-byte" inventory. Fixed:
   `FileSessionLog::records()` (`cli/migrate-adapters/src/session_log_factory.rs`)
   now checks the parsed value against the one supported version and
   refuses with bash's own `"[resume] unknown schema_version N — supported:
   {1}."` message plus the `rm <path>` discard hint, naming the real
   session log path. Unit-tested plus an end-to-end black-box test seeding
   `schema_version: 99` against the compiled binary
   (`cli/migrate-cli/tests/migration_0007.rs`).
3. ✅ **RESOLVED, this session — with a correction.** The session-log
   banner was genuinely missing (bash prints "Session log: `<path>` (resume
   from this file by re-running /accelerator:migrate)" once, before the
   first prompt) — now implemented in `TtyDecisionSource`, guarded so it
   prints exactly once per run, tested via an injectable-writer
   `render_prompt`. **The "author-declared `extras` display line" this
   audit item also named turned out not to exist as a separate bash
   feature** — rereading `interactive-lib.sh:189-224` directly shows bash's
   own `render_prompt` never renders its `extras_tsv` parameter at all;
   extras are session-log-storage-only in bash too. The free-text `display`
   field Rust already printed verbatim (unchanged by this fix) is the
   complete equivalent — no new extras-display mechanism was needed. The
   original audit finding conflated the two.
4. ✅ **RESOLVED, this session.** Migration 0006's config-read failure
   handling silently weakened bash's safety net — a corrupted config file
   got a soft warning where bash hard-aborted. `MigrationContext::config_value`/
   `configured_path_override` changed signature from `Option<String>` to
   `Result<Option<String>, MigrationError>` — `Ok(None)` still means "key
   legitimately absent, falls back to the catalogue default" (unchanged),
   but a genuine read/parse failure (a malformed `.accelerator/config.md`)
   now surfaces as `Err` instead of being collapsed into `None` by
   `FileMigrationContext`'s old `.ok()?` chain. This is not 0006-specific —
   every mechanical migration that reads config (0001, 0002, 0004, 0005,
   0006) now aborts on a genuinely corrupted config file, matching bash's
   own uniform behaviour (every migration shelled out to the same
   `accelerator config` subprocess, so a corrupted file aborted whichever
   migration happened to read it, not just 0006). Every call site across
   the five migration files, `ports.rs`'s default trait method, and one
   test double (`m0003.rs`'s `FakeContext`) updated to propagate via `?`;
   two non-`Result`-returning helpers (`m0004.rs`'s `resolve_layout`,
   `m0006.rs`'s `resolve_corpus_path`, `m0003.rs`'s `warn_pinned_overrides`)
   changed to return `Result` so they could. Black-box test
   (`a_malformed_config_file_aborts_rather_than_silently_skipping`,
   `cli/migrate-cli/tests/migration_0006.rs`) seeds an unterminated YAML
   list in `paths.plans` and asserts a non-zero exit with 0006 never
   recorded applied.
5. **Bash's richer "in-flight session, not fully owned" dirty-tree steer is
   a *documented*, deliberate drop** (Phase 3's own deviation note already
   says so) — repeated here because the interactive-suite audit
   independently rediscovered it as a real, user-observable regression
   (a confused user gets the generic "dirty working tree" refusal instead
   of a session-log-aware resume/discard scaffold) via two different
   trigger paths (fresh foreign dirt; a stale run-id). Not a new finding,
   but the practical impact is now confirmed from two independent angles —
   flagging in case the "deliberate narrowing" call is worth revisiting.

### `test-migrate-0007.sh` — 195 assertions, 35 scenarios: 4 covered, 14 moot, **17 genuine gaps**

Headline: `cli/migrate/src/migrations/m0007/rewrite.rs` (772 lines, the
single largest file in the port — the entire mechanical rewrite dispatch:
type canonicalisation, forbidden-key drop, `pr_title` fold, date
normalisation, status vocab/legacy-map, linkage-key normalisation, the
required-extras backfill/sentinel pipeline) **has zero tests of its own**,
unlike every sibling `m0007/*.rs` file. ~120 of the 195 bash assertions
protect this file's behaviour and currently have no Rust equivalent at any
level.

🔴 marks a gap touching actual mutation correctness (REFUSE/DIVERGE
conditions, merge/precedence rules, status/type reconciliation) — this
migration does irreversible corpus-wide rewrites, so these are the
highest-priority items in the whole audit.

1. 🔴 **Mechanical rewrite happy path** (bash L113-208) — `work_item_id`→quoted
   `id`, `skill`→`producer`, bare-date→ISO, empty-placeholder omission;
   `status: accepted`→`done`, `git_commit`→`revision`; `adr_id`→`id` fold.
   *No test at all.* Add a `mod tests` to `rewrite.rs` (mirroring
   `backfill.rs`'s style) with one fixture per bash case, asserting exact
   output lines.
2. 🔴 **Idempotency** (L212-220) — re-running an already-migrated corpus
   must be a byte-for-byte no-op; untested at every level for the single
   highest-value property of a destructive rewrite. Black-box double-run
   diff in `migration_0007.rs`, plus a unit-level
   `rewrite(&rewrite(x).0) == rewrite(x).0` check.
3. **Plan-path linkage → typed, list arm** (L289-360) — a `relates_to` list
   value carrying a path to a *plan* resolves to `plan:<full-stem>`. Add to
   `linkage_value.rs`'s existing test module.
4. 🔴 **`resolve_path_target`'s default table** (L534-592) — 8 pinned arms,
   notably two *nested* `design-inventory` manifests both literally named
   `inventory.md` that must derive distinct ids from their parent directory
   names (a basename-derived id would collapse both). `cli/corpus/src/linkage.rs`'s
   own test module has zero coverage of `resolve_path_target` at all. Given
   this session's own linkage-table bug was exactly this class of
   regression, this is a high-value addition. New `mod tests` in
   `linkage.rs`, one case per bash arm.
5. **`resolve_path_target`, custom configured dirs** (L593-609) — same
   function, config-awareness. Extend the new `linkage.rs` test module.
6. **`meta/prs/` empty-type inference + `meta/docs/` skip** (L431-533) —
   `resolve_type`'s path-inference fallback is untested (no `rewrite.rs`
   tests exist); the docs-skip is structurally guaranteed but unverified.
   Unit test for the inference; black-box test seeding a `meta/docs/` file
   and asserting it's byte-unchanged.
7. 🔴 **Forbidden-key drop + `pr_title`→`title` fold, 4 cases** (L610-787) —
   no-existing-title→fold; differing-title→drop+DIVERGE; equal-title→drop
   silently; empty-`pr_title`+no-title→clean drop, stem/H1 default
   supplies title. `rewrite.rs:403-418`, zero tests. Four `rewrite.rs`
   unit tests, one per case, asserting exact lines and diagnostic presence
   /absence.
8. 🔴 **`ticket`/`ticket_id` unconditional drop** (L895-987) — a blanket
   destructive-drop rule with no type gating, zero tests. Two `rewrite.rs`
   tests (a note with `ticket:`, a non-note with `ticket_id:`), asserting
   removal + `DIVERGE[dropped-legacy-key]`.
9. 🔴 **Required-extras backfill** (L1073-1475) — the single largest gap.
   `topic` backfills from title (never overwritten); empty
   `verdict`/`lenses` get a quoted `"unknown"` sentinel + diagnostic;
   `pr_number` derives from a genuine `pr-`/`PR-` segment (excluding a
   date-prefix year) or falls back to a **bare, unquoted** `unknown`
   sentinel + a distinct diagnostic; numeric/boolean typed defaults
   (`sequence: 1`, `screenshots_incomplete: true`) must be bare, not
   quoted; the string/enum sentinel bundle routes together; derived
   titles/topics must survive a semicolon unmangled. Zero tests anywhere.
   Needs ~8 `rewrite.rs` tests, one per behaviour, `assert_eq!` on exact
   lines (bash's own emphasis, not `contains`).
10. 🔴 **Backfill completes rather than aborts when underivable** (L1476-1523)
    — must exit 0 via the sentinel path, not fail, when a required extra
    can't be derived (distinct from prepass REFUSE, which does abort).
    Black-box test with a numberless pr-review fixture, asserting exit 0
    + sentinel line present.
11. **Populated extras not clobbered** (L1524-1555) — a presence-check
    regression could overwrite real data with a sentinel or fire a spurious
    diagnostic. `rewrite.rs` test on a fully-populated fixture, asserting
    byte-unchanged output and no backfill diagnostics.
12. **`pr_number_of` boundary cases + frozen required-extras contract**
    (L1575-1612) — 5 pinned stem→number cases (cheap addition); a contract
    test that the derived required-extras set per type hasn't silently
    drifted (catches a schema-TSV edit reclassifying a required field as
    optional).
13. **Combined-corpus capstone** (L1943-1972) — an integration test
    catching cross-rule ordering/interaction bugs isolated unit tests
    can't. Black-box fixture mirroring bash's `seed_combined()`.
14. **Shared stem across types does not collide** (L2034-2082) — protects
    against a **false-positive REFUSE**: `prepass.rs`'s `seen_ids` is
    type-scoped (`"{linkage_type}:{id}"`); only the opposite direction
    (same-type collision refuses) is currently tested. Add to `prepass.rs`'s
    test module.
15. **Byte-equivalence golden** (L2083-2106) — the only bash test pinning a
    whole file's bytes against a checked-in golden; lower priority
    (regression net, not a new rule). Reuse the existing
    `skills/config/migrate/scripts/test-fixtures/migrate-byte-equiv/`
    fixture tree directly in a new `migration_0007.rs` test.
16. **Arbitrary unconfigured subtree skipped** (L2166-2179) — the migration
    must only ever touch configured doc-type directories; architecturally
    guaranteed but untested directly (the exact class of bug this session
    already found once in `linkage_table()`). Black-box test seeding
    `meta/arbitrary/thing.md`, asserting byte-unchanged.
17. *(Non-gap, confirmed correctly disclosed already)*: the VCS-backfilled
    `revision:` case (L203) is the direct, already-disclosed consequence of
    Phase 8's "no per-file VCS history query capability" deviation — not a
    fresh gap.

### `test-migrate-interactive.sh` — 265 assertions, 13 phases: 5 covered, 1 mostly-moot, **7 phases carry genuine gaps**

(The five architectural findings above were discovered via this audit;
listed once, in the "Confirmed real bugs" section, not repeated per-phase.)

1. Phase 2, **session-log-aware dirty-tree pre-flight** (L121-213) — the
   richer steer message; see finding 5 above. No new test without a
   product decision first (a test would just pin the current, poorer
   message).
2. Phase 4, **display elements + inline help + session-log banner**
   (L356-575) — `render_prompt`'s own output text has zero test coverage
   at all (proposed/source/predicate/inline-help lines), independent of
   the banner/extras findings (3 above). Needs `render_prompt` made
   testable (injectable writer) before it can be tested.
3. Phase 5, **WAL invariant across multiple transformations** (L576-731) —
   `write_ahead_log_ordering_is_enforced` only proves ordering for a
   *single* transformation; bash proves that when `apply_decision` fails on
   the *third* of three, the first two (and the failing one) are all
   durably recorded and the ledger never updates. New `engine.rs` test:
   3-transformation `FixtureMigration`, `apply_decision` errors only on the
   third key, assert 3 session-log records afterward.
4. Phase 6, **unknown `schema_version` fail-fast** (L733-975) — see finding
   2 above; also, **"orphan resume record (key no longer emitted) is
   preserved and the run still completes"** is untested (low risk, quick
   to add): `InMemorySessionLog` pre-seeded with an orphan key, run to
   completion, assert the orphan record survives.
5. Phase 7, **doc-example drift test (AC-13)** (L976-1121) — a docs↔behaviour
   drift guard with no Rust equivalent yet, because `skills/config/migrate/SKILL.md`
   itself hasn't been rewritten for the Rust invocation (that's Phase 10).
   Not actionable until Phase 10's doc rewrite lands; tracked here so it
   isn't forgotten afterward — the plan's own `skill_doc_worked_example.rs`
   (Phase 10 point 3) is exactly this test.
6. Phase 9, **structured-stall rendering** (L1192-1299) — the domain event
   is well covered (`engine.rs`), but `StdoutReporter::interactive_stalled`'s
   actual rendered stderr text (`cli/migrate-cli/src/render.rs`) — the
   "MIGRATION STALLED" block, the copy-pasteable resume commands — has
   zero test coverage anywhere. Also untested: a rejected edit immediately
   followed by input exhaustion (validate-then-stall combination). New
   black-box test seeding 0007's ambiguous-band scenario with stdin closed
   and no decisions file, grepping stderr for the exact block; new
   `engine.rs` test for the combined rejected-edit-then-stall sequence.
7. Phase 10, **guarded resume, real-binary rendering** (L1301-1542) — the
   ownership/decision-count *domain logic* is solidly unit-tested
   (`manifest.rs`, `preflight.rs`), but **no black-box test anywhere drives
   `PreflightOutcome::Resumed` through the compiled binary** — `render::resume_affordance`'s
   actual rendered text is completely untested end to end. Single
   highest-value concrete new test in this phase: real git/jj repo (via
   `vcs_test_support::hermetic::Hermetic`, `dirty_tree_preflight.rs`'s
   pattern), manifest+run-id at current HEAD, dirty a manifested path +
   session log, run, assert stderr contains the resume text, the log path,
   and the correct decision count.
8. Phase 11, **multi-migration `--list` segmentation** (L1543-1836) — the
   `# migration <id>` header / position-restart-at-1 branch in `render_list`
   has zero coverage; every existing `--list` test passes exactly one
   group. Can't be black-box tested (0007 is the only real interactive
   migration), but *can* and should be unit-tested: two `FixtureMigration`
   entries through `list_pending`, asserting two independently-1-indexed
   `ListGroup`s.
9. Phase 12, **drift consuming a decision in dry-apply** (L1837-2049,
   minor) — no test combines a drifted resumed record with
   `decisions_file::validate` to prove the drifted key is correctly
   re-included as needing a decision, not mis-flagged as a surplus. New
   `list_and_decisions_file.rs` test: pre-seed a drifted+skipped record,
   assert `pending_transformations` still returns it and `validate` accepts
   exactly one verb.
10. Phase 13, **SKILL.md invoker contract** (L2050-2081) — pure docs-content
    pin against the *old* bash SKILL.md; not actionable until Phase 10's
    rewrite lands (same as Phase 7 above).

### `test-migrate.sh` — 522 assertions, corrected to **164 distinct scenarios** (the `# ── ─` header count of ~87 undercounts — many scenarios use bare `echo "Test: ..."` with no header): 77 covered, 9 moot, **78 genuine gaps**

Full per-scenario detail (line ranges, exact suggested fixtures) is
preserved in this session's agent transcript
(`aa3daa25badcf7811`) and should be pulled from there when the work is
picked up — reproducing all 78 verbatim here would roughly double this
document's length for marginal benefit over the tally below. Summary by
section (bash line ranges):

| Section | Scenarios | Covered | Moot | **Gaps** |
|---|---|---|---|---|
| A. Generic driver (ledger/preflight/manifest/skip/banner), L60-908 | 40 | 20 | 2 | **18** |
| B. Migration 0002, L908-1058 | 18 | 15 | 1 | **2** |
| C. Migration 0003, L1060-1423 | 22 | 8 | 0 | **14** |
| D. Migration 0005, L1734-1945 | 11 | 6 | 0 | **5** |
| E. Migration 0004, L1423-1734 | 35 | 17 | 3 | **15** |
| F. Migration 0006, L1945-2593 | 38 | 11 | 3 | **24** |
| **Total** | **164** | **77** | **9** | **78** |

Highest-priority individual gaps worth naming here even without full
per-item detail (the rest lives in the agent transcript):

- **Section A**: no end-to-end test chains multiple *real* migrations
  through one invocation from a pristine repo (every current black-box
  test pre-marks all-but-one migration applied to isolate it); the path
  manifest's write-during-execution behaviour (append, dedup,
  survives-a-failing-migration) is entirely untested — only its
  read/classify side is; `ManifestStore::clear()` is never asserted to be
  called after a successful run; several `Reporter` event wirings
  (`unknown_applied_id`, `unknown_skipped_id`, `applied_and_skipped`) are
  unit-tested as pure functions but never proven to actually fire during a
  real `run_pending` call.
- **Section C (0003)**: thinnest black-box coverage of any migration (3
  integration tests vs 22 bash scenarios) — `paths.tmp` override handling
  (3 variants), partial-prior-manual-move idempotency (2 variants),
  destination-collision merge semantics, and the pinned-override warning
  for `paths.integrations` (only `paths.templates` is currently exercised)
  are all untested.
- **Section E (0004)**: `cli/migrate-adapters/src/merge_move.rs` — the
  shared collision-resolution primitive multiple migrations depend on —
  **has no tests of any kind, in any crate**; config-key-rename
  notifications (`"0004: renamed ..."`), the `.bak` backup-file creation,
  and three independent `paths.*` override branches are all unreached by
  any currently-passing test.
- **Section F (0006)**: the entire userspace-template-rewriting code path
  (`rewrite_userspace_templates`/`resolve_user_template_path` — tier-1
  vs tier-2 resolution, precedence, alias dedup) has **zero coverage** —
  the single largest gap in this section; the quoted-value/trailing-whitespace/
  empty-value normalisation branches of `normalise_value` are untested;
  the corpus-path-alias dedup warning is untested. Also surfaced the
  confirmed config-read-error-swallowing regression (finding 4 above).

### What this audit does *not* cover

`--list`, `--decisions-file`, `--discoverability-hook`, and migration 0007
are out of `test-migrate.sh`'s scope entirely (covered by the other three
suites above). The audit did not attempt to close any of the ~100 gaps
found — Phase 9's remaining work (closing the confirmed regressions,
writing the highest-value new tests) is scoped separately.

### Gaps closed this session (a first pass, not exhaustive)

All five confirmed real bugs/design questions above are resolved (✅ marked
inline). Beyond those, a small set of the highest-value *test* gaps (not
design questions) were also closed, chosen for being both high-risk
(zero coverage on a widely-depended-on primitive, or on the largest file
in the whole port) and cheaply testable in isolation:

- **`cli/migrate-adapters/src/merge_move.rs`** — had zero tests despite
  being the shared collision-resolution primitive `merge_move` migrations
  0001/0003/0004 all depend on (test-migrate.sh Section E gap #5). Added 7
  tests: absent-source no-op, plain move (file and directory), leaf
  collision (source wins, including a source-file-replaces-a-destination-directory
  case), two-directory merge (source wins on collision, destination-only
  entries survive), and a root-escaping destination refusal.
- **`cli/migrate/src/migrations/m0007/rewrite.rs`** — the 772-line single
  largest file in the port, previously with *zero* tests of its own
  (test-migrate-0007.sh gaps #1/#2, 🔴 both). Added 3 tests: the mechanical
  happy path in one pass (own-id-key canonicalisation, `skill:`→`producer:`,
  `git_commit:`→`revision:`, `ticket:` drop with its diagnostic, all
  together — matching bash's own combined scenario rather than each rule
  in isolation), byte-for-byte idempotency (rewriting an already-rewritten
  document is a pure no-op, no fresh diagnostics), and the empty-`pr_title`-with-no-title
  fallback (confirms no `title: ""` placeholder is ever left behind).
- **`cli/corpus/src/linkage.rs`'s `resolve_path_target`** — zero direct
  tests despite being the function both `CorpusIndex::target_exists` and
  the value-rewrite path depend on (test-migrate-0007.sh gap #4, 🔴).
  Added 6 tests, most notably **two nested `design-inventory` manifests
  literally both named `inventory.md` correctly deriving distinct ids from
  their parent directory names** — the exact regression class this
  session's own `linkage_table()` bug (documented earlier in this plan)
  belonged to. Also covers work-item/plan/pr-description id derivation, a
  path outside every configured directory, and a custom-configured
  directory override.

The remaining gaps (the bulk of the ~100 found, including the
required-extras-backfill cluster in `rewrite.rs`, the userspace-template
tier-1/tier-2 resolution in `m0006.rs`, and the resume-affordance/structured-stall
rendering black-box tests in the interactive suite) are recorded above in
full detail and were **not** closed this session — left as a work list for
a follow-up pass, per this session's own "highest-value only" scoping
decision rather than attempting all ~100 in one sitting.
