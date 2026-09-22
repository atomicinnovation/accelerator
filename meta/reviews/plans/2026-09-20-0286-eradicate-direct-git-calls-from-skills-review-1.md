---
type: "plan-review"
id: "2026-09-20-0286-eradicate-direct-git-calls-from-skills-review-1"
title: "Plan Review: Eradicate Direct Git Calls From Skills Implementation Plan"
date: "2026-09-20T22:58:04+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-09-20-0286-eradicate-direct-git-calls-from-skills"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["correctness", "compatibility", "architecture", "test-coverage", "code-quality", "documentation", "standards", "usability"]
review_number: 1
review_pass: 3
tags: ["vcs", "skills", "cli"]
last_updated: "2026-09-22T08:37:27+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Eradicate Direct Git Calls From Skills Implementation Plan

**Verdict:** REVISE

The plan is well-grounded — it corrects three inaccuracies inherited from the
work item, mirrors existing crate patterns faithfully (the `Root` handler over
`detect.rs`'s direct-probe-port style, the clap derive shape, the 80-column
`concat!` additions), and makes a sound `discover`-over-`repository_root`
design decision with a discriminating test to pin it. But the load-bearing
mechanism — the diff-range idiom that `validate-plan`'s evidence depends on — is
incorrect: the jj revset `fork_point(trunk())` cannot express the working-copy-
vs-trunk divergence point, so the jj diff genuinely differs from the git form
and breaks AC5, and both renderings assume a trunk literally named `main`. Layered
on top are an internal contradiction in Phase 5 (its replacement text reintroduces
the very `git config` token its own gate forbids), a colocated author-resolution
regression, and a cluster of "documented but unenforced / verified only manually"
gaps that leave the plan's headline invariants without CI teeth.

### Cross-Cutting Themes

- **The diff-range idiom is the weakest, most correctness-sensitive part**
  (flagged by: correctness, compatibility, usability, architecture,
  documentation) — the jj revset is semantically wrong (critical), the git side
  hard-codes `main`, the two backends differ on whether uncommitted changes are
  included, the jj form is under-specified for the model to reconstruct, and the
  whole thing rides pure prose-steering with no backstop or drift detection.
- **`vcs root` / `discover` / `repository_root` naming collision** (flagged by:
  code-quality, documentation, architecture) — the command is named `root` and
  its clap help calls it "a repository-root lookup that resolves identically",
  yet it deliberately surfaces `discover` (the working-copy root) precisely
  because it must *not* match `repository_root` in a jj secondary workspace. The
  help text contradicts the design decision it implements.
- **"One documented vocabulary" is undercut by re-inlining** (flagged by:
  code-quality, documentation, usability) — the skill rewrites paste the concrete
  backend commands (`main...HEAD`, `fork_point(...)`, `git/jj config user.name`)
  into the skill bodies instead of deferring to the reference the way
  `research-issue` (the cited template) does, creating two-to-three sources of
  truth and three different reference styles across the four skills.
- **Headline invariants documented but not enforced** (flagged by: test-coverage,
  architecture, documentation) — the "no direct git token in skills/" sweep is a
  one-shot manual grep (no lint), AC5's diff-equality is manual-only, phase
  independence rests on discipline not tooling, and the work-item corrections the
  plan describes are never actually written back to the work item.

### Tradeoff Analysis

- **Single-source-of-truth (DRY) vs. actionability**: code-quality and
  documentation want the skills to defer to the reference with no inlined
  backend commands (avoids drift); usability wants concrete inline commands so
  the model/human has something copy-pasteable at the point of use (especially
  for error-recovery). Recommendation: pick *one* shape and apply it uniformly —
  a neutral label plus a reference pointer, with an inline per-backend example
  only where the command is non-trivial, matching `research-issue`. Inconsistency
  is the actual defect; either pure-defer or defer-plus-example is defensible if
  applied to all four skills.
- **Backend-neutrality vs. recovery-command precision** (`jj restore` prose):
  neutralising `jj restore` in the partial-write abort diagnostics removes a
  copy-pasteable recovery command — and `jj restore` is not even matched by the
  AC1 `git <subcommand>` sweep, so the cost is paid without the invariant
  requiring it. Recommendation: keep a neutral label with inline per-backend
  examples, or add a discard/restore idiom to the reference and point at it.

### Findings

#### Critical

- 🔴 **Correctness**: jj `fork_point(trunk())` cannot express the @-vs-trunk
  divergence point, so the jj diff diverges from `git diff main...HEAD`
  **Location**: Phase 2 (jj diff-range bullet); Phase 3 §2 (validate-plan
  evidence block); work-item AC5
  The divergence point is a function of *both* `@` and `trunk` (the git form
  passes both: `main...HEAD` = `merge-base(main, HEAD)`). `fork_point(trunk())`
  passes only `trunk()`, so it resolves to trunk's own tip; the command becomes
  `jj diff --from <trunk-tip> --to @`, which equals the git form only when trunk
  has not advanced past the fork point. Whenever trunk moved on, the jj diff
  injects spurious reverse-diffs of trunk's advancement. The flawed revset
  appears in three coupled places (the `detect.rs` const, the skill body, AC5),
  and the plan's linear fixtures mask it. Fix: name both endpoints, e.g.
  `jj diff --from 'fork_point(trunk() | @)' --to @`, in all three places, and add
  a fixture where trunk advances past divergence so git and jj are asserted equal
  in the divergent case.

#### Major

- 🟡 **Correctness · Compatibility · Documentation · Usability**: the diff-range
  idiom hard-codes a branch named `main`
  **Location**: Phase 2 §1 (git diff-range bullet); Phase 3 §2
  `git diff main...HEAD` assumes a resolvable local ref named `main`, while the
  jj side resolves the trunk dynamically via `trunk()`. In a consumer repo whose
  trunk is `master`/`develop`/`trunk`, or where `main` exists only as
  `origin/main`, the git form fails (`fatal: bad revision 'main...HEAD'`) while
  jj succeeds — the two backends diverge and the branch-agnostic `HEAD~N..HEAD`
  it replaces regresses. Accelerator ships to arbitrary repos, so this is not
  safe. Express the git anchor against the resolved default branch, or document
  `main` as a placeholder for the repo's trunk.
