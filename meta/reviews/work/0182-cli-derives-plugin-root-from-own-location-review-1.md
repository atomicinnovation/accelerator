---
type: "work-item-review"
id: "0182-cli-derives-plugin-root-from-own-location-review-1"
title: "Work Item Review: bin/accelerator requires CLAUDE_PLUGIN_ROOT in the environment (never exported to skills)"
date: "2026-07-26T22:51:54+00:00"
author: "Toby Clemson"
producer: "review-work-item"
status: "complete"
target: "work-item:0182"
work_item_id: "0182"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["clarity", "completeness", "dependency", "scope", "testability"]
review_number: 1
review_pass: 2
tags: ["cli", "launcher", "bootstrap", "plugin-root"]
last_updated: "2026-07-26T23:24:12+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Work Item Review: bin/accelerator requires CLAUDE_PLUGIN_ROOT in the environment (never exported to skills)

**Verdict:** REVISE

This is an unusually well-evidenced work item: nearly every claim is anchored
to a file:line, a measured probe, or a documented Claude Code contract, the
reproduction is byte-identical to the report, and the Drafting Notes record the
trigger for every scope decision. Two structural defects nonetheless block
implementation. First, the item's flagship automated check asserts only exit 0
for a command family that R8 makes exit 0 on failure — so the one test intended
to prove all 43 skills are fixed cannot fail. Second, R4's negative test seam
and R1/AC4's "self-location wins unconditionally" rule are mutually
unsatisfiable as written, and three lenses independently reached that
conclusion. Beyond those, the largest open risks are scope-shaped: the
`allowed-tools` substitution question can trapdoor into a 43-file migration
discovered at the pre-release gate, and R9's hook-plus-documentation thread is
independently deliverable but bolted onto a fix that blocks the next release.

### Cross-Cutting Themes

- **R4's seam contradicts R1's write-only rule** (flagged by: clarity,
  dependency, testability) — R4 moves the `/nonexistent` injection onto
  `ACCELERATOR_PLUGIN_ROOT` while R1 and AC4 require the bootstrap to ignore and
  overwrite exactly that variable. The Technical Notes excuse this by saying
  injection seams target the launcher and server, not the bootstrap — which does
  not hold for the R4 site. Every lens that traced the seam arrived here
  independently, which makes it the single highest-value edit in the item.
- **Success is asserted as exit 0, and exit 0 is what failure now returns**
  (flagged by: testability, clarity) — the critical finding on AC1/AC9 and the
  R5 under-specification are the same root problem seen at two altitudes: the
  item never states an output observable that distinguishes a working CLI from a
  degraded one, even though it documents the silent-empty-table mode as its
  central hazard.
- **The `allowed-tools` substitution question has no bounded disposition**
  (flagged by: completeness, dependency, scope, testability) — all four lenses
  noted the same trapdoor: the question is resolvable only by the manual
  pre-release check, its negative branch is a 43-file frontmatter migration, and
  nothing states whether that branch is in scope, out of scope, or a successor
  item. It is also the one question whose answer determines whether the item's
  stated goal is met at all.
- **R9 is a separable capability carrying under-specified deliverables**
  (flagged by: scope, completeness, clarity, testability) — the hook is never
  named, its documentation target is never located, its containment criterion is
  unbounded, its version floor is unstated, and the item's own Drafting Notes
  already nominate it as the cut line.
- **External couplings are absent from Dependencies** (flagged by: dependency,
  completeness) — "Blocked by: none" sits alongside four unverified Claude Code
  behaviours, an unstated `${CLAUDE_PLUGIN_DATA}` version floor, a lockstep
  launcher-artifact requirement, and two named split points with no successor.

### Findings

#### Critical

- 🔴 **Testability**: Exit-0-only assertions are tautological for the
  `--fail-safe` command family
  **Location**: Acceptance Criteria (criterion 1 and the **Automated**
  criterion)
  AC1 and the flagship automated sweep assert nothing but exit status, yet R8
  makes any bootstrap-tier abort exit 0 when `--fail-safe` is in argv, and the
  research measured that the launcher's `config` path already exits 0 with empty
  output when no root is present. Every command in that family carries
  `--fail-safe` by lint contract, so both criteria are satisfied by R8 alone —
  with R1's self-location broken and R2's export absent.

#### Major

- 🟡 **Clarity / Dependency / Testability**: R4's negative test seam is
  unsatisfiable against R1's write-only rule and AC4
  **Location**: Requirements: R4; Acceptance Criteria (stale-value criterion)
  R4 moves `CLAUDE_PLUGIN_ROOT="/nonexistent"` at
  `skills/work/scripts/test-work-item-scripts.sh:1053,1086,1096,1106` onto
  `ACCELERATOR_PLUGIN_ROOT`, but R1 makes the bootstrap write-only for that
  variable and AC4 requires an ambient value to be ignored. The source research
  records that this suite reaches the CLI *through* the bootstrap, so
  self-location would clobber the injected value and the four fallback
  assertions would pass vacuously by finding real templates. AC4's "then
  **both** values are ignored" also reads oddly against its own "**or**"
  premise.
- 🟡 **Dependency / Scope / Completeness / Testability**: the `allowed-tools`
  substitution question leaves contingent scope unbounded and unrouted
  **Location**: Open Questions (first bullet)
  If `${CLAUDE_PLUGIN_ROOT}` no longer substitutes into `allowed-tools`, the
  remediation is a `${CLAUDE_SKILL_DIR}` migration across 43+ SKILL.md files
  plus an update to `tasks/lint/skill_permissions.py`'s `_PLUGIN_PREFIX` model.
  That branch is assigned neither to Requirements nor to "Deliberately
  unchanged", has no successor item and no Dependencies edge, and is discovered
  only at the manual gate — immediately before the release this item declares
  itself blocking.
- 🟡 **Completeness / Clarity**: R9's documentation deliverable is unlocated and
  has no acceptance criterion
  **Location**: Requirements: R9; Acceptance Criteria
  The Summary and Context both make terminal invocation a "documented" surface
  and R9 says "the documentation then gives a one-line `ln -s`", but no
  requirement names the artefact, its required content (per-channel shim path,
  mid-session upgrade caveat), or its audience — and none of the twelve criteria
  mention documentation. The only requirement whose output is a written artefact
  is the only one with no verification.
- 🟡 **Scope**: R9/R10 add an independent user-facing capability to a
  critical-path bug fix
  **Location**: Requirements: R9, R10
  Nothing in the reported failure requires a fourth `SessionStart` hook, a
  `${CLAUDE_PLUGIN_DATA}` shim, or a documented two-hop `PATH` install; the
  Summary joins it with "alongside that" and the Drafting Notes already call it
  the separable piece. A hook that writes to the filesystem every session also
  has a materially worse rollback profile than a self-locating bootstrap, on an
  item that blocks the next prerelease.
- 🟡 **Testability / Clarity**: the hook filesystem-containment criterion is
  unbounded, and "the hook" has no unique referent
  **Location**: Acceptance Criteria (containment and SessionStart criteria)
  "When the filesystem outside `${CLAUDE_PLUGIN_DATA}` is inspected, then
  nothing was created or modified" has no terminating procedure and no
  per-machine definition of "any other `PATH` directory". Read as covering the
  whole `SessionStart` batch it is also literally false, since the pre-existing
  config-detection and migration hooks legitimately write elsewhere. R9 never
  names or paths its hook, so the referent is ambiguous.
- 🟡 **Testability / Clarity**: R5 names neither the command nor the observable
  that constitutes success
  **Location**: Requirements: R5
  AC12 makes R5 the item's falsifiability anchor, yet R5 says only "asserting
  success". If the chosen command carries `--fail-safe`, success reduces to exit
  0 — which R8's new abort path also produces. Separately, "no environment
  beyond an absolute path" reads either as `env -u` of two variables (matching
  AC1) or as a fully cleared `env -i`, which would leave no `PATH`, `HOME`, or
  cache location for the bootstrap's external tools.
- 🟡 **Dependency**: Claude Code's behavioural contract is an external
  dependency but Dependencies records "Blocked by: none"
  **Location**: Dependencies
  The item turns entirely on Claude Code v2.1.220 behaviour — `allowed-tools`
  substitution (unconfirmed), skill-content substitution, hook-only export, and
  mid-session upgrade staleness — all recorded in Assumptions and Open Questions
  but invisible in Dependencies. A reader scheduling from that section sees a
  self-contained fix.
- 🟡 **Dependency**: R9's `${CLAUDE_PLUGIN_DATA}` dependency carries no minimum
  Claude Code version
  **Location**: Requirements: R9; Assumptions
  All evidence was gathered on v2.1.220 while the plugin's declared floor is
  v2.1.144, and the two are never reconciled. On any release predating the
  variable, `${CLAUDE_PLUGIN_DATA}/bin` degenerates to an absolute `/bin` path,
  so the hook either silently no-ops or writes outside plugin-owned space.
- 🟡 **Dependency**: the lockstep coupling between the shipped bootstrap and the
  separately-distributed launcher binary is not captured
  **Location**: Dependencies; Technical Notes: Why the export is mandatory
  After R1–R3 the bootstrap exports only `ACCELERATOR_PLUGIN_ROOT`, while a
  launcher built before the rename reads only `CLAUDE_PLUGIN_ROOT` — and the
  item's own notes establish that a rootless launcher does not error but
  silently drops the plugin-default template tier. Dependencies records the
  downstream release but not the upstream requirement that a matching launcher
  artifact ship in the same version.

#### Minor

- 🔵 **Clarity**: "tier" carries four senses and "CLI tiers" is an undefined
  scope boundary
  **Location**: Requirements: R3; Summary
  "Tier" denotes CLI layers, the Claude Code adapter layer, a config-precedence
  layer, and a historical state. R3 defines membership by participation
  regardless of directory while R7 and the grep criterion define it as "under
  `cli/`". Related slip: Blast radius says 43 skills "invoke the launcher" where
  the Summary establishes they invoke the bootstrap.
- 🔵 **Clarity**: "stable" is used in two senses within one bullet
  **Location**: Open Questions (second bullet)
  "A user tracking both the prerelease and **stable** marketplaces gets **two
  stable** shims" — the second sense means "path that does not move", and one of
  the two belongs to the prerelease channel, so the sentence first parses as the
  inverse of its point.
