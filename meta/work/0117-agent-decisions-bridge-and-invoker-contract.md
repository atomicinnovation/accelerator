---
type: work-item
id: "0117"
title: "Agent-Decisions Bridge and Documented Invoker Contract"
date: "2026-06-19T23:13:17+00:00"
author: Toby Clemson
producer: refine-work-item
status: done
kind: task
priority: high
parent: "work-item:0115"
relates_to: ["work-item:0069", "work-item:0092", "work-item:0116", "work-item:0118", "work-item:0119"]
tags: [migrate, interactive-migration, agent-invocation, tooling]
last_updated: "2026-06-22T21:46:54+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-139
---

# 0117: Agent-Decisions Bridge and Documented Invoker Contract

**Kind**: Task
**Status**: Done
**Priority**: High
**Author**: Toby Clemson

## Summary

Give an agent a supported way to answer interactive migration prompts: a
`--list` mode (a dry emit that surfaces every pending interactive transformation
before any prompt blocks, without mutating the corpus), a
promoted-and-documented `ACCELERATOR_MIGRATE_DECISIONS_FILE`, and an
invoker-side contract written into `SKILL.md`. This is fix A of 0115 — the full
bridge that makes interactive migrations completable under agent invocation.
0115 elected to pursue fix A alongside the C/D mitigations (0118 and siblings)
even though the source research ranked A as a follow-up rather than the first
move; see Drafting Notes for the rationale.

## Context

Child of 0115 — Make Interactive Migrations Satisfiable Under Agent Invocation.

The concrete interactive migration driving this is
`0007-unify-meta-corpus-frontmatter`, whose `harness_run` stage emits
per-transformation `PROMPT` frames for ambiguous-band body-section typed
linkage. `--list` and the decisions bridge are framework-level driver surfaces
(not specific to 0007), but 0007 is the corpus migration that exercises them.

The driver currently exposes only `--skip`/`--unskip`; there is no way to
enumerate pending interactive transformations without running the migration and
hitting a blocking prompt. `ACCELERATOR_MIGRATE_DECISIONS_FILE` already exists
but is a deliberately hidden test-only seam (documented only in an inline source
comment as "never documented in --help or any user-facing banner"). `SKILL.md`
documents the *author* side of the interactive contract extensively but says
nothing about the *invoker* side.

## Requirements

- Add a `--list` (dry-emit) mode to the migration driver that surfaces every
  pending interactive transformation — key, proposed value, and context —
  up front, before any prompt blocks, without mutating the corpus.
- Promote `ACCELERATOR_MIGRATE_DECISIONS_FILE` from a hidden test-only seam to a
  documented, user-facing interface (including its format: newline-delimited,
  positional `accept` / `skip` / `edit <value>` verbs matched by emission
  order). The driver validates the decisions file against the pending
  transformations and **fails closed** on malformed input — a count mismatch
  (too few or too many verbs) or an unknown verb — exiting non-zero with a
  message naming the offending position and leaving the corpus unmutated, rather
  than applying decisions partially.
- Document the invoker contract in `SKILL.md`: how an agent answers prompts
  (list → decide → write decisions file → resume), and what happens when it
  cannot (links to the structured stall from 0116).

Each behavioural criterion runs against a **standalone reference fixture**: a
self-contained test corpus (in `test-migrate-interactive.sh`, or alongside it)
seeded with **exactly three** pending interactive transformations (the minimum
that exercises one of each verb), in a fixed emission order.

Field definitions for a transformation:

- `<key>` — the frontmatter linkage key the harness proposes to write for that
  transformation (e.g. `relates_to`).
- `<proposed-value>` — the value the harness proposes for that key.
- `<source-file-path>:<band/field>` — the **context**: the corpus file and the
  band/field locating the decision within it.

The fixture pins concrete literals so tests assert exact strings:

| Pos | Decision verb        | `<key>`      | `<proposed-value>` | context (`<path>:<band/field>`)              | Expected post-migration field           |
|-----|----------------------|--------------|--------------------|----------------------------------------------|------------------------------------------|
| 1   | `accept`             | `relates_to` | `work-item:0042`   | `meta/work/0050-example-a.md:body/relates_to`| `relates_to: [work-item:0042]` written   |
| 2   | `skip`               | `parent`     | `work-item:0031`   | `meta/work/0051-example-b.md:body/parent`    | `parent` left unchanged/absent           |
| 3   | `edit work-item:0100`| `relates_to` | `work-item:0099`   | `meta/work/0052-example-c.md:body/relates_to`| `relates_to: [work-item:0100]` (edit wins)|

This fixture is deliberately independent of the live `0007` corpus, so the
bridge's behaviour is fully verifiable without 0118 having landed — it does not
depend on `0007` reaching its interactive stage (which is gated on fix C /
0118).

