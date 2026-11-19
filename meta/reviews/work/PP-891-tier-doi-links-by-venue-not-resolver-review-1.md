---
type: "work-item-review"
id: "PP-891-tier-doi-links-by-venue-not-resolver-review-1"
title: "Work Item Review: Tier DOI Links by Venue, Not by the doi.org Resolver"
date: "2026-11-19T20:18:45+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:PP-891"
work_item_id: "PP-891"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 7
tags: []
last_updated: "2026-11-20T00:10:24+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Tier DOI Links by Venue, Not by the doi.org Resolver

**Verdict:** REVISE

PP-891 is thorough and precise where it pins down the CLI contract: exact
JSON shapes, exit codes, config precedence, and deterministic
Given/When/Then criteria over recorded Crossref responses. Its weaknesses
sit at the edges of that contract. The web profile's tiering fallback
leaves the Crossref-miss case and the "no landing host" case unassigned.
Pacing and the queue's waiting behaviour have no thresholds or
deterministic criteria. The item has also grown from a profile-text bug
fix into a bundle that includes a hidden refactor of the shared queue
engine across several crates.

### Cross-Cutting Themes

- **Tiering fallback chain has holes** (flagged by: clarity, testability)
  — "when the call fails" does not cover an `ok` result with neither venue
  nor publisher (a Crossref 404, which is the expected arXiv/DataCite
  path). "The host its page fetch landed on" has no referent when the
  landing page refused the fetch. Requirements and ACs word the chain
  differently.
- **Queue-engine generalisation is hidden scope** (flagged by: scope,
  dependency) — it appears only in Technical Notes, touches arXiv's
  existing queue (PP-864), and has no AC for arXiv non-regression.
- **Queue and pacing contract is under-specified** (flagged by: clarity,
  testability) — pool rates are not stated, "exhausted budget" has no
  definition or `reason`, the owner and start of the 100 s deadline are
  unclear, and the waiting criterion accepts either outcome.
- **Requirements with no matching AC** (flagged by: completeness,
  testability) — never a tier, no key, `doi` among `--ticket` families in
  `--help`, per-pool pacing, metadata entries copied unchanged.

### Findings

#### Major

- 🟡 **Clarity / Testability**: Tiering fallback chain is ambiguous and
  leaves the Crossref-miss case undecided
  **Location**: Requirements / Acceptance Criteria (tiering fallback)
  The landing-host fallback applies only "when the call fails", but a
  Crossref 404 is `ok` with no venue and no publisher. That is exactly the
  arXiv DataCite case the Assumptions expect to fall back to `arxiv.org`.
  The AC flattens the chain to venue → publisher → host, so a text check
  cannot settle pass or fail.
- 🟡 **Clarity**: "The host its page fetch landed on" has no clear
  referent, and none at all when the landing page refused the fetch
  **Location**: Requirements
  The item does not say which fetch supplies the host or where the
  researcher reads it. If the landing page is refused and Crossref is
  `unavailable`, the rule forbids the resolver and points at a host that
  may not exist, which risks reintroducing the `doi.org` tier-3.
- 🟡 **Scope / Dependency**: Queue-engine generalisation is significant
  scope missing from Requirements and ACs
  **Location**: Technical Notes
  Generalising the arXiv queue across `cli/research`,
  `cli/research-adapters` and `cli/research-cli` is a refactor across
  several crates that sits inside a bug fix. arXiv (PP-864) is in its
  blast radius and does not appear in Dependencies, and no AC asserts
  arXiv non-regression.
- 🟡 **Scope**: Several separately deliverable pieces are bundled into one
  bug
  **Location**: Requirements
  The item bundles the profile resolver rule, the new `doi` fetch family
  with its Crossref queue, the `crossref.mailto` polite pool, and the
  profile/outputter integration. The quick fix for the reported bug waits
  on the largest piece.
