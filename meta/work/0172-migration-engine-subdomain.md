---
type: work-item
id: "0172"
title: "Migration Engine Subdomain"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: ready
kind: story
priority: high
parent: "work-item:0136"
blocked_by: ["work-item:0166", "work-item:0167", "work-item:0169", "work-item:0187"]
blocks: ["work-item:0174"]
relates_to: ["work-item:0173", "work-item:0180", "work-item:0182", "work-item:0183"]
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
tags: [rust, migration-engine, concurrency, interactive]
last_updated: "2026-08-01T16:57:37+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-193"
---

# 0172: Migration Engine Subdomain

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

As a maintainer of the Accelerator plugin, I want the meta-directory migration
engine running as native Rust, so that the framework's most stateful cluster
stops depending on bash 3.2 FIFO IPC and on two independently hand-rolled JSON
escapers that must agree.

Port `skills/config/migrate/` into `accelerator-migrate` — the highest-risk port
on the 0136 spine. The bash FIFO/fd IPC, the watchdog, and the embedded awk JSON
parser are replaced by an in-process Rust model with a single `serde_json`-backed
compose/parse path, eliminating the writer/reader escape-agreement hazard.

Also in scope, all of it load-bearing rather than incidental: the author-facing
bash harness at the repository's `scripts/` root (`interactive-harness.sh`,
`interactive-protocol.sh`, the `# INTERACTIVE: yes` opt-in) is retired outright
and its authoring documentation rewritten; `hooks/migrate-discoverability.sh` is
ported onto the bootstrap path and this story rewrites its own `hooks.json`
registration; the `/migrate` skill's call sites and `allowed-tools` globs are
repointed; the new sub-binary is registered in the cli/ workspace, the
enforcement policy and the signed release manifest; the build-system guards in
`tasks/` are adjusted in lockstep with the deletions; and — a deliberate
departure from 0167's remainder-only pattern — **every** assertion in all six
retiring suites is mapped to a named Rust test or an explicit dropped-with-reason
row, not just the non-repointable remainder.

## Context

`skills/config/migrate/` is the most complex and stateful cluster in the plugin.
`run-migrations.sh` (687 lines) drives the lifecycle; `interactive-lib.sh` (984)
runs two named FIFOs plus literal fds for bidirectional IPC (bash 3.2 has no
`coproc`) behind a 30s watchdog escalating SIGTERM→SIGKILL; session-log JSON is
composed by shell and parsed by awk — two independent escape implementations
that must agree; resume is guarded on jj `change_id` vs git `HEAD` with a
fail-closed dirty-tree ownership check; and 7 numbered migrations (2,632 lines)
sit on top.

A **migration** is one numbered, one-shot upgrade step applied to a repo's
`meta/` directory. Migration IDs are slugged, not bare numbers —
`0001-rename-tickets-to-work` — and that full ID is what the state files record
and what `--skip`/`--unskip`/`--unapply` take. A **transformation** is the unit
within a migration that a single interactive decision addresses, so one migration
contains many transformations and the decisions-file contract is
per-transformation.

Migrations `0001`–`0006` are **non-interactive**; `0007` is the only interactive
one. The entire interactive surface of this story — prompt loop, decisions file,
session log, resume, timeout — therefore exists for 0007 alone.

Alongside them, `scripts/interactive-harness.sh` (688 lines) is a **published
author-facing API**: a migration opts in with a `# INTERACTIVE: yes` header and
implements `migration_emit_transformations`, `migration_evaluate_predicate`,
`migration_validate_edit`, `migration_apply_decision` and the optional
`migration_session_log_path` / `migration_verify_applied`, calling
`harness_emit_transformation`, `harness_extras_set`, `harness_field`,
`harness_reject` and finally `harness_run` — all documented in
`skills/config/migrate/SKILL.md`. The FIFO/fd IPC exists precisely because the
driver and each migration are separate bash processes, so the shape of this API
determines whether the IPC can be removed or only relocated. Once the migrations
are Rust and live inside the same binary as the engine, that API has nothing left
to bridge: it is retired rather than replaced, and authoring a migration becomes
ordinary in-crate Rust.

