---
title: Capture a Decision
description: How to record an architectural decision as an ADR, get it
  reviewed, and manage its lifecycle.
---

This guide records an architectural decision — a technology choice, a
design trade-off, a convention — as an architecture decision record
(ADR) in `meta/decisions/`, and takes it through review to an accepted,
immutable record.

## Steps

1. **Create the ADR.** Run
   [`create-adr`](../reference/skills/decisions/create-adr.md) with the
   decision topic:

   ```
   /accelerator:create-adr use PostgreSQL row-level security for tenancy
   ```

   The skill interviews you about the forces at play, the alternatives
   you considered, and the trade-offs, then drafts the record. Once you
   approve, it writes `meta/decisions/ADR-NNNN-<description>.md` with
   `status: proposed`. Numbers are allocated sequentially and never
   reused.

2. **Or extract decisions already made.** If the decision is buried in
   a research document or plan, run
   [`extract-adrs`](../reference/skills/decisions/extract-adrs.md):

   ```
   /accelerator:extract-adrs meta/plans/2026-07-10-tenancy.md
   ```

   (Leave the argument empty to scan all research and plan documents.)
   It surfaces candidate decisions as one-line summaries, you pick
   which to capture, and each approved draft becomes a `proposed` ADR.

3. **Review it.** Run
   [`review-adr`](../reference/skills/decisions/review-adr.md) — bare
   invocation lists all `proposed` ADRs to choose from. The review
   checks the record's quality (real alternatives, both-sided
   consequences) and then asks you to **accept**, **reject**, or
   **revise**. Revisions loop back; acceptance makes the content
   immutable.

4. **Change your mind later — by superseding, not editing.** Accepted
   ADRs cannot be edited. To replace one:

   ```
   /accelerator:create-adr move tenancy to schema-per-tenant --supersedes ADR-0012
   ```

   The old record is marked `superseded` and points at its successor;
   the new one starts life as `proposed` and goes through review as
   normal. To retire a decision without replacing it, run
   `review-adr` with `--deprecate <reason>`.

## The lifecycle at a glance

`proposed` (editable) → `accepted` (immutable) or `rejected`;
`accepted` → `superseded` (via `--supersedes`) or `deprecated`.
Rejected, superseded, and deprecated are terminal.

## See also

- [ADRs](../skills/adrs.md) — the decision-record family overview.
- [`create-note`](../reference/skills/notes/create-note.md) — for
  observations that do not warrant a formal record.
- [Which skill do I need?](which-skill.md)