- 🟡 **Testability**: Pacing to each Crossref pool's limits has no
  threshold and no criterion
  **Location**: Requirements (Crossref queue pacing)
  No rates or concurrency are stated for the anonymous or polite pools,
  and no AC checks that pacing changes when `crossref.mailto` is set.
- 🟡 **Testability**: Waiting criterion accepts either outcome and has no
  deterministic setup
  **Location**: Acceptance Criteria (waiting / `--ticket`)
  "Settles it as `ok` or `unavailable`" would pass a ticket path that
  always returns `unavailable`. The criterion also does not say how a test
  holds the queue or avoids a real 100 s wait.

#### Minor

- 🔵 **Clarity / Testability**: "Exhausted budget" has no definition,
  owner or `reason` value
  **Location**: Requirements
  Crossref is called without a key and the item never defines a Crossref
  budget. Only `upstream_error` has a criterion.
- 🔵 **Completeness / Testability**: Several Requirements have no matching
  AC
  **Location**: Acceptance Criteria
  These are: never a tier; calls only Crossref with no key; `doi` among
  `--ticket` families in `--help`; per-pool pacing; metadata entries
  copied unchanged in emitted order.
- 🔵 **Testability**: Retry-After and project-wide deferral lack an
  observable assertion
  **Location**: Acceptance Criteria (429)
  The criterion gives no header value and no tolerance, and does not
  describe the setup for a second caller. It is also unclear whether the
  429 wait counts against the 100 s deadline.
- 🔵 **Testability**: Expected WWW venue is described, not quoted
  **Location**: Acceptance Criteria (`10.1145/371920.372066`)
  `container-title` is a list. No rule says which element becomes `venue`.
- 🔵 **Testability**: Only one example defines a "plausible email"
  **Location**: Acceptance Criteria (`crossref.mailto` validation)
  Borderline inputs (`a@b`, empty, whitespace, `mailto:` prefix) have no
  expected outcome.
- 🔵 **Testability**: Some profile text checks depend on judgement
  **Location**: Acceptance Criteria (web profile)
  "Modelled on the arXiv profile's" has no defined test of similarity.
- 🔵 **Clarity**: Key-ordering requirement is worded in a convoluted way
  **Location**: Requirements
  "In canonical order" can be read as a sorting rule rather than a
  constraint on which keys may be emitted.
- 🔵 **Clarity**: The owner and start of "its 100 s deadline" differ
  between Requirements and AC
  **Location**: Requirements / Acceptance Criteria
  Requirements tie the deadline to the call, while the AC could mean the
  queue. Neither says whether the clock restarts on each `--ticket`
  re-presentation.
- 🔵 **Clarity**: Several domain terms are used without definition
  **Location**: Context / Requirements
  composers, W-id, dossier, citing-once rule, polite pool, venue signals.
- 🔵 **Clarity**: Summary does not acknowledge the Context row it does not
  explain
  **Location**: Summary / Context
  Row 1 ("tier-3, no venue") is not explained by the resolver hypothesis.
  The Summary also says "both paths" while the Context lists four.
- 🔵 **Completeness**: No explicit expected outcome or reproduction of the
  suspected cause
  **Location**: Context
  The expected outcome (tier-1, conference venue) is only implied, and
  nobody has reproduced the web-profile `doi.org` tiering.
- 🔵 **Scope**: OpenAlex search/lookup parity test is unrelated to the fix
  **Location**: Requirements
  It shares no code, tests or rollback path with the web-profile and
  Crossref work.
- 🔵 **Dependency**: Crossref external service not recorded in
  Dependencies
  **Location**: Dependencies
  Dependencies lists only internal work items.
- 🔵 **Dependency**: No order set with PP-897, which edits the same
  Reputation Tiers section
  **Location**: Dependencies
  Built in parallel, either item could overwrite the other's rules, or
  PP-897's domain-class table could classify `doi.org`.

#### Suggestions

- 🔵 **Scope**: Kind "bug" understates a mostly new-capability item
  **Location**: Frontmatter: kind