- 🟡 **Correctness**: `trunk()` falls back to `root()` with no origin trunk
  bookmark, turning the jj diff into a whole-history diff
  **Location**: Phase 2 (jj bullet); work-item AC5 fixture; Phase 3 manual
  verification
  jj's `trunk()` resolves to a remote bookmark on origin and falls back to
  `root()` when none exists. Local-only jj repos and most self-built fixtures
  (including AC5's three-commit fixture) have no origin trunk, so the diff runs
  from the virtual root — the entire history, not the implementation change.
  State the origin-trunk precondition and have the jj fixtures establish an
  `origin` trunk bookmark (or override `trunk()`).
- 🟡 **Correctness**: Phase 5's replacement text reintroduces `git config
  user.name`, which its own sweep gate forbids
  **Location**: Phase 5 §1 vs Phase 5 Automated Verification
  The proposed author-chain text spells the idiom inline as "`jj config get
  user.name` under jj, `git config user.name` under git". The Phase 5 automated
  criterion reuses the Phase 3 sweep, whose pattern includes `config`
  (`\bgit (…|config)\b`), so the sweep over `refine-work-item/SKILL.md` would
  *not* return nothing — applying the shown text and running the shown check
  cannot both succeed. Point to the idiom by name without spelling the backend
  commands, or relax the criterion to AC1's reviewer clause.
- 🟡 **Compatibility**: collapsing the author chain drops the git fallback for
  colocated repositories
  **Location**: Phase 5 §1
  A colocated repo is a jj session (mode jj-colocated selects the JJ reference),
  and jj reads its own config stack, not git's. Where the jj identity is unset
  but the git identity is configured, the old `jj … → git config` chain resolved
  via git; the collapsed single step returns nothing and skips to prompting. The
  work item only guarantees the git-*only* case; colocated regresses. Keep an
  explicit git-identity fallback for jj/colocated sessions before prompting.
- 🟡 **Code Quality · Documentation · Architecture**: `vcs root` clap help
  mislabels the working-copy-root command as a "repository-root lookup that
  resolves identically"
  **Location**: Phase 1 §1 (CLI variant doc-comment); Design decision
  The variant doc-comment (surfaced by `accelerator vcs root --help`) reads
  "…for a repository-root lookup that resolves identically under git, colocated
  jj, and pure-jj", but the command surfaces `discover` *because* it must not
  resolve to `repository_root` in a jj secondary workspace, where it returns the
  workspace, not the shared main repo. The help contradicts the module doc's
  accurate "working-copy root" and omits the command's single defining nuance.
  The command *name* `root` compounds this by inviting the `repository_root`
  mental model. Reword the help to name the working-copy root and note the
  secondary-workspace behaviour; consider a more faithful name.
- 🟡 **Code Quality · Documentation · Usability**: skill rewrites re-inline the
  backend commands the reference exists to centralise
  **Location**: Phase 3 §2; Phase 5 §1 vs Phase 2
  The reference extension exists so skills "lean on one documented vocabulary",
  yet validate-plan inlines `main...HEAD` / `fork_point(...)` and refine-work-item
  inlines `git/jj config user.name`, so each backend command lives in the const,
  the skill body, and the plan. The cited template `research-issue` does the
  opposite ("the session's VCS log/diff command", no inlining). This also yields
  three different reference styles across the four skills (bare abstraction,
  abstraction-plus-pointer, abstraction-plus-inline), giving the model no
  consistent signal. Standardise on one shape.
- 🟡 **Architecture**: independent-mergeability is overstated; the skill→CLI
  cross-phase dependency is unenforced by tooling
  **Location**: Implementation Approach
  Phase 3's validate-plan prose invokes `accelerator vcs root` (Phase 1) and a
  reference idiom that only exists after Phase 2, but Phase 3's automated checks
  are a git-token grep plus `mise run check` — neither executes `vcs root` nor
  asserts the idiom is present. With CI-only enforcement, Phase 3 can merge before
  Phases 1-2 with CI green while the skill breaks at invocation. Either land
  1+2+3 as one unit, or add a check that every `accelerator vcs <sub>` referenced
  in a SKILL.md resolves to a real subcommand and every named idiom substring
  exists in `detect.rs`.
- 🟡 **Architecture**: steering-vs-backstop asymmetry leaves the diff on
  undetectable prose-steering
  **Location**: Design decision; Phase 3 (diff); What We're NOT Doing
  A deterministic CLI is built for the trivial root lookup, but the most
  correctness-sensitive operation (the implementation diff) rides prose-steering
  with no port, no verification, and no backstop — the guard only blocks *raw
  git* in jj mode, not a wrong-but-valid revset. The work item's acknowledged
  pivot ("if steering proves unreliable, extend the CLI route to the diff") has
  no trigger because the failure is silent. Add a determinism anchor for the
  diff range, or an explicit detection note so the pivot is triggerable.
- 🟡 **Test Coverage**: the headline no-direct-git invariant has no CI-gated
  regression guard
  **Location**: Testing Strategy → Cross-skill Verification; Phase 3/4/5
  The plan repeatedly calls the `git <subcommand>` sweep "the automated gate",
  but no registered lint enforces it — `mise run check` runs
  frontmatter/bare-invocation/permission lints only. The sweep is a one-shot
  manual command, so `mise run` green will not catch a future skill reintroducing
  raw git. Add a `tasks/lint/` sweep task (mirroring `bare_invocation.py`) wired
  into the aggregate `lint`/`check`.
- 🟡 **Test Coverage**: the core diff-equality guarantee is manual-verification-
  only
  **Location**: Phase 3 → Manual Verification (AC4/AC5)
  The single load-bearing behavioural promise — validate-plan's diff equals the
  trunk-divergence change across git-only, colocated, and pure-jj — sits entirely
  under Manual Verification because the skill is model-driven prose. Nothing
  repeatably verifies the reconstruction. Pin what can be pinned: a golden/
  `.contains` lock on the exact idiom strings, an eval scenario, and a documented
  mandatory release gate rather than an ad-hoc step.
- 🟡 **Usability**: neutralising `jj restore` strips the exact command from
  error-recovery diagnostics
  **Location**: Phase 5 §2 (lines ~171, 187, 214)
  The three reworded sites are partial-write abort diagnostics that currently
  hand a copy-pasteable `jj restore <file>` / `jj restore <parent-path>`. The
  plan replaces them with "discard … with your session's VCS" but adds no
  restore/discard idiom to the reference, so the model/user must reconstruct the
  command unaided at a failure moment — and inconsistently, since the same skill's
  identity edit keeps inline examples. Keep a neutral label plus inline
  per-backend examples, or add a discard idiom to the reference.
- 🟡 **Documentation**: plan documents work-item corrections but no phase updates
  the work item
  **Location**: Design decision; Key Discoveries
  The plan says it "narrows" the work item's Requirement/AC2 (which name
  `repository_root`) and "corrects" two Technical Notes, but no phase edits
  `meta/work/0286-…md`, so those docs stay wrong and the AC2 acceptance gate
  disagrees with the delivered design. Add an explicit step to update the work
  item's Requirement, AC2, and the two Technical Notes, and state whether the
  twice-extended session-VCS reference contract now warrants an ADR (research
  notes none covers it).

#### Minor

- 🔵 **Compatibility**: git and jj diff-range renderings differ on uncommitted
  changes
  **Location**: Desired End State / Phase 2; AC5
  `git diff main...HEAD` compares committed trees (excludes the working tree),
  whereas `jj diff … --to @` ends at the working-copy commit, which includes
  tracked-but-uncommitted changes. For identical state the jj diff contains edits
  the git diff omits. Align the endpoints or document "committed changes only".
- 🔵 **Test Coverage**: the Phase 1 unit test cannot discriminate `discover` from
  `repository_root`
  **Location**: Phase 1 Success Criteria; Testing Strategy
  `RepoRoot::repository_root` has a default identity implementation, so a stub
  implementing only `discover` returns the same value for both methods — the unit
  test passes even if the handler mistakenly called `repository_root`. Only the
  secondary-workspace golden distinguishes them, and only if it asserts
  `printed == secondary_canonical` (not merely non-empty), both canonicalised for
  the macOS `/private` symlink. Treat that assertion as non-optional.
- 🔵 **Test Coverage · Documentation**: "regenerate the goldens" is imprecise and
  hand-editing benchmark evidence risks fabricated records
  **Location**: Phase 2 §3; Phase 1 root_goldens; Phase 5 §4
  `detect_goldens.rs` does exact `assert_eq!` against committed fixtures with no
  `REGENERATE_GOLDENS` mechanism (unlike `status_log_goldens.rs`), so they are
  hand-edited — the "regeneration" wording muddies this. The new `root` test
  computes expectations from the temp checkout, so it is a parity test, not a
  golden. And hand-"updating" `benchmark.md` records eval evidence no run
  produced. Prefer harness-regenerated benchmark evidence; edit only the added
  golden lines and review the fixture diff.
- 🔵 **Test Coverage**: eval-spec changes are not CI-gated and the `contains`
  assertion loses discriminating power
  **Location**: Phase 5 §3-4
  No eval task exists in `mise.toml`, so neither the eval spec nor benchmark files
  gate `mise run`. The plan replaces `"value": "jj restore"` with unspecified
  "backend-neutral wording"; a loose phrase weakens the `contains` check. Choose a
  stable literal matching the exact neutral phrase, and state that eval/benchmark
  verification is a manual release step.
- 🔵 **Test Coverage**: research-issue pure-jj regression is manual-only
  **Location**: Testing Strategy → Cross-skill Verification
  research-issue shares the session-VCS reference this plan edits, but its "no
  `fatal: not a git repository` in pure-jj" regression is a manual note with no
  automated tie. Record it as a required release-time verification.
- 🔵 **Code Quality**: `--fail-safe` on `Root` is a dead flag on a fallible
  handler
  **Location**: Phase 1 §1-2
  No planned caller passes it (validate-plan calls `accelerator vcs root` bare;
  it is not hook-wired), and the launcher's `swallow_under_fail_safe` scans argv
  rather than requiring the subcommand to declare it. Unlike Status/Log (whose
  handlers never fail), `Root` has a real failure mode, so the inert flag reads as
  misleading. Either omit it or make the doc-comment say it exists purely for
  launcher-shape consistency.
- 🔵 **Usability · Correctness**: out-of-repo failure does not reliably abort the
  chained checks command
  **Location**: Phase 3 §2; Phase 1 §3
  `cd "$(accelerator vcs root)" && make check test` captures only stdout; on the
  out-of-repo error, substitution yields `""` and `cd ""` is a silent success
  no-op in bash (aborts in zsh), so `make check test` runs in the wrong tree. The
  plan's "an error aborts the cd" premise does not hold uniformly. Capture into a
  variable and test non-empty / propagate the exit status before `cd`.
- 🔵 **Usability**: out-of-repo error message omits the searched-from path
  **Location**: Phase 1 §3
  The handler errors with a bare "not inside a repository". git's analog signals
  that parents were searched. Include the start path, e.g. "not inside a
  repository (searched from <path>)".
- 🔵 **Compatibility**: the new allowed-tools entry does not auto-permit the
  compound checks command
  **Location**: Phase 3 §1-2
  `Bash(accelerator vcs root)` is an exact-match entry; the command actually run
  is `cd "$(accelerator vcs root)" && make check test`, whose `cd`/`make`
  segments and command-substitution are not covered. Not a regression, but the
  success criterion should not be read as granting the checks step unattended
  permission.
- 🔵 **Compatibility**: presenting both renderings inline slightly raises
  wrong-backend risk in pure-jj
  **Location**: Phase 3 §2
  If the model runs the git form in a pure-jj session, the guard denies it —
  reintroducing exactly the breakage the plan removes (only on model
  misbehaviour). Deferring purely to the mode-selected reference (as
  research-issue does) lowers the chance.
- 🔵 **Standards**: the new golden test file must carry the bash-parity cfg gate
  **Location**: Phase 1 Success Criteria; Testing Strategy
  Every file in `cli/vcs-cli/tests/` opens with `#![cfg(feature = "bash-parity")]`.
  The plan prescribes running `root_goldens.rs` with `--features bash-parity` but
  never states the file must carry the crate-level `cfg`; without it, its real
  jj/git topology setup (including `jj workspace add`) runs in the default lane.
- 🔵 **Standards**: per-phase step order lists production before the failing test
  **Location**: Implementation Approach; Phase 1 & 2 Changes Required
  Red-green-refactor is stated globally but the per-phase enumeration inverts it
  (variant/wiring/handler as steps 1-3, tests only under Success Criteria; Phase 2
  const change ahead of render tests). Reorder so the failing test comes first in
  each compiled phase.
- 🔵 **Correctness**: the config/migrate `old_string` does not match the wrapped
  source
  **Location**: Phase 4 §1
  The plan quotes "confirm via `jj status`/`git status` that the dirty paths", but
  the source wraps `jj` and ` status` across lines 272-273, so a verbatim
  exact-string edit would not match. Reword the whole sentence rather than
  matching a single-line substring.

#### Suggestions

- 🔵 **Documentation**: cross-document line-number references disagree and will
  drift
  **Location**: Phase 5 §2; Current State Analysis
  The plan (rev df17c8e6) and the research doc (rev f418ac31) cite different line
  numbers for the same sites (e.g. refine-work-item `jj restore` at plan 171-172
  vs research 172/188 vs actual 171/187/214). Note that line numbers are as-of the
  pinned revision and prefer content anchors for within-file edits.
- 🔵 **Standards**: the new jj reference bullets depart from the "instead of git"
  phrasing
  **Location**: Phase 2 §1
  Existing `JJ_COMMAND_REFERENCE` bullets follow "Use `jj X` instead of `git Y`";
  the two new ones read "…for the cumulative change…" / "…to read the configured
  user identity". Defensible (no 1:1 git command), but either phrase against a git
  counterpart or accept the divergence deliberately.

### Strengths

- ✅ The `root.rs` handler correctly mirrors `detect.rs`'s direct-probe-port
  pattern rather than the `report::run` never-fail boundary (which exists only to
  wrap the fallible `VcsReporter`), depends on the `RepoRoot` port with
  `InProcessProbe` injected at the composition root, and matches the verified
  `discover` signature (`Option<PathBuf>`), so `.ok_or_else(kernel::Error::Failed)`
  compiles and correctly ports `git rev-parse --show-toplevel`'s out-of-repo exit.
- ✅ The `discover`-over-`repository_root` decision is verified correct: for a jj
  secondary workspace `repository_root` maps up to the shared main repo, which
  would `cd` validate-plan's checks out of the workspace; `discover` returns the
  checkout it runs in.
- ✅ The git three-dot `main...HEAD` is the right *shape* (`merge-base(main,
  HEAD)..HEAD`), correctly distinct from `git log A...B`'s symmetric-difference
  semantics.
- ✅ The plan corrects three inaccuracies it inherited: `repository_root`'s real
  consumer, the not-thirteen-point checklist classification, and validate-plan's
  scoped (not blanket) allowed-tools — and the added `Bash(accelerator vcs root)`
  entry is genuinely required and correctly formatted.
- ✅ The `Root` clap variant faithfully mirrors the Detect/Status/Log/Guard derive
  style, and the `concat!` reference additions respect the 80-column floor and the
  backslash-continuation style; every proposed doc-comment is a legitimate
  module/rustdoc/clap-help form with no policy-violating comments.
- ✅ The reference extension is genuinely additive: exactly three descriptive
  goldens embed the reference, the `.contains` render tests tolerate additions,
  and `additionalContext` has no downstream structured parser — no hidden consumer
  breaks, and no `hooks.json` change is needed.
- ✅ The Current State table is revision-anchored with per-site line numbers and a
  "Nature" column, the References section is thorough and specific, and
  load-bearing line-number citations were verified accurate.

### Recommended Changes

1. **Fix the jj diff-range revset** (addresses: the Critical; "trunk() → root()";
   "diff-range is the weakest part") — replace `fork_point(trunk())` with a
   two-endpoint form such as `fork_point(trunk() | @)` in the `detect.rs` const,
   the validate-plan body, and AC5; add a fixture where trunk advances past
   divergence, and state the origin-trunk-bookmark precondition (or override
   `trunk()` in the jj fixtures).
2. **Make the git diff anchor trunk-name-agnostic** (addresses: hard-coded `main`;
   uncommitted-changes divergence) — resolve the default branch rather than
   literal `main`, and either align the git/jj endpoints on uncommitted changes or
   document "committed changes only".
3. **Resolve the Phase 5 self-contradiction and the colocated regression**
   (addresses: `git config` reintroduced; colocated fallback dropped) — have the
   author step point to the user-identity idiom by name without inlining backend
   commands, and keep an explicit git-identity fallback for jj/colocated sessions
   before prompting.
4. **Fix the `vcs root` help text** (addresses: the naming/mislabel theme) —
   describe the working-copy root of the checkout the command runs in, note the
   secondary-workspace behaviour, and drop "repository-root lookup that resolves
   identically"; consider a more faithful command name.
5. **Choose one skill-reference shape and apply it uniformly** (addresses:
   re-inlining/drift; inconsistent vocabulary; `jj restore` recovery prose) —
   either defer to the reference (research-issue style) or defer-plus-inline-
   example, across validate-plan, config/migrate, refine-work-item; if discard
   guidance is neutralised, keep an inline recovery example or add a discard idiom
   to the reference.
6. **Give the headline invariants CI teeth** (addresses: no git-sweep lint; AC5
   manual-only; unenforced phase order; research-issue manual) — add a
   `tasks/lint/` git-token sweep wired into `check`; add a check binding SKILL
   `accelerator vcs <sub>` references and named idioms to real
   subcommands/const substrings; pin the idiom strings with a golden/`.contains`;
   record the manual diff-equality and research-issue checks as a documented
   release gate.
7. **Write the corrections back to the work item** (addresses: stale work item) —
   update 0286's Requirement, AC2, and the two Technical Notes to the `discover`
   decision and scoped-allowed-tools reality; decide whether an ADR is warranted.
8. **Tidy the smaller gaps** (addresses: several minors) — `#![cfg(feature =
   "bash-parity")]` on `root_goldens.rs`; assert `printed == secondary_canonical`
   in that test; guard `cd "$(accelerator vcs root)"` against an empty result;
   include the searched-from path in the error; reword the config/migrate sentence
   rather than matching the wrapped substring; reorder each compiled phase's steps
   test-first.

## Per-Lens Results

### Correctness

**Summary**: The Rust-side mechanics are sound (`discover` returns
`Option<PathBuf>`, so the handler's error contract is correct; `discover`-over-
`repository_root` is the right semantics for `make check test`). The load-bearing
defect is the diff-range idiom: the git three-dot form is correct, but the jj
`fork_point(trunk())` form cannot compute the @-vs-trunk divergence point and is
not equal to the git form whenever trunk has advanced — contradicting AC5. A
second, independent break: Phase 5's proposed text reintroduces the literal
`git config user.name` its own sweep gate forbids.

**Strengths**:
- git three-dot `main...HEAD` correctly means `merge-base(main, HEAD)..HEAD`,
  distinct from `git log A...B`'s symmetric-difference semantics.
- `discover` (working-copy root) over `repository_root` (secondary workspace →
  shared main repo) is verified correct for validate-plan's checks step.
- The handler's `Option`-based error path compiles and matches
  `git rev-parse --show-toplevel`'s out-of-repo exit.
- Load-bearing line-number citations verified accurate; exactly three descriptive
  goldens embed the reference (no fourth missed).

**Findings**:
- 🔴 critical / high — jj `fork_point(trunk())` cannot express the @-vs-trunk
  divergence point, so the jj diff diverges from `git diff main...HEAD` (Phase 2
  jj bullet; Phase 3 §2; AC5). The fork point depends on both `@` and `trunk`;
  the single-arg revset resolves to trunk's tip, yielding
  `jj diff --from <trunk-tip> --to @`, which equals the git form only when trunk
  has not advanced. Fix with `fork_point(trunk() | @)` in all three places plus a
  divergent-case fixture.
- 🟡 major / high — `trunk()` falls back to `root()` with no origin trunk
  bookmark, turning the jj diff into a whole-history diff (Phase 2; AC5 fixture).
  State the precondition and establish an `origin` trunk bookmark in fixtures.
- 🟡 major / high — Phase 5 §1's text reintroduces `git config user.name`, matched
  by the Phase 5 sweep (`config` is in the pattern); the edit fails its own gate.
  Point to the idiom by name, or relax the criterion to AC1's reviewer clause.
- 🔵 minor / medium — `git diff main...HEAD` hard-codes a `main` ref that need not
  exist; fails on non-`main` trunks.
- 🔵 minor / low — the config/migrate `old_string` ("confirm via `jj status`/`git
  status`…") does not match the source wrapped across lines 272-273.
- 🔵 suggestion / low — out-of-repo error swallowed by `cd "$(accelerator vcs
  root)"` (empty substitution → `cd ""`), so checks may run in the wrong tree.

### Compatibility

**Summary**: Largely compatibility-sound — the `Root` variant is purely additive
and reachable via the launcher's existing External routing, `--fail-safe` is
launcher-generic, the reference extension is additive with correctly identified
consumers, and `discover` matches `git rev-parse --show-toplevel`/`jj workspace
root` across all four topologies. The dominant risk is the hard-coded `main` in
the git diff-range idiom (breaks cross-backend parity and non-`main` repos); a
secondary risk is a colocated author-resolution regression.

**Strengths**:
- Adding `Root` is backward compatible; the launcher routes unknown subcommands
  via `External`, so no launcher/manifest/token change is needed.
- `--fail-safe` is handled generically (argv scan), so `Root` needs no special
  wiring.
- The plan correctly overrides the work item's stale allowed-tools claim (three
  scoped entries, not blanket), making `Bash(accelerator vcs root)` genuinely
  required.
