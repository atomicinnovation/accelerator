---
type: plan
id: "2026-08-07-0172-migration-engine-subdomain"
title: "Migration Engine Subdomain (accelerator-migrate) Implementation Plan"
date: "2026-08-07T09:08:23+00:00"
author: Toby Clemson
producer: create-plan
status: in-progress
work_item_id: "work-item:0172"
parent: "work-item:0172"
derived_from: ["codebase-research:2026-08-06-0172-migration-engine-implementation-research"]
tags: [rust, migration-engine, concurrency, interactive, cli]
revision: "4056d016bf415f182aa18b785d3177b81c04a458"
repository: "accelerator"
last_updated: "2026-08-08T13:15:00+00:00"
last_updated_by: Toby Clemson
last_updated_note: "Phases 0-4 implemented and committed. Phase 4 wires the session log onto corpus::RecordStore/FileCorpusStore and adds the bash-session-log cutover. Deviations recorded inline: SessionLogRewriter landed as cutover(path) (no bytes parameter) since the zero-dependency domain crate cannot compose/parse JSON at all; no standalone session_cutover.rs domain file (AC-8 validation reuses compose_record's own checks); corpus_adapters::jsonl widened to pub (mirroring lock in Phase 3), gaining a new parse_record; and a real, previously-unenforced gap this phase's own AC surfaced — the user_value/outcome coupling rule — is now enforced in compose_record with tests for both violation directions."
schema_version: 1
---

# Migration Engine Subdomain (accelerator-migrate) Implementation Plan

## Overview

Port `skills/config/migrate/` — the bash meta-directory migration engine (a
687-line driver, a 984-line FIFO/fd interactive runner, a 688-line author-facing
harness, a 169-line wire protocol, three awk helpers, and 7 numbered migrations
totalling 2,632 lines, 856 of them the only interactive one) — into a native
Rust sub-binary, `accelerator-migrate`. The FIFO/fd IPC, the 30s watchdog, and
the dual hand-rolled JSON escaper (shell writer, awk reader) are retired
outright: with migrations as in-process Rust rather than forked bash children,
the wire protocol they existed to carry has nothing left to bridge.

## Current State Analysis

The crate foundation this item depends on is fully landed: `config` /
`config-adapters` (0178), `corpus` / `corpus-adapters` / `document` / `vcs` /
`vcs-adapters` (0179), the atomic-store lock and canonical-order JSONL
primitives in `corpus-adapters` (0180), the standalone `cli/store` crate and
the `--allow-legacy-layout` flag (0167), the bootstrap/fetch-verify-cache path
and `hooks::session_start`'s `systemMessage` envelope (0164/0169), and the
sub-binary registration checklist and template (0187, realised concretely by
`cli/vcs-cli/`).

**Correction, this review pass:** an earlier version of this plan believed
migration 0007's typed-linkage extraction had "no crate yet exists" and was
entirely 0195's to build. This is false — `cli/corpus/src/linkage.rs`
already implements extraction/classification (`parse_document`,
`type_from_path`, `classify_band`, `resolve_path_target`) as landed Rust in
`corpus`, a crate `migrate` already depends on; only target-existence
checking is missing, and 0172 builds that itself (Phase 8 point 5). **0172's
only remaining dependency on work item 0195 is Phase 8's Obligation 2**
(whether `scripts/validate-corpus-frontmatter.sh`'s self-validation logic
gets a Rust equivalent or stays alive as a shell script until this plan's
cutover) — narrower than the original blanket "0195 must expose linkage
extraction" framing. Per the confirmed direction below, Phase 8 (0007's
port, for Obligation 2 only), Phase 7 (the discoverability hook and skill
rebinding — both compute their behaviour from the compiled registry, which
is incomplete until Phase 8 registers 0007, so Phase 7 stays gated on
Phase 8 regardless of which obligation is blocking it), and Phase 10 (the
retirement cutover) cannot complete until 0195's Obligation-2 edge resolves.
Phases 0–6 and 9 have no such dependency and are independently implementable
and mergeable now.

### Key Discoveries

- **The bash IPC exists only because the driver and each migration are
  separate processes.** Once migrations are ordinary in-process Rust
  (`meta/work/0172-migration-engine-subdomain.md:88-91`), the TSV frame
  protocol (`scripts/interactive-protocol.sh:9-59`), the two named FIFOs plus
  literal fds (`skills/config/migrate/scripts/interactive-lib.sh:736-753`),
  and the 30s SIGTERM→SIGKILL watchdog
  (`interactive-lib.sh:937-954`) have nothing left to carry. A migration's
  four callbacks become four trait methods called as plain Rust function
  calls — no wire format, no escaping, no base64, no dry-fork subprocess mode
  (`_interactive_fork`, `interactive-lib.sh:403-515`) — because `--list` and
  the dry-apply validation pass can simply call the same in-process functions
  a second time instead of forking a child in a different mode.
- **`config::paths::doc_type_dirs()` already exists** as a pure,
  infrastructure-free function (`cli/config/src/paths.rs:30-67`) taking a
  `&dyn ConfigAccess` and returning the doc-type → directory table. This is
  the in-process replacement for `scripts/doc-type-table.sh`'s
  `config paths --doc-types --format tsv` subprocess shell-out
  (`scripts/doc-type-table.sh:17-56`) — migration 0007's Rust port calls it
  directly against a `FileConfigStore::at(root).with_legacy_policy(LegacyPolicy::Allow)`,
  with no `ACCELERATOR_MIGRATION_MODE`-gated flag threading needed at all.
- **`corpus::Record`/`corpus::Outcome` already match 0180's canonical
  session-log shape exactly** (`cli/corpus/src/record.rs:26-34`:
  `transformation_key, schema_version, outcome, proposed_value,
  user_value: Option<String>, timestamp, extras: Vec<(String,String)>`), and
  `corpus_adapters::FileCorpusStore` already implements `RecordStore`
  (`append_record`/`remove_by_key`) with the mkdir-lock and canonical-order
  JSONL compose (`cli/corpus-adapters/src/store.rs:107-161`,
  `cli/corpus-adapters/src/lock.rs`). The session log needs **no new record
  type and no new store** — it is a `FileCorpusStore` instance rooted at the
  project, writing through the existing port.
- **`atomic_jsonl_remove_by_key` (`scripts/atomic-common.sh:222-247`) has
  exactly one production caller anywhere in the repository —
  `interactive-lib.sh:907`, which this plan deletes** — confirmed by a
  repository-wide grep. Its only other real consumer is
  `scripts/jsonl-common.sh` itself (self-referential). Per the confirmed
  direction, Phase 10 deletes `scripts/jsonl-common.sh` and strips
  `atomic_jsonl_remove_by_key` (and its `source jsonl-common.sh` line) from
  `scripts/atomic-common.sh` in the same change — `atomic-common.sh`'s other
  functions (`atomic_write`, `atomic_append_unique`, `atomic_remove_line`,
  `atomic_jsonl_append`) keep their many unrelated production callers and are
  untouched.
- **The registration template is `cli/vcs-cli/`** (`cli/vcs-cli/Cargo.toml`,
  `cli/vcs-cli/src/main.rs`, `cli/pup.ron:75-121`): a three-crate split —
  `vcs` (domain, zero infrastructure), `vcs-adapters` (outbound adapters,
  `InProcessProbe` using `gix`/`jj-lib` directly, no subprocess), `vcs-cli`
  (the `accelerator-vcs` binary, thin `main.rs` dispatching to `run_*`
  functions, a shared `report()` mapping `kernel::Error::Refusal` → exit 2).
  This plan mirrors that shape exactly: `migrate` (domain), `migrate-adapters`
  (outbound), `migrate-cli` (the `accelerator-migrate` binary). Because the
  binary crate is **not** at `cli/migrate/` (the domain crate owns that path),
  a `_SUBBINARY_MANIFESTS` entry is required in `tasks/manifest.py`
  (`"migrate": CLI_DIR / "migrate-cli/Cargo.toml"`), mirroring `vcs-cli`'s
  own entry there.
