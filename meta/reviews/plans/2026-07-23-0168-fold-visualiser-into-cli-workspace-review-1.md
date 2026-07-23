---
type: plan-review
id: "2026-07-23-0168-fold-visualiser-into-cli-workspace-review-1"
title: "Plan Review: Fold the Visualiser into the cli/ Workspace"
date: "2026-07-23T08:53:32+00:00"
author: Toby Clemson
producer: review-plan
status: complete
target: "plan:2026-07-23-0168-fold-visualiser-into-cli-workspace"
reviewer: Toby Clemson
verdict: APPROVE
lenses: [architecture, code-quality, test-coverage, correctness, security, safety, compatibility, portability]
review_number: 1
review_pass: 3
tags: [rust, visualiser, cli, launcher, corpus, workspace]
last_updated: "2026-07-23T10:06:43+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

## Plan Review: Fold the Visualiser into the cli/ Workspace

**Verdict:** REVISE

The plan is well-sequenced, TDD-disciplined (parity fixtures frozen before
deletion), and architecturally sound: it collapses ~15.4k lines of duplicated
domain logic onto the shared crates, keeps a clean sync-domain/async-shell split
behind `spawn_blocking`, confines serde to thin wire views over the serde-free
`corpus::DocTypeKey`, and consolidates distribution onto the signed manifest.
There are **no critical findings**, but **21 major findings** cluster around a
handful of concrete gaps: a recycle-guard identity confusion that would break the
single safeguard against SIGKILL hitting a recycled PID; a parity net that is
narrower than the modules it must protect (missing YAML-dialect coercion,
patcher/linkage, and doc-type matcher divergence); a release-safety coupling
where the old distribution path is deleted before 0165's live manifest carries
the `visualiser` entry; a version-coherence gate left reading a now-absent literal
between Phases 1 and 4; and an incomplete path-repointing inventory that would
break Phase 1's own green-gate criterion. Each is addressable with targeted plan
edits rather than a redesign.

### Cross-Cutting Themes

- **Recycle guard conflates owner-PID with server-PID** (flagged by: correctness
  [high], safety) — The plan says `stop` terminates "only when recorded
  owner-PID **and** start_time both still match." But in the live system the kill
  guard compares the **server's own** pid + start_time (`server-info.json`),
  whereas owner-PID/start_time is the **parent Claude process** used solely by
  idle self-shutdown. Keying the kill on the owner identity would break the
  recycle guard entirely — killing an unrelated recycled PID or refusing to stop
  a legitimate server. This is the review's single highest-consensus, high-
  confidence defect.

- **Parity net is narrower than the modules it must protect** (flagged by:
  code-quality, test-coverage, correctness, compatibility) — The frozen fixtures
  cover only frontmatter map / slug / doc-type on clean inputs. Three gaps recur:
  (a) `patcher::patch_status`, `typed_ref`, and `related`/`clusters`/`cluster_key`
  → `corpus::linkage` are retired in the same "parity-verified" phase with no
  frozen equivalence check; (b) the `serde_yml` → serde-saphyr swap crosses the
  YAML-1.1↔1.2 boundary (`yes`/`no`/`on`/`off`, unquoted numbers/versions, null
  spellings, duplicate keys) whose scalar-type flips reach the React frontend as
  JSON, yet are unfixtured; (c) the `infer` vs `kind_for_canonical_path` matcher
  swap only diverges on nested/overlapping roots, which the one-doc-per-variant
  fixtures never exercise.

- **Distribution cut-over deletes the failover before the replacement is proven
  live** (flagged by: architecture, safety [high]) — Phase 3 reroutes SKILL.md
  through `accelerator visualiser start` and Phase 4 deletes the old fetch path
  (`checksums.json`, `launch-server.sh`), but launcher resolution fails closed
  with `AssetNotFound` until 0165's live manifest carries `visualiser`. The plan
  gates only the *test assertion* on that entry, not the *deletions/merge* — so a
  release cut before 0165 lands is a full feature outage with no user-facing
  recovery (the only fallback, `ACCELERATOR_VISUALISER_BIN`, is an undocumented
  internal override).

- **Version-inheritance and coherence-gate rewrite are split across phases**
  (flagged by: compatibility [high], test-coverage) — Phase 1 sets
  `version.workspace = true` (dropping the literal), but `validate_version_
  coherence` still calls `_read_cargo_toml_version` until Phase 4, so after
  Phase 1 it reads `{"workspace": true}` and always mismatches. `mise run` stays
  green (it does not invoke coherence), but every release/version/manifest task
  is broken for the whole Phase 1→3 window — contradicting "independently
  mergeable."

- **doc-type matcher decision deferred as "pick one"** (flagged by:
  architecture, code-quality, correctness, test-coverage) — Recommending `infer`
  but leaving the choice open understates a genuine semantics change
  (deterministic longest-match vs iteration-order first-match) and hands the next
  maintainer an unrecorded, unfixtured behavioural decision.

- **config.rs reshape leaves dead duplication and an undescribed end-state**
  (flagged by: code-quality) — Phase 3 gives the server the `config` dependency
  its hand-synced `DEFAULT_IDLE_TIMEOUT`/`DEFAULT_KANBAN_COLUMN_KEYS` constants
  were a workaround for, but never schedules their removal; and the ~900-line
  three-responsibility module has no described target shape after the ID logic
  and `config.json` reader are excised.

- **Write-path preservation is only manually spot-checked** (flagged by:
  test-coverage, safety, code-quality) — Repointing the atomic write through the
  sync `FileCorpusStore` across a new `spawn_blocking` seam is verified for
  perms/line-endings and concurrent-write safety only by manual verification,
  despite being the highest-risk surface (it writes real user documents).

- **The one network daemon is exempted from the workspace panic/unwrap policy**
  (flagged by: architecture, security) — Keeping the crate out of workspace
  lints (and pup.ron) leaves the workspace's only long-running HTTP daemon
  outside the very DoS-via-panic policy the rest of the workspace enforces.

### Tradeoff Analysis

- **Lint strictness vs pragmatic large-crate migration**: Security wants the
  request-handling paths under `unwrap_used`/`panic` denial (a panic in a handler
  is a local DoS); Architecture concedes a blanket opt-out may be a justified
  interim for a large pre-existing axum crate. Recommendation: don't blanket-
  exempt — either opt the request/SSE modules in with narrow justified
  exceptions, or record the exemption as tracked debt with a concrete follow-up
  story so it does not silently become permanent.

- **Phase independence vs release-pipeline integrity**: The plan's headline value
  is four independently-mergeable phases, but two contracts (version-coherence,
  distribution) span phase boundaries. Recommendation: pull the coherence-gate
  reader removal forward into Phase 1 and bind the Phase 4 deletions to 0165's
  live manifest — accept a small loss of "pure" independence to keep every
  mergeable point releasable.

### Findings

#### Critical

_None._

#### Major

- 🟡 **Correctness**: Recycle guard conflates the owner PID with the server's own PID
  **Location**: Phase 3, Section 2: Lifecycle port (Rust)
  The plan ties the stop guard to the "owner-PID/start_time handshake," but the
  live kill guard compares the **server** pid + start_time; owner-PID is the
  parent Claude process used only for idle shutdown. Keying the kill on the owner
  identity breaks the recycle guard. (Confidence: high.)

