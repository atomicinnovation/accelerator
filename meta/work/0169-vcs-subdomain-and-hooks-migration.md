---
type: work-item
id: "0169"
title: "VCS Subdomain and Hooks Migration"
date: "2026-06-28T17:01:56+00:00"
author: Toby Clemson
producer: extract-work-items
status: done
kind: story
priority: high
parent: "work-item:0136"
blocked_by: ["work-item:0164", "work-item:0166", "work-item:0167", "work-item:0179", "work-item:0186", "work-item:0187", "work-item:0188"]
blocks: ["work-item:0170", "work-item:0171", "work-item:0172", "work-item:0173", "work-item:0174"]
relates_to: ["work-item:0125", "work-item:0165", "work-item:0182", "work-item:0183", "work-item:0185", "work-item:0189", "work-item:0198", "work-item:0199", "work-item:0200", "codebase-research:2026-07-29-0169-vcs-subdomain-and-hooks-migration"]
derived_from: ["codebase-research:2026-06-28-0136-rust-cli-migration-scope-and-architecture"]
tags: [rust, vcs, hooks, migration]
last_updated: "2026-08-17T13:00:00+00:00"
last_updated_by: Toby Clemson
schema_version: 1
external_id: "PP-190"
---

# 0169: VCS Subdomain and Hooks Migration

**Kind**: Story
**Status**: Done
**Priority**: High
**Author**: Toby Clemson

## Summary

Build the VCS subdomain — `vcs detect`, `vcs status`, `vcs log`, `vcs guard` —
over the library-backed adapters 0188 delivers, migrate the SessionStart
VCS-detection and PreToolUse guard logic into the CLI (ADR-0048), and repoint
`skills/vcs/commit` at the new subcommands so the two shell helpers it solely
consumes retire alongside the hooks.

**Scope of the hook surface**: this story owns the two VCS hooks
(`vcs-detect.sh`, `vcs-guard.sh`) and the `config-detect.sh` wrapper's
registration — three of the five `hooks.json` entries.
`migrate-discoverability.sh` belongs to 0172; `launcher-link-refresh.sh` is
untouched. The SessionStart config *behaviour* shipped with 0167 as `accelerator
config summary --format=hook --fail-safe`; there is no `config detect`
subcommand and none is added, but this story folds that registration into
`hooks.json` and deletes the wrapper.

Three concerns that once lived here are now siblings: the bootstrap exec-probe
fix (0186), the sub-binary registration surface (0187), and the `gix`/`jj-lib`
adapter swap (0188). This story consumes all three.

## Terminology

- **bootstrap** — `bin/accelerator`, the bash script registered in `hooks.json`.
- **launcher binary** — the cached Rust binary the bootstrap execs, package
  `accelerator`.
- **verify shim** — `bin/accelerator-verify-<platform>`, the minisign verifier
  the bootstrap stages and runs.
- **VCS subdomain** — the conceptual grouping of `vcs detect|status|log|guard`.
- **`vcs` / `vcs-adapters`** — the domain and adapter crates from 0179, extended
  with library-backed implementations by 0188.
- **`accelerator-vcs`** — the Cargo **package** producing the dispatched
  sub-binary, in a directory other than `cli/vcs/`. Never used for the crates or
  the subdomain.
- **dispatch token** — `vcs`, the argv token users type.
- **probe layer** — the *shell* detection functions in `scripts/vcs-common.sh`
  (`classify_checkout` and friends). Never abbreviated to "probe".
- **the wrapper** — `hooks/config-detect.sh` specifically.
- **the parity gate** — `hooks/test-vcs-detect.sh`, the 42-case suite. Distinct
  from `cli/corpus-adapters`' metadata parity suite.
- **plugin-root variables** — Claude-Code-substituted surfaces (`hooks.json`,
  `SKILL.md`) use `${CLAUDE_PLUGIN_ROOT}`; Rust under `cli/**` must use
  `ACCELERATOR_PLUGIN_ROOT` and is forbidden from naming any `CLAUDE_*` variable
  by `tasks/lint/claude_coupling.py`.

## Context

`scripts/vcs-common.sh` backs both the VCS skills and the hooks. ADR-0048 says
hook logic moves into the CLI. Two user-visible symptoms motivate it beyond the
refactor: `skills/planning/validate-plan` is **blocked in pure-jj repos today**
because `log` and `diff` sit in the guard's blocked set, and the colocated
"prefer jj" warning has never reached a user because the shell nests it where
the hook schema has no field.

Only the second is fixed here. **Unblocking `log`/`diff` is deliberately out of
scope**: the blocked set is reproduced verbatim under the parity requirement, so
`validate-plan` stays blocked in pure-jj repos after this story. Changing that
set is a user-facing policy decision that should not ride inside a migration
whose whole safety argument is decision-parity; it is raised as a follow-up by
the hand-off criterion. What the migration does deliver is making it a one-line
change afterwards, which is the sense in which the symptom motivates this work.

This story removes the **hooks'** copy of the detection logic and retires the
two status/log helpers. The lexical shell layer (`find_repo_root`, `vcs_mode`)
survives for its 20+ non-VCS callers, so duplication is reduced, not eliminated
— see the 0125 note in Dependencies.

`hooks.json` registers **five** hooks across **two owning stories plus one
unowned**: four SessionStart (`vcs-detect.sh` → this story; `config-detect.sh` →
this story, registration only; `migrate-discoverability.sh` → 0172;
`launcher-link-refresh.sh` → **no epic-0136 story**) and one PreToolUse
(`vcs-guard.sh` → this story).

## The guard's inputs

Stated authoritatively here so the decision table's row count is derivable
without consulting the capture. Blocked git subcommands, from
`hooks/vcs-guard.sh:53` (13):

```
status  diff  add  commit  log  branch  checkout  switch
merge   rebase  reset  stash  show
```

`log` and `diff` are **blocked**, not allowed. Everything outside the set
passes; the 7 named allowed subcommands are `push`, `pull`, `fetch`, `remote`,
`clone`, `config`, `tag`. `gh` and `rtk` are allowed unconditionally. **Compound
rule**: the command splits on `&&`, `||`, `;` and `|`, and is blocked if *any*
segment matches; the first matching segment names the reported subcommand.

