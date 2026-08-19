---
type: work-item
id: "0213"
title: Conversational Conflict Resolution Flow for Sync
date: 2026-08-17T11:17:18+00:00
author: Toby Clemson
producer: review-work-item
status: done
kind: story
priority: high
parent: "work-item:0171"
relates_to: ["work-item:0194", "work-item:0210", "work-item:0212"]
tags: [skills, sync, work-items, conflicts, cli]
last_updated: 2026-08-19T01:14:08+00:00
last_updated_by: Toby Clemson
schema_version: 1
---

# 0213: Conversational Conflict Resolution Flow for Sync

**Kind**: Story
**Status**: Done
**Priority**: High
**Author**: Toby Clemson

## Summary

Close the report → prompt → resolve loop for `/sync-work-items`. Extend
`accelerator work sync` to materialise a renderable **conflict dossier** for
every item it leaves unresolved, then repoint the existing conflict section of
`skills/work/sync-work-items/SKILL.md` at that dossier: render each conflict,
collect one choice per work item, and re-invoke with matching
`--resolve <id>=<remote|local|skip>` orders.

## Context

0194 shipped a machine-parseable report carrying `<id>`, `<action>`, `<state>`
and a failure-class `<detail>` — and nothing else. It carries no title, no
differing field, no local or remote value and no timestamps, by design: 0194's
plan made fixed arity contractual and pushed human-readable material to stderr
"where it can change freely". A consumer reading only the report cannot show a
user what is in conflict.

The conversational half is **not** missing. `sync-work-items/SKILL.md:193-237`
already renders a section-grouped diff, prompts with a typed
`[remote/local/skip]` token and branches three ways — against the bash
cluster, which fetches and projects the remote body so a diff has two sides.
That cluster
is the production path today, so a user hitting a conflict is not currently
stuck. The degradation this item prevents is the one **0212** introduces when it
repoints the same file at a binary that cannot feed the render.

0194 is `done`, so the CLI change belongs here rather than to a follow-up on a
closed story. It is small: every field the render needs already exists inside a
sync run — `SectionDiff { name, local, remote }` from
`work::section_diff::differing_sections`, the title from
`local_title_and_body`, the local mtime from `ItemDigests::mtime` and the remote
stamp from `Subject.remote_updated`. Only the reconstructed remote side is
unreachable: `reconstruct_pulled_content` builds it but is private and runs on
the pull path alone. This is an output-surface change, not new domain logic and
not a new port operation.

Fourth of four children of 0171, and now unblocked in fact. 0210 is `done`, so
`work sync` resolves real `jira`/`linear` clients and reaches the run engine —
verified against a source build (the cached launcher is stale). 0213 still needs
no *live* tracker: a complete `Prompt` conflict is driven offline two ways —
over `RecordingTracker` (`sync_run.rs`) and, better, over the real clients
pointed at a `MockServer` (`sync_run_real_client.rs`, which already threads a
`Resolution` map). None of 0171's Open Questions gate it.

## Requirements

### The CLI half

- For every item whose planned action is `Prompt`, `accelerator work sync` must
  materialise a **conflict dossier** carrying six fields: the work-item id, its
  title, the differing field, the local value, the remote value, and the local
  and remote timestamps as a pair. Build it from the run's own data — reuse
  `differing_sections` over the local content and the reconstructed remote, and
  the existing per-section renderer — rather than adding a port operation or a
  second fetch.
- The differing field is a **section name**, which is the granularity the engine
  actually works at: sync compares projected body hashes, and
  `SectionDiff.name` already resolves to `frontmatter`, `(preamble)` or the
  heading text. An item differing in several sections yields several
  `SectionDiff`s under one id.
- Two fields are legitimately unavailable and must render explicitly as absent
  rather than as an empty string: `RemoteTimestamp` has `NotReported` and
  `NotRead` alongside `Reported`, and `ItemDigests::mtime` returns `Ok(None)`
  when no mtime is available.
- Rendering can fail. `diff_shellout::render` spawns real `diff -u` under a
  ten-second cap and returns `DiffUnavailable` when the binary is missing or
  slow. An item whose dossier cannot be rendered must be reported as
  unrenderable and left unresolved — never prompted against blind.
- Decide and state what `--preview` does. A conflict currently writes neither
  side, which is the safety property 0194 defends; dossiers give a
  conflict-only run a side effect. Either exempt `--preview` or site the
  dossiers where a preview may legitimately write, and add the gitignore entry
  alongside the `pending-push/` precedent.
