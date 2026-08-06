---
type: codebase-research
id: "2026-08-06-0172-migration-engine-implementation-research"
title: "Research: Migration Engine Subdomain (0172) implementation groundwork"
date: "2026-08-06T08:16:17+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0172"
parent: "work-item:0172"
topic: "Migration Engine Subdomain (0172) implementation groundwork"
tags: [research, codebase, migration-engine, rust-cli, migrate, concurrency, interactive]
revision: "3eff062ce78e622016adc1f663ea4bbcbed17145"
repository: "accelerator"
last_updated: "2026-08-06T08:16:17+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Migration Engine Subdomain (0172) implementation groundwork

**Date**: 2026-08-06T08:16:17+00:00
**Author**: Toby Clemson
**Git Commit**: 3eff062ce78e622016adc1f663ea4bbcbed17145
**Branch**: HEAD (jj workspace `build-system`)
**Repository**: accelerator

## Research Question

Educate an implementation plan for the story at `meta/work/0172-migration-engine-subdomain.md` — porting `skills/config/migrate/` (the bash meta-directory migration engine: driver, FIFO/fd interactive IPC, 7 numbered migrations, author-facing harness) into a native Rust `accelerator-migrate` sub-binary, over the already-landed `config`/`config-adapters`/`corpus`/`corpus-adapters`/`document`/`vcs`/`store` crates.

## Summary

The crate foundation 0172 depends on is **further along than 0172's own prose admits**: `--allow-legacy-layout` has shipped end-to-end (0167), `cli/store` already exists as a standalone crate, `corpus-adapters` already has a working `FileCorpusStore` implementing 0180's canonical-order JSONL compose/remove and the PID-owner mkdir lock, and `ACCELERATOR_MIGRATION_MODE` is confirmed dead with a negative test in place. All five blocking/foundation work items (0167, 0169, 0178, 0179, 0180) are effectively landed, though 0167's and 0164's frontmatter/body status text disagree (frontmatter `done`, body still reads "Ready"/"In Progress") — read frontmatter as authoritative, corroborated here by a merged-PR description (`meta/prs/26-description.md`) for 0167.

The bash implementation's lifecycle, FIFO/fd IPC, harness API, and all 7 migrations (0001–0007) have been read in full and are documented below with file:line precision, sufficient to drive a flag-for-flag, behaviour-for-behaviour Rust port without re-reading the bash source during planning.

