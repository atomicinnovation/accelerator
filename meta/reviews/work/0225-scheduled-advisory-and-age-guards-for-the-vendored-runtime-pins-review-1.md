---
type: "work-item-review"
id: "0225-scheduled-advisory-and-age-guards-for-the-vendored-runtime-pins-review-1"
title: "Work Item Review: Scheduled Advisory and Age Guards for the Vendored Runtime Pins"
date: "2026-10-08T23:04:52+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0225"
work_item_id: "0225"
reviewer: "Toby Clemson"
verdict: "COMMENT"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 6
tags: ["security", "ci", "advisories"]
last_updated: "2026-10-09T17:29:19+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Scheduled Advisory and Age Guards for the Vendored Runtime Pins

**Verdict:** REVISE

The work item is well researched and densely specified: feeds, endpoints,
matching predicates, dedup mechanics and every exclusion are named and
justified, and the acceptance criteria use Given/When/Then with negative
cases for each noise filter. It needs revision because the age-and-expiry
half is under-defined. The owner, thresholds and age source are still open
questions, the keyring-age guard has no criterion, the Chromium pin and the
keyring have unclear referents, and the dedup identity for age and expiry
findings is unspecified in a way that could permanently suppress later
breaches.

### Cross-Cutting Themes

- **Unresolved age and ownership policy** (flagged by: completeness,
  testability, dependency, clarity) — the owner, per-pin and keyring maximum
  ages, the warning window and the age source are open questions, yet several
  requirements and criteria depend on them. They block acceptance, cannot be
  fixtured deterministically, and nothing records them as blockers.
- **Keyring age guard is specified but untested and loosely defined**
  (flagged by: completeness, testability, clarity) — no criterion covers the
  keyring maximum-age breach, which is the only control over
  `keys/npm-registry.pem`, and "the keyring" never lists its files.
- **Requirements without matching criteria** (flagged by: completeness,
  testability) — unparseable feeds, the schedule and `workflow_dispatch`
  triggers, fixed versions in non-Chromium issues, owner runbooks and the
  exit status on findings.
- **Two separable halves** (flagged by: scope, clarity) — the 0196 plan
  designed only the age-and-expiry half and deferred advisory matching; the
  item bundles both, and the Context overstates what 0196 designed.

### Findings

#### Major

- 🟡 **Testability / Completeness**: Keyring maximum-age breach has no
  acceptance criterion
  **Location**: Acceptance Criteria
  The Requirements call for an issue when the trust-anchor keyring exceeds its
  maximum age, but no criterion covers it. It is the only guard over the npm
  registry key, so omitting it would leave every box ticked.
- 🟡 **Dependency / Testability / Completeness**: Owner and threshold
  decisions block acceptance but are not captured as dependencies
  **Location**: Open Questions
  Owner, maximum ages, warning window and age source are unresolved, yet
  criteria name them directly. VCS-history age is hard to fixture; the check
  should take "now" as an input.
- 🟡 **Clarity**: The identity of an age or expiry finding is undefined
  **Location**: Requirements
  "Never reopens, open or closed" plus a marker keyed on finding and subject
  means a per-pin age key would suppress every future breach after the first
  issue closes.
- 🟡 **Clarity**: "The Chromium pin" refers to three different things
  **Location**: Requirements
  `chromium.revision` in `pins.toml`, `CHROMIUM_REVISION` and the
  `browserVersion` from `browsers.json` are each used; the age and owner
  requirements do not say which one is tracked.

#### Minor

- 🔵 **Testability**: Unparseable feed responses not covered by a criterion
  **Location**: Acceptance Criteria
  Only "unreachable" is tested; schema drift that parses to all-clear is the
  likelier silent failure.
- 🔵 **Testability**: Chromium criteria miss comparison boundaries and the
  no-fixed-version case
  **Location**: Acceptance Criteria
  Equality, lexical-versus-numeric fourth components, non-Chromium Google
  products and a KEV CVE without an OSV fix are all unspecified.
- 🔵 **Testability**: Node `patched` exclusion and `playwright-core`
  direct/duplicate cases untested
  **Location**: Acceptance Criteria
  An implementation that ignores `patched`, or opens two issues for one GHSA
  listed under both packages, passes.