- 🔵 **Completeness**: two prevention recommendations from the source research
  are neither adopted nor excluded
  **Location**: Requirements: R7; Deliberately unchanged
  The research's "no plugin entry point may require `CLAUDE_PLUGIN_ROOT` from
  its environment" convention (broader than R7's `cli/`-only guard) and the
  extension of the `allowed-tools` conformance check to env-assignment prefixes
  are both absent. Given the item has a "Deliberately unchanged" section, the
  omission reads as oversight.
- 🔵 **Dependency**: the named split points leave no captured home for the
  deferred remainder
  **Location**: Drafting Notes; Dependencies
  Both sanctioned splits — R1+R2 for urgency, R9 for scope — leave R3/R7 or
  R9/R10 with no tracked destination and no dependency edge back to the release
  that motivated the split.
- 🔵 **Dependency**: the new no-env tests depend on unstated launcher artifact
  availability
  **Location**: Acceptance Criteria
  R5, R10, and the `!`-block sweep all assert a zero exit, which means clearing
  all 14 gates including fetch-and-verify. The item never says whether the
  launcher comes from a network fetch, the gated dev override, or a fixture tree
  — an ordering or flakiness dependency discovered when the suite is wired into
  `mise run`.
- 🔵 **Testability**: the two-hop symlink criterion names real user paths and an
  unobservable outcome
  **Location**: Acceptance Criteria (two-hop symlink criterion)
  Literal `~/.local/bin` and real `${CLAUDE_PLUGIN_DATA}` paths make a hermetic
  test impossible, and "resolves to the real installation directory" is an
  internal derivation with no external observable. R10 already states the shape
  neutrally.
- 🔵 **Testability**: the hook criteria give no injection seam, so their
  verification procedure is undefined
  **Location**: Acceptance Criteria (SessionStart criteria); Requirements: R9
  Verifying the version-change half needs a controllable
  `${CLAUDE_PLUGIN_DATA}`, two fake versions, and an out-of-band way to run the
  hook — none of which R9 states are overridable. Without a seam this is likely
  to be eyeballed across a real upgrade and never regression-guarded.
- 🔵 **Testability**: four requirements are covered only by the blanket
  `mise run` criterion
  **Location**: Acceptance Criteria vs Requirements R3, R4, R6, R9
  R3's four out-of-tree writers and its two renamed error-message contracts
  (`launcher/tests/version.rs:179`, `cache_root.rs:132,134`), R4's seam
  migration, R6's `mise.local.toml` removal, and R9's "prints the path it
  refreshed" have no distinguishing criterion. A green suite is not evidence
  those specific behaviours changed.
- 🔵 **Testability**: `grep -r 'CLAUDE_' cli/` as literally stated scans build
  output and `node_modules`
  **Location**: Acceptance Criteria (grep criterion)
  Run verbatim it traverses `cli/target/` and
  `cli/visualiser/frontend/node_modules/`, so the outcome depends on tree
  cleanliness rather than source state — or is silently checked by a different
  procedure than the R7 guard applies.
- 🔵 **Scope**: kind `bug` understates ten requirements spanning bootstrap,
  Rust, build system, hooks, and docs
  **Location**: Frontmatter: kind
  Only R1, R2 and R5 close the reported defect; the remainder is a rename, a
  prevention guard, a new capability, and a file deletion. Low delivery risk,
  but the label understates the review and coordination footprint.

#### Suggestions

- 🔵 **Scope**: R6 (remove `mise.local.toml`) is orthogonal, contingent, and has
  no acceptance criterion
  **Location**: Requirements: R6
  It touches nothing the other nine touch and the Drafting Notes leave it
  conditional ("drop R6 if it is serving another purpose"), so either the item
  cannot close until a side question is answered, or the deletion ships
  unnoticed and changes other developers' local setups.
- 🔵 **Scope**: R8 is separately deliverable, though defensibly bundled
  **Location**: Requirements: R8
  The research classifies it as "a separable robustness question", and it only
  takes effect on failures R1/R2 are meant to eliminate. Keeping it is a sound
  judgement — but if the item is split for urgency it belongs with the defect
  closure, since it is what limits blast radius the next time a gate fails.
- 🔵 **Testability**: the manual check has no recorded output and no defined
  disposition on failure
  **Location**: Acceptance Criteria (manual pre-release criterion)
  The procedure is well specified but no artefact records the result, and the
  skill selection is left open ("one `integrations/*`", "one `planning/*`"), so
  two verifiers may exercise different surfaces and the criterion can be marked
  passed without evidence.
- 🔵 **Testability**: tighten "the plugin-default templates are listed" to a
  named expected row
  **Location**: Acceptance Criteria (templates-list criterion)
  A table with one unrelated row satisfies it as written; pinning one concrete
  row (name plus `plugin default` source plus a path under the resolved root) is
  specific and stable against future template additions.
- 🔵 **Clarity**: "R9 adds a **fourth** hook" has no antecedent in context
  **Location**: Requirements (note following R10)
  Only two hooks are named in the surrounding text, so the reader cannot tell
  whether the ordinal counts all of `hooks/`, only the `SessionStart` ones, or is
  an error.
- 🔵 **Clarity**: a few terms and references are unresolvable as written
  **Location**: Assumptions; Requirements: R7
  "RCA" is never expanded, "ADR-0048" is cited as the sole justification for R7
  not being a Rust check but given no path or title, and "verify shim" is
  recoverable only from the quoted 0164 commit title.

### Strengths

- ✅ Nearly every factual claim is pinned to a file:line, a commit hash, or a
  measured probe — the abort site (`bin/accelerator:25`), the silent-degradation
  site (`config-adapters/src/store.rs:343-352`), the exec propagation site
  (`exec.rs:16-17`) — so premises can be verified rather than trusted.
- ✅ The Reproduction is exact and self-contained: the precise `env -u` command,
  byte-identical stderr, both live-session failure shapes, and the environment
  version (v2.1.220).
- ✅ The Expected-vs-actual table separates all four affected invocation
  surfaces into their own rows, so four distinct failure modes are not blurred
  into "skills are broken".
- ✅ R3's widening from "under `cli/`" to "readers and writers together,
  wherever they live" rests on a genuine indivisibility argument — four
  out-of-tree writers feed the `cli/` readers, so renaming readers alone cannot
  ship green — backed by a complete site table.
- ✅ The "Precedence and directionality" section pre-empts the obvious
  "isn't `ACCELERATOR_PLUGIN_ROOT` the same staleness hazard?" objection and
  answers it explicitly, rather than leaving the asymmetry to be inferred.
- ✅ Both boundaries of the rename are stated: "Exempt — do not blanket-rename"
  (the Claude Code adapter tier plus fixture strings) and "Deliberately
  unchanged" (XDG fallback, version source, visualiser fatal exit, empty-table
  visibility). Standalone packaging is explicitly out of scope.
- ✅ AC12's fail-before/pass-after demand is exactly the falsifiability check
  most bug items omit, and AC2 anticipates and closes the known false-pass mode
  ("**not** an empty table at exit 0") — the pattern the weaker criteria should
  copy.
- ✅ AC10 specifies the manual check's preconditions precisely enough to be
  repeatable by someone other than the author, and AC9 defines a mechanical,
  machine-derivable extraction procedure over an enumerable set.
- ✅ Assumptions is honest about assumption *strength*, recording that the
  `allowed-tools` claim came back weaker on re-check and why it cannot be
  confirmed from a broadly-permissioned session.
- ✅ Drafting Notes record what changed, why, and on whose decision, and name
  the separable cut lines up front (R1+R2 as the minimum shippable fix, R9 as
  the largest addition).
- ✅ Requirements name their actor concretely and state outcomes as observable
  states, and every requirement except R6 has a corresponding acceptance
  criterion with no Summary/Requirements/AC drift.

### Recommended Changes

1. **Give the success criteria an output observable, not just exit 0**
   (addresses: exit-0 tautology; R5 under-specification; templates-list
   tightening)
   Amend AC1 to "exits 0 **and** stdout is non-empty **and** stderr is empty",
   and the automated sweep likewise — since the fail-safe degradation is defined
   as empty stdout plus one stderr line, those three together separate real
   success from degraded success. Name R5's command explicitly and prefer one
   *without* `--fail-safe` (e.g. `config templates list`), asserting a concrete
   row with `plugin default` as its source.

2. **Resolve the R4 seam against R1's write-only rule** (addresses: R4/AC4
   contradiction)
   State which tier consumes the seam value in
   `skills/work/scripts/test-work-item-scripts.sh`. If it traverses the
   bootstrap, either route that call site via `ACCELERATOR_BIN` to bypass it, or
   give the seam a distinct bootstrap-transparent variable — and narrow AC4 to
   "an ambient `CLAUDE_PLUGIN_ROOT` is ignored; an ambient
   `ACCELERATOR_PLUGIN_ROOT` is overwritten by the bootstrap". R4 is currently
   budgeted as four one-line edits; it is not.

3. **Bound the `allowed-tools` substitution branch before implementation starts**
   (addresses: unbounded contingent scope; manual-check disposition)
   Add one line stating that the `${CLAUDE_SKILL_DIR}` migration is a separate
   work item if the probe fails, record a "conditionally blocks" edge in
   Dependencies, and state where the manual result is written down. Better
   still, run the probe first so this item's size is known up front rather than
   at the pre-release gate.

4. **Split R9 out, keeping R10 and the symlink chase** (addresses: R9/R10 scope;
   R9 documentation; hook containment; `${CLAUDE_PLUGIN_DATA}` version floor)
   Move the hook, the shim, the user-facing documentation, and the dual-channel
   naming question into a follow-on item covering terminal invocation as a
   supported surface. Retain R10's two-hop resolution test and R1's loop-form
   chase here as insurance for user-made links — the Drafting Notes already
   concede they stand alone. This also removes four findings from this item's
   ledger rather than requiring them to be fixed.

5. **Populate Dependencies with the external and distribution couplings**
   (addresses: "Blocked by: none"; launcher lockstep; split-point successors;
   test-time launcher source)
   Replace "Blocked by: none" with a Claude Code entry naming the version and
   the three depended-on behaviours; add the requirement that renamed launcher
   and server binaries ship in the same version as the renamed bootstrap; note
   that a split ship creates a follow-up carrying the deferred half; and state
   which launcher source the new no-env tests use.