- 🟡 **Safety**: Cut-over routes production binary resolution through a manifest that lacks the visualiser entry until 0165
  **Location**: Phase 4 / Phase 3 §4 SKILL.md switch; Migration Notes
  Launcher resolution fails closed (`AssetNotFound`) when the manifest lacks the
  entry; the plan gates only the test assertion on 0165, not the deletions. A
  release cut before 0165 is a full feature outage with no user-facing fallback.
  (Confidence: high.)

- 🟡 **Compatibility**: Version-inheritance and coherence-gate rewrite split across phases breaks the release pipeline in the interim
  **Location**: Phase 1 §2 vs Phase 4 §2
  After Phase 1, `_read_cargo_toml_version` reads `{"workspace": true}` and always
  mismatches; the reader is only removed in Phase 4. Every release/version/manifest
  task is broken for the Phase 1→3 window. (Confidence: high.)

- 🟡 **Portability**: Repo-relative test paths that escape the crate break on relocation
  **Location**: Phase 1, Section 1: File move (unit) / Key Discoveries
  Only `../frontend/dist` was audited, but `slug.rs:574,595`
  (`../../../../skills/...`), `config_contract.rs:75` (`../../../../templates`),
  and `config_contract.rs:7` (`../scripts/write-visualiser-config.sh`) derive
  repo-layout paths from `CARGO_MANIFEST_DIR` at the old depth-4 location. From
  the new depth-3 location the `..` counts overshoot — failing Phase 1's own
  green-gate criterion. (Confidence: high.)

- 🟡 **Security**: Blanket advisory-ignore instruction can silently suppress real RUSTSEC vulnerabilities
  **Location**: Phase 1, Section 3: cargo-deny license reconciliation
  "Add advisory `ignore` entries the new graph requires… derived empirically from
  deny:check output" can mute a genuine vulnerability advisory in a network-facing
  daemon's tree; unused ignores linger after their crate leaves the closure.
  (Confidence: high.)

- 🟡 **Compatibility**: Fixture key rename has an unlisted co-located unit-test consumer
  **Location**: Phase 4 §1: manifest.example.json key rename
  The fixture is `include_str!`-embedded by `manifest.rs:135`, whose unit test
  asserts `platform_entry("accelerator-visualiser", …)` / `bare_sha256(…)` at
  `:146-148`. Renaming the fixture key without updating those assertions breaks
  `cli:check`. (Confidence: high.)

- 🟡 **Code Quality**: Hand-synced fallback constants become removable dead duplication after the config dependency lands
  **Location**: Phase 3 §3: Direct config reading (Model 1)
  Phase 3 gives the server the `config` dependency that `DEFAULT_IDLE_TIMEOUT`/
  `DEFAULT_KANBAN_COLUMN_KEYS` ("keep in sync" comments) existed to avoid, but
  never schedules their removal — preserving the DRY violation the consolidation
  targets. (Confidence: high.)

- 🟡 **Architecture**: Old distribution path deleted before the live launcher manifest is guaranteed to carry the visualiser
  **Location**: Phase 4 / "What We're NOT Doing"
  The deletions/merge should be ordered after (or co-merged with) 0165's live
  manifest entry, not just the test assertion. (Confidence: medium.)

- 🟡 **Architecture**: New workspace member opts out of both the workspace lint policy and pup.ron governance
  **Location**: Phase 1 §2 / "What We're NOT Doing" (pup.ron)
  The largest member escapes the panic/unwrap and structural rules every sibling
  meets — a permanent governance asymmetry unless recorded as tracked debt.
  (Confidence: high.)

- 🟡 **Code Quality**: Phase 2 change surface is wider than its "parity-verified" net
  **Location**: Phase 2: Refactor onto the Shared Crates
  The frozen fixtures cover frontmatter/slug/doc-type only, but Phase 2 also
  retires patcher, typed-ref, and linkage/cluster and re-homes the atomic-write
  path — none exercised by parity. (Confidence: medium.)

- 🟡 **Code Quality**: config.rs mixes schema, domain, and resolution; end-state shape is undescribed
  **Location**: Phase 2 §3 / Phase 3 §3: config.rs reshape
  A ~900-line three-responsibility module is reshaped with no described target,
  risking an SRP-violating translation layer. (Confidence: medium.)

- 🟡 **Test Coverage**: Parity fixtures omit patcher/typed-ref/linkage modules
  **Location**: Phase 2, Section 1: Freeze parity golden fixtures
  `patch_status` byte output and `linkage` records (Band classification,
  TYPE_PAIRS) get no frozen-before-deletion assertion — only pre-existing example
  tests not designed to catch engine-swap divergence. (Confidence: medium.)

- 🟡 **Test Coverage**: doc-type matcher divergence not exercised by fixtures
  **Location**: Phase 2, Section 3: doc-type matcher choice
  One-doc-per-variant clean-path fixtures never hit the nested/overlapping-root
  inputs where `infer` and `kind_for_canonical_path` differ; `config_path_key` is
  also unpinned. (Confidence: medium.)

- 🟡 **Test Coverage**: No concurrent-write regression test for the spawn_blocking store swap
  **Location**: Performance Considerations / Phase 2 §3
  The etag-verify-then-write flow under concurrent SSE writes is only manually
  confirmed; a lost-update/etag-race/deadlock on the highest-risk (user-document)
  surface would go uncaught. (Confidence: medium.)

- 🟡 **Correctness**: Parity fixtures omit the YAML-dialect divergences that are the real engine-swap risk
  **Location**: Phase 2, Section 1: Freeze parity golden fixtures
  `serde_yml` (1.1) → serde-saphyr (1.2) can flip `yes`/`no`/`on`/`off`,
  `1.20`, null spellings, and duplicate-key handling — value-type changes that
  are in scope yet unfixtured. (Confidence: medium.)

- 🟡 **Correctness**: Status token transitions omit the crashed / self-shutdown "stale" state
  **Location**: Phase 3, Section 2: status tokens
  The shell distinguishes `stale` (info file present, PID dead or start_time
  mismatched) — exactly what idle-shutdown/crash leaves behind. A naive "info file
  exists → running" reports a dead server as `running` with a stale URL.
  (Confidence: medium.)

- 🟡 **Correctness**: infer vs kind_for_canonical_path change resolution semantics unverified by parity fixtures
  **Location**: Phase 2, Section 3: doc-type matcher choice
  The two matchers diverge on nested roots and non-anchored segments; the clean-
  path fixtures give false confidence. (Confidence: medium.)

- 🟡 **Correctness**: idle_timeout disable-token semantics depend on the raw string surviving compose
  **Location**: Phase 3, Section 3: Direct config reading (Model 1)
  `resolve_idle_limit_ms` needs the verbatim token for its `never`/`0`/zero-length/
  empty disable set; if `ConfigService::effective` normalises/trims/coerces or
  substitutes the `8h` default for an empty value, disable semantics change
  silently. Phase 3 tests omit the zero-length and empty-string cases.
  (Confidence: medium.)

- 🟡 **Security**: Release-staged visualiser binary feature profile unspecified — risks shipping the dev-frontend loopback bypass
  **Location**: Phase 4, Section 1: Producer wiring
  The `dev-frontend`-gated `e2e_insecure_allowed` relaxes both the non-loopback
  bind and Host-header guards; the plan never states the staged artifact must be
  built with default (`embed-dist`) features only, and neighbouring tasks build
  `--all-features`. (Confidence: medium.)

- 🟡 **Safety**: Orphaned old-version server on upgrade has an unclear recovery path
  **Location**: Migration Notes
  The old server is *not* unrelated — its PID/start_time live in the preserved
  state files. If the new Rust stop computes start_time in a different
  representation than the shell recorded, every cross-upgrade stop refuses,
  accumulating orphans that keep writing to `.accelerator/*.md`. (Confidence:
  medium.)