## Requirements

- Implement the VCS subdomain as a hexagon over the `vcs`/`vcs-adapters` crates,
  using the library-backed adapters 0188 delivers. This story adds no
  dependencies and changes no dependency policy.
- Extend the crates with the `classify_checkout` taxonomy. The **authoritative
  arm list**, stated once and referenced by the fixture criterion, is: `main`,
  `jj-secondary`, `git-worktree`, `colocated`, `nested-jj-in-git`,
  `nested-git-in-jj`, `none` — with the submodule, bare and `GIT_DIR` handling
  that feeds them. First-match-wins ordering is load-bearing and `colocated`
  must precede the `nested-*` arms. No classifications beyond what the shell
  produces.
- **Normative parity reference, per subcommand** — mode detection is duplicated
  four ways today and the hooks do not call `vcs_mode()`, so "reproduce the
  shell" needs a named target each:
  - `vcs detect` → `hooks/vcs-detect.sh` (three-valued:
    `git`/`jj`/`jj-colocated`)
  - `vcs guard` → `hooks/vcs-guard.sh`
  - `vcs status` / `vcs log` → `scripts/vcs-status.sh` / `scripts/vcs-log.sh`
  - `classify_checkout` semantics → `scripts/vcs-common.sh`
- **Declared behavioural changes** — exactly four departures, each tested as
  the *new* behaviour:
  1. the PreToolUse envelope moves to the `permissionDecision` shape;
  2. the `.git`-as-*file* colocated misclassification is **corrected**, for `vcs
     detect` and `vcs guard` only. Those two test `-d "$REPO_ROOT/.git"`
     (`vcs-detect.sh:29`, `vcs-guard.sh:77`), so a colocated checkout whose
     `.git` is a *file* (worktree, submodule) is misread as pure-jj.
     `vcs-status.sh:9` and `vcs-log.sh:9` branch on `-d "$REPO_ROOT/.jj"` alone
     and never inspect `.git`, so they are **unaffected** and stay strict
     parity. The concrete change, stated as values because the shell cannot be
     the oracle for a deliberate departure:

     | Subcommand | Today (`.git` is a file) | After |
     | --- | --- | --- |
     | `classify_checkout` | `main` (git side unseen) | `colocated` |
     | `vcs detect` mode | `jj` | `jj-colocated` |
     | `vcs guard` | **blocks** (pure-jj) | **warns** (colocated) |

     Everything else is parity.
  3. `vcs detect`'s default output narrows to structured-only (the boundary
     block, or nothing when there is no boundary — satisfying the
     success-with-nothing-to-report contract cleanly). The shell's
     unconditional "VCS Command Reference" text (`vcs-detect.sh:40-82`) moves
     behind a new `--descriptive` flag. The registered SessionStart command
     passes `--descriptive`, so the user-visible transcript is unchanged;
     no other caller of `vcs detect` is known to exist.
  4. `vcs guard`'s compound-command splitter is **quote-aware**, not a port of
     the shell's quote-blind `sed`-then-split (`hooks/vcs-guard.sh:44-70`).
     The shell wrongly splits inside a quoted string — `git commit -m "build
     && test"` is misread as two segments — and this is a parsing defect, not
     a policy decision worth carrying forward under a "parity" banner. The
     Rust splitter tracks single-/double-quote state and only treats
     `&&`/`||`/`;`/`|` as real separators when unquoted. Tested as the new
     behaviour with a hand-authored decision-table row (not captured from the
     shell, which gets this case wrong) — the same treatment as departure 2.
- Migrate the hook logic into the CLI. **The three registered command strings,
  verbatim** (the parity gate asserts the exact literal):
  - SessionStart: `${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs detect
    --format=hook --fail-safe --descriptive`
  - SessionStart: `${CLAUDE_PLUGIN_ROOT}/bin/accelerator config summary
    --format=hook --fail-safe`
  - PreToolUse(`Bash`): `${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs guard
    --format=hook --fail-safe`

  `hooks/config-detect.sh` is deleted once its registration is inlined. No
  hook-specific subcommand. The migration-discoverability reminder stays until
  0172.
- **Fail-open on both hook event types.** `--fail-safe` is on all three
  registrations because a PreToolUse hook exiting non-zero is a *blocking
  error*: any failure — a failed first-use fetch, an unreachable release host,
  an unreadable repository — must let the Bash call through. The bootstrap
  honours `--fail-safe` as a global token (`bin/accelerator:28-39`), so
  bootstrap and trust-chain aborts inherit it. **Scope note**: "any failure"
  means any *external-dispatch resolution* failure — `kernel::Error::LogFilter`
  (a malformed `ACCELERATOR_LOG` value, surfaced by `kernel::logging::init()`
  before dispatch even runs) is deliberately excluded, since it is a
  pre-dispatch, developer-controlled logging setup step shared by every
  subcommand, not a per-subcommand availability or integrity failure — see
  the plan's Phase 5, item 1.
- **Output contract, stated once.** `--format=hook` renders a per-hook-type
  envelope. The two non-normal outcomes are **disjoint by definition** — one is
  a success, the other an error, and no run is both:
  - **Success with nothing to report** (the adapter answered; there is simply no
    context — e.g. a main checkout with no boundary, or no repository at all):
    **zero bytes**, exit 0. For `vcs detect` this applies to the default,
    non-`--descriptive` invocation only — `--descriptive` always carries the
    reference text regardless of boundary presence, per the third declared
    departure above.
  - **Adapter failure** (the adapter could not answer — unreadable or corrupt
    repository): **exactly one JSON object containing `systemMessage`**, exit 0
    under `--fail-safe`. On stdout, not stderr, because 0183 establishes stderr
    at exit 0 is discarded.

  At most one object reaches stdout, so `systemMessage` merges into the same
  object as any `hookSpecificOutput`. `--fail-safe` governs the exit code only.
  Without `--format=hook`, `vcs detect` emits the same context text unwrapped
  and `vcs guard` emits a human-readable allow/deny line; both plain forms are
  pinned by goldens so they cannot drift from the enveloped ones.
