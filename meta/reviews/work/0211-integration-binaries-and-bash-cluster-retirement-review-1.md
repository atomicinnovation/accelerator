---
type: "work-item-review"
id: "0211-integration-binaries-and-bash-cluster-retirement-review-1"
title: "Work Item Review: Integration Binaries and Bash Cluster Retirement"
date: "2026-08-17T11:17:18+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
parent: "work-item:0171"
target: "work-item:0211"
work_item_id: "0211"
relates_to: ["work-item-review:0171-jira-and-linear-integrations-review-1"]
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: ["rust", "jira", "linear", "cli", "cutover", "registration"]
last_updated: "2026-08-17T12:40:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Integration Binaries and Bash Cluster Retirement

**Verdict:** REVISE

0211 attracted more findings than any other child. Three of them are structural
rather than editorial: its exit-code criterion measures the implementation
against a document the same child authors, so it can never fail; the per-flow
fixture *capture-source* recording was lost in the carve-out, so all sixteen
fixtures could be captured from the mock servers this child deletes and every
criterion would read green; and its `allowed-tools` equality criterion can only
be satisfied by 0212's work, inverting the declared order. It also blocks 0174
without declaring the edge, and the cross-cluster coupling analysis that justified
the 0210 → 0211 → 0212 order was done in one direction only.

This review was conducted as one pass over all four children of 0171. The
cross-child critical — the three port-less bridge capabilities landing in no
child — is stated in full in 0212's review.

### Findings

#### Major

- 🟡 **Testability**: The exit-code contract criterion is circular
  **Location**: Acceptance Criteria
  It asserts that every failure class maps to "the integer the named document of
  record specifies", while that document is a deliverable of this same child.
  Any integers the implementation emits can be written into the document, after
  which the table-driven test passes. The repointed `SKILL.md` bodies branch on
  these values, so a mapping that silently differs from the bash exit codes they
  previously branched on breaks skill behaviour with the criterion green. 0171's
  anchor — that these be pinned by CLI-level tests at the layer the skills invoke
  — was also dropped.

- 🟡 **Testability / Completeness / Dependency**: The per-flow fixture
  capture-source recording was lost
  **Location**: Acceptance Criteria
  0171 requires `## Decisions` to record, per flow, whether each fixture came
  from the credentialed target or the retiring mock server, naming the blocker
  where the real target was unreachable. 0211 keeps the per-flow assertion and
  drops the provenance obligation entirely, while 0171's `## Decisions` still
  lists that entry as *pending* with no child owning it. All sixteen fixtures
  could pin the new clients to a test double this child deletes.

- 🟡 **Dependency**: The `allowed-tools` equality criterion depends on 0212's
  outcome, inverting the declared order
  **Location**: Acceptance Criteria
  It asserts the surviving set contains "no jira, linear or work skill" — but the
  three work skills are backed by `skills/work/scripts/*.sh` and are repointed
  only in 0212, which this child blocks. Either 0211 cannot be accepted at its own
  merge boundary, or the criterion is a silent no-op.

- 🟡 **Dependency**: 0211 blocks 0174 but declares no edge
  **Location**: Frontmatter: blocks
  This child retires `_EXPECTED_INTEGRATIONS_SUITES` and seven of the eight
  orphaned `SHELL_LIBRARIES` entries that 0174 waits on, yet declares only
  `blocks: 0212`. 0174 can appear unblocked while a live integrations floor and
  seven orphaned entries still exist.

- 🟡 **Dependency**: Cross-cluster coupling was analysed in one direction only
  **Location**: Dependencies
  The ordering rationale covers jira/linear scripts calling
  `work-item-sync-label.sh`. The reverse coupling is unexamined: the work-item
  remote bridges and their three suites survive until 0212, and the two Python
  mock servers those suites may use are deleted *here*. If any surviving work
  script or suite reaches into the clusters or the mock servers, this child's
  blanket deletion breaks the still-live work-item bash path — the mirror image
  of the break the ordering was chosen to avoid, created rather than prevented
  by the chosen order.

- 🟡 **Clarity / Completeness**: "The four crates'" pup rules and public-API
  snapshots
  **Location**: Requirements
  This child introduces two crates; the other two are 0210's. Neither child
  states unambiguously where `jira-client` and `linear-client` get their pup
  rules and public-API snapshots, and "the four crates" has no antecedent here.

