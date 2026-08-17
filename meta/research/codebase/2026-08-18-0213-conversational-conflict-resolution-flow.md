---
type: codebase-research
id: "2026-08-18-0213-conversational-conflict-resolution-flow"
title: "Research: Implementation ground for 0213's conversational conflict resolution flow"
date: "2026-08-17T23:16:08+00:00"
author: "Toby Clemson"
producer: research-codebase
status: complete
work_item_id: "0213"
parent: "work-item:0213"
relates_to: ["codebase-research:2026-08-12-0194-tracker-crate-and-remote-sync-engine", "codebase-research:2026-08-11-0204-remote-tracker-port", "codebase-research:2026-06-18-0051-sync-work-items-skill"]
topic: "Conversational conflict resolution flow for /sync-work-items"
tags: [research, codebase, skills, sync, work-items, conflicts, tracker]
revision: "b8cc72701ab72574d9ef2d761113e0fb4488ead1"
repository: "accelerator"
last_updated: "2026-08-19T01:14:08+00:00"
last_updated_note: "0210 landed and the repo rebased onto it — clients are now wired, inverting the never-exercisable finding; see the second follow-up"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Research: Implementation ground for 0213's conversational conflict resolution flow

**Date**: 2026-08-18T00:16:08+01:00 (2026-08-17T23:16:08Z)
**Author**: Toby Clemson
**Git Commit**: `c740556af9b9fc9c4777b75e60f5af9e283dbfb3`
**Branch**: no bookmark — empty working-copy change atop `main` (`77621c9b866b`)
**Repository**: accelerator

## Research Question

What is the implementation ground for `meta/work/0213-conversational-conflict-resolution-flow.md` — adding the conversational half of 0194's two-invocation conflict flow to `skills/work/sync-work-items/SKILL.md`?

## Summary