The crate foundation has largely landed: 0178 (`config`, `config-adapters`), 0179
(`corpus`, `corpus-adapters`, `document`, `vcs`) and 0180 (atomic-store lock and
JSONL record primitives in `corpus-adapters`) are all done. Two blockers remain
live: **0167**, which carves `atomic_write` out into the standalone `cli/store`
crate (0166's 2026-07-19 amendment: "0167 owns the carve-out, not 0180"),
establishes the invocation contract and permission-coverage check, and is
expected to add the `--allow-legacy-layout` flag the ported migrations need —
though that flag is recorded only in a note 0167 wrote onto 0178, not in 0167's
own criteria (see Dependencies); and **0169**, which
settles the `hooks.json` registration pattern and the `${CLAUDE_PLUGIN_ROOT}`
expansion probe, while explicitly leaving `hooks/migrate-discoverability.sh` —
script *and* registration — here.

## Requirements

- Implement `accelerator-migrate` over the landed `config` / `config-adapters` /
  `corpus` / `corpus-adapters` / `document` crates plus the `cli/store` crate
  0167 carves out, exposing the surface `run-migrations.sh` has today
  **flag-for-flag**: a default run plus `--list`, `--skip <id>`, `--unskip <id>`,
  `--unapply <id>`, `--decisions-file <path>` and `--help`. The flag shape is
  preserved verbatim — no subcommand redesign is in scope, and 0164's dispatch
  conventions apply only to how `migrate` itself is reached from the
  `accelerator` entrypoint. A `status` command is **out of scope** — it has no
  bash antecedent.
  - `<id>` is a full slugged migration ID (`0001-rename-tickets-to-work`), not a
    bare number and not a transformation key. `--unapply` removes the ID from the
    applied ledger so the migration becomes pending again.
- Preserve the documented lifecycle contract in full (see Technical Notes for the
  authoritative list drawn from `skills/config/migrate/SKILL.md`): the clean-tree
  pre-flight over `meta/`, `.claude/accelerator*.md` and `.accelerator/`; state
  reading with unknown-ID preservation and the applied-wins warning when an ID
  appears in both files; discovery of `migrations/[0-9][0-9][0-9][0-9]-*.sh`
  in sorted order with the `ACCELERATOR_MIGRATIONS_DIR` override; the
  `<ID> — <description>` preview banner with a per-migration `--skip` hint and
  the `No pending migrations.` early exit 0; apply-in-order with atomic ledger
  append on success and last-successful-migration state on failure; the
  `MIGRATION_RESULT: no_op_pending` soft-deferral sentinel (stripped from
  user-visible output, migration stays pending); and the applied/skipped/pending
  summary counts.
- Port the 7 numbered migrations as Rust, retiring the author-facing bash harness
  (`scripts/interactive-harness.sh`, `scripts/interactive-protocol.sh`, the
  `# INTERACTIVE: yes` opt-in) and rewriting the authoring documentation in
  `skills/config/migrate/SKILL.md` and `docs/migrations.md` accordingly. Only
  with the migrations in-process does the FIFO/fd IPC genuinely disappear rather
  than get reimplemented across a Rust↔bash boundary.
  - The harness is retired **without a replacement author-facing API**. A
    migration becomes ordinary Rust inside the migrate crate; there is no opt-in
    header, no published entrypoint set, and no stable authoring contract to
    specify beyond the user- and agent-facing behaviour below.
- Replace the FIFO/fd IPC and the 30s watchdog with an in-process Rust
  concurrency model, and replace the dual hand-rolled JSON escape (shell writer
  + awk reader) with a single `serde_json`-backed compose/parse path.
  - Records follow 0180's canonical field order (AC-7): `transformation_key`,
    `schema_version: 1`, `outcome` ∈ `{accepted, edited, skipped}`,
    `proposed_value`, `user_value` (presence-based), `timestamp`, then
    author-declared extras in declaration order. (0180's AC-7 says "declaration
    order"; with the author-facing harness retired, in-crate emission order *is*
    that order — the terms are the same rule, not a divergence.)
  - Enforce the invariant 0180 delegated here: `user_value` is present exactly
    when `outcome` is `edited`. 0180's emission is presence-based by design and
    does not police the coupling.
  - Honour 0180's stricter record validation (its AC-8): `proposed_value` is
    required and non-empty. The bash writer documents it required
    (`jsonl-common.sh:60`) but omits it from its emptiness check (`:115-116`), so
    a bash-written log can hold records the Rust composer refuses — see the
    cutover requirement below.
  - **Session-log cutover.** On encountering a session log a bash writer produced
    before the cutover, the engine reads it and rewrites the whole file in
    canonical Rust form as a single atomic replacement *before* appending
    anything. That is the atomic cutover 0180's byte-parity scope-out assumes: the
    Rust engine never appends to a bash-written file. A record that is readable
    but invalid under 0180's stricter rule is **normalised if its decision is
    recoverable and refused otherwise, never silently dropped** — the concrete
    rule is settled in planning against the real corpus, since stranding an
    in-flight repo defeats the cutover's purpose.
  - The timeout keeps the bash watchdog's **30s default bound**, injectable for
    tests, **not** user-configurable — no new config key or CLI flag.
- Preserve the interactive framework's user- and agent-facing contract as
  documented:
  - The accept / `edit <value>` / skip decision verbs (ADR-0037 §4), each
    prompted decision durably persisted to the session log **before** the
    artefact is mutated (§3 write-ahead-log invariant), `RESUMED_APPLIED` /
    `RESUMED_SKIPPED` on re-entry, and the migration ID appended to the applied
    ledger only when the harness emits `DONE`.
  - Predicate routing (§1): exit 0 routes to the prompt loop, exit 1 applies
    mechanically with **no record persisted**, any other non-zero aborts the
    migration.
  - The `[interactive] <message>` rejection format, re-prompting rather than
    applying.
  - Sticky skip semantics; source-drift re-prompting with the stale record
    discarded by key; refusal to resume from a log with an unknown
    `schema_version`, with a recovery instruction.
  - Callbacks are invoked more than once per run (`--list` enumeration, dry-apply,
    live apply), so the ported equivalents must be deterministic, and the
    edit validator must remain a pure function of its arguments.
- Preserve the agent-invocation contract (0115) as documented, with its steps
  named correctly — `list → decide → write → resume`:
  - `--list` dry-emits **every pending interactive transformation**, one
    tab-delimited line each, as
    `<position>\t<key>\t<proposed>\t<path>:<field>`, without mutating the corpus.
    With more than one pending interactive migration the output is segmented by a
    `# migration <id>` header, `<position>` restarts at 1 per migration, and a
    stderr note is printed. It is the **decisions file** that is per-migration —
    a single multi-migration decisions file is not supported — not `--list`.
  - Decisions map to transformations by emission order; skipped and mechanical
    transformations consume no line; blank lines and `#` comment lines are
    ignored; CRLF endings are tolerated.
  - `--decisions-file <path>` and the documented
    `ACCELERATOR_MIGRATE_DECISIONS_FILE` env var (discoverable via `--help`) are
    both retained.
  - The up-front **dry-apply validation pass** mutates nothing and fails closed:
    an unknown verb, a count mismatch either way, or a rejected `edit` value that
    no following line corrects exits non-zero, **names the offending position**,
    and leaves the corpus unmutated. Once the live apply begins there is no
    rollback.
  - With no decision input available, the run emits the structured stall
    (`MIGRATION STALLED: no decision input available`, owned by **0116**) naming
    the decisions-file path `.accelerator/state/migrations-<id>-decisions.txt` and
    a copy-pasteable bare resume command requiring no `FORCE`.
- Preserve the guarded-resume and staleness semantics: jj `change_id` vs git
  `HEAD` keying, and 0119's contract exactly as it shipped —
  - Ownership comes from a **per-run path manifest**: plain text, one
    repo-relative path per line, co-located with the per-migration ledger in the
    run's state area, appended **at the moment each path is mutated** so a
    mid-migration failure's partial writes are still recorded as owned.
  - When **every** dirty path is owned, the pre-flight **proceeds** into the apply
    loop (exit 0) without `ACCELERATOR_MIGRATE_FORCE=1`, emitting a
    resume-affordance message to stderr listing **every** owned dirty path.
  - When any dirty path is not owned, the pre-flight **refuses**: non-zero exit,
    **no** resume-affordance message, and the existing dirty-tree refusal message
    (the `ACCELERATOR_MIGRATE_FORCE` hint) present.
  - An **unusable** manifest — absent, empty, unreadable, or carrying a different
    run's identity (stale) — yields an empty owned set, so any dirty tree gets
    the same observable refusal. Never "everything owned".
  - `ACCELERATOR_MIGRATE_FORCE=1` bypasses the dirty-tree pre-flight only;
    skipped migrations stay skipped even with FORCE.
- Consume 0167's legacy-layout escape rather than the retired env var.
  `ACCELERATOR_MIGRATION_MODE` was **deliberately dropped** by 0178 and stays
  unhonoured, with a retained negative test
  (`config-adapters/tests/config_reader.rs`). What replaces it is 0167's explicit
  per-invocation `--allow-legacy-layout` flag on the **read** subcommands, which
  carries both halves the env var used to: suppressing the uniform legacy-layout
  refusal *and* enabling the `.claude/accelerator{,.local}.md` source fallback.
  Migrations `0001`–`0006` pass it directly; `0007` reaches it via the allowlisted
  `doc-type-table.sh`; `check-call-site-migration.sh` confines it to the migration
  engine. The Rust migrations must obtain the same capability through the same
  explicit, greppable path — no reintroduced ambient env var.
- Port `hooks/migrate-discoverability.sh` onto the **bootstrap path**
  (`${CLAUDE_PLUGIN_ROOT}/bin/accelerator`, shipped by 0164) **and rewrite its
  own `hooks.json` registration**. 0169 rewrites only the `vcs-detect`/
  `vcs-guard` registrations, and 0167 records that "`migrate-discoverability` is
  0172's", so this edit is unowned by any other story. What is inherited from
  0169 is the registration *pattern* and the resolved `${CLAUDE_PLUGIN_ROOT}`
  expansion/argument-splitting probe.
- Rewrite every remaining call site of the deleted scripts onto `accelerator …`,
  with their `allowed-tools` globs, in the same change that deletes them. That is
  the `/migrate` skill's own invocations **and
  `skills/config/configure/SKILL.md:561`**, which names
  `bash run-migrations.sh --skip` and is claimed by no other story — 0167 scoped
  this cluster's call sites out of its removal set and 0173 claims only the
  corpus/design/collaboration sites. The confinement guard is now Python
  (`tasks/lint/skill_permissions.py`, with
  `tests/unit/tasks/test_call_site_migration.py`) rather than the shell
  `check-call-site-migration.sh` earlier drafts named.
- Register the new sub-binary: a `cli/Cargo.toml` workspace-member entry, a
  `cargo-pup` rule, `cli/deny.toml` coverage for any new dependency (0162), and a
  `manifest.json` entry with per-target artefacts, checksum, minisign signature
  and `description` in 0165's pipeline. A sub-binary absent from the signed
  manifest cannot be fetched or verified at first use.
- Retire the shell on 0167's pattern — repoint, verify green, inventory, port,
  delete — with two deliberate departures:
  1. Where `test-migrate-interactive.sh` and `scripts/test-interactive-protocol.sh`
     cannot be repointed because their assertions drive the FIFO/fd protocol
     directly, they are rewritten in Rust as **black-box tests of the migration
     process** — driving the compiled binary and asserting observable behaviour,
     not engine internals — preserving the intent of the original assertions.
  2. **Every** assertion in all six retiring suites is mapped, not only the
     non-repointable remainder. 0167's pattern treats the repointed green run as
     the gate for the bulk and inventories only what cannot repoint; this story
     inventories everything, so the cluster keeps a countable regression floor
     after the shell is gone. See Drafting Notes for the cost.
  Then drop `_EXPECTED_MIGRATE_SUITES` from `tasks/test/integration.py`, the three
  migrate/interactive entries from `SHELL_LIBRARIES` in `tasks/lint/scripts.py`,
  and adjust the suite-count floor for every retiring suite, all in the same
  change as the deletions.
- State the disposition of `scripts/jsonl-common.sh`. Per 0180 its only
  production caller is the migrate session log in `interactive-lib.sh`, which this
  story deletes — so it is orphaned here. Either retire it in this change or defer
  it to 0174 with a recorded surviving-consumer count, following the precedent
  0167 set for `config-common.sh`. (`scripts/atomic-common.sh` is different: it
  has many callers beyond this cluster and survives.)

## Acceptance Criteria

### Suite classification

- [ ] Before any script is deleted, every assertion in the six retiring suites is
      classified repointable-at-the-CLI-surface or not, per-assertion where a
      suite is mixed, using a committed extractor, with each suite's total
      assertion count recorded. The classification sizes the black-box rewrite and
      supplies the corpus the assertion inventory is checked against. **The
      recorded total is a decision point, not a formality**: for calibration 0167
      measured 337 assertions in a 6,289-line suite, so several hundred rows are
      plausible here. If the total exceeds 400, the exhaustive mapping is narrowed
      to the three suites where irrecoverable `meta/` corruption actually lives
      (`test-migrate-0007.sh`, `test-migrate-interactive.sh`,
      `scripts/test-interactive-protocol.sh`) and 0167's remainder-only pattern is
      retained for the rest, with the decision and the count recorded in the
      inventory.
- [ ] Where `test-migrate-interactive.sh` or `scripts/test-interactive-protocol.sh`
      prove non-repointable, their assertions are rewritten in Rust as black-box
      tests driving the compiled binary, each mapped to a named test or a
      dropped-with-reason row.

### Command surface

- [ ] The applied ledger (`.accelerator/state/migrations-applied`), skip list
      (`.accelerator/state/migrations-skipped`) and session log
      (`.accelerator/state/migrations-<id>-session.jsonl`, overridable) are read
      and written at those documented paths with the same record semantics —
      newline-delimited full slugged IDs for the two state files, one JSON object
      per line for the log — so a repo left mid-migration by the bash engine is
      picked up rather than stranded. The per-run path manifest is a **sidecar
      pair** at the shipped paths `.accelerator/state/migrations-run-paths.txt`
      (`RUN_PATHS_FILE`, one repo-relative path per line) and its run-id sidecar
      (`RUN_ID_FILE`, holding the recorded base revision); both are read and
      written there.
- [ ] Given the `migrate/all-pending/` fixture, when `accelerator migrate` runs
      with no flags, then the same migrations apply in the same order and the
      applied ledger holds the same IDs in the same order as the ledger
      `run-migrations.sh` produced. The comparison is set-and-order over ledger
      entries, **not** bytes.
- [ ] Bash baselines are captured and committed as goldens **as the first ordered
      step of the work**, at a recorded commit, before any other change — the
      capture window closes irreversibly once the scripts are deleted, and it also
      closes if 0173 removes `scripts/validate-corpus-frontmatter.sh` first (see
      Dependencies). If the window is missed, the validator is temporarily
      restored at the capture commit rather than falling back to a self-authored
      oracle. Planning fixes the fixture × artefact table before capture;
      non-interactive fixtures (`0001`–`0006`) require ledger, skip-list,
      manifest, corpus-state and stdout goldens only, having no session log and an
      empty `--list`. Artefacts covered: applied ledger, skip list, the per-run
      path manifest pair, post-run corpus state, session log, `--list` stdout,
      user-visible stdout of `--skip`/`--unskip`/`--unapply`, the preview banner,
      and the exit code of **every** flag invocation and every enumerated error
      condition. Each artefact's comparison basis is stated with it: ledger and
      skip list as ordered ID lists; manifest as an ordered path list; session log
      record-by-record on `transformation_key`/`outcome`/`proposed_value`/
      `user_value` with `timestamp` normalised (byte comparison is excluded per
      0180's carve-out); corpus state byte-for-byte after normalising volatile
      frontmatter; `--list` stdout byte-for-byte after sandbox-root normalisation;
      banners and remaining stdout byte-for-byte **after normalising any
      invocation path or program name** — this story rewrites every invocation to
      `accelerator …` and deletes `run-migrations.sh`, so bash-byte parity is not
      achievable for output naming the driver.
- [ ] `--help` is **not** compared against bash bytes. It is pinned to a committed
      **Rust** snapshot (0167's precedent that doc comments are contract) and
      asserted for content parity with the bash surface: every flag of the
      Requirements surface present, the same `<path>:<field>` column vocabulary,
      and `ACCELERATOR_MIGRATE_DECISIONS_FILE` discoverable.
- [ ] A fixture matrix covers each migration `0001`–`0007` individually with its
      own before/after golden.
- [ ] Every flag has a test asserting its observable effect and its exit code
      against the captured baseline: `--list` matches the bash-derived `--list`
      golden byte-for-byte (after sandbox-root normalisation) without mutating the
      tree; `--skip` causes the named
      migration not to apply on the next run and `--unskip` causes it to apply
      again; `--unapply` removes the ID from the applied ledger so the migration
      re-runs; `--decisions-file` and `ACCELERATOR_MIGRATE_DECISIONS_FILE` drive a
      non-interactive run identically; `--help` matches its committed Rust snapshot
      per the content-parity criterion above.
- [ ] Each error condition exits with the same code `run-migrations.sh` produces,
      with empty stdout and a stderr message pinned by substring: unknown `<id>`
      to `--skip`/`--unskip`/`--unapply`, a missing or unreadable decisions file,
      an unrecognised verb, and a verb-count mismatch in either direction. The
      dry-apply pass names the offending position and leaves the corpus unmutated.
- [ ] Lifecycle behaviours hold: `No pending migrations.` with exit 0 when nothing
      is pending; the `<ID> — <description>` banner with a `--skip` hint per
      pending migration; an unknown ID from a newer plugin preserved verbatim and
      warned about; an ID in both state files warned about with applied winning; a
      migration emitting `MIGRATION_RESULT: no_op_pending` staying pending with
      the sentinel stripped from output; and a mid-run failure leaving the ledger
      at the last successful migration.

### Interactive framework

- [ ] The transcript for the retained `interactive/doc-example/` fixture under its
      **documented** scripted decisions — `edit ` (empty, rejected),
      `edit 0123-renamed`, `skip` — is captured from the bash implementation at a
      recorded pre-deletion commit and committed as a golden. Both captures are
      normalised by replacing the sandbox root with `<SANDBOX>` (the documented
      transcript's first line is
      `Session log: <SANDBOX>/.accelerator/state/migrations-0099-doc-example-session.jsonl`)
      and the session-log basename's migration id with `<ID>`; **no other
      normalisation and no exemptions are permitted**. The normalised Rust
      transcript must then match the normalised golden exactly, including the
      `[interactive] empty value not allowed` line and the re-prompt that follows
      it. A committed test driver supplies the scripted decisions over the chosen
      transport and captures combined stdout/stderr for both implementations.
- [ ] For that same fixture the session log contains exactly two records —
      `link-A` `edited` with `user_value` `0123-renamed`, and `link-C` `skipped` —
      and **no** record for the mechanical `link-B`, whose predicate exited 1.
- [ ] The `accept` verb is covered by its own fixture and criterion (the
      doc-example transcript exercises only `edit` and `skip`): accepting applies
      the proposed value and records `outcome: accepted` with no `user_value`.
- [ ] Given a three-decision interactive fixture and a test-only abort seam that
      fails deterministically after the second decision is persisted, a re-run
      reads the first two from the session log and prompts exactly the third.
- [ ] For each decision, a recording store port shows the session-log append
      completing **before** the first corpus mutation. Aborting at the seam
      between them leaves resumable on-disk state.
- [ ] Given a validator-rejecting fixture, the rejection is reported as
      `[interactive] <message>` and the transformation is re-prompted, never
      applied.
- [ ] Source drift — a recorded `proposed_value` differing from the live emission
      — re-prompts and discards the stale record by key. A log with an unknown
      `schema_version` is refused with a recovery instruction. A transformation
      skipped on a prior run stays skipped.

### Agent invocation (0115 / 0116)

- [ ] Given the `migrate/0007/` fixture (an interactive migration pending), no
      TTY, fd 0 at EOF and no decisions file, the run emits the structured stall
      `MIGRATION STALLED: no decision input available` naming the pending decision
      keys, the decisions-file path and a bare copy-pasteable resume command
      requiring no `FORCE`, exits non-zero, and mutates no corpus artefact beyond
      any non-interactive migration that legitimately ran first. The stall is
      reached without the timeout being armed, asserted via the injected timeout
      seam rather than by elapsed time.
- [ ] Given a decisions file with blank lines, `#` comments and CRLF endings,
      decisions map to transformations in `--list` emission order, and skipped and
      mechanical transformations consume no line.
- [ ] Given two pending interactive migrations, `--list` emits both, segmented by
      `# migration <id>` headers with `<position>` restarting at 1 per migration
      and a stderr note, matching the bash golden; a decisions file is consumed
      against one migration only.
- [ ] Given a dirty tree whose paths are **all** in this run's manifest, with two
      such paths, the run proceeds without `ACCELERATOR_MIGRATE_FORCE`, does not
      re-prompt recorded decisions, completes, and emits a stderr
      resume-affordance message naming **both** owned paths (pinned by substring).
- [ ] Given a dirty tree including at least one path **not** in the manifest, the
      run exits non-zero, emits **no** resume-affordance message, and emits the
      dirty-tree refusal message carrying the `ACCELERATOR_MIGRATE_FORCE` hint
      (pinned by substring), mutating nothing.
- [ ] The usability gate matches the shipped implementation, which is finer than
      0119's prose: an **absent or unreadable** manifest, or an **absent, empty or
      unreadable run-id sidecar**, refuses; a **stale** pair — recorded base
      revision unequal to the current one, i.e. the operator committed since the
      failed run — refuses. Each produces the same observable refusal (non-zero,
      no affordance message, refusal message present). An **empty manifest is
      valid, not a refusal**: an interactive interrupt before any mechanical delta
      leaves one, and requiring non-empty would make that resume unreachable — the
      per-path ownership loop is the sole authority, so an empty manifest plus a
      dirty mechanical path still refuses.
- [ ] Ownership matches the implementation's three classes: runner-managed
      bookkeeping files (applied ledger, skip list, and the manifest pair itself)
      are implicitly owned; current-run interactive session artefacts
      (`migrations-*-session.jsonl`, `-stderr.log`, `-resume-state.tmp`) are owned
      **by pattern**, gated by the base-revision check so a stale run's artefacts
      are not owned, and deliberately stay out of the mechanical manifest; every
      other dirty path must appear in the manifest verbatim.
- [ ] Given a stub migration that mutates a known set of paths then exits
      non-zero, the manifest contains exactly the paths mutated before the
      failure, one repo-relative path per line, including the failing migration's
      partial writes.
- [ ] Given a foreign dirty path with `ACCELERATOR_MIGRATE_FORCE=1`, the run
      proceeds and applies; a skipped migration remains skipped even so.

### Timeout

- [ ] The default bound is 30s, asserted against the default value directly.
- [ ] With the bound injected short, an interactive migration that never yields a
      decision exits non-zero within that bound plus 2s, writes a stderr message
      pinned by substring with empty stdout, and leaves a session log whose next
      run prompts only undecided transformations. The SIGTERM→SIGKILL escalation
      is an implementation detail.

### Resume and staleness

- [ ] Given a session log recorded at revision R, resuming under jj at a different
      `change_id` — or git at a different `HEAD` — exits non-zero with the
      stale-log diagnostic on stderr and mutates nothing.
- [ ] Given a log at the current `change_id`/`HEAD`, the log **is** reused and
      only undecided transformations are prompted.

### JSON

- [ ] Records are composed and parsed through one `serde_json`-backed path: the
      awk parser is absent from the tree, and a round-trip covers adversarial
      values (embedded double quotes, backslashes, newlines, tabs, non-ASCII).
- [ ] Emitted records match 0180's canonical field order pinned against 0180's
      golden record; `user_value` is present exactly when `outcome` is `edited`
      (both violation directions rejected); and a record whose `proposed_value` is
      empty or absent is rejected per 0180's AC-8.
- [ ] A static check asserts no surviving shell or awk file names a session log:
      `grep -rn 'session[-_]log' scripts/ hooks/ skills/ tasks/ --include='*.sh'
      --include='*.bash' --include='*.awk'` prints **zero matching lines** (`grep -c`
      totals 0; grep's own exit status is 1), with its run at the
      recorded pre-deletion commit committed as a known-positive floor. Any
      intended exemption is listed inline in this criterion.
- [ ] Given a bash-written session log, the injected store port records exactly one
      rename onto the log path with no prior append, the rename precedes any
      append, and the decision set read back after the rewrite equals the
      pre-rewrite set compared field-by-field with timestamps normalised. Given
      that log truncated mid-record, the run exits non-zero with a stderr message
      pinned by substring, the log file is byte-unchanged, and no corpus artefact
      is mutated.

### In-process transport

- [ ] A committed check (cargo-pup rule or equivalent) asserts the migrate crate
      creates no named FIFO and spawns no child process on the decision path, so
      the IPC is removed rather than reimplemented inside the binary. Any
      legitimate exception is allowlisted with a reason.

### Discoverability hook

- [ ] The SessionStart reminder runs via Rust through the bootstrap path, and this
      story's change rewrites the `migrate-discoverability` entry in `hooks.json`
      from the bash script path to that invocation.
      `hooks/test-migrate-discoverability.sh` passes repointed at the binary
      before that suite is retired.

### Legacy layout

- [ ] The ported migrations obtain legacy-layout access through 0167's
      `--allow-legacy-layout` read-subcommand flag, and a committed check asserts
      no reintroduced `ACCELERATOR_MIGRATION_MODE` handling anywhere in `cli/` —
      0178's negative test stays green.
- [ ] `0007`'s legacy access no longer routes through `doc-type-table.sh`, and the
      confinement guard (`tasks/lint/skill_permissions.py`) is **either** updated
      for a tree where the bash migration engine it confined no longer exists,
      **or** deleted with 0167's owner's agreement recorded — both branches
      satisfy this criterion, since which applies is an Open Question owned by
      0167, not by this story.

### Documentation and call sites

- [ ] `skills/config/migrate/SKILL.md` and `docs/migrations.md` describe writing
      an in-crate Rust migration, covering declaring transformations, predicate
      routing, prompting, validating an edited value, and resume, with a worked
      example compiled or doctested in CI so it cannot rot.
- [ ] A committed check —
      `grep -rn 'interactive-harness\|interactive-protocol\|# INTERACTIVE:\|harness_run\|harness_reject\|migration_validate_edit' scripts/ hooks/ skills/ docs/ tasks/ cli/ .claude-plugin/`
      — prints **zero matching lines** (`grep -c` totals 0; grep exits 1), with its
      pre-deletion run committed as a
      known-positive floor.
- [ ] `SKILL.md`'s invocations and `allowed-tools` rules name `accelerator …`
      rather than any deleted script path, verified by 0167's permission-coverage
      check, in the same change as the deletions.

### Registration and distribution

- [ ] `accelerator-migrate` is a `cli/Cargo.toml` workspace member, carries a
      `cargo-pup` rule, and every new dependency is covered by `cli/deny.toml`;
      `mise run cli:check`, cargo-deny and cargo-pup all pass.
- [ ] `manifest.json` carries an `accelerator-migrate` entry with per-target
      artefacts, checksum, minisign signature and `description`, and a
      fetch-and-verify test resolves it end to end.

### Parity and retirement

- [ ] Every suite (or every assertion of a mixed suite) classified **repointable**
      is repointed at the compiled binary and observed **green in CI at a recorded
      commit**, the commit recorded in the inventory, before any script it covers
      is deleted. Non-repointable assertions are instead covered by their named
      Rust black-box tests, passing, before deletion.
- [ ] **Every** assertion in all six suites is inventoried, keyed by
      `<file>:<line>`, and mapped to a named Rust test with a recorded disposition
      — ported, rewritten against the new invocation shape, rewritten as a Rust
      black-box test, or dropped with a reason. The repointed green run proves
      equivalence at the cutover; it does **not** discharge the mapping.
- [ ] The inventory names the non-repointable subset explicitly: assertions
      driving the FIFO/fd protocol or the harness directly; the
      `test-migrate-0007.sh:2208` `exec`-stub region; assertions covering the
      three awk helpers; and every retiring script with no covering suite. For
      members with no covering suite it carries 0167's depth floor — every
      top-level branch and every distinct exit code as its own row.
- [ ] A committed script, run in CI, asserts the inventory has no duplicates and
      no gaps against a fresh extraction over every suite and retiring file named
      in Technical Notes, using the classification extractor.
- [ ] These files are absent from the tree: the six suites at the paths given in
      Technical Notes; `skills/config/migrate/scripts/run-migrations.sh` and
      `interactive-lib.sh`; the three awk helpers; the seven migrations;
      `scripts/interactive-harness.sh`; `scripts/interactive-protocol.sh`;
      `hooks/migrate-discoverability.sh`; and the non-retained part of
      `scripts/test-fixtures/interactive/`. `skills/config/migrate/scripts/` and
      `migrations/` contain no `.sh`, `.bash` or `.awk` file. A residual check
      `grep -rn 'mkfifo' scripts/ hooks/ skills/ --include='*.sh'` returns exactly
      0, with a committed pre-deletion known-positive floor.
- [ ] `_EXPECTED_MIGRATE_SUITES` is removed from `tasks/test/integration.py`, the
      three `SHELL_LIBRARIES` entries are removed from `tasks/lint/scripts.py`,
      and the suite-count floor covering each retiring suite is corrected by the
      number of suites it loses (or confirmed not to exist) — all in the same
      change as the deletions, so CI never goes green→red on a floor mismatch.
- [ ] `scripts/jsonl-common.sh` is either absent from the tree or recorded in
      Technical Notes as deferred to 0174 with its surviving-consumer count.
- [ ] `mise run` exits 0.

### Cross-item records

- [ ] The golden-capture ordering constraint is recorded on **0173** — its removal
      of `scripts/validate-corpus-frontmatter.sh` must follow this story's
      recorded golden-capture commit — with a reciprocal edge, so 0173 cannot
      destroy the 0007 oracle unwarned.
- [ ] The 0180/0168 session-log-reader question is settled: either the visualiser
      does read session logs and a `relates_to` edge is recorded on both items, or
      0180's consumer claim is marked superseded by 0168's record.
- [ ] An ADR-reconciliation follow-up work item exists and is linked from here,
      covering ADR-0023, ADR-0037 §5 and ADR-0038 against the Rust port, before
      the deletion commit.
- [ ] The `--allow-legacy-layout` obligation — the flag itself, the crate-level
      form this binary needs, and the confinement guard — is written into **0167's
      own** Requirements and Acceptance Criteria before 0167 closes, so it cannot
      be marked done without shipping what these migrations depend on.
- [ ] The 0182 coupling is recorded on 0182 (reciprocal edge) and its
      `CLAUDE_PLUGIN_ROOT` allowlist entries for the files this story deletes are
      removed in the deletion change, with the `hooks.json` entry ordering agreed
      so the two edits do not collide.
- [ ] 0183's disposition is agreed and recorded on both items before either
      starts, and this story's discoverability criterion is updated to pin the
      resulting output channel.
- [ ] If `scripts/jsonl-common.sh` is deferred rather than retired here, its
      disposition and surviving-consumer count are recorded on **0174** itself,
      following the precedent 0167 set for `config-common.sh`.

## Open Questions

- The concrete shape of the interactive transport — an in-process channel versus
  stdin/stdout framing — is deferred to planning. The binding constraint is the
  documented interactive contract, not the mechanism. Note that under agent
  invocation there is no TTY and fd 0 is at EOF, which may rule out stdin framing
  outright.
- Whether a `migrate status` command should exist. Scoped out here for want of a
  bash antecedent.
- The normalisation rule for a bash-written record that is readable but invalid
  under 0180's AC-8 (`proposed_value` empty or absent). Settled in planning
  against the real corpus: the choice is between normalising a recoverable
  decision and refusing the log, and it decides whether a real in-flight repo
  migrates or is stranded.
- What replaces `doc-type-table.sh`'s allowlisted `--allow-legacy-layout` use for
  the Rust 0007, and whether `check-call-site-migration.sh` survives at all once
  the bash migration engine it confines is gone. Raised with 0167's owner.

## Dependencies

- Blocked by **0167** (`ready`, not done): carves `atomic_write` into the
  standalone `cli/store` crate (0166's 2026-07-19 amendment — "0167 owns the
  carve-out, not 0180"); establishes the invocation contract, the bootstrap-path
  naming, the `allowed-tools` matcher conventions and the permission-coverage
  check one criterion invokes; and adds the `--allow-legacy-layout` read flag the
  ported migrations depend on. **The edge is one-sided by 0167's deliberate
  choice**: its `blocks` is `[0169, 0173, 0174]` and it records 0170–0172 in prose
  only. If bidirectional traversal matters, 0167's `blocks` needs amending — this
  item does not claim a reciprocal edge exists.
- Blocked by **0169**: settles the `hooks.json` registration pattern and resolves
  the `${CLAUDE_PLUGIN_ROOT}` expansion/argument-splitting probe. It rewrites the
  `vcs-detect`/`vcs-guard` registrations and leaves `migrate-discoverability` —
  script and registration — here.
- Blocked by **0166** (done): retained because 0166 declares `blocks: 0172`.
- Blocked by **0187** (generalises the sub-binary registration surface). This
  story adds a dispatch token; it does not generalise the surface. Registration
  follows the checklist 0187 adds at
  `tasks/README.md#registering-a-dispatched-sub-binary`. (2026-08-01)
- Blocks: 0174.
- Parent: epic 0136.
- Crate foundation (done): 0178 (`config`, `config-adapters`), 0179 (`corpus`,
  `corpus-adapters`, `document`, `vcs`), 0180 (mkdir lock with PID-owner reclaim,
  and canonical-order JSONL compose/remove, in `corpus-adapters`). `document` is
  consumed for 0007's frontmatter rewriting.
- **0164** (done): supplies the bootstrap path `${CLAUDE_PLUGIN_ROOT}/bin/accelerator`
  that the hook is ported onto, and the fetch-verify-cache model that makes the
  new sub-binary resolvable at first use.
- **0116** (done): owns the structured stall this story preserves.
- **0115**: the agent-invocation contract (`list → decide → write → resume`) and
  the `ACCELERATOR_MIGRATE_DECISIONS_FILE` interface.
- **0119** (done): the per-run path manifest, affordance-on-proceed, refusal
  without affordance, and fail-closed-on-unusable-manifest contract, carried in
  full above.
- Live constraint from **0180** (`relates_to`): its byte-parity scope-out rests on
  the premise that no JSONL file is written by both implementations. This engine
  honours it via the read-then-atomically-rewrite cutover — it never appends to a
  bash-written file. If that cannot be guaranteed, raise it on 0180 before
  cutover. 0180 also delegated the `outcome=edited` ⇔ `user_value` coupling here
  (AC-7) and imposes the stricter `proposed_value` rule (AC-8).
- **`scripts/jsonl-common.sh` is orphaned by this story.** 0180 records its only
  production caller as the migrate session log in `interactive-lib.sh`. No other
  0136 story claims it. Its disposition is a requirement above.
- Cross-cluster coupling on **0173**. This item declares `relates_to: 0173`;
  0173 declares nothing back, so the edge is one-sided today and recording the
  reciprocal note is Cross-item-records work, not something already done.
  *Post-port*: 0007's `self_validate_structural` gate runs against
  `scripts/validate-corpus-frontmatter.sh`, which 0173 owns; the Rust 0007
  self-validates through `corpus`/`document` instead, so no inversion is created.
  *Pre-port*: bash 0007 must stay runnable to capture its golden and serve as the
  `test-migrate-0007.sh` oracle, but 0173 is `blocked_by: [0167]` only, mentions
  0172 nowhere, and its criteria already delete that validator — so it is
  unblocked the moment 0167 lands. A criterion above requires recording the
  ordering constraint on 0173.
- Also inherited from 0167: `test-migrate-0007.sh:2208` writes an `exec` stub
  hard-coding the config resolver path, so that suite breaks at 0167's deletion
  rather than at this cutover; repointing must absorb the fix first.
- **0182 (plugin-root rename, `ready`, high) — a live constraint on almost every
  surface this story touches.** It renames `CLAUDE_PLUGIN_ROOT` to
  `ACCELERATOR_PLUGIN_ROOT` and carries a criterion that **no tracked file under
  `cli/` contains the string `CLAUDE_`**, which the new migrate crate would
  violate as this item is currently written. It adds `hooks/shim-refresh.sh` as a
  fourth SessionStart entry that must be appended **at index 3**, so this story's
  `hooks.json` edit must not collide with it. Its `CLAUDE_PLUGIN_ROOT` allowlist
  enumerates `skills/config/migrate/**`, migrations `0001`–`0007`,
  `scripts/interactive-harness.sh` and `hooks/**` — every one of which this story
  deletes, so those entries must be removed with them. And it records that
  `run-migrations.sh:643` and `interactive-lib.sh:433,744` *write* the variable
  into migration environments, behaviour the Rust engine must replace under the
  new name. 0182's `relates_to` omits 0172, so the coupling needs recording from
  both ends.
- **0183 (SessionStart advisories on stderr reach nobody, bug) — same hook.** It
  moves the migrate-discoverability advisory off stderr onto a top-level
  `systemMessage` in the single JSON object on stdout, keeps the advisory naming
  `/accelerator:migrate` plus the highest-applied and highest-available ids, and
  extends the very suite this story retires. Whichever lands first invalidates the
  other. Disposition must be settled before either starts: either this story
  absorbs 0183's `systemMessage` contract into its discoverability criterion and
  0183 closes as superseded, or 0183 lands first and this story inherits its
  envelope. The discoverability criterion pins no output channel until that is
  decided.
- **`--allow-legacy-layout` is not yet an obligation on 0167.** The flag, the
  `doc-type-table.sh` allowlisting and the confinement guard are recorded only in
  a "Notes from 0167 (2026-07-22)" block inside **0178** — dated after 0167's own
  last update — and appear in none of 0167's Requirements, Acceptance Criteria or
  Dependencies. 0167 can therefore be marked done without shipping it, and because
  0167's `blocks` deliberately omits 0172, nothing signals the loss. A further
  shape question: the flag as described is a **CLI affordance on the `accelerator
  config` read subcommands**, while this binary consumes the config crates
  in-process, and no item records a crate-level equivalent. Both are Cross-item
  records work below.
- New-binary registration: 0165's pipeline and 0162's enforcement policy. Not
  optional.
- External (Claude Code): no TTY and fd 0 at EOF under Bash-tool agent invocation
  — the defect 0115/0116 address — constraining the transport question. Hook I/O
  envelope shapes and `hooks.json` expansion behaviour come from 0167/0169;
  record the version each is verified against. Minimum supported version
  v2.1.144.

## Assumptions

- The 7 migrations are ported to Rust rather than kept as bash children. If they
  stay bash, the FIFO/fd IPC must be reimplemented as a Rust↔bash wire protocol
  and the story's headline goal is not met.
- All seven are ported rather than some retired. A publicly distributed plugin
  cannot assume a floor on how far behind a user's repo is. If a
  "repos below migration N upgrade via an earlier release" policy were ever
  adopted, the oldest migrations and their fixtures become the cheapest scope
  reduction — recorded so the lever stays visible.
- The author-facing bash harness is not an external compatibility surface, so
  retiring it breaks no external author, and no replacement API is owed.
- Most migrate suites repoint in bulk as 0167 found for config, but the
  interactive pair may not. That is a sizing input, not a scope risk: the fallback
  is a Rust black-box rewrite of known kind.

## Technical Notes

- Source bash to retire (~12.5k lines: ≈5,233 source + ≈7,276 suites, before the
  awk helpers and fixture tree):
  - `skills/config/migrate/scripts/run-migrations.sh` (687),
    `skills/config/migrate/scripts/interactive-lib.sh` (984)
  - awk helpers in the same directory: `0007-frontmatter-rewrite.awk`,
    `frontmatter-frag.awk`, `frontmatter-merge.awk`
  - `scripts/interactive-harness.sh` (688), `scripts/interactive-protocol.sh` (169)
  - `skills/config/migrate/migrations/0001`–`0007` (2,632; 0007 is 856)
  - `hooks/migrate-discoverability.sh` (73)
  - Suites, with paths: `skills/config/migrate/scripts/test-migrate.sh` (2,593),
    `skills/config/migrate/scripts/test-migrate-0007.sh` (2,229),
    `skills/config/migrate/scripts/test-migrate-interactive.sh` (2,081),
    `skills/config/migrate/scripts/test-migrate-snapshot.sh` (159),
    `scripts/test-interactive-protocol.sh` (108),
    `hooks/test-migrate-discoverability.sh` (106)
- **Documented state paths** (from `skills/config/migrate/SKILL.md` — these are
  not planning unknowns): applied ledger
  `.accelerator/state/migrations-applied`; skip list
  `.accelerator/state/migrations-skipped`; session log
  `.accelerator/state/migrations-<id>-session.jsonl` (override via
  `migration_session_log_path`, relative paths resolved against `PROJECT_ROOT`);
  decisions file `.accelerator/state/migrations-<id>-decisions.txt`; per-run path
  manifest `.accelerator/state/migrations-run-paths.txt` plus its run-id sidecar
  (`run-migrations.sh:27-31`). 0119 left the manifest filename "to be fixed at
  planning" but the implementation shipped, so it is a fact, not a choice.
- **`--list` skips the entire pre-flight** (`run-migrations.sh:311-315`) — no
  manifest or run-id setup, no RESUME state, no guarded-resume branch. It is a
  dry read-only emit that excludes already-decided keys via the resume filter and
  notes any in-flight session log on stderr. Criteria that assert `--list` does
  not mutate the tree are asserting this property.
- **Which `store` is which.** `cli/store` is the standalone atomic
  whole-file-write crate (`atomic_write` only, 0167's carve-out, not yet landed);
  `cli/corpus-adapters/src/store.rs` holds 0180's mkdir lock and JSONL
  compose/remove; `cli/config-adapters/src/store.rs` holds the config layout
  read. The "recording store port" in the criteria is a test double for the
  corpus-adapters write path. `scripts/atomic-common.sh` survives retirement —
  it has many callers beyond this cluster (config, work-item-sync, the
  jira/linear write wrappers).
- **Per-migration inventory.** `0001`–`0006` are non-interactive; `0007` is the
  only interactive one and the largest (856 lines), owns
  `0007-frontmatter-rewrite.awk`, its own 2,229-line suite and its
  `self_validate_structural` gate. Planning fills in what each of `0001`–`0006`
  transforms and their individual line counts, to size the fixture matrix.
- Fixtures. `scripts/test-fixtures/interactive/doc-example/` is **retained**,
  relocated under the migrate crate's fixtures as `interactive/doc-example/`; the
  rest of `scripts/test-fixtures/interactive/` retires with the suites. Additional
  fixtures the criteria name and which planning must define:
  `migrate/all-pending/`, `migrate/0001/`…`migrate/0007/`, a three-decision
  interactive fixture, an `accept`-verb fixture, a validator-rejecting fixture, a
  foreign-dirty-path fixture, a two-owned-dirty-paths fixture, and the four
  manifest-state variants. Technical Notes records which additionally need a
  captured bash baseline.
- Guards to adjust: `_EXPECTED_MIGRATE_SUITES = 4` in `tasks/test/integration.py`
  is an **at-least floor** (`integration.py:163` compares with `<`, matching
  0167's account), covering four of the six suites, so it is
  *removed* rather than decremented; the three `SHELL_LIBRARIES` entries
  (`scripts/interactive-harness.sh`, `scripts/interactive-protocol.sh`,
  `skills/config/migrate/scripts/interactive-lib.sh`); and the floors covering
  `hooks/test-migrate-discoverability.sh` and `scripts/test-interactive-protocol.sh`
  — planning confirms where each lives or that none exists.
- 0167 is the parity-pattern exemplar; read its Parity and Suite-lifecycle groups
  first, including its rule that a residual grep's pattern and corpus are fixed
  in the criterion rather than chosen at verification time.
- This is the most subtle concurrency port in the epic; the repointed suites and
  the bash-derived goldens are the oracle until Rust tests replace them.

## Drafting Notes

- **Command surface fixed flag-for-flag.** The extracted item named `migrate run`,
  `migrate status` and `migrate discoverability`, none of which exist. `--unapply`
  was verified present (`run-migrations.sh:60,92`). `<id>` was corrected from a
  bare number to the full slugged migration ID the state files record.
- **Review 1 pass 3 correction — the `--list` contract.** Earlier drafts asserted
  `--list` emits "the first pending migration's transformations only" and raised
  an Open Question about non-interactive first migrations. `SKILL.md` documents
  the opposite: `--list` emits **every** pending interactive transformation,
  segmented by `# migration <id>` with `<position>` restarting per migration and a
  stderr note; it is the **decisions file** that is per-migration. Corrected in
  Requirements and criteria, and the Open Question deleted as already answered.
  The invented "single-pending-migration scoping" term is gone.
- **Review 1 pass 3 correction — 0119's semantics were inverted.** Earlier drafts
  made the every-owned-path message a *refusal*. In 0119 it is a
  **resume-affordance emitted when the pre-flight proceeds (exit 0)**, and the
  refusal path asserts *no* affordance message plus the FORCE-hint refusal
  message. Split into separate proceed and refuse criteria, with manifest
  correctness and the four fail-closed states each given their own criterion and
  "stale" defined as carrying a different run's identity.
- **Review 1 pass 3 correction — the legacy read path.** Earlier drafts had the
  migrations consuming an `ACCELERATOR_MIGRATION_MODE` gate in
  `cli/config/src/legacy.rs`. No such gate exists: 0178 deliberately dropped the
  env var and keeps a negative test proving it is unhonoured. The replacement is
  0167's per-invocation `--allow-legacy-layout` read flag, passed directly by
  `0001`–`0006` and via the allowlisted `doc-type-table.sh` for `0007`, confined by
  `check-call-site-migration.sh`. Rewritten and re-attributed to 0167, with the
  two knock-on questions raised.
- **Review 1 pass 3 correction — the parity gate.** The gate demanded all six
  suites go green repointed while the neighbouring criteria exist because two may
  not be repointable. Scoped to repointable suites and assertions, with
  non-repointable ones discharged by their passing black-box tests, matching
  0167's structure.
- **Review 1 pass 3 correction — the four steps and the stall's owner.** The flow
  is `list → decide → write → resume`, not "`--list` → decisions-file → run →
  verify"; there is no verify step. The structured stall is owned by **0116**, not
  0115.
- **Review 1 pass 3 correction — the transcript's scripted decisions.** An earlier
  draft added `accept` to the doc-example transcript. The documented transcript
  uses `edit ` (empty, rejected), `edit 0123-renamed`, `skip` over three
  transformations of which `link-B` is mechanical. Restored, with the two-record
  session-log assertion added and `accept` given its own fixture and criterion.
- **Lifecycle contract captured.** Earlier drafts omitted most of the documented
  contract: the `MIGRATION_RESULT: no_op_pending` soft deferral, the preview
  banner and `No pending migrations.` exit, unknown-ID preservation, the
  applied-wins warning, `ACCELERATOR_MIGRATIONS_DIR`, the dry-apply validation
  pass with its position-naming failure, predicate routing, sticky skip, source
  drift, and the callback-determinism requirement. These are now in Requirements.
  **Not all have criteria** — an earlier version of this note wrongly claimed they
  did. Uncovered at work-item level and deferred to the plan: `MECHANICAL_APPLIED`,
  callback determinism and validator purity (documented but not enforced in the
  bash either), the summary counts, the pre-flight's three path roots,
  `ACCELERATOR_MIGRATIONS_DIR`'s in-crate translation, the
  `RESUMED_APPLIED`/`RESUMED_SKIPPED`/`DONE` tokens, and the
  abort-on-other-non-zero predicate branch.
- **No replacement author-facing API.** With migrations in-crate the harness has
  nothing to bridge, so it is retired outright. What survives is the user- and
  agent-facing behaviour, pinned against bash-era goldens.
- **Every assertion is mapped, not just the remainder.** Decided deliberately
  against the cheaper alternative of treating the repointed green run as the
  coverage contract. The fixture matrix covers each migration's main paths, but
  the branch-level cases inside ~7,276 lines of suite are where a regression in a
  one-shot engine hides, and it surfaces only while rewriting a user's `meta/`
  directory. The cost is the largest single addition to this story — roughly three
  times the remainder-only volume — and it is a stated departure from 0167's
  pattern, now recorded in the Summary and Requirements rather than only in the
  criteria.
- **Watchdog reframed as a timeout contract**; 30s preserved, asserted directly,
  test-injectable, deliberately not user-configurable.
- **Oracles moved off self-authored documents**, and every golden's comparison
  basis now stated per artefact — the session log cannot be compared byte-wise
  (0180's canonical record carries a `timestamp`, and byte parity is scoped out),
  so it is compared record-by-record with timestamps normalised.
- **Sizing — kept as one story, with the parent-with-children option recorded.**
  The retired surface exceeds 0167's, and this epic decomposed 0166 into
  0178/0179/0180, so decomposition deserves an answer rather than a dismissal.
  The axes:
  - (a) engine, ledger flags and `0001`–`0006` versus (b)/(d) the interactive
    framework plus `0007` and its suite — one cut, since 0007 is the only
    interactive migration, and cleaner than it first appears (item A needs neither
    the session log nor the decisions-file flow). Rejected as a *release* split
    because **0007 must stay applicable**: shipping A alone leaves the newest
    migration unavailable, so either 0007 lands in A or A's engine bridges to bash
    0007, relocating the IPC.
  - (c) the discoverability hook and shell retirement — separable, but the hook is
    small and the retirement must land with the deletions it guards.
  - **Parent-with-children, delivered in sequence under one release** — the 0166
    precedent. An earlier draft rejected this because "only the final child would
    be independently releasable", which does not discriminate: 0166's children
    (0178/0179/0180) were independently *mergeable* increments that only
    collectively delivered releasable value, so per-child releasability was never
    the test. The real argument is narrower. The cutover itself is one indivisible
    commit — the deletions, the guard and floor edits, the `hooks.json` rewrite,
    the call-site and `allowed-tools` rewrites and the 0182 allowlist removals all
    land together or CI goes green→red — and 0182's `no CLAUDE_ under cli/` guard
    plus its index-3 `hooks.json` entry make a long-lived half-migrated state
    *riskier* than a single cutover, not safer. Children would therefore partition
    the work *before* that commit (engine and `0001`–`0006`; interactive framework
    and `0007`) while the commit itself stayed whole — buying sequencing, not
    smaller delivery. **Decision: keep one story**, with an explicit trigger — if
    the Suite-classification total exceeds 400 assertions, *or* planning cannot
    produce a single cutover commit, the item is promoted to a parent along that
    seam. Both are measurable before implementation starts.
- **Assertion-grade detail is deferred to `/create-plan`, deliberately.** Review 1
  ran four passes and findings did not converge (48 → 37 → 42 → 45): each pass
  added precision, and precision created new surfaces to contradict. Pass 4's
  remaining findings are largely plan-grade — the fixture × artefact table, exact
  normalisation tokens, the extraction procedure behind the depth floor, the
  stale-log/manifest/FORCE precedence, stream-emptiness preconditions where the
  preview banner and prompts have already written to stdout, and the
  `RESUMED_*`/`DONE` tokens restated in observable terms. Sibling 0119 — whose
  guarded-resume contract this story inherits — set the precedent: its review
  reached APPROVE at pass 3 having explicitly deferred "assertion-grade detail
  (exact stderr marker token, AC4 condition-splitting, manifest dedup/ordering) to
  be settled in /create-plan". This item closes out on the same basis: criticals,
  cross-item couplings and factual corrections fixed here; the assertion-grade
  remainder is planning work.
- Removed the `extract-work-items` provenance disclaimer, which no longer holds.

## References

- Source: `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Blocked by: `meta/work/0167-config-command-and-invocation-contract-migration.md`,
  `meta/work/0169-vcs-subdomain-and-hooks-migration.md`,
  `meta/work/0166-shared-config-corpus-store-crates.md`
- Blocks: `meta/work/0174-retire-shell-tooling-and-ci-guards.md`
- Ordering constraint to record on:
  `meta/work/0173-remaining-subdomains-corpus-design-collaboration.md`
- Live constraints to reconcile with:
  `meta/work/0182-cli-derives-plugin-root-from-own-location.md` (the
  `ACCELERATOR_PLUGIN_ROOT` rename, the `no CLAUDE_ under cli/` guard, the
  index-3 `hooks.json` entry, and the allowlist entries covering files this story
  deletes), and
  `meta/work/0183-session-start-hook-advisories-reach-nobody-on-stderr.md`
  (the advisory-channel change to this story's hook)
- Bootstrap path and dispatch: `meta/work/0164-launcher-and-git-style-dispatch.md`
- Distribution and enforcement:
  `meta/work/0165-multi-binary-distribution-and-release-pipeline.md`,
  `meta/work/0162-rust-toolchain-guard-rails.md`
- Contracts this story preserves: 0115 (agent invocation), 0116 (structured
  stall), 0119 (guarded resume and the per-run path manifest), 0069 (interactive
  validation hooks), 0092 (interactive contract ADR), 0180 (atomic-store
  primitives), 0168 (visualiser fold-in — possible session-log reader)
- ADRs: ADR-0023 (meta-directory migration framework), ADR-0037 (optional
  interactive contract — §§1–4 are the runner guarantees this story preserves; §5
  routes new framework primitives back as supplementary ADRs), ADR-0038
  (interactive validation parameters for the unified-schema linkage migration),
  ADR-0047 (multi-level userspace configuration model), ADR-0052 (filesystem as
  message bus and knowledge corpus), ADR-0053 (thin CLI over a hexagonal
  ports-and-adapters core)
- Docs: `skills/config/migrate/SKILL.md` (the authoritative lifecycle,
  interactive and agent-invocation contract), `docs/migrations.md`
- Prior research: `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md`