- 🔵 **Testability**: Dedup criterion is negative-only
  **Location**: Acceptance Criteria
  An over-broad key (per pin rather than per advisory and pin) would pass.
- 🔵 **Testability / Completeness**: Several requirements lack criteria
  **Location**: Acceptance Criteria
  Schedule and `workflow_dispatch` triggers, the `mise` task, fixed version
  for Node and Playwright issues, owner runbooks, and exit status when
  findings open.
- 🔵 **Clarity**: "The trust-anchor keyring" has unclear scope
  **Location**: Requirements
  The item never says whether the keyring includes `npm-registry.pem`, or how
  keyring age is measured.
- 🔵 **Clarity**: "Fixed version" and "affected pin" ambiguous for Node and
  Playwright
  **Location**: Requirements
  The Node feed gives a `patched` range, not a version; "CVE" and "advisory
  ID" are used interchangeably.
- 🔵 **Clarity**: Single owner versus per-pin owners
  **Location**: Open Questions
  The Requirements imply three owners and the Open Question implies one.
- 🔵 **Clarity**: Unclear whether a feed failure stops the other checks
  **Location**: Requirements
  Aborting early lets one flaky feed silence the age and expiry guards.
- 🔵 **Clarity**: "Designed this in two halves" contradicts "advisory
  matching was deferred"
  **Location**: Context
  0196 designed only the age-and-expiry half; the matching rules here are
  new.
- 🔵 **Dependency**: Three external feeds absent from Dependencies
  **Location**: Dependencies
  OSV, the Node security working group feed and CISA KEV have no SLA or
  versioned schema, and an outage fails the run by design.
- 🔵 **Dependency**: No recipient for failed scheduled runs
  **Location**: Requirements
  GitHub notifies only the last cron editor, so a persistent feed failure
  could go unnoticed.
- 🔵 **Dependency**: Repository setup for the dedup label and issue
  permissions not captured
  **Location**: Technical Notes
  A missing label or a capped token permission surfaces only after merge.
- 🔵 **Dependency**: The 0196 relationship understates the hand-over
  **Location**: Dependencies
  This item discharges a 0196 acceptance criterion and needs its shipped pins
  and keys, so 0196 should be recorded as "Blocked by", not only "Relates to".
- 🔵 **Scope**: Age/expiry tripwire and advisory matching are separable
  deliverables
  **Location**: Drafting Notes
  0196 wanted the cheap tripwire landed promptly; it now waits on three feed
  integrations.

#### Suggestions

- 🔵 **Scope**: Scope is larger than a typical task
  **Location**: Frontmatter: kind
  It covers a workflow, dedup plumbing, three feeds, three guard types and
  eleven criteria.
- 🔵 **Scope**: Parent epic theme does not obviously cover CI advisory
  monitoring
  **Location**: Frontmatter: parent
  Epic 0136 migrates shell to Rust; this item is Python plus GitHub Actions.
- 🔵 **Scope**: File slug reflects the pre-widening scope
  **Location**: Summary
  `advisory-feed-monitoring-…` omits the age and expiry guards.
- 🔵 **Dependency**: Recording pin dates may trigger the trust-anchor
  approval gate
  **Location**: Open Questions
  An explicit date beside each pin in `pins.toml` touches the anchor set.
- 🔵 **Dependency**: Tooling to read key expiry not identified
  **Location**: Technical Notes
  The item does not say whether the job uses `gpg` on the runner or a Python
  OpenPGP library.
- 🔵 **Clarity**: Undefined acronyms and plan-specific jargon
  **Location**: Assumptions
  NVD, CPE, GHSA, "per-exec re-verification" and "the reuse path" are not
  defined.

### Strengths

- ✅ Feeds, endpoints, auth, rate limits and blind spots are named precisely,
  so every third-party coupling is traceable.
- ✅ Matching rules are concrete predicates, including the explicit split of
  `browserVersion` from `CHROMIUM_REVISION`.
- ✅ The criteria use Given/When/Then with negative cases for the `win32`-only
  Node advisory, non-KEV Chromium CVEs and the all-clear path.
- ✅ The `playwright`-filed advisory case cites a real GHSA as a regression
  fixture.
- ✅ Exclusions (auto-close, non-KEV CVEs, `ffmpeg`/`winldd`, GitHub Advisory
  Database, PEM expiry) are each justified, so the boundary is deliberate.