- Leave the four-column report untouched. Its arity is contractual, so the
  dossier is a separate surface, not a fifth column.
- The binary stays non-interactive: it must never read stdin.

### The skill half

- Read the report on exits `0`, `4`, `70` **and** `71` — four codes, not
  three. The report is printed on the `Ok(report)` path and
  `exit_code_for_report`
  yields exactly those four, in the precedence terminal (71) > awaiting-human
  (4) > retryable (70) > clean (0).
- Do **not** branch on `unresolved` lines alone. `awaiting_human` counts
  `Prompt`, `SkipConflict` and `SkipDirty` actions and the `RemoteAbsent` and
  `Indeterminate` states, but only `Prompt` renders the keyword `unresolved`. A
  run can exit `4` carrying no `unresolved` line while genuinely awaiting a
  human.
- Render each conflict from its dossier, then collect a single choice **per work
  item, not per field**, because `--resolve` is keyed by id and carries one
  order per id. Where an item differs in several sections, show every one, then
  ask once.
- Emit exactly one `--resolve <id>=<remote|local|skip>` order per collected
  choice. Naming an id twice is a usage error (exit `2`), and an order naming an
  id the report did not report is silently inert.
- Normalise the typed token in the skill before emitting it. `--resolve` treats
  an unrecognised token as `skip` with only a stderr warning, which would
  silently discard a typo; today's flow re-asks once before falling to skip and
  that behaviour must survive.
- Preserve the existing prompt's shape: a typed token with **no Enter default**,
  distinct from the `[y/N]` polarity of the batch-push and untracked-pull gates,
  and distinct from the `AskUserQuestion` blast-radius gates. A reflexive Enter
  must never discard local edits.
- A clean run carries no `unresolved` lines, so the flow reports no conflicts
  and issues no `--resolve` re-invocation.
- Surface, don't parse, the non-report exits. `72` (recognised, no client),
  `73` (unset or unrecognised) and the new `74` (`UNCONFIGURED` — wired but
  credentials or configuration missing) all return before any report is
  printed; report the binary's stderr message and stop rather than parsing an
  absent report.

## Acceptance Criteria

- [ ] **The dossier, at the `work-adapters` boundary.** Rust tests drive a
      two-conflict corpus and assert each dossier carries all six fields, with
      distinct ids and a multi-section item among them. Prefer
      `cli/work-adapters/tests/sync_run_real_client.rs` — the real
      `Jira`/`Linear` clients pointed at a `MockServer`, which already threads a
      `Resolution` map and exercises the actual projection path the dossier
      renders from; `sync_run.rs`'s `RecordingTracker` scenario (`:451-500`) is
      the lighter alternative. Both are network-free.
- [ ] **The absent and unrenderable paths.** Further tests at the same boundary
      cover a `NotReported` remote stamp, an unavailable local mtime, and a
      `DiffUnavailable` render — asserting the first two render as explicitly
      absent and the third leaves the item unresolved and unprompted.
- [ ] **The report format is frozen.**
      `cli/work-cli/tests/fixtures/sync-report.golden` exists and is asserted
      byte-for-byte. 0194's Phase 4 specified this file and never wrote it, so
      the format the skill parses is currently unasserted.
- [ ] Statically: `sync-work-items/SKILL.md` contains the
      `--resolve <id>=<remote|local|skip>` invocation template, instructs
      reading the report on exits `0`, `4`, `70` and `71`, instructs branching
      on awaiting-human actions and states rather than the `unresolved` keyword
      alone, and names all six render fields. Asserted by an automated check in
      the build system — an invoke lint under `tasks/lint/` reusing
      `tasks/shared/skill_parsing.py`, with a pytest unit test — not by
      inspection, since 0212 edits the same file.
- [ ] **Argv acceptance, against the real binary.** The argv the flow emits is
      replayed against the real `accelerator work sync` and its exit code is not
      `2`, the usage-error code (`exit_codes::USAGE`). Post-0210 a credentialed
      run over a conflict-free corpus returns `0` and a creds-absent run returns
      `74` (`UNCONFIGURED`); accept `0`, `4`, `70`, `71` and `74`, and fail only
      on `2`. A usage error means the invocation template is malformed, the
      likeliest defect in a mostly-prose change. Cover the negative cases too: a
      repeated id and a value with no `=` must both be shown to exit `2`.
