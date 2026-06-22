---
type: codebase-research
id: "2026-06-21-0117-agent-decisions-bridge-and-invoker-contract"
title: "Research: Agent-Decisions Bridge and Documented Invoker Contract (0117)"
date: "2026-06-21T00:14:37+00:00"
author: Toby Clemson
producer: research-codebase
status: complete
work_item_id: "0117"
parent: "work-item:0117"
relates_to: ["codebase-research:2026-06-20-0116-structured-stall-on-no-decision-input"]
topic: "Agent-Decisions Bridge: --list mode, decisions-file promotion, invoker contract"
tags: [research, codebase, migrate, interactive-migration, agent-invocation, run-migrations, interactive-lib, skill-md]
revision: "5a9ac98c68492ad71bea388d0ceeafc41d9aa7a6"
repository: "accelerator"
last_updated: "2026-06-21T00:14:37+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# Research: Agent-Decisions Bridge and Documented Invoker Contract (0117)

**Date**: 2026-06-21T00:14:37+00:00 (UTC)
**Author**: Toby Clemson
**Git Commit**: 5a9ac98c68492ad71bea388d0ceeafc41d9aa7a6
**Branch**: HEAD (detached / jj-colocated)
**Repository**: accelerator

## Research Question

What does the codebase look like for implementing work item 0117 — adding a
`--list` (dry-emit) mode to the migration driver, promoting
`ACCELERATOR_MIGRATE_DECISIONS_FILE` to a documented user-facing interface with
fail-closed validation, and documenting the invoker-side interactive contract in
`SKILL.md`? Where do the relevant surfaces live, how do they work today, and what
constraints bind the implementation?

## Summary

The implementation touches three files — `run-migrations.sh` (driver),
`interactive-lib.sh` (runner-side prompt/decision loop), and `SKILL.md` (skill
doc) — plus a new test suite. Six findings reshape how the work item should be
planned:

1. **The work item's premise is partly stale.** It says "the driver exposes only
   `--skip`/`--unskip`" and that the env var is "documented only in an inline
   source comment as a hidden test-only seam." Both are now out of date. Sibling
   work item **0116 has already landed** (`--decisions-file` switch, the
   structured stall, and a `--help` block), so a chunk of 0117's *described*
   scope is already done. What genuinely remains: **`--list`**, **env-var
   promotion into `--help`** (AC4), **decisions-file content validation**
   (count/verb fail-closed, AC6), and the **`SKILL.md` invoker section** (AC5).

2. **`--list` has no dry-run path today and the architecture resists one.** The
   migration runs as a *forked child* that emits `PROMPT` frames; the runner only
   advances the child past a prompt by sending it a `DECIDE` frame, and the only
   non-mutating verb (`skip`) still triggers a session-log write. There is no
   protocol verb for "enumerate without deciding." The clean design is a
   **child-side list mode** that dumps the already-buffered transformation list
   (`TX_LINES`) before the decide handshake — not a runner that feeds skips.

3. **All four `--list` fields already exist at emission time** (key, proposed
   value, source path, anchor/band-field), and `render_prompt` already prints
   `path:anchor` joined with a colon (`interactive-lib.sh:213`) — exactly the
   `--list` join format the work item specifies.

4. **The flag block must be converted from `if/case` to `while/shift`.** It is
   still a single-leading-flag one-shot (verified at `run-migrations.sh:29-73`);
   0116 explicitly flagged that 0117's `--list` forces this conversion. There is
   also **no unknown-flag rejection** today (no `*)` arm).

5. **Position/ordering has a subtlety.** Decisions-file line *i* maps to the
   *i*-th **decision-requiring** PROMPT in emission order — not the *i*-th
   transformation. Mechanical/resumed transformations consume no line;
   `VALIDATE_ERR` re-prompts consume an *extra* line. `--list` numbering must
   derive from the same predicate-filtered subset to stay 1:1 with consumption.

