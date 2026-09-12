---
type: "plan-review"
id: "2026-09-11-0216-close-the-sha2-hardware-intrinsics-gap-review-1"
title: "Plan Review: Close the sha2 hardware-intrinsics gap"
date: "2026-09-11T08:03:02+00:00"
author: "Toby Clemson"
producer: "review-plan"
status: "complete"
parent: "plan:2026-09-11-0216-close-the-sha2-hardware-intrinsics-gap"
target: "plan:2026-09-11-0216-close-the-sha2-hardware-intrinsics-gap"
reviewer: "Toby Clemson <toby@go-atomic.io>"
verdict: "APPROVE"
lenses: ["correctness", "portability", "performance", "compatibility", "security", "architecture", "test-coverage", "code-quality"]
review_number: 1
review_pass: 2
tags: ["cli", "launcher", "performance"]
last_updated: "2026-09-11T10:06:20+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

## Plan Review: Close the sha2 hardware-intrinsics gap

**Verdict:** REVISE

The plan is unusually well-grounded — its factual claims about the pins, the
`deny.toml` posture, the harness `asset_bytes` line, and the phase-ordering
rationale all check out against the live tree, and the throughput arithmetic is
internally consistent. But one critical defect falsifies its load-bearing
premise: `sha2` 0.11 is a **major** RustCrypto bump that swaps the digest output
type from `generic-array` to `hybrid-array`, dropping the `LowerHex` impl three
first-party `format!("{:x}", …)` sites depend on — so the plan's "no source
changes, the six sites inherit the backend for free" claim does not compile.
Around that sit a cluster of majors: the musl backend-selection evidence never
executes on musl (wrong predicate, wrong target), the AC 7 fallback can't catch
the realistic silent-soft-backend degradation, two `cargo` verification commands
misbehave, and the Phase 4 deny flip is under-specified on advisories/licenses
and over-broad on `skip-tree`.

### Cross-Cutting Themes

- **A major-version bump treated as a transparent drop-in** (flagged by:
  compatibility, correctness, test-coverage) — the plan's premise that only the
  dependency declaration changes is wrong at the source level (`{:x}` break),
  the test level (no multi-block KAT on the gated function), and the substack
  level (the plan mischaracterises pre-existing 0.10/0.11 straddles as newly
  added).
- **musl is asserted, never exercised** (flagged by: portability, correctness,
  performance) — every AC 3 evidence prong (no-forced-flag inspection, `.rlib`
  symbol dump, darwin runtime print) sidesteps the one fragile mechanism:
  getauxval/HWCAP feature detection inside a `crt-static` musl binary on aarch64.
  The darwin print exercises macOS `sysctlbyname`, and the predicate
  (`std::arch`) differs from the shipped selector (`cpufeatures`).
- **The Phase 4 deny flip weakens the gate it strengthens** (flagged by:
  security, architecture, test-coverage) — unbounded `skip-tree` masks whole
  subtrees, the new skips carry no `review-by:` audit (unlike the advisory
  ignores), and the advisories/licenses/sources surface the new substack shifts
  is never re-vetted.
- **The plan's own verification gates are unsound** (flagged by: correctness,
  compatibility) — `cargo tree -d -i sha2` prints empty on success; `cargo
  update --package sha2 --precise` is ambiguous while two versions coexist.

### Tradeoff Analysis

- **Supply-chain strictness vs maintenance coupling** (security vs
  architecture): security wants the `deny` flip and narrow skips to catch new
  duplicates on the signed-binary closure; architecture notes the same flip
  couples routine `cargo update` churn to CI and accretes unaudited carve-outs.
  Recommendation — keep `deny`, but prefer versioned `skip` over `skip-tree`
  (bounded `depth` where a tree is unavoidable) and give the straddle skips
  `review-by:` dates so the carve-out set stays auditable. This satisfies both.
- **Verification cost vs conclusiveness on musl** (portability/correctness vs
  scope): fully proving musl runtime selection means emulated aarch64 execution
  (QEMU) inside 0216, which the plan deliberately hands to 0217. Recommendation —
  do not import 0217's throughput scope, but downgrade AC 3c's darwin print to
  corroborating-only and make "soft backend silently selected on musl" a
  first-class 0216 failure condition, so the gap is honestly bounded rather than
  overclaimed.

### Findings

#### Critical

- 🔴 **Compatibility**: `sha2` 0.11 removes `LowerHex` from the digest output — three `{:x}` call sites break the build
  **Location**: Phase 3 (Test-Driven Development; Success Criteria: `mise run test` / `cli:check`)
  0.11 migrates `Output<T>` from `generic-array::GenericArray` to
  `hybrid-array::Array`, which implements no hex-formatting traits, so
  `format!("{:x}", hasher.finalize())` fails to compile at
  `cli/work-adapters/tests/corpus_hashes.rs:27`,
  `cli/work-adapters/tests/bash_parity_baseline.rs:98`, and
  `cli/jira-client/tests/adf_oracle_manifest.rs:33`. All three sites and the
  absence of a `hex` dependency in those two crates were verified directly.
  Phase 3's own gates fail; the "dependency-declaration change only" premise is
  false.

#### Major