- [ ] **The skill flow.** An eval suite at
      `skills/work/sync-work-items/evals/` — the convention eighteen other
      skills already follow — covering one two-conflict case and one clean
      case. Its assertions: both conflicts render with all six fields, exactly
      one
      prompt is issued per conflict, exactly one `--resolve <id>=<choice>` order
      is emitted per choice with ids matching the fixture, and the clean case
      reports no conflicts and issues no re-invocation. A shell harness cannot
      discharge this — a SKILL body is prose interpreted by the model, and no
      script in this repo drives one.
- [ ] The eval evidence is committed under the same `evals/` directory, one
      file per case, in the **reduced, secret-scrubbed** shape 0210 established
      (`cli/tracker-test-support/src/evidence.rs` — a `name PASS|FAIL count Nms`
      grammar whose `is_reduced` guard refuses payloads and token shapes; a
      committed instance lives at `cli/linear-client/tests/evidence/`). A
      verbatim transcript would carry remote issue bodies and could leak a
      token, so it must not be committed raw. 0171's `## Decisions` entry for
      conflict-flow walkthrough evidence points at it; 0171 carries that entry
      twice, so reconcile the duplicate while pointing it.
- [ ] No 0213 test makes a live tracker call in `mise run`. Every real-client
      test is `MockServer`-backed, or filtered from the default nextest profile
      behind `ACCELERATOR_TRACKER_CONTRACT`, per
      `cli/work-cli/tests/no_network_by_default.rs`.
- [ ] `mise run` exits 0, including the new lint, the new Rust tests and the
      report golden. The public API of `work` and `work-adapters` changes, so
      `cargo-public-api` and `cargo-pup` apply — see `tasks/README.md`.

## Dependencies

- Depends on 0194, which is `done` and ships `accelerator work sync`, the
  four-column report and the `--resolve` flag. Verified against a binary built
  from source — the cached `1.24.0-pre.43` launcher is stale and predates the
  0210 rebase: `--resolve` is repeatable, takes `<id>=<remote|local|skip>`, and
  rejects a malformed flag with exit `2`.
- **0210 is `done`**, so `work sync` now resolves real `jira`/`linear` clients
  and reaches the engine, with a new exit `74` (`UNCONFIGURED`) for
  wired-but-no-credentials. This unblocks 0213 rather than gating it: the CLI
  half and its tests run offline over `MockServer` / `RecordingTracker`, and no
  live tracker call may enter `mise run`.
- **Not gated by any of 0171's Open Questions**, nor by a credentialed target.
- Touches `sync-work-items/SKILL.md`, which 0212 also repoints — and touches
  the *same section*, not a disjoint part of the body: 0212 deletes the bash
  cluster
  that currently feeds the conflict render. Landing them together avoids writing
  that section twice. Whichever lands second rebases onto the other.
- Parent: 0171.

## Assumptions

- The dossier is the right surface. The alternative — sync writing only the
  reconstructed remote and the skill calling `accelerator work diff` against it
  — reuses more existing code, since that subcommand's help already reads "for
  conflict-resolution review", but leaves the skill assembling six fields from
  three sources. ADR-0045 puts deterministic extraction and rendering in the
  CLI and leaves the skill the prompt and the choice, which is why the dossier
  is preferred. Settle this in planning; it changes the CLI surface, not the
  size.
- Section granularity is sufficient for "the differing field". Sync's conflict
  verdict is a whole-document hash comparison, so no finer granularity exists to
  expose.

## References

- Research, in `meta/research/codebase/`:
  `2026-08-18-0213-conversational-conflict-resolution-flow.md` — the
  field-by-field audit of the shipped report, the exit-code and `--resolve`
  semantics verified against the binary, and the CLI-extension options. Its
  2026-08-19 follow-up records the 0210 rebase: clients wired, exit `74`, the
  `MockServer` test seam and the `evidence.rs` convention.
- Parent: `meta/work/0171-jira-and-linear-integrations.md`
- Related: 0194 (`done`), 0210 (`done` — provider clients over the tracker
  port), 0212 (shares the same section of `sync-work-items/SKILL.md`)
- `meta/decisions/ADR-0045-skills-vs-cli-division-of-labour.md` — skills own
  probabilistic work, the CLI owns deterministic work.
- `meta/plans/2026-08-13-0194-tracker-crate-and-remote-sync-engine.md` — the
  report contract as designed, and the deferral of the conversational flow.