- **The bash watchdog bounds post-decision child teardown, not the human's
  decision wait — bash's TTY read has no timeout at all today.** The
  watchdog (`interactive-lib.sh:937-954`, "30s after sending the last
  frame, escalate") only starts once frame relay has completed — i.e.,
  after every decision has already been made — and bounds how long the
  forked child takes to exit cleanly afterward. The actual blocking TTY
  read (`read_decision`, `interactive-lib.sh:269`:
  `IFS= read -r line </dev/tty || return 1`) has no `-t` flag or any other
  timeout mechanism anywhere in the file; a human at a real terminal can
  currently wait indefinitely with no bash-side cutoff. **The Rust port's
  30s TTY decision timeout is therefore new, stricter behaviour this port
  introduces, not a narrowing of an existing bash mechanism** — it is
  justified directly by 0172's own AC (Timeout section: default 30s,
  injectable, exits non-zero within bound+2s, stderr message, empty
  stdout, resumable session log; the SIGTERM→SIGKILL escalation itself "is
  an implementation detail" the AC doesn't require reproducing). Once
  migrations are in-process Rust, the only remaining source of unbounded
  blocking is a human at a TTY not answering a prompt — every other
  callback (`emit_transformations`, `evaluate_predicate`, `validate_edit`,
  `apply_decision`) is required to be a deterministic, pure, terminating
  function by ADR-0037's own callback-determinism requirement
  (`SKILL.md:147`). Phase 0's fixture matrix cannot capture a bash golden
  for this specific timeout, since bash never observably times out here —
  the Rust behaviour is validated against the AC's stated contract
  directly, not against a bash-captured baseline. Phase 10's documentation
  rewrite must describe this as a behaviour change, not silently port it
  as if bash already worked this way.
- **The bash "no decision input available" stall is immediate, not
  timeout-gated** — it fires on an up-front TTY/decisions-file absence check
  (`emit_no_input_stall`, `interactive-lib.sh:313-345`), never after waiting
  30s. The Rust design keeps this distinction: a `NoInputDecisionSource`
  adapter (selected when there is no TTY and no decisions file) returns
  `DecisionError::NoInputAvailable` immediately; only the TTY adapter's
  blocking read is subject to the injectable timeout.
- **Every exact user-facing string this plan must reproduce byte-for-byte or
  substring-pin** was extracted with file:line citations across
  `run-migrations.sh`, `interactive-lib.sh`, and `interactive-harness.sh` (the
  preview banner's em dash, `No pending migrations.`, the dirty-tree refusal,
  the resume-affordance block, the full `--help` text, every dry-apply
  validation message naming "position N", the `MIGRATION STALLED` block with
  its flush-left copy-pasteable commands, the stale-log/unknown-`schema_version`
  diagnostics, `[interactive] <message>`, the `--list` tab-delimited format,
  and the watchdog's two escalation lines) — reproduced verbatim per phase
  below rather than re-derived at implementation time.
- **`ACCELERATOR_MIGRATE_FORCE` and `ACCELERATOR_MIGRATE_DECISIONS_FILE` are
  the only two env vars the ported surface honours**; `ACCELERATOR_MIGRATION_MODE`
  stays permanently dead (0178's negative test,
  `cli/config-adapters/tests/config_reader.rs:67`, must stay green — the Rust
  engine obtains legacy access via
  `FileConfigStore::with_legacy_policy(LegacyPolicy::Allow)`
  (`cli/config-adapters/src/store.rs:29-33,57-60`) called directly, never via
  an environment read). **`ACCELERATOR_MIGRATIONS_DIR` is dropped, not
  honoured** — see Phase 2's registry design; it is a documented removal, not
  a silent one (Phase 10's documentation rewrite records it explicitly as no
  longer recognised).

## Desired End State

`accelerator migrate` (dispatched via the `migrate` token) reproduces
`run-migrations.sh`'s full flag surface flag-for-flag
(`--skip <id>`, `--unskip <id>`, `--unapply <id>`, `--list`,
`--decisions-file <path>`, `--help`, and a default run) with the same
observable lifecycle, guarded-resume, interactive, agent-invocation, and JSON
contracts as the bash implementation, verified against bash-captured
goldens — with one deliberate exception: the 30s TTY decision timeout is new
behaviour this port introduces (bash's own decision wait is unbounded today,
per the Key Discoveries correction), validated directly against 0172's own
AC rather than against a bash golden, since none can exist for a scenario
bash never observably hits. All 7 migrations run as in-process Rust. The
FIFO/fd IPC, the
watchdog, the author-facing harness, the awk JSON parser, and all six retiring
shell test suites are deleted in one cutover commit once (a) every
`repointable`-classified suite/assertion is green against the compiled binary
and (b) migration 0007's self-validation dependency on work item 0195's
Obligation 2 is resolved (linkage extraction itself has no such dependency —
Current State Analysis's correction). `mise run` exits 0 throughout every
intermediate phase and after the cutover.

**Verification**: `mise run cli:check` is green after every phase; the
fixture-driven test suite added in each phase passes; the six retiring shell
suites stay green and unmodified (they are the oracle) until Phase 9/10;
`mise run` exits 0 at the end of Phase 10.

## What We're NOT Doing

- No `migrate status` command (no bash antecedent — explicitly out of scope
  per the work item).
- No subcommand redesign — the flag shape is preserved verbatim; `migrate`
  takes flat flags, not `migrate <subcommand>`.
- No replacement author-facing migration-authoring API — a migration becomes
  ordinary in-crate Rust; there is no opt-in header or published hook set to
  design.
- No new interactive framework primitive (control verb, display element,
  resumability guarantee) beyond what ADR-0037/0038 already specify — ADR-0037
  §5's recursive supplement clause forbids introducing one ad hoc; this plan
  introduces none.
- No dry-run mode (ADR-0023 rejects it) and no rollback mechanism beyond VCS
  revert.
- No change to `scripts/atomic-common.sh` beyond the one `jsonl-common.sh`-only
  strip in Phase 10 — its other functions and their many unrelated production
  callers (config, work-item-sync, jira/linear wrappers) are untouched.
- No implementation of 0195's `linkage extract` primitive itself — this plan
  only consumes it once it ships, and records the interface contract 0195
  must satisfy (Phase 8) as a cross-item obligation.
- No decomposition into a parent-with-children work item ahead of time. The
  work item's own trigger (Suite-classification total exceeding 400
  assertions, or an unproduceable single cutover commit) is measured in Phase
  9 and decided then, per its own stated process — not pre-decided here.

## Implementation Approach

Ten phases. Phases 0–6 and 9 are independently mergeable now; Phases 7, 8,
and 10 are each sequenced but gated on 0195's narrower Obligation 2
(self-validation script lifetime — see Current State Analysis's correction;
linkage extraction itself has no 0195 dependency, since `corpus::linkage`
already implements it) — Phase 7 because both its discoverability-hook
logic and its skill rebinding compute their behaviour from the compiled
registry, incomplete until Phase 8 registers 0007. Phase 0 captures
bash-derived goldens first (the item's own irreversibility constraint — the
capture window closes once the scripts are deleted or once 0195 deletes
`scripts/validate-corpus-frontmatter.sh`). Phases 1–6 build the full
mechanical engine, guarded resume, JSON model, interactive framework, and
mechanical migrations 0001–0006, entirely alongside the still-live bash
implementation (both exist simultaneously; nothing bash is touched or
deleted). Phase 7 ports the discoverability hook and rebinds the skill,
blocked on Phase 8. Phase 8 ports 0007 — extraction, classification, and
existence-checking are all built directly in this plan now; only the
self-validation obligation remains gated on 0195. Phase 9 builds the
assertion-inventory extractor and classifies/repoints/rewrites the six
suites (no 0195 dependency — it measures the shell suites, not 0007
itself). Phase 10 is the single indivisible cutover commit: delete bash,
rewrite call sites, adjust guards
and floors, retire `jsonl-common.sh`, and close out every cross-item record
the work item's acceptance criteria require.

Test-driven throughout: each phase's Rust behaviour is written against a
fixture and a captured bash golden (Phase 0) or a table-driven unit test
(domain logic with no bash antecedent, e.g. the ownership-class resolution),
never against a re-derivation of what the bash "should" do from memory.

---

## Phase 0: Fixture Matrix & Bash Golden Capture

### Overview

The item's own AC requires bash baselines be captured **as the first ordered
step of the work, before any other change** — the window closes irreversibly
once the scripts are deleted, and independently if 0195 deletes
`scripts/validate-corpus-frontmatter.sh` first. This phase fixes the fixture ×
artefact table and captures every golden, following the `hooks/test-fixtures/vcs-detect/regenerate.sh`
+ `CAPTURE-SOURCE.txt` template. No Rust code is written in this phase.

### Fixture matrix

New fixture root: `cli/migrate-cli/tests/fixtures/` (mirrors
`cli/vcs-cli/tests/fixtures/` in shape). Fixtures, each a constructed `meta/` +
`.accelerator/` tree plus a `CAPTURE-SOURCE.txt`:

| Fixture | Purpose | Artefacts captured |
|---|---|---|
| `all-pending/` | every migration pending, default run | ledger, stdout (preview banner + summary), exit code |
| `0001/` … `0006/` | one migration each, before/after state | ledger, corpus-state diff, stdout, exit code |
| `0007/` | interactive migration pending, no decisions | `MIGRATION STALLED` block (exact, unredacted-except-`<SANDBOX>`), exit code |
| `interactive/doc-example/` | retained fixture, scripted `edit ` / `edit 0123-renamed` / `skip` | transcript (stdout+stderr), session log (2 records, timestamps redacted), exit code |
| `interactive/accept-verb/` | new — exercises `accept` alone | session log (`outcome: accepted`, no `user_value`), transcript |
| `interactive/three-decision/` | new — abort seam after 2 of 3 decisions, then resume | session log after abort, transcript of resumed run (only 3rd prompted) |
| `interactive/validator-rejecting/` | new — an edit that fails `migration_validate_edit` | `[interactive] <message>` line, re-prompt |
| `interactive/foreign-dirty-path/` | dirty tree, one path not in manifest | refusal message (no affordance), exit code |
| `interactive/two-owned-dirty-paths/` | dirty tree, both paths owned | resume-affordance message naming both paths, exit code |
| `manifest-states/{absent,empty,unreadable,stale}/` | 0119's four fail-closed states | refusal message + exit code (identical across all four) |
| `list/single-pending/`, `list/multi-pending/` | `--list` output shape | tab-delimited stdout, `# migration <id>` segmentation, stderr note |
| `decisions-file/{blank-crlf-comments,too-few,too-many,unknown-verb,rejected-edit-no-recovery}/` | dry-apply validation | position-naming error message, exit code, corpus-unmutated assertion |

Every artefact's **comparison basis** is fixed now, matching the AC precisely:
ledger/skip-list as ordered ID lists; manifest as an ordered path list; session
log record-by-record on `transformation_key`/`outcome`/`proposed_value`/`user_value`
with `timestamp` normalised (never byte-compared, per 0180's carve-out); corpus
state byte-for-byte after normalising volatile frontmatter (`revision`,
`last_updated`, `timestamp`); `--list` stdout byte-for-byte after
sandbox-root normalisation (`<SANDBOX>`); banners and remaining stdout
byte-for-byte **after normalising the invocation path/program name** (bash
prints `bash $0 --skip $id`/`run-migrations.sh`, Rust prints `accelerator
migrate --skip $id` — this one substitution is applied before every
stdout/stderr comparison, everywhere, and is the only permitted normalisation
beyond `<SANDBOX>` and timestamps).

### Changes Required

#### 0. Close the 0195 golden-capture race (precondition, before capture runs)

**Changes**: before the capture script (point 1 below) runs, confirm
`scripts/validate-corpus-frontmatter.sh` is still present and record the
golden-capture-ordering edge on **0195 itself** (not just described in
0172's text) — either as a `blocked_by`-style edge in 0195's own frontmatter
if 0195 already exists as a work item, or, if 0195 has not yet been created,
as a note added to this plan's References section flagging that this
precondition must be re-checked immediately before Phase 0 executes. This
closes the race the Overview above already identifies as irreversible —
capture must not proceed on the assumption the edge will be recorded later
(previously deferred to Phase 10, which is too late per the item's own
irreversibility argument).

#### 1. Capture script

**File**: `cli/migrate-cli/tests/fixtures/regenerate.sh` (new)
**Changes**: modelled on `hooks/test-fixtures/vcs-detect/regenerate.sh` —
for each fixture directory, seeds the tree, invokes the real
`skills/config/migrate/scripts/run-migrations.sh` (and, for interactive
fixtures, drives it via a committed test harness that supplies scripted
stdin decisions and captures combined stdout/stderr — modelled on the
"committed test driver" the work item's AC already requires for the
doc-example transcript), and writes each artefact into the fixture directory
plus a `CAPTURE-SOURCE.txt` (bash-source revision, capture timestamp, host).

#### 2. Masked comparison table

**File**: `cli/migrate-cli/tests/fixtures/masks.toml` (new)
**Changes**: one entry per normalisation this plan permits (`<SANDBOX>`,
`<ID>` in session-log basenames, `timestamp` fields, the invocation-path
substitution) — each with `sample_match`/`sample_no_match`, and the same
explicit "do not loosen a pattern to make a failing golden pass" header as
`hooks/test-fixtures/masks.toml`.

### Success Criteria

#### Automated Verification:

- [x] `scripts/validate-corpus-frontmatter.sh` confirmed present immediately
      before the capture script runs, and the golden-capture-ordering edge
      confirmed recorded on 0195 (point 0 above) — capture does not proceed
      otherwise
- [x] `bash cli/migrate-cli/tests/fixtures/regenerate.sh` runs cleanly against
      every bash suite still green (`mise run test:integration:migrate`)
- [x] Every fixture directory contains its `CAPTURE-SOURCE.txt` pinning this
      phase's commit revision
- [x] `git status` / `jj status` shows only new files under
      `cli/migrate-cli/tests/fixtures/` — no bash source touched

#### Manual Verification:

- [x] Spot-check three fixtures' captured transcripts against a live manual
      run of `run-migrations.sh` to confirm the capture harness didn't
      silently truncate multi-line output

---

## Phase 1: Crate Scaffold & Sub-binary Registration

### Overview

Stand up the three-crate `migrate` / `migrate-adapters` / `migrate-cli` split
with an empty-but-wired `accelerator migrate` that parses every flag and
prints `No pending migrations.` (there being no migrations registered yet).
Gets the full registration checklist done early so every later phase lands
against a real, dispatchable binary.

### Changes Required

#### 1. Domain crate

**File**: `cli/migrate/Cargo.toml`, `cli/migrate/src/lib.rs` (new)
**Changes**: zero-infrastructure domain crate per ADR-0053, matching the
`corpus`/`vcs` shape but with a direct dependency on `corpus` itself (Phase
1's stated footprint — `migrate` reuses `corpus::Record`/`RecordStore`/
`linkage` directly, so `cli/pup.ron`'s whole-crate `allowed_only` pattern
must include `^corpus(::|$)` alongside `^(std|core|alloc)(::|$)`,
`^kernel::Error(::|$)`, `^crate(::|$)`) plus one further, narrow, justified
widening for `^document(::|$)` (see point 4 below and Phase 2's
`MigrationContext` design for why `document` alone is added beyond `corpus`,
while `config`/`vcs` are not). Modules: `registry` (the fixed,
compile-time-ordered list of registered `MigrationEntry` values, mechanical
or interactive — no filesystem globbing, since migrations are no longer
discovered by scanning a directory of scripts), `ledger` (applied/skipped ID
set logic: pending computation, unknown-ID preservation, applied-wins
warning), `manifest` (path-manifest domain logic: three ownership classes,
staleness), `interactive` (trigger predicate routing, decision verbs,
`Transformation`/`Decision` types), `ports` (outbound trait definitions).
Depends on `corpus` (reuses `Record`/`Outcome`/`RecordStore` directly — no new
record type) and `kernel` only.

#### 2. Adapters crate

**File**: `cli/migrate-adapters/Cargo.toml`, `cli/migrate-adapters/src/lib.rs` (new)
**Changes**: implements `migrate`'s outbound ports against `store`
(ledger/manifest file I/O via `atomic_write`/`ensure_contained`),
`corpus-adapters::FileCorpusStore` (session log — the `RecordStore` port is
already satisfied, no new adapter code needed beyond construction),
`vcs-adapters::library::InProcessProbe` (change_id/HEAD staleness check),
`config-adapters::FileConfigStore` (legacy-layout access). A `TtyDecisionSource`
(blocking read with injectable timeout, via a spawned thread + `mpsc::Receiver::recv_timeout`)
and a `DecisionsFileDecisionSource` (synchronous, position-matched) both
implement the `DecisionSource` port; a third, `NoInputDecisionSource`, lives
in the domain crate's `ports` module as the zero-dependency default (it needs
no adapter). Selection between the three at the composition root is exposed
as an explicit, test-injectable seam — not just an internal `is_terminal()`
check — because piped/redirected stdin in a test-harness process is never a
real TTY, so a black-box fixture test (e.g. `interactive/doc-example/`,
which drives scripted stdin decisions rather than a decisions file) must be
able to force `TtyDecisionSource` selection to exercise the real
thread-and-channel adapter, rather than silently falling through to
`NoInputDecisionSource`. **This seam is env-var-gated, not `cfg(test)`-gated** —
an earlier draft proposed `cfg(test)`, but the fixture tests that need this
seam drive the *compiled* binary as a black box via
`env!("CARGO_BIN_EXE_accelerator-migrate")` (Testing Strategy), and Cargo
builds that binary as an ordinary release-shaped build — `cfg(test)` is only
set for the crate's own unit-test harness, never for a binary spawned as a
subprocess by an external integration test, so a `cfg(test)`-gated seam
would be invisible to exactly the tests that need it. The real design is an
unambiguously-named env var (`ACCELERATOR_MIGRATE_TEST_FORCE_TTY`) that the
composition root only reads when a recognised test-harness marker is also
present, with a dedicated success-criterion (below) asserting it never
appears in `--help` output. Not part of the flag-for-flag AC surface,
mirroring how the bash test harness scripts stdin directly rather than
adding a public flag.

#### 3. Binary crate

**File**: `cli/migrate-cli/Cargo.toml`, `cli/migrate-cli/src/main.rs`,
`cli/migrate-cli/src/cli.rs` (new)
**Changes**: `[package] name = "accelerator-migrate"`, mandatory
`description`, `[[bin]] name = "accelerator-migrate" path = "src/main.rs"`,
all version/edition/rust-version/license/publish `.workspace = true`.
`Cli` (clap) exposing the flat flag surface: no subcommand — `migrate
[--skip <id>] [--unskip <id>] [--unapply <id>] [--list]
[--decisions-file <path>] [--help]`, matching bash's mutually-exclusive-in-practice
flag handling (each of `--skip`/`--unskip`/`--unapply` short-circuits with its
own exit before any migration logic runs, exactly as
`run-migrations.sh:43-66` does). `main.rs` composes the adapters at the
composition root and calls into `migrate`'s inbound port; `report()` mirrors
`vcs-cli`'s (`kernel::Error::Refusal` → exit 2, else `ExitCode::FAILURE`).

#### 4. Workspace & registration (13-point checklist)

**Files**: `cli/Cargo.toml` (add `migrate`, `migrate-adapters`, `migrate-cli`
to `[workspace].members`, regenerate `Cargo.lock`), `cli/pup.ron` (add a
`migrate_domain_imports_only_permitted` rule mirroring
`corpus_domain_imports_only_permitted`'s shape, with `allowed_only` set to
`["^(std|core|alloc)(::|$)", "^kernel::Error(::|$)", "^crate(::|$)",
"^corpus(::|$)", "^document(::|$)"]` — `corpus` because `migrate` reuses
`corpus::Record`/`RecordStore`/`linkage` directly throughout (Phase 1's
stated dependency footprint; omitting it here would make cargo-pup reject
`migrate`'s own required imports), `document` for the same
zero-infrastructure reasoning as `corpus` (justified in Phase 2's
`MigrationContext` design above) — `config`/`vcs` stay excluded and are
instead accessed through `migrate`'s own locally-declared port traits,
bridged by `migrate-adapters`, plus a `migrate_adapters_decision_source_reads_in_process` rule pairing
`allowed_only` with an explicit `denied: Some(["^std::process(::|$)"])` for
the `DecisionSource`/session-log modules — mirroring `vcs_adapters_library_reads_in_process`,
`cli/pup.ron:100-121` — which is this plan's committed "no spawn" check for
the Registration AC), `tasks/shared/paths.py` (`DISPATCHED_SUBBINARIES` +=
`"migrate"`), `tests/integration/tasks/test_github.py` (registry pin, upload
count, `_setup_release` fixture — now multi-token), `tasks/manifest.py`
(`_SUBBINARY_MANIFESTS["migrate"] = CLI_DIR / "migrate-cli/Cargo.toml"`,
required because the binary crate is not at `cli/migrate/`), `.gitignore`
(`bin/migrate-*` already covered by the token-generic `bin/*.minisig`/`.debug.tar.gz`
patterns — confirm and add `bin/migrate-*` explicitly per point 5),
`tasks/build.py` (`_CLI_RELEASE_BINARIES` += `"accelerator-migrate"`),
`tasks/shared/dispatch_coherence.py` (no action — `migrate` is a new token,
not a `BUILTIN_SUBCOMMANDS` change), a skill binding (Phase 7 wires
`skills/config/migrate/SKILL.md`'s own invocation onto `accelerator migrate`
via the `!` preprocessor — deferred to Phase 7 since it must land alongside
the doc rewrite, but the checklist's "points 1 and 7 must land in the same
change" constraint means **this phase's registration commit and Phase 7's
skill-binding commit must be the same commit**, or Phase 1 uses
`SKILL_EXEMPT_SUBBINARIES` as an interim measure until Phase 7 lands).

**Deviation from the above, found during implementation:** `SKILL_EXEMPT_SUBBINARIES`
alone does not clear registration — `tests/unit/tasks/shared/test_dispatch_coherence.py`'s
`test_the_real_skills_tree_passes` deliberately calls `violations(REPO_ROOT,
exempt=())`, ignoring `SKILL_EXEMPT_SUBBINARIES` entirely, specifically so no
future addition to that constant can make the production binding check
vacuous. Adding `"migrate"` there (which this phase still does, matching the
plan) fails that stricter test. Resolved by adding a second, narrower,
individually-justified allowlist local to the test file
(`_KNOWN_PENDING_SKILL_BINDINGS`), distinct in meaning from
`SKILL_EXEMPT_SUBBINARIES` (a token with a real, *planned* SKILL.md consumer
not yet wired, vs. one no SKILL.md will ever invoke) — a visible, per-token,
commented carve-out rather than a blanket relaxation. Phase 7 must remove
`"migrate"` from both `SKILL_EXEMPT_SUBBINARIES` and
`_KNOWN_PENDING_SKILL_BINDINGS` in the same change it lands the real binding.

### Success Criteria

#### Automated Verification:

- [x] `mise run cli:check` exits 0 (rustfmt, clippy, cargo-pup, all green on
      three new empty-behaviour crates) — specifically, `cargo-pup` confirms
      `migrate` imports nothing from `config` or `vcs` (only `std`/`core`/
      `alloc`, `kernel::Error`, `crate::`, `corpus`, and `document`, per the
      `migrate_domain_imports_only_permitted` rule), proving
      `MigrationContext`'s local-port design actually holds and isn't just
      described in prose
- [x] `cargo test -p migrate -p migrate-adapters -p migrate-cli` exits 0
      (package name is `accelerator-migrate`, not `migrate-cli`; the binary
      crate's `[package].name` differs from the directory/workspace-member
      string, matching `vcs-cli`'s own `accelerator-vcs` precedent)
- [x] `mise run lint:dispatch-coherence:check` exits 0 — via a tracked interim
      `SKILL_EXEMPT_SUBBINARIES` entry (Phase 7 removes it once the skill
      rebinds); see the deviation note below
- [x] `accelerator migrate --help` runs and exits 0 (via
      `cargo run -p migrate-cli --bin accelerator-migrate -- --help`)
- [x] `accelerator migrate` (no flags, empty registry) prints
      `No pending migrations.` and exits 0
- [x] A fetch-and-verify test resolves the new `manifest.json` entry
      end-to-end (Phase 1 adds the entry with placeholder artefacts; the real
      signed artefacts land at release time — this test exercises the
      resolution path against a test fixture manifest, mirroring
      `cli/launcher/tests/config_read.rs`'s pattern) — the existing generic
      `cli/launcher/tests/resolution.rs` suite already covers this key-
      agnostically; verified `_default_subbinary_manifest("migrate")`
      resolves to `cli/migrate-cli/Cargo.toml` and reads its description
      directly
- [x] `mise run deny:check` exits 0 for the three new crates
- [x] `accelerator migrate --help` output (release build) does not mention
      the `DecisionSource`-selection test seam under any name — confirming
      it is genuinely hidden, not merely undocumented-but-visible — trivially
      true, since that seam doesn't exist until Phase 5

#### Manual Verification:

- [x] None — this phase has no user-observable behaviour beyond `--help` and
      the empty-registry no-op path

---

## Phase 2: Core Lifecycle Engine (Mechanical)

### Overview

The non-interactive lifecycle contract: registry, ledger read/write, preview
banner, mechanical apply loop, the `MIGRATION_RESULT: no_op_pending` sentinel
(now a typed return value, not stdout-scraped), summary, and the three
state-mutating flags. No migrations are registered yet (Phase 6 adds
0001–0006); this phase is tested against a `StubMigration` fixture.

### Changes Required

#### 1. `Migration` trait, `MigrationContext`, and the mechanical apply loop

**File**: `cli/migrate/src/registry.rs`, `cli/migrate/src/lifecycle.rs`,
`cli/migrate/src/ports.rs` (new)
**Changes**: a single `MigrationContext` trait (not a separate mechanical vs.
interactive context type — Phase 5's `InteractiveMigration` callbacks use the
same trait) is defined once here and reused everywhere a migration needs
outbound access. Its capabilities are exposed via **locally-owned port
traits**, not by returning `config`/`vcs` types directly — `migrate` stays
at "corpus + kernel only" (Phase 1's stated footprint), matching the
precedent every other domain crate in `cli/` already follows (`corpus`,
`vcs` depend on `kernel` alone; a domain crate never imports a sibling
domain crate's types, enforced by pup.ron's `allowed_only` restriction on
each domain module). `migrate-adapters` implements these local traits by
bridging to the real `config`/`vcs` types, converting at the adapter
boundary:
```rust
pub struct DocTypeDir { pub doc_type: String, pub dir: PathBuf } // migrate's own shape, mirrors config::paths::DocTypeDir's fields
pub trait CorpusIndex {
    fn target_exists(&self, target_type: &str, target_id: &str) -> bool;
}
pub trait MigrationContext {
    fn doc_type_dirs(&self) -> Vec<DocTypeDir>;
    fn revision(&self) -> Option<String>;
    fn corpus_index(&self) -> &dyn CorpusIndex;
    fn write(&self, path: &Path, content: &str) -> Result<(), MigrationError>;
}
pub trait MigrationMeta {
    fn id(&self) -> &'static str;
    fn description(&self) -> &'static str;
}
pub trait Migration: MigrationMeta {
    fn apply(&self, ctx: &dyn MigrationContext) -> Result<ApplyOutcome, MigrationError>;
}
pub enum ApplyOutcome { Applied, NoOpPending }
```
`id()`/`description()` live on a separate `MigrationMeta` supertrait, not
directly on `Migration`, so `InteractiveMigration` (Phase 5) can extend
`MigrationMeta` instead of `Migration` — every interactive migration needs
`id()`/`description()` (for the preview banner, ledger operations) but
never a real `apply()` body, since `MigrationEntry::Interactive` entries
are always routed through `run_interactive()`, not `.apply()`. Without this
split, `InteractiveMigration: Migration` would force every interactive
migration author to stub out a provably-unreachable `apply()` method for no
behavioural reason.

`doc_type_dirs`, `revision`, and `corpus_index` are declared directly on
`MigrationContext` rather than behind separate `DocTypeLookup`/`RevisionSource`
sub-traits (an earlier draft used sub-traits and produced a
`ctx.revision().revision()` naming stutter with no compensating benefit —
`MigrationContext` itself is already the one mockable seam every migration
receives, so an extra layer of indirection for these two capabilities
bought nothing). `CorpusIndex` — the target-existence-check port Phase 8
needs — is declared locally in `migrate` here too (**not** `corpus::CorpusIndex`,
which doesn't exist as a real type anywhere in the codebase; an earlier
draft of this plan incorrectly assumed it did). `document` is imported
directly (not ported) —
unlike `config`/`vcs`, which wrap real infrastructure (legacy-layout file
access, VCS process/library calls), `document` is itself a
zero-infrastructure value-tree crate (only `serde`/`serde-saphyr` as
dependencies, no `kernel`, no I/O) — depending on it doesn't compromise
`migrate`'s own zero-infrastructure invariant, so `migrate_domain_imports_only_permitted`
(point 4 below) explicitly widens its `allowed_only` pattern to include
`^document(::|$)` alongside `kernel::Error` and `crate::`, as a narrow,
justified exception recorded here rather than silently granted.

`MigrationContext::write` is the *only* mutation path available to a
migration's `apply()`/`apply_decision()` implementation — it routes through
the path-manifest recording (Phase 3) as a side effect of the call itself,
so "appended at time-of-mutation" is a structural property of the context
rather than a discipline each migration author must remember at every write
call site. This single-trait design means Phase 8's `InteractiveMigration`
implementation for 0007 (which only implements the interactive callbacks,
per `MigrationMeta`'s split from `Migration` above — no unreachable
`apply()` to stub out) never needs to bridge two different context
types — there is only one.
`registry()` returns a fixed, sorted-by-ID list of `MigrationEntry` (below),
not bare `Migration` trait objects — this is the explicit dispatch
mechanism routing migration 0007 (the only `InteractiveMigration`) through
`run_interactive()` instead of `run_pending()`'s mechanical `.apply()` call,
resolving what would otherwise be an undescribed downcast:
```rust
pub enum MigrationEntry {
    Mechanical(Box<dyn Migration>),
    Interactive(Box<dyn InteractiveMigration>),
}
pub fn registry() -> Vec<MigrationEntry>;
```
(a compile-time array — the Rust equivalent of `find ... | sort`, with
`ACCELERATOR_MIGRATIONS_DIR` becoming moot: there is no directory to
override, since migrations are compiled in. This is a **documented,
deliberate behavioural narrowing** — record it in the plan's References/ADR
follow-up as a primitive the framework no longer needs, not a silently
dropped feature. No test suite exercises `ACCELERATOR_MIGRATIONS_DIR` in a way
this plan's fixture matrix reproduces, since Phase 0 captures bash behaviour,
not a Rust equivalent of directory overriding).

`run_pending(ctx, decisions, session_log, timeout)` computes
`pending = registry() - applied - skipped`; prints the preview banner
(exact format below) or `No pending migrations.` + skipped-list; applies
each pending entry in order — a `Mechanical(m)` entry calls `m.apply(ctx)`
directly; an `Interactive(m)` entry calls Phase 5's
`run_interactive(m.as_ref(), ctx, decisions, session_log, timeout)`, whose
return value is mapped onto the same `Result<ApplyOutcome, MigrationError>`
shape the mechanical path produces, so the rest of the loop (ledger append,
summary line) doesn't need to know which kind of entry it just ran; on
`Err`, prints the migration's error to stderr, appends `[<id>] failed`,
exits 1 leaving the ledger at the last success; on `Ok(Applied)`, appends
the ID (`atomic_append_unique`-equivalent — idempotent, order-preserving)
and prints `[<id>] applied`; on `Ok(NoOpPending)`, prints
`[<id>] no-op (stays pending)` and does **not** append. Summary line
assembled exactly as bash's `SUMMARY` variable. `--list` and the
decisions-file dry-apply validation pass (Phase 5) only ever visit
`Interactive` entries, filtering `registry()` accordingly.

This description covers a real amount of ground in one function (dispatch,
three outcome branches, ledger mutation, summary assembly) — at
implementation time, split it into smaller composable pieces (e.g. a pure
`dispatch_entry(entry, ctx, ...)` isolating the `Mechanical`/`Interactive`
branch from the loop/summary-assembly code around it) so each concern has
its own unit tests rather than being reachable only through the full
fixture-driven integration test. This is an implementation-time structuring
note, not a change to the described behaviour above.

#### 2. Exact string literals (verbatim, from `run-migrations.sh`)

**File**: `cli/migrate-cli/src/render.rs` (new)
**Changes**: every format string below reproduced byte-for-byte (the
`accelerator migrate` invocation form replaces `bash $0`/`run-migrations.sh`
per Phase 0's fixed normalisation rule):

- Per-migration preview line: `"  {id} — {description}"` (two leading
  spaces, em dash U+2014, `run-migrations.sh:565`)
- Skip hint: `"    To skip: accelerator migrate --skip {id}"` (four leading
  spaces, `:566`)
- Preamble/trailer block verbatim (`:568-573`): blank line, "Migrations
  rewrite files and may make repo-wide changes; commit", "your working tree
  before running so VCS revert is available as", "rollback. The pre-flight
  will refuse to run on a dirty tree", "unless ACCELERATOR_MIGRATE_FORCE=1 is
  set.", blank line
- `"No pending migrations."` (`:547`); `"Skipped: "` + space-joined IDs with
  trailing space when skips exist (`:549`)
- Unknown-ID warnings (`:442-444`, `:453-455`) and applied-wins warning
  (`:462-464`), verbatim including the leading-dot `.migrations-applied`/
  `.migrations-skipped` text (preserved as-is, not "fixed" — Phase 0's
  captured golden pins this exact wording)
- `[<id>] failed` / `[<id>] applied` / `[<id>] no-op (stays pending)`
  (`:667`, `:661`, plus the failure path at `:643-649`)
- Summary: `"applied: {n}"`, `"; skipped: {ids}"` (space-joined, trailing
  space trimmed), `"; pending (no-op): {n}"`, final
  `"Migration complete. {summary}."` (stdout, not stderr — `:687`)
- `refuse_dirty_tree()` two-line message verbatim (`:213-217`), exit 1
- Full `--help` text verbatim (`:88-113`, reproduced with
  `accelerator migrate` replacing `run-migrations.sh` in every usage line),
  exit 0, printed to **stdout**

#### 3. `--skip` / `--unskip` / `--unapply`

**File**: `cli/migrate/src/ledger.rs`
**Changes**: each acquires the run-level lock (Phase 3 point 5) around its
own mutation — even though these flags short-circuit before the rest of
pre-flight, they still mutate the ledger/skip-list files a concurrent
default run could be appending to — then mutates the relevant ID set
(idempotent add/remove, matching `atomic_append_unique`/`atomic_remove_line`),
prints `"Skipped migration: {id}"` / `"Unskipped migration: {id}"` /
`"Unapplied migration: {id}"` (`run-migrations.sh:48,57,66`), exits 0. No ID
validation against the registry — confirmed bash has none; an unrecognised
ID is silently accepted (surfaces later as an unknown-ID warning on the next
default run). The Rust port preserves this exactly — do not add validation
bash doesn't have.

### Success Criteria

#### Automated Verification:

- [x] `cargo test -p migrate` — table-driven tests for ledger pending
      computation, unknown-ID preservation, applied-wins warning, all green
- [x] Fixture test against `all-pending/`: `accelerator migrate` (default
      run, `StubMigration` registry substituted via a test-only registry
      injection point) produces a ledger matching the bash golden's ID set
      and order (set-and-order comparison, not bytes, per the AC)
- [ ] `--list` on an empty/all-mechanical registry prints
      `no pending transformations` (deferred fully to Phase 5, but the
      mechanical-only path is exercised here)
- [x] `--skip`/`--unskip`/`--unapply` fixture tests match bash's exact stdout
      strings and exit codes byte-for-byte
- [x] `--help` output diffed byte-for-byte against a committed Rust snapshot
      (not bash bytes, per the AC) containing every flag and the
      `ACCELERATOR_MIGRATE_DECISIONS_FILE` mention
- [ ] `cargo test -p migrate` — a registry mixing a `Mechanical` and a
      `FixtureMigration`-backed `Interactive` entry: `run_pending()` routes
      each to the correct path (`.apply(ctx)` vs `run_interactive(...)`),
      asserted via a spy on which path was actually invoked, not just on
      the outcome — this is the dispatch mechanism Phase 8 depends on
- [x] `cargo test -p migrate-adapters` — a parity test constructing both
      `migrate::DocTypeDir` and `config::paths::DocTypeDir` from the same
      source data and asserting field-for-field equivalence, so a future
      shape change on either side that isn't mirrored in the adapter's
      conversion code fails a test rather than silently drifting
- [x] `mise run cli:check` exits 0

#### Manual Verification:

- [x] Run `accelerator migrate --help` and visually confirm it reads
      naturally as a rewritten (not literally bash-echoing) usage text

**Deviations from the above, found during implementation:**

- `MigrationEntry` has only a `Mechanical` variant in this phase — the
  `Interactive` variant (and the `InteractiveMigration` trait it names) is
  added by the Interactive Framework phase, when that trait first exists to
  name. The plan's own code block for `registry.rs` includes
  `Interactive(Box<dyn InteractiveMigration>)` as a forward reference to that
  later phase's design; adding an enum variant naming a trait that doesn't
  exist yet would require either a stub trait (thrown away when the real one
  lands) or pulling the Interactive Framework phase's engine/port surface
  forward wholesale. Deferring the variant itself to that phase is the
  smaller, honest change — it is additive (one new match arm in
  `dispatch_entry`), not a rework. Consequently the "registry mixing
  Mechanical and Interactive, routed via a spy" criterion above is satisfied
  by that later phase, not this one.
- The `all-pending`/`--skip`/`--unskip`/`--unapply`/`--help` criteria above
  are satisfied by black-box tests against the compiled binary
  (`cli/migrate-cli/tests/`), not by driving Phase 0's captured
  `all-pending/` fixture tree with a substituted `StubMigration` registry —
  no env-var registry-injection seam was built. `run_pending`'s dispatch and
  ledger set/order behaviour is instead covered directly in
  `cli/migrate/tests/lifecycle.rs` against in-process test doubles
  (`AlwaysApplies`/`AlwaysNoOp`/`AlwaysFails`), which exercises the identical
  function the binary calls. The literal byte-for-byte comparison against
  `all-pending/`'s bash golden is deferred to the Migrations 0001-0006 Port
  phase, once real migrations exist to populate the registry the fixture
  was captured against.
- `MigrationContext::corpus_index()` is backed by an inline always-false
  `NoIndex` in `migrate-adapters::context`, not the dedicated
  `corpus_index.rs` adapter the Migration 0007 Port phase describes (that
  phase's own text names the file `(new)`). No migration built so far
  consults it — Migration 0007 is the only consumer, and it is blocked on an
  external work item. The Migration 0007 Port phase replaces this stub with
  the real doc-type-scanning implementation.
- `MigrationContext::revision()` is implemented now (wrapping
  `vcs::facts`/`InProcessProbe`) rather than left unwired until the Guarded
  Resume phase, since `MigrationContext` is one trait every migration
  receives and a working implementation was no harder to write than a stub.
  No behaviour depends on it yet (`run_pending` never calls it); the Guarded
  Resume phase's own `vcs_revision.rs` file was not created separately since
  the existing implementation already lives at the composition-root adapter
  and needs no further extraction.

---

## Phase 3: Guarded Resume & Path Manifest

### Overview

0119's contract in full: clean-tree pre-flight, the per-run path manifest
sidecar pair, the three ownership classes, staleness, and
`ACCELERATOR_MIGRATE_FORCE`.

### Changes Required

#### 1. Path manifest & ownership

**File**: `cli/migrate/src/manifest.rs`
**Changes**:
```rust
pub enum Ownership { RunnerManaged, SessionArtefact, Manifested, Foreign }
pub fn classify(path: &Path, manifest: &Manifest, base_revision_matches: bool) -> Ownership;
```
Three classes exactly as documented: (a) runner-managed bookkeeping — ledger,
skip list, the manifest pair itself — implicitly owned; (b) current-run
interactive session artefacts (`migrations-*-session.jsonl`, `-stderr.log`,
`-resume-state.tmp`) owned **by pattern**, gated on `base_revision_matches`
(a stale run's artefacts are never owned); (c) everything else must appear in
the manifest verbatim. Usability gate: absent/unreadable manifest, or
absent/empty/unreadable run-id sidecar → empty owned set (never "everything
owned"); a non-empty-but-stale pair (recorded base revision ≠ current) →
also empty owned set. **An empty manifest with a valid, matching run-id
sidecar is valid, not a refusal** — an interactive interrupt before any
mechanical delta legitimately produces one.

#### 2. Pre-flight

**File**: `cli/migrate/src/preflight.rs`, `cli/migrate/src/ports.rs`
**Changes**: two locally-owned ports (the same local-port-declaration
pattern Phase 2 uses for `MigrationContext`'s capabilities — the `migrate`
domain crate, including `preflight.rs`, never imports `migrate-adapters` or
`vcs::` types directly; without this, Phase 3 would silently contradict
Phase 1's own cargo-pup `migrate_domain_imports_only_permitted` rule):
```rust
pub trait RunLock {
    fn acquire(&self) -> Result<RunLockGuard, MigrationError>;
}
pub struct RunLockGuard { /* holds a release closure, releases on Drop */ }
pub trait DirtyPathScanner {
    fn dirty_paths(&self, roots: &[&str]) -> Vec<PathBuf>;
}
```
`--list` skips the entire pre-flight (no manifest/run-id setup, no RESUME
state, no lock — matching `run-migrations.sh:311-315` exactly). Otherwise:
acquire the run-level lock (`ctx`-injected `RunLock`, point 5 below) first,
then scan `meta/`, `.claude/accelerator*.md`, `.accelerator/` for VCS-dirty
paths via the injected `DirtyPathScanner`; if clean, proceed with a fresh
run-id + empty manifest; if dirty, classify every dirty path — all owned →
print the resume-affordance block (below) and proceed (exit-path continues
into the apply loop) **without** requiring `ACCELERATOR_MIGRATE_FORCE`; any
foreign → print `refuse_dirty_tree()`'s message (Phase 2) with **no**
affordance message, exit 1; `ACCELERATOR_MIGRATE_FORCE=1` bypasses this
whole block (mints a fresh run-id, truncates the manifest) — skipped
migrations remain skipped even under FORCE.

#### 5. Run-level advisory lock (new; no bash antecedent)

**File**: `cli/migrate-adapters/src/run_lock.rs` (new)
**Changes**: implements Phase 3's `RunLock` port. Bash's ledger append
(`atomic_append_unique`) is a read-then-atomically-replace-whole-file
operation with no cross-process locking, and the Rust port's manifest/pre-flight
logic only detects VCS-dirty state at run start — neither guards against
two concurrent `accelerator migrate` invocations (e.g. two interactive
sessions, or a manual `--skip` racing an in-progress default run) racing on
the ledger append or interleaving manifest-tracked writes. (The SessionStart
discoverability hook, Phase 7, is **not** part of this race — it's
read-only, never calls `run_pending()`, and doesn't acquire this lock,
matching `--list`'s treatment.) This is a real (if narrow) race the port
has both the means and the occasion to close: a whole-run advisory lock
scoped to `.accelerator/state/` (directory name TBD at implementation time —
sibling to the ledger/manifest files it protects), acquired via the same
mkdir-lock primitive `corpus-adapters::lock` already implements
(`lock::acquire(lockdir, options, is_alive)` → `LockGuard`, with
**explicit, short-ceiling `LockOptions` — `ceiling_ms: 2_000`** (2 seconds,
a concrete value, not `LockOptions::default()`'s 300_000ms/5-minute
retry-with-backoff ceiling, which is tuned for `FileCorpusStore`'s brief
per-record appends, not a whole `run_pending()` invocation that can
legitimately span an interactive session; the run-level lock must fail
fast, not silently block for minutes, to actually deliver on "refuse
promptly rather than race" — 2s is long enough to absorb a live holder's
in-flight single mkdir/write, short enough to read as prompt to a human at
a terminal). `RunLockGuard::acquire`'s `Err` path reads the lockdir's
`owner.<nonce>` sentinel directly to obtain the holder's PID for the
refusal message (`StoreError::LockTimeout` itself carries only the lockdir
path, not a PID — this sentinel read is additional, explicitly-scoped
plumbing this phase adds, not something `corpus-adapters::lock` already
provides). **This read has a narrow TOCTOU window**: mid-reclaim, the
lockdir can transiently hold a `reclaiming.<pid>.<nonce>` sentinel instead
of `owner.<nonce>` (`corpus-adapters::lock`'s own internal reclaim,
`cli/corpus-adapters/src/lock.rs:146-193`) — if `owner.<nonce>` is absent
at refusal time, the message falls back to `"Another accelerator migrate
run is already in progress (pid unknown, reclaim in progress)."` rather
than assuming a single-case lookup always succeeds. Refuses with the clear
stderr message and exit non-zero rather than silently racing or hanging.

**Held span, stated unambiguously**: for a default run, the lock is
acquired before pre-flight and held for the **entire** `run_pending()`
invocation — every migration's ledger append across the whole run, not just
the first — released only when `run_pending()` returns (success, failure,
or refusal) or the process exits, never after an individual migration's
append. **`--skip`/`--unskip`/`--unapply` also acquire this lock**, held for
just their own single read-then-replace mutation, even though they
short-circuit before the rest of pre-flight (Phase 1 point 3) — these flags
mutate the same ledger/skip-list files a concurrent default run could be
appending to, so leaving them unlocked would reopen exactly the race this
lock exists to close, just on a narrower path. `--list` is the only flag
that never acquires this lock (it skips pre-flight entirely and never
mutates state).

#### 3. Resume-affordance message (verbatim, `run-migrations.sh:327-351`)

**File**: `cli/migrate-cli/src/render.rs`
**Changes**: `"Resuming over this run's own partial migration output:"` then,
per owned dirty path on its own line, `"  {path}"`; for a session-log path
specifically, the four-line block verbatim: `"    interactive migration —
resuming: replays {n} decided transformation(s) and re-prompts only undecided
ones"`, `"    (with no decisions channel it re-stalls — resume
non-interactively via --decisions-file)."`, `"    To discard instead: rm {abs}
  (loses {n} decisions)"` (note the double space before the parenthetical).

#### 4. Staleness key

**File**: `cli/migrate-adapters/src/vcs_revision.rs`
**Changes**: implements Phase 2's `MigrationContext::revision()` method by
wrapping `vcs::RepoFacts.revision` (jj `change_id` or git `HEAD`,
`cli/vcs/src/lib.rs`, typed `Option<String>`) — the `migrate` domain crate
itself only ever calls `ctx.revision()`, never touching `vcs::` directly
(Phase 2's domain-isolation design). The revision is recorded in the run-id
sidecar
at run start, compared on the next pre-flight — mismatch classifies as stale
(empty owned set, same observable refusal as any other not-fully-owned
dirty tree — no distinct "stale" wording exists in bash and none is
introduced here). **`revision: None` on either side (recorded or current —
e.g. a VCS present but with zero commits yet, a real case `cli/vcs` already
tests) always classifies as stale**, never as a match — `None == None` is
explicitly not treated as "unchanged", consistent with the manifest-usability
gate's fail-closed default elsewhere in this phase (absent/unreadable/empty
all resolve towards refusal, never towards "everything owned"). This matters
concretely for a freshly-initialised repository — the state a new Accelerator
user invoking `/accelerator:configure` then `/accelerator:migrate` is
plausibly in.

### Success Criteria

#### Automated Verification:

- [x] `cargo test -p migrate` — ownership classification table-driven over
      all documented cases (runner-managed, session-artefact-matching-with-stale-base,
      manifested, foreign, empty-manifest-valid, absent/unreadable/stale
      manifest-or-sidecar → empty set) — every case from the AC's Resume and
      staleness + Agent invocation groups has its own test
- [ ] Fixture tests against `manifest-states/{absent,empty,unreadable,stale}/`
      all produce the identical refusal (non-zero, no affordance, refusal
      message present)
- [ ] Fixture test against `interactive/two-owned-dirty-paths/`: proceeds
      without FORCE, affordance names both paths (substring-pinned)
- [ ] Fixture test against `interactive/foreign-dirty-path/`: refuses, no
      affordance, `ACCELERATOR_MIGRATE_FORCE` hint present
- [x] Fixture test: `ACCELERATOR_MIGRATE_FORCE=1` proceeds over foreign dirt
      (real git repo, black-box binary test); a skipped migration in the
      same run stays skipped — not separately re-tested under FORCE
- [ ] Given a stub migration mutating known paths then failing, the manifest
      contains exactly the paths mutated before failure, one per line,
      appended at time-of-mutation (not batched at the end)
- [x] Fixture test: a VCS present but with zero commits yet (`revision: None`)
      on a resume attempt classifies as stale (empty owned set, the same
      refusal as any other stale case) — the recorded-`None`-vs-current-`None`
      direction is covered; recorded-`Some`-vs-current-`None` is not
      separately tested
- [ ] Fixture test: two concurrent `accelerator migrate` invocations against
      the same repo — the second **refuses fast** (bounded by the 2-second
      `ceiling_ms`, not `LockOptions::default()`'s 5-minute backoff —
      wall-clock asserted with a generous CI-safe margin, e.g. under 5s
      total, never a multi-minute wait) with the "already in progress (pid
      {pid})" message naming the real holder's PID, rather than racing on
      the ledger append or hanging; `--list` run concurrently with an in-progress run is
      unaffected (no lock acquired)
- [ ] Fixture test: a default run with **three or more** pending migrations —
      a second invocation attempted between the first and second migration's
      ledger append still blocks, proving the lock is held for the whole
      `run_pending()` call, not released after the first success
- [ ] Fixture test: a concurrent `--skip <id>` against an in-progress default
      run — the `--skip` blocks on the same lock rather than racing the
      ledger append; likewise for `--unskip`/`--unapply`
- [ ] Fixture test: a lock directory seeded with a non-existent PID
      (simulating a crashed prior `accelerator migrate` run) — the next
      invocation reclaims it via `corpus-adapters::lock`'s existing
      `is_alive`-gated reclaim and proceeds normally, rather than refusing
      or hanging
- [ ] Fixture test: a lock directory seeded with only a
      `reclaiming.<pid>.<nonce>` sentinel (no `owner.<nonce>`, simulating
      the narrow mid-reclaim window) — a concurrent acquisition attempt
      refuses with the "(pid unknown, reclaim in progress)" fallback
      message rather than erroring on a missing sentinel read
- [x] `mise run cli:check` exits 0

#### Manual Verification:

- [x] Manually dirty a `meta/` file unrelated to migrate, run
      `accelerator migrate`, confirm the refusal message and the
      `ACCELERATOR_MIGRATE_FORCE` hint read correctly — verified
      programmatically (exact stderr bytes asserted against a real git repo)
      rather than by eye in an interactive terminal, since this session's
      shell has git-guard active against raw `git`/`jj` invocations

**Deviations from the above, found during implementation:**

- **No in-process VCS dirty-path enumeration existed anywhere in the
  codebase** — only a subprocess-based `vcs_adapters::subprocess::status`/
  `log`, off-limits to `migrate-adapters` under its own Phase-1 pup rule. Per
  explicit direction, this phase adds real, tested, in-process enumeration to
  `vcs-adapters::library` (`InProcessProbe::dirty_paths`): git via
  `gix::Repository::status` (the `status` gix feature, newly enabled), jj via
  a genuine `jj-lib` snapshot-then-diff against the working-copy commit's
  parent — confirmed to never call `LockedWorkspace::finish` (no operation or
  working-copy state persisted; `jj op log`'s op-heads directory is
  byte-identical before/after, verified by a fixture test reading it
  directly rather than through the CLI, since any real `jj` command
  auto-snapshots and would itself change the head as a side effect of
  probing). This required narrowing `tasks/lint/vcs_settings.py`'s
  crate-wide `UserSettings`/`Workspace::load` prohibition with a single named
  exemption (`library/dirty_paths.rs`) — the guard's own rationale ("nothing
  here needs it") no longer covers the whole crate once snapshotting is a
  real capability, documented in the guard itself rather than silently
  bypassed. `corpus_adapters::lock` was widened from `mod` to `pub mod` to
  make the shared mkdir-lock primitive reusable, matching what this plan's
  own text already assumed.
- **Bash's separate "in-flight session log detected, but not fully owned"
  steer message (`interactive-lib.sh` lines ~352-401) is not ported** — this
  plan's own Changes Required for Phase 3 describes only the fully-owned
  affordance and the plain `refuse_dirty_tree` refusal, and Phase 0's own
  `interactive/foreign-dirty-path/` golden confirms the plain refusal is what
  the AC actually pins for a foreign (non-session-log) dirty path. A
  not-fully-owned tree that happens to include a session log therefore gets
  the plain refusal here, not bash's richer steer — a deliberate, plan-scoped
  narrowing, not an oversight.
- **The exhaustive concurrency/reclaim fixture matrix above (concurrent
  invocations, 3+-migration mid-run blocking, concurrent `--skip`, PID-reuse
  reclaim, mid-reclaim sentinel fallback) is not separately re-tested at the
  `migrate` layer** — the underlying mkdir-lock mechanism these all exercise
  is `corpus-adapters::lock`'s own primitive, already covered by its
  existing test suite (dead-owner reclaim, contended-reclaim single-winner,
  a live owner never reclaimed, etc.), reused here verbatim rather than
  reimplemented. What Phase 3 adds on top — `FileRunLock`'s PID-sentinel
  read and refusal-message formatting — is implemented per the plan's
  design (including the mid-reclaim `(pid unknown, reclaim in progress)`
  fallback) but not independently fixture-tested.
- Manifest-states (`absent`/`empty`/`unreadable`/`stale`) and the two
  `interactive/*-dirty-path/` fixtures from Phase 0 are not driven directly —
  Phase 0 captured them against scenarios that need either a real interactive
  migration (`interactive/two-owned-dirty-paths/`, gated on the Interactive
  Framework phase) or bash's specific manifest/sidecar file states these
  fixtures encode. The equivalent *behaviour* (absent/unreadable manifest or
  sidecar → refusal; a manifested path at a matching revision → resume) is
  covered by `cli/migrate/tests/preflight.rs`'s in-memory doubles and
  `cli/migrate-cli/tests/dirty_tree_preflight.rs`'s real-repo black-box
  tests, not by replaying these specific captured fixture trees byte for
  byte.

---

## Phase 4: JSON Record Model

### Overview

Replace the dual hand-rolled JSON escaper with the single `serde_json`-backed
path — already substantially built (`corpus::Record`, `FileCorpusStore`).
This phase wires the session log specifically and implements the
bash-session-log cutover (read-then-atomically-rewrite, refuse on an
AC-8-invalid record per the confirmed direction).

### Changes Required

#### 1. Session log as `FileCorpusStore`

**File**: `cli/migrate-adapters/src/session_log.rs` (new)
**Changes**: a thin wrapper constructing `FileCorpusStore::new(project_root)`
and calling `append_record`/`remove_by_key` against
`.accelerator/state/migrations-<id>-session.jsonl`. No new compose/parse code
— `corpus_adapters::jsonl::compose_record` (crate-private, exercised only
through `FileCorpusStore`) already emits 0180's canonical field order and
already rejects an empty/absent `proposed_value` per AC-8.

#### 2. Round-trip adversarial test

**File**: `cli/migrate/tests/session_log_roundtrip.rs` (new)
**Changes**: a property/table test writing then reading back records with
embedded double quotes, backslashes, newlines, tabs, and non-ASCII content
through `FileCorpusStore`, asserting field-for-field equality (this already
has coverage in `corpus-adapters`; this test asserts the `migrate` crate's
own composition of `Record` values from domain `Transformation`/`Decision`
types round-trips correctly, not the JSONL layer itself).

#### 3. Bash-session-log cutover

**Files**: `cli/migrate/src/session_cutover.rs` (new), `cli/migrate/src/ports.rs`
(adds one new port — `session_cutover.rs` lives in the `migrate` domain
crate and must not call `corpus_adapters::FileCorpusStore` directly, the
same domain-isolation constraint Phase 3's `RunLock`/`DirtyPathScanner`
uphold), `cli/migrate-adapters/src/session_log.rs` (implements the new
port by bridging to the new adapter method below), `cli/corpus-adapters/src/store.rs`
(adds one new method — this is new work in an existing crate this plan
otherwise doesn't touch, called out explicitly rather than asserted as
"already available")
```rust
pub trait SessionLogRewriter {
    fn rewrite_locked(&self, path: &Path, bytes: &[u8]) -> Result<(), MigrationError>;
}
```
**Changes**: on every session-log open (once per run, at first access — not
conditional on detecting "not yet canonical"), the engine calls
`ctx`-injected `SessionLogRewriter::rewrite_locked` to perform **one
whole-file rewrite, with no prior `append_record` call**, before any new
record is appended. This is deliberately unconditional rather than
detection-gated: distinguishing an already-canonical Rust-written log from a
bash-written one would require either a field-order comparison (unreliable
via `serde_json::Value`, whose default map type is unordered, without
enabling `preserve_order`) or a required-field check that can't actually
tell the two apart. Skipping detection entirely and always re-canonicalising
is simpler and just as correct: for an already-canonical log the rewrite is
idempotent (byte-identical output), so running it unconditionally costs one
redundant rewrite of a small file, not a correctness risk. The rewrite is
performed through the **same locked path** `RecordStore::append_record`/
`remove_by_key` use — not the bare `AtomicWrite::write`, which acquires no
lock at all. `migrate-adapters`' `SessionLogRewriter` implementation
delegates to `FileCorpusStore::replace_locked`, a new method
(`cli/corpus-adapters/src/store.rs`) mirroring `append_record`'s existing
`ensure_contained` → `create_dir_all` parent → `lock::acquire(&lockdir(path),
self.lock)` → write sequence, but replacing the file's whole content instead
of appending a line:
```rust
impl FileCorpusStore {
    pub fn replace_locked(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError>;
}
```
so the cutover participates in the same critical section as every other
session-log writer, past or concurrent — this method does not exist today
and must be added as part of this phase, not assumed present. Per the
confirmed direction: a
record that is syntactically valid JSON but fails AC-8 (empty/absent
`proposed_value`) makes the **whole cutover refuse** — exit non-zero, stderr
message pinned by substring, the log file left byte-unchanged, no corpus
artefact mutated (this exact contract — one rename, no prior append,
refuse-not-normalise on an invalid record, byte-unchanged file on refusal —
is a named AC and gets its own test). **When no session-log file exists yet
at all** (the ordinary first-ever-run case for a migration never touched
interactively before), the cutover is a no-op — there is nothing to
rewrite — and the subsequent `append_record` call creates the file exactly
as it does today; the cutover never creates an empty file ahead of the
first real record.

### Success Criteria

#### Automated Verification:

- [x] `cargo test -p migrate -p corpus-adapters` — round-trip adversarial
      test green (quotes, backslashes, newlines, tabs, non-ASCII)
- [x] `cargo test -p corpus-adapters` — `FileCorpusStore::replace_locked` has
      its own unit tests: whole-file replace succeeds, acquires the same
      `lockdir(path)` a concurrent `append_record` call would block on
      (lock-contention test), and respects `ensure_contained`/parent-creation
      exactly as `append_record` does
- [x] A static check (new `tasks/lint` addition or a `cargo test` assertion
      over `cli/`) confirms no `awk` invocation or `awk`-based JSON parsing
      exists under the new crates — trivially true at this phase since no awk
      exists in Rust, but recorded as a permanent regression guard
- [x] Fixture test: given a bash-written session log, the Rust engine
      performs exactly one rename onto the log path with no prior append and
      the decision set read back afterward equals the pre-rewrite set — not
      driven against Phase 0's `interactive/doc-example/` fixture
      specifically (deferred; see deviations), but against an equivalent
      hand-built bash-shaped record; the "same lock a concurrent writer
      would block on" claim is proven structurally (`replace_locked` and
      `append_record` share one `lockdir(path)` derivation and one
      `lock::acquire` call site) rather than by an instrumented spy
- [x] Fixture test: given an already-canonical Rust-written log, the
      unconditional rewrite still occurs but is byte-identical before/after
- [x] Fixture test: given no session-log file at all, the cutover no-ops —
      no file is created
- [x] Fixture test: given that same log hand-truncated mid-record, the run
      refuses, log file byte-unchanged
- [x] Fixture test: given a syntactically-valid record with empty
      `proposed_value`, the cutover refuses, log byte-unchanged
- [x] Emitted records' field order asserted against the existing
      canonical-order test in `corpus-adapters`'s test suite — not exercised
      through `migrate`'s own record construction, since the domain types
      that would construct one (`Transformation`/`Decision`) belong to the
      Interactive Framework phase and don't exist yet
- [x] `user_value` presence-vs-`outcome` coupling: both violation directions
      (present-when-not-edited, absent-when-edited) rejected — test each.
      **This rule was not previously enforced anywhere in the codebase** —
      added to `compose_record` in this phase, a real (if narrow) gap this
      plan's own AC surfaced, not a pre-existing guarantee this phase merely
      exercised
- [x] `mise run cli:check` exits 0

#### Manual Verification:

- [x] None — this phase is pure domain/adapter logic with no new CLI surface

**Deviations from the above, found during implementation:**

- **The domain crate cannot compose or parse JSON at all** (`corpus`
  depends on `kernel` only) — so `SessionLogRewriter::rewrite_locked(path,
  bytes)` as originally sketched, taking caller-prepared bytes, was not
  buildable: no crate on the domain side of the port boundary can produce
  those bytes. Landed instead as `SessionLogRewriter::cutover(path)` — no
  `bytes` parameter; the adapter reads the current file itself, since
  reading-and-recomposing is inherently the same infrastructure concern as
  writing it back. The domain engine's role (Phase 5) is unchanged: decide
  *when* to call it, once per run at first access.
- **No standalone `cli/migrate/src/session_cutover.rs` domain file exists.**
  AC-8 validation (a record needs a non-empty `proposed_value`) is enforced
  by reusing `compose_record`'s existing structural checks directly — a
  record that fails to recompose fails the whole cutover — rather than
  duplicating that rule as a separate domain-level predicate with nothing
  else to validate once the JSON round-trip itself is adapter-side work.
- **`corpus_adapters::jsonl` (`compose_record`/`remove_prefix`, plus the new
  `parse_record`) was widened from a private `mod` to `pub mod`**, the same
  shape of change Phase 3 made to `corpus_adapters::lock` — the parsing
  inverse this phase needs has to live somewhere, and duplicating the
  composer's field-order/escaping knowledge in a second crate was rejected
  as the worse option.
- Extras recovered by `parse_record` come back in the parsed JSON object's
  own (lexicographic, `BTreeMap`-backed) key order, not necessarily a
  bash-written record's original declaration order — disclosed in the
  function's own doc comment. `compose_record` still re-canonicalises
  deterministically regardless, so cutover idempotency is unaffected; only a
  non-alphabetical extras order in the source record would not be
  reproduced verbatim.
- The two fixtures Phase 0 captured specifically for this phase
  (`interactive/doc-example/`'s bash-written session log) are not replayed
  directly — Phase 5 is what actually drives that fixture end to end (it
  needs the interactive engine to exist first); this phase's own tests use
  an equivalent hand-built bash-shaped record instead, covering the same
  behaviour the fixture would exercise.

---

## Phase 5: Interactive Framework (In-Process, No FIFO)

### Overview

The full ADR-0037/0038 contract, implemented as direct Rust trait calls
instead of a wire protocol: trigger predicate routing, the three mandatory
display elements, resumability (write-ahead-log invariant), accept/edit/skip,
sticky skip, source drift, `--list`, the decisions-file flow, the up-front
dry-apply validation pass, the structured stall (0116), and the injectable
30s timeout. No concrete migration exists yet (0007 lands in Phase 8, gated
on 0195) — this phase is built and fully tested against a
`FixtureMigration` test double implementing the same four-callback trait a
real migration will.

### Changes Required

#### 1. Transformation/Decision types and the `InteractiveMigration` trait

**File**: `cli/migrate/src/interactive.rs`
**Changes**:
```rust
pub struct Transformation {
    pub key: String, pub path: String, pub anchor: String,
    pub proposed: String, pub predicate_value: String,
    pub display: String, pub extras: Vec<(String, String)>,
}
pub enum PredicateOutcome { Prompt, Mechanical, Fail(String) }
pub enum Decision { Accept, Edit(String), Skip }
pub trait InteractiveMigration: MigrationMeta {
    fn emit_transformations(&self, ctx: &dyn MigrationContext) -> Vec<Transformation>;
    fn evaluate_predicate(&self, t: &Transformation) -> PredicateOutcome;
    fn validate_edit(&self, t: &Transformation, value: &str) -> Result<(), String>;
    fn apply_decision(&self, t: &Transformation, d: &Decision, ctx: &dyn MigrationContext) -> Result<(), String>;
    fn verify_applied(&self, t: &Transformation, recorded: &corpus::Record) -> bool { true }
}
```
No wire protocol: `emit_transformations` returns a `Vec` directly (replacing
`harness_emit_transformation`'s base64-display-block TSV emission);
`evaluate_predicate` is called as a plain function per transformation
(replacing the `PROMPT\tkey...` frame — there is no `harness_field` TSV
extraction, since the whole `Transformation` struct is already typed and in
hand); `validate_edit` returns `Result<(), String>` directly (replacing
`harness_reject`'s stderr side-effect — the engine prints
`"[interactive] {message}"` itself from the `Err` value, at exactly one call
site, so the "do not double-prefix" bash comment
(`interactive-lib.sh:864-868`) has no Rust analogue to get wrong).

#### 2. `DecisionSource` port and the engine loop

**File**: `cli/migrate/src/ports.rs`, `cli/migrate/src/engine.rs`
**Changes**:
```rust
pub enum DecisionError { NoInputAvailable, Timeout, Eof }
```
`Eof` is distinct from `Timeout`: it's what `TtyDecisionSource` returns if
stdin closes mid-session (the reader thread's channel disconnects rather
than timing out) — a real, different code path from "no response within
bound", not a spare variant. The engine treats it identically to `Timeout`'s
terminal contract (stderr message, empty stdout, exit non-zero, session log
left as of the last completed decision) — bash's own `read_decision`
collapses both "never had input" and "input closed mid-read" into the same
stall/failure path, so no distinct wording is introduced for `Eof` either.
```rust
pub trait DecisionSource {
    fn next_decision(&self, t: &Transformation, timeout: Duration) -> Result<Decision, DecisionError>;
}
```
```rust
pub fn run_interactive(
    migration: &dyn InteractiveMigration,
    ctx: &dyn MigrationContext,
    decisions: &dyn DecisionSource,
    session_log: &dyn RecordStore,
    timeout: Duration,
) -> Result<ApplyOutcome, MigrationError>;
```
Same `Result<ApplyOutcome, MigrationError>` shape `Migration::apply` uses —
`run_pending()` (Phase 2) needs no separate mapping step, only the enum
dispatch itself. `ApplyOutcome::NoOpPending` is never produced by this
function — an interactive migration either completes every transformation
(`Ok(Applied)`) or fails/stalls/times out (`Err(MigrationError)`); bash's
`MIGRATION_RESULT: no_op_pending` sentinel has no interactive-migration
caller (0007 doesn't emit it) and is therefore not ported to
`InteractiveMigration` — a documented narrowing, not an oversight, matching
this plan's treatment of `ACCELERATOR_MIGRATIONS_DIR`'s removal. On
`Fail(msg)`, the structured stall, or a `DecisionSource::Timeout`, the
engine prints its own dedicated terminal message (the `"[{id}] {msg}"`
FAIL relay, the `MIGRATION STALLED` block, or the timeout stderr line) and
returns `Err`. **`run_pending()`'s generic `[<id>] failed` line still
prints on top of that** — this is not a double-print bug to avoid: bash's
own `run-migrations.sh` apply loop prints the identical generic
`[<id>] failed` after any non-zero migration exit, mechanical or
interactive, including after a stall or timeout's own dedicated message —
the unification mirrors an existing bash symmetry, not a Rust-side
simplification introducing new behaviour.

`run_interactive`'s own logic: emits
transformations once; for each, if a session-log record already exists for
its key, **source drift is checked first, before any outcome-specific
handling, and applies to every resumed record regardless of outcome —
accepted, edited, or skipped alike** (verified against bash:
`interactive-harness.sh:642-686`'s `_harness_handle_resume` compares
`recorded.proposed_value` against the live proposed value before branching
on outcome at all — a skipped record is not exempt from drift, only from
`verify_applied` below). If the recorded `proposed_value` differs from the
live `Transformation.proposed`, discard via `RecordStore::remove_by_key`
and re-prompt (no distinct "drift" wording, matching bash exactly) — this
takes priority over everything below, for every outcome.

Only when there is no drift does the engine branch by outcome: for an
accepted/edited record specifically, `verify_applied(t, recorded)` is
called **before** replaying `RESUMED_APPLIED` (mirroring bash's
`migration_verify_applied`, `SKILL.md:145` — called on resume, before
emitting `RESUMED_APPLIED`, for accepted/edited keys only); `false` is
handled identically to the drift case above (discard via
`RecordStore::remove_by_key`, re-prompt — the framework doesn't distinguish
"mutation absent, e.g. a partial-apply crash" from "source changed" in its
recovery action, only in which check detected it); a skipped record always
replays `RESUMED_SKIPPED` silently without calling `verify_applied` (SKILL.md's
"per resumed-accepted/edited key" scope) — replaying silently matches
bash's silent no-op (`interactive-lib.sh:833-835`).

If no session-log record exists yet for the key, evaluate the
predicate — `Mechanical` applies immediately with no record persisted;
`Fail(msg)` aborts the whole migration with `"[{id}] {msg}"` verbatim
(matching bash's `FAIL` frame relay exactly, including that the message is
NOT re-wrapped); `Prompt` renders the display (three mandatory elements:
proposed value, `path:anchor` source location, predicate's evaluated value,
plus author `display=` content) and calls `decisions.next_decision(t, timeout)`.
On `Decision::Edit(v)`, `validate_edit` runs first — a rejection prints
`"[interactive] {message}"` and re-prompts (no record persisted for the
rejected attempt). On any accepted decision, the write-ahead-log invariant is
enforced structurally: `session_log.append_record(...)` is called and must
return `Ok` **before** `apply_decision` is invoked — never the reverse, and
never concurrently (this ordering is asserted by a test double that fails
the run if `apply_decision` is observed before the record's write, per the
AC's "recording store port shows completing before the first corpus
mutation" requirement). Sticky skip: a `skipped` record replays without
re-evaluating the predicate, permanently **unless the source drifts** — a
drifted skip is discarded and re-prompted exactly like a drifted
accept/edit (see above), not treated as exempt. Migration completes
(ledger append happens) only when every transformation has a terminal
record.

#### 3. Timeout via the TTY adapter

**File**: `cli/migrate-adapters/src/tty_decision_source.rs`
**Changes**: `TtyDecisionSource::next_decision` renders the prompt, spawns a
thread reading one line from stdin, blocks on
`mpsc::Receiver::recv_timeout(timeout)`; on timeout, returns
`DecisionError::Timeout` (the reader thread is detached — abandoned, not
joined, since stdin reads have no cancellation primitive in std; acceptable
because the whole `accelerator-migrate` process exits immediately after,
which the OS reclaims). This 30s bound is new behaviour, not a bash-parity
requirement — per the Key Discoveries correction above, bash's own TTY read
has no timeout at all; the bound here comes directly from 0172's own AC.
The engine maps `Timeout` to: stderr message pinned
by substring, empty stdout beyond what was already flushed, exit non-zero,
session log left exactly as of the last completed decision (so the next run
prompts only the remaining undecided transformations). Default timeout is a
`Duration::from_secs(30)` constant in `migrate-cli`, threaded down as a
parameter — test-injectable, no CLI flag, no config key (matching the
Requirements' explicit "not user-configurable").

#### 4. `NoInputDecisionSource` and the structured stall (0116)

**File**: `cli/migrate/src/ports.rs`
**Changes**: selected by the composition root when stdin is not a TTY and no
decisions file was supplied. The engine's call site (point 2 above) is
genuinely generic — it always calls `next_decision(t, timeout)` with the
same `timeout` value regardless of which `DecisionSource` is active; there
is no per-source special-casing at the call site. The "timeout never armed"
guarantee instead comes from `NoInputDecisionSource::next_decision`'s own
implementation: it returns `DecisionError::NoInputAvailable` synchronously
on the very first call **without reading or otherwise consulting its
`timeout` argument at all** — no `Instant`, no sleep, no channel wait, the
parameter is simply unused. The engine renders the full `MIGRATION STALLED`
block (verbatim below) immediately and exits non-zero. Requirements:
"reached without the timeout being armed, asserted via the injected timeout
seam rather than by elapsed time" — the engine asserts this via a spy
`DecisionSource` (or an instrumented `NoInputDecisionSource`) that records
whether any timing primitive was touched during the call, not by asserting
a special-cased call-site branch (there is none).

#### 5. `MIGRATION STALLED` block (verbatim, `interactive-lib.sh:313-345`)

**File**: `cli/migrate-cli/src/render.rs`
**Changes**: per-migration-ID-prefixed lines exactly as captured:
```
[{id}] MIGRATION STALLED: no decision input available
[{id}]   pending decision: {key}
[{id}]   No decisions file, terminal, or piped input was available to
[{id}]   answer this prompt, so the migration cannot proceed.
[{id}]
[{id}]   This migration may have already partially modified the
[{id}]   working tree. Re-running /accelerator:migrate resumes this
[{id}]   partial run when the base revision is unchanged (decided
[{id}]   transformations are replayed, not re-applied).
[{id}]
[{id}]   To resume: each run answers the current prompt only (you
[{id}]   may be stalled again for the next undecided transformation):
[{id}]     1. write the decision (accept | skip | edit <value>),
[{id}]        one per line, to: {decisions_path}
[{id}]        (create this file yourself; do not overwrite existing
[{id}]        migrations-{id}-* state files)
[{id}]     2. then run (copy-pasteable):

accelerator migrate --decisions-file {decisions_path}

[{id}]   equivalent env-var form:

ACCELERATOR_MIGRATE_DECISIONS_FILE={decisions_path} accelerator migrate
```
(the two command lines flush-left, no `[{id}]` prefix, blank lines around
them — exactly matching bash's documented copy-pasteable-command intent).
`decisions_path` = `.accelerator/state/migrations-{id}-decisions.txt`.
Names every pending decision key, not just the current one (0172's AC text
strengthens 0116's "best-effort" plural into every pending key for this
port — the stalled transformation plus every transformation still after it
in emission order that has no session-log record).

#### 6. `--list` and the decisions-file flow

**File**: `cli/migrate/src/list.rs`, `cli/migrate/src/decisions_file.rs` (new)
**Changes**: `--list` skips pre-flight entirely (Phase 3), calls
`emit_transformations` + `evaluate_predicate` for every pending interactive
migration, excludes already-decided (session-logged) keys, prints
`"{pos}\t{key}\t{proposed}\t{path}:{anchor}"` per undecided transformation
(note: `anchor`, not `field` — confirmed field order from
`run-migrations.sh:530-541` is `pos, key, proposed, path:anchor`), segmented
by `"# migration {id}"` headers with position restarting at 1 per migration
when more than one is pending, plus the exact stderr note
(`run-migrations.sh:506-508`, verbatim, reproduced with
`--decisions-file`/`# migration <id>` unchanged). `"no pending
transformations"` (lowercase, no trailing punctuation) when nothing pending.
Never mutates. The decisions-file reader: blank lines and `#`-prefixed lines
ignored, CRLF tolerated, one verb per undecided-transformation position in
emission order — `accept | skip | edit <value>`. The dry-apply validation
pass runs before any live decision: replays every position against
`evaluate_predicate`/`validate_edit` with **no** `apply_decision` call and
**no** session-log write, failing closed and naming "position N" on: unknown
verb, too-few (missing at position N), too-many (surplus at position N+1),
rejected edit with no correcting line following — all four exact message
templates reproduced verbatim from Phase-0-captured strings
(`interactive-lib.sh:600-604,613-616,619-624,633-644`), each substituting
`accelerator migrate` for the bash invocation where applicable.

### Success Criteria

#### Automated Verification:

- [ ] `cargo test -p migrate -p migrate-adapters` — the engine loop tested
      end-to-end against `FixtureMigration` for: fresh accept/edit/skip;
      resumed-applied/resumed-skipped (silent, no stderr text — asserted via
      an empty-stderr check on the resume-only path); source drift on an
      accepted/edited record (differing `proposed_value` discards and
      re-prompts, no drift wording); source drift on a **skipped** record
      (discards and re-prompts identically — not exempt from drift despite
      sticky-skip's otherwise-permanent replay); sticky skip across two runs
      with an *unchanged* `proposed_value` (replays permanently, no
      re-prompt); predicate `Fail` aborting with the exact
      `"[{id}] {message}"` text **followed by** `run_pending()`'s generic
      `[<id>] failed` line (both present, matching bash's own doubled
      output on an interactive migration failure — not a bug to suppress);
      write-ahead-log ordering enforced (a test
      double that would panic if `apply_decision` observed before the record
      write); `verify_applied` consulted on resume for an accepted/edited
      record — `false` discards and re-prompts identically to source drift,
      `true` (the default) replays silently, and a skipped record never
      triggers the call at all (mirroring bash's own
      `test-migrate-interactive.sh:917-941` coverage)
- [ ] Fixture test against `interactive/doc-example/`: normalised transcript
      (stdout+stderr, `<SANDBOX>` + invocation-path normalised) matches the
      Phase-0 golden **exactly**, including the `[interactive] empty value
      not allowed` line and the re-prompt that follows it; session log has
      exactly two records (`link-A` edited, `link-C` skipped), no record for
      mechanical `link-B`
- [ ] Fixture test against `interactive/accept-verb/`: `outcome: accepted`,
      no `user_value` key present at all (not `null` — presence-based)
- [ ] Fixture test against `interactive/three-decision/`: abort seam after 2
      decisions leaves a resumable log; re-run prompts exactly the 3rd
- [ ] Fixture test against `interactive/validator-rejecting/`:
      `[interactive] <message>` printed, transformation re-prompted, never
      applied
- [ ] Timeout test (generic engine loop): injected `Duration::from_millis(50)`
      bound, a `DecisionSource` test double that never resolves — run exits
      non-zero within bound+2s (wall-clock asserted with a generous CI-safe
      margin, never a bare `sleep 30`), stderr pinned by substring, stdout
      empty, session log unchanged, next run prompts only the undecided
      transformation
- [ ] Timeout test (real `TtyDecisionSource`, not a test double): a
      pipe-backed stdin fixture that never writes a line, injected
      `Duration::from_millis(50)` bound — asserts the actual spawned-thread +
      `mpsc::recv_timeout` implementation fires within bound+2s with the same
      stderr/stdout/session-log contract as the generic test above, and that
      the process exits promptly rather than blocking on the detached
      reader thread's teardown
- [ ] Default-bound test: asserts the constant equals `Duration::from_secs(30)`
      directly (no timing involved)
- [ ] Composition-root selection test (the real dispatch logic, not the
      test-injectable override seam from Phase 1): each combination of
      TTY-present/decisions-file-present is exercised and asserted to
      construct the correct `DecisionSource` — decisions-file supplied (with
      or without a TTY) selects `DecisionsFileDecisionSource`; a real TTY
      with no decisions-file selects `TtyDecisionSource`; neither present
      selects `NoInputDecisionSource` — this is the code path every real
      invocation depends on, not just the paths the fixture-test override
      seam exercises one at a time
- [ ] `migrate/0007/`-shaped fixture test (using `FixtureMigration`, not yet
      real 0007): no TTY, fd 0 EOF, no decisions file → `MIGRATION STALLED`
      block matches Phase-0 capture exactly, naming every pending key,
      reached with the timeout seam never armed (asserted structurally, per
      point 4 above), exits non-zero, mutates no corpus artefact beyond any
      non-interactive migration that legitimately ran first
- [ ] `list/single-pending/` and `list/multi-pending/` fixture tests match
      captured `--list` goldens byte-for-byte after sandbox-root
      normalisation, including the `# migration <id>` segmentation, restart-at-1
      positions, and the stderr note
- [ ] `decisions-file/*` fixture tests: each of the five malformed-input
      cases produces its exact captured error message and leaves the corpus
      unmutated (checked via a full-tree hash before/after)
- [ ] `mise run cli:check` exits 0

#### Manual Verification:

- [ ] Run the `doc-example` fixture interactively at a real terminal (not
      the automated harness) and confirm the prompt rendering is readable
      and the accept/edit/skip loop feels correct
- [ ] Manually trigger the structured stall (pipe `/dev/null` to stdin, no
      decisions file) and confirm the copy-pasteable commands work verbatim
      when pasted into a shell

---

## Phase 6: Migrations 0001–0006 Port

### Overview

Port the six non-interactive migrations as ordinary Rust `Migration`
implementations. Each gets its own fixture-golden test from Phase 0. This
phase has no dependency on 0195 and can land independently.

### Changes Required

For each migration, **File**: `cli/migrate/src/migrations/m0001.rs` …
`m0006.rs` (new), implementing `Migration` with `id()` returning the exact
slugged ID and `apply()` performing the transform via `ctx: &dyn MigrationContext`
(Phase 2) — `document::{parse, render}` is imported directly for
frontmatter rewrites (a permitted zero-infrastructure exception, Phase 1
point 4), config access goes through `ctx.doc_type_dirs()` where a migration
needs it (rather than `config::ConfigAccess` directly), and all file
writes go through `ctx.write()` (which routes through the manifest and the
underlying `store::atomic_write`/`corpus_adapters` machinery at the adapter
layer — migration code itself never imports `store` or `config` directly).
A filesystem-walk port (declared alongside `MigrationContext`'s other local
ports) covers directory enumeration/rename. Per migration:

- **0001** (`rename-tickets-to-work`): frontmatter `ticket_id:`→`work_item_id:`
  key rename (both-present → drop `ticket_id:`; only-old → rename), pinned-path-aware
  directory renames (`meta/tickets`→`meta/work`,
  `meta/reviews/tickets`→`meta/reviews/work`, gated on the resolved path
  matching the plugin default — ADR-0023's pinned-path preservation rule),
  and the ordered config-key rewrite chain (nested-then-flat, value-aware-then-generic,
  matching `rewrite_config`'s exact ordering at `run-migrations.sh` counterpart
  `0001-rename-tickets-to-work.sh:98-124`).
- **0002** (`rename-work-items-with-project-prefix`): `no_op_pending` when
  `work.id_pattern` lacks `{project}`; fatal (typed `MigrationError`, not a
  bash `exit 1` + stderr scrape) when the pattern has it but
  `work.default_project_code` is empty, with the fatal message text
  preserved verbatim; collision detection before any rename; frontmatter
  `work_item_id:` rewrite, corpus-wide reference-field rewrite (inline and
  multi-line list forms), Markdown link rewrite, and the two-pass prose
  rewrite (heading `#NNNN` and fenced-code-block path references) — using
  `document`'s value tree for the frontmatter passes and targeted string
  operations (matching bash's `sed`/parameter-substitution behaviour exactly,
  including the macOS-bash-3.2-noted backslash divergence bash had to work
  around, which Rust's `str::replace` does not have) for the prose passes.
- **0003** (`relocate-accelerator-state`): the `has_source`-or-`.accelerator`-non-scaffold-content
  idempotency gate reproduced exactly (the `no_op_pending` sentinel fires
  only when *neither* condition holds); scaffold init; root/inner
  `.gitignore` rewrites; pinned-override warnings for `paths.templates`/`paths.integrations`
  (probed **before** any move, matching bash's `paths.tmp`-before-move
  ordering note); the source→destination move table; state-file merge
  (union-dedup, destination-first-wins ordering) via `corpus_adapters`;
  the Jira inner-`.gitignore` rule set (`JIRA_INNER_GITIGNORE_RULES`,
  byte-identical to the pinned test contract in `test-jira-paths.sh`).
- **0004** (`restructure-meta-research-into-subject-subcategories`): the
  per-key config probe (nested/flat form detection), mixed-state fatal
  guard (old+new key both present), the nothing-to-migrate `exit 0`
  equivalent, planned moves executed via the injected filesystem port, the
  boundary-aware inbound-link rewrite with its sibling-subcategory exclusion
  set (the exact idempotency mechanism — a Rust port must reproduce the same
  exclusion list, not just "rewrite once").
- **0005** (`rename-work-item-type-to-kind`): dangerous-path refusal (exact
  list: `. | .. | / | /* | */.. | ../* | */../*`), per-file frontmatter
  `type:`→`kind:` and body-label `**Type**:`→`**Kind**:` rewrites with
  divergence-warn-then-drop-losing-side semantics, exact warning text
  reproduced.
- **0006** (`canonicalise-work-item-id-and-author`): the six-pattern
  single-pass rewrite (`work-item:`/`work_item_id:`/`researcher:`/`author:`
  frontmatter, `**Researcher**:`/`**Author**:` pre-first-H2 body), unsafe-value-shape
  refusal (`0006-REFUSE`), divergence diagnostics (`0006-DIVERGE`) preserved
  as stable, tested diagnostic prefixes (even though nothing downstream
  parses stderr any more — these strings are asserted by name in the AC's
  parity criteria via the fixture goldens), corpus-walk deduplication by
  canonicalised path, and the userspace-template-override pass.

Every migration's idempotency self-check is preserved (ADR-0023's
belt-and-suspenders requirement — the ledger filter is not the only
guard); every `atomic_write` call site becomes an injected-port write.

### Success Criteria

#### Automated Verification:

- [ ] `cargo test -p migrate` — one fixture-golden test per migration
      (`migrate/0001/` … `migrate/0006/`) asserting corpus-state byte-parity
      (after volatile-field normalisation) against the Phase-0 bash golden
- [ ] Each migration's idempotency asserted directly: running it twice in a
      row produces a byte-identical second-run no-op (via the ledger filter)
      **and**, separately, running the underlying transform function twice
      without the ledger filter also converges (the self-detection guard)
- [ ] 0001: pinned-path preservation asserted — a fixture with a
      user-pinned `paths.tickets` value renames the frontmatter key but
      leaves the directory untouched
- [ ] 0002: `no_op_pending` fixture (pattern without `{project}`) and the
      fatal-missing-project-code fixture both assert exact message text and
      typed outcome
- [ ] 0003: the `has_source`-false-and-scaffold-only-content fixture asserts
      `no_op_pending`; a fixture with one stray non-scaffold file under
      `.accelerator/` asserts it proceeds instead
- [ ] 0004: mixed-state fixture (both old and new keys present) asserts the
      fatal guard fires before any move; sibling-subcategory exclusion
      asserted by re-running the inbound-link rewrite twice and diffing
      (must be byte-identical on the second run)
- [ ] 0005: dangerous-path fixture for each of the six listed shapes asserts
      refusal; divergence fixture asserts the warning text and that the
      losing side is dropped
- [ ] 0006: unsafe-value-shape fixture asserts `0006-REFUSE`-equivalent
      refusal (typed, not stderr-grepped) with the original line preserved
      unrewritten; divergence fixture asserts `0006-DIVERGE`-equivalent
      diagnostic and new-key-wins
- [ ] `mise run cli:check` exits 0

#### Manual Verification:

- [ ] Run `accelerator migrate` against a scratch repo seeded with
      pre-0001-through-0006 state and confirm the end state matches a
      parallel bash-migrated copy of the same seed, by eye

---

## Phase 7: Discoverability Hook Port (absorbs 0183; blocked by work item 0195)

### Overview

**This phase cannot land until work item 0195 lands and Phase 8 registers
migration 0007** — mirroring Phase 8's own blocking annotation, not just a
note. This is a hard precondition, not a scheduling preference: both this
phase's discoverability-hook logic and its skill-rebinding sub-step compute
their behaviour from the compiled `registry()`, which omits 0007 until
Phase 8 lands. If Phase 7 merged first, the discoverability hook's
"highest-available" figure would be drawn from the incomplete registry (the
SessionStart advisory would falsely report the repo up to date even with
0007 pending), and the rebound `/accelerator:migrate` skill invocation would
silently stop covering 0007 entirely — the exact regression a prior planning
pass flagged and addressed with an explanatory note alone; that note has no
teeth without this blocking annotation, so it is added here instead of
relied on by itself.

Port `hooks/migrate-discoverability.sh` onto the bootstrap path, rewrite its
`hooks.json` registration, and — per the confirmed direction — absorb 0183's
`systemMessage` contract directly rather than preserving stderr output, using
the already-landed `kernel::hooks::session_start` envelope.

### Changes Required

#### 1. Hook logic

**File**: `cli/migrate-cli/src/discoverability.rs` (new), dispatched as
`accelerator migrate --discoverability-hook` or a dedicated flag — **decide
the exact invocation shape to match 0169's registered pattern**: the other
three converted hooks are separate subcommands on their own sub-binaries
(`vcs detect`, `config summary`), not flags, so this hook is exposed as
`accelerator migrate discoverability` if `migrate` gains a subcommand for it
— but the Requirements explicitly forbid subcommand redesign of the
flag-for-flag surface. Resolution: this hook is **not** part of the
flag-for-flag surface (bash's `hooks/migrate-discoverability.sh` was never a
`run-migrations.sh` flag) — it is a new, hook-only entry point. Expose it as
`accelerator migrate --discoverability-hook --format=hook --fail-safe`, a
flag orthogonal to the flag-for-flag migration surface, matching bash's
"separate script, separate hooks.json entry" shape most closely while
staying inside the single `migrate` dispatch token 0187 requires.

Logic: compares highest-available (from the compiled-in `registry()`, no
filesystem glob needed — Phase 2) against highest-applied (from the ledger,
falling back to the legacy `meta/.migrations-applied` path per the existing
exist-aware fallback chain, `hooks/migrate-discoverability.sh:33-39`,
reproduced exactly including the deprecation-track-shim framing); fires only
for an Accelerator-managed repo (`.accelerator/` or
`.claude/accelerator.md` or `meta/` present); always exits 0.

#### 2. `systemMessage` envelope (absorbing 0183)

**File**: `cli/migrate-cli/src/discoverability.rs`
**Changes**: calls `kernel::hooks::session_start(context, Some(&advisory))`
where `advisory` is the same content bash printed to stderr
(`hooks/migrate-discoverability.sh:68-70`, "[accelerator] {state_file} is
behind the plugin (highest applied: {label}; highest available:
{highest}). Run /accelerator:migrate to bring it up to date." — text
unchanged, only the delivery channel moves from stderr to `systemMessage`).
Update `meta/work/0183-session-start-hook-advisories-reach-nobody-on-stderr.md`:
set `status: abandoned`, record the reciprocal note that its contract was
absorbed into 0172, per the confirmed direction.

#### 3. `hooks.json` rewrite

**File**: `hooks/hooks.json`
**Changes**: replace the `migrate-discoverability.sh` command string
(currently index 2) with
`"${CLAUDE_PLUGIN_ROOT}/bin/accelerator migrate --discoverability-hook --format=hook --fail-safe"`
— **select by command string, not array index**, per 0169's own
parity-gate precedent (`hooks/test-vcs-detect.sh`'s rewrite pattern), since
0182 has since inserted `hooks/launcher-link-refresh.sh` at index 3 and this
edit must not collide with it. Note: `${CLAUDE_PLUGIN_ROOT}` here is the
literal token bash/hooks.json still uses as of this plan's writing (0182's
own rename to `ACCELERATOR_PLUGIN_ROOT` covers `cli/`-internal references —
verify against 0182's landed state at implementation time whether
`hooks.json`'s own command strings were also renamed; if so, match the
already-converted `vcs detect`/`config summary` entries' current form
exactly rather than the token named here).

#### 4. Skill binding (co-lands with Phase 1's registration)

**File**: `skills/config/migrate/SKILL.md`
**Changes**: the `!` preprocessor invocation and any `Bash(...)` rule
covering `run-migrations.sh` repointed to `accelerator migrate` — this is
the skill-binding half of the 13-point checklist's point 7, and per its own
"points 1 and 7 must land in the same change" rule, this file's edit and
Phase 1's `DISPATCHED_SUBBINARIES` addition must be the same commit **unless**
Phase 1 used the `SKILL_EXEMPT_SUBBINARIES` interim measure, in which case
this phase removes that exemption entry in the same commit as this rewrite —
**and**, per Phase 1's implementation-time deviation note, also removes
`"migrate"` from `_KNOWN_PENDING_SKILL_BINDINGS` in
`tests/unit/tasks/shared/test_dispatch_coherence.py` in that same commit;
leaving either behind after the real binding lands makes that allowlist stale.
Blocked for the reason stated in this phase's Overview.

### Success Criteria

#### Automated Verification:

- [ ] `hooks/test-migrate-discoverability.sh` (still bash, still the oracle)
      repointed to invoke the compiled `accelerator-migrate` binary and
      passes — required by the AC as a precondition of that suite's later
      retirement in Phase 10
- [ ] A new Rust test asserts the `systemMessage` envelope's exact JSON
      shape via `kernel::hooks::session_start`, matching the pattern already
      tested for `vcs detect`/`config summary`
- [ ] A `hooks.json`-parsing test selects the migrate-discoverability entry
      by command-string substring match (not index) and asserts it invokes
      `accelerator migrate --discoverability-hook`
- [ ] `mise run lint:skill-permissions:check` (or equivalent) passes with
      the new invocation covered
- [ ] `mise run cli:check` exits 0

#### Manual Verification:

- [ ] Start a fresh Claude Code session in a repo with a pending migration
      and confirm the advisory appears as a system message (not silently
      swallowed on stderr) — this is the concrete, observable fix for
      0183's original bug

---

## Phase 8: Migration 0007 Port (blocked by work item 0195's self-validation obligation only)

### Overview

**Correction to a stale premise from this plan's original planning session:**
the Current State Analysis's claim that typed-linkage extraction has "no
crate yet exists" is false as of this review — `cli/corpus/src/linkage.rs`
already implements the extraction/classification pipeline (`parse_document`,
`type_from_path`, `classify_band`, `resolve_path_target`, `Band`,
`LinkageRecord`) as landed, tested, in-process Rust in `corpus`, a crate
`migrate` already depends on (verified directly against the file, and
against `cli/corpus-adapters/src/assemble.rs`, which already consumes it for
the visualiser). **This phase's dependency on work item 0195 for linkage
extraction is therefore removed** — Phase 8 calls `corpus::linkage` directly
for extraction/classification, with no obligation on 0195 for it at all.

What genuinely doesn't exist yet, verified by reading `resolve_path_target`
and `assemble()` directly: neither checks whether a resolved target actually
exists in the corpus (both infer a type/ID from a path or token; neither
scans the corpus to confirm a file with that ID is really there). This is
the one piece Phase 8 still needs and doesn't yet have — but it's a small,
self-contained capability (the same local-port-declaration pattern Phase 2
uses for `MigrationContext`'s other capabilities), not a reason to depend
on 0195: **0172
builds its own `CorpusIndex` port and adapter for it (point 5 below)**, with
no cross-item obligation required.

**This phase remains blocked on work item 0195, but only for Obligation 2
below (the self-validation script's lifetime)** — a narrower, unrelated
dependency this correction doesn't touch. Everything else in 0007
(precondition prepass, fence-less backfill, the mechanical rewrite, and now
linkage extraction/classification too) has no dependency on 0195 and can be
built now.

### Cross-item obligation on 0195 (precondition of starting this phase)

**Phase 8 does not begin implementation until the obligation below is
recorded on 0195 itself — not merely described in this plan's text.** A plan
note referencing an interface 0195 hasn't agreed to is not a contract; the
edge must exist on 0195's own work item (as a Requirements line and a
`blocked_by`/`relates_to` edge) before this phase's Changes Required are
implemented.

**Obligation — self-validation script lifetime.** `self_validate_structural`/
`self_validate_referential` (point 3 below) currently shell out to
`scripts/validate-corpus-frontmatter.sh`, which 0195 also claims for
retirement. Before this phase begins, 0195's own edge must state one of:
(a) 0195 ships a Rust equivalent this phase calls in-process, or (b) 0195
keeps `validate-corpus-frontmatter.sh` on disk, undeleted, until this
phase's cutover (Phase 10) completes. Do not begin Phase 8 on the assumption
that a "temporary shell-out" fallback is available — 0195's own Requirements
already claim the script for deletion, so that fallback cannot be assumed
live without 0195 explicitly agreeing to option (b). Unlike the
now-removed linkage-extraction obligation, this one has not been
independently verified against the landed codebase in this review pass —
if a Rust equivalent to `validate-corpus-frontmatter.sh` also already
exists somewhere in `corpus`, that would narrow this obligation too, but
that's a separate investigation this pass didn't do.

### Changes Required (buildable now, ahead of 0195)

#### 1. Precondition prepass

**File**: `cli/migrate/src/migrations/m0007/prepass.rs` (new)
**Changes**: read-only corpus scan reproducing every refusal in
`precondition_prepass` (`0007-unify-meta-corpus-frontmatter.sh:322-380`): a
`work-item`-typed file missing `kind:` (run 0005 first), a work-item whose
`work_item_id` disagrees with its filename-derived ID, an unquoted foreign
`work_item_id` (run 0006 first), duplicate post-rewrite `type:id` identity.
Any refusal aborts the whole migration before any mutation — typed
`MigrationError`, not a stderr-grep count.

#### 2. Fence-less backfill and mechanical rewrite

**File**: `cli/migrate/src/migrations/m0007/backfill.rs`,
`.../rewrite.rs` (new)
**Changes**: reimplements the two-awk-program cascade
(`frontmatter-frag.awk`'s four shared primitives —
`fm_is_fence`/`fm_normalise_value`/`fm_semantic_inner`/`fm_refuses` — plus
`0007-frontmatter-rewrite.awk`'s single-pass rewrite: type canonicalisation,
`git_commit`→`revision`, forbidden-key drop with `pr_title`→`title` fold,
date/`last_updated` ISO normalisation, author capture, status vocab/legacy-map
reconciliation, linkage-key normalisation) using `document::{parse, render}`'s
value tree instead of a line-oriented awk pass — this is a genuine
re-architecture (tree-based, not line-based), not a transliteration, and its
own fixture-golden coverage (per-field, not just whole-file diff) is what
proves equivalence. `frontmatter-merge.awk`'s inject-or-replace-or-append-if-absent
merge logic is reimplemented directly against the `document` value tree
(single-cardinality replace, list-cardinality append-if-not-present,
preserving existing member order) — this is `migration_apply_decision`'s
core, called from the `InteractiveMigration::apply_decision` implementation
in point 5 below.

#### 3. Self-validation

**File**: `cli/migrate/src/migrations/m0007/validate.rs` (new)
**Changes**: `self_validate_structural` (post-rewrite, pre-`harness_run`
equivalent — pre-interactive) and `self_validate_referential` (post-interactive,
whole-corpus) both currently shell out to `scripts/validate-corpus-frontmatter.sh`.
That script is **also** claimed by 0195 for retirement
(`meta/work/0195-...:67,84,94`) — resolved per Obligation 2 above: this
phase calls 0195's in-process Rust equivalent if it has landed, or
`validate-corpus-frontmatter.sh` if 0195 has agreed (via its own recorded
edge) to keep it on disk until this phase's cutover. Either way the choice
is settled before this phase starts, not discovered mid-implementation.

#### 4. Doc-type table access

**File**: `cli/migrate/src/migrations/m0007/mod.rs`
**Changes**: `ctx.doc_type_dirs()` (Phase 2's `MigrationContext` method)
called — no `doc-type-table.sh` subprocess, no `ACCELERATOR_MIGRATION_MODE`
env var, no `--allow-legacy-layout` flag threading. `migrate-adapters`'
concrete `MigrationContext` implementation wraps `config::paths::doc_type_dirs`
against a `FileConfigStore` already constructed with `LegacyPolicy::Allow`
at the composition root for the whole migrate binary (Key Discovery
above) — the domain-crate migration code itself never imports `config`
directly.

#### 5. `CorpusIndex` adapter (new; no 0195 dependency, per the Overview correction)

**File**: `cli/migrate-adapters/src/corpus_index.rs` (new)
**Changes**: implements Phase 2's `CorpusIndex` port (declared alongside
`MigrationContext` — no new trait needed here). The concrete implementation
scans the doc-type directories (via the already-injected `doc_type_dirs()`)
once per run and, for each file found, calls `corpus::linkage::resolve_path_target`
on that file's own path — **the same function that produces the
`target_id` values `target_exists` must match against** — collecting the
resulting `(type, id)` pairs into a set; `target_exists(target_type,
target_id)` is then a direct set membership check. **This is deliberately
not** `corpus::work_item_id`/`corpus::slug`: `slug::derive` is a
display/UI convention that *strips* the date/ID prefix (the opposite of
what `target_id` contains for most doc types — plans, notes, research,
reviews — where `resolve_path_target`'s default case returns the *entire*
file stem, matching bash's own `derive_id` default,
`0007-unify-meta-corpus-frontmatter.sh:176-184`); and
`WorkItemIdScheme::extract_id` conditionally project-prefixes its result
when `work.id_pattern` contains `{project}`, while `resolve_path_target`'s
`WorkItems` branch and the bare-digit `## Dependencies`-section extraction
both always produce bare digits, never a project prefix — using `extract_id`
would silently fail to match every resolved-band work-item reference in any
repo that has run migration 0002 (Phase 6). Deriving both sides through the
identical function makes the comparison correct by construction rather than
by two independently-written conventions happening to agree. This is the
one piece of Phase 8's original linkage-handling design that genuinely
doesn't exist anywhere yet (verified: neither
`corpus::linkage::resolve_path_target` nor `corpus-adapters::assemble()`
check target existence, they only infer type/ID from a path or token) —
and it's small enough to build directly in this plan, with no 0195
obligation.

#### 6. `InteractiveMigration` implementation

**File**: `cli/migrate/src/migrations/m0007/mod.rs`
**Changes**: `emit_transformations` calls `corpus::linkage::parse_document`
directly (already an allowed dependency — Phase 1 point 1 — no seam or
cross-item obligation needed) per in-scope file, then consults
`ctx.corpus_index().target_exists(...)` (point 5 above) to drop
unresolvable `resolved`-band records with a diagnostic (never surfaced to
the human — matching bash's `0007-DIVERGE[reverse-orphan]` silent-drop
behaviour), and emits a `Transformation` per surviving record with
`key = "{path}#{anchor}"`, `proposed = "{linkage_key}={target}"`,
`predicate_value = band`. `evaluate_predicate` returns `Prompt` iff
`predicate_value == "ambiguous"`. `validate_edit` rejects unless the value
matches `{linkage-key}={typed-ref}` against the same `LINKAGE_REF_RE`-equivalent
regex and a known linkage-key set. `apply_decision` implements the
set-if-absent-or-equal single-cardinality
rule (never overwrites a divergent existing value — logs a diagnostic and
returns) before delegating to point 2's merge logic.

### Success Criteria

#### Automated Verification:

- [ ] `cargo test -p migrate -p migrate-adapters` — points 1, 2, 4, 5, and 6
      (prepass, backfill/rewrite, doc-type access, `CorpusIndex`, and the
      `InteractiveMigration` implementation) are all testable now, with no
      0195 dependency: `corpus::linkage::parse_document` is called directly
      against fixture documents, and `CorpusIndex::target_exists` is tested
      against a synthetic doc-type-directory fixture covering the two cases
      that would silently misclassify if derived via the wrong convention:
      (a) a **plan** file present at `2026-05-13-0055-sidebar-activity-feed.md`
      resolves via `resolve_path_target` to `target_id = "2026-05-13-0055-sidebar-activity-feed"`
      (the full stem) — asserting `target_exists("plan",
      "2026-05-13-0055-sidebar-activity-feed")` is `true`, and that a
      `slug`-derived candidate (`"sidebar-activity-feed"`) would have
      wrongly reported `false`; (b) a **work-item** file present under a
      `work.id_pattern` containing `{project}` — asserting
      `target_exists("work-item", "0042")` (the bare-digit form
      `resolve_path_target`/linkage actually produce) is `true`, and that
      an `extract_id`-derived project-prefixed candidate (`"PROJ-0042"`)
      would have wrongly reported `false`
- [ ] Fixture test against `migrate/0007/` (stall path, Phase 5 already
      covers the mechanism generically — this asserts the real migration's
      actual pending keys) and `interactive/doc-example/` (now via the real
      0007, not `FixtureMigration`) both match their Phase-0 goldens exactly
      — **not blocked on 0195**, since extraction/classification/existence-checking
      are all now built directly in this plan
- [ ] Blocked only on Obligation 2 (self-validation script lifetime, above):
      `self_validate_structural`/`referential` both pass against every
      backfilled/rewritten fixture once 0195's edge resolves which path
      (in-process Rust equivalent, or the kept-alive shell script) applies
- [ ] `mise run cli:check` exits 0

#### Manual Verification:

- [ ] Once landed, run 0007 against a real pre-ADR-0033/0034 `meta/`
      snapshot and manually compare a sample of resolved-band mechanical
      linkages against the bash-produced equivalent

---

## Phase 9: Suite Classification, Assertion Inventory & Black-box Rewrites

### Overview

The item's stricter "every assertion mapped" requirement: a committed
extractor over all six retiring suites, sized against the 400-assertion
threshold that decides whether the exhaustive mapping narrows to the three
suites where irrecoverable corruption lives. This phase can run any time
after Phase 0 (goldens exist) — it does not require 0195, since it's
measuring the *shell* suites, not building 0007.

### Changes Required

#### 1. Assertion extractor

**File**: `tasks/lint/migrate_suite_inventory.py` (new)
**Changes**: modelled on 0167's now-lost `check-inventory.sh`, but durable
(Python, under `tasks/`, unlike bash's disposable script) — walks all six
suites plus the retiring bash source files, extracting every `assert_*` call
site (and, for `scripts/test-interactive-protocol.sh`, every direct
function-level assertion, since it has no CLI surface) keyed by `<file>:<line>`,
classifying each `repointable` (drives the CLI surface, can run unmodified
against the compiled binary via a black-box wrapper) or `not-repointable`
(drives FIFO/fd internals directly, or the `test-migrate-0007.sh:2208` `exec`
stub, or the three awk helpers). Outputs a table matching
`meta/inventories/0167-suite-audit.md`'s shape (file/line-or-suite,
classification, disposition, pinning test). Since this extractor is the sole
gate for the "no gaps, no duplicates" completeness guarantee Phase 10's
irreversible deletion relies on, its own detection logic gets a unit test
before being trusted: a small synthetic fixture corpus (`tasks/lint/tests/fixtures/migrate_suite_inventory/`)
covering each recognised assertion form the six real suites actually use
(`assert_*` calls, `scripts/test-interactive-protocol.sh`'s direct
function-level assertions), plus at least one deliberately-tricky non-match
(a commented-out assertion, an assertion embedded in a multi-line call) —
asserting the extractor finds exactly the expected set, no more, no fewer.

#### 2. Threshold decision

**Changes**: run the extractor once, record the total. If ≤400: exhaustive
mapping applies to all six suites. If >400: narrow to the three suites named
in the AC (`test-migrate-0007.sh`, `test-migrate-interactive.sh`,
`scripts/test-interactive-protocol.sh`) with 0167's remainder-only pattern
for the rest — record the decision and the count in the inventory document,
`meta/inventories/0172-suite-audit.md`. This also settles the sizing
decision the work item deferred to planning: if this measurement, combined
with Phase 10's commit-composition below, cannot produce one indivisible
cutover commit, promote to a parent-with-children along the item's own
documented seam (engine+0001–0006 vs interactive+0007). Given Phases 1–6 are
already independently mergeable ahead of the cutover (Phase 7 is gated on
0195/Phase 8, per Phase 7's own blocking annotation), and only Phase 10
itself must be indivisible, this plan does **not** currently expect that
trigger to fire — but the check is real, not rhetorical, and gets re-run
against the actual measured count before Phase 10 starts.

#### 3. Black-box rewrites

**File**: `cli/migrate-cli/tests/black_box/interactive_protocol.rs`,
`.../fifo_free.rs` (new)
**Changes**: `test-migrate-interactive.sh` and
`scripts/test-interactive-protocol.sh`'s non-repointable assertions rewritten
as black-box tests driving the compiled `accelerator-migrate` binary and
asserting **observable behaviour** (stdout/stderr/exit-code/file-state), not
internals — modelled on `cli/launcher/tests/config_read.rs`'s
`Fixture`/`Workspace` harness plus `vcs-cli`'s real-VCS-state builders. Each
mapped to a named test in the inventory.

#### 4. Repointing the rest

**Changes**: `test-migrate.sh`, `test-migrate-snapshot.sh`,
`hooks/test-migrate-discoverability.sh` (already repointed in Phase 7),
`test-migrate-0007.sh` (blocked on Phase 8's completion) repointed at
`env!("CARGO_BIN_EXE_accelerator-migrate")` in place, absorbing the
`test-migrate-0007.sh:2208` `exec`-stub fix inherited from 0167 first.

### Success Criteria

#### Automated Verification:

- [ ] `tasks/lint/tests/fixtures/migrate_suite_inventory/`'s synthetic
      corpus test passes — the extractor finds exactly the expected
      assertion set against known-form and known-tricky-non-match fixtures,
      run before the extractor is trusted against the real six suites
- [ ] `tasks/lint/migrate_suite_inventory.py` runs in CI, asserts no
      duplicate and no gap against a fresh extraction over every suite and
      retiring file named in Technical Notes
- [ ] The recorded total and threshold decision are committed in
      `meta/inventories/0172-suite-audit.md`
- [ ] Every suite/assertion classified `repointable` is green in CI at a
      recorded commit, before any script it covers is deleted (an actual CI
      run, its commit SHA recorded in the inventory)
- [ ] Every `not-repointable` assertion has a named, passing Rust black-box
      test
- [ ] `mise run` (full local CI mirror) exits 0 with both the bash suites
      (still present) and the new Rust suites green side by side

#### Manual Verification:

- [ ] Spot-review the inventory table for a handful of assertions to confirm
      the repointable/not-repointable classification reads correctly against
      the actual suite source

---

## Phase 10: Retirement Cutover (single commit)

### Overview

**Blocked on Phase 8 (0195) and Phase 9.** The one indivisible commit: delete
every retiring bash/awk file, rewrite every remaining call site, adjust every
guard/floor, retire `jsonl-common.sh` in full, and close out every cross-item
record the AC requires. Everything here lands together — CI must never go
green→red on a floor mismatch or a stray reference.

### Changes Required

#### 1. Deletions

Every file in Technical Notes' "Source bash to retire" list: both driver
scripts, all three awk helpers, both harness/protocol files, all seven
migrations, `hooks/migrate-discoverability.sh`, all six suites, the
non-retained part of `scripts/test-fixtures/interactive/`. Plus, per the
confirmed direction: `scripts/jsonl-common.sh` itself, and the
`atomic_jsonl_remove_by_key` function + its `source jsonl-common.sh` line
from `scripts/atomic-common.sh` (leaving that file's other functions and
their unrelated production callers untouched).

#### 2. Call-site and guard/floor rewrites (same commit)

- `skills/config/configure/SKILL.md:561`'s `bash run-migrations.sh --skip`
  reference → `accelerator migrate --skip`
- `_EXPECTED_MIGRATE_SUITES` removed entirely from `tasks/test/integration.py`
  (an at-least floor covering exactly the four now-deleted migrate-specific
  suites, per the confirmed removal, not decrement)
- `_EXPECTED_CONFIG_SUITES`/`_EXPECTED_HOOKS_SUITES` decremented by one each
  (`scripts/test-interactive-protocol.sh`, `hooks/test-migrate-discoverability.sh`)
- The three `SHELL_LIBRARIES` entries removed from `tasks/lint/scripts.py`
  (`scripts/interactive-harness.sh`, `scripts/interactive-protocol.sh`,
  `skills/config/migrate/scripts/interactive-lib.sh`) — plus, per the
  jsonl-common.sh decision, `scripts/jsonl-common.sh`'s own entry
- `tasks/lint/call_site_migration.py`'s `stray_legacy_flag()` allowlist:
  remove the `skills/config/migrate/migrations/` prefix exemption (directory
  no longer exists) and the `scripts/doc-type-table.sh` exemption **only if**
  0167's owner has agreed the guard itself is retired or updated for a
  bash-migration-engine-free tree — otherwise leave the guard in place with
  its allowlist narrowed (both branches satisfy the AC; resolve with 0167's
  owner before this phase, not during it)
- Static checks asserting zero matches for `session[-_]log` across
  `scripts/hooks/skills/tasks` shell/awk files, zero matches for
  `interactive-harness|interactive-protocol|# INTERACTIVE:|harness_run|harness_reject|migration_validate_edit`,
  and zero matches for `mkfifo` — each with its pre-deletion run committed as
  a known-positive floor per the AC's own wording

#### 3. Documentation

`skills/config/migrate/SKILL.md` and `docs-site/src/content/docs/migrations.md`
(**not** `docs/migrations.md` — that path was relocated as part of the
docs-tree split; there is no `docs/` directory in the current tree) rewritten
to describe authoring an in-crate Rust migration, mapping each
currently-documented bash guarantee to its Rust equivalent one-for-one:
predicate routing (the `evaluate_predicate`/`PredicateOutcome` shape),
the three mandatory display elements, write-ahead-log ordering/resumability
(the `MigrationContext`/`DecisionSource` port contract), sticky skip, source
drift, and callback determinism (ADR-0037 §5's requirement, restated for
Rust — `emit_transformations`/`evaluate_predicate`/`validate_edit`/
`apply_decision` must all be pure and terminating). Explicitly documents
that `ACCELERATOR_MIGRATIONS_DIR` is no longer recognised (compiled-in
registry replaces directory scanning) — a removed-env-var note, not
silence. Manual verification includes running `mise run docs:check`/
`docs:build` after the rewrite (per this repo's own guidance for anyone
touching `docs-site/` — deliberately outside `mise run check`'s aggregate
set, so it must be run by hand).

The worked example's "doctested in CI" requirement (the work item's own AC)
is met by a named successor test, not left aspirational: a
`cli/migrate-cli/tests/skill_doc_worked_example.rs` (new) that ports the
existing `extract_block`-style markdown scraping
(`test-migrate-interactive.sh:986-996`, deleted by this same commit) to pull
the worked example's code blocks out of `SKILL.md` and assert them against
the compiled `accelerator-migrate` binary's actual output — so the example
cannot silently drift once its bash-era verification is gone.

#### 4. 0182 allowlist cleanup

Remove every `CLAUDE_PLUGIN_ROOT`-allowlist entry in 0182's tracked scope
covering files this phase deletes (`skills/config/migrate/**`, the seven
migrations, `scripts/interactive-harness.sh`, `hooks/**`'s now-removed
entries) — coordinate the `hooks.json` entry ordering with 0182's already-landed
index-3 `launcher-link-refresh.sh` entry so neither edit collides.

#### 5. Cross-item records (all before or in this commit)

- **ADR-reconciliation follow-up work item**: create it now (via
  `create-work-item`, referencing ADR-0023, ADR-0037 §5, ADR-0038 against
  the Rust port — specifically that the Rust engine encodes ADR-0038's
  two-band/field-set parameters as ordinary in-crate logic with no
  replacement author-facing API, which ADR-0037 §5's recursive supplement
  clause requires be reconciled formally, not silently) and link it from
  0172 before this commit
- **0195 reciprocal edges — confirm, don't create**: the golden-capture-ordering
  edge (Phase 0, point 0) and the self-validation obligation (Phase 8's
  precondition section — the only remaining 0195 dependency, linkage
  extraction having none per Current State Analysis's correction) must
  already exist on 0195 itself by this point, since both are preconditions
  of the phases that depend on them, not Phase 10 deliverables. This step
  only confirms they are still accurate and closed out — if either was
  never actually recorded
  on 0195, that is a process failure to flag, not something to backfill here
- **0180/0168 session-log-reader question**: settle whether the visualiser
  reads session logs — record a `relates_to` edge on both 0180 and 0168, or
  mark 0180's consumer claim superseded by 0168's record
- **0182 edge**: record the coupling reciprocally on 0182
- **0183**: already closed as abandoned in Phase 7 — confirm the edge is
  bidirectional
- **`--allow-legacy-layout` obligation**: confirm at this point whether
  0167's own Requirements/AC text documents the crate-level
  `with_legacy_policy` capability this plan relies on; if not, record the
  gap rather than treat it as blocking (0167 is closed, so this cannot gate
  0167 itself — it is a documentation completeness note only)

#### 6. `mise run` green

Full local CI mirror run, once, at the end of this commit.

### Success Criteria

#### Automated Verification:

- [ ] Every deletion listed above confirmed absent:
      `find skills/config/migrate/scripts skills/config/migrate/migrations -name '*.sh' -o -name '*.bash' -o -name '*.awk'`
      returns nothing
- [ ] `grep -rn 'session[-_]log' scripts/ hooks/ skills/ tasks/ --include='*.sh' --include='*.bash' --include='*.awk'`
      → 0 matches
- [ ] `grep -rn 'interactive-harness\|interactive-protocol\|# INTERACTIVE:\|harness_run\|harness_reject\|migration_validate_edit\|run-migrations\.sh\|interactive-lib\.sh' scripts/ hooks/ skills/ docs-site/ tasks/ cli/ .claude-plugin/`
      → 0 matches (`docs-site/`, not `docs/` — there is no `docs/` directory
      in the current tree, so this check must scan the actual rewritten
      documentation; `run-migrations.sh`/`interactive-lib.sh` added to catch
      leftover bash-invocation examples in prose, not just internal-API
      vocabulary)
- [ ] `grep -rn 'mkfifo' scripts/ hooks/ skills/ --include='*.sh'` → 0 matches
- [ ] `grep -rn 'ACCELERATOR_MIGRATION_MODE' cli/` → 0 matches (0178's
      negative test in `config-adapters/tests/config_reader.rs` stays green)
- [ ] `_EXPECTED_MIGRATE_SUITES` absent from `tasks/test/integration.py`;
      `_EXPECTED_CONFIG_SUITES`/`_EXPECTED_HOOKS_SUITES` each decremented by
      exactly one; the four `SHELL_LIBRARIES` entries absent
      (three migrate/interactive + `jsonl-common.sh`)
- [ ] `tests/unit/tasks/test_call_site_migration.py` passes against the
      narrowed/removed allowlist
- [ ] `skills/config/migrate/SKILL.md`'s `allowed-tools` coverage verified
      by 0167's permission-coverage check
- [ ] `mise run` exits 0 end-to-end
- [ ] The ADR-reconciliation follow-up work item exists and is linked from
      0172 (`relates_to` or equivalent, verified by
      `accelerator work-item show 0172` or the equivalent read path)
- [ ] `accelerator-migrate` fetch-and-verify test resolves the real signed
      manifest entry at a release-shaped fixture
- [ ] `docs-site/src/content/docs/migrations.md` exists and is rewritten
      (not `docs/migrations.md`, which is confirmed not to exist in the
      current tree)
- [ ] `cli/migrate-cli/tests/skill_doc_worked_example.rs` exists and passes
      — the worked example's code blocks, scraped from `SKILL.md`, match the
      compiled binary's actual output

#### Manual Verification:

- [ ] `mise run` locally on a clean checkout, end to end, watching for any
      unexpected warning noise from the deletions
- [ ] Manually verify the 0182 `hooks.json` index ordering (migrate's
      SessionStart entry and `launcher-link-refresh.sh`'s index-3 entry)
      resolves without collision by inspecting the file directly
- [ ] `mise run docs:check`/`docs:build` pass against the rewritten
      `docs-site/src/content/docs/migrations.md` (deliberately outside
      `mise run check`'s aggregate set per this repo's own convention for
      anyone touching `docs-site/` — run by hand, not via CI)

---

## Testing Strategy

### Unit Tests

Domain logic (`migrate` crate) is tested with no filesystem, no subprocess,
and no bash — ledger/manifest/interactive-engine logic against in-memory
ports and test doubles, table-driven over every documented edge case (unknown
IDs, applied-wins, the four manifest-usability states, source drift, sticky
skip, write-ahead-log ordering).

### Integration Tests

Fixture-golden tests (`cli/migrate-cli/tests/`) drive the compiled
`accelerator-migrate` binary against Phase-0-captured bash goldens, using the
`Fixture`/`Workspace` black-box harness pattern from
`cli/launcher/tests/config_read.rs` plus real-VCS-state builders from
`vcs-cli`'s test-support crate for the guarded-resume/dirty-tree cases.

### Manual Testing Steps

1. Run each phase's fixture set interactively at a real terminal at least
   once, not only through the automated harness, to catch anything a
   byte-comparison assertion wouldn't (prompt readability, copy-paste
   correctness of stall commands).
2. After Phase 10, run a full `/accelerator:migrate` upgrade cycle against a
   scratch repo seeded at a pre-0001 state, end to end, and diff the result
   against a parallel bash-migrated copy.
3. Confirm the SessionStart advisory renders as a system message (not
   stderr) in a live Claude Code session with a pending migration.

## Performance Considerations

The in-process design removes two full process forks per interactive
transformation (bash's dry-fork for `--list`/dry-apply plus the live fork)
and the FIFO/watchdog machinery entirely — this is a strict performance win
with no corresponding regression risk to design around. The mkdir-lock
contention/backoff behaviour (`corpus-adapters::lock`) is already tuned and
tested independently of this plan.

## Migration Notes

No data migration is required — the on-disk artefact formats
(`.accelerator/state/migrations-applied`, `-skipped`, `-run-paths.txt`, and
the session-log JSONL shape) are unchanged; a repo mid-migration under bash
is picked up by the Rust engine via the same paths, with the one-time
bash-session-log cutover rewrite (Phase 4) handling any still-bash-authored
session log transparently on first Rust read.

## References

- Work item: `meta/work/0172-migration-engine-subdomain.md`
- Research: `meta/research/codebase/2026-08-06-0172-migration-engine-implementation-research.md`
- Registration template: `cli/vcs-cli/`, `tasks/README.md#registering-a-dispatched-sub-binary`
- Crate foundation: `cli/config/`, `cli/config-adapters/`, `cli/corpus/`,
  `cli/corpus-adapters/`, `cli/document/`, `cli/vcs/`, `cli/vcs-adapters/`,
  `cli/store/`
- ADRs: ADR-0023, ADR-0037, ADR-0038, ADR-0047, ADR-0052, ADR-0053
- Cross-item obligations recorded on: work item 0195 (self-validation
  primitive only — linkage extraction has no 0195 dependency, per Current
  State Analysis's correction), work item 0183 (absorbed, closed abandoned)
- Bash source (retiring in Phase 10): `skills/config/migrate/scripts/run-migrations.sh`,
  `interactive-lib.sh`, `scripts/interactive-harness.sh`,
  `scripts/interactive-protocol.sh`, `scripts/jsonl-common.sh`,
  `skills/config/migrate/migrations/0001`–`0007-*.sh`,
  the three `*.awk` helpers, `hooks/migrate-discoverability.sh`
