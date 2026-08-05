---
type: work-item
id: "0187"
title: "Generalise the Sub-Binary Registration Surface"
date: "2026-07-31T10:41:51+00:00"
author: Toby Clemson
producer: create-work-item
status: ready
kind: task
priority: high
parent: "work-item:0136"
blocked_by: ["work-item:0168"]
blocks: ["work-item:0169", "work-item:0170", "work-item:0171", "work-item:0172", "work-item:0173"]
relates_to: ["work-item:0165", "work-item:0182"]
tags: [build-system, distribution, rust]
last_updated: "2026-08-01T17:29:06+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0187: Generalise the Sub-Binary Registration Surface

**Kind**: Task
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

The dispatched-sub-binary machinery 0165 delivered is parameterised over
`DISPATCHED_SUBBINARIES` almost everywhere — but `validate_dispatch_coherence`
is hardcoded to the visualiser, and the registration surface is documented only
in research, not in `tasks/README.md`. Make the coherence guard generic over the
**dispatch token** (the subcommand name in `accelerator <token>`, and the
corresponding entry in `DISPATCHED_SUBBINARIES`) and document the registration
surface as a mechanical checklist, so the five stories that each ship a
sub-binary add a token rather than rediscovering the surface and re-fighting the
same visualiser-shaped assumptions — and are not serialised behind whichever
gets there first.

Two smaller deliverables support those: a parameter-with-default signature
change across three release-stage builders, so the "already token-parameterised"
assumption is discharged by test rather than by inspection; and the reciprocal
dependency edges on the four consuming stories that lacked them, on the 0168
blocker, and in the parent epic's annotation, so the coupling is visible from
the consuming side while this task is still in flight. A one-symbol rename in
`tasks/lint/skill_permissions.py` rides along, so the guard depends on a public
parsing contract rather than a private one.

Collapsing registration to a single allowlist entry — deriving manifest paths,
ignore entries and the fixture manifest from the token — is explicitly **out of
scope**. Registration stays multi-step; this task makes the steps documented and
enforced rather than rediscovered.

## Context

`accelerator-visualiser` is currently the only dispatched sub-binary. Five
epic-0136 stories each ship one — 0169 (`vcs`), 0170, 0171, 0172, 0173 — and
each would otherwise rediscover the same registration points and re-fight the
same visualiser-shaped assumptions.

Signing (`tasks/signing.py:50-73`) and release upload and re-verification
(`tasks/github.py:218-235`, `:270-293`) already iterate `DISPATCHED_SUBBINARIES`
and need no behavioural change. `validate_dispatch_coherence`
(`tasks/build.py:35`, `:189-208`) does: it reads a hardcoded
`_VISUALISE_SKILL_RELATIVE` and compares `"accelerator visualiser" in skill`
against `"visualiser" in DISPATCHED_SUBBINARIES`, so the binding it enforces —
between a consuming skill and the sub-binary subcommand that skill invokes — is
unenforced for every other token.

Two properties of today's guard are easy to lose in a naive generalisation and
must survive. Both directions are named descriptively below and in the
Requirements, rather than as "first"/"reverse", so the two sections cannot drift
apart:

- **token→consumer**: every shipped token must have a consuming skill. This is
  the direction a loop over `DISPATCHED_SUBBINARIES` gives for free.
- **invocation→registration**: every skill invocation of a non-built-in
  subcommand must correspond to a shipped token. Today's `invokes != dispatched`
  comparison catches this; a loop over the allowlist alone would silently drop
  it.

A third property is a *defect* to fix rather than preserve: the current match is
a bare substring, which the visualiser SKILL.md satisfies in prose
(`skills/visualisation/visualise/SKILL.md:46`, `:160`) as well as in its real
invocations (`:8`, `:30`). A generalised matcher must not inherit that
looseness, or it will report tokens as bound on the strength of a documentation
sentence.

Extracted from 0169 (review 2, re-review pass 4 — see
`meta/reviews/work/0169-vcs-subdomain-and-hooks-migration-review-2.md`), where
the scope lens observed that shared platform work delivered as a side effect of
its first consumer both inflates that consumer and silently gates its siblings.

