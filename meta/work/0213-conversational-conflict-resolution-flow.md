---
type: work-item
id: "0213"
title: "Conversational Conflict Resolution Flow for Sync"
date: "2026-08-17T11:17:18+00:00"
author: Toby Clemson
producer: review-work-item
status: ready
kind: task
priority: high
parent: "work-item:0171"
relates_to: ["work-item:0194", "work-item:0212"]
tags: [skills, sync, work-items, conflicts]
last_updated: "2026-08-17T11:17:18+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0213: Conversational Conflict Resolution Flow for Sync

**Kind**: Task
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Add the conversational half of 0194's two-invocation conflict flow to
`skills/work/sync-work-items/SKILL.md`: invoke `accelerator work sync`, parse its
machine-parseable conflict report, render each conflict with enough context for
the user to judge, collect a choice per item, and re-invoke with matching
`--resolve` orders.

## Context

`/sync-work-items` can detect a conflict today but cannot resolve one. 0194's
binary is non-interactive by requirement, so it reports conflicts and exits; until
something closes the report → prompt → resolve loop, a user who hits a conflict
has no way through it. This is live, user-visible degradation.

Fourth of four children of 0171 and the only one gated by none of its
prerequisites — no credentialed tracker target, no client crate, none of the three
Open Questions. It can therefore land first, ahead of 0210, which is why it
carries a higher priority than its siblings.

## Requirements

- Invoke `accelerator work sync`, parse its machine-parseable conflict report,
  render each conflict, collect a choice per item, and re-invoke with the
  matching `--resolve <id>=<remote|local|skip>` orders — three permitted
  resolutions (`remote`, `local`, `skip`), not two.
- Read the report on exits `0`, `4` and `71` — the only codes it can accompany —
  and branch on whether it carries `unresolved` lines rather than on the code
  alone. `4` means items await a human; a `71` run may carry conflicts alongside
  its failure; `0` is a clean run, so its report carries **no** `unresolved`
  lines and the flow must report no conflicts and issue no `--resolve`
  re-invocation. Parsing `0` defensively is what stops a clean run from prompting
  against an empty list.
- Render each conflict with at least these six fields: the work-item id, its
  title, the differing field, the local value, the remote value, and the local and
  remote timestamps as a pair. More is allowed; fewer is not.
- Resolve **per work item, not per field**, because `--resolve` is keyed by id and
  can carry only one choice per id. Where one item conflicts on several fields,
  render every differing field for that item, then collect a single choice
  covering all of them. Prompting per field would collect choices that cannot all
  be expressed, silently dropping one — confirm against the report format whether
  it can emit multiple differing fields for a single id, and if it can, add a
  fixture exercising that case.
- The binary is non-interactive by 0194's requirement, so the SKILL body must
  never expect it to read stdin.
- Deliver the walkthrough as a **committed harness** under
  `skills/work/sync-work-items/test-fixtures/`, not as an undocumented manual
  procedure: a script that puts the stub `accelerator` on `PATH`, drives the flow
  against one fixture, captures the transcript, then replays the emitted argv
  against the real binary with the stub removed. Whoever runs it produces the
  evidence artefact by running one command, so the check is repeatable rather
  than a one-off inspection.

## Acceptance Criteria

- [ ] Statically: `sync-work-items/SKILL.md` contains the
      `--resolve <id>=<remote|local|skip>` invocation template, instructs reading
      the report on exits `0`, `4` and `71`, and names all six render fields.
      Asserted by an automated test in the existing skills test lane, not by
      inspection — 0212 edits the same file, so this is the guard that survives.
- [ ] Three committed fixture reports exist at
      `skills/work/sync-work-items/test-fixtures/`: `conflicts-exit-4.txt` and
      `conflicts-exit-71.txt`, each carrying two conflict records populated in all
      six fields with distinct work-item ids, and `clean-exit-0.txt` carrying no
      `unresolved` lines.
- [ ] **Behaviour, against the stub.** The walkthrough is run once per fixture,
      driven through a stub `accelerator` on `PATH` that emits the fixture and
      exits with its code, so no credentialed target or live tracker is involved.
      Its pass predicate is a checklist: both conflicts render with all six
      fields; exactly one prompt is issued per conflict; and exactly one
      `--resolve <id>=<choice>` order is emitted per collected choice, with ids
      matching the fixture's. For `clean-exit-0.txt`, the predicate is that no
      conflict is reported and no re-invocation is issued.
- [ ] **Argv acceptance, against the real binary.** The argv the flow emitted in
      each conflict-bearing run is replayed against the real `accelerator work
      sync` with the stub **off** `PATH`, and its exit code is not the
      usage-error code. A configuration or tracker failure is a pass — it proves
      the parser accepted the flags — while a usage error means the invocation
      template is malformed, which is the likeliest defect in a `SKILL.md`-only
      change and the one the stub cannot catch. Name the usage-error code in the
      test so the assertion is not merely "some non-zero exit".
- [ ] Both halves' evidence — the per-fixture transcripts (rendered conflicts,
      prompts issued, emitted argv) and the replay exit codes — is committed at
      `skills/work/sync-work-items/test-fixtures/walkthrough-evidence/`, one file
      per fixture, with 0171's `## Decisions` entry pointing at the directory.
- [ ] `mise run` exits 0, including the new skills-lane assertion above.

## Dependencies

- Depends only on 0194, which already ships `accelerator work sync`, its
  machine-parseable conflict report and the `--resolve` flag. 0194's record as
  visible from this workspace reads `status: ready`, so confirm the artefacts
  themselves rather than the status field: `accelerator work sync --help` lists
  `--resolve`, and a conflicting pair produces a report carrying `unresolved`
  lines. If either check fails, add `blocked_by: ["work-item:0194"]` to this child
  before planning — its entire deliverable rests on that one upstream.
- **Gated by none of 0171's other prerequisites**: it needs no credentialed
  tracker target, no client crate, and none of the three Open Questions. It can
  land before its siblings, and should — Context in 0171 records that
  `/sync-work-items` can detect a conflict but cannot resolve one today.
- Touches `sync-work-items/SKILL.md`, which 0212 also repoints. Whichever lands
  second rebases onto the other; the edits are in different parts of the body
  (0212 changes the invocations, this child adds the conflict loop).
- Parent: 0171.

## Assumptions

- ⚠️ **Unconfirmed, and it bounds this child's size**: 0194's shipped conflict
  report already carries all six render fields — work-item id, title, differing
  field, local value, remote value, and both timestamps — on every exit code that
  can carry a report. Confirm against the report format before planning. If a
  field is absent (the title and either timestamp are the plausible candidates),
  this stops being a `SKILL.md`-only change and grows into 0194's binary,
  invalidating both its stated independence and its ability to land first.
- The stub-on-`PATH` seam is sufficient to drive the walkthrough deterministically
  without a live tracker, so the flow can be verified while 0210 is still
  outstanding.

## References

- Parent: `meta/work/0171-jira-and-linear-integrations.md`
- Related: 0194, 0212 (shares `sync-work-items/SKILL.md`)
- Corrected against `cli/work-cli/src/cli.rs` for the `--resolve` token set and
  the exit codes carrying a conflict report.