- The reference extension is additive (three descriptive goldens; `.contains`
  render tests; free-text `additionalContext`) — no hidden consumer breaks.
- `discover`-not-`repository_root` verified correct across all four topologies.
- Identity/diff idioms are guard-compatible on the intended per-mode path.

**Findings**:
- 🟡 major / high — hard-coded `main` in the git diff-range idiom breaks
  cross-backend equivalence and non-`main` consumer repos (Phase 2 §1; Phase 3
  §2). Resolve the default branch dynamically or use a placeholder trunk ref.
- 🟡 major / medium — collapsing the author chain drops the git fallback for
  colocated repositories (Phase 5 §1): a colocated repo is a jj session, jj reads
  its own config, so a git-only identity no longer resolves. Keep a git fallback.
- 🔵 minor / medium — git and jj diff-range renderings differ on uncommitted
  changes (git committed trees vs jj `--to @`). Align endpoints or document.
- 🔵 minor / low — guard-safe only if the model picks the backend-matching idiom;
  presenting both inline raises wrong-backend risk in pure-jj.
- 🔵 minor / low — the new allowed-tools entry scopes only the bare invocation,
  not the compound `cd … && make …` checks command.

### Architecture

**Summary**: Architecturally sound and well-grounded in the ports-and-adapters
structure — the handler mirrors `detect.rs`'s direct-probe pattern, depends on
the `RepoRoot` port with the concretion injected at the composition root, and is
correctly scoped as plain clap. The `discover`-over-`repository_root` decision is
well-reasoned and introduces no coupling problem. The genuine structural tension
is the deliberate asymmetry: a deterministic CLI for the trivial root lookup, but
pure prose-steering with no backstop for the correctness-sensitive diff.