- 🟡 **Compatibility**: a major RustCrypto bump is assumed API-transparent without a per-consumer audit
  **Location**: Overview; What We're NOT Doing ("the six non-gated sites inherit the crate-global backend")
  The plan enumerates seven launcher/server sites and declares zero code
  changes, but the actually-breaking usage lives in `work-adapters`/`jira-client`
  consumers the enumeration omits. A `sha2` 0.10→0.11 API audit across all five
  inheriting crates is missing.

- 🟡 **Portability / Correctness**: AC 3's musl backend-selection evidence is inconclusive — wrong predicate, wrong target, never run on musl
  **Location**: Phase 3, evidence steps 3–5 (AC 3a/3b/3c)
  The no-forced-flag inspection is trivially true and says nothing about
  selection; the `.rlib` symbol dump proves *compiled-in*, not *selected* (the
  soft backend is in the same rlib); the darwin `std::arch::is_aarch64_feature_detected!`
  print exercises macOS `sysctlbyname`, not the Linux getauxval/HWCAP path a
  `crt-static` musl binary uses — and `std::arch` is not the `cpufeatures`
  selector `sha2` 0.11 actually ships. Nothing confirms hardware selection on
  aarch64-musl.

- 🟡 **Portability**: the AC 7 fallback triggers only on build/link failure, missing the realistic silent soft-backend degradation
  **Location**: Phase 3, Fallback (AC 7)
  With detection runtime-gated and no forced `+sha2`, the probable musl failure
  is `cpufeatures` returning `false` and silently selecting the ~555 MB/s soft
  backend — which builds clean, links clean, passes the `.rlib` check and the
  darwin print. 0216 closes green while a shipped target misses the whole point
  of the work item, invisible until 0217 measures it.

- 🟡 **Security**: the new `sha2` 0.11 substack is not re-vetted for advisories / licenses / sources
  **Location**: Phase 4
  The bump pulls a new RustCrypto major substack (`digest`, `crypto-common`,
  `block-buffer`, `cpufeatures 0.3`, `hybrid-array`) into the signed-binary
  closure, but Phase 4 reasons only about `[bans]`. `deny:check` runs all four
  checks and `[licenses].allow` is pruned to exactly the current closure (it
  warns on an unused allowance), so a new advisory or license would surface as a
  surprise failure with no planned disposition — and vulnerability-class
  advisories may never be `ignore`d on this closure.

- 🟡 **Security / Test-Coverage / Architecture**: unbounded `skip-tree` weakens the very gate the `deny` flip adds
  **Location**: Phase 4, Section 1
  `skip-tree` on the rand/getrandom and syn 1/2 ecosystems, without a `depth`
  bound or version pin, masks all duplicates under those subtrees — so a future
  first-party-collapsible duplicate there is silently accepted, defeating the
  manual criterion "no first-party-collapsible duplicate was skipped rather than
  resolved."

- 🟡 **Architecture**: the hard `deny` gate plus a permanent, unaudited skip-list couples routine dependency churn to CI
  **Location**: Phase 4; Migration Notes
  Flipping `multiple-versions` to `deny` with a ~17-entry skip list — mostly
  transitive straddles no first-party change can collapse — makes a routine
  `cargo update` (or `rust-embed` moving off 0.11.0, which the plan itself flags)
  break the build. The `deny`-vs-`warn` tradeoff is never weighed, and unlike the
  advisory `ignore`s the new skips carry no test-enforced `review-by:` date.

- 🟡 **Correctness**: `cargo tree -d … -i sha2` prints empty on the *correct* outcome — a false-negative gate
  **Location**: Phase 3, Success Criteria; Desired End State
  `-d`/`--duplicates` restricts output to crates with multiple versions, so once
  `sha2` collapses to a single 0.11.0 it no longer qualifies and the command
  prints nothing — not "only 0.11.0". Split into: `sha2` absent from `cargo tree
  -d` (no duplicate) plus `cargo tree -i sha2` (without `-d`) showing the single
  version.

- 🟡 **Correctness / Compatibility**: `cargo update --package sha2 --precise 0.11.0` is ambiguous while two `sha2` versions coexist
  **Location**: Phase 3, Section 3
  With both `sha2 0.10.9` and `0.11.0` still in the lock, the bare spec `sha2`
  matches two packages and the command can error. Disambiguate with
  `sha2@0.10.9`, or rely on a plain re-resolve after the manifest edits. A
  desynced lock makes `cli:check` (clippy `--locked`) fail with an
  unrelated-looking error, undermining "Phase 3 leaves the tree green."

- 🟡 **Test-Coverage / Security**: the AC-gated `sha256_hex` is covered only by an empty-input vector
  **Location**: Phase 3, Test-Driven Development (`verifier.rs:74`)
  `sha256_hex_matches_a_known_vector` tests only `b""` — a single padding block
  that never drives the multi-block compression loop the ARMv8 intrinsic backend
  replaces. Regression safety rests on distant crate-global tests
  (`corpus_hashes.rs`) the plan does not credit. Add cheap KATs to `verifier.rs`
  (the `"abc"` vector, a >64-byte input, a 55/56-byte padding boundary, and a
  streaming-vs-one-shot equivalence check).

