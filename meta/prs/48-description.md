---
type: pr-description
id: "48"
title: "Add a library-backed VCS adapter over gix and jj-lib"
date: "2026-08-03T23:18:17+00:00"
author: "Toby Clemson"
producer: describe-pr
status: complete
work_item_id: "0188"
parent: "work-item:0188"
relates_to: ["work-item:0125", "work-item:0169", "work-item:0185"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/48"
pr_number: 48
tags: [rust, vcs, dependencies, gix, jj-lib]
revision: "53fe29fdd423a3f0617d9fa1a762928d88b77744"
repository: "accelerator"
last_updated: "2026-08-03T23:58:14+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Add a library-backed VCS adapter over gix and jj-lib

## Summary

Adds `InProcessProbe` — an implementation of `cli/vcs`'s `RepoRoot` and `VcsProbe` ports that reads git through `gix` and jj through `jj-lib` in the calling process rather than by spawning the VCS binaries — plus six taxonomy queries the subprocess pair has no equivalent for, and the test apparatus that proves both idioms are read without a subprocess.

**Nothing is wired.** `vcs_adapters::facts` still names the subprocess pair, so no runtime behaviour changes. The value delivered is risk isolation: two dependency trees, the workspace's first copyleft licence exception and a pre-1.0 API bet land where they can be reviewed and reverted on their own.

## Changes

### The adapter (`cli/vcs-adapters`)

- **`library.rs`** — `InProcessProbe`, both port implementations, and six inherent queries returning `Result<Option<T>, Error>` so failure is distinguishable from absence: `is_bare`, `worktree`, `superproject`, `jj_workspace_root`, `jj_repository`, and `dual_roots` (infallible, with a per-side `Result` so a one-sided failure can't read as "jj only").
- **Three distinct walks, not one.** The combined `.jj`-or-`.git` boundary walk answers `RepoRoot::discover` only; a **`.jj`-only** walk answers the jj queries, because jj-lib's loader performs no walk of its own and feeding it the combined boundary reports absence on a git checkout nested inside a jj workspace — where `jj workspace root` reports a root; `gix::discover` performs the third.
- **`subprocess.rs`** — the retained `MarkerWalkRoot`/`CommandProbe` pair, moved out of the crate root so each file holds one shape. `lib.rs` drops from 315 lines to 59: module declarations, `facts`, one test.
- **`markers.rs`** — the ancestor walk and marker reading both adapters share. Each delegates *to* it, so retiring either strands nothing.
- Every returned path is canonicalised at one choke point, and `start` is absolutised before any walk — without it a relative start makes a colocated checkout report a git root and no jj root, which is the wrong classifier arm.

### Dependency policy and enforcement

- `jj-lib = "=0.43.0"` (exact — declared-unstable internals) and `gix = { version = "~0.85.0", default-features = false }`. Defaults are off because gix's `default` reaches `gix-credentials`, the one gix subsystem that spawns `git credential-*` helpers.
- `prost` and `pollster` as direct edges for the jj revision route. Both were already in the lock through jj-lib, so this adds **two edges and no packages**.
- The workspace's first `[[licenses.exceptions]]` — `uluru`, MPL-2.0, reached from `gix-pack` and not feature-gatable out.
- A cargo-pup rule scoping `vcs_adapters::library`'s imports to a permit list and denying `std::process`, with probe cases for the rule.
- A source guard (`lint:vcs-settings:check`) forbidding jj settings construction crate-wide — cargo-pup resolves `use` paths and cannot see a fully-qualified call.
- Graph invariants: single gix/prost/pollster versions, the enabled gix feature set, no TLS in the subtree on all five targets, and no package requiring a Rust newer than the pin.

### Test apparatus (`cli/vcs-test-support`, new crate)

- A 34-pair fixture matrix covering colocated, secondary-workspace, nested, submodule (depths 1 and 2, plus three awkward shapes), worktree, bare, reftable, sha256, hostile-config and three degenerate shapes.
- The hermetic environment, the marker-writing stubs, and a platform-aware report of the absolute paths it cannot shadow. The Rust harness never writes outside its own temp directories.
- Zero-spawn proven **across a crate boundary** from `cli/corpus-adapters`, the crate that will converge onto the adapter.

### Build system and CI

- `test:integration:zero-spawn` (PATH-only) and `test:integration:zero-spawn:strong` (shadows the real binaries; gated behind `ACCELERATOR_ZERO_SPAWN_SHADOW=yes`), plus a `check-zero-spawn` Linux job.
- A committed size floor on the reference artefact — linked ≥ 3× stubbed everywhere, plus an absolute byte floor on musl — guarding this story's headline false pass, where dead-code elimination lets the musl and size checks succeed while linking almost none of the trees.
- `version.write` now syncs `cli/Cargo.lock`, which carries a copy of the version per workspace member and was being left behind.

## Context

- Work item: `meta/work/0188-library-backed-vcs-adapter.md`
- Plan: `meta/plans/2026-08-03-0188-library-backed-vcs-adapter.md`, carrying the full oracle mapping — every query against every fixture, with the shell implementation in `scripts/vcs-common.sh` as the behavioural oracle
- Research: `meta/research/codebase/2026-08-02-0188-library-backed-vcs-adapter.md`
- Plan review: `meta/reviews/plans/2026-08-03-0188-library-backed-vcs-adapter-review-1.md`
- Validation: `meta/validations/2026-08-03-0188-library-backed-vcs-adapter-validation.md`
- Dated hand-off notes are appended to **0125**, **0169** and **0185**; ADR-0053 (thin CLI over a hexagonal core) is the governing decision

## Testing

- [x] `mise run` green end to end (601s, zero task failures, no formatter drift)
- [x] `test:unit:cli` — 625 tests. Includes the six queries against all 34 (fixture, start directory) pairs, every expected value traceable to a cell in the recorded oracle mapping
- [x] The scrub invariant across the whole matrix, with a live-poison control (`gix::discover_with_environment_overrides` resolving the poison target) so the assertion can't hold vacuously
- [x] `detection.rs` runs every pre-existing case through an injection seam against **both** implementations, asserting identical `RepoFacts` for both idioms while keeping its own fixed expected values — agreement between two implementations is not on its own an oracle
- [x] `test:integration:deny` (65), `test:unit:tasks` (660), `test:integration:pup`, `lint:vcs-settings:check`
- [x] Non-vacuity committed rather than demonstrated by hand: a `std::process` import fails cargo-pup naming the rule, a deliberate settings construction fails the source guard, and a compliant module passes as the positive control
- [x] `test:integration:zero-spawn` in PATH-only mode
- [x] All four release triples cross-compile; both musl builds pass the static-ELF assertion; size ratios 6.86×–7.37× against a 3× floor
- [x] The jj-fixture shell suites (`hooks/test-vcs-detect.sh`, `scripts/test-metadata-helpers.sh`) green under the jj 0.43 pin
- [x] The strong-form task refuses to run without its opt-in, and touches nothing
- [ ] **The strong-form zero-spawn run itself — see Notes.** It has never executed and will not pass as written
- [ ] Cold-cache CI timings against `test-visual-regression`'s 20-minute cap. Measured locally: the two new trees cost 16.92s wall / 65.8s CPU cold, so roughly a minute or two on a 4-vCPU runner. No budget raise expected — worth confirming on the first run

## Notes for Reviewers

**Start with the dependency policy.** `cli/Cargo.toml`, `cli/deny.toml` and `cli/pup.ron` are the reviewable core; the adapter is replaceable, a licence exception and a pre-1.0 pin are not. The trees enter the build graph of the shipped `accelerator-visualiser`, though dead-code elimination removes them from the binary — verified, and the basis for the MPL-2.0 finding below.

**The MPL-2.0 §3.2 question is closed by measurement, conditionally.** `uluru` reaches exactly one shipped binary, and an unstripped release build of it carries zero symbols from `gix`, `gix-pack`, `gix-odb`, `jj_lib`, `clru` or `uluru` against 26,247 total, and none of the distinctive literals the linked reference artefact does carry. So the notice obligation does not bind and no attribution artefact is needed. The finding is **contingent** on nothing in the visualiser reaching `vcs-adapters`, so its re-check trigger is recorded in the `deny.toml` comment and on 0185.

**A spike conclusion was reversed during review.** The plan descoped the jj half of `revision` to 0185, on a finding that jj-lib 0.43 offers no read-only, settings-free route to the working-copy commit id. That was wrong — `jj_lib::protos` is a **public** module, so the workspace's checkout state decodes through published API rather than a private wire format. The route is delivered here and matches the CLI exactly across pure-jj, colocated, commitless, secondary-workspace and multi-workspace shapes, writing nothing. Amendment 8 is withdrawn by amendment 10.

**One deliberate behavioural divergence, and it is the read-only direction.** Asking the `jj` binary snapshots the working copy first, so with unsnapshotted edits present it reports — and *writes* — a new commit. The in-process route reports the commit as of the last recorded operation and writes nothing. After 0185's switch, deriving metadata therefore stops mutating the user's repository. Pinned by `an_unsnapshotted_edit_is_the_one_documented_divergence`.

**Known defect, not fixed here.** The strong-form zero-spawn suite builds its own fixture matrix, which needs the real binaries — so on CI that build lands inside the shadow window with no `git` or `jj` to run it. The job has never executed, so this is latent rather than a regression. Fixing it needs a Rust-side handoff (build the matrix outside the window, pass its root in), which is scoped separately rather than smuggled into this branch.

**Pre-existing flake worth knowing about.** `test:integration:entrypoint` failed in two of five full local runs, always because `tests/integration/support/installation.py:125-141` builds into the shared `cli/target/debug` and asserts the artefact's presence with no tolerance for a concurrent rebuild. CI is structurally immune — `test-unit` and `test-integration` are separate jobs — so the exposure is to the local `mise run` gate. Deserves its own item; this branch enlarges the Rust build and so widens the window.

**What this deliberately does not do:** wire anything (`facts` stays hard-wired, 0185 owns the switch), build `classify_checkout`'s arm cascade or `vcs status` / `vcs log` (0169), define a domain port over the six queries (0169), gate on cost, or migrate the ~26 shell call sites. `cli/vcs/src/**` is byte-for-byte unmodified.

**Sizing:** 16 commits, +14,268/−528 across 51 files. Roughly 6,700 lines are `meta/` documents (plan, research, review, validation, work-item amendments) and 1,447 are `cli/Cargo.lock`, whose delta is worth reading only for the `vcs-adapters` dependency edges. That leaves about 4,200 lines of Rust — two thirds of it fixtures and tests — and about 1,400 of Python task and guard code.

**Two commits are housekeeping over this branch's own additions**, not delivered capability: one extracts the retained subprocess pair out of the crate root into `subprocess.rs` so each file holds one adapter, and one cuts the comments back to this repo's bar — 900 comment lines down to 635, with every reference to work items, ADRs, ACs and plan documents removed, since those go stale across work item boundaries. Neither changes behaviour, and both are worth skipping when reading the diff for correctness.
