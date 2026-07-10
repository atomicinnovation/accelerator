---
type: work-item
id: "0177"
title: "Stand up a documentation site for the docs/ tree"
date: "2026-06-29T10:28:21+00:00"
author: Phil Helm
producer: refine-work-item
status: draft
kind: story
priority: medium
parent: "work-item:0145"
tags: []
last_updated: "2026-06-29T14:46:59+00:00"
last_updated_by: Phil Helm
schema_version: 1
external_id: PP-699
---

# 0177: Stand up a documentation site for the docs/ tree

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Phil Helm

## Summary

Select a documentation-site generator and wire publishing so the `docs/` tree
produced by 0175 and 0176 is built and served as a browsable site.

## Context

Child of 0145 — Documentation Improvements, satisfying the epic's second
requirement ("Build a documentation site"). No docs-site tooling exists today:
there is no mkdocs, Docusaurus, VitePress, or mdBook config, and no
docs-publishing job in `.github/workflows/main.yml`. This story adds the build
+ publish pipeline and resolves the epic's open tooling question. It serves
readers of the documentation — primarily new users and contributors browsing
the published `docs/` tree rather than reading raw Markdown in the repo.

The tooling-selection, hosting, and publishing-trigger decisions are
deliberately left open here: they are research-shaped and will be scoped when
the work item is picked up (see Open Questions). At that point the selection may
be recorded as an Architecture Decision Record (ADR) or split into a dedicated
spike, and the story re-kinded if the investigation needs a time-box.

## Requirements

- Choose a documentation-site generator appropriate for a markdown `docs/`
  tree (candidates: mkdocs / mkdocs-material, Docusaurus, VitePress, mdBook).
- Add the generator's configuration and any required dependencies.
- Wire a publishing pipeline (e.g. a GitHub Actions job publishing to GitHub
  Pages) that builds the `docs/` tree on the trigger chosen during pickup-time
  scoping (see Open Questions).
- Ensure the site's navigation reflects the `docs/` tree structure (including
  the `docs/skills/` reference layer).

## Acceptance Criteria

- [ ] A documentation-site generator is selected and its choice is recorded in
      an Architecture Decision Record (ADR) or work-item note, with rationale
      that weighs the named candidates against the project's constraints (the
      `mise` Rust/Python/TS/shell toolchain).
- [ ] The generator's config exists in the repo and builds the `docs/` tree
      locally with no errors and no broken-link or missing-page warnings.
- [ ] A CI job publishes the built site on the trigger chosen during pickup-time
      scoping, and the published site is reachable at a documented URL.
- [ ] The site navigation exposes every page produced by 0175 (its narrative
      pages and any sections it relocates) and 0176 (the `docs/skills/` reference
      pages), grouped to reflect the `docs/` tree structure — verified against
      those siblings' final Technical Notes mappings.

## Open Questions

These are deliberately deferred and will be scoped when the work item is picked
up; they do not block promoting it from draft. Each chosen answer should be
captured — the generator and host as an ADR/note per AC1, and recorded in
Dependencies if it introduces an external coupling.

- Which generator best fits the project's constraints (build toolchain already
  spans Rust/Python/TS/shell via `mise`)?
- Where should the site be hosted — GitHub Pages, or elsewhere — and on what
  trigger should it publish (push to the default branch, a release tag, or
  manual dispatch)?
- Should tooling selection be split out as a dedicated spike (re-kinding this
  item) before the build work begins?

## Dependencies

- Blocked by: 0175, 0176 (the `docs/` tree must exist to publish). 0177's
  navigation scope tracks the *final* page set those siblings produce —
  including the resolution of 0175's two unassigned README sections — not merely
  the tree's existence.
- External: a publishing host and CI deploy permissions. The default candidate
  is GitHub Pages via GitHub Actions, which needs a one-time Pages-enablement
  repo setting and deploy permissions; the actual external coupling is confirmed
  once the host is chosen during pickup (see Open Questions).
- Related: 0145.

## Assumptions

- A static-site generator over a markdown source tree is sufficient; no
  dynamic/server-rendered docs platform is required.

## Technical Notes

## Drafting Notes

- Left as kind `story`; consider re-kinding the tooling-selection portion to a
  spike if the decision needs a time-boxed investigation.
- Author inherited from parent 0145.

## References

- Parent: 0145 — Documentation Improvements
- Related: 0175, 0176