- ✅ The Context explains why key expiry is a scheduled guard rather than a
  unit test, and corrects the earlier claim that the stale-pin guard existed.
- ✅ Matching logic lives in `tasks/`, driven by recorded fixtures, with the
  workflow kept as a thin scheduler.

### Recommended Changes

1. **Resolve the Open Questions and record the decisions** (addresses: owner
   and threshold decisions; single versus per-pin owners; age criteria setup)
   Fix the owner model, per-pin and keyring maximum ages, the warning window
   and the age source; an explicit date beside each pin fixtures
   deterministically. State that the check takes the current date as an
   input. Until resolved, list them under Dependencies as "Blocked by".
2. **Define finding identity per kind** (addresses: identity of an age or
   expiry finding; dedup criterion is negative-only)
   Key advisories by advisory ID and pin, age breaches by pin and pinned
   version (or bump date), and expiry by fingerprint and expiry date. Add a
   positive dedup criterion for a new advisory on an already-flagged pin.
3. **Name the Chromium pin and the keyring precisely** (addresses: Chromium
   pin referents; keyring scope)
   Say which artefact carries Chromium's age and owner, how it relates to
   `browserVersion`, and which files form the keyring and how its age is
   measured.
4. **Fill the acceptance criteria gaps** (addresses: keyring age; unparseable
   feeds; Chromium boundaries; Node `patched`; duplicate GHSA; requirements
   without criteria)
   Add criteria for keyring age, schema drift per feed, version equality and
   numeric ordering, a missing OSV fix, `patched` exclusion, one issue per
   GHSA, triggers, fixed version in every advisory issue, owner runbooks and
   exit status on findings.
5. **Specify failure semantics and notification** (addresses: feed failure
   stops other checks; no recipient for failed runs)
   Run every independent check, open its issues, then fail; say how the owner
   learns of a failed run.
6. **Expand Dependencies** (addresses: external feeds; repository setup;
   0196 hand-over; key-expiry tooling)
   List the three feeds, label creation and token permissions, 0196 as
   "Blocked by" with a note that this item discharges its stale-pin
   criterion, and the OpenPGP tooling.
7. **Decide on splitting** (addresses: separable deliverables; task sizing;
   slug)
   Split into the job plus age-and-expiry guards, then advisory matching
   depending on it; or keep the bundle and allow partial delivery. Align the
   slug and kind either way, and correct the Context's "two halves" wording.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is mostly precise. It names concrete feeds,
endpoints, filters and matching rules, and explains why each design choice
was made. The clarity problems sit in the domain nouns the guard depends on:
what "the Chromium pin" is, which keys make up "the keyring", and what
identifies an age or expiry finding for deduplication. A few acronyms and
plan-specific terms appear without definition.

**Strengths**:
- Explicitly separates `browserVersion` from `CHROMIUM_REVISION`.
- Matching rules are concrete predicates per feed.
- Context explains why key expiry is a scheduled guard.
- Actors and the dedup mechanism are explicit.

**Findings**:
- 🟡 major (high) — Requirements — The identity of an age or expiry finding
  is undefined, so "never reopens" could suppress later breaches. The marker
  is `<!-- finding: <id> subject: <pin-or-key> -->`, but `<id>` is undefined
  for age and expiry findings. A per-pin key suppresses every later breach
  once the first issue closes. Define the identity per kind and state that a
  fresh breach after a bump is a new finding.
- 🟡 major (high) — Requirements — "The Chromium pin" refers to three things:
  `chromium.revision` in `pins.toml`, `CHROMIUM_REVISION`, and
  `browserVersion`. Name it once and state its relation to `browserVersion`
  and to the `playwright-core` pin.
- 🔵 minor (medium) — Requirements — "The trust-anchor keyring" has an unclear
  scope; list its files and how its age is measured.
- 🔵 minor (medium) — Requirements — "Fixed version" and "affected pin" are
  ambiguous for Node (a `patched` range) and Playwright; "CVE" and "advisory
  ID" are used interchangeably.
- 🔵 minor (medium) — Open Questions — Single owner versus per-pin owners.
- 🔵 minor (medium) — Requirements — Unclear whether a feed failure stops the
  other checks.