- 🟡 **Compatibility**: serde_yml → serde-saphyr scalar coercion divergence reaches the frontend as JSON
  **Location**: Phase 2 §1-3: frontmatter engine swap
  The parsed map is serialised to JSON for the SPA; a string→bool/number flip
  silently alters the shape the frontend keys on while structural parity passes.
  (Confidence: medium.) [Overlaps the Correctness YAML-dialect finding from the
  consumer-contract angle.]

#### Minor

- 🔵 **Code Quality**: Doc-comments and error-message sync notes referencing deleted scripts will go stale
  **Location**: Phase 4 §4: Remove the retired surface
  `config.rs` module header, `ConfigError::InvalidIdleTimeout` note, and
  `main.rs` `--config` help all cite `launch-server.sh`/`write-visualiser-config.sh`
  after those files are deleted.

- 🔵 **Code Quality**: "Pick one" doc-type matcher understates a real design decision
  **Location**: Phase 2 §3: doc-type call-site replacement
  Iteration-order first-match vs deterministic longest-match are materially
  different designs; record the choice and rationale.

- 🔵 **Test Coverage**: Perms/line-ending preservation across the write-path swap is only manually spot-checked
  **Location**: Phase 2 Manual Verification / Phase 3 orchestration
  No automated test round-trips a CRLF document at a non-default mode through the
  new `FileCorpusStore` write path.

- 🔵 **Test Coverage**: status stale/recycled-info-file case untested
  **Location**: Phase 3, Section 2: status tokens
  The status↔recycle-guard interaction (info present, owner PID recycled) is the
  one lifecycle state with no coverage.

- 🔵 **Test Coverage**: Version-coherence gate tests not updated when the two readers are dropped
  **Location**: Phase 4, Section 2
  No assertion that a skewed visualiser member version is still caught via
  `_read_workspace_version` after `_read_checksums_json_version`/
  `_read_cargo_toml_version` are removed.

- 🔵 **Test Coverage**: Dispatch test left as spy-or-black-box; the load-bearing check is under-committed
  **Location**: Phase 3, Section 5: Dispatch tests
  External dispatch is name-agnostic, so a `visualiser`-specific `RecordingExec`
  unit assertion adds little; commit to the black-box `launcher_for("visualiser",
  "ACCELERATOR_VISUALISER_BIN")` exec-replace test per subcommand.

- 🔵 **Correctness**: Owner-PID grandparent heuristic may not survive the launcher exec-replace
  **Location**: Phase 3, Section 1: Subcommand surface
  The old `ppid_of($PPID)` two-level-up heuristic assumed the shell tree; under
  `external_subcommand` → exec-replace the ancestry differs, risking monitoring
  the wrong owner process.

- 🔵 **Correctness**: Detached daemon is not a child of stop; must poll signal-0, not waitpid
  **Location**: Phase 3, Section 2: SIGTERM → 2s → SIGKILL
  A `waitpid`-based stop gets `ECHILD`; the port must poll `kill(pid,0)`. Also
  drop the shell's legacy ±1s start_time drift tolerance now that both sides use
  identical `process_start_time`.

- 🔵 **Security**: Network-facing daemon deliberately exempted from the workspace unwrap/panic lint policy
  **Location**: Phase 1, Section 2
  A panic in a request/SSE path crashes the workspace's one HTTP daemon (local
  DoS) — the failure mode the workspace policy exists to catch.

- 🔵 **Security**: Local-manifest test asserts only the happy path, not signature/hash rejection
  **Location**: Phase 4, Section 3
  Add a tamper case (mutated bytes → ChecksumMismatch, foreign-key signature →
  SignatureMismatch) for the visualiser entry, or note the generic negative cases
  cover the mechanism.

- 🔵 **Safety**: Recycle-guard identity must key on the server PID's start_time, not the owner-PID's
  **Location**: Phase 3 §2 (Safety-lens restatement of the Correctness major)
  Gate the kill strictly on the server PID's start_time; keep owner-PID confined
  to idle shutdown.

- 🔵 **Safety**: Forced-SIGKILL post-shutdown invariant and temp-file cleanup not carried into the Rust port
  **Location**: Phase 3, Section 2
  The shell synthesises `server-stopped.json` and unlinks pid/info files after a
  forced kill; dropping this leaves lifecycle files inconsistent, and a SIGKILL
  mid-write can orphan temp files in `.accelerator/`.

- 🔵 **Compatibility**: thiserror 1→2 major bump is not a no-op
  **Location**: Phase 1 §2 / Phase 2 §2
  2.0 changes `#[from]`/`#[error(transparent)]` and display field-reference rules;
  budget a derive-audit step, not just a version-line swap.

- 🔵 **Compatibility**: License allowances added in Phase 1 for YAML deps go unused after Phase 2
  **Location**: Phase 1 §3 vs Phase 2 §2
  Re-prune `deny.toml` after dropping `gray_matter`/`serde_yml` to keep the exact-
  closure convention and avoid unused-allowance warnings.

- 🔵 **Portability**: Crate-local target/ derivations must switch to workspace-shared cli/target/; dev.py:43 unlisted
  **Location**: Phase 1, Section 4
  `tasks/dev.py:43` (`_SERVER_BIN = SERVER / "target/debug/..."`) is omitted;
  derive from `CLI_DIR / "target"`, not `SERVER / "target"`.

- 🔵 **Portability**: Hardcoded skills/visualisation/visualise path inventory is incomplete
  **Location**: Phase 1 §4 / Phase 4 §4
  Live references also exist in `tasks/lint/scripts.py`, `tasks/shared/sources.py`,
  the `tests/unit/tasks/*` assertions, `tests/conftest.py`, and
  `.github/workflows/main.yml`; grep the whole tree before Phase 1.

- 🔵 **Architecture**: Doc-type inference strategy left as an implementer choice
  **Location**: Phase 2, Section 3
  Commit to `corpus::doc_type::infer` so all corpus consumers share one inference
  semantics.

#### Suggestions

- 🔵 **Architecture**: Config-resolution parity asserted only for idle_timeout
  **Location**: Phase 3, Section 3
  Extend Phase 3 config-resolution assertions to the doc-path/template/work-item/
  kanban settings the server also consumes, confirming the resolution boundary
  moved faithfully.

- 🔵 **Code Quality**: Document the async-façade-over-sync-store spawn_blocking seam
  **Location**: Phase 2 §3 / Phase 3 §2
  Add a short module/seam doc-comment stating where blocking work runs and that
  the per-path mutex still bounds contention.

- 🔵 **Portability**: New Rust orchestration path is unix-only by construction — make the coupling explicit
  **Location**: Phase 3, Section 2
  Note that `nix::sys::signal::kill` + `process_start_time()` are unix-only
  (matching the darwin/linux target closure) and that any non-unix target would
  need a portable termination + start-time strategy.

### Strengths

- ✅ Parity-first sequencing: golden fixtures are frozen against the pre-refactor
  engine **before** any deletion — the correct golden-master ordering.
- ✅ Clean functional-core/imperative-shell split: sync domain moves to
  corpus/document; the async I/O boundary stays behind `spawn_blocking`.
- ✅ Serde is confined to server-owned wire views over the serde-free
  `corpus::DocTypeKey` via `wire_str`/`from_wire_str`; the shared domain type is
  not polluted with API concerns.
