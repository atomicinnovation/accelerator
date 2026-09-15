---
type: "pr-description"
id: "117"
title: "[0277] Single-Round Web Research Engine"
date: "2026-09-11T16:31:43+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0277"
parent: "work-item:0277"
relates_to: ["work-item:0278"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/117"
pr_number: 117
tags: []
revision: "801ea3c5f1f25d85733a9b1e08cd033b41524c6e"
repository: "accelerator"
last_updated: "2026-09-11T16:31:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0277] Single-Round Web Research Engine

## Summary

The walking-skeleton engine for topic research: it runs the full build loop
once over web sources — `brief` → `outline` → `conduct` (single round) →
`synthesise` — producing a contract-conforming *set* (an on-disk document
collection for one subject, rooted at `meta/research/topics/<slug>/`). This is
the engine half of epic Slice 1 (work item 0121); the sibling PR #118 (work
item 0278) registers the visualiser doc type and nested-manifest indexer that
make these artifacts reader-observable. The two co-land — the engine's output
is only browsable once 0278 lands.

## Changes

- **`(type, kind)`-discriminated corpus schema and template resolution.** The
  frontmatter schema and template lookup now key on a `(type, kind)` pair, so a
  single type (`topic-research`) carries several kinds (manifest, brief,
  outline, finding, synthesis). A new `UnknownKind` violation distinguishes a
  typo'd `kind` on a valid type from an unknown type.
- **General `corpus resolve` subcommand.** `accelerator corpus resolve --type
  <type> <slug>` resolves a slug (or path) to a document root — a flat dated
  file or a nested-manifest set directory — with a dedicated exit-code taxonomy
  (resolved / invalid / ambiguous / not-found / unknown-type / outside-root).
  Every `research-topic` verb but `brief` resolves the set root through it.
- **Topic-research templates** (web-only): manifest, brief, outline, finding,
  and synthesis, each resolving through the 3-tier override (config path → user
  override → plugin default).
- **`research-topic` skill.** Dispatches `brief` + `outline` + `conduct`
  (single round) + `synthesise` on the `configure` pattern, and writes a
  `manifest.md` aggregate root at `brief` time carrying the `primary` pointer
  (`brief.md` before synthesis, `synthesis.md` after).
- **Generic `researcher` agent, web source profile, and finding outputter.**
  The `researcher` is granted WebFetch and specialised at spawn time by an
  injected `(source_profile, question)` pair, mirroring the `reviewer` agent.
  The web profile captures each source URL inline with a reputation tier from a
  closed vocabulary — `tier-1` (authoritative primary), `tier-2` (reputable
  secondary), `tier-3` (unvetted) — reflecting venue standing only, never a
  claim's correctness, so the corpus conforms from the first file written.
- **`paths.research_topics` config key** (default `meta/research/topics`) and
  `topic-research` registered as a typed-linkage source type.
- **Migration m0007** — unify meta corpus frontmatter (backfill + rewrite) onto
  the new schema.
- **Planning artifacts** — the 0277 work item, plan, plan/work reviews,
  codebase research, and decision records.

## Context

- Implements work item `work-item:0277`, under epic `work-item:0121`.
- Base of a stack: PR #118 (`work-item:0278`) targets this branch. The engine
  writes conforming artifacts to disk; 0278 makes them browsable, so the
  vertical demo lands whole only with both.
- Output quality is judged later by the Slice 3 gate (work item 0280); this PR
  establishes the loop and the on-disk contract, not the dossier-quality bar.

## Testing

The branch was built test-first; the changes carry unit tests (corpus schema
and `resolve`, config wiring), integration/conformance tests, and committed
golden and fixture suites (the `topic-research-set` fixtures, `resolve` and
frontmatter goldens).

- [ ] `mise run check` — the read-only CI mirror (format + lint + types across
  all four components). Not run while generating this description; it is the CI
  gate.
- [ ] `mise run test` — the full suite. Not run in this session; CI gate.

## Notes for Reviewers

- **Review and merge this PR before #118** — it is the base of the stack.
- Focus areas: the `(type, kind)` schema overload in frontmatter validation,
  the `corpus resolve` exit-code taxonomy, and the reputation-tier vocabulary
  in the web profile (venue standing, not correctness).
- The m0007 migration rewrites existing meta frontmatter — worth verifying the
  backfill/rewrite against a representative corpus.
- The engine's artifacts are not browsable in the visualiser until #118 lands;
  evaluate the two PRs together for the end-to-end research loop.
