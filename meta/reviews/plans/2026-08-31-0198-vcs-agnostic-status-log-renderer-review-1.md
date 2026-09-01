---
type: "plan-review"
id: "2026-08-31-0198-vcs-agnostic-status-log-renderer-review-1"
title: "Plan Review: VCS-agnostic status/log renderer"
date: "2026-08-31T15:41:46+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
target: "plan:2026-08-31-0198-vcs-agnostic-status-log-renderer"
reviewer: "Toby Clemson"
verdict: "APPROVE"
lenses: ["architecture", "correctness", "test-coverage", "code-quality", "compatibility", "standards", "security", "safety"]
review_number: 1
review_pass: 3
tags: []
last_updated: "2026-08-31T17:52:02+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: VCS-agnostic status/log renderer

**Verdict:** REVISE

The plan is architecturally sound and unusually well-grounded — a clean
functional-core/imperative-shell split, a spike that settled every deferred
`gix`/`jj-lib` API, and honest phase sequencing. It needs revision before
implementation for two reasons that recur across lenses: the "never-fail"
contract is implemented as an `Err`-only match that does not survive the new
in-process fault classes (panic, hang, OOM), and the test matrix and a few
enforcement-gate references do not actually cover what the plan claims. A
factual error in the golden-regeneration guidance and a design decision resting
on a false orphan-rule premise round out the required changes.

### Cross-Cutting Themes

- **The never-fail contract is not preserved for in-process faults** (flagged by:
  safety, security, architecture) — the `match { Ok => render, Err => fallback }`
  boundary catches only `Err`. A library panic (including the `reverse_hex()[..12]`
  slice), an unbounded hang (the 10-second subprocess cap is deleted with nothing
  replacing it), or an OOM (`max_new_file_size: u64::MAX` hashes an arbitrarily
  large file) all escape the fallback and crash or stall a `/commit`-invoked
  subcommand contracted never to fail. This is the headline finding.