6. **AC7 (live-0007 integration) is correctly out of scope.** Live 0007 cannot
   reach its interactive stage until 0118 lands; 0117's fixture is deliberately
   standalone, built on the existing `0002-predicate` interactive fixture.

There is also an **ADR-0037 open question**: the invoker-side change does *not*
trip the ADR's recursive supplement triggers (no new verb / display element /
resumability guarantee) and the ADR is neutral on invocation mechanics, so it
reads as an implementation detail under the existing decision — but 0117 must
record this call explicitly.

## Detailed Findings

### Driver: `run-migrations.sh` (the `--list` and validation surface)

`skills/config/migrate/scripts/run-migrations.sh`

- **Flag parsing is a single-leading-flag `if/case` on `$1`** —
  `run-migrations.sh:29-73` (verified). Arms: `--skip` (31-40), `--unskip`
  (41-49), `--decisions-file` (50-60, sets+exports the env var, `shift 2`, falls
  through), `--help | -h` (61-71). There is **no `*)` catch-all** — an unknown
  first argument silently falls through and a normal run proceeds. For `--list`
  to coexist with `--decisions-file`, this block must become a `while`/`shift`
  loop (0116's plan called this out explicitly).
- **`--help` output** — `run-migrations.sh:61-71`, prints to **stderr**, exits 0.
  Today it documents `--skip`, `--unskip`, and `--decisions-file` but **does not
  mention `ACCELERATOR_MIGRATE_DECISIONS_FILE`** (AC4 is genuinely outstanding)
  and has no `--list` line.
- **Env-var resolution + comment** — `run-migrations.sh:14-23`. The inline
  comment **no longer** calls it a "hidden test-only seam"; it now reads
  "Reachable two ways — the documented `--decisions-file` flag … and this env
  var" and even names the ticket: *"(0117 promotes the env var into --help and
  adds --list.)"* The work item's quoted "test-only" framing is stale; that
  phrase survives only at `interactive-lib.sh:375-376`.
- **Decisions-file validation is filesystem-only today** —
  `run-migrations.sh:75-92` (section "1b"): not-a-directory (80), exists (85),
  readable (90). There is **no content validation** — no count check, no verb
  check. AC6's fail-closed count-mismatch / unknown-verb validation is net-new
  and belongs here (or a new section near it). Note the consumer side
  (`read_decision`) currently passes unknown verbs through silently via its `*)`
  arm (`interactive-lib.sh:293-296`) — that silent pass-through is what AC6 must
  replace with an up-front fail-closed check.
- **Dirty-tree guard** — `run-migrations.sh:94-168` (section 2), unconditional
  unless `ACCELERATOR_MIGRATE_FORCE` is set. The plan must decide whether `--list`
  (read-only) should run *before* this guard so a dry list doesn't require a
  clean tree.
- **Discovery & dispatch** — pending computed at 231-246; preview banner 248-273;
  `interactive-lib.sh` sourced late at **277**; apply loop 279-324 dispatches via
  `is_interactive_migration` (287). **No corpus mutation happens before the apply
  loop** — so a `--list` dry-emit can run discovery and `exit 0` cleanly between
  the preview (273) and the source/apply (277/281).
- **Exit codes** — `exit 1` on bad args, bad decisions file, dirty tree, and
  migration failure; `exit 0` on `--skip`/`--unskip`/`--help`, no-pending, and
  successful completion. `set -euo pipefail` at line 2.

### Runner-side prompt/decision loop: `interactive-lib.sh`

`skills/config/migrate/scripts/interactive-lib.sh` (+ child-side
`scripts/interactive-harness.sh`, wire contract `scripts/interactive-protocol.sh`)

- **Two-process architecture (the key constraint).** The migration runs as a
  forked **child**; the runner talks to it over two FIFOs. The child *emits*
  `PROMPT` frames (`interactive-harness.sh:363-364`); the runner *relays* them
  (`interactive-lib.sh:509-515`). The runner does not originate prompt data.
- **PROMPT wire fields** — `interactive-protocol.sh:26`:
  `PROMPT  key path anchor proposed predicate_value extras_tsv display_b64`. The
  four fields `--list` needs map to `key` / `proposed` / `path` / `anchor`, **all
  present at emission**. `render_prompt` already joins path+anchor with a colon:
  `printf 'Source: %s:%s\n' "$path" "$anchor"` (`interactive-lib.sh:213`) — the
  precedent for the `--list` `<path>:<band/field>` join.
- **`read_decision()`** — `interactive-lib.sh:240-298` (the work item's cited
  "266-287" shifted after 0116). Verbs at 276-297: `accept` / `skip` /
  `edit <value>` / bare `edit`; the `*)` wildcard passes unknown lines through
  verbatim (no rejection). CRLF stripped only on the decisions-file branch (249);
  blank lines skipped only there (251-258). **Three-valued return** (documented
  at 238-239): `0` decision read, `1` read-error/exhausted/TTY-EOF, `2` no input
  channel.
- **The structured stall (0116, already landed)** — `emit_no_input_stall`
  (306-338) and `read_decision_or_stall` (344-355). On return `2` it prints a
  stderr block with the stable marker **`MIGRATION STALLED: no decision input
  available`**, the current pending key, and resume commands in two forms
  (`--decisions-file <path>` at 332, env-var form at 336). It returns `rc` (does
  not exit); both call sites tear down with `exec 7>&-; return 1` (PROMPT site
  516-519, VALIDATE_ERR site 550-553). **0117 must not collapse the three-valued
  return to a generic failure** — this is the shared-region merge constraint.
- **fd 9 / decisions-file wiring** — opened in `run_interactive_migration` at
  `interactive-lib.sh:377-383` (`exec 9<"$ACCELERATOR_MIGRATE_DECISIONS_FILE"`),
  literal fd 9 because bash 3.2 has no `{var}<` allocator; closed at 606-608.
  Consumed line-by-line via `read -r line <&"$DECISIONS_FD"` (243, 252), one read
  per `read_decision` call, strictly sequential.
- **No dry-run path exists.** Mutation lives entirely in the child's
  `migration_apply_decision`, gated on the runner sending `APPLY`
  (`interactive-lib.sh:573`) after persisting the session record. `skip` mutates
  nothing but **still writes a session-log record + triggers APPLY**. So a
  "feed all skips" `--list` would leave session-log side effects. **Cleaner
  design: a child-side list mode dumping the pre-filtered `TX_LINES`**
  (`interactive-harness.sh:272-287`, buffered up-front before any prompting) and
  exiting before the decide handshake.
- **Emission order / position** — established in the child: the order
  `migration_emit_transformations` yields TX records (buffered into `TX_LINES`,
  iterated at `interactive-harness.sh:287`), the "canonical iteration order." No
  per-PROMPT position counter exists on the wire; `PROMPT_INDEX`
  (`interactive-lib.sh:204`) is a display counter that *decrements* on
  re-prompts. **Decisions-file line `i` ↔ the `i`-th decision-requiring PROMPT**,
  not the `i`-th transformation — mechanical/resumed transformations consume no
  line, VALIDATE_ERR re-prompts consume an extra one. `--list` `<position>` must
  derive from the same predicate-passing, non-resumed subset the child filters at
  `interactive-harness.sh:320-336` to remain 1:1 with consumption.

### Skill doc: `SKILL.md` (the invoker-contract gap)

`skills/config/migrate/SKILL.md` (233 lines)

- **Frontmatter** (1-9): `name: migrate`, a description, `allowed-tools: [Read,
  Write, Edit, Bash]`. **No `argument-hint`.**
- **`## Optional interactive contract`** (89-214) is entirely **author-facing**:
  API reference table (95-108), header marker + template (110-129),
  author-facing helpers (131-139), callback contracts (141-145), runner
  guarantees citing ADR-0037 §§1–4 (147-152), runner-level decisions (154-158),
  session log (160-164), worked example (166-214). Terse, reference-style, bold
  lead-ins, backtick identifiers.
- **`## Executing the migration`** (216-224) documents only the bare invocation;
  nothing about headless/agent invocation, decisions files, or `--list`.
- **No `!` preprocessor** is used in this SKILL.md — it is static prose
  (illustrative ```bash``` blocks only); live context comes from the driver at
  run time.
- **Nothing present** for: `ACCELERATOR_MIGRATE_DECISIONS_FILE`, `--list`,
  `--decisions-file`, agent invocation, the interactive stall, or work item 0116.
  `--skip`/`--unskip` are documented under `## Skip-tracking` (67-81).
- **Terminology to match**: "transformations," "prompt loop," verbs
  `accept`/`edit`/`skip` (defined at 152), **"emission order"** (157, already the
  doc's phrase — aligns with AC5's "matched by emission order"), persisted
  outcomes `accepted`/`edited`/`skipped` (163). **Existing fail-closed
  precedent** to mirror for AC5(e): line 164, "The runner refuses to resume from
  a log with an unknown `schema_version` and prints a clear recovery
  instruction."
- **Best home for the new invoker section**: a new top-level `##` section
  between line 214 (end of worked example) and line 216 (`## Executing the
  migration`), keeping author-side and invoker-side adjacent and separated by
  header level. The `## Cross-references` list (226-232) can also surface the
  0116 link, but AC5(c) wants it inside the new section.

### Test patterns for the standalone fixture

`scripts/test-helpers.sh`, `skills/config/migrate/scripts/test-migrate-interactive.sh`,
`test-migrate-0007.sh`, `test-fixtures/interactive/0002-predicate/`

- **Harness**: plain bash + shared `scripts/test-helpers.sh` assert library (not
  bats). Preamble at `test-migrate-interactive.sh:1-15`; asserts include
  `assert_eq`, `assert_neq` (non-zero exit checks), `assert_contains`,
  `assert_matches_regex`, `skip_test`. **The suite ends with `test_summary`
  (line 1271, verified)** — a new suite must call it or it exits 0 regardless of
  failures.
- **Reuse the `0002-predicate` fixture** rather than authoring a new migration.
  It reads `key|path|anchor|proposed|band|prose` lines from
  `$PROJECT_ROOT/.fixture/transformations` and emits them in file order; band
  `ambiguous` → PROMPT, `resolved` → mechanical
  (`0002-predicate.sh:13-33`). The test-side seeder is `seed_predicate_sandbox`
  (`test-migrate-interactive.sh:360-369`). **Three pending transformations in
  fixed order** = three `ambiguous` rows.
- **Decisions file** = newline-delimited verbs via `mktemp` + `printf`, fed by
  `ACCELERATOR_MIGRATE_DECISIONS_FILE`; e.g. `printf 'accept\naccept\naccept\n'`
  (`:380-381`), `printf 'edit corrected-value\n'` (`:581-582`),
  `printf 'skip\n'` (`:605`). Full invocation with
  `ACCELERATOR_MIGRATE_FORCE=1` to bypass the dirty-tree preflight at `:393-399`.
- **Protocol-frame assertions** via TSV sidecar logs
  (`MIGRATION_PROTOCOL_LOG_RUNNER` / `MIGRATION_PROTOCOL_LOG_MIGRATION`); count
  with `grep -c $'^PROMPT\t'` (`:401-404`); ordering via `grep -n … | head -1`.
- **Fail-closed precedents** (model AC6 on these): decisions-file preflight at
  `:49-118` (`assert_neq "…" "0" "$RC"` + `assert_contains` on the named error);
  malformed session JSONL at `:824-852` ("unknown outcome" / unknown
  `schema_version`).
- **Per-key application** is verifiable via the fixture's sentinel log
  (`migration_apply_decision` appends `key\tpath\tanchor\tdecision\tvalue` to
  `.fixture/applied/log`, `0002-predicate.sh:44-58`) — lets AC2 assert which keys
  applied with which values, and AC6 assert nothing was written.

> **Note for the standalone fixture (AC1–AC6):** the work item pins concrete
> literals (`relates_to`/`work-item:0042`, `meta/work/0050-example-a.md:body/
> relates_to`, etc.). The `0002-predicate` fixture is file-driven, so those
> literals are just the `key|path|anchor|proposed|band` values seeded into
> `.fixture/transformations`. AC1's byte-for-byte `--list` output and AC3's
> `no pending transformations` line are new assertions to add.

## Code References

- `skills/config/migrate/scripts/run-migrations.sh:29-73` — flag `if/case` block (convert to `while/shift`; add `--list`; no `*)` arm today)
- `skills/config/migrate/scripts/run-migrations.sh:61-71` — `--help` text (AC4: add the env var)
- `skills/config/migrate/scripts/run-migrations.sh:14-23` — env-var resolution + ticket-naming comment
- `skills/config/migrate/scripts/run-migrations.sh:75-92` — decisions-file validation (filesystem-only; AC6 content checks go here)
- `skills/config/migrate/scripts/run-migrations.sh:94-168` — dirty-tree guard (decide whether `--list` precedes it)
- `skills/config/migrate/scripts/run-migrations.sh:277,279-324` — late `source interactive-lib.sh` + apply loop (the dry-emit `exit 0` boundary)
- `skills/config/migrate/scripts/interactive-lib.sh:240-298` — `read_decision()` (verbs, CRLF, three-valued return)
- `skills/config/migrate/scripts/interactive-lib.sh:306-355` — `emit_no_input_stall` + `read_decision_or_stall` (0116 stall; shared region)
- `skills/config/migrate/scripts/interactive-lib.sh:377-383,606-608` — fd 9 decisions-file wiring
- `skills/config/migrate/scripts/interactive-lib.sh:204,213,509-515` — PROMPT relay + `path:anchor` join + display counter
- `scripts/interactive-harness.sh:272-287,320-364` — child TX buffering, predicate filter, PROMPT emission (the natural list-mode emit point)
- `scripts/interactive-protocol.sh:26` — PROMPT wire field contract
- `skills/config/migrate/SKILL.md:89-214,216-224` — author-side contract + execution guidance (invoker section goes at 214/216 boundary)
- `skills/config/migrate/scripts/test-migrate-interactive.sh:360-369,380-404,49-118,1271` — fixture seeder, decisions-file + frame assertions, fail-closed precedent, `test_summary`
- `skills/config/migrate/scripts/test-fixtures/interactive/0002-predicate/migrations/0002-predicate.sh:13-58` — reusable file-driven fixture migration

## Architecture Insights

- **Decisions are positional and keyless by design** (ADR-0037 §4 verbs;
  0115 cross-cutting constraint). `--list` output ordering is therefore
  load-bearing: it is the only thing that lets an agent map a list entry to a
  decisions-file line. The mapping is to *decision-requiring* prompts, not
  transformations — a distinction the plan must encode in `--list`'s position
  numbering.
- **The cleanest `--list` lives in the child, not the runner.** The child already
  buffers the full transformation list (`TX_LINES`) and applies the same
  predicate filter that decides which transformations prompt. Emitting the list
  from there — before the decide handshake — avoids the runner's
  "skip-still-records" side effect and reuses the single source of truth for
  ordering and filtering. This is the central design decision for AC1/AC3.
- **bash 3.2 floor, ASCII-only, 80-col** apply to all new shell (macOS CI). The
  existing fd-9 literal and the `if/case`→`while/shift` conversion are both bash
  3.2 considerations.
- **Contract-ownership split with 0116 is explicit**: 0116 owns the
  `--decisions-file` flag and its minimal `--help` line; 0117 owns env-var
  `--help` promotion (AC4), `--list`, content validation (AC6), and the SKILL.md
  invoker section (AC5). 0117 must **not** add a second `--decisions-file` flag.
- **ADR-0037 read**: the invoker bridge adds no new control verb, display
  element, or resumability guarantee, and ADR-0037 is neutral on *how* the runner
  is invoked — so it sits as an implementation detail under the existing
  decision, not an amendment. The recursive-supplement clause is the test; 0117
  should record this judgment (resolves the work item's Open Question / 0115's
  inherited open question).

## Historical Context

- `meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md`
  — source RCA. Under agent invocation all three `read_decision` input sources
  are unreachable; the run is structurally unsatisfiable *after* the corpus is
  already mutated. **`--list` exists because the agent cannot populate the
  decisions file blind** — proposed values are only revealed by the prompts. The
  research ranked fix A "better as a follow-up than the first move" because it
  "still asks the agent to make editorial linkage judgments arguably the human's
  to make."
- `meta/work/0115-…-satisfiable-under-agent-invocation.md` — parent. Chose fix A
  and **explicitly rejected fix D** (split 0007). Pins the decisions-file format
  contract and notes `--list` "must preserve consumption order." Leaves the
  ADR-0037 amendment question open for 0117.
- `meta/plans/2026-06-20-0116-structured-stall-on-no-decision-input.md` +
  `meta/research/codebase/2026-06-20-0116-structured-stall-on-no-decision-input.md`
  — 0116 (landed). Documents the stall marker/exit contract, the three-valued
  `read_decision` return, and the explicit directive that 0117 "must not collapse
  any non-zero to a generic failure" and that `--list` forces the
  `if/case`→`while/shift` conversion.
- `meta/decisions/ADR-0037-optional-interactive-contract-supplement-to-adr-0023.md`
  — the contract (accept/edit/skip verbs, three display elements, resumability)
  and its recursive-supplement clause; neutral on invocation mechanics.
- `meta/work/0118-reconcile-0007-backfill-sentinel-with-validator.md` — fix C.
  Live 0007 hard-aborts in `self_validate_structural` (MISSING-EXTRA) before its
  interactive stage; the `unknown` sentinel (supersedes the research's `pending`)
  clears it. This is why **AC7 is gated on 0118 and out of scope for 0117**, and
  why 0117's fixture is standalone.
- `meta/reviews/work/0117-agent-decisions-bridge-and-invoker-contract-review-1.md`
  — prior work-item review (context).

## Related Research

- `meta/research/codebase/2026-06-20-0116-structured-stall-on-no-decision-input.md`
  (the immediately adjacent sibling)
- `meta/research/codebase/2026-06-17-0114-migration-0007-incomplete-mechanical-normalisation.md`
- `meta/research/codebase/2026-05-30-0069-migration-framework-interactive-validation-hooks.md`
- `meta/research/codebase/2026-05-26-0092-adr-optional-interactive-contract-for-migration-framework.md`

## Open Questions

1. **`--list` emit site**: child-side list mode (dump `TX_LINES`, exit before the
   decide handshake) vs a runner-only flag that feeds skips. Findings strongly
   favour child-side — it reuses the single source of truth for ordering and the
   predicate filter, and avoids the skip-records-a-session-log side effect. The
   plan should confirm this and define how the child is invoked in list mode
   (a new protocol mode or an env/flag the runner passes through).
2. **Should `--list` bypass the dirty-tree guard?** A dry, read-only list
   arguably should not require a clean tree, but the guard is currently
   unconditional (except `ACCELERATOR_MIGRATE_FORCE`).
3. **ADR-0037 amendment vs implementation detail** (work item Open Question /
   inherited from 0115): findings support "implementation detail," but 0117 must
   record the judgment.
4. **Position numbering for `--list`** when mechanical/resumed transformations or
   VALIDATE_ERR re-prompts are in play — confirm `<position>` is computed over the
   decision-requiring subset so it stays 1:1 with decisions-file consumption.
   (The standalone fixture's three `ambiguous` rows sidestep this, but the
   contract documented in SKILL.md should be precise.)
