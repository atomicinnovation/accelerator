---
name: migrate
description: Apply pending Accelerator meta-directory migrations to bring a repo into line with the latest plugin schema. Destructive by default but guarded — refuses to run on a dirty working tree and prints a one-line preview per pending migration before applying.
allowed-tools: 
  - Read
  - Write
  - Edit
  - Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator migrate *)
---

> **Warning: this skill rewrites files in `meta/` and `.claude/accelerator*.md`.** Recovery is via VCS revert. Before running, ensure your repo is committed and you understand what each pending migration does. The safety guards (clean-tree check, preview) exist to give you a moment to stop — they are not a substitute for understanding the changes.

## When to invoke

Run `/accelerator:migrate` after upgrading the Accelerator plugin to a version that bundles new migrations. The SessionStart hook will tell you when this is needed:

```
[accelerator] .accelerator/state/migrations-applied is behind the plugin
(highest applied: 0001-rename-tickets-to-work; highest available: 0002-...).
Run /accelerator:migrate to bring it up to date.
```

You can also run it proactively — if no migrations are pending, it prints `No pending migrations.` and exits cleanly.

**Upgrade sequence.** After pulling a new plugin version, run `/accelerator:migrate` before invoking any skill that reads or writes paths affected by pending migrations. Skills do not gate themselves on pending migrations; the SessionStart hook only warns when migrations are pending. Running write-side skills (e.g., `/accelerator:research-codebase`) between the plugin upgrade and the migration may produce results written to or read from new-default paths that do not yet exist on disk.

## How it works

`accelerator migrate` (the `accelerator-migrate` sub-binary) runs the migration lifecycle in-process — there is no forked child per migration and no wire protocol to a subprocess.

1. **Clean-tree pre-flight.** Scans `meta/`, `.claude/accelerator*.md`, and `.accelerator/` for uncommitted changes. A clean tree proceeds with a fresh run. A dirty tree where every dirty path belongs to this run's own prior partial output (a resumable interactive session log, in-flight manifest-tracked writes) proceeds too, printing a one-line resume affordance. A dirty tree carrying anything else aborts. Set `ACCELERATOR_MIGRATE_FORCE=1` to bypass entirely (advanced users only) — skipped migrations stay skipped even under FORCE. A run-level advisory lock (`.accelerator/state/`) refuses a second concurrent invocation fast rather than racing it.
2. **Read state.** Loads `.accelerator/state/migrations-applied` and `.accelerator/state/migrations-skipped` — newline-delimited lists of migration IDs. If either file is absent, its set is empty. Unknown IDs (from a newer plugin version) are preserved verbatim and warned about. An ID appearing in both files triggers a warning; applied takes precedence.
3. **The registry.** Migrations are not discovered by scanning a directory of scripts — they are a fixed, compile-time-ordered list (`migrate::registry::registry()`) baked into the binary. `ACCELERATOR_MIGRATIONS_DIR` is not recognised; there is no directory to override.
4. **Compute pending.** A migration is pending if its ID is in neither the applied nor the skipped set.
5. **Preview banner.** Prints one line per pending migration — `<ID> — <description>` — with a per-migration skip hint (`--skip <id>`). If nothing is pending, prints `No pending migrations.` (plus any skipped names) and exits 0 immediately.
6. **Apply in order.** For each pending migration: a mechanical migration's `apply()` runs to completion in one call; an interactive migration (see below) runs the full accept/edit/skip prompt loop. On success, atomically appends its ID to `.accelerator/state/migrations-applied` and prints `[<id>] applied`. On failure, prints the error and `[<id>] failed`, exits 1, and leaves the state file at the last successful migration. A migration whose `apply()` returns `NoOpPending` is treated as a soft skip — it stays pending, printing `[<id>] no-op (stays pending)`, and will be retried on future runs.
7. **Summary.** Prints counts of applied, skipped, and pending (no-op) migrations.

## Authoring a migration

A migration is ordinary Rust, not a bash script — there is no opt-in header, no published shell hook set, and nothing to source. Add a module under `cli/migrate/src/migrations/` and register it in `cli/migrate/src/registry::registry()`.

```rust
pub trait MigrationMeta {
    fn id(&self) -> &'static str;
    fn description(&self) -> &'static str;
}

pub enum ApplyOutcome { Applied, NoOpPending }

pub trait Migration: MigrationMeta {
    fn apply(&self, ctx: &dyn MigrationContext) -> Result<ApplyOutcome, MigrationError>;
}
```