- 🟡 **Test-Coverage**: the `asset_bytes` threading into the report (AC 6) is only manually verified
  **Location**: Phase 1, Test-Driven Development
  The three planned tests stop at the isolated parser; the actual deliverable —
  `asset_bytes` landing in the report dict via `LauncherTerms` and
  `close_the_budget` — is checked only by a manual checkbox. Nothing catches a
  refactor that drops or mis-keys the field. Extract the report-dict assembly and
  unit-test that `asset_bytes` lands alongside `cache_root_bytes`, including the
  `None` case.

#### Minor

- 🔵 **Code-Quality**: the plan references a non-existent function `residual_and_terms`
  **Location**: Phase 1, Section 1; References
  Verified: no such symbol exists; the report assembler is `close_the_budget`
  (`tasks/measure.py:1858`, called at `:1697`). Replace every `residual_and_terms`
  reference (the line numbers are already right).

- 🔵 **Code-Quality**: "top-level field, sibling to `cache_root_bytes`" contradicts where the field lands
  **Location**: Phase 1, Section 1; Manual Verification
  Verified: `cache_root_bytes` lives inside the dict `close_the_budget` returns,
  which is assigned to `record["terms"]` (`:1697`), so a sibling of it nests at
  `record["terms"].asset_bytes`, not the record top level. The pseudocode is
  right; the prose and the manual-check wording mislead.

- 🔵 **Correctness**: the RustCrypto substack straddles pre-exist the bump rather than being added by it
  **Location**: Current State Analysis (Key Discoveries); Phase 3, Section 3 note
  The lock already carries `cpufeatures 0.2.17`+`0.3.0` and `digest 0.10.7`+`0.11.3`,
  pulled by `sha1`/`hmac`/`blake2`/`jj-lib` — not first-party `sha2`. The bump
  *repoints* first-party consumers onto the already-present 0.11 substack; it adds
  no new straddle. Reword so Phase 4's justification is accurate.

- 🔵 **Test-Coverage**: the parser fixture is untied to the harness, disagrees with an existing fixture, and the harness is `#[ignore]`d
  **Location**: Phase 1, Test-Driven Development
  The fixture value `2493792` disagrees with the committed `2493376` at
  `test_measure.py:1882`; nothing ties the hand-authored shape to `warm_terms.rs:151`;
  and the emitting harness never runs in CI, so a Rust-side format change makes
  `parse_asset_bytes` silently return `None`. Share one fixture constant and
  reconcile the byte count.

- 🔵 **Test-Coverage**: parser error/edge cases unspecified, and one of the three proposed tests already exists
  **Location**: Phase 1, Test-Driven Development
  Missing: malformed JSON, non-integer value, and multiple `asset_bytes` lines
  (where `parse_asset_bytes` returns the *first* but `parse_term_report` keeps the
  *last* — an untested asymmetry). The third proposed test already exists as
  `test_non_term_lines_are_ignored_rather_than_parsed` (`test_measure.py:1892`).

- 🔵 **Portability**: the "musl-safe" claim depends on an unstated zig-bundled musl version providing getauxval
  **Location**: Overview / Phase 3 ("musl-safe under crt-static")
  The static links zig's bundled musl (`ziglang>=0.16.0`); `cpufeatures`' aarch64
  detection needs `getauxval`, which musl gained only in 1.1.21. Record the
  effective musl version and the getauxval floor as an explicit AC 3 precondition.

- 🔵 **Performance**: AC 1 gates on a fixed latency, not throughput — equivalence breaks if the asset size drifts
  **Location**: Overview AC table (AC 1); Phase 3, item 1
  ≤1.79 ms equals ≥1,390 MB/s only when `asset_bytes` is exactly 2,493,792. The
  harness times whatever cached `vcs` exists, whose size tracks the plugin
  version. Make the derived MB/s the primary gate, or assert `asset_bytes` equals
  the assumed size in both committed records.

- 🔵 **Performance**: the single-run median-only gate leaves the captured p97.5 band and harness validity verdicts unused
  **Location**: Phase 3, item 1; Testing Strategy
  One 200-sample median per side, gated on the median alone; the recorded
  baselines show loadavg ~3.8–4.0. Require `analysis.validity: valid` on both
  records and assert the after-run p97.5 also clears 1.79 ms.

- 🔵 **Portability**: the before/after evidence and the fixed 1.79 ms gate are calibrated to one Apple chip
  **Location**: Phase 2 / Phase 3 ("quiet darwin-arm64 host")
  Derived from a single M4 Max; the "~4.3 ms before" validity check and the fixed
  gate can pass/fail spuriously on M1/M2/M3 or a noisier host. Record the exact
  CPU model in the committed JSON and note the gate is meaningful only on an
  M4-Max-class quiet host.

- 🔵 **Security**: the 0215 fallback pulls the cache-hit name/version binding into scope without restating its integrity constraint
  **Location**: Phase 3, Fallback (AC 7)
  0215 warns that the cache-hit SHA-256 recompute is the only thing binding a
  cached `{name}-{version}` filename to its content (minisign signs bytes, not
  name/version). The fallback frames 0215 as a pure performance route; have it
  defer explicitly to 0215's "filename-disagrees-with-content is still rejected"
  criterion and reaffirm minisign stays unchanged.

- 🔵 **Architecture**: the orthogonal workspace-wide deny cleanup is bundled into a performance work item
  **Location**: Overview; Implementation Approach; Phase 4
  The work item's own Drafting Notes call the cleanup "orthogonal … a
  delivery-sequencing convenience." The sequencing argument only requires Phase 4
  to run *after* the bump, not to live in the same plan. Either extract it or
  strengthen the justification beyond "convenience."