## Requirements

### The generalised guard

- Generalise `validate_dispatch_coherence` so it checks the skill↔subcommand
  binding for **every non-exempt** entry in `DISPATCHED_SUBBINARIES`, not only
  the visualiser. Remove `_VISUALISE_SKILL_RELATIVE` and the `"visualiser" in
  DISPATCHED_SUBBINARIES` membership assertion. No literal `visualiser` may
  remain in the guard or its helpers.
- Iterating the collection does **not** subsume the membership assertion being
  removed: an emptied or truncated allowlist makes the loop check nothing and
  pass, which is exactly what that assertion caught. Replace its anti-vacuity
  role explicitly — the guard fails when the resolved token collection is
  empty. On the release path an empty allowlist means the constant was lost,
  not that the project ships no sub-binaries.
- Define an **invocation** precisely, reusing the parsing
  `tasks/lint/skill_permissions.py` already exposes rather than duplicating it:
  a `!`-preprocessor command (`preprocessor_commands`) that `is_plugin_invocation`
  recognises and whose first argument after `bin/accelerator` is the token.
  Prose mentions, backticked references and plain body text do **not** count.
- Define the **permission half** as two conditions that must both hold of a
  *single* one of the invoking skill's `Bash(...)` rules
  (`frontmatter_bash_rules`). At least one rule must:
  1. authorise the token's subcommand — `covered_by` is true for the probe
     `${CLAUDE_PLUGIN_ROOT}/bin/accelerator <token>`; **and**
  2. *not* also authorise the bare launcher — `covered_by` is false for the
     `BARE_LAUNCHER` probe.

  Condition 2 is what rejects an ancestor glob, which satisfies condition 1.
  `skill_permissions.py:183-188` applies condition 2 alone to flag over-broad
  rules; this guard needs both. Omitting condition 2 produces a guard that
  accepts exactly the ancestor-glob shape the task exists to reject.
- Make the shared parsing contract public. Of the six names the guard reuses
  from `tasks/lint/skill_permissions.py`, five are already public
  (`preprocessor_commands`, `frontmatter_bash_rules`, `is_plugin_invocation`,
  `covered_by`, `PLUGIN_PREFIX`); one is not. Rename `_BARE_LAUNCHER`
  (`:42-44`) to `BARE_LAUNCHER` and update its existing caller at `:183-188`.
  A release-gating guard depending on a lint module's *private* surface is the
  coupling to avoid, and one rename removes it — the alternative is that a
  future change to a private symbol silently alters what the release considers
  a bound token. Reuse is the point: a guard that re-implemented this parsing
  inline would satisfy every behavioural criterion while drifting away from the
  lint rule it is meant to agree with, so the acceptance criterion asserts the
  imports positively rather than only forbidding private ones.
- State the quantifier once: a token is **bound** if at least one skill both
  invokes it and carries a rule satisfying both permission conditions. Other
  skills invoking the same token are not checked — multiple consumers are
  permitted.
- Preserve the **invocation→registration** direction: a plugin invocation of
  `accelerator <X>` in any skill, where `X` is neither a launcher built-in nor
  an entry in `DISPATCHED_SUBBINARIES`, fails the guard. The built-in set is
  `version` and `config` (`cli/launcher/src/launch/inbound/cli.rs:16-29`).
- Declare legitimately consumer-less tokens in an explicit exemption set —
  `SKILL_EXEMPT_SUBBINARIES`, a sibling constant to `DISPATCHED_SUBBINARIES` in
  `tasks/shared/paths.py`, empty when this task lands. The bar for an exemption:
  the sub-binary is invoked only by hooks or by another binary, never from a
  SKILL.md. A token that is neither bound nor declared exempt fails the guard.

### Test seams

- Make the guard's inputs injectable: `validate_dispatch_coherence` already
  takes `repo_root` (the repository root, from which it derives `skills/`); add
  the token collection and the exemption set as parameters defaulting to
  `DISPATCHED_SUBBINARIES` and `SKILL_EXEMPT_SUBBINARIES` — so tests pass
  fixture values rather than patching module state, and the test exercises the
  same code path the release runs.
