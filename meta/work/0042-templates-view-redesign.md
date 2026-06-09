---
id: "0042"
title: "Templates View Redesign"
date: "2026-05-06T14:04:04+00:00"
author: Toby Clemson
kind: story
status: done
priority: low
tags: [design, frontend, templates, backend]
type: work-item
schema_version: 1
last_updated: "2026-05-06T14:04:04+00:00"
last_updated_by: Toby Clemson
blocked_by: ["work-item:0033", "work-item:0037"]
source: "design-gap:2026-05-06-current-app-vs-claude-design-prototype"
---

# 0042: Templates View Redesign

**Kind**: Story
**Status**: Done
**Priority**: Low
**Author**: Toby Clemson

## Summary

As a power user customising the accelerator plugin with team-level and/or
individual overrides, I want to see at a glance which template tier is active
and verify the rendered content, so that I can understand the effect of my
configuration without inspecting the resolution logic by hand.

Redesign the templates index to surface per-tier presence inline on each row
(with the winning tier shown using the green Chip variant), add a visible
`sha256-…` content-hash label as the first row inside the rendered template
preview pane on the detail screen, and expose the backing `sha256` field on
the backend detail endpoint so the value is read from the server rather than
computed in the client.

## Context

The current app's templates view (`/library/templates/{name}`) renders three
stacked tier panels with an active-tier marker plus the rendered template body.
The prototype's `templates-view` adds an inline tier-presence row on the index
list (each tier shown in one of three states — `absent`, `present`, or
`present-and-winning` — using Chip variants from 0038), expands a chosen
template into a stacked tier card alongside a separate template preview pane
on the right that renders the template body with a `sha256-…` content-hash
label as its first row.

The three tiers are, in resolution order (lowest priority first):

- `plugin default` — the baseline shipped with the plugin.
- `user override` — committed in the repo; team-shared by nature.
- `config override` — defined in `config.md` (team-shared) or
  `config.local.md` (local-only), as defined in 0028.

Reference screenshot: `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/templates-view.png`.

## Requirements

- On the templates index list, render an inline per-tier presence indicator
  for each row, showing the three tiers in resolution order from lowest to
  highest priority — `plugin default` → `user override` → `config override` —
  with each tier in its appropriate state.
- The tier-presence row renders three distinct visual states per tier,
  each bound to a Chip variant from 0038's palette:
  - `absent` (no override defined for that tier) — neutral Chip variant
  - `present` (override exists but is not the winning tier) — indigo Chip
    variant
  - `present-and-winning` (override exists and is the winning tier) —
    green Chip variant
  The green Chip variant is the sole visual signal that a tier is
  winning on the index — no additional row-level highlight is applied.
- The `plugin default` tier is always rendered as `present`; it is the
  winning tier only when no user override or config override exists above
  it.
- On the templates detail screen, render a two-column layout with the
  stacked tier card occupying the left column and the template preview
  pane occupying the right column. Collapse / responsive behaviour at
  narrow viewport widths is out of scope for this work item.
- On the detail screen's stacked tier card, the winning tier card is
  marked with an accent-coloured outline ring (the existing active-tier
  marker, retained as-is in both visual treatment and meaning).
- The `sha256` value is supplied by the backend on the templates
  **detail** endpoint, not computed client-side, so the same value can be
  pushed via SSE when the resolved content changes (following the
  `Option<String>` etag pattern referenced in 0055).
- Render a visible `sha256-…` content-hash label as the first row inside
  the template preview pane (immediately above the rendered template
  body). The label is a UI element on the page; it is **not** an HTTP
  `ETag:` response header. It displays the literal prefix `sha256-`
  followed by the full 64-character lowercase hex digest of the resolved
  winning-tier content.
- The content-hash label updates live via SSE when the resolved
  winning-tier content changes for the template currently being viewed.
  The displayed text reflects the new hash within 1s of the client
  receiving the SSE event, without a full page reload.
- The content-hash label is not interactive: the cursor remains the
  browser default, there is no hover background/colour change, no
  tooltip, no click handler, and no copy/selection helper. The browser's
  default text-selection behaviour for static text is otherwise
  unchanged.
- The redesign surfaces tier presence and winner more explicitly on the
  index without changing the underlying tier resolution model.

## Acceptance Criteria

- [x] Given the user navigates to the templates index, when each row
  renders, then a per-tier presence row is visible inline showing all
  three tiers (`plugin default`, `user override`, `config override`) in
  that fixed left-to-right order, each in its appropriate state.