`ctx: &dyn MigrationContext` is the only way a migration touches the outside world — `migrate` itself carries no filesystem, config, or VCS dependency. The capabilities it exposes (all in `cli/migrate/src/ports.rs`, with a full doc comment on each):

- `doc_type_dirs()` — the configured doc-type → directory table (the in-process replacement for shelling out to read config).
- `revision()` — the current VCS revision, used for guarded-resume staleness checks.
- `corpus_index()` — target-existence checking (`target_exists(type, id)`), for migrations that validate cross-references.
- `write(path, content)` — the **only** content-mutation path. Every write routes through the path manifest as a side effect of the call itself, so "recorded at time-of-mutation" is structural, not a discipline to remember.
- `config_value(key)` / `configured_path_override(key)` — full-stack config lookup, including legacy key names migrations still read.
- `read(path)`, `dir_exists(path)`, `remove_file(path)`, `remove_dir_if_empty(path)`, `list_md_files(dir)`, `list_all_under(dir)` — filesystem access, sandboxed and manifest-aware.
- `merge_move(src, dst)` — whole-file/whole-directory relocation (directory renames, state relocation).
- `canonicalise_work_item_id(bare_number)` — renders a bare number under the configured `work.id_pattern`.
- `validate_frontmatter(files)` — runs `accelerator corpus frontmatter validate` in-process.

A mechanical migration must self-detect and no-op on already-applied state (idempotent) — the ledger filter is not the only guard, matching ADR-0023's belt-and-suspenders requirement. Return `ApplyOutcome::NoOpPending` when its preconditions are not yet met and it should stay pending, retried on future runs; guarantee no destructive work happened before returning it. There is no `DRY_RUN` concept — this framework has no dry-run mode.

## State file format

`.accelerator/state/migrations-applied` contains one migration ID per line, in the order migrations were applied:

```
0001-rename-tickets-to-work
0002-some-future-migration
```

`.accelerator/state/migrations-skipped` contains one migration ID per line for migrations the user has chosen to defer:

```
0002-some-future-migration
```

Both files are human-readable and constitute the audit trail. Do not edit them manually unless you are deliberately marking a migration as applied, unapplied, or skipped.

## Skip-tracking

Skip a migration to defer it indefinitely:

```bash
${CLAUDE_PLUGIN_ROOT}/bin/accelerator migrate --skip <migration-id>
```

Unskip a previously skipped migration so it becomes pending again:

```bash
${CLAUDE_PLUGIN_ROOT}/bin/accelerator migrate --unskip <migration-id>
```

Skipped migrations never run and do not block other pending migrations. The pre-run banner includes a `--skip` hint for each pending migration. `ACCELERATOR_MIGRATE_FORCE=1` bypasses the dirty-tree pre-flight only; skipped migrations remain skipped even with FORCE.

## Optional interactive contract

Mechanical migrations are the default — they implement `Migration` and run start-to-finish with no user interaction. A migration that needs to **ask the user about ambiguous transformations** implements `InteractiveMigration` instead:

```rust
pub struct Transformation {
    pub key: String,           // session-log identity, "{path}#{anchor}"
    pub path: String,
    pub anchor: String,
    pub proposed: String,
    pub predicate_value: String,
    pub display: String,       // author-declared prompt content
    pub extras: Vec<(String, String)>,
}

pub enum PredicateOutcome { Prompt, Mechanical, Fail(String) }

pub enum Decision { Accept, Edit(String), Skip }

pub trait InteractiveMigration: MigrationMeta {
    fn emit_transformations(&self, ctx: &dyn MigrationContext) -> Vec<Transformation>;
    fn evaluate_predicate(&self, t: &Transformation) -> PredicateOutcome;
    fn validate_edit(&self, t: &Transformation, value: &str) -> Result<(), String>;
    fn apply_decision(&self, t: &Transformation, d: &Decision, ctx: &dyn MigrationContext) -> Result<(), String>;
    fn verify_applied(&self, t: &Transformation, recorded: &corpus::Record) -> bool { true }
    fn finalise(&self, ctx: &dyn MigrationContext) -> Result<(), String> { Ok(()) }
}
```