- 🔵 **Architecture**: Phase 4's "independently mergeable" claim understates its content coupling to Phase 3
  **Location**: Overview; Phase 4
  Phase 4's whole deliverable (the skip list) is determined by the post-bump
  graph Phase 3 produces — a downstream, content-coupled increment, not an
  independent one. Reword to "a self-contained mergeable increment in strict
  sequence" and note it must be re-derived if the bump changes.

- 🔵 **Architecture**: the cross-work-item reciprocal-edge guarantee rests on a one-time manual check
  **Location**: Phase 3, step 8; What We're NOT Doing
  0217 currently carries the reciprocal `work-item:0216` edge (verified), but the
  repo has no referential-integrity check, so a later edit to 0217 silently
  breaks the deferred-evidence chain. Record the confirmation as a dated evidence
  note on both items.

#### Suggestions

- 🔵 **Performance**: note that `calibration.holds` is false on the recorded baselines (it bears on shell-floor terms, not the direct `sha256_hex` timing), so ~4.3 ms is a soft sanity band and the authoritative evidence is the fresh same-host before/after delta.
- 🔵 **Performance**: state the throughput claim is darwin-one-shot-only, and that the 64 KiB streaming path amortises across ~1,024 blocks per chunk so no chunk-size/intrinsics interaction is expected — making "ride for free" an expectation, not a measured result.
- 🔵 **Code-Quality**: name `LauncherTerms.terms` rather than `.intervals` so the file's "terms" vocabulary carries through to the merge site.
- 🔵 **Code-Quality**: optionally extract a `json_report_lines(stdout)` generator shared by both parsers, or consciously keep two tiny passes (KISS).
- 🔵 **Code-Quality**: the README formula is 81 columns and uses non-ASCII `÷`/`×`; rewrap under 80 and prefer ASCII `/` and `*` (a fenced `text` block is not auto-reflowed).
- 🔵 **Correctness**: state that the authoritative straddle-disposition list is `cargo deny check bans` across all five targets, and the work item's 18-crate set is a host-scoped lower bound, not a ceiling.
- 🔵 **Test-Coverage**: when Phase 4 lands, update the stale "warn-level policy" rationale comment in `tests/integration/deny/test_vcs_library_graph.py:14`.
- 🔵 **Test-Coverage**: specify the AC 3c throwaway detector lives outside the cargo workspace (or a stash-only edit) so nothing has to catch its removal.
- 🔵 **Architecture**: align the Phase 2 artefact naming with the existing `warm-dispatch-N.json` convention (or document the new `<date>-<workitem>-<name>` scheme), and reconsider whether the freeform warning dump needs to be a durable committed file.

### Strengths

- ✅ The plan is exceptionally well-grounded: the pins, the `deny.toml` `warn`
  posture and five-target graph, the harness `asset_bytes` line, the
  `parse_term_report` `"term"`-only filter, and the KAT test location all verify
  against the live tree.
- ✅ Single source of truth is achieved completely: converting the server's
  independent `sha2 = "0.10"` literal to `{ workspace = true }` routes every
  consumer through one declaration, with no crate left diverging.
- ✅ The four-phase ordering is a clean, well-argued dependency DAG:
  plumbing → recoverable baseline → bump → post-bump-only deny inventory, with
  state handed off through committed artefacts rather than conversation.
- ✅ The throughput arithmetic is internally consistent across plan, work item,
  and research (2,493,792 ÷ (1.79 × 1000) ≈ 1,393 MB/s ≥ 1,390 floor).
- ✅ The microbenchmark isolates pure digest cost (asset read once before the
  200-sample loop), and the ≥1,390 MB/s floor is an excellent classifier sitting
  in the empty gap between the soft (~555 MB/s) and hardware (~2,000 MB/s)
  regimes.
- ✅ MSRV is genuinely safe across the whole 0.11 substack (rust-version 1.85 /
  edition 2024, cleared by the pinned 1.90 toolchain), and the `.into()`→`[u8;32]`,
  `hex::encode`, and byte-iteration call-site patterns were verified to remain
  drop-in under `hybrid-array`.
- ✅ Phase 4 derives the skip list empirically from `cargo deny check bans`
  rather than hard-coding it, and the new `[bans].skip` entries correctly will
  not trip `test_advisory_ignores.py` (which scans only `[advisories].ignore`).
- ✅ The security boundary is preserved: only the `sha2` backend of the
  corruption gate changes; minisign / `TrustedKeys` and the vendor-shim marker
  (which closes over `minisign-verify`, not `sha2`) are untouched.

### Recommended Changes

1. **Add source edits for the `sha2` 0.11 `Output` type migration to Phase 3**
   (addresses: the critical `LowerHex` break; "API-transparent" major). Replace
   the three `format!("{:x}", hasher.finalize())` sites with
   `hex::encode(hasher.finalize())` (the pattern already used in the server), and
   add `hex` as a dev-dependency to `work-adapters` and `jira-client` (only the
   server declares it today). Add a pre-Phase-3 step that runs `cargo check
   --all-targets -p <crate>` across all five inheriting crates and records the
   migration. Drop the "no source changes" / "inherit for free" framing.