- 🔵 **Dependency**: PP-864 and PP-865 appear in `relates_to` but not in
  Dependencies
  **Location**: Frontmatter: relates_to / Dependencies
- 🔵 **Testability**: No end-to-end check that the original tier conflict
  is gone
  **Location**: Assumptions / Summary
  Consider a manual re-run of `research-topic conduct` that cites the
  three Context DOIs through `https://doi.org/…`.
- 🔵 **Testability**: Parity test is only meaningful with real
  per-endpoint recordings
  **Location**: Acceptance Criteria (OpenAlex parity)

### Strengths

- ✅ The CLI contract is exact: literal JSON shapes for
  `ok`/`waiting`/`unavailable`, every non-error status exits 0, and
  `E_DOI_MALFORMED` exit 2 is the only non-zero exit.
- ✅ CLI ACs are deterministic Given/When/Then triples over recorded
  Crossref responses (named DOI, 404, publisher-only, unreachable,
  malformed).
- ✅ The Context gives concrete evidence: three DOIs with conflicting
  records, the plugin version, and a dated re-check that rules out the
  OpenAlex field-selection hypothesis.
- ✅ The `crossref.mailto` criteria cover all three precedence sources,
  the anonymous fallback, the malformed-address warning, and keeping the
  address out of URLs.
- ✅ The canonical key order is made correct by construction with a CLI
  test rather than by asking the model to sort.
- ✅ The Drafting Notes and Assumptions record why the item was reframed,
  which alternatives were rejected, and which risks were accepted,
  including that profile ACs are text checks only.
