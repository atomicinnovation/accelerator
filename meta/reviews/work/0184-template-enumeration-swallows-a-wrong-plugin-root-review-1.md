---
type: "work-item-review"
id: "0184-template-enumeration-swallows-a-wrong-plugin-root-review-1"
title: "Work Item Review: Template resolution succeeds silently on a plugin root that is not an installation"
date: "2026-09-08T19:34:01+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0184"
work_item_id: "0184"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 4
tags: []
last_updated: "2026-09-08T22:25:16+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: Template resolution succeeds silently on a plugin root that is not an installation

**Verdict:** REVISE

For a low-priority bug, 0184 is unusually well-specified: it quotes the offending
code, distinguishes the *unknown* root (`None`) from the *known-and-wrong* root
(`Some(path)`), anchors every symbol with a `store.rs` file:line reference, and
frames most acceptance criteria as observable CLI outcomes with an explicit
contrast to today's behaviour. The verdict is driven by one issue seen through
four lenses — the `eject` site is mandated in Requirements yet its intended
behaviour is an unresolved Open Question and no acceptance criterion covers it —
compounded by a "wrong root" referent that silently narrows between the Summary
and the Requirements, and a "three" count that names two different sets. These
are correctness-of-specification defects, not polish: an implementer cannot
currently tell what "done" means for `eject`, nor whether the fix is meant to
catch every non-installation root or only the no-`templates/` subset.

### Cross-Cutting Themes

- **The `eject` site is under-specified** (flagged by: clarity, completeness,
  scope, testability) — Requirement 2 commits `eject` to raising
  `PluginRootUnavailable` on a wrong root, while Open Question 2 says exactly that
  is undecided. No acceptance criterion exercises `eject`, so one of the three
  mandated change sites has neither a settled directive nor a pass/fail check.
  This single unresolved decision is the dominant reason for the REVISE.
- **"Wrong root" narrows between Summary and Requirements** (flagged by: clarity,
  testability) — the Summary defines a wrong root as "present, but not an
  Accelerator installation", but every requirement and criterion detects only the
  narrower "root carries no `templates/` directory". A mis-pointed root that
  happens to contain an unrelated `templates/` directory is a wrong root by the
  Summary's wording yet is caught by nothing, so passing all criteria would not
  confirm the stated intent.
- **The load-bearing `templates/` assumption is prose-only** (flagged by:
  completeness, dependency) — the entire `NotFound → PluginRootUnavailable`
  mapping rests on "every installation ships a `templates/` directory", which the
  item itself flags as needing confirmation against the release artifact, but that
  check is not tracked as a precondition or acceptance step.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Clarity / Completeness / Testability / Scope**: `eject` site is mandated
  but its behaviour is undecided and untested
  **Location**: Requirements / Open Questions / Acceptance Criteria
  Requirement 2 directs the fix at "all three sites — `resolve_template`'s
  plugin-default tier, and `plugin_template_path`'s callers `plugin_default` and
  `eject`", committing `eject` to refuse on a wrong root; Open Question 2 then
  asks whether `eject` should refuse at all or whether `EjectOutcome::NoDefault`
  is legitimate, "left open". No acceptance criterion exercises `eject`, so its
  "done" is undefined and can be claimed either way. Resolve the question, then
  either add a criterion pinning eject's wrong-root outcome or scope `eject` out
  of Requirements until the decision lands.

- 🟡 **Testability / Clarity**: Criteria verify "no `templates/` directory",
  narrower than the Summary's "wrong root" intent
  **Location**: Summary / Acceptance Criteria
  Every criterion operationalises a wrong root as one lacking a `templates/`
  directory (AC1 explicitly; AC2's "a wrong root" only refuses under that same
  condition per the Assumption). A wrong root containing an unrelated
  `templates/` directory — a plausible mis-pointed root — is covered by no
  criterion and, by the design's own discriminator, would still succeed silently
  as a template-not-found: the exact failure mode the item exists to close.
  Either tighten the Summary to "a root lacking a `templates/` directory", or add
  a criterion for a wrong root that retains a `templates/` directory.

- 🟡 **Clarity**: "Three" refers to two different, only-partly-overlapping sets
  **Location**: Summary / Requirements
  The Summary names "three plugin-root template accessors" (`template_names`,
  `resolve_template`'s tier, `plugin_template_path`); Requirement 2's "all three
  sites" is a different triple (`resolve_template`'s tier, `plugin_default`,
  `eject`) sharing only one member. A reader mapping "all three sites" onto the
  Summary's "three accessors" will mis-scope the change. Use one consistent
  vocabulary and state how many of each — accessors versus application sites —
  there are.