2. **Make the musl story honest and its failure mode catchable** (addresses: the
   inconclusive-evidence major; the AC 7 blind-spot major; the getauxval-floor
   and predicate minors). Downgrade the darwin `std::arch` print to
   corroborating-only, verify selection through `cpufeatures` itself (or a
   measured delta), record the zig-bundled musl version and the getauxval ≥1.1.21
   floor, and add a first-class 0216 failure condition — "runtime detection
   selects the soft backend on aarch64-musl" — ideally checked under QEMU
   user-mode against the zigbuilt static, so the silent-degradation case triggers
   the 0215 decision path rather than closing green.

3. **Tighten the Phase 4 deny flip** (addresses: the advisories/licenses major;
   the `skip-tree` major; the maintenance-coupling major). Add an explicit step
   diffing `cargo deny check advisories licenses sources` before/after and
   recording that the new substack introduces no advisory and no out-of-allow
   license (or adding the allowance with justification). Prefer versioned `skip`
   over `skip-tree`; where a tree is unavoidable, bound `depth` and pin the
   version. Give the straddle skips `review-by:` dates. Weigh `deny`-vs-`warn`
   explicitly, or gate only first-party-introducible duplicates.

4. **Fix the two broken verification commands** (addresses: the `cargo tree -d`
   false-negative major; the `cargo update --precise` ambiguity major). Split the
   single-version check into `sha2` absent from `cargo tree -d` plus `cargo tree
   -i sha2` (no `-d`) showing `0.11.0`. Disambiguate the update as
   `sha2@0.10.9 --precise 0.11.0` or rely on a plain re-resolve, then assert the
   lock reconciled.

5. **Close the two test gaps the ACs depend on** (addresses: the empty-vector KAT
   major; the AC 6 threading major). Add multi-block/padding-boundary/streaming
   KATs to `verifier.rs`, and extract + unit-test the report-dict assembly so
   `asset_bytes` persistence has an automated guard.

6. **Correct the plan's factual slips** (addresses: the `residual_and_terms`,
   "top-level field", pre-existing-straddle, redundant-test, and fixture-value
   minors). Rename to `close_the_budget`; say the field nests under
   `record["terms"]`; state the substack straddles pre-exist; drop the redundant
   third Phase 1 test and reconcile the fixture byte count with `2493376`.

## Per-Lens Results

### Compatibility

**Summary**: The plan's central claim — that the bump is a pure
dependency-declaration change requiring zero source edits — is unsafe. 0.11
migrates the digest `Output` type from `GenericArray` to `hybrid-array::Array`,
and `hybrid-array` drops the `LowerHex`/`UpperHex` impls three first-party
`format!("{:x}", …)` sites depend on, so Phase 3's own gates fail to compile.
The remaining call-site patterns remain drop-in, MSRV/edition clears 1.90, and
the substack straddle is benign for compilation — so the fix is bounded, but the
plan must scope the source edits (and a `hex` dev-dependency) it currently
denies exist.

**Strengths**:
- MSRV is genuinely safe across the whole 0.11 substack (rust-version 1.85 /
  edition 2024), cleared by the pinned 1.90 toolchain.
- The 0.10/0.11 substack coexistence is benign for compilation: no first-party
  code bridges `digest` 0.10 and 0.11 types.
- The `.into()`→`[u8;32]`, `hex::encode(...)`, and byte-iteration sites were
  verified against `hybrid-array` 0.4.14 source to remain drop-in.
- The five-target deny graph is correctly accounted for (the fifth
  `x86_64-unknown-linux-gnu` triple is enumerated).
- Collapsing the server literal onto the workspace pin establishes a single
  source of truth.

**Findings**:
- 🔴 **critical / high** — sha2 0.11 removes `LowerHex`; three `{:x}` sites break
  the build (`corpus_hashes.rs:27`, `bash_parity_baseline.rs:98`,
  `adf_oracle_manifest.rs:33`). Fix via `hex::encode` + `hex` dev-dep in
  `work-adapters`/`jira-client`.
- 🟡 **major / high** — a major RustCrypto bump assumed API-transparent without a
  per-consumer audit; the breaking usage is in the un-enumerated consuming
  crates.
- 🔵 **minor / medium** — `cargo update -p sha2 --precise 0.11.0` is ambiguous
  while two `sha2` versions are in the lock.

### Correctness

**Summary**: Core mechanics are largely sound — the throughput arithmetic is
consistent, phase ordering is justified, every AC maps to a phase, and the
`decompose_terms` threading correctly updates the single caller. But three
verification steps have defects: the `cargo tree -d -i sha2` gate misreports a
collapsed dependency as empty, the `cargo update --precise` command risks an
ambiguous spec, and the AC 3c runtime print uses a different predicate and target
than the backend it claims to prove.

**Strengths**:
- Throughput arithmetic internally consistent and matching the work item and
  research.
- The `decompose_terms` signature change is safe: one caller, correctly updated;
  the new field sits before the early return so it is always present.
- `parse_term_report` genuinely ignores the trailing `asset_bytes` line, with an
  existing test asserting it.
- The AC→phase mapping table is complete (all seven ACs covered).
- Phase 4 derives the skip list empirically; new `[bans].skip` entries won't trip
  `test_advisory_ignores.py`.

