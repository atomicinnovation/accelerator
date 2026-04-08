# Tech Debt: `adr-read-status.sh` does not use `config_extract_frontmatter`

## Problem

`skills/decisions/scripts/adr-read-status.sh` contains its own hand-rolled
YAML frontmatter state machine (tracking `IN_FRONTMATTER`,
`FRONTMATTER_CLOSED`, `---` delimiter detection, field matching with
`grep -qE`, and value cleanup via `sed`). The same parsing logic now
exists in three places across the plugin:

1. `scripts/config-common.sh` — the authoritative `config_extract_frontmatter`
   helper (awk-based, closure-aware, emits non-zero for missing or
   unclosed frontmatter).
2. `skills/decisions/scripts/adr-read-status.sh` — hand-rolled bash
   state machine, pre-dates the helper.
3. `skills/tickets/scripts/ticket-read-field.sh` (Phase 1, in-progress
   plan at `meta/plans/2026-04-08-ticket-management-phase-1-foundation.md`) —
   delegates to `config_extract_frontmatter`.

The ticket reader sets the new convention: delegate to the shared helper
and focus the script on its own concern (field lookup + value cleanup).
The ADR reader still carries the old duplicated pattern.

## Divergences that make convergence non-trivial

Beyond "it's duplicated", the two parsers have drifted in observable
behaviour:

- **Duplicate-key handling**: `adr-read-status.sh` has no `break` after
  matching, so a frontmatter with two `status:` lines returns the last
  value (last-match-wins). `ticket-read-field.sh` breaks on first match
  (first-match-wins), matching `config-read-value.sh`. A straight
  migration of the ADR script onto `config_extract_frontmatter` plus
  first-match-wins would be a behaviour change — likely invisible in
  practice but worth calling out.
- **Error messaging**: `adr-read-status.sh` emits a second stderr line
  listing the valid status enum values (`proposed|accepted|rejected|
  superseded|deprecated`). `ticket-read-field.sh` is generic and cannot
  know per-field enums. A convergence needs either an optional
  `--expected-values` flag on the generic reader, or a thin wrapper on
  the ADR side that re-emits the hint on failure.
- **Metacharacter safety**: `adr-read-status.sh` hardcodes `status:` as
  the search prefix, so regex-metacharacter injection is not possible.
  `ticket-read-field.sh` takes a field name at runtime and therefore
  uses bash prefix matching (`[[ "$line" == "${PREFIX}"* ]]`) instead
  of grep/sed regex. Migrating the ADR script to the defensive pattern
  is a no-op for its current inputs but is the right shape for the
  convention.

## Suggested path forward (future phase)

1. Extract a shared "read a named field from YAML frontmatter" primitive
   — either a function in `config-common.sh` or a generic
   `read-frontmatter-field.sh` script — that combines
   `config_extract_frontmatter` with the bash-prefix field lookup and
   the whitespace-then-quotes value-cleanup sed pipeline.
2. Rewrite `adr-read-status.sh` to delegate to that primitive, then
   layer the status-specific enum hint on the error path.
3. Rewrite `ticket-read-status.sh` the same way so both families share
   a single authoritative reader.
4. Update the ADR test suite for the last-match-wins → first-match-wins
   change (or preserve last-match-wins in the primitive — but the
   `config-read-value.sh` precedent points the other way).

## Why Phase 1 does not do this

Phase 1 of the ticket management initiative explicitly scopes out the
ADR migration (see plan's "What We're NOT Doing" section). The ADR
scripts are not under active change, and the risk of subtly shifting
ADR-writer behaviour mid-phase outweighs the DRY benefit. Convergence
is a natural follow-up when a future phase touches ADR tooling for
other reasons.

## References

- Plan: `meta/plans/2026-04-08-ticket-management-phase-1-foundation.md`
- Helper: `scripts/config-common.sh` (`config_extract_frontmatter` at
  line 33)
- Duplicated parsers: `skills/decisions/scripts/adr-read-status.sh`;
  `skills/tickets/scripts/ticket-read-field.sh` (not yet implemented,
  per plan above)
- Convention precedent: `scripts/config-read-value.sh` (awk-based
  first-match-wins)
