# ADR-0026 body was edited in place, against ADR-0031 immutability

## Problem

`meta/decisions/ADR-0026-css-design-token-application-conventions.md` is
`status: accepted`, but its **body** has been edited after acceptance to record
the 0075 typography supersession by ADR-0036:

- a supersession blockquote near the top (`:17` — "… (typography rows) is
  superseded by ADR-0036. The spacing rule in §2 and …");
- inline "governed by ADR-0036" pointers in §3 (`:112`, `:129`).

(It also carries `superseded_by: "adr:ADR-0036"` in frontmatter — that part is
fine; frontmatter supersession metadata is an *allowed* write.)

ADR-0031 (`ADR-0031-skill-level-adr-immutability.md`) is unambiguous:

> Only `proposed` permits content edits. … No further edits are permitted on
> non-`proposed` ADRs.

The only writes allowed on a non-`proposed` ADR are status transitions
(`accepted → superseded`/`deprecated`) and their *associated metadata*
(`superseded_by`, `rejected_reason`). Editing prose/table rows in the body is
not permitted. So the 0075 typography pointers added to ADR-0026's body appear
to have crossed the immutability line.

## Why it surfaced

Surfaced while reviewing the radius-tokens plan
(`meta/plans/2026-06-02-0090-radius-tokens-consumption.md`). That plan
originally proposed the *same* in-place amendment for radius — remove the §3
"In-between border radii" row and add an ADR-0039 pointer — explicitly citing
the 0075 typography amendment as precedent. The plan was corrected to leave
ADR-0026 entirely untouched and record the supersession solely on the new
(proposed) ADR-0039 via its `supersedes` edge (see that plan's Authorised
Deviation 6). The radius work no longer repeats the violation, but the existing
typography edit remains in ADR-0026.

## The tension this exposes

The supersession lifecycle has no clean way to express **partial** supersession:

- ADR-0026 covers several concerns (spacing §2, typography, code-block §5,
  irreducible-literals §3). Only *parts* are superseded over time.
- ADR-0031's transition table only offers whole-document `accepted → superseded`
  — which would be wrong here, because §2 spacing (and others) stay in force.
- So the team's instinct has been to keep ADR-0026 `accepted` and edit the body
  to point at the superseding ADR for the retired sections. That records intent
  legibly but violates immutability.
- The strict-immutability alternative (adopted for radius/0090): leave the older
  ADR fully untouched; the *newer* ADR's `supersedes` edge is the single source
  of truth and the inverse is derivable (per ADR-0034). The cost is that the
  older ADR still reads as if the retired section is active unless the reader
  follows the supersession graph.

## Possible follow-up

- Decide the canonical convention for **partial** supersession (body pointer vs
  supersedes-edge-only) and capture it — likely an amendment or companion to
  ADR-0031, or a note in ADR-0034's linkage guidance.
- If supersedes-edge-only wins: consider reverting ADR-0026's typography body
  pointers (`:17`, `:112`, `:129`) — but reverting is itself a body edit on an
  accepted ADR, so it would need to ride the agreed convention/transition rather
  than be a casual fix.
- If body-pointer wins: relax ADR-0031 to explicitly permit a narrow
  "supersession-pointer" edit class on accepted ADRs, so the practice is
  sanctioned rather than a quiet exception.

Either way the inconsistency should be resolved deliberately, not by continuing
to edit accepted ADRs ad hoc.

## References

- `meta/decisions/ADR-0026-css-design-token-application-conventions.md` (`:6`,
  `:17`, `:112`, `:129`)
- `meta/decisions/ADR-0031-skill-level-adr-immutability.md`
- `meta/decisions/ADR-0034-*` (typed linkage / supersession edge derivability)
- `meta/plans/2026-06-02-0090-radius-tokens-consumption.md` (Authorised
  Deviation 6 — the strict-immutability approach adopted for radius)