**Findings**:
- 🟡 **major / medium** — `cargo tree -d … -i sha2` yields empty once sha2
  collapses; false-negative on the correct outcome. Split the check.
- 🟡 **major / medium** — `cargo update --package sha2 --precise 0.11.0` may fail
  on an ambiguous spec; disambiguate `sha2@0.10.9` or re-resolve.
- 🟡 **major / medium** — AC 3c uses `std::arch` on darwin, not `cpufeatures` on
  musl; not conclusive for the gated backend.
- 🔵 **minor / high** — the substack straddles pre-exist the bump (owned by
  sha1/hmac/blake2/jj-lib); the bump repoints rather than adds.
- 🔵 **suggestion / low** — the work item's enumerated straddle set is
  host-scoped; the authoritative list is `cargo deny check bans` across five
  targets.

### Portability

**Summary**: The plan makes a single load-bearing portability claim — that sha2
0.11's runtime-detected backend is musl-safe under `crt-static` — but never
executes anything on aarch64-musl to substantiate it. All three evidence prongs
sidestep the getauxval/HWCAP auxv detection at issue, so the real question is
deferred to 0217 while 0216 closes green. The strip/rlib handling, the absence
of forced target flags, and the single-source pin are sound; the gap is the musl
runtime-detection path plus an AC 7 fallback scoped only to build/link failures.

**Strengths**:
- Correctly establishes no `.cargo/config.toml` / `rust-toolchain.toml` /
  RUSTFLAGS force `+sha2` (verified), so worst case is graceful soft-backend
  degradation, not SIGILL.
- Handles the `strip = true` gotcha correctly (symbol evidence from the `.rlib`).
- Collapsing the server literal improves the portability of the backend decision.
- The vendor-shim marker analysis is sound and defensively handled.
- Explicit that aarch64-musl execution is deferred to 0217 as an accepted
  limitation.

**Findings**:
- 🟡 **major / high** — the musl-safe claim is never exercised on aarch64-musl;
  all three prongs sidestep auxv detection (`.rlib` = compiled-in ≠ selected;
  darwin print = macOS `sysctlbyname`).
- 🟡 **major / high** — AC 7 is scoped to build/link failure but the realistic
  musl failure is silent fallback to the soft backend.
- 🔵 **minor / medium** — the musl-safe claim depends on the unstated zig-bundled
  musl providing getauxval (≥1.1.21).
- 🔵 **minor / medium** — the verification predicate (`std_detect`) differs from
  the shipped selector (`cpufeatures`); they can diverge on Linux.
- 🔵 **minor / medium** — before/after and the fixed 1.79 ms gate are calibrated
  to one specific Apple chip.

### Security

**Summary**: A dependency bump on a local verification path, not a new attack
surface; the real security boundary (minisign over BLAKE2b) is untouched and no
secrets are handled. The two genuine security-relevant weaknesses are
supply-chain: Phase 4 flips the duplicate policy but reasons only about `bans`,
never re-vetting the advisories/licenses/sources surface the new substack shifts,
and it endorses broad `skip-tree` entries that mask future duplicate
introductions. A secondary concern: the 0215 fallback pulls the cache-hit
name/version binding into scope without carrying forward 0215's integrity
constraint.

**Strengths**:
- Preserves the actual security boundary (minisign / `TrustedKeys`, vendor-shim
  marker unmoved); a byte-identical-digest requirement is stated.
- The `warn`→`deny` flip materially strengthens supply-chain posture on the
  signed-binary closure; the plan refuses to skip any first-party-collapsible
  duplicate.
- Correctly recognises `sha2 0.10.9` collapses onto the already-present 0.11.0.
- No secret/credential handling in scope.

**Findings**:
- 🟡 **major / medium** — the new sha2 0.11 substack is not explicitly re-vetted
  for advisories/licenses/sources; `deny:check` would surface a surprise failure
  with no planned disposition.
- 🟡 **major / medium** — unbounded `skip-tree` on rand/getrandom and syn
  subtrees weakens the new duplicate gate.
- 🔵 **minor / medium** — a single empty-input KAT is thin defence-in-depth for a
  crypto-backend swap on the corruption gate.
- 🔵 **minor / medium** — the 0215 fallback pulls the cache-hit name/version
  binding into scope without restating its integrity constraint.

### Architecture

**Summary**: Structurally sound — the crate-global switch via a single workspace
pin, with the server literal collapsed onto it, is a clean, high-leverage change
with a correctly enumerated blast radius and no missed consumer, and the
four-phase DAG is well-justified. The principal concerns are cohesion and
evolutionary fitness: an orthogonal workspace-wide deny policy flip is bundled
into a performance work item, and the hard `deny` gate with a permanent,
unaudited skip-list introduces ongoing coupling between routine dependency churn
and CI.

**Strengths**:
- Single source of truth achieved completely (verified across all seven
  declarations).
- The phase ordering is a clean dependency DAG with sound rationale.
- Inter-phase state handed off through committed filesystem artefacts.
- The blast radius of the crate-global switch is enumerated explicitly.

**Findings**:
- 🟡 **major / medium** — the hard `deny` gate with a permanent skip-list couples
  routine dependency churn to CI; the skips carry no `review-by:` audit.