Each `--list` entry is emitted as a single tab-delimited line in the canonical
form `<position>\t<key>\t<proposed-value>\t<source-file-path>:<band/field>`, one
entry per line. The exact expected `--list` output for the fixture is therefore:

```
1	relates_to	work-item:0042	meta/work/0050-example-a.md:body/relates_to
2	parent	work-item:0031	meta/work/0051-example-b.md:body/parent
3	relates_to	work-item:0099	meta/work/0052-example-c.md:body/relates_to
```

"Emission order" is the order in which the harness emits `PROMPT` frames for the
fixture (positions 1..3 above); `--list` prints in that same order, and
decisions-file position *i* maps to `--list` entry *i*. ("Consumption order" and
"emission order" denote this same single ordering.)

- [x] **AC1** — Given the standalone reference fixture (3 pending
      transformations), when the driver is run with `--list`, then it prints
      exactly the three canonical lines shown in the fixture's expected `--list`
      output block above (byte-for-byte, tab-delimited, in emission order
      1..3), and exits 0 without mutating the corpus.
- [x] **AC2** — Given the standalone reference fixture and a decisions file
      holding `accept` (position 1), `skip` (position 2), and
      `edit work-item:0100` (position 3), when the driver is resumed against that
      decisions file, then the migration completes and the corpus reflects the
      pinned per-position outcomes: `0050-example-a.md` gains
      `relates_to: [work-item:0042]`, `0051-example-b.md`'s `parent` is left
      unchanged/absent, and `0052-example-c.md` gains
      `relates_to: [work-item:0100]` (the edit value, not the proposed
      `work-item:0099`). (Binding form is deterministic — a fixed decisions file,
      not a live agentic decision; verifiable without 0118 having landed.)
- [x] **AC3** — Given a corpus with no pending interactive transformations, when
      the driver is run with `--list`, then it prints exactly the single line
      `no pending transformations` and nothing else, and exits 0 without mutating
      the corpus.
- [x] **AC4** — Given the migration driver, when its `--help` output is
      inspected, then it contains the literal string
      `ACCELERATOR_MIGRATE_DECISIONS_FILE` together with a one-line description
      of its format — i.e. a `grep` for the variable name in `--help` succeeds
      (it is no longer described only as a hidden test-only seam).
- [x] **AC5** — Given the invoker-contract section of `SKILL.md`, then it
      contains all of: (a) the literal phrase `list → decide → write → resume`
      (or the four steps named in order); (b) the verb tokens `accept`, `skip`,
      and `edit` and the phrase "matched by emission order"; (c) a link to 0116
      for the no-input/structured-stall outcome; (d) the literal string
      `ACCELERATOR_MIGRATE_DECISIONS_FILE` with a pointer to where it is
      discoverable (`--help`); and (e) the fail-closed behaviour on a malformed
      decisions file (count mismatch / unknown verb → non-zero exit, corpus
      unmutated). Each element is confirmable by a string search.
- [x] **AC6** — Given the standalone reference fixture (3 pending
      transformations) and a **malformed** decisions file, when the driver is
      resumed against it, then the driver fails closed without mutating the
      corpus, exiting non-zero with a message naming the offending position, for
      each of:
      (a) **count mismatch — too few**: a decisions file with two verbs for the
      three pending transformations → error names the first unmatched position
      (3);
      (b) **count mismatch — too many**: a decisions file with four verbs →
      error names the surplus position (4);
      (c) **unknown verb**: a decisions file whose position-2 line is neither
      `accept`/`skip`/`edit <value>` → error names position 2.
      In all three cases the corpus is left unmutated (no partial application).
- [ ] **AC7** (Integration — **out of scope for 0117; owned by 0118 / the parent
      epic**) — Given the live `0007` interactive stage, when the driver is
      resumed against a decisions file for the real corpus, then the migration
      completes with the decisions applied. This confirms the bridge end-to-end
      against the live migration, but is gated on fix C (0118) letting `0007`
      reach its interactive stage; it is **not** part of 0117's definition of
      done and is tracked under 0118 / the parent epic's integration check.

## Open Questions

- ~~Should this change to the invoker side of the interactive contract be recorded
  as an amendment to ADR-0037 (work item 0092), or treated as an implementation
  detail under the existing decision? (Inherited from 0115.)~~ **Resolved:
  implementation detail, not an amendment.** The invoker bridge (`--list`,
  decisions-file validation, the documented `list → decide → write → resume`
  contract) adds no new control verb, display element, or resumability guarantee,
  and ADR-0037 is neutral on how the runner is invoked — so the recursive-supplement
  clause is not tripped. The immutable, accepted ADR-0037 is left unedited.

## Dependencies