- Give the three release-stage builders the same parameter-with-default shape,
  so the "already parameterised" assumption can be discharged by test rather
  than by inspection: the expected-set construction in `sign_staged_binaries`
  (`tasks/signing.py:50-73`, extracted to a pure helper), `_release_uploads` and
  `_subbinary_reverifies` (`tasks/github.py:218-235`, `:270-293`). This is a
  signature change only — no behavioural change, and each default keeps today's
  behaviour. *Fallback*: if the `sign_staged_binaries` extraction proves
  non-local (touching the signing flow rather than just the expected-set
  construction), drop this bullet to a sibling task and discharge the assumption
  by inspection, rather than growing this task's blast radius into the signing
  path.

### The registration checklist

- Document the registration surface in `tasks/README.md`, under a heading
  `## Registering a dispatched sub-binary`, as a checklist covering every
  registration point below. **This list is the normative enumeration**; research
  §8 is its provenance, and where the two differ (point 6's co-readers) this
  list is a correction of stale references rather than a divergence.
  1. `DISPATCHED_SUBBINARIES` (`tasks/shared/paths.py:25`)
  2. `_SUBBINARY_MANIFESTS` (`tasks/manifest.py:51-53`) when the crate is not at
     `cli/<token>/`
  3. the new crate's `Cargo.toml` — a mandatory `package.description`
     (`tasks/manifest.py:60-66`) and `version.workspace = true` (else the
     *version*-coherence check at `tasks/build.py:74-101` fires; this is a
     different check from the dispatch guard this task generalises)
  4. `cli/Cargo.toml` members
  5. `.gitignore` (`bin/<token>-*`)
  6. `cli/launcher/tests/fixtures/manifest.example.json`, together with **both**
     its co-readers, which must be updated in the same change:
     `tests/unit/tasks/test_manifest_contract.py:16` and the `include_str!` at
     `cli/launcher/src/launch/outbound/resolve/manifest.rs:135`. (Research §8
     names `tests/unit/tasks/test_manifest.py:38`; that reference is stale.)
  7. the skill binding — a skill invoking `accelerator <token>` with a
     `Bash(...)` rule satisfying both permission conditions — or an entry in
     `SKILL_EXEMPT_SUBBINARIES`
  8. cross-compile staging (`tasks/build.py:37`, `:290-331`; musl builds must
     pass `_assert_static_elf`, `:132-159`)
  9. SLSA provenance (`.github/workflows/main.yml:423-425`) — `subject-path` is
     token-generic only for binaries staged in `dist/release/`
     (`dist/release/accelerator-*`). Its other line,
     `skills/visualisation/visualise/bin/accelerator-visualiser-*`, is
     visualiser-shaped: a sub-binary staged under a skill's `bin/` tree needs a
     sibling line. **No action when** the binary is staged only in
     `dist/release/`.

  Every point states either an author action or an explicit "No action when …"
  clause; a point that needs no action for a given shape of crate says so
  rather than being omitted.
- Add a tenth entry covering the guard's own lockstep obligation: **adding a
  launcher built-in subcommand** (beyond `version` and `config`) requires
  updating the guard's built-in set, because the invocation→registration
  direction would otherwise fail the release on an unrelated change.
- State in the checklist that points 1 and 7 must land in the **same change**.
  The guard runs from `tasks/manifest.py:138` on every release, so a token
  registered before its binding (or its exemption) exists leaves the release
  path red in the interim.
- Make the naming constraints explicit in that checklist:
  - the Cargo **package** is `accelerator-<token>`, and where a domain crate
    already owns `cli/<token>/` the binary crate must live elsewhere with a
    `_SUBBINARY_MANIFESTS` entry — because `tasks/manifest.py` defaults the
    manifest path to `cli/<token>/Cargo.toml` and cargo-pup rules match on whole
    crate names;
  - the **token itself** may not contain `_`, because it derives
    `ACCELERATOR_<TOKEN>_BIN` (`cli/launcher/src/launch/core.rs:215-240`), which
    rejects underscores. 0170 is the consumer most likely to reach for
    `work_item`.

### Hand-offs

- **Done 2026-08-01** — the reciprocal dependency edges were recorded up front
  rather than at acceptance, because they only do their job while this task is
  in flight and 0170–0173 otherwise look unblocked:
  - `blocked_by: work-item:0187` plus a dated `- Blocked by: 0187 (…)`
    Dependencies bullet on 0170, 0171, 0172 and 0173, each pointing at
    `tasks/README.md#registering-a-dispatched-sub-binary`. 0170's bullet also
    carries the no-underscore token constraint, since `work_item` is the name
    that story would otherwise reach for;
  - `blocks: work-item:0187` on 0168, with a bullet stating that closing it —
    or confirming its residual scope cannot move the visualiser crate path —
    discharges this task's blocker;
  - epic 0136's decomposition annotation for 0187 corrected in full: the
    `unblocks` list gains 0172, the now-false "no blockers" clause is replaced
    by the 0168 prerequisite and its two discharge routes, and the annotation
    states why 0187 stays a Foundations item rather than moving to Phase 5
    behind its blocker. The Drafting Notes count was corrected from three
    sibling stories to four.
- The bullets already point at the `tasks/README.md` anchor; that link goes live
  when this task lands the section. The only obligation remaining to the siblings
  is the acceptance-time grep re-verification named in the criterion — no
  further edits are owed to them.

### Verification strategy

- Verify the generalisation against a **fixture token** — a test-only value
  passed into the guard (and into the three release-stage builders) through
  their injected parameters, never registered in `DISPATCHED_SUBBINARIES` — so
  this task can land before any of its consumers and so the real signing,
  manifest generation and upload paths are untouched by the verification itself.
- Where a test needs manifest data, it constructs an **in-test manifest**. The
  golden manifest fixture (`manifest.example.json`) is a shipped artefact and is
  not modified by this task.

## Acceptance Criteria

- [ ] **The hardcoding is gone.** `_VISUALISE_SKILL_RELATIVE` no longer appears
      anywhere under `tasks/`, and neither `validate_dispatch_coherence` nor its
      helpers contain a literal `visualiser` — asserted by a source-scan test in
      `tests/unit/tasks/`, so reintroduction fails rather than merely being
      noticed at review.
- [ ] **Missing binding fails.** `validate_dispatch_coherence` iterates the
      injected token collection and fails when any non-exempt token has no
      skill invoking `accelerator <token>`. Verified by a unit test passing a
      fixture token with a deliberately missing binding — the guard must be
      demonstrably **non-vacuous**, not merely passing.
- [ ] **Every token is checked, not just the first.** A two-token fixture
      collection whose *second* token is unbound and whose first is bound fails
      the guard, and the error names the second token.
- [ ] **An empty collection fails.** The guard fails when the resolved token
      collection is empty, rather than passing vacuously — verified with an
      injected empty collection. This is what replaces the removed
      `"visualiser" in DISPATCHED_SUBBINARIES` assertion, and it holds on the
      release path, where the test-time criteria below cannot reach.
- [ ] **Ancestor-glob permission fails.** A fixture token whose consuming skill
      exists but carries only `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator *)`
      fails the guard — i.e. permission condition 2 is applied, not just
      condition 1. Covered by its own fixture case rather than folded into the
      missing-binding test.
- [ ] **Prose does not bind.** Two fixture cases, each leaving the target token
      unbound and the guard firing: a skill that mentions the token only in
      prose or a backticked reference, and a skill that invokes a *different*
      token which is itself registered in the fixture collection and separately
      bound — so the guard fires because the target token is unbound, not
      because the invocation→registration rule caught an unregistered token.
- [ ] **Multiple consumers are tolerated.** A fixture token invoked by two
      skills passes when at least one carries a rule satisfying both permission
      conditions, even if the other carries only an ancestor glob.
- [ ] **invocation→registration fires, and built-ins are allowed.** A fixture
      skill invoking `accelerator <X>` where `X` is neither a launcher built-in
      nor in the token collection fails the guard; a fixture skill invoking
      `accelerator config` — a built-in with no entry in the collection —
      passes.
- [ ] **Exemption works in both directions.** A fixture token in the injected
      exemption set with no skill consumer passes; the same token with the
      exemption removed fails.
- [ ] **The visualiser binding still passes.** The generalised guard reports
      `visualiser` as bound, and `skills/visualisation/visualise/SKILL.md` is
      unedited — *unless* the drafting-time permission assumption has failed by
      pickup, in which case the scoped `allowed-tools` edit that Assumptions
      prescribes is permitted and its diff is recorded in Validation Results.
      Seeding `SKILL_EXEMPT_SUBBINARIES` with `visualiser` is not an acceptable
      route: it would make the guard vacuous for its only real binding.
      `tests/unit/tasks/test_build.py` covers this pass path and every fail path
      above.
- [ ] **The release path still calls the guard, and passes no override.** A
      unit test replaces `validate_dispatch_coherence` with a spy, runs manifest
      generation, and asserts it was called exactly once with **no** collection
      or exemption argument — so the call site cannot start passing a stale or
      narrowed collection unnoticed.
- [ ] **…and the defaults it falls back to are the real constants.** A separate
      assertion on `inspect.signature(validate_dispatch_coherence)` pins the two
      parameter defaults by **identity** (`is DISPATCHED_SUBBINARIES`,
      `is SKILL_EXEMPT_SUBBINARIES`), not equality. The spy above cannot observe
      this — passing no argument means the spy never sees the defaults, and the
      real function's resolution never runs — so the two assertions together are
      what stop a signature change leaving `tasks/manifest.py:138` operating on
      an empty or wrong collection with every other test green.
- [ ] **The already-parameterised stages hold for injected values.** With a
      two-token fixture collection injected: the signing expected-set helper
      yields one unsigned binary path per token per target across the four
      `TARGETS`; `_release_uploads` yields an asset *and* its `.minisig` per
      token per target; and `_subbinary_reverifies`, given an in-test manifest
      carrying both tokens, yields one item per token per target. No signing key
      and no network access are required — these are pure list-derivation
      assertions. *If the signing-extraction fallback in Test seams was taken*,
      this criterion applies to `_release_uploads` and `_subbinary_reverifies`
      only; the sibling task id is recorded in Dependencies and the signing
      inspection outcome in Validation Results.
- [ ] **…and for the defaults.** Called with no collection argument, each of the
      three builders yields precisely today's visualiser asset set — 4 unsigned
      binary paths from the signing helper, 8 upload entries (asset plus
      `.minisig` per target), and 4 re-verify items, every one naming
      `visualiser` — so a mis-wired default (empty tuple, wrong constant,
      mutable-default aliasing) fails rather than silently emptying the real
      release. The same fallback carve-out as the criterion above applies.
- [ ] **The shared parsing contract is public, and reused rather than copied.**
      `tasks/lint/skill_permissions.py` exposes `BARE_LAUNCHER` with no
      underscore prefix, its caller at `:183-188` is updated, and the literal
      `_BARE_LAUNCHER` no longer appears anywhere under `tasks/` — so a retained
      alias cannot satisfy the rename. A source-scan test additionally asserts
      that the dispatch guard **imports** `preprocessor_commands`,
      `frontmatter_bash_rules`, `is_plugin_invocation`, `covered_by`,
      `PLUGIN_PREFIX` and `BARE_LAUNCHER` from `tasks.lint.skill_permissions`,
      shadows none of those names locally, imports no underscore-prefixed name
      from that module, and compiles no regex of its own against `Bash(` or the
      `!`-preprocessor form. Every behavioural criterion above passes equally
      against an inline re-implementation of the parsing, so without the
      positive import assertion the reuse requirement — the whole justification
      for the rename and for the shared-artefact coupling — would go
      unverified.
- [ ] **The checklist is mechanically checkable.** The
      `## Registering a dispatched sub-binary` section of `tasks/README.md`
      contains exactly ten numbered items, each item body carrying either an
      imperative action line or the literal phrase `No action when`, and a test
      in `tests/unit/tasks/` asserts the section contains the literal strings
      `DISPATCHED_SUBBINARIES`, `SKILL_EXEMPT_SUBBINARIES`,
      `_SUBBINARY_MANIFESTS`, `package.description`, `version.workspace`,
      `cli/Cargo.toml`, `bin/<token>-*`, `manifest.example.json`,
      `test_manifest_contract.py`, `resolve/manifest.rs`, `_assert_static_elf`,
      `dist/release/accelerator-*`, `accelerator-<token>`,
      `ACCELERATOR_<TOKEN>_BIN` and `same change` — so renaming a registration
      point fails a test rather than silently ageing the docs.
- [x] **The hand-offs are recorded** *(done 2026-08-01, ahead of pickup)*. 0170,
      0171, 0172 and 0173 each carry `blocked_by: work-item:0187` plus a dated
      Dependencies bullet naming the `tasks/README.md` anchor; 0168 carries
      `blocks: work-item:0187`; epic 0136's 0187 annotation names the 0168
      prerequisite and reads "unblocks 0169/0170/0171/0172/0173". Re-verify by
      grep at acceptance in case a sibling was rewritten in the interim.
- [ ] `mise run` is green end to end.

## Dependencies

- **Blocked by**: 0168 (folded the visualiser into `cli/` — code landed, but its
  work item is still `ready`). Its landed layout is load-bearing: the checklist
  documents the `cli/<token>/Cargo.toml` default and the `_SUBBINARY_MANIFESTS`
  override precisely because of the visualiser's nested `cli/visualiser/server/`
  placement, and the visualiser is the guard's one existing passing binding.
  **Discharge condition** (this exact wording is mirrored in 0168's reciprocal
  bullet, epic 0136's annotation and Open Questions — keep the four in step):
  0168 is closed, *or* its residual scope is confirmed unable to move the
  visualiser crate path. If neither holds at pickup, proceed anyway and
  re-verify the path at acceptance — so this is a gate on confirmation, not a
  wait on 0168's phase.
- **Blocks**: 0169, 0170, 0171, 0172, 0173 — each registers a dispatched
  sub-binary and inherits this surface. Landing this first means each of those
  stories adds a token rather than reworking the pipeline. All five reciprocal
  edges are recorded as of 2026-08-01 — 0169's pre-dated this task, and the
  Hand-offs requirement added 0170–0173. The only remaining obligation is the
  acceptance-time grep re-verification, in case a sibling is rewritten in the
  interim.
- **Shared artefact**: `tasks/lint/skill_permissions.py`. This task makes a
  release-gating guard a second consumer of that module's parsing surface
  (`preprocessor_commands`, `frontmatter_bash_rules`, `is_plugin_invocation`,
  `covered_by`, `PLUGIN_PREFIX`, and the `_BARE_LAUNCHER` probe). A future
  change to that parser now changes what the release guard considers a bound
  token — a coupling that previously existed only within the lint task. The
  Requirements carry the mitigation (rename the one private symbol so the
  contract is explicit) and an acceptance criterion pins it.
- **Related**: 0165 (delivered the distribution pipeline — **done**); 0182
  (plugin-root self-location) — SKILL.md retains `${CLAUDE_PLUGIN_ROOT}` under
  that work, which is the invocation form the new scanner must match. If 0182
  changes the SKILL.md-facing prefix while this task is in flight, the scanner's
  matching must move with it or the guard passes vacuously.
- **Parent**: epic 0136.

## Assumptions

- The visualiser skill already satisfies the stricter permission definition:
  `skills/visualisation/visualise/SKILL.md:8` carries
  `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator visualiser *)`, which authorises
  the token probe without authorising the bare launcher. Confirmed at drafting
  time. *If this changes before pickup*, the fallback is a scoped `allowed-tools`
  edit to that skill — not seeding `SKILL_EXEMPT_SUBBINARIES`, which would make
  the guard vacuous for its only real binding.
- Signing, upload and re-verification genuinely need no behavioural change —
  they iterate `DISPATCHED_SUBBINARIES` already, and the two criteria above
  discharge that by test (for injected values and for the defaults) rather than
  by inspection. The signature change adding an injectable collection is not a
  behavioural change.
- SLSA provenance is the one stage confirmed **partially** visualiser-shaped:
  `dist/release/accelerator-*` is generic, its sibling line is not. Handled by
  documenting the condition as checklist point 9 rather than by changing the
  workflow, because no consumer needs the skill-`bin/` staging path today.
- If any other stage turns out visualiser-shaped, absorb the fix here **when it
  is local to the token loop** — meaning confined to the per-token iteration in
  `tasks/signing.py:50-73` / `tasks/github.py:218-235`, `:270-293`, requiring no
  change to the surrounding release orchestration and no new external
  dependency. Anything larger is raised as a sibling task and recorded in
  Dependencies, so this task's boundary stays statable regardless of what
  implementation finds.

## Open Questions

- **Can 0168's residual scope move the visualiser crate path?** If it can, the
  checklist's `cli/<token>/Cargo.toml` default and the guard's one existing
  passing binding both go stale immediately after this lands. Resolve by closing
  0168, *or* confirming its residual scope cannot move the visualiser crate path
  — the same discharge condition stated in Dependencies. *Default if neither
  holds at pickup*: proceed, and re-verify the visualiser's manifest path and
  its `_SUBBINARY_MANIFESTS` entry against the tree at acceptance, recording the
  outcome in Validation Results.

## Validation Results

*Filled at acceptance.*

- **SLSA glob inspection** (checklist point 9 is discharged by inspection, not
  test, because the globs live in workflow YAML): _pending_
- **Visualiser manifest path re-verification** (required only if 0168's
  discharge condition was unmet at pickup — see Open Questions): _pending_
- **Visualiser `allowed-tools` edit** (required only if the drafting-time
  permission assumption failed; record the diff): _not required as drafted_
- **Signing-extraction fallback** (required only if the Test-seams fallback was
  taken; record the inspection outcome and the sibling task id): _not required
  as drafted_
- **Hand-off edge re-verification** (grep `blocked_by: work-item:0187` on
  0170–0173, `blocks: work-item:0187` on 0168, and the 0136 annotation):
  _pending_

## Technical Notes

- `tasks/build.py:35` — `_VISUALISE_SKILL_RELATIVE`, the hardcoded skill path,
  removed by this task.
- `tasks/build.py:189-208` — `validate_dispatch_coherence`, invoked from
  `tasks/manifest.py:138` so it runs on every release. It already takes an
  optional `repo_root`; the token collection and exemption set join it.
- `tasks/lint/skill_permissions.py` — `preprocessor_commands`,
  `frontmatter_bash_rules`, `is_plugin_invocation`, `covered_by` and
  `PLUGIN_PREFIX` are the parsing this task reuses, all already public.
  `_BARE_LAUNCHER` (`:42-44`) is the second permission probe, applied on its
  own at `:183-188` to flag over-broad rules; this task renames it
  `BARE_LAUNCHER`, which is the name the Requirements and criteria use.
  Note `covered_by(command, pattern)` takes the probe as the command and the
  skill's rule as the pattern.
- A fixture token is test-only — passed through injected parameters, never added
  to `DISPATCHED_SUBBINARIES` — which is what keeps this task independent of any
  real sub-binary and leaves the release path untouched.

## References

- Extracted from: `meta/work/0169-vcs-subdomain-and-hooks-migration.md`
- Registration points originally enumerated in:
  `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`
  §8 — superseded as the normative list by the Requirements above
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0054 (git-style modular CLI of on-demand static binaries)
- Review: `0187-generalise-sub-binary-registration-surface-review-1.md` in
  `meta/reviews/work/`
