---
type: work-item-review
id: "0100-configurable-visualiser-auto-shutdown-review-1"
title: "Work Item Review: Configurable Visualiser Auto-Shutdown"
date: "2026-06-06T12:31:39+00:00"
author: "Toby Clemson"
producer: review-work-item
status: complete
target: "work-item:0100"
work_item_id: "0100"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 3
tags: [visualiser, server, configuration, lifecycle]
last_updated: "2026-06-06T12:55:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Configurable Visualiser Auto-Shutdown

**Verdict:** COMMENT

This is a tightly-scoped, structurally complete story whose five Given/When/Then
acceptance criteria map cleanly onto its requirements, with an explicit Out of
Scope section and well-anchored technical notes. The work item is acceptable as-is
but could be improved — the one major finding is an internally inconsistent
description of the configuration surface (a `visualiser:` block in some sections
versus a dotted `visualiser.idle_timeout` key in others), and several minor
testability gaps leave stated requirements (precedence ordering, case-insensitive
disable tokens) unexercised by any criterion.

### Cross-Cutting Themes

- **Inconsistent rendering of the config key** (flagged by: clarity, testability) — the
  config key is written three ways across the document (`visualiser:` block in
  Summary/Out-of-scope, `visualiser.idle_timeout` in Requirements/Technical Notes,
  and bare `idle_timeout` in the disable Acceptance Criterion). Clarity flags this as
  a referent inconsistency; testability inherits it because the criteria that should
  verify "the same key" appear to target different settings.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity**: Inconsistent description of the configuration surface (`visualiser:` block vs dotted key in `.md` files)
  **Location**: Summary
  The Summary/Out-of-scope describe a "`visualiser:` config block" (nested YAML),
  while Requirements/Technical Notes use a `visualiser.idle_timeout` dotted key inside
  `.accelerator/config.md`. A reader cannot tell whether the surface is a nested block
  or a flat dotted key, and the Acceptance Criterion phrasing does not match the
  Summary's.

#### Minor

- 🔵 **Clarity**: Disable-value criterion uses bare `idle_timeout:` while the prior criterion uses `visualiser.idle_timeout:`
  **Location**: Acceptance Criteria
  The configured-timeout case is written `visualiser.idle_timeout: "30m"` but the
  disable case is `idle_timeout: "never"` without the prefix, making the two criteria
  look like they target different keys.

- 🔵 **Dependency**: New `humantime` crate dependency not recorded in Dependencies
  **Location**: Technical Notes
  Technical Notes name `humantime` as the duration parser — a new third-party crate on
  the server — but Dependencies records no such coupling, so the supply-chain/licensing
  impact is invisible at planning time.

- 🔵 **Dependency**: Config-schema / docs / release-notes coupling not captured
  **Location**: Dependencies
  Adding `visualiser.idle_timeout` and changing the default 30m → 8h may require updating
  config-reference docs, a schema, or a changelog; no coupling to those artefacts is named.

- 🔵 **Testability**: Disable criterion uses unbounded "well past any normal timeout"
  **Location**: Acceptance Criteria
  AC4 gives no concrete observation point, so a verifier cannot derive how long to wait
  before declaring the disable behaviour working.

- 🔵 **Testability**: 8-hour and 30-minute criteria lack a tolerance and a fast-forward seam
  **Location**: Acceptance Criteria
  AC1/AC2 give exact durations with no tolerance window and no clock-injection mechanism,
  making the 8-hour case infeasible to test in real time.

- 🔵 **Testability**: Precedence requirement only partially exercised by criteria
  **Location**: Acceptance Criteria
  Requirements state `env > config > default`, but AC3 verifies only env-over-config; no
  criterion independently confirms config-over-default.

- 🔵 **Testability**: Case-insensitive disable-token matching is not covered by any criterion
  **Location**: Requirements
  Assumptions say `"never"`/`0` match case-insensitively, but no criterion exercises a
  mixed-case token (e.g. `"Never"`), so a lowercase-only implementation would pass every
  criterion while violating the contract.

#### Suggestions

- 🔵 **Clarity**: "idle" is used as a shutdown trigger without defining what counts as activity
  **Location**: Context
  "idle" anchors the whole feature but is never defined; a one-line definition of what
  resets the idle timer would remove ambiguity.