- 🔵 **minor / medium** — an orthogonal workspace-wide deny cleanup bundled into
  a performance work item reduces cohesion.
- 🔵 **minor / high** — Phase 4's "independently mergeable" claim understates its
  content coupling to Phase 3.
- 🔵 **minor / medium** — the cross-work-item reciprocal-edge guarantee rests on
  a one-time manual check with no durable enforcement.
- 🔵 **suggestion / low** — the new measurement-artefact naming/kind diverges
  from the established `meta/measurements/` convention.

### Test-Coverage

**Summary**: Disciplined about the one seam it unit-tests (the new
`parse_asset_bytes` parser gets failing-test-first treatment), but automated
coverage stops short of the behaviours the ACs depend on: the AC-gated
`sha256_hex` keeps only an empty-input KAT that never drives the multi-block
loop, the persistence of `asset_bytes` into the report JSON (AC 6) is verified
only by a manual checkbox, and the Python↔Rust JSON-line contract has no
round-trip guard while its harness is `#[ignore]`d.

**Strengths**:
- Phase 1 follows red-green-refactor honestly.
- The crate-global bump is implicitly regression-covered by existing multi-block
  known-answer digest tests (e.g. `corpus_hashes.rs`).
- Phase 4 gates on the existing six-file deny suite and reminds the implementer
  to confirm no deny test asserts the old `warn` posture.
- Manual verification steps are concrete and artefact-backed.

**Findings**:
- 🟡 **major / medium** — the AC-gated `sha256_hex` is verified only by an
  empty-input vector; add multi-block/padding/streaming KATs.
- 🟡 **major / high** — the `asset_bytes` threading into the report (AC 6) is only
  manually verified; nothing references `close_the_budget`.
- 🔵 **minor / medium** — the parser fixture is untied to the harness (which is
  `#[ignore]`d) and disagrees with the committed `2493376`.
- 🔵 **minor / medium** — parser error/edge cases unspecified; one of the three
  proposed tests already exists.
- 🔵 **minor / medium** — "the driver is the test" plus `skip-tree` risks masking
  future first-party-collapsible duplicates.
- 🔵 **suggestion / low** — a deny test's warn-level rationale comment goes stale
  after the flip.
- 🔵 **suggestion / low** — the throwaway runtime print has no guard ensuring
  removal.

### Performance

**Summary**: Targets a well-chosen bottleneck with the simplest possible fix, and
the microbenchmark methodology is fundamentally sound — the timed region isolates
pure CPU digest cost, and the ≥1,390 MB/s floor is an excellent binary classifier
between the soft and hardware regimes. The gaps are methodological framing: AC 1
is phrased as a fixed latency only equivalent to the throughput floor at the
assumed asset size, the gate rests on a single-run median with the p97.5 band
unused, and the plan leans on the uncalibrated baselines without noting
`calibration.holds: false`.

**Strengths**:
- The timed region isolates pure digest cost (asset read once before the loop).
- The ≥1,390 MB/s floor is an excellent classifier with ~30% headroom.
- The throughput derivation is dimensionally correct and consistently decimal.
- Phase 2 includes a backend-sanity anchor rejecting an already-fast "before".
- Scope discipline: MB/s derived in prose, six non-gated sites ride for free.

**Findings**:
- 🔵 **minor / medium** — AC 1 gates fixed latency, not throughput; equivalence
  breaks if the asset size drifts.
- 🔵 **minor / medium** — single-run median-only gate leaves the p97.5 band and
  the harness validity verdicts unused.
- 🔵 **suggestion / medium** — the plan treats ~4.3 ms as authoritative without
  noting `calibration.holds` is false.
- 🔵 **suggestion / low** — the perf benefit is throughput-validated only on
  darwin, on a best-case one-shot site.

### Code-Quality

