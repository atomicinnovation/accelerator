---
type: work-item
id: "0123"
title: "User-Facing CHANGELOG and README Update for 1.23.0"
date: "2026-06-19T21:20:28+00:00"
author: Toby Clemson
producer: create-work-item
status: done
kind: task
priority: high
tags: [documentation, release, changelog, readme]
last_updated: "2026-06-19T21:20:28+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: PP-144
---

# 0123: User-Facing CHANGELOG and README Update for 1.23.0

**Kind**: Task
**Status**: Done
**Priority**: High
**Author**: Toby Clemson

## Summary

Curate the user-facing CHANGELOG and README updates for the 1.23.0 release.
Derive the change set from jj history between the 1.22.0 release point and
current `main`, record consumer-relevant changes under the CHANGELOG's
`[Unreleased]` section (Keep a Changelog conventions), and refresh the README
feature/skill catalogue to reflect skills and capabilities added or materially
changed this cycle. This gates the 1.23.0 release.

## Context

1.23.0 is in pre-release (`1.23.0-pre.1`). Before it ships, the user-facing
documentation must reflect what changed since 1.22.0 (released 2026-06-17).

The CHANGELOG follows Keep a Changelog: a `## [Unreleased]` section sits above
the dated, released versions, with changes grouped into `Added` / `Changed` /
`Fixed` / `Removed` subsections. The 1.22.0 entry also carries an upgrade
callout blockquote because it introduced migration 0007.

This cycle introduces **no new migration**, but it does include **fixes to the
existing 0007 migration** — significant enough that upgraders should be alerted
via a callout in the changelog.

A prior precedent exists for this work: the 1.22.0 refresh was researched in
`meta/research/codebase/2026-06-17-readme-changelog-1.22.0-refresh.md`.

## Requirements

- Derive the change set from jj history between the 1.22.0 release point and the
  current `main` head.
- Record user-facing changes under the CHANGELOG's existing `[Unreleased]`
  section, grouped into `Added` / `Changed` / `Fixed` / `Removed` as applicable,
  written for plugin *consumers*. **Do not** promote to a `## [1.23.0]` heading
  or add a release date — the release process handles version heading and date.
- Exclude developer-of-the-plugin changes (CI wiring, build-system, test
  infrastructure, internal refactors, lint/tooling) from the CHANGELOG.
- Include an upgrade callout noting the fixes to migration 0007, so upgraders
  understand the migration behaviour changed even though no new migration was
  added.
- Update the README feature/skill catalogue to reflect skills and capabilities
  added or materially changed this cycle.

## Acceptance Criteria

- [ ] Given the jj history from the 1.22.0 release point to `main`, when the
      CHANGELOG is updated, then every user-facing change in that range is
      represented under `[Unreleased]` and no developer-internal change appears.
- [ ] Given the CHANGELOG edits, when the file is inspected, then entries remain
      under `## [Unreleased]` with no `## [1.23.0]` heading and no release date
      added by this work.
- [ ] Given the fixes to migration 0007, when the `[Unreleased]` section is
      written, then an upgrade callout describes the 0007 fixes for upgraders.
- [ ] Given a new or materially-changed user-facing skill/capability this cycle,
      when the README is reviewed, then its feature/skill catalogue reflects it.
- [ ] Given the `[Unreleased]` entries, when compared to the 1.22.0 entry, then
      they follow the same Keep a Changelog grouping conventions.
- [ ] Given the edits are complete, then the relevant doc/lint checks pass and no
      version-coherence files (`plugin.json`, `Cargo.toml`, `checksums.json`)
      were modified.

## Open Questions

- None outstanding.

## Dependencies

- Blocked by: none
- Blocks: the 1.23.0 release (this work gates it)

## Assumptions

- "Between 1.22.0 and 1.23.0" means the jj revisions from the 1.22.0 release
  commit up to the current `main` head.
- "User-facing" means changes a plugin *consumer* experiences (skills, agents,
  hooks, templates, visualiser behaviour); CI / build-system / test / refactor /
  tooling changes are excluded.

## Technical Notes

- The CHANGELOG `[Unreleased]` section is the staging ground the release process
  later stamps with a version and date — leave that promotion to the release
  process.
- Model the upgrade callout on the 1.22.0 entry's blockquote style.

## Drafting Notes

- README "feature/skill list" interpreted as the skill-catalogue sections spread
  across the README (Work Item Management, Remote Work Item Management,
  Configuration, …), not a single list — new/changed skills get entries in their
  relevant section.
- Scoped strictly to documentation edits: version coherence bump and the actual
  release/tag/publish action are explicitly out of scope.

## References

- Research: `meta/research/codebase/2026-06-17-readme-changelog-1.22.0-refresh.md`
- Related: 0113 (references the 1.22.0 refresh research)
