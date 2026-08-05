---
type: work-item-review
id: "0169-vcs-subdomain-and-hooks-migration-review-3"
title: "Work Item Review: VCS Subdomain and Hooks Migration"
date: "2026-07-31T11:04:14+00:00"
author: Toby Clemson
producer: review-work-item
status: complete
target: "work-item:0169"
parent: "work-item:0136"
relates_to: ["work-item-review:0169-vcs-subdomain-and-hooks-migration-review-2"]
work_item_id: "0169"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [clarity, completeness, dependency, scope, testability]
review_number: 3
review_pass: 2
tags: [rust, vcs, hooks, migration]
last_updated: "2026-07-31T11:10:05+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Work Item Review: VCS Subdomain and Hooks Migration

**Verdict:** APPROVE *(changed from REVISE on 2026-07-31 — see Verdict Change)*

First review of the **reduced** 0169, after the 2026-07-31 split extracted 0185,
0186, 0187 and 0188. **13 majors and 1 critical**, against review-2 pass 4's 14
majors and none.

**The split did not reduce the finding count.** That was the explicit prediction
behind the recommendation to split, and the measurement falsifies it.

| | Majors | Criticals |
| --- | --- | --- |
| review-2 pass 1 | 19 | 0 |
| review-2 pass 2 | 15 | 1 |
| review-2 pass 3 | 14 | 0 |
| review-2 pass 4 | 14 | 0 |
| **review-3 (reduced story)** | **13** | **1** |

### What the split did change

Scope's findings changed *character* entirely. "Six workstreams", "four
toolchains", "the library swap is separable", "review-1 approved a different
bundle" — all resolved. In their place are two finer-grained findings that could
not have been seen before: a **release boundary bisects the story** (the
`hooks.json` rewrite cannot ship before a release listing `accelerator-vcs`),
and the **hooks migration and skill repoint are two independently deliverable
consumer threads**. Dependency independently reached the release-cut finding.

So the split bought real things — independent delivery, three sibling stories
unblocked by 0187, the pre-1.0 `jj-lib` bet isolated in 0188 — but it did not
buy review convergence.

### The critical

🔴 **Testability — the `.git`-as-file case has a self-contradictory oracle.**
The correction is declared a *departure* from shell behaviour, while the
verification oracle is a golden *captured from the shell*, and the fixture list
explicitly includes "`.git`-as-file colocated" among the captured goldens. A
verifier following the criteria literally compares corrected output against buggy
expectations and fails the case the story exists to fix.

This is the clearest instance of the pattern: it is the collision of two
individually-correct fixes made in different passes — "capture before delete"
(pass 3) and "the correction applies to all four subcommands" (pass 4).

### Cross-cutting themes

- **The validate-plan motivation is contradicted by the parity requirement**
  (clarity, completeness, testability — three lenses). Context names
  `validate-plan` being blocked in pure-jj repos as a motivating symptom; the
  guard's blocked set keeps `log` and `diff`, and the decision table reproduces
  it. Passing every criterion leaves the symptom intact. The motivation was added
  in response to a pass-3 completeness suggestion, without checking it against
  the parity requirement.
- **The output contract is still contradictory** (clarity, testability), in the
  section titled "Output contract, stated once". "No context to report → zero
  bytes" overlaps "adapter failure → `systemMessage` object" on the scenario the
  degraded-SessionStart criterion names. Third consecutive review to find this,
  each time narrower.
- **The release cut has no owner or gate** (scope, dependency).

### Other majors

- 🟡 **Testability**: no criterion pins the corrected `.git`-as-file behaviour for
  `vcs detect` specifically — three of four subcommands get an enumerated case.
- 🟡 **Testability**: submodule, bare and `GIT_DIR` handling is required but no
  criterion exercises it — they are inputs, not arms, so an arm-covering fixture
  need never construct them.
- 🟡 **Clarity**: the `.git` correction never states the corrected classification
  — "the new value" is never written down.
- 🟡 **Dependency**: 0182 is a stated prerequisite recorded only in
  `relates_to`; 0172's blocking edge is established only at *this* story's
  acceptance, so nothing stops 0172 landing concurrently and removing the suite
  this story's floor criterion depends on.
- 🟡 **Scope**: the release boundary; the two consumer threads.
- 🟡 **Completeness**: the validate-plan gap (above).

### Strengths

- ✅ Terminology, the authoritative guard-input enumeration, and the derivable
  136-row table are all credited by multiple lenses as removing whole classes of
  ambiguity.
- ✅ Anti-vacuity clauses are working: the closed mask set, the closed taxonomy
  assertion, the ban on permission-based fault injection, and the
  fixtures-precede-deletion commit ordering are each called out as unusually
  rigorous.