- 🔵 minor (low) — Context — "Designed this in two halves" contradicts
  "advisory-feed matching was deferred".
- 🔵 suggestion (medium) — Assumptions — Undefined acronyms (NVD, CPE, GHSA)
  and plan jargon ("per-exec re-verification", "the reuse path").

### Completeness

**Summary**: A well-populated task with a precise Summary, a motivating
Context, detailed Requirements, eleven Given/When/Then criteria, and filled
Open Questions, Dependencies, Assumptions, Technical Notes and Drafting
Notes. The gaps are two requirements without criteria and unresolved
parameters that requirements depend on.

**Strengths**:
- The Summary states the deliverable and the gap in two sentences.
- The Context explains the 0196 origin and the design rationale.
- The Requirements are specific enough to start work.
- The criteria cover positive and negative cases per source.
- Scoping decisions are recorded in the Assumptions and Drafting Notes.
- The frontmatter is complete and consistent.

**Findings**:
- 🔵 minor (high) — Acceptance Criteria — Neither the keyring maximum-age
  check nor the owner runbook has an acceptance criterion.
- 🔵 minor (medium) — Open Questions — Requirements depend on unresolved
  parameters (owner, ages, warning window, age measurement); resolve them
  before leaving draft.

### Dependency

**Summary**: Feeds and artefacts are named clearly in the Technical Notes, but
Dependencies holds only "Relates to: 0196". Missing: the three feeds whose
outages turn the run red, the owner and threshold decisions acceptance needs,
GitHub repository setup and failure notification, and the 0196 criterion
this item takes over.

**Strengths**:
- External feeds are named with endpoints, auth and limits.
- The fail-loudly requirement acknowledges feed availability.
- Traceability to 0196 is clear and corrected.
- Exclusions are explained.

**Findings**:
- 🟡 major (high) — Open Questions — Owner and threshold decisions block
  acceptance but are not captured as dependencies; add a "Blocked by" entry
  that names who decides.
- 🔵 minor (high) — Dependencies — The three external feeds are absent from
  Dependencies.
- 🔵 minor (medium) — Requirements — No recipient for failure notifications of
  the scheduled workflow.
- 🔵 minor (medium) — Technical Notes — Repository setup for the dedup label
  and `issues: write` is not captured.
- 🔵 minor (medium) — Dependencies — The 0196 relationship understates that
  this item takes over a 0196 acceptance criterion; record it as "Blocked by".
- 🔵 suggestion (low) — Open Questions — Recording pin dates beside the pins
  may trigger the trust-anchor approval gate.
- 🔵 suggestion (low) — Technical Notes — Tooling to read key expiry (`gpg`
  or a Python library) is not identified.

### Scope

**Summary**: One clear purpose with well-drawn, explicit exclusions, but it
bundles two deliverables the source plan kept apart: the cheap age-and-expiry
tripwire and the harder three-ecosystem advisory matching. That makes it
large for a task, and the open age questions now gate the advisory half. The
parent epic placement looks thematically loose.

**Strengths**:
- The Summary, Requirements and criteria describe the same scope.
- Out-of-scope boundaries are explicit and justified.
- The bundling decision is recorded in the Drafting Notes.
- One toolchain and one ownership domain.

**Findings**:
- 🔵 minor (medium) — Drafting Notes — The age/expiry tripwire and advisory
  matching are separable deliverables; split them, or allow partial delivery.
- 🔵 suggestion (medium) — Frontmatter: kind — The scope is larger than a
  typical task.
- 🔵 suggestion (low) — Frontmatter: parent — The theme of parent epic 0136
  does not obviously cover CI advisory monitoring.
- 🔵 suggestion (medium) — Summary — The file slug reflects the pre-widening
  scope.

### Testability

**Summary**: Highly testable overall, with Given/When/Then framing and
negative cases for the noise filters. Gaps: requirements without criteria,
missing comparison boundaries and a missing-fix case, and age and expiry setup
that depends on unresolved Open Questions.

**Strengths**:
- Each criterion maps to a fixture-driven test.
- Negative criteria cover the deliberate filters and the all-clear path.
- The `playwright`-filed advisory has a real regression fixture.
- Fixture-driven matching in `tasks/` keeps tests off live feeds.
- Dedup covers both open and closed issues.