There is no wire protocol, no TSV frames, no base64 display blocks, and no hand-written JSON — every callback is a plain Rust function call against typed values.

### Callback contracts

- `emit_transformations` runs once, unconditionally, before any prompting — mechanical work (a precondition pass, a rewrite pass) happens here. It has no `Result` of its own; a mechanical failure is carried as a single sentinel `Transformation` whose `evaluate_predicate` returns `Fail(message)`.
- `evaluate_predicate` routes a transformation to the prompt loop (`Prompt`), applies it mechanically with no prompt (`Mechanical`), or aborts the whole migration with `[<id>] <message>` (`Fail`).
- `validate_edit` returns `Err(message)` to reject a user's edit; the engine prints `"[interactive] {message}"` and re-prompts. No record is persisted for a rejected attempt.
- `apply_decision` is called once per `accept` or `edit` decision, **after** the engine has durably persisted the session-log record (write-ahead-log invariant) — enforced structurally, never the reverse. It is **never** called for `skip`.
- `verify_applied` (optional; defaults to `true`) is consulted on resume, before replaying an accepted/edited record. `false` is handled identically to source drift: the stale record is discarded and re-prompted. Never consulted for a skipped record.
- `finalise` (optional; defaults to a no-op) is called exactly once, unconditionally, after every transformation this run decided has been applied — or immediately if there were none to begin with. This is where whole-corpus post-apply validation belongs; it is not threaded through any one transformation's own callback.

> **Callbacks may be invoked more than once per run, so they must be deterministic and side-effect-free.** A `--decisions-file` run calls `emit_transformations`/`evaluate_predicate` during `--list` enumeration and again during the dry-apply validation pass before the live run, and calls `validate_edit` during dry-apply and again at live apply. `validate_edit` in particular **must be a pure function of its arguments** — it must not read corpus state an earlier transformation in the same run could mutate, or dry-apply could pass against the unmutated corpus while the live run fails later after earlier files changed.

### Runner guarantees

- **Predicate routing**: each transformation is routed to the prompt loop or the mechanical path based on `evaluate_predicate`'s outcome.
- **Display elements**: every prompt shows the proposed value, the source location as `path:anchor`, the predicate's evaluated value, plus the author's own `display` content.
- **Resumability**: every prompted decision is durably persisted to `.accelerator/state/migrations-<id>-session.jsonl` **before** the artefact is mutated. On re-entry, already-decided keys replay silently (`verify_applied` consulted for accepted/edited keys only) and skip the prompt. A migration completes (its ID is appended to `migrations-applied`) only when every transformation has a terminal record.
- **Decision verbs**: `accept` applies the proposed value; `edit <value>` substitutes a user-provided value (validated by `validate_edit`); `skip` records the decision but does not mutate the artefact.

### Runner-level decisions