**Summary**: The only new production code (Phase 1's Python) is small, pure, and
highly testable, and it reuses the file's naming and docstring conventions well.
The main concerns are inaccuracies in how the plan names and locates existing
code — it references a function (`residual_and_terms`) that does not exist and
describes the new field as top-level when it nests under `record["terms"]` — plus
minor DRY/naming/line-width refinements. None blocking; the pseudocode is sound.

**Strengths**:
- The new parser seam is pure functions over an in-memory string — trivially
  unit-testable, tests written first.
- Naming is domain-consistent (`parse_asset_bytes`, the `asset_bytes` key mirrors
  the harness).
- Wrapping two return values in a frozen `LauncherTerms` dataclass is cleaner than
  a bare tuple.
- The one-line docstring adds genuine domain meaning without offending the
  comments-as-last-resort rule.
- The error contract matches `parse_term_report`'s existing unguarded style.

**Findings**:
- 🔵 **minor / high** — the plan references a non-existent function
  `residual_and_terms`; the real one is `close_the_budget` (`measure.py:1858`).
- 🔵 **minor / medium** — "top-level field ... sibling to `cache_root_bytes`"
  contradicts the actual nesting under `record["terms"]`.
- 🔵 **suggestion / medium** — `LauncherTerms.intervals` diverges from the file's
  "terms" vocabulary.
- 🔵 **suggestion / medium** — the two parsers duplicate the scan-and-filter-JSON
  idiom; optionally share a generator.
- 🔵 **suggestion / medium** — the README formula exceeds 80 columns and uses
  non-ASCII operators.

---
*Review generated by /accelerator:review-plan*

## Re-Review (Pass 2) — 2026-09-11

**Verdict:** APPROVE

All eight lenses re-ran against the edited plan. Every finding from the initial
review — the critical and all ten majors — is resolved and verified against the
live tree. The first edit round introduced a handful of small regressions (one
major, several minor), which a second edit round then fixed; the plan is now
sound and ready for implementation.

### Previously Identified Issues

- 🔴 **Compatibility** — sha2 0.11 `LowerHex` build break: **Resolved.** Phase 3
  §4 migrates the three `{:x}` sites to `hex::encode`; verified as the only
  breakage in the tree.
- 🟡 **Compatibility** — major bump assumed API-transparent: **Resolved.**
  Per-consumer `cargo check` audit added (and corrected to all six consumers in
  the follow-up).
- 🟡 **Portability / Correctness** — musl backend-selection evidence
  inconclusive: **Resolved.** Compiled-in vs selected separated; darwin print
  corroborating-only; queries `cpufeatures`, not `std::arch`.
- 🟡 **Portability** — AC 7 missed the silent soft-backend degradation:
  **Resolved.** Added as a first-class trigger with the 0217 throughput backstop.
- 🟡 **Security** — substack advisories/licenses/sources not re-vetted:
  **Resolved.** Phase 4 §2 diff + Phase 3 `deny:check` gate + Phase 2 baseline.
- 🟡 **Security / Test-Coverage / Architecture** — unbounded `skip-tree`:
  **Resolved.** Narrowed to versioned `skip` / depth-bounded, version-pinned
  trees, with a verification checkbox.
- 🟡 **Architecture** — hard `deny` gate + unaudited skip-list: **Resolved.**
  Narrowed skips keep the gate honest; the coupling is acknowledged in Migration
  Notes.
- 🟡 **Correctness** — `cargo tree -d … -i sha2` false-negative gate:
  **Resolved.** Split into `sha2` absent from `cargo tree -d` plus `cargo tree -i
  sha2` (no `-d`).
- 🟡 **Correctness / Compatibility** — `cargo update --precise` ambiguity:
  **Resolved.** Disambiguated to `sha2@0.10.9 --precise 0.11.0`.
- 🟡 **Test-Coverage / Security** — empty-input-only KAT: **Resolved.**
  Multi-block, padding-boundary, and streaming-equivalence vectors added.
- 🟡 **Test-Coverage** — AC 6 `asset_bytes` threading only manually verified:
  **Resolved.** Report-assembly extracted and unit-tested.
- 🔵 **Minor / suggestion set** (wrong function name, nesting wording, pre-existing
  straddle wording, README formula, calibration note, AC-1 throughput framing,
  0215 integrity binding, mergeability wording): **Resolved.** A few residuals
  persist by explicit decision — measurement-artefact naming, reciprocal-edge
  lacking tooling enforcement, parser-duplication KISS, and no `review-by:` dates
  on the straddle skips.

### New Issues Introduced (first edit round) — now fixed

- 🟡 **Correctness (major)** — AC 7 table row and "What We're NOT Doing" still
  said "build/link failure only" while the body carried two triggers: **Fixed.**
  Both now name both triggers and note the soft-backend trigger stays open
  pending 0217.
- 🔵 **Correctness (minor)** — AC 1 gated on whole-run `analysis.validity`, which
  derives from dispatch-budget drift, not the isolated term: **Fixed.** Gates on
  the term's own median + p97.5.
- 🔵 **Compatibility (minor)** — audit list said "five consumers", omitting the
  server (a sixth): **Fixed.** Corrected to six with `server` added.
- 🔵 **Compatibility / Architecture (minor)** — `hex` added as a scattered
  per-crate dev-dep, riskable under the deny flip: **Fixed.** Hoisted to
  `[workspace.dependencies]` and referenced via `hex = { workspace = true }`.
- 🔵 **Security / Architecture (minor)** — substack ships in Phase 3 but the
  re-vet was gated only in Phase 4: **Fixed.** `deny:check` added to Phase 3, a
  pre-bump baseline committed in Phase 2, and the licence-closure evidence named.
- 🔵 **Correctness (suggestion)** — Phase 4 diff had no committed pre-bump
  referent: **Fixed.** Phase 2 commits `deny-surface-before.txt`.
- 🔵 **Portability (minor)** — conclusive musl verification rests entirely on
  0217 (no CI lane): **Fixed.** Step 8 made a hard close-out gate.
- 🔵 **Test-Coverage / Code-Quality (suggestions)** — stale deny-test comment,
  throwaway-probe removal, fixture-vs-canonical divergence, elision markers:
  **Addressed.** Phase 4 comment-update step; out-of-tree scratch crate;
  test-double clarification plus a harness-contract golden; implementer note.

### Assessment

The plan is in good shape. The critical compile-break and every major are
resolved and verified against the live tree, and the regressions the first edit
round introduced are fixed. The remaining residuals are conscious, recorded
decisions — the deny cleanup kept in 0216, no `review-by:` dates on the straddle
skips, inspection-only musl with the 0217 backstop, and the measurement-artefact
naming — rather than open defects. Ready for implementation.

---
*Re-review generated by /accelerator:review-plan*