**Findings**:
- 🟡 major (high) — Acceptance Criteria — The keyring maximum-age breach has
  no acceptance criterion.
- 🟡 major (medium) — Open Questions — The age and expiry criteria cannot be
  set up until age measurement and thresholds are decided; take "now" as an
  input and add boundary tests.
- 🔵 minor (high) — Acceptance Criteria — Unparseable feed responses must
  fail the run but no criterion covers them.
- 🔵 minor (medium) — Acceptance Criteria — The Chromium criteria miss
  comparison boundaries and the no-fixed-version case.
- 🔵 minor (medium) — Acceptance Criteria — The Node `patched` exclusion and
  the `playwright-core` direct and duplicate cases are untested.
- 🔵 minor (medium) — Acceptance Criteria — The dedup criterion is
  negative-only and does not pin down the subject key.
- 🔵 minor (high) — Acceptance Criteria — Several requirements have no
  criterion: the triggers, the `mise` task, fixed version for Node and
  Playwright, owner runbooks, and exit status on findings.

## Re-Review (Pass 2) — 2026-10-08T23:17:00+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability / Completeness**: Keyring maximum-age breach has no criterion — Resolved
- 🟡 **Dependency / Testability / Completeness**: Owner and threshold decisions unrecorded — Partially resolved (owner, ages, window and age source decided; bump-date location still open)
- 🟡 **Clarity**: Identity of an age or expiry finding undefined — Partially resolved (four kinds defined; feed-failure identity missing)
- 🟡 **Clarity**: "The Chromium pin" has three referents — Partially resolved (pin named; which value is its "version" in issues and identity is unstated)
- 🔵 **Testability**: Unparseable feed responses untested — Partially resolved (criterion added, but its outcome is unobservable)
- 🔵 **Testability**: Chromium boundaries and no-fixed-version case — Resolved
- 🔵 **Testability**: Node `patched` and duplicate GHSA — Resolved
- 🔵 **Testability**: Dedup criterion negative-only — Resolved
- 🔵 **Testability / Completeness**: Requirements lacking criteria — Partially resolved (owner assignment verified only for feed-failure issues)
- 🔵 **Clarity**: Keyring scope — Resolved
- 🔵 **Clarity**: "Fixed version" and "affected pin" — Resolved
- 🔵 **Clarity**: Single versus per-pin owners — Resolved
- 🔵 **Clarity**: Feed failure stops other checks — Resolved
- 🔵 **Clarity**: "Two halves" contradiction — Resolved
- 🔵 **Clarity**: Undefined acronyms — Partially resolved (OSV, KEV, "hydrate" still unglossed)
- 🔵 **Dependency**: External feeds absent — Resolved
- 🔵 **Dependency**: No recipient for failed runs — Resolved
- 🔵 **Dependency**: Repository setup — Resolved
- 🔵 **Dependency**: 0196 hand-over — Partially resolved (reverse coupling not recorded on 0196)
- 🔵 **Dependency**: Pin dates may trigger trust-anchor gate — Partially resolved (still conditional; keyring date location unstated)
- 🔵 **Dependency**: Key-expiry tooling — Resolved (deferred to planning, recorded)
- 🔵 **Scope**: Separable deliverables bundled — Still present (deliberately kept bundled; scope lens now rates it major)
- 🔵 **Scope**: Larger than a task — Still present
- 🔵 **Scope**: Parent epic theme — Not re-raised
- 🔵 **Scope**: Stale slug — Resolved

### New Issues Introduced