- ✅ Dependencies distinguishes blocked-by from completed-but-unlisted from
  in-flight, and names the debt the story creates rather than hiding it.
- ✅ Requirements and criteria map nearly one-to-one; scope reports no
  section-to-section drift.

### Assessment

Five review rounds have produced genuinely valuable findings — the floor-check
critical, the fail-open posture, the `corpus-adapters` revert, the split itself.
But the count has not fallen: 19 → 15 → 14 → 14 → 13, with a critical returning.

The size hypothesis is falsified. The residual cause is visible in the
composition: nearly every finding is a contradiction between sections edited at
different times, and the two longest-lived defects (the output contract, the
`.git` oracle) are collisions between fixes that were each correct in isolation.
That is a property of iteratively patching a prose specification, not of how much
the specification covers.

**Recommendation: stop reviewing and start planning.** A plan cannot be written
against a contradiction — the `.git` oracle and the output contract must both
resolve to concrete values before any test can be authored, and planning forces
that in a way another review round demonstrably has not. Take the critical and
the three cross-cutting themes into the plan as the first questions to settle;
leave the remaining majors to be resolved as the plan makes them concrete.

---
*Review generated by /accelerator:review-work-item*


## Verdict Change (Pass 2) — 2026-07-31

**Verdict:** REVISE → **APPROVE** (author decision).

The critical and the three cross-cutting themes were fixed; five majors are
**accepted rather than resolved**. Recorded so the approval is not read as "all
findings addressed".

### Fixed — the critical and 8 of 13 majors

- 🔴 **`.git`-as-file oracle contradiction.** Resolved — and the fix corrected an
  error in the work item first. The story claimed the correction "applies to all
  four subcommands", but `vcs-status.sh:9` and `vcs-log.sh:9` branch on
  `-d "$REPO_ROOT/.jj"` and never inspect `.git`; only `vcs-detect.sh:29` and
  `vcs-guard.sh:77` are affected. Narrowing the scope removes the
  captured-from-shell golden for a corrected case in status/log entirely. For the
  two affected subcommands the correction is now stated **as concrete values**
  (`classify_checkout` `main`→`colocated`; detect mode `jj`→`jj-colocated`; guard
  blocks→warns), and the detect fixture's golden is authored as the new
  expectation and marked a deliberate divergence.
- 🟡 **Output contract** (clarity, testability). The two outcomes are now disjoint
  *by definition* — success-with-nothing-to-report → zero bytes; adapter failure →
  exactly one `systemMessage` object. No run is both, so the overlap that survived
  three prior fixes is structurally impossible rather than reworded. Both are
  pinned separately and the guard's failure output is covered.
- 🟡 **validate-plan motivation** (clarity, completeness, testability). Stated as
  deliberately out of scope, with the reason — changing the blocked set is a
  user-facing policy decision that should not ride inside a parity-argued
  migration — and a follow-up item the implementer creates.
- 🟡 **Release cut** (scope, dependency). Now a named Dependencies entry with an
  owner, plus an acceptance criterion gating the rewrite on a *published*
  manifest rather than the locally generated one.
- 🟡 **`.git` correction lacked a stated value** (clarity) and **no criterion
  pinned it for `vcs detect`** (testability). Both closed by the above.

### Accepted, not resolved — 5 majors

- 🟡 **Scope**: the release boundary still bisects the story, and the hooks
  migration and skill repoint remain two independently deliverable consumer
  threads. A further split was declined; the boundary is at least now gated by a
  criterion rather than only narrated.
- 🟡 **Dependency**: 0182 remains a `relates_to` edge despite being a stated
  prerequisite, and 0172's blocking edge is still established only at *this*
  story's acceptance — so nothing structurally prevents 0172 landing concurrently
  and removing the suite this story's floor criterion depends on.
- 🟡 **Testability**: submodule, bare and `GIT_DIR` handling is required by the
  taxonomy but no criterion exercises it — they are inputs feeding the arms, not
  arms themselves, so an arm-covering fixture need never construct them. **This is
  the cheapest outstanding gap and the likeliest to bite**, since those are
  precisely the cases where a library-backed adapter diverges from the shell probe
  layer.

### Standing note

Five review rounds (review-2 passes 1-4, review-3) moved the major count
19 → 15 → 14 → 14 → 13. The split changed the *character* of the findings without
reducing the count, and the residual cause — contradictions between sections
edited at different times — is a property of iteratively patching a prose
specification. This approval reflects a judgement that the remaining findings are
better resolved by planning and implementation, which cannot proceed against a
contradiction, than by further review rounds.