- ✅ Doc-type wire strings and back-compat config keys are preserved
  (`config_path_key`: PrDescriptions→`prs`, Research→`research_codebase`), so the
  frontend JSON and existing user config keep resolving.
- ✅ The distribution cut-over strictly *strengthens* integrity: sha256-only
  `checksums.json` → sha256 + minisign verified against the embedded trusted key,
  failing closed with no unverified fallback; serde-saphyr confinement is
  re-asserted structurally as a Phase 2 criterion.
- ✅ Strong DI posture: injected `IdScanner`, the `AtomicWrite`/`FileCorpusStore`
  port, and reuse of the existing `FileDriver` trait keep components testable in
  isolation; established seams (`RecordingExec`, `accelerator-fixture`,
  `MockServer`) are reused rather than reinvented.
- ✅ Idle-timeout is verified deterministically (short-timeout shutdown +
  `never`/0 stays-up + `8h` resolution assertion), not by sleeping; the
  Host/Origin 403 guards are exercised independently; the recycle guard and the
  loopback fail-closed model are explicit acceptance criteria.
- ✅ The frontend+server move as a unit, preserving the `../frontend/dist` embed;
  the plan already recognises the workspace-shared `cli/target/` and repoints the
  E2E `SERVER_BIN` and cross-compile output.

### Recommended Changes