- Blocked by: none as a hard build-time blocker — the `--list` and
  decisions-bridge code compiles and ships independently. See the soft ordering
  and functional-precondition notes below (0116, 0118, 0119).
- Blocks: none.
- Soft ordering / content dependency on 0116: AC for the `SKILL.md` contract
  documents the no-input outcome as 0116's structured stall, so 0116 should land
  before — or alongside — this work for the documented stall to match reality.
  (0116 records the reciprocal constraint.) Not a hard build-time blocker: the
  `--list` and decisions-bridge code stands alone.
- Functional precondition on 0118 (integration only): all of 0117's
  definition-of-done criteria (AC1–AC6, including the positional list→verb flow
  and the malformed-file fail-closed behaviour) are verified against the
  standalone reference fixture and do not depend on 0118. The live-corpus
  integration check (AC7) runs against the live 0007
  interactive stage — which 0007 reaches only once fix C (0118) lets it past its
  structural self-validation — and is therefore **out of scope for 0117**, owned
  and tracked by 0118 / the parent epic's integration check. The bridge code and
  all in-item verification stand alone.
- Functional precondition on 0119 (resume-on-a-dirty-tree only): the documented
  `list -> decide -> write -> resume` flow's final `--decisions-file` resume hits
  the unconditional dirty-tree pre-flight, and a partial interactive run is
  exactly what dirties the tree. Relaxing that guard for scripted resumes belongs
  to 0119 (resume-safe partial migration failure) — the concern the stall message
  already defers to it. AC1–AC6 were unaffected (read-only `--list`, or
  `ACCELERATOR_MIGRATE_FORCE=1`), so this did not block 0117's definition of
  done. **0119 has since landed**, so the shipped SKILL.md invoker contract
  documents its guarded resume: a `--decisions-file` resume over the run's own
  partial output proceeds **without** `FORCE` when the base revision is
  unchanged, and `FORCE` is required only for un-owned dirt.
- Merge-ordering coordination with 0116: both touch `interactive-lib.sh` in the
  same `PROMPT`/`read_decision` region, so if this is scheduled first, coordinate
  merge order to avoid conflicts and preserve 0116's standalone-mitigation value.
- Relates to: 0115 (parent), 0069 (runner-side interactive/resume machinery),
  0092 / ADR-0037 (the interactive contract this work touches on the invoker
  side — see Open Questions on whether it is recorded as an amendment),
  0116 (the stall the contract documents as the no-input outcome),
  0118 (fix C, the functional precondition for AC2 above),
  0119 (resume-safe partial migration failure; owns the dirty-tree bypass for
  `--decisions-file` resume — functional precondition for resume-on-a-dirty-tree).

## Assumptions

- The decisions file remains positional (no keys); `--list` output is ordered
  to match the consumption order so an agent can map list entries to verbs.

## Technical Notes

**Size**: M — net-new `--list` driver surface, promotion of
`ACCELERATOR_MIGRATE_DECISIONS_FILE` to a user-facing interface, and a new
SKILL.md invoker-contract section, spanning `run-migrations.sh`,
`interactive-lib.sh`, and `SKILL.md`.

- Driver flags today are only `--skip`/`--unskip` — `run-migrations.sh:43-65`;
  `--list` is genuinely net-new surface.
- `ACCELERATOR_MIGRATE_DECISIONS_FILE` resolution and validation —
  `run-migrations.sh:14-37`; opened on literal fd 9 in
  `run_interactive_migration` (`interactive-lib.sh:308-316`).
- Decisions-file format is positional verbs (`accept` / `skip` /
  `edit <value>`), CRLF-tolerant, blank-lines skipped — `read_decision()`
  parsing at `interactive-lib.sh:266-287`.
- Invoker contract belongs alongside the existing author-facing "Optional
  interactive contract" section — `SKILL.md:89-214`; execution guidance is at
  `:216-224`.

## Drafting Notes

- Decomposed from 0115 fix A. The medium-effort task in the bundle; net-new
  driver surface plus user-facing documentation.
- The source research ranked fix A ("a full agent↔decisions bridge") as "better
  as a follow-up than the first move", recommending C + D + B as the durable
  fix because A still asks the agent to make editorial linkage judgments. 0115
  nonetheless elected to pursue A *in addition to* the C/D mitigations (this
  item is A; 0118 is fix C; 0116 is fix B): the bridge is the only option that
  makes interactive migrations *completable* under agent invocation, whereas
  B/C/D each only make the failure graceful or avoid reaching it. The open
  question about whether body-section linkage should run under agent invocation
  at all is tracked below and informs how far this bridge is exercised in
  practice.

## References

- Source: `meta/research/issues/2026-06-19-interactive-migration-unsatisfiable-under-agent-invocation.md`
- Related: 0115, 0069, 0092
