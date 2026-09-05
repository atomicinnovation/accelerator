---
type: "pr-description"
id: "94"
title: "Relocate insecure-local override marker to .accelerator"
date: "2026-08-31T23:41:07+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "work-item:0272"
parent: "work-item:0272"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/94"
pr_number: 94
tags: ["security", "config", "cleanup"]
revision: "6c531963e1e26480068852cdb22b48a47ca0348b"
repository: "accelerator"
last_updated: "2026-08-31T23:41:07+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Relocate insecure-local override marker to .accelerator

## Summary

Moves the credential-override marker from `.claude/insecure-local-ok` to
`.accelerator/allow-insecure-local`, consolidating every Accelerator-owned file
under `.accelerator/` and aligning the basename with the
`ACCELERATOR_ALLOW_INSECURE_LOCAL` variable it pairs with. The production path
becomes a single `INSECURE_MARKER_RELATIVE` constant so the compiler — not a
grep — guarantees no caller drifts to a stale path. Hard cutover: the old path
is not read as a fallback; the feature is unreleased, so no committed marker is
stranded.

## Changes

- **New `INSECURE_MARKER_RELATIVE` constant** in `tracker-support`
  (`credentials.rs`), re-exported at the crate root, carrying
  `.accelerator/allow-insecure-local` as the single source of truth.
- **Five callers repointed** at the constant: the three production context
  builders (`jira-cli`, `linear-cli`, `work-cli`) and the two contract harnesses
  (`jira-client`, `linear-client`).
- **Three flat test fixtures** renamed to the bare `allow-insecure-local`
  basename, keeping the one marker-write site free of `create_dir_all`.
- **Three new tests**: a seam pin on the constant's value, plus two Unix-gated
  symlink characterisation tests closing a standing coverage gap on the
  personal-config and marker symlink gates.
- **Docs corrected**: the resolver docstring and both `SKILL.md` override
  passages name the new path; the inaccurate Jira "warns if looser than `0600`"
  sentence becomes the shared refuse-plus-override description, so it mirrors the
  Linear section as that section claims.
- **`public-api.txt` regenerated** to record the new `pub const` on the
  `tracker-support` surface.

The override's security semantics are byte-for-byte unchanged — only the
supplied path moves. The path-agnostic resolver gates
(`refuse_insecure_personal_config`, `insecure_override_allowed`) are untouched.

## Context

- Work item: `meta/work/0272-relocate-insecure-local-override-marker.md`
  (renamed from `0272-move-insecure-local-ok-under-accelerator.md`).
- Plan: `meta/plans/2026-08-31-0272-relocate-insecure-local-override-marker.md`
  (status `done`).
- Validation:
  `meta/validations/2026-08-31-0272-relocate-insecure-local-override-marker-validation.md`
  (result `pass`).

This PR bundles the full 0272 lifecycle: research, work item, work-item and plan
reviews, plan, implementation, and validation report.

## Testing

- [x] Rust credential suite passes, including the seam pin and both symlink
  tests: `cargo test -p tracker-support --test credentials` (22 passed).
- [x] Full read-only CI mirror passes: `mise run check` (exit 0).
- [x] Acceptance greps hold: no `insecure-local-ok` or
  `.claude/allow-insecure-local` under `cli/`, `skills/`, `hooks/`; constant
  carries the new path; five callers reference it; both `SKILL.md` passages and
  the resolver docstring name the new path; `warns if looser than` is gone;
  `.accelerator/allow-insecure-local` is not gitignored.
- [ ] Manual runtime override paths (marker honoured / legacy path refused /
  symlinked or untracked marker refused) — indirectly covered by the passing
  unit suite; run in a scratch repo before relying on the feature.

## Notes for Reviewers

- ⚠️ This branch stacks on `0221-mark-canonical-quoting-standard-done` (PR #92).
  The diff therefore includes one commit unrelated to 0272 —
  `f55a693b Reconcile 0136 decomposition against the parent-linked child set`,
  touching only `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`. Review it
  as incidental, or merge PR #92 first so it drops out of this diff.
- The plan's original Key Discovery claimed the public-API snapshot was
  unaffected. That was wrong — a new `pub const` changes the surface — so
  `public-api.txt` was correctly regenerated (+2 lines).
- Focus review on `credentials.rs` (constant + docstring) and the two new
  symlink tests in `tests/credentials.rs`.