**Strengths**:
- `root.rs` mirrors `detect.rs` (direct probe ports), not `status.rs`/`log.rs`
  (the `report::run` boundary that exists only to wrap the fallible reporter).
- Generic `run<P: RepoRoot>` with `InProcessProbe` wired in the thunk — textbook
  dependency inversion, consistent with existing thunks.
- Keeping `repository_root` with its sole `vcs::facts` caller creates no coupling
  problem — the two `RepoRoot` methods answer distinct questions.
- The two-level dispatch model is correctly applied (rides the `vcs` token; skips
  the thirteen-point checklist; confined to the public-API-exempt crate).
- `--fail-safe` on `Root` is verified safe (launcher degrades only its own
  availability failures, never the handler's exit).
- One shared reference for multiple skills is good cohesion; flows via existing
  hook wiring.

**Findings**:
- 🟡 major / medium — steering-vs-backstop asymmetry leaves the correctness-
  sensitive diff on undetectable prose-steering (Design decision; Phase 3; What
  We're NOT Doing). The acknowledged pivot has no trigger because failure is
  silent. Add a determinism anchor or an explicit detection note.
- 🟡 major / medium — independent-mergeability is overstated; the cross-phase
  skill→CLI dependency is unenforced (Implementation Approach). Phase 3 can merge
  before 1-2 with CI green while the skill breaks at invocation. Land 1+2+3
  together, or add a SKILL↔CLI reference check.
- 🔵 minor / medium — the command named `root` surfaces `discover`, not the
  trait's `repository_root` — domain-alignment ambiguity. Pin semantics in the
  doc-comment; consider `checkout-root`/`workspace-root`.
- 🔵 suggestion / medium — narrowing a documented AC inside the plan leaves plan
  and work item inconsistent; flow the amendment back to 0286.

### Test Coverage

**Summary**: The compiled phases are well-covered (handler unit test on both
branches; per-backend render `.contains`; bash-parity goldens that *do* run in CI
under `--all-features`; a genuinely discriminating secondary-workspace golden).
The weakness is the skill side: the headline no-direct-git invariant (AC1) has no
CI-gated guard (the "automated gate" is a manual grep), and the load-bearing
diff-equality guarantee (AC5) is manual-only because the skill is model-driven
prose.

**Strengths**:
- Phase 1 handler test covers `Some`/`None`.
- The secondary-workspace bash-parity golden genuinely discriminates
  `discover` from `repository_root`.
- Bash-parity goldens are CI-gated (`mise run test:cli` uses `--all-features`).
- `.contains` render tests are mode-specific, catching a bullet in the wrong
  block.
- The plan is honest that evals are not `mise`-wired and benchmark regeneration is
  not CI-gated.

**Findings**:
- 🟡 major / high — the headline no-direct-git invariant has no CI-gated
  regression guard; add a `tasks/lint/` sweep wired into `check`.
- 🟡 major / high — the core diff-equality guarantee (AC4/AC5) is manual-only; pin
  the idiom strings and record a documented release gate.
- 🔵 minor / high — the unit test cannot discriminate `discover` vs
  `repository_root` (default identity impl); the golden must assert
  `printed == secondary_canonical`.
- 🔵 minor / medium — hand-edited golden regeneration can rubber-stamp an
  unintended reference change; review the fixture diff.
- 🔵 minor / medium — eval-spec changes are not CI-gated and the `contains`
  assertion needs a precise literal.
- 🔵 minor / medium — research-issue pure-jj regression is manual-only despite
  sharing the edited reference.

### Code Quality

**Summary**: The Phase 1 slice is small, idiomatic, and faithful to the crate:
the handler's `discover(...).map(...).ok_or_else(...)` matches the real `Option`
signature and the `kernel::Error::Failed` variant, mirrors `detect.rs`, and keeps
the generic seam for stub testability. The doc-comments are policy-compliant. Two
maintainability concerns: the `Root` help text calls a working-copy-root command
a "repository-root lookup", and the skill rewrites re-inline the concrete backend
renderings the reference exists to centralise.

**Strengths**:
- Minimal, idiomatic handler matching the verified `discover` signature and the
  crate's `kernel::Error::Failed` convention.
- Mirroring `detect.rs` over `status.rs`/`log.rs` is the correct, simpler choice.
- `run<P: RepoRoot>` preserves stub-based testability; the thunk/dispatch match
  the Status/Log idiom.
- No comment-policy violation — module `//!`, clap `///`, and `/// # Errors` all
  match sibling conventions and describe intent, not code.
- `Command::Root { fail_safe: _ } => run_root()` and the `Failed` (→ exit 1)
  categorisation match the crate.

**Findings**:
- 🟡 major / medium — the `Root` help text calls it a "repository-root lookup" but
  surfaces the working-copy root (`discover`); a maintainer could "align" it onto
  `repository_root` and reintroduce the secondary-workspace divergence. Reword to
  name the working-copy root.
- 🟡 major / medium — skill prose re-inlines the concrete idiom renderings
  (`main...HEAD`, the jj revset, `git/jj config user.name`) the reference exists
  to centralise, diverging from research-issue and risking drift. Refer by name
  and point at the reference.
- 🔵 minor / medium — `--fail-safe` on `Root` is unused by any caller and ignored
  by a handler that can actually fail; omit it or document the consistency intent.

### Documentation

**Summary**: Unusually strong as documentation-of-a-change (revision-anchored
current-state table, explicit correction of inherited inaccuracies, concrete
testable criteria, a rich file:line References section). Risks cluster in three
areas: source docs left stale (the work-item corrections are described but never
written back), the two model-facing idioms documented inconsistently (skills
re-inline rather than defer; the git idiom hard-codes `main` while the prose says
"trunk"), and the `vcs root` clap help mislabelling a working-copy-root command.

**Strengths**:
- Revision-anchored Current State table with per-site line numbers mitigates
  drift.
- Explicitly documents and corrects the inherited inaccuracies rather than
  propagating them.
- Thorough, specific References section.
- Accurate, proportional module/`# Errors` doc-comments.
- The design decision is captured with rationale and a dedicated test.

**Findings**:
- 🔴 major / high — the plan documents work-item corrections but no phase updates
  the work item, leaving Requirement/AC2/Technical Notes stale. Add an explicit
  update step; decide whether an ADR is warranted.
- 🔴 major / high — the clap help mislabels the working-copy-root command as a
  "repository-root lookup that resolves identically", contradicting the design
  decision. Rewrite to name the working-copy root and the secondary-workspace
  behaviour.
- 🟡 major / medium — skill rewrites re-inline backend commands, duplicating the
  reference and diverging from the research-issue template. Defer, or document why
  inlining is intentional and how copies stay in sync.
- 🟡 major / medium — the diff-range idiom hard-codes `main` while its prose says
  "trunk divergence point" — inaccurate for non-`main` trunks. Make trunk
  resolution explicit or mark `main` a placeholder.
- 🔵 minor / medium — inconsistent VCS vocabulary across rewrites and unlabeled
  "idiom" cross-references; standardise phrasing and give the new bullets
  recognisable anchors.
- 🔵 minor / medium — "regenerate the goldens" is imprecise (detect goldens are
  hand-edited, no regen env var), the new `root` test is a parity test not a
  golden, and hand-editing `benchmark.md` records evidence no run produced.
- 🔵 suggestion / low — cross-document line-number references (plan vs research)
  disagree; note they are as-of the pinned revision and prefer content anchors.

### Standards

**Summary**: Strongly conformant — the `Root` variant faithfully mirrors the
existing derive style (including a verbatim copy of the Log variant's
`--fail-safe` doc), the `concat!` additions respect the 80-column floor and the
continuation style, every doc-comment is a legitimate form with no restating or
stale references, and the allowed-tools entry matches the exact-match format. The
plan correctly classifies `vcs root` as plain second-level clap and identifies the
narrower set of points that do apply. The only gaps are a missing bash-parity cfg
gate on the new test file and a per-phase step order that lists production before
tests despite red-green-refactor.

**Strengths**:
- `Root` faithfully mirrors Detect/Status/Log/Guard (doc-as-help, `#[arg(long)]`,
  matching thunk/dispatch).
- Correctly classifies the change as plain clap, not the thirteen-point checklist,
  and notes the cargo-public-api exemption.
- All comments conform to the strict policy.
- `concat!` additions respect 80 columns and the continuation style, placed
  correctly before the separator.
- allowed-tools addition matches the bare-`accelerator` exact-match format.
- Correctly prescribes hand-editing the detect goldens (no `REGENERATE_GOLDENS`),
  the `*_goldens.rs` naming, and the `--features bash-parity` invocation.

**Findings**:
- 🔵 minor / medium — the new `root_goldens.rs` must open with `#![cfg(feature =
  "bash-parity")]` (every sibling test file does) or it runs its real topology
  setup in the default lane.
- 🔵 minor / medium — per-phase step order lists production before the failing
  test, contradicting red-green-refactor; reorder each compiled phase test-first.
- 🔵 suggestion / low — the new jj reference bullets depart from the block's
  "Use `jj X` instead of `git Y`" phrasing; phrase against a git counterpart or
  accept the divergence deliberately.

### Usability

**Summary**: Thoughtfully scoped and gets the CLI surface right —
`accelerator vcs root` is intuitive and composes cleanly, and the plan catches the
allowed-tools scoping gotcha the work item missed. The dominant risk is that the
primary consumer is the model, yet the rewritten skills present three different
VCS-reference styles, one error-recovery message loses its concrete command with
no fallback idiom, and the diff-range abstraction asks the model to reconstruct an
under-specified jj command while giving git a trunk-hardcoded example.

**Strengths**:
- `accelerator vcs root` is a discoverable, least-surprise name that composes into
  `cd "$(...)"` and reads better than `git rev-parse --show-toplevel`.
- The plan corrects the stale allowed-tools claim, avoiding a silent
  permission-denied at invocation.
- After Phase 2 the reference carries every idiom the skills point to.
- Inlining the concrete forms in validate-plan spares the human reader a lookup,
  and the manual diff-equality check is a sensible mitigation.

**Findings**:
- 🟡 major / medium — the VCS-reference idiom is inconsistent across the rewritten
  skills (bare abstraction / abstraction-plus-pointer / abstraction-plus-inline),
  even within one block. Pick one canonical shape and apply it uniformly.
- 🟡 major / high — neutralising `jj restore` strips the exact command from
  partial-write error-recovery diagnostics, with no restore idiom added to the
  reference; also inconsistent with the identity edit that keeps inline examples.
- 🟡 major / medium — the diff-range abstraction is under-specified for jj ("the
  fork_point(trunk())-anchored revset" does not uniquely determine the command),
  while git is near-inline — the backends are not equally actionable from prose.
- 🔵 minor / medium — `main...HEAD` hard-codes the trunk name in the concrete
  example; wrong for non-`main` trunks (overlaps correctness/compatibility).
- 🔵 minor / medium — out-of-repo failure does not reliably abort the chained `cd`
  (`cd ""` is a silent no-op in bash), so checks may run in the wrong tree.
- 🔵 minor / low — the out-of-repo error message omits the searched-from path.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-09-22

**Verdict:** REVISE

Re-ran all eight lenses against the revised plan. **Every review-1 finding is
resolved** — the critical (jj revset), all 12 majors, and the minors are
confirmed fixed across all eight lenses, verified against source (e.g.
`fork_point(trunk() | @)` genuinely computes the `@`/trunk merge base; `discover`
verified over `repository_root`; the `root="$(…)" && cd` abort verified; the
colocated fallback verified to land in the jj reference block that a colocated
session selects). The re-review surfaced **four new majors, all in the material
this revision added** (Phase 6, the discard idiom, the reconciliation section) —
not regressions of the original findings. All four, plus the converging minors,
were **addressed in a same-session follow-up edit** (recorded below). A pass-3
confirmation of that follow-up edit would close the loop, since the new edits are
themselves unreviewed.

### Previously Identified Issues

- 🔴 **Correctness** — jj `fork_point(trunk())` revset — **Resolved** (now
  `fork_point(trunk() | @)` in const, skill, and AC5 verification).
- 🟡 **Correctness/Compatibility/Documentation/Usability** — hard-coded `main` —
  **Resolved** (`<trunk>...HEAD`, default-branch placeholder).
- 🟡 **Correctness** — `trunk()` → `root()` fallback — **Resolved** (precondition
  stated; fixtures set an origin trunk bookmark).
- 🟡 **Correctness** — Phase 5 reintroduced `git config` — **Resolved** (skill
  defers; sweep clean).
- 🟡 **Compatibility** — colocated author fallback dropped — **Resolved**
  (fallback clause in the jj reference block).
- 🟡 **Code Quality/Documentation/Architecture** — `vcs root` help mislabel —
  **Resolved** (help/module doc name the working-copy root).
- 🟡 **Code Quality/Documentation/Usability** — skills re-inline backend commands
  — **Resolved** (all four defer to the reference).
- 🟡 **Test Coverage** — no CI-gated no-git-token guard — **Resolved** (Phase 6
  git-token lint).
- 🟡 **Test Coverage** — AC5 diff-equality manual-only — **Resolved** (documented
  release gate + render-lock + detection note).
- 🟡 **Architecture** — mergeability overstated / dependency unenforced —
  **Resolved** (honest ordering + Phase 6 subcommand binding).
- 🟡 **Architecture** — steering-vs-backstop, no detection — **Resolved**
  (detection note; render-lock).
- 🟡 **Usability** — `jj restore` recovery command stripped — **Resolved then
  refined** (discard idiom added; see new issue below on untracked files).
- 🟡 **Documentation** — stale work item — **Resolved** (Work Item Reconciliation
  section).
- 🔵 All review-1 minors (bash-parity gate, strict-equality parity assertion,
  `cd ""` guard, error path, `--fail-safe`, golden hand-edit, eval literal,
  test-first ordering, config/migrate reword, line-number anchoring) —
  **Resolved**.

### New Issues Introduced (and addressed in follow-up edit)

- 🟡 **Correctness/Documentation/Usability** — discard idiom: `git restore
  <path>` reverts tracked paths but does not remove newly-created untracked files
  (two of refine-work-item's three recovery sites write new files), and those
  messages are printed to the user, who cannot see the model-only reference —
  **Addressed**: Phase 5 §2 now splits the operations (new-file sites *delete* the
  file; the modified-parent site uses the discard idiom and, being user-facing,
  resolves it to the concrete per-backend command inline), and the reference
  discard bullet is caveated (tracked-path only, git 2.23+).
- 🟡 **Correctness/Documentation/Code Quality/Standards** — Work Item
  Reconciliation omitted AC3/AC5, leaving the acceptance gate encoding the old
  `main...HEAD` / `fork_point(trunk())` — **Addressed**: a reconciliation bullet
  now rewords AC3/AC5 to the delivered forms (noting the merge-base fix).
- 🟡 **Test Coverage** — Phase 6's idiom render-lock half had no specified test —
  **Addressed**: Phase 6 §2 now writes a failing test for both assertions
  (mode-specific, idiom-anchor-removed fixture).
- 🟡 **Standards** — Phase 6 under-specified the lint wiring (one surface named of
  four; wrong module home) — **Addressed**: a "Wiring surface" subsection names
  all four (`tasks/lint/__init__.py`, `tasks/__init__.py`, `mise.toml` into both
  `build-system:check` and `lint:check`, `test_mise.py`'s
  `_BUILD_SYSTEM_CHECK_GATES`) and moves the guards to own modules built on
  `skill_parsing`.
- 🔵 Minors — **Addressed**: subcommand set pinned to the clap enum (not
  hardcoded); `<trunk>` resolution hint (`git symbolic-ref … origin/HEAD`,
  origin-anchored to match jj `trunk()`); `git restore` 2.23+ floor;
  committed-vs-`@` caveat moved into the reference bullet; criss-cross
  single-merge-base note; module-doc secondary-workspace clause; `--fail-safe`
  omitted from `Root` (unit variant); jj-token sweep scope noted deliberate;
  ADR-0066 over-claim reworded (owns output format only; concrete fifth-dependent
  ADR trigger); `the session's VCS` possessive standardised; identity-fallback
  adapter test + semantic-clause render assertions added; render tests semantic
  phrases; pure-defer aged-out-reference re-derivation (`vcs detect
  --descriptive`, with allowed-tools entry).

### Assessment

The plan's substance is sound: the original correctness-critical defect and every
review-1 major are resolved and source-verified. The re-review's new findings were
all second-order consequences of the fixes — under-specification in the newly
added Phase 6, an incomplete reconciliation list, and a discard-idiom edge case —
and each has been addressed in the follow-up edit above. The remaining caveat is
that those follow-up edits are unreviewed; a pass-3 covering Phase 6, the discard
prose, and the AC3/AC5 reconciliation would confirm convergence. Recommend
implementing behind that confirmation, or accepting the follow-up edits as
low-risk given their targeted scope.

---
*Re-review generated by /accelerator:review-plan*

## Re-Review (Pass 3) — 2026-09-22

**Verdict:** COMMENT

Focused pass over only the sections the pass-2 follow-up edit changed (Phase 6,
the discard idiom, the discard prose, the AC3/AC5 reconciliation, and the
converging minors), across the five lenses that raised the pass-2 majors. The
convergence trend is clear: **critical + 12 majors (review 1) → 4 majors (pass 2)
→ 1 major (pass 3)** — and that one is a plan-text accuracy error, now fixed, not
a design flaw. All pass-3 findings have been addressed in a follow-up edit.

### Previously Identified Issues (pass-2 new majors)

- 🟡 **Discard idiom — untracked files + user-facing messages** (correctness,
  documentation, usability) — **Resolved, and simplified further.** Pass-3
  correctness verified all three refine-work-item abort sites (including "Parent
  not updated") recover newly-written *child files*, not a tracked-parent
  modification — so the discard idiom had no valid consumer. Recovery is now plain
  "delete the newly-written child file(s)" (correct for git-untracked and
  jj-snapshotted alike), and the **discard idiom has been removed** from the
  reference, render tests, goldens, and Phase 6 (reducing reference accretion).
- 🟡 **AC3/AC5 reconciliation** (correctness, documentation, code-quality,
  standards) — **Resolved.** The reconciliation bullet rewords AC3/AC5 to the
  delivered `fork_point(trunk() | @)` / `<trunk>...HEAD` forms; verified against
  the work item that the old literals are still there and genuinely need it.
- 🟡 **Phase 6 idiom render-lock had no test** (test-coverage) — **Resolved.**
  Both assertions are now test-first, mode-specific, with an anchor-removed
  fixture.
- 🟡 **Phase 6 lint wiring under-specified** (standards) — **Resolved.** All four
  wiring surfaces named and source-verified against the live convention; enum-pin
  and module home confirmed correct.

### New Issues Introduced (and addressed in follow-up edit)

- 🟡 **Correctness** — the Testing Strategy claimed an adapter test pinning the
  jj→git identity fallback, but `InProcessProbe::user_name` has no such fallback
  (verified: its `Jj` arm returns `None` when unset). **Addressed**: the bullet now
  asserts only git-kind resolution; the colocated fallback is documented as
  model-driven (reference bullet), verified by the Phase 5 release-time check.
- 🔵 **Correctness** — the parent-site discard advice was attached to a pre-Edit
  "Parent not updated" abort with nothing to discard. **Addressed** (folded into
  the discard-idiom removal — all three sites now delete child files).
- 🔵 Minors — **Addressed**: `latest(fork_point(trunk() | @))` for criss-cross
  (not `heads()`); `origin/HEAD`-unset fallback noted alongside jj's `root()`;
  `@task check` raise-on-violation leaf test per guard; render-lock anchors kept
  minimal (existence, not exact wording); AC6 colocated leg added to
  reconciliation; possessive fixed in the Phase 5 snippet; canonical
  reference-pointer convention stated (research-issue's bare form left by design);
  `vcs detect --descriptive` JSON-envelope note; ADR/detect.rs enumeration kept in
  the Drafting Note, not a drift-prone source comment.

### Assessment

The plan has converged. Review 1's critical and every major are resolved and
source-verified; pass-2's four new majors are resolved (one, the discard idiom,
by removal once pass-3 showed it had no consumer); pass-3's single new major (a
Testing-Strategy over-claim) and its minors are fixed. Remaining items are
low-severity polish already applied. The follow-up edits are themselves the only
unreviewed delta, but each is a small, targeted wording change with a verified
codebase precedent. **Recommend accepting the plan as ready for implementation** —
a further full pass would yield diminishing returns. The behavioural guarantees
that cannot be unit-tested (validate-plan's diff-equality, refine-work-item's
colocated author fallback, research-issue's pure-jj regression) are recorded as
mandatory release-time gates.

---
*Re-review generated by /accelerator:review-plan*

## Approval — 2026-09-22

**Verdict:** APPROVE

Approved after three review passes converged (critical + 12 majors → 4 majors → 1
major, all resolved and source-verified). The plan is marked `ready` for
implementation. The behavioural guarantees that model-driven prose cannot
unit-test remain mandatory release-time gates (see the Pass 3 assessment).

---
*Approval recorded via /accelerator:review-plan*
