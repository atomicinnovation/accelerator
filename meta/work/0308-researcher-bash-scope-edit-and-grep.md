---
type: "work-item"
id: "0308"
title: "Tell Researchers What Bash Can Do and Give Them Edit and Grep"
date: "2026-10-09T17:58:47+00:00"
author: "Toby Clemson"
producer: "extract-work-items"
status: "draft"
kind: "story"
priority: "low"
parent: "work-item:0121"
relates_to: ["work-item:0283", "work-item:0307"]
tags: ["research", "deep-research", "researcher-agent", "research-guard"]
last_updated: "2026-10-09T17:58:47+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# 0308: Tell Researchers What Bash Can Do and Give Them Edit and Grep

**Kind**: Story
**Status**: Draft
**Priority**: Low
**Author**: Toby Clemson

## Summary

The researcher agent has Bash, but the research guard lets it run only
`accelerator research fetch`. Researchers repeatedly try Bash for local
checks, downloads, greps and edits, get blocked, and fall back to whole-file
rewrites. State the restriction plainly, give researchers `Edit` and `Grep`
for the jobs they reach for Bash to do, and tighten the guard so writes and
edits are confined to paths in the active run's offered batch.

## Context

The researcher's tools are `WebSearch`, `WebFetch`, `Write`, `Read` and
`Bash`, and nothing in its definition says Bash is restricted. In a depth-3
`conduct` run, many researchers reported blocked attempts:

- checking follow-up length and the single-question rule;
- editing their own note;
- downloading the PostgreSQL source;
- grepping oversized fetch results Claude Code had saved to disk.

Each blocked attempt wastes a turn; whole-file rewrites for one-line fixes
cost tokens and risk dropping content.

The guard already matches `Write|Edit|MultiEdit|NotebookEdit` and treats an
edit as a write. Today it permits any topic file of a kind the role may
write (findings and level notes for a researcher; findings for a composer).
It identifies an agent only by `agent_type`, so it cannot tie a specific
spawn to its injected output path.

The level-note outputter asks for follow-ups of "at most 300 characters",
one question each, no slash-joined pairs. Researchers tried to verify these
with Bash, though `outstanding` already trims and rejects at re-plan.

## Requirements

- **Bash stated**: the researcher agent definition states that Bash runs
  only `accelerator research fetch`; local checks, downloads and shell edits
  are unavailable. It names `Edit` for small fixes to its own output and
  `Grep` for searching saved fetch results.
- **Edit**: `Edit` is added to the researcher's tools.
- **Grep**: `Grep` is added to the researcher's tools. It is read-only and
  needs no guard change.
- **Offered-batch write scope**: the guard permits a confined agent
  (researcher or composer) to write or edit a topic file only when its path
  is in the active run's current offered batch, in addition to the existing
  per-role kind rule. Other topic paths — other runs', earlier batches' —
  are refused.
- **Follow-up checks**: the level-note outputter states that `outstanding`
  enforces follow-up length and single-question limits at re-plan, and that
  the researcher should follow them but not attempt mechanical checks.

## Acceptance Criteria

- [ ] The researcher agent definition states that Bash permits only
      `accelerator research fetch`, and names `Edit` and `Grep` as the
      alternatives.
- [ ] `agents/researcher.md` lists `Edit` and `Grep` in `tools`.
- [ ] Given a researcher whose output path is in the active run's offered
      batch, when it calls `Edit` on that path, then the guard allows it.
- [ ] Given a researcher, when it calls `Write` or `Edit` on a level-note or
      finding path not in the active run's offered batch, then the guard
      blocks it with an out-of-scope refusal.
- [ ] Given a composer, when it writes a finding path in the offered batch,
      then the guard allows it; a finding outside the batch is blocked.
- [ ] Given no active run ledger for the topic, when a confined agent writes
      a topic file, then the guard blocks it.
- [ ] The level-note outputter states that `outstanding` enforces the
      follow-up limits at re-plan.

## Open Questions

- Can the guard locate the topic's run ledger from the write target alone,
  or does it need another way to find the active run?
- Should agents of a superseded run that are still in flight be refused as
  soon as a new run starts, or allowed to finish their batch?

## Dependencies

- Blocked by: none known
- Blocks: none known
- Related: 0307, which adds a second command the guard must permit; both
  change `confinement.rs`.

## Assumptions

- Stating the restriction up front and providing `Edit` and `Grep` removes
  most blocked Bash attempts. Not measured.

## Technical Notes

- Researcher tools: `agents/researcher.md:7`.
- Guard registration: `hooks/hooks.json:51,56-60`.
- Guard decision and role rules: `cli/research/src/confinement.rs:60-73`
  (`may_write`) and `:278-306` (`decide`); agent identification at
  `:164-173`.
- Run ledger: `cli/research/src/conduct/ledger.rs`.
- Follow-up limit: `skills/research/outputters/level-note-outputter/SKILL.md:40`.

## Drafting Notes

- Single-output-path scope was the first preference, but the guard cannot
  distinguish spawns; the user accepted offered-batch scope. A researcher
  can still write a sibling's path in the same batch.
- `Grep` rather than documenting `Read` paging was a user decision.
- Offered-batch scope also applies to composers, since the same check fits
  and composers only write offered findings. Drop that criterion if
  composers should keep kind-scope.
- Low priority: blocked attempts waste turns but do not corrupt output.

## References

- Source: https://github.com/atomicinnovation/accelerator/issues/145
- Related: 0121, 0283, 0307