- 🟡 **Clarity**: ADF↔markdown, JQL and GraphQL construction — verified here,
  implemented in 0210, owned explicitly by neither
  **Location**: Requirements / Acceptance Criteria
  This child carries the only criterion for that behaviour while its Requirements
  never state it implements any of it. A 0210 implementer can read the conversion
  as in scope with no criterion; a 0211 implementer as already delivered.

- 🟡 **Clarity**: "Library entry" carries two meanings and the seven-versus-eight
  count cannot be reconciled
  **Location**: Requirements / Acceptance Criteria
  "All seven library entries" and "the seven jira and linear entries are gone
  from `SHELL_LIBRARIES`" sit beside a requirement to classify
  `linear-graphql.sh` as possibly "an eighth library entry". The term is used
  both for a sourced-only file on disk and for a frozenset member, and the
  outcome determines whether a lint guard passes.

- 🟡 **Testability**: The eight-flow enumeration's completeness has no coverage
  criterion
  **Location**: Acceptance Criteria
  Every behavioural criterion is scoped to "the eight flows" while the child's own
  ⚠️ assumption says that enumeration is unconfirmed against 34 production
  scripts — and another criterion empties both directories outright. A flow that
  exists in bash but is absent from the eight can be deleted with everything
  green.

- 🟡 **Scope**: Three separately deliverable concerns behind an "and" in the
  title
  **Location**: Summary / Requirements
  Sixteen subcommands with stdout goldens and an exit-code contract; end-to-end
  registration through 0165's signed manifest with 0203 as a conditional
  dependency; and two whole-cluster deletions. Only "capture goldens before
  deleting" couples them. Folding release-pipeline registration into a cutover
  child means a manifest or licence problem blocks the bash retirement.

#### Minor

- 🔵 **Dependency**: The fixture capture step may need the credentialed target,
  which this child's Dependencies never names.
  **Location**: Dependencies

- 🔵 **Dependency**: The conditional 0203 release-path dependency has no edge, no
  trigger mechanism and no owner for converting it when the copyleft check fires.
  **Location**: Dependencies

- 🔵 **Testability**: The stdout-golden criterion's "shape deliberately changed"
  branch has a judgement-based pass condition and requires no golden for the new
  shape, so any inconvenient subcommand can be routed down it.
  **Location**: Acceptance Criteria

- 🔵 **Clarity**: `search` names both a migrated flow here and the parent's
  still-undecided unkeyed-discovery capability, with no disambiguation.
  **Location**: Requirements

- 🔵 **Clarity**: ADF and JQL are used unexpanded in the child whose criteria pin
  their fidelity, though 0210 and the parent both expand them.
  **Location**: Context / Requirements

### Strengths

- ✅ The `allowed-tools` criterion is an equality assertion against a named
  survivor set, not a "no unexpected entries" formulation.
- ✅ Ownership of the mechanical surfaces is split explicitly and exhaustively —
  seven `SHELL_LIBRARIES` entries here, one in 0212; one suite floor each — with
  each child stating what it does *not* own.
- ✅ It carries a `mise run` green criterion scoped to its own merge boundary,
  "not only after the whole of 0171".
- ✅ It restates the ⚠️ size-bounding assumption it inherits, naming what happens
  to the child if it fails to hold.
- ✅ The `work-item-sync-label.sh` ordering rationale is stated from this side as
  well as 0212's, in mutually consistent terms.

### Recommended Changes

1. **Anchor the exit-code mapping externally** (addresses: the circular
   criterion) Require each class's integer to equal the value the retiring bash
   flow returned for the same condition, captured pre-deletion, or to reuse
   `tracker`'s existing `E_DISPATCH_*` integers — and restore the CLI-level
   pinning at the layer the skills invoke.
2. **Restore the capture-source clause** (addresses: lost provenance) Per flow,
   record provenance in 0171's `## Decisions` as credentialed-target or
   mock-served, with each mock-served entry naming the blocker.
3. **Move the whole-repository `allowed-tools` equality to 0212** (addresses: the
   inverted dependency) Leave 0211 the jira and linear subset plus a recorded
   enumeration of residual declarers.
4. **Add `blocks: 0174`** and a Dependencies bullet naming what 0174 waits on
   from this child (addresses: the missing edge).
5. **Sweep the reverse cross-cluster coupling before deleting** (addresses:
   one-directional analysis) Enumerate every reference from
   `skills/work/scripts/` into the two clusters and into the mock servers, with
   the recorded result; where references exist, either keep the work path green
   explicitly or move the mock-server deletion to 0212.
6. **Add a flow-coverage criterion** (addresses: enumeration completeness) Each
   of the 22 Jira and 12 Linear production scripts maps to a named subcommand or
   a recorded internal-helper classification, with the count reconciling to the
   pre-deletion file list.
