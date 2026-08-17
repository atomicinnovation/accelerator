---
type: work-item-review
id: "0210-provider-client-crates-over-the-tracker-port-review-1"
title: "Work Item Review: Provider Client Crates over the RemoteTracker Port"
date: "2026-08-17T11:17:18+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
parent: "work-item:0171"
target: "work-item:0210"
work_item_id: "0210"
relates_to: ["work-item-review:0171-jira-and-linear-integrations-review-1"]
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: [rust, jira, linear, tracker, clients]
last_updated: "2026-08-17T12:20:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Provider Client Crates over the RemoteTracker Port

**Verdict:** REVISE

0210 is the strongest of the four children on verification craft — the tripwire
that tests itself by planting a violation, the T-relative timeout window, the
every-row table assertion, the enforcing contract route that explicitly refuses
"recording that no gate exists". Two gaps matter more than any of that: the
projection recipes it is responsible for reproducing exactly have no criterion
beyond the absent-description case, even though this child is the only window in
which the bash corpus still exists as an oracle; and the three oracle
transcriptions two siblings verify against name no path or format, so those
sibling criteria cannot be checked when their children are accepted.

This review was conducted as one pass over all four children of 0171. The
cross-child critical — the three port-less bridge capabilities landing in no
child — is stated in full in 0212's review, since 0212 owns the deleting change.

### Findings

#### Major

- 🟡 **Testability**: Projection fidelity has no criterion beyond the
  absent-description case
  **Location**: Acceptance Criteria
  The Requirements demand reproducing both recipes *exactly*, including Jira's
  key-sorted ADF ordering, and 0171's parent requirement said projection is
  "verified against the bash-generated baseline corpus 0194 committed" — a
  clause 0210 dropped. No criterion in any child exercises the corpus offline
  while it is still on disk. 0210 can be accepted with populated-description
  projection unverified, and first detection would be 0212's credentialed run,
  after every deletion has landed.

- 🟡 **Testability**: The oracle-transcription criterion is forward-looking and
  names no artefact for two of three oracles
  **Location**: Acceptance Criteria
  Only the exit-code fixture has a named form. The ADF node-type inventory and
  the eleven-test baseline name no path, no format and no location, yet 0211's
  "every entry in 0210's recorded node-type inventory" and 0212's "at least the
  same fixture cases as 0210's recorded baseline" resolve to them. The criterion
  is also stated against events that have not happened at 0210's acceptance —
  "before 0211 or 0212 begins" — so it is vacuously satisfiable when checked and
  only violable later.

- 🟡 **Clarity / Scope**: "Leaves the production path unchanged" contradicts
  wiring real providers into the composition root
  **Location**: Summary
  The same sentence says the child wires the sync engine to resolve real
  providers rather than fakes *and* leaves the production path unchanged. One
  reading is inert (the skills still call bash); the other puts live Jira and
  Linear calls behind an already-shipped binary before any skill repointing or
  contract gate exists. The two readings imply different merge risk.

- 🟡 **Clarity / Scope**: ADF↔markdown is implemented here but its only
  verification gate lives in 0211
  **Location**: Requirements
  The Requirements put ADF↔markdown inside `jira-client` and make this child
  transcribe the node-type inventory, but no criterion here exercises the
  conversion; 0211 carries it, despite being described as thin adapters over
  these crates. The inventory this child pays to transcribe has no consumer
  inside this child.

#### Minor

- 🔵 **Scope**: An acceptance criterion is phrased as a condition on when
  siblings begin, so it cannot be closed by inspecting this child's own change.
  **Location**: Acceptance Criteria

- 🔵 **Completeness**: The parent's instruction to *reuse* the existing
  `tracker_contract` exclusion mechanism rather than introduce a second one was
  not restated — only its outcome was.
  **Location**: Requirements

- 🔵 **Completeness / Clarity**: Pup rules and public-API snapshots for
  `jira-client` and `linear-client` are claimed by 0211 ("the four crates"),
  though this child creates them and demands `mise run` green at its own merge
  boundary.
  **Location**: Requirements

- 🔵 **Testability**: The copyleft record is paired with a check that cannot
  detect a wrong answer — `deny:check` goes green by committing allowances
  regardless of what the recorded answer says, and that answer decides whether
  0203 becomes a release-path dependency for 0211.
  **Location**: Acceptance Criteria

- 🔵 **Clarity**: "It has no work item; 0171 names its owner" does not resolve —
  0171 names an owner only for its Open Questions, not for the credentialed
  target's provisioning.
  **Location**: Dependencies