- 🟡 **Testability**: Feed-failure criterion asserts that no "no findings" result is recorded, which nothing observable represents
- 🟡 **Testability**: No criterion that a new feed-failure issue opens after the previous one is closed
- 🟡 **Clarity**: Bump-date location stated inconsistently across Terms, Dependencies and the `RELEASING.md` criterion; "beside the pin" has no meaning for `playwright-core` or the keyring
- 🟡 **Dependency**: 0196 listed as a blocker while this item discharges a 0196 criterion (0196 is `done`, so the blocker is satisfied; the reverse coupling is unrecorded)
- 🔵 **Clarity / Completeness**: Feed-failure issues are not a finding kind in Terms and lack an identity; "never-reopen" names the wrong rule being relaxed
- 🔵 **Clarity**: Check-to-feed mapping unclear when Chromium needs both KEV and OSV
- 🔵 **Testability / Clarity**: Already-expired keys and keys without expiry unspecified
- 🔵 **Testability**: Missing or malformed bump date, or a revision absent from `browsers.json`, has no outcome
- 🔵 **Testability**: Node environment filter exercised only for `linux` and `win32`
- 🔵 **Testability**: Identity of a Node advisory with no CVE or several CVEs undefined
- 🔵 **Dependency**: Source of the pinned `browsers.json` and `playwright-core` version unstated
- 🔵 **Completeness**: Remaining open decisions scattered with no Open Questions section; label and `mise` task unnamed
- 🔵 **Clarity / Scope**: Summary lists three triggers; Requirements define keyring age and feed failure too
- 🔵 **Dependency**: GitHub Actions cron delays and auto-disable after 60 days of inactivity not captured
- 🔵 **Testability**: No post-merge end-to-end criterion

### Assessment

The first pass's structural gaps are closed: policy values are decided, finding
identity is defined, the criteria now cover boundaries and negative cases, and
Dependencies are complete. The remaining majors are mostly introduced by the
new feed-failure lifecycle and the unpinned bump-date location, plus the
deliberately accepted bundling. One more focused edit — pick the bump-date
location, model feed failure as its own issue kind with observable criteria,
and record the 0196 reverse coupling — should bring the item to COMMENT or
APPROVE.

## Re-Review (Pass 3) — 2026-10-09T07:08:28+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability**: Unobservable "no findings recorded" clause — Resolved
- 🟡 **Testability**: No criterion for a new feed-failure issue after closure — Resolved
- 🟡 **Clarity**: Bump-date location inconsistent — Resolved (all in `pins.toml`)
- 🟡 **Dependency**: Circular 0196 ordering — Resolved (Builds on, with hand-over note)
- 🟡 **Scope**: Separable halves bundled — Still present, downgraded to minor (split declined and recorded)
- 🔵 **Clarity / Completeness**: Feed failure lacks identity — Resolved
- 🔵 **Clarity**: Check-to-feed mapping — Resolved
- 🔵 **Testability / Clarity**: Already-expired keys — Resolved
- 🔵 **Testability**: Missing or malformed bump date — Partially resolved (outcome defined; effect on other checks unstated)
- 🔵 **Testability**: Node environment filter — Resolved
- 🔵 **Testability**: Node entry with zero or several CVEs — Resolved
- 🔵 **Dependency**: `browsers.json` source — Partially resolved (npm registry still conditional)
- 🔵 **Completeness**: No Open Questions section — Resolved (no decider or deadline stated)
- 🔵 **Clarity / Scope**: Summary triggers — Resolved
- 🔵 **Dependency**: GitHub cron limits — Resolved
- 🔵 **Testability**: No post-merge criterion — Partially resolved ("completes" lacks a pass condition)
- 🔵 **Completeness / Testability**: Owner assignment — Resolved
- 🔵 **Clarity**: Chromium "version" — Resolved

### New Issues Introduced

- 🟡 **Clarity**: "A new pinned version is a new finding" contradicts the Advisory identity (advisory ID + pin)
- 🟡 **Testability / Clarity**: OSV 404, or a record without a fixed-version field, for a KEV CVE could be a finding or a feed failure
- 🟡 **Clarity / Testability**: Local-input failure does not say whether other checks still run or issues open
- 🔵 **Testability**: CVE records with several fixed versions have no selection rule
- 🔵 **Testability**: Identity renewal untested for key expiry and keyring age
- 🔵 **Testability**: Pin-age boundary not parameterised per pin
- 🔵 **Clarity / Testability**: OpenPGP subkey expiry unspecified
- 🔵 **Clarity**: Nearing and passing expiry are one finding, unstated
- 🔵 **Clarity**: Which keyring actions reset `keyring.bumped`
- 🔵 **Clarity**: Owner-to-GitHub-login resolution unstated
- 🔵 **Clarity**: "Job", "check", "run" and "task" used for overlapping scopes
- 🔵 **Dependency**: Existing `pins.toml` readers must accept the new `bumped` keys
- 🔵 **Completeness**: `RELEASING.md` criterion omits the update-bump-date instruction
- 🔵 **Scope**: Kind `task` understates size (suggestion)

