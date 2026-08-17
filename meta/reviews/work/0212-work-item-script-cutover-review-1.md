---
type: work-item-review
id: "0212-work-item-script-cutover-review-1"
title: "Work Item Review: Work-Item Script Cutover"
date: "2026-08-17T11:17:18+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
parent: "work-item:0171"
target: "work-item:0212"
work_item_id: "0212"
relates_to: ["work-item-review:0171-jira-and-linear-integrations-review-1"]
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 1
review_pass: 2
tags: [rust, cutover, work-items, fixtures]
last_updated: "2026-08-17T12:40:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: Work-Item Script Cutover

**Verdict:** REVISE

0212's deletion criteria are the most mechanically checkable in the set — `ls`
matching nothing, a repository-wide grep with the command and empty output as the
recorded result, relocated-count-plus-deletions equalling the pre-change file
count. It carries one critical, found independently by three lenses: the three
port-less bridge capabilities that 0171 spent a requirement, two criteria and an
Open Question on land in no child, and this is the child that deletes the scripts
carrying them. Secondarily, it inherits two live-tracker criteria without
inheriting the credentialed-target prerequisite that gates them, so it can be
scheduled as needing nothing external and then strand mid-cutover.

### Findings

#### Critical

- 🔴 **Completeness / Scope / Testability**: The three port-less bridge
  capabilities are owned by no child, and this child deletes the scripts carrying
  them
  **Location**: Requirements / Acceptance Criteria
  0171 carries a requirement, two acceptance criteria, one of three
  pickup-blocking Open Questions and three *open* `## Decisions` entries covering
  the fate of the unkeyed discovery `search` mode of `work-item-fetch-remote.sh`,
  the create bridge's `--dry-run` field-resolution preview, and the update
  bridge's `--dry-run` payload validation. No child mentions any of them: 0210
  addresses only the out-of-scope branch, 0211's `search` is the standalone
  provider search flow rather than the unkeyed discovery mode, 0213 is unrelated,
  and this child deletes `work-item-fetch-remote.sh` and both remote bridges
  wholesale with no requirement to re-site, drop or replace the behaviour.

  Verified directly: grepping the four children for `dry-run`, `unkeyed` and
  `port-less` returns nothing, while 0171 references them ten times.

  **Impact**: If the children are the acceptance gates, every one of the four can
  pass green while `/sync-work-items` silently loses its ability to list remote
  issues with no local work item and `/sync-work-items --preview` loses live push
  validation — the exact user-visible regression 0171 warns against. The Open
  Question also now has no child in which it can be discharged. This is the same
  partition failure that caused the superseded `## Increments` section to be
  replaced.

#### Major

- 🟡 **Dependency / Testability / Clarity**: Two live-tracker criteria with no
  credentialed-target dependency, and "the scratch project and team" has no
  antecedent
  **Location**: Dependencies / Acceptance Criteria
  The corpus criterion creates "one remote issue per relocated corpus record on
  the scratch project and team" and runs sync "through the real clients", yet
  Dependencies lists only 0210, 0211 and 0194 — no credentialed target, no Jira
  REST or Linear GraphQL entry, unlike 0210 which spells both out. The child that
  performs the irreversible deletions carries an unsatisfiable-without-
  provisioning criterion invisible in its own dependency record.

- 🟡 **Dependency**: 0212 → 0174 is a dangling half-edge
  **Location**: Frontmatter: blocks
  This child declares `blocks: 0174`, but 0174's `blocked_by` names 0171, not
  0212, while 0171 still declares `blocks: 0174` too. The edge now exists in
  three places with two different tails and no reciprocal entry for the new one.
  Anyone resolving blockers from 0174 lands on a parent that performs no work
  directly.

- 🟡 **Testability**: The dirty-guard and corpus criteria name no procedure
  **Location**: Acceptance Criteria
  "A work item carrying an uncommitted edit is not overwritten by a pull that
  would otherwise apply, with the evidence location recorded" names no harness,
  no way to stage such a pull, and no automated home. A verifier cannot
  distinguish a genuine failure from an unprovisioned environment, so both slide
  into recorded evidence of an unrepeatable run.