- [x] Given a template has no override defined for a given tier, when the
  index row renders, then that tier is shown using 0038's neutral Chip
  variant (`absent`), distinct from the indigo Chip variant used for
  `present` and the green Chip variant used for `present-and-winning`.
  No additional row-level highlight is applied to the winning tier — the
  green Chip is the sole winning-state signal on the index.
- [x] Given a template has no user override and no config override, when
  the index row renders, then the `plugin default` tier is in the
  `present-and-winning` state and the `user override` and `config
  override` tiers are in the `absent` state.
- [x] Given a template has both a user override and a config override,
  when the index row renders, then the `user override` tier is in the
  `present` (non-winning) state and the `config override` tier is in the
  `present-and-winning` state.
- [x] Given the user opens a template detail screen at a viewport width
  of ≥ 1024px, when the page renders, then the page uses a two-column
  layout with the stacked tier card in the first (left) column and the
  template preview pane in the second (right) column. Collapse
  behaviour at narrower widths is out of scope.
- [x] Given the detail screen renders, when the winning tier card is
  inspected, then it is marked with an accent-coloured outline ring
  (the active-tier marker); non-winning tier cards have no such ring.
- [x] Given the template preview pane renders, then the first row inside
  the pane is the content-hash label, displaying the `sha256-` prefix
  followed by a truncated digest (first 5 hex characters + ellipsis) in
  a compact monospace form; the full `sha256-<64-hex>` digest is
  available on hover via the element's `title` attribute. The rendered
  template body follows immediately below.
- [x] Given a `GET` request to the templates detail endpoint
  (`/api/templates/{name}`) for a template whose winning-tier content is
  non-empty, when the response is returned, then the response body
  contains a `sha256` field whose value has the form `sha256-<hex>`,
  where `<hex>` is the 64-character lowercase hex SHA-256 digest of that
  resolved content (matching the project-wide etag encoding used by
  `TemplateTier.etag` and `SsePayload::DocChanged.etag`).
- [x] Given a template whose resolved winning-tier content is empty or
  absent, when the detail endpoint responds, then the `sha256` field is
  omitted from the response body — it is not present as `null` or as an
  empty string.
- [x] Given the backend detail endpoint response omits the `sha256`
  field, when the detail screen renders, then the content-hash label is
  not displayed.
- [x] Given the content-hash label is rendered, when its displayed value
  (recovered from the `title` attribute so the full digest, not the
  truncated form, is compared) is matched against the response's
  `sha256` field, then the two values are equal.
- [x] Given the backend emits an SSE event named `template-changed` on
  the `/api/events` stream (payload `{ template: string, sha256?:
  string }`, where `sha256` is omitted when the new winning content is
  empty or absent) for the currently-viewed template, when the client
  receives the event, then the content-hash label text reflects the new
  hash within 1s of client receipt without a full page reload.
- [x] Given the content-hash label is rendered, when the user hovers or
  clicks on it, then the cursor remains the browser default, no hover
  background/colour change occurs, no click handler fires, and no
  copy/selection helper is presented. The browser-native tooltip
  surfaced by the `title` attribute (carrying the full digest) is
  intentional and is the sole hover affordance.

## Dependencies

- Blocked by: 0033 (token system), 0037 (Glyph for template-type icons),
  0038 (Chip for tier-presence indicators), 0041 (Page wrapper
  consistency).
- Depends on 0028 (userspace customisation directory, assumed delivered)
  for the `config.md` / `config.local.md` semantics underlying the
  `config override` tier.
- Depends on 0029 (template resolution model, assumed delivered) for the
  three-tier `plugin default` / `user override` / `config override`
  semantics this work item presents.
- Depends on 0055 (SSE `/api/events` channel and `Option<String>` etag
  pattern, assumed delivered) for the live-update infrastructure this
  work item extends with the `template-changed` event.
- Blocks: none.

## Assumptions

- The three-tier resolution model (`plugin default` / `user override` /
  `config override`) from 0029 is delivered and stable; this work item
  is purely presentational over that model and does not change
  resolution semantics.
- The user is a power user familiar with the three-tier resolution
  model. The redesign is not aimed at first-time users for whom the
  `plugin default` tier is sufficient.
- The `plugin default` tier is always present in the resolved tier set —
  there is no "no template at all" state for a row on the index.