### Assessment

The item is close. Every pass-2 major is resolved; the three remaining majors
are edge semantics — advisory re-alerting after a bump, OSV not-found
handling, and local-input failure scope — each a one-sentence decision plus a
criterion. The npm-registry and `pins.toml`-reader questions can be settled
from the codebase.

## Re-Review (Pass 4) — 2026-10-09T08:48:43+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: Advisory identity contradicted by "new version, new finding" — Resolved
- 🟡 **Testability / Clarity**: OSV not-found ambiguity — Partially resolved (now a feed failure, but which checks it skips is contradictory)
- 🟡 **Clarity / Testability**: Local-input failure scope — Resolved (aborts, no issues)
- 🔵 **Testability**: Several fixed versions — Partially resolved (same-major rule tested; lowest-overall fallback untested)
- 🔵 **Testability**: Identity renewal for key expiry and keyring age — Resolved
- 🔵 **Testability**: Pin-age boundary per pin — Resolved
- 🔵 **Clarity / Testability**: OpenPGP subkeys — Partially resolved (encryption-only subkey exclusion untested)
- 🔵 **Clarity**: Nearing and passing expiry — Resolved
- 🔵 **Clarity**: Keyring refresh — Resolved
- 🔵 **Clarity**: Owner login — Partially resolved (`RELEASING.md` now a machine-read input, unrecorded)
- 🔵 **Clarity**: Guard, check and run terminology — Partially resolved ("check" definition does not fit multi-feed checks; "every guard")
- 🔵 **Dependency**: npm registry and `pins.toml` readers — Resolved
- 🔵 **Completeness**: Bump-date instruction in `RELEASING.md` criterion — Resolved
- 🔵 **Scope**: Bundled halves and `task` kind — Still present (accepted)

### New Issues Introduced

- 🟡 **Testability / Clarity**: `browsers.json` is both a local input and an npm-registry feed response; the abort and feed-failure paths overlap
- 🔵 **Scope / Dependency**: The two-change delivery split does not partition the criteria
- 🔵 **Dependency**: No second trust-anchor approver named
- 🔵 **Testability**: Other local inputs (owner login, `node.version`, keyring parse) have no failure behaviour
- 🔵 **Testability**: Key-expiry day count from a timestamp unspecified
- 🔵 **Testability**: npm advisory with no fix, or several fixes, unspecified
- 🔵 **Completeness**: Issue content for non-advisory kinds only in criteria; owner login and per-kind owner action not stated
- 🔵 **Clarity**: Project jargon (anchor reviewer team, drift test, release lane) unglossed
- 🔵 **Clarity / Scope**: Summary says "disclosed advisory" though Chromium is KEV-only; title omits the keyring (suggestions)
- 🔵 **Dependency**: Assignee must be a repository collaborator (suggestion)
- 🔵 **Testability**: No check that the schedule fires (suggestion)

### Assessment

Both remaining majors are boundary definitions introduced by the pass-3
decisions: which checks an OSV per-CVE gap skips, and where `browsers.json`
stops being a feed response and becomes a local input. Each is one sentence
plus a criterion. The minors are refinements; none blocks planning.

## Re-Review (Pass 5) — 2026-10-09T09:03:55+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability / Clarity**: Scope of an OSV per-CVE gap — Resolved
- 🟡 **Testability / Clarity**: `browsers.json` local input versus feed failure — Resolved
- 🔵 **Testability**: Other local inputs — Resolved
- 🔵 **Testability**: Key-expiry day count — Resolved
- 🔵 **Testability**: npm fix rules — Resolved
- 🔵 **Testability**: Lowest-overall fallback and encryption-only subkeys — Resolved
- 🔵 **Scope / Dependency**: Two-change criteria partition — Resolved (ordering between the changes unstated)
- 🔵 **Dependency**: `RELEASING.md` machine-read input — Partially resolved (`Owner:` line format unspecified)
- 🔵 **Dependency**: Second approver — Partially resolved (Open Question, not a recorded blocker)
- 🔵 **Clarity**: "Check" definition, "every guard", jargon — Resolved
- 🔵 **Scope**: Bundled halves, `task` kind, title — Still present (accepted)

### New Issues Introduced