- 🟡 **Clarity / Testability**: "0210's recorded baseline" is ambiguous between
  the assertion count and the fixture-case list
  **Location**: Acceptance Criteria
  0210 records both, and this child dropped 0171's disambiguating clause that
  "the recorded assertion count is context for that comparison, not the bar". A
  reviewer can hold a faithful pure-Rust rewrite to a count it need not match.

#### Minor

- 🔵 **Testability**: The fixture-relocation criterion compares against "the
  pre-change file count under that directory" — a number that exists nowhere once
  the directory is gone, recoverable only by VCS archaeology.
  **Location**: Acceptance Criteria

- 🔵 **Testability**: The `EXIT_CODES.md` "rewritten for the Rust surface" branch
  has no check that the documented codes match what the CLI emits.
  **Location**: Acceptance Criteria

- 🔵 **Dependency**: The shared-file coupling on `sync-work-items/SKILL.md` is
  recorded only on 0213's side, though this child sits at the end of the chain
  and is more likely to land second.
  **Location**: Dependencies

- 🔵 **Clarity**: Several criteria say evidence is "recorded" without naming the
  record, inconsistently with sibling criteria that name 0171's `## Decisions`
  explicitly.
  **Location**: Acceptance Criteria

### Strengths

- ✅ The deletion criteria are mechanically checkable end to end: `ls` matching
  nothing, a repository-wide grep excluding `meta/` with the command and empty
  output as the recorded result, and count arithmetic on relocated fixtures.
- ✅ The consumer sweep explicitly covers `hooks/`, `templates/`, `docs-site/`,
  `tasks/` and agent definitions rather than only `skills/`.
- ✅ It labels its half of the absent-description guarantee and points at 0210
  for the other half, so neither reader mistakes a half for the whole.
- ✅ The dirty guard's two-part verification separates the static call-site check
  from the behavioural one.
- ✅ It restates the ⚠️ size-bounding assumption it inherits and names the
  consequence — that a missing Rust replacement turns deletion into new
  behaviour.
- ✅ It repeats the confirm-0194-by-artefact instruction rather than trusting the
  parent's judgement.

### Recommended Changes

1. **Assign the three port-less capabilities here** (addresses: the critical)
   Add 0171's fate-decision requirement and both verification criteria verbatim,
   including the *drop* branch's obligation to state the replacement outcome in
   observable terms and verify that outcome in place of the original behaviour.
   Size the child for it, and update 0171's `## Decomposition` table so the
   partition is provably total against the parent's Requirements. If the
   re-siting is large enough to unbalance this child, carve a fifth child
   blocking it instead.
2. **Name the credentialed target and both external services in Dependencies**
   (addresses: the invisible gate) With the same explicitness 0210 uses, saying
   which criterion each gates, and introduce "the scratch project and team" as a
   defined referent inside this child.
3. **Settle the 0174 edge convention** (addresses: the half-edge) Either move it
   to the children — adding 0211 and 0212 to 0174's `blocked_by` and recording in
   0171 that the edge is discharged by its children — or keep it at parent level
   and drop it from 0212.
4. **Specify the dirty-guard check as a named test** (addresses: no procedure) A
   fixture work item with an uncommitted edit plus a remote-modified counterpart,
   asserting the file's bytes are unchanged and the refusal diagnostic emitted,
   rather than recorded evidence.
5. **Restore the assertion-count-is-context clause**, have 0210's baseline record
   the pre-change fixture file count as a committed number, pin the rewritten
   `EXIT_CODES.md` branch with a test asserting each documented integer against
   the CLI's emitted value, add 0213 to `relates_to` with the shared-file note,
   and name the record location in every criterion that requires recording.

## Per-Lens Summaries

- **Clarity**: precise on the deletion inventory; the defects are an undefined
  referent ("the scratch project and team"), an ambiguous baseline, and
  inconsistent "recorded" phrasing.
- **Completeness**: complete as a `task`, and explicit about what it does not own
  — but the port-less capabilities were dropped in the carve-out.
- **Dependency**: correctly blocked by 0210 and 0211, with the sync-label
  rationale stated; missing the credentialed target and carrying a dangling 0174
  half-edge.