- ✅ Related items carry reasons, upstream items are done ("Blocked by:
  none" is credible), and the required Bash permission is confirmed to
  exist already.

### Recommended Changes

1. **Rewrite the tiering fallback as an ordered list over outcomes**
   (addresses: fallback chain ambiguity; Crossref-miss case; landing-host
   referent)
   Cover `ok` with venue, `ok` with publisher only, `ok` with neither, and
   `unavailable`. Name the fetch that supplies the landing host and where
   the researcher reads it. State what happens when there is no landing
   host. Use identical wording in Requirements and ACs, and add an AC for
   the arXiv/DataCite miss case.
2. **Bring the queue generalisation into the open** (addresses: hidden
   queue refactor; arXiv consumer missing from Dependencies)
   Either split it into a preparatory item that this one is blocked by,
   or move it into Requirements with an AC that arXiv queue behaviour is
   unchanged. List PP-864 in Dependencies either way.
3. **Specify the queue contract** (addresses: pacing threshold; waiting
   criterion; exhausted budget; 100 s deadline; Retry-After)
   State the pacing for each pool, define or remove "exhausted budget"
   (with its `reason`), and fix the deadline's owner and start. Rewrite
   the waiting, `--ticket` and 429 ACs under a controllable clock with
   single expected outcomes.
4. **Decide on the bundle** (addresses: bundled deliverables; parity
   test; kind)
   Either split the item (profile resolver rule / `doi` command / mailto /
   integration, with parity as a chore) or record why it is kept whole,
   and reconsider `kind`.
5. **Add the missing ACs** (addresses: Requirements with no AC; exact
   venue; plausible email; "modelled on")
   Add checks for: no `tier` key, no key or auth on the request, and `doi`
   among the `--ticket` families. Quote the exact WWW venue string and
   the `container-title` selection rule. Give the email plausibility
   rule. List the elements the Waiting section must contain.
6. **Tidy Dependencies** (addresses: Crossref external; PP-897 ordering;
   PP-864/PP-865)
   Add Crossref as an external dependency, set an order or reconciliation
   rule with PP-897, and give PP-864/PP-865 one-line reasons.
7. **Clarity polish** (addresses: key-ordering wording; domain terms;
   Context row 1; expected outcome)
   Split the key-ordering bullet in two, gloss the domain terms, state the
   expected outcome, and note that row 1 is out of scope or unverified.

## Per-Lens Results

### Clarity

**Summary**: The item is precise for the most part. The main gap is the
web profile's tiering fallback chain: "fails" is not defined, `ok` with
neither venue nor publisher is not covered, and the landing host is
undefined when the fetch was refused. There are smaller referent and
terminology issues.

**Strengths**: literal JSON contracts and exit codes; honest reframing in
the Summary; actors named consistently; config precedence stated
identically in Requirements and ACs; Drafting Notes explain the
non-obvious choices.

**Findings**:
- major / high — Requirements — Tiering fallback chain is ambiguous about
  what "fails" means and leaves cases unassigned.
- major / medium — Requirements — "The host its page fetch landed on" has
  no clear referent, and none at all when the landing page refused the
  fetch.
- minor / medium — Requirements — "An exhausted budget" has no defined
  owner for a keyless Crossref call.
- minor / medium — Requirements — Key-ordering requirement is worded in a
  convoluted way.
- minor / low — Requirements — "Its 100 s deadline" refers to the call in
  one place and possibly the queue in another.
- minor / medium — Context — Several domain terms are used without
  definition or link.
- minor / medium — Summary — Summary's cause claim does not acknowledge
  the Context row it does not explain.

### Completeness

**Summary**: Structurally very complete, with every section substantive.
The bug's expected outcome is only implied, the suspected cause is
inferred rather than reproduced, and a few Requirements have no matching
AC.

**Strengths**: concrete inputs and actual outcomes in the Context;
Requirements ready to implement; Assumptions and Drafting Notes are
candid; frontmatter is complete; Dependencies and Open Questions are
filled in on purpose.

**Findings**:
- minor / medium — Context — Bug reproduction lacks an explicit expected
  outcome and a reproduction of the suspected cause.
- minor / high — Acceptance Criteria — Some Requirements have no matching
  Acceptance Criterion.

### Dependency

**Summary**: Mostly well-mapped. Crossref as a runtime external
dependency, the shared queue's existing arXiv consumer, and the overlap
with PP-897 are not captured in Dependencies.

**Strengths**: Related entries carry reasons; the existing Bash permission
is confirmed; Crossref failure modes degrade gracefully; upstream items
are done.

**Findings**:
- minor / high — Dependencies — Crossref external service not recorded in
  Dependencies.
- minor / medium — Technical Notes — Generalising the arXiv queue affects
  an existing consumer that isn't listed.
- minor / medium — Dependencies — No order set with PP-897, which edits
  the same Reputation Tiers section.
- suggestion / medium — Frontmatter: relates_to — PP-864 and PP-865 appear
  in `relates_to` but not in Dependencies.

### Scope

**Summary**: The item has one clear purpose, but it has grown into a
bundle: a new CLI family, a Crossref client and queue, a config setting, a
queue refactor mentioned only in Technical Notes, an outputter change, and
an unrelated parity test. It is still filed as a bug.

**Strengths**: the Summary, Requirements and ACs agree on the core
deliverable; out-of-scope boundaries are explicit; the growth is recorded
as a decision; a single team owns all of the work.

**Findings**:
- major / medium — Technical Notes — Queue-engine generalisation is
  significant scope that is missing from Requirements and ACs.
- major / medium — Requirements — Several separately deliverable pieces
  are bundled into one bug.
- minor / medium — Requirements — OpenAlex search/lookup parity test is
  orthogonal to the fix.
- suggestion / medium — Frontmatter: kind — Kind "bug" understates a
  mostly new-capability work item.

### Testability

**Summary**: The CLI half is highly testable. The weak spots are queue
and pacing behaviour (no thresholds, and either outcome passes) and the
Crossref-miss tiering case. The end-to-end bug fix is only checked by
reading profile text, which the item accepts openly.

**Strengths**: deterministic fixture-based criteria; exact output
contracts; crisp equivalence and parity criteria; thorough mailto
criteria; candour about the limits of the text checks; ordering correct
by construction.

**Findings**:
- major / high — Requirements / ACs — Tiering after a Crossref miss (`ok`,
  no venue, no publisher) is undecided.
- major / high — Requirements — Pacing to each Crossref pool's limits has
  no threshold and no criterion.
- major / medium — ACs — Waiting criterion accepts either outcome and has
  no deterministic setup.
- minor / high — ACs — Retry-After and project-wide deferral lack an
  observable assertion.
- minor / high — Requirements / ACs — Exhausted-budget path has no
  criterion or reason value.
- minor / high — Requirements vs ACs — Several Requirements have no
  matching criterion.
- minor / medium — ACs — Expected venue is described, not given as an
  exact value.
- minor / medium — ACs — Only one example defines a "plausible email".
- minor / medium — ACs — Some profile text checks depend on judgement
  rather than content.
- suggestion / medium — Assumptions / Summary — No end-to-end check that
  the original tier conflict is gone.
- suggestion / low — ACs — Parity test is only meaningful if fixtures are
  real per-endpoint recordings.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-11-19T21:47:00+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity / Testability**: Tiering fallback chain leaves the
  Crossref-miss case undecided — Resolved (ordered four-case ladder)
- 🟡 **Clarity**: Landing-host referent — Partially resolved (defined as
  the `doi.org` redirect host; the fetch that reports it is still unnamed)
- 🟡 **Scope / Dependency**: Hidden queue-engine generalisation — Resolved
  (raised into Requirements with an arXiv non-regression criterion)
- 🟡 **Scope**: Bundled deliverables — Resolved by decision (kept whole,
  rationale recorded); residual minor notes on mailto and parity bundling
- 🟡 **Testability**: Pacing threshold per pool — Partially resolved
  (ceilings stated; no lower bound for the polite pool)
- 🟡 **Testability**: Waiting criterion accepts either outcome — Resolved
- 🔵 **Clarity / Testability**: "Exhausted budget" undefined — Still
  present (the `budget_exhausted` wording added is OpenAlex's quota signal,
  not a Crossref concept)
- 🔵 **Completeness / Testability**: Requirements with no AC — Resolved
- 🔵 **Testability**: Retry-After, exact venue, plausible email, "modelled
  on" — Resolved
- 🔵 **Clarity**: key-ordering wording, deadline owner, domain terms,
  Summary row 1 — Resolved
- 🔵 **Completeness**: expected outcome — Resolved; reproduction —
  Still present (accepted as inferred)
- 🔵 **Dependency**: Crossref external, PP-897 ordering, PP-864/PP-865 —
  Resolved

### New Issues Introduced

- 🟡 **Clarity**: Plausible-address rule accepts `mailto:x@example.org`,
  which the AC rejects
- 🟡 **Clarity / Completeness / Testability**: 20-attempt Waiting cap and
  the researcher-side `waiting` reason appear only in the AC
- 🟡 **Testability**: No outcome-level check that the reported tier
  conflict is fixed (previously a suggestion; the accepted risk is still
  undecided)
- 🔵 **Testability**: 503 AC takes its expected reason from the
  implementation; 100 s limit and ticket cap only partly checked; "one
  engine type" is structural
- 🔵 **Clarity**: venue-to-tier mapping and expected venue string;
  precedence when a higher source holds an implausible address; "may print
  only"; undefined ticket cap
- 🔵 **Dependency**: PP-897 constraint recorded on one side only; effect of
  multi-command blocks on the citing-once merge; PP-864 status
- 🔵 **Scope**: Summary omits the queue generalisation and
  `crossref.mailto`
- 🔵 **Completeness**: Row 1 has no follow-up or Open Question

### Assessment

The first pass's structural problems are fixed: the fallback ladder is
explicit, the queue refactor is visible and guarded, and the CLI criteria
are deterministic. What remains is mostly inconsistencies the edits
introduced between Requirements and ACs (mailto rule, Waiting cap, retry
reason), all of them mechanical, plus one decision: whether the profile fix
needs an outcome-level check. Once those are settled the item should be
ready for planning.

## Re-Review (Pass 3) — 2026-11-19T22:12:35+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: Plausible-address rule vs `mailto:` rejection — Resolved
- 🟡 **Clarity / Completeness / Testability**: Waiting cap only in AC —
  Resolved
- 🟡 **Testability**: No outcome-level check — Partially resolved (manual
  run added, but nothing ensures the run cites the target DOIs)
- 🔵 **Testability**: retry reason, 100 s bound, ticket cap, structural
  engine clause — Resolved
- 🔵 **Clarity**: venue-to-tier mapping, precedence fall-through, "may
  print only", ticket cap — Resolved
- 🔵 **Dependency**: PP-897 reciprocity, citing-once merge, PP-864 status —
  Resolved
- 🔵 **Scope**: Summary omissions — Resolved; bundling of the queue
  refactor, `crossref.mailto` and the parity test — Still present (raised
  as major again by scope, but kept whole by the user's decision after pass 1)
- 🔵 **Completeness**: Row 1 follow-up — Resolved; reproduction — Still
  present (inferred; the closing run is the only confirmation)

### New Issues Introduced

- 🟡 **Clarity**: "Nothing else leads to the fallback" contradicts step 3,
  where an `ok` Crossref miss also reaches the landing host
- 🟡 **Testability**: Manual closing run has no controlled precondition
  that makes it cite the target DOIs
- 🟡 **Testability**: No criterion pins a Crossref proceedings title to
  tier-1
- 🔵 **Clarity**: Expected statement covers row 1; `reason` vs `cause`;
  researcher vs profile as actor; undefined "ticket family" and "project";
  other 4xx unclassified; PP-892 parenthetical
- 🔵 **Testability**: simulated latency and window boundaries for pacing;
  no 429 time bound or mixed-sequence last-retry case; arXiv test coverage
  of the listed behaviours not confirmed; Retry-After reference point and
  cross-process callers; scenario profile criteria lack named evidence
- 🔵 **Dependency**: `WebFetch` redirect behaviour is unrecorded; risk that
  Crossref's limits drift; no order stated for the item's own pieces; the
  closing run needs live services

### Assessment

The remaining findings are about precision, not structure: wording that
contradicts other wording, and test preconditions. They can all be fixed
mechanically. The scope lens's repeated major restates a decision already
taken, and it is recorded as such. Once these fixes are made the item
should be ready for planning.

## Re-Review (Pass 4) — 2026-11-19T23:01:14+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: "Nothing else leads to the fallback" contradiction —
  Resolved
- 🟡 **Testability**: Manual run not controlled — Resolved (seeded brief,
  fails if uncited, tier matched to `openalex lookup`)
- 🟡 **Testability**: Crossref proceedings title not pinned to tier-1 —
  Resolved
- 🔵 **Clarity**: Expected scope, `reason`/`cause`, actor, ticket
  family/project, other 4xx, PP-892 role — Resolved
- 🔵 **Testability**: pacing latency and windows, retry bounds and mixed
  sequence, arXiv characterisation, Retry-After reference, scenario
  evidence — Resolved
- 🔵 **Dependency**: `WebFetch` platform, Crossref limit drift, internal
  order, live-service needs — Resolved
- 🔵 **Scope**: bundling of the queue refactor, `crossref.mailto` and the
  parity test — Still present (now minor; kept whole by the user's decision)
- 🔵 **Completeness**: reproduction — Still present (now raised as a major
  by testability, as a pre-fix baseline)

### New Issues Introduced

- 🟡 **Clarity / Testability**: The tier for the publisher-only and
  landing-host venue branches is unstated, yet the worked-example criterion
  claims to show it
- 🟡 **Testability / Completeness**: No pre-fix baseline run, so the
  closing run cannot show that the fix changed anything
- 🟡 **Testability / Completeness**: Ticket-family isolation is required
  but has no criterion or defined outcome. A related contradiction: the
  shared engine brings arXiv's exit-2 ticket usage errors (malformed,
  mismatched, live), so `E_DOI_MALFORMED` is not the only non-zero exit
- 🔵 **Clarity**: "the same recorded response" referent; the `reason` that
  goes with `lock_contention`; the closing sentence of the waiting
  instructions; when the resolver `WebFetch` runs; the polite-pool window
- 🔵 **Testability**: no lower bound on public-pool pacing; no 100 s bound
  in the waiting criterion; an empty `container-title` list and the
  neither-field case are untested; the closing run skips the page-declared
  route and the dossier-level check; the unreachable case has no injected
  clock; how the clock is shared across processes is unstated
- 🔵 **Dependency**: ordering with PP-865; the arXiv characterisation step
  is missing from the order; capturing Crossref fixtures needs live access

### Assessment

The structure has held steady across passes. The two remaining majors that
need decisions (the tier for the publisher and landing-host branches, and a
pre-fix baseline) are policy choices. The rest are gaps in the edge-case
contract that can be fixed mechanically. One of these, the inherited
ticket usage errors, corrects a stated exit-code contract.

## Re-Review (Pass 5) — 2026-11-19T23:56:29+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity / Testability**: Tier for the publisher and landing-host
  branches — Resolved (publisher tier-2; landing host by domain rules, so
  `arxiv.org` is tier-2)
- 🟡 **Testability / Completeness**: No pre-fix baseline — Resolved
  (a baseline run gates the build)
- 🟡 **Testability / Completeness**: Ticket-family isolation, and the
  exit-code contract — Partially resolved (criteria added; which
  family's error code applies is ambiguous)
- 🔵 **Clarity / Testability / Dependency**: pass-4 minors — Resolved, except
  that the `lock_contention` `reason` is still given only as "as arXiv does"
- 🔵 **Scope**: bundling — Still present (minor; kept by the user's decision)

### New Issues Introduced

- 🟡 **Clarity**: "That family's `TICKET_MISMATCH`" can mean either family
- 🟡 **Testability**: The pacing and concurrency criterion does not require
  calls from separate processes
- 🔵 **Completeness / Testability**: no criterion for `E_DOI_TICKET_MALFORMED`,
  or for the "no request" guarantee on a live ticket; a fall-through past an
  empty higher-precedence mailto is untested; some profile instructions
  (`WebFetch` position, no invented `reason`, live-ticket handling) have no
  text check
- 🔵 **Testability**: the baseline can be falsely refuted when the URLs go
  uncited; the closing run's non-resolver page is unnamed
- 🔵 **Clarity**: unclear referent for "that waiting ticket"; the Expected
  statement overpromises on degraded paths; "source cited by DOI" is
  undefined; live-ticket handling is given only by reference; pronoun
  referents
- 🔵 **Dependency**: the PP-897 coordination should also protect
  `arxiv.org`'s tier; OpenAlex access is needed for fixtures and the closing
  run
- 🔵 **Completeness**: PP-865 is missing from References

### Assessment

Only two majors remain, and both are precision gaps that can be fixed
mechanically (an ambiguous error-code referent and a missing cross-process
precondition). No structural or policy questions are left. Once these
small fixes are made the item is ready for planning.

## Re-Review (Pass 6) — 2026-11-20T00:02:01+00:00

**Verdict:** REVISE

### Previously Identified Issues

- 🟡 **Clarity**: "That family's `TICKET_MISMATCH`" — Resolved
- 🟡 **Testability**: Pacing calls from separate processes — Resolved (but
  see the new pacing finding below)
- 🔵 **Completeness / Testability**: `E_DOI_TICKET_MALFORMED`, no-request on
  live, empty-env fall-through, profile instruction checks — Resolved
- 🔵 **Testability**: baseline false refutation, closing-run page — Resolved
- 🔵 **Clarity**: waiting-ticket referent, Expected overpromise, "source
  cited by DOI", live-ticket handling, pronouns — Resolved
- 🔵 **Dependency**: PP-897 `arxiv.org` tier, OpenAlex access — Partially
  resolved (PP-897's own entry lacks the `arxiv.org` constraint)
- 🔵 **Completeness**: PP-865 in References — Resolved
- 🔵 **Scope**: bundling — Still present (minor; kept by the user's decision)

### New Issues Introduced

- 🟡 **Clarity**: It is unclear whether the tiers in steps 1–2 are new fixed
  rules or the output of the existing Reputation Tiers rules
- 🟡 **Testability**: The pacing criterion's 50 ms latency means the
  concurrency caps never bind, and "more than one in flight" conflicts with
  evenly paced requests
- 🔵 **Clarity / Testability**: the expiry-reason exception is hard to
  parse and has no criterion; "the three DOIs" points to different sets; a
  failed `web metadata` call is not mapped to "printed none"; code
  identifiers are explained only in Technical Notes
- 🔵 **Testability**: "not delayed" has no bound; nothing records the arXiv
  characterisation coverage; no whitespace rejection example; profile-text
  criteria are phrased as behaviour
- 🔵 **Scope**: the widened non-resolver trigger needs its reason stated
- 🔵 **Dependency**: whether queue state and tickets from the previous
  release survive the upgrade is unstated
- 🔵 **Completeness**: record the baseline's reproduction in Context once
  it has run

### Assessment

Both majors come from pass-5 wording and fixtures. Both can be fixed
mechanically: say that steps 1–2 are new explicit rules, and split pacing
into separate rate and concurrency scenarios. One small contract question
remains, about queue state from the previous release.

## Re-Review (Pass 7) — 2026-11-20T00:07:12+00:00

**Verdict:** COMMENT

### Previously Identified Issues

- 🟡 **Clarity**: Steps 1–2 as fixed rules vs existing rules — Resolved
- 🟡 **Testability**: Pacing criterion never binds the concurrency caps —
  Resolved (separate rate and concurrency scenarios)
- 🔵 **Clarity / Testability**: expiry exception, "the three DOIs", failed
  `web metadata`, code identifiers, "not delayed", coverage mapping,
  whitespace example, profile-text phrasing — Resolved
- 🔵 **Dependency**: PP-897 `arxiv.org` reciprocity, upgrade state — Resolved
  (state invalidated by the user's decision)
- 🔵 **Scope**: bundling and the widened trigger — Still present (minor;
  kept by decision, with the reason now recorded)
- 🔵 **Completeness**: reproduction — Partially resolved (the baseline
  reproduction feeds Context; there is no provisional subsection yet)

### New Issues Introduced

- 🟡 **Clarity**: Step 2 lists chapters as publisher-only, but Crossref
  chapter records carry the book title in `container-title`, so under the
  `venue` definition they reach step 1 at tier-1
- 🔵 **Clarity**: the scope of the cross-record tier agreement; the main
  trigger criterion omits "or failed"; the status when the 100 s bound
  arrives during retry backoff; first-hop vs final landing host; where the
  exit-2 outcomes fall in the venue order; who owns the baseline gate
- 🔵 **Testability**: pacing for mixed pools within one project; pacing
  with an invalid mailto; expiry after a 429; exact upgrade error code and
  old-format fixture; closing run doesn't exercise the fallback branches;
  no cap on baseline reruns; exact parity venue string
- 🔵 **Dependency**: old and new plugin versions running side by side; the
  arXiv profile's preprint tier as a coupling; the Claude Code version for
  the `WebFetch` redirect behaviour
- 🔵 **Completeness**: provisional Reproduction subsection
- 🔵 **Scope**: say in the Summary that the change re-tiers pages that
  declare a DOI; mark a split point in the build order

### Assessment

Only one major finding remains, below the threshold of two, so the
verdict moves to COMMENT. That finding is a genuine policy question: how
book chapters and other container types are tiered. Everything else is
mechanical edge-case precision.

## Verdict Change — 2026-11-20T00:10:24+00:00

**Verdict:** APPROVE

Changed from COMMENT to APPROVE by the reviewer: every pass-7 finding was
addressed after that pass, and the work item is ready for planning.