> ⚠️ **Superseded in part by the second follow-up (2026-08-19).** 0210 is now
> `done` and the repo is rebased onto it, which **inverts** this section's
> fourth finding: `work sync` now wires real `jira`/`linear` clients, reaches
> the run engine, and prints the report in production — with credentials
> present (they are, in this repo's `config.local.md`) a `--preview` exits `0`
> with a real report. The report-contents findings below still hold verbatim;
> the "never printed in production / exits 72 / no credentialed target"
> statements do not. Read the second follow-up for the current state.

**0213's ⚠️-marked size-bounding assumption is false, and the work item told us what that means: this is not a `SKILL.md`-only change, it cannot land first, and it grows into 0194's binary.** `accelerator work sync`'s report carries one of the six required render fields.

Four findings drive everything else.

**The report carries the work-item id and nothing else of the six.** Its record type is `PlannedAction { id, state, action }` (`cli/work/src/sync/plan.rs:29-34`) and its line is `<id>\t<action>\t<state>\t<detail>` (`cli/work-cli/src/sync.rs:186-189`). No title, no differing field name, no local value, no remote value, no timestamps. This is not a shortfall against 0194's design — the plan specified exactly this shape and defended it (`meta/plans/2026-08-13-0194-tracker-crate-and-remote-sync-engine.md:374-376`: "One tab-separated line per item — `<id>\t<action>\t<state>` — for **every** classified item"). Field-level diffing does not exist in the domain at all: classification is whole-document hash comparison (`cli/work/src/sync/classify.rs:105-129`).

**The conversational flow already exists in the SKILL and already renders those fields — against the bash cluster, not the Rust binary.** `skills/work/sync-work-items/SKILL.md:193-237` is a complete conflict loop: a section-grouped `accelerator work diff <local-file> <remote-reconstructed-file>` render, a pinned typed-token `[remote/local/skip]` prompt with no Enter default, and all three resolution branches. So 0213 is not "add the conversational half" — it is "re-point an existing conversational half at a binary that can no longer feed it", and the thing that must be built is the missing data path, not the prompt.

**The premise "this is live, user-visible degradation" is false today.** 0171 itself records that "The bash path stays the production path until this story lands" (`meta/work/0171-jira-and-linear-integrations.md:69-70`) while simultaneously claiming `/sync-work-items` "can detect a conflict but cannot resolve one" (`:53-55`). The second claim is true only of the Rust path. The regression 0213 fixes is one **0212 would introduce**, which couples 0213 to 0212 and destroys its stated independence.

**The flow can never be exercised end-to-end today.** `ConfiguredTrackers` has no client for any provider (`cli/work-cli/src/tracker_registry.rs:42-64`), so every real `work sync` exits 72 before the run engine is reached and the report is never printed in production. Verified empirically below.

⚠️ Two AC-level defects follow from the exit-code analysis alone, independent of the above: the report also accompanies exit **70**, and `skip-conflict`/`skip-dirty` items await a human without emitting an `unresolved` line.

## Detailed Findings

### The conflict report: what it actually carries

Emitted to **stdout** by a single `println!` (`cli/work-cli/src/sync.rs:351`). Everything else — warnings, refusals, errors — goes to stderr. There is no `--json`.

Two line shapes, both four tab-separated columns:

```text
<id>\t<action>\t<state>\t<detail>
#\tsummary\tsynced\t<count>
```

Column 2 vocabulary (`sync.rs:152-161`, plus the literal `failed`): `push`, `pull`, `skip-conflict`, `skip-dirty`, `unresolved`, `noop`, `failed`. Column 3 is the `SyncState` keyword (`cli/work/src/sync/state.rs:34-46`): `synced`, `unsynced`, `locally-modified`, `remotely-modified`, `conflict`, `remote-absent`, `indeterminate`. Column 4 is `retryable`, `terminal` or `-`.

Against 0213's six required render fields:

| Field | Present | Where it lives instead |
|---|---|---|
| work-item id | 🟢 yes | column 1 |
| title | 🔴 absent | read locally at `cli/work-adapters/src/sync/run.rs:97-104`, never reaches the report |
| differing field name | 🔴 absent | no field-level diff exists anywhere in the domain |
| local value | 🔴 absent | only hashes are compared |
| remote value | 🔴 absent | fetched in-process, never persisted |
| local + remote timestamps | 🔴 absent | exist on `BaselineEntry`/`RemoteTimestamp`, not on `PlannedAction` |

**Requirement 4 resolves favourably by accident.** The report cannot emit multiple differing fields for one id, because it has no field dimension: one `PlannedAction` per item (`plan.rs:110-135`), one `ReportedItem` per planned action (`run.rs:205-288`), one line per reported item (`sync.rs:166-189`). No fixture is needed for the multi-field case — it is unrepresentable.

### Why the remote side is unreachable from the skill

The existing SKILL renders a conflict by diffing the local file against a **remote-reconstructed file** (`SKILL.md:200-201`). The bash path produces that file: `work-item-fetch-remote.sh … show` fetches the issue, `work-item-project-remote.sh … body` projects it, and the result is passed around as `--remote-body-file` (`work-item-sync-classify.sh:179-181`, `work-item-sync-apply.sh:175`).

The Rust path has the same reconstruction — `reconstruct_pulled_content` (`cli/work-adapters/src/sync/run.rs:106-114`) — but it is a private function on the *pull* path only. For a `Prompt` item nothing is written and nothing is exposed. The baseline on disk holds `remote_updated_at`, `remote_hash` and `local_hash` (`.accelerator/state/integrations/linear/last-sync.json`) — hashes, not content.

So there is no route from `accelerator work sync` to a renderable remote side. Closing 0213 requires a new CLI surface: sync writing the reconstructed remote for each conflicted item, or a subcommand that fetches and projects one item on demand. Either is a change to 0194's binary. ADR-0045 points the same way — skills own probabilistic work, the CLI owns deterministic work (`meta/decisions/ADR-0045-skills-vs-cli-division-of-labour.md:69-75`), so reconstructing a remote body in-prompt is the wrong side of the line.

### Exit codes, verified empirically

Full set from `cli/work-cli/src/exit_codes.rs:5-15`, reachable from `sync`: `0` clean, `1` internal error, `2` usage, `4` awaiting human, `5` refused bulk overwrite, `70` retryable, `71` terminal, `72` tracker has no client, `73` integration unset or unrecognised.

⚠️ **The report accompanies 0, 4, 70 and 71 — not the three the work item names.** It is printed only on the `Ok(report)` path (`sync.rs:351`), and `exit_code_for_report` (`sync.rs:198-224`) yields exactly those four, in the precedence terminal (71) > awaiting-human (4) > retryable (70) > clean (0). The work item's "only codes it can accompany" claim traces to the doc comment at `cli.rs:78-87`, which itself omits 70. Codes 5, 72 and 73 return before any line is printed.

⚠️ **Branching on `unresolved` lines alone under-reports.** `RunReport::awaiting_human` (`cli/work-adapters/src/sync/run.rs:84-94`) counts `Prompt | SkipConflict | SkipDirty` actions **and** `RemoteAbsent | Indeterminate` states, but only `Action::Prompt` renders the keyword `unresolved` (`sync.rs:158`). An exit-4 run can therefore carry zero `unresolved` lines while genuinely awaiting a human — the flow would report "no conflicts" on a 4.

Empirical confirmation, run against the installed `1.24.0-pre.42` launcher in this repo:

```console
$ accelerator work sync --preview
work.integration names 'linear', which is recognised but has no client wired yet.
$ echo $?
72
$ accelerator work sync --preview --resolve 0213=remote --resolve 0212=local --resolve 0194=skip
work.integration names 'linear', which is recognised but has no client wired yet.   # exit 72
$ accelerator work sync --resolve-conflict 0213=remote
error: unexpected argument '--resolve-conflict' found                                # exit 2
$ accelerator work sync --preview --resolve 0213                                     # exit 2
error: invalid value '0213' for '--resolve <RESOLUTIONS>': expected KEY=VALUE, got '0213'
$ accelerator work sync --preview --resolve 0213=bogus
warning: --resolve 0213=bogus — 'bogus' is not a recognised order (accepted: remote, local, skip); treating as skip
work.integration names 'linear', which is recognised but has no client wired yet.   # exit 72
```

✅ This settles AC4 favourably: the argv replay is deterministic, offline, credential-free, and today returns 72 rather than the usage-error code 2. **Name 2 as the usage-error code** — the AC asks for it and it is `exit_codes::USAGE`.

### `--resolve` semantics

Declared at `cli/work-cli/src/cli.rs:203-207`, repeatable, `KEY=VALUE` split on the first `=`. Three tokens, ASCII-lowercased and whitespace-trimmed (`cli/work/src/sync/decide.rs:122-131`): `remote` → `AcceptRemote`, `local` → `PushLocal`, `skip` → `Skip`. Resolutions are keyed by id only, in a `BTreeMap` (`sync.rs:128-150`); only a `Prompt` action is rewritten (`cli/work/src/sync/plan.rs:120-128`), so an order naming any other id is silently inert.

Three behaviours the SKILL body must account for:

- ⚠️ **A repeated id is a usage error** (`sync.rs:131-137`, exit 2). One order per id, never two — which is also why per-item rather than per-field prompting is right.
- ⚠️ **An unrecognised token is not an error.** It warns on stderr and becomes `skip` (`sync.rs:138-146`). Today's SKILL re-asks once before falling to skip (`SKILL.md:234-237`); routing the raw token straight into `--resolve` loses that, and the shell helper that normalised it (`work-item-sync-decide.sh resolve-conflict-token`) is deleted by 0212 with no CLI replacement. The SKILL must normalise before emitting.
- **Sync cannot be scoped to one id.** There are no positional arguments; it always walks the whole work directory (`sync.rs:89-125`).

### What `sync-work-items/SKILL.md` already does

349 lines. Frontmatter `allowed-tools` already permits `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator work *)`, so `work sync` needs no permission change.

`### Conflict resolution (bidirectional only)` (`:193-237`) is the flow 0213 proposes to add, already written:

```text
Conflict on <id> (<external_id>). Recommended: keep remote.
Type 'remote' to OVERWRITE your local edits with the remote version,
'local' to push your local version to the remote, or
'skip' to leave both unchanged and resolve it later. [remote/local/skip]
No default — Enter (or an unrecognised entry) re-asks once, then skips.
```

preceded by the `accelerator work diff` render and followed by three branches mapping to pull, push (with an `OVERRIDE` summary line), and skip. The string `--resolve` does not appear in the file; `accelerator work sync` is not invoked anywhere. Every sync-specific step goes through `skills/work/scripts/work-item-*.sh`.

⚠️ The typed-token prompt is deliberately *not* `AskUserQuestion` — `:203-206` justifies it as anti-collision with the `[y/N]` gates and anti-reflexive-Enter. `AskUserQuestion` is used in this file only for the two aggregate blast-radius gates (`:176-181`, `:292-298`). Preserve that split. Other skills' per-item choose-then-act loops (`extract-work-items:206-222`, `github/respond-to-pr:303-356`) use `AskUserQuestion`, but they are not resolving destructive overwrites.

Also stale and worth fixing in passing: `:35` still names `config-read-work.sh` for the config gate that `:16` actually performs with `accelerator config work integration --fail-safe`.

### The surface 0213 depends on is unfrozen and untested

⚠️ **The report has no golden and no byte-comparison test.** 0194's plan named `cli/work-cli/tests/fixtures/sync-report.golden` as a Phase 4 deliverable; the file does not exist — `cli/work-cli/tests/fixtures/` holds only `cli_surface.golden`. The 0194 validation (`result: partial`) records the cause: "All five phases ship working code, and every box in the plan is ticked… what is incomplete is the test matrix Phases 4 and 5 specify." Its Finding 1 names the conflict loop specifically as writable-today-but-unwritten at the `work-adapters` boundary with `RecordingTracker`. So 0213 would pin fixtures against a format that nothing else asserts.

Two smaller consequences of the same gap:

- ⚠️ **A latent ordering deviation.** The plan makes line order contractual — "ordered by **ascending work-item id**" — while the code does `lines.sort()` (`cli/work-cli/src/sync.rs:191`), a lexicographic sort of whole rendered lines. These coincide for zero-padded numeric ids, so it is latent rather than live, but a fixture encoding one order does not test the other.
- ⚠️ **Per-item inspection is expensive by construction.** 0194 explicitly declined `--only <id>`: "Every run classifies the whole corpus, so investigating one item costs a full pass and a `fetch_all`… recorded here so the absence is a decision rather than an oversight." Any design that re-invokes `sync` per conflict pays a full corpus classification each time.

### Testing surfaces

**Static assertion (AC1) — mechanisable, and the right shape is Python.** Four mechanisms assert on SKILL.md content today; the modern one is an invoke lint under `tasks/lint/` reusing `tasks/shared/skill_parsing.py`, plus a pytest unit test under `tests/unit/tasks/`. `tasks/lint/call_site_migration.py:26-40` is the smallest working template. The bash equivalent (`scripts/test-skill-frontmatter-population.sh`, run by `mise run test:unit:templates`) predates Python becoming the test language for non-Rust surfaces.

**Behaviour against a stub (AC3) — not mechanisable as specified.** A SKILL.md body is prose interpreted by the model; no shell script can drive it. The AC's "a script that puts the stub `accelerator` on `PATH`, drives the flow against one fixture, captures the transcript" has no executor in this repo. Three honest alternatives:

- 🔵 **An eval suite.** `skills/*/evals/evals.json` is the repo's existing prompt → expected-output → assertions mechanism (18 skills carry one; `sync-work-items` does not). Not wired into any `mise run` task — `claude plugin eval` is the runner.
- 🔵 **A recorded manual transcript**, committed as evidence.
- 🔵 **Drop AC3 to what is automatable** — the static assertion plus the argv replay.

⚠️ **`PATH` is the wrong seam anyway.** The repo's established stub seam for the launcher is the `ACCELERATOR_BIN` / `ACCELERATOR_PLUGIN_ROOT` overlay (`tasks/test/helpers.py:17-71`), honoured by every shell script. `PATH` shadowing is reserved for tools resolved by bare name (`jq`, `git`) — and `tasks/test/integration.py:195-255` documents why PATH stubs alone are insufficient when a binary is reachable by absolute path. The SKILL body invokes `${CLAUDE_PLUGIN_ROOT}/bin/accelerator` literally, which `ACCELERATOR_BIN` does not intercept; `CLAUDE_PLUGIN_ROOT` is the lever.

**Suite discovery.** Any executable `test-*.sh` anywhere under `skills/work/**` is auto-enrolled into `mise run test:integration:work` (`tasks/test/helpers.py:96-102`). The exec bit is the enrolment mechanism and `lint:scripts:exec-bits:check` enforces its inverse, so a new harness must be `0755` and absent from `SHELL_LIBRARIES`. Floors are at-least (`_EXPECTED_WORK_SUITES = 5`, `tasks/test/integration.py:51`), so adding a suite is safe — but 0212 removes that floor outright along with all five suites.

**Fixture location.** The repo convention is `<subtree>/scripts/test-fixtures/`, not `<skill>/test-fixtures/`. ⚠️ `skills/work/scripts/test-fixtures/` — the nearest precedent, holding the goldens shared with `cli/work/tests/` — is **deleted wholesale by 0212**, which relocates every fixture into the Rust test tree (`meta/work/0212-work-item-script-cutover.md:81-88`). Siting new fixtures under `skills/` while a sibling is emptying that tree needs a deliberate justification. `test-fixtures/` is at least pruned from SKILL.md discovery walks (`tests/conftest.py:67`).

**Evidence artefacts.** No `evidence/`, `walkthrough/` or `transcripts/` directory exists anywhere in the repo. The only committed machine-captured evidence precedent is `meta/research/design-inventories/<timestamp>-<name>/screenshots/`. 0171 holds two `pending` Decisions entries for this evidence — one attributed to 0213 (`:230-231`) and a bare duplicate (`:240`); one of them is redundant.

### Ordering and the independence claim

0171 sequences 0210 → 0212 → 0211 "with 0213 free" (`:147`), justifying 0213 landing first because "`/sync-work-items` cannot resolve a conflict today" (`:142-145`). The evidence contradicts this on three counts:

1. The shipped skill **does** resolve conflicts, via the bash cluster, which 0171 itself calls "the production path until this story lands" (`:69-70`).
2. The report cannot feed the render, so 0213 needs a 0194-binary change — which needs the run engine reachable, which needs 0210's clients.
3. 0213 without 0212 edits a conflict section still wired to scripts 0212 deletes; landing it first means writing the section twice.

The realistic sequence is **0210 → 0194-follow-up (expose the remote side) → 0212 + 0213 together**, since both edit the same section of the same file and 0212's requirement list preserves only the dirty-work-item guard (`0212:73-80`), not the conflict render.

## Code References

- `cli/work/src/sync/plan.rs:29-34` — `PlannedAction { id, state, action }`, the whole conflict record
- `cli/work-cli/src/sync.rs:163-196` — `render_report`, the four-column format strings
- `cli/work-cli/src/sync.rs:198-224` — `exit_code_for_report`; 0, 4, 70, 71 all carry a report
- `cli/work-cli/src/sync.rs:128-150` — `parse_resolutions`; duplicate id is exit 2, bad token warns and skips
- `cli/work-cli/src/exit_codes.rs:5-15` — the exit-code constants; `USAGE = 2`
- `cli/work-cli/src/cli.rs:203-207` — the `--resolve` flag declaration
- `cli/work/src/sync/decide.rs:122-131` — `resolve_conflict_token`, the three-token set
- `cli/work/src/sync/classify.rs:105-129` — whole-document hash comparison; no field-level diff
- `cli/work-adapters/src/sync/run.rs:84-94` — `awaiting_human` counts more than `unresolved` lines
- `cli/work-adapters/src/sync/run.rs:106-114` — `reconstruct_pulled_content`, private, pull-path only
- `cli/work-cli/src/tracker_registry.rs:42-64` — `ConfiguredTrackers`; no provider has a client
- `cli/work-cli/tests/cli_sync.rs:3-8` — why the conflict loop is untestable from a subprocess
- `skills/work/sync-work-items/SKILL.md:193-237` — the conflict loop that already exists
- `skills/work/sync-work-items/SKILL.md:7-11` — `allowed-tools` already covers `work *`
- `tasks/test/helpers.py:74-107` — `test-*.sh` auto-discovery under a subtree
- `tasks/test/helpers.py:17-71` — the `ACCELERATOR_BIN` stub overlay
- `tasks/lint/call_site_migration.py:26-40` — smallest template for a SKILL.md content lint
- `tasks/test/integration.py:45-78` — the suite floors
- `cli/work-cli/tests/fixtures/` — holds only `cli_surface.golden`; the specified `sync-report.golden` is absent

## Architecture Insights

**The report is a plan ledger, not a diff.** 0194 chose fixed arity so a consumer can split and read field 3 unconditionally, matching `migrate-cli --list` (`0194 plan:374-383`), and chose to emit every classified item so a total read failure is distinguishable from a clean corpus. Both choices are sound and neither leaves room for per-field detail. Adding render fields means a second surface, not a wider line.

**ADR-0045 decides where the missing data comes from.** Skills own judgement; the CLI owns deterministic work. Reconstructing a remote work item in-prompt from a fetched body would put deterministic projection back in the model — precisely what the bash-to-Rust migration is unwinding.

**The two-invocation shape is sound and worth keeping.** Non-interactive binary plus `--resolve` on re-invocation avoids stdin entirely and is regression-tested (`cli_sync.rs:126-134`). The gap is purely in what the first invocation tells the caller.

**Skill-body behaviour has no automated guard in this repo.** Four mechanisms assert SKILL.md *content*; none asserts what the model *does* with it, apart from the unwired `evals/` corpora. Any AC promising behavioural verification of a prose flow is either an eval, a manual transcript, or aspirational.

## Historical Context

- `meta/plans/2026-08-13-0194-tracker-crate-and-remote-sync-engine.md:374-390` — the report contract as designed: fixed arity, `<id>\t<action>\t<state>`, failure class as a fourth field. Never intended to carry title, values or timestamps.
- `meta/work/0171-jira-and-linear-integrations.md:53-70` — the degradation claim and the "bash path stays the production path" statement, in tension.
- `meta/work/0171-jira-and-linear-integrations.md:433-435` — the `0, 4, 71` claim, corrected against `cli.rs`; 70 was missed.
- `meta/work/0212-work-item-script-cutover.md:66-88` — repoints the same SKILL.md and deletes `skills/work/scripts/test-fixtures/` entirely.
- `meta/reviews/work/0213-conversational-conflict-resolution-flow-review-1.md:70-79` — the review raised the six-field assumption as a major and predicted exactly this outcome; it was **accepted rather than resolved** (`:240-266`), along with the finding that the stub-on-`PATH` seam defeats the malformed-template predicate.
- `meta/validations/2026-08-13-0194-tracker-crate-and-remote-sync-engine-validation.md` — `result: partial`; Phase 4/5 test deliverables absent, the conflict loop and report rendering uncovered.
- `meta/plans/2026-08-13-0194-tracker-crate-and-remote-sync-engine.md` — "What We're NOT Doing" defers the conversational flow to 0171 and declines `--only <id>`.
- ⚠️ `meta/work/0171-jira-and-linear-integrations.md` Drafting Notes still record the **refuted** ordering (0210 → 0211 → 0212, on the sync-label rationale disproved on 2026-08-17). Decomposition carries the corrected 0210 → 0212 → 0211. A reader starting at the notes gets the wrong order; 0213 is free in both.
- `meta/decisions/ADR-0045-skills-vs-cli-division-of-labour.md:69-75` — the skills-versus-CLI rule.
- `meta/decisions/ADR-0044-remote-work-item-identity-in-external-id.md` — `external_id` as the join key any conflict render must display.

## Related Research

- `meta/research/codebase/2026-08-12-0194-tracker-crate-and-remote-sync-engine.md`
- `meta/research/codebase/2026-08-11-0204-remote-tracker-port.md`
- `meta/research/codebase/2026-06-18-0051-sync-work-items-skill.md`

## Open Questions

- ❓ **Which surface exposes the remote side?** `work sync` writing a reconstructed remote file per conflicted item into the integrations state dir, versus a `work sync --show-conflict <id>` or a `work remote show` subcommand. The first keeps one invocation; the second is scopable, which matters because `sync` has no per-id form.
- ❓ **Does 0213 stay a separate child, or fold into 0212?** Both edit the same section; 0213's data dependency lands in 0194's binary and its ordering claim does not survive.
- ❓ **What replaces the token-normalisation helper?** `resolve-conflict-token` dies with 0212 and `--resolve`'s warn-and-skip is a silent downgrade. Either the SKILL normalises in-prompt or the CLI grows a validating surface.
- ❓ **Does the report format get frozen first?** Writing 0213's fixtures against an unasserted format inverts the usual order. A `sync-report.golden` plus the deferred `work-adapters` conflict-loop tests would freeze it — arguably a 0194 follow-up that should precede 0213 either way.
- ❓ **How is AC3 discharged?** Eval suite, committed transcript, or scope reduction to static-plus-replay.
- ❓ **Should the flow branch on the report at all, or on exit 4?** Given `skip-conflict`/`skip-dirty`/`remote-absent`/`indeterminate` await a human without an `unresolved` line, the correct predicate is probably "exit 4 or any non-`noop`, non-`push`, non-`pull` line", not a keyword grep.


## Follow-up Research — 2026-08-18T00:43:52+01:00 (2026-08-17T23:43:52Z)

**Question**: 0194 is `done`, so any CLI change belongs to 0213. What is that change, concretely, and what does it do to the ordering conclusion?

### Every missing field already exists inside the run

The extension is an **output-surface problem only**. No new port operation, no new fetch, no new domain logic:

| Field | Already computed at |
|---|---|
| differing field name | `SectionDiff.name` — `cli/work/src/section_diff.rs:153-157` |
| local value | `SectionDiff.local` — same struct |
| remote value | `SectionDiff.remote` — same struct |
| title | `local_title_and_body` — `cli/work-adapters/src/sync/run.rs:97-104` |
| local timestamp | `ItemDigests::mtime` — `cli/work/src/sync/classify.rs:25` |
| remote timestamp | `Subject.remote_updated: &RemoteTimestamp` — `classify.rs:54-59` |

`differing_sections(local, remote) -> Vec<SectionDiff>` (`section_diff.rs:176-179`) is public, and its section label already resolves to `frontmatter`, `(preamble)` or the heading text (`:159-165`). That *is* the "differing field", and it is the right granularity: sync compares projected **body** hashes, so a conflict's differing sections are body sections.

The one genuinely absent input is the reconstructed remote side. `reconstruct_pulled_content` (`run.rs:106-114`) builds it — local frontmatter plus remote body — but is private and runs on the pull path only. A `Prompt` item never materialises it.

### Two shapes for the surface

The report's four-column arity is contractual, so the detail cannot ride on those lines. Both options write into `paths.integrations/<tracker>/` (`.accelerator/state/integrations`, `cli/config/src/catalogue.rs:48-49`), which already has a precedent for transient per-run state — `pending-push/` is gitignored at `.gitignore:92` with the reasoning recorded there, and each tracker dir carries its own `.gitignore`.

- 🔵 **Dossier plus `work diff`.** Sync writes the reconstructed remote to `conflicts/<id>.md`; the skill runs `accelerator work diff <local> conflicts/<id>.md`. Reuses the subcommand whose help already reads "for conflict-resolution review", so the render is existing, tested code. Title and timestamps still need a second source.
- 🔵 **Self-contained rendered dossier.** Sync writes one file per conflicted item carrying all six fields already rendered — header block for id/title/timestamps, then the `differing_sections` render. The skill reads one file, prints it, prompts, re-invokes. One call, no further extraction.

**Recommendation: the self-contained dossier.** ADR-0045 puts deterministic extraction and rendering in the CLI and leaves the skill only the prompt and the choice; the first option leaves the skill assembling three sources. The cost is that `work diff`'s renderer gets a second caller rather than being the entry point — cheap, since both go through `work_adapters::diff_shellout::render`.

⚠️ **Both options give a conflict-only run side effects.** Today a conflict writes neither side, which is the safety property `0194 plan` defends. Writing dossiers does not touch work items, but it does mean `--preview` stops being write-free unless dossiers are explicitly exempted or suppressed under preview. Decide this deliberately.

⚠️ **The rendering path can fail.** `diff_shellout::render` spawns real `diff -u` with a 10-second cap and returns `DiffUnavailable` if the binary is missing or slow (`cli/work-adapters/src/diff_shellout.rs:18-33`). It is the crate's only subprocess-spawning module, isolated by a `pup.ron` rule. The flow needs a defined behaviour when a conflict cannot be rendered — prompting blind is not acceptable, so the honest fallback is to report the item as unrenderable and leave it unresolved.

⚠️ **Two of the six fields are legitimately absent-able.** `RemoteTimestamp` has `NotReported` and `NotRead` alongside `Reported(String)` (`cli/tracker/src/lib.rs:51-70`), and `ItemDigests::mtime` returns `Ok(None)` when no mtime is available (`classify.rs:20-25`). An AC demanding all six fields populated on every conflict is unsatisfiable in general; it needs an explicit rendering for absent, and the fixtures should exercise it.

### This restores most of 0213's independence — correcting the earlier conclusion

The Summary above concluded that 0213 needs 0210 first. **With the CLI work in scope, that is wrong for everything except a live end-to-end run**, which 0213's acceptance criteria never demanded.

A complete `Prompt` conflict is already driven at the `work-adapters` boundary with no provider client: `cli/work-adapters/tests/sync_run.rs:451-500` seeds a stale-both-sides baseline against `RecordingTracker::holding(...)` and asserts `Action::Prompt` and `awaiting_human().count() == 1`. The dossier's contents, the six fields, the absent-timestamp cases and the `--resolve` round trip are all testable there today. ✅ This is the seam 0213 should use — it is deterministic, offline, and already exists, unlike the stub-on-`PATH` the acceptance criteria specify.

Revised sequencing: **0213 (CLI extension + skill flow) can proceed now**, independent of 0210. It remains coupled to 0212, which repoints the same section of the same file; landing them together still avoids writing that section twice.

### What this adds to 0213's scope

- A CLI surface change in `cli/work-cli` and `cli/work-adapters`: materialise the reconstructed remote for `Prompt` items and render the six fields.
- Rust tests at the `work-adapters` boundary via `RecordingTracker`, including the absent-timestamp and `DiffUnavailable` paths.
- ⚠️ `cli/work-cli/tests/fixtures/sync-report.golden` — specified by 0194's Phase 4, never written. Freeze the report format here rather than pinning skill fixtures against an unasserted surface.
- A decision on `--preview` and dossier side effects, plus the gitignore entry.
- The skill-side edits, now reduced to: read dossier, prompt, normalise the token, emit one `--resolve` order per id.

The public-API surface of `work`/`work-adapters` changes, so `cargo-public-api` and `cargo-pup` checks apply (see `tasks/README.md`).

## Follow-up Research — 2026-08-19T02:14:08+01:00 (2026-08-19T01:14:08Z)

**Question**: 0210 is `done` and the repo is rebased onto it (`main` at
`d3c73ae7`, working copy `b8cc7270`). What does that change?

**Answer**: it inverts the central "never exercisable" finding and hands 0213
its test seam, its evidence convention and a new exit code — while leaving every
report-contents finding, every code anchor and the core scope conclusion intact.
0210 wired the provider clients; it added **no** conflict-render surface, so
0213's CLI half is unchanged in substance and now unblocked in fact.

### The clients are wired — verified against a freshly built binary

`ConfiguredTrackers::resolve` now resolves real clients for `jira` and `linear`
(`cli/work-cli/src/tracker_registry.rs:154-188`): `JiraClient::from_config` and
`LinearClient::from_config`, through a `credential_context` that reads
`.accelerator/config.local.md` behind a `VcsProvenance` trust boundary
(`:113-127`). Only `trello` and `github-issues` still report `NotAvailable`
(72); `Unset`/`Unrecognised` stay 73.

⚠️ The installed launcher (`1.24.0-pre.43` in the plugin cache) is **stale** — it
still prints "no client wired yet" and exits 72. The findings below come from a
binary built from the rebased source (`cargo build -p accelerator-work`,
`cli/target/debug/accelerator-work`). Do not probe the shipped launcher; it
predates the rebase.

This repo carries a live Linear token in `.accelerator/config.local.md` (the
file is untracked, so `VcsProvenance` trusts it). Against the fresh binary:

```console
$ accelerator-work sync --preview
0106	push	locally-modified	-
0107	push	locally-modified	-
...                                           # exit 0, a real report
```

⚠️ That `--preview` **hit the live Linear API** — remote reads still occur under
`--preview`. A deterministic test must not depend on it; use the offline seams
below.

### New exit code: 74 `UNCONFIGURED`

`cli/work-cli/src/exit_codes.rs:16` adds `UNCONFIGURED: u8 = 74` — "the
configured tracker is wired but its configuration or credentials are missing
(nothing was sent)". `run_sync` maps `SelectionError::Unconfigured` to it
(`cli/work-cli/src/sync.rs:275-277`). The `--resolve` argv, re-verified against
the fresh binary:

| Invocation | Exit | Meaning |
|---|---|---|
| valid `--resolve id=token` over an empty/clean corpus | `0` | report printed, argv accepted |
| creds scrubbed | `74` | `UNCONFIGURED` — wired, no credentials |
| `--resolve-conflict …` (unknown flag) | `2` | `USAGE` |
| `--resolve 0213` (no `=`) | `2` | `USAGE` |
| `--resolve 0213=remote --resolve 0213=local` | `2` | `USAGE` — repeated id |
| `--resolve 0213=bogus` | `0` | warn on stderr, treated as `skip` |

**AC4 update.** The argv-acceptance criterion's "today returns 72" is now "today
returns `0` (this corpus is conflict-free) or `74` (creds absent)". Both remain
passes — neither is the usage-error `2`. `72` no longer appears for
`linear`/`jira`. The criterion should name `2` as the failure code and treat any
of `0`/`4`/`70`/`71`/`74` as acceptance.

### The report-contents findings are unchanged — anchors re-verified

The rebase shifted nothing material. Every cited anchor still resolves:
`PlannedAction` (`plan.rs:30`), `render_report` and its four-column format
(`sync.rs:163`, `:188`), `exit_code_for_report` (`sync.rs:198`),
`reconstruct_pulled_content` (`run.rs:106`), `local_title_and_body`
(`run.rs:97`), `SectionDiff`/`differing_sections` (`section_diff.rs:153`,
`:176`), `--resolve` (`cli.rs:207-208`), `resolve_conflict_token`
(`decide.rs:122`), `awaiting_human` (`run.rs:84`), `diff_shellout::render`
(`diff_shellout.rs:33`), and the `RecordingTracker` `Prompt` test
(`sync_run.rs:451`). The report still carries only id/action/state/detail, and
0210 added no dossier, no render surface, and no change to
`sync-work-items/SKILL.md`. **0213's CLI half is still to be built, unchanged in
substance.**

### 0210 hands 0213 a better offline test seam

The earlier follow-up pointed 0213 at `sync_run.rs`'s `RecordingTracker` scenario.
0210 added a stronger one:

- 🔵 **`cli/work-adapters/tests/sync_run_real_client.rs`** drives the real sync
  engine through the real `JiraClient`/`LinearClient` pointed at a
  `http_test_support::MockServer`, built via public constructors that accept a
  loopback base (`from_config` refuses one). It already threads a
  `resolutions: BTreeMap<String, Resolution>` (`:205`) — the `--resolve` round
  trip. This is where 0213's dossier and conflict-loop tests belong: it
  exercises the actual projection and classification path, not a trait double,
  and stays network-free.
- 🔵 **`cli/work-cli/tests/sync_resolves_real_client.rs`** pins the CLI-boundary
  resolution: creds present + empty corpus → exit `0`, no network call; token
  scrubbed → `Unconfigured` → exit `74`. This is the pattern for 0213's
  argv-acceptance criterion, run against the real binary offline.
- 🔵 **`cli/work-cli/tests/no_network_by_default.rs`** and
  `cli/.config/nextest.toml` establish the rule 0213 must follow: `work sync`
  now resolves real clients, so the only live-API test — the contract harness —
  is filtered out of the default profile (`not binary(=contract)`) and gated
  behind `ACCELERATOR_TRACKER_CONTRACT`. Any 0213 test must be offline
  (MockServer) or sit behind that gate.

### The evidence convention 0213's AC needs now exists

The earlier research reported "no `evidence/` … directory exists anywhere in the
repo". 0210 created one. `cli/tracker-test-support/src/evidence.rs` defines a
**reduced conformance-evidence format** — `name PASS|FAIL count Nms` — with
`render` emitting only those fields and `is_reduced` refusing a committed file
that carries a payload or a secret shape. A committed instance lives at
`cli/linear-client/tests/evidence/`. This is the precedent 0213's
walkthrough-evidence criterion should adopt: a reduced, secret-scrubbed,
committed artefact rather than a verbatim transcript. It also settles a latent
risk — a verbatim conflict transcript would carry remote issue bodies and could
leak a token; the reduced format is the safe shape.

### Revised sequencing — 0213 is now cleanly unblocked

The earlier "realistic sequence" (0210 → 0194-follow-up → 0212 + 0213) collapses.
0210 is done, the clients are wired, the offline mock seam and the evidence
convention exist, and no 0194 follow-up is needed because the report contents
0213 must add are a new surface, not a report-format change. **0213 can proceed
now.** Its only remaining coupling is 0212, which repoints the same section of
`sync-work-items/SKILL.md`; land them together to avoid writing that section
twice. The CLI half — materialise the reconstructed remote for `Prompt` items
and render the six fields — is unchanged from the first follow-up's scoping.

### What this adds or changes in 0213's scope

- Test the CLI half at `sync_run_real_client.rs` (MockServer + real clients),
  not only `sync_run.rs` (`RecordingTracker`). The mock seam exercises the real
  projection path the dossier renders from.
- Keep every 0213 test offline or behind `ACCELERATOR_TRACKER_CONTRACT`; a live
  Linear/Jira call must never enter `mise run`.
- Adopt the `evidence.rs` reduced format for the walkthrough-evidence artefact.
- The argv-acceptance criterion accepts `0`/`4`/`70`/`71`/`74` and fails only on
  `2`; drop the "returns 72" wording.
- `sync-report.golden` is **still absent** — freezing the report format is still
  outstanding and still belongs in 0213, now with the `evidence.rs` guard as a
  model for a secret-safe committed fixture.