7. **Resolve the ownership and terminology defects** — name the four crates and
   split the pup/public-API claim with 0210, state which child implements
   ADF/JQL/GraphQL, distinguish sourced-only library file from `SHELL_LIBRARIES`
   member and state the expected count under each `linear-graphql.sh` branch,
   disambiguate `search`, and expand both acronyms on first use.
8. **Consider splitting the child** (addresses: the bundle) A "binaries and
   registration" child blocking a "cluster retirement" child, or fold retirement
   into per-provider children if the provider seam is cut.

## Per-Lens Summaries

- **Clarity**: individually precise, but ownership of three shared obligations
  (pup artefacts, ADF/JQL/GraphQL, `search`) is stated in terms incompatible with
  0210 or the parent.
- **Completeness**: complete as a `task`, bar the lost capture-source clause.
- **Dependency**: two real graph defects (the missing 0174 edge, the inverted
  `allowed-tools` criterion) plus the one-directional cluster analysis.
- **Scope**: the largest child and the most easily reduced; three concerns behind
  an "and".
- **Testability**: two criteria that cannot fail as written, and a coverage
  measure scoped to an unconfirmed enumeration.

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

- 🟡 → 🟡 **Exit-code criterion was circular** — **Improved, not closed.** The
  anchor now points at "the value the retiring bash flow returned … captured
  pre-deletion", but no criterion requires that capture to be committed, and the
  free choice between it and `tracker`'s `E_DISPATCH_*` value (qualified only by
  "genuinely the same taxonomy") restores the unfalsifiability. Flagged by both
  clarity and testability.
- 🟡 → ✅ **Per-flow fixture provenance was lost** — **Resolved**. Provenance is
  recorded per flow, each mock-served entry naming its blocker.
- 🟡 → ✅ **`allowed-tools` equality depended on 0212's work** — **Resolved** by
  moving the equality assertion to 0212 — but the Requirements bullet was not
  updated to match, so the two now contradict each other (below).
- 🟡 → ✅ **Blocked 0174 without declaring the edge** — **Resolved**; `blocks`
  names it and 0174's `blocked_by` reciprocates.
- 🟡 → 🟡 **Cross-cluster coupling analysed one direction only** — **Addressed but
  self-contradictory.** The reverse sweep is now a requirement, but its deferral
  branch collides with two of this child's own criteria (below).
- 🟡 → ✅ **"The four crates'" pup artefacts** — **Resolved**; two binary crates
  here, two client crates in 0210, stated on both sides.
- 🟡 → ✅ **ADF ownership ambiguity** — **Resolved**; the assertions moved to 0210.
- 🟡 → ✅ **"Library entry" two senses** — **Resolved** in the `SHELL_LIBRARIES`
  requirement, though Context and the deletion bullet were not updated to match
  (below).
- 🟡 → ✅ **Flow enumeration had no coverage criterion** — **Resolved in form**,
  but the new criterion has an unconstrained escape (below).
- 🟡 → 🟡 **Three concerns behind an "and"** — **Still present.** Declined at the
  set level; this child remains the heaviest.
- 🔵 → ✅ Acronyms, `search` disambiguation, the 0203 trigger, the stdout escape
  branch — **all resolved**. The stdout branch now demands a new golden plus a
  recorded diff.

### New Issues Introduced

- 🟡 **Clarity / Dependency / Testability**: **The mock-server deletion is
  simultaneously unconditional and deferrable** (introduced by the fix round).
  The reverse-sweep requirement permits deferring mock-server deletion to 0212;
  two criteria here assert the servers do not exist, and 0212 carries no
  requirement or criterion mentioning them. If the sweep finds references — the
  outcome it exists to detect — the permitted resolution makes two of this child's
  own criteria unpassable, and the deletion lands in no child.
- 🟡 **Clarity**: **The `jq`/`curl` requirement asserts a survivor set its own
  criterion says this child cannot reach** (introduced by the fix round). The
  requirement still expects "no jira, linear or work skill among them"; at this
  child's boundary the three work skills have not been repointed, so they must
  still declare `jq`/`curl`.
- 🟡 **Clarity**: **Context settles the `linear-graphql.sh` partition the
  Requirements leave open.** Context states "12 production scripts plus
  `linear-common` and `linear-auth`" as exhaustive, while the requirement makes the
  classification an open decision that would change both counts — and the
  flow-coverage criterion asserts "the 22 Jira and 12 Linear production scripts"
  as a fixed denominator.