#### Minor

- 🔵 **Scope**: Extended scope bundles two independently-deliverable fixes the
  item itself flags as splittable
  **Location**: Drafting Notes
  The "Option B" enlargement added `resolve_template`/`plugin_template_path` to
  the original `template_names`-only bug, and the Drafting Notes concede "a
  reviewer may still prefer to split" that work out. The two slices
  (`config templates list` versus `config template <name>`/eject) could each be
  delivered and rolled back independently. Bundling is defensible given the shared
  fix pattern, but the keep-or-split call should be deliberate, not implicit.

- 🔵 **Testability**: AC3 relies on a relative message discriminator with no exit
  code or substring pinned
  **Location**: Acceptance Criteria
  AC3 requires a present `templates/` directory lacking `<name>.md` to "still
  yield a genuine template-not-found ... distinguishable from the message alone",
  but names no command, pins no exit code, and asserts only that the two messages
  differ. State the command (`config template <name>` with no resolving
  override), the expected exit code, and a concrete substring the not-found
  message must contain (and that the `ACCELERATOR_PLUGIN_ROOT` diagnostic is
  absent).

- 🔵 **Dependency / Completeness**: Load-bearing release-artefact assumption is
  not tracked as a precondition
  **Location**: Assumptions
  The `NotFound → PluginRootUnavailable` mapping rests on "every Accelerator
  installation ships a `templates/` directory", which the Assumption flags as
  "worth confirming against the release artifact's file list before relying on
  it" — but that confirmation lives only as prose, not as an Open Question, a
  first verification step, or a Dependencies note. If an installation ever ships
  without `templates/`, the mapping misdiagnoses a genuine installation. Promote
  the check to a tracked precondition.

#### Suggestions

- 🔵 **Clarity**: Filename slug and title frame the scope differently
  **Location**: Frontmatter: title
  The slug `template-enumeration-swallows-a-wrong-plugin-root` ("enumeration"
  points at `template_names` alone) is narrower than the title "Template
  resolution succeeds silently ..." ("resolution" spans `resolve_template`). The
  Drafting Notes record the scope extension that "motivated the title change", but
  the narrower slug was kept. Align the two framings, or note the slug reflects
  the original narrower framing.

### Strengths

- ✅ Every code symbol is anchored with a `store.rs` file:line reference in
  Technical Notes, so specialised identifiers are resolvable rather than opaque.
- ✅ The two root states are consistently distinguished — "unknown" (`None`) vs
  "known and wrong" (`Some(path)`, passes `require_plugin_root`) — with the
  mechanism spelled out.
- ✅ All four bug-reproduction elements are present: input, action, expected
  outcome, and the named actual outcomes per accessor (header-only table at
  exit 0; `Ok(None)` rendered as a template-name "not found").
- ✅ Requirements and criteria state concrete, observable outcomes (specific
  `ConfigError` variants, non-zero exit, a diagnostic naming the greppable
  literal `ACCELERATOR_PLUGIN_ROOT`), falsifiable in both directions.
- ✅ The behaviour change is anchored at a named existing characterisation test
  (`a_root_without_a_templates_directory_still_renders_an_empty_table`) replaced
  by its inverse rather than deleted, recording the change at the same site.
- ✅ The dependency on 0182 is named with its correct `done` status and the
  reconciliation is transparently recorded in Drafting Notes; provenance as a
  0182 Phase 5 follow-up is traceable end to end.
- ✅ Scope is bounded to one component and one struct (`FileConfigStore`), a
  legitimate carve-out of 0182 with standalone value; the `bug` kind fits.

### Recommended Changes

1. **Resolve the `eject` decision and reconcile Requirements, Open Questions and
   Acceptance Criteria** (addresses: the `eject` major, the three cross-cutting
   `eject` findings)
   Decide whether `eject` on a wrong root refuses (`PluginRootUnavailable`) or is
   a legitimate `EjectOutcome::NoDefault`. Then make exactly one section state it:
   if in scope, add an acceptance criterion pinning eject's wrong-root outcome and
   delete Open Question 2; if deferred, remove `eject` from Requirement 2's site
   list and record the deferral.

2. **Make "wrong root" mean one thing across the item** (addresses: the wrong-root
   major)
   Either narrow the Summary/title to "a root that carries no `templates/`
   directory" so intent matches the criteria, or add a criterion covering a wrong
   root that contains an unrelated `templates/` directory and note the design's
   discriminator. Restate AC2's precondition to match AC1's precise wording.