- Serve the guard as a dispatched sub-binary invoked through the bootstrap. The
  **package** is `accelerator-vcs` in a directory other than `cli/vcs/`, with
  the dispatch token `vcs` mapped via `_SUBBINARY_MANIFESTS`. Registration
  follows 0187's checklist — this story adds the token, it does not generalise
  the surface.
- Repoint `skills/vcs/commit` at the new subcommands and delete the two shell
  scripts it solely consumes. `SKILL.md:13-14` are their only references in the
  repo, and `tasks/lint/call_site_migration.py` guards only `scripts/config-`
  call sites, so nothing would catch the omission. This also lets the skill's
  broad `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/*)` permission be dropped for a
  subcommand-scoped `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs *)`.

## Sequencing Constraints

1. **Confirm the hook schema at the floor before designing the envelope.** If
   `permissionDecision` is not honoured at Claude Code v2.1.144, that decision
   must be revisited. Minutes-long empirical check; gates planning.
   **RESOLVED 2026-08-05**: both `permissionDecision:"deny"` and a bare
   top-level `systemMessage` (no `permissionDecision` key) are honoured, at
   both the current client and the declared floor (v2.1.144, run via `npx
   @anthropic-ai/claude-code@2.1.144`). See Validation Results. The envelope
   design in this work item and its plan stands as specified; no exit-2
   fallback is needed.
2. **Capture the shell's behaviour as fixtures before deleting any of it**, in a
   commit that precedes the deletion commit so the ordering is checkable from
   history. `hooks/vcs-guard.sh` has zero coverage today and the status/log
   comparators are removed by this story.
3. **Land the `test:integration:hooks` launcher edge before repointing the
   parity gate** — it cannot run against a binary until the task gains
   `build:cli:dev` and `accelerator_env()`.
4. **Do not ship the `hooks.json` rewrite ahead of a release whose manifest
   lists `accelerator-vcs`.** The dev path (`build:cli:dev` plus
   `ACCELERATOR_VCS_BIN`) is unaffected; the installed-plugin path is not.
   **Revised 2026-08-05**: a missing `accelerator-vcs` manifest entry is a
   `Failed`-class `ResolutionError` (`AssetNotFound`), which the plan's
   strengthened fail-safe fix (Phase 5) already swallows gracefully —
   distinct from the integrity-class failures (`SignatureMismatch` etc.)
   that fix deliberately does *not* swallow. So a premature merge degrades
   to the guard silently not protecting anyone on that release, not a hard
   failure across every installed plugin. The ordering constraint still
   stands (silent non-protection is still worth avoiding), but its severity
   is narrower than "hard failure" implies.

## Acceptance Criteria

- [x] **The envelope is honoured at the declared floor**: on Claude Code
      v2.1.144, a real Bash call to a blocked git subcommand in a pure-jj repo
      is denied, and the colocated warning appears in the session transcript.
      Recorded in Validation Results with the observed client version.
      **Confirmed 2026-08-05** via a synthetic PreToolUse hook exercising both
      shapes directly (ahead of the guard's own implementation, which is
      re-verified end to end in Phase 10 of the plan) — no exit-2 fallback is
      needed.
- [x] **Shell behaviour captured as committed fixtures before any deletion**, in
      a commit preceding the deletion commit: the `vcs status`/`vcs log` goldens
      and the `vcs guard` decision table. The volatile-field mask set is derived
      from the capture, enumerated per subcommand in the plan, committed as a
      file alongside the goldens, and closed thereafter — no mask may be added
      to make a failing golden pass. It covers at least: hex object ids of 7-40
      characters, jj change ids (32-character non-hex), ISO-8601 *and* jj's
      space-separated timestamps, relative age strings, the fixture tempdir
      path, and author identity.
- [x] `accelerator vcs detect --descriptive` reproduces `hooks/vcs-detect.sh`,
      verified against the repointed parity gate. The two goldens
      (`hooks/test-fixtures/vcs-detect/*.json`) are compared after `jq -S .`
      canonicalisation on both sides and must equal the current shell-produced
      content; a value difference fails. Separately, a **third** detect fixture
      covers the corrected case: a colocated checkout whose `.git` is a file
      must report mode `jj-colocated` (today: `jj`). Its golden is **authored as
      the new expectation and marked in the fixture as a deliberate
      divergence**, not derived from the capture — the shell is not the oracle
      for a declared behavioural change. A **fourth** fixture pins
      `vcs detect`'s default (non-`--descriptive`) structured-only output,
      which has no shell equivalent — the third declared departure.