- 🟡 **Testability**: **The flow-coverage criterion is total by construction.**
  Any script not mapping to one of the eight flows can be declared an
  internal helper by fiat, and the count still reconciles — so the case it exists
  to catch is the case it cannot catch.
- 🟡 **Testability / Clarity**: The exit-code anchor names two authorities with no
  precedence rule, and never requires the bash capture to be committed.
- 🟡 **Dependency**: **0165 is treated as an existing artefact with no
  confirmation instruction**, unlike 0194. 0165's record shows the same status
  divergence (frontmatter `done` against a body reading In Progress, no criteria
  ticked) *and* names an undischarged privileged prerequisite — a minisign secret
  key installed by a repository administrator. The registration criterion could be
  undischargeable for reasons outside this change.
- 🔵 **Dependency**: No External systems entry, though its fixtures are captured
  against the real providers "wherever reachable" — the one child whose work is
  provider-by-provider records no availability coupling.
- 🔵 **Completeness**: No before-pickup marker for the exit-code contract, which
  the parent's register still lists as *open*; siblings carry such markers.
- 🔵 **Testability**: The reverse sweep names no search procedure, so
  enumeration completeness is unverifiable — unlike 0212's sweep, which names its
  grep and records the command.
- 🔵 **Testability**: "At minimum" leaves the failure-class set open, so per-class
  coverage is satisfied by the five-class floor regardless of what the binaries
  can actually exit on.
- 🔵 **Clarity**: "Mock server" denotes both `wiremock` and the Python servers in
  adjacent sentences.

### Assessment

This child absorbed the most fixes and now carries the most residue. Two of its
three new contradictions were introduced by the fix round — the mock-server
deferral and the `jq`/`curl` survivor set — and both are the same shape: a
requirement updated without its criteria, or vice versa. The exit-code anchor
moved the hole rather than closing it: anchoring to an uncommitted capture is not
an anchor.

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

## Correction — 2026-08-17

**A premise this review relied on was false, and the finding built on it is
withdrawn.**

Both the pass-1 and pass-2 reviews treated `linear-create-flow.sh:304` and
`jira-resolve-fields.sh:140` as **live callers** of `work-item-sync-label.sh`, and
the pass-3 dependency finding on the parent described them the same way. They are
not callers. Both lines are comments:

```bash
# (Same normalisation as the Jira guard and work-item-sync-label.sh.)
# guard and work-item-sync-label.sh).
```

Verified by grepping `skills/integrations/*/scripts/*.sh` for invocations of
`work-item-*.sh` with comment lines excluded: no matches. The clusters invoke no
work script at all.

What the coupling actually is, and it runs the other way: **four** of the five
suites 0212 deletes — `test-work-item-create-remote.sh`, `-update-remote.sh`,
`-fetch-remote.sh` and `-sync-apply.sh` — resolve paths into
`skills/integrations/{jira,linear}/scripts/test-helpers/` for the Python mock
servers and `.../test-fixtures/` for their scenario fixtures.

Consequences for this review:

- The **0211-before-0212 ordering it endorsed was wrong.** In that order, 0211's
  wholesale cluster deletion would have broken all four suites and the
  `_EXPECTED_WORK_SUITES` floor at 0211's own merge boundary. The order is now
  0210 → 0212 → 0211.
- The pass-2 major *"the mock-server deferral branch has no receiving requirement
  in 0212 and contradicts 0211's own criteria"* is **resolved by the reordering
  rather than by the patch this review recommended**. With the work suites deleted
  first, nothing outside the clusters consumes their test assets, so 0211 deletes
  the mock servers unconditionally as its criteria always stated. The deferral
  branch has been removed from 0211 entirely; no conditional criteria were added
  to either child.
- The pass-2 major on 0211's `jq`/`curl` survivor set is **resolved as a side
  effect**: with 0211 landing last, the work skills are already repointed at its
  boundary, so its expectation of no surviving work-skill declarer is correct as
  written. 0211 now owns the whole-repository equality assertion and 0212 asserts
  only that no work skill declares `jq` or `curl`.
- The reverse-coupling sweep 0211 gained in the previous round survives in reduced
  form: a confirmation at its boundary that nothing outside the clusters still
  references `test-helpers/` or `test-fixtures/`, expected to be empty.

The reviews' strengths sections credit the 0211-before-0212 ordering as
"justified by a concrete, named break". That credit was misplaced — the break was
named but not verified, and verifying it would have inverted the order three
passes earlier.