A concrete registration template exists: `accelerator-vcs` (0169's sub-binary) is a full worked example of the sub-binary crate shape, the `cargo-pup` rule pattern, the golden/parity-test structure (captured bash fixtures + `#[cfg(feature = "bash-parity")]`-gated Rust replay tests), and the thirteen-point registration checklist in `tasks/README.md`. 0167 additionally left three `meta/inventories/*.md` documents as a template for the assertion-inventory 0172's stricter "every assertion mapped" requirement demands — but 0167's own extractor script did not survive its own cutover commit, so 0172 must build a durable one rather than reuse 0167's.

Three factual corrections are needed in 0172's own text before/during planning:
1. **0173 is abandoned**, split 2026-08-05 into 0195/0196/0197; the golden-capture ordering constraint and `scripts/validate-corpus-frontmatter.sh` deletion now belong to **0195**, which has zero awareness of 0172 today. This is a live, unguarded race.
2. **`hooks/shim-refresh.sh` → `hooks/launcher-link-refresh.sh`** — 0182 shipped the index-3 SessionStart hook under a different name than 0172's Dependencies section states.
3. **The confinement guard is `tasks/lint/call_site_migration.py`**, not `tasks/lint/skill_permissions.py` as 0172's Requirements state — the latter exists but checks something unrelated (SKILL.md `allowed-tools`/injection census); `call_site_migration.py` is the actual `--allow-legacy-layout` confinement + retired-script-reference check, tested by `tests/unit/tasks/test_call_site_migration.py` (which 0172 correctly names).

The ADR-reconciliation follow-up work item 0172's own acceptance criteria require does not yet exist — it must be created before the deletion commit, per 0172's own text.

## Detailed Findings

### 1. Bash migration engine — lifecycle contract (`run-migrations.sh`, `interactive-lib.sh`)

Full file reads: `skills/config/migrate/scripts/run-migrations.sh` (687 lines), `skills/config/migrate/scripts/interactive-lib.sh` (984 lines).

**Setup and paths** (`run-migrations.sh:4-32`): `PLUGIN_ROOT`/`PROJECT_ROOT` resolution; state paths — applied ledger `.accelerator/state/migrations-applied`, skip list `.accelerator/state/migrations-skipped`, per-run path manifest `.accelerator/state/migrations-run-paths.txt` (`RUN_PATHS_FILE`), its run-id sidecar `.accelerator/state/migrations-run.id` (`RUN_ID_FILE`).

**Clean-tree pre-flight** (`run-migrations.sh:294-405`), gated on `[ -z "$LIST_MODE" ] && [ -z "${ACCELERATOR_MIGRATE_FORCE:-}" ]` (`:316`) so **`--list` skips the entire block**, confirmed exactly as prior notes described (comment `:311-315`). `enumerate_scoped_dirty()` (`:154-166`) is the single dirty-scan source of truth — jj: `jj --no-pager diff --name-only` filtered to `^(meta/|\.claude/accelerator|\.accelerator/)`; git: `git status --porcelain` over the same three roots, excluding untracked. Dirty-but-owned → guarded resume (`RESUME=1`); dirty-and-not-owned-but-has-in-flight-log → structured resume/discard scaffold, `exit 1`; otherwise `refuse_dirty_tree()` (`:212-218`, prints the `ACCELERATOR_MIGRATE_FORCE=1` hint), `exit 1`.

**State reading / discovery / preview** (`:407-573`): unknown-ID preservation with a warning (`:436-456`); applied-wins warning when an ID is in both ledgers (`:459-466`); migration discovery via `find "$MIGRATIONS_DIR" -maxdepth 1 -name '[0-9][0-9][0-9][0-9]-*.sh' -print0 | sort -z` (`:422-428`), `ACCELERATOR_MIGRATIONS_DIR` overridable; preview banner `"About to apply N migration(s):"` then `"  <id> — <description>"` (description from `# DESCRIPTION:` header line) plus a per-migration `"    To skip: bash $0 --skip $id"` hint (`:557-573`); `No pending migrations.` early exit prints skipped-IDs if any, calls `clear_run_manifest()`, `exit 0` (`:546-555`).

**Apply loop** (`:618-669`): sorted-glob order; mechanical migrations run with `ACCELERATOR_MIGRATION_MODE=1` exported, stdout+stderr captured; on failure, `manifest_record_delta` still runs first to capture partial writes, then `exit 1` (whole run aborts, no continuation). On success: `atomic_append_unique "$STATE_FILE" "$id"` (idempotent temp-file+rename append). **`MIGRATION_RESULT: no_op_pending` sentinel** (`:651-663`): detected via `grep -qx`, stripped from user-visible output via `grep -v -x`, migration stays pending (no ledger append), `continue`. Interactive migrations dispatch to `run_interactive_migration` and are deliberately excluded from the mechanical run-paths manifest (comment `:625-628`) — their resumability is governed entirely by the session log. Post-loop: `clear_run_manifest` + baseline-file removal only on a clean full completion (`:671-675`). **Summary**: `"applied: N"` + optional `"; skipped: <ids>"` + optional `"; pending (no-op): M"` + `"Migration complete. $SUMMARY."` (`:677-687`).

**CLI flags** (`:39-121`), unknown-arg rejection enforced (`:115-119`, "was previously silently ignored" per comment `:37`): `--skip <id>` / `--unskip <id>` / `--unapply <id>` each `atomic_append_unique`/`atomic_remove_line` + `exit 0`; `--decisions-file <path>` sets/exports `ACCELERATOR_MIGRATE_DECISIONS_FILE` and **falls through** to a normal run (no exit); `--list` sets `LIST_MODE=1`, falls through; `--help`/`-h` prints to **stdout** (deliberate convention deviation from error-path stderr, comment `:85-87`), documents all flags + the env var, `exit 0`. Decisions-file validated (directory / non-existent / unreadable) regardless of source (`:123-140`).

**`--list` body** (`:494-543`): emits `<pos>\t<key>\t<proposed>\t<path>:<anchor>` lines, segmented by `# migration <id>` headers with position restarting at 1 per migration when >1 pending interactive migration exists (`:530`), `"no pending transformations"` if nothing pending, always `exit 0`, never mutates.

### 2. FIFO/fd IPC and watchdog mechanics (`interactive-lib.sh`)

Two named FIFOs (`migrations-<id>-r2m.fifo`, `migrations-<id>-m2r.fifo`) under `.accelerator/state/`. `exec 7<>"$fifo_r2m"` opens read-write first to avoid the classic FIFO-open-blocks-until-both-ends-open deadlock (`:736-740`); child forked with `<fifo_r2m >fifo_m2r 2>stderr_file &` (`:747`); `exec 8<"$fifo_m2r"` opened read-only after fork (`:753`). Header comment (`:1-5`): "bash 3.2 compatible (no coproc, no associative arrays)". Main loop `while IFS= read -r -u "$mig_out" frame` (`:771`) parses `type\tfield1\tfield2...` by hand via `IFS=$'\t'` + per-field `unescape_field`, `case`-dispatching on 15+ frame types (`:808-927`). Both fds explicitly closed and FIFOs unlinked on completion (`:930-932`).

**Watchdog** (`:937-954`): background subshell, `sleep 30` then `kill -0`; still-alive → `SIGTERM`, 1s grace, re-check, `SIGKILL` if still alive — strictly sequential, no retry loop. Parent does a blocking `wait "$pid"`, then kills+waits the watchdog subshell itself so it never outlives the parent.

**Shared "dry-fork" plumbing** (`_interactive_fork`, `:403-515`), used by `--list` (mode 1) and the decisions-file dry-apply validation pass (mode 2), **not** the live run: same FIFO pattern but child's fd 7 is explicitly closed inside the child (`7<&-`) so it sees EOF on abort; no watchdog (`_interactive_teardown` synchronously `kill`s + `wait`s since a dry child never mutates); mode passed via env var `MIGRATION_HARNESS_MODE` (not a positional field, to avoid tab-split collapse); protocol-log destinations are parameterised per-caller so dry passes never pollute live-run frame-count test assertions.

**Predicate routing** (documented in `SKILL.md:98,143,151` per ADR-0037 §1; runner-side consumption `interactive-lib.sh:833-835,921-926`): exit 0 → `prompt`; exit 1 → `mechanical` (no record); other non-zero → `fail`, abort whole run.

**Dry-apply validation pass** (`dry_apply_interactive_migration`, `interactive-lib.sh:583-607`, run before the live loop for every pending interactive migration when a decisions file is set): position-named errors for missing decisions (`:614-615`), unknown verb (`:621-622`), rejected-edit-with-no-recovery-line (`:634-635`), unknown decision outcome (`:648-651`), and surplus decisions (`:600-604`); sends **no** `APPLY` frame — the child never durably mutates during dry-apply, so any error propagates to `exit 1` before the live loop starts.

**Resume/staleness** (`run-migrations.sh:186-198`): jj `change_id` (stable across working-copy edits, unlike the content-hash `commit_id`) vs git `HEAD`. **Ownership** (`dirty_tree_fully_owned`, `:257-292`): usability gate (run-id sidecar readable+non-empty; manifest readable but **may be empty** — an interactive interrupt before any mechanical delta leaves one empty, `:261-265`); staleness = recorded vs current base revision mismatch; **three ownership classes** — (a) runner-managed bookkeeping files (ledger/skip/manifest/run-id, implicit), (b) current-run interactive session artefacts matched **by pattern** (`is_session_artifact`, `:240-247`, gated on base-revision equality so a stale run's artefacts aren't owned), (c) everything else must appear in the manifest verbatim. `ACCELERATOR_MIGRATE_FORCE` skips the **entire** pre-flight block (`:316`), so a FORCE run always mints a fresh run-id and truncates the manifest.

**Sticky skip / source drift** (`SKILL.md:158,160`; runner-side `interactive-lib.sh:905-916`): skip outcomes replay as `RESUMED_SKIPPED` without re-predicate/re-prompt; a `DRIFT` frame triggers `atomic_jsonl_remove_by_key` on the stale record then a `DRIFT_CLEARED` reply so the transformation re-enters the normal predicate/prompt path as undecided.

### 3. Interactive harness API and the 7 migrations

Full reads: `scripts/interactive-harness.sh` (688), `scripts/interactive-protocol.sh` (169), `skills/config/migrate/migrations/0001`–`0007-*.sh` (2,632 total), the three awk helpers, `scripts/jsonl-common.sh`.

**Author-facing lifecycle hooks** (migration-implemented, called by `harness_run`): `migration_emit_transformations` (`interactive-harness.sh:282`); `migration_evaluate_predicate` (`:380-397`, here-string not pipe to avoid SIGPIPE misclassification); `migration_validate_edit` (`:600-606`); `migration_apply_decision` (`:325,577,616`); optional `migration_session_log_path` (`:268-272`, defaults to `.accelerator/state/migrations-${MIGRATION_ID:-unknown}-session.jsonl`) and `migration_verify_applied` (`:485-489`, drives resume-drift detection).

**Author-facing helpers**: `harness_emit_transformation key= path= anchor= proposed= predicate_value= display=` (`:109-176`, base64-encodes `display`, prints one `TX\t...` line); `harness_extras_set`/`harness_extras_clear` (`:81-107`, rejects the six reserved field names); `harness_field` (`:182-215`); `harness_reject <message>` (`:220-223`, prints `[interactive] <message>` to stderr, returns 1); `harness_run` (`:244-340`, the top-level driver: INIT/READY handshake, resume-state load, list/dry-apply mode branch, per-TX classify-and-route, final `DONE`).

**Migrations 0001–0006** (all non-interactive): 0001 renames tickets→work-item vocabulary (frontmatter + dir + config keys); 0002 renames legacy IDs to a `{project}`-prefixed pattern with a no-op-pending short-circuit and three-pass cross-reference rewrite; 0003 relocates `.claude/`/`meta/`-owned files into `.accelerator/` with a preflight no-op gate and pinned-path warnings; 0004 splits `meta/research/` into `codebase`/`issues` subcategories with mixed-state rejection and boundary-aware inbound-link rewriting; 0005 is the simplest — renames frontmatter `type:`→`kind:`; 0006 canonicalises `work-item:`→`work_item_id:` and `researcher:`→`author:` with divergence/refuse diagnostics, later parity primitives extracted into `frontmatter-frag.awk` for 0007.

**Migration 0007** (`0007-unify-meta-corpus-frontmatter.sh`, 856 lines, the only interactive one): transformation `key` is `<file>#<section-anchor>` (not an enumerated link-A/B/C scheme); `proposed` is `<linkage-key>=<typed-ref>`; predicate is `band == "ambiguous"` (resolved band applies mechanically). `self_validate_structural` (`:610-623`) runs **before** `harness_run` against the deterministic backfill output; `self_validate_referential` (`:624-629`) runs **after**, covering interactive-apply writes too — both shell out to `scripts/validate-corpus-frontmatter.sh`. `doc-type-table.sh`'s `--allow-legacy-layout` append is gated on `ACCELERATOR_MIGRATION_MODE=1` (`scripts/doc-type-table.sh:41-45`) — the "one shared helper the confinement check allowlists to carry `--allow-legacy-layout`."

**`jsonl-common.sh`** (`scripts/jsonl-common.sh`): `jsonl_json_escape` (`:21-50`, order-sensitive: backslash → quote → named escapes → per-byte `\u00XX` for remaining control chars). `jsonl_compose_record` (`:66-149`) emits in **fixed declaration order regardless of caller arg order**: `transformation_key`, `schema_version`, `outcome`, `proposed_value` (unconditional), `user_value` (conditional on `has_user_value=1`), `timestamp`, then extras in passed order — matching 0180's canonical order exactly. **Confirmed gap**: the doc comment at `:60` documents `proposed_value` as required, but the actual emptiness guard at `:115-116` omits it — an empty/absent `proposed_value` passes silently in the bash writer (0180's Rust port fixes this per its AC-8). Only production caller besides `interactive-lib.sh`'s `write_session_record` is `atomic-common.sh:95`'s `atomic_jsonl_remove_by_key` sourcing.