- [x] **The 42 parity-gate cases partition into five disjoint buckets**, each
      case in exactly one, stated in the plan as a line-range partition
      summing to 42 (**revised 2026-08-05** — corrected from an earlier
      four-bucket, 27-case count that undercounted a host-artefact case the
      plan's own Key Discoveries derives by reading the full 713-line file):
      (a) 26 in-process `vcs-common.sh` cases → moved to
      `scripts/test-vcs-common.sh`; (b) 8 subprocess cases that repoint by
      changing the `HOOK` constant; (c) 5 missing-`jj`/`git`-binary cases →
      deleted, since no external binary is consulted; (d) two singletons — the
      comment-block grep case → deleted, and the `hooks.json` literal assertion
      → updated to the new registration and made order-independent rather than
      pinned to `SessionStart[0]`; (e) one static golden-snapshot host-artefact
      check (`hooks/test-vcs-detect.sh:196-202`) that calls neither a
      `vcs-common.sh` function nor `$HOOK` — preserved in place, now asserting
      the Rust-produced goldens are equally free of host-specific path
      leakage. 26+8+5+2+1 = 42.
- [x] `accelerator vcs status` and `accelerator vcs log` match the captured
      goldens across: clean git, dirty git, git ahead/behind, detached-HEAD git,
      clean jj, dirty jj, colocated, jj secondary workspace, and no repository
      at all. (No `.git`-as-file case: `vcs-status.sh:9`/`vcs-log.sh:9` branch
      on `.jj` alone, so the correction does not reach them and strict parity
      holds.) Definitions, so two people build them identically — "dirty" is one
      untracked plus one modified tracked file (plus one staged change for git);
      "ahead/behind" is a local clone two commits ahead and one behind upstream;
      the no-repository expectation is the shell's own behaviour, captured with
      the rest.
- [x] `accelerator vcs guard` reproduces the captured decision table. The row
      count is **34 command cases × 4 repo modes = 136**, plus the
      `.git`-as-file colocated case, plus one quote-aware compound-split
      departure case (`git commit -m "build && test"` against pure-jj — the
      quoted `&&` must not be treated as a separator, the fourth declared
      departure; see below) = **138**: 13 blocked subcommands + 7 allowed +
      `gh` + `rtk` + 12 compound cases (4 separators × match-first /
      match-later / no-match) =
      34; repo modes are pure-jj, colocated, git, non-repo. The multiplier
      applies uniformly, so the total is checkable against the capture.
- [x] The `classify_checkout` port is verified by a fixture covering every arm
      in the authoritative list, asserting first-match-wins for the named
      ambiguous case: **a colocated checkout nested inside another repository
      classifies as `colocated`, not `nested-*`**. The set is asserted
      **closed** — exactly the seven named variants — so a superset taxonomy is
      detectable.
- [x] `hooks.json` registers the three command strings verbatim. A golden exists
      **per emitted output shape**; the plan enumerates the final list, which
      includes at least: SessionStart with `systemMessage`, SessionStart
      without, plain `vcs detect` (structured and `--descriptive`), plain
      `vcs guard`, PreToolUse deny, and PreToolUse warn-only.
- [x] The PreToolUse guard emits
      `{hookSpecificOutput:{hookEventName:"PreToolUse",
      permissionDecision:"deny", permissionDecisionReason:…}}` for a pure-jj
      block. The colocated "warn but permit" case emits a bare top-level
      `{systemMessage:…}` with **no** `permissionDecision` — `"allow"` skips the
      interactive permission prompt and would widen privilege under cover of a
      format change; `"ask"` would force a prompt where none is forced today.
- [x] **The guard fails open, three ways.** (a) Release host unreachable with an
      empty cache — `ACCELERATOR_RELEASE_BASE_URL` at a dead address — the hook
      exits 0 and emits no blocking envelope. (b) Host reachable but serving a
      manifest with no `accelerator-vcs` entry — same outcome. (c) A corrupt
      repository fixture (`.git/HEAD` truncated to non-ref bytes) — exits 0 and
      emits no `permissionDecision`. Fault injection is by a test-only failing
      adapter or a named env override, never by file permissions, so none of
      these can pass vacuously under root.
- [x] **The two non-normal outputs are pinned separately**, matching the
      disjoint contract: (a) *success with nothing to report* — in a main
      checkout with no boundary, `vcs detect --format=hook --fail-safe` exits 0
      and writes exactly zero bytes to stdout; (b) *adapter failure* — with a
      test-only failing adapter, the same command exits 0 and writes exactly one
      JSON object containing `systemMessage`. The guard's failure output is
      pinned by the same rule: its corrupt-repository case emits that one object
      and no `permissionDecision`.
- [ ] `accelerator-vcs` is registered per 0187's checklist and ships end to end:
      it appears in the generated manifest with a description and signature;
      `validate_dispatch_coherence` (generalised by 0187) covers the `vcs`
      token; `cargo deny`, `cargo-pup` and `--locked` clippy pass; the musl
      build passes `_assert_static_elf`. **Partially verified 2026-08-06**:
      manifest/description generation, `validate_dispatch_coherence`, `cargo
      deny`, `cargo-pup` and `--locked` clippy all confirmed green via `mise
      run`. The musl cross-compile (`build:cli:cross-compile`) is a
      release-pipeline-only task, not a `mise run` dependency, so
      `_assert_static_elf` has not yet been exercised against a real
      cross-compiled `accelerator-vcs` binary — left unchecked pending the
      next release build alongside the release-cut criterion below.
- [ ] **The release precedes the rewrite.** A published manifest listing
      `accelerator-vcs` exists before the `hooks.json` rewrite reaches an
      installed-plugin path, verified against the *published* manifest rather
      than the locally generated one. (The reachable-host-but-missing-entry case
      is covered by the fail-open criterion; this one covers the ordering.)
- [x] After first use, the guard resolves from the warmed cache — zero
      sub-binary fetch invocations against a stubbed fetcher across repeated
      calls.
- [ ] **Warm-call latency**, host-relative: in one session on one darwin-arm64
      host with no build running, capture the median of 20 `hooks/vcs-guard.sh`
      invocations (**B**) and 20 warm `accelerator vcs guard` invocations
      (**G**), using the same stdin payload (a blocked `git status` call)
      against the same pure-jj fixture. Acceptance requires **G ≤ 1.1 × B**.
      Record B, G, the ratio, the payload, the fixture and the host in
      Validation Results. Not a CI job — which means "not automated", not "not
      required".
      **Resolved 2026-08-17 — recorded, not met, and discharged on 0189.**
      Measured on a quiet darwin-arm64 host (Apple M4 Max, macOS 26.3) under the
      committed harness `mise run measure:warm-dispatch`, third and valid
      attempt, record at `meta/measurements/warm-dispatch-3.json`:
      **B = 26.796 ms**, **G = 35.531 ms**, **ratio of medians = 1.3260**
      (two-sided 95% paired-bootstrap CI [1.3236, 1.3279], n = 2,659 interleaved
      pairs). Payload: the `PreToolUse` envelope for a blocked `git status`.
      Fixture: a fresh pure-jj scratch repository, colocation pinned off.

      **This criterion stays unticked: `G ≤ 1.1 × B` was not met, and neither
      was the 1.3 that superseded it.** The threshold is superseded twice over —
      1.1 → 1.3 → **1.4**, the last on 2026-08-17 by author decision — and the
      ratio is demoted beneath an absolute `median`/`p90` budget, which the same
      session clears with 26% to 36% of headroom. The obligation is discharged
      at **work item 0189**, whose Latency Criterion is authoritative; ticking a
      box whose stated threshold was not met would be the post-hoc relaxation
      that criterion records as a deviation.

      **Why 1.1 was never reachable**: it was calibrated against a cost model
      attributing `jj` and `git` spawns to this baseline. The recovered guard
      calls `find_repo_root` (`scripts/vcs-common.sh:8-18`), a pure-bash upward
      walk, and decides mode by two literal `[ -d ]` tests — there is no `jj`
      spawn and no `git` spawn anywhere in B. B does two directory-entry tests
      where G loads the repository through jj-lib behind a verified signature
      chain, so no ratio between them is a like-for-like comparison.

      ⚠️ **The criterion decision was taken on 0189, not here**, contrary to
      0205's recommendation that "0169 decides what the criterion should be"
      before 0189 measures. 0169 is closed, so the decision was landed on the
      open item that inherited the obligation.

      ⚠️ **Method deviations from this criterion as written**: n is 2,659 pairs
      rather than 20; the gate is an interval's upper bound rather than a bare
      median comparison; the ratio is demoted beneath an absolute budget; and
      the criterion is split per digest backend into six identified cells. Full
      record and rationale in
      `meta/plans/2026-08-11-0189-warm-dispatch-latency-measurement.md`.
- [x] `skills/vcs/commit/SKILL.md:13-14` invoke
      `${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs status --fail-safe` and
      `… vcs log --fail-safe` (the flag lets a cold-cache dispatch failure
      degrade the skill's injected context rather than failing the
      invocation, per the External systems dependency note below), and
      its `allowed-tools` drops `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/*)` for
      `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator vcs *)`. Verified by (a) the
      permission lint and the count of SKILL.md files using `!` injection
      staying at 42 (`tasks/lint/skill_permissions.py:39`,
      `EXPECTED_INJECTION_SKILLS` — this replaces two sites *within one
      already-counted skill*, so the file count is unaffected), and (b) a
      runtime check that both commands exit 0 and emit non-empty output in a
      fixture repo.
- [x] `hooks/vcs-detect.sh`, `hooks/vcs-guard.sh`, `hooks/config-detect.sh`,
      `scripts/vcs-status.sh` and `scripts/vcs-log.sh` are removed, and prose
      documentation naming any of them (`README.md`, `docs/internals.md`,
      `tasks/README.md`) is updated to the new subcommands — no listed guard
      would catch a stale reference. The hooks suite floor
      (`tasks/test/integration.py:47`) **remains 2, unchanged** — the two
      surviving suites are `hooks/test-migrate-discoverability.sh` and the
      repointed parity gate — and is re-verified green after the deletions. All
      five removed files are entrypoints, so `SHELL_LIBRARIES` needs no edit.
- [x] **The downstream hand-offs are raised** as a dated note appended to each
      receiving work item's Dependencies section — 0172 (proposed `blocked_by:
      work-item:0169` plus `hooks/migrate-discoverability.sh` in its source
      list), 0183 (`accelerator vcs detect` is a new SessionStart site for its
      audit), and 0125 (the in-process adapter dissolves its lexical-fallback
      rationale) — plus **the implementer creates** two follow-up work items:
      one owning the `scripts/vcs-common.sh` residue and
      `hooks/launcher-link-refresh.sh`, and one owning the decision on whether
      `log`/`diff` leave the guard's blocked set (which would unblock
      `skills/planning/validate-plan` in pure-jj repos). Appending the notes and
      creating the two items are in scope; re-scoping existing items is not.
- [x] `mise run` is green end to end.

## Open Questions

- ~~Are `permissionDecision` and top-level `systemMessage` honoured at Claude
  Code v2.1.144?~~ **RESOLVED 2026-08-05** — yes, both are honoured at the
  floor. See Sequencing Constraint 1 and Validation Results.
- Where does the shared hook-envelope module live? 0167's renderer has no
  `systemMessage` slot, and cargo-pup forbids `config_command` importing
  `crate::launch`. **Default if unresolved: a new module in the launcher**
  (`crate::hooks::envelope`), imported by both renderers; `kernel` is the
  alternative. 0170/0171/0173 inherit whichever is chosen.
- Is `hooks.json`'s `args` exec form available at the floor? **Default: register
  the shell form**, which works either way.

## Dependencies

- **Blocked by**: 0164 (fetch-verify-cache — **done**), 0166 (shared crates —
  **done**), 0179 (the `vcs`/`vcs-adapters` crates — **done**), 0167 (bootstrap
  invocation contract and the `config` command — **code landed on `main`**; its
  work item is still `ready`, so **close out 0167's status before this story
  starts** rather than at its acceptance, or the edge stays stale throughout),
  0186 (exec-probe fix — the latency criterion measures against a fixed
  bootstrap), 0187 (registration surface — this story adds a token to it), and
  0188 (library-backed adapters — the subdomain is built on them).
- **Completed dependencies not in `blocked_by`**: 0165 (distribution pipeline —
  **done**).
- **Hand-off note from 0186 (2026-07-31): 0186 is necessary but may not be
  sufficient for this story's warm-call latency criterion.** 0186 removes the
  ~108 ms exec probe but deliberately retains the verify shim's second
  `sha256_file` (~11.7 ms of a ~23 ms staging cost), because three existing
  tests
  in `tests/integration/entrypoint/test_accelerator_entrypoint.py` assert the
  planted-stub defence it provides — removing it would weaken a tested trust
  boundary, so no work item owns that residual. Expect a warm bootstrap around
  41 ms, against this story's `G ≤ 1.1 × B` gate of ≈ 38.6 ms, with a sub-binary
  exec and verify on top. **Resolve this before acceptance**: either relax the
  threshold, or accept the overrun with a stated rationale. See 0186's
  Dependencies and Validation Results.
- **Hand-off note re-confirmed with measured figures (2026-08-03).** The note
  above still holds in shape, but three of its numbers were wrong and the
  headline moved in this story's favour. **This story's threshold and its
  rationale are untouched — that decision remains 0169's own work.**
  - **The warm bootstrap lands at ~30 ms, not ~41 ms.** Measured on
    darwin-arm64 (Apple M4 Max, macOS 26.3), 50 interleaved samples per variant
    in one process: median **125.35 ms before**, **29.92 ms after**, gate
    `after ≤ 0.5 × before` passed at a ratio of 0.239. Instrument floors 1.60 ms
    (`/usr/bin/true`) and 6.10 ms (a trivial bash script). The 149.1 ms in
    0186's Context table is a pre-0182 reference produced by an unrecorded
    method and is **not** method-comparable to these medians.
  - **The retained residual is backend-dependent, and the ~11.7 ms figure
    describes the wrong backend.** Where `command -v sha256sum` resolves to
    Apple's `/sbin/sha256sum` — as it does on the measuring host — one
    `sha256_file` call costs **~3.5 ms** and both together **~7 ms**. Where only
    the Perl `shasum` fallback exists it is **~12 ms** per call and **~24 ms**
    for both. Take the range and check `command -v sha256sum` on the target
    host; do not carry a point estimate.
  - **The measured composition of the ~30 ms**, for budgeting the sub-binary
    dispatch on top: 6.1 ms bash interpreter startup, ~7 ms for the two hashes,
    ~6.8 ms for the staged shim exec plus minisign verify (note: not the 2.3 ms
    the Context table quotes for minisign alone — the shim's own startup and its
    read of the 7.6 MB launcher dominate), ~2.4 ms for the launcher exec, and
    ~4.6 ms across two `uname` substitutions, the `plugin.json` `sed`, two
    `cd -P`/`pwd -P` subshells and the `resolve_cache_dir` substitution. ~2.9 ms
    (≈10%) unexplained.
  - **The dominant unaddressed cost is now the launcher-side probe, raised as
    0189.** `cache_root::resolve` runs the identical write-chmod-exec probe on
    **every** external-subcommand dispatch, before the sub-binary cache-hit
    test — so a warm `accelerator vcs guard` still pays a first-exec penalty of
    the same shape, measured at **131.97 ms** in the repo's `bin/` (107.15 ms in
    `/tmp`) against a 3.72 ms re-exec of a file left in place. Built-ins never
    reach it, which is why 0186's `version`-based measurement cannot see it.
    **0189 is necessary but not sufficient for this story either, and this
    story's threshold decision must not be deferred pending it.**
  - **Retracted 2026-08-11.** The two paragraphs above no longer describe the
    tree. This story's own Phase 5 absorbed the launcher-side fix: `candidate`
    is selection-only and the probe (`verify_writable`) runs only from
    `FetchVerifyCacheResolver::fetch_verify_store`, never on a warm hit. The
    launcher probe is therefore **not** an unaddressed cost and 0189 does not
    gate this story's latency work — nothing waits on it. 0189 now carries only
    a regression guard plus the Phase 10 measurement this story left unmeasured
    at closure; the ~132 ms figure above is pre-fix history. 0191 remains the
    cheapest outstanding lever.
  - **The cheapest remaining lever is 0191**: batching the two shim hashes into
    one `sha256sum f1 f2` with no `awk`, measured at ~2.5 ms — essentially this
    story's whole shortfall. It needs a branch to preserve today's
    short-circuit, which is why it was not absorbed into 0186.
  - The host runs no third-party EndpointSecurity or anti-malware agent (only
    Apple's `xprotectd`, SIP and Gatekeeper on), so the first-exec penalty is
    stock macOS behaviour rather than a machine artefact and should transfer.
- **In-flight dependency**: 0182 (plugin-root self-location — **`in-progress`**,
  code landed). It delivers the `ACCELERATOR_PLUGIN_ROOT` regime the new
  `cli/**` code must use, and `hooks/launcher-link-refresh.sh`, which must
  survive untouched. It also edits `bin/accelerator`, as does 0186 — those two
  must be sequenced against each other.
- **External systems**: the **Claude Code hook I/O schema** at floor v2.1.144
  (see the first Open Question); the **release-artefact host**, on which both
  the guard hook *and* the repointed `skills/vcs/commit` depend for their first
  invocation — the fail-open posture covers the hook paths, and the skill
  degrades its injected context rather than failing when the sub-binary cannot
  be fetched.
- **Process prerequisite — the release cut.** Sequencing Constraint 4 forbids
  shipping the `hooks.json` rewrite ahead of a release whose manifest lists
  `accelerator-vcs`. **Severity revised 2026-08-05, see Sequencing Constraint
  4**: a missing entry is `Failed`-class and covered by Phase 5's fail-safe
  swallow, so this degrades to the guard silently not protecting anyone on
  that release, not a hard failure across every installed plugin. The
  ordering constraint still stands regardless. That release is **not**
  produced by this story's code changes — it requires a release run and the
  minisign signing key. Owner: whoever performs epic-0136 releases. It must
  be scheduled as part of accepting this story, and is gated by an acceptance
  criterion rather than left to the sequencing constraint alone.
- **Blocks**: 0172 (its migrate-discoverability migration builds on this
  `hooks.json` rewrite; 0172 currently records **no `blocked_by` at all**);
  0170, 0171, 0173 (each ships its own sub-binary and inherits the hook-envelope
  module home decided here); 0174 (retires the surviving shell tooling).
- **Related**: 0125 — not closed here; this story leaves the shell probe and
  lexical layers alive alongside the Rust classifier. 0183 — owns the
  SessionStart audit this story adds a site to. 0185 — converges
  `corpus-adapters` and deletes `CommandProbe`, blocked by 0188 rather than by
  this story.
- **Unowned debt this story creates**: `classify_checkout` and the probe-layer
  helpers in `scripts/vcs-common.sh` lose every production caller while
  `find_repo_root` (20+ callers) and `vcs_mode` (1) survive. **0174's recorded
  scope is tooling and CI guards and never names `vcs-common.sh` or any hook
  script**, so despite the `blocks: 0174` edge it does not claim this.
  Separately, `hooks/launcher-link-refresh.sh` is claimed by no epic-0136 story.
  An acceptance criterion requires a follow-up item owning both.

  **Resolved 2026-08-06**: [`0199`](0199-retire-vcs-common-sh-residue-and-launcher-link-refresh.md)
  now owns both — the `find_repo_root`/`vcs_mode` residue (with
  `classify_checkout`'s fate as part of its inventory) and
  `hooks/launcher-link-refresh.sh`. Separately,
  [`0200`](0200-decide-vcs-guard-log-diff-blocklist-membership.md) now owns
  the `log`/`diff` blocklist-membership decision this story's Phase 1/Phase 7
  declined to make (see the parity-scope note above and Phase 10 of the
  implementation plan). A third follow-up,
  [`0198`](0198-migrate-vcs-status-log-onto-library-backed-adapters.md), owns
  the deliberate subprocess-not-library choice for `vcs status`/`vcs log`
  themselves — Key Discoveries' "no byte-parity guarantee" note above is
  where that scope originates.
- **Parent**: epic 0136.

## Assumptions

- The four target platforms (`tasks/shared/targets.py`) are all Unix.
- The darwin-arm64 latency reference (B ≈ 35 ms) is representative of the host
  the parity measurement runs on. The measurement is host-relative precisely so
  this assumption need not hold exactly.

## Technical Notes

- Source bash: `scripts/vcs-common.sh`, `scripts/vcs-status.sh`,
  `scripts/vcs-log.sh`, `hooks/vcs-detect.sh`, `hooks/vcs-guard.sh`,
  `hooks/config-detect.sh`, `hooks/hooks.json`, and the
  `hooks/test-fixtures/vcs-detect/` fixtures.
- **Mode detection is duplicated four ways** and the hooks do not use
  `vcs_mode()`: `vcs-common.sh:27-36` tests `-e`, while `vcs-detect.sh:28-36`,
  `vcs-guard.sh:22,77`, `vcs-status.sh:9` and `vcs-log.sh:9` each inline `-d`.
  Hence the per-subcommand normative reference, and hence the `.git`-as-file
  correction applying to all four.
- Behavioural reference (`path:line`): jj-outranks-git dispatch
  (`vcs-common.sh:27-36`); `classify_checkout` contract and six-line record
  (`:157-176`, body `:177-280`); load-bearing arm cascade (`:240-272`);
  SessionStart envelope (`vcs-detect.sh:177-181`); the deprecated PreToolUse
  shapes deliberately not reproduced (`vcs-guard.sh:97-108`); guard
  command-parsing (`vcs-guard.sh:44-108`).
- At most one JSON object may reach stdout — see
  `hooks/launcher-link-refresh.sh:16-27` for the accumulate-then-emit-once
  pattern. The parity gate extracts via `jq -r
  '.hookSpecificOutput.additionalContext'`, so compact-versus-pretty rendering
  is transparent to it.
- New code under `cli/**` must not name any `CLAUDE_*` variable
  (`tasks/lint/claude_coupling.py`). The parity gate's `CLAUDE_PLUGIN_ROOT=`
  overlay changes accordingly, and **`test:integration:hooks` gaining a
  `build:cli:dev` dependency plus `accelerator_env()` is in scope for this
  story**, including the `tests/unit/tasks/test_mise.py` pin update that
  currently asserts their absence.
- The launcher dispatches any new external subcommand with no launcher changes
  (`cli/launcher/src/launch/inbound/cli.rs:15-22`).

## Notes from 0167 (2026-07-22)

- **The SessionStart envelope contract is settled by 0167** and inherited here:
  `accelerator config summary --format=hook` emits
  `{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"…"}}`
  (compact `serde_json`), emits **nothing** when the summary is empty, and with
  `--fail-safe` exits 0 on a read/IO failure. `vcs detect` slots into the same
  `additionalContext` shape but must **extend** it with the optional
  `systemMessage` sibling that 0167's renderer
  (`cli/launcher/src/config_command/render/summary.rs:66-74`) has no slot for.
- **The registration names the bootstrap**, not a "universal wrapper".
- **The `hooks.json` argument-splitting question is RESOLVED.** The `command`
  field is handed to `sh -c` in shell form, so `${CLAUDE_PLUGIN_ROOT}` *is*
  expanded and argument tokens *are* split; an alternative exec form takes a
  sibling `args` array passed verbatim with no shell. Either way no wrapper is
  needed, so `hooks/config-detect.sh` is inlined and deleted here.

## Amendment 2026-08-03 — 0188 has landed; what this story inherits

[`0188`](0188-library-backed-vcs-adapter.md) delivered the library-backed
adapter, the six taxonomy queries and the shared fixture matrix. This story
builds its classifier over them.

- **The six-query contract is closed.** `is_bare`, `worktree`, `superproject`,
  `jj_workspace_root`, `jj_repository` and `dual_roots` are inherent methods on
  `vcs_adapters::library::InProcessProbe`. Anything beyond them is a change to
  0188, not silent growth here. This story defines whatever **domain port** its
  classifier needs over them; 0188 deliberately defined none.
- **Widen the cargo-pup rule** (`vcs_adapters_library_reads_in_process`) to
  cover wherever `vcs status` / `vcs log` land. It is scoped to
  `^vcs_adapters::library($|::)` and pairs a permit list with an explicit
  `std::process` deny; the permit list rejects **grouped** imports, so that code
  must write one single-item `use` per import.
- **Reuse the named pure-jj builder** (`vcs_test_support::fixtures::pure_jj`)
  rather than rebuilding the benchmark fixture, so the cost figures stay
  comparable.
- **The 0125 hand-off sub-clause is redundant** — 0188 owns that note and it has
  been appended.

Seven findings that change how the classifier must be built:

1. **Three walks, not one.** Queries 4, 5 and the jj half of 6 resolve through a
   **`.jj`-only** walk. `DefaultWorkspaceLoaderFactory::create` performs no walk
   of its own, so feeding it the combined `.jj`-or-`.git` boundary makes it
   report absence on both nested-git-in-jj shapes — where `jj workspace root`
   reports a root. A classifier built on the combined walk reports absence where
   the oracle reports presence.
2. **Dual-root equality is necessary but not sufficient** for `colocated`, and
   inequality is not sufficient for either `nested-` arm. A real
   `jj git init --colocate` main has equal roots and classifies as `main`; the
   arms also need the jj-secondary bit (query 5) and the linked-worktree bit
   (query 2).
3. **`JS-in` is the shape that catches a naive dual-root rule** — a jj secondary
   workspace inside its own colocated main, which is *this repo's own*
   `workspaces/<name>` layout. Its dual roots **differ** while
   `classify_checkout` reports `jj-secondary`, because that arm's
   `jj_main_root != git_main_root` guard fails. It is in the matrix.
4. **The queries return `Result<Option<T>, Error>`**, so the classifier must
   decide what a repository the pinned library cannot parse classifies as. It is
   **not** `none`. `dual_roots` is infallible with a `Result` per side; callers
   comparing the sides must treat any `Err` as "not comparable", never as
   inequality.
5. **`WorktreeFacts`, `JjRepositoryFacts`, `JjWorkspaceRole` and `DualRoots`
   must move into `cli/vcs`** when this story defines its port. They are
   domain-shaped value types currently declared in the adapter crate, and
   `vcs_domain_imports_only_permitted` restricts `vcs` to `std`/`kernel::Error`/
   `crate` — so a domain port structurally cannot reference them where they sit.
   0188 could not pre-empt the move: its own criteria forbid touching
   `cli/vcs/src/**`. Expect it to churn the `queries.rs` expected-value tables.
6. **`superproject` gates on the git-dir vs common-dir comparison, not on
   `kind()`.** Measured, `kind()` is wrong in both directions: a linked worktree
   *of* a submodule reports `Submodule` and has **no** superproject, while a
   submodule *inside* a linked worktree does not report `Submodule` and does.
7. **gix 0.85 cannot read sha256 repositories** — every gix-backed query returns
   `Err`. Reftable reads normally.

Cost figures, for the `G <= 1.1 x B` gate (darwin-arm64, matching 0186's
`B = 35.1 ms`): the library-backed cold per-process path is **3.6-4.7 ms** for
all six queries plus both port methods, against **23.8 ms** for the single
`jj log -r @` the subprocess probe ran. Warm in-process calls are 14-50 us. Two
provenance notes carried forward: `B = 35.1 ms` is from 0186's table, not this
story's, and the "~41 ms warm bootstrap" this item cites is **derived**
(149.1 - 107.9), not measured.

## Validation Results

- **Claude Code floor check (schema pre-check, 2026-08-05)**: observed client
  versions — the then-current client, and the declared floor v2.1.144 (run via
  `npx @anthropic-ai/claude-code@2.1.144`); both `permissionDecision` and
  top-level `systemMessage` honoured — **yes, at both versions**: a synthetic
  PreToolUse hook returning `permissionDecision:"deny"` blocked the matching
  Bash call, and a bare `{"systemMessage":...}` (no `permissionDecision` key)
  let the call through while still surfacing the message; fallback taken —
  **none needed**. This pre-checks the raw schema ahead of the guard's own
  implementation; Phase 10's "Claude Code floor check" manual item re-verifies
  the same shapes end to end once `vcs guard` is built.
- **Warm-call latency**, filled 2026-08-17 from
  `meta/measurements/warm-dispatch-3.json` (third attempt, the first valid one):
  **B** — 26.796 ms median over 2,659 samples (min 25.182, p90 28.896, IQR
  1.192); **G** — 35.531 ms median over 2,659 samples (min 33.999, p90 38.230,
  IQR 1.642); **ratio** — 1.3260, 95% CI [1.3236, 1.3279], with the
  `true`-floor-subtracted robustness ratio at 1.3432 [1.3407, 1.3451] and the
  bash-floor-subtracted diagnostic at 1.3909; **payload** — the `PreToolUse`
  envelope for a blocked `git status`, byte-identical to both variants;
  **fixture** — a fresh pure-jj scratch repository outside the plugin root,
  `jj --config git.colocate=false git init`, asserted non-colocated; **host and
  OS** — Apple M4 Max, macOS 26.3 (darwin-arm64), load 3.81 over 16 CPUs,
  instrument floors 4.45/1.34 ms before and 4.26/1.35 ms after sampling.

  ⚠️ `G ≤ 1.1 × B` is **not met** and this hand-off's criterion above stays
  unticked. The obligation is discharged on work item 0189 under a superseding
  criterion; see the dated resolution on that criterion for the full reasoning
  and the method deviations.

## Drafting Notes

- **Split 2026-07-31**, after review-2 pass 4 measured that three editing passes
  had not reduced the major-finding count (19 → 15 → 14 → 14), with eleven of
  the fourteen pass-4 majors being defects introduced by the previous pass's
  fixes. Four concerns were extracted: 0186 (exec-probe fix), 0187 (registration
  surface), 0188 (library-backed adapters), and 0185 (corpus-adapters
  convergence, created earlier). This story keeps the subdomain, the hooks
  migration and the skill repoint — the three pieces that cannot be delivered
  separately, since the shell hooks cannot be deleted before their Rust
  replacements exist, and `vcs status`/`vcs log` would otherwise ship with no
  consumer.
- The PreToolUse envelope moved to the `permissionDecision` shape (2026-07-30),
  making the guard a deliberate behavioural change rather than a byte-for-byte
  port.
- Priority high: critical-path Phase 6 story gating the hooks migration.

## References

- Source:
  `meta/research/codebase/2026-06-28-0136-rust-cli-migration-scope-and-architecture.md`
- Implementation surface:
  `meta/research/codebase/2026-07-29-0169-vcs-subdomain-and-hooks-migration.md`
- Review driving the split:
  `meta/reviews/work/0169-vcs-subdomain-and-hooks-migration-review-2.md`
- Parent: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- Siblings from the split: 0185, 0186, 0187, 0188
- ADRs: ADR-0048 (hook logic lives in the CLI), ADR-0053 (thin CLI over a
  hexagonal ports-and-adapters core)