6. **Make the negative and enforcement criteria enumerable** (addresses:
   containment criterion; grep criterion; uncovered R3/R4/R6/R9)
   Rewrite the containment criterion as a temp-tree snapshot diff scoped to the
   named R9 hook alone. Restate the grep criterion as "no tracked source file
   under `cli/` contains `CLAUDE_`, enforced by the R7 lint task (honouring
   `.gitignore`)", and require the guard to carry a negative test. Add two cheap
   criteria for `mise.local.toml`'s absence and for the renamed error-message
   contracts.

7. **Tighten vocabulary and name the unnamed** (addresses: "tier" overload;
   "stable" overload; hook has no referent; "fourth hook"; RCA/ADR-0048/verify
   shim)
   Define the layer names once (bootstrap / launcher / server / adapter) and use
   "tier" for one sense only; reserve "stable" for the release channel; give the
   R9 hook a name and path in the same style as the other hooks; drop or ground
   the "fourth" ordinal; expand "RCA" on first use and give ADR-0048 a path.

8. **Account for the two dropped prevention recommendations** (addresses:
   research recommendations neither adopted nor excluded)
   Either add them as requirements or list them under "Deliberately unchanged"
   with a one-line reason, so every recommendation from the item's own root-cause
   analysis is visibly dispositioned.

---
*Review generated by /accelerator:review-work-item*

## Per-Lens Results

### Clarity

**Summary**: This is an unusually precise work item: nearly every claim is
anchored to a file:line, a measured probe, or a documented Claude Code contract,
and the Drafting Notes record the trigger for each scope decision so a reader
can reconstruct the author's reasoning without asking. The main clarity defect
is a genuine requirement conflict between R4's negative test seam (which moves
to `ACCELERATOR_PLUGIN_ROOT`) and R1/AC4's rule that the bootstrap
unconditionally overwrites that variable — as written the two cannot both hold.
Beyond that, the residual issues are vocabulary rather than logic: "tier"
carries four distinct senses, "CLI tiers" is load-bearing as a scope boundary
but never defined, "stable" is used in two senses in the same bullet, and the
new R9 hook is never named, leaving "the hook" in the Acceptance Criteria
without a unique referent.

**Strengths**:

- Almost every factual claim is pinned to a file:line, a commit hash, or a
  byte-identical reproduction, so a reader can verify the premise rather than
  trust it — e.g. the abort site (`bin/accelerator:25`), the silent-degradation
  site (`config-adapters/src/store.rs:343-352`), and the exec propagation site
  (`exec.rs:16-17`).
- The "Precedence and directionality" section pre-empts the obvious "isn't
  `ACCELERATOR_PLUGIN_ROOT` the same staleness hazard?" objection and answers it
  explicitly, so the asymmetry between the two variables is not left for the
  reader to infer.
- The Expected-vs-actual table gives each invocation surface (`!` site, Bash
  tool, terminal, `--fail-safe`) its own row, so the four distinct failure modes
  are separated rather than blurred into "skills are broken".
- The Drafting Notes name what changed, why, and on whose decision (R3 widened
  after finding four out-of-tree writers; R9 revised from one hop to two after a
  maintainer question), which removes any guesswork about whether a requirement
  is deliberate.
- The R3 "Exempt — do not blanket-rename" list and the
  hooks-keep-`CLAUDE_PLUGIN_ROOT` paragraph state the boundary of the rename
  explicitly, closing off the most likely misreading of "rename every
  participant … wherever they live".
- Requirements name their actor concretely (`bin/accelerator` derives, the
  bootstrap exports, a `SessionStart` hook refreshes, a lint guard rejects)
  rather than hiding behind passive constructions, and outcomes are stated as
  observable states (exit 0, empty stdout, `plugin.json` found, grep returns no
  matches).

**Findings**:

- 🟡 **major** (confidence: medium) — **Requirements: R4 / Acceptance Criteria
  (stale-value criterion)**
  R4 requires the negative test seam in
  `skills/work/scripts/test-work-item-scripts.sh` (currently
  `CLAUDE_PLUGIN_ROOT="/nonexistent"`, which forces the hardcoded-fallback path)
  to move to `ACCELERATOR_PLUGIN_ROOT`, while R1 states the bootstrap is
  "**write-only** for `ACCELERATOR_PLUGIN_ROOT` — it never reads it, so no
  ambient value can redirect it" and the fourth Acceptance Criterion requires
  that a present `ACCELERATOR_PLUGIN_ROOT` be "ignored and self-location wins".
  The source research records that this seam reaches the bootstrap (via
  `test-work-item-scripts.sh:53`), so a bootstrap that unconditionally
  self-locates and exports its own value would clobber the injected
  `/nonexistent` and hand the launcher the real templates — defeating the four
  assertions R4 is trying to preserve. The Technical Notes assert the opposite
  premise ("the tests that inject a synthetic root target the launcher and
  server, not the bootstrap"), which the R4 site appears to contradict.
  **Impact**: An implementer cannot satisfy R4 and R1/AC4 as written; they will
  discover the conflict only when the four work-item-script assertions fail, and
  will then have to invent a resolution (a bootstrap-read escape hatch, a
  launcher-direct invocation, or deleting the seam) that the work item has not
  sanctioned.
  **Suggestion**: State whether the R4 seam invokes the bootstrap or the
  launcher directly, and reconcile the precedence rule accordingly — either
  narrow R1/AC4 to "no `CLAUDE_*` value can redirect it" and define how the seam
  injects a root through the bootstrap, or say explicitly that the seam must be
  restructured to bypass the bootstrap. Also note that AC4's "then **both**
  values are ignored" reads oddly against its own "**or**" premise where only
  one variable may be present.

- 🔵 **minor** (confidence: medium) — **Requirements: R5**
  R5 specifies a regression test that runs `bin/accelerator` "with **both**
  variables removed via `env -u` and no environment beyond an absolute path,
  asserting success". "No environment beyond an absolute path" has two readings:
  (a) only the two plugin-root variables are unset and the rest of the
  environment is inherited (which is what `env -u` does, and what the first
  Acceptance Criterion describes), or (b) the environment is cleared entirely
  (`env -i`), leaving no `PATH`, `HOME`, or `TMPDIR`. Reading (b) would change
  the test's meaning substantially, since the bootstrap invokes external tools
  (`uname`, a downloader) and needs a cache location.
  **Impact**: The two readings produce tests with different pass conditions —
  one reproduces the production invocation environment, the other asserts a much
  stronger and probably unattainable property — so the "single assertion that
  would have caught the bug at 0164" is not pinned down.
  **Suggestion**: Replace "no environment beyond an absolute path" with the
  intended mechanism explicitly, e.g. "the ambient environment otherwise
  inherited, with only `CLAUDE_PLUGIN_ROOT` and `ACCELERATOR_PLUGIN_ROOT`
  removed, and `argv[0]` given as an absolute path".

- 🔵 **minor** (confidence: medium) — **Acceptance Criteria (SessionStart shim
  and filesystem-containment criteria)**
  R9 introduces a new `SessionStart` hook but never gives it a name or path, and
  the Acceptance Criteria then refer to it as "a `SessionStart` has run" and
  "the hook". Because the work item also states that `hooks/` already contains
  other `SessionStart` hooks (config detection, migration reminders) which R9
  joins, "the hook" has more than one candidate referent, and the containment
  criterion — "Given the hook has run, when the filesystem outside
  `${CLAUDE_PLUGIN_DATA}` is inspected, then nothing was created or modified" —
  is literally false if read as covering the whole `SessionStart` batch, since
  the pre-existing hooks legitimately write elsewhere (config detection state,
  migration markers).
  **Impact**: A verifier reading the criterion as written could either fail a
  correct implementation (because another `SessionStart` hook touched the
  filesystem) or scope the check to the wrong process, leaving R9's deliberate
  "never writes to `~/.local/bin`" guarantee unverified.
  **Suggestion**: Give the R9 hook a concrete name/path in R9 (as the other
  hooks are named), and rephrase the containment criterion to constrain that
  hook alone — e.g. "when only `<named hook>` is run, nothing outside
  `${CLAUDE_PLUGIN_DATA}` is created or modified".

- 🔵 **minor** (confidence: medium) — **Requirements: R3 / Summary**
  The phrase "the CLI tiers" carries the scope of the whole rename (Summary:
  "replacing `CLAUDE_PLUGIN_ROOT` throughout the CLI tiers"; R3: "Rename every
  participant in the CLI tiers … wherever they live — not just under `cli/`")
  but is never defined, and "tier" is used elsewhere in at least four unrelated
  senses: the layers of the CLI stack ("bootstrap tier", "the Rust tiers"), the
  Claude Code integration layer ("the whole adapter tier"), a config-precedence
  layer ("the plugin-default template tier"), and a historical state ("the
  pre-CLI tier", "the shell tier"). The boundary also appears to shift between
  sections: R3 defines membership by participation regardless of directory,
  whereas R7's lint guard and the `grep -r 'CLAUDE_' cli/` criterion define it
  as "under `cli/`". A related slip: Blast radius says "43 skills invoke the
  launcher from a `!` preprocessor site", whereas the Summary establishes that
  skills invoke the bootstrap and "the bootstrap fails before any Rust runs".
  **Impact**: A reader deciding whether a given call site is in scope has to
  reverse-engineer the intended sense of "tier" from the Technical Notes rename
  tables, and could reasonably conclude the enforcement guard covers the full
  rename set when it only covers `cli/`.
  **Suggestion**: Define the layer names once (bootstrap / launcher / server /
  adapter layer) near the top of Requirements, use "tier" for only one of those
  senses, and state R3's scope as the enumerated rename set plus the direction
  rule rather than via the undefined "CLI tiers".

- 🔵 **minor** (confidence: high) — **Open Questions (second bullet)**
  The second Open Question uses "stable" in two senses within one bullet: "A
  user tracking both the prerelease and **stable** marketplaces gets **two
  stable** shims" — where the second "stable" means "a path that does not move
  across upgrades" and one of those two shims belongs to the *prerelease*
  channel. The bullet then closes with the release-channel sense again
  ("`accelerator` for stable and `accelerator-pre` for prerelease"). R9
  reinforces the overload with "a stable, upgrade-surviving path" and "that
  stable shim".
  **Impact**: On first read "two stable shims" parses as "two shims for the
  stable channel", inverting the point of the question, which is about
  disambiguating two channels' shims.
  **Suggestion**: Reserve "stable" for the release channel and use a different
  adjective for the path property (e.g. "fixed-path shim",
  "version-independent shim") throughout R9 and this Open Question.

- 🔵 **suggestion** (confidence: medium) — **Requirements (note following R10)**
  The note after R10 says `hooks/config-detect.sh` and
  `hooks/migrate-discoverability.sh` keep reading `CLAUDE_PLUGIN_ROOT`, then
  states "R9 adds a **fourth** hook to that same tier for the same reason." Only
  two hooks are named in the surrounding text, so the ordinal has no antecedent
  in context — the reader cannot tell whether "fourth" counts all hooks in
  `hooks/`, only the `SessionStart` ones (as the Drafting Notes' "a fourth
  `SessionStart` hook" suggests), or is simply an error.
  **Impact**: A minor stumble that makes the reader pause to check whether a
  hook has been overlooked in the exemption list.
  **Suggestion**: Either drop the ordinal ("R9 adds a further hook to that same
  tier") or name the set being counted ("a fourth `SessionStart` hook, alongside
  X, Y and Z").

- 🔵 **suggestion** (confidence: medium) — **Requirements: R9**
  R9 makes documentation a deliverable — "The documentation then gives a
  one-line `ln -s` from a `PATH` directory the user already owns" — but never
  identifies which document, and the actor is elided by the definite article.
  The Context section similarly promises terminal invocation will be
  "documented" without naming an artefact. By contrast, every other requirement
  in the item names its target file or directory precisely.
  **Impact**: The implementer must guess the destination (README, a skill, a
  docs page, the plugin marketplace description), and a reviewer has no named
  artefact to check the deliverable against.
  **Suggestion**: Name the document (and section, if it exists) that R9 must
  update, in the same style as the other requirements' file references.

- 🔵 **suggestion** (confidence: medium) — **Assumptions / Requirements: R7**
  A small set of terms and references are used without definition or a
  resolvable link: "RCA" (Assumptions and Drafting Notes — the acronym is never
  expanded, though the underlying document is linked by path in References),
  "ADR-0048" (cited in R7 as the authority for "lint guards are Python invoke
  tasks" with no path or title), "E2E" (rename-set table), and "verify shim"
  (used in R9 and in the `--fail-safe` Acceptance Criterion; only recoverable
  from the quoted 0164 commit title). "XDG" in Deliberately unchanged is
  borderline but is at least anchored to a quoted code comment.
  **Impact**: Low individually, but ADR-0048 is doing real work — it is the sole
  justification for R7 not being a Rust check — so a reader who cannot locate it
  cannot evaluate that decision.
  **Suggestion**: Expand "RCA" on first use, give ADR-0048 a path (or title) as
  the other references have, and add a one-clause gloss for "verify shim" at its
  first appearance.

### Completeness

**Summary**: This is an unusually complete bug work item: every expected section
is present and substantively populated, the reproduction gives command,
environment version and byte-identical actual output alongside a four-row
Expected-vs-Actual table, and ten numbered requirements name concrete file and
line targets backed by exhaustive rename tables in Technical Notes. Twelve
Given/When/Then acceptance criteria cover most requirements and are explicitly
split into automated and manual gates. The main completeness gap is the
user-facing documentation deliverable introduced by R9 — declared in the Summary
as part of the fix but never located, outlined, or given an acceptance criterion
— with smaller gaps around unrouted open questions and prevention
recommendations from the source research that are neither adopted nor explicitly
excluded.

**Strengths**:

- Full section set for a bug kind — Summary, Context, Requirements (with
  dedicated Reproduction and Expected-vs-Actual subsections), Acceptance
  Criteria, Open Questions, Dependencies, Assumptions, Technical Notes, Drafting
  Notes, References — with no empty or placeholder-only section.
- Reproduction is exact and self-contained: the precise `env -u` command, the
  byte-identical stderr, the two live-session failure forms, and the environment
  version (Claude Code v2.1.220), so a reader can reproduce without follow-up.
- Expected-vs-Actual is tabulated across all four affected invocation surfaces
  (`!` preprocessor site, Bash-tool call, terminal invocation, `--fail-safe`),
  making the defect's shape unambiguous.
- Requirements R1–R10 each name concrete targets (files, line numbers, edit
  counts), and Technical Notes supplies the complete rename set as tables
  including out-of-tree writers and explicitly exempt sites — an implementer has
  a work list, not a description.
- Twelve Given/When/Then acceptance criteria, explicitly labelled Automated vs
  Manual, including a meta-criterion that the R5 regression test must fail
  before the fix and pass after.
- Assumptions is populated with evidence and honesty about strength — notably
  recording that the `allowed-tools` substitution assumption came back weaker on
  re-check, and why it cannot be confirmed from a broadly-permissioned session.
- Drafting Notes record decision history and name the separable cut lines
  (R1+R2 as the minimum shippable fix, R9 as the largest separable addition), so
  a reader understands how the scope was assembled.
- Frontmatter is complete and coherent: `kind: bug`, `status: draft`,
  `priority: high`, plus `source`, `relates_to`, `tags` and `schema_version`.

**Findings**:

- 🟡 **major** (confidence: high) — **Requirements: R9 / Acceptance Criteria** —
  *Documentation deliverable for terminal invocation is unlocated and has no
  acceptance criterion*
  The work item makes user-facing documentation a defining part of the fix — the
  Summary says terminal invocation "becomes a supported, documented and tested
  surface", the Context repeats "documented, given a stable non-version-pinned
  path, and covered by a test", and R9 says "the documentation then gives a
  one-line `ln -s` from a `PATH` directory the user already owns". But no
  requirement says *which* artefact carries that documentation (README, a docs
  page, plugin install instructions, a skill), what it must cover beyond the
  single `ln -s` line (e.g. the per-channel shim path, the mid-session upgrade
  caveat recorded in Technical Notes), or who reads it. None of the twelve
  acceptance criteria mention documentation at all, so the one requirement whose
  output is a written artefact is the only one with no verification.
  **Impact**: An implementer can complete every acceptance criterion —
  including the hook, both symlink hops, and the tests — while shipping no
  documentation, leaving the "supported surface" claim unmet and the feature
  undiscoverable; alternatively they must stop and ask where to write it.
  **Suggestion**: Name the documentation target and required content in R9 (file
  path plus a bullet list of what it must state), and add an acceptance
  criterion asserting the documented `ln -s` recipe exists at that location and
  reflects the two-hop chain.

- 🔵 **minor** (confidence: medium) — **Open Questions** — *The dual-channel
  naming question has no resolution route, yet gates R9's documentation*
  The second Open Question — whether to document a way to have both marketplace
  channels available under distinct names (`accelerator` for stable,
  `accelerator-pre` for prerelease) or "stay silent" — is stated without a
  resolution route, unlike the first open question, which explicitly designates
  the manual pre-release acceptance criterion as its resolver. Because the
  answer determines what R9's documented `ln -s` recipe says, it needs settling
  before that deliverable can be written.
  **Impact**: An implementer reaching R9's documentation step has no recorded
  decision and no named way to obtain one, so the question is likely to be
  resolved silently and inconsistently at implementation time.
  **Suggestion**: Either record the decision inline (document both names, or
  deliberately stay silent) or state how and by whom it will be resolved,
  matching the treatment given to the substitution question.

- 🔵 **minor** (confidence: medium) — **Requirements / Open Questions** — *No
  requirement or scope statement for the remediation the substitution question
  would trigger*
  The first Open Question states that if `${CLAUDE_PLUGIN_ROOT}` no longer
  substitutes into `allowed-tools` Bash rules, "all 43 skills prompt even after
  this fix, and the rules must move to `${CLAUDE_SKILL_DIR}`-relative paths" —
  and that this is discoverable only via the manual pre-release acceptance
  criterion, i.e. after the rest of the work is done. Neither the Requirements
  section nor the "Deliberately unchanged" list says whether that
  `${CLAUDE_SKILL_DIR}` migration is in scope for this item or a follow-up, and
  no acceptance criterion covers it.
  **Impact**: If the manual check fails, the implementer has no recorded
  instruction on whether to extend this item or raise a new one, at the point
  where the release is already gated.
  **Suggestion**: Add one line to Requirements or Dependencies stating
  explicitly that the `${CLAUDE_SKILL_DIR}` remediation is out of scope and
  becomes a separate item if the manual check fails (or in scope, with a matching
  requirement).

- 🔵 **minor** (confidence: medium) — **Requirements: R7 / Deliberately
  unchanged** — *Two prevention recommendations from the source research are
  neither adopted nor explicitly excluded*
  The referenced source research
  (`meta/research/issues/2026-07-26-cli-requires-claude-plugin-root-env-var.md`,
  Prevention section) makes five prevention recommendations. Three are carried
  into requirements (lint guard → R7, real-invocation-environment test → R5,
  `mise.local.toml` → R6), but two are absent from the work item entirely:
  stating the convention once that *no plugin entry point* may require
  `CLAUDE_PLUGIN_ROOT` from its environment (R7's guard covers only `cli/`, a
  narrower surface), and extending the `allowed-tools` conformance check to
  cover env-assignment prefixes as well as `bash`/`sh` wrappers. The work item
  has a "Deliberately unchanged" section that lists four excluded items, so the
  omission reads as an oversight rather than a decision.
  **Impact**: Recommendations from the item's own root-cause analysis are
  silently dropped, and a reader cannot tell whether they were rejected or
  forgotten — inviting the same class of leak at a non-`cli/` entry point.
  **Suggestion**: Either add them as requirements or list them under
  "Deliberately unchanged" with a one-line reason, so every prevention
  recommendation from the source research is visibly accounted for.

### Dependency

**Summary**: Intra-item coupling is unusually well mapped: R3's reader/writer
lockstep rule (with the full out-of-tree writer table), the
bootstrap-writes/Rust-reads directionality contract, and R10's dependence on R9
and R1's loop form are all stated explicitly. The gap is at the boundary —
Dependencies says "Blocked by: none" while the item's success turns on
unresolved Claude Code behaviour (`allowed-tools` substitution,
`${CLAUDE_PLUGIN_DATA}` semantics and its version floor) and on a launcher
binary that is fetched, verified and cached separately from the bootstrap being
renamed. Contingent follow-on work implied by the item's own split points and
open questions is likewise uncaptured, so a partial ship or a failed pre-release
probe leaves no tracked successor.

**Strengths**:

- R3 states the ordering constraint by direction rather than by directory and
  names the exact reason it cannot be split ("Renaming the `cli/` readers alone
  breaks the dev harness and the shell suites"), backed by a complete site table
  including the four out-of-tree writers in `tasks/` and
  `tests/integration/dev/` — this is the coupling most likely to have been
  missed and it is fully captured.
- The adapter-tier exemption is explicit (`hooks/config-detect.sh`,
  `hooks/migrate-discoverability.sh`, `scripts/interactive-harness.sh`,
  `skills/config/migrate/**` and migrations `0001`–`0007`, plus the SKILL.md
  fixture strings), so the rename's blast radius is bounded rather than left to
  the implementer to infer.
- The directionality contract in Technical Notes ("the bootstrap writes it and
  never reads it… the Rust tiers read it and never write it") makes the internal
  coupling between R1, R2 and the test-injection seam explicit, and explains why
  the stale-value hazard applies to `CLAUDE_PLUGIN_ROOT` but not to the new
  variable.
- Dependencies names a concrete downstream consumer with justification ("Blocks:
  the next prerelease. The plugin is substantially unusable as shipped in
  1.24.0-pre.16"), and Related correctly identifies 0164 as the introducing
  change and 0167 as the change that routed the skills through it.
- R10's dependency on R9 is stated in both directions — R9 makes the two-hop
  symlink the documented shape, which is what forces R1's resolution to be a
  loop rather than a single dereference — so neither requirement can be
  implemented in ignorance of the other.
- Assumptions itemises the four `${CLAUDE_PLUGIN_DATA}` behaviours R9 relies on
  and says so outright ("R9 depends on all four"), rather than leaving the
  reliance implicit in the requirement text.

**Findings**:

- 🟡 **major** (confidence: high) — **Dependencies** — *Claude Code's
  behavioural contract is an external dependency but Dependencies records
  "Blocked by: none"*
  This bug work item turns entirely on the behaviour of an external system —
  Claude Code v2.1.220 — yet Dependencies states "Blocked by: none" and names no
  external system at all. The item's own Open Questions and Assumptions record
  that whether `${CLAUDE_PLUGIN_ROOT}` still substitutes into `allowed-tools`
  Bash rules is *unconfirmed on v2.1.220* and that "if it has stopped, all 43
  skills prompt even after this fix", plus three further Claude-Code-owned
  behaviours the fix relies on (substitution into skill content, export to hook
  processes only, mid-session upgrade path staleness).
  **Impact**: A reader scheduling or releasing this work from the Dependencies
  section alone sees an unblocked, self-contained fix, when in fact its success
  is gated on unverified third-party behaviour pinned to a specific Claude Code
  version — the exact class of coupling that should be visible before the work
  starts.
  **Suggestion**: Add an explicit external dependency entry to Dependencies
  naming Claude Code (with the verified version, v2.1.220) and the three
  behaviours depended on — `allowed-tools` substitution of
  `${CLAUDE_PLUGIN_ROOT}` (unconfirmed), skill-content substitution, and
  hook-process export — cross-referencing the Assumptions section rather than
  leaving the coupling discoverable only there.

- 🟡 **major** (confidence: medium) — **Open Questions** — *Contingent follow-on
  work for 43 skills' `allowed-tools` rules has no captured successor*
  The first Open Question states that if `${CLAUDE_PLUGIN_ROOT}` no longer
  substitutes in `allowed-tools` Bash rules, "all 43 skills prompt even after
  this fix, and the rules must move to `${CLAUDE_SKILL_DIR}`-relative paths" — a
  change across 43 SKILL.md files. That contingent work is resolvable only by
  the manual pre-release check, i.e. *after* this item's implementation is
  complete, and no successor work item, Blocks entry, or deferred-work note
  records it.
  **Impact**: If the probe comes back negative at the pre-release gate, a
  43-file follow-on change appears with no tracked home, immediately before a
  release that this item declares itself blocking — the worst possible moment to
  discover unplanned scope.
  **Suggestion**: Record the contingent follow-up explicitly in Dependencies
  (e.g. "Conditionally blocks: an `allowed-tools` rule migration across 43
  skills, required only if the pre-release substitution probe fails"), so the
  branch is visible on the plan before the probe runs.

- 🟡 **major** (confidence: medium) — **Requirements: R9 / Assumptions** — *R9's
  `${CLAUDE_PLUGIN_DATA}` dependency carries no minimum Claude Code version*
  R9 adds a `SessionStart` hook that creates and refreshes a symlink under
  `${CLAUDE_PLUGIN_DATA}`, and Assumptions states that R9 depends on four
  `${CLAUDE_PLUGIN_DATA}` behaviours "all documented" — but the item never
  states which Claude Code version first provides that variable. All of the
  item's evidence was gathered on v2.1.220, while the plugin's declared minimum
  supported Claude Code is an earlier version, so the two are not reconciled
  anywhere in the work item.
  **Impact**: On any Claude Code release predating `${CLAUDE_PLUGIN_DATA}`, the
  variable is unset in the hook environment and `${CLAUDE_PLUGIN_DATA}/bin`
  degenerates to an absolute `/bin` path, so R9 either silently no-ops or writes
  outside plugin-owned space — a version coupling that is invisible to anyone
  planning or releasing the change.
  **Suggestion**: Add the minimum Claude Code version that provides
  `${CLAUDE_PLUGIN_DATA}` to Dependencies (or to the R9 requirement), and state
  whether the plugin's declared floor must be raised as part of this item; add
  an acceptance criterion that the hook is inert rather than destructive when
  the variable is absent.

- 🟡 **major** (confidence: medium) — **Dependencies / Technical Notes: Why the
  export is mandatory** — *Lockstep coupling between the shipped bootstrap and
  the separately-distributed launcher binary is not captured*
  The rename spans two independently distributed tiers: `bin/accelerator` ships
  inside the plugin, while the launcher (and through it the visualiser server)
  is fetched, verified against a signed manifest, and cached by the version read
  from `.claude-plugin/plugin.json`. After R1–R3 the bootstrap exports only
  `ACCELERATOR_PLUGIN_ROOT`, whereas a launcher built before the rename reads
  only `CLAUDE_PLUGIN_ROOT` — and the item's own Technical Notes establish that a
  launcher without a root does not error but silently drops the plugin-default
  template tier (empty table, exit 0). Dependencies records the downstream
  release ("Blocks: the next prerelease") but not the upstream requirement that a
  matching launcher artifact be produced and cache-resolved in the same release.
  **Impact**: A version bump that ships the new bootstrap against a stale or
  independently-versioned launcher artifact reproduces exactly the
  silent-wrong-answer failure mode the item was written to avoid, and the item
  explicitly defers making that degradation visible ("fold in only if cheap").
  **Suggestion**: Add a Dependencies entry for the release/distribution coupling
  — the renamed launcher and visualiser-server binaries must ship in the same
  version as the renamed bootstrap, with the version-keyed cache guaranteeing no
  pre-rename launcher is reachable — and state whether any cache invalidation is
  required beyond the version bump.

- 🟡 **major** (confidence: medium) — **Requirements: R4** — *R4's negative test
  seam depends on a tier that R1 forbids from reading the variable*
  R4 moves the negative seam in
  `skills/work/scripts/test-work-item-scripts.sh`
  (`CLAUDE_PLUGIN_ROOT="/nonexistent"`, forcing the hardcoded-fallback path)
  onto `ACCELERATOR_PLUGIN_ROOT`, described as "four one-line edits". But R1
  makes the bootstrap "write-only for `ACCELERATOR_PLUGIN_ROOT` — it never reads
  it", and an acceptance criterion requires that a wrong
  `ACCELERATOR_PLUGIN_ROOT` in the environment be *ignored* in favour of
  self-location. Technical Notes reconciles this by asserting the injection seam
  "target[s] the launcher and server, not the bootstrap", yet the source
  research states this particular shell suite reaches the CLI through the
  bootstrap; the work item never says which tier consumes the seam value in that
  suite, nor that it must route via `ACCELERATOR_BIN` to bypass the bootstrap.
  **Impact**: If the seam does traverse the bootstrap, R1's unconditional
  self-location overwrites the injected `/nonexistent` value, the real templates
  are found, and the four assertions R4 was written to preserve break — a
  mid-implementation surprise in a requirement budgeted as trivial.
  **Suggestion**: State in R4 which tier consumes the seam variable and, if it
  is reached through the bootstrap, note the additional edit required (e.g.
  routing that call site via `ACCELERATOR_BIN`) so the coupling between R4 and
  R1's write-only rule is explicit.

- 🔵 **minor** (confidence: medium) — **Drafting Notes / Dependencies** —
  *Named split points leave no captured home for the deferred remainder*
  Drafting Notes offers two explicit split points — "R1+R2 alone would ship a
  correct fix if the next prerelease is urgent — split there if needed" and "If
  scope needs cutting, R9 is the separable piece" — but Dependencies records no
  successor for either path. Nothing states where R3/R7 (the variable purge and
  its lint guard) or R9/R10 (the stable shim and its two-hop test) would land if
  the item ships reduced.
  **Impact**: Exercising a split that the item itself sanctions silently drops
  requirements, because the deferred half has no tracked destination and no
  dependency edge back to the release that motivated the split.
  **Suggestion**: Note in Dependencies that a split ship creates a follow-up
  carrying the deferred requirements (R3/R7 or R9/R10), so the deferred half is
  visible as outstanding work rather than lost in Drafting Notes.

- 🔵 **minor** (confidence: medium) — **Acceptance Criteria** — *New no-env tests
  depend on launcher artifact availability, which is unstated*
  Three new automated checks assert that the bootstrap *succeeds* with both
  variables removed via `env -u`: R5's regression test, R10's two-hop symlink
  test, and the sweep of every `bin/accelerator config *` command extracted from
  `!` blocks in `skills/**/SKILL.md`. Reaching a zero exit means clearing all 14
  bootstrap gates, which per Technical Notes include locating the verify shim and
  release public key, a usable cache dir, and fetch-and-verify of the launcher.
  The work item does not state what supplies the launcher for those tests — a
  network fetch from GitHub Releases, a pre-built local launcher via the gated
  dev override, or a fixture tree.
  **Impact**: If the tests resolve to a real fetch, they couple CI to external
  release-artifact availability; if they rely on a locally built launcher, they
  couple to a build step that must run first — either way an unstated
  prerequisite that surfaces as a flaky or ordering-dependent suite.
  **Suggestion**: State in the acceptance criteria (or Technical Notes) which
  launcher source the new no-env tests use and what must exist before they run,
  so the test-time dependency is explicit rather than discovered when the suite
  is wired into `mise run`.

### Scope

**Summary**: The work item has a genuinely indivisible core — self-location plus
export plus the readers/writers rename must land together, and the item argues
that atomicity convincingly — but it also bundles two independently deliverable
threads: a new user-facing capability (R9/R10: a fourth `SessionStart` hook, a
`${CLAUDE_PLUGIN_DATA}` shim, and user-facing `PATH` documentation) and a
launcher failure-mode change (R8), plus repo hygiene (R6), all under kind `bug`
on the declared critical path to the next prerelease. The Drafting Notes
identify the split points ("R1+R2 alone would ship a correct fix"; "If scope
needs cutting, R9 is the separable piece") but the Requirements and Acceptance
Criteria do not act on them, so the shippable boundary exists only as commentary.
Section coherence is otherwise strong: Summary, Requirements and AC describe the
same three threads with no drift, and each requirement has matching AC.

**Strengths**:

- R3's widening from "under `cli/`" to "readers and writers together, wherever
  they live" is justified by a genuine indivisibility argument — renaming the
  `cli/` readers alone cannot ship green because four out-of-tree writers feed
  them — which is exactly the right reasoning for keeping a large change as one
  unit rather than splitting it artificially.
- The "Deliberately unchanged" section draws an explicit out-of-scope boundary
  (XDG fallback, version source, visualiser fatal exit, empty-table visibility),
  and the "Exempt — do not blanket-rename" list prevents the rename from
  overreaching into the Claude Code adapter tier.
- Standalone packaging is explicitly declared out of scope in the Context "Scope
  decision" paragraph, cleanly separating invocation context from distribution
  shape — a distinction that would otherwise have made this item unbounded.
- Every requirement R1–R10 (except R6) has a corresponding Acceptance Criterion,
  and the Summary names the same three threads the Requirements contain — there
  is no Summary/Requirements/AC scope drift.
- The Drafting Notes are unusually candid about scope trade-offs, recording why
  the item was scoped as bug-plus-refactor, which requirements were promoted
  from Open Questions, and where the item can be cut if needed.

**Findings**:

- 🟡 **major** (confidence: high) — **Requirements (R9, R10)** — *R9/R10 add an
  independent user-facing capability to a critical-path bug fix*
  Work item 0182 is a `bug` fixing `bin/accelerator`'s hard dependency on
  `CLAUDE_PLUGIN_ROOT`, but R9 and R10 additionally introduce a new user-facing
  capability: a fourth `SessionStart` hook that maintains a symlink under
  `${CLAUDE_PLUGIN_DATA}/bin/`, a documented two-hop `PATH` install procedure,
  and end-to-end coverage of that shape. Nothing in the reported failure
  requires it — the item's own Summary joins it with "alongside that", and the
  Drafting Notes call R9 "the largest scope addition in this item" and "the
  separable piece" if scope needs cutting — and it could be built, shipped, and
  rolled back with no effect on R1–R8.
  **Impact**: Dependencies states this item blocks the next prerelease because
  "the plugin is substantially unusable as shipped in 1.24.0-pre.16", so
  bundling a new hook plus documentation (and its own unresolved dual-channel
  naming question in Open Questions) attaches discretionary design and review
  surface to an urgent fix, with a different rollback profile — a hook that
  writes to the filesystem every session is a materially riskier change to back
  out than a self-locating bootstrap.
  **Suggestion**: Split R9 into a follow-on work item covering terminal
  invocation as a supported surface (hook + shim + documentation + dual-channel
  decision), and keep R10's two-hop symlink-resolution test here as insurance
  for user-made links — the Drafting Notes already note that "R10 and the
  symlink chase stand on their own".

- 🟡 **major** (confidence: medium) — **Open Questions** — *Unresolved
  `allowed-tools` substitution question leaves contingent scope unbounded*
  The first Open Question asks whether `${CLAUDE_PLUGIN_ROOT}` still substitutes
  into `allowed-tools` Bash rules on Claude Code v2.1.220, and states that if it
  does not, "all 43 skills prompt even after this fix, and the rules must move to
  `${CLAUDE_SKILL_DIR}`-relative paths". That remediation — editing frontmatter
  across 43+ SKILL.md files and updating `tasks/lint/skill_permissions.py`'s
  `_PLUGIN_PREFIX` model — is not assigned to either in-scope or out-of-scope,
  and the question is resolvable only by the manual pre-release check listed as
  one of this item's own Acceptance Criteria.
  **Impact**: The unit of work has a trapdoor: a negative result at the last
  verification gate either balloons the item by a repo-wide frontmatter
  migration or leaves the stated goal (43 skills usable without prompts) unmet,
  in both cases discovered after the rest of the work is complete and
  immediately before the release this item blocks.
  **Suggestion**: State the boundary explicitly — e.g. "if the substitution has
  stopped, the `allowed-tools` migration is tracked as a separate work item and
  this item ships with the prompt behaviour documented as a known issue" — and
  consider probing the question before starting implementation so the sizing of
  this item is known up front rather than at the pre-release gate.

- 🔵 **minor** (confidence: medium) — **Frontmatter: kind** — *Kind `bug` for ten
  requirements spanning bootstrap, Rust, build system, hooks and docs*
  The item is filed as `kind: bug`, but its ten requirements span a bash
  bootstrap rewrite (R1, R2, R8), a rename across three Rust production sites
  plus nine test/tooling files and four out-of-tree Python writers (R3, R4), a
  new invoke lint task (R7), a new `SessionStart` hook with user-facing
  documentation (R9), new tests (R5, R10), and a repo file deletion (R6). Only
  R1, R2 and R5 close the reported defect; the remainder is a refactor, a
  prevention guard, and a new capability.
  **Impact**: Low direct delivery risk — the work is coherent and single-owner —
  but a `bug` label understates the review, verification and coordination
  footprint, and it obscures the fact that the defect closure and the
  boundary-cleanup goal have different urgency profiles.
  **Suggestion**: Consider filing the defect closure (R1, R2, R5, and the R3
  rename slice required to ship green) as the `bug`, and the boundary-rule work
  (R7, R9, R10, and any remaining purge) as one or more follow-on stories — or,
  if the team's norm is one item per investigation, promote this to a story so
  the sizing signal matches the content.

- 🔵 **suggestion** (confidence: medium) — **Requirements (R6)** — *R6 (remove
  `mise.local.toml`) is orthogonal local-environment hygiene*
  R6 removes `mise.local.toml` from the repo. It touches nothing the other nine
  requirements touch, has no Acceptance Criterion, is contingent on a question
  the item cannot answer itself ("Drop R6 if it is serving another purpose", per
  the Drafting Notes), and is a local developer-environment concern rather than
  part of the shipped fix.
  **Impact**: Minimal — but carrying a contingent, unverified deletion inside an
  urgent fix means either the item cannot be closed until the maintainer answers
  a side question, or the deletion ships unnoticed and silently changes other
  developers' local setups.
  **Suggestion**: Either resolve the contingency before starting (converting R6
  into an unconditional one-line change with its own AC), or move it out to a
  chore — its value is preventing future masking, which does not need to land in
  the same change as the fix.

- 🔵 **suggestion** (confidence: medium) — **Requirements (R8)** — *R8
  (`--fail-safe`-aware bootstrap) is separately deliverable, though defensibly
  bundled*
  R8 changes the bootstrap's abort path so a bootstrap-tier failure under
  `--fail-safe` exits 0 with empty stdout. The source research classifies this
  as "a separable robustness question", and it is independently deliverable and
  rollbackable: it only takes effect on failures that R1/R2 are intended to
  eliminate.
  **Impact**: Low. The inclusion is well argued — R8 is the amplifier that
  turned one broken command into 43 skills failing at load — so keeping it here
  is a reasonable judgement, but it is a third distinct thread in an item
  already carrying a fix and a refactor.
  **Suggestion**: Keep R8 if the item stays as one unit (the argument for it is
  sound), but if the item is split for urgency, place R8 with the defect closure
  rather than the boundary-cleanup work, since it is the change that limits
  blast radius the next time a bootstrap gate fails.

### Testability

**Summary**: Work item 0182 is unusually strong on reproduction specification —
the bug's trigger, exact command, byte-identical stderr, and a four-row
Expected-vs-Actual table make the defect itself unambiguous — and AC12's
fail-before/pass-after demand is exactly the falsifiability check most bug items
omit. The critical weakness is that the two criteria carrying the most
verification load (the automated `!`-block sweep and the primary
`config instructions commit --fail-safe` criterion) assert only exit 0, which R8
and the launcher's documented silent degradation both make achievable while the
bug is unfixed — the flagship regression test cannot fail. Secondary gaps: one
unbounded whole-filesystem negative criterion, a direct contradiction between
AC4 and R4 over whether an ambient `ACCELERATOR_PLUGIN_ROOT` is honoured, and
four requirements (R3's out-of-tree writers, R4, R6, R9's diagnostic output)
whose only verification is the blanket `mise run` criterion.

**Strengths**:

- The Reproduction subsection gives an exact `env -u` command, the
  byte-identical stderr, and both live failure shapes (`!` site abort and
  Bash-tool permission prompt) — the bug's trigger, expected result, and actual
  result are all fully specified, which is the hardest thing for a bug item to
  get right.
- The Expected vs actual table enumerates four distinct invocation contexts
  (`!` site, Bash tool, terminal, `--fail-safe`) with a per-context expected
  outcome, so each context has its own verifiable target rather than one vague
  "it works".
- AC12 ("the regression test from R5 fails against the current
  `bin/accelerator` and passes after the fix") is an explicit falsifiability
  requirement — it forces the test to be proven capable of detecting the defect
  rather than merely passing.
- AC2 anticipates the false-pass mode identified in the research (Hypothesis 4)
  and closes it explicitly: "the plugin-default templates are listed — **not** an
  empty table at exit 0". This is the pattern the weaker criteria should copy.
- AC10 specifies the manual check's preconditions precisely enough to be
  repeatable by someone other than the author: clean install of the release
  artifact, no `mise.local.toml`, neither variable exported, permission mode
  `default`, no broad Bash allow rules, one skill per `!`-site family.
- AC9 defines a mechanical extraction procedure over an enumerable set (every
  `bin/accelerator config *` command in a `!` block of any `skills/**/SKILL.md`),
  so its scope is bounded and machine-derivable rather than aspirational.

**Findings**:

- 🔴 **critical** (confidence: high) — **Acceptance Criteria (criterion 1 and the
  "Automated" criterion)** — *Exit-0-only assertions are tautological for the
  `--fail-safe` command family*
  Work item 0182's first criterion ("when `bin/accelerator config instructions
  commit --fail-safe` is run by absolute path, then it exits 0") and its
  flagship **Automated** criterion ("every `bin/accelerator config *` command
  extracted from a `!` block … run with both variables removed via `env -u`,
  exits 0") assert nothing but the exit status — yet the item's own R8 makes any
  bootstrap-tier abort exit **0** when `--fail-safe` is in argv, and the
  referenced research measured that the launcher's `config` path already exits 0
  with empty output when no root is present (`config instructions commit
  --fail-safe` "likewise exits 0 with no output"; `config templates list`
  returns an empty table at exit 0). Every command in that family carries
  `--fail-safe` by lint contract, so both criteria are satisfied by R8 alone,
  with R1's self-location broken and R2's export absent.
  **Impact**: The single automated check intended to prove all 43 broken skills
  are fixed cannot fail — it would pass against a fix that only added the
  fail-safe exit path, and would continue passing if the export (R2) later
  regressed, which is precisely the silent-wrong-answer mode the research called
  out as the reason a bootstrap-only fix is dangerous.
  **Suggestion**: Make both criteria assert an output signal, not just status:
  for criterion 1, "exits 0 **and** stdout contains the commit instructions
  content (non-empty), with no diagnostic line on stderr"; for the automated
  sweep, "exits 0, stdout is non-empty, and stderr is empty" — since the
  fail-safe degradation is defined by empty stdout plus one stderr line
  (criterion 5), those three together distinguish real success from degraded
  success.

- 🟡 **major** (confidence: high) — **Acceptance Criteria (hook
  filesystem-containment criterion)** — *Whole-filesystem negative criterion is
  unbounded and unverifiable as written*
  Work item 0182 requires: "Given the hook has run, when the filesystem outside
  `${CLAUDE_PLUGIN_DATA}` is inspected, then nothing was created or modified —
  in particular no entry in `~/.local/bin` or any other `PATH` directory."
  Inspecting "the filesystem outside `${CLAUDE_PLUGIN_DATA}`" has no terminating
  procedure, and "any other `PATH` directory" varies per machine, so no test can
  conclusively demonstrate the criterion; a verifier can only spot-check and
  claim it passed.
  **Impact**: An always-claimable criterion provides no verification value for
  R9's central safety property (the hook must not write user-general space),
  which the Technical Notes treat as the deciding design constraint.
  **Suggestion**: Bound it to a defined procedure — e.g. "run the hook with
  `HOME` and `${CLAUDE_PLUGIN_DATA}` pointed at a temp tree; assert the only
  paths created or modified under that `HOME` are inside the plugin data
  directory (snapshot the tree before/after and diff), and assert
  `<HOME>/.local/bin` does not exist." That is enumerable and produces a
  definitive pass/fail.

- 🟡 **major** (confidence: medium) — **Acceptance Criteria (stale-variable
  criterion) vs Requirements R4** — *AC4 and R4 demand contradictory behaviour
  for an ambient `ACCELERATOR_PLUGIN_ROOT`*
  Work item 0182's criterion "Given a stale or wrong `CLAUDE_PLUGIN_ROOT`
  **or** `ACCELERATOR_PLUGIN_ROOT` is present in the environment … both values
  are ignored and self-location wins" contradicts R4, which moves the negative
  seam at `skills/work/scripts/test-work-item-scripts.sh:1053,1086,1096,1106`
  (`CLAUDE_PLUGIN_ROOT="/nonexistent"`, forcing the hardcoded-fallback path)
  onto the new variable. Per the referenced research, that seam reaches the CLI
  *through the bootstrap* (via that script's line 53), so if the bootstrap
  ignores and overwrites an inbound `ACCELERATOR_PLUGIN_ROOT`, the injected
  `/nonexistent` never reaches the launcher and the four assertions it supports
  stop forcing the fallback. The item's "Precedence and directionality" note
  excuses this by saying injection tests "target the launcher and server, not the
  bootstrap", which does not hold for the R4 seam.
  **Impact**: A verifier cannot satisfy both statements — either the
  stale-variable criterion fails or R4's four assertions become vacuous (they
  would pass by finding real templates, silently no longer testing the
  fallback), and the ambiguity will be resolved arbitrarily at implementation
  time.
  **Suggestion**: Decide and state the seam explicitly: either narrow the
  criterion to "an ambient `CLAUDE_PLUGIN_ROOT` is ignored; an ambient
  `ACCELERATOR_PLUGIN_ROOT` is overwritten by the bootstrap" and give R4's seam a
  distinct bootstrap-transparent variable (e.g. `ACCELERATOR_TEMPLATE_ROOT` or
  invoking the launcher directly), or restate R4 as "the seam invokes the
  launcher directly, bypassing the bootstrap" and add a criterion asserting the
  four fallback assertions still fail when the fallback is removed.

- 🟡 **major** (confidence: medium) — **Requirements: R5** — *R5 names neither
  the command nor the observable that constitutes "success"*
  Work item 0182's R5 says "A regression test runs `bin/accelerator` with
  **both** variables removed via `env -u` and no environment beyond an absolute
  path, asserting success", without naming the subcommand or defining "success".
  AC12 makes this test the item's falsifiability anchor ("fails against the
  current `bin/accelerator` and passes after the fix"), so its observable
  matters: if the chosen command carries `--fail-safe`, "success" reduces to
  exit 0, which R8's new fail-safe abort path also produces.
  **Impact**: The one assertion the item claims "would have caught the bug at
  0164" may be written in a form that only catches the pre-R8 loud failure,
  leaving no durable guard against the export (R2) regressing.
  **Suggestion**: Specify the command and the observable in R5 — e.g. "runs
  `bin/accelerator config templates list` (no `--fail-safe`) and asserts exit 0
  with a row whose Source column is `plugin default`", which is falsifiable both
  before the fix (hard error today) and after any future loss of the export.

- 🔵 **minor** (confidence: high) — **Acceptance Criteria (two-hop symlink
  criterion)** — *Symlink criterion names real user paths and an unobservable
  internal outcome*
  Work item 0182's symlink criterion is phrased as "Given `bin/accelerator` is
  invoked through a two-hop symlink chain on `PATH`
  (`~/.local/bin/accelerator` → the `${CLAUDE_PLUGIN_DATA}` shim → the
  version-pinned root), when it resolves its root, then it resolves to the real
  installation directory and finds `.claude-plugin/plugin.json`." The literal
  `~/.local/bin` and real `${CLAUDE_PLUGIN_DATA}` paths make a hermetic test
  impossible (an automated run must not create entries in the developer's own
  `PATH` directory), and "resolves to the real installation directory" is an
  internal derivation with no stated external observable. R10 states the same
  shape more neutrally ("a symlink outside the installation root, pointing at a
  second symlink").
  **Impact**: As written the criterion either drives a test that mutates the
  developer's home directory or is verified by inspection only, and "resolves
  correctly" can be claimed without a defined check.
  **Suggestion**: Restate using temp-dir paths and an observable: "invoked
  through `<tmp>/userbin/accelerator` → `<tmp>/plugin-data/bin/accelerator` →
  `<fixture-root>/bin/accelerator`, the command exits 0 and its output reflects
  the fixture root's `plugin.json` (e.g. the fixture version, or a fixture-only
  plugin-default template), not the symlink parents."

- 🔵 **minor** (confidence: medium) — **Acceptance Criteria (SessionStart shim
  criteria) and Requirements: R9** — *Hook criteria give no injection seam, so
  their verification procedure is undefined*
  Work item 0182's SessionStart criteria ("Given a `SessionStart` has run…", and
  "when the installation version changes, the next `SessionStart` re-points it")
  require a verifier to stand up a hook environment — a controllable
  `${CLAUDE_PLUGIN_DATA}`, a `${CLAUDE_PLUGIN_ROOT}` for two distinct fake
  versions, and a way to run the hook out-of-band — but neither R9 nor the
  criteria state that those inputs are overridable, and the Assumptions section
  notes `${CLAUDE_PLUGIN_DATA}` is supplied by Claude Code and "created on first
  reference".
  **Impact**: Without a stated seam, the version-change half of the criterion is
  likely to be verified by manual eyeballing across a real upgrade — i.e.
  deferred indefinitely and never regression-guarded.
  **Suggestion**: Add to R9 that the hook takes both `${CLAUDE_PLUGIN_DATA}` and
  `${CLAUDE_PLUGIN_ROOT}` from its environment with no hardcoded home-relative
  paths, and restate the criterion as a procedure: "running the hook with
  `CLAUDE_PLUGIN_DATA=<tmp>/data` and `CLAUDE_PLUGIN_ROOT=<tmp>/v1` creates
  `<tmp>/data/bin/accelerator → <tmp>/v1/bin/accelerator`; re-running with
  `…/v2` re-points it, and a pre-existing link at `<tmp>/userbin/accelerator →
  <tmp>/data/bin/accelerator` still executes."

- 🔵 **minor** (confidence: medium) — **Acceptance Criteria vs Requirements (R3,
  R4, R6, R9)** — *Four requirements have no criterion beyond the blanket
  `mise run` bullet*
  Several of work item 0182's requirements are not covered by any criterion that
  could distinguish done from not-done: R3's four out-of-tree writers
  (`tasks/dev.py`, `tasks/shared/dev/circus.py`, `tasks/test/helpers.py`,
  `tests/integration/dev/dev_integration_driver.py`) and its two renamed *error-
  message contracts* (`launcher/tests/version.rs:179` and
  `cache_root.rs:132,134` assert the literal message text); R4's seam migration;
  R6's removal of `mise.local.toml`; and R9's "prints the path it refreshed". The
  `grep -r 'CLAUDE_' cli/` criterion is scoped to `cli/` only, so the sole
  verification for these is "`mise run` exits 0" — which is a green-suite check,
  not evidence the specific behaviours changed.
  **Impact**: A change could satisfy every stated criterion while leaving
  `mise.local.toml` in place (which the item itself identifies as the factor
  masking this bug class locally) or leaving the renamed diagnostic text
  unpinned, so the item's intent is only partly captured by its criteria.
  **Suggestion**: Add two cheap criteria — "no tracked file outside `hooks/`,
  `scripts/interactive-harness.sh`, `scripts/test-design.sh`,
  `skills/config/migrate/**` and the documented fixture strings writes or reads
  `CLAUDE_PLUGIN_ROOT`; `mise.local.toml` is absent from the repo" and "the
  bootstrap-tier diagnostic and the launcher/`cache_root` error messages name
  `ACCELERATOR_PLUGIN_ROOT`, asserted by the existing message-text tests" — plus
  one asserting the hook prints the refreshed path.

- 🔵 **minor** (confidence: medium) — **Acceptance Criteria (grep criterion)** —
  *`grep -r 'CLAUDE_' cli/` as literally stated will scan build output and
  `node_modules`*
  Work item 0182 states the criterion as a literal command: "`grep -r 'CLAUDE_'
  cli/` returns no matches". Run verbatim, that traverses `cli/target/`
  (compiled artefacts and vendored dependency sources, which retain the old
  string in any stale build) and `cli/visualiser/frontend/node_modules/`, so the
  criterion's outcome depends on whether the tree happens to be clean rather
  than on the source state.
  **Impact**: The criterion may fail spuriously on a normal working tree or, if
  the verifier silently adds excludes, be checked by a different procedure than
  the R7 lint guard applies — leaving the boundary rule's actual coverage
  undefined.
  **Suggestion**: State the scope rather than the raw command: "no tracked
  source file under `cli/` contains the string `CLAUDE_`, as enforced by the R7
  lint task (which honours `.gitignore` and skips `target/` and
  `node_modules/`)", and add that the guard has a negative test proving it fails
  on a reintroduced reference.

- 🔵 **suggestion** (confidence: medium) — **Acceptance Criteria (manual
  pre-release criterion) and Open Questions** — *Manual check has no recorded
  output and no defined disposition on failure*
  Work item 0182's manual pre-release criterion is well specified as a
  *procedure* but says nothing about what artefact records the result, and the
  Open Questions section makes this same check the sole resolution path for the
  `${CLAUDE_PLUGIN_ROOT}`-in-`allowed-tools` substitution question — whose
  failure mode ("all 43 skills prompt even after this fix, and the rules must
  move to `${CLAUDE_SKILL_DIR}`-relative paths") has no corresponding
  requirement or criterion. The skill selection is also left open ("one
  `integrations/*`", "one `planning/*`"), so two verifiers may exercise
  different surfaces.
  **Impact**: A criterion whose outcome is not written down anywhere can be
  marked passed without evidence, and if it fails there is no stated disposition
  — the item could be closed with the substitution assumption unresolved.
  **Suggestion**: Name the specific skills to invoke, state where the result is
  recorded (e.g. a validation note under `meta/` or a comment on the item
  resolving the Open Question), and add one line stating the disposition if the
  prompt appears: the `allowed-tools` migration becomes a follow-up work item and
  this item does not close as fixed.

- 🔵 **suggestion** (confidence: low) — **Acceptance Criteria (templates-list
  criterion)** — *Tighten "the plugin-default templates are listed" to a named
  expected row*
  Work item 0182's criterion "the plugin-default templates are listed — **not**
  an empty table at exit 0" already rules out the known false pass, but "the
  plugin-default templates" leaves the expected set unstated, so a partial or
  wrong-source listing could be argued as passing.
  **Impact**: Minor — the criterion is verifiable in spirit, but a table with one
  unrelated row would satisfy it as written.
  **Suggestion**: Pin one concrete row, e.g. "the output contains a row for
  `adr` with Source `plugin default` and a Path under the resolved installation
  root", which is both specific and stable against future template additions.

## Re-Review (Pass 2) — 2026-07-26

**Verdict:** APPROVE

*Verdict history: the lenses' assessment at re-review time was REVISE (five new
majors, listed below). Every one was addressed in the follow-up edit recorded in
the item's Drafting Notes, and the maintainer approved the result on
2026-07-26 — so the item's standing verdict is APPROVE. The findings below are
retained as the record of what was fixed, not as outstanding work.*

All five lenses re-run against the revised item. The pass-1 critical is gone and
the headline contradiction is resolved, but the revision introduced three new
majors of its own and the lenses surfaced three substantive errors the first pass
had missed — including that R4 was wrong about its own mechanism. All were
addressed in a further edit immediately after this pass; the verdict records the
state at re-review, not after those fixes.

### Previously Identified Issues

- 🔴 **Testability**: exit-0-only assertions are tautological — **Resolved.** Every
  criterion now pairs exit status with an output signature, and the preamble states
  why.
- 🟡 **Clarity/Dependency/Testability**: R4's seam unsatisfiable against R1/AC4 —
  **Resolved, then corrected.** The contradiction is gone, but the replacement
  mechanism was itself wrong (see New Issues).
- 🟡 **All four other lenses**: `allowed-tools` question unbounded — **Resolved.**
  R11 probes before implementation; the migration is out of scope in both branches
  with a conditional Dependencies edge. (The probe *procedure* was wrong — see New
  Issues.)
- 🟡 **Completeness/Clarity**: R9's documentation unlocated — **Partially
  resolved.** R9a names `docs/internals.md` and its required content, but no
  acceptance criterion covered it until the post-pass fix.
- 🟡 **Scope**: R9/R10 bolted onto a critical-path fix — **Still present, by
  decision.** The maintainer kept R9 in scope; the lens re-raised it and the gaps
  were closed instead.
- 🟡 **Testability/Clarity**: hook containment unbounded, "the hook" ambiguous —
  **Resolved.** Hook named `shim-refresh.sh`; containment is a temp-tree snapshot
  diff scoped to it alone.
- 🟡 **Testability/Clarity**: R5 under-specified — **Resolved.** Pinned to
  `config templates list` without `--fail-safe`, asserting a `plugin default` row.
- 🟡 **Dependency**: "Blocked by: none" with unrecorded external couplings —
  **Resolved for content, broken for direction.** The four Claude Code behaviours
  and the launcher lockstep are recorded, but the new closure clause contradicted
  "Blocked by: none" (see New Issues).
- 🟡 **Dependency**: no `${CLAUDE_PLUGIN_DATA}` version floor — **Partially
  resolved.** Recorded as an open question with an inert-when-unset guard, but had
  no requirement or criterion until R12 was added post-pass.
- 🔵 All ten minors and six suggestions — **Resolved**, except the `kind: bug`
  sizing signal (declined by decision) and two residues the pass caught: "tier" as
  a layer synonym survived in the Expected-vs-actual table, and Blast radius still
  said "the two `hooks/` entry points" against four enumerated hooks.

### New Issues Introduced

- 🟡 **Testability**: "empty stderr" assertions contradict the gated dev-launcher
  override the same criteria mandate — that override warns on stderr every
  invocation, so the flagship criteria could not pass. Fixed: the assertion is now
  "no `accelerator:` diagnostic line on stderr".
- 🟡 **Clarity/Testability**: the repo-wide purge criterion omitted
  `skills/**/SKILL.md` and `agents/**`, whose 410 `${CLAUDE_PLUGIN_ROOT}`
  substitution tokens R11 deliberately keeps — unsatisfiable as written. Fixed: a
  mechanical `grep -rl` with the full exemption set, and "reads" defined as
  process-environment access.
- 🟡 **Clarity/Dependency/Scope**: closure was stated in both directions —
  "Blocked by: none" beside "does not close until the successor ships". Fixed:
  closes on its own criteria, with the prerelease gated instead.
- 🟡 **Testability/Dependency**: bootstrap-level criteria omitted the launcher and
  fixture preconditions (14 gates, no shim or key in a repo checkout), so a red
  result would not have been attributable. Fixed: shared preconditions stated once,
  plus the build-order dependency.
- 🟡 **Completeness/Testability**: R9a's documentation, the `hooks.json`
  registration, R11's recorded result, and the floor decision had no criteria — an
  unregistered hook would have passed every hook criterion. Fixed: four criteria
  added.

### Substantive Errors Found (not introduced by pass 1)

- **R4 was wrong about its own mechanism.**
  `work-item-template-field-hints.sh` never reads `CLAUDE_PLUGIN_ROOT`; it
  self-locates and shells out, and the seam works only because the *bootstrap*
  rejects `/nonexistent` as a directory. Renaming the variable would not restore it,
  and neither would the pass-1 fix of injecting into the launcher, since a rootless
  launcher exits 0 and the fallback never fires. Now `ACCELERATOR_BIN` points at a
  nonexistent binary, with a criterion proving the fallback is still exercised.
- **R11's probe could not answer its own question.** Running the manual check early
  proves nothing while all 43 skills abort at load before any Bash-tool call. The
  probe now addresses the matcher directly, with both outcomes distinguishable under
  the broken bootstrap.
- **The sanctioned urgency split would have shipped a dead export.** R2 exports the
  new name; R3 teaches the launcher to read it. The split lines are now exhaustive
  and record that R3's reader half cannot be deferred.

### Assessment

The item is materially stronger than at pass 1 and the fixes applied after this
pass close every finding that is a defect rather than a decision. Two open items
remain, both maintainer calls rather than quality problems: whether to land R6
standalone before implementation (the file masks this bug class in every local run
while the work proceeds), and whether the urgent repair should ship ahead of R9's
terminal surface. Two determinations — R11's probe and R12's floor check — must be
performed before implementation, and both are now specified precisely enough to
yield an unambiguous answer. A third pass is unlikely to be worth its cost; the
remaining risk sits in execution, not specification.