### Strengths

- ✅ The tripwire criterion tests the tripwire itself by planting a deliberate
  violation — a rare falsifiable meta-assertion.
- ✅ The timeout criterion is T-relative at two concrete values plus a separate
  defaults assertion, so it is neither wall-clock-expensive nor waveable.
- ✅ The exit-code criterion fails the build on a row with no assertion, and
  names the divergent Linear cases explicitly.
- ✅ The contract-run criterion demands an enforcing route and rules out
  recording that no gate exists.
- ✅ It states plainly that the credentialed target gates *this child's*
  acceptance, not merely the eventual cutover.
- ✅ The absent-description criterion is explicitly offline, so the
  highest-risk behaviour does not depend on the unprovisioned lane.

### Recommended Changes

1. **Add an offline whole-corpus projection criterion** (addresses: projection
   fidelity) For every record in `skills/work/scripts/test-fixtures/`, the
   client's projection is byte-identical to the committed corpus entry for that
   record, per provider, key ordering included — passing with no network target
   and before any deletion.
2. **Name the three transcription artefacts by path and format, and scope the
   criterion to this merge** (addresses: forward-looking criterion) e.g. a
   one-node-type-per-line inventory file and a per-test baseline listing fixture-
   case identifiers, both committed as part of this child. Move the
   "before any deletion" ordering to Dependencies, where the ordering rule lives.
3. **Say what a user can reach after this merges** (addresses: the Summary
   contradiction) State that the real clients become resolvable by
   `accelerator work sync` while no skill invokes it until 0212 — or move the
   composition-root binding to the child that repoints the skills.
4. **Move the ADF, JQL and GraphQL construction criteria here** (addresses: the
   split gate) They pin this child's code; leave 0211 the CLI-surface and
   retirement criteria that match its thin-adapter role.
5. **Apply the minors** — restate the reuse-the-existing-gate clause, claim the
   two client crates' pup and public-API artefacts explicitly, require the
   copyleft answer to be the committed output of a named reproducible command,
   and name the provisioning owner directly.

## Per-Lens Summaries

- **Clarity**: precise sentence by sentence; the problems are at the seams — the
  Summary contradiction, and ownership of ADF/JQL/GraphQL and the pup artefacts
  stated in incompatible terms with 0211.
- **Completeness**: complete as a `task` — Summary, Context, Requirements,
  Acceptance Criteria, Dependencies, Assumptions, References all substantive.
  Two parent clauses were lost in transcription.
- **Dependency**: the strongest dependency record of the four — names the
  credentialed target and both external services, states they gate this child's
  own acceptance, and repeats the confirm-0194-by-artefact instruction.
- **Scope**: correctly sequenced ahead of every deletion and correctly made
  responsible for the oracles. Still story-scale despite `kind: task`.
- **Testability**: the tightest criteria set of the four, undermined by the two
  majors above.

---
*Review generated by /accelerator:review-work-item*

## Re-Review (Pass 2) — 2026-08-17

**Verdict:** REVISE

All five lenses re-ran over the four children as a set. **No criticals.** The
pass-1 critical is resolved and independently confirmed: the completeness lens
verified that every obligation the parent's Scope narrative and `## Decisions`
register still names resolves to a child requirement or criterion, including the
three port-less bridge capabilities now owned by 0212.

The verdict holds on major count. Three of the majors are defects the fix round
introduced; the rest are pre-existing gaps the tightened criteria exposed.

### Previously Identified Issues

- 🟡 → ✅ **Projection fidelity had no criterion beyond absent-description** —
  **Resolved**. The whole-corpus offline criterion now pins every record in
  `skills/work/scripts/test-fixtures/`, including Jira's key-sorted ADF ordering,
  with no network target.
- 🟡 → 🟡 **Oracle transcriptions named no artefact** — **Half-resolved, and now
  a defect of its own.** The requirement and criterion both promise all three
  land "each at a named committed path", but only one path is written down
  (`cli/jira-client/tests/fixtures/adf-node-types.txt`). Flagged independently by
  clarity, completeness and testability.
- 🟡 → ✅ **"Leaves the production path unchanged" contradiction** —
  **Resolved**. The Summary now states the user-visible delta is nothing, and why.
- 🟡 → ✅ **ADF↔markdown verified in the wrong child** — **Resolved**. The
  ADF/JQL/GraphQL assertions moved here, where the code lives.