3. **Disambiguate the "three" count** (addresses: the "three sets" major)
   Distinguish "three accessors" (Summary) from "the sites where the distinction
   is applied" (Requirement 2) with a single consistent vocabulary, stating how
   many of each so "three" is never reused for a different set.

4. **Track the `templates/`-presence check as actionable work** (addresses: the
   assumption minor)
   Promote "confirm `templates/` is always present in the release artifact" to an
   Open Question, a first verification step, or a Dependencies note so the
   coupling is scheduled rather than relied on as prose.

5. **Pin AC3 concretely** (addresses: the AC3 minor)
   Name the command, the exact exit code, and a specific substring the genuine
   not-found message must contain, plus the absence of the
   `ACCELERATOR_PLUGIN_ROOT` diagnostic.

6. **Confirm or split the extended scope; align the slug** (addresses: the scope
   minor, the slug suggestion)
   Record why the two slices are one unit (e.g. a shared "is this root an
   installation?" helper), or split them; and align the filename slug with the
   broadened title.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: The work item is generally precise about the mechanism, carefully
separating the "unknown root" (`None`) state from the "known-and-wrong root"
(`Some(bad path)`) state and anchoring every code symbol with file:line
references. However, clarity is undermined by an overloaded "three" count (the
Summary's "three accessors" and Requirement 2's "three sites" are different,
only-partly-overlapping triples), a direct contradiction between a committed
requirement and an open question about eject, and a "wrong root" referent whose
meaning silently narrows between the Summary and the Requirements.

**Strengths**:
- Every code symbol (`template_names`, `resolve_template`, `plugin_template_path`,
  `ConfigError::PluginRootUnavailable`, `io_error`, etc.) is anchored with a
  `store.rs` file:line reference in Technical Notes.
- The two root states are consistently distinguished throughout — "unknown"
  (`None`, passes nothing) versus "known and wrong" (`Some(path)`, passes
  `require_plugin_root`) — with the `Some(path)` mechanism spelled out.
- Requirements and Acceptance Criteria state concrete, observable outcomes rather
  than vague desired properties.
- The Assumption underpinning detection (installation implies a `templates/`
  directory) is stated explicitly and flagged as needing confirmation.
- Drafting Notes transparently record the two corrected behavioural claims and the
  Option-B scope extension.

**Findings**:
- 🟡 **major** (confidence: high) — *"Three" refers to two different,
  only-partly-overlapping sets* — Location: Requirements. The Summary names
  "three plugin-root template accessors" — `template_names`, `resolve_template`'s
  plugin-default tier, and `plugin_template_path`. Requirement 2 then says "Apply
  the distinction at all three sites — `resolve_template`'s plugin-default tier,
  and `plugin_template_path`'s callers `plugin_default` and `eject`". These are
  two different triples: the second excludes `template_names` and includes
  `plugin_default` and `eject`, sharing only one member with the first. A reader
  who maps "all three sites" onto the Summary's "three accessors" will mis-scope
  the change. Use a single, consistent vocabulary and count.
- 🟡 **major** (confidence: high) — *Requirement mandates eject behaviour that an
  Open Question leaves undecided* — Location: Requirements / Open Questions.
  Requirement 2 commits `eject` to surface `PluginRootUnavailable` on a wrong
  root; Open Question 2 asks whether it should, "left open". The requirement
  commits to exactly what the open question says is undecided, so the item does
  not have a single coherent intent for one of its named sites. Reconcile the two.
- 🔵 **minor** (confidence: medium) — *"Wrong root" means something broader in the
  Summary than the Requirements detect* — Location: Summary. The Summary defines a
  wrong root as "present, but not an Accelerator installation"; the Requirements
  and Assumption narrow the detected case to "the root carries no `templates/`
  directory". That establishes "no `templates/` implies not an installation", not
  the converse: a non-installation root containing a `templates/` directory is a
  wrong root by the Summary's wording yet caught by none of the Requirements.
- 🔵 **suggestion** (confidence: low) — *Filename slug and title frame the scope
  differently* — Location: Frontmatter: title. The slug
  `template-enumeration-swallows-a-wrong-plugin-root` ("enumeration" points at
  `template_names` alone) is narrower than the title "Template resolution succeeds
  silently ..." A reader navigating by filename may expect a `template_names`-only
  fix. Align the two framings.

### Completeness

**Summary**: For a bug, 0184 is structurally complete and richly populated: a
precise Summary naming the three affected accessors, a motivating Context, three
specific Requirements, seven Acceptance Criteria, plus Assumptions, Open
Questions, Dependencies, Technical Notes and Drafting Notes. The four
reproduction elements (input, action, expected, actual) are all present, though
distributed across sections rather than consolidated. The one substantive
completeness gap is that the `eject` site is scoped in Requirements yet left
unresolved in Open Questions with no acceptance criterion, so "done" is not fully
defined for all three sites the item claims.

**Strengths**:
- The Summary states the bug unambiguously, naming the three specific accessors
  and the exact failure mode.
- The Context explains why the work is needed now: 0182 Phase 5 deliberately left
  these arms alone, the surviving behaviour is pinned as a named characterisation
  test, and a wrong root became newly plausible in the same change.
- All four bug reproduction elements are present.
- Seven specific Acceptance Criteria, including preservation criteria and the
  full-build gate (`mise run` exits 0).
- Frontmatter is complete and well-formed for a bug.
- Optional sections that add value — Assumptions, Open Questions, Dependencies,
  Technical Notes (with concrete `store.rs` references), Drafting Notes — are all
  populated.

**Findings**:
- 🟡 **major** (confidence: medium) — *eject site is scoped but its "done" is
  undefined and uncovered by any acceptance criterion* — Location: Acceptance
  Criteria. Requirements name three sites including `plugin_template_path`'s
  `eject` caller, but Open Questions asks whether eject against a wrong root should
  refuse at all, and none of the seven Acceptance Criteria define the expected
  outcome for the eject site. An implementer reaching eject has neither a settled
  directive nor a criterion to verify against. Resolve the Open Question (or defer
  eject), and if it stays in scope, add a criterion pinning eject's outcome.
- 🔵 **suggestion** (confidence: medium) — *Load-bearing assumption's verification
  is not captured as an actionable step* — Location: Assumptions. The Assumptions
  section states that every installation ships a `templates/` directory — the
  signal the entire `NotFound → PluginRootUnavailable` mapping rests on — and
  self-flags it as unconfirmed, but that confirmation is not elevated to an Open
  Question or acceptance step. Promote the "confirm against the release artifact"
  step so the check is tracked.

### Dependency

**Summary**: This bug is unusually well dependency-mapped for its kind: its sole
upstream dependency (0182) is named, its `done` status is correctly reconciled
against the frontmatter, and its provenance as a follow-up explicitly raised by
0182's Phase 5 §3 is captured in Context and confirmed by the referenced plan.
No downstream consumers are implied by Context or Requirements, so the empty
Blocks is appropriate rather than a hidden coupling, and the internal
characterisation-test it inherits is captured with a dedicated acceptance
criterion. The one coupling worth surfacing more prominently is the load-bearing
release-artifact assumption on which the new error mapping rests.

**Strengths**:
- The sole upstream dependency (0182) is named with its correct `done` status, and
  the Drafting Notes record reconciling that status against the frontmatter
  `relates_to` — no stale "blocked by" claim and no hidden upstream blocker.
- Provenance is fully traceable: Context matches the referenced plan's Phase 5 §3
  that raised this exact item as the follow-up.
- The inherited characterisation test is named in Context, and an acceptance
  criterion requires replacing it with its inverse rather than deleting it.
- No downstream consumers are named or implied, so the absence of Blocks entries
  reflects the scope accurately.

**Findings**:
- 🔵 **minor** (confidence: medium) — *Load-bearing release-artefact assumption is
  not tracked as a precondition* — Location: Assumptions. The fix's core
  `NotFound → PluginRootUnavailable` mapping rests on the Assumption that "every
  Accelerator installation ships a `templates/` directory", flagged as "worth
  confirming against the release artifact's file list before relying on it". This
  coupling to the distribution/release-artefact structure lives only as a prose
  assumption. If a valid installation ever ships without `templates/`, the mapping
  misdiagnoses a genuine installation. Surface the check as a tracked precondition
  or a Dependencies note.

### Scope

**Summary**: Work item 0184 is a coherent, appropriately-sized bug fix confined to
a single component (the template accessors of `FileConfigStore` in
`cli/config-adapters`) and a single defect class: a wrong-but-present plugin root
degrading silently instead of being diagnosed. Requirements, Acceptance Criteria
and Summary describe the same scope, and carving it out of the now-done 0182
(which deliberately stayed scoped to root *presence*) is a legitimate follow-up
with standalone value. The only scope considerations are the deliberate extension
from the original `template_names`-only bug to all three accessors — which the
item itself flags as splittable — and an unresolved in-or-out question for the
`eject` site.

**Strengths**:
- All requirements serve one unified purpose: turning a silent-wrong-answer on a
  known-but-wrong plugin root into a named `PluginRootUnavailable` refusal.
- Bounded to one component and one struct (`FileConfigStore`); no cross-service or
  cross-team boundaries are spanned.
- The boundary is stated explicitly — the Requirements name what must keep working
  (root-independent config families and project-local overrides).
- The split from 0182 is well-justified rather than artificial fragmentation.
- The `bug` kind fits the scope.

**Findings**:
- 🔵 **minor** (confidence: medium) — *Extended scope bundles two
  independently-deliverable fixes the item itself flags as splittable* — Location:
  Drafting Notes. The item was deliberately enlarged (the "Option B" choice) from
  the original `template_names`-only bug to also cover `resolve_template`'s
  plugin-default tier and `plugin_template_path`, and the Drafting Notes
  acknowledge a reviewer "may still prefer to split" that work. The two slices
  could each be delivered, tested and rolled back independently. Bundling is
  defensible given the high cohesion, but is worth a deliberate keep-or-split
  decision rather than an implicit default.
- 🔵 **suggestion** (confidence: medium) — *Whether the eject site is in scope is
  left undecided, leaving the unit-of-work boundary partly open* — Location: Open
  Questions. The Requirements name `eject` as one of three sites, but an Open
  Question asks whether eject should surface `PluginRootUnavailable` at all, "left
  open", and no criterion pins eject's behaviour. One of the three named change
  sites has an unresolved in-or-out status, so the boundary of the deliverable
  cannot be fully stated. Resolve the eject semantics before implementation (or
  scope it out) and align Requirements and Acceptance Criteria.

### Testability

**Summary**: As a bug, 0184 is unusually well-specified for verification: the
broken behaviours are quoted from source, the actual outcomes (header-only table
at exit 0; `Ok(None)` rendered as a template-name "not found") are named, and most
criteria pin observable CLI outcomes with an explicit contrast to today's
behaviour, making them falsifiable in both directions. Two gaps weaken the
verification strategy: the `eject` site named in Requirements has no acceptance
criterion and its intended outcome is an unresolved Open Question, and every
criterion operationalises "a wrong root" as "a root lacking a `templates/`
directory" — a narrower condition than the Summary's stated intent, leaving a
plausible wrong-root case unverified. A relative message discriminator in one
criterion could also be pinned more concretely.

**Strengths**:
- The bug is reproducibly specified: the offending code is quoted, the actual
  broken outcomes are named per accessor, and expected outcomes appear in the
  criteria.
- AC1 and AC2 are framed as observable CLI outcomes — exit non-zero plus a
  diagnostic naming the greppable literal `ACCELERATOR_PLUGIN_ROOT` — each with an
  explicit "replacing today's ..." contrast.
- AC4 supplies a concrete reproduction (mode `0o000`) and a specific expected
  error variant naming the path.
- AC5 anchors the behaviour change at a named existing characterisation test
  replaced by its inverse.
- Scope is bounded to three named accessors with no unbounded language, and no
  criterion is tautological.

**Findings**:
- 🟡 **major** (confidence: medium) — *eject site has no acceptance criterion and
  an undecided expected outcome* — Location: Requirements / Open Questions.
  Requirements direct the fix to "apply the distinction at all three sites — ...
  and eject", but no Acceptance Criterion exercises `eject`, and the Open
  Questions leave its intended behaviour undecided. One-third of the mandated
  change has no pass/fail check and no defined expected outcome. Resolve the Open
  Question, then add a criterion of the form "`config template <name> --eject`
  against a root lacking a `templates/` directory yields <decided outcome>"; or
  scope `eject` out of Requirements until the decision lands.
- 🟡 **major** (confidence: medium) — *Criteria verify "no `templates/`
  directory", narrower than the Summary's "wrong root" intent* — Location:
  Acceptance Criteria. The Summary's intent is that the accessors "succeed
  silently when the root is known and wrong — present, but not an Accelerator
  installation", but every criterion operationalises this as a root that lacks a
  `templates/` directory. A wrong root that contains an unrelated `templates/`
  directory is covered by no criterion and, by the design's own discriminator,
  would still succeed silently — the exact failure mode the item exists to close.
  Either narrow the Summary/title, or add a criterion covering a wrong root that
  retains a `templates/` directory; and restate AC2's precondition to match AC1's
  precise wording.
- 🔵 **minor** (confidence: medium) — *AC3 relies on a relative message
  discriminator with no exit code or substring pinned* — Location: Acceptance
  Criteria. AC3 requires a present `templates/` directory lacking `<name>.md` to
  "still yield a genuine template-not-found ... distinguishable from the message
  alone", but neither names the command nor pins the expected exit code, and its
  discriminator is relative (the two messages must merely differ). State the
  command, the expected exit code, and a specific substring the genuine not-found
  message must contain, plus that the `ACCELERATOR_PLUGIN_ROOT` diagnostic is
  absent.

## Re-Review (Pass 2) — 2026-09-08

**Verdict:** REVISE

All five lenses were re-run against the revised work item. Every finding from
Pass 1 is resolved. The verdict remains REVISE because a fresh pass surfaced two
new major findings — both pre-existing gaps the first round did not reach, plus
minor artefacts of the Pass 1 edits.

### Previously Identified Issues

- 🟡 **Clarity**: "Three" refers to two different sets — **Resolved** (Requirement
  2 now reads "at each plugin-default site"; re-review finds no count collision).
- 🟡 **Clarity**: Requirement mandates eject behaviour an Open Question leaves
  undecided — **Resolved** (eject resolved to refuse; the contradiction is gone).
- 🔵 **Clarity**: "Wrong root" broader in Summary than Requirements — **Resolved**
  (the Summary now defines the term explicitly and scopes out the retained-
  `templates/` case; re-review praises the consistent definition).
- 🟡 **Completeness**: eject scoped but "done" undefined and uncovered —
  **Resolved** (dedicated eject acceptance criterion added).
- 🔵 **Completeness**: assumption verification not actionable — **Resolved**
  (promoted to a tracked precondition under Dependencies).
- 🔵 **Dependency**: load-bearing release-artefact assumption untracked —
  **Resolved** (re-review cites "strong dependency hygiene").
- 🔵 **Scope**: extended scope bundles splittable fixes — **Resolved / accepted**
  (re-review downgrades to a suggestion, "no change required — the split was
  consciously declined").
- 🔵 **Scope**: eject in-or-out undecided — **Resolved**.
- 🟡 **Testability**: eject has no criterion / undecided outcome — **Resolved**.
- 🟡 **Testability**: criteria narrower than the "wrong root" intent —
  **Resolved** (narrowed spec aligns intent and criteria).
- 🔵 **Testability**: AC3 relative discriminator — **Resolved** (now AC4, praised
  as a model criterion, falsifiable in both directions).

### New Issues Introduced

- 🟡 **Dependency** (major): The changed `template_names` error semantics
  propagate to the visualiser server's `visualiser/server/src/compose.rs` — a
  consumer named in the 0182 plan (Phase 5 §2) that propagates `ConfigError` via
  `?` into `ComposeError`. That downstream coupling is captured nowhere in the
  item, so the server would newly hard-error where it previously degraded to an
  empty template set. Pre-existing coupling, first surfaced this pass. Name the
  server consumer in Dependencies/Requirements and state whether it refuses
  identically or is out of scope.
- 🟡 **Testability** (major): AC7 ("A user override still resolves against a wrong
  root") names no command, no override setup, and no observable pass condition —
  unlike AC1–AC6. It cannot lean on the absent-root property
  `a_user_override_still_resolves_with_no_plugin_root`, which pins a different
  case. Restate as a full input/action/expected triple.
- 🔵 **Clarity / Testability** (minor): The new eject criterion (AC3) is framed as
  an internal enum (`ConfigError::PluginRootUnavailable`) with an ambiguous
  "naming the root" — variable name or path value? — where AC1/AC2 pin a
  CLI-observable diagnostic naming `ACCELERATOR_PLUGIN_ROOT`. Artefact of the
  Pass 1 edit; pin AC3 to the same observable shape and disambiguate.
- 🔵 **Clarity** (minor): The "Open Questions" section now holds only resolved
  items (one annotated "Resolved in favour of…", one prefixed "Resolved:"), so
  the header contradicts its content. Artefact of the Pass 1 edit; retitle to a
  Decisions section or leave Open Questions as "none".
- 🔵 **Testability** (minor): AC5's `chmod 0o000` trigger is bypassed when the
  test runs as root (common in CI containers), so the `Io` branch may never fire.
  Note the non-root constraint or use an environment-independent trigger (e.g. a
  `templates` path that is a file). Pre-existing.
- 🔵 **Testability** (minor, low): AC6's "replaced by its inverse" does not name
  the concrete replacement assertion; tie it to AC1. Pre-existing.
- 🔵 **Clarity** (suggestion, low): Requirement 3's heading names "root-independent
  families" the body never addresses (it discusses only project-local overrides).
  Pre-existing.
- 🔵 **Clarity** (suggestion, low): the title's "succeeds silently" describes only
  `template_names`; the other two accessors misdiagnose rather than succeed.
- 🔵 **Completeness** (suggestion): the four bug-reproduction elements are present
  but scattered across Summary and Acceptance Criteria rather than consolidated
  into a Reproduction section as sibling 0182 does. Pre-existing.

### Assessment

The Pass 1 revisions landed cleanly — the eject decision, the narrowed "wrong
root", the count fix, the pinned AC4, and the tracked assumption are all
confirmed resolved, and no lens re-raised them. The item is not yet ready: two
new majors stand. The **visualiser `compose.rs` downstream coupling** is the
substantive one — it is a genuine behavioural change in a second component that
the item does not scope, and it should be decided (refuse identically, or
explicitly out of scope) before implementation. The **AC7 user-override
criterion** and the two edit-artefact minors (the eject AC's internal-enum
framing, the mis-titled "Open Questions" section) are quick, mechanical fixes.
Addressing these four would clear the path to APPROVE.

## Re-Review (Pass 3) — 2026-09-08

**Verdict:** REVISE

All Pass 2 findings are resolved. The two Pass 2 majors — the visualiser
`compose.rs` coupling and the terse AC7 — are both confirmed fixed and now
praised (the server is reasoned into scope with rationale; AC8 is a full
input/action/expected triple). The verdict remains REVISE because two new majors
surfaced, both finer than the earlier rounds: a count conflict, and an
un-exercised code site. This pass shows clear diminishing returns — several new
findings are consumer-enumeration refinements opened by Pass 2's scope expansion.

### Previously Identified Issues

- 🟡 **Dependency**: visualiser `compose.rs` downstream coupling uncaptured —
  **Resolved** (brought into scope with a requirement, criterion, dependency
  entry and technical note; re-review calls it "well-orchestrated").
- 🟡 **Testability**: AC7 user-override omits action/outcome — **Resolved** (now
  a full triple, praised).
- 🔵 **Clarity / Testability**: eject AC internal-enum framing / "naming the root"
  ambiguity — **Resolved** (re-pinned to a CLI-observable diagnostic naming
  `ACCELERATOR_PLUGIN_ROOT`; wording standardised).
- 🔵 **Clarity**: "Open Questions" held only resolved items — **Resolved**
  (retitled "Decisions"; re-review calls it "honest").
- 🔵 **Testability**: AC5 `0o000` root-bypass — **Resolved** (environment-
  independent trigger + non-root guard, praised).
- 🔵 **Testability**: AC6 "inverse" not concrete — **Resolved** (tied to AC1).
- 🔵 **Clarity**: Requirement 3 heading vs body — **Resolved** (root-independent
  families clause added).

### New Issues Introduced

- 🟡 **Clarity** (major): The Summary's "Three plugin-root template accessors"
  conflicts with Dependencies' "0182 introduced `PluginRootUnavailable` and the
  two named accessors this extends" — two vs three is unreconciled at the point a
  reader sizes the work. State how many 0182 covered vs how many this item
  touches. (Pre-existing "two named accessors" phrasing, first caught this pass.)
- 🟡 **Testability** (major): Of the four enumerated sites (`template_names`,
  `resolve_template`'s tier, `plugin_default`, `eject`), the criteria observably
  exercise only three — `plugin_default` is not driven distinctly. State that
  `config template <name>` reaches it (so AC2/AC4 cover it) or add a criterion.
- 🔵 **Dependency** (minor): `eject --all` (`eject_all`, `inbound/cli.rs:541` per
  the 0182 plan Phase 5 §2) is a second `template_names` consumer whose wrong-root
  behaviour changes but is uncaptured; AC3 pins only single-target `eject`. Name
  it and add a refusal criterion.
- 🔵 **Testability** (minor): AC3 (eject) omits the exact command form (does eject
  take a name argument?), unlike AC1/AC2. Give the runnable invocation.
- 🔵 **Testability** (minor): Requirement 4's root-independent-families
  preservation has no dedicated criterion (relies on the blanket `mise run`
  gate). Add a `config paths`-style criterion or note existing coverage.
- 🔵 **Clarity** (minor): Requirements and criteria are cross-referenced by
  ordinal ("AC1", "Requirement 3") but neither list is numbered, and one ordinal
  points at the wrong bullet. Number the lists or reference by bold label.
- 🔵 **Dependency** (minor, low): the visualiser frontend/`visualise` surface
  consuming the new `ComposeError` is not traced one step further.
- 🔵 **Clarity / Dependency / Completeness** (suggestions): None-root called
  "unknown"/"missing"/"absent" interchangeably; "Option B" referenced without
  definition; distribution lockstep with the launcher artifact not restated for a
  two-binary change; no consolidated Reproduction block; scope bundle confirmed
  cohesive (keep).

### Assessment

The item is now high quality and the review has hit diminishing returns. Passes 1
and 2 closed every structural defect; Pass 3's two majors are a wording conflict
(two vs three accessors) and a coverage gap (`plugin_default` not distinctly
exercised) — both cheap to fix, neither a design flaw. Several new findings
(`eject --all`, the visualiser frontend, distribution lockstep) are
consumer-enumeration refinements that Pass 2's scope expansion opened, and further
passes would likely keep surfacing similarly fine points. Recommended close-out: a
single consolidated fix round covering both majors and the actionable minors
(number the lists, name `eject --all` with a criterion, clarify the
`plugin_default`/`config template <name>` relationship, add a root-independent-
family criterion), then treat the item as ready rather than re-entering the loop.

## Re-Review (Pass 4, targeted: clarity + testability) — 2026-09-08

**Verdict:** COMMENT

A targeted confirmation pass of the two lenses that carried Pass 3 majors. Both
majors are resolved and no major remains, so the verdict moves from REVISE to
COMMENT: the work item is acceptable as-is, with only minor polish outstanding.
The Pass 3 consolidated fix was made after reading the call graph in `store.rs`
and `inbound/cli.rs`, which also corrected a factual error the clarity lens had
suggested (0182 gated all three accessors on an absent root, not "two of three").

### Previously Identified Issues

- 🟡 **Clarity**: two-vs-three accessor count unreconciled — **Resolved** (stated
  consistently as three across Summary, Decisions, Technical Notes, Dependencies).
- 🟡 **Testability**: `plugin_default` site not exercised — **Resolved** (AC5
  covers `diff`/`reset`, its verified CLI surface; R2 and the mapping table
  document that `config template <name>` does not reach it).
- 🔵 **Dependency**: `eject --all` consumer uncaptured — **Resolved** in the
  consolidated fix (AC4 + Dependencies entry).
- 🔵 **Testability**: eject command form / root-independent-family criterion —
  **Resolved** (AC3 spells `config templates eject <name>`; AC11 added).
- 🔵 **Clarity**: ordinal references to unnumbered lists — **Resolved** (R1–R4,
  AC1–AC12 labels; re-review confirms every cross-reference resolves).

### Remaining Minor Polish (addressed in pass 4)

- 🔵 **Testability**: AC11 asserted bare "succeeds" — now pinned to exit 0 **and**
  non-empty output, since the `--fail-safe` `config` family can exit 0 while
  degraded.
- 🔵 **Clarity**: a lone "unknown-root" deviated from the standardised
  "absent root" — now fixed.
- 🔵 **Clarity**: "plugin-default tier" vs `plugin_default` near-identical names —
  R2 now states explicitly they are two distinct sites.
- 🔵 **Testability** (suggestion): AC6 named the outcome but not the trigger — now
  gives the concrete compose-path invocation and assertion.

### Assessment

The work item is ready for implementation. Four review passes closed every
structural defect and every finding above the suggestion floor; the pass-4 minors
were mechanical and are applied. The item now carries twelve labelled acceptance
criteria mapped to verified code sites, names every consumer of the changed
accessors (the three CLI surfaces, `eject --all`, `diff`/`reset`, and the
visualiser server), and records its decisions and the load-bearing `templates/`
precondition. Two conscious non-changes remain on the record: no consolidated
Reproduction block, and the title's "succeeds silently" strictly describing only
`template_names`.

## Verdict Change — APPROVE (2026-09-08)

The reviewer (Toby Clemson) approved the work item, moving the verdict from
COMMENT to APPROVE. All findings from the four review passes above the suggestion
floor are resolved; the only outstanding items are the two conscious non-changes
noted above (no consolidated Reproduction block; the title's "succeeds silently"
framing), neither of which blocks implementation. 0184 is accepted as ready for
planning. Status transition to `ready` handled separately via
`/accelerator:update-work-item`.