### 4. Landed Rust crate foundation — further along than 0172's prose states

**`--allow-legacy-layout` has already shipped**, not pending: `config-adapters/src/store.rs:29-33` defines `pub enum LegacyPolicy { Reject, Allow }`; `FileConfigStore::with_legacy_policy` wires it; the launcher's clap surface exposes it as a real per-subcommand flag on ~14 read actions (`cli/launcher/src/launch/inbound/cli.rs:56,94,106,...`); black-box tests exist (`cli/launcher/tests/config_read.rs:435,1853`). Pure predicate: `config/src/legacy.rs:6` `is_blocked(team_present, legacy_present) -> bool`.

**`ACCELERATOR_MIGRATION_MODE` confirmed dead** with a negative test: `config-adapters/tests/config_reader.rs:67` `a_legacy_layout_fails_closed_under_migration_mode` sets the env var and asserts the Rust reader **still fails closed** — only `--allow-legacy-layout` bypasses.

**`cli/store` already exists as a standalone crate** (0167's carve-out has landed): `cli/store/src/lib.rs` exposes `atomic_write`, `ensure_contained`, `read_within`, `WriteBounds`, `NewFileMode`, `WriteError`; both `corpus-adapters` and `config-adapters` depend on it as a path dependency rather than embedding their own writer.

**`corpus-adapters/src/store.rs`** implements 0180's deliverable via `FileCorpusStore` (public) composed from private `jsonl.rs` + `lock.rs` modules. Ports (`corpus/src/store.rs`): `AtomicWrite` and `RecordStore { append_record, remove_by_key }` traits, `StoreError` enum. `Record` domain type (`corpus/src/record.rs:26`): `{ transformation_key, schema_version, outcome: Outcome, proposed_value, user_value: Option<String>, extras: Vec<(String,String)>, timestamp }`. `jsonl.rs`'s `compose_record`/`remove_prefix` and `lock.rs`'s mkdir-lock/PID-reclaim are **crate-private** — only `FileCorpusStore` is public; a migration engine needing lower-level access would need either an upstream `pub` change or to depend on `corpus-adapters` and use `FileCorpusStore` as-is (the expected path).

**`document`** (no adapter split, infra-free besides serde/serde-saphyr): `parse`/`render`/`fence_offsets`/`split`, `Yaml`/`Scalar`/`Mapping` value tree. `render` is a **structural rewrite** (re-serialises the whole frontmatter tree), not line-preserving. A narrower **line-preserving** single-field rewrite already exists but is `status:`-specific and not parameterised over key name: `corpus-adapters/src/patcher.rs:48` `patch_status`.

**`vcs`/`vcs-adapters`**: `vcs::RepoFacts { root, name, kind: VcsKind, revision: Option<String> }` — unified revision field, jj working-copy commit id or git `HEAD` depending on `kind`. Two `VcsProbe` impls: subprocess-based `CommandProbe` (default, used by `vcs_adapters::facts`) and a fully in-process `InProcessProbe` (`gix` for git, `jj-lib` directly for jj — no subprocess) available for direct use but not wired into the crate's default `facts()`. A migration engine's resume/staleness check can use either.

### 5. Sub-binary registration — concrete `accelerator-vcs` template + the 13-point checklist

**Crate shape** (`cli/vcs-cli/`): `Cargo.toml` — `name = "accelerator-vcs"`, mandatory `description`, `[[bin]] name = "accelerator-vcs" path = "src/main.rs"`, all version/edition/rust-version/license/publish fields `.workspace = true` (never hardcoded). Workspace member added to `cli/Cargo.toml`. `main.rs` matches on subcommand, dispatches to thin `run_*` functions, shared `report()` maps `kernel::Error::Refusal` → exit 2, else `ExitCode::FAILURE`.

**`cargo-pup` rule pattern** (`cli/pup.ron`): a `Module` rule matching the domain crate's module path with `RestrictImports { allowed_only: [...std/core/alloc, kernel::Error, crate], denied: None }` — plus a sibling rule for an infra submodule adding an explicit `denied: Some(["^std::process(::|$)"])`. This second pattern (allow-list + explicit deny) is the direct template for 0172's own "no FIFO, no child-process spawn" cargo-pup requirement.

**`kernel::hooks` module** (`cli/kernel/src/hooks.rs`): `session_start(context, system_message: Option<&str>) -> String` (`:9-23`, merges `systemMessage` into the SessionStart envelope when present); `pre_tool_use_deny`/`pre_tool_use_warn` (`:27-44`); `adapter_failure` (`:52-54`, same wire shape as `pre_tool_use_warn`, kept distinct for call-site clarity); private `json_escape` (`:60-85`, RFC 8259). Used by `vcs detect`/`vcs guard` (`cli/vcs-cli/src/detect.rs:191-214`, `guard.rs:39-80`) exactly as 0172's ported `migrate-discoverability` should use it.

**`swallow_under_fail_safe`** (`cli/launcher/src/launch/core.rs:219-224`): `forwarded_fail_safe(args) && matches!(error, kernel::Error::Failed(_))` — an explicit allowlist (availability failures only, never integrity/`Refusal` failures), generic to any `Command::External` token, applied at the launcher's exit-code boundary (`main.rs:218-229`). `accelerator migrate ... --fail-safe` gets this fail-open guarantee for free once `migrate` is a registered dispatch token — no migrate-specific wiring needed.

**Current `hooks/hooks.json`** (full file quoted by the sub-agent): four SessionStart entries (`vcs detect`, `config summary`, **`migrate-discoverability.sh`** — still the raw `.sh` path, unconverted — and `hooks/launcher-link-refresh.sh` at index 3) plus one PreToolUse (`vcs guard`). The three already-converted entries are verbatim `${CLAUDE_PLUGIN_ROOT}/bin/accelerator <token> ... --format=hook --fail-safe` command strings — the pattern 0172's edit must follow.

**`hooks/migrate-discoverability.sh`** (73 lines): gates on repo looking Accelerator-managed; picks `.accelerator/state/migrations-applied` if that file exists, else legacy `meta/.migrations-applied`; computes highest-available vs highest-applied migration ID by lexicographic max; prints a stderr advisory (`[accelerator] ... Run /accelerator:migrate to bring it up to date.`) when behind; always exits 0. Its suite `hooks/test-migrate-discoverability.sh` (106 lines) is directly repointable at the compiled binary (pure hook-invocation + exit-code/stdout-substring assertions).

**The 13-point registration checklist** (`tasks/README.md:304-441`, reconstructed in full by the sub-agent): (1) `DISPATCHED_SUBBINARIES` in `tasks/shared/paths.py` + the `test_github.py` upload-count/fixture updates; (2) `_SUBBINARY_MANIFESTS` in `tasks/manifest.py` only if the crate isn't at `cli/<token>/`; (3) crate `Cargo.toml` shape as above; (4) workspace member + regenerated `Cargo.lock`; (5) `.gitignore` cache pattern; (6) `cli/launcher/tests/fixtures/manifest.example.json` — optional, but if touched, its **two co-readers** (`tests/unit/tasks/test_manifest_contract.py:16`, `cli/launcher/src/launch/outbound/resolve/manifest.rs:135`) must move together; (7) skill binding (a `Bash(...)`-scoped invocation in a SKILL.md, or `SKILL_EXEMPT_SUBBINARIES`) — **points 1 and 7 must land in the same change** (pre-release-gated dispatch-coherence guard); (8) `_CLI_RELEASE_BINARIES` in `tasks/build.py`; (9) `attest-build-provenance` blocks in CI — no action if staged only under `dist/release/`; (10) `BUILTIN_SUBCOMMANDS` lockstep — no action for a pure dispatch-token addition; (11) documentation (optional/author-judgement); (12) `DEBUG_ARCHIVE_DIRS` only if shipping symbolication; (13) `cli/deny.toml` only if `deny:check` reddens. Token naming: `^[a-z][a-z0-9-]*$`, no underscores (collides with `ACCELERATOR_<TOKEN>_BIN` derivation).

### 6. Guard/test infrastructure — one misattribution in 0172's own text

`_EXPECTED_MIGRATE_SUITES = 4` (`tasks/test/integration.py:34`, used at `:397-403` via the shared `_require_suite_floor` at-least-floor helper `:77-101`) covers exactly the four suites under `skills/config/migrate/` (`test-migrate.sh`, `test-migrate-0007.sh`, `test-migrate-interactive.sh`, `test-migrate-snapshot.sh`) and should be **removed entirely** at retirement (0172's own Technical Notes call this correctly). The other two retiring suites (`scripts/test-interactive-protocol.sh`, `hooks/test-migrate-discoverability.sh`) count against `_EXPECTED_CONFIG_SUITES`/`_EXPECTED_HOOKS_SUITES` respectively and must be **decremented**, not removed — worth an explicit plan line since 0172's prose doesn't distinguish these two floors from the migrate-specific one.

`SHELL_LIBRARIES` (`tasks/lint/scripts.py:18-49`) currently still lists all three migrate/interactive sourced-only libraries (`scripts/interactive-harness.sh:33`, `scripts/interactive-protocol.sh:34`, `skills/config/migrate/scripts/interactive-lib.sh:37`) — the `exec_bits` guard (`:97-133`) enforces a **stale-entry check**: a listed path that no longer exists on disk fails the guard, so removing the source files *requires* removing these three entries in the same change.

**Misattribution to correct**: 0172's Requirements text (line 253) names `tasks/lint/skill_permissions.py` as the Python confinement guard replacing `check-call-site-migration.sh`. The actual successor is **`tasks/lint/call_site_migration.py`** (confirmed by its own docstring and by `stray_legacy_flag()` at `:43-66`, which confines `--allow-legacy-layout` usage to `skills/config/migrate/migrations/` and the single allowlisted `scripts/doc-type-table.sh`), tested by `tests/unit/tasks/test_call_site_migration.py` (which 0172 does correctly name — only the module name is wrong). `tasks/lint/skill_permissions.py` is a real, different guard (SKILL.md `!`-preprocessor / `allowed-tools` coverage checker) with no reference to `migrate` or `--allow-legacy-layout` anywhere in it.

**Six retiring suites — structural characterisation** (all line counts confirmed exact): `test-migrate.sh` (2593, 481 `assert_*` sites, CLI-invocation-shaped, mostly repointable), `test-migrate-0007.sh` (2229, 179 `assert_*` sites, mostly repointable but contains the noted `exec` stub at line 2208 hard-coding a config-resolver wrapper), `test-migrate-interactive.sh` (2081, 239 assertion/frame sites, drives via env-var + a protocol side-log rather than raw FIFO/fd calls but still inspects frame-count internals — one of the two suites 0172 correctly flags as needing a black-box rewrite), `test-migrate-snapshot.sh` (159, pure golden/snapshot diffing, structurally simplest to repoint), `scripts/test-interactive-protocol.sh` (108, unit-tests the wire-protocol helper functions directly with no CLI surface — the other suite needing a black-box rewrite, since the functions under test disappear entirely with the FIFO/fd layer), `hooks/test-migrate-discoverability.sh` (106, directly repointable).

**0167's assertion-extractor precedent does not survive in the tree.** The 337-assertion/6,289-line figure is real (`meta/work/0167-...md:815`, plan `meta/plans/2026-07-19-0167-....md:1566-1589`), and the mechanism (`scripts/check-inventory.sh`) ran and passed per `meta/validations/2026-07-19-0167-....md:68-70`, but neither that script nor two of the three inventory documents it validated survive today — only `meta/inventories/0167-removal-set.md`, `0167-suite-audit.md`, and `0167-divergences.md` remain (glob-confirmed). 0172 must build its own durable extractor rather than reuse 0167's; the `0167-suite-audit.md` table shape (file/line-or-suite, classification, disposition, pinning test) is the reusable part.

### 7. Golden/parity test structure — concrete templates from `accelerator-vcs`

Two committed golden-capture mechanisms exist as direct templates for 0172's "bash baselines captured as goldens, first ordered step" requirement:
- **Bash-capture script**: `hooks/test-fixtures/vcs-detect/regenerate.sh` drives the real bash hook against constructed jj/git states and writes stdout straight into committed JSON fixtures, plus a `CAPTURE-SOURCE.txt` pinning the exact bash-source revision, capture timestamp, and host.
- **Masked comparison table**: `hooks/test-fixtures/masks.toml` (shared by the Python capture side and `cli/vcs-test-support/src/masks.rs` on the Rust side) — each mask pattern carries its own `sample_match`/`sample_no_match`; the file states explicitly: "do not add a pattern, or loosen an existing one, to make a failing golden pass."

**Rust replay tests**: `cli/vcs-cli/tests/detect_goldens.rs` and `status_log_goldens.rs`, both `#![cfg(feature = "bash-parity")]`-gated, build real jj/git checkouts in a tempdir via a `Hermetic` test-support wrapper (`cli/vcs-test-support/src/hermetic.rs`), invoke `env!("CARGO_BIN_EXE_accelerator-vcs")`, and assert byte-exact parsed-output equality against the fixtures, collecting failures across all states before failing the whole test (richer diagnostics than fail-fast).

**Black-box CLI test scaffolding template**: `cli/launcher/tests/config_read.rs` — `Fixture`/`Workspace` helpers, a `run_in`/`fixture.run()` wrapper spawning `env!("CARGO_BIN_EXE_accelerator")` with a scrubbed environment, byte-exact `output.stdout`/`output.stderr`/exit-code assertions (explicit discipline against `from_utf8_lossy`). This — combined with `vcs-cli`'s real-VCS-state builders for the guarded-resume/dirty-tree tests — is the closer template for 0172 than config's now-pruned in-process differential parity harness (`cli/config-adapters/tests/parity.rs`, which shelled out live to bash during 0167's migration and was cut down to fixed-value assertions once bash was deleted — itself a useful precedent for how a parity harness's shape changes across a migration's lifecycle).

### 8. Work item / dependency status — corrections needed in 0172

| Item | Frontmatter status | Note |
|---|---|---|
| 0167 | `done` | Body still reads "Ready" (stale); corroborated done by merged PR `meta/prs/26-description.md` (pr #26, revision `4af9f104c3153a6801518e43a735c6177d16d47c`) — removal-set deletion, `check-inventory.sh`/`check-call-site-migration.sh`/`check-skill-permissions.sh` all green at merge. **0172's own Dependencies text ("blocked by 0167 (ready, not done)") is stale and should be updated.** |
| 0169 | `ready` | Two open ACs remain (musl cross-compile verification, "release precedes rewrite" gate) but the hooks-migration deliverables 0172 needs are `[x]` complete. |
| 0178, 0179, 0180 | `done` | Foundation crates fully landed. 0180 carries an explicit reciprocal `relates_to: [0172]` with a binding clean-cutover obligation (below). |
| 0187 | `done` | Registration-surface generaliser; 0172 already correctly carries `blocked_by: work-item:0187`. |
| 0164 | `done` (frontmatter) | Body still says "In Progress," all body ACs unchecked — frontmatter is authoritative per 0172's own citation. |
| 0116, 0119, 0115 | `done` | Contracts 0172 preserves; see §9 below for precision gaps. |
| 0182 | `done` | Ships `hooks/launcher-link-refresh.sh` at index 3, **not** `hooks/shim-refresh.sh` as 0172:719-721 currently states. `0182`'s `relates_to` still omits 0172 (confirmed unchanged). |
| 0183 | `draft` | Not landed. `vcs-detect`/`config-detect` are already `systemMessage`-compliant (0169 landed them that way); **`migrate-discoverability` is the one hook still needing the stderr→`systemMessage` conversion** — 0183's `relates_to` still omits 0172. |
| **0173** | **`abandoned`** | **Split 2026-08-05 into 0195 (`accelerator-corpus`), 0196 (`accelerator-design`), 0197 (`accelerator-collaboration`)**. `scripts/validate-corpus-frontmatter.sh`'s deletion now lives in **0195** (confirmed present in `meta/work/0195-....md:67,83,94,125`), which has **zero mention of 0172 or the golden-capture ordering constraint** anywhere in its text. 0195 is `status: ready`, `last_updated` six minutes after 0172's own last update, with no blockers recorded against this concern — **the golden-capture race 0172's own AC is meant to prevent is currently live and unguarded** against an item 0172 doesn't reference. 0136 (the epic) already documents the 0173→0195/0196/0197 split; 0172 does not. |
| 0136 | `in-progress` | Epic still lists 0172 in the same unchanged slot in its decomposition. |

**Corrective actions this implies for planning**: retarget every reference to `work-item:0173` in 0172's Requirements/AC/Dependencies/References concerning the golden-capture ordering constraint to `work-item:0195`, and add the reciprocal edge to 0195 (not 0173, which is inert). Correct the `shim-refresh.sh`→`launcher-link-refresh.sh` naming. Update the "0167 (ready, not done)" framing.

### 9. Precision gaps between 0172's paraphrase and its source contracts (0187/0164/0116/0119/0115)

- **0187**: the "bound" quantifier is precise — a token is bound if *at least one* skill invokes it with a satisfying rule; other skills invoking the same token aren't checked. `SKILL_EXEMPT_SUBBINARIES` is the exemption path if no skill directly invokes `accelerator migrate`. Points 1 and 7 of the checklist must land together (pre-release dispatch-coherence gate).
- **0164**: verification happens **twice** — on fetch and again before every exec (not cached-forever); a failure names which check failed and leaves no cache entry for the failed binary while any pre-existing verified entry for that name+version is left intact. `ACCELERATOR_<SUB>_BIN` env override skips fetch/verify entirely for local dev.
- **0116**: **two independent stall-emission sites** exist historically — the initial PROMPT-frame path (`interactive-lib.sh:450`, old message `failed to obtain decision`) and the VALIDATE_ERR re-prompt path (`:485`, old message `failed to obtain re-decision`) — both need their own Rust-port test case, not just the initial-prompt scenario 0172's AC currently implies. The "list all pending keys" behaviour is explicitly **best-effort, not a firm contract** per 0116's own AC notes — 0172's AC phrasing ("naming the pending decision keys," unqualified plural) should be checked at plan time against whether it intends to strengthen this into a hard requirement.
- **0119**: left the manifest filename "to be fixed at planning" — 0172's own Technical Notes (having inspected the shipped code) are the more authoritative source for the concrete paths/three-ownership-class breakdown, which do **not** appear in 0119's own text at all. Trust 0172's findings over 0119's prose here.
- **0115**: Fix C (0118, reconciling the 0007 backfill "pending" sentinel with `self_validate_structural` for non-derivable required extras like `pr_number`) is **not mentioned anywhere in 0172** — worth an explicit open question about whether the Rust 0007's self-validation path needs equivalent sentinel-tolerance. Fix D (splitting 0007 into mechanical+human-run steps) was explicitly **rejected** — good citation if anyone questions why 0007 stays a single migration. The `#`-comment-line tolerance and the exact `--list` tab-delimited wire format that 0172 specifies are **not sourced from 0115/0116/0119** in this set — likely from 0117 (not read here) or direct code inspection; worth a quick check of `meta/work/0117-*.md` if it exists, to confirm provenance.

### 10. ADR constraints

- **ADR-0023** (meta-directory migration framework, accepted, not superseded): five-step lifecycle (pre-flight → read state → preview → apply-in-order with atomic append → summary), no rollback (VCS revert only), pinned-path preservation (rename only when resolved path matches plugin default; both-exist → abort touching neither), `ACCELERATOR_MIGRATE_FORCE` scoped to the clean-tree check only. Idempotency is "belt and suspenders" — both the ledger filter AND each migration's own no-op self-detection are required in the Rust port.
- **ADR-0037** (optional interactive contract, accepted, supplement not edit): §1 trigger predicate (boolean over a named field set, ≥1 confidence field); §2 mandatory display elements (proposed transformation, source location, predicate's evaluated value); §3 session-log resumability with the write-ahead-log invariant (append before mutation, migration ID appended to the ledger only on all-decided); §4 accept/edit/skip verbs; **§5 recursive supplement clause** — any new framework primitive the Rust port introduces must be routed through a new supplementary ADR, not introduced ad hoc.
- **ADR-0038** (0007's concrete parameterisation of ADR-0037, accepted): two-band model only (`resolved`/`ambiguous` — three-band rejected as statistically unjustified, 88% vs 90%); hybrid trigger (`band == ambiguous`); field set `{band, inferred_key, inferred_target, artifact_path, source_anchor}`; session-log path/format and `(artifact_path, source_anchor)` resume matching. Since 0172 retires the harness with no replacement API, the Rust 0007 encodes these parameters as ordinary in-crate logic — exactly the reconciliation gap the still-missing follow-up work item must cover.
- **ADR-0047** (multi-level userspace config, accepted, supersedes 0016/0017): the compiled CLI is the native reader; this is *why* 0172 routes legacy access through `--allow-legacy-layout` rather than reintroducing `ACCELERATOR_MIGRATION_MODE`.
- **ADR-0052** (filesystem as message bus/corpus, accepted, supersedes 0027): architectural justification for the `meta/` vs `.accelerator/` split that shapes 0172's clean-tree pre-flight scope.
- **ADR-0053** (thin CLI over hexagonal core, accepted): domain core must depend on zero infrastructure — no direct fs/serde_json/process calls, all expressed as outbound ports satisfied by adapters at a composition root. 0172's own "recording store port" AC and cargo-pup FIFO/spawn-ban requirement are direct instantiations of this ADR's mechanical enforcement model.
- **The ADR-reconciliation follow-up work item 0172's own AC requires does not exist.** Confirmed via grep across `meta/work/` — only 0172 itself and its review document reference the ADRs in this context. 0172's review history (`meta/reviews/work/0172-migration-engine-subdomain-review-1.md:1339,1066-1068`) shows this was escalated from an Open Question into a formal criterion across review passes but the actual work item was never created. **Must be created and linked before the deletion commit.**

### 11. Prior architecture research — residual, non-duplicated guidance

Both `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md` and the earlier `2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md` are almost fully absorbed into 0172's own text already. Two residual points:
- The earlier doc characterises `interactive-protocol.sh:9-59` as a **"19-frame TSV state machine"** — this exact framing/count doesn't appear anywhere in 0172 or the later architecture doc. Worth reading that file section directly during planning rather than relying on either document to have restated its shape.
- The later architecture doc's epic-wide default is **"interface redesign, not transliteration"** for CLI surfaces — 0172 deliberately departs from this (flag-for-flag preservation, no subcommand redesign, no `status` command), which is already a recorded, intentional divergence in 0172's own Requirements/Open Questions, not a gap to fix.
- Both documents agree the migration engine is the epic's highest-complexity/highest-risk cluster and belongs last on the phased spine — fully consistent with 0172's own framing.

## Code References

- `skills/config/migrate/scripts/run-migrations.sh` — the bash driver (lifecycle, flags, apply loop)
- `skills/config/migrate/scripts/interactive-lib.sh` — FIFO/fd IPC, watchdog, dry-apply validation, resume
- `scripts/interactive-harness.sh` — author-facing migration API
- `scripts/interactive-protocol.sh:9-59` — the wire-protocol frame encoding (unread in this pass beyond existence — read directly during planning)
- `scripts/jsonl-common.sh:21-50,60,66-149,115-116` — JSON escaping, canonical compose order, the undetected `proposed_value` emptiness gap
- `skills/config/migrate/migrations/0001-*.sh` … `0007-unify-meta-corpus-frontmatter.sh` — the 7 migrations
- `cli/store/src/lib.rs` — standalone `atomic_write` crate (0167's carve-out, already landed)
- `cli/config-adapters/src/store.rs:29-33` — `LegacyPolicy::{Reject,Allow}`, the shipped `--allow-legacy-layout` mechanism
- `cli/launcher/src/launch/inbound/cli.rs:56,94,...` — the clap flag wiring for `--allow-legacy-layout` across ~14 read actions
- `cli/config-adapters/tests/config_reader.rs:67` — negative test proving `ACCELERATOR_MIGRATION_MODE` is honoured by nothing
- `cli/corpus/src/store.rs` — `AtomicWrite`/`RecordStore` ports, `StoreError`
- `cli/corpus/src/record.rs:26` — the `Record` domain type matching 0180's canonical field order
- `cli/corpus-adapters/src/store.rs` — `FileCorpusStore` (public), composed from private `jsonl.rs`/`lock.rs`
- `cli/corpus-adapters/src/patcher.rs:48` — the only existing line-preserving frontmatter rewrite, `status:`-specific
- `cli/document/src/lib.rs` — `parse`/`render`/`fence_offsets`/`split`, `Yaml` value tree
- `cli/vcs-adapters/src/library.rs` — `InProcessProbe`, gix+jj-lib, no subprocess
- `cli/vcs-cli/` — the `accelerator-vcs` sub-binary, the concrete registration template
- `cli/kernel/src/hooks.rs:9-58` — `session_start`/`pre_tool_use_deny`/`pre_tool_use_warn`/`adapter_failure`
- `cli/launcher/src/launch/core.rs:219-224` — `swallow_under_fail_safe`
- `hooks/hooks.json` — current SessionStart/PreToolUse registrations (migrate-discoverability still unconverted)
- `hooks/migrate-discoverability.sh` — the bash hook to port
- `tasks/README.md:304-441` — the 13-point sub-binary registration checklist
- `tasks/test/integration.py:34,77-101,397-403` — `_EXPECTED_MIGRATE_SUITES` and the shared floor helper
- `tasks/lint/scripts.py:18-49` — `SHELL_LIBRARIES`, including the three migrate/interactive entries to remove
- `tasks/lint/call_site_migration.py` — the actual `--allow-legacy-layout` confinement guard (0172 misnames this as `skill_permissions.py`)
- `tests/unit/tasks/test_call_site_migration.py` — its test (correctly named in 0172)
- `hooks/test-fixtures/vcs-detect/regenerate.sh`, `hooks/test-fixtures/masks.toml` — bash-golden capture templates
- `cli/vcs-cli/tests/detect_goldens.rs`, `status_log_goldens.rs` — Rust golden-replay templates
- `cli/launcher/tests/config_read.rs` — black-box CLI test scaffolding template
- `meta/inventories/0167-suite-audit.md`, `0167-removal-set.md`, `0167-divergences.md` — surviving assertion-inventory template (extractor script itself did not survive)
- `meta/prs/26-description.md` — 0167's merged PR description, corroborating its `done` status

## Architecture Insights

- **The crate foundation is Model 1** (each sub-binary wires its own adapters at its own composition root — no shared adapter registry), so `accelerator-migrate`'s `main.rs` should construct its own `config-adapters`/`corpus-adapters` instances directly, mirroring `accelerator-vcs`.
- **ADR-0053's hexagonal enforcement is two-tiered**: crate-boundary + `cargo-deny` between crates, `cargo-pup` at module granularity within a single crate. A new subdomain starts as one crate with internal hexagon-layer modules; splitting into separate crates is a later, not automatic, step.
- **The in-process-transport requirement is architecturally consistent with the epic's general precedent**, not migrate-specific: the epic's own `work`/`tracker` coupling design already favours in-process composition over subprocess dispatch when a state machine needs transactionality and port-fakeable tests — the same reasoning applies to replacing the FIFO/fd IPC.
- **The registration surface (0187) treats "token exists in `DISPATCHED_SUBBINARIES`" and "some skill binds it" as a single atomic unit** enforced pre-release — this is a hard ordering constraint on the implementation plan's commit structure, not just a checklist nicety.
- **Golden-test lifecycle has a documented shape-shift**: differential (live shell-out vs Rust) during an active migration, pruned to fixed-value/static-golden assertions once the bash source is deleted (seen in `config-adapters/tests/parity.rs`'s own header comment). 0172's plan should anticipate the same two-phase shape for its own parity suite.

## Historical Context

- `meta/decisions/ADR-0023-*.md`, `ADR-0037-*.md`, `ADR-0038-*.md`, `ADR-0047-*.md`, `ADR-0052-*.md`, `ADR-0053-*.md` — see §10 above.
- `meta/plans/2026-07-19-0167-config-command-and-invocation-contract-migration.md:1566-1589` — the Phase-6 "behaviour inventory" design that 0172's stricter "every assertion mapped" requirement should be sized against (0167's own extractor script did not survive its cutover, so treat this as design precedent only).
- `meta/validations/2026-07-19-0167-config-command-and-invocation-contract-migration-validation.md` — confirms 0167's inventory/confinement guards were green at merge.
- `meta/reviews/work/0172-migration-engine-subdomain-review-1.md:1066-1068,1339` — the review history showing the ADR-reconciliation follow-up item was escalated to a formal AC across passes but never actually created.

## Related Research

- `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md` — epic-wide crate layout, phased decomposition (migration engine = Phase 9, last), testing-strategy resolution (hybrid repoint-then-classify).
- `meta/research/codebase/2026-06-23-0136-shell-scripts-rust-cli-migration-surface.md` — earlier surface survey; source of the "19-frame TSV state machine" characterisation of `interactive-protocol.sh`.

## Open Questions

- **0173→0195 retargeting**: does the plan (or a preceding small edit to 0172 itself) need to formally retarget the golden-capture ordering constraint from the abandoned 0173 to 0195 before implementation starts, given the race is currently live and unguarded?
- **ADR-reconciliation follow-up item**: should this research/plan cycle create it now, or is that explicitly out of scope for `/create-plan` and left as a pre-deletion-commit gate?
- **`--allow-legacy-layout` crate-level equivalent**: 0172 flags that the flag as shipped is a CLI-level affordance on `accelerator config` read subcommands, while `accelerator-migrate` will consume the config crates in-process — does the plan need `FileConfigStore::with_legacy_policy(LegacyPolicy::Allow)` directly (confirmed accessible per §4), or is a crate-level "equivalent" a distinct open question still requiring cross-item recording?
- **0118's sentinel-tolerance behaviour**: does the Rust 0007's self-validation path need an equivalent to the shell backfill's "pending" sentinel for non-derivable required extras, or does moving self-validation onto `corpus`/`document` change this need entirely?
- **`#`-comment-line and `--list` wire-format provenance**: confirm these originate in 0117 (not read in this pass) rather than being 0172-invented, since neither 0115/0116/0119 documents them.
- **0182's own unchecked AC**: the mise.local.toml/allowlist-invariant acceptance criterion is unchecked despite 0182's `status: done` — worth confirming this is closed before 0172's plan relies on the allowlist being final.