- 🔵 **Completeness**: Open decisions captured in Assumptions/Technical Notes rather than an Open Questions section
  **Location**: Assumptions
  The disable-token-set and the env-var/config-key naming are genuinely open ("an
  implementation-level call") but embedded in prose rather than flagged as Open Questions.

- 🔵 **Scope**: Default-raise and full configurability are separable but cohesively bundled
  **Location**: Requirements
  The 30m → 8h default raise alone solves the stated pain and could ship independently if
  the configurability work needs deferring; optionally note this in Requirements.

### Strengths

- ✅ Tightly scoped: every requirement serves the single purpose of making the idle
  timeout configurable; each of the five acceptance criteria maps directly to a
  requirement with no scope drift.
- ✅ The precedence rule (env > config > default) is stated explicitly and reinforced by
  a dedicated criterion.
- ✅ Disable semantics are described consistently across Requirements, Acceptance Criteria,
  and Assumptions, including the caveat that launching-process exit and explicit stop
  still apply.
- ✅ An explicit Out of Scope subsection bounds the work (no CLI flag, no change to the
  other two shutdown triggers).
- ✅ The fail-fast criterion (AC5) is fully specified — named bad value, expected error,
  and the explicit negative (no silent default).
- ✅ The one real upstream coupling (the existing tracker at `server/src/activity.rs`) is
  captured in both Dependencies and References (via work item 0055).
- ✅ Frontmatter is valid and complete (kind=story, status=draft, priority=low, descriptive
  tags).

### Recommended Changes

1. **Pick one canonical form of the config key and use it everywhere** (addresses:
   Inconsistent description of the configuration surface; Disable-value criterion uses
   bare `idle_timeout:`). Choose either the nested `visualiser:` block with an
   `idle_timeout:` child or the dotted `visualiser.idle_timeout` form, show the literal
   expected snippet once, and use that identical fully-qualified form in every Requirement,
   Acceptance Criterion, and Technical Note.

2. **Close the testability gaps in the acceptance criteria** (addresses: unbounded "well
   past"; missing tolerance/fast-forward seam; partial precedence; case-insensitive tokens).
   Add a stated tolerance and note the clock-injection seam in `server/src/activity.rs`;
   add a config-over-default criterion; add a mixed-case disable-token criterion; bound the
   disable check to a concrete idle interval.

3. **Record the new external dependency and any docs coupling** (addresses: `humantime`
   crate not in Dependencies; config-schema/docs coupling). Note the new duration-parsing
   crate in Dependencies, and either name the config-reference/changelog artefacts that need
   updating or confirm none exist.

4. **Surface the open decisions explicitly** (addresses: open decisions in prose; "idle"
   undefined). Add a short Open Questions section for the disable-token-set and naming
   decisions, and add a one-line definition of what counts as "idle" (what resets the timer).

5. **Optionally note the default-raise can ship independently** (addresses: default-raise
   separable). A one-line note in Requirements preserves an early win if the configurability
   work needs to be deferred.

---
*Review generated by /review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is largely unambiguous: actors, triggers, and outcomes are
concrete, the precedence chain is explicit, and disable/error behaviours are consistently
described across Requirements, Acceptance Criteria, and Assumptions. The main clarity
weakness is an inconsistent description of where and how the timeout is configured — the
Summary calls it a `visualiser:` config block while the Requirements use dotted-key
notation in `.md` files — which leaves the actual configuration surface open to two
interpretations.

**Strengths**:
- The precedence rule (env var > config key > built-in default) is stated explicitly in
  Requirements and reinforced by a dedicated Acceptance Criterion.
- Disable semantics are described consistently in Requirements, Acceptance Criteria, and
  Assumptions — same `"never"`/`0` tokens and the same launching-process-exit/stop caveat.
- Out of scope and Assumptions pre-empt likely reader questions (no CLI flag, fail-fast on
  invalid values, which disable tokens are accepted).
- Opaque domain terms (humantime, AtomicI64, the activity tracker) are each given a
  definition, crate name, or file-location anchor.

**Findings**:
- 🟡 **major** (confidence: high) — _Summary_ — Inconsistent description of the configuration
  surface (`visualiser:` block vs dotted key in `.md` files). The Summary/Out-of-scope imply
  a nested YAML block; Requirements/Technical Notes use a `visualiser.idle_timeout` dotted key
  inside `.accelerator/config.md`. A reader cannot tell which, and the AC phrasing does not
  match the Summary. Pick one canonical representation and show the literal snippet once.
- 🔵 **minor** (confidence: medium) — _Acceptance Criteria_ — Disable-value criterion uses bare
  `idle_timeout:` while the prior criterion uses `visualiser.idle_timeout:`, making the two
  look like different keys. Use the identical fully-qualified key form in every criterion/note.
- 🔵 **suggestion** (confidence: low) — _Context_ — "idle" anchors the whole feature but is
  never defined; only Technical Notes hints (request-middleware timestamp). Add a one-line
  definition of what resets the idle timer.

### Completeness

**Summary**: This story is structurally complete and densely populated: Summary, Context,
Requirements, Acceptance Criteria, Dependencies, Assumptions, and Technical Notes are all
present and substantive. Frontmatter is valid (kind=story, status=draft, priority=low), and
the kind-appropriate content is present. No completeness gaps rise to a blocking level; the
only observation is the absence of an explicit Open Questions section despite open
naming/token decisions surfacing in Assumptions and Technical Notes.

**Strengths**:
- All core story sections are present and substantively populated rather than placeholders.
- Context explains the motivating problem rather than restating the Summary.
- Five concrete Given/When/Then criteria define "done" across default, config, env-override,
  disable, and fail-fast paths.
- An explicit Out of scope subsection bounds the work.
- Frontmatter is complete and valid with descriptive tags and a dated authorship record.

**Findings**:
- 🔵 **suggestion** (confidence: medium) — _Assumptions_ — Open decisions (whether to also accept
  `"off"`/`"none"`; whether env-var/config-key naming is final) are captured in
  Assumptions/Technical Notes rather than an Open Questions section, so a reader scanning for
  unresolved decisions could miss them. Add a short Open Questions section or confirm they are
  author-resolved.

### Dependency

**Summary**: Work item 0100 is a small, self-contained configuration story that modifies an
existing subsystem (the idle-timeout activity tracker at `server/src/activity.rs`) without
introducing new shared artefacts or cross-team couplings. Its one substantive dependency is
captured in both Dependencies and References. The main gaps are minor: an external crate
dependency (`humantime`) named in Technical Notes but absent from Dependencies, and no Blocks
entries (acceptable, as there are no current consumers).

**Strengths**:
- The one real upstream coupling (building on the tracker at `server/src/activity.rs`) is
  captured in both Dependencies and References (work item 0055).
- Dependencies distinguishes "Related" (broader lifecycle) from "Builds on" (the concrete
  tracker the change extends).
- Correctly identifies no downstream consumers — a config knob, not a shared contract — so the
  absence of Blocks entries is appropriate.

**Findings**:
- 🔵 **minor** (confidence: high) — _Technical Notes_ — The `humantime` crate is named as the
  duration parser, introducing a new third-party crate, but this is not reflected in
  Dependencies (affects dependency tree, supply-chain review, licensing). Add a Dependencies
  note for the new crate.
- 🔵 **minor** (confidence: medium) — _Dependencies_ — Adding `visualiser.idle_timeout` and
  changing the default 30m → 8h may require updating config-schema docs, a config reference, or
  a changelog; no coupling to those artefacts is named. Add a "Coordinates with" entry or
  confirm none exists.

### Scope

**Summary**: This is a tightly-scoped, coherent story: every requirement serves the single
purpose of making the visualiser's idle auto-shutdown timeout configurable. Summary,
Requirements, and Acceptance Criteria all describe the same scope, and an explicit Out of Scope
section bounds the work. The declared "story" kind fits a single increment of
system-configurability value owned end-to-end by one team within one service boundary.

**Strengths**:
- All five requirements serve one unified purpose; default change, config key, env-var override,
  precedence, disable token, and fail-fast validation are facets of the same mechanism.
- An explicit Out of scope subsection bounds the work unambiguously.
- Summary, Requirements, and Acceptance Criteria are mutually consistent — each criterion maps
  to a requirement.
- Work stays within a single service boundary (the Rust visualiser server) with no cross-team
  coordination.
- The "story" kind is appropriately sized.

**Findings**:
- 🔵 **suggestion** (confidence: medium) — _Requirements_ — Default-raise and full configurability
  are separable but cohesively bundled. The default raise alone solves the stated pain and could
  ship first if the configurability work proves larger. Keep as one story; optionally note that
  the default raise can land independently.

### Testability

**Summary**: The story is unusually testable for its kind: five Given/When/Then criteria cover
the default, config-driven, env-override, disable, and fail-fast behaviours with concrete inputs
and observable outcomes. The main gaps are an unbounded duration phrase in the disable criterion,
an unverifiable real-time 8-hour wait with no stated tolerance or clock-injection seam, and a
couple of requirements (precedence ordering, case-insensitive disable tokens) stated but not
exercised by any criterion.

**Strengths**:
- Acceptance criteria use explicit precondition/action/expected-outcome framing with concrete
  inputs (`"30m"`, `"2h"`, `"soon"`, `"never"`/0).
- The fail-fast criterion (AC5) is fully specified — named bad value, expected action, explicit
  negative (no silent default).
- The disable criterion (AC4) pins down that the server still exits on launching-process exit or
  stop, scoping the disable to idle shutdown only.

**Findings**:
- 🔵 **minor** (confidence: high) — _Acceptance Criteria_ — AC4 uses unbounded "well past any
  normal timeout"; no concrete observation point, so the disable behaviour is effectively
  unverifiable. Specify a concrete bound (e.g. remains up after an idle period exceeding the
  previous 30-minute default, via a shortened/injectable clock).
- 🔵 **minor** (confidence: medium) — _Acceptance Criteria_ — AC1/AC2 give exact durations with no
  tolerance and no fast-forward seam, making the 8-hour case infeasible to test directly. State a
  tolerance and note the activity-tracker clock-injection seam in `server/src/activity.rs`.
- 🔵 **minor** (confidence: medium) — _Acceptance Criteria_ — Precedence `env > config > default`
  is only partially exercised; AC3 verifies env-over-config but no criterion confirms
  config-over-default. Add a config-over-default criterion.
- 🔵 **minor** (confidence: medium) — _Requirements_ — Case-insensitive disable-token matching
  (Assumptions) is not covered; a lowercase-only implementation would pass all criteria. Add a
  mixed-case disable-token criterion (e.g. `idle_timeout: "Never"`).

## Re-Review (Pass 2) — 2026-06-06T12:40:51+00:00

**Verdict:** REVISE

All eleven issues from Pass 1 were resolved by the edits. However, the deeper
pass surfaced **two new major testability gaps** (both: a stated requirement
with no verifying criterion), which meet the configured REVISE threshold of two
major findings. These are quick to close — each needs one more acceptance
criterion plus a couple of clarity tightenings.

### Previously Identified Issues

- 🟡 **Clarity**: Inconsistent config surface (`visualiser:` block vs dotted key) — **Resolved.** Requirements now show the literal nested YAML block once and declare `visualiser.idle_timeout` as the reference form; clarity no longer flags it.
- 🔵 **Clarity**: Disable criterion used bare `idle_timeout:` — **Resolved.** All criteria use the fully-qualified key.
- 🔵 **Clarity**: "idle" undefined — **Resolved.** Context now defines idle and the reset-on-request semantic.
- 🔵 **Completeness**: Open decisions buried, no Open Questions section — **Resolved.** Open Questions section added; completeness returned zero findings this pass.
- 🔵 **Dependency**: `humantime` crate not recorded — **Resolved.** Now a Dependencies entry with supply-chain/licensing flag.
- 🔵 **Dependency**: Config-docs/schema/changelog coupling — **Resolved.** Coordination entry added.
- 🔵 **Testability**: Unbounded "well past any normal timeout" — **Resolved.** Bounded to "exceeding the previous 30-minute default" via the injectable clock.
- 🔵 **Testability**: Missing tolerance / fast-forward seam — **Resolved.** ±5s tolerance and injectable-clock preamble added.
- 🔵 **Testability**: Precedence only partially exercised — **Resolved.** Dedicated config-over-default criterion added.
- 🔵 **Testability**: Case-insensitive token uncovered — **Resolved.** Mixed-case (`"Never"`) criterion added.
- 🔵 **Scope**: Default-raise separable — **Resolved.** Requirements now note the default raise can land independently; scope downgraded it to an optional split.

### New Issues Surfaced

- 🟡 **Testability** (major): No criterion verifies the **numeric disable token `0`** — every disable criterion exercises only `"never"`/`"Never"`; `0` appears parenthetically but is never pinned to the disabled outcome. A plausible mis-implementation (`0` = shut down immediately) would pass all criteria.
- 🟡 **Testability** (major): **Compound duration `"1h30m"` is required but uncovered** — the configured-value criterion only uses `"30m"`; a parser that handles `"30m"` but mis-handles `"1h30m"` would still pass.
- 🔵 **Clarity** (minor): The `0` disable token's **type/quoting is ambiguous** (YAML integer `0` vs string `"0"`; whether `"0s"` counts). *Partly a side-effect of the Pass 1 edits.*
- 🔵 **Clarity** (minor): **"matched case-insensitively" applied to `0`** is meaningless — should be scoped to the textual `never` token only. *Introduced by the Pass 1 edit.*
- 🔵 **Clarity** (suggestion): Accepted duration grammar is illustrated by example only — defer explicitly to the `humantime` grammar or list constraints.
- 🔵 **Dependency** (minor): The existing **visualiser config-loading / env-var-override machinery** (backing `visualiser.binary` / `ACCELERATOR_VISUALISER_BIN`) is not named as the upstream artefact this work extends.
- 🔵 **Dependency** (suggestion): Provisional key/env names are coupled to the docs-coordination entry — note that coordination must use the final names.
- 🔵 **Testability** (minor): Fail-fast "clear error" lacks an observable threshold (exit code / stderr / message format).
- 🔵 **Testability** (minor): **`config.md` vs `config.local.md` layering** is documented but unverified by any criterion.
- 🔵 **Testability** (minor): The **idle-timer reset-on-request** semantic (now in Context) has no verifying criterion.

### Assessment

The first round of edits fully landed — every Pass 1 finding is resolved, and
completeness is now clean. The work item is materially stronger. It does not
yet clear the bar for APPROVE only because the sharper second pass found two
major coverage gaps in the acceptance criteria (numeric `0` disable token and
compound-duration parsing), plus two clarity nits that the Pass 1 wording
itself introduced (the `0` token's type and the "case-insensitively" phrasing).
All are small, well-understood edits. One more iteration — adding two criteria
and tightening the disable-token wording — should reach APPROVE.

## Re-Review (Pass 3) — 2026-06-06T12:52:38+00:00

**Verdict:** COMMENT

Re-ran the four lenses that had findings in Pass 2 (completeness was clean and
was not re-run). **Both Pass 2 major findings are resolved** and no new major
or critical findings were raised — the verdict drops from REVISE to COMMENT.
The work item is acceptable as-is for implementation; everything remaining is
optional polish.

### Previously Identified Issues (Pass 2 → Pass 3)

- 🟡 **Testability**: Numeric disable token `0` uncovered — **Resolved.** A dedicated criterion now pins `visualiser.idle_timeout: 0` to the disabled outcome (explicitly not "shut down immediately").
- 🟡 **Testability**: Compound `"1h30m"` uncovered — **Resolved.** A criterion now asserts shutdown at the 90-minute boundary via the injectable clock.
- 🔵 **Clarity**: `0` token type/quoting ambiguous — **Largely resolved.** The numeric `0` vs string `"never"` usage now reads consistently across Requirements and criteria.
- 🔵 **Clarity**: "case-insensitively" applied to `0` — **Still present** (not addressed; the user opted to fix only the two criteria). Re-flagged as a minor.
- 🔵 **Dependency**: Config-loading / env-override machinery not named — **Still present**, re-flagged at suggestion severity.
- 🔵 **Clarity** (grammar-by-example), **Dependency** (provisional-names↔docs), **Testability** (clear-error threshold; config-vs-config-local layering; idle-timer reset) — **Not re-raised** this pass.

### New Issues Surfaced (all minor / suggestion — none blocking)

- 🔵 **Clarity** (minor): "expressed internally as the same duration-string format" is ambiguous — is the default literally the string `"8h"` through the same parser, or just conceptually equivalent?
- 🔵 **Clarity** (minor): "the previous 30-minute default" used as a test interval sits next to the new 8h default and could read as if 30m still has standing.
- 🔵 **Testability** (minor): Boundary criteria assert shutdown *at* D but not survival *before* D (no "still alive at D−tolerance" negative assertion).
- 🔵 **Testability** (minor): Disable criteria use an open-ended interval; no concrete clock-advance bound (e.g. "advance well past 8h") is stated for the non-event check.
- 🔵 **Dependency** (suggestion): Docs/schema coordination is left as an open conditional rather than resolved to a concrete artefact or discharged.
- 🔵 **Scope** (suggestion): Default raise remains bundled with the configurability machinery (split path already documented).

### Assessment

The work item is now in good shape and ready for planning/implementation. Both
major coverage gaps are closed, the acceptance criteria cover all eight
behaviours (8h default, config-over-default, env-over-config, compound parsing,
mixed-case disable, numeric `0` disable, and fail-fast), and the remaining
findings are minor wording/coverage refinements that an implementer can absorb
without re-scoping. APPROVE is within reach with a couple of optional wording
tweaks, but COMMENT reflects that none of the open items block the work.

### Verdict Override — 2026-06-06T12:55:16+00:00

The reviewer (Toby Clemson) accepted the work item as-is and **overrode the
Pass 3 computed verdict (COMMENT) to APPROVE**. The remaining minor/suggestion
findings are acknowledged as optional polish and are not blocking; the work
item is approved for implementation. The work item status was transitioned
`draft → ready` alongside this decision.