## Technical Notes

- "content-hash label" is a project-local term used throughout this
  work item; it is **not** an HTTP `ETag:` response header. The
  precedent from 0055 used the term "etag" for the analogous concept
  (a server-supplied content hash exposed in the response body); this
  work item adopts "content-hash label" for the UI rendering and
  "`sha256` field" for the JSON property to avoid the HTTP-ETag
  collision.
- Backend scope is in-scope for this work item: the templates **detail**
  endpoint (`GET /api/library/templates/{name}`) must add a new `sha256`
  field on the response payload, containing the 64-character lowercase
  hex SHA-256 digest of the resolved winning-tier content.
- The field is serialised as `Option<String>` on the backend with
  `#[serde(skip_serializing_if = "Option::is_none")]` semantics. On the
  wire the key is either present with a string value or absent — it is
  never `null` and never an empty string. The frontend must treat field
  absence as the "no winning content" signal. The index endpoint is
  unchanged.
- A new SSE event named `template-changed` must be emitted on the
  existing `/api/events` stream when the resolved winning-tier content
  for any template changes. The name follows the kebab-case convention
  established by the existing change-notification events on
  `/api/events`. The event payload is
  `{ template: string, sha256: string }`. This mirrors the
  `Option<String>` content-hash shape established by 0055.
- The underlying three-tier resolution model is defined in 0029; this
  work item is purely presentational over that model and should not
  change resolution semantics.
- Tier-presence states map to existing 0038 Chip variants (neutral /
  indigo / green); no new chip variant is required.

## Drafting Notes

- User persona was clarified during enrichment: power users with team
  and/or individual overrides — not first-time users. Scope decisions
  favour clarity-of-effect over discoverability.
- The `sha256` field is sourced from the backend (not computed
  client-side) to enable SSE-driven live updates, even though
  client-side hashing would have been simpler for the static-render
  case. The SSE open question was resolved in favour of live updates,
  so the backend `sha256` field and the `template-changed` SSE event
  together form the complete delivery for the detail screen.
- 0055's deliverables (the `/api/events` SSE channel and the
  `Option<String>` content-hash JSON shape) are treated as assumed
  delivered in Dependencies. This work item extends both: it adds the
  per-template `sha256` field on the templates detail endpoint and the
  `template-changed` SSE event on the existing channel. 0055 is not a
  hard blocker because its UI (the sidebar activity feed) is
  independent of this work, but the underlying plumbing must be in
  place.
- This work item bundles a small backend addition (the `sha256` field
  on the detail endpoint + the `template-changed` SSE event) with the
  frontend redesign because the hash value must originate server-side.
  Both halves are owned within this story; ownership split was
  considered and rejected as it would have added coordination overhead
  for ~20 lines of backend code.
- The index and detail changes are kept in one story rather than split
  because the redesign reads as a single screen redesign; partial
  delivery would leave the screen visually inconsistent.
- Tier-presence row treats the `plugin default` tier as always present;
  the `absent` state applies only to the `user override` and `config
  override` tiers when no override is defined for them. The three
  states map directly to 0038 Chip variants (neutral / indigo / green).
- Tier terminology was clarified during review (Pass 2) — earlier
  drafts used `default / team / user` labels, which conflated two
  distinct overlay mechanisms. The correct model is `plugin default`
  → `user override` (committed in the repo, team-shared by nature) →
  `config override` (in `config.md` for team-shared, or
  `config.local.md` for local-only).
- The content-hash label is visual-only by design; selection/copy
  affordance was considered and explicitly rejected.
- UI term was changed from "etag header" to "content-hash label"
  during review (Pass 3) to avoid collision with the HTTP `ETag:`
  response header. "etag" is reserved for any actual HTTP-level
  concept; the JSON field is `sha256`.
- This work item is marked as low priority because the redesign is a
  single screen with no shared-component dependencies that other items
  need; the new backend `sha256` field and SSE event requirements are
  narrow and don't change the low-priority judgement.

## References

- Source: `meta/research/design-gaps/2026-05-06-current-app-vs-claude-design-prototype.md`
- Screenshots: `meta/research/design-inventories/2026-05-06-140608-claude-design-prototype/screenshots/templates-view.png`
- Related: 0028 (userspace customisation directory), 0029 (template
  resolution model), 0033, 0037, 0038, 0041, 0055 (etag/SSE pattern)
