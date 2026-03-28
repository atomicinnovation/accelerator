---
title: "Plugin organisation: two-commit strategy and four skill groups"
type: adr-creation-task
status: todo
---

# ADR Ticket: Plugin organisation: two-commit strategy and four skill groups

## Summary

In the context of migrating 18 skills from a flat layout, we decided for a
two-phase approach (flat extraction then restructure into `git/`, `planning/`,
`review/`, `research/` groups with review subdivided into `lenses/` and
`output-formats/`) to achieve independent verifiability and intuitive
navigation, accepting deeper nesting in cross-reference paths.

## Context and Forces

- 18 skills in a flat directory made discoverability poor
- Skills naturally cluster by domain: VCS operations, planning workflows,
  review (lenses + output formats), and research
- Claude Code's plugin system uses recursive SKILL.md discovery, so any
  directory structure works
- Restructuring and extraction are distinct concerns that could introduce
  different bugs
- Conflating extraction bugs with restructuring bugs makes debugging harder
- Path references in cross-skill references need to be updated when moving files

## Decision Drivers

- Discoverability: users and contributors should find skills intuitively
- Independent verifiability: extraction and restructuring should be separately
  testable
- Clear domain boundaries for where new skills should be placed
- Alignment with the plugin's conceptual architecture

## Considered Options

1. **Flat extraction** — Extract all skills into a single directory. Simple but
   poor discoverability.
2. **Grouped extraction in one step** — Extract and restructure simultaneously.
   Harder to debug if something breaks.
3. **Two-phase: flat first, then restructure** — Phase 1-5: extract flat with
   verified commit. Phase 6-7: restructure into groups with separate commit.

For grouping:
1. **Alphabetical** — Simple but meaningless grouping
2. **Four logical categories** — `git/` (VCS), `planning/` (plans/validation),
   `review/` (lenses, output formats, orchestration), `research/` (codebase
   research)

## Decision

We will use a two-commit approach: first extract flat with verification, then
restructure into four groups (`git/`, `planning/`, `review/`, `research/`)
with review further subdivided into `lenses/` and `output-formats/`. Each
commit is independently verifiable. The four groups establish the taxonomy for
where new skills should be placed.

## Consequences

### Positive
- Each phase is independently verifiable and debuggable
- Intuitive navigation through domain-aligned grouping
- Clear taxonomy for placing new skills
- Recursive SKILL.md discovery means grouping just works

### Negative
- Path references must be updated twice (after extraction, after restructuring)
- Deeper nesting means longer cross-reference paths
- Two commits for what could be a single change

### Neutral
- The principle that structural changes should be independently verifiable is
  reusable for future reorganizations

## Source References

- `meta/plans/2026-03-14-plugin-extraction.md` — Two-phase extraction plan and
  four-group taxonomy