1. **Correct the recycle-guard identity** (addresses: "Recycle guard conflates
   the owner PID with the server's own PID"; "Recycle-guard identity must key on
   the server PID's start_time"). In Phase 3 §2, state that stop/status gate the
   kill strictly on the **server's** recorded pid + start_time from
   `server-info.json` (refuse on any mismatch or unreadable start_time), keep the
   owner-PID/start_time pair confined to idle self-shutdown, and add a test that
   exercises exactly that split.

2. **Bind the distribution deletions to 0165's live manifest** (addresses: "Cut-
   over routes production binary resolution through a manifest that lacks the
   entry"; "Old distribution path deleted before the live manifest"). Make it an
   enforced release gate — do not tag a release carrying the Phase 3 SKILL.md
   switch or Phase 4 deletions until the live manifest verifiably carries a
   `visualiser` entry (e.g. a release-time coherence assertion against the real
   manifest, or land the deletions in the same release train as 0165).

3. **Pull the coherence-gate reader removal into Phase 1** (addresses: "Version-
   inheritance and coherence-gate rewrite split across phases"; "Version-coherence
   gate tests not updated"). Move the `_read_cargo_toml_version`/
   `_read_checksums_json_version` removal from Phase 4 §2 into Phase 1 alongside
   the `version.workspace = true` change so the release pipeline never reads an
   absent literal, and add a coherence test asserting a skewed member version is
   caught via `_read_workspace_version`.

4. **Complete the path-repointing inventory before Phase 1** (addresses: "Repo-
   relative test paths that escape the crate"; "Crate-local target/ derivations;
   dev.py:43 unlisted"; "Hardcoded path inventory is incomplete"). Grep the whole
   tree for `skills/visualisation/visualise` and every `CARGO_MANIFEST_DIR`-
   relative `..` escape, and fold each hit — `slug.rs`/`config_contract.rs` test
   paths, `tasks/dev.py:43`, `tasks/shared/sources.py`, `tasks/lint/scripts.py`,
   the `tests/unit/tasks/*` assertions, `tests/conftest.py`, and
   `.github/workflows/main.yml` — into the Phase 1 repointing list. Derive
   crate-local binary paths from `CLI_DIR / "target"`, not `SERVER / "target"`.

5. **Widen the parity net to the whole retired surface** (addresses: "Parity
   fixtures omit patcher/typed-ref/linkage"; "YAML-dialect divergences"; "infer
   vs kind_for_canonical_path unverified"; "scalar coercion reaches the frontend").
   Extend the frozen fixtures to cover: (a) `patch_status` byte output and
   `linkage` records over the same corpus; (b) YAML-1.1↔1.2 scalar cases
   (`yes`/`no`/`on`/`off`, unquoted numbers/versions, null spellings, duplicate
   keys) asserting the JSON-serialised *value type*; (c) nested/overlapping-root
   and non-anchored-segment paths plus a `config_path_key` pin for all 14
   variants. Either rename the phase to reflect the true covered surface or split
   the non-parity-covered rewires into a separately-verified step.

6. **Resolve the doc-type matcher choice in the plan** (addresses: "'Pick one'
   understates a real design decision"; "Doc-type inference left as implementer
   choice"). Commit to `corpus::doc_type::infer`, record the rationale, and
   confirm the `infer` table is built from the same canonicalised absolute roots
   the old matcher used.

7. **Schedule config.rs cleanup and describe its end-state** (addresses:
   "Hand-synced fallback constants become dead duplication"; "config.rs mixes
   responsibilities"; "Doc-comments referencing deleted scripts"). Add a Phase 3
   step to delete `DEFAULT_IDLE_TIMEOUT`/`DEFAULT_KANBAN_COLUMN_KEYS` (sourcing
   from the catalogue), specify the surviving module's shape (a thin adapter over
   `ConfigService::effective`), and sweep stale doc-comments/error notes/CLI help
   that cite the deleted scripts.

8. **Pin the idle-timeout token contract across compose** (addresses: "idle_
   timeout disable-token semantics depend on the raw string surviving compose").
   Assert the composed effective value hands `resolve_idle_limit_ms` the raw token
   unchanged, and extend Phase 3 tests to cover zero-length (`0s`/`0ms`) and
   empty-string cases, not just `never`/`0`.

9. **Add the stale/self-shutdown status state and its test** (addresses: "Status
   tokens omit the crashed/self-shutdown stale state"; "status stale/recycled-
   info-file case untested"). Specify that `status` maps to `stopped` whenever the
   recorded PID is not alive OR its start_time no longer matches, and test a
   server that dies without cleanup.

10. **Constrain the release binary feature profile and harden the deny/lint
    posture** (addresses: "Release-staged binary feature profile unspecified";
    "Blanket advisory-ignore"; "daemon exempted from unwrap/panic policy"). Make
    it a Phase 4 criterion that the staged `visualiser` binary is built with
    default `embed-dist` features (with a negative assertion that the insecure env
    var is inert); require every advisory `ignore` to carry a per-entry
    justification and never ignore a `vulnerability`-class advisory; and either
    opt the request/SSE modules into `unwrap_used`/`panic` denial or record the
    exemption as tracked debt with a follow-up.

11. **Automate the write-path preservation and concurrency checks** (addresses:
    "No concurrent-write regression test"; "Perms/line-ending preservation only
    manual"; "Forced-SIGKILL invariant not ported"). Add an automated concurrent
    conditional-patch test (exactly-one-wins/one-412, no corruption), a CRLF +
    non-default-mode round-trip assertion over the new store path, and port the
    forced-kill sentinel synthesis + stale-temp cleanup with a test.

12. **Tighten the remaining orchestration and dependency details** (addresses:
    "Owner-PID grandparent heuristic"; "poll signal-0 not waitpid"; "thiserror
    1→2 not a no-op"; "License allowances unused after Phase 2"; "unix-only
    coupling"; "dispatch test under-committed"; "signature/hash rejection test").
    Specify the owner ancestor under exec-replace with a test; note stop polls
    `kill(pid,0)` and uses exact start_time equality; scope a thiserror-2 derive
    audit; re-prune `deny.toml` in Phase 2; note the unix-only coupling; commit to
    the black-box dispatch test per subcommand; and add a manifest tamper/rejection
    case.

---
*Review generated by /accelerator:review-plan*

## Per-Lens Results

### Architecture

**Summary**: Architecturally sound — collapses duplicated domain logic onto the
shared crates, keeps a clean sync-core/async-shell split behind `spawn_blocking`,
confines serde to thin wire views, and consolidates distribution/orchestration on
the unified launcher with correct dependency direction and green phasing. Two
structural concerns: the cut-over deletes the old fetch path before the live
manifest is guaranteed to carry `visualiser`, and the new member exits both the
workspace clippy policy and pup.ron governance.

**Strengths**:
- Clean functional-core/imperative-shell separation; sync domain in
  corpus/corpus-adapters/document, async I/O behind `spawn_blocking`.
- Serde confined to server wire view types over serde-free `corpus::DocTypeKey`.
- Sound dependency direction (downward, no cycles) retiring eight duplicated
  modules onto public shared APIs.
- Alias-free `visualiser` token precedent matching the launcher's
  `format!("{name}-{platform}")` derivation.
- Parity-first sequencing; four individually green/shippable phases.

**Findings**:
- 🟡 (major, medium) Old distribution path deleted before the live launcher
  manifest is guaranteed to carry the visualiser — Phase 4 / "What We're NOT
  Doing". Deletion/merge is gated only via the test assertion, not on 0165's live
  entry; a release before 0165 is a broken distribution with the failover removed
  first.
- 🟡 (major, high) New workspace member opts out of both the workspace lint policy
  and pup.ron governance — Phase 1 §2 / "What We're NOT Doing". A permanent
  governance asymmetry unless recorded as tracked debt with a follow-up.
- 🔵 (minor, medium) Doc-type inference strategy left as an implementer choice —
  Phase 2 §3. Commit to `corpus::doc_type::infer` for one shared inference
  semantics.
- 🔵 (suggestion, low) Config-resolution boundary moves into the server with
  parity asserted only for idle_timeout — Phase 3 §3. Extend assertions to the
  other resolved settings.

### Code Quality

**Summary**: A well-sequenced consolidation leaning on DI and a clean serde-free-
domain/wire-view split. Main risks: Phase 2 bundles more retirement than its
parity net covers, and the heavily-tested `config.rs` is reshaped without a
described end-state, leaving mixed responsibilities and hand-synced duplicate
constants the refactor was meant to remove. Several stale doc-comments and a
deferred "pick one" decision would land on the next maintainer.

**Strengths**:
- Strong DI posture (injected `IdScanner`, `AtomicWrite`/`FileCorpusStore` port,
  reused `FileDriver` trait).
- Domain/wire separation preserved deliberately.
- TDD-first: parity fixtures frozen before deletion; established seams reused.
- Phasing keeps the repo green; ~15.4k-line DRY win.

**Findings**:
- 🟡 (major, medium) Phase 2 change surface wider than its "parity-verified" net —
  Phase 2. Linkage/cluster and atomic-write re-homing land on ordinary confidence
  under a "parity-verified" label.
- 🟡 (major, medium) config.rs mixes schema/domain/resolution; end-state
  undescribed — Phase 2 §3 / Phase 3 §3.
- 🟡 (major, high) Hand-synced fallback constants become removable dead
  duplication after the config dependency lands — Phase 3 §3.
- 🔵 (minor, medium) Doc-comments/error-message sync notes referencing deleted
  scripts will go stale — Phase 4 §4.
- 🔵 (minor, medium) "Pick one" doc-type matcher understates a real design
  decision — Phase 2 §3.
- 🔵 (suggestion, low) async-over-sync spawn_blocking seam adds indirection worth
  documenting — Phase 2 §3 / Phase 3 §2.

### Test Coverage

**Summary**: Strong, deliberate discipline (golden fixtures frozen before
deletion, deterministic idle-timeout verification, independently-exercised
Host/Origin guards, reuse of hermetic seams). Main gaps: parity is scoped to three
surfaces while the equally-swapped patcher/typed-ref/linkage fall back to visual
spot-checks; the matcher swap and the spawn_blocking store swap have their
highest-risk edges only manually confirmed; and the version-coherence gate loses
two readers with no test update.

**Strengths**:
- Golden parity fixtures captured before deletion (correct ordering).
- Idle-timeout verified without sleeping.
- Host/Origin guards decomposed and exercised independently.
- Hermetic seams (RecordingExec, accelerator-fixture, MockServer with a runtime
  keypair that skips cleanly without minisign).
- Distribution verified against a local fixture, live assertion deferred to 0165.

**Findings**:
- 🟡 (major, medium) Parity fixtures omit patcher/typed-ref/linkage — Phase 2 §1.
- 🟡 (major, medium) doc-type matcher divergence not exercised by fixtures — Phase
  2 §3.
- 🟡 (major, medium) No concurrent-write regression test for the spawn_blocking
  store swap — Performance Considerations / Phase 2 §3.
- 🔵 (minor, medium) Perms/line-ending preservation only manual — Phase 2 Manual
  Verification.
- 🔵 (minor, medium) status stale/recycled-info-file case untested — Phase 3 §2.
- 🔵 (minor, medium) Version-coherence gate tests not updated — Phase 4 §2.
- 🔵 (minor, low) Dispatch test spy vs black-box under-committed — Phase 3 §5.

### Correctness

**Summary**: Well-sequenced and identifies most invariants, but conflates two
process identities in the stop recycle guard, understates the YAML-dialect
divergence of the engine swap, enumerates only happy-path status transitions
(omitting the stale state), and leaves the doc-type matcher swap unverified by
single-variant fixtures — each a latent logic error that passes the listed tests.

**Strengths**:
- Freezes parity fixtures before deletion.
- Requires owner-PID + start_time recycle refusal.
- Preserves the idle-disable sentinel and never/0/zero-length tokens.
- Requires Host-only and state-changing Origin-only 403s independently.
- Keeps the state/lock layout unchanged.

**Findings**:
- 🟡 (major, high) Recycle guard conflates the owner PID with the server's own
  PID — Phase 3 §2.
- 🟡 (major, medium) Parity fixtures omit YAML-dialect divergences — Phase 2 §1.
- 🟡 (major, medium) Status token transitions omit the crashed/self-shutdown stale
  state — Phase 3 §2.
- 🟡 (major, medium) infer vs kind_for_canonical_path unverified by parity
  fixtures — Phase 2 §3.
- 🟡 (major, medium) idle_timeout disable-token semantics depend on the raw string
  surviving compose — Phase 3 §3.
- 🔵 (minor, medium) Owner-PID grandparent heuristic may not survive the launcher
  exec-replace — Phase 3 §1.
- 🔵 (minor, medium) Detached daemon is not a child of stop; must poll signal-0,
  not waitpid; drop the shell's ±1s drift tolerance — Phase 3 §2.

### Security

**Summary**: Security-aware where it matters — loopback + Host/Origin 403 guards
carried across and independently exercised, recycle guard preserved, and the
cut-over strengthens integrity (sha256 + minisign vs sha256-only, fail-closed).
Two gaps: the blanket advisory-ignore instruction risks muting real RUSTSEC
advisories, and the release binary feature profile is unspecified, leaving open
the `dev-frontend` loopback/Host-guard bypass.

**Strengths**:
- Loopback + Host/Origin 403 model is an explicit criterion, guards exercised
  independently.
- Recycle guard preserved verbatim (SIGTERM → 2s → SIGKILL).
- Distribution strictly strengthens integrity; fails closed.
- serde-saphyr confinement enforced structurally as a Phase 2 criterion.

**Findings**:
- 🔴/🟡 (major, high) Blanket advisory-ignore instruction can silently suppress
  real RUSTSEC vulnerabilities — Phase 1 §3.
- 🟡 (major, medium) Release-staged binary feature profile unspecified — risks
  shipping the dev-frontend loopback bypass — Phase 4 §1.
- 🔵 (minor, high) Network-facing daemon exempted from the workspace unwrap/panic
  policy — Phase 1 §2.
- 🔵 (minor, medium) Local-manifest test asserts only the happy path, not
  signature/hash rejection — Phase 4 §3.

### Safety

**Summary**: For a dev-tooling plugin the dominant concern is operational
availability and the fidelity of the process-lifecycle safeguards being ported.
The plan preserves the key mechanisms, but its "independently mergeable" claim
hides a release coupling (binary resolution routes through a manifest that lacks
the entry until 0165), and the recycle-guard port and orphaned-server upgrade path
need tightening.

**Strengths**:
- Recycle guard (start_time match) explicitly preserved.
- Idle self-shutdown retained as a runaway bound.
- `process_start_time()` removes fragile locale-dependent `ps lstart` parsing.
- Shared write path fails safe (`PreserveOr`, byte-splice `patch_status`, symlink
  containment).
- Parity fixtures frozen before deletion; fail-closed 403 model verified.

**Findings**:
- 🔴/🟡 (major, high) Cut-over routes production binary resolution through a
  manifest that lacks the visualiser entry until 0165 — Phase 4 / Phase 3 §4 /
  Migration Notes.
- 🟡 (major, medium) Orphaned old-version server on upgrade has an unclear recovery
  path — Migration Notes.
- 🔵 (minor, medium) Recycle-guard identity must key on the server PID's
  start_time, not the owner-PID's — Phase 3 §2.
- 🔵 (minor, medium) Forced-SIGKILL post-shutdown invariant and temp-file cleanup
  not carried into the Rust port — Phase 3 §2.
- 🔵 (minor, low) Permission/line-ending preservation across the write rewiring is
  only manually spot-checked — Phase 2 Manual Verification.

### Compatibility

**Summary**: Unusually disciplined about contract stability (parity frozen before
deletion, kebab wire strings preserved by `corpus::wire_str`, back-compat config
keys retained). Dominant risks: cross-phase contract drift (version-inheritance vs
coherence-gate split; the release pipeline reads an absent literal in the interim)
and the frontmatter engine swap whose YAML-version coercion divergences reach the
frontend as JSON. A fixture-key rename also has an unlisted co-located test.

**Strengths**:
- Doc-type wire strings preserved (frontend JSON keys unchanged).
- Back-compat config keys retained (`config_path_key`).
- Manifest/asset/override key rename internally consistent on the `visualiser`
  token.
- TDD sequencing freezes parity before deletion.
- MSRV 1.85→1.90 safe (publish=false, prebuilt binary).

**Findings**:
- 🟡 (major, high) Version-inheritance and coherence-gate rewrite split across
  phases breaks the release pipeline in the interim — Phase 1 §2 vs Phase 4 §2.
- 🟡 (major, medium) serde_yml → serde-saphyr scalar coercion divergence reaches
  the frontend as JSON — Phase 2 §1-3.
- 🟡 (major, medium) Fixture key rename has an unlisted co-located unit-test
  consumer (`manifest.rs:146-148`) — Phase 4 §1.
- 🔵 (minor, medium) thiserror 1→2 major bump is not a no-op — Phase 1 §2 / Phase
  2 §2.
- 🔵 (minor, medium) License allowances added in Phase 1 for YAML deps go unused
  after Phase 2 — Phase 1 §3 vs Phase 2 §2.

### Portability

**Summary**: The unit move preserves the `../frontend/dist` embed and the plan
recognises the workspace-shared `cli/target/`. The weak spot is the set of
`CARGO_MANIFEST_DIR`-relative paths that escape the moving crate — only
`../frontend/dist` was audited, but several test files reach `../../../../skills/`,
`../../../../templates`, and `../scripts/`, which change depth on the move. The new
Rust orchestration is unix-only (acceptable for the darwin/linux closure) but the
coupling should be explicit.

**Strengths**:
- server+frontend move as a unit; embed literals verified.
- Workspace-shared target dir recognised (E2E `SERVER_BIN`, cross-compile source
  repointed).
- Manifest key / asset / override consistent on the bare token; platform closure
  matches the launcher.
- Retiring `config.json` for a runtime `.accelerator/*.md` read improves config
  externalisation.

**Findings**:
- 🔴/🟡 (major, high) Repo-relative test paths that escape the crate break on
  relocation — Phase 1 §1 / Key Discoveries.
- 🔵 (minor, high) Crate-local `target/` derivations must switch to the workspace-
  shared `cli/target/`; `dev.py:43` unlisted — Phase 1 §4.
- 🔵 (minor, medium) Hardcoded `skills/visualisation/visualise` path inventory is
  incomplete — Phase 1 §4 / Phase 4 §4.
- 🔵 (minor, low) New Rust orchestration path is unix-only by construction — make
  the coupling explicit — Phase 3 §2.

## Re-Review (Pass 2) — 2026-07-23

**Verdict:** REVISE

The revision is a substantial improvement. Of the 21 original majors,
**~18 are resolved** and both governance tradeoffs are settled the way the
author chose (lint opt-in for request/SSE paths; pup.ron as tracked debt). The
three load-bearing structural fixes were verified against the code by the
re-review agents: the recycle guard now correctly keys on the **server** pid's
`start_time` (the server writes its own pid+`start_time` via
`process_start_time`, matching `stop_server_stop`); the coherence-gate reader
removal is correctly co-located with version inheritance in Phase 1
(`_read_cargo_toml_version` would otherwise parse `{workspace: true}` and always
mismatch); and the distribution cut-over is now a properly release-gated
expand-then-contract across Phases 4→5.

The newly-detailed plan surfaced a fresh crop of issues — mostly second-order
refinements, but ~10 rise to major, so the verdict stays REVISE. Notably, two
are corrections to edits made in this review pass (the empty-string idle token
and the asset-rename collision), and one high-confidence Code-Quality finding
shows the config re-home is still under-specified.

### Cross-Cutting Themes (this pass)

- **Manual gates want automated enforcement** (flagged by: architecture, safety,
  security, test-coverage) — Two safety-critical invariants now rest on prose +
  human discipline: the Phase 3↔4 co-release gate (SKILL switch must not ship
  without `DISPATCHED_SUBBINARIES`), and the release feature-profile constraint
  (staged binary must exclude `dev-frontend`/the insecure switch). Both are
  fixable with a CI coherence assertion / a symbol-grep of the staged artifact.
- **doc-type divergence goldens are self-contradictory** (flagged by:
  test-coverage [high], correctness) — Capturing goldens from the old
  `kind_for_canonical_path` (which is `HashMap`-order first-match, i.e.
  non-deterministic for overlapping roots) conflicts with asserting field-equality
  against the deterministic `infer`. Split the doc-type fixtures into pure-parity
  paths (assert equality to old output) and divergence paths (assert the intended
  new `infer` result, hand-authored).
- **config re-home is under-specified** (flagged by: code-quality [high],
  correctness, safety) — "Thin adapter" understates the composition layer: the
  retired `write-visualiser-config.sh` also resolves `ACCELERATOR_VISUALISER_*`
  env precedence, template override-source, and a `tickets`→`work` migration
  guard, none enumerated; and the empty-string idle token is described as
  "keep the server up" while the resolver actually rejects it at boot
  (`InvalidIdleTimeout`) — only an *absent* key defaults to 8h.
- **Asset-rename ripple** (flagged by: compatibility, 2 majors) — Renaming the
  staged-asset const to `visualiser-{platform}` in Phase 4 (a) ripples to the
  workflow provenance glob and Python task tests not in the change set, and
  (b) collides with `cli_binary_path`'s identically-named manifest-flow asset
  during the pre-Phase-5 window (two uploads, same filename).

### Previously Identified Issues

- 🟡 **Architecture**: Old distribution path deleted before manifest carries
  visualiser — **Resolved** (restructured into release-gated Phase 5;
  expand-then-contract praised).
- 🟡 **Architecture**: Workspace member opts out of lint + pup.ron — **Resolved**
  (request/SSE paths opted into `unwrap_used`/`panic`; pup.ron → tracked debt,
  downgraded to minor).
- 🟡 **Code Quality**: Phase 2 surface wider than parity net — **Resolved**
  (parity net widened to patcher/linkage/dialect/matcher).
- 🟡 **Code Quality**: config.rs end-state undescribed — **Partially resolved**
  ("thin adapter" described but understates the composition layer — see new
  findings).
- 🟡 **Code Quality**: Hand-synced fallback constants — **Resolved** (removal
  scheduled, sourced from catalogue).
- 🟡 **Test Coverage**: Parity omits patcher/typed-ref/linkage — **Resolved**.
- 🟡 **Test Coverage**: doc-type matcher divergence not exercised — **Partially
  resolved** (now exercised, but golden-capture method contradicts the
  assertion — see new findings).
- 🟡 **Test Coverage**: No concurrent-write regression test — **Partially
  resolved** (test added, but lacks a deterministic interleaving seam).
- 🟡 **Correctness**: Recycle guard conflates owner/server PID — **Resolved**
  (verified against code: keys on server pid).
- 🟡 **Correctness**: Parity omits YAML dialect — **Resolved** (JSON value-type
  assertions added).
- 🟡 **Correctness**: Status omits stale state — **Resolved**.
- 🟡 **Correctness**: infer vs kind_for_canonical_path unverified — **Resolved**
  (pinned by fixtures; golden-capture caveat noted below).
- 🟡 **Correctness**: idle_timeout disable-token semantics — **Partially
  resolved** (raw-token contract added, but the empty-string case now
  contradicts the resolver — see new findings).
- 🟡 **Security**: Blanket advisory-ignore — **Resolved** (per-entry
  justification + re-prune).
- 🟡 **Security**: Release binary feature profile — **Partially resolved**
  (requirement added, but manual verification only).
- 🟡 **Safety**: Cut-over routes through entry-less manifest — **Resolved**
  (Phase 5 gate).
- 🟡 **Safety**: Orphaned old-version server recovery — **Mostly resolved**
  (reap test + idle fallback added; `never`/0 case and a start_time
  misattribution remain — minor).
- 🟡 **Compatibility**: Version-inheritance vs coherence split — **Resolved**
  (moved to Phase 1; verified correct).
- 🟡 **Compatibility**: scalar coercion reaches frontend — **Resolved**
  (JSON value-type parity).
- 🟡 **Compatibility**: Fixture key rename unlisted test consumer — **Resolved**
  (`manifest.rs:146-148` update added).
- 🟡 **Portability**: Repo-relative test paths break on relocation — **Resolved**
  (inventory expanded with depth-adjustment rule).

### New Issues Introduced

- 🟡 (major, high) **Correctness/Code-Quality**: config.rs "thin adapter"
  understates a real composition/resolution layer — `ACCELERATOR_VISUALISER_*`
  env precedence, template override-source, and the `tickets`→`work` migration
  guard are unmentioned and at risk of being silently dropped — Phase 3 §3.
- 🟡 (major, high) **Correctness**: empty-string `idle_timeout` self-contradiction
  — listed as a "keep the server up" disable token, but the resolver rejects it
  at boot (`InvalidIdleTimeout`); only an *absent* key → 8h — Phase 3 §3/§5,
  Success Criteria, Testing Strategy.
- 🟡 (major, high) **Test-Coverage/Correctness**: matcher-divergence goldens
  contradict the field-equality parity model — the old matcher is `HashMap`-order
  non-deterministic; split fixtures into parity vs intended-new — Phase 2 §1/§3.
- 🟡 (major, medium) **Compatibility**: asset-rename to `visualiser-{platform}`
  ripples to the workflow provenance glob and Python task tests outside the
  change set — Phase 4 §1.
- 🟡 (major, medium) **Compatibility**: duplicate GitHub-Release asset name
  during the pre-Phase-5 window (old-flow const + `cli_binary_path` both emit
  `visualiser-{platform}`) — Phase 4 §1. (Fix: leave the old-flow const emitting
  `accelerator-visualiser-{platform}` until Phase 5.)
- 🟡 (major, medium) **Architecture/Safety/Security**: the Phase 3↔4 co-release
  gate and the release feature-profile constraint are enforced only by prose /
  manual verification — add a CI coherence assertion and a staged-artifact
  symbol-grep — Overview, Phase 3 §4, Phase 4 §1, Phase 5 gate.
- 🟡 (major, medium) **Test-Coverage**: concurrent-conditional-patch test lacks a
  deterministic interleaving seam (risks tautology or flakiness) — Phase 2 §5.
- 🟡 (major, medium) **Test-Coverage**: orphan-reap start_time reconciliation test
  lives only in Migration Notes prose, not any phase's success criteria (and the
  old shell path it needs is deleted in Phase 5) — Migration Notes / Phase 3.
- 🟡 (major, medium) **Test-Coverage**: lifecycle integration tests spawn real
  detached daemons with no stated isolation contract (unique tempdir, port 0,
  teardown reaping) — Phase 3 §2.
- 🔵 (minor, medium) **Correctness**: Migration Notes misattribute the server's
  `start_time` to locale-sensitive `ps lstart` — it was always written by the
  Rust server via `process_start_time`, so cross-upgrade reap matches by
  construction — Migration Notes.
- 🔵 (minor, medium) **Security**: preserved Origin guard uses a loose
  `starts_with("http://127.0.0.1")` prefix match, exploitable via
  `127.0.0.1.evil.com` — tighten to exact host parse — Phase 3.
- 🔵 (minor, medium) **Safety**: the `launch-server.sh` init-sentinel precondition
  (refuse to launch in an uninitialised repo) is not enumerated in the Rust
  `start` port — Phase 3 §2.
- 🔵 (minor, medium) **Portability**: the `start` lock port is unspecified; the
  shell's `mkdir` fallback exists because macOS lacks the `flock` *binary* — use
  the `flock(2)` syscall in-process — Phase 3 §2.
- 🔵 (minor, low) **Test-Coverage/Security**: the "insecure switch inert" release
  invariant has manual verification only — add an automated symbol-grep guard —
  Phase 4 §1.

### Assessment

The plan is materially stronger and structurally sound: the destructive and
release-ordering hazards are now gated, the correctness fixes verify against the
code, and the parity/test net covers the surfaces that matter. It is not yet
approve-ready — the config-composition under-specification, the empty-string
idle contradiction, the doc-type golden-capture contradiction, and the
asset-rename collision are concrete, addressable items (several are quick), and
the recurring "automate the manual gate" theme is worth one CI assertion. A
focused third pass on these would likely reach APPROVE.

## Re-Review (Pass 3) — 2026-07-23

**Verdict:** REVISE (converging — remaining items are implementation-grade)

Every pass-2 major was addressed, and two verifier agents independently
**confirmed the revised claims against the code**: `resolve_idle_limit_ms`
genuinely separates absent-key→`8h`, disable-tokens→disabled, and
empty/whitespace→`InvalidIdleTimeout` (the whitespace rejection is already
tested); and `server-info.json`'s `start_time` is written by the Rust server via
`process_start_time`, so the exact-match cross-upgrade reap holds by
construction. This pass surfaced a final, finer layer — three of which were
corrections to pass-2 edits (a `0ml` typo, a wrong test path, and an inaccurate
"write coordinator serialises writes" claim), now fixed. The remaining items are
implementation-grade detail rather than structural risk; the verdict stays
REVISE only because the count still exceeds threshold, but the plan is
approve-adjacent.

### Cross-Cutting Themes (this pass)

- **config.rs composition depth** (flagged by: code-quality [high],
  test-coverage [high], architecture) — The composition layer needed a
  decompose-by-concern instruction (it had none, unlike `orchestration/`), a
  re-homed composition contract test (the retiring `config_contract.rs` pinned
  13 doc-paths + template tiers; only idle_timeout had replacement coverage),
  `config_override_source` derived from the resolved `Source` (not hand-rolled
  frontmatter scanning), and the runtime-derived fields (`plugin_root` etc.)
  named. All now added.
- **Tests must drive the real critical section** (flagged by: test-coverage) —
  The concurrency seam rested on a false premise (`write_coordinator.rs` is a
  dedup cache, not a serialiser); it must anchor on the production
  verify-then-write path with a mutation check. The symlink/writable-root
  containment guard also needed an explicit carry-forward test. Both corrected.
- **Producer-contract ripple is more than a stem rename** (flagged by:
  compatibility [high]) — The `DISPATCHED_SUBBINARIES` flip changes the release
  **upload count**, breaking a fixed-count assertion in
  `tests/integration/tasks/test_github.py` (not `unit/`, as pass-2 stated); a
  stem-only grep misses the count literal. Path and count now called out.
- **Automated controls need the right target** (flagged by: security, safety,
  architecture) — The feature-profile grep must match the exact
  `ACCELERATOR_VISUALISER_E2E_INSECURE` string (Phase 3 now embeds other
  `ACCELERATOR_VISUALISER_*` literals) and run on the final signed artifact; the
  Origin guard must use a real URL parser (not `split_once(':')`, which
  `…:x@evil.com` defeats). Both tightened.

### Previously Identified Issues (pass-2 new findings)

- 🟡 **Code-Quality/Correctness**: config.rs "thin adapter" understates the
  composition layer — **Resolved** (decompose-by-concern + full enumeration
  incl. env fields and precondition placement).
- 🟡 **Correctness**: empty-string idle contradiction — **Resolved** (three
  classes stated: absent→8h, disable→up, empty→`InvalidIdleTimeout`); verifier
  confirmed against `config.rs`. The `0ml` typo introduced in the fix is
  **corrected** to `0ms`.
- 🟡 **Test-Coverage/Correctness**: matcher-divergence golden contradiction —
  **Resolved** (split into pure-parity vs hand-authored divergence classes).
- 🟡 **Compatibility**: asset-rename ripple — **Resolved and extended** (path
  corrected to `integration/`, upload-count assertion + fixture called out).
- 🟡 **Compatibility**: duplicate asset collision — **Resolved** (old-flow const
  left unrenamed until Phase 5; verifier confirmed the two filenames stay
  distinct).
- 🟡 **Architecture/Safety/Security**: co-release + feature-profile gates
  unenforced — **Resolved** (automated coherence check + artifact symbol-grep).
- 🟡 **Test-Coverage**: concurrent-patch interleaving seam — **Resolved and
  corrected** (false "serialises" premise fixed; seam anchored on the real path
  with a mutation check).
- 🟡 **Test-Coverage**: orphan-reap test not pinned — **Resolved** (promoted to
  a Phase 3 success criterion, incl. the `never`/0 case).
- 🟡 **Test-Coverage**: lifecycle isolation contract — **Resolved** (now
  specifies an RAII/`Drop` reaper that fires on panic).
- 🔵 **Correctness**: Migration `start_time` misattribution — **Resolved**
  (corrected; verifier confirmed).
- 🔵 **Security**: loose Origin prefix match — **Resolved** (robust URL parse,
  lookalike + userinfo test cases).
- 🔵 **Safety**: init-sentinel precondition — **Resolved** (preserved in the
  Rust `start` port).
- 🔵 **Portability**: `start` lock port unspecified — **Resolved** (`flock(2)`
  syscall in-process, with a local-FS assumption note).

### New Issues Introduced

- 🟡 (major, medium) **Test-Coverage**: no test that the writable-root /
  symlink-escape containment guard survives the `FileCorpusStore` write swap —
  **now addressed** in Phase 2 §5 (path-safety regression test) this pass.
- 🔵 (minor, medium) **Correctness**: the YAML-null spelling (`idle_timeout:`) vs
  empty-string vs absent distinction should be resolved explicitly, not just
  "confirm the resolver does not normalise" — **now addressed** in Phase 3 §3.
- 🔵 (minor, medium) **Code-Quality**: `config_override_source` should derive
  from the resolved `Source`, not hand-rolled frontmatter scanning — **now
  addressed**.
- 🔵 (minor, medium) **Safety**: stale-temp cleanup rationale misattributed to a
  visualiser-specific temp prefix (the `.tmp-` prefix is shared) — **now
  corrected** to state-dir scoping.
- 🔵 (minor, low) **Test-Coverage/Compatibility**: graceful-degradation contract
  for a document the stricter engine now rejects — **now addressed** (indexer
  skips + surfaces per-doc error; integration test).
- 🔵 (minor, medium) **Architecture**: owner-PID ancestor-walking is coupled to
  Claude Code's spawn topology; an injected-ancestor unit test can't catch a
  real mismatch — **now addressed** (prefer explicit owner-PID passing; e2e
  chain check if ancestry-walking is kept). *This remains the one item most
  worth an implementer's early validation.*
- 🔵 (minor, low) **Portability**: non-unix orchestration gap should be tracked
  debt, not an inline aside — **now addressed** (follow-up story, mirroring
  pup.ron).

### Assessment

The plan is thorough, structurally sound, and now carries verified,
code-checked guidance on its highest-risk mechanisms (recycle guard, idle
resolution, distribution ordering, engine-swap parity). Across three passes it
went from 21 structural majors → ~10 refinement majors → this pass's
implementation-grade detail, all applied. The single item I'd flag for early
validation during implementation is the **owner-PID identity under launcher
exec-replace** — it depends on Claude Code's spawn topology and is best proven
with a real end-to-end spawn rather than a unit seam. Everything else is
captured in the plan's success criteria. I'd treat this as **ready to implement**
with that one spike up front; a further review pass would yield diminishing
returns.

**Verdict accepted as APPROVE by the reviewer**: the three review passes drove
the plan from 21 structural majors to fully-applied implementation-grade detail,
with the highest-risk mechanisms verified against the code. The residual items
are captured in the plan's success criteria; the owner-PID-under-exec-replace
identity is flagged for an early implementation spike. The plan is ready to
implement.