- 🟡 **Testability / Completeness / Clarity**: Skip mapping omits `vuln/core` failures and OSV record fetches for `playwright-core`
- 🟡 **Dependency**: The 0196 trust-anchor approval gate is assumed to exist. Verified against the repository: `main.yml` runs only `vendor:check-trust-anchors` (a placeholder check), and there is no `CODEOWNERS`, so the second-approver gate was never built
- 🔵 **Completeness**: Initial bump-date values unspecified (seeding with the merge date would mask an already-stale runtime)
- 🔵 **Testability**: Expected fields per feed unlisted for missing-field fixtures
- 🔵 **Testability**: No post-merge live-feed criterion for the second change
- 🔵 **Testability**: Feed-failure issue content (failed request) and label creation unverified
- 🔵 **Clarity**: The post-merge "same conditions" criterion conflicts with open-only feed-failure dedup
- 🔵 **Clarity**: "npm" names the registry, the key and an advisory category; `playwright` versus `playwright-core` pin naming
- 🔵 **Clarity**: The Summary's "so" joins two unrelated reasons
- 🔵 **Scope**: Chromium advisory matching could land last (suggestion)

### Assessment

The pass-4 majors are resolved. One remaining major is mechanical: failures
belong to the check that issued the request. The other is substantive: the
trust-anchor approval gate this item depends on does not exist, so the
Dependencies entry describes infrastructure that was never built, and the
item must decide whether to require it, build it, or drop the assumption.

## Re-Review (Pass 6) — 2026-10-09T14:24:35+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Testability / Completeness / Clarity**: Skip mapping incomplete — Resolved (failures belong to the issuing check)
- 🟡 **Dependency**: Trust-anchor gate assumed — Resolved (recorded as unbuilt, non-blocking)
- 🔵 **Completeness**: Initial bump dates — Partially resolved (rule stated; which history date counts is unspecified)
- 🔵 **Testability**: Expected fields per feed — Resolved
- 🔵 **Testability**: Live-feed post-merge criterion — Partially resolved ("independently confirmed" outage has no procedure)
- 🔵 **Testability**: Label creation and failed-request content — Partially resolved (several failed requests to one feed unspecified)
- 🔵 **Clarity**: Post-merge criterion versus dedup — Resolved
- 🔵 **Clarity**: npm and `playwright` naming, Summary wording — Resolved
- 🔵 **Scope**: Bundled halves — Still present, raised to major again (the explicit two-change split is now cited as evidence)

### New Issues Introduced

- 🟡 **Clarity**: An empty `browsers` array fits both the npm feed-failure rule and the `browsers.json` local-input abort
- 🔵 **Clarity / Scope**: Criteria reference "first" and "second" changes defined only in optional Drafting Notes
- 🔵 **Clarity**: `browsers.json` abort is called local-input but is not in the local-input list
- 🔵 **Testability**: No stated test seam for the issue tracker
- 🔵 **Testability**: Future-dated or malformed bump-date format unspecified
- 🔵 **Testability**: Node outside `vulnerable` and unaffected `playwright-core` negatives (suggestion)
- 🔵 **Dependency**: Owner login missing from Dependencies; gate follow-up and 0196 plan annotation unlinked; reliance on OSV `database_specific`; `gh` tooling
- 🔵 **Clarity**: The carried-over half goes by four names; "advisory feed" in the Summary

### Assessment

The pass-5 majors are resolved. The remaining clarity major is one sentence:
an empty `browsers` array is a feed failure, and only a populated array
lacking a `chromium-headless-shell` entry for the revision aborts. The scope
major has now recurred in five of six passes, and the detail added to make
the bundle workable (the per-criterion change split, change-ordinal criteria)
is itself generating findings. Splitting along the existing boundary would
resolve it and simplify both items.

### Verdict Change — 2026-10-09T17:29:19+00:00

Verdict changed from REVISE to COMMENT by the work item owner after pass 6.
The clarity major (empty `browsers` array) was fixed: a missing or empty
array is an npm-registry feed failure, and only a populated array without a
`chromium-headless-shell` entry for the revision aborts. The scope major
(two separately deliverable halves) is accepted: splitting was considered in
passes 1, 2 and 6 and declined. Low-cost minors from pass 6 were also
applied; the trust-anchor gate work item and the 0196 plan annotation remain
open follow-ups.