- **Scope**: coherent as the cutover unit, still story-scale despite
  `kind: task`.
- **Testability**: the strongest deletion criteria in the set, weakest on the two
  behavioural checks that need a live tracker or a staged pull.

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

- 🔴 → ✅ **The three port-less bridge capabilities were owned by no child** —
  **Resolved.** They are now a Requirements bullet here with two dedicated
  criteria, one for the recorded fate and one demanding the behaviour be verified
  against whichever option was taken. Independently confirmed by the completeness
  lens against the parent's register.
- 🟡 → ✅ **Two live-tracker criteria with no credentialed-target dependency** —
  **Resolved**; the prerequisite and both external systems are named, with the
  owner and the criterion each gates.
- 🟡 → ✅ **0174 dangling half-edge** — **Resolved**; the edge sits at child level
  and 0174's `blocked_by` reciprocates. (0174's *prose* still names 0171 — see
  cross-child.)
- 🟡 → ✅ **Dirty-guard criterion named no procedure** — **Resolved**, and now
  exemplary: a static half plus a named automated test asserting the file's bytes
  unchanged and the refusal diagnostic emitted, with recorded manual evidence
  explicitly ruled out.
- 🟡 → ✅ **"0210's recorded baseline" ambiguity** — **Resolved**; the fixture-case
  list is the bar and the assertion count is context.
- 🔵 → ✅ The pre-change file count, the `EXIT_CODES.md` pin, the 0213 shared-file
  edge, and the record locations — **resolved**, bar the branch asymmetry below.

### New Issues Introduced

- 🟡 **Completeness**: **The two `--dry-run` capabilities are specified more
  thinly than the discovery-`search` one beside them** (introduced by the fix
  round). `search` names its carrier script, its consumer and why `fetch_all`
  cannot express it; the two `--dry-run` capabilities name neither carrier script
  nor flag, the create one is attributed to two different callers in two places
  (the confirm gate in the requirement, `--preview` in the criterion), and the
  criterion requires "each emitting its named diagnostic" while no diagnostic is
  named anywhere.
- 🟡 **Scope**: **This child now bundles a mechanical deletion cutover with three
  unsettled net-new re-sitings.** Every other obligation here is migration
  bookkeeping resting on the assumption that the Rust surface already covers the
  deleted behaviour. The three capabilities are different in kind: fates still
  `open`, and one permitted fate is net-new design of unknown size. The child
  performing the irreversible deletions now has a size unbounded by three
  undecided questions.
- 🟡 **Testability**: **The `create-work-item` and `list-work-items` repoints have
  only a static criterion**, while this change deletes the five
  `test-work-item-*.sh` suites and the work suite floor — the only executable
  checks on them. A repointed invocation with a wrong subcommand name, flag or
  argument passes everything, `mise run` included.
- 🔵 **Scope**: The Summary omits both obligations that most recently moved in —
  the port-less capabilities and the repository-wide `jq`/`curl` equality
  assertion.
- 🔵 **Scope**: One of 0210's three transcriptions (the parity-test baseline) is
  destroyed by *this* child, in the same change that consumes it, so it could sit
  here instead and be closed by inspecting one change.
- 🔵 **Testability**: The preferred `EXIT_CODES.md` branch carries the weaker gate
  — folding into the CLI docs drops the doc-versus-emitted-value test that the
  non-preferred branch requires.
- 🔵 **Testability**: The drop branch of the port-less criterion verifies a
  replacement outcome defined by the same decision under test.
- 🔵 **Completeness**: Alone among the children that consume 0194, this one
  carries no confirm-the-artefacts-not-the-status instruction — and it is the
  child whose deletions are irreversible.
- 🔵 **Clarity**: The corpus criterion points at a project and team "named in
  Dependencies", which describes them generically without naming either.
- 🔵 **Clarity**: The "preserve the guard" requirement also carries an unrelated
  normalisation repoint, outside its own stated rationale.

### Assessment

The pass-1 critical is genuinely closed and the dirty-guard criterion is now the
best-specified behavioural check in the set. The cost of closing the critical is
that this child inherited three undecided capability fates, which the scope lens
reads as unbounding its size — worth weighing against carving them into their own
child that blocks this one.

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