- **Source drift**: if a recorded record's `proposed_value` differs from the live emission's, the engine discards the old record and re-prompts — this applies to every resumed record regardless of outcome (accepted, edited, or skipped alike), and takes priority over everything else, including sticky skip.
- **Transformation ordering**: emission order from `emit_transformations` is the canonical iteration order.
- **Sticky skip**: a transformation skipped on a prior run remains skipped on every subsequent run, unless its source drifts. Delete the corresponding session-log line to re-prompt it.
- **The 30-second decision timeout**: a live TTY prompt that receives no answer within 30 seconds fails the run (stderr message, exit non-zero, session log left as of the last completed decision — the next run prompts only what's still undecided). This is new behaviour the Rust port introduces, not a port of anything bash did — bash's own terminal read had no timeout at all. It is not user-configurable (no flag, no config key). When there is no TTY and no decisions file at all, the run stalls immediately without arming any timeout — see the structured stall below.

### Session log

- Path: `.accelerator/state/migrations-<id>-session.jsonl`. Relative paths in `--decisions-file`/stall messages are resolved against the project root.
- One JSON object per line, canonical field order: `transformation_key`, `schema_version: 1`, `outcome` ∈ `{accepted, edited, skipped}`, `proposed_value`, optional `user_value` (only for `edited`), `timestamp`. Author-declared `extras` on a `Transformation` are not persisted to the session log — they exist only within the run (e.g. `--list`'s short-key display reads a `linkage_key` extra).
- The session log is retained as an audit artefact after full completion; users may delete it manually. The runner refuses to resume from a log with an unknown `schema_version` and prints a clear recovery instruction.

## Worked example

`meta/work/0001-improve-startup-time.md` references another work item in an unstructured `## References` section:

```markdown
## References
- `meta/work/0042-add-a-caching-layer.md`
```

Migration `0007-unify-meta-corpus-frontmatter` recognises this as an ambiguous body-section linkage — a `relates_to` reference it can propose but must not apply without confirmation. `--list` reveals it, dry-emitting the pending transformation without mutating anything:

<!-- @list-output-start -->
```
1	relates_to	relates_to=work-item:0042	meta/work/0001-improve-startup-time.md:body:references#0
```
<!-- @list-output-end -->

Writing `accept` to a decisions file and resuming with `--decisions-file` applies it:

<!-- @decisions-file-start -->
```
accept
```
<!-- @decisions-file-end -->

```bash
accelerator migrate --decisions-file decisions.txt
```

The frontmatter gains a typed `relates_to` reference, and the session log records the decision:

<!-- @after-frontmatter-start -->
```
relates_to: ["work-item:0042"]
```
<!-- @after-frontmatter-end -->

<!-- @session-log-start -->
```
{"transformation_key":"meta/work/0001-improve-startup-time.md#body:references#0","schema_version":1,"outcome":"accepted","proposed_value":"relates_to=work-item:0042","timestamp":"<REDACTED>"}
```
<!-- @session-log-end -->

At a real terminal (no `--decisions-file`), the same transformation renders as a live prompt instead:

```
Session log: <SANDBOX>/.accelerator/state/migrations-0007-unify-meta-corpus-frontmatter-session.jsonl  (resume from this file by re-running /accelerator:migrate)

Proposed linkage: relates_to: "work-item:0042"
Section anchor: body:references#0
Band: ambiguous
  proposed: relates_to=work-item:0042
  source: meta/work/0001-improve-startup-time.md:body:references#0
  predicate: ambiguous
accept | skip | edit <value>: 
```

## Answering prompts as an agent (the invoker contract)

When `/accelerator:migrate` runs without a human at a terminal, an agent answers the interactive prompts with a **decisions file**, following the four steps `list → decide → write → resume`:

In practice the agent first runs the migration and hits the **structured stall**, which names the exact decisions-file path — including the migration `<id>` — (`.accelerator/state/migrations-<id>-decisions.txt`) and a copy-pasteable resume command. `--list` is then the step that **reveals the proposed values** (which the stall does not show), so the realistic order is run → stall (learn the `<id>` and path) → `--list` (see proposed values) → write → resume. The `<id>` comes from the stall/preview, not from `--list` output.

1. **list** — `accelerator migrate --list` dry-emits every pending
   interactive transformation, one tab-delimited line each, without mutating the
   corpus:

   ```
   <position>\t<key>\t<proposed>\t<path>:<field>
   ```

   (Fields are separated by a literal TAB, shown as `\t` here; the same column
   vocabulary — `<path>:<field>` — is used in `--help`.) Proposed values are
   revealed only here, so list before deciding. When parsing, **skip lines
   beginning with `#`**: with more than one pending interactive migration the
   output is segmented by a `# migration <id>` header and `<position>` restarts
   at 1 per migration. Resume each `# migration <id>` section separately with its
   own decisions file (a single multi-migration decisions file is not yet
   supported; `--list` prints a stderr note when more than one is pending).
2. **decide** — choose a verb per transformation: `accept`, `skip`, or
   `edit <value>`.
3. **write** — write one verb per line to a decisions file,
   **matched by emission order** (line *i* answers list position *i*;
   skipped/mechanical transformations consume no line). Create the file yourself
   at a path that exists and is readable — the stall message points at
   `.accelerator/state/migrations-<id>-decisions.txt`; do not overwrite existing
   `migrations-<id>-*` state files. `#`-prefixed lines are **not** comments —
   they parse as an unknown verb, matching bash's own behaviour exactly. For
   example:

   ```bash
   printf 'accept\nskip\nedit work-item:0100\n' \
     > .accelerator/state/migrations-<id>-decisions.txt
   ```
4. **resume** — re-run with `--decisions-file <path>` (or the equivalent
   `ACCELERATOR_MIGRATE_DECISIONS_FILE` env var, discoverable via `--help`; a
   supplied flag always wins over a pre-existing env var). The
   stall's copy-pasteable command is exactly this bare form — **no
   `ACCELERATOR_MIGRATE_FORCE=1` is needed** in the normal case. A partial
   interactive run dirties the tree only with files this run owns (the
   interactive session log, plus any frontmatter already written), and the
   **guarded resume** lets the re-run proceed over that own output without
   `FORCE` when the base revision is unchanged, printing a
   one-line affordance listing the owned paths being resumed over. `FORCE` is
   required **only** when the pre-flight refuses — i.e. the tree carries dirt this
   run does *not* own (foreign changes, or you have committed since the partial
   run so the base revision moved). In that case, re-run once without `FORCE`
   first to read the refusal guidance, confirm via `jj
   status`/`git status` that the dirty paths really are this migration's own, and
   only then add `ACCELERATOR_MIGRATE_FORCE=1`.

The driver **validates the decisions file up front (a no-mutation dry-apply pass)
and fails closed**: an unknown verb, a count mismatch (too few or too many
verbs), or a rejected `edit` value that no following line corrects exits
non-zero, names the offending position, and leaves the corpus **unmutated** —
validation never partially applies. Once validation passes and the live
apply begins, transformations are applied in order without rollback, so an
apply-time failure can leave a partial corpus; recover with VCS revert, then
re-run — guarded resume replays the run's own partial output without
`FORCE` when the base revision is unchanged.

When no decision input is available at all, the run emits the structured stall
(`MIGRATION STALLED: no decision input available`) and stops without further
mutation:

```
[0007-unify-meta-corpus-frontmatter] MIGRATION STALLED: no decision input available
[0007-unify-meta-corpus-frontmatter]   pending decision: meta/work/0001-improve-startup-time.md#body:references#0
[0007-unify-meta-corpus-frontmatter]   No decisions file, terminal, or piped input was available to
[0007-unify-meta-corpus-frontmatter]   answer this prompt, so the migration cannot proceed.
[0007-unify-meta-corpus-frontmatter]
[0007-unify-meta-corpus-frontmatter]   This migration may have already partially modified the
[0007-unify-meta-corpus-frontmatter]   working tree. Re-running /accelerator:migrate resumes this
[0007-unify-meta-corpus-frontmatter]   partial run when the base revision is unchanged (decided
[0007-unify-meta-corpus-frontmatter]   transformations are replayed, not re-applied).
[0007-unify-meta-corpus-frontmatter]
[0007-unify-meta-corpus-frontmatter]   To resume: each run answers the current prompt only (you
[0007-unify-meta-corpus-frontmatter]   may be stalled again for the next undecided transformation):
[0007-unify-meta-corpus-frontmatter]     1. write the decision (accept | skip | edit <value>),
[0007-unify-meta-corpus-frontmatter]        one per line, to: <path>
[0007-unify-meta-corpus-frontmatter]        (create this file yourself; do not overwrite existing
[0007-unify-meta-corpus-frontmatter]        migrations-0007-unify-meta-corpus-frontmatter-* state files)
[0007-unify-meta-corpus-frontmatter]     2. then run (copy-pasteable):

accelerator migrate --decisions-file <path>

[0007-unify-meta-corpus-frontmatter]   equivalent env-var form:

ACCELERATOR_MIGRATE_DECISIONS_FILE=<path> accelerator migrate
```

This contract is scoped to a single pending interactive migration (the realistic
case); decisions files are consumed per migration.

## Executing the migration

Invoke via Bash:

```bash
${CLAUDE_PLUGIN_ROOT}/bin/accelerator migrate
```

`accelerator migrate` resolves the project root automatically from the current working directory. Run it from within the consumer repository.

## Cross-references

- `meta/decisions/ADR-0023-meta-directory-migration-framework.md` — framework design rationale
- `meta/decisions/ADR-0037-optional-interactive-contract-supplement-to-adr-0023.md` — optional interactive contract
- `meta/decisions/ADR-0038-interactive-validation-parameters-for-unified-schema-linkage-migration.md` — first consumer's parameterisation
- `meta/work/0116-structured-stall-on-no-decision-input.md` — the structured stall (`MIGRATION STALLED: no decision input available`) the invoker contract defers to when no decision input is available
- `skills/config/init/SKILL.md` — `init` bootstraps fresh repos; `migrate` upgrades existing ones
- `skills/config/configure/SKILL.md` — configuration reference