- **The test matrix does not cover the ADR it verifies** (flagged by:
  test-coverage, correctness) — three of the five change-types (`deleted`, plus
  rename's `deleted`+`added`), the five-commit cap, git unborn HEAD, and non-empty
  jj bookmarks each have no fixture. Goldens are regenerated from the binary and
  guarded only by manual review, so an adapter bug enshrines itself as the oracle.
- **Enforcement-gate references are imprecise** (flagged by: standards,
  compatibility) — the feature-graph regression is attributed to `mise run check`
  (which does not run it), the new `_EXEMPT` entry lands without its paired
  anti-vacuity test, and the `dirwalk` gix feature is left unresolved while the
  "adds no crates" claim is only verified for `blob-diff`.
- **Crate-boundary error handling** (flagged by: architecture, code-quality) — the
  new `ReportError` is justified by a false orphan-rule claim and collapses the
  structured error to a `Display` string, severing the `source()` chain that AC6's
  diagnostic exists to surface.

### Tradeoff Analysis

- **In-process latency vs fault isolation**: the move buys the ~23.8 ms → ~4 ms
  win and removes the `PATH` dependency, but forfeits the subprocess boundary's
  free crash containment and 10-second ceiling. Recommendation: keep the move,
  but re-add a panic boundary (`catch_unwind`) and decide explicitly whether a
  wall-clock bound is worth a worker thread for these interactive callers, rather
  than leaving "no time/memory bound" as a bare note.
- **Generated goldens vs red-first TDD**: regenerating goldens from the binary is
  pragmatic for a 24-file matrix but inverts red-green for the adapter layer.
  Recommendation: keep generation for the bulk, but add a handful of hand-authored
  assertions (change-type per file, the five-commit cap, unborn HEAD) so the
  highest-risk mappings have an oracle not derived from their own output.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Safety / Security / Architecture**: Never-fail boundary catches `Err` but
  not panics, hangs, or OOM — the in-process fault classes escape the fallback
  **Location**: Phase 1 §4; Performance Considerations; Phase 3 §1
  The contract is `match { Ok => render, Err => fallback }`, which only intercepts
  `Err`. A `gix`/`jj-lib` panic on a corrupt repo, the `reverse_hex()[..12]` slice,
  the deleted 10s cap (no replacement), and `max_new_file_size: u64::MAX` each
  break "status never fails" — a new failure class these two commands never had.

- 🟡 **Correctness**: Ahead/behind LOG goldens will not render identically to
  `clean-git` — a factual error in the regeneration guidance
  **Location**: Phase 1 §5
  The "render identically to `clean-git`" claim holds for *status* only. The
  fixtures carry different histories (`git-ahead-log` = 3 commits, `git-behind-log`
  = 1, `clean-git-log` = 1), so the flat first-parent log renders three distinct
  outputs. Following the guidance would commit a wrong test oracle.

- 🟡 **Architecture / Code Quality**: `ReportError` rests on a false orphan-rule
  premise and collapses the error to a string, dropping the diagnostic `source()`
  **Location**: Phase 1 §1a, §2
  Every sibling port returns `Result<_, kernel::Error>`, and `impl From<Error> for
  kernel::Error` already compiles at `library.rs:429` — refuting "the orphan rule
  forbids it". `ReportError::new(error.to_string())` also severs the `gix`/`jj-lib`
  cause before AC6's `warn!(%error)` runs.

- 🟡 **Standards**: The feature-graph regression is attributed to `mise run check`,
  which does not run it
  **Location**: Phase 1 Success Criteria
  `test_vcs_library_graph.py` runs under `test:integration:deny` (and the bare
  `default`), not `check`. `check` runs `deny:check` (cargo-deny by crate name),
  which by design does not assert the gix feature set. A developer verifying via
  `check` gets a false green on the Phase 1 §5 Cargo.toml edit.

- 🟡 **Compatibility / Architecture**: Broad pre-1.0 `gix`/`jj-lib` coupling with
  the work item's Open Question left unresolved
  **Location**: "API and behaviour verified by spike" section; Phase 1 §3
  The plan verifies ~a-dozen pre-1.0 API points exist *today* but never costs the
  per-bump maintenance the work item requires be resolved "before the jj adapter is
  built" — which Phase 1 does. A jj-lib 0.43→0.44 or gix 0.85.x bump can break
  several call sites at once.

- 🟡 **Code Quality / Standards**: Inline comments in the `git_log`/`jj_log`
  snippets breach the comments-as-last-resort standard
  **Location**: Phase 1 §2, §3
  `// Each ? maps its gix error…`, `// .map_err(→ Error::Git)?`, and the jj
  equivalent describe what the code does and defend a convention; the review
  process explicitly says to strip comments from plans. Carried through, they fail
  the project's own bar.

- 🟡 **Test Coverage**: No fixture exercises the `deleted` or rename change-types
  **Location**: Phase 1 §5–§6; Phase 2 §3
  The matrix only produces `added`, `modified`, `untracked`, `conflicted`. The
  `Summary::Removed → Deleted` and rename → `deleted`+`added` mappings are pinned
  by no test, so mis-mapping a deletion passes every named test.

- 🟡 **Test Coverage**: The five-commit log cap truncation is never tested
  **Location**: Phase 1 §2–§3; Phase 2 §3
  No fixture exceeds five commits (`git-ahead` has three). Phase 2 §3's five-line
  assertion over a five-commit repo cannot distinguish a working `take(5)` from a
  broken one. Changing the bound would go uncaught.

- 🟡 **Test Coverage**: AC6 real-binary logging wiring and the
  malformed-`ACCELERATOR_LOG` branch have no automated test
  **Location**: Phase 1 §4; Phase 3 §3
  The fault-injection test drives `status::run`/`log::run` with a fake reporter,
  bypassing `main` — so neither the `kernel::logging::init()` wiring (the exact
  AC6 bug being fixed) nor the never-fail malformed-env branch is covered. Both are
  left to manual verification.

- 🟡 **Test Coverage**: Git unborn HEAD (empty repo) is absent from the fixture
  matrix
  **Location**: Phase 1 §2, §5
  ADR-0066 specifies `No commits` for a repo with no commits, and `git_log` has a
  dedicated `is_unborn_head` branch — but no state is a fresh `git init`. If
  detection breaks, an unborn-HEAD repo falls to `(log unavailable)` undetected.

- 🟡 **Standards**: The new `_EXEMPT` entry lands without its paired anti-vacuity
  test
  **Location**: Phase 1 §3
  Every `_EXEMPT` entry in `vcs_settings.py` has a 1:1 regression test in
  `test_vcs_settings.py`. The plan adds `status_log.rs` to the list but not the
  test, and may leave `dirty_paths.rs`'s own exemption vacuous once the
  `UserSettings` construction moves.

- 🟡 **Correctness**: Git status collapse tie-break for divergent staged vs
  worktree types is unspecified
  **Location**: Phase 1 §2
  For a path with a staged item and a worktree item of *different* non-conflict
  types (staged-add + worktree-modify), neither the winning change type nor whether
  `N` counts 1 or 2 is defined — arbitrary or iteration-order dependent.

- 🟡 **Test Coverage**: Non-empty jj bookmark reading (single and multi) is
  untested
  **Location**: Phase 1 §3
  Every jj fixture leaves the working-copy commit bookmark-less, always rendering
  `(none)`. The bookmark collection, byte-sort, and comma-join the ADR pins are
  never exercised.

#### Minor

- 🔵 **Code Quality**: Repeated inline `map_err` — the "one shared closure won't
  compile" justification overlooks a generic helper
  **Location**: Phase 1 §2, §3
  A closure can't be shared across differing error types, but a generic
  `fn git_err<E: Error…>(root) -> impl Fn(E) -> Error` monomorphises per site and
  compiles, reducing each mapping to `.map_err(git_err(root))?` and removing the
  defensive comment.

- 🔵 **Code Quality**: The injection seam virtualises only the report read while
  hard-coding discovery
  **Location**: Phase 1 §4
  `run(start, reporter)` builds its own `InProcessProbe` for discovery, then reads
  through the injected `&dyn VcsReporter`; production passes the same zero-sized
  type twice. Lifting discovery into the caller (`run(dir, kind, &dyn …)`) makes it
  a pure fallback wrapper.

- 🔵 **Code Quality / Test Coverage**: Change-kind classification is entangled with
  I/O yet listed as a unit test
  **Location**: Phase 1 §2–§3 vs Testing Strategy
  The `Summary → ChangeType` and presence → `ChangeType` mappings sit inline inside
  `git_status`/`jj_status`, reachable only through real fixtures. Extracting a pure
  `classify(summary) -> ChangeType` makes the branch-heavy table genuinely
  unit-testable red-first.

- 🔵 **Correctness**: Untracked and staged-add both surface as `Summary::Added` —
  `summary()` alone cannot separate them
  **Location**: Phase 1 §2
  The rule list maps untracked dirwalk `Added → Untracked` and staged `Added →
  Added` from the identical summary. The code must key on the item *source*
  (dirwalk vs tree-index variant); taken literally, untracked would render as
  `added` and break the AC3 divergence.

- 🔵 **Code Quality / Correctness**: jj `short_id` uses a raw `[..12]` slice with an
  unstated invariant, asymmetric with git's width-checked accessor
  **Location**: Phase 1 §3
  Panic-safe under jj's fixed 16-byte change-id invariant (correctness confirmed),
  but a bare magic `12` and a raw byte slice against git's purpose-built
  `to_hex_with_len(12)`. Name the width once and take it length-safely.

- 🔵 **Compatibility**: The `dirwalk` gix feature is left unpinned; "adds no crates"
  is only verified for `blob-diff`
  **Location**: Phase 1 §5
  `status` and `blob-diff` add nothing (already present), but a net-new `dirwalk`
  could pull a subcrate, perturbing the build-script/proc-macro snapshots — leaving
  the "feature graph unchanged" criterion unverified.

- 🔵 **Compatibility**: Strong zero-spawn runs only on Linux CI; the macOS local gap
  is not stated
  **Location**: Phase 3 §4
  Absolute-path shadowing cannot run under macOS SIP, so macOS developers get only
  the path-only lane, which by construction cannot see an absolute-path spawn. An
  inherited 0188 constraint worth stating so a green local run isn't misread.

- 🔵 **Security**: In-process `gix` reads lose the subprocess environment scrub
  **Location**: Phase 3 §1; Phase 1 §2; Migration Notes
  The deleted `subprocess.rs` scrubbed `GIT_*`/`GIT_CONFIG*` and forced
  `GIT_CONFIG_NOSYSTEM=1`. `gix::open(root)` honours ambient `GIT_*` and
  system/global config. Low risk (gix runs no hooks/aliases), but a deliberate
  isolation reduction the plan does not call out.

- 🔵 **Security**: The zero-spawn assertion never exercises an adversarial repo
  config
  **Location**: Phase 3 §4; Phase 1 §5
  The fixture matrix is all benign states — none sets `core.fsmonitor`, an external
  `filter.*.process`, or a `diff.*.textconv`. The proof shows the happy path spawns
  nothing, not that a hostile config cannot induce a spawn from the ranged gix pin.

- 🔵 **Safety**: jj `status` acquires the working-copy lock; concurrent jj processes
  serialise and can block unboundedly
  **Location**: Phase 1 §3
  `working_copy_diff` calls `start_working_copy_mutation()`, taking jj's lock before
  snapshotting. A concurrent long-running `jj` holding the lock stalls `vcs status`
  with no bound (compounding the deleted cap). No corruption risk — RAII releases
  it, `panic=unwind` preserved.

- 🔵 **Test Coverage**: Adapter behaviour is pinned only by binary-generated goldens
  plus manual review
  **Location**: Implementation Approach; Phase 1 §5
  Generation captures an adapter bug into the golden as correct; the plan's "a
  failing golden precedes each behaviour" is unachievable for generated goldens.
  Only the renderer units and the conflict assertion are genuinely red-first.

- 🔵 **Test Coverage**: The parity harness's "unmasked control" is under-specified
  **Location**: Phase 2 §2
  AC3 requires a control proving masks cover only volatile values, but the
  mechanism is unstated and the per-side regexes are permissive. Specify it
  concretely — e.g. stripping the id leaves a suffix byte-identical to the masked
  line's suffix.

- 🔵 **Test Coverage**: The sha256-repo fallback claim is not pinned by a test
  **Location**: Resolved research questions; Phase 1 §5
  The plan relies on a spike finding that a sha256 repo folds to `(… unavailable)`
  but declines a golden — though the `S256` shape already exists in the checkout
  matrix. A future gix that partially supports sha256 would change behaviour
  uncaught.

- 🔵 **Test Coverage**: Zero-spawn under shadowing never exercises the real binary's
  `main` path
  **Location**: Phase 3 §4
  The lane drives `vcs-adapters-fixture`, but real `main` now additionally calls
  `kernel::logging::init()` (new here) reading `ACCELERATOR_LOG` — precisely the
  path never run inside the shadow window. A narrow blind spot in the strong-form
  guarantee.

- 🔵 **Standards**: The Python build-system gate is not named for phases that edit
  `tasks/`
  **Location**: Phase 1; Phase 3 §4
  Phase 1 edits `vcs_settings.py` and Phase 3 §4 edits `integration.py`, both gated
  by `build-system:check` and the tasks tests, which neither phase names — weakening
  "each phase ends green".

- 🔵 **Standards**: The public-api criterion names the check task, not the update
  action
  **Location**: Phase 1 §1a; Success Criteria
  `public-api:check` verifies against the snapshot and fails until
  `public-api:update` regenerates it (diff reviewed as intended). The phrasing
  conflates the gate with the action.

- 🔵 **Test Coverage**: The git conflict fixture builder must tolerate `merge`'s
  non-zero exit
  **Location**: Phase 1 §6
  An unresolved `git merge` exits non-zero; the shared `Hermetic::git` helper
  returns `Err` on non-zero exit, so a naive builder fails to construct the fixture
  and the AC4 assertion silently never runs.

#### Suggestions

- 🔵 **Safety**: "Mutates nothing" understates that `status` writes objects to the
  backend on a read
  **Location**: What We're NOT Doing (line 148); Migration Notes
  The write is safe (content-addressed, GC-reclaimable), but the shorthand can
  mislead a maintainer into assuming zero disk-write side effects. Align with the
  `dirty_paths.rs` module-doc framing (writes objects, persists no operation).

- 🔵 **Architecture**: The shared state-builder extraction in Phase 2 couples the
  deletion phase to the parity phase
  **Location**: Phase 2 §1
  The `conflict-*` states are born in Phase 1 but `build_states` is extracted in
  Phase 2, which Phase 3's zero-spawn then consumes — so Phase 3 depends on Phase 2
  only by placement. Move the extraction into Phase 1.

- 🔵 **Security**: The diagnostic `warn!` path can leak absolute repository paths
  into logs
  **Location**: Phase 1a; Phase 1 §4
  `Error::Git { path, source }` carries the absolute path; `warn!(%error)` surfaces
  it once `ACCELERATOR_LOG` is wired. Low-severity local disclosure — note it as a
  conscious choice or redact to repo-relative.

- 🔵 **Compatibility**: `ChangeType` is an exhaustive public enum with no
  `#[non_exhaustive]`
  **Location**: Phase 1 §1, §1a
  A future ADR-driven variant registers as a breaking public-api diff. Low impact
  (internal crate, compile-time caught); mark `#[non_exhaustive]` if the set is
  expected to grow, or document the closed set as intentional.

- 🔵 **Code Quality**: `branch: Vec<String>` is a plural collection with a singular
  name
  **Location**: Phase 1 §1
  A `Vec` named in the singular obscures the multi-bookmark case. Consider
  `bookmarks`/`refs` or a `BranchLabel` value type owning the join/`(none)` logic.

- 🔵 **Correctness**: The extracted `working_copy_diff` signature cannot represent
  the no-working-copy-commit state
  **Location**: Phase 1 §3
  Today's `jj_dirty_paths` returns `Ok(Vec::new())` when there is no wc commit; the
  proposed `-> Result<(Vec<DiffEntry>, MergedTree), Error>` has no tree to return.
  Return an `Option` for the tree so `jj_status`/`jj_log` short-circuit cleanly.

### Strengths

- ✅ The pure renderer (`vcs::status::render` / `log::render`) is side-effect-free
  over plain value types with guard clauses — highly readable and unit-testable in
  isolation, independent of any generated golden.
- ✅ Domain-rich naming throughout (`ChangeType`, `FileChange`, `StatusReport`,
  `VcsReporter`, `working_copy_diff`) mirrors the ADR-0066 vocabulary rather than
  `gix`/`jj-lib` incidental detail.
- ✅ The hexagonal layering is preserved and cycle-free (vcs ← vcs-adapters ←
  vcs-cli), consistent with ADR-0053 and the existing `RepoRoot`/`VcsProbe` ports;
  the `VcsReporter` seam makes AC6 fault injection deterministic without file
  permissions.
- ✅ The atomic both-backends flip correctly avoids a mixed-format intermediate
  state, and the plan is honest that the phases are deliberately not independently
  mergeable.
- ✅ Conflict (AC4) is TDD'd where it is built: the spike-validated jj union from
  `tree.conflicts()` (git conflict as a first-class status item) is covered by a
  focused, falsifiable `1 changed, 1 conflicted` assertion, not deferred a phase.
- ✅ The core logic is provably sound where scrutinised: the git/jj log-depth
  asymmetry is internally consistent and matches the ADR; empty states, `(none)`,
  `(no description)`, byte-sort, and `VcsKind::None`-folds-to-fallback are all
  correct; masked git and jj log lines can never cross-match (disjoint alphabets).
- ✅ Output-format compatibility is verified, not assumed: a grep confirms the sole
  consumer is `skills/vcs/commit/SKILL.md` injecting free-form prose — no hook, no
  parser, no cross-backend comparison — so the format shift breaks nothing.
- ✅ The move is a net security improvement: `gix` executes none of the
  hooks/pager/aliases/credential-helpers a real `git` binary would honour from a
  malicious repo config, and gix-credentials is double-guarded (deny.toml + the
  feature-graph test).

### Recommended Changes

1. **Make the boundary panic- and resource-safe, not just `Err`-safe**
   (addresses: never-fail boundary theme). Wrap the reporter call at
   `status::run`/`log::run` in `std::panic::catch_unwind(AssertUnwindSafe(…))`,
   folding a caught panic to `(status|log unavailable)` + `warn!`; replace
   `reverse_hex()[..12]` with a length-checked `.get(..12)`; cap
   `max_new_file_size` to a sane ceiling instead of `u64::MAX`; and decide
   explicitly whether an interactive wall-clock bound (worker thread + bounded
   join) replaces the deleted 10s cap or is a documented accepted risk. Record the
   fault-isolation regression in Migration Notes.

2. **Correct the ahead/behind golden guidance** (addresses: ahead/behind LOG
   goldens). Scope "renders identically to `clean-git`" to the *status* goldens
   only; state that the ahead/behind *log* goldens legitimately reflect their own
   commit histories (it is the remote relationship, not the commits, that must not
   leak).

3. **Reuse `kernel::Error` for the port, or preserve the cause** (addresses:
   ReportError). Drop `ReportError` in favour of `Result<_, kernel::Error>` (the
   sibling-port convention; the `From` impl already compiles) so the adapter uses a
   plain `?`; correct the orphan-rule reasoning. If a domain error is genuinely
   wanted, format the full `source()` chain into it so AC6 keeps the real cause.

4. **Fix the enforcement-gate references** (addresses: feature-graph task,
   `_EXEMPT` test, public-api action, build-system gate). Name
   `test:integration:deny` for the feature graph and `deny:check` for the licence
   closure; add `test_the_status_log_module_is_individually_exempt` (red-first) and
   confirm/prune the `dirty_paths.rs` exemption; state `public-api:update` then
   `public-api:check`; add `build-system:check` + the tasks tests to Phases 1 and 3.

5. **Close the test-matrix gaps** (addresses: deleted/rename, five-commit cap,
   unborn HEAD, jj bookmarks, generated-goldens). Add `deleted`/rename states, a
   six-plus-commit state (assert exactly five lines, sixth omitted), an
   `unborn-git` state (`No commits`), and one- and two-bookmark jj states
   (byte-sorted comma-join). Extract a pure `Summary → ChangeType` classifier and
   unit-test it red-first, including the staged-vs-worktree tie-break and the
   untracked-vs-staged-`Added` distinction.

6. **Specify the two under-defined logic points** (addresses: collapse tie-break,
   untracked vs staged). State the git collapse precedence explicitly (e.g.
   `conflicted > added > deleted > modified`, dedupe by path) and that untracked is
   detected from the dirwalk item variant, not `summary()`.

7. **Strip the inline comments from the snippets** (addresses: comments). Remove
   the `map_err` annotations; move the one genuinely non-obvious note (differing
   error types) into prose beneath the block — or eliminate it via the generic
   `git_err`/`jj_err` helper.

8. **Resolve the deferred decisions** (addresses: Open Question, `dirwalk`,
   Phase 2 coupling). Add a note resolving the jj-lib maintenance-cost Open
   Question (accept and enumerate the coupled API points, or invoke the git-only
   re-scope); pin the exact gix feature set and confirm it against
   `test_vcs_library_graph.py`; move the shared state-builder extraction into
   Phase 1.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: The plan preserves a clean hexagonal layering (vcs ← vcs-adapters ←
vcs-cli): a pure renderer and neutral model in the domain core, I/O in adapters,
injection at the CLI seam, consistent with ADR-0053 and a textbook
functional-core/imperative-shell split. The atomic both-backends flip, the
`working_copy_diff` extraction, and the conflict-read design are sound structural
decisions, honestly sequenced. The main weaknesses are a new crate-boundary error
type (`ReportError`) diverging from the crate's uniform `kernel::Error` port
convention on a mistaken orphan-rule premise, and an unacknowledged loss of the
subprocess fault-isolation boundary (time cap and crash containment) for two
commands contracted to never fail.

**Strengths**:
- Dependency direction is preserved and cycle-free; a clean functional-core /
  imperative-shell split consistent with the existing `RepoRoot`/`VcsProbe`/
  `CheckoutProbe` ports.
- The atomic both-backends flip avoids a mixed-format intermediate state; the plan
  is honest that the phases are not independently mergeable.
- Extracting `working_copy_diff` and re-pointing `jj_dirty_paths` removes ~70 lines
  of duplicated snapshot logic without changing semantics.
- Strong domain language matching ADR-0066 vocabulary.
- The conflict-read design (git first-class item vs jj `tree.conflicts()` union) is
  spike-validated and TDD'd where built.

**Findings**:
- **Major / high** — Phase 1 §1a, §2: `ReportError` diverges from the
  `kernel::Error` port convention on a false orphan-rule premise. Every fallible
  port returns `Result<_, kernel::Error>`, and `impl From<Error> for kernel::Error`
  at `library.rs:429` is exactly the shape claimed forbidden. A second port-error
  convention enters the domain crate and grows the public-api baseline for no gain.
- **Minor / medium** — Performance Considerations / Phase 1 §4: the in-process move
  drops the subprocess fault-isolation boundary (10s cap + crash containment) for
  two never-fail commands. The `Result`-based contract catches `Err` but neither a
  hang nor a library panic; a library panic is a genuinely new failure class.
- **Minor / medium** — Phase 1 §3: the widened pre-1.0 jj-lib API surface
  (`MergedTree::conflicts()`, `Store::get_commit`, `reverse_hex()`, first-parent
  peeling) deepens coupling to a per-release-churning library. Consider
  concentrating the touch points behind `working_copy_diff`.
- **Suggestion / medium** — Phase 2 §1: the shared state-builder extraction placed
  in Phase 2 needlessly couples the Phase 3 deletion/zero-spawn work to the Phase 2
  parity phase. Move it to Phase 1 where the states are created.

### Correctness

**Summary**: The renderer logic is largely sound: the git/jj log-depth asymmetry
(git includes HEAD, jj excludes `@`) is internally consistent and matches
ADR-0066's recorded-history semantics; the jj conflict-union is self-consistent
with the summary counter; empty states, `(none)`, `(no description)`, `No changes`/
`No commits`, byte-sort, and `VcsKind::None`-folds-to-fallback are all handled
correctly; and the 12-char id renderings sit inside the committed masks. The most
material issue is a factual error in the golden-regeneration guidance, plus two
under-specified collapse/dedup mechanisms in git status classification.

**Strengths**:
- Log-depth reasoning is correct: git's `first_parent_only().take(5)` yields HEAD +
  four ancestors, the jj walk starts at `parent_ids().first()` of `@` excluding `@`
  and root — both surface the same five recorded commits for identical history.
- The jj conflict-union is self-consistent: a conflicted-only file is pushed into
  `report.changes` as `Conflicted`, so `render` produces `1 changed, 1 conflicted`
  with one file line.
- Empty-state handling is complete: conflicts live in `report.changes`, so
  `No changes` can never hide a conflict.
- `reverse_hex()[..12]` is panic-safe under jj's fixed 16-byte change-id invariant.
- The 12-char renderings stay inside the committed masks, and the git/jj alphabets
  are disjoint so masked lines cannot cross-match.
- Routing `VcsKind::None` to git_status/git_log is provably safe (gix::open fails,
  folds to fallback).

**Findings**:
- **Major / high** — Phase 1 §5: ahead/behind LOG goldens will not render
  identically to `clean-git`. The fixtures carry different histories (`git-ahead` =
  3 commits, `git-behind` = 1, `clean-git` = 1), so the flat log renders three
  distinct outputs; the "identical" claim holds for status only.
- **Major / medium** — Phase 1 §2: the git status collapse tie-break for a path
  with divergent staged vs worktree types (staged-add + worktree-modify) is
  unspecified, so the collapsed type and the `N` count are arbitrary.
- **Minor / medium** — Phase 1 §2: untracked and staged-add both surface as
  `Summary::Added`; `summary()` alone cannot separate them. The code must key on
  the item source (dirwalk vs tree-index variant).
- **Suggestion / medium** — Phase 1 §3: the extracted `working_copy_diff` signature
  cannot represent the no-working-copy-commit state (no `MergedTree` to return).
  Return an `Option` for the tree.
- **Suggestion / low** — Phase 1 §3: jj `short_id` uses a raw byte slice where git
  uses a width-checked accessor; safe today but fragile to a future reverse-hex
  width change.

### Test Coverage

**Summary**: The plan has a strong pure-renderer unit layer (red-first,
hand-authored, mutation-resistant) and a well-TDD'd conflict path, plus a clean
port seam that makes AC6 fault injection deterministic. However, coverage of the
adapter layer leans heavily on binary-generated goldens guarded only by manual
review, and several ADR-specified behaviours have no fixture: the `deleted`/rename
change-types, the five-commit log cap, git unborn HEAD, and non-empty jj bookmarks.
The AC6 production fix is verified only manually.

**Strengths**:
- The pure renderer is unit-tested red-first with hand-authored expectations
  covering every empty state, the conflict grammar, all five labels, and byte-sort.
- Conflict (AC4) is TDD'd where built, with a specific `1 changed, 1 conflicted`
  shape verifying the jj union.
- The `VcsReporter` seam makes AC6 fault injection deterministic and
  file-permission-free.
- Zero-spawn reuses the existing two-assertion pattern; output equivalence between
  the fixture and real binary holds by shared goldens.
- Masks stay a closed set cross-validated in two languages; adding a mask to rescue
  a comparison is explicitly forbidden.

**Findings**:
- **Major / high** — Phase 1 §5–§6; Phase 2 §3: no fixture exercises `deleted` or
  rename; the `Removed → Deleted` and rename → `deleted`+`added` mappings are pinned
  by nothing.
- **Major / high** — Phase 1 §2–§3; Phase 2 §3: the five-commit cap is never tested;
  no fixture exceeds five commits, so a five-commit assertion cannot distinguish a
  working `take(5)` from a broken one.
- **Major / high** — Phase 1 §4; Phase 3 §3: the AC6 real-binary logging wiring and
  malformed-`ACCELERATOR_LOG` branch are untested; the fault-injection test bypasses
  `main`.
- **Major / high** — Phase 1 §2, §5: git unborn HEAD is absent from the matrix; the
  `is_unborn_head` branch (`No commits`) is untested.
- **Major / medium** — Phase 1 §3: non-empty jj bookmark reading (single and multi)
  is untested; every jj fixture renders `(none)`.
- **Minor / medium** — Implementation Approach / Phase 1 §5: adapter behaviour is
  pinned only by binary-generated goldens plus manual review, inverting red-green.
- **Minor / medium** — Phase 2 §2: the parity harness's "unmasked control" is
  under-specified; the mechanism is unstated and the per-side regexes are permissive.
- **Minor / medium** — Resolved research questions / Phase 1 §5: the sha256-repo
  fallback claim is not pinned by a test though the `S256` shape already exists.
- **Minor / medium** — Phase 3 §4: zero-spawn under shadowing drives the fixture, not
  the real binary's `main` (which now calls `logging::init()`).
- **Minor / low** — Phase 1 §6: the git conflict fixture builder must tolerate
  `merge`'s non-zero exit or `Hermetic::git` errors and the AC4 test never runs.

### Code Quality

**Summary**: The core design is strong — a pure renderer over domain-rich value
types in `vcs`, adapters in `vcs-adapters`, and a `VcsReporter` port cleanly
inverting the dependency so the never-fail fallback is testable with a fake. The
weaknesses are in the adapter layer: explanatory/placeholder comments violating the
project's very-low comment tolerance, repeated inline `map_err` boilerplate
defended on incomplete grounds, a structured-error collapse that undercuts the
fallback path's own diagnosability rationale, and change-kind classification
entangled with I/O despite being promised as unit tests.

**Strengths**:
- The pure renderer is a pair of side-effect-free functions over plain value types
  with guard clauses — highly readable and trivially unit-testable.
- Domain-rich naming throughout satisfies the DDD convention.
- The `VcsReporter` port is a genuine dependency-inversion seam, cleaner than the
  existing permission-flipping tests.
- Placing model/renderer/port in `vcs` (free of gix/jj-lib types) with adapters in
  `vcs-adapters` keeps the hexagonal boundary intact.

**Findings**:
- **Major / high** — Phase 1 §2, §3: inline comments in the adapter snippets
  (`// Each ? maps its gix error…`, `// .map_err(→ Error::Git)?`) violate the
  very-low comment tolerance and the review rule to strip comments from plans.
- **Major / medium** — Phase 1 §1a, §2: collapsing the internal `Error` to
  `ReportError::new(error.to_string())` drops the `source()` chain, so the AC6
  `warn!(%error)` names only the top line — the exact detail AC6 exists to surface.
- **Minor / high** — Phase 1 §2, §3: the repeated inline `map_err`'s "one shared
  closure won't compile" justification overlooks a generic helper that
  monomorphises per site.
- **Minor / medium** — Phase 1 §4: the injection seam virtualises only the report
  read while hard-coding discovery; lifting discovery into the caller makes `run` a
  pure fallback wrapper.
- **Minor / medium** — Phase 1 §2–§3 vs Testing Strategy: change-kind classification
  is entangled with I/O yet listed as a unit test; extract a pure `classify`.
- **Minor / medium** — Phase 1 §3: jj `short_id` uses a raw `[..12]` slice with an
  implicit invariant and a bare magic `12`, inconsistent with git's
  `to_hex_with_len(12)`.
- **Suggestion / low** — Phase 1 §1: `branch: Vec<String>` is a plural collection
  with a singular name.

### Compatibility

**Summary**: The compatibility posture is mostly sound: the output-format change
reaches exactly one internal, prose-consuming caller (grep-verified), the new
public surface is purely additive with an explicit baseline-regeneration step, and
the feared gix-credentials re-admission is double-guarded. The two live risks are
dependency-coupling to pre-1.0 gix 0.85 / jj-lib 0.43 internal APIs — verified to
exist today but never costed for per-bump maintenance the work item's Open Question
demands — and an unpinned "maybe dirwalk" gix feature whose "adds no crates" claim
is only actually verified for `blob-diff`.

**Strengths**:
- The output-format change is safe: the only production consumer is
  `skills/vcs/commit/SKILL.md` injecting free-form LLM orientation — no hook, no
  parser, no cross-backend comparison.
- The new public API is purely additive with `public-api:check` baseline
  regeneration an explicit Phase 1 criterion.
- The gix feature addition is version-safe (feature unification cannot split the
  single 0.85.x graph) and gix-credentials is guarded twice over with
  `default-features = false` preserved.
- The pre-1.0 API surface was empirically validated against the exact pins by a
  spike over real conflict repos.

**Findings**:
- **Major / high** — spike section (cross-ref work item Open Question): couples the
  data path to a broad set of pre-1.0, no-stable-API surfaces, verifying only that
  they exist today, and does not resolve the Open Question the work item requires
  answered before the jj adapter is built.
- **Minor / medium** — Phase 1 §5: the final gix feature set is unpinned ("add
  dirwalk if…") while asserting "adds no crates"; that claim is only established for
  `blob-diff`.
- **Minor / medium** — Phase 3 §4: the strong-form zero-spawn runs only on Linux CI
  (macOS SIP); macOS developers get only the weaker path-only lane, which the plan
  does not state.
- **Suggestion / low** — Phase 1 §1, §1a: `ChangeType` lands as an exhaustive public
  enum with no `#[non_exhaustive]`; a future variant is a breaking public-api diff.

### Standards

**Summary**: The plan shows strong awareness of the repo's enforcement surface — it
correctly treats the new units as modules inside existing crates, follows the `vcs`
domain-module naming, reuses the golden naming, mirrors the crate-wide zero-spawn
deny shape, and picks id widths that fit the committed masks. The main gaps are
gate-naming errors in the Success Criteria (the feature-graph regression attributed
to `mise run check`, which does not run it) and an artefact-sync omission (the
`_EXEMPT` addition lands without its paired anti-vacuity test). Inline comments in
the snippets also breach the comments-as-last-resort standard.

**Strengths**:
- Correctly scopes the new files as modules within existing crates, so no
  sub-binary/library-crate registration checklist is owed.
- New module and golden names match existing conventions.
- The pup.ron widening mirrors the existing crate-wide zero-spawn deny precedent
  while retaining the module-scoped import allow-list.
- Chosen id widths fit the committed masks with no new mask.
- Correctly recognises `vcs` as a pinned public-api crate whose surface must be
  re-pinned, and names the right guard tasks.

**Findings**:
- **Major / high** — Phase 1 Success Criteria: the feature-graph regression is
  attributed to `mise run check`; it runs under `test:integration:deny`, not
  `check` (which runs `deny:check`, a name-based supply-chain gate).
- **Major / high** — Phase 1 §3: the new `_EXEMPT` entry lands without its paired
  anti-vacuity test in `test_vcs_settings.py`; the `dirty_paths.rs` exemption may
  become vacuous once `UserSettings` moves.
- **Minor / medium** — Phase 1 / Phase 3 §4: `build-system:check` and the tasks tests
  are not named for phases editing `tasks/`.
- **Minor / high** — Phase 1 §2, §3: inline comments in the snippets breach the
  comments-as-last-resort standard and the strip-comments rule.
- **Minor / medium** — Phase 1 §1a / Success Criteria: the public-api criterion names
  `public-api:check` but the snapshot must first be regenerated with
  `public-api:update`.
- **Suggestion / medium** — Phase 1 §5: the gix feature edit leaves the
  `test_vcs_library_graph.py` sync obligation conditional ("add dirwalk if…").

### Security

**Summary**: The plan replaces a scrubbed, time-capped subprocess with in-process
gix/jj-lib reads that parse repository-controlled data in the caller's own address
space, with an explicit "no time/memory bound". The dominant threat is
attacker-driven resource exhaustion from a hostile or corrupt repository: the
never-fail `Result -> String` fallback catches only `Err`, so a hang, OOM, or panic
bypasses it, and the 10-second cap is removed. Against this, the move is a net
code-execution improvement (gix runs no hooks/pagers/aliases/credential helpers),
and the gix-credentials ban plus the strong zero-spawn assertion are solid
defence-in-depth — the residual gaps are the unbounded work, a reduced environment
scrub, and a benign-only fixture set.

**Strengths**:
- The user-facing fallback is a fixed literal carrying no repository data; the
  failed-adapter identity is derived from `VcsKind` and confined to `ACCELERATOR_LOG`.
- Switching to gix/jj-lib reduces the code-execution attack surface: gix executes no
  config-driven program vectors a real git binary would honour from a malicious repo.
- Defence-in-depth against network/credential/spawn is retained and strengthened
  (default-features=false, the deny.toml bans, the feature-absence assertions, the
  crate-wide std::process deny, the strong zero-spawn job).

**Findings**:
- **Major / medium** — Performance Considerations / Phase 1 §3 / Current State: the
  never-fail fallback does not cover hostile-repo hang/OOM/panic; the 10s cap is
  removed and `max_new_file_size: u64::MAX` reads/hashes an arbitrarily large file
  (worse than native jj's 1 MiB default). Mitigations: cap the file size, wrap the
  read in `catch_unwind`, decide on a wall-clock bound.
- **Minor / medium** — Phase 3 §1 / Phase 1 §2 / Migration: in-process gix reads lose
  the subprocess environment scrub; `gix::open` honours ambient `GIT_*` and
  system/global config. Low risk (no code execution) but an undocumented isolation
  reduction.
- **Minor / medium** — Phase 3 §4 / Phase 1 §5: the zero-spawn assertion never
  exercises an adversarial repo config (`core.fsmonitor`, external filters/textconv),
  so it proves the happy path spawns nothing, not that a hostile config cannot induce
  a spawn from the ranged gix pin.
- **Suggestion / medium** — Phase 1a / Phase 1 §4: the diagnostic `warn!` path can
  leak absolute repository paths into logs once `ACCELERATOR_LOG` is wired.

### Safety

**Summary**: The plan trades process isolation for in-process execution on two
never-fail commands, and the safety-critical consequences are only partially
addressed. The stated never-fail contract is implemented purely as an Err-catching
boundary, so any panic in gix/jj-lib or the change-id slice below it escapes and
crashes the subcommand. The plan also deletes the 10-second cap without replacing
it, leaving the working-copy-lock-acquiring jj snapshot unbounded — though the
object-store write it acknowledges is genuinely safe and the loss of the
persist-on-status side effect is an improvement, not a data-loss risk.

**Strengths**:
- The adapter-failure fallback is preserved correctly: any `Err` folds to the exact
  literal with an `ACCELERATOR_LOG` warn carrying the backend token.
- The AC6 logging-init is made fail-safe: a malformed `ACCELERATOR_LOG` prints to
  stderr and continues.
- Dropping the jj snapshot without `finish()` removes a side effect without losing
  work — a subsequent real `jj` re-snapshots current on-disk state.
- The object-store write is content-addressed, idempotent, atomic loose objects, and
  GC-reclaimable; the plan's two claims are consistent once scoped.
- The release profile leaves `panic = "unwind"`, so a panic during snapshot releases
  the working-copy lock via RAII (no stale lock).

**Findings**:
- **Major / high** — Phase 1 §4: the never-fail boundary catches `Err` but not
  panics; `reverse_hex()[..12]` and gix/jj-lib parsing of repository-controlled data
  can panic and unwind through `main` (no `catch_unwind`), aborting with exit 101.
  Wrap the call in `catch_unwind`; use `.get(..12)`.
- **Major / medium** — Performance Considerations / Phase 3 §1: deleting the 10s cap
  leaves the in-process status/log with no time bound; a pathological working copy or
  contended lock can hang `vcs status` indefinitely, recoverable only by Ctrl-C.
- **Minor / medium** — Phase 1 §3: jj status acquires the working-copy lock via
  `start_working_copy_mutation()`; a concurrent long-running jj holding it stalls
  `vcs status` with no bound (compounding the deleted cap). No corruption risk.
- **Suggestion / high** — What We're NOT Doing (line 148) / Migration / Performance:
  "mutates nothing" understates that status writes objects to the backend on a read;
  align the wording with the `dirty_paths.rs` module-doc framing.

## Re-Review (Pass 2) — 2026-08-31

**Verdict:** APPROVE

All eight lenses were re-run against the edited plan. Every review-1 finding is
resolved. The five structural lenses whose review-1 majors were the load-bearing
ones — architecture (`ReportError` → `kernel::Error`), standards (the two gate
fixes), plus security, safety, and compatibility — explicitly confirmed the fixes
landed and, in architecture's words, "the recent edits improve it". The re-review
then surfaced 9 new majors, nearly all refinements of the review-1 edits rather
than structural defects; all 9 were addressed in a follow-up edit pass, together
with the minors and suggestions.

⚠️ The follow-up edit pass that resolved the re-review's new findings was **not**
itself re-run through the lenses. Risk is low (the edits were localised
wording/spec tightening and making three conditionals mandatory, not structural
change), but a further pass could confirm no new issues were introduced.

### Previously Identified Issues (review-1) — all resolved

- 🟡 **Architecture / Code Quality**: `ReportError` false orphan premise + error
  collapse — **Resolved**. Port now returns `kernel::Error`; architecture confirmed
  it matches the `CheckoutProbe`/`ModeProbe`/`OriginRemote` family.
- 🟡 **Correctness**: ahead/behind log goldens ≠ `clean-git` — **Resolved** (scoped
  to status; log reflects own history).
- 🟡 **Safety / Security / Architecture**: never-fail boundary catches only `Err` —
  **Resolved**. `catch_unwind` added; safety confirmed the lock/RAII interaction is
  safe and `panic=unwind` holds workspace-wide. Hang/OOM accepted and documented.
- 🟡 **Standards**: feature-graph task; `_EXEMPT` anti-vacuity test — **Resolved**
  (both re-confirmed correct).
- 🟡 **Compatibility / Architecture**: Open Question + pre-1.0 coupling —
  **Resolved** (accept-full-migration note, coupled API points enumerated).
- 🟡 **Code Quality / Standards**: inline comments — **Resolved** (stripped; generic
  `git_err`/`jj_err` helper; no comments remain).
- 🟡 **Test Coverage** (deleted/rename, five-commit cap, AC6 wiring, unborn HEAD,
  jj bookmarks) — **Resolved** (states + assertions added in §7).
- 🟡 **Correctness**: collapse tie-break unspecified — **Resolved** (precedence
  stated; then corrected — see new issues).

### New Issues Introduced/Surfaced by the review-1 edits — all now addressed

- 🟡 **Code Quality / Architecture / Compatibility**: the "chain `source()`" mandate
  conflicted with reusing the shared `From` (5-sibling-port ripple; `kernel::Error`
  has no `source()` channel) — **Fixed**: dropped the mandate; token from `kind`,
  top-level Display suffices, shared `From` untouched.
- 🟡 **Code Quality / Correctness**: pure `classify` `-> ChangeType` could not
  express rename's two entries — **Fixed**: returns `Vec<FileChange>` over a neutral
  input value.
- 🟡 **Correctness**: `Copied` mapped the unchanged source to `deleted` — **Fixed**:
  `Copied → added`-only, split from `Renamed`.
- 🟡 **Correctness**: collapse precedence omitted `Untracked` (the `git rm --cached`
  collision) — **Fixed**: `Conflicted > Deleted > Added > Untracked > Modified`.
- 🟡 **Test Coverage**: sha256 assertion had no fixture home — **Fixed**: dedicated
  `sha256-git` state in §7.
- 🟡 **Test Coverage**: AC6 stderr-token test could degrade to manual — **Fixed**:
  required `D2`-fixture integration test.
- 🟡 **Test Coverage**: adversarial zero-spawn state was optional — **Fixed**: now a
  required, always-present state.
- 🟡 **Standards**: Phase 1 omitted the `pup` gate — **Fixed**: `pup:check` +
  `test:integration:pup` added.
- 🟡 **Standards**: new crate-wide `std::process` rule lacked a probe pair —
  **Fixed**: probe pair added to `test_import_rule.py`.

### Minors and suggestions also addressed

- Gated `logging::init()` on `ACCELERATOR_LOG` (compat: unconditional init would
  make `detect`/`guard` emit INFO to stderr by default — verified against
  `logging.rs`).
- Reconciled the env-scrub note with `scrub.rs` (which proves the queries are
  env-invariant): extend `scrub.rs` to pin status/log rather than claim a reduction.
- Downcast the panic payload into the `warn!`; noted the `panic=unwind` dependency +
  a compiled-release-binary panic test; softened "restores the contract" to "panics
  that unwind cleanly"; noted `discover`/`kind` sit outside the guard.
- `working_copy_diff` returns a named `WorkingCopySnapshot`, presence encodes the
  `is_present() && !is_tree()` keep predicate; jj cap fixture needs six ancestors of
  `@`; negative log assertion runs on unmasked output; boundary tests moved to
  Phase 1; disk axis, jj-lock, Ctrl-C stale-lock, and LLM-prompt-injection surface
  all documented; jj-lib seam acknowledged as two sites; `VcsKind::None`
  per-method divergence and `vcs_settings.py` docstring sync noted.

### Assessment

The plan is in good shape and ready for implementation. The structural decisions
from review-1 are settled and confirmed by the re-review; the remaining work the
re-review surfaced was refinement, now applied. The one open caveat is that this
last edit pass was not itself agent-re-reviewed — addressed by the Pass 3 below.

## Re-Review (Pass 3) — 2026-08-31

**Verdict:** APPROVE

All eight lenses were re-run over the pass-2 edits. **No lens found a structural
defect**, and each lens verified its pass-2 fixes correct against the codebase
(e.g. `kernel::Error` has no `source()` channel; `logging.rs` `filter_from_env(None)`
defaults to INFO so the gate is needed; `dirty_paths.rs:147` is exactly
`is_present() && !is_tree()`; the `vcs_adapters::library` permit list genuinely
excludes `kernel` yet the fully-qualified `kernel::Error` reference is fine). The
pass surfaced 2 medium-confidence majors and a batch of minors, all now addressed.

### Pass-2 fixes confirmed correct (per lens)

- ✅ **Architecture / Code Quality / Compatibility**: the dropped source-chaining
  mandate — verified non-rippling (no `source()` channel), port matches the sibling
  family.
- ✅ **Security / Compatibility**: the `logging::init()` gate on `ACCELERATOR_LOG` —
  verified against `logging.rs` as exactly what preserves the quiet `detect`/`guard`
  default and ADR-0066's disclosure boundary.
- ✅ **Safety / Correctness**: the `catch_unwind` scope, downcast, `.get(..12)`, and
  `is_present() && !is_tree()` encoding — all verified sound.
- ✅ **Standards**: pup permit list, `_EXEMPT`/docstring move, feature-graph
  constants, per-phase public-api handling — all verified correct.
- ✅ **Test Coverage**: every AC1–AC8 maps to a concrete named automated test; the
  cap fixture is correctly sized for the git/jj `@`-exclusion asymmetry.

### New issues surfaced by pass 2 — all now addressed

- 🟡 **Safety / Architecture / Test Coverage** (convergent): the `panic=unwind`
  guarantee was enforced only by an unrealizable release-binary panic test —
  **Fixed**: replaced with a static manifest guard asserting `[profile.release]`
  omits `panic = "abort"`, as a tracked criterion.
- 🟡 **Compatibility**: "pin status/log env-invariant" was vacuous — status reads
  global `core.excludesFile`/`status.showUntrackedFiles` the facts path never
  surfaced — **Fixed**: characterise the real sensitivity (enrich the `scrub.rs`
  poison + route status/log through it), and document that status now honours the
  user's global git config (the scrub was the anomaly) as an accepted behaviour
  change in Migration Notes.
- 🔵 **Correctness**: the `Deleted > Added` rank mislabelled a staged-add +
  worktree-delete as `deleted` — **Fixed**: replaced the fixed `ChangeType` rank
  with a commit-accurate rule (conflict overrides; else the staged type wins),
  expressed as a pure `resolve()` beside `classify`.
- 🔵 **Architecture / Code Quality**: the shared `working_copy_diff` primitive sat
  in a status/log-named module (inverting `dirty_paths`'s dependency) — **Fixed**:
  moved to a new `library/snapshot.rs`; `_EXEMPT`, docstring, and the renamed
  `Error::JjWorkingCopyDiff` variant all follow.
- 🔵 **Correctness**: `jj_log` needs its own no-wc-commit guard (it doesn't call
  `working_copy_diff`) — **Fixed**.
- 🔵 **Test Coverage / Security**: the `jj-lib` token arm, the zero-spawn non-empty
  guard, the tree-valued `jj_dirty_paths` case, the broadened adversarial config
  (`filter.<name>.process`, `core.hooksPath`), and routing status/log through the
  poisoned scrub run — all now specified.
- 🔵 **Standards / Safety / Code Quality**: named the new pup rule
  (`vcs_adapters_is_zero_spawn`); scrubbed stale `subprocess` doc references; made
  the jj-lock-mechanism check a blocking pre-req; noted the `adapter` vs `vcs`
  `warn!` field split; made the `/commit` delimiter-framing an in-scope follow-up.

### Assessment

The plan is ready for implementation. Three passes have converged: pass 1 fixed
structural issues, pass 2 fixed refinements of those fixes, and pass 3 found no
structural defects — only localised tightenings, now applied. Further passes would
yield diminishing returns. Two items are flagged as **pre-implementation
determinations** rather than plan defects: whether jj-lib 0.43's working-copy lock
is a marker file (safety), and the exact resolved `gix` feature set incl. `dirwalk`
(compatibility) — both already called out in the plan as resolve-before-coding.

---
*Re-review generated by /accelerator:review-plan*