- 🔵 → ✅ Minors on the reuse-the-gate clause, pup ownership, the copyleft
  command, the provisioning owner, and the sibling-dated criterion — **all
  resolved**.

### New Issues Introduced

- 🟡 **Testability / Clarity / Completeness**: Two of three transcriptions still
  have no path (introduced by the fix round). The requirement's own stated failure
  mode applies to its own bullets — "a transcription with no path is a criterion
  neither sibling can check" — and 0212's fixture-count and fixture-case criteria
  resolve to those files after the source directory is deleted.
- 🟡 **Testability**: No criterion covers HTTP-status or GraphQL-level error
  classification, or auth. Pre-existing. The four-`curl`-table criterion covers
  transport exit codes, and the item stresses those are *not* HTTP statuses —
  so nothing pins a 401, 404, 429 or a `200`-carrying-`errors` GraphQL body to a
  `TrackerError` class, and nothing pins auth-header construction. A client that
  misclassifies an auth failure as retryable passes every criterion.
- 🟡 **Testability**: The ADF criterion is scoped by an inventory this child
  authors, with no anchor to the bash source. Coverage is total by construction:
  an inventory omitting a node type yields a passing fixture set while that node
  type regresses at cutover — and the bash oracle dies in 0211.
- 🟡 **Scope**: The non-port provider surface is owned by neither this child nor
  0211 — see the cross-child note below.
- 🔵 **Testability**: The identifier-safety criterion asserts "no value is written
  to a work item's frontmatter", an observation point below the port where this
  child's crates write no files.
- 🔵 **Clarity**: `accelerator-work` is never tied to a crate directory, though
  siblings name `cli/work/`, `cli/work-cli/` and `cli/work-adapters/` distinctly
  and the pup/public-API criterion needs to know which.
- 🔵 **Clarity**: The projection recipe shifts between "summary line" and "title
  line", and describes output via the `jq -S` invocation being deleted; how the
  no-blank-line rule composes with Linear's absent-description golden "ending in
  an empty line" is left to inference.
- 🔵 **Suggestion**: The 1.35×T timeout window at T = 200ms leaves ~70ms of
  headroom — precise enough to flake under parallel CI load.

### Cross-Child

- 🟡 **Scope**: **The non-port provider surface is unowned.** This child scopes
  its crates to the four port operations and forbids `reqwest::` use outside them;
  0211 declares *thin* adapters exposing eight flows, of which `comment`,
  `transition`, provider `search`, `attach` and `init` have no port operation. Their
  request construction must therefore live in crates whose Requirements never asked
  for it. Either this child is materially larger than it states, or 0211 must extend
  and re-snapshot crates it does not own while calling itself thin.

### Assessment

The strongest child in the set, and the criteria that were rewritten did close
their holes — bar the transcription paths, which are a half-fix. The two
substantive gaps are pre-existing rather than introduced: error classification
beyond transport codes has never had a criterion, and the ADF inventory has no
anchor to the source it transcribes.

## Acceptance — 2026-08-17

**Accepted.** Verdict changed from REVISE to APPROVE by Toby Clemson, and the
target work item moved to `ready`.

The open findings recorded above were **accepted rather than resolved**. They are
not withdrawn and remain the record of what is known to be imperfect; they carry
into planning rather than blocking it. Specifically still open at acceptance:

- The three Open Questions on 0171 — the credentialed target's secrets siting, the
  fate of the three port-less bridge capabilities, and the `EXIT_CODES.md` siting —
  plus the two ⚠️ size-bounding assumptions in 0211 and 0212.
- Three self-contradictions introduced during the fix rounds: 0211's mock-server
  deletion being simultaneously unconditional and deferrable, 0211's `jq`/`curl`
  requirement asserting a survivor set its own criterion says the child cannot
  reach, and 0213's stub-on-`PATH` seam defeating the one predicate item that would
  catch a malformed `--resolve` template.
- Two latent gaps better closed by implementation than by further specification:
  the non-port provider surface (five of eight flows) being owned by neither 0210
  nor 0211, and 0210 carrying no criterion for HTTP-status or GraphQL error
  classification or auth.

Rationale for accepting rather than iterating: across four review passes the
severity ceiling fell (3 criticals → 1 → 0 → 0) while the major count did not
converge, and each fix round introduced two or three new majors of one shape — a
requirement updated without its criteria, or the reverse. The two correctness traps
that mattered (the unowned port-less capabilities, and the `work-item-sync-label.sh`
ordering break) are both closed. Further speculative specification was judged less
valuable than starting 0210 and discovering the real shape of the client crates.
